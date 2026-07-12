; ModuleID = 'instance_compaction_scatter_lane'
source_filename = "instance_compaction_scatter_lane"
target datalayout = "e-m:e-p:32:32-i1:32-i8:8-i16:16-i32:32-i64:64-f16:16-f32:32-f64:64-n8:16:32:64"
target triple = "dxil-unknown-shadermodel6.0-compute"

%__cblayout_instance_compaction_scatter_lane = type <{ i64, i64 }>

define void @rx_instance_compaction_scatter_move_lane_8() #0 {
entry:
  %rx_cb = call target("dx.CBuffer", %__cblayout_instance_compaction_scatter_lane) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %rx_h_src_transforms = call target("dx.RawBuffer", float, 0, 0) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %rx_h_dst_transforms = call target("dx.RawBuffer", float, 1, 0) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %v0.u32 = call i32 @llvm.dx.thread.id(i32 0)
  %v0 = zext i32 %v0.u32 to i64
  %v1.ptr = call ptr addrspace(2) @llvm.dx.resource.getpointer(target("dx.CBuffer", %__cblayout_instance_compaction_scatter_lane) %rx_cb, i32 0)
  %v1 = load i64, ptr addrspace(2) %v1.ptr
  %v2 = icmp slt i64 %v0, %v1
  br i1 %v2, label %if.then.0, label %if.end.0
