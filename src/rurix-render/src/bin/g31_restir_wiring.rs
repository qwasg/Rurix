// Assisted-by: Kimi-K3（G31+ 波 B Task B2 ReSTIR 高档 reservoir 车道集成）
//! G31+ 波 B Task B2 ReSTIR 高档 reservoir 车道集成 harness（门
//! `g31.waveB.restir`；G30 收官承接锚 M100-high 行 `g31_anchor: M100 车道集成窗`
//! 第三件兑现面；RFC-0038 out-of-scope 锚的车道集成项）。
//!
//! ## 集成路径（加性纪律）
//!
//! **接入点 = 生产管线 direct GI pass 现有灯光表**：bistro-interior 契约
//! lighting JSON（milestones/g13/g13_ue_upscale_parity_contract.json）的
//! `point_lights`（与 g14_3_lane_body 装配面同口径：I_rgb = color_linear_rgb
//! × intensity_cd；标量投影 intensity = (r+g+b)/3 喂入标量 reservoir 模型）
//! → host 随机带单源预生成（已对齐消费序双带 + offset 三元组表，RFC-0045
//! §1.2 同律）→ `kernels/g28_restir.rx`（**本体 0-byte**）经
//! `rurix_rt::vk::run_compute` 单 dispatch [n_trials,1,1] 真跑——多灯场景下
//! reservoir 采样作为高档车道接入多灯采样链。kernel 的 target_phat 逐字面
//! 钉死着色点原点/法线 [0,1,0] ⇒ device 上传**着色点局部系**灯表
//!（pos−shade_pos 平移，p̂ 位级不变量；逐像素局部系 = 生产 reservoir 采样
//! pass 的逐着色点求值语义），host 参考臂维持世界系逐字。
//!
//! - `--restir off`（**默认档**）= 低档 MegaLights 语义车道镜像：RIS
//!   m_candidates=1 与 `estimate_uniform` 代数恒等（M=1 时 u < w/w_sum = 1
//!   恒真 ⇒ y = cand、W = L、estimate = p̂(y)·L），同一 kernel 同一灯表
//!   真跑；低档生产面 `gi/multi_light.rs` 默认档语义 **0-byte 不破坏**。
//! - `--restir on` = 高档 reservoir 车道：m_candidates=16 WRS/RIS 链 +
//!   空间重用加性臂（8×8 着色点网格 gather 合并前快照 → von Neumann 4 邻接
//!   字面固定序 → 受点重评快照变换后直调冻结 merge，m_cap=60——RFC-0045 §2
//!   同律，禁第二实现）。
//! - `--compare` = 双臂真跑 + 双跑位级 + 计时循环 → measured 对照单行 JSON。
//!
//! ## 冻结面（机核归 CI）
//!
//! `kernels/g28_restir.rx`（vs g28-closed）+ `gi/restir_reservoir.rs` +
//! `gi/multi_light.rs`（vs g27-closed）三处 0-byte；host 金标准
//! `estimate_ris`/`update`/`merge`/`unbiased_weight`/`target_phat`/
//! `exact_direct` 只消费不改写。
//!
//! ## 判据面
//!
//! - 逐 trial 保留样本 y device vs host 全等（整数锚真实承重）+ 判定带消费
//!   计数全等（钉死夹具平凡化事实照登恒跑）；
//! - device vs host 逐 trial estimate 绝对差 p100 ≤ 冻结容差口径（G28 标定
//!   冻结带 g28.restir_device.host_device_estimate_tol 由 CI 经 --tol 传入）；
//! - 无偏 3σ 维持：双臂 device estimate 均值 vs `exact_direct` 解析参考；
//! - 方差对照 measured：var(off)/var(on) 如实登记（on < off 方向硬门，数值
//!   不设伪造通过线）；dispatch 墙钟 ms 双臂 measured 登记（G6 无硬门纪律）；
//! - 固定 seed 双臂双跑输出 digest 位级一致（RXS-0357 L2 同律）。
//!
//! ## 三态
//!
//! 无 Vulkan loader/设备 → `skipped_dev_env` JSON 退 0（非 fake pass；
//! `RURIX_REQUIRE_REAL=1` 下 SKIP→硬红由 smoke 脚本层裁决）；--host-only 纯
//! host 恒跑；判据不符 ⇒ FAIL 退 1。
//!
//! ## 用法
//!
//! ```text
//! g31_restir_wiring --restir off --spv <k.spv> --tol <F> [--trials N] [--out <path>]
//! g31_restir_wiring --restir on  --spv <k.spv> --tol <F> [--trials N] [--spatial] [--out <path>]
//! g31_restir_wiring --compare --spv <k.spv> --tol <F> [--trials N] [--timing-runs K] [--out <path>]
//! g31_restir_wiring --host-only [--out <path>]
//! ```

#![forbid(unsafe_code)]

use rurix_render::gi::restir_reservoir::{
    Pcg32, PointLight, Reservoir, ShadePoint, estimate_ris, estimate_uniform, exact_direct,
    target_phat,
};
use rurix_rt::vk;

const TAG: &str = "[g31_restir_wiring]";
/// 车道集成夹具字面（独立于 G28 SEED 与 M100_SEED，避免跨模块流耦合）。
const SEED: u64 = 0xB261_0007_2026_0825;
/// 车道窗长（与 G28 锚同格 20000 trial）。
const N_TRIALS: u32 = 20_000;
/// 高档车道候选数（RFC-0045 §1 同格）。
const M_ON: u32 = 16;
/// 默认档候选数（MegaLights 式均匀选灯 = RIS M=1 代数恒等面）。
const M_OFF: u32 = 1;
/// 空间臂（RFC-0045 §2 同律）：8×8 网格闭集 + M-cap 60。
const GRID_N: usize = 8;
const N_POINTS: usize = GRID_N * GRID_N;
const M_CAP: u32 = 60;
const N_TRIALS_SPATIAL: u32 = 20_000;
/// von Neumann 4-邻接字面固定序（RFC-0045 §2.2 逐字）。
const NEIGHBOR_ORDER: [(i64, i64); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
/// 生产管线 direct GI pass 灯表契约面（bistro-interior 行 point_lights）。
const CONTRACT_PATH: &str = "milestones/g13/g13_ue_upscale_parity_contract.json";
const SCENE_ID: &str = "bistro-interior";
/// 车道着色点（bistro 室内代表点；四灯 y≈3 全在上半球 ⇒ p̂>0 全支撑）。
const SHADE: ShadePoint = ShadePoint {
    pos: [5.0, 1.0, -4.0],
    normal: [0.0, 1.0, 0.0],
};
/// 计时循环默认 dispatch 次数（2 次位级跑兼任 warmup）。
const TIMING_RUNS: u32 = 20;

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

// ---------------------------------------------------------------------------
// 最小 JSON 解析（bin-local 独立实现；g14_3_lane_body 同型复制子集——
// 重复键拒/控制字符拒/深度限 64）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Num(v) => Some(*v),
            _ => None,
        }
    }
    fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }
    fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(a) => Some(a),
            _ => None,
        }
    }
}

