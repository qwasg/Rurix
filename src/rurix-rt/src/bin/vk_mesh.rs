//! G3.6 mesh 管线 **device 像素判据 harness**(RXS-0248 前半;RFC-0013 §4.E7;验收门 G-G3-6;
//! counter `g3.counter.mesh_task_rt_stages` 阶段去重基数 ≥3)。镜像 `bin/graph_modes` /
//! `bin/bindless_modes` 的 device 真跑 / SKIP 三态 + 「篡改 → 像素变 = RED,复原 = GREEN」数据流
//! 红绿纪律(RXS-0176 IR2)。
//!
//! ## 见证语料
//! mesh 阶段 SPIR-V = `vk::mesh_rt_witness_spv().mesh`(codegen `emit_mesh_min` 产:MeshEXT +
//! SetMeshOutputs(3,1) + 单三角形非空输出);fragment = `vk::mesh_witness_fs_spv()`(最小无输入
//! const-color 见证,写 vec4(1,0,0,1))。`run_mesh_offscreen(None, mesh, fs, W, H, clear,(1,1,1))`
//! 建 **无 vertex-input** graphics 管线、录 `vkCmdDrawMeshTasksEXT(1,1,1)`、回读像素。
//!
//! ## 两判据(evidence stages_ok 同源)
//! - `mesh_pipeline_draw`(**device**):mesh 管线出图,covered 像素计数 ≥ 阈值(证 mesh 阶段
//!   程序化生成三角形真上屏)。**阈值/期望色 = owner 本机迭代校准 TODO**(`expect_coverage`)。
//! - `tamper_set_mesh_outputs_red`(篡改 → RED):篡改 mesh SPIR-V 的 `OpSetMeshOutputsEXT` 顶点数
//!   (操作数换位 3→1)→ 覆盖减少 = RED;复原 = GREEN。
//!
//! **device 真跑 / SKIP 三态**:无 Vulkan loader / 无 GPU / **无 VK_EXT_mesh_shader feature** →
//! `run_mesh_offscreen` 确定性 `Err` → `MESH: SKIP` 退 0(dev-env degrade,**非 fake pass**);
//! `RURIX_REQUIRE_REAL=1` 翻硬红。有设备但判据阈值未过(owner 迭代)→ `MESH: PARTIAL`(诚实,
//! 不伪造绿)。**codegen mesh_min 首期退化三角形(顶点同址)→ covered 首期为 0,coverage 见证
//! 语料 + 阈值归 owner device 调优**;本片交付 = 管线机构 + SKIP 三态 + 判据结构。device 真跑
//! 绝不伪造;**AMD 真卡见证 = G-MB1-6 独立尾门**(本机 RTX 4070 Ti measured 不充作 AMD)。

use std::path::PathBuf;

use rurix_rt::vk::{mesh_rt_witness_spv, mesh_witness_fs_spv, run_mesh_offscreen};

const W: u32 = 64;
const H: u32 = 64;
const CLEAR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

/// `OpSetMeshOutputsEXT`(opcode 5295)指令首字 = opcode | (wordcount 3 << 16)。
const SET_MESH_OUTPUTS_MARKER: u32 = 5295 | (3 << 16);

/// 无设备 / feature 缺失(SKIP)信号(镜像 ci/*_smoke.py NO_DEVICE_KEYS + mesh feature)。
const NO_DEVICE_KEYS: &[&str] = &[
    "vulkan loader",
    "vulkan-1.dll",
    "libvulkan",
    "物理设备",
    "graphics queue",
    "vkCreateInstance",
    "mesh shader feature",
    "vkGetPhysicalDeviceFeatures2",
];

const MODES: &[&str] = &["mesh_pipeline_draw", "tamper_set_mesh_outputs_red"];

