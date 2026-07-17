> **Status: DRAFT — do NOT file.** Owner review gate; agent does not file externally.

# Upstream issue draft — KhronosGroup/Vulkan-ValidationLayers

> Public-facing text for a public repository. Complete every `<FILL: …>` placeholder, resolve the
> `<PENDING: …>` MRP item, and run the pre-filing checklist at the bottom before submitting.
> Evidence source: `evidence/mb1-android-ondevice/round1_halt_excerpt.md` (verbatim crash capture,
> 2026-07-16) and `evidence/mb1-android-ondevice/android_present_smoke_report.md` (environment and
> layer provenance).

---

## Title

Validation layer itself crashes (SIGSEGV / SEGV_ACCERR, use-after-free signature caught by
hardware pointer tagging) instead of reporting pCode-08742 when given byte-corrupted SPIR-V on
Adreno / Android 16

## Environment

| Item | Value |
|---|---|
| VVL version | **1.4.350.1** (`libVkLayer_khronos_validation.so` from vulkan-sdk android-binaries 1.4.350.1; arm64; 26,345,704 B; sha256 `34a741d51cb6e9111ec52cda20eee812bcfbcd197348c1404232aacb60e89ef3`) |
| Crashing binary BuildId (from tombstone) | `13204c6e71811fabb9fd173b89b19c786d8337b4` — **confirmed 2026-07-17** to match `readelf -n` of the official 1.4.350.1 arm64 `.so` byte-for-byte (the on-hand redistributable is the crashing binary; but it is stripped — see the entry-point note below) |
| Device | HONOR BKQ-AN10, arm64 |
| SoC / GPU | Qualcomm SM8850 (Adreno) |
| Adreno driver / Vulkan ICD version | `<FILL / PENDING: not present in any tracked evidence — the EA1 back-pack search of evidence/mb1-android-ondevice/ found only "SM8850 (Adreno系)" and "libvulkan.so present", no driverVersion/ICD string; read from vulkaninfo / GPU driver package on the device (owner-held)>` |
| OS | Android 16 (SDK 36), build fingerprint `HONOR/BKQ-AN10/HNBKQ:16/HONORBKQ-ANXX/10DLDLD160SP1C00E160:user/release-keys` |
| Pointer tagging | Active on the crashing process: `tagged_addr_ctrl: 0000000000000001 (PR_TAGGED_ADDR_ENABLE)` (verbatim from tombstone) |
| Layer loading | Minimal NativeActivity APK (`hasCode=false`, `android:debuggable="true"`), layer `.so` packaged in-APK, enabled via `com.android.graphics.injectLayers.enable`; loader log confirms `added global layer 'VK_LAYER_KHRONOS_validation'` |
| Layer settings | `<FILL: confirm defaults — no vk_layer_settings.txt / VK_EXT_layer_settings override is recorded in the archived evidence>` |

## Describe the issue

An intentionally **byte-corrupted vertex-stage SPIR-V module** (fault-injection RED leg of a
validation smoke test) was fed to shader-module / graphics-pipeline creation, expecting the layer
to report `VUID-VkShaderModuleCreateInfo-pCode-08742` (invalid SPIR-V).

Instead of emitting the VUID, the validation layer **hard-crashed with SIGSEGV
(`SEGV_ACCERR`)** while handling the invalid SPIR-V. The fault address
`0xb400007063e834d4` lies in the `0xb400…` tagged-pointer / scudo heap range; `SEGV_ACCERR` on
such a pointer is the signature of a **use-after-free / pointer-tag mismatch** — i.e. the layer
dereferenced a freed or mistagged pointer somewhere in its invalid-SPIR-V parsing /
error-message-formatting path, and the device's hardware pointer tagging caught it. No
validation message of any kind reached logcat before the process died.