struct JParser<'a> {
    b: &'a [u8],
    i: usize,
    depth: usize,
}

impl<'a> JParser<'a> {
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn expect(&mut self, c: u8) -> Result<(), String> {
        self.ws();
        if self.i < self.b.len() && self.b[self.i] == c {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("JSON: 期待 '{}' @{}", c as char, self.i))
        }
    }

    fn value(&mut self) -> Result<Json, String> {
        if self.depth >= 64 {
            return Err("JSON: 嵌套深度越 64".into());
        }
        self.ws();
        let Some(&c) = self.b.get(self.i) else {
            return Err("JSON: 意外结尾".into());
        };
        match c {
            b'{' => {
                self.i += 1;
                self.depth += 1;
                let mut pairs: Vec<(String, Json)> = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    self.depth -= 1;
                    return Ok(Json::Obj(pairs));
                }
                loop {
                    self.ws();
                    let k = self.string()?;
                    if pairs.iter().any(|(ek, _)| ek == &k) {
                        return Err(format!("JSON: 重复键 {k}"));
                    }
                    self.expect(b':')?;
                    let v = self.value()?;
                    pairs.push((k, v));
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            break;
                        }
                        _ => return Err("JSON: 对象缺 ,/}".into()),
                    }
                }
                self.depth -= 1;
                Ok(Json::Obj(pairs))
            }
            b'[' => {
                self.i += 1;
                self.depth += 1;
                let mut items = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b']') {
                    self.i += 1;
                    self.depth -= 1;
                    return Ok(Json::Arr(items));
                }
                loop {
                    items.push(self.value()?);
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            break;
                        }
                        _ => return Err("JSON: 数组缺 ,/]".into()),
                    }
                }
                self.depth -= 1;
                Ok(Json::Arr(items))
            }
            b'"' => Ok(Json::Str(self.string()?)),
            b't' => self.lit("true", Json::Bool(true)),
            b'f' => self.lit("false", Json::Bool(false)),
            b'n' => self.lit("null", Json::Null),
            _ => self.number(),
        }
    }

    fn lit(&mut self, s: &str, v: Json) -> Result<Json, String> {
        if self.b[self.i..].starts_with(s.as_bytes()) {
            self.i += s.len();
            Ok(v)
        } else {
            Err(format!("JSON: 字面 {s} 不符 @{}", self.i))
        }
    }

    fn string(&mut self) -> Result<String, String> {
        if self.b.get(self.i) != Some(&b'"') {
            return Err(format!("JSON: 期待字符串 @{}", self.i));
        }
        self.i += 1;
        let mut out = String::new();
        loop {
            let Some(&c) = self.b.get(self.i) else {
                return Err("JSON: 字符串未闭合".into());
            };
            self.i += 1;
            match c {
                b'"' => return Ok(out),
                b'\\' => {
                    let Some(&e) = self.b.get(self.i) else {
                        return Err("JSON: 转义未闭合".into());
                    };
                    self.i += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hi = self.hex4()?;
                            let cp = if (0xD800..0xDC00).contains(&hi) {
                                if self.b.get(self.i) == Some(&b'\\')
                                    && self.b.get(self.i + 1) == Some(&b'u')
                                {
                                    self.i += 2;
                                    let lo = self.hex4()?;
                                    if !(0xDC00..0xE000).contains(&lo) {
                                        return Err("JSON: 低代理越域".into());
                                    }
                                    0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                                } else {
                                    return Err("JSON: 孤高代理".into());
                                }
                            } else if (0xDC00..0xE000).contains(&hi) {
                                return Err("JSON: 孤低代理".into());
                            } else {
                                hi
                            };
                            let ch = char::from_u32(cp).ok_or("JSON: \\u 码点越域")?;
                            out.push(ch);
                        }
                        _ => return Err("JSON: 非法转义".into()),
                    }
                }
                0x00..=0x1F => return Err("JSON: 未转义控制字符".into()),
                _ => {
                    let s = std::str::from_utf8(&self.b[self.i - 1..]).map_err(|_| "JSON: UTF-8")?;
                    let ch = s.chars().next().ok_or("JSON: 字符串截断")?;
                    out.push(ch);
                    self.i += ch.len_utf8() - 1;
                }
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, String> {
        if self.i + 4 > self.b.len() {
            return Err("JSON: \\u 截断".into());
        }
        let s = std::str::from_utf8(&self.b[self.i..self.i + 4]).map_err(|_| "JSON: \\u 非 hex")?;
        let v = u32::from_str_radix(s, 16).map_err(|_| "JSON: \\u 非 hex")?;
        self.i += 4;
        Ok(v)
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.i;
        if self.b.get(self.i) == Some(&b'-') {
            self.i += 1;
        }
        let mut saw_digit = false;
        while self.i < self.b.len() && self.b[self.i].is_ascii_digit() {
            self.i += 1;
            saw_digit = true;
        }
        if !saw_digit {
            return Err(format!("JSON: 非法数字 @{start}"));
        }
        if self.b.get(self.i) == Some(&b'.') {
            self.i += 1;
            if !self.b.get(self.i).is_some_and(|c| c.is_ascii_digit()) {
                return Err("JSON: 小数点缺位".into());
            }
            while self.i < self.b.len() && self.b[self.i].is_ascii_digit() {
                self.i += 1;
            }
        }
        if matches!(self.b.get(self.i), Some(b'e') | Some(b'E')) {
            self.i += 1;
            if matches!(self.b.get(self.i), Some(b'+') | Some(b'-')) {
                self.i += 1;
            }
            if !self.b.get(self.i).is_some_and(|c| c.is_ascii_digit()) {
                return Err("JSON: 指数缺位".into());
            }
            while self.i < self.b.len() && self.b[self.i].is_ascii_digit() {
                self.i += 1;
            }
        }
        let raw = std::str::from_utf8(&self.b[start..self.i]).map_err(|_| "JSON: 数字 UTF-8")?;
        let v: f64 = raw.parse().map_err(|_| format!("JSON: 数字解析 {raw}"))?;
        if !v.is_finite() {
            return Err(format!("JSON: 数字 {raw} 非有限"));
        }
        Ok(Json::Num(v))
    }
}

