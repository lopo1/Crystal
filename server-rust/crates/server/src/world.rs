//! 游戏世界内核（可玩闭环版）。
//!
//! 单地图、玩家 / 怪物 / 地面掉落物 / NPC 商人，以及战斗、掉落、拾取、
//! 经验升级、死亡复活等核心玩法。怪物刷新与 AI 由后台 tick 任务驱动。
//!
//! 对应原版 C# `MirEnvir` / `MapObject` / `MonsterObject` 的极简重建，
//! 用自建程序化资产替代缺失的原版二进制数据文件。
#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;

use crystal_protocol::binary::{Argb, Point};
use crystal_protocol::frame::encode_packet;
use crystal_protocol::server as s;
use crystal_protocol::types::{MirClass, MirDirection, MirGender, UserItem};
use tokio::sync::{broadcast, mpsc, Mutex};

/// 默认地图（Crystal 的 0 号图即新手村）
pub const MAP_WIDTH: i32 = 800;
pub const MAP_HEIGHT: i32 = 800;
pub const SPAWN: Point = Point { x: 400, y: 400 };

/// 传送门定义：踏上源坐标即传送到目标地图的坐标（目标会自动吸附可行走格）。
#[derive(Debug, Clone, Copy)]
pub struct PortalDef {
    pub src_map: u32,
    pub x: i32,
    pub y: i32,
    pub dest_map: u32,
    pub dest_x: i32,
    pub dest_y: i32,
}

/// 传送门配置（垂直切片）：新手村(地图0) <-> 0100(地图100)。
/// 源坐标必须为可通行的格，玩家走上去即触发。
pub fn portals() -> Vec<PortalDef> {
    vec![
        // 新手村南下到 0100 洞穴
        PortalDef { src_map: 0, x: 404, y: 412, dest_map: 100, dest_x: 8, dest_y: 6 },
        // 回新手村
        PortalDef { src_map: 100, x: 4, y: 4, dest_map: 0, dest_x: 404, dest_y: 404 },
    ]
}

// ---------------------------------------------------------------------------
// 实体
// ---------------------------------------------------------------------------

/// 在线玩家
#[derive(Debug, Clone)]
pub struct Player {
    pub object_id: u32,
    pub account_id: String,
    pub name: String,
    pub class: MirClass,
    pub gender: MirGender,
    pub level: u16,
    pub location: Point,
    pub direction: MirDirection,
    pub max_hp: i32,
    pub hp: i32,
    pub max_mp: i32,
    pub mp: i32,
    pub attack: i32,
    pub defence: i32,
    pub experience: u32,
    pub gold: u32,
    pub weapon: i16,
    pub armour: i16,
    pub character_index: i32,
    pub sender: mpsc::Sender<Vec<u8>>,
    pub hp_changed: bool,
    /// 已穿戴装备：EquipmentSlot -> UserItem（用于生成 UserSlotsRefresh）
    pub equipment: std::collections::BTreeMap<i32, UserItem>,
    /// 当前所在地图编号（默认 0）
    pub map_index: u32,
    /// 自动喝药：要自动使用的消耗品 item_index（0=关闭）
    pub auto_pot_item: i32,
    /// 自动喝药：HP 低于该阈值时触发
    pub auto_pot_hp: u32,
    /// 最近出售记录：(unique_id, item_index, count, 售出价)，供 NPC 回购
    pub recently_sold: Vec<(u64, i32, u16, u32)>,
    /// 钓鱼：是否正抛竿
    pub fishing: bool,
    /// 钓鱼进度 0..100
    pub fishing_progress: u16,
    /// 自动抛竿：钓到后是否继续
    pub auto_fish: bool,
    /// 本会话是否已解锁仓库密码
    pub storage_unlocked: bool,
    /// 待确认的 mentor 邀请（对方角色名）
    pub pending_mentor: Option<String>,
    /// 是否允许他人邀请我收徒
    pub can_be_mentor: bool,
    /// 待确认的结婚邀请（对方角色名）
    pub pending_marriage: Option<String>,
    /// 是否处于死亡状态（死亡保留：直到 TownRevive 才复活）
    pub dead: bool,
    /// 攻击模式（AttackMode：0=和平 1=编组 2=行会 3=敌对行会 4=红名 5=全体）
    pub a_mode: u8,
    /// 是否允许组队邀请（C# AllowGroup，默认允许）
    pub allow_group: bool,
    /// 和平模式（PeaceMode：PK 保护开关状态值，同 C# PMode）
    pub p_mode: u8,
    /// 法术快捷键绑定：spell -> key（MagicKey 包，会话内有效）
    pub magic_keys: std::collections::HashMap<u8, u8>,
}

/// 地面掉落物
#[derive(Debug, Clone)]
pub struct GroundItem {
    pub object_id: u32,
    pub item_index: i32,
    pub count: u16,
    pub location: Point,
    pub unique_id: u64,
    pub map_index: u32,
    /// 金币堆：>0 表示这是一个金币掉落，而不是物品
    pub gold: u32,
}

/// 怪物
#[derive(Debug, Clone)]
pub struct Monster {
    pub object_id: u32,
    pub name: String,
    pub image: u16,
    pub location: Point,
    pub direction: MirDirection,
    pub level: u16,
    pub max_hp: i32,
    pub hp: i32,
    pub attack: i32,
    pub defence: i32,
    pub exp_reward: u32,
    pub gold_reward: u32,
    pub drops: Vec<i32>,
    pub dead: bool,
    /// 死亡后的 tick 计数（到阈值则复活）
    pub dead_ticks: u32,
    /// 尸体是否已被采集（Harvest 割肉，同 C# Harvested）
    pub harvested: bool,
    /// 采集所得物品 item_index（0=不可采集）
    pub harvest_item: i32,
    /// 攻击目标玩家 / 攻击冷却
    pub target: Option<u32>,
    pub cooldown: u32,
    /// 怪物所在地图
    pub map_index: u32,
    /// 攻击方式：false 近战（相邻格），true 远程（射程内发射弹体）
    pub ranged: bool,
}

/// 静态 NPC（用 ObjectPlayer 显示）
#[derive(Debug, Clone)]
pub struct Npc {
    pub object_id: u32,
    pub name: String,
    pub image: u16,
    pub location: Point,
    pub shop_items: Vec<i32>,
    pub map_index: u32,
}

// ---------------------------------------------------------------------------
// 世界
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct World {
    pub players: Arc<Mutex<HashMap<u32, Player>>>,
    pub monsters: Arc<Mutex<HashMap<u32, Monster>>>,
    pub items: Arc<Mutex<HashMap<u32, GroundItem>>>,
    npcs: Arc<Mutex<Vec<Npc>>>,
    broadcast_tx: broadcast::Sender<Vec<u8>>,
    next_object_id: Arc<std::sync::atomic::AtomicU32>,
    next_item_unique: Arc<std::sync::atomic::AtomicU64>,
    /// 当前地图（含碰撞网格，默认主地图 0）
    pub map: Arc<crate::maps::GameMap>,
    /// 地图注册表：map_index -> GameMap（支持多地图传送）
    pub maps: Arc<std::sync::RwLock<std::collections::HashMap<u32, Arc<crate::maps::GameMap>>>>,
    /// 组队管理器（跨连接共享）
    pub group: Arc<Mutex<crate::group::GroupManager>>,
    pub trade: Arc<Mutex<crate::trade::TradeManager>>,
    pub guild: Arc<Mutex<crate::guild::GuildManager>>,
    pub market: Arc<Mutex<crate::market::MarketManager>>,
    /// 任务管理器（记录触碰的 NPC，供对话接任务）
    pub quest: Arc<Mutex<crate::quest::QuestManager>>,
}

/// 无真实地图时的程序化空地图（保持原有 800x800 全通行为，供无头/缺图运行）
pub fn default_map() -> crate::maps::GameMap {
    crate::maps::load_map_bytes(
        0,
        &{
            // 构造 800x800 全通 V100 字节
            let w = 800u16;
            let h = 800u16;
            let mut b = vec![0u8; 8 + (w as usize) * (h as usize) * 26];
            b[0] = 1;
            b[2] = 0x43;
            b[3] = 0x23;
            b[4..6].copy_from_slice(&w.to_le_bytes());
            b[6..8].copy_from_slice(&h.to_le_bytes());
            b
        },
    )
    .expect("程序化空地图构造失败")
}

impl World {
    pub fn new() -> Self {
        Self::with_map(default_map())
    }

    pub fn with_map(map: crate::maps::GameMap) -> Self {
        let (broadcast_tx, _) = broadcast::channel(512);
        let mut maps = std::collections::HashMap::new();
        maps.insert(map.index, Arc::new(map.clone()));
        World {
            players: Arc::new(Mutex::new(HashMap::new())),
            monsters: Arc::new(Mutex::new(HashMap::new())),
            items: Arc::new(Mutex::new(HashMap::new())),
            npcs: Arc::new(Mutex::new(Vec::new())),
            broadcast_tx,
            next_object_id: Arc::new(std::sync::atomic::AtomicU32::new(1)),
            next_item_unique: Arc::new(std::sync::atomic::AtomicU64::new(1)),
            map: Arc::new(map),
            maps: Arc::new(std::sync::RwLock::new(maps)),
            group: Arc::new(Mutex::new(crate::group::GroupManager::new())),
            trade: Arc::new(Mutex::new(crate::trade::TradeManager::new())),
            guild: Arc::new(Mutex::new(crate::guild::GuildManager::new())),
            market: Arc::new(Mutex::new(crate::market::MarketManager::new())),
            quest: Arc::new(Mutex::new(crate::quest::QuestManager::new())),
        }
    }

    /// 注册一张地图到注册表（多地图支持）
    pub fn register_map(&self, map: crate::maps::GameMap) {
        self.maps.write().unwrap().insert(map.index, Arc::new(map));
    }

    /// 取指定地图句柄；不存在回退主地图 map
    pub fn get_map(&self, map_index: u32) -> Arc<crate::maps::GameMap> {
        self.maps
            .read()
            .unwrap()
            .get(&map_index)
            .cloned()
            .unwrap_or_else(|| self.map.clone())
    }

    /// 该坐标是否可通行（地图内且非墙）
    pub fn is_walkable(&self, x: i32, y: i32) -> bool {
        self.map.is_walkable(x, y)
    }

    /// 找一个可通行的坐标（供出生/刷新用；从给定点向外搜索）
    pub fn nearest_walkable(&self, x: i32, y: i32) -> (i32, i32) {
        self.nearest_walkable_on(self.map.index, x, y)
    }

