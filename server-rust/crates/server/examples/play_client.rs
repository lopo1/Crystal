//! 玩法闭环端到端验证客户端。
//!
//! 用法: 先启动服务器 (`cargo run -p crystal-server`)，再:
//! `cargo run -p crystal-server --example play_client`
//!
//! 流程: 连接 → ClientVersion → Login(demo) → StartGame → 朝前方怪物攻击直到击杀
//! → 拾取掉落 → 确认获得物品/经验 → 访问 NPC 商人 → 买入金创药。

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
    // 阻塞读一个包
    async fn recv(buf: &mut Vec<u8>, stream: &mut TcpStream) -> (i16, Vec<u8>) {
        loop {
            if let Some(p) = recv_timed(buf, stream, 5000).await {
                return p;
            }
        }
    }

    // 连接 + 版本
    let (id, payload) = recv(&mut buf, &mut stream).await;
    assert!(matches!(ServerPacket::decode(id, &payload)?, ServerPacket::Connected(_)));
    println!("✓ 连接");
    send_packet(&mut stream, &c::ClientVersion { version_hash: vec![] }).await;
    let (id, payload) = recv(&mut buf, &mut stream).await;
    match ServerPacket::decode(id, &payload)? {
        ServerPacket::ClientVersion(v) => assert_eq!(v.result, 1),
        _ => fail("版本失败"),
    }
    println!("✓ 版本");

    // 登录
    send_packet(&mut stream, &c::Login { account_id: "demo".into(), password: "x".into() }).await;
    let (id, payload) = recv(&mut buf, &mut stream).await;
    let chars = match ServerPacket::decode(id, &payload)? {
        ServerPacket::LoginSuccess(ls) => ls.characters,
        other => fail(&format!("登录失败 {other:?}")),
    };
    println!("✓ 登录，角色数={}", chars.len());

    // 进入世界并排空
    send_packet(&mut stream, &c::StartGame { character_index: chars[0].index }).await;
    let mut my_oid = 0u32;
    let mut monster_pos: Option<(i32, i32)> = None;
    let mut merchant_id: Option<u32> = None;
    let mut shop_pos: Option<(i32, i32)> = None;
    for _ in 0..120 {
        let Some((id, payload)) = recv_timed(&mut buf, &mut stream, 800).await else { break };
        match ServerPacket::decode(id, &payload)? {
            ServerPacket::ObjectMonster(m) if monster_pos.is_none() => {
                println!("✓ 看到怪物: {} @({},{})", m.name, m.location.x, m.location.y);
                monster_pos = Some((m.location.x, m.location.y));
            }
            ServerPacket::ObjectPlayer(p) if p.name == "铁匠铺" => {
                merchant_id = Some(p.object_id);
                shop_pos = Some((p.location.x, p.location.y));
            }
            ServerPacket::UserInformation(ui) => my_oid = ui.object_id,
            _ => {}
        }
    }
    let _ = my_oid;
    let (tx, ty) = monster_pos.expect("未在场上看到怪物");

    // 从出生点朝怪物方向一步步走过去（直到相邻）
    println!("== 走向怪物 ({tx},{ty}) ==");
    let mut px = 400;
    let mut py = 400;
    for _ in 0..200 {
        if manhattan(px, py, tx, ty) <= 1 {
            break;
        }
        let dir = if px < tx {
            2 // Right
        } else if px > tx {
            6 // Left
        } else if py < ty {
            4 // Down
        } else {
            0 // Up
        };
        send_packet(&mut stream, &c::Walk { direction: MirDirection::from_u8(dir) }).await;
        match dir {
            2 => px += 1,
            6 => px -= 1,
            4 => py += 1,
            _ => py -= 1,
        }
        // 读移动确认
        let _ = recv_timed(&mut buf, &mut stream, 500).await;
    }
    println!("✓ 到达怪物附近 ({px},{py})");

    // 攻击直到击杀
    println!("== 攻击怪物直到击杀 ==");
    let mut gained_xp = false;
    let mut attacks = 0;
    loop {
        let dir = if px < tx {
            2
        } else if px > tx {
            6
        } else if py < ty {
            4
        } else {
            0
        };
        send_packet(&mut stream, &c::Attack { direction: dir, spell: 0 }).await;
        attacks += 1;
        let mut killed = false;
        for _ in 0..8 {
            let Some((id, payload)) = recv_timed(&mut buf, &mut stream, 600).await else { break };
            match ServerPacket::decode(id, &payload)? {
                ServerPacket::GainedGold(_) => println!("✓ 获得金币"),
                ServerPacket::ObjectDied(_) => {
                    println!("✓ 怪物死亡!");
                    killed = true;
                }
                ServerPacket::GainExperience(g) => {
                    println!("✓ 获得经验 {g:?}");
                    gained_xp = true;
                }
                ServerPacket::DamageIndicator(d) => println!("  伤害 {d:?}"),
                _ => {}
            }
        }
        if killed || attacks > 40 {
            break;
        }
    }
    assert!(gained_xp, "未获得经验");
    println!("✓ 击杀耗时 {attacks} 次攻击，经验已获得");

    // 拾取（掉落物就在怪物死亡位置旁）
    println!("== 拾取 ==");
    send_packet(&mut stream, &c::PickUp).await;
    let mut picked = false;
    for _ in 0..8 {
        let Some((id, payload)) = recv_timed(&mut buf, &mut stream, 600).await else { break };
        if let ServerPacket::GainedItem(_) = ServerPacket::decode(id, &payload)? {
            picked = true;
        }
    }
    assert!(picked, "未拾取到掉落物");
    println!("✓ 拾取成功");

    // 走向 NPC 商人买药
    println!("== NPC 商人 ==");
    let merchant_id = merchant_id.expect("未找到商人");
    let (sx, sy) = shop_pos.expect("商人位置缺失");
    for _ in 0..200 {
        if manhattan(px, py, sx, sy) <= 2 {
            break;
        }
        let dir = if px < sx {
            2
        } else if px > sx {
            6
        } else if py < sy {
            4
        } else {
            0
        };
        send_packet(&mut stream, &c::Walk { direction: MirDirection::from_u8(dir) }).await;
        match dir {
            2 => px += 1,
            6 => px -= 1,
            4 => py += 1,
            _ => py -= 1,
        }
        let _ = recv_timed(&mut buf, &mut stream, 300).await;
    }
    println!("✓ 到达商人 ({px},{py})");
    send_packet(
        &mut stream,
        &c::CallNPC { object_id: merchant_id, key: String::new() },
    )
    .await;
    let mut saw_goods = false;
    for _ in 0..8 {
        let Some((id, payload)) = recv_timed(&mut buf, &mut stream, 600).await else { break };
        if let ServerPacket::NPCGoods(g) = ServerPacket::decode(id, &payload)? {
            println!("✓ 商店在售 {} 件商品", g.list.len());
            saw_goods = true;
            break;
        }
    }
    assert!(saw_goods, "未收到商店列表 NPCGoods");
    // 买金创药(index=3)，用 LoseGold 判定扣款成功
    send_packet(&mut stream, &c::BuyItem { item_index: 3, count: 1, r#type: 0 }).await;
    let mut saw_lose_gold = false;
    for _ in 0..8 {
        let Some((id, payload)) = recv_timed(&mut buf, &mut stream, 600).await else { break };
        if let ServerPacket::LoseGold(_) = ServerPacket::decode(id, &payload)? {
            saw_lose_gold = true;
            break;
        }
    }
    assert!(saw_lose_gold, "购买未扣款(LoseGold)");
    println!("✓ 购买金创药成功，金币已扣除");

    println!("\n✅ 玩法闭环全部验证通过（战斗→击杀→经验→拾取→商店购买）");
    Ok(())
}

fn manhattan(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    (ax - bx).abs() + (ay - by).abs()
}
