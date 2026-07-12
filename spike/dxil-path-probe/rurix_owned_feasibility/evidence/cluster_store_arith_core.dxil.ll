; ModuleID = 'cluster_store_arith_core'
source_filename = "cluster_store_arith_core"
target datalayout = "e-m:e-p:32:32-i1:32-i8:8-i16:16-i32:32-i64:64-f16:16-f32:32-f64:64-n8:16:32:64"
target triple = "dxil-unknown-shadermodel6.0-compute"

define void @rx_cluster_store_decode_pack_8() #0 {
entry:
  %rx_h_cluster_render = call target("dx.RawBuffer", i32, 0, 0) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %rx_h_cluster_out = call target("dx.RawBuffer", i32, 1, 0) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %local.bits.0.addr = alloca i32
  %local.popcnt.1.addr = alloca i32
  %v0.u32 = call i32 @llvm.dx.thread.id(i32 0)
  %v0 = zext i32 %v0.u32 to i64
  %v1.idx = trunc i64 %v0 to i32
  %v1.ld = call { i32, i1 } @llvm.dx.resource.load.rawbuffer(target("dx.RawBuffer", i32, 0, 0) %rx_h_cluster_render, i32 %v1.idx, i32 0)
  %v1 = extractvalue { i32, i1 } %v1.ld, 0
  store i32 %v1, ptr %local.bits.0.addr
  store i32 0, ptr %local.popcnt.1.addr
  br label %while.cond.0
while.cond.0:
  %v2 = load i32, ptr %local.bits.0.addr
  %v3 = icmp ne i32 %v2, 0
  br i1 %v3, label %while.body.0, label %while.end.0
while.body.0:
  %v4 = load i32, ptr %local.bits.0.addr
  %v5 = call i32 @llvm.dx.firstbitlow(i32 %v4)
  %v6 = load i32, ptr %local.popcnt.1.addr
  %v7 = add i32 %v6, 1
  store i32 %v7, ptr %local.popcnt.1.addr
  %v8 = load i32, ptr %local.bits.0.addr
  %v9.shamt = and i32 %v5, 31
  %v9 = shl i32 1, %v9.shamt
  %v10 = xor i32 %v8, %v9
  store i32 %v10, ptr %local.bits.0.addr
  br label %while.cond.0
while.end.0:
  %v11.shamt = and i32 8, 31
  %v11 = lshr i32 %v1, %v11.shamt
  %v12 = and i32 %v11, 255
  %v13 = icmp ne i32 %v12, 0
  br i1 %v13, label %if.then.1, label %if.end.1
if.then.1:
  %v14 = call i32 @llvm.dx.firstbitlow(i32 %v12)
  %v15.raw = call i32 @llvm.dx.firstbituhigh(i32 %v12)
  %v15.norm = sub i32 31, %v15.raw
  %v15.isz = icmp eq i32 %v15.raw, -1
  %v15 = select i1 %v15.isz, i32 -1, i32 %v15.norm
  %v16 = add i32 %v15, 1
  %v17 = load i32, ptr %local.popcnt.1.addr
  %v18.shamt = and i32 24, 31
  %v18 = shl i32 %v17, %v18.shamt
  %v19.shamt = and i32 8, 31
  %v19 = shl i32 %v14, %v19.shamt
  %v20 = or i32 %v18, %v19
  %v21 = or i32 %v20, %v16
  %v22.idx = trunc i64 %v0 to i32
  call void @llvm.dx.resource.store.rawbuffer(target("dx.RawBuffer", i32, 1, 0) %rx_h_cluster_out, i32 %v22.idx, i32 0, i32 %v21)
  br label %if.end.1
if.end.1:
  ret void
}

attributes #0 = { noinline nounwind "hlsl.numthreads"="1,1,1" "hlsl.shader"="compute" }

!dx.valver = !{!0}

!0 = !{i32 1, i32 8}
