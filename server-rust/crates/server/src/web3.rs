//! Web3 钱包登录（阶段3）：EVM EIP-191 签名验证 + 挑战-应答。
//!
//! 流程:
//!   1. 客户端上报钱包地址 (`Web3ChallengeRequest`)
//!   2. 服务器签发一次性挑战 (`Web3Challenge`)，过期作废
//!   3. 客户端用钱包对挑战做 `personal_sign` (EIP-191)
//!   4. 服务器恢复公钥 → 推导地址 → 校验与上报一致 → 登录
//!
//! 账号即钱包地址（小写 0x 十六进制），首次成功登录自动注册。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use k256::ecdsa::{RecoveryId, Signature, SigningKey, VerifyingKey};
use sha3::{Digest, Keccak256};

/// 挑战有效期（秒）
pub const CHALLENGE_TTL: Duration = Duration::from_secs(300);

/// 地址规范长度：`0x` + 40 个十六进制字符
const ADDR_LEN: usize = 42;

/// 签名长度：r(32) || s(32) || v(1)
const SIG_LEN: usize = 65;

#[derive(Debug, Clone)]
struct Challenge {
    message: String,
    issued_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Web3Error {
    /// 地址格式非法（非 0x + 40 hex）
    InvalidAddress,
    /// 挑战过期或从未签发（需重新请求）
    ChallengeExpired,
    /// 签名长度非法 / ECDSA 解析失败 / 恢复的地址与上报不一致
    InvalidSignature,
}

impl std::fmt::Display for Web3Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Web3Error::InvalidAddress => write!(f, "invalid wallet address"),
            Web3Error::ChallengeExpired => write!(f, "challenge expired or missing"),
            Web3Error::InvalidSignature => write!(f, "invalid signature or address mismatch"),
        }
    }
}

/// 钱包登录认证状态（每连接一个，跨连接共享挑战在 stateless 模式下无需共享；
/// 这里用一个进程内挑战表，便于校验时间与一次性）。
#[derive(Debug, Clone)]
pub struct Web3Auth {
    challenges: Arc<Mutex<HashMap<String, Challenge>>>,
    /// 本服务器实例指纹（让挑战原文唯一且可辨识）
    server_id: String,
}

impl Web3Auth {
    pub fn new() -> Self {
        Web3Auth {
            challenges: Arc::new(Mutex::new(HashMap::new())),
            server_id: format!("{:x}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos()),
        }
    }

    /// 校验并规范化钱包地址（小写 0x 十六进制）。
    /// 返回 `(address_lower, addr_bytes)`。
    pub fn normalize_address(raw: &str) -> std::result::Result<(String, [u8; 20]), Web3Error> {
        let raw = raw.trim();
        let lower = raw.to_ascii_lowercase();
        if !lower.starts_with("0x") || lower.len() != ADDR_LEN || !lower[2..].chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(Web3Error::InvalidAddress);
        }
        let hex_part = &lower[2..];
        let mut bytes = [0u8; 20];
        for (i, ch) in hex_part.as_bytes().chunks(2).enumerate() {
            let b = u8::from_str_radix(std::str::from_utf8(ch).unwrap_or("00"), 16).unwrap_or(0);
            bytes[i] = b;
        }
        Ok((lower, bytes))
    }

    /// 签发挑战。`address` 为规范化小写地址。
    pub fn issue_challenge(&self, address: &str) -> Web3ChallengeMsg {
        let nonce = format!(
            "Crystal ({}) 正在请求登录：\n\n请在下述挑战上签名以验证钱包所有权。\n\nNonce: {}",
            &self.server_id[..12],
            random_hex(16),
        );
        self.challenges.lock().unwrap().insert(
            address.to_string(),
            Challenge {
                message: nonce.clone(),
                issued_at: Instant::now(),
            },
        );
        Web3ChallengeMsg {
            address: address.to_string(),
            message: nonce,
            expires_in: CHALLENGE_TTL.as_secs() as i64,
        }
    }

