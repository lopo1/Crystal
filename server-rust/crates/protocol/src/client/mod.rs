//! 客户端→服务器数据包（对应 `Shared/ClientPackets.cs`）。
//!
//! 逐个移植中（当前覆盖登录/角色/移动/聊天核心包）。

use crate::binary::{Reader, Writer};
use crate::frame::PacketCodec;
use crate::ids::ClientPacketId;
use crate::types::{ChatItem, MirClass, MirDirection, MirGender};
use crate::Result;

// ----------------------------- ID 0: ClientVersion -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientVersion {
    pub version_hash: Vec<u8>,
}

impl PacketCodec for ClientVersion {
    const ID: i16 = ClientPacketId::ClientVersion as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let len = r.read_i32()?;
        Ok(ClientVersion {
            version_hash: r.read_bytes(len.max(0) as usize)?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.version_hash.len() as i32);
        w.write_bytes(&self.version_hash);
    }
}

// ----------------------------- ID 1: Disconnect -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Disconnect;

impl PacketCodec for Disconnect {
    const ID: i16 = ClientPacketId::Disconnect as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(Disconnect)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 2: KeepAlive -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeepAlive {
    pub time: i64,
}

impl PacketCodec for KeepAlive {
    const ID: i16 = ClientPacketId::KeepAlive as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(KeepAlive {
            time: r.read_i64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i64(self.time);
    }
}

// ----------------------------- ID 3: NewAccount -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewAccount {
    pub account_id: String,
    pub password: String,
    /// .NET DateTime binary（ToBinary / FromBinary）
    pub birth_date: i64,
    pub user_name: String,
    pub secret_question: String,
    pub secret_answer: String,
    pub email_address: String,
}

impl PacketCodec for NewAccount {
    const ID: i16 = ClientPacketId::NewAccount as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(NewAccount {
            account_id: r.read_string()?,
            password: r.read_string()?,
            birth_date: r.read_i64()?,
            user_name: r.read_string()?,
            secret_question: r.read_string()?,
            secret_answer: r.read_string()?,
            email_address: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.account_id);
        w.write_string(&self.password);
        w.write_i64(self.birth_date);
        w.write_string(&self.user_name);
        w.write_string(&self.secret_question);
        w.write_string(&self.secret_answer);
        w.write_string(&self.email_address);
    }
}

// ----------------------------- ID 4: ChangePassword -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangePassword {
    pub account_id: String,
    pub current_password: String,
    pub new_password: String,
}

impl PacketCodec for ChangePassword {
    const ID: i16 = ClientPacketId::ChangePassword as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ChangePassword {
            account_id: r.read_string()?,
            current_password: r.read_string()?,
            new_password: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.account_id);
        w.write_string(&self.current_password);
        w.write_string(&self.new_password);
    }
}

// ----------------------------- ID 5: Login -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Login {
    pub account_id: String,
    pub password: String,
}

impl PacketCodec for Login {
    const ID: i16 = ClientPacketId::Login as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Login {
            account_id: r.read_string()?,
            password: r.read_string()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.account_id);
        w.write_string(&self.password);
    }
}

// ----------------------------- ID 6: NewCharacter -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewCharacter {
    pub name: String,
    pub gender: MirGender,
    pub class: MirClass,
}

impl PacketCodec for NewCharacter {
    const ID: i16 = ClientPacketId::NewCharacter as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(NewCharacter {
            name: r.read_string()?,
            gender: match r.read_u8()? {
                0 => MirGender::Male,
                _ => MirGender::Female,
            },
            class: match r.read_u8()? {
                0 => MirClass::Warrior,
                1 => MirClass::Wizard,
                2 => MirClass::Taoist,
                3 => MirClass::Assassin,
                _ => MirClass::Archer,
            },
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.name);
        w.write_u8(self.gender as u8);
        w.write_u8(self.class as u8);
    }
}

// ----------------------------- ID 7: DeleteCharacter -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeleteCharacter {
    pub character_index: i32,
}

impl PacketCodec for DeleteCharacter {
    const ID: i16 = ClientPacketId::DeleteCharacter as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(DeleteCharacter {
            character_index: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.character_index);
    }
}

// ----------------------------- ID 8: StartGame -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StartGame {
    pub character_index: i32,
}

impl PacketCodec for StartGame {
    const ID: i16 = ClientPacketId::StartGame as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(StartGame {
            character_index: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.character_index);
    }
}

// ----------------------------- ID 9: LogOut -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogOut;

