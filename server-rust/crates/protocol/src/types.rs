//! 共享数据类型（对应 `Shared/Data/*.cs` 中随数据包传输的结构）。
//!
//! 所有读写必须与 C# 侧字段顺序、类型完全一致（见 `docs/PROTOCOL.md`）。

use crate::binary::{
    datetime_from_binary, datetime_to_binary, DateTimeKind, Point, Reader, Writer,
};
use crate::Result;

// ---------------------------------------------------------------------------
// 基础枚举（C# `Enums.cs`）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum MirGender {
    #[default]
    Male = 0,
    Female = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum MirClass {
    #[default]
    Warrior = 0,
    Wizard = 1,
    Taoist = 2,
    Assassin = 3,
    Archer = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum MirDirection {
    #[default]
    Up = 0,
    UpRight = 1,
    Right = 2,
    DownRight = 3,
    Down = 4,
    DownLeft = 5,
    Left = 6,
    UpLeft = 7,
}

impl MirDirection {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => MirDirection::Up,
            1 => MirDirection::UpRight,
            2 => MirDirection::Right,
            3 => MirDirection::DownRight,
            4 => MirDirection::Down,
            5 => MirDirection::DownLeft,
            6 => MirDirection::Left,
            _ => MirDirection::UpLeft,
        }
    }
    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum MirGridType {
    #[default]
    None = 0,
    Inventory = 1,
    Equipment = 2,
    Trade = 3,
    Storage = 4,
    BuyBack = 5,
    DropPanel = 6,
    Inspect = 7,
    TrustMerchant = 8,
    GuildStorage = 9,
    GuestTrade = 10,
    Mount = 11,
    Fishing = 12,
    QuestInventory = 13,
    AwakenItem = 14,
    Mail = 15,
    Refine = 16,
    Renting = 17,
    GuestRenting = 18,
    Craft = 19,
    Socket = 20,
    HeroEquipment = 21,
    HeroInventory = 22,
    HeroHpItem = 23,
    HeroMpItem = 24,
}

impl MirGridType {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => MirGridType::Inventory,
            2 => MirGridType::Equipment,
            3 => MirGridType::Trade,
            4 => MirGridType::Storage,
            5 => MirGridType::BuyBack,
            6 => MirGridType::DropPanel,
            7 => MirGridType::Inspect,
            8 => MirGridType::TrustMerchant,
            9 => MirGridType::GuildStorage,
            10 => MirGridType::GuestTrade,
            11 => MirGridType::Mount,
            12 => MirGridType::Fishing,
            13 => MirGridType::QuestInventory,
            14 => MirGridType::AwakenItem,
            15 => MirGridType::Mail,
            16 => MirGridType::Refine,
            17 => MirGridType::Renting,
            18 => MirGridType::GuestRenting,
            19 => MirGridType::Craft,
            20 => MirGridType::Socket,
            21 => MirGridType::HeroEquipment,
            22 => MirGridType::HeroInventory,
            23 => MirGridType::HeroHpItem,
            24 => MirGridType::HeroMpItem,
            _ => MirGridType::None,
        }
    }
    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

/// 位标志: 级别光效（u16）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LevelEffects(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum HeroBehaviour {
    #[default]
    Attack = 0,
    CounterAttack = 1,
    Follow = 2,
    Custom = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum AwakeType {
    #[default]
    None = 0,
    Dc = 1,
    Mc = 2,
    Sc = 3,
    Ac = 4,
    Mac = 5,
    Hpmp = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum IntelligentCreatureType {
    // 注意: C# 枚举值为 None = 99, BabyPig = 0...
    BabyPig = 0,
    Chick = 1,
    Kitten = 2,
    BabySkeleton = 3,
    Baekdon = 4,
    Wimaen = 5,
    BlackKitten = 6,
    BabyDragon = 7,
    OlympicFlame = 8,
    BabySnowMan = 9,
    #[default]
    None = 99,
}

impl IntelligentCreatureType {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => IntelligentCreatureType::BabyPig,
            1 => IntelligentCreatureType::Chick,
            2 => IntelligentCreatureType::Kitten,
            3 => IntelligentCreatureType::BabySkeleton,
            4 => IntelligentCreatureType::Baekdon,
            5 => IntelligentCreatureType::Wimaen,
            6 => IntelligentCreatureType::BlackKitten,
            7 => IntelligentCreatureType::BabyDragon,
            8 => IntelligentCreatureType::OlympicFlame,
            9 => IntelligentCreatureType::BabySnowMan,
            _ => IntelligentCreatureType::None,
        }
    }
    pub fn to_u8(self) -> u8 {
        match self {
            IntelligentCreatureType::None => 99,
            v => v as u8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum IntelligentCreaturePickupMode {
    #[default]
    Manual = 0,
    SemiAutomatic = 1,
    Automatic = 2,
}

/// 位标志: 毒种类（u16）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PoisonType(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum SpellEffect {
    #[default]
    None = 0,
}

/// 位标志: 绑定模式（i16）
pub type BindMode = i16;

// ---------------------------------------------------------------------------
// Stats（对应 `Shared/Data/Stat.cs`）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stats {
    /// (Stat 枚举值, 数值) —— 读写时按键序排列（C# SortedDictionary<Stat,int>）
    pub values: Vec<(u8, i32)>,
}

impl Stats {
    pub fn read(r: &mut Reader) -> Result<Self> {
        let count = r.read_i32()?;
        let mut values = Vec::with_capacity(count.max(0) as usize);
        for _ in 0..count.max(0) {
            let stat = r.read_u8()?;
            let v = r.read_i32()?;
            values.push((stat, v));
        }
        Ok(Stats { values })
    }

    pub fn write(&self, w: &mut Writer) {
        w.write_i32(self.values.len() as i32);
        for (stat, v) in &self.values {
            w.write_u8(*stat);
            w.write_i32(*v);
        }
    }
}

