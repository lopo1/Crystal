extends RefCounted
class_name Web3Wallet
## 钱包签名桥（Web3 登录）。
##
## Godot (GDScript) 没有原生 secp256k1/keccak-256，因此真实钱包的
## `personal_sign`(EIP-191) 必须由外部提供：
##   - Web 导出：通过 JavaScript 桥接 MetaMask / WalletConnect（`window.ethereum`）
##   - 桌面/开发：调用本地签名服务 HTTP（本机测试用），或原生 GDExtension
##
## 本脚本只约定接口并给出两种实现；运行时通过 `Web3Wallet.wallet_type` 配置。
## 成功调用后通过回调把 "0x 地址" 或签名字节数组交回上层。

enum Type { JS, RPC }

## 选择钱包实现（默认 JS=MetaMask；开发无浏览器时改 RPC 指向本地签名服务）
var wallet_type: Type = Type.JS

## 本地签名服务地址（仅 RPC 模式使用；见 examples/signer 说明）
var rpc_url := "http://127.0.0.1:8545"


## 返回钱包地址（小写 0x 十六进制）。异步，结果经回调 `callable(address)` 返回。
func get_address(callable: Callable) -> void:
	match wallet_type:
		Type.JS:
			_js_get_address(callable)
		Type.RPC:
			_rpc_request("/wallet/address", {}, callable)


## 对 message 做 EIP-191 personal_sign，返回 65 字节签名 PackedByteArray。
## 异步，结果经回调 `callable(signature: PackedByteArray, address: String)` 返回。
func personal_sign(message: String, callable: Callable) -> void:
	match wallet_type:
		Type.JS:
			_js_personal_sign(message, callable)
		Type.RPC:
			_rpc_personal_sign(message, callable)


# ---------------------------------------------------------------------------
# JS (MetaMask) 实现 —— Web 导出时可用
# ---------------------------------------------------------------------------

func _js_get_address(callable: Callable) -> void:
	if not _js_available():
		callable.call("")
		return
	var js := "
		const a = await window.ethereum.request({ method: 'eth_requestAccounts' });
		return a && a[0] ? String(a[0]).toLowerCase() : '';
	"
	_js_exec(js, func(result: String) -> void:
		callable.call(result)
	)


func _js_personal_sign(message: String, callable: Callable) -> void:
	if not _js_available():
		callable.call(PackedByteArray(), "")
		return
	var msg_hex := message.to_utf8_buffer().hex_encode()
	var js := "
		const from = (await window.ethereum.request({ method: 'eth_requestAccounts' }))[0];
		const sig = await window.ethereum.request({
			method: 'personal_sign',
			params: ['0x' + '%s', from]
		});
		return (sig || '');
	" % msg_hex
	_js_exec(js, func(result: String) -> void:
		var sig_bytes := PackedByteArray()
		if result.begins_with("0x"):
			sig_bytes = _hex_decode(result.substr(2))
		callable.call(sig_bytes, "")
	)


func _js_available() -> bool:
	return ClassDB.class_exists("JavaScriptBridge")


func _js_exec(js: String, on_result: Callable) -> void:
	if not _js_available():
		on_result.call("")
		return
	JavaScriptBridge.eval(js, true)


static func _hex_decode(hex_str: String) -> PackedByteArray:
	var out := PackedByteArray()
	for i in range(0, hex_str.length(), 2):
		out.append(hex_str.substr(i, 2).hex_to_int())
	return out


# ---------------------------------------------------------------------------
# RPC (本地签名服务) 实现 —— 桌面/开发联调用
# ---------------------------------------------------------------------------

func _rpc_request(path: String, body: Dictionary, callable: Callable) -> void:
	# 用 HTTPRequest 异步请求，开发时由本地签名服务返回结果
	var http := HTTPRequest.new()
	var tree := Engine.get_main_loop()
	if tree and tree is SceneTree:
		tree.root.add_child(http)
	http.request_completed.connect(func(_r, _c, _h, b: PackedByteArray) -> void:
		var text := b.get_string_from_utf8().strip_edges()
		callable.call(text)
		http.queue_free()
	)
	var err := http.request(rpc_url + path, [], HTTPClient.METHOD_POST, JSON.stringify(body))
	if err != OK:
		callable.call("")


func _rpc_personal_sign(message: String, callable: Callable) -> void:
	# 请求本地签名服务返回 "0x" + 130 hex 字符的签名（65 字节）。
	_rpc_request("/wallet/sign", {"message": message}, func(text: String) -> void:
		var sig_bytes := PackedByteArray()
		if text.begins_with("0x"):
			sig_bytes = _hex_decode(text.substr(2))
		# 同时把地址也带回（简单起见从签名服务 GET；此处置空，由上层缓存）
		callable.call(sig_bytes, "")
	)
