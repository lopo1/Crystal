//! batch_5 —— `Shared/ServerPackets.cs` 第 4768–5808 行范围（HeroInformation + SB5 清单）。
//!
//! 顺序严格对齐 C# 源码: HeroInformation(4768) → UnlockHeroAutoPot(4904) → ... → NPCReset(5798)。

use super::*;
use crate::binary::{Argb, Point, Reader, Writer};
use crate::frame::PacketCodec;
use crate::ids::ServerPacketId;
use crate::types::*;
use crate::Result;

// ----------------------------- HeroInformation -----------------------------
// C#: `public sealed class HeroInformation : UserInformation`（第 4768 行）。
// 注意: C# 中该类**完全重写**了 ReadPacket/WritePacket，并不复用基类 UserInformation
// 的线格式，读写顺序为: ObjectID, Name, Class, Gender, Level, Hair, HP, MP,
// Experience, MaxExperience, Inventory, Equipment, Magics, AutoPot, AutoHPPercent,
// AutoMPPercent, HPItemIndex, MPItemIndex。
// 另外背包空槽标志方向与 UserInformation 相反: C# 写 `Items[i] != null`（true=有物品），
// 因此不能复用 read_item_slots/write_item_slots（那套是 true=空槽），这里手写循环。

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeroInformation {
    pub object_id: u32,
    pub name: String,
    pub class: MirClass,
    pub gender: MirGender,
    pub level: u16,
    pub hair: u8,
    pub hp: i32,
    pub mp: i32,
    pub experience: i64,
    pub max_experience: i64,
    /// 空槽标志: true=有物品（与 read_item_slots 方向相反）
    pub inventory: Option<ItemSlots>,
    /// 空槽标志: true=有物品（与 read_item_slots 方向相反）
    pub equipment: Option<ItemSlots>,
    pub magics: Vec<ClientMagic>,
    pub auto_pot: bool,
    pub auto_hp_percent: u8,
    pub auto_mp_percent: u8,
    pub hp_item_index: i32,
    pub mp_item_index: i32,
}

/// 读取"true=有物品"方向的物品槽数组（HeroInformation 专用，勿与 read_item_slots 混用）
fn read_item_slots_present_first(r: &mut Reader) -> Result<Option<ItemSlots>> {
    if !r.read_bool()? {
        return Ok(None);
    }
    let len = r.read_i32()?;
    let mut slots = Vec::with_capacity(len.max(0) as usize);
    for _ in 0..len.max(0) {
        if r.read_bool()? {
            slots.push(Some(UserItem::read(r)?));
        } else {
            slots.push(None);
        }
    }
    Ok(Some(slots))
}