    /// 在指定地图上查找可通行坐标
    pub fn nearest_walkable_on(&self, map_index: u32, x: i32, y: i32) -> (i32, i32) {
        let map = self.get_map(map_index);
        for r in 0i32..40 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs().max(dy.abs()) != r {
                        continue;
                    }
                    if map.is_walkable(x + dx, y + dy) {
                        return (x + dx, y + dy);
                    }
                }
            }
        }
        // 兜底：地图中心
        (map.width as i32 / 2, map.height as i32 / 2)
    }

    /// 在指定地图上判断某个坐标是否可通行
    pub fn is_walkable_on(&self, map_index: u32, x: i32, y: i32) -> bool {
        self.get_map(map_index).is_walkable(x, y)
    }

    /// 在指定地图作一步移动（碰撞校验）
    pub fn try_move_on(&self, map_index: u32, location: Point, dir: MirDirection, steps: i32) -> Option<Point> {
        let map = self.get_map(map_index);
        let (dx, dy) = direction_offset(dir, steps);
        let nx = location.x + dx;
        let ny = location.y + dy;
        if !map.is_walkable(nx, ny) {
            return None;
        }
        Some(Point::new(nx, ny))
    }

    /// 传送玩家到指定地图的 (x,y)（自动吸附可行走格），并下发地图信息给客户端。
    pub async fn teleport_player(&self, player_id: u32, map_index: u32, x: i32, y: i32) -> bool {
        let (wx, wy) = self.nearest_walkable_on(map_index, x, y);
        {
            let mut players = self.players.lock().await;
            if let Some(p) = players.get_mut(&player_id) {
                p.map_index = map_index;
                p.location = Point::new(wx, wy);
            } else {
                return false;
            }
        }
        let map = self.get_map(map_index);
        self.send_to(player_id, encode_packet(&s::MapInformation {
            map_index: map_index as i32,
            file_name: map_index.to_string(),
            title: format!("地图 {map_index}"),
            mini_map: 0,
            big_map: 0,
            lights: 0,
            lightning: false,
            fire: false,
            map_dark_light: 0,
            music: 0,
            weather_particles: 0,
        })).await;
        self.send_to(player_id, encode_packet(&s::NewMapInfo {
            map_index: map_index as i32,
            info: crystal_protocol::types::ClientMapInfo {
                title: format!("地图 {map_index}"),
                width: map.width as i32,
                height: map.height as i32,
                big_map: 0,
                movements: vec![],
                npcs: vec![],
            },
        })).await;
        self.send_to(player_id, encode_packet(&s::UserLocation {
            location: Point::new(wx, wy),
            direction: MirDirection::Up,
        })).await;
        true
    }

    /// 查找某地图坐标是否有传送门；命中返回 (目标地图, 目标x, 目标y)。
    pub fn portal_at(&self, map_index: u32, x: i32, y: i32) -> Option<(u32, i32, i32)> {
        portals()
            .into_iter()
            .find(|p| p.src_map == map_index && p.x == x && p.y == y)
            .map(|p| (p.dest_map, p.dest_x, p.dest_y))
    }

    pub fn next_object_id(&self) -> u32 {
        self.next_object_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }
    pub fn next_item_unique(&self) -> u64 {
        self.next_item_unique
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    pub fn broadcast(&self, frame: Vec<u8>) {
        let _ = self.broadcast_tx.send(frame);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Vec<u8>> {
        self.broadcast_tx.subscribe()
    }

    pub async fn send_to(&self, object_id: u32, frame: Vec<u8>) {
        if let Some(p) = self.players.lock().await.get(&object_id) {
            let _ = p.sender.try_send(frame);
        }
    }

    /// 对除 `except` 外所有玩家广播
    pub async fn broadcast_except(&self, frame: Vec<u8>, except: u32) {
        let players = self.players.lock().await;
        for p in players.values() {
            if p.object_id != except {
                let _ = p.sender.try_send(frame.clone());
            }
        }
    }

    /// 仅对所在地图为 `map_index` 的玩家广播
    pub async fn broadcast_on(&self, map_index: u32, frame: Vec<u8>) {
        let players = self.players.lock().await;
        for p in players.values() {
            if p.map_index == map_index {
                let _ = p.sender.try_send(frame.clone());
            }
        }
    }

    /// 仅对所在地图为 `map_index` 且非 `except` 的玩家广播
    pub async fn broadcast_on_except(&self, map_index: u32, frame: Vec<u8>, except: u32) {
        let players = self.players.lock().await;
        for p in players.values() {
            if p.map_index == map_index && p.object_id != except {
                let _ = p.sender.try_send(frame.clone());
            }
        }
    }

    pub async fn add_player(&self, player: Player) {
        let p = player.clone();
        self.broadcast_except(encode_packet(&s::ObjectPlayer {
            object_id: p.object_id,
            name: p.name.clone(),
            guild_name: String::new(),
            guild_rank_name: String::new(),
            name_colour: Argb(0),
            class: p.class,
            gender: p.gender,
            level: p.level,
            location: p.location,
            direction: p.direction,
            hair: 0,
            light: 0,
            weapon: p.weapon,
            weapon_effect: 0,
            armour: p.armour,
            poison: 0,
            dead: p.dead,
            hidden: false,
            effect: 0,
            wing_effect: 0,
            extra: false,
            mount_type: 0,
            riding_mount: false,
            fishing: false,
            transform_type: 0,
            element_orb_effect: 0,
            element_orb_lvl: 0,
            element_orb_max: 0,
            buffs: vec![],
            level_effects: crystal_protocol::types::LevelEffects(0),
        }), p.object_id)
        .await;
        self.players.lock().await.insert(player.object_id, player);
        // 把场上怪物/掉落/NPC 同步给新玩家
        self.send_world_state_to(p.object_id).await;
    }

    async fn send_world_state_to(&self, oid: u32) {
        let player_map = self.players.lock().await.get(&oid).map(|p| p.map_index).unwrap_or(0);
        // 怪物（仅本图）
        let monsters = self.monsters.lock().await.clone();
        for m in monsters.values().filter(|m| m.map_index == player_map) {
            self.send_to(oid, encode_packet(&s::ObjectMonster {
                object_id: m.object_id,
                name: m.name.clone(),
                name_colour: Argb(0xFFFF2222),
                location: m.location,
                image: m.image,
                direction: m.direction,
                effect: 0,
                ai: 0,
                light: 0,
                dead: m.dead,
                skeleton: false,
                poison: 0,
                hidden: false,
                shock_time: 0,
                binding_shot_center: false,
                extra: false,
                extra_byte: 0,
                master_object_id: 0,
                rarity: 0,
                buffs: vec![],
            }))
            .await;
        }
        // 掉落物（仅本图）
        let items = self.items.lock().await.clone();
        for gi in items.values().filter(|gi| gi.map_index == player_map) {
            self.send_to(oid, encode_packet(&s::ObjectItem {
                object_id: gi.object_id,
                name: format!("#{}", gi.item_index),
                name_colour: Argb(0),
                location: gi.location,
                image: gi.item_index as u16,
                grade: 0,
            }))
            .await;
        }
        // NPC（仅本图）
        let npcs = self.npcs.lock().await.clone();
        for n in npcs.iter().filter(|n| n.map_index == player_map) {
            self.send_to(oid, encode_packet(&s::ObjectPlayer {
                object_id: n.object_id,
                name: n.name.clone(),
                guild_name: String::new(),
                guild_rank_name: String::new(),
                name_colour: Argb(0xFFFFAA00),
                class: MirClass::Warrior,
                gender: MirGender::Male,
                level: 1,
                location: n.location,
                direction: MirDirection::Down,
                hair: 0,
                light: 0,
                weapon: 0,
                weapon_effect: 0,
                armour: 0,
                poison: 0,
                dead: false,
                hidden: false,
                effect: 0,
                wing_effect: 0,
                extra: false,
                mount_type: 0,
                riding_mount: false,
                fishing: false,
                transform_type: 0,
                element_orb_effect: 0,
                element_orb_lvl: 0,
                element_orb_max: 0,
                buffs: vec![],
                level_effects: crystal_protocol::types::LevelEffects(0),
            }))
            .await;
        }
    }

    pub async fn remove_player(&self, object_id: u32) {
        self.players.lock().await.remove(&object_id);
        self.broadcast(encode_packet(&s::ObjectRemove { object_id }));
    }

    pub async fn get_player(&self, object_id: u32) -> Option<Player> {
        self.players.lock().await.get(&object_id).cloned()
    }

    pub async fn add_monster(&self, m: Monster) {
        let mi = m.map_index;
        self.broadcast_on(mi, encode_packet(&s::ObjectMonster {
            object_id: m.object_id,
            name: m.name.clone(),
            name_colour: Argb(0xFFFF2222),
            location: m.location,
            image: m.image,
            direction: m.direction,
            effect: 0,
            ai: 0,
            light: 0,
            dead: m.dead,
            skeleton: false,
            poison: 0,
            hidden: false,
            shock_time: 0,
            binding_shot_center: false,
            extra: false,
            extra_byte: 0,
            master_object_id: 0,
            rarity: 0,
            buffs: vec![],
        })).await;
        self.monsters.lock().await.insert(m.object_id, m);
    }

    pub async fn remove_monster(&self, object_id: u32) {
        let mi = self
            .monsters
            .lock()
            .await
            .get(&object_id)
            .map(|m| m.map_index)
            .unwrap_or(0);
        self.monsters.lock().await.remove(&object_id);
        self.broadcast_on(mi, encode_packet(&s::ObjectRemove { object_id })).await;
    }

    pub async fn add_ground_item(&self, gi: GroundItem) {
        let _ = gi.unique_id;
        let mi = gi.map_index;
        let (name, image) = if gi.gold > 0 {
            ("金币".to_string(), 0u16)
        } else {
            (format!("#{}", gi.item_index), gi.item_index as u16)
        };
        self.broadcast_on(mi, encode_packet(&s::ObjectItem {
            object_id: gi.object_id,
            name,
            name_colour: Argb(0),
            location: gi.location,
            image,
            grade: 0,
        })).await;
        self.items.lock().await.insert(gi.object_id, gi);
    }

    pub async fn remove_ground_item(&self, object_id: u32) {
        let mi = self.items.lock().await.get(&object_id).map(|i| i.map_index).unwrap_or(0);
        self.items.lock().await.remove(&object_id);
        self.broadcast_on(mi, encode_packet(&s::ObjectRemove { object_id })).await;
    }

    pub async fn npcs(&self) -> Vec<Npc> {
        self.npcs.lock().await.clone()
    }
}

// ---------------------------------------------------------------------------
// 后台世界 tick
// ---------------------------------------------------------------------------

pub fn spawn_world_tick(world: Arc<World>, db: Arc<crate::db::Database>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        seed_world(&world).await;
        let mut tick: u32 = 0;
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            tick = tick.wrapping_add(1);
            world_tick(&world, &db, tick).await;
        }
    })
}

