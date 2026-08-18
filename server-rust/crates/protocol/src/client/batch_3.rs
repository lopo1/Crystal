// batch_3 —— 客户端→服务器数据包，对应 `Shared/ClientPackets.cs` 行 1604–2206
// （CB3 批次: MarketPage .. DeleteMail，32 个包）。
//
// 字段顺序严格照抄 C# ReadPacket/WritePacket；枚举一律存原始整数（u8），
// 注释注明 C# 枚举名；字符串为 .NET 7-bit 前缀。

use crate::binary::{Reader, Writer};
use crate::frame::PacketCodec;
use crate::ids::ClientPacketId;
use crate::Result;

// ----------------------------- ID 73: MarketPage -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarketPage {
    pub page: i32,
}

impl PacketCodec for MarketPage {
    const ID: i16 = ClientPacketId::MarketPage as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(MarketPage {
            page: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.page);
    }
}

// ----------------------------- ID 74: MarketBuy -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarketBuy {
    pub auction_id: u64,
    pub bid_price: u32,
}

impl PacketCodec for MarketBuy {
    const ID: i16 = ClientPacketId::MarketBuy as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(MarketBuy {
            auction_id: r.read_u64()?,
            bid_price: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.auction_id);
        w.write_u32(self.bid_price);
    }
}

// ----------------------------- ID 76: MarketSellNow -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarketSellNow {
    pub auction_id: u64,
}

impl PacketCodec for MarketSellNow {
    const ID: i16 = ClientPacketId::MarketSellNow as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(MarketSellNow {
            auction_id: r.read_u64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.auction_id);
    }
}

// ----------------------------- ID 75: MarketGetBack -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarketGetBack {
    /// MarketCollectionMode (u8)
    pub mode: u8,
    pub auction_id: u64,
}

impl PacketCodec for MarketGetBack {
    const ID: i16 = ClientPacketId::MarketGetBack as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(MarketGetBack {
            mode: r.read_u8()?,
            auction_id: r.read_u64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.mode);
        w.write_u64(self.auction_id);
    }
}

// ----------------------------- ID 77: RequestUserName -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestUserName {
    pub user_id: u32,
}

impl PacketCodec for RequestUserName {
    const ID: i16 = ClientPacketId::RequestUserName as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RequestUserName {
            user_id: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.user_id);
    }
}

// ----------------------------- ID 78: RequestChatItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestChatItem {
    pub chat_item_id: u64,
}

impl PacketCodec for RequestChatItem {
    const ID: i16 = ClientPacketId::RequestChatItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RequestChatItem {
            chat_item_id: r.read_u64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.chat_item_id);
    }
}

// ----------------------------- ID 79: EditGuildMember -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EditGuildMember {
    pub change_type: u8,
    pub rank_index: u8,
    pub name: String,
    pub rank_name: String,
}

impl PacketCodec for EditGuildMember {
    const ID: i16 = ClientPacketId::EditGuildMember as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(EditGuildMember {
            change_type: r.read_u8()?,
            rank_index: r.read_u8()?,
            name: r.read_string()?,
            rank_name: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.change_type);
        w.write_u8(self.rank_index);
        w.write_string(&self.name);
        w.write_string(&self.rank_name);
    }
}

// ----------------------------- ID 80: EditGuildNotice -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EditGuildNotice {
    pub notice: Vec<String>,
}

impl PacketCodec for EditGuildNotice {
    const ID: i16 = ClientPacketId::EditGuildNotice as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let count = r.read_i32()?;
        let mut notice = Vec::with_capacity(count.max(0) as usize);
        for _ in 0..count.max(0) {
            notice.push(r.read_string()?);
        }
        Ok(EditGuildNotice { notice })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.notice.len() as i32);
        for line in &self.notice {
            w.write_string(line);
        }
    }
}

// ----------------------------- ID 81: GuildInvite -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuildInvite {
    pub accept_invite: bool,
}

impl PacketCodec for GuildInvite {
    const ID: i16 = ClientPacketId::GuildInvite as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(GuildInvite {
            accept_invite: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_bool(self.accept_invite);
    }
}

// ----------------------------- ID 83: RequestGuildInfo -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestGuildInfo {
    pub r#type: u8,
}

