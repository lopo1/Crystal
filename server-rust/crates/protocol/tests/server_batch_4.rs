//! 服务器→客户端包 batch_4 回环测试（对应 `Shared/ServerPackets.cs` 4001–4900 行）。
//!
//! 每个包: write→read 必须相等，且 `reader.is_empty()`。用非平凡值（非 0、非空字符串、含子列表）。

use crystal_protocol::binary::{datetime_to_binary, DateTimeKind, Point, Reader, Writer};
use crystal_protocol::frame::PacketCodec;
use crystal_protocol::server as s;
use crystal_protocol::server::{
    BaseStat, BaseStats, ClientGtMap, GuildMember, GuildRank, GuildStorageItem,
    GuildStorageItemChangeItem,
};
use crystal_protocol::types::*;

fn roundtrip<P: PacketCodec + PartialEq + std::fmt::Debug>(p: P) {
    let mut w = Writer::new();
    p.write(&mut w);
    let bytes = w.into_inner();
    let mut r = Reader::new(bytes);
    let decoded = P::read(&mut r).unwrap();
    assert_eq!(decoded, p, "回环不一致 (id={})", P::ID);
    assert!(r.is_empty(), "包未消费全部字节 (id={})", P::ID);
}

// 构造一个非平凡 UserItem
fn sample_item() -> UserItem {
    UserItem {
        unique_id: 42,
        item_index: 100,
        current_dura: 20,
        max_dura: 30,
        count: 3,
        soul_bound_id: -1,
        identified: true,
        cursed: false,
        slots: vec![
            None,
            Some(Box::new(UserItem {
                unique_id: 7,
                item_index: 101,
                ..Default::default()
            })),
        ],
        gem_count: 1,
        added_stats: Stats {
            values: vec![(1, 5), (5, 10)],
        },
        awake: Awake {
            r#type: AwakeType::Dc,
            list: vec![1, 2],
        },
        refined_value: 1,
        refine_added: 2,
        refine_success_chance: 70,
        wedding_ring: -1,
        expire_info: Some(ExpireInfo {
            expiry_date: datetime_to_binary(1_700_000_000, DateTimeKind::Utc),
        }),
        rental_information: Some(RentalInformation {
            owner_name: "租户".into(),
            binding_flags: 3,
            expiry_date: 456,
            rental_locked: true,
        }),
        is_shop_item: false,
        sealed_info: Some(SealedInfo {
            expiry_date: 789,
            next_seal_date: 790,
        }),
        gm_made: false,
    }
}

fn sample_auction() -> ClientAuction {
    ClientAuction {
        auction_id: 1001,
        item: sample_item(),
        seller: "寄售商".into(),
        price: 4_000_000,
        consignment_date: datetime_to_binary(1_700_001_000, DateTimeKind::Utc),
        item_type: 3, // MarketItemType
    }
}

#[test]
fn pause_buff_roundtrip() {
    roundtrip(s::PauseBuff {
        r#type: 5, // BuffType
        object_id: 1234,
        paused: true,
    });
}

#[test]
fn object_hidden_roundtrip() {
    roundtrip(s::ObjectHidden {
        object_id: 987,
        hidden: true,
    });
}

#[test]
fn refresh_item_roundtrip() {
    roundtrip(s::RefreshItem {
        item: sample_item(),
    });
}

#[test]
fn object_spell_roundtrip() {
    roundtrip(s::ObjectSpell {
        object_id: 55,
        location: Point::new(10, 20),
        spell: 7, // Spell
        direction: MirDirection::DownLeft,
        param: true,
    });
}

#[test]
fn dash_packets_roundtrip() {
    let loc = Point::new(15, 25);
    roundtrip(s::UserDash {
        location: loc,
        direction: MirDirection::Right,
    });
    roundtrip(s::ObjectDash {
        object_id: 77,
        location: loc,
        direction: MirDirection::Down,
    });
    roundtrip(s::UserDashFail {
        location: loc,
        direction: MirDirection::Left,
    });
    roundtrip(s::ObjectDashFail {
        object_id: 78,
        location: loc,
        direction: MirDirection::Up,
    });
}

#[test]
fn remove_delayed_explosion_roundtrip() {
    roundtrip(s::RemoveDelayedExplosion { object_id: 321 });
}

#[test]
fn npc_consign_roundtrip() {
    roundtrip(s::NPCConsign);
}

#[test]
fn npc_market_roundtrip() {
    roundtrip(s::NPCMarket {
        listings: vec![sample_auction(), sample_auction()],
        pages: 3,
        user_mode: true,
    });
}

#[test]
fn npc_market_page_roundtrip() {
    roundtrip(s::NPCMarketPage {
        listings: vec![sample_auction()],
    });
}

#[test]
fn guild_territory_page_roundtrip() {
    let map = ClientGtMap {
        index: 1,
        name: "沙巴克".into(),
        owner: "行会A".into(),
        leader: "甲".into(),
        leader2: "乙".into(),
        price: 500_000,
        days: 7,
        begin: 1_700_000_000,
    };
    roundtrip(s::GuildTerritoryPage {
        length: 2,
        listings: vec![map.clone(), map],
    });
}

#[test]
fn consign_item_roundtrip() {
    roundtrip(s::ConsignItem {
        unique_id: 999_999,
        success: true,
    });
}

