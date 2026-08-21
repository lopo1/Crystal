extends Control
## 主场景控制: 登录 → 角色选择 → 进入世界(网格+移动+聊天+战斗+HUD+商店+技能)。
## 运行前提: 已启动 Rust 服务器 (`cargo run -p crystal-server`)。

const GameClient := preload("res://scripts/net/game_client.gd")
const Packets := preload("res://scripts/net/crystal_packets.gd")
const Web3Wallet := preload("res://scripts/net/web3_wallet.gd")
const LegacyLibResources := preload("res://scripts/legacy_lib.gd")

const TILE := 32
const ACTION_STANDING := "standing"
const ACTION_WALKING := "walking"
const ACTION_RUNNING := "running"
const ACTION_ATTACK1 := "attack1"
const ACTION_ATTACK2 := "attack2"
const ACTION_ATTACK3 := "attack3"
const ACTION_ATTACK_RANGE1 := "attack_range1"
const ACTION_SPELL := "spell"
const ACTION_STRUCK := "struck"
const ACTION_DIE := "die"
const ACTION_DEAD := "dead"
const WALK_MOVE_MS := 360.0
const RUN_MOVE_MS := 220.0
const DIE_HOLD_MS := 900.0
const UI_TEXTURES := {
	"launch_base": "res://assets/textures/Launch_Base.png",
	"launch_hover": "res://assets/textures/Launch_Hover.png",
	"launch_pressed": "res://assets/textures/Launch_Pressed.png",
	"config_base": "res://assets/textures/Config_Base.png",
	"config_hover": "res://assets/textures/Config_Hover.png",
	"config_pressed": "res://assets/textures/Config_Pressed.png",
	"cross_base": "res://assets/textures/Cross_Base.png",
	"cross_hover": "res://assets/textures/Cross_Hover.png",
	"cross_pressed": "res://assets/textures/Cross_Pressed.png",
	"panel": "res://assets/textures/Config_Base1.png",
	"hp_fill": "res://assets/textures/Blue Progress.png",
	"mp_fill": "res://assets/textures/Green Progress.png",
	"check_base": "res://assets/textures/CheckF_Base2.png",
	"check_hover": "res://assets/textures/CheckF_Hover.png",
	"check_pressed": "res://assets/textures/CheckF_Pressed.png",
	"line_edit": "res://assets/textures/server_base.png",
	"check_off": "res://assets/textures/Config_Check_Off1.png",
	"check_on": "res://assets/textures/Config_Check_On.png",
	"background": "res://assets/textures/pfffft.png",
}

var client: GameClient
var wallet := Web3Wallet.new()
var _wallet_address := ""
var _pending_challenge := ""
var _texture_cache := {}
var _generated_texture_cache := {}
var _legacy_resources := LegacyLibResources.new()

# 游戏内状态
var my_pos := Vector2i(400, 400)
var my_dir := 0
var my_hp := 100
var my_mp := 50
var my_max_hp := 100
var my_max_mp := 50
var my_level := 1
var my_experience: int = 0
var my_max_experience: int = 10
var my_gold: int = 0
var my_name := ""
var _allow_group := true

var players := {}   # object_id -> {sprite, label, pos, name}
var monsters := {}  # object_id -> {sprite, label, pos, name}
var npcs := {}      # object_id -> {sprite, label, pos}
var ground_items := {} # object_id -> {sprite, label, pos}
var _my_inventory: Array = []
var _my_equipment: Array = []
var _my_magics: Array = []
var _last_tick := 0.0

# 输入防抖
var _input_cooldown := 0.0
const INPUT_DELAY := 0.12

# 商店状态
var _shop_goods: Array = []
var _shop_rate: float = 1.0
var _shop_type: int = 0
var _shop_npc_name: String = ""

@onready var login_panel: Control = $LoginPanel
@onready var background: TextureRect = $Background
@onready var game_view: CanvasLayer = $GameView
@onready var world: Node2D = $World
@onready var map_root: Node2D = $World/MapRoot
@onready var camera: Camera2D = $World/Camera2D
@onready var chat_log: RichTextLabel = $GameView/ChatLog
@onready var chat_input: LineEdit = $GameView/ChatInput
@onready var status_label: Label = $LoginPanel/VBox/StatusLabel
@onready var inventory_panel: PanelContainer = $GameView/InventoryPanel
@onready var npc_dialog: PanelContainer = $GameView/NPCDialog
@onready var shop_panel: PanelContainer = $GameView/ShopPanel
@onready var skill_bar: Control = $GameView/SkillBar
var _tile_root: Node2D

func _load_ui_texture(path: String) -> Texture2D:
	if _texture_cache.has(path):
		return _texture_cache[path]
	var image := Image.new()
	var err := image.load(path)
	if err != OK:
		push_error("UI 纹理加载失败: %s (err=%d)" % [path, err])
		return null
	var texture := ImageTexture.create_from_image(image)
	_texture_cache[path] = texture
	return texture

func _make_texture_style(texture_path: String, left: float, top: float, right: float, bottom: float, content: float = -1.0) -> StyleBoxTexture:
	var style := StyleBoxTexture.new()
	style.texture = _load_ui_texture(texture_path)
	style.texture_margin_left = left
	style.texture_margin_top = top
	style.texture_margin_right = right
	style.texture_margin_bottom = bottom
	if content >= 0.0:
		style.content_margin_left = content
		style.content_margin_top = content
		style.content_margin_right = content
		style.content_margin_bottom = content
	return style

func _make_flat_style(bg: Color, border: Color = Color.TRANSPARENT, border_width: int = 0, corner_radius: int = 0) -> StyleBoxFlat:
	var style := StyleBoxFlat.new()
	style.bg_color = bg
	style.border_color = border
	style.border_width_left = border_width
	style.border_width_top = border_width
	style.border_width_right = border_width
	style.border_width_bottom = border_width
	style.corner_radius_top_left = corner_radius
	style.corner_radius_top_right = corner_radius
	style.corner_radius_bottom_left = corner_radius
	style.corner_radius_bottom_right = corner_radius
	return style

func _fill_px(image: Image, x: int, y: int, w: int, h: int, color: Color) -> void:
	image.fill_rect(Rect2i(x, y, w, h), color)

func _image_to_texture(image: Image) -> Texture2D:
	return ImageTexture.create_from_image(image)

func _get_generated_texture(key: String) -> Texture2D:
	if _generated_texture_cache.has(key):
		return _generated_texture_cache[key]
	var image := Image.create(32, 32, false, Image.FORMAT_RGBA8)
	image.fill(Color(0, 0, 0, 0))
	match key:
		"tile_dark":
			image.fill(Color(0.10, 0.13, 0.18, 1.0))
			_fill_px(image, 0, 0, 32, 2, Color(0.16, 0.20, 0.27, 1.0))
			_fill_px(image, 0, 0, 2, 32, Color(0.16, 0.20, 0.27, 1.0))
			_fill_px(image, 30, 0, 2, 32, Color(0.06, 0.08, 0.12, 1.0))
			_fill_px(image, 0, 30, 32, 2, Color(0.06, 0.08, 0.12, 1.0))
		"tile_light":
			image.fill(Color(0.12, 0.16, 0.21, 1.0))
			_fill_px(image, 0, 0, 32, 2, Color(0.19, 0.24, 0.31, 1.0))
			_fill_px(image, 0, 0, 2, 32, Color(0.19, 0.24, 0.31, 1.0))
			_fill_px(image, 30, 0, 2, 32, Color(0.07, 0.09, 0.13, 1.0))
			_fill_px(image, 0, 30, 32, 2, Color(0.07, 0.09, 0.13, 1.0))
		"player_self":
			_fill_px(image, 10, 3, 12, 8, Color(0.95, 0.80, 0.65, 1.0))
			_fill_px(image, 8, 10, 16, 12, Color(0.20, 0.72, 0.30, 1.0))
			_fill_px(image, 6, 11, 4, 10, Color(0.95, 0.80, 0.65, 1.0))
			_fill_px(image, 22, 11, 4, 10, Color(0.95, 0.80, 0.65, 1.0))
			_fill_px(image, 9, 22, 5, 8, Color(0.22, 0.24, 0.30, 1.0))
			_fill_px(image, 18, 22, 5, 8, Color(0.22, 0.24, 0.30, 1.0))
			_fill_px(image, 8, 9, 16, 3, Color(0.10, 0.22, 0.12, 1.0))
			_fill_px(image, 12, 1, 8, 3, Color(0.85, 0.95, 0.30, 1.0))
		"player_other":
			_fill_px(image, 10, 3, 12, 8, Color(0.95, 0.80, 0.65, 1.0))
			_fill_px(image, 8, 10, 16, 12, Color(0.28, 0.48, 0.90, 1.0))
			_fill_px(image, 6, 11, 4, 10, Color(0.95, 0.80, 0.65, 1.0))
			_fill_px(image, 22, 11, 4, 10, Color(0.95, 0.80, 0.65, 1.0))
			_fill_px(image, 9, 22, 5, 8, Color(0.22, 0.24, 0.30, 1.0))
			_fill_px(image, 18, 22, 5, 8, Color(0.22, 0.24, 0.30, 1.0))
			_fill_px(image, 8, 9, 16, 3, Color(0.10, 0.14, 0.24, 1.0))
			_fill_px(image, 11, 1, 10, 3, Color(0.65, 0.40, 0.16, 1.0))
		"npc":
			_fill_px(image, 10, 4, 12, 8, Color(0.95, 0.82, 0.66, 1.0))
			_fill_px(image, 8, 12, 16, 12, Color(0.95, 0.75, 0.18, 1.0))
			_fill_px(image, 7, 10, 18, 3, Color(0.42, 0.18, 0.05, 1.0))
			_fill_px(image, 8, 24, 6, 6, Color(0.45, 0.28, 0.10, 1.0))
			_fill_px(image, 18, 24, 6, 6, Color(0.45, 0.28, 0.10, 1.0))
			_fill_px(image, 5, 6, 22, 3, Color(0.70, 0.10, 0.10, 1.0))
		"item":
			_fill_px(image, 12, 16, 8, 8, Color(0.18, 0.62, 0.22, 1.0))
			_fill_px(image, 10, 14, 12, 2, Color(0.80, 0.95, 0.30, 1.0))
			_fill_px(image, 11, 24, 10, 3, Color(0.10, 0.18, 0.08, 1.0))
		_:
			if key.begins_with("monster:"):
				var variant: int = int(key.get_slice(":", 1)) % 4
				var primary_colors: Array[Color] = [
					Color(0.85, 0.22, 0.22, 1.0),
					Color(0.62, 0.22, 0.82, 1.0),
					Color(0.22, 0.70, 0.78, 1.0),
					Color(0.84, 0.46, 0.18, 1.0),
				]
				var accent_colors: Array[Color] = [
					Color(1.0, 0.85, 0.25, 1.0),
					Color(0.90, 0.45, 1.0, 1.0),
					Color(0.50, 1.0, 0.92, 1.0),
					Color(1.0, 0.72, 0.24, 1.0),
				]
				var primary: Color = primary_colors[variant]
				var accent: Color = accent_colors[variant]
				_fill_px(image, 6, 10, 20, 14, primary)
				_fill_px(image, 4, 14, 4, 10, primary.darkened(0.2))
				_fill_px(image, 24, 14, 4, 10, primary.darkened(0.2))
				_fill_px(image, 9, 4, 14, 8, primary.lightened(0.15))
				_fill_px(image, 8, 2, 4, 4, accent)
				_fill_px(image, 20, 2, 4, 4, accent)
				_fill_px(image, 11, 13, 3, 3, Color.BLACK)
				_fill_px(image, 18, 13, 3, 3, Color.BLACK)
				_fill_px(image, 11, 24, 4, 6, primary.darkened(0.35))
				_fill_px(image, 17, 24, 4, 6, primary.darkened(0.35))
			else:
				image.fill(Color(1.0, 0.0, 1.0, 1.0))
	var texture := _image_to_texture(image)
	_generated_texture_cache[key] = texture
	return texture

