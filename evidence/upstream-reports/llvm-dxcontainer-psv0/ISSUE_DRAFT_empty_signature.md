> **Status: DRAFT — do NOT file.** Owner review gate; agent does not file externally.

> **STATUS — supplementary material for an existing upstream issue; not a to-be-filed draft.**
> This concerns a DirectX-backend gap that is **already tracked upstream** as
> **llvm/llvm-project#[90504](https://github.com/llvm/llvm-project/issues/90504)** ("support
> graphics shader" signatures), which the backend source itself references via a `// FIXME`. This
> file is a **supplementary-material draft** (measured reproduction + localized root cause) that
> the owner may attach to the existing issue if useful. The `DRAFT — do NOT file` header means
> **do NOT open a new issue** and **do NOT post to #90504** — the agent does not act on the
> upstream repo; the owner decides whether/how to contribute. Reference: `registry/deferred.json`
> RD-011 (2026-06-25 history) and RD-013.

---

# Upstream gap record — llvm/llvm-project (DirectX backend graphics signatures)

## Title

DirectX backend `DXContainerGlobals::addSignature()` unconditionally writes **empty** `ISG1`/`OSG1`
signature parts (`elemcount = 0`) for graphics shaders — no module metadata is consulted, so no
input/output signature element ever reaches the DXContainer (tracks existing issue #90504)

## Environment

| Item | Value |
|---|---|
| llc | `H:\llvm-clean-82c5bce5-build\bin\llc.exe`, LLVM 23.0.0git, `Optimized build with assertions`, dxil target present |
| Signed validator (cross-check) | DXC / `dxil.dll` / `dxv.exe` 1.9.2602.24 (2026 signed validator) |
| Inputs | `tests/dxil/vs_io.dxil-ll` (vertex; `!rurix.dxil.sig.in={vid:SV_VertexID}`, `.sig.out={pos:SV_Position, uv}`) and `tests/dxil/ps_io.dxil-ll` (fragment; `.sig.in={coord:SV_Position, uv}`, `.sig.out={color:SV_Target}`); entry bodies are `void` stubs |

## Describe the issue

For graphics shaders the DirectX backend emits `ISG1` (input signature) and `OSG1` (output
signature) DXContainer parts, but always emits them **empty**. The signature elements implied by
the module (system values such as `SV_Position` / `SV_Target` / `SV_VertexID`, interpolation
modifiers, etc.) are never lowered into the parts — regardless of which metadata form encodes them.

`vs_io` / `ps_io` emitted through `llc -filetype=obj` produce containers whose parts are
`['DXIL', 'SFI0', 'HASH', 'ISG1', 'OSG1', 'PSV0']`, and the containers **validate** (IDxcValidator
25/25 accept, `dxv.exe` `Validation succeeded.`). But that acceptance is because an **empty**
signature is a structurally valid part for a `void` entry — **not** because the SV mapping reached
the product:

```
=== vs_io
  ISG1 size 8 elemcount 0
  OSG1 size 8 elemcount 0
=== ps_io
  ISG1 size 8 elemcount 0
  OSG1 size 8 elemcount 0
```

Both `ISG1` and `OSG1` are `size = 8` (an 8-byte header only: `ParamCount = 0`, `ParamOffset = 8`)
with `elemcount = 0` — no SV element of any kind. `accept ≠ signature contains SV`.

### Root cause (localized to function)

`llvm/lib/Target/DirectX/DXContainerGlobals.cpp` `addSignature()`:

```cpp
void DXContainerGlobals::addSignature(Module &M,
                                      SmallVector<GlobalValue *> &Globals) {
  // FIXME: support graphics shader.
  //  see issue https://github.com/llvm/llvm-project/issues/90504.
  Signature InputSig;
  Globals.emplace_back(buildSignature(M, InputSig, "dx.isg1", "ISG1"));
  Signature OutputSig;
  Globals.emplace_back(buildSignature(M, OutputSig, "dx.osg1", "OSG1"));
}
```

- `InputSig` / `OutputSig` are constructed empty and **never** `addParam`'d, so an 8-byte empty
  part is written directly. The backend reads **no** module metadata to populate a graphics
  signature (the `// FIXME` explicitly points at #90504).
- Filling a signature element requires
  `llvm/include/llvm/MC/DXContainerPSVInfo.h` `Signature::addParam(... uint32_t Register,
  uint8_t Mask, uint8_t ExclusiveMask ...)` — i.e. a register/component-mask **binary layout**. A
  search of the backend + `mcdxbc` finds **no** call site that populates graphics `ISG1`/`OSG1` via
  `addParam` (root-signature `addParameter` is unrelated).

**Implication:** because no consumer exists, this is not a metadata-namespace problem — no metadata
form (project-specific `rurix.*`, `hlsl.*`, or standard `dx.entryPoints` signature elements)
currently lowers into `ISG1`/`OSG1`. The gap is the missing backend consumer, exactly as #90504
records.

## Adjacent robustness observation (supporting, from the same container-writer area)

During the round-4 / round-5 spikes the `obj` DXContainer **writer** exhibited a non-deterministic
crash (`0xC0000005`) when writing the container out (the ASCII-DXIL text path was stable at 96/96;
the crash was specific to the `DXContainer` object writer). This is recorded here only as an
adjacent robustness signal in the same code area — not the subject of #90504, and not independently
re-localized in this back-pack.

## Upstream status

Existing upstream issue **llvm/llvm-project#[90504](https://github.com/llvm/llvm-project/issues/90504)**
("support graphics shader"). Do **not** open a new issue; the owner decides whether the measured
reproduction above is worth attaching. Landing graphics-signature support upstream is the
prerequisite for lowering SV semantics into the product (tracked in RD-011 / RD-013).
