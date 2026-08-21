extends RefCounted
class_name LegacyLibResources

const CrystalBinary := preload("res://scripts/net/crystal_binary.gd")

const CLASS_WARRIOR := 0
const CLASS_WIZARD := 1
const CLASS_TAOIST := 2
const CLASS_ASSASSIN := 3
const CLASS_ARCHER := 4
const CLASS_WEAPON_COUNT := 100
const DIRECTION_STRIDE := 4
const ACTION_STANDING := "standing"
const ACTION_WALKING := "walking"
const ACTION_RUNNING := "running"
const ACTION_ATTACK1 := "attack1"
const ACTION_ATTACK2 := "attack2"
const ACTION_ATTACK3 := "attack3"
const ACTION_ATTACK_RANGE1 := "attack_range1"
const ACTION_WALKING_BOW := "walking_bow"
const ACTION_RUNNING_BOW := "running_bow"
const ACTION_SPELL := "spell"
const ACTION_STRUCK := "struck"
const ACTION_DIE := "die"
const ACTION_DEAD := "dead"
const MONSTER_GREAT_FOX_SPIRIT := 134
const MONSTER_CAVE_STATUE := 321
const MONSTER_EVIL_MIR := 900
const MONSTER_EVIL_MIR_BODY := 901
const MONSTER_DRAGON_STATUE := 902
const MONSTER_HELL_BOMB_1 := 903
const MONSTER_HELL_BOMB_3 := 905
const MONSTER_CATAPULT := 940
const MONSTER_CANON_TREBUCHET := 944
const MONSTER_SABUK_GATE := 950
const MONSTER_FROZEN_DOOR := 964
const MONSTER_BABY_PIG := 10000
const MONSTER_MEDICAL_RAT := 10014

var _data_root := ""
var _scanned := false
var _library_cache := {}
var _frame_cache := {}
var _composite_cache := {}

func data_root() -> String:
	_ensure_data_root()
	return _data_root

func has_data_root() -> bool:
	_ensure_data_root()
	return _data_root != ""

func clear_caches() -> void:
	_library_cache.clear()
	_frame_cache.clear()
	_composite_cache.clear()

func refresh() -> void:
	_scanned = false
	_data_root = ""
	clear_caches()
	_ensure_data_root()

func gold_frame_for_amount(amount: int) -> int:
	if amount < 100:
		return 112
	if amount < 200:
		return 113
	if amount < 500:
		return 114
	if amount < 1000:
		return 115
	return 116

func get_player_texture(appearance: Dictionary, direction: int) -> Dictionary:
	return get_player_visual(appearance, direction, ACTION_STANDING, 0.0, 0)

