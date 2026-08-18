// batch_3 —— 服务器→客户端包（Shared/ServerPackets.cs 行 3074–4009）
// 对应枚举 ID 102–145（另有 GroupMembersMap=274 / SendMemberLocation=275）。
// 含 NPCGoods（唯一 `Compressed => true` 的包）、NPC 商店/修理/精炼、物品、
// 魔法、队伍、复活、BUFF 等。

use super::*;
use crate::binary::{Point, Reader, Writer};
use crate::frame::PacketCodec;
use crate::ids::ServerPacketId;
use crate::types::*;
use crate::Result;

// ----------------------------- ID 102: NPCGoods（Compressed） -----------------------------

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NPCGoods {
    /// C# `List<UserItem> List` —— count + 逐项 UserItem
    pub list: Vec<UserItem>,
    pub rate: f32,
    /// PanelType (u8)
    pub r#type: u8,
    pub hide_added_stats: bool,
}

impl PacketCodec for NPCGoods {
    const ID: i16 = ServerPacketId::NPCGoods as i16;
    /// C# `public override bool Compressed => true;` —— 载荷是 gzip 流
    const COMPRESSED: bool = true;

    fn read(r: &mut Reader) -> Result<Self> {
        let mut list = Vec::new();
        let count = r.read_i32()?;
        for _ in 0..count.max(0) {
            list.push(UserItem::read(r)?);
        }
        Ok(NPCGoods {
            list,
            rate: r.read_f32()?,
            r#type: r.read_u8()?,
            hide_added_stats: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.list.len() as i32);
        for item in &self.list {
            item.write(w);
        }
        w.write_f32(self.rate);
        w.write_u8(self.r#type);
        w.write_bool(self.hide_added_stats);
    }
}

// ----------------------------- ID 103: NPCSell -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NPCSell;

impl PacketCodec for NPCSell {
    const ID: i16 = ServerPacketId::NPCSell as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(NPCSell)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 104: NPCRepair -----------------------------

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NPCRepair {
    pub rate: f32,
}

impl PacketCodec for NPCRepair {
    const ID: i16 = ServerPacketId::NPCRepair as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(NPCRepair {
            rate: r.read_f32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_f32(self.rate);
    }
}

// ----------------------------- ID 105: NPCSRepair -----------------------------

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NPCSRepair {
    pub rate: f32,
}

impl PacketCodec for NPCSRepair {
    const ID: i16 = ServerPacketId::NPCSRepair as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(NPCSRepair {
            rate: r.read_f32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_f32(self.rate);
    }
}

// ----------------------------- ID 106: NPCRefine -----------------------------

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NPCRefine {
    pub rate: f32,
    pub refining: bool,
}

impl PacketCodec for NPCRefine {
    const ID: i16 = ServerPacketId::NPCRefine as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(NPCRefine {
            rate: r.read_f32()?,
            refining: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_f32(self.rate);
        w.write_bool(self.refining);
    }
}

// ----------------------------- ID 107: NPCCheckRefine -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NPCCheckRefine;

impl PacketCodec for NPCCheckRefine {
    const ID: i16 = ServerPacketId::NPCCheckRefine as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(NPCCheckRefine)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 108: NPCCollectRefine -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NPCCollectRefine {
    pub success: bool,
}

impl PacketCodec for NPCCollectRefine {
    const ID: i16 = ServerPacketId::NPCCollectRefine as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(NPCCollectRefine {
            success: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_bool(self.success);
    }
}

// ----------------------------- ID 109: NPCReplaceWedRing -----------------------------

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NPCReplaceWedRing {
    pub rate: f32,
}

impl PacketCodec for NPCReplaceWedRing {
    const ID: i16 = ServerPacketId::NPCReplaceWedRing as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(NPCReplaceWedRing {
            rate: r.read_f32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_f32(self.rate);
    }
}

// ----------------------------- ID 110: NPCStorage -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NPCStorage;

impl PacketCodec for NPCStorage {
    const ID: i16 = ServerPacketId::NPCStorage as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(NPCStorage)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 111: SellItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SellItem {
    pub unique_id: u64,
    pub count: u16,
    pub success: bool,
}

impl PacketCodec for SellItem {
    const ID: i16 = ServerPacketId::SellItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(SellItem {
            unique_id: r.read_u64()?,
            count: r.read_u16()?,
            success: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
        w.write_u16(self.count);
        w.write_bool(self.success);
    }
}

// ----------------------------- ID 113: RepairItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RepairItem {
    pub unique_id: u64,
}

impl PacketCodec for RepairItem {
    const ID: i16 = ServerPacketId::RepairItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RepairItem {
            unique_id: r.read_u64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
    }
}

// ----------------------------- ID 114: ItemRepaired -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemRepaired {
    pub unique_id: u64,
    pub max_dura: u16,
    pub current_dura: u16,
}

impl PacketCodec for ItemRepaired {
    const ID: i16 = ServerPacketId::ItemRepaired as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ItemRepaired {
            unique_id: r.read_u64()?,
            max_dura: r.read_u16()?,
            current_dura: r.read_u16()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
        w.write_u16(self.max_dura);
        w.write_u16(self.current_dura);
    }
}

// ----------------------------- ID 115: ItemSlotSizeChanged -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemSlotSizeChanged {
    pub unique_id: u64,
    pub slot_size: i32,
}

impl PacketCodec for ItemSlotSizeChanged {
    const ID: i16 = ServerPacketId::ItemSlotSizeChanged as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ItemSlotSizeChanged {
            unique_id: r.read_u64()?,
            slot_size: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
        w.write_i32(self.slot_size);
    }
}

// ----------------------------- ID 116: ItemSealChanged -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemSealChanged {
    pub unique_id: u64,
    /// DateTime.ToBinary()（.NET ticks）
    pub expiry_date: i64,
}

impl PacketCodec for ItemSealChanged {
    const ID: i16 = ServerPacketId::ItemSealChanged as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ItemSealChanged {
            unique_id: r.read_u64()?,
            expiry_date: r.read_i64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
        w.write_i64(self.expiry_date);
    }
}

// ----------------------------- ID 117: NewMagic -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewMagic {
    pub magic: ClientMagic,
    pub hero: bool,
}

impl PacketCodec for NewMagic {
    const ID: i16 = ServerPacketId::NewMagic as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(NewMagic {
            magic: ClientMagic::read(r)?,
            hero: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        self.magic.write(w);
        w.write_bool(self.hero);
    }
}

// ----------------------------- ID 118: RemoveMagic -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RemoveMagic {
    pub place_id: i32,
}

impl PacketCodec for RemoveMagic {
    const ID: i16 = ServerPacketId::RemoveMagic as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RemoveMagic {
            place_id: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.place_id);
    }
}

// ----------------------------- ID 119: MagicLeveled -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MagicLeveled {
    pub object_id: u32,
    /// Spell (u8)
    pub spell: u8,
    pub level: u8,
    pub experience: u16,
}

impl PacketCodec for MagicLeveled {
    const ID: i16 = ServerPacketId::MagicLeveled as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(MagicLeveled {
            object_id: r.read_u32()?,
            spell: r.read_u8()?,
            level: r.read_u8()?,
            experience: r.read_u16()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_u8(self.spell);
        w.write_u8(self.level);
        w.write_u16(self.experience);
    }
}

// ----------------------------- ID 120: Magic -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Magic {
    /// Spell (u8)
    pub spell: u8,
    pub target_id: u32,
    pub target: Point,
    pub cast: bool,
    pub level: u8,
    pub secondary_target_ids: Vec<u32>,
}

impl PacketCodec for Magic {
    const ID: i16 = ServerPacketId::Magic as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let spell = r.read_u8()?;
        let target_id = r.read_u32()?;
        let target = Point::read(r)?;
        let cast = r.read_bool()?;
        let level = r.read_u8()?;
        let mut secondary_target_ids = Vec::new();
        let count = r.read_i32()?;
        for _ in 0..count.max(0) {
            secondary_target_ids.push(r.read_u32()?);
        }
        Ok(Magic {
            spell,
            target_id,
            target,
            cast,
            level,
            secondary_target_ids,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.spell);
        w.write_u32(self.target_id);
        self.target.write(w);
        w.write_bool(self.cast);
        w.write_u8(self.level);
        w.write_i32(self.secondary_target_ids.len() as i32);
        for id in &self.secondary_target_ids {
            w.write_u32(*id);
        }
    }
}

// ----------------------------- ID 121: MagicDelay -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MagicDelay {
    pub object_id: u32,
    /// Spell (u8)
    pub spell: u8,
    pub delay: i64,
}

impl PacketCodec for MagicDelay {
    const ID: i16 = ServerPacketId::MagicDelay as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(MagicDelay {
            object_id: r.read_u32()?,
            spell: r.read_u8()?,
            delay: r.read_i64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_u8(self.spell);
        w.write_i64(self.delay);
    }
}

// ----------------------------- ID 122: MagicCast -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MagicCast {
    /// Spell (u8)
    pub spell: u8,
}

impl PacketCodec for MagicCast {
    const ID: i16 = ServerPacketId::MagicCast as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(MagicCast {
            spell: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.spell);
    }
}

// ----------------------------- ID 123: ObjectMagic -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectMagic {
    pub object_id: u32,
    pub location: Point,
    pub direction: MirDirection,
    /// Spell (u8)
    pub spell: u8,
    pub target_id: u32,
    pub target: Point,
    pub cast: bool,
    pub level: u8,
    pub self_broadcast: bool,
    pub secondary_target_ids: Vec<u32>,
}

impl PacketCodec for ObjectMagic {
    const ID: i16 = ServerPacketId::ObjectMagic as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let object_id = r.read_u32()?;
        let location = Point::read(r)?;
        let direction = MirDirection::from_u8(r.read_u8()?);
        let spell = r.read_u8()?;
        let target_id = r.read_u32()?;
        let target = Point::read(r)?;
        let cast = r.read_bool()?;
        let level = r.read_u8()?;
        let self_broadcast = r.read_bool()?;
        let mut secondary_target_ids = Vec::new();
        let count = r.read_i32()?;
        for _ in 0..count.max(0) {
            secondary_target_ids.push(r.read_u32()?);
        }
        Ok(ObjectMagic {
            object_id,
            location,
            direction,
            spell,
            target_id,
            target,
            cast,
            level,
            self_broadcast,
            secondary_target_ids,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        self.location.write(w);
        w.write_u8(self.direction.to_u8());
        w.write_u8(self.spell);
        w.write_u32(self.target_id);
        self.target.write(w);
        w.write_bool(self.cast);
        w.write_u8(self.level);
        w.write_bool(self.self_broadcast);
        w.write_i32(self.secondary_target_ids.len() as i32);
        for id in &self.secondary_target_ids {
            w.write_u32(*id);
        }
    }
}

// ----------------------------- ID 124: ObjectEffect -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectEffect {
    pub object_id: u32,
    /// SpellEffect (u8)
    pub effect: u8,
    pub effect_type: u32,
    pub delay_time: u32,
    pub time: u32,
}

impl PacketCodec for ObjectEffect {
    const ID: i16 = ServerPacketId::ObjectEffect as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectEffect {
            object_id: r.read_u32()?,
            effect: r.read_u8()?,
            effect_type: r.read_u32()?,
            delay_time: r.read_u32()?,
            time: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_u8(self.effect);
        w.write_u32(self.effect_type);
        w.write_u32(self.delay_time);
        w.write_u32(self.time);
    }
}

// ----------------------------- ID 125: ObjectProjectile -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectProjectile {
    /// Spell (u8)
    pub spell: u8,
    pub source: u32,
    pub destination: u32,
}

impl PacketCodec for ObjectProjectile {
    const ID: i16 = ServerPacketId::ObjectProjectile as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectProjectile {
            spell: r.read_u8()?,
            source: r.read_u32()?,
            destination: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.spell);
        w.write_u32(self.source);
        w.write_u32(self.destination);
    }
}