/// 初始化新手村：若干怪物 + 一个商人 NPC（程序化资产）
async fn seed_world(world: &World) {
    if !world.monsters.lock().await.is_empty() {
        return;
    }
    // 刷怪点数据化：怪物模板 + 每图刷怪点来自 spawn_config（唯一数据源）
    let defs: Vec<(u16, String, u16, i32, i32, i32, u32, u32, bool, i32)> =
        crate::spawn_config::MONSTER_TEMPLATES
            .iter()
            .map(|t| {
                (
                    t.image,
                    t.name.to_string(),
                    t.level,
                    t.hp,
                    t.attack,
                    t.defence,
                    t.exp,
                    t.gold,
                    t.ranged,
                    t.harvest_item,
                )
            })
            .collect();
    let mut oid = world.next_object_id();
    // 遍历所有已注册地图的配置刷怪点
    let map_indexes: Vec<u32> = world.maps.read().unwrap().keys().copied().collect();
    for mi in map_indexes {
        let map = world.get_map(mi);
        for &(cx, cy) in crate::spawn_config::spawn_points_for(mi) {
            let t = defs[(oid as usize) % defs.len()].clone();
            let (wx, wy) = if map.is_walkable(cx, cy) {
                (cx, cy)
            } else {
                world.nearest_walkable_on(mi, cx, cy)
            };
            world
                .add_monster(Monster {
                    object_id: oid,
                    name: t.1.clone(),
                    image: t.0,
                    location: Point::new(wx, wy),
                    direction: MirDirection::Up,
                    level: t.2,
                    max_hp: t.3,
                    hp: t.3,
                    attack: t.4,
                    defence: t.5,
                    exp_reward: t.6,
                    gold_reward: t.7,
                    drops: vec![1, 2, 3, 4, 5],
                    dead: false,
                    dead_ticks: 0,
                    harvested: false,
                    harvest_item: t.9,
                    target: None,
                    cooldown: 0,
                    map_index: mi,
                    ranged: t.8,
                })
                .await;
            oid += 1;
        }
    }
    // 商人 NPC：卖 木剑(1)/布衣(2)/金创药(3)/回城卷(4)/铜钱袋(5)
    if world.npcs.lock().await.is_empty() {
        let (wx, wy) = world.nearest_walkable(404, 400);
        let npc = Npc {
            object_id: world.next_object_id(),
            name: "铁匠铺".to_string(),
            image: 0,
            location: Point::new(wx, wy),
            shop_items: vec![1, 2, 3, 4, 5],
            map_index: 0,
        };
        world
            .broadcast_on(0, encode_packet(&s::ObjectPlayer {
                object_id: npc.object_id,
                name: npc.name.clone(),
                guild_name: String::new(),
                guild_rank_name: String::new(),
                name_colour: Argb(0xFFFFAA00),
                class: MirClass::Warrior,
                gender: MirGender::Male,
                level: 1,
                location: npc.location,
                direction: MirDirection::Down,
                hair: 0,
                light: 0,
                weapon: 0,
                weapon_effect: 0,
                armour: 0,
                poison: 0,
                dead: false,
                hidden: false,
                effect: 0,
                wing_effect: 0,
                extra: false,
                mount_type: 0,
                riding_mount: false,
                fishing: false,
                transform_type: 0,
                element_orb_effect: 0,
                element_orb_lvl: 0,
                element_orb_max: 0,
                buffs: vec![],
                level_effects: crystal_protocol::types::LevelEffects(0),
            }))
            .await;
        world.npcs.lock().await.push(npc);

        // 任务管理员 NPC：触碰它(CallNPC)可对话接任务（猎杀骷髅/除蜘蛛）
        let (qx, qy) = world.nearest_walkable(402, 400);
        let quest_npc = Npc {
            object_id: world.next_object_id(),
            name: "任务管理员".to_string(),
            image: 0,
            location: Point::new(qx, qy),
            shop_items: vec![],
            map_index: 0,
        };
        world
            .broadcast_on(0, encode_packet(&s::ObjectPlayer {
                object_id: quest_npc.object_id,
                name: quest_npc.name.clone(),
                guild_name: String::new(),
                guild_rank_name: String::new(),
                name_colour: Argb(0xFF55FF55),
                class: MirClass::Warrior,
                gender: MirGender::Male,
                level: 1,
                location: quest_npc.location,
                direction: MirDirection::Down,
                hair: 0,
                light: 0,
                weapon: 0,
                weapon_effect: 0,
                armour: 0,
                poison: 0,
                dead: false,
                hidden: false,
                effect: 0,
                wing_effect: 0,
                extra: false,
                mount_type: 0,
                riding_mount: false,
                fishing: false,
                transform_type: 0,
                element_orb_effect: 0,
                element_orb_lvl: 0,
                element_orb_max: 0,
                buffs: vec![],
                level_effects: crystal_protocol::types::LevelEffects(0),
            }))
            .await;
        world.npcs.lock().await.push(quest_npc);

        // 传送门标记 NPC（在新手村，踏上其所在格即传送到 0100）
        let (pw, ph) = world.nearest_walkable(404, 412);
        let portal_npc = Npc {
            object_id: world.next_object_id(),
            name: "传送门(0100)".to_string(),
            image: 0,
            location: Point::new(pw, ph),
            shop_items: vec![],
            map_index: 0,
        };
        world.npcs.lock().await.push(portal_npc);
        // 广而告之（此传送门为视觉标记，真正的触发是玩家走上去的坐标）
        let _ = portal_npc;
    }
}

async fn world_tick(world: &World, db: &crate::db::Database, tick: u32) {
    respawn_monsters(world).await;
    monster_ai(world, db).await;
    regen_players(world, db).await;
    fishing_tick(world, db).await;
    // 每 25 tick（约 10 秒）周期性持久化玩家状态，降低掉线/宕机丢失进度风险
    if tick % 25 == 0 {
        persist_all_players(world, db).await;
    }
}

/// 钓鱼 tick：每 tick 给正在抛竿的玩家加进度；进度到 100 即“上钩”，随机钓起物品入库。
async fn fishing_tick(world: &World, db: &crate::db::Database) {
    // 先收集上钩事件，避免在锁内调用 IO
    let mut catches: Vec<(u32, i32, u16)> = Vec::new(); // (oid, item_index, count)
    let mut stop_fishing: Vec<u32> = Vec::new();
    {
        let mut players = world.players.lock().await;
        for p in players.values_mut() {
            if !p.fishing {
                continue;
            }
            p.fishing_progress = p.fishing_progress.saturating_add(8);
            if p.fishing_progress >= 100 {
                // 上钩：从演示掉落 [3 金创药, 5 铜币袋] 随机一个
                use rand::seq::IteratorRandom;
                let idx = {
                    let mut rng = rand::thread_rng();
                    *[3i32, 5].iter().choose(&mut rng).unwrap()
                };
                catches.push((p.object_id, idx, 1));
                p.fishing_progress = 0;
                if !p.auto_fish {
                    p.fishing = false;
                    stop_fishing.push(p.object_id);
                }
            }
        }
    }
    for (oid, item_index, count) in catches {
        let Some(player) = world.get_player(oid).await else { continue };
        let ok = db.add_item_to_inventory(player.character_index, item_index, count).unwrap_or(false);
        if ok {
            let item = UserItem {
                unique_id: 0,
                item_index,
                count,
                current_dura: 0,
                max_dura: 0,
                ..Default::default()
            };
            world.send_to(oid, encode_packet(&s::GainedItem { item })).await;
        } else {
            world.send_to(oid, encode_packet(&s::ObjectChat {
                object_id: oid,
                text: "背包已满，未能拾取钓到的物品".to_string(),
                r#type: 1,
            })).await;
        }
    }
    for oid in stop_fishing {
        world.send_to(oid, encode_packet(&s::FishingUpdate {
            object_id: oid,
            fishing: false,
            progress_percent: 0,
            chance_percent: 0,
            fishing_point: world.get_player(oid).await.map(|p| p.location).unwrap_or(SPAWN),
            found_fish: false,
        })).await;
    }
}

/// 死亡怪物计数复活（约 30 tick ≈ 12 秒）
async fn respawn_monsters(world: &World) {
    let mut respawns = Vec::new();
    {
        let mut mons = world.monsters.lock().await;
        for m in mons.values_mut() {
            if m.dead {
                m.dead_ticks += 1;
                if m.dead_ticks >= 30 {
                    m.dead = false;
                    m.dead_ticks = 0;
                    m.harvested = false; // 复活后尸体可重新采集
                    m.hp = m.max_hp;
                    m.target = None;
                    m.cooldown = 0;
                    respawns.push(m.clone());
                }
            }
        }
    }
    for m in respawns {
        world.add_monster(m).await;
    }
}

/// 怪物 AI：索敌（感知范围）→ 追击（贪心靠近）→ 攻击分支；目标消失则放弃仇恨。
///
/// - 感知半径 `MONSTER_PERCEPTION`：进入的玩家会被设为仇恨目标（主动索敌）。
/// - `MONSTER_LEASH`：追击超过此距离则脱战，清空仇恨。
/// - 攻击分支：近战怪相邻格平A（`ObjectAttack`）；远程怪射程内发射弹体
///   （`ObjectRangeAttack`，同 C# 远程怪物覆写 Attack 的表现）。
async fn monster_ai(world: &World, db: &crate::db::Database) {
    const PERCEPTION: i32 = 5;
    const LEASH: i32 = 12;
    /// 远程怪攻击射程
    const RANGE: i32 = 6;

    // (怪物id, 玩家id, 伤害, 是否远程)
    let mut attacks: Vec<(u32, u32, i32, bool)> = Vec::new();
    let mut moves: Vec<(u32, Point, MirDirection)> = Vec::new();
    {
        let mut mons = world.monsters.lock().await;
        let players = world.players.lock().await;
        for m in mons.values_mut() {
            // 冷却递减（攻击/移动的节奏控制）
            if m.cooldown > 0 {
                m.cooldown -= 1;
                continue;
            }
            if m.dead {
                continue;
            }
            // 主动索敌：无目标时，感知范围内的最近存活玩家成为目标
            let mut target_oid = m.target;
            let mut target_loc: Option<Point> = None;
            if target_oid.is_some() {
                let tid = target_oid.unwrap();
                let alive = players
                    .get(&tid)
                    .map(|p| p.map_index == m.map_index && !p.dead)
                    .unwrap_or(false);
                if alive {
                    target_loc = players.get(&tid).map(|p| p.location);
                } else {
                    target_oid = None; // 目标消失/跨图/死亡，脱战
                }
            }
            if target_oid.is_none() {
                let mut nearest: Option<(u32, i32)> = None;
                for p in players
                    .values()
                    .filter(|p| p.map_index == m.map_index && !p.dead)
                {
                    let d = manhattan(m.location, p.location);
                    if d <= PERCEPTION && nearest.map(|(_, nd)| d < nd).unwrap_or(true) {
                        nearest = Some((p.object_id, d));
                    }
                }
                if let Some((oid, _)) = nearest {
                    target_oid = Some(oid);
                }
            }
            m.target = target_oid; // 回写仇恨目标（含脱战清空）

            let Some(tid) = target_oid else { continue };
            let Some(tloc) = target_loc.or_else(|| players.get(&tid).map(|p| p.location)) else {
                continue;
            };

            // 超过脱战距离则放弃
            if manhattan(m.location, tloc) > LEASH {
                continue;
            }

            let dist = manhattan(m.location, tloc);
            if dist <= 1 {
                // 相邻 -> 近战分支
                let dmg = (m.attack.max(1) - players.get(&tid).map(|p| p.defence / 2).unwrap_or(0))
                    .max(1);
                attacks.push((m.object_id, tid, dmg, false));
            } else if m.ranged && dist <= RANGE {
                // 射程内 -> 远程分支（不移动，原地射击）
                let dmg = (m.attack.max(1) - players.get(&tid).map(|p| p.defence / 4).unwrap_or(0))
                    .max(1);
                attacks.push((m.object_id, tid, dmg, true));
            } else {
                // 不相邻 -> 选择最佳可行走邻格逼近目标（可绕开简单障碍）
                if let Some((new_loc, dir)) = world.chase_step(m.location, tloc) {
                    moves.push((m.object_id, new_loc, dir));
                }
            }
        }
    }
    // 应用移动（更新位置 + 广播）
    for (mid, new_loc, dir) in moves {
        let mi = {
            let mut mons = world.monsters.lock().await;
            let Some(m) = mons.get_mut(&mid) else { continue };
            m.location = new_loc;
            m.direction = dir;
            m.cooldown = 1; // 限制移动频率
            m.map_index
        };
        world
            .broadcast_on(mi, encode_packet(&s::ObjectWalk {
                object_id: mid,
                location: new_loc,
                direction: dir,
            }))
            .await;
    }
    // 应用攻击：先广播攻击动画（近战 ObjectAttack / 远程 ObjectRangeAttack），再结算伤害
    for (mid, pid, dmg, ranged) in attacks {
        let (mloc, mi) = {
            let mons = world.monsters.lock().await;
            match mons.get(&mid) {
                Some(m) => (m.location, m.map_index),
                None => continue,
            }
        };
        let Some(p) = world.get_player(pid).await else { continue };
        let dir = dir_from_delta(
            (p.location.x - mloc.x).signum(),
            (p.location.y - mloc.y).signum(),
        );
        if ranged {
            world
                .broadcast_on(mi, encode_packet(&s::ObjectRangeAttack {
                    object_id: mid,
                    location: mloc,
                    direction: dir,
                    target_id: pid,
                    target: p.location,
                    r#type: 0,
                    spell: 0,
                    level: 0,
                }))
                .await;
        } else {
            world
                .broadcast_on(mi, encode_packet(&s::ObjectAttack {
                    object_id: mid,
                    location: mloc,
                    direction: dir,
                    spell: 0,
                    level: 0,
                    r#type: 0,
                }))
                .await;
        }
        monster_hit_player(world, db, mid, pid, dmg).await;
    }
}

