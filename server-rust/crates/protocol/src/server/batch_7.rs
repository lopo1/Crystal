//! 补齐批（batch_7）—— 覆盖各批次遗漏的核心包。
//!
//! server: StorageUnlockResult, StoragePasswordResult, SplitItem1, NewItemInfo
//! 这些是第一批(阶段0)清单外的遗漏，按 C# Shared/ServerPackets.cs 逐字节移植。

use crate::binary::{Argb, Point, Reader, Writer};
use crate::frame::PacketCodec;
use crate::ids::ServerPacketId;
use crate::types::{ItemInfo, MirDirection};
use crate::Result;

// ----------------------------- ID 170: StorageUnlockResult -----------------------------
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StorageUnlockResult {
    /// 0成功 1错密 2错密码 3不可用 4未设密
    pub result: u8,
    pub has_password: bool,
}

impl PacketCodec for StorageUnlockResult {
    const ID: i16 = ServerPacketId::StorageUnlockResult as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(StorageUnlockResult {
            result: r.read_u8()?,
            has_password: r.read_bool()?,
        })
    }
    fn write(&self, w: &mut Writer) {
        w.write_u8(self.result);
        w.write_bool(self.has_password);
    }
}

// ----------------------------- ID 278: StoragePasswordResult -----------------------------
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoragePasswordResult {
    pub result: u8,
    pub removing: bool,
    pub has_password: bool,
    /// DateTime .NET binary
    pub last_set_time: i64,
}

impl PacketCodec for StoragePasswordResult {
    const ID: i16 = ServerPacketId::StoragePasswordResult as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(StoragePasswordResult {
            result: r.read_u8()?,
            removing: r.read_bool()?,
            has_password: r.read_bool()?,
            last_set_time: r.read_i64()?,
        })
    }
    fn write(&self, w: &mut Writer) {
        w.write_u8(self.result);
        w.write_bool(self.removing);
        w.write_bool(self.has_password);
        w.write_i64(self.last_set_time);
    }
}

// ----------------------------- ID 45: SplitItem1 -----------------------------
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SplitItem1 {
    /// MirGridType (u8)
    pub grid: u8,
    pub unique_id: u64,
    pub count: u16,
    pub success: bool,
}

impl PacketCodec for SplitItem1 {
    const ID: i16 = ServerPacketId::SplitItem1 as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(SplitItem1 {
            grid: r.read_u8()?,
            unique_id: r.read_u64()?,
            count: r.read_u16()?,
            success: r.read_bool()?,
        })
    }
    fn write(&self, w: &mut Writer) {
        w.write_u8(self.grid);
        w.write_u64(self.unique_id);
        w.write_u16(self.count);
        w.write_bool(self.success);
    }
}

// ----------------------------- ID 33: NewItemInfo -----------------------------
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewItemInfo {
    pub info: ItemInfo,
}

impl PacketCodec for NewItemInfo {
    const ID: i16 = ServerPacketId::NewItemInfo as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(NewItemInfo {
            info: ItemInfo::read(r)?,
        })
    }
    fn write(&self, w: &mut Writer) {
        self.info.write(w);
    }
}

// (ObjectNPC 已在 batch_2 实现为 ObjectNPC，ID 用 ObjectNpc；WorldMapSetup 由 WorldMapSetupInfo 覆盖；
//  UserDash 系列已由 batch_4 宏生成；此处不再重复)
