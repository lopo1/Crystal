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
## Web3: 收到待签名挑战 {"address","message","expires_in"}
signal web3_challenge_received(challenge: Dictionary)
## Web3: 登录结果 {"result": int, "characters": Array}
signal web3_login_result(result: Dictionary)

const Packets := preload("res://scripts/net/crystal_packets.gd")
const CrystalBinary := preload("res://scripts/net/crystal_binary.gd")
const Reader := CrystalBinary.Reader

var _stream: StreamPeerTCP
var _rx_buffer := PackedByteArray()
var _connected := false
var _stage := "none" # none | login | select | game
var _my_object_id: int = 0

const MAX_FRAME := 64 * 1024 * 1024

func _ready() -> void:
	_stream = StreamPeerTCP.new()

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
			disconnected.emit("连接错误")
			_stage = "none"
		StreamPeerTCP.STATUS_NONE, StreamPeerTCP.STATUS_CONNECTING:
			pass

func connect_to_server(host: String, port: int) -> void:
	if _stream.get_status() == StreamPeerTCP.STATUS_CONNECTED:
		_stream.disconnect_from_host()
	_stream.connect_to_host(host, port)

func disconnect_from_server() -> void:
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
	## 第一步：请求服务器为指定钱包地址签发登录挑战
	send(Packets.c_web3_challenge_request(address))

func web3_login(address: String, challenge: String, signature: PackedByteArray) -> void:
	## 第二步：用钱包对挑战签名后提交（signature 为 65 字节 r||s||v，EIP-191）
	_stage = "login"
	send(Packets.c_web3_login(address, challenge, signature))

func new_character(char_name: String, gender: int, class_id: int) -> void:
	send(Packets.c_new_character(char_name, gender, class_id))

func delete_character(index: int) -> void:
	send(Packets.c_delete_character(index))

func start_game(index: int) -> void:
	send(Packets.c_start_game(index))

func logout() -> void:
	send(Packets.c_logout())

func walk(direction: int) -> void:
	send(Packets.c_walk(direction))

func run(direction: int) -> void:
	send(Packets.c_run(direction))

func turn(direction: int) -> void:
	send(Packets.c_turn(direction))

func chat(message: String) -> void:
	send(Packets.c_chat(message))

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
			# 畸形帧: 丢弃整个缓冲（同 C#）
			_rx_buffer = PackedByteArray()
			return
		if _rx_buffer.size() < len:
			return
		var id_u: int = _rx_buffer.decode_u16(2)
		var id: int = id_u - 65536 if id_u >= 32768 else id_u
		var payload := _rx_buffer.slice(4, len)
		_rx_buffer = _rx_buffer.slice(len, _rx_buffer.size())
		_dispatch(id, payload)

func _dispatch(id: int, payload: PackedByteArray) -> void:
	var packet := Packets.decode_server_packet(id, payload)
	server_packet.emit(packet)
	var data: Dictionary = packet.get("data", {})
	match id:
		Packets.S_CONNECTED:
			# 连接建立: 服务器要求客户端上报版本
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
			login_result.emit(-data.get("result", 0)) # 负数表示建角色失败
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
			entered_world.emit(data)
		Packets.S_OBJECT_CHAT, Packets.S_CHAT:
			var text: String = data.get("text", data.get("message", ""))
			var oid: int = data.get("object_id", 0)
			chat_line.emit(oid, text)

func my_object_id() -> int:
	return _my_object_id