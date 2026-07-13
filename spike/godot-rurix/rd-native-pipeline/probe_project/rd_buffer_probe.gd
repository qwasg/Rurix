extends Node

# Route B buffer-runtime-equivalence probe — the raw-buffer sibling of
# rd_native_probe.gd (S2 texture path).
#
# S2 proved a TEXTURE-typed Rurix container runs unmodified as a first-class
# RenderingDevice compute pass. It did NOT cover the raw-buffer containers
# (StructuredBuffer<T> -> UNIFORM_TYPE_STORAGE_BUFFER). The S1 pipeline report
# flags the open gap: Godot's D3D12 driver binds STORAGE_BUFFER uniforms as a
# RAW R32_TYPELESS view (StructureByteStride = 0, D3D12_BUFFER_*_FLAG_RAW;
# rendering_device_driver_d3d12.cpp:3547-3563), whereas the offline dispatch
# harnesses bind STRUCTURED views (real element strides). Whether the
# DXC-compiled StructuredBuffer DXIL runs equivalently under a RAW view is the
# runtime-equivalence question this probe answers — with real GPU bytes.
#
# The manifest is generic over the binding shape so one probe covers both the
# 3-binding cluster_store pass and the 5-binding instance_compaction scatter
# pass. Per case it:
#   container bytes -> shader_create_from_bytecode -> compute_pipeline_create
#   for each `inputs[i]`:  storage_buffer_create(bytes) -> RDUniform STORAGE_BUFFER @ binding
#   `output`:             storage_buffer_create(zeros) -> RDUniform STORAGE_BUFFER @ binding
#   uniform_set_create([...all uniforms...], shader, 0)  (all
#     UNIFORM_TYPE_STORAGE_BUFFER; the driver picks UAV vs SRV per the
#     container's per-binding writable reflection flag)
#   compute_list dispatch(gx,gy,gz) with a 32-byte b0 push constant
#   submit + sync -> buffer_get_data(output) -> write the raw readback file
#
# The CI smoke (ci/grx_rd_buffer_probe_smoke.py) owns the CPU reference and the
# per-u32-word zero-tolerance comparison. This probe only exercises the real GPU
# path and writes the readback bytes. It fails closed (nonzero exit +
# RD_BUFFER_PROBE_RESULT status=fail) on any invalid RID, size mismatch, or I/O
# error, and never reports a silent pass.

const OK_EXIT := 0
const FAIL_EXIT := 2


