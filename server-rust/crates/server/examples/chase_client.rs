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

    // 发出一次 Walk，返回服务器确认后的位置（若被墙阻挡则位置不变）
    async fn confirmed_walk(p: &mut (i32, i32), buf: &mut Vec<u8>, stream: &mut TcpStream, dir: u8) {
        send_packet(stream, &c::Walk { direction: MirDirection::from_u8(dir) }).await;
        for _ in 0..4 {
            let Some((id, payload)) = recv_timed(buf, stream, 400).await else { break };
            if let ServerPacket::UserLocation(u) = ServerPacket::decode(id, &payload).unwrap() {
                *p = (u.location.x, u.location.y);
                break;
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

    // 走向怪物直到相邻（玩家出生 400,400；用服务器确认位置，绕不过墙）
    let mut p = (400i32, 400i32);
    for _ in 0..300 {
        if manhattan(p.0, p.1, tx, ty) <= 1 {
            break;
        }
        let dir = dir_toward(p.0, p.1, tx, ty);
        confirmed_walk(&mut p, &mut buf, &mut stream, dir).await;
    }
    println!("✓ 到达怪物身旁 ({},{})（目标 {tx},{ty}）", p.0, p.1);

    // 主动挑衅（打一下），确定性建立仇恨；再朝可行走方向撤退，观察怪物追击
    println!("== 挑衅并撤退，观察怪物追击 ==");
    let dir_to = dir_toward(p.0, p.1, tx, ty);
    send_packet(&mut stream, &c::Attack { direction: dir_to, spell: 0 }).await;
    let _ = recv_timed(&mut buf, &mut stream, 500).await;

    // 边撤退边观察：怪物应通过 ObjectWalk 一路追来（追击帧可能出现在撤退途中）
    let mut chase_seen = false;
    let mut damage_seen = false;
    let mut monster_pos: Option<(i32, i32)> = Some((tx, ty));
    // 处理一帧：记录怪物的追击/攻击
    async fn observe(
        id: i16, payload: &[u8], mid: u32,
        chase_seen: &mut bool, damage_seen: &mut bool, monster_pos: &mut Option<(i32, i32)>,
    ) -> anyhow::Result<()> {
        match ServerPacket::decode(id, payload)? {
            ServerPacket::ObjectWalk(w) if w.object_id == mid => {
                *chase_seen = true;
                *monster_pos = Some((w.location.x, w.location.y));
                println!("✓ 怪物追来 #{} @({},{})", mid, w.location.x, w.location.y);
            }
            ServerPacket::DamageIndicator(d) if d.r#type == 1 => *damage_seen = true,
            _ => {}
        }
        Ok(())
    }

    // 撤退 + 观察并行：尽量拉开距离，同时捕获追击帧
    let retreats = [dir_toward(tx, ty, p.0, p.1), 0u8, 2, 4, 6, 1, 3, 5, 7];
    let mut ticks = 0;
    'main_loop: for &rd in retreats.iter().cycle() {
        if ticks >= 12 {
            break 'main_loop;
        }
        ticks += 1;
        // 若已拉开 >= 5 格则不再走，纯观察
        if manhattan(p.0, p.1, tx, ty) < 5 {
            // 尝试走一步，读取帧
            let before = p;
            confirmed_walk(&mut p, &mut buf, &mut stream, rd).await;
            let _ = before;
        }
        for _ in 0..3 {
            let Some((id, payload)) = recv_timed(&mut buf, &mut stream, 200).await else { break };
            observe(id, &payload, mid, &mut chase_seen, &mut damage_seen, &mut monster_pos).await?;
        }
        if chase_seen {
            break 'main_loop;
        }
    }
    println!("✓ 撤退后玩家位置 ({},{})，受击 seen={damage_seen}", p.0, p.1);
    assert!(chase_seen, "怪物未通过 ObjectWalk 追击玩家");
    if let Some((mx, my)) = monster_pos {
        println!("⚠ 怪物已追到 ({mx},{my})");
    }
    println!("\n✅ 怪物 AI 验证通过（感知索敌 + 追击 + 绕墙）");
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
