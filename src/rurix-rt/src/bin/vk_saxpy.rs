//! mb1 Vulkan compute 真跑 demo(RXS-0207;RFC-0011 §4.6)。
//!
//! 用法:`vk_saxpy <saxpy.spv>`(Phase 1 `rurixc --target vulkan saxpy.rx -o saxpy.spv` 产)。
//! 在本机 Vulkan 设备(NVIDIA / AMD 桌面 / lavapipe)真跑 saxpy = a*x + out,回读校验数值。
//! entry 名自 SPIR-V `OpEntryPoint` 解析(codegen mangled 符号名)。push constant 布局
//! {a: f32 @0, n: u32 @4};buffer binding0=out / binding1=x;dispatch [n,1,1](LocalSize 1)。

fn to_bytes(v: &[f32]) -> Vec<u8> {
    let mut b = Vec::with_capacity(v.len() * 4);
    for f in v {
        b.extend_from_slice(&f.to_le_bytes());
    }
    b
}

fn to_f32(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: vk_saxpy <saxpy.spv>");
        std::process::exit(2);
    });
    let raw = std::fs::read(&path).unwrap_or_else(|e| {
        eprintln!("读 {path} 失败: {e}");
        std::process::exit(2);
    });
    let spv: Vec<u32> = raw
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    let entry = rurix_rt::vk::entry_point_name(&spv).unwrap_or_else(|| {
        eprintln!("SPIR-V 无 OpEntryPoint");
        std::process::exit(1);
    });

    let n: u32 = 1024;
    let a: f32 = 2.0;
    let x: Vec<f32> = (0..n).map(|i| i as f32).collect();
    let out0: Vec<f32> = (0..n).map(|i| (i as f32) * 0.5).collect();

    // buffer binding0=out(in/out),binding1=x。
    let mut buffers = vec![to_bytes(&out0), to_bytes(&x)];
    // push constant:a(f32 @0) + n(u32 @4)。
    let mut pc = Vec::new();
    pc.extend_from_slice(&a.to_le_bytes());
    pc.extend_from_slice(&n.to_le_bytes());

    // 经 ComputeBackend 抽象(RXS-0206)选定 Vulkan 后端跑 —— 证明 trait 端到端真跑,
    // 非直调 vk::run_compute。artifact = 原始 SPIR-V 字节(backend 内转 u32)。
    use rurix_rt::backend::{BackendKind, ComputeJob, run_job};
    let mut job = ComputeJob {
        artifact: &raw,
        entry: &entry,
        buffers: &mut buffers,
        scalars: &pc,
        groups: [n, 1, 1],
        block: [1, 1, 1],
    };
    match run_job(BackendKind::Vulkan, &mut job) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("VK_SAXPY: backend dispatch 失败: {e}");
            std::process::exit(1);
        }
    }
    let _ = &spv; // spv(u32)保留供 entry 解析;dispatch 经 backend 走 raw 字节

    let out = to_f32(&buffers[0]);
    let mut max_err = 0.0f32;
    for i in 0..n as usize {
        let expected = a * x[i] + out0[i];
        max_err = max_err.max((out[i] - expected).abs());
    }
    if max_err > 1e-3 {
        eprintln!("VK_SAXPY: FAIL 数值不符 max_err={max_err}");
        std::process::exit(1);
    }
    println!(
        "VK_SAXPY: ok entry={entry} n={n} a={a} out[0]={} out[1]={} out[1023]={} max_err={max_err:.2e}",
        out[0], out[1], out[1023]
    );
}
