//! 独立字节级验证: 槽布尔方向 true=有物品（与 C# UserInformation 一致）
use crystal_protocol::binary::{Reader, Writer};
use crystal_protocol::frame::{encode_packet, PacketCodec};
use crystal_protocol::server::{ObjectPlayer, UserInformation};
use crystal_protocol::types::UserItem;

#[test]
fn item_slots_direction_true_has_item() {
    let mut w = Writer::new();
    w.write_bool(true);   // 外层存在
    w.write_i32(1);       // len
    w.write_bool(true);   // 槽有物品
    UserItem::default().write(&mut w);
    let b = w.into_inner();
    assert_eq!(b[0], 1, "外层存在标志=1");
    assert_eq!(&b[1..5], &[1, 0, 0, 0][..], "len=1");
    assert_eq!(b[5], 1, "槽标志=1 表示有物品");
}

#[test]
fn user_information_slots_roundtrip() {
    let ui = UserInformation {
        object_id: 1, real_id: 1, name: "t".into(), guild_name: String::new(),
        guild_rank: String::new(), name_colour: crystal_protocol::binary::Argb(0),
        class: crystal_protocol::types::MirClass::Warrior, gender: crystal_protocol::types::MirGender::Male, level: 1,
        location: crystal_protocol::binary::Point::new(0,0), direction: crystal_protocol::types::MirDirection::Up,
        hair: 0, hp: 100, mp: 100, experience: 0, max_experience: 100,
        level_effects: crystal_protocol::types::LevelEffects(0),
        has_hero: false, hero_behaviour: 0,
        inventory: Some(vec![Some(UserItem { unique_id: 9, item_index: 5, ..Default::default() }), None]),
        equipment: None, quest_inventory: None,
        gold: 10, credit: 0, has_expanded_storage: false, has_storage_password: false,
        require_storage_password: false, storage_password_last_set: 0, expanded_storage_expiry_time: 0,
        magics: vec![], intelligent_creatures: vec![], summoned_creature_type: 0,
        creature_summoned: false, allow_observe: false, observer: false,
    };
    let f = encode_packet(&ui);
    let d = UserInformation::read(&mut Reader::new(f[4..].to_vec())).unwrap();
    assert_eq!(d.inventory, ui.inventory);
}

#[test]
fn object_player_write_read() {
    let op = ObjectPlayer { object_id: 3, name: "X".into(), ..Default::default() };
    let f = encode_packet(&op);
    let d = ObjectPlayer::read(&mut Reader::new(f[4..].to_vec())).unwrap();
    assert_eq!(d.object_id, 3);
    assert_eq!(d.name, "X");
}