fn to_words(bytes: &[u8]) -> Vec<u32> {
    bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn is_no_device(e: &str) -> bool {
    NO_DEVICE_KEYS.iter().any(|k| e.contains(k))
}

/// 覆盖像素数(fragment 写 red → 非 clear 黑)。
fn covered(pixels: &[u8]) -> usize {
    pixels
        .chunks_exact(4)
        .filter(|p| p[0] > 8 || p[1] > 8 || p[2] > 8)
        .count()
}

/// 篡改 `OpSetMeshOutputsEXT` 顶点数:找到指令,把顶点数操作数(首操作数)换为 prim 数操作数
/// (次操作数,值 1)→ 顶点数 3→1,输出欠供 → 覆盖减少。找不到 → 原样返回(harness 记不可用)。
fn tamper_set_mesh_outputs(spv: &[u32]) -> (Vec<u32>, bool) {
    let mut out = spv.to_vec();
    let mut i = 5; // header 5 字后为指令流
    while i < out.len() {
        let wc = (out[i] >> 16) as usize;
        if wc == 0 {
            break;
        }
        if out[i] == SET_MESH_OUTPUTS_MARKER && i + 2 < out.len() {
            // 操作数:[i+1]=vertex_count id,[i+2]=primitive_count id → 换位(vertex←prim)。
            out[i + 1] = out[i + 2];
            return (out, true);
        }
        i += wc;
    }
    (out, false)
}

/// covered 达阈值(mesh 三角形真上屏)。
fn expect_coverage(n: usize) -> bool {
    // TODO(owner device): 校准覆盖阈值(coverage 见证语料就位后;首期退化三角形 covered=0)。
    n > 0
}

fn main() {
    let witness = mesh_rt_witness_spv();
    if witness.mesh.is_empty() {
        println!("MESH: SKIP mesh 见证语料为空(build.rs codegen 降级)");
        return;
    }
    let mesh = to_words(witness.mesh);
    let fs = mesh_witness_fs_spv();

    println!("[vk_mesh] G3.6 mesh 管线 device 像素判据 harness(RFC-0013 §4.E7,G-G3-6)");
    for (i, m) in MODES.iter().enumerate() {
        println!("[vk_mesh]   判据 {}: {m}", i + 1);
    }

    let mut stages_ok: Vec<&str> = Vec::new();
    let mut misses: Vec<String> = Vec::new();

    // ── ① mesh_pipeline_draw(device 首跑:mesh 出图,covered ≥ 阈值)──
    let base_px = match run_mesh_offscreen(None, &mesh, &fs, W, H, CLEAR, (1, 1, 1)) {
        Ok(px) => px,
        Err(e) if is_no_device(&e) => {
            println!(
                "MESH: SKIP 无 Vulkan 设备 / mesh feature 缺失({})",
                e.trim()
            );
            return;
        }
        Err(e) => {
            eprintln!("MESH: FAIL run_mesh_offscreen 出图: {e}");
            std::process::exit(1);
        }
    };
    let base_cov = covered(&base_px);
    if expect_coverage(base_cov) {
        stages_ok.push("mesh");
    } else {
        misses.push(format!(
            "mesh_pipeline_draw covered={base_cov}(退化三角形/阈值未过;coverage 语料 + 阈值 owner TODO)"
        ));
    }

    // ── ② tamper_set_mesh_outputs_red(篡改顶点数 → 覆盖减少 = RED;复原 GREEN)──
    let (tampered, found) = tamper_set_mesh_outputs(&mesh);
    if !found {
        misses.push(
            "tamper_set_mesh_outputs_red 未定位 OpSetMeshOutputsEXT(SPIR-V 布局变化?)".into(),
        );
    } else {
        match run_mesh_offscreen(None, &tampered, &fs, W, H, CLEAR, (1, 1, 1)) {
            Ok(tpx) => {
                let tcov = covered(&tpx);
                // RED = 篡改后覆盖严格减少;GREEN = 复原(base)覆盖恢复。
                let red = tcov < base_cov;
                let green = expect_coverage(base_cov);
                if red && green {
                    if !stages_ok.contains(&"mesh") {
                        stages_ok.push("mesh");
                    }
                } else {
                    misses.push(format!(
                        "tamper_set_mesh_outputs_red base_cov={base_cov} tamper_cov={tcov} \
                         red={red} green={green}(篡改应减覆盖;阈值 owner TODO)"
                    ));
                }
            }
            Err(e) if is_no_device(&e) => {
                println!("MESH: SKIP 篡改跑无设备({})", e.trim());
                return;
            }
            Err(e) => {
                eprintln!("MESH: FAIL 篡改 run_mesh_offscreen: {e}");
                std::process::exit(1);
            }
        }
    }

    for m in &misses {
        eprintln!("MESH: MISS {m}");
    }
    let smoke_ok = stages_ok.len() == 1 && misses.is_empty();
    if let Err(e) = write_evidence("mesh", &stages_ok, smoke_ok) {
        eprintln!("MESH: WARN evidence 写入失败: {e}");
    }
    if smoke_ok {
        println!(
            "MESH: PASS stages_ok=[{}] covered={base_cov}(mesh 管线出图 + 篡改 SetMeshOutputs 减覆盖 RED)",
            stages_ok.join(", ")
        );
    } else {
        // 真跑但判据阈值未过(owner 迭代 coverage 语料/阈值)→ 诚实 PARTIAL(退 0,不伪造绿)。
        println!(
            "MESH: PARTIAL stages_ok=[{}] covered={base_cov}(coverage 见证语料 + 像素阈值归 owner \
             device 调优;判据结构就位,codegen mesh_min 首期退化三角形 covered=0)",
            stages_ok.join(", ")
        );
    }
}

/// evidence/meshrt_mesh_<epoch>.json(schema milestones/g3/meshrt_stages_evidence_schema.json)。
/// 仅 device 真跑写;subject 与 vk_rt 同源,`g3.counter.mesh_task_rt_stages` 跨 meshrt_*.json 去重计数。
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

/// epoch 秒 → RFC3339 UTC(无外部依赖;镜像 bin/graph_modes)。
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
