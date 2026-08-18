//! batch_7（补齐批）回环测试

use crystal_protocol::client as c;
use crystal_protocol::frame::{decode_packet, encode_packet, PacketCodec};
use crystal_protocol::server as s;
use crystal_protocol::types::{ItemInfo, Stats};

fn ci<P: PacketCodec + PartialEq + std::fmt::Debug>(p: P) {
    let f = encode_packet(&p);
    let id = i16::from_le_bytes([f[2], f[3]]);
    assert_eq!(id, P::ID);
    let d = P::read(&mut crystal_protocol::binary::Reader::new(f[4..].to_vec())).unwrap();
    assert_eq!(d, p, "client 回环失败 (id={})", P::ID);
}

fn si<P: PacketCodec + PartialEq + std::fmt::Debug>(p: P) {
    let f = encode_packet(&p);
    let id = i16::from_le_bytes([f[2], f[3]]);
    assert_eq!(id, P::ID);
    let d = P::read(&mut crystal_protocol::binary::Reader::new(f[4..].to_vec())).unwrap();
    assert_eq!(d, p, "server 回环失败 (id={})", P::ID);
}

#[test]
fn client_storage_packets() {
    ci(c::UnlockStorage {
        password: "pw123".into(),
    });
    ci(c::SetStoragePassword {
        current_password: "a".into(),
        new_password: "b".into(),
    });
    ci(c::RemoveStoragePassword {
        current_password: "x".into(),
    });
    ci(c::GuildStorageGoldChange {
        r#type: 1,
        amount: 5000,
    });
    ci(c::GuildStorageItemChange {
        r#type: 2,
        from: 0,
        to: 3,
    });
}

#[test]
fn server_storage_splititem_newitem() {
    si(s::StorageUnlockResult {
        result: 0,
        has_password: true,
    });
    si(s::StoragePasswordResult {
        result: 4,
        removing: false,
        has_password: true,
        last_set_time: 123456789,
    });
    si(s::SplitItem1 {
        grid: 1,
        unique_id: 42,
        count: 5,
        success: true,
    });
    si(s::NewItemInfo {
        info: ItemInfo {
            index: 1,
            name: "木剑".into(),
            item_type: 2,
            grade: 1,
            required_type: 0,
            required_class: 0,
            required_gender: 0,
            set: 0,
            shape: 0,
            weight: 5,
            light: 0,
            required_amount: 1,
            image: 100,
            durability: 30,
            stack_size: 1,
            price: 100,
            start_item: false,
            effect: 0,
            need_identify: false,
            show_group_pickup: false,
            class_based: false,
            level_based: false,
            can_mine: false,
            global_drop_notify: false,
            bind: 0,
            unique: 1,
            random_stats_id: 0,
            can_fast_run: false,
            can_awakening: false,
            slots: 0,
            stats: Stats {
                values: vec![(1, 5)],
            },
            tool_tip: Some("新手装备".into()),
        },
    });
}

#[test]
fn dispatch_integration() {
    // 校验新包可通过 ServerPacket/ClientPacket 分发解码
    use crystal_protocol::ServerPacket;
    let p = s::SplitItem1 {
        grid: 0,
        unique_id: 7,
        count: 1,
        success: true,
    };
    let f = encode_packet(&p);
    let id = i16::from_le_bytes([f[2], f[3]]);
    let decoded = ServerPacket::decode(id, &f[4..]).unwrap();
    match decoded {
        ServerPacket::SplitItem1(d) => assert_eq!(d, p),
        x => panic!("分发到错误变体: {x:?}"),
    }
}