fn write_item_slots_present_first(w: &mut Writer, slots: &Option<ItemSlots>) {
    match slots {
        None => w.write_bool(false),
        Some(items) => {
            w.write_bool(true);
            w.write_i32(items.len() as i32);
            for item in items {
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
}

impl PacketCodec for HeroInformation {
    const ID: i16 = ServerPacketId::HeroInformation as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let object_id = r.read_u32()?;
        let name = r.read_string()?;
        let class = match r.read_u8()? {
            0 => MirClass::Warrior,
            1 => MirClass::Wizard,
            2 => MirClass::Taoist,
            3 => MirClass::Assassin,
            _ => MirClass::Archer,
        };
        let gender = match r.read_u8()? {
            0 => MirGender::Male,
            _ => MirGender::Female,
        };
        let level = r.read_u16()?;
        let hair = r.read_u8()?;
        let hp = r.read_i32()?;
        let mp = r.read_i32()?;
        let experience = r.read_i64()?;
        let max_experience = r.read_i64()?;
        let inventory = read_item_slots_present_first(r)?;
        let equipment = read_item_slots_present_first(r)?;
        let mut magics = Vec::new();
        let mcount = r.read_i32()?;
        for _ in 0..mcount.max(0) {
            magics.push(ClientMagic::read(r)?);
        }
        let auto_pot = r.read_bool()?;
        let auto_hp_percent = r.read_u8()?;
        let auto_mp_percent = r.read_u8()?;
        let hp_item_index = r.read_i32()?;
        let mp_item_index = r.read_i32()?;
        Ok(HeroInformation {
            object_id,
            name,
            class,
            gender,
            level,
            hair,
            hp,
            mp,
            experience,
            max_experience,
            inventory,
            equipment,
            magics,
            auto_pot,
            auto_hp_percent,
            auto_mp_percent,
            hp_item_index,
            mp_item_index,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_string(&self.name);
        w.write_u8(self.class as u8);
        w.write_u8(self.gender as u8);
        w.write_u16(self.level);
        w.write_u8(self.hair);
        w.write_i32(self.hp);
        w.write_i32(self.mp);
        w.write_i64(self.experience);
        w.write_i64(self.max_experience);
        write_item_slots_present_first(w, &self.inventory);
        write_item_slots_present_first(w, &self.equipment);
        w.write_i32(self.magics.len() as i32);
        for m in &self.magics {
            m.write(w);
        }
        w.write_bool(self.auto_pot);
        w.write_u8(self.auto_hp_percent);
        w.write_u8(self.auto_mp_percent);
        w.write_i32(self.hp_item_index);
        w.write_i32(self.mp_item_index);
    }
}

// ----------------------------- UnlockHeroAutoPot -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UnlockHeroAutoPot;

impl PacketCodec for UnlockHeroAutoPot {
    const ID: i16 = ServerPacketId::UnlockHeroAutoPot as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(UnlockHeroAutoPot)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- SetAutoPotValue -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SetAutoPotValue {
    /// Stat (u8)
    pub stat: u8,
    pub value: u32,
}

impl PacketCodec for SetAutoPotValue {
    const ID: i16 = ServerPacketId::SetAutoPotValue as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(SetAutoPotValue {
            stat: r.read_u8()?,
            value: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.stat);
        w.write_u32(self.value);
    }
}

// ----------------------------- SetAutoPotItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SetAutoPotItem {
    pub grid: MirGridType,
    pub item_index: i32,
}

impl PacketCodec for SetAutoPotItem {
    const ID: i16 = ServerPacketId::SetAutoPotItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(SetAutoPotItem {
            grid: MirGridType::from_u8(r.read_u8()?),
            item_index: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.grid.to_u8());
        w.write_i32(self.item_index);
    }
}

// ----------------------------- SetHeroBehaviour -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SetHeroBehaviour {
    /// HeroBehaviour (u8)
    pub behaviour: u8,
}

impl PacketCodec for SetHeroBehaviour {
    const ID: i16 = ServerPacketId::SetHeroBehaviour as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(SetHeroBehaviour {
            behaviour: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.behaviour);
    }
}

// ----------------------------- ManageHeroes -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ManageHeroes {
    pub maximum_count: i32,
    pub current_hero: Option<ClientHeroInformation>,
    /// 外层 Option = C# `Heroes != null`；内层每槽 bool = `Heroes[i] != null`（true=有数据）
    pub heroes: Option<Vec<Option<ClientHeroInformation>>>,
}

impl PacketCodec for ManageHeroes {
    const ID: i16 = ServerPacketId::ManageHeroes as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let maximum_count = r.read_i32()?;
        let current_hero = if r.read_bool()? {
            Some(ClientHeroInformation::read(r)?)
        } else {
            None
        };
        let heroes = if r.read_bool()? {
            let len = r.read_i32()?;
            let mut list = Vec::with_capacity(len.max(0) as usize);
            for _ in 0..len.max(0) {
                if r.read_bool()? {
                    list.push(Some(ClientHeroInformation::read(r)?));
                } else {
                    list.push(None);
                }
            }
            Some(list)
        } else {
            None
        };
        Ok(ManageHeroes {
            maximum_count,
            current_hero,
            heroes,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.maximum_count);
        match &self.current_hero {
            Some(h) => {
                w.write_bool(true);
                h.write(w);
            }
            None => w.write_bool(false),
        }
        match &self.heroes {
            None => w.write_bool(false),
            Some(list) => {
                w.write_bool(true);
                w.write_i32(list.len() as i32);
                for h in list {
                    match h {
                        Some(h) => {
                            w.write_bool(true);
                            h.write(w);
                        }
                        None => w.write_bool(false),
                    }
                }
            }
        }
    }
}

// ----------------------------- ChangeHero -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangeHero {
    pub from_index: i32,
}

impl PacketCodec for ChangeHero {
    const ID: i16 = ServerPacketId::ChangeHero as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ChangeHero {
            from_index: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.from_index);
    }
}