/// 计算朝目标走一步的 (dx, dy)，优先沿距离更大的轴逼近。
fn chase_step(from: Point, to: Point) -> (i32, i32) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let (ax, ay) = (dx.abs(), dy.abs());
    if ax >= ay && dx != 0 {
        (dx.signum(), 0)
    } else if dy != 0 {
        (0, dy.signum())
    } else {
        (dx.signum(), 0)
    }
}

/// dx/dy -> MirDirection
fn dir_from_delta(dx: i32, dy: i32) -> MirDirection {
    match (dx, dy) {
        (0, -1) => MirDirection::Up,
        (1, -1) => MirDirection::UpRight,
        (1, 0) => MirDirection::Right,
        (1, 1) => MirDirection::DownRight,
        (0, 1) => MirDirection::Down,
        (-1, 1) => MirDirection::DownLeft,
        (-1, 0) => MirDirection::Left,
        (-1, -1) => MirDirection::UpLeft,
        _ => MirDirection::Up,
    }
}

/// 玩家生命回复
async fn regen_players(world: &World, db: &crate::db::Database) {
    let mut to_heal: Vec<(u32, i32, i32)> = Vec::new();
    let mut to_autopot: Vec<(u32, i32)> = Vec::new();
    {
        let players = world.players.lock().await;
        for p in players.values() {
            // 死亡保留：尸体不回血、不触发自动喝药
            if p.dead {
                continue;
            }
            if p.hp < p.max_hp {
                let nhp = (p.hp + 4).min(p.max_hp);
                let nmp = (p.mp + 2).min(p.max_mp);
                to_heal.push((p.object_id, nhp, nmp));
            }
            // 自动喝药：血量低于阈值且有配置的消耗品
            if p.auto_pot_item > 0 && p.auto_pot_hp > 0 && (p.hp as u32) <= p.auto_pot_hp {
                to_autopot.push((p.object_id, p.auto_pot_item));
            }
        }
    }
    for (oid, hp, mp) in to_heal {
        let mut players = world.players.lock().await;
        if let Some(p) = players.get_mut(&oid) {
            p.hp = hp;
            p.mp = mp;
        }
        drop(players);
        world.send_to(oid, encode_packet(&s::HealthChanged { hp, mp })).await;
    }
    for (oid, item_index) in to_autopot {
        auto_pot(world, db, oid, item_index).await;
    }
}

/// 自动喝药：从背包中找到指定消耗品并使用一个（回复 HP 并扣除一格）。
async fn auto_pot(world: &World, db: &crate::db::Database, player_id: u32, item_index: i32) {
    let Some(player) = world.get_player(player_id).await else { return };
    let char_index = player.character_index;
    // 在背包中找该 item_index 的物品
    let uid = db
        .load_inventory(char_index)
        .ok()
        .and_then(|inv| inv.into_iter().find(|(_, it)| it.item_index == item_index).map(|(_, it)| it.unique_id));
    let Some(uid) = uid else { return };
    // 找到物品模板并确认是消耗品（heal>0）
    if !crate::items::find(item_index).map(|t| t.heal > 0).unwrap_or(false) {
        return;
    }
    let Some((_slot, item)) = db.find_inventory_item(char_index, uid).ok().flatten() else { return };
    if use_consumable(world, player_id, item.clone()).await {
        let _ = db.consume_inventory_item(char_index, uid);
    }
}

fn manhattan(a: Point, b: Point) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

// ---------------------------------------------------------------------------
// 战斗
// ---------------------------------------------------------------------------

/// 玩家攻击：命中正前方相邻格内的怪物并结算。返回是否命中。
pub async fn player_attack(
    world: &World,
    db: &crate::db::Database,
    player_id: u32,
    direction: MirDirection,
) -> bool {
    let Some(player) = world.get_player(player_id).await else {
        return false;
    };
    if player.dead {
        return false;
    }
    world
        .broadcast_on(player.map_index, encode_packet(&s::ObjectAttack {
            object_id: player_id,
            location: player.location,
            direction,
            spell: 0,
            level: 0,
            r#type: 0,
        }))
        .await;
    let (dx, dy) = direction_offset(direction, 1);
    let target_pos = Point::new(player.location.x + dx, player.location.y + dy);

    // 首选正前方邻格；若怪物贴侧/斜角，则回退到任意相邻死亡与否的怪物（同图）
    let target_monster = {
        let mons = world.monsters.lock().await;
        let front = mons
            .values()
            .find(|m| !m.dead && m.map_index == player.map_index && m.location == target_pos)
            .map(|m| m.object_id);
        front.or_else(|| {
            [
                (-1, -1), (0, -1), (1, -1),
                (-1, 0), (1, 0),
                (-1, 1), (0, 1), (1, 1),
            ]
            .iter()
            .map(|(ox, oy)| Point::new(player.location.x + ox, player.location.y + oy))
            .find_map(|cand| {
                mons
                    .values()
                    .find(|m| !m.dead && m.map_index == player.map_index && m.location == cand)
                    .map(|m| m.object_id)
            })
        })
    };
    let Some(monster_id) = target_monster else { return false };

    let dmg = {
        let mons = world.monsters.lock().await;
        let m = mons.get(&monster_id).unwrap();
        (player.attack + rand_range(1, 3) - m.defence.max(0)).max(1)
    };
    player_hit_monster(world, db, player_id, monster_id, dmg).await;
    damage_equipment(world, db, player_id, 0, 1).await; // 武器耐久消耗
    true
}

/// 玩家魔法攻击（范围指向）：消耗 MP，对朝向射线上的首个怪物打出一发法术。
/// 返回是否施放成功（射程内有目标且有足额 MP）。
pub async fn player_magic_attack(
    world: &World,
    db: &crate::db::Database,
    player_id: u32,
    direction: MirDirection,
    spell: u8,
) -> bool {
    let Some(player) = world.get_player(player_id).await else {
        return false;
    };
    if player.dead {
        return false;
    }
    let Some(tmpl) = crate::magics::find(spell) else {
        return false;
    };
    if player.mp < tmpl.base_cost as i32 {
        return false; // MP 不足
    }

    // 选取目标：首选朝向射线上（射程内）的怪物；否则退化为朝向半平面内的最近怪物
    let (dx, dy) = direction_offset(direction, 1);
    let mut target_monster: Option<(u32, Point)> = None;
    {
        let mons = world.monsters.lock().await;
        'line: for step in 1..=tmpl.range as i32 {
            let pos = Point::new(player.location.x + dx * step, player.location.y + dy * step);
            if let Some(m) = mons
                .values()
                .find(|m| !m.dead && m.map_index == player.map_index && m.location == pos)
            {
                target_monster = Some((m.object_id, m.location));
                break 'line;
            }
        }
    }
    if target_monster.is_none() {
        // 半平面兜底：朝向方向分量对齐、且在射程内的最近怪物
        let mut best: Option<(u32, Point, i32)> = None;
        let mons = world.monsters.lock().await;
        for m in mons.values().filter(|m| !m.dead && m.map_index == player.map_index) {
            let ox = m.location.x - player.location.x;
            let oy = m.location.y - player.location.y;
            // 朝向分量需同向：前方方向的分量 > 0
            let aligned = (dx != 0 && ox.signum() == dx) || (dy != 0 && oy.signum() == dy);
            let dist = manhattan(player.location, m.location);
            if aligned && dist <= tmpl.range as i32 {
                if best.map(|(_, _, bd)| dist < bd).unwrap_or(true) {
                    best = Some((m.object_id, m.location, dist));
                }
            }
        }
        target_monster = best.map(|(id, loc, _)| (id, loc));
    }
    let Some((target_id, target_pos)) = target_monster else {
        return false; // 射程内无目标
    };

    // 扣除 MP
    {
        let mut players = world.players.lock().await;
        if let Some(p) = players.get_mut(&player_id) {
            p.mp = (p.mp - tmpl.base_cost as i32).max(0);
        }
    }
    let p = world.get_player(player_id).await;
    if let Some(p) = p {
        world
            .send_to(
                player_id,
                encode_packet(&s::HealthChanged { hp: p.hp, mp: p.mp }),
            )
            .await;
    }

    // 施法动画（前台广播 ObjectMagic）
    world
        .broadcast_on(player.map_index, encode_packet(&s::ObjectMagic {
            object_id: player_id,
            location: player.location,
            direction,
            spell,
            target_id,
            target: target_pos,
            cast: true,
            level: 1,
            self_broadcast: true,
            secondary_target_ids: vec![],
        }))
        .await;

    // 命中结算（复用物理近战的击杀/掉落/经验逻辑）
    let dmg = (tmpl.damage + player.attack / 2 - monster_defence(world, target_id).await.max(0)).max(1);
    player_hit_monster(world, db, player_id, target_id, dmg).await;
    true
}

