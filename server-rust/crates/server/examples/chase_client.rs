//! 怪物 AI（感知索敌 + 追击）端到端验证客户端。
//!
//! 用法: 先启动服务器 (`cargo run -p crystal-server`)，再:
//! `cargo run -p crystal-server --example chase_client`
//!
//! 流程: 连接 → Login(demo) → StartGame → 走向一只怪物（进入 5 格感知半径，
//! 怪物主动索敌）→ 贴近后撤退 → 验证怪物通过 ObjectWalk 朝玩家追击。

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crystal_protocol::client as c;
use crystal_protocol::frame::{encode_packet, PacketCodec};
use crystal_protocol::server as s;
use crystal_protocol::types::MirDirection;
use crystal_protocol::ServerPacket;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = std::env::var("CRYSTAL_SERVER").unwrap_or_else(|_| "127.0.0.1:7000".to_string());
    let mut stream = TcpStream::connect(&addr).await.expect("连接服务器失败");
    let mut buf: Vec<u8> = Vec::new();

    async fn send_packet<P: PacketCodec>(stream: &mut TcpStream, p: &P) {
        stream.write_all(&encode_packet(p)).await.unwrap();
    }
    async fn recv_timed(buf: &mut Vec<u8>, stream: &mut TcpStream, ms: u64) -> Option<(i16, Vec<u8>)> {
        loop {
            if buf.len() >= 4 {
                let len = u16::from_le_bytes([buf[0], buf[1]]) as usize;
                if len >= 4 && buf.len() >= len {
                    let id = i16::from_le_bytes([buf[2], buf[3]]);
                    let payload = buf[4..len].to_vec();
                    buf.drain(..len);
                    return Some((id, payload));
                }
            }
            let mut chunk = [0u8; 8192];
            match tokio::time::timeout(std::time::Duration::from_millis(ms), stream.read(&mut chunk)).await {
                Ok(Ok(n)) if n > 0 => buf.extend_from_slice(&chunk[..n]),
                _ => return None,
            }
        }
    }
    async fn recv(buf: &mut Vec<u8>, stream: &mut TcpStream) -> (i16, Vec<u8>) {
        loop {
            if let Some(p) = recv_timed(buf, stream, 5000).await {
                return p;
            }
        }
    }

    // 连接 + 版本 + 登录
    let (id, payload) = recv(&mut buf, &mut stream).await;
    assert!(matches!(ServerPacket::decode(id, &payload)?, ServerPacket::Connected(_)));
    send_packet(&mut stream, &c::ClientVersion { version_hash: vec![] }).await;
    let (id, payload) = recv(&mut buf, &mut stream).await;
    assert!(matches!(ServerPacket::decode(id, &payload)?, ServerPacket::ClientVersion(_)));
    send_packet(&mut stream, &c::Login { account_id: "demo".into(), password: "x".into() }).await;
    let (id, payload) = recv(&mut buf, &mut stream).await;
    let chars = match ServerPacket::decode(id, &payload)? {
        ServerPacket::LoginSuccess(ls) => ls.characters,
        other => panic!("登录失败 {other:?}"),
    };
    println!("✓ 登录，角色数={}", chars.len());

    // 进世界并找第一只怪物（记录 id + 位置）
    send_packet(&mut stream, &c::StartGame { character_index: chars[0].index }).await;
    let mut monster: Option<(u32, (i32, i32))> = None;
    for _ in 0..120 {
        let Some((id, payload)) = recv_timed(&mut buf, &mut stream, 800).await else { break };
        if let ServerPacket::ObjectMonster(m) = ServerPacket::decode(id, &payload)? {
            if monster.is_none() {
                println!("✓ 首家怪物: {} #{} @({},{})", m.name, m.object_id, m.location.x, m.location.y);
                monster = Some((m.object_id, (m.location.x, m.location.y)));
            }
        }
    }
    let (mid, (tx, ty)) = monster.expect("场上没有怪物");

    // 走向怪物直到相邻（玩家出生 400,400）
    let mut px = 400i32;
    let mut py = 400i32;
    for _ in 0..200 {
        if manhattan(px, py, tx, ty) <= 1 {
            break;
        }
        let dir = dir_toward(px, py, tx, ty);
        send_packet(&mut stream, &c::Walk { direction: MirDirection::from_u8(dir) }).await;
        match dir {
            2 => px += 1,
            6 => px -= 1,
            4 => py += 1,
            _ => py -= 1,
        }
        let _ = recv_timed(&mut buf, &mut stream, 300).await;
    }
    println!("✓ 到达怪物身旁 ({px},{py})");

    // 撤退 5 格（离开攻击/贴近范围），观察怪物追击
    println!("== 撤退并观察怪物追击 ==");
    let (ox, oy) = (px - tx, py - ty); // 背离方向
    let retreat_dir = if ox.abs() >= oy.abs() {
        if ox > 0 { 6 } else { 2 }
    } else {
        if oy > 0 { 0 } else { 4 }
    };
    // 先转身让怪物能看见/继续索敌，再撤退
    for _ in 0..5 {
        send_packet(&mut stream, &c::Walk { direction: MirDirection::from_u8(retreat_dir) }).await;
        match retreat_dir {
            2 => px += 1,
            6 => px -= 1,
            4 => py += 1,
            _ => py -= 1,
        }
        let _ = recv_timed(&mut buf, &mut stream, 200).await;
    }

    // 观察 3 秒：怪物应通过 ObjectWalk 朝玩家追击
    let mut chase_seen = false;
    let mut monster_pos: Option<(i32, i32)> = Some((tx, ty));
    for _ in 0..12 {
        let Some((id, payload)) = recv_timed(&mut buf, &mut stream, 250).await else { break };
        if let ServerPacket::ObjectWalk(w) = ServerPacket::decode(id, &payload)? {
            if w.object_id == mid {
                chase_seen = true;
                monster_pos = Some((w.location.x, w.location.y));
                println!("✓ 怪物追来 #{} @({},{})", mid, w.location.x, w.location.y);
            }
        }
    }
    assert!(chase_seen, "怪物未通过 ObjectWalk 追击玩家");
    if let Some((mx, my)) = monster_pos {
        let before = manhattan(px, py, tx, ty);
        let after = manhattan(px, py, mx, my);
        println!("✓ 撤退后玩家与怪物距离 {before} -> {after}");
        assert!(after < before, "怪物未靠近玩家（追击失效）");
    }
    println!("\n✅ 怪物 AI 验证通过（感知索敌 + 追击）");
    Ok(())
}

fn manhattan(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    (ax - bx).abs() + (ay - by).abs()
}

fn dir_toward(ax: i32, ay: i32, bx: i32, by: i32) -> u8 {
    if ax < bx {
        2
    } else if ax > bx {
        6
    } else if ay < by {
        4
    } else {
        0
    }
}
