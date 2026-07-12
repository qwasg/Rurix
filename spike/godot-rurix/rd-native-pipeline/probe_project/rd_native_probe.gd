extends Node

# Route B S2 — zero-patch in-engine RD-native compute probe.
#
# Reads a JSON manifest (absolute path passed after `--`), loads a
# RenderingShaderContainerD3D12 container built from Rurix offline artifacts,
# and for each case:
#   container bytes -> shader_create_from_bytecode  (-> shader_create_from_container)
#                   -> compute_pipeline_create      (-> CreateRootSignature + PSO)
#   seed src Texture2D from a raw RGBA32F input file (SAMPLING, t0 SRV)
#   allocate dst RWTexture2D (STORAGE, u0 UAV, CAN_COPY_FROM for readback)
#   uniform_set_create([t0, u0], shader, 0)
#   compute_list dispatch (ceil(w/8), ceil(h/8), 1) with a 28-byte b0 push constant
#   submit + sync, texture_get_data(dst) -> write raw RGBA32F output file
#
# The CI smoke (ci/grx_rd_native_probe_smoke.py) owns the CPU reference and the
# per-texel comparison; this probe only exercises the real GPU path and writes
# the readback bytes. All progress is reported through parseable stdout markers
# and the probe fails closed (nonzero exit + RD_NATIVE_PROBE_RESULT status=fail)
# on any invalid RID or I/O error.

const OK_EXIT := 0
const FAIL_EXIT := 2


func _fail(reason: String) -> void:
	print("RD_NATIVE_PROBE_RESULT status=fail reason=", reason)
	get_tree().quit(FAIL_EXIT)


func _find_manifest_arg() -> String:
	var args := OS.get_cmdline_user_args()
	var i := 0
	while i < args.size():
		if args[i] == "--manifest" and i + 1 < args.size():
			return args[i + 1]
		i += 1
	return ""


