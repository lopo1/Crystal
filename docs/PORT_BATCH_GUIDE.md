# 数据包批量移植指南（给移植子代理）

把 Crystal C# 数据包类逐字节移植为 Rust。**字段顺序错一位即全错**，必须严格对照。

## 1. 必读文件

- `docs/PROTOCOL.md` — 字节级兼容规则（真源）
- `server-rust/crates/protocol/src/client/mod.rs` — 客户端包已移植范例（模式参考）
- `server-rust/crates/protocol/src/server/mod.rs` — 服务器包已移植范例（含 `read_item_slots`/`write_item_slots` 等 `pub(crate)` 工具）
- `server-rust/crates/protocol/src/types.rs` — 已移植的内嵌类型（可直接用）

## 2. 你创建的文件（只动这两个，别碰其它文件）

1. `server-rust/crates/protocol/src/client/batch_N.rs` 或 `server-rust/crates/protocol/src/server/batch_N.rs`
   （占位文件已存在，直接覆盖内容）
2. `server-rust/crates/protocol/tests/batch_N.rs` — 本批回环测试

## 3. 代码模式

```rust
// batch 文件头
use crate::binary::{Argb, Point, Reader, Writer};
use crate::frame::PacketCodec;
use crate::ids::ClientPacketId;   // 或 ServerPacketId
use crate::types::*;
use crate::Result;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FooPacket {
    pub some_int: i32,
    pub name: String,
    pub location: Point,
    pub colour: Argb,
    pub dt: i64,               // DateTime.ToBinary()
}

impl PacketCodec for FooPacket {
    const ID: i16 = ClientPacketId::FooPacket as i16;   // 服务器包用 ServerPacketId

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(FooPacket {
            some_int: r.read_i32()?,
            name: r.read_string()?,
            location: Point::read(r)?,
            colour: Argb::from_i32(r.read_i32()?),
            dt: r.read_i64()?,
        })
    }

    fn write(&self, w: &mut Writer) {
        w.write_i32(self.some_int);
        w.write_string(&self.name);
        self.location.write(w);
        w.write_i32(self.colour.to_i32());
        w.write_i64(self.dt);
    }
}
```

## 4. C# → Rust 类型映射（速查）

| C# | Rust |
|---|---|
| int / uint / short / ushort / long / ulong | i32 / u32 / i16 / u16 / i64 / u64 |
| byte / sbyte / bool | u8 / i8 / bool（Reader::read_bool/writer.write_bool） |
| float / double | f32 / f64 |
| string | String（read_string / write_string，含 7-bit 前缀） |
| byte[] | Vec\<u8\>（read_bytes(n) / write_bytes） |
| Point (X,Y) | `Point`（read/write 方法） |
| Color.ToArgb()/FromArgb | `Argb`（Argb::from_i32 / to_i32） |
| DateTime.ToBinary()/FromBinary | `i64`（原值直通，不做换算） |
| 枚举（byte/short） | u8 / u16 / i16 原始值字段（注释注明枚举名，见 types.rs 惯例） |
| List\<T\> | Vec\<T\>（count 在前） |
| 数组 UserItem[] | Vec\<Option\<UserItem\>\> 或 `server::read_item_slots`（服务器包专用，注意 C# 的空槽布尔方向: `Write(Items[i] == null)` / 读 `if (ReadBoolean()) continue;`） |

**服务器包层**的 `ItemSlots`/`read_item_slots`/`write_item_slots` 在 `server/mod.rs` 里，是 `pub(crate)` 的，
batch 文件里用 `use super::{read_item_slots, write_item_slots};` 引用。

## 5. 通用循环/条件模式

- 列表: C# 先 `writer.Write(Count)` 再逐项；读先读 count 再循环——照搬。
- 位标志（bools）: 一个 byte 按位组合，读时 `if ((bools & 0x01) == 0x01) X = true;` → 照搬 AND 判断。
- 可空对象: `if (reader.ReadBoolean()) X = new Type(reader); else X = null;` —— **注意方向**：有的地方
  Write(bool 存在)，有的地方 Write(bool 为空)——以 C# 原代码为准！

## 6. 测试要求（tests/batch_N.rs）

为每个包写 `write→read` 回环测试，断言字段相等且 `reader.is_empty()`，并用**有代表性的值**
（非 0、非空字符串、含子列表）。范例见 `tests/packets.rs`。

## 7. 质量自检

1. 改完运行 `cargo build -p crystal-protocol 2>&1 | grep error`（模块已在 mod.rs 声明，可直接编译）。
2. 运行 `cargo test -p crystal-protocol --test batch_N` 全绿。
3. 对照 C# 原文件逐字段核对 read 顺序 == write 顺序 == C# 顺序。
4. 遇到拿不准的（如继承类、重载 ReadPacket/WritePacket、特殊枚举），在交付说明里列出让主代理复核。