// ----------------------------- ID 126: RangeAttack -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RangeAttack {
    pub target_id: u32,
    pub target: Point,
    /// Spell (u8)
    pub spell: u8,
}

impl PacketCodec for RangeAttack {
    const ID: i16 = ServerPacketId::RangeAttack as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RangeAttack {
            target_id: r.read_u32()?,
            target: Point::read(r)?,
            spell: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.target_id);
        self.target.write(w);
        w.write_u8(self.spell);
    }
}

// ----------------------------- ID 127: Pushed -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pushed {
    pub location: Point,
    pub direction: MirDirection,
}

impl PacketCodec for Pushed {
    const ID: i16 = ServerPacketId::Pushed as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Pushed {
            location: Point::read(r)?,
            direction: MirDirection::from_u8(r.read_u8()?),
        })
    }

    fn write(&self, w: &mut Writer) {
        self.location.write(w);
        w.write_u8(self.direction.to_u8());
    }
}

// ----------------------------- ID 128: ObjectPushed -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectPushed {
    pub object_id: u32,
    pub location: Point,
    pub direction: MirDirection,
}

impl PacketCodec for ObjectPushed {
    const ID: i16 = ServerPacketId::ObjectPushed as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectPushed {
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

// ----------------------------- ID 129: ObjectName -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectName {
    pub object_id: u32,
    pub name: String,
}

impl PacketCodec for ObjectName {
    const ID: i16 = ServerPacketId::ObjectName as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectName {
            object_id: r.read_u32()?,
            name: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_string(&self.name);
    }
}

// ----------------------------- ID 130: UserStorage -----------------------------

/// 仓库物品数组（`UserItem[] Storage`，可整体为 null）。
///
/// 注意: 与 `read_item_slots` 的布尔方向相反 —— C# 这里写/读的是
/// `Storage[i] != null`（true=有物品），而 `read_item_slots` 写 true=空槽，
/// 因此不能复用该 helper，按原码逐字节实现。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserStorage {
    pub storage: Option<ItemSlots>,
}

