//! `uc04-demo` 可执行入口(blocked-honest interim slice)。
//!
//! 跑 host 侧装配/编排模型(RXS-0167~0170:PSO 装配 → deferred 编排 → barrier 锚点 →
//! readback 布局)并打印结果,然后显式声明 **device 段 blocked-on-RD-013**(不伪造
//! device 绿,G-G2-4 防降级硬门)。无 MSVC/D3D12 SDK 即可编译运行(纯 host)。

use uc04_demo::Format;
use uc04_demo::barrier::{BarrierTransition, ResourceState, plan_barriers};
use uc04_demo::deferred::{DeferredGraph, GBufferTarget, Pass, plan_deferred_passes};
use uc04_demo::pso::{GraphicsPsoDesc, assemble_graphics_pso};
use uc04_demo::readback::{ReadbackRequest, plan_readback};

use rurixc::binding_layout::{Psv0Reflection, Psv0Resource, infer_register_assignments};
use rurixc::hir::PrimTy;
use rurixc::mir::{MirResourceType, ResourceBinding, ResourceCount};

fn rb(name: &str, res: MirResourceType) -> ResourceBinding {
    ResourceBinding {
        name: name.to_owned(),
        res,
        count: ResourceCount::One,
    }
}

fn main() {
    println!("UC-04 deferred 渲染器 demo — blocked-honest interim slice (RXS-0167~0170)");

    // RXS-0167:lighting pass PSO 装配(CBV + 3 G-buffer SRV + Sampler,单 RT 输出 + 深度)。
    let resources = vec![
        rb("light_params", MirResourceType::ConstantBuffer),
        rb("g_albedo", MirResourceType::Texture2D(PrimTy::F32)),
        rb("g_normal", MirResourceType::Texture2D(PrimTy::F32)),
        rb("g_depth", MirResourceType::Texture2D(PrimTy::F32)),
        rb("g_samp", MirResourceType::Sampler),
    ];
    let intent = infer_register_assignments(&resources).expect("绑定布局推导(RFC-0005)");
    let reflected = Psv0Reflection {
        resources: intent
            .iter()
            .map(|a| Psv0Resource {
                class: a.class,
                register: a.register,
                space: a.space,
                count: a.span,
            })
            .collect(),
    };
    let desc = GraphicsPsoDesc {
        resources,
        reflected,
        ps_render_target_outputs: 1,
        rtv_formats: vec![Format::Rgba8Unorm],
        dsv_format: Some(Format::D32Float),
        depth_write: false,
    };
    let pso = assemble_graphics_pso(&desc).expect("PSO 装配一致");
    println!(
        "[RXS-0167] graphics PSO 装配:root params={} RTS0={}B",
        pso.root_signature.parameters.len(),
        pso.rts0_bytes.len()
    );

    // RXS-0168:deferred 多 pass 编排。
    let graph = DeferredGraph {
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
    };
    let plan = plan_deferred_passes(&graph).expect("deferred 编排合法");
    println!(
        "[RXS-0168] deferred 编排:G-buffer color={} depth={} lighting SRV={}",
        plan.gbuffer_color.len(),
        plan.has_depth,
        plan.lighting_srv.len()
    );

    // RXS-0169:手动 barrier 编排锚点。
    let mut barriers: Vec<BarrierTransition> = plan
        .lighting_srv
        .iter()
        .map(|g| BarrierTransition {
            resource: format!("gbuf:{g:?}"),
            from: ResourceState::RenderTarget,
            to: ResourceState::PixelShaderResource,
        })
        .collect();
    barriers.push(BarrierTransition {
        resource: "lighting_out".to_owned(),
        from: ResourceState::RenderTarget,
        to: ResourceState::CopySource,
    });
    barriers.push(BarrierTransition {
        resource: "readback".to_owned(),
        from: ResourceState::Common,
        to: ResourceState::CopyDest,
    });
    let anchors = plan_barriers(&plan, &barriers).expect("barrier 编排覆盖全部所需转换");
    println!("[RXS-0169] barrier 锚点:{} 个", anchors.len());

    // RXS-0170:offscreen readback 布局(64×64 RGBA8)。
    let layout = plan_readback(&ReadbackRequest {
        width: 64,
        height: 64,
        src_format: Format::Rgba8Unorm,
        dst_format: Format::Rgba8Unorm,
        row_pitch: 256,
        buffer_size: 256 * 64,
    })
    .expect("readback 布局合法");
    println!(
        "[RXS-0170] readback 布局:row_pitch={} buffer={}B",
        layout.row_pitch, layout.buffer_size
    );

    // device 段:blocked-honest(G-G2-4 防降级硬门)。
    println!(
        "[device] blocked-on-RD-013:hardware 多 pass deferred draw + offscreen 像素对照须 Rurix \
         source → 图形=B DXIL 出图;RD-013 open → 不伪造 device 绿、不签 G-G2-4。"
    );
    #[cfg(feature = "d3d12-runtime")]
    {
        use uc04_demo::device::{OffscreenRequest, execute_offscreen};
        let req = OffscreenRequest {
            pso: &pso,
            plan: &plan,
            barriers: &anchors,
            readback: &layout,
        };
        match execute_offscreen(&req) {
            Err(e) => println!("[device] execute_offscreen → {e}"),
            Ok(_) => unreachable!("device 段 blocked-honest,不应返回 Ok"),
        }
    }
}
