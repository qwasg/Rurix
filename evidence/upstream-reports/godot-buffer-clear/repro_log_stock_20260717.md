> **Status: DRAFT — do NOT file.** Owner review gate; agent does not file externally.

# repro log — stock Godot 4.7.1-stable, buffer_clear misaligned-offset device removal

Verbatim commands + outputs from re-confirming the `RenderingDevice.buffer_clear()`
non-16-byte-aligned-offset D3D12 device removal on an **official stock** Godot build. This
clears the four `<FILL>` placeholders in `ISSUE_DRAFT.md` that required a clean stock build.

- Date: 2026-07-17
- Owner authorization: 白栀 authorized this download + on-machine measurement in the
  2026-07-17 session (tool binaries live in scratch, not committed).
- Machine: Windows 11 Pro Insider Preview (build 10.0.28120), NVIDIA GeForce RTX 4070 Ti,
  13th Gen Intel Core i5-13600KF.
- Stock engine: `Godot_v4.7.1-stable_win64_console.exe` — official godotengine
  godot-builds release **4.7.1-stable** (2026-07-14). **Note:** upstream has advanced to
  4.7.1-stable since the original diagnosis (which was on a self-built 4.7-**dev**); this
  re-confirmation therefore also answers "does it still reproduce on the latest official
  stable?".
- MRP run copy: the tracked `mrp/` folder copied to a scratch dir so the generated
  `.godot/` import cache lands outside the repo; `project.godot` forces
  `rendering_device/driver.windows="d3d12"`.
- Crash judged by **process exit code**, not by grepping stdout (buffered stdout is not
  flushed on the crash — see below). Each run bounded by a `py -3` subprocess wrapper with a
  90 s timeout guard; the crash run was executed **last**, after the clean Vulkan control.

---

## 1. Stock build version + official commit hash

```
$ Godot_v4.7.1-stable_win64_console.exe --version
4.7.1.stable.official.a13da4feb
```

Official build hash: **`a13da4feb`** (full: `a13da4feb8d8aefc283c3763d33a2f170a18d541`,
as printed in the crash banner below). Original diagnosis was on a self-built **4.7-dev**;
this is the official **4.7.1-stable** release.

## 2. First-run import (headless — generates the `.godot/` cache in scratch)

```
$ Godot_v4.7.1-stable_win64_console.exe --headless --path <scratch-mrp> --import
Godot Engine v4.7.1.stable.official.a13da4feb - https://godotengine.org
[ ... first_scan_filesystem / update_scripts_classes / loading_editor_layout: all DONE ... ]
exit=0
```

## 3. Vulkan control (expected: clean — the bug is D3D12-specific)

```
$ Godot_v4.7.1-stable_win64_console.exe --path <scratch-mrp> --rendering-driver vulkan --quit-after 300
Godot Engine v4.7.1.stable.official.a13da4feb - https://godotengine.org
Vulkan 1.4.351 - Forward+ - Using Device #0: NVIDIA - NVIDIA GeForce RTX 4070 Ti

[repro] running: buffer_clear at a non-16-byte-aligned offset on the main D3D12 device
[repro] expect DXGI_ERROR_DEVICE_REMOVED (0x887A0005) on frame 1
[repro] frame 1: buffer_clear(offset=4, size=4), offset % 16 = 4
[repro] frame 2: buffer_clear(offset=4, size=4), offset % 16 = 4
   ... (frames 3..299 identical) ...
[repro] frame 300: buffer_clear(offset=4, size=4), offset % 16 = 4

--- STDERR ---
WARNING: 1 RID of type "StorageBuffer" was leaked.
   at: _free_rids (servers/rendering/rendering_device.cpp:8684)

exit=0
```

**Result: CLEAN.** `buffer_clear(offset=4)` on the Vulkan backend ran all 300 frames and
exited 0. (The `StorageBuffer` leak warning is benign — the MRP effect intentionally never
frees its buffer; it is unrelated to the fault.) This confirms the removal is **D3D12-only**.

## 4. D3D12 reproduction (expected: device removal, nonzero exit) — run LAST

