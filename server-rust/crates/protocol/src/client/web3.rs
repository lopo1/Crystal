//! Web3 钱包登录扩展包（客户端→服务器，自定义 ID，非原 Crystal 协议）。
//!
//! 原 Crystal 客户端包 ID 范围为 0..152，本扩展使用 200+ 的保留段，
//! 与原始枚举不冲突。两端（Rust 服务器 / Godot 客户端）必须一致。

use crate::binary::{Reader, Writer};
use crate::frame::PacketCodec;
use crate::Result;

/// Web3 反向通道客户端包起始 ID（自定义，避开原始 0..152）。
pub const WEB3_CLIENT_BASE: i16 = 200;

/// ID 200 —— 请求登录挑战（握手 = 签名前的第一步）
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Web3ChallengeRequest {
    /// 钱包地址（小写 0x 十六进制，如 0xabcd…）
    pub address: String,
}

impl PacketCodec for Web3ChallengeRequest {
    const ID: i16 = WEB3_CLIENT_BASE;
    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Web3ChallengeRequest {
            address: r.read_string()?,
        })
    }
    fn write(&self, w: &mut Writer) {
        w.write_string(&self.address);
    }
}

/// ID 201 —— 提交已签名挑战完成登录
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Web3Login {
    /// 钱包地址（小写 0x 十六进制）
    pub address: String,
    /// 服务器下发的挑战原文（EIP-191 personal_sign 的 message）
    pub challenge: String,
    /// 65 字节 ECDSA 签名: r(32) || s(32) || v(1)（v ∈ {0,1,27,28}）
    pub signature: Vec<u8>,
}

impl PacketCodec for Web3Login {
    const ID: i16 = WEB3_CLIENT_BASE + 1;
    fn read(r: &mut Reader) -> Result<Self> {
        let address = r.read_string()?;
        let challenge = r.read_string()?;
        let sig_len = r.read_i32()?.max(0) as usize;
        let signature = r.read_bytes(sig_len)?;
        Ok(Web3Login {
            address,
            challenge,
            signature,
        })
    }
    fn write(&self, w: &mut Writer) {
        w.write_string(&self.address);
        w.write_string(&self.challenge);
        w.write_i32(self.signature.len() as i32);
        w.write_bytes(&self.signature);
    }
}
