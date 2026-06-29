//! RXS-0169:资源状态 + barrier 编排锚点(host 侧 safe 核验)。
//!
//! §9 Q-Barrier 首期**手动** barrier 编排:pass 间 G-buffer 资源状态转换
//! (`RENDER_TARGET` → `PIXEL_SHADER_RESOURCE` → Copy / Readback)由运行时显式插入;
//! 不做自动状态跟踪(defer → RD-020)。本模块只核验**编排锚点的存在性与状态转换合法性**:
//! 给定 deferred 计划推出所需转换,核验调用方提供的 barrier 计划逐一覆盖且合法;缺
//! barrier / 非法转换 → strict-only 显式错。
//!
//! 🔒 **barrier 的并发 / 可见性 / 内存序语义本体不在本模块**——happens-before / 跨队列
//! 可见性 / 缓存刷新语义触及即停手标「需升档」(agent Full RFC)。本模块仅核验锚点
//! 存在性与转换合法性,不定义并发内存模型。

use crate::deferred::DeferredPlan;
use crate::error::Uc04Error;

/// D3D12 资源状态(编排锚点用归类;🔒 非并发内存模型语义本体)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceState {
    /// `D3D12_RESOURCE_STATE_COMMON`。
    Common,
    /// `D3D12_RESOURCE_STATE_RENDER_TARGET`。
    RenderTarget,
    /// `D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE`。
    PixelShaderResource,
    /// `D3D12_RESOURCE_STATE_COPY_SOURCE`。
    CopySource,
    /// `D3D12_RESOURCE_STATE_COPY_DEST`。
    CopyDest,
}

/// 单个资源状态转换(barrier)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarrierTransition {
    /// 资源标识(诊断用名,非物理布局)。
    pub resource: String,
    /// 源状态。
    pub from: ResourceState,
    /// 目标状态。
    pub to: ResourceState,
}

/// 校验通过的 barrier 锚点(编排位置 + 转换)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarrierAnchor {
    /// 编排位置标签(pass 边界)。
    pub at: &'static str,
    /// 该锚点的状态转换。
    pub transition: BarrierTransition,
}

/// 某状态转换是否为本模型核验的合法 D3D12 转换(锚点级,非并发语义本体)。
fn is_legal_transition(from: ResourceState, to: ResourceState) -> bool {
    use ResourceState::*;
    if from == to {
        return false; // 自转换非法(无意义 barrier)。
    }
    matches!(
        (from, to),
        (RenderTarget, PixelShaderResource)
            | (PixelShaderResource, RenderTarget)
            | (RenderTarget, CopySource)
            | (CopySource, RenderTarget)
            | (Common, CopyDest)
            | (CopyDest, Common)
            | (Common, RenderTarget)
            | (RenderTarget, Common)
    )
}

/// 由 deferred 计划推出所需的资源状态转换锚点(首期手动编排的 ground-truth)。
fn required_transitions(plan: &DeferredPlan) -> Vec<(&'static str, BarrierTransition)> {
    let mut req = Vec::new();
    // 几何 pass 后:被 lighting 采样的 G-buffer 资源 RT → SRV。
    for srv in &plan.lighting_srv {
        req.push((
            "after-geometry",
            BarrierTransition {
                resource: format!("gbuf:{srv:?}"),
                from: ResourceState::RenderTarget,
                to: ResourceState::PixelShaderResource,
            },
        ));
    }
    // lighting pass 后:lighting 输出 RT → COPY_SOURCE(供 readback)。
    req.push((
        "after-lighting",
        BarrierTransition {
            resource: "lighting_out".to_owned(),
            from: ResourceState::RenderTarget,
            to: ResourceState::CopySource,
        },
    ));
    // readback 目标 buffer → COPY_DEST。
    req.push((
        "before-readback",
        BarrierTransition {
            resource: "readback".to_owned(),
            from: ResourceState::Common,
            to: ResourceState::CopyDest,
        },
    ));
    req
}

