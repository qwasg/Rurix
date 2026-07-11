; =============================================================================
; Form F: handlefrombinding only, target("dx.Texture2D<float>") unused
; =============================================================================
; Label:   F
; Spec:    grx009_texture_intrinsic_toolchain_blocker_evidence
; Intrinsic under test:   llvm.dx.resource.handlefrombinding  (no load/store)
; Target-ext type tested: target("dx.Texture2D<float>")  (no suffix)
; Expected result:        CRASH
;                         llc crashes in DXContainer Global Emitter
; Proves: The DXContainer Global Emitter crash for dx.Texture2D<float>
;         occurs regardless of whether the ", 0, 0" type suffix is
;         present. The unsuffixed form is not a workaround.
;
; Verification target:
;   llc.exe case_F.ll -filetype=obj -o case_F.obj
;   exit code != 0; crash
; =============================================================================

target triple = "dxil-unknown-shadermodel6.0-compute"

define void @main() {
entry:
  %h = call target("dx.Texture2D<float>") @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  ret void
}
