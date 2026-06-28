# RD-013 Shader-Entry Body Dataflow Lowering — Implementation Feasibility Preflight (2026-06-28)

Status: **blocked-honest preflight**. This note is the implementation-feasibility
evaluation requested as the front-half of an "RD-013 unblock PR": can the minimal
graphics=B shader-entry body dataflow lowering slice (signature input read +
constant/basic arithmetic + signature output write) be implemented honestly today,
without inventing semantics or crossing an owner-cut boundary?

Conclusion: **no minimal slice can be landed as an AI-authored PR without first
obtaining owner-scoped spec/semantics decisions.** No code, spec clause, error
code, golden, or CI step is added by this preflight. No G-G2-4 sign-off, no PR-F2
implementation, and no replacement for CI step 48 is claimed.

- Branch: `codex/g2.4-rd013-body-lowering`, forked from `main` at `87f6bb9`.
- This is distinct from the PR #112 (`codex/g2.4-uc04-pr-f2`) PR-F2 scope audit:
  that note records *that* G-G2-4 is blocked-on RD-013; this note records *why*
  the RD-013 minimal body-lowering slice itself cannot be implemented honestly
  right now, based on tracing the live codegen pipeline.

## Scope Checked

- `src/rurixc/src/dxil_spirv.rs` — the SPIR-V emitter (`emit_spirv`).
- `src/rurixc/src/dxil_codegen.rs` — graphics=B dispatch and B-chain wiring
  (`classify_stage`, `dispatch_and_emit`, `emit_dxil_b`, `run_b_chain`).
- `src/rurixc/src/mir.rs` — `Body`, `IoSigElem`, `Statement`/`Rvalue`/`Place`.
- `src/rurixc/src/mir_build.rs` — `attach_graphics_io_sig`, `dxil_io::io_sig_for`.
- `spec/dxil_backend.md` — RXS-0159 (incl. IR4), RXS-0161, RXS-0162, RD-013 note.
- `registry/deferred.json` — RD-013 reason/backfill, RD-021 texture boundary.

## Findings (machine facts)

1. **The body is never threaded into the SPIR-V emitter — by design.**
   `emit_spirv(stage, io_sig, resources)` takes only the signature type-face and
   emits a trivial passthrough `main` (`OpFunction` / `OpLabel` / `OpReturn` /
   `OpFunctionEnd`). The dispatch `dispatch_and_emit(.., body, ..)` calls
   `emit_dxil_b(stage, &body.io_sig, &body.resources)`; `body.blocks`
   (statements / rvalues / terminators) are not passed anywhere on the B path.
   To lower a real body, `body.blocks` + `locals` must first be threaded
   through `emit_dxil_b` → `run_b_chain` → `emit_spirv`.

2. **`io_sig` is built from AST field annotations, not from the body.**
   `attach_graphics_io_sig` → `dxil_io::io_sig_for` extracts `IoSigElem`s from the
   shader function's parameter/return I/O struct field annotations
   (`#[builtin]` / `#[interpolate]` / plain varying). It is pure signature
   metadata. It has no relationship to the body's `LocalIdx` / `Place` /
   `ProjElem::Field` projections.

3. **No MIR↔signature binding exists, and defining it is a semantics decision.**
   To lower "read input element → compute → write output element", the emitter
   needs a rule binding MIR places to `io_sig` Input/Output elements (e.g. param
   struct field-order ↔ In elements; return struct field-order ↔ Out elements).
   That rule exists in neither the spec nor the code today. Establishing it is a
   source-level shader-I/O-access language semantics decision, not a mechanical
   transcription.

4. **The spec deliberately reserves body dataflow to RD-013.**
   `spec/dxil_backend.md` RXS-0159 IR4 states the clause covers the I/O
   *signature type-face* only; entry body dataflow lowering (statement-level
   codegen that actually reads/writes I/O) is RD-013 and out of scope. There is
   no landed clause defining the body dataflow semantics, so per AGENTS hard
   rule 7 (clause-before-implementation) an implementation PR is not yet
   admissible — and authoring those clauses defines new language semantics,
   which is owner scope (hard rule 1).

5. **Body dataflow is registered as coupled to the §4.6 ABI forbidden zone.**
   RD-013's own `reason` ties full body dataflow lowering to signature ABI
   layout (registers/offsets, RFC-0003 §4.6 🔒 FFI ABI forbidden zone) and to a
   device-codegen statement-lowering extension (the RX6001/RX6003 host/device
   subset boundary). The minimal slice cannot adopt a binding that pre-commits
   to that layout without crossing the owner-cut boundary.

6. **The backfill DoD requires a validator-accepted end-to-end golden.**
   RD-013's `backfill_condition` requires real body dataflow codegen plus a
   dxc-validator-accepted end-to-end golden. Per `spec/dxil_backend.md` RXS-0162
   IR5 and the bless discipline, that golden must be blessed in an owner-pinned
   signed DXC environment. This machine has no signed DXC pin, so any locally
   produced artifact could only be marked NOT BLESSED / pending-human-review and
   could not close RD-013 regardless of implementation.

## Boundary Stop (per task stop-condition)

The minimal slice requires (a) defining source-level shader body I/O dataflow
semantics, and (b) defining the MIR→signature binding. Both are beyond the
owner-cut graphics=B "type-face only" boundary (RXS-0159) and would constitute
inventing semantics. Therefore the work stops here and is registered blocked,
rather than implemented. RD-021 (texture path memory model) is **not** touched:
the minimal arithmetic slice operates only on scalar/vector `MirIoType` I/O
elements and consumes no resource/sampler access, so it does not approach the
RFC-0006 §4.2 texture forbidden zone — but it is independently blocked by the
items above.

## What an RD-013 unblock PR would need (DoD, for the owner)

1. An owner-authorized spec clause (shader_stages.md / dxil_backend.md) defining
   how a Rurix shader body reads its declared inputs and writes its declared
   outputs, and the MIR→signature element binding — without committing to the
   §4.6 binary ABI layout (which stays owner Full RFC).
2. Threading `body.blocks` + `locals` into the B path and lowering the minimal
   rvalue set (`Use`, `Const`, scalar/vector `BinaryOp`) to SPIR-V
   `OpLoad`/`OpConstant`/arithmetic/`OpStore`, kept spirv-val clean.
3. Validator-accepted red/green tests and a golden, blessed in the owner-pinned
   signed DXC environment (this machine: NOT BLESSED only).

## Explicit Non-Claims

- Does **not** implement RD-013 body lowering.
- Does **not** implement PR-F2 and does **not** replace CI step 48.
- Does **not** close or sign G-G2-4, and is not an owner sign-off.
- Does **not** touch RD-021 / texture memory-model semantics.
- AI records machine facts only; status flips and semantics decisions are owner
  scope (AGENTS hard rules 1 and 7).
