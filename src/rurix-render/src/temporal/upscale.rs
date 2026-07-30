//! 时域超分后端接口(报告7 §2.4 接口设计 + §3 映射三件套之二;
//! RFC-0016 §4.H3:冻结接口 §4.0-3 照抄——「输入颜色/深度/MV/reactive/曝光
//! → 输出目标分辨率颜色」)。
//!
//! 接口语义(冻结,波次内不得漂移):
//! - 三个实现位:自研 TSR 类([`crate::temporal::tsr`],P1 主实现,任何平台
//!   保底)、FSR 3.1 开源、DirectSR(vendor 后端留口 = 本 trait 即接口;
//!   **本期不接 SDK**,接入评估归 RD-037+ 存续,接入时不改本文件)。
//! - 历史状态**内置于 backend 实现**,对外不可见:双缓冲语义——第 N 帧的
//!   [`upscale`](UpscaleBackend::upscale) 输出即 backend 内部第 N+1 帧的历史
//!   输入,调用方不得跨 backend 实例共享/外置历史;输出分辨率变化时实现必须
//!   自动丢弃历史(等效 [`UpscaleBackend::reset_history`])。
//! - vendor 语义对齐:reactive mask 是 DLSS/FSR/XeSS 集成接口的标准输入槽位
//!   (报告7 §2.3),非可选件;帧生成 FG/MFG 为 P3+ 独立层,不在本 trait 面。
//!
//! 坐标/单位约定(与 [`crate::temporal::common`] 一致):
//! - uv ∈ \[0,1\],原点左上;`mv` 为 uv 位移,历史采样位置 = uv + mv
//!   (mv = prev_uv - cur_uv,见 [`crate::temporal::common::compute_camera_mv`])。
//! - `jitter` 为**输入分辨率像素单位**的亚像素抖动(相机投影 jitter 的等价
//!   采样口径,见 [`crate::temporal::common::jitter_sequence`] 文档)。

use crate::temporal::image::ImageF32;

/// 时域超分统一输入(冻结接口,RFC-0016 §4.0-3)。
///
/// 生命周期为单次调用借入;backend 不得持有引用。除 `reactive` 外全部图像
/// 为**输入(内部分辨率)尺寸**;输出尺寸由 `output_size` 独立指定
/// (输入/输出分辨率解耦,报告7 §4 P1 第一机制)。
#[derive(Debug, Clone, Copy)]
pub struct UpscaleInputs<'a> {
    /// 当前帧颜色(3 通道 RGB,输入分辨率;预曝光——`exposure` 之前的光空间,
    /// 曝光语义见 `exposure` 字段)。
    pub color: &'a ImageF32,
    /// 当前帧深度(1 通道,输入分辨率;历史验证深度判据的唯一事实来源,
    /// vendor 输入契约同构)。
    pub depth: &'a ImageF32,
    /// 当前帧 motion vectors(2 通道,输入分辨率,uv 位移;完整 MV 由主几何
    /// pass MRT 供给,几何/蒙皮/WPO 三类速度,报告7 §2.1)。
    pub mv: &'a ImageF32,
    /// reactive mask(1 通道,输入分辨率;自动通道语义——透明/粒子 pass 的
    /// R8 附加输出,报告7 §2.3;`None` 等价全 0)。手工通道(材质级「永不
    /// 累积」标记)由调用方在写入本图前取 max 合并,接口面只此一槽。
    pub reactive: Option<&'a ImageF32>,
    /// 曝光系数(> 0;backend 将 `color × exposure` 转入显示域后做时域累积,
    /// 历史在显示域常驻——FSR2/TSR 的后曝光历史口径;输出即显示域颜色)。
    pub exposure: f32,
    /// 本帧亚像素抖动(输入分辨率像素单位;通常取自
    /// [`crate::temporal::common::jitter_sequence`])。
    pub jitter: [f32; 2],
    /// 输出(目标)分辨率;必须逐维 ≥ 输入分辨率(超分定义域;1:1 即
    /// NativeAA 档,FSR 3.1 Native AA 模式同构)。
    pub output_size: (u32, u32),
    /// 帧序号(诊断/收敛统计用;backend 不得依赖其连续性做正确性判断——
    /// 掉帧/暂停语义由 `reset` 表达)。
    pub frame_index: u32,
    /// 本帧强制丢弃历史(场景切换/相机跳切;等效先调
    /// [`UpscaleBackend::reset_history`] 再 upscale)。
    pub reset: bool,
}

