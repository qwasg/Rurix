// Assisted-by: Kimi-K3（G34 全特性合流 G34-2 HZB 接统一车道）
// G34-2 HZB 接统一车道——独立 include 段（`include!` 进 g34_full_lane.rs;
// 与 G34-3 蒙皮同窗并行面分区:本文件承载 G34-2 全部 HZB 面,bin 本体仅
// 加性挂点)。门 `g34.wave2.hzb`。
//
// ## 架构(G31+ 波 B Task B1 生产接线面逐字同律 + G34 统一车道合流)
//
// - **剔除对象粒度 = TLAS 实例**(bistro 逐 mesh 节点 BLAS 分解 + 动态实例
//   尾槽;tris/mats SSBO 与 G34Full 双实例面位级同 buffer——节点段为装配序
//   连续段,g34_unified_primary 经 inst_base 前缀和表把 (inst, prim) 映回
//   全局下标〔动态槽 inst_base = dyn_tri_base ⇒ 与 fork B 字面同值〕,着色
//   数学与统一 mega 面同 op 序)。
// - **消费点 = 主射线 pass 的 TLAS 实例 mask**(被剔实例 mask=0x00 ⇒ ray
//   query 零遍历其 BLAS);kernels/g34_unified_primary.rx 相机射线走初剔后
//   TLAS(表 0),kernels/g34_unified_shade.rx 阴影射线走全量 TLAS(表 1——
//   被剔实例仍投阴影,遮挡物阴影正确性面;RXS-0297 单 TLAS 签名纪律 ⇒ 拆
//   pass)。动态实例 = A4 核验对象,**恒可见不参剔**(如实登记——剔除计数
//   面 = 静态节点;12 三角形遍历代价可不计)。
// - **双 TLAS 逐帧 refit**(动态实例场景更新策略):表 0 = 逐帧实例掩码 +
//   动态变换 refit(tlas_update),表 1 = 全 0xFF 掩码 + 动态变换 refit
//   (render_exec G34-2 加性 `execute_with_frame_update_dual_tlas` 第二更新
//   位——双表各自 refit 同帧单提交);dyn off = 表 0 掩码等价重更跳过(G31
//   同律)、表 1 零更新(内容静态)。
// - **帧内金字塔轮换**(g27_hzb_reduce/g27_hzb_test 0-byte 冻结消费 +
//   g31_hzb_pack glue 0-byte):本帧**真深度** = g34_unified_shade ④b 段
//   out_depth_hz(vp 行 2/3 另算真 ZO NDC;U_SCENE_DEPTH 沿用生产字面供
//   MV/TSR,两路并存互不染指)逐级归约 + 平铺打包;pass 序 = primary →
//   shade → mv → tsr×2 → encode → test_p1(全实例 rect vs 上帧平铺 =
//   「上帧金字塔初剔」)→ reduce×(L−1) + pack×L(本帧重建覆写) →
//   test_p2(上帧被剔集 vs 本帧金字塔 = 「本帧重建重测」)。
// - **两阶段闭环第二段**(RFC-0044 §5.8):collect 结算应见集 = p1 可见 ∪
//   p2 翻回——应见而有未渲者 ⇒ 掩码并集同帧重渲,迭代 ≤4 未收敛 ⇒ 全掩码
//   兜底(= 零剔除精确收敛);剔除零假阳性 ⇒ 闭环后画面与全集渲染位级一致,
//   由 RURIX_HZB_ALL_VISIBLE=1 登记实验臂 digest_seq 逐帧对拍承载。
// - **host 金标准面对拍**(geometry/{hzb,cull}.rs 只读消费 0-byte):
//   cull::Frustum 视锥离屏第一关 + probe 帧 HzbPyramid::build/test_rect/
//   exact_rect_occluded 复算(mips 逐级位级 + 判定序列逐字节 + 零假阳性
//   独立复核,harness fail-fast 硬门)。

// 冻结金标准面只读消费(geometry/{cull,hzb}.rs 0-byte;include! 文本展开位于
// bin 模块顶层 ⇒ use 声明模块级生效,主 bin 零触碰)。
use rurix_render::geometry::cull::Frustum;
use rurix_render::geometry::hzb::{DepthConvention, HzbPyramid, Occlusion, exact_rect_occluded};

/// G34-2 门键(evidence `gate` 字段字面)。
const G34HZB_GATE: &str = "g34.wave2.hzb";
/// G34-2 harness evidence schema 字面(.tmp 工作区件;G31 HZB 同律——harness
/// 真跑件不注册 check_schemas,数字经门裁决件蒸馏登记)。
const G34HZB_SCHEMA: &str = "rurix.g34.hzb_unified_evidence.v1";
/// 主射线 kernel 默认 SPV(源 = kernels/g34_unified_primary.rx——G34-2 加性件,
/// CI 门脚本保障编译)。
const G34HZB_DEFAULT_SPV_PRIMARY: &str = ".tmp/g34_gates/hzb/g34_unified_primary.spv";
/// 着色 kernel 默认 SPV(源 = kernels/g34_unified_shade.rx——G34-1 骨架经
/// G34-2 生产接线扩展〔fork A 采样块 + inline 阴影 + out_depth_hz〕)。
const G34HZB_DEFAULT_SPV_SHADE: &str = ".tmp/g34_gates/hzb/g34_unified_shade.spv";
/// 平铺打包 glue kernel 默认 SPV(g31_hzb_pack 0-byte 冻结消费)。
const G34HZB_DEFAULT_SPV_PACK: &str = ".tmp/g14_gates/m_c/g31_hzb_pack.spv";
/// 金字塔归约 kernel 默认 SPV(g27_hzb_reduce G27 M-a 本体 0-byte 冻结消费)。
const G34HZB_DEFAULT_SPV_REDUCE: &str = ".tmp/g14_gates/m_c/g27_hzb_reduce.spv";
/// 遮挡测试 kernel 默认 SPV(g27_hzb_test G27 M-a 本体 0-byte 冻结消费)。
const G34HZB_DEFAULT_SPV_TEST: &str = ".tmp/g14_gates/m_c/g27_hzb_test.spv";
/// 闭环重渲迭代上限(未收敛 ⇒ 全掩码兜底重渲 = 精确收敛;如实登记)。
const G34HZB_CLOSURE_MAX: u32 = 4;
/// 深度约定(车道深度 = ZO NDC 小值近/miss=1.0 远 ⇒ standard-Z;g27 kernel
/// 约定位 conv=1.0,host `DepthConvention::StandardZ` 同律)。
const G34HZB_CONV_FLAG: f32 = 1.0;

// ---------------------------------------------------------------------------
// 逐实例初剔分类(host;视锥面 + rect 流——G31 B1 逐字同模 + fork B 动态实例
// 尾槽恒可见面)。
// ---------------------------------------------------------------------------

/// G34-2 逐实例初剔分类(G31HzbClass 逐字同模)。
enum G34HzbClass {
    /// 视锥外(离屏/相机后)——像素中性直接剔,不进 test 流。
    Offscreen,
    /// 在屏:rect(uv 闭区间,±半像素 jitter 保守裕量外扩)+ 最近深度。
    Rect {
        uv_min: [f32; 2],
        uv_max: [f32; 2],
        nearest: f32,
    },
}

/// G34-2 逐实例初剔分类器(静态逐节点 AABB + 动态实例尾槽恒 visible 追加;
/// host 确定性 f32)。视锥/骑跨/退化三面语义 = g31_hzb_classify 逐字同模
/// (注释见 G31 B1 段——相机面骑跨超保守恒可见、nearest 只钳上界 1.0 保负值
/// ⇒ 严格不等式自遮挡结构上不可达)。**动态实例恒可见**(A4 核验对象面:
/// 纯发光体验证谱,剔除会破坏逐帧位置核验硬门;如实登记不参剔)。
/// RURIX_HZB_ALL_VISIBLE=1 = 登记实验臂(全实例恒可见 ⇒ 掩码恒全 0xFF;
/// 中性门消费面)。
fn g34_hzb_classify(
    vp: &Mat4,
    iw: u32,
    ih: u32,
    groups: &[SceneNodeGroup],
    dyn_on: bool,
) -> Vec<G34HzbClass> {
    let n = groups.len() + usize::from(dyn_on);
    // 登记实验臂(ci/g34_hzb_unified_smoke.py 剔除像素中性门消费):全实例恒
    // 可见(无视锥/无剔除 ⇒ 掩码恒全 0xFF)——同一分解车道渲染全集;--hzb on
    // 常态臂 vs 本臂 digest_seq 逐帧位级一致 ⇒ 「剔除不改变可见像素」机核门
    // 成立(可见集一致性结构判据)。
    if std::env::var("RURIX_HZB_ALL_VISIBLE").ok().as_deref() == Some("1") {
        return (0..n)
            .map(|_| G34HzbClass::Rect {
                uv_min: [0.0, 0.0],
                uv_max: [1.0, 1.0],
                nearest: f32::NEG_INFINITY,
            })
            .collect();
    }
    let frustum = Frustum::from_view_proj(&vp.m);
    let (w0, h0) = (iw as f32, ih as f32);
    let (du, dv) = (0.5 / w0, 0.5 / h0);
    let mut out = Vec::with_capacity(n);
    for g in groups {
        // 相机面骑跨预审(G31 B1 逐字):w ≤ 0 角点存在 ⇒ 视锥判定失真——全部
        // w ≤ 0 ⇒ 相机后像素中性剔;部分 ⇒ 骑跨超保守恒可见;全 w > 0 ⇒ 视锥
        // 面判定可信。
        let mut cs = [[0.0f32; 4]; 8];
        let (mut any_back, mut any_front) = (false, false);
        let mut k = 0usize;
        for &x in &[g.aabb_min[0], g.aabb_max[0]] {
            for &y in &[g.aabb_min[1], g.aabb_max[1]] {
                for &z in &[g.aabb_min[2], g.aabb_max[2]] {
                    let c = vp.transform_vec4([x, y, z, 1.0]);
                    if c[3] <= 1e-6 {
                        any_back = true;
                    } else {
                        any_front = true;
                    }
                    cs[k] = c;
                    k += 1;
                }
            }
        }
        if any_back {
            if any_front {
                out.push(G34HzbClass::Rect {
                    uv_min: [0.0, 0.0],
                    uv_max: [1.0, 1.0],
                    nearest: f32::NEG_INFINITY,
                });
            } else {
                out.push(G34HzbClass::Offscreen);
            }
            continue;
        }
        if !frustum.intersects_aabb(g.aabb_min, g.aabb_max) {
            out.push(G34HzbClass::Offscreen);
            continue;
        }
        let (mut u_min, mut v_min, mut u_max, mut v_max, mut nearest) =
            (f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY, f32::INFINITY);
        for c in &cs {
            let inv_w = 1.0 / c[3];
            let u = (c[0] * inv_w + 1.0) * 0.5;
            let v = (1.0 - c[1] * inv_w) * 0.5;
            let zz = (c[2] * inv_w).min(1.0);
            u_min = u_min.min(u);
            u_max = u_max.max(u);
            v_min = v_min.min(v);
            v_max = v_max.max(v);
            nearest = nearest.min(zz);
        }
        let umin = (u_min - du).clamp(0.0, 1.0);
        let umax = (u_max + du).clamp(0.0, 1.0);
        let vmin = (v_min - dv).clamp(0.0, 1.0);
        let vmax = (v_max + dv).clamp(0.0, 1.0);
        if umax <= umin || vmax <= vmin {
            out.push(G34HzbClass::Rect {
                uv_min: [0.0, 0.0],
                uv_max: [1.0, 1.0],
                nearest: f32::NEG_INFINITY,
            });
            continue;
        }
        out.push(G34HzbClass::Rect {
            uv_min: [umin, vmin],
            uv_max: [umax, vmax],
            nearest,
        });
    }
    // fork B 动态实例尾槽:恒可见(A4 核验对象不参剔——如实登记;nearest=−∞
    // ⇒ standard-Z 严格不等式恒 Visible,rect 流占位保判定序对齐)。
    if dyn_on {
        out.push(G34HzbClass::Rect {
            uv_min: [0.0, 0.0],
            uv_max: [1.0, 1.0],
            nearest: f32::NEG_INFINITY,
        });
    }
    out
}

// ---------------------------------------------------------------------------
// SPV/常量字节所有者(desc 数组借用源;借用纪律 = bits → descs → session 声明
// 序 drop 逆序)。
// ---------------------------------------------------------------------------

struct G34HzbBits {
    spv_primary: Vec<u8>,
    spv_shade: Vec<u8>,
    spv_reduce: Vec<u8>,
    spv_test: Vec<u8>,
    spv_pack: Vec<u8>,
    /// pass 名(telemetry 逐 pass 唯一键;kernel 身份由 SPV provenance 登记——
    /// reduce/test = g27 本体 0-byte,pack = g31 glue 0-byte,primary/shade =
    /// G34 统一件)。
    name_primary: String,
    name_shade: String,
    name_test_p1: String,
    name_test_p2: String,
    reduce_names: Vec<String>,
    pack_names: Vec<String>,
    primary_dispatch: [u32; 3],
    shade_dispatch: [u32; 3],
    test_dispatch: [u32; 3],
    reduce_dispatch: Vec<[u32; 3]>,
    pack_dispatch: Vec<[u32; 3]>,
    /// mip 逐級 (w,h)(mip0 = 内部分辨率;直至 1×1)。
    levels: Vec<(u32, u32)>,
    /// 平铺金字塔逐級纹素偏移(前缀和;g27_hzb_test mip_table offset 段同源)。
    flat_offsets: Vec<u32>,
    flat_texels: usize,
    mip_table_bytes: Vec<u8>,
    reduce_params_bytes: Vec<Vec<u8>>,
    pack_params_bytes: Vec<Vec<u8>>,
    /// 平铺金字塔初值 = 全 1.0f32(standard-Z 最远 ⇒ 首帧前全 Visible 保守
    /// 初值,空金字塔假阳性构造性不可达)。
    flat_init_bytes: Vec<u8>,
    /// 逐实例全局三角形下标基底(静态节点段前缀和 + 动态槽 dyn_tri_base;
    /// g34_unified_primary inst_base 面)。
    inst_base_bytes: Vec<u8>,
}

