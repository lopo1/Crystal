//! 服务器→客户端数据包（对应 `Shared/ServerPackets.cs`）。
//!
//! 逐个移植中（当前覆盖登录/角色/进世界/移动/聊天核心包）。

use crate::binary::{Argb, Point, Reader, Writer};
use crate::frame::PacketCodec;
use crate::ids::ServerPacketId;
use crate::types::{
    ClientIntelligentCreature, ClientMagic, ClientMapInfo, LevelEffects, MirClass, MirDirection,
    MirGender, MirGridType, SelectInfo, UserItem, WorldMapSetup,
};
use crate::Result;

/// 可空物品槽（同 C# 的 `UserItem[]`，元素可 null）
pub type ItemSlots = Vec<Option<UserItem>>;

fn read_item_slots(r: &mut Reader) -> Result<Option<ItemSlots>> {
    if !r.read_bool()? {
        return Ok(None);
    }
    let len = r.read_i32()?;
    let mut slots = Vec::with_capacity(len.max(0) as usize);
    for _ in 0..len.max(0) {
        if r.read_bool()? {
            slots.push(None);
        } else {
            slots.push(Some(UserItem::read(r)?));
        }
    }
    Ok(Some(slots))
}

fn write_item_slots(w: &mut Writer, slots: &Option<ItemSlots>) {
    match slots {
        None => w.write_bool(false),
        Some(items) => {
            w.write_bool(true);
            w.write_i32(items.len() as i32);
            for item in items {
                match item {
                    None => w.write_bool(true),
                    Some(item) => {
                        w.write_bool(false);
                        item.write(w);
                    }
                }
            }
        }
    }
}

// ----------------------------- ID 0: Connected -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Connected;

impl PacketCodec for Connected {
    const ID: i16 = ServerPacketId::Connected as i16;

    fn read(_r: &mut Reader) -> Result<Self> {
        Ok(Connected)
    }

    fn write(&self, _w: &mut Writer) {}
}

// ----------------------------- ID 1: ClientVersion -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientVersion {
    /// 0 错误版本; 1 正确版本
    pub result: u8,
}

impl PacketCodec for ClientVersion {
    const ID: i16 = ServerPacketId::ClientVersion as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ClientVersion {
            result: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.result);
    }
}

// ----------------------------- ID 2: Disconnect -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Disconnect {
    /// 0 服务器关闭; 1 顶号; 2 包错误; 3 服务器崩溃
    pub reason: u8,
}

impl PacketCodec for Disconnect {
    const ID: i16 = ServerPacketId::Disconnect as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Disconnect {
            reason: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.reason);
    }
}

// ----------------------------- ID 3: KeepAlive -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeepAlive {
    pub time: i64,
}

impl PacketCodec for KeepAlive {
    const ID: i16 = ServerPacketId::KeepAlive as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(KeepAlive {
            time: r.read_i64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i64(self.time);
    }
}

// ----------------------------- ID 4: NewAccount -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewAccount {
    /// 0 禁用; 1 账号名非法; 2 密码非法; 3 邮箱非法; 4 名字非法;
    /// 5 问题非法; 6 答案非法; 7 账号已存在; 8 成功
    pub result: u8,
}

impl PacketCodec for NewAccount {
    const ID: i16 = ServerPacketId::NewAccount as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(NewAccount {
            result: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.result);
    }
}

// ----------------------------- ID 5: ChangePassword -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangePassword {
    pub result: u8,
}

impl PacketCodec for ChangePassword {
    const ID: i16 = ServerPacketId::ChangePassword as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ChangePassword {
            result: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.result);
    }
}

// ----------------------------- ID 6: ChangePasswordBanned -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangePasswordBanned {
    pub reason: String,
    pub expiry_date: i64,
}

impl PacketCodec for ChangePasswordBanned {
    const ID: i16 = ServerPacketId::ChangePasswordBanned as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ChangePasswordBanned {
            reason: r.read_string()?,
            expiry_date: r.read_i64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.reason);
        w.write_i64(self.expiry_date);
    }
}

// ----------------------------- ID 7: Login -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Login {
    /// 0 禁用; 1 账号名非法; 2 密码非法; 3 账号不存在; 4 密码错误
    pub result: u8,
}

impl PacketCodec for Login {
    const ID: i16 = ServerPacketId::Login as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Login {
            result: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.result);
    }
}

