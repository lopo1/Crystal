extends Node
## 游戏网络客户端: TCP 连接 + 帧收发 + 服务器包分发。
## 用法: 实例化后 connect 信号，调用 connect_to_server()。

signal connected_ok()
signal disconnected(reason: String)
signal server_packet(packet: Dictionary) ## {"id": int, "data": Dictionary}
signal login_result(result: int)
signal characters_loaded(characters: Array) ## Array[Dictionary] SelectInfo
signal entered_world(user_info: Dictionary)
signal chat_line(object_id: int, text: String)
signal user_location(location: Vector2i, direction: int)
signal health_changed(hp: int, mp: int)
signal gained_experience(amount: int)
signal level_changed(level: int)
signal damage_indicator(damage: int, type: int, object_id: int)
signal object_died(object_id: int)
signal object_revived(object_id: int)
signal struck(attacker_id: int)
signal gained_gold(amount: int)
signal lost_gold(amount: int)
signal gained_item(item: Dictionary)
signal death()
signal user_slots_refresh(inventory: Array, equipment: Array)
signal magics_loaded(magics: Array)
## Web3: 收到待签名挑战 {"address","message","expires_in"}
signal web3_challenge_received(challenge: Dictionary)
## Web3: 登录结果 {"result": int, "characters": Array, "session_token": String}
signal web3_login_result(result: Dictionary)

const Packets := preload("res://scripts/net/crystal_packets.gd")
const CrystalBinary := preload("res://scripts/net/crystal_binary.gd")
const CrystalPackets := preload("res://scripts/net/crystal_packets.gd")
const Reader := CrystalBinary.Reader

## Client packet IDs (matches Rust ClientPacketId enum)
const ClientPacketId := {
	"CLIENT_VERSION": 0,
	"DISCONNECT": 1,
	"KEEP_ALIVE": 2,
	"NEW_ACCOUNT": 3,
	"CHANGE_PASSWORD": 4,
	"LOGIN": 5,
	"NEW_CHARACTER": 6,
	"DELETE_CHARACTER": 7,
	"START_GAME": 8,
	"LOGOUT": 9,
	"TURN": 10,
	"WALK": 11,
	"RUN": 12,
	"CHAT": 13,
	"ATTACK": 47,
	"RANGE_ATTACK": 48,
	"MAGIC": 58,
	"MAGIC_KEY": 59,
	"PICK_UP": 35,
	"EQUIP_ITEM": 18,
	"REMOVE_ITEM": 19,
	"USE_ITEM": 22,
	"DROP_ITEM": 23,
	"CALL_NPC": 50,
	"BUY_ITEM": 51,
	"SELL_ITEM": 52,
	"TOWN_RELOCATE": 68,
}

var _stream: StreamPeerTCP
var _rx_buffer := PackedByteArray()
var _connected := false
var _stage := "none" # none | login | select | game
var _my_object_id: int = 0
var _keepalive_timer: Timer

const MAX_FRAME := 64 * 1024 * 1024

func _ready() -> void:
	_stream = StreamPeerTCP.new()
	_keepalive_timer = Timer.new()
	_keepalive_timer.wait_time = 30.0
	_keepalive_timer.autostart = false
	_keepalive_timer.timeout.connect(_send_keepalive)
	add_child(_keepalive_timer)

func _process(_delta: float) -> void:
	if _stream == null:
		return
	_stream.poll()
	match _stream.get_status():
		StreamPeerTCP.STATUS_CONNECTED:
			if not _connected:
				_connected = true
				connected_ok.emit()
			_receive()
		StreamPeerTCP.STATUS_ERROR:
			_connected = false
			_keepalive_timer.stop()
			disconnected.emit("连接错误")
			_stage = "none"
		StreamPeerTCP.STATUS_NONE, StreamPeerTCP.STATUS_CONNECTING:
			pass

func connect_to_server(host: String, port: int) -> void:
	if _stream.get_status() == StreamPeerTCP.STATUS_CONNECTED:
		_stream.disconnect_from_host()
	_stream.connect_to_host(host, port)

func disconnect_from_server() -> void:
	_keepalive_timer.stop()
	if _stream != null:
		_stream.disconnect_from_host()
	_connected = false

func is_connected() -> bool:
	return _connected and _stream != null and _stream.get_status() == StreamPeerTCP.STATUS_CONNECTED

func send(packet) -> void:
	if not is_connected():
		return
	var frame: PackedByteArray = packet.encode()
	_stream.put_data(frame)

