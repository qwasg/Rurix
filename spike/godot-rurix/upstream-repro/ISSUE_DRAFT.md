# Upstream issue draft — godotengine/godot

> **Status: DRAFT for owner review — do NOT file yet.**
> Public-facing text for a public repository. Complete the `<FILL: …>` placeholders and the
> pre-filing checklist at the bottom before submitting. The minimal reproduction project is
> at `spike/godot-rurix/upstream-repro/rd-buffer-clear-misaligned-offset/` (zip without `.godot/`).

---

## Title

`RenderingDevice.buffer_clear()` at a non-16-byte-aligned offset removes the Direct3D 12 device (`DXGI_ERROR_DEVICE_REMOVED` / `0x887A0005`)

---

## Tested versions

- Reproducible in: 4.7.dev [`<FILL: exact commit hash of the stock build you confirm on>`], Direct3D 12 backend.
- Not tested / please confirm: Vulkan backend (same project, `--rendering-driver vulkan`) — expected not to reproduce.
- `RenderingDevice.buffer_clear()` predates 4.0, so this is very likely present in all 4.x with the D3D12 backend. `<FILL: whether an earlier stable (e.g. 4.3/4.4) also reproduces>`.

## System information

`<FILL via editor: Help → Copy System Info>` — e.g.:
`Windows 11 (build 22631) - Godot v4.7.dev [<commit>] - Direct3D 12 (Forward+) - NVIDIA GeForce RTX 4070 Ti (nvidia; <driver version>) - <CPU model>`

Known-fixed parts: Windows 11, Direct3D 12 (Forward+), NVIDIA GeForce RTX 4070 Ti.

## Issue description

On the **Direct3D 12** backend, calling `RenderingDevice.buffer_clear(buffer, offset, size)`
with a byte `offset` that is **not a multiple of 16**, on the **main** rendering device inside
the frame graph (e.g. from a `CompositorEffect._render_callback`), **removes the device**:
`DXGI_ERROR_DEVICE_REMOVED` (`HRESULT 0x887A0005`). The error surfaces on the next GPU API
call after the faulting frame (an asynchronous GPU-side fault, not a CPU-side API error):

```
ERROR: CreateCommandAllocator failed with error 0x887a0005.
   at: RenderingDeviceDriverD3D12::command_buffer_create (rendering_device_driver_d3d12.cpp)
```

This was isolated with a controlled offset sweep (crash detected by process exit code — a
crash exits 139, a clean run exits 0):

| `buffer_clear` offset | offset % 16 | result |
|---|---|---|
| 0, 16, 32, 48 | 0 | clean |
| 4, 8, 12, 20, 36 | ≠ 0 | **device removed** |

A perfect `offset % 16` boundary, with no exceptions across the sweep. Additional controls:

- **Independent of `size`** (offset 0 is clean at sizes 4/8/12/16/20; the start offset is what matters).
- **Independent of buffer usage** — a plain storage buffer and a
  `STORAGE_BUFFER_USAGE_DISPATCH_INDIRECT` buffer behave identically.
- **No compute or draw involved** — the buffer is only ever cleared.
- **Silent** — the D3D12 debug layer and `--gpu-validation` (GPU-Based Validation) print no
  message; they neither flag the misaligned UAV nor prevent the removal.
- **Main-device-specific** — the same misaligned clear on a *local* `RenderingDevice`
  (`RenderingServer.create_local_rendering_device()` + `submit()`/`sync()`) does not fault.

### Likely root cause

`RenderingDeviceDriverD3D12::command_clear_buffer` (`drivers/d3d12/rendering_device_driver_d3d12.cpp`)
builds a raw buffer UAV to clear the range:

```cpp
uav_desc.Format             = DXGI_FORMAT_R32_TYPELESS;
uav_desc.ViewDimension      = D3D12_UAV_DIMENSION_BUFFER;
uav_desc.Buffer.FirstElement = p_offset / 4;   // <-- no alignment enforcement
uav_desc.Buffer.NumElements  = p_size / 4;
uav_desc.Buffer.Flags        = D3D12_BUFFER_UAV_FLAG_RAW;
device->CreateUnorderedAccessView(buf_info->resource, nullptr, &uav_desc, ...);
...
cmd_list->ClearUnorderedAccessViewUint(..., buf_info->resource, values, 0, nullptr);
```