impl PacketCodec for LogOut {
    const ID: i16 = ClientPacketId::LogOut as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(LogOut)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 10-12: Turn / Walk / Run -----------------------------

macro_rules! direction_packet {
    ($name:ident, $id:expr) => {
        #[derive(Debug, Clone, Default, PartialEq, Eq)]
        pub struct $name {
            pub direction: MirDirection,
        }

        impl PacketCodec for $name {
            const ID: i16 = $id;

            fn read(r: &mut Reader) -> Result<Self> {
                Ok($name {
                    direction: MirDirection::from_u8(r.read_u8()?),
                })
            }

            fn write(&self, w: &mut Writer) {
                w.write_u8(self.direction.to_u8());
            }
        }
    };
}

direction_packet!(Turn, ClientPacketId::Turn as i16);
direction_packet!(Walk, ClientPacketId::Walk as i16);
direction_packet!(Run, ClientPacketId::Run as i16);

// ----------------------------- ID 13: Chat -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Chat {
    pub message: String,
    pub linked_items: Vec<ChatItem>,
}

impl PacketCodec for Chat {
    const ID: i16 = ClientPacketId::Chat as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let message = r.read_string()?;
        let count = r.read_i32()?;
        let mut linked_items = Vec::with_capacity(count.max(0) as usize);
        for _ in 0..count.max(0) {
            linked_items.push(ChatItem::read(r)?);
        }
        Ok(Chat {
            message,
            linked_items,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.message);
        w.write_i32(self.linked_items.len() as i32);
        for item in &self.linked_items {
            item.write(w);
        }
    }
}

// ----------------------------- 分发（宏驱动） -----------------------------

pub mod batch_1;
pub mod batch_2;
pub mod batch_3;
pub mod batch_4;
pub mod batch_7;
pub mod web3;

pub use batch_1::*;
pub use batch_2::*;
pub use batch_3::*;
pub use batch_4::*;
pub use batch_7::*;
pub use web3::*;

/// 客户端数据包分发宏: 由 (变体名 => 类型) 列表生成枚举与按 ID 解码。
/// 每移植一批，在 `client_packet_dispatch!` 调用中追加条目。
macro_rules! client_packet_dispatch {
    ($($v:ident => $t:ty),* $(,)?) => {
        #[derive(Debug, Clone, PartialEq)]
        pub enum ClientPacket {
            $( $v($t) ),*
        }
        impl ClientPacket {
            /// 按 ID 解码（未移植的 ID 返回 `InvalidPacketId`）
            pub fn decode(id: i16, payload: &[u8]) -> Result<Self> {
                use crate::frame::PacketCodec as _;
                $(
                    if id == <$t as PacketCodec>::ID {
                        return Ok(ClientPacket::$v(crate::frame::decode_packet::<$t>(id, payload)?));
                    }
                )*
                Err(crate::ProtocolError::InvalidPacketId(id))
            }
        }
    };
}

