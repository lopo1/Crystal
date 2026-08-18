//! 游戏世界（垂直切片版）: 单地图、玩家对象、广播。
//!
//! 后续阶段将按系统扩展: 怪物/Mob线程、物品、战斗、NPC 等（对照 C# `MirEnvir`）。

use std::collections::HashMap;
use std::sync::Arc;

use crystal_protocol::binary::Point;
use crystal_protocol::types::{MirClass, MirDirection, MirGender};
use tokio::sync::{broadcast, Mutex};

/// 默认地图（Crystal 的 0 号图即新手村）
pub const MAP_WIDTH: i32 = 800;
pub const MAP_HEIGHT: i32 = 800;
pub const SPAWN: Point = Point { x: 400, y: 400 };

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
    pub hp: i32,
    pub mp: i32,
}

#[derive(Debug, Clone)]
pub struct World {
    pub players: Arc<Mutex<HashMap<u32, Player>>>,
    /// 可观测事件广播（ObjectWalk/Turn/Run/Chat 等帧字节）
    pub broadcast_tx: broadcast::Sender<Vec<u8>>,
    next_object_id: Arc<std::sync::atomic::AtomicU32>,
}

impl World {
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(256);
        World {
            players: Arc::new(Mutex::new(HashMap::new())),
            broadcast_tx,
            next_object_id: Arc::new(std::sync::atomic::AtomicU32::new(1)),
        }
    }

    pub fn next_object_id(&self) -> u32 {
        self.next_object_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Vec<u8>> {
        self.broadcast_tx.subscribe()
    }

    pub async fn add_player(&self, player: Player) {
        self.players.lock().await.insert(player.object_id, player);
    }

    pub async fn remove_player(&self, object_id: u32) {
        self.players.lock().await.remove(&object_id);
    }

    pub async fn get_player(&self, object_id: u32) -> Option<Player> {
        self.players.lock().await.get(&object_id).cloned()
    }

    /// 派发对除 `except` 外所有在线玩家的可观测包
    pub fn broadcast_except(&self, frame: Vec<u8>, except: u32) {
        let _ = self.broadcast_tx.send(frame);
        // 注意: 垂直切片简化——广播给所有订阅者，由连接层过滤自己（except）。
        // 后续用玩家视角(带视野)替换全图广播。
        let _ = except;
    }
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
