extends Node
## 主场景控制: 登录 → 角色选择 → 进入世界(网格+移动+聊天+战斗+HUD+商店+技能)。
## 运行前提: 已启动 Rust 服务器 (`cargo run -p crystal-server`)。

const GameClient := preload("res://scripts/net/game_client.gd")
const Packets := preload("res://scripts/net/crystal_packets.gd")
const Web3Wallet := preload("res://scripts/net/web3_wallet.gd")

const TILE := 32

var client: GameClient
var wallet := Web3Wallet.new()
var _wallet_address := ""
var _pending_challenge := ""

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
@onready var game_view: Control = $GameView
@onready var map_root: Node2D = $GameView/MapRoot
@onready var chat_log: RichTextLabel = $GameView/ChatLog
@onready var chat_input: LineEdit = $GameView/ChatInput
@onready var status_label: Label = $LoginPanel/StatusLabel
@onready var inventory_panel: PanelContainer = $GameView/InventoryPanel
@onready var npc_dialog: PanelContainer = $GameView/NPCDialog
@onready var shop_panel: PanelContainer = $GameView/ShopPanel
@onready var skill_bar: Control = $GameView/SkillBar

func _ready() -> void:
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
	client.switch_group.connect(_on_switch_group)
	client.delete_member.connect(_on_delete_member)
	client.group_invite.connect(_on_group_invite)
	client.add_member.connect(_on_add_member)
	client.friend_update.connect(_on_friend_update)
	client.npc_refine.connect(func(rate, refining): chat_log.append_text("[color=cyan]精炼: 成功率%.1f%% %s[/color]\n" % [rate * 100.0, "进行中" if refining else ""]))
	client.object_hidden.connect(_on_object_hidden)
	client.object_harvest.connect(func(oid): chat_log.append_text("[color=gray]采集中...[/color]\n"))
	$LoginPanel/ConnectButton.pressed.connect(_on_connect_pressed)
	$LoginPanel/NewAccountButton.pressed.connect(_on_new_account_pressed)
	$LoginPanel/StartGameButton.pressed.connect(_on_start_game_pressed)
	$LoginPanel/CreateButton.pressed.connect(_on_create_pressed)
	$LoginPanel/WalletLoginButton.pressed.connect(_on_wallet_login_pressed)
	$LoginPanel/DeleteCharButton.pressed.connect(_on_delete_char_pressed)
	$GameView/LogoutButton.pressed.connect(_on_logout_pressed)
	$GameView/NPCDialog/CloseButton.pressed.connect(func(): npc_dialog.hide())
	$GameView/ShopPanel/CloseButton.pressed.connect(func(): shop_panel.hide())
	$GameView/ShopPanel/VBox/ItemList.item_activated.connect(_on_shop_buy_pressed)
	$GameView/InventoryPanel/ItemList.item_activated.connect(_on_inventory_item_activated)
	chat_input.text_submitted.connect(_on_chat_submitted)
	game_view.hide()

func _process(delta: float) -> void:
	_input_cooldown = max(0.0, _input_cooldown - delta)
	if game_view.visible:
		_handle_movement_input()
		# 切换背包面板 (I 键)
		if Input.is_key_pressed(KEY_I):
			inventory_panel.visible = not inventory_panel.visible
	# 相机跟随
	if my_pos != Vector2i.ZERO:
		$GameView/Camera2D.position = Vector2(my_pos.x * TILE + TILE / 2.0, my_pos.y * TILE + TILE / 2.0)
	# 更新 HUD
	_update_hud()
	_update_skill_bar()

# ---------------------------------------------------------------------------
# 登录
# ---------------------------------------------------------------------------

var _wallet_login_pending := false
var _login_pending := false

func _on_connect_pressed() -> void:
	var host: String = $LoginPanel/ServerLineEdit.text
	if host == "":
		host = "127.0.0.1"
	var port: int = int($LoginPanel/PortLineEdit.text)
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
	var acc: String = $LoginPanel/AccountLineEdit.text
	var pw: String = $LoginPanel/PasswordLineEdit.text
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
		$LoginPanel/WalletStatus.text = "钱包: " + addr.substr(0, 10) + "..."
		if client.is_connected():
			_begin_wallet_login()
		else:
			_wallet_login_pending = true
			client.connect_to_server($LoginPanel/ServerLineEdit.text, int($LoginPanel/PortLineEdit.text))
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
	var acc: String = $LoginPanel/AccountLineEdit.text
	var pw: String = $LoginPanel/PasswordLineEdit.text
	if acc == "" or pw == "":
		status_label.text = "账号/密码不能为空"
		return
	client.new_account(acc, pw, "godot@example.com", "Godot玩家")

