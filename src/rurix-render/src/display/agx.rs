//! AgX view transform 插件（G9.5 M118；RFC-0025 §4.I；RXS-0369 L2）。
//!
//! 参考公式 = Troy Sobotka AgX 的 iolite minimal 实现（MIT；
//! iolite-engine.com/blog_posts/minimal_agx_implementation）host 逐字移植：
//! inset 矩阵 → log2 编码（min_ev=-12.47393 / max_ev=4.026069）→ 6 阶 sigmoid
//! 多项式 → look（ASC CDL + 饱和度）→ outset 矩阵得显示线性；原参考的
//! `pow(2.2)` 显示编码段由共享输出编码承担（[`super::view_transform`]）。
//!
//! **对比度补偿参数随 view transform 资产化**（RXS-0369 L2 字面）：[`AgxLook`]
//! 为插件资产字段（slope/offset/power/saturation 四元组），canonical 默认 =
//! Punchy look（slope=1, offset=0, power=1.35, sat=1.4）；禁止硬编码进
//! tonemap 节点——harness 篡改探针验证参数确实来自资产字段（改资产 ⇒ 输出
//! 分叉）。
//!
//! 已知差异记录（D4 R-D4-5）：AgX 与 ACES 的 hue-skew 差异由 harness 实测
//! 写带，不作 bug 返工。

use super::color::{self, Mat3};

/// AgX inset 矩阵（iolite minimal 逐字；GLSL 列主序已转置为本库行向量约定）。
const AGX_INSET: Mat3 = [
    [0.842479062253094, 0.0784335999999992, 0.0792237451477643],
    [0.0423282422610123, 0.878468636469772, 0.0791661274605434],
    [0.0423756549057051, 0.0784336, 0.879142973793104],
];

/// AgX outset 矩阵（同上逐字转置）。
const AGX_OUTSET: Mat3 = [
    [1.19687900512017, -0.0980208811401368, -0.0990297440797205],
    [-0.0528968517574562, 1.15190312990417, -0.0989611768448433],
    [-0.0529716355144438, -0.0980434501171241, 1.15107367264116],
];

/// log2 编码域（iolite 逐字）。
const AGX_MIN_EV: f64 = -12.47393;
const AGX_MAX_EV: f64 = 4.026069;

/// 6 阶 sigmoid 多项式（iolite `agxDefaultContrastApprox` 逐字）。
fn agx_sigmoid(x: f64) -> f64 {
    let x2 = x * x;
    let x4 = x2 * x2;
    15.5 * x4 * x2 - 40.14 * x4 * x + 31.96 * x4 - 6.868 * x2 * x + 0.4298 * x2 + 0.1191 * x
        - 0.00232
}

/// AgX look 资产（对比度补偿参数面;随 view transform 资产化,RXS-0369 L2）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgxLook {
    /// ASC CDL slope。
    pub slope: [f64; 3],
    /// ASC CDL offset。
    pub offset: [f64; 3],
    /// ASC CDL power。
    pub power: [f64; 3],
    /// 饱和度（Rec.709 亮度权重）。
    pub saturation: f64,
}

impl AgxLook {
    /// canonical 默认资产 = Punchy look（iolite 参考逐字:slope=1, offset=0,
    /// power=1.35, sat=1.4）。
    pub fn punchy() -> Self {
        Self {
            slope: [1.0, 1.0, 1.0],
            offset: [0.0, 0.0, 0.0],
            power: [1.35, 1.35, 1.35],
            saturation: 1.4,
        }
    }
}

/// AgX 插件（look 资产内嵌;golden 消费 canonical Punchy 资产）。
pub struct AgX {
    look: AgxLook,
}

impl Default for AgX {
    fn default() -> Self {
        Self::canonical()
    }
}

impl AgX {
    /// canonical 构造(Punchy 资产)。
    pub fn canonical() -> Self {
        Self {
            look: AgxLook::punchy(),
        }
    }

