extends RefCounted
class_name CrystalPackets
## 客户端-服务器数据包（GDScript 版）
## 与 Rust crystal-protocol / C# Shared 逐字段一致。包 ID 见 ids.rs（脚本生成）。

const CrystalBinary = preload("res://scripts/net/crystal_binary.gd")
const Reader = CrystalBinary.Reader
const Writer = CrystalBinary.Writer

# ---------------------------------------------------------------------------
# 通用读取辅助
# ---------------------------------------------------------------------------

static func _read_items(r: Reader, item_reader: Callable, count: int) -> Array:
	var out := []
	for i in range(count):
		out.append(item_reader.call(r))
	return out

static func _read_item_slots(r: Reader, item_reader: Callable) -> Array:
	## 与 C# 一致: bool 是否存在数组; 若存在 int32 长度 + 逐槽 bool(存在) + 数据
	if not r.read_bool():
		return []
	var len := r.read_i32()
	var slots := []
	slots.resize(len)
	for i in range(len):
		if r.read_bool():
			slots[i] = null
		else:
			slots[i] = item_reader.call(r)
	return slots

static func _write_item_slots(w: Writer, slots: Array, item_writer: Callable) -> void:
	w.write_bool(slots.size() > 0)
	if slots.size() > 0:
		w.write_i32(slots.size())
		for s in slots:
			if s == null:
				w.write_bool(true)
			else:
				w.write_bool(false)
				item_writer.call(w, s)

# ---------------------------------------------------------------------------
# 内嵌类型
# ---------------------------------------------------------------------------

static func read_point(r: Reader) -> Vector2i:
	return Vector2i(r.read_i32(), r.read_i32())

static func write_point(w: Writer, p: Vector2i) -> void:
	w.write_i32(p.x)
	w.write_i32(p.y)

static func read_datetime(r: Reader) -> int:
	return r.read_i64()

static func write_datetime(w: Writer, v: int) -> void:
	w.write_i64(v)

# --- UserItem（最新线格式，对应 Rust types.rs UserItem） ---
static func read_user_item(r: Reader) -> Dictionary:
	var item := {}
	item["unique_id"] = r.read_u64()
	item["item_index"] = r.read_i32()
	item["current_dura"] = r.read_u16()
	item["max_dura"] = r.read_u16()
	item["count"] = r.read_u16()
	item["soul_bound_id"] = r.read_i32()
	var flags := r.read_u8()
	item["identified"] = (flags & 0x01) == 0x01
	item["cursed"] = (flags & 0x02) == 0x02
	var slot_count := r.read_i32()
	var slots := []
	slots.resize(slot_count)
	for i in range(slot_count):
		if r.read_bool():
			slots[i] = null
		else:
			slots[i] = read_user_item(r)
	item["slots"] = slots
	item["gem_count"] = r.read_u16()
	# added_stats: i32 count + (u8 stat, i32 value)*
	var stats := []
	var scount := r.read_i32()
	for i in range(scount):
		stats.append([r.read_u8(), r.read_i32()])
	item["added_stats"] = stats
	# awake: u8 type + i32 count + u8*
	var awake_type := r.read_u8()
	var awcount := r.read_i32()
	var awake := [awake_type]
	for i in range(awcount):
		awake.append(r.read_u8())
	item["awake"] = awake
	item["refined_value"] = r.read_u8()
	item["refine_added"] = r.read_u8()
	item["refine_success_chance"] = r.read_i32()
	item["wedding_ring"] = r.read_i32()
	item["expire_info"] = r.read_i64() if r.read_bool() else null
	var ri = null
	if r.read_bool():
		ri = {
			"owner_name": r.read_string(),
			"binding_flags": r.read_i16(),
			"expiry_date": r.read_i64(),
			"rental_locked": r.read_bool(),
		}
	item["rental_information"] = ri
	item["is_shop_item"] = r.read_bool()
	var si = null
	if r.read_bool():
		si = {"expiry_date": r.read_i64(), "next_seal_date": r.read_i64()}
	item["sealed_info"] = si
	item["gm_made"] = r.read_bool()
	return item