// ----------------------------- ID 8: LoginBanned -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoginBanned {
    pub reason: String,
    pub expiry_date: i64,
}

impl PacketCodec for LoginBanned {
    const ID: i16 = ServerPacketId::LoginBanned as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(LoginBanned {
            reason: r.read_string()?,
            expiry_date: r.read_i64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.reason);
        w.write_i64(self.expiry_date);
    }
}

// ----------------------------- ID 9: LoginSuccess -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoginSuccess {
    pub characters: Vec<SelectInfo>,
}

impl PacketCodec for LoginSuccess {
    const ID: i16 = ServerPacketId::LoginSuccess as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let count = r.read_i32()?;
        let mut characters = Vec::with_capacity(count.max(0) as usize);
        for _ in 0..count.max(0) {
            characters.push(SelectInfo::read(r)?);
        }
        Ok(LoginSuccess { characters })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.characters.len() as i32);
        for c in &self.characters {
            c.write(w);
        }
    }
}

// ----------------------------- ID 10: NewCharacter -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewCharacter {
    /// 0 禁用; 1 名字非法; 2 性别非法; 3 职业非法; 4 角色满; 5 角色已存在; 10 成功
    pub result: u8,
}

impl PacketCodec for NewCharacter {
    const ID: i16 = ServerPacketId::NewCharacter as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(NewCharacter {
            result: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.result);
    }
}

// ----------------------------- ID 11: NewCharacterSuccess -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewCharacterSuccess {
    pub char_info: SelectInfo,
}

impl PacketCodec for NewCharacterSuccess {
    const ID: i16 = ServerPacketId::NewCharacterSuccess as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(NewCharacterSuccess {
            char_info: SelectInfo::read(r)?,
        })
    }

    fn write(&self, w: &mut Writer) {
        self.char_info.write(w);
    }
}

// ----------------------------- ID 12: DeleteCharacter -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeleteCharacter {
    /// 0 禁用; 1 角色不存在
    pub result: u8,
}

impl PacketCodec for DeleteCharacter {
    const ID: i16 = ServerPacketId::DeleteCharacter as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(DeleteCharacter {
            result: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.result);
    }
}

// ----------------------------- ID 13: DeleteCharacterSuccess -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeleteCharacterSuccess {
    pub character_index: i32,
}

impl PacketCodec for DeleteCharacterSuccess {
    const ID: i16 = ServerPacketId::DeleteCharacterSuccess as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(DeleteCharacterSuccess {
            character_index: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.character_index);
    }
}

// ----------------------------- ID 14: StartGame -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StartGame {
    /// 0 禁用; 1 未登录; 2 角色不存在; 3 开服错误
    pub result: u8,
    pub resolution: i32,
}

impl PacketCodec for StartGame {
    const ID: i16 = ServerPacketId::StartGame as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(StartGame {
            result: r.read_u8()?,
            resolution: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u8(self.result);
        w.write_i32(self.resolution);
    }
}

// ----------------------------- ID 15: StartGameBanned -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StartGameBanned {
    pub reason: String,
    pub expiry_date: i64,
}

impl PacketCodec for StartGameBanned {
    const ID: i16 = ServerPacketId::StartGameBanned as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(StartGameBanned {
            reason: r.read_string()?,
            expiry_date: r.read_i64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.reason);
        w.write_i64(self.expiry_date);
    }
}

// ----------------------------- ID 16: StartGameDelay -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StartGameDelay {
    pub milliseconds: i64,
}

impl PacketCodec for StartGameDelay {
    const ID: i16 = ServerPacketId::StartGameDelay as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(StartGameDelay {
            milliseconds: r.read_i64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i64(self.milliseconds);
    }
}

// ----------------------------- ID 17: MapInformation -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MapInformation {
    pub map_index: i32,
    pub file_name: String,
    pub title: String,
    pub mini_map: u16,
    pub big_map: u16,
    /// LightSetting (u8)
    pub lights: u8,
    pub lightning: bool,
    pub fire: bool,
    pub map_dark_light: u8,
    pub music: u16,
    /// WeatherSetting (u16)
    pub weather_particles: u16,
}

