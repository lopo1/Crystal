//! 服务器→客户端包 batch_4（对应 `Shared/ServerPackets.cs` 4001–4900 行）。
//!
//! 覆盖: PauseBuff … UpdateHeroSpawnState（SB4.txt 清单）。
//! 字段顺序与 C# read/write 完全一致。未移植的枚举一律存原始整数并注明 C# 枚举名；
//! 已移植枚举（MirDirection/MirClass）沿用 types.rs。

use crate::binary::{Point, Reader, Writer};
use crate::frame::PacketCodec;
use crate::ids::ServerPacketId;
use crate::types::*;
use crate::Result;

// ---------------------------------------------------------------------------
// 本批内嵌支持类型（对应 Shared/BaseStats.cs、Shared/Data/GuildData.cs、
// Shared/Data/SharedData.cs 的 ClientGTMap；尚未进 types.rs，先在本批内定义）
// ---------------------------------------------------------------------------

/// 对应 C# `BaseStats.cs` 的 `BaseStat`。
/// 线上顺序: Type(Stat), FormulaType(StatFormula), Base, Gain, GainRate, Max。
#[derive(Debug, Clone, Default)]
pub struct BaseStat {
    /// Stat (u8)
    pub r#type: u8,
    /// StatFormula (u8)
    pub formula_type: u8,
    pub base: i32,
    pub gain: f32,
    pub gain_rate: f32,
    pub max: i32,
}

// f32 不能 derive Eq；按位精确比较（回环要求字节完全一致）
impl PartialEq for BaseStat {
    fn eq(&self, other: &Self) -> bool {
        self.r#type == other.r#type
            && self.formula_type == other.formula_type
            && self.base == other.base
            && self.gain.to_bits() == other.gain.to_bits()
            && self.gain_rate.to_bits() == other.gain_rate.to_bits()
            && self.max == other.max
    }
}
impl Eq for BaseStat {}