# ---------------------------------------------------------------------------
# 心跳包
# ---------------------------------------------------------------------------

func _send_keepalive() -> void:
	if is_connected():
		send(Packets.c_keep_alive(Time.get_unix_time_from_system()))

# ---------------------------------------------------------------------------
# 客户端动作
# ---------------------------------------------------------------------------

func send_client_version(hash: PackedByteArray) -> void:
	send(Packets.c_client_version(hash))

func login(account: String, password: String) -> void:
	_stage = "login"
	send(Packets.c_login(account, password))

func new_account(account: String, password: String, email: String, name: String) -> void:
	send(Packets.c_new_account(account, password, email, name))

func web3_request_challenge(address: String) -> void:
	send(Packets.c_web3_challenge_request(address))

func web3_login(address: String, challenge: String, signature: PackedByteArray) -> void:
	_stage = "login"
	send(Packets.c_web3_login(address, challenge, signature))

func new_character(char_name: String, gender: int, class_id: int) -> void:
	send(Packets.c_new_character(char_name, gender, class_id))

func delete_character(index: int) -> void:
	send(Packets.c_delete_character(index))

func start_game(index: int) -> void:
	send(Packets.c_start_game(index))

func logout() -> void:
	_keepalive_timer.stop()
	send(Packets.c_logout())

func walk(direction: int) -> void:
	send(Packets.c_walk(direction))

func run(direction: int) -> void:
	send(Packets.c_run(direction))

func turn(direction: int) -> void:
	send(Packets.c_turn(direction))

func chat(message: String) -> void:
	send(Packets.c_chat(message))

func attack(direction: int, spell: int = 0) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.ATTACK
	p.write_fn = func(w) -> void:
		w.write_u8(direction)
		w.write_u8(spell)
	send(p)

func magic(direction: int, spell_id: int, target_id: int = 0) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.MAGIC
	p.write_fn = func(w) -> void:
		w.write_u32(_my_object_id)
		w.write_u8(spell_id)
		w.write_u8(direction)
		w.write_u32(target_id)
		CrystalPackets.write_point(w, Vector2i.ZERO)
		w.write_bool(false)
	send(p)

func magic_key(spell: int, key: int, old_key: int) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.MAGIC_KEY
	p.write_fn = func(w) -> void:
		w.write_u8(spell)
		w.write_u8(key)
		w.write_u8(old_key)
	send(p)

func call_npc(object_id: int) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.CALL_NPC
	p.write_fn = func(w) -> void:
		w.write_u32(object_id)
	send(p)

func pick_up() -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.PICK_UP
	send(p)

func accept_death() -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.TOWN_RELOCATE
	send(p)

func town_revive() -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.TOWN_RELOCATE
	send(p)

func use_item(unique_id: int, grid: int = 1) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.USE_ITEM
	p.write_fn = func(w) -> void:
		w.write_u64(unique_id)
		w.write_u8(grid)
	send(p)

func equip_item(unique_id: int, grid: int = 1, to: int = -1) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.EQUIP_ITEM
	p.write_fn = func(w) -> void:
		w.write_u8(grid)
		w.write_u64(unique_id)
		w.write_i32(to)
	send(p)

func buy_item(item_index: int, count: int = 1, panel_type: int = 0) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.BUY_ITEM
	p.write_fn = func(w) -> void:
		w.write_u64(item_index)
		w.write_u16(count)
		w.write_u8(panel_type)
	send(p)

func sell_item(unique_id: int, count: int = 1) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.SELL_ITEM
	p.write_fn = func(w) -> void:
		w.write_u64(unique_id)
		w.write_u16(count)
	send(p)

func drop_item(unique_id: int, count: int = 1) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.DROP_ITEM
	p.write_fn = func(w) -> void:
		w.write_u64(unique_id)
		w.write_u16(count)
		w.write_bool(false)
	send(p)

func take_back_hero_item(from: int = 0, to: int = 0) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 32
	p.write_fn = func(w) -> void:
		w.write_i32(from)
		w.write_i32(to)
	send(p)

# ---------------------------------------------------------------------------
# 接收与分发
# ---------------------------------------------------------------------------

