//! batch_5 回环测试: 写→读 必须完全一致，reader 必须消费完所有字节。
//! 对照 `Shared/ServerPackets.cs` 第 4768–5808 行所有包。

use crystal_protocol::binary::{Argb, Point, Reader, Writer};
use crystal_protocol::frame::{encode_packet, PacketCodec};
use crystal_protocol::server as s;
use crystal_protocol::types::*;

fn roundtrip<P: PacketCodec + PartialEq + std::fmt::Debug>(p: P) {
    let frame = encode_packet(&p);
    let id = i16::from_le_bytes([frame[2], frame[3]]);
    assert_eq!(id, P::ID, "包 ID 不匹配 ({} -> {})", P::ID, id);
    let payload = &frame[4..];
    let mut r = Reader::new(payload.to_vec());
    let decoded = P::read(&mut r).unwrap();
    assert_eq!(decoded, p, "回环不一致 (id={})", P::ID);
    assert!(r.is_empty(), "有未消费字节 (id={})", P::ID);
}

fn sample_item() -> UserItem {
    UserItem {
        unique_id: 9001,
        item_index: 123,
        current_dura: 15,
        max_dura: 30,
        count: 2,
        soul_bound_id: -1,
        identified: true,
        cursed: false,
        slots: vec![Some(Box::new(UserItem {
            unique_id: 9002,
            item_index: 456,
            ..Default::default()
        }))],
        gem_count: 3,
        added_stats: Stats {
            values: vec![(1, 7), (2, 11)],
        },
        awake: Awake {
            r#type: AwakeType::Dc,
            list: vec![1, 2, 3],
        },
        refined_value: 2,
        refine_added: 1,
        refine_success_chance: 80,
        wedding_ring: -1,
        expire_info: Some(ExpireInfo { expiry_date: 111 }),
        rental_information: Some(RentalInformation {
            owner_name: "玩家甲".into(),
            binding_flags: 5,
            expiry_date: 222,
            rental_locked: false,
        }),
        is_shop_item: false,
        sealed_info: Some(SealedInfo {
            expiry_date: 333,
            next_seal_date: 334,
        }),
        gm_made: true,
    }
}

fn sample_item_info() -> ItemInfo {
    ItemInfo {
        index: 1000,
        name: "裁决之杖".into(),
        item_type: 1,
        grade: 3,
        required_type: 0,
        required_class: 0,
        required_gender: 0,
        set: 0,
        shape: 2,
        weight: 40,
        light: 1,
        required_amount: 30,
        image: 10,
        durability: 35,
        stack_size: 1,
        price: 50000,
        start_item: false,
        effect: 0,
        need_identify: true,
        show_group_pickup: false,
        class_based: false,
        level_based: true,
        can_mine: false,
        global_drop_notify: true,
        bind: 3,
        unique: 0,
        random_stats_id: 1,
        can_fast_run: false,
        can_awakening: true,
        slots: 3,
        stats: Stats {
            values: vec![(0, 8), (1, 5)],
        },
        tool_tip: Some("这是一把神兵".into()),
    }
}

fn sample_client_magic() -> ClientMagic {
    ClientMagic {
        name: "火球术".into(),
        spell: 1,
        base_cost: 3,
        level_cost: 2,
        icon: 4,
        level1: 1,
        level2: 2,
        level3: 3,
        need1: 100,
        need2: 200,
        need3: 300,
        level: 2,
        key: 1,
        experience: 500,
        delay: 1000,
        range: 5,
        cast_time: 2000,
    }
}

fn sample_hero_info() -> ClientHeroInformation {
    ClientHeroInformation {
        index: 1,
        name: "英雄甲".into(),
        level: 38,
        class: MirClass::Assassin,
        gender: MirGender::Female,
    }
}

