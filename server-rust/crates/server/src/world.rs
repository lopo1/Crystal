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
}

/// 地面掉落物
#[derive(Debug, Clone)]
pub struct GroundItem {
    pub object_id: u32,
    pub item_index: i32,
    pub count: u16,
    pub location: Point,
    pub unique_id: u64,
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
    /// 攻击目标玩家 / 攻击冷却
    pub target: Option<u32>,
    pub cooldown: u32,
}

/// 静态 NPC（用 ObjectPlayer 显示）
#[derive(Debug, Clone)]
pub struct Npc {
    pub object_id: u32,
    pub name: String,
    pub image: u16,
    pub location: Point,
    pub shop_items: Vec<i32>,
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
}

impl World {
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(512);
        World {
            players: Arc::new(Mutex::new(HashMap::new())),
            monsters: Arc::new(Mutex::new(HashMap::new())),
            items: Arc::new(Mutex::new(HashMap::new())),
            npcs: Arc::new(Mutex::new(Vec::new())),
            broadcast_tx,
            next_object_id: Arc::new(std::sync::atomic::AtomicU32::new(1)),
            next_item_unique: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
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
        }), p.object_id)
        .await;
        self.players.lock().await.insert(player.object_id, player);
        // 把场上怪物/掉落/NPC 同步给新玩家
        self.send_world_state_to(p.object_id).await;
    }

    async fn send_world_state_to(&self, oid: u32) {
        // 怪物
        let monsters = self.monsters.lock().await.clone();
        for m in monsters.values() {
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
        // 掉落物
        let items = self.items.lock().await.clone();
        for gi in items.values() {
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
        // NPC
        let npcs = self.npcs.lock().await.clone();
        for n in npcs.iter() {
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
        // 若已存在则更新广播
        self.broadcast(encode_packet(&s::ObjectMonster {
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
        }));
        self.monsters.lock().await.insert(m.object_id, m);
    }

    pub async fn remove_monster(&self, object_id: u32) {
        self.monsters.lock().await.remove(&object_id);
        self.broadcast(encode_packet(&s::ObjectRemove { object_id }));
    }

    pub async fn add_ground_item(&self, gi: GroundItem) {
        let _ = gi.unique_id;
        self.broadcast(encode_packet(&s::ObjectItem {
            object_id: gi.object_id,
            name: format!("#{}", gi.item_index),
            name_colour: Argb(0),
            location: gi.location,
            image: gi.item_index as u16,
            grade: 0,
        }));
        self.items.lock().await.insert(gi.object_id, gi);
    }

    pub async fn remove_ground_item(&self, object_id: u32) {
        self.items.lock().await.remove(&object_id);
        self.broadcast(encode_packet(&s::ObjectRemove { object_id }));
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
    // (image, name, level, hp, attack, defence, exp, gold)
    let defs: [(u16, &str, u16, i32, i32, i32, u32, u32); 3] = [
        (2, "稻草人", 1, 12, 1, 0, 5, 3),
        (3, "骷髅", 3, 20, 3, 1, 12, 8),
        (4, "蜘蛛", 4, 26, 5, 2, 20, 12),
    ];
    let mut oid = world.next_object_id();
    for i in 0..20 {
        let (image, name, level, hp, attack, defence, exp, gold) = defs[i % 3];
        let x = 380 + (i % 10) * 10;
        let y = 350 + (i / 10) * 10;
        world
            .add_monster(Monster {
                object_id: oid,
                name: name.to_string(),
                image,
                location: Point::new(x as i32, y as i32),
                direction: MirDirection::Up,
                level,
                max_hp: hp,
                hp,
                attack,
                defence,
                exp_reward: exp,
                gold_reward: gold,
                drops: vec![1, 2, 3, 4, 5],
                dead: false,
                dead_ticks: 0,
                target: None,
                cooldown: 0,
            })
            .await;
        oid += 1;
    }
    // 商人 NPC：卖 木剑(1)/布衣(2)/金创药(3)/回城卷(4)/铜钱袋(5)
    if world.npcs.lock().await.is_empty() {
        let npc = Npc {
            object_id: world.next_object_id(),
            name: "铁匠铺".to_string(),
            image: 0,
            location: Point::new(400, 430),
            shop_items: vec![1, 2, 3, 4, 5],
        };
        world
            .broadcast(encode_packet(&s::ObjectPlayer {
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
            }));
        world.npcs.lock().await.push(npc);
    }
}

async fn world_tick(world: &World, db: &crate::db::Database, tick: u32) {
    respawn_monsters(world).await;
    monster_ai(world).await;
    regen_players(world).await;
    // 每 25 tick（约 10 秒）周期性持久化玩家状态，降低掉线/宕机丢失进度风险
    if tick % 25 == 0 {
        persist_all_players(world, db).await;
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

/// 怪物 AI：有仇恨且相邻则攻击玩家
async fn monster_ai(world: &World) {
    let mut attacks: Vec<(u32, u32, i32)> = Vec::new();
    {
        let mons = world.monsters.lock().await;
        let players = world.players.lock().await;
        for m in mons.values() {
            if m.dead || m.cooldown > 0 {
                continue;
            }
            let Some(target_oid) = m.target else { continue };
            let Some(p) = players.get(&target_oid) else { continue };
            if manhattan(m.location, p.location) <= 1 {
                let dmg = (m.attack.max(1) - p.defence / 2).max(1);
                attacks.push((m.object_id, target_oid, dmg));
            }
        }
    }
    for (mid, pid, dmg) in attacks {
        monster_hit_player(world, mid, pid, dmg).await;
    }
}

/// 玩家生命回复
async fn regen_players(world: &World) {
    let mut to_heal: Vec<(u32, i32, i32)> = Vec::new();
    {
        let players = world.players.lock().await;
        for p in players.values() {
            if p.hp < p.max_hp {
                let nhp = (p.hp + 4).min(p.max_hp);
                let nmp = (p.mp + 2).min(p.max_mp);
                to_heal.push((p.object_id, nhp, nmp));
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
}

fn manhattan(a: Point, b: Point) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

// ---------------------------------------------------------------------------
// 战斗
// ---------------------------------------------------------------------------

/// 玩家攻击：命中正前方相邻格内的怪物并结算。返回是否命中。
pub async fn player_attack(world: &World, player_id: u32, direction: MirDirection) -> bool {
    let Some(player) = world.get_player(player_id).await else {
        return false;
    };
    world.broadcast(encode_packet(&s::ObjectAttack {
        object_id: player_id,
        location: player.location,
        direction,
        spell: 0,
        level: 0,
        r#type: 0,
    }));
    let (dx, dy) = direction_offset(direction, 1);
    let target_pos = Point::new(player.location.x + dx, player.location.y + dy);

    let target_monster = {
        let mons = world.monsters.lock().await;
        mons.values()
            .find(|m| !m.dead && m.location == target_pos)
            .map(|m| m.object_id)
    };
    let Some(monster_id) = target_monster else { return false };

    let dmg = {
        let mons = world.monsters.lock().await;
        let m = mons.get(&monster_id).unwrap();
        (player.attack + rand_range(1, 3) - m.defence.max(0)).max(1)
    };
    player_hit_monster(world, player_id, monster_id, dmg).await;
    true
}

pub async fn player_hit_monster(world: &World, player_id: u32, monster_id: u32, dmg: i32) {
    let mut died = false;
    {
        let mut mons = world.monsters.lock().await;
        if let Some(m) = mons.get_mut(&monster_id) {
            if m.dead {
                return;
            }
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
    world.broadcast(encode_packet(&s::DamageIndicator {
        damage: dmg,
        r#type: 0,
        object_id: player_id,
    }));
    let loc = {
        let mons = world.monsters.lock().await;
        mons.get(&monster_id).map(|m| m.location).unwrap_or(Point::new(0, 0))
    };
    world.broadcast(encode_packet(&s::ObjectStruck {
        object_id: monster_id,
        attacker_id: player_id,
        location: loc,
        direction: MirDirection::Up,
    }));
    if died {
        monster_died(world, player_id, monster_id).await;
    }
}

async fn monster_died(world: &World, player_id: u32, monster_id: u32) {
    let (loc, drops, exp_reward, gold_reward) = {
        let mons = world.monsters.lock().await;
        let m = mons.get(&monster_id).unwrap();
        (m.location, m.drops.clone(), m.exp_reward, m.gold_reward)
    };
    world.broadcast(encode_packet(&s::ObjectDied {
        object_id: monster_id,
        location: loc,
        direction: MirDirection::Up,
        r#type: 0,
    }));
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
            })
            .await;
    }
    if exp_reward > 0 {
        gain_experience(world, player_id, exp_reward).await;
    }
    if gold_reward > 0 {
        gain_gold(world, player_id, gold_reward).await;
    }
}

async fn monster_hit_player(world: &World, monster_id: u32, player_id: u32, dmg: i32) {
    {
        let mut mons = world.monsters.lock().await;
        if let Some(m) = mons.get_mut(&monster_id) {
            m.cooldown = 3;
        }
    }
    let (loc, mut killer, mut hp, mut mp) = {
        let players = world.players.lock().await;
        let p = players.get(&player_id);
        match p {
            Some(p) => (p.location, false, p.hp, p.mp),
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
    world.broadcast(encode_packet(&s::ObjectStruck {
        object_id: monster_id,
        attacker_id: player_id,
        location: loc,
        direction: MirDirection::Up,
    }));
    if killer {
        player_died(world, player_id).await;
    }
}

/// 玩家死亡：回城复活
pub async fn player_died(world: &World, player_id: u32) {
    world
        .send_to(
            player_id,
            encode_packet(&s::Death {
                location: SPAWN,
                direction: MirDirection::Up,
            }),
        )
        .await;
    world.broadcast(encode_packet(&s::ObjectDied {
        object_id: player_id,
        location: SPAWN,
        direction: MirDirection::Up,
        r#type: 0,
    }));
    {
        let mut players = world.players.lock().await;
        if let Some(p) = players.get_mut(&player_id) {
            p.location = SPAWN;
            p.hp = p.max_hp / 2;
            p.mp = p.max_mp;
        }
    }
    world
        .send_to(
            player_id,
            encode_packet(&s::UserLocation {
                location: SPAWN,
                direction: MirDirection::Up,
            }),
        )
        .await;
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
        match slot {
            0 => {
                // 武器槽：加攻击
                player.weapon = tmpl.index as i16;
                attack += tmpl.bonus;
            }
            1 => {
                // 护甲槽：加防御
                player.armour = tmpl.index as i16;
                defence += tmpl.bonus;
            }
            _ => {}
        }
    }
    player.attack = attack;
    player.defence = defence;
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
            .find(|gi| manhattan(gi.location, player.location) <= 1)
            .map(|gi| gi.object_id)
    };
    let Some(oid) = target else { return };
    let gi = {
        let mut items = world.items.lock().await;
        items.remove(&oid)
    };
    let Some(gi) = gi else { return };
    world.remove_ground_item(oid).await;

    let ok = db.add_item_to_inventory(player.character_index, gi.item_index, gi.count);
    if ok.is_err() {
        return;
    }
    let item = UserItem {
        unique_id: gi.unique_id,
        item_index: gi.item_index,
        count: gi.count,
        ..Default::default()
    };
    world.send_to(player_id, encode_packet(&s::GainedItem { item })).await;
    // 背包变化需刷新（简化：用 UserSlotsRefresh 提示，此处先发 GainedItem 即可）
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

/// 地图边界校验 + 返回新位置
pub fn try_move(location: Point, dir: MirDirection, steps: i32) -> Option<Point> {
    let (dx, dy) = direction_offset(dir, steps);
    let nx = location.x + dx;
    let ny = location.y + dy;
    if nx < 0 || ny < 0 || nx >= MAP_WIDTH || ny >= MAP_HEIGHT {
        return None;
    }
    Some(Point::new(nx, ny))
}