#[test]
fn market_fail_roundtrip() {
    roundtrip(s::MarketFail { reason: 4 });
}

#[test]
fn market_success_roundtrip() {
    roundtrip(s::MarketSuccess {
        message: "出售成功，金币已入账".into(),
    });
}

#[test]
fn object_sit_down_roundtrip() {
    roundtrip(s::ObjectSitDown {
        object_id: 202,
        location: Point::new(1, 1),
        direction: MirDirection::DownRight,
        sitting: true,
    });
}

#[test]
fn in_trap_rock_roundtrip() {
    roundtrip(s::InTrapRock { trapped: true });
}

#[test]
fn base_stats_roundtrip() {
    let stats = BaseStats {
        job: MirClass::Wizard,
        stats: vec![
            BaseStat {
                r#type: 1, // Stat.HP
                formula_type: 0,
                base: 14,
                gain: 15.0,
                gain_rate: 1.8,
                max: 0,
            },
            BaseStat {
                r#type: 2, // Stat.MP
                formula_type: 1,
                base: 13,
                gain: 5.0,
                gain_rate: 0.0,
                max: 0,
            },
        ],
        caps: Stats {
            values: vec![(20, 6), (21, 6)],
        },
    };
    roundtrip(s::BaseStatsInfo {
        stats: stats.clone(),
    });
    roundtrip(s::HeroBaseStatsInfo { stats });
}

#[test]
fn user_name_roundtrip() {
    roundtrip(s::UserName {
        id: 404,
        name: "玩家名".into(),
    });
}

#[test]
fn chat_item_stats_roundtrip() {
    roundtrip(s::ChatItemStats {
        chat_item_id: 88,
        stats: sample_item(),
    });
}

#[test]
fn guild_notice_change_roundtrip() {
    // 正常路径: update >= 0（写侧写 notice.Count，读侧 update = count）
    let notice = vec!["第一条".into(), "第二条".into()];
    let count = notice.len() as i32;
    roundtrip(s::GuildNoticeChange {
        update: count,
        notice,
    });
}

#[test]
fn guild_member_change_roundtrip() {
    let rank = GuildRank {
        name: "长老".into(),
        options: 2, // GuildRankOptions
        index: 1,
        members: vec![GuildMember {
            name: "小甲".into(),
            id: 5,
            last_login: 1_700_100_000,
            has_voted: true,
            online: true,
        }],
    };
    // status > 5 触发 Ranks 附带
    roundtrip(s::GuildMemberChange {
        name: "改名".into(),
        rank_index: 1,
        status: 6,
        ranks: vec![rank.clone()],
    });
    // status <= 5 不附带 Ranks（rank 仅用于高状态分支）
    roundtrip(s::GuildMemberChange {
        name: "低状态".into(),
        rank_index: 2,
        status: 3,
        ranks: vec![],
    });
}

#[test]
fn guild_status_roundtrip() {
    roundtrip(s::GuildStatus {
        guild_name: "王者行会".into(),
        guild_rank_name: "会长".into(),
        level: 5,
        experience: 123_456,
        max_experience: 1_000_000,
        gold: 50_000,
        spare_points: 3,
        member_count: 20,
        max_members: 30,
        voting: true,
        item_count: 12,
        buff_count: 4,
        my_options: 7, // GuildRankOptions 位标志
        my_rank_id: 0,
    });
}

#[test]
fn guild_invite_roundtrip() {
    roundtrip(s::GuildInvite {
        name: "邀请对象".into(),
    });
}

#[test]
fn guild_exp_gain_roundtrip() {
    roundtrip(s::GuildExpGain { amount: 2_500 });
}

#[test]
fn guild_name_request_roundtrip() {
    roundtrip(s::GuildNameRequest);
}

#[test]
fn guild_storage_gold_change_roundtrip() {
    roundtrip(s::GuildStorageGoldChange {
        amount: 9_999,
        r#type: 1,
        name: "存金币".into(),
    });
}

#[test]
fn guild_storage_item_change_roundtrip() {
    roundtrip(s::GuildStorageItemChange {
        r#type: 2,
        to: 3,
        from: 1,
        user: 7,
        item: Some(GuildStorageItemChangeItem {
            user_id: 42,
            item: sample_item(),
        }),
    });
    // 无 item 分支
    roundtrip(s::GuildStorageItemChange {
        r#type: 2,
        to: 3,
        from: 1,
        user: 7,
        item: None,
    });
}

#[test]
fn guild_storage_list_roundtrip() {
    roundtrip(s::GuildStorageList {
        items: vec![
            Some(GuildStorageItem {
                item: sample_item(),
                user_id: 1,
            }),
            None,
            Some(GuildStorageItem {
                item: sample_item(),
                user_id: 2,
            }),
        ],
    });
    // 全空
    roundtrip(s::GuildStorageList { items: vec![] });
}

#[test]
fn guild_request_war_roundtrip() {
    roundtrip(s::GuildRequestWar);
}

#[test]
fn hero_create_request_roundtrip() {
    roundtrip(s::HeroCreateRequest {
        can_create_class: vec![true, true, true, false, false],
    });
}

#[test]
fn new_hero_roundtrip() {
    roundtrip(s::NewHero { result: 5 });
}

#[test]
fn update_hero_spawn_state_roundtrip() {
    roundtrip(s::UpdateHeroSpawnState { state: 2 }); // HeroSpawnState
}