static func write_user_item(w: Writer, item: Dictionary) -> void:
	w.write_u64(item.get("unique_id", 0))
	w.write_i32(item.get("item_index", 0))
	w.write_u16(item.get("current_dura", 0))
	w.write_u16(item.get("max_dura", 0))
	w.write_u16(item.get("count", 1))
	w.write_i32(item.get("soul_bound_id", -1))
	var flags := 0
	if item.get("identified", false):
		flags |= 0x01
	if item.get("cursed", false):
		flags |= 0x02
	w.write_u8(flags)
	var slots: Array = item.get("slots", [])
	w.write_i32(slots.size())
	for s in slots:
		if s == null:
			w.write_bool(true)
		else:
			w.write_bool(false)
			write_user_item(w, s)
	w.write_u16(item.get("gem_count", 0))
	var stats: Array = item.get("added_stats", [])
	w.write_i32(stats.size())
	for kv in stats:
		w.write_u8(kv[0])
		w.write_i32(kv[1])
	var awake: Array = item.get("awake", [0])
	w.write_u8(awake[0])
	w.write_i32(max(awake.size() - 1, 0))
	for i in range(1, awake.size()):
		w.write_u8(awake[i])
	w.write_u8(item.get("refined_value", 0))
	w.write_u8(item.get("refine_added", 0))
	w.write_i32(item.get("refine_success_chance", 0))
	w.write_i32(item.get("wedding_ring", -1))
	var ei: Variant = item.get("expire_info")
	w.write_bool(ei != null)
	if ei != null:
		w.write_i64(ei)
	var ri: Variant = item.get("rental_information")
	w.write_bool(ri != null)
	if ri != null:
		w.write_string(ri.get("owner_name", ""))
		w.write_i16(ri.get("binding_flags", 0))
		w.write_i64(ri.get("expiry_date", 0))
		w.write_bool(ri.get("rental_locked", false))
	w.write_bool(item.get("is_shop_item", false))
	var si: Variant = item.get("sealed_info")
	w.write_bool(si != null)
	if si != null:
		w.write_i64(si.get("expiry_date", 0))
		w.write_i64(si.get("next_seal_date", 0))
	w.write_bool(item.get("gm_made", false))

# --- SelectInfo ---
static func read_select_info(r: Reader) -> Dictionary:
	return {
		"index": r.read_i32(),
		"name": r.read_string(),
		"level": r.read_u16(),
		"class": r.read_u8(),
		"gender": r.read_u8(),
		"last_access": r.read_i64(),
	}

static func write_select_info(w: Writer, info: Dictionary) -> void:
	w.write_i32(info.get("index", 0))
	w.write_string(info.get("name", ""))
	w.write_u16(info.get("level", 1))
	w.write_u8(info.get("class", 0))
	w.write_u8(info.get("gender", 0))
	w.write_i64(info.get("last_access", 0))

# --- ChatItem ---
static func read_chat_item(r: Reader) -> Dictionary:
	return {
		"unique_id": r.read_u64(),
		"title": r.read_string(),
		"grid": r.read_u8(),
	}

static func write_chat_item(w: Writer, item: Dictionary) -> void:
	w.write_u64(item.get("unique_id", 0))
	w.write_string(item.get("title", ""))
	w.write_u8(item.get("grid", 0))

# --- ClientMagic ---
static func read_client_magic(r: Reader) -> Dictionary:
	var m := {}
	m["name"] = r.read_string()
	m["spell"] = r.read_u8()
	m["base_cost"] = r.read_u8()
	m["level_cost"] = r.read_u8()
	m["icon"] = r.read_u8()
	m["level1"] = r.read_u8()
	m["level2"] = r.read_u8()
	m["level3"] = r.read_u8()
	m["need1"] = r.read_u16()
	m["need2"] = r.read_u16()
	m["need3"] = r.read_u16()
	m["level"] = r.read_u8()
	m["key"] = r.read_u8()
	m["experience"] = r.read_u16()
	m["delay"] = r.read_i64()
	m["range"] = r.read_u8()
	m["cast_time"] = r.read_i64()
	return m

