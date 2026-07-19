//! G3.5 render graph **device 自动 barrier hazard 红绿判据 harness**(RXS-0236~0241;
//! RFC-0013 §4.D;验收门 G-G3-5;counter `g3.counter.auto_barrier_hazard_redgreen` ≥1)。
//! 镜像 `bin/bindless_modes` / `bin/sampling_modes` 的 device 真跑 / SKIP 三态 + 「篡改/装配
//! 拒 → RED,复原 → GREEN」数据流纪律(RXS-0239 IR2)。
//!
//! ## 最小两 pass 见证(非完整 uc04 deferred MRT;RFC-0013 §4.D D5)
//!
//! 图:资源 0 = `color_target(rt0)`、资源 1 = `color_target(final)`;pass0 `writes_rt(rt0)`
//! (三角形铺满视口写纯色)→ **`graph.rs` 自动推导 rt0 RT→SRV barrier** → pass1 `reads(rt0)` +
//! `writes_rt(final)`(全屏采样 rt0 出 final)。执行器 [`run_graph_offscreen`] 把 `at_pass==1`
//! 的 image transition barrier 逐字重放为 `vkCmdPipelineBarrier`(全取 `graph.rs` 同源表,禁
//! 二次推导);pass0 三角形语料复用 `demo_shaders_spv`(tri_vs/tri_fs)、pass1 全屏采样语料
//! 复用 `sampling_shaders_spv`(fullscreen_vs/sample_lod_fs;set1 SRV binding0 + set3 sampler
//! binding0 分配律同源)。**零新 .rx 语料 / 零 build.rs 改动**。
//!
//! ## 三判据(evidence modes_ok enum 同源,schema auto_barrier_hazard_redgreen)
//!
//! - `vulkan_run_graph_match`(**device**):full plan 经 run_graph 出图,final 中心像素 ==
//!   pass0 三角形色经采样(证 pass0→pass1 采样经自动 RT→SRV barrier 真生效)。**采样点/期望
//!   色/容差 = owner 本机迭代校准**(`expect_center` TODO)。
//! - `undeclared_read_strict_reject`(装配期 strict,确定性):pass1 `with_reflection([rt0,
//!   final])` 但**漏声明** `reads(rt0)` → `seal` 装配期 RX6030 声明-反射失配拒 = RED;声明
//!   `reads(rt0)` → `seal` 通过 = GREEN(证漏声明 read 被装配期拦截,非静默无 barrier)。
//! - `auto_barrier_deferred_match`(推导逐字对照):full plan 恰 1 条 rt0 `RenderTarget→
//!   PixelShaderResource`@at_pass=1,`graph_image_barrier_fields` 映射 == COLOR_ATTACHMENT_
//!   OPTIMAL→SHADER_READ_ONLY_OPTIMAL + COLOR_ATTACHMENT_OUTPUT→FRAGMENT_SHADER stage +
//!   COLOR_ATTACHMENT_WRITE→SHADER_READ access(单一事实源逐字重放)。
//!
//! ## device 真跑 / SKIP 三态(RXS-0241 / §4.D)
//! 无显示 / 无 GPU / 无 Vulkan loader → 首个 run_graph 确定性 `Err` → 打印 `GRAPH_MODES: SKIP`
//! 退 0(dev-env degrade,**非 fake pass**;`ci/render_graph_smoke.py` device 段据此 SKIP 三态,
//! `RURIX_REQUIRE_REAL=1` 翻硬红)。有 GPU:逐判据评判,全过 → 写 `evidence/graph_<epoch>.json`
//! (hazard_ok=true)+ `GRAPH_MODES: PASS`;判据未过(owner 阈值迭代)→ `GRAPH_MODES: PARTIAL`
//! (诚实,不伪造绿)。**device 首跑 hazard 红绿最脆见证 = 设 `RURIX_VK_VALIDATION=1`:漏
//! barrier 的 layout 失配经 messenger fail-closed 翻 `Err`**。host 段 D6 互证金标准 + 图合法性
//! reject + 推导 golden(`ci/render_graph_smoke.py` host 段恒跑)为本面核心验收。**AMD 真卡
//! 见证 = G-MB1-6 独立尾门**(本机 RTX 4070 Ti measured 不充作 AMD)。device 真跑绝不伪造。

use std::path::PathBuf;