impl PacketCodec for UserStorage {
    const ID: i16 = ServerPacketId::UserStorage as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        if !r.read_bool()? {
            return Ok(UserStorage { storage: None });
        }
        let len = r.read_i32()?;
        let mut storage = Vec::with_capacity(len.max(0) as usize);
        for _ in 0..len.max(0) {
            if r.read_bool()? {
                storage.push(Some(UserItem::read(r)?));
            } else {
                storage.push(None);
            }
        }
        Ok(UserStorage {
            storage: Some(storage),
        })
    }

    fn write(&self, w: &mut Writer) {
        match &self.storage {
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
}

// ----------------------------- ID 131: SwitchGroup -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SwitchGroup {
    pub allow_group: bool,
}

impl PacketCodec for SwitchGroup {
    const ID: i16 = ServerPacketId::SwitchGroup as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(SwitchGroup {
            allow_group: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_bool(self.allow_group);
    }
}

// ----------------------------- ID 132: DeleteGroup -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeleteGroup;

impl PacketCodec for DeleteGroup {
    const ID: i16 = ServerPacketId::DeleteGroup as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(DeleteGroup)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 133: DeleteMember -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeleteMember {
    pub name: String,
}

impl PacketCodec for DeleteMember {
    const ID: i16 = ServerPacketId::DeleteMember as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(DeleteMember {
            name: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.name);
    }
}

// ----------------------------- ID 134: GroupInvite -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GroupInvite {
    pub name: String,
}

impl PacketCodec for GroupInvite {
    const ID: i16 = ServerPacketId::GroupInvite as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(GroupInvite {
            name: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.name);
    }
}

// ----------------------------- ID 135: AddMember -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AddMember {
    pub name: String,
}

