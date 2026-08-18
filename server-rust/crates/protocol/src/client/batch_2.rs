// batch_2 —— 客户端→服务器包（对应 `Shared/ClientPackets.cs` 行 1012–1603）
//
// 覆盖: RequestMonsterInfo .. MarketRefresh（ID 37–72 及 87–100、147–148 等）。
// 枚举字段一律按线上原文保留为原始整数（u8/i8/u16/i16），注释注明 C# 枚举名。

use crate::binary::{Point, Reader, Writer};
use crate::frame::PacketCodec;
use crate::ids::ClientPacketId;
use crate::Result;

// ----------------------------- ID 37: RequestMonsterInfo -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestMonsterInfo {
    pub monster_index: i32,
}

impl PacketCodec for RequestMonsterInfo {
    const ID: i16 = ClientPacketId::RequestMonsterInfo as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RequestMonsterInfo {
            monster_index: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.monster_index);
    }
}

// ----------------------------- ID 38: RequestNPCInfo -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestNPCInfo {
    pub npc_index: i32,
}

impl PacketCodec for RequestNPCInfo {
    const ID: i16 = ClientPacketId::RequestNPCInfo as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RequestNPCInfo {
            npc_index: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.npc_index);
    }
}

// ----------------------------- ID 39: RequestItemInfo -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestItemInfo {
    pub item_index: i32,
}

impl PacketCodec for RequestItemInfo {
    const ID: i16 = ClientPacketId::RequestItemInfo as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RequestItemInfo {
            item_index: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.item_index);
    }
}

// ----------------------------- ID 40: TeleportToNPC -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TeleportToNPC {
    pub object_id: u32,
}

impl PacketCodec for TeleportToNPC {
    const ID: i16 = ClientPacketId::TeleportToNPC as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(TeleportToNPC {
            object_id: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
    }
}

// ----------------------------- ID 41: SearchMap -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchMap {
    pub text: String,
}

impl PacketCodec for SearchMap {
    const ID: i16 = ClientPacketId::SearchMap as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(SearchMap {
            text: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.text);
    }
}

// ----------------------------- ID 57: MagicKey -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MagicKey {
    /// Spell (u8)
    pub spell: u8,
    pub key: u8,
    pub old_key: u8,
}

impl PacketCodec for MagicKey {
    const ID: i16 = ClientPacketId::MagicKey as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(MagicKey {
            spell: r.read_u8()?,
            key: r.read_u8()?,
            old_key: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.spell);
        w.write_u8(self.key);
        w.write_u8(self.old_key);
    }
}

// ----------------------------- ID 58: Magic -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Magic {
    pub object_id: u32,
    /// Spell (u8)
    pub spell: u8,
    /// MirDirection (u8)
    pub direction: u8,
    pub target_id: u32,
    pub location: Point,
    pub spell_target_lock: bool,
}

impl PacketCodec for Magic {
    const ID: i16 = ClientPacketId::Magic as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Magic {
            object_id: r.read_u32()?,
            spell: r.read_u8()?,
            direction: r.read_u8()?,
            target_id: r.read_u32()?,
            location: Point::read(r)?,
            spell_target_lock: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_u8(self.spell);
        w.write_u8(self.direction);
        w.write_u32(self.target_id);
        self.location.write(w);
        w.write_bool(self.spell_target_lock);
    }
}

// ----------------------------- ID 59: SwitchGroup -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SwitchGroup {
    pub allow_group: bool,
}

impl PacketCodec for SwitchGroup {
    const ID: i16 = ClientPacketId::SwitchGroup as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(SwitchGroup {
            allow_group: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_bool(self.allow_group);
    }
}

// ----------------------------- ID 60: AddMember -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AddMember {
    pub name: String,
}

impl PacketCodec for AddMember {
    const ID: i16 = ClientPacketId::AddMember as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(AddMember {
            name: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.name);
    }
}

// ----------------------------- ID 61: DelMember -----------------------------
// 注意: C# 类名是 DelMember（单 l），枚举名是 DellMember（双 l，见 Enums.cs），
// 故 ID 取 ClientPacketId::DellMember。

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DelMember {
    pub name: String,
}

impl PacketCodec for DelMember {
    const ID: i16 = ClientPacketId::DellMember as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(DelMember {
            name: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.name);
    }
}

