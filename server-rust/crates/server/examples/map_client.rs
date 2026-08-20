//! 多地图 / 传送端到端验证客户端。
//!
//! 用法: 先启动服务器（已注册 0/0100/0101 地图），再:
//! `cargo run -p crystal-server --example map_client`
//!
//! 流程: 连接 → Login(demo) → StartGame(地图0) → /map 100 传送到 0100 →
//! 校验收到 MapInformation(地图100) → 在 0100 走一步（碰撞生效）→ /map 0 回新手村。
//!
//! 注：传送门的"走上传送门格触发"逻辑由服务器端单元测试覆盖
//! （world.rs teleport_player_switches_map / walk_onto_portal_detects_dest），
//! 这里验证多地图传送（/map 命令走同一套 teleport_player 机制）。

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
    // 发送聊天命令并等待
    async fn command(stream: &mut TcpStream, msg: &str) {
        send_packet(stream, &c::Chat { message: msg.to_string(), linked_items: vec![] }).await;
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    }
    // 读取接下来若干帧中的首个 MapInformation，返回 (map_index, 宽, 高)
    async fn expect_map(buf: &mut Vec<u8>, stream: &mut TcpStream, n: usize) -> Option<(i32, i32, i32)> {
        for _ in 0..n {
            let Some((id, payload)) = recv_timed(buf, stream, 400).await else { break };
            match ServerPacket::decode(id, &payload).expect("decode") {
                ServerPacket::MapInformation(m) => {
                    for _ in 0..4 {
                        let Some((id2, p2)) = recv_timed(buf, stream, 300).await else { break };
                        if let ServerPacket::NewMapInfo(nm) = ServerPacket::decode(id2, &p2).expect("decode") {
                            return Some((m.map_index, nm.info.width, nm.info.height));
                        }
                    }
                    return Some((m.map_index, 0, 0));
                }
                _ => {}
            }
        }
        None
    }

    // 连接 + 版本 + 登录
    let (id, payload) = recv(&mut buf, &mut stream).await;
    assert!(matches!(ServerPacket::decode(id, &payload)?, ServerPacket::Connected(_)));
    send_packet(&mut stream, &c::ClientVersion { version_hash: vec![] }).await;
    let _ = recv(&mut buf, &mut stream).await;
    send_packet(&mut stream, &c::Login { account_id: "demo".into(), password: "x".into() }).await;
    let (id, payload) = recv(&mut buf, &mut stream).await;
    let chars = match ServerPacket::decode(id, &payload)? {
        ServerPacket::LoginSuccess(ls) => ls.characters,
        other => panic!("登录失败 {other:?}"),
    };
    send_packet(&mut stream, &c::StartGame { character_index: chars[0].index }).await;
    // 等首次进地图（地图 0）
    for _ in 0..40 {
        let Some((id, payload)) = recv_timed(&mut buf, &mut stream, 400).await else { break };
        if let ServerPacket::MapInformation(m) = ServerPacket::decode(id, &payload)? {
            println!("✓ 初始地图 #{}", m.map_index);
            assert_eq!(m.map_index, 0, "初始应为地图 0");
            break;
        }
    }

    // 传送到 0100（index=100）：/map 走同一套 teleport_player 机制
    println!("== /map 100 传送到 0100 ==");
    command(&mut stream, "/map 100").await;
    let m = expect_map(&mut buf, &mut stream, 40).await.expect("未收到地图变更 MapInformation");
    assert_eq!(m.0, 100, "应传送到地图 100，实际 {}", m.0);
    println!("✓ 已切换到地图 100 ({}x{})", m.1, m.2);

    // 在 0100 走一步（碰撞生效，走不了也不报错）
    send_packet(&mut stream, &c::Walk { direction: MirDirection::Down }).await;
    let _ = recv_timed(&mut buf, &mut stream, 300).await;

    // 回新手村 0
    println!("== /map 0 回新手村 ==");
    command(&mut stream, "/map 0").await;
    let m = expect_map(&mut buf, &mut stream, 40).await.expect("未收到回地图 0 的 MapInformation");
    assert_eq!(m.0, 0, "应回到地图 0");
    println!("✓ 已回到地图 0 ({}x{})", m.1, m.2);

    println!("\n✅ 多地图传送验证通过（地图0 -> 0100 -> 地图0）");
    Ok(())
}
