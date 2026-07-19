//! G3.4 bindless **device 索引判据 harness**(RXS-0231~0235;RFC-0013 §4.C4;验收门
//! G-G3-4;counter `g3.counter.bindless_descriptor_smoke` ≥1)。镜像 `bin/sampling_modes`
//! 的 device 真跑 / SKIP 三态 + 「篡改→像素变=RED,复原=GREEN」数据流纪律(RXS-0176 IR2)。
//!
//! ## 判据结构(RFC-0013 §4.C4;数值阈值 = **owner 本机迭代填**,TODO)
//!
//! 无界表 `[Texture2D<f32>]` 注册 4 纹理(红/绿/蓝/白,注册序 = 索引),四象限双三角形
//! 逐象限 flat idx = 象限号,片元 `table[nonuniform(idx)].sample(samp, uv)` 动态非均匀
//! 索引采样(`bin` 真调 [`rurix_rt::vk::run_graphics_offscreen_bindless`]:feature chain
//! 四 bit 探测 + UPDATE_AFTER_BIND/PARTIALLY_BOUND + set4 注册序写入 + **push-constant
//! table_len 下发**〔负重点 a:缺注册则 clamp 上界 = 垃圾〕)。三判据:
//!
//! - `quadrant_index_four_color`:四象限采样点像素 == 对应注册纹理主色(四色;
//!   `expect_quadrant` 容差 TODO owner 校准)。
//! - `tamper_register_order_swap`:交换注册序(表序 0↔3)→ 象限 0/3 颜色换位 = RED
//!   (证动态索引**真按注册序命中**,非静默取元素 0——NVIDIA 上丢 NonUniform 常「碰巧
//!   能跑」,负重点 b,故须真验换位);复原注册序 → 与 baseline 逐点复原 = GREEN。
//! - `feature_chain_missing_err`:四 bit(shaderSampledImageArrayNonUniformIndexing /
//!   descriptorBindingSampledImageUpdateAfterBind / descriptorBindingPartiallyBound /
//!   runtimeDescriptorArray)任一缺失 → 确定性 `Err`(RXS-0193 封口,不占 RX 码,无静默
//!   降级)。**本机 4070 Ti 四 bit 全在 → missing 路以 mock 判定函数见证**
//!   ([`check_descriptor_indexing_bits`] 纯函数,与 device 路同一判定点);device run
//!   成功即证 present 路真探测通过。
//!
//! ## device 真跑 / SKIP 三态(RXS-0235 / §4.C4)
//! 无显示 / 无 GPU / 无 Vulkan loader → 首个 bindless run 确定性 `Err` → 打印
//! `BINDLESS_MODES: SKIP` 退 0(dev-env degrade,**非 fake pass**;`ci/bindless_smoke.py`
//! 据此 SKIP 三态,`RURIX_REQUIRE_REAL=1` 翻硬红)。有 GPU:逐判据评判,全过 → 写
//! `evidence/bindless_<epoch>.json`(smoke_ok=true)+ `BINDLESS_MODES: PASS`;判据未过
//! (owner 阈值迭代)→ `BINDLESS_MODES: PARTIAL`(诚实,不伪造绿)。**AMD 真卡见证 =
//! G-MB1-6 独立尾门**(本机 RTX 4070 Ti measured 不充作 AMD)。device 真跑绝不伪造。

use std::path::PathBuf;

use rurix_rt::sampler::{Address, Filter, SamplerDesc};
use rurix_rt::vk::{
    DescriptorIndexingBits, GraphicsResource, TextureData, bindless_shaders_spv,
    check_descriptor_indexing_bits, run_graphics_offscreen_bindless,
};

// ── VkFormat(顶点属性)──────────────────────────────────────────────────────
const FMT_RGBA32F: u32 = 109; // R32G32B32A32_SFLOAT
const FMT_RG32F: u32 = 103; // R32G32_SFLOAT
const FMT_R32UI: u32 = 98; // R32_UINT(flat idx)

const W: u32 = 64;
const H: u32 = 64;
const CLEAR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

/// RGBA8 采样点像素(回读附件逐通道字节)。
type Px = (u8, u8, u8, u8);

