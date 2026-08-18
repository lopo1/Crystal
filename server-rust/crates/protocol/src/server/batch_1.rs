//! batch_1 —— 服务器→客户端包（`Shared/ServerPackets.cs` 第 1219–2191 行）。
//!
//! 移植自 C# `NewHeroInfo` ... `GainedItem`（忠实复刻字段顺序，见 docs/PROTOCOL.md）。

use crate::binary::{Argb, Point, Reader, Writer};
use crate::frame::PacketCodec;
use crate::ids::ServerPacketId;
use crate::types::{ClientHeroInformation, SelectInfo, UserItem};
use crate::Result;

// ----------------------------- ID 35: NewHeroInfo -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewHeroInfo {
    pub info: ClientHeroInformation,
    pub storage_index: i32,
}

impl PacketCodec for NewHeroInfo {
    const ID: i16 = ServerPacketId::NewHeroInfo as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(NewHeroInfo {
            info: ClientHeroInformation::read(r)?,
            storage_index: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        self.info.write(w);
        w.write_i32(self.storage_index);
    }
}

// ----------------------------- ID 36: NewChatItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewChatItem {
    pub item: UserItem,
}

impl PacketCodec for NewChatItem {
    const ID: i16 = ServerPacketId::NewChatItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(NewChatItem {
            item: UserItem::read(r)?,
        })
    }

    fn write(&self, w: &mut Writer) {
        self.item.write(w);
    }
}

// ----------------------------- ID 37: MoveItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MoveItem {
    /// MirGridType (u8)
    pub grid: u8,
    pub from: i32,
    pub to: i32,
    pub success: bool,
}

impl PacketCodec for MoveItem {
    const ID: i16 = ServerPacketId::MoveItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(MoveItem {
            grid: r.read_u8()?,
            from: r.read_i32()?,
            to: r.read_i32()?,
            success: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.grid);
        w.write_i32(self.from);
        w.write_i32(self.to);
        w.write_bool(self.success);
    }
}

// ----------------------------- ID 38: EquipItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EquipItem {
    /// MirGridType (u8)
    pub grid: u8,
    pub unique_id: u64,
    pub to: i32,
    pub success: bool,
}

impl PacketCodec for EquipItem {
    const ID: i16 = ServerPacketId::EquipItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(EquipItem {
            grid: r.read_u8()?,
            unique_id: r.read_u64()?,
            to: r.read_i32()?,
            success: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.grid);
        w.write_u64(self.unique_id);
        w.write_i32(self.to);
        w.write_bool(self.success);
    }
}

// ----------------------------- ID 39: MergeItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MergeItem {
    /// MirGridType (u8)
    pub grid_from: u8,
    /// MirGridType (u8)
    pub grid_to: u8,
    pub id_from: u64,
    pub id_to: u64,
    pub success: bool,
}

impl PacketCodec for MergeItem {
    const ID: i16 = ServerPacketId::MergeItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(MergeItem {
            grid_from: r.read_u8()?,
            grid_to: r.read_u8()?,
            id_from: r.read_u64()?,
            id_to: r.read_u64()?,
            success: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.grid_from);
        w.write_u8(self.grid_to);
        w.write_u64(self.id_from);
        w.write_u64(self.id_to);
        w.write_bool(self.success);
    }
}

// ----------------------------- ID 40: RemoveItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RemoveItem {
    /// MirGridType (u8)
    pub grid: u8,
    pub unique_id: u64,
    pub to: i32,
    pub success: bool,
}

impl PacketCodec for RemoveItem {
    const ID: i16 = ServerPacketId::RemoveItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RemoveItem {
            grid: r.read_u8()?,
            unique_id: r.read_u64()?,
            to: r.read_i32()?,
            success: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.grid);
        w.write_u64(self.unique_id);
        w.write_i32(self.to);
        w.write_bool(self.success);
    }
}

