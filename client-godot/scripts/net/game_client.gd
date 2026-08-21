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
signal equip_result(grid: int, unique_id: int, success: bool)
signal use_item_result(unique_id: int, success: bool)
signal delete_item(unique_id: int, count: int)
signal colour_changed(name_colour: int)
signal player_inspect(info: Dictionary)
signal logout_success(characters: Array)
signal return_to_login()
signal attack_mode_changed(mode: int)
signal peace_mode_changed(mode: int)
signal object_harvest(object_id: int)
signal object_harvested(object_id: int)
signal opendoor(door_index: int, close: bool)
signal trade_request(name: String)
signal trade_accept(name: String)
signal trade_gold(amount: int)
signal trade_items(items: Array)
signal trade_confirmed()
signal trade_cancelled(unlock: bool)
signal market_list(listings: Array, pages: int, user_mode: bool)
signal consign_result(unique_id: int, success: bool)
signal market_fail(reason: int)
signal market_success(message: String)
signal mailbox_loaded(mails: Array)
signal mail_sent(result: int)
signal parcel_collected(result: int)
signal npc_refine(rate: float, refining: bool)
signal object_magic(data: Dictionary)
signal range_attack(target_id: int, target: Vector2i, spell: int)
signal object_range_attack(data: Dictionary)
signal new_magic(magic: Dictionary)
signal magic_leveled(spell: int, level: int, experience: int)
signal group_switched(allow: bool)
signal delete_member(name: String)
signal group_invite(name: String)
signal add_member(name: String)
signal friend_update(friends: Array)
signal marriage_request(name: String)
signal lover_update(data: Dictionary)
signal mentor_update(data: Dictionary)
signal request_reincarnation()
signal fishing_update(data: Dictionary)
signal object_hidden(object_id: int, hidden: bool)
signal base_stats_info(data: Dictionary)
signal storage_unlock_result(result: int, has_password: bool)
signal storage_password_result(result: int)
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
	"HARVEST": 49,
	"CHANGE_A_MODE": 44,
	"OPENDOOR": 136,
	"DEPOSIT_TRADE_ITEM": 30,
	"RETRIEVE_TRADE_ITEM": 31,
	"CONSIGN_ITEM": 70,
	"MARKET_SEARCH": 71,
	"MARKET_REFRESH": 72,
	"MARKET_PAGE": 73,
	"MARKET_BUY": 74,
	"MARKET_GET_BACK": 75,
	"MARKET_SELL_NOW": 76,
	"SEND_MAIL": 117,
	"READ_MAIL": 118,
	"COLLECT_PARCEL": 119,
	"DELETE_MAIL": 120,
	"TRADE_REQUEST": 96,
	"TRADE_REPLY": 97,
	"TRADE_GOLD": 98,
	"TRADE_CONFIRM": 99,
	"TRADE_CANCEL": 100,
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

func is_server_connected() -> bool:
	return _connected and _stream != null and _stream.get_status() == StreamPeerTCP.STATUS_CONNECTED

func send(packet) -> void:
	if not is_server_connected():
		return
	var frame: PackedByteArray = packet.encode()
	_stream.put_data(frame)

# ---------------------------------------------------------------------------
# 心跳包
# ---------------------------------------------------------------------------

func _send_keepalive() -> void:
	if is_server_connected():
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

## 远程攻击（弓手）：direction 朝向 + 目标对象与目标位置（同 C# C.RangeAttack）
func range_attack(direction: int, target_id: int, location: Vector2i, target_location: Vector2i) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.RANGE_ATTACK
	p.write_fn = func(w) -> void:
		w.write_u8(direction)
		CrystalPackets.write_point(w, location)
		w.write_u32(target_id)
		CrystalPackets.write_point(w, target_location)
	send(p)

## 采集（割肉）：朝向方向割取尸体
func harvest(direction: int) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.HARVEST
	p.write_fn = func(w) -> void:
		w.write_u8(direction)
	send(p)

## 切换攻击模式（0和平/1编组/2行会/3敌对行会/4红名/5全体）
func change_a_mode(mode: int) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.CHANGE_A_MODE
	p.write_fn = func(w) -> void:
		w.write_u8(mode)
	send(p)