// ----------------------------- DefaultNPC -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DefaultNPC {
    pub object_id: u32,
}

impl PacketCodec for DefaultNPC {
    const ID: i16 = ServerPacketId::DefaultNPC as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(DefaultNPC {
            object_id: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
    }
}

// ----------------------------- NPCUpdate -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NPCUpdate {
    pub npc_id: u32,
}

impl PacketCodec for NPCUpdate {
    const ID: i16 = ServerPacketId::NPCUpdate as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(NPCUpdate {
            npc_id: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.npc_id);
    }
}

// ----------------------------- NPCImageUpdate -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NPCImageUpdate {
    pub object_id: u32,
    pub image: u16,
    pub colour: Argb,
}

impl PacketCodec for NPCImageUpdate {
    const ID: i16 = ServerPacketId::NPCImageUpdate as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(NPCImageUpdate {
            object_id: r.read_u32()?,
            image: r.read_u16()?,
            colour: Argb::from_i32(r.read_i32()?),
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_u16(self.image);
        w.write_i32(self.colour.to_i32());
    }
}

// ----------------------------- MountUpdate -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MountUpdate {
    pub object_id: u32,
    pub mount_type: i16,
    pub riding_mount: bool,
}

impl PacketCodec for MountUpdate {
    const ID: i16 = ServerPacketId::MountUpdate as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(MountUpdate {
            object_id: r.read_u32()?,
            mount_type: r.read_i16()?,
            riding_mount: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_i16(self.mount_type);
        w.write_bool(self.riding_mount);
    }
}

// ----------------------------- TransformUpdate -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TransformUpdate {
    pub object_id: u32,
    pub transform_type: i16,
}

impl PacketCodec for TransformUpdate {
    const ID: i16 = ServerPacketId::TransformUpdate as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(TransformUpdate {
            object_id: r.read_u32()?,
            transform_type: r.read_i16()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_i16(self.transform_type);
    }
}

// ----------------------------- EquipSlotItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EquipSlotItem {
    pub grid: MirGridType,
    pub unique_id: u64,
    pub to: i32,
    pub grid_to: MirGridType,
    pub success: bool,
}

impl PacketCodec for EquipSlotItem {
    const ID: i16 = ServerPacketId::EquipSlotItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(EquipSlotItem {
            grid: MirGridType::from_u8(r.read_u8()?),
            unique_id: r.read_u64()?,
            to: r.read_i32()?,
            grid_to: MirGridType::from_u8(r.read_u8()?),
            success: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.grid.to_u8());
        w.write_u64(self.unique_id);
        w.write_i32(self.to);
        w.write_u8(self.grid_to.to_u8());
        w.write_bool(self.success);
    }
}

// ----------------------------- FishingUpdate -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FishingUpdate {
    pub object_id: u32,
    pub fishing: bool,
    pub progress_percent: i32,
    pub chance_percent: i32,
    pub fishing_point: Point,
    pub found_fish: bool,
}

impl PacketCodec for FishingUpdate {
    const ID: i16 = ServerPacketId::FishingUpdate as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(FishingUpdate {
            object_id: r.read_u32()?,
            fishing: r.read_bool()?,
            progress_percent: r.read_i32()?,
            chance_percent: r.read_i32()?,
            fishing_point: Point::read(r)?,
            found_fish: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_bool(self.fishing);
        w.write_i32(self.progress_percent);
        w.write_i32(self.chance_percent);
        self.fishing_point.write(w);
        w.write_bool(self.found_fish);
    }
}

// ----------------------------- ChangeQuest -----------------------------
// C# 5181 行附近 `//UpdateQuests` 为注释掉的类，不属于本批。

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangeQuest {
    pub quest: ClientQuestProgress,
    /// QuestState (u8)
    pub quest_state: u8,
    pub track_quest: bool,
}

impl PacketCodec for ChangeQuest {
    const ID: i16 = ServerPacketId::ChangeQuest as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ChangeQuest {
            quest: ClientQuestProgress::read(r)?,
            quest_state: r.read_u8()?,
            track_quest: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        self.quest.write(w);
        w.write_u8(self.quest_state);
        w.write_bool(self.track_quest);
    }
}

// ----------------------------- CompleteQuest -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompleteQuest {
    pub completed_quests: Vec<i32>,
}

