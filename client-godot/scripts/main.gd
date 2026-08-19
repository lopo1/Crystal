extends Node
## 主场景控制: 登录 → 角色选择 → 进入世界(网格+移动+聊天)。
## 运行前提: 已启动 Rust 服务器 (`cargo run -p crystal-server`)。

const GameClient := preload("res://scripts/net/game_client.gd")
const Packets := preload("res://scripts/net/crystal_packets.gd")
const Web3Wallet := preload("res://scripts/net/web3_wallet.gd")

const TILE := 32 # 每个格子像素（垂直切片用色块代替贴图）

var client: GameClient
var wallet := Web3Wallet.new() # 默认 JS (MetaMask)；开发模式可切 RPC
var _wallet_address := ""
var _pending_challenge := ""

# 游戏内状态
var my_pos := Vector2i(400, 400)
var my_dir := 0
var players := {} # object_id -> {sprite: Node2D, pos: Vector2i}

@onready var login_panel: Control = $LoginPanel
@onready var game_view: Control = $GameView
@onready var map_root: Node2D = $GameView/MapRoot
@onready var chat_log: RichTextLabel = $GameView/ChatLog
@onready var chat_input: LineEdit = $GameView/ChatInput
@onready var status_label: Label = $LoginPanel/StatusLabel

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
	$LoginPanel/ConnectButton.pressed.connect(_on_connect_pressed)
	$LoginPanel/NewAccountButton.pressed.connect(_on_new_account_pressed)
	$LoginPanel/StartGameButton.pressed.connect(_on_start_game_pressed)
	$LoginPanel/CreateButton.pressed.connect(_on_create_pressed)
	$LoginPanel/WalletLoginButton.pressed.connect(_on_wallet_login_pressed)
	chat_input.text_submitted.connect(_on_chat_submitted)
	game_view.hide()

func _process(_delta: float) -> void:
	if game_view.visible:
		_handle_movement_input()
	# 相机跟随
	if my_pos != Vector2i.ZERO:
		$GameView/Camera2D.position = Vector2(my_pos.x * TILE + TILE / 2.0, my_pos.y * TILE + TILE / 2.0)
	$GameView/StatusBar.text = "位置: (%d, %d)" % [my_pos.x, my_pos.y]

# ---------------------------------------------------------------------------
# 登录
# ---------------------------------------------------------------------------

func _on_connect_pressed() -> void:
	var host: String = $LoginPanel/ServerLineEdit.text
	if host == "":
		host = "127.0.0.1"
	var port: int = int($LoginPanel/PortLineEdit.text)
	status_label.text = "连接 %s:%d ..." % [host, port]
	client.connect_to_server(host, port)

func _on_connected() -> void:
	status_label.text = "已连接，上报版本..."
	client.send_client_version(PackedByteArray())
	# 若是钱包登录流程，连接后就继续取地址→请求挑战
	if _wallet_login_pending:
		_wallet_login_pending = false
		_begin_wallet_login()

var _wallet_login_pending := false

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
	## 第一步：请求服务器为钱包地址签发挑战
	status_label.text = "请求登录挑战..."
	client.web3_request_challenge(_wallet_address)

func _on_web3_challenge(ch: Dictionary) -> void:
	## 第二步：让钱包对挑战 personal_sign，再提交
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
		return # Success 由 characters_loaded 处理
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

func _class_name(class_id: int) -> String:
	return ["战士", "法师", "道士", "刺客", "弓手"][class_id] if class_id < 5 else "?"

# ---------------------------------------------------------------------------
# 游戏内
# ---------------------------------------------------------------------------

func _on_entered_world(ui: Dictionary) -> void:
	login_panel.hide()
	game_view.show()
	my_pos = ui.get("location", Vector2i(400, 400))
	my_dir = ui.get("direction", 0)
	status_label.text = "进入世界: %s" % ui.get("name", "?")
	# 先渲染网格，再生成玩家精灵（网格会清空子节点）
	_render_grid()
	_ensure_player(ui.get("object_id", 0), ui.get("name", "我"), Vector2i(my_pos), ui.get("direction", 0), Color.GREEN)

