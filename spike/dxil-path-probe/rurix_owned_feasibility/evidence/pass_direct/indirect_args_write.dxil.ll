; ModuleID = 'indirect_args_write_kernel'
source_filename = "indirect_args_write_kernel"
target datalayout = "e-m:e-p:32:32-i1:32-i8:8-i16:16-i32:32-i64:64-f16:16-f32:32-f64:64-n8:16:32:64"
target triple = "dxil-unknown-shadermodel6.0-compute"

%__cblayout_indirect_args_write_kernel = type <{ i64, i64 }>

define void @rx_indirect_args_write_8() #0 {
entry:
  %rx_cb = call target("dx.CBuffer", %__cblayout_indirect_args_write_kernel) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %rx_h_src_survivor_counts = call target("dx.RawBuffer", i32, 0, 0) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %rx_h_surface_template = call target("dx.RawBuffer", i32, 0, 0) @llvm.dx.resource.handlefrombinding(i32 0, i32 1, i32 1, i32 0, ptr null)
  %rx_h_caps = call target("dx.RawBuffer", i32, 0, 0) @llvm.dx.resource.handlefrombinding(i32 0, i32 2, i32 1, i32 0, ptr null)
  %rx_h_dst_command_buffer = call target("dx.RawBuffer", i32, 1, 0) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %v0.u32 = call i32 @llvm.dx.thread.id(i32 0)
  %v0 = zext i32 %v0.u32 to i64
  %v1.ptr = call ptr addrspace(2) @llvm.dx.resource.getpointer(target("dx.CBuffer", %__cblayout_indirect_args_write_kernel) %rx_cb, i32 0)
  %v1 = load i64, ptr addrspace(2) %v1.ptr
  %v2 = icmp slt i64 %v0, %v1
  br i1 %v2, label %if.then.0, label %if.end.0
if.then.0:
  %v3.ptr = call ptr addrspace(2) @llvm.dx.resource.getpointer(target("dx.CBuffer", %__cblayout_indirect_args_write_kernel) %rx_cb, i32 8)
  %v3 = load i64, ptr addrspace(2) %v3.ptr
  %v4.idx = trunc i64 %v3 to i32
  %v4.ld = call { i32, i1 } @llvm.dx.resource.load.rawbuffer(target("dx.RawBuffer", i32, 0, 0) %rx_h_src_survivor_counts, i32 %v4.idx, i32 0)
  %v4 = extractvalue { i32, i1 } %v4.ld, 0
  %v5.ld = call { i32, i1 } @llvm.dx.resource.load.rawbuffer(target("dx.RawBuffer", i32, 0, 0) %rx_h_caps, i32 0, i32 0)
  %v5 = extractvalue { i32, i1 } %v5.ld, 0
  %v6 = icmp ult i32 %v4, %v5
  %v7 = select i1 %v6, i32 %v4, i32 %v5
  %v8 = mul i64 %v0, 5
  %v9 = mul i64 %v0, 5
  %v10 = add i64 %v8, 0
  %v11 = add i64 %v9, 0
  %v12.idx = trunc i64 %v11 to i32
  %v12.ld = call { i32, i1 } @llvm.dx.resource.load.rawbuffer(target("dx.RawBuffer", i32, 0, 0) %rx_h_surface_template, i32 %v12.idx, i32 0)
  %v12 = extractvalue { i32, i1 } %v12.ld, 0
  %v13.idx = trunc i64 %v10 to i32
  call void @llvm.dx.resource.store.rawbuffer(target("dx.RawBuffer", i32, 1, 0) %rx_h_dst_command_buffer, i32 %v13.idx, i32 0, i32 %v12)
  %v14 = add i64 %v8, 1
  %v15.idx = trunc i64 %v14 to i32
  call void @llvm.dx.resource.store.rawbuffer(target("dx.RawBuffer", i32, 1, 0) %rx_h_dst_command_buffer, i32 %v15.idx, i32 0, i32 %v7)
  %v16 = add i64 %v8, 2
  %v17 = add i64 %v9, 2
  %v18.idx = trunc i64 %v17 to i32
  %v18.ld = call { i32, i1 } @llvm.dx.resource.load.rawbuffer(target("dx.RawBuffer", i32, 0, 0) %rx_h_surface_template, i32 %v18.idx, i32 0)
  %v18 = extractvalue { i32, i1 } %v18.ld, 0
  %v19.idx = trunc i64 %v16 to i32
  call void @llvm.dx.resource.store.rawbuffer(target("dx.RawBuffer", i32, 1, 0) %rx_h_dst_command_buffer, i32 %v19.idx, i32 0, i32 %v18)
  %v20 = add i64 %v8, 3
  %v21 = add i64 %v9, 3
  %v22.idx = trunc i64 %v21 to i32
  %v22.ld = call { i32, i1 } @llvm.dx.resource.load.rawbuffer(target("dx.RawBuffer", i32, 0, 0) %rx_h_surface_template, i32 %v22.idx, i32 0)
  %v22 = extractvalue { i32, i1 } %v22.ld, 0
  %v23.idx = trunc i64 %v20 to i32
  call void @llvm.dx.resource.store.rawbuffer(target("dx.RawBuffer", i32, 1, 0) %rx_h_dst_command_buffer, i32 %v23.idx, i32 0, i32 %v22)
  %v24 = add i64 %v8, 4
  %v25 = add i64 %v9, 4
  %v26.idx = trunc i64 %v25 to i32
  %v26.ld = call { i32, i1 } @llvm.dx.resource.load.rawbuffer(target("dx.RawBuffer", i32, 0, 0) %rx_h_surface_template, i32 %v26.idx, i32 0)
  %v26 = extractvalue { i32, i1 } %v26.ld, 0
  %v27.idx = trunc i64 %v24 to i32
  call void @llvm.dx.resource.store.rawbuffer(target("dx.RawBuffer", i32, 1, 0) %rx_h_dst_command_buffer, i32 %v27.idx, i32 0, i32 %v26)
  br label %if.end.0
if.end.0:
  ret void
}

attributes #0 = { noinline nounwind "hlsl.numthreads"="1,1,1" "hlsl.shader"="compute" }

!dx.valver = !{!0}

!0 = !{i32 1, i32 8}