static func write_client_magic(w: Writer, m: Dictionary) -> void:
	w.write_string(m.get("name", ""))
	w.write_u8(m.get("spell", 0))
	w.write_u8(m.get("base_cost", 0))
	w.write_u8(m.get("level_cost", 0))
	w.write_u8(m.get("icon", 0))
	w.write_u8(m.get("level1", 0))
	w.write_u8(m.get("level2", 0))
	w.write_u8(m.get("level3", 0))
	w.write_u16(m.get("need1", 0))
	w.write_u16(m.get("need2", 0))
	w.write_u16(m.get("need3", 0))
	w.write_u8(m.get("level", 0))
	w.write_u8(m.get("key", 0))
	w.write_u16(m.get("experience", 0))
	w.write_i64(m.get("delay", 0))
	w.write_u8(m.get("range", 0))
	w.write_i64(m.get("cast_time", 0))

# --- ClientMapInfo ---
static func read_client_map_info(r: Reader) -> Dictionary:
	var info := {
		"title": r.read_string(),
		"width": r.read_i32(),
		"height": r.read_i32(),
		"big_map": r.read_i32(),
		"movements": [],
		"npcs": [],
	}
	var mcount := r.read_i32()
	for i in range(mcount):
		var mv := {}
		mv["destination"] = r.read_i32()
		mv["title"] = r.read_string()
		mv["location"] = read_point(r)
		mv["icon"] = r.read_i32()
		info["movements"].append(mv)
	var ncount := r.read_i32()
	for i in range(ncount):
		var npc := {}
		npc["index"] = r.read_i32()
		npc["file_name"] = r.read_string()
		npc["name"] = r.read_string()
		npc["map_index"] = r.read_i32()
		npc["location"] = read_point(r)
		npc["image"] = r.read_u16()
		npc["rate"] = r.read_u16()
		npc["show_on_big_map"] = r.read_bool()
		npc["big_map_icon"] = r.read_i32()
		npc["object_id"] = r.read_u32()
		npc["icon"] = r.read_i32()
		npc["can_teleport_to"] = r.read_bool()
		info["npcs"].append(npc)
	return info

static func write_client_map_info(w: Writer, info: Dictionary) -> void:
	w.write_string(info.get("title", ""))
	w.write_i32(info.get("width", 0))
	w.write_i32(info.get("height", 0))
	w.write_i32(info.get("big_map", 0))
	var movements: Array = info.get("movements", [])
	w.write_i32(movements.size())
	for mv in movements:
		w.write_i32(mv.get("destination", 0))
		w.write_string(mv.get("title", ""))
		write_point(w, mv.get("location", Vector2i.ZERO))
		w.write_i32(mv.get("icon", 0))
	var npcs: Array = info.get("npcs", [])
	w.write_i32(npcs.size())
	for npc in npcs:
		w.write_i32(npc.get("index", 0))
		w.write_string(npc.get("file_name", ""))
		w.write_string(npc.get("name", ""))
		w.write_i32(npc.get("map_index", 0))
		write_point(w, npc.get("location", Vector2i.ZERO))
		w.write_u16(npc.get("image", 0))
		w.write_u16(npc.get("rate", 0))
		w.write_bool(npc.get("show_on_big_map", false))
		w.write_i32(npc.get("big_map_icon", 0))
		w.write_u32(npc.get("object_id", 0))
		w.write_i32(npc.get("icon", 0))
		w.write_bool(npc.get("can_teleport_to", false))

