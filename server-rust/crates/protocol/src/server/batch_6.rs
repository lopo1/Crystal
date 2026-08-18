//! batch_6 —— 对应 `Shared/ServerPackets.cs` 末尾一节（约 5809–6831 行）。
//!
//! 邮件 / 灵兽 / 好友 / 行会 BUFF / 排行 / 租赁 / 祝福 / 公告 / 地图对象信息等包。
//! 字段顺序严格对照 C# ReadPacket/WritePacket，逐字节兼容（见 `docs/PROTOCOL.md`）。

use crate::binary::{Point, Reader, Writer};
use crate::frame::PacketCodec;
use crate::ids::ServerPacketId;
use crate::types::*;
use crate::Result;

// ---------------------------------------------------------------------------
// 本批包内嵌类型（C# Shared/Data 中对应类，尚未收入 types.rs，故定义于此）
// ---------------------------------------------------------------------------

/// 对应 `Shared/Data/GuildData.cs` 的 `GuildBuffInfo`（供 GuildBuffList 使用）
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuildBuffInfo {
    pub id: i32,
    pub icon: i32,
    pub name: String,
    pub level_requirement: u8,
    pub points_requirement: u8,
    pub time_limit: i32,
    pub activation_cost: i32,
    pub stats: Stats,
}

impl GuildBuffInfo {
    pub fn read(r: &mut Reader) -> Result<Self> {
        Ok(GuildBuffInfo {
            id: r.read_i32()?,
            icon: r.read_i32()?,
            name: r.read_string()?,
            level_requirement: r.read_u8()?,
            points_requirement: r.read_u8()?,
            time_limit: r.read_i32()?,
            activation_cost: r.read_i32()?,
            stats: Stats::read(r)?,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_i32(self.id);
        w.write_i32(self.icon);
        w.write_string(&self.name);
        w.write_u8(self.level_requirement);
        w.write_u8(self.points_requirement);
        w.write_i32(self.time_limit);
        w.write_i32(self.activation_cost);
        self.stats.write(w);
    }
}

/// 对应 `Shared/Data/GuildData.cs` 的 `GuildBuff`（供 GuildBuffList 使用）
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuildBuff {
    pub id: i32,
    pub active: bool,
    pub active_time_remaining: i32,
}

impl GuildBuff {
    pub fn read(r: &mut Reader) -> Result<Self> {
        Ok(GuildBuff {
            id: r.read_i32()?,
            active: r.read_bool()?,
            active_time_remaining: r.read_i32()?,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_i32(self.id);
        w.write_bool(self.active);
        w.write_i32(self.active_time_remaining);
    }
}

/// 对应 `Shared/Data/ItemData.cs` 的 `ItemRentalInformation`（供 GetRentedItems 使用）
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemRentalInformation {
    pub item_id: u64,
    pub item_name: String,
    pub renting_player_name: String,
    /// .NET DateTime binary (i64)
    pub item_return_date: i64,
}

impl ItemRentalInformation {
    pub fn read(r: &mut Reader) -> Result<Self> {
        Ok(ItemRentalInformation {
            item_id: r.read_u64()?,
            item_name: r.read_string()?,
            renting_player_name: r.read_string()?,
            item_return_date: r.read_i64()?,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_u64(self.item_id);
        w.write_string(&self.item_name);
        w.write_string(&self.renting_player_name);
        w.write_i64(self.item_return_date);
    }
}

/// 对应 `Shared/Data/Notice.cs` 的 `Notice`（供 UpdateNotice 使用）。
/// C# 中 `LastUpdate` 字段不参与序列化。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Notice {
    pub title: String,
    pub message: String,
}

impl Notice {
    pub fn read(r: &mut Reader) -> Result<Self> {
        Ok(Notice {
            title: r.read_string()?,
            message: r.read_string()?,
        })
    }
    pub fn write(&self, w: &mut Writer) {
        w.write_string(&self.title);
        w.write_string(&self.message);
    }
}

// ----------------------------- ID 228: AwakeningNeedMaterials -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AwakeningNeedMaterials {
    /// 外层 Option: C# `Materials != null` 标志；内层元素可 null（`Materials[i] != null`），
    /// 每项成对 (ItemInfo, 对应 MaterialsCount 字节)，null 槽不写 count。
    pub materials: Option<Vec<Option<(ItemInfo, u8)>>>,
}

impl PacketCodec for AwakeningNeedMaterials {
    const ID: i16 = ServerPacketId::AwakeningNeedMaterials as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        if !r.read_bool()? {
            return Ok(AwakeningNeedMaterials { materials: None });
        }
        let count = r.read_i32()?;
        let mut materials = Vec::with_capacity(count.max(0) as usize);
        for _ in 0..count.max(0) {
            if !r.read_bool()? {
                materials.push(None);
            } else {
                let info = ItemInfo::read(r)?;
                let count_byte = r.read_u8()?;
                materials.push(Some((info, count_byte)));
            }
        }
        Ok(AwakeningNeedMaterials {
            materials: Some(materials),
        })
    }

    fn write(&self, w: &mut Writer) {
        match &self.materials {
            None => w.write_bool(false),
            Some(list) => {
                w.write_bool(true);
                w.write_i32(list.len() as i32);
                for m in list {
                    match m {
                        None => w.write_bool(false),
                        Some((info, count_byte)) => {
                            w.write_bool(true);
                            info.write(w);
                            w.write_u8(*count_byte);
                        }
                    }
                }
            }
        }
    }
}

// ----------------------------- ID 229: AwakeningLockedItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AwakeningLockedItem {
    pub unique_id: u64,
    pub locked: bool,
}

impl PacketCodec for AwakeningLockedItem {
    const ID: i16 = ServerPacketId::AwakeningLockedItem as i16;

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

// ----------------------------- ID 230: Awakening -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Awakening {
    pub result: i32,
    pub remove_id: i64,
}

impl PacketCodec for Awakening {
    const ID: i16 = ServerPacketId::Awakening as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Awakening {
            result: r.read_i32()?,
            remove_id: r.read_i64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.result);
        w.write_i64(self.remove_id);
    }
}

// ----------------------------- ID 231: ReceiveMail -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReceiveMail {
    pub mail: Vec<ClientMail>,
}