impl PacketCodec for MapInformation {
    const ID: i16 = ServerPacketId::MapInformation as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let map_index = r.read_i32()?;
        let file_name = r.read_string()?;
        let title = r.read_string()?;
        let mini_map = r.read_u16()?;
        let big_map = r.read_u16()?;
        let lights = r.read_u8()?;
        let bools = r.read_u8()?;
        let lightning = bools & 0x01 == 0x01;
        let fire = bools & 0x02 == 0x02;
        let map_dark_light = r.read_u8()?;
        let music = r.read_u16()?;
        let weather_particles = r.read_u16()?;
        Ok(MapInformation {
            map_index,
            file_name,
            title,
            mini_map,
            big_map,
            lights,
            lightning,
            fire,
            map_dark_light,
            music,
            weather_particles,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.map_index);
        w.write_string(&self.file_name);
        w.write_string(&self.title);
        w.write_u16(self.mini_map);
        w.write_u16(self.big_map);
        w.write_u8(self.lights);
        let mut bools = 0u8;
        if self.lightning {
            bools |= 0x01;
        }
        if self.fire {
            bools |= 0x02;
        }
        w.write_u8(bools);
        w.write_u8(self.map_dark_light);
        w.write_u16(self.music);
        w.write_u16(self.weather_particles);
    }
}

// ----------------------------- ID 18: NewMapInfo -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewMapInfo {
    pub map_index: i32,
    pub info: ClientMapInfo,
}

impl PacketCodec for NewMapInfo {
    const ID: i16 = ServerPacketId::NewMapInfo as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(NewMapInfo {
            map_index: r.read_i32()?,
            info: ClientMapInfo::read(r)?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.map_index);
        self.info.write(w);
    }
}

// ----------------------------- ID 19: WorldMapSetupInfo -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorldMapSetupInfo {
    pub setup: WorldMapSetup,
    pub teleport_to_npc_cost: i32,
}

impl PacketCodec for WorldMapSetupInfo {
    const ID: i16 = ServerPacketId::WorldMapSetup as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(WorldMapSetupInfo {
            setup: WorldMapSetup::read(r)?,
            teleport_to_npc_cost: r.read_i32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        self.setup.write(w);
        w.write_i32(self.teleport_to_npc_cost);
    }
}

// ----------------------------- ID 20: SearchMapResult -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchMapResult {
    pub map_index: i32,
    pub npc_index: u32,
}

impl PacketCodec for SearchMapResult {
    const ID: i16 = ServerPacketId::SearchMapResult as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(SearchMapResult {
            map_index: r.read_i32()?,
            npc_index: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.map_index);
        w.write_u32(self.npc_index);
    }
}

// ----------------------------- ID 21: UserInformation -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserInformation {
    pub object_id: u32,
    pub real_id: u32,
    pub name: String,
    pub guild_name: String,
    pub guild_rank: String,
    pub name_colour: Argb,
    pub class: MirClass,
    pub gender: MirGender,
    pub level: u16,
    pub location: Point,
    pub direction: MirDirection,
    pub hair: u8,
    pub hp: i32,
    pub mp: i32,
    pub experience: i64,
    pub max_experience: i64,
    pub level_effects: LevelEffects,
    pub has_hero: bool,
    pub hero_behaviour: u8,
    pub inventory: Option<ItemSlots>,
    pub equipment: Option<ItemSlots>,
    pub quest_inventory: Option<ItemSlots>,
    pub gold: u32,
    pub credit: u32,
    pub has_expanded_storage: bool,
    pub has_storage_password: bool,
    pub require_storage_password: bool,
    pub storage_password_last_set: i64,
    pub expanded_storage_expiry_time: i64,
    pub magics: Vec<ClientMagic>,
    pub intelligent_creatures: Vec<ClientIntelligentCreature>,
    pub summoned_creature_type: u8,
    pub creature_summoned: bool,
    pub allow_observe: bool,
    pub observer: bool,
}

impl PacketCodec for UserInformation {
    const ID: i16 = ServerPacketId::UserInformation as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let object_id = r.read_u32()?;
        let real_id = r.read_u32()?;
        let name = r.read_string()?;
        let guild_name = r.read_string()?;
        let guild_rank = r.read_string()?;
        let name_colour = Argb::from_i32(r.read_i32()?);
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
        let location = Point::read(r)?;
        let direction = MirDirection::from_u8(r.read_u8()?);
        let hair = r.read_u8()?;
        let hp = r.read_i32()?;
        let mp = r.read_i32()?;
        let experience = r.read_i64()?;
        let max_experience = r.read_i64()?;
        let level_effects = LevelEffects(r.read_u16()?);
        let has_hero = r.read_bool()?;
        let hero_behaviour = r.read_u8()?;