/// 弓箭手远程攻击最大射程（同 C# `Globals.MaxAttackRange`）
pub const MAX_ATTACK_RANGE: i32 = 9;

/// 玩家远程攻击（弓/弩，对应 C# `PlayerObject.RangeAttack`）：
/// 按 target_id 寻找射程内的怪物，伤害随距离衰减；表现层为
/// 自己收到 `RangeAttack` 回执、同图广播 `ObjectRangeAttack`。
/// 返回是否命中目标。
pub async fn player_range_attack(
    world: &World,
    db: &crate::db::Database,
    player_id: u32,
    direction: MirDirection,
    target_id: u32,
    target_location: Point,
) -> bool {
    let Some(player) = world.get_player(player_id).await else {
        return false;
    };
    if player.dead {
        return false;
    }

    // 目标校验：必须存活、同图、在最大射程内
    let found = {
        let mons = world.monsters.lock().await;
        mons.get(&target_id)
            .filter(|m| !m.dead && m.map_index == player.map_index)
            .map(|m| (m.location, manhattan(player.location, m.location)))
    };
    let Some((target_pos, dist)) = found else {
        return false;
    };
    if dist > MAX_ATTACK_RANGE {
        return false;
    }
    // 客户端上报的目标位置仅作参考，以服务器实测位置为准（防作弊同步）
    let _ = target_location;

    // 表现：自己收到回执 + 同图广播（含未命中也广播动作）
    world
        .send_to(
            player_id,
            encode_packet(&s::RangeAttack {
                target_id,
                target: target_pos,
                spell: 0,
            }),
        )
        .await;
    world
        .broadcast_on_except(
            player.map_index,
            encode_packet(&s::ObjectRangeAttack {
                object_id: player_id,
                location: player.location,
                direction,
                target_id,
                target: target_pos,
                r#type: 0,
                spell: 0,
                level: 0,
            }),
            player_id,
        )
        .await;

    // 命中率随距离线性下降（同 C# chanceToHit），简化为距离越远伤害越低
    let falloff = 1 + (MAX_ATTACK_RANGE - dist) / 3;
    let dmg = (player.attack * falloff.max(1) + rand_range(1, 3)).max(1);
    player_hit_monster(world, db, player_id, target_id, dmg).await;
    damage_equipment(world, db, player_id, 0, 1).await; // 武器耐久消耗
    true
}

async fn monster_defence(world: &World, monster_id: u32) -> i32 {
    let mons = world.monsters.lock().await;
    mons.get(&monster_id).map(|m| m.defence).unwrap_or(0)
}

pub async fn player_hit_monster(
    world: &World,
    db: &crate::db::Database,
    player_id: u32,
    monster_id: u32,
    dmg: i32,
) {
    let mut died = false;
    let mut monster_map = 0u32;
    {
        let mut mons = world.monsters.lock().await;
        if let Some(m) = mons.get_mut(&monster_id) {
            if m.dead {
                return;
            }
            monster_map = m.map_index;
            m.target = Some(player_id);
            m.cooldown = 2;
            m.hp -= dmg;
            if m.hp <= 0 {
                m.hp = 0;
                m.dead = true;
                m.dead_ticks = 0;
                died = true;
            }
        }
    }
    world
        .broadcast_on(monster_map, encode_packet(&s::DamageIndicator {
            damage: dmg,
            r#type: 0,
            object_id: player_id,
        }))
        .await;
    let loc = {
        let mons = world.monsters.lock().await;
        mons.get(&monster_id).map(|m| m.location).unwrap_or(Point::new(0, 0))
    };
    world
        .broadcast_on(monster_map, encode_packet(&s::ObjectStruck {
            object_id: monster_id,
            attacker_id: player_id,
            location: loc,
            direction: MirDirection::Up,
        }))
        .await;
    if died {
        monster_died(world, db, player_id, monster_id).await;
    }
}

async fn monster_died(
    world: &World,
    db: &crate::db::Database,
    player_id: u32,
    monster_id: u32,
) {
    let (loc, drops, exp_reward, gold_reward, map_index, monster_image) = {
        let mons = world.monsters.lock().await;
        let m = mons.get(&monster_id).unwrap();
        (
            m.location,
            m.drops.clone(),
            m.exp_reward,
            m.gold_reward,
            m.map_index,
            m.image,
        )
    };
    world
        .broadcast_on(map_index, encode_packet(&s::ObjectDied {
            object_id: monster_id,
            location: loc,
            direction: MirDirection::Up,
            r#type: 0,
        }))
        .await;
    for &item_index in &drops {
        if item_index <= 0 {
            continue;
        }
        let oid = world.next_object_id();
        world
            .add_ground_item(GroundItem {
                object_id: oid,
                item_index,
                count: 1,
                location: loc,
                unique_id: world.next_item_unique(),
                map_index,
                gold: 0,
            })
            .await;
    }
    if exp_reward > 0 {
        gain_experience(world, player_id, exp_reward).await;
    }
    if gold_reward > 0 {
        gain_gold(world, player_id, gold_reward).await;
    }
    // 任务击杀进度登记（击杀怪物可能推进任务）
    let player_name = world
        .get_player(player_id)
        .await
        .map(|p| p.name)
        .unwrap_or_default();
    if !player_name.is_empty() {
        if let Some(char_index) = world
            .get_player(player_id)
            .await
            .map(|p| p.character_index)
        {
            crate::quest::register_kill(&player_name, char_index, monster_image, db);
        }
    }
}

async fn monster_hit_player(
    world: &World,
    db: &crate::db::Database,
    monster_id: u32,
    player_id: u32,
    dmg: i32,
) {
    {
        let mut mons = world.monsters.lock().await;
        if let Some(m) = mons.get_mut(&monster_id) {
            m.cooldown = 3;
        }
    }
    let (loc, dir, map_index, mut killer, mut hp, mut mp) = {
        let players = world.players.lock().await;
        let p = players.get(&player_id);
        match p {
            Some(p) => (p.location, p.direction, p.map_index, false, p.hp, p.mp),
            None => return,
        }
    };
    {
        let mut players = world.players.lock().await;
        if let Some(p) = players.get_mut(&player_id) {
            p.hp -= dmg;
            if p.hp <= 0 {
                p.hp = 0;
                killer = true;
            }
            hp = p.hp;
            mp = p.mp;
        }
    }
    world
        .send_to(
            player_id,
            encode_packet(&s::DamageIndicator {
                damage: dmg,
                r#type: 1,
                object_id: player_id,
            }),
        )
        .await;
    world
        .send_to(player_id, encode_packet(&s::HealthChanged { hp, mp }))
        .await;
    // 被击表现：自己收 Struck（同 C#），同图广播 ObjectStruck（object_id=被打者）
    world
        .send_to(player_id, encode_packet(&s::Struck { attacker_id: monster_id }))
        .await;
    world
        .broadcast_on_except(
            map_index,
            encode_packet(&s::ObjectStruck {
                object_id: player_id,
                attacker_id: monster_id,
                location: loc,
                direction: dir,
            }),
            player_id,
        )
        .await;
    // 被击中：护甲耐久消耗
    damage_equipment(world, db, player_id, 1, 1).await;
    if killer {
        player_died(world, player_id).await;
    }
}

/// 玩家死亡（死亡保留）：原地倒地，不自动复活、不回城。
/// 表现同 C# `PlayerObject.Die`：自己收 `Death`，同图广播 `ObjectDied`。
/// 之后由玩家主动 TownRevive（或登录恢复死亡态）处理。
pub async fn player_died(world: &World, player_id: u32) {
    let (loc, dir, map_index) = {
        let mut players = world.players.lock().await;
        match players.get_mut(&player_id) {
            Some(p) => {
                p.hp = 0;
                p.dead = true;
                (p.location, p.direction, p.map_index)
            }
            None => return,
        }
    };
    world
        .send_to(
            player_id,
            encode_packet(&s::Death {
                location: loc,
                direction: dir,
            }),
        )
        .await;
    world
        .broadcast_on(
            map_index,
            encode_packet(&s::ObjectDied {
                object_id: player_id,
                location: loc,
                direction: dir,
                r#type: 0,
            }),
        )
        .await;
}

/// 回城复活（对应 C# `PlayerObject.TownRevive`）：仅对死亡玩家生效。
/// 传送回出生点（复用传送门的地图信息下发），满血满蓝复活；
/// 表现：自己收 `Revived`，同图广播 `ObjectRevived`。未死亡返回 false。
pub async fn revive_player(world: &World, player_id: u32) -> bool {
    let is_dead = world
        .get_player(player_id)
        .await
        .map(|p| p.dead)
        .unwrap_or(false);
    if !is_dead {
        return false;
    }

    // 回城：传送到出生点附近可通行格
    let (sx, sy) = world.nearest_walkable_on(0, SPAWN.x, SPAWN.y);
    world.teleport_player(player_id, 0, sx, sy).await;

    let (map_index, hp, mp) = {
        let mut players = world.players.lock().await;
        match players.get_mut(&player_id) {
            Some(p) => {
                p.dead = false;
                p.hp = p.max_hp;
                p.mp = p.max_mp;
                (p.map_index, p.hp, p.mp)
            }
            None => return false,
        }
    };
    world
        .send_to(player_id, encode_packet(&s::HealthChanged { hp, mp }))
        .await;
    world.send_to(player_id, encode_packet(&s::Revived)).await;
    world
        .broadcast_on_except(
            map_index,
            encode_packet(&s::ObjectRevived {
                object_id: player_id,
                effect: true,
            }),
            player_id,
        )
        .await;
    true
}

// ---------------------------------------------------------------------------
// 经验 / 金币 / 升级
// ---------------------------------------------------------------------------

/// 升级所需经验：level * 10
fn xp_needed(level: u16) -> u32 {
    (level as u32) * 10
}