// ----------------------------- ID 41: RemoveSlotItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RemoveSlotItem {
    /// MirGridType (u8)
    pub grid: u8,
    /// MirGridType (u8)
    pub grid_to: u8,
    pub unique_id: u64,
    pub to: i32,
    pub success: bool,
}

impl PacketCodec for RemoveSlotItem {
    const ID: i16 = ServerPacketId::RemoveSlotItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RemoveSlotItem {
            grid: r.read_u8()?,
            grid_to: r.read_u8()?,
            unique_id: r.read_u64()?,
            to: r.read_i32()?,
            success: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.grid);
        w.write_u8(self.grid_to);
        w.write_u64(self.unique_id);
        w.write_i32(self.to);
        w.write_bool(self.success);
    }
}

// ----------------------------- ID 42: TakeBackItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TakeBackItem {
    pub from: i32,
    pub to: i32,
    pub success: bool,
}

impl PacketCodec for TakeBackItem {
    const ID: i16 = ServerPacketId::TakeBackItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(TakeBackItem {
            from: r.read_i32()?,
            to: r.read_i32()?,
            success: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.from);
        w.write_i32(self.to);
        w.write_bool(self.success);
    }
}

// ----------------------------- ID 43: StoreItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoreItem {
    pub from: i32,
    pub to: i32,
    pub success: bool,
}

impl PacketCodec for StoreItem {
    const ID: i16 = ServerPacketId::StoreItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(StoreItem {
            from: r.read_i32()?,
            to: r.read_i32()?,
            success: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.from);
        w.write_i32(self.to);
        w.write_bool(self.success);
    }
}

// ----------------------------- ID 46: DepositRefineItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DepositRefineItem {
    pub from: i32,
    pub to: i32,
    pub success: bool,
}

impl PacketCodec for DepositRefineItem {
    const ID: i16 = ServerPacketId::DepositRefineItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(DepositRefineItem {
            from: r.read_i32()?,
            to: r.read_i32()?,
            success: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.from);
        w.write_i32(self.to);
        w.write_bool(self.success);
    }
}

// ----------------------------- ID 47: RetrieveRefineItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RetrieveRefineItem {
    pub from: i32,
    pub to: i32,
    pub success: bool,
}

impl PacketCodec for RetrieveRefineItem {
    const ID: i16 = ServerPacketId::RetrieveRefineItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RetrieveRefineItem {
            from: r.read_i32()?,
            to: r.read_i32()?,
            success: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.from);
        w.write_i32(self.to);
        w.write_bool(self.success);
    }
}

// ----------------------------- ID 48: RefineCancel -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RefineCancel {
    pub unlock: bool,
}

impl PacketCodec for RefineCancel {
    const ID: i16 = ServerPacketId::RefineCancel as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RefineCancel {
            unlock: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_bool(self.unlock);
    }
}

// ----------------------------- ID 49: RefineItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RefineItem {
    pub unique_id: u64,
}

impl PacketCodec for RefineItem {
    const ID: i16 = ServerPacketId::RefineItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RefineItem {
            unique_id: r.read_u64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
    }
}

// ----------------------------- ID 50: DepositTradeItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DepositTradeItem {
    pub from: i32,
    pub to: i32,
    pub success: bool,
}

impl PacketCodec for DepositTradeItem {
    const ID: i16 = ServerPacketId::DepositTradeItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(DepositTradeItem {
            from: r.read_i32()?,
            to: r.read_i32()?,
            success: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.from);
        w.write_i32(self.to);
        w.write_bool(self.success);
    }
}

// ----------------------------- ID 51: RetrieveTradeItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RetrieveTradeItem {
    pub from: i32,
    pub to: i32,
    pub success: bool,
}

impl PacketCodec for RetrieveTradeItem {
    const ID: i16 = ServerPacketId::RetrieveTradeItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RetrieveTradeItem {
            from: r.read_i32()?,
            to: r.read_i32()?,
            success: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.from);
        w.write_i32(self.to);
        w.write_bool(self.success);
    }
}

