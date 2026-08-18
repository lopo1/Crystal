//! 补齐批（client batch_7）—— 覆盖遗漏的存储密码客户端包。
//!
//! UnlockStorage, SetStoragePassword, RemoveStoragePassword (ID 150-152)
//! 按 C# Shared/ClientPackets.cs 逐字节移植。

use crate::binary::{Reader, Writer};
use crate::frame::PacketCodec;
use crate::ids::ClientPacketId;
use crate::Result;

// ----------------------------- ID 150: UnlockStorage -----------------------------
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UnlockStorage {
    pub password: String,
}

impl PacketCodec for UnlockStorage {
    const ID: i16 = ClientPacketId::UnlockStorage as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(UnlockStorage {
            password: r.read_string()?,
        })
    }
    fn write(&self, w: &mut Writer) {
        w.write_string(&self.password);
    }
}

// ----------------------------- ID 151: SetStoragePassword -----------------------------
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SetStoragePassword {
    pub current_password: String,
    pub new_password: String,
}

impl PacketCodec for SetStoragePassword {
    const ID: i16 = ClientPacketId::SetStoragePassword as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(SetStoragePassword {
            current_password: r.read_string()?,
            new_password: r.read_string()?,
        })
    }
    fn write(&self, w: &mut Writer) {
        w.write_string(&self.current_password);
        w.write_string(&self.new_password);
    }
}

// ----------------------------- ID 152: RemoveStoragePassword -----------------------------
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RemoveStoragePassword {
    pub current_password: String,
}

impl PacketCodec for RemoveStoragePassword {
    const ID: i16 = ClientPacketId::RemoveStoragePassword as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(RemoveStoragePassword {
            current_password: r.read_string()?,
        })
    }
    fn write(&self, w: &mut Writer) {
        w.write_string(&self.current_password);
    }
}

// ----------------------------- ID 117: GuildStorageGoldChange -----------------------------
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuildStorageGoldChange {
    pub r#type: u8,
    pub amount: u32,
}

impl PacketCodec for GuildStorageGoldChange {
    const ID: i16 = ClientPacketId::GuildStorageGoldChange as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(GuildStorageGoldChange {
            r#type: r.read_u8()?,
            amount: r.read_u32()?,
        })
    }
    fn write(&self, w: &mut Writer) {
        w.write_u8(self.r#type);
        w.write_u32(self.amount);
    }
}

// ----------------------------- ID 118: GuildStorageItemChange -----------------------------
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuildStorageItemChange {
    pub r#type: u8,
    pub from: i32,
    pub to: i32,
}

impl PacketCodec for GuildStorageItemChange {
    const ID: i16 = ClientPacketId::GuildStorageItemChange as i16;

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(GuildStorageItemChange {
            r#type: r.read_u8()?,
            from: r.read_i32()?,
            to: r.read_i32()?,
        })
    }
    fn write(&self, w: &mut Writer) {
        w.write_u8(self.r#type);
        w.write_i32(self.from);
        w.write_i32(self.to);
    }
}