impl BaseStat {
    pub fn read(r: &mut Reader) -> Result<Self> {
        Ok(BaseStat {
            r#type: r.read_u8()?,
            formula_type: r.read_u8()?,
            base: r.read_i32()?,
            gain: r.read_f32()?,
            gain_rate: r.read_f32()?,
            max: r.read_i32()?,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_u8(self.r#type);
        w.write_u8(self.formula_type);
        w.write_i32(self.base);
        w.write_f32(self.gain);
        w.write_f32(self.gain_rate);
        w.write_i32(self.max);
    }
}

/// 对应 C# `BaseStats.cs` 的 `BaseStats`。
/// 线上顺序: Job(MirClass), 条目数, [BaseStat...], Caps(Stats)。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BaseStats {
    /// MirClass (u8)
    pub job: MirClass,
    pub stats: Vec<BaseStat>,
    pub caps: Stats,
}

impl BaseStats {
    pub fn read(r: &mut Reader) -> Result<Self> {
        let job = match r.read_u8()? {
            0 => MirClass::Warrior,
            1 => MirClass::Wizard,
            2 => MirClass::Taoist,
            3 => MirClass::Assassin,
            _ => MirClass::Archer,
        };
        let count = r.read_i32()?;
        let mut stats = Vec::with_capacity(count.max(0) as usize);
        for _ in 0..count.max(0) {
            stats.push(BaseStat::read(r)?);
        }
        let caps = Stats::read(r)?;
        Ok(BaseStats { job, stats, caps })
    }

    pub fn write(&self, w: &mut Writer) {
        w.write_u8(self.job as u8);
        w.write_i32(self.stats.len() as i32);
        for s in &self.stats {
            s.write(w);
        }
        self.caps.write(w);
    }
}

/// 对应 C# `GuildData.cs` 的 `GuildMember`。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuildMember {
    pub name: String,
    pub id: i32,
    /// DateTime.ToBinary() (i64)
    pub last_login: i64,
    pub has_voted: bool,
    pub online: bool,
}

impl GuildMember {
    pub fn read(r: &mut Reader) -> Result<Self> {
        Ok(GuildMember {
            name: r.read_string()?,
            id: r.read_i32()?,
            last_login: r.read_i64()?,
            has_voted: r.read_bool()?,
            online: r.read_bool()?,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_string(&self.name);
        w.write_i32(self.id);
        w.write_i64(self.last_login);
        w.write_bool(self.has_voted);
        w.write_bool(self.online);
    }
}

/// 对应 C# `GuildData.cs` 的 `GuildRank`（非 offline 线格式）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuildRank {
    pub name: String,
    /// GuildRankOptions (u8)
    pub options: u8,
    pub index: i32,
    pub members: Vec<GuildMember>,
}

impl GuildRank {
    pub fn read(r: &mut Reader) -> Result<Self> {
        let name = r.read_string()?;
        let options = r.read_u8()?;
        let index = r.read_i32()?;
        let count = r.read_i32()?;
        let mut members = Vec::with_capacity(count.max(0) as usize);
        for _ in 0..count.max(0) {
            members.push(GuildMember::read(r)?);
        }
        Ok(GuildRank {
            name,
            options,
            index,
            members,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_string(&self.name);
        w.write_u8(self.options);
        w.write_i32(self.index);
        w.write_i32(self.members.len() as i32);
        for m in &self.members {
            m.write(w);
        }
    }
}

/// 对应 C# `GuildData.cs` 的 `GuildStorageItem`（Save 顺序: Item 在前, UserId 在后）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuildStorageItem {
    pub item: UserItem,
    pub user_id: i64,
}

impl GuildStorageItem {
    pub fn read(r: &mut Reader) -> Result<Self> {
        Ok(GuildStorageItem {
            item: UserItem::read(r)?,
            user_id: r.read_i64()?,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        self.item.write(w);
        w.write_i64(self.user_id);
    }
}

/// `GuildStorageItemChange` 内联条目（注意: 该包线序是 UserId 在前, Item 在后，
/// 与 `GuildStorageItem` 的 Save/构造顺序相反，以 C# 原码为准）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuildStorageItemChangeItem {
    pub user_id: i64,
    pub item: UserItem,
}

impl GuildStorageItemChangeItem {
    pub fn read(r: &mut Reader) -> Result<Self> {
        Ok(GuildStorageItemChangeItem {
            user_id: r.read_i64()?,
            item: UserItem::read(r)?,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_i64(self.user_id);
        self.item.write(w);
    }
}

/// 对应 C# `SharedData.cs` 的 `ClientGTMap`（用于 GuildTerritoryPage）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientGtMap {
    pub index: i32,
    pub name: String,
    pub owner: String,
    pub leader: String,
    pub leader2: String,
    pub price: i32,
    pub days: i32,
    pub begin: i32,
}

impl ClientGtMap {
    pub fn read(r: &mut Reader) -> Result<Self> {
        Ok(ClientGtMap {
            index: r.read_i32()?,
            name: r.read_string()?,
            owner: r.read_string()?,
            leader: r.read_string()?,
            leader2: r.read_string()?,
            price: r.read_i32()?,
            days: r.read_i32()?,
            begin: r.read_i32()?,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_i32(self.index);
        w.write_string(&self.name);
        w.write_string(&self.owner);
        w.write_string(&self.leader);
        w.write_string(&self.leader2);
        w.write_i32(self.price);
        w.write_i32(self.days);
        w.write_i32(self.begin);
    }
}

// ----------------------------- ID 146: PauseBuff -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PauseBuff {
    /// BuffType (u8)
    pub r#type: u8,
    pub object_id: u32,
    pub paused: bool,
}

impl PacketCodec for PauseBuff {
    const ID: i16 = ServerPacketId::PauseBuff as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(PauseBuff {
            r#type: r.read_u8()?,
            object_id: r.read_u32()?,
            paused: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.r#type);
        w.write_u32(self.object_id);
        w.write_bool(self.paused);
    }
}

// ----------------------------- ID 147: ObjectHidden -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectHidden {
    pub object_id: u32,
    pub hidden: bool,
}

impl PacketCodec for ObjectHidden {
    const ID: i16 = ServerPacketId::ObjectHidden as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectHidden {
            object_id: r.read_u32()?,
            hidden: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_bool(self.hidden);
    }
}

// ----------------------------- ID 148: RefreshItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RefreshItem {
    pub item: UserItem,
}

impl PacketCodec for RefreshItem {
    const ID: i16 = ServerPacketId::RefreshItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RefreshItem {
            item: UserItem::read(r)?,
        })
    }

