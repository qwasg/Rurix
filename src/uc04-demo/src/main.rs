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

    // device 段(G2.4 选项 B:不采样 G-buffer 的最小多 pass deferred)。real-shim 下经
    // D3D12 离屏 shim 真出图:几何 pass(Rurix VS/FS)写 G-buffer MRT → lighting/合成 pass
    // (Rurix VS/FS,不采样)写 final → offscreen readback 中心像素对照;无 real-shim → 显式
    // ShimUnavailable(不伪造 device 绿,G-G2-4 防降级硬门)。
    let _ = (&pso, &plan, &anchors, &layout); // host 模型已打印;device 走 Rurix DXIL 真出图。
    #[cfg(feature = "d3d12-runtime")]
    device_dispatch();
}

/// device 子命令分发(`d3d12-runtime`):`present <...>` → 可见窗口 present(G3.2 RXS-0220~0222);
/// 否则 → offscreen(G2.4 RXS-0167~0170)。
#[cfg(feature = "d3d12-runtime")]
fn device_dispatch() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("present") {
        present_run();
    } else {
        device_run();
    }
}

/// device 真出图(`d3d12-runtime`):读 4 个 Rurix 图形=B DXIL(命令行给出)+ 构造**每 pass**
/// RFC-0005 RTS0(几何 = 空资源 + IA flag;lighting = SRV t0 + Sampler s0,经 `infer_root_signature`
/// → `serialize_rts0` 单一事实源,RFC-0007 真采样)→ execute_offscreen → 打印 `DXIL_UC04` 见证行 /
/// 显式错误(非伪造)。
#[cfg(feature = "d3d12-runtime")]
fn device_run() {
    use uc04_demo::Format;
    use uc04_demo::barrier::{BarrierAnchor, BarrierTransition, ResourceState};
    use uc04_demo::deferred::{DeferredPlan, GBufferTarget};
    use uc04_demo::device::{OffscreenRequest, OffscreenResult, execute_offscreen};
    use uc04_demo::pso::AssembledPso;
    use uc04_demo::readback::ReadbackLayout;

    use rurixc::binding_layout::{infer_root_signature, serialize_rts0};

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 {
        eprintln!(
            "[device] usage: uc04-demo <geom_vs.dxil> <geom_fs.dxil> <light_vs.dxil> <light_fs.dxil>"
        );
        std::process::exit(2);
    }
    let read = |p: &str| -> Vec<u8> {
        std::fs::read(p).unwrap_or_else(|e| {
            eprintln!("[device] 读 DXIL {p} 失败: {e}");
            std::process::exit(2);
        })
    };
    let geom_vs = read(&args[1]);
    let geom_fs = read(&args[2]);
    let light_vs = read(&args[3]);
    let light_fs = read(&args[4]);

    // 几何 pass RFC-0005 RTS0(P-11 单一事实源):无资源 → 空 root signature;加
    // ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT(0x1)以承顶点缓冲输入布局(带 IA 的 PSO 必需)。
    let mut rs =
        infer_root_signature(&[]).expect("几何 pass 空资源集 root signature 推导(RFC-0005)");
    rs.flags = 0x1; // D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT
    let rts0_bytes = serialize_rts0(&rs);

    // lighting pass RFC-0005 RTS0:SRV t0(albedo)+ Sampler s0,经 infer_root_signature 推导
    // (RFC-0007 真采样 G-buffer;descriptor table 由 device CreateRootSignature 真机解析)。
    let light_resources = vec![
        rb("g_albedo", MirResourceType::Texture2D(PrimTy::F32)),
        rb("g_samp", MirResourceType::Sampler),
    ];
    let mut light_rs = infer_root_signature(&light_resources)
        .expect("lighting pass root signature 推导(RFC-0005)");
    light_rs.flags = 0x1; // 带 IA 输入布局(lighting VS 亦消费顶点缓冲)
    let light_rts0_bytes = serialize_rts0(&light_rs);

    let pso = AssembledPso {
        root_signature: rs,
        rts0_bytes,
        rtv_formats: vec![Format::Rgba8Unorm],
        dsv_format: None,
    };
    let plan = DeferredPlan {
        gbuffer_color: vec![GBufferTarget::Albedo, GBufferTarget::Normal],
        has_depth: false,
        lighting_srv: vec![GBufferTarget::Albedo], // RFC-0007:lighting 真采样 albedo SRV。
    };
    let barriers: Vec<BarrierAnchor> = vec![
        BarrierAnchor {
            at: "after-geometry",
            transition: BarrierTransition {
                resource: "gbuf:Albedo".to_owned(),
                from: ResourceState::RenderTarget,
                to: ResourceState::PixelShaderResource,
            },
        },
        BarrierAnchor {
            at: "after-lighting",
            transition: BarrierTransition {
                resource: "final".to_owned(),
                from: ResourceState::RenderTarget,
                to: ResourceState::CopySource,
            },
        },
    ];
    let readback = ReadbackLayout {
        row_pitch: 256,
        buffer_size: 256 * 64,
        format: Format::Rgba8Unorm,
    };
    let req = OffscreenRequest {
        pso: &pso,
        light_rts0: &light_rts0_bytes,
        plan: &plan,
        barriers: &barriers,
        readback: &readback,
        width: 64,
        height: 64,
        geom_vs_dxil: &geom_vs,
        geom_fs_dxil: &geom_fs,
        light_vs_dxil: &light_vs,
        light_fs_dxil: &light_fs,
    };
    match execute_offscreen(&req) {
        Ok(OffscreenResult {
            adapter,
            gbuffer_albedo,
            final_pixel,
        }) => {
            // G-G2-4 device 见证行(对齐 G-G2-2/G-G2-3 DXIL_DEVICE/DXIL_BIND 范式)。
            println!(
                "DXIL_UC04: ok adapter=\"{adapter}\" gbuffer={},{},{},{} final={},{},{},{} draw=ok",
                gbuffer_albedo[0],
                gbuffer_albedo[1],
                gbuffer_albedo[2],
                gbuffer_albedo[3],
                final_pixel[0],
                final_pixel[1],
                final_pixel[2],
                final_pixel[3],
            );
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("DXIL_UC04: fail {e}");
            std::process::exit(1);
        }
    }
}