impl PacketCodec for CompleteQuest {
    const ID: i16 = ServerPacketId::CompleteQuest as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let count = r.read_i32()?;
        let mut completed_quests = Vec::with_capacity(count.max(0) as usize);
        for _ in 0..count.max(0) {
            completed_quests.push(r.read_i32()?);
        }
        Ok(CompleteQuest { completed_quests })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.completed_quests.len() as i32);
        for q in &self.completed_quests {
            w.write_i32(*q);
        }
    }
}

// ----------------------------- ShareQuest -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShareQuest {
    pub quest_index: i32,
    pub sharer_name: String,
}

impl PacketCodec for ShareQuest {
    const ID: i16 = ServerPacketId::ShareQuest as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ShareQuest {
            quest_index: r.read_i32()?,
            sharer_name: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.quest_index);
        w.write_string(&self.sharer_name);
    }
}

// ----------------------------- NewQuestInfo -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewQuestInfo {
    pub info: ClientQuestInfo,
}

impl PacketCodec for NewQuestInfo {
    const ID: i16 = ServerPacketId::NewQuestInfo as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(NewQuestInfo {
            info: ClientQuestInfo::read(r)?,
        })
    }

    fn write(&self, w: &mut Writer) {
        self.info.write(w);
    }
}

// ----------------------------- GainedQuestItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GainedQuestItem {
    pub item: UserItem,
}

impl PacketCodec for GainedQuestItem {
    const ID: i16 = ServerPacketId::GainedQuestItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(GainedQuestItem {
            item: UserItem::read(r)?,
        })
    }

    fn write(&self, w: &mut Writer) {
        self.item.write(w);
    }
}

// ----------------------------- DeleteQuestItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeleteQuestItem {
    pub unique_id: u64,
    pub count: u16,
}

impl PacketCodec for DeleteQuestItem {
    const ID: i16 = ServerPacketId::DeleteQuestItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(DeleteQuestItem {
            unique_id: r.read_u64()?,
            count: r.read_u16()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
        w.write_u16(self.count);
    }
}

// ----------------------------- GameShopItem（内嵌，GameShopInfo 用） -----------------------------
// 对应 `Shared/Data/ItemData.cs` 的 `GameShopItem`，线格式取 `Save(writer, true)` / 构造 `(reader, true)` 版本。

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameShopItem {
    pub item_index: i32,
    pub g_index: i32,
    pub info: ItemInfo,
    pub gold_price: u32,
    pub credit_price: u32,
    pub count: u16,
    pub class: String,
    pub category: String,
    pub stock: i32,
    pub i_stock: bool,
    pub deal: bool,
    pub top_item: bool,
    pub date: i64,
    pub can_buy_credit: bool,
    pub can_buy_gold: bool,
}

impl GameShopItem {
    pub fn read(r: &mut Reader) -> Result<Self> {
        Ok(GameShopItem {
            item_index: r.read_i32()?,
            g_index: r.read_i32()?,
            info: ItemInfo::read(r)?,
            gold_price: r.read_u32()?,
            credit_price: r.read_u32()?,
            count: r.read_u16()?,
            class: r.read_string()?,
            category: r.read_string()?,
            stock: r.read_i32()?,
            i_stock: r.read_bool()?,
            deal: r.read_bool()?,
            top_item: r.read_bool()?,
            date: r.read_i64()?,
            can_buy_credit: r.read_bool()?,
            can_buy_gold: r.read_bool()?,
        })
    }

    pub fn write(&self, w: &mut Writer) {
        w.write_i32(self.item_index);
        w.write_i32(self.g_index);
        self.info.write(w);
        w.write_u32(self.gold_price);
        w.write_u32(self.credit_price);
        w.write_u16(self.count);
        w.write_string(&self.class);
        w.write_string(&self.category);
        w.write_i32(self.stock);
        w.write_bool(self.i_stock);
        w.write_bool(self.deal);
        w.write_bool(self.top_item);
        w.write_i64(self.date);
        w.write_bool(self.can_buy_credit);
        w.write_bool(self.can_buy_gold);
    }
}

// ----------------------------- GameShopInfo -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameShopInfo {
    pub item: GameShopItem,
    pub stock_level: i32,
}

impl PacketCodec for GameShopInfo {
    const ID: i16 = ServerPacketId::GameShopInfo as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(GameShopInfo {
            item: GameShopItem::read(r)?,
            stock_level: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        self.item.write(w);
        w.write_i32(self.stock_level);
    }
}

// ----------------------------- GameShopStock -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameShopStock {
    pub g_index: i32,
    pub stock_level: i32,
}

