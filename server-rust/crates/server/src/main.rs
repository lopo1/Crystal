//! Crystal Web3 MMORPG —— Rust 服务器（垂直切片 / 阶段2）
//!
//! 阶段 1: 登录握手 → 角色选择 → 进世界 → 移动/聊天。
//! 阶段 2: SQLite 持久化（账户/角色/物品背包）。
//! 启动: `cargo run -p crystal-server`（默认 127.0.0.1:7000）

mod db;
mod group;
mod guild;
mod items;
mod magics;
mod maps;
mod net;
mod trade;
mod web3;
mod world;

use std::sync::Arc;

use crystal_protocol::types::{MirClass, MirGender};
use db::Database;
use tokio::net::TcpListener;
use web3::Web3Auth;
use world::World;

/// 创建演示账号角色；`add_character` 返回 `Result<Result<SelectInfo,u8>, _>`，
/// 内层 u8 是业务错误码，这里忽略（首启动通常成功）。
fn add_demo_chars(db: &db::Database) -> anyhow::Result<()> {
    let _ = db.add_character("demo", "战士一号", MirClass::Warrior, MirGender::Male)?;
    let _ = db.add_character("demo", "法师一号", MirClass::Wizard, MirGender::Female)?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 日志级别：默认 info，可用 RUST_LOG=debug/trace 开启更细日志
    let level = match std::env::var("RUST_LOG").unwrap_or_default().to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };
    tracing_subscriber::fmt().with_max_level(level).init();

    let addr = std::env::var("CRYSTAL_BIND").unwrap_or_else(|_| "127.0.0.1:7000".to_string());
    let db_path = std::env::var("CRYSTAL_DB").unwrap_or_else(|_| "data/crystal.db".to_string());
    // 地图数据目录（存放 Crystal.Database 的 .map 文件）
    let map_dir = std::env::var("CRYSTAL_MAPS").unwrap_or_else(|_| "data/maps".to_string());

    let db = Database::open(&db_path)?;
    let map_dir_p = |n: &str| std::path::Path::new(&map_dir).join(format!("{n}.map"));
    // 加载默认地图（0 = 新手村；缺图时退回程序化空地图以便无头运行）
    let map = maps::load_map_file(0, map_dir_p("0")).unwrap_or_else(|e| {
        tracing::warn!("未加载到 0.map（{e}），使用默认空地图");
        world::default_map()
    });
    let world = Arc::new(World::with_map(map));
    // 注册更多真实地图（供 /map 传送）；缺失则跳过
    for idx in ["0100", "0101"] {
        let mi = u32::from_str_radix(idx, 10).unwrap();
        match maps::load_map_file(mi, map_dir_p(idx)) {
            Ok(m) => world.register_map(m),
            Err(e) => tracing::warn!("注册地图 {idx}({mi}) 失败: {e}"),
        }
    }
    let web3_auth = Arc::new(Web3Auth::new());

    // 启动世界 tick（怪物刷新 + AI + 玩家回复 + 周期性存档）
    let _tick_task = world::spawn_world_tick(world.clone(), db.clone());

    // 演示账号（幂等；已存在则跳过）
    if !db.login("demo")? {
        db.register("demo")?;
        add_demo_chars(&db)?;
        tracing::info!("已创建演示账号 demo");
    }

    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("Crystal Rust 服务器监听 {addr} (DB: {db_path})");

    let mut conn_id = 0u32;
    loop {
        let (stream, peer) = listener.accept().await?;
        conn_id += 1;
        let db = db.clone();
        let world = world.clone();
        let web3_auth = web3_auth.clone();
        tokio::spawn(async move {
            tracing::info!("[conn {conn_id}] {peer} 连接");
            if let Err(e) = net::handle_connection(stream, db, (*world).clone(), web3_auth).await {
                tracing::debug!("[conn {conn_id}] 连接结束: {e}");
            }
            tracing::debug!("[conn {conn_id}] 断开");
        });
    }
}
