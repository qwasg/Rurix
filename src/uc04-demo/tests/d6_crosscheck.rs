//! D6 互证金标准(RFC-0013 §4.D6,RXS-0239/0241):**两独立实现互证**。
//!
//! uc04 deferred 三 pass 图(`deferred::plan_deferred_passes` / RXS-0168 结构)经 rurix-rt
//! `graph.rs`(RXS-0238 纯 host 自动状态推导)推导出的 barrier 集,与 uc04 手动
//! `barrier::plan_barriers`(RXS-0169 / RX6021,main 已冻结)的 `required_transitions` 手动锚点集
//! **集合相等断言(双向)**。恒跑纯 host,无 GPU。
//!
//! **oracle 独立性**:`graph.rs` 禁止 import `barrier.rs` 任何推导逻辑;本互证是两条独立代码路
//! 对同一 deferred 场景的 barrier 计划一致性证明。`barrier.rs` 条款 0-byte 不动(D6 oracle)。
//!
//! **depth 建模说明**:uc04 冻结 oracle `barrier::ResourceState` 无 depth 变体,`required_transitions`
//! 对 `lighting_srv`(含 `Depth`)统一发 `RenderTarget → PixelShaderResource`。故本互证把 depth
//! G-buffer 资源以 graph.rs `color_target`(RENDER_TARGET)建模,逐字镜像 oracle 处置;graph.rs
//! `writes_depth`(`DEPTH_WRITE`)路由由 rurix-rt graph.rs 自身 golden 单测覆盖。

use rurix_rt::graph::{D3d12State, Graph, PassSpec, PlannedBarrier};
use uc04_demo::barrier::{self, BarrierTransition, ResourceState};
use uc04_demo::deferred::{self, DeferredGraph, DeferredPlan, GBufferTarget, Pass};

/// 标准 uc04 deferred 图(RXS-0168):几何 MRT(albedo+normal+depth)→ lighting 采样 → readback。
fn deferred_graph_input() -> DeferredGraph {
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

/// 由 RXS-0168 `DeferredPlan` 构造等价的 rurix-rt `graph.rs` 图(声明序 = 提交序)。
/// 资源命名逐字对齐 `barrier::required_transitions` 的 `gbuf:{target:?}` / `lighting_out` /
/// `readback`,使推导产物可与 oracle 锚点集直接比对。
fn build_graph_from_plan(plan: &DeferredPlan) -> Graph {
    let mut g = Graph::new();
    // 被 lighting 采样(= 会经历 RT→SRV 转换)的 G-buffer 资源集,逐一建 color_target。
    let gbuf_ids: Vec<_> = plan
        .lighting_srv
        .iter()
        .map(|t| g.color_target(&format!("gbuf:{t:?}")))
        .collect();
    let lit = g.color_target("lighting_out");
    let readback = g.readback_buffer("readback");

    // 几何 pass:写全部 G-buffer 目标(RT)。
    let mut geo = PassSpec::new("geometry");
    for id in &gbuf_ids {
        geo = geo.writes_rt(*id);
    }
    g.add_pass(geo).expect("geometry pass");

    // lighting pass:采样全部 G-buffer(RT→SRV)+ 写 lighting 输出。
    let mut light = PassSpec::new("lighting");
    for id in &gbuf_ids {
        light = light.reads(*id);
    }
    light = light.writes_rt(lit);
    g.add_pass(light).expect("lighting pass");

    // readback:lighting 输出 → COPY_SOURCE + readback buffer → COPY_DEST。
    g.readback(lit, readback).expect("readback pass");
    g
}

/// graph.rs D3D12 状态 → uc04 oracle `ResourceState`(D6 场景内不应出现其余状态)。
fn to_uc04_state(s: D3d12State) -> ResourceState {
    match s {
        D3d12State::Common => ResourceState::Common,
        D3d12State::RenderTarget => ResourceState::RenderTarget,
        D3d12State::PixelShaderResource => ResourceState::PixelShaderResource,
        D3d12State::CopySource => ResourceState::CopySource,
        D3d12State::CopyDest => ResourceState::CopyDest,
        other => panic!("D6 deferred 图不应出现状态 {other:?}(depth 建模为 RT,见模块注释)"),
    }
}

/// graph.rs `PlannedBarrier` → uc04 `BarrierTransition`(经 D3D12 视图逐字映射)。
fn to_uc04_transition(b: &PlannedBarrier) -> BarrierTransition {
    BarrierTransition {
        resource: b.resource_name.clone(),
        from: to_uc04_state(b.d3d12_before),
        to: to_uc04_state(b.d3d12_after),
    }
}

/// **D6 互证金标准**:graph.rs 推导集 == uc04 `plan_barriers` RXS-0169 手动锚点集(双向)。
//@ spec: RXS-0239, RXS-0241
#[test]
fn graph_derivation_equals_manual_barrier_oracle() {
    // 单一 deferred 计划(RXS-0168)喂两条独立实现。
    let plan: DeferredPlan =
        deferred::plan_deferred_passes(&deferred_graph_input()).expect("合法 deferred 编排");

    // 实现 A:rurix-rt graph.rs 纯 host 自动状态推导(RXS-0238)。
    let mut g = build_graph_from_plan(&plan);
    let derived: Vec<PlannedBarrier> = g.execute().expect("合法图应 execute 通过");
    let derived_transitions: Vec<BarrierTransition> =
        derived.iter().map(to_uc04_transition).collect();

    // 实现 B(oracle):uc04 手动 barrier 核验器(RXS-0169)。以 graph.rs 推导集作 provided,
    // plan_barriers 成功即证「required ⊆ 推导集 且 全部合法」;返回 anchors = required 集。
    let anchors = barrier::plan_barriers(&plan, &derived_transitions)
        .expect("graph.rs 推导集须覆盖全部所需转换且全部为合法 D3D12 转换");
    let oracle_transitions: Vec<BarrierTransition> =
        anchors.iter().map(|a| a.transition.clone()).collect();

    // 双向集合相等:① 无重复(资源名唯一);② 逐元素双向 contains。
    assert_eq!(
        derived_transitions.len(),
        oracle_transitions.len(),
        "推导集与 oracle 集基数须相等:graph={derived_transitions:?} oracle={oracle_transitions:?}"
    );
    for t in &derived_transitions {
        assert!(
            oracle_transitions.contains(t),
            "graph.rs 推导出 oracle 无的 barrier:{t:?}"
        );
    }
    for t in &oracle_transitions {
        assert!(
            derived_transitions.contains(t),
            "oracle 有 graph.rs 未推导的 barrier:{t:?}"
        );
    }

    // 具体锚点存在性(RXS-0169 五转换:3×RT→PSR + lighting_out RT→CopySource + readback Common→CopyDest)。
    assert_eq!(derived_transitions.len(), 5, "deferred 图恰 5 条 barrier");
    assert!(derived_transitions.contains(&BarrierTransition {
        resource: "lighting_out".to_owned(),
        from: ResourceState::RenderTarget,
        to: ResourceState::CopySource,
    }));
    assert!(derived_transitions.contains(&BarrierTransition {
        resource: "readback".to_owned(),
        from: ResourceState::Common,
        to: ResourceState::CopyDest,
    }));
}
