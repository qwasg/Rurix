//! UC-04 deferred 运行时装配/编排错误(strict-only;RXS-0167~0170 / P-01,无运行期
//! fallback)。装配期可预测错误映射 6xxx 诊断码 **RX6018~RX6022**
//! (`registry/error_codes.json` + en/zh message-key `runtime.uc04_*`)。
//!
//! [`Uc04Error::ShimUnavailable`] / [`Uc04Error::DeviceRunFailed`] 为 device 段 sentinel,
//! **非语言 RX**(不滥发诊断码;D3D12 纯运行期/环境失败,06 §8.2 / spec/d3d12_runtime.md §0)——
//! 缺 `real-shim`/MSVC/D3D12 或 device 真跑失败按环境失败报告,不以替代物伪造 device 绿
//! (G-G2-4 防降级硬门)。RD-013(图形=B 入口 body 数据流降级)已由 RXS-0171 + 本 device
//! 路径(消费 Rurix 图形=B DXIL 真出图)兑现闭环。

use std::fmt;

/// UC-04 deferred 装配/编排失败(strict-only,无运行期 fallback)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Uc04Error {
    /// RXS-0167 L2:PS 输出签名 ↔ 渲染目标格式集失配 / 深度状态矛盾(RX6018)。
    PsoTargetMismatch {
        /// 失配诊断上下文。
        detail: String,
    },
    /// RXS-0167 L3:着色器资源绑定反射 ↔ RTS0 推导意图失配(RX6019;复用
    /// `rurixc::binding_layout::check_binding_consistency`,P-11 单一事实源)。
    Rts0PsoMismatch {
        /// 失配诊断上下文。
        detail: String,
    },
    /// RXS-0168 L2:deferred pass 顺序 / MRT 目标 / SRV 输入缺失(RX6020)。
    PassOrchestration {
        /// 编排失败诊断上下文。
        detail: String,
    },
    /// RXS-0169 L2:缺 barrier / 非法资源状态转换(RX6021)。
    BarrierPlan {
        /// barrier 编排失败诊断上下文。
        detail: String,
    },
    /// RXS-0170 L2:offscreen readback 缓冲布局/格式失配(RX6022)。
    ReadbackLayout {
        /// readback 布局失败诊断上下文。
        detail: String,
    },
    /// RXS-0220 L2:UC-04 可见窗口 present 装配核验失败(swapchain desc ↔ final RT 格式/
    /// 缓冲数失配 / blt-model 或不支持 swap effect / 缺 PRESENT 态迁移锚点;RX6027)。
    PresentAssembly {
        /// present 装配失败诊断上下文。
        detail: String,
    },
    /// RXS-0221 L2:UC-04 swapchain 重建核验失败(重建后格式/缓冲数漂移 / 视图未重建;
    /// RX6028;失效=正常路径,但重建违例装配期显式拒)。
    ResizeRebuild {
        /// 重建核验失败诊断上下文。
        detail: String,
    },
    /// device 段:`real-shim`(D3D12 离屏 shim)未编入 / pin 工具缺失 → 无法真跑。
    /// **非语言 RX**(环境失败,不滥发诊断码);按 G-G2-4 防降级硬门标环境缺失,不伪造 device 绿。
    ShimUnavailable {
        /// 缺失上下文(缺 real-shim feature / MSVC / D3D12)。
        detail: String,
    },
    /// device 段:D3D12 shim 真跑失败(adapter/PSO/draw/readback 返回非 0,或像素对照失败)。
    /// **非语言 RX**(运行期/环境失败,不滥发诊断码)。
    DeviceRunFailed {
        /// shim 返回码(HRESULT 位码或哨兵负码;0 表示像素对照失败)。
        code: i32,
        /// 失败上下文。
        detail: String,
    },
}

