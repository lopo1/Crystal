//! 单连接处理: 帧解析 + 登录握手状态机 + 进世界。

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc};

use crystal_protocol::client as c;
use crystal_protocol::frame::encode_packet;
use crystal_protocol::server as s;
use crystal_protocol::types::{MirClass, MirDirection, MirGender};
use crystal_protocol::ClientPacket;

use crate::account::AccountStore;
use crate::world::{try_move, Player, World, MAP_HEIGHT, MAP_WIDTH, SPAWN};

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
    accounts: AccountStore,
    world: World,
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
                        &accounts,
                        &world,
                        &tx,
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

    // 断线清理
    if let Some(oid) = object_id {
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
    accounts: &AccountStore,
    world: &World,
    tx: &mpsc::Sender<Vec<u8>>,
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
            } else if !accounts.register(&na.account_id) {
                7
            } else {
                8
            };
            tx.send(encode_packet(&s::NewAccount { result })).await.ok();
        }
        ClientPacket::Login(login) => {
            let result = if !accounts.login(&login.account_id) {
                3 // 账号不存在
            } else {
                *account_id = Some(login.account_id.clone());
                *stage = Stage::Select;
                0
            };
            if result == 0 {
                let characters = accounts.select_infos(account_id.as_ref().unwrap());
                tx.send(encode_packet(&s::LoginSuccess { characters }))
                    .await
                    .ok();
            } else {
                tx.send(encode_packet(&s::Login { result })).await.ok();
            }
        }
        ClientPacket::NewCharacter(nc) => {
            let result = match (account_id.as_ref(), char_name_valid(&nc.name)) {
                (Some(aid), true) => {
                    match accounts.add_character(aid, &nc.name, nc.class, nc.gender) {
                        Ok(info) => {
                            tx.send(encode_packet(&s::NewCharacterSuccess { char_info: info }))
                                .await
                                .ok();
                            return Ok(());
                        }
                        Err(code) => code,
                    }
                }
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
                .map(|aid| accounts.delete_character(aid, dc.character_index))
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
            let Some(ch) = accounts.get_character(aid, sg.character_index) else {
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

            let oid = enter_world(world, tx, aid, &ch.name, ch.class, ch.gender, ch.level).await;
            *object_id = Some(oid);
            *char_info = Some((ch.class, ch.gender, ch.name.clone()));
            *stage = Stage::Game;
        }
        ClientPacket::LogOut(_) => {
            if let Some(oid) = object_id.take() {
                world.remove_player(oid).await;
            }
            *stage = Stage::Select;
            if let Some(aid) = account_id.as_ref() {
                let characters = accounts.select_infos(aid);
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
                world.broadcast_except(frame.clone(), oid);
                let _ = tx.send(frame).await; // 自己也收到
            }
        }
        ClientPacket::Disconnect(_) => {}
        _ => {
            tracing::warn!("未处理的客户端包: {:?}", std::mem::discriminant(packet));
        }
    }
    Ok(())
}

/// 进入世界: 发送地图信息/自身信息/位置，广播 ObjectPlayer，返回 object_id
async fn enter_world(
    world: &World,
    tx: &mpsc::Sender<Vec<u8>>,
    account_id: &str,
    name: &str,
    class: MirClass,
    gender: MirGender,
    level: u16,
) -> u32 {
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
            width: MAP_WIDTH,
            height: MAP_HEIGHT,
            big_map: 0,
            movements: vec![],
            npcs: vec![],
        },
    }))
    .await
    .ok();

    let object_id = world.next_object_id();
    let player = Player {
        object_id,
        account_id: account_id.to_string(),
        name: name.to_string(),
        class,
        gender,
        level,
        location: SPAWN,
        direction: MirDirection::Up,
        hp: 100,
        mp: 100,
    };

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
        experience: 0,
        max_experience: 100,
        level_effects: crystal_protocol::types::LevelEffects(0),
        has_hero: false,
        hero_behaviour: 0,
        inventory: None,
        equipment: None,
        quest_inventory: None,
        gold: 1000,
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

    // 告知其他玩家: 新人进入
    let enter_frame = encode_packet(&s::ObjectPlayer {
        object_id,
        name: player.name.clone(),
        guild_name: String::new(),
        guild_rank_name: String::new(),
        name_colour: crystal_protocol::binary::Argb(0),
        class: player.class,
        gender: player.gender,
        level: player.level,
        location: player.location,
        direction: player.direction,
        hair: 0,
        light: 0,
        weapon: 0,
        weapon_effect: 0,
        armour: 0,
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
    world.broadcast_except(enter_frame, object_id);

    world.add_player(player).await;
    object_id
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

    let Some(new_loc) = try_move(player.location, direction, steps) else {
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
    world.broadcast_except(frame, oid);
}

fn valid_account_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 30 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn char_name_valid(name: &str) -> bool {
    !name.is_empty() && name.len() <= 16
}