## 开城门
func opendoor(door_index: int) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.OPENDOOR
	p.write_fn = func(w) -> void:
		w.write_u8(door_index)
	send(p)

## 发起面对面交易（须面对目标玩家）
func trade_request() -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.TRADE_REQUEST
	send(p)

## 回应交易邀请
func trade_reply(accept: bool) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.TRADE_REPLY
	p.write_fn = func(w) -> void:
		w.write_bool(accept)
	send(p)

## 放入交易金币
func trade_gold(amount: int) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.TRADE_GOLD
	p.write_fn = func(w) -> void:
		w.write_u32(amount)
	send(p)

## 放入/取回交易物品（from/to 为槽位）
func trade_deposit_item(from_slot: int, to_slot: int) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.DEPOSIT_TRADE_ITEM
	p.write_fn = func(w) -> void:
		w.write_i32(from_slot)
		w.write_i32(to_slot)
	send(p)

func trade_retrieve_item(from_slot: int, to_slot: int) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.RETRIEVE_TRADE_ITEM
	p.write_fn = func(w) -> void:
		w.write_i32(from_slot)
		w.write_i32(to_slot)
	send(p)

## 确认/解锁交易
func trade_confirm(locked: bool) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.TRADE_CONFIRM
	p.write_fn = func(w) -> void:
		w.write_bool(locked)
	send(p)

func trade_cancel() -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.TRADE_CANCEL
	send(p)

## 寄售行：上架/浏览/购买/取回
func consign_item(unique_id: int, price: int, panel_type: int = 0) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.CONSIGN_ITEM
	p.write_fn = func(w) -> void:
		w.write_u64(unique_id)
		w.write_u32(price)
		w.write_u8(panel_type)
	send(p)

func market_search(match_text: String, item_type: int = 0) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.MARKET_SEARCH
	p.write_fn = func(w) -> void:
		w.write_string(match_text)
		w.write_u8(item_type)
		w.write_bool(false)
		w.write_i16(0)
		w.write_i16(0)
		w.write_u8(0)
	send(p)

func market_page(page: int) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.MARKET_PAGE
	p.write_fn = func(w) -> void:
		w.write_i32(page)
	send(p)

func market_refresh() -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.MARKET_REFRESH
	send(p)

func market_buy(auction_id: int, bid_price: int) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.MARKET_BUY
	p.write_fn = func(w) -> void:
		w.write_u64(auction_id)
		w.write_u32(bid_price)
	send(p)

func market_get_back(auction_id: int, mode: int = 0) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.MARKET_GET_BACK
	p.write_fn = func(w) -> void:
		w.write_u8(mode)
		w.write_u64(auction_id)
	send(p)

## 邮件：寄送/阅读/领取/删除
func send_mail(to_name: String, message: String, gold: int = 0, item_uids: Array = []) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.SEND_MAIL
	p.write_fn = func(w) -> void:
		w.write_string(to_name)
		w.write_string(message)
		w.write_u32(gold)
		for i in range(5):
			var uid: int = item_uids[i] if i < item_uids.size() else 0
			w.write_u64(uid)
		w.write_bool(false)
	send(p)

func read_mail(mail_id: int) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.READ_MAIL
	p.write_fn = func(w) -> void:
		w.write_u64(mail_id)
	send(p)

func collect_parcel(mail_id: int) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.COLLECT_PARCEL
	p.write_fn = func(w) -> void:
		w.write_u64(mail_id)
	send(p)

func delete_mail(mail_id: int) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = ClientPacketId.DELETE_MAIL
	p.write_fn = func(w) -> void:
		w.write_u64(mail_id)
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

func inspect(player_name: String) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 42
	p.write_fn = func(w) -> void:
		w.write_string(player_name)
	send(p)

func move_item(grid: int, from: int, to: int) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 14
	p.write_fn = func(w) -> void:
		w.write_u8(grid)
		w.write_i32(from)
		w.write_i32(to)
	send(p)

func merge_item(grid: int, from: int, to: int) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 17
	p.write_fn = func(w) -> void:
		w.write_u8(grid)
		w.write_i32(from)
		w.write_i32(to)
	send(p)

