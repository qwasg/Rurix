; =============================================================================
; Form B: llvm.dx.resource.load.texture.2d with target("dx.Texture2D<float>")
; =============================================================================
; Label:   B
; Spec:    grx009_texture_intrinsic_toolchain_blocker_evidence
; Intrinsic under test:   llvm.dx.resource.load.texture.2d
; Target-ext type tested: target("dx.Texture2D<float>")  (no ", 0, 0" suffix)
; Expected result:        REJECT
;                         llc reports: unknown intrinsic 'llvm.dx.resource.load.texture.2d'
; Proves: The reject is keyed on the intrinsic NAME, not the target-ext type
;         suffix. Even with the unsuffixed type form, llc still fails to
;         recognize llvm.dx.resource.load.texture.2d.
;
; Verification target:
;   llc.exe case_B.ll -filetype=obj -o case_B.obj
;   exit code != 0; stderr contains "unknown intrinsic 'llvm.dx.resource.load.texture.2d'"
; =============================================================================

target triple = "dxil-unknown-shadermodel6.0-compute"

define void @main() {
entry:
  %h = call target("dx.Texture2D<float>") @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %r = call { float, i1 } @llvm.dx.resource.load.texture.2d(target("dx.Texture2D<float>") %h, i32 0, i32 0)
  ret void
}