func _render_grid() -> void:
	for child in map_root.get_children():
		child.queue_free()
	var info: Dictionary = _map_info
	var w: int = info.get("width", 100)
	var h: int = info.get("height", 100)
	# 只渲染视野附近 60x40 格
	var cam := $GameView/Camera2D
	for x in range(max(0, my_pos.x - 30), min(w, my_pos.x + 31)):
		for y in range(max(0, my_pos.y - 20), min(h, my_pos.y + 21)):
			var cell := ColorRect.new()
			cell.color = Color(0.12, 0.16, 0.2) if (x + y) % 2 == 0 else Color(0.14, 0.18, 0.22)
			cell.position = Vector2(x * TILE, y * TILE)
			cell.size = Vector2(TILE, TILE)
			map_root.add_child(cell)

var _map_info: Dictionary = {}

func _on_server_packet(packet: Dictionary) -> void:
	var id: int = packet.get("id", -1)
	var data: Dictionary = packet.get("data", {})
	match id:
		Packets.S_MAP_INFORMATION:
			_map_info = data
		Packets.S_NEW_MAP_INFO:
			_map_info = data.get("info", _map_info)
		Packets.S_OBJECT_PLAYER:
			if data.get("object_id", 0) != client.my_object_id():
				_ensure_player(
					data.get("object_id", 0),
					data.get("name", "?"),
					data.get("location", Vector2i.ZERO),
					data.get("direction", 0),
					Color(0.2, 0.6, 0.9)
				)
		Packets.S_OBJECT_WALK, Packets.S_OBJECT_RUN:
			var oid: int = data.get("object_id", 0)
			if oid != client.my_object_id():
				_move_sprite(oid, data.get("location", Vector2i.ZERO))
		Packets.S_OBJECT_TURN:
			var oid2: int = data.get("object_id", 0)
			if players.has(oid2):
				players[oid2]["pos"] = data.get("location", players[oid2]["pos"])
		Packets.S_OBJECT_REMOVE:
			_remove_sprite(data.get("object_id", 0))

func _on_chat_line(object_id: int, text: String) -> void:
	var name := "?" 
	if players.has(object_id):
		name = players[object_id].get("name", "?")
	if object_id == client.my_object_id():
		name = "我"
	chat_log.append_text("[b]%s[/b]: %s\n" % [name, text])

func _on_chat_submitted(text: String) -> void:
	if text != "":
		client.chat(text)
	chat_input.clear()

func _handle_movement_input() -> void:
	if not client.is_connected():
		return
	var dir := -1
	if Input.is_action_just_pressed("ui_up"): dir = 0
	elif Input.is_action_just_pressed("ui_right"): dir = 2
	elif Input.is_action_just_pressed("ui_down"): dir = 4
	elif Input.is_action_just_pressed("ui_left"): dir = 6
	if dir >= 0:
		my_dir = dir
		client.walk(dir)

func _ensure_player(object_id: int, name: String, pos: Vector2i, direction: int, color: Color) -> void:
	if players.has(object_id):
		return
	var sprite := ColorRect.new()
	sprite.color = color
	sprite.size = Vector2(TILE - 4, TILE - 4)
	sprite.position = Vector2(pos.x * TILE + 2, pos.y * TILE + 2)
	var label := Label.new()
	label.text = name
	label.position = Vector2(pos.x * TILE, pos.y * TILE - 14)
	label.add_theme_font_size_override("font_size", 10)
	map_root.add_child(sprite)
	map_root.add_child(label)
	players[object_id] = {"sprite": sprite, "label": label, "pos": pos}

func _move_sprite(object_id: int, pos: Vector2i) -> void:
	if not players.has(object_id):
		return
	var p: Dictionary = players[object_id]
	p["pos"] = pos
	var sprite: ColorRect = p["sprite"]
	sprite.position = Vector2(pos.x * TILE + 2, pos.y * TILE + 2)
	var label: Label = p["label"]
	label.position = Vector2(pos.x * TILE, pos.y * TILE - 14)

func _remove_sprite(object_id: int) -> void:
	if players.has(object_id):
		map_root.remove_child(players[object_id]["sprite"])
		map_root.remove_child(players[object_id]["label"])
		players.erase(object_id)