pub async fn gain_experience(world: &World, player_id: u32, amount: u32) {
    world
        .send_to(player_id, encode_packet(&s::GainExperience { amount }))
        .await;
    let mut leveled = false;
    {
        let mut players = world.players.lock().await;
        if let Some(p) = players.get_mut(&player_id) {
            p.experience += amount;
            while p.level < 50 && p.experience >= xp_needed(p.level) {
                p.experience -= xp_needed(p.level);
                p.level += 1;
                p.max_hp += 5;
                p.hp = p.max_hp;
                p.max_mp += 3;
                p.mp = p.max_mp;
                p.attack += 1;
                leveled = true;
            }
        }
    }
    if leveled {
        let p = world.get_player(player_id).await.unwrap_or_else(|| unreachable!());
        world
            .send_to(
                player_id,
                encode_packet(&s::HealthChanged {
                    hp: p.max_hp,
                    mp: p.max_mp,
                }),
            )
            .await;
        world.broadcast(encode_packet(&s::ObjectPlayer {
            object_id: p.object_id,
            name: p.name,
            guild_name: String::new(),
            guild_rank_name: String::new(),
            name_colour: Argb(0),
            class: p.class,
            gender: p.gender,
            level: p.level,
            location: p.location,
            direction: p.direction,
            hair: 0,
            light: 0,
            weapon: p.weapon,
            weapon_effect: 0,
            armour: p.armour,
            poison: 0,
            dead: false,
            hidden: false,
            effect: 0,
            wing_effect: 0,
            extra: false,
            mount_type: 0,
            riding_mount: false,
            fishing: false,
            transform_type: 0,
            element_orb_effect: 0,
            element_orb_lvl: 0,
            element_orb_max: 0,
            buffs: vec![],
            level_effects: crystal_protocol::types::LevelEffects(0),
        }));
    }
}

pub async fn gain_gold(world: &World, player_id: u32, amount: u32) {
    {
        let mut players = world.players.lock().await;
        if let Some(p) = players.get_mut(&player_id) {
            p.gold = p.gold.saturating_add(amount);
        }
    }
    world
        .send_to(player_id, encode_packet(&s::GainedGold { gold: amount }))
        .await;
}

/// 玩家当前金币
pub async fn player_gold(world: &World, player_id: u32) -> u32 {
    world
        .players
        .lock()
        .await
        .get(&player_id)
        .map(|p| p.gold)
        .unwrap_or(0)
}

/// 扣除金币；余额不足返回 false
pub async fn remove_gold(world: &World, player_id: u32, amount: u32) -> bool {
    let mut ok = false;
    {
        let mut players = world.players.lock().await;
        if let Some(p) = players.get_mut(&player_id) {
            if p.gold >= amount {
                p.gold -= amount;
                ok = true;
            }
        }
    }
    if ok {
        world
            .send_to(player_id, encode_packet(&s::LoseGold { gold: amount }))
            .await;
    }
    ok
}

/// 查找 NPC 商人的在售物品索引列表
pub async fn npc_shop(world: &World, npc_object_id: u32) -> Option<Vec<i32>> {
    let npcs = world.npcs.lock().await.clone();
    npcs
        .into_iter()
        .find(|n| n.object_id == npc_object_id)
        .map(|n| n.shop_items)
}

/// 按 object_id 查 NPC 名字；无返回 None。
pub async fn npc_name(world: &World, npc_object_id: u32) -> Option<String> {
    let npcs = world.npcs.lock().await.clone();
    npcs
        .into_iter()
        .find(|n| n.object_id == npc_object_id)
        .map(|n| n.name)
}

// ---------------------------------------------------------------------------
// 装备 / 道具使用
// ---------------------------------------------------------------------------

/// 装备槽数量（含未使用槽位，与 C# `EquipmentSlot` 枚举一致）
pub const EQUIPMENT_SLOTS: usize = 14;

/// 把装备映射为固定 14 槽的 `Vec<Option<UserItem>>`（空槽为 None）。
pub fn equipment_slots(player: &Player) -> Vec<Option<UserItem>> {
    let mut slots: Vec<Option<UserItem>> = (0..EQUIPMENT_SLOTS).map(|_| None).collect();
    for (slot, item) in &player.equipment {
        if let Some(s) = slots.get_mut(*slot as usize) {
            *s = Some(item.clone());
        }
    }
    slots
}

/// 根据已穿戴装备重算玩家的攻击/防御（叠加装备 bonus；基础值由等级决定）。
pub fn recompute_stats(player: &mut Player) {
    // 基础攻击/防御来自职业与等级（与 net::base_stats 逻辑一致）
    let base_attack = 1 + player.level as i32 / 2;
    let base_defence = player.level as i32 / 2;
    let mut attack = base_attack;
    let mut defence = base_defence;
    player.weapon = 0;
    player.armour = 0;
    for (slot, item) in &player.equipment {
        let Some(tmpl) = crate::items::find(item.item_index) else { continue };
        // 耐久为 0 时装备失效（不提供加成/不显示外观）
        let broken = item.max_dura > 0 && item.current_dura == 0;
        match slot {
            0 => {
                // 武器槽：加攻击
                if !broken {
                    player.weapon = tmpl.index as i16;
                    attack += tmpl.bonus;
                }
            }
            1 => {
                // 护甲槽：加防御
                if !broken {
                    player.armour = tmpl.index as i16;
                    defence += tmpl.bonus;
                }
            }
            _ => {}
        }
    }
    player.attack = attack;
    player.defence = defence;
}

/// 扣除玩家某装备槽的耐久（同步 DB 与内存，并在耐久耗尽时刷新外观/属性）。
pub async fn damage_equipment(
    world: &World,
    db: &crate::db::Database,
    player_id: u32,
    slot: i32,
    amount: u16,
) {
    let Some(player) = world.get_player(player_id).await else { return };
    let char_index = player.character_index;
    let Ok((new_cd, md)) = db.damage_equipment(char_index, slot, amount) else {
        return;
    };
    if md == 0 {
        return;
    }
    let mut broke_now = false;
    {
        let mut players = world.players.lock().await;
        if let Some(p) = players.get_mut(&player_id) {
            if let Some(item) = p.equipment.get_mut(&slot) {
                let prev = item.current_dura;
                item.current_dura = new_cd;
                if prev > 0 && new_cd == 0 {
                    broke_now = true;
                }
            }
            recompute_stats(p);
        }
    }
    if broke_now {
        // 耐久耗尽，通知玩家
        world
            .send_to(
                player_id,
                encode_packet(&s::ObjectChat {
                    object_id: player_id,
                    text: "你的装备耐久耗尽，已失效！请到铁匠处修理。".to_string(),
                    r#type: 1,
                }),
            )
            .await;
    }
}

/// 使用物品：金创药等消耗品回复 HP。成功消耗并回复返回 (true, 用后剩余数量)。
/// `item` 为背包中待使用的物品（由调用方按 unique_id 查得）。
pub async fn use_consumable(world: &World, player_id: u32, item: UserItem) -> bool {
    let Some(tmpl) = crate::items::find(item.item_index) else {
        return false;
    };
    if tmpl.heal <= 0 {
        return false;
    }
    {
        let mut players = world.players.lock().await;
        if let Some(p) = players.get_mut(&player_id) {
            p.hp = (p.hp + tmpl.heal).min(p.max_hp);
        }
    }
    let p = world.get_player(player_id).await;
    if let Some(p) = p {
        world
            .send_to(
                player_id,
                encode_packet(&s::HealthChanged { hp: p.hp, mp: p.mp }),
            )
            .await;
    }
    true
}

// ---------------------------------------------------------------------------
// 拾取
// ---------------------------------------------------------------------------

/// 拾取玩家相邻格的地面物品并加入背包（DB 持久化）
pub async fn pick_up(world: &World, player_id: u32, db: &crate::db::Database) {
    let Some(player) = world.get_player(player_id).await else { return };
    let target = {
        let items = world.items.lock().await;
        items
            .values()
            .find(|gi| gi.map_index == player.map_index && manhattan(gi.location, player.location) <= 1)
            .map(|gi| gi.object_id)
    };
    let Some(oid) = target else { return };
    let gi = {
        let mut items = world.items.lock().await;
        items.remove(&oid)
    };
    let Some(gi) = gi else { return };
    world.remove_ground_item(oid).await;

    // 金币堆：直接加金币
    if gi.gold > 0 {
        gain_gold(world, player_id, gi.gold).await;
        return;
    }

    let ok = db.add_item_to_inventory(player.character_index, gi.item_index, gi.count);
    if ok.is_err() {
        return;
    }
    let item = UserItem {
        unique_id: gi.unique_id,
        item_index: gi.item_index,
        count: gi.count,
        current_dura: 0,
        max_dura: 0,
        ..Default::default()
    };
    world.send_to(player_id, encode_packet(&s::GainedItem { item })).await;
    // 背包变化需刷新（简化：用 UserSlotsRefresh 提示，此处先发 GainedItem 即可）
}

/// 采集（Harvest）：朝向方向割取尸体，获得怪物模板配置的采集物（如肉）。
/// 同 C# PlayerObject.Harvest：转身 → 自发 UserLocation + 广播 ObjectHarvest →
/// 在面前一圈找 dead 且未采集的同图尸体 → 直接入包并广播 ObjectHarvested。
pub async fn player_harvest(
    world: &World,
    db: &crate::db::Database,
    player_id: u32,
    direction: MirDirection,
) -> bool {
    let Some(player) = world.get_player(player_id).await else {
        return false;
    };
    if player.dead {
        return false;
    }
    // 转身 + 表现
    {
        let mut players = world.players.lock().await;
        if let Some(p) = players.get_mut(&player_id) {
            p.direction = direction;
        }
    }
    let (loc, map_index) = {
        let p = world.get_player(player_id).await.unwrap();
        (p.location, p.map_index)
    };
    world
        .send_to(
            player_id,
            encode_packet(&s::UserLocation { location: loc, direction }),
        )
        .await;
    world
        .broadcast_on_except(
            map_index,
            encode_packet(&s::ObjectHarvest { object_id: player_id, location: loc, direction }),
            player_id,
        )
        .await;

    // 在面前 3x3 范围找可采集尸体（同 C# front 周围 d=0..1）
    let (dx, dy) = direction_offset(direction, 1);
    let mut target: Option<(u32, i32)> = None; // (monster_id, harvest_item)
    {
        let mons = world.monsters.lock().await;
        'outer: for oy in -1..=1 {
            for ox in -1..=1 {
                let px = loc.x + dx + ox;
                let py = loc.y + dy + oy;
                if let Some(m) = mons.values().find(|m| {
                    m.dead && !m.harvested && m.map_index == map_index
                        && m.location.x == px && m.location.y == py
                }) {
                    if m.harvest_item > 0 {
                        target = Some((m.object_id, m.harvest_item));
                        break 'outer;
                    }
                }
            }
        }
    }
    let Some((mid, item_index)) = target else {
        // 没有可采集的尸体：系统提示（同 C# NoNearbyOwnedCarcasses 的简化版）
        world
            .send_to(
                player_id,
                encode_packet(&s::ObjectChat {
                    object_id: player_id,
                    text: "附近没有可采集的尸体".to_string(),
                    r#type: 1,
                }),
            )
            .await;
        return false;
    };

    // 标记已采集 + 发采集物
    if let Some(m) = world.monsters.lock().await.get_mut(&mid) {
        m.harvested = true;
    }
    if db.add_item_to_inventory(player.character_index, item_index, 1).is_err() {
        return false;
    }
    let item = UserItem {
        unique_id: 0,
        item_index,
        count: 1,
        ..Default::default()
    };
    world.send_to(player_id, encode_packet(&s::GainedItem { item })).await;
    let (mloc, mdir) = {
        let mons = world.monsters.lock().await;
        mons.get(&mid).map(|m| (m.location, m.direction)).unwrap()
    };
    world
        .broadcast_on(
            map_index,
            encode_packet(&s::ObjectHarvested { object_id: mid, location: mloc, direction: mdir }),
        )
        .await;
    true
}

