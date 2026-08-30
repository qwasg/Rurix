// Assisted-by: Claude（G37 W3 frame-cut）
//! G37 W3：逐帧 device cut → AS 更新 判档 harness（TODO #77 × #89 合流窗;
//! G36 五留窗「出帧几何冻结于装配期选层」的解冻最小判档面）。
//! 臂实现 = `g14_3_lane/g31_frame_cut_arm.rs` 单源（窗口 --cluster-per-frame-cut
//! 合入提案消费同一文件,禁旁路复刻——本 bin 只做装配/轨迹/编排）。
//!
//! 链：生产契约装配 bistro（prelude digest 门 + assemble_scene 单源）→ RXCP
//! 簇包读取 + `verify_cluster_pack` fail-closed → 固定前向 dolly 轨迹 N 帧 →
//! `run_frame_cut_arm`（全簇固定槽位 refit 竞技场:逐帧 host 金标准 cut →
//! 槽位增量〔进 cut 真几何/出 cut 零面积折叠〕→ `FrameUpdate::blas_refit`
//! UPDATE build〔B5 冻结通路〕→ ray query 命中流 digest;判据 = 双跑逐帧
//! digest 位级 + cut_tris 单调变化 + 命中槽位 ∈ 已施加 cut + 哨兵 canary +
//! 零命中防伪,全 fail-closed）→ sidecar JSON（AS 更新 measured 分解）。
//!
//! 三态：无 Vulkan loader / bistro 资产缺失 → `skipped_dev_env` 退 0（不冒充
//! PASS;`RURIX_REQUIRE_REAL=1` 翻硬 FAIL）。`--selftest` = 纯 host 腿（零
//! device:合成 DAG 单调细化/覆盖性/竞技场增量写器/双跑确定性/kernel 结构）。
//!
//! 用法：
//!   g31_frame_cut_probe --selftest
//!   g31_frame_cut_probe --cluster-pack <bistro.rxcp> [--contract <json>]
//!     [--gltf <path>] [--tier 100] [--error-px 2.0] [--frames 16]
//!     [--step-m 0.15] [--res 96x54] [--cut-every 1] [--blocks-limit 0]
//!     [--evidence <sidecar.json>]
#![forbid(unsafe_code)]
// 共享体含本 bin 未消费面（bench/render 腿、vendor 双臂、EXR 出图等）——
// dead_code 豁免如实登记;本 bin 消费面 = 契约装配/簇包读取/frame-cut 臂。
#![allow(dead_code)]

include!("g14_3_lane/g14_3_lane_body.rs");
include!("g14_3_lane/g31_frame_cut_arm.rs");

const FCTAG: &str = "[g31_frame_cut_probe]";