func get_player_visual(appearance: Dictionary, direction: int, action: String, elapsed_ms: float, action_variant: int = 0) -> Dictionary:
	_ensure_data_root()
	if _data_root == "":
		return {}

	var player_class: int = appearance.get("class", CLASS_WARRIOR)
	var gender: int = appearance.get("gender", 0)
	var hair: int = appearance.get("hair", 0)
	var weapon: int = appearance.get("weapon", -1)
	var weapon_effect: int = appearance.get("weapon_effect", -1)
	var wing_effect: int = appearance.get("wing_effect", 0)
	var armour: int = appearance.get("armour", 0)
	var dir8: int = _normalize_direction(direction)
	var class_weapon := _has_class_weapon(player_class, weapon)
	var action_key := _player_action_key(player_class, class_weapon, action)
	var frame := _player_frame(player_class, action_key)
	if frame.is_empty():
		frame = _player_frame(player_class, ACTION_STANDING)
	var frame_index := _frame_index(frame, elapsed_ms, _is_looping_action(action_key))
	var effect_index := _effect_frame_index(frame, elapsed_ms, _is_looping_action(action_key))
	var body_frame := _frame_draw_index(frame, dir8, frame_index)
	var wing_frame := _effect_draw_index(frame, dir8, effect_index)
	var cache_key := "player:%d:%d:%d:%d:%d:%d:%d:%s:%d:%d" % [
		player_class, gender, hair, weapon, weapon_effect, wing_effect, armour, action_key, body_frame, wing_frame
	]
	if _composite_cache.has(cache_key):
		return _composite_cache[cache_key]

	var layers: Array = []
	var female := gender != 0
	var body_prefix := "CArmour"
	var hair_prefix := "CHair"
	var body_offset := 808 if female else 0
	var hair_offset := body_offset
	var weapon_offset := 416 if female else 0
	var wing_prefix := "CHumEffect"
	var wing_offset := 840 if female else 0
	match player_class:
		CLASS_ASSASSIN:
			var assassin_alt := class_weapon or weapon < 0
			if assassin_alt:
				body_prefix = "AArmour"
				hair_prefix = "AHair"
				body_offset = 512 if female else 0
				hair_offset = body_offset
				weapon_offset = 512 if female else 0
				wing_prefix = "AHumEffect"
				wing_offset = 544 if female else 0
		CLASS_ARCHER:
			var archer_alt := class_weapon and action_key in [ACTION_WALKING_BOW, ACTION_RUNNING_BOW, ACTION_ATTACK_RANGE1]
			if archer_alt:
				body_prefix = "ARArmour"
				hair_prefix = "ARHair"
				body_offset = 352 if female else 0
				hair_offset = body_offset
				weapon_offset = 352 if female else 0
				wing_prefix = "ARHumEffect"
				wing_offset = 352 if female else 0
		_:
			body_prefix = "CArmour"
			hair_prefix = "CHair"
			body_offset = 808 if female else 0
			hair_offset = body_offset
			weapon_offset = 416 if female else 0
			wing_prefix = "CHumEffect"
			wing_offset = 840 if female else 0

	_append_layer(layers, _get_indexed_frame(body_prefix, "%02d" % max(armour, 0), body_frame + body_offset))
	_append_layer(layers, _get_indexed_frame(hair_prefix, "%02d" % max(hair, 0), body_frame + hair_offset))

	if weapon >= 0:
		match player_class:
			CLASS_ASSASSIN:
				if class_weapon:
					var assassin_index := weapon - CLASS_WEAPON_COUNT
					_append_layer(layers, _get_indexed_frame("AWeapon", "%02d L" % max(assassin_index, 0), body_frame + weapon_offset))
					_append_layer(layers, _get_indexed_frame("AWeapon", "%02d R" % max(assassin_index, 0), body_frame + weapon_offset))
				else:
					_append_layer(layers, _get_indexed_frame("CWeapon", "%02d" % weapon, body_frame + weapon_offset))
			CLASS_ARCHER:
				if class_weapon:
					var archer_index := weapon - (CLASS_WEAPON_COUNT * 2)
					var archer_frame := body_frame + weapon_offset
					var archer_stem := "%02d" % max(archer_index, 0)
					_append_layer(layers, _get_indexed_frame("ARWeapon", archer_stem, archer_frame))
				else:
					_append_layer(layers, _get_indexed_frame("CWeapon", "%02d" % weapon, body_frame + weapon_offset))
			_:
				_append_layer(layers, _get_indexed_frame("CWeapon", "%02d" % weapon, body_frame + weapon_offset))

		if not class_weapon and weapon_effect > 0:
			_append_layer(layers, _get_indexed_frame("CWeaponEffect", "%02d" % weapon_effect, body_frame + weapon_offset))

	if wing_effect > 0 and wing_effect < 100 and wing_frame >= 0:
		_append_layer(layers, _get_indexed_frame(wing_prefix, "%02d" % (wing_effect - 1), wing_frame + wing_offset))

	var composite := _compose_layers(layers)
	if not composite.is_empty():
		_composite_cache[cache_key] = composite
	return composite

func get_monster_texture(monster: Dictionary, direction: int) -> Dictionary:
	return get_monster_visual(monster, direction, ACTION_STANDING, 0.0, 0)

func get_monster_visual(monster: Dictionary, direction: int, action: String, elapsed_ms: float, action_variant: int = 0) -> Dictionary:
	_ensure_data_root()
	if _data_root == "":
		return {}
	var image: int = monster.get("image", 0)
	var dir8: int = _normalize_direction(direction)
	var extra_byte: int = monster.get("extra_byte", 0)
	var frame := _monster_frame(monster, action, action_variant)
	if frame.is_empty():
		frame = _monster_frame(monster, ACTION_STANDING, 0)
	if frame.is_empty():
		return {}
	var frame_index := _frame_index(frame, elapsed_ms, _is_looping_action(action))
	var draw_frame := _frame_draw_index(frame, dir8, frame_index)
	var cache_key := "monster:%d:%d:%d:%s:%d" % [image, dir8, extra_byte, action, draw_frame]
	if _composite_cache.has(cache_key):
		return _composite_cache[cache_key]
	var file_path := _monster_library_path(image)
	var frame_data := _get_library_frame(file_path, draw_frame)
	if frame_data.is_empty():
		return {}
	var composite := _compose_layers([frame_data])
	if not composite.is_empty():
		_composite_cache[cache_key] = composite
	return composite

