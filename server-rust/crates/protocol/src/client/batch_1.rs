//! 客户端→服务器数据包 batch 1（对应 `Shared/ClientPackets.cs` 322–996 行，
//! 清单见 `docs/batches/CB1.txt`）: 物品操作/交易/战斗/商店等包。
//!
//! 字段顺序与 C# `ReadPacket`/`WritePacket` 完全一致；枚举按指令存原始整数
//! （u8/u16/i16），注释注明 C# 枚举名。

use crate::binary::{Point, Reader, Writer};
use crate::frame::PacketCodec;
use crate::ids::ClientPacketId;
use crate::Result;

// ----------------------------- ID 14: MoveItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MoveItem {
    /// MirGridType (u8)
    pub grid: u8,
    pub from: i32,
    pub to: i32,
}

impl PacketCodec for MoveItem {
    const ID: i16 = ClientPacketId::MoveItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(MoveItem {
            grid: r.read_u8()?,
            from: r.read_i32()?,
            to: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.grid);
        w.write_i32(self.from);
        w.write_i32(self.to);
    }
}

// ----------------------------- ID 15: StoreItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoreItem {
    pub from: i32,
    pub to: i32,
}

impl PacketCodec for StoreItem {
    const ID: i16 = ClientPacketId::StoreItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(StoreItem {
            from: r.read_i32()?,
            to: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.from);
        w.write_i32(self.to);
    }
}

// ----------------------------- ID 24: DepositRefineItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DepositRefineItem {
    pub from: i32,
    pub to: i32,
}

impl PacketCodec for DepositRefineItem {
    const ID: i16 = ClientPacketId::DepositRefineItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(DepositRefineItem {
            from: r.read_i32()?,
            to: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.from);
        w.write_i32(self.to);
    }
}

// ----------------------------- ID 25: RetrieveRefineItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RetrieveRefineItem {
    pub from: i32,
    pub to: i32,
}

impl PacketCodec for RetrieveRefineItem {
    const ID: i16 = ClientPacketId::RetrieveRefineItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RetrieveRefineItem {
            from: r.read_i32()?,
            to: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.from);
        w.write_i32(self.to);
    }
}

// ----------------------------- ID 26: RefineCancel -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RefineCancel;

impl PacketCodec for RefineCancel {
    const ID: i16 = ClientPacketId::RefineCancel as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(RefineCancel)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 27: RefineItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RefineItem {
    pub unique_id: u64,
}

impl PacketCodec for RefineItem {
    const ID: i16 = ClientPacketId::RefineItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RefineItem {
            unique_id: r.read_u64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
    }
}

// ----------------------------- ID 28: CheckRefine -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CheckRefine {
    pub unique_id: u64,
}

impl PacketCodec for CheckRefine {
    const ID: i16 = ClientPacketId::CheckRefine as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(CheckRefine {
            unique_id: r.read_u64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
    }
}

// ----------------------------- ID 29: ReplaceWedRing -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReplaceWedRing {
    pub unique_id: u64,
}

impl PacketCodec for ReplaceWedRing {
    const ID: i16 = ClientPacketId::ReplaceWedRing as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ReplaceWedRing {
            unique_id: r.read_u64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
    }
}

// ----------------------------- ID 30: DepositTradeItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DepositTradeItem {
    pub from: i32,
    pub to: i32,
}

impl PacketCodec for DepositTradeItem {
    const ID: i16 = ClientPacketId::DepositTradeItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(DepositTradeItem {
            from: r.read_i32()?,
            to: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.from);
        w.write_i32(self.to);
    }
}

// ----------------------------- ID 31: RetrieveTradeItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RetrieveTradeItem {
    pub from: i32,
    pub to: i32,
}

impl PacketCodec for RetrieveTradeItem {
    const ID: i16 = ClientPacketId::RetrieveTradeItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RetrieveTradeItem {
            from: r.read_i32()?,
            to: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.from);
        w.write_i32(self.to);
    }
}

// ----------------------------- ID 16: TakeBackItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TakeBackItem {
    pub from: i32,
    pub to: i32,
}

impl PacketCodec for TakeBackItem {
    const ID: i16 = ClientPacketId::TakeBackItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(TakeBackItem {
            from: r.read_i32()?,
            to: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.from);
        w.write_i32(self.to);
    }
}

// ----------------------------- ID 17: MergeItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MergeItem {
    /// MirGridType (u8)
    pub grid_from: u8,
    /// MirGridType (u8)
    pub grid_to: u8,
    pub id_from: u64,
    pub id_to: u64,
}

impl PacketCodec for MergeItem {
    const ID: i16 = ClientPacketId::MergeItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(MergeItem {
            grid_from: r.read_u8()?,
            grid_to: r.read_u8()?,
            id_from: r.read_u64()?,
            id_to: r.read_u64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.grid_from);
        w.write_u8(self.grid_to);
        w.write_u64(self.id_from);
        w.write_u64(self.id_to);
    }
}

// ----------------------------- ID 18: EquipItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EquipItem {
    /// MirGridType (u8)
    pub grid: u8,
    pub unique_id: u64,
    pub to: i32,
}

