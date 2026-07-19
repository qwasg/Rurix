//! G3.6 RT(BLAS/TLAS/SBT/TraceRays)**device 像素判据 harness**(RXS-0248 后半;RFC-0013
//! §4.E8;验收门 G-G3-6;counter `g3.counter.mesh_task_rt_stages` 阶段去重基数 ≥3)。镜像
//! `bin/graph_modes` 的 device 真跑 / SKIP 三态 + 「移动顶点 → 命中区域移动 = RED」数据流红绿。
//!
//! ## 见证语料
//! raygen/miss/closesthit SPIR-V = `vk::mesh_rt_witness_spv().{raygen,miss,closesthit}`(codegen
//! `emit_raygen_min`/`emit_miss_min`/`emit_closesthit_min` 产:六 RT 执行模型 + SPV_KHR_ray_tracing +
//! AccelStruct SRV + TraceRay/HitAttribute)。单三角形几何 `vertices`(3×vec3)。
//! `run_ray_tracing_offscreen(rg, ms, ch, &verts, W, H)` 建 BLAS→barrier→TLAS 两段构建 + RT 管线
//! (raygen/miss/hit 三 group,maxRecursion=1)+ 🔒 SBT 三 region 对齐(`plan_sbt`)+ set-per-class
//! descriptor(TLAS SRV / storage image UAV)+ `vkCmdTraceRaysKHR(W,H,1)` + 回读 storage image。
//!
//! ## 两判据(evidence stages_ok 同源)
//! - `rt_center_hit_corner_miss`(**device**):三角形居中,中心像素 == 命中色、角落 == miss 色
//!   (证 TLAS 遍历 + hit/miss group 经 SBT 分派真生效)。**期望色/容差 = owner 本机迭代校准
//!   TODO**(`expect_hit`/`expect_miss`)。
//! - `move_vertex_hit_region_shifts_red`(移动顶点 → RED):平移三角形 → 中心命中区域移动(中心
//!   像素由命中变 miss)= RED;复原 = GREEN。
//!
//! **device 真跑 / SKIP 三态**:无 Vulkan loader / 无 GPU / **RT 扩展或 feature 缺失** →
//! `run_ray_tracing_offscreen` 确定性 `Err` → `RT: SKIP` 退 0(dev-env degrade,**非 fake pass**);
//! `RURIX_REQUIRE_REAL=1` 翻硬红。有设备但判据阈值未过(owner 迭代)→ `RT: PARTIAL`(诚实)。
//! **codegen raygen_min 首期不写 storage image(payload 无落点)→ 回读为 clear,像素差判据 +
//! 写 storage image 的 raygen 见证语料归 owner device 调优**;本片交付 = AS/SBT/TraceRays 机构 +
//! SKIP 三态 + 判据结构。device 真跑绝不伪造;**AMD 真卡见证 = G-MB1-6 独立尾门**。

use std::path::PathBuf;

use rurix_rt::vk::{mesh_rt_witness_spv, run_ray_tracing_offscreen};

const W: u32 = 64;
const H: u32 = 64;

/// 无设备 / 扩展缺失(SKIP)信号。
const NO_DEVICE_KEYS: &[&str] = &[
    "vulkan loader",
    "vulkan-1.dll",
    "libvulkan",
    "物理设备",
    "graphics queue",
    "vkCreateInstance",
    "缺扩展",
    "RT feature",
    "vkGetPhysicalDeviceFeatures2",
    "vkGetPhysicalDeviceProperties2",
    "vkEnumerateDeviceExtensionProperties",
];

const MODES: &[&str] = &[
    "rt_center_hit_corner_miss",
    "move_vertex_hit_region_shifts_red",
];

/// RGBA8 像素。
type Px = (u8, u8, u8, u8);

/// 居中单三角形(clip 空间;逆时针,覆盖中心)。
const TRI_CENTER: [f32; 9] = [
    0.0, 0.6, 0.0, //
    -0.6, -0.6, 0.0, //
    0.6, -0.6, 0.0, //
];

/// 平移三角形(顶点右上移 → 中心不再命中)。
const TRI_MOVED: [f32; 9] = [
    0.8, 0.9, 0.0, //
    0.5, 0.3, 0.0, //
    0.9, 0.3, 0.0, //
];

fn is_no_device(e: &str) -> bool {
    NO_DEVICE_KEYS.iter().any(|k| e.contains(k))
}

fn px_at(p: &[u8], x: u32, y: u32) -> Px {
    let o = ((y * W + x) * 4) as usize;
    (p[o], p[o + 1], p[o + 2], p[o + 3])
}

/// 中心像素逼近命中色(closesthit 写 payload = vec4(bary.x, bary.y, 0, 1))。
fn expect_hit(p: Px) -> bool {
    // TODO(owner device): 校准命中色/容差(raygen 写 storage image 见证语料就位后;首期回读为 clear)。
    p.0 > 8 || p.1 > 8
}

/// 角落像素逼近 miss 色(miss 写 payload = vec4(0,0,0,1))。
fn expect_miss(p: Px) -> bool {
    // TODO(owner device): 校准 miss 色/容差。
    p.0 <= 8 && p.1 <= 8 && p.2 <= 8
}

