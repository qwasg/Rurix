# Minimal reproduction — D3D12 device removal from a non-16-byte-aligned `buffer_clear`

`RenderingDevice.buffer_clear(buffer, offset, size)` with an `offset` that is **not a
multiple of 16** removes the Direct3D 12 device (`DXGI_ERROR_DEVICE_REMOVED` / `0x887A0005`)
when issued on the **main** rendering device inside the frame graph (e.g. from a
`CompositorEffect`).

- **Engine:** Godot 4.7-dev, Direct3D 12 (Forward+), Windows 11, NVIDIA RTX 4070 Ti.
- **Deterministic:** offset `0/16/32/48` → clean; offset `4/8/12/20/36` → device removed.
  A clean `offset % 16` law.
- **Independent of** clear size and of buffer usage flags (plain storage buffer or
  `STORAGE_BUFFER_USAGE_DISPATCH_INDIRECT` behave the same); no compute or draw is involved.
- **Silent:** the D3D12 debug layer and `--gpu-validation` neither flag the misaligned UAV
  nor prevent the removal.
- **Main-device-specific:** the same misaligned clear on a *local* `RenderingDevice`
  (`create_local_rendering_device()`) does not fault.

## Root cause

`RenderingDeviceDriverD3D12::command_clear_buffer` (`drivers/d3d12/rendering_device_driver_d3d12.cpp`)
creates a raw buffer UAV with `uav_desc.Buffer.FirstElement = p_offset / 4` and
`D3D12_BUFFER_UAV_FLAG_RAW`, with no alignment enforcement. D3D12 requires raw buffer
UAV byte offsets to be a multiple of 16 (`D3D12_RAW_UAV_SRV_BYTE_ALIGNMENT`). A non-aligned
`p_offset` yields an out-of-spec UAV; `ClearUnorderedAccessViewUint` on it removes the device.

## Contents

| File | Role |
|---|---|
| `misaligned_clear_effect.gd` | `CompositorEffect` that creates a storage buffer and `buffer_clear`s it at `CLEAR_OFFSET` (default 4). |
| `main.gd`, `main.tscn` | Builds a trivial 3D scene + attaches the effect. Default main scene. |
| `project.godot` | Forces the D3D12 backend on Windows. |

## How to run

1. Open this folder in a **Godot 4.7-dev** editor (official dev snapshot or a clean
   self-build — no custom modules), with the **Direct3D 12** backend active (`project.godot`
   forces it on Windows; `--rendering-driver d3d12` also works; confirm via
   *Help → Copy System Info*).
2. Run (F5). **Expected:** the run dies on frame 1 with
   `CreateCommandAllocator failed with error 0x887a0005` (the removal surfaces on the next
   GPU API call after the faulting frame). The process exits non-zero.
3. **Boundary check:** set `const CLEAR_OFFSET := 0` (or 16) in
   `misaligned_clear_effect.gd` and run again — it now runs cleanly. Any non-multiple-of-16
   offset (4, 8, 12, 20, …) reproduces the removal.

### Notes for anyone re-running the sweep

- Detect the crash by **process exit code** (139 = device removed, 0 = clean). Do **not**
  rely on grepping stdout for `0x887a0005`: on the crash, buffered stdout may not flush, so
  the line can be missing from a captured log even though the device was removed.
- Rapid, back-to-back device removals can leave the GPU/driver in a degraded TDR-recovery
  state that makes the *next* run fault spuriously. Space runs out, or run clean-expected
  offsets first, when characterizing the boundary.

## Provenance

This bug was first hit inside a patched engine build where a module cleared a MultiMesh
indirect command buffer's per-surface count dword at byte offset `(surface*5 + 1)*4`
(= 4, 24, 44, … — all non-16-aligned). That earlier investigation attributed the removal to
a same-frame "compute-write → indirect-draw" hazard; running this minimal reproducer showed
that attribution was wrong — the trigger is purely the misaligned `buffer_clear` offset, with
no compute or indirect draw required (see the sweep above).

The reproducer was run on a local self-built 4.7-dev engine (whose only nonstandard content
is an inactive module never touched by this project); the code path exercised here is 100%
stock `RenderingDevice` / D3D12. Reconfirm on an official 4.7-dev build before filing.

## Packaging for the issue (MRP attachment)

Zip this folder **without** the `.godot/` cache (keep `project.godot`), under 10 MB:

```
cd spike/godot-rurix/upstream-repro
7z a rd-buffer-clear-misaligned-offset-mrp.zip rd-buffer-clear-misaligned-offset -xr!.godot
```
