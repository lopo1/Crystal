//! 数据包回环测试: 写→读 必须完全一致，且字段顺序与 C# 定义一致。

use crystal_protocol::binary::{Argb, Point, Reader, Writer};
use crystal_protocol::client as c;
use crystal_protocol::frame::{encode_packet, PacketCodec};
use crystal_protocol::server as s;
use crystal_protocol::types::*;

fn roundtrip_client<P: PacketCodec + PartialEq + std::fmt::Debug>(p: P) {
    let frame = encode_packet(&p);
    assert_eq!(
        &frame[..2],
        &(frame.len() as u16).to_le_bytes()[..],
        "帧长度错误"
    );
    let id = i16::from_le_bytes([frame[2], frame[3]]);
    assert_eq!(id, P::ID, "包 ID 不匹配");
    let payload = &frame[4..];
    let decoded = P::read(&mut Reader::new(payload.to_vec())).unwrap();
    assert_eq!(decoded, p, "回环不一致 (id={})", P::ID);
}

fn roundtrip_server<P: PacketCodec + PartialEq + std::fmt::Debug>(p: P) {
    let frame = encode_packet(&p);
    let id = i16::from_le_bytes([frame[2], frame[3]]);
    assert_eq!(id, P::ID, "包 ID 不匹配");
    let payload = &frame[4..];
    let decoded = P::read(&mut Reader::new(payload.to_vec())).unwrap();
    assert_eq!(decoded, p, "回环不一致 (id={})", P::ID);
}

#[test]
fn client_core_packets_roundtrip() {
    roundtrip_client(c::ClientVersion {
        version_hash: vec![1, 2, 3, 4],
    });
    roundtrip_client(c::Disconnect);
    roundtrip_client(c::KeepAlive { time: 123456789 });
    roundtrip_client(c::NewAccount {
        account_id: "player1".into(),
        password: "secret".into(),
        birth_date: crystal_protocol::binary::datetime_to_binary(
            800_000_000,
            crystal_protocol::binary::DateTimeKind::Utc,
        ),
        user_name: "张三".into(),
        secret_question: "q?".into(),
        secret_answer: "a!".into(),
        email_address: "a@b.c".into(),
    });
    roundtrip_client(c::Login {
        account_id: "player1".into(),
        password: "secret".into(),
    });
    roundtrip_client(c::NewCharacter {
        name: "hero".into(),
        gender: MirGender::Female,
        class: MirClass::Wizard,
    });
    roundtrip_client(c::DeleteCharacter { character_index: 2 });
    roundtrip_client(c::StartGame { character_index: 0 });
    roundtrip_client(c::LogOut);
    for dir in [
        MirDirection::Up,
        MirDirection::DownRight,
        MirDirection::UpLeft,
    ] {
        roundtrip_client(c::Turn { direction: dir });
        roundtrip_client(c::Walk { direction: dir });
        roundtrip_client(c::Run { direction: dir });
    }
    roundtrip_client(c::Chat {
        message: "全体集合 🔥".into(),
        linked_items: vec![ChatItem {
            unique_id: 42,
            title: "木剑".into(),
            grid: MirGridType::Inventory,
        }],
    });
}

#[test]
fn server_core_packets_roundtrip() {
    roundtrip_server(s::Connected);
    roundtrip_server(s::ClientVersion { result: 1 });
    roundtrip_server(s::Disconnect { reason: 0 });
    roundtrip_server(s::KeepAlive { time: 1 });
    roundtrip_server(s::NewAccount { result: 8 });
    roundtrip_server(s::ChangePassword { result: 6 });
    roundtrip_server(s::Login { result: 0 });
    roundtrip_server(s::NewCharacter { result: 10 });
    roundtrip_server(s::StartGame {
        result: 0,
        resolution: 1024,
    });
    roundtrip_server(s::StartGameDelay { milliseconds: 2500 });
    roundtrip_server(s::DeleteCharacterSuccess { character_index: 3 });
    roundtrip_server(s::LoginBanned {
        reason: "禁止登录".into(),
        expiry_date: 1_700_000_000,
    });

    // 带角色的 LoginSuccess
    roundtrip_server(s::LoginSuccess {
        characters: vec![SelectInfo {
            index: 0,
            name: "英雄".into(),
            level: 34,
            class: MirClass::Warrior,
            gender: MirGender::Male,
            last_access: 0,
        }],
    });
    roundtrip_server(s::NewCharacterSuccess {
        char_info: SelectInfo {
            index: 1,
            name: "法神".into(),
            level: 1,
            class: MirClass::Wizard,
            gender: MirGender::Female,
            last_access: 0,
        },
    });
}