fn json_parse(text: &str) -> Result<Json, String> {
    let mut p = JParser {
        b: text.as_bytes(),
        i: 0,
        depth: 0,
    };
    let v = p.value()?;
    p.ws();
    if p.i != p.b.len() {
        return Err("JSON: 尾部余留字节".into());
    }
    Ok(v)
}

// ---------------------------------------------------------------------------
// 契约 lighting → reservoir 灯表（生产管线 direct GI pass 现有灯表同口径：
// I_rgb = color_linear_rgb × intensity_cd；标量投影 = (r+g+b)/3）
// ---------------------------------------------------------------------------

fn bistro_point_lights(path: &str) -> Result<Vec<PointLight>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("读契约 {path}: {e}"))?;
    let doc = json_parse(&text)?;
    let scenes = doc
        .get("scenes")
        .and_then(|v| v.as_array())
        .ok_or("契约缺 scenes 数组")?;
    let row = scenes
        .iter()
        .find(|s| {
            s.get("scene_id")
                .and_then(|v| v.as_str())
                .map(|x| x == SCENE_ID)
                .unwrap_or(false)
        })
        .ok_or_else(|| format!("契约缺场景行 {SCENE_ID}"))?;
    let pts = row
        .get("lighting")
        .and_then(|l| l.get("point_lights"))
        .and_then(|v| v.as_array())
        .ok_or("契约缺 lighting.point_lights")?;
    if pts.is_empty() {
        return Err("point_lights 空（多灯契约面不成立）".into());
    }
    let f3 = |v: Option<&Json>, name: &str| -> Result<[f32; 3], String> {
        let a = v
            .and_then(|x| x.as_array())
            .ok_or_else(|| format!("{name} 非数组"))?;
        if a.len() != 3 {
            return Err(format!("{name} 长度 {} ≠ 3", a.len()));
        }
        let mut out = [0.0f32; 3];
        for (k, x) in a.iter().enumerate() {
            out[k] = x
                .as_f64()
                .ok_or_else(|| format!("{name}[{k}] 非数值"))? as f32;
        }
        Ok(out)
    };
    let mut lights = Vec::with_capacity(pts.len());
    for (i, p) in pts.iter().enumerate() {
        let pos = f3(p.get("position"), &format!("point_lights[{i}].position"))?;
        let col = f3(
            p.get("color_linear_rgb"),
            &format!("point_lights[{i}].color_linear_rgb"),
        )?;
        let inten = p
            .get("intensity_cd")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| format!("point_lights[{i}].intensity_cd 非数值"))?;
        // 标量投影口径：I_rgb 三通道均值（寄存映射文档化；host/device 同表同值）。
        let intensity = ((col[0] as f64 * inten + col[1] as f64 * inten + col[2] as f64 * inten)
            / 3.0) as f32;
        if !(intensity.is_finite() && intensity > 0.0) {
            return Err(format!("point_lights[{i}] 标量强度非正有限"));
        }
        lights.push(PointLight { pos, intensity });
    }
    Ok(lights)
}

// ---------------------------------------------------------------------------
// 字节/digest 助手（g28_restir_device 同模）
// ---------------------------------------------------------------------------

fn lights_flat(lights: &[PointLight]) -> Vec<f32> {
    lights
        .iter()
        .flat_map(|l| [l.pos[0], l.pos[1], l.pos[2], l.intensity])
        .collect()
}

/// 世界系灯表 → 着色点局部系（g28_restir.rx 的 target_phat 逐字面钉死
/// sp.pos=[0,0,0]/normal=[0,1,0]——kernel 0-byte 纪律下的接线语义 =
/// 逐着色点局部系求值；平移后 d = (pos−sp.pos)−0 与 host
/// `target_phat(SHADE, light)` 的 f32 减序位级相等，p̂ 不变量）。
fn lights_local_frame(lights: &[PointLight], sp: &ShadePoint) -> Vec<PointLight> {
    lights
        .iter()
        .map(|l| PointLight {
            pos: [
                l.pos[0] - sp.pos[0],
                l.pos[1] - sp.pos[1],
                l.pos[2] - sp.pos[2],
            ],
            intensity: l.intensity,
        })
        .collect()
}

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

fn sha256_f32(v: &[f32]) -> String {
    let mut bytes = Vec::with_capacity(v.len() * 4);
    for &x in v {
        bytes.extend_from_slice(&x.to_le_bytes());
    }
    rurix_pkg::sha256::hex_digest(&bytes)
}

fn sha256_f64(v: &[f64]) -> String {
    let mut bytes = Vec::with_capacity(v.len() * 8);
    for &x in v {
        bytes.extend_from_slice(&x.to_le_bytes());
    }
    rurix_pkg::sha256::hex_digest(&bytes)
}

/// 顺序 f64 均值/样本方差（RFC-0045 §5.7 同律：统计聚合 host f64 顺序累加）。
fn mean_var(vals: &[f64]) -> (f64, f64) {
    let n = vals.len();
    let mut sum = 0.0f64;
    for &v in vals {
        sum += v;
    }
    let mean = sum / n as f64;
    let mut var_sum = 0.0f64;
    for &v in vals {
        var_sum += (v - mean) * (v - mean);
    }
    (mean, var_sum / (n - 1) as f64)
}

// ---------------------------------------------------------------------------
// 随机带录制器（RFC-0045 §1.2 字面同律，m_candidates 参数化：
// 冻结 update 本体驱动 + Pcg32 Copy 快照重放；录制自检锚 F2 = 录制终态 vs
// estimate_ris 直调终态逐 trial 位级相等）
// ---------------------------------------------------------------------------

struct Bands {
    cand: Vec<f32>,
    dec: Vec<f32>,
    offsets: Vec<f32>,
    host_est: Vec<f64>,
    host_y: Vec<usize>,
    host_dec_len: Vec<u32>,
}

