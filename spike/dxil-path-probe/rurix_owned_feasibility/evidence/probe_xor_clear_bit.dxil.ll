; ModuleID = 'probe_xor_clear_bit'
source_filename = "probe_xor_clear_bit"
target datalayout = "e-m:e-p:32:32-i1:32-i8:8-i16:16-i32:32-i64:64-f16:16-f32:32-f64:64-n8:16:32:64"
target triple = "dxil-unknown-shadermodel6.0-compute"

define void @rx_k_8() #0 {
entry:
  %rx_h_src = call target("dx.RawBuffer", i32, 0, 0) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %rx_h_dst = call target("dx.RawBuffer", i32, 1, 0) @llvm.dx.resource.handlefrombinding(i32 0, i32 0, i32 1, i32 0, ptr null)
  %local.bits.0.addr = alloca i32
  %local.count.1.addr = alloca i32
  %v0.u32 = call i32 @llvm.dx.thread.id(i32 0)
  %v0 = zext i32 %v0.u32 to i64
  %v1.idx = trunc i64 %v0 to i32
  %v1.ld = call { i32, i1 } @llvm.dx.resource.load.rawbuffer(target("dx.RawBuffer", i32, 0, 0) %rx_h_src, i32 %v1.idx, i32 0)
  %v1 = extractvalue { i32, i1 } %v1.ld, 0
  store i32 %v1, ptr %local.bits.0.addr
  store i32 0, ptr %local.count.1.addr
  br label %while.cond.0
while.cond.0:
  %v2 = load i32, ptr %local.bits.0.addr
  %v3 = icmp ne i32 %v2, 0
  br i1 %v3, label %while.body.0, label %while.end.0
while.body.0:
  %v4 = load i32, ptr %local.bits.0.addr
  %v5 = call i32 @llvm.dx.firstbitlow(i32 %v4)
  %v6 = load i32, ptr %local.count.1.addr
  %v7 = add i32 %v6, 1
  store i32 %v7, ptr %local.count.1.addr
  %v8 = load i32, ptr %local.bits.0.addr
  %v9.shamt = and i32 %v5, 31
  %v9 = shl i32 1, %v9.shamt
  %v10 = xor i32 %v8, %v9
  store i32 %v10, ptr %local.bits.0.addr
  br label %while.cond.0
while.end.0:
  %v11 = load i32, ptr %local.count.1.addr
  %v12.idx = trunc i64 %v0 to i32
  call void @llvm.dx.resource.store.rawbuffer(target("dx.RawBuffer", i32, 1, 0) %rx_h_dst, i32 %v12.idx, i32 0, i32 %v11)
  ret void
}

attributes #0 = { noinline nounwind "hlsl.numthreads"="1,1,1" "hlsl.shader"="compute" }

!dx.valver = !{!0}

!0 = !{i32 1, i32 8}
