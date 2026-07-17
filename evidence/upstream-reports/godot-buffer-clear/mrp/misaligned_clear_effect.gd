# DRAFT — do NOT file — upstream MRP source
extends CompositorEffect
## Minimal reproduction: RenderingDevice.buffer_clear() at a byte offset that is
## NOT a multiple of 16 removes the Direct3D 12 device
## (DXGI_ERROR_DEVICE_REMOVED / 0x887A0005) when issued on the main rendering
## device inside the frame graph.
##
## Root cause: RenderingDeviceDriverD3D12::command_clear_buffer builds a RAW UAV
## (D3D12_BUFFER_UAV_FLAG_RAW) with FirstElement = offset / 4 and no alignment
## guard. D3D12 requires raw buffer UAV byte offsets to be a multiple of 16
## (D3D12_RAW_UAV_SRV_BYTE_ALIGNMENT). A non-aligned offset produces an
## out-of-spec UAV and ClearUnorderedAccessViewUint removes the device.
##
## Empirically confirmed on RTX 4070 Ti / Godot 4.7-dev / D3D12 (crash = process
## exit 139; clean = exit 0):
##   * offset 0/16/32/48 -> clean;  offset 4/8/12/20/36 -> device removed.
##     A perfect offset % 16 law, 0 exceptions across the sweep.
##   * Independent of clear size and of buffer usage flags (a plain storage
##     buffer and a STORAGE_BUFFER_USAGE_DISPATCH_INDIRECT buffer behave the same).
##   * The D3D12 debug layer AND --gpu-validation are silent (they neither flag
##     the misaligned UAV nor prevent the removal).
##   * A LOCAL RenderingDevice tolerates the same misaligned clear -> this is
##     specific to the main device's frame graph.
## Flip CLEAR_OFFSET to 0 or 16 to see it run cleanly.

const CLEAR_OFFSET := 4  # not a multiple of 16 => device removed. 0/16/32 => clean.
const CLEAR_SIZE := 4

var rd: RenderingDevice
var _buf: RID
var _frame := 0


func _init() -> void:
	effect_callback_type = EFFECT_CALLBACK_TYPE_POST_TRANSPARENT
	rd = RenderingServer.get_rendering_device()


func _render_callback(p_type: int, _render_data: RenderData) -> void:
	if rd == null or p_type != EFFECT_CALLBACK_TYPE_POST_TRANSPARENT:
		return
	if not _buf.is_valid():
		# A plain storage buffer; no special usage flag is needed.
		_buf = rd.storage_buffer_create(64)
	_frame += 1
	rd.buffer_clear(_buf, CLEAR_OFFSET, CLEAR_SIZE)
	print("[repro] frame %d: buffer_clear(offset=%d, size=%d), offset %% 16 = %d" % [_frame, CLEAR_OFFSET, CLEAR_SIZE, CLEAR_OFFSET % 16])