// ----------------------------- ID 44: SplitItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SplitItem {
    /// C#: `if (reader.ReadBoolean()) Item = new UserItem(reader);` —— true 表示存在
    pub item: Option<UserItem>,
    /// MirGridType (u8)
    pub grid: u8,
}

impl PacketCodec for SplitItem {
    const ID: i16 = ServerPacketId::SplitItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let item = if r.read_bool()? {
            Some(UserItem::read(r)?)
        } else {
            None
        };
        let grid = r.read_u8()?;
        Ok(SplitItem { item, grid })
    }

    fn write(&self, w: &mut Writer) {
        match &self.item {
            Some(item) => {
                w.write_bool(true);
                item.write(w);
            }
            None => w.write_bool(false),
        }
        w.write_u8(self.grid);
    }
}

// ----------------------------- ID 52: UseItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UseItem {
    pub unique_id: u64,
    pub success: bool,
    /// MirGridType (u8)
    pub grid: u8,
}

impl PacketCodec for UseItem {
    const ID: i16 = ServerPacketId::UseItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(UseItem {
            unique_id: r.read_u64()?,
            success: r.read_bool()?,
            grid: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
        w.write_bool(self.success);
        w.write_u8(self.grid);
    }
}

// ----------------------------- ID 53: DropItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DropItem {
    pub unique_id: u64,
    pub count: u16,
    pub hero_item: bool,
    pub success: bool,
}

impl PacketCodec for DropItem {
    const ID: i16 = ServerPacketId::DropItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(DropItem {
            unique_id: r.read_u64()?,
            count: r.read_u16()?,
            hero_item: r.read_bool()?,
            success: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
        w.write_u16(self.count);
        w.write_bool(self.hero_item);
        w.write_bool(self.success);
    }
}

// ----------------------------- ID 54: TakeBackHeroItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TakeBackHeroItem {
    pub from: i32,
    pub to: i32,
    pub success: bool,
}

impl PacketCodec for TakeBackHeroItem {
    const ID: i16 = ServerPacketId::TakeBackHeroItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(TakeBackHeroItem {
            from: r.read_i32()?,
            to: r.read_i32()?,
            success: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.from);
        w.write_i32(self.to);
        w.write_bool(self.success);
    }
}

// ----------------------------- ID 55: TransferHeroItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TransferHeroItem {
    pub from: i32,
    pub to: i32,
    pub success: bool,
}

impl PacketCodec for TransferHeroItem {
    const ID: i16 = ServerPacketId::TransferHeroItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(TransferHeroItem {
            from: r.read_i32()?,
            to: r.read_i32()?,
            success: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.from);
        w.write_i32(self.to);
        w.write_bool(self.success);
    }
}

// ----------------------------- ID 56: PlayerUpdate -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerUpdate {
    pub object_id: u32,
    pub light: u8,
    pub weapon: i16,
    pub weapon_effect: i16,
    pub armour: i16,
    pub wing_effect: u8,
}

impl PacketCodec for PlayerUpdate {
    const ID: i16 = ServerPacketId::PlayerUpdate as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(PlayerUpdate {
            object_id: r.read_u32()?,
            light: r.read_u8()?,
            weapon: r.read_i16()?,
            weapon_effect: r.read_i16()?,
            armour: r.read_i16()?,
            wing_effect: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_u8(self.light);
        w.write_i16(self.weapon);
        w.write_i16(self.weapon_effect);
        w.write_i16(self.armour);
        w.write_u8(self.wing_effect);
    }
}

// ----------------------------- ID 57: PlayerInspect -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerInspect {
    pub name: String,
    pub guild_name: String,
    pub guild_rank: String,
    /// C# `UserItem[] Equipment`: 先 count，再逐槽 `Write(T != null)`（true=存在）
    pub equipment: Vec<Option<UserItem>>,
    /// MirClass (u8)
    pub class: u8,
    /// MirGender (u8)
    pub gender: u8,
    pub hair: u8,
    pub level: u16,
    pub lover_name: String,
    pub allow_observe: bool,
    pub is_hero: bool,
}