fn record_bands(lights: &[PointLight], sp: &ShadePoint, n_trials: u32, m_cand: u32) -> Bands {
    let n = lights.len();
    let mut b = Bands {
        cand: Vec::with_capacity(n_trials as usize * m_cand as usize),
        dec: Vec::with_capacity(n_trials as usize * m_cand as usize),
        offsets: Vec::with_capacity(n_trials as usize * 3),
        host_est: Vec::with_capacity(n_trials as usize),
        host_y: Vec::with_capacity(n_trials as usize),
        host_dec_len: Vec::with_capacity(n_trials as usize),
    };
    for t in 0..n_trials {
        // RIS 流字面：stream = t·4+1（与 G28 夹具同布局，本车道独立 SEED）。
        let mut rng = Pcg32::new(SEED, u64::from(t) * 4 + 1);
        let mut r = Reservoir::empty();
        let cand_offset = b.cand.len();
        let dec_offset = b.dec.len();
        for _ in 0..m_cand {
            // 候选抽取与 w 提升两行字面同源复写（RFC-0045 §1.2 允许面）。
            let cand = (rng.next_u32() as usize) % n;
            let phat = target_phat(sp, &lights[cand]);
            let w = f64::from(phat) * n as f64;
            let pre = rng; // Pcg32 Copy 快照（update 前）
            r.update(cand, phat, w, &mut rng); // 冻结 update 本体驱动
            if r.w_sum > 0.0 {
                // 消费判定事实 = update 后 w_sum>0；消费值 = 快照重放 next_f32。
                let mut replay = pre;
                b.dec.push(replay.next_f32());
            }
            b.cand.push(cand as f32);
        }
        let dec_len = b.dec.len() - dec_offset;
        // ── 录制自检锚（F2）：录制终态 vs estimate_ris 直调终态逐 trial 位级 ──
        let mut rng_ref = Pcg32::new(SEED, u64::from(t) * 4 + 1);
        let (est_ref, r_ref) = estimate_ris(sp, lights, m_cand, &mut rng_ref);
        if r.y != r_ref.y
            || r.phat_y.to_bits() != r_ref.phat_y.to_bits()
            || r.w_sum.to_bits() != r_ref.w_sum.to_bits()
            || r.m != r_ref.m
        {
            fail(&format!(
                "录制自检锚失败 trial {t}（m={m_cand}）：录制终态 ≠ estimate_ris 直调终态（位级）"
            ));
        }
        b.offsets
            .extend_from_slice(&[cand_offset as f32, dec_offset as f32, dec_len as f32]);
        b.host_est.push(est_ref);
        b.host_y.push(r_ref.y);
        b.host_dec_len.push(dec_len as u32);
    }
    b
}

// ---------------------------------------------------------------------------
// device 臂（kernels/g28_restir.rx 0-byte 复用；单 dispatch [n_trials,1,1]）
// ---------------------------------------------------------------------------

/// kernel 参数面打包（与 g28_restir.rx 参数面逐字同源；8 f32 位级编码）。
fn pack_params(n_trials: u32, n_lights: u32, m_cand: u32) -> Vec<f32> {
    let mut v = vec![
        n_trials as f32,
        n_lights as f32,
        m_cand as f32,
        0.0, // red_bias：本车道零偏置（RED 臂归 G28 门，本面不注入）
    ];
    v.resize(8, 0.0);
    v
}

struct DeviceLane {
    spv: Vec<u32>,
    entry: String,
}

impl DeviceLane {
    fn create(spv: Vec<u32>) -> Result<Self, String> {
        if !vk::vulkan_available() {
            return Err("vulkan loader 不可用".into());
        }
        let entry = vk::entry_point_name(&spv).ok_or("SPV 无 OpEntryPoint")?;
        Ok(Self { spv, entry })
    }

    /// 单 dispatch 全 trial；返回输出缓冲（4 f32/trial
    /// [estimate, y(−1=空), dec_consumed, phat_y]）。
    fn run(&self, lights: &[PointLight], bands: &Bands, n_trials: u32, m_cand: u32) -> Vec<f32> {
        let params = pack_params(n_trials, lights.len() as u32, m_cand);
        let mut bufs = vec![
            bytes_f32(&lights_flat(lights)),
            bytes_f32(&bands.cand),
            bytes_f32(&bands.dec),
            bytes_f32(&bands.offsets),
            bytes_f32(&params),
            vec![0u8; n_trials as usize * 16],
        ];
        vk::run_compute(&self.spv, &self.entry, &mut bufs, &[], [n_trials, 1, 1])
            .unwrap_or_else(|e| panic!("restir 车道 dispatch 失败: {e}"));
        read_f32(&bufs[5])
    }
}

// ---------------------------------------------------------------------------
// 判据（整数锚 / estimate p100 / 无偏 3σ+方差；g28_restir_device 同模参数化）
// ---------------------------------------------------------------------------

fn y_matches(dev_y: f32, host_y: usize) -> bool {
    if host_y == usize::MAX {
        dev_y == -1.0
    } else {
        dev_y == host_y as f32
    }
}

struct IntegerAnchor {
    y_all_equal: bool,
    dec_all_equal: bool,
    dec_constant: bool,
    first_mismatch: Option<usize>,
}

fn check_integer_anchor(out: &[f32], bands: &Bands, n_trials: u32, m_cand: u32) -> IntegerAnchor {
    let mut a = IntegerAnchor {
        y_all_equal: true,
        dec_all_equal: true,
        dec_constant: true,
        first_mismatch: None,
    };
    for t in 0..n_trials as usize {
        let dev_y = out[t * 4 + 1];
        let dev_dec = out[t * 4 + 2];
        if !y_matches(dev_y, bands.host_y[t]) {
            a.y_all_equal = false;
            if a.first_mismatch.is_none() {
                a.first_mismatch = Some(t);
            }
        }
        if dev_dec != bands.host_dec_len[t] as f32 {
            a.dec_all_equal = false;
            if a.first_mismatch.is_none() {
                a.first_mismatch = Some(t);
            }
        }
        if bands.host_dec_len[t] != m_cand {
            a.dec_constant = false;
        }
    }
    a
}

/// 逐 trial estimate 绝对差 p100（device f32 提升 f64 vs host f64 直调参考）。
fn estimate_p100(out: &[f32], host_est: &[f64], n_trials: u32) -> f64 {
    let mut p100 = 0.0f64;
    for t in 0..n_trials as usize {
        let d = (f64::from(out[t * 4]) - host_est[t]).abs();
        if d > p100 {
            p100 = d;
        }
    }
    p100
}

/// 无偏 3σ + 逐 trial 方差：返回 (pass, mean, var, dev, bound)。
fn unbiased_3sigma_var(out: &[f32], reference: f64, n_trials: u32) -> (bool, f64, f64, f64, f64) {
    let n = n_trials as usize;
    let mut sum = 0.0f64;
    for t in 0..n {
        sum += f64::from(out[t * 4]);
    }
    let mean = sum / n as f64;
    let mut var_sum = 0.0f64;
    for t in 0..n {
        let d = f64::from(out[t * 4]) - mean;
        var_sum += d * d;
    }
    let var = var_sum / (n - 1) as f64;
    let sigma_mean = (var / n as f64).sqrt();
    let dev = (mean - reference).abs();
    let bound = 3.0 * sigma_mean;
    (dev < bound + 1e-9, mean, var, dev, bound)
}

// ---------------------------------------------------------------------------
// 空间重用加性臂（纯 host；RFC-0045 §2 同律：受点重评快照变换后直调冻结
// merge，禁第二实现；网格 = bistro 室内 y=1.0 平面 8×8 闭集）
// ---------------------------------------------------------------------------