func get_npc_texture(image: int, direction: int) -> Dictionary:
	_ensure_data_root()
	if _data_root == "":
		return {}
	var dir8: int = _normalize_direction(direction)
	var cache_key := "npc:%d:%d" % [image, dir8]
	if _composite_cache.has(cache_key):
		return _composite_cache[cache_key]
	var frame := _get_indexed_frame("NPC", "%02d" % max(image, 0), dir8 * DIRECTION_STRIDE)
	if frame.is_empty():
		return {}
	var composite := _compose_layers([frame])
	if not composite.is_empty():
		_composite_cache[cache_key] = composite
	return composite

func get_ground_item_texture(image: int) -> Dictionary:
	_ensure_data_root()
	if _data_root == "":
		return {}
	var cache_key := "ground_item:%d" % image
	if _composite_cache.has(cache_key):
		return _composite_cache[cache_key]
	var frame := _get_root_library_frame("DNItems", image)
	if frame.is_empty():
		return {}
	var composite := _compose_layers([frame])
	if not composite.is_empty():
		_composite_cache[cache_key] = composite
	return composite

func get_player_action_duration_ms(appearance: Dictionary, action: String) -> float:
	var player_class: int = appearance.get("class", CLASS_WARRIOR)
	var weapon: int = appearance.get("weapon", -1)
	var frame := _player_frame(player_class, _player_action_key(player_class, _has_class_weapon(player_class, weapon), action))
	if frame.is_empty():
		return 0.0
	return _frame_duration_ms(frame)

func get_monster_action_duration_ms(monster: Dictionary, action: String, action_variant: int = 0) -> float:
	var frame := _monster_frame(monster, action, action_variant)
	if frame.is_empty():
		return 0.0
	return _frame_duration_ms(frame)

func _ensure_data_root() -> void:
	if _scanned:
		return
	_scanned = true

	var project_dir: String = ProjectSettings.globalize_path("res://")
	if project_dir.ends_with("/"):
		project_dir = project_dir.substr(0, project_dir.length() - 1)
	var repo_root := project_dir.get_base_dir()
	var executable_dir := OS.get_executable_path().get_base_dir()
	var candidates := [
		project_dir.path_join("Data"),
		repo_root.path_join("Data"),
		repo_root.path_join("Client").path_join("Data"),
		repo_root.path_join("Build").path_join("Client").path_join("Data"),
		project_dir.path_join("Build").path_join("Client").path_join("Data"),
		executable_dir.path_join("Data"),
		executable_dir.get_base_dir().path_join("Data"),
	]

	for candidate in candidates:
		if _looks_like_data_root(candidate):
			_data_root = candidate
			break

func _looks_like_data_root(path: String) -> bool:
	if not DirAccess.dir_exists_absolute(path):
		return false
	if FileAccess.file_exists(path.path_join("DNItems.Lib")):
		return true
	if FileAccess.file_exists(path.path_join("Items.Lib")):
		return true
	if DirAccess.dir_exists_absolute(path.path_join("Monster")):
		return true
	if DirAccess.dir_exists_absolute(path.path_join("CArmour")):
		return true
	return false

func _has_class_weapon(player_class: int, weapon: int) -> bool:
	if weapon < 0:
		return false
	var group: int = int(weapon / CLASS_WEAPON_COUNT)
	match group:
		0:
			return player_class == CLASS_WARRIOR or player_class == CLASS_WIZARD or player_class == CLASS_TAOIST
		1:
			return player_class == CLASS_ASSASSIN
		2:
			return player_class == CLASS_ARCHER
		_:
			return false

func _normalize_direction(direction: int) -> int:
	return clampi(direction, 0, 7)

func _append_layer(layers: Array, layer: Dictionary) -> void:
	if not layer.is_empty():
		layers.append(layer)