func _world_pos(pos: Vector2i, offset: Vector2 = Vector2.ZERO) -> Vector2:
	return Vector2(pos.x * TILE, pos.y * TILE) + offset

func _world_posf(pos: Vector2, offset: Vector2 = Vector2.ZERO) -> Vector2:
	return Vector2(pos.x * TILE, pos.y * TILE) + offset

func _make_world_sprite(texture: Texture2D, pos: Vector2i, offset: Vector2 = Vector2.ZERO) -> Sprite2D:
	var sprite := Sprite2D.new()
	sprite.texture = texture
	sprite.centered = false
	sprite.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	sprite.position = _world_pos(pos, offset)
	return sprite

func _make_name_label(text: String, pos: Vector2i, color: Color = Color.WHITE) -> Label:
	var label := Label.new()
	label.text = text
	label.position = Vector2(pos.x * TILE - 4, pos.y * TILE - 14)
	label.add_theme_font_size_override("font_size", 10)
	label.add_theme_color_override("font_color", color)
	return label

func _apply_entity_visual(sprite: Sprite2D, pos: Vector2i, visual: Dictionary) -> Vector2:
	return _apply_entity_visualf(sprite, Vector2(pos), visual)

func _apply_entity_visualf(sprite: Sprite2D, pos: Vector2, visual: Dictionary) -> Vector2:
	var texture: Texture2D = visual.get("texture", null)
	var offset: Vector2 = visual.get("offset", Vector2.ZERO)
	if texture != null:
		sprite.texture = texture
	sprite.position = _world_posf(pos, offset)
	return offset

func _now_ms() -> float:
	return float(Time.get_ticks_msec())

func _entity_action_elapsed_ms(entity: Dictionary, now_ms: float) -> float:
	return max(0.0, now_ms - float(entity.get("action_started_ms", now_ms)))

func _entity_draw_pos(entity: Dictionary, now_ms: float) -> Vector2:
	var move_duration: float = float(entity.get("move_duration_ms", 0.0))
	if move_duration <= 0.0:
		return Vector2(entity.get("pos", Vector2i.ZERO))
	var move_started: float = float(entity.get("move_started_ms", now_ms))
	var t := clampf((now_ms - move_started) / move_duration, 0.0, 1.0)
	var from_pos: Vector2 = entity.get("move_from", Vector2(entity.get("pos", Vector2i.ZERO)))
	var to_pos: Vector2 = entity.get("move_to", Vector2(entity.get("pos", Vector2i.ZERO)))
	return from_pos.lerp(to_pos, t)

func _set_entity_action(entity: Dictionary, action: String, duration_ms: float, action_variant: int = 0) -> void:
	entity["action"] = action
	entity["action_variant"] = action_variant
	entity["action_started_ms"] = _now_ms()
	entity["action_duration_ms"] = duration_ms

func _start_entity_move(entity: Dictionary, target_pos: Vector2i, direction: int, running: bool) -> void:
	var now_ms := _now_ms()
	var current_draw_pos := _entity_draw_pos(entity, now_ms)
	entity["move_from"] = current_draw_pos
	entity["move_to"] = Vector2(target_pos)
	entity["move_started_ms"] = now_ms
	entity["move_duration_ms"] = RUN_MOVE_MS if running else WALK_MOVE_MS
	entity["pos"] = target_pos
	entity["dir"] = direction
	_set_entity_action(entity, ACTION_RUNNING if running else ACTION_WALKING, float(entity["move_duration_ms"]))

func _set_entity_pose(entity: Dictionary, target_pos: Vector2i, direction: int) -> void:
	entity["pos"] = target_pos
	entity["dir"] = direction
	entity["move_from"] = Vector2(target_pos)
	entity["move_to"] = Vector2(target_pos)
	entity["move_started_ms"] = _now_ms()
	entity["move_duration_ms"] = 0.0

func _step_tile(pos: Vector2i, direction: int, distance: int) -> Vector2i:
	match direction:
		0:
			return Vector2i(pos.x, pos.y - distance)
		1:
			return Vector2i(pos.x + distance, pos.y - distance)
		2:
			return Vector2i(pos.x + distance, pos.y)
		3:
			return Vector2i(pos.x + distance, pos.y + distance)
		4:
			return Vector2i(pos.x, pos.y + distance)
		5:
			return Vector2i(pos.x - distance, pos.y + distance)
		6:
			return Vector2i(pos.x - distance, pos.y)
		7:
			return Vector2i(pos.x - distance, pos.y - distance)
		_:
			return pos

func _update_entity_motion_state(entity: Dictionary, now_ms: float) -> void:
	var move_duration: float = float(entity.get("move_duration_ms", 0.0))
	if move_duration > 0.0:
		var progress := (now_ms - float(entity.get("move_started_ms", now_ms))) / move_duration
		if progress >= 1.0:
			entity["move_duration_ms"] = 0.0
			entity["move_from"] = Vector2(entity.get("pos", Vector2i.ZERO))
			entity["move_to"] = Vector2(entity.get("pos", Vector2i.ZERO))
			var action: String = entity.get("action", ACTION_STANDING)
			if action == ACTION_WALKING or action == ACTION_RUNNING:
				_set_entity_action(entity, ACTION_STANDING, 0.0)

	var action: String = entity.get("action", ACTION_STANDING)
	var action_duration: float = float(entity.get("action_duration_ms", 0.0))
	if action_duration <= 0.0:
		return
	if action in [ACTION_WALKING, ACTION_RUNNING, ACTION_DEAD]:
		return
	if now_ms - float(entity.get("action_started_ms", now_ms)) >= action_duration:
		if action == ACTION_DIE:
			_set_entity_action(entity, ACTION_DEAD, 0.0)
		else:
			_set_entity_action(entity, ACTION_STANDING, 0.0)

func _resolve_player_visual(entity: Dictionary, now_ms: float) -> Dictionary:
	var appearance: Dictionary = entity.get("appearance", {})
	var direction: int = entity.get("dir", 0)
	var action: String = entity.get("action", ACTION_STANDING)
	var action_variant: int = entity.get("action_variant", 0)
	var is_self: bool = entity.get("is_self", false)
	var visual := _legacy_resources.get_player_visual(appearance, direction, action, _entity_action_elapsed_ms(entity, now_ms), action_variant)
	if not visual.is_empty() and visual.get("texture", null) != null:
		return visual
	return {
		"texture": _get_generated_texture("player_self" if is_self else "player_other"),
		"offset": Vector2.ZERO,
	}

