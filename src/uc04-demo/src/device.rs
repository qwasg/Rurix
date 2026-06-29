//! device 执行入口(gate `d3d12-runtime`;RFC-0006 §9 Q-Gate)。
//!
//! **blocked-honest(G-G2-4 防降级硬门)**:hardware 多 pass deferred draw + offscreen
//! readback + 像素对照须 Rurix source → 图形=B DXIL(RD-013)→ RFC-0005 RTS0 → D3D12 PSO
//! → hardware 多 pass deferred draw → offscreen readback 全链兑现。前置 RD-013 open
//! (图形=B 入口 body 数据流降级未实现 → `rurixc::dxil_spirv::emit_spirv` 仅产接口 + 平凡
//! `main`,无 Rurix 自产可出图着色器)→ 本入口**显式返回 [`Uc04Error::BlockedOnRd013`]**,
//! **不**以手写 HLSL/DXIL、CPU 预填、单 pass、fullscreen copy、固定像素、host-only 模拟、
//! 窗口截图或 SKIP 伪造 device 绿。真实 D3D12 执行 + CI step 48 + golden bless + device
//! run URL + G-G2-4 签字归 RD-013 解锁后的 device PR + owner。

use crate::barrier::BarrierAnchor;
use crate::deferred::DeferredPlan;
use crate::error::Uc04Error;
use crate::pso::AssembledPso;
use crate::readback::ReadbackLayout;

/// offscreen 出图请求(host 侧已校验的装配/编排/barrier/readback 产物聚合)。
pub struct OffscreenRequest<'a> {
    /// RXS-0167 装配出的 graphics PSO 描述。
    pub pso: &'a AssembledPso,
    /// RXS-0168 校验通过的 deferred 编排计划。
    pub plan: &'a DeferredPlan,
    /// RXS-0169 校验通过的 barrier 锚点集。
    pub barriers: &'a [BarrierAnchor],
    /// RXS-0170 校验通过的 readback 布局。
    pub readback: &'a ReadbackLayout,
}

/// device offscreen 出图 + 像素回读(**blocked-honest**:阻塞于 RD-013,显式返回
/// [`Uc04Error::BlockedOnRd013`],不伪造 device 绿)。
///
/// # Errors
/// 恒返回 [`Uc04Error::BlockedOnRd013`]:无 Rurix 自产可出图着色器(RD-013 open)。
/// 待 RD-013 解锁后由 device PR 接通真实 D3D12 执行(G-G2-4 防降级硬门约束下)。
pub fn execute_offscreen(req: &OffscreenRequest<'_>) -> Result<Vec<u8>, Uc04Error> {
    // host 侧装配/编排/barrier/readback 均已校验(RXS-0167~0170),但 device 真跑须 Rurix
    // 自产 DXIL 出图(RD-013)→ 当前阻塞,不以替代物伪造。
    let _ = (req.pso, req.plan, req.barriers, req.readback);
    Err(Uc04Error::BlockedOnRd013 {
        detail: "hardware 多 pass deferred draw + offscreen readback 须 Rurix source → 图形=B \
                 DXIL(RD-013)出图;RD-013 open,无 Rurix 自产可出图着色器 → 按 G-G2-4 防降级硬门标 \
                 blocked,不以替代物伪造 device 绿"
            .to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Format;
    use crate::barrier::{BarrierAnchor, BarrierTransition, ResourceState};
    use crate::deferred::{DeferredPlan, GBufferTarget};
    use crate::pso::AssembledPso;
    use crate::readback::ReadbackLayout;
    use rurixc::binding_layout::RootSignature;

    /// device 段 blocked-honest:execute_offscreen 显式返回 BlockedOnRd013(非语言 RX),
    /// **不**伪造 device 绿(G-G2-4 防降级硬门)。
    //@ spec: RXS-0170
    #[test]
    fn device_path_is_blocked_on_rd013() {
        let pso = AssembledPso {
            root_signature: RootSignature {
                parameters: Vec::new(),
                flags: 0,
            },
            rts0_bytes: Vec::new(),
            rtv_formats: vec![Format::Rgba8Unorm],
            dsv_format: Some(Format::D32Float),
        };
        let plan = DeferredPlan {
            gbuffer_color: vec![GBufferTarget::Albedo, GBufferTarget::Normal],
            has_depth: true,
            lighting_srv: vec![GBufferTarget::Albedo, GBufferTarget::Normal],
        };
        let barriers = vec![BarrierAnchor {
            at: "after-lighting",
            transition: BarrierTransition {
                resource: "lighting_out".to_owned(),
                from: ResourceState::RenderTarget,
                to: ResourceState::CopySource,
            },
        }];
        let readback = ReadbackLayout {
            row_pitch: 256,
            buffer_size: 256 * 64,
            format: Format::Rgba8Unorm,
        };
        let req = OffscreenRequest {
            pso: &pso,
            plan: &plan,
            barriers: &barriers,
            readback: &readback,
        };
        let err = execute_offscreen(&req).expect_err("device 段须 blocked-on-RD-013");
        assert!(matches!(err, Uc04Error::BlockedOnRd013 { .. }));
        // blocked-honest sentinel 非语言诊断码(不伪造 device 绿、不滥发 RX)。
        assert_eq!(err.rx_code(), None);
    }
}