func _compose_layers(layers: Array) -> Dictionary:
	if layers.is_empty():
		return {}

	var min_x := 0
	var min_y := 0
	var max_x := 0
	var max_y := 0
	var initialized := false

	for layer in layers:
		var image: Image = layer.get("image")
		if image == null:
			continue
		var x: int = layer.get("x", 0)
		var y: int = layer.get("y", 0)
		if not initialized:
			min_x = x
			min_y = y
			max_x = x + image.get_width()
			max_y = y + image.get_height()
			initialized = true
		else:
			min_x = mini(min_x, x)
			min_y = mini(min_y, y)
			max_x = maxi(max_x, x + image.get_width())
			max_y = maxi(max_y, y + image.get_height())

	if not initialized:
		return {}

	var width: int = maxi(1, max_x - min_x)
	var height: int = maxi(1, max_y - min_y)
	var canvas := Image.create(width, height, false, Image.FORMAT_RGBA8)
	canvas.fill(Color(0, 0, 0, 0))

	for layer in layers:
		var src: Image = layer.get("image")
		if src == null:
			continue
		var dst_x: int = layer.get("x", 0) - min_x
		var dst_y: int = layer.get("y", 0) - min_y
		for y in range(src.get_height()):
			for x in range(src.get_width()):
				var color: Color = src.get_pixel(x, y)
				if color.a <= 0.0:
					continue
				canvas.set_pixel(dst_x + x, dst_y + y, color)

	var texture := ImageTexture.create_from_image(canvas)
	return {
		"texture": texture,
		"offset": Vector2(min_x, min_y),
	}

func _get_indexed_frame(subdir: String, stem: String, frame_index: int) -> Dictionary:
	var file_path := _library_file_path(subdir, stem)
	return _get_library_frame(file_path, frame_index)

func _get_root_library_frame(stem: String, frame_index: int) -> Dictionary:
	var file_path := _data_root.path_join("%s.Lib" % stem)
	return _get_library_frame(file_path, frame_index)

func _monster_library_path(image: int) -> String:
	if image == MONSTER_EVIL_MIR or image == MONSTER_DRAGON_STATUE or image == MONSTER_EVIL_MIR_BODY:
		return _data_root.path_join("Dragon.Lib")
	if image >= MONSTER_HELL_BOMB_1 and image <= MONSTER_HELL_BOMB_3:
		return _library_file_path("Monster", "%03d" % 247)
	if image >= MONSTER_CATAPULT and image <= MONSTER_CANON_TREBUCHET:
		return _library_file_path("Siege", "%02d" % (image - MONSTER_CATAPULT))
	if image >= MONSTER_SABUK_GATE and image <= MONSTER_FROZEN_DOOR:
		return _library_file_path("Gate", "%02d" % (image - MONSTER_SABUK_GATE))
	if image >= MONSTER_BABY_PIG and image <= MONSTER_MEDICAL_RAT:
		return _library_file_path("Pet", "%02d" % (image - MONSTER_BABY_PIG))
	return _library_file_path("Monster", "%03d" % max(image, 0))

func _player_action_key(player_class: int, class_weapon: bool, action: String) -> String:
	if player_class == CLASS_ARCHER and class_weapon:
		match action:
			ACTION_WALKING:
				return ACTION_WALKING_BOW
			ACTION_RUNNING:
				return ACTION_RUNNING_BOW
			ACTION_ATTACK1, ACTION_ATTACK_RANGE1:
				return ACTION_ATTACK_RANGE1
	return action

func _player_frame(player_class: int, action: String) -> Dictionary:
	match action:
		ACTION_STANDING:
			return _frame_def(0, 4, 0, 500, 0, 8, 0, 250)
		ACTION_WALKING_BOW:
			return _frame_def(0, 6, 0, 100, 0, 6, 0, 100)
		ACTION_WALKING:
			return _frame_def(32, 6, 0, 100, 64, 6, 0, 100)
		ACTION_RUNNING_BOW:
			return _frame_def(48, 6, 0, 100, 48, 6, 0, 100)
		ACTION_RUNNING:
			return _frame_def(80, 6, 0, 100, 112, 6, 0, 100)
		ACTION_ATTACK1:
			return _frame_def(136, 6, 0, 100, 168, 6, 0, 100)
		ACTION_ATTACK2:
			return _frame_def(184, 6, 0, 100, 216, 6, 0, 100)
		ACTION_ATTACK3:
			return _frame_def(232, 8, 0, 100, 264, 8, 0, 100)
		ACTION_ATTACK_RANGE1:
			return _frame_def(96, 8, 0, 100, 96, 8, 0, 100)
		ACTION_SPELL:
			return _frame_def(296, 6, 0, 100, 328, 6, 0, 100)
		ACTION_STRUCK:
			return _frame_def(360, 3, 0, 100, 392, 3, 0, 100)
		ACTION_DIE:
			return _frame_def(384, 4, 0, 100, 416, 4, 0, 100)
		ACTION_DEAD:
			return _frame_def(387, 1, 3, 1000, 419, 1, 3, 1000)
		_:
			return {}