impl PacketCodec for GameShopStock {
    const ID: i16 = ServerPacketId::GameShopStock as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(GameShopStock {
            g_index: r.read_i32()?,
            stock_level: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.g_index);
        w.write_i32(self.stock_level);
    }
}

// ----------------------------- CancelReincarnation -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CancelReincarnation;

impl PacketCodec for CancelReincarnation {
    const ID: i16 = ServerPacketId::CancelReincarnation as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(CancelReincarnation)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- RequestReincarnation -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestReincarnation;

impl PacketCodec for RequestReincarnation {
    const ID: i16 = ServerPacketId::RequestReincarnation as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(RequestReincarnation)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- UserBackStep -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserBackStep {
    pub location: Point,
    pub direction: MirDirection,
}

impl PacketCodec for UserBackStep {
    const ID: i16 = ServerPacketId::UserBackStep as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(UserBackStep {
            location: Point::read(r)?,
            direction: MirDirection::from_u8(r.read_u8()?),
        })
    }

    fn write(&self, w: &mut Writer) {
        self.location.write(w);
        w.write_u8(self.direction.to_u8());
    }
}

// ----------------------------- ObjectBackStep -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectBackStep {
    pub object_id: u32,
    pub location: Point,
    pub direction: MirDirection,
    pub distance: i32,
}

impl PacketCodec for ObjectBackStep {
    const ID: i16 = ServerPacketId::ObjectBackStep as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectBackStep {
            object_id: r.read_u32()?,
            location: Point::read(r)?,
            direction: MirDirection::from_u8(r.read_u8()?),
            distance: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        self.location.write(w);
        w.write_u8(self.direction.to_u8());
        w.write_i32(self.distance);
    }
}

// ----------------------------- UserDashAttack -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserDashAttack {
    pub location: Point,
    pub direction: MirDirection,
}

impl PacketCodec for UserDashAttack {
    const ID: i16 = ServerPacketId::UserDashAttack as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(UserDashAttack {
            location: Point::read(r)?,
            direction: MirDirection::from_u8(r.read_u8()?),
        })
    }

    fn write(&self, w: &mut Writer) {
        self.location.write(w);
        w.write_u8(self.direction.to_u8());
    }
}

// ----------------------------- ObjectDashAttack -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectDashAttack {
    pub object_id: u32,
    pub location: Point,
    pub direction: MirDirection,
    pub distance: i32,
}

impl PacketCodec for ObjectDashAttack {
    const ID: i16 = ServerPacketId::ObjectDashAttack as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectDashAttack {
            object_id: r.read_u32()?,
            location: Point::read(r)?,
            direction: MirDirection::from_u8(r.read_u8()?),
            distance: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        self.location.write(w);
        w.write_u8(self.direction.to_u8());
        w.write_i32(self.distance);
    }
}

// ----------------------------- UserAttackMove -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserAttackMove {
    pub location: Point,
    pub direction: MirDirection,
}

impl PacketCodec for UserAttackMove {
    const ID: i16 = ServerPacketId::UserAttackMove as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(UserAttackMove {
            location: Point::read(r)?,
            direction: MirDirection::from_u8(r.read_u8()?),
        })
    }

    fn write(&self, w: &mut Writer) {
        self.location.write(w);
        w.write_u8(self.direction.to_u8());
    }
}

// ----------------------------- CombineItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CombineItem {
    pub grid: MirGridType,
    pub id_from: u64,
    pub id_to: u64,
    pub success: bool,
    pub destroy: bool,
}

impl PacketCodec for CombineItem {
    const ID: i16 = ServerPacketId::CombineItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(CombineItem {
            grid: MirGridType::from_u8(r.read_u8()?),
            id_from: r.read_u64()?,
            id_to: r.read_u64()?,
            success: r.read_bool()?,
            destroy: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.grid.to_u8());
        w.write_u64(self.id_from);
        w.write_u64(self.id_to);
        w.write_bool(self.success);
        w.write_bool(self.destroy);
    }
}

// ----------------------------- ItemUpgraded -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemUpgraded {
    pub item: UserItem,
}