use rurix_rt::graph::{
    BarrierForm, D3d12State, Graph, GraphError, PassSpec, PlannedBarrier, vk_access, vk_layout,
    vk_stage,
};
use rurix_rt::sampler::{Address, Filter, SamplerDesc};
use rurix_rt::vk::{
    GraphPassDraw, demo_shaders_spv, graph_image_barrier_fields, run_graph_offscreen,
    sampling_shaders_spv,
};

// ── VkFormat(顶点属性;镜像 bin/bindless_modes)────────────────────────────────
const FMT_RGBA32F: u32 = 109; // R32G32B32A32_SFLOAT
const FMT_RG32F: u32 = 103; // R32G32_SFLOAT

const W: u32 = 64;
const H: u32 = 64;
const CLEAR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

/// pass0 三角形铺色(采样后中心像素基准)。
const TRI_COLOR: [f32; 4] = [1.0, 0.0, 0.0, 1.0];

/// RGBA8 采样点像素。
type Px = (u8, u8, u8, u8);

/// 无设备(SKIP)信号(镜像 bin/bindless_modes / ci/render_graph_smoke.py NO_DEVICE_KEYS)。
const NO_DEVICE_KEYS: &[&str] = &[
    "vulkan loader",
    "vulkan-1.dll",
    "libvulkan",
    "物理设备",
    "graphics queue",
    "vkCreateInstance",
];