func _monster_frame(monster: Dictionary, action: String, action_variant: int = 0) -> Dictionary:
	var image: int = monster.get("image", 0)
	var extra_byte: int = monster.get("extra_byte", 0)

	if image == MONSTER_EVIL_MIR or image == MONSTER_DRAGON_STATUE:
		match action:
			ACTION_ATTACK1, ACTION_ATTACK_RANGE1, ACTION_STRUCK:
				return _frame_def(300, 1, -1, 120)
			_:
				return _frame_def(300, 1, -1, 1000)

	if image == MONSTER_EVIL_MIR_BODY:
		return {}

	if image >= MONSTER_HELL_BOMB_1 and image <= MONSTER_HELL_BOMB_3:
		var hell_start := 52 + ((image - MONSTER_HELL_BOMB_1) * 18)
		match action:
			ACTION_STANDING, ACTION_STRUCK, ACTION_ATTACK1:
				return _frame_def(hell_start, 9, -9, 100)
			_:
				return _frame_def(hell_start, 9, -9, 100)

	if image == MONSTER_GREAT_FOX_SPIRIT:
		var fox_stage := clampi(extra_byte, 0, 4)
		var standing_start := fox_stage * 60
		match action:
			ACTION_ATTACK1:
				return _frame_def(standing_start + 22, 8, -8, 120)
			ACTION_STRUCK:
				return _frame_def(standing_start + 20, 2, -2, 200)
			ACTION_DIE:
				return _frame_def(300, 18, -18, 120)
			ACTION_DEAD:
				return _frame_def(317, 1, -1, 1000)
			_:
				return _frame_def(standing_start, 20, -20, 100)

	if image == MONSTER_CAVE_STATUE:
		var cave_start := 18 if monster.get("dir", 0) != 0 else 0
		match action:
			ACTION_DIE:
				return _frame_def(cave_start + 2, 8, -8, 100)
			ACTION_DEAD:
				return _frame_def(cave_start + 9, 1, -1, 1000)
			_:
				return _frame_def(cave_start, 1, -1, 100)

	match action:
		ACTION_WALKING, ACTION_RUNNING:
			return _frame_def(32, 6, 0, 100)
		ACTION_ATTACK2:
			return _frame_def(80, 6, 0, 100)
		ACTION_ATTACK3:
			return _frame_def(80, 6, 0, 100)
		ACTION_ATTACK1, ACTION_ATTACK_RANGE1:
			return _frame_def(80, 6, 0, 100)
		ACTION_STRUCK:
			return _frame_def(128, 2, 0, 200)
		ACTION_DIE:
			return _frame_def(144, 10, 0, 100)
		ACTION_DEAD:
			return _frame_def(153, 1, 9, 1000)
		_:
			return _frame_def(0, 4, 0, 500)

func _frame_def(start: int, count: int, skip: int, interval: int, effect_start: int = 0, effect_count: int = 0, effect_skip: int = 0, effect_interval: int = 0) -> Dictionary:
	return {
		"start": start,
		"count": count,
		"skip": skip,
		"interval": interval,
		"effect_start": effect_start,
		"effect_count": effect_count,
		"effect_skip": effect_skip,
		"effect_interval": effect_interval,
	}

func _frame_draw_index(frame: Dictionary, direction: int, frame_index: int) -> int:
	return frame.get("start", 0) + ((frame.get("count", 1) + frame.get("skip", 0)) * direction) + frame_index

func _effect_draw_index(frame: Dictionary, direction: int, frame_index: int) -> int:
	if frame.get("effect_count", 0) <= 0:
		return -1
	return frame.get("effect_start", 0) + ((frame.get("effect_count", 1) + frame.get("effect_skip", 0)) * direction) + frame_index