/// 8×8 网格闭集（行主序 p = i·8+j；x ∈ [2,8]，z ∈ [−8,0]，y=1.0，法线 +y；
/// 四灯 y≈3 全在上半球 ⇒ 逐点全灯 p̂>0 全支撑）。
fn spatial_points() -> Vec<ShadePoint> {
    let mut pts = Vec::with_capacity(N_POINTS);
    for i in 0..GRID_N {
        for j in 0..GRID_N {
            pts.push(ShadePoint {
                pos: [
                    2.0 + 6.0 * i as f32 / (GRID_N - 1) as f32,
                    1.0,
                    -8.0 + 8.0 * j as f32 / (GRID_N - 1) as f32,
                ],
                normal: [0.0, 1.0, 0.0],
            });
        }
    }
    pts
}

/// 一次空间臂全跑：返回 (reuse 矩阵, no-reuse 矩阵)，行主序 [t·64+p] f64。
fn run_spatial(lights: &[PointLight], pts: &[ShadePoint], n_trials: u32) -> (Vec<f64>, Vec<f64>) {
    let n = n_trials as usize;
    let mut reuse = vec![0.0f64; n * N_POINTS];
    let mut noreuse = vec![0.0f64; n * N_POINTS];
    let mut snaps: Vec<Reservoir> = Vec::with_capacity(N_POINTS);
    let mut rngs: Vec<Pcg32> = Vec::with_capacity(N_POINTS);
    for t in 0..n {
        snaps.clear();
        rngs.clear();
        // ── 本点 RIS（流 = (t·64+p)·4+3；k=3 残差类与车道臂三流构造性不相交）；
        //    gather 合并前快照闭集（禁 in-place 链式污染）──
        for (p, sp) in pts.iter().enumerate() {
            let stream = (t as u64 * N_POINTS as u64 + p as u64) * 4 + 3;
            let mut rng = Pcg32::new(SEED, stream);
            let (est, r) = estimate_ris(sp, lights, M_ON, &mut rng);
            noreuse[t * N_POINTS + p] = est;
            snaps.push(r);
            rngs.push(rng);
        }
        // ── 受点重评快照变换后直调冻结 merge（邻域字面固定序；m_cap=60）──
        for p in 0..N_POINTS {
            let (pi, pj) = ((p / GRID_N) as i64, (p % GRID_N) as i64);
            let mut merged = snaps[p];
            let rng = &mut rngs[p];
            for (di, dj) in NEIGHBOR_ORDER {
                let (ni, nj) = (pi + di, pj + dj);
                if ni < 0 || ni >= GRID_N as i64 || nj < 0 || nj >= GRID_N as i64 {
                    continue;
                }
                let q = (ni as usize) * GRID_N + nj as usize;
                let other = &snaps[q];
                if other.y == usize::MAX {
                    merged.merge(other, rng, M_CAP);
                    continue;
                }
                // 受点重评快照变换（RFC-0045 §2.2 F5 同律）；W_other =
                // other.unbiased_weight() 冻结 API 直调。
                let w_other = other.unbiased_weight();
                let phat_recv = target_phat(&pts[p], &lights[other.y]);
                let other_prime = Reservoir {
                    y: other.y,
                    phat_y: phat_recv,
                    w_sum: f64::from(phat_recv) * w_other * f64::from(other.m),
                    m: other.m,
                };
                merged.merge(&other_prime, rng, M_CAP);
            }
            reuse[t * N_POINTS + p] = f64::from(merged.phat_y) * merged.unbiased_weight();
        }
    }
    (reuse, noreuse)
}

struct SpatialSummary {
    aggregate_pass: bool,
    agg_mean: f64,
    ref_grid: f64,
    agg_dev: f64,
    agg_bound: f64,
    all_within_5: bool,
    worst_ratio: f64,
    within_3_count: usize,
    gain_min: f64,
    gain_mean: f64,
    gain_max: f64,
    bitexact: bool,
    digest: String,
    single_run_seconds: f64,
}

fn spatial_arm(lights: &[PointLight], n_trials: u32) -> SpatialSummary {
    let pts = spatial_points();
    let n = n_trials as usize;
    let refs: Vec<f64> = pts.iter().map(|p| exact_direct(p, lights)).collect();
    let mut ref_grid = 0.0f64;
    for &r in &refs {
        ref_grid += r;
    }
    ref_grid /= N_POINTS as f64;

    let t0 = std::time::Instant::now();
    let (reuse_a, noreuse_a) = run_spatial(lights, &pts, n_trials);
    let single_run_seconds = t0.elapsed().as_secs_f64();
    let (reuse_b, noreuse_b) = run_spatial(lights, &pts, n_trials);
    let digest_a = format!("{}:{}", sha256_f64(&reuse_a), sha256_f64(&noreuse_a));
    let digest_b = format!("{}:{}", sha256_f64(&reuse_b), sha256_f64(&noreuse_b));
    let bitexact = digest_a == digest_b;

    let mut grid_means = Vec::with_capacity(n);
    for t in 0..n {
        let mut s = 0.0f64;
        for p in 0..N_POINTS {
            s += reuse_a[t * N_POINTS + p];
        }
        grid_means.push(s / N_POINTS as f64);
    }
    let (agg_mean, agg_var) = mean_var(&grid_means);
    let agg_sigma_mean = (agg_var / n as f64).sqrt();
    let agg_dev = (agg_mean - ref_grid).abs();
    let agg_bound = 3.0 * agg_sigma_mean;
    let aggregate_pass = agg_dev < agg_bound + 1e-9;

    let mut all_within_5 = true;
    let mut within_3_count = 0usize;
    let mut worst_ratio = 0.0f64;
    let mut gain_min = f64::INFINITY;
    let mut gain_max = f64::NEG_INFINITY;
    let mut gain_sum = 0.0f64;
    for p in 0..N_POINTS {
        let series_reuse: Vec<f64> = (0..n).map(|t| reuse_a[t * N_POINTS + p]).collect();
        let series_noreuse: Vec<f64> = (0..n).map(|t| noreuse_a[t * N_POINTS + p]).collect();
        let (m_r, v_r) = mean_var(&series_reuse);
        let (_m_n, v_n) = mean_var(&series_noreuse);
        let sigma_mean = (v_r / n as f64).sqrt();
        let dev = (m_r - refs[p]).abs();
        if dev < 3.0 * sigma_mean + 1e-9 {
            within_3_count += 1;
        }
        if !(dev < 5.0 * sigma_mean + 1e-9) {
            all_within_5 = false;
        }
        let ratio = if sigma_mean > 0.0 { dev / sigma_mean } else { 0.0 };
        if ratio > worst_ratio {
            worst_ratio = ratio;
        }
        let gain = v_n / v_r.max(1e-30);
        gain_min = gain_min.min(gain);
        gain_max = gain_max.max(gain);
        gain_sum += gain;
    }

    SpatialSummary {
        aggregate_pass,
        agg_mean,
        ref_grid,
        agg_dev,
        agg_bound,
        all_within_5,
        worst_ratio,
        within_3_count,
        gain_min,
        gain_mean: gain_sum / N_POINTS as f64,
        gain_max,
        bitexact,
        digest: digest_a,
        single_run_seconds,
    }
}

