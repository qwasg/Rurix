/// G37 W2(夜航 night_0830)色彩分级 LUT host 资产模块——TODO #79 收口件
/// (M119 后处理五级链第 4 级 color grading 的 device 接线 host 面)。
///
/// 组织律 = day_0829 六臂「host 面落窗口 bin 自有文件」同律的模块化演进:
/// 本文件为窗口 bin 可 `include!("g37_w2/g31_lut_assets.rs")` 的独立 mod
/// (src/bin 顶层每文件一 bin,故落子目录;g14_3_lane/ 先例——无 main.rs
/// 的子目录不被 cargo 自动发现为 bin)。共享体/母版 kernel/既有 SPV 0-byte。
///
/// ## 传输面设计(反红修 #2 地雷:零新资源/绑定/屏障/下标族)
/// LUT 表不走独立 SSBO,内嵌 encode 参数 buffer(G31_U_ENC_PARAMS,22 号)
/// 尾部——A2 增益槽 params[133] 同律的 reserved 槽升级:
/// - `[134]` = lut_gate(1.0;kernel ≤0.5 直通守卫)
/// - `[135]` = lut_dim N(∈ [2,64];默认 17)
/// - `[136..136+3N³)` = 表体(R 最快序 idx = r + g·N + b·N²,每格点 out RGB
///   3 f32,显示线性域 [0,1] → [0,1])
/// off 臂 = 既有 136 f32 参数面 0-byte(锚零漂移);on 臂 = buffer 变长 +
/// 换载 g31_display_encode_lut.spv(字节隔离,day_0828 C 相纪律)。
mod g31_lut_assets {
    /// 内嵌 preset 的格点边长(17³ = 4913 格点 × 3 f32 ≈ 57.6 KB;工业
    /// 常用档,.cube 文件可覆写 ∈ [2,64])。
    pub const G31_LUT_DIM_DEFAULT: usize = 17;
    /// 表体在 encode 参数 buffer 内的起始 f32 下标(= 既有参数面长度;
    /// kernel 字面 136 同源——变更须双侧同步,fail-closed 断言在
    /// [`extend_encode_params`])。
    pub const G31_LUT_PARAMS_BASE: usize = 136;
    /// .cube 解析格点边长闭集(2 = 最小合法 trilinear 格;64³×3×4 ≈ 3.1 MB
    /// 上界——SSBO 尾挂预算面)。
    pub const G31_LUT_DIM_MAX: usize = 64;

    /// LUT 资产(host 单一事实源;表体 R 最快序,长度 = 3·dim³)。
    pub struct G31LutAsset {
        pub dim: usize,
        /// R 最快序表体:`table[3·(r + g·dim + b·dim²) + c]`,c ∈ {0,1,2}。
        pub table: Vec<f32>,
        /// 来源登记字面(evidence/报告消费:"neutral" / "warm" / 文件路径)。
        pub source: String,
    }

    impl G31LutAsset {
        /// 表体 SHA-256(f32 LE 字节流;digest 锚/双构建相等核验面)。
        pub fn table_sha256(&self) -> String {
            let mut buf = Vec::with_capacity(self.table.len() * 4);
            for v in &self.table {
                buf.extend_from_slice(&v.to_le_bytes());
            }
            let d = rurix_pkg::sha256::digest(&buf);
            let mut s = String::with_capacity(64);
            for b in d {
                s.push_str(&format!("{b:02x}"));
            }
            s
        }
    }

    /// 格点输入坐标(r,g,b 格点号 → 显示线性域 [0,1] 输入色;f32 端点精确)。
    fn lattice_input(idx: usize, dim: usize) -> f32 {
        idx as f32 / (dim - 1) as f32
    }

