# 协议移植规则（字节级兼容契约）

Crystal 客户端/服务器使用 `BinaryReader`/`BinaryWriter`（.NET，小端序）。Rust 与 Godot 端口必须逐字节复刻。**本文件是移植的唯一真源（source of truth）。**

## 1. 帧格式 (frame)

```
偏移 0   : u16 LE  总长度 (length)  —— 含本身 2 字节
偏移 2   : i16 LE  数据包 ID
偏移 4   : 载荷    —— 长度 = length - 4
```

- 收到数据后:**长度 < 2 或 > 缓冲剩余** → 丢弃整个缓冲（防死循环）
- 压缩:包若 `Compressed == true`，载荷是 gzip 流（.NET GZipStream 默认头），解压后才是真实负载
- .NET GZipStream ⇔ Rust `flate2::read::GzDecoder/GzEncoder`（默认配置）字节兼容

## 2. 基础类型映射 (binary)

| C# (BinaryReader/Writer) | Rust | 备注 |
|---|---|---|
| `ReadByte` / `Write(byte)` | `u8` | |
| `ReadSByte` / `Write(sbyte)` | `i8` | |
| `ReadInt16` / `Write(short)` | `i16` LE | |
| `ReadUInt16` / `Write(ushort)` | `u16` LE | |
| `ReadInt32` / `Write(int)` | `i32` LE | |
| `ReadUInt32` / `Write(uint)` | `u32` LE | |
| `ReadInt64` / `Write(long)` | `i64` LE | |
| `ReadUInt64` / `Write(ulong)` | `u64` LE | |
| `ReadSingle` / `Write(float)` | `f32` LE | |
| `ReadDouble` / `Write(double)` | `f64` LE | |
| `ReadBoolean` / `Write(bool)` | `u8 != 0` | 1 字节 |
| `ReadBytes(int n)` | `Vec<u8>` 读 n 字节 | **无长度前缀** |
| `ReadString` / `Write(string)` | 见 §3 | |

## 3. 字符串 — .NET 7-bit 编码 (关键!)

`BinaryReader.ReadString` 编码:
1. 7-bit 可变长长度前缀(小端 7-bit groups，同 LEB128)，最多 5 字节，每字节高 1 位为"续"
2. 随后是 UTF-8 字节

```rust
pub fn read_len_prefixed_string(r: &mut Reader) -> String {
    let len = read_7bit_encoded_int(r);          // usize
    let bytes = r.read_bytes(len)?;
    String::from_utf8(bytes)?
}
pub fn write_len_prefixed_string(w: &mut Writer, s: &str) {
    write_7bit_encoded_int(w, s.len());
    w.write_bytes(s.as_bytes());
}
```

## 4. 特殊类型

| C# 表达式 | 字节格式 | 说明 |
|---|---|---|
| `writer.Write(Color.ToArgb())` | `i32` LE = ARGB (A 在高字节) | 读: `Color.FromArgb(reader.ReadInt32())` |
| `writer.Write(DateTime.ToBinary())` | `i64` LE | `.NET ticks`，见下 |
| `Point` | `i32 X` + `i32 Y`（按字段分别写） | 无独立序列化方法 |

### DateTime.ToBinary / FromBinary 复刻

.NET `DateTime.ToBinary()` = `(kind << 62) | ticks`，其中:
- `ticks` = 自 0001-01-01 起 100ns 单位数 (`unix_sec * 10_000_000 + 621355968000000000`)
- `kind`: 0 = Unspecified, 1 = Utc, 2 = Local
- 写: 服务器多用 `DateTime.Now` (Kind=Local) 或 `DateTime.UtcNow` (Kind=Utc)

```rust
pub fn datetime_to_binary(unix_secs: i64, kind: Kind) -> i64 {
    let ticks = unix_secs * 10_000_000 + 621355968000000000;
    (kind as i64) << 62 | ticks
}
pub fn datetime_from_binary(v: i64) -> (i64 /*unix_secs*/, Kind) {
    let kind = ((v >> 62) & 0x3) as Kind;
    let ticks = v & 0x3FFF_FFFF_FFFF_FFFF;
    ((ticks - 621355968000000000) / 10_000_000, kind)
}
```

## 5. 内嵌数据结构

多个包内嵌结构（`UserItem`、`ClientMapInfo`、`UserInfo`、`QuestInfo`...），由所在包直接读写字段（无统一方法），移植时**逐字段按包内顺序搬运**。字段顺序错一位即全部错乱。

## 6. 移植校验方法

1. 对每个包写 Rust 回环测试（write→read 相等）
2. 与 C# 定义逐字段比对（字段名/顺序/类型）
3. 抽检二进制: C# 序列化样例输出 vs Rust 输出逐字节比对（需 dotnet 环境，见 CI 备注）

## 7. 包 ID

- `ClientPacketId`: 0..=152（对应 `ClientPacketIds` 枚举顺序）
- `ServerPacketId`: 0..=278（对应 `ServerPacketIds` 枚举顺序）
- ID 文件由 `scripts/gen_ids.sh` 从 `Shared/Enums.cs` 生成，**不要手改**，与 C# 顺序必须一致