func _receive() -> void:
	var available := _stream.get_available_bytes()
	if available > 0:
		var data := _stream.get_data(available)
		if data[0] == OK:
			_rx_buffer.append_array(data[1])
	# 解析帧
	while true:
		if _rx_buffer.size() < 4:
			return
		var len: int = _rx_buffer.decode_u16(0)
		if len < 4 or len > MAX_FRAME:
			_rx_buffer = PackedByteArray()
			return
		if _rx_buffer.size() < len:
			return
		var id_u: int = _rx_buffer.decode_u16(2)
		var id: int = id_u - 65536 if id_u >= 32768 else id_u
		var payload := _rx_buffer.slice(4, len)
		_rx_buffer = _rx_buffer.slice(len, _rx_buffer.size())
		# NPCGoods (102) 在 Rust 服务端标记为 COMPRESSED，载荷是 gzip 流
		if id == Packets.S_NPC_GOODS:
			var gz := StreamPeerGZip.new()
			if gz.start_decompress() == OK:
				gz.put_data(payload)
				var avail := gz.get_available_bytes()
				if avail > 0:
					var result := gz.get_data(avail)
					if result[0] == OK:
						payload = result[1]
		_dispatch(id, payload)

func _dispatch(id: int, payload: PackedByteArray) -> void:
	var packet := Packets.decode_server_packet(id, payload)
	server_packet.emit(packet)
	var data: Dictionary = packet.get("data", {})
	match id:
		Packets.S_CONNECTED:
			pass
		Packets.S_CLIENT_VERSION:
			if data.get("result", 0) == 1:
				_stage = "login"
			else:
				disconnected.emit("版本不匹配")
		Packets.S_LOGIN:
			login_result.emit(data.get("result", 0))
		Packets.S_LOGIN_SUCCESS:
			_stage = "select"
			characters_loaded.emit(data.get("characters", []))
		Packets.S_WEB3_CHALLENGE:
			web3_challenge_received.emit(data)
		Packets.S_WEB3_LOGIN_RESULT:
			if data.get("result", 1) == 0:
				_stage = "select"
				characters_loaded.emit(data.get("characters", []))
			web3_login_result.emit(data)
		Packets.S_NEW_CHARACTER:
			login_result.emit(-data.get("result", 0))
		Packets.S_NEW_CHARACTER_SUCCESS:
			characters_loaded.emit([data.get("char_info", {})])
		Packets.S_DISCONNECT:
			disconnected.emit("服务器断开: %d" % data.get("reason", 0))
		Packets.S_START_GAME:
			if data.get("result", 0) != 0:
				disconnected.emit("进入游戏失败: %d" % data.get("result", 0))
		Packets.S_USER_INFORMATION:
			_my_object_id = data.get("object_id", 0)
			_stage = "game"
			_keepalive_timer.start()
			entered_world.emit(data)
			magics_loaded.emit(data.get("magics", []))
		Packets.S_USER_SLOTS_REFRESH:
			user_slots_refresh.emit(data.get("inventory", []), data.get("equipment", []))
		Packets.S_USER_LOCATION:
			user_location.emit(data.get("location", Vector2i.ZERO), data.get("direction", 0))
		Packets.S_OBJECT_CHAT, Packets.S_CHAT:
			var text: String = data.get("text", data.get("message", ""))
			var oid: int = data.get("object_id", 0)
			chat_line.emit(oid, text)
		Packets.S_HEALTH_CHANGED:
			health_changed.emit(data.get("hp", 0), data.get("mp", 0))
		Packets.S_GAIN_EXPERIENCE:
			gained_experience.emit(data.get("amount", 0))
		Packets.S_LEVEL_CHANGED:
			level_changed.emit(data.get("level", 1))
		Packets.S_DAMAGE_INDICATOR:
			damage_indicator.emit(data.get("damage", 0), data.get("type", 0), data.get("object_id", 0))
		Packets.S_OBJECT_DIED:
			object_died.emit(data.get("object_id", 0))
		Packets.S_DEATH:
			death.emit()
		Packets.S_REVIVED:
			pass
		Packets.S_OBJECT_REVIVED:
			object_revived.emit(data.get("object_id", 0))
		Packets.S_STRUCK:
			struck.emit(data.get("attacker_id", 0))
		Packets.S_GAINED_GOLD:
			gained_gold.emit(data.get("gold", 0))
		Packets.S_LOSE_GOLD:
			lost_gold.emit(data.get("gold", 0))
		Packets.S_GAINED_ITEM:
			gained_item.emit(data.get("item", {}))
		Packets.S_OBJECT_ATTACK:
			pass
		Packets.S_OBJECT_STRUCK:
			pass
		Packets.S_NPC_GOODS:
			pass
		Packets.S_NPC_SELL:
			pass
		Packets.S_NPC_REPAIR:
			pass
		Packets.S_NPC_STORAGE:
			pass

func my_object_id() -> int:
	return _my_object_id