impl G34HzbBits {
    fn load(
        spv_primary: &str,
        spv_shade: &str,
        spv_reduce: &str,
        spv_test: &str,
        spv_pack: &str,
        iw: u32,
        ih: u32,
        groups: &[SceneNodeGroup],
        dyn_tri_base: usize,
        dyn_on: bool,
        inject_primary_shade: bool,
    ) -> Self {
        let to_bytes = |words: &[u32]| -> Vec<u8> {
            words.iter().flat_map(|w| w.to_le_bytes()).collect()
        };
        // primary/shade SPV 处置(textures on = NoContraction 注入——B4 同律:
        // fork A bilinear/LUT 采样链与 host 参考逐 op 位级对拍前提;off = 原始
        // SPV 零注入,母版处置同字面);HZB 两 kernel(g27 本体 0-byte)恒注入
        // ——G27 零容差协议 conv 乘法门在负深度域保门形逐 op IEEE 位级(G31
        // B1 同律);pack glue 纯拷贝零注入面。
        let pw = if inject_primary_shade {
            spv_inject_no_contraction(&load_spv(spv_primary))
        } else {
            load_spv(spv_primary)
        };
        let sw = if inject_primary_shade {
            spv_inject_no_contraction(&load_spv(spv_shade))
        } else {
            load_spv(spv_shade)
        };
        let rw = spv_inject_no_contraction(&load_spv(spv_reduce));
        let tw = spv_inject_no_contraction(&load_spv(spv_test));
        let kw = load_spv(spv_pack);
        let (px, py, _) = spv_local_size(&pw);
        let (sx, sy, _) = spv_local_size(&sw);
        // mip 拓扑 = host `HzbPyramid::build` 逐字(非 2 幂 ceil 减半 max 1,
        // 直至 1×1)。
        let mut levels: Vec<(u32, u32)> = vec![(iw, ih)];
        while levels.last().unwrap().0 > 1 || levels.last().unwrap().1 > 1 {
            let (w, h) = *levels.last().unwrap();
            levels.push((w.div_ceil(2).max(1), h.div_ceil(2).max(1)));
        }
        let mut flat_offsets = Vec::with_capacity(levels.len());
        let mut acc = 0u32;
        for &(w, h) in &levels {
            flat_offsets.push(acc);
            acc += w * h;
        }
        let flat_texels = acc as usize;
        // mip 表(3 f32/級 [offset,w,h];g27_hzb_test 参数面逐字同源)。
        let mut mip_table: Vec<f32> = Vec::with_capacity(levels.len() * 3);
        for (k, &(w, h)) in levels.iter().enumerate() {
            mip_table.push(flat_offsets[k] as f32);
            mip_table.push(w as f32);
            mip_table.push(h as f32);
        }
        // reduce 参数(級 k=1..L−1:g27_hzb_reduce 8 f32 参数面逐字同源;
        // conv = standard-Z 1.0——车道深度 ZO NDC 小值近)。
        let mut reduce_params_bytes = Vec::with_capacity(levels.len() - 1);
        let mut reduce_dispatch = Vec::with_capacity(levels.len() - 1);
        for k in 1..levels.len() {
            let (nw, nh) = levels[k];
            let (pw2, ph2) = levels[k - 1];
            let p = [
                (nw * nh) as f32,
                nw as f32,
                nh as f32,
                pw2 as f32,
                ph2 as f32,
                G34HZB_CONV_FLAG,
                0.0,
                0.0,
            ];
            reduce_params_bytes.push(bytes_f32(&p));
            reduce_dispatch.push([nw * nh, 1, 1]);
        }
        // pack 参数(級 k=0..L−1:[count, dst_offset, 0..])。
        let mut pack_params_bytes = Vec::with_capacity(levels.len());
        let mut pack_dispatch = Vec::with_capacity(levels.len());
        for (k, &(w, h)) in levels.iter().enumerate() {
            let p = [
                (w * h) as f32,
                flat_offsets[k] as f32,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
            ];
            pack_params_bytes.push(bytes_f32(&p));
            pack_dispatch.push([w * h, 1, 1]);
        }
        let flat_init: Vec<f32> = vec![1.0f32; flat_texels];
        // inst_base = 静态逐节点段前缀和(装配序连续段 ⇒ tri_offset 即前缀和)
        // + 动态槽 dyn_tri_base(fork B 字面同值;< 2^24 f32 精确域)。
        let mut inst_base: Vec<f32> = Vec::with_capacity(groups.len() + 1);
        for g in groups {
            inst_base.push(g.tri_offset as f32);
        }
        if dyn_on {
            inst_base.push(dyn_tri_base as f32);
        }
        let n_inst = (groups.len() + usize::from(dyn_on)).max(1) as u32;
        Self {
            spv_primary: to_bytes(&pw),
            spv_shade: to_bytes(&sw),
            spv_reduce: to_bytes(&rw),
            spv_test: to_bytes(&tw),
            spv_pack: to_bytes(&kw),
            name_primary: "g34_unified_primary".to_owned(),
            name_shade: "g34_unified_shade".to_owned(),
            name_test_p1: "g27_hzb_test_p1".to_owned(),
            name_test_p2: "g27_hzb_test_p2".to_owned(),
            reduce_names: (1..levels.len())
                .map(|k| format!("g27_hzb_reduce_l{k}"))
                .collect(),
            pack_names: (0..levels.len())
                .map(|k| format!("g31_hzb_pack_l{k}"))
                .collect(),
            primary_dispatch: [iw.div_ceil(px), ih.div_ceil(py), 1],
            shade_dispatch: [iw.div_ceil(sx), ih.div_ceil(sy), 1],
            test_dispatch: [n_inst, 1, 1],
            reduce_dispatch,
            pack_dispatch,
            levels,
            flat_offsets,
            flat_texels,
            mip_table_bytes: bytes_f32(&mip_table),
            reduce_params_bytes,
            pack_params_bytes,
            flat_init_bytes: bytes_f32(&flat_init),
            inst_base_bytes: bytes_f32(&inst_base),
        }
    }
}

// ---------------------------------------------------------------------------
// G34-2 HZB 车道描述组(G34Full 27 SSBO 0..=26 + encode 27/28 + HZB 追加面
// 29+;pass 终序 = primary → shade → mv → resample → resolve → encode →
// test_p1 → reduce×(L−1) → pack×L → test_p2——帧内金字塔轮换调度字面;mega
// scene pass 本体 0-byte 不进 HZB 车道,primary/shade 双 pass 替换之)。
// ---------------------------------------------------------------------------

/// G34-2 车道资源/回读下标面(hzb on 才存在;29 起编与 encode 27/28 无撞面)。
#[derive(Debug, Clone)]
struct G34HzbIds {
    hit_t: u32,
    hit_pg: u32,
    hit_bary: u32,
    depth_hz: u32,
    inst_base: u32,
    flat: u32,
    mip_table: u32,
    stage: Vec<u32>,
    reduce_params: Vec<u32>,
    pack_params: Vec<u32>,
    rects_p1: u32,
    params_p1: u32,
    verdicts_p1: u32,
    rects_p2: u32,
    params_p2: u32,
    verdicts_p2: u32,
    rb_verdicts_p1: u32,
    rb_verdicts_p2: u32,
    rb_flat: u32,
    rb_depth_hz: u32,
}

/// G34-2 HZB 车道描述组装配(G34Full 四 pass 解构:mega scene 不进 HZB 车道
/// ——0-byte 不触;mv/resample/resolve 逐字保留;encode = G34 面逐字同)。
#[allow(clippy::too_many_arguments)]
fn g34_lane_descs_hzb<'x>(
    g34: (
        [ResourceDesc<'x>; U_RESOURCE_COUNT_G34],
        [Pass<'x>; 4],
        [&'static [(u32, TargetState)]; 4],
        [Readback; 5],
    ),
    enc_spv: &'x [u8],
    enc_dispatch: [u32; 3],
    enc_params_bytes: &'x [u8],
    hz: &'x G34HzbBits,
    n_instances: usize,
    iw: u32,
    ih: u32,
    ow: u32,
    oh: u32,
) -> (
    Vec<ResourceDesc<'x>>,
    Vec<Pass<'x>>,
    Vec<Vec<(u32, TargetState)>>,
    Vec<Readback>,
    G34HzbIds,
) {
    let (resources, passes, barriers, readbacks) = g34;
    let ipc = (iw * ih) as u64;
    let opc = (ow * oh) as u64;
    let storage = BufferUsage {
        storage: true,
        ..BufferUsage::default()
    };
    let mut resources = resources.to_vec();
    let mut readbacks = readbacks.to_vec();
    // G34Full 四 pass 解构:mega scene 不进 HZB 车道(0-byte 不触);mv/
    // resample/resolve 逐字保留(pass 对象与屏障计划同序搬运)。
    let [mega_scene, mv_pass, resample_pass, resolve_pass] = passes;
    let [plan_mega, plan_mv, plan_resample, plan_resolve] = barriers;
    let _ = (mega_scene, plan_mega);
    let buf = |size: u64| {
        ResourceDesc::Buffer(BufferDesc {
            size,
            usage: storage,
            data: None,
            device_local: true,
        })
    };
    let init = |bytes: &'x [u8]| {
        ResourceDesc::Buffer(BufferDesc {
            size: bytes.len() as u64,
            usage: storage,
            data: Some(bytes),
            device_local: true,
        })
    };
    let host_buf = |size: u64| {
        ResourceDesc::Buffer(BufferDesc {
            size,
            usage: storage,
            data: None,
            device_local: false,
        })
    };
    // 27/28 = 编码参数 + BGRA8 输出(G34 面逐字同)。
    resources.push(init(enc_params_bytes));
    resources.push(buf(opc * 4));
    let mut next = G34_U_RESOURCE_COUNT as u32;
    macro_rules! take {
        ($r:expr) => {{
            let id = next;
            next += 1;
            resources.push($r);
            id
        }};
    }
    let n_rect_bytes = (n_instances.max(1) * 5 * 4) as u64;
    let n_verd_bytes = (n_instances.max(1) * 4) as u64;
    let hit_t = take!(buf(ipc * 4));
    let hit_pg = take!(buf(ipc * 4));
    // fork A hitinfo 第三路(重心 bu/bv;shade 采样链 UV 插值输入)。
    let hit_bary = take!(buf(ipc * 8));
    // HZB 真深度面(g34_unified_shade ④b 段写出 = 真 ZO NDC;剔除链金字塔
    // mip0 专用源——U_SCENE_DEPTH 沿用生产字面供 MV/TSR,两路并存互不染指)。
    let depth_hz = take!(buf(ipc * 4));
    let inst_base = take!(init(&hz.inst_base_bytes));
    let flat = take!(init(&hz.flat_init_bytes));
    let mip_table = take!(init(&hz.mip_table_bytes));
    let mut stage = Vec::with_capacity(hz.levels.len() - 1);
    let mut reduce_params = Vec::with_capacity(hz.levels.len() - 1);
    for k in 1..hz.levels.len() {
        let (w, h) = hz.levels[k];
        stage.push(take!(buf((w * h) as u64 * 4)));
        reduce_params.push(take!(init(&hz.reduce_params_bytes[k - 1])));
    }
    let mut pack_params = Vec::with_capacity(hz.levels.len());
    for k in 0..hz.levels.len() {
        pack_params.push(take!(init(&hz.pack_params_bytes[k])));
    }
    let rects_p1 = take!(host_buf(n_rect_bytes));
    let params_p1 = take!(host_buf(8 * 4));
    let verdicts_p1 = take!(buf(n_verd_bytes));
    let rects_p2 = take!(host_buf(n_rect_bytes));
    let params_p2 = take!(host_buf(8 * 4));
    let verdicts_p2 = take!(buf(n_verd_bytes));
    let _ = next;
    let ids = G34HzbIds {
        hit_t,
        hit_pg,
        hit_bary,
        depth_hz,
        inst_base,
        flat,
        mip_table,
        stage,
        reduce_params,
        pack_params,
        rects_p1,
        params_p1,
        verdicts_p1,
        rects_p2,
        params_p2,
        verdicts_p2,
        rb_verdicts_p1: 6,
        rb_verdicts_p2: 7,
        rb_flat: 8,
        rb_depth_hz: 9,
    };
    let mut out_passes: Vec<Pass<'x>> = Vec::with_capacity(8 + 2 * hz.levels.len());
    let mut out_barriers: Vec<Vec<(u32, TargetState)>> =
        Vec::with_capacity(8 + 2 * hz.levels.len());
    // ── pass 0:primary(初剔后 TLAS = AS 表 0;读 inst_base/params,写 hitinfo
    //    三路——t/pg 全局下标/bary)──
    out_passes.push(Pass::Compute(ComputePass {
        name: &hz.name_primary,
        spirv: &hz.spv_primary,
        entry: None,
        dispatch: DispatchSpec::Direct(hz.primary_dispatch),
        bindings: Bindings {
            accel_structs: vec![0],
            storage_buffers: vec![inst_base, U_SCENE_PARAMS, hit_t, hit_pg, hit_bary],
            ..Bindings::default()
        },
    }));
    out_barriers.push(vec![
        (inst_base, TargetState::ShaderRead),
        (U_SCENE_PARAMS, TargetState::ShaderRead),
        (hit_t, TargetState::StorageWrite),
        (hit_pg, TargetState::StorageWrite),
        (hit_bary, TargetState::StorageWrite),
    ]);
    // ── pass 1:shade(全量 TLAS = AS 表 1;阴影射线零剔除 ⇒ 与统一 mega 面
    //    同域;fork A 五件 + hitinfo 三路读,写 out_color/out_depth/depth_hz)──
    out_passes.push(Pass::Compute(ComputePass {
        name: &hz.name_shade,
        spirv: &hz.spv_shade,
        entry: None,
        dispatch: DispatchSpec::Direct(hz.shade_dispatch),
        bindings: Bindings {
            accel_structs: vec![1],
            storage_buffers: vec![
                U_TRIS,
                U_MATS,
                U_QUADS,
                U_POINTS,
                U_SCENE_PARAMS,
                hit_t,
                hit_pg,
                hit_bary,
                G34_U_TEX_UV,
                G34_U_TEX_META,
                G34_U_TEX_TRITEX,
                G34_U_TEX_ATLAS,
                G34_U_TEX_LINLUT,
                U_SCENE_COLOR,
                U_SCENE_DEPTH,
                depth_hz,
            ],
            ..Bindings::default()
        },
    }));
    out_barriers.push(vec![
        (U_TRIS, TargetState::ShaderRead),
        (U_MATS, TargetState::ShaderRead),
        (U_QUADS, TargetState::ShaderRead),
        (U_POINTS, TargetState::ShaderRead),
        (U_SCENE_PARAMS, TargetState::ShaderRead),
        (hit_t, TargetState::ShaderRead),
        (hit_pg, TargetState::ShaderRead),
        (hit_bary, TargetState::ShaderRead),
        (G34_U_TEX_UV, TargetState::ShaderRead),
        (G34_U_TEX_META, TargetState::ShaderRead),
        (G34_U_TEX_TRITEX, TargetState::ShaderRead),
        (G34_U_TEX_ATLAS, TargetState::ShaderRead),
        (G34_U_TEX_LINLUT, TargetState::ShaderRead),
        (U_SCENE_COLOR, TargetState::StorageWrite),
        (U_SCENE_DEPTH, TargetState::StorageWrite),
        (depth_hz, TargetState::StorageWrite),
    ]);
    // ── pass 2..4:mv/resample/resolve(G34Full 逐字搬运)+ pass 5:encode ──
    out_passes.push(mv_pass);
    out_barriers.push(plan_mv.to_vec());
    out_passes.push(resample_pass);
    out_barriers.push(plan_resample.to_vec());
    out_passes.push(resolve_pass);
    out_barriers.push(plan_resolve.to_vec());
    out_passes.push(Pass::Compute(ComputePass {
        name: "g31_display_encode",
        spirv: enc_spv,
        entry: None,
        dispatch: DispatchSpec::Direct(enc_dispatch),
        bindings: Bindings {
            storage_buffers: vec![U_OUT_COLOR[0], G34_U_ENC_PARAMS, G34_U_ENC_OUT],
            ..Bindings::default()
        },
    }));
    out_barriers.push(G34_U_PLAN_ENCODE.to_vec());
    // ── pass 6:test_p1(全实例 rect vs 上帧金字塔——「上帧金字塔初剔」字面)──
    out_passes.push(Pass::Compute(ComputePass {
        name: &hz.name_test_p1,
        spirv: &hz.spv_test,
        entry: None,
        dispatch: DispatchSpec::Direct(hz.test_dispatch),
        bindings: Bindings {
            storage_buffers: vec![flat, mip_table, rects_p1, params_p1, verdicts_p1],
            ..Bindings::default()
        },
    }));
    out_barriers.push(vec![
        (flat, TargetState::ShaderRead),
        (mip_table, TargetState::ShaderRead),
        (rects_p1, TargetState::ShaderRead),
        (params_p1, TargetState::ShaderRead),
        (verdicts_p1, TargetState::StorageWrite),
    ]);
    // ── pass 7..:reduce×(L−1)(級 k:src = 上級〔k=1 = depth_hz 真深度,余 =
    //    stage k−1〕→ stage k;g27_hzb_reduce 0-byte 冻结消费)──
    for k in 1..hz.levels.len() {
        let src = if k == 1 { depth_hz } else { ids.stage[k - 2] };
        out_passes.push(Pass::Compute(ComputePass {
            name: &hz.reduce_names[k - 1],
            spirv: &hz.spv_reduce,
            entry: None,
            dispatch: DispatchSpec::Direct(hz.reduce_dispatch[k - 1]),
            bindings: Bindings {
                storage_buffers: vec![src, ids.reduce_params[k - 1], ids.stage[k - 1]],
                ..Bindings::default()
            },
        }));
        out_barriers.push(vec![
            (src, TargetState::ShaderRead),
            (ids.reduce_params[k - 1], TargetState::ShaderRead),
            (ids.stage[k - 1], TargetState::StorageWrite),
        ]);
    }
    // ── pack×L(級 0 = depth_hz 真深度原字节平铺,級 k≥1 = stage k;
    //    g31_hzb_pack 纯拷贝 glue 0-byte)──
    for k in 0..hz.levels.len() {
        let src = if k == 0 { depth_hz } else { ids.stage[k - 1] };
        out_passes.push(Pass::Compute(ComputePass {
            name: &hz.pack_names[k],
            spirv: &hz.spv_pack,
            entry: None,
            dispatch: DispatchSpec::Direct(hz.pack_dispatch[k]),
            bindings: Bindings {
                storage_buffers: vec![src, ids.pack_params[k], flat],
                ..Bindings::default()
            },
        }));
        out_barriers.push(vec![
            (src, TargetState::ShaderRead),
            (ids.pack_params[k], TargetState::ShaderRead),
            (flat, TargetState::StorageWrite),
        ]);
    }
    // ── 末 pass:test_p2(上帧被剔集 vs 本帧金字塔——「本帧重建重测」字面)──
    out_passes.push(Pass::Compute(ComputePass {
        name: &hz.name_test_p2,
        spirv: &hz.spv_test,
        entry: None,
        dispatch: DispatchSpec::Direct(hz.test_dispatch),
        bindings: Bindings {
            storage_buffers: vec![flat, mip_table, rects_p2, params_p2, verdicts_p2],
            ..Bindings::default()
        },
    }));
    out_barriers.push(vec![
        (flat, TargetState::ShaderRead),
        (mip_table, TargetState::ShaderRead),
        (rects_p2, TargetState::ShaderRead),
        (params_p2, TargetState::ShaderRead),
        (verdicts_p2, TargetState::StorageWrite),
    ]);
    // ── 回读表:0..=3 = unified 面(OUT_COLOR f32 双 parity/MV/DEPTH);4 =
    //    scene color(fork B 核验 + host 对拍);5 = BGRA8;6/7 = p1/p2 判定
    //    (逐帧决策面);8 = 平铺金字塔(probe 对拍面);9 = depth_hz 真深度
    //    (probe 对拍面——host 金字塔构建源与本帧平铺 mip0 位级同源)。──
    readbacks.push(Readback::Buffer {
        res: G34_U_ENC_OUT,
        offset: 0,
        size: opc * 4,
    });
    readbacks.push(Readback::Buffer {
        res: verdicts_p1,
        offset: 0,
        size: n_verd_bytes,
    });
    readbacks.push(Readback::Buffer {
        res: verdicts_p2,
        offset: 0,
        size: n_verd_bytes,
    });
    readbacks.push(Readback::Buffer {
        res: flat,
        offset: 0,
        size: (hz.flat_texels * 4) as u64,
    });
    readbacks.push(Readback::Buffer {
        res: depth_hz,
        offset: 0,
        size: ipc * 4,
    });
    (resources, out_passes, out_barriers, readbacks, ids)
}

// ---------------------------------------------------------------------------
// G34-2 车道状态机(顺序入口——逐帧 host 决策在环,FIF 流水面天然不适用;两
// 阶段调度 + 闭环重渲全记录;双 TLAS 逐帧 refit = dyn 面更新策略)。
// ---------------------------------------------------------------------------

/// G34-2 一帧 HZB 决策/调度产物(evidence 计数面 + probe 对拍面)。
struct G34HzbDecisionRec {
    tested_p1: u32,
    occluded_p1: u32,
    offscreen: u32,
    retested_p2: u32,
    flipped_p2: u32,
    closure_extra_submits: u32,
    closure_full_fallback: bool,
    visible_final: u32,
    hzb_gpu_ns: f64,
    closure_extra_gpu_ns: f64,
    host_ms: f64,
    probe_depth: Option<Vec<f32>>,
    probe_flat: Option<Vec<f32>>,
    verdicts_p1: Vec<u8>,
    rects_p1: Vec<f32>,
    rects_inst_p1: Vec<u32>,
}

/// G34-2 一帧产物(G34FrameRec 同构面 + HZB 决策块)。
struct G34HzbFrameRec {
    scene_gpu_ns: f64,
    mv_gpu_ns: f64,
    resample_gpu_ns: f64,
    resolve_gpu_ns: f64,
    encode_gpu_ns: f64,
    cpu_record_ns: u64,
    cpu_submit_ns: u64,
    cpu_fence_wait_ns: u64,
    validation_error_count: u64,
    leaked_object_count: u64,
    leaked_allocation_count: u64,
    bgra8: Option<Vec<u8>>,
    out_color: Option<Vec<f32>>,
    scene_color: Option<Vec<f32>>,
    scene_depth: Option<Vec<f32>>,
    readback_convert_ms: f64,
    hzb: G34HzbDecisionRec,
}

struct G34HzbLane<'a> {
    session: DeviceFrameSession<'a>,
    parity: usize,
    has_history_state: bool,
    prev_vp_j: Option<Mat4>,
    ids: G34HzbIds,
    /// 静态节点分组(剔除对象面;动态实例尾槽不参剔恒可见)。
    groups: Vec<SceneNodeGroup>,
    dyn_on: bool,
    dyn_tri_base: usize,
    /// 下一帧渲染掩码(host 决策面;0xFF = 可见 / 0x00 = 剔除;动态槽恒 0xFF)。
    masks: Vec<u8>,
    /// TLAS[0] 当前上传态(等价重更跳过——dyn off 静态相机稳态零 TLAS 税)。
    uploaded_masks: Vec<u8>,
    /// 上帧终判被剔集(本帧 test_p2 重测对象;rect 流 5 f32/rect + 实例号列)。
    prev_p2_rects: Vec<f32>,
    prev_p2_inst: Vec<u32>,
    /// 本帧 p1 流(决策/对拍消费;5 f32/rect + 实例号列)。
    last_rects_p1: Vec<f32>,
    last_rects_inst: Vec<u32>,
    n_levels: usize,
}