impl PacketCodec for ReceiveMail {
    const ID: i16 = ServerPacketId::ReceiveMail as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let count = r.read_i32()?;
        let mut mail = Vec::with_capacity(count.max(0) as usize);
        for _ in 0..count.max(0) {
            mail.push(ClientMail::read(r)?);
        }
        Ok(ReceiveMail { mail })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.mail.len() as i32);
        for m in &self.mail {
            m.write(w);
        }
    }
}

// ----------------------------- ID 232: MailLockedItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MailLockedItem {
    pub unique_id: u64,
    pub locked: bool,
}

impl PacketCodec for MailLockedItem {
    const ID: i16 = ServerPacketId::MailLockedItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(MailLockedItem {
            unique_id: r.read_u64()?,
            locked: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.unique_id);
        w.write_bool(self.locked);
    }
}

// ----------------------------- ID 233: MailSendRequest -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MailSendRequest;

impl PacketCodec for MailSendRequest {
    const ID: i16 = ServerPacketId::MailSendRequest as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(MailSendRequest)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 234: MailSent -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MailSent {
    /// C# `sbyte Result`
    pub result: i8,
}

impl PacketCodec for MailSent {
    const ID: i16 = ServerPacketId::MailSent as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(MailSent {
            result: r.read_i8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i8(self.result);
    }
}

// ----------------------------- ID 235: ParcelCollected -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParcelCollected {
    /// C# `sbyte Result`
    pub result: i8,
}

impl PacketCodec for ParcelCollected {
    const ID: i16 = ServerPacketId::ParcelCollected as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ParcelCollected {
            result: r.read_i8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i8(self.result);
    }
}

// ----------------------------- ID 236: MailCost -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MailCost {
    pub cost: u32,
}

impl PacketCodec for MailCost {
    const ID: i16 = ServerPacketId::MailCost as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(MailCost {
            cost: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.cost);
    }
}

// ----------------------------- ID 237: ResizeInventory -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResizeInventory {
    pub size: i32,
}

impl PacketCodec for ResizeInventory {
    const ID: i16 = ServerPacketId::ResizeInventory as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ResizeInventory {
            size: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.size);
    }
}

// ----------------------------- ID 238: ResizeStorage -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResizeStorage {
    pub size: i32,
    pub has_expanded_storage: bool,
    /// .NET DateTime binary (i64)
    pub expiry_time: i64,
}