impl PacketCodec for EquipItem {
    const ID: i16 = ClientPacketId::EquipItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(EquipItem {
            grid: r.read_u8()?,
            unique_id: r.read_u64()?,
            to: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.grid);
        w.write_u64(self.unique_id);
        w.write_i32(self.to);
    }
}

// ----------------------------- ID 19: RemoveItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RemoveItem {
    /// MirGridType (u8)
    pub grid: u8,
    pub unique_id: u64,
    pub to: i32,
}

impl PacketCodec for RemoveItem {
    const ID: i16 = ClientPacketId::RemoveItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RemoveItem {
            grid: r.read_u8()?,
            unique_id: r.read_u64()?,
            to: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.grid);
        w.write_u64(self.unique_id);
        w.write_i32(self.to);
    }
}

// ----------------------------- ID 20: RemoveSlotItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RemoveSlotItem {
    /// MirGridType (u8)
    pub grid: u8,
    /// MirGridType (u8)
    pub grid_to: u8,
    pub unique_id: u64,
    pub to: i32,
    pub from_unique_id: u64,
}

impl PacketCodec for RemoveSlotItem {
    const ID: i16 = ClientPacketId::RemoveSlotItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RemoveSlotItem {
            grid: r.read_u8()?,
            grid_to: r.read_u8()?,
            unique_id: r.read_u64()?,
            to: r.read_i32()?,
            from_unique_id: r.read_u64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.grid);
        w.write_u8(self.grid_to);
        w.write_u64(self.unique_id);
        w.write_i32(self.to);
        w.write_u64(self.from_unique_id);
    }
}

// ----------------------------- ID 21: SplitItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SplitItem {
    /// MirGridType (u8)
    pub grid: u8,
    pub unique_id: u64,
    pub count: u16,
}

impl PacketCodec for SplitItem {
    const ID: i16 = ClientPacketId::SplitItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(SplitItem {
            grid: r.read_u8()?,
            unique_id: r.read_u64()?,
            count: r.read_u16()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.grid);
        w.write_u64(self.unique_id);
        w.write_u16(self.count);
    }
}

// ----------------------------- ID 22: UseItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UseItem {
    pub unique_id: u64,
    /// MirGridType (u8)
    pub grid: u8,
}

impl PacketCodec for UseItem {
    const ID: i16 = ClientPacketId::UseItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(UseItem {
            unique_id: r.read_u64()?,
            grid: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
        w.write_u8(self.grid);
    }
}

// ----------------------------- ID 23: DropItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DropItem {
    pub unique_id: u64,
    pub count: u16,
    pub hero_inventory: bool,
}

impl PacketCodec for DropItem {
    const ID: i16 = ClientPacketId::DropItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(DropItem {
            unique_id: r.read_u64()?,
            count: r.read_u16()?,
            hero_inventory: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
        w.write_u16(self.count);
        w.write_bool(self.hero_inventory);
    }
}

// ----------------------------- ID 32: TakeBackHeroItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TakeBackHeroItem {
    pub from: i32,
    pub to: i32,
}

impl PacketCodec for TakeBackHeroItem {
    const ID: i16 = ClientPacketId::TakeBackHeroItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(TakeBackHeroItem {
            from: r.read_i32()?,
            to: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.from);
        w.write_i32(self.to);
    }
}

// ----------------------------- ID 33: TransferHeroItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TransferHeroItem {
    pub from: i32,
    pub to: i32,
}

impl PacketCodec for TransferHeroItem {
    const ID: i16 = ClientPacketId::TransferHeroItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(TransferHeroItem {
            from: r.read_i32()?,
            to: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.from);
        w.write_i32(self.to);
    }
}

// ----------------------------- ID 34: DropGold -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DropGold {
    pub amount: u32,
}

impl PacketCodec for DropGold {
    const ID: i16 = ClientPacketId::DropGold as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(DropGold {
            amount: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.amount);
    }
}

// ----------------------------- ID 35: PickUp -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PickUp;

impl PacketCodec for PickUp {
    const ID: i16 = ClientPacketId::PickUp as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(PickUp)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 42: Inspect -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Inspect {
    pub object_id: u32,
    pub ranking: bool,
    pub hero: bool,
}

impl PacketCodec for Inspect {
    const ID: i16 = ClientPacketId::Inspect as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Inspect {
            object_id: r.read_u32()?,
            ranking: r.read_bool()?,
            hero: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_bool(self.ranking);
        w.write_bool(self.hero);
    }
}

// ----------------------------- ID 43: Observe -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Observe {
    pub name: String,
}

impl PacketCodec for Observe {
    const ID: i16 = ClientPacketId::Observe as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Observe {
            name: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.name);
    }
}

// ----------------------------- ID 44: ChangeAMode -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangeAMode {
    /// AttackMode (u8)
    pub mode: u8,
}

impl PacketCodec for ChangeAMode {
    const ID: i16 = ClientPacketId::ChangeAMode as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ChangeAMode { mode: r.read_u8()? })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.mode);
    }
}

