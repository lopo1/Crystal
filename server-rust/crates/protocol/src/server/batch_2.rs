//! 服务器→客户端数据包 batch_2（对应 `Shared/ServerPackets.cs` 行 ~2210–3106）。
//!
//! 覆盖 ID 67–102: 金币/声望、怪物/攻击/受击/死亡、耐久/血量、经验/等级、
//! NPC/任务、传送等。字段顺序严格按 C# `ReadPacket`/`WritePacket`（两者必须一致）。
//!
//! 注意: `ObjectMonster` 的读/写顺序与字段声明顺序不同（`Hidden, ShockTime,
//! BindingShotCenter, Extra, ExtraByte`），以 Read/Write 为准。
//!
//! 注意: ID 102 `NPCGoods`（`Compressed => true`，含 `f32`）与此批次边界重叠，
//! 已在 `batch_3.rs` 移植（行 3074 起），为避免 glob 重导出歧义，本文件不再重复定义；
//! 其实现与本批风格一致（count + UserItem 列表、f32 Rate、`PanelType` u8、bool）。

use crate::binary::{Argb, Point, Reader, Writer};
use crate::frame::PacketCodec;
use crate::ids::ServerPacketId;
use crate::types::MirDirection;
use crate::Result;

// ----------------------------- ID 67: GainedGold -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GainedGold {
    pub gold: u32,
}

impl PacketCodec for GainedGold {
    const ID: i16 = ServerPacketId::GainedGold as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(GainedGold {
            gold: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.gold);
    }
}

// ----------------------------- ID 68: LoseGold -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoseGold {
    pub gold: u32,
}

impl PacketCodec for LoseGold {
    const ID: i16 = ServerPacketId::LoseGold as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(LoseGold {
            gold: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.gold);
    }
}

// ----------------------------- ID 69: GainedCredit -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GainedCredit {
    pub credit: u32,
}

impl PacketCodec for GainedCredit {
    const ID: i16 = ServerPacketId::GainedCredit as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(GainedCredit {
            credit: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.credit);
    }
}

// ----------------------------- ID 70: LoseCredit -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoseCredit {
    pub credit: u32,
}

impl PacketCodec for LoseCredit {
    const ID: i16 = ServerPacketId::LoseCredit as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(LoseCredit {
            credit: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.credit);
    }
}

// ----------------------------- ID 71: ObjectMonster -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectMonster {
    pub object_id: u32,
    pub name: String,
    pub name_colour: Argb,
    pub location: Point,
    /// Monster (u16) —— C# 枚举
    pub image: u16,
    pub direction: MirDirection,
    pub effect: u8,
    pub ai: u8,
    pub light: u8,
    pub dead: bool,
    pub skeleton: bool,
    /// PoisonType (u16) —— C# 枚举（位标志）
    pub poison: u16,
    pub hidden: bool,
    pub shock_time: i64,
    pub binding_shot_center: bool,
    pub extra: bool,
    pub extra_byte: u8,
    pub master_object_id: u32,
    /// MonsterType (u8) —— C# 枚举
    pub rarity: u8,
    /// BuffType (u8) 列表 —— C# 枚举
    pub buffs: Vec<u8>,
}

