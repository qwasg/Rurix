; ModuleID = 'lib'
source_filename = "lib"
target datalayout = "e-m:e-p:32:32-i1:32-i8:8-i16:16-i32:32-i64:64-f16:16-f32:32-f64:64-n8:16:32:64"
target triple = "dxil-unknown-shadermodel6.0-compute"

%__cblayout_lib = type <{ i64, i64, float, float, float }>

define void @rx_luminance_reduce_level_8() #0 {
entry:
  %rx_cb = call target("dx.CBuffer", %__cblayout_lib) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %rx_h_src_luminance = call target("dx.RawBuffer", float, 0, 0) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %rx_h_dst_luminance = call target("dx.RawBuffer", float, 1, 0) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %local.accum.0.addr = alloca float
  %local.count.1.addr = alloca float
  %local.dy.2.addr = alloca i64
  %local.dx.3.addr = alloca i64
  %v0.ptr = call ptr addrspace(2) @llvm.dx.resource.getpointer(target("dx.CBuffer", %__cblayout_lib) %rx_cb, i32 0)
  %v0 = load i64, ptr addrspace(2) %v0.ptr
  %v1 = icmp sgt i64 %v0, 1
  %v2.ptr = call ptr addrspace(2) @llvm.dx.resource.getpointer(target("dx.CBuffer", %__cblayout_lib) %rx_cb, i32 0)
  %v2 = load i64, ptr addrspace(2) %v2.ptr
  %v3 = add i64 %v2, 7
  %v4 = sdiv i64 %v3, 8
  %v5 = select i1 %v1, i64 %v4, i64 1
  %v6.ptr = call ptr addrspace(2) @llvm.dx.resource.getpointer(target("dx.CBuffer", %__cblayout_lib) %rx_cb, i32 8)
  %v6 = load i64, ptr addrspace(2) %v6.ptr
  %v7 = icmp sgt i64 %v6, 1
  %v8.ptr = call ptr addrspace(2) @llvm.dx.resource.getpointer(target("dx.CBuffer", %__cblayout_lib) %rx_cb, i32 8)
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
  %v23.ptr = call ptr addrspace(2) @llvm.dx.resource.getpointer(target("dx.CBuffer", %__cblayout_lib) %rx_cb, i32 8)
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
  %v29.ptr = call ptr addrspace(2) @llvm.dx.resource.getpointer(target("dx.CBuffer", %__cblayout_lib) %rx_cb, i32 0)
  %v29 = load i64, ptr addrspace(2) %v29.ptr
  %v30 = icmp slt i64 %v28, %v29
  br i1 %v30, label %if.then.0, label %if.end.0
if.then.0:
  %v31.ptr = call ptr addrspace(2) @llvm.dx.resource.getpointer(target("dx.CBuffer", %__cblayout_lib) %rx_cb, i32 0)
  %v31 = load i64, ptr addrspace(2) %v31.ptr
  %v32 = mul i64 %v22, %v31
  %v33 = add i64 %v32, %v28
  %v34 = load float, ptr %local.accum.0.addr
  %v35.idx = trunc i64 %v33 to i32
  %v35.ld = call { float, i1 } @llvm.dx.resource.load.rawbuffer(target("dx.RawBuffer", float, 0, 0) %rx_h_src_luminance, i32 %v35.idx, i32 0)
  %v35 = extractvalue { float, i1 } %v35.ld, 0
  %v36 = fadd float %v34, %v35
  store float %v36, ptr %local.accum.0.addr
  %v37 = load float, ptr %local.count.1.addr
  %v38 = fadd float %v37, 1.0
  store float %v38, ptr %local.count.1.addr
  br label %if.end.0
if.end.0:
  %v39 = load i64, ptr %local.dx.3.addr
  %v40 = add i64 %v39, 1
  store i64 %v40, ptr %local.dx.3.addr
  br label %while.cond.1
while.end.1:
  br label %if.end.2
if.end.2:
  %v41 = load i64, ptr %local.dy.2.addr
  %v42 = add i64 %v41, 1
  store i64 %v42, ptr %local.dy.2.addr
  br label %while.cond.3
while.end.3:
  %v43 = load float, ptr %local.count.1.addr
  %v44 = fcmp ogt float %v43, 0.0
  %v45 = load float, ptr %local.accum.0.addr
  %v46 = load float, ptr %local.count.1.addr
  %v47 = fdiv float %v45, %v46
  %v48 = select i1 %v44, float %v47, float 0.0
  %v49.ptr = call ptr addrspace(2) @llvm.dx.resource.getpointer(target("dx.CBuffer", %__cblayout_lib) %rx_cb, i32 20)
  %v49 = load float, ptr addrspace(2) %v49.ptr
  %v50 = fcmp olt float %v48, %v49
  %v51.ptr = call ptr addrspace(2) @llvm.dx.resource.getpointer(target("dx.CBuffer", %__cblayout_lib) %rx_cb, i32 20)
  %v51 = load float, ptr addrspace(2) %v51.ptr
  %v52 = select i1 %v50, float %v51, float %v48
  %v53.ptr = call ptr addrspace(2) @llvm.dx.resource.getpointer(target("dx.CBuffer", %__cblayout_lib) %rx_cb, i32 16)
  %v53 = load float, ptr addrspace(2) %v53.ptr
  %v54 = fcmp ogt float %v52, %v53
  %v55.ptr = call ptr addrspace(2) @llvm.dx.resource.getpointer(target("dx.CBuffer", %__cblayout_lib) %rx_cb, i32 16)
  %v55 = load float, ptr addrspace(2) %v55.ptr
  %v56 = select i1 %v54, float %v55, float %v52
  %v57.ptr = call ptr addrspace(2) @llvm.dx.resource.getpointer(target("dx.CBuffer", %__cblayout_lib) %rx_cb, i32 24)
  %v57 = load float, ptr addrspace(2) %v57.ptr
  %v58 = fmul float %v56, %v57
  %v59.idx = trunc i64 %v13 to i32
  call void @llvm.dx.resource.store.rawbuffer(target("dx.RawBuffer", float, 1, 0) %rx_h_dst_luminance, i32 %v59.idx, i32 0, float %v58)
  br label %if.end.4
if.end.4:
  ret void
}

attributes #0 = { noinline nounwind "hlsl.numthreads"="1,1,1" "hlsl.shader"="compute" }

!dx.valver = !{!0}

!0 = !{i32 1, i32 8}
