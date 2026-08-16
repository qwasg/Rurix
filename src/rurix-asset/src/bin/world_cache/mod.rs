//! G11.4 M154 世界辐射缓存 host CPU 参考实现（spec/global_illumination.md
//! RXS-0396 条款语义面逐字承接；RFC-0028 §4.2 冻结语义；M99-clipmap 世界级
//! 承接 = G10.6 rejudged-go 承接锚兑现）。
//!
//! ## 语义面（RXS-0396 L2/L3/L4 机核消费）
//!
//! - **空间索引形态**：世界空间哈希缓存——位置按距离自适应量化（**对数族**，
//!   量化函数族闭集 {对数族, 幂律族} 取定）：`level(p) = clamp(floor(log2(1 +
//!   dist(p,camera)/d_ref)), 0, LEVELS−1)`，格长 `s(ℓ) = s0×2^ℓ`（`LEVELS=4`、
//!   `s0 = scene_diag×2^-8`、`d_ref = scene_diag×2^-4`，实现波 measured 标定
//!   冻结）；哈希冲突走**双哈希步长线性探测**（h1 定位 + h2 步长，探测上界
//!   [`WC_PROBE_BOUND`] 闭集登记）；在线构建、零离线预处理。
//! - **辐射 LOD（clipmap 级）**：按距离自适应的辐射度细节层级；层级数/每层
//!   命中率/沉积计数逐帧进 evidence（[`WcStats`]）；禁静默降层级。
//! - **回落语义**：级内未命中 → 更粗级查询（级间回落链，逐级计数）→ 查询
//!   失败返回 None（调用方末级兜底 = 天光/常量环境项，显式登记）。
//! - **能量口径**：条目 = 出射辐射度沉淀（直接光命中辐射度；多反弹迭代
//!   `it ≥ 1` 级 = 直接 + albedo×L_上一级查询）；查询 = 路径终止式辐射度
//!   查询（3×3×3 邻域权重均值 + 级间回落链），调用方辐照度恒等式
//!   `E = π × L`（均匀朗伯环境，白炉自洽）⇒ 间接出射 = albedo×L_query
//!   （只丢能量不漏光方向）+ 朝向闸（条目法线与命中着色法线对齐
//!   `dot(e.n, n_hit) ≥ 0.5`——跨墙/背向面片拒绝 = 丢能量方向）。
//!
//! 确定性：全部 f32/f64 定点序累加；哈希与探测序确定；构建种子走契约
//! `time.random_seed` 派生链（`gi::probe::probe_seed` 同律）。
//!
//! Assisted-by: Kimi-K3（G11.4 波）

use rurix_render::rt::bvh::Vec3;

/// 辐射 LOD 层级数（clipmap 级；RXS-0396 L2 实测标定冻结）。
pub const WC_LEVELS: u32 = 4;
/// 每级哈希表槽位 log2（2^16 = 65536 槽/级；容量闭集登记）。
pub const WC_CAPACITY_LOG2: u32 = 16;
/// 双哈希线性探测上界（闭集登记；超出即沉积丢弃计数 + dropped 登记，不静默）。
pub const WC_PROBE_BOUND: u32 = 8;
/// 构建探针屏幕块边长（像素；远场低频面粗采样，实测标定冻结）。
pub const WC_BUILD_CELL: u32 = 16;
/// 每构建探针光线数（余弦半球；实测标定冻结）。
pub const WC_BUILD_RAYS: u32 = 8;
/// 多反弹迭代级数（≥2 满足 RXS-0395 L2；渲染消费第 iters 级 ⇒ 路径深度
/// iters+1 = 4 = M96 匹配深度 full 档，RXS-0357 L2 表消费）。
pub const WC_BOUNCE_ITERS: u32 = 3;