- Corruption recipe: the **exact** offsets/values used by the Android round-1 RED leg were **not
  archived** (that leg was retired in favour of a bogus-entry-point control — see
  `src/rurix-rt/src/vk.rs` `red_selftest`, which no longer feeds corrupted bytes). The repository's
  tracked deterministic byte-corruption recipe is `ci/vulkan_codegen_smoke.py:121-125` —
  `spv_bytes[20] ^= 0xFF` (XOR the first byte of instruction word 5, i.e. the first instruction
  after the 20-byte / 5-word SPIR-V header). The 2026-07-17 desktop control (below) uses this
  same-class recipe; whether the device round-1 leg used these exact offsets is not recorded, so
  the standalone MRP must pin the corrupted `.spv` bytes explicitly. `<FILL: exact device-side
  offsets/values — pin together with the standalone MRP>`
- Vulkan entry point in flight at crash time: expected trigger was
  `vkCreateShaderModule` / pipeline creation (per the test design), but the six layer frames in
  the backtrace are unsymbolized. A 2026-07-17 symbolization pass (NDK 27.3
  `llvm-symbolizer.exe` / `llvm-readelf.exe`) obtained the **exact-BuildId** official VVL
  1.4.350.1 arm64 binary and confirmed `readelf -n` Build ID
  `13204c6e71811fabb9fd173b89b19c786d8337b4` **matches the tombstone byte-for-byte** — i.e. the
  crashing binary itself is on hand. **However, the official Khronos android-binaries
  redistributable is stripped** (`.dynsym`-only: ~140 exported entry points clustered at
  `0xf1ec88–0xf1edd4`, no `.symtab`/`.debug_*`), and all six crash PCs
  (`0xb72f08`, `0x1283494`, `0x128a064`, `0x12a291c`, `0x129a0e8`, `0x12df32c`) lie **outside**
  every exported symbol range — the symbolizer returns `??` for all six. So the exact entry
  point **stays** `<PENDING: symbolize against an unstripped/debug (.sym) build of VVL 1.4.350.1
  — the shipped redistributable is stripped; the exact-BuildId binary is confirmed but carries
  no internal symbols>` (full log: `symbolication_log_20260717.md`). Data point from the
  2026-07-17 desktop control (below): desktop VVL emitted a `vkCreateShaderModule` VUID
  (`pCode-08737`) immediately before the access violation, which is consistent with the crash
  being in the invalid-SPIR-V handling path at/after shader-module creation — but this is a
  desktop 1.3.296 signal, not a symbolization of the device 1.4.350.1 frames.

### Expected behavior

The layer reports `VUID-VkShaderModuleCreateInfo-pCode-08742` (or otherwise cleanly rejects the
invalid SPIR-V). If the layer chooses to abort, it should be a controlled abort (SIGABRT with an
`Abort message` carrying the VUID) — not a segmentation fault.

### Actual behavior

Hard SIGSEGV inside `libVkLayer_khronos_validation.so` before any VUID / "Validation Error" text
is emitted (full logcat buffer scan: 0 hits for either string; also 0 log lines from the app
itself — the process died before producing any output).

### Valid Usage ID

`VUID-VkShaderModuleCreateInfo-pCode-08742` (the VUID the fault injection was designed to
trigger; never emitted).

## Crash backtrace (verbatim logcat, 2026-07-16)

Paths were abbreviated in the archived excerpt (`.../lib/arm64/` = the APK's extracted
native-lib directory); frames #11–#13 are not present in the archived excerpt.

