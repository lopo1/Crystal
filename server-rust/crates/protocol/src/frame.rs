//! 帧编解码 —— 复刻 `Shared/Packet.cs` 的收发包格式。
//!
//! 帧: `[u16 LE 总长][i16 LE 包ID][载荷]`，总长含自身 2 字节与 ID 2 字节。

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::{Read, Write};

use crate::{ProtocolError, Result};

/// 编码一个帧（不做压缩）
///
/// 帧: `[u16 LE 总长][i16 LE 包ID][载荷]`，总长 = 4 + 载荷长（含 4 字节头），
/// 与 C# `Packet.GetPacketBytes` 一致。
pub fn encode_frame(id: i16, payload: &[u8]) -> Vec<u8> {
    let len = 4 + payload.len(); // 4 字节头 + 载荷
    let mut out = Vec::with_capacity(len);
    out.extend_from_slice(&(len as u16).to_le_bytes());
    out.extend_from_slice(&id.to_le_bytes());
    out.extend_from_slice(payload);
    out
}

/// 从缓冲解码一个帧。
///
/// 返回 `(包ID, 载荷, 本帧占用字节数)`。与 C# 一致: 长度非法（<2 或长度>缓冲余量）
/// 返回错误，调用方应丢弃整个缓冲（防死循环）。
pub fn decode_frame(data: &[u8]) -> Result<(i16, &[u8], usize)> {
    if data.len() < 4 {
        return Err(ProtocolError::MalformedFrame(format!(
            "frame shorter than 4 bytes (got {})",
            data.len()
        )));
    }
    let len = u16::from_le_bytes([data[0], data[1]]) as usize;
    if len < 2 || len > data.len() {
        return Err(ProtocolError::MalformedFrame(format!(
            "invalid frame length {len} (buffer {})",
            data.len()
        )));
    }
    let id = i16::from_le_bytes([data[2], data[3]]);
    Ok((id, &data[4..len], len))
}

/// gzip 压缩（.NET `GZipStream` 默认头，与 `CompressBytes` 一致）
pub fn compress_bytes(raw: &[u8]) -> Vec<u8> {
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(raw).expect("gzip write");
    enc.finish().expect("gzip finish")
}

/// gzip 解压（与 `DecompressBytes` 一致）
pub fn decompress_bytes(gzip: &[u8]) -> Result<Vec<u8>> {
    let mut dec = GzDecoder::new(gzip);
    let mut out = Vec::new();
    dec.read_to_end(&mut out)
        .map_err(|e| ProtocolError::MalformedFrame(format!("gzip decompress: {e}")))?;
    Ok(out)
}

/// 数据包编解码接口（对应 C# 的 Packet 类）
pub trait PacketCodec: Sized {
    const ID: i16;
    /// 该包载荷是否 gzip 压缩（C# `Compressed`）
    const COMPRESSED: bool = false;

    fn read(r: &mut crate::binary::Reader) -> Result<Self>;
    fn write(&self, w: &mut crate::binary::Writer);
}

/// 编码数据包为完整帧
pub fn encode_packet<P: PacketCodec>(p: &P) -> Vec<u8> {
    let mut w = crate::binary::Writer::new();
    p.write(&mut w);
    let payload = w.into_inner();
    if P::COMPRESSED {
        let compressed = compress_bytes(&payload);
        encode_frame(P::ID, &compressed)
    } else {
        encode_frame(P::ID, &payload)
    }
}

/// 从帧载荷解码数据包。
///
/// `id`: 数据包 ID；`payload`: 帧载荷（`decode_frame` 输出）。
/// 若包声明压缩，自动解压。
pub fn decode_packet<P: PacketCodec>(id: i16, payload: &[u8]) -> Result<P> {
    if id != P::ID {
        return Err(ProtocolError::InvalidPacketId(id));
    }
    let bytes = if P::COMPRESSED {
        decompress_bytes(payload)?
    } else {
        payload.to_vec()
    };
    let mut r = crate::binary::Reader::new(bytes);
    P::read(&mut r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_roundtrip() {
        let payload = [1u8, 2, 3, 4, 5];
        let frame = encode_frame(42, &payload);
        assert_eq!(frame.len(), 4 + 5);
        assert_eq!(&frame[..2], &9u16.to_le_bytes()[..]);
        assert_eq!(&frame[2..4], &42i16.to_le_bytes()[..]);
        let (id, out, used) = decode_frame(&frame).unwrap();
        assert_eq!(id, 42);
        assert_eq!(out, &payload[..]);
        assert_eq!(used, 9);
    }

    #[test]
    fn frame_rejects_bad_length() {
        // 长度字段大于缓冲 -> 应报错（调用方丢弃缓冲）
        let mut frame = encode_frame(1, &[0u8; 10]);
        frame[0] = 0xff;
        frame[1] = 0xff;
        assert!(decode_frame(&frame).is_err());
    }

    #[test]
    fn gzip_roundtrip() {
        let raw = vec![7u8; 1024];
        let comp = compress_bytes(&raw);
        assert!(comp.len() < raw.len());
        assert_eq!(decompress_bytes(&comp).unwrap(), raw);
    }
}
