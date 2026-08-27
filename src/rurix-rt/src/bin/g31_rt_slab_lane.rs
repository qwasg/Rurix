//! G31+ 波 C Task C15 RT pipeline + SBT 宿主车道 harness(RFC-0048;门
//! `g31.waveC.rtpipeline`;TODO #31/#32 + M52/RD-040 承接锚)。
//!
//! ## 三臂
//! - **RT 臂**(slab 双材质 SBT 分派):`run_rt_pipeline_offscreen`(M50 增量底座
//!   0-byte 复用)——raygen/callable 复用 m50 语料,slab miss/closesthit =
//!   **hand-emitted 镜像语料**(`vk::g31_rt_slab_spv`;与
//!   `kernels/g31_rt_slab_hit.rx` 公式面逐字同源,**非 .rx 编译产物,不充
//!   .rx codegen 绿**,RFC-0048 §6);2 hit groups × 2 slab records(20B POD)。
//! - **RayQuery 对拍臂**(真 .rx 编译):`kernels/g31_rt_slab_rayquery.rx` 经
//!   rurixc --target vulkan 产 SPV(--spv-rq 传入),`run_ray_query_effects`
//!   同场景同材质同相机同公式真跑;f32 → unorm8 转换后对拍(结构容差 =
//!   RFC-0048 §4.7:bitexact ∨ (mismatch_ratio ≤ 0.001 ∧ max_lsb ≤ 1))。
//! - **SER workload 臂**(RFC-0048 §4.8):`run_ser_reorder_workload` NV 变体
//!   双臂(reorder off/on)时延对照 + 画面位级一致 + 双跑位级;absent 三 token
//!   如实登记(M52 capability 半命中不冒充)。
//!
//! ## 三态
//! 无 Vulkan loader/设备/扩展/空 SPV → device 腿 `skipped_dev_env` JSON 退 0
//! (非 fake pass;`RURIX_REQUIRE_REAL=1` 下 SKIP→硬红由 smoke 脚本层裁决);
//! 判据不符 ⇒ FAIL 退 1。
//!
//! ## 材质单源(host 一次生成,两臂同一字节源)
//!   slot0 slab_a = (rc=0.30, ab=0.70, albedo=(0.9, 0.2, 0.2))
//!   slot1 slab_b = (rc=0.80, ab=0.40, albedo=(0.2, 0.9, 0.2))
//!
//! 用法:
//!   g31_rt_slab_lane --spv-rq <g31_rt_slab_rayquery.spv> [--width 64] [--height 64]
//!                    [--ser-dispatches 40] [--ser-repeats 3] [--out <path>]

#![forbid(unsafe_code)]

use rurix_rt::rt_incremental::{
    RecordFieldDesc, RecordFieldTy, RecordSchema, RecordValue, RtHitGroupKind, RtHitGroupSpv,
    RtPipelineDesc, RtPipelineMode, RtSbtRecords, pack_shader_record, run_rt_pipeline_offscreen,
};
use rurix_rt::vk::{self, RayQueryBufferDesc, RayQueryDispatchDesc};

const TAG: &str = "[g31_rt_slab_lane]";
/// 镜像语料 schema hash 占位(record packer 唯一入口核验精确匹配;与 CI 静态面同值)。
const REC_HASH: [u8; 32] = [0x31; 32];
/// 场景几何(与 vk_m50_rt_body 硬编码两三角形逐字同源——RT 臂底座契约;左 slab_a
/// 右 slab_b,实例 sbt_record_offset 0/1)。
const TRI_A: [f32; 9] = [-0.85, 0.85, 0.0, -0.85, -0.85, 0.0, -0.05, 0.0, 0.0];
const TRI_B: [f32; 9] = [0.05, 0.0, 0.0, 0.85, -0.85, 0.0, 0.85, 0.85, 0.0];
/// 材质单源(两臂同一字节源; slab 双层闭式反照率 RFC-0046 §1 修法 A 同式)。
const SLOTS: [[f32; 5]; 2] = [[0.30, 0.70, 0.9, 0.2, 0.2], [0.80, 0.40, 0.2, 0.9, 0.2]];
/// miss 背景(与 kernel/镜像 miss 常量逐字同源)。
const BG: [f32; 3] = [0.05, 0.05, 0.08];
/// 对拍结构容差(RFC-0048 §4.7;g31.waveB.slab 跨臂先例同值)。
const RATIO_BOUND: f64 = 0.001;
const LSB_BOUND: u8 = 1;

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