impl PacketCodec for AddMember {
    const ID: i16 = ServerPacketId::AddMember as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(AddMember {
            name: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.name);
    }
}

// ----------------------------- ID 136: Revived -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Revived;

impl PacketCodec for Revived {
    const ID: i16 = ServerPacketId::Revived as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(Revived)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 137: ObjectRevived -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectRevived {
    pub object_id: u32,
    pub effect: bool,
}

impl PacketCodec for ObjectRevived {
    const ID: i16 = ServerPacketId::ObjectRevived as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectRevived {
            object_id: r.read_u32()?,
            effect: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_bool(self.effect);
    }
}

// ----------------------------- ID 138: SpellToggle -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellToggle {
    pub object_id: u32,
    /// Spell (u8)
    pub spell: u8,
    pub can_use: bool,
}

impl PacketCodec for SpellToggle {
    const ID: i16 = ServerPacketId::SpellToggle as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(SpellToggle {
            object_id: r.read_u32()?,
            spell: r.read_u8()?,
            can_use: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_u8(self.spell);
        w.write_bool(self.can_use);
    }
}

// ----------------------------- ID 139: ObjectHealth -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectHealth {
    pub object_id: u32,
    pub percent: u8,
    pub expire: u8,
}

impl PacketCodec for ObjectHealth {
    const ID: i16 = ServerPacketId::ObjectHealth as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectHealth {
            object_id: r.read_u32()?,
            percent: r.read_u8()?,
            expire: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_u8(self.percent);
        w.write_u8(self.expire);
    }
}