impl<'a> G34HzbLane<'a> {
    fn create(
        resources: &'a [ResourceDesc<'a>],
        passes: &'a [Pass<'a>],
        barriers: &'a [&'a [(u32, TargetState)]],
        readbacks: &'a [Readback],
        accel_structs: &[AccelStructDesc<'a>],
        ids: G34HzbIds,
        groups: Vec<SceneNodeGroup>,
        dyn_on: bool,
        dyn_tri_base: usize,
        n_levels: usize,
    ) -> Result<Self, String> {
        if !vk::vulkan_available() {
            return Err("vulkan loader 不可用".into());
        }
        if groups.is_empty() {
            return Err("HZB 面场景零可剔除实例(节点分组为空,fail-closed 不冒充)".into());
        }
        let n = groups.len() + usize::from(dyn_on);
        // frame_slots=2(顺序全同步既有面逐字同;逐帧 host 决策在环本就顺序,
        // FIF 拒 tlas_update——A2 约束同律登记)。
        let session = DeviceFrameSession::new_with_accel_structs(
            resources,
            passes,
            barriers,
            readbacks,
            2,
            accel_structs,
        )?;
        Ok(Self {
            session,
            parity: 0,
            has_history_state: false,
            prev_vp_j: None,
            ids,
            groups,
            dyn_on,
            dyn_tri_base,
            masks: vec![0xFF; n],
            uploaded_masks: vec![0xFF; n],
            prev_p2_rects: Vec::new(),
            prev_p2_inst: Vec::new(),
            last_rects_p1: Vec::new(),
            last_rects_inst: Vec::new(),
            n_levels,
        })
    }

    /// 实例表组装(表 0 = 掩码面/表 1 = 全 0xFF 面;动态槽变换 = 本帧 xf——
    /// 双 TLAS 各自 refit 的实例内容面)。
    fn instances_with(&self, masks: &[u8], dyn_xf: Option<[f32; 12]>) -> Vec<RayQueryTransformedInstanceDesc> {
        let mut v = Vec::with_capacity(self.masks.len());
        for (i, _) in self.groups.iter().enumerate() {
            v.push(RayQueryTransformedInstanceDesc {
                blas: i as u32,
                custom_index: i as u32,
                mask: masks[i],
                sbt_record_offset: 0,
                transform: vk::RAY_QUERY_IDENTITY_TRANSFORM,
            });
        }
        if self.dyn_on {
            let s = self.groups.len() as u32;
            v.push(RayQueryTransformedInstanceDesc {
                blas: s,
                custom_index: s,
                mask: masks[s as usize],
                sbt_record_offset: 0,
                transform: dyn_xf.unwrap_or(vk::RAY_QUERY_IDENTITY_TRANSFORM),
            });
        }
        v
    }

    /// 单次提交(两阶段调度的一拍):参数三小件 + rect 双流 + 双 TLAS 更新
    /// (表 0 = 掩码 + 动态变换〔dyn on 逐帧/dyn off 掩码等价重更跳过〕,表 1
    /// = 全 0xFF + 动态变换〔dyn on 逐帧;off 零更新〕)+ parity 三 pass 绑定
    /// 轮换 + 回读子集。
    #[allow(clippy::too_many_arguments)]
    fn submit_once(
        &mut self,
        scene_params: &[f32],
        mv_params: &[f32],
        tsr_params: &[f32],
        n_p1: u32,
        rects_p2: &[f32],
        n_p2: u32,
        masks: &[u8],
        dyn_xf: Option<[f32; 12]>,
        readback: G34Readback,
        probe_pre: bool,
        iw: u32,
        ih: u32,
    ) -> Result<DeviceFrameOutput, String> {
        let params_p1 = [
            n_p1 as f32,
            self.n_levels as f32,
            iw as f32,
            ih as f32,
            G34HZB_CONV_FLAG,
            0.0,
            0.0,
            0.0,
        ];
        let params_p2 = [
            n_p2 as f32,
            self.n_levels as f32,
            iw as f32,
            ih as f32,
            G34HZB_CONV_FLAG,
            0.0,
            0.0,
            0.0,
        ];
        let ids = &self.ids;
        let mut uploads: Vec<(StableResourceId, u64, Vec<u8>)> = vec![
            (
                StableResourceId(u64::from(U_SCENE_PARAMS) + 1),
                0,
                bytes_f32(scene_params),
            ),
            (
                StableResourceId(u64::from(U_MV_PARAMS) + 1),
                0,
                bytes_f32(mv_params),
            ),
            (
                StableResourceId(u64::from(U_TSR_PARAMS) + 1),
                0,
                bytes_f32(tsr_params),
            ),
        ];
        // rect 流空段不上传(执行器 fail-closed 拒空段;kernel 以 params[0]=n
        // 门守卫,缓冲陈旧段永不被消费——n=0 拍跳过上传零语义差)。
        if !self.last_rects_p1.is_empty() {
            uploads.push((
                StableResourceId(u64::from(ids.rects_p1) + 1),
                0,
                bytes_f32(&self.last_rects_p1),
            ));
        }
        uploads.push((
            StableResourceId(u64::from(ids.params_p1) + 1),
            0,
            bytes_f32(&params_p1),
        ));
        if !rects_p2.is_empty() {
            uploads.push((
                StableResourceId(u64::from(ids.rects_p2) + 1),
                0,
                bytes_f32(rects_p2),
            ));
        }
        uploads.push((
            StableResourceId(u64::from(ids.params_p2) + 1),
            0,
            bytes_f32(&params_p2),
        ));
        // ── 双 TLAS 逐帧 refit(dyn 面更新策略)──
        // 表 0(初剔):dyn on = 逐帧(动态变换逐帧变);dyn off = 掩码等价重更
        // 跳过(静态相机稳态零 TLAS 税——G31 B1 同律)。
        let tlas_update = if self.dyn_on || masks != self.uploaded_masks.as_slice() {
            Some((
                0u32,
                self.instances_with(masks, dyn_xf),
                TlasBuildAction::Refit,
            ))
        } else {
            None
        };
        // 表 1(全量):dyn on = 逐帧 refit(全 0xFF 掩码 + 动态变换——阴影射线
        // 与主射线同帧同位姿);dyn off = 零更新(创建期内容静态正确)。
        let tlas_update_b = if self.dyn_on {
            Some((
                1u32,
                self.instances_with(&vec![0xFF; self.masks.len()], dyn_xf),
                TlasBuildAction::Refit,
            ))
        } else {
            None
        };
        let p = self.parity;
        let binding_overrides = vec![
            (
                3u32,
                Bindings {
                    storage_buffers: vec![
                        U_SCENE_COLOR,
                        U_SCENE_DEPTH,
                        U_TSR_PARAMS,
                        U_CUR_RGB,
                        U_LUMA[p],
                        U_DEPTH_HI[p],
                    ],
                    ..Bindings::default()
                },
            ),
            (
                4u32,
                Bindings {
                    storage_buffers: vec![
                        U_CUR_RGB,
                        U_LUMA[p],
                        U_DEPTH_HI[p],
                        U_MV_OUT,
                        U_REACTIVE,
                        U_OUT_COLOR[1 - p],
                        U_DEPTH_HI[1 - p],
                        U_LUMA[1 - p],
                        U_OUT_SIGN[1 - p],
                        U_OUT_SCORE[1 - p],
                        U_TSR_PARAMS,
                        U_OUT_COLOR[p],
                        U_OUT_SIGN[p],
                        U_OUT_SCORE[p],
                    ],
                    ..Bindings::default()
                },
            ),
            (
                5u32,
                Bindings {
                    storage_buffers: vec![U_OUT_COLOR[p], G34_U_ENC_PARAMS, G34_U_ENC_OUT],
                    ..Bindings::default()
                },
            ),
        ];
        // 回读子集(序即解析序):模式面(BGRA8/scene depth/scene color/f32
        // out_color)→ probe_pre 深度+平铺 → p1/p2 判定(逐帧恒在,决策面)。
        let mut subset: Vec<u32> = Vec::new();
        match readback {
            G34Readback::None => {}
            G34Readback::Bgra => subset.push(G34_RB_BGRA),
            G34Readback::BgraAndScene => {
                subset.push(3);
                subset.push(G34_RB_SCENE);
                subset.push(G34_RB_BGRA);
            }
            G34Readback::Full => {
                subset.push(p as u32);
                subset.push(3);
                subset.push(G34_RB_SCENE);
                subset.push(G34_RB_BGRA);
            }
        }
        if probe_pre {
            // probe 深度面 = depth_hz 真深度(回读下标 9;剔除链深度域与设备
            // 平铺 mip0 位级同源——3 = U_SCENE_DEPTH 生产字面不供剔除链消费)。
            subset.push(ids.rb_depth_hz);
            subset.push(ids.rb_flat);
        }
        subset.push(ids.rb_verdicts_p1);
        subset.push(ids.rb_verdicts_p2);
        let update = FrameUpdate {
            tlas_update,
            buffer_uploads: uploads,
            binding_overrides,
            push_constant_overrides: vec![],
            readback_subset: Some(subset),
            blas_refit: None, // G34-2 无 BLAS refit 面(fork B = TLAS 实例变换 UPDATE)
        };
        let prov = self
            .session
            .next_provenance_with_update_dual_tlas(&update, tlas_update_b.as_ref())?;
        let out = self
            .session
            .execute_with_frame_update_dual_tlas(&prov, &update, tlas_update_b)?;
        if update.tlas_update.is_some() {
            self.uploaded_masks = masks.to_vec();
        }
        Ok(out)
    }

    /// 一帧:初剔分类(host)→ 提交(两阶段 pass 序)→ collect 结算应见集 →
    /// 误剔/出新闭环重渲(迭代上限 + 全掩码兜底)→ 终判掩码/被剔集滚动。
    /// scene_params 由调用方预打包(60 f32 dyn 面;dyn_tri_base 同包)。
    #[allow(clippy::too_many_arguments)]
    fn frame(
        &mut self,
        iw: u32,
        ih: u32,
        ow: u32,
        oh: u32,
        scene_params: &[f32],
        jitter: [f32; 2],
        vp: &Mat4,
        vp_j: &Mat4,
        exposure: f32,
        reset: bool,
        readback: G34Readback,
        probe_pre: bool,
        dyn_xf: Option<[f32; 12]>,
    ) -> Result<G34HzbFrameRec, String> {
        let t_host = std::time::Instant::now();
        // ── ① 初剔分类(视锥面 + rect 流 + 动态尾槽恒可见;cull::Frustum 冻结
        //    金标准只读消费)──
        let class = g34_hzb_classify(vp, iw, ih, &self.groups, self.dyn_on);
        let n = self.masks.len();
        let mut rects: Vec<f32> = Vec::with_capacity(n * 5);
        let mut rect_inst: Vec<u32> = Vec::with_capacity(n);
        let mut offscreen = 0u32;
        for (i, c) in class.iter().enumerate() {
            match c {
                G34HzbClass::Offscreen => offscreen += 1,
                G34HzbClass::Rect {
                    uv_min,
                    uv_max,
                    nearest,
                } => {
                    rect_inst.push(i as u32);
                    rects.extend_from_slice(&[uv_min[0], uv_min[1], uv_max[0], uv_max[1], *nearest]);
                }
            }
        }
        self.last_rects_p1 = rects.clone();
        self.last_rects_inst = rect_inst.clone();
        let n_p1 = rect_inst.len() as u32;
        // ── ② mv/tsr 参数面(与 off 车道同一打包面逐字同源)──
        let inv_cur = vp_j
            .inverse()
            .ok_or("jittered view-proj 必须可逆(mv 参数面)")?;
        let prev = self.prev_vp_j.unwrap_or(*vp_j);
        let mv_params = pack_mv_params(iw, ih, &inv_cur, &prev, self.prev_vp_j.is_some());
        let has_history = !reset && self.has_history_state;
        // jitter 位级同源面:scene_params[3/4] = pack_frame_params_dyn 写入的同
        // 一 jitter——直用参数面与 off 车道 pack_tsr_params 调用字面同形。
        let tsr_params = pack_tsr_params(iw, ih, ow, oh, jitter, exposure, has_history, false);
        // host 决策面耗时 = 初剔分类 + rect 流打包段(µs 级,不重复计 GPU 段)。
        let host_ms = t_host.elapsed().as_secs_f64() * 1000.0;

        // ── ③ 两阶段提交 + 闭环重渲循环 ──
        let mut rendered = self.masks.clone();
        let mut p2_rects = self.prev_p2_rects.clone();
        let mut p2_inst = self.prev_p2_inst.clone();
        let mut closure_extra_submits = 0u32;
        let mut closure_full_fallback = false;
        let mut hzb_gpu_ns = 0.0f64;
        let mut prod_gpu_total_ns = 0.0f64;
        // 主提交 p1 判定面(probe 对拍消费——「上帧金字塔初剔」字面;闭环重拍
        // 的 p1 读本帧重建金字塔属第二阶段调度,不进对拍面)。
        let mut v1_main: Option<Vec<u8>> = None;
        // 末次提交面(循环出口赋值):判定/遥测/回读归属。
        let (out_last, v1_last, v2_last, p2_inst_last);
        loop {
            let n_p2 = p2_inst.len() as u32;
            let out = self.submit_once(
                scene_params,
                &mv_params,
                &tsr_params,
                n_p1,
                &p2_rects,
                n_p2,
                &rendered,
                dyn_xf,
                readback,
                probe_pre,
                iw,
                ih,
            )?;
            let (v1, v2) = g34_hzb_parse_verdicts(&out, readback, probe_pre, n_p1, n_p2)?;
            if v1_main.is_none() {
                v1_main = Some(v1.clone());
            }
            let prod_ns = g34_hzb_prod_gpu_ns(&out)?;
            prod_gpu_total_ns += prod_ns;
            hzb_gpu_ns += g34_hzb_aux_gpu_ns(&out);
            // 应见集结算:p1 可见 ∪ p2 翻回(offscreen 恒剔)。
            let mut correct = vec![0u8; n];
            for (j, &inst) in rect_inst.iter().enumerate() {
                if v1[j] == 0 {
                    correct[inst as usize] = 0xFF;
                }
            }
            for (j, &inst) in p2_inst.iter().enumerate() {
                if v2[j] == 0 {
                    correct[inst as usize] = 0xFF;
                }
            }
            let need = (0..n).any(|i| correct[i] == 0xFF && rendered[i] == 0);
            if !need {
                out_last = out;
                v1_last = v1;
                v2_last = v2;
                p2_inst_last = p2_inst;
                break;
            }
            // 闭环:并集掩码重渲(并集内每一员要么应见、要么被并集内他员遮挡
            // ⇒ 超集渲染像素安全;金字塔逐次更完备 ⇒ 遮挡集单调扩 ⇒ 不振荡)。
            for (i, c) in correct.iter().enumerate() {
                if *c == 0xFF {
                    rendered[i] = 0xFF;
                }
            }
            // 下一拍重测集 = 在屏且仍被剔(并集外)。
            p2_rects = Vec::new();
            p2_inst = Vec::new();
            for (j, &inst) in rect_inst.iter().enumerate() {
                if rendered[inst as usize] == 0 {
                    p2_inst.push(inst);
                    p2_rects.extend_from_slice(&rects[j * 5..j * 5 + 5]);
                }
            }
            closure_extra_submits += 1;
            if closure_extra_submits >= G34HZB_CLOSURE_MAX {
                // 迭代上限耗尽 ⇒ 全掩码兜底重渲(= 零剔除精确收敛,必终止)。
                rendered = vec![0xFF; n];
                p2_rects = Vec::new();
                p2_inst = Vec::new();
                closure_full_fallback = true;
                let out2 = self.submit_once(
                    scene_params,
                    &mv_params,
                    &tsr_params,
                    n_p1,
                    &p2_rects,
                    0,
                    &rendered,
                    dyn_xf,
                    readback,
                    probe_pre,
                    iw,
                    ih,
                )?;
                let (v1b, v2b) = g34_hzb_parse_verdicts(&out2, readback, probe_pre, n_p1, 0)?;
                prod_gpu_total_ns += g34_hzb_prod_gpu_ns(&out2)?;
                hzb_gpu_ns += g34_hzb_aux_gpu_ns(&out2);
                out_last = out2;
                v1_last = v1b;
                v2_last = v2b;
                p2_inst_last = p2_inst;
                break;
            }
        }

        // ── ④ 终判滚动:下帧渲染掩码 = 本帧应见集(末次提交判定面);下帧 p2
        //    重测集 = 本帧终判被剔(在屏且应见集外)。──
        let mut visible_final = vec![0u8; n];
        for (j, &inst) in rect_inst.iter().enumerate() {
            if v1_last[j] == 0 {
                visible_final[inst as usize] = 0xFF;
            }
        }
        for (j, &inst) in p2_inst_last.iter().enumerate() {
            if v2_last[j] == 0 {
                visible_final[inst as usize] = 0xFF;
            }
        }
        let mut next_p2_rects: Vec<f32> = Vec::new();
        let mut next_p2_inst: Vec<u32> = Vec::new();
        for (j, &inst) in rect_inst.iter().enumerate() {
            if visible_final[inst as usize] == 0 {
                next_p2_inst.push(inst);
                next_p2_rects.extend_from_slice(&rects[j * 5..j * 5 + 5]);
            }
        }
        self.masks = visible_final;
        self.prev_p2_rects = next_p2_rects;
        self.prev_p2_inst = next_p2_inst;

        // ── ⑤ 产物组装(遥测 = 末次提交;HZB/闭环追加 GPU 分列;判定面 = 主
        //    提交)──
        let prod_last_ns = g34_hzb_prod_gpu_ns(&out_last)?;
        let closure_extra_ns = prod_gpu_total_ns - prod_last_ns;
        let verdicts_p1_rec = v1_main.clone().unwrap_or_else(|| v1_last.clone());
        let rec = self.rec_from_output_hz(
            out_last,
            readback,
            probe_pre,
            ow,
            oh,
            iw,
            ih,
            hzb_gpu_ns,
            G34HzbDecisionRec {
                tested_p1: n_p1,
                occluded_p1: 0, // 占位——rec_from_output_hz 统计口径填入
                offscreen,
                retested_p2: 0,
                flipped_p2: 0,
                closure_extra_submits,
                closure_full_fallback,
                visible_final: self.masks.iter().filter(|&&m| m == 0xFF).count() as u32,
                hzb_gpu_ns,
                closure_extra_gpu_ns: closure_extra_ns,
                host_ms,
                probe_depth: None,
                probe_flat: None,
                verdicts_p1: verdicts_p1_rec.clone(),
                rects_p1: self.last_rects_p1.clone(),
                rects_inst_p1: self.last_rects_inst.clone(),
            },
            &verdicts_p1_rec,
            &p2_inst_last,
            &v2_last,
        )?;
        self.prev_vp_j = Some(*vp_j);
        self.has_history_state = true;
        self.parity = 1 - self.parity;
        Ok(rec)
    }

    /// 末次提交产物组装(回读按子集同序解析 + 尺寸校验 + 遥测按 pass 名提取)。
    #[allow(clippy::too_many_arguments)]
    fn rec_from_output_hz(
        &self,
        mut out: DeviceFrameOutput,
        readback: G34Readback,
        probe_pre: bool,
        ow: u32,
        oh: u32,
        iw: u32,
        ih: u32,
        hzb_gpu_ns: f64,
        mut hz: G34HzbDecisionRec,
        v1_last: &[u8],
        p2_inst_last: &[u32],
        v2_last: &[u8],
    ) -> Result<G34HzbFrameRec, String> {
        let gpu = |name: &str| -> Result<f64, String> {
            out.telemetry
                .passes
                .iter()
                .find(|pp| pp.name == name)
                .map(|pp| pp.gpu_ns)
                .ok_or_else(|| format!("telemetry 缺 {name} pass 行"))
        };
        let scene_gpu_ns = gpu("g34_unified_primary")? + gpu("g34_unified_shade")?;
        let mv_gpu_ns = gpu("g14_mv")?;
        let resample_gpu_ns = gpu("g14_8_tsr_resample")?;
        let resolve_gpu_ns = gpu("g14_8_tsr_resolve")?;
        let encode_gpu_ns = gpu("g31_display_encode")?;
        let t_convert = std::time::Instant::now();
        let bgra_px = (ow * oh * 4) as usize;
        let f32_px = (ow * oh * 3) as usize;
        let scene_px = (iw * ih * 3) as usize;
        let depth_px = (iw * ih) as usize;
        let mut idx = 0usize;
        let take_rb = |out: &mut DeviceFrameOutput, idx: &mut usize| -> Result<Vec<u8>, String> {
            if *idx >= out.readbacks.len() {
                return Err(format!(
                    "G34-2 回读路数 {} 少于子集消费序 {idx}",
                    out.readbacks.len()
                ));
            }
            let b = std::mem::take(&mut out.readbacks[*idx]);
            *idx += 1;
            Ok(b)
        };
        let (bgra8, out_color, scene_color, scene_depth) = match readback {
            G34Readback::None => (None, None, None, None),
            G34Readback::Bgra => {
                let b = take_rb(&mut out, &mut idx)?;
                if b.len() != bgra_px {
                    return Err(format!("G34-2 BGRA8 回读字节 {} ≠ {ow}x{oh}x4", b.len()));
                }
                (Some(b), None, None, None)
            }
            G34Readback::BgraAndScene => {
                let d = read_f32(&take_rb(&mut out, &mut idx)?);
                if d.len() != depth_px {
                    return Err("G34-2 scene depth 回读字节数与内部分辨率不符".into());
                }
                let s = read_f32(&take_rb(&mut out, &mut idx)?);
                if s.len() != scene_px {
                    return Err("G34-2 scene color 回读字节数与内部分辨率不符".into());
                }
                let b = take_rb(&mut out, &mut idx)?;
                if b.len() != bgra_px {
                    return Err(format!("G34-2 BGRA8 回读字节 {} ≠ {ow}x{oh}x4", b.len()));
                }
                (Some(b), None, Some(s), Some(d))
            }
            G34Readback::Full => {
                let c = read_f32(&take_rb(&mut out, &mut idx)?);
                if c.len() != f32_px {
                    return Err("G34-2 f32 out_color 回读字节数与输出分辨率不符".into());
                }
                let d = read_f32(&take_rb(&mut out, &mut idx)?);
                if d.len() != depth_px {
                    return Err("G34-2 scene depth 回读字节数与内部分辨率不符".into());
                }
                let s = read_f32(&take_rb(&mut out, &mut idx)?);
                if s.len() != scene_px {
                    return Err("G34-2 scene color 回读字节数与内部分辨率不符".into());
                }
                let b = take_rb(&mut out, &mut idx)?;
                if b.len() != bgra_px {
                    return Err(format!("G34-2 BGRA8 回读字节 {} ≠ {ow}x{oh}x4", b.len()));
                }
                (Some(b), Some(c), Some(s), Some(d))
            }
        };
        let (probe_depth, probe_flat) = if probe_pre {
            let d = read_f32(&take_rb(&mut out, &mut idx)?);
            let f = read_f32(&take_rb(&mut out, &mut idx)?);
            (Some(d), Some(f))
        } else {
            (None, None)
        };
        // 判定两路(逐帧恒在子集末两位)。
        let _ = take_rb(&mut out, &mut idx)?; // verdicts_p1 字节(frame() 已解析消费)
        let _ = take_rb(&mut out, &mut idx)?; // verdicts_p2
        if idx != out.readbacks.len() {
            return Err(format!(
                "G34-2 回读消费序 {idx} ≠ 实到路数 {}",
                out.readbacks.len()
            ));
        }
        hz.occluded_p1 = v1_last.iter().filter(|&&b| b == 1).count() as u32;
        hz.retested_p2 = p2_inst_last.len() as u32;
        hz.flipped_p2 = v2_last.iter().filter(|&&b| b == 0).count() as u32;
        hz.probe_depth = probe_depth;
        hz.probe_flat = probe_flat;
        hz.hzb_gpu_ns = hzb_gpu_ns;
        let readback_convert_ms = t_convert.elapsed().as_secs_f64() * 1000.0;
        Ok(G34HzbFrameRec {
            scene_gpu_ns,
            mv_gpu_ns,
            resample_gpu_ns,
            resolve_gpu_ns,
            encode_gpu_ns,
            cpu_record_ns: out.telemetry.cpu_record_ns,
            cpu_submit_ns: out.telemetry.cpu_submit_ns,
            cpu_fence_wait_ns: out.telemetry.cpu_fence_wait_ns,
            validation_error_count: out.telemetry.validation_error_count,
            leaked_object_count: out.telemetry.leaked_object_count,
            leaked_allocation_count: out.telemetry.leaked_allocation_count,
            bgra8,
            out_color,
            scene_color,
            scene_depth,
            readback_convert_ms,
            hzb: hz,
        })
    }
}

