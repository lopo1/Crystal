//! batch_6 回环测试: 写→读 必须完全一致，且 reader 消费完毕。
//! 覆盖 ServerPackets.cs 末尾一节（邮件/灵兽/租赁/排行/公告等，ID 228–273 及 33/34/112）。

use crystal_protocol::binary::{datetime_to_binary, DateTimeKind, Point, Reader};
use crystal_protocol::frame::{encode_packet, PacketCodec};
use crystal_protocol::server as s;
use crystal_protocol::types::*;

fn roundtrip<P: PacketCodec + PartialEq + std::fmt::Debug>(p: P) {
    let frame = encode_packet(&p);
    let id = i16::from_le_bytes([frame[2], frame[3]]);
    assert_eq!(id, P::ID, "包 ID 不匹配");
    let payload = &frame[4..];
    let mut r = Reader::new(payload.to_vec());
    let decoded = P::read(&mut r).unwrap();
    assert_eq!(decoded, p, "回环不一致 (id={})", P::ID);
    assert!(r.is_empty(), "reader 未消费完 (id={})", P::ID);
}

fn dt(secs: i64) -> i64 {
    datetime_to_binary(secs, DateTimeKind::Utc)
}

fn sample_stats() -> Stats {
    Stats {
        values: vec![(0, 5), (1, -3), (2, 100)],
    }
}

fn sample_item(unique: u64, index: i32) -> UserItem {
    UserItem {
        unique_id: unique,
        item_index: index,
        current_dura: 65000,
        max_dura: 65000,
        count: 3,
        soul_bound_id: 42,
        identified: true,
        cursed: false,
        slots: vec![
            None,
            Some(Box::new(UserItem {
                unique_id: 99,
                item_index: 88,
                current_dura: 1,
                max_dura: 100,
                count: 5,
                soul_bound_id: -1,
                identified: true,
                cursed: true,
                ..Default::default()
            })),
        ],
        gem_count: 2,
        added_stats: sample_stats(),
        awake: Awake {
            r#type: AwakeType::Dc,
            list: vec![1, 2, 3],
        },
        refined_value: 7,
        refine_added: 3,
        refine_success_chance: 55,
        wedding_ring: -1,
        expire_info: Some(ExpireInfo {
            expiry_date: 123_456_789,
        }),
        rental_information: Some(RentalInformation {
            owner_name: "玩家甲".into(),
            binding_flags: 1,
            expiry_date: 987_654_321,
            rental_locked: true,
        }),
        is_shop_item: true,
        sealed_info: Some(SealedInfo {
            expiry_date: 111,
            next_seal_date: 222,
        }),
        gm_made: false,
    }
}

fn sample_item_info(index: i32) -> ItemInfo {
    ItemInfo {
        index,
        name: format!("材料{index}"),
        item_type: 1,
        grade: 3,
        required_type: 0,
        required_class: 2,
        required_gender: 0,
        set: 0,
        shape: -1,
        weight: 10,
        light: 2,
        required_amount: 1,
        image: 500,
        durability: 1000,
        stack_size: 99,
        price: 5000,
        start_item: false,
        effect: 0,
        need_identify: true,
        show_group_pickup: false,
        class_based: true,
        level_based: false,
        can_mine: true,
        global_drop_notify: false,
        bind: 0,
        unique: -1,
        random_stats_id: 0,
        can_fast_run: true,
        can_awakening: false,
        slots: 0,
        stats: sample_stats(),
        tool_tip: Some("觉醒材料".into()),
    }
}

