> **Status: DRAFT — do NOT file.** Owner review gate; agent does not file externally.

# symbolication log — VVL 1.4.350.1 arm64 crash frames (BuildId match, stripped binary)

Verbatim commands + outputs from attempting to symbolize the six `libVkLayer_khronos_validation.so`
frames (#00–#05) of the 2026-07-16 Adreno/MTE SIGSEGV against the **official** VVL 1.4.350.1
arm64 binary.

- Date: 2026-07-17
- Owner authorization: 白栀 authorized this download + on-machine analysis in the 2026-07-17
  session (binaries live in scratch, not committed).
- Layer binary: `libVkLayer_khronos_validation.so` (arm64-v8a) from the official
  KhronosGroup **vulkan-sdk-1.4.350.1** android-binaries release, 26,345,704 B — the exact
  redistributable whose BuildId appears in the crash tombstone.
- Tools: NDK 27.3.13750724 `llvm-readelf.exe` and `llvm-symbolizer.exe` (LLVM 18.0.4).
- Crash frames from `evidence/mb1-android-ondevice/round1_halt_excerpt.md` (= the ISSUE_DRAFT
  backtrace). Android tombstone `pc` values are library-relative offsets; the `.so`'s first
  `LOAD` segment has vaddr 0, so the offsets are fed directly to the symbolizer as addresses.

---

## 1. BuildId comparison (readelf -n)

```
$ llvm-readelf.exe -n libVkLayer_khronos_validation.so
Displaying notes found in: .note.gnu.build-id
  Owner                Data size 	Description
  GNU                  0x00000014	NT_GNU_BUILD_ID (unique build ID bitstring)
    Build ID: 13204c6e71811fabb9fd173b89b19c786d8337b4
```

| Source | Build ID |
|---|---|
| Tombstone frame #00 (`round1_halt_excerpt.md`) | `13204c6e71811fabb9fd173b89b19c786d8337b4` |
| This official 1.4.350.1 arm64 `.so` (readelf) | `13204c6e71811fabb9fd173b89b19c786d8337b4` |

**MATCH (byte-for-byte).** This is the exact binary that crashed on the device.

## 2. Symbolization attempt (all six layer frames)

```
$ llvm-symbolizer.exe --version
LLVM version 18.0.4

$ for pc in 0x1283494 0x128a064 0x12a291c 0x129a0e8 0x12df32c 0xb72f08 ; do
    llvm-symbolizer.exe --obj=libVkLayer_khronos_validation.so --demangle "$pc"
  done
# frame #00 pc=0x1283494 :
??
??:0:0
# frame #01 pc=0x128a064 :
??
??:0:0
# frame #02 pc=0x12a291c :
??
??:0:0
# frame #03 pc=0x129a0e8 :
??
??:0:0
# frame #04 pc=0x12df32c :
??
??:0:0
# frame #05 pc=0xb72f08 :
??
??:0:0
```

**All six frames resolve to `??` — no function names.**

## 3. Why: the official release binary is stripped

```
$ llvm-readelf.exe -S libVkLayer_khronos_validation.so   (symbol/debug sections)
  [ 3] .dynsym   DYNSYM   00000000000002f8 ... 000d98   (present — dynamic exports only)
  [14] .text     PROGBITS 0000000000b36270 ... c0b670
  # NO .symtab, NO .debug_* sections   (grep -c symtab = 0)

$ llvm-readelf.exe -l ...   (first LOAD segment)
  LOAD  0x000000  vaddr 0x0000000000000000  ...  R E   -> tombstone offset == vaddr

$ llvm-readelf.exe --dyn-syms ...   (140 FUNC entries; nonzero-addr tail, sorted)
  0x0000000000f1ec88   4  vkGetInstanceProcAddr
  0x0000000000f1ec8c   4  vkGetDeviceProcAddr
  0x0000000000f1ec90  24  vkEnumerateInstanceLayerProperties
  0x0000000000f1eca8  88  vkEnumerateInstanceExtensionProperties
  0x0000000000f1ed00  52  vkNegotiateLoaderLayerInterfaceVersion
  0x0000000000f1ed34  24  vkEnumerateDeviceLayerProperties
  0x0000000000f1ed4c 136  vkEnumerateDeviceExtensionProperties   (highest export; ends ~0xf1edd4)
```

The binary carries **only `.dynsym`** (the ~140 exported Vulkan/loader entry points) — no
`.symtab`, no `.debug_*`. The exported FUNC symbols all cluster at `0xf1ec88–0xf1edd4`. None of
the six crash PCs fall inside any exported function:

| frame | pc | region vs exports (`0xf1ec88–0xf1edd4`) |
|---|---|---|
| #05 | `0x00b72f08` | **below** all exports (low `.text`, dispatch/intercept) — no covering symbol |
| #04 | `0x012df32c` | **above** all exports (high `.text`, internal) — no covering symbol |
| #03 | `0x0129a0e8` | above all exports — no covering symbol |
| #02 | `0x012a291c` | above all exports — no covering symbol |
| #01 | `0x0128a064` | above all exports — no covering symbol |
| #00 | `0x01283494` | above all exports — no covering symbol |

All six PCs are in **internal (static) functions** that the stripped redistributable does not
name. The symbolizer therefore cannot resolve them even though it has the exact-BuildId binary.

## Conclusion (honest outcome)

- **BuildId: MATCH.** The on-hand official 1.4.350.1 arm64 `.so` is byte-for-byte the crashing
  binary (`13204c6e…`).
- **Symbolization: not possible from this binary.** The official redistributable release is
  **stripped** (`.dynsym`-only; the six crash PCs lie outside every exported symbol), so the
  layer frames cannot be resolved to function names here. **No names are fabricated.**
- **Effect on the draft's `<FILL: exact Vulkan entry point>`:** it stays **pending**, but the
  reason is now sharpened and demonstrated: it is *not* "the symbol package is not on hand" —
  the exact-BuildId binary **is** on hand and confirmed as the crasher; rather, resolving the
  six frames requires an **unstripped / debug (`.sym`) build of VVL 1.4.350.1**, which the
  Khronos android-binaries redistributable does not ship. The desktop control's
  `vkCreateShaderModule` VUID (`pCode-08737`) remains the only (cross-version, 1.3.296)
  side-evidence that the crash is on the shader-module-creation path.