func split_item(grid: int, from: int, count: int) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 33
	p.write_fn = func(w) -> void:
		w.write_u8(grid)
		w.write_i32(from)
		w.write_i32(count)
	send(p)

func drop_gold(amount: int) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 34
	p.write_fn = func(w) -> void:
		w.write_u32(amount)
	send(p)

func store_item(unique_id: int) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 15
	p.write_fn = func(w) -> void:
		w.write_u64(unique_id)
	send(p)

func take_back_item(from: int, to: int) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 16
	p.write_fn = func(w) -> void:
		w.write_i32(from)
		w.write_i32(to)
	send(p)

func repair_item(unique_id: int) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 54
	p.write_fn = func(w) -> void:
		w.write_u64(unique_id)
	send(p)

func switch_group() -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 59
	send(p)

func add_group_member(name: String) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 60
	p.write_fn = func(w) -> void:
		w.write_string(name)
	send(p)

func del_group_member(name: String) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 61
	p.write_fn = func(w) -> void:
		w.write_string(name)
	send(p)

func group_invite_response(accept: bool) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 62
	p.write_fn = func(w) -> void:
		w.write_bool(accept)
	send(p)

func change_attack_mode() -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 44
	send(p)

func change_peace_mode() -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 45
	send(p)

func add_friend(name: String) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 127
	p.write_fn = func(w) -> void:
		w.write_string(name)
	send(p)

func remove_friend(name: String) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 128
	p.write_fn = func(w) -> void:
		w.write_string(name)
	send(p)

func refresh_friends() -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 129
	send(p)

func spell_toggle(spell: int) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 69
	p.write_fn = func(w) -> void:
		w.write_u8(spell)
	send(p)

func fishing_cast() -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 102
	send(p)

func fishing_change_autocast() -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 103
	send(p)

func accept_reincarnation() -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 108
	send(p)

func cancel_reincarnation() -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 109
	send(p)

func harvest(direction: int) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 49
	p.write_fn = func(w) -> void:
		w.write_u8(direction)
	send(p)

func request_map_info() -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 36
	send(p)

func request_item_info() -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 39
	send(p)

func request_monster_info() -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 37
	send(p)

func teleport_to_npc(object_id: int) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 40
	p.write_fn = func(w) -> void:
		w.write_u32(object_id)
	send(p)

func set_storage_password(password: String) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 151
	p.write_fn = func(w) -> void:
		w.write_string(password)
	send(p)

func remove_storage_password(password: String) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 152
	p.write_fn = func(w) -> void:
		w.write_string(password)
	send(p)

func unlock_storage(password: String) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 150
	p.write_fn = func(w) -> void:
		w.write_string(password)
	send(p)

func request_guild_info() -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 83
	send(p)

