; ModuleID = 'reorder_particles'
source_filename = "reorder_particles"
target datalayout = "e-m:e-p:32:32-i1:32-i8:8-i16:16-i32:32-i64:64-f16:16-f32:32-f64:64-n8:16:32:64"
target triple = "dxil-unknown-shadermodel6.0-compute"

%__cblayout_reorder_particles = type <{ i64, i64, float }>

define void @rx_k_8() #0 {
entry:
  %rx_cb = call target("dx.CBuffer", %__cblayout_reorder_particles) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %rx_h_particles = call target("dx.RawBuffer", float, 0, 0) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %rx_h_instances = call target("dx.RawBuffer", float, 1, 0) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %v0.u32 = call i32 @llvm.dx.thread.id(i32 0)
  %v0 = zext i32 %v0.u32 to i64
  %v1.ptr = call ptr addrspace(2) @llvm.dx.resource.getpointer(target("dx.CBuffer", %__cblayout_reorder_particles) %rx_cb, i32 0)
  %v1 = load i64, ptr addrspace(2) %v1.ptr
  %v2 = icmp slt i64 %v0, %v1
  br i1 %v2, label %if.then.0, label %if.end.0
if.then.0:
  %v3 = mul i64 %v0, 28
  %v4.ptr = call ptr addrspace(2) @llvm.dx.resource.getpointer(target("dx.CBuffer", %__cblayout_reorder_particles) %rx_cb, i32 8)
  %v4 = load i64, ptr addrspace(2) %v4.ptr
  %v5 = add i64 %v0, %v4
  %v6 = mul i64 %v5, 20
  %v7 = add i64 %v6, 0
  %v8 = add i64 %v3, 0
  %v9.idx = trunc i64 %v8 to i32
  %v9.ld = call { float, i1 } @llvm.dx.resource.load.rawbuffer(target("dx.RawBuffer", float, 0, 0) %rx_h_particles, i32 %v9.idx, i32 0)
  %v9 = extractvalue { float, i1 } %v9.ld, 0
  %v10 = add i64 %v3, 16
  %v11.idx = trunc i64 %v10 to i32
  %v11.ld = call { float, i1 } @llvm.dx.resource.load.rawbuffer(target("dx.RawBuffer", float, 0, 0) %rx_h_particles, i32 %v11.idx, i32 0)
  %v11 = extractvalue { float, i1 } %v11.ld, 0
  %v12.ptr = call ptr addrspace(2) @llvm.dx.resource.getpointer(target("dx.CBuffer", %__cblayout_reorder_particles) %rx_cb, i32 16)
  %v12 = load float, ptr addrspace(2) %v12.ptr
  %v13 = fmul float %v11, %v12
  %v14 = fadd float %v9, %v13
  %v15.idx = trunc i64 %v7 to i32
  call void @llvm.dx.resource.store.rawbuffer(target("dx.RawBuffer", float, 1, 0) %rx_h_instances, i32 %v15.idx, i32 0, float %v14)
  br label %if.end.0
if.end.0:
  ret void
}

attributes #0 = { noinline nounwind "hlsl.numthreads"="1,1,1" "hlsl.shader"="compute" }

!dx.valver = !{!0}

!0 = !{i32 1, i32 8}
