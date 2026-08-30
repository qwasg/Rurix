// Assisted-by: Claude（G37 W2 visbuffer）
//! G37 W2 visbuffer：#74/#111 VisBuffer + classify/resolve 窗口生产证据臂的
//! **独立接线 harness**（g31_window_present 合入前的编译/device 冒烟面;
//! 臂实现 = `g14_3_lane/g31_visbuffer_arm.rs` 单源,窗口合入提案消费同一文件,
//! 禁旁路复刻——本 bin 只做装配/样本/编排,机制链零重写）。
//!
//! 链：生产契约装配 bistro（prelude digest 门 + assemble_scene 装配语义单源）
//! → RXCP 簇包读取 + `verify_cluster_pack` fail-closed（gltf sha/覆盖恰一次/
//! 叶几何位级）→ 装配相机 + 前向 dolly 样本梯（k×0.15m,相机驱动 cut 的
//! 存在性面;真窗口逐帧轨迹消费归窗口臂合入后）→ `run_visbuffer_arm`
//! （cut→32px SW/HW 分箱→SW compute 软光栅 device 真跑〔M95 u64 原子腿〕→
//! 覆盖对拍 + 双跑位级→合并→classify/resolve 材质分箱）→ sidecar JSON。
//!
//! 三态：无 Vulkan loader / bistro 资产缺失 → `skipped_dev_env` 退 0（不冒充
//! PASS;`RURIX_REQUIRE_REAL=1` 翻硬 FAIL）。
//!
//! 用法：
//!   g31_visbuffer_wiring --cluster-pack <bistro.rxcp> [--contract <json>]
//!     [--gltf <path>] [--tier 100] [--error-px 2.0] [--samples 3]
//!     [--res 96x54] [--evidence <sidecar.json>] [--expect-digest <sha256:…>]
#![forbid(unsafe_code)]
// 共享体含本 bin 未消费面（bench/render 腿、vendor 双臂、EXR 出图等）——
// dead_code 豁免如实登记;本 bin 消费面 = 契约装配/簇包读取/visbuffer 臂。
#![allow(dead_code)]

include!("g14_3_lane/g14_3_lane_body.rs");
include!("g14_3_lane/g31_visbuffer_arm.rs");

const VTAG: &str = "[g31_visbuffer_wiring]";