func _resolve_monster_visual(monster_data: Dictionary, now_ms: float) -> Dictionary:
	var image: int = monster_data.get("image", 0)
	var action: String = monster_data.get("action", ACTION_STANDING)
	var action_variant: int = monster_data.get("action_variant", 0)
	var direction: int = monster_data.get("dir", 0)
	var visual := _legacy_resources.get_monster_visual(monster_data, direction, action, _entity_action_elapsed_ms(monster_data, now_ms), action_variant)
	if not visual.is_empty() and visual.get("texture", null) != null:
		return visual
	return {
		"texture": _get_generated_texture("monster:%d" % image),
		"offset": Vector2(0, 2),
	}

func _resolve_npc_visual(image: int, direction: int) -> Dictionary:
	var visual := _legacy_resources.get_npc_texture(image, direction)
	if not visual.is_empty() and visual.get("texture", null) != null:
		return visual
	return {
		"texture": _get_generated_texture("npc"),
		"offset": Vector2.ZERO,
	}

func _resolve_ground_item_visual(image: int) -> Dictionary:
	var visual := _legacy_resources.get_ground_item_texture(image)
	if not visual.is_empty() and visual.get("texture", null) != null:
		return visual
	return {
		"texture": _get_generated_texture("item"),
		"offset": Vector2(0, 8),
	}

func _refresh_player_entity(entity: Dictionary, now_ms: float) -> void:
	var draw_pos := _entity_draw_pos(entity, now_ms)
	var visual := _resolve_player_visual(entity, now_ms)
	entity["offset"] = _apply_entity_visualf(entity["sprite"], draw_pos, visual)
	entity["label"].position = Vector2(draw_pos.x * TILE - 4, draw_pos.y * TILE - 14)

func _refresh_monster_entity(entity: Dictionary, now_ms: float) -> void:
	var draw_pos := _entity_draw_pos(entity, now_ms)
	var visual := _resolve_monster_visual(entity, now_ms)
	entity["offset"] = _apply_entity_visualf(entity["sprite"], draw_pos, visual)
	entity["label"].position = Vector2(draw_pos.x * TILE - 4, draw_pos.y * TILE - 14)

func _update_world_animations() -> void:
	var now_ms := _now_ms()
	for entry in players.values():
		_update_entity_motion_state(entry, now_ms)
		_refresh_player_entity(entry, now_ms)
	for entry in monsters.values():
		_update_entity_motion_state(entry, now_ms)
		_refresh_monster_entity(entry, now_ms)

func _queue_free_node(node: Variant) -> void:
	if node == null:
		return
	if node is Node and is_instance_valid(node):
		(node as Node).queue_free()

func _clear_entity_dict(entity_dict: Dictionary) -> void:
	for key in entity_dict.keys():
		var entry: Dictionary = entity_dict[key]
		_queue_free_node(entry.get("sprite"))
		_queue_free_node(entry.get("label"))
	entity_dict.clear()

func _clear_world_state() -> void:
	_clear_entity_dict(players)
	_clear_entity_dict(monsters)
	_clear_entity_dict(npcs)
	_clear_entity_dict(ground_items)
	if _tile_root != null and is_instance_valid(_tile_root):
		for child in _tile_root.get_children():
			child.queue_free()
	_map_info.clear()

func _release_runtime_resources() -> void:
	_clear_world_state()
	if background != null and is_instance_valid(background):
		background.texture = null
	theme = null
	_legacy_resources.clear_caches()
	_generated_texture_cache.clear()
	_texture_cache.clear()

func _build_runtime_theme() -> Theme:
	var theme := Theme.new()
	theme.default_font_size = 12
	theme.set_type_variation(&"ConfigButton", &"Button")
	theme.set_type_variation(&"CloseButton", &"Button")
	theme.set_type_variation(&"HPBar", &"ProgressBar")
	theme.set_type_variation(&"MPBar", &"ProgressBar")
	theme.set_type_variation(&"XPBar", &"ProgressBar")

	var launch_normal := _make_texture_style(UI_TEXTURES.launch_base, 8, 8, 8, 8, 8)
	var launch_hover := _make_texture_style(UI_TEXTURES.launch_hover, 8, 8, 8, 8, 8)
	var launch_pressed := _make_texture_style(UI_TEXTURES.launch_pressed, 8, 8, 8, 8, 8)
	var config_normal := _make_texture_style(UI_TEXTURES.config_base, 3, 3, 3, 3, 6)
	var config_hover := _make_texture_style(UI_TEXTURES.config_hover, 3, 3, 3, 3, 6)
	var config_pressed := _make_texture_style(UI_TEXTURES.config_pressed, 3, 3, 3, 3, 6)
	var close_normal := _make_texture_style(UI_TEXTURES.cross_base, 3, 3, 3, 3)
	var close_hover := _make_texture_style(UI_TEXTURES.cross_hover, 3, 3, 3, 3)
	var close_pressed := _make_texture_style(UI_TEXTURES.cross_pressed, 3, 3, 3, 3)
	var panel_style := _make_texture_style(UI_TEXTURES.panel, 12, 12, 12, 12, 8)
	var line_edit_normal := _make_texture_style(UI_TEXTURES.line_edit, 4, 3, 4, 3)
	var line_edit_focus := _make_texture_style(UI_TEXTURES.line_edit, 4, 3, 4, 3)
	var list_style := _make_flat_style(Color(0.05, 0.05, 0.1, 0.86), Color(0.6, 0.5, 0.3, 0.9), 2, 2)
	var progress_bg := _make_flat_style(Color(0.1, 0.05, 0.05, 0.9), Color(0.3, 0.2, 0.1, 0.9), 1, 1)
	var hp_fill := _make_texture_style(UI_TEXTURES.hp_fill, 2, 0, 2, 0)
	var mp_fill := _make_texture_style(UI_TEXTURES.mp_fill, 2, 0, 2, 0)
	var xp_fill := _make_flat_style(Color(0.85, 0.7, 0.1, 1.0), Color.TRANSPARENT, 0, 1)
	var check_normal := _make_texture_style(UI_TEXTURES.check_base, 4, 4, 4, 4)
	var check_hover := _make_texture_style(UI_TEXTURES.check_hover, 4, 4, 4, 4)
	var check_pressed := _make_texture_style(UI_TEXTURES.check_pressed, 4, 4, 4, 4)

	theme.set_stylebox("normal", &"Button", launch_normal)
	theme.set_stylebox("hover", &"Button", launch_hover)
	theme.set_stylebox("pressed", &"Button", launch_pressed)
	theme.set_stylebox("focus", &"Button", launch_hover)
	theme.set_stylebox("disabled", &"Button", launch_normal)
	theme.set_color("font_color", &"Button", Color(1, 1, 1, 1))
	theme.set_color("font_hover_color", &"Button", Color(1, 0.95, 0.6, 1))
	theme.set_color("font_pressed_color", &"Button", Color(1, 0.8, 0.4, 1))
	theme.set_color("font_disabled_color", &"Button", Color(0.6, 0.6, 0.6, 0.7))
	theme.set_font_size("font_size", &"Button", 12)

	theme.set_stylebox("normal", &"ConfigButton", config_normal)
	theme.set_stylebox("hover", &"ConfigButton", config_hover)
	theme.set_stylebox("pressed", &"ConfigButton", config_pressed)
	theme.set_stylebox("focus", &"ConfigButton", config_hover)
	theme.set_color("font_color", &"ConfigButton", Color(1, 1, 1, 1))
	theme.set_font_size("font_size", &"ConfigButton", 11)

	theme.set_stylebox("normal", &"CloseButton", close_normal)
	theme.set_stylebox("hover", &"CloseButton", close_hover)
	theme.set_stylebox("pressed", &"CloseButton", close_pressed)
	theme.set_stylebox("focus", &"CloseButton", close_hover)

	theme.set_stylebox("panel", &"PanelContainer", panel_style)
	theme.set_stylebox("panel", &"Panel", panel_style)

	theme.set_stylebox("normal", &"LineEdit", line_edit_normal)
	theme.set_stylebox("focus", &"LineEdit", line_edit_focus)
	theme.set_color("font_color", &"LineEdit", Color(1, 1, 1, 1))
	theme.set_color("cursor_color", &"LineEdit", Color(1, 0.9, 0.5, 1))
	theme.set_color("selection_color", &"LineEdit", Color(0.6, 0.5, 0.2, 0.5))
	theme.set_font_size("font_size", &"LineEdit", 12)

	theme.set_stylebox("normal", &"TextEdit", line_edit_normal)
	theme.set_stylebox("focus", &"TextEdit", line_edit_focus)
	theme.set_color("font_color", &"TextEdit", Color(1, 1, 1, 1))
	theme.set_font_size("font_size", &"TextEdit", 12)

	theme.set_stylebox("panel", &"ItemList", list_style)
	theme.set_stylebox("focus", &"ItemList", list_style)
	theme.set_color("font_color", &"ItemList", Color(1, 1, 1, 1))
	theme.set_color("font_selected_color", &"ItemList", Color(1, 0.95, 0.6, 1))
	theme.set_color("guide_color", &"ItemList", Color(0.6, 0.5, 0.3, 0.5))
	theme.set_font_size("font_size", &"ItemList", 12)

	theme.set_stylebox("background", &"ProgressBar", progress_bg)
	theme.set_stylebox("fill", &"ProgressBar", hp_fill)
	theme.set_stylebox("fill", &"HPBar", hp_fill)
	theme.set_stylebox("fill", &"MPBar", mp_fill)
	theme.set_stylebox("fill", &"XPBar", xp_fill)

	theme.set_stylebox("normal", &"OptionButton", config_normal)
	theme.set_stylebox("hover", &"OptionButton", config_hover)
	theme.set_stylebox("pressed", &"OptionButton", config_pressed)
	theme.set_stylebox("focus", &"OptionButton", config_hover)
	theme.set_color("font_color", &"OptionButton", Color(1, 1, 1, 1))
	theme.set_font_size("font_size", &"OptionButton", 12)
	theme.set_stylebox("panel", &"PopupMenu", list_style)
	theme.set_color("font_color", &"PopupMenu", Color(1, 1, 1, 1))
	theme.set_color("font_hover_color", &"PopupMenu", Color(1, 0.95, 0.6, 1))
	theme.set_font_size("font_size", &"PopupMenu", 12)

	theme.set_stylebox("normal", &"CheckBox", check_normal)
	theme.set_stylebox("hover", &"CheckBox", check_hover)
	theme.set_stylebox("pressed", &"CheckBox", check_pressed)
	theme.set_stylebox("focus", &"CheckBox", check_hover)
	theme.set_color("font_color", &"CheckBox", Color(1, 1, 1, 1))
	theme.set_icon("checked", &"CheckBox", _load_ui_texture(UI_TEXTURES.check_on))
	theme.set_icon("unchecked", &"CheckBox", _load_ui_texture(UI_TEXTURES.check_off))
	theme.set_icon("checked_disabled", &"CheckBox", _load_ui_texture(UI_TEXTURES.check_on))
	theme.set_icon("unchecked_disabled", &"CheckBox", _load_ui_texture(UI_TEXTURES.check_off))

	theme.set_color("font_color", &"Label", Color(0.95, 0.9, 0.7, 1))
	theme.set_font_size("font_size", &"Label", 12)
	return theme