        let inventory = read_item_slots(r)?;
        let equipment = read_item_slots(r)?;
        let quest_inventory = read_item_slots(r)?;

        let gold = r.read_u32()?;
        let credit = r.read_u32()?;
        let has_expanded_storage = r.read_bool()?;
        let has_storage_password = r.read_bool()?;
        let require_storage_password = r.read_bool()?;
        let storage_password_last_set = r.read_i64()?;
        let expanded_storage_expiry_time = r.read_i64()?;

        let mut magics = Vec::new();
        let mcount = r.read_i32()?;
        for _ in 0..mcount.max(0) {
            magics.push(ClientMagic::read(r)?);
        }
        let mut intelligent_creatures = Vec::new();
        let ccount = r.read_i32()?;
        for _ in 0..ccount.max(0) {
            intelligent_creatures.push(ClientIntelligentCreature::read(r)?);
        }
        let summoned_creature_type = r.read_u8()?;
        let creature_summoned = r.read_bool()?;
        let allow_observe = r.read_bool()?;
        let observer = r.read_bool()?;

        Ok(UserInformation {
            object_id,
            real_id,
            name,
            guild_name,
            guild_rank,
            name_colour,
            class,
            gender,
            level,
            location,
            direction,
            hair,
            hp,
            mp,
            experience,
            max_experience,
            level_effects,
            has_hero,
            hero_behaviour,
            inventory,
            equipment,
            quest_inventory,
            gold,
            credit,
            has_expanded_storage,
            has_storage_password,
            require_storage_password,
            storage_password_last_set,
            expanded_storage_expiry_time,
            magics,
            intelligent_creatures,
            summoned_creature_type,
            creature_summoned,
            allow_observe,
            observer,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_u32(self.real_id);
        w.write_string(&self.name);
        w.write_string(&self.guild_name);
        w.write_string(&self.guild_rank);
        w.write_i32(self.name_colour.to_i32());
        w.write_u8(self.class as u8);
        w.write_u8(self.gender as u8);
        w.write_u16(self.level);
        self.location.write(w);
        w.write_u8(self.direction.to_u8());
        w.write_u8(self.hair);
        w.write_i32(self.hp);
        w.write_i32(self.mp);
        w.write_i64(self.experience);
        w.write_i64(self.max_experience);
        w.write_u16(self.level_effects.0);
        w.write_bool(self.has_hero);
        w.write_u8(self.hero_behaviour);

        write_item_slots(w, &self.inventory);
        write_item_slots(w, &self.equipment);
        write_item_slots(w, &self.quest_inventory);

        w.write_u32(self.gold);
        w.write_u32(self.credit);
        w.write_bool(self.has_expanded_storage);
        w.write_bool(self.has_storage_password);
        w.write_bool(self.require_storage_password);
        w.write_i64(self.storage_password_last_set);
        w.write_i64(self.expanded_storage_expiry_time);

        w.write_i32(self.magics.len() as i32);
        for m in &self.magics {
            m.write(w);
        }
        w.write_i32(self.intelligent_creatures.len() as i32);
        for c in &self.intelligent_creatures {
            c.write(w);
        }
        w.write_u8(self.summoned_creature_type);
        w.write_bool(self.creature_summoned);
        w.write_bool(self.allow_observe);
        w.write_bool(self.observer);
    }
}

// ----------------------------- ID 22: UserSlotsRefresh -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserSlotsRefresh {
    pub inventory: Option<ItemSlots>,
    pub equipment: Option<ItemSlots>,
}

impl PacketCodec for UserSlotsRefresh {
    const ID: i16 = ServerPacketId::UserSlotsRefresh as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let inventory = read_item_slots(r)?;
        let equipment = read_item_slots(r)?;
        Ok(UserSlotsRefresh {
            inventory,
            equipment,
        })
    }

    fn write(&self, w: &mut Writer) {
        write_item_slots(w, &self.inventory);
        write_item_slots(w, &self.equipment);
    }
}

// ----------------------------- ID 23: UserLocation -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserLocation {
    pub location: Point,
    pub direction: MirDirection,
}

impl PacketCodec for UserLocation {
    const ID: i16 = ServerPacketId::UserLocation as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(UserLocation {
            location: Point::read(r)?,
            direction: MirDirection::from_u8(r.read_u8()?),
        })
    }

    fn write(&self, w: &mut Writer) {
        self.location.write(w);
        w.write_u8(self.direction.to_u8());
    }
}