# --- 灵兽（ClientIntelligentCreature，对应 Rust types.rs） ---
static func read_client_creature(r: Reader) -> Dictionary:
	var c := {}
	c["pet_type"] = r.read_u8()
	c["icon"] = r.read_i32()
	c["custom_name"] = r.read_string()
	c["fullness"] = r.read_i32()
	c["slot_index"] = r.read_i32()
	c["expire"] = r.read_i64()
	c["blackstone_time"] = r.read_i64()
	c["pet_mode"] = r.read_u8()
	# IntelligentCreatureRules: 8 字段
	c["rules"] = {
		"minimal_fullness": r.read_i32(),
		"mouse_pickup_enabled": r.read_bool(),
		"mouse_pickup_range": r.read_i32(),
		"auto_pickup_enabled": r.read_bool(),
		"auto_pickup_range": r.read_i32(),
		"semi_auto_pickup_enabled": r.read_bool(),
		"semi_auto_pickup_range": r.read_i32(),
		"can_produce_black_stone": r.read_bool(),
	}
	# IntelligentCreatureItemFilter: 9 bool
	c["filter"] = {
		"pickup_all": r.read_bool(),
		"pickup_gold": r.read_bool(),
		"pickup_weapons": r.read_bool(),
		"pickup_armours": r.read_bool(),
		"pickup_helmets": r.read_bool(),
		"pickup_boots": r.read_bool(),
		"pickup_belts": r.read_bool(),
		"pickup_accessories": r.read_bool(),
		"pickup_others": r.read_bool(),
	}
	c["pickup_grade"] = r.read_u8()
	c["maintain_food_time"] = r.read_i64()
	return c

static func write_client_creature(w: Writer, c: Dictionary) -> void:
	w.write_u8(c.get("pet_type", 99))
	w.write_i32(c.get("icon", 0))
	w.write_string(c.get("custom_name", ""))
	w.write_i32(c.get("fullness", 0))
	w.write_i32(c.get("slot_index", 0))
	w.write_i64(c.get("expire", 0))
	w.write_i64(c.get("blackstone_time", 0))
	w.write_u8(c.get("pet_mode", 0))
	var rules: Dictionary = c.get("rules", {})
	w.write_i32(rules.get("minimal_fullness", 1))
	w.write_bool(rules.get("mouse_pickup_enabled", false))
	w.write_i32(rules.get("mouse_pickup_range", 0))
	w.write_bool(rules.get("auto_pickup_enabled", false))
	w.write_i32(rules.get("auto_pickup_range", 0))
	w.write_bool(rules.get("semi_auto_pickup_enabled", false))
	w.write_i32(rules.get("semi_auto_pickup_range", 0))
	w.write_bool(rules.get("can_produce_black_stone", false))
	var filter: Dictionary = c.get("filter", {})
	w.write_bool(filter.get("pickup_all", true))
	w.write_bool(filter.get("pickup_gold", false))
	w.write_bool(filter.get("pickup_weapons", false))
	w.write_bool(filter.get("pickup_armours", false))
	w.write_bool(filter.get("pickup_helmets", false))
	w.write_bool(filter.get("pickup_boots", false))
	w.write_bool(filter.get("pickup_belts", false))
	w.write_bool(filter.get("pickup_accessories", false))
	w.write_bool(filter.get("pickup_others", false))
	w.write_u8(c.get("pickup_grade", 0))
	w.write_i64(c.get("maintain_food_time", 0))

# --- 客户端→服务器 包 ---

static func c_client_version(hash: PackedByteArray) -> CrystalBinary.Packet:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 0
	p.write_fn = func(w: Writer) -> void:
		w.write_i32(hash.size())
		w.data.append_array(hash)
	return p

static func c_disconnect() -> CrystalBinary.Packet:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 1
	return p

static func c_keep_alive(time: int) -> CrystalBinary.Packet:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 2
	p.write_fn = func(w: Writer) -> void: w.write_i64(time)
	return p