/// device present(`d3d12-runtime`,真出图 gate `real-shim`;G3.2 RXS-0220~0222):host 先核验
/// present 会话([`assemble_present`],RXS-0220)+ 每 pass RFC-0005 RTS0,再经 execute_present
/// 走可见窗口 flip-model swapchain present + resize 重建 + 三点 backbuffer 回读。缺 `real-shim`
/// → 显式 `DXIL_UC04_PRESENT: skip ShimUnavailable`(不伪造 present device 绿,退 0 由 CI 三态
/// 纪律裁定;本入口打印后退 0)。
///
/// usage: `uc04-demo present <geom_vs.dxil> <geom_fs.dxil> <light_vs.dxil> <light_fs.dxil>
///         [frames] [sync_interval] [resize_frame resize_w resize_h]`
#[cfg(feature = "d3d12-runtime")]
fn present_run() {
    use uc04_demo::Format;
    use uc04_demo::device::{PresentDeviceRequest, PresentResult, execute_present};
    use uc04_demo::present::{PresentRequest, PresentState, SwapEffect, assemble_present};
    use uc04_demo::pso::AssembledPso;

    use rurixc::binding_layout::{infer_root_signature, serialize_rts0};

    let args: Vec<String> = std::env::args().collect();
    // args[0]=exe, args[1]="present", args[2..6]=4 DXIL, args[6..]=可选 frames/sync/resize。
    if args.len() < 6 {
        eprintln!(
            "[present] usage: uc04-demo present <geom_vs.dxil> <geom_fs.dxil> <light_vs.dxil> \
             <light_fs.dxil> [frames] [sync_interval] [resize_frame resize_w resize_h]"
        );
        std::process::exit(2);
    }
    let read = |p: &str| -> Vec<u8> {
        std::fs::read(p).unwrap_or_else(|e| {
            eprintln!("[present] 读 DXIL {p} 失败: {e}");
            std::process::exit(2);
        })
    };
    let geom_vs = read(&args[2]);
    let geom_fs = read(&args[3]);
    let light_vs = read(&args[4]);
    let light_fs = read(&args[5]);
    let parse_at =
        |i: usize, dflt: u32| -> u32 { args.get(i).and_then(|s| s.parse().ok()).unwrap_or(dflt) };
    let frames = parse_at(6, 8);
    let sync_interval = parse_at(7, 1);
    let resize_frame = parse_at(8, 0);
    let resize_width = parse_at(9, 0);
    let resize_height = parse_at(10, 0);

    // 几何 pass RFC-0005 RTS0(P-11):空资源 + IA flag(承 offscreen device_run 口径)。
    let mut rs =
        infer_root_signature(&[]).expect("几何 pass 空资源集 root signature 推导(RFC-0005)");
    rs.flags = 0x1;
    let rts0_bytes = serialize_rts0(&rs);

    // lighting pass RFC-0005 RTS0:SRV t0 + Sampler s0(RFC-0007 真采样)。
    let light_resources = vec![
        rb("g_albedo", MirResourceType::Texture2D(PrimTy::F32)),
        rb("g_samp", MirResourceType::Sampler),
    ];
    let mut light_rs = infer_root_signature(&light_resources)
        .expect("lighting pass root signature 推导(RFC-0005)");
    light_rs.flags = 0x1;
    let light_rts0_bytes = serialize_rts0(&light_rs);

    let pso = AssembledPso {
        root_signature: rs,
        rts0_bytes,
        rtv_formats: vec![Format::Rgba8Unorm],
        dsv_format: None,
    };

    // RXS-0220:host 先核验 present 会话(swapchain desc ↔ backbuffer 格式一致 + 迁移锚点)。
    let session = assemble_present(&PresentRequest {
        swapchain_format: Format::Rgba8Unorm,
        final_rt_format: Format::Rgba8Unorm,
        buffer_count: 3,
        swap_effect: SwapEffect::FlipDiscard,
        width: 256,
        height: 256,
        sync_interval,
        tearing_requested: false,
        frames,
        present_transitions: vec![
            (PresentState::RenderTarget, PresentState::CopySource),
            (PresentState::CopySource, PresentState::Present),
        ],
    })
    .unwrap_or_else(|e| {
        eprintln!("DXIL_UC04_PRESENT: fail present 装配核验拒 {e}");
        std::process::exit(1);
    });

    let req = PresentDeviceRequest {
        session: &session,
        pso: &pso,
        light_rts0: &light_rts0_bytes,
        resize_frame,
        resize_width,
        resize_height,
        geom_vs_dxil: &geom_vs,
        geom_fs_dxil: &geom_fs,
        light_vs_dxil: &light_vs,
        light_fs_dxil: &light_fs,
    };
    match execute_present(&req) {
        Ok(PresentResult {
            adapter,
            first_pixel,
            rebuilt_pixel,
            last_pixel,
            frames_presented,
        }) => {
            // present device 见证行(对齐 DXIL_UC04 offscreen 范式;三点 backbuffer 像素)。
            println!(
                "DXIL_UC04_PRESENT: ok adapter=\"{adapter}\" frames_presented={frames_presented} \
                 first={},{},{},{} rebuilt={},{},{},{} last={},{},{},{} present=ok",
                first_pixel[0],
                first_pixel[1],
                first_pixel[2],
                first_pixel[3],
                rebuilt_pixel[0],
                rebuilt_pixel[1],
                rebuilt_pixel[2],
                rebuilt_pixel[3],
                last_pixel[0],
                last_pixel[1],
                last_pixel[2],
                last_pixel[3],
            );
            std::process::exit(0);
        }
        Err(uc04_demo::Uc04Error::ShimUnavailable { detail }) => {
            // dev-env degrade:缺 real-shim / 无显示环境 → SKIP sentinel,不伪造 present 绿。
            println!("DXIL_UC04_PRESENT: skip ShimUnavailable {detail}");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("DXIL_UC04_PRESENT: fail {e}");
            std::process::exit(1);
        }
    }
}
