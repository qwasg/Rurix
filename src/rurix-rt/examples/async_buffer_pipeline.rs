//! 三 stream 流序分配端到端示例(G1.2,MR-0001 / RXS-0148):供 Compute Sanitizer nightly
//! 包裹(racecheck+memcheck,CUDA.jl #780 use-after-free 事故类永久回归项)+ 步骤 42 device
//! 佐证的稳定 exe 入口。`three_stream_async_pipeline`:流序分配(`cuMemAllocAsync`)+ 两条
//! `share_with` 跨 stream 时序边 + `cuMemFreeAsync` 流序释放 + 往返数值对照。
//!
//! 无 GPU / 老驱动无 `cuMemAllocAsync` → 打印 skip,return 0(降级,对齐 GPU 步骤降级先例)。
use rurix_rt::pipeline::three_stream_async_pipeline;

fn main() {
    let len = std::env::var("RURIX_ASYNC_BUFFER_LEN")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4096);
    match three_stream_async_pipeline(len) {
        Ok(true) => println!("ASYNC_BUFFER_RESULT: ok pipeline=1 len={len} roundtrip=match"),
        Ok(false) => {
            eprintln!("ASYNC_BUFFER_RESULT: fail roundtrip=mismatch");
            std::process::exit(1);
        }
        Err(e) => println!("ASYNC_BUFFER_RESULT: skip reason={e:?}"),
    }
}