#[test]
fn server_world_packets_roundtrip() {
    roundtrip_server(s::MapInformation {
        map_index: 3,
        file_name: "0".into(),
        title: "比奇省".into(),
        mini_map: 1,
        big_map: 2,
        lights: 0,
        lightning: true,
        fire: false,
        map_dark_light: 50,
        music: 0,
        weather_particles: 0,
    });
    roundtrip_server(s::NewMapInfo {
        map_index: 3,
        info: ClientMapInfo {
            title: "比奇省".into(),
            width: 100,
            height: 100,
            big_map: 1,
            movements: vec![ClientMovementInfo {
                destination: 1,
                title: "盟重".into(),
                location: Point::new(50, 50),
                icon: 1,
            }],
            npcs: vec![],
        },
    });
    roundtrip_server(s::UserLocation {
        location: Point::new(10, 20),
        direction: MirDirection::Right,
    });
    roundtrip_server(s::ObjectRemove { object_id: 7 });

    let mut op = s::ObjectPlayer {
        object_id: 1,
        name: "老玩家".into(),
        guild_name: "行会".into(),
        guild_rank_name: "会长".into(),
        name_colour: Argb(0xFFFF0000),
        class: MirClass::Taoist,
        gender: MirGender::Female,
        level: 40,
        location: Point::new(3, 4),
        direction: MirDirection::Down,
        hair: 1,
        light: 0,
        weapon: 20,
        weapon_effect: 0,
        armour: 10,
        poison: 4,
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
        buffs: vec![1, 2, 3],
        level_effects: LevelEffects(0),
    };
    roundtrip_server(op.clone());
    roundtrip_server(s::ObjectHero {
        player: op.clone(),
        owner_name: "号主".into(),
    });
    roundtrip_server(s::ObjectTurn {
        object_id: 1,
        location: Point::new(3, 4),
        direction: MirDirection::Left,
    });
    roundtrip_server(s::ObjectWalk {
        object_id: 1,
        location: Point::new(3, 5),
        direction: MirDirection::Down,
    });
    roundtrip_server(s::ObjectRun {
        object_id: 1,
        location: Point::new(3, 7),
        direction: MirDirection::Down,
    });
    roundtrip_server(s::Chat {
        message: "hello".into(),
        r#type: 0,
    });
    roundtrip_server(s::ObjectChat {
        object_id: 1,
        text: "hi".into(),
        r#type: 1,
    });
    roundtrip_server(s::TimeOfDay { lights: 3 });

    // UserInformation（空背包/空魔法列表）
    roundtrip_server(s::UserInformation {
        object_id: 100,
        real_id: 200,
        name: "测试".into(),
        guild_name: String::new(),
        guild_rank: String::new(),
        name_colour: Argb(0),
        class: MirClass::Warrior,
        gender: MirGender::Male,
        level: 1,
        location: Point::new(5, 5),
        direction: MirDirection::Up,
        hair: 0,
        hp: 100,
        mp: 100,
        experience: 0,
        max_experience: 100,
        level_effects: LevelEffects(0),
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
    });
}

#[test]
fn user_item_roundtrip() {
    // 完整 UserItem（含槽、强化、过期、绑定、封印）
    let item = UserItem {
        unique_id: 1,
        item_index: 100,
        current_dura: 20,
        max_dura: 30,
        count: 3,
        soul_bound_id: -1,
        identified: true,
        cursed: false,
        slots: vec![
            Some(Box::new(UserItem {
                unique_id: 2,
                item_index: 101,
                ..Default::default()
            })),
            None,
        ],
        gem_count: 1,
        added_stats: Stats {
            values: vec![(1, 5), (5, 10)],
        },
        awake: Awake {
            r#type: AwakeType::Dc,
            list: vec![1, 2],
        },
        refined_value: 1,
        refine_added: 2,
        refine_success_chance: 70,
        wedding_ring: -1,
        expire_info: Some(ExpireInfo { expiry_date: 123 }),
        rental_information: Some(RentalInformation {
            owner_name: "租户".into(),
            binding_flags: 3,
            expiry_date: 456,
            rental_locked: true,
        }),
        is_shop_item: false,
        sealed_info: Some(SealedInfo {
            expiry_date: 789,
            next_seal_date: 790,
        }),
        gm_made: false,
    };

    let mut w = Writer::new();
    item.write(&mut w);
    let bytes = w.into_inner();
    let mut r = Reader::new(bytes.clone());
    let decoded = UserItem::read(&mut r).unwrap();
    assert_eq!(decoded, item);
    assert!(r.is_empty(), "UserItem 有未消费字节");

    // 字节级抽查关键字段（与 C# UserItem.Save 顺序一致）
    assert_eq!(&bytes[0..8], &1u64.to_le_bytes());
    assert_eq!(&bytes[8..12], &100i32.to_le_bytes());
    assert_eq!(&bytes[12..14], &20u16.to_le_bytes());
    assert_eq!(&bytes[14..16], &30u16.to_le_bytes());
    assert_eq!(&bytes[16..18], &3u16.to_le_bytes());
    assert_eq!(&bytes[18..22], &(-1i32).to_le_bytes());
    assert_eq!(bytes[22], 0x01, "Identified 位");
}