// ---------------------------------------------------------------------------
// Awake（对应 ItemData.cs 中 Awake 类）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Awake {
    pub r#type: AwakeType,
    pub list: Vec<u8>,
}

impl Awake {
    pub fn read(r: &mut Reader) -> Result<Self> {
        let t = r.read_u8()?;
        let count = r.read_i32()?;
        let mut list = Vec::with_capacity(count.max(0) as usize);
        for _ in 0..count.max(0) {
            list.push(r.read_u8()?);
        }
        Ok(Awake {
            r#type: match t {
                6 => AwakeType::Hpmp,
                5 => AwakeType::Mac,
                4 => AwakeType::Ac,
                3 => AwakeType::Sc,
                2 => AwakeType::Mc,
                1 => AwakeType::Dc,
                _ => AwakeType::None,
            },
            list,
        })
    }

    pub fn write(&self, w: &mut Writer) {
        w.write_u8(self.r#type as u8);
        w.write_i32(self.list.len() as i32);
        for v in &self.list {
            w.write_u8(*v);
        }
    }
}

// ---------------------------------------------------------------------------
// ExpireInfo / SealedInfo / RentalInformation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExpireInfo {
    /// .NET DateTime binary (i64)
    pub expiry_date: i64,
}

impl ExpireInfo {
    pub fn read(r: &mut Reader) -> Result<Self> {
        Ok(ExpireInfo {
            expiry_date: r.read_i64()?,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_i64(self.expiry_date);
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SealedInfo {
    pub expiry_date: i64,
    pub next_seal_date: i64,
}

impl SealedInfo {
    pub fn read(r: &mut Reader) -> Result<Self> {
        Ok(SealedInfo {
            expiry_date: r.read_i64()?,
            next_seal_date: r.read_i64()?,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_i64(self.expiry_date);
        w.write_i64(self.next_seal_date);
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RentalInformation {
    pub owner_name: String,
    pub binding_flags: BindMode,
    pub expiry_date: i64,
    pub rental_locked: bool,
}

impl RentalInformation {
    pub fn read(r: &mut Reader) -> Result<Self> {
        Ok(RentalInformation {
            owner_name: r.read_string()?,
            binding_flags: r.read_i16()?,
            expiry_date: r.read_i64()?,
            rental_locked: r.read_bool()?,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_string(&self.owner_name);
        w.write_i16(self.binding_flags);
        w.write_i64(self.expiry_date);
        w.write_bool(self.rental_locked);
    }
}

// ---------------------------------------------------------------------------
// UserItem（对应 ItemData.cs UserItem 类，最新版线格式）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserItem {
    pub unique_id: u64,
    pub item_index: i32,
    pub current_dura: u16,
    pub max_dura: u16,
    pub count: u16,
    pub soul_bound_id: i32,
    pub identified: bool,
    pub cursed: bool,
    pub slots: Vec<Option<Box<UserItem>>>,
    pub gem_count: u16,
    pub added_stats: Stats,
    pub awake: Awake,
    pub refined_value: u8,
    pub refine_added: u8,
    pub refine_success_chance: i32,
    pub wedding_ring: i32,
    pub expire_info: Option<ExpireInfo>,
    pub rental_information: Option<RentalInformation>,
    pub is_shop_item: bool,
    pub sealed_info: Option<SealedInfo>,
    pub gm_made: bool,
}

impl UserItem {
    pub fn read(r: &mut Reader) -> Result<Self> {
        let unique_id = r.read_u64()?;
        let item_index = r.read_i32()?;
        let current_dura = r.read_u16()?;
        let max_dura = r.read_u16()?;
        let count = r.read_u16()?;
        let soul_bound_id = r.read_i32()?;
        let bools = r.read_u8()?;
        let identified = bools & 0x01 == 0x01;
        let cursed = bools & 0x02 == 0x02;

        let slot_count = r.read_i32()?;
        let mut slots = Vec::with_capacity(slot_count.max(0) as usize);
        for _ in 0..slot_count.max(0) {
            // C#: `if (reader.ReadBoolean()) continue;` —— true 表示空槽
            if r.read_bool()? {
                slots.push(None);
            } else {
                slots.push(Some(Box::new(UserItem::read(r)?)));
            }
        }

        let gem_count = r.read_u16()?;
        let added_stats = Stats::read(r)?;
        let awake = Awake::read(r)?;
        let refined_value = r.read_u8()?;
        let refine_added = r.read_u8()?;
        let refine_success_chance = r.read_i32()?;
        let wedding_ring = r.read_i32()?;

        let expire_info = if r.read_bool()? {
            Some(ExpireInfo::read(r)?)
        } else {
            None
        };
        let rental_information = if r.read_bool()? {
            Some(RentalInformation::read(r)?)
        } else {
            None
        };
        let is_shop_item = r.read_bool()?;
        let sealed_info = if r.read_bool()? {
            Some(SealedInfo::read(r)?)
        } else {
            None
        };
        let gm_made = r.read_bool()?;

        Ok(UserItem {
            unique_id,
            item_index,
            current_dura,
            max_dura,
            count,
            soul_bound_id,
            identified,
            cursed,
            slots,
            gem_count,
            added_stats,
            awake,
            refined_value,
            refine_added,
            refine_success_chance,
            wedding_ring,
            expire_info,
            rental_information,
            is_shop_item,
            sealed_info,
            gm_made,
        })
    }

    pub fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
        w.write_i32(self.item_index);
        w.write_u16(self.current_dura);
        w.write_u16(self.max_dura);
        w.write_u16(self.count);
        w.write_i32(self.soul_bound_id);
        let mut bools = 0u8;
        if self.identified {
            bools |= 0x01;
        }
        if self.cursed {
            bools |= 0x02;
        }
        w.write_u8(bools);

        w.write_i32(self.slots.len() as i32);
        for slot in &self.slots {
            match slot {
                None => w.write_bool(true),
                Some(item) => {
                    w.write_bool(false);
                    item.write(w);
                }
            }
        }

        w.write_u16(self.gem_count);
        self.added_stats.write(w);
        self.awake.write(w);
        w.write_u8(self.refined_value);
        w.write_u8(self.refine_added);
        w.write_i32(self.refine_success_chance);
        w.write_i32(self.wedding_ring);

        match &self.expire_info {
            Some(info) => {
                w.write_bool(true);
                info.write(w);
            }
            None => w.write_bool(false),
        }
        match &self.rental_information {
            Some(info) => {
                w.write_bool(true);
                info.write(w);
            }
            None => w.write_bool(false),
        }
        w.write_bool(self.is_shop_item);
        match &self.sealed_info {
            Some(info) => {
                w.write_bool(true);
                info.write(w);
            }
            None => w.write_bool(false),
        }
        w.write_bool(self.gm_made);
    }
}

// ---------------------------------------------------------------------------
// ChatItem（对应 ItemData.cs ChatItem 类）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChatItem {
    pub unique_id: u64,
    pub title: String,
    pub grid: MirGridType,
}

impl ChatItem {
    pub fn read(r: &mut Reader) -> Result<Self> {
        Ok(ChatItem {
            unique_id: r.read_u64()?,
            title: r.read_string()?,
            grid: MirGridType::from_u8(r.read_u8()?),
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
        w.write_string(&self.title);
        w.write_u8(self.grid.to_u8());
    }
}

// ---------------------------------------------------------------------------
// SelectInfo（对应 SharedData.cs SelectInfo 类）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SelectInfo {
    pub index: i32,
    pub name: String,
    pub level: u16,
    pub class: MirClass,
    pub gender: MirGender,
    pub last_access: i64, // .NET DateTime binary
}

impl SelectInfo {
    pub fn read(r: &mut Reader) -> Result<Self> {
        Ok(SelectInfo {
            index: r.read_i32()?,
            name: r.read_string()?,
            level: r.read_u16()?,
            class: match r.read_u8()? {
                0 => MirClass::Warrior,
                1 => MirClass::Wizard,
                2 => MirClass::Taoist,
                3 => MirClass::Assassin,
                _ => MirClass::Archer,
            },
            gender: match r.read_u8()? {
                0 => MirGender::Male,
                _ => MirGender::Female,
            },
            last_access: r.read_i64()?,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_i32(self.index);
        w.write_string(&self.name);
        w.write_u16(self.level);
        w.write_u8(self.class as u8);
        w.write_u8(self.gender as u8);
        w.write_i64(self.last_access);
    }

    pub fn last_access_unix(&self) -> i64 {
        datetime_from_binary(self.last_access).0
    }
    pub fn set_last_access_unix(&mut self, secs: i64) {
        self.last_access = datetime_to_binary(secs, DateTimeKind::Utc);
    }
}

// ---------------------------------------------------------------------------
// ClientMagic（对应 ClientData.cs ClientMagic 类）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientMagic {
    pub name: String,
    pub spell: u8,
    pub base_cost: u8,
    pub level_cost: u8,
    pub icon: u8,
    pub level1: u8,
    pub level2: u8,
    pub level3: u8,
    pub need1: u16,
    pub need2: u16,
    pub need3: u16,
    pub level: u8,
    pub key: u8,
    pub experience: u16,
    pub delay: i64,
    pub range: u8,
    pub cast_time: i64,
}

impl ClientMagic {
    pub fn read(r: &mut Reader) -> Result<Self> {
        Ok(ClientMagic {
            name: r.read_string()?,
            spell: r.read_u8()?,
            base_cost: r.read_u8()?,
            level_cost: r.read_u8()?,
            icon: r.read_u8()?,
            level1: r.read_u8()?,
            level2: r.read_u8()?,
            level3: r.read_u8()?,
            need1: r.read_u16()?,
            need2: r.read_u16()?,
            need3: r.read_u16()?,
            level: r.read_u8()?,
            key: r.read_u8()?,
            experience: r.read_u16()?,
            delay: r.read_i64()?,
            range: r.read_u8()?,
            cast_time: r.read_i64()?,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_string(&self.name);
        w.write_u8(self.spell);
        w.write_u8(self.base_cost);
        w.write_u8(self.level_cost);
        w.write_u8(self.icon);
        w.write_u8(self.level1);
        w.write_u8(self.level2);
        w.write_u8(self.level3);
        w.write_u16(self.need1);
        w.write_u16(self.need2);
        w.write_u16(self.need3);
        w.write_u8(self.level);
        w.write_u8(self.key);
        w.write_u16(self.experience);
        w.write_i64(self.delay);
        w.write_u8(self.range);
        w.write_i64(self.cast_time);
    }
}

// ---------------------------------------------------------------------------
// IntelligentCreature 系列（对应 IntelligentCreatureData.cs）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IntelligentCreatureRules {
    pub minimal_fullness: i32,
    pub mouse_pickup_enabled: bool,
    pub mouse_pickup_range: i32,
    pub auto_pickup_enabled: bool,
    pub auto_pickup_range: i32,
    pub semi_auto_pickup_enabled: bool,
    pub semi_auto_pickup_range: i32,
    pub can_produce_black_stone: bool,
}

impl IntelligentCreatureRules {
    pub fn read(r: &mut Reader) -> Result<Self> {
        Ok(IntelligentCreatureRules {
            minimal_fullness: r.read_i32()?,
            mouse_pickup_enabled: r.read_bool()?,
            mouse_pickup_range: r.read_i32()?,
            auto_pickup_enabled: r.read_bool()?,
            auto_pickup_range: r.read_i32()?,
            semi_auto_pickup_enabled: r.read_bool()?,
            semi_auto_pickup_range: r.read_i32()?,
            can_produce_black_stone: r.read_bool()?,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_i32(self.minimal_fullness);
        w.write_bool(self.mouse_pickup_enabled);
        w.write_i32(self.mouse_pickup_range);
        w.write_bool(self.auto_pickup_enabled);
        w.write_i32(self.auto_pickup_range);
        w.write_bool(self.semi_auto_pickup_enabled);
        w.write_i32(self.semi_auto_pickup_range);
        w.write_bool(self.can_produce_black_stone);
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IntelligentCreatureItemFilter {
    pub pet_pickup_all: bool,
    pub pet_pickup_gold: bool,
    pub pet_pickup_weapons: bool,
    pub pet_pickup_armours: bool,
    pub pet_pickup_helmets: bool,
    pub pet_pickup_boots: bool,
    pub pet_pickup_belts: bool,
    pub pet_pickup_accessories: bool,
    pub pet_pickup_others: bool,
}

impl IntelligentCreatureItemFilter {
    pub fn read(r: &mut Reader) -> Result<Self> {
        Ok(IntelligentCreatureItemFilter {
            pet_pickup_all: r.read_bool()?,
            pet_pickup_gold: r.read_bool()?,
            pet_pickup_weapons: r.read_bool()?,
            pet_pickup_armours: r.read_bool()?,
            pet_pickup_helmets: r.read_bool()?,
            pet_pickup_boots: r.read_bool()?,
            pet_pickup_belts: r.read_bool()?,
            pet_pickup_accessories: r.read_bool()?,
            pet_pickup_others: r.read_bool()?,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_bool(self.pet_pickup_all);
        w.write_bool(self.pet_pickup_gold);
        w.write_bool(self.pet_pickup_weapons);
        w.write_bool(self.pet_pickup_armours);
        w.write_bool(self.pet_pickup_helmets);
        w.write_bool(self.pet_pickup_boots);
        w.write_bool(self.pet_pickup_belts);
        w.write_bool(self.pet_pickup_accessories);
        w.write_bool(self.pet_pickup_others);
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientIntelligentCreature {
    pub pet_type: IntelligentCreatureType,
    pub icon: i32,
    pub custom_name: String,
    pub fullness: i32,
    pub slot_index: i32,
    pub expire: i64,
    pub blackstone_time: i64,
    pub pet_mode: IntelligentCreaturePickupMode,
    pub creature_rules: IntelligentCreatureRules,
    pub filter: IntelligentCreatureItemFilter,
    pub pickup_grade: u8,
    pub maintain_food_time: i64,
}

impl ClientIntelligentCreature {
    pub fn read(r: &mut Reader) -> Result<Self> {
        let pet_type = IntelligentCreatureType::from_u8(r.read_u8()?);
        let icon = r.read_i32()?;
        let custom_name = r.read_string()?;
        let fullness = r.read_i32()?;
        let slot_index = r.read_i32()?;
        let expire = r.read_i64()?;
        let blackstone_time = r.read_i64()?;
        let pet_mode = match r.read_u8()? {
            1 => IntelligentCreaturePickupMode::SemiAutomatic,
            2 => IntelligentCreaturePickupMode::Automatic,
            _ => IntelligentCreaturePickupMode::Manual,
        };
        let creature_rules = IntelligentCreatureRules::read(r)?;
        let filter = IntelligentCreatureItemFilter::read(r)?;
        let pickup_grade = r.read_u8()?;
        let maintain_food_time = r.read_i64()?;
        Ok(ClientIntelligentCreature {
            pet_type,
            icon,
            custom_name,
            fullness,
            slot_index,
            expire,
            blackstone_time,
            pet_mode,
            creature_rules,
            filter,
            pickup_grade,
            maintain_food_time,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_u8(self.pet_type.to_u8());
        w.write_i32(self.icon);
        w.write_string(&self.custom_name);
        w.write_i32(self.fullness);
        w.write_i32(self.slot_index);
        w.write_i64(self.expire);
        w.write_i64(self.blackstone_time);
        w.write_u8(self.pet_mode as u8);
        self.creature_rules.write(w);
        self.filter.write(w);
        w.write_u8(self.pickup_grade);
        w.write_i64(self.maintain_food_time);
    }
}

// ---------------------------------------------------------------------------
// WorldMapSetup / WorldMapIcon（对应 SharedData.cs，供 WorldMapSetupInfo 使用）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorldMapIcon {
    pub image_index: i32,
    pub title: String,
    pub map_index: i32,
}

impl WorldMapIcon {
    pub fn read(r: &mut Reader) -> Result<Self> {
        Ok(WorldMapIcon {
            image_index: r.read_i32()?,
            title: r.read_string()?,
            map_index: r.read_i32()?,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_i32(self.image_index);
        w.write_string(&self.title);
        w.write_i32(self.map_index);
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorldMapSetup {
    pub enabled: bool,
    pub icons: Vec<WorldMapIcon>,
}

impl WorldMapSetup {
    pub fn read(r: &mut Reader) -> Result<Self> {
        let enabled = r.read_bool()?;
        let count = r.read_i32()?;
        let mut icons = Vec::with_capacity(count.max(0) as usize);
        for _ in 0..count.max(0) {
            icons.push(WorldMapIcon::read(r)?);
        }
        Ok(WorldMapSetup { enabled, icons })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_bool(self.enabled);
        w.write_i32(self.icons.len() as i32);
        for icon in &self.icons {
            icon.write(w);
        }
    }
}

// ---------------------------------------------------------------------------
// ClientMovementInfo / ClientNPCInfo / ClientMapInfo
// （对应 ClientData.cs，供 NewMapInfo 使用）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientMovementInfo {
    pub destination: i32,
    pub title: String,
    pub location: Point,
    pub icon: i32,
}

impl ClientMovementInfo {
    pub fn read(r: &mut Reader) -> Result<Self> {
        Ok(ClientMovementInfo {
            destination: r.read_i32()?,
            title: r.read_string()?,
            location: Point::read(r)?,
            icon: r.read_i32()?,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_i32(self.destination);
        w.write_string(&self.title);
        self.location.write(w);
        w.write_i32(self.icon);
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientNpcInfo {
    pub index: i32,
    pub file_name: String,
    pub name: String,
    pub map_index: i32,
    pub location: Point,
    pub image: u16,
    pub rate: u16,
    pub show_on_big_map: bool,
    pub big_map_icon: i32,
    pub object_id: u32,
    pub icon: i32,
    pub can_teleport_to: bool,
}

impl ClientNpcInfo {
    pub fn read(r: &mut Reader) -> Result<Self> {
        Ok(ClientNpcInfo {
            index: r.read_i32()?,
            file_name: r.read_string()?,
            name: r.read_string()?,
            map_index: r.read_i32()?,
            location: Point::read(r)?,
            image: r.read_u16()?,
            rate: r.read_u16()?,
            show_on_big_map: r.read_bool()?,
            big_map_icon: r.read_i32()?,
            object_id: r.read_u32()?,
            icon: r.read_i32()?,
            can_teleport_to: r.read_bool()?,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_i32(self.index);
        w.write_string(&self.file_name);
        w.write_string(&self.name);
        w.write_i32(self.map_index);
        self.location.write(w);
        w.write_u16(self.image);
        w.write_u16(self.rate);
        w.write_bool(self.show_on_big_map);
        w.write_i32(self.big_map_icon);
        w.write_u32(self.object_id);
        w.write_i32(self.icon);
        w.write_bool(self.can_teleport_to);
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientMapInfo {
    pub title: String,
    pub width: i32,
    pub height: i32,
    pub big_map: i32,
    pub movements: Vec<ClientMovementInfo>,
    pub npcs: Vec<ClientNpcInfo>,
}

impl ClientMapInfo {
    pub fn read(r: &mut Reader) -> Result<Self> {
        let title = r.read_string()?;
        let width = r.read_i32()?;
        let height = r.read_i32()?;
        let big_map = r.read_i32()?;
        let mut movements = Vec::new();
        let mcount = r.read_i32()?;
        for _ in 0..mcount.max(0) {
            movements.push(ClientMovementInfo::read(r)?);
        }
        let mut npcs = Vec::new();
        let ncount = r.read_i32()?;
        for _ in 0..ncount.max(0) {
            npcs.push(ClientNpcInfo::read(r)?);
        }
        Ok(ClientMapInfo {
            title,
            width,
            height,
            big_map,
            movements,
            npcs,
        })
    }

    pub fn write(&self, w: &mut Writer) {
        w.write_string(&self.title);
        w.write_i32(self.width);
        w.write_i32(self.height);
        w.write_i32(self.big_map);
        w.write_i32(self.movements.len() as i32);
        for m in &self.movements {
            m.write(w);
        }
        w.write_i32(self.npcs.len() as i32);
        for n in &self.npcs {
            n.write(w);
        }
    }
}

// ---------------------------------------------------------------------------
// ItemInfo（对应 ItemData.cs ItemInfo 类，最新线格式）
// 枚举字段按线上原文保留为 u8/u16/i16（避免庞大枚举移植，见 PROTOCOL.md）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemInfo {
    pub index: i32,
    pub name: String,
    /// ItemType (u8)
    pub item_type: u8,
    /// ItemGrade (u8)
    pub grade: u8,
    /// RequiredType (u8)
    pub required_type: u8,
    /// RequiredClass (u8)
    pub required_class: u8,
    /// RequiredGender (u8)
    pub required_gender: u8,
    /// ItemSet (u8)
    pub set: u8,
    pub shape: i16,
    pub weight: u8,
    pub light: u8,
    pub required_amount: u8,
    pub image: u16,
    pub durability: u16,
    pub stack_size: u16,
    pub price: u32,
    pub start_item: bool,
    pub effect: u8,
    pub need_identify: bool,
    pub show_group_pickup: bool,
    pub class_based: bool,
    pub level_based: bool,
    pub can_mine: bool,
    pub global_drop_notify: bool,
    /// BindMode (i16)
    pub bind: i16,
    /// SpecialItemMode (i16)
    pub unique: i16,
    pub random_stats_id: u8,
    pub can_fast_run: bool,
    pub can_awakening: bool,
    pub slots: u8,
    pub stats: Stats,
    pub tool_tip: Option<String>,
}

impl ItemInfo {
    pub fn read(r: &mut Reader) -> Result<Self> {
        let index = r.read_i32()?;
        let name = r.read_string()?;
        let item_type = r.read_u8()?;
        let grade = r.read_u8()?;
        let required_type = r.read_u8()?;
        let required_class = r.read_u8()?;
        let required_gender = r.read_u8()?;
        let set = r.read_u8()?;
        let shape = r.read_i16()?;
        let weight = r.read_u8()?;
        let light = r.read_u8()?;
        let required_amount = r.read_u8()?;
        let image = r.read_u16()?;
        let durability = r.read_u16()?;
        let stack_size = r.read_u16()?;
        let price = r.read_u32()?;
        let start_item = r.read_bool()?;
        let effect = r.read_u8()?;
        let bools = r.read_u8()?;
        let need_identify = bools & 0x01 == 0x01;
        let show_group_pickup = bools & 0x02 == 0x02;
        let class_based = bools & 0x04 == 0x04;
        let level_based = bools & 0x08 == 0x08;
        let can_mine = bools & 0x10 == 0x10;
        let global_drop_notify = bools & 0x20 == 0x20;
        let bind = r.read_i16()?;
        let unique = r.read_i16()?;
        let random_stats_id = r.read_u8()?;
        let can_fast_run = r.read_bool()?;
        let can_awakening = r.read_bool()?;
        let slots = r.read_u8()?;
        let stats = Stats::read(r)?;
        let tool_tip = if r.read_bool()? {
            Some(r.read_string()?)
        } else {
            None
        };
        Ok(ItemInfo {
            index,
            name,
            item_type,
            grade,
            required_type,
            required_class,
            required_gender,
            set,
            shape,
            weight,
            light,
            required_amount,
            image,
            durability,
            stack_size,
            price,
            start_item,
            effect,
            need_identify,
            show_group_pickup,
            class_based,
            level_based,
            can_mine,
            global_drop_notify,
            bind,
            unique,
            random_stats_id,
            can_fast_run,
            can_awakening,
            slots,
            stats,
            tool_tip,
        })
    }

    pub fn write(&self, w: &mut Writer) {
        w.write_i32(self.index);
        w.write_string(&self.name);
        w.write_u8(self.item_type);
        w.write_u8(self.grade);
        w.write_u8(self.required_type);
        w.write_u8(self.required_class);
        w.write_u8(self.required_gender);
        w.write_u8(self.set);
        w.write_i16(self.shape);
        w.write_u8(self.weight);
        w.write_u8(self.light);
        w.write_u8(self.required_amount);
        w.write_u16(self.image);
        w.write_u16(self.durability);
        w.write_u16(self.stack_size);
        w.write_u32(self.price);
        w.write_bool(self.start_item);
        w.write_u8(self.effect);
        let mut bools = 0u8;
        if self.need_identify {
            bools |= 0x01;
        }
        if self.show_group_pickup {
            bools |= 0x02;
        }
        if self.class_based {
            bools |= 0x04;
        }
        if self.level_based {
            bools |= 0x08;
        }
        if self.can_mine {
            bools |= 0x10;
        }
        if self.global_drop_notify {
            bools |= 0x20;
        }
        w.write_u8(bools);
        w.write_i16(self.bind);
        w.write_i16(self.unique);
        w.write_u8(self.random_stats_id);
        w.write_bool(self.can_fast_run);
        w.write_bool(self.can_awakening);
        w.write_u8(self.slots);
        self.stats.write(w);
        match &self.tool_tip {
            Some(t) => {
                w.write_bool(true);
                w.write_string(t);
            }
            None => w.write_bool(false),
        }
    }
}

// ---------------------------------------------------------------------------
// QuestItemReward（SharedData.cs）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuestItemReward {
    pub item: ItemInfo,
    pub count: u16,
}

impl QuestItemReward {
    pub fn read(r: &mut Reader) -> Result<Self> {
        Ok(QuestItemReward {
            item: ItemInfo::read(r)?,
            count: r.read_u16()?,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        self.item.write(w);
        w.write_u16(self.count);
    }
}

// ---------------------------------------------------------------------------
// ClientRecipeInfo（ClientData.cs）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientRecipeInfo {
    pub gold: u32,
    pub chance: u8,
    pub item: UserItem,
    pub tools: Vec<UserItem>,
    pub ingredients: Vec<UserItem>,
}

impl ClientRecipeInfo {
    pub fn read(r: &mut Reader) -> Result<Self> {
        let gold = r.read_u32()?;
        let chance = r.read_u8()?;
        let item = UserItem::read(r)?;
        let mut tools = Vec::new();
        let tcount = r.read_i32()?;
        for _ in 0..tcount.max(0) {
            tools.push(UserItem::read(r)?);
        }
        let mut ingredients = Vec::new();
        let icount = r.read_i32()?;
        for _ in 0..icount.max(0) {
            ingredients.push(UserItem::read(r)?);
        }
        Ok(ClientRecipeInfo {
            gold,
            chance,
            item,
            tools,
            ingredients,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_u32(self.gold);
        w.write_u8(self.chance);
        self.item.write(w);
        w.write_i32(self.tools.len() as i32);
        for t in &self.tools {
            t.write(w);
        }
        w.write_i32(self.ingredients.len() as i32);
        for i in &self.ingredients {
            i.write(w);
        }
    }
}

// ---------------------------------------------------------------------------
// ClientFriend / ClientMail / ClientAuction（ClientData.cs）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientFriend {
    pub index: i32,
    pub name: String,
    pub memo: String,
    pub blocked: bool,
    pub online: bool,
}

impl ClientFriend {
    pub fn read(r: &mut Reader) -> Result<Self> {
        Ok(ClientFriend {
            index: r.read_i32()?,
            name: r.read_string()?,
            memo: r.read_string()?,
            blocked: r.read_bool()?,
            online: r.read_bool()?,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_i32(self.index);
        w.write_string(&self.name);
        w.write_string(&self.memo);
        w.write_bool(self.blocked);
        w.write_bool(self.online);
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientMail {
    pub mail_id: u64,
    pub sender_name: String,
    pub message: String,
    pub opened: bool,
    pub locked: bool,
    pub can_reply: bool,
    pub collected: bool,
    pub date_sent: i64,
    pub gold: u32,
    pub items: Vec<UserItem>,
}

impl ClientMail {
    pub fn read(r: &mut Reader) -> Result<Self> {
        let mail_id = r.read_u64()?;
        let sender_name = r.read_string()?;
        let message = r.read_string()?;
        let opened = r.read_bool()?;
        let locked = r.read_bool()?;
        let can_reply = r.read_bool()?;
        let collected = r.read_bool()?;
        let date_sent = r.read_i64()?;
        let gold = r.read_u32()?;
        let mut items = Vec::new();
        let icount = r.read_i32()?;
        for _ in 0..icount.max(0) {
            items.push(UserItem::read(r)?);
        }
        Ok(ClientMail {
            mail_id,
            sender_name,
            message,
            opened,
            locked,
            can_reply,
            collected,
            date_sent,
            gold,
            items,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_u64(self.mail_id);
        w.write_string(&self.sender_name);
        w.write_string(&self.message);
        w.write_bool(self.opened);
        w.write_bool(self.locked);
        w.write_bool(self.can_reply);
        w.write_bool(self.collected);
        w.write_i64(self.date_sent);
        w.write_u32(self.gold);
        w.write_i32(self.items.len() as i32);
        for i in &self.items {
            i.write(w);
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientAuction {
    pub auction_id: u64,
    pub item: UserItem,
    pub seller: String,
    pub price: u32,
    pub consignment_date: i64,
    /// MarketItemType (u8)
    pub item_type: u8,
}

impl ClientAuction {
    pub fn read(r: &mut Reader) -> Result<Self> {
        Ok(ClientAuction {
            auction_id: r.read_u64()?,
            item: UserItem::read(r)?,
            seller: r.read_string()?,
            price: r.read_u32()?,
            consignment_date: r.read_i64()?,
            item_type: r.read_u8()?,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_u64(self.auction_id);
        self.item.write(w);
        w.write_string(&self.seller);
        w.write_u32(self.price);
        w.write_i64(self.consignment_date);
        w.write_u8(self.item_type);
    }
}

// ---------------------------------------------------------------------------
// ClientQuestInfo / ClientQuestProgress（ClientData.cs）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientQuestInfo {
    pub index: i32,
    pub npc_index: u32,
    pub name: String,
    pub group: String,
    pub description: Vec<String>,
    pub task_description: Vec<String>,
    pub return_description: Vec<String>,
    pub completion_description: Vec<String>,
    pub min_level_needed: i32,
    pub max_level_needed: i32,
    pub quest_needed: i32,
    /// RequiredClass (u8)
    pub class_needed: u8,
    /// QuestType (u8)
    pub quest_type: u8,
    pub time_limit_in_seconds: i32,
    pub reward_gold: u32,
    pub reward_exp: u32,
    pub reward_credit: u32,
    pub rewards_fixed_item: Vec<QuestItemReward>,
    pub rewards_select_item: Vec<QuestItemReward>,
    pub finish_npc_index: u32,
}

impl ClientQuestInfo {
    pub fn read(r: &mut Reader) -> Result<Self> {
        let index = r.read_i32()?;
        let npc_index = r.read_u32()?;
        let name = r.read_string()?;
        let group = r.read_string()?;
        let mut description = Vec::new();
        let dcount = r.read_i32()?;
        for _ in 0..dcount.max(0) {
            description.push(r.read_string()?);
        }
        let mut task_description = Vec::new();
        let tcount = r.read_i32()?;
        for _ in 0..tcount.max(0) {
            task_description.push(r.read_string()?);
        }
        let mut return_description = Vec::new();
        let rcount = r.read_i32()?;
        for _ in 0..rcount.max(0) {
            return_description.push(r.read_string()?);
        }
        let mut completion_description = Vec::new();
        let ccount = r.read_i32()?;
        for _ in 0..ccount.max(0) {
            completion_description.push(r.read_string()?);
        }
        let min_level_needed = r.read_i32()?;
        let max_level_needed = r.read_i32()?;
        let quest_needed = r.read_i32()?;
        let class_needed = r.read_u8()?;
        let quest_type = r.read_u8()?;
        let time_limit_in_seconds = r.read_i32()?;
        let reward_gold = r.read_u32()?;
        let reward_exp = r.read_u32()?;
        let reward_credit = r.read_u32()?;
        let mut rewards_fixed_item = Vec::new();
        let fcount = r.read_i32()?;
        for _ in 0..fcount.max(0) {
            rewards_fixed_item.push(QuestItemReward::read(r)?);
        }
        let mut rewards_select_item = Vec::new();
        let scount = r.read_i32()?;
        for _ in 0..scount.max(0) {
            rewards_select_item.push(QuestItemReward::read(r)?);
        }
        let finish_npc_index = r.read_u32()?;
        Ok(ClientQuestInfo {
            index,
            npc_index,
            name,
            group,
            description,
            task_description,
            return_description,
            completion_description,
            min_level_needed,
            max_level_needed,
            quest_needed,
            class_needed,
            quest_type,
            time_limit_in_seconds,
            reward_gold,
            reward_exp,
            reward_credit,
            rewards_fixed_item,
            rewards_select_item,
            finish_npc_index,
        })
    }

    pub fn write(&self, w: &mut Writer) {
        w.write_i32(self.index);
        w.write_u32(self.npc_index);
        w.write_string(&self.name);
        w.write_string(&self.group);
        w.write_i32(self.description.len() as i32);
        for s in &self.description {
            w.write_string(s);
        }
        w.write_i32(self.task_description.len() as i32);
        for s in &self.task_description {
            w.write_string(s);
        }
        w.write_i32(self.return_description.len() as i32);
        for s in &self.return_description {
            w.write_string(s);
        }
        w.write_i32(self.completion_description.len() as i32);
        for s in &self.completion_description {
            w.write_string(s);
        }
        w.write_i32(self.min_level_needed);
        w.write_i32(self.max_level_needed);
        w.write_i32(self.quest_needed);
        w.write_u8(self.class_needed);
        w.write_u8(self.quest_type);
        w.write_i32(self.time_limit_in_seconds);
        w.write_u32(self.reward_gold);
        w.write_u32(self.reward_exp);
        w.write_u32(self.reward_credit);
        w.write_i32(self.rewards_fixed_item.len() as i32);
        for r in &self.rewards_fixed_item {
            r.write(w);
        }
        w.write_i32(self.rewards_select_item.len() as i32);
        for r in &self.rewards_select_item {
            r.write(w);
        }
        w.write_u32(self.finish_npc_index);
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientQuestProgress {
    pub id: i32,
    pub task_list: Vec<String>,
    pub taken: bool,
    pub completed: bool,
    pub new: bool,
}

impl ClientQuestProgress {
    pub fn read(r: &mut Reader) -> Result<Self> {
        let id = r.read_i32()?;
        let mut task_list = Vec::new();
        let tcount = r.read_i32()?;
        for _ in 0..tcount.max(0) {
            task_list.push(r.read_string()?);
        }
        let taken = r.read_bool()?;
        let completed = r.read_bool()?;
        let new = r.read_bool()?;
        Ok(ClientQuestProgress {
            id,
            task_list,
            taken,
            completed,
            new,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_i32(self.id);
        w.write_i32(self.task_list.len() as i32);
        for s in &self.task_list {
            w.write_string(s);
        }
        w.write_bool(self.taken);
        w.write_bool(self.completed);
        w.write_bool(self.new);
    }
}

// ---------------------------------------------------------------------------
// ClientBuff（ClientData.cs）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientBuff {
    /// BuffType (u8)
    pub buff_type: u8,
    pub visible: bool,
    pub object_id: u32,
    pub expire_time: i64,
    pub infinite: bool,
    pub paused: bool,
    pub stats: Stats,
    pub values: Vec<i32>,
}

impl ClientBuff {
    pub fn read(r: &mut Reader) -> Result<Self> {
        let buff_type = r.read_u8()?;
        let visible = r.read_bool()?;
        let object_id = r.read_u32()?;
        let expire_time = r.read_i64()?;
        let infinite = r.read_bool()?;
        let paused = r.read_bool()?;
        let stats = Stats::read(r)?;
        let mut values = Vec::new();
        let vcount = r.read_i32()?;
        for _ in 0..vcount.max(0) {
            values.push(r.read_i32()?);
        }
        Ok(ClientBuff {
            buff_type,
            visible,
            object_id,
            expire_time,
            infinite,
            paused,
            stats,
            values,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_u8(self.buff_type);
        w.write_bool(self.visible);
        w.write_u32(self.object_id);
        w.write_i64(self.expire_time);
        w.write_bool(self.infinite);
        w.write_bool(self.paused);
        self.stats.write(w);
        w.write_i32(self.values.len() as i32);
        for v in &self.values {
            w.write_i32(*v);
        }
    }
}

// ---------------------------------------------------------------------------
// ClientHeroInformation（ClientData.cs）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientHeroInformation {
    pub index: i32,
    pub name: String,
    pub level: u16,
    pub class: MirClass,
    pub gender: MirGender,
}

impl ClientHeroInformation {
    pub fn read(r: &mut Reader) -> Result<Self> {
        Ok(ClientHeroInformation {
            index: r.read_i32()?,
            name: r.read_string()?,
            level: r.read_u16()?,
            class: match r.read_u8()? {
                0 => MirClass::Warrior,
                1 => MirClass::Wizard,
                2 => MirClass::Taoist,
                3 => MirClass::Assassin,
                _ => MirClass::Archer,
            },
            gender: match r.read_u8()? {
                0 => MirGender::Male,
                _ => MirGender::Female,
            },
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_i32(self.index);
        w.write_string(&self.name);
        w.write_u16(self.level);
        w.write_u8(self.class as u8);
        w.write_u8(self.gender as u8);
    }
}

// ---------------------------------------------------------------------------
// ClientMonsterInfo（MonsterData.cs）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientMonsterInfo {
    pub index: i32,
    pub name: String,
    pub game_name: String,
    /// Monster (u16)
    pub image: u16,
    pub ai: u8,
    pub effect: u8,
    pub level: u16,
    pub view_range: u8,
    pub cool_eye: u8,
    pub light: u8,
    pub attack_speed: u16,
    pub move_speed: u16,
    pub experience: u32,
    pub can_push: bool,
    pub can_tame: bool,
    pub auto_rev: bool,
    pub undead: bool,
    pub can_recall: bool,
    pub stats: Stats,
}

impl ClientMonsterInfo {
    pub fn read(r: &mut Reader) -> Result<Self> {
        let index = r.read_i32()?;
        let name = r.read_string()?;
        let game_name = r.read_string()?;
        let image = r.read_u16()?;
        let ai = r.read_u8()?;
        let effect = r.read_u8()?;
        let level = r.read_u16()?;
        let view_range = r.read_u8()?;
        let cool_eye = r.read_u8()?;
        let light = r.read_u8()?;
        let attack_speed = r.read_u16()?;
        let move_speed = r.read_u16()?;
        let experience = r.read_u32()?;
        let can_push = r.read_bool()?;
        let can_tame = r.read_bool()?;
        let auto_rev = r.read_bool()?;
        let undead = r.read_bool()?;
        let can_recall = r.read_bool()?;
        let stats = Stats::read(r)?;
        Ok(ClientMonsterInfo {
            index,
            name,
            game_name,
            image,
            ai,
            effect,
            level,
            view_range,
            cool_eye,
            light,
            attack_speed,
            move_speed,
            experience,
            can_push,
            can_tame,
            auto_rev,
            undead,
            can_recall,
            stats,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_i32(self.index);
        w.write_string(&self.name);
        w.write_string(&self.game_name);
        w.write_u16(self.image);
        w.write_u8(self.ai);
        w.write_u8(self.effect);
        w.write_u16(self.level);
        w.write_u8(self.view_range);
        w.write_u8(self.cool_eye);
        w.write_u8(self.light);
        w.write_u16(self.attack_speed);
        w.write_u16(self.move_speed);
        w.write_u32(self.experience);
        w.write_bool(self.can_push);
        w.write_bool(self.can_tame);
        w.write_bool(self.auto_rev);
        w.write_bool(self.undead);
        w.write_bool(self.can_recall);
        self.stats.write(w);
    }
}

// ---------------------------------------------------------------------------
// RankCharacterInfo（SharedData.cs）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RankCharacterInfo {
    pub player_id: i64,
    pub name: String,
    pub level: i32,
    pub class: MirClass,
}

impl RankCharacterInfo {
    pub fn read(r: &mut Reader) -> Result<Self> {
        Ok(RankCharacterInfo {
            player_id: r.read_i64()?,
            name: r.read_string()?,
            level: r.read_i32()?,
            class: match r.read_u8()? {
                0 => MirClass::Warrior,
                1 => MirClass::Wizard,
                2 => MirClass::Taoist,
                3 => MirClass::Assassin,
                _ => MirClass::Archer,
            },
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_i64(self.player_id);
        w.write_string(&self.name);
        w.write_i32(self.level);
        w.write_u8(self.class as u8);
    }
}