// ----------------------------- ID 24: ObjectPlayer（含 ID 25: ObjectHero） -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectPlayer {
    pub object_id: u32,
    pub name: String,
    pub guild_name: String,
    pub guild_rank_name: String,
    pub name_colour: Argb,
    pub class: MirClass,
    pub gender: MirGender,
    pub level: u16,
    pub location: Point,
    pub direction: MirDirection,
    pub hair: u8,
    pub light: u8,
    pub weapon: i16,
    pub weapon_effect: i16,
    pub armour: i16,
    pub poison: u16,
    pub dead: bool,
    pub hidden: bool,
    pub effect: u8,
    pub wing_effect: u8,
    pub extra: bool,
    pub mount_type: i16,
    pub riding_mount: bool,
    pub fishing: bool,
    pub transform_type: i16,
    pub element_orb_effect: u32,
    pub element_orb_lvl: u32,
    pub element_orb_max: u32,
    pub buffs: Vec<u8>,
    pub level_effects: LevelEffects,
}

impl ObjectPlayer {
    pub(crate) fn read_fields(r: &mut Reader) -> Result<Self> {
        let object_id = r.read_u32()?;
        let name = r.read_string()?;
        let guild_name = r.read_string()?;
        let guild_rank_name = r.read_string()?;
        let name_colour = Argb::from_i32(r.read_i32()?);
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
        let location = Point::read(r)?;
        let direction = MirDirection::from_u8(r.read_u8()?);
        let hair = r.read_u8()?;
        let light = r.read_u8()?;
        let weapon = r.read_i16()?;
        let weapon_effect = r.read_i16()?;
        let armour = r.read_i16()?;
        let poison = r.read_u16()?;
        let dead = r.read_bool()?;
        let hidden = r.read_bool()?;
        let effect = r.read_u8()?;
        let wing_effect = r.read_u8()?;
        let extra = r.read_bool()?;
        let mount_type = r.read_i16()?;
        let riding_mount = r.read_bool()?;
        let fishing = r.read_bool()?;
        let transform_type = r.read_i16()?;
        let element_orb_effect = r.read_u32()?;
        let element_orb_lvl = r.read_u32()?;
        let element_orb_max = r.read_u32()?;
        let mut buffs = Vec::new();
        let bcount = r.read_i32()?;
        for _ in 0..bcount.max(0) {
            buffs.push(r.read_u8()?);
        }
        let level_effects = LevelEffects(r.read_u16()?);
        Ok(ObjectPlayer {
            object_id,
            name,
            guild_name,
            guild_rank_name,
            name_colour,
            class,
            gender,
            level,
            location,
            direction,
            hair,
            light,
            weapon,
            weapon_effect,
            armour,
            poison,
            dead,
            hidden,
            effect,
            wing_effect,
            extra,
            mount_type,
            riding_mount,
            fishing,
            transform_type,
            element_orb_effect,
            element_orb_lvl,
            element_orb_max,
            buffs,
            level_effects,
        })
    }

    pub(crate) fn write_fields(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_string(&self.name);
        w.write_string(&self.guild_name);
        w.write_string(&self.guild_rank_name);
        w.write_i32(self.name_colour.to_i32());
        w.write_u8(self.class as u8);
        w.write_u8(self.gender as u8);
        w.write_u16(self.level);
        self.location.write(w);
        w.write_u8(self.direction.to_u8());
        w.write_u8(self.hair);
        w.write_u8(self.light);
        w.write_i16(self.weapon);
        w.write_i16(self.weapon_effect);
        w.write_i16(self.armour);
        w.write_u16(self.poison);
        w.write_bool(self.dead);
        w.write_bool(self.hidden);
        w.write_u8(self.effect);
        w.write_u8(self.wing_effect);
        w.write_bool(self.extra);
        w.write_i16(self.mount_type);
        w.write_bool(self.riding_mount);
        w.write_bool(self.fishing);
        w.write_i16(self.transform_type);
        w.write_u32(self.element_orb_effect);
        w.write_u32(self.element_orb_lvl);
        w.write_u32(self.element_orb_max);
        w.write_i32(self.buffs.len() as i32);
        for b in &self.buffs {
            w.write_u8(*b);
        }
        w.write_u16(self.level_effects.0);
    }
}

