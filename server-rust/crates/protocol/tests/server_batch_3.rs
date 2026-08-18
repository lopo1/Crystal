//! 服务器批次 3 数据包回环测试（Shared/ServerPackets.cs 行 3074–4009）。
//!
//! 规则: 写→读 完全一致 + reader.is_empty()；NPCGoods 是唯一 Compressed 包，
//! 其测试必须走 `encode_packet`/`decode_packet`（自动压缩/解压），并验证 gzip 魔数。

use crystal_protocol::binary::{Point, Reader};
use crystal_protocol::frame::{decode_packet, encode_packet, PacketCodec};
use crystal_protocol::server::batch_3::*;
use crystal_protocol::types::*;

fn rt<P: PacketCodec + PartialEq + std::fmt::Debug>(p: P) {
    let frame = encode_packet(&p);
    let id = i16::from_le_bytes([frame[2], frame[3]]);
    assert_eq!(id, P::ID, "包 ID 不匹配 (id={})", P::ID);
    let payload = &frame[4..];
    if P::COMPRESSED {
        // 压缩包: 载荷必须是 gzip 流，用 decode_packet 自动解压（勿手动解压）
        assert_eq!(
            &payload[..2],
            &[0x1f, 0x8b],
            "压缩包载荷应为 gzip 流 (id={})",
            P::ID
        );
        let decoded = decode_packet::<P>(id, payload).unwrap();
        assert_eq!(decoded, p, "回环不一致 (id={})", P::ID);
    } else {
        let mut r = Reader::new(payload.to_vec());
        let decoded = P::read(&mut r).unwrap();
        assert_eq!(decoded, p, "回环不一致 (id={})", P::ID);
        assert!(r.is_empty(), "有未消费字节 (id={})", P::ID);
    }
}

fn fixture_item(uid: u64, idx: i32) -> UserItem {
    UserItem {
        unique_id: uid,
        item_index: idx,
        current_dura: 30,
        max_dura: 40,
        count: 1,
        soul_bound_id: -1,
        identified: true,
        cursed: false,
        slots: vec![],
        gem_count: 0,
        added_stats: Stats {
            values: vec![(2, 7)],
        },
        awake: Awake {
            r#type: AwakeType::None,
            list: vec![],
        },
        refined_value: 0,
        refine_added: 0,
        refine_success_chance: 0,
        wedding_ring: -1,
        expire_info: None,
        rental_information: None,
        is_shop_item: false,
        sealed_info: None,
        gm_made: false,
    }
}

#[test]
fn npc_shop_packets_roundtrip() {
    // NPCGoods —— 唯一 Compressed 包（自动压缩/解压）
    rt(NPCGoods {
        list: vec![fixture_item(1, 300), fixture_item(2, 301)],
        rate: 1.5,
        r#type: 2, // PanelType.Craft
        hide_added_stats: true,
    });
    rt(NPCSell);
    rt(NPCRepair { rate: 0.75 });
    rt(NPCSRepair { rate: 0.9 });
    rt(NPCRefine {
        rate: 0.3,
        refining: true,
    });
    rt(NPCCheckRefine);
    rt(NPCCollectRefine { success: true });
    rt(NPCReplaceWedRing { rate: 0.25 });
    rt(NPCStorage);
}

#[test]
fn item_packets_roundtrip() {
    rt(SellItem {
        unique_id: 0x1122_3344_5566_7788,
        count: 99,
        success: true,
    });
    rt(RepairItem {
        unique_id: 0xDEAD_BEEF_0000_0001,
    });
    rt(ItemRepaired {
        unique_id: 7,
        max_dura: 300,
        current_dura: 120,
    });
    rt(ItemSlotSizeChanged {
        unique_id: 8,
        slot_size: 4,
    });
    rt(ItemSealChanged {
        unique_id: 9,
        expiry_date: 638_123_456_789_012_345, // DateTime.ToBinary()
    });
    // UserStorage: 非标准空槽布尔方向（true=有物品），含 null 槽与整体 null
    rt(UserStorage {
        storage: Some(vec![
            Some(fixture_item(10, 400)),
            None,
            Some(fixture_item(11, 401)),
        ]),
    });
    rt(UserStorage { storage: None });
}