fn vskip(reason: &str) {
    println!("{VTAG}: {{\"state\":\"skipped_dev_env\",\"reason\":{}}}", jstr(reason));
    if require_real() {
        eprintln!("{VTAG}: FAIL RURIX_REQUIRE_REAL=1 但 device 面降级");
        std::process::exit(1);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut contract_path = DEFAULT_CONTRACT.to_owned();
    let mut gltf_path = String::new();
    let mut pack_path = String::new();
    let mut tier: u32 = 100;
    let mut error_px: f32 = 2.0;
    let mut samples: u32 = 3;
    let mut res = String::from("96x54");
    let mut evidence = String::new();
    let mut expect_digest: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--contract" => contract_path = take_arg(&args, &mut i),
            "--gltf" => gltf_path = take_arg(&args, &mut i),
            "--cluster-pack" => pack_path = take_arg(&args, &mut i),
            "--tier" => {
                tier = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--tier 非 u32"))
            }
            "--error-px" => {
                error_px = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--error-px 非 f32"))
            }
            "--samples" => {
                samples = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--samples 非 u32"))
            }
            "--res" => res = take_arg(&args, &mut i),
            "--evidence" => evidence = take_arg(&args, &mut i),
            "--expect-digest" => expect_digest = Some(take_arg(&args, &mut i)),
            other => fail(&format!("未知参数 {other}")),
        }
        i += 1;
    }
    if pack_path.is_empty() {
        fail("--cluster-pack <RXCP> 必填（g31_cluster_lod_bake 产物）");
    }
    if !(error_px.is_finite() && error_px > 0.0) {
        fail("--error-px 必须为正有限 f32");
    }
    if samples == 0 {
        fail("--samples 必须 ≥1");
    }
    let (vw, vh) = {
        let mut it = res.split('x');
        let w: u32 = it
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| fail("--res 形如 96x54"));
        let h: u32 = it
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| fail("--res 形如 96x54"));
        if it.next().is_some() || w == 0 || h == 0 {
            fail("--res 形如 96x54（两正整数）");
        }
        (w, h)
    };

    // 三态：无 Vulkan → skipped_dev_env（臂前置;不冒充 PASS）。
    if !vk::vulkan_available() {
        vskip("vulkan loader 不可用");
        return;
    }
    if !Path::new(&pack_path).is_file() {
        vskip(&format!("RXCP 簇包缺失 {pack_path}"));
        return;
    }

    // 契约装配（prelude digest 门 + 装配语义单源;资产缺失 = dev_env 降级）。
    let scene_id = "bistro-interior";
    let (pre, _) = prelude(scene_id, tier, 1, false, &contract_path, expect_digest.as_deref());
    if gltf_path.is_empty() {
        gltf_path = default_gltf(scene_id).to_owned();
    }
    if !Path::new(&gltf_path).is_file() {
        vskip(&format!("bistro gltf 缺失 {gltf_path}"));
        return;
    }
    let scene = match assemble_scene(&pre.contract.raw, scene_id, Path::new(&gltf_path)) {
        Ok(s) => s,
        Err(e) => {
            vskip(&format!("场景装配: {e}"));
            return;
        }
    };
    eprintln!(
        "{VTAG}: 装配就绪 tris={} in={}x{} tier={tier}（LOD 判据分辨率 = 内部分辨率,窗口臂同口径）",
        scene.tri_count, pre.in_w, pre.in_h,
    );

    // 簇包读取 + fail-closed 校验（--cluster-lod 同一冻结校验面直调）。
    let pack = read_cluster_pack(Path::new(&pack_path))
        .unwrap_or_else(|e| fail(&format!("RXCP 读取: {e}")));
    verify_cluster_pack(&pack, &scene)
        .unwrap_or_else(|e| fail(&format!("簇包校验 fail-closed: {e}")));
    eprintln!(
        "{VTAG}: 簇包就绪 blocks={} passthrough={}（gltf sha/覆盖恰一次/叶几何位级 已核）",
        pack.blocks.len(),
        pack.passthrough.len(),
    );

    // 相机样本梯：装配相机 + 前向 XZ dolly k×0.15m（相机驱动 cut 的存在性面;
    // 真窗口逐帧轨迹 = 窗口臂合入后消费,本 bin 如实登记为合成样本梯）。
    let cam0 = scene.camera;
    let fxz = (cam0.forward[0] * cam0.forward[0] + cam0.forward[2] * cam0.forward[2]).sqrt();
    let step = if fxz > 1e-6 {
        [cam0.forward[0] / fxz * 0.15, 0.0, cam0.forward[2] / fxz * 0.15]
    } else {
        [0.0; 3]
    };
    let cam_samples: Vec<VisBufferCamSample> = (0..samples)
        .map(|k| {
            let mut spec = cam0;
            spec.eye = [
                cam0.eye[0] + step[0] * k as f32,
                cam0.eye[1],
                cam0.eye[2] + step[2] * k as f32,
            ];
            VisBufferCamSample {
                frame: k,
                spec,
                in_w: pre.in_w,
                in_h: pre.in_h,
            }
        })
        .collect();

    let opt = VisBufferArmOpt {
        enabled: true,
        res_w: vw,
        res_h: vh,
        samples,
        out_path: evidence,
    };
    let stats = run_visbuffer_arm(VTAG, &pack, &opt, error_px, &cam_samples);
    visbuffer_finish(VTAG, &pack, &opt, error_px, &stats);
    // dolly 样本梯下 cut 变化的存在性登记（measured;单样本时不适用）。
    if stats.len() >= 2 {
        let (mn, mx) = stats
            .iter()
            .fold((u64::MAX, 0u64), |a, s| (a.0.min(s.cut_tris), a.1.max(s.cut_tris)));
        eprintln!(
            "{VTAG}: dolly 样本梯 cut_tris ∈ [{mn},{mx}]（相机驱动 cut measured 登记,不设通过线）"
        );
    }
    println!(
        "{VTAG}: PASS samples={} res={vw}x{vh}（cut 覆盖性机核 + SW device 覆盖集合与 oracle 全等 + 双跑位级 + resolve 恒等,全 fail-closed 已过）",
        stats.len(),
    );
}
