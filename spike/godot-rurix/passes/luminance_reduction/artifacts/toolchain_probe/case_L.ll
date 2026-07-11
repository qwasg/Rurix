; =============================================================================
; Form L: upstream texture load.level + local-patch texture store.texture
; =============================================================================
; Label:   L
; Spec:    grx009_texture_intrinsic_toolchain_blocker_evidence (segment 4-texture)
; Intrinsics under test:
;   llvm.dx.resource.load.level   (SRV Texture2D<float> load, upstream)
;   llvm.dx.resource.store.texture(UAV RWTexture2D<float> store, LOCAL PATCH)
; Target-ext types tested:
;   target("dx.Texture", float, 0, 0, 0, 2)  (SRV: IsWriteable=0)
;   target("dx.Texture", float, 1, 0, 0, 2)  (UAV: IsWriteable=1)
; Expected result:        ACCEPT
;   llc emits a DXIL object; dx.op.textureLoad(66) + dx.op.textureStore(67).
; Proves: the locally-patched llc at H:\llvm-clean-82c5bce5-build\bin\llc.exe
;   (LLVM 23.0.0git + int_dx_resource_store_texture / DXILOp<67> textureStore
;    patch) lowers both the upstream texture LOAD and the newly-added texture
;    STORE to their dx.op.* forms. Load a texel from an SRV texture and store
;    it to a UAV texture at texel (0,0).
;
; Verification target:
;   llc.exe case_L.ll -filetype=obj -o case_L.obj
;   exit code == 0; obj produced; byte-identical across repeated runs.
; =============================================================================

target datalayout = "e-m:e-p:32:32-i1:32-i8:8-i16:16-i32:32-i64:64-f16:16-f32:32-f64:64-n8:16:32:64"
target triple = "dxil-unknown-shadermodel6.0-compute"

define void @main() #0 {
entry:
  %tex = call target("dx.Texture", float, 0, 0, 0, 2) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %dst = call target("dx.Texture", float, 1, 0, 0, 2) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %texel = call float @llvm.dx.resource.load.level(target("dx.Texture", float, 0, 0, 0, 2) %tex, <2 x i32> zeroinitializer, i32 0, <2 x i32> zeroinitializer)
  call void @llvm.dx.resource.store.texture(target("dx.Texture", float, 1, 0, 0, 2) %dst, <2 x i32> zeroinitializer, float %texel)
  ret void
}

attributes #0 = { noinline nounwind "hlsl.numthreads"="1,1,1" "hlsl.shader"="compute" }

!dx.valver = !{!0}

!0 = !{i32 1, i32 8}