impl PacketCodec for ResizeStorage {
    const ID: i16 = ServerPacketId::ResizeStorage as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ResizeStorage {
            size: r.read_i32()?,
            has_expanded_storage: r.read_bool()?,
            expiry_time: r.read_i64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.size);
        w.write_bool(self.has_expanded_storage);
        w.write_i64(self.expiry_time);
    }
}

// ----------------------------- ID 239: NewIntelligentCreature -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewIntelligentCreature {
    pub creature: ClientIntelligentCreature,
}

impl PacketCodec for NewIntelligentCreature {
    const ID: i16 = ServerPacketId::NewIntelligentCreature as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(NewIntelligentCreature {
            creature: ClientIntelligentCreature::read(r)?,
        })
    }

    fn write(&self, w: &mut Writer) {
        self.creature.write(w);
    }
}

// ----------------------------- ID 240: UpdateIntelligentCreatureList -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UpdateIntelligentCreatureList {
    pub creature_list: Vec<ClientIntelligentCreature>,
    pub creature_summoned: bool,
    /// IntelligentCreatureType (u8)
    pub summoned_creature_type: u8,
    pub pearl_count: i32,
}

impl PacketCodec for UpdateIntelligentCreatureList {
    const ID: i16 = ServerPacketId::UpdateIntelligentCreatureList as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let count = r.read_i32()?;
        let mut creature_list = Vec::with_capacity(count.max(0) as usize);
        for _ in 0..count.max(0) {
            creature_list.push(ClientIntelligentCreature::read(r)?);
        }
        Ok(UpdateIntelligentCreatureList {
            creature_list,
            creature_summoned: r.read_bool()?,
            summoned_creature_type: r.read_u8()?,
            pearl_count: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.creature_list.len() as i32);
        for c in &self.creature_list {
            c.write(w);
        }
        w.write_bool(self.creature_summoned);
        w.write_u8(self.summoned_creature_type);
        w.write_i32(self.pearl_count);
    }
}

// ----------------------------- ID 241: IntelligentCreatureEnableRename -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IntelligentCreatureEnableRename;

impl PacketCodec for IntelligentCreatureEnableRename {
    const ID: i16 = ServerPacketId::IntelligentCreatureEnableRename as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(IntelligentCreatureEnableRename)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 242: IntelligentCreaturePickup -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IntelligentCreaturePickup {
    pub object_id: u32,
}

impl PacketCodec for IntelligentCreaturePickup {
    const ID: i16 = ServerPacketId::IntelligentCreaturePickup as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(IntelligentCreaturePickup {
            object_id: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
    }
}

// ----------------------------- ID 243: NPCPearlGoods -----------------------------

// 注意: C# `float Rate`（BinaryWriter.Write(float)）→ Rust f32。f32 未实现 Eq，
// 故本结构体无法 derive Eq（仅 PartialEq），其余包均满足 Eq。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NPCPearlGoods {
    pub list: Vec<UserItem>,
    pub rate: f32,
    /// PanelType (u8)
    pub r#type: u8,
}

impl PacketCodec for NPCPearlGoods {
    const ID: i16 = ServerPacketId::NPCPearlGoods as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let count = r.read_i32()?;
        let mut list = Vec::with_capacity(count.max(0) as usize);
        for _ in 0..count.max(0) {
            list.push(UserItem::read(r)?);
        }
        Ok(NPCPearlGoods {
            list,
            rate: r.read_f32()?,
            r#type: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.list.len() as i32);
        for i in &self.list {
            i.write(w);
        }
        w.write_f32(self.rate);
        w.write_u8(self.r#type);
    }
}

// ----------------------------- ID 245: FriendUpdate -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FriendUpdate {
    pub friends: Vec<ClientFriend>,
}

impl PacketCodec for FriendUpdate {
    const ID: i16 = ServerPacketId::FriendUpdate as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let count = r.read_i32()?;
        let mut friends = Vec::with_capacity(count.max(0) as usize);
        for _ in 0..count.max(0) {
            friends.push(ClientFriend::read(r)?);
        }
        Ok(FriendUpdate { friends })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.friends.len() as i32);
        for f in &self.friends {
            f.write(w);
        }
    }
}

// ----------------------------- ID 246: LoverUpdate -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoverUpdate {
    pub name: String,
    /// .NET DateTime binary (i64)
    pub date: i64,
    pub map_name: String,
    pub married_days: i16,
}