fn sample_creature() -> ClientIntelligentCreature {
    ClientIntelligentCreature {
        pet_type: IntelligentCreatureType::Baekdon,
        icon: 100,
        custom_name: "小白".into(),
        fullness: 50,
        slot_index: 2,
        expire: dt(1_700_000_000),
        blackstone_time: 12_345,
        pet_mode: IntelligentCreaturePickupMode::Automatic,
        creature_rules: IntelligentCreatureRules {
            minimal_fullness: 10,
            mouse_pickup_enabled: true,
            mouse_pickup_range: 3,
            auto_pickup_enabled: false,
            auto_pickup_range: 0,
            semi_auto_pickup_enabled: true,
            semi_auto_pickup_range: 5,
            can_produce_black_stone: true,
        },
        filter: IntelligentCreatureItemFilter {
            pet_pickup_all: true,
            pet_pickup_gold: false,
            pet_pickup_weapons: true,
            pet_pickup_armours: false,
            pet_pickup_helmets: true,
            pet_pickup_boots: false,
            pet_pickup_belts: true,
            pet_pickup_accessories: false,
            pet_pickup_others: true,
        },
        pickup_grade: 2,
        maintain_food_time: 86_400,
    }
}

fn sample_mail() -> ClientMail {
    ClientMail {
        mail_id: 0xFEDCBA9876543210,
        sender_name: "发件人".into(),
        message: "包裹内容 🔥".into(),
        opened: true,
        locked: false,
        can_reply: true,
        collected: false,
        date_sent: dt(1_650_000_000),
        gold: 12_345,
        items: vec![sample_item(1, 100), sample_item(2, 200)],
    }
}

#[test]
fn awakening_need_materials_roundtrip() {
    // 有数据: 含非空与空槽
    roundtrip(s::AwakeningNeedMaterials {
        materials: Some(vec![
            Some((sample_item_info(1), 5)),
            None,
            Some((sample_item_info(2), 9)),
        ]),
    });
    // 无数据 (Materials == null)
    roundtrip(s::AwakeningNeedMaterials { materials: None });
}

#[test]
fn awakening_locked_item_roundtrip() {
    roundtrip(s::AwakeningLockedItem {
        unique_id: 0x123456789ABCDEF0,
        locked: true,
    });
}

#[test]
fn awakening_roundtrip() {
    roundtrip(s::Awakening {
        result: 1,
        remove_id: -99,
    });
}

#[test]
fn receive_mail_roundtrip() {
    roundtrip(s::ReceiveMail {
        mail: vec![
            sample_mail(),
            ClientMail {
                mail_id: 7,
                sender_name: "乙".into(),
                message: "回信".into(),
                opened: false,
                locked: true,
                can_reply: false,
                collected: true,
                date_sent: dt(1),
                gold: 0,
                items: vec![],
            },
        ],
    });
}

#[test]
fn mail_locked_item_roundtrip() {
    roundtrip(s::MailLockedItem {
        unique_id: 5,
        locked: false,
    });
}

#[test]
fn mail_send_request_roundtrip() {
    roundtrip(s::MailSendRequest);
}

#[test]
fn mail_sent_roundtrip() {
    roundtrip(s::MailSent { result: -2 });
}

#[test]
fn parcel_collected_roundtrip() {
    roundtrip(s::ParcelCollected { result: 1 });
}

#[test]
fn mail_cost_roundtrip() {
    roundtrip(s::MailCost { cost: 4_000_000 });
}

#[test]
fn resize_inventory_roundtrip() {
    roundtrip(s::ResizeInventory { size: 120 });
}

#[test]
fn resize_storage_roundtrip() {
    roundtrip(s::ResizeStorage {
        size: 200,
        has_expanded_storage: true,
        expiry_time: dt(1_800_000_000),
    });
}

#[test]
fn new_intelligent_creature_roundtrip() {
    roundtrip(s::NewIntelligentCreature {
        creature: sample_creature(),
    });
}

#[test]
fn update_intelligent_creature_list_roundtrip() {
    roundtrip(s::UpdateIntelligentCreatureList {
        creature_list: vec![sample_creature(), sample_creature()],
        creature_summoned: true,
        summoned_creature_type: 4, // IntelligentCreatureType.Baekdon
        pearl_count: 7,
    });
}

#[test]
fn intelligent_creature_enable_rename_roundtrip() {
    roundtrip(s::IntelligentCreatureEnableRename);
}