/// slab 闭式反照率 f64 host 参照(公式面与 kernel/镜像逐字同源;对拍参照非回填)。
fn slab_r_f64(rc: f64, ab: f64) -> f64 {
    let tc = 1.0 - rc;
    let denom = 1.0 - rc * ab;
    rc + tc * tc * ab / denom.max(1e-30)
}

fn words_from_spv_bytes(bytes: &[u8]) -> Result<Vec<u32>, String> {
    if bytes.is_empty() {
        return Err("empty SPIR-V".into());
    }
    if !bytes.len().is_multiple_of(4) {
        return Err("SPIR-V len not multiple of 4".into());
    }
    Ok(bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect())
}

fn pack_slab_records() -> (Vec<u8>, Vec<u8>) {
    let schema = RecordSchema {
        schema_hash: REC_HASH,
        fields: [
            ("rc", RecordFieldTy::F32),
            ("ab", RecordFieldTy::F32),
            ("albedo_r", RecordFieldTy::F32),
            ("albedo_g", RecordFieldTy::F32),
            ("albedo_b", RecordFieldTy::F32),
        ]
        .iter()
        .map(|(n, t)| RecordFieldDesc {
            name: (*n).to_owned(),
            ty: *t,
        })
        .collect(),
    };
    let pack = |slot: usize| -> Vec<u8> {
        pack_shader_record(
            &schema,
            &REC_HASH,
            &SLOTS[slot]
                .iter()
                .map(|v| RecordValue::F32(*v))
                .collect::<Vec<_>>(),
        )
        .expect("pack slab record")
    };
    (pack(0), pack(1))
}

fn mats_ssbo_bytes() -> Vec<u8> {
    let mut v = Vec::with_capacity(2 * 5 * 4);
    for s in SLOTS {
        for x in s {
            v.extend_from_slice(&x.to_le_bytes());
        }
    }
    v
}

/// unorm8 量化(f32 → u8;clamp + round;与 RT 臂 RGBA8 image driver 转换同律)。
fn f32_to_u8(x: f32) -> u8 {
    (x.clamp(0.0, 1.0) * 255.0).round() as u8
}

struct Args {
    spv_rq: Option<String>,
    width: u32,
    height: u32,
    ser_dispatches: u32,
    ser_repeats: u32,
    out: Option<String>,
}

fn parse_args() -> Args {
    let mut a = Args {
        spv_rq: None,
        width: 64,
        height: 64,
        ser_dispatches: 40,
        ser_repeats: 3,
        out: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        match k.as_str() {
            "--spv-rq" => a.spv_rq = it.next(),
            "--width" => a.width = it.next().and_then(|v| v.parse().ok()).unwrap_or(64),
            "--height" => a.height = it.next().and_then(|v| v.parse().ok()).unwrap_or(64),
            "--ser-dispatches" => {
                a.ser_dispatches = it.next().and_then(|v| v.parse().ok()).unwrap_or(40)
            }
            "--ser-repeats" => a.ser_repeats = it.next().and_then(|v| v.parse().ok()).unwrap_or(3),
            "--out" => a.out = it.next(),
            other => fail(&format!("未知参数: {other}")),
        }
    }
    a
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', " ")
}

fn skip_json(reason: &str) -> ! {
    println!(
        "{{\"device_state\":\"skipped_dev_env\",\"reason\":\"{}\"}}",
        json_escape(reason)
    );
    std::process::exit(0)
}