// ---------------------------------------------------------------------------
// 车道臂（off/on 同形）：录制带（F2 自检恒跑）→ device 双跑位级 → 整数锚 →
// p100 → 无偏 3σ+方差 → 计时循环
// ---------------------------------------------------------------------------

struct ArmReport {
    m_candidates: u32,
    mean: f64,
    reference: f64,
    variance: f64,
    dev: f64,
    bound: f64,
    unbiased_pass: bool,
    y_all_equal: bool,
    dec_all_equal: bool,
    dec_constant: bool,
    p100: f64,
    in_tol: bool,
    digest: String,
    bitexact: bool,
    dispatch_ms: Vec<f64>,
    problems: Vec<String>,
}

fn run_arm(
    dev: &DeviceLane,
    lights: &[PointLight],
    n_trials: u32,
    m_cand: u32,
    tol: f64,
    timing_runs: u32,
) -> ArmReport {
    let reference = exact_direct(&SHADE, lights);
    let bands = record_bands(lights, &SHADE, n_trials, m_cand);
    // kernel shade 点钉死原点 ⇒ device 上传着色点局部系灯表（p̂ 位级不变量，
    // 见 lights_local_frame）；host 参考臂/录制带维持世界系逐字。
    let lights_dev = lights_local_frame(lights, &SHADE);
    let mut problems: Vec<String> = Vec::new();
    let out_a = dev.run(&lights_dev, &bands, n_trials, m_cand);
    // ① 前置整数锚（y 真实承重 + 消费计数平凡化恒跑）。
    let anchor = check_integer_anchor(&out_a, &bands, n_trials, m_cand);
    if !anchor.y_all_equal {
        problems.push(format!(
            "y 整数锚失败（首失配 trial {:?}）",
            anchor.first_mismatch
        ));
    }
    if !anchor.dec_all_equal {
        problems.push(format!(
            "判定带消费计数锚失败（首失配 trial {:?}）",
            anchor.first_mismatch
        ));
    }
    // ② 逐 trial estimate p100 ≤ 冻结容差（--tol 由 CI 自 g28 冻结带传入）。
    let p100 = estimate_p100(&out_a, &bands.host_est, n_trials);
    let in_tol = p100 <= tol;
    if anchor.y_all_equal && anchor.dec_all_equal && !in_tol {
        problems.push(format!("estimate p100={p100:.6e} 超容差 {tol:.6e}"));
    }
    // ③ 无偏 3σ 维持 + 逐 trial 方差（variance 对照面）。
    let (unbiased, mean, var, dev_3s, bound) = unbiased_3sigma_var(&out_a, reference, n_trials);
    if !unbiased {
        problems.push(format!(
            "无偏 3σ 失败：mean={mean:.9} ref={reference:.9} dev={dev_3s:.3e} > bound={bound:.3e}"
        ));
    }
    // ④ device 双跑位级一致（输出缓冲 digest）。
    let out_b = dev.run(&lights_dev, &bands, n_trials, m_cand);
    let digest_a = sha256_f32(&out_a);
    let digest_b = sha256_f32(&out_b);
    let bitexact = digest_a == digest_b;
    if !bitexact {
        problems.push("device 双跑非位级一致".into());
    }
    // ⑤ 计时循环（dispatch+回读墙钟；两跑位级兼任 warmup；如实登记不设通过线）。
    let mut dispatch_ms = Vec::with_capacity(timing_runs as usize);
    for _ in 0..timing_runs {
        let t0 = std::time::Instant::now();
        let _ = dev.run(&lights_dev, &bands, n_trials, m_cand);
        dispatch_ms.push(t0.elapsed().as_secs_f64() * 1000.0);
    }
    ArmReport {
        m_candidates: m_cand,
        mean,
        reference,
        variance: var,
        dev: dev_3s,
        bound,
        unbiased_pass: unbiased,
        y_all_equal: anchor.y_all_equal,
        dec_all_equal: anchor.dec_all_equal,
        dec_constant: anchor.dec_constant,
        p100,
        in_tol,
        digest: digest_a,
        bitexact,
        dispatch_ms,
        problems,
    }
}

// ---------------------------------------------------------------------------
// JSON 出报（手写，零新依赖；g26/g28 同模）
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

fn ms_stats(v: &[f64]) -> (f64, f64, f64) {
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean = s.iter().sum::<f64>() / s.len() as f64;
    (mean, s[0], s[s.len() - 1])
}

fn arm_json(tier: &str, a: &ArmReport, tol: f64) -> String {
    let (ms_mean, ms_min, ms_max) = if a.dispatch_ms.is_empty() {
        (0.0, 0.0, 0.0)
    } else {
        ms_stats(&a.dispatch_ms)
    };
    format!(
        "{{\"tier\":{},\"m_candidates\":{},\"mean\":{:.15e},\"reference\":{:.15e},\"variance\":{:.15e},\"dev\":{:.9e},\"bound_3sigma\":{:.9e},\"unbiased_3sigma_pass\":{},\"y_anchor_all_equal\":{},\"dec_consumed_all_equal\":{},\"dec_consumed_constant_m\":{},\"p100_vs_host\":{:.15e},\"tol\":{:.15e},\"in_tol\":{},\"digest\":{},\"double_run_bitexact\":{},\"dispatch_ms\":{{\"mean\":{:.6},\"min\":{:.6},\"max\":{:.6},\"runs\":{}}},\"problems\":{}}}",
        jstr(tier),
        a.m_candidates,
        a.mean,
        a.reference,
        a.variance,
        a.dev,
        a.bound,
        a.unbiased_pass,
        a.y_all_equal,
        a.dec_all_equal,
        a.dec_constant,
        a.p100,
        tol,
        a.in_tol,
        jstr(&format!("sha256:{}", a.digest)),
        a.bitexact,
        ms_mean,
        ms_min,
        ms_max,
        a.dispatch_ms.len(),
        strs_json(&a.problems),
    )
}

