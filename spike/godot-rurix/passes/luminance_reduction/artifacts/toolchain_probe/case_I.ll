; =============================================================================
; Form I: llvm.dx.resource.store.typedbuffer with target("dx.TypedBuffer", float, 1, 0, 0)
; =============================================================================
; Label:   I
; Spec:    grx009_texture_intrinsic_toolchain_blocker_evidence
; Intrinsic under test:   llvm.dx.resource.store.typedbuffer
; Target-ext type tested: target("dx.TypedBuffer", float, 1, 0, 0)  (writable: 1)
; Expected result:        CRASH
;                         llc crashes (in store emission path)
; Proves: While the typedbuffer LOAD intrinsic (Form C) is accepted, the
;         typedbuffer STORE intrinsic crashes llc. This means the
;         workaround of substituting typedbuffer for texture intrinsics
;         is viable for LOAD but NOT for STORE. Store needs a different
;         workaround (e.g. rawbuffer store, or lowering via DXIL ops).
;
; Verification target:
;   llc.exe case_I.ll -filetype=obj -o case_I.obj
;   exit code != 0; crash
; =============================================================================

target triple = "dxil-unknown-shadermodel6.0-compute"

define void @main() {
entry:
  %h = call target("dx.TypedBuffer", float, 1, 0, 0) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  call void @llvm.dx.resource.store.typedbuffer(target("dx.TypedBuffer", float, 1, 0, 0) %h, i32 0, float 1.0)
  ret void
}