```
$ Godot_v4.7.1-stable_win64_console.exe --path <scratch-mrp> --rendering-driver d3d12 --quit-after 300
Godot Engine v4.7.1.stable.official.a13da4feb - https://godotengine.org
D3D12 12_0 - Forward+ - Using Device #0: NVIDIA - NVIDIA GeForce RTX 4070 Ti

[repro] running: buffer_clear at a non-16-byte-aligned offset on the main D3D12 device
   <-- buffered stdout stops here; the "[repro] expect..." and per-frame lines were NOT
       flushed before the crash, exactly as mrp/README.md warns. Judge by exit code.

--- STDERR (excerpted; inter-record blank lines collapsed) ---
ERROR: CreateCommandAllocator failed with error 0x887a0005.
   at: command_buffer_create (drivers/d3d12/rendering_device_driver_d3d12.cpp:2575)
ERROR: Condition "!((HRESULT)(res) >= 0)" is true. Returning: SemaphoreID()
   at: semaphore_create (drivers/d3d12/rendering_device_driver_d3d12.cpp:2422)
================================================================
CrashHandlerException: Program crashed with signal 11
Engine version: Godot Engine v4.7.1.stable.official (a13da4feb8d8aefc283c3763d33a2f170a18d541)
Dumping the backtrace. Please include this when reporting the bug on: https://github.com/godotengine/godot/issues
Load address: 7ff553630000
[1] 7ff6977024a1 (main+40d24a1) - no debug info in PE/COFF executable
[2] 7ffb9d0e43a2 (ntdll.dll+1243a2) - no debug info in PE/COFF executable
   ... (frames [3]..[19]: ntdll / main / kernel32, no debug info — official build is stripped) ...
-- END OF C++ BACKTRACE --
================================================================

exit code = 3221225477 (0xC0000005 STATUS_ACCESS_VIOLATION; signal 11)
```

**Result: DEVICE REMOVED — reproduces on the latest official stable.** The exact predicted
signature surfaces: `CreateCommandAllocator failed with error 0x887a0005`
(`DXGI_ERROR_DEVICE_REMOVED`) at `rendering_device_driver_d3d12.cpp:2575`, immediately after
the first misaligned `buffer_clear(offset=4)`; the removal cascades into a
`semaphore_create` failure and the engine's crash handler catches signal 11, exiting
`0xC0000005` (nonzero). Note the `0x887a0005` line arrives on **stderr** (Godot's `ERROR:`
channel), which is flushed; the stdout `[repro]` frame prints were lost to the crash, so
stdout-grepping would have missed it — exit code is the reliable signal.

**Impact note (flagged for owner):** the original draft scoped this to `4.7.dev`. It now
reproduces on the **official 4.7.1-stable release (a13da4feb, 2026-07-14)** on a 100%-stock
build — this is a shipped-release defect, not a dev-branch-only artifact. `buffer_clear`
predates 4.0, so it is very likely long-standing across the whole 4.x D3D12 line (an earlier
stable such as 4.3/4.4 was **not** tested in this pass — only the 4.7.1-stable build was
downloaded/authorized — so that remains a suggested pre-filing check).

## 5. System information (CLI-composed — see honesty note)

Gathered without the editor GUI: `--version` (build id) + PowerShell
(`Get-CimInstance Win32_OperatingSystem / Win32_Processor / Win32_VideoController`):

```
OS Caption      : Microsoft Windows 11 专业版 Insider Preview
OS Version/Build : 10.0.28120
CPU Name        : 13th Gen Intel(R) Core(TM) i5-13600KF
GPU Name        : NVIDIA GeForce RTX 4070 Ti
GPU DriverVersion: 32.0.16.2002   (Win32_VideoController format; NVIDIA-numbering ~620.02)
GPU DriverDate  : 2026-06-10
```

Composed into Godot's usual System-Information template:

```
Windows 11 (build 28120, Insider Preview) - Godot v4.7.1.stable.official [a13da4feb8d8aefc283c3763d33a2f170a18d541] - Direct3D 12 (Forward+) - NVIDIA GeForce RTX 4070 Ti (nvidia; 32.0.16.2002 / ~620.02) - 13th Gen Intel(R) Core(TM) i5-13600KF
```

> **Honesty note:** this string is **CLI-composed** (engine `--version` + PowerShell CIM
> queries), **not** the editor's *Help → Copy System Info* verbatim output. The GPU driver
> version is the Windows `DriverVersion` field (`32.0.16.2002`); the editor / Vulkan report
> would show NVIDIA's own numbering (~620.02, matching the `620.2.0.0` seen in the VVL
> desktop-control pack). At file time the owner should paste the editor's real
> *Copy System Info* line in place of this composed one. The OS here is a Windows 11 Insider
> Preview build (28120) — a normal retail Windows 11 build number is expected on other
> machines.

## Summary

| # | Draft `<FILL>` | Measured result |
|---|---|---|
| 1 | stock build commit hash | `4.7.1.stable.official.a13da4feb` (full `a13da4feb8d8aefc283c3763d33a2f170a18d541`) |
| 2 | Vulkan control + earlier stable | Vulkan `--rendering-driver vulkan`: **clean**, exit 0, 300 frames. Earlier stable (4.3/4.4): **not tested this pass** (only 4.7.1-stable downloaded) |
| 3 | System information | composed string in §5 (CLI-composed, annotated) |
| 4 | Reproduction on stock | **Reproduces** on official 4.7.1-stable: `0x887a0005` device removal, exit `0xC0000005` |