// ----------------------------- ID 140: ObjectMana -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectMana {
    pub object_id: u32,
    pub percent: u8,
}

impl PacketCodec for ObjectMana {
    const ID: i16 = ServerPacketId::ObjectMana as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectMana {
            object_id: r.read_u32()?,
            percent: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_u8(self.percent);
    }
}

// ----------------------------- ID 141: MapEffect -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MapEffect {
    pub location: Point,
    /// SpellEffect (u8)
    pub effect: u8,
    pub value: u8,
}

impl PacketCodec for MapEffect {
    const ID: i16 = ServerPacketId::MapEffect as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(MapEffect {
            location: Point::read(r)?,
            effect: r.read_u8()?,
            value: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        self.location.write(w);
        w.write_u8(self.effect);
        w.write_u8(self.value);
    }
}

// ----------------------------- ID 142: AllowObserve -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AllowObserve {
    pub allow: bool,
}

impl PacketCodec for AllowObserve {
    const ID: i16 = ServerPacketId::AllowObserve as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(AllowObserve {
            allow: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_bool(self.allow);
    }
}

// ----------------------------- ID 143: ObjectRangeAttack -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectRangeAttack {
    pub object_id: u32,
    pub location: Point,
    pub direction: MirDirection,
    pub target_id: u32,
    pub target: Point,
    pub r#type: u8,
    /// Spell (u8)
    pub spell: u8,
    pub level: u8,
}

impl PacketCodec for ObjectRangeAttack {
    const ID: i16 = ServerPacketId::ObjectRangeAttack as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectRangeAttack {
            object_id: r.read_u32()?,
            location: Point::read(r)?,
            direction: MirDirection::from_u8(r.read_u8()?),
            target_id: r.read_u32()?,
            target: Point::read(r)?,
            r#type: r.read_u8()?,
            spell: r.read_u8()?,
            level: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        self.location.write(w);
        w.write_u8(self.direction.to_u8());
        w.write_u32(self.target_id);
        self.target.write(w);
        w.write_u8(self.r#type);
        w.write_u8(self.spell);
        w.write_u8(self.level);
    }
}

// ----------------------------- ID 144: AddBuff -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AddBuff {
    pub buff: ClientBuff,
}

impl PacketCodec for AddBuff {
    const ID: i16 = ServerPacketId::AddBuff as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(AddBuff {
            buff: ClientBuff::read(r)?,
        })
    }

    fn write(&self, w: &mut Writer) {
        self.buff.write(w);
    }
}

// ----------------------------- ID 145: RemoveBuff -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RemoveBuff {
    /// BuffType (u8)
    pub r#type: u8,
    pub object_id: u32,
}

impl PacketCodec for RemoveBuff {
    const ID: i16 = ServerPacketId::RemoveBuff as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RemoveBuff {
            r#type: r.read_u8()?,
            object_id: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.r#type);
        w.write_u32(self.object_id);
    }
}

// ----------------------------- ID 274: GroupMembersMap -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GroupMembersMap {
    pub player_name: String,
    pub player_map: String,
}

impl PacketCodec for GroupMembersMap {
    const ID: i16 = ServerPacketId::GroupMembersMap as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(GroupMembersMap {
            player_name: r.read_string()?,
            player_map: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.player_name);
        w.write_string(&self.player_map);
    }
}

// ----------------------------- ID 275: SendMemberLocation -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SendMemberLocation {
    pub member_name: String,
    pub member_location: Point,
}

impl PacketCodec for SendMemberLocation {
    const ID: i16 = ServerPacketId::SendMemberLocation as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(SendMemberLocation {
            member_name: r.read_string()?,
            member_location: Point::read(r)?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.member_name);
        self.member_location.write(w);
    }
}