// ----------------------------- ID 45: ChangePMode -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangePMode {
    /// PetMode (u8)
    pub mode: u8,
}

impl PacketCodec for ChangePMode {
    const ID: i16 = ClientPacketId::ChangePMode as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ChangePMode { mode: r.read_u8()? })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.mode);
    }
}

// ----------------------------- ID 46: ChangeTrade -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangeTrade {
    pub allow_trade: bool,
}

impl PacketCodec for ChangeTrade {
    const ID: i16 = ClientPacketId::ChangeTrade as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ChangeTrade {
            allow_trade: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_bool(self.allow_trade);
    }
}

// ----------------------------- ID 47: Attack -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Attack {
    /// MirDirection (u8)
    pub direction: u8,
    /// Spell (u8)
    pub spell: u8,
}

impl PacketCodec for Attack {
    const ID: i16 = ClientPacketId::Attack as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Attack {
            direction: r.read_u8()?,
            spell: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.direction);
        w.write_u8(self.spell);
    }
}

// ----------------------------- ID 48: RangeAttack -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RangeAttack {
    /// MirDirection (u8)
    pub direction: u8,
    pub location: Point,
    pub target_id: u32,
    pub target_location: Point,
}

impl PacketCodec for RangeAttack {
    const ID: i16 = ClientPacketId::RangeAttack as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RangeAttack {
            direction: r.read_u8()?,
            location: Point::read(r)?,
            target_id: r.read_u32()?,
            target_location: Point::read(r)?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.direction);
        self.location.write(w);
        w.write_u32(self.target_id);
        self.target_location.write(w);
    }
}

// ----------------------------- ID 49: Harvest -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Harvest {
    /// MirDirection (u8)
    pub direction: u8,
}

impl PacketCodec for Harvest {
    const ID: i16 = ClientPacketId::Harvest as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Harvest {
            direction: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.direction);
    }
}

// ----------------------------- ID 50: CallNPC -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CallNPC {
    pub object_id: u32,
    pub key: String,
}

impl PacketCodec for CallNPC {
    const ID: i16 = ClientPacketId::CallNPC as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(CallNPC {
            object_id: r.read_u32()?,
            key: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_string(&self.key);
    }
}

// ----------------------------- ID 51: BuyItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BuyItem {
    pub item_index: u64,
    pub count: u16,
    /// PanelType (u8)
    pub r#type: u8,
}

impl PacketCodec for BuyItem {
    const ID: i16 = ClientPacketId::BuyItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(BuyItem {
            item_index: r.read_u64()?,
            count: r.read_u16()?,
            r#type: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.item_index);
        w.write_u16(self.count);
        w.write_u8(self.r#type);
    }
}

// ----------------------------- ID 52: SellItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SellItem {
    pub unique_id: u64,
    pub count: u16,
}

impl PacketCodec for SellItem {
    const ID: i16 = ClientPacketId::SellItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(SellItem {
            unique_id: r.read_u64()?,
            count: r.read_u16()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
        w.write_u16(self.count);
    }
}

// ----------------------------- ID 53: CraftItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CraftItem {
    pub unique_id: u64,
    pub count: u16,
    /// int[]：先写元素数，再逐个 i32
    pub slots: Vec<i32>,
}

impl PacketCodec for CraftItem {
    const ID: i16 = ClientPacketId::CraftItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let unique_id = r.read_u64()?;
        let count = r.read_u16()?;
        let slot_count = r.read_i32()?;
        let mut slots = Vec::with_capacity(slot_count.max(0) as usize);
        for _ in 0..slot_count.max(0) {
            slots.push(r.read_i32()?);
        }
        Ok(CraftItem {
            unique_id,
            count,
            slots,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
        w.write_u16(self.count);
        w.write_i32(self.slots.len() as i32);
        for slot in &self.slots {
            w.write_i32(*slot);
        }
    }
}

// ----------------------------- ID 54: RepairItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RepairItem {
    pub unique_id: u64,
}

impl PacketCodec for RepairItem {
    const ID: i16 = ClientPacketId::RepairItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RepairItem {
            unique_id: r.read_u64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
    }
}

// ----------------------------- ID 55: BuyItemBack -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BuyItemBack {
    pub unique_id: u64,
    pub count: u16,
}

impl PacketCodec for BuyItemBack {
    const ID: i16 = ClientPacketId::BuyItemBack as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(BuyItemBack {
            unique_id: r.read_u64()?,
            count: r.read_u16()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
        w.write_u16(self.count);
    }
}

// ----------------------------- ID 56: SRepairItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SRepairItem {
    pub unique_id: u64,
}

impl PacketCodec for SRepairItem {
    const ID: i16 = ClientPacketId::SRepairItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(SRepairItem {
            unique_id: r.read_u64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
    }
}

// ----------------------------- ID 36: RequestMapInfo -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestMapInfo {
    pub map_index: i32,
}

impl PacketCodec for RequestMapInfo {
    const ID: i16 = ClientPacketId::RequestMapInfo as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RequestMapInfo {
            map_index: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.map_index);
    }
}
