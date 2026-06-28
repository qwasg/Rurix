//! `emit_binding_rts0` — G2.3 PR-E2b-4 device 真跑支撑工具(非生产 codegen、非
//! emit 逻辑改动)。把 G2.3 绑定布局推导产物的 **RTS0 root signature 容器字节**
//! (生产可达资源子集 `Texture2D<f32>` + `Sampler`,与 `tests/dxil/binding/
//! fs_tex_samp.binding-golden` 同源)落盘,供 `ci/dxil_binding_device_smoke.py`
//! 喂给真实 D3D12 `CreateRootSignature` 做 device 核验(G-G2-3 收口)。
//!
//! 纯 host:仅调用既有公开 API [`rurixc::binding_layout::infer_root_signature`] +
//! [`rurixc::binding_layout::serialize_rts0`](不改推导/序列化逻辑);确定性字节
//! 由 device smoke 以 SHA-256 与 blessed golden 基线交叉核对,证明 device 消费的
//! 正是已 bless 的 RTS0 产物。
//!
//! 用法:`cargo run -p rurixc --features dxil-backend --example emit_binding_rts0 -- <out.bin>`

#[cfg(feature = "dxil-backend")]
fn main() -> std::process::ExitCode {
    use rurixc::binding_layout::{infer_root_signature, serialize_rts0};
    use rurixc::hir::PrimTy;
    use rurixc::mir::{MirResourceType, ResourceBinding, ResourceCount};

    let out = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("用法: emit_binding_rts0 <out.bin>");
            return std::process::ExitCode::from(2);
        }
    };

    // 生产可达资源子集(与 fs_tex_samp.binding-golden 同源:声明序 tex(SRV) → samp(Sampler))。
    let resources = vec![
        ResourceBinding {
            name: "tex".to_owned(),
            res: MirResourceType::Texture2D(PrimTy::F32),
            count: ResourceCount::One,
        },
        ResourceBinding {
            name: "samp".to_owned(),
            res: MirResourceType::Sampler,
            count: ResourceCount::One,
        },
    ];

    let rs = match infer_root_signature(&resources) {
        Ok(rs) => rs,
        Err(e) => {
            eprintln!("infer_root_signature 失败: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let rts0 = serialize_rts0(&rs);
    if let Err(e) = std::fs::write(&out, &rts0) {
        eprintln!("写 {out} 失败: {e}");
        return std::process::ExitCode::FAILURE;
    }
    println!("emit_binding_rts0: wrote {} bytes to {out}", rts0.len());
    std::process::ExitCode::SUCCESS
}

#[cfg(not(feature = "dxil-backend"))]
fn main() {
    eprintln!("emit_binding_rts0 需 --features dxil-backend");
}
