//! 服务器→客户端 batch_2 回环测试（ID 67–102）。
//! write→read 必须完全一致，且 `reader.is_empty()`（无未消费字节）。

use crystal_protocol::binary::{Argb, Point, Reader, Writer};
use crystal_protocol::frame::{decode_packet, encode_packet, PacketCodec};
use crystal_protocol::server as s;
use crystal_protocol::types::{Awake, AwakeType, MirDirection, Stats, UserItem};
use crystal_protocol::ServerPacketId;

/// 载荷级回环: write → read == 原值，且全部消费。
fn roundtrip<p: PacketCodec + PartialEq + std::fmt::Debug>(p: p) {
    let mut w = Writer::new();
    p.write(&mut w);
    let bytes = w.into_inner();
    let mut r = Reader::new(bytes);
    let decoded = p::read(&mut r).unwrap();
    assert_eq!(decoded, p, "回环不一致 (id={})", p::ID);
    assert!(r.is_empty(), "包未完全消费 (id={})", p::ID);
}

/// 断言 ID 常量与 ServerPacketId 枚举一致。
fn assert_id<p: PacketCodec>(expected: ServerPacketId) {
    assert_eq!(p::ID, expected as i16, "ID 常量与枚举不一致");
}

fn sample_item() -> UserItem {
    UserItem {
        unique_id: 9001,
        item_index: 1024,
        current_dura: 33,
        max_dura: 40,
        count: 2,
        soul_bound_id: 7,
        identified: true,
        cursed: false,
        slots: vec![
            None,
            Some(Box::new(UserItem {
                unique_id: 9002,
                item_index: 5,
                ..Default::default()
            })),
        ],
        gem_count: 1,
        added_stats: Stats {
            values: vec![(1, 3), (5, 8)],
        },
        awake: Awake {
            r#type: AwakeType::Mc,
            list: vec![1, 2, 3],
        },
        refined_value: 2,
        refine_added: 1,
        refine_success_chance: 90,
        wedding_ring: -1,
        ..Default::default()
    }
}

#[test]
fn ids_match_enum() {
    assert_id::<s::GainedGold>(ServerPacketId::GainedGold);
    assert_id::<s::LoseGold>(ServerPacketId::LoseGold);
    assert_id::<s::GainedCredit>(ServerPacketId::GainedCredit);
    assert_id::<s::LoseCredit>(ServerPacketId::LoseCredit);
    assert_id::<s::ObjectMonster>(ServerPacketId::ObjectMonster);
    assert_id::<s::ObjectAttack>(ServerPacketId::ObjectAttack);
    assert_id::<s::Struck>(ServerPacketId::Struck);
    assert_id::<s::ObjectStruck>(ServerPacketId::ObjectStruck);
    assert_id::<s::DamageIndicator>(ServerPacketId::DamageIndicator);
    assert_id::<s::DuraChanged>(ServerPacketId::DuraChanged);
    assert_id::<s::HealthChanged>(ServerPacketId::HealthChanged);
    assert_id::<s::HeroHealthChanged>(ServerPacketId::HeroHealthChanged);
    assert_id::<s::DeleteItem>(ServerPacketId::DeleteItem);
    assert_id::<s::Death>(ServerPacketId::Death);
    assert_id::<s::ObjectDied>(ServerPacketId::ObjectDied);
    assert_id::<s::ColourChanged>(ServerPacketId::ColourChanged);
    assert_id::<s::ObjectColourChanged>(ServerPacketId::ObjectColourChanged);
    assert_id::<s::ObjectGuildNameChanged>(ServerPacketId::ObjectGuildNameChanged);
    assert_id::<s::GainExperience>(ServerPacketId::GainExperience);
    assert_id::<s::GainHeroExperience>(ServerPacketId::GainHeroExperience);
    assert_id::<s::LevelChanged>(ServerPacketId::LevelChanged);
    assert_id::<s::HeroLevelChanged>(ServerPacketId::HeroLevelChanged);
    assert_id::<s::ObjectLeveled>(ServerPacketId::ObjectLeveled);
    assert_id::<s::ObjectHarvest>(ServerPacketId::ObjectHarvest);
    assert_id::<s::ObjectHarvested>(ServerPacketId::ObjectHarvested);
    assert_id::<s::ObjectNPC>(ServerPacketId::ObjectNpc);
    assert_id::<s::NPCResponse>(ServerPacketId::NPCResponse);
    assert_id::<s::ObjectHide>(ServerPacketId::ObjectHide);
    assert_id::<s::ObjectShow>(ServerPacketId::ObjectShow);
    assert_id::<s::Poisoned>(ServerPacketId::Poisoned);
    assert_id::<s::ObjectPoisoned>(ServerPacketId::ObjectPoisoned);
    assert_id::<s::MapChanged>(ServerPacketId::MapChanged);
    assert_id::<s::ObjectTeleportOut>(ServerPacketId::ObjectTeleportOut);
    assert_id::<s::ObjectTeleportIn>(ServerPacketId::ObjectTeleportIn);
    assert_id::<s::TeleportIn>(ServerPacketId::TeleportIn);
    assert_id::<s::NPCGoods>(ServerPacketId::NPCGoods);
}