#[test]
fn intelligent_creature_pickup_roundtrip() {
    roundtrip(s::IntelligentCreaturePickup { object_id: 999 });
}

#[test]
fn npc_pearl_goods_roundtrip() {
    roundtrip(s::NPCPearlGoods {
        list: vec![sample_item(11, 300), sample_item(12, 400)],
        rate: 1.5,
        r#type: 5, // PanelType.Repair
    });
}

#[test]
fn friend_update_roundtrip() {
    roundtrip(s::FriendUpdate {
        friends: vec![
            ClientFriend {
                index: 1,
                name: "好友甲".into(),
                memo: "备注".into(),
                blocked: false,
                online: true,
            },
            ClientFriend {
                index: 2,
                name: "好友乙".into(),
                memo: String::new(),
                blocked: true,
                online: false,
            },
        ],
    });
}

#[test]
fn lover_update_roundtrip() {
    roundtrip(s::LoverUpdate {
        name: "爱人".into(),
        date: dt(1_600_000_000),
        map_name: "比奇省".into(),
        married_days: -3,
    });
}

#[test]
fn mentor_update_roundtrip() {
    roundtrip(s::MentorUpdate {
        name: "师傅".into(),
        level: 45,
        online: true,
        mentee_exp: 12_345_678,
    });
}

#[test]
fn guild_buff_list_roundtrip() {
    roundtrip(s::GuildBuffList {
        remove: 1,
        active_buffs: vec![
            s::GuildBuff {
                id: 100,
                active: true,
                active_time_remaining: 3600,
            },
            s::GuildBuff {
                id: 200,
                active: false,
                active_time_remaining: 0,
            },
        ],
        guild_buffs: vec![
            s::GuildBuffInfo {
                id: 100,
                icon: 3,
                name: "行会攻击".into(),
                level_requirement: 10,
                points_requirement: 5,
                time_limit: 7200,
                activation_cost: 500,
                stats: sample_stats(),
            },
            s::GuildBuffInfo {
                id: 101,
                icon: 4,
                name: String::new(),
                level_requirement: 0,
                points_requirement: 1,
                time_limit: -1,
                activation_cost: 0,
                stats: Stats { values: vec![] },
            },
        ],
    });
}

#[test]
fn npc_request_input_roundtrip() {
    roundtrip(s::NPCRequestInput {
        npc_id: 77,
        page_name: "main".into(),
    });
}

#[test]
fn rankings_roundtrip() {
    roundtrip(s::Rankings {
        rank_type: 3,
        my_rank: 2,
        listing_details: vec![
            RankCharacterInfo {
                player_id: 1_000_001,
                name: "榜首".into(),
                level: 99,
                class: MirClass::Warrior,
            },
            RankCharacterInfo {
                player_id: 2,
                name: "次席".into(),
                level: 88,
                class: MirClass::Wizard,
            },
        ],
        listings: vec![-1, 9_223_372_036_854_775_807],
        count: 5,
    });
}

#[test]
fn opendoor_roundtrip() {
    roundtrip(s::Opendoor {
        door_index: 3,
        close: true,
    });
}

#[test]
fn get_rented_items_roundtrip() {
    roundtrip(s::GetRentedItems {
        rented_items: vec![
            s::ItemRentalInformation {
                item_id: 88,
                item_name: "屠龙".into(),
                renting_player_name: "租客".into(),
                item_return_date: dt(1_700_000_000),
            },
            s::ItemRentalInformation {
                item_id: 89,
                item_name: String::new(),
                renting_player_name: String::new(),
                item_return_date: 0,
            },
        ],
    });
}

#[test]
fn item_rental_request_roundtrip() {
    roundtrip(s::ItemRentalRequest {
        name: "交易对象".into(),
        renting: true,
    });
}

#[test]
fn item_rental_fee_roundtrip() {
    roundtrip(s::ItemRentalFee { amount: 9_999 });
}

#[test]
fn item_rental_period_roundtrip() {
    roundtrip(s::ItemRentalPeriod { days: 30 });
}