func _setup_runtime_ui_theme() -> void:
	theme = _build_runtime_theme()
	var bg_texture := _load_ui_texture(UI_TEXTURES.background)
	background.texture = bg_texture
	background.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST

func _exit_tree() -> void:
	_release_runtime_resources()

func _ready() -> void:
	_setup_runtime_ui_theme()
	_legacy_resources.refresh()
	_tile_root = Node2D.new()
	_tile_root.name = "TileRoot"
	map_root.add_child(_tile_root)
	client = GameClient.new()
	add_child(client)
	client.connected_ok.connect(_on_connected)
	client.login_result.connect(_on_login_result)
	client.characters_loaded.connect(_on_characters_loaded)
	client.entered_world.connect(_on_entered_world)
	client.chat_line.connect(_on_chat_line)
	client.disconnected.connect(func(reason): status_label.text = "断开: " + reason)
	client.server_packet.connect(_on_server_packet)
	client.web3_challenge_received.connect(_on_web3_challenge)
	client.user_location.connect(_on_user_location)
	client.health_changed.connect(_on_health_changed)
	client.gained_experience.connect(_on_gained_experience)
	client.level_changed.connect(_on_level_changed)
	client.damage_indicator.connect(_on_damage_indicator)
	client.object_died.connect(_on_object_died)
	client.object_revived.connect(_on_object_revived)
	client.struck.connect(_on_struck)
	client.gained_gold.connect(_on_gained_gold)
	client.lost_gold.connect(_on_lost_gold)
	client.gained_item.connect(_on_gained_item)
	client.death.connect(_on_death)
	client.user_slots_refresh.connect(_on_user_slots_refresh)
	client.magics_loaded.connect(_on_magics_loaded)
	client.equip_result.connect(_on_equip_result)
	client.use_item_result.connect(_on_use_item_result)
	client.delete_item.connect(_on_delete_item)
	client.colour_changed.connect(_on_colour_changed)
	client.player_inspect.connect(_on_player_inspect)
	client.logout_success.connect(_on_logout_success)
	client.return_to_login.connect(_on_return_to_login)
	client.attack_mode_changed.connect(func(m): chat_log.append_text("[color=gray]攻击模式: %d[/color]\n" % m))
	client.peace_mode_changed.connect(func(m): chat_log.append_text("[color=gray]和平模式: %d[/color]\n" % m))
	client.object_magic.connect(_on_object_magic)
	client.new_magic.connect(_on_new_magic)
	client.magic_leveled.connect(_on_magic_leveled)
	client.group_switched.connect(_on_switch_group)
	client.delete_member.connect(_on_delete_member)
	client.group_invite.connect(_on_group_invite)
	client.add_member.connect(_on_add_member)
	client.friend_update.connect(_on_friend_update)
	client.npc_refine.connect(func(rate, refining): chat_log.append_text("[color=cyan]精炼: 成功率%.1f%% %s[/color]\n" % [rate * 100.0, "进行中" if refining else ""]))
	client.object_hidden.connect(_on_object_hidden)
	client.object_harvest.connect(func(oid): chat_log.append_text("[color=gray]采集中...[/color]\n"))
	$"LoginPanel/VBox/ConnectRow/ConnectButton".pressed.connect(_on_connect_pressed)
	$"LoginPanel/VBox/ConnectRow/NewAccountButton".pressed.connect(_on_new_account_pressed)
	$"LoginPanel/VBox/CharButtonRow/StartGameButton".pressed.connect(_on_start_game_pressed)
	$"LoginPanel/VBox/NewCharRow/CreateButton".pressed.connect(_on_create_pressed)
	$"LoginPanel/VBox/ConnectRow/WalletLoginButton".pressed.connect(_on_wallet_login_pressed)
	$"LoginPanel/VBox/CharButtonRow/DeleteCharButton".pressed.connect(_on_delete_char_pressed)
	$GameView/LogoutButton.pressed.connect(_on_logout_pressed)
	$GameView/NPCDialog/CloseButton.pressed.connect(func(): npc_dialog.hide())
	$GameView/ShopPanel/CloseButton.pressed.connect(func(): shop_panel.hide())
	$GameView/ShopPanel/VBox/ItemList.item_activated.connect(_on_shop_buy_pressed)
	$GameView/InventoryPanel/VBox/ItemList.item_activated.connect(_on_inventory_item_activated)
	chat_input.text_submitted.connect(_on_chat_submitted)
	_show_login()

func _process(delta: float) -> void:
	_input_cooldown = max(0.0, _input_cooldown - delta)
	if not game_view.visible:
		return
	_handle_movement_input()
	if Input.is_key_pressed(KEY_I):
		inventory_panel.visible = not inventory_panel.visible
	_update_world_animations()
	camera.position = Vector2(my_pos.x * TILE + TILE / 2.0, my_pos.y * TILE + TILE / 2.0)
	_update_hud()
	_update_skill_bar()

# ---------------------------------------------------------------------------
# 登录
# ---------------------------------------------------------------------------

var _wallet_login_pending := false
var _login_pending := false

func _on_connect_pressed() -> void:
	var host: String = $"LoginPanel/VBox/ServerRow/ServerLineEdit".text
	if host == "":
		host = "127.0.0.1"
	var port: int = int($"LoginPanel/VBox/ServerRow/PortLineEdit".text)
	status_label.text = "连接 %s:%d ..." % [host, port]
	_login_pending = true
	client.connect_to_server(host, port)

func _on_connected() -> void:
	status_label.text = "已连接，上报版本..."
	client.send_client_version(PackedByteArray())
	if _wallet_login_pending:
		_wallet_login_pending = false
		_begin_wallet_login()
	elif _login_pending:
		_login_pending = false
		_do_account_login()

func _do_account_login() -> void:
	var acc: String = $"LoginPanel/VBox/AccountRow/AccountLineEdit".text
	var pw: String = $"LoginPanel/VBox/PasswordRow/PasswordLineEdit".text
	if acc == "" or pw == "":
		status_label.text = "账号/密码不能为空"
		return
	status_label.text = "登录中..."
	client.login(acc, pw)

func _on_wallet_login_pressed() -> void:
	status_label.text = "连接钱包..."
	wallet.get_address(func(addr: String) -> void:
		if addr == "":
			status_label.text = "未获取到钱包地址（请安装 MetaMask 或改用 RPC 签名服务）"
			return
		_wallet_address = addr
		$"LoginPanel/VBox/WalletStatus".text = "钱包: " + addr.substr(0, 10) + "..."
		if client.is_server_connected():
			_begin_wallet_login()
		else:
			_wallet_login_pending = true
			client.connect_to_server($"LoginPanel/VBox/ServerRow/ServerLineEdit".text, int($"LoginPanel/VBox/ServerRow/PortLineEdit".text))
	)

func _begin_wallet_login() -> void:
	status_label.text = "请求登录挑战..."
	client.web3_request_challenge(_wallet_address)

func _on_web3_challenge(ch: Dictionary) -> void:
	_pending_challenge = ch.get("message", "")
	status_label.text = "请在钱包中确认签名..."
	wallet.personal_sign(_pending_challenge, func(sig: PackedByteArray, _addr: String) -> void:
		if sig.is_empty():
			status_label.text = "签名失败或已取消"
			return
		status_label.text = "提交签名..."
		client.web3_login(_wallet_address, _pending_challenge, sig)
	)

func _on_login_result(result: int) -> void:
	if result == 0:
		return
	status_label.text = "登录失败 (result=%d)" % result

func _on_new_account_pressed() -> void:
	var acc: String = $"LoginPanel/VBox/AccountRow/AccountLineEdit".text
	var pw: String = $"LoginPanel/VBox/PasswordRow/PasswordLineEdit".text
	if acc == "" or pw == "":
		status_label.text = "账号/密码不能为空"
		return
	client.new_account(acc, pw, "godot@example.com", "Godot玩家")

