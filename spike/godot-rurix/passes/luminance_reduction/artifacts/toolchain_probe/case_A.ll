; =============================================================================
; Form A: llvm.dx.resource.load.texture.2d with target("dx.Texture2D<float>", 0, 0)
; =============================================================================
; Label:   A
; Spec:    grx009_texture_intrinsic_toolchain_blocker_evidence
; Intrinsic under test:   llvm.dx.resource.load.texture.2d
; Target-ext type tested: target("dx.Texture2D<float>", 0, 0)
; Expected result:        REJECT
;                         llc reports: unknown intrinsic 'llvm.dx.resource.load.texture.2d'
; Proves: llc (LLVM 22.1.7 at H:\llvm-dxil\build\bin\llc.exe) rejects the
;         texture load intrinsic by name. This is the form rurixc currently
;         emits, and it is the upstream toolchain blocker for texture load.
;
; Verification target:
;   llc.exe case_A.ll -filetype=obj -o case_A.obj
;   exit code != 0; stderr contains "unknown intrinsic 'llvm.dx.resource.load.texture.2d'"
; =============================================================================

target triple = "dxil-unknown-shadermodel6.0-compute"

define void @main() {
entry:
  %h = call target("dx.Texture2D<float>", 0, 0) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %r = call { float, i1 } @llvm.dx.resource.load.texture.2d(target("dx.Texture2D<float>", 0, 0) %h, i32 0, i32 0)
  ret void
}
