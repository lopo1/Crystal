//! batch_4 回环测试: 客户端→服务器包（ClientPackets.cs 行 2208–2704）。
//! 每个包 write→read 必须完全一致，且读取后缓冲耗尽（reader.is_empty()）。

use crystal_protocol::binary::{Point, Reader};
use crystal_protocol::client::batch_4 as b4;
use crystal_protocol::frame::{encode_packet, PacketCodec};
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
    let mut r = Reader::new(payload.to_vec());
    let decoded = P::read(&mut r).unwrap();
    assert!(r.is_empty(), "读取后缓冲未耗尽 (id={})", P::ID);
    assert_eq!(decoded, p, "回环不一致 (id={})", P::ID);
}

fn dummy_creature() -> ClientIntelligentCreature {
    ClientIntelligentCreature {
        pet_type: IntelligentCreatureType::BlackKitten,
        icon: 5,
        custom_name: "小黑".into(),
        fullness: 50,
        slot_index: 2,
        expire: 0x4000_0000_0000_0000, // DateTime.ToBinary()，往返直通
        blackstone_time: 123_456_789,
        pet_mode: IntelligentCreaturePickupMode::Automatic,
        creature_rules: IntelligentCreatureRules {
            minimal_fullness: 30,
            mouse_pickup_enabled: true,
            mouse_pickup_range: 5,
            auto_pickup_enabled: true,
            auto_pickup_range: 3,
            semi_auto_pickup_enabled: false,
            semi_auto_pickup_range: 0,
            can_produce_black_stone: true,
        },
        filter: IntelligentCreatureItemFilter {
            pet_pickup_all: true,
            pet_pickup_gold: true,
            pet_pickup_weapons: false,
            pet_pickup_armours: true,
            pet_pickup_helmets: false,
            pet_pickup_boots: false,
            pet_pickup_belts: true,
            pet_pickup_accessories: false,
            pet_pickup_others: true,
        },
        pickup_grade: 4,
        maintain_food_time: 999,
    }
}

#[test]
fn batch4_client_packets_roundtrip() {
    roundtrip_client(b4::LockMail {
        mail_id: 9_007_199_254_740_993,
        lock: true,
    });
    roundtrip_client(b4::MailLockedItem {
        unique_id: 123_456_789_012_345_678,
        locked: false,
    });
    roundtrip_client(b4::MailCost {
        gold: 250_000,
        items_idx: [11, 22, 33, 44, 55],
        stamped: true,
    });
    roundtrip_client(b4::RequestIntelligentCreatureUpdates { update: true });
    roundtrip_client(b4::UpdateIntelligentCreature {
        creature: dummy_creature(),
        summon_me: true,
        un_summon_me: false,
        release_me: true,
    });
    roundtrip_client(b4::IntelligentCreaturePickup {
        mouse_mode: true,
        location: Point::new(-320, 640),
    });
    roundtrip_client(b4::AddFriend {
        name: "老王".into(),
        blocked: true,
    });
    roundtrip_client(b4::RemoveFriend {
        character_index: 42,
    });
    roundtrip_client(b4::RefreshFriends);
    roundtrip_client(b4::AddMemo {
        character_index: 7,
        memo: "悍匪一个 🔥".into(),
    });
    roundtrip_client(b4::GuildBuffUpdate { action: 2, id: 9 });
    roundtrip_client(b4::GameshopBuy {
        g_index: 1001,
        quantity: 3,
        p_type: 1,
    });
    roundtrip_client(b4::NPCConfirmInput {
        npcid: 0xDEAD_BEEF,
        page_name: "main".into(),
        value: "甲乙丙丁123".into(),
    });
    roundtrip_client(b4::ReportIssue {
        image: vec![1, 2, 3, 4, 255, 0, 128],
        image_size: 1024,
        image_chunk: 7,
    });
    roundtrip_client(b4::GetRanking {
        rank_type: 1,
        rank_index: 12,
        online_only: true,
    });
    roundtrip_client(b4::Opendoor { door_index: 3 });
    roundtrip_client(b4::GetRentedItems);
    roundtrip_client(b4::ItemRentalRequest);
    roundtrip_client(b4::ItemRentalFee { amount: 88_000 });
    roundtrip_client(b4::ItemRentalPeriod { days: 30 });
    roundtrip_client(b4::DepositRentalItem { from: 0, to: 3 });
    roundtrip_client(b4::RetrieveRentalItem { from: 5, to: 1 });
    roundtrip_client(b4::CancelItemRental);
    roundtrip_client(b4::ItemRentalLockFee);
    roundtrip_client(b4::ItemRentalLockItem);
    roundtrip_client(b4::ConfirmItemRental);
    roundtrip_client(b4::DeleteItem {
        unique_id: 9_876_543_210,
        count: 250,
        hero_inventory: true,
    });
}
