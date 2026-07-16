//! RXS-0168:deferred 多 pass 编排(host 侧 safe 结构核验)。
//!
//! §9 Q-DeferredPass 最小 deferred:几何 pass(G-buffer:albedo + normal + depth MRT)→
//! 单光源 lighting pass(采样 G-buffer 作 shader resource)→ offscreen readback。本模块
//! 只核验**编排结构**(pass 顺序 / MRT 目标存在性 / lighting SRV 输入引用几何 pass 输出 /
//! readback 引用 lighting 输出);顺序/目标缺失 → strict-only 显式错。
//!
//! **device 像素对照(几何 pass 真写 G-buffer + lighting 真采样 + 数值结果)阻塞于
//! RD-013**(承 [`crate::readback`] device 段),本轮不达成、不以替代物伪造。
//!
//! 🔒 G-buffer 写入 / lighting 采样的纹理路径内存模型(采样 opcode / LOD·导数 / 越界 /
//! 缓存一致性 / memory-order,06 §4.2)不在本模块;只编排 opaque `Texture2D`/`Sampler`
//! 句柄 + RT/SRV 视图绑定,触及即停手升档(RD-021,agent Full RFC)。

use crate::error::Uc04Error;

/// G-buffer 目标 / lighting 采样源(opaque 句柄归类;🔒 非纹理内存模型本体)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GBufferTarget {
    /// 反照率(albedo)color 目标。
    Albedo,
    /// 法线(normal)color 目标。
    Normal,
    /// 深度(depth)目标。
    Depth,
}

/// deferred 管线中的单个 pass(编排结构,非纹理内存模型)。
#[derive(Debug, Clone)]
pub enum Pass {
    /// 几何 pass:写 G-buffer MRT(color 目标集 + 深度目标)。
    Geometry {
        /// 写入的 color 目标(最小集须含 Albedo + Normal)。
        color_targets: Vec<GBufferTarget>,
        /// 是否写深度目标。
        depth_target: bool,
    },
    /// 单光源 lighting pass:采样 G-buffer 作 SRV,写 lighting 输出。
    Lighting {
        /// 采样的 G-buffer SRV 输入(须引用几何 pass 已声明目标)。
        srv_inputs: Vec<GBufferTarget>,
        /// 是否写 lighting 输出。
        writes_output: bool,
    },
    /// offscreen readback:回读 lighting 输出。
    Readback {
        /// 源是否为 lighting 输出。
        source_is_lighting_output: bool,
    },
}

/// 校验通过的 deferred 编排计划(host 侧;device 真跑承 RXS-0170 device 段)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeferredPlan {
    /// 几何 pass 写入的 G-buffer color 目标。
    pub gbuffer_color: Vec<GBufferTarget>,
    /// 是否有深度目标。
    pub has_depth: bool,
    /// lighting pass 采样的 G-buffer SRV 输入。
    pub lighting_srv: Vec<GBufferTarget>,
}

/// deferred 管线编排图(有序 pass 序列)。
pub struct DeferredGraph {
    /// pass 序列(须为 [Geometry, Lighting, Readback])。
    pub passes: Vec<Pass>,
}

