extends Node3D
## Builds a trivial 3D scene so the Forward+ renderer runs and invokes the
## CompositorEffect each frame. On the D3D12 driver, the effect's
## buffer_clear at a non-16-byte-aligned offset removes the device on frame 1.

const Effect = preload("res://misaligned_clear_effect.gd")


func _ready() -> void:
	var camera := Camera3D.new()
	add_child(camera)
	camera.current = true
	camera.position = Vector3(0.0, 0.0, 3.0)

	var mesh_instance := MeshInstance3D.new()
	mesh_instance.mesh = BoxMesh.new()
	add_child(mesh_instance)

	var environment := Environment.new()
	environment.background_mode = Environment.BG_COLOR
	environment.background_color = Color(0.15, 0.15, 0.2)

	var compositor := Compositor.new()
	compositor.compositor_effects = [Effect.new()]
	camera.environment = environment
	camera.compositor = compositor

	print("[repro] running: buffer_clear at a non-16-byte-aligned offset on the main D3D12 device")
	print("[repro] expect DXGI_ERROR_DEVICE_REMOVED (0x887A0005) on frame 1")