impl PacketCodec for ObjectMonster {
    const ID: i16 = ServerPacketId::ObjectMonster as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let object_id = r.read_u32()?;
        let name = r.read_string()?;
        let name_colour = Argb::from_i32(r.read_i32()?);
        let location = Point::read(r)?;
        let image = r.read_u16()?;
        let direction = MirDirection::from_u8(r.read_u8()?);
        let effect = r.read_u8()?;
        let ai = r.read_u8()?;
        let light = r.read_u8()?;
        let dead = r.read_bool()?;
        let skeleton = r.read_bool()?;
        let poison = r.read_u16()?;
        let hidden = r.read_bool()?;
        let shock_time = r.read_i64()?;
        let binding_shot_center = r.read_bool()?;
        let extra = r.read_bool()?;
        let extra_byte = r.read_u8()?;
        let master_object_id = r.read_u32()?;
        let rarity = r.read_u8()?;
        let mut buffs = Vec::new();
        let bcount = r.read_i32()?;
        for _ in 0..bcount.max(0) {
            buffs.push(r.read_u8()?);
        }
        Ok(ObjectMonster {
            object_id,
            name,
            name_colour,
            location,
            image,
            direction,
            effect,
            ai,
            light,
            dead,
            skeleton,
            poison,
            hidden,
            shock_time,
            binding_shot_center,
            extra,
            extra_byte,
            master_object_id,
            rarity,
            buffs,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_string(&self.name);
        w.write_i32(self.name_colour.to_i32());
        self.location.write(w);
        w.write_u16(self.image);
        w.write_u8(self.direction.to_u8());
        w.write_u8(self.effect);
        w.write_u8(self.ai);
        w.write_u8(self.light);
        w.write_bool(self.dead);
        w.write_bool(self.skeleton);
        w.write_u16(self.poison);
        w.write_bool(self.hidden);
        w.write_i64(self.shock_time);
        w.write_bool(self.binding_shot_center);
        w.write_bool(self.extra);
        w.write_u8(self.extra_byte);
        w.write_u32(self.master_object_id);
        w.write_u8(self.rarity);
        w.write_i32(self.buffs.len() as i32);
        for b in &self.buffs {
            w.write_u8(*b);
        }
    }
}

// ----------------------------- ID 72: ObjectAttack -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectAttack {
    pub object_id: u32,
    pub location: Point,
    pub direction: MirDirection,
    /// Spell (u8) —— C# 枚举
    pub spell: u8,
    pub level: u8,
    pub r#type: u8,
}

impl PacketCodec for ObjectAttack {
    const ID: i16 = ServerPacketId::ObjectAttack as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectAttack {
            object_id: r.read_u32()?,
            location: Point::read(r)?,
            direction: MirDirection::from_u8(r.read_u8()?),
            spell: r.read_u8()?,
            level: r.read_u8()?,
            r#type: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        self.location.write(w);
        w.write_u8(self.direction.to_u8());
        w.write_u8(self.spell);
        w.write_u8(self.level);
        w.write_u8(self.r#type);
    }
}

// ----------------------------- ID 73: Struck -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Struck {
    pub attacker_id: u32,
}

impl PacketCodec for Struck {
    const ID: i16 = ServerPacketId::Struck as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Struck {
            attacker_id: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.attacker_id);
    }
}

// ----------------------------- ID 74: ObjectStruck -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectStruck {
    pub object_id: u32,
    pub attacker_id: u32,
    pub location: Point,
    pub direction: MirDirection,
}

impl PacketCodec for ObjectStruck {
    const ID: i16 = ServerPacketId::ObjectStruck as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectStruck {
            object_id: r.read_u32()?,
            attacker_id: r.read_u32()?,
            location: Point::read(r)?,
            direction: MirDirection::from_u8(r.read_u8()?),
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_u32(self.attacker_id);
        self.location.write(w);
        w.write_u8(self.direction.to_u8());
    }
}

// ----------------------------- ID 75: DamageIndicator -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DamageIndicator {
    pub damage: i32,
    /// DamageType (u8) —— C# 枚举
    pub r#type: u8,
    pub object_id: u32,
}

impl PacketCodec for DamageIndicator {
    const ID: i16 = ServerPacketId::DamageIndicator as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(DamageIndicator {
            damage: r.read_i32()?,
            r#type: r.read_u8()?,
            object_id: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.damage);
        w.write_u8(self.r#type);
        w.write_u32(self.object_id);
    }
}

// ----------------------------- ID 76: DuraChanged -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DuraChanged {
    pub unique_id: u64,
    pub current_dura: u16,
}

