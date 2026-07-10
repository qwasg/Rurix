; =============================================================================
; Form E: handlefrombinding only, target("dx.Texture2D<float>", 0, 0) unused
; =============================================================================
; Label:   E
; Spec:    grx009_texture_intrinsic_toolchain_blocker_evidence
; Intrinsic under test:   llvm.dx.resource.handlefrombinding  (no load/store)
; Target-ext type tested: target("dx.Texture2D<float>", 0, 0)
; Expected result:        CRASH
;                         llc crashes in DXContainer Global Emitter
; Proves: The mere presence of a dx.Texture2D<float> handle (with the , 0, 0
;         suffix) triggers a crash in the DXContainer Global Emitter, even
;         when no texture load/store intrinsic is invoked. This isolates
;         the crash to type-emission, not intrinsic selection.
;
; Verification target:
;   llc.exe case_E.ll -filetype=obj -o case_E.obj
;   exit code != 0; crash (stderr may mention DXContainer Global Emitter
;   or access violation)
; =============================================================================

target triple = "dxil-unknown-shadermodel6.0-compute"

define void @main() {
entry:
  %h = call target("dx.Texture2D<float>", 0, 0) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  ret void
}
