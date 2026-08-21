//! 单连接处理: 帧解析 + 登录握手状态机 + 进世界。

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc};

use std::sync::Arc;

use crystal_protocol::binary::Point;
use crystal_protocol::client as c;
use crystal_protocol::frame::encode_packet;
use crystal_protocol::server as s;
use crystal_protocol::types::{MirClass, MirDirection, MirGender, UserItem};
use crystal_protocol::ClientPacket;

use crate::db::Database;
use crate::items;
use crate::web3::Web3Auth;
use crate::world::{
    drop_ground_item, drop_gold, equipment_slots, gain_experience, gain_gold, npc_name, npc_shop,
    persist_player, player_attack, player_gold, player_harvest, player_magic_attack,
    player_range_attack, pick_up, recompute_stats, remove_gold, use_consumable, Player, World,
};

/// 连接所处的游戏阶段（对应 C# `GameStage`）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    None,
    Login,
    Select,
    Game,
}

const MAX_FRAME: usize = 64 * 1024 * 1024; // 防超大帧

pub async fn handle_connection(
    stream: TcpStream,
    db: Arc<Database>,
    world: World,
    web3_auth: Arc<Web3Auth>,
) -> anyhow::Result<()> {
    let (mut reader_half, mut writer_half) = stream.into_split();
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(256);

    // 写出任务: 帧队列 → socket
    let writer_task = tokio::spawn(async move {
        while let Some(frame) = rx.recv().await {
            if writer_half.write_all(&frame).await.is_err() {
                break;
            }
        }
    });

    // 服务器握手: 连接建立即发送 Connected（与 Crystal 一致）
    tx.send(encode_packet(&s::Connected)).await.ok();

    // 世界广播转发任务: 其他玩家的移动/聊天转发给本连接（客户端自行过滤自身 ID）
    let mut broadcast_rx: broadcast::Receiver<Vec<u8>> = world.subscribe();
    let fwd_tx = tx.clone();
    let fwd_task = tokio::spawn(async move {
        while let Ok(frame) = broadcast_rx.recv().await {
            if fwd_tx.send(frame).await.is_err() {
                break;
            }
        }
    });

    let mut account_id: Option<String> = None;
    let mut stage = Stage::None;
    let mut object_id: Option<u32> = None;
    let mut char_info: Option<(MirClass, MirGender, String)> = None;

    let mut buf: Vec<u8> = Vec::with_capacity(8 * 1024);

    loop {
        // 读入缓冲
        let mut chunk = [0u8; 8192];
        let n = match reader_half.read(&mut chunk).await {
            Ok(0) => break, // 对方关闭
            Ok(n) => n,
            Err(_) => break,
        };
        buf.extend_from_slice(&chunk[..n]);

        // 持续从缓冲解析帧
        loop {
            let Some((id, payload, used)) = try_parse_frame(&buf) else {
                break;
            };
            buf.drain(..used);

            match ClientPacket::decode(id, &payload) {
                Ok(cpacket) => {
                    if let Err(e) = handle_client_packet(
                        &cpacket,
                        &mut stage,
                        &mut account_id,
                        &mut object_id,
                        &mut char_info,
                        &db,
                        &world,
                        &tx,
                        &web3_auth,
                    )
                    .await
                    {
                        tracing::warn!("处理包失败 id={id}: {e}");
                    }
                }
                Err(e) => {
                    tracing::warn!("无效客户端包 id={id}: {e}");
                    // 与 C# 一致: 数据损坏时丢弃剩余缓冲，防死循环
                    buf.clear();
                }
            }
        }
    }

    // 断线清理：先持久化进度（金币/经验/等级/位置/血量），再移除在线实体
    if let Some(oid) = object_id {
        // 进行中的交易自动取消并通知对方（物品未转移、金币未预扣，无需返还）
        handle_trade_cancel(&world, oid, false).await;
        // 在队伍中则自动退队并通知剩余成员（同 C# 断线离队）
        let name = world.get_player(oid).await.map(|p| p.name);
        if let Some(name) = name {
            group_leave_broadcast(&world, &name).await;
        }
        persist_player(&world, &db, oid).await;
        world.remove_player(oid).await;
    }
    fwd_task.abort();
    writer_task.abort();
    Ok(())
}

/// 从缓冲尝试解析一个帧；数据不足返回 None。
///
/// 帧长字段 < 4 或 > 上限时按畸形帧处理: 消耗整块缓冲（防死循环，同 C#）。
fn try_parse_frame(buf: &[u8]) -> Option<(i16, Vec<u8>, usize)> {
    if buf.len() < 4 {
        return None;
    }
    let len = u16::from_le_bytes([buf[0], buf[1]]) as usize;
    if len < 4 || len > MAX_FRAME {
        return Some((0, Vec::new(), buf.len()));
    }
    if buf.len() < len {
        return None;
    }
    let id = i16::from_le_bytes([buf[2], buf[3]]);
    Some((id, buf[4..len].to_vec(), len))
}