/// RXS-0169:核验手动 barrier 编排是否覆盖所有所需状态转换且合法。
///
/// # Errors
/// `provided` 含非法转换(自转换 / 非合法 D3D12 转换)或缺某所需转换 →
/// [`Uc04Error::BarrierPlan`](RX6021)。首期手动编排,**不**自动补 barrier(自动状态
/// 跟踪 defer RD-020)。
pub fn plan_barriers(
    plan: &DeferredPlan,
    provided: &[BarrierTransition],
) -> Result<Vec<BarrierAnchor>, Uc04Error> {
    // 每个提供的转换须合法。
    for t in provided {
        if !is_legal_transition(t.from, t.to) {
            return Err(Uc04Error::BarrierPlan {
                detail: format!(
                    "非法资源状态转换:`{}` {:?} → {:?}",
                    t.resource, t.from, t.to
                ),
            });
        }
    }
    // 每个所需转换须在 provided 中存在(手动编排,缺即红)。
    let required = required_transitions(plan);
    let mut anchors = Vec::with_capacity(required.len());
    for (at, req) in required {
        if !provided.contains(&req) {
            return Err(Uc04Error::BarrierPlan {
                detail: format!(
                    "缺 barrier:`{}` {:?} → {:?}(在 {at})",
                    req.resource, req.from, req.to
                ),
            });
        }
        anchors.push(BarrierAnchor {
            at,
            transition: req,
        });
    }
    Ok(anchors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deferred::GBufferTarget;

    /// 最小 deferred 计划:albedo+normal color + depth,lighting 采样 albedo+normal+depth。
    fn plan() -> DeferredPlan {
        DeferredPlan {
            gbuffer_color: vec![GBufferTarget::Albedo, GBufferTarget::Normal],
            has_depth: true,
            lighting_srv: vec![
                GBufferTarget::Albedo,
                GBufferTarget::Normal,
                GBufferTarget::Depth,
            ],
        }
    }

    /// 完整的手动 barrier 计划(覆盖全部所需转换且合法)。
    fn full_barriers() -> Vec<BarrierTransition> {
        let mut v: Vec<BarrierTransition> = [
            GBufferTarget::Albedo,
            GBufferTarget::Normal,
            GBufferTarget::Depth,
        ]
        .iter()
        .map(|g| BarrierTransition {
            resource: format!("gbuf:{g:?}"),
            from: ResourceState::RenderTarget,
            to: ResourceState::PixelShaderResource,
        })
        .collect();
        v.push(BarrierTransition {
            resource: "lighting_out".to_owned(),
            from: ResourceState::RenderTarget,
            to: ResourceState::CopySource,
        });
        v.push(BarrierTransition {
            resource: "readback".to_owned(),
            from: ResourceState::Common,
            to: ResourceState::CopyDest,
        });
        v
    }

    /// accept:完整状态转换 → 合法 barrier 锚点集(覆盖全部所需)。
    //@ spec: RXS-0169
    #[test]
    fn plans_full_barrier_set() {
        let anchors = plan_barriers(&plan(), &full_barriers()).expect("完整 barrier 应通过");
        // 3 个 G-buffer SRV 转换 + lighting_out + readback = 5 个锚点。
        assert_eq!(anchors.len(), 5);
        assert!(anchors.iter().any(|a| a.at == "after-lighting"));
    }

    /// reject:漏一个所需 G-buffer RT→SRV barrier → BarrierPlan(RX6021)。
    //@ spec: RXS-0169
    #[test]
    fn rejects_missing_barrier() {
        let mut barriers = full_barriers();
        barriers.remove(0); // 漏掉 albedo RT→SRV
        match plan_barriers(&plan(), &barriers) {
            Err(e @ Uc04Error::BarrierPlan { .. }) => assert_eq!(e.rx_code(), Some("RX6021")),
            other => panic!("缺 barrier 应 BarrierPlan,实得 {other:?}"),
        }
    }

    /// reject:非法状态转换(PSR → COPY_DEST 跳态)→ BarrierPlan(RX6021)。
    //@ spec: RXS-0169
    #[test]
    fn rejects_illegal_transition() {
        let mut barriers = full_barriers();
        barriers.push(BarrierTransition {
            resource: "gbuf:Albedo".to_owned(),
            from: ResourceState::PixelShaderResource,
            to: ResourceState::CopyDest, // 非法跳态
        });
        assert!(matches!(
            plan_barriers(&plan(), &barriers),
            Err(Uc04Error::BarrierPlan { .. })
        ));
    }

    /// reject:自转换(from == to)非法 → BarrierPlan。
    //@ spec: RXS-0169
    #[test]
    fn rejects_self_transition() {
        let barriers = vec![BarrierTransition {
            resource: "gbuf:Albedo".to_owned(),
            from: ResourceState::RenderTarget,
            to: ResourceState::RenderTarget,
        }];
        assert!(matches!(
            plan_barriers(&plan(), &barriers),
            Err(Uc04Error::BarrierPlan { .. })
        ));
    }
}