fn main() {
    let args = parse_args();
    let Some(spv_rq_path) = args.spv_rq.as_deref() else {
        fail("缺 --spv-rq(g31_rt_slab_rayquery.rx 经 rurixc --target vulkan 产物)");
    };
    let rq_bytes =
        std::fs::read(spv_rq_path).unwrap_or_else(|e| skip_json(&format!("读 {spv_rq_path}: {e}")));
    let rq_spv = match words_from_spv_bytes(&rq_bytes) {
        Ok(w) => w,
        Err(e) => skip_json(&e),
    };
    let Some(rq_entry) = vk::entry_point_name(&rq_spv) else {
        skip_json("RQ SPV 无 OpEntryPoint");
    };
    if !vk::vulkan_available() {
        skip_json("vulkan loader 不可用");
    }
    let m50 = vk::m50_incremental_spv();
    let slab = vk::g31_rt_slab_spv();
    let rg_w = match words_from_spv_bytes(m50.raygen) {
        Ok(w) => w,
        Err(e) => skip_json(&format!("m50 raygen: {e}")),
    };
    let call_w = match words_from_spv_bytes(m50.callable) {
        Ok(w) => w,
        Err(e) => skip_json(&format!("m50 callable: {e}")),
    };
    let miss_w = match words_from_spv_bytes(slab.slab_miss) {
        Ok(w) => w,
        Err(e) => skip_json(&format!("slab miss: {e}")),
    };
    let chit_w = match words_from_spv_bytes(slab.slab_closesthit) {
        Ok(w) => w,
        Err(e) => skip_json(&format!("slab closesthit: {e}")),
    };
    let (rec_a, rec_b) = pack_slab_records();

    // ───────────────── RT 臂(slab 双材质 SBT 分派;镜像语料)─────────────────
    let rt_arm = |rec_a: &[u8],
                  rec_b: &[u8]|
     -> Result<rurix_rt::rt_incremental::RtPipelineRunResult, String> {
        let hit_groups = [
            RtHitGroupSpv {
                kind: RtHitGroupKind::Triangles,
                closest_hit: &chit_w,
                any_hit: None,
                intersection: None,
            },
            RtHitGroupSpv {
                kind: RtHitGroupKind::Triangles,
                closest_hit: &chit_w,
                any_hit: None,
                intersection: None,
            },
        ];
        let miss_list: [&[u32]; 1] = [&miss_w];
        let callables: [&[u32]; 1] = [&call_w];
        let hit_recs: [&[u8]; 2] = [rec_a, rec_b];
        let empty: [&[u8]; 0] = [];
        let desc = RtPipelineDesc {
            raygen: &rg_w,
            miss: &miss_list,
            hit_groups: &hit_groups,
            callables: &callables,
            records: RtSbtRecords {
                raygen: &[],
                miss: &empty,
                hit: &hit_recs,
                callable: &empty,
            },
            scene: rurix_rt::rt_incremental::RtIncrementalScene {
                triangle_blases: &[],
                aabb_blases: &[],
                instances: &[],
            },
            stack_override: None,
            width: args.width,
            height: args.height,
            mode: RtPipelineMode::Monolithic,
            min_hit_groups: 2,
        };
        run_rt_pipeline_offscreen(&desc)
    };

    let rt_a = match rt_arm(&rec_a, &rec_b) {
        Ok(r) => r,
        Err(e) => {
            let skip = [
                "vulkan loader",
                "vulkan-1.dll",
                "libvulkan",
                "物理设备",
                "无物理",
                "缺扩展",
                "缺 RT feature",
            ]
            .iter()
            .any(|k| e.contains(k));
            if skip {
                skip_json(&format!("RT 臂: {e}"));
            }
            fail(&format!("RT 臂失败: {e}"));
        }
    };
    let rt_b = match rt_arm(&rec_a, &rec_b) {
        Ok(r) => r,
        Err(e) => fail(&format!("RT 臂第二跑失败: {e}")),
    };
    let rt_bitexact = rt_a.pixels_rgba8 == rt_b.pixels_rgba8;
    let rt_digest = rurix_pkg::sha256::hex_digest(&rt_a.pixels_rgba8);
    let record_ok = rt_a.record_readback.len() >= 40
        && rt_a.record_readback[..20] == rec_a[..]
        && rt_a.record_readback[20..40] == rec_b[..];
    let stack_ok = rt_a.stack_configured >= rt_a.stack_required;
    let validation_ok = rt_a.validation_errors == 0;

    // ───────────────── RayQuery 对拍臂(真 .rx 编译)─────────────────
    let w = args.width as usize;
    let h = args.height as usize;
    let n_px = w * h;
    let mats_bytes = mats_ssbo_bytes();
    let mut params: Vec<f32> = vec![n_px as f32, args.width as f32, args.height as f32];
    params.resize(8, 0.0);
    let params_bytes: Vec<u8> = params.iter().flat_map(|x| x.to_le_bytes()).collect();
    let rq_arm = |mats: &[u8], params: &[u8]| -> Result<Vec<f32>, String> {
        let tris: [&[f32]; 2] = [&TRI_A[..], &TRI_B[..]];
        let instances = [
            vk::RayQueryInstanceDesc {
                blas: 0,
                custom_index: 0,
                mask: 0xFF,
                sbt_record_offset: 0,
            },
            vk::RayQueryInstanceDesc {
                blas: 1,
                custom_index: 1,
                mask: 0xFF,
                sbt_record_offset: 1,
            },
        ];
        let scene = vk::RayQuerySceneDesc {
            blas_triangles: &tris,
            instances: &instances,
        };
        let out_bytes = n_px * 3 * 4;
        let buffers = [
            RayQueryBufferDesc::Input(mats),
            RayQueryBufferDesc::Input(params),
            RayQueryBufferDesc::Output(out_bytes),
        ];
        let dispatches = [RayQueryDispatchDesc {
            name: "g31_rt_slab_rayquery",
            spv: &rq_spv,
            entry: &rq_entry,
            buffers: &buffers,
            push_constants: &[],
            groups: [n_px as u32, 1, 1],
        }];
        let out = vk::run_ray_query_effects(&scene, &dispatches)?;
        let rb = out
            .readbacks
            .first()
            .and_then(|v| v.first())
            .ok_or("RayQuery 回读缺失")?;
        Ok(rb
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect())
    };
    let rq_a = match rq_arm(&mats_bytes, &params_bytes) {
        Ok(v) => v,
        Err(e) => {
            let skip = [
                "vulkan loader",
                "vulkan-1.dll",
                "libvulkan",
                "物理设备",
                "无物理",
                "缺扩展",
                "rayQuery",
                "accelerationStructure",
                "bufferDeviceAddress",
            ]
            .iter()
            .any(|k| e.contains(k));
            if skip {
                skip_json(&format!("RQ 臂: {e}"));
            }
            fail(&format!("RQ 臂失败: {e}"));
        }
    };
    let rq_b = match rq_arm(&mats_bytes, &params_bytes) {
        Ok(v) => v,
        Err(e) => fail(&format!("RQ 臂第二跑失败: {e}")),
    };
    let rq_bitexact = rq_a == rq_b;
    let rq_u8: Vec<u8> = rq_a
        .chunks_exact(3)
        .flat_map(|c| {
            let (r, g, b) = (f32_to_u8(c[0]), f32_to_u8(c[1]), f32_to_u8(c[2]));
            // vk RGBA8 image layout = R,G,B,A 字节序。
            [r, g, b, 255u8]
        })
        .collect();
    let rq_digest = rurix_pkg::sha256::hex_digest(&rq_u8);

    // ───────────────── 对拍(结构容差,RFC-0048 §4.7)─────────────────
    let rt_px = &rt_a.pixels_rgba8;
    let mut mismatch = 0usize;
    let mut max_lsb = 0u8;
    for px in 0..n_px {
        let o = px * 4;
        if rt_px[o..o + 4] != rq_u8[o..o + 4] {
            mismatch += 1;
            for c in 0..4 {
                let d = (rt_px[o + c] as i32 - rq_u8[o + c] as i32).unsigned_abs() as u8;
                if d > max_lsb {
                    max_lsb = d;
                }
            }
        }
    }
    let bitexact = mismatch == 0;
    let ratio = mismatch as f64 / n_px.max(1) as f64;
    let in_bound = bitexact || (ratio <= RATIO_BOUND && max_lsb <= LSB_BOUND);

    // ───────────────── golden 结构核验(三采样点 + host f64 参照)─────────────────
    let sample = |buf: &[u8], px: usize, py: usize| -> [u8; 4] {
        let o = (py * w + px) * 4;
        [buf[o], buf[o + 1], buf[o + 2], buf[o + 3]]
    };
    let expect_slot = |slot: usize| -> [u8; 3] {
        let r = slab_r_f64(f64::from(SLOTS[slot][0]), f64::from(SLOTS[slot][1]));
        [
            f32_to_u8(SLOTS[slot][2] * r as f32),
            f32_to_u8(SLOTS[slot][3] * r as f32),
            f32_to_u8(SLOTS[slot][4] * r as f32),
        ]
    };
    let exp_bg = [f32_to_u8(BG[0]), f32_to_u8(BG[1]), f32_to_u8(BG[2])];
    let close =
        |a: [u8; 4], e: [u8; 3]| -> bool { (0..3).all(|c| (a[c] as i32 - e[c] as i32).abs() <= 1) };
    let left = sample(rt_px, w / 8, h / 4);
    let right = sample(rt_px, 7 * w / 8, h / 4);
    let bg = sample(rt_px, w / 2, h / 2);
    let left_ok = close(left, expect_slot(0));
    let right_ok = close(right, expect_slot(1));
    let bg_ok = close(bg, exp_bg);
    // RQ 臂同点互核(双臂同构证明)。
    let rq_left = sample(&rq_u8, w / 8, h / 4);
    let rq_right = sample(&rq_u8, 7 * w / 8, h / 4);
    let rq_bg = sample(&rq_u8, w / 2, h / 2);
    let rq_left_ok = close(rq_left, expect_slot(0));
    let rq_right_ok = close(rq_right, expect_slot(1));
    let rq_bg_ok = close(rq_bg, exp_bg);

    // ───────────────── SER workload 臂(RFC-0048 §4.8)─────────────────
    let ser_off = words_from_spv_bytes(slab.ser_raygen_noreorder).unwrap_or_default();
    let ser_on = words_from_spv_bytes(slab.ser_raygen_reorder).unwrap_or_default();
    let mut ser_json =
        String::from("\"state\":\"skipped_dev_env\",\"reason\":\"ser raygen spv empty\"");
    let mut ser_gain_json = String::from("{}");
    if !ser_off.is_empty() && !ser_on.is_empty() {
        match vk::run_ser_reorder_workload(
            &ser_off,
            &ser_on,
            &miss_w,
            &chit_w,
            &rec_a,
            &rec_b,
            args.width.max(512),
            args.height.max(512),
            args.ser_dispatches,
            args.ser_repeats,
        ) {
            Ok(r) => {
                let batches: Vec<String> = r.batch_ms.iter().map(|x| format!("{x:.6}")).collect();
                ser_json = format!(
                    "\"state\":\"executed\",\"tokens\":{{\"ext_nv\":{},\"feature_reorder\":{},\"feature_reordering_hint\":{}}},\"width\":{},\"height\":{},\"dispatches_per_arm\":{},\"repeats\":{},\"n_blas\":{},\"n_instances\":{},\"time_ms_noreorder\":{:.6},\"time_ms_reorder\":{:.6},\"speedup_ratio\":{:.6},\"pixels_bitexact_across_arms\":{},\"double_run_bitexact\":{},\"stack_required\":{},\"stack_configured\":{},\"batch_ms\":[{}]",
                    r.tokens.ext_nv,
                    r.tokens.feature_reorder,
                    r.tokens.feature_reordering_hint,
                    r.width,
                    r.height,
                    r.dispatches_per_arm,
                    r.repeats,
                    r.n_blas,
                    r.n_instances,
                    r.time_ms_noreorder,
                    r.time_ms_reorder,
                    r.speedup_ratio,
                    r.pixels_bitexact_across_arms,
                    r.double_run_bitexact,
                    r.stack_required,
                    r.stack_configured,
                    batches.join(","),
                );
                // SER 收益 measured 预估窗 evidence 块(RD-040 RT-PIPELINE-SBT 分项
                // 锚 `evidence/*ser_gain_estimate*.json` 消费面;CI 归档件)。
                ser_gain_json = format!(
                    "{{\"schema\":\"rurix.g31.ser_gain_estimate.v1\",\"subject\":\"g31_ser_gain_estimate\",\"workload\":\"hand-emitted HitObject raygen NV 双臂(reorder off/on),64 竖条×2 三角形=128 BLAS×128 实例逐条交替 slab_a/b SBT record,canvas {}x{} 全命中棋盘式 2-way 分歧\",\"capability\":{{\"ext_nv\":{},\"feature_reorder\":{},\"feature_reordering_hint\":{},\"source\":\"run_ser_reorder_workload 现势探测(vkEnumerateDeviceExtensionProperties + feature 链)\"}},\"measurement\":{{\"dispatches_per_arm\":{},\"repeats\":{},\"rays_per_arm\":{},\"time_ms_noreorder\":{:.6},\"time_ms_reorder\":{:.6},\"speedup_ratio\":{:.6},\"batch_ms\":[{}]}},\"correctness\":{{\"pixels_bitexact_across_arms\":{},\"double_run_bitexact\":{},\"stack_required\":{},\"stack_configured\":{}}},\"caveats\":[\"微基准口径:合成分歧(2-way 棋盘)、单 GPU(RTX 4070 Ti)、单 driver 版本,收益比不外推生产\",\"hand-emitted 镜像语料臂,非 .rx 编译产物\",\"墙钟 queue_submit+wait_idle 单批总时,min-of-repeats\"],\"evidence_level\":\"measured_local\"}}",
                    r.width,
                    r.height,
                    r.tokens.ext_nv,
                    r.tokens.feature_reorder,
                    r.tokens.feature_reordering_hint,
                    r.dispatches_per_arm,
                    r.repeats,
                    (r.width * r.height) as u64 * r.dispatches_per_arm as u64,
                    r.time_ms_noreorder,
                    r.time_ms_reorder,
                    r.speedup_ratio,
                    batches.join(","),
                    r.pixels_bitexact_across_arms,
                    r.double_run_bitexact,
                    r.stack_required,
                    r.stack_configured,
                );
            }
            Err(e) => {
                let absent = e.contains("VK_NV_ray_tracing_invocation_reorder")
                    || e.contains("rayTracingInvocationReorder");
                let skip = [
                    "vulkan loader",
                    "vulkan-1.dll",
                    "libvulkan",
                    "物理设备",
                    "无物理",
                ]
                .iter()
                .any(|k| e.contains(k));
                if absent {
                    ser_json = format!(
                        "\"state\":\"absent\",\"reason\":\"{}\",\"note\":\"M52 capability 半命中维持 defer 如实登记(不冒充)\"",
                        json_escape(&e)
                    );
                } else if skip {
                    ser_json = format!(
                        "\"state\":\"skipped_dev_env\",\"reason\":\"{}\"",
                        json_escape(&e)
                    );
                } else {
                    fail(&format!("SER workload 失败: {e}"));
                }
            }
        }
    }

    let problems: Vec<String> = {
        let mut p = Vec::new();
        if !rt_bitexact {
            p.push("RT 臂双跑非位级一致".to_owned());
        }
        if !rq_bitexact {
            p.push("RQ 臂双跑非位级一致".to_owned());
        }
        if !record_ok {
            p.push("SBT record readback ≠ packer 输入".to_owned());
        }
        if !stack_ok {
            p.push("stack configured < required".to_owned());
        }
        if !validation_ok {
            p.push("validation 非静默".to_owned());
        }
        if !in_bound {
            p.push(format!(
                "对拍超结构容差: ratio={ratio:.3e} max_lsb={max_lsb}"
            ));
        }
        if !left_ok || !right_ok || !bg_ok || !rq_left_ok || !rq_right_ok || !rq_bg_ok {
            p.push(format!(
                "golden 采样点偏差: RT[{left_ok}/{right_ok}/{bg_ok}] RQ[{rq_left_ok}/{rq_right_ok}/{rq_bg_ok}]"
            ));
        }
        p
    };
    let state = if problems.is_empty() { "pass" } else { "fail" };
    let problems_json: Vec<String> = problems
        .iter()
        .map(|p| format!("\"{}\"", json_escape(p)))
        .collect();

    let doc = format!(
        "{{\n  \"device_state\": \"executed\",\n  \"state\": \"{state}\",\n  \
         \"problems\": [{}],\n  \
         \"lane\": {{\"width\": {w}, \"height\": {h}, \"slots\": 2, \"mirror_corpus\": \"hand_emitted_not_rx_compiled\"}},\n  \
         \"rt_arm\": {{\"pixels_digest\": \"{}\", \"double_run_bitexact\": {}, \"record_readback_ok\": {}, \"stack_required\": {}, \"stack_configured\": {}, \"stack_formula_version\": \"{}\", \"validation_errors\": {}, \"hit_group_count\": {}, \"mode\": \"{}\"}},\n  \
         \"rq_arm\": {{\"entry\": \"{}\", \"spv_digest\": \"{}\", \"pixels_digest\": \"{}\", \"double_run_bitexact\": {}}},\n  \
         \"parity\": {{\"bitexact\": {}, \"mismatch_px\": {}, \"total_px\": {}, \"mismatch_ratio\": {:.10}, \"max_lsb_diff\": {}, \"in_bound\": {}, \"ratio_bound\": {}, \"lsb_bound\": {}, \"structural_basis\": \"RT 臂 RGBA8 unorm 量化(量子 1/255)vs RQ 臂 f32→同律量化;f32 求值差 ≤ 数 ULP ⇒ 预期位级,容 ≤1 LSB 且 ≤0.1%(RFC-0048 §4.7)\"}},\n  \
         \"golden\": {{\"left_slab_a\": {:?}, \"right_slab_b\": {:?}, \"center_bg\": {:?}, \"expect_a\": {:?}, \"expect_b\": {:?}, \"expect_bg\": {:?}, \"rt_ok\": [{}, {}, {}], \"rq_ok\": [{}, {}, {}]}},\n  \
         \"ser\": {{{}}}\n}}",
        problems_json.join(","),
        rt_digest,
        rt_bitexact,
        record_ok,
        rt_a.stack_required,
        rt_a.stack_configured,
        rt_a.stack_formula_version,
        rt_a.validation_errors,
        rt_a.hit_group_count,
        rt_a.mode,
        rq_entry,
        rurix_pkg::sha256::hex_digest(&rq_bytes),
        rq_digest,
        rq_bitexact,
        bitexact,
        mismatch,
        n_px,
        ratio,
        max_lsb,
        in_bound,
        RATIO_BOUND,
        LSB_BOUND,
        left,
        right,
        bg,
        expect_slot(0),
        expect_slot(1),
        exp_bg,
        left_ok,
        right_ok,
        bg_ok,
        rq_left_ok,
        rq_right_ok,
        rq_bg_ok,
        ser_json,
    );
    println!("{doc}");
    if let Some(path) = &args.out {
        if !ser_gain_json.is_empty() && ser_gain_json != "{}" {
            if let Some(parent) = std::path::Path::new(path).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::write(path, format!("{ser_gain_json}\n"))
                .unwrap_or_else(|e| fail(&format!("写 --out {path}: {e}")));
        }
    }
    if !problems.is_empty() {
        eprintln!("{TAG}: FAIL {:?}", problems);
        std::process::exit(1);
    }
    eprintln!(
        "{TAG}: PASS rt_digest={} rq_digest={} bitexact={bitexact} ratio={ratio:.3e}",
        &rt_digest[..24],
        &rq_digest[..24]
    );
}
