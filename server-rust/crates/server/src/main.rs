//! Crystal Web3 MMORPG —— Rust 服务器（垂直切片）
//!
//! 阶段 1: 登录握手 → 角色选择 → 进世界 → 移动/聊天。
//! 启动: `cargo run -p crystal-server`（默认 127.0.0.1:7000）

mod account;
mod net;
mod world;

use std::sync::Arc;

use account::AccountStore;
use crystal_protocol::types::{MirClass, MirGender};
use tokio::net::TcpListener;
use world::World;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let addr = std::env::var("CRYSTAL_BIND").unwrap_or_else(|_| "127.0.0.1:7000".to_string());

    let accounts = AccountStore::new();
    let world = Arc::new(World::new());

    // 内置演示账号（方便快速测试）
    accounts.register("demo");
    accounts
        .add_character("demo", "战士一号", MirClass::Warrior, MirGender::Male)
        .ok();
    accounts
        .add_character("demo", "法师一号", MirClass::Wizard, MirGender::Female)
        .ok();

    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("Crystal Rust 服务器监听 {addr}");

    let mut conn_id = 0u32;
    loop {
        let (stream, peer) = listener.accept().await?;
        conn_id += 1;
        let accounts = accounts.clone();
        let world = world.clone();
        tokio::spawn(async move {
            tracing::info!("[conn {conn_id}] {peer} 连接");
            if let Err(e) = net::handle_connection(stream, accounts, (*world).clone()).await {
                tracing::debug!("[conn {conn_id}] 连接结束: {e}");
            }
            tracing::debug!("[conn {conn_id}] 断开");
        });
    }
}