/// G34-2 生产链六段 GPU(primary+shade+mv+resample+resolve+encode;末次提交
/// 口径由 rec_from_output_hz 逐名提取,本面供闭环追加量分列)。
fn g34_hzb_prod_gpu_ns(out: &DeviceFrameOutput) -> Result<f64, String> {
    let mut sum = 0.0;
    for name in [
        "g34_unified_primary",
        "g34_unified_shade",
        "g14_mv",
        "g14_8_tsr_resample",
        "g14_8_tsr_resolve",
        "g31_display_encode",
    ] {
        sum += out
            .telemetry
            .passes
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.gpu_ns)
            .ok_or_else(|| format!("telemetry 缺 {name} pass 行"))?;
    }
    Ok(sum)
}

/// G34-2 HZB 辅助 pass GPU 合计(g27_hzb_reduce_l*/g27_hzb_test_p*/
/// g31_hzb_pack_l* 前缀族;缺行 = 0 容差〔辅助面不全不冒充,主链六段缺失
/// 才 fail〕)。
fn g34_hzb_aux_gpu_ns(out: &DeviceFrameOutput) -> f64 {
    out.telemetry
        .passes
        .iter()
        .filter(|p| {
            p.name.starts_with("g27_hzb_reduce_l")
                || p.name.starts_with("g27_hzb_test_p")
                || p.name.starts_with("g31_hzb_pack_l")
        })
        .map(|p| p.gpu_ns)
        .sum()
}