static func c_login(account: String, password: String) -> CrystalBinary.Packet:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 5
	p.write_fn = func(w: Writer) -> void:
		w.write_string(account)
		w.write_string(password)
	return p

static func c_new_account(account: String, password: String, email: String, name: String) -> CrystalBinary.Packet:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 3
	p.write_fn = func(w: Writer) -> void:
		w.write_string(account)
		w.write_string(password)
		w.write_i64(CrystalBinary.datetime_binary(0)) # BirthDate
		w.write_string(name)
		w.write_string("") # secret question
		w.write_string("") # secret answer
		w.write_string(email)
	return p

static func c_new_character(char_name: String, gender: int, class_id: int) -> CrystalBinary.Packet:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 6
	p.write_fn = func(w: Writer) -> void:
		w.write_string(char_name)
		w.write_u8(gender)
		w.write_u8(class_id)
	return p

static func c_delete_character(index: int) -> CrystalBinary.Packet:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 7
	p.write_fn = func(w: Writer) -> void: w.write_i32(index)
	return p

static func c_start_game(index: int) -> CrystalBinary.Packet:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 8
	p.write_fn = func(w: Writer) -> void: w.write_i32(index)
	return p

static func c_logout() -> CrystalBinary.Packet:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 9
	return p

static func c_turn(direction: int) -> CrystalBinary.Packet:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 10
	p.write_fn = func(w: Writer) -> void: w.write_u8(direction)
	return p

static func c_walk(direction: int) -> CrystalBinary.Packet:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 11
	p.write_fn = func(w: Writer) -> void: w.write_u8(direction)
	return p

static func c_run(direction: int) -> CrystalBinary.Packet:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 12
	p.write_fn = func(w: Writer) -> void: w.write_u8(direction)
	return p

static func c_chat(message: String) -> CrystalBinary.Packet:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 13
	p.write_fn = func(w: Writer) -> void:
		w.write_string(message)
		w.write_i32(0) # linked items
	return p

# --- Web3 钱包登录扩展（自定义 ID 200+，与 Rust client/web3.rs 一致） ---

static func c_web3_challenge_request(address: String) -> CrystalBinary.Packet:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 200
	p.write_fn = func(w: Writer) -> void:
		w.write_string(address)
	return p

static func c_web3_login(address: String, challenge: String, signature: PackedByteArray) -> CrystalBinary.Packet:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 201
	p.write_fn = func(w: Writer) -> void:
		w.write_string(address)
		w.write_string(challenge)
		w.write_i32(signature.size())
		w.data.append_array(signature)
	return p

# ---------------------------------------------------------------------------
# 服务器 → 客户端 包解码（按 ID 分发）
# ---------------------------------------------------------------------------

const S_CONNECTED := 0
const S_CLIENT_VERSION := 1
const S_DISCONNECT := 2
const S_KEEPALIVE := 3
const S_NEW_ACCOUNT := 4
const S_LOGIN := 7
const S_LOGIN_SUCCESS := 9
const S_NEW_CHARACTER := 10
const S_NEW_CHARACTER_SUCCESS := 11
const S_DELETE_CHARACTER := 12
const S_DELETE_CHARACTER_SUCCESS := 13
const S_START_GAME := 14
const S_START_GAME_BANNED := 15
const S_START_GAME_DELAY := 16
const S_MAP_INFORMATION := 17
const S_NEW_MAP_INFO := 18
const S_USER_INFORMATION := 21
const S_USER_LOCATION := 23
const S_OBJECT_PLAYER := 24
const S_OBJECT_REMOVE := 26
const S_OBJECT_TURN := 27
const S_OBJECT_WALK := 28
const S_OBJECT_RUN := 29
const S_CHAT := 30
const S_OBJECT_CHAT := 31
const S_TIME_OF_DAY := 61
# Web3 钱包登录扩展（自定义 ID 300+，与 Rust server/web3.rs 一致）
const S_WEB3_CHALLENGE := 300
const S_WEB3_LOGIN_RESULT := 301


