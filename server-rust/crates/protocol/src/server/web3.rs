//! Web3 钱包登录扩展包（服务器→客户端，自定义 ID，非原 Crystal 协议）。
//!
//! 服务器包起始 ID 使用 300+ 保留段，与原始枚举（0..278）不冲突。

use crate::binary::{Reader, Writer};
use crate::frame::PacketCodec;
use crate::types::SelectInfo;
use crate::Result;

/// Web3 服务器包起始 ID。
pub const WEB3_SERVER_BASE: i16 = 300;

/// ID 300 —— 返回待签名的挑战
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Web3Challenge {
    /// 钱包地址（回显，供客户端核对）
    pub address: String,
    /// 待签名原文（客户端用 MetaMask 等以 personal_sign 签名）
    pub message: String,
    /// 挑战有效期（秒），超时需重新请求
    pub expires_in: i64,
}

impl PacketCodec for Web3Challenge {
    const ID: i16 = WEB3_SERVER_BASE;
    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Web3Challenge {
            address: r.read_string()?,
            message: r.read_string()?,
            expires_in: r.read_i64()?,
        })
    }
    fn write(&self, w: &mut Writer) {
        w.write_string(&self.address);
        w.write_string(&self.message);
        w.write_i64(self.expires_in);
    }
}

/// ID 301 —— 钱包登录结果
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Web3LoginResult {
    /// 0 成功; 1 地址非法; 2 挑战已过期; 3 签名无效/地址不匹配
    pub result: u8,
    /// 成功时的角色列表（与 LoginSuccess 同构，供客户端进入选择界面）
    pub characters: Vec<SelectInfo>,
}

impl PacketCodec for Web3LoginResult {
    const ID: i16 = WEB3_SERVER_BASE + 1;
    fn read(r: &mut Reader) -> Result<Self> {
        let result = r.read_u8()?;
        let count = r.read_i32()?.max(0) as usize;
        let mut characters = Vec::with_capacity(count);
        for _ in 0..count {
            characters.push(SelectInfo::read(r)?);
        }
        Ok(Web3LoginResult { result, characters })
    }
    fn write(&self, w: &mut Writer) {
        w.write_u8(self.result);
        w.write_i32(self.characters.len() as i32);
        for c in &self.characters {
            c.write(w);
        }
    }
}
