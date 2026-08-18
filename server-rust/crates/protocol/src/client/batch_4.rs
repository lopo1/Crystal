// batch_4 —— 客户端→服务器包（对应 `Shared/ClientPackets.cs` 行 2208–2704，最后一节）
//
// 覆盖: LockMail .. DeleteItem（ClientPacketId 121–146 及 149）。
// 枚举/动作字段一律按线上原文保留为原始整数（u8/u32/i32），注释注明 C# 侧含义。

use crate::binary::{Point, Reader, Writer};
use crate::frame::PacketCodec;
use crate::ids::ClientPacketId;
use crate::types::*;
use crate::Result;

// ----------------------------- ID 121: LockMail -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LockMail {
    pub mail_id: u64,
    pub lock: bool,
}

impl PacketCodec for LockMail {
    const ID: i16 = ClientPacketId::LockMail as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(LockMail {
            mail_id: r.read_u64()?,
            lock: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u64(self.mail_id);
        w.write_bool(self.lock);
    }
}

// ----------------------------- ID 122: MailLockedItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MailLockedItem {
    pub unique_id: u64,
    pub locked: bool,
}

impl PacketCodec for MailLockedItem {
    const ID: i16 = ClientPacketId::MailLockedItem as i16;

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

// ----------------------------- ID 123: MailCost -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MailCost {
    pub gold: u32,
    /// 固定 5 个物品索引（C# `ulong[5]`，无长度前缀）
    pub items_idx: [u64; 5],
    pub stamped: bool,
}

impl PacketCodec for MailCost {
    const ID: i16 = ClientPacketId::MailCost as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let gold = r.read_u32()?;
        let mut items_idx = [0u64; 5];
        for slot in items_idx.iter_mut() {
            *slot = r.read_u64()?;
        }
        let stamped = r.read_bool()?;
        Ok(MailCost {
            gold,
            items_idx,
            stamped,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.gold);
        for idx in &self.items_idx {
            w.write_u64(*idx);
        }
        w.write_bool(self.stamped);
    }
}

// ----------------------------- ID 126: RequestIntelligentCreatureUpdates -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestIntelligentCreatureUpdates {
    pub update: bool,
}

impl PacketCodec for RequestIntelligentCreatureUpdates {
    const ID: i16 = ClientPacketId::RequestIntelligentCreatureUpdates as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RequestIntelligentCreatureUpdates {
            update: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_bool(self.update);
    }
}

// ----------------------------- ID 124: UpdateIntelligentCreature -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UpdateIntelligentCreature {
    pub creature: ClientIntelligentCreature,
    pub summon_me: bool,
    pub un_summon_me: bool,
    pub release_me: bool,
}

impl PacketCodec for UpdateIntelligentCreature {
    const ID: i16 = ClientPacketId::UpdateIntelligentCreature as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(UpdateIntelligentCreature {
            creature: ClientIntelligentCreature::read(r)?,
            summon_me: r.read_bool()?,
            un_summon_me: r.read_bool()?,
            release_me: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        self.creature.write(w);
        w.write_bool(self.summon_me);
        w.write_bool(self.un_summon_me);
        w.write_bool(self.release_me);
    }
}

// ----------------------------- ID 125: IntelligentCreaturePickup -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IntelligentCreaturePickup {
    pub mouse_mode: bool,
    pub location: Point,
}

impl PacketCodec for IntelligentCreaturePickup {
    const ID: i16 = ClientPacketId::IntelligentCreaturePickup as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(IntelligentCreaturePickup {
            mouse_mode: r.read_bool()?,
            location: Point::read(r)?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_bool(self.mouse_mode);
        self.location.write(w);
    }
}

// ----------------------------- ID 127: AddFriend -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AddFriend {
    pub name: String,
    pub blocked: bool,
}

impl PacketCodec for AddFriend {
    const ID: i16 = ClientPacketId::AddFriend as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(AddFriend {
            name: r.read_string()?,
            blocked: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.name);
        w.write_bool(self.blocked);
    }
}

// ----------------------------- ID 128: RemoveFriend -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RemoveFriend {
    pub character_index: i32,
}

impl PacketCodec for RemoveFriend {
    const ID: i16 = ClientPacketId::RemoveFriend as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RemoveFriend {
            character_index: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.character_index);
    }
}

// ----------------------------- ID 129: RefreshFriends -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RefreshFriends;

impl PacketCodec for RefreshFriends {
    const ID: i16 = ClientPacketId::RefreshFriends as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(RefreshFriends)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 130: AddMemo -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AddMemo {
    pub character_index: i32,
    pub memo: String,
}

impl PacketCodec for AddMemo {
    const ID: i16 = ClientPacketId::AddMemo as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(AddMemo {
            character_index: r.read_i32()?,
            memo: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.character_index);
        w.write_string(&self.memo);
    }
}

// ----------------------------- ID 131: GuildBuffUpdate -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuildBuffUpdate {
    /// 动作（C# 侧无命名枚举，原始 byte）: 0 = 请求列表, 1 = 请求启用 buff, 2 = 请求激活 buff
    pub action: u8,
    pub id: i32,
}

impl PacketCodec for GuildBuffUpdate {
    const ID: i16 = ClientPacketId::GuildBuffUpdate as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(GuildBuffUpdate {
            action: r.read_u8()?,
            id: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.action);
        w.write_i32(self.id);
    }
}

