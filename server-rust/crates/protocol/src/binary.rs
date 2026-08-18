//! .NET `BinaryReader`/`BinaryWriter` 兼容的读取/写入器（小端序）。
//!
//! 兼容语义（与 C# 完全一致）:
//! - 所有基本类型均为小端序
//! - `string`: 7-bit 可变长长度前缀 + UTF-8 字节（.NET `BinaryReader.ReadString`）
//! - `bool`: 1 字节（非 0 为 true）
//! - 附带 `DateTime.ToBinary/FromBinary` 与 ARGB 颜色助手

use crate::{ProtocolError, Result};

/// 从字节缓冲读取（带位置游标）
#[derive(Debug, Clone)]
pub struct Reader {
    data: Vec<u8>,
    pos: usize,
}

impl Reader {
    pub fn new(data: Vec<u8>) -> Self {
        Reader { data, pos: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    pub fn is_empty(&self) -> bool {
        self.pos >= self.data.len()
    }

    fn take(&mut self, n: usize) -> Result<Vec<u8>> {
        if self.remaining() < n {
            return Err(ProtocolError::UnexpectedEof {
                need: n,
                have: self.remaining(),
            });
        }
        let out = self.data[self.pos..self.pos + n].to_vec();
        self.pos += n;
        Ok(out)
    }

    pub fn read_bytes(&mut self, n: usize) -> Result<Vec<u8>> {
        self.take(n)
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    /// .NET `ReadSByte`
    pub fn read_i8(&mut self) -> Result<i8> {
        Ok(self.read_u8()? as i8)
    }

    pub fn read_bool(&mut self) -> Result<bool> {
        Ok(self.read_u8()? != 0)
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    pub fn read_i16(&mut self) -> Result<i16> {
        Ok(self.read_u16()? as i16)
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn read_i32(&mut self) -> Result<i32> {
        Ok(self.read_u32()? as i32)
    }

    pub fn read_u64(&mut self) -> Result<u64> {
        let b = self.take(8)?;
        Ok(u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    pub fn read_i64(&mut self) -> Result<i64> {
        Ok(self.read_u64()? as i64)
    }

    /// .NET `ReadSingle`（f32 小端）
    pub fn read_f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.read_u32()?))
    }

    /// .NET `ReadDouble`
    pub fn read_f64(&mut self) -> Result<f64> {
        Ok(f64::from_bits(self.read_u64()?))
    }

    /// .NET `BinaryReader.ReadString`: 7-bit 编码长度 + UTF-8
    pub fn read_string(&mut self) -> Result<String> {
        let len = self.read_7bit_encoded_int()?;
        let bytes = self.take(len)?;
        String::from_utf8(bytes).map_err(ProtocolError::InvalidString)
    }

    /// .NET 7-bit 编码整数（LEB128 风格，最多 5 字节）
    pub fn read_7bit_encoded_int(&mut self) -> Result<usize> {
        let mut result: usize = 0;
        let mut shift = 0;
        loop {
            if shift > 35 {
                return Err(ProtocolError::InvalidLengthPrefix);
            }
            let b = self.read_u8()?;
            result |= ((b & 0x7f) as usize) << shift;
            shift += 7;
            if b & 0x80 == 0 {
                break;
            }
        }
        Ok(result)
    }
}

/// 向字节缓冲写入
#[derive(Debug, Clone, Default)]
pub struct Writer {
    pub data: Vec<u8>,
}

impl Writer {
    pub fn new() -> Self {
        Writer { data: Vec::new() }
    }

    pub fn write_bytes(&mut self, b: &[u8]) {
        self.data.extend_from_slice(b);
    }

    pub fn write_u8(&mut self, v: u8) {
        self.data.push(v);
    }

    pub fn write_i8(&mut self, v: i8) {
        self.write_u8(v as u8);
    }

    pub fn write_bool(&mut self, v: bool) {
        self.write_u8(if v { 1 } else { 0 });
    }

    pub fn write_u16(&mut self, v: u16) {
        self.data.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_i16(&mut self, v: i16) {
        self.write_u16(v as u16);
    }

    pub fn write_u32(&mut self, v: u32) {
        self.data.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_i32(&mut self, v: i32) {
        self.write_u32(v as u32);
    }

    pub fn write_u64(&mut self, v: u64) {
        self.data.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_i64(&mut self, v: i64) {
        self.write_u64(v as u64);
    }

    pub fn write_f32(&mut self, v: f32) {
        self.write_u32(v.to_bits());
    }

    pub fn write_f64(&mut self, v: f64) {
        self.write_u64(v.to_bits());
    }

    /// .NET `BinaryWriter.Write(string)`: 7-bit 长度 + UTF-8
    pub fn write_string(&mut self, s: &str) {
        let bytes = s.as_bytes();
        self.write_7bit_encoded_int(bytes.len());
        self.write_bytes(bytes);
    }

    pub fn write_7bit_encoded_int(&mut self, mut v: usize) {
        while v >= 0x80 {
            self.write_u8(((v & 0x7f) | 0x80) as u8);
            v >>= 7;
        }
        self.write_u8(v as u8);
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

/// .NET `DateTime.Kind`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateTimeKind {
    Unspecified = 0,
    Utc = 1,
    Local = 2,
}

/// .NET 纪元偏移: 0001-01-01 至 1970-01-01（单位: 100ns ticks）
const DOTNET_EPOCH_OFFSET_TICKS: i64 = 621_355_968_000_000_000;
/// 每秒 tick 数（100ns）
const TICKS_PER_SECOND: i64 = 10_000_000;

/// 复刻 .NET `DateTime.ToBinary()`（返回写进协议的 i64）。
///
/// `binary = (kind << 62) | ticks`，ticks 为自 0001-01-01 的 100ns 数。
pub fn datetime_to_binary(unix_secs: i64, kind: DateTimeKind) -> i64 {
    let ticks = unix_secs * TICKS_PER_SECOND + DOTNET_EPOCH_OFFSET_TICKS;
    ((kind as i64) << 62) | ticks
}

/// 复刻 .NET `DateTime.FromBinary(long)` 的可逆解读。
///
/// 返回 (unix 秒, Kind)。注意: .NET 的 FromBinary 在本地时区下会做
/// Utc→Local 换算；协议侧我们只关心往返一致性，如实还原编码即可。
pub fn datetime_from_binary(v: i64) -> (i64, DateTimeKind) {
    let kind = match (v >> 62) & 0x3 {
        1 => DateTimeKind::Utc,
        2 => DateTimeKind::Local,
        _ => DateTimeKind::Unspecified,
    };
    let ticks = v & 0x3FFF_FFFF_FFFF_FFFF; // 62 bits
    let unix_secs = (ticks - DOTNET_EPOCH_OFFSET_TICKS) / TICKS_PER_SECOND;
    (unix_secs, kind)
}

/// ARGB 颜色（.NET `Color.ToArgb()` 返回 int，A 在高字节）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Argb(pub u32);

impl Argb {
    /// 从 `Color.ToArgb()` 的 i32 构造
    pub fn from_i32(v: i32) -> Self {
        Argb(v as u32)
    }
    /// 复刻 `Color.ToArgb()` 输出
    pub fn to_i32(self) -> i32 {
        self.0 as i32
    }
}

/// 二维点（Crystal 用两个 int32 序列化，无独立方法）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Point { x, y }
    }

    pub fn read(r: &mut Reader) -> Result<Self> {
        Ok(Point {
            x: r.read_i32()?,
            y: r.read_i32()?,
        })
    }

    pub fn write(&self, w: &mut Writer) {
        w.write_i32(self.x);
        w.write_i32(self.y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_roundtrip() {
        let mut w = Writer::new();
        w.write_string("hello");
        w.write_string("");
        w.write_string("传奇世界 🔥 测试");
        let data = w.into_inner();
        let mut r = Reader::new(data);
        assert_eq!(r.read_string().unwrap(), "hello");
        assert_eq!(r.read_string().unwrap(), "");
        assert_eq!(r.read_string().unwrap(), "传奇世界 🔥 测试");
        assert!(r.is_empty());
    }

    #[test]
    fn primitives_roundtrip() {
        let mut w = Writer::new();
        w.write_i8(-5);
        w.write_u8(200);
        w.write_bool(true);
        w.write_bool(false);
        w.write_i16(-1234);
        w.write_u16(65000);
        w.write_i32(-2_000_000_000);
        w.write_u32(4_000_000_000);
        w.write_i64(-9_000_000_000_000_000_000);
        w.write_u64(18_000_000_000_000_000_000);
        w.write_f32(3.5);
        w.write_f64(-2.25);
        let data = w.into_inner();
        let mut r = Reader::new(data);
        assert_eq!(r.read_i8().unwrap(), -5);
        assert_eq!(r.read_u8().unwrap(), 200);
        assert_eq!(r.read_bool().unwrap(), true);
        assert_eq!(r.read_bool().unwrap(), false);
        assert_eq!(r.read_i16().unwrap(), -1234);
        assert_eq!(r.read_u16().unwrap(), 65000);
        assert_eq!(r.read_i32().unwrap(), -2_000_000_000);
        assert_eq!(r.read_u32().unwrap(), 4_000_000_000);
        assert_eq!(r.read_i64().unwrap(), -9_000_000_000_000_000_000);
        assert_eq!(r.read_u64().unwrap(), 18_000_000_000_000_000_000);
        assert_eq!(r.read_f32().unwrap(), 3.5);
        assert_eq!(r.read_f64().unwrap(), -2.25);
    }

    #[test]
    fn datetime_binary_roundtrip() {
        // 已知值: Unix 0 (1970-01-01) UTC 的 ToBinary
        let bin = datetime_to_binary(0, DateTimeKind::Utc);
        let (secs, kind) = datetime_from_binary(bin);
        assert_eq!(secs, 0);
        assert_eq!(kind, DateTimeKind::Utc);
        // 与 .NET 已知数值对比: Utc + 0 ticks => ((1i64) << 62) | 621355968000000000
        assert_eq!(bin, (1i64 << 62) | 621_355_968_000_000_000);
    }
}
