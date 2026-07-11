; =============================================================================
; Form H: llvm.dx.resource.store.texture.2d with target("dx.RWTexture2D<float>", 0, 0)
; =============================================================================
; Label:   H
; Spec:    grx009_texture_intrinsic_toolchain_blocker_evidence
; Intrinsic under test:   llvm.dx.resource.store.texture.2d
; Target-ext type tested: target("dx.RWTexture2D<float>", 0, 0)
; Expected result:        REJECT
;                         llc reports: unknown intrinsic 'llvm.dx.resource.store.texture.2d'
; Proves: llc rejects the texture STORE intrinsic by name, mirroring Form A
;         for the load direction. This is the upstream toolchain blocker
;         for texture store. Both load and store texture intrinsics are
;         equally unsupported.
;
; Verification target:
;   llc.exe case_H.ll -filetype=obj -o case_H.obj
;   exit code != 0; stderr contains "unknown intrinsic 'llvm.dx.resource.store.texture.2d'"
; =============================================================================

target triple = "dxil-unknown-shadermodel6.0-compute"

define void @main() {
entry:
  %h = call target("dx.RWTexture2D<float>", 0, 0) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  call void @llvm.dx.resource.store.texture.2d(target("dx.RWTexture2D<float>", 0, 0) %h, i32 0, i32 0, float 1.0)
  ret void
}