impl PacketCodec for LoverUpdate {
    const ID: i16 = ServerPacketId::LoverUpdate as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(LoverUpdate {
            name: r.read_string()?,
            date: r.read_i64()?,
            map_name: r.read_string()?,
            married_days: r.read_i16()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.name);
        w.write_i64(self.date);
        w.write_string(&self.map_name);
        w.write_i16(self.married_days);
    }
}

// ----------------------------- ID 247: MentorUpdate -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MentorUpdate {
    pub name: String,
    pub level: u16,
    pub online: bool,
    pub mentee_exp: i64,
}

impl PacketCodec for MentorUpdate {
    const ID: i16 = ServerPacketId::MentorUpdate as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(MentorUpdate {
            name: r.read_string()?,
            level: r.read_u16()?,
            online: r.read_bool()?,
            mentee_exp: r.read_i64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.name);
        w.write_u16(self.level);
        w.write_bool(self.online);
        w.write_i64(self.mentee_exp);
    }
}

// ----------------------------- ID 248: GuildBuffList -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuildBuffList {
    pub remove: u8,
    pub active_buffs: Vec<GuildBuff>,
    pub guild_buffs: Vec<GuildBuffInfo>,
}

impl PacketCodec for GuildBuffList {
    const ID: i16 = ServerPacketId::GuildBuffList as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let remove = r.read_u8()?;
        let mut active_buffs = Vec::new();
        let acount = r.read_i32()?;
        for _ in 0..acount.max(0) {
            active_buffs.push(GuildBuff::read(r)?);
        }
        let mut guild_buffs = Vec::new();
        let gcount = r.read_i32()?;
        for _ in 0..gcount.max(0) {
            guild_buffs.push(GuildBuffInfo::read(r)?);
        }
        Ok(GuildBuffList {
            remove,
            active_buffs,
            guild_buffs,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.remove);
        w.write_i32(self.active_buffs.len() as i32);
        for b in &self.active_buffs {
            b.write(w);
        }
        w.write_i32(self.guild_buffs.len() as i32);
        for b in &self.guild_buffs {
            b.write(w);
        }
    }
}

// ----------------------------- ID 249: NPCRequestInput -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NPCRequestInput {
    pub npc_id: u32,
    pub page_name: String,
}

impl PacketCodec for NPCRequestInput {
    const ID: i16 = ServerPacketId::NPCRequestInput as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(NPCRequestInput {
            npc_id: r.read_u32()?,
            page_name: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.npc_id);
        w.write_string(&self.page_name);
    }
}

// ----------------------------- ID 252: Rankings -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rankings {
    /// RankType (u8)
    pub rank_type: u8,
    pub my_rank: i32,
    pub listing_details: Vec<RankCharacterInfo>,
    pub listings: Vec<i64>,
    pub count: i32,
}

impl PacketCodec for Rankings {
    const ID: i16 = ServerPacketId::Rankings as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let rank_type = r.read_u8()?;
        let my_rank = r.read_i32()?;
        let mut listing_details = Vec::new();
        let dcount = r.read_i32()?;
        for _ in 0..dcount.max(0) {
            listing_details.push(RankCharacterInfo::read(r)?);
        }
        let mut listings = Vec::new();
        let lcount = r.read_i32()?;
        for _ in 0..lcount.max(0) {
            listings.push(r.read_i64()?);
        }
        Ok(Rankings {
            rank_type,
            my_rank,
            listing_details,
            listings,
            count: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.rank_type);
        w.write_i32(self.my_rank);
        w.write_i32(self.listing_details.len() as i32);
        for d in &self.listing_details {
            d.write(w);
        }
        w.write_i32(self.listings.len() as i32);
        for l in &self.listings {
            w.write_i64(*l);
        }
        w.write_i32(self.count);
    }
}

// ----------------------------- ID 253: Opendoor -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Opendoor {
    /// 注意: C# 序列化顺序是 DoorIndex 先于 Close
    pub door_index: u8,
    pub close: bool,
}

impl PacketCodec for Opendoor {
    const ID: i16 = ServerPacketId::Opendoor as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Opendoor {
            door_index: r.read_u8()?,
            close: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.door_index);
        w.write_bool(self.close);
    }
}

// ----------------------------- ID 254: GetRentedItems -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GetRentedItems {
    pub rented_items: Vec<ItemRentalInformation>,
}