impl PacketCodec for RequestGuildInfo {
    const ID: i16 = ClientPacketId::RequestGuildInfo as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RequestGuildInfo {
            r#type: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.r#type);
    }
}

// ----------------------------- ID 82: GuildNameReturn -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuildNameReturn {
    pub name: String,
}

impl PacketCodec for GuildNameReturn {
    const ID: i16 = ClientPacketId::GuildNameReturn as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(GuildNameReturn {
            name: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.name);
    }
}

// ----------------------------- ID 86: GuildWarReturn -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuildWarReturn {
    pub name: String,
}

impl PacketCodec for GuildWarReturn {
    const ID: i16 = ClientPacketId::GuildWarReturn as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(GuildWarReturn {
            name: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.name);
    }
}

// ----------------------------- ID 101: EquipSlotItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EquipSlotItem {
    /// MirGridType (u8)
    pub grid: u8,
    pub unique_id: u64,
    pub to: i32,
    /// MirGridType (u8)
    pub grid_to: u8,
    pub to_unique_id: u64,
}

impl PacketCodec for EquipSlotItem {
    const ID: i16 = ClientPacketId::EquipSlotItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(EquipSlotItem {
            grid: r.read_u8()?,
            unique_id: r.read_u64()?,
            to: r.read_i32()?,
            grid_to: r.read_u8()?,
            to_unique_id: r.read_u64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.grid);
        w.write_u64(self.unique_id);
        w.write_i32(self.to);
        w.write_u8(self.grid_to);
        w.write_u64(self.to_unique_id);
    }
}

// ----------------------------- ID 102: FishingCast -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FishingCast {
    pub cast_out: bool,
}

impl PacketCodec for FishingCast {
    const ID: i16 = ClientPacketId::FishingCast as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(FishingCast {
            cast_out: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_bool(self.cast_out);
    }
}

// ----------------------------- ID 103: FishingChangeAutocast -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FishingChangeAutocast {
    pub auto_cast: bool,
}

impl PacketCodec for FishingChangeAutocast {
    const ID: i16 = ClientPacketId::FishingChangeAutocast as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(FishingChangeAutocast {
            auto_cast: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_bool(self.auto_cast);
    }
}

// ----------------------------- ID 104: AcceptQuest -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AcceptQuest {
    pub npc_index: u32,
    pub quest_index: i32,
}

impl PacketCodec for AcceptQuest {
    const ID: i16 = ClientPacketId::AcceptQuest as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(AcceptQuest {
            npc_index: r.read_u32()?,
            quest_index: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.npc_index);
        w.write_i32(self.quest_index);
    }
}

// ----------------------------- ID 105: FinishQuest -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FinishQuest {
    pub quest_index: i32,
    pub selected_item_index: i32,
}

impl PacketCodec for FinishQuest {
    const ID: i16 = ClientPacketId::FinishQuest as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(FinishQuest {
            quest_index: r.read_i32()?,
            selected_item_index: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.quest_index);
        w.write_i32(self.selected_item_index);
    }
}

// ----------------------------- ID 106: AbandonQuest -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AbandonQuest {
    pub quest_index: i32,
}

impl PacketCodec for AbandonQuest {
    const ID: i16 = ClientPacketId::AbandonQuest as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(AbandonQuest {
            quest_index: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.quest_index);
    }
}

// ----------------------------- ID 107: ShareQuest -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShareQuest {
    pub quest_index: i32,
}

impl PacketCodec for ShareQuest {
    const ID: i16 = ClientPacketId::ShareQuest as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ShareQuest {
            quest_index: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.quest_index);
    }
}

// ----------------------------- ID 108: AcceptReincarnation -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AcceptReincarnation;