impl PacketCodec for DuraChanged {
    const ID: i16 = ServerPacketId::DuraChanged as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(DuraChanged {
            unique_id: r.read_u64()?,
            current_dura: r.read_u16()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
        w.write_u16(self.current_dura);
    }
}

// ----------------------------- ID 77: HealthChanged -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HealthChanged {
    pub hp: i32,
    pub mp: i32,
}

impl PacketCodec for HealthChanged {
    const ID: i16 = ServerPacketId::HealthChanged as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(HealthChanged {
            hp: r.read_i32()?,
            mp: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.hp);
        w.write_i32(self.mp);
    }
}

// ----------------------------- ID 78: HeroHealthChanged -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeroHealthChanged {
    pub hp: i32,
    pub mp: i32,
}

impl PacketCodec for HeroHealthChanged {
    const ID: i16 = ServerPacketId::HeroHealthChanged as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(HeroHealthChanged {
            hp: r.read_i32()?,
            mp: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.hp);
        w.write_i32(self.mp);
    }
}

// ----------------------------- ID 79: DeleteItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeleteItem {
    pub unique_id: u64,
    pub count: u16,
}

impl PacketCodec for DeleteItem {
    const ID: i16 = ServerPacketId::DeleteItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(DeleteItem {
            unique_id: r.read_u64()?,
            count: r.read_u16()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
        w.write_u16(self.count);
    }
}

// ----------------------------- ID 80: Death -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Death {
    pub location: Point,
    pub direction: MirDirection,
}

impl PacketCodec for Death {
    const ID: i16 = ServerPacketId::Death as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Death {
            location: Point::read(r)?,
            direction: MirDirection::from_u8(r.read_u8()?),
        })
    }

    fn write(&self, w: &mut Writer) {
        self.location.write(w);
        w.write_u8(self.direction.to_u8());
    }
}

// ----------------------------- ID 81: ObjectDied -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectDied {
    pub object_id: u32,
    pub location: Point,
    pub direction: MirDirection,
    pub r#type: u8,
}

impl PacketCodec for ObjectDied {
    const ID: i16 = ServerPacketId::ObjectDied as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectDied {
            object_id: r.read_u32()?,
            location: Point::read(r)?,
            direction: MirDirection::from_u8(r.read_u8()?),
            r#type: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        self.location.write(w);
        w.write_u8(self.direction.to_u8());
        w.write_u8(self.r#type);
    }
}

// ----------------------------- ID 82: ColourChanged -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ColourChanged {
    pub name_colour: Argb,
}

impl PacketCodec for ColourChanged {
    const ID: i16 = ServerPacketId::ColourChanged as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ColourChanged {
            name_colour: Argb::from_i32(r.read_i32()?),
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.name_colour.to_i32());
    }
}

// ----------------------------- ID 83: ObjectColourChanged -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectColourChanged {
    pub object_id: u32,
    pub name_colour: Argb,
}

impl PacketCodec for ObjectColourChanged {
    const ID: i16 = ServerPacketId::ObjectColourChanged as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectColourChanged {
            object_id: r.read_u32()?,
            name_colour: Argb::from_i32(r.read_i32()?),
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_i32(self.name_colour.to_i32());
    }
}

// ----------------------------- ID 84: ObjectGuildNameChanged -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectGuildNameChanged {
    pub object_id: u32,
    pub guild_name: String,
}

impl PacketCodec for ObjectGuildNameChanged {
    const ID: i16 = ServerPacketId::ObjectGuildNameChanged as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectGuildNameChanged {
            object_id: r.read_u32()?,
            guild_name: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_string(&self.guild_name);
    }
}

// ----------------------------- ID 85: GainExperience -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GainExperience {
    pub amount: u32,
}

impl PacketCodec for GainExperience {
    const ID: i16 = ServerPacketId::GainExperience as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(GainExperience {
            amount: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.amount);
    }
}

// ----------------------------- ID 86: GainHeroExperience -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GainHeroExperience {
    pub amount: u32,
}