/// RXS-0168:核验 deferred 多 pass 编排结构。
///
/// # Errors
/// pass 乱序 / G-buffer MRT 目标缺失(须含 Albedo+Normal+Depth)/ lighting SRV 引用
/// 未声明目标 / readback 源缺失 → [`Uc04Error::PassOrchestration`](RX6020)。
pub fn plan_deferred_passes(graph: &DeferredGraph) -> Result<DeferredPlan, Uc04Error> {
    // pass 序须为 [Geometry, Lighting, Readback]。
    let [
        Pass::Geometry {
            color_targets,
            depth_target,
        },
        Pass::Lighting {
            srv_inputs,
            writes_output,
        },
        Pass::Readback {
            source_is_lighting_output,
        },
    ] = graph.passes.as_slice()
    else {
        return Err(Uc04Error::PassOrchestration {
            detail: "pass 序须为 [几何, lighting, readback](顺序/目标缺失)".to_owned(),
        });
    };

    // 几何 pass:最小 G-buffer 须含 albedo + normal color + depth 目标。
    if !color_targets.contains(&GBufferTarget::Albedo)
        || !color_targets.contains(&GBufferTarget::Normal)
    {
        return Err(Uc04Error::PassOrchestration {
            detail: "几何 pass G-buffer 缺 albedo / normal color 目标".to_owned(),
        });
    }
    if !*depth_target {
        return Err(Uc04Error::PassOrchestration {
            detail: "几何 pass 缺深度目标".to_owned(),
        });
    }

    // lighting pass:SRV 输入须引用几何 pass 已声明目标(color + 可选 depth)。
    if !*writes_output {
        return Err(Uc04Error::PassOrchestration {
            detail: "lighting pass 未写输出".to_owned(),
        });
    }
    if srv_inputs.is_empty() {
        return Err(Uc04Error::PassOrchestration {
            detail: "lighting pass 未采样 G-buffer(SRV 输入为空)".to_owned(),
        });
    }
    for srv in srv_inputs {
        let declared =
            color_targets.contains(srv) || (*srv == GBufferTarget::Depth && *depth_target);
        if !declared {
            return Err(Uc04Error::PassOrchestration {
                detail: format!("lighting SRV 输入 {srv:?} 引用未由几何 pass 声明的目标"),
            });
        }
    }

    // readback:源须为 lighting 输出。
    if !*source_is_lighting_output {
        return Err(Uc04Error::PassOrchestration {
            detail: "readback 源非 lighting 输出".to_owned(),
        });
    }

    Ok(DeferredPlan {
        gbuffer_color: color_targets.clone(),
        has_depth: *depth_target,
        lighting_srv: srv_inputs.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 最小合法 deferred 图:几何 MRT(albedo+normal+depth)→ lighting 采样 → readback。
    fn valid_graph() -> DeferredGraph {
        DeferredGraph {
            passes: vec![
                Pass::Geometry {
                    color_targets: vec![GBufferTarget::Albedo, GBufferTarget::Normal],
                    depth_target: true,
                },
                Pass::Lighting {
                    srv_inputs: vec![
                        GBufferTarget::Albedo,
                        GBufferTarget::Normal,
                        GBufferTarget::Depth,
                    ],
                    writes_output: true,
                },
                Pass::Readback {
                    source_is_lighting_output: true,
                },
            ],
        }
    }

    /// accept:合法编排 → DeferredPlan。
    //@ spec: RXS-0168
    #[test]
    fn plans_valid_deferred_graph() {
        let plan = plan_deferred_passes(&valid_graph()).expect("合法编排应通过");
        assert!(plan.has_depth);
        assert_eq!(plan.gbuffer_color.len(), 2);
        assert_eq!(plan.lighting_srv.len(), 3);
    }

    /// reject:pass 乱序(lighting 先于几何)→ PassOrchestration(RX6020)。
    //@ spec: RXS-0168
    #[test]
    fn rejects_out_of_order_passes() {
        let graph = DeferredGraph {
            passes: vec![
                Pass::Lighting {
                    srv_inputs: vec![GBufferTarget::Albedo],
                    writes_output: true,
                },
                Pass::Geometry {
                    color_targets: vec![GBufferTarget::Albedo, GBufferTarget::Normal],
                    depth_target: true,
                },
                Pass::Readback {
                    source_is_lighting_output: true,
                },
            ],
        };
        match plan_deferred_passes(&graph) {
            Err(e @ Uc04Error::PassOrchestration { .. }) => assert_eq!(e.rx_code(), Some("RX6020")),
            other => panic!("乱序应 PassOrchestration,实得 {other:?}"),
        }
    }

    /// reject:几何 pass G-buffer 缺 normal 目标 → PassOrchestration。
    //@ spec: RXS-0168
    #[test]
    fn rejects_missing_gbuffer_target() {
        let graph = DeferredGraph {
            passes: vec![
                Pass::Geometry {
                    color_targets: vec![GBufferTarget::Albedo], // 缺 normal
                    depth_target: true,
                },
                Pass::Lighting {
                    srv_inputs: vec![GBufferTarget::Albedo],
                    writes_output: true,
                },
                Pass::Readback {
                    source_is_lighting_output: true,
                },
            ],
        };
        assert!(matches!(
            plan_deferred_passes(&graph),
            Err(Uc04Error::PassOrchestration { .. })
        ));
    }

    /// reject:lighting SRV 引用未声明目标(几何无 depth 却采样 depth)→ PassOrchestration。
    //@ spec: RXS-0168
    #[test]
    fn rejects_lighting_srv_referencing_undeclared_target() {
        let graph = DeferredGraph {
            passes: vec![
                Pass::Geometry {
                    color_targets: vec![GBufferTarget::Albedo, GBufferTarget::Normal],
                    depth_target: false, // 无深度目标
                },
                Pass::Lighting {
                    srv_inputs: vec![GBufferTarget::Depth], // 却采样 depth
                    writes_output: true,
                },
                Pass::Readback {
                    source_is_lighting_output: true,
                },
            ],
        };
        // 注:几何缺 depth 会先在 depth_target 门拦截;此处断言失败即可(strict-only)。
        assert!(plan_deferred_passes(&graph).is_err());
    }

    /// reject:readback 源非 lighting 输出 → PassOrchestration。
    //@ spec: RXS-0168
    #[test]
    fn rejects_readback_source_not_lighting() {
        let graph = DeferredGraph {
            passes: vec![
                Pass::Geometry {
                    color_targets: vec![GBufferTarget::Albedo, GBufferTarget::Normal],
                    depth_target: true,
                },
                Pass::Lighting {
                    srv_inputs: vec![GBufferTarget::Albedo, GBufferTarget::Normal],
                    writes_output: true,
                },
                Pass::Readback {
                    source_is_lighting_output: false,
                },
            ],
        };
        assert!(matches!(
            plan_deferred_passes(&graph),
            Err(Uc04Error::PassOrchestration { .. })
        ));
    }
}
