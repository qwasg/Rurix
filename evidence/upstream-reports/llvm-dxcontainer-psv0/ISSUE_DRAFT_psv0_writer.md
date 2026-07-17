> **Status: DRAFT — do NOT file.** Owner review gate; agent does not file externally.

> **STATUS — already filed upstream; this document is a back-pack record, not a to-be-filed draft.**
> This PSV0 defect was already reported upstream by the agent during the G2.2 milestone as
> **llvm/llvm-project PR [#205546](https://github.com/llvm/llvm-project/pull/205546)** (opened
> 2026-06-24; source of record: `registry/deferred.json` **RD-011** history). This file is the
> **back-pack (evidence-archive) form** of that report, kept for provenance under EA1 side-branch
> B. The `DRAFT — do NOT file` header therefore means **do NOT initiate any new filing** (issue,
> PR, discussion, or comment) — the upstream PR is the single live channel; its progress is
> tracked in RD-011, not here. Agent does not act on the upstream repo. See `PROVENANCE.md` for
> the caliber-reconciliation note surfaced to the owner.

---

# Upstream defect record — llvm/llvm-project (DirectX backend)

## Title

`DXContainerGlobals` writes the `PSV0` part at the maximum PSV version (v3 = 52 B) ignoring the
module validator version, so a module without `dx.valver` fails validation with
`0x80aa0013 PSVRuntimeInfoSize` (`'PSV0' part:('52')` vs `DXIL module:('24')`)

## Environment

| Item | Value |
|---|---|
| LLVM | 23.0.0git, source worktree HEAD `82c5bce5` (RelWithDebInfo + Assertions build) |
| Target | `dxil - DirectX Intermediate Language` (`-DLLVM_EXPERIMENTAL_TARGETS_TO_BUILD=DirectX`) |
| Signed validator (cross-check) | DXC / `dxil.dll` / `dxv.exe` **1.9.2602.24** (`d355aa836`, 2026-06 release) |
| Input | `official_cs.ll` (289 B, a minimal `cs_6_0` compute module, **no** `dx.valver` named metadata) |
| Host | Windows 11, x86_64 |

## Describe the issue

The DirectX backend emits the `PSV0` DXContainer part at the **maximum** PSV runtime-info version
regardless of the module's validator version. `DXContainerGlobals::addPipelineStateValidationInfo()`
calls `PSV.finalize(...)` / `PSV.write(OS)` **without a `Version` argument**, which defaults to
`std::numeric_limits<uint32_t>::max()`; `write()` then falls through the version `switch` to the
`default` case and encodes `InfoSize = sizeof(dxbc::PSV::v3::RuntimeInfo) = 52`.

For a module that carries no `dx.valver`, the validator derives validator version `0.0` and
therefore **expects** `PSV0 RuntimeInfoSize = sizeof(v0::RuntimeInfo) = 24`. Writing 52 while the
module implies 24 is an **internal inconsistency inside the same container** — the validator
rejects it with `0x80aa0013 PSVRuntimeInfoSize`.

This is not a validator-version gap: the same 2026 signed validator (DXC 1.9.2602.24) accepts a
DXC-produced container whose `PSV0` is also 52 bytes, because DXC's module and part agree. It is
the LLVM-emitted container that is internally inconsistent (round-7 differential evidence).

### Root cause (localized to function/line)

Write side (emits 52):

- `llvm/lib/Target/DirectX/DXContainerGlobals.cpp:388-389` —
  `PSV.finalize(MMI.ShaderProfile)` / `PSV.write(OS)` pass **no** `Version`.
- `llvm/include/llvm/MC/DXContainerPSVInfo.h:74-82` — default
  `Version = std::numeric_limits<uint32_t>::max()`.
- `llvm/lib/MC/DXContainerPSVInfo.cpp:73-90` — `write()` `switch (Version)`: `max` hits `default`
  → `InfoSize = sizeof(dxbc::PSV::v3::RuntimeInfo)` (v0 = 24 / v1 = 36 / v2 = 48 / v3 = 52).

Expected side (implies 24 for a module without `dx.valver`):

- `llvm/lib/Analysis/DXILMetadataAnalysis.cpp:43-50` — `getNamedMetadata("dx.valver")` empty →
  `ValidatorVersion` stays an empty `VersionTuple{}`.
