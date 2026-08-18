//! batch_1 回环测试: 每个包 write→read 必须相等，且 reader 消费干净。
//! 字段顺序与 C# `Shared/ServerPackets.cs` 第 1219–2191 行一致。

use crystal_protocol::binary::{Argb, Point, Reader, Writer};
use crystal_protocol::frame::PacketCodec;
use crystal_protocol::server::batch_1 as b1;
use crystal_protocol::types::{
    Awake, AwakeType, ClientHeroInformation, ExpireInfo, MirClass, MirGender, RentalInformation,
    SealedInfo, SelectInfo, Stats, UserItem,
};

fn roundtrip<P: PacketCodec + PartialEq + std::fmt::Debug>(p: P) {
    let mut w = Writer::new();
    p.write(&mut w);
    let bytes = w.into_inner();
    let mut r = Reader::new(bytes);
    let decoded = P::read(&mut r).unwrap();
    assert_eq!(decoded, p, "回环不一致 (id={})", P::ID);
    assert!(r.is_empty(), "有未消费字节 (id={})", P::ID);
}

/// 非平凡 UserItem（含槽、强化、过期、绑定、封印），供多个包复用
fn sample_item() -> UserItem {
    UserItem {
        unique_id: 0x1122334455667788,
        item_index: 12345,
        current_dura: 40,
        max_dura: 50,
        count: 7,
        soul_bound_id: -5,
        identified: true,
        cursed: true,
        slots: vec![
            Some(Box::new(UserItem {
                unique_id: 99,
                item_index: 900,
                ..Default::default()
            })),
            None,
        ],
        gem_count: 3,
        added_stats: Stats {
            values: vec![(1, 12), (5, -3)],
        },
        awake: Awake {
            r#type: AwakeType::Mc,
            list: vec![1, 2, 3],
        },
        refined_value: 2,
        refine_added: 3,
        refine_success_chance: 80,
        wedding_ring: 777,
        expire_info: Some(ExpireInfo {
            expiry_date: 1_700_000_000_000,
        }),
        rental_information: Some(RentalInformation {
            owner_name: "租户甲".into(),
            binding_flags: 5,
            expiry_date: 1_700_000_000_001,
            rental_locked: true,
        }),
        is_shop_item: true,
        sealed_info: Some(SealedInfo {
            expiry_date: 1_700_000_000_002,
            next_seal_date: 1_700_000_000_003,
        }),
        gm_made: true,
    }
}

#[test]
fn character_and_item_info_roundtrip() {
    roundtrip(b1::NewHeroInfo {
        info: ClientHeroInformation {
            index: 3,
            name: "英雄甲".into(),
            level: 66,
            class: MirClass::Taoist,
            gender: MirGender::Female,
        },
        storage_index: -1,
    });
    roundtrip(b1::NewChatItem {
        item: sample_item(),
    });
    roundtrip(b1::GainedItem {
        item: sample_item(),
    });
}