fn spatial_json(s: &SpatialSummary) -> String {
    format!(
        "{{\"grid\":\"{GRID_N}x{GRID_N}\",\"n_points\":{N_POINTS},\"m_candidates\":{M_ON},\"m_cap\":{M_CAP},\"neighbor_order\":\"(-1,0)(+1,0)(0,-1)(0,+1)\",\"n_trials\":{N_TRIALS_SPATIAL},\"single_run_seconds\":{:.3},\"aggregate_3sigma\":{{\"mean\":{:.12e},\"reference\":{:.12e},\"dev\":{:.9e},\"bound_3sigma\":{:.9e},\"pass\":{}}},\"per_point_5sigma_all_within\":{},\"worst_dev_over_sigma\":{:.4},\"per_point_3sigma_within_count\":{},\"variance_gain\":{{\"min\":{:.6},\"mean\":{:.6},\"max\":{:.6},\"no_pass_line\":true}},\"double_run_bitexact\":{},\"digest\":{}}}",
        s.single_run_seconds,
        s.agg_mean,
        s.ref_grid,
        s.agg_dev,
        s.agg_bound,
        s.aggregate_pass,
        s.all_within_5,
        s.worst_ratio,
        s.within_3_count,
        s.gain_min,
        s.gain_mean,
        s.gain_max,
        s.bitexact,
        jstr(&format!("sha256:{}", s.digest)),
    )
}

fn emit_line(line: &str, out: &Option<String>) {
    println!("{line}");
    if let Some(path) = out {
        if let Some(parent) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(path, format!("{line}\n"))
            .unwrap_or_else(|e| fail(&format!("写 --out {path}: {e}")));
    }
}

fn skip_line(schema: &str, reason: &str) -> String {
    format!(
        "{{\"schema\":{},\"state\":\"skipped_dev_env\",\"reason\":{}}}",
        jstr(schema),
        jstr(reason)
    )
}

// ---------------------------------------------------------------------------
// 参数
// ---------------------------------------------------------------------------

struct Args {
    restir: String,
    compare: bool,
    host_only: bool,
    spatial: bool,
    spv: Option<String>,
    tol: f64,
    trials: u32,
    timing_runs: u32,
    contract: String,
    out: Option<String>,
}

fn parse_args() -> Args {
    let mut a = Args {
        restir: "off".into(),
        compare: false,
        host_only: false,
        spatial: false,
        spv: None,
        tol: 0.0,
        trials: N_TRIALS,
        timing_runs: TIMING_RUNS,
        contract: CONTRACT_PATH.into(),
        out: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        match k.as_str() {
            // --restir <off|on>：默认 off（低档 MegaLights 语义车道镜像）；
            // on = 高档 reservoir 车道（M=16 + 可选 --spatial 空间重用加性臂）。
            "--restir" => a.restir = it.next().unwrap_or_else(|| fail("缺 --restir 值")),
            "--compare" => a.compare = true,
            "--host-only" => a.host_only = true,
            "--spatial" => a.spatial = true,
            "--spv" => a.spv = it.next(),
            "--tol" => {
                a.tol = it
                    .next()
                    .unwrap_or_else(|| fail("缺 --tol 值"))
                    .parse()
                    .unwrap_or_else(|_| fail("--tol 非 f64"))
            }
            "--trials" => {
                a.trials = it
                    .next()
                    .unwrap_or_else(|| fail("缺 --trials 值"))
                    .parse()
                    .unwrap_or_else(|_| fail("--trials 非 u32"))
            }
            "--timing-runs" => {
                a.timing_runs = it
                    .next()
                    .unwrap_or_else(|| fail("缺 --timing-runs 值"))
                    .parse()
                    .unwrap_or_else(|_| fail("--timing-runs 非 u32"))
            }
            "--contract" => a.contract = it.next().unwrap_or_else(|| fail("缺 --contract 值")),
            "--out" => a.out = it.next(),
            other => fail(&format!("未知参数: {other}")),
        }
    }
    if a.restir != "off" && a.restir != "on" {
        fail(&format!(
            "--restir {}：只接受 off|on（off=低档 MegaLights 语义默认档；on=高档 reservoir 车道）",
            a.restir
        ));
    }
    a
}

// ---------------------------------------------------------------------------
// host 腿（纯 host 恒跑：契约灯表解析 + F2 录制自检 + host 直调 3σ +
// M=1 ≡ uniform 代数恒等实测登记 + 空间重用加性臂）
// ---------------------------------------------------------------------------

fn host_only_leg(args: &Args) -> ! {
    let lights = bistro_point_lights(&args.contract).unwrap_or_else(|e| fail(&e));
    let reference = exact_direct(&SHADE, &lights);
    // 录制自检锚恒跑（record_bands 内嵌逐 trial 位级，失败即 exit 1）。
    let bands_on = record_bands(&lights, &SHADE, args.trials, M_ON);
    let bands_off = record_bands(&lights, &SHADE, args.trials, M_OFF);
    let (mean_on, var_on) = mean_var(&bands_on.host_est);
    let (mean_off, var_off) = mean_var(&bands_off.host_est);
    let sigma_on = (var_on / f64::from(args.trials)).sqrt();
    let sigma_off = (var_off / f64::from(args.trials)).sqrt();
    let unb_on = (mean_on - reference).abs() < 3.0 * sigma_on + 1e-9;
    let unb_off = (mean_off - reference).abs() < 3.0 * sigma_off + 1e-9;
    // M=1 ≡ uniform 代数恒等实测：逐 trial estimate_ris(m=1) vs estimate_uniform
    // 同流对拍，最大相对差如实登记（f64 除法舍入 ~ulp 级，口径 ≤1e-9 硬门）。
    let mut max_rel = 0.0f64;
    for t in 0..args.trials {
        let mut rng_u = Pcg32::new(SEED, u64::from(t) * 4 + 1);
        let uni = estimate_uniform(&SHADE, &lights, &mut rng_u);
        let ris1 = bands_off.host_est[t as usize];
        let rel = (ris1 - uni).abs() / uni.abs().max(1e-30);
        if rel > max_rel {
            max_rel = rel;
        }
    }
    let m1_equiv = max_rel <= 1e-9;
    let table_digest = sha256_f32(&lights_flat(&lights));
    let state = if unb_on && unb_off && m1_equiv {
        "pass"
    } else {
        "fail"
    };
    let line = format!(
        "{{\"schema\":\"rurix.g31restir.host.v1\",\"state\":{},\"scene_id\":{},\"n_lights\":{},\"light_table_digest\":{},\"n_trials\":{},\"reference\":{:.15e},\"on\":{{\"mean\":{:.15e},\"variance\":{:.15e},\"unbiased_3sigma\":{}}},\"off\":{{\"mean\":{:.15e},\"variance\":{:.15e},\"unbiased_3sigma\":{}}},\"m1_uniform_equiv\":{{\"max_rel_dev\":{:.3e},\"bound\":1e-9,\"pass\":{}}},\"recorder_selfcheck_bitexact\":true,\"base_commit\":{}}}",
        jstr(state),
        jstr(SCENE_ID),
        lights.len(),
        jstr(&format!("sha256:{table_digest}")),
        args.trials,
        reference,
        mean_on,
        var_on,
        unb_on,
        mean_off,
        var_off,
        unb_off,
        max_rel,
        m1_equiv,
        jstr(&base_commit()),
    );
    emit_line(&line, &args.out);
    std::process::exit(if state == "pass" { 0 } else { 1 })
}

