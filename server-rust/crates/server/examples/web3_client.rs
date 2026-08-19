//! Web3 钱包登录端到端示例客户端。
//!
//! 用法: 先启动服务器 (`cargo run -p crystal-server`)，再:
//! `cargo run -p crystal-server --example web3_client`
//!
//! 流程: 连接 → ClientVersion → Web3ChallengeRequest(地址) → Web3Challenge(原文)
//! → 本地生成密钥对并用私钥对挑战 `personal_sign`(EIP-191) → Web3Login
//! → Web3LoginResult(0 成功，含角色列表)。
//!
//! 这里在客户端本地生成一个临时密钥对来模拟钱包（生产中由 Godot 嵌入的
//! MetaMask / WalletConnect 完成签名）。EIP-191 原文与 secp256k1 恢复逻辑与服务器一致，
//! 见 `crates/server/src/web3.rs`。

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use k256::ecdsa::{RecoveryId, Signature, SigningKey, VerifyingKey};
use k256::elliptic_curve::sec1::ToEncodedPoint;
use sha3::{Digest, Keccak256};

use crystal_protocol::client as c;
use crystal_protocol::frame::{encode_packet, PacketCodec};
use crystal_protocol::ServerPacket;

fn fail(msg: &str) -> ! {
    eprintln!("✗ {msg}");
    std::process::exit(1);
}

/// EIP-191 personal_sign 原文构造
fn eip191_message(message: &str) -> Vec<u8> {
    let len = message.len();
    let mut s = b"\x19Ethereum Signed Message:\n".to_vec();
    s.extend_from_slice(len.to_string().as_bytes());
    s.push(b' ');
    s.extend_from_slice(message.as_bytes());
    s
}

/// 用私钥对消息做 personal_sign，返回 65 字节 r||s||v（v=27/28），模拟钱包行为。
fn personal_sign(key: &SigningKey, message: &str) -> Vec<u8> {
    let digest = Keccak256::new_with_prefix(eip191_message(message));
    let (sig, recid) = key.sign_digest_recoverable(digest).unwrap();
    let mut out = sig.to_bytes().to_vec();
    out.push(recid.to_byte() + 27);
    out
}

/// 从私钥推导钱包地址（EVM: keccak256(公钥X||Y) 后 20 字节，小写 0x）。
fn address_from_key(key: &SigningKey) -> String {
    let pk = key.verifying_key().to_encoded_point(false);
    let mut hasher = Keccak256::new();
    hasher.update(&pk.as_bytes()[1..]);
    let d = hasher.finalize();
    format!("0x{}", hex::encode(&d[12..]))
}

/// 从签名恢复公钥并推导地址（复现服务端验签路径，供演示比对）。
fn address_from_sig(message: &str, sig: &[u8]) -> String {
    let signature = Signature::try_from(&sig[..64]).unwrap();
    let recid_byte = if sig[64] >= 27 { sig[64] - 27 } else { sig[64] };
    let recid = RecoveryId::try_from(recid_byte).unwrap();
    let digest = Keccak256::new_with_prefix(eip191_message(message));
    let vk = VerifyingKey::recover_from_digest(digest, &signature, recid).unwrap();
    let pk = vk.to_encoded_point(false);
    let mut hasher = Keccak256::new();
    hasher.update(&pk.as_bytes()[1..]);
    let d = hasher.finalize();
    format!("0x{}", hex::encode(&d[12..]))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let srv = std::env::var("CRYSTAL_SERVER").unwrap_or_else(|_| "127.0.0.1:7000".to_string());
    let mut stream = TcpStream::connect(&srv).await.expect("连接服务器失败");
    let mut buf: Vec<u8> = Vec::new();

    async fn send_packet<P: PacketCodec>(stream: &mut TcpStream, p: &P) {
        stream.write_all(&encode_packet(p)).await.unwrap();
    }
    async fn recv_packet(buf: &mut Vec<u8>, stream: &mut TcpStream) -> (i16, Vec<u8>) {
        loop {
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

    // 生成模拟钱包（生产环境为 MetaMask 托管私钥）
    let mut secret = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut secret);
    let signer = SigningKey::from_bytes(&secret.into()).expect("valid scalar");
    let wallet_addr = address_from_key(&signer);

    println!("== 连接 {srv} ==");
    let (id, payload) = recv_packet(&mut buf, &mut stream).await;
    match ServerPacket::decode(id, &payload)? {
        ServerPacket::Connected(_) => println!("✓ 收到 Connected"),
        other => fail(&format!("期望 Connected，收到 {other:?}")),
    }

    println!("== ClientVersion ==");
    send_packet(&mut stream, &c::ClientVersion { version_hash: vec![] }).await;
    let (id, payload) = recv_packet(&mut buf, &mut stream).await;
    match ServerPacket::decode(id, &payload)? {
        ServerPacket::ClientVersion(v) => {
            assert_eq!(v.result, 1);
            println!("✓ 版本结果 = {}", v.result);
        }
        other => fail(&format!("期望 ClientVersion，收到 {other:?}")),
    }

    println!("钱包地址: {wallet_addr}");

    println!("== Web3ChallengeRequest ==");
    send_packet(&mut stream, &c::Web3ChallengeRequest { address: wallet_addr.clone() }).await;
    let (id, payload) = recv_packet(&mut buf, &mut stream).await;
    let challenge_msg = match ServerPacket::decode(id, &payload)? {
        ServerPacket::Web3Challenge(ch) => {
            assert_eq!(ch.address, wallet_addr);
            println!("✓ 收到挑战 (expires_in={}s): {}", ch.expires_in, ch.message);
            ch.message
        }
        other => fail(&format!("期望 Web3Challenge，收到 {other:?}")),
    };

    println!("== Web3Login (对挑战 personal_sign) ==");
    let sig = personal_sign(&signer, &challenge_msg);
    let sig_addr = address_from_sig(&challenge_msg, &sig);
    assert_eq!(sig_addr, wallet_addr, "恢复的地址应与上报地址一致");
    send_packet(
        &mut stream,
        &c::Web3Login {
            address: wallet_addr.clone(),
            challenge: challenge_msg,
            signature: sig,
        },
    )
    .await;

    let (id, payload) = recv_packet(&mut buf, &mut stream).await;
    match ServerPacket::decode(id, &payload)? {
        ServerPacket::Web3LoginResult(r) => {
            if r.result != 0 {
                fail(&format!("钱包登录失败 result={}", r.result));
            }
            println!("✓ 钱包登录成功，角色数 = {}", r.characters.len());
            assert_eq!(r.characters.len(), 0, "新钱包账号应尚无角色");
        }
        other => fail(&format!("期望 Web3LoginResult，收到 {other:?}")),
    }

    println!("\n✅ Web3 钱包登录端到端通过（地址即账号，已自动注册）");
    Ok(())
}