impl PacketCodec for GainHeroExperience {
    const ID: i16 = ServerPacketId::GainHeroExperience as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(GainHeroExperience {
            amount: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.amount);
    }
}

// ----------------------------- ID 87: LevelChanged -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LevelChanged {
    pub level: u16,
    pub experience: i64,
    pub max_experience: i64,
}

impl PacketCodec for LevelChanged {
    const ID: i16 = ServerPacketId::LevelChanged as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(LevelChanged {
            level: r.read_u16()?,
            experience: r.read_i64()?,
            max_experience: r.read_i64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u16(self.level);
        w.write_i64(self.experience);
        w.write_i64(self.max_experience);
    }
}

// ----------------------------- ID 88: HeroLevelChanged -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HeroLevelChanged {
    pub level: u16,
    pub experience: i64,
    pub max_experience: i64,
}

impl PacketCodec for HeroLevelChanged {
    const ID: i16 = ServerPacketId::HeroLevelChanged as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(HeroLevelChanged {
            level: r.read_u16()?,
            experience: r.read_i64()?,
            max_experience: r.read_i64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u16(self.level);
        w.write_i64(self.experience);
        w.write_i64(self.max_experience);
    }
}

// ----------------------------- ID 89: ObjectLeveled -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectLeveled {
    pub object_id: u32,
}

impl PacketCodec for ObjectLeveled {
    const ID: i16 = ServerPacketId::ObjectLeveled as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectLeveled {
            object_id: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
    }
}

// ----------------------------- ID 90: ObjectHarvest -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectHarvest {
    pub object_id: u32,
    pub location: Point,
    pub direction: MirDirection,
}

impl PacketCodec for ObjectHarvest {
    const ID: i16 = ServerPacketId::ObjectHarvest as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectHarvest {
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

// ----------------------------- ID 91: ObjectHarvested -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectHarvested {
    pub object_id: u32,
    pub location: Point,
    pub direction: MirDirection,
}

impl PacketCodec for ObjectHarvested {
    const ID: i16 = ServerPacketId::ObjectHarvested as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectHarvested {
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

// ----------------------------- ID 92: ObjectNPC -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectNPC {
    pub object_id: u32,
    pub name: String,
    pub name_colour: Argb,
    pub image: u16,
    pub colour: Argb,
    pub location: Point,
    pub direction: MirDirection,
    pub quest_ids: Vec<i32>,
}

impl PacketCodec for ObjectNPC {
    const ID: i16 = ServerPacketId::ObjectNpc as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let object_id = r.read_u32()?;
        let name = r.read_string()?;
        let name_colour = Argb::from_i32(r.read_i32()?);
        let image = r.read_u16()?;
        let colour = Argb::from_i32(r.read_i32()?);
        let location = Point::read(r)?;
        let direction = MirDirection::from_u8(r.read_u8()?);
        let mut quest_ids = Vec::new();
        let qcount = r.read_i32()?;
        for _ in 0..qcount.max(0) {
            quest_ids.push(r.read_i32()?);
        }
        Ok(ObjectNPC {
            object_id,
            name,
            name_colour,
            image,
            colour,
            location,
            direction,
            quest_ids,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_string(&self.name);
        w.write_i32(self.name_colour.to_i32());
        w.write_u16(self.image);
        w.write_i32(self.colour.to_i32());
        self.location.write(w);
        w.write_u8(self.direction.to_u8());
        w.write_i32(self.quest_ids.len() as i32);
        for q in &self.quest_ids {
            w.write_i32(*q);
        }
    }
}

// ----------------------------- ID 93: NPCResponse -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NPCResponse {
    pub page: Vec<String>,
}

impl PacketCodec for NPCResponse {
    const ID: i16 = ServerPacketId::NPCResponse as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let mut page = Vec::new();
        let count = r.read_i32()?;
        for _ in 0..count.max(0) {
            page.push(r.read_string()?);
        }
        Ok(NPCResponse { page })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.page.len() as i32);
        for s in &self.page {
            w.write_string(s);
        }
    }
}