    fn write(&self, w: &mut Writer) {
        self.item.write(w);
    }
}

// ----------------------------- ID 149: ObjectSpell -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectSpell {
    pub object_id: u32,
    pub location: Point,
    /// Spell (u8)
    pub spell: u8,
    pub direction: MirDirection,
    pub param: bool,
}

impl PacketCodec for ObjectSpell {
    const ID: i16 = ServerPacketId::ObjectSpell as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectSpell {
            object_id: r.read_u32()?,
            location: Point::read(r)?,
            spell: r.read_u8()?,
            direction: MirDirection::from_u8(r.read_u8()?),
            param: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        self.location.write(w);
        w.write_u8(self.spell);
        w.write_u8(self.direction.to_u8());
        w.write_bool(self.param);
    }
}

// ----------------------------- ID 150-153: Dash 系列 -----------------------------
// 模式: UserDash / ObjectDash / UserDashFail / ObjectDashFail
// （UserDash/UserDashFail: Location+Direction; ObjectDash/ObjectDashFail: 前加 ObjectID）

/// 无 ObjectID 的冲刺包（UserDash / UserDashFail）
macro_rules! user_dash_packet {
    ($name:ident => $id:expr) => {
        #[derive(Debug, Clone, Default, PartialEq, Eq)]
        pub struct $name {
            pub location: Point,
            pub direction: MirDirection,
        }

        impl PacketCodec for $name {
            const ID: i16 = $id;

            fn read(r: &mut Reader) -> Result<Self> {
                Ok($name {
                    location: Point::read(r)?,
                    direction: MirDirection::from_u8(r.read_u8()?),
                })
            }

            fn write(&self, w: &mut Writer) {
                self.location.write(w);
                w.write_u8(self.direction.to_u8());
            }
        }
    };
}

/// 带 ObjectID 的冲刺包（ObjectDash / ObjectDashFail）
macro_rules! object_dash_packet {
    ($name:ident => $id:expr) => {
        #[derive(Debug, Clone, Default, PartialEq, Eq)]
        pub struct $name {
            pub object_id: u32,
            pub location: Point,
            pub direction: MirDirection,
        }

        impl PacketCodec for $name {
            const ID: i16 = $id;

            fn read(r: &mut Reader) -> Result<Self> {
                Ok($name {
                    object_id: r.read_u32()?,
                    location: Point::read(r)?,
                    direction: MirDirection::from_u8(r.read_u8()?),
                })
            }

            fn write(&self, w: &mut Writer) {
                w.write_u32(self.object_id);
                self.location.write(w);
                w.write_u8(self.direction.to_u8());
            }
        }
    };
}

user_dash_packet!(UserDash => ServerPacketId::UserDash as i16);
object_dash_packet!(ObjectDash => ServerPacketId::ObjectDash as i16);
user_dash_packet!(UserDashFail => ServerPacketId::UserDashFail as i16);
object_dash_packet!(ObjectDashFail => ServerPacketId::ObjectDashFail as i16);

// ----------------------------- ID 218: RemoveDelayedExplosion -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RemoveDelayedExplosion {
    pub object_id: u32,
}

impl PacketCodec for RemoveDelayedExplosion {
    const ID: i16 = ServerPacketId::RemoveDelayedExplosion as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RemoveDelayedExplosion {
            object_id: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
    }
}

// ----------------------------- ID 154: NPCConsign（空包） -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NPCConsign;

impl PacketCodec for NPCConsign {
    const ID: i16 = ServerPacketId::NPCConsign as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(NPCConsign)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 155: NPCMarket -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NPCMarket {
    pub listings: Vec<ClientAuction>,
    pub pages: i32,
    pub user_mode: bool,
}

impl PacketCodec for NPCMarket {
    const ID: i16 = ServerPacketId::NPCMarket as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let count = r.read_i32()?;
        let mut listings = Vec::with_capacity(count.max(0) as usize);
        for _ in 0..count.max(0) {
            listings.push(ClientAuction::read(r)?);
        }
        Ok(NPCMarket {
            listings,
            pages: r.read_i32()?,
            user_mode: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.listings.len() as i32);
        for l in &self.listings {
            l.write(w);
        }
        w.write_i32(self.pages);
        w.write_bool(self.user_mode);
    }
}

// ----------------------------- ID 156: NPCMarketPage -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NPCMarketPage {
    pub listings: Vec<ClientAuction>,
}

impl PacketCodec for NPCMarketPage {
    const ID: i16 = ServerPacketId::NPCMarketPage as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let count = r.read_i32()?;
        let mut listings = Vec::with_capacity(count.max(0) as usize);
        for _ in 0..count.max(0) {
            listings.push(ClientAuction::read(r)?);
        }
        Ok(NPCMarketPage { listings })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.listings.len() as i32);
        for l in &self.listings {
            l.write(w);
        }
    }
}