/// 丢弃：把背包物品丢到玩家脚下（生成地面掉落物）。成功后返回 true。
/// `item` 为被丢弃物品的原始信息（由调用方先做 DB 扣减）。
pub async fn drop_ground_item(world: &World, player_id: u32, item: UserItem) -> bool {
    let Some(player) = world.get_player(player_id).await else {
        return false;
    };
    world
        .add_ground_item(GroundItem {
            object_id: world.next_object_id(),
            item_index: item.item_index,
            count: item.count,
            location: player.location,
            unique_id: item.unique_id,
            map_index: player.map_index,
            gold: 0,
        })
        .await;
    true
}

/// 丢弃金币到脚下（生成地面金币堆）。成功返回 true。
pub async fn drop_gold(world: &World, player_id: u32, amount: u32) -> bool {
    let Some(player) = world.get_player(player_id).await else {
        return false;
    };
    world
        .add_ground_item(GroundItem {
            object_id: world.next_object_id(),
            item_index: 0,
            count: 0,
            location: player.location,
            unique_id: 0,
            map_index: player.map_index,
            gold: amount,
        })
        .await;
    true
}

/// 把玩家当前状态（位置/血量/金币/经验/等级）写回 DB。掉线与定期备份共用。
pub async fn persist_player(world: &World, db: &crate::db::Database, player_id: u32) {
    let Some(p) = world.get_player(player_id).await else { return };
    let _ = db.save_character_state(
        p.character_index,
        p.location.x,
        p.location.y,
        p.direction as i32,
        p.hp,
        p.mp,
        p.gold as i64,
        p.experience as i64,
        p.level as i64,
    );
}

/// 把场上所有在线玩家状态写回 DB（世界 tick 定期备份 + 服务器停机前调用）。
pub async fn persist_all_players(world: &World, db: &crate::db::Database) {
    let players = world.players.lock().await.clone();
    let ids: Vec<u32> = players.keys().copied().collect();
    drop(players);
    for oid in ids {
        persist_player(world, db, oid).await;
    }
}

// ---------------------------------------------------------------------------
// 工具
// ---------------------------------------------------------------------------

fn rand_range(lo: i32, hi: i32) -> i32 {
    use rand::Rng;
    rand::thread_rng().gen_range(lo..=hi)
}

/// 方向偏移（Mir2: Y 向下增长）
pub fn direction_offset(dir: MirDirection, steps: i32) -> (i32, i32) {
    match dir {
        MirDirection::Up => (0, -steps),
        MirDirection::UpRight => (steps, -steps),
        MirDirection::Right => (steps, 0),
        MirDirection::DownRight => (steps, steps),
        MirDirection::Down => (0, steps),
        MirDirection::DownLeft => (-steps, steps),
        MirDirection::Left => (-steps, 0),
        MirDirection::UpLeft => (-steps, -steps),
    }
}

/// 地图边界与碰撞校验：返回新位置；若越界或被墙阻挡则 None。
impl World {
    pub fn try_move(&self, location: Point, dir: MirDirection, steps: i32) -> Option<Point> {
        let (dx, dy) = direction_offset(dir, steps);
        let nx = location.x + dx;
        let ny = location.y + dy;
        if !self.map.is_walkable(nx, ny) {
            return None;
        }
        Some(Point::new(nx, ny))
    }

    /// 是否可从 from 一步移动到 to（用于怪物追击校验）
    pub fn walkable_line(&self, from: Point, to: Point) -> bool {
        let (dx, dy) = (to.x - from.x, to.y - from.y);
        if dx.abs() > 1 || dy.abs() > 1 {
            return false;
        }
        self.map.is_walkable(to.x, to.y)
    }