func _on_characters_loaded(characters: Array) -> void:
	var list: ItemList = $LoginPanel/CharList
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
	var selected := $LoginPanel/CharList.get_selected_items()
	var sel: int = selected[0] if selected.size() > 0 else 0
	client.start_game(_loaded_chars[sel].index)

func _on_create_pressed() -> void:
	var nm: String = $LoginPanel/NewCharName.text
	if nm == "":
		status_label.text = "请输入角色名"
		return
	var gender: int = $LoginPanel/GenderOption.selected
	var class_id: int = $LoginPanel/ClassOption.selected
	client.new_character(nm, gender, class_id)

func _on_delete_char_pressed() -> void:
	if _loaded_chars.is_empty():
		status_label.text = "没有角色可删除"
		return
	var selected := $LoginPanel/CharList.get_selected_items()
	var sel: int = selected[0] if selected.size() > 0 else 0
	client.delete_character(_loaded_chars[sel].index)
	status_label.text = "已请求删除角色: %s" % _loaded_chars[sel].name

func _on_logout_pressed() -> void:
	client.logout()
	game_view.hide()
	login_panel.show()
	status_label.text = "已登出"
	inventory_panel.hide()
	npc_dialog.hide()
	shop_panel.hide()
	players.clear()
	monsters.clear()
	npcs.clear()
	ground_items.clear()
	_my_inventory.clear()
	_my_equipment.clear()
	_my_magics.clear()
	_shop_goods.clear()

func _class_name(class_id: int) -> String:
	return ["战士", "法师", "道士", "刺客", "弓手"][class_id] if class_id < 5 else "?"

# ---------------------------------------------------------------------------
# 游戏内 - 进入世界
# ---------------------------------------------------------------------------

func _on_entered_world(ui: Dictionary) -> void:
	login_panel.hide()
	game_view.show()
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
	_ensure_player(ui.get("object_id", 0), my_name, Vector2i(my_pos), my_dir, Color.GREEN, true)

func _render_grid() -> void:
	for child in map_root.get_children():
		child.queue_free()
	var info: Dictionary = _map_info
	var w: int = info.get("width", 100)
	var h: int = info.get("height", 100)
	for x in range(max(0, my_pos.x - 30), min(w, my_pos.x + 31)):
		for y in range(max(0, my_pos.y - 20), min(h, my_pos.y + 21)):
			var cell := ColorRect.new()
			cell.color = Color(0.12, 0.16, 0.2) if (x + y) % 2 == 0 else Color(0.14, 0.18, 0.22)
			cell.position = Vector2(x * TILE, y * TILE)
			cell.size = Vector2(TILE, TILE)
			map_root.add_child(cell)
	# 重新渲染所有实体
	for oid in players:
		var p: Dictionary = players[oid]
		var pos: Vector2i = p["pos"]
		p["sprite"].position = Vector2(pos.x * TILE + 2, pos.y * TILE + 2)
		p["label"].position = Vector2(pos.x * TILE, pos.y * TILE - 14)
	for oid in monsters:
		var m: Dictionary = monsters[oid]
		var pos: Vector2i = m["pos"]
		m["sprite"].position = Vector2(pos.x * TILE + 2, pos.y * TILE + 2)
		m["label"].position = Vector2(pos.x * TILE, pos.y * TILE - 14)
	for oid in npcs:
		var n: Dictionary = npcs[oid]
		var pos: Vector2i = n["pos"]
		n["sprite"].position = Vector2(pos.x * TILE + 2, pos.y * TILE + 2)
		n["label"].position = Vector2(pos.x * TILE, pos.y * TILE - 14)
	for oid in ground_items:
		var gi: Dictionary = ground_items[oid]
		var pos: Vector2i = gi["pos"]
		gi["sprite"].position = Vector2(pos.x * TILE + 2, pos.y * TILE + 2)
		gi["label"].position = Vector2(pos.x * TILE, pos.y * TILE - 14)