// ----------------------------- ID 276: GuildTerritoryPage -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuildTerritoryPage {
    pub length: i32,
    pub listings: Vec<ClientGtMap>,
}

impl PacketCodec for GuildTerritoryPage {
    const ID: i16 = ServerPacketId::GuildTerritoryPage as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let length = r.read_i32()?;
        let count = r.read_i32()?;
        let mut listings = Vec::with_capacity(count.max(0) as usize);
        for _ in 0..count.max(0) {
            listings.push(ClientGtMap::read(r)?);
        }
        Ok(GuildTerritoryPage { length, listings })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.length);
        w.write_i32(self.listings.len() as i32);
        for l in &self.listings {
            l.write(w);
        }
    }
}

// ----------------------------- ID 157: ConsignItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConsignItem {
    pub unique_id: u64,
    pub success: bool,
}

impl PacketCodec for ConsignItem {
    const ID: i16 = ServerPacketId::ConsignItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ConsignItem {
            unique_id: r.read_u64()?,
            success: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
        w.write_bool(self.success);
    }
}

// ----------------------------- ID 158: MarketFail -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarketFail {
    /// 0 已死; 1 未对话; 2 已售; 3 过期; 4 金币不足; 5 背包不足;
    /// 6 不能买自己的物品; 7 太远; 8 金币过多
    pub reason: u8,
}

impl PacketCodec for MarketFail {
    const ID: i16 = ServerPacketId::MarketFail as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(MarketFail {
            reason: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.reason);
    }
}

// ----------------------------- ID 159: MarketSuccess -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarketSuccess {
    pub message: String,
}

impl PacketCodec for MarketSuccess {
    const ID: i16 = ServerPacketId::MarketSuccess as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(MarketSuccess {
            message: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.message);
    }
}

// ----------------------------- ID 160: ObjectSitDown -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectSitDown {
    pub object_id: u32,
    pub location: Point,
    pub direction: MirDirection,
    pub sitting: bool,
}

impl PacketCodec for ObjectSitDown {
    const ID: i16 = ServerPacketId::ObjectSitDown as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectSitDown {
            object_id: r.read_u32()?,
            location: Point::read(r)?,
            direction: MirDirection::from_u8(r.read_u8()?),
            sitting: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        self.location.write(w);
        w.write_u8(self.direction.to_u8());
        w.write_bool(self.sitting);
    }
}

// ----------------------------- ID 161: InTrapRock -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InTrapRock {
    pub trapped: bool,
}

impl PacketCodec for InTrapRock {
    const ID: i16 = ServerPacketId::InTrapRock as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(InTrapRock {
            trapped: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_bool(self.trapped);
    }
}

// ----------------------------- ID 162: BaseStatsInfo -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BaseStatsInfo {
    pub stats: BaseStats,
}

impl PacketCodec for BaseStatsInfo {
    const ID: i16 = ServerPacketId::BaseStatsInfo as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(BaseStatsInfo {
            stats: BaseStats::read(r)?,
        })
    }

    fn write(&self, w: &mut Writer) {
        self.stats.write(w);
    }
}

// ----------------------------- ID 163: HeroBaseStatsInfo -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeroBaseStatsInfo {
    pub stats: BaseStats,
}

impl PacketCodec for HeroBaseStatsInfo {
    const ID: i16 = ServerPacketId::HeroBaseStatsInfo as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(HeroBaseStatsInfo {
            stats: BaseStats::read(r)?,
        })
    }

    fn write(&self, w: &mut Writer) {
        self.stats.write(w);
    }
}

// ----------------------------- ID 164: UserName -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserName {
    pub id: u32,
    pub name: String,
}

impl PacketCodec for UserName {
    const ID: i16 = ServerPacketId::UserName as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(UserName {
            id: r.read_u32()?,
            name: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.id);
        w.write_string(&self.name);
    }
}

// ----------------------------- ID 165: ChatItemStats -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChatItemStats {
    pub chat_item_id: u64,
    pub stats: UserItem,
}