impl PacketCodec for ItemUpgraded {
    const ID: i16 = ServerPacketId::ItemUpgraded as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ItemUpgraded {
            item: UserItem::read(r)?,
        })
    }

    fn write(&self, w: &mut Writer) {
        self.item.write(w);
    }
}

// ----------------------------- SetConcentration -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SetConcentration {
    pub object_id: u32,
    pub enabled: bool,
    pub interrupted: bool,
}

impl PacketCodec for SetConcentration {
    const ID: i16 = ServerPacketId::SetConcentration as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(SetConcentration {
            object_id: r.read_u32()?,
            enabled: r.read_bool()?,
            interrupted: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_bool(self.enabled);
        w.write_bool(self.interrupted);
    }
}

// ----------------------------- SetElemental -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SetElemental {
    pub object_id: u32,
    pub enabled: bool,
    pub casted: bool,
    pub value: u32,
    pub element_type: u32,
    pub exp_last: u32,
}

impl PacketCodec for SetElemental {
    const ID: i16 = ServerPacketId::SetElemental as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(SetElemental {
            object_id: r.read_u32()?,
            enabled: r.read_bool()?,
            casted: r.read_bool()?,
            value: r.read_u32()?,
            element_type: r.read_u32()?,
            exp_last: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_bool(self.enabled);
        w.write_bool(self.casted);
        w.write_u32(self.value);
        w.write_u32(self.element_type);
        w.write_u32(self.exp_last);
    }
}

// ----------------------------- ObjectDeco -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectDeco {
    pub object_id: u32,
    pub location: Point,
    pub image: i32,
}

impl PacketCodec for ObjectDeco {
    const ID: i16 = ServerPacketId::ObjectDeco as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectDeco {
            object_id: r.read_u32()?,
            location: Point::read(r)?,
            image: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        self.location.write(w);
        w.write_i32(self.image);
    }
}

// ----------------------------- ObjectSneaking -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectSneaking {
    pub object_id: u32,
    pub sneaking_active: bool,
}

impl PacketCodec for ObjectSneaking {
    const ID: i16 = ServerPacketId::ObjectSneaking as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectSneaking {
            object_id: r.read_u32()?,
            sneaking_active: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_bool(self.sneaking_active);
    }
}

// ----------------------------- ObjectLevelEffects -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectLevelEffects {
    pub object_id: u32,
    pub level_effects: LevelEffects,
}

impl PacketCodec for ObjectLevelEffects {
    const ID: i16 = ServerPacketId::ObjectLevelEffects as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectLevelEffects {
            object_id: r.read_u32()?,
            level_effects: LevelEffects(r.read_u16()?),
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_u16(self.level_effects.0);
    }
}

// ----------------------------- SetBindingShot -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SetBindingShot {
    pub object_id: u32,
    pub enabled: bool,
    pub value: i64,
}

impl PacketCodec for SetBindingShot {
    const ID: i16 = ServerPacketId::SetBindingShot as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(SetBindingShot {
            object_id: r.read_u32()?,
            enabled: r.read_bool()?,
            value: r.read_i64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_bool(self.enabled);
        w.write_i64(self.value);
    }
}

// ----------------------------- SendOutputMessage -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SendOutputMessage {
    pub message: String,
    /// OutputMessageType (u8)
    pub r#type: u8,
}

impl PacketCodec for SendOutputMessage {
    const ID: i16 = ServerPacketId::SendOutputMessage as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(SendOutputMessage {
            message: r.read_string()?,
            r#type: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.message);
        w.write_u8(self.r#type);
    }
}

// ----------------------------- NPCAwakening -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NPCAwakening;

impl PacketCodec for NPCAwakening {
    const ID: i16 = ServerPacketId::NPCAwakening as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(NPCAwakening)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- NPCDisassemble -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NPCDisassemble;

impl PacketCodec for NPCDisassemble {
    const ID: i16 = ServerPacketId::NPCDisassemble as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(NPCDisassemble)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- NPCDowngrade -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NPCDowngrade;

impl PacketCodec for NPCDowngrade {
    const ID: i16 = ServerPacketId::NPCDowngrade as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(NPCDowngrade)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- NPCReset -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NPCReset;

impl PacketCodec for NPCReset {
    const ID: i16 = ServerPacketId::NPCReset as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(NPCReset)
    }

    fn write(&self, _w: &mut Writer) {}
}
