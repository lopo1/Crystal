extends RefCounted
class_name CrystalBinary
## Crystal 协议二进制编解码（GDScript 版）
## 与 Rust `crystal-protocol` / C# `BinaryReader`/`BinaryWriter` 逐字节兼容（小端序）。
## 兼容规则见 docs/PROTOCOL.md


## 读取器: 小端序游标读取（对应 C# BinaryReader）
class Reader:
	var data: PackedByteArray
	var pos: int = 0

	func _init(bytes: PackedByteArray) -> void:
		data = bytes

	func remaining() -> int:
		return data.size() - pos

	func _take(n: int) -> PackedByteArray:
		assert(pos + n <= data.size(), "UnexpectedEof need=%d have=%d" % [n, remaining()])
		var out := data.slice(pos, pos + n)
		pos += n
		return out

	func read_u8() -> int:
		return _take(1)[0]

	func read_i8() -> int:
		var v := _take(1)[0]
		return v - 256 if v >= 128 else v

	func read_bool() -> bool:
		return read_u8() != 0

	func read_u16() -> int:
		var b := _take(2)
		return b[0] | (b[1] << 8)

	func read_i16() -> int:
		var v := read_u16()
		return v - 65536 if v >= 32768 else v

	func read_u32() -> int:
		var b := _take(4)
		return b[0] | (b[1] << 8) | (b[2] << 16) | (b[3] << 24)

	func read_i32() -> int:
		var v := read_u32()
		return v - 4294967296 if v >= 2147483648 else v

	func read_u64() -> int:
		var b := _take(8)
		var v: int = 0
		for i in range(8):
			v |= b[i] << (i * 8)
		return v

	func read_i64() -> int:
		# GDScript int 为 64 位: 直接按位还原
		var v := read_u64()
		return v - 18446744073709551616 if v >= 9223372036854775808 else v

	func read_f32() -> float:
		return _take(4).decode_float(0)

	func read_f64() -> float:
		return _take(8).decode_double(0)

	## .NET WriteString: 7-bit 编码长度 + UTF-8
	func read_string() -> String:
		var len := read_7bit()
		var bytes := _take(len)
		return bytes.get_string_from_utf8()

	## 7-bit 编码整数（LEB128 风格）
	func read_7bit() -> int:
		var result: int = 0
		var shift: int = 0
		while true:
			var b := read_u8()
			result |= (b & 0x7f) << shift
			shift += 7
			if b & 0x80 == 0:
				break
		return result


## 写入器（对应 C# BinaryWriter）
class Writer:
	var data := PackedByteArray()

	func write_u8(v: int) -> void:
		data.append(v & 0xff)

	func write_i8(v: int) -> void:
		data.append(v & 0xff)

	func write_bool(v: bool) -> void:
		data.append(1 if v else 0)

	func write_u16(v: int) -> void:
		var b := PackedByteArray()
		b.resize(2)
		b.encode_u16(0, v & 0xffff)
		data.append_array(b)

	func write_i16(v: int) -> void:
		write_u16(v & 0xffff)

	func write_u32(v: int) -> void:
		var b := PackedByteArray()
		b.resize(4)
		b.encode_u32(0, v & 0xffffffff)
		data.append_array(b)

	func write_i32(v: int) -> void:
		write_u32(v & 0xffffffff)

	func write_u64(v: int) -> void:
		var b := PackedByteArray()
		b.resize(8)
		b.encode_u64(0, v & 0xffffffffffffffff)
		data.append_array(b)

	func write_i64(v: int) -> void:
		write_u64(v & 0xffffffffffffffff)

	func write_f32(v: float) -> void:
		var b := PackedByteArray()
		b.resize(4)
		b.encode_float(0, v)
		data.append_array(b)

	func write_f64(v: float) -> void:
		var b := PackedByteArray()
		b.resize(8)
		b.encode_double(0, v)
		data.append_array(b)

	func write_string(s: String) -> void:
		var bytes := s.to_utf8_buffer()
		write_7bit(bytes.size())
		data.append_array(bytes)

	func write_7bit(v: int) -> void:
		var val := v
		while val >= 0x80:
			write_u8((val & 0x7f) | 0x80)
			val >>= 7
		write_u8(val)


## 帧编码: [u16 LE 总长][i16 LE 包ID][载荷]; 总长 = 4 + 载荷长
static func encode_frame(id: int, payload: PackedByteArray) -> PackedByteArray:
	var w := Writer.new()
	w.write_u16(4 + payload.size())
	w.write_i16(id)
	w.data.append_array(payload)
	return w.data


## 包编解码基类（数据字段形式，GDScript 方法不可赋值）:
## 子类设置 packet_id 与 write_fn；encode() 产出完整帧
class Packet:
	var packet_id: int = -1
	var write_fn: Callable = Callable()

	func encode() -> PackedByteArray:
		var w := Writer.new()
		if write_fn.is_valid():
			write_fn.call(w)
		return encode_frame(packet_id, w.data)


## ARGB 颜色辅助
static func argb_to_i32(a: int, r: int, g: int, b: int) -> int:
	return (a << 24) | (r << 16) | (g << 8) | b


## DateTime.ToBinary 复刻（unix 秒 -> .NET i64，Kind=Utc）
static func datetime_binary(unix_secs: int) -> int:
	var ticks: int = unix_secs * 10000000 + 621355968000000000
	return (1 << 62) | ticks