//! G3.3 PR-S3 采样超集 **device 数值判据 harness**(RXS-0223~0230;RFC-0013 §4.B8;验收门
//! G-G3-3;counter `g3.counter.sampling_superset_modes` ≥6)。
//!
//! 每模式:descriptor-消费着色器(`vk::sampling_shaders_spv`,build.rs 经
//! `emit_spirv_body_vulkan` Vk-native set-per-class 绑定装饰产)配 per-dispatch 资源(mip
//! 纹理 / `SamplerDesc` 状态 / storage image)→ `run_graphics_offscreen_v2[_readback]` 真渲染
//! → 采样点像素读取 → **判据 + 篡改红绿**(数据流纪律 RXS-0176 IR2:篡改令采样点像素变 = RED,
//! 复原 = GREEN;无篡改敏感者 = 非真采样,不计入 modes_ok)。
//!
//! ## device 真跑 / SKIP 三态(RFC-0013 §4.B8 / RXS-0230 L4)
//! 无显示 / 无 GPU / 无 Vulkan loader → 首个 `run_graphics_offscreen_v2` 返回确定性 `Err`
//! (loader/物理设备缺失)→ 打印 `SAMPLING_MODES: SKIP` 退 0(dev-env degrade,**非 fake
//! pass**;`ci/sampling_superset_smoke.py` 据此 SKIP 三态,`RURIX_REQUIRE_REAL=1` 翻硬红)。
//! 有 GPU:逐模式评判,≥6 模式过 → 写 `evidence/sampling_superset_<date>.json`(modes_ok /
//! num_modes / adapter / backend_consistency),打印 `SAMPLING_MODES: ok modes_ok=N`。
//!
//! ## device 判据阈值 = **owner 本机迭代填**(TODO,见各 `expect_*` 谓词)
//! 下列 `expect_*` 谓词 + 采样点坐标 + 篡改集为**预测基线**(agent 无 GPU,只到编译 +
//! spirv-val)。owner 首跑按实测像素校准阈值/容差;线性过滤精度为实现近似,逐位不承诺
//! (§4.B8 诚实边界:nearest 逐位 / linear 容差)。**AMD 真卡见证 = G-MB1-6 独立尾门**
//! (本机 RTX 4070 Ti measured 不充作 AMD)。device 真跑绝不伪造。

use std::path::PathBuf;

use rurix_rt::sampler::{Address, Compare, Filter, SamplerDesc};
use rurix_rt::vk::{
    GraphicsResource, StorageFormat, TextureData, run_graphics_offscreen_v2,
    run_graphics_offscreen_v2_readback, sampling_shaders_spv,
};

// ── VkFormat(顶点属性)──────────────────────────────────────────────────────
const FMT_RGBA32F: u32 = 109; // R32G32B32A32_SFLOAT
const FMT_RG32F: u32 = 103; // R32G32_SFLOAT
const FMT_RG32UI: u32 = 101; // R32G32_UINT

const W: u32 = 64;
const H: u32 = 64;
const CLEAR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

/// RGBA8 采样点像素(回读附件逐通道字节)。
type Px = (u8, u8, u8, u8);

