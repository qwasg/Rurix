; =============================================================================
; Form G: handlefrombinding only, target("dx.TypedBuffer", float, 0, 0, 0) unused
; =============================================================================
; Label:   G
; Spec:    grx009_texture_intrinsic_toolchain_blocker_evidence
; Intrinsic under test:   llvm.dx.resource.handlefrombinding  (no load/store)
; Target-ext type tested: target("dx.TypedBuffer", float, 0, 0, 0)
; Expected result:        CRASH
;                         llc crashes in DXContainer Global Emitter
; Proves: Even the canonical dx.TypedBuffer type crashes the DXContainer
;         Global Emitter when the handle is created but never consumed by
;         a load/store intrinsic. The crash is structural to emitting a
;         resource handle as a global, not specific to texture types.
;
; Verification target:
;   llc.exe case_G.ll -filetype=obj -o case_G.obj
;   exit code != 0; crash
; =============================================================================

target triple = "dxil-unknown-shadermodel6.0-compute"

define void @main() {
entry:
  %h = call target("dx.TypedBuffer", float, 0, 0, 0) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  ret void
}