impl PacketCodec for ChatItemStats {
    const ID: i16 = ServerPacketId::ChatItemStats as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ChatItemStats {
            chat_item_id: r.read_u64()?,
            stats: UserItem::read(r)?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.chat_item_id);
        self.stats.write(w);
    }
}

// ----------------------------- ID 166: GuildNoticeChange -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuildNoticeChange {
    /// C# 原码: update<0 时只写该负数；否则写 notice.Count（读侧得到的 update=count）
    pub update: i32,
    pub notice: Vec<String>,
}

impl PacketCodec for GuildNoticeChange {
    const ID: i16 = ServerPacketId::GuildNoticeChange as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let update = r.read_i32()?;
        let mut notice = Vec::with_capacity(update.max(0) as usize);
        for _ in 0..update.max(0) {
            notice.push(r.read_string()?);
        }
        Ok(GuildNoticeChange { update, notice })
    }

    fn write(&self, w: &mut Writer) {
        if self.update < 0 {
            w.write_i32(self.update);
            return;
        }
        w.write_i32(self.notice.len() as i32);
        for s in &self.notice {
            w.write_string(s);
        }
    }
}

// ----------------------------- ID 167: GuildMemberChange -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuildMemberChange {
    pub name: String,
    pub rank_index: u8,
    /// Status > 5 时附带 Ranks
    pub status: u8,
    pub ranks: Vec<GuildRank>,
}

impl PacketCodec for GuildMemberChange {
    const ID: i16 = ServerPacketId::GuildMemberChange as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let name = r.read_string()?;
        let rank_index = r.read_u8()?;
        let status = r.read_u8()?;
        let mut ranks = Vec::new();
        if status > 5 {
            let count = r.read_i32()?;
            for _ in 0..count.max(0) {
                ranks.push(GuildRank::read(r)?);
            }
        }
        Ok(GuildMemberChange {
            name,
            rank_index,
            status,
            ranks,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.name);
        w.write_u8(self.rank_index);
        w.write_u8(self.status);
        if self.status > 5 {
            w.write_i32(self.ranks.len() as i32);
            for rk in &self.ranks {
                rk.write(w);
            }
        }
    }
}

// ----------------------------- ID 168: GuildStatus -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuildStatus {
    pub guild_name: String,
    pub guild_rank_name: String,
    pub level: u8,
    pub experience: i64,
    pub max_experience: i64,
    pub gold: u32,
    pub spare_points: u8,
    pub member_count: i32,
    pub max_members: i32,
    pub voting: bool,
    pub item_count: u8,
    pub buff_count: u8,
    /// GuildRankOptions (u8)
    pub my_options: u8,
    pub my_rank_id: i32,
}

impl PacketCodec for GuildStatus {
    const ID: i16 = ServerPacketId::GuildStatus as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(GuildStatus {
            guild_name: r.read_string()?,
            guild_rank_name: r.read_string()?,
            level: r.read_u8()?,
            experience: r.read_i64()?,
            max_experience: r.read_i64()?,
            gold: r.read_u32()?,
            spare_points: r.read_u8()?,
            member_count: r.read_i32()?,
            max_members: r.read_i32()?,
            voting: r.read_bool()?,
            item_count: r.read_u8()?,
            buff_count: r.read_u8()?,
            my_options: r.read_u8()?,
            my_rank_id: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.guild_name);
        w.write_string(&self.guild_rank_name);
        w.write_u8(self.level);
        w.write_i64(self.experience);
        w.write_i64(self.max_experience);
        w.write_u32(self.gold);
        w.write_u8(self.spare_points);
        w.write_i32(self.member_count);
        w.write_i32(self.max_members);
        w.write_bool(self.voting);
        w.write_u8(self.item_count);
        w.write_u8(self.buff_count);
        w.write_u8(self.my_options);
        w.write_i32(self.my_rank_id);
    }
}

// ----------------------------- ID 169: GuildInvite -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuildInvite {
    pub name: String,
}

impl PacketCodec for GuildInvite {
    const ID: i16 = ServerPacketId::GuildInvite as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(GuildInvite {
            name: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.name);
    }
}

// ----------------------------- ID 170: GuildExpGain -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuildExpGain {
    pub amount: u32,
}

impl PacketCodec for GuildExpGain {
    const ID: i16 = ServerPacketId::GuildExpGain as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(GuildExpGain {
            amount: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.amount);
    }
}

