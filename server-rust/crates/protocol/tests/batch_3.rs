//! CB3 批次回环测试: write→read 完全一致 + 无残留字节。
//!
//! 对应 `Shared/ClientPackets.cs` 行 1604–2206 的 32 个客户端包
//! （MarketPage .. DeleteMail），每个包用非平凡值验证。

use crystal_protocol::binary::Reader;
use crystal_protocol::client as c;
use crystal_protocol::frame::{encode_packet, PacketCodec};

/// 写→读 回环: 帧头长度/ID 正确、解码结果相等、载荷无未消费字节。
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
    assert!(r.is_empty(), "有未消费字节 (id={})", P::ID);
}

#[test]
fn market_packets_roundtrip() {
    roundtrip(c::MarketPage { page: 7 });
    roundtrip(c::MarketBuy {
        auction_id: 9_223_372_036_854_775_807, // u64::MAX
        bid_price: 5_000_000,
    });
    roundtrip(c::MarketSellNow {
        auction_id: 123_456_789_012_345,
    });
    roundtrip(c::MarketGetBack {
        mode: 2, // MarketCollectionMode
        auction_id: 42,
    });
}

#[test]
fn user_name_and_chat_item_roundtrip() {
    roundtrip(c::RequestUserName {
        user_id: 4_000_000_000,
    });
    roundtrip(c::RequestChatItem {
        chat_item_id: 999_999,
    });
}

#[test]
fn guild_packets_roundtrip() {
    roundtrip(c::EditGuildMember {
        change_type: 1,
        rank_index: 3,
        name: "张三".into(),
        rank_name: "会长".into(),
    });
    roundtrip(c::EditGuildNotice {
        notice: vec!["第一行公告".into(), "第二行 🔥".into(), String::new()],
    });
    roundtrip(c::GuildInvite {
        accept_invite: true,
    });
    roundtrip(c::RequestGuildInfo { r#type: 1 });
    roundtrip(c::GuildNameReturn {
        name: "龙腾四海".into(),
    });
    roundtrip(c::GuildWarReturn {
        name: "敌对行会".into(),
    });
}

#[test]
fn equip_and_fishing_roundtrip() {
    roundtrip(c::EquipSlotItem {
        grid: 2, // MirGridType: Equipment
        unique_id: 55,
        to: 12,
        grid_to: 3, // MirGridType: Trade
        to_unique_id: 66,
    });
    roundtrip(c::FishingCast { cast_out: true });
    roundtrip(c::FishingChangeAutocast { auto_cast: true });
}

#[test]
fn quest_packets_roundtrip() {
    roundtrip(c::AcceptQuest {
        npc_index: 300,
        quest_index: 5,
    });
    roundtrip(c::FinishQuest {
        quest_index: 1,
        selected_item_index: 4,
    });
    roundtrip(c::AbandonQuest { quest_index: 9 });
    roundtrip(c::ShareQuest { quest_index: 2 });
}

#[test]
fn reincarnation_packets_roundtrip() {
    roundtrip(c::AcceptReincarnation);
    roundtrip(c::CancelReincarnation);
}

#[test]
fn item_combine_and_awakening_roundtrip() {
    roundtrip(c::CombineItem {
        grid: 1, // MirGridType: Inventory
        id_from: 100,
        id_to: 200,
    });
    roundtrip(c::AwakeningNeedMaterials {
        unique_id: 7,
        r#type: 4, // AwakeType: Ac
    });
    roundtrip(c::AwakeningLockedItem {
        unique_id: 8,
        locked: true,
    });
    roundtrip(c::Awakening {
        unique_id: 9,
        r#type: 6, // AwakeType: Hpmp
        position_idx: 3,
    });
    roundtrip(c::DisassembleItem { unique_id: 10 });
    roundtrip(c::DowngradeAwakening { unique_id: 11 });
    roundtrip(c::ResetAddedItem { unique_id: 12 });
}

#[test]
fn mail_packets_roundtrip() {
    roundtrip(c::SendMail {
        name: "收件人".into(),
        message: "包裹内容 🔥".into(),
        gold: 10_000,
        items_idx: [1, 2, 3, 4, 5],
        stamped: true,
    });
    roundtrip(c::ReadMail { mail_id: 100 });
    roundtrip(c::CollectParcel { mail_id: 101 });
    roundtrip(c::DeleteMail { mail_id: 102 });
}
