//! 演示客户端（垂直切片端到端验证）。
//!
//! 用法: 先启动服务器 (`cargo run -p crystal-server`)，再:
//! `cargo run -p crystal-server --example demo_client`
//!
//! 流程: 连接 → ClientVersion → Login(demo) → 角色列表 → StartGame →
//! 收地图/自身信息 → Walk → Chat → LogOut

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crystal_protocol::client as c;
use crystal_protocol::frame::{encode_packet, PacketCodec};
use crystal_protocol::server as s;
use crystal_protocol::types::MirDirection;
use crystal_protocol::ServerPacket;

fn fail(msg: &str) -> ! {
    eprintln!("✗ {msg}");
    std::process::exit(1);
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = std::env::var("CRYSTAL_SERVER").unwrap_or_else(|_| "127.0.0.1:7000".to_string());
    let mut stream = TcpStream::connect(&addr).await.expect("连接服务器失败");

    let mut buf: Vec<u8> = Vec::new();

    async fn send_packet<P: PacketCodec>(stream: &mut TcpStream, p: &P) {
        stream.write_all(&encode_packet(p)).await.unwrap();
    }
    async fn recv_packet(buf: &mut Vec<u8>, stream: &mut TcpStream) -> (i16, Vec<u8>) {
        loop {
            // 尝试解析
            if buf.len() >= 4 {
                let len = u16::from_le_bytes([buf[0], buf[1]]) as usize;
                if len >= 4 && buf.len() >= len {
                    let id = i16::from_le_bytes([buf[2], buf[3]]);
                    let payload = buf[4..len].to_vec();
                    buf.drain(..len);
                    return (id, payload);
                }
            }
            let mut chunk = [0u8; 8192];
            let n = stream.read(&mut chunk).await.unwrap();
            if n == 0 {
                fail("连接被服务器关闭");
            }
            buf.extend_from_slice(&chunk[..n]);
        }
    }

    println!("== 连接 {addr} ==");
    let (id, payload) = recv_packet(&mut buf, &mut stream).await;
    match ServerPacket::decode(id, &payload)? {
        ServerPacket::Connected(_) => println!("✓ 收到 Connected"),
        other => fail(&format!("期望 Connected，收到 {other:?}")),
    }

    println!("== ClientVersion ==");
    send_packet(
        &mut stream,
        &c::ClientVersion {
            version_hash: vec![],
        },
    )
    .await;
    let (id, payload) = recv_packet(&mut buf, &mut stream).await;
    match ServerPacket::decode(id, &payload)? {
        ServerPacket::ClientVersion(v) => {
            println!("✓ 版本结果 = {}", v.result);
            assert_eq!(v.result, 1);
        }
        other => fail(&format!("期望 ClientVersion，收到 {other:?}")),
    }

    println!("== Login(demo) ==");
    send_packet(
        &mut stream,
        &c::Login {
            account_id: "demo".into(),
            password: "x".into(),
        },
    )
    .await;
    let (id, payload) = recv_packet(&mut buf, &mut stream).await;
    let characters = match ServerPacket::decode(id, &payload)? {
        ServerPacket::LoginSuccess(ls) => {
            println!("✓ 登录成功，角色数 = {}", ls.characters.len());
            ls.characters
        }
        ServerPacket::Login(l) => fail(&format!("登录失败 result={}", l.result)),
        other => fail(&format!("期望 LoginSuccess，收到 {other:?}")),
    };

    println!("== StartGame(第一个角色) ==");
    let char_index = characters.first().map(|c| c.index).unwrap_or(1);
    send_packet(
        &mut stream,
        &c::StartGame {
            character_index: char_index,
        },
    )
    .await;
    // StartGame 后服务器连发 6 个包: StartGame, MapInformation, NewMapInfo,
    // UserInformation, UserLocation, TimeOfDay
    let mut got_start = false;
    let mut got_map = false;
    let mut got_user = false;
    let mut got_loc = false;
    for _ in 0..6 {
        let (id, payload) = recv_packet(&mut buf, &mut stream).await;
        match ServerPacket::decode(id, &payload)? {
            ServerPacket::StartGame(sg) => {
                assert_eq!(sg.result, 0);
                got_start = true;
                println!("✓ StartGame 成功 resolution={}", sg.resolution);
            }
            ServerPacket::MapInformation(m) => {
                got_map = true;
                println!("✓ 地图: {} ({})", m.title, m.file_name);
            }
            ServerPacket::NewMapInfo(nm) => {
                println!("✓ NewMapInfo: {} x {}", nm.info.width, nm.info.height);
            }
            ServerPacket::UserInformation(ui) => {
                got_user = true;
                println!("✓ 玩家: {} Lv.{} 金币 {}", ui.name, ui.level, ui.gold);
            }
            ServerPacket::UserLocation(ul) => {
                got_loc = true;
                println!("✓ 出生点: ({}, {})", ul.location.x, ul.location.y);
            }
            ServerPacket::TimeOfDay(t) => println!("✓ 时辰 lights={}", t.lights),
            other => println!("  (其他包) {other:?}"),
        }
    }
    assert!(
        got_start && got_map && got_user && got_loc,
        "进世界包不完整"
    );

    println!("== Walk(向下) ==");
    send_packet(
        &mut stream,
        &c::Walk {
            direction: MirDirection::Down,
        },
    )
    .await;
    // 广播会回显自己的 ObjectPlayer / ObjectWalk，跳过无关包直到 UserLocation
    let mut walked = false;
    for _ in 0..10 {
        let (id, payload) = recv_packet(&mut buf, &mut stream).await;
        match ServerPacket::decode(id, &payload)? {
            ServerPacket::UserLocation(ul) => {
                println!("✓ 移动到位: ({}, {})", ul.location.x, ul.location.y);
                walked = true;
                break;
            }
            _ => continue,
        }
    }
    if !walked {
        fail("未收到 UserLocation");
    }

    println!("== Chat ==");
    send_packet(
        &mut stream,
        &c::Chat {
            message: "大家好!".into(),
            linked_items: vec![],
        },
    )
    .await;
    let mut chatted = false;
    for _ in 0..10 {
        let (id, payload) = recv_packet(&mut buf, &mut stream).await;
        match ServerPacket::decode(id, &payload)? {
            ServerPacket::ObjectChat(oc) => {
                println!("✓ 聊天回显: [{}] {}", oc.object_id, oc.text);
                chatted = true;
                break;
            }
            _ => continue,
        }
    }
    if !chatted {
        fail("未收到 ObjectChat");
    }

    println!("== LogOut ==");
    send_packet(&mut stream, &c::LogOut).await;
    let mut logged_out = false;
    for _ in 0..10 {
        let (id, payload) = recv_packet(&mut buf, &mut stream).await;
        match ServerPacket::decode(id, &payload)? {
            ServerPacket::LoginSuccess(_) => {
                println!("✓ 返回角色选择");
                logged_out = true;
                break;
            }
            _ => continue,
        }
    }
    if !logged_out {
        fail("未回到角色选择");
    }

    println!("\n✅ 垂直切片端到端全部通过");
    Ok(())
}