impl<'a> UpscaleInputs<'a> {
    /// 冻结接口形状校验(全部后端共享;违例 panic = 装配期契约违例,
    /// 与底座 assert 纪律一致)。
    ///
    /// 返回 `(输入宽, 输入高, 输出宽, 输出高)` 便于实现解构。
    pub fn validated(&self) -> (u32, u32, u32, u32) {
        let (iw, ih) = (self.color.w, self.color.h);
        let (ow, oh) = self.output_size;
        assert_eq!(self.color.c, 3, "color 必须 3 通道 RGB");
        assert!(
            self.depth.c == 1 && self.depth.w == iw && self.depth.h == ih,
            "depth 必须 1 通道且与 color 同尺寸"
        );
        assert!(
            self.mv.c == 2 && self.mv.w == iw && self.mv.h == ih,
            "mv 必须 2 通道且与 color 同尺寸"
        );
        if let Some(r) = self.reactive {
            assert!(
                r.c == 1 && r.w == iw && r.h == ih,
                "reactive 必须 1 通道且与 color 同尺寸"
            );
            assert!(
                r.data.iter().all(|&v| (0.0..=1.0).contains(&v)),
                "reactive ∈ [0,1]"
            );
        }
        assert!(
            self.exposure.is_finite() && self.exposure > 0.0,
            "exposure 必须为正有限值"
        );
        assert!(
            self.jitter[0].is_finite() && self.jitter[1].is_finite(),
            "jitter 必须为有限值"
        );
        assert!(
            ow >= iw && oh >= ih,
            "输出分辨率必须逐维 ≥ 输入分辨率(超分定义域)"
        );
        (iw, ih, ow, oh)
    }
}

/// 时域超分后端 trait(冻结接口,RFC-0016 §4.0-3)。
///
/// 实现义务(接口契约,仿 UE `ITemporalUpscaler`,报告7 §2.4):
/// - 输出 = `output_size` 尺寸的 3 通道 RGB 显示域图像;
/// - 历史状态内置、双缓冲轮换(见模块文档);reset/首帧直接上采样当前帧;
/// - 重投影/历史验证一律经 [`crate::temporal::common`] 公共底座,
///   **禁私写重投影**(G-G5-7 代码审计点)。
pub trait UpscaleBackend {
    /// 后端名(诊断/日志;vendor 后端返回 SDK 标识)。
    fn name(&self) -> &str;
    /// 执行一帧超分;返回 `output_size` 的 3 通道显示域图像。
    fn upscale(&mut self, inputs: &UpscaleInputs) -> ImageF32;
    /// 丢弃全部历史状态(下一帧按首帧处理;输出分辨率变化的自动重置
    /// 不替代本方法——外部跳切仍应显式调用或置 `inputs.reset`)。
    fn reset_history(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img3(w: u32, h: u32) -> ImageF32 {
        ImageF32::from_fn(w, h, 3, |x, y, ch| (x + y + ch) as f32 * 0.01)
    }

    fn base_inputs<'a>(
        color: &'a ImageF32,
        depth: &'a ImageF32,
        mv: &'a ImageF32,
    ) -> UpscaleInputs<'a> {
        UpscaleInputs {
            color,
            depth,
            mv,
            reactive: None,
            exposure: 1.0,
            jitter: [0.0, 0.0],
            output_size: (color.w * 2, color.h * 2),
            frame_index: 0,
            reset: true,
        }
    }

    #[test]
    fn validated_accepts_well_formed() {
        let color = img3(16, 16);
        let depth = ImageF32::new(16, 16, 1);
        let mv = ImageF32::new(16, 16, 2);
        let reactive = ImageF32::from_fn(16, 16, 1, |x, _, _| (x % 2) as f32);
        let mut inputs = base_inputs(&color, &depth, &mv);
        inputs.reactive = Some(&reactive);
        assert_eq!(inputs.validated(), (16, 16, 32, 32));
    }

    #[test]
    #[should_panic]
    fn validated_rejects_wrong_channels() {
        let color = ImageF32::new(16, 16, 4);
        let depth = ImageF32::new(16, 16, 1);
        let mv = ImageF32::new(16, 16, 2);
        base_inputs(&color, &depth, &mv).validated();
    }

    #[test]
    #[should_panic]
    fn validated_rejects_downscale() {
        let color = img3(16, 16);
        let depth = ImageF32::new(16, 16, 1);
        let mv = ImageF32::new(16, 16, 2);
        let mut inputs = base_inputs(&color, &depth, &mv);
        inputs.output_size = (8, 8);
        inputs.validated();
    }

    #[test]
    #[should_panic]
    fn validated_rejects_bad_exposure() {
        let color = img3(16, 16);
        let depth = ImageF32::new(16, 16, 1);
        let mv = ImageF32::new(16, 16, 2);
        let mut inputs = base_inputs(&color, &depth, &mv);
        inputs.exposure = 0.0;
        inputs.validated();
    }

    #[test]
    #[should_panic]
    fn validated_rejects_reactive_out_of_range() {
        let color = img3(16, 16);
        let depth = ImageF32::new(16, 16, 1);
        let mv = ImageF32::new(16, 16, 2);
        let reactive = ImageF32::from_fn(16, 16, 1, |_, _, _| 1.5);
        let mut inputs = base_inputs(&color, &depth, &mv);
        inputs.reactive = Some(&reactive);
        inputs.validated();
    }
}
