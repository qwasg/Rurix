//! G27.2 M-a HZB device kernel harness(门 g27.p0.m_a.hzb_device_kernel;
//! G27_CONTRACT §4.2 M-a 行逐字;RFC-0044 §1 判据事实源;g26_framegen_device 同模)。
//!
//! ## 集成路径
//!
//! bin-local 全部逻辑:`DeviceHzb` 经 `rurix_rt::vk::run_compute`(G12/G13/G26
//! compute 派发面同车道)驱动 `kernels/g27_hzb_reduce.rx`(单级 2×2 farther-of
//! 归约,host 逐级 dispatch 直至 1×1;mip0 = host 深度原字节直传不经 kernel)与
//! `kernels/g27_hzb_test.rx`(逐 rect 保守遮挡测试,金字塔平铺 + mip 表单
//! dispatch)。公式面与 host 金标准 `geometry/hzb.rs`(HzbPyramid::build /
//! test_rect / exact_rect_occluded)逐字同源;**geometry/ 冻结面 0-byte 不接线**
//! (hzb.rs/cull.rs/visbuffer.rs vs g26-closed git-diff 机核归 CI),host 参考臂
//! 只消费不改写(farther/is_farther 为 hzb.rs 私有方法,bin-local 同律复制)。
//!
//! ## 夹具(g20_hzb_probe.rs 逐字同源)
//!
//! 193×117 非 2 幂三段式确定性深度场(fx<0.42 近墙 0.88+0.05|sin(9fy)| /
//! fy>0.7 中景 0.55 / 其余远景)+ det_rects(800)(位混合伪随机)+
//! reverse-Z/standard-Z 双臂(standard 臂深度 1−d 变换,g20 双臂构造同口径)。
//! 深度场 host 单源生成一次、原字节上传(device 不重生成,RFC-0044 §1.1 域前提)。
//!
//! ## 判据面(RFC-0044 §1.2;全零容差,无标定腿)
//!
//! ① device 金字塔 vs host HzbPyramid::build 逐级**位级相等**(to_bits 全等);
//! ② 800 rect × 双约定判定序列 vs host test_rect **逐 rect 逐字节**全等;
//! ③ 零假阳性硬不变量:device 判 Occluded ⇒ exact_rect_occluded 必同判
//!    (独立纵深防御复核,F3);④ device 双跑位级一致(digest = sha256(判定位
//!    序列 1 字节/rect ‖ 金字塔逐级 f32 LE),g20_hzb_probe 同口径 F11);
//!    ⑤ 剔除数 > 0。
//!
//! ## RED 臂(RFC-0044 §1.2 ⑤ F4 构造性注入协议)
//!
//! `--red-arm tamper`:host 扫描预算定位单一金字塔纹素(优先 mip1;被 ≥1 个
//! host-Visible 且精确真值可见 rect 的 ≤2×2 采样窗覆盖、模拟注入后必翻
//! Occluded——构造性保证消费路径命中),写「更近」极值(reverse-Z 1.0)→
//! 臂 A:逐 rect 字节序列必异;臂 B:假阳性哨兵(③ 裁判函数)必检出 ≥1。
//!
//! ## 三态
//!
//! 无 Vulkan loader/设备 → `skipped_dev_env` JSON 退 0(非 fake pass;
//! `RURIX_REQUIRE_REAL=1` 下 SKIP→硬红由 smoke 脚本层裁决);host 腿恒可
//! `--host-only`;判据不符 / RED 臂失效 ⇒ FAIL 退 1。
//!
//! ## 用法
//!
//! ```text
//! g27_hzb_device --spv-reduce <r.spv> --spv-test <t.spv>       # 全档验证(默认,双约定)
//! g27_hzb_device --red-arm tamper --spv-reduce <r> --spv-test <t>
//! g27_hzb_device --probe --spv-reduce <r> --spv-test <t> [--out <path>]  # soak 快车道
//! g27_hzb_device --host-only
//! ```

#![forbid(unsafe_code)]

use rurix_render::geometry::hzb::{DepthConvention, HzbPyramid, Occlusion, exact_rect_occluded};
use rurix_render::temporal::image::ImageF32;
use rurix_rt::vk;