client_packet_dispatch! {
    ClientVersion => ClientVersion,
    Disconnect => Disconnect,
    KeepAlive => KeepAlive,
    NewAccount => NewAccount,
    ChangePassword => ChangePassword,
    Login => Login,
    NewCharacter => NewCharacter,
    DeleteCharacter => DeleteCharacter,
    StartGame => StartGame,
    LogOut => LogOut,
    Turn => Turn,
    Walk => Walk,
    Run => Run,
    Chat => Chat,
    MarketPage => MarketPage,
    MarketBuy => MarketBuy,
    MarketSellNow => MarketSellNow,
    MarketGetBack => MarketGetBack,
    RequestUserName => RequestUserName,
    RequestChatItem => RequestChatItem,
    EditGuildMember => EditGuildMember,
    EditGuildNotice => EditGuildNotice,
    GuildInvite => GuildInvite,
    RequestGuildInfo => RequestGuildInfo,
    GuildNameReturn => GuildNameReturn,
    GuildWarReturn => GuildWarReturn,
    EquipSlotItem => EquipSlotItem,
    FishingCast => FishingCast,
    FishingChangeAutocast => FishingChangeAutocast,
    AcceptQuest => AcceptQuest,
    FinishQuest => FinishQuest,
    AbandonQuest => AbandonQuest,
    ShareQuest => ShareQuest,
    AcceptReincarnation => AcceptReincarnation,
    CancelReincarnation => CancelReincarnation,
    CombineItem => CombineItem,
    AwakeningNeedMaterials => AwakeningNeedMaterials,
    AwakeningLockedItem => AwakeningLockedItem,
    Awakening => Awakening,
    DisassembleItem => DisassembleItem,
    DowngradeAwakening => DowngradeAwakening,
    ResetAddedItem => ResetAddedItem,
    SendMail => SendMail,
    ReadMail => ReadMail,
    CollectParcel => CollectParcel,
    DeleteMail => DeleteMail,
    LockMail => LockMail,
    MailLockedItem => MailLockedItem,
    MailCost => MailCost,
    RequestIntelligentCreatureUpdates => RequestIntelligentCreatureUpdates,
    UpdateIntelligentCreature => UpdateIntelligentCreature,
    IntelligentCreaturePickup => IntelligentCreaturePickup,
    AddFriend => AddFriend,
    RemoveFriend => RemoveFriend,
    RefreshFriends => RefreshFriends,
    AddMemo => AddMemo,
    GuildBuffUpdate => GuildBuffUpdate,
    GameshopBuy => GameshopBuy,
    NPCConfirmInput => NPCConfirmInput,
    ReportIssue => ReportIssue,
    GetRanking => GetRanking,
    Opendoor => Opendoor,
    GetRentedItems => GetRentedItems,
    ItemRentalRequest => ItemRentalRequest,
    ItemRentalFee => ItemRentalFee,
    ItemRentalPeriod => ItemRentalPeriod,
    DepositRentalItem => DepositRentalItem,
    RetrieveRentalItem => RetrieveRentalItem,
    CancelItemRental => CancelItemRental,
    ItemRentalLockFee => ItemRentalLockFee,
    ItemRentalLockItem => ItemRentalLockItem,
    ConfirmItemRental => ConfirmItemRental,
    DeleteItem => DeleteItem,
    MoveItem => MoveItem,
    StoreItem => StoreItem,
    DepositRefineItem => DepositRefineItem,
    RetrieveRefineItem => RetrieveRefineItem,
    RefineCancel => RefineCancel,
    RefineItem => RefineItem,
    CheckRefine => CheckRefine,
    ReplaceWedRing => ReplaceWedRing,
    DepositTradeItem => DepositTradeItem,
    RetrieveTradeItem => RetrieveTradeItem,
    TakeBackItem => TakeBackItem,
    MergeItem => MergeItem,
    EquipItem => EquipItem,
    RemoveItem => RemoveItem,
    RemoveSlotItem => RemoveSlotItem,
    SplitItem => SplitItem,
    UseItem => UseItem,
    DropItem => DropItem,
    TakeBackHeroItem => TakeBackHeroItem,
    TransferHeroItem => TransferHeroItem,
    DropGold => DropGold,
    PickUp => PickUp,
    Inspect => Inspect,
    Observe => Observe,
    ChangeAMode => ChangeAMode,
    ChangePMode => ChangePMode,
    ChangeTrade => ChangeTrade,
    Attack => Attack,
    RangeAttack => RangeAttack,
    Harvest => Harvest,
    CallNPC => CallNPC,
    BuyItem => BuyItem,
    SellItem => SellItem,
    CraftItem => CraftItem,
    RepairItem => RepairItem,
    BuyItemBack => BuyItemBack,
    SRepairItem => SRepairItem,
    RequestMapInfo => RequestMapInfo,
    RequestMonsterInfo => RequestMonsterInfo,
    RequestNPCInfo => RequestNPCInfo,
    RequestItemInfo => RequestItemInfo,
    TeleportToNPC => TeleportToNPC,
    SearchMap => SearchMap,
    MagicKey => MagicKey,
    Magic => Magic,
    SwitchGroup => SwitchGroup,
    AddMember => AddMember,
    DelMember => DelMember,
    GroupInvite => GroupInvite,
    NewHero => NewHero,
    SetAutoPotValue => SetAutoPotValue,
    SetAutoPotItem => SetAutoPotItem,
    SetHeroBehaviour => SetHeroBehaviour,
    ChangeHero => ChangeHero,
    MarriageRequest => MarriageRequest,
    MarriageReply => MarriageReply,
    ChangeMarriage => ChangeMarriage,
    DivorceRequest => DivorceRequest,
    DivorceReply => DivorceReply,
    AddMentor => AddMentor,
    MentorReply => MentorReply,
    AllowMentor => AllowMentor,
    CancelMentor => CancelMentor,
    TradeRequest => TradeRequest,
    TradeReply => TradeReply,
    TradeGold => TradeGold,
    TradeConfirm => TradeConfirm,
    TradeCancel => TradeCancel,
    TownRevive => TownRevive,
    SpellToggle => SpellToggle,
    ConsignItem => ConsignItem,
    GuildTerritoryPage => GuildTerritoryPage,
    PurchaseGuildTerritory => PurchaseGuildTerritory,
    MarketSearch => MarketSearch,
    MarketRefresh => MarketRefresh,
    UnlockStorage => UnlockStorage,
    SetStoragePassword => SetStoragePassword,
    RemoveStoragePassword => RemoveStoragePassword,
    GuildStorageGoldChange => GuildStorageGoldChange,
    GuildStorageItemChange => GuildStorageItemChange,
    // ---- Web3 钱包登录扩展（自定义 ID 200+）----
    Web3ChallengeRequest => Web3ChallengeRequest,
    Web3Login => Web3Login,
    Web3SessionLogin => Web3SessionLogin,
}
