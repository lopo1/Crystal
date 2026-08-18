//! batch_1 回环测试: 每个包 write→read 一致 + reader.is_empty()。
//! 字段顺序与 C# `Shared/ClientPackets.cs`（322–996 行）逐字段对应。

use crystal_protocol::binary::{Point, Reader};
use crystal_protocol::client as c;
use crystal_protocol::frame::{encode_packet, PacketCodec};

fn roundtrip<P: PacketCodec + PartialEq + std::fmt::Debug>(p: P) {
    let frame = encode_packet(&p);
    assert_eq!(
        &frame[..2],
        &(frame.len() as u16).to_le_bytes()[..],
        "帧长度错误 (id={})",
        P::ID
    );
    let id = i16::from_le_bytes([frame[2], frame[3]]);
    assert_eq!(id, P::ID, "包 ID 不匹配");
    let payload = &frame[4..];
    let mut r = Reader::new(payload.to_vec());
    let decoded = P::read(&mut r).unwrap();
    assert_eq!(decoded, p, "回环不一致 (id={})", P::ID);
    assert!(r.is_empty(), "存在未消费字节 (id={})", P::ID);
}

#[test]
fn inventory_item_packets() {
    // MoveItem: Grid(byte) From(int) To(int)
    roundtrip(c::MoveItem {
        grid: 1,
        from: 5,
        to: 8,
    });
    // StoreItem / DepositRefineItem / RetrieveRefineItem / TakeBackItem / DepositTradeItem /
    // RetrieveTradeItem / TakeBackHeroItem / TransferHeroItem: From(int) To(int)
    roundtrip(c::StoreItem { from: 3, to: 7 });
    roundtrip(c::DepositRefineItem { from: 1, to: 2 });
    roundtrip(c::RetrieveRefineItem { from: 9, to: 0 });
    roundtrip(c::TakeBackItem { from: 4, to: 6 });
    roundtrip(c::DepositTradeItem { from: 2, to: 3 });
    roundtrip(c::RetrieveTradeItem { from: 6, to: 5 });
    roundtrip(c::TakeBackHeroItem { from: 0, to: 12 });
    roundtrip(c::TransferHeroItem { from: 12, to: 0 });

    // MergeItem: GridFrom(byte) GridTo(byte) IDFrom(ulong) IDTo(ulong)
    roundtrip(c::MergeItem {
        grid_from: 1,
        grid_to: 2,
        id_from: 0x1122_3344_5566_7788,
        id_to: 0xAABB_CCDD_EEFF_0011,
    });
    // EquipItem / RemoveItem: Grid(byte) UniqueID(ulong) To(int)
    roundtrip(c::EquipItem {
        grid: 2,
        unique_id: 4242,
        to: 3,
    });
    roundtrip(c::RemoveItem {
        grid: 2,
        unique_id: 4242,
        to: 4,
    });
    // RemoveSlotItem: Grid(byte) GridTo(byte) UniqueID(ulong) To(int) FromUniqueID(ulong)
    roundtrip(c::RemoveSlotItem {
        grid: 2,
        grid_to: 1,
        unique_id: 55,
        to: 0,
        from_unique_id: 99,
    });
    // SplitItem: Grid(byte) UniqueID(ulong) Count(ushort)
    roundtrip(c::SplitItem {
        grid: 1,
        unique_id: 7,
        count: 12,
    });
    // UseItem: UniqueID(ulong) Grid(byte) —— 注意 UniqueID 在前
    roundtrip(c::UseItem {
        unique_id: 8,
        grid: 1,
    });
    // DropItem: UniqueID(ulong) Count(ushort) HeroInventory(bool)
    roundtrip(c::DropItem {
        unique_id: 9,
        count: 3,
        hero_inventory: true,
    });

    // UniqueID 单字段包
    roundtrip(c::RefineItem { unique_id: 123 });
    roundtrip(c::CheckRefine { unique_id: 456 });
    roundtrip(c::ReplaceWedRing { unique_id: 789 });
    roundtrip(c::RepairItem { unique_id: 321 });
    roundtrip(c::SRepairItem { unique_id: 654 });
}

#[test]
fn refiner_trade_noop_packets() {
    roundtrip(c::RefineCancel);
    roundtrip(c::PickUp);
}

#[test]
fn gold_observe_mode_packets() {
    roundtrip(c::DropGold {
        amount: 4_000_000_000,
    });
    // Inspect: ObjectID(uint) Ranking(bool) Hero(bool)
    roundtrip(c::Inspect {
        object_id: 1001,
        ranking: true,
        hero: false,
    });
    roundtrip(c::Observe {
        name: "老玩家".into(),
    });
    // ChangeAMode: AttackMode(byte) / ChangePMode: PetMode(byte)
    roundtrip(c::ChangeAMode { mode: 3 }); // EnemyGuild
    roundtrip(c::ChangePMode { mode: 2 }); // AttackOnly
    roundtrip(c::ChangeTrade { allow_trade: true });
    roundtrip(c::RequestMapInfo { map_index: 42 });
}

#[test]
fn combat_packets() {
    // Attack: Direction(byte) Spell(byte)
    roundtrip(c::Attack {
        direction: 5,
        spell: 3,
    });
    // RangeAttack: Direction(byte) Location(Point) TargetID(uint) TargetLocation(Point)
    roundtrip(c::RangeAttack {
        direction: 1,
        location: Point::new(10, 20),
        target_id: 77,
        target_location: Point::new(30, 40),
    });
    roundtrip(c::Harvest { direction: 6 });
}

#[test]
fn npc_shop_packets() {
    roundtrip(c::CallNPC {
        object_id: 66,
        key: "start".into(),
    });
    // BuyItem: ItemIndex(ulong) Count(ushort) Type(PanelType byte)
    roundtrip(c::BuyItem {
        item_index: 500,
        count: 2,
        r#type: 1, // BuySub
    });
    roundtrip(c::SellItem {
        unique_id: 501,
        count: 5,
    });
    roundtrip(c::BuyItemBack {
        unique_id: 502,
        count: 1,
    });
    // CraftItem: UniqueID(ulong) Count(ushort) Slots(int[])
    roundtrip(c::CraftItem {
        unique_id: 503,
        count: 2,
        slots: vec![1, 2, 3, 4],
    });
}