```
--------- beginning of crash
07-16 17:00:50.866 10987 11030 F libc    : Fatal signal 11 (SIGSEGV), code 2 (SEGV_ACCERR), fault addr 0xb400007063e834d4 in tid 11030 (com.rurix.vk), pid 10987 (com.rurix.vk)
07-16 17:00:51.104 11045 11045 F DEBUG   : Build fingerprint: 'HONOR/BKQ-AN10/HNBKQ:16/HONORBKQ-ANXX/10DLDLD160SP1C00E160:user/release-keys'
07-16 17:00:51.104 11045 11045 F DEBUG   : ABI: 'arm64'
07-16 17:00:51.104 11045 11045 F DEBUG   : Cmdline: com.rurix.vk
07-16 17:00:51.104 11045 11045 F DEBUG   : pid: 10987, tid: 11030, name: com.rurix.vk  >>> com.rurix.vk <<<
07-16 17:00:51.104 11045 11045 F DEBUG   : tagged_addr_ctrl: 0000000000000001 (PR_TAGGED_ADDR_ENABLE)
07-16 17:00:51.104 11045 11045 F DEBUG   : signal 11 (SIGSEGV), code 2 (SEGV_ACCERR), fault addr 0xb400007063e834d4
07-16 17:00:51.104 11045 11045 F DEBUG   : 16 total frames
07-16 17:00:51.104 11045 11045 F DEBUG   : backtrace:
07-16 17:00:51.104 11045 11045 F DEBUG   :       #00 pc 0000000001283494  .../lib/arm64/libVkLayer_khronos_validation.so (BuildId: 13204c6e71811fabb9fd173b89b19c786d8337b4)
07-16 17:00:51.104 11045 11045 F DEBUG   :       #01 pc 000000000128a064  .../lib/arm64/libVkLayer_khronos_validation.so
07-16 17:00:51.104 11045 11045 F DEBUG   :       #02 pc 00000000012a291c  .../lib/arm64/libVkLayer_khronos_validation.so
07-16 17:00:51.104 11045 11045 F DEBUG   :       #03 pc 000000000129a0e8  .../lib/arm64/libVkLayer_khronos_validation.so
07-16 17:00:51.104 11045 11045 F DEBUG   :       #04 pc 00000000012df32c  .../lib/arm64/libVkLayer_khronos_validation.so
07-16 17:00:51.104 11045 11045 F DEBUG   :       #05 pc 0000000000b72f08  .../lib/arm64/libVkLayer_khronos_validation.so
07-16 17:00:51.104 11045 11045 F DEBUG   :       #06 pc 00000000000253d4  .../lib/arm64/librurix_vk.so (rurix_rt::vk::present_body::{{closure}}::h8635107f2d791f91+68)
07-16 17:00:51.104 11045 11045 F DEBUG   :       #07 pc 0000000000024848  .../lib/arm64/librurix_vk.so (rurix_rt::vk::present_body::hef55352a118f8926+3840)
07-16 17:00:51.104 11045 11045 F DEBUG   :       #08 pc 0000000000027578  .../lib/arm64/librurix_vk.so (rurix_rt::vk::run_graphics_present_android::h17848797eabd6f4c+2532)
07-16 17:00:51.104 11045 11045 F DEBUG   :       #09 pc 00000000000277fc  .../lib/arm64/librurix_vk.so (rurix_rt::vk::run_graphics_present_android_safe::h0317a6c8e2fc9a64+204)
07-16 17:00:51.104 11045 11045 F DEBUG   :       #10 pc 000000000001f734  .../lib/arm64/librurix_vk.so (rurix_vk::render_thread::h4f66fdfa6903d8f1+1576)
07-16 17:00:51.104 11045 11045 F DEBUG   :       #14 pc 0000000000082858  /apex/com.android.runtime/lib64/bionic/libc.so (__pthread_start(void*)+232)
07-16 17:00:51.104 11045 11045 F DEBUG   :       #15 pc 0000000000075730  /apex/com.android.runtime/lib64/bionic/libc.so (__start_thread+64)
```

## Analysis of the captured evidence