// ----------------------------- ID 171: GuildNameRequest（空包） -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuildNameRequest;

impl PacketCodec for GuildNameRequest {
    const ID: i16 = ServerPacketId::GuildNameRequest as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(GuildNameRequest)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 172: GuildStorageGoldChange -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuildStorageGoldChange {
    pub amount: u32,
    pub r#type: u8,
    pub name: String,
}

impl PacketCodec for GuildStorageGoldChange {
    const ID: i16 = ServerPacketId::GuildStorageGoldChange as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(GuildStorageGoldChange {
            amount: r.read_u32()?,
            r#type: r.read_u8()?,
            name: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.amount);
        w.write_u8(self.r#type);
        w.write_string(&self.name);
    }
}

// ----------------------------- ID 173: GuildStorageItemChange -----------------------------
// 注意: 该包内联条目线序为 UserId(i64) 在前、UserItem 在后（与 GuildStorageItem.Save 相反）。

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuildStorageItemChange {
    pub r#type: u8,
    pub to: i32,
    pub from: i32,
    pub user: i32,
    pub item: Option<GuildStorageItemChangeItem>,
}

impl PacketCodec for GuildStorageItemChange {
    const ID: i16 = ServerPacketId::GuildStorageItemChange as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let r#type = r.read_u8()?;
        let to = r.read_i32()?;
        let from = r.read_i32()?;
        let user = r.read_i32()?;
        let item = if r.read_bool()? {
            Some(GuildStorageItemChangeItem::read(r)?)
        } else {
            None
        };
        Ok(GuildStorageItemChange {
            r#type,
            to,
            from,
            user,
            item,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.r#type);
        w.write_i32(self.to);
        w.write_i32(self.from);
        w.write_i32(self.user);
        match &self.item {
            Some(item) => {
                w.write_bool(true);
                item.write(w);
            }
            None => w.write_bool(false),
        }
    }
}

// ----------------------------- ID 174: GuildStorageList -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuildStorageList {
    pub items: Vec<Option<GuildStorageItem>>,
}

impl PacketCodec for GuildStorageList {
    const ID: i16 = ServerPacketId::GuildStorageList as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let count = r.read_i32()?;
        let mut items = Vec::with_capacity(count.max(0) as usize);
        for _ in 0..count.max(0) {
            // C#: `if (reader.ReadBoolean() == true) Items[i] = new GuildStorageItem(reader);`
            if r.read_bool()? {
                items.push(Some(GuildStorageItem::read(r)?));
            } else {
                items.push(None);
            }
        }
        Ok(GuildStorageList { items })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.items.len() as i32);
        for item in &self.items {
            match item {
                Some(item) => {
                    w.write_bool(true);
                    item.write(w);
                }
                None => w.write_bool(false),
            }
        }
    }
}

// ----------------------------- ID 175: GuildRequestWar（空包） -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuildRequestWar;

impl PacketCodec for GuildRequestWar {
    const ID: i16 = ServerPacketId::GuildRequestWar as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(GuildRequestWar)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 176: HeroCreateRequest -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeroCreateRequest {
    pub can_create_class: Vec<bool>,
}

impl PacketCodec for HeroCreateRequest {
    const ID: i16 = ServerPacketId::HeroCreateRequest as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let count = r.read_i32()?;
        let mut can_create_class = Vec::with_capacity(count.max(0) as usize);
        for _ in 0..count.max(0) {
            can_create_class.push(r.read_bool()?);
        }
        Ok(HeroCreateRequest { can_create_class })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.can_create_class.len() as i32);
        for b in &self.can_create_class {
            w.write_bool(*b);
        }
    }
}

// ----------------------------- ID 177: NewHero -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewHero {
    /// 0 禁用; 1 名字非法; 2 性别非法; 3 职业非法; 4 英雄满; 5 名字已存在
    pub result: u8,
}

impl PacketCodec for NewHero {
    const ID: i16 = ServerPacketId::NewHero as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(NewHero {
            result: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.result);
    }
}

// ----------------------------- ID 179: UpdateHeroSpawnState -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UpdateHeroSpawnState {
    /// HeroSpawnState (u8)
    pub state: u8,
}

impl PacketCodec for UpdateHeroSpawnState {
    const ID: i16 = ServerPacketId::UpdateHeroSpawnState as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(UpdateHeroSpawnState {
            state: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.state);
    }
}