func _on_characters_loaded(characters: Array) -> void:
	var list: ItemList = $"LoginPanel/VBox/CharList"
	list.clear()
	for c in characters:
		list.add_item("%s  Lv.%d  (%s)" % [c.name, c.level, _class_name(c.class)])
	status_label.text = "角色数量: %d" % characters.size()
	_loaded_chars = characters

var _loaded_chars: Array = []

func _on_start_game_pressed() -> void:
	if _loaded_chars.is_empty():
		status_label.text = "没有角色，请先创建"
		return
	var selected: PackedInt32Array = $"LoginPanel/VBox/CharList".get_selected_items()
	var sel: int = selected[0] if selected.size() > 0 else 0
	client.start_game(_loaded_chars[sel].index)

func _on_create_pressed() -> void:
	var nm: String = $"LoginPanel/VBox/NewCharRow/NewCharName".text
	if nm == "":
		status_label.text = "请输入角色名"
		return
	var gender: int = $"LoginPanel/VBox/NewCharRow/GenderOption".selected
	var class_id: int = $"LoginPanel/VBox/NewCharRow/ClassOption".selected
	client.new_character(nm, gender, class_id)

func _on_delete_char_pressed() -> void:
	if _loaded_chars.is_empty():
		status_label.text = "没有角色可删除"
		return
	var selected: PackedInt32Array = $"LoginPanel/VBox/CharList".get_selected_items()
	var sel: int = selected[0] if selected.size() > 0 else 0
	client.delete_character(_loaded_chars[sel].index)
	status_label.text = "已请求删除角色: %s" % _loaded_chars[sel].name

func _on_logout_pressed() -> void:
	client.logout()
	_clear_world_state()
	_show_login()
	status_label.text = "已登出"
	_my_inventory.clear()
	_my_equipment.clear()
	_my_magics.clear()
	_shop_goods.clear()

func _show_login() -> void:
	camera.enabled = false
	world.hide()
	game_view.hide()
	background.show()
	login_panel.show()
	inventory_panel.hide()
	npc_dialog.hide()
	shop_panel.hide()


func _show_game() -> void:
	background.hide()
	login_panel.hide()
	world.show()
	game_view.show()
	camera.enabled = true


func _class_name(class_id: int) -> String:
	return ["战士", "法师", "道士", "刺客", "弓手"][class_id] if class_id < 5 else "?"

# ---------------------------------------------------------------------------
# 游戏内 - 进入世界
# ---------------------------------------------------------------------------

func _on_entered_world(ui: Dictionary) -> void:
	_show_game()
	_legacy_resources.refresh()
	my_pos = ui.get("location", Vector2i(400, 400))
	my_dir = ui.get("direction", 0)
	my_hp = ui.get("hp", 100)
	my_mp = ui.get("mp", 50)
	my_max_hp = ui.get("hp", 100)
	my_max_mp = ui.get("mp", 50)
	my_level = ui.get("level", 1)
	my_experience = ui.get("experience", 0)
	my_max_experience = ui.get("max_experience", 10)
	my_gold = ui.get("gold", 0)
	my_name = ui.get("name", "")
	status_label.text = "进入世界: %s" % my_name
	# 加载背包
	_my_inventory = ui.get("inventory", [])
	_my_equipment = ui.get("equipment", [])
	_my_magics = ui.get("magics", [])
	_populate_inventory_panel()
	_populate_skill_bar()
	_render_grid()
	_ensure_player(ui.get("object_id", 0), my_name, Vector2i(my_pos), my_dir, Color.GREEN, true, ui)

func _render_grid() -> void:
	for child in _tile_root.get_children():
		child.queue_free()
	var info: Dictionary = _map_info
	var w: int = info.get("width", 100)
	var h: int = info.get("height", 100)
	var dark_tile := _get_generated_texture("tile_dark")
	var light_tile := _get_generated_texture("tile_light")
	for x in range(max(0, my_pos.x - 30), min(w, my_pos.x + 31)):
		for y in range(max(0, my_pos.y - 20), min(h, my_pos.y + 21)):
			var cell := Sprite2D.new()
			cell.texture = dark_tile if (x + y) % 2 == 0 else light_tile
			cell.centered = false
			cell.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
			cell.position = Vector2(x * TILE, y * TILE)
			_tile_root.add_child(cell)
	# 重新渲染所有实体
	for oid in players:
		var p: Dictionary = players[oid]
		var draw_pos := _entity_draw_pos(p, _now_ms())
		p["sprite"].position = _world_posf(draw_pos, p.get("offset", Vector2.ZERO))
		p["label"].position = Vector2(draw_pos.x * TILE - 4, draw_pos.y * TILE - 14)
	for oid in monsters:
		var m: Dictionary = monsters[oid]
		var draw_pos := _entity_draw_pos(m, _now_ms())
		m["sprite"].position = _world_posf(draw_pos, m.get("offset", Vector2.ZERO))
		m["label"].position = Vector2(draw_pos.x * TILE - 4, draw_pos.y * TILE - 14)
	for oid in npcs:
		var n: Dictionary = npcs[oid]
		var pos: Vector2i = n["pos"]
		n["sprite"].position = _world_pos(pos, n.get("offset", Vector2.ZERO))
		n["label"].position = Vector2(pos.x * TILE - 4, pos.y * TILE - 14)
	for oid in ground_items:
		var gi: Dictionary = ground_items[oid]
		var pos: Vector2i = gi["pos"]
		gi["sprite"].position = _world_pos(pos, gi.get("offset", Vector2.ZERO))
		gi["label"].position = Vector2(pos.x * TILE - 4, pos.y * TILE - 14)

var _map_info: Dictionary = {}

# ---------------------------------------------------------------------------
# HUD 更新
# ---------------------------------------------------------------------------

func _update_hud() -> void:
	$GameView/StatusBar.text = "位置: (%d, %d)  Lv.%d %s  金:%d" % [my_pos.x, my_pos.y, my_level, my_name, my_gold]
	$"GameView/HUD/HUDBox/HPRow/HPLabel".text = "HP: %d/%d" % [my_hp, my_max_hp]
	$"GameView/HUD/HUDBox/MPRow/MPLabel".text = "MP: %d/%d" % [my_mp, my_max_mp]
	$"GameView/HUD/HUDBox/XPRow/XPLabel".text = "EXP: %d/%d" % [my_experience, my_max_experience]
	$"GameView/HUD/HUDBox/InfoRow/GoldLabel".text = "金: %d" % my_gold
	$"GameView/HUD/HUDBox/InfoRow/LevelLabel".text = "Lv.%d" % my_level
	# HP bar (ProgressBar 使用主题贴图)
	$"GameView/HUD/HUDBox/HPRow/HPBar".max_value = max(my_max_hp, 1)
	$"GameView/HUD/HUDBox/HPRow/HPBar".value = my_hp
	# MP bar
	$"GameView/HUD/HUDBox/MPRow/MPBar".max_value = max(my_max_mp, 1)
	$"GameView/HUD/HUDBox/MPRow/MPBar".value = my_mp
	# XP bar
	$"GameView/HUD/HUDBox/XPRow/XPBar".max_value = max(my_max_experience, 1)
	$"GameView/HUD/HUDBox/XPRow/XPBar".value = my_experience

# ---------------------------------------------------------------------------
# 服务器包处理
# ---------------------------------------------------------------------------