- **The layer was loaded and live** — this is not a "layer failed to load" mode:
  `libVkLayer_khronos_validation.so` owns the **top 6 frames** (#00–#05), entered
  **synchronously** from the application's own frame #06. The layer was on the call path, not
  merely mapped.
- **Crash precedes any output**: full logcat buffer scan found **0** hits for `VUID` /
  `Validation Error`, and **0** log lines from the app (the app logs before writing its result
  file; neither happened).
- **Not a controlled validation abort**: a clean abort-on-error would be SIGABRT (signal 6) with
  an `Abort message` carrying the VUID — neither is present; this is a hard SIGSEGV (signal 11,
  `SEGV_ACCERR`).
- **Use-after-free / tag-mismatch signature** (diagnosis from the captured signals; the layer
  frames are not yet symbolized): fault address `0xb400007063e834d4` is in the `0xb4000070…`
  tagged-pointer / scudo heap range, and `SEGV_ACCERR` on such a pointer indicates the layer
  dereferenced a freed or mistagged pointer while handling the intentionally invalid input.
- **PC cluster shape**: frame #05 at `0xb72f08` (low region — dispatch/intercept entry) leading
  to #00–#04 at `0x128…–0x12d…` (high region of the 26 MB layer — core-validation /
  error-reporting machinery). An intercept → core-validation → crash chain is most consistent
  with the layer crashing **while detecting/formatting the intentional error**, which is why the
  expected clean VUID never surfaced.

## Control experiments

- **Same device, same APK shell, same layer binary (1.4.350.1), fully valid SPIR-V + bogus
  entry-point name** (`pName = "rurix_red_bogus_entry"`, module only exports `main`): the layer
  cleanly reports the intentional error and the process survives — zero crash, verbatim logcat:

  ```
  E RurixVK-VVL: vkCreateGraphicsPipelines(): pCreateInfos[0].pStages[0].pName "rurix_red_bogus_entry" entry point not found for stage VK_SHADER_STAGE_VERTEX_BIT. (The only entry point found was "main" for VK_SHADER_STAGE_VERTEX_BIT)
  E RurixVK-VVL: The Vulkan spec states: pName must be the name of an OpEntryPoint in module with an execution model that matches stage (https://docs.vulkan.org/spec/latest/chapters/pipelines.html#VUID-VkPipelineShaderStageCreateInfo-pName-00707)
  ```

  This isolates the crash to the **invalid-SPIR-V handling path**: the layer install, loading
  mechanism, and error-reporting pipeline are all demonstrably functional on this device when
  the SPIR-V bytes are valid.
- **Desktop control: performed 2026-07-17 — the crash reproduces on desktop VVL without
  hardware pointer tagging.** A same-class byte-corrupted SPIR-V module was fed to
  `vkCreateShaderModule` / graphics-pipeline creation under VVL on Windows/NVIDIA (no MTE), and
  **VVL crashed** (`0xC0000005` STATUS_ACCESS_VIOLATION; shell exit 139), **deterministically
  3/3**. Details:
  - **Environment:** Windows 11, NVIDIA GeForce RTX 4070 Ti (driver 620.2.0.0), VVL
    **1.3.296** (`VkLayer_khronos_validation` from `HKLM\SOFTWARE\Khronos\Vulkan\ExplicitLayers`
    → `C:\…\vulkan-1.3.296.0\Bin`), instance apiVersion 1.4.351. x86_64 has **no** hardware
    pointer tagging (MTE), so this is the "does it reproduce without MTE?" control.
  - **Recipe (same class, deterministic):** the repository's tracked corruption recipe
    (`ci/vulkan_codegen_smoke.py:121-125`, `spv_bytes[20] ^= 0xFF`) applied to a valid vertex
    `.spv` (`conformance/vulkan/accept/vk_tri_vs.rx` compiled with `rurixc --target vulkan`,
    376 B; byte 20 `0x11 → 0xEE`, corrupting the first instruction word). `spirv-val` rejects the
    tampered module (`End of input reached while decoding OpAtomicSMax starting at word 5`).
  - **Harness:** `rurix-rt` `vk_triangle` (loads vs/fs `.spv` from argv), VVL enabled via
    `RURIX_VK_VALIDATION=1` + `VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation`.
  - **Result — VVL on:** exit `0xC0000005` (3/3 runs), **empty stdout** (no `VK_TRIANGLE: ok`).
    Unlike Android, desktop VVL **does** emit one validation message to stderr first —
    `VUID-VkShaderModuleCreateInfo-pCode-08737` ("spirv-val produced an error … OpAtomicSMax …") —
    and then the process access-violates.
  - **Result — VVL off (control of the control):** the **same** tampered module runs **clean**
    (exit 0, `VK_TRIANGLE: ok`, covered=968). The NVIDIA driver tolerates the corrupted module;
    therefore the crash is attributable to **VVL's** processing, not the driver or the harness
    proceeding with a bad handle.
  - **Caveats (do not over-claim):** desktop VVL is **1.3.296**, the device capture is on VVL
    **1.4.350.1** — a version difference. The desktop VUID is `pCode-08737` (spirv-val-error),
    while the Android RED leg targeted `pCode-08742`; the **exact** Android round-1 corruption
    offsets/values were **not** archived (see PROVENANCE), so this uses the repo's tracked
    same-class recipe, not a byte-for-byte replay of the device input. Still, the control answers
    the open question: **the invalid-SPIR-V handling path in VVL is crash-fragile on desktop too,
    not only under Adreno/MTE** — MTE merely caught it harder/earlier (SEGV_ACCERR, before any
    VUID surfaced), whereas desktop got one VUID out before the access violation. This broadens
    the apparent scope beyond "Adreno / Android 16"; the title/scope may warrant revision at file
    time (flagged for owner).