/// G34-2 判定回读解析(子集序 = 末两位;f32 恒 ∈ {0.0,1.0} 门输出 ⇒ >0.5
/// 判读字节,g27/G31 harness 同律)。返回 (p1 字节列, p2 字节列)(各取前 n 项)。
fn g34_hzb_parse_verdicts(
    out: &DeviceFrameOutput,
    readback: G34Readback,
    probe_pre: bool,
    n_p1: u32,
    n_p2: u32,
) -> Result<(Vec<u8>, Vec<u8>), String> {
    let base = match readback {
        G34Readback::None => 0,
        G34Readback::Bgra => 1,
        G34Readback::BgraAndScene => 3,
        G34Readback::Full => 4,
    } + if probe_pre { 2 } else { 0 };
    let rbs = &out.readbacks;
    if rbs.len() != base + 2 {
        return Err(format!(
            "G34-2 判定回读路数 {} ≠ {}(readback={readback:?} probe_pre={probe_pre})",
            rbs.len(),
            base + 2
        ));
    }
    let v1f = read_f32(&rbs[base]);
    let v2f = read_f32(&rbs[base + 1]);
    if (v1f.len() as u32) < n_p1 || (v2f.len() as u32) < n_p2 {
        return Err("G34-2 判定回读长度小于本拍 rect 数".into());
    }
    let to_bytes = |v: &[f32], n: u32| -> Vec<u8> {
        v.iter()
            .take(n as usize)
            .map(|&x| u8::from(x > 0.5))
            .collect()
    };
    Ok((to_bytes(&v1f, n_p1), to_bytes(&v2f, n_p2)))
}

// ---------------------------------------------------------------------------
// G34-2 接线态对拍(probe 两帧成对:预备帧回读上帧平铺金字塔 + depth_hz 真
// 深度,本帧 p1 判定序列 host 复算;geometry/{hzb,cull}.rs 冻结面只读消费)。
// ---------------------------------------------------------------------------

/// G34-2 接线态对拍结果(evidence `hzb.parity` 组装面)。
struct G34HzbWiredParity {
    mips: usize,
    n_rects: u32,
    mips_bitexact: bool,
    verdict_equal: bool,
    false_positives: u32,
    occluded: u32,
    pyramid_digest: String,
    host_pyramid_digest: String,
    verdict_digest: String,
    host_verdict_digest: String,
}

/// G34-2 probe 帧 host 金标准复算对拍(判据三面 = G31 B1 同律:① 车道平铺
/// 金字塔 vs host `HzbPyramid::build` 逐级位级全等〔to_bits;零容差协议——纯
/// min/max 选择归约 + 纯拷贝 pack glue ⇒ 位级蕴含〕;② p1 判定序列 vs host
/// `test_rect` 逐 rect 逐字节全等〔同一金字塔 + 同一 rect 流〕;③ 零假阳性
/// 硬不变量:device 判 Occluded ⇒ `exact_rect_occluded` 对上帧深度必同判)。
fn g34_hzb_wired_parity(
    depth_data: &[f32],
    flat_data: &[f32],
    iw: u32,
    ih: u32,
    levels: &[(u32, u32)],
    flat_offsets: &[u32],
    rects: &[f32],
    verdicts: &[u8],
) -> Result<G34HzbWiredParity, String> {
    if depth_data.len() != (iw * ih) as usize {
        return Err(format!(
            "probe 深度回读 {} ≠ {}x{}",
            depth_data.len(),
            iw,
            ih
        ));
    }
    let depth_img = ImageF32 {
        w: iw,
        h: ih,
        c: 1,
        data: depth_data.to_vec(),
    };
    let host = HzbPyramid::build(&depth_img, DepthConvention::StandardZ);
    // ① 逐级位级(平铺偏移逐級比;零容差)。
    let mut mips_bitexact = host.mips.len() == levels.len();
    if host.mips.len() != levels.len() {
        eprintln!(
            "{GTAG}: HZB 对拍① 级数不等 host={} lane={}",
            host.mips.len(),
            levels.len()
        );
    }
    if mips_bitexact {
        'levels: for (k, m) in host.mips.iter().enumerate() {
            let off = flat_offsets[k] as usize;
            if (m.w, m.h) != levels[k] || off + m.data.len() > flat_data.len() {
                mips_bitexact = false;
                break;
            }
            for (j, v) in m.data.iter().enumerate() {
                if flat_data[off + j].to_bits() != v.to_bits() {
                    eprintln!(
                        "{GTAG}: HZB 对拍① 首失配 level={k} j={j} dev={:08x} host={:08x}",
                        flat_data[off + j].to_bits(),
                        v.to_bits()
                    );
                    mips_bitexact = false;
                    break 'levels;
                }
            }
        }
    }
    // ② 判定序列逐字节。
    let host_seq: Vec<u8> = rects
        .chunks_exact(5)
        .map(|r| match host.test_rect([r[0], r[1]], [r[2], r[3]], r[4]) {
            Occlusion::Occluded => 1u8,
            Occlusion::Visible => 0u8,
        })
        .collect();
    let verdict_equal = host_seq.as_slice() == verdicts;
    if !verdict_equal {
        eprintln!(
            "{GTAG}: HZB 对拍② 差异计数 {}",
            host_seq
                .iter()
                .zip(verdicts.iter())
                .filter(|(a, b)| a != b)
                .count()
        );
    }
    // ③ 零假阳性独立复核(对上帧深度——device 初剔消费的金字塔同源)。
    let mut fp = 0u32;
    let mut occ = 0u32;
    for (j, &b) in verdicts.iter().enumerate() {
        if b == 1 {
            occ += 1;
            let r = &rects[j * 5..j * 5 + 5];
            if !exact_rect_occluded(
                &depth_img,
                DepthConvention::StandardZ,
                [r[0], r[1]],
                [r[2], r[3]],
                r[4],
            ) {
                fp += 1;
            }
        }
    }
    // digest(判定字节序 ‖ 金字塔逐级 f32 LE;G31 F11 字面同律)。
    let mut pyr_bytes = Vec::with_capacity(flat_data.len() * 4);
    for v in flat_data {
        pyr_bytes.extend_from_slice(&v.to_le_bytes());
    }
    let mut host_pyr_bytes = Vec::new();
    for m in &host.mips {
        for v in &m.data {
            host_pyr_bytes.extend_from_slice(&v.to_le_bytes());
        }
    }
    let mut vtrace = verdicts.to_vec();
    vtrace.extend_from_slice(&pyr_bytes);
    let mut htrace = host_seq.clone();
    htrace.extend_from_slice(&host_pyr_bytes);
    Ok(G34HzbWiredParity {
        mips: host.mips.len(),
        n_rects: verdicts.len() as u32,
        mips_bitexact,
        verdict_equal,
        false_positives: fp,
        occluded: occ,
        pyramid_digest: format!("sha256:{}", sha256_hex(&pyr_bytes)),
        host_pyramid_digest: format!("sha256:{}", sha256_hex(&host_pyr_bytes)),
        verdict_digest: format!("sha256:{}", sha256_hex(&vtrace)),
        host_verdict_digest: format!("sha256:{}", sha256_hex(&htrace)),
    })
}

// ---------------------------------------------------------------------------
// G34-2 主流程（--hzb on 面：HZB 两阶段遮挡剔除×纹理×slab×动态实例四特性
// 同开真窗口统一车道;main() 早分支唯一消费面）
// ---------------------------------------------------------------------------

/// --hzb on CLI 面（主 bin 早分支消费;全字段 = main 既有 CLI 同名面 + HZB
/// kernel SPV 五件——primary/shade = G34-2 统一件,pack = g31 glue 0-byte,
/// reduce/test = g27 本体 0-byte 冻结消费;spv_scene = 统一 mega kernel,仅供
/// 描述组装配期借用,mega pass 不进 HZB 车道零消费）。
struct G34HzbCli {
    frames: u32,
    warmup: u32,
    tier: u32,
    contract_path: String,
    g10_dir: String,
    gltf_path: String,
    spv_hzb_primary: String,
    spv_hzb_shade: String,
    spv_hzb_pack: String,
    spv_hzb_reduce: String,
    spv_hzb_test: String,
    spv_scene: String,
    spv_mv: String,
    spv_resample: String,
    spv_resolve: String,
    spv_encode: String,
    spv_slab: String,
    spv_texture_probe: String,
    evidence_path: String,
    expect_digest: Option<String>,
    hidden: bool,
    auto_move: String,
    slab_table: String,
}