impl PacketCodec for ObjectPlayer {
    const ID: i16 = ServerPacketId::ObjectPlayer as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        ObjectPlayer::read_fields(r)
    }

    fn write(&self, w: &mut Writer) {
        self.write_fields(w);
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectHero {
    pub player: ObjectPlayer,
    pub owner_name: String,
}

impl PacketCodec for ObjectHero {
    const ID: i16 = ServerPacketId::ObjectHero as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        let player = ObjectPlayer::read_fields(r)?;
        let owner_name = r.read_string()?;
        Ok(ObjectHero { player, owner_name })
    }

    fn write(&self, w: &mut Writer) {
        self.player.write_fields(w);
        w.write_string(&self.owner_name);
    }
}

// ----------------------------- ID 26: ObjectRemove -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectRemove {
    pub object_id: u32,
}

impl PacketCodec for ObjectRemove {
    const ID: i16 = ServerPacketId::ObjectRemove as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectRemove {
            object_id: r.read_u32()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
    }
}

// ----------------------------- ID 27-29: ObjectTurn / ObjectWalk / ObjectRun -----------------------------

macro_rules! object_movement_packet {
    ($name:ident, $id:expr) => {
        #[derive(Debug, Clone, Default, PartialEq, Eq)]
        pub struct $name {
            pub object_id: u32,
            pub location: Point,
            pub direction: MirDirection,
        }

        impl PacketCodec for $name {
            const ID: i16 = $id;

            fn read(r: &mut Reader) -> Result<Self> {
                Ok($name {
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
    };
}

object_movement_packet!(ObjectTurn, ServerPacketId::ObjectTurn as i16);
object_movement_packet!(ObjectWalk, ServerPacketId::ObjectWalk as i16);
object_movement_packet!(ObjectRun, ServerPacketId::ObjectRun as i16);

// ----------------------------- ID 30: Chat -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Chat {
    pub message: String,
    pub r#type: u8,
}

impl PacketCodec for Chat {
    const ID: i16 = ServerPacketId::Chat as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Chat {
            message: r.read_string()?,
            r#type: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_string(&self.message);
        w.write_u8(self.r#type);
    }
}

// ----------------------------- ID 31: ObjectChat -----------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectChat {
    pub object_id: u32,
    pub text: String,
    pub r#type: u8,
}

impl PacketCodec for ObjectChat {
    const ID: i16 = ServerPacketId::ObjectChat as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(ObjectChat {
            object_id: r.read_u32()?,
            text: r.read_string()?,
            r#type: r.read_u8()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_u32(self.object_id);
        w.write_string(&self.text);
        w.write_u8(self.r#type);
    }
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

// ----------------------------- 分发枚举 -----------------------------

/// 已移植服务器包的分发枚举（随移植进度扩展）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerPacket {
    Connected(Connected),
    ClientVersion(ClientVersion),
    Disconnect(Disconnect),
    KeepAlive(KeepAlive),
    NewAccount(NewAccount),
    ChangePassword(ChangePassword),
    ChangePasswordBanned(ChangePasswordBanned),
    Login(Login),
    LoginBanned(LoginBanned),
    LoginSuccess(LoginSuccess),
    NewCharacter(NewCharacter),
    NewCharacterSuccess(NewCharacterSuccess),
    DeleteCharacter(DeleteCharacter),
    DeleteCharacterSuccess(DeleteCharacterSuccess),
    StartGame(StartGame),
    StartGameBanned(StartGameBanned),
    StartGameDelay(StartGameDelay),
    MapInformation(MapInformation),
    NewMapInfo(NewMapInfo),
    WorldMapSetupInfo(WorldMapSetupInfo),
    SearchMapResult(SearchMapResult),
    UserInformation(UserInformation),
    UserSlotsRefresh(UserSlotsRefresh),
    UserLocation(UserLocation),
    ObjectPlayer(ObjectPlayer),
    ObjectHero(ObjectHero),
    ObjectRemove(ObjectRemove),
    ObjectTurn(ObjectTurn),
    ObjectWalk(ObjectWalk),
    ObjectRun(ObjectRun),
    Chat(Chat),
    ObjectChat(ObjectChat),
    TimeOfDay(TimeOfDay),
}

impl ServerPacket {
    /// 按 ID 解码（未移植的 ID 返回 `InvalidPacketId`）
    pub fn decode(id: i16, payload: &[u8]) -> Result<Self> {
        use ServerPacketId::*;
        Ok(
            match ServerPacketId::from_i16(id).ok_or(crate::ProtocolError::InvalidPacketId(id))? {
                Connected => ServerPacket::Connected(crate::frame::decode_packet(id, payload)?),
                ClientVersion => {
                    ServerPacket::ClientVersion(crate::frame::decode_packet(id, payload)?)
                }
                Disconnect => ServerPacket::Disconnect(crate::frame::decode_packet(id, payload)?),
                KeepAlive => ServerPacket::KeepAlive(crate::frame::decode_packet(id, payload)?),
                NewAccount => ServerPacket::NewAccount(crate::frame::decode_packet(id, payload)?),
                ChangePassword => {
                    ServerPacket::ChangePassword(crate::frame::decode_packet(id, payload)?)
                }
                ChangePasswordBanned => {
                    ServerPacket::ChangePasswordBanned(crate::frame::decode_packet(id, payload)?)
                }
                Login => ServerPacket::Login(crate::frame::decode_packet(id, payload)?),
                LoginBanned => ServerPacket::LoginBanned(crate::frame::decode_packet(id, payload)?),
                LoginSuccess => {
                    ServerPacket::LoginSuccess(crate::frame::decode_packet(id, payload)?)
                }
                NewCharacter => {
                    ServerPacket::NewCharacter(crate::frame::decode_packet(id, payload)?)
                }
                NewCharacterSuccess => {
                    ServerPacket::NewCharacterSuccess(crate::frame::decode_packet(id, payload)?)
                }
                DeleteCharacter => {
                    ServerPacket::DeleteCharacter(crate::frame::decode_packet(id, payload)?)
                }
                DeleteCharacterSuccess => {
                    ServerPacket::DeleteCharacterSuccess(crate::frame::decode_packet(id, payload)?)
                }
                StartGame => ServerPacket::StartGame(crate::frame::decode_packet(id, payload)?),
                StartGameBanned => {
                    ServerPacket::StartGameBanned(crate::frame::decode_packet(id, payload)?)
                }
                StartGameDelay => {
                    ServerPacket::StartGameDelay(crate::frame::decode_packet(id, payload)?)
                }
                MapInformation => {
                    ServerPacket::MapInformation(crate::frame::decode_packet(id, payload)?)
                }
                NewMapInfo => ServerPacket::NewMapInfo(crate::frame::decode_packet(id, payload)?),
                WorldMapSetup => {
                    ServerPacket::WorldMapSetupInfo(crate::frame::decode_packet(id, payload)?)
                }
                SearchMapResult => {
                    ServerPacket::SearchMapResult(crate::frame::decode_packet(id, payload)?)
                }
                UserInformation => {
                    ServerPacket::UserInformation(crate::frame::decode_packet(id, payload)?)
                }
                UserSlotsRefresh => {
                    ServerPacket::UserSlotsRefresh(crate::frame::decode_packet(id, payload)?)
                }
                UserLocation => {
                    ServerPacket::UserLocation(crate::frame::decode_packet(id, payload)?)
                }
                ObjectPlayer => {
                    ServerPacket::ObjectPlayer(crate::frame::decode_packet(id, payload)?)
                }
                ObjectHero => ServerPacket::ObjectHero(crate::frame::decode_packet(id, payload)?),
                ObjectRemove => {
                    ServerPacket::ObjectRemove(crate::frame::decode_packet(id, payload)?)
                }
                ObjectTurn => ServerPacket::ObjectTurn(crate::frame::decode_packet(id, payload)?),
                ObjectWalk => ServerPacket::ObjectWalk(crate::frame::decode_packet(id, payload)?),
                ObjectRun => ServerPacket::ObjectRun(crate::frame::decode_packet(id, payload)?),
                Chat => ServerPacket::Chat(crate::frame::decode_packet(id, payload)?),
                ObjectChat => ServerPacket::ObjectChat(crate::frame::decode_packet(id, payload)?),
                TimeOfDay => ServerPacket::TimeOfDay(crate::frame::decode_packet(id, payload)?),
                _ => return Err(crate::ProtocolError::InvalidPacketId(id)),
            },
        )
    }
}

// 保留 MirGridType 引用，确保与 Items 系列包后续移植的依赖可见
#[allow(unused_imports)]
use MirGridType as _GridTypeReexport;