impl Uc04Error {
    /// 装配期可预测错误对应的 6xxx 诊断码(RXS-0167~0170)。
    ///
    /// [`Uc04Error::ShimUnavailable`] / [`Uc04Error::DeviceRunFailed`] 为 device 段 sentinel,
    /// **非**语言诊断码 → `None`(D3D12 纯运行期/环境失败不滥发语言 RX,06 §8.2 /
    /// spec/d3d12_runtime.md §0)。
    pub fn rx_code(&self) -> Option<&'static str> {
        match self {
            Uc04Error::PsoTargetMismatch { .. } => Some("RX6018"),
            Uc04Error::Rts0PsoMismatch { .. } => Some("RX6019"),
            Uc04Error::PassOrchestration { .. } => Some("RX6020"),
            Uc04Error::BarrierPlan { .. } => Some("RX6021"),
            Uc04Error::ReadbackLayout { .. } => Some("RX6022"),
            Uc04Error::PresentAssembly { .. } => Some("RX6027"),
            Uc04Error::ResizeRebuild { .. } => Some("RX6028"),
            Uc04Error::ShimUnavailable { .. } | Uc04Error::DeviceRunFailed { .. } => None,
        }
    }
}

impl fmt::Display for Uc04Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Uc04Error::PsoTargetMismatch { detail } => {
                write!(f, "UC-04 PSO 装配不一致(渲染目标): {detail}")
            }
            Uc04Error::Rts0PsoMismatch { detail } => {
                write!(f, "UC-04 RTS0 ↔ PSO 绑定不一致: {detail}")
            }
            Uc04Error::PassOrchestration { detail } => {
                write!(f, "UC-04 deferred pass 编排失败: {detail}")
            }
            Uc04Error::BarrierPlan { detail } => {
                write!(f, "UC-04 barrier 编排失败: {detail}")
            }
            Uc04Error::ReadbackLayout { detail } => {
                write!(f, "UC-04 readback 布局失配: {detail}")
            }
            Uc04Error::PresentAssembly { detail } => {
                write!(f, "UC-04 可见窗口 present 装配不一致: {detail}")
            }
            Uc04Error::ResizeRebuild { detail } => {
                write!(f, "UC-04 swapchain 重建核验失败: {detail}")
            }
            Uc04Error::ShimUnavailable { detail } => {
                write!(
                    f,
                    "UC-04 device shim 不可用(real-shim/MSVC/D3D12 缺失): {detail}"
                )
            }
            Uc04Error::DeviceRunFailed { code, detail } => {
                write!(f, "UC-04 device 真跑失败(code={code}): {detail}")
            }
        }
    }
}

impl std::error::Error for Uc04Error {}

#[cfg(test)]
mod tests {
    use super::*;

    /// 装配期错误映射其专属 6xxx 码;device 阻塞 sentinel 非语言码(None)。
    //@ spec: RXS-0167
    #[test]
    fn rx_code_mapping_is_stable() {
        assert_eq!(
            Uc04Error::PsoTargetMismatch {
                detail: String::new()
            }
            .rx_code(),
            Some("RX6018")
        );
        assert_eq!(
            Uc04Error::Rts0PsoMismatch {
                detail: String::new()
            }
            .rx_code(),
            Some("RX6019")
        );
        assert_eq!(
            Uc04Error::PassOrchestration {
                detail: String::new()
            }
            .rx_code(),
            Some("RX6020")
        );
        assert_eq!(
            Uc04Error::BarrierPlan {
                detail: String::new()
            }
            .rx_code(),
            Some("RX6021")
        );
        assert_eq!(
            Uc04Error::ReadbackLayout {
                detail: String::new()
            }
            .rx_code(),
            Some("RX6022")
        );
        // RXS-0220/0221:present 装配 / swapchain 重建核验失败(G3.2 present 面)。
        assert_eq!(
            Uc04Error::PresentAssembly {
                detail: String::new()
            }
            .rx_code(),
            Some("RX6027")
        );
        assert_eq!(
            Uc04Error::ResizeRebuild {
                detail: String::new()
            }
            .rx_code(),
            Some("RX6028")
        );
        // device 段 sentinel(shim 缺失 / 真跑失败)= 环境/运行期失败,非语言诊断码。
        assert_eq!(
            Uc04Error::ShimUnavailable {
                detail: String::new()
            }
            .rx_code(),
            None
        );
        assert_eq!(
            Uc04Error::DeviceRunFailed {
                code: -1,
                detail: String::new()
            }
            .rx_code(),
            None
        );
    }
}
