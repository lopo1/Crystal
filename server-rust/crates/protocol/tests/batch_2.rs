//! batch_2 客户端包回环测试: write→read 必须完全一致，且读完后缓冲为空。
//! 字段顺序与 `Shared/ClientPackets.cs`（行 1012–1603）逐一对应。

use crystal_protocol::binary::{Point, Reader};
use crystal_protocol::client as c;
use crystal_protocol::frame::{encode_packet, PacketCodec};

fn roundtrip_client<P: PacketCodec + PartialEq + std::fmt::Debug>(p: P) {
    let frame = encode_packet(&p);
    let id = i16::from_le_bytes([frame[2], frame[3]]);
    assert_eq!(id, P::ID, "包 ID 不匹配");
    let payload = &frame[4..];
    let mut r = Reader::new(payload.to_vec());
    let decoded = P::read(&mut r).unwrap();
    assert_eq!(decoded, p, "回环不一致 (id={})", P::ID);
    assert!(r.is_empty(), "id={} 读取后有未消费字节", P::ID);
}

#[test]
fn request_info_packets_roundtrip() {
    roundtrip_client(c::RequestMonsterInfo { monster_index: -7 });
    roundtrip_client(c::RequestNPCInfo { npc_index: 123_456 });
    roundtrip_client(c::RequestItemInfo { item_index: 99_999 });
    roundtrip_client(c::TeleportToNPC {
        object_id: 0xDEAD_BEEF,
    });
    roundtrip_client(c::SearchMap {
        text: "比奇省 盟重".into(),
    });
}

#[test]
fn magic_packets_roundtrip() {
    roundtrip_client(c::MagicKey {
        spell: 7,
        key: 9,
        old_key: 0,
    });
    roundtrip_client(c::Magic {
        object_id: 1,
        spell: 3,
        direction: 4, // MirDirection::Down
        target_id: 2,
        location: Point::new(-5, 12),
        spell_target_lock: true,
    });
}

#[test]
fn group_packets_roundtrip() {
    roundtrip_client(c::SwitchGroup { allow_group: true });
    roundtrip_client(c::AddMember {
        name: "战士甲".into(),
    });
    roundtrip_client(c::DelMember {
        name: "法师乙".into(),
    });
    roundtrip_client(c::GroupInvite {
        accept_invite: true,
    });
}

#[test]
fn hero_packets_roundtrip() {
    roundtrip_client(c::NewHero {
        name: "英雄一号".into(),
        gender: 1, // MirGender::Female
        class: 2,  // MirClass::Taoist
    });
    roundtrip_client(c::SetAutoPotValue {
        stat: 3,
        value: 400,
    });
    roundtrip_client(c::SetAutoPotItem {
        grid: 1, // MirGridType::Inventory
        item_index: -2,
    });
    roundtrip_client(c::SetHeroBehaviour {
        behaviour: 2, // HeroBehaviour::Follow
    });
    roundtrip_client(c::ChangeHero { list_index: 0 });
}

#[test]
fn marriage_packets_roundtrip() {
    roundtrip_client(c::MarriageRequest);
    roundtrip_client(c::MarriageReply {
        accept_invite: false,
    });
    roundtrip_client(c::ChangeMarriage);
    roundtrip_client(c::DivorceRequest);
    roundtrip_client(c::DivorceReply {
        accept_invite: true,
    });
}

#[test]
fn mentor_packets_roundtrip() {
    roundtrip_client(c::AddMentor {
        name: "师傅大人".into(),
    });
    roundtrip_client(c::MentorReply {
        accept_invite: true,
    });
    roundtrip_client(c::AllowMentor);
    roundtrip_client(c::CancelMentor);
}

#[test]
fn trade_packets_roundtrip() {
    roundtrip_client(c::TradeRequest);
    roundtrip_client(c::TradeReply {
        accept_invite: false,
    });
    roundtrip_client(c::TradeGold { amount: 1_000_000 });
    roundtrip_client(c::TradeConfirm { locked: true });
    roundtrip_client(c::TradeCancel);
}

#[test]
fn misc_packets_roundtrip() {
    roundtrip_client(c::TownRevive);
    roundtrip_client(c::SpellToggle {
        spell: 5,
        can_use: -1,
    });
    roundtrip_client(c::ConsignItem {
        unique_id: 123_456_789,
        price: 5_000,
        r#type: 2, // MarketPanelType
    });
    roundtrip_client(c::GuildTerritoryPage { page: 3 });
    roundtrip_client(c::PurchaseGuildTerritory {
        owner: "行会名".into(),
    });
}

#[test]
fn market_packets_roundtrip() {
    roundtrip_client(c::MarketSearch {
        r#match: "木剑".into(),
        r#type: 1, // ItemType::Weapon
        usermode: true,
        min_shape: -100,
        max_shape: 5_000,
        market_type: 0, // MarketPanelType::Market
    });
    roundtrip_client(c::MarketRefresh);
}