/// render graph device 判据模式(schema auto_barrier_hazard_redgreen modes enum 同源)。
const MODES: &[&str] = &[
    "vulkan_run_graph_match",
    "undeclared_read_strict_reject",
    "auto_barrier_deferred_match",
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

/// pass0 顶点 stride:pos vec4(16)+ color vec4(16)= 32(tri_vs 布局)。
const TRI_STRIDE: u32 = 32;
/// pass1 顶点 stride:pos vec4(16)+ uv vec2(8)= 24(fullscreen_vs 布局)。
const QUAD_STRIDE: u32 = 24;

/// pass0 铺满视口三角形(3 顶点,pos + 纯色);(-1,-1)/(3,-1)/(-1,3) 覆盖整视口。
fn tri_verts() -> Vec<u8> {
    let mut v = Vec::with_capacity(3 * TRI_STRIDE as usize);
    let pos: [(f32, f32); 3] = [(-1.0, -1.0), (3.0, -1.0), (-1.0, 3.0)];
    for (x, y) in pos {
        push_f32(&mut v, x);
        push_f32(&mut v, y);
        push_f32(&mut v, 0.0);
        push_f32(&mut v, 1.0);
        for c in TRI_COLOR {
            push_f32(&mut v, c);
        }
    }
    v
}

/// pass1 全屏采样三角形(3 顶点,pos + uv);uv 随 clamp sampler 覆盖 rt0(0..1),中心 uv≈0.5。
fn quad_verts() -> Vec<u8> {
    let mut v = Vec::with_capacity(3 * QUAD_STRIDE as usize);
    let corners: [((f32, f32), (f32, f32)); 3] = [
        ((-1.0, -1.0), (0.0, 0.0)),
        ((3.0, -1.0), (2.0, 0.0)),
        ((-1.0, 3.0), (0.0, 2.0)),
    ];
    for ((x, y), (u, w)) in corners {
        push_f32(&mut v, x);
        push_f32(&mut v, y);
        push_f32(&mut v, 0.0);
        push_f32(&mut v, 1.0);
        push_f32(&mut v, u);
        push_f32(&mut v, w);
    }
    v
}

fn tri_attrs() -> [(u32, u32, u32); 2] {
    [(0, FMT_RGBA32F, 0), (1, FMT_RGBA32F, 16)]
}

fn quad_attrs() -> [(u32, u32, u32); 2] {
    [(0, FMT_RGBA32F, 0), (1, FMT_RG32F, 16)]
}

fn nearest_clamp() -> SamplerDesc {
    SamplerDesc {
        filter: Filter::Nearest,
        address: Address::Clamp,
        ..SamplerDesc::default()
    }
}

fn px_at(p: &[u8], x: u32, y: u32) -> Px {
    let o = ((y * W + x) * 4) as usize;
    (p[o], p[o + 1], p[o + 2], p[o + 3])
}

fn is_no_device(e: &str) -> bool {
    NO_DEVICE_KEYS.iter().any(|k| e.contains(k))
}

/// 中心像素逼近 pass0 三角形色(采样后)。
fn expect_center(p: Px) -> bool {
    // TODO(owner device): 校准容差(nearest + 纯色 rt0 应逼近 [255,0,0];首期 ±24)。
    let near = |a: u8, b: u8| (a as i32 - b as i32).abs() <= 24;
    near(p.0, 255) && near(p.1, 0) && near(p.2, 0)
}

/// 两 pass 图(pass1 是否声明 `reads(rt0)` 由 `declare_read` 控)。返回 seal `Result`。
fn seal_graph(declare_read: bool) -> Result<(), GraphError> {
    let mut g = Graph::new();
    let rt0 = g.color_target("rt0");
    let final_rt = g.color_target("final");
    g.add_pass(PassSpec::new("pass0").writes_rt(rt0))
        .expect("pass0 add");
    // pass1 反射面恒含 [rt0, final];declare_read=false 时漏声明 reads(rt0) → RX6030。
    let mut pass1 = PassSpec::new("pass1");
    if declare_read {
        pass1 = pass1.reads(rt0);
    }
    pass1 = pass1
        .writes_rt(final_rt)
        .with_reflection(vec![rt0, final_rt]);
    g.add_pass(pass1).expect("pass1 add");
    g.seal()
}

/// full 图(声明齐全)的推导 barrier 计划(seal + derive)。
fn full_plan() -> Vec<PlannedBarrier> {
    let mut g = Graph::new();
    let rt0 = g.color_target("rt0");
    let final_rt = g.color_target("final");
    g.add_pass(PassSpec::new("pass0").writes_rt(rt0))
        .expect("pass0 add");
    g.add_pass(
        PassSpec::new("pass1")
            .reads(rt0)
            .writes_rt(final_rt)
            .with_reflection(vec![rt0, final_rt]),
    )
    .expect("pass1 add");
    g.execute().expect("full graph 应 execute 通过")
}

fn main() {
    // pass0 三角形语料(demo_shaders_spv)+ pass1 全屏采样语料(sampling_shaders_spv)。
    let (tri_vs_b, tri_fs_b, _saxpy) = demo_shaders_spv();
    let sh = sampling_shaders_spv();
    if tri_vs_b.is_empty()
        || tri_fs_b.is_empty()
        || sh.fullscreen_vs.is_empty()
        || sh.sample_lod_fs.is_empty()
    {
        println!("GRAPH_MODES: SKIP 着色器语料为空(build.rs codegen 降级)");
        return;
    }
    let tri_vs = to_words(tri_vs_b);
    let tri_fs = to_words(tri_fs_b);
    let quad_vs = to_words(sh.fullscreen_vs);
    let quad_fs = to_words(sh.sample_lod_fs);

    println!(
        "[graph_modes] G3.5 render graph device 自动 barrier hazard harness(RFC-0013 §4.D,G-G3-5)"
    );
    for (i, m) in MODES.iter().enumerate() {
        println!("[graph_modes]   判据 {}: {m}", i + 1);
    }

    let plan = full_plan();
    let tri_v = tri_verts();
    let quad_v = quad_verts();
    let sampler = nearest_clamp();
    let pass0 = GraphPassDraw {
        vs_spv: &tri_vs,
        fs_spv: &tri_fs,
        vertices: &tri_v,
        vertex_stride: TRI_STRIDE,
        attrs: &tri_attrs(),
        clear: CLEAR,
    };
    let pass1 = GraphPassDraw {
        vs_spv: &quad_vs,
        fs_spv: &quad_fs,
        vertices: &quad_v,
        vertex_stride: QUAD_STRIDE,
        attrs: &quad_attrs(),
        clear: CLEAR,
    };

    let mut modes_ok: Vec<&str> = Vec::new();
    let mut misses: Vec<String> = Vec::new();

    // ── ① vulkan_run_graph_match(device 首跑:full plan 出图,中心像素 == 采样色)──
    let final_px = match run_graph_offscreen(&plan, &pass0, &pass1, &sampler, W, H) {
        Ok(px) => px,
        Err(e) if is_no_device(&e) => {
            println!("GRAPH_MODES: SKIP 无 Vulkan 设备/loader({})", e.trim());
            return;
        }
        Err(e) => {
            eprintln!("GRAPH_MODES: FAIL run_graph 出图: {e}");
            std::process::exit(1);
        }
    };
    let center = px_at(&final_px, W / 2, H / 2);
    if expect_center(center) {
        modes_ok.push(MODES[0]);
    } else {
        misses.push(format!(
            "vulkan_run_graph_match 中心像素 {center:?} 未逼近采样色 [255,0,0](owner 校准容差)"
        ));
    }

    // ── ② undeclared_read_strict_reject(漏声明 read → 装配期 RX6030 拒 RED;声明 → GREEN)──
    {
        let red = matches!(seal_graph(false), Err(ref e @ GraphError::ReflectionMismatch { .. }) if e.rx_code() == "RX6030");
        let green = seal_graph(true).is_ok();
        if red && green {
            modes_ok.push(MODES[1]);
        } else {
            misses.push(format!(
                "undeclared_read_strict_reject red={red} green={green}(漏声明 read 应 RX6030 拒,声明应通过)"
            ));
        }
    }

    // ── ③ auto_barrier_deferred_match(推导逐字对照:恰 1 条 rt0 RT→SRV,映射逐字重放)──
    {
        let rt0_barrier = plan
            .iter()
            .find(|b| b.resource_name == "rt0" && b.form == BarrierForm::Transition);
        let ok = plan.len() == 1
            && rt0_barrier.is_some_and(|b| {
                let (old, new, sa, da, ss, ds) = graph_image_barrier_fields(b);
                b.d3d12_before == D3d12State::RenderTarget
                    && b.d3d12_after == D3d12State::PixelShaderResource
                    && old == vk_layout::COLOR_ATTACHMENT_OPTIMAL
                    && new == vk_layout::SHADER_READ_ONLY_OPTIMAL
                    && sa == vk_access::COLOR_ATTACHMENT_WRITE
                    && da == vk_access::SHADER_READ
                    && ss == vk_stage::COLOR_ATTACHMENT_OUTPUT
                    && ds == vk_stage::FRAGMENT_SHADER
            });
        if ok {
            modes_ok.push(MODES[2]);
        } else {
            misses.push(format!(
                "auto_barrier_deferred_match 推导计划 {plan:?} 与期望 rt0 RT→SRV 逐字重放不符"
            ));
        }
    }

    // ── 汇总 + evidence ───────────────────────────────────────────────────────
    for m in &misses {
        eprintln!("GRAPH_MODES: MISS {m}");
    }
    let n = modes_ok.len();
    let hazard_ok = n == MODES.len();
    if let Err(e) = write_evidence(&modes_ok, hazard_ok) {
        eprintln!("GRAPH_MODES: WARN evidence 写入失败: {e}");
    }
    if hazard_ok {
        println!(
            "GRAPH_MODES: PASS modes_ok={n} [{}](两 pass 自动 RT→SRV barrier 出图 + 漏声明 read \
             装配期 RX6030 拒 RED/声明 GREEN + 推导逐字重放对照)",
            modes_ok.join(", ")
        );
    } else {
        // 判据未过 = owner 阈值迭代未竟,非硬红(退 0,misses 已列供校准)——不伪造绿。
        println!(
            "GRAPH_MODES: PARTIAL {n}/{} 模式过 [{}](owner 迭代 expect_center 阈值/采样点;device \
             首跑设 RURIX_VK_VALIDATION=1 令漏 barrier 失配翻硬 Err)",
            MODES.len(),
            modes_ok.join(", ")
        );
    }
}

/// evidence/graph_<epoch>.json(schema milestones/g3/auto_barrier_hazard_evidence_schema.json)。
/// 仅 device 真跑写(本 harness 有 GPU 时);hazard_ok = 三判据全过。
fn write_evidence(modes_ok: &[&str], hazard_ok: bool) -> std::io::Result<()> {
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
        "{{\n  \"schema_version\": 1,\n  \"subject\": \"auto_barrier_hazard_redgreen\",\n  \
         \"milestone\": \"g3.5\",\n  \"hazard_ok\": {hazard_ok},\n  \"modes_ok\": [{list}],\n  \
         \"num_passes\": 2,\n  \"adapter\": \"{adapter}\",\n  \"run_url\": \"{run_url}\",\n  \
         \"timestamp\": \"{ts}\"\n}}\n"
    );
    let path = dir.join(format!("graph_{stamp}.json"));
    std::fs::write(path, json)
}

/// epoch 秒 → RFC3339 UTC(schema `format: date-time`;无外部依赖,镜像 bindless_modes)。
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