func request_user_name(name: String) -> void:
	var p := CrystalBinary.Packet.new()
	p.packet_id = 77
	p.write_fn = func(w) -> void:
		w.write_string(name)
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
			var decompressed: PackedByteArray = payload.decompress_dynamic(-1, FileAccess.COMPRESSION_GZIP)
			if decompressed.size() > 0:
				payload = decompressed
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
		Packets.S_EQUIP_ITEM:
			equip_result.emit(data.get("grid", 0), data.get("unique_id", 0), data.get("success", false))
		Packets.S_USE_ITEM:
			use_item_result.emit(data.get("unique_id", 0), data.get("success", false))
		Packets.S_DELETE_ITEM:
			delete_item.emit(data.get("unique_id", 0), data.get("count", 0))
		Packets.S_COLOUR_CHANGED:
			colour_changed.emit(data.get("name_colour", 0))
		Packets.S_PLAYER_INSPECT:
			player_inspect.emit(data)
		Packets.S_LOG_OUT_SUCCESS:
			_stage = "select"
			_keepalive_timer.stop()
			logout_success.emit(data.get("characters", []))
		Packets.S_RETURN_TO_LOGIN:
			_stage = "none"
			_keepalive_timer.stop()
			return_to_login.emit()
		Packets.S_CHANGE_A_MODE:
			attack_mode_changed.emit(data.get("mode", 0))
		Packets.S_CHANGE_P_MODE:
			peace_mode_changed.emit(data.get("mode", 0))
		Packets.S_OBJECT_HARVEST:
			object_harvest.emit(data.get("object_id", 0))
		Packets.S_OBJECT_HARVESTED:
			object_harvested.emit(data.get("object_id", 0))
		Packets.S_OPENDOOR:
			opendoor.emit(data.get("door_index", 0), data.get("close", false))
		Packets.S_TRADE_REQUEST:
			trade_request.emit(data.get("name", ""))
		Packets.S_TRADE_ACCEPT:
			trade_accept.emit(data.get("name", ""))
		Packets.S_TRADE_GOLD:
			trade_gold.emit(data.get("amount", 0))
		Packets.S_TRADE_ITEM:
			trade_items.emit(data.get("trade_items", []))
		Packets.S_TRADE_CONFIRM:
			trade_confirmed.emit()
		Packets.S_TRADE_CANCEL:
			trade_cancelled.emit(data.get("unlock", true))
		Packets.S_DEPOSIT_TRADE_ITEM, Packets.S_RETRIEVE_TRADE_ITEM:
			pass # 槽位回执：UI 层可经 server_packet 信号取用
		Packets.S_NPC_MARKET:
			market_list.emit(data.get("listings", []), data.get("pages", 0), data.get("user_mode", false))
		Packets.S_CONSIGN_ITEM:
			consign_result.emit(data.get("unique_id", 0), data.get("success", false))
		Packets.S_MARKET_FAIL:
			market_fail.emit(data.get("reason", 0))
		Packets.S_MARKET_SUCCESS:
			market_success.emit(data.get("message", ""))
		Packets.S_RECEIVE_MAIL:
			mailbox_loaded.emit(data.get("mails", []))
		Packets.S_MAIL_SENT:
			mail_sent.emit(data.get("result", 0))
		Packets.S_PARCEL_COLLECTED:
			parcel_collected.emit(data.get("result", 0))
		Packets.S_NPC_REFINE:
			npc_refine.emit(data.get("rate", 0.0), data.get("refining", false))
		Packets.S_OBJECT_MAGIC:
			object_magic.emit(data)
		Packets.S_RANGE_ATTACK:
			range_attack.emit(data.get("target_id", 0), data.get("target", Vector2i.ZERO), data.get("spell", 0))
		Packets.S_OBJECT_RANGE_ATTACK:
			object_range_attack.emit(data)
		Packets.S_NEW_MAGIC:
			new_magic.emit(data.get("magic", {}))
		Packets.S_MAGIC_LEVELED:
			magic_leveled.emit(data.get("spell", 0), data.get("level", 0), data.get("experience", 0))
		Packets.S_SWITCH_GROUP:
			group_switched.emit(data.get("allow_group", false))
		Packets.S_DELETE_MEMBER:
			delete_member.emit(data.get("name", ""))
		Packets.S_GROUP_INVITE:
			group_invite.emit(data.get("name", ""))
		Packets.S_ADD_MEMBER:
			add_member.emit(data.get("name", ""))
		Packets.S_FRIEND_UPDATE:
			friend_update.emit(data.get("friends", []))
		Packets.S_MARRIAGE_REQUEST:
			marriage_request.emit(data.get("name", ""))
		Packets.S_LOVER_UPDATE:
			lover_update.emit(data)
		Packets.S_MENTOR_UPDATE:
			mentor_update.emit(data)
		Packets.S_REQUEST_REINCARNATION:
			request_reincarnation.emit()
		Packets.S_FISHING_UPDATE:
			fishing_update.emit(data)
		Packets.S_OBJECT_HIDDEN:
			object_hidden.emit(data.get("object_id", 0), data.get("hidden", false))
		Packets.S_BASE_STATS_INFO:
			base_stats_info.emit(data)
		Packets.S_STORAGE_UNLOCK_RESULT:
			storage_unlock_result.emit(data.get("result", 0), data.get("has_password", false))
		Packets.S_STORAGE_PASSWORD_RESULT:
			storage_password_result.emit(data.get("result", 0))
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