impl PacketCodec for GetRentedItems {
    const ID: i16 = ServerPacketId::GetRentedItems as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let count = r.read_i32()?;
        let mut rented_items = Vec::with_capacity(count.max(0) as usize);
        for _ in 0..count.max(0) {
            rented_items.push(ItemRentalInformation::read(r)?);
        }
        Ok(GetRentedItems { rented_items })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.rented_items.len() as i32);
        for i in &self.rented_items {
            i.write(w);
        }
    }
}

// ----------------------------- ID 255: ItemRentalRequest -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemRentalRequest {
    pub name: String,
    pub renting: bool,
}

impl PacketCodec for ItemRentalRequest {
    const ID: i16 = ServerPacketId::ItemRentalRequest as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ItemRentalRequest {
            name: r.read_string()?,
            renting: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.name);
        w.write_bool(self.renting);
    }
}

// ----------------------------- ID 256: ItemRentalFee -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemRentalFee {
    pub amount: u32,
}

impl PacketCodec for ItemRentalFee {
    const ID: i16 = ServerPacketId::ItemRentalFee as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ItemRentalFee {
            amount: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.amount);
    }
}

// ----------------------------- ID 257: ItemRentalPeriod -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemRentalPeriod {
    pub days: u32,
}

impl PacketCodec for ItemRentalPeriod {
    const ID: i16 = ServerPacketId::ItemRentalPeriod as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ItemRentalPeriod {
            days: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.days);
    }
}

// ----------------------------- ID 258: DepositRentalItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DepositRentalItem {
    pub from: i32,
    pub to: i32,
    pub success: bool,
}

impl PacketCodec for DepositRentalItem {
    const ID: i16 = ServerPacketId::DepositRentalItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(DepositRentalItem {
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

// ----------------------------- ID 259: RetrieveRentalItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RetrieveRentalItem {
    pub from: i32,
    pub to: i32,
    pub success: bool,
}

impl PacketCodec for RetrieveRentalItem {
    const ID: i16 = ServerPacketId::RetrieveRentalItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RetrieveRentalItem {
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

// ----------------------------- ID 260: UpdateRentalItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UpdateRentalItem {
    /// C#: `HasData`/`LoanItem != null` 标志；true 时读一个 UserItem
    pub loan_item: Option<UserItem>,
}

impl PacketCodec for UpdateRentalItem {
    const ID: i16 = ServerPacketId::UpdateRentalItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let has_data = r.read_bool()?;
        let loan_item = if has_data {
            Some(UserItem::read(r)?)
        } else {
            None
        };
        Ok(UpdateRentalItem { loan_item })
    }

    fn write(&self, w: &mut Writer) {
        match &self.loan_item {
            Some(item) => {
                w.write_bool(true);
                item.write(w);
            }
            None => w.write_bool(false),
        }
    }
}

// ----------------------------- ID 261: CancelItemRental -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CancelItemRental;

impl PacketCodec for CancelItemRental {
    const ID: i16 = ServerPacketId::CancelItemRental as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(CancelItemRental)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 262: ItemRentalLock -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemRentalLock {
    pub success: bool,
    pub gold_locked: bool,
    pub item_locked: bool,
}

impl PacketCodec for ItemRentalLock {
    const ID: i16 = ServerPacketId::ItemRentalLock as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ItemRentalLock {
            success: r.read_bool()?,
            gold_locked: r.read_bool()?,
            item_locked: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_bool(self.success);
        w.write_bool(self.gold_locked);
        w.write_bool(self.item_locked);
    }
}

// ----------------------------- ID 263: ItemRentalPartnerLock -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemRentalPartnerLock {
    pub gold_locked: bool,
    pub item_locked: bool,
}

impl PacketCodec for ItemRentalPartnerLock {
    const ID: i16 = ServerPacketId::ItemRentalPartnerLock as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ItemRentalPartnerLock {
            gold_locked: r.read_bool()?,
            item_locked: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_bool(self.gold_locked);
        w.write_bool(self.item_locked);
    }
}

// ----------------------------- ID 264: CanConfirmItemRental -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CanConfirmItemRental;

impl PacketCodec for CanConfirmItemRental {
    const ID: i16 = ServerPacketId::CanConfirmItemRental as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(CanConfirmItemRental)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 265: ConfirmItemRental -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfirmItemRental;

impl PacketCodec for ConfirmItemRental {
    const ID: i16 = ServerPacketId::ConfirmItemRental as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(ConfirmItemRental)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 266: NewRecipeInfo -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewRecipeInfo {
    pub info: ClientRecipeInfo,
}

impl PacketCodec for NewRecipeInfo {
    const ID: i16 = ServerPacketId::NewRecipeInfo as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(NewRecipeInfo {
            info: ClientRecipeInfo::read(r)?,
        })
    }

    fn write(&self, w: &mut Writer) {
        self.info.write(w);
    }
}

// ----------------------------- ID 112: CraftItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CraftItem {
    pub success: bool,
}

impl PacketCodec for CraftItem {
    const ID: i16 = ServerPacketId::CraftItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(CraftItem {
            success: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_bool(self.success);
    }
}

// ----------------------------- ID 267: OpenBrowser -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OpenBrowser {
    pub url: String,
}

impl PacketCodec for OpenBrowser {
    const ID: i16 = ServerPacketId::OpenBrowser as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(OpenBrowser {
            url: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.url);
    }
}

// ----------------------------- ID 268: PlaySound -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlaySound {
    pub sound: i32,
}

impl PacketCodec for PlaySound {
    const ID: i16 = ServerPacketId::PlaySound as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(PlaySound {
            sound: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.sound);
    }
}

// ----------------------------- ID 269: SetTimer -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SetTimer {
    pub key: String,
    /// TimerType (u8)
    pub r#type: u8,
    pub seconds: i32,
}