impl PacketCodec for PlayerInspect {
    const ID: i16 = ServerPacketId::PlayerInspect as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let name = r.read_string()?;
        let guild_name = r.read_string()?;
        let guild_rank = r.read_string()?;
        let ec = r.read_i32()?;
        let mut equipment = Vec::with_capacity(ec.max(0) as usize);
        for _ in 0..ec.max(0) {
            if r.read_bool()? {
                equipment.push(Some(UserItem::read(r)?));
            } else {
                equipment.push(None);
            }
        }
        let class = r.read_u8()?;
        let gender = r.read_u8()?;
        let hair = r.read_u8()?;
        let level = r.read_u16()?;
        let lover_name = r.read_string()?;
        let allow_observe = r.read_bool()?;
        let is_hero = r.read_bool()?;
        Ok(PlayerInspect {
            name,
            guild_name,
            guild_rank,
            equipment,
            class,
            gender,
            hair,
            level,
            lover_name,
            allow_observe,
            is_hero,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.name);
        w.write_string(&self.guild_name);
        w.write_string(&self.guild_rank);
        w.write_i32(self.equipment.len() as i32);
        for item in &self.equipment {
            match item {
                Some(item) => {
                    w.write_bool(true);
                    item.write(w);
                }
                None => w.write_bool(false),
            }
        }
        w.write_u8(self.class);
        w.write_u8(self.gender);
        w.write_u8(self.hair);
        w.write_u16(self.level);
        w.write_string(&self.lover_name);
        w.write_bool(self.allow_observe);
        w.write_bool(self.is_hero);
    }
}

// ----------------------------- ID 189: MarriageRequest -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarriageRequest {
    pub name: String,
}

impl PacketCodec for MarriageRequest {
    const ID: i16 = ServerPacketId::MarriageRequest as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(MarriageRequest {
            name: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.name);
    }
}

// ----------------------------- ID 190: DivorceRequest -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DivorceRequest {
    pub name: String,
}

impl PacketCodec for DivorceRequest {
    const ID: i16 = ServerPacketId::DivorceRequest as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(DivorceRequest {
            name: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.name);
    }
}

// ----------------------------- ID 191: MentorRequest -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MentorRequest {
    pub name: String,
    pub level: u16,
}

impl PacketCodec for MentorRequest {
    const ID: i16 = ServerPacketId::MentorRequest as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(MentorRequest {
            name: r.read_string()?,
            level: r.read_u16()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.name);
        w.write_u16(self.level);
    }
}

// ----------------------------- ID 192: TradeRequest -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TradeRequest {
    pub name: String,
}

impl PacketCodec for TradeRequest {
    const ID: i16 = ServerPacketId::TradeRequest as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(TradeRequest {
            name: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.name);
    }
}

// ----------------------------- ID 193: TradeAccept -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TradeAccept {
    pub name: String,
}

impl PacketCodec for TradeAccept {
    const ID: i16 = ServerPacketId::TradeAccept as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(TradeAccept {
            name: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.name);
    }
}

// ----------------------------- ID 194: TradeGold -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TradeGold {
    pub amount: u32,
}

impl PacketCodec for TradeGold {
    const ID: i16 = ServerPacketId::TradeGold as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(TradeGold {
            amount: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.amount);
    }
}

// ----------------------------- ID 195: TradeItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TradeItem {
    /// C# `UserItem[] TradeItems`: 先 count，再逐槽 `Write(T != null)`（true=存在）
    pub trade_items: Vec<Option<UserItem>>,
}