func _on_server_packet(packet: Dictionary) -> void:
	var id: int = packet.get("id", -1)
	var data: Dictionary = packet.get("data", {})
	match id:
		Packets.S_MAP_INFORMATION:
			_map_info = data
		Packets.S_NEW_MAP_INFO:
			_map_info = data.get("info", _map_info)
		Packets.S_OBJECT_PLAYER:
			var oid: int = data.get("object_id", 0)
			var is_self := oid == client.my_object_id()
			if is_self:
				my_pos = data.get("location", my_pos)
				my_dir = data.get("direction", my_dir)
			var name_colour: int = data.get("name_colour", 0)
			var is_npc: bool = name_colour != 0
			var color: Color = Color(0.2, 0.6, 0.9) if not is_npc else Color(1.0, 0.7, 0.0)
			_ensure_player(oid, data.get("name", "?"), data.get("location", Vector2i.ZERO),
				data.get("direction", 0), color, is_self, data)
		Packets.S_OBJECT_MONSTER:
			var oid: int = data.get("object_id", 0)
			if data.get("dead", false):
				_remove_monster(oid)
			else:
				_ensure_monster(oid, data.get("name", "?"), data.get("location", Vector2i.ZERO),
					data.get("direction", 0), data.get("image", 0), data)
		Packets.S_OBJECT_NPC:
			_ensure_npc(data.get("object_id", 0), data.get("name", "?"),
				data.get("location", Vector2i.ZERO), data.get("image", 0), data.get("direction", 0))
		Packets.S_OBJECT_ITEM:
			_ensure_ground_item(data.get("object_id", 0), data.get("name", "?"),
				data.get("location", Vector2i.ZERO), data.get("image", 0))
		Packets.S_OBJECT_GOLD:
			_ensure_ground_item(data.get("object_id", 0), "金 %d" % data.get("gold", 0),
				data.get("location", Vector2i.ZERO), _legacy_resources.gold_frame_for_amount(data.get("gold", 0)))
		Packets.S_OBJECT_WALK, Packets.S_OBJECT_RUN:
			var oid: int = data.get("object_id", 0)
			var direction: int = data.get("direction", 0)
			var target_pos: Vector2i = data.get("location", Vector2i.ZERO)
			if oid == client.my_object_id():
				my_pos = target_pos
				my_dir = direction
				_move_sprite(oid, my_pos, direction, id == Packets.S_OBJECT_RUN)
			elif players.has(oid):
				_move_sprite(oid, target_pos, direction, id == Packets.S_OBJECT_RUN)
			elif monsters.has(oid):
				_move_monster(oid, target_pos, direction, id == Packets.S_OBJECT_RUN)
		Packets.S_OBJECT_TURN:
			var oid: int = data.get("object_id", 0)
			var turn_pos: Vector2i = data.get("location", my_pos if oid == client.my_object_id() else Vector2i.ZERO)
			var turn_dir: int = data.get("direction", 0)
			if oid == client.my_object_id():
				my_pos = turn_pos
				my_dir = turn_dir
			if players.has(oid):
				_set_entity_pose(players[oid], turn_pos if oid == client.my_object_id() else data.get("location", players[oid]["pos"]), turn_dir)
			elif monsters.has(oid):
				_set_entity_pose(monsters[oid], data.get("location", monsters[oid]["pos"]), turn_dir)
			elif npcs.has(oid):
				_move_npc(oid, data.get("location", npcs[oid]["pos"]), turn_dir)
		Packets.S_OBJECT_REMOVE:
			var oid: int = data.get("object_id", 0)
			_remove_sprite(oid)
			_remove_monster(oid)
			_remove_npc(oid)
			_remove_ground_item(oid)
		Packets.S_OBJECT_ATTACK:
			_on_object_attack_visual(data)
		Packets.S_NPC_GOODS:
			_shop_goods = data.get("goods", [])
			_shop_rate = data.get("rate", 1.0)
			_shop_type = data.get("type", 0)
			_show_shop_panel()
		Packets.S_NPC_SELL:
			var result: int = data.get("result", 0)
			if result == 0:
				chat_log.append_text("[color=green]出售成功[/color]\n")
			else:
				chat_log.append_text("[color=red]出售失败[/color]\n")
		Packets.S_NPC_REPAIR:
			var result: int = data.get("result", 0)
			if result == 0:
				chat_log.append_text("[color=green]修理成功[/color]\n")
			else:
				chat_log.append_text("[color=red]修理失败[/color]\n")
		Packets.S_OBJECT_STRUCK:
			var struck_oid: int = data.get("object_id", 0)
			_set_object_action(struck_oid, ACTION_STRUCK)
			_flash_entity(struck_oid)
		Packets.S_OBJECT_DIED:
			var died_oid: int = data.get("object_id", 0)
			_set_object_action(died_oid, ACTION_DIE)

# ---------------------------------------------------------------------------
# 用户位置回调
# ---------------------------------------------------------------------------

func _on_user_location(loc: Vector2i, dir: int) -> void:
	my_pos = loc
	my_dir = dir
	if players.has(client.my_object_id()):
		_set_entity_pose(players[client.my_object_id()], loc, dir)

# ---------------------------------------------------------------------------
# HP/MP/经验值/等级 变化
# ---------------------------------------------------------------------------

func _on_health_changed(hp: int, mp: int) -> void:
	my_hp = hp
	my_mp = mp

func _on_gained_experience(amount: int) -> void:
	my_experience += amount
	chat_log.append_text("[color=yellow]+%d 经验[/color]\n" % amount)

func _on_level_changed(level: int) -> void:
	my_level = level
	chat_log.append_text("[color=cyan]升级! 当前等级: Lv.%d[/color]\n" % level)

func _on_gained_gold(amount: int) -> void:
	my_gold += amount
	chat_log.append_text("[color=yellow]+%d 金[/color]\n" % amount)

func _on_lost_gold(amount: int) -> void:
	my_gold = max(0, my_gold - amount)
	chat_log.append_text("[color=red]-%d 金[/color]\n" % amount)

func _on_gained_item(item: Dictionary) -> void:
	var name: String = item.get("name", "物品#%d" % item.get("item_index", 0))
	chat_log.append_text("[color=green]获得: %s[/color]\n" % name)

func _on_user_slots_refresh(inventory: Array, equipment: Array) -> void:
	_my_inventory = inventory
	_my_equipment = equipment
	_populate_inventory_panel()

func _on_magics_loaded(magics: Array) -> void:
	_my_magics = magics
	_populate_skill_bar()

func _on_equip_result(grid: int, unique_id: int, success: bool) -> void:
	if success:
		chat_log.append_text("[color=green]装备成功[/color]\n")
	else:
		chat_log.append_text("[color=red]装备失败[/color]\n")

func _on_use_item_result(unique_id: int, success: bool) -> void:
	if success:
		chat_log.append_text("[color=green]使用成功 uid=%d[/color]\n" % unique_id)
	else:
		chat_log.append_text("[color=red]使用失败[/color]\n")

func _on_delete_item(unique_id: int, count: int) -> void:
	chat_log.append_text("[color=gray]物品消失 uid=%d x%d[/color]\n" % [unique_id, count])

func _on_colour_changed(name_colour: int) -> void:
	chat_log.append_text("[color=gray]名称颜色已变化[/color]\n")

func _on_player_inspect(info: Dictionary) -> void:
	_show_inspect_panel(info)

func _on_logout_success(characters: Array) -> void:
	_clear_world_state()
	_show_login()
	_on_characters_loaded(characters)
	status_label.text = "已登出"

func _on_return_to_login() -> void:
	_clear_world_state()
	_show_login()
	status_label.text = "已返回登录界面"

func _on_object_magic(data: Dictionary) -> void:
	var oid: int = data.get("object_id", 0)
	_set_object_action(oid, ACTION_SPELL)
	if oid != client.my_object_id():
		_flash_entity(oid)

func _on_new_magic(magic: Dictionary) -> void:
	_my_magics.append(magic)
	_populate_skill_bar()
	chat_log.append_text("[color=cyan]学会新技能: %s[/color]\n" % magic.get("name", "?"))

func _on_magic_leveled(spell: int, level: int, experience: int) -> void:
	for m in _my_magics:
		if m.get("spell", -1) == spell:
			m["level"] = level
			m["experience"] = experience
			chat_log.append_text("[color=cyan]技能升级: %s Lv.%d[/color]\n" % [m.get("name", "?"), level])
			break

func _on_switch_group(allow: bool) -> void:
	_allow_group = allow
	chat_log.append_text("[color=gray]组队邀请: %s[/color]\n" % ["允许" if allow else "禁止"])

func _on_delete_member(name: String) -> void:
	chat_log.append_text("[color=yellow]队友离开: %s[/color]\n" % name)

func _on_group_invite(name: String) -> void:
	chat_log.append_text("[color=yellow]收到组队邀请: %s (输入 /join 加入)[/color]\n" % name)

func _on_add_member(name: String) -> void:
	chat_log.append_text("[color=green]队友加入: %s[/color]\n" % name)

func _on_friend_update(friends: Array) -> void:
	chat_log.append_text("[color=gray]好友列表更新 (%d 人)[/color]\n" % friends.size())

func _on_object_hidden(object_id: int, hidden: bool) -> void:
	if hidden:
		_remove_sprite(object_id)
		_remove_monster(object_id)
		_remove_npc(object_id)
	else:
		chat_log.append_text("[color=gray]实体显现 oid=%d[/color]\n" % object_id)

func _show_inspect_panel(info: Dictionary) -> void:
	var text: String = "查看: %s\n行会: %s %s\nclass: %d  等级: %d\n配偶: %s\n装备:\n" % [
		info.get("name", "?"),
		info.get("guild_name", ""),
		info.get("guild_rank", ""),
		info.get("class", 0),
		info.get("level", 0),
		info.get("lover_name", ""),
	]
	for item in info.get("equipment", []):
		if item != null:
			text += "  物品#%d\n" % item.get("item_index", 0)
	_show_npc_dialog("查看 " + info.get("name", "?"), text)

# ---------------------------------------------------------------------------
# 战斗
# ---------------------------------------------------------------------------

func _on_damage_indicator(damage: int, dmg_type: int, object_id: int) -> void:
	var pos := Vector2i.ZERO
	if object_id == client.my_object_id():
		pos = my_pos
	elif players.has(object_id):
		pos = players[object_id]["pos"]
	elif monsters.has(object_id):
		pos = monsters[object_id]["pos"]
	elif npcs.has(object_id):
		pos = npcs[object_id]["pos"]
	_show_damage_number(pos, damage, Color.RED if dmg_type == 0 else Color.YELLOW)

func _on_struck(attacker_id: int) -> void:
	_show_damage_number(my_pos, 0, Color.RED)

func _on_object_died(object_id: int) -> void:
	_set_object_action(object_id, ACTION_DIE)

func _on_object_revived(object_id: int) -> void:
	_set_object_action(object_id, ACTION_STANDING)

func _on_death() -> void:
	chat_log.append_text("[color=red]你已死亡! 按 R 键回城复活[/color]\n")