- `llvm/lib/Target/DirectX/DXILTranslateMetadata.cpp:291-293` (`emitValidatorVersionMD`) —
  `if (MMDI.ValidatorVersion.empty()) return;` → **no** `dx.valver` in the output module → the
  validator derives valver `0.0` → expects PSV `v0` = 24 B.

Result: part 52 (v3) vs module-implied 24 (v0) → `0x80aa0013`.

## Steps to reproduce

Emit a DXContainer object from a module without `dx.valver`, then validate:

```
llc official_cs.ll -filetype=obj -o official_cs.obj
dxv.exe official_cs.obj
```

**Observed** (measured 2026-07-17, this back-pack — see `repro_log_20260717.md` for verbatim
commands and output):

- An **unpatched** LLVM emits `PSV0 InfoSize = 52`; `dxv.exe` prints
  `error: DXIL container mismatch for 'PSVRuntimeInfoSize' between 'PSV0' part:('52') and DXIL module:('24')` → `Validation failed.` (exit 1).
- With the fix below applied, LLVM emits `PSV0 InfoSize = 24`; `dxv.exe` prints
  `Validation succeeded.` (exit 0). Adding `!dx.valver = !{i32 1, i32 8}` to the module makes the
  fixed emitter write 52 again, which also validates — i.e. the written version now tracks the
  module (round-8 `post_valver18` sub-case).

## Proposed fix

Derive the PSV version from the module's validator version and pass it to `finalize`/`write`, so
the emitted `PSVRuntimeInfoSize` matches what the validator computes from the module (mirrors DXC).
Minimal local proof-of-concept diff on `DXContainerGlobals.cpp`:

```diff
@@ -385,8 +385,21 @@ void DXContainerGlobals::addPipelineStateValidationInfo(
       MMI.ShaderProfile != Triple::RootSignature)
     PSV.EntryName = MMI.EntryPropertyVec[0].Entry->getName();

-  PSV.finalize(MMI.ShaderProfile);
-  PSV.write(OS);
+  // Derive the PSV version from the module validator version so the emitted
+  // PSVRuntimeInfoSize matches what the validator computes from the module
+  // (mirrors DXC). Without this, PSV is always written at the max version
+  // (v3=52B) while a module lacking dx.valver makes the validator expect
+  // v0 (24B) -> 0x80aa0013 PSVRuntimeInfoSize mismatch.
+  uint32_t PSVVersion = 3;
+  VersionTuple ValVer = MMI.ValidatorVersion;
+  if (ValVer.empty() || ValVer < VersionTuple(1, 1))
+    PSVVersion = 0;
+  else if (ValVer < VersionTuple(1, 6))
+    PSVVersion = 1;
+  else if (ValVer < VersionTuple(1, 8))
+    PSVVersion = 2;
+  PSV.finalize(MMI.ShaderProfile, PSVVersion);
+  PSV.write(OS, PSVVersion);
   addSection(M, Globals, Data, "dx.psv0", "PSV0");
 }
```

**Note on the final upstream shape.** The as-filed upstream PR #205546 is **not** an isolated
14-line change (+93/-4 across 4 files): it factors the derivation into a `getPSVVersion()` helper
and updates two pre-existing DirectX codegen tests (`RuntimeInfoCS.ll`, `PipelineStateValidation.ll`,
`valver 1.7 → 1.8`) plus a new `PSVVersionFromValidatorVersion.ll`. Those existing tests encoded
the buggy "always write PSV v3, ignore `dx.valver`" behavior, so fixing the derivation necessarily
updates their expectations. This is a localized fix with a semantic ripple through existing tests,
not an isolated single point. (Precision correction recorded in RD-011 history and
`spike/dxil-path-probe/dxil_psv_patch_recipe.md` §8.)

## Upstream status

Filed as **llvm/llvm-project PR [#205546](https://github.com/llvm/llvm-project/pull/205546)**
(OPEN, base `main`, +93/-4, 4 files; local DirectX lit 465/465 pass, new test pre-fail/post-pass,
2026 signed validator post 25/25 accept). Do **not** open a new report — track progress on the
existing PR and in `registry/deferred.json` RD-011.