#[test]
fn currency_packets_roundtrip() {
    roundtrip(s::GainedGold { gold: 1_234_567 });
    roundtrip(s::LoseGold { gold: 987_654 });
    roundtrip(s::GainedCredit { credit: 3_333_333 });
    roundtrip(s::LoseCredit { credit: 444_444 });
}

#[test]
fn object_monster_roundtrip() {
    roundtrip(s::ObjectMonster {
        object_id: 42,
        name: "骷髅精灵".into(),
        name_colour: Argb::from_i32(-6_553_601),
        location: Point::new(123, 456),
        image: 7,
        direction: MirDirection::DownRight,
        effect: 2,
        ai: 5,
        light: 1,
        dead: false,
        skeleton: true,
        poison: 0x0003,
        hidden: true,
        shock_time: 999_999_999_999,
        binding_shot_center: true,
        extra: false,
        extra_byte: 0xEF,
        master_object_id: 77,
        rarity: 3,
        buffs: vec![1, 2, 5, 200],
    });

    // 空 Buffs
    roundtrip(s::ObjectMonster {
        object_id: 1,
        ..Default::default()
    });
}

#[test]
fn combat_packets_roundtrip() {
    roundtrip(s::ObjectAttack {
        object_id: 9,
        location: Point::new(1, 2),
        direction: MirDirection::Up,
        spell: 5,
        level: 2,
        r#type: 1,
    });
    roundtrip(s::Struck { attacker_id: 88 });
    roundtrip(s::ObjectStruck {
        object_id: 99,
        attacker_id: 88,
        location: Point::new(-3, -4),
        direction: MirDirection::Left,
    });
    roundtrip(s::DamageIndicator {
        damage: -2500,
        r#type: 3,
        object_id: 66,
    });
    roundtrip(s::Struck { attacker_id: 0 });
}

#[test]
fn durability_inventory_health_packets_roundtrip() {
    roundtrip(s::DuraChanged {
        unique_id: 9_876_543_210,
        current_dura: 25,
    });
    roundtrip(s::HealthChanged { hp: 1000, mp: 500 });
    roundtrip(s::HeroHealthChanged { hp: 750, mp: 300 });
    roundtrip(s::DeleteItem {
        unique_id: 12_345_678,
        count: 3,
    });
}

#[test]
fn death_packets_roundtrip() {
    roundtrip(s::Death {
        location: Point::new(55, 66),
        direction: MirDirection::Down,
    });
    roundtrip(s::ObjectDied {
        object_id: 1234,
        location: Point::new(55, 66),
        direction: MirDirection::Down,
        r#type: 2,
    });
}

