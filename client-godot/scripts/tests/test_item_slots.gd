extends SceneTree
## UserInformation 物品槽布尔方向必须与 C#/Rust 一致：true=有物品。
## Run: Godot --path client-godot --headless --script res://scripts/tests/test_item_slots.gd

const Packets := preload("res://scripts/net/crystal_packets.gd")
const CrystalBinary := preload("res://scripts/net/crystal_binary.gd")
const Writer := CrystalBinary.Writer
const Reader := CrystalBinary.Reader


func _initialize() -> void:
	var errors: PackedStringArray = []
	_test_present_slot_direction(errors)
	_test_user_information_like_login(errors)
	_test_nested_user_item_slots_empty_is_true(errors)
	if errors.is_empty():
		print("PASS: item slot boolean direction")
		quit(0)
		return
	for err in errors:
		push_error(err)
	quit(1)


func _sample_item(uid: int, index: int) -> Dictionary:
	return {
		"unique_id": uid,
		"item_index": index,
		"current_dura": 100,
		"max_dura": 100,
		"count": 1,
		"soul_bound_id": -1,
		"identified": false,
		"cursed": false,
		"slots": [],
		"gem_count": 0,
		"added_stats": [],
		"awake": [0],
		"refined_value": 0,
		"refine_added": 0,
		"refine_success_chance": 0,
		"wedding_ring": -1,
		"expire_info": null,
		"rental_information": null,
		"is_shop_item": false,
		"sealed_info": null,
		"gm_made": false,
	}


func _write_server_item_slots(w: Writer, slots: Array) -> void:
	w.write_bool(true)
	w.write_i32(slots.size())
	for slot in slots:
		if slot == null:
			w.write_bool(false)
		else:
			w.write_bool(true)
			Packets.write_user_item(w, slot)


func _test_present_slot_direction(errors: PackedStringArray) -> void:
	var w := Writer.new()
	_write_server_item_slots(w, [_sample_item(9, 5), null])
	var r := Reader.new(w.data)
	var slots: Array = Packets._read_item_slots(r, func(rr): return Packets.read_user_item(rr))
	if r.remaining() != 0:
		errors.append("slot payload leftover %d bytes" % r.remaining())
	if slots.size() != 2:
		errors.append("expected 2 slots, got %d" % slots.size())
		return
	if slots[0] == null or int(slots[0].get("unique_id", 0)) != 9:
		errors.append("slot 0 should contain unique_id 9 (true=has item), got %s" % str(slots[0]))
	if slots[1] != null:
		errors.append("slot 1 should be empty, got %s" % str(slots[1]))


func _test_user_information_like_login(errors: PackedStringArray) -> void:
	var w := Writer.new()
	w.write_u32(1)
	w.write_u32(1)
	w.write_string("demo")
	w.write_string("")
	w.write_string("")
	w.write_i32(0)
	w.write_u8(0)
	w.write_u8(0)
	w.write_u16(1)
	w.write_i32(400)
	w.write_i32(400)
	w.write_u8(0)
	w.write_u8(0)
	w.write_i32(100)
	w.write_i32(50)
	w.write_i64(0)
	w.write_i64(10)
	w.write_u16(0)
	w.write_bool(false)
	w.write_u8(0)
	var inventory: Array = []
	inventory.resize(40)
	inventory[0] = _sample_item(11, 1)
	inventory[1] = _sample_item(12, 3)
	_write_server_item_slots(w, inventory)
	var equipment: Array = []
	equipment.resize(14)
	_write_server_item_slots(w, equipment)
	w.write_bool(false)
	w.write_u32(0)
	w.write_u32(0)
	w.write_bool(false)
	w.write_bool(false)
	w.write_bool(false)
	w.write_i64(0)
	w.write_i64(0)
	w.write_i32(0)
	w.write_i32(0)
	w.write_u8(0)
	w.write_bool(false)
	w.write_bool(false)
	w.write_bool(false)
	var packet: Dictionary = Packets.decode_server_packet(Packets.S_USER_INFORMATION, w.data)
	var ui: Dictionary = packet.get("data", {})
	var inv: Array = ui.get("inventory", [])
	if inv.size() != 40:
		errors.append("UserInformation inventory size %d, expected 40" % inv.size())
		return
	if inv[0] == null or int(inv[0].get("item_index", 0)) != 1:
		errors.append("starter sword in slot 0 was lost: %s" % str(inv[0]))
	if inv[1] == null or int(inv[1].get("count", 0)) != 1:
		errors.append("starter item in slot 1 was lost: %s" % str(inv[1]))
	if inv[2] != null:
		errors.append("empty inventory slot 2 should be null")


func _test_nested_user_item_slots_empty_is_true(errors: PackedStringArray) -> void:
	var inner := _sample_item(2, 8)
	var outer := _sample_item(1, 7)
	outer["slots"] = [null, inner]
	var w := Writer.new()
	Packets.write_user_item(w, outer)
	var decoded: Dictionary = Packets.read_user_item(Reader.new(w.data))
	var nested: Array = decoded.get("slots", [])
	if nested.size() != 2:
		errors.append("nested slots size %d" % nested.size())
		return
	if nested[0] != null:
		errors.append("UserItem inner slot true=empty was inverted")
	if nested[1] == null or int(nested[1].get("unique_id", 0)) != 2:
		errors.append("UserItem nested occupied slot lost")