func _on_object_attack_visual(data: Dictionary) -> void:
	var atk_oid: int = data.get("object_id", 0)
	var dir: int = data.get("direction", 0)
	var attack_type: int = data.get("type", 0)
	if players.has(atk_oid):
		players[atk_oid]["dir"] = dir
		_set_object_action(atk_oid, ACTION_ATTACK1)
	elif monsters.has(atk_oid):
		monsters[atk_oid]["dir"] = dir
		match attack_type:
			1:
				_set_object_action(atk_oid, ACTION_ATTACK2, 1)
			2:
				_set_object_action(atk_oid, ACTION_ATTACK3, 2)
			_:
				_set_object_action(atk_oid, ACTION_ATTACK1, attack_type)
	_flash_entity(atk_oid)

func _flash_entity(object_id: int) -> void:
	var sprite: CanvasItem = null
	if monsters.has(object_id):
		sprite = monsters[object_id]["sprite"]
	elif players.has(object_id):
		sprite = players[object_id]["sprite"]
	elif npcs.has(object_id):
		sprite = npcs[object_id]["sprite"]
	if sprite == null:
		return
	var orig_modulate: Color = sprite.modulate
	sprite.modulate = Color(1.4, 0.4, 0.4, 1.0)
	var tween := create_tween()
	tween.tween_interval(0.1)
	tween.tween_property(sprite, "modulate", orig_modulate, 0.1)

func _show_damage_number(pos: Vector2i, amount: int, color: Color) -> void:
	var label := Label.new()
	label.text = str(amount)
	label.position = Vector2(pos.x * TILE + randf_range(-10, 10), pos.y * TILE - 20)
	label.add_theme_font_size_override("font_size", 14)
	label.add_theme_color_override("font_color", color)
	map_root.add_child(label)
	# 自动消失动画
	var tween := create_tween()
	tween.tween_property(label, "position:y", label.position.y - 30, 0.8)
	tween.parallel().tween_property(label, "modulate:a", 0.0, 0.8)
	tween.tween_callback(label.queue_free)

# ---------------------------------------------------------------------------
# 聊天
# ---------------------------------------------------------------------------

func _on_chat_line(object_id: int, text: String) -> void:
	var name := "系统"
	if players.has(object_id):
		name = players[object_id].get("name", "?")
	if object_id == client.my_object_id():
		name = "我"
	chat_log.append_text("[b]%s[/b]: %s\n" % [name, text])

func _on_chat_submitted(text: String) -> void:
	if text == "":
		return
	# 斜杠命令
	if text.begins_with("/"):
		_handle_chat_command(text)
	else:
		client.chat(text)
	chat_input.clear()

func _handle_chat_command(text: String) -> void:
	var parts := text.split(" ")
	var cmd: String = parts[0].to_lower()
	match cmd:
		"/inspect", "/look", "/查看":
			if parts.size() > 1:
				client.inspect(parts[1])
			else:
				chat_log.append_text("[color=gray]用法: /inspect <玩家名>[/color]\n")
		"/join", "/加入组队":
			client.group_invite_response(true)
		"/leave", "/离队":
			client.del_group_member(my_name)
		"/group", "/组队":
			client.switch_group(not _allow_group)
		"/invite", "/邀请":
			if parts.size() > 1:
				client.add_group_member(parts[1])
			else:
				chat_log.append_text("[color=gray]用法: /invite <玩家名>[/color]\n")
		"/friend", "/好友":
			if parts.size() > 1:
				client.add_friend(parts[1])
			else:
				chat_log.append_text("[color=gray]用法: /friend <玩家名>[/color]\n")
		"/unfriend", "/删好友":
			if parts.size() > 1:
				client.remove_friend(parts[1])
		"/friends", "/好友列表":
			client.refresh_friends()
		"/amode", "/攻击模式":
			client.change_attack_mode()
		"/pmode", "/和平模式":
			client.change_peace_mode()
		"/fish", "/钓鱼":
			client.fishing_cast()
		"/guild", "/行会":
			client.request_guild_info()
		"/dropgold", "/丢金":
			if parts.size() > 1:
				client.drop_gold(int(parts[1]))
			else:
				chat_log.append_text("[color=gray]用法: /dropgold <数量>[/color]\n")
		"/tp", "/传送":
			if parts.size() > 1:
				# 尝试通过名称找到 NPC 并传送
				for oid in npcs:
					if npcs[oid]["name"] == parts[1]:
						client.teleport_to_npc(oid)
						return
				chat_log.append_text("[color=red]未找到NPC: %s[/color]\n" % parts[1])
		_:
			chat_log.append_text("[color=gray]未知命令: %s[/color]\n" % cmd)

func _show_npc_dialog(npc_name: String, text: String) -> void:
	npc_dialog.visible = true
	$GameView/NPCDialog/VBox/NPCName.text = npc_name
	$GameView/NPCDialog/VBox/DialogText.text = text

func _populate_inventory_panel() -> void:
	var item_list: ItemList = $GameView/InventoryPanel/VBox/ItemList
	item_list.clear()
	item_list.add_item("--- 装备 ---")
	for item in _my_equipment:
		if item != null:
			var name: String = "物品#%d" % item.get("item_index", 0)
			var dura: int = item.get("current_dura", 0)
			var uid: int = item.get("unique_id", 0)
			item_list.add_item("%s (耐久:%d) [%d]" % [name, dura, uid])
	item_list.add_item("--- 背包 ---")
	for item in _my_inventory:
		if item != null:
			var name: String = "物品#%d" % item.get("item_index", 0)
			var count: int = item.get("count", 1)
			var dura: int = item.get("current_dura", 0)
			var uid: int = item.get("unique_id", 0)
			if count > 1:
				item_list.add_item("%s x%d [%d]" % [name, count, uid])
			else:
				item_list.add_item("%s (耐久:%d) [%d]" % [name, dura, uid])

func _on_inventory_item_activated(index: int) -> void:
	var item_list: ItemList = $GameView/InventoryPanel/VBox/ItemList
	var text: String = item_list.get_item_text(index)
	var regex := RegEx.new()
	regex.compile("\\[(\\d+)\\]$")
	var m: RegExMatch = regex.search(text)
	if m == null:
		return
	var uid: int = int(m.get_string(1))
	if text.begins_with("---"):
		return
	if text.find("耐久") >= 0 and text.find("x") < 0:
		client.equip_item(uid, 1, -1)
		chat_log.append_text("[color=gray]装备物品 uid=%d[/color]\n" % uid)
	else:
		client.use_item(uid, 1)
		chat_log.append_text("[color=gray]使用物品 uid=%d[/color]\n" % uid)

# ---------------------------------------------------------------------------
# 商店面板
# ---------------------------------------------------------------------------

func _show_shop_panel() -> void:
	shop_panel.visible = true
	var list: ItemList = $GameView/ShopPanel/VBox/ItemList
	list.clear()
	$GameView/ShopPanel/VBox/ShopTitle.text = "商店 - %s" % _shop_npc_name
	for item in _shop_goods:
		if item != null:
			var name: String = "物品#%d" % item.get("item_index", 0)
			var count: int = item.get("count", 1)
			if count > 1:
				list.add_item("%s (x%d)" % [name, count])
			else:
				list.add_item(name)

func _on_shop_buy_pressed(index: int) -> void:
	if index < 0 or index >= _shop_goods.size():
		return
	var item: Dictionary = _shop_goods[index]
	var item_index: int = item.get("item_index", 0)
	client.buy_item(item_index, 1, _shop_type)
	chat_log.append_text("[color=green]购买物品 #%d[/color]\n" % item_index)

# ---------------------------------------------------------------------------
# 技能栏
# ---------------------------------------------------------------------------

func _populate_skill_bar() -> void:
	for child in skill_bar.get_children():
		child.queue_free()
	for i in range(min(_my_magics.size(), 8)):
		var magic: Dictionary = _my_magics[i]
		var btn := Button.new()
		btn.text = "F%d:%s" % [i + 1, magic.get("name", "?")]
		btn.custom_minimum_size = Vector2(80, 32)
		btn.add_theme_font_size_override("font_size", 11)
		btn.theme_type_variation = &"ConfigButton"
		var spell_id: int = magic.get("spell", 0)
		btn.pressed.connect(func(): _cast_spell(spell_id, magic.get("name", "?")))
		skill_bar.add_child(btn)

func _cast_spell(spell_id: int, spell_name: String) -> void:
	client.magic(my_dir, spell_id)
	_set_object_action(client.my_object_id(), ACTION_SPELL)
	chat_log.append_text("[color=cyan]施放: %s[/color]\n" % spell_name)

func _update_skill_bar() -> void:
	pass

func _move_monster(object_id: int, pos: Vector2i, direction: int = -1, running: bool = false) -> void:
	if monsters.has(object_id):
		var m: Dictionary = monsters[object_id]
		_start_entity_move(m, pos, direction if direction >= 0 else m.get("dir", 0), running)

# ---------------------------------------------------------------------------
# 移动输入
# ---------------------------------------------------------------------------