    /// 验证签名并返回恢复出的钱包地址（成功即账号名）。
    ///
    /// 仅在 `verify_signature` 返回的地址与上报地址一致时返回 Ok((address, addr_bytes))，
    /// 且原子性地删除挑战（一次性）。
    pub fn verify_and_consume(
        &self,
        address: &str,
        challenge: &str,
        signature: &[u8],
    ) -> std::result::Result<(String, [u8; 20]), Web3Error> {
        let (addr_norm, addr_bytes) = Self::normalize_address(address)?;

        // 校验挑战存在且为当前签发的原文
        {
            let mut map = self.challenges.lock().unwrap();
            let ch = map
                .get(&addr_norm)
                .ok_or(Web3Error::ChallengeExpired)?;
            if ch.message != challenge {
                return Err(Web3Error::ChallengeExpired);
            }
            if ch.issued_at.elapsed() > CHALLENGE_TTL {
                map.remove(&addr_norm);
                return Err(Web3Error::ChallengeExpired);
            }
            map.remove(&addr_norm); // 一次性
        }

        let recovered = recover_address(challenge, signature).map_err(|_| Web3Error::InvalidSignature)?;
        if recovered != addr_bytes {
            return Err(Web3Error::InvalidSignature);
        }
        Ok((addr_norm, addr_bytes))
    }
}

pub struct Web3ChallengeMsg {
    pub address: String,
    pub message: String,
    pub expires_in: i64,
}

/// 构造 EIP-191 personal_sign 的待哈希原文: `\x19Ethereum Signed Message:\n<len> <msg>`。
fn eip191_message(message: &str) -> Vec<u8> {
    let len = message.len();
    let mut s = b"\x19Ethereum Signed Message:\n".to_vec();
    s.extend_from_slice(len.to_string().as_bytes());
    s.push(b' ');
    s.extend_from_slice(message.as_bytes());
    s
}

/// 从 65 字节签名恢复出钱包地址（20 字节）。
fn recover_address(message: &str, sig: &[u8]) -> std::result::Result<[u8; 20], Box<dyn std::error::Error>> {
    if sig.len() != SIG_LEN {
        return Err("signature length must be 65".into());
    }
    let signature = Signature::try_from(&sig[..64])?;
    // Ethereum personal_sign 的 v ∈ {27,28}，部分钱包返回 {0,1}；两者都映射为 recovery id。
    let recid_byte = if sig[64] >= 27 { sig[64] - 27 } else { sig[64] };
    let recid = RecoveryId::try_from(recid_byte)?;

    let digest = Keccak256::new_with_prefix(eip191_message(message));
    let recovered_key = VerifyingKey::recover_from_digest(digest, &signature, recid)?;

    // 非压缩公钥: 0x04 || X(32) || Y(32)；地址 = keccak256(X||Y) 后 20 字节
    let pk = recovered_key.to_encoded_point(false);
    let xy = &pk.as_bytes()[1..];
    let mut hasher = Keccak256::new();
    hasher.update(xy);
    let digest = hasher.finalize();
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&digest[12..]);
    Ok(addr)
}