/// 无设备(SKIP)信号:run_graphics_offscreen* 缺 Vulkan 运行时的确定性 Err 串
/// (镜像 ci/sampling_superset_smoke.py NO_DEVICE_KEYS)。
const NO_DEVICE_KEYS: &[&str] = &[
    "vulkan loader",
    "vulkan-1.dll",
    "libvulkan",
    "物理设备",
    "graphics queue",
    "vkCreateInstance",
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

/// 全屏三角形(镜像单遍 fullscreen tri)顶点:pos(vec4 clip)+ uv(vec2)。uv 在视口内
/// 插值 0..`uv_span`(clip (1,1) 处 = uv_span/2 处的一半…);`uv_span` = 视口对角 uv 跨度
/// (uv_span=2 → 视口内 uv∈[0,1];uv_span=4 → uv∈[0,2],右上像素 uv>1,供 wrap-vs-clamp)。
const FS_STRIDE: u32 = 24;
fn fullscreen_verts(uv_span: f32) -> Vec<u8> {
    let mut v = Vec::with_capacity(3 * FS_STRIDE as usize);
    // v0 左下 clip(-1,-1) uv(0,0)
    push_f32(&mut v, -1.0);
    push_f32(&mut v, -1.0);
    push_f32(&mut v, 0.0);
    push_f32(&mut v, 1.0);
    push_f32(&mut v, 0.0);
    push_f32(&mut v, 0.0);
    // v1 右下 clip(3,-1) uv(uv_span,0)
    push_f32(&mut v, 3.0);
    push_f32(&mut v, -1.0);
    push_f32(&mut v, 0.0);
    push_f32(&mut v, 1.0);
    push_f32(&mut v, uv_span);
    push_f32(&mut v, 0.0);
    // v2 左上 clip(-1,3) uv(0,uv_span)
    push_f32(&mut v, -1.0);
    push_f32(&mut v, 3.0);
    push_f32(&mut v, 0.0);
    push_f32(&mut v, 1.0);
    push_f32(&mut v, 0.0);
    push_f32(&mut v, uv_span);
    v
}
fn fs_attrs() -> [(u32, u32, u32); 2] {
    [(0, FMT_RGBA32F, 0), (1, FMT_RG32F, 16)]
}

/// 整型取址全屏三角形:pos(vec4)+ px(uvec2 flat)+ val(vec4 flat)。px/val 三顶点同值
/// (flat → 全三角形常量;load OOB 钳制 / storage 唯一写者〔identity 见 TODO〕)。
const FETCH_STRIDE: u32 = 40;
fn fetch_verts(px: [u32; 2], val: [f32; 4]) -> Vec<u8> {
    let mut v = Vec::with_capacity(3 * FETCH_STRIDE as usize);
    let clip = [[-1.0f32, -1.0], [3.0, -1.0], [-1.0, 3.0]];
    for c in clip {
        push_f32(&mut v, c[0]);
        push_f32(&mut v, c[1]);
        push_f32(&mut v, 0.0);
        push_f32(&mut v, 1.0);
        push_u32(&mut v, px[0]);
        push_u32(&mut v, px[1]);
        for f in val {
            push_f32(&mut v, f);
        }
    }
    v
}
fn fetch_attrs() -> [(u32, u32, u32); 3] {
    [
        (0, FMT_RGBA32F, 0),
        (1, FMT_RG32UI, 16),
        (2, FMT_RGBA32F, 24),
    ]
}

/// 单色 RGBA8 层(w×h×4)。
fn solid(w: u32, h: u32, rgba: [u8; 4]) -> Vec<u8> {
    let mut v = Vec::with_capacity((w * h * 4) as usize);
    for _ in 0..w * h {
        v.extend_from_slice(&rgba);
    }
    v
}

/// 4 纹素 2×2 RGBA8(行优先:(0,0)(1,0)/(0,1)(1,1))。
fn tex2x2(t: [[u8; 4]; 4]) -> Vec<u8> {
    let mut v = Vec::with_capacity(16);
    for texel in t {
        v.extend_from_slice(&texel);
    }
    v
}

fn px_at(p: &[u8], x: u32, y: u32) -> Px {
    let o = ((y * W + x) * 4) as usize;
    (p[o], p[o + 1], p[o + 2], p[o + 3])
}

/// 无设备信号检测(SKIP vs FAIL 分诊)。
fn is_no_device(e: &str) -> bool {
    NO_DEVICE_KEYS.iter().any(|k| e.contains(k))
}

/// 模式评判结果。
enum Outcome {
    /// device 判据真过(计入 modes_ok)。
    Pass,
    /// 判据未过(device 真跑但值不符 / 篡改不敏感;不计入,非硬红——owner 迭代阈值)。
    Miss(String),
    /// 无 Vulkan 设备(SKIP 三态,退 0)。
    NoDevice(String),
    /// device 真跑内部错误(硬红)。
    Err(String),
}

// ── uv 模式渲染 + 基线/篡改采样点评判 ────────────────────────────────────────
/// 渲染一遍 uv 模式(全屏 tri + fs + 资源)→ 采样点像素。Err 透传(含无设备信号)。
fn render_uv(
    fs: &[u32],
    vs: &[u32],
    uv_span: f32,
    resources: &[GraphicsResource],
    sample: (u32, u32),
) -> Result<Px, String> {
    let verts = fullscreen_verts(uv_span);
    let px = run_graphics_offscreen_v2(
        vs,
        fs,
        &verts,
        FS_STRIDE,
        &fs_attrs(),
        W,
        H,
        CLEAR,
        resources,
    )?;
    Ok(px_at(&px, sample.0, sample.1))
}

/// uv 模式统一评判:pass = expect(baseline 采样点) && (篡改采样点 != baseline)。
#[allow(clippy::too_many_arguments)]
fn eval_uv(
    name: &str,
    fs: &[u32],
    vs: &[u32],
    uv_span: f32,
    sample: (u32, u32),
    base: &[GraphicsResource],
    tampered: &[GraphicsResource],
    expect: fn(Px) -> bool,
) -> Outcome {
    let b = match render_uv(fs, vs, uv_span, base, sample) {
        Ok(p) => p,
        Err(e) if is_no_device(&e) => return Outcome::NoDevice(e),
        Err(e) => return Outcome::Err(format!("{name} baseline 渲染: {e}")),
    };
    let t = match render_uv(fs, vs, uv_span, tampered, sample) {
        Ok(p) => p,
        Err(e) if is_no_device(&e) => return Outcome::NoDevice(e),
        Err(e) => return Outcome::Err(format!("{name} 篡改渲染: {e}")),
    };
    if !expect(b) {
        return Outcome::Miss(format!(
            "{name} baseline 采样点 {b:?} 未满足 expect 谓词(owner 校准阈值)"
        ));
    }
    if b == t {
        return Outcome::Miss(format!(
            "{name} 篡改后采样点未变({b:?} == {t:?})= 非真采样数据流,不计(RXS-0176 IR2)"
        ));
    }
    Outcome::Pass
}

// ── device 判据谓词(TODO:owner 本机迭代校准阈值/容差)──────────────────────
/// ① sample_lod 选层:LOD 1.0 采 level1〔绿 (0,255,0)〕。nearest 采纹素中心 → 绿主导。
fn expect_sample_lod(p: Px) -> bool {
    // TODO(owner device): 校准绿主导阈值(nearest 应逼近 (0,255,0,255))。
    p.1 > 180 && p.0 < 80 && p.2 < 80
}
/// ② load 越界钳制:px=(100,100) 钳制到边缘纹素 (3,3)〔品红 (255,0,255)〕。
fn expect_load_oob(p: Px) -> bool {
    // TODO(owner device): 校准边缘纹素色阈值。
    p.0 > 180 && p.2 > 180 && p.1 < 80
}
/// ③ gather:2×2 邻域四纹素 R 聚合为 vec4 四分量。四 R = {64,128,192,255} 之置换。
fn expect_gather(p: Px) -> bool {
    // TODO(owner device): D3D vs Vulkan gather 角点序不同——按置换核验(order-agnostic)。
    let mut got = [p.0, p.1, p.2, p.3];
    got.sort_unstable();
    got == [64, 128, 192, 255]
}
/// ⑦ 多分量:1×1 RGBA=(50,100,150,200) 四通道各异,sample_lod 全 vec4 采样。
fn expect_multi(p: Px) -> bool {
    // TODO(owner device): 校准四通道容差(linear 近似;nearest 应逼近精确值)。
    let near = |a: u8, b: u8| (a as i32 - b as i32).abs() <= 12;
    near(p.0, 50) && near(p.1, 100) && near(p.2, 150) && near(p.3, 200)
}

// ── 逐模式资源装配 ───────────────────────────────────────────────────────────
/// mip 逐层异色 4×4 纹理:level0 红 / level1 绿 / level2 蓝(选层判据);`lvl1` 可替换
/// (篡改)。
fn mip_tex(lvl1: [u8; 4]) -> GraphicsResource {
    GraphicsResource::Texture2D {
        width: 4,
        height: 4,
        data: TextureData::Rgba8(vec![
            solid(4, 4, [255, 0, 0, 255]),
            solid(2, 2, lvl1),
            solid(1, 1, [0, 0, 255, 255]),
        ]),
    }
}
fn nearest_clamp() -> GraphicsResource {
    GraphicsResource::Sampler(SamplerDesc {
        filter: Filter::Nearest,
        address: Address::Clamp,
        ..SamplerDesc::default()
    })
}

fn main() {
    let sh = sampling_shaders_spv();
    // build.rs codegen 降级(极少)→ 空切片,消费侧 SKIP(对齐既有降级纪律,非 fake)。
    if sh.fullscreen_vs.is_empty() || sh.sample_lod_fs.is_empty() {
        println!("SAMPLING_MODES: SKIP 采样模式着色器为空(build.rs codegen 降级)");
        return;
    }
    let vs_full = to_words(sh.fullscreen_vs);
    let vs_fetch = to_words(sh.fetch_vs);
    let fs_lod = to_words(sh.sample_lod_fs);
    let fs_load = to_words(sh.load_fs);
    let fs_gather = to_words(sh.gather_fs);
    let fs_cmp = to_words(sh.cmp_fs);
    let fs_storage = to_words(sh.storage_fs);

    let center = (W / 2, H / 2);
    let mut modes_ok: Vec<&str> = Vec::new();
    let mut misses: Vec<String> = Vec::new();

    macro_rules! handle {
        ($name:expr, $outcome:expr) => {
            match $outcome {
                Outcome::Pass => modes_ok.push($name),
                Outcome::Miss(m) => misses.push(m),
                Outcome::NoDevice(e) => {
                    println!("SAMPLING_MODES: SKIP 无 Vulkan 设备/loader({})", e.trim());
                    return;
                }
                Outcome::Err(e) => {
                    eprintln!("SAMPLING_MODES: FAIL {e}");
                    std::process::exit(1);
                }
            }
        };
    }

    // ── ① sample_lod_level(mip 选层)──────────────────────────────────────────
    handle!(
        "sample_lod_level",
        eval_uv(
            "sample_lod_level",
            &fs_lod,
            &vs_full,
            2.0,
            center,
            &[mip_tex([0, 255, 0, 255]), nearest_clamp()],
            &[mip_tex([0, 0, 0, 255]), nearest_clamp()], // 篡改 level1 → 采样点应变
            expect_sample_lod,
        )
    );

    // ── ⑦ multi_component(1×1 全 RGBA)──────────────────────────────────────
    let multi_tex = |rgba: [u8; 4]| GraphicsResource::Texture2D {
        width: 1,
        height: 1,
        data: TextureData::Rgba8(vec![solid(1, 1, rgba)]),
    };
    handle!(
        "multi_component",
        eval_uv(
            "multi_component",
            &fs_lod, // sample_lod 全 vec4 采样;LOD 1.0 → 1×1 单层钳到 level0
            &vs_full,
            2.0,
            center,
            &[multi_tex([50, 100, 150, 200]), nearest_clamp()],
            &[multi_tex([50, 100, 9, 200]), nearest_clamp()], // 篡改 B 通道
            expect_multi,
        )
    );

    // ── ③ gather_corner(2×2 邻域 R 聚合)────────────────────────────────────
    let gather_tex = |r11: u8| GraphicsResource::Texture2D {
        width: 2,
        height: 2,
        data: TextureData::Rgba8(vec![tex2x2([
            [64, 0, 0, 255],
            [128, 0, 0, 255],
            [192, 0, 0, 255],
            [r11, 0, 0, 255],
        ])]),
    };
    handle!(
        "gather_corner",
        eval_uv(
            "gather_corner",
            &fs_gather,
            &vs_full,
            2.0,
            center,
            &[gather_tex(255), nearest_clamp()],
            &[gather_tex(200), nearest_clamp()], // 篡改一纹素 R → 聚合分量变(255→200)
            expect_gather,
        )
    );

    // ── ② load_oob_clamp(整型取址越界钳制)──────────────────────────────────
    // px=(100,100) 越界 → 钳制到 4×4 边缘纹素 (3,3);edge 纹素置品红,篡改改边缘色。
    {
        let load_tex = |edge: [u8; 4]| {
            // 4×4 全黑,仅 (3,3) = edge(钳制目标)。
            let mut lvl = solid(4, 4, [0, 0, 0, 255]);
            let o = ((3 * 4 + 3) * 4) as usize;
            lvl[o..o + 4].copy_from_slice(&edge);
            GraphicsResource::Texture2D {
                width: 4,
                height: 4,
                data: TextureData::Rgba8(vec![lvl]),
            }
        };
        let verts = fetch_verts([100, 100], [0.0, 0.0, 0.0, 1.0]);
        let run = |edge: [u8; 4]| -> Result<Px, String> {
            let p = run_graphics_offscreen_v2(
                &vs_fetch,
                &fs_load,
                &verts,
                FETCH_STRIDE,
                &fetch_attrs(),
                W,
                H,
                CLEAR,
                &[load_tex(edge)],
            )?;
            Ok(px_at(&p, center.0, center.1))
        };
        let out = match (run([255, 0, 255, 255]), run([0, 0, 0, 255])) {
            (Ok(b), Ok(t)) => {
                if !expect_load_oob(b) {
                    Outcome::Miss(format!("load_oob_clamp baseline {b:?} 未满足谓词"))
                } else if b == t {
                    Outcome::Miss("load_oob_clamp 篡改边缘纹素后采样点未变".into())
                } else {
                    Outcome::Pass
                }
            }
            (Err(e), _) | (_, Err(e)) if is_no_device(&e) => Outcome::NoDevice(e),
            (Err(e), _) | (_, Err(e)) => Outcome::Err(format!("load_oob_clamp 渲染: {e}")),
        };
        handle!("load_oob_clamp", out);
    }

    // ── ④ sample_cmp_shadow(比较采样双色)──────────────────────────────────
    // 深度纹理左右两半异值 + SamplerCmp(compare)+ ref 0.5 → 两采样点比较结果异色。
    {
        // 4×4 深度纹理:左两列深度 0.2(< ref → pass=1)/ 右两列 0.8(> ref → fail=0)。
        // 以 R8 近似深度(0..1 → 0..255);SamplerCmp compare=Less。
        let depth_tex = |left: u8, right: u8| {
            let mut lvl = Vec::with_capacity(64);
            for _y in 0..4 {
                for x in 0..4 {
                    let d = if x < 2 { left } else { right };
                    lvl.extend_from_slice(&[d, d, d, 255]);
                }
            }
            GraphicsResource::Texture2D {
                width: 4,
                height: 4,
                data: TextureData::Rgba8(vec![lvl]),
            }
        };
        let scmp = GraphicsResource::Sampler(SamplerDesc {
            filter: Filter::Nearest,
            address: Address::Clamp,
            compare: Some(Compare::Less),
            ..SamplerDesc::default()
        });
        // uv_span=2 → 视口 uv∈[0,1];采样点取左(uv.x<0.5)与右(uv.x>0.5)两像素。
        let left_pt = (W / 4, H / 2);
        let right_pt = (3 * W / 4, H / 2);
        let verts = fullscreen_verts(2.0);
        let run_two = |left: u8, right: u8| -> Result<(Px, Px), String> {
            let p = run_graphics_offscreen_v2(
                &vs_full,
                &fs_cmp,
                &verts,
                FS_STRIDE,
                &fs_attrs(),
                W,
                H,
                CLEAR,
                &[depth_tex(left, right), scmp.clone()],
            )?;
            Ok((
                px_at(&p, left_pt.0, left_pt.1),
                px_at(&p, right_pt.0, right_pt.1),
            ))
        };
        let out = match (run_two(51, 204), run_two(204, 204)) {
            (Ok((bl, br)), Ok((tl, _tr))) => {
                // TODO(owner device): 校准 shadow 因子阈值(pass≈R 高 / fail≈R 低)。
                let two_color = bl.0 != br.0; // 左右比较结果双色
                let tamper_sensitive = bl.0 != tl.0; // 篡改左半深度 → 左采样点变
                if two_color && tamper_sensitive {
                    Outcome::Pass
                } else {
                    Outcome::Miss(format!(
                        "sample_cmp_shadow 双色={two_color} 篡改敏感={tamper_sensitive}(owner 校准)"
                    ))
                }
            }
            (Err(e), _) | (_, Err(e)) if is_no_device(&e) => Outcome::NoDevice(e),
            (Err(e), _) | (_, Err(e)) => Outcome::Err(format!("sample_cmp_shadow 渲染: {e}")),
        };
        handle!("sample_cmp_shadow", out);
    }

    // ── ⑥ wrap_vs_clamp(同 sample_lod 着色器 clamp vs wrap 双跑)──────────────
    // uv_span=4 → 视口 uv∈[0,2];右上采样点 uv>1:clamp 采边缘 / wrap 采回绕,像素必异。
    {
        // level1(采样层)左上/右下异色,使 wrap 与 clamp 在 uv>1 处取不同纹素。
        let tex = GraphicsResource::Texture2D {
            width: 4,
            height: 4,
            data: TextureData::Rgba8(vec![
                solid(4, 4, [255, 0, 0, 255]),
                tex2x2([
                    [0, 255, 0, 255],
                    [0, 0, 255, 255],
                    [255, 255, 0, 255],
                    [255, 0, 255, 255],
                ]),
                solid(1, 1, [0, 0, 255, 255]),
            ]),
        };
        let clamp = GraphicsResource::Sampler(SamplerDesc {
            filter: Filter::Nearest,
            address: Address::Clamp,
            ..SamplerDesc::default()
        });
        let wrap = GraphicsResource::Sampler(SamplerDesc {
            filter: Filter::Nearest,
            address: Address::Wrap,
            ..SamplerDesc::default()
        });
        let sample = (3 * W / 4, H / 4); // uv≈(1.5, 0.5),uv.x>1
        let verts = fullscreen_verts(4.0);
        let run = |samp: GraphicsResource| -> Result<Px, String> {
            let p = run_graphics_offscreen_v2(
                &vs_full,
                &fs_lod,
                &verts,
                FS_STRIDE,
                &fs_attrs(),
                W,
                H,
                CLEAR,
                &[tex.clone(), samp],
            )?;
            Ok(px_at(&p, sample.0, sample.1))
        };
        let out = match (run(clamp), run(wrap)) {
            (Ok(c), Ok(w)) => {
                // 判据 = clamp 与 wrap 在 uv>1 采样点像素必异(篡改替身 = 换 sampler 状态)。
                if c != w {
                    Outcome::Pass
                } else {
                    Outcome::Miss(format!(
                        "wrap_vs_clamp uv>1 采样点 clamp={c:?} == wrap={w:?}"
                    ))
                }
            }
            (Err(e), _) | (_, Err(e)) if is_no_device(&e) => Outcome::NoDevice(e),
            (Err(e), _) | (_, Err(e)) => Outcome::Err(format!("wrap_vs_clamp 渲染: {e}")),
        };
        handle!("wrap_vs_clamp", out);
    }

    // ── ⑤ storage_unique_writer(TextureRw2D store→回读)────────────────────────
    // fetch_vs 提供 px + val;fragment store(px, val) → load;v2 storage readback 出口回读
    // storage image 本体,断言写入的 texel == val。篡改 val → 回读纹素变(数据流红绿)。
    // **device 迭代 TODO**:首期 px flat = 全三角形常量(每 texel 多写者,非严格唯一写者);
    // 严格 identity(每 fragment 写本目标像素坐标)须 FragCoord 作 fragment 输入 builtin
    // 接线(device 迭代面,§4.B5)。本 harness 见证 store→readback 全链 + 数据流敏感。
    {
        let px = [3u32, 5]; // 8×8 storage image 内某 texel
        let run = |val: [f32; 4]| -> Result<Option<Vec<u8>>, String> {
            let verts = fetch_verts(px, val);
            let (_color, storage) = run_graphics_offscreen_v2_readback(
                &vs_fetch,
                &fs_storage,
                &verts,
                FETCH_STRIDE,
                &fetch_attrs(),
                W,
                H,
                CLEAR,
                &[GraphicsResource::StorageImage {
                    width: 8,
                    height: 8,
                    format: StorageFormat::Rgba32Float,
                }],
            )?;
            Ok(storage)
        };
        // 读 storage image 中 texel (3,5) 的 R 分量(Rgba32Float:16B/纹素,小端 f32)。
        let read_texel_r = |buf: &[u8]| -> Option<f32> {
            let idx = ((px[1] * 8 + px[0]) * 16) as usize;
            buf.get(idx..idx + 4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        };
        let out = match (run([0.25, 0.5, 0.75, 1.0]), run([0.9, 0.5, 0.75, 1.0])) {
            (Ok(Some(b)), Ok(Some(t))) => {
                let br = read_texel_r(&b);
                let tr = read_texel_r(&t);
                match (br, tr) {
                    // TODO(owner device): 校准写入值容差(唯一写者下应逼近 val.r 精确值)。
                    (Some(bv), Some(tv)) => {
                        let wrote = (bv - 0.25).abs() < 0.05; // baseline val.r=0.25
                        let sensitive = (bv - tv).abs() > 0.05; // 篡改 val.r 0.25→0.9
                        if wrote && sensitive {
                            Outcome::Pass
                        } else {
                            Outcome::Miss(format!(
                                "storage_unique_writer 写入={wrote}(texel.r={bv}) 篡改敏感={sensitive}"
                            ))
                        }
                    }
                    _ => Outcome::Miss("storage_unique_writer 回读缓冲过短".into()),
                }
            }
            (Ok(None), _) | (_, Ok(None)) => {
                Outcome::Err("storage_unique_writer 回读出口返回 None(应有 storage image)".into())
            }
            (Err(e), _) | (_, Err(e)) if is_no_device(&e) => Outcome::NoDevice(e),
            (Err(e), _) | (_, Err(e)) => Outcome::Err(format!("storage_unique_writer 渲染: {e}")),
        };
        handle!("storage_unique_writer", out);
    }

    // ── 汇总 + evidence ───────────────────────────────────────────────────────
    for m in &misses {
        eprintln!("SAMPLING_MODES: MISS {m}");
    }
    let n = modes_ok.len();
    println!("SAMPLING_MODES: ok modes_ok={n} [{}]", modes_ok.join(", "));
    if let Err(e) = write_evidence(&modes_ok) {
        eprintln!("SAMPLING_MODES: WARN evidence 写入失败: {e}");
    }
    // ≥6 = counter 判据满足(g3.counter.sampling_superset_modes);< 6 = owner 迭代未竟,
    // 非硬红(退 0,misses 已列供校准)——真跑值不伪造,counter 由 evidence modes_ok 计。
    if n >= 6 {
        println!("SAMPLING_MODES: PASS ≥6 模式 device 数值判据(counter 满足)");
    } else {
        println!("SAMPLING_MODES: PARTIAL {n}/6 模式过(owner 迭代 expect_* 阈值/采样点)");
    }
}

/// evidence/sampling_superset_<date>.json(schema milestones/g3/sampling_superset_evidence_schema.json)。
/// 仅 device 真跑写(此 harness 有 GPU 时);modes_ok 为真过模式,num_modes = len。
fn write_evidence(modes_ok: &[&str]) -> std::io::Result<()> {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let dir = repo.join("evidence");
    std::fs::create_dir_all(&dir)?;
    // 日期戳(UTC 简易:秒级 epoch → YYYYMMDD 由 owner CI 环境替代;此处用 epoch 秒防覆盖)。
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
        "{{\n  \"schema_version\": 1,\n  \"subject\": \"sampling_superset_smoke\",\n  \
         \"milestone\": \"g3.3\",\n  \"modes_ok\": [{list}],\n  \"num_modes\": {},\n  \
         \"adapter\": \"{adapter}\",\n  \"run_url\": \"{run_url}\",\n  \
         \"timestamp\": \"{ts}\"\n}}\n",
        modes_ok.len()
    );
    let path = dir.join(format!("sampling_superset_{stamp}.json"));
    std::fs::write(path, json)
}

/// epoch 秒 → RFC3339 UTC(schema `format: date-time`;无外部依赖,简易换算)。
fn rfc3339_utc(secs: u64) -> String {
    // 简化:仅用于 evidence timestamp 字段;精度到秒,UTC。
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // 1970-01-01 起的天数 → Y/M/D(Howard Hinnant civil_from_days 算法)。
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
