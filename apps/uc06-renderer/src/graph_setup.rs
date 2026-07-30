//! RenderGraph 帧调度骨架声明(G5 uc06;报告5 P0–P2;RFC-0016 章 A;门 G-G5-3
//! demo 核验点)——全帧管线以声明式 pass 描述,**帧内零手写屏障**:barrier/fence/
//! transient 别名全部由 `graph_compile` 四趟产出;AO/GI 滤波 pass 标
//! `QueueClass::AsyncCompute`(报告5 §2.4 首批候选);历史颜色/深度、VSM 页表/物理
//! 池、GI 探针历史经 `import`(跨帧外部资源纪律 §4.0-3)。
//!
//! 本模块只建图;pass 的实际工作(host 波为 CPU 参考执行)在 `pipeline.rs` 逐阶段
//! 驱动,**声明序 = 依赖语义序**,执行序与编译产物的 barrier/fence 对拍锚定。

use rurix_render::graph::types::{
    AccessKind, PassDesc, QueueClass, ResAccess, ResourceDesc, ResourceId, ResourceKind,
    TextureFormat,
};
use rurix_render::graph::{CompileOptions, CompiledGraph, RenderGraph};

/// 帧内资源句柄集(建图与执行对拍用;字段即管线阶段名)。
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct FrameResources {
    pub visbuffer: ResourceId,
    pub depth: ResourceId,
    pub material_buf: ResourceId,
    pub shadow_raw: ResourceId,
    pub gi_irradiance: ResourceId,
    pub ao_raw: ResourceId,
    pub ao_filtered: ResourceId,
    pub shadow_filtered: ResourceId,
    pub hdr_color: ResourceId,
    pub taa_color: ResourceId,
    pub out_color: ResourceId,
    pub taa_history: ResourceId,
    pub depth_history: ResourceId,
    pub gi_history: ResourceId,
    pub vsm_page_table: ResourceId,
    pub vsm_phys_pool: ResourceId,
    pub mv: ResourceId,
}

fn tex(name: &str, w: u32, h: u32, format: TextureFormat) -> ResourceDesc {
    ResourceDesc {
        name: name.to_owned(),
        kind: ResourceKind::Texture2d {
            width: w,
            height: h,
            format,
            mip_levels: 1,
        },
        imported: false,
    }
}

fn buf(name: &str, size: u64) -> ResourceDesc {
    ResourceDesc {
        name: name.to_owned(),
        kind: ResourceKind::Buffer { size },
        imported: false,
    }
}