var _map_info: Dictionary = {}

# ---------------------------------------------------------------------------
# HUD 更新
# ---------------------------------------------------------------------------

func _update_hud() -> void:
	$GameView/StatusBar.text = "位置: (%d, %d)  Lv.%d %s  金:%d" % [my_pos.x, my_pos.y, my_level, my_name, my_gold]
	$GameView/HUD/HPLabel.text = "HP: %d/%d" % [my_hp, my_max_hp]
	$GameView/HUD/MPLabel.text = "MP: %d/%d" % [my_mp, my_max_mp]
	$GameView/HUD/XPLabel.text = "EXP: %d/%d" % [my_experience, my_max_experience]
	$GameView/HUD/GoldLabel.text = "金: %d" % my_gold
	$GameView/HUD/LevelLabel.text = "Lv.%d" % my_level
	# HP bar
	var hp_ratio: float = float(my_hp) / float(max(my_max_hp, 1))
	$GameView/HUD/HPBar.color = Color(0.8, 0.1, 0.1)
	$GameView/HUD/HPBarFill.size.x = $GameView/HUD/HPBar.size.x * hp_ratio
	$GameView/HUD/HPBarFill.color = Color(0.1, 0.8, 0.1)
	# MP bar
	var mp_ratio: float = float(my_mp) / float(max(my_max_mp, 1))
	$GameView/HUD/MPBar.color = Color(0.1, 0.1, 0.6)
	$GameView/HUD/MPBarFill.size.x = $GameView/HUD/MPBar.size.x * mp_ratio
	$GameView/HUD/MPBarFill.color = Color(0.2, 0.4, 0.9)
	# XP bar
	var xp_ratio: float = float(my_experience) / float(max(my_max_experience, 1))
	$GameView/HUD/XPBar.color = Color(0.3, 0.3, 0.1)
	$GameView/HUD/XPBarFill.size.x = $GameView/HUD/XPBar.size.x * xp_ratio
	$GameView/HUD/XPBarFill.color = Color(0.9, 0.8, 0.1)

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
			if oid != client.my_object_id():
				var name_colour: int = data.get("name_colour", 0)
				var is_npc: bool = name_colour != 0
				var color: Color = Color(0.2, 0.6, 0.9) if not is_npc else Color(1.0, 0.7, 0.0)
				_ensure_player(oid, data.get("name", "?"), data.get("location", Vector2i.ZERO),
					data.get("direction", 0), color, false)
		Packets.S_OBJECT_MONSTER:
			var oid: int = data.get("object_id", 0)
			if data.get("dead", false):
				_remove_monster(oid)
			else:
				_ensure_monster(oid, data.get("name", "?"), data.get("location", Vector2i.ZERO),
					data.get("direction", 0), data.get("image", 0))
		Packets.S_OBJECT_NPC:
			_ensure_npc(data.get("object_id", 0), data.get("name", "?"),
				data.get("location", Vector2i.ZERO), data.get("image", 0))
		Packets.S_OBJECT_ITEM:
			_ensure_ground_item(data.get("object_id", 0), data.get("name", "?"),
				data.get("location", Vector2i.ZERO))
		Packets.S_OBJECT_GOLD:
			_ensure_ground_item(data.get("object_id", 0), "金 %d" % data.get("gold", 0),
				data.get("location", Vector2i.ZERO))
		Packets.S_OBJECT_WALK, Packets.S_OBJECT_RUN:
			var oid: int = data.get("object_id", 0)
			if oid == client.my_object_id():
				my_pos = data.get("location", my_pos)
			elif players.has(oid):
				_move_sprite(oid, data.get("location", Vector2i.ZERO))
			elif monsters.has(oid):
				_move_monster(oid, data.get("location", Vector2i.ZERO))
		Packets.S_OBJECT_TURN:
			var oid: int = data.get("object_id", 0)
			if players.has(oid):
				players[oid]["pos"] = data.get("location", players[oid]["pos"])
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
			_flash_entity(struck_oid)
		Packets.S_OBJECT_DIED:
			var died_oid: int = data.get("object_id", 0)
			_remove_monster(died_oid)
			_remove_sprite(died_oid)

# ---------------------------------------------------------------------------
# 用户位置回调
# ---------------------------------------------------------------------------