#[allow(clippy::too_many_arguments)]
async fn handle_client_packet(
    packet: &ClientPacket,
    stage: &mut Stage,
    account_id: &mut Option<String>,
    object_id: &mut Option<u32>,
    char_info: &mut Option<(MirClass, MirGender, String)>,
    db: &Database,
    world: &World,
    tx: &mpsc::Sender<Vec<u8>>,
    web3_auth: &Web3Auth,
) -> anyhow::Result<()> {
    match packet {
        ClientPacket::ClientVersion(c::ClientVersion { .. }) => {
            tx.send(encode_packet(&s::ClientVersion { result: 1 }))
                .await
                .ok();
            *stage = Stage::Login;
        }
        ClientPacket::KeepAlive(ka) => {
            tx.send(encode_packet(&s::KeepAlive { time: ka.time }))
                .await
                .ok();
        }
        ClientPacket::NewAccount(na) => {
            let result = if !valid_account_id(&na.account_id) {
                1
            } else if na.password.len() < 3 {
                2
            } else if !db.register(&na.account_id)? {
                7
            } else {
                8
            };
            tx.send(encode_packet(&s::NewAccount { result })).await.ok();
        }
        ClientPacket::Login(login) => {
            let result = if !db.login(&login.account_id)? {
                3 // 账号不存在
            } else {
                *account_id = Some(login.account_id.clone());
                *stage = Stage::Select;
                0
            };
            if result == 0 {
                let characters = db.select_infos(account_id.as_ref().unwrap())?;
                tx.send(encode_packet(&s::LoginSuccess { characters }))
                    .await
                    .ok();
            } else {
                tx.send(encode_packet(&s::Login { result })).await.ok();
            }
        }
        ClientPacket::Web3ChallengeRequest(req) => {
            // 规范化地址；非法则回 result=1
            match Web3Auth::normalize_address(&req.address) {
                Ok((addr, _)) => {
                    let ch = web3_auth.issue_challenge(&addr);
                    tx.send(encode_packet(&s::Web3Challenge {
                        address: ch.address,
                        message: ch.message,
                        expires_in: ch.expires_in,
                    }))
                    .await
                    .ok();
                }
                Err(_) => {
                    tx.send(encode_packet(&s::Web3LoginResult {
                        result: 1,
                        characters: vec![],
                        session_token: String::new(),
                    }))
                    .await
                    .ok();
                }
            }
        }
        ClientPacket::Web3Login(wl) => {
            // 校验签名并消耗挑战
            let outcome = web3_auth.verify_and_consume(&wl.address, &wl.challenge, &wl.signature);
            match outcome {
                Ok((addr, _)) => {
                    // 地址即账号：首次自动注册，随后取角色列表
                    *account_id = Some(addr.clone());
                    *stage = Stage::Select;
                    let characters = db.web3_login(&addr)?;
                    // 登录成功签发会话 token（免签名重连）
                    let session = web3_auth.issue_session(&addr);
                    tx.send(encode_packet(&s::Web3LoginResult {
                        result: 0,
                        characters,
                        session_token: session,
                    }))
                    .await
                    .ok();
                }
                Err(crate::web3::Web3Error::ChallengeExpired) => {
                    tx.send(encode_packet(&s::Web3LoginResult {
                        result: 2,
                        characters: vec![],
                        session_token: String::new(),
                    }))
                    .await
                    .ok();
                }
                Err(_) => {
                    tx.send(encode_packet(&s::Web3LoginResult {
                        result: 3,
                        characters: vec![],
                        session_token: String::new(),
                    }))
                    .await
                    .ok();
                }
            }
        }
        ClientPacket::Web3SessionLogin(sl) => {
            // 用会话 token 免签名重连：消耗 token -> 恢复钱包地址为账号
            match web3_auth.consume_session(&sl.token) {
                Some(addr) => {
                    *account_id = Some(addr.clone());
                    *stage = Stage::Select;
                    let characters = db.web3_login(&addr)?;
                    // 换发新 token，便于持续重连
                    let session = web3_auth.issue_session(&addr);
                    tx.send(encode_packet(&s::Web3LoginResult {
                        result: 0,
                        characters,
                        session_token: session,
                    }))
                    .await
                    .ok();
                }
                None => {
                    tx.send(encode_packet(&s::Web3LoginResult {
                        result: 2, // token 无效或过期
                        characters: vec![],
                        session_token: String::new(),
                    }))
                    .await
                    .ok();
                }
            }
        }
        ClientPacket::AddMember(am) => {
            // 队长按名字邀请玩家入队（同 C# PlayerObject.AddMember）
            if let Some(oid) = *object_id {
                let inviter = world.get_player(oid).await.map(|p| p.name).unwrap_or_default();
                let t_oid = player_oid_by_name(world, &am.name).await;
                // 校验与入队登记（锁内不做 IO）
                let check: Result<(), &str> = {
                    let mut g = world.group.lock().await;
                    match t_oid {
                        None => Err("对方不在线或不存在"),
                        Some(t) if t == oid => Err("不能邀请自己"),
                        Some(t) => {
                            let allow = world
                                .players
                                .lock()
                                .await
                                .get(&t)
                                .map(|p| p.allow_group)
                                .unwrap_or(false);
                            if !allow {
                                Err("对方未开放组队")
                            } else {
                                g.invite(&inviter, &am.name)
                                    .map(|_| ())
                                    .map_err(|e| match e {
                                    crate::group::GroupError::AlreadyInGroup => "对方已在其他队伍",
                                    crate::group::GroupError::AlreadyInvited => "对方已有待处理的邀请",
                                    crate::group::GroupError::GroupFull => "队伍已满",
                                    _ => "无法邀请：你不是队长",
                                })
                            }
                        }
                    }
                };
                match check {
                    Err(msg) => send_sys(world, oid, msg).await,
                    Ok(()) => {
                        // 邀请方强制开启允许组队并回执（C# SwitchGroup(true)）
                        {
                            let mut players = world.players.lock().await;
                            if let Some(p) = players.get_mut(&oid) {
                                p.allow_group = true;
                            }
                        }
                        tx.send(encode_packet(&s::SwitchGroup { allow_group: true }))
                            .await
                            .ok();
                        if let Some(t) = t_oid {
                            let frame = encode_packet(&s::GroupInvite { name: inviter.clone() });
                            world.send_to(t, frame).await;
                        }
                        send_sys(world, oid, &format!("已邀请 {} 加入队伍", am.name)).await;
                    }
                }
            }
        }
        ClientPacket::GroupInvite(gi) => {
            // 响应邀请（同 C# PlayerObject.GroupInvite）：accept=true 入队并双向广播 AddMember
            if let Some(oid) = *object_id {
                let me = world.get_player(oid).await.map(|p| p.name).unwrap_or_default();
                let inviter = { world.group.lock().await.pending_inviter(&me) };
                if inviter.is_none() {
                    send_sys(world, oid, "没有待处理的组队邀请").await;
                } else if !gi.accept_invite {
                    world.group.lock().await.decline(&me);
                    let host_oid = match inviter.as_ref() {
                        Some(h) => player_oid_by_name(world, h).await,
                        None => None,
                    };
                    if let Some(host_oid) = host_oid {
                        send_sys(world, host_oid, &format!("{} 拒绝了你的组队邀请", me)).await;
                    }
                } else {
                    let joined = { world.group.lock().await.accept(&me) };
                    match joined {
                        Err(_) => send_sys(world, oid, "邀请已失效或队伍已满").await,
                        Ok(list) => {
                            // 新成员收到全队名单；老成员收到新成员加入
                            for m in &list {
                                if m != &me {
                                    let frame = encode_packet(&s::AddMember { name: m.clone() });
                                    world.send_to(oid, frame).await;
                                }
                            }
                            let frame = encode_packet(&s::AddMember { name: me.clone() });
                            for m in &list {
                                if let Some(m_oid) = player_oid_by_name(world, m).await {
                                    world.send_to(m_oid, frame.clone()).await;
                                }
                            }
                            send_sys(world, oid, &format!("已加入 {} 的队伍", inviter.unwrap_or_default())).await;
                        }
                    }
                }
            }
        }
        ClientPacket::DelMember(dm) => {
            // 空名字=自己退队；否则队长移除成员（同 C# DelMember+LeaveGroup）
            if let Some(oid) = *object_id {
                let me = world.get_player(oid).await.map(|p| p.name).unwrap_or_default();
                let target = if dm.name.is_empty() { me.clone() } else { dm.name.clone() };
                let mut allowed = true;
                if target != me {
                    let is_leader = { world.group.lock().await.is_leader(&me) };
                    if !is_leader {
                        send_sys(world, oid, "只有队长可以移除成员").await;
                        allowed = false;
                    } else {
                        let in_my_group = {
                            let mut g = world.group.lock().await;
                            match g.group_of(&target) {
                                Some(gid) => g.members(gid).contains(&me),
                                None => false,
                            }
                        };
                        if !in_my_group {
                            send_sys(world, oid, &format!("{} 不在你的队伍中", target)).await;
                            allowed = false;
                        }
                    }
                    if allowed {
                        // 被踢者收 DeleteGroup，其余走统一离队广播
                        if let Some(t_oid) = player_oid_by_name(world, &target).await {
                            world.send_to(t_oid, encode_packet(&s::DeleteGroup)).await;
                            send_sys(world, t_oid, &format!("你已被 {} 移出队伍", me)).await;
                        }
                    }
                }
                if allowed {
                    group_leave_broadcast(world, &target).await;
                }
            }
        }
        ClientPacket::SwitchGroup(sg) => {
            // 切换是否接收邀请：存储 + 回执；关闭时若在队中则立即退队（C# 语义）
            if let Some(oid) = *object_id {
                {
                    let mut players = world.players.lock().await;
                    if let Some(p) = players.get_mut(&oid) {
                        p.allow_group = sg.allow_group;
                    }
                }
                tx.send(encode_packet(&s::SwitchGroup {
                    allow_group: sg.allow_group,
                }))
                .await
                .ok();
                if !sg.allow_group {
                    let name = world.get_player(oid).await.map(|p| p.name).unwrap_or_default();
                    group_leave_broadcast(world, &name).await;
                }
            }
        }
        ClientPacket::NewCharacter(nc) => {
            let result = match (account_id.as_ref(), char_name_valid(&nc.name)) {
                (Some(aid), true) => match db.add_character(aid, &nc.name, nc.class, nc.gender)? {
                    Ok(info) => {
                        tx.send(encode_packet(&s::NewCharacterSuccess { char_info: info }))
                            .await
                            .ok();
                        return Ok(());
                    }
                    Err(code) => code,
                },
                (Some(_), false) => 1,
                (None, _) => 3,
            };
            tx.send(encode_packet(&s::NewCharacter { result }))
                .await
                .ok();
        }
        ClientPacket::DeleteCharacter(dc) => {
            let deleted = account_id
                .as_ref()
                .map(|aid| db.delete_character(aid, dc.character_index))
                .transpose()?
                .unwrap_or(false);
            if deleted {
                tx.send(encode_packet(&s::DeleteCharacterSuccess {
                    character_index: dc.character_index,
                }))
                .await
                .ok();
            } else {
                tx.send(encode_packet(&s::DeleteCharacter { result: 1 }))
                    .await
                    .ok();
            }
        }
        ClientPacket::StartGame(sg) => {
            let Some(aid) = account_id.as_ref() else {
                tx.send(encode_packet(&s::StartGame {
                    result: 1,
                    resolution: 0,
                }))
                .await
                .ok();
                return Ok(());
            };
            let Some(ch) = db.get_character(aid, sg.character_index)? else {
                tx.send(encode_packet(&s::StartGame {
                    result: 2,
                    resolution: 0,
                }))
                .await
                .ok();
                return Ok(());
            };

            tx.send(encode_packet(&s::StartGame {
                result: 0,
                resolution: 1024,
            }))
            .await
            .ok();

            let oid = enter_world(db, world, tx, aid, &ch, ch.level as u16).await?;
            *object_id = Some(oid);
            *char_info = Some((num_class(ch.class), num_gender(ch.gender), ch.name.clone()));
            *stage = Stage::Game;
        }
        ClientPacket::LogOut(_) => {
            if let Some(oid) = object_id.take() {
                // 主动登出也需持久化进度
                persist_player(&world, db, oid).await;
                world.remove_player(oid).await;
            }
            *stage = Stage::Select;
            if let Some(aid) = account_id.as_ref() {
                let characters = db.select_infos(aid)?;
                tx.send(encode_packet(&s::LoginSuccess { characters }))
                    .await
                    .ok();
            }
        }
        ClientPacket::Turn(t) => {
            move_player(world, object_id, tx, t.direction, 0).await;
        }
        ClientPacket::Walk(wk) => {
            if !is_dead(world, *object_id).await {
                move_player(world, object_id, tx, wk.direction, 1).await;
            }
        }
        ClientPacket::Run(r) => {
            if !is_dead(world, *object_id).await {
                move_player(world, object_id, tx, r.direction, 2).await;
            }
        }
        ClientPacket::Chat(chat) => {
            if let Some(oid) = *object_id {
                // 调试/演示命令：/map <index> 传送，/spawn 回新手村
                let msg = chat.message.trim();
                let cmd: Vec<&str> = msg.split_whitespace().collect();
                if !cmd.is_empty() && cmd[0].starts_with('/') {
                    handle_chat_command(world, db, tx, oid, &msg, cmd).await;
                } else {
                    let frame = encode_packet(&s::ObjectChat {
                        object_id: oid,
                        text: chat.message.clone(),
                        r#type: 0,
                    });
                    world.broadcast_except(frame.clone(), oid).await;
                    let _ = tx.send(frame).await; // 自己也收到
                }
            }
        }
        ClientPacket::Attack(atk) => {
            if let Some(oid) = *object_id {
                let dir = MirDirection::from_u8(atk.direction);
                if atk.spell != 0 {
                    // 魔法攻击（远程范围指向）
                    player_magic_attack(world, db, oid, dir, atk.spell).await;
                } else {
                    // 基础近战平A
                    player_attack(world, db, oid, dir).await;
                }
            }
        }
        // 远程攻击（弓手，对应 C# MirConnection.RangeAttack -> Player.RangeAttack）
        ClientPacket::RangeAttack(ra) => {
            if let Some(oid) = *object_id {
                let dir = MirDirection::from_u8(ra.direction);
                player_range_attack(world, db, oid, dir, ra.target_id, ra.target_location).await;
            }
        }
        // 采集（割肉，对应 C# PlayerObject.Harvest）
        ClientPacket::Harvest(h) => {
            if let Some(oid) = *object_id {
                if !is_dead(world, Some(oid)).await {
                    let dir = MirDirection::from_u8(h.direction);
                    player_harvest(world, db, oid, dir).await;
                }
            }
        }
        // 开城门（对应 C# PlayerObject.Opendoor：自发 + 广播；门体数据待地图资源接入）
        ClientPacket::Opendoor(od) => {
            if let Some(oid) = *object_id {
                let frame = encode_packet(&s::Opendoor { door_index: od.door_index, close: false });
                tx.send(frame.clone()).await.ok();
                if let Some(p) = world.get_player(oid).await {
                    world.broadcast_on_except(p.map_index, frame, oid).await;
                }
            }
        }
        // 攻击模式（和平/编组/行会/全体…，对应 C# MirConnection.ChangeAMode）
        ClientPacket::ChangeAMode(am) => {
            if let Some(oid) = *object_id {
                {
                    let mut players = world.players.lock().await;
                    if let Some(p) = players.get_mut(&oid) {
                        p.a_mode = am.mode;
                    }
                }
                tx.send(encode_packet(&s::ChangeAMode { mode: am.mode }))
                    .await
                    .ok();
            }
        }
        ClientPacket::ChangePMode(pm) => {
            // 和平模式切换：存储 + 回执（同 C# MirConnection.ChangePMode）
            if let Some(oid) = *object_id {
                {
                    let mut players = world.players.lock().await;
                    if let Some(p) = players.get_mut(&oid) {
                        p.p_mode = pm.mode;
                    }
                }
                tx.send(encode_packet(&s::ChangePMode { mode: pm.mode }))
                    .await
                    .ok();
            }
        }
        ClientPacket::Magic(m) => {
            if let Some(oid) = *object_id {
                let dir = MirDirection::from_u8(m.direction);
                player_magic_attack(world, db, oid, dir, m.spell).await;
            }
        }
        ClientPacket::MagicKey(mk) => {
            // 法术快捷键绑定（会话内有效）：先清掉占用同一按键的其他法术，再绑定
            if let Some(oid) = *object_id {
                let mut players = world.players.lock().await;
                if let Some(p) = players.get_mut(&oid) {
                    if mk.key == 0 {
                        p.magic_keys.remove(&mk.spell);
                    } else {
                        p.magic_keys.retain(|_, k| *k != mk.key);
                        p.magic_keys.insert(mk.spell, mk.key);
                    }
                }
            }
        }
        ClientPacket::PickUp(_) => {
            if let Some(oid) = *object_id {
                if !is_dead(world, Some(oid)).await {
                    pick_up(world, oid, db).await;
                }
            }
        }
        ClientPacket::CallNPC(c) => {
            // 1) 任务 NPC：记录“触碰”，并给出接任务/进度/领奖提示
            let npc_name = npc_name(world, c.object_id).await;
            if let Some(oid) = *object_id {
                let me = world
                    .get_player(oid)
                    .await
                    .map(|p| p.name)
                    .unwrap_or_default();
                if let Some(nn) = &npc_name {
                    let quest_npc = crate::quest::QUESTS.iter().any(|q| q.npc_name == nn.as_str());
                    if quest_npc {
                        world.quest.lock().await.touch(&me, nn);
                        let char_index = world
                            .get_player(oid)
                            .await
                            .map(|p| p.character_index)
                            .unwrap_or(0);
                        let q = world.quest.lock().await.quest_for_touch(&me, &db, char_index);
                        match q {
                            Some(def) => {
                                let progress = db
                                    .load_quest_progress(char_index)
                                    .unwrap_or_default();
                                let mine = progress.iter().find(|p| p.quest_id == def.id);
                                let msg = match mine {
                                    Some(p) if p.completed => format!(
                                        "【{}】: {} —— 任务已完成！用 /quest_reward 领取奖励",
                                        def.name, def.description
                                    ),
                                    Some(p) => {
                                        let target = match def.objective {
                                            crate::quest::QuestObjective::Kill { count, .. } => count,
                                        };
                                        format!(
                                            "【{}】: {} 进度 {}/{}  (继续击杀推进)",
                                            def.name, def.description, p.killed, target
                                        )
                                    }
                                    None => format!(
                                        "【{}】: {} —— /quest_accept 接受任务",
                                        def.name, def.description
                                    ),
                                };
                                tx.send(encode_packet(&s::ObjectChat {
                                    object_id: oid,
                                    text: msg,
                                    r#type: 1,
                                }))
                                .await
                                .ok();
                            }
                            None => {
                                tx.send(encode_packet(&s::ObjectChat {
                                    object_id: oid,
                                    text: format!("{}: 你暂时没有可接/未完成的任务。", nn),
                                    r#type: 1,
                                }))
                                .await
                                .ok();
                            }
                        }
                    }
                }
            }
            // 2) 商人 -> 发送 NPCGoods 商店列表
            if let Some(shop) = npc_shop(world, c.object_id).await {
                let list: Vec<UserItem> = shop
                    .iter()
                    .map(|&idx| UserItem {
                        item_index: idx,
                        count: 1,
                        ..Default::default()
                    })
                    .collect();
                tx.send(encode_packet(&s::NPCGoods {
                    list,
                    rate: 1.0,
                    r#type: 0,
                    hide_added_stats: false,
                }))
                .await
                .ok();
            }
        }
        ClientPacket::BuyItem(b) => {
            if let Some(oid) = *object_id {
                let idx = b.item_index as i32;
                if let Some(tmpl) = items::find(idx) {
                    let cost = tmpl.price.saturating_mul(b.count as u32);
                    if remove_gold(world, oid, cost).await {
                        if let Some(p) = world.get_player(oid).await {
                            let _ = db.add_item_to_inventory(p.character_index, idx, b.count);
                        }
                    }
                }
            }
        }
        ClientPacket::SellItem(si) => {
            if let Some(oid) = *object_id {
                if let Some(p) = world.get_player(oid).await {
                    if let Some(tmpl_idx) =
                        db.remove_from_inventory(p.character_index, si.unique_id)?
                    {
                        if let Some(tmpl) = items::find(tmpl_idx) {
                            let gain = tmpl.price.saturating_mul(si.count as u32) / 2;
                            gain_gold(world, oid, gain).await;
                            // 记录最近出售，供 NPC 回购（含价格与件数）
                            let mut players = world.players.lock().await;
                            if let Some(pl) = players.get_mut(&oid) {
                                if pl.recently_sold.len() >= 20 {
                                    pl.recently_sold.remove(0);
                                }
                                pl.recently_sold.push((si.unique_id, tmpl_idx, si.count, gain));
                            }
                        }
                    }
                }
            }
        }
        ClientPacket::UseItem(u) => {
            handle_use_item(world, db, object_id, u, tx).await?;
        }
        ClientPacket::EquipItem(e) => {
            handle_equip_item(world, db, object_id, e, tx).await?;
        }
        ClientPacket::DropItem(d) => {
            handle_drop_item(world, db, object_id, d, tx).await?;
        }
        ClientPacket::MoveItem(m) => {
            // 背包(1) 或 装备(2) 槽内移动/互换
            if let Some(oid) = *object_id {
                if let Some(p) = world.get_player(oid).await {
                    let ok = if m.grid == GRID_INVENTORY {
                        db.move_inventory_item(p.character_index, m.from, m.to).map(|r| r.0)
                    } else if m.grid == GRID_EQUIPMENT {
                        db.move_equipment_slot(p.character_index, m.from, m.to)
                    } else {
                        Ok(false)
                    };
                    if ok.unwrap_or(false) {
                        send_slots_refresh(world, db, oid, p.character_index, tx).await;
                    }
                }
            }
        }
        ClientPacket::SplitItem(sp) => {
            if let Some(oid) = *object_id {
                if let Some(p) = world.get_player(oid).await {
                    if sp.grid == GRID_INVENTORY {
                        if let Some(slot) =
                            db.split_inventory_item(p.character_index, sp.unique_id, sp.count)?
                        {
                            let _ = slot;
                            send_slots_refresh(world, db, oid, p.character_index, tx).await;
                            send_sys(world, oid, "已拆分堆叠").await;
                        }
                    }
                }
            }
        }
        ClientPacket::StoreItem(st) => {
            // 存入仓库：背包槽 from → 仓库槽 to
            if let Some(oid) = *object_id {
                if let Some(p) = world.get_player(oid).await {
                    // 仓库密码门：未设密码或本会话已解锁才可存入
                    if db.get_storage_pw(p.character_index).is_some() && !p.storage_unlocked {
                        send_sys(world, oid, "需要先解锁仓库密码").await;
                    } else if db.store_item(p.character_index, st.from, st.to).unwrap_or(false) {
                        send_slots_refresh(world, db, oid, p.character_index, tx).await;
                        send_sys(world, oid, &format!("已存入仓库槽 {}", st.to)).await;
                    }
                }
            }
        }
        ClientPacket::TakeBackItem(tb) => {
            // 取出：仓库槽 from → 背包槽 to
            if let Some(oid) = *object_id {
                if let Some(p) = world.get_player(oid).await {
                    // 仓库密码门：未设密码或本会话已解锁才可取出
                    if db.get_storage_pw(p.character_index).is_some() && !p.storage_unlocked {
                        send_sys(world, oid, "需要先解锁仓库密码").await;
                    } else if db.take_item(p.character_index, tb.from, tb.to).unwrap_or(false) {
                        send_slots_refresh(world, db, oid, p.character_index, tx).await;
                        send_sys(world, oid, &format!("已从仓库取出到背包槽 {}", tb.to)).await;
                    }
                }
            }
        }
        ClientPacket::SetStoragePassword(ssp) => {
            if let Some(oid) = *object_id {
                handle_set_storage_password(world, db, oid, ssp.current_password.clone(), ssp.new_password.clone()).await;
            }
        }
        ClientPacket::RemoveStoragePassword(rsp) => {
            if let Some(oid) = *object_id {
                handle_remove_storage_password(world, db, oid, rsp.current_password.clone()).await;
            }
        }
        ClientPacket::UnlockStorage(us) => {
            if let Some(oid) = *object_id {
                handle_unlock_storage(world, db, oid, us.password.clone(), tx).await;
            }
        }
        // 钓鱼
        ClientPacket::FishingCast(fc) => {
            if let Some(oid) = *object_id {
                handle_fishing_cast(world, db, oid, fc.cast_out).await;
            }
        }
        ClientPacket::FishingChangeAutocast(fac) => {
            if let Some(oid) = *object_id {
                handle_fishing_change_autocast(world, oid, fac.auto_cast).await;
            }
        }
        // 精炼
        ClientPacket::CheckRefine(cr) => {
            if let Some(oid) = *object_id {
                handle_check_refine(world, db, oid, cr.unique_id).await;
            }
        }
        ClientPacket::RefineItem(ri) => {
            if let Some(oid) = *object_id {
                handle_refine_item(world, db, oid, ri.unique_id, tx).await;
            }
        }
        ClientPacket::RefineCancel(_) => {
            if let Some(oid) = *object_id {
                handle_refine_cancel(world, oid).await;
            }
        }
        // ------------------------- 面对面交易 -------------------------
        ClientPacket::TradeRequest(_) => {
            if let Some(oid) = *object_id {
                handle_trade_request(world, db, oid).await;
            }
        }
        ClientPacket::TradeReply(tr) => {
            if let Some(oid) = *object_id {
                handle_trade_reply(world, oid, tr.accept_invite).await;
            }
        }
        ClientPacket::TradeGold(tg) => {
            if let Some(oid) = *object_id {
                handle_trade_gold(world, db, oid, tg.amount).await;
            }
        }
        ClientPacket::DepositTradeItem(dt) => {
            if let Some(oid) = *object_id {
                handle_trade_deposit(world, db, oid, dt.from, dt.to).await;
            }
        }
        ClientPacket::RetrieveTradeItem(rt) => {
            if let Some(oid) = *object_id {
                handle_trade_retrieve(world, db, oid, rt.from, rt.to).await;
            }
        }
        ClientPacket::TradeConfirm(tc) => {
            if let Some(oid) = *object_id {
                handle_trade_confirm(world, db, oid, tc.locked).await;
            }
        }
        ClientPacket::TradeCancel(_) => {
            if let Some(oid) = *object_id {
                handle_trade_cancel(world, oid, true).await;
            }
        }
        // ------------------------- 寄售行（Market） -------------------------
        ClientPacket::ConsignItem(ci) => {
            if let Some(oid) = *object_id {
                handle_market_consign(world, db, oid, ci.unique_id, ci.price).await;
            }
        }
        ClientPacket::MarketPage(mp) => {
            if let Some(oid) = *object_id {
                handle_market_page(world, db, oid, mp.page, "").await;
            }
        }
        ClientPacket::MarketSearch(ms) => {
            if let Some(oid) = *object_id {
                handle_market_page(world, db, oid, 0, &ms.r#match).await;
            }
        }
        ClientPacket::MarketRefresh(_) => {
            if let Some(oid) = *object_id {
                handle_market_page(world, db, oid, 0, "").await;
            }
        }
        ClientPacket::MarketBuy(mb) => {
            if let Some(oid) = *object_id {
                handle_market_buy(world, db, oid, mb.auction_id, mb.bid_price).await;
            }
        }
        // 固定一口价市场：立即购买与竞价购买同路径
        ClientPacket::MarketSellNow(msn) => {
            if let Some(oid) = *object_id {
                handle_market_buy(world, db, oid, msn.auction_id, 0).await;
            }
        }
        ClientPacket::MarketGetBack(mg) => {
            if let Some(oid) = *object_id {
                handle_market_get_back(world, db, oid, mg.auction_id).await;
            }
        }
        // ------------------------- 邮件（Mail） -------------------------
        ClientPacket::SendMail(sm) => {
            if let Some(oid) = *object_id {
                handle_send_mail(world, db, oid, sm).await;
            }
        }
        ClientPacket::ReadMail(rm) => {
            if let Some(oid) = *object_id {
                handle_read_mail(world, db, oid, rm.mail_id).await;
            }
        }
        ClientPacket::CollectParcel(cp) => {
            if let Some(oid) = *object_id {
                handle_collect_parcel(world, db, oid, cp.mail_id).await;
            }
        }
        ClientPacket::DeleteMail(dm) => {
            if let Some(oid) = *object_id {
                handle_delete_mail(world, db, oid, dm.mail_id).await;
            }
        }
        ClientPacket::LockMail(lm) => {
            // 锁定/解锁邮件（锁定的邮件不可删除，同 C# LockMail）
            if let Some(oid) = *object_id {
                if let Some(me) = world.get_player(oid).await {
                    match db.set_mail_locked(lm.mail_id as i64, me.character_index, lm.lock) {
                        Ok(true) => {
                            send_sys(
                                world,
                                oid,
                                if lm.lock { "邮件已锁定" } else { "邮件已解锁" },
                            )
                            .await;
                        }
                        _ => send_sys(world, oid, "邮件不存在").await,
                    }
                }
            }
        }
        ClientPacket::MailCost(_) => {
            // 邮资查询：当前邮寄免费（同 C# GetMailCost，费率参数均为 0）
            tx.send(encode_packet(&s::MailCost { cost: 0 })).await.ok();
        }
        ClientPacket::MailLockedItem(_) => {
            // 写信界面的附件锁定切换：纯客户端状态，服务端无需处理
        }
        ClientPacket::SpellToggle(st) => {
            // 战士剑术/心法类 buff 切换（Thusting/HalfMoon 等）：战斗系统未实装，先接受不处理
            let _ = st;
            tracing::debug!("SpellToggle 暂未实装（spell={}）", st.spell);
        }
        // 师徒
        ClientPacket::MentorReply(mr) => {
            if let Some(oid) = *object_id {
                handle_mentor_reply(world, db, oid, mr.accept_invite, tx).await;
            }
        }
        ClientPacket::AllowMentor(_) => {
            if let Some(oid) = *object_id {
                let mut players = world.players.lock().await;
                if let Some(p) = players.get_mut(&oid) {
                    p.can_be_mentor = !p.can_be_mentor;
                    let v = p.can_be_mentor;
                    send_sys(world, oid, if v { "已开启：他人可邀请你收徒" } else { "已关闭：他人无法邀请你收徒" }).await;
                }
            }
        }
        ClientPacket::CancelMentor(_) => {
            if let Some(oid) = *object_id {
                handle_mentor_cancel(world, db, oid, tx).await;
            }
        }
        // 婚姻
        ClientPacket::MarriageRequest(_) => {
            if let Some(oid) = *object_id {
                handle_marriage_prompt(world, oid, tx).await;
            }
        }
        ClientPacket::MarriageReply(mr) => {
            if let Some(oid) = *object_id {
                handle_marriage_reply(world, db, oid, mr.accept_invite, tx).await;
            }
        }
        ClientPacket::DivorceRequest(_) => {
            if let Some(oid) = *object_id {
                handle_divorce(world, db, oid, tx).await;
            }
        }
        // 转生
        ClientPacket::AcceptReincarnation(_) => {
            if let Some(oid) = *object_id {
                handle_accept_reincarnation(world, db, oid, tx).await;
            }
        }
        ClientPacket::CancelReincarnation(_) => {
            // 取消转生：仅确认，无需动作
        }
        // 商城
        ClientPacket::GameshopBuy(gb) => {
            if let Some(oid) = *object_id {
                handle_gameshop_buy(world, db, oid, gb.g_index, gb.quantity, gb.p_type, tx).await;
            }
        }
        ClientPacket::DropGold(dg) => {
            if let Some(oid) = *object_id {
                handle_drop_gold(world, oid, dg.amount).await;
            }
        }
        ClientPacket::RepairItem(ri) => {
            if let Some(oid) = *object_id {
                handle_repair_item(world, db, oid, ri.unique_id, tx).await;
            }
        }
        // 查看玩家：回 PlayerInspect（装备/等级/职业等）
        ClientPacket::Inspect(ins) => {
            if let Some(oid) = *object_id {
                handle_inspect(world, tx, oid, ins.object_id).await;
            }
        }
        // 按名字观察玩家
        ClientPacket::Observe(ob) => {
            if let Some(oid) = *object_id {
                handle_inspect_observe(world, tx, oid, &ob.name).await;
            }
        }
        // 好友
        ClientPacket::AddFriend(af) => {
            if let Some(oid) = *object_id {
                handle_add_friend(world, db, tx, oid, &af.name, af.blocked).await;
            }
        }
        ClientPacket::RemoveFriend(rf) => {
            if let Some(oid) = *object_id {
                if let Some(p) = world.get_player(oid).await {
                    if db.remove_friend(p.character_index, rf.character_index).unwrap_or(false) {
                        send_sys(world, oid, "已移除好友").await;
                    }
                }
                send_friends(world, db, tx, oid).await;
            }
        }
        ClientPacket::RefreshFriends(_) => {
            if let Some(oid) = *object_id {
                send_friends(world, db, tx, oid).await;
            }
        }
        // 信息请求
        ClientPacket::RequestMapInfo(rmi) => {
            if let Some(oid) = *object_id {
                handle_request_map_info(world, tx, oid, rmi.map_index).await;
            }
        }
        ClientPacket::RequestItemInfo(rii) => {
            if let Some(oid) = *object_id {
                handle_request_item_info(tx, oid, rii.item_index).await;
            }
        }
        ClientPacket::RequestUserName(ru) => {
            if let Some(oid) = *object_id {
                let name = db.char_name(ru.user_id as i32).ok().flatten().unwrap_or_default();
                tx.send(encode_packet(&s::UserName { id: ru.user_id, name })).await.ok();
            }
        }
        ClientPacket::RequestNPCInfo(rni) => {
            if let Some(oid) = *object_id {
                handle_request_npc_info(world, tx, oid, rni.npc_index).await;
            }
        }
        // 回城复活（仅死亡状态生效，见 world::revive_player）
        ClientPacket::TownRevive(_) => {
            if let Some(oid) = *object_id {
                crate::world::revive_player(world, oid).await;
            }
        }
        // 自动喝药设置
        ClientPacket::SetAutoPotItem(sp) => {
            if let Some(oid) = *object_id {
                let mut players = world.players.lock().await;
                if let Some(p) = players.get_mut(&oid) {
                    p.auto_pot_item = sp.item_index;
                }
                drop(players);
                send_sys(world, oid, if sp.item_index > 0 { "已设置自动喝药物品" } else { "已关闭自动喝药" }).await;
            }
        }
        ClientPacket::SetAutoPotValue(sv) => {
            if let Some(oid) = *object_id {
                let mut players = world.players.lock().await;
                if let Some(p) = players.get_mut(&oid) {
                    p.auto_pot_hp = sv.value;
                }
                drop(players);
                send_sys(world, oid, &format!("已设置自动喝药血量阈值 {}", sv.value)).await;
            }
        }
        // 回购：把最近出售的物品买回
        ClientPacket::BuyItemBack(bi) => {
            if let Some(oid) = *object_id {
                handle_buy_back(world, db, oid, bi.unique_id, bi.count).await;
            }
        }
        // 信息请求
        ClientPacket::RequestMonsterInfo(rm) => {
            if let Some(oid) = *object_id {
                handle_request_monster_info(tx, oid, rm.monster_index).await;
            }
        }
        ClientPacket::RequestGuildInfo(rgi) => {
            if let Some(oid) = *object_id {
                handle_request_guild_info(world, tx, oid, rgi.r#type).await;
            }
        }
        ClientPacket::SearchMap(sm) => {
            if let Some(oid) = *object_id {
                handle_search_map(world, tx, oid, &sm.text).await;
            }
        }
        ClientPacket::TeleportToNPC(ttn) => {
            if let Some(oid) = *object_id {
                handle_teleport_to_npc(world, oid, ttn.object_id).await;
            }
        }
        // 装备到指定槽 / 合并堆叠
        ClientPacket::EquipSlotItem(esi) => {
            if let Some(oid) = *object_id {
                handle_equip_slot_item(world, db, oid, esi, tx).await;
            }
        }
        ClientPacket::MergeItem(mi) => {
            if let Some(oid) = *object_id {
                if let Some(p) = world.get_player(oid).await {
                    if db.merge_inventory_items(p.character_index, mi.id_from, mi.id_to).unwrap_or((false, 0)).0 {
                        send_slots_refresh(world, db, oid, p.character_index, tx).await;
                        send_sys(world, oid, "已合并堆叠").await;
                    } else {
                        send_sys(world, oid, "合并失败：非同类物品或目标无效").await;
                    }
                }
            }
        }
        ClientPacket::DeleteItem(di) => {
            // 删除背包物品（部分或全部，同 C# PlayerObject.DeleteItem）：回执 S.DeleteItem
            if let Some(oid) = *object_id {
                if let Some(me) = world.get_player(oid).await {
                    let count = if di.count == 0 { 1 } else { di.count };
                    match db.find_inventory_item(me.character_index, di.unique_id) {
                        Ok(Some((_, item))) => {
                            let take = count.min(item.count.max(1));
                            let _ = db.reduce_inventory_item_count(me.character_index, di.unique_id, take);
                            tx.send(encode_packet(&s::DeleteItem {
                                unique_id: di.unique_id,
                                count: take,
                            }))
                            .await
                            .ok();
                            send_slots_refresh(world, db, oid, me.character_index, tx).await;
                        }
                        _ => {
                            tx.send(encode_packet(&s::DeleteItem {
                                unique_id: di.unique_id,
                                count: 0,
                            }))
                            .await
                            .ok();
                        }
                    }
                }
            }
        }
        ClientPacket::Disconnect(_) => {}
        _ => {
            tracing::warn!("未处理的客户端包: {:?}", std::mem::discriminant(packet));
        }
    }
    Ok(())
}

/// 进入世界: 发送地图信息/自身信息(含背包)/位置，广播 ObjectPlayer，返回 object_id
async fn enter_world(
    db: &Database,
    world: &World,
    tx: &mpsc::Sender<Vec<u8>>,
    account_id: &str,
    ch: &crate::db::CharacterRow,
    level: u16,
) -> anyhow::Result<u32> {
    // 地图信息
    tx.send(encode_packet(&s::MapInformation {
        map_index: 0,
        file_name: "0".to_string(),
        title: "新手村".to_string(),
        mini_map: 0,
        big_map: 0,
        lights: 0,
        lightning: false,
        fire: false,
        map_dark_light: 0,
        music: 0,
        weather_particles: 0,
    }))
    .await
    .ok();

    tx.send(encode_packet(&s::NewMapInfo {
        map_index: 0,
        info: crystal_protocol::types::ClientMapInfo {
            title: "新手村".to_string(),
            width: world.map.width as i32,
            height: world.map.height as i32,
            big_map: 0,
            movements: vec![],
            npcs: vec![],
        },
    }))
    .await
    .ok();

    let object_id = world.next_object_id();
    // 职业/等级决定基础属性
    let (base_hp, base_attack) = base_stats(class_from_db(ch.class), level);
    let equipment = db.load_equipment(ch.index)?;
    // 出生点：吸附到最近可行走格子（地图有障碍，存档位置可能落在墙上）
    let (sx, sy) = world.nearest_walkable(ch.x, ch.y);
    let mut player = Player {
        object_id,
        account_id: account_id.to_string(),
        name: ch.name.clone(),
        class: class_from_db(ch.class),
        gender: num_gender(ch.gender),
        level,
        location: Point::new(sx, sy),
        direction: MirDirection::from_u8(ch.direction as u8),
        max_hp: base_hp,
        // 存档血量 ≤0 视为死亡上线（死亡保留跨会话）
        hp: ch.hp.max(0),
        max_mp: 20 + level as i32 * 5,
        mp: ch.mp,
        attack: base_attack,
        defence: level as i32 / 2,
        experience: 0,
        gold: ch.gold as u32,
        weapon: 0,
        armour: 0,
        character_index: ch.index,
        sender: tx.clone(),
        hp_changed: false,
        equipment: equipment.clone(),
        map_index: 0,
        auto_pot_item: 0,
        auto_pot_hp: 0,
        recently_sold: vec![],
        fishing: false,
        fishing_progress: 0,
        auto_fish: false,
        storage_unlocked: false,
        pending_mentor: None,
        can_be_mentor: false,
        pending_marriage: None,
        dead: ch.hp <= 0,
        a_mode: 0,
        allow_group: true,
        p_mode: 0,
        magic_keys: std::collections::HashMap::new(),
    };
    // 穿戴装备后重算攻击/防御
    recompute_stats(&mut player);

    // 加载背包（固定 40 槽）+ 装备（固定 14 槽）
    let inventory = db.inventory_slots(ch.index)?;
    let equipment_slots = equipment_slots(&player);
    // 学会的法术（垂直切片：按职业发放基础法术）
    let magics = crate::magics::player_magics(player.class);

    tx.send(encode_packet(&s::UserInformation {
        object_id,
        real_id: object_id,
        name: player.name.clone(),
        guild_name: String::new(),
        guild_rank: String::new(),
        name_colour: crystal_protocol::binary::Argb(0),
        class: player.class,
        gender: player.gender,
        level: player.level,
        location: player.location,
        direction: player.direction,
        hair: 0,
        hp: player.hp,
        mp: player.mp,
        experience: ch.experience as i64,
        max_experience: 100,
        level_effects: crystal_protocol::types::LevelEffects(0),
        has_hero: false,
        hero_behaviour: 0,
        inventory: Some(inventory),
        equipment: Some(equipment_slots),
        quest_inventory: None,
        gold: ch.gold as u32,
        credit: 0,
        has_expanded_storage: false,
        has_storage_password: false,
        require_storage_password: false,
        storage_password_last_set: 0,
        expanded_storage_expiry_time: 0,
        magics,
        intelligent_creatures: vec![],
        summoned_creature_type: 0,
        creature_summoned: false,
        allow_observe: false,
        observer: false,
    }))
    .await
    .ok();

    tx.send(encode_packet(&s::UserLocation {
        location: player.location,
        direction: player.direction,
    }))
    .await
    .ok();

    tx.send(encode_packet(&s::TimeOfDay { lights: 0 }))
        .await
        .ok();

    world.add_player(player).await;
    // 登录同步攻击模式（同 C# 登录流程 Enqueue S.ChangeAMode）
    if let Some(p) = world.get_player(object_id).await {
        tx.send(encode_packet(&s::ChangeAMode { mode: p.a_mode }))
            .await
            .ok();
        // 同步组队开关状态（同 C# Enqueue S.SwitchGroup）
        tx.send(encode_packet(&s::SwitchGroup {
            allow_group: p.allow_group,
        }))
        .await
        .ok();
    }
    // 下发收件箱（客户端邮件面板数据源）
    send_mailbox(world, db, object_id).await;
    // 死亡上线：补发死亡表现，让客户端进入倒地状态（之后 TownRevive 复活）
    if let Some(p) = world.get_player(object_id).await {
        if p.dead {
            tx.send(encode_packet(&s::Death {
                location: p.location,
                direction: p.direction,
            }))
            .await
            .ok();
        }
    }
    Ok(object_id)
}

/// 玩家是否处于死亡状态（死亡保留期间禁止移动/攻击/拾取）
async fn is_dead(world: &World, object_id: Option<u32>) -> bool {
    match object_id {
        Some(oid) => world.get_player(oid).await.map(|p| p.dead).unwrap_or(false),
        None => false,
    }
}

/// 移动/转身: 校验边界、更新世界、给自己发 UserLocation、广播给他人 Object*
async fn move_player(
    world: &World,
    object_id: &mut Option<u32>,
    tx: &mpsc::Sender<Vec<u8>>,
    direction: MirDirection,
    steps: i32,
) {
    let Some(oid) = *object_id else { return };
    let Some(player) = world.get_player(oid).await else {
        return;
    };

    let Some(new_loc) = world.try_move_on(player.map_index, player.location, direction, steps) else {
        return;
    };

    {
        let mut players = world.players.lock().await;
        if let Some(p) = players.get_mut(&oid) {
            p.location = new_loc;
            p.direction = direction;
        }
    }

    // 自己: UserLocation
    tx.send(encode_packet(&s::UserLocation {
        location: new_loc,
        direction,
    }))
    .await
    .ok();

    // 他人: ObjectWalk / ObjectRun / ObjectTurn
    let frame = if steps == 2 {
        encode_packet(&s::ObjectRun {
            object_id: oid,
            location: new_loc,
            direction,
        })
    } else if steps == 1 {
        encode_packet(&s::ObjectWalk {
            object_id: oid,
            location: new_loc,
            direction,
        })
    } else {
        encode_packet(&s::ObjectTurn {
            object_id: oid,
            location: new_loc,
            direction,
        })
    };
    world.broadcast_except(frame, oid).await;

    // 传送门：踏上传送门所在格即传送到目标地图（仅 Walk/Run，转身不触发）
    if steps > 0 && player.map_index != u32::MAX {
        if let Some((dm, dx, dy)) = world.portal_at(player.map_index, new_loc.x, new_loc.y) {
            tracing::debug!(
                "传送门 {} 玩家@{:?} -> 地图 {dm} ({dx},{dy})",
                oid,
                new_loc
            );
            world.teleport_player(oid, dm, dx, dy).await;
        }
    }
}

/// 向指定玩家发送系统提示（ObjectChat type=1）
async fn send_sys(world: &World, oid: u32, msg: &str) {
    world
        .send_to(
            oid,
            encode_packet(&s::ObjectChat {
                object_id: oid,
                text: msg.to_string(),
                r#type: 1,
            }),
        )
        .await;
}

/// 玩家离队广播（退队/被踢/关邀请/断线共用，同 C# LeaveGroup）：
/// 离队者收 DeleteGroup；剩余 >1 人收 DeleteMember{leaver}；剩 1 人（原队长）收 DeleteGroup。
async fn group_leave_broadcast(world: &World, leaver: &str) {
    let snapshot = {
        let mut g = world.group.lock().await;
        let Some(gid) = g.group_of(leaver) else { return };
        let members = g.members(gid);
        let _ = g.leave(leaver);
        members
    };
    if snapshot.len() < 2 {
        return; // 本就不成队
    }
    if let Some(oid) = player_oid_by_name(world, leaver).await {
        world.send_to(oid, encode_packet(&s::DeleteGroup)).await;
    }
    let rest: Vec<String> = snapshot.into_iter().filter(|m| m != leaver).collect();
    if rest.len() > 1 {
        let frame = encode_packet(&s::DeleteMember { name: leaver.to_string() });
        for m in rest {
            if let Some(oid) = player_oid_by_name(world, &m).await {
                world.send_to(oid, frame.clone()).await;
            }
        }
    } else if rest.len() == 1 {
        // 只剩队长一人：队伍 UI 解散
        if let Some(oid) = player_oid_by_name(world, &rest[0]).await {
            world.send_to(oid, encode_packet(&s::DeleteGroup)).await;
        }
    }
}

/// 按角色名查找玩家 object_id；不在线则 None。
async fn player_oid_by_name(world: &World, name: &str) -> Option<u32> {
    let players = world.players.lock().await;
    players.iter().find(|(_, p)| p.name == name).map(|(oid, _)| *oid)
}

// ---------------------------------------------------------------------------
// 面对面交易（对应 C# PlayerObject.TradeRequest/Reply/Gold/Confirm/Cancel）
// ---------------------------------------------------------------------------

/// 找面前一格的玩家（同图、相邻）。返回 (object_id, 角色名)。
async fn facing_player(world: &World, oid: u32) -> Option<(u32, String)> {
    let me = world.get_player(oid).await?;
    let (dx, dy) = crate::world::direction_offset(me.direction, 1);
    let fx = me.location.x + dx;
    let fy = me.location.y + dy;
    let players = world.players.lock().await;
    players
        .iter()
        .find(|(_, p)| {
            p.object_id != oid && p.map_index == me.map_index
                && p.location.x == fx && p.location.y == fy
        })
        .map(|(toid, p)| (*toid, p.name.clone()))
}

async fn handle_trade_request(world: &World, db: &Database, oid: u32) {
    let me = match world.get_player(oid).await {
        Some(p) => p,
        None => return,
    };
    if me.dead {
        send_sys(world, oid, "死亡状态无法交易").await;
        return;
    }
    // 已在交易中
    let in_trade = world.trade.lock().await.in_trade(&me.name);
    if in_trade {
        send_sys(world, oid, "你已经在交易中").await;
        return;
    }
    // 面前必须有玩家（同 C# FaceToTrade）
    let Some((toid, tname)) = facing_player(world, oid).await else {
        send_sys(world, oid, "请面对要交易的玩家").await;
        return;
    };
    let target = world.get_player(toid).await.unwrap();
    if target.dead {
        send_sys(world, oid, "对方处于死亡状态，无法交易").await;
        return;
    }
    // 对方已在交易中 / 已有邀请
    if world.trade.lock().await.in_trade(&tname) {
        send_sys(world, oid, &format!("{}正在交易中", tname)).await;
        return;
    }
    if !world.trade.lock().await.request(&me.name, &tname) {
        send_sys(world, oid, &format!("{}已有待处理的交易邀请", tname)).await;
        return;
    }
    let _ = db;
    world
        .send_to(toid, encode_packet(&s::TradeRequest { name: me.name.clone() }))
        .await;
}

async fn handle_trade_reply(world: &World, oid: u32, accept: bool) {
    let Some(me) = world.get_player(oid).await else { return };
    if accept {
        // 建立交易会话并取发起者名
        let inviter = {
            let mut mgr = world.trade.lock().await;
            if mgr.accept(&me.name).is_ok() {
                mgr.active_partner_of(&me.name)
            } else {
                None
            }
        };
        let Some(inviter) = inviter else { return };
        let Some(ioid) = player_oid_by_name(world, &inviter).await else {
            // 发起者已掉线：取消
            world.trade.lock().await.cancel(&me.name);
            return;
        };
        world
            .send_to(oid, encode_packet(&s::TradeAccept { name: inviter.clone() }))
            .await;
        world
            .send_to(ioid, encode_packet(&s::TradeAccept { name: me.name.clone() }))
            .await;
    } else {
        // 拒绝：移除 pending 并通知发起者
        let inviter = world.trade.lock().await.reject(&me.name);
        if let Some(inviter) = inviter {
            if let Some(ioid) = player_oid_by_name(world, &inviter).await {
                send_sys(world, ioid, &format!("{}拒绝了你的交易请求", me.name)).await;
            }
        }
    }
}

async fn handle_trade_gold(world: &World, db: &Database, oid: u32, amount: u32) {
    let _ = db;
    let Some(me) = world.get_player(oid).await else { return };
    if amount == 0 || me.gold < amount {
        return;
    }
    // 累计放入不能超过持有金币
    let already = world.trade.lock().await.side_gold(&me.name);
    if already + amount > me.gold {
        return;
    }
    if world.trade.lock().await.add_gold(&me.name, amount).is_err() {
        return;
    }
    let total = world.trade.lock().await.side_gold(&me.name);
    // 通知对方累计金额（同 C# S.TradeGold{Amount=累计}）
    if let Some(partner) = trade_partner_oid(world, &me.name).await {
        world
            .send_to(partner, encode_packet(&s::TradeGold { amount: total }))
            .await;
    }
}

async fn handle_trade_deposit(world: &World, db: &Database, oid: u32, from: i32, to: i32) {
    let Some(me) = world.get_player(oid).await else { return };
    // 背包槽位 -> unique_id
    let uid = match db.inventory_slots(me.character_index) {
        Ok(slots) => slots
            .get(from as usize)
            .and_then(|o| o.as_ref())
            .map(|it| it.unique_id),
        Err(_) => None,
    };
    let Some(uid) = uid else {
        tx_send(world, oid, &s::DepositTradeItem { from, to, success: false }).await;
        return;
    };
    let ok = world.trade.lock().await.add_item(&me.name, uid).is_ok();
    tx_send(
        world,
        oid,
        &s::DepositTradeItem { from, to, success: ok },
    )
    .await;
    if ok {
        send_trade_items_to_partner(world, db, &me.name).await;
    }
}

async fn handle_trade_retrieve(world: &World, db: &Database, oid: u32, from: i32, to: i32) {
    let Some(me) = world.get_player(oid).await else { return };
    // from = 已放入列表的下标（放入顺序）
    let removed = if from >= 0 {
        world.trade.lock().await.remove_item_at(&me.name, from as usize)
    } else {
        None
    };
    tx_send(
        world,
        oid,
        &s::RetrieveTradeItem { from, to, success: removed.is_some() },
    )
    .await;
    if removed.is_some() {
        send_trade_items_to_partner(world, db, &me.name).await;
    }
}

async fn handle_trade_confirm(world: &World, db: &Database, oid: u32, locked: bool) {
    let Some(me) = world.get_player(oid).await else { return };
    if !locked {
        // 取消锁定：只解锁自己（对方保持），下次改动再联动解锁
        let _ = world.trade.lock().await.confirm_cancel(&me.name);
        return;
    }
    if world.trade.lock().await.confirm(&me.name).is_err() {
        return;
    }
    // 对方未锁定：提示等待
    let both_locked = {
        let mgr = world.trade.lock().await;
        mgr.both_locked(&me.name)
    };
    if !both_locked {
        if let Some(partner) = trade_partner_name(world, &me.name).await {
            if let Some(poid) = player_oid_by_name(world, &partner).await {
                send_sys(world, poid, &format!("{}已确认交易，等待你确认", me.name)).await;
            }
        }
        return;
    }
    // 双方已锁定 -> 结算
    let settle = world.trade.lock().await.complete(&me.name).ok().flatten();
    let Some(settle) = settle else { return };
    execute_trade_settle(world, db, settle).await;
}

/// 结算中的物品转移：uid 从 from 背包移除，按模板重新入包到 to。
async fn transfer_trade_items(
    world: &World,
    db: &Database,
    uids: &[u64],
    from_char: i32,
    to_char: i32,
    to_oid: u32,
) {
    for uid in uids {
        let item = db
            .find_inventory_item(from_char, *uid)
            .ok()
            .flatten()
            .map(|(_, it)| it);
        let Some(item) = item else { continue };
        if db
            .remove_from_inventory(from_char, *uid)
            .ok()
            .flatten()
            .is_none()
        {
            continue;
        }
        if db
            .add_item_to_inventory(to_char, item.item_index, item.count)
            .unwrap_or(false)
        {
            world
                .send_to(to_oid, encode_packet(&s::GainedItem { item }))
                .await;
        }
    }
}

async fn handle_trade_cancel(world: &World, oid: u32, notify_self: bool) {
    let Some(me) = world.get_player(oid).await else { return };
    let partner = trade_partner_name(world, &me.name).await;
    let cancelled = world.trade.lock().await.cancel(&me.name);
    if !cancelled {
        return;
    }
    // 物品从未离开背包、金币未预扣，无需返还
    if let Some(pname) = partner {
        if let Some(poid) = player_oid_by_name(world, &pname).await {
            world
                .send_to(poid, encode_packet(&s::TradeCancel { unlock: true }))
                .await;
            send_sys(world, poid, "对方取消了交易").await;
        }
    }
    if notify_self {
        world
            .send_to(oid, encode_packet(&s::TradeCancel { unlock: true }))
            .await;
    }
}

/// 把某方已放入的交易物品列表发给其交易对象（同 C# S.TradeItem 全量刷新）
async fn send_trade_items_to_partner(world: &World, db: &Database, who: &str) {
    let uids = world.trade.lock().await.side_items(who);
    let char_index = match player_oid_by_name(world, who).await {
        Some(o) => world.get_player(o).await.map(|p| p.character_index),
        None => None,
    };
    let Some(char_index) = char_index else { return };
    let mut items: Vec<Option<UserItem>> = Vec::new();
    for uid in uids {
        let it = db
            .find_inventory_item(char_index, uid)
            .ok()
            .flatten()
            .map(|(_, item)| item);
        items.push(it);
    }
    if let Some(partner) = trade_partner_oid(world, who).await {
        world
            .send_to(
                partner,
                encode_packet(&s::TradeItem { trade_items: items }),
            )
            .await;
    }
}

/// 双方确认后的结算：转移物品与金币，通知双方。
async fn execute_trade_settle(world: &World, db: &Database, settle: crate::trade::Settle) {
    let a_oid = player_oid_by_name(world, &settle.a).await;
    let b_oid = player_oid_by_name(world, &settle.b).await;
    let (Some(a_oid), Some(b_oid)) = (a_oid, b_oid) else {
        return; // 结算时必须双方在线（正常流程保证）
    };
    let a_char = world.get_player(a_oid).await.map(|p| p.character_index);
    let b_char = world.get_player(b_oid).await.map(|p| p.character_index);
    let (Some(a_char), Some(b_char)) = (a_char, b_char) else {
        return;
    };

    // 物品交换（uid 从卖方背包移除，按模板重新入包到买方）
    transfer_trade_items(world, db, &settle.a_items_to_b, a_char, b_char, b_oid).await;
    transfer_trade_items(world, db, &settle.b_items_to_a, b_char, a_char, a_oid).await;

    // 金币交换
    if settle.a_gold_to_b > 0 && remove_gold(world, a_oid, settle.a_gold_to_b).await {
        gain_gold(world, b_oid, settle.a_gold_to_b).await;
    }
    if settle.b_gold_to_a > 0 && remove_gold(world, b_oid, settle.b_gold_to_a).await {
        gain_gold(world, a_oid, settle.b_gold_to_a).await;
    }

    // 完成表现
    world.send_to(a_oid, encode_packet(&s::TradeConfirm)).await;
    world.send_to(b_oid, encode_packet(&s::TradeConfirm)).await;
    send_sys(world, a_oid, "交易成功").await;
    send_sys(world, b_oid, "交易成功").await;
}

/// 某玩家当前交易对象的名字
async fn trade_partner_name(world: &World, who: &str) -> Option<String> {
    let mgr = world.trade.lock().await;
    mgr.active_partner_of(who)
}

/// 某玩家当前交易对象的 object_id
async fn trade_partner_oid(world: &World, who: &str) -> Option<u32> {
    let partner = trade_partner_name(world, who).await?;
    player_oid_by_name(world, &partner).await
}

/// 给指定玩家直接发送一个服务器包（内部小工具）
async fn tx_send<T: crystal_protocol::frame::PacketCodec>(world: &World, oid: u32, pkt: &T) {
    world.send_to(oid, encode_packet(pkt)).await;
}

// ---------------------------------------------------------------------------
// 寄售行（Market）：挂单内存态（重启清空），物品在成交/取回时才真正转移，
// remove_from_inventory 的原子性保证不会重复出售。
// ---------------------------------------------------------------------------

/// 寄售行每页条数
const MARKET_PAGE_SIZE: usize = 10;

/// 挂单：把背包物品上架。物品暂留背包，成交时才转移。
async fn handle_market_consign(
    world: &World,
    db: &Database,
    oid: u32,
    unique_id: u64,
    price: u32,
) {
    let Some(me) = world.get_player(oid).await else { return };
    // 物品必须在背包中
    let owned = db
        .find_inventory_item(me.character_index, unique_id)
        .ok()
        .flatten()
        .is_some();
    let success = owned && world.market.lock().await.list(&me.name, unique_id, price).is_ok();
    tx_send(world, oid, &s::ConsignItem { unique_id, success }).await;
    if success {
        send_sys(world, oid, &format!("物品已上架，要价 {} 金币", price)).await;
    }
}

/// 构建一个挂单的客户端展示结构
async fn market_order_to_auction(
    db: &Database,
    order: &crate::market::MarketOrder,
) -> Option<crystal_protocol::types::ClientAuction> {
    use crystal_protocol::types::ClientAuction;
    let seller_char = db.char_index_by_name(&order.seller).ok().flatten()?;
    let (_, item) = db.find_inventory_item(seller_char, order.item_uid).ok().flatten()?;
    Some(ClientAuction {
        auction_id: order.order_id,
        item,
        seller: order.seller.clone(),
        price: order.price,
        consignment_date: order.listed_at,
        item_type: 0,
    })
}

/// 挂单物品名（用于搜索过滤；查不到返回空）
async fn market_order_item_name(db: &Database, order: &crate::market::MarketOrder) -> String {
    let Some(seller_char) = db.char_index_by_name(&order.seller).ok().flatten() else {
        return String::new();
    };
    let Some((_, item)) = db
        .find_inventory_item(seller_char, order.item_uid)
        .ok()
        .flatten()
    else {
        return String::new();
    };
    crate::items::find(item.item_index)
        .map(|t| t.name.to_string())
        .unwrap_or_default()
}

/// 市场列表（page 从 0 起；`name_filter` 非空时按物品名模糊过滤）
async fn handle_market_page(world: &World, db: &Database, oid: u32, page: i32, name_filter: &str) {
    use crystal_protocol::types::ClientAuction;
    let filter = name_filter.to_lowercase();
    let orders: Vec<crate::market::MarketOrder> = {
        let mgr = world.market.lock().await;
        mgr.all_orders()
    };
    let mut auctions: Vec<ClientAuction> = Vec::new();
    for o in &orders {
        if !filter.is_empty() {
            let name = market_order_item_name(db, o).await;
            if !name.to_lowercase().contains(&filter) {
                continue;
            }
        }
        if let Some(a) = market_order_to_auction(db, o).await {
            auctions.push(a);
        }
    }
    let pages = auctions.len().div_ceil(MARKET_PAGE_SIZE) as i32;
    let start = (page.max(0) as usize) * MARKET_PAGE_SIZE;
    let slice: Vec<ClientAuction> = auctions
        .into_iter()
        .skip(start)
        .take(MARKET_PAGE_SIZE)
        .collect();
    tx_send(
        world,
        oid,
        &s::NPCMarket {
            listings: slice,
            pages,
            user_mode: false,
        },
    )
    .await;
}

/// 购买挂单
async fn handle_market_buy(
    world: &World,
    db: &Database,
    oid: u32,
    auction_id: u64,
    bid_price: u32,
) {
    let Some(me) = world.get_player(oid).await else { return };
    if me.dead {
        tx_send(world, oid, &s::MarketFail { reason: 0 }).await; // 已死
        return;
    }
    let order = world.market.lock().await.orders.get(&auction_id).cloned();
    let Some(order) = order else {
        tx_send(world, oid, &s::MarketFail { reason: 2 }).await; // 已售/不存在
        return;
    };
    if order.seller == me.name {
        tx_send(world, oid, &s::MarketFail { reason: 6 }).await; // 不能买自己的
        return;
    }
    if bid_price < order.price || me.gold < order.price {
        tx_send(world, oid, &s::MarketFail { reason: 4 }).await; // 金币不足
        return;
    }
    // 卖家角色（可离线）
    let Some(seller_char) = db.char_index_by_name(&order.seller).ok().flatten() else {
        tx_send(world, oid, &s::MarketFail { reason: 2 }).await;
        return;
    };
    // 原子取物：从卖家背包移除（防重复出售）
    let item = db
        .find_inventory_item(seller_char, order.item_uid)
        .ok()
        .flatten()
        .map(|(_, it)| it);
    let Some(item) = item else {
        tx_send(world, oid, &s::MarketFail { reason: 2 }).await; // 已售
        return;
    };
    if db
        .remove_from_inventory(seller_char, order.item_uid)
        .ok()
        .flatten()
        .is_none()
    {
        tx_send(world, oid, &s::MarketFail { reason: 2 }).await;
        return;
    }
    // 移除挂单 + 转金币
    if world.market.lock().await.buy(auction_id).is_err() {
        // 理论不可达（上面刚查过）；回滚物品
        let _ = db.add_item_to_inventory(seller_char, item.item_index, item.count);
        tx_send(world, oid, &s::MarketFail { reason: 2 }).await;
        return;
    }
    let _ = db.add_char_gold(seller_char, order.price as i64);
    // 卖家在线则同步世界侧金币并提示
    if let Some(soid) = player_oid_by_name(world, &order.seller).await {
        gain_gold(world, soid, order.price).await;
        send_sys(world, soid, &format!("{} 购买了你的寄售物品，+{} 金币", me.name, order.price)).await;
    }
    // 买家扣金 + 入包
    remove_gold(world, oid, order.price).await;
    if db
        .add_item_to_inventory(me.character_index, item.item_index, item.count)
        .unwrap_or(false)
    {
        tx_send(world, oid, &s::GainedItem { item }).await;
    }
    tx_send(
        world,
        oid,
        &s::MarketSuccess { message: "购买成功".to_string() },
    )
    .await;
}

/// 取回自己的挂单（物品一直在背包，仅撤单）
async fn handle_market_get_back(world: &World, db: &Database, oid: u32, auction_id: u64) {
    let _ = db;
    let Some(me) = world.get_player(oid).await else { return };
    let is_mine = world
        .market
        .lock()
        .await
        .orders
        .get(&auction_id)
        .map(|o| o.seller == me.name)
        .unwrap_or(false);
    if !is_mine {
        tx_send(world, oid, &s::MarketFail { reason: 2 }).await;
        return;
    }
    if world.market.lock().await.cancel(auction_id).is_ok() {
        tx_send(
            world,
            oid,
            &s::MarketSuccess { message: "已取回寄售物品".to_string() },
        )
        .await;
    } else {
        tx_send(world, oid, &s::MarketFail { reason: 2 }).await;
    }
}

// ---------------------------------------------------------------------------
// 邮件（Mail）：站内信 + 金币/单件物品附件。
// 附件在寄出时即从寄件人背包移除（防复制），领取时入收件人背包；
// 收件人可离线，金币直接落库（add_char_gold）。
// ---------------------------------------------------------------------------

/// MailSent 结果码：0 成功 / 1 收件人不存在 / 2 不能寄给自己 / 3 金币不足 / 4 附件无效
async fn handle_send_mail(world: &World, db: &Database, oid: u32, sm: &c::SendMail) {
    let Some(me) = world.get_player(oid).await else { return };
    // 收件人校验
    if sm.name == me.name {
        tx_send(world, oid, &s::MailSent { result: 2 }).await;
        return;
    }
    let Some(to_char) = db.char_index_by_name(&sm.name).ok().flatten() else {
        tx_send(world, oid, &s::MailSent { result: 1 }).await;
        return;
    };
    // 金币附件
    if sm.gold > 0 && !remove_gold(world, oid, sm.gold).await {
        tx_send(world, oid, &s::MailSent { result: 3 }).await;
        return;
    }
    // 物品附件：取第一个非空槽位（数据模型一封一附件）
    let first_uid = sm.items_idx.iter().find(|&&u| u != 0).copied();
    let attached: Option<crystal_protocol::types::UserItem> = match first_uid {
        None => None,
        Some(uid) => {
            let item = db
                .find_inventory_item(me.character_index, uid)
                .ok()
                .flatten()
                .map(|(_, it)| it);
            let Some(item) = item else {
                // 附件无效：回滚金币并失败
                if sm.gold > 0 {
                    gain_gold(world, oid, sm.gold).await;
                }
                tx_send(world, oid, &s::MailSent { result: 4 }).await;
                return;
            };
            if db
                .remove_from_inventory(me.character_index, uid)
                .ok()
                .flatten()
                .is_none()
            {
                if sm.gold > 0 {
                    gain_gold(world, oid, sm.gold).await;
                }
                tx_send(world, oid, &s::MailSent { result: 4 }).await;
                return;
            }
            Some(item)
        }
    };
    // 落库
    let mail_id = match db.send_mail(to_char, &me.name, "", &sm.message, sm.gold as i64, 0) {
        Ok(id) => id,
        Err(_) => {
            if sm.gold > 0 {
                gain_gold(world, oid, sm.gold).await;
            }
            tx_send(world, oid, &s::MailSent { result: 1 }).await;
            return;
        }
    };
    if let Some(item) = &attached {
        let _ = db.attach_mail_item(mail_id, item.item_index, item.count);
    }
    tx_send(world, oid, &s::MailSent { result: 0 }).await;
    send_sys(world, oid, &format!("邮件已寄给 {}", sm.name)).await;
    // 收件人在线则提示
    if let Some(toid) = player_oid_by_name(world, &sm.name).await {
        send_sys(world, toid, "收到新邮件").await;
    }
}

/// 构建客户端邮件结构（含附件与已领取状态）
async fn build_client_mail(
    world: &World,
    db: &Database,
    m: &crate::db::Mail,
) -> Option<crystal_protocol::types::ClientMail> {
    use crystal_protocol::types::{ClientMail, UserItem};
    let _ = world;
    let attachment = db.mail_attachment(m.id).ok().flatten();
    let collected = m.gold == 0 && attachment.is_none();
    let items = attachment
        .map(|(item_index, count)| {
            vec![UserItem {
                unique_id: 0,
                item_index,
                count,
                ..Default::default()
            }]
        })
        .unwrap_or_default();
    Some(ClientMail {
        mail_id: m.id as u64,
        sender_name: m.from_name.clone(),
        message: m.body.clone(),
        opened: m.is_read,
        locked: m.locked,
        can_reply: false,
        collected,
        date_sent: m.created_at,
        gold: m.gold.max(0) as u32,
        items,
    })
}

/// 下发整个收件箱
async fn send_mailbox(world: &World, db: &Database, oid: u32) {
    let Some(me) = world.get_player(oid).await else { return };
    let mails = db.mail_inbox(me.character_index).unwrap_or_default();
    let mut list = Vec::new();
    for m in &mails {
        if let Some(cm) = build_client_mail(world, db, m).await {
            list.push(cm);
        }
    }
    tx_send(world, oid, &s::ReceiveMail { mail: list }).await;
}

async fn handle_read_mail(world: &World, db: &Database, oid: u32, mail_id: u64) {
    let Some(me) = world.get_player(oid).await else { return };
    let mail = db
        .get_mail(mail_id as i64, me.character_index)
        .ok()
        .flatten();
    let Some(m) = mail else { return };
    let _ = db.mark_mail_read(m.id);
    let m = crate::db::Mail { is_read: true, ..m };
    if let Some(cm) = build_client_mail(world, db, &m).await {
        tx_send(world, oid, &s::ReceiveMail { mail: vec![cm] }).await;
    }
}

async fn handle_collect_parcel(world: &World, db: &Database, oid: u32, mail_id: u64) {
    let Some(me) = world.get_player(oid).await else { return };
    let mail = db
        .get_mail(mail_id as i64, me.character_index)
        .ok()
        .flatten();
    let Some(m) = mail else {
        tx_send(world, oid, &s::ParcelCollected { result: 1 }).await;
        return;
    };
    // 金币附件
    if m.gold > 0 {
        match db.claim_mail_gold(m.id) {
            Ok(gold) if gold > 0 => gain_gold(world, oid, gold as u32).await,
            _ => {}
        }
    }
    // 物品附件
    if let Ok(Some((item_index, count))) = db.claim_mail_attachment(m.id) {
        if db
            .add_item_to_inventory(me.character_index, item_index, count)
            .unwrap_or(false)
        {
            tx_send(
                world,
                oid,
                &s::GainedItem {
                    item: crystal_protocol::types::UserItem {
                        unique_id: 0,
                        item_index,
                        count,
                        ..Default::default()
                    },
                },
            )
            .await;
        }
    }
    tx_send(world, oid, &s::ParcelCollected { result: 0 }).await;
}

async fn handle_delete_mail(world: &World, db: &Database, oid: u32, mail_id: u64) {
    let Some(me) = world.get_player(oid).await else { return };
    let mail = db
        .get_mail(mail_id as i64, me.character_index)
        .ok()
        .flatten();
    let Some(m) = mail else { return };
    // 有未领取附件不允许删除（同 C# 语义）
    let has_attachment = m.gold > 0 || db.mail_attachment(m.id).ok().flatten().is_some();
    if has_attachment {
        send_sys(world, oid, "请先领取邮件附件").await;
        return;
    }
    // 锁定的邮件不允许删除
    if m.locked {
        send_sys(world, oid, "邮件已锁定，无法删除").await;
        return;
    }
    let _ = db.delete_mail(m.id);
}

// ---------------------------------------------------------------------------
// 1) 仓库密码
// ---------------------------------------------------------------------------

async fn handle_set_storage_password(
    world: &World,
    db: &Database,
    oid: u32,
    current_password: String,
    new_password: String,
) {
    let Some(p) = world.get_player(oid).await else { return };
    let c = p.character_index;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let existing = db.get_storage_pw(c);
    let (result, removing, has_password, last_set_time);
    match existing {
        None => {
            // 未设过：当前密码必须为空才接受
            if current_password.is_empty() {
                let h = crate::db::hash_password(&new_password);
                let _ = db.set_storage_pw(c, &h, now);
                result = 1;
                removing = false;
                has_password = true;
                last_set_time = now;
            } else {
                result = 2;
                removing = false;
                has_password = false;
                last_set_time = 0;
            }
        }
        Some((hash, set_at)) => {
            if hash == crate::db::hash_password(&current_password) {
                // 校验旧密码通过，改新密码
                let h = crate::db::hash_password(&new_password);
                let _ = db.set_storage_pw(c, &h, now);
                result = 1;
                removing = false;
                has_password = true;
                last_set_time = now;
            } else {
                result = 2;
                removing = false;
                has_password = true;
                last_set_time = set_at;
            }
        }
    }
    world.send_to(oid, encode_packet(&s::StoragePasswordResult {
        result,
        removing,
        has_password,
        last_set_time,
    })).await;
}

async fn handle_remove_storage_password(
    world: &World,
    db: &Database,
    oid: u32,
    current_password: String,
) {
    let Some(p) = world.get_player(oid).await else { return };
    let c = p.character_index;
    let (result, removing, has_password, last_set_time) = match db.get_storage_pw(c) {
        Some((hash, _)) if hash == crate::db::hash_password(&current_password) => {
            let _ = db.clear_storage_pw(c);
            (1u8, true, false, 0i64)
        }
        Some((_, set_at)) => (2u8, false, true, set_at),
        None => (2u8, false, false, 0i64),
    };
    world.send_to(oid, encode_packet(&s::StoragePasswordResult {
        result, removing, has_password, last_set_time,
    })).await;
}

async fn handle_unlock_storage(
    world: &World,
    db: &Database,
    oid: u32,
    password: String,
    tx: &mpsc::Sender<Vec<u8>>,
) {
    let Some(p) = world.get_player(oid).await else { return };
    let c = p.character_index;
    let (result, has_password) = match db.get_storage_pw(c) {
        Some((hash, _)) if hash == crate::db::hash_password(&password) => (1u8, true),
        Some(_) => (2u8, true),
        None => (1u8, false),
    };
    if result == 1 {
        // 解锁成功后设置本会话解锁标记
        let mut players = world.players.lock().await;
        if let Some(pl) = players.get_mut(&oid) {
            pl.storage_unlocked = true;
        }
        drop(players);
    }
    tx.send(encode_packet(&s::StorageUnlockResult { result, has_password })).await.ok();
}

// ---------------------------------------------------------------------------
// 2) 钓鱼
// ---------------------------------------------------------------------------

async fn handle_fishing_cast(world: &World, db: &Database, oid: u32, cast_out: bool) {
    {
        let mut players = world.players.lock().await;
        if let Some(p) = players.get_mut(&oid) {
            p.fishing = cast_out;
            p.fishing_progress = 0;
        }
    }
    let loc = world.get_player(oid).await.map(|p| p.location).unwrap_or(crystal_protocol::binary::Point::new(0, 0));
    world.send_to(oid, encode_packet(&s::FishingUpdate {
        object_id: oid,
        fishing: cast_out,
        progress_percent: 0,
        chance_percent: 30,
        fishing_point: loc,
        found_fish: false,
    })).await;
    let _ = db; // 预留：di保卫钓到的鱼入库在 fishing_tick 中处理
}

async fn handle_fishing_change_autocast(world: &World, oid: u32, auto_cast: bool) {
    let mut players = world.players.lock().await;
    if let Some(p) = players.get_mut(&oid) {
        p.auto_fish = auto_cast;
        // 关闭自动抛竿：若正在钓鱼则停止
        if !auto_cast {
            p.fishing = false;
            p.fishing_progress = 0;
        }
    }
    drop(players);
    let loc = world.get_player(oid).await.map(|p| p.location).unwrap_or(crystal_protocol::binary::Point::new(0, 0));
    world.send_to(oid, encode_packet(&s::FishingUpdate {
        object_id: oid,
        fishing: false,
        progress_percent: 0,
        chance_percent: 0,
        fishing_point: loc,
        found_fish: false,
    })).await;
}

// ---------------------------------------------------------------------------
// 3) 精炼
// ---------------------------------------------------------------------------

async fn handle_check_refine(world: &World, db: &Database, oid: u32, unique_id: u64) {
    let Some(p) = world.get_player(oid).await else { return };
    let c = p.character_index;
    let cur = refine_of(db, c, unique_id);
    let (rate, _cost) = Database::next_refine(cur);
    world.send_to(oid, encode_packet(&s::NPCRefine { rate, refining: true })).await;
}

async fn handle_refine_item(
    world: &World,
    db: &Database,
    oid: u32,
    unique_id: u64,
    tx: &mpsc::Sender<Vec<u8>>,
) {
    let Some(p) = world.get_player(oid).await else { return };
    let c = p.character_index;
    // 定位物品：先装备表后背包表
    let (table, cur_refines) = locate_item_refine(db, c, unique_id);
    let Some(table) = table else {
        send_sys(world, oid, "未找到要精炼的物品").await;
        return;
    };
    let (rate, cost) = Database::next_refine(cur_refines);
    // 扣金币
    if !remove_gold(world, oid, cost).await {
        send_sys(world, oid, &format!("金币不足，精炼需要 {} 金币", cost)).await;
        world.send_to(oid, encode_packet(&s::NPCRefine { rate: 0.0, refining: false })).await;
        return;
    }
    use rand::Rng;
    let roll: f32 = rand::thread_rng().gen::<f32>();
    let success = roll < rate;
    if success {
        let _ = db.set_refines(&table, c, unique_id, cur_refines + 1);
        world.send_to(oid, encode_packet(&s::NPCRefine { rate, refining: false })).await;
        send_sys(world, oid, &format!("精炼成功！({} -> {})", cur_refines, cur_refines + 1)).await;
    } else {
        world.send_to(oid, encode_packet(&s::NPCRefine { rate: 0.0, refining: false })).await;
        send_sys(world, oid, "精炼失败").await;
    }
    // 若装备被精炼且已穿戴，重算属性
    if table == "equipment" {
        let mut players = world.players.lock().await;
        if let Some(pl) = players.get_mut(&oid) {
            recompute_stats(pl);
        }
        drop(players);
        let p = world.get_player(oid).await;
        if let Some(p) = p {
            broadcast_player(world, &p).await;
        }
    }
    send_slots_refresh(world, db, oid, c, tx).await;
}

async fn handle_refine_cancel(world: &World, oid: u32) {
    world.send_to(oid, encode_packet(&s::NPCRefine { rate: 0.0, refining: false })).await;
}

/// 返回 (表名, 当前精炼值)；物品不存在则 (None, 0)。
fn locate_item_refine(db: &Database, c: i32, unique_id: u64) -> (Option<String>, u32) {
    // 装备表
    let equip = db.read_refines("equipment", c, unique_id);
    // 判断该物品是否确实在装备表中（read_refines 对不存在的行返回 0，需再验证存在性）
    let equip_exists = db.find_equipment_by_uid(c, unique_id);
    if equip_exists {
        return (Some("equipment".to_string()), equip);
    }
    let inv_exists = db.find_inventory_item(c, unique_id).ok().flatten().is_some();
    if inv_exists {
        return (Some("inventory".to_string()), db.read_refines("inventory", c, unique_id));
    }
    (None, 0)
}

fn refine_of(db: &Database, c: i32, unique_id: u64) -> u32 {
    let (table, cur) = locate_item_refine(db, c, unique_id);
    if table.is_some() { cur } else { 0 }
}

// ---------------------------------------------------------------------------
// 4) 师徒
// ---------------------------------------------------------------------------

async fn handle_mentor_reply(
    world: &World,
    db: &Database,
    oid: u32,
    accept: bool,
    _tx: &mpsc::Sender<Vec<u8>>,
) {
    let Some(p) = world.get_player(oid).await else { return };
    let mentee_name = p.name.clone();
    let mentor_name = match p.pending_mentor.clone() {
        Some(n) => n,
        None => {
            send_sys(world, oid, "当前没有待确认的收徒邀请").await;
            return;
        }
    };
    let Some(mentor_oid) = player_oid_by_name(world, &mentor_name).await else {
        send_sys(world, oid, &format!("师父 {mentor_name} 不在线")).await;
        return;
    };
    // 清除待确认
    {
        let mut players = world.players.lock().await;
        if let Some(pl) = players.get_mut(&oid) {
            pl.pending_mentor = None;
        }
    }
    if !accept {
        send_sys(world, oid, &format!("已拒绝 {mentor_name} 的收徒邀请")).await;
        world.send_to(mentor_oid, encode_packet(&s::ObjectChat {
            object_id: mentor_oid,
            text: format!("{mentee_name} 拒绝了你的收徒邀请"),
            r#type: 1,
        })).await;
        return;
    }
    // 建立师徒关系（mentee=oid, mentor=mentor_oid）
    let mentee_char = p.character_index;
    let Some(mentor) = world.get_player(mentor_oid).await else { return };
    let mentor_char = mentor.character_index;
    let _ = db.set_mentor(mentee_char, mentor_char);
    // 双方发 MentorUpdate
    let mentee_level = world.get_player(oid).await.map(|x| x.level).unwrap_or(1);
    let mentor_level = world.get_player(mentor_oid).await.map(|x| x.level).unwrap_or(1);
    world.send_to(oid, encode_packet(&s::MentorUpdate {
        name: mentor_name.clone(),
        level: mentor_level,
        online: true,
        mentee_exp: 0,
    })).await;
    world.send_to(mentor_oid, encode_packet(&s::MentorUpdate {
        name: mentee_name.clone(),
        level: mentee_level,
        online: true,
        mentee_exp: 0,
    })).await;
    send_sys(world, oid, &format!("✓ 已拜 {mentor_name} 为师")).await;
    send_sys(world, mentor_oid, &format!("✓ {mentee_name} 已成为你的徒弟")).await;
}

async fn handle_mentor_cancel(
    world: &World,
    db: &Database,
    oid: u32,
    tx: &mpsc::Sender<Vec<u8>>,
) {
    let Some(p) = world.get_player(oid).await else { return };
    let c = p.character_index;
    db.clear_mentor(c).ok();
    tx.send(encode_packet(&s::MentorUpdate {
        name: String::new(),
        level: 0,
        online: false,
        mentee_exp: 0,
    })).await.ok();
    send_sys(world, oid, "已解除师徒关系").await;
}

/// /mentor <师父名>：向指定玩家发出收徒邀请（要求对方已开启 can_be_mentor）。
async fn chat_mentor_request(world: &World, oid: u32, target: &str, tx: &mpsc::Sender<Vec<u8>>) {
    let Some(p) = world.get_player(oid).await else { return };
    let my_name = p.name.clone();
    let target_oid = match player_oid_by_name(world, target).await {
        Some(o) => o,
        None => {
            let _ = tx.send(encode_packet(&s::ObjectChat {
                object_id: oid, text: format!("玩家 {target} 不在线"), r#type: 1,
            })).await;
            return;
        }
    };
    let target_player = world.get_player(target_oid).await;
    if let Some(tp) = target_player.clone() {
        if tp.name == my_name {
            let _ = tx.send(encode_packet(&s::ObjectChat {
                object_id: oid, text: "不能拜自己为师".to_string(), r#type: 1,
            })).await;
            return;
        }
        if !tp.can_be_mentor {
            let _ = tx.send(encode_packet(&s::ObjectChat {
                object_id: oid, text: format!("{target} 未开启收徒（对方需先允许）"), r#type: 1,
            })).await;
            return;
        }
    }
    // 记录待确认（存储到 target 的 pending_mentor 与自身的请求）
    {
        let mut players = world.players.lock().await;
        if let Some(tp) = players.get_mut(&target_oid) {
            tp.pending_mentor = Some(my_name.clone());
        }
    }
    // 向 target 发送 MentorRequest
    let level = world.get_player(oid).await.map(|x| x.level).unwrap_or(1);
    world.send_to(target_oid, encode_packet(&s::MentorRequest {
        name: my_name.clone(),
        level,
    })).await;
    let _ = tx.send(encode_packet(&s::ObjectChat {
        object_id: oid, text: format!("已向 {target} 发出收徒请求"), r#type: 1,
    })).await;
}

// ---------------------------------------------------------------------------
// 5) 婚姻
// ---------------------------------------------------------------------------

async fn handle_marriage_prompt(world: &World, oid: u32, tx: &mpsc::Sender<Vec<u8>>) {
    let Some(p) = world.get_player(oid).await else { return };
    match p.pending_marriage.clone() {
        Some(name) => {
            tx.send(encode_packet(&s::MarriageRequest { name })).await.ok();
        }
        None => {
            send_sys(world, oid, "当前没有待确认的结婚邀请").await;
        }
    }
}

async fn handle_marriage_reply(
    world: &World,
    db: &Database,
    oid: u32,
    accept: bool,
    tx: &mpsc::Sender<Vec<u8>>,
) {
    let Some(p) = world.get_player(oid).await else { return };
    let my_char = p.character_index;
    let my_name = p.name.clone();
    let spouse_name = match p.pending_marriage.clone() {
        Some(n) => n,
        None => {
            send_sys(world, oid, "当前没有待确认的结婚邀请").await;
            return;
        }
    };
    let spouse_oid = match player_oid_by_name(world, &spouse_name).await {
        Some(o) => o,
        None => {
            send_sys(world, oid, &format!("{spouse_name} 不在线")).await;
            return;
        }
    };
    // 清除双方待确认
    {
        let mut players = world.players.lock().await;
        if let Some(pl) = players.get_mut(&oid) {
            pl.pending_marriage = None;
        }
        if let Some(sp) = players.get_mut(&spouse_oid) {
            sp.pending_marriage = None;
        }
    }
    if !accept {
        send_sys(world, oid, &format!("已拒绝 {spouse_name} 的求婚")).await;
        world.send_to(spouse_oid, encode_packet(&s::ObjectChat {
            object_id: spouse_oid,
            text: format!("{my_name} 拒绝了你的求婚"),
            r#type: 1,
        })).await;
        return;
    }
    // 双方在线才可结婚
    let spouse = world.get_player(spouse_oid).await;
    let Some(spouse) = spouse else {
        send_sys(world, oid, &format!("{spouse_name} 不在线")).await;
        return;
    };
    let spouse_char = spouse.character_index;
    let date = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let _ = db.set_spouse(my_char, spouse_char, date);
    // 双方发送 LoverUpdate
    let my_map = world.get_player(oid).await.map(|x| x.map_index).unwrap_or(0);
    let sp_map = world.get_player(spouse_oid).await.map(|x| x.map_index).unwrap_or(0);
    tx.send(encode_packet(&s::LoverUpdate {
        name: spouse_name.clone(),
        date,
        map_name: format!("m{sp_map}"),
        married_days: 0,
    })).await.ok();
    world.send_to(spouse_oid, encode_packet(&s::LoverUpdate {
        name: my_name.clone(),
        date,
        map_name: format!("m{my_map}"),
        married_days: 0,
    })).await;
    send_sys(world, oid, &format!("✓ 你与 {spouse_name} 喜结连理！")).await;
    send_sys(world, spouse_oid, &format!("✓ 你与 {my_name} 喜结连理！")).await;
}

async fn handle_divorce(world: &World, db: &Database, oid: u32, tx: &mpsc::Sender<Vec<u8>>) {
    let Some(p) = world.get_player(oid).await else { return };
    let my_char = p.character_index;
    let my_name = p.name.clone();
    let spouse_char = db.get_spouse(my_char);
    // 清除双方关系
    db.clear_spouse(my_char).ok();
    if let Some(sc) = spouse_char {
        if db.get_spouse(sc) == Some(my_char) {
            db.clear_spouse(sc).ok();
        }
    }
    tx.send(encode_packet(&s::LoverUpdate {
        name: String::new(),
        date: 0,
        map_name: String::new(),
        married_days: 0,
    })).await.ok();
    // 若对方在线，通知
    let _ = my_name;
    let _ = spouse_char;
    send_sys(world, oid, "你已离婚").await;
    if let Some(sc) = spouse_char {
        if let Some(sc_name) = db.char_name(sc).ok().flatten() {
            if let Some(so) = player_oid_by_name(world, &sc_name).await {
                world.send_to(so, encode_packet(&s::LoverUpdate {
                    name: String::new(),
                    date: 0,
                    map_name: String::new(),
                    married_days: 0,
                })).await;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 6) 转生
// ---------------------------------------------------------------------------

async fn handle_accept_reincarnation(
    world: &World,
    db: &Database,
    oid: u32,
    tx: &mpsc::Sender<Vec<u8>>,
) {
    let Some(p) = world.get_player(oid).await else { return };
    let c = p.character_index;
    let _ = db.add_reincarnation(c);
    // 重置等级为 1，保留金币，恢复满 HP/MP
    {
        let mut players = world.players.lock().await;
        if let Some(pl) = players.get_mut(&oid) {
            pl.level = 1;
            let (bh, bk) = base_stats(pl.class, 1);
            pl.max_hp = bh;
            pl.hp = bh;
            pl.max_mp = 25;
            pl.mp = 25;
            pl.attack = bk;
            pl.defence = 0;
            recompute_stats(pl);
        }
    }
    tx.send(encode_packet(&s::RequestReincarnation {})).await.ok();
    let p = world.get_player(oid).await;
    if let Some(p) = p {
        tx.send(encode_packet(&s::UserInformation {
            object_id: oid,
            real_id: oid,
            name: p.name.clone(),
            guild_name: String::new(),
            guild_rank: String::new(),
            name_colour: crystal_protocol::binary::Argb(0),
            class: p.class,
            gender: p.gender,
            level: 1,
            location: p.location,
            direction: p.direction,
            hair: 0,
            hp: p.hp,
            mp: p.mp,
            experience: 0,
            max_experience: 100,
            level_effects: crystal_protocol::types::LevelEffects(0),
            has_hero: false,
            hero_behaviour: 0,
            inventory: None,
            equipment: None,
            quest_inventory: None,
            gold: p.gold,
            credit: 0,
            has_expanded_storage: false,
            has_storage_password: false,
            require_storage_password: false,
            storage_password_last_set: 0,
            expanded_storage_expiry_time: 0,
            magics: vec![],
            intelligent_creatures: vec![],
            summoned_creature_type: 0,
            creature_summoned: false,
            allow_observe: false,
            observer: false,
        })).await.ok();
    }
    send_sys(world, oid, "你已转生，等级重置为 1，金币保留").await;
}

// ---------------------------------------------------------------------------
// 7) 商城
// ---------------------------------------------------------------------------

/// 定义两个商城道具（内置内存库存）。
/// 回城卷(4) 价格 50 金币；布衣(2) 价格 120 金币。
fn gameshop_items() -> Vec<s::GameShopItem> {
    let mk = |item_index: i32, gold_price: u32, stock: i32| -> s::GameShopItem {
        let name = items::find(item_index).map(|t| t.name.to_string()).unwrap_or_else(|| format!("#{item_index}"));
        s::GameShopItem {
            item_index,
            g_index: item_index,
            info: crystal_protocol::types::ItemInfo {
                index: item_index,
                name,
                ..Default::default()
            },
            gold_price,
            credit_price: 0,
            count: 1,
            class: String::new(),
            category: String::new(),
            stock,
            i_stock: true,
            deal: false,
            top_item: false,
            date: 0,
            can_buy_credit: false,
            can_buy_gold: true,
        }
    };
    vec![
        mk(4, 50, 100),   // 回城卷
        mk(2, 120, 100),  // 布衣
    ]
}

async fn handle_gameshop_buy(
    world: &World,
    db: &Database,
    oid: u32,
    g_index: i32,
    quantity: u8,
    p_type: i32,
    tx: &mpsc::Sender<Vec<u8>>,
) {
    if p_type != 0 {
        send_sys(world, oid, "暂不支持点心/点券支付").await;
        return;
    }
    let Some(item) = gameshop_items().into_iter().find(|i| i.g_index == g_index) else {
        send_sys(world, oid, "商城物品不存在").await;
        return;
    };
    let qty = quantity.max(1) as u32;
    let cost = item.gold_price.saturating_mul(qty);
    let Some(p) = world.get_player(oid).await else { return };
    if p.gold < cost {
        send_sys(world, oid, "金币不足").await;
        return;
    }
    if !remove_gold(world, oid, cost).await {
        send_sys(world, oid, "金币不足").await;
        return;
    }
    // 扣除剩余库存
    let remaining_stock = (item.stock - qty as i32).max(0);
    let _ = db.add_item_to_inventory(p.character_index, item.item_index, qty as u16);
    tx.send(encode_packet(&s::GameShopStock { g_index, stock_level: remaining_stock })).await.ok();
    send_sys(world, oid, &format!("✓ 已购买 {}x{qty}，花费 {cost} 金币", item.info.name)).await;
}

fn valid_account_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 30 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn char_name_valid(name: &str) -> bool {
    !name.is_empty() && name.len() <= 16
}

// MirGridType（见 Shared/Enums.cs）
const GRID_INVENTORY: u8 = 1;
const GRID_EQUIPMENT: u8 = 2;

/// 使用物品（金创药等消耗品）：回复 HP 并消耗一格。
/// 聊天命令处理（调试/演示）：/map <index> 传送到指定地图，/spawn 回新手村。
async fn handle_chat_command(
    world: &World,
    db: &Database,
    tx: &mpsc::Sender<Vec<u8>>,
    oid: u32,
    full: &str,
    cmd: Vec<&str>,
) {
    let sys = |text: &str| {
        encode_packet(&s::ObjectChat {
            object_id: oid,
            text: text.to_string(),
            r#type: 1, // 系统提示
        })
    };
    match cmd[0] {
        "/map" => {
            if let Some(idx) = cmd.get(1).and_then(|s| s.parse::<u32>().ok()) {
                // 目标地图中心作为目的地
                let map = world.get_map(idx);
                let (cx, cy) = (map.width as i32 / 2, map.height as i32 / 2);
                let ok = world.teleport_player(oid, idx, cx, cy).await;
                let _ = tx
                    .send(sys(&format!("{}= 传送到地图 {idx}", if ok { "✓" } else { "✗" })))
                    .await;
            } else {
                let _ = tx.send(sys("用法: /map <index>  (0/0100/0101...)")).await;
            }
        }
        "/spawn" => {
            let ok = world.teleport_player(oid, 0, 400, 400).await;
            let _ = tx.send(sys(&format!("{}回新手村", if ok { "✓" } else { "✗" }))).await;
        }
        "/trade" | "/trade_info" => {
            let name = world.get_player(oid).await.map(|p| p.name).unwrap_or_default();
            let in_trade = world.trade.lock().await.in_trade(&name);
            let _ = tx.send(sys(&format!("交易状态: {}", if in_trade { "进行中" } else { "无" }))).await;
        }
        "/trade_req" => {
            let name = world.get_player(oid).await.map(|p| p.name).unwrap_or_default();
            if let Some(target) = cmd.get(1) {
                if player_oid_by_name(world, target).await.is_some() {
                    world.trade.lock().await.request(&name, target);
                    let _ = tx.send(sys(&format!("已向 {target} 发起交易请求（对方用 /trade_accept 接受）"))).await;
                } else {
                    let _ = tx.send(sys("目标不在线")).await;
                }
            } else {
                let _ = tx.send(sys("用法: /trade_req <角色名>")).await;
            }
        }
        "/trade_accept" => {
            let name = world.get_player(oid).await.map(|p| p.name).unwrap_or_default();
            match world.trade.lock().await.accept(&name) {
                Ok(()) => { let _ = tx.send(sys("✓ 已接受交易，双方用 /trade_gold <n> 或 /trade_item <槽位> 放入交易物")).await; }
                Err(_) => { let _ = tx.send(sys("没有待接受的交易请求")).await; }
            }
        }
        "/trade_gold" => {
            let name = world.get_player(oid).await.map(|p| p.name).unwrap_or_default();
            let amt = cmd.get(1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            if world.trade.lock().await.add_gold(&name, amt).is_ok() {
                let _ = tx.send(sys(&format!("已放入金币 {amt}"))).await;
            } else { let _ = tx.send(sys("不在交易中")).await; }
        }
        "/trade_item" => {
            let name = world.get_player(oid).await.map(|p| p.name).unwrap_or_default();
            let slot = cmd.get(1).and_then(|s| s.parse::<i32>().ok());
            let uid = match (slot, world.get_player(oid).await) {
                (Some(sl), Some(p)) => {
                    db.load_inventory(p.character_index).ok()
                        .and_then(|inv| inv.into_iter().find(|(s, _)| *s == sl).map(|(_, it)| it.unique_id))
                }
                _ => None,
            };
            if let Some(uid) = uid {
                if world.trade.lock().await.add_item(&name, uid).is_ok() {
                    let _ = tx.send(sys("已放入一件背包物品")).await;
                } else { let _ = tx.send(sys("不在交易中")).await; }
            } else { let _ = tx.send(sys("用法: /trade_item <背包槽位> 或该槽无物品")).await; }
        }
        "/trade_confirm" => {
            let name = world.get_player(oid).await.map(|p| p.name).unwrap_or_default();
            let _ = world.trade.lock().await.confirm(&name);
            let settle = world.trade.lock().await.complete(&name);
            if let Ok(Some(s)) = settle {
                // 结算: 转移物品(按名字找双方角色)与金币
                let a_oid = player_oid_by_name(world, &s.a).await;
                let b_oid = player_oid_by_name(world, &s.b).await;
                if let (Some(ao), Some(bo)) = (a_oid, b_oid) {
                    if let (Some(pa), Some(pb)) =
                        (world.get_player(ao).await, world.get_player(bo).await)
                    {
                        for uid in &s.a_items_to_b { let _ = db.transfer_item(pa.character_index, pb.character_index, *uid); }
                        for uid in &s.b_items_to_a { let _ = db.transfer_item(pb.character_index, pa.character_index, *uid); }
                    }
                }
                if let Some(ao) = a_oid { if s.a_gold_to_b > 0 && remove_gold(world, ao, s.a_gold_to_b).await { if let Some(bo) = b_oid { gain_gold(world, bo, s.a_gold_to_b).await; } } }
                if let Some(bo) = b_oid { if s.b_gold_to_a > 0 && remove_gold(world, bo, s.b_gold_to_a).await { if let Some(ao) = a_oid { gain_gold(world, ao, s.b_gold_to_a).await; } } }
                let _ = tx.send(sys("✓ 交易完成，物品与金币已交换")).await;
            } else {
                let _ = tx.send(sys("已确认（等待对方确认后自动完成）")).await;
            }
        }
        "/trade_cancel" => {
            let name = world.get_player(oid).await.map(|p| p.name).unwrap_or_default();
            if world.trade.lock().await.cancel(&name) {
                let _ = tx.send(sys("已取消交易")).await;
            } else { let _ = tx.send(sys("不在交易中")).await; }
        }

        "/guild" => {
            let name = world.get_player(oid).await.map(|p| p.name).unwrap_or_default();
            let g = world.guild.lock().await.guild_of(&name).map(|g| (g.name.clone(), g.owner.clone(), g.members.len()));
            match g {
                Some((n, o, c)) => { let _ = tx.send(sys(&format!("公会 {n}（会长 {o}），成员 {c} 人"))).await; }
                None => { let _ = tx.send(sys("你不在任何公会")).await; }
            }
        }
        "/guild_create" => {
            let name = world.get_player(oid).await.map(|p| p.name).unwrap_or_default();
            if let Some(gname) = cmd.get(1) {
                match world.guild.lock().await.create(gname, &name) {
                    Ok(_) => { let _ = tx.send(sys(&format!("已创建公会 {gname}"))).await; }
                    Err(_) => { let _ = tx.send(sys("公会名已存在")).await; }
                }
            } else { let _ = tx.send(sys("用法: /guild_create <公会名>")).await; }
        }
        "/guild_join" => {
            let name = world.get_player(oid).await.map(|p| p.name).unwrap_or_default();
            if let Some(gname) = cmd.get(1) {
                match world.guild.lock().await.join(gname, &name) {
                    Ok(_) => { let _ = tx.send(sys(&format!("已加入公会 {gname}"))).await; }
                    Err(_) => { let _ = tx.send(sys("加入失败（公会不存在或你已在公会）")).await; }
                }
            } else { let _ = tx.send(sys("用法: /guild_join <公会名>")).await; }
        }
        "/guild_leave" => {
            let name = world.get_player(oid).await.map(|p| p.name).unwrap_or_default();
            if world.guild.lock().await.leave(&name) {
                let _ = tx.send(sys("已离开（会长离开则解散公会）")).await;
            } else { let _ = tx.send(sys("你不在公会")).await; }
        }

        "/market" => {
            let orders = world.market.lock().await.all_orders();
            if orders.is_empty() {
                let _ = tx.send(sys("市场暂无挂单")).await;
            } else {
                let msg = orders.iter().map(|o| format!("#{} <{}> 价{}", o.order_id, o.item_uid, o.price)).collect::<Vec<_>>().join("; ");
                let _ = tx.send(sys(&format!("市场 {} 单: {msg}", orders.len()))).await;
            }
        }
        "/market_sell" => {
            let name = world.get_player(oid).await.map(|p| p.name).unwrap_or_default();
            let slot = cmd.get(1).and_then(|s| s.parse::<i32>().ok());
            let price = cmd.get(2).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            let uid = if let (Some(sl), Some(p)) = (slot, world.get_player(oid).await) {
                db.load_inventory(p.character_index).ok().and_then(|inv| inv.into_iter().find(|(s, _)| *s == sl).map(|(_, it)| it.unique_id))
            } else { None };
            if let Some(uid) = uid {
                match world.market.lock().await.list(&name, uid, price) {
                    Ok(id) => { let _ = tx.send(sys(&format!("已挂单 #{id}，价 {price}"))).await; }
                    Err(_) => { let _ = tx.send(sys("价格需>0")).await; }
                }
            } else { let _ = tx.send(sys("用法: /market_sell <背包槽位> <价格>")).await; }
        }
        "/market_buy" => {
            if let Some(id) = cmd.get(1).and_then(|s| s.parse::<u64>().ok()) {
                // 先查价格与卖家，再决定是否买入（避免金币不足仍扣单）
                let (my_gold, buyer_char) = {
                    let p = world.get_player(oid).await;
                    (p.as_ref().map(|p| p.gold).unwrap_or(0), p.map(|p| p.character_index))
                };
                let (seller_name, price, item_uid) = {
                    let m = world.market.lock().await;
                    m.orders.get(&id).map(|o| (o.seller.clone(), o.price, o.item_uid)).unwrap_or_default()
                };
                let seller_char = match player_oid_by_name(world, &seller_name).await {
                    Some(so) => world.get_player(so).await.map(|p| p.character_index),
                    None => None,
                };
                if my_gold >= price && price > 0 {
                    let order = world.market.lock().await.buy(id);
                    if let Ok(o) = order {
                        if let (Some(bc), Some(sc)) = (buyer_char, seller_char) {
                            let _ = db.transfer_item(sc, bc, o.item_uid);
                        }
                        if remove_gold(world, oid, o.price).await {
                            if let Some(so) = player_oid_by_name(world, &o.seller).await {
                                gain_gold(world, so, o.price).await;
                            }
                        }
                        let _ = tx.send(sys(&format!("✓ 已购买，扣除金币 {}", o.price))).await;
                    } else { let _ = tx.send(sys("订单不存在")).await; }
                } else { let _ = tx.send(sys("金币不足或价格非法")).await; }
                let _ = item_uid;
            } else { let _ = tx.send(sys("用法: /market_buy <订单id>")).await; }
        }
        "/market_cancel" => {
            if let Some(id) = cmd.get(1).and_then(|s| s.parse::<u64>().ok()) {
                if world.market.lock().await.cancel(id).is_ok() {
                    let _ = tx.send(sys("已撤单")).await;
                } else { let _ = tx.send(sys("订单不存在")).await; }
            } else { let _ = tx.send(sys("用法: /market_cancel <订单id>")).await; }
        }

        // ------------------------- 邮件（站内信） -------------------------
        "/mail" => {
            let char_index = world.get_player(oid).await.map(|p| p.character_index).unwrap_or(0);
            let inbox = db.mail_inbox(char_index).unwrap_or_default();
            if inbox.is_empty() {
                let _ = tx.send(sys("收件箱为空")).await;
            } else {
                let summary = inbox.iter().map(|m| {
                    format!("#{} {} {}金{}", m.id, m.title, m.gold, if m.is_read { "[读]" } else { "[新]" })
                }).collect::<Vec<_>>().join("; ");
                let _ = tx.send(sys(&format!("收件箱 {} 封: {summary}", inbox.len()))).await;
            }
        }
        "/mail_detail" => {
            let char_index = world.get_player(oid).await.map(|p| p.character_index).unwrap_or(0);
            if let Some(id) = cmd.get(1).and_then(|s| s.parse::<i64>().ok()) {
                if let Some(m) = db.get_mail(id, char_index).ok().flatten() {
                    let _ = tx.send(sys(&format!(
                        "邮件 #{} 来自「{}」: {} —— {}\n附件: 金币{} 物品{}",
                        m.id, m.from_name, m.title, m.body, m.gold, if m.item_uid>0 {"有"} else {"无"}
                    ))).await;
                    let _ = db.mark_mail_read(id);
                } else { let _ = tx.send(sys("邮件不存在")).await; }
            } else { let _ = tx.send(sys("用法: /mail_detail <id>")).await; }
        }
        "/mail_send" => {
            // /mail_send <角色名> <金币> <标题...>
            let from = world.get_player(oid).await.map(|p| p.name).unwrap_or_default();
            let Some(target) = cmd.get(1).cloned() else {
                let _ = tx.send(sys("用法: /mail_send <角色名> <金币> <标题>")).await; return;
            };
            let gold = cmd.get(2).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
            // 若金币>0 且玩家在线，先扣除余额（避免凭空发钱）
            if gold > 0 && player_gold(world, oid).await < gold as u32 {
                let _ = tx.send(sys("你的金币不足")).await; return;
            }
            let Some(to_char) = db.char_index_by_name(&target).ok().flatten() else {
                let _ = tx.send(sys("收件人角色不存在")).await; return;
            };
            if gold > 0 {
                let _ = remove_gold(world, oid, gold as u32).await;
            }
            let title = cmd.get(3..).unwrap_or(&[]).iter().copied().collect::<Vec<_>>().join(" ");
            let title = if title.is_empty() { "无标题".to_string() } else { title };
            let _ = db.send_mail(to_char, &from, &title, "", gold, 0);
            let _ = tx.send(sys(&format!("✓ 已发出邮件（金币 {gold}）"))).await;
        }
        "/mail_send_item" => {
            // /mail_send_item <角色名> <背包槽位> <标题...>  物品所有权转移
            let from = world.get_player(oid).await.map(|p| p.name).unwrap_or_default();
            let Some(target) = cmd.get(1).cloned() else {
                let _ = tx.send(sys("用法: /mail_send_item <角色名> <背包槽位> <标题>")).await; return;
            };
            let slot = cmd.get(2).and_then(|s| s.parse::<i32>().ok());
            let Some(to_char) = db.char_index_by_name(&target).ok().flatten() else {
                let _ = tx.send(sys("收件人角色不存在")).await; return;
            };
            let Some(p) = world.get_player(oid).await else { return; };
            let from_char = p.character_index;
            let (uid,) = match slot {
                Some(sl) => db.load_inventory(from_char).ok().map(|inv| {
                    inv.into_iter().find(|(s, _)| *s == sl).map(|(_, it)| it.unique_id)
                }).flatten().map(|u| (u,)).unwrap_or((0,)),
                None => (0,),
            };
            if uid == 0 {
                let _ = tx.send(sys("该槽位没有物品")).await; return;
            }
            // 转移到收件人背包（转移即视为附件；不占用 mail.item_uid）
            if !db.transfer_item(from_char, to_char, uid).unwrap_or(false) {
                let _ = tx.send(sys("转移失败")).await; return;
            }
            let title = cmd.get(3..).unwrap_or(&[]).iter().copied().collect::<Vec<_>>().join(" ");
            let title = if title.is_empty() { "物品附赠".to_string() } else { title };
            let _ = db.send_mail(to_char, &from, &title, "给你寄来一件物品", 0, 0);
            let _ = tx.send(sys("✓ 物品已随邮件寄出")).await;
        }
        "/mail_read" => {
            let char_index = world.get_player(oid).await.map(|p| p.character_index).unwrap_or(0);
            if let Some(id) = cmd.get(1).and_then(|s| s.parse::<i64>().ok()) {
                if db.get_mail(id, char_index).ok().flatten().is_some() {
                    let _ = db.mark_mail_read(id);
                    let _ = tx.send(sys("已标记为已读")).await;
                } else { let _ = tx.send(sys("邮件不存在")).await; }
            } else { let _ = tx.send(sys("用法: /mail_read <id>")).await; }
        }
        "/mail_reward" => {
            let char_index = world.get_player(oid).await.map(|p| p.character_index).unwrap_or(0);
            if let Some(id) = cmd.get(1).and_then(|s| s.parse::<i64>().ok()) {
                let Some(m) = db.get_mail(id, char_index).ok().flatten() else {
                    let _ = tx.send(sys("邮件不存在")).await; return;
                };
                let gold = m.gold;
                let item_uid = m.item_uid;
                if gold > 0 {
                    let _ = db.claim_mail_gold(id);
                    // 收件人本人在线，直接入账
                    let _ = gain_gold(world, oid, gold as u32).await;
                }
                if item_uid > 0 {
                    let _ = db.claim_mail_item(id);
                    // 物品所有权已在寄出时转移；此处仅作确认
                }
                let _ = tx.send(sys(&format!("✓ 已领取附件: 金币 {}, 物品 {}", gold, if item_uid>0 {"有"} else {"无"}))).await;
            } else { let _ = tx.send(sys("用法: /mail_reward <id>")).await; }
        }
        "/mail_delete" => {
            let char_index = world.get_player(oid).await.map(|p| p.character_index).unwrap_or(0);
            if let Some(id) = cmd.get(1).and_then(|s| s.parse::<i64>().ok()) {
                if db.get_mail(id, char_index).ok().flatten().is_some() {
                    let _ = db.delete_mail(id);
                    let _ = tx.send(sys("已删除邮件")).await;
                } else { let _ = tx.send(sys("邮件不存在")).await; }
            } else { let _ = tx.send(sys("用法: /mail_delete <id>")).await; }
        }

        // ------------------------- 任务 -------------------------
        "/quest_accept" => {
            let p = world.get_player(oid).await;
            let (name, char_index) = match p {
                Some(p) => (p.name, p.character_index),
                None => return,
            };
            // 取当前触碰 NPC 对应的任务
            let npc = { world.quest.lock().await.touched_npc(&name).map(|s| s.to_string()) };
            let def = npc.and_then(|n| crate::quest::QUESTS.iter().find(|q| q.npc_name == n).cloned());
            match def {
                Some(q) => {
                    let _ = db.accept_quest(char_index, q.id);
                    let _ = tx.send(sys(&format!("✓ 已接受任务【{}】：{}", q.name, q.description))).await;
                }
                None => {
                    let _ = tx.send(sys("请先走到任务管理员旁并触碰(CallNPC)，或 /quest_touch 任务管理员")).await;
                }
            }
        }
        "/quest_touch" => {
            // 手动指定触碰某 NPC（调试/演示），便于不走近也能查进度
            let name = world.get_player(oid).await.map(|p| p.name).unwrap_or_default();
            if let Some(n) = cmd.get(1) {
                world.quest.lock().await.touch(&name, n);
                let _ = tx.send(sys(&format!("已触碰 NPC「{n}」，可用 /quest_accept 接受任务"))).await;
            } else { let _ = tx.send(sys("用法: /quest_touch <NPC名>")).await; }
        }
        "/quest_status" => {
            let char_index = world.get_player(oid).await.map(|p| p.character_index).unwrap_or(0);
            let progress = db.load_quest_progress(char_index).unwrap_or_default();
            if progress.is_empty() {
                let _ = tx.send(sys("你还没有接受任何任务")).await;
            } else {
                let lines = progress.iter().map(|prog| {
                    let def = crate::quest::QUESTS.iter().find(|q| q.id == prog.quest_id);
                    match def {
                        Some(d) => {
                            let target = match d.objective {
                                crate::quest::QuestObjective::Kill { count, .. } => count,
                            };
                            let state = if prog.finished { "✓已领奖".to_string() }
                                else if prog.completed { "可领奖".to_string() }
                                else { format!("{}/{}", prog.killed, target) };
                            format!("[{}] {} {}", d.id, d.name, state)
                        }
                        None => format!("[{}] (未知任务)", prog.quest_id),
                    }
                }).collect::<Vec<_>>().join("; ");
                let _ = tx.send(sys(&format!("任务: {lines}"))).await;
            }
        }
        "/quest_reward" => {
            let char_index = world.get_player(oid).await.map(|p| p.character_index).unwrap_or(0);
            // 领取当前触碰 NPC 对应任务奖励；若无触碰则领取首个已完成未领奖的任务
            let name = world.get_player(oid).await.map(|p| p.name).unwrap_or_default();
            let touched = { world.quest.lock().await.touched_npc(&name).map(|s| s.to_string()) };
            let quest_id: Option<u32> = {
                let progress = db.load_quest_progress(char_index).unwrap_or_default();
                touched
                    .and_then(|n| crate::quest::QUESTS.iter().find(|q| q.npc_name == n).map(|q| q.id))
                    .or_else(|| progress.iter().find(|p| p.completed && !p.finished).map(|p| p.quest_id))
            };
            match quest_id {
                Some(qid) => {
                    match crate::quest::reward(char_index, qid, &db) {
                        Ok((def, gold, exp)) => {
                            let _ = gain_gold(world, oid, gold).await;
                            gain_experience(world, oid, exp).await;
                            if def.reward_item > 0 {
                                if let Some(p) = world.get_player(oid).await {
                                    let _ = db.add_item_to_inventory(p.character_index, def.reward_item, def.reward_item_count);
                                }
                            }
                            let _ = tx.send(sys(&format!(
                                "✓ 领取任务【{}】奖励: 金币{} 经验{} 物品{}",
                                def.name, gold, exp,
                                if def.reward_item>0 { format!("{}x{}", def.reward_item, def.reward_item_count) } else { "无".into() }
                            ))).await;
                        }
                        Err(e) => { let _ = tx.send(sys(&format!("无法领取: {e}"))).await; }
                    }
                }
                None => { let _ = tx.send(sys("没有可领取的任务奖励（任务需完成且未领奖）")).await; }
            }
        }
        "/quest_forget" => {
            let char_index = world.get_player(oid).await.map(|p| p.character_index).unwrap_or(0);
            if let Some(id) = cmd.get(1).and_then(|s| s.parse::<u32>().ok()) {
                let _ = db.forget_quest(char_index, id);
                let _ = tx.send(sys("已放弃任务")).await;
            } else { let _ = tx.send(sys("用法: /quest_forget <任务ID>")).await; }
        }

        // 仓库查看（仓库存取走 SelectGame/Slot 协议：StoreItem 存入 / TakeBackItem 取出）
        "/storage" | "/仓库" => {
            let char_index = world.get_player(oid).await.map(|p| p.character_index).unwrap_or(0);
            let slots = db.storage_slots(char_index).unwrap_or_default();
            let occupied = slots.iter().enumerate()
                .filter_map(|(i, s)| s.as_ref().map(|it| format!("槽{i}:#{}x{}", it.item_index, it.count)))
                .collect::<Vec<_>>();
            if occupied.is_empty() {
                let _ = tx.send(sys("仓库为空（容量 48）。用 StoreItem/TakeBackItem 协议存入/取出，或/help")).await;
            } else {
                let _ = tx.send(sys(&format!("仓库 {} 件: {}", occupied.len(), occupied.join("; ")))).await;
            }
        }

        // 好友
        "/friends" | "/好友" => {
            let char_index = world.get_player(oid).await.map(|p| p.character_index).unwrap_or(0);
            let online: std::collections::HashSet<i32> =
                world.players.lock().await.values().map(|p| p.character_index).collect();
            let rows = db.friend_rows(char_index).unwrap_or_default();
            if rows.is_empty() {
                let _ = tx.send(sys("好友列表为空。用 /friend_add <角色名> 添加")).await;
            } else {
                let txt = rows.iter().map(|(idx, name, _m, _b)| {
                    format!("{}{}", name, if online.contains(idx) { "[在线]" } else { "[离线]" })
                }).collect::<Vec<_>>().join("; ");
                let _ = tx.send(sys(&format!("好友 {} 人: {}", rows.len(), txt))).await;
            }
        }
        "/friend_add" => {
            let char_index = world.get_player(oid).await.map(|p| p.character_index).unwrap_or(0);
            if let Some(n) = cmd.get(1) {
                let (ok, _) = db.add_friend(char_index, n, "", false).unwrap_or((false, None));
                let _ = tx.send(sys(if ok { "✓ 已添加好友" } else { "添加失败（不存在/已是好友/加自己）" })).await;
            } else { let _ = tx.send(sys("用法: /friend_add <角色名>")).await; }
        }
        "/friend_del" => {
            let char_index = world.get_player(oid).await.map(|p| p.character_index).unwrap_or(0);
            if let Some(idx) = cmd.get(1).and_then(|s| s.parse::<i32>().ok()) {
                let _ = db.remove_friend(char_index, idx);
                let _ = tx.send(sys("已移除好友")).await;
            } else { let _ = tx.send(sys("用法: /friend_del <好友角色索引>（/friends 可看）")).await; }
        }

        // ------------------------- 师徒 -------------------------
        "/mentor" => {
            if let Some(target) = cmd.get(1) {
                chat_mentor_request(world, oid, target, tx).await;
            } else { let _ = tx.send(sys("用法: /mentor <师父角色名>")).await; }
        }
        "/mentor_accept" => {
            // 视作收到 MentorReply{accept=true}
            handle_mentor_reply(world, db, oid, true, tx).await;
        }
        "/mentor_refuse" => {
            handle_mentor_reply(world, db, oid, false, tx).await;
        }
        "/mentor_cancel" => {
            handle_mentor_cancel(world, db, oid, tx).await;
        }
        "/mentor_toggle" => {
            let mut players = world.players.lock().await;
            if let Some(p) = players.get_mut(&oid) {
                p.can_be_mentor = !p.can_be_mentor;
                let v = p.can_be_mentor;
                let _ = tx.send(sys(if v { "已开启收徒（他人可 /mentor 你）" } else { "已关闭收徒" })).await;
            }
        }

        // ------------------------- 婚姻 -------------------------
        "/marry" => {
            let my_name = world.get_player(oid).await.map(|p| p.name.clone()).unwrap_or_default();
            if let Some(target) = cmd.get(1) {
                let target_oid = match player_oid_by_name(world, target).await {
                    Some(o) => o,
                    None => { let _ = tx.send(sys(&format!("玩家 {target} 不在线"))).await; return; }
                };
                if target_oid == oid {
                    let _ = tx.send(sys("不能和自己结婚")).await; return;
                }
                // 记录双方待确认
                {
                    let mut players = world.players.lock().await;
                    if let Some(me) = players.get_mut(&oid) {
                        me.pending_marriage = Some(target.to_string());
                    }
                    if let Some(sp) = players.get_mut(&target_oid) {
                        sp.pending_marriage = Some(my_name.clone());
                    }
                }
                world.send_to(target_oid, encode_packet(&s::MarriageRequest { name: my_name.clone() })).await;
                let _ = tx.send(sys(&format!("已向 {target} 求婚（对方 /marry_accept 接受）"))).await;
            } else { let _ = tx.send(sys("用法: /marry <角色名>")).await; }
        }
        "/marry_accept" => {
            handle_marriage_reply(world, db, oid, true, tx).await;
        }
        "/marry_refuse" => {
            handle_marriage_reply(world, db, oid, false, tx).await;
        }
        "/divorce" => {
            handle_divorce(world, db, oid, tx).await;
        }

        "/help" | "/?" => {
            let _ = tx.send(sys("命令: /map <index> /spawn /guild_create <名> /guild_join <名> /guild /market /market_sell <槽> <价> /market_buy <id> /market_cancel <id> /trade_req <名> /trade_accept /trade_gold <n> /trade_item <槽> /trade_confirm /trade_cancel /mail /mail_send <名> <金币> <标题> /mail_send_item <名> <槽> <标题> /mail_read <id> /mail_reward <id> /mail_delete <id> /quest_accept /quest_status /quest_reward /quest_forget <id> /storage /friends /friend_add <名> /friend_del <索引> /mentor <师父> /mentor_accept /mentor_toggle /mentor_cancel /marry <名> /marry_accept /divorce")).await;
        }
        _ => {
            let _ = tx.send(sys(&format!("未知命令 {full}"))).await;
        }
    }
}

async fn handle_use_item(
    world: &World,
    db: &Database,
    object_id: &mut Option<u32>,
    u: &c::UseItem,
    tx: &mpsc::Sender<Vec<u8>>,
) -> anyhow::Result<()> {
    let Some(oid) = *object_id else { return Ok(()) };
    let Some(player) = world.get_player(oid).await else {
        return Ok(());
    };

    let Some((_slot, item)) = db.find_inventory_item(player.character_index, u.unique_id)? else {
        // 找不到该物品
        tx.send(encode_packet(&s::UseItem {
            unique_id: u.unique_id,
            success: false,
            grid: u.grid,
        }))
        .await
        .ok();
        return Ok(());
    };

    let used = use_consumable(world, oid, item.clone()).await;
    if !used {
        tx.send(encode_packet(&s::UseItem {
            unique_id: u.unique_id,
            success: false,
            grid: u.grid,
        }))
        .await
        .ok();
        return Ok(());
    }

    // 消耗成功：扣除一格
    db.consume_inventory_item(player.character_index, u.unique_id)?;

    tx.send(encode_packet(&s::UseItem {
        unique_id: u.unique_id,
        success: true,
        grid: u.grid,
    }))
    .await
    .ok();

    // 刷新背包槽位（消耗后数量/格位变化）
    send_slots_refresh(world, db, oid, player.character_index, tx).await;
    Ok(())
}

/// 丢弃背包物品到脚下（生成地面掉落物，供他人拾取）。
async fn handle_drop_item(
    world: &World,
    db: &Database,
    object_id: &mut Option<u32>,
    d: &c::DropItem,
    tx: &mpsc::Sender<Vec<u8>>,
) -> anyhow::Result<()> {
    let Some(oid) = *object_id else { return Ok(()) };
    let Some(player) = world.get_player(oid).await else {
        return Ok(());
    };
    // 先扣减背包，再生成地面物
    let Some((_slot, dropped)) =
        db.remove_item_count(player.character_index, d.unique_id, d.count)?
    else {
        return Ok(());
    };
    // 实际丢弃数量 = min(请求数, 原持有数)
    let mut ground = dropped.clone();
    ground.count = d.count.min(dropped.count);
    if drop_ground_item(world, oid, ground).await {
        send_slots_refresh(world, db, oid, player.character_index, tx).await;
    }
    Ok(())
}

/// 丢弃金币到脚下（扣除背包/身上金币，生成地面金币堆）。
async fn handle_drop_gold(world: &World, oid: u32, amount: u32) {
    if amount == 0 {
        return;
    }
    // 余额校验并扣除
    if !remove_gold(world, oid, amount).await {
        return;
    }
    drop_gold(world, oid, amount).await;
}

/// 修理装备：按缺失耐久计费，扣除金币后修复。
async fn handle_repair_item(
    world: &World,
    db: &Database,
    oid: u32,
    unique_id: u64,
    tx: &mpsc::Sender<Vec<u8>>,
) {
    let Some(player) = world.get_player(oid).await else { return };
    let char_index = player.character_index;
    // 单点耐久价格
    let price_per = 1u32;
    let Some((_cd, _md, cost)) = db.repair_item(char_index, unique_id, price_per).unwrap_or(None) else {
        send_sys(world, oid, "无法修理：物品不存在或无需修理").await;
        return;
    };
    if cost > 0 {
        if !remove_gold(world, oid, cost).await {
            send_sys(world, oid, &format!("金币不足，修理需 {} 金币", cost)).await;
            return;
        }
        let _ = db.apply_repair(char_index, unique_id);
        send_sys(world, oid, &format!("✓ 已修理，花费 {} 金币", cost)).await;
    } else {
        send_sys(world, oid, "物品耐久是满的，无需修理").await;
    }
    // 同步内存中对应装备/背包物品的耐久，并重算属性
    {
        let mut players = world.players.lock().await;
        if let Some(p) = players.get_mut(&oid) {
            for (_slot, item) in p.equipment.iter_mut() {
                if item.unique_id == unique_id && item.max_dura > 0 {
                    item.current_dura = item.max_dura;
                }
            }
            recompute_stats(p);
        }
    }
    send_slots_refresh(world, db, oid, char_index, tx).await;
}

// ------------------------- 查看玩家（Inspect / Observe） -------------------------

/// 回 PlayerInspect：查看目标玩家装备/等级/职业。
async fn handle_inspect(
    world: &World,
    tx: &mpsc::Sender<Vec<u8>>,
    oid: u32,
    target_id: u32,
) {
    let Some(target) = world.get_player(target_id).await else { return };
    let name = target.name.clone();
    let guild_name = world
        .guild
        .lock()
        .await
        .guild_of(&name)
        .map(|g| g.name.clone())
        .unwrap_or_default();
    let equipment = equipment_slots(&target);
    tx.send(encode_packet(&s::PlayerInspect {
        name,
        guild_name,
        guild_rank: String::new(),
        equipment,
        class: target.class as u8,
        gender: target.gender as u8,
        hair: 0,
        level: target.level,
        lover_name: String::new(),
        allow_observe: true,
        is_hero: false,
    }))
    .await
    .ok();
}

/// 按名字观察玩家：Observe
async fn handle_inspect_observe(
    world: &World,
    tx: &mpsc::Sender<Vec<u8>>,
    oid: u32,
    name: &str,
) {
    let tid = {
        let players = world.players.lock().await;
        players.values().find(|p| p.name == name).map(|p| p.object_id)
    };
    if let Some(tid) = tid {
        handle_inspect(world, tx, oid, tid).await;
    }
}

// ------------------------- 好友 -------------------------

/// 读取好友列表并按在线状态发给客户端（FriendUpdate）。
async fn send_friends(
    world: &World,
    db: &Database,
    tx: &mpsc::Sender<Vec<u8>>,
    oid: u32,
) {
    let Some(p) = world.get_player(oid).await else { return };
    let char_index = p.character_index;
    let online: std::collections::HashSet<i32> = world
        .players
        .lock()
        .await
        .values()
        .map(|pl| pl.character_index)
        .collect();
    let friends: Vec<crystal_protocol::types::ClientFriend> = db
        .friend_rows(char_index)
        .unwrap_or_default()
        .into_iter()
        .map(|(idx, name, memo, blocked)| crystal_protocol::types::ClientFriend {
            index: idx,
            name,
            memo,
            blocked,
            online: online.contains(&idx),
        })
        .collect();
    tx.send(encode_packet(&s::FriendUpdate { friends })).await.ok();
}

/// 添加好友并刷新列表
async fn handle_add_friend(
    world: &World,
    db: &Database,
    tx: &mpsc::Sender<Vec<u8>>,
    oid: u32,
    name: &str,
    blocked: bool,
) {
    let Some(p) = world.get_player(oid).await else { return };
    let (ok, _fi) = db.add_friend(p.character_index, name, "", blocked).unwrap_or((false, None));
    let msg = if ok {
        format!("✓ 已添加好友 {name}")
    } else {
        "添加失败：角色不存在、已是好友或不能加自己".to_string()
    };
    send_sys(world, oid, &msg).await;
    send_friends(world, db, tx, oid).await;
}

// ------------------------- 信息请求 -------------------------

/// RequestMapInfo：回地图信息
async fn handle_request_map_info(world: &World, tx: &mpsc::Sender<Vec<u8>>, oid: u32, map_index: i32) {
    let map = world.maps.read().unwrap().get(&(map_index as u32)).cloned();
    let Some(map) = map else { return };
    let (w, h) = (map.width as i32, map.height as i32);
    tx.send(encode_packet(&s::NewMapInfo {
        map_index: map.index as i32,
        info: crystal_protocol::types::ClientMapInfo {
            title: format!("地图 {}", map.index),
            width: w,
            height: h,
            big_map: 0,
            movements: vec![],
            npcs: vec![],
        },
    }))
    .await
    .ok();
    tx.send(encode_packet(&s::MapInformation {
        map_index: map.index as i32,
        file_name: format!("{}", map.index),
        title: format!("地图 {}", map.index),
        mini_map: 0,
        big_map: 0,
        lights: 0,
        lightning: false,
        fire: false,
        map_dark_light: 0,
        music: 0,
        weather_particles: 0,
    }))
    .await
    .ok();
    let _ = oid;
}

/// RequestItemInfo：回物品信息
async fn handle_request_item_info(tx: &mpsc::Sender<Vec<u8>>, oid: u32, item_index: i32) {
    let Some(t) = items::find(item_index) else { return };
    tx.send(encode_packet(&s::NewItemInfo {
        info: crystal_protocol::types::ItemInfo {
            index: t.index,
            name: t.name.to_string(),
            item_type: t.item_type,
            grade: 0,
            required_type: 0,
            required_class: 0,
            required_gender: 0,
            set: 0,
            shape: 0,
            weight: 0,
            light: 0,
            required_amount: 0,
            image: t.image,
            durability: t.max_dura,
            stack_size: 1,
            price: t.price,
            start_item: false,
            effect: 0,
            need_identify: false,
            show_group_pickup: false,
            class_based: false,
            level_based: false,
            can_mine: false,
            global_drop_notify: false,
            bind: 0,
            unique: 0,
            random_stats_id: 0,
            can_fast_run: false,
            can_awakening: false,
            slots: 0,
            stats: Default::default(),
            tool_tip: None,
        },
    }))
    .await
    .ok();
    let _ = oid;
}

/// RequestNPCInfo：回 NPC 信息
async fn handle_request_npc_info(world: &World, tx: &mpsc::Sender<Vec<u8>>, oid: u32, npc_index: i32) {
    let npcs = world.npcs().await;
    if let Some(n) = npcs.iter().find(|n| n.object_id as i32 == npc_index) {
        tx.send(encode_packet(&s::NewNPCInfo {
            info: crystal_protocol::types::ClientNpcInfo {
                index: npc_index,
                file_name: String::new(),
                name: n.name.clone(),
                map_index: n.map_index as i32,
                location: n.location,
                image: n.image,
                rate: 0,
                show_on_big_map: false,
                big_map_icon: 0,
                object_id: n.object_id,
                icon: 0,
                can_teleport_to: false,
            },
        }))
        .await
        .ok();
    }
    let _ = oid;
}

/// 回购：把最近出售的物品买回背包（按售出价计费）。
async fn handle_buy_back(
    world: &World,
    db: &Database,
    oid: u32,
    unique_id: u64,
    count: u16,
) {
    let Some(player) = world.get_player(oid).await else { return };
    let (item_index, sold_count, price) = {
        let players = world.players.lock().await;
        let p = players.get(&oid);
        match p.and_then(|p| p.recently_sold.iter().find(|(u, _, _, _)| *u == unique_id).cloned()) {
            Some((_, idx, sc, pr)) => (idx, sc, pr),
            None => { let _ = (count,); return; }
        }
    };
    let _ = sold_count;
    if price == 0 || !items::exists(item_index) {
        return;
    }
    // 扣除金币后放回背包
    if !remove_gold(world, oid, price).await {
        send_sys(world, oid, "金币不足，无法回购").await;
        return;
    }
    if db.add_item_to_inventory(player.character_index, item_index, 1).unwrap_or(false) {
        // 从最近出售中移除
        let mut players = world.players.lock().await;
        if let Some(p) = players.get_mut(&oid) {
            p.recently_sold.retain(|(u, _, _, _)| *u != unique_id);
        }
        drop(players);
        send_sys(world, oid, &format!("✓ 已回购物品（花费 {} 金币）", price)).await;
    } else {
        gain_gold(world, oid, price).await; // 失败回退
    }
}

/// RequestMonsterInfo：回怪物信息
async fn handle_request_monster_info(tx: &mpsc::Sender<Vec<u8>>, oid: u32, monster_index: i32) {
    if let Some(t) = crate::spawn_config::MONSTER_TEMPLATES.iter().find(|t| t.image as i32 == monster_index) {
        tx.send(encode_packet(&s::NewMonsterInfo {
            info: crystal_protocol::types::ClientMonsterInfo {
                index: monster_index,
                name: t.name.to_string(),
                game_name: t.name.to_string(),
                image: t.image,
                ai: 0,
                effect: 0,
                level: t.level,
                view_range: 5,
                cool_eye: 0,
                light: 0,
                attack_speed: 0,
                move_speed: 0,
                experience: t.exp,
                can_push: true,
                can_tame: false,
                auto_rev: false,
                undead: false,
                can_recall: false,
                stats: Default::default(),
            },
        }))
        .await
        .ok();
    }
    let _ = oid;
}

/// RequestGuildInfo：回玩家所在公会的状态
async fn handle_request_guild_info(world: &World, tx: &mpsc::Sender<Vec<u8>>, oid: u32, _type: u8) {
    let name = world.get_player(oid).await.map(|p| p.name).unwrap_or_default();
    if let Some(g) = world.guild.lock().await.guild_of(&name) {
        let g = g.clone();
        tx.send(encode_packet(&s::GuildStatus {
            guild_name: g.name,
            guild_rank_name: if g.owner == name { "会长".to_string() } else { "成员".to_string() },
            level: 1,
            experience: 0,
            max_experience: 100,
            gold: 0,
            spare_points: 0,
            member_count: g.members.len() as i32,
            max_members: 100,
            voting: false,
            item_count: 0,
            buff_count: 0,
            my_options: 0,
            my_rank_id: 0,
        }))
        .await
        .ok();
    } else {
        send_sys(world, oid, "你不在任何公会").await;
    }
}

/// SearchMap：搜索地图列表（以系统消息返回匹配项）
async fn handle_search_map(world: &World, tx: &mpsc::Sender<Vec<u8>>, oid: u32, text: &str) {
    let matches: Vec<u32> = {
        let maps = world.maps.read().unwrap();
        maps.keys()
            .filter(|idx| idx.to_string().contains(text))
            .copied()
            .collect()
    };
    if matches.is_empty() {
        send_sys(world, oid, &format!("未找到匹配「{text}」的地图")).await;
    } else {
        let list = matches.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(" ");
        send_sys(world, oid, &format!("匹配地图: {list}")).await;
    }
    let _ = tx;
}

/// TeleportToNPC：传送到指定 NPC 位置
async fn handle_teleport_to_npc(world: &World, oid: u32, npc_object_id: u32) {
    let npcs = world.npcs().await;
    if let Some(n) = npcs.iter().find(|n| n.object_id == npc_object_id) {
        world.teleport_player(oid, n.map_index, n.location.x, n.location.y).await;
        send_sys(world, oid, &format!("已传送至 NPC「{}」", n.name)).await;
    }
}

/// EquipSlotItem：把背包物品穿戴到指定装备槽（等价 EquipItem 的简化版）
async fn handle_equip_slot_item(
    world: &World,
    db: &Database,
    oid: u32,
    esi: &c::EquipSlotItem,
    tx: &mpsc::Sender<Vec<u8>>,
) {
    let Some(player) = world.get_player(oid).await else { return };
    let char_index = player.character_index;
    if esi.grid != GRID_INVENTORY {
        return;
    }
    let Some((_slot, item)) = db.find_inventory_item(char_index, esi.unique_id).ok().flatten() else { return };
    let Some(tmpl) = items::find(item.item_index) else { return };
    let valid = (tmpl.item_type == 1 && esi.to == 0) || (tmpl.item_type == 3 && esi.to == 1);
    if !valid {
        send_equip_fail(tx, &c::EquipItem { grid: esi.grid, unique_id: esi.unique_id, to: esi.to }).await;
        return;
    }
    let outcome = db.equip_item(char_index, esi.unique_id, item.item_index, esi.to).unwrap_or(crate::db::EquipOutcome { returned_to_inventory: false });
    if outcome.returned_to_inventory {
        let mut players = world.players.lock().await;
        if let Some(p) = players.get_mut(&oid) {
            p.equipment.insert(esi.to, item.clone());
            recompute_stats(p);
        }
        drop(players);
        tx.send(encode_packet(&s::EquipItem { grid: esi.grid, unique_id: esi.unique_id, to: esi.to, success: true })).await.ok();
    } else {
        send_equip_fail(tx, &c::EquipItem { grid: esi.grid, unique_id: esi.unique_id, to: esi.to }).await;
    }
}

/// 穿戴/卸下装备：grid=Inventory 穿戴到 to 装备槽；grid=Equipment 从 to 槽卸下。
async fn handle_equip_item(
    world: &World,
    db: &Database,
    object_id: &mut Option<u32>,
    e: &c::EquipItem,
    tx: &mpsc::Sender<Vec<u8>>,
) -> anyhow::Result<()> {
    let Some(oid) = *object_id else { return Ok(()) };
    let Some(player) = world.get_player(oid).await else {
        return Ok(());
    };
    let char_index = player.character_index;
    let success: bool;

    if e.grid == GRID_INVENTORY {
        // 穿戴：从背包取物品放到装备槽 to
        let Some((_slot, item)) = db.find_inventory_item(char_index, e.unique_id)? else {
            send_equip_fail(tx, e).await;
            return Ok(());
        };
        let Some(tmpl) = items::find(item.item_index) else {
            send_equip_fail(tx, e).await;
            return Ok(());
        };
        // 校验类型与槽位匹配：武器(1)->0，护甲(3)->1
        let valid_slot = (tmpl.item_type == 1 && e.to == 0) || (tmpl.item_type == 3 && e.to == 1);
        if !valid_slot {
            send_equip_fail(tx, e).await;
            return Ok(());
        }
        let outcome = db.equip_item(char_index, e.unique_id, item.item_index, e.to)?;
        if !outcome.returned_to_inventory {
            // 背包满，无法换下旧装备
            send_equip_fail(tx, e).await;
            return Ok(());
        }
        success = true;
        // 更新玩家内存装备
        {
            let mut players = world.players.lock().await;
            if let Some(p) = players.get_mut(&oid) {
                p.equipment.insert(e.to, item.clone());
                recompute_stats(p);
            }
        }
    } else if e.grid == GRID_EQUIPMENT {
        // 卸下：从装备槽 to 放回背包
        let Some(_) = db.unequip_item(char_index, e.to)? else {
            send_equip_fail(tx, e).await;
            return Ok(());
        };
        success = true;
        {
            let mut players = world.players.lock().await;
            if let Some(p) = players.get_mut(&oid) {
                p.equipment.remove(&e.to);
                recompute_stats(p);
            }
        }
    } else {
        send_equip_fail(tx, e).await;
        return Ok(());
    }

    tx.send(encode_packet(&s::EquipItem {
        grid: e.grid,
        unique_id: e.unique_id,
        to: e.to,
        success,
    }))
    .await
    .ok();

    // 装备变化 -> 刷新槽位 + 广播新外观（武器/护甲）
    send_slots_refresh(world, db, oid, char_index, tx).await;
    let p = world.get_player(oid).await;
    if let Some(p) = p {
        broadcast_player(world, &p).await;
    }
    Ok(())
}

async fn send_equip_fail(tx: &mpsc::Sender<Vec<u8>>, e: &c::EquipItem) {
    tx.send(encode_packet(&s::EquipItem {
        grid: e.grid,
        unique_id: e.unique_id,
        to: e.to,
        success: false,
    }))
    .await
    .ok();
}

/// 向玩家发送 UserSlotsRefresh（背包 40 槽 + 装备 14 槽）。
async fn send_slots_refresh(
    world: &World,
    db: &Database,
    oid: u32,
    char_index: i32,
    tx: &mpsc::Sender<Vec<u8>>,
) {
    let Ok(inventory) = db.inventory_slots(char_index) else {
        return;
    };
    let player = world.get_player(oid).await;
    let equip = player.map(|p| world_equipment_slots(&p)).unwrap_or_default();
    tx.send(encode_packet(&s::UserSlotsRefresh {
        inventory: Some(inventory),
        equipment: Some(equip),
    }))
    .await
    .ok();
}

fn world_equipment_slots(player: &Player) -> Vec<Option<crystal_protocol::types::UserItem>> {
    equipment_slots(player)
}

/// 广播玩家最新外观（穿戴变化后 weapon/armour 字段需要同步给所有人）。
async fn broadcast_player(world: &World, p: &Player) {
    let frame = encode_packet(&s::ObjectPlayer {
        object_id: p.object_id,
        name: p.name.clone(),
        guild_name: String::new(),
        guild_rank_name: String::new(),
        name_colour: crystal_protocol::binary::Argb(0),
        class: p.class,
        gender: p.gender,
        level: p.level,
        location: p.location,
        direction: p.direction,
        hair: 0,
        light: 0,
        weapon: p.weapon,
        weapon_effect: 0,
        armour: p.armour,
        poison: 0,
        dead: false,
        hidden: false,
        effect: 0,
        wing_effect: 0,
        extra: false,
        mount_type: 0,
        riding_mount: false,
        fishing: false,
        transform_type: 0,
        element_orb_effect: 0,
        element_orb_lvl: 0,
        element_orb_max: 0,
        buffs: vec![],
        level_effects: crystal_protocol::types::LevelEffects(0),
    });
    world.broadcast(frame);
}

fn num_class(v: u8) -> MirClass {
    match v {
        0 => MirClass::Warrior,
        1 => MirClass::Wizard,
        2 => MirClass::Taoist,
        3 => MirClass::Assassin,
        _ => MirClass::Archer,
    }
}

fn num_gender(v: u8) -> MirGender {
    match v {
        0 => MirGender::Male,
        _ => MirGender::Female,
    }
}

/// 与 `num_class` 一致，供 enter_world 复用
fn class_from_db(v: u8) -> MirClass {
    num_class(v)
}

/// 职业/等级决定基础 HP 与攻击力（程序化属性）
fn base_stats(_class: MirClass, level: u16) -> (i32, i32) {
    let hp = 40 + level as i32 * 8;
    let attack = 1 + level as i32 / 2;
    (hp, attack)
}