const TAG: &str = "[g27_hzb_device]";
/// 夹具分辨率(g20_hzb_probe 逐字:非 2 幂)。
const W: u32 = 193;
const H: u32 = 117;
const RECTS: u32 = 800;

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

// ---------------------------------------------------------------------------
// 夹具(g20_hzb_probe.rs 逐字同源)
// ---------------------------------------------------------------------------

fn scene_depth_reverse_z(w: u32, h: u32) -> ImageF32 {
    ImageF32::from_fn(w, h, 1, |x, y, _| {
        let fx = (x as f32 + 0.5) / w as f32;
        let fy = (y as f32 + 0.5) / h as f32;
        if fx < 0.42 {
            0.88 + 0.05 * (fy * 9.0).sin().abs()
        } else if fy > 0.7 {
            0.55 // 中景带
        } else {
            0.08 + 0.06 * ((fx * 7.0 + fy * 3.0).sin() * 0.5 + 0.5)
        }
    })
}

fn det_rects(n: u32) -> Vec<([f32; 2], [f32; 2], f32)> {
    let mut out = Vec::new();
    for i in 0..n {
        let mut v = i.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
        let mut next = || {
            v ^= v >> 15;
            v = v.wrapping_mul(0x7FEB_352D);
            v ^= v >> 13;
            (v % 1000) as f32 / 1000.0
        };
        let cx = next();
        let cy = next();
        let hw = 0.02 + 0.22 * next();
        let hh = 0.02 + 0.22 * next();
        let d = next();
        out.push((
            [(cx - hw).clamp(0.0, 1.0), (cy - hh).clamp(0.0, 1.0)],
            [(cx + hw).clamp(0.0, 1.0), (cy + hh).clamp(0.0, 1.0)],
            d,
        ));
    }
    out
}

/// 约定臂深度场(standard 臂 = 1 − reverse 场;g20 run_arm 同口径)。
fn arm_depth(conv: DepthConvention) -> ImageF32 {
    let rz = scene_depth_reverse_z(W, H);
    match conv {
        DepthConvention::ReverseZ => rz,
        DepthConvention::StandardZ => ImageF32::from_fn(W, H, 1, |x, y, _| 1.0 - rz.get(x, y, 0)),
    }
}

/// 约定臂 rect 流(standard 臂深度 1−d 变换;g20 run_arm 同口径)。
fn arm_rects(conv: DepthConvention) -> Vec<([f32; 2], [f32; 2], f32)> {
    det_rects(RECTS)
        .into_iter()
        .map(|(mn, mx, d0)| {
            let d = match conv {
                DepthConvention::ReverseZ => d0,
                DepthConvention::StandardZ => 1.0 - d0,
            };
            (mn, mx, d)
        })
        .collect()
}

fn conv_name(conv: DepthConvention) -> &'static str {
    match conv {
        DepthConvention::ReverseZ => "reverse_z",
        DepthConvention::StandardZ => "standard_z",
    }
}

/// kernel 参数面约定位(0.0=reverse-Z 取 min / 1.0=standard-Z 取 max)。
fn conv_flag(conv: DepthConvention) -> f32 {
    match conv {
        DepthConvention::ReverseZ => 0.0,
        DepthConvention::StandardZ => 1.0,
    }
}

// ---------------------------------------------------------------------------
// bin-local farther/is_farther(hzb.rs 私有方法同律复制;冻结面只读不改)
// ---------------------------------------------------------------------------

fn farther(conv: DepthConvention, a: f32, b: f32) -> f32 {
    match conv {
        DepthConvention::ReverseZ => a.min(b),
        DepthConvention::StandardZ => a.max(b),
    }
}

fn is_farther(conv: DepthConvention, a: f32, b: f32) -> bool {
    match conv {
        DepthConvention::ReverseZ => a < b,
        DepthConvention::StandardZ => a > b,
    }
}

// ---------------------------------------------------------------------------
// 字节工具 + digest(g20_hzb_probe 序列化字面 F11:判定位序列 ‖ 金字塔 f32 LE)
// ---------------------------------------------------------------------------

