; =============================================================================
; Form D: llvm.dx.resource.load.typedbuffer with target("dx.Texture2D<float>", 0, 0)
; =============================================================================
; Label:   D
; Spec:    grx009_texture_intrinsic_toolchain_blocker_evidence
; Intrinsic under test:   llvm.dx.resource.load.typedbuffer  (typedbuffer name, NOT texture name)
; Target-ext type tested: target("dx.Texture2D<float>", 0, 0)  (texture type, possibly mismatched)
; Expected result:        ACCEPT
;                         llc produces case_D.obj
; Proves: llc accepts the typedbuffer intrinsic even when the handle's
;         target-ext type is dx.Texture2D<float>. IR-level selection does
;         NOT cross-validate the resource kind against the intrinsic name;
;         it only checks the intrinsic name exists. (Whether the resulting
;         DXIL is semantically valid at the driver layer is a separate
;         question -- this case proves only IR-level acceptance.)
;
; Verification target:
;   llc.exe case_D.ll -filetype=obj -o case_D.obj
;   exit code == 0; case_D.obj produced
; =============================================================================

target triple = "dxil-unknown-shadermodel6.0-compute"

define void @main() {
entry:
  %h = call target("dx.Texture2D<float>", 0, 0) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %r = call { float, i1 } @llvm.dx.resource.load.typedbuffer(target("dx.Texture2D<float>", 0, 0) %h, i32 0)
  ret void
}