#[test]
fn hero_information_roundtrip() {
    // 继承自查: C# HeroInformation 完全重写 ReadPacket/WritePacket（见 batch_5.rs 头注释）
    roundtrip(s::HeroInformation {
        object_id: 777,
        name: "传奇英雄".into(),
        class: MirClass::Warrior,
        gender: MirGender::Male,
        level: 45,
        hair: 2,
        hp: 3500,
        mp: 1200,
        experience: 12_345_678,
        max_experience: 20_000_000,
        inventory: Some(vec![Some(sample_item()), None, Some(sample_item())]),
        equipment: None,
        magics: vec![sample_client_magic(), sample_client_magic()],
        auto_pot: true,
        auto_hp_percent: 60,
        auto_mp_percent: 30,
        hp_item_index: 5,
        mp_item_index: 9,
    });
    // 空槽方向抽查: 背包 None 时只写一个 false
    roundtrip(s::HeroInformation {
        inventory: None,
        equipment: Some(vec![Some(sample_item())]),
        ..Default::default()
    });
}

#[test]
fn hero_management_roundtrip() {
    roundtrip(s::UnlockHeroAutoPot);
    roundtrip(s::SetAutoPotValue { stat: 7, value: 42 });
    roundtrip(s::SetAutoPotItem {
        grid: MirGridType::Inventory,
        item_index: 12,
    });
    roundtrip(s::SetHeroBehaviour { behaviour: 2 });

    // ManageHeroes: 有当前英雄 + 完整列表（含空槽）
    roundtrip(s::ManageHeroes {
        maximum_count: 4,
        current_hero: Some(sample_hero_info()),
        heroes: Some(vec![
            Some(sample_hero_info()),
            None,
            Some(sample_hero_info()),
        ]),
    });
    // ManageHeroes: 无当前英雄 + 列表为 null
    roundtrip(s::ManageHeroes {
        maximum_count: 4,
        current_hero: None,
        heroes: None,
    });

    roundtrip(s::ChangeHero { from_index: 2 });
}

#[test]
fn npc_packets_roundtrip() {
    roundtrip(s::DefaultNPC { object_id: 300 });
    roundtrip(s::NPCUpdate { npc_id: 301 });
    roundtrip(s::NPCImageUpdate {
        object_id: 302,
        image: 7,
        colour: Argb(0xFF00FF00),
    });
}

#[test]
fn object_state_updates_roundtrip() {
    roundtrip(s::MountUpdate {
        object_id: 10,
        mount_type: 3,
        riding_mount: true,
    });
    roundtrip(s::TransformUpdate {
        object_id: 11,
        transform_type: -2,
    });
    roundtrip(s::EquipSlotItem {
        grid: MirGridType::Equipment,
        unique_id: 88_888,
        to: 4,
        grid_to: MirGridType::Inventory,
        success: true,
    });
    roundtrip(s::FishingUpdate {
        object_id: 12,
        fishing: true,
        progress_percent: 55,
        chance_percent: 33,
        fishing_point: Point::new(101, 202),
        found_fish: true,
    });
    roundtrip(s::ObjectSneaking {
        object_id: 13,
        sneaking_active: true,
    });
    roundtrip(s::ObjectLevelEffects {
        object_id: 14,
        level_effects: LevelEffects(0x0006),
    });
    roundtrip(s::SetBindingShot {
        object_id: 15,
        enabled: true,
        value: 9_000_000,
    });
    roundtrip(s::ObjectDeco {
        object_id: 16,
        location: Point::new(5, 6),
        image: 21,
    });
}