## 解码服务器包: 返回 { "id": int, "data": Dictionary }；未知包返回 { "id": id, "data": {} }
static func decode_server_packet(id: int, payload: PackedByteArray) -> Dictionary:
	var r := Reader.new(payload)
	match id:
		S_CONNECTED:
			return {"id": id, "data": {}}
		S_CLIENT_VERSION:
			return {"id": id, "data": {"result": r.read_u8()}}
		S_DISCONNECT:
			return {"id": id, "data": {"reason": r.read_u8()}}
		S_KEEPALIVE:
			return {"id": id, "data": {"time": r.read_i64()}}
		S_NEW_ACCOUNT:
			return {"id": id, "data": {"result": r.read_u8()}}
		S_LOGIN:
			return {"id": id, "data": {"result": r.read_u8()}}
		S_LOGIN_SUCCESS:
			var chars := []
			var count := r.read_i32()
			for i in range(count):
				chars.append(read_select_info(r))
			return {"id": id, "data": {"characters": chars}}
		S_NEW_CHARACTER:
			return {"id": id, "data": {"result": r.read_u8()}}
		S_NEW_CHARACTER_SUCCESS:
			return {"id": id, "data": {"char_info": read_select_info(r)}}
		S_DELETE_CHARACTER:
			return {"id": id, "data": {"result": r.read_u8()}}
		S_DELETE_CHARACTER_SUCCESS:
			return {"id": id, "data": {"character_index": r.read_i32()}}
		S_START_GAME:
			return {"id": id, "data": {"result": r.read_u8(), "resolution": r.read_i32()}}
		S_START_GAME_BANNED:
			return {"id": id, "data": {"reason": r.read_string(), "expiry_date": r.read_i64()}}
		S_START_GAME_DELAY:
			return {"id": id, "data": {"milliseconds": r.read_i64()}}
		S_MAP_INFORMATION:
			return {
				"id": id,
				"data": {
					"map_index": r.read_i32(),
					"file_name": r.read_string(),
					"title": r.read_string(),
					"mini_map": r.read_u16(),
					"big_map": r.read_u16(),
					"lights": r.read_u8(),
					"bools": r.read_u8(),
					"map_dark_light": r.read_u8(),
					"music": r.read_u16(),
					"weather_particles": r.read_u16(),
				},
			}
		S_NEW_MAP_INFO:
			return {"id": id, "data": {"map_index": r.read_i32(), "info": read_client_map_info(r)}}
		S_USER_INFORMATION:
			var ui := {}
			ui["object_id"] = r.read_u32()
			ui["real_id"] = r.read_u32()
			ui["name"] = r.read_string()
			ui["guild_name"] = r.read_string()
			ui["guild_rank"] = r.read_string()
			ui["name_colour"] = r.read_i32()
			ui["class"] = r.read_u8()
			ui["gender"] = r.read_u8()
			ui["level"] = r.read_u16()
			ui["location"] = read_point(r)
			ui["direction"] = r.read_u8()
			ui["hair"] = r.read_u8()
			ui["hp"] = r.read_i32()
			ui["mp"] = r.read_i32()
			ui["experience"] = r.read_i64()
			ui["max_experience"] = r.read_i64()
			ui["level_effects"] = r.read_u16()
			ui["has_hero"] = r.read_bool()
			ui["hero_behaviour"] = r.read_u8()
			ui["inventory"] = _read_item_slots(r, func(rr): return read_user_item(rr))
			ui["equipment"] = _read_item_slots(r, func(rr): return read_user_item(rr))
			ui["quest_inventory"] = _read_item_slots(r, func(rr): return read_user_item(rr))
			ui["gold"] = r.read_u32()
			ui["credit"] = r.read_u32()
			ui["has_expanded_storage"] = r.read_bool()
			ui["has_storage_password"] = r.read_bool()
			ui["require_storage_password"] = r.read_bool()
			ui["storage_password_last_set"] = r.read_i64()
			ui["expanded_storage_expiry_time"] = r.read_i64()
			var magics := []
			var mcount := r.read_i32()
			for i in range(mcount):
				magics.append(read_client_magic(r))
			ui["magics"] = magics
			var creatures := []
			var ccount := r.read_i32()
			for i in range(ccount):
				creatures.append(read_client_creature(r))
			ui["intelligent_creatures"] = creatures
			ui["summoned_creature_type"] = r.read_u8()
			ui["creature_summoned"] = r.read_bool()
			ui["allow_observe"] = r.read_bool()
			ui["observer"] = r.read_bool()
			return {"id": id, "data": ui}
		S_USER_LOCATION:
			return {"id": id, "data": {"location": read_point(r), "direction": r.read_u8()}}
		S_OBJECT_PLAYER:
			return {"id": id, "data": read_object_player(r)}
		S_OBJECT_REMOVE:
			return {"id": id, "data": {"object_id": r.read_u32()}}
		S_OBJECT_TURN, S_OBJECT_WALK, S_OBJECT_RUN:
			return {
				"id": id,
				"data": {
					"object_id": r.read_u32(),
					"location": read_point(r),
					"direction": r.read_u8(),
				},
			}
		S_CHAT:
			return {"id": id, "data": {"message": r.read_string(), "type": r.read_u8()}}
		S_OBJECT_CHAT:
			return {"id": id, "data": {"object_id": r.read_u32(), "text": r.read_string(), "type": r.read_u8()}}
		S_TIME_OF_DAY:
			return {"id": id, "data": {"lights": r.read_u8()}}
		S_WEB3_CHALLENGE:
			return {
				"id": id,
				"data": {
					"address": r.read_string(),
					"message": r.read_string(),
					"expires_in": r.read_i64(),
				},
			}
		S_WEB3_LOGIN_RESULT:
			# 布局与 Rust server/web3.rs 一致: result(u8) + int32 角色数 + SelectInfo*
			var wres := r.read_u8()
			var wcc := r.read_i32()
			var wchars := []
			for i in range(wcc):
				wchars.append(read_select_info(r))
			return {"id": id, "data": {"result": wres, "characters": wchars}}
		_:
			return {"id": id, "data": {}}


