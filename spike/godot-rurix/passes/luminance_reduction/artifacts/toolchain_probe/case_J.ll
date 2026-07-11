; =============================================================================
; Form J: llvm.dx.resource.load.rawbuffer with target("dx.RawBuffer", float, 0, 0)
; =============================================================================
; Label:   J
; Spec:    grx009_texture_intrinsic_toolchain_blocker_evidence
; Intrinsic under test:   llvm.dx.resource.load.rawbuffer
; Target-ext type tested: target("dx.RawBuffer", float, 0, 0)
; Expected result:        ACCEPT
;                         llc produces case_J.obj
; Proves: The rawbuffer load intrinsic IS recognized by llc and produces
;         a valid DXContainer object. This is the BASELINE: the form
;         rurixc already uses successfully for raw buffer loads, and a
;         reference point against which the texture/typedbuffer forms
;         are compared.
;
; Verification target:
;   llc.exe case_J.ll -filetype=obj -o case_J.obj
;   exit code == 0; case_J.obj produced
; =============================================================================

target triple = "dxil-unknown-shadermodel6.0-compute"

define void @main() {
entry:
  %h = call target("dx.RawBuffer", float, 0, 0) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %r = call { float, i1 } @llvm.dx.resource.load.rawbuffer(target("dx.RawBuffer", float, 0, 0) %h, i32 0, i32 0)
  ret void
}