func _frame_index(frame: Dictionary, elapsed_ms: float, looped: bool) -> int:
	var count: int = frame.get("count", 1)
	var interval: int = max(frame.get("interval", 100), 1)
	if count <= 1:
		return 0
	var index := int(floor(elapsed_ms / float(interval)))
	if looped:
		return posmod(index, count)
	return mini(index, count - 1)

func _effect_frame_index(frame: Dictionary, elapsed_ms: float, looped: bool) -> int:
	var count: int = frame.get("effect_count", 0)
	if count <= 1:
		return 0
	var interval: int = max(frame.get("effect_interval", frame.get("interval", 100)), 1)
	var index := int(floor(elapsed_ms / float(interval)))
	if looped:
		return posmod(index, count)
	return mini(index, count - 1)

func _frame_duration_ms(frame: Dictionary) -> float:
	return float(max(frame.get("count", 1), 1) * max(frame.get("interval", 100), 1))

func _is_looping_action(action: String) -> bool:
	return action in [ACTION_STANDING, ACTION_WALKING, ACTION_RUNNING, ACTION_WALKING_BOW, ACTION_RUNNING_BOW, ACTION_DEAD]

func _library_file_path(subdir: String, stem: String) -> String:
	return _data_root.path_join(subdir).path_join("%s.Lib" % stem)

func _get_library_frame(file_path: String, frame_index: int) -> Dictionary:
	if frame_index < 0:
		return {}
	var cache_key := "%s#%d" % [file_path, frame_index]
	if _frame_cache.has(cache_key):
		return _frame_cache[cache_key]

	var library := _load_library(file_path)
	if library.is_empty():
		return {}
	var count: int = library.get("count", 0)
	if frame_index >= count:
		return {}
	var index_list: Array = library.get("index_list", [])
	if frame_index >= index_list.size():
		return {}
	var frame := _decode_frame(library, index_list[frame_index])
	if not frame.is_empty():
		_frame_cache[cache_key] = frame
	return frame

func _load_library(file_path: String) -> Dictionary:
	if _library_cache.has(file_path):
		return _library_cache[file_path]
	if not FileAccess.file_exists(file_path):
		return {}

	var bytes := FileAccess.get_file_as_bytes(file_path)
	if bytes.is_empty():
		return {}

	var reader := CrystalBinary.Reader.new(bytes)
	var version := reader.read_i32()
	if version < 2:
		return {}
	var count := reader.read_i32()
	var frame_seek := reader.read_i32() if version >= 3 else 0
	var index_list: Array = []
	for i in range(count):
		index_list.append(reader.read_i32())

	var library := {
		"bytes": bytes,
		"version": version,
		"count": count,
		"frame_seek": frame_seek,
		"index_list": index_list,
	}
	_library_cache[file_path] = library
	return library

func _decode_frame(library: Dictionary, offset: int) -> Dictionary:
	var bytes: PackedByteArray = library.get("bytes", PackedByteArray())
	if offset < 0 or offset >= bytes.size():
		return {}

	var reader := CrystalBinary.Reader.new(bytes)
	reader.pos = offset

	var width := reader.read_i16()
	var height := reader.read_i16()
	var x := reader.read_i16()
	var y := reader.read_i16()
	var shadow_x := reader.read_i16()
	var shadow_y := reader.read_i16()
	var shadow := reader.read_u8()
	var length := reader.read_i32()
	if width <= 0 or height <= 0 or length <= 0:
		return {}

	var compressed: PackedByteArray = reader._take(length)
	var raw := compressed.decompress_dynamic(-1, FileAccess.COMPRESSION_GZIP)
	if raw.is_empty():
		return {}

	var expected := width * height * 4
	if raw.size() < expected:
		return {}

	var rgba := PackedByteArray()
	rgba.resize(expected)
	for i in range(0, expected, 4):
		rgba[i] = raw[i + 2]
		rgba[i + 1] = raw[i + 1]
		rgba[i + 2] = raw[i]
		rgba[i + 3] = raw[i + 3]

	var image := Image.create_from_data(width, height, false, Image.FORMAT_RGBA8, rgba)
	return {
		"image": image,
		"x": x,
		"y": y,
		"shadow_x": shadow_x,
		"shadow_y": shadow_y,
		"shadow": shadow,
		"has_mask": (shadow & 0x80) != 0,
	}