/// 建图(参数 = 内部分辨率/输出分辨率/流送页池字节估算),编译并返回产物。
///
/// pass 线性序(依赖语义序):
/// 0 instance_cull → 1 cluster_cull → 2 visbuffer_raster → 3 mat_classify_resolve
/// → 4 vsm_page_mark → 5 vsm_page_alloc_raster → 6 gi_probe_trace(**AsyncCompute**)
/// → 7 rtao(**AsyncCompute**) → 8 hard_shadow(**AsyncCompute**) → 9 ao_filter
/// → 10 shadow_project → 11 deferred_shade → 12 taa → 13 tsr_upscale → 14 readback
///
/// AO/GI/硬阴影三 pass 只读 GBuffer 不读图形中间态、写自有缓冲、消费者(9/11)距离
/// 生产者(2)足够远——候选三条件声明式满足(报告5 §2.4);机制正确性(FencePair
/// 注入)由编译产物锚定,时间戳重叠量由 evidence 度量段记录(P-09 不进硬门)。
pub fn build_frame_graph(
    internal_w: u32,
    internal_h: u32,
    out_w: u32,
    out_h: u32,
    pool_bytes_est: u64,
) -> (CompiledGraph, FrameResources) {
    let mut g = RenderGraph::new();

    // transient 帧内资源。
    let cull_args = g.create(buf("cull_args", 4096));
    let draw_args = g.create(buf("draw_args", 4096));
    let visbuffer = g.create(tex(
        "visbuffer",
        internal_w,
        internal_h,
        TextureFormat::Rg32Uint,
    ));
    let depth = g.create(tex(
        "depth",
        internal_w,
        internal_h,
        TextureFormat::R32Float,
    ));
    let material_buf = g.create(tex(
        "material",
        internal_w,
        internal_h,
        TextureFormat::R32Uint,
    ));
    let shadow_raw = g.create(tex(
        "shadow_raw",
        internal_w,
        internal_h,
        TextureFormat::R32Float,
    ));
    let gi_irradiance = g.create(tex(
        "gi_irradiance",
        internal_w,
        internal_h,
        TextureFormat::Rgba16Float,
    ));
    let ao_raw = g.create(tex(
        "ao_raw",
        internal_w,
        internal_h,
        TextureFormat::R32Float,
    ));
    let ao_filtered = g.create(tex(
        "ao_filtered",
        internal_w,
        internal_h,
        TextureFormat::R32Float,
    ));
    let shadow_filtered = g.create(tex(
        "shadow_filtered",
        internal_w,
        internal_h,
        TextureFormat::R32Float,
    ));
    let hdr_color = g.create(tex(
        "hdr_color",
        internal_w,
        internal_h,
        TextureFormat::Rgba16Float,
    ));
    let taa_color = g.create(tex(
        "taa_color",
        internal_w,
        internal_h,
        TextureFormat::Rgba16Float,
    ));
    let out_color = g.create(tex("out_color", out_w, out_h, TextureFormat::Rgba16Float));
    let mv = g.create(tex("mv", internal_w, internal_h, TextureFormat::Rg16Float));

    // imported 跨帧外部资源(历史/页表/物理池;§4.0-3 纪律)。
    let taa_history = g.import(tex(
        "taa_history",
        internal_w,
        internal_h,
        TextureFormat::Rgba16Float,
    ));
    let depth_history = g.import(tex(
        "depth_history",
        internal_w,
        internal_h,
        TextureFormat::R32Float,
    ));
    let gi_history = g.import(tex(
        "gi_probe_history",
        internal_w / 4,
        internal_h / 4,
        TextureFormat::Rgba16Float,
    ));
    let vsm_page_table = g.import(buf("vsm_page_table", 128 * 128 * 4 * 6));
    let vsm_phys_pool = g.import(buf("vsm_phys_pool", pool_bytes_est));

    let fr = FrameResources {
        visbuffer,
        depth,
        material_buf,
        shadow_raw,
        gi_irradiance,
        ao_raw,
        ao_filtered,
        shadow_filtered,
        hdr_color,
        taa_color,
        out_color,
        taa_history,
        depth_history,
        gi_history,
        vsm_page_table,
        vsm_phys_pool,
        mv,
    };

    let r = |res, access| ResAccess { res, access };
    let gfx = |name: &str| PassDesc {
        name: name.to_owned(),
        queue: QueueClass::Graphics,
        reads: vec![],
        writes: vec![],
    };
    let async_c = |name: &str| PassDesc {
        name: name.to_owned(),
        queue: QueueClass::AsyncCompute,
        reads: vec![],
        writes: vec![],
    };

    // 0 实例剔除(读 GpuScene 概念面,写剔除 args buffer)。
    let mut d = gfx("instance_cull");
    d.writes = vec![r(cull_args, AccessKind::ShaderWrite)];
    g.add_pass(d);

    // 1 簇剔除(读 args,写紧凑 draw args buffer)。
    let mut d = gfx("cluster_cull");
    d.reads = vec![r(cull_args, AccessKind::ShaderRead)];
    d.writes = vec![r(draw_args, AccessKind::ShaderWrite)];
    g.add_pass(d);

    // 2 VisBuffer 光栅(读 draw args,写 visbuffer + depth + mv MRT)。
    let mut d = gfx("visbuffer_raster");
    d.reads = vec![r(draw_args, AccessKind::IndirectArgs)];
    d.writes = vec![
        r(visbuffer, AccessKind::ShaderWrite),
        r(depth, AccessKind::DepthTarget),
        r(mv, AccessKind::ShaderWrite),
    ];
    g.add_pass(d);

    // 3 材质 classify/resolve(读 visbuffer,写材质窄缓冲)。
    let mut d = gfx("mat_classify_resolve");
    d.reads = vec![r(visbuffer, AccessKind::ShaderRead)];
    d.writes = vec![r(material_buf, AccessKind::ShaderWrite)];
    g.add_pass(d);

    // 4 VSM 页标记(读深度,写页表 import)。
    let mut d = gfx("vsm_page_mark");
    d.reads = vec![r(depth, AccessKind::DepthRead)];
    d.writes = vec![r(vsm_page_table, AccessKind::ShaderWrite)];
    g.add_pass(d);

    // 5 VSM 分配+深度光栅(读页表,写物理池 import;页表回写并入写侧单条 ShaderWrite)。
    let mut d = gfx("vsm_page_alloc_raster");
    d.reads = vec![r(vsm_page_table, AccessKind::ShaderRead)];
    d.writes = vec![r(vsm_phys_pool, AccessKind::ShaderWrite)];
    g.add_pass(d);

    // 6 GI 探针追踪(异步;读深度/gi_history,写 gi_irradiance;gi_history 回写并入写侧单条)。
    let mut d = async_c("gi_probe_trace");
    d.reads = vec![
        r(depth, AccessKind::ShaderRead),
        r(gi_history, AccessKind::ShaderRead),
    ];
    d.writes = vec![r(gi_irradiance, AccessKind::ShaderWrite)];
    g.add_pass(d);

    // 7 RTAO(异步;读深度/法线,写 ao_raw)。
    let mut d = async_c("rtao");
    d.reads = vec![r(depth, AccessKind::ShaderRead)];
    d.writes = vec![r(ao_raw, AccessKind::ShaderWrite)];
    g.add_pass(d);

    // 8 硬阴影(异步;读深度,写 shadow_raw)。
    let mut d = async_c("hard_shadow");
    d.reads = vec![r(depth, AccessKind::ShaderRead)];
    d.writes = vec![r(shadow_raw, AccessKind::ShaderWrite)];
    g.add_pass(d);

    // 9 AO 时域滤波(图形车道 join 点;读 ao_raw + 历史 + gi_irradiance(异步产出
    // 的图形消费者锚点——fence 弧闭环:w=max 生产者→min 消费者),写 ao_filtered)。
    let mut d = gfx("ao_filter");
    d.reads = vec![
        r(ao_raw, AccessKind::ShaderRead),
        r(depth, AccessKind::DepthRead),
        r(depth_history, AccessKind::ShaderRead),
        r(mv, AccessKind::ShaderRead),
        r(gi_irradiance, AccessKind::ShaderRead),
    ];
    d.writes = vec![r(ao_filtered, AccessKind::ShaderWrite)];
    g.add_pass(d);

    // 10 VSM 投影采样(读深度/页表/物理池,写 shadow_filtered)。
    let mut d = gfx("shadow_project");
    d.reads = vec![
        r(depth, AccessKind::DepthRead),
        r(vsm_page_table, AccessKind::ShaderRead),
        r(vsm_phys_pool, AccessKind::ShaderRead),
        r(shadow_raw, AccessKind::ShaderRead),
    ];
    d.writes = vec![r(shadow_filtered, AccessKind::ShaderWrite)];
    g.add_pass(d);

    // 11 延迟着色(读 gi/ao/shadow/材质/深度,写 hdr_color)。
    let mut d = gfx("deferred_shade");
    d.reads = vec![
        r(gi_irradiance, AccessKind::ShaderRead),
        r(ao_filtered, AccessKind::ShaderRead),
        r(shadow_filtered, AccessKind::ShaderRead),
        r(material_buf, AccessKind::ShaderRead),
        r(depth, AccessKind::ShaderRead),
        r(visbuffer, AccessKind::ShaderRead),
    ];
    d.writes = vec![r(hdr_color, AccessKind::ShaderWrite)];
    g.add_pass(d);

    // 12 TAA(读 hdr + 历史 + mv,写 taa_color;taa_history 回写并入写侧单条)。
    let mut d = gfx("taa");
    d.reads = vec![
        r(hdr_color, AccessKind::ShaderRead),
        r(taa_history, AccessKind::ShaderRead),
        r(mv, AccessKind::ShaderRead),
    ];
    d.writes = vec![r(taa_color, AccessKind::ShaderWrite)];
    g.add_pass(d);

    // 13 TSR 超分(读 taa + 深度 + mv,写 out_color)。
    let mut d = gfx("tsr_upscale");
    d.reads = vec![
        r(taa_color, AccessKind::ShaderRead),
        r(depth, AccessKind::ShaderRead),
        r(mv, AccessKind::ShaderRead),
    ];
    d.writes = vec![r(out_color, AccessKind::ShaderWrite)];
    g.add_pass(d);

    // 14 readback 终端(Present 终端消费 = 反向可达剔除的根;host 波 readback 为
    // 直接内存读,Present 语义锚定「帧输出被消费」)。
    let mut d = gfx("readback");
    d.reads = vec![r(out_color, AccessKind::Present)];
    g.add_pass(d);

    let compiled = g
        .compile(CompileOptions::default())
        .expect("uc06 帧图必须编译通过(声明完整性由图编译器兜底)");

    // 结构性锚定(门 G-G5-3 demo 核验点,失败 = 声明面回归,panic 早于静默):
    // ① fence 非空(异步车道真注入);② 别名峰值 < 无别名峰值(transient 池真省)。
    assert!(
        !compiled.fences().is_empty(),
        "异步车道 FencePair 必须非空(AO/GI/硬阴影候选)"
    );
    let pool = compiled.pool();
    assert!(
        pool.high_water() < pool.no_alias_peak(),
        "transient 别名后峰值必须低于无别名峰值"
    );

    (compiled, fr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_graph_compiles_with_async_fences_and_alias_savings() {
        let (compiled, _fr) = build_frame_graph(128, 72, 256, 144, 16 * 128 * 128 * 4);
        assert_eq!(
            compiled.passes().len(),
            15,
            "15 个 pass 全保留(终端 Present 防剔除)"
        );
        let fences = compiled.fences();
        assert!(!fences.is_empty(), "FencePair 非空");
        // 每个 fence 的 wait_before 晚于 signal_after(timeline 因果序)。
        for f in fences {
            assert!(
                f.wait_before > f.signal_after,
                "fence 因果序:wait 必晚于 signal"
            );
            assert!(f.value >= 1);
        }
        let pool = compiled.pool();
        assert!(pool.high_water() < pool.no_alias_peak());
        // 图 dump 可产且非平凡。
        let dump = compiled.dump_json();
        assert!(dump.len() > 1024);
        assert!(dump.contains("visbuffer") && dump.contains("deferred_shade"));
    }
}