/// 无设备(SKIP)信号(镜像 bin/sampling_modes / ci/bindless_smoke.py NO_DEVICE_KEYS)。
const NO_DEVICE_KEYS: &[&str] = &[
    "vulkan loader",
    "vulkan-1.dll",
    "libvulkan",
    "物理设备",
    "graphics queue",
    "vkCreateInstance",
];

/// bindless device 判据模式(RFC-0013 §4.C4;evidence modes_ok enum 同源)。
const MODES: &[&str] = &[
    "quadrant_index_four_color",
    "tamper_register_order_swap",
    "feature_chain_missing_err",
];

fn to_words(bytes: &[u8]) -> Vec<u32> {
    bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn push_f32(buf: &mut Vec<u8>, v: f32) {
    buf.extend_from_slice(&v.to_le_bytes());
}
fn push_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

/// 顶点 stride:pos vec4(16)+ uv vec2(8)+ idx u32(4)。
const QUAD_STRIDE: u32 = 28;

/// 四象限双三角形顶点(24 顶点):象限 q 的全部顶点 flat idx = q(= 无界表注册序索引),
/// uv 在象限内 0..1。Vulkan clip:(-1,-1) = framebuffer 左上,+y 向下 → 象限序
/// 0=左上 / 1=右上 / 2=左下 / 3=右下(与 `sample_points` 同序)。
fn quadrant_verts() -> Vec<u8> {
    let mut v = Vec::with_capacity(24 * QUAD_STRIDE as usize);
    // (x0, y0, x1, y1) per 象限。
    let rects: [(f32, f32, f32, f32); 4] = [
        (-1.0, -1.0, 0.0, 0.0), // q0 左上
        (0.0, -1.0, 1.0, 0.0),  // q1 右上
        (-1.0, 0.0, 0.0, 1.0),  // q2 左下
        (0.0, 0.0, 1.0, 1.0),   // q3 右下
    ];
    for (q, &(x0, y0, x1, y1)) in rects.iter().enumerate() {
        // 两三角形:(x0,y0)(x1,y0)(x0,y1) + (x1,y0)(x1,y1)(x0,y1);uv 对应角点。
        let corners: [((f32, f32), (f32, f32)); 6] = [
            ((x0, y0), (0.0, 0.0)),
            ((x1, y0), (1.0, 0.0)),
            ((x0, y1), (0.0, 1.0)),
            ((x1, y0), (1.0, 0.0)),
            ((x1, y1), (1.0, 1.0)),
            ((x0, y1), (0.0, 1.0)),
        ];
        for ((x, y), (u, w)) in corners {
            push_f32(&mut v, x);
            push_f32(&mut v, y);
            push_f32(&mut v, 0.0);
            push_f32(&mut v, 1.0);
            push_f32(&mut v, u);
            push_f32(&mut v, w);
            push_u32(&mut v, q as u32);
        }
    }
    v
}

fn quad_attrs() -> [(u32, u32, u32); 3] {
    [(0, FMT_RGBA32F, 0), (1, FMT_RG32F, 16), (2, FMT_R32UI, 24)]
}

/// 注册纹理主色(注册序 = 索引;四象限四色判据基准)。
const COLORS: [[u8; 4]; 4] = [
    [255, 0, 0, 255],     // T0 红
    [0, 255, 0, 255],     // T1 绿
    [0, 0, 255, 255],     // T2 蓝
    [255, 255, 255, 255], // T3 白
];

/// 单色 4×4 单层纹理。
fn solid_tex(rgba: [u8; 4]) -> GraphicsResource {
    let mut lvl = Vec::with_capacity(64);
    for _ in 0..16 {
        lvl.extend_from_slice(&rgba);
    }
    GraphicsResource::Texture2D {
        width: 4,
        height: 4,
        data: TextureData::Rgba8(vec![lvl]),
    }
}

/// 按注册序装配无界表(`order[i]` = 第 i 个注册位放哪个色;identity = [0,1,2,3])。
fn table(order: [usize; 4]) -> Vec<GraphicsResource> {
    order.map(|c| solid_tex(COLORS[c])).to_vec()
}

fn nearest_clamp() -> GraphicsResource {
    GraphicsResource::Sampler(SamplerDesc {
        filter: Filter::Nearest,
        address: Address::Clamp,
        ..SamplerDesc::default()
    })
}

/// 四象限采样点(象限中心;序同 `quadrant_verts` 象限序)。
fn sample_points() -> [(u32, u32); 4] {
    [
        (W / 4, H / 4),         // q0 左上
        (3 * W / 4, H / 4),     // q1 右上
        (W / 4, 3 * H / 4),     // q2 左下
        (3 * W / 4, 3 * H / 4), // q3 右下
    ]
}

fn px_at(p: &[u8], x: u32, y: u32) -> Px {
    let o = ((y * W + x) * 4) as usize;
    (p[o], p[o + 1], p[o + 2], p[o + 3])
}

/// 无设备信号检测(SKIP vs FAIL 分诊)。
fn is_no_device(e: &str) -> bool {
    NO_DEVICE_KEYS.iter().any(|k| e.contains(k))
}

/// device 判据谓词:象限采样点逼近注册纹理主色。
fn expect_quadrant(p: Px, rgba: [u8; 4]) -> bool {
    // TODO(owner device): 校准容差(nearest + 单色纹理应逼近精确值;首期 ±24)。
    let near = |a: u8, b: u8| (a as i32 - b as i32).abs() <= 24;
    near(p.0, rgba[0]) && near(p.1, rgba[1]) && near(p.2, rgba[2])
}

/// 渲染一遍四象限 bindless 帧 → 四象限采样点像素。Err 透传(含无设备信号)。
fn render_quadrants(vs: &[u32], fs: &[u32], order: [usize; 4]) -> Result<[Px; 4], String> {
    let verts = quadrant_verts();
    let px = run_graphics_offscreen_bindless(
        vs,
        fs,
        &verts,
        QUAD_STRIDE,
        &quad_attrs(),
        W,
        H,
        CLEAR,
        &[nearest_clamp()],
        &table(order),
    )?;
    let pts = sample_points();
    Ok([
        px_at(&px, pts[0].0, pts[0].1),
        px_at(&px, pts[1].0, pts[1].1),
        px_at(&px, pts[2].0, pts[2].1),
        px_at(&px, pts[3].0, pts[3].1),
    ])
}

fn main() {
    let sh = bindless_shaders_spv();
    // build.rs codegen 降级(极少)→ 空切片,消费侧 SKIP(对齐既有降级纪律,非 fake)。
    if sh.quadrant_vs.is_empty() || sh.sample_fs.is_empty() {
        println!("BINDLESS_MODES: SKIP bindless 模式着色器为空(build.rs codegen 降级)");
        return;
    }
    let vs = to_words(sh.quadrant_vs);
    let fs = to_words(sh.sample_fs);

    println!("[bindless_modes] G3.4 bindless device 判据 harness(RFC-0013 §4.C4,G-G3-4)");
    for (i, m) in MODES.iter().enumerate() {
        println!("[bindless_modes]   判据 {}: {m}", i + 1);
    }

    let mut modes_ok: Vec<&str> = Vec::new();
    let mut misses: Vec<String> = Vec::new();

    // ── ① quadrant_index_four_color(baseline:注册序 identity → 四象限四色)──
    let baseline = match render_quadrants(&vs, &fs, [0, 1, 2, 3]) {
        Ok(p) => p,
        Err(e) if is_no_device(&e) => {
            println!("BINDLESS_MODES: SKIP 无 Vulkan 设备/loader({})", e.trim());
            return;
        }
        Err(e) => {
            eprintln!("BINDLESS_MODES: FAIL baseline 渲染: {e}");
            std::process::exit(1);
        }
    };
    let four_color = (0..4).all(|q| expect_quadrant(baseline[q], COLORS[q]));
    if four_color {
        modes_ok.push(MODES[0]);
    } else {
        misses.push(format!(
            "quadrant_index_four_color 采样点 {baseline:?} 未满足四色谓词(owner 校准容差)"
        ));
    }

    // ── ② tamper_register_order_swap(注册序 0↔3 → 象限 0/3 换位 RED;复原 GREEN)──
    match (
        render_quadrants(&vs, &fs, [3, 1, 2, 0]),
        render_quadrants(&vs, &fs, [0, 1, 2, 3]),
    ) {
        (Ok(swapped), Ok(restored)) => {
            // RED:被换位的注册位对应象限像素必变(证索引真按注册序命中,非静默元素 0)。
            let red = swapped[0] != baseline[0] && swapped[3] != baseline[3];
            // 未换位象限不应受牵连(1/2 位注册不动)。
            let untouched = swapped[1] == baseline[1] && swapped[2] == baseline[2];
            // GREEN:复原注册序 → 四点逐点复原。
            let green = restored == baseline;
            if red && untouched && green {
                modes_ok.push(MODES[1]);
            } else {
                misses.push(format!(
                    "tamper_register_order_swap red={red} untouched={untouched} green={green} \
                     (swapped={swapped:?} restored={restored:?} baseline={baseline:?})"
                ));
            }
        }
        (Err(e), _) | (_, Err(e)) if is_no_device(&e) => {
            println!("BINDLESS_MODES: SKIP 无 Vulkan 设备/loader({})", e.trim());
            return;
        }
        (Err(e), _) | (_, Err(e)) => {
            eprintln!("BINDLESS_MODES: FAIL 篡改/复原渲染: {e}");
            std::process::exit(1);
        }
    }

    // ── ③ feature_chain_missing_err(missing 路 mock + device 路真探测已通过)──
    // 本机四 bit 全在(device run 成功 ⇒ run_graphics_offscreen_bindless 内 feature chain
    // 探测真通过);missing 路无真设备可跑 → 以同一判定纯函数 mock 见证确定性 Err
    // (不伪造缺 feature 设备)。
    {
        let mut missing = DescriptorIndexingBits::all_present();
        missing.runtime_descriptor_array = false;
        let mock_err = check_descriptor_indexing_bits(&missing);
        let all_ok = check_descriptor_indexing_bits(&DescriptorIndexingBits::all_present());
        match (&mock_err, &all_ok) {
            (Err(e), Ok(())) if e.contains("runtimeDescriptorArray") => {
                modes_ok.push(MODES[2]);
            }
            _ => misses.push(format!(
                "feature_chain_missing_err mock 判定异常: missing={mock_err:?} all={all_ok:?}"
            )),
        }
    }

    // ── 汇总 + evidence ───────────────────────────────────────────────────────
    for m in &misses {
        eprintln!("BINDLESS_MODES: MISS {m}");
    }
    let n = modes_ok.len();
    let smoke_ok = n == MODES.len();
    if let Err(e) = write_evidence(&modes_ok, smoke_ok) {
        eprintln!("BINDLESS_MODES: WARN evidence 写入失败: {e}");
    }
    if smoke_ok {
        println!(
            "BINDLESS_MODES: PASS modes_ok={n} [{}](四象限动态索引四色 + 篡改注册序换位 RED/复原 \
             GREEN + feature chain 确定性 Err)",
            modes_ok.join(", ")
        );
    } else {
        // 判据未过 = owner 阈值迭代未竟,非硬红(退 0,misses 已列供校准)——不伪造绿。
        println!(
            "BINDLESS_MODES: PARTIAL {n}/{} 模式过 [{}](owner 迭代 expect_* 阈值/采样点)",
            MODES.len(),
            modes_ok.join(", ")
        );
    }
}

/// evidence/bindless_<epoch>.json(schema milestones/g3/bindless_descriptor_smoke_evidence_schema.json)。
/// 仅 device 真跑写(本 harness 有 GPU 时);smoke_ok = 三判据全过。
fn write_evidence(modes_ok: &[&str], smoke_ok: bool) -> std::io::Result<()> {
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
    let list = modes_ok
        .iter()
        .map(|m| format!("\"{m}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let json = format!(
        "{{\n  \"schema_version\": 1,\n  \"subject\": \"bindless_descriptor_smoke\",\n  \
         \"milestone\": \"g3.4\",\n  \"smoke_ok\": {smoke_ok},\n  \"modes_ok\": [{list}],\n  \
         \"num_textures\": 4,\n  \"adapter\": \"{adapter}\",\n  \"run_url\": \"{run_url}\",\n  \
         \"timestamp\": \"{ts}\"\n}}\n"
    );
    let path = dir.join(format!("bindless_{stamp}.json"));
    std::fs::write(path, json)
}

/// epoch 秒 → RFC3339 UTC(schema `format: date-time`;无外部依赖,镜像 sampling_modes)。
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