fn bytes_f32(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn read_f32(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn load_spv(path: &str) -> Vec<u32> {
    let bytes = std::fs::read(path).unwrap_or_else(|e| fail(&format!("读 {path}: {e}")));
    if bytes.len() % 4 != 0 {
        fail("SPIR-V 字节数非 4 对齐");
    }
    bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// digest = sha256(判定位序列 1 字节/rect ‖ 金字塔逐级 f32 LE)(F11 字面,
/// g20_hzb_probe run_arm trace 同口径)。
fn trace_digest(verdicts: &[u8], mips: &[ImageF32]) -> String {
    let mut trace: Vec<u8> = verdicts.to_vec();
    for m in mips {
        for &v in &m.data {
            trace.extend_from_slice(&v.to_le_bytes());
        }
    }
    rurix_pkg::sha256::hex_digest(&trace)
}

/// 金字塔逐级位级全等(to_bits 全等;零容差协议 §1.1——PartialEq 的 f32 ==
/// 在 −0/NaN 面弱于位级,故显式 to_bits)。
fn pyramids_bitexact(a: &[ImageF32], b: &[ImageF32]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b.iter()).all(|(ma, mb)| {
            ma.w == mb.w
                && ma.h == mb.h
                && ma.data.len() == mb.data.len()
                && ma
                    .data
                    .iter()
                    .zip(mb.data.iter())
                    .all(|(x, y)| x.to_bits() == y.to_bits())
        })
}

// ---------------------------------------------------------------------------
// device 臂(bin-local;经 vk::run_compute 逐级/单 dispatch 派发)
// ---------------------------------------------------------------------------

struct DeviceHzb {
    spv_reduce: Vec<u32>,
    entry_reduce: String,
    spv_test: Vec<u32>,
    entry_test: String,
}

impl DeviceHzb {
    fn create(spv_reduce: Vec<u32>, spv_test: Vec<u32>) -> Result<Self, String> {
        if !vk::vulkan_available() {
            return Err("vulkan loader 不可用".into());
        }
        let entry_reduce = vk::entry_point_name(&spv_reduce).ok_or("reduce SPV 无 OpEntryPoint")?;
        let entry_test = vk::entry_point_name(&spv_test).ok_or("test SPV 无 OpEntryPoint")?;
        Ok(Self {
            spv_reduce,
            entry_reduce,
            spv_test,
            entry_test,
        })
    }

    /// device 逐级 dispatch 建全金字塔(host HzbPyramid::build 同拓扑:
    /// mip0 = host 深度原字节直传不经 kernel;非 2 幂 ceil 减半直至 1×1;
    /// 每级输入 = 上一级 device 输出字节,全链 device 归约)。
    fn build_pyramid(&self, depth: &ImageF32, conv: DepthConvention) -> Vec<ImageF32> {
        assert!(depth.c == 1, "HZB 输入必须单通道深度");
        let mut mips = vec![depth.clone()];
        while mips.last().unwrap().w > 1 || mips.last().unwrap().h > 1 {
            let prev = mips.last().unwrap();
            let (pw, ph) = (prev.w, prev.h);
            let nw = pw.div_ceil(2).max(1);
            let nh = ph.div_ceil(2).max(1);
            // 参数面(g27_hzb_reduce.rx 逐字同源;8 f32)。
            let params = [
                (nw * nh) as f32,
                nw as f32,
                nh as f32,
                pw as f32,
                ph as f32,
                conv_flag(conv),
                0.0,
                0.0,
            ];
            let mut bufs = vec![
                bytes_f32(&prev.data),
                bytes_f32(&params),
                vec![0u8; (nw * nh) as usize * 4],
            ];
            vk::run_compute(
                &self.spv_reduce,
                &self.entry_reduce,
                &mut bufs,
                &[],
                [nw * nh, 1, 1],
            )
            .unwrap_or_else(|e| fail(&format!("reduce dispatch({pw}×{ph}→{nw}×{nh})失败: {e}")));
            mips.push(ImageF32 {
                w: nw,
                h: nh,
                c: 1,
                data: read_f32(&bufs[2]),
            });
        }
        mips
    }

    /// device 逐 rect 保守遮挡测试:金字塔平铺 + mip 表 + rect 流单 dispatch;
    /// 返回判定字节序列(1 字节/rect,0=Visible/1=Occluded,F11 字面)。
    fn test_rects(
        &self,
        mips: &[ImageF32],
        conv: DepthConvention,
        rects: &[([f32; 2], [f32; 2], f32)],
    ) -> Vec<u8> {
        let mut flat: Vec<f32> = Vec::new();
        let mut table: Vec<f32> = Vec::new();
        for m in mips {
            table.push(flat.len() as f32);
            table.push(m.w as f32);
            table.push(m.h as f32);
            flat.extend_from_slice(&m.data);
        }
        let mut rect_buf: Vec<f32> = Vec::with_capacity(rects.len() * 5);
        for (mn, mx, d) in rects {
            rect_buf.extend_from_slice(&[mn[0], mn[1], mx[0], mx[1], *d]);
        }
        let n = rects.len() as u32;
        // 参数面(g27_hzb_test.rx 逐字同源;8 f32)。
        let params = [
            n as f32,
            mips.len() as f32,
            mips[0].w as f32,
            mips[0].h as f32,
            conv_flag(conv),
            0.0,
            0.0,
            0.0,
        ];
        let mut bufs = vec![
            bytes_f32(&flat),
            bytes_f32(&table),
            bytes_f32(&rect_buf),
            bytes_f32(&params),
            vec![0u8; rects.len() * 4],
        ];
        vk::run_compute(&self.spv_test, &self.entry_test, &mut bufs, &[], [n, 1, 1])
            .unwrap_or_else(|e| fail(&format!("test dispatch({n} rect)失败: {e}")));
        // verdict f32 恒 ∈ {0.0, 1.0}(算术门输出);>0.5 判读为 Occluded 字节。
        read_f32(&bufs[4])
            .iter()
            .map(|&v| u8::from(v > 0.5))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// host 参考臂(金标准;hzb.rs 只消费)
// ---------------------------------------------------------------------------

/// host 判定字节序列(test_rect 金标准;1 字节/rect,F11 字面)。
fn host_verdicts(pyr: &HzbPyramid, rects: &[([f32; 2], [f32; 2], f32)]) -> Vec<u8> {
    rects
        .iter()
        .map(|(mn, mx, d)| match pyr.test_rect(*mn, *mx, *d) {
            Occlusion::Occluded => 1u8,
            Occlusion::Visible => 0u8,
        })
        .collect()
}

/// bin-local 复算 host test_rect 的选级与 ≤2×2 采样窗(tamper 注入点预算用;
/// 公式面与 hzb.rs test_rect 逐字同源,只读不改 host 模块)。
fn rect_window(
    mips: &[ImageF32],
    uv_min: [f32; 2],
    uv_max: [f32; 2],
) -> (usize, u32, u32, u32, u32) {
    let base = &mips[0];
    let (w0, h0) = (base.w as f32, base.h as f32);
    let x0 = (uv_min[0].clamp(0.0, 1.0) * w0).floor().clamp(0.0, w0 - 1.0) as u32;
    let y0 = (uv_min[1].clamp(0.0, 1.0) * h0).floor().clamp(0.0, h0 - 1.0) as u32;
    let x1 = (uv_max[0].clamp(0.0, 1.0) * w0).ceil().clamp(1.0, w0) as u32 - 1;
    let y1 = (uv_max[1].clamp(0.0, 1.0) * h0).ceil().clamp(1.0, h0) as u32 - 1;
    let span = (x1 - x0 + 1).max(y1 - y0 + 1);
    let mut mip = 0u32;
    while (span >> mip) > 2 {
        mip += 1;
    }
    let mip = (mip as usize).min(mips.len() - 1);
    let img = &mips[mip];
    let mx0 = x0 >> mip as u32;
    let my0 = y0 >> mip as u32;
    let mx1 = (x1 >> mip as u32).min(img.w - 1);
    let my1 = (y1 >> mip as u32).min(img.h - 1);
    (mip, mx0, my0, mx1, my1)
}

// ---------------------------------------------------------------------------
// JSON 出报(手写零新依赖;g26_framegen_device 同模)
// ---------------------------------------------------------------------------

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn jstr(s: &str) -> String {
    format!("\"{}\"", json_escape(s))
}

fn strs_json(items: &[String]) -> String {
    let inner: Vec<String> = items.iter().map(|s| jstr(s)).collect();
    format!("[{}]", inner.join(","))
}

fn base_commit() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}

// ---------------------------------------------------------------------------
// 参数
// ---------------------------------------------------------------------------

struct Args {
    spv_reduce: Option<String>,
    spv_test: Option<String>,
    red_arm: Option<String>,
    probe: bool,
    host_only: bool,
    out: Option<String>,
}

fn parse_args() -> Args {
    let mut a = Args {
        spv_reduce: None,
        spv_test: None,
        red_arm: None,
        probe: false,
        host_only: false,
        out: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        match k.as_str() {
            "--spv-reduce" => a.spv_reduce = it.next(),
            "--spv-test" => a.spv_test = it.next(),
            "--red-arm" => a.red_arm = it.next(),
            "--probe" => a.probe = true,
            "--host-only" => a.host_only = true,
            "--out" => a.out = it.next(),
            other => fail(&format!("未知参数: {other}")),
        }
    }
    a
}

fn device_arm(args: &Args) -> Result<DeviceHzb, String> {
    let spv_r = load_spv(
        args.spv_reduce
            .as_deref()
            .unwrap_or_else(|| fail("缺 --spv-reduce")),
    );
    let spv_t = load_spv(
        args.spv_test
            .as_deref()
            .unwrap_or_else(|| fail("缺 --spv-test")),
    );
    DeviceHzb::create(spv_r, spv_t)
}

// ---------------------------------------------------------------------------
// 全档验证单臂(①mips 位级 ②判定序列全等 ③零假阳性 ④双跑位级 ⑤剔除数>0)
// ---------------------------------------------------------------------------

struct DeviceArmReport {
    conv: &'static str,
    mips_len: usize,
    mips_bitexact: bool,
    verdict_equal: bool,
    occluded: u32,
    visible: u32,
    false_positives: u32,
    double_run_bitexact: bool,
    digest: String,
    host_digest: String,
}

impl DeviceArmReport {
    fn all_green(&self) -> bool {
        self.mips_bitexact
            && self.verdict_equal
            && self.false_positives == 0
            && self.double_run_bitexact
            && self.occluded > 0
    }

    fn to_json(&self) -> String {
        format!(
            "{{\"conv\":{},\"rects\":{RECTS},\"mips\":{},\"mips_bitexact\":{},\
             \"verdict_sequence_equal\":{},\"occluded\":{},\"visible\":{},\
             \"false_positives\":{},\"double_run_bitexact\":{},\"digest\":{},\
             \"host_digest\":{},\"digest_equal_host\":{}}}",
            jstr(self.conv),
            self.mips_len,
            self.mips_bitexact,
            self.verdict_equal,
            self.occluded,
            self.visible,
            self.false_positives,
            self.double_run_bitexact,
            jstr(&format!("sha256:{}", self.digest)),
            jstr(&format!("sha256:{}", self.host_digest)),
            self.digest == self.host_digest,
        )
    }
}

fn run_device_arm(dev: &DeviceHzb, conv: DepthConvention) -> DeviceArmReport {
    let depth = arm_depth(conv);
    let rects = arm_rects(conv);
    // host 金标准腿(恒跑)。
    let host_pyr = HzbPyramid::build(&depth, conv);
    let host_seq = host_verdicts(&host_pyr, &rects);
    let host_digest = trace_digest(&host_seq, &host_pyr.mips);
    // device 全链双跑(build 逐级 dispatch + test 单 dispatch)。
    let run = || {
        let mips = dev.build_pyramid(&depth, conv);
        let verdicts = dev.test_rects(&mips, conv, &rects);
        (mips, verdicts)
    };
    let (mips_a, verd_a) = run();
    let (mips_b, verd_b) = run();
    let digest_a = trace_digest(&verd_a, &mips_a);
    let digest_b = trace_digest(&verd_b, &mips_b);
    // ③ 零假阳性独立复核(裁判函数直核,不依赖 ② 判定链路;F3 纵深防御)。
    let mut occluded = 0u32;
    let mut fp = 0u32;
    for (i, &b) in verd_a.iter().enumerate() {
        if b == 1 {
            occluded += 1;
            let (mn, mx, d) = rects[i];
            if !exact_rect_occluded(&depth, conv, mn, mx, d) {
                fp += 1;
            }
        }
    }
    DeviceArmReport {
        conv: conv_name(conv),
        mips_len: mips_a.len(),
        mips_bitexact: pyramids_bitexact(&mips_a, &host_pyr.mips),
        verdict_equal: verd_a == host_seq,
        occluded,
        visible: RECTS - occluded,
        false_positives: fp,
        double_run_bitexact: digest_a == digest_b,
        digest: digest_a,
        host_digest,
    }
}

// ---------------------------------------------------------------------------
// RED 臂:tamper(构造性注入;RFC-0044 §1.2 ⑤ F4)
// ---------------------------------------------------------------------------

struct TamperPlan {
    mip: usize,
    tx: u32,
    ty: u32,
    rect_idx: usize,
}

/// host 扫描预算注入点:优先 mip1;候选 = host-Visible 且精确真值可见
/// (exact_rect_occluded=false,哨兵面保证)的 rect 的 ≤2×2 采样窗内纹素,
/// 模拟注入「更近」极值后按 host 字面重算 farthest 必翻 Occluded 才选中
/// (构造性保证消费路径命中 + 假阳性哨兵必检出)。
fn plan_tamper(
    mips: &[ImageF32],
    conv: DepthConvention,
    depth: &ImageF32,
    rects: &[([f32; 2], [f32; 2], f32)],
    host_seq: &[u8],
) -> Option<TamperPlan> {
    let inject = match conv {
        DepthConvention::ReverseZ => 1.0f32,
        DepthConvention::StandardZ => 0.0f32,
    };
    for require_mip1 in [true, false] {
        for (ri, (mn, mx, d)) in rects.iter().enumerate() {
            if host_seq[ri] != 0 || exact_rect_occluded(depth, conv, *mn, *mx, *d) {
                continue;
            }
            let (mip, mx0, my0, mx1, my1) = rect_window(mips, *mn, *mx);
            if require_mip1 && mip != 1 {
                continue;
            }
            let img = &mips[mip];
            for ty in my0..=my1 {
                for tx in mx0..=mx1 {
                    let val =
                        |wx: u32, wy: u32| if wx == tx && wy == ty { inject } else { img.get(wx, wy, 0) };
                    let mut far = val(mx0, my0);
                    for wy in my0..=my1 {
                        for wx in mx0..=mx1 {
                            far = farther(conv, far, val(wx, wy));
                        }
                    }
                    if is_farther(conv, *d, far) {
                        return Some(TamperPlan {
                            mip,
                            tx,
                            ty,
                            rect_idx: ri,
                        });
                    }
                }
            }
        }
    }
    None
}

fn red_arm_tamper(dev: &DeviceHzb) -> Result<String, String> {
    let conv = DepthConvention::ReverseZ;
    let depth = arm_depth(conv);
    let rects = arm_rects(conv);
    let host_pyr = HzbPyramid::build(&depth, conv);
    let host_seq = host_verdicts(&host_pyr, &rects);
    // honest device 全链(金字塔与 host 位级等由全档验证承载,此处为注入基线)。
    let mips = dev.build_pyramid(&depth, conv);
    let honest = dev.test_rects(&mips, conv, &rects);
    // host 预算注入点(构造性;不可达即如实 Err→FAIL 不冒充)。
    let plan = plan_tamper(&mips, conv, &depth, &rects, &host_seq)
        .ok_or("构造性注入点不可达:无可翻转的 host-Visible rect 采样窗纹素")?;
    let mut tampered_mips = mips.clone();
    let inject = 1.0f32; // reverse-Z「更近」极值
    tampered_mips[plan.mip].set(plan.tx, plan.ty, 0, inject);
    let tampered = dev.test_rects(&tampered_mips, conv, &rects);
    // 臂 A:逐 rect 字节序列必异(比较面 = 逐 rect 字节,非仅组合 digest)。
    let diff_n = honest
        .iter()
        .zip(tampered.iter())
        .filter(|(a, b)| a != b)
        .count();
    if diff_n == 0 {
        return Err("臂 A 漏检:注入后逐 rect 字节序列未变".into());
    }
    // 臂 B:假阳性哨兵——③ 裁判函数必检出 ≥1(device Occluded 且精确真值可见)。
    let mut fp = 0u32;
    for (i, &b) in tampered.iter().enumerate() {
        if b == 1 {
            let (mn, mx, d) = rects[i];
            if !exact_rect_occluded(&depth, conv, mn, mx, d) {
                fp += 1;
            }
        }
    }
    if fp == 0 {
        return Err(format!(
            "臂 B 漏检:注入后裁判函数未检出假阳性(序列已异 {diff_n} rect)"
        ));
    }
    Ok(format!(
        "注入 mip{} 纹素 ({},{}) rect#{} 命中;臂 A 序列异 {} rect;臂 B 假阳性哨兵检出 {}",
        plan.mip, plan.tx, plan.ty, plan.rect_idx, diff_n, fp
    ))
}

// ---------------------------------------------------------------------------
// probe(soak 快车道:单约定 reverse-Z 全 800 rect 全链 + host 对拍 + 双跑)
// ---------------------------------------------------------------------------

fn emit_probe(line: &str, args: &Args) {
    println!("{line}");
    if let Some(path) = &args.out {
        if let Some(parent) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(path, format!("{line}\n"))
            .unwrap_or_else(|e| fail(&format!("写 --out {path}: {e}")));
    }
}

fn probe_leg(args: &Args) -> ! {
    let dev = match device_arm(args) {
        Ok(d) => d,
        Err(e) => {
            let line = format!(
                "{{\"schema\":\"rurix.g27hzb.probe.v1\",\"state\":\"skipped_dev_env\",\"reason\":{}}}",
                jstr(&e)
            );
            emit_probe(&line, args);
            std::process::exit(0);
        }
    };
    let r = run_device_arm(&dev, DepthConvention::ReverseZ);
    let state = if r.all_green() { "pass" } else { "fail" };
    let line = format!(
        "{{\"schema\":\"rurix.g27hzb.probe.v1\",\"state\":{},\"conv\":\"reverse_z\",\"rects\":{RECTS},\
         \"mips\":{},\"mips_bitexact\":{},\"verdict_sequence_equal\":{},\"occluded\":{},\
         \"false_positives\":{},\"bitexact\":{},\"digest\":{},\"base_commit\":{}}}",
        jstr(state),
        r.mips_len,
        r.mips_bitexact,
        r.verdict_equal,
        r.occluded,
        r.false_positives,
        r.double_run_bitexact,
        jstr(&format!("sha256:{}", r.digest)),
        jstr(&base_commit()),
    );
    emit_probe(&line, args);
    std::process::exit(if state == "pass" { 0 } else { 1 })
}

// ---------------------------------------------------------------------------
// host 腿(--host-only:g20_hzb_probe 判据面同构调试腿)
// ---------------------------------------------------------------------------

fn host_only_leg() -> ! {
    let mut arms_json = Vec::new();
    let mut all_ok = true;
    for conv in [DepthConvention::ReverseZ, DepthConvention::StandardZ] {
        let depth = arm_depth(conv);
        let rects = arm_rects(conv);
        let pyr = HzbPyramid::build(&depth, conv);
        let seq = host_verdicts(&pyr, &rects);
        let occluded = seq.iter().filter(|&&b| b == 1).count() as u32;
        let mut fp = 0u32;
        for (i, &b) in seq.iter().enumerate() {
            if b == 1 {
                let (mn, mx, d) = rects[i];
                if !exact_rect_occluded(&depth, conv, mn, mx, d) {
                    fp += 1;
                }
            }
        }
        let ok = fp == 0 && occluded > 0;
        all_ok = all_ok && ok;
        arms_json.push(format!(
            "{{\"conv\":{},\"rects\":{RECTS},\"mips\":{},\"occluded\":{},\"visible\":{},\
             \"false_positives\":{},\"digest\":{}}}",
            jstr(conv_name(conv)),
            pyr.mips.len(),
            occluded,
            RECTS - occluded,
            fp,
            jstr(&format!("sha256:{}", trace_digest(&seq, &pyr.mips))),
        ));
        eprintln!(
            "{TAG}: host {} occluded={} visible={} fp={} mips={}",
            conv_name(conv),
            occluded,
            RECTS - occluded,
            fp,
            pyr.mips.len()
        );
    }
    let state = if all_ok { "pass" } else { "fail" };
    println!(
        "{{\"schema\":\"rurix.g27hzb.harness.v1\",\"mode\":\"host-only\",\"state\":{},\
         \"resolution\":[{W},{H}],\"arms\":[{}]}}",
        jstr(state),
        arms_json.join(","),
    );
    std::process::exit(if all_ok { 0 } else { 1 })
}

// ---------------------------------------------------------------------------
// main(默认 = 全档验证:双约定逐臂)
// ---------------------------------------------------------------------------

fn main() {
    let args = parse_args();

    if args.host_only {
        host_only_leg();
    }
    if args.probe {
        probe_leg(&args);
    }
    if let Some(arm) = &args.red_arm {
        if arm != "tamper" {
            fail(&format!("未知 RED 臂: {arm}(tamper)"));
        }
        let dev = match device_arm(&args) {
            Ok(d) => d,
            Err(e) => {
                println!(
                    "{{\"schema\":\"rurix.g27hzb.red_arm.v1\",\"arm\":\"tamper\",\"detected\":false,\
                     \"state\":\"skipped_dev_env\",\"reason\":{}}}",
                    jstr(&e)
                );
                std::process::exit(0);
            }
        };
        match red_arm_tamper(&dev) {
            Ok(detail) => {
                eprintln!("{TAG}: red-arm tamper 检出 — {detail}");
                println!(
                    "{{\"schema\":\"rurix.g27hzb.red_arm.v1\",\"arm\":\"tamper\",\"detected\":true,\"detail\":{}}}",
                    jstr(&detail)
                );
                std::process::exit(0);
            }
            Err(e) => fail(&format!("red-arm tamper 失效(漏检): {e}")),
        }
    }

    // ── 全档验证(双约定逐臂;判据 ①~⑤ 全零容差)──
    let dev = match device_arm(&args) {
        Ok(d) => d,
        Err(e) => {
            println!(
                "{{\"schema\":\"rurix.g27hzb.harness.v1\",\"mode\":\"device\",\
                 \"state\":\"skipped_dev_env\",\"skip_reason\":{}}}",
                jstr(&e)
            );
            return;
        }
    };
    let mut problems: Vec<String> = Vec::new();
    let mut arms_json: Vec<String> = Vec::new();
    for conv in [DepthConvention::ReverseZ, DepthConvention::StandardZ] {
        let r = run_device_arm(&dev, conv);
        if !r.mips_bitexact {
            problems.push(format!("{} mips 非逐级位级相等(①零容差)", r.conv));
        }
        if !r.verdict_equal {
            problems.push(format!("{} 判定序列与 host 非逐 rect 全等(②)", r.conv));
        }
        if r.false_positives > 0 {
            problems.push(format!("{} 假阳性 {}(③硬不变量)", r.conv, r.false_positives));
        }
        if !r.double_run_bitexact {
            problems.push(format!("{} device 双跑非位级一致(④)", r.conv));
        }
        if r.occluded == 0 {
            problems.push(format!("{} 剔除数为零(⑤)", r.conv));
        }
        eprintln!(
            "{TAG}: {} mips={} bitexact={} verdict_eq={} occluded={} fp={} double_run={}",
            r.conv,
            r.mips_len,
            r.mips_bitexact,
            r.verdict_equal,
            r.occluded,
            r.false_positives,
            r.double_run_bitexact
        );
        arms_json.push(r.to_json());
    }
    let state = if problems.is_empty() { "pass" } else { "fail" };
    println!(
        "{{\"schema\":\"rurix.g27hzb.harness.v1\",\"mode\":\"device\",\"state\":{},\
         \"problems\":{},\"resolution\":[{W},{H}],\"rects\":{RECTS},\"arms\":[{}],\
         \"base_commit\":{}}}",
        jstr(state),
        strs_json(&problems),
        arms_json.join(","),
        jstr(&base_commit()),
    );
    if !problems.is_empty() {
        std::process::exit(1);
    }
}