static func read_object_player(r: Reader) -> Dictionary:
	var op := {}
	op["object_id"] = r.read_u32()
	op["name"] = r.read_string()
	op["guild_name"] = r.read_string()
	op["guild_rank_name"] = r.read_string()
	op["name_colour"] = r.read_i32()
	op["class"] = r.read_u8()
	op["gender"] = r.read_u8()
	op["level"] = r.read_u16()
	op["location"] = read_point(r)
	op["direction"] = r.read_u8()
	op["hair"] = r.read_u8()
	op["light"] = r.read_u8()
	op["weapon"] = r.read_i16()
	op["weapon_effect"] = r.read_i16()
	op["armour"] = r.read_i16()
	op["poison"] = r.read_u16()
	op["dead"] = r.read_bool()
	op["hidden"] = r.read_bool()
	op["effect"] = r.read_u8()
	op["wing_effect"] = r.read_u8()
	op["extra"] = r.read_bool()
	op["mount_type"] = r.read_i16()
	op["riding_mount"] = r.read_bool()
	op["fishing"] = r.read_bool()
	op["transform_type"] = r.read_i16()
	op["element_orb_effect"] = r.read_u32()
	op["element_orb_lvl"] = r.read_u32()
	op["element_orb_max"] = r.read_u32()
	var buffs := []
	var bcount := r.read_i32()
	for i in range(bcount):
		buffs.append(r.read_u8())
	op["buffs"] = buffs
	op["level_effects"] = r.read_u16()
	return op