if.then.0:
  %v3 = mul i64 %v0, 12
  %v4.ptr = call ptr addrspace(2) @llvm.dx.resource.getpointer(target("dx.CBuffer", %__cblayout_instance_compaction_scatter_lane) %rx_cb, i32 8)
  %v4 = load i64, ptr addrspace(2) %v4.ptr
  %v5 = add i64 %v4, 0
  %v6 = add i64 %v3, 0
  %v7.idx = trunc i64 %v6 to i32
  %v7.ld = call { float, i1 } @llvm.dx.resource.load.rawbuffer(target("dx.RawBuffer", float, 0, 0) %rx_h_src_transforms, i32 %v7.idx, i32 0)
  %v7 = extractvalue { float, i1 } %v7.ld, 0
  %v8.idx = trunc i64 %v5 to i32
  call void @llvm.dx.resource.store.rawbuffer(target("dx.RawBuffer", float, 1, 0) %rx_h_dst_transforms, i32 %v8.idx, i32 0, float %v7)
  %v9 = add i64 %v4, 1
  %v10 = add i64 %v3, 1
  %v11.idx = trunc i64 %v10 to i32
  %v11.ld = call { float, i1 } @llvm.dx.resource.load.rawbuffer(target("dx.RawBuffer", float, 0, 0) %rx_h_src_transforms, i32 %v11.idx, i32 0)
  %v11 = extractvalue { float, i1 } %v11.ld, 0
  %v12.idx = trunc i64 %v9 to i32
  call void @llvm.dx.resource.store.rawbuffer(target("dx.RawBuffer", float, 1, 0) %rx_h_dst_transforms, i32 %v12.idx, i32 0, float %v11)
  %v13 = add i64 %v4, 2
  %v14 = add i64 %v3, 2
  %v15.idx = trunc i64 %v14 to i32
  %v15.ld = call { float, i1 } @llvm.dx.resource.load.rawbuffer(target("dx.RawBuffer", float, 0, 0) %rx_h_src_transforms, i32 %v15.idx, i32 0)
  %v15 = extractvalue { float, i1 } %v15.ld, 0
  %v16.idx = trunc i64 %v13 to i32
  call void @llvm.dx.resource.store.rawbuffer(target("dx.RawBuffer", float, 1, 0) %rx_h_dst_transforms, i32 %v16.idx, i32 0, float %v15)
  %v17 = add i64 %v4, 3
  %v18 = add i64 %v3, 3
  %v19.idx = trunc i64 %v18 to i32
  %v19.ld = call { float, i1 } @llvm.dx.resource.load.rawbuffer(target("dx.RawBuffer", float, 0, 0) %rx_h_src_transforms, i32 %v19.idx, i32 0)
  %v19 = extractvalue { float, i1 } %v19.ld, 0
  %v20.idx = trunc i64 %v17 to i32
  call void @llvm.dx.resource.store.rawbuffer(target("dx.RawBuffer", float, 1, 0) %rx_h_dst_transforms, i32 %v20.idx, i32 0, float %v19)
  %v21 = add i64 %v4, 4
  %v22 = add i64 %v3, 4
  %v23.idx = trunc i64 %v22 to i32
  %v23.ld = call { float, i1 } @llvm.dx.resource.load.rawbuffer(target("dx.RawBuffer", float, 0, 0) %rx_h_src_transforms, i32 %v23.idx, i32 0)
  %v23 = extractvalue { float, i1 } %v23.ld, 0
  %v24.idx = trunc i64 %v21 to i32
  call void @llvm.dx.resource.store.rawbuffer(target("dx.RawBuffer", float, 1, 0) %rx_h_dst_transforms, i32 %v24.idx, i32 0, float %v23)
  %v25 = add i64 %v4, 5
  %v26 = add i64 %v3, 5
  %v27.idx = trunc i64 %v26 to i32
  %v27.ld = call { float, i1 } @llvm.dx.resource.load.rawbuffer(target("dx.RawBuffer", float, 0, 0) %rx_h_src_transforms, i32 %v27.idx, i32 0)
  %v27 = extractvalue { float, i1 } %v27.ld, 0
  %v28.idx = trunc i64 %v25 to i32
  call void @llvm.dx.resource.store.rawbuffer(target("dx.RawBuffer", float, 1, 0) %rx_h_dst_transforms, i32 %v28.idx, i32 0, float %v27)
  %v29 = add i64 %v4, 6
  %v30 = add i64 %v3, 6
  %v31.idx = trunc i64 %v30 to i32
  %v31.ld = call { float, i1 } @llvm.dx.resource.load.rawbuffer(target("dx.RawBuffer", float, 0, 0) %rx_h_src_transforms, i32 %v31.idx, i32 0)
  %v31 = extractvalue { float, i1 } %v31.ld, 0
  %v32.idx = trunc i64 %v29 to i32
  call void @llvm.dx.resource.store.rawbuffer(target("dx.RawBuffer", float, 1, 0) %rx_h_dst_transforms, i32 %v32.idx, i32 0, float %v31)
  %v33 = add i64 %v4, 7
  %v34 = add i64 %v3, 7
  %v35.idx = trunc i64 %v34 to i32
  %v35.ld = call { float, i1 } @llvm.dx.resource.load.rawbuffer(target("dx.RawBuffer", float, 0, 0) %rx_h_src_transforms, i32 %v35.idx, i32 0)
  %v35 = extractvalue { float, i1 } %v35.ld, 0
  %v36.idx = trunc i64 %v33 to i32
  call void @llvm.dx.resource.store.rawbuffer(target("dx.RawBuffer", float, 1, 0) %rx_h_dst_transforms, i32 %v36.idx, i32 0, float %v35)
  %v37 = add i64 %v4, 8
  %v38 = add i64 %v3, 8
  %v39.idx = trunc i64 %v38 to i32
  %v39.ld = call { float, i1 } @llvm.dx.resource.load.rawbuffer(target("dx.RawBuffer", float, 0, 0) %rx_h_src_transforms, i32 %v39.idx, i32 0)
  %v39 = extractvalue { float, i1 } %v39.ld, 0
  %v40.idx = trunc i64 %v37 to i32
  call void @llvm.dx.resource.store.rawbuffer(target("dx.RawBuffer", float, 1, 0) %rx_h_dst_transforms, i32 %v40.idx, i32 0, float %v39)
  %v41 = add i64 %v4, 9
  %v42 = add i64 %v3, 9
  %v43.idx = trunc i64 %v42 to i32
  %v43.ld = call { float, i1 } @llvm.dx.resource.load.rawbuffer(target("dx.RawBuffer", float, 0, 0) %rx_h_src_transforms, i32 %v43.idx, i32 0)
  %v43 = extractvalue { float, i1 } %v43.ld, 0
  %v44.idx = trunc i64 %v41 to i32
  call void @llvm.dx.resource.store.rawbuffer(target("dx.RawBuffer", float, 1, 0) %rx_h_dst_transforms, i32 %v44.idx, i32 0, float %v43)
  %v45 = add i64 %v4, 10
  %v46 = add i64 %v3, 10
  %v47.idx = trunc i64 %v46 to i32
  %v47.ld = call { float, i1 } @llvm.dx.resource.load.rawbuffer(target("dx.RawBuffer", float, 0, 0) %rx_h_src_transforms, i32 %v47.idx, i32 0)
  %v47 = extractvalue { float, i1 } %v47.ld, 0
  %v48.idx = trunc i64 %v45 to i32
  call void @llvm.dx.resource.store.rawbuffer(target("dx.RawBuffer", float, 1, 0) %rx_h_dst_transforms, i32 %v48.idx, i32 0, float %v47)
  %v49 = add i64 %v4, 11
  %v50 = add i64 %v3, 11
  %v51.idx = trunc i64 %v50 to i32
  %v51.ld = call { float, i1 } @llvm.dx.resource.load.rawbuffer(target("dx.RawBuffer", float, 0, 0) %rx_h_src_transforms, i32 %v51.idx, i32 0)
  %v51 = extractvalue { float, i1 } %v51.ld, 0
  %v52.idx = trunc i64 %v49 to i32
  call void @llvm.dx.resource.store.rawbuffer(target("dx.RawBuffer", float, 1, 0) %rx_h_dst_transforms, i32 %v52.idx, i32 0, float %v51)
  br label %if.end.0
if.end.0:
  ret void
}

attributes #0 = { noinline nounwind "hlsl.numthreads"="1,1,1" "hlsl.shader"="compute" }

!dx.valver = !{!0}

!0 = !{i32 1, i32 8}