/// splitmix64 终态混合（确定性哈希原语；与流无关的纯函数）。
fn mix64(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// 世界缓存参数（场景标定面；s0/d_ref 自场景包围盒对角线实测派生）。
#[derive(Debug, Clone, Copy)]
pub struct WcParams {
    /// 最细级格长（= scene_diag × 2^-8）。
    pub s0: f32,
    /// 对数量化参考距离（= scene_diag × 2^-4）。
    pub d_ref: f32,
    /// 相机世界位置（距离自适应量化基准点）。
    pub cam: [f32; 3],
}

/// 缓存条目（出射辐射度沉淀；weight = 沉积计数，辐射度/位置/法线为加权均值；
/// key = 沉积格键——建槽时冻结，不归并漂移）。
#[derive(Debug, Clone, Copy, Default)]
pub struct WcEntry {
    /// 格键（量化格坐标）。
    pub key: (i64, i64, i64),
    /// 沉积点加权均值位置。
    pub pos: [f32; 3],
    /// 沉积点加权均值法线（查询时归一化）。
    pub normal: [f32; 3],
    /// 出射辐射度加权均值（RGB 线性）。
    pub radiance: [f32; 3],
    /// 沉积权重（样本计数）。
    pub weight: f32,
    /// 占用旗标。
    pub occupied: bool,
}

/// 缓存计数面（逐 LOD 级；Cell 面供 &self 查询路径计数——单线程确定性）。
#[derive(Debug, Default)]
pub struct WcStats {
    /// 逐 LOD 级沉积计数。
    pub deposits: [u64; 4],
    /// 逐 LOD 级探测上界溢出丢弃计数。
    pub dropped: [u64; 4],
    /// 逐 LOD 级查询计数。
    pub queries: [core::cell::Cell<u64>; 4],
    /// 逐 LOD 级命中计数。
    pub hits: [core::cell::Cell<u64>; 4],
    /// 逐 LOD 级级间回落计数（级内未命中落向更粗级）。
    pub coarse_fallback: [core::cell::Cell<u64>; 4],
    /// 全级未命中（调用方末级兜底面）。
    pub miss: core::cell::Cell<u64>,
    /// 逐 LOD 级沉积能量（Σ 亮度，f64 累加）。
    pub energy: [f64; 4],
}

/// 世界辐射缓存（空间哈希 + 距离自适应辐射 LOD + 级间回落链）。
pub struct WorldCache {
    tables: Vec<Vec<WcEntry>>,
    params: WcParams,
    /// 计数面（evidence 消费）。
    pub stats: WcStats,
}

impl WorldCache {
    /// 建表（全空槽；在线构建零离线预处理）。
    pub fn new(params: WcParams) -> Self {
        let cap = 1usize << WC_CAPACITY_LOG2;
        WorldCache {
            tables: (0..WC_LEVELS).map(|_| vec![WcEntry::default(); cap]).collect(),
            params,
            stats: WcStats::default(),
        }
    }

    /// 场景标定派生（scene_diag 实测 → s0/d_ref；RXS-0396 L2 冻结规则）。
    pub fn params_from_scene(scene_diag: f32, cam: [f32; 3]) -> WcParams {
        WcParams {
            s0: scene_diag * 2.0f32.powi(-8),
            d_ref: scene_diag * 2.0f32.powi(-4),
            cam,
        }
    }

    /// 距离自适应量化级（对数族）。
    pub fn level_of(&self, p: Vec3) -> u32 {
        let c = Vec3::from_array(self.params.cam);
        let d = (p - c).length();
        let r = 1.0 + d / self.params.d_ref.max(1e-12);
        let l = r.log2().floor().max(0.0) as u32;
        l.min(WC_LEVELS - 1)
    }

    fn cell_key(&self, p: Vec3, level: u32) -> (i64, i64, i64) {
        let s = self.params.s0 * 2.0f32.powi(level as i32);
        let inv = 1.0 / s.max(1e-12);
        (
            (f64::from(p.x) * f64::from(inv)).floor() as i64,
            (f64::from(p.y) * f64::from(inv)).floor() as i64,
            (f64::from(p.z) * f64::from(inv)).floor() as i64,
        )
    }

    fn slots(level: u32, key: (i64, i64, i64)) -> (usize, usize) {
        let cap = 1u64 << WC_CAPACITY_LOG2;
        let seed = mix64(
            (key.0 as u64) ^ mix64((key.1 as u64).wrapping_add(mix64(key.2 as u64))),
        ) ^ mix64(level as u64 + 0x9E37_79B9);
        let h1 = (seed % cap) as usize;
        let stride = 1 + (mix64(seed ^ 0xD1B5_4A32_D192_ED03) % (cap - 2)) as usize;
        (h1, stride)
    }

    /// 沉积（同键加权归并；冲突双哈希步长线性探测；上界溢出丢弃计数）。
    pub fn deposit(&mut self, p: Vec3, n: Vec3, radiance: [f32; 3]) {
        let level = self.level_of(p);
        let key = self.cell_key(p, level);
        let (h1, stride) = Self::slots(level, key);
        let cap = 1usize << WC_CAPACITY_LOG2;
        let lum =
            f64::from(radiance[0] * 0.2126 + radiance[1] * 0.7152 + radiance[2] * 0.0722);
        let table = &mut self.tables[level as usize];
        for k in 0..WC_PROBE_BOUND {
            let slot = (h1 + k as usize * stride) % cap;
            let e = &mut table[slot];
            if !e.occupied {
                *e = WcEntry {
                    key,
                    pos: p.to_array(),
                    normal: n.to_array(),
                    radiance,
                    weight: 1.0,
                    occupied: true,
                };
                self.stats.deposits[level as usize] += 1;
                self.stats.energy[level as usize] += lum;
                return;
            }
            if e.key == key {
                let w = e.weight + 1.0;
                let pa = p.to_array();
                let na = n.to_array();
                for ch in 0..3 {
                    e.pos[ch] = (e.pos[ch] * e.weight + pa[ch]) / w;
                    e.normal[ch] = (e.normal[ch] * e.weight + na[ch]) / w;
                    e.radiance[ch] = (e.radiance[ch] * e.weight + radiance[ch]) / w;
                }
                e.weight = w;
                self.stats.deposits[level as usize] += 1;
                self.stats.energy[level as usize] += lum;
                return;
            }
        }
        self.stats.dropped[level as usize] += 1;
    }

    /// 辐射度查询（路径终止式：3×3×3 邻域权重均值 + 级内未命中 → 更粗级
    /// 回落链；朝向闸 = 条目法线与命中着色法线对齐 dot(e.n, n_hit) ≥ 0.5
    /// 〔同面转移选择——背向/跨墙面片拒绝 = 只丢能量不漏光方向〕；全级未
    /// 命中 = None——调用方末级兜底显式登记）。
    ///
    /// 返回 = 命中点朝查询来向的出射辐射度 L（朗伯面半球常值 ⇒ 格内权重
    /// 均值即局部辐射场）；调用方辐照度恒等式 E = π×L（均匀朗伯环境，
    /// 白炉自洽）。多反弹迭代 it ≥ 1 级沉积 = 直接 + albedo×L_上一级查询
    /// （E=πL 恒等式消 π）。
    pub fn query_radiance(&self, p: Vec3, n: Vec3) -> Option<[f32; 3]> {
        let cap = 1usize << WC_CAPACITY_LOG2;
        let start = self.level_of(p);
        for level in start..WC_LEVELS {
            self.stats.queries[level as usize].set(self.stats.queries[level as usize].get() + 1);
            if level > start {
                self.stats.coarse_fallback[level as usize]
                    .set(self.stats.coarse_fallback[level as usize].get() + 1);
            }
            let key = self.cell_key(p, level);
            let table = &self.tables[level as usize];
            let mut acc = [0.0f64; 3];
            let mut wsum = 0.0f64;
            // 3×3×3 邻域（含本格；跨表面能量经邻域条目进入）。
            for dx in -1i64..=1 {
                for dy in -1i64..=1 {
                    for dz in -1i64..=1 {
                        let nk = (key.0 + dx, key.1 + dy, key.2 + dz);
                        let (h1, stride) = Self::slots(level, nk);
                        for k in 0..WC_PROBE_BOUND {
                            let slot = (h1 + k as usize * stride) % cap;
                            let e = &table[slot];
                            if !e.occupied {
                                break;
                            }
                            if e.key != nk {
                                continue;
                            }
                            let en = Vec3::from_array(e.normal);
                            if en.length() <= 1e-12 {
                                continue;
                            }
                            // 朝向闸：条目面片与命中面同向（跨墙/背向拒绝）。
                            if en.normalize().dot(n) < 0.5 {
                                continue;
                            }
                            let w = f64::from(e.weight);
                            for ch in 0..3 {
                                acc[ch] += f64::from(e.radiance[ch]) * w;
                            }
                            wsum += w;
                        }
                    }
                }
            }
            if wsum > 0.0 {
                self.stats.hits[level as usize]
                    .set(self.stats.hits[level as usize].get() + 1);
                return Some([
                    (acc[0] / wsum) as f32,
                    (acc[1] / wsum) as f32,
                    (acc[2] / wsum) as f32,
                ]);
            }
        }
        self.stats.miss.set(self.stats.miss.get() + 1);
        None
    }
}