#[test]
fn colour_and_guild_packets_roundtrip() {
    roundtrip(s::ColourChanged {
        name_colour: Argb::from_i32(-1),
    });
    roundtrip(s::ObjectColourChanged {
        object_id: 5,
        name_colour: Argb::from_i32(0x00FF00FF),
    });
    roundtrip(s::ObjectGuildNameChanged {
        object_id: 5,
        guild_name: "沙巴克".into(),
    });
}

#[test]
fn experience_level_packets_roundtrip() {
    roundtrip(s::GainExperience { amount: 50_000 });
    roundtrip(s::GainHeroExperience { amount: 12_000 });
    roundtrip(s::LevelChanged {
        level: 40,
        experience: 1_000_000,
        max_experience: 9_999_999,
    });
    roundtrip(s::HeroLevelChanged {
        level: 41,
        experience: 2_000_000,
        max_experience: 10_000_000,
    });
    roundtrip(s::ObjectLeveled { object_id: 321 });
}

#[test]
fn harvest_packets_roundtrip() {
    roundtrip(s::ObjectHarvest {
        object_id: 10,
        location: Point::new(7, 8),
        direction: MirDirection::UpRight,
    });
    roundtrip(s::ObjectHarvested {
        object_id: 10,
        location: Point::new(7, 9),
        direction: MirDirection::UpRight,
    });
}

#[test]
fn npc_packets_roundtrip() {
    roundtrip(s::ObjectNPC {
        object_id: 110,
        name: "武器店老板".into(),
        name_colour: Argb::from_i32(0xFF00FF00u32 as i32),
        image: 3,
        colour: Argb::from_i32(0xFFFF0000u32 as i32),
        location: Point::new(50, 50),
        direction: MirDirection::Right,
        quest_ids: vec![101, 202, 303],
    });
    roundtrip(s::ObjectNPC {
        object_id: 0,
        ..Default::default()
    });
    roundtrip(s::NPCResponse {
        page: vec!["欢迎光临".into(), "需要点什么?".into(), "再会".into()],
    });
    roundtrip(s::NPCResponse { page: vec![] });
}

#[test]
fn visibility_poison_packets_roundtrip() {
    roundtrip(s::ObjectHide { object_id: 12 });
    roundtrip(s::ObjectShow { object_id: 13 });
    roundtrip(s::Poisoned { poison: 0x00FF });
    roundtrip(s::ObjectPoisoned {
        object_id: 14,
        poison: 0x0101,
    });
}

#[test]
fn map_changed_roundtrip() {
    roundtrip(s::MapChanged {
        map_index: 3,
        file_name: "0".into(),
        title: "比奇省".into(),
        mini_map: 1,
        big_map: 2,
        lights: 2,
        location: Point::new(10, 20),
        direction: MirDirection::DownLeft,
        map_dark_light: 30,
        music: 7,
        weather: 5,
    });
}

#[test]
fn teleport_packets_roundtrip() {
    roundtrip(s::ObjectTeleportOut {
        object_id: 15,
        r#type: 1,
    });
    roundtrip(s::ObjectTeleportIn {
        object_id: 16,
        r#type: 0,
    });
    roundtrip(s::TeleportIn);
}

#[test]
fn npc_goods_roundtrip() {
    let goods = s::NPCGoods {
        list: vec![sample_item(), sample_item()],
        rate: 1.25,
        r#type: 6,
        hide_added_stats: true,
    };
    roundtrip(goods.clone());

    // 空列表
    roundtrip(s::NPCGoods {
        list: vec![],
        rate: 1.0,
        r#type: 0,
        hide_added_stats: false,
    });

    // 帧级压缩回环: COMPRESSED 应被 encode/decode 链路正确处理
    let frame = encode_packet(&goods);
    let id = i16::from_le_bytes([frame[2], frame[3]]);
    assert_eq!(id, s::NPCGoods::ID);
    let decoded = decode_packet::<s::NPCGoods>(id, &frame[4..]).unwrap();
    assert_eq!(decoded, goods);
}