    /// 中性(恒等)LUT:格点值 = 格点输入坐标。trilinear 重建对逐坐标线性
    /// 函数精确(至 f32 舍入 ~1-2 ULP;8-bit 量化后与直通几乎处处同字节,
    /// 非位级保证——如实登记,验收口径 = on(neutral) 双跑位级 + 对 off 的
    /// A/B 近恒等度量,不冒充 off 锚)。
    pub fn neutral(dim: usize) -> G31LutAsset {
        assert!(
            (2..=G31_LUT_DIM_MAX).contains(&dim),
            "LUT dim 必须 ∈ [2,{G31_LUT_DIM_MAX}]"
        );
        let mut table = Vec::with_capacity(3 * dim * dim * dim);
        for b in 0..dim {
            for g in 0..dim {
                for r in 0..dim {
                    table.push(lattice_input(r, dim));
                    table.push(lattice_input(g, dim));
                    table.push(lattice_input(b, dim));
                }
            }
        }
        G31LutAsset {
            dim,
            table,
            source: "neutral".to_owned(),
        }
    }

    /// 暖色分级 preset(闭式确定性 f64 求值 → f32 收窄一次;字节跨构建
    /// 稳定):白平衡暖移(R×1.06/B×0.94)→ Rec.709 luma 保持的饱和度
    /// ×1.12 → 轻抬暗部 γ0.96 → 钳 [0,1]。host 金标准
    /// (post_chain::apply_color_grading 逐通道仿射)是本 preset 的一阶
    /// 近似子集;3D 表形态承载饱和度跨通道耦合(1D 仿射表达不了——
    /// device 面即 M119 注释「完整 3D LUT 在 device 面」的兑现)。
    pub fn preset_warm(dim: usize) -> G31LutAsset {
        assert!(
            (2..=G31_LUT_DIM_MAX).contains(&dim),
            "LUT dim 必须 ∈ [2,{G31_LUT_DIM_MAX}]"
        );
        let mut table = Vec::with_capacity(3 * dim * dim * dim);
        for b in 0..dim {
            for g in 0..dim {
                for r in 0..dim {
                    let rin = f64::from(lattice_input(r, dim));
                    let gin = f64::from(lattice_input(g, dim));
                    let bin = f64::from(lattice_input(b, dim));
                    // 白平衡暖移。
                    let rw = rin * 1.06;
                    let gw = gin;
                    let bw = bin * 0.94;
                    // luma 保持饱和度(Rec.709 权重)。
                    let luma = 0.2126 * rw + 0.7152 * gw + 0.0722 * bw;
                    let rs = luma + (rw - luma) * 1.12;
                    let gs = luma + (gw - luma) * 1.12;
                    let bs = luma + (bw - luma) * 1.12;
                    // 轻抬暗部(γ<1;负值先钳 0——显示域输入本非负)。
                    let rq = rs.max(0.0).powf(0.96).min(1.0);
                    let gq = gs.max(0.0).powf(0.96).min(1.0);
                    let bq = bs.max(0.0).powf(0.96).min(1.0);
                    table.push(rq as f32);
                    table.push(gq as f32);
                    table.push(bq as f32);
                }
            }
        }
        G31LutAsset {
            dim,
            table,
            source: "warm".to_owned(),
        }
    }