D3D12 requires the byte offset of a raw buffer UAV to be a multiple of 16
(`D3D12_RAW_UAV_SRV_BYTE_ALIGNMENT` = 16), i.e. `FirstElement` must be a multiple of 4. When
`p_offset` is not 16-byte-aligned, this creates an out-of-spec raw UAV, and
`ClearUnorderedAccessViewUint` on it removes the device. (`p_offset` only has to be a multiple
of 4 to pass `RenderingDevice::buffer_clear`'s own `p_size % 4` check, so callers can and do
pass 4-byte-aligned but not 16-byte-aligned offsets.)

## Steps to reproduce

The attached project is pure GDScript against `RenderingDevice` — no C++ modules, GDExtensions,
or editor plugins.

1. Open the attached project in a **Godot 4.7-dev** editor with the **Direct3D 12** backend
   active (the project forces `rendering/rendering_device/driver.windows="d3d12"`; confirm via
   *Help → Copy System Info*).
2. Run the project (**F5**). A `CompositorEffect` on the main device runs, per frame:
   `buffer_clear(buffer, 4, 4)` on a 64-byte storage buffer (offset 4 is not a multiple of 16).
3. **Result:** the device is removed on frame 1 — `0x887A0005` on the next GPU API call; the
   process exits non-zero.
4. **Boundary check:** set `const CLEAR_OFFSET := 0` (or 16) in `misaligned_clear_effect.gd`
   and run again — it now runs cleanly.

Minimal equivalent (inside a render-thread context such as a `CompositorEffect`):

```gdscript
var rd := RenderingServer.get_rendering_device()
var buf := rd.storage_buffer_create(64)
rd.buffer_clear(buf, 4, 4)   # offset 4 is not a multiple of 16 -> device removed on D3D12
```

## Minimal reproduction project (MRP)

`<FILL: attach rd-buffer-clear-misaligned-offset-mrp.zip>` — the
`spike/godot-rurix/upstream-repro/rd-buffer-clear-misaligned-offset/` folder zipped without the
`.godot/` cache (keep `project.godot`).

---

## Pre-filing checklist (owner — remove this whole section before pasting into GitHub)

1. **Confirm on a STOCK build.** The reproducer was run on a local self-built 4.7-dev engine
   (its only nonstandard content is an inactive module never touched by this project; the code
   path is 100% stock `RenderingDevice`/D3D12). Reconfirm on an official 4.7-dev snapshot or a
   clean self-build before filing.
2. **Fill `System information`** from *Help → Copy System Info* (GPU driver version + CPU).
3. **Fill `Tested versions`**: the exact commit hash you confirmed on; the Vulkan result
   (`--rendering-driver vulkan`); and, ideally, whether an earlier stable reproduces
   (`buffer_clear` predates 4.0, so this is likely long-standing, not a 4.7 regression).
4. **Search once more** at file time. As of this draft there is **no duplicate**: searches for
   `buffer_clear`, `ClearUnorderedAccessViewUint`, `D3D12_RAW_UAV_SRV_BYTE_ALIGNMENT`, and
   `buffer_clear` + device-removal returned nothing relevant, and a broad 80-query
   device-removal sweep found no match. Nearest non-duplicates: #120857 (Intel Arc alpha-scissor,
   same HRESULT, different cause), #103488 (Mac/Metal, different cause).
5. **Consider a fix suggestion in the issue** (optional): align `FirstElement` down to a
   16-byte boundary and widen `NumElements` to cover the requested range, or clear via a path
   that does not require a raw UAV. (Godot maintainers will decide the actual fix; offering the
   `D3D12_RAW_UAV_SRV_BYTE_ALIGNMENT` pointer is enough.)
6. **Zip the MRP** without `.godot/`, under 10 MB.
7. File as a **bug report** using the repository's bug template; one bug per issue.
