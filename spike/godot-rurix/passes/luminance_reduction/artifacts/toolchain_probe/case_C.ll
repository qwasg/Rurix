; =============================================================================
; Form C: llvm.dx.resource.load.typedbuffer with target("dx.TypedBuffer", float, 0, 0, 0)
; =============================================================================
; Label:   C
; Spec:    grx009_texture_intrinsic_toolchain_blocker_evidence
; Intrinsic under test:   llvm.dx.resource.load.typedbuffer
; Target-ext type tested: target("dx.TypedBuffer", float, 0, 0, 0)
; Expected result:        ACCEPT
;                         llc produces case_C.obj
; Proves: The typedbuffer load intrinsic IS recognized by llc, and the
;         dx.TypedBuffer target-ext type is the canonical accepted form.
;         This is the workaround basis for non-texture resources.
;
; Verification target:
;   llc.exe case_C.ll -filetype=obj -o case_C.obj
;   exit code == 0; case_C.obj produced
; =============================================================================

target triple = "dxil-unknown-shadermodel6.0-compute"

define void @main() {
entry:
  %h = call target("dx.TypedBuffer", float, 0, 0, 0) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %r = call { float, i1 } @llvm.dx.resource.load.typedbuffer(target("dx.TypedBuffer", float, 0, 0, 0) %h, i32 0)
  ret void
}