#[test]
fn magic_packets_roundtrip() {
    let magic = ClientMagic {
        name: "冰咆哮".into(),
        spell: 0,
        base_cost: 10,
        level_cost: 2,
        icon: 3,
        level1: 4,
        level2: 5,
        level3: 6,
        need1: 7,
        need2: 8,
        need3: 9,
        level: 2,
        key: 1,
        experience: 100,
        delay: 1234,
        range: 5,
        cast_time: 600,
    };
    rt(NewMagic {
        magic: magic.clone(),
        hero: true,
    });
    rt(RemoveMagic { place_id: 5 });
    rt(MagicLeveled {
        object_id: 1,
        spell: 3,
        level: 2,
        experience: 100,
    });
    rt(Magic {
        spell: 0,
        target_id: 10,
        target: Point::new(3, 4),
        cast: true,
        level: 1,
        secondary_target_ids: vec![5, 6, 7],
    });
    rt(MagicDelay {
        object_id: 1,
        spell: 2,
        delay: -123_456_789,
    });
    rt(MagicCast { spell: 4 });
    rt(ObjectMagic {
        object_id: 100,
        location: Point::new(10, 20),
        direction: MirDirection::DownRight,
        spell: 1,
        target_id: 200,
        target: Point::new(11, 21),
        cast: false,
        level: 3,
        self_broadcast: true,
        secondary_target_ids: vec![7, 8, 9],
    });
    rt(ObjectEffect {
        object_id: 100,
        effect: 3, // SpellEffect
        effect_type: 0xDEAD_BEEF,
        delay_time: 100,
        time: 200,
    });
    rt(ObjectProjectile {
        spell: 0,
        source: 1,
        destination: 2,
    });
    rt(RangeAttack {
        target_id: 42,
        target: Point::new(5, 6),
        spell: 1,
    });
    rt(SpellToggle {
        object_id: 42,
        spell: 1,
        can_use: true,
    });
    rt(ObjectRangeAttack {
        object_id: 77,
        location: Point::new(8, 9),
        direction: MirDirection::UpLeft,
        target_id: 78,
        target: Point::new(8, 11),
        r#type: 0,
        spell: 2,
        level: 1,
    });
}

#[test]
fn movement_and_group_packets_roundtrip() {
    rt(Pushed {
        location: Point::new(10, 12),
        direction: MirDirection::UpRight,
    });
    rt(ObjectPushed {
        object_id: 5,
        location: Point::new(3, 3),
        direction: MirDirection::Down,
    });
    rt(ObjectName {
        object_id: 5,
        name: "测试NPC".into(),
    });
    rt(SwitchGroup { allow_group: true });
    rt(DeleteGroup);
    rt(DeleteMember {
        name: "队员A".into(),
    });
    rt(GroupInvite {
        name: "候选B".into(),
    });
    rt(AddMember {
        name: "队员C".into(),
    });
    rt(GroupMembersMap {
        player_name: "小明".into(),
        player_map: "比奇省".into(),
    });
    rt(SendMemberLocation {
        member_name: "小红".into(),
        member_location: Point::new(33, 44),
    });
}

#[test]
fn combat_and_buff_packets_roundtrip() {
    rt(Revived);
    rt(ObjectRevived {
        object_id: 6,
        effect: true,
    });
    rt(ObjectHealth {
        object_id: 6,
        percent: 75,
        expire: 1,
    });
    rt(ObjectMana {
        object_id: 6,
        percent: 50,
    });
    rt(MapEffect {
        location: Point::new(1, 2),
        effect: 0, // SpellEffect
        value: 3,
    });
    rt(AllowObserve { allow: true });
    rt(AddBuff {
        buff: ClientBuff {
            buff_type: 0,
            visible: true,
            object_id: 99,
            expire_time: -1,
            infinite: false,
            paused: false,
            stats: Stats {
                values: vec![(0, 5), (1, 10)],
            },
            values: vec![1, 2, 3],
        },
    });
    rt(RemoveBuff {
        r#type: 0, // BuffType
        object_id: 99,
    });
}
