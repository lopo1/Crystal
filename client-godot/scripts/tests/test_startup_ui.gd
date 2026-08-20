extends SceneTree
## Asserts the login screen is not displaced by Camera2D and keeps widget layout.
## Run: Godot --path client-godot --headless --script res://scripts/tests/test_startup_ui.gd

var _frames := 0
var _scene: Node


func _initialize() -> void:
	var packed: PackedScene = load("res://scenes/main.tscn")
	if packed == null:
		_fail("could not load res://scenes/main.tscn")
		return
	_scene = packed.instantiate()
	root.add_child(_scene)


func _process(_delta: float) -> bool:
	_frames += 1
	if _frames < 3:
		return false
	_assert_startup()
	return true


func _assert_startup() -> void:
	var errors: PackedStringArray = []
	var cam: Camera2D = _scene.find_child("Camera2D", true, false) as Camera2D
	if cam == null:
		errors.append("Camera2D missing")
	else:
		if cam.enabled:
			errors.append("Camera2D must be disabled on the login screen")
		if cam.is_current():
			errors.append("Camera2D must not be current on the login screen")

	var login: Control = _scene.find_child("LoginPanel", true, false) as Control
	if login == null:
		errors.append("LoginPanel missing")
	else:
		if not login.visible:
			errors.append("LoginPanel should be visible at startup")
		if login is Container:
			errors.append("LoginPanel must not be a Container (it would crush absolutely-positioned children)")

	var connect_btn: Button = _scene.find_child("ConnectButton", true, false) as Button
	if connect_btn == null:
		errors.append("ConnectButton missing")
	else:
		if connect_btn.size.x > 400.0 or connect_btn.size.y > 80.0:
			errors.append("ConnectButton was stretched to %s; login widgets should keep their authored size" % connect_btn.size)

	var origin: Vector2 = root.get_viewport().get_canvas_transform().origin
	if origin.length() > 1.0:
		errors.append("canvas transform origin is %s; login UI would be off-screen" % origin)

	var hud: Node = _scene.find_child("GameView", true, false)
	if hud == null:
		errors.append("GameView missing")
	elif not (hud is CanvasLayer):
		errors.append("GameView must be a CanvasLayer so in-game HUD stays in screen space")

	if not errors.is_empty():
		for err in errors:
			push_error(err)
		_fail("%d assertion(s) failed" % errors.size())
		return
	print("PASS: login UI is on-screen and isolated from Camera2D")
	quit(0)


func _fail(message: String) -> void:
	push_error(message)
	quit(1)