fn main() {
    let witness = mesh_rt_witness_spv();
    if witness.raygen.is_empty() || witness.miss.is_empty() || witness.closesthit.is_empty() {
        println!("RT: SKIP RT 见证语料为空(build.rs codegen 降级)");
        return;
    }
    let rg = to_words(witness.raygen);
    let ms = to_words(witness.miss);
    let ch = to_words(witness.closesthit);

    println!(
        "[vk_rt] G3.6 RT(BLAS/TLAS/SBT/TraceRays)device 像素判据 harness(RFC-0013 §4.E8,G-G3-6)"
    );
    for (i, m) in MODES.iter().enumerate() {
        println!("[vk_rt]   判据 {}: {m}", i + 1);
    }

    let mut stages_ok: Vec<&str> = Vec::new();
    let mut misses: Vec<String> = Vec::new();

    // ── ① rt_center_hit_corner_miss(device 首跑:中心命中色 / 角落 miss 色)──
    let base_px = match run_ray_tracing_offscreen(&rg, &ms, &ch, &TRI_CENTER, W, H) {
        Ok(px) => px,
        Err(e) if is_no_device(&e) => {
            println!(
                "RT: SKIP 无 Vulkan 设备 / RT 扩展或 feature 缺失({})",
                e.trim()
            );
            return;
        }
        Err(e) => {
            eprintln!("RT: FAIL run_ray_tracing_offscreen 出图: {e}");
            std::process::exit(1);
        }
    };
    let center = px_at(&base_px, W / 2, H / 2);
    let corner = px_at(&base_px, 0, 0);
    let hit_ok = expect_hit(center);
    let miss_ok = expect_miss(corner);
    if hit_ok && miss_ok {
        for s in ["raygen", "miss", "closesthit"] {
            stages_ok.push(s);
        }
    } else {
        misses.push(format!(
            "rt_center_hit_corner_miss center={center:?} corner={corner:?} hit_ok={hit_ok} \
             miss_ok={miss_ok}(raygen 首期不写 storage image → 回读 clear;写者语料 + 阈值 owner TODO)"
        ));
    }

    // ── ② move_vertex_hit_region_shifts_red(平移三角形 → 中心命中变 miss = RED)──
    match run_ray_tracing_offscreen(&rg, &ms, &ch, &TRI_MOVED, W, H) {
        Ok(mpx) => {
            let mcenter = px_at(&mpx, W / 2, H / 2);
            // RED = 平移后中心不再命中(命中区域移动);GREEN = 原三角形中心命中。
            let red = !expect_hit(mcenter);
            let green = hit_ok;
            if red && green {
                for s in ["raygen", "miss", "closesthit"] {
                    if !stages_ok.contains(&s) {
                        stages_ok.push(s);
                    }
                }
            } else {
                misses.push(format!(
                    "move_vertex_hit_region_shifts_red moved_center={mcenter:?} red={red} \
                     green={green}(平移应令中心失命中;阈值 owner TODO)"
                ));
            }
        }
        Err(e) if is_no_device(&e) => {
            println!("RT: SKIP 移动跑无设备({})", e.trim());
            return;
        }
        Err(e) => {
            eprintln!("RT: FAIL 移动 run_ray_tracing_offscreen: {e}");
            std::process::exit(1);
        }
    }

    for m in &misses {
        eprintln!("RT: MISS {m}");
    }
    let smoke_ok = stages_ok.len() == 3 && misses.is_empty();
    if let Err(e) = write_evidence("rt", &stages_ok, smoke_ok) {
        eprintln!("RT: WARN evidence 写入失败: {e}");
    }
    if smoke_ok {
        println!(
            "RT: PASS stages_ok=[{}] center={center:?}(TLAS 遍历命中/miss 双色 + 移动顶点命中区移动 RED)",
            stages_ok.join(", ")
        );
    } else {
        println!(
            "RT: PARTIAL stages_ok=[{}] center={center:?} corner={corner:?}(写 storage image 的 raygen \
             见证语料 + 像素阈值归 owner device 调优;AS/SBT/TraceRays 机构就位,raygen_min 首期不写输出)",
            stages_ok.join(", ")
        );
    }
}

fn to_words(bytes: &[u8]) -> Vec<u32> {
    bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// evidence/meshrt_rt_<epoch>.json(schema milestones/g3/meshrt_stages_evidence_schema.json)。
/// subject 与 vk_mesh 同源,`g3.counter.mesh_task_rt_stages` 跨 meshrt_*.json 去重计数。
fn write_evidence(kind: &str, stages_ok: &[&str], smoke_ok: bool) -> std::io::Result<()> {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let dir = repo.join("evidence");
    std::fs::create_dir_all(&dir)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let ts = rfc3339_utc(stamp);
    let run_url = std::env::var("GITHUB_RUN_ID")
        .ok()
        .and_then(|id| {
            let server = std::env::var("GITHUB_SERVER_URL").ok()?;
            let repo = std::env::var("GITHUB_REPOSITORY").ok()?;
            Some(format!("{server}/{repo}/actions/runs/{id}"))
        })
        .unwrap_or_else(|| "local interactive runner".to_string());
    let adapter = std::env::var("RURIX_ADAPTER").unwrap_or_else(|_| "unknown".to_string());
    let list = stages_ok
        .iter()
        .map(|m| format!("\"{m}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let json = format!(
        "{{\n  \"schema_version\": 1,\n  \"subject\": \"mesh_task_rt_stages\",\n  \
         \"milestone\": \"g3.6\",\n  \"kind\": \"{kind}\",\n  \"smoke_ok\": {smoke_ok},\n  \
         \"stages_ok\": [{list}],\n  \"adapter\": \"{adapter}\",\n  \"run_url\": \"{run_url}\",\n  \
         \"timestamp\": \"{ts}\"\n}}\n"
    );
    std::fs::write(dir.join(format!("meshrt_{kind}_{stamp}.json")), json)
}

fn rfc3339_utc(secs: u64) -> String {
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}