#[test]
fn quest_packets_roundtrip() {
    // ChangeQuest
    roundtrip(s::ChangeQuest {
        quest: ClientQuestProgress {
            id: 3,
            task_list: vec!["收集 10 个狼肉".into(), "击败巨型蠕虫".into()],
            taken: true,
            completed: false,
            new: true,
        },
        quest_state: 1,
        track_quest: true,
    });

    // CompleteQuest
    roundtrip(s::CompleteQuest {
        completed_quests: vec![4, 5, 6],
    });

    // ShareQuest
    roundtrip(s::ShareQuest {
        quest_index: 7,
        sharer_name: "热心玩家".into(),
    });

    // NewQuestInfo
    roundtrip(s::NewQuestInfo {
        info: ClientQuestInfo {
            index: 8,
            npc_index: 900,
            name: "讨伐狼王".into(),
            group: "比奇任务".into(),
            description: vec!["第一段".into(), "第二段".into()],
            task_description: vec!["任务目标".into()],
            return_description: vec!["回城报告".into()],
            completion_description: vec!["恭喜完成".into()],
            min_level_needed: 10,
            max_level_needed: 50,
            quest_needed: -1,
            class_needed: 0,
            quest_type: 2,
            time_limit_in_seconds: 3600,
            reward_gold: 10_000,
            reward_exp: 5_000,
            reward_credit: 100,
            rewards_fixed_item: vec![QuestItemReward {
                item: sample_item_info(),
                count: 2,
            }],
            rewards_select_item: vec![],
            finish_npc_index: 901,
        },
    });
}

#[test]
fn quest_item_packets_roundtrip() {
    roundtrip(s::GainedQuestItem {
        item: sample_item(),
    });
    roundtrip(s::DeleteQuestItem {
        unique_id: 1234,
        count: 3,
    });
}

#[test]
fn game_shop_packets_roundtrip() {
    roundtrip(s::GameShopInfo {
        item: s::GameShopItem {
            item_index: 900,
            g_index: 12,
            info: sample_item_info(),
            gold_price: 88_000,
            credit_price: 120,
            count: 5,
            class: "武器".into(),
            category: "战士".into(),
            stock: 3,
            i_stock: true,
            deal: false,
            top_item: true,
            date: crystal_protocol::binary::datetime_to_binary(
                1_700_000_000,
                crystal_protocol::binary::DateTimeKind::Utc,
            ),
            can_buy_credit: false,
            can_buy_gold: true,
        },
        stock_level: 2,
    });
    roundtrip(s::GameShopStock {
        g_index: 12,
        stock_level: -1,
    });
}

#[test]
fn reincarnation_roundtrip() {
    roundtrip(s::CancelReincarnation);
    roundtrip(s::RequestReincarnation);
}

#[test]
fn movement_packets_roundtrip() {
    roundtrip(s::UserBackStep {
        location: Point::new(3, 4),
        direction: MirDirection::DownLeft,
    });
    roundtrip(s::ObjectBackStep {
        object_id: 20,
        location: Point::new(5, 6),
        direction: MirDirection::Right,
        distance: 2,
    });
    roundtrip(s::UserDashAttack {
        location: Point::new(7, 8),
        direction: MirDirection::UpRight,
    });
    roundtrip(s::ObjectDashAttack {
        object_id: 21,
        location: Point::new(9, 10),
        direction: MirDirection::Left,
        distance: 3,
    });
    roundtrip(s::UserAttackMove {
        location: Point::new(11, 12),
        direction: MirDirection::Down,
    });
}

#[test]
fn item_action_packets_roundtrip() {
    roundtrip(s::CombineItem {
        grid: MirGridType::Inventory,
        id_from: 1001,
        id_to: 1002,
        success: true,
        destroy: false,
    });
    roundtrip(s::ItemUpgraded {
        item: sample_item(),
    });
}

#[test]
fn buff_effect_packets_roundtrip() {
    roundtrip(s::SetConcentration {
        object_id: 30,
        enabled: true,
        interrupted: false,
    });
    roundtrip(s::SetElemental {
        object_id: 31,
        enabled: true,
        casted: true,
        value: 250,
        element_type: 3,
        exp_last: 999,
    });
    roundtrip(s::SendOutputMessage {
        message: "物品已强化成功！".into(),
        r#type: 5,
    });
}

#[test]
fn npc_action_packets_roundtrip() {
    roundtrip(s::NPCAwakening);
    roundtrip(s::NPCDisassemble);
    roundtrip(s::NPCDowngrade);
    roundtrip(s::NPCReset);
}
