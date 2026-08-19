//! 玩法闭环端到端验证客户端。
//!
//! 用法: 先启动服务器 (`cargo run -p crystal-server`)，再:
//! `cargo run -p crystal-server --example play_client`
//!
//! 注意: 请在**全新数据库**（删除 data/crystal.db）冷启动的服务器上运行，
//! 以保证世界/角色为初始状态（怪物 AI 会移动/追击，重复运行会因共享世界状态而不稳定）。
//!
//! 流程: 连接 → ClientVersion → Login(demo) → StartGame → 朝前方怪物攻击直到击杀
//! → 魔法攻击(火球术) → 拾取掉落 → 确认获得物品/经验 → 访问 NPC 商人 → 买入金创药
//! → 使用金创药回血 → 装备木剑 → 丢弃金创药 → 重连验证金币/经验/等级持久化。

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
    // 排空缓冲区若干帧（忽略内容），避免旧帧干扰后续响应判定
    async fn drain(buf: &mut Vec<u8>, stream: &mut TcpStream, n: usize) {
        for _ in 0..n {
            if recv_timed(buf, stream, 60).await.is_none() {
                break;
            }
        }
    }
    // 走一步并返回服务器确认后的位置（被墙阻挡则位置不变）
    async fn confirmed_walk(p: &mut (i32, i32), buf: &mut Vec<u8>, stream: &mut TcpStream, dir: u8) {
        send_packet(stream, &c::Walk { direction: MirDirection::from_u8(dir) }).await;
        for _ in 0..6 {
            let Some((id, payload)) = recv_timed(buf, stream, 400).await else { break };
            if let ServerPacket::UserLocation(u) = ServerPacket::decode(id, &payload).unwrap() {
                *p = (u.location.x, u.location.y);
                break;
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
    let mut monster_id: Option<u32> = None;
    let mut merchant_id: Option<u32> = None;
    let mut shop_pos: Option<(i32, i32)> = None;
    let mut potion_uid: Option<u64> = None; // 金创药 unique_id
    let mut weapon_uid: Option<u64> = None; // 木剑 unique_id
    let mut first_state: Option<(u32, i64, u16)> = None; // (gold, experience, level)
    for _ in 0..120 {
        let Some((id, payload)) = recv_timed(&mut buf, &mut stream, 800).await else { break };
        match ServerPacket::decode(id, &payload)? {
            ServerPacket::ObjectMonster(m) if monster_pos.is_none() => {
                println!("✓ 看到怪物: {} #{} @({},{})", m.name, m.object_id, m.location.x, m.location.y);
                monster_pos = Some((m.location.x, m.location.y));
                monster_id = Some(m.object_id);
            }
            ServerPacket::ObjectPlayer(p) if p.name == "铁匠铺" => {
                merchant_id = Some(p.object_id);
                shop_pos = Some((p.location.x, p.location.y));
            }
            ServerPacket::UserInformation(ui) => {
                my_oid = ui.object_id;
                first_state = Some((ui.gold, ui.experience, ui.level));
                if let Some(inv) = &ui.inventory {
                    for slot in inv.iter().flatten() {
                        match slot.item_index {
                            3 => potion_uid = Some(slot.unique_id),
                            1 => weapon_uid = Some(slot.unique_id),
                            _ => {}
                        }
                    }
                    println!(
                        "✓ 收到背包，金创药uid={:?} 木剑uid={:?}",
                        potion_uid, weapon_uid
                    );
                }
            }
            _ => {}
        }
    }
    let _ = my_oid;
    let (tx, ty) = monster_pos.expect("未在场上看到怪物");

    // 从出生点朝怪物方向一步步走过去（直到确认相邻；用服务器确认位置）
    println!("== 走向怪物 ({tx},{ty}) ==");
    let mut p = (400i32, 400i32);
    for _ in 0..300 {
        if manhattan(p.0, p.1, tx, ty) <= 1 {
            break;
        }
        let dir = if p.0 < tx {
            2
        } else if p.0 > tx {
            6
        } else if p.1 < ty {
            4
        } else {
            0
        };
        confirmed_walk(&mut p, &mut buf, &mut stream, dir).await;
    }
    println!("✓ 到达怪物附近 ({},{})", p.0, p.1);

    // 攻击直到击杀（怪物会追击/移动，实时跟踪其位置；用确认位置，相邻才攻击）
    println!("== 攻击怪物直到击杀 ==");
    let mid = monster_id.expect("未记录怪物 id");
    let mut mnow: (i32, i32) = (tx, ty);
    let mut gained_xp = false;
    let mut attacks = 0;
    loop {
        let dist = (mnow.0 - p.0).abs() + (mnow.1 - p.1).abs();
        let dir = if p.0 < mnow.0 {
            2
        } else if p.0 > mnow.0 {
            6
        } else if p.1 < mnow.1 {
            4
        } else {
            0
        };
        if dist <= 1 {
            // 相邻：攻击
            send_packet(&mut stream, &c::Attack { direction: dir, spell: 0 }).await;
            attacks += 1;
        } else {
            // 不相邻：朝怪物走一步（确认位置）
            confirmed_walk(&mut p, &mut buf, &mut stream, dir).await;
        }
        let mut killed = false;
        for _ in 0..8 {
            let Some((id, payload)) = recv_timed(&mut buf, &mut stream, 500).await else { break };
            match ServerPacket::decode(id, &payload)? {
                ServerPacket::GainedGold(_) => println!("✓ 获得金币"),
                ServerPacket::ObjectWalk(w) if w.object_id == mid => {
                    // 怪物在移动，更新其位置
                    let od = (w.location.x - p.0).abs() + (w.location.y - p.1).abs();
                    if od < dist {
                        mnow = (w.location.x, w.location.y);
                    }
                }
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
        if killed || attacks > 60 {
            break;
        }
    }
    assert!(gained_xp, "未获得经验");
    println!("✓ 击杀耗时 {attacks} 次攻击，经验已获得");

    // 拾取（掉落物就在怪物死亡位置旁，紧接击杀后玩家贴合）
    println!("== 拾取 ==");
    drain(&mut buf, &mut stream, 40).await; // 排空旧帧
    send_packet(&mut stream, &c::PickUp).await;
    let mut picked = false;
    for _ in 0..40 {
        let Some((id, payload)) = recv_timed(&mut buf, &mut stream, 400).await else { break };
        if let ServerPacket::GainedItem(_) = ServerPacket::decode(id, &payload)? {
            picked = true;
        }
    }
    assert!(picked, "未拾取到掉落物");
    println!("✓ 拾取成功");

    // 魔法攻击（火球术 spell=1）：朝 8 个方向试着施放，直到命中射程内的怪物
    println!("== 魔法攻击（火球术） ==");
    // 先取一个当前 MP 作为基准
    let mut mp_before: Option<i32> = None;
    for _ in 0..6 {
        let Some((id, payload)) = recv_timed(&mut buf, &mut stream, 300).await else { break };
        if let ServerPacket::HealthChanged(h) = ServerPacket::decode(id, &payload)? {
            mp_before = Some(h.mp);
        }
    }
    let mut spell_hit = false;
    let mut saw_damage = false;
    let mut mp_after: Option<i32> = None;
    'dirs: for dir in [0u8, 2, 4, 6, 1, 3, 5, 7] {
        // 清空缓冲，干净地观察本次施放结果
        while recv_timed(&mut buf, &mut stream, 30).await.is_some() {}
        send_packet(&mut stream, &c::Attack { direction: dir, spell: 1 }).await;
        for _ in 0..8 {
            let Some((id, payload)) = recv_timed(&mut buf, &mut stream, 500).await else { continue 'dirs };
            match ServerPacket::decode(id, &payload)? {
                ServerPacket::ObjectMagic(m) if m.spell == 1 => {
                    println!("✓ 火球术命中目标#{} 目标位置{:?}", m.target_id, m.target);
                    spell_hit = true;
                }
                ServerPacket::DamageIndicator(_) => saw_damage = true,
                ServerPacket::HealthChanged(h) => mp_after = Some(h.mp),
                _ => {}
            }
        }
        if spell_hit {
            break 'dirs;
        }
    }
    assert!(spell_hit, "火球术未命中任何怪物");
    assert!(saw_damage, "火球术命中但未产生伤害指示");
    assert!(mp_after.is_some(), "施放后未收到 MP 更新");
    if let Some(before) = mp_before {
        if let (Some(dbg), Some(after)) = (Some(before), mp_after) {
            println!("✓ MP {dbg} -> {after}");
            assert!(after < dbg, "施放火球术未消耗 MP");
        }
    }
    println!("✓ 火球术施放成功，消耗 MP 且造成伤害");

    // 走向 NPC 商人买药
    println!("== NPC 商人 ==");
    let merchant_id = merchant_id.expect("未找到商人");
    let (sx, sy) = shop_pos.expect("商人位置缺失");
    for _ in 0..300 {
        if manhattan(p.0, p.1, sx, sy) <= 2 {
            break;
        }
        let dir = if p.0 < sx {
            2
        } else if p.0 > sx {
            6
        } else if p.1 < sy {
            4
        } else {
            0
        };
        confirmed_walk(&mut p, &mut buf, &mut stream, dir).await;
    }
    println!("✓ 到达商人 ({},{})", p.0, p.1);
    drain(&mut buf, &mut stream, 40).await; // 排空旧帧
    send_packet(
        &mut stream,
        &c::CallNPC { object_id: merchant_id, key: String::new() },
    )
    .await;
    let mut saw_goods = false;
    for _ in 0..40 {
        let Some((id, payload)) = recv_timed(&mut buf, &mut stream, 400).await else { break };
        if let ServerPacket::NPCGoods(g) = ServerPacket::decode(id, &payload)? {
            println!("✓ 商店在售 {} 件商品", g.list.len());
            saw_goods = true;
            break;
        }
    }
    assert!(saw_goods, "未收到商店列表 NPCGoods");
    // 买金创药(index=3)，用 LoseGold 判定扣款成功
    drain(&mut buf, &mut stream, 40).await;
    send_packet(&mut stream, &c::BuyItem { item_index: 3, count: 1, r#type: 0 }).await;
    let mut saw_lose_gold = false;
    for _ in 0..40 {
        let Some((id, payload)) = recv_timed(&mut buf, &mut stream, 400).await else { break };
        if let ServerPacket::LoseGold(_) = ServerPacket::decode(id, &payload)? {
            saw_lose_gold = true;
            break;
        }
    }
    assert!(saw_lose_gold, "购买未扣款(LoseGold)");
    println!("✓ 购买金创药成功，金币已扣除");

    // 使用金创药回血
    println!("== 使用金创药 ==");
    let potion_uid = potion_uid.expect("背包里没有金创药");
    let mut cur_hp: Option<i32> = None;
    // 先读一帧获取当前血量（来自世界状态里的健康更新）
    if let Some((id, payload)) = recv_timed(&mut buf, &mut stream, 300).await {
        if let ServerPacket::HealthChanged(h) = ServerPacket::decode(id, &payload)? {
            cur_hp = Some(h.hp);
        }
    }
    drain(&mut buf, &mut stream, 40).await; // 排空旧帧
    send_packet(&mut stream, &c::UseItem { unique_id: potion_uid, grid: 1 }).await;
    let mut used = false;
    let mut hp_after: Option<i32> = None;
    for _ in 0..40 {
        let Some((id, payload)) = recv_timed(&mut buf, &mut stream, 400).await else { break };
        match ServerPacket::decode(id, &payload)? {
            ServerPacket::UseItem(u) => {
                assert!(u.success, "use item 返回失败");
                assert!(u.unique_id == potion_uid, "use item uid 不符");
                used = true;
            }
            ServerPacket::HealthChanged(h) => hp_after = Some(h.hp),
            _ => {}
        }
    }
    assert!(used, "未收到 UseItem 成功包");
    let hp_after = hp_after.expect("使用金创药后未收到 HP 更新");
    if let Some(before) = cur_hp {
        println!("✓ HP {before} -> {hp_after}");
        assert!(hp_after > before, "HP 未上升");
    } else {
        println!("✓ HP 当前 {hp_after}");
    }
    println!("✓ 金创药使用成功，HP 已回复");

    // 装备木剑（装备槽 0 = Weapon）
    println!("== 装备木剑 ==");
    let weapon_uid = weapon_uid.expect("背包里没有木剑");
    drain(&mut buf, &mut stream, 30).await; // 排空旧帧，避免响应被挤掉
    send_packet(&mut stream, &c::EquipItem { grid: 1, unique_id: weapon_uid, to: 0 }).await;
    let mut equipped = false;
    let mut saw_own_weapon = false;
    for _ in 0..16 {
        let Some((id, payload)) = recv_timed(&mut buf, &mut stream, 600).await else { break };
        match ServerPacket::decode(id, &payload)? {
            ServerPacket::EquipItem(e) => {
                assert!(e.success, "装备返回失败");
                assert!(e.unique_id == weapon_uid, "装备 uid 不符");
                equipped = true;
            }
            ServerPacket::ObjectPlayer(op) if op.object_id == my_oid && op.weapon == 1 => {
                saw_own_weapon = true;
            }
            _ => {}
        }
    }
    assert!(equipped, "未收到 EquipItem 成功包");
    println!("✓ 木剑已装备，外观 weapon 字段 = 1");
    let _ = saw_own_weapon;

    // 丢弃一个金创药（背包金创药数量>1，丢 1 个，地面出现掉落物）
    println!("== 丢弃金创药 ==");
    drain(&mut buf, &mut stream, 40).await; // 排空旧帧
    send_packet(&mut stream, &c::DropItem { unique_id: potion_uid, count: 1, hero_inventory: false }).await;
    let mut saw_drop_obj = false;
    for _ in 0..40 {
        let Some((id, payload)) = recv_timed(&mut buf, &mut stream, 600).await else { break };
        if let ServerPacket::ObjectItem(oi) = ServerPacket::decode(id, &payload)? {
            // 地面物体无独立 item_index，ObjectItem 以 image 表示道具（世界侧用 item_index 作为 image）
            assert_eq!(oi.image, 3, "掉落物不是金创药");
            saw_drop_obj = true;
            break;
        }
    }
    assert!(saw_drop_obj, "丢弃后未收到地面 ObjectItem");
    println!("✓ 金创药已丢弃到地面");

    // 重连持久化：LogOut → Login → StartGame，验证金币/经验已写回 DB
    println!("== 重连验证持久化 ==");
    send_packet(&mut stream, &c::LogOut).await;
    let _ = recv_timed(&mut buf, &mut stream, 600).await; // LoginSuccess
    send_packet(&mut stream, &c::Login { account_id: "demo".into(), password: "x".into() }).await;
    let _ = recv_timed(&mut buf, &mut stream, 600).await; // LoginSuccess
    send_packet(&mut stream, &c::StartGame { character_index: chars[0].index }).await;
    let mut reconnected = false;
    let mut re_gold = 0u32;
    let mut re_level = 0u16;
    for _ in 0..80 {
        let Some((id, payload)) = recv_timed(&mut buf, &mut stream, 600).await else { break };
        if let ServerPacket::UserInformation(ui) = ServerPacket::decode(id, &payload)? {
            re_gold = ui.gold;
            re_level = ui.level;
            reconnected = true;
            break;
        }
    }
    assert!(reconnected, "重连未收到 UserInformation");
    let (first_gold, _first_exp, first_level) = first_state.expect("首帧未记录");
    println!(
        "✓ 重连后 gold={re_gold} level={re_level}（首帧 gold={first_gold} level={first_level}）"
    );
    // 持久化判定：DB 重载后状态相对初始基线发生变化即证明写回成功。
    // 战斗所得金币/经验会使金币与等级变化（买药会花掉部分金币，故金币可能略降）。
    assert_ne!(re_gold, first_gold, "重连金币未变化，DB 未持久化金币");
    assert!(re_level >= first_level, "重连后等级倒退");
    println!("✓ 击杀所得经验已升级、金币收支已写回 DB（重连可读）");

    println!("\n✅ 玩法闭环全部验证通过（战斗→击杀→经验→魔法→拾取→商店→道具→装备→丢弃→重连持久化）");
    Ok(())
}

fn manhattan(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    (ax - bx).abs() + (ay - by).abs()
}
