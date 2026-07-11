; =============================================================================
; Form K: upstream texture load.level with target("dx.Texture", float, 0,0,0,2)
; =============================================================================
; Label:   K
; Spec:    grx009_texture_intrinsic_toolchain_blocker_evidence (segment 4-texture)
; Intrinsic under test:   llvm.dx.resource.load.level  (SRV Texture2D<float> load)
; Target-ext type tested: target("dx.Texture", float, 0, 0, 0, 2)
;                         (ElemTy=float, IsWriteable=0, IsROV=0, IsSigned=0, Dim=2)
; Expected result:        ACCEPT
;                         llc emits a DXIL object; dx.op.textureLoad(66) lowering.
; Proves: the patched/upstream llc at H:\llvm-clean-82c5bce5-build\bin\llc.exe
;         (LLVM 23.0.0git, contains the merged texture load.level path,
;          PR #193343) recognizes and lowers the UPSTREAM texture load form,
;         unlike the self-invented llvm.dx.resource.load.texture.2d of Form A.
;         The loaded texel is kept live by a rawbuffer store sink (known-good
;         path) so the load survives to DXIL lowering rather than being DCE'd.
;
; Verification target:
;   llc.exe case_K.ll -filetype=obj -o case_K.obj
;   exit code == 0; obj produced; byte-identical across repeated runs.
; =============================================================================

target datalayout = "e-m:e-p:32:32-i1:32-i8:8-i16:16-i32:32-i64:64-f16:16-f32:32-f64:64-n8:16:32:64"
target triple = "dxil-unknown-shadermodel6.0-compute"

define void @main() #0 {
entry:
  %tex = call target("dx.Texture", float, 0, 0, 0, 2) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %buf = call target("dx.RawBuffer", float, 1, 0) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %texel = call float @llvm.dx.resource.load.level(target("dx.Texture", float, 0, 0, 0, 2) %tex, <2 x i32> zeroinitializer, i32 0, <2 x i32> zeroinitializer)
  call void @llvm.dx.resource.store.rawbuffer(target("dx.RawBuffer", float, 1, 0) %buf, i32 0, i32 0, float %texel)
  ret void
}

attributes #0 = { noinline nounwind "hlsl.numthreads"="1,1,1" "hlsl.shader"="compute" }

!dx.valver = !{!0}

!0 = !{i32 1, i32 8}