// ----------------------------- ID 94: ObjectHide -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectHide {
    pub object_id: u32,
}

impl PacketCodec for ObjectHide {
    const ID: i16 = ServerPacketId::ObjectHide as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectHide {
            object_id: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
    }
}

// ----------------------------- ID 95: ObjectShow -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectShow {
    pub object_id: u32,
}

impl PacketCodec for ObjectShow {
    const ID: i16 = ServerPacketId::ObjectShow as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectShow {
            object_id: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
    }
}

// ----------------------------- ID 96: Poisoned -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Poisoned {
    /// PoisonType (u16) —— C# 枚举（位标志）
    pub poison: u16,
}

impl PacketCodec for Poisoned {
    const ID: i16 = ServerPacketId::Poisoned as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Poisoned {
            poison: r.read_u16()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u16(self.poison);
    }
}

// ----------------------------- ID 97: ObjectPoisoned -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectPoisoned {
    pub object_id: u32,
    /// PoisonType (u16) —— C# 枚举（位标志）
    pub poison: u16,
}

impl PacketCodec for ObjectPoisoned {
    const ID: i16 = ServerPacketId::ObjectPoisoned as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectPoisoned {
            object_id: r.read_u32()?,
            poison: r.read_u16()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_u16(self.poison);
    }
}

// ----------------------------- ID 98: MapChanged -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MapChanged {
    pub map_index: i32,
    pub file_name: String,
    pub title: String,
    pub mini_map: u16,
    pub big_map: u16,
    /// LightSetting (u8) —— C# 枚举
    pub lights: u8,
    pub location: Point,
    pub direction: MirDirection,
    pub map_dark_light: u8,
    pub music: u16,
    /// WeatherSetting (u16) —— C# 枚举
    pub weather: u16,
}

impl PacketCodec for MapChanged {
    const ID: i16 = ServerPacketId::MapChanged as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(MapChanged {
            map_index: r.read_i32()?,
            file_name: r.read_string()?,
            title: r.read_string()?,
            mini_map: r.read_u16()?,
            big_map: r.read_u16()?,
            lights: r.read_u8()?,
            location: Point::read(r)?,
            direction: MirDirection::from_u8(r.read_u8()?),
            map_dark_light: r.read_u8()?,
            music: r.read_u16()?,
            weather: r.read_u16()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.map_index);
        w.write_string(&self.file_name);
        w.write_string(&self.title);
        w.write_u16(self.mini_map);
        w.write_u16(self.big_map);
        w.write_u8(self.lights);
        self.location.write(w);
        w.write_u8(self.direction.to_u8());
        w.write_u8(self.map_dark_light);
        w.write_u16(self.music);
        w.write_u16(self.weather);
    }
}

// ----------------------------- ID 99: ObjectTeleportOut -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectTeleportOut {
    pub object_id: u32,
    pub r#type: u8,
}

impl PacketCodec for ObjectTeleportOut {
    const ID: i16 = ServerPacketId::ObjectTeleportOut as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectTeleportOut {
            object_id: r.read_u32()?,
            r#type: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_u8(self.r#type);
    }
}

// ----------------------------- ID 100: ObjectTeleportIn -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectTeleportIn {
    pub object_id: u32,
    pub r#type: u8,
}

impl PacketCodec for ObjectTeleportIn {
    const ID: i16 = ServerPacketId::ObjectTeleportIn as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectTeleportIn {
            object_id: r.read_u32()?,
            r#type: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_u8(self.r#type);
    }
}

// ----------------------------- ID 101: TeleportIn -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TeleportIn;

impl PacketCodec for TeleportIn {
    const ID: i16 = ServerPacketId::TeleportIn as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(TeleportIn)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 102: NPCGoods -----------------------------
// 见文件头说明: 已由 batch_3.rs 移植（Compressed + f32 Rate + PanelType），
// 此处不重复定义以免与 `pub use batch_3::*` 重导出冲突。
