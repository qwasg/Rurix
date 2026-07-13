extends Node

# Minimal reproduction for a Godot D3D12 backend device-removal (DXGI_ERROR_DEVICE_REMOVED,
# 0x887A0005).
#
# Pattern under test (all on the MAIN RenderingDevice, within one frame):
#   1. A compute pass writes a storage buffer that was created with
#      STORAGE_BUFFER_USAGE_DISPATCH_INDIRECT (bound as a UAV / writable storage buffer).
#   2. The SAME frame, draw_list_draw_indirect() consumes that buffer as the indirect
#      draw-argument source.
#
# The RenderingDeviceGraph tracks the buffer as a write (compute) then as
# RESOURCE_USAGE_INDIRECT_BUFFER_READ (draw). On the D3D12 backend the realized
# GPU-timeline synchronization between the UAV write and the INDIRECT_ARGUMENT read is
# insufficient, so the GPU faults asynchronously and the device is removed. The failure
# surfaces on the NEXT device API call (e.g. CreateCommandAllocator / a pipeline create)
# as 0x887A0005. The D3D12 debug layer and GPU-Based Validation are both silent.
#
# Expected on a correct backend: runs indefinitely, no device removal.
# Observed on D3D12 (NVIDIA RTX 4070 Ti, Windows 11): device removed within 1-2 frames.
# Not reproducible under the Vulkan backend on the same machine (per the reporter).
#
# Run with:  godot --path <this_dir> --rendering-driver d3d12 --rendering-method forward_plus

const COMPUTE_SRC := """
#version 450
layout(local_size_x = 1, local_size_y = 1, local_size_z = 1) in;

// The indirect-draw argument buffer, bound as a writable storage buffer (UAV).
// Layout matches a non-indexed indirect draw: {vertexCount, instanceCount, firstVertex, firstInstance}.
layout(set = 0, binding = 0, std430) restrict buffer DrawArgs {
	uint v[];
} args;

void main() {
	// Compute produces the draw arguments this frame. The exact values do not matter to
	// the hazard: an empty main() that binds the UAV but writes nothing still removes the
	// device. Here we write a valid single-triangle, single-instance draw.
	args.v[0] = 3u; // vertexCount
	args.v[1] = 1u; // instanceCount
	args.v[2] = 0u; // firstVertex
	args.v[3] = 0u; // firstInstance
}
"""

const VERTEX_SRC := """
#version 450
// Vertexless fullscreen triangle; no vertex buffer bound.
void main() {
	vec2 p = vec2(float((gl_VertexIndex << 1) & 2), float(gl_VertexIndex & 2));
	gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}
"""

const FRAGMENT_SRC := """
#version 450
layout(location = 0) out vec4 frag_color;
void main() {
	frag_color = vec4(1.0, 0.5, 0.2, 1.0);
}
"""

var rd: RenderingDevice
var comp_shader: RID
var comp_pipeline: RID
var comp_uniform_set: RID
var raster_shader: RID
var raster_pipeline: RID
var indirect_buffer: RID
var color_tex: RID
var framebuffer: RID
var setup_ok := false
var frame := 0


func _ready() -> void:
	# Build all RenderingDevice resources on the render thread against the MAIN device.
	RenderingServer.call_on_render_thread(_rd_setup)


func _process(_delta: float) -> void:
	if setup_ok:
		RenderingServer.call_on_render_thread(_rd_frame)
	frame += 1
	if frame > 300:
		# Long enough to observe the removal; if it never removes, exit cleanly.
		get_tree().quit()