#[test]
fn inventory_operation_roundtrip() {
    roundtrip(b1::MoveItem {
        grid: 1, // MirGridType.Inventory
        from: 3,
        to: 4,
        success: true,
    });
    roundtrip(b1::EquipItem {
        grid: 2, // MirGridType.Equipment
        unique_id: 0xDEADBEEF,
        to: 5,
        success: false,
    });
    roundtrip(b1::MergeItem {
        grid_from: 1,
        grid_to: 2,
        id_from: 111,
        id_to: 222,
        success: true,
    });
    roundtrip(b1::RemoveItem {
        grid: 4, // MirGridType.Storage
        unique_id: 333,
        to: 6,
        success: true,
    });
    roundtrip(b1::RemoveSlotItem {
        grid: 3, // MirGridType.Trade
        grid_to: 1,
        unique_id: 444,
        to: 7,
        success: false,
    });
    roundtrip(b1::TakeBackItem {
        from: 0,
        to: 12,
        success: true,
    });
    roundtrip(b1::StoreItem {
        from: 5,
        to: 6,
        success: true,
    });
    roundtrip(b1::DepositRefineItem {
        from: 1,
        to: 2,
        success: true,
    });
    roundtrip(b1::RetrieveRefineItem {
        from: 2,
        to: 1,
        success: true,
    });
    roundtrip(b1::RefineCancel { unlock: true });
    roundtrip(b1::RefineItem {
        unique_id: 0xCAFEBABE,
    });
    roundtrip(b1::DepositTradeItem {
        from: 0,
        to: 0,
        success: true,
    });
    roundtrip(b1::RetrieveTradeItem {
        from: 0,
        to: 0,
        success: false,
    });
    roundtrip(b1::SplitItem {
        item: Some(sample_item()),
        grid: 1,
    });
    roundtrip(b1::SplitItem {
        item: None,
        grid: 5,
    });
    roundtrip(b1::UseItem {
        unique_id: 555,
        success: true,
        grid: 1,
    });
    roundtrip(b1::DropItem {
        unique_id: 666,
        count: 9,
        hero_item: true,
        success: true,
    });
    roundtrip(b1::TakeBackHeroItem {
        from: 1,
        to: 2,
        success: true,
    });
    roundtrip(b1::TransferHeroItem {
        from: 3,
        to: 4,
        success: false,
    });
}

#[test]
fn player_packets_roundtrip() {
    roundtrip(b1::PlayerUpdate {
        object_id: 1001,
        light: 7,
        weapon: 101,
        weapon_effect: 2,
        armour: 88,
        wing_effect: 3,
    });
    // 含物品/空物品混合的装备栏
    roundtrip(b1::PlayerInspect {
        name: "被观察者".into(),
        guild_name: "测试行会".into(),
        guild_rank: "长老".into(),
        equipment: vec![Some(sample_item()), None, Some(sample_item())],
        class: 1,  // MirClass.Wizard
        gender: 0, // MirGender.Male
        hair: 4,
        level: 55,
        lover_name: "心上人".into(),
        allow_observe: true,
        is_hero: false,
    });
    // 空装备栏
    roundtrip(b1::PlayerInspect {
        name: String::new(),
        guild_name: String::new(),
        guild_rank: String::new(),
        equipment: vec![],
        class: 4,
        gender: 1,
        hair: 0,
        level: 1,
        lover_name: String::new(),
        allow_observe: false,
        is_hero: true,
    });
}

#[test]
fn relationship_and_trade_roundtrip() {
    roundtrip(b1::MarriageRequest {
        name: "配偶".into(),
    });
    roundtrip(b1::DivorceRequest {
        name: "前配偶".into(),
    });
    roundtrip(b1::MentorRequest {
        name: "师父".into(),
        level: 40,
    });
    roundtrip(b1::TradeRequest {
        name: "商贩".into(),
    });
    roundtrip(b1::TradeAccept {
        name: "商贩".into(),
    });
    roundtrip(b1::TradeGold { amount: 99_999 });
    roundtrip(b1::TradeItem {
        trade_items: vec![None, Some(sample_item())],
    });
    roundtrip(b1::TradeConfirm);
    roundtrip(b1::TradeCancel { unlock: true });
}

#[test]
fn logout_and_login_return_roundtrip() {
    roundtrip(b1::LogOutSuccess {
        characters: vec![
            SelectInfo {
                index: 0,
                name: "角色一".into(),
                level: 34,
                class: MirClass::Warrior,
                gender: MirGender::Male,
                last_access: 1_700_000_000,
            },
            SelectInfo {
                index: 1,
                name: "角色二".into(),
                level: 12,
                class: MirClass::Assassin,
                gender: MirGender::Female,
                last_access: 1_600_000_000,
            },
        ],
    });
    roundtrip(b1::LogOutFailed);
    roundtrip(b1::ReturnToLogin);
}