## Minimal reproduction

`<PENDING: standalone MRP not yet extracted>`

The crash was captured inside this project's own test APK (a NativeActivity shell driving a
Rust `cdylib` renderer), which is not a suitable upstream reproducer. A **standalone minimal
reproduction** — a plain C/NDK NativeActivity project with no dependency on this project,
feeding the same corrupted SPIR-V bytes to shader-module / pipeline creation under
VVL 1.4.350.1 — has not yet been produced. Producing it requires the physical device (held by
the project owner); a 2026-07-17 back-pack re-check (`adb devices`) found **no device attached**,
so the standalone MRP and on-device recapture stay **pending** (G-EA1-7 explicitly permits marking
the real-device-dependent sub-item pending rather than fabricating it). The 2026-07-17 **desktop**
control (Windows/NVIDIA, VVL 1.3.296 — see Control experiments) does reproduce a same-class VVL
crash without a device, but a device-side standalone MRP on VVL 1.4.350.1 is still required. This
issue must not be filed until the standalone MRP exists and reconfirms the crash.

---

## Pre-filing checklist (owner — remove this whole section before pasting into GitHub)

1. **Extract the standalone MRP** (plain C/NDK NativeActivity, no Rurix dependency; pin the
   exact corrupted `.spv` bytes) and reconfirm the crash on the device. Do not file without it.
2. **Reconfirm against current VVL** — the capture is on 1.4.350.1 (2026-07-16); the
   invalid-SPIR-V handling path may have changed upstream. Re-run the MRP against the latest
   SDK release and/or VVL `main`; if it no longer reproduces, do not file.
3. **Symbolize the six layer frames** — ⚠️ BLOCKED (2026-07-17). BuildId
   `13204c6e71811fabb9fd173b89b19c786d8337b4` **matches** the official 1.4.350.1 arm64 `.so`
   byte-for-byte, but that redistributable is **stripped** (`.dynsym`-only; all six PCs fall
   outside every export → `llvm-symbolizer` returns `??`). Needs an **unstripped/debug (`.sym`)
   build of VVL 1.4.350.1** to resolve the exact entry point + function names. See
   `symbolication_log_20260717.md`.
4. **Fill the corruption recipe** (exact offsets/values) and attach the corrupted `.spv`, the
   full tombstone, and the full logcat capture.
5. **Run the desktop control** (same corrupted bytes, desktop VVL) and record the result.
6. **Fill remaining environment `<FILL>`s** (Adreno driver / Vulkan ICD version; layer settings
   confirmation).
7. **Search the VVL issue tracker for duplicates** at file time (suggested terms: SPIR-V crash,
   SIGSEGV, MTE, Android, use-after-free, pCode-08742, corrupted SPIR-V).
8. File using the repository's bug template; one bug per issue.