func _rd_setup() -> void:
	rd = RenderingServer.get_rendering_device()
	if rd == null:
		push_error("No RenderingDevice available. Run with --rendering-driver d3d12 (or vulkan).")
		return

	# --- Compute shader + pipeline ------------------------------------------------------
	var csrc := RDShaderSource.new()
	csrc.language = RenderingDevice.SHADER_LANGUAGE_GLSL
	csrc.source_compute = COMPUTE_SRC
	var cspv := rd.shader_compile_spirv_from_source(csrc)
	if cspv.compile_error_compute != "":
		push_error("compute compile error: " + cspv.compile_error_compute)
		return
	comp_shader = rd.shader_create_from_spirv(cspv)
	comp_pipeline = rd.compute_pipeline_create(comp_shader)

	# --- Indirect-args buffer created with DISPATCH_INDIRECT usage -----------------------
	# 4 x uint32 = 16 bytes = the minimum a non-indexed indirect draw reads.
	var init := PackedInt32Array([3, 1, 0, 0]).to_byte_array()
	indirect_buffer = rd.storage_buffer_create(
		init.size(), init, RenderingDevice.STORAGE_BUFFER_USAGE_DISPATCH_INDIRECT)

	# Bind the indirect buffer as a writable storage buffer (UAV) for the compute pass.
	var u := RDUniform.new()
	u.uniform_type = RenderingDevice.UNIFORM_TYPE_STORAGE_BUFFER
	u.binding = 0
	u.add_id(indirect_buffer)
	comp_uniform_set = rd.uniform_set_create([u], comp_shader, 0)

	# --- Raster shader + offscreen target + vertexless pipeline --------------------------
	var rsrc := RDShaderSource.new()
	rsrc.language = RenderingDevice.SHADER_LANGUAGE_GLSL
	rsrc.source_vertex = VERTEX_SRC
	rsrc.source_fragment = FRAGMENT_SRC
	var rspv := rd.shader_compile_spirv_from_source(rsrc)
	if rspv.compile_error_vertex != "" or rspv.compile_error_fragment != "":
		push_error("raster compile error: " + rspv.compile_error_vertex + rspv.compile_error_fragment)
		return
	raster_shader = rd.shader_create_from_spirv(rspv)

	var tf := RDTextureFormat.new()
	tf.format = RenderingDevice.DATA_FORMAT_R8G8B8A8_UNORM
	tf.width = 64
	tf.height = 64
	tf.usage_bits = RenderingDevice.TEXTURE_USAGE_COLOR_ATTACHMENT_BIT | RenderingDevice.TEXTURE_USAGE_CAN_COPY_FROM_BIT
	color_tex = rd.texture_create(tf, RDTextureView.new())
	framebuffer = rd.framebuffer_create([color_tex])

	var blend := RDPipelineColorBlendState.new()
	blend.attachments = [RDPipelineColorBlendStateAttachment.new()]
	raster_pipeline = rd.render_pipeline_create(
		raster_shader,
		rd.framebuffer_get_format(framebuffer),
		RenderingDevice.INVALID_ID, # vertexless
		RenderingDevice.RENDER_PRIMITIVE_TRIANGLES,
		RDPipelineRasterizationState.new(),
		RDPipelineMultisampleState.new(),
		RDPipelineDepthStencilState.new(),
		blend)

	setup_ok = comp_pipeline.is_valid() and raster_pipeline.is_valid() and indirect_buffer.is_valid()
	if setup_ok:
		print("MRP setup complete; issuing compute->draw_indirect on the main RenderingDevice each frame.")


func _rd_frame() -> void:
	print("MRP frame %d" % frame)

	# 1) Compute pass: write the indirect draw arguments (UAV write to the DISPATCH_INDIRECT buffer).
	var cl := rd.compute_list_begin()
	rd.compute_list_bind_compute_pipeline(cl, comp_pipeline)
	rd.compute_list_bind_uniform_set(cl, comp_uniform_set, 0)
	rd.compute_list_dispatch(cl, 1, 1, 1)
	rd.compute_list_end()

	# 2) Same frame, same device: consume that buffer as the indirect draw-argument source.
	var clear := PackedColorArray([Color(0, 0, 0, 1)])
	var dl := rd.draw_list_begin(framebuffer, RenderingDevice.DRAW_CLEAR_COLOR_ALL, clear)
	rd.draw_list_bind_render_pipeline(dl, raster_pipeline)
	rd.draw_list_draw_indirect(dl, false, indirect_buffer, 0, 1, 0)
	rd.draw_list_end()