func _handle_movement_input() -> void:
	if not client.is_server_connected():
		return
	if _input_cooldown > 0.0:
		return
	var dir := -1
	if Input.is_action_just_pressed("ui_up"): dir = 0
	elif Input.is_action_just_pressed("ui_right"): dir = 2
	elif Input.is_action_just_pressed("ui_down"): dir = 4
	elif Input.is_action_just_pressed("ui_left"): dir = 6
	if dir >= 0:
		my_dir = dir
		_input_cooldown = INPUT_DELAY
		if Input.is_key_pressed(KEY_SHIFT):
			client.run(dir)
			if players.has(client.my_object_id()):
				_start_entity_move(players[client.my_object_id()], _step_tile(my_pos, dir, 2), dir, true)
		else:
			client.walk(dir)
			if players.has(client.my_object_id()):
				_start_entity_move(players[client.my_object_id()], _step_tile(my_pos, dir, 1), dir, false)
	# 攻击: 空格键
	if Input.is_action_just_pressed("ui_accept"):
		client.attack(my_dir)
		_set_object_action(client.my_object_id(), ACTION_ATTACK1)
		_input_cooldown = INPUT_DELAY
	# 拾取: Z 键
	if Input.is_key_pressed(KEY_Z):
		client.pick_up()
		_input_cooldown = INPUT_DELAY
	# NPC交互: C 键 (找最近的NPC)
	if Input.is_key_pressed(KEY_C):
		_interact_nearest_npc()
		_input_cooldown = INPUT_DELAY
	# 复活: R 键
	if Input.is_key_pressed(KEY_R):
		client.town_revive()
		_input_cooldown = INPUT_DELAY
	# 技能快捷键: F1-F8
	for i in range(8):
		if Input.is_key_pressed(KEY_F1 + i) and i < _my_magics.size():
			var magic: Dictionary = _my_magics[i]
			var spell_id: int = magic.get("spell", 0)
			client.magic(my_dir, spell_id)
			_set_object_action(client.my_object_id(), ACTION_SPELL)
			chat_log.append_text("[color=cyan]施放: %s[/color]\n" % magic.get("name", "?"))
			_input_cooldown = INPUT_DELAY
			break

func _interact_nearest_npc() -> void:
	var nearest_id := -1
	var nearest_dist := 999999
	for oid in npcs:
		var n: Dictionary = npcs[oid]
		var dist: int = abs(n["pos"].x - my_pos.x) + abs(n["pos"].y - my_pos.y)
		if dist < nearest_dist:
			nearest_dist = dist
			nearest_id = oid
	if nearest_id >= 0 and nearest_dist <= 3:
		client.call_npc(nearest_id)
		var n: Dictionary = npcs[nearest_id]
		_shop_npc_name = n["name"]
		_show_npc_dialog(n["name"], "与 %s 对话中..." % n["name"])

func _ensure_player(object_id: int, pname: String, pos: Vector2i, direction: int, color: Color, is_self: bool, appearance: Dictionary = {}) -> void:
	if players.has(object_id):
		var p: Dictionary = players[object_id]
		p["pos"] = pos
		p["name"] = pname
		p["dir"] = direction
		p["is_self"] = is_self
		p["appearance"] = appearance.duplicate(true)
		p["label"].text = pname
		_refresh_player_entity(p, _now_ms())
		return
	var sprite := _make_world_sprite(null, pos, Vector2.ZERO)
	var label := _make_name_label(pname, pos, Color.WHITE if is_self else Color(0.85, 0.95, 1.0, 1.0))
	map_root.add_child(sprite)
	map_root.add_child(label)
	players[object_id] = {
		"sprite": sprite,
		"label": label,
		"pos": pos,
		"name": pname,
		"dir": direction,
		"offset": Vector2.ZERO,
		"is_self": is_self,
		"appearance": appearance.duplicate(true),
		"action": ACTION_STANDING,
		"action_variant": 0,
		"action_started_ms": _now_ms(),
		"action_duration_ms": 0.0,
		"move_from": Vector2(pos),
		"move_to": Vector2(pos),
		"move_started_ms": _now_ms(),
		"move_duration_ms": 0.0,
	}
	_refresh_player_entity(players[object_id], _now_ms())

func _ensure_monster(object_id: int, mname: String, pos: Vector2i, direction: int, image: int, monster_data: Dictionary = {}) -> void:
	if monsters.has(object_id):
		var m: Dictionary = monsters[object_id]
		m["pos"] = pos
		m["name"] = mname
		m["image"] = image
		m["dir"] = direction
		m["extra_byte"] = monster_data.get("extra_byte", m.get("extra_byte", 0))
		m["label"].text = mname
		_refresh_monster_entity(m, _now_ms())
		return
	var sprite := _make_world_sprite(null, pos, Vector2.ZERO)
	var label := _make_name_label(mname, pos, Color.RED)
	map_root.add_child(sprite)
	map_root.add_child(label)
	monsters[object_id] = {
		"sprite": sprite,
		"label": label,
		"pos": pos,
		"name": mname,
		"image": image,
		"dir": direction,
		"offset": Vector2.ZERO,
		"extra_byte": monster_data.get("extra_byte", 0),
		"action": ACTION_STANDING,
		"action_variant": 0,
		"action_started_ms": _now_ms(),
		"action_duration_ms": 0.0,
		"move_from": Vector2(pos),
		"move_to": Vector2(pos),
		"move_started_ms": _now_ms(),
		"move_duration_ms": 0.0,
	}
	_refresh_monster_entity(monsters[object_id], _now_ms())

func _ensure_npc(object_id: int, nname: String, pos: Vector2i, image: int, direction: int) -> void:
	if npcs.has(object_id):
		var n: Dictionary = npcs[object_id]
		n["pos"] = pos
		n["name"] = nname
		n["image"] = image
		n["dir"] = direction
		var existing_visual := _resolve_npc_visual(image, direction)
		n["offset"] = _apply_entity_visual(n["sprite"], pos, existing_visual)
		n["label"].position = Vector2(pos.x * TILE - 4, pos.y * TILE - 14)
		n["label"].text = nname
		return
	var visual := _resolve_npc_visual(image, direction)
	var offset: Vector2 = visual.get("offset", Vector2.ZERO)
	var sprite := _make_world_sprite(visual.get("texture", null), pos, offset)
	var label := _make_name_label(nname, pos, Color.GOLD)
	map_root.add_child(sprite)
	map_root.add_child(label)
	npcs[object_id] = {"sprite": sprite, "label": label, "pos": pos, "name": nname, "image": image, "dir": direction, "offset": offset}

func _ensure_ground_item(object_id: int, iname: String, pos: Vector2i, image: int) -> void:
	if ground_items.has(object_id):
		var gi: Dictionary = ground_items[object_id]
		gi["pos"] = pos
		gi["name"] = iname
		gi["image"] = image
		var existing_visual := _resolve_ground_item_visual(image)
		gi["offset"] = _apply_entity_visual(gi["sprite"], pos, existing_visual)
		gi["label"].position = Vector2(pos.x * TILE - 4, pos.y * TILE - 14)
		gi["label"].text = iname
		return
	var visual := _resolve_ground_item_visual(image)
	var offset: Vector2 = visual.get("offset", Vector2(0, 8))
	var sprite := _make_world_sprite(visual.get("texture", null), pos, offset)
	var label := _make_name_label(iname, pos, Color.GREEN)
	label.add_theme_font_size_override("font_size", 8)
	map_root.add_child(sprite)
	map_root.add_child(label)
	ground_items[object_id] = {"sprite": sprite, "label": label, "pos": pos, "name": iname, "image": image, "offset": offset}

func _move_sprite(object_id: int, pos: Vector2i, direction: int = -1, running: bool = false) -> void:
	if players.has(object_id):
		var p: Dictionary = players[object_id]
		_start_entity_move(p, pos, direction if direction >= 0 else p.get("dir", 0), running)

func _move_npc(object_id: int, pos: Vector2i, direction: int = -1) -> void:
	if npcs.has(object_id):
		var n: Dictionary = npcs[object_id]
		n["pos"] = pos
		if direction >= 0:
			n["dir"] = direction
		var visual := _resolve_npc_visual(n.get("image", 0), n.get("dir", 0))
		n["offset"] = _apply_entity_visual(n["sprite"], pos, visual)
		n["label"].position = Vector2(pos.x * TILE - 4, pos.y * TILE - 14)

func _set_object_action(object_id: int, action: String, action_variant: int = 0) -> void:
	var now_ms := _now_ms()
	if players.has(object_id):
		var p: Dictionary = players[object_id]
		_set_entity_action(p, action, _legacy_resources.get_player_action_duration_ms(p.get("appearance", {}), action), action_variant)
		_refresh_player_entity(p, now_ms)
	elif monsters.has(object_id):
		var m: Dictionary = monsters[object_id]
		var duration := _legacy_resources.get_monster_action_duration_ms(m, action, action_variant)
		if action == ACTION_DIE:
			duration = max(duration, DIE_HOLD_MS)
		_set_entity_action(m, action, duration, action_variant)
		_refresh_monster_entity(m, now_ms)

func _remove_sprite(object_id: int) -> void:
	if players.has(object_id):
		var p: Dictionary = players[object_id]
		_queue_free_node(p.get("sprite"))
		_queue_free_node(p.get("label"))
		players.erase(object_id)

func _remove_monster(object_id: int) -> void:
	if monsters.has(object_id):
		var m: Dictionary = monsters[object_id]
		_queue_free_node(m.get("sprite"))
		_queue_free_node(m.get("label"))
		monsters.erase(object_id)

func _remove_npc(object_id: int) -> void:
	if npcs.has(object_id):
		var n: Dictionary = npcs[object_id]
		_queue_free_node(n.get("sprite"))
		_queue_free_node(n.get("label"))
		npcs.erase(object_id)

func _remove_ground_item(object_id: int) -> void:
	if ground_items.has(object_id):
		var gi: Dictionary = ground_items[object_id]
		_queue_free_node(gi.get("sprite"))
		_queue_free_node(gi.get("label"))
		ground_items.erase(object_id)