/// G34-2 HZB 主流程（main() 早分支唯一消费面;装配段 = main ①..⑤ 同函复用
/// ——契约链/G10 转引/scene 装配〔节点分组 + UV sink 双记录面同装配一次产出〕/
/// slab 接线/纹理接线/窗口创建逐字同律;host 金标准全场景对拍面不建——HZB 腿
/// 对拍 = probe 帧金字塔/判定/零假阳性三面承载,登记口径）。
fn g34_hzb_main(cli: G34HzbCli) -> ! {
    let G34HzbCli {
        frames,
        warmup,
        tier,
        contract_path,
        g10_dir,
        mut gltf_path,
        spv_hzb_primary,
        spv_hzb_shade,
        spv_hzb_pack,
        spv_hzb_reduce,
        spv_hzb_test,
        spv_scene,
        spv_mv,
        spv_resample,
        spv_resolve,
        spv_encode,
        spv_slab,
        spv_texture_probe,
        evidence_path,
        expect_digest,
        hidden,
        auto_move,
        slab_table,
    } = cli;
    // 登记实验臂位（分类器 env 消费面同源读出;evidence hzb.all_visible_arm）。
    let all_visible_arm = std::env::var("RURIX_HZB_ALL_VISIBLE").ok().as_deref() == Some("1");

    // ① 生产契约 + ② G10 语料转引一致性核验（main 同律,不等即 RED 拒跑）。
    let scene_id = "bistro-interior";
    let (pre, _) = prelude(
        scene_id,
        tier,
        frames,
        false,
        &contract_path,
        expect_digest.as_deref(),
    );
    let contract = &pre.contract;
    let (out_w, out_h, seed) = (pre.out_w, pre.out_h, pre.seed);
    let srow = contract_scene_row(&contract.raw, scene_id).unwrap_or_else(|e| fail(&e));
    let g10_fragment = match g34_g10_corpus_gate(srow, &g10_dir) {
        Ok(f) => f,
        Err(e) => fail(&format!("G10 语料转引一致性核验 RED: {e}")),
    };
    eprintln!(
        "{GTAG}: [hzb] 契约链就绪 contract_digest={} g10 转引一致性=pass all_visible_arm={all_visible_arm}",
        contract.digest
    );

    // ③ 场景装配（B1 节点分组 + B4 UV sink 双记录面同装配一次产出——SceneData
    //    各字段与两 sink 均 None 形态逐位同值,纯记录面;剔除对象粒度 = TLAS
    //    实例粒度 = 逐 mesh 节点）。
    if gltf_path.is_empty() {
        gltf_path = default_gltf(scene_id).to_owned();
    }
    let mut hzb_groups: Vec<SceneNodeGroup> = Vec::new();
    let mut tri_uv: Vec<f32> = Vec::new();
    let mut scene = match assemble_scene_ex(
        &contract.raw,
        scene_id,
        Path::new(&gltf_path),
        Some(&mut hzb_groups),
        Some(&mut tri_uv),
    ) {
        Ok(s) => s,
        Err(e) => dev_env_or_fail("scene_assets", &e),
    };
    if hzb_groups.is_empty() {
        fail("HZB 面场景零可剔除实例（节点分组为空,fail-closed 不冒充）");
    }

    // ③.5 slab 侧表生产接线（main ③.5 逐字同律——HZB 腿 = --full 面,slab
    //     资产必经;16 槽 device/host 双臂对拍 + 逐三角 albedo × R_slot 预调制）。
    let (slab_report, slab_arm): (Option<(SlabSideTableAsset, SlabEval, usize)>, Option<[f32; SLAB_N_SLOTS]>) = {
        let asset = match slab_load_asset(&slab_table) {
            Ok(a) => a,
            Err(e) => fail(&format!("slab 侧表资产加载: {e}")),
        };
        if asset.scene_id != scene_id {
            fail(&format!(
                "slab 资产 scene_id={} ≠ 生产场景 {scene_id}（资产-场景绑定 fail-closed）",
                asset.scene_id
            ));
        }
        let eval = match slab_evaluate(&asset, &spv_slab) {
            Ok(v) => v,
            Err(e) => dev_env_or_fail("slab_device_eval", &e),
        };
        let arm_r = slab_arm_r(&eval, "device");
        let n_slab = slab_apply(&mut scene, &asset, &arm_r);
        eprintln!(
            "{GTAG}: [hzb] slab 接线 arm=device slots=16 mapped_mats={} slab_tris={} parity_p100={:.6e} eval_ms={:.3} abi={}",
            asset.material_slots.len(),
            n_slab,
            eval.parity_p100,
            eval.eval_ms,
            asset.abi_digest,
        );
        (Some((asset, eval, n_slab)), Some(arm_r))
    };

    // ③.6 纹理采样生产接线（main ③.6 逐字同律 + texmeta mod × R_slot 预调制;
    //     --full 闭集 ⇒ textures 恒开）。
    let (tex_report, tex_premod_slots): (Option<(G31TexAssets, G31TexProbeReport)>, usize) = {
        let mut assets = match g31_tex_load(&scene, Path::new(&gltf_path), &tri_uv) {
            Ok(a) => a,
            Err(e) => dev_env_or_fail("texture_assets", &e),
        };
        let probes = g31_tex_probes(assets.slots.len());
        let report = match g31_tex_probe_evaluate(&assets, &probes, &spv_texture_probe) {
            Ok(r) => r,
            Err(e) => dev_env_or_fail("texture_probe", &e),
        };
        if !report.ssbo_bitexact {
            fail(&format!(
                "B4 probe SSBO 腿 device vs host 非位级一致（p100={:.6e} > 0.0 硬门）",
                report.ssbo_p100
            ));
        }
        if !report.ssbo_double_run_bitexact {
            fail("B4 probe SSBO 腿 device 双跑非位级一致（确定性门红）");
        }
        if report.sampler_max_lsb > 1 {
            fail(&format!(
                "B4 sampler 腿硬件采样 vs host 参考 max_lsb={} > 1（结构容差界红）",
                report.sampler_max_lsb
            ));
        }
        if report.nonconstant_slots == 0 {
            fail("B4 映射纹理探针输出全常量（空接线冒充即红,fail-closed）");
        }
        let premod = if let (Some(asset_eval), Some(arm_r)) = (slab_report.as_ref(), slab_arm.as_ref()) {
            g34_slab_premod_texmeta(&mut assets, &asset_eval.0, arm_r)
        } else {
            0
        };
        eprintln!(
            "{GTAG}: [hzb] B4 纹理接线 mapped={} tex_tris={} atlas={}x{} probes={} ssbo_p100={:.6e}（位级={} 双跑={}） sampler_max_lsb={} nonconstant_slots={} eval_ms={:.3} slab_premod_slots={}",
            assets.slots.len(),
            assets.tex_tris,
            assets.atlas_w,
            assets.atlas_h,
            report.probe_count,
            report.ssbo_p100,
            report.ssbo_bitexact,
            report.ssbo_double_run_bitexact,
            report.sampler_max_lsb,
            report.nonconstant_slots,
            report.eval_ms,
            premod,
        );
        (Some((assets, report)), premod)
    };
    let eps = scene_eps(&scene.positions);
    eprintln!(
        "{GTAG}: [hzb] 装配 scene={scene_id} tris={} quads={} points={} nodes={} output={out_w}x{out_h} eps={eps:.6} features=[tex=true slab=true dyn=true hzb=true]",
        scene.tri_count,
        scene.quads.len(),
        scene.points.len(),
        hzb_groups.len(),
    );

    // ③.7 环境面（evidence environment 块;三态裁决同律——无设备即 skip）。
    let caps = match rurix_rt::render_exec::probe_device_caps() {
        Ok(c) => c,
        Err(e) => dev_env_or_fail("device_caps", &e),
    };

    // ④ 真窗口 present 会话（main ④ 同律;--hzb on 闭集已拒 headless ⇒ 窗口
    //    必建,创建失败走三态 skip）。
    let mut window = match vk::ExternalImagePresent::create(
        out_w,
        out_h,
        "rurix g34 unified lane + hzb (bistro-interior 1080p;G34-2 HZB 剔除四特性同开;ESC 退出)",
        !hidden,
    ) {
        Ok(w) => w,
        Err(e) => dev_env_or_fail("window_present", &e),
    };
    let bgra = window.channel_order() == "bgra8_unorm";
    eprintln!(
        "{GTAG}: [hzb] 窗口就绪 {}x{} channel_order={} visible={}",
        window.extent().0,
        window.extent().1,
        window.channel_order(),
        !hidden
    );

    // ⑤ 初态（相机 = 契约位姿;auto-move 轨迹基位;fork B 动态轨迹原点）。
    let cam0 = G34Camera::from_spec(&scene.camera);
    let mut cam = cam0;
    let ev100 = f64::from(scene.ev100);
    let jitter_base = (seed % JITTER_WINDOW_MOD) as u32;
    let dyn_origin = dyn_trajectory_origin(&scene.camera);

    // ⑥ era 循环状态（main ⑦ 同律 + HZB 决策计数/probe 对拍面）。
    let total = warmup + frames;
    let mut fi = 0u32;
    let mut exit_reason = "frames_done";
    let mut resize_eras = 0u32;
    let mut render_ms: Vec<f64> = Vec::new();
    let mut present_ms: Vec<f64> = Vec::new();
    let mut digest_ms: Vec<f64> = Vec::new();
    let mut encode_gpu_ms: Vec<f64> = Vec::new();
    let mut scene_gpu_ms: Vec<f64> = Vec::new();
    let mut prod_gpu_ms: Vec<f64> = Vec::new();
    let mut hzb_aux_ms: Vec<f64> = Vec::new();
    let mut hzb_closure_ms: Vec<f64> = Vec::new();
    let mut hzb_host_ms: Vec<f64> = Vec::new();
    let mut digest_seq: Vec<String> = Vec::new();
    let mut ev100_seq: Vec<f64> = Vec::new();
    let mut pose_seq: Vec<[f64; 5]> = Vec::new();
    let mut render_digest = String::new();
    let mut presented_digest = String::new();
    let mut real_render_seconds: f64 = 0.0;
    let mut real_frames: u64 = 0;
    let mut verify_recs: Vec<DynVerifyFrame> = Vec::new();
    // HZB 决策计数（逐帧恒记账;ms 序列 = post-warmup 测量口径）。
    let mut hzb_tested: u64 = 0;
    let mut hzb_occluded: u64 = 0;
    let mut hzb_offscreen: u64 = 0;
    let mut hzb_retested: u64 = 0;
    let mut hzb_flipped: u64 = 0;
    let mut hzb_visible_sum: u64 = 0;
    let mut hzb_closure_frames: u64 = 0;
    let mut hzb_closure_submits: u64 = 0;
    let mut hzb_fallbacks: u64 = 0;
    // probe 两帧成对（预备帧回读上帧平铺金字塔 + depth_hz 真深度,本帧 p1 判定
    // host 复算;一次性——G31 B1 同律,post-warmup 首测量帧）。
    let probe_fi = warmup.max(1);
    let mut hzb_pre_data: Option<(Vec<f32>, Vec<f32>)> = None;
    let mut hzb_parity: Option<(G34HzbWiredParity, u32)> = None;
    // mip 拓扑元信息（era 创建期恒先赋值——'eras 为 loop 至少一轮,出口全在
    // 赋值后 ⇒ 定赋值分析成立,免初值）。
    let mut hzb_levels_meta: Vec<(u32, u32)>;
    let mut hzb_flat_offsets_meta: Vec<u32>;
    let mut hzb_meta_json: String;
    'eras: loop {
        let (ew, eh) = window.extent();
        let in_w = ((ew as u64 * u64::from(tier)) / 100).max(1) as u32;
        let in_h = ((eh as u64 * u64::from(tier)) / 100).max(1) as u32;
        // ── era 资产（--full ⇒ dyn 恒开:静态汤 + 动态立方体尾段合并 SSBO）──
        let assets_dyn = lane_assets_dyn(&scene, in_w, in_h);
        let assets = &assets_dyn.base;
        let dyn_tri_base = assets_dyn.dyn_tri_base;
        let bits = UnifiedLaneBits::load(
            &spv_scene,
            &spv_mv,
            &spv_resample,
            &spv_resolve,
            in_w,
            in_h,
            ew,
            eh,
            false,
        );
        // （统一 mega scene pass 不进 HZB 车道——bits.spv_scene 仅描述组装配期
        //   借用后随 pass 对象弃置,零注入零消费;primary/shade 注入面由
        //   G34HzbBits::load 承载。）
        let enc_words = load_spv(&spv_encode);
        let (ex, ey, _) = spv_local_size(&enc_words);
        let enc_dispatch = [ew.div_ceil(ex), eh.div_ceil(ey), 1];
        let enc_spv_bytes: Vec<u8> = enc_words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let enc_params = aces13_device_encode_params(ew, eh, bgra);
        let enc_params_bytes = bytes_f32(&enc_params);
        // 纹理侧表（三分区总三角数 = 静态 + 动态段;动态段 tritex −1 常量面）。
        let total_tris = dyn_tri_base + assets_dyn.dyn_tris.len() / 9;
        let side = {
            let (t, _) = tex_report.as_ref().unwrap_or_else(|| {
                fail("HZB 面缺 B4 纹理报告（--full 闭集破缺,内部防御性复核）")
            });
            let mut tritex = t.tritex.clone();
            tritex.resize(total_tris, -1.0);
            let mut texuv: Vec<f32> = read_f32(&t.texuv_bytes);
            texuv.resize(total_tris * 6, 0.0);
            G34TexSideTable {
                texuv_bytes: bytes_f32(&texuv),
                texmeta_bytes: t.texmeta_bytes.clone(),
                tritex_bytes: bytes_f32(&tritex),
                atlas_bytes: t.atlas_bytes.clone(),
                linlut_bytes: t.linlut_bytes.clone(),
            }
        };
        // ── HZB era 常量面（SPV 五件 + mip 拓扑/静态参数/平铺初值/inst_base;
        //    inject_primary_shade = true——--full 闭集 textures 恒开,B4 同律
        //    NoContraction 注入）──
        let hz = G34HzbBits::load(
            &spv_hzb_primary,
            &spv_hzb_shade,
            &spv_hzb_reduce,
            &spv_hzb_test,
            &spv_hzb_pack,
            in_w,
            in_h,
            &hzb_groups,
            dyn_tri_base,
            true,
            true,
        );
        hzb_levels_meta = hz.levels.clone();
        hzb_flat_offsets_meta = hz.flat_offsets.clone();
        {
            let dims: Vec<String> = hz
                .levels
                .iter()
                .map(|&(w, h)| format!("[{w},{h}]"))
                .collect();
            hzb_meta_json = format!(
                "{{\"instances\":{},\"static_nodes\":{},\"dyn_tail_slot\":1,\"levels\":{},\"level_dims\":[{}],\"flat_texels\":{},\"conv\":\"standard_z\"}}",
                hzb_groups.len() + 1,
                hzb_groups.len(),
                hz.levels.len(),
                dims.join(","),
                hz.flat_texels
            );
        }
        // ── 描述组（G34Full 27 SSBO 四 pass 解构 + HZB 追加面）──
        let g34_tuple = unified_lane_descs_g34(assets, &bits, &side, in_w, in_h, ew, eh);
        let (resources, passes, barriers, readbacks, ids) = g34_lane_descs_hzb(
            g34_tuple,
            &enc_spv_bytes,
            enc_dispatch,
            &enc_params_bytes,
            &hz,
            hzb_groups.len() + 1,
            in_w,
            in_h,
            ew,
            eh,
        );
        let bar_refs: Vec<&[(u32, TargetState)]> =
            barriers.iter().map(|b| b.as_slice()).collect();
        // ── BLAS 分解 + 双 TLAS（表 0 = 初剔后〔逐帧掩码 + 动态变换 refit〕,
        //    表 1 = 全量〔阴影射线零剔除〕;节点段 = 装配序连续段与合并 SSBO
        //    位级同 buffer,动态立方体局部段 = 尾 BLAS——两 desc 引用同 BLAS
        //    表/实例表,创建期各自建 AS）──
        let mut blas_refs: Vec<&[f32]> = hzb_groups
            .iter()
            .map(|g| {
                let lo = g.tri_offset as usize * 9;
                &assets.tris[lo..lo + g.tri_count as usize * 9]
            })
            .collect();
        blas_refs.push(&assets.tris[dyn_tri_base * 9..]);
        let n_inst = hzb_groups.len() + 1;
        let hzb_insts: Vec<RayQueryInstanceDesc> = (0..n_inst as u32)
            .map(|i| RayQueryInstanceDesc {
                blas: i,
                custom_index: i,
                mask: 0xFF,
                sbt_record_offset: 0,
            })
            .collect();
        let hzb_accel = [
            AccelStructDesc {
                scene: RayQuerySceneDesc {
                    blas_triangles: &blas_refs,
                    instances: &hzb_insts,
                },
                transforms: None,
                updatable_blas: &[], // G34-2 全静态 BLAS（动态面 = TLAS 实例变换 refit）
            },
            AccelStructDesc {
                scene: RayQuerySceneDesc {
                    blas_triangles: &blas_refs,
                    instances: &hzb_insts,
                },
                transforms: None,
                updatable_blas: &[],
            },
        ];
        let mut lane = match G34HzbLane::create(
            &resources,
            &passes,
            &bar_refs,
            &readbacks,
            &hzb_accel,
            ids,
            hzb_groups.clone(),
            true,
            dyn_tri_base,
            hz.levels.len(),
        ) {
            Ok(l) => l,
            Err(e) => dev_env_or_fail("device_lane", &e),
        };
        eprintln!(
            "{GTAG}: [hzb] era 就绪 extent={ew}x{eh} internal={in_w}x{in_h}（车道:g34_unified_primary→g34_unified_shade→g14_mv→tsr×2→display_encode→test_p1→reduce×{}+pack×{}→test_p2;instances={}〔静态节点 {} + 动态尾槽 1〕 mips={} flat_texels={};resize_eras={resize_eras}）",
            hz.levels.len() - 1,
            hz.levels.len(),
            n_inst,
            hzb_groups.len(),
            hz.levels.len(),
            hz.flat_texels,
        );
        let mut resized = false;
        let mut era_first = true;
        while fi < total {
            // ── 窗口事件面（main 同律）──
            {
                let input = window.poll_input();
                if input.close_requested {
                    exit_reason = "user_close";
                    break 'eras;
                }
                if input.minimized {
                    std::thread::sleep(std::time::Duration::from_millis(8));
                    continue;
                }
                if let Some((nw, nh)) = input.resize_pending {
                    if (nw, nh) != (ew, eh) {
                        if let Err(e) = window.resize(nw, nh) {
                            fail(&format!("窗口 resize {nw}x{nh}: {e}"));
                        }
                        if window.extent() != (ew, eh) {
                            resized = true;
                            resize_eras += 1;
                            break;
                        }
                    }
                }
            }
            // ── 相机（auto-move 确定性轨迹;闭集裁决保证恒有轨迹面）──
            let spec = {
                let (yaw, pitch, eye) = g34_auto_move_pose(&auto_move, &cam0, fi, total);
                cam.yaw = yaw;
                cam.pitch = pitch;
                cam.eye = eye;
                cam.spec()
            };
            let vp = build_vp(&spec, in_w, in_h);
            let inv_vp = vp.inverse().unwrap_or_else(|| fail("view-proj 必须可逆"));
            let exposure = 2.0f32.powf(-(ev100 as f32));
            let j = [
                halton(jitter_base + fi + 1, 2) - 0.5,
                halton(jitter_base + fi + 1, 3) - 0.5,
            ];
            let vp_j = jittered_vp(&vp, j, in_w, in_h);
            // ── fork B 逐帧实例变换 + 60 f32 场景参数（dyn_tri_base 同包）──
            let (pos, yaw) = dyn_trajectory(fi, dyn_origin);
            let xf = dyn_transform_3x4(pos, yaw);
            let scene_params = pack_frame_params_dyn(
                in_w,
                in_h,
                j,
                eps,
                scene.quads.len(),
                scene.points.len(),
                &inv_vp,
                &vp,
                dyn_tri_base,
            );
            let last = fi + 1 == total;
            let verify = fi >= warmup && (fi - warmup) % DYN_VERIFY_EVERY == 0;
            let rb_mode = if last {
                G34Readback::Full
            } else if verify {
                G34Readback::BgraAndScene
            } else {
                G34Readback::Bgra
            };
            let hzb_pre_frame = fi + 1 == probe_fi && hzb_pre_data.is_none();
            let hzb_cmp_frame = fi == probe_fi && hzb_parity.is_none();
            let reset = fi == 0 || era_first;
            era_first = false;
            let t_render = std::time::Instant::now();
            let rec = match lane.frame(
                in_w,
                in_h,
                ew,
                eh,
                &scene_params,
                j,
                &vp,
                &vp_j,
                exposure,
                reset,
                rb_mode,
                hzb_pre_frame,
                Some(xf),
            ) {
                Ok(r) => r,
                Err(e) => fail(&format!("帧 {fi} HZB 车道: {e}")),
            };
            let render_el = t_render.elapsed().as_secs_f64() * 1000.0;
            if rec.validation_error_count != 0 {
                fail(&format!(
                    "帧 {fi} validation ERROR 计数 {} ≠ 0",
                    rec.validation_error_count
                ));
            }
            if rec.leaked_object_count != 0 || rec.leaked_allocation_count != 0 {
                fail(&format!(
                    "帧 {fi} leak 账本非零 object={} allocation={}（资源无泄漏机核判红）",
                    rec.leaked_object_count, rec.leaked_allocation_count
                ));
            }

            // ── HZB 逐帧决策面记账 + probe 两帧成对接线态对拍 ──
            {
                let hzrec = &rec.hzb;
                hzb_tested += u64::from(hzrec.tested_p1);
                hzb_occluded += u64::from(hzrec.occluded_p1);
                hzb_offscreen += u64::from(hzrec.offscreen);
                hzb_retested += u64::from(hzrec.retested_p2);
                hzb_flipped += u64::from(hzrec.flipped_p2);
                hzb_visible_sum += u64::from(hzrec.visible_final);
                if hzrec.closure_extra_submits > 0 || hzrec.closure_full_fallback {
                    hzb_closure_frames += 1;
                    hzb_closure_submits += u64::from(hzrec.closure_extra_submits);
                    if hzrec.closure_full_fallback {
                        hzb_fallbacks += 1;
                    }
                }
                if fi >= warmup {
                    hzb_aux_ms.push(hzrec.hzb_gpu_ns / 1e6);
                    hzb_closure_ms.push(hzrec.closure_extra_gpu_ns / 1e6);
                    hzb_host_ms.push(hzrec.host_ms);
                }
                if hzb_pre_frame {
                    let (Some(d), Some(f)) =
                        (hzrec.probe_depth.as_ref(), hzrec.probe_flat.as_ref())
                    else {
                        fail(&format!("帧 {fi} HZB probe 预备回读缺失"));
                    };
                    hzb_pre_data = Some((d.clone(), f.clone()));
                }
                if hzb_cmp_frame {
                    let Some((d, f)) = hzb_pre_data.as_ref() else {
                        fail(&format!("帧 {fi} HZB probe 预备数据缺失（对拍面不完整判红）"));
                    };
                    let wp = match g34_hzb_wired_parity(
                        d,
                        f,
                        in_w,
                        in_h,
                        &hzb_levels_meta,
                        &hzb_flat_offsets_meta,
                        &hzrec.rects_p1,
                        &hzrec.verdicts_p1,
                    ) {
                        Ok(w) => w,
                        Err(e) => fail(&format!("帧 {fi} HZB 接线态对拍复算: {e}")),
                    };
                    if !wp.mips_bitexact {
                        // 现场取证 dump（depth/flat 原字节;离线归因面,仅红路径）。
                        let _ = std::fs::create_dir_all(".tmp/g34_gates/hzb");
                        let dump = |name: &str, v: &[f32]| {
                            let b: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
                            let _ = std::fs::write(format!(".tmp/g34_gates/hzb/{name}"), &b);
                        };
                        dump("probe_depth.bin", d);
                        dump("probe_flat.bin", f);
                        eprintln!(
                            "{GTAG}: [hzb] probe 现场 dump → .tmp/g34_gates/hzb/probe_{{depth,flat}}.bin"
                        );
                        fail(&format!(
                            "帧 {fi} HZB 接线态对拍：车道平铺金字塔 vs host HzbPyramid::build 非逐级位级全等（①零容差破坏）"
                        ));
                    }
                    if !wp.verdict_equal {
                        fail(&format!(
                            "帧 {fi} HZB 接线态对拍：p1 判定序列与 host test_rect 非逐 rect 全等（②破坏）"
                        ));
                    }
                    if wp.false_positives != 0 {
                        fail(&format!(
                            "帧 {fi} HZB 接线态对拍：假阳性 {}（③硬不变量破坏,exact_rect_occluded 独立复核检出）",
                            wp.false_positives
                        ));
                    }
                    eprintln!(
                        "{GTAG}: [hzb] 帧 {} 接线态对拍 mips={} 位级全等 + p1 判定 {} rect 逐字节全等 + 零假阳性（剔除 {}）+ digest {}",
                        fi + 1,
                        wp.mips,
                        wp.n_rects,
                        wp.occluded,
                        &wp.verdict_digest[..23]
                    );
                    hzb_parity = Some((wp, fi));
                }
            }

            // ── fork B 动态实例位置核验（main 同律：A4 范式 host 投影 vs scene
            //    color 纯绿谱检测,fail-closed——动态尾槽恒可见不参剔的核验承载面）──
            if verify {
                let scene_color = rec
                    .scene_color
                    .as_ref()
                    .unwrap_or_else(|| fail("帧核验面缺 scene color 回读（内部破缺）"));
                let obs = dyn_detect(scene_color, in_w, in_h);
                let pred_c = dyn_project(&vp_j, pos, in_w, in_h)
                    .unwrap_or_else(|| fail("轨迹点投影在相机背面（轨迹规格破缺）"));
                let mut pred_aabb = [
                    f64::INFINITY,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    f64::NEG_INFINITY,
                ];
                for k in 0..8 {
                    let lp = [
                        if k & 1 == 0 { -DYN_CUBE_HALF } else { DYN_CUBE_HALF },
                        if k & 2 == 0 { -DYN_CUBE_HALF } else { DYN_CUBE_HALF },
                        if k & 4 == 0 { -DYN_CUBE_HALF } else { DYN_CUBE_HALF },
                    ];
                    let wp = [
                        xf[0] * lp[0] + xf[1] * lp[1] + xf[2] * lp[2] + xf[3],
                        xf[4] * lp[0] + xf[5] * lp[1] + xf[6] * lp[2] + xf[7],
                        xf[8] * lp[0] + xf[9] * lp[1] + xf[10] * lp[2] + xf[11],
                    ];
                    let (u, v) = dyn_project(&vp_j, wp, in_w, in_h)
                        .unwrap_or_else(|| fail("角点投影在相机背面（轨迹规格破缺）"));
                    pred_aabb[0] = pred_aabb[0].min(u);
                    pred_aabb[1] = pred_aabb[1].min(v);
                    pred_aabb[2] = pred_aabb[2].max(u);
                    pred_aabb[3] = pred_aabb[3].max(v);
                }
                let (obs_px, obs_aabb, obs_count) = match obs {
                    Some((cx, cy, bb, n)) => ([cx, cy], bb, n),
                    None => ([f64::NAN; 2], [f64::NAN; 4], 0),
                };
                let centroid_delta = if obs_count > 0 {
                    ((obs_px[0] - pred_c.0).powi(2) + (obs_px[1] - pred_c.1).powi(2)).sqrt()
                } else {
                    f64::INFINITY
                };
                let aabb_delta = if obs_count > 0 {
                    (obs_aabb[0] - pred_aabb[0])
                        .abs()
                        .max((obs_aabb[1] - pred_aabb[1]).abs())
                        .max((obs_aabb[2] - pred_aabb[2]).abs())
                        .max((obs_aabb[3] - pred_aabb[3]).abs())
                } else {
                    f64::INFINITY
                };
                let pred_area = (pred_aabb[2] - pred_aabb[0]).max(0.0)
                    * (pred_aabb[3] - pred_aabb[1]).max(0.0);
                let min_count = 200.0f64.max(DYN_MIN_COUNT_AREA_RATIO * pred_area) as usize;
                // 质心容差域界式——main --full 车道同律（门窗标定域 √A ≤100px
                // 维持绝对 2.5px 逐字,域外近大目标按轮廓直径 5% 界模型偏差;
                // 注释详见 main 同名块）。
                let pred_diag = pred_area.sqrt();
                let tol_centroid = if pred_diag <= 100.0 {
                    DYN_TOL_CENTROID_PX
                } else {
                    DYN_TOL_CENTROID_PX.max(0.05 * pred_diag)
                };
                let pass = obs_count >= min_count
                    && centroid_delta <= tol_centroid
                    && aabb_delta <= DYN_TOL_AABB_PX;
                verify_recs.push(DynVerifyFrame {
                    frame: fi,
                    transform: xf,
                    pred_px: [pred_c.0, pred_c.1],
                    pred_aabb,
                    obs_px,
                    obs_aabb,
                    obs_count,
                    centroid_delta_px: centroid_delta,
                    aabb_delta_px: aabb_delta,
                    pass,
                });
                if !pass {
                    fail(&format!(
                        "帧 {fi} 动态实例位置核验 fail（obs_count={obs_count}（min {min_count}）centroid_Δ={centroid_delta:.3}px（tol {tol_centroid:.3}）aabb_Δ={aabb_delta:.3}px）"
                    ));
                }
            }

            // ── present（device 已编码;host 仅拷贝/present）──
            let mut pres_el = 0.0f64;
            {
                let Some(px) = rec.bgra8.as_ref() else {
                    fail(&format!("帧 {fi} 窗口面缺 BGRA8 回读"));
                };
                let t_one = std::time::Instant::now();
                if let Err(e) = window.present_rgba8(px) {
                    fail(&format!("帧 {fi} 窗口 present: {e}"));
                }
                let el = t_one.elapsed().as_secs_f64() * 1000.0;
                pres_el += el;
                if fi >= warmup {
                    present_ms.push(el);
                }
            }

            // ── digest（auto-move 逐帧序列;双臂位级对拍机核门承载面）──
            let t_dig = std::time::Instant::now();
            {
                let Some(px) = rec.bgra8.as_ref() else {
                    fail(&format!("帧 {fi} auto-move 面缺 BGRA8 回读"));
                };
                digest_seq.push(g34_bgra_digest(ew, eh, px));
                ev100_seq.push(ev100);
                pose_seq.push([
                    f64::from(cam.eye[0]),
                    f64::from(cam.eye[1]),
                    f64::from(cam.eye[2]),
                    f64::from(cam.yaw),
                    f64::from(cam.pitch),
                ]);
            }
            if last {
                let Some(px) = rec.bgra8.as_ref() else {
                    fail("末帧缺 BGRA8 回读");
                };
                presented_digest = g34_bgra_digest(ew, eh, px);
                let Some(out_data) = rec.out_color.as_ref() else {
                    fail("末帧缺 f32 out_color 回读");
                };
                if !out_data.iter().all(|v| v.is_finite()) {
                    fail("末帧 TSR 输出非有限");
                }
                render_digest = frame_content_digest(ew, eh, 3, out_data);
            }
            let dig_el = t_dig.elapsed().as_secs_f64() * 1000.0;

            if fi >= warmup {
                render_ms.push(render_el);
                digest_ms.push(dig_el);
                encode_gpu_ms.push(rec.encode_gpu_ns / 1e6);
                scene_gpu_ms.push(rec.scene_gpu_ns / 1e6);
                prod_gpu_ms.push(
                    (rec.scene_gpu_ns
                        + rec.mv_gpu_ns
                        + rec.resample_gpu_ns
                        + rec.resolve_gpu_ns
                        + rec.encode_gpu_ns)
                        / 1e6,
                );
                real_frames += 1;
                real_render_seconds += render_el / 1000.0;
            }
            if fi == 0 || (fi + 1) % 20 == 0 || fi + 1 == total {
                eprintln!(
                    "{GTAG}: [hzb] 帧 {}/{total} render={render_el:.3}ms(gpu_scene={:.3}ms gpu_hzb={:.4}ms gpu_encode={:.3}ms) tested={} occluded={} offscreen={} flip={} closure_extra={}{} visible={} present={pres_el:.3}ms",
                    fi + 1,
                    rec.scene_gpu_ns / 1e6,
                    rec.hzb.hzb_gpu_ns / 1e6,
                    rec.encode_gpu_ns / 1e6,
                    rec.hzb.tested_p1,
                    rec.hzb.occluded_p1,
                    rec.hzb.offscreen,
                    rec.hzb.flipped_p2,
                    rec.hzb.closure_extra_submits,
                    if rec.hzb.closure_full_fallback { "(全掩码兜底)" } else { "" },
                    rec.hzb.visible_final,
                );
            }
            fi += 1;
        }
        if fi >= total || !resized {
            break 'eras;
        }
    }

    // ⑦ 多口径稳态统计 + evidence（main 同律 + hzb 块;证据保全先于判红）。
    let frames_done = fi;
    let dyn_all_pass = !verify_recs.is_empty() && verify_recs.iter().all(|v| v.pass);
    let (r_mean, _, r_cv, r_min, r_max) = g34_stats(&render_ms);
    let (p_mean, _, p_cv, p_min, p_max) = if present_ms.iter().all(|v| *v == 0.0) {
        (0.0, 0.0, 0.0, 0.0, 0.0)
    } else {
        g34_stats(&present_ms)
    };
    let (eg_mean, _, _, _, _) = g34_stats(&encode_gpu_ms);
    let (sg_mean, _, _, _, _) = g34_stats(&scene_gpu_ms);
    let (pg_mean, _, _, _, _) = g34_stats(&prod_gpu_ms);
    let (hza_mean, _, _, _, _) = g34_stats(&hzb_aux_ms);
    let (hzc_mean, _, _, _, _) = g34_stats(&hzb_closure_ms);
    let (hzh_mean, _, _, _, _) = g34_stats(&hzb_host_ms);
    let (dg_mean, _, _, _, _) = g34_stats(&digest_ms);
    let encode_host_ms = 0.0f64;
    let overhead_mean = encode_host_ms + p_mean;
    let visible_mean = if frames_done > 0 {
        hzb_visible_sum as f64 / f64::from(frames_done)
    } else {
        0.0
    };
    let real_render_fps = if real_render_seconds > 0.0 {
        real_frames as f64 / real_render_seconds
    } else {
        0.0
    };
    let counts = window.counts();
    let (fw, fh) = window.extent();
    let window_json = format!(
        "{{\"visible\":{},\"channel_order\":{},\"extent\":{{\"w\":{fw},\"h\":{fh}}},\"frames_presented\":{},\"swapchain_rebuilds\":{}}}",
        !hidden,
        jstr(if bgra { "bgra8_unorm" } else { "rgba8_unorm" }),
        counts.frames_presented,
        counts.swapchain_rebuilds
    );
    let encode_spv_json = format!(
        "{{\"path\":{},\"sha256\":{}}}",
        jstr(&spv_encode.replace('\\', "/")),
        jstr(&g34_file_sha(&spv_encode).unwrap_or_else(|e| fail(&e)))
    );
    let hzb_spv_sha = |p: &str| {
        format!(
            "{{\"path\":{},\"sha256\":{}}}",
            jstr(&p.replace('\\', "/")),
            jstr(&g34_file_sha(p).unwrap_or_else(|e| fail(&e)))
        )
    };

    // ── features/textures/slab/dyn 块（main 同面;hzb 位追加）──
    let features_json =
        "{\"textures\":true,\"slab\":true,\"dyn\":true,\"full\":true,\"static_camera\":false,\"hzb\":true}"
            .to_owned();
    let textures_json = if let Some((t, rep)) = tex_report.as_ref() {
        let c = &t.census;
        format!(
            "{{\"census\":{{\"materials_total\":{},\"with_base_color_texture\":{},\"with_normal_texture\":{},\"with_metallic_roughness_texture\":{},\"primitives_total\":{},\"primitives_with_texcoord0\":{},\"primitives_with_tangent\":{}}},\"mapping_law\":\"逐材质三角数降序 top-12（并列时 material_index 升序;其余走常量面 0-byte）\",\"mapped_materials\":{},\"tex_tris\":{},\"atlas\":{{\"width\":{},\"height\":{},\"tile\":2048,\"format\":\"u32_packed_rgba8\",\"digest\":{}}},\"linlut_digest\":{},\"slab_premod_slots\":{},\"probe\":{{\"probe_count\":{},\"eval_ms\":{:.6},\"ssbo\":{{\"p100\":{:.15e},\"bitexact\":{},\"double_run_bitexact\":{},\"device_digest\":{},\"host_digest\":{}}},\"sampler_leg\":{{\"max_lsb_diff\":{},\"bound_lsb\":1,\"bitexact\":{}}}}},\"spv_shade\":{{\"path\":{},\"sha256\":{},\"no_contraction_injected\":true}}}}",
            c.materials_total,
            c.with_base_color_texture,
            c.with_normal_texture,
            c.with_metallic_roughness_texture,
            c.primitives_total,
            c.primitives_with_texcoord0,
            c.primitives_with_tangent,
            t.slots.len(),
            t.tex_tris,
            t.atlas_w,
            t.atlas_h,
            jstr(&t.atlas_digest),
            jstr(&t.linlut_digest),
            tex_premod_slots,
            rep.probe_count,
            rep.eval_ms,
            rep.ssbo_p100,
            rep.ssbo_bitexact,
            rep.ssbo_double_run_bitexact,
            jstr(&rep.ssbo_device_digest),
            jstr(&rep.ssbo_host_digest),
            rep.sampler_max_lsb,
            rep.sampler_bitexact,
            jstr(&spv_hzb_shade.replace('\\', "/")),
            jstr(&g34_file_sha(&spv_hzb_shade).unwrap_or_else(|e| fail(&e))),
        )
    } else {
        "null".to_owned()
    };
    let slab_json = if let Some((asset, eval, n_slab)) = slab_report.as_ref() {
        format!(
            "{{\"asset_path\":{},\"abi_digest\":{},\"mapped_materials\":{},\"slab_tris\":{},\"parity_p100\":{:.15e},\"eval_ms\":{:.6},\"arm\":\"device\",\"tex_premod_slots\":{},\"device_digest\":{},\"host_digest\":{}}}",
            jstr(&asset.path.replace('\\', "/")),
            jstr(&asset.abi_digest),
            asset.material_slots.len(),
            n_slab,
            eval.parity_p100,
            eval.eval_ms,
            tex_premod_slots,
            jstr(&eval.device_digest),
            jstr(&eval.host_digest),
        )
    } else {
        "null".to_owned()
    };
    let dyn_json = {
        let mut frames_json = String::new();
        for (k, v) in verify_recs.iter().enumerate() {
            if k > 0 {
                frames_json.push(',');
            }
            frames_json.push_str(&format!(
                "{{\"frame\":{},\"pred_px\":[{:.4},{:.4}],\"pred_aabb\":[{:.4},{:.4},{:.4},{:.4}],\"obs_px\":[{:.4},{:.4}],\"obs_aabb\":[{:.4},{:.4},{:.4},{:.4}],\"obs_count\":{},\"centroid_delta_px\":{:.6},\"aabb_delta_px\":{:.6},\"pass\":{}}}",
                v.frame,
                v.pred_px[0], v.pred_px[1],
                v.pred_aabb[0], v.pred_aabb[1], v.pred_aabb[2], v.pred_aabb[3],
                v.obs_px[0], v.obs_px[1],
                v.obs_aabb[0], v.obs_aabb[1], v.obs_aabb[2], v.obs_aabb[3],
                v.obs_count,
                v.centroid_delta_px,
                v.aabb_delta_px,
                v.pass,
            ));
        }
        format!(
            "{{\"dyn_tris\":12,\"dyn_tri_base\":{},\"action\":\"refit\",\"always_visible\":true,\"verify_every\":{},\"tol_centroid_px\":{:.3},\"tol_aabb_px\":{:.3},\"min_count_area_ratio\":{:.4},\"verify_frames\":[{}],\"verify_count\":{},\"all_pass\":{}}}",
            scene.indices.len(),
            DYN_VERIFY_EVERY,
            DYN_TOL_CENTROID_PX,
            DYN_TOL_AABB_PX,
            DYN_MIN_COUNT_AREA_RATIO,
            frames_json,
            verify_recs.len(),
            dyn_all_pass,
        )
    };
    // hzb 块（拓扑元信息 + 剔除计数合计 + 两阶段闭环 + 接线态对拍三面）。
    let parity_json = if let Some((wp, pf)) = hzb_parity.as_ref() {
        format!(
            "{{\"probe_frame\":{},\"mips\":{},\"n_rects\":{},\"mips_bitexact\":{},\"verdict_equal\":{},\"false_positives\":{},\"occluded\":{},\"pyramid_digest\":{},\"host_pyramid_digest\":{},\"pyramid_digest_equal_host\":{},\"verdict_digest\":{},\"host_verdict_digest\":{},\"verdict_digest_equal_host\":{}}}",
            pf + 1,
            wp.mips,
            wp.n_rects,
            wp.mips_bitexact,
            wp.verdict_equal,
            wp.false_positives,
            wp.occluded,
            jstr(&wp.pyramid_digest),
            jstr(&wp.host_pyramid_digest),
            wp.pyramid_digest == wp.host_pyramid_digest,
            jstr(&wp.verdict_digest),
            jstr(&wp.host_verdict_digest),
            wp.verdict_digest == wp.host_verdict_digest,
        )
    } else {
        "null".to_owned()
    };
    let hzb_json = format!(
        "{{\"mode\":\"on\",\"all_visible_arm\":{all_visible_arm},\"instances\":{},\"mips\":{},\"meta\":{hzb_meta_json},\"tested\":{hzb_tested},\"occluded_p1\":{hzb_occluded},\"offscreen\":{hzb_offscreen},\"retested_p2\":{hzb_retested},\"flipped_p2\":{hzb_flipped},\"closure_frames\":{hzb_closure_frames},\"closure_extra_submits\":{hzb_closure_submits},\"closure_full_fallback_frames\":{hzb_fallbacks},\"closure_max_iters\":{G34HZB_CLOSURE_MAX},\"visible_mean\":{visible_mean:.6},\"parity\":{parity_json}}}",
        hzb_groups.len() + 1,
        hzb_levels_meta.len(),
    );

    let mut ev = String::with_capacity(16384);
    ev.push('{');
    ev.push_str(&format!("\"schema\":{},", jstr(G34HZB_SCHEMA)));
    ev.push_str(&format!("\"gate\":{},", jstr(G34HZB_GATE)));
    ev.push_str(&format!("\"scene\":{},", jstr(scene_id)));
    ev.push_str(&format!("\"tier\":{tier},\"backend\":\"tsr_device\","));
    ev.push_str(&format!("\"trajectory\":{},", jstr(&auto_move)));
    ev.push_str(&format!("\"frames\":{frames},\"warmup\":{warmup},"));
    ev.push_str(&format!("\"frames_completed\":{frames_done},"));
    ev.push_str(&format!("\"exit_reason\":{},", jstr(exit_reason)));
    ev.push_str(&format!("\"resize_eras\":{resize_eras},"));
    ev.push_str(&format!("\"resolution\":{{\"w\":{out_w},\"h\":{out_h}}},"));
    ev.push_str(&format!(
        "\"internal_resolution\":{{\"w\":{},\"h\":{}}},",
        (out_w as u64 * u64::from(tier) / 100).max(1),
        (out_h as u64 * u64::from(tier) / 100).max(1)
    ));
    ev.push_str(&format!("\"real_render_frame_ms\":{r_mean:.6},"));
    ev.push_str(&format!("\"real_render_fps\":{real_render_fps:.6},"));
    ev.push_str(&format!("\"present_frame_ms\":{p_mean:.6},"));
    ev.push_str(&format!("\"present_overhead_ms\":{overhead_mean:.6},"));
    ev.push_str(&format!("\"encode_frame_ms\":{encode_host_ms:.6},"));
    ev.push_str(&format!("\"encode_gpu_ms\":{eg_mean:.6},"));
    ev.push_str(&format!("\"digest_frame_ms\":{dg_mean:.6},"));
    ev.push_str(&format!("\"render_digest\":{},", jstr(&render_digest)));
    ev.push_str(&format!("\"digest\":{},", jstr(&presented_digest)));
    ev.push_str("\"digest_seq\":[");
    for (k, d) in digest_seq.iter().enumerate() {
        if k > 0 {
            ev.push(',');
        }
        ev.push_str(&jstr(d));
    }
    ev.push_str("],");
    ev.push_str("\"ev100_seq\":[");
    for (k, v) in ev100_seq.iter().enumerate() {
        if k > 0 {
            ev.push(',');
        }
        ev.push_str(&format!("{v}"));
    }
    ev.push_str("],");
    ev.push_str("\"camera_poses\":[");
    for (k, p) in pose_seq.iter().enumerate() {
        if k > 0 {
            ev.push(',');
        }
        ev.push_str(&format!("[{},{},{},{},{}]", p[0], p[1], p[2], p[3], p[4]));
    }
    ev.push_str("],");
    ev.push_str("\"ev100_ramp\":null,");
    ev.push_str("\"headless\":false,");
    ev.push_str(&format!("\"window\":{window_json},"));
    ev.push_str("\"contracts\":{\"production\":");
    ev.push_str(&format!(
        "{{\"path\":{},\"digest\":{}}},",
        jstr(&contract_path.replace('\\', "/")),
        jstr(&contract.digest)
    ));
    ev.push_str(&g10_fragment);
    ev.push_str(&format!(",\"encode_spv\":{encode_spv_json}"));
    ev.push_str(&format!(
        ",\"hzb_spv\":{{\"primary\":{},\"shade\":{},\"pack\":{},\"reduce\":{},\"test\":{}}}",
        hzb_spv_sha(&spv_hzb_primary),
        hzb_spv_sha(&spv_hzb_shade),
        hzb_spv_sha(&spv_hzb_pack),
        hzb_spv_sha(&spv_hzb_reduce),
        hzb_spv_sha(&spv_hzb_test)
    ));
    ev.push_str("},");
    ev.push_str("\"render_includes_forced_readback\":true,");
    ev.push_str(&format!(
        "\"spv\":{},",
        unified_provenance_json(&spv_hzb_primary, &spv_mv, &spv_resample, &spv_resolve)
    ));
    ev.push_str(&format!("\"features\":{features_json},"));
    ev.push_str(&format!("\"textures\":{textures_json},"));
    ev.push_str(&format!("\"slab\":{slab_json},"));
    ev.push_str(&format!("\"dyn\":{dyn_json},"));
    ev.push_str(&format!("\"hzb\":{hzb_json},"));
    ev.push_str("\"host_parity\":null,");
    ev.push_str(&format!(
        "\"stats\":{{\"render_cv\":{r_cv:.6},\"render_min_ms\":{r_min:.6},\"render_max_ms\":{r_max:.6},\"scene_gpu_ms\":{sg_mean:.6},\"prod_gpu_ms\":{pg_mean:.6},\"hzb_aux_gpu_ms\":{hza_mean:.6},\"closure_extra_gpu_ms\":{hzc_mean:.6},\"hzb_host_ms\":{hzh_mean:.6},\"encode_gpu_ms\":{eg_mean:.6},\"present_cv\":{p_cv:.6},\"present_min_ms\":{p_min:.6},\"present_max_ms\":{p_max:.6}}},",
    ));
    ev.push_str(&format!(
        "\"environment\":{{\"gpu\":{},\"os\":{},\"validation\":{}}},",
        jstr(&caps.device_name),
        jstr(std::env::consts::OS),
        jstr(
            if std::env::var("RURIX_VK_VALIDATION").ok().as_deref() == Some("1") {
                "on"
            } else {
                "off"
            }
        ),
    ));
    ev.push_str(&format!(
        "\"notes\":{}",
        jstr("G34 全特性合流 G34-2 HZB 接统一车道：HZB 两阶段遮挡剔除×纹理×slab×动态实例四特性同开真窗口生产车道（bistro-interior 1080p swapchain present）。剔除对象粒度 = TLAS 实例（bistro 逐 mesh 节点 BLAS 分解 + 动态实例尾槽恒可见——A4 核验对象如实登记不参剔;tris/mats SSBO 与 G34Full 双实例面位级同 buffer,g34_unified_primary 经 inst_base 前缀和表映回全局下标）;消费点 = 主射线 TLAS 实例 mask（表 0 = 初剔后逐帧掩码 + 动态变换 refit,表 1 = 全量 0xFF + 动态变换 refit——阴影射线零剔除保遮挡物阴影正确性,RXS-0297 单 TLAS 签名纪律拆 primary/shade 双 pass;双表同帧单提交 = render_exec G34-2 加性 execute_with_frame_update_dual_tlas 第二更新位）;帧内金字塔轮换（pass 序 = primary→shade→mv→tsr×2→encode→test_p1〔全实例 rect vs 上帧平铺金字塔 = 上帧金字塔初剔〕→g27_hzb_reduce×(L−1)+g31_hzb_pack×L〔本帧重建覆写,g27 两 kernel + g31 pack glue 0-byte 冻结消费〕→test_p2〔上帧被剔集 vs 本帧金字塔 = 本帧重建重测〕;剔除链深度域 = 真 ZO NDC——g34_unified_shade ④b 段 out_depth_hz vp 行 2/3 另算,U_SCENE_DEPTH 生产字面供 MV/TSR 两路并存互不染指）+ 两阶段闭环第二段（RFC-0044 §5.8:collect 结算应见集 = p1 可见 ∪ p2 翻回,应见而有未渲者 ⇒ 掩码并集同帧重渲,迭代 ≤4 未收敛 ⇒ 全掩码兜底 = 零剔除精确收敛;剔除零假阳性 ⇒ 闭环后画面与全集渲染位级一致,由 RURIX_HZB_ALL_VISIBLE=1 登记实验臂 digest_seq 逐帧对拍机核门承载——ci 门脚本裁决）;host 金标准面只读消费 0-byte（geometry/{hzb,cull}.rs:Frustum 视锥离屏第一关 + probe 帧 HzbPyramid::build/test_rect/exact_rect_occluded 复算——hzb.parity 三面硬门 harness fail-fast）;real_render_frame_ms = 生产链渲染墙钟（含 BGRA8 强制回读 + 逐帧判定小回读,闭环重渲墙钟含内——closure_extra_gpu_ms 单列强加 GPU 段）;stats.scene_gpu_ms = primary+shade 末次提交 GPU,prod_gpu_ms = 主链六段,hzb_aux_gpu_ms = 剔除链 GPU（test×2+reduce+pack 全提交累计）,hzb_host_ms = host 初剔分类段;host 金标准全场景色彩对拍面 = null 诚实登记（HZB 腿对拍 = parity 三面承载,G34-1 非 HZB 腿色彩对拍锚在案不混口径）。g31_window_present.rs/g27_hzb_reduce.rx/g27_hzb_test.rx/g31_hzb_pack.rx/g34_unified_gi.rx 0-byte——其门为回归锚。")
    ));
    ev.push('}');

    if evidence_path.is_empty() {
        println!("{ev}");
    } else {
        std::fs::write(&evidence_path, format!("{ev}\n"))
            .unwrap_or_else(|e| fail(&format!("evidence 写 {evidence_path}: {e}")));
        eprintln!("{GTAG}: [hzb] evidence → {evidence_path}");
    }
    // 对拍完成性硬门（frames_done 覆盖 probe 帧而对拍未成 = 内部破缺;
    // user_close/短跑未覆盖 = parity null 如实登记不冒充）。
    if exit_reason == "frames_done" && frames_done > probe_fi && hzb_parity.is_none() {
        fail("HZB 接线态对拍未执行（probe 帧覆盖窗内未完成,内部破缺）");
    }
    if !dyn_all_pass {
        fail("动态实例位置核验汇总 fail（帧详情见 evidence dyn.verify_frames）");
    }
    eprintln!(
        "{GTAG}: [hzb] PASS frames={frames_done}/{total} real_render={r_mean:.3}ms present={p_mean:.3}ms tested={hzb_tested} occluded_p1={hzb_occluded} flipped_p2={hzb_flipped} closure_extra={hzb_closure_submits} fallback_frames={hzb_fallbacks} all_visible_arm={all_visible_arm} exit={exit_reason}"
    );
    std::process::exit(0)
}