// ----------------------------- ID 133: GameshopBuy -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameshopBuy {
    pub g_index: i32,
    pub quantity: u8,
    /// 支付类型（C# 侧无命名枚举，原始 int）
    pub p_type: i32,
}

impl PacketCodec for GameshopBuy {
    const ID: i16 = ClientPacketId::GameshopBuy as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(GameshopBuy {
            g_index: r.read_i32()?,
            quantity: r.read_u8()?,
            p_type: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.g_index);
        w.write_u8(self.quantity);
        w.write_i32(self.p_type);
    }
}

// ----------------------------- ID 132: NPCConfirmInput -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NPCConfirmInput {
    pub npcid: u32,
    pub page_name: String,
    pub value: String,
}

impl PacketCodec for NPCConfirmInput {
    const ID: i16 = ClientPacketId::NPCConfirmInput as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(NPCConfirmInput {
            npcid: r.read_u32()?,
            page_name: r.read_string()?,
            value: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.npcid);
        w.write_string(&self.page_name);
        w.write_string(&self.value);
    }
}

// ----------------------------- ID 134: ReportIssue -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReportIssue {
    /// 截图字节（C# `byte[] Image`，int32 长度前缀 + 原始字节，无 padding）
    pub image: Vec<u8>,
    pub image_size: i32,
    pub image_chunk: i32,
}

impl PacketCodec for ReportIssue {
    const ID: i16 = ClientPacketId::ReportIssue as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let len = r.read_i32()?;
        let image = r.read_bytes(len.max(0) as usize)?;
        Ok(ReportIssue {
            image,
            image_size: r.read_i32()?,
            image_chunk: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.image.len() as i32);
        w.write_bytes(&self.image);
        w.write_i32(self.image_size);
        w.write_i32(self.image_chunk);
    }
}

// ----------------------------- ID 135: GetRanking -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GetRanking {
    /// RankType（C# 侧无命名枚举，原始 byte）
    pub rank_type: u8,
    pub rank_index: i32,
    pub online_only: bool,
}

impl PacketCodec for GetRanking {
    const ID: i16 = ClientPacketId::GetRanking as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(GetRanking {
            rank_type: r.read_u8()?,
            rank_index: r.read_i32()?,
            online_only: r.read_bool()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.rank_type);
        w.write_i32(self.rank_index);
        w.write_bool(self.online_only);
    }
}

// ----------------------------- ID 136: Opendoor -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Opendoor {
    pub door_index: u8,
}

impl PacketCodec for Opendoor {
    const ID: i16 = ClientPacketId::Opendoor as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Opendoor {
            door_index: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.door_index);
    }
}

// ----------------------------- ID 137: GetRentedItems -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GetRentedItems;

impl PacketCodec for GetRentedItems {
    const ID: i16 = ClientPacketId::GetRentedItems as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(GetRentedItems)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 138: ItemRentalRequest -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemRentalRequest;

impl PacketCodec for ItemRentalRequest {
    const ID: i16 = ClientPacketId::ItemRentalRequest as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(ItemRentalRequest)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 139: ItemRentalFee -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemRentalFee {
    pub amount: u32,
}

impl PacketCodec for ItemRentalFee {
    const ID: i16 = ClientPacketId::ItemRentalFee as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ItemRentalFee {
            amount: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.amount);
    }
}

// ----------------------------- ID 140: ItemRentalPeriod -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemRentalPeriod {
    pub days: u32,
}

impl PacketCodec for ItemRentalPeriod {
    const ID: i16 = ClientPacketId::ItemRentalPeriod as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ItemRentalPeriod {
            days: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.days);
    }
}

// ----------------------------- ID 141: DepositRentalItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DepositRentalItem {
    pub from: i32,
    pub to: i32,
}

impl PacketCodec for DepositRentalItem {
    const ID: i16 = ClientPacketId::DepositRentalItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(DepositRentalItem {
            from: r.read_i32()?,
            to: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.from);
        w.write_i32(self.to);
    }
}

// ----------------------------- ID 142: RetrieveRentalItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RetrieveRentalItem {
    pub from: i32,
    pub to: i32,
}

impl PacketCodec for RetrieveRentalItem {
    const ID: i16 = ClientPacketId::RetrieveRentalItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RetrieveRentalItem {
            from: r.read_i32()?,
            to: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.from);
        w.write_i32(self.to);
    }
}

// ----------------------------- ID 143: CancelItemRental -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CancelItemRental;

impl PacketCodec for CancelItemRental {
    const ID: i16 = ClientPacketId::CancelItemRental as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(CancelItemRental)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 144: ItemRentalLockFee -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemRentalLockFee;

impl PacketCodec for ItemRentalLockFee {
    const ID: i16 = ClientPacketId::ItemRentalLockFee as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(ItemRentalLockFee)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 145: ItemRentalLockItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemRentalLockItem;

impl PacketCodec for ItemRentalLockItem {
    const ID: i16 = ClientPacketId::ItemRentalLockItem as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(ItemRentalLockItem)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 146: ConfirmItemRental -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfirmItemRental;

impl PacketCodec for ConfirmItemRental {
    const ID: i16 = ClientPacketId::ConfirmItemRental as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(ConfirmItemRental)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 149: DeleteItem -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeleteItem {
    pub unique_id: u64,
    pub count: u16,
    pub hero_inventory: bool,
}

impl PacketCodec for DeleteItem {
    const ID: i16 = ClientPacketId::DeleteItem as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(DeleteItem {
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