fn fcskip(reason: &str) {
    println!(
        "{FCTAG}: {{\"state\":\"skipped_dev_env\",\"reason\":{}}}",
        jstr(reason)
    );
    if require_real() {
        eprintln!("{FCTAG}: FAIL RURIX_REQUIRE_REAL=1 但 device 面降级");
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
    let mut frames: u32 = 16;
    let mut step_m: f32 = 0.15;
    let mut res = String::from("96x54");
    let mut cut_every: u32 = 1;
    let mut blocks_limit: usize = 0;
    let mut evidence = String::new();
    let mut selftest = false;
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
            "--frames" => {
                frames = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--frames 非 u32"))
            }
            "--step-m" => {
                step_m = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--step-m 非 f32"))
            }
            "--res" => res = take_arg(&args, &mut i),
            "--cut-every" => {
                cut_every = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--cut-every 非 u32"))
            }
            "--blocks-limit" => {
                blocks_limit = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--blocks-limit 非 usize"))
            }
            "--evidence" => evidence = take_arg(&args, &mut i),
            "--selftest" => selftest = true,
            other => fail(&format!("未知参数 {other}")),
        }
        i += 1;
    }
    if selftest {
        frame_cut_selftest(FCTAG);
        println!("{FCTAG}: PASS selftest（host 腿;device 判档归 GPU 验收窗）");
        return;
    }
    if pack_path.is_empty() {
        fail("--cluster-pack <RXCP> 必填（g31_cluster_lod_bake 产物;或 --selftest 走 host 腿）");
    }
    if !(error_px.is_finite() && error_px > 0.0) {
        fail("--error-px 必须为正有限 f32");
    }
    if frames < 2 {
        fail("--frames 必须 ≥2（单调变化判据需相机推进）");
    }
    if !(step_m.is_finite() && step_m > 0.0) {
        fail("--step-m 必须为正有限 f32");
    }
    if cut_every == 0 {
        fail("--cut-every 必须 ≥1（1 = 逐帧;>1 = 惰性节拍臂）");
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
        fcskip("vulkan loader 不可用");
        return;
    }
    if !Path::new(&pack_path).is_file() {
        fcskip(&format!("RXCP 簇包缺失 {pack_path}"));
        return;
    }

    // 契约装配（prelude digest 门 + 装配语义单源;资产缺失 = dev_env 降级）。
    let scene_id = "bistro-interior";
    let (pre, _) = prelude(scene_id, tier, 1, false, &contract_path, None);
    if gltf_path.is_empty() {
        gltf_path = default_gltf(scene_id).to_owned();
    }
    if !Path::new(&gltf_path).is_file() {
        fcskip(&format!("bistro gltf 缺失 {gltf_path}"));
        return;
    }
    let scene = match assemble_scene(&pre.contract.raw, scene_id, Path::new(&gltf_path)) {
        Ok(s) => s,
        Err(e) => {
            fcskip(&format!("场景装配: {e}"));
            return;
        }
    };
    eprintln!(
        "{FCTAG}: 装配就绪 tris={} in={}x{} tier={tier}（LOD 判据分辨率 = 内部分辨率,#58/W2 同口径）",
        scene.tri_count, pre.in_w, pre.in_h,
    );

    // 簇包读取 + fail-closed 校验（--cluster-lod 同一冻结校验面直调）。
    let pack = read_cluster_pack(Path::new(&pack_path))
        .unwrap_or_else(|e| fail(&format!("RXCP 读取: {e}")));
    verify_cluster_pack(&pack, &scene)
        .unwrap_or_else(|e| fail(&format!("簇包校验 fail-closed: {e}")));
    eprintln!(
        "{FCTAG}: 簇包就绪 blocks={} clusters={} passthrough={}（gltf sha/覆盖恰一次/叶几何位级 已核）",
        pack.blocks.len(),
        pack.blocks.iter().map(|b| b.records.len()).sum::<usize>(),
        pack.passthrough.len(),
    );

    // 固定轨迹：装配相机 + 前向 XZ dolly k×step_m（确定性协议的轨迹半;
    // 真窗口逐帧轨迹 = 窗口臂合入后消费,--auto-move dolly 同向）。
    let cam0 = scene.camera;
    let fxz = (cam0.forward[0] * cam0.forward[0] + cam0.forward[2] * cam0.forward[2]).sqrt();
    if fxz <= 1e-6 {
        fail("装配相机前向 XZ 退化（dolly 轨迹无定义）");
    }
    let step = [
        cam0.forward[0] / fxz * step_m,
        0.0,
        cam0.forward[2] / fxz * step_m,
    ];
    let samples: Vec<FrameCutCamSample> = (0..frames)
        .map(|k| {
            let mut spec = cam0;
            spec.eye = [
                cam0.eye[0] + step[0] * k as f32,
                cam0.eye[1],
                cam0.eye[2] + step[2] * k as f32,
            ];
            FrameCutCamSample {
                frame: k,
                spec,
                in_w: pre.in_w,
                in_h: pre.in_h,
            }
        })
        .collect();

    let opt = FrameCutArmOpt {
        enabled: true,
        res_w: vw,
        res_h: vh,
        frames,
        step_m,
        cut_every,
        blocks_limit,
        // probe = 固定单向 dolly ⇒ 单调严门（窗口真轨迹臂 = 宽门,合入提案登记）。
        monotone_gate: true,
        out_path: evidence,
    };
    // passthrough 源三角流须自源装配场景提取（窗口合入 = apply_cluster_lod
    // 施加前锚点同式;probe 场景未经 cut 重建,直接提取）。
    let pt_stream = frame_cut_passthrough_stream(&scene, &pack.passthrough);
    let stats = run_frame_cut_arm(FCTAG, &pack, &pt_stream, &opt, error_px, &samples);
    frame_cut_finish(FCTAG, &pack, &opt, error_px, &stats);
    println!(
        "{FCTAG}: PASS frames={} res={vw}x{vh} cut_every={cut_every}（逐帧 cut→BLAS refit→RQ 出帧;双跑 digest 位级 + cut_tris 单调 + 命中∈已施加 cut + 哨兵 canary,全 fail-closed 已过）",
        stats.len(),
    );
}