#[test]
fn mode_and_world_object_roundtrip() {
    roundtrip(b1::TimeOfDay { lights: 3 });
    roundtrip(b1::ChangeAMode { mode: 2 });
    roundtrip(b1::ChangePMode { mode: 1 });
    roundtrip(b1::ObjectItem {
        object_id: 42,
        name: "屠龙".into(),
        name_colour: Argb(0xFF00FF00),
        location: Point::new(32, 47),
        image: 65000,
        grade: 2, // ItemGrade
    });
    roundtrip(b1::ObjectGold {
        object_id: 43,
        gold: 5_000_000,
        location: Point::new(-1, -1),
    });
}

/// 验证压缩标志未被错误开启（本批 C# 无 Compressed 覆写）
#[test]
fn no_compressed_packets_in_batch() {
    assert!(!<b1::NewHeroInfo as PacketCodec>::COMPRESSED);
    assert!(!<b1::NewChatItem as PacketCodec>::COMPRESSED);
    assert!(!<b1::MoveItem as PacketCodec>::COMPRESSED);
    assert!(!<b1::EquipItem as PacketCodec>::COMPRESSED);
    assert!(!<b1::MergeItem as PacketCodec>::COMPRESSED);
    assert!(!<b1::RemoveItem as PacketCodec>::COMPRESSED);
    assert!(!<b1::RemoveSlotItem as PacketCodec>::COMPRESSED);
    assert!(!<b1::TakeBackItem as PacketCodec>::COMPRESSED);
    assert!(!<b1::StoreItem as PacketCodec>::COMPRESSED);
    assert!(!<b1::DepositRefineItem as PacketCodec>::COMPRESSED);
    assert!(!<b1::RetrieveRefineItem as PacketCodec>::COMPRESSED);
    assert!(!<b1::RefineCancel as PacketCodec>::COMPRESSED);
    assert!(!<b1::RefineItem as PacketCodec>::COMPRESSED);
    assert!(!<b1::DepositTradeItem as PacketCodec>::COMPRESSED);
    assert!(!<b1::RetrieveTradeItem as PacketCodec>::COMPRESSED);
    assert!(!<b1::SplitItem as PacketCodec>::COMPRESSED);
    assert!(!<b1::UseItem as PacketCodec>::COMPRESSED);
    assert!(!<b1::DropItem as PacketCodec>::COMPRESSED);
    assert!(!<b1::TakeBackHeroItem as PacketCodec>::COMPRESSED);
    assert!(!<b1::TransferHeroItem as PacketCodec>::COMPRESSED);
    assert!(!<b1::PlayerUpdate as PacketCodec>::COMPRESSED);
    assert!(!<b1::PlayerInspect as PacketCodec>::COMPRESSED);
    assert!(!<b1::MarriageRequest as PacketCodec>::COMPRESSED);
    assert!(!<b1::DivorceRequest as PacketCodec>::COMPRESSED);
    assert!(!<b1::MentorRequest as PacketCodec>::COMPRESSED);
    assert!(!<b1::TradeRequest as PacketCodec>::COMPRESSED);
    assert!(!<b1::TradeAccept as PacketCodec>::COMPRESSED);
    assert!(!<b1::TradeGold as PacketCodec>::COMPRESSED);
    assert!(!<b1::TradeItem as PacketCodec>::COMPRESSED);
    assert!(!<b1::TradeConfirm as PacketCodec>::COMPRESSED);
    assert!(!<b1::TradeCancel as PacketCodec>::COMPRESSED);
    assert!(!<b1::LogOutSuccess as PacketCodec>::COMPRESSED);
    assert!(!<b1::LogOutFailed as PacketCodec>::COMPRESSED);
    assert!(!<b1::ReturnToLogin as PacketCodec>::COMPRESSED);
    assert!(!<b1::TimeOfDay as PacketCodec>::COMPRESSED);
    assert!(!<b1::ChangeAMode as PacketCodec>::COMPRESSED);
    assert!(!<b1::ChangePMode as PacketCodec>::COMPRESSED);
    assert!(!<b1::ObjectItem as PacketCodec>::COMPRESSED);
    assert!(!<b1::ObjectGold as PacketCodec>::COMPRESSED);
    assert!(!<b1::GainedItem as PacketCodec>::COMPRESSED);
}