    /// 怪物追击一步：在 8 个邻格中选距离目标最近的可行走格（可绕开简单障碍）。
    /// 返回 (新位置, 方向)；无可行走邻格则 None。
    pub fn chase_step(&self, from: Point, to: Point) -> Option<(Point, MirDirection)> {
        let all = [
            MirDirection::Up,
            MirDirection::UpRight,
            MirDirection::Right,
            MirDirection::DownRight,
            MirDirection::Down,
            MirDirection::DownLeft,
            MirDirection::Left,
            MirDirection::UpLeft,
        ];
        let (tx, ty) = (to.x, to.y);
        let mut best: Option<(Point, MirDirection, i32, i32)> = None; // (loc, dir, dist, tiebreak)
        for dir in all {
            let Some(cand) = self.try_move(from, dir, 1) else { continue };
            let d = manhattan(cand, to);
            // 平局偏好：先朝轴对齐方向走（更接近直线）
            let on_axis = (cand.x == tx) as i32 + (cand.y == ty) as i32;
            let better = match best {
                None => true,
                Some((_, _, bd, bt)) => d < bd || (d == bd && on_axis > bt),
            };
            if better {
                best = Some((cand, dir, d, on_axis));
            }
        }
        best.map(|(loc, dir, _, _)| (loc, dir))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chase_step_prioritizes_larger_axis() {
        // 目标在东偏北，东向差距更大 -> 先往东
        let from = Point::new(0, 0);
        assert_eq!(chase_step(from, Point::new(5, 2)), (1, 0));
        // 目标在北偏东，北向差距更大 -> 先往北
        assert_eq!(chase_step(from, Point::new(2, 5)), (0, 1));
        // 南北方向
        assert_eq!(chase_step(from, Point::new(0, 3)), (0, 1));
        // 东西方向
        assert_eq!(chase_step(from, Point::new(-3, 0)), (-1, 0));
    }

    #[test]
    fn dir_from_delta_cardinals_and_diagonals() {
        assert_eq!(dir_from_delta(0, -1), MirDirection::Up);
        assert_eq!(dir_from_delta(1, 0), MirDirection::Right);
        assert_eq!(dir_from_delta(1, 1), MirDirection::DownRight);
        assert_eq!(dir_from_delta(-1, -1), MirDirection::UpLeft);
        assert_eq!(dir_from_delta(0, 0), MirDirection::Up);
    }

    #[test]
    fn try_move_bounds_and_walls() {
        let world = World::new(); // 默认空地图 800x800 全通
        assert_eq!(world.try_move(Point::new(0, 0), MirDirection::Up, 1), None);
        assert_eq!(world.try_move(Point::new(0, 0), MirDirection::Right, 1), Some(Point::new(1, 0)));
        // 越界（地图右上角向右）
        assert_eq!(
            world.try_move(Point::new(799, 0), MirDirection::Right, 1),
            None
        );
    }

    #[test]
    fn try_move_respects_wall() {
        // 构造 3x3 地图，中心 (1,1) 设为墙
        let mut b = vec![0u8; 8 + 3 * 3 * 26];
        b[0] = 1;
        b[2] = 0x43;
        b[3] = 0x23;
        b[4..6].copy_from_slice(&3u16.to_le_bytes());
        b[6..8].copy_from_slice(&3u16.to_le_bytes());
        // 中心格子 (1,1) index = 1*3+1 = 4, cell offset = 8 + 4*26
        let cell = 8 + 4 * 26;
        let hi = cell + 2;
        let orig = u32::from_le_bytes([b[hi], b[hi + 1], b[hi + 2], b[hi + 3]]);
        let v = orig | 0x2000_0000;
        b[hi..hi + 4].copy_from_slice(&v.to_le_bytes());
        let m = crate::maps::load_map_bytes(0, &b).unwrap();
        let world = World::with_map(m);
        // 从 (1,0) 向下进中心墙 -> 被挡
        assert_eq!(world.try_move(Point::new(1, 0), MirDirection::Down, 1), None);
        // 从 (1,0) 向左到 (0,0) 可通行
        assert_eq!(world.try_move(Point::new(1, 0), MirDirection::Left, 1), Some(Point::new(0, 0)));
    }

    #[test]
    fn portal_at_matches_config() {
        let world = World::new();
        // 新手村 (404,412) -> 地图 100 (8,6)
        assert_eq!(world.portal_at(0, 404, 412), Some((100, 8, 6)));
        // 0100 (4,4) -> 新手村 (404,404)
        assert_eq!(world.portal_at(100, 4, 4), Some((0, 404, 404)));
        // 非传送门格不匹配
        assert_eq!(world.portal_at(0, 400, 400), None);
        assert_eq!(world.portal_at(1, 404, 412), None);
    }

    #[test]
    fn walk_onto_portal_detects_dest() {
        let world = World::new();
        // 从 (404,411) 向下走一步正好踏上传送门格 (404,412)
        let new_loc = world.try_move(Point::new(404, 411), MirDirection::Down, 1);
        assert_eq!(new_loc, Some(Point::new(404, 412)));
        if let Some(Point { x, y }) = new_loc {
            assert_eq!(world.portal_at(0, x, y), Some((100, 8, 6)));
        }
    }

    #[tokio::test]
    async fn teleport_player_switches_map() {
        // 构造 10x10 全通地图 0 与 100
        let mk = |w: u16, h: u16| {
            let mut b = vec![0u8; 8 + w as usize * h as usize * 26];
            b[0] = 1;
            b[2] = 0x43;
            b[3] = 0x23;
            b[4..6].copy_from_slice(&w.to_le_bytes());
            b[6..8].copy_from_slice(&h.to_le_bytes());
            crate::maps::load_map_bytes(0, &b).unwrap()
        };
        let mut m0 = mk(10, 10);
        m0.index = 0;
        let mut m100 = mk(10, 10);
        m100.index = 100;
        let world = World::with_map(m0);
        world.register_map(m100);

        let (tx, _rx) = mpsc::channel(16);
        let player = Player {
            object_id: 1,
            account_id: "t".into(),
            name: "T".into(),
            class: MirClass::Warrior,
            gender: MirGender::Male,
            level: 1,
            location: Point::new(404, 411),
            direction: MirDirection::Down,
            max_hp: 100,
            hp: 100,
            max_mp: 10,
            mp: 10,
            attack: 1,
            defence: 0,
            experience: 0,
            gold: 0,
            weapon: 0,
            armour: 0,
            character_index: 1,
            sender: tx,
            hp_changed: false,
            equipment: std::collections::BTreeMap::new(),
            map_index: 0,
            auto_pot_item: 0,
            auto_pot_hp: 0,
            recently_sold: vec![],
            fishing: false,
            fishing_progress: 0,
            auto_fish: false,
            storage_unlocked: false,
            pending_mentor: None,
            can_be_mentor: false,
            pending_marriage: None,
            dead: false,
            a_mode: 0,
            allow_group: true,
            p_mode: 0,
            magic_keys: std::collections::HashMap::new(),
        };
        world.players.lock().await.insert(1, player);

        // 传送门触发动作：跨地图传送
        let ok = world.teleport_player(1, 100, 8, 6).await;
        assert!(ok, "传送失败");
        let p = world.players.lock().await.get(&1).cloned().unwrap();
        assert_eq!(p.map_index, 100, "应切换到地图 100");
        // 目标位置落在 100 地图可通行格
        let map = world.get_map(100);
        assert!(map.is_walkable(p.location.x, p.location.y));
    }

    #[tokio::test]
    async fn monsters_are_tagged_with_map() {
        let world = World::new();
        // 手动布两只不同地图的怪
        let mk = |oid: u32, mi: u32| Monster {
            object_id: oid,
            name: format!("M{oid}"),
            image: 1,
            location: Point::new(oid as i32, 0),
            direction: MirDirection::Up,
            level: 1,
            max_hp: 10,
            hp: 10,
            attack: 1,
            defence: 0,
            exp_reward: 5,
            gold_reward: 1,
            drops: vec![],
            dead: false,
            dead_ticks: 0,
            harvested: false,
            harvest_item: 0,
            target: None,
            cooldown: 0,
            map_index: mi,
            ranged: false,
        };
        world.add_monster(mk(1, 0)).await;
        world.add_monster(mk(2, 100)).await;
        let mons = world.monsters.lock().await.clone();
        assert_eq!(mons.get(&1).unwrap().map_index, 0);
        assert_eq!(mons.get(&2).unwrap().map_index, 100);
    }

    // ------------------------------------------------------------------
    // 死亡保留 / 复活 / 远程攻击
    // ------------------------------------------------------------------

    async fn add_test_player(world: &World, oid: u32, loc: Point) {
        let (tx, _rx) = mpsc::channel(64);
        world.players.lock().await.insert(
            oid,
            Player {
                object_id: oid,
                account_id: "t".into(),
                name: format!("P{oid}"),
                class: MirClass::Warrior,
                gender: MirGender::Male,
                level: 5,
                location: loc,
                direction: MirDirection::Up,
                max_hp: 100,
                hp: 100,
                max_mp: 50,
                mp: 50,
                attack: 10,
                defence: 2,
                experience: 0,
                gold: 0,
                weapon: 0,
                armour: 0,
                character_index: oid as i32,
                sender: tx,
                hp_changed: false,
                equipment: std::collections::BTreeMap::new(),
                map_index: 0,
                auto_pot_item: 0,
                auto_pot_hp: 0,
                recently_sold: vec![],
                fishing: false,
                fishing_progress: 0,
                auto_fish: false,
                storage_unlocked: false,
                pending_mentor: None,
                can_be_mentor: false,
                pending_marriage: None,
                dead: false,
                a_mode: 0,
                allow_group: true,
                p_mode: 0,
                magic_keys: std::collections::HashMap::new(),
            },
        );
    }

    fn test_db() -> Arc<crate::db::Database> {
        let path = std::env::temp_dir().join(format!(
            "crystal_world_test_{}.db",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        crate::db::Database::open(path).expect("测试数据库创建失败")
    }

    #[tokio::test]
    async fn death_retention_keeps_corpse_until_town_revive() {
        let world = World::new();
        let db = test_db();
        add_test_player(&world, 1, Point::new(420, 420)).await;

        // 死亡：原地倒地，不回城不复活
        player_died(&world, 1).await;
        {
            let p = world.players.lock().await.get(&1).cloned().unwrap();
            assert!(p.dead, "死亡后应保持死亡状态");
            assert_eq!(p.hp, 0);
            assert_eq!(p.location, Point::new(420, 420), "尸体应留在原地");
        }

        // 死亡期间不自动回血（regen 跳过尸体）
        regen_players(&world, &db).await;
        let p = world.players.lock().await.get(&1).cloned().unwrap();
        assert_eq!(p.hp, 0, "死亡期间不应回血");

        // TownRevive：满状态回城复活
        let ok = revive_player(&world, 1).await;
        assert!(ok);
        let p = world.players.lock().await.get(&1).cloned().unwrap();
        assert!(!p.dead);
        assert_eq!(p.hp, p.max_hp, "回城复活应满血");
        assert_eq!(
            p.location,
            Point::new(SPAWN.x, SPAWN.y),
            "复活应回到出生点"
        );
    }

    #[tokio::test]
    async fn revive_rejects_alive_player() {
        let world = World::new();
        add_test_player(&world, 1, Point::new(400, 400)).await;
        assert!(!revive_player(&world, 1).await, "活人不应能回城复活");
    }

    #[tokio::test]
    async fn range_attack_hits_within_max_range_only() {
        let world = World::new();
        let db = test_db();
        add_test_player(&world, 1, Point::new(400, 400)).await;

        let mk_monster = |oid: u32, loc: Point| Monster {
            object_id: oid,
            name: "靶子".into(),
            image: 3,
            location: loc,
            direction: MirDirection::Up,
            level: 1,
            max_hp: 1000,
            hp: 1000,
            attack: 0,
            defence: 0,
            exp_reward: 0,
            gold_reward: 0,
            drops: vec![],
            dead: false,
            dead_ticks: 0,
            harvested: false,
            harvest_item: 0,
            target: None,
            cooldown: 0,
            map_index: 0,
            ranged: false,
        };
        // 射程内（距离 5）
        world.add_monster(mk_monster(10, Point::new(405, 400))).await;
        let hit = player_range_attack(&world, &db, 1, MirDirection::Right, 10, Point::new(405, 400)).await;
        assert!(hit, "射程内的远程攻击应命中");
        let hp = world.monsters.lock().await.get(&10).unwrap().hp;
        assert!(hp < 1000, "命中后怪物应掉血");

        // 超射程（距离 > MAX_ATTACK_RANGE）
        world.add_monster(mk_monster(11, Point::new(400 + MAX_ATTACK_RANGE + 2, 400))).await;
        let miss = player_range_attack(
            &world,
            &db,
            1,
            MirDirection::Right,
            11,
            Point::new(400 + MAX_ATTACK_RANGE + 2, 400),
        )
        .await;
        assert!(!miss, "超射程的远程攻击不应命中");
        assert_eq!(
            world.monsters.lock().await.get(&11).unwrap().hp,
            1000,
            "未命中不掉血"
        );
    }

    #[tokio::test]
    async fn ranged_monster_shoots_without_moving() {
        let world = World::new();
        let db = test_db();
        add_test_player(&world, 1, Point::new(400, 400)).await;
        // 远程怪距玩家 4 格（>1 相邻、<=6 射程）
        world
            .add_monster(Monster {
                object_id: 7,
                name: "远程蛛".into(),
                image: 4,
                location: Point::new(404, 400),
                direction: MirDirection::Left,
                level: 4,
                max_hp: 26,
                hp: 26,
                attack: 5,
                defence: 2,
                exp_reward: 20,
                gold_reward: 12,
                drops: vec![],
                dead: false,
                dead_ticks: 0,
                harvested: false,
                harvest_item: 0,
                target: None,
                cooldown: 0,
                map_index: 0,
                ranged: true,
            })
            .await;

        monster_ai(&world, &db).await;

        let p = world.players.lock().await.get(&1).cloned().unwrap();
        assert!(p.hp < p.max_hp, "远程怪应在射程内打到玩家");
        let m = world.monsters.lock().await.get(&7).cloned().unwrap();
        assert_eq!(m.location, Point::new(404, 400), "远程怪射击时不应移动");
        assert_eq!(m.target, Some(1), "应锁定仇恨目标");
    }

    #[tokio::test]
    async fn monsters_ignore_dead_players() {
        let world = World::new();
        let db = test_db();
        add_test_player(&world, 1, Point::new(400, 400)).await;
        world
            .add_monster(Monster {
                object_id: 7,
                name: "近战骷".into(),
                image: 3,
                location: Point::new(401, 400),
                direction: MirDirection::Left,
                level: 3,
                max_hp: 20,
                hp: 20,
                attack: 3,
                defence: 1,
                exp_reward: 12,
                gold_reward: 8,
                drops: vec![],
                dead: false,
                dead_ticks: 0,
                harvested: false,
                harvest_item: 0,
                target: None,
                cooldown: 0,
                map_index: 0,
                ranged: false,
            })
            .await;
        // 玩家死亡 -> 怪不应索敌/攻击
        player_died(&world, 1).await;
        monster_ai(&world, &db).await;
        let m = world.monsters.lock().await.get(&7).cloned().unwrap();
        assert_eq!(m.target, None, "死亡玩家不应被索敌");
        let p = world.players.lock().await.get(&1).cloned().unwrap();
        assert_eq!(p.hp, 0, "尸体不应再被打");
    }

    #[tokio::test]
    async fn harvest_dead_monster_grants_item_and_marks_harvested() {
        let world = World::new();
        let db = test_db();
        add_test_player(&world, 1, Point::new(400, 400)).await;
        // 面前一格（Right -> 401,400）的可采集尸体
        world
            .add_monster(Monster {
                object_id: 10,
                name: "骷髅".into(),
                image: 3,
                location: Point::new(401, 400),
                direction: MirDirection::Up,
                level: 3,
                max_hp: 20,
                hp: 0,
                attack: 0,
                defence: 0,
                exp_reward: 0,
                gold_reward: 0,
                drops: vec![],
                dead: true,
                dead_ticks: 1,
                harvested: false,
                harvest_item: 6,
                target: None,
                cooldown: 0,
                map_index: 0,
                ranged: false,
            })
            .await;
        let ok = player_harvest(&world, &db, 1, MirDirection::Right).await;
        assert!(ok, "采集可采集尸体应成功");
        assert!(
            world.monsters.lock().await.get(&10).unwrap().harvested,
            "尸体应标记为已采集"
        );
        // 背包应出现肉(item 6)
        let slots = db.inventory_slots(1).unwrap();
        assert!(slots
            .iter()
            .any(|s| s.as_ref().map(|it| it.item_index == 6).unwrap_or(false)));
        // 已采集的尸体不能再采
        let again = player_harvest(&world, &db, 1, MirDirection::Right).await;
        assert!(!again, "重复采集应失败");
    }

    #[tokio::test]
    async fn harvest_ignores_alive_monsters_and_empty_cells() {
        let world = World::new();
        let db = test_db();
        add_test_player(&world, 1, Point::new(400, 400)).await;
        // 面前一格是活怪：不可采集
        world
            .add_monster(Monster {
                object_id: 11,
                name: "蜘蛛".into(),
                image: 4,
                location: Point::new(401, 400),
                direction: MirDirection::Up,
                level: 4,
                max_hp: 26,
                hp: 26,
                attack: 5,
                defence: 2,
                exp_reward: 20,
                gold_reward: 12,
                drops: vec![],
                dead: false,
                dead_ticks: 0,
                harvested: false,
                harvest_item: 6,
                target: None,
                cooldown: 0,
                map_index: 0,
                ranged: true,
            })
            .await;
        let ok = player_harvest(&world, &db, 1, MirDirection::Right).await;
        assert!(!ok, "活怪不可采集");
        // 空地同样失败
        let none = player_harvest(&world, &db, 1, MirDirection::Left).await;
        assert!(!none, "无尸体处采集应失败");
    }
}