func _on_user_location(loc: Vector2i, dir: int) -> void:
	my_pos = loc
	my_dir = dir

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
	game_view.hide()
	login_panel.show()
	_on_characters_loaded(characters)
	status_label.text = "已登出"
	inventory_panel.hide()
	npc_dialog.hide()
	shop_panel.hide()
	players.clear()
	monsters.clear()
	npcs.clear()
	ground_items.clear()

func _on_return_to_login() -> void:
	game_view.hide()
	login_panel.show()
	status_label.text = "已返回登录界面"
	inventory_panel.hide()
	npc_dialog.hide()
	shop_panel.hide()

func _on_object_magic(data: Dictionary) -> void:
	var oid: int = data.get("object_id", 0)
	var spell: int = data.get("spell", 0)
	var level: int = data.get("level", 0)
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
	_remove_monster(object_id)
	_remove_sprite(object_id)

func _on_object_revived(object_id: int) -> void:
	pass

func _on_death() -> void:
	chat_log.append_text("[color=red]你已死亡! 按 R 键回城复活[/color]\n")

func _on_object_attack_visual(data: Dictionary) -> void:
	var atk_oid: int = data.get("object_id", 0)
	var dir: int = data.get("direction", 0)
	if players.has(atk_oid):
		players[atk_oid]["dir"] = dir
	_flash_entity(atk_oid)

func _flash_entity(object_id: int) -> void:
	var sprite: ColorRect = null
	if monsters.has(object_id):
		sprite = monsters[object_id]["sprite"]
	elif players.has(object_id):
		sprite = players[object_id]["sprite"]
	elif npcs.has(object_id):
		sprite = npcs[object_id]["sprite"]
	if sprite == null:
		return
	var orig_color: Color = sprite.color
	sprite.color = Color.RED
	var tween := create_tween()
	tween.tween_interval(0.1)
	tween.tween_property(sprite, "color", orig_color, 0.1)

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
			client.switch_group()
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
		btn.custom_minimum_size = Vector2(80, 24)
		btn.add_theme_font_size_override("font_size", 10)
		var spell_id: int = magic.get("spell", 0)
		btn.pressed.connect(func(): _cast_spell(spell_id, magic.get("name", "?")))
		skill_bar.add_child(btn)

func _cast_spell(spell_id: int, spell_name: String) -> void:
	client.magic(my_dir, spell_id)
	chat_log.append_text("[color=cyan]施放: %s[/color]\n" % spell_name)

func _update_skill_bar() -> void:
	pass

func _move_monster(object_id: int, pos: Vector2i) -> void:
	if monsters.has(object_id):
		var m: Dictionary = monsters[object_id]
		m["pos"] = pos
		m["sprite"].position = Vector2(pos.x * TILE + 2, pos.y * TILE + 2)
		m["label"].position = Vector2(pos.x * TILE, pos.y * TILE - 14)

# ---------------------------------------------------------------------------
# 移动输入
# ---------------------------------------------------------------------------

func _handle_movement_input() -> void:
	if not client.is_connected():
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
		else:
			client.walk(dir)
	# 攻击: 空格键
	if Input.is_action_just_pressed("ui_accept"):
		client.attack(my_dir)
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

func _ensure_player(object_id: int, pname: String, pos: Vector2i, direction: int, color: Color, is_self: bool) -> void:
	if players.has(object_id):
		var p: Dictionary = players[object_id]
		p["pos"] = pos
		p["name"] = pname
		p["sprite"].position = Vector2(pos.x * TILE + 2, pos.y * TILE + 2)
		p["label"].position = Vector2(pos.x * TILE, pos.y * TILE - 14)
		p["label"].text = pname
		return
	var sprite := ColorRect.new()
	sprite.color = color
	sprite.size = Vector2(TILE - 4, TILE - 4)
	sprite.position = Vector2(pos.x * TILE + 2, pos.y * TILE + 2)
	var label := Label.new()
	label.text = pname
	label.position = Vector2(pos.x * TILE, pos.y * TILE - 14)
	label.add_theme_font_size_override("font_size", 10)
	map_root.add_child(sprite)
	map_root.add_child(label)
	players[object_id] = {"sprite": sprite, "label": label, "pos": pos, "name": pname, "dir": direction}

