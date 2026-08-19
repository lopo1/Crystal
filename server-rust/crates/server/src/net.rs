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
    drop_ground_item, equipment_slots, gain_gold, npc_shop, persist_player, player_attack,
    player_magic_attack, pick_up, recompute_stats, remove_gold, use_consumable, Player, World,
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
                    tx.send(encode_packet(&s::Web3LoginResult {
                        result: 0,
                        characters,
                    }))
                    .await
                    .ok();
                }
                Err(crate::web3::Web3Error::ChallengeExpired) => {
                    tx.send(encode_packet(&s::Web3LoginResult {
                        result: 2,
                        characters: vec![],
                    }))
                    .await
                    .ok();
                }
                Err(_) => {
                    tx.send(encode_packet(&s::Web3LoginResult {
                        result: 3,
                        characters: vec![],
                    }))
                    .await
                    .ok();
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
            move_player(world, object_id, tx, wk.direction, 1).await;
        }
        ClientPacket::Run(r) => {
            move_player(world, object_id, tx, r.direction, 2).await;
        }
        ClientPacket::Chat(chat) => {
            if let Some(oid) = *object_id {
                let frame = encode_packet(&s::ObjectChat {
                    object_id: oid,
                    text: chat.message.clone(),
                    r#type: 0,
                });
                world.broadcast_except(frame.clone(), oid).await;
                let _ = tx.send(frame).await; // 自己也收到
            }
        }
        ClientPacket::Attack(atk) => {
            if let Some(oid) = *object_id {
                let dir = MirDirection::from_u8(atk.direction);
                if atk.spell != 0 {
                    // 魔法攻击（远程范围指向）
                    player_magic_attack(world, oid, dir, atk.spell).await;
                } else {
                    // 基础近战平A
                    player_attack(world, oid, dir).await;
                }
            }
        }
        ClientPacket::PickUp(_) => {
            if let Some(oid) = *object_id {
                pick_up(world, oid, db).await;
            }
        }
        ClientPacket::CallNPC(c) => {
            // 商人 -> 发送 NPCGoods 商店列表
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
        hp: if ch.hp > 0 { ch.hp } else { base_hp },
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
    Ok(object_id)
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

    let Some(new_loc) = world.try_move(player.location, direction, steps) else {
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