    /// 指定资产的构造(资产化面;golden 之外的消费路径)。
    pub fn with_look(look: AgxLook) -> Self {
        Self { look }
    }

    /// 当前 look 资产(provenance 面)。
    pub fn look(&self) -> &AgxLook {
        &self.look
    }

    /// `agx()` 逐字(inset → log2 编码 → sigmoid)。
    fn agx_core(v: [f64; 3]) -> [f64; 3] {
        let mut val = color::vmul(v, &AGX_INSET);
        for c in val.iter_mut() {
            *c = c
                .max(f64::MIN_POSITIVE)
                .log2()
                .clamp(AGX_MIN_EV, AGX_MAX_EV);
            *c = (*c - AGX_MIN_EV) / (AGX_MAX_EV - AGX_MIN_EV);
        }
        [
            agx_sigmoid(val[0]),
            agx_sigmoid(val[1]),
            agx_sigmoid(val[2]),
        ]
    }

    /// `agxLook()` 逐字(ASC CDL + Rec.709 亮度权重饱和度;参数全来自资产)。
    fn apply_look(&self, v: [f64; 3]) -> [f64; 3] {
        let look = &self.look;
        const LW: [f64; 3] = [0.2126, 0.7152, 0.0722];
        let mut out = [0.0f64; 3];
        for i in 0..3 {
            out[i] = (v[i] * look.slope[i] + look.offset[i])
                .max(0.0)
                .powf(look.power[i]);
        }
        let luma = out[0] * LW[0] + out[1] * LW[1] + out[2] * LW[2];
        for c in out.iter_mut() {
            *c = luma + look.saturation * (*c - luma);
        }
        out
    }
}

impl super::view_transform::ViewTransform for AgX {
    fn id(&self) -> &'static str {
        "agx"
    }

    fn display_name(&self) -> &'static str {
        "AgX (Troy Sobotka AgX;iolite minimal host 逐字;look=Punchy 资产化)"
    }

    fn to_display_linear(&self, hdr_linear: [f64; 3]) -> [f64; 3] {
        let core = Self::agx_core(hdr_linear);
        let looked = self.apply_look(core);
        color::vmul(looked, &AGX_OUTSET)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::view_transform::ViewTransform;

    //@ spec: RXS-0369
    #[test]
    fn agx_known_output_landmarks() {
        let p = AgX::canonical();
        // 黑 → 近 0(sigmoid 下端)。
        let b = p.to_display_linear([0.0, 0.0, 0.0]);
        assert!(b.iter().all(|v| v.is_finite()), "黑: {b:?}");
        // 中性灰保持近中性(AgX outset 往返对 neutral 轴近恒等)。
        let g = p.to_display_linear([0.18, 0.18, 0.18]);
        assert!(
            (g[0] - g[1]).abs() < 0.02 && (g[1] - g[2]).abs() < 0.02,
            "中性近保持: {g:?}"
        );
        assert!(g[0] > 0.1 && g[0] < 0.6, "0.18 灰映射域: {g:?}");
        // 单调。
        let a = p.to_display_linear([0.09, 0.09, 0.09]);
        let m = p.to_display_linear([0.36, 0.36, 0.36]);
        assert!(a[0] < g[0] && g[0] < m[0]);
        // 确定性。
        assert_eq!(
            p.to_display_linear([1.0, 0.5, 0.25]),
            p.to_display_linear([1.0, 0.5, 0.25])
        );
    }

    //@ spec: RXS-0369
    #[test]
    fn agx_look_assetized_not_hardcoded() {
        // 对比度补偿参数随资产:改资产 ⇒ 输出分叉(禁硬编码的机核面)。
        let punchy = AgX::canonical();
        let mut flat_look = AgxLook::punchy();
        flat_look.power = [1.0, 1.0, 1.0];
        flat_look.saturation = 1.0;
        let flat = AgX::with_look(flat_look);
        let x = [0.5, 0.25, 0.125];
        assert_ne!(punchy.to_display_linear(x), flat.to_display_linear(x));
    }
}