impl PacketCodec for SetTimer {
    const ID: i16 = ServerPacketId::SetTimer as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(SetTimer {
            key: r.read_string()?,
            r#type: r.read_u8()?,
            seconds: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.key);
        w.write_u8(self.r#type);
        w.write_i32(self.seconds);
    }
}

// ----------------------------- ID 270: ExpireTimer -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExpireTimer {
    pub key: String,
}

impl PacketCodec for ExpireTimer {
    const ID: i16 = ServerPacketId::ExpireTimer as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ExpireTimer {
            key: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.key);
    }
}

// ----------------------------- ID 271: UpdateNotice -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UpdateNotice {
    pub notice: Notice,
}

impl PacketCodec for UpdateNotice {
    const ID: i16 = ServerPacketId::UpdateNotice as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(UpdateNotice {
            notice: Notice::read(r)?,
        })
    }

    fn write(&self, w: &mut Writer) {
        self.notice.write(w);
    }
}

// ----------------------------- ID 272: Roll -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Roll {
    pub r#type: i32,
    pub page: String,
    pub result: i32,
    pub auto_roll: bool,
}

impl PacketCodec for Roll {
    const ID: i16 = ServerPacketId::Roll as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Roll {
            r#type: r.read_i32()?,
            page: r.read_string()?,
            result: r.read_i32()?,
            auto_roll: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.r#type);
        w.write_string(&self.page);
        w.write_i32(self.result);
        w.write_bool(self.auto_roll);
    }
}

// ----------------------------- ID 273: SetCompass -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SetCompass {
    pub location: Point,
}

impl PacketCodec for SetCompass {
    const ID: i16 = ServerPacketId::SetCompass as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(SetCompass {
            location: Point::read(r)?,
        })
    }

    fn write(&self, w: &mut Writer) {
        self.location.write(w);
    }
}

// ----------------------------- ID 33: NewMonsterInfo -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewMonsterInfo {
    pub info: ClientMonsterInfo,
}

impl PacketCodec for NewMonsterInfo {
    const ID: i16 = ServerPacketId::NewMonsterInfo as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(NewMonsterInfo {
            info: ClientMonsterInfo::read(r)?,
        })
    }

    fn write(&self, w: &mut Writer) {
        self.info.write(w);
    }
}

// ----------------------------- ID 34: NewNPCInfo -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewNPCInfo {
    /// 对应 C# `ClientNPCInfo`（types.rs 中为 ClientNpcInfo，字段顺序一致）
    pub info: ClientNpcInfo,
}

impl PacketCodec for NewNPCInfo {
    const ID: i16 = ServerPacketId::NewNPCInfo as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(NewNPCInfo {
            info: ClientNpcInfo::read(r)?,
        })
    }

    fn write(&self, w: &mut Writer) {
        self.info.write(w);
    }
}