// ----------------------------- ID 62: GroupInvite -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GroupInvite {
    pub accept_invite: bool,
}

impl PacketCodec for GroupInvite {
    const ID: i16 = ClientPacketId::GroupInvite as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(GroupInvite {
            accept_invite: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_bool(self.accept_invite);
    }
}

// ----------------------------- ID 63: NewHero -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewHero {
    pub name: String,
    /// MirGender (u8)
    pub gender: u8,
    /// MirClass (u8)
    pub class: u8,
}

impl PacketCodec for NewHero {
    const ID: i16 = ClientPacketId::NewHero as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(NewHero {
            name: r.read_string()?,
            gender: r.read_u8()?,
            class: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.name);
        w.write_u8(self.gender);
        w.write_u8(self.class);
    }
}

// ----------------------------- ID 64: SetAutoPotValue -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SetAutoPotValue {
    /// Stat (u8)
    pub stat: u8,
    pub value: u32,
}

impl PacketCodec for SetAutoPotValue {
    const ID: i16 = ClientPacketId::SetAutoPotValue as i16;

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

// ----------------------------- ID 65: SetAutoPotItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SetAutoPotItem {
    /// MirGridType (u8)
    pub grid: u8,
    pub item_index: i32,
}

impl PacketCodec for SetAutoPotItem {
    const ID: i16 = ClientPacketId::SetAutoPotItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(SetAutoPotItem {
            grid: r.read_u8()?,
            item_index: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.grid);
        w.write_i32(self.item_index);
    }
}

// ----------------------------- ID 66: SetHeroBehaviour -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SetHeroBehaviour {
    /// HeroBehaviour (u8)
    pub behaviour: u8,
}

impl PacketCodec for SetHeroBehaviour {
    const ID: i16 = ClientPacketId::SetHeroBehaviour as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(SetHeroBehaviour {
            behaviour: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.behaviour);
    }
}

// ----------------------------- ID 67: ChangeHero -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangeHero {
    pub list_index: i32,
}

impl PacketCodec for ChangeHero {
    const ID: i16 = ClientPacketId::ChangeHero as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ChangeHero {
            list_index: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.list_index);
    }
}

// ----------------------------- ID 87: MarriageRequest -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarriageRequest;