    /// .cube 3D LUT 文本解析(Adobe/Resolve 惯例子集,fail-closed):
    /// - `LUT_3D_SIZE N`(N ∈ [2,64]);`TITLE` 忽略;`LUT_1D_SIZE` 拒;
    /// - `DOMAIN_MIN`/`DOMAIN_MAX` 缺省 = 0/1,显式给出须恰为 0 0 0 / 1 1 1
    ///   (kernel 采样域钉死 [0,1],非单位域不静默重映射);
    /// - 数据行 = 3 个有限 f32(R 最快序,与本模块表序同——逐行顺灌);
    /// - 行数必须 == N³;越界值不钳不拒(显示域外值由 kernel 输出钳兜底,
    ///   如实透传资产作者意图)。
    pub fn parse_cube(text: &str, source: &str) -> Result<G31LutAsset, String> {
        let mut dim: Option<usize> = None;
        let mut table: Vec<f32> = Vec::new();
        for (ln, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let upper = line.to_ascii_uppercase();
            if upper.starts_with("TITLE") {
                continue;
            }
            if upper.starts_with("LUT_1D_SIZE") {
                return Err(format!("{source}:{}: LUT_1D_SIZE 不支持(仅 3D)", ln + 1));
            }
            if upper.starts_with("LUT_3D_SIZE") {
                let n: usize = line
                    .split_whitespace()
                    .nth(1)
                    .ok_or_else(|| format!("{source}:{}: LUT_3D_SIZE 缺值", ln + 1))?
                    .parse()
                    .map_err(|e| format!("{source}:{}: LUT_3D_SIZE 非整数: {e}", ln + 1))?;
                if !(2..=G31_LUT_DIM_MAX).contains(&n) {
                    return Err(format!(
                        "{source}:{}: LUT_3D_SIZE {n} 越界(须 ∈ [2,{G31_LUT_DIM_MAX}])",
                        ln + 1
                    ));
                }
                if dim.replace(n).is_some() {
                    return Err(format!("{source}:{}: LUT_3D_SIZE 重复声明", ln + 1));
                }
                continue;
            }
            if upper.starts_with("DOMAIN_MIN") || upper.starts_with("DOMAIN_MAX") {
                let want = if upper.starts_with("DOMAIN_MIN") { 0.0 } else { 1.0 };
                let vals: Vec<f32> = line
                    .split_whitespace()
                    .skip(1)
                    .map(|t| t.parse::<f32>())
                    .collect::<Result<_, _>>()
                    .map_err(|e| format!("{source}:{}: DOMAIN 值非数: {e}", ln + 1))?;
                if vals.len() != 3 || vals.iter().any(|&v| v != want) {
                    return Err(format!(
                        "{source}:{}: DOMAIN 须恰为 {want} {want} {want}(采样域钉死 [0,1])",
                        ln + 1
                    ));
                }
                continue;
            }
            // 数据行:3 个有限 f32。
            let vals: Vec<f32> = line
                .split_whitespace()
                .map(|t| t.parse::<f32>())
                .collect::<Result<_, _>>()
                .map_err(|e| format!("{source}:{}: 数据行非数: {e}", ln + 1))?;
            if vals.len() != 3 {
                return Err(format!(
                    "{source}:{}: 数据行须恰 3 分量(得 {})",
                    ln + 1,
                    vals.len()
                ));
            }
            if vals.iter().any(|v| !v.is_finite()) {
                return Err(format!("{source}:{}: 数据行含非有限值", ln + 1));
            }
            table.extend_from_slice(&vals);
        }
        let dim = dim.ok_or_else(|| format!("{source}: 缺 LUT_3D_SIZE 声明"))?;
        let expect = dim * dim * dim;
        if table.len() != expect * 3 {
            return Err(format!(
                "{source}: 数据行数 {} ≠ N³ = {expect}(N = {dim})",
                table.len() / 3
            ));
        }
        Ok(G31LutAsset {
            dim,
            table,
            source: source.to_owned(),
        })
    }

    /// CLI 字面 → 资产(闭集:`off` = None;`neutral`/`warm` = 内嵌 preset;
    /// 其余按 .cube 文件路径读取解析,fail-closed)。
    pub fn from_arg(arg: &str) -> Result<Option<G31LutAsset>, String> {
        match arg {
            "off" => Ok(None),
            "neutral" => Ok(Some(neutral(G31_LUT_DIM_DEFAULT))),
            "warm" => Ok(Some(preset_warm(G31_LUT_DIM_DEFAULT))),
            path => {
                let text = std::fs::read_to_string(path)
                    .map_err(|e| format!("读 LUT 文件 {path}: {e}"))?;
                parse_cube(&text, path).map(Some)
            }
        }
    }