#[test]
fn deposit_rental_item_roundtrip() {
    roundtrip(s::DepositRentalItem {
        from: 1,
        to: 5,
        success: true,
    });
}

#[test]
fn retrieve_rental_item_roundtrip() {
    roundtrip(s::RetrieveRentalItem {
        from: 2,
        to: 6,
        success: false,
    });
}

#[test]
fn update_rental_item_roundtrip() {
    roundtrip(s::UpdateRentalItem {
        loan_item: Some(sample_item(7, 500)),
    });
    roundtrip(s::UpdateRentalItem { loan_item: None });
}

#[test]
fn cancel_item_rental_roundtrip() {
    roundtrip(s::CancelItemRental);
}

#[test]
fn item_rental_lock_roundtrip() {
    roundtrip(s::ItemRentalLock {
        success: true,
        gold_locked: false,
        item_locked: true,
    });
}

#[test]
fn item_rental_partner_lock_roundtrip() {
    roundtrip(s::ItemRentalPartnerLock {
        gold_locked: true,
        item_locked: false,
    });
}

#[test]
fn can_confirm_item_rental_roundtrip() {
    roundtrip(s::CanConfirmItemRental);
}

#[test]
fn confirm_item_rental_roundtrip() {
    roundtrip(s::ConfirmItemRental);
}

#[test]
fn new_recipe_info_roundtrip() {
    roundtrip(s::NewRecipeInfo {
        info: ClientRecipeInfo {
            gold: 1000,
            chance: 80,
            item: sample_item(1, 600),
            tools: vec![sample_item(2, 601), sample_item(3, 602)],
            ingredients: vec![sample_item(4, 700)],
        },
    });
}

#[test]
fn craft_item_roundtrip() {
    roundtrip(s::CraftItem { success: true });
}

#[test]
fn open_browser_roundtrip() {
    roundtrip(s::OpenBrowser {
        url: "https://example.com/game".into(),
    });
}

#[test]
fn play_sound_roundtrip() {
    roundtrip(s::PlaySound { sound: -1_000 });
}

#[test]
fn set_timer_roundtrip() {
    roundtrip(s::SetTimer {
        key: "限时任务".into(),
        r#type: 2, // TimerType
        seconds: 30,
    });
}

#[test]
fn expire_timer_roundtrip() {
    roundtrip(s::ExpireTimer {
        key: "限时任务".into(),
    });
}

#[test]
fn update_notice_roundtrip() {
    roundtrip(s::UpdateNotice {
        notice: s::Notice {
            title: "维护公告".into(),
            message: "服务器将于今晚维护 🔧".into(),
        },
    });
}

#[test]
fn roll_roundtrip() {
    roundtrip(s::Roll {
        r#type: 1,
        page: "roll_page".into(),
        result: 2,
        auto_roll: true,
    });
}

#[test]
fn set_compass_roundtrip() {
    roundtrip(s::SetCompass {
        location: Point::new(-123, 456),
    });
}

#[test]
fn new_monster_info_roundtrip() {
    roundtrip(s::NewMonsterInfo {
        info: ClientMonsterInfo {
            index: 42,
            name: "鸡".into(),
            game_name: "Chick".into(),
            image: 500,
            ai: 1,
            effect: 2,
            level: 10,
            view_range: 8,
            cool_eye: 3,
            light: 1,
            attack_speed: 1000,
            move_speed: 500,
            experience: 123_456,
            can_push: true,
            can_tame: false,
            auto_rev: true,
            undead: false,
            can_recall: true,
            stats: sample_stats(),
        },
    });
}

#[test]
fn new_npc_info_roundtrip() {
    roundtrip(s::NewNPCInfo {
        info: ClientNpcInfo {
            index: 7,
            file_name: "NPC_01".into(),
            name: "屠夫".into(),
            map_index: 5,
            location: Point::new(10, 20),
            image: 300,
            rate: 50,
            show_on_big_map: true,
            big_map_icon: 9,
            object_id: 8_000_001,
            icon: 4,
            can_teleport_to: true,
        },
    });
}