// ---------------------------------------------------------------------------
// 单臂腿（--restir off|on [--spatial]）：车道验收单行 JSON
// ---------------------------------------------------------------------------

fn lane_leg(args: &Args) -> ! {
    let lights = bistro_point_lights(&args.contract).unwrap_or_else(|e| fail(&e));
    let spv = load_spv(args.spv.as_deref().unwrap_or_else(|| fail("缺 --spv")));
    let dev = match DeviceLane::create(spv) {
        Ok(d) => d,
        Err(e) => {
            emit_line(&skip_line("rurix.g31restir.lane.v1", &e), &args.out);
            std::process::exit(0);
        }
    };
    let m_cand = if args.restir == "on" { M_ON } else { M_OFF };
    let arm = run_arm(&dev, &lights, args.trials, m_cand, args.tol, args.timing_runs);
    let mut problems = arm.problems.clone();
    let spatial_seg = if args.restir == "on" && args.spatial {
        let s = spatial_arm(&lights, N_TRIALS_SPATIAL);
        if !s.aggregate_pass || !s.all_within_5 || !s.bitexact {
            problems.push("空间重用加性臂判据失败（聚合3σ/逐点5σ/双跑位级）".into());
        }
        format!(",\"spatial\":{}", spatial_json(&s))
    } else {
        String::new()
    };
    let state = if problems.is_empty() { "pass" } else { "fail" };
    eprintln!(
        "{TAG}: --restir {} trials={} m={} y_anchor={} dec_anchor={} p100={:.6e} tol={:.6e} unbiased_3sigma={} bitexact={} dispatch_ms_mean={:.4}",
        args.restir,
        args.trials,
        m_cand,
        arm.y_all_equal,
        arm.dec_all_equal,
        arm.p100,
        args.tol,
        arm.unbiased_pass,
        arm.bitexact,
        if arm.dispatch_ms.is_empty() {
            0.0
        } else {
            ms_stats(&arm.dispatch_ms).0
        },
    );
    let line = format!(
        "{{\"schema\":\"rurix.g31restir.lane.v1\",\"state\":{},\"restir\":{},\"scene_id\":{},\"n_lights\":{},\"light_table_digest\":{},\"n_trials\":{},\"arm\":{}{},\"base_commit\":{}}}",
        jstr(state),
        jstr(&args.restir),
        jstr(SCENE_ID),
        lights.len(),
        jstr(&format!("sha256:{}", sha256_f32(&lights_flat(&lights)))),
        args.trials,
        arm_json(&args.restir, &arm, args.tol),
        spatial_seg,
        jstr(&base_commit()),
    );
    emit_line(&line, &args.out);
    std::process::exit(if state == "pass" { 0 } else { 1 })
}

// ---------------------------------------------------------------------------
// compare 腿（双臂真跑 + 双跑位级 + 计时对照 + 空间重用加性臂 → 单行 JSON）
// ---------------------------------------------------------------------------

fn compare_leg(args: &Args) -> ! {
    let lights = bistro_point_lights(&args.contract).unwrap_or_else(|e| fail(&e));
    let spv = load_spv(args.spv.as_deref().unwrap_or_else(|| fail("缺 --spv")));
    let dev = match DeviceLane::create(spv) {
        Ok(d) => d,
        Err(e) => {
            emit_line(&skip_line("rurix.g31restir.compare.v1", &e), &args.out);
            std::process::exit(0);
        }
    };
    let off = run_arm(&dev, &lights, args.trials, M_OFF, args.tol, args.timing_runs);
    let on = run_arm(&dev, &lights, args.trials, M_ON, args.tol, args.timing_runs);
    let spatial = spatial_arm(&lights, N_TRIALS_SPATIAL);

    let mut problems: Vec<String> = Vec::new();
    problems.extend(off.problems.iter().map(|p| format!("off: {p}")));
    problems.extend(on.problems.iter().map(|p| format!("on: {p}")));
    if !spatial.aggregate_pass || !spatial.all_within_5 || !spatial.bitexact {
        problems.push("空间重用加性臂判据失败（聚合3σ/逐点5σ/双跑位级）".into());
    }
    // 方差对照（方向硬门：高档臂方差必须严格低于默认档；比值 measured 登记）。
    let variance_reduction = off.variance / on.variance.max(1e-30);
    if !(variance_reduction > 1.0) {
        problems.push(format!(
            "方差对照方向破缺：var(off)/var(on)={variance_reduction:.6} 未 >1"
        ));
    }
    let state = if problems.is_empty() { "pass" } else { "fail" };
    let (off_ms, _, _) = ms_stats(&off.dispatch_ms);
    let (on_ms, _, _) = ms_stats(&on.dispatch_ms);
    eprintln!(
        "{TAG}: compare trials={} off(m={}) vs on(m={}) var {:.6e}→{:.6e}（reduction {:.3}）dispatch_ms {:.4}→{:.4} y_anchor on={} p100 on={:.3e} state={state}",
        args.trials, M_OFF, M_ON, off.variance, on.variance, variance_reduction, off_ms, on_ms,
        on.y_all_equal, on.p100,
    );
    let line = format!(
        "{{\"schema\":\"rurix.g31restir.compare.v1\",\"state\":{},\"scene_id\":{},\"seed\":{},\"n_trials\":{},\"n_lights\":{},\"light_table_digest\":{},\"tol\":{:.15e},\"off\":{},\"on\":{},\"spatial\":{},\"variance_reduction\":{:.6},\"problems\":{},\"base_commit\":{}}}",
        jstr(state),
        jstr(SCENE_ID),
        SEED,
        args.trials,
        lights.len(),
        jstr(&format!("sha256:{}", sha256_f32(&lights_flat(&lights)))),
        args.tol,
        arm_json("off", &off, args.tol),
        arm_json("on", &on, args.tol),
        spatial_json(&spatial),
        variance_reduction,
        strs_json(&problems),
        jstr(&base_commit()),
    );
    emit_line(&line, &args.out);
    std::process::exit(if state == "pass" { 0 } else { 1 })
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    let args = parse_args();
    if args.host_only {
        host_only_leg(&args);
    }
    if args.compare {
        compare_leg(&args);
    }
    lane_leg(&args);
}
