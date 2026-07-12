; ModuleID = 'lib_texture'
source_filename = "lib_texture"
target datalayout = "e-m:e-p:32:32-i1:32-i8:8-i16:16-i32:32-i64:64-f16:16-f32:32-f64:64-n8:16:32:64"
target triple = "dxil-unknown-shadermodel6.0-compute"

%__cblayout_lib_texture = type <{ i64, i64, float, float, float }>

define void @rx_luminance_reduce_level_texture_8() #0 {
entry:
  %rx_cb = call target("dx.CBuffer", %__cblayout_lib_texture) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %rx_h_src_luminance = call target("dx.Texture", float, 0, 0, 0, 2) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %rx_h_dst_luminance = call target("dx.Texture", float, 1, 0, 0, 2) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %local.accum.0.addr = alloca float
  %local.count.1.addr = alloca float
  %local.dy.2.addr = alloca i64
  %local.dx.3.addr = alloca i64
  %v0.ptr = call ptr addrspace(2) @llvm.dx.resource.getpointer(target("dx.CBuffer", %__cblayout_lib_texture) %rx_cb, i32 0)
  %v0 = load i64, ptr addrspace(2) %v0.ptr
  %v1 = icmp sgt i64 %v0, 1
  %v2.ptr = call ptr addrspace(2) @llvm.dx.resource.getpointer(target("dx.CBuffer", %__cblayout_lib_texture) %rx_cb, i32 0)
  %v2 = load i64, ptr addrspace(2) %v2.ptr
  %v3 = add i64 %v2, 7
  %v4 = sdiv i64 %v3, 8
  %v5 = select i1 %v1, i64 %v4, i64 1
  %v6.ptr = call ptr addrspace(2) @llvm.dx.resource.getpointer(target("dx.CBuffer", %__cblayout_lib_texture) %rx_cb, i32 8)
  %v6 = load i64, ptr addrspace(2) %v6.ptr
  %v7 = icmp sgt i64 %v6, 1
  %v8.ptr = call ptr addrspace(2) @llvm.dx.resource.getpointer(target("dx.CBuffer", %__cblayout_lib_texture) %rx_cb, i32 8)
  %v8 = load i64, ptr addrspace(2) %v8.ptr
  %v9 = add i64 %v8, 7
  %v10 = sdiv i64 %v9, 8
  %v11 = select i1 %v7, i64 %v10, i64 1
  %v12 = mul i64 %v5, %v11
  %v13.u32 = call i32 @llvm.dx.thread.id(i32 0)
  %v13 = zext i32 %v13.u32 to i64
  %v14 = icmp slt i64 %v13, %v12
  br i1 %v14, label %if.then.4, label %if.end.4
if.then.4:
  %v15 = srem i64 %v13, %v5
  %v16 = sdiv i64 %v13, %v5
  %v17 = mul i64 %v15, 8
  %v18 = mul i64 %v16, 8
  store float 0.0, ptr %local.accum.0.addr
  store float 0.0, ptr %local.count.1.addr
  store i64 0, ptr %local.dy.2.addr
  br label %while.cond.3
while.cond.3:
  %v19 = load i64, ptr %local.dy.2.addr
  %v20 = icmp slt i64 %v19, 8
  br i1 %v20, label %while.body.3, label %while.end.3
while.body.3:
  %v21 = load i64, ptr %local.dy.2.addr
  %v22 = add i64 %v18, %v21
  %v23.ptr = call ptr addrspace(2) @llvm.dx.resource.getpointer(target("dx.CBuffer", %__cblayout_lib_texture) %rx_cb, i32 8)
  %v23 = load i64, ptr addrspace(2) %v23.ptr
  %v24 = icmp slt i64 %v22, %v23
  br i1 %v24, label %if.then.2, label %if.end.2
if.then.2:
  store i64 0, ptr %local.dx.3.addr
  br label %while.cond.1
while.cond.1:
  %v25 = load i64, ptr %local.dx.3.addr
  %v26 = icmp slt i64 %v25, 8
  br i1 %v26, label %while.body.1, label %while.end.1
while.body.1:
  %v27 = load i64, ptr %local.dx.3.addr
  %v28 = add i64 %v17, %v27
  %v29.ptr = call ptr addrspace(2) @llvm.dx.resource.getpointer(target("dx.CBuffer", %__cblayout_lib_texture) %rx_cb, i32 0)
  %v29 = load i64, ptr addrspace(2) %v29.ptr
  %v30 = icmp slt i64 %v28, %v29
  br i1 %v30, label %if.then.0, label %if.end.0
if.then.0:
  %v31.x = trunc i64 %v28 to i32
  %v31.y = trunc i64 %v22 to i32
  %v31.c0 = insertelement <2 x i32> poison, i32 %v31.x, i32 0
  %v31.coords = insertelement <2 x i32> %v31.c0, i32 %v31.y, i32 1
  %v31 = call float @llvm.dx.resource.load.level(target("dx.Texture", float, 0, 0, 0, 2) %rx_h_src_luminance, <2 x i32> %v31.coords, i32 0, <2 x i32> zeroinitializer)
  %v32 = load float, ptr %local.accum.0.addr
  %v33 = fadd float %v32, %v31
  store float %v33, ptr %local.accum.0.addr
  %v34 = load float, ptr %local.count.1.addr
  %v35 = fadd float %v34, 1.0
  store float %v35, ptr %local.count.1.addr
  br label %if.end.0
if.end.0:
  %v36 = load i64, ptr %local.dx.3.addr
  %v37 = add i64 %v36, 1
  store i64 %v37, ptr %local.dx.3.addr
  br label %while.cond.1
while.end.1:
  br label %if.end.2
if.end.2:
  %v38 = load i64, ptr %local.dy.2.addr
  %v39 = add i64 %v38, 1
  store i64 %v39, ptr %local.dy.2.addr
  br label %while.cond.3
while.end.3:
  %v40 = load float, ptr %local.count.1.addr
  %v41 = fcmp ogt float %v40, 0.0
  %v42 = load float, ptr %local.accum.0.addr
  %v43 = load float, ptr %local.count.1.addr
  %v44 = fdiv float %v42, %v43
  %v45 = select i1 %v41, float %v44, float 0.0
  %v46.x = trunc i64 %v15 to i32
  %v46.y = trunc i64 %v16 to i32
  %v46.c0 = insertelement <2 x i32> poison, i32 %v46.x, i32 0
  %v46.coords = insertelement <2 x i32> %v46.c0, i32 %v46.y, i32 1
  call void @llvm.dx.resource.store.texture(target("dx.Texture", float, 1, 0, 0, 2) %rx_h_dst_luminance, <2 x i32> %v46.coords, float %v45)
  br label %if.end.4
if.end.4:
  ret void
}

attributes #0 = { noinline nounwind "hlsl.numthreads"="1,1,1" "hlsl.shader"="compute" }

!dx.valver = !{!0}

!0 = !{i32 1, i32 8}