impl PacketCodec for TradeItem {
    const ID: i16 = ServerPacketId::TradeItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let tc = r.read_i32()?;
        let mut trade_items = Vec::with_capacity(tc.max(0) as usize);
        for _ in 0..tc.max(0) {
            if r.read_bool()? {
                trade_items.push(Some(UserItem::read(r)?));
            } else {
                trade_items.push(None);
            }
        }
        Ok(TradeItem { trade_items })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.trade_items.len() as i32);
        for item in &self.trade_items {
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

// ----------------------------- ID 196: TradeConfirm -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TradeConfirm;

impl PacketCodec for TradeConfirm {
    const ID: i16 = ServerPacketId::TradeConfirm as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(TradeConfirm)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 197: TradeCancel -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TradeCancel {
    pub unlock: bool,
}

impl PacketCodec for TradeCancel {
    const ID: i16 = ServerPacketId::TradeCancel as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(TradeCancel {
            unlock: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_bool(self.unlock);
    }
}

// ----------------------------- ID 58: LogOutSuccess -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogOutSuccess {
    pub characters: Vec<SelectInfo>,
}

impl PacketCodec for LogOutSuccess {
    const ID: i16 = ServerPacketId::LogOutSuccess as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let count = r.read_i32()?;
        let mut characters = Vec::with_capacity(count.max(0) as usize);
        for _ in 0..count.max(0) {
            characters.push(SelectInfo::read(r)?);
        }
        Ok(LogOutSuccess { characters })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.characters.len() as i32);
        for c in &self.characters {
            c.write(w);
        }
    }
}

// ----------------------------- ID 59: LogOutFailed -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogOutFailed;

impl PacketCodec for LogOutFailed {
    const ID: i16 = ServerPacketId::LogOutFailed as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(LogOutFailed)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 60: ReturnToLogin -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReturnToLogin;

impl PacketCodec for ReturnToLogin {
    const ID: i16 = ServerPacketId::ReturnToLogin as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(ReturnToLogin)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 61: TimeOfDay -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TimeOfDay {
    /// LightSetting (u8)
    pub lights: u8,
}

impl PacketCodec for TimeOfDay {
    const ID: i16 = ServerPacketId::TimeOfDay as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(TimeOfDay {
            lights: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.lights);
    }
}

// ----------------------------- ID 62: ChangeAMode -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangeAMode {
    /// AttackMode (u8)
    pub mode: u8,
}

impl PacketCodec for ChangeAMode {
    const ID: i16 = ServerPacketId::ChangeAMode as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ChangeAMode { mode: r.read_u8()? })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.mode);
    }
}

// ----------------------------- ID 63: ChangePMode -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangePMode {
    /// PetMode (u8)
    pub mode: u8,
}

impl PacketCodec for ChangePMode {
    const ID: i16 = ServerPacketId::ChangePMode as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ChangePMode { mode: r.read_u8()? })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.mode);
    }
}

// ----------------------------- ID 64: ObjectItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectItem {
    pub object_id: u32,
    pub name: String,
    pub name_colour: Argb,
    pub location: Point,
    pub image: u16,
    /// ItemGrade (u8)
    pub grade: u8,
}

impl PacketCodec for ObjectItem {
    const ID: i16 = ServerPacketId::ObjectItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectItem {
            object_id: r.read_u32()?,
            name: r.read_string()?,
            name_colour: Argb::from_i32(r.read_i32()?),
            location: Point::read(r)?,
            image: r.read_u16()?,
            grade: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_string(&self.name);
        w.write_i32(self.name_colour.to_i32());
        self.location.write(w);
        w.write_u16(self.image);
        w.write_u8(self.grade);
    }
}

// ----------------------------- ID 65: ObjectGold -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectGold {
    pub object_id: u32,
    pub gold: u32,
    pub location: Point,
}

impl PacketCodec for ObjectGold {
    const ID: i16 = ServerPacketId::ObjectGold as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectGold {
            object_id: r.read_u32()?,
            gold: r.read_u32()?,
            location: Point::read(r)?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_u32(self.gold);
        self.location.write(w);
    }
}

// ----------------------------- ID 66: GainedItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GainedItem {
    pub item: UserItem,
}

impl PacketCodec for GainedItem {
    const ID: i16 = ServerPacketId::GainedItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(GainedItem {
            item: UserItem::read(r)?,
        })
    }

    fn write(&self, w: &mut Writer) {
        self.item.write(w);
    }
}