    /// encode 参数 buffer 尾挂 LUT(施加于 `aces13_device_encode_params_ex`
    /// 产物;断言既有参数面恰 136 f32——kernel 表基址字面 136 的双侧同步
    /// 卫兵,漂移即红):置 [134]=1.0 门 + [135]=dim + 追加表体。
    pub fn extend_encode_params(params: &mut Vec<f32>, asset: &G31LutAsset) {
        assert_eq!(
            params.len(),
            G31_LUT_PARAMS_BASE,
            "encode 参数面长度漂移(须 == {G31_LUT_PARAMS_BASE};kernel 表基址字面同源)"
        );
        assert_eq!(
            asset.table.len(),
            3 * asset.dim * asset.dim * asset.dim,
            "LUT 表体长度与 dim³ 不符"
        );
        params[134] = 1.0;
        params[135] = asset.dim as f32;
        params.extend_from_slice(&asset.table);
    }

    /// host 参考 trilinear 采样(与 kernel LUT 段逐操作同序同 f32 语义——
    /// device/host 对拍与自检面;输入应在 [0,1],与 kernel 上游钳制同前提)。
    pub fn sample_trilinear_f32(asset: &G31LutAsset, rgb: [f32; 3]) -> [f32; 3] {
        let lutn = asset.dim;
        let lutnm1 = asset.dim as f32 - 1.0;
        let lux = rgb[0] * lutnm1;
        let luy = rgb[1] * lutnm1;
        let luz = rgb[2] * lutnm1;
        let mut lx0f = lux.floor();
        if lx0f > lutnm1 - 1.0 {
            lx0f = lutnm1 - 1.0;
        }
        let mut ly0f = luy.floor();
        if ly0f > lutnm1 - 1.0 {
            ly0f = lutnm1 - 1.0;
        }
        let mut lz0f = luz.floor();
        if lz0f > lutnm1 - 1.0 {
            lz0f = lutnm1 - 1.0;
        }
        let ltx = lux - lx0f;
        let lty = luy - ly0f;
        let ltz = luz - lz0f;
        let lx0 = lx0f as usize;
        let ly0 = ly0f as usize;
        let lz0 = lz0f as usize;
        let lx1 = lx0 + 1;
        let ly1 = ly0 + 1;
        let lz1 = lz0 + 1;
        let idx = |r: usize, g: usize, b: usize| (r + g * lutn + b * lutn * lutn) * 3;
        let c000 = idx(lx0, ly0, lz0);
        let c100 = idx(lx1, ly0, lz0);
        let c010 = idx(lx0, ly1, lz0);
        let c110 = idx(lx1, ly1, lz0);
        let c001 = idx(lx0, ly0, lz1);
        let c101 = idx(lx1, ly0, lz1);
        let c011 = idx(lx0, ly1, lz1);
        let c111 = idx(lx1, ly1, lz1);
        let w000 = (1.0 - ltx) * (1.0 - lty) * (1.0 - ltz);
        let w100 = ltx * (1.0 - lty) * (1.0 - ltz);
        let w010 = (1.0 - ltx) * lty * (1.0 - ltz);
        let w110 = ltx * lty * (1.0 - ltz);
        let w001 = (1.0 - ltx) * (1.0 - lty) * ltz;
        let w101 = ltx * (1.0 - lty) * ltz;
        let w011 = (1.0 - ltx) * lty * ltz;
        let w111 = ltx * lty * ltz;
        let t = &asset.table;
        let mut out = [0.0f32; 3];
        for (c, o) in out.iter_mut().enumerate() {
            *o = (t[c000 + c] * w000
                + t[c100 + c] * w100
                + t[c010 + c] * w010
                + t[c110 + c] * w110
                + t[c001 + c] * w001
                + t[c101 + c] * w101
                + t[c011 + c] * w011
                + t[c111 + c] * w111)
                .max(0.0)
                .min(1.0);
        }
        out
    }
}