impl PacketCodec for AcceptReincarnation {
    const ID: i16 = ClientPacketId::AcceptReincarnation as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(AcceptReincarnation)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 109: CancelReincarnation -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CancelReincarnation;

impl PacketCodec for CancelReincarnation {
    const ID: i16 = ClientPacketId::CancelReincarnation as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(CancelReincarnation)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 110: CombineItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CombineItem {
    /// MirGridType (u8)
    pub grid: u8,
    pub id_from: u64,
    pub id_to: u64,
}

impl PacketCodec for CombineItem {
    const ID: i16 = ClientPacketId::CombineItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(CombineItem {
            grid: r.read_u8()?,
            id_from: r.read_u64()?,
            id_to: r.read_u64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.grid);
        w.write_u64(self.id_from);
        w.write_u64(self.id_to);
    }
}

// ----------------------------- ID 111: AwakeningNeedMaterials -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AwakeningNeedMaterials {
    pub unique_id: u64,
    /// AwakeType (u8)
    pub r#type: u8,
}

impl PacketCodec for AwakeningNeedMaterials {
    const ID: i16 = ClientPacketId::AwakeningNeedMaterials as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(AwakeningNeedMaterials {
            unique_id: r.read_u64()?,
            r#type: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
        w.write_u8(self.r#type);
    }
}

// ----------------------------- ID 112: AwakeningLockedItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AwakeningLockedItem {
    pub unique_id: u64,
    pub locked: bool,
}

impl PacketCodec for AwakeningLockedItem {
    const ID: i16 = ClientPacketId::AwakeningLockedItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(AwakeningLockedItem {
            unique_id: r.read_u64()?,
            locked: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
        w.write_bool(self.locked);
    }
}

// ----------------------------- ID 113: Awakening -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Awakening {
    pub unique_id: u64,
    /// AwakeType (u8)
    pub r#type: u8,
    pub position_idx: u32,
}

impl PacketCodec for Awakening {
    const ID: i16 = ClientPacketId::Awakening as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Awakening {
            unique_id: r.read_u64()?,
            r#type: r.read_u8()?,
            position_idx: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
        w.write_u8(self.r#type);
        w.write_u32(self.position_idx);
    }
}

// ----------------------------- ID 114: DisassembleItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DisassembleItem {
    pub unique_id: u64,
}

impl PacketCodec for DisassembleItem {
    const ID: i16 = ClientPacketId::DisassembleItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(DisassembleItem {
            unique_id: r.read_u64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
    }
}

// ----------------------------- ID 115: DowngradeAwakening -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DowngradeAwakening {
    pub unique_id: u64,
}

impl PacketCodec for DowngradeAwakening {
    const ID: i16 = ClientPacketId::DowngradeAwakening as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(DowngradeAwakening {
            unique_id: r.read_u64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
    }
}

// ----------------------------- ID 116: ResetAddedItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResetAddedItem {
    pub unique_id: u64,
}

impl PacketCodec for ResetAddedItem {
    const ID: i16 = ClientPacketId::ResetAddedItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ResetAddedItem {
            unique_id: r.read_u64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
    }
}

// ----------------------------- ID 117: SendMail -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SendMail {
    pub name: String,
    pub message: String,
    pub gold: u32,
    /// C# `ulong[] ItemsIdx = new ulong[5];` —— 固定 5 个 u64，无长度前缀
    pub items_idx: [u64; 5],
    pub stamped: bool,
}

impl PacketCodec for SendMail {
    const ID: i16 = ClientPacketId::SendMail as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let name = r.read_string()?;
        let message = r.read_string()?;
        let gold = r.read_u32()?;
        let mut items_idx = [0u64; 5];
        for slot in items_idx.iter_mut() {
            *slot = r.read_u64()?;
        }
        let stamped = r.read_bool()?;
        Ok(SendMail {
            name,
            message,
            gold,
            items_idx,
            stamped,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.name);
        w.write_string(&self.message);
        w.write_u32(self.gold);
        for slot in &self.items_idx {
            w.write_u64(*slot);
        }
        w.write_bool(self.stamped);
    }
}

// ----------------------------- ID 118: ReadMail -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReadMail {
    pub mail_id: u64,
}

impl PacketCodec for ReadMail {
    const ID: i16 = ClientPacketId::ReadMail as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ReadMail {
            mail_id: r.read_u64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.mail_id);
    }
}

// ----------------------------- ID 119: CollectParcel -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CollectParcel {
    pub mail_id: u64,
}

impl PacketCodec for CollectParcel {
    const ID: i16 = ClientPacketId::CollectParcel as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(CollectParcel {
            mail_id: r.read_u64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.mail_id);
    }
}

// ----------------------------- ID 120: DeleteMail -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeleteMail {
    pub mail_id: u64,
}

impl PacketCodec for DeleteMail {
    const ID: i16 = ClientPacketId::DeleteMail as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(DeleteMail {
            mail_id: r.read_u64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.mail_id);
    }
}