func _ready() -> void:
	print("RD_NATIVE_PROBE_BEGIN")

	var manifest_path := _find_manifest_arg()
	if manifest_path == "":
		_fail("no_manifest_arg")
		return
	var manifest_text := FileAccess.get_file_as_string(manifest_path)
	if manifest_text == "":
		_fail("manifest_unreadable:" + manifest_path)
		return
	var parsed: Variant = JSON.parse_string(manifest_text)
	if typeof(parsed) != TYPE_DICTIONARY:
		_fail("manifest_not_object")
		return
	var manifest: Dictionary = parsed

	var container_path := String(manifest.get("container_path", ""))
	var cases: Array = manifest.get("cases", [])
	if container_path == "" or cases.is_empty():
		_fail("manifest_missing_container_or_cases")
		return

	var rd := RenderingServer.create_local_rendering_device()
	if rd == null:
		# Q1: local RD unavailable under this driver — report so the smoke can
		# fall back to the main-RD route. Fail closed (never a silent pass).
		print("RD_NATIVE_PROBE_ENV local_rd=null")
		_fail("create_local_rendering_device_null")
		return
	print("RD_NATIVE_PROBE_ENV local_rd=ok adapter=", rd.get_device_name(),
		" vendor=", rd.get_device_vendor_name())

	var bytes := FileAccess.get_file_as_bytes(container_path)
	if bytes.is_empty():
		_fail("container_unreadable:" + container_path)
		return
	print("RD_NATIVE_PROBE_STAGE stage=container bytes=", bytes.size())

	var shader := rd.shader_create_from_bytecode(bytes)
	if not shader.is_valid():
		_fail("shader_create_from_bytecode_invalid")
		return
	print("RD_NATIVE_PROBE_STAGE stage=shader ok=true")

	var pipeline := rd.compute_pipeline_create(shader)
	if not pipeline.is_valid():
		_fail("compute_pipeline_create_invalid")
		return
	print("RD_NATIVE_PROBE_STAGE stage=pipeline ok=true")

	for case_v in cases:
		var case: Dictionary = case_v
		var case_id := String(case.get("case_id", "?"))
		var w := int(case.get("width", 0))
		var h := int(case.get("height", 0))
		var exposure := float(case.get("exposure", 1.0))
		var white := float(case.get("white", 1.0))
		var lum := float(case.get("luminance_multiplier", 1.0))
		var input_path := String(case.get("input_path", ""))
		var output_path := String(case.get("output_path", ""))
		if w <= 0 or h <= 0 or input_path == "" or output_path == "":
			_fail("case_fields_invalid:" + case_id)
			return

		var indata := FileAccess.get_file_as_bytes(input_path)
		var expect_bytes := w * h * 16
		if indata.size() != expect_bytes:
			_fail("input_size_mismatch:" + case_id + ":got=" + str(indata.size())
				+ ":want=" + str(expect_bytes))
			return

		var fmt := RDTextureFormat.new()
		fmt.format = RenderingDevice.DATA_FORMAT_R32G32B32A32_SFLOAT
		fmt.width = w
		fmt.height = h
		fmt.depth = 1
		fmt.array_layers = 1
		fmt.mipmaps = 1
		fmt.texture_type = RenderingDevice.TEXTURE_TYPE_2D
		fmt.usage_bits = (RenderingDevice.TEXTURE_USAGE_SAMPLING_BIT
			| RenderingDevice.TEXTURE_USAGE_CAN_UPDATE_BIT)
		var src := rd.texture_create(fmt, RDTextureView.new(), [indata])
		if not src.is_valid():
			_fail("src_texture_invalid:" + case_id)
			return

		var dfmt := RDTextureFormat.new()
		dfmt.format = RenderingDevice.DATA_FORMAT_R32G32B32A32_SFLOAT
		dfmt.width = w
		dfmt.height = h
		dfmt.depth = 1
		dfmt.array_layers = 1
		dfmt.mipmaps = 1
		dfmt.texture_type = RenderingDevice.TEXTURE_TYPE_2D
		dfmt.usage_bits = (RenderingDevice.TEXTURE_USAGE_STORAGE_BIT
			| RenderingDevice.TEXTURE_USAGE_CAN_COPY_FROM_BIT)
		var dst := rd.texture_create(dfmt, RDTextureView.new())
		if not dst.is_valid():
			_fail("dst_texture_invalid:" + case_id)
			return

		var u0 := RDUniform.new()
		u0.uniform_type = RenderingDevice.UNIFORM_TYPE_TEXTURE
		u0.binding = 0
		u0.add_id(src)
		var u1 := RDUniform.new()
		u1.uniform_type = RenderingDevice.UNIFORM_TYPE_IMAGE
		u1.binding = 1
		u1.add_id(dst)
		var uset := rd.uniform_set_create([u0, u1], shader, 0)
		if not uset.is_valid():
			_fail("uniform_set_invalid:" + case_id)
			return

		# b0 push constant: [i64 width(lo,hi), i64 height(lo,hi), f32 exposure,
		# f32 white, f32 luminance_multiplier] = 7 dwords / 28 bytes. The i64
		# high dwords are written 0 per the canonical Rurix layout.
		var pc := PackedByteArray()
		pc.resize(28)
		pc.encode_u32(0, w)
		pc.encode_u32(4, 0)
		pc.encode_u32(8, h)
		pc.encode_u32(12, 0)
		pc.encode_float(16, exposure)
		pc.encode_float(20, white)
		pc.encode_float(24, lum)

		var gx := int(ceil(float(w) / 8.0))
		var gy := int(ceil(float(h) / 8.0))
		var cl := rd.compute_list_begin()
		rd.compute_list_bind_compute_pipeline(cl, pipeline)
		rd.compute_list_bind_uniform_set(cl, uset, 0)
		rd.compute_list_set_push_constant(cl, pc, 28)
		rd.compute_list_dispatch(cl, gx, gy, 1)
		rd.compute_list_end()
		rd.submit()
		rd.sync()

		var outdata := rd.texture_get_data(dst, 0)
		if outdata.size() != expect_bytes:
			_fail("readback_size_mismatch:" + case_id + ":got=" + str(outdata.size())
				+ ":want=" + str(expect_bytes))
			return
		var f := FileAccess.open(output_path, FileAccess.WRITE)
		if f == null:
			_fail("output_open_failed:" + case_id + ":" + output_path)
			return
		f.store_buffer(outdata)
		f.close()

		rd.free_rid(uset)
		rd.free_rid(src)
		rd.free_rid(dst)
		print("RD_NATIVE_PROBE_CASE case_id=", case_id, " w=", w, " h=", h,
			" groups=", gx, "x", gy, " out_bytes=", outdata.size(), " ok=true")

	rd.free_rid(pipeline)
	rd.free_rid(shader)
	rd.free()
	print("RD_NATIVE_PROBE_RESULT status=ok cases=", cases.size())
	print("RD_NATIVE_PROBE_END")
	get_tree().quit(OK_EXIT)