fn random_hex(len_bytes: usize) -> String {
    use rand::RngCore;
    let mut bytes = vec![0u8; len_bytes];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// 从一个 ECDSA 签名推导钱包地址的共享子路（内部）。
#[allow(dead_code)]
fn address_from_pk_xy(xy: &[u8]) -> String {
    let mut hasher = Keccak256::new();
    hasher.update(xy);
    let digest = hasher.finalize();
    let addr = &digest[12..];
    format!("0x{}", hex::encode(addr))
}

/// 从 `VerifyingKey` 推导钱包地址（小写 0x 十六进制）。
/// 供测试与 Godot 侧对照；生产 path 由签名恢复（见 `recover_address`）。
#[allow(dead_code)]
pub fn address_from_verifying(vk: &VerifyingKey) -> String {
    let pk = vk.to_encoded_point(false);
    address_from_pk_xy(&pk.as_bytes()[1..])
}

/// 生成一个测试用密钥对（供 demo_client / 集成测试）。
/// 返回 (secret_bytes, address_hex)
#[allow(dead_code)]
pub fn generate_test_wallet() -> (Vec<u8>, String) {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let key = SigningKey::from_bytes(&bytes.into()).expect("valid scalar");
    let addr = address_from_verifying(key.verifying_key());
    (bytes.to_vec(), addr)
}

/// 对 EIP-191 personal_sign 消息签名（供 demo 客户端 / 测试使用）。
/// 返回 65 字节 r||s||v（v=27/28）。
#[allow(dead_code)]
pub fn personal_sign(key: &SigningKey, message: &str) -> Vec<u8> {
    let digest = Keccak256::new_with_prefix(eip191_message(message));
    let (sig, recid) = key.sign_digest_recoverable(digest).expect("sign");
    let mut out = sig.to_bytes().to_vec();
    out.push(recid.to_byte() + 27);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::SecretKey;

    fn test_key() -> SigningKey {
        // 固定的测试私钥（不依赖随机，保证可复现）
        let sk = SecretKey::from_slice(&[7u8; 32]).expect("valid key");
        SigningKey::from(sk)
    }

    #[test]
    fn address_derivation_is_40_hex_chars() {
        let key = test_key();
        let addr = address_from_verifying(key.verifying_key());
        assert!(addr.starts_with("0x"));
        assert_eq!(addr.len(), 42);
        assert!(addr[2..].chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn normalize_address_accepts_and_lowercases() {
        let raw = "0xAbCdEf0000000000000000000000000000000001";
        let (lower, bytes) = Web3Auth::normalize_address(raw).unwrap();
        assert_eq!(lower, "0xabcdef0000000000000000000000000000000001");
        assert_eq!(bytes.len(), 20);
        assert!(Web3Auth::normalize_address("0xzz").is_err());
        assert!(Web3Auth::normalize_address("0x1234").is_err()); // 长度不足
    }

    #[test]
    fn full_challenge_signature_verification() {
        let key = test_key();
        let addr = address_from_verifying(key.verifying_key());
        let auth = Web3Auth::new();

        let ch = auth.issue_challenge(&addr);
        assert_eq!(ch.address, addr);
        assert!(ch.expires_in > 0);

        // 用私钥对挑战签名（模拟钱包 personal_sign）
        let sig = personal_sign(&key, &ch.message);

        let (recovered_addr, _) = auth.verify_and_consume(&addr, &ch.message, &sig).unwrap();
        assert_eq!(recovered_addr, addr);    }

    #[test]
    fn wrong_signer_is_rejected() {
        let key = test_key();
        let addr = address_from_verifying(key.verifying_key());
        let auth = Web3Auth::new();
        let ch = auth.issue_challenge(&addr);

        // 用另一个密钥签名
        let other_sk = SecretKey::from_slice(&[9u8; 32]).unwrap();
        let other_key = SigningKey::from(other_sk);
        let sig = personal_sign(&other_key, &ch.message);

        assert_eq!(
            auth.verify_and_consume(&addr, &ch.message, &sig),
            Err(Web3Error::InvalidSignature)
        );
    }

    #[test]
    fn challenge_is_single_use() {
        let key = test_key();
        let addr = address_from_verifying(key.verifying_key());
        let auth = Web3Auth::new();
        let ch = auth.issue_challenge(&addr);
        let sig = personal_sign(&key, &ch.message);

        assert!(auth.verify_and_consume(&addr, &ch.message, &sig).is_ok());
        // 二次使用：挑战已消耗，应失败
        assert!(auth.verify_and_consume(&addr, &ch.message, &sig).is_err());
    }

    #[test]
    fn tampered_message_is_rejected() {
        let key = test_key();
        let addr = address_from_verifying(key.verifying_key());
        let auth = Web3Auth::new();
        let ch = auth.issue_challenge(&addr);
        let sig = personal_sign(&key, &ch.message);

        // 篡改挑战原文
        assert!(auth.verify_and_consume(&addr, &"篡改后的消息", &sig).is_err());
    }
}
