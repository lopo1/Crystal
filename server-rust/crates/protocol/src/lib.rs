//! Crystal 协议核心库 —— C# (Shared) 二进制协议层的 Rust 移植。
//!
//! 兼容规则见 `docs/PROTOCOL.md`。所有序列化必须与 .NET `BinaryReader`/`BinaryWriter`
//! 逐字节一致（小端序、7-bit 编码字符串、gzip 压缩等）。

pub mod binary;
pub mod client;
pub mod frame;
pub mod ids;
pub mod server;
pub mod types;

pub use client::ClientPacket;
pub use ids::{ClientPacketId, ServerPacketId};
pub use server::ServerPacket;

/// 协议层错误
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("unexpected end of buffer (need {need} bytes, only {have} left)")]
    UnexpectedEof { need: usize, have: usize },
    #[error("invalid utf-8 string: {0}")]
    InvalidString(#[from] std::string::FromUtf8Error),
    #[error("invalid utf-8 string slice: {0}")]
    InvalidStringSlice(#[from] std::str::Utf8Error),
    #[error("invalid packet id {0}")]
    InvalidPacketId(i16),
    #[error("malformed frame: {0}")]
    MalformedFrame(String),
    #[error("invalid 7-bit encoded length")]
    InvalidLengthPrefix,
}

pub type Result<T> = std::result::Result<T, ProtocolError>;