func _fail(reason: String) -> void:
	print("RD_BUFFER_PROBE_RESULT status=fail reason=", reason)
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
	print("RD_BUFFER_PROBE_BEGIN")

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
		# Local RD unavailable under this driver — report so the smoke can record
		# an honest skip. Fail closed (never a silent pass).
		print("RD_BUFFER_PROBE_ENV local_rd=null")
		_fail("create_local_rendering_device_null")
		return
	print("RD_BUFFER_PROBE_ENV local_rd=ok adapter=", rd.get_device_name(),
		" vendor=", rd.get_device_vendor_name())

	var bytes := FileAccess.get_file_as_bytes(container_path)
	if bytes.is_empty():
		_fail("container_unreadable:" + container_path)
		return
	print("RD_BUFFER_PROBE_STAGE stage=container bytes=", bytes.size())

	var shader := rd.shader_create_from_bytecode(bytes)
	if not shader.is_valid():
		_fail("shader_create_from_bytecode_invalid")
		return
	print("RD_BUFFER_PROBE_STAGE stage=shader ok=true")

	var pipeline := rd.compute_pipeline_create(shader)
	if not pipeline.is_valid():
		_fail("compute_pipeline_create_invalid")
		return
	print("RD_BUFFER_PROBE_STAGE stage=pipeline ok=true")

	for case_v in cases:
		var case: Dictionary = case_v
		var case_id := String(case.get("case_id", "?"))
		var dispatch: Array = case.get("dispatch", [])
		var b0_path := String(case.get("b0_path", ""))
		var inputs: Array = case.get("inputs", [])
		var output: Dictionary = case.get("output", {})
		if dispatch.size() != 3 or b0_path == "" or inputs.is_empty() or output.is_empty():
			_fail("case_fields_invalid:" + case_id)
			return
		var gx := int(dispatch[0])
		var gy := int(dispatch[1])
		var gz := int(dispatch[2])
		if gx <= 0 or gy <= 0 or gz <= 0:
			_fail("dispatch_invalid:" + case_id)
			return

		var b0 := FileAccess.get_file_as_bytes(b0_path)
		if b0.size() != 32:
			_fail("b0_size_mismatch:" + case_id + ":got=" + str(b0.size()) + ":want=32")
			return

		var out_binding := int(output.get("binding", -1))
		var dst_bytes := int(output.get("dst_bytes", 0))
		var output_path := String(output.get("output_path", ""))
		if out_binding < 0 or dst_bytes <= 0 or output_path == "":
			_fail("output_fields_invalid:" + case_id)
			return

		# Build every input SRV/UAV buffer + the zero-initialized destination.
		# The destination is explicitly zeroed, mirroring the native buffer_clear
		# each kernel assumes.
		var rids: Array = []
		var uniforms: Array = []
		var abort_reason := ""
		for input_v in inputs:
			var input: Dictionary = input_v
			var binding := int(input.get("binding", -1))
			var path := String(input.get("path", ""))
			if binding < 0 or path == "":
				abort_reason = "input_fields_invalid:" + case_id
				break
			var data := FileAccess.get_file_as_bytes(path)
			if data.is_empty():
				abort_reason = "input_unreadable:" + case_id + ":b" + str(binding)
				break
			var buf := rd.storage_buffer_create(data.size(), data)
			if not buf.is_valid():
				abort_reason = "input_buffer_invalid:" + case_id + ":b" + str(binding)
				break
			rids.append(buf)
			var u := RDUniform.new()
			u.uniform_type = RenderingDevice.UNIFORM_TYPE_STORAGE_BUFFER
			u.binding = binding
			u.add_id(buf)
			uniforms.append(u)
		if abort_reason != "":
			for rid in rids:
				rd.free_rid(rid)
			_fail(abort_reason)
			return

		var zeros := PackedByteArray()
		zeros.resize(dst_bytes)  # resize zero-fills
		var dst := rd.storage_buffer_create(dst_bytes, zeros)
		if not dst.is_valid():
			for rid in rids:
				rd.free_rid(rid)
			_fail("output_buffer_invalid:" + case_id)
			return
		rids.append(dst)
		var uo := RDUniform.new()
		uo.uniform_type = RenderingDevice.UNIFORM_TYPE_STORAGE_BUFFER
		uo.binding = out_binding
		uo.add_id(dst)
		uniforms.append(uo)

		var uset := rd.uniform_set_create(uniforms, shader, 0)
		if not uset.is_valid():
			for rid in rids:
				rd.free_rid(rid)
			_fail("uniform_set_invalid:" + case_id)
			return

		var cl := rd.compute_list_begin()
		rd.compute_list_bind_compute_pipeline(cl, pipeline)
		rd.compute_list_bind_uniform_set(cl, uset, 0)
		rd.compute_list_set_push_constant(cl, b0, 32)
		rd.compute_list_dispatch(cl, gx, gy, gz)
		rd.compute_list_end()
		rd.submit()
		rd.sync()

		var outdata := rd.buffer_get_data(dst)
		var readback_ok := outdata.size() == dst_bytes
		if readback_ok:
			var f := FileAccess.open(output_path, FileAccess.WRITE)
			if f == null:
				readback_ok = false
				abort_reason = "output_open_failed:" + case_id + ":" + output_path
			else:
				f.store_buffer(outdata)
				f.close()
		else:
			abort_reason = "readback_size_mismatch:" + case_id + ":got=" \
				+ str(outdata.size()) + ":want=" + str(dst_bytes)

		rd.free_rid(uset)
		for rid in rids:
			rd.free_rid(rid)
		if not readback_ok:
			_fail(abort_reason)
			return
		print("RD_BUFFER_PROBE_CASE case_id=", case_id, " groups=", gx, "x", gy, "x", gz,
			" inputs=", inputs.size(), " dst_bytes=", outdata.size(), " ok=true")

	rd.free_rid(pipeline)
	rd.free_rid(shader)
	rd.free()
	print("RD_BUFFER_PROBE_RESULT status=ok cases=", cases.size())
	print("RD_BUFFER_PROBE_END")
	get_tree().quit(OK_EXIT)