func _ensure_monster(object_id: int, mname: String, pos: Vector2i, direction: int, image: int) -> void:
	if monsters.has(object_id):
		var m: Dictionary = monsters[object_id]
		m["pos"] = pos
		m["sprite"].position = Vector2(pos.x * TILE + 2, pos.y * TILE + 2)
		m["label"].position = Vector2(pos.x * TILE, pos.y * TILE - 14)
		return
	var sprite := ColorRect.new()
	sprite.color = Color(0.9, 0.2, 0.2)
	sprite.size = Vector2(TILE - 4, TILE - 4)
	sprite.position = Vector2(pos.x * TILE + 2, pos.y * TILE + 2)
	var label := Label.new()
	label.text = mname
	label.position = Vector2(pos.x * TILE, pos.y * TILE - 14)
	label.add_theme_font_size_override("font_size", 10)
	label.add_theme_color_override("font_color", Color.RED)
	map_root.add_child(sprite)
	map_root.add_child(label)
	monsters[object_id] = {"sprite": sprite, "label": label, "pos": pos, "name": mname, "image": image}

func _ensure_npc(object_id: int, nname: String, pos: Vector2i, image: int) -> void:
	if npcs.has(object_id):
		return
	var sprite := ColorRect.new()
	sprite.color = Color(1.0, 0.8, 0.2)
	sprite.size = Vector2(TILE - 4, TILE - 4)
	sprite.position = Vector2(pos.x * TILE + 2, pos.y * TILE + 2)
	var label := Label.new()
	label.text = nname
	label.position = Vector2(pos.x * TILE, pos.y * TILE - 14)
	label.add_theme_font_size_override("font_size", 10)
	label.add_theme_color_override("font_color", Color.GOLD)
	map_root.add_child(sprite)
	map_root.add_child(label)
	npcs[object_id] = {"sprite": sprite, "label": label, "pos": pos, "name": nname}

func _ensure_ground_item(object_id: int, iname: String, pos: Vector2i) -> void:
	if ground_items.has(object_id):
		return
	var sprite := ColorRect.new()
	sprite.color = Color(0.4, 0.8, 0.4)
	sprite.size = Vector2(TILE / 2, TILE / 2)
	sprite.position = Vector2(pos.x * TILE + TILE / 4, pos.y * TILE + TILE / 4)
	var label := Label.new()
	label.text = iname
	label.position = Vector2(pos.x * TILE, pos.y * TILE - 14)
	label.add_theme_font_size_override("font_size", 8)
	label.add_theme_color_override("font_color", Color.GREEN)
	map_root.add_child(sprite)
	map_root.add_child(label)
	ground_items[object_id] = {"sprite": sprite, "label": label, "pos": pos, "name": iname}

func _move_sprite(object_id: int, pos: Vector2i) -> void:
	if players.has(object_id):
		var p: Dictionary = players[object_id]
		p["pos"] = pos
		p["sprite"].position = Vector2(pos.x * TILE + 2, pos.y * TILE + 2)
		p["label"].position = Vector2(pos.x * TILE, pos.y * TILE - 14)

func _remove_sprite(object_id: int) -> void:
	if players.has(object_id):
		var p: Dictionary = players[object_id]
		if p["sprite"].get_parent():
			map_root.remove_child(p["sprite"])
		if p["label"].get_parent():
			map_root.remove_child(p["label"])
		players.erase(object_id)

func _remove_monster(object_id: int) -> void:
	if monsters.has(object_id):
		var m: Dictionary = monsters[object_id]
		if m["sprite"].get_parent():
			map_root.remove_child(m["sprite"])
		if m["label"].get_parent():
			map_root.remove_child(m["label"])
		monsters.erase(object_id)

func _remove_npc(object_id: int) -> void:
	if npcs.has(object_id):
		var n: Dictionary = npcs[object_id]
		if n["sprite"].get_parent():
			map_root.remove_child(n["sprite"])
		if n["label"].get_parent():
			map_root.remove_child(n["label"])
		npcs.erase(object_id)

func _remove_ground_item(object_id: int) -> void:
	if ground_items.has(object_id):
		var gi: Dictionary = ground_items[object_id]
		if gi["sprite"].get_parent():
			map_root.remove_child(gi["sprite"])
		if gi["label"].get_parent():
			map_root.remove_child(gi["label"])
		ground_items.erase(object_id)