impl PacketCodec for MarriageRequest {
    const ID: i16 = ClientPacketId::MarriageRequest as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(MarriageRequest)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 88: MarriageReply -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarriageReply {
    pub accept_invite: bool,
}

impl PacketCodec for MarriageReply {
    const ID: i16 = ClientPacketId::MarriageReply as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(MarriageReply {
            accept_invite: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_bool(self.accept_invite);
    }
}

// ----------------------------- ID 89: ChangeMarriage -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangeMarriage;

impl PacketCodec for ChangeMarriage {
    const ID: i16 = ClientPacketId::ChangeMarriage as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(ChangeMarriage)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 90: DivorceRequest -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DivorceRequest;

impl PacketCodec for DivorceRequest {
    const ID: i16 = ClientPacketId::DivorceRequest as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(DivorceRequest)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 91: DivorceReply -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DivorceReply {
    pub accept_invite: bool,
}

impl PacketCodec for DivorceReply {
    const ID: i16 = ClientPacketId::DivorceReply as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(DivorceReply {
            accept_invite: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_bool(self.accept_invite);
    }
}

// ----------------------------- ID 92: AddMentor -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AddMentor {
    pub name: String,
}

impl PacketCodec for AddMentor {
    const ID: i16 = ClientPacketId::AddMentor as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(AddMentor {
            name: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.name);
    }
}

// ----------------------------- ID 93: MentorReply -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MentorReply {
    pub accept_invite: bool,
}

impl PacketCodec for MentorReply {
    const ID: i16 = ClientPacketId::MentorReply as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(MentorReply {
            accept_invite: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_bool(self.accept_invite);
    }
}

// ----------------------------- ID 94: AllowMentor -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AllowMentor;

impl PacketCodec for AllowMentor {
    const ID: i16 = ClientPacketId::AllowMentor as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(AllowMentor)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 95: CancelMentor -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CancelMentor;

impl PacketCodec for CancelMentor {
    const ID: i16 = ClientPacketId::CancelMentor as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(CancelMentor)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 96: TradeRequest -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TradeRequest;

impl PacketCodec for TradeRequest {
    const ID: i16 = ClientPacketId::TradeRequest as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(TradeRequest)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 97: TradeReply -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TradeReply {
    pub accept_invite: bool,
}

impl PacketCodec for TradeReply {
    const ID: i16 = ClientPacketId::TradeReply as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(TradeReply {
            accept_invite: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_bool(self.accept_invite);
    }
}

// ----------------------------- ID 98: TradeGold -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TradeGold {
    pub amount: u32,
}

impl PacketCodec for TradeGold {
    const ID: i16 = ClientPacketId::TradeGold as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(TradeGold {
            amount: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.amount);
    }
}

// ----------------------------- ID 99: TradeConfirm -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TradeConfirm {
    pub locked: bool,
}

impl PacketCodec for TradeConfirm {
    const ID: i16 = ClientPacketId::TradeConfirm as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(TradeConfirm {
            locked: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_bool(self.locked);
    }
}

// ----------------------------- ID 100: TradeCancel -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TradeCancel;

impl PacketCodec for TradeCancel {
    const ID: i16 = ClientPacketId::TradeCancel as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(TradeCancel)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 68: TownRevive -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TownRevive;

impl PacketCodec for TownRevive {
    const ID: i16 = ClientPacketId::TownRevive as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(TownRevive)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 69: SpellToggle -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpellToggle {
    /// Spell (u8)
    pub spell: u8,
    /// SpellToggleState (i8, sbyte) —— C# 通过 Convert.ToBoolean 暴露属性，
    /// 线路上始终序列化原始 sbyte 值。
    pub can_use: i8,
}

impl PacketCodec for SpellToggle {
    const ID: i16 = ClientPacketId::SpellToggle as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(SpellToggle {
            spell: r.read_u8()?,
            can_use: r.read_i8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.spell);
        w.write_i8(self.can_use);
    }
}

// ----------------------------- ID 70: ConsignItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConsignItem {
    pub unique_id: u64,
    pub price: u32,
    /// MarketPanelType (u8)
    pub r#type: u8,
}

impl PacketCodec for ConsignItem {
    const ID: i16 = ClientPacketId::ConsignItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ConsignItem {
            unique_id: r.read_u64()?,
            price: r.read_u32()?,
            r#type: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
        w.write_u32(self.price);
        w.write_u8(self.r#type);
    }
}

// ----------------------------- ID 147: GuildTerritoryPage -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuildTerritoryPage {
    pub page: i32,
}

impl PacketCodec for GuildTerritoryPage {
    const ID: i16 = ClientPacketId::GuildTerritoryPage as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(GuildTerritoryPage {
            page: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.page);
    }
}

// ----------------------------- ID 148: PurchaseGuildTerritory -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PurchaseGuildTerritory {
    pub owner: String,
}

impl PacketCodec for PurchaseGuildTerritory {
    const ID: i16 = ClientPacketId::PurchaseGuildTerritory as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(PurchaseGuildTerritory {
            owner: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.owner);
    }
}

// ----------------------------- ID 71: MarketSearch -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarketSearch {
    pub r#match: String,
    /// ItemType (u8)
    pub r#type: u8,
    pub usermode: bool,
    pub min_shape: i16,
    pub max_shape: i16,
    /// MarketPanelType (u8)
    pub market_type: u8,
}

impl PacketCodec for MarketSearch {
    const ID: i16 = ClientPacketId::MarketSearch as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(MarketSearch {
            r#match: r.read_string()?,
            r#type: r.read_u8()?,
            usermode: r.read_bool()?,
            min_shape: r.read_i16()?,
            max_shape: r.read_i16()?,
            market_type: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.r#match);
        w.write_u8(self.r#type);
        w.write_bool(self.usermode);
        w.write_i16(self.min_shape);
        w.write_i16(self.max_shape);
        w.write_u8(self.market_type);
    }
}

// ----------------------------- ID 72: MarketRefresh -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarketRefresh;

impl PacketCodec for MarketRefresh {
    const ID: i16 = ClientPacketId::MarketRefresh as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(MarketRefresh)
    }

    fn write(&self, _w: &mut Writer) {}
}
