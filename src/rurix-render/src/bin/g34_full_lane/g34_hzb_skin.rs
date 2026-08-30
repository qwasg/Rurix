// Assisted-by: Cursor:Claude（G37 W3 hzb_skin——HZB×蒙皮同车道合并面）
// G37 W3 合并区段——独立 include 区段（`g34_full_lane.rs` 尾部 include! 拼接；
// 与 G34-2 HZB / G34-3 蒙皮分区并行面零交叠：本段全部符号 `G34HS*`/`g34hs*`
// 前缀自持，主 bin 挂钩面 = `--hzb on --skin` 同开早分支 + primary_skin SPV
// 旗标解析）。G36 W4-W5 留窗字面「HZB×蒙皮同车道（新 kernel 合并面）归后续
// 波」的兑现件。门 `g37.wave3.hzb_skin`。
//
// ## 合并架构（G34-2 HZB 车道骨架 + G34-3 蒙皮三件接入）
//
// - **实例分解** = [静态逐节点 BLAS 0..N-1 | 动态立方体 N | 蒙皮角色 N+1]
//   （G34-2 节点分解 + G34-3 角色尾槽;tris/mats SSBO 三分区 =
//   g34skin_assets 逐字复用——静态段节点分组 tri_offset 与合并 SSBO 位级同
//   buffer）。动态/角色两尾槽**恒可见不参剔**（核验对象面:剔除会破坏 A4
//   位置核验与 B5 蒙皮核验硬门——剔除计数面 = 静态节点,如实登记）。
// - **蒙皮通路** = G34-3 逐字：g31_skin pass（pass 0）逐顶点 LBS 重写 tris
//   SSBO 角色段（当帧蒙皮世界空间）+ blas_refit **双桥**（after_pass=0:
//   表 0 副本经 `FrameUpdate::blas_refit`,表 1 副本经 render_exec G37 W3
//   加性 `blas_refit_b`——双 TLAS 双 manager 各持 BLAS 副本,主射线初剔表与
//   阴影全量表须同帧同内容,单 refit 位结构不足 = 本合并面的执行器件）。
//   TLAS 帧首 refit 读上一帧角色 BLAS 内容 ⇒ 角色 TLAS 级 AABB 滞后一帧
//   （G34-3 单表车道在案语义逐字继承;质心 ≤4px/AABB ≤6px 容差吸收面）。
// - **主射线腿** = kernels/g34_unified_primary_skin.rx（G34-2 primary 全字面
//   + out_hit [inst,prim,bu,bv] 加性扩面——G36 留窗字面「新 kernel」件;母版
//   g34_unified_primary.rx 0-byte）。shade/reduce/test/pack 四件 0-byte 消费
//   （角色段经 inst_base[char]=char_tri_base 前缀和分派,着色数学与 gi_skin
//   逐 op 同式;角色 tritex −1 ⇒ 常量面,mats 角色段品红发射行）。
// - **MV 腿** = kernels/g34_unified_mv.rx 0-byte 消费（G34-3 独消费变体三臂:
//   类 1 相机/类 2 刚性/类 3 蒙皮——char_inst/dyn_inst 经参数面下发 =
//   (N+1)/N as f32,kernel 零硬编码实例号;g14_mv 不进本车道）。
// - **HZB 剔除链** = G34-2 逐字（两阶段闭环 + 帧内金字塔轮换 + probe 帧
//   host 金标准三面对拍;角色/动态深度进金字塔——两尾槽恒可见故对自身零
//   假阴性,静态节点误剔经闭环重渲收敛,零假阳性硬门维持）。
// - **核验面** = 两车道口径并集：HZB probe 三面（mips 位级/判定逐字节/零假
//   阳性）∧ 蒙皮三面（① 逐顶点位级 ② hit inst==char 位置核验 ③ MV 三类）
//   ∧ fork B 动态位置核验 ∧ 确定性双跑（门脚本裁决面）。

// ---------------------------------------------------------------------------
// G37 W3 常量面（门键 / schema 字面 / SPV 默认路径）
// ---------------------------------------------------------------------------

/// G37 W3 门键（evidence `gate` 字段字面）。
const G34HS_GATE: &str = "g37.wave3.hzb_skin";
/// G37 W3 harness evidence schema 字面（.tmp 工作区件;G34-2 同律——harness
/// 真跑件不注册 check_schemas,数字经门裁决件蒸馏登记）。
const G34HS_SCHEMA: &str = "rurix.g37.hzb_skin_unified_evidence.v1";
/// 合并主射线 kernel 默认 SPV（源 = kernels/g34_unified_primary_skin.rx——
/// G37 W3 加性件,验收脚本保障编译;母版 primary 0-byte）。
const G34HS_DEFAULT_SPV_PRIMARY_SKIN: &str = ".tmp/g34_gates/hzb_skin/g34_unified_primary_skin.spv";

// ---------------------------------------------------------------------------
// G37 W3 描述组（HZB 描述组 + 蒙皮七件 + 三处 pass 面补丁）
// ---------------------------------------------------------------------------

/// G37 W3 车道资源/回读下标面（HZB 面 + 蒙皮七件 + 两路回读追加）。
#[derive(Debug, Clone)]
struct G34HsIds {
    hzb: G34HzbIds,
    hit4: u32,
    rest: u32,
    wt: u32,
    pal_cur: u32,
    pal_prev: u32,
    prev: u32,
    skin_params: u32,
    rb_hit: u32,
    rb_char: u32,
}

/// G37 W3 合并描述组装配：`g34_lane_descs_hzb` 产物（G34Full 27 + encode 2 +
/// HZB 追加面）三处补丁 + 蒙皮七件追加：
/// ① 资源:U_MV_PARAMS 扩容 64 f32 原位置换（G34-3 同律）+ 蒙皮七件尾接;
/// ② pass:g31_skin 前插（pass 0,全 pass 下标 +1）+ primary 绑定尾接 out_hit
///   （kernels/g34_unified_primary_skin.rx 签名第 4 输出）+ mv pass 整体置换
///   （g14_mv 不进本车道 → kernels/g34_unified_mv.rx 六 SSBO 绑定面）;
/// ③ 回读:hit 通道 + tris 角色段两路尾接（下标 10/11）。
#[allow(clippy::too_many_arguments)]
fn g34hs_descs<'x>(
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
    bits: &'x UnifiedLaneBits,
    assets: &'x G34SkinAssets,
    skin_spv: &'x [u8],
    skin_dispatch: [u32; 3],
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
    G34HsIds,
) {
    let ipc = (iw * ih) as u64;
    let (mut resources, mut passes, mut barriers, mut readbacks, hzb_ids) = g34_lane_descs_hzb(
        g34,
        enc_spv,
        enc_dispatch,
        enc_params_bytes,
        hz,
        n_instances,
        iw,
        ih,
        ow,
        oh,
    );
    let storage = BufferUsage {
        storage: true,
        ..BufferUsage::default()
    };
    // ── ① 资源补丁:U_MV_PARAMS 扩容置换（G34-3 逐字——64 f32 = 256B;
    //    g34_unified_mv 刚性臂扩面 [40..54] 消费）+ 蒙皮七件尾接 ──
    resources[U_MV_PARAMS as usize] = ResourceDesc::Buffer(BufferDesc {
        size: (G34S_MV_PARAMS_LEN * 4) as u64,
        usage: storage,
        data: None,
        device_local: false,
    });
    let host_buf = |size: u64| {
        ResourceDesc::Buffer(BufferDesc {
            size,
            usage: storage,
            data: None,
            device_local: false,
        })
    };
    let bone_bytes = (assets.character.bone_count * 48) as u64;
    let mut next = resources.len() as u32;
    macro_rules! take {
        ($r:expr) => {{
            let id = next;
            next += 1;
            resources.push($r);
            id
        }};
    }
    let hit4 = take!(ResourceDesc::Buffer(BufferDesc {
        size: ipc * 16,
        usage: storage,
        data: None,
        device_local: true,
    }));
    let rest = take!(ResourceDesc::Buffer(BufferDesc {
        size: assets.rest_bytes.len() as u64,
        usage: storage,
        data: Some(&assets.rest_bytes),
        device_local: true,
    }));
    let wt = take!(ResourceDesc::Buffer(BufferDesc {
        size: assets.wt_bytes.len() as u64,
        usage: storage,
        data: Some(&assets.wt_bytes),
        device_local: true,
    }));
    let pal_cur = take!(host_buf(bone_bytes));
    let pal_prev = take!(host_buf(bone_bytes));
    let prev = take!(ResourceDesc::Buffer(BufferDesc {
        size: (assets.character.vertex_count * 12) as u64,
        usage: storage,
        data: None,
        device_local: true,
    }));
    let skin_params = take!(host_buf((SKIN_PARAMS_LEN * 4) as u64));
    let _ = next;
    // ── ② pass 补丁 ──
    // primary（现下标 0）绑定尾接 out_hit（primary_skin 签名第 4 输出;
    // SPV 本体已由 G34HzbBits::load 以 primary_skin 路径装载）。
    if let Pass::Compute(cp) = &mut passes[0] {
        cp.bindings.storage_buffers.push(hit4);
    }
    barriers[0].push((hit4, TargetState::StorageWrite));
    // mv pass（现下标 2）整体置换:g14_mv 不进本车道——kernels/g34_unified_mv
    // 六 SSBO 绑定面（G34-3 g34skin_descs 同字面;spv/dispatch = bits 面——
    // 调用方以 g34_unified_mv SPV 路径装载 UnifiedLaneBits.spv_mv）。
    passes[2] = Pass::Compute(ComputePass {
        name: "g34_unified_mv",
        spirv: &bits.spv_mv,
        entry: None,
        dispatch: DispatchSpec::Direct(bits.mv_dispatch),
        bindings: Bindings {
            storage_buffers: vec![
                U_SCENE_DEPTH,
                U_MV_PARAMS,
                hit4,
                prev,
                U_TRIS,
                U_MV_OUT,
            ],
            ..Bindings::default()
        },
    });
    barriers[2] = vec![
        (U_SCENE_DEPTH, TargetState::StorageReadWrite),
        (U_MV_PARAMS, TargetState::StorageReadWrite),
        (U_MV_OUT, TargetState::StorageReadWrite),
        (hit4, TargetState::StorageReadWrite),
        (prev, TargetState::StorageReadWrite),
        (U_TRIS, TargetState::StorageReadWrite),
    ];
    // g31_skin 前插（pass 0;全 pass 下标 +1——blas_refit after_pass=0 挂本
    // pass,绑定序 = kernel 签名序,G34-3 逐字）。
    passes.insert(
        0,
        Pass::Compute(ComputePass {
            name: "g31_skin",
            spirv: skin_spv,
            entry: None,
            dispatch: DispatchSpec::Direct(skin_dispatch),
            bindings: Bindings {
                storage_buffers: vec![rest, wt, pal_cur, pal_prev, skin_params, U_TRIS, prev],
                ..Bindings::default()
            },
        }),
    );
    barriers.insert(
        0,
        vec![
            (U_TRIS, TargetState::StorageReadWrite),
            (rest, TargetState::StorageReadWrite),
            (wt, TargetState::StorageReadWrite),
            (pal_cur, TargetState::StorageReadWrite),
            (pal_prev, TargetState::StorageReadWrite),
            (prev, TargetState::StorageReadWrite),
            (skin_params, TargetState::StorageReadWrite),
        ],
    );
    // ── ③ 回读追加:hit 通道（位置核验/刚性分派/MV 源）+ tris 角色段
    //    （① 逐顶点 device/host 对拍面——B5 max_abs == 0 位级口径）──
    let rb_hit = readbacks.len() as u32;
    readbacks.push(Readback::Buffer {
        res: hit4,
        offset: 0,
        size: ipc * 16,
    });
    let rb_char = readbacks.len() as u32;
    readbacks.push(Readback::Buffer {
        res: U_TRIS,
        offset: (assets.char_tri_base * 36) as u64,
        size: (assets.character.tri_count * 36) as u64,
    });
    let ids = G34HsIds {
        hzb: hzb_ids,
        hit4,
        rest,
        wt,
        pal_cur,
        pal_prev,
        prev,
        skin_params,
        rb_hit,
        rb_char,
    };
    (resources, passes, barriers, readbacks, ids)
}

// ---------------------------------------------------------------------------
// G37 W3 车道状态机（G34HzbLane 同律闭环 + 蒙皮逐帧上传 + 双 BLAS refit）
// ---------------------------------------------------------------------------

/// G37 W3 一帧产物（G34HzbFrameRec 同构 + 蒙皮四路）。
struct G34HsFrameRec {
    skin_gpu_ns: f64,
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
    mv_out: Option<Vec<f32>>,
    scene_color: Option<Vec<f32>>,
    scene_depth: Option<Vec<f32>>,
    hit: Option<Vec<f32>>,
    char_tris: Option<Vec<f32>>,
    readback_convert_ms: f64,
    hzb: G34HzbDecisionRec,
}

/// G37 W3 逐实例初剔分类（G34-2 分类器 + 角色尾槽恒可见追加——核验对象面
/// 同 fork B 律:nearest=−∞ ⇒ standard-Z 严格不等式恒 Visible;
/// RURIX_HZB_ALL_VISIBLE=1 登记实验臂经内层分类器同源生效,尾槽本就恒可见）。
fn g34hs_classify(vp: &Mat4, iw: u32, ih: u32, groups: &[SceneNodeGroup]) -> Vec<G34HzbClass> {
    let mut out = g34_hzb_classify(vp, iw, ih, groups, true);
    out.push(G34HzbClass::Rect {
        uv_min: [0.0, 0.0],
        uv_max: [1.0, 1.0],
        nearest: f32::NEG_INFINITY,
    });
    out
}

struct G34HsLane<'a> {
    session: DeviceFrameSession<'a>,
    parity: usize,
    has_history_state: bool,
    prev_vp_j: Option<Mat4>,
    ids: G34HsIds,
    groups: Vec<SceneNodeGroup>,
    dyn_tri_base: usize,
    /// 角色段字节面（blas_refit 双桥 src_offset/byte_len;era 内恒定）。
    char_src_offset: u64,
    char_byte_len: u64,
    /// 角色 BLAS 下标（= 静态节点数 + 1;双 AS 表同下标副本）。
    char_blas: u32,
    /// 上一帧动态实例变换（类 2 刚性 MV 臂 prev_dyn_xf 源;G34-3 同律）。
    prev_dyn_xf: Option<[f32; 12]>,
    /// 下一帧渲染掩码（G34-2 同律;动态/角色两尾槽恒 0xFF）。
    masks: Vec<u8>,
    prev_p2_rects: Vec<f32>,
    prev_p2_inst: Vec<u32>,
    last_rects_p1: Vec<f32>,
    last_rects_inst: Vec<u32>,
    n_levels: usize,
}

impl<'a> G34HsLane<'a> {
    #[allow(clippy::too_many_arguments)]
    fn create(
        resources: &'a [ResourceDesc<'a>],
        passes: &'a [Pass<'a>],
        barriers: &'a [&'a [(u32, TargetState)]],
        readbacks: &'a [Readback],
        accel_structs: &[AccelStructDesc<'a>],
        ids: G34HsIds,
        groups: Vec<SceneNodeGroup>,
        dyn_tri_base: usize,
        char_tri_base: usize,
        char_tri_count: usize,
        n_levels: usize,
    ) -> Result<Self, String> {
        if !vk::vulkan_available() {
            return Err("vulkan loader 不可用".into());
        }
        if groups.is_empty() {
            return Err("HZB×蒙皮面场景零可剔除实例(节点分组为空,fail-closed 不冒充)".into());
        }
        let n = groups.len() + 2;
        // frame_slots=2（G34-2 同律顺序全同步;tlas_update 双表 + blas_refit
        // 双桥全走顺序入口,FIF 流水面拒——render_exec 校验面承载）。
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
            char_blas: (groups.len() + 1) as u32,
            groups,
            dyn_tri_base,
            char_src_offset: (char_tri_base * 36) as u64,
            char_byte_len: (char_tri_count * 36) as u64,
            prev_dyn_xf: None,
            masks: vec![0xFF; n],
            prev_p2_rects: Vec::new(),
            prev_p2_inst: Vec::new(),
            last_rects_p1: Vec::new(),
            last_rects_inst: Vec::new(),
            n_levels,
        })
    }

    /// 实例表组装（表 0 = 掩码面/表 1 = 全 0xFF 面;静态 identity + 动态本帧
    /// 变换 + 角色恒 identity——形变全在 BLAS 顶点内,G34-3 逐字）。
    fn instances_with(
        &self,
        masks: &[u8],
        dyn_xf: [f32; 12],
    ) -> Vec<RayQueryTransformedInstanceDesc> {
        let n_static = self.groups.len();
        let mut v = Vec::with_capacity(n_static + 2);
        for i in 0..n_static {
            v.push(RayQueryTransformedInstanceDesc {
                blas: i as u32,
                custom_index: i as u32,
                mask: masks[i],
                sbt_record_offset: 0,
                transform: vk::RAY_QUERY_IDENTITY_TRANSFORM,
            });
        }
        v.push(RayQueryTransformedInstanceDesc {
            blas: n_static as u32,
            custom_index: n_static as u32,
            mask: masks[n_static],
            sbt_record_offset: 0,
            transform: dyn_xf,
        });
        v.push(RayQueryTransformedInstanceDesc {
            blas: self.char_blas,
            custom_index: self.char_blas,
            mask: masks[n_static + 1],
            sbt_record_offset: 0,
            transform: vk::RAY_QUERY_IDENTITY_TRANSFORM,
        });
        v
    }

    /// 单次提交（两阶段调度的一拍）:G34-2 submit_once 同律 + 蒙皮三小件上传
    /// + 双 TLAS 逐帧 refit + **双 BLAS refit**（表 0 = FrameUpdate::blas_refit,
    /// 表 1 = execute_with_frame_update_dual_tlas_ex 的 blas_refit_b——
    /// after_pass=0〔g31_skin〕双桥同帧;闭环重拍幂等:palette 同帧不变 ⇒
    /// g31_skin 重跑同输出 ⇒ 双桥重拷同字节）。
    #[allow(clippy::too_many_arguments)]
    fn submit_once(
        &mut self,
        scene_params: &[f32],
        mv_params: &[f32],
        tsr_params: &[f32],
        skin_params: &[f32],
        pal_cur_bytes: &[u8],
        pal_prev_bytes: &[u8],
        n_p1: u32,
        rects_p2: &[f32],
        n_p2: u32,
        masks: &[u8],
        dyn_xf: [f32; 12],
        rb_out: bool,
        rb_verify: bool,
        rb_bgra: bool,
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
            (
                StableResourceId(u64::from(ids.skin_params) + 1),
                0,
                bytes_f32(skin_params),
            ),
            (
                StableResourceId(u64::from(ids.pal_cur) + 1),
                0,
                pal_cur_bytes.to_vec(),
            ),
            (
                StableResourceId(u64::from(ids.pal_prev) + 1),
                0,
                pal_prev_bytes.to_vec(),
            ),
        ];
        // rect 流空段不上传（G34-2 同律:执行器拒空段,kernel params[0]=n 门守卫）。
        if !self.last_rects_p1.is_empty() {
            uploads.push((
                StableResourceId(u64::from(ids.hzb.rects_p1) + 1),
                0,
                bytes_f32(&self.last_rects_p1),
            ));
        }
        uploads.push((
            StableResourceId(u64::from(ids.hzb.params_p1) + 1),
            0,
            bytes_f32(&params_p1),
        ));
        if !rects_p2.is_empty() {
            uploads.push((
                StableResourceId(u64::from(ids.hzb.rects_p2) + 1),
                0,
                bytes_f32(rects_p2),
            ));
        }
        uploads.push((
            StableResourceId(u64::from(ids.hzb.params_p2) + 1),
            0,
            bytes_f32(&params_p2),
        ));
        // ── 双 TLAS 逐帧 refit + 双 BLAS refit（--full 闭集 ⇒ dyn/skin 恒动,
        //    每拍全量更新——表 0 掩码 + 动态变换,表 1 全 0xFF + 动态变换;
        //    角色 BLAS 两副本 after_pass=0 双桥）──
        let tlas_update = Some((0u32, self.instances_with(masks, dyn_xf), TlasBuildAction::Refit));
        let tlas_update_b = Some((
            1u32,
            self.instances_with(&vec![0xFF; masks.len()], dyn_xf),
            TlasBuildAction::Refit,
        ));
        let blas_refit_b = Some(BlasRefitUpdate {
            as_index: 1,
            blas_index: self.char_blas,
            src: StableResourceId(u64::from(U_TRIS) + 1),
            src_offset: self.char_src_offset,
            byte_len: self.char_byte_len,
            after_pass: 0,
        });
        let p = self.parity;
        // parity 三 pass 绑定轮换（合并 pass 序:skin0/primary1/shade2/mv3 →
        // resample=4 / resolve=5 / encode=6）。
        let binding_overrides = vec![
            (
                4u32,
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
                5u32,
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
                6u32,
                Bindings {
                    storage_buffers: vec![U_OUT_COLOR[p], G34_U_ENC_PARAMS, G34_U_ENC_OUT],
                    ..Bindings::default()
                },
            ),
        ];
        // 回读子集（解析序 = push 序）:[out(p)?] [mv2,depth3,scene4]? [bgra5]?
        // → probe[depth_hz9,flat8]? → 判定[6,7] 恒在 → 蒙皮[hit10,char11]?。
        let mut subset: Vec<u32> = Vec::new();
        if rb_out {
            subset.push(p as u32);
        }
        if rb_verify {
            subset.extend_from_slice(&[2, 3, 4]);
        }
        if rb_bgra {
            subset.push(G34_RB_BGRA);
        }
        if probe_pre {
            subset.push(ids.hzb.rb_depth_hz);
            subset.push(ids.hzb.rb_flat);
        }
        subset.push(ids.hzb.rb_verdicts_p1);
        subset.push(ids.hzb.rb_verdicts_p2);
        if rb_verify {
            subset.push(ids.rb_hit);
            subset.push(ids.rb_char);
        }
        let update = FrameUpdate {
            tlas_update,
            buffer_uploads: uploads,
            binding_overrides,
            push_constant_overrides: vec![],
            readback_subset: Some(subset),
            // 表 0 角色副本桥（tlas_update 同槽纪律;after_pass=0 = g31_skin）。
            blas_refit: Some(BlasRefitUpdate {
                as_index: 0,
                blas_index: self.char_blas,
                src: StableResourceId(u64::from(U_TRIS) + 1),
                src_offset: self.char_src_offset,
                byte_len: self.char_byte_len,
                after_pass: 0,
            }),
        };
        let prov = self.session.next_provenance_with_update_dual_tlas_ex(
            &update,
            tlas_update_b.as_ref(),
            blas_refit_b.as_ref(),
        )?;
        self.session
            .execute_with_frame_update_dual_tlas_ex(&prov, &update, tlas_update_b, blas_refit_b)
    }

    /// 一帧:G34-2 frame 同律（初剔分类 → 两阶段提交 + 闭环重渲 → 终判滚动）
    /// + 蒙皮逐帧面（palette 双表/skin 参数/64 f32 mv 参数调用方预打包）。
    #[allow(clippy::too_many_arguments)]
    fn frame(
        &mut self,
        iw: u32,
        ih: u32,
        ow: u32,
        oh: u32,
        scene_params: &[f32],
        skin_params: &[f32],
        pal_cur_bytes: &[u8],
        pal_prev_bytes: &[u8],
        jitter: [f32; 2],
        vp: &Mat4,
        vp_j: &Mat4,
        exposure: f32,
        reset: bool,
        rb_out: bool,
        rb_verify: bool,
        rb_bgra: bool,
        probe_pre: bool,
        dyn_xf: [f32; 12],
    ) -> Result<G34HsFrameRec, String> {
        let t_host = std::time::Instant::now();
        // ── ① 初剔分类（静态节点 + 动态/角色两尾槽恒可见）──
        let class = g34hs_classify(vp, iw, ih, &self.groups);
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
        // ── ② mv（64 f32 蒙皮扩面:前 40 = 相机臂逐字,[35]=char_inst,
        //    [40..52]=prev_dyn_xf,[52]=dyn_tri_base,[53]=dyn_inst——G34-3
        //    同律,实例号 = 合并分解 (N+1)/N）/ tsr 参数面 ──
        let inv_cur = vp_j
            .inverse()
            .ok_or("jittered view-proj 必须可逆(mv 参数面)")?;
        let prev = self.prev_vp_j.unwrap_or(*vp_j);
        let mut mv_params = pack_mv_params(iw, ih, &inv_cur, &prev, self.prev_vp_j.is_some());
        mv_params[35] = (self.groups.len() + 1) as f32;
        let prev_xf = self.prev_dyn_xf.unwrap_or(dyn_xf);
        mv_params.resize(G34S_MV_PARAMS_LEN, 0.0);
        for k in 0..12 {
            mv_params[40 + k] = prev_xf[k];
        }
        mv_params[52] = self.dyn_tri_base as f32;
        mv_params[53] = self.groups.len() as f32;
        let has_history = !reset && self.has_history_state;
        let tsr_params = pack_tsr_params(iw, ih, ow, oh, jitter, exposure, has_history, false);
        let host_ms = t_host.elapsed().as_secs_f64() * 1000.0;

        // ── ③ 两阶段提交 + 闭环重渲循环（G34-2 逐字同律）──
        let mut rendered = self.masks.clone();
        let mut p2_rects = self.prev_p2_rects.clone();
        let mut p2_inst = self.prev_p2_inst.clone();
        let mut closure_extra_submits = 0u32;
        let mut closure_full_fallback = false;
        let mut hzb_gpu_ns = 0.0f64;
        let mut prod_gpu_total_ns = 0.0f64;
        let mut v1_main: Option<Vec<u8>> = None;
        let (out_last, v1_last, v2_last, p2_inst_last);
        loop {
            let n_p2 = p2_inst.len() as u32;
            let out = self.submit_once(
                scene_params,
                &mv_params,
                &tsr_params,
                skin_params,
                pal_cur_bytes,
                pal_prev_bytes,
                n_p1,
                &p2_rects,
                n_p2,
                &rendered,
                dyn_xf,
                rb_out,
                rb_verify,
                rb_bgra,
                probe_pre,
                iw,
                ih,
            )?;
            let (v1, v2) =
                g34hs_parse_verdicts(&out, rb_out, rb_verify, rb_bgra, probe_pre, n_p1, n_p2)?;
            if v1_main.is_none() {
                v1_main = Some(v1.clone());
            }
            let prod_ns = g34hs_prod_gpu_ns(&out)?;
            prod_gpu_total_ns += prod_ns;
            hzb_gpu_ns += g34_hzb_aux_gpu_ns(&out);
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
            for (i, c) in correct.iter().enumerate() {
                if *c == 0xFF {
                    rendered[i] = 0xFF;
                }
            }
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
                rendered = vec![0xFF; n];
                p2_rects = Vec::new();
                p2_inst = Vec::new();
                closure_full_fallback = true;
                let out2 = self.submit_once(
                    scene_params,
                    &mv_params,
                    &tsr_params,
                    skin_params,
                    pal_cur_bytes,
                    pal_prev_bytes,
                    n_p1,
                    &p2_rects,
                    0,
                    &rendered,
                    dyn_xf,
                    rb_out,
                    rb_verify,
                    rb_bgra,
                    probe_pre,
                    iw,
                    ih,
                )?;
                let (v1b, v2b) =
                    g34hs_parse_verdicts(&out2, rb_out, rb_verify, rb_bgra, probe_pre, n_p1, 0)?;
                prod_gpu_total_ns += g34hs_prod_gpu_ns(&out2)?;
                hzb_gpu_ns += g34_hzb_aux_gpu_ns(&out2);
                out_last = out2;
                v1_last = v1b;
                v2_last = v2b;
                p2_inst_last = p2_inst;
                break;
            }
        }

        // ── ④ 终判滚动（G34-2 逐字）──
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

        // ── ⑤ 产物组装 ──
        let prod_last_ns = g34hs_prod_gpu_ns(&out_last)?;
        let closure_extra_ns = prod_gpu_total_ns - prod_last_ns;
        let verdicts_p1_rec = v1_main.clone().unwrap_or_else(|| v1_last.clone());
        let rec = self.rec_from_output(
            out_last,
            rb_out,
            rb_verify,
            rb_bgra,
            probe_pre,
            ow,
            oh,
            iw,
            ih,
            hzb_gpu_ns,
            G34HzbDecisionRec {
                tested_p1: n_p1,
                occluded_p1: 0,
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
        self.prev_dyn_xf = Some(dyn_xf);
        Ok(rec)
    }

    /// 末次提交产物组装（回读按子集 push 序解析 + 尺寸校验 + 遥测逐名提取）。
    #[allow(clippy::too_many_arguments)]
    fn rec_from_output(
        &self,
        mut out: DeviceFrameOutput,
        rb_out: bool,
        rb_verify: bool,
        rb_bgra: bool,
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
    ) -> Result<G34HsFrameRec, String> {
        let gpu = |name: &str| -> Result<f64, String> {
            out.telemetry
                .passes
                .iter()
                .find(|pp| pp.name == name)
                .map(|pp| pp.gpu_ns)
                .ok_or_else(|| format!("telemetry 缺 {name} pass 行"))
        };
        let skin_gpu_ns = gpu("g31_skin")?;
        let scene_gpu_ns = gpu("g34_unified_primary_skin")? + gpu("g34_unified_shade")?;
        let mv_gpu_ns = gpu("g34_unified_mv")?;
        let resample_gpu_ns = gpu("g14_8_tsr_resample")?;
        let resolve_gpu_ns = gpu("g14_8_tsr_resolve")?;
        let encode_gpu_ns = gpu("g31_display_encode")?;
        let t_convert = std::time::Instant::now();
        let bgra_px = (ow * oh * 4) as usize;
        let f32_px = (ow * oh * 3) as usize;
        let scene_px = (iw * ih * 3) as usize;
        let depth_px = (iw * ih) as usize;
        let mv_px = (iw * ih * 2) as usize;
        let hit_px = (iw * ih * 4) as usize;
        let char_px = (self.char_byte_len / 4) as usize;
        let mut idx = 0usize;
        let take_rb = |out: &mut DeviceFrameOutput, idx: &mut usize| -> Result<Vec<u8>, String> {
            if *idx >= out.readbacks.len() {
                return Err(format!(
                    "G37 W3 回读路数 {} 少于子集消费序 {idx}",
                    out.readbacks.len()
                ));
            }
            let b = std::mem::take(&mut out.readbacks[*idx]);
            *idx += 1;
            Ok(b)
        };
        let out_color = if rb_out {
            let c = read_f32(&take_rb(&mut out, &mut idx)?);
            if c.len() != f32_px {
                return Err("G37 W3 f32 out_color 回读字节数与输出分辨率不符".into());
            }
            Some(c)
        } else {
            None
        };
        let (mv_out, scene_depth, scene_color) = if rb_verify {
            let m = read_f32(&take_rb(&mut out, &mut idx)?);
            if m.len() != mv_px {
                return Err("G37 W3 mv 回读字节数与内部分辨率不符".into());
            }
            let d = read_f32(&take_rb(&mut out, &mut idx)?);
            if d.len() != depth_px {
                return Err("G37 W3 scene depth 回读字节数与内部分辨率不符".into());
            }
            let s = read_f32(&take_rb(&mut out, &mut idx)?);
            if s.len() != scene_px {
                return Err("G37 W3 scene color 回读字节数与内部分辨率不符".into());
            }
            (Some(m), Some(d), Some(s))
        } else {
            (None, None, None)
        };
        let bgra8 = if rb_bgra {
            let b = take_rb(&mut out, &mut idx)?;
            if b.len() != bgra_px {
                return Err(format!("G37 W3 BGRA8 回读字节 {} ≠ {ow}x{oh}x4", b.len()));
            }
            Some(b)
        } else {
            None
        };
        let (probe_depth, probe_flat) = if probe_pre {
            let d = read_f32(&take_rb(&mut out, &mut idx)?);
            let f = read_f32(&take_rb(&mut out, &mut idx)?);
            (Some(d), Some(f))
        } else {
            (None, None)
        };
        // 判定两路（frame() 已解析消费;此处按序耗掉保对齐）。
        let _ = take_rb(&mut out, &mut idx)?;
        let _ = take_rb(&mut out, &mut idx)?;
        let (hit, char_tris) = if rb_verify {
            let h = read_f32(&take_rb(&mut out, &mut idx)?);
            if h.len() != hit_px {
                return Err("G37 W3 hit 回读字节数与内部分辨率不符".into());
            }
            let t = read_f32(&take_rb(&mut out, &mut idx)?);
            if t.len() != char_px {
                return Err("G37 W3 蒙皮顶点回读字节数与角色段不符".into());
            }
            (Some(h), Some(t))
        } else {
            (None, None)
        };
        if idx != out.readbacks.len() {
            return Err(format!(
                "G37 W3 回读消费序 {idx} ≠ 实到路数 {}",
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
        Ok(G34HsFrameRec {
            skin_gpu_ns,
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
            mv_out,
            scene_color,
            scene_depth,
            hit,
            char_tris,
            readback_convert_ms,
            hzb: hz,
        })
    }
}

/// G37 W3 生产链七段 GPU（skin+primary_skin+shade+mv+resample+resolve+encode;
/// G34-2 六段面 + 蒙皮 pass——合并 pass 名字面）。
fn g34hs_prod_gpu_ns(out: &DeviceFrameOutput) -> Result<f64, String> {
    let mut sum = 0.0;
    for name in [
        "g31_skin",
        "g34_unified_primary_skin",
        "g34_unified_shade",
        "g34_unified_mv",
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

/// G37 W3 判定回读解析（子集 push 序:[out?][mv/depth/scene?][bgra?]
/// [probe×2?] → 判定两路 → [hit/char?];G34-2 同律 >0.5 判读字节）。
fn g34hs_parse_verdicts(
    out: &DeviceFrameOutput,
    rb_out: bool,
    rb_verify: bool,
    rb_bgra: bool,
    probe_pre: bool,
    n_p1: u32,
    n_p2: u32,
) -> Result<(Vec<u8>, Vec<u8>), String> {
    let base = usize::from(rb_out)
        + 3 * usize::from(rb_verify)
        + usize::from(rb_bgra)
        + 2 * usize::from(probe_pre);
    let expect = base + 2 + 2 * usize::from(rb_verify);
    let rbs = &out.readbacks;
    if rbs.len() != expect {
        return Err(format!(
            "G37 W3 判定回读路数 {} ≠ {expect}(out={rb_out} verify={rb_verify} bgra={rb_bgra} probe={probe_pre})",
            rbs.len(),
        ));
    }
    let v1f = read_f32(&rbs[base]);
    let v2f = read_f32(&rbs[base + 1]);
    if (v1f.len() as u32) < n_p1 || (v2f.len() as u32) < n_p2 {
        return Err("G37 W3 判定回读长度小于本拍 rect 数".into());
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
// G37 W3 主流程（--hzb on --skin 同开面：HZB 剔除×蒙皮×纹理×slab×动态五特性
// 同开真窗口统一车道;main() 早分支唯一消费面）
// ---------------------------------------------------------------------------

/// --hzb on --skin 同开 CLI 面（主 bin 早分支消费;= G34HzbCli 字段面 −
/// geo 组合 + 蒙皮 SPV 三件——geo × skin 组合维持留窗如实拒跑,主 bin 闭集
/// 裁决先行）。
struct G34HsCli {
    frames: u32,
    warmup: u32,
    tier: u32,
    contract_path: String,
    g10_dir: String,
    gltf_path: String,
    /// 合并主射线 kernel（g34_unified_primary_skin.rx——G37 W3 加性件）。
    spv_primary_skin: String,
    spv_hzb_shade: String,
    spv_hzb_pack: String,
    spv_hzb_reduce: String,
    spv_hzb_test: String,
    /// 蒙皮 compute kernel（g31_skin.rx 0-byte 复用）。
    spv_skin: String,
    /// 统一 MV kernel（g34_unified_mv.rx 0-byte 复用——G34-3 独消费变体）。
    spv_skin_mv: String,
    /// 统一 mega kernel（描述组装配期借用,mega pass 不进本车道零消费）。
    spv_scene: String,
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

/// G37 W3 合并主流程（main() 早分支唯一消费面;装配段 = G34-2 hzb_main 同律
/// 〔契约链/G10 转引/节点分组 + UV sink 双记录装配/slab/纹理/窗口〕+ G34-3
/// 蒙皮资产/核验三面;host 金标准全场景对拍面不建——HZB 腿 = probe 三面,
/// 蒙皮腿 = ① 逐顶点臂,登记口径）。
fn g34hs_main(cli: G34HsCli) -> ! {
    let G34HsCli {
        frames,
        warmup,
        tier,
        contract_path,
        g10_dir,
        mut gltf_path,
        spv_primary_skin,
        spv_hzb_shade,
        spv_hzb_pack,
        spv_hzb_reduce,
        spv_hzb_test,
        spv_skin,
        spv_skin_mv,
        spv_scene,
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
        "{GTAG}: [hzb_skin] 契约链就绪 contract_digest={} g10 转引一致性=pass all_visible_arm={all_visible_arm}",
        contract.digest
    );

    // ③ 场景装配（B1 节点分组 + B4 UV sink 双记录面同装配一次产出——G34-2
    //    同律;剔除对象粒度 = 静态逐 mesh 节点,动态/角色尾槽恒可见）。
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
        fail("HZB×蒙皮面场景零可剔除实例（节点分组为空,fail-closed 不冒充）");
    }

    // ③.5 slab 侧表生产接线（main ③.5 逐字同律）。
    let (slab_report, slab_arm): (
        Option<(SlabSideTableAsset, SlabEval, usize)>,
        Option<[f32; SLAB_N_SLOTS]>,
    ) = {
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
            "{GTAG}: [hzb_skin] slab 接线 arm=device slots=16 mapped_mats={} slab_tris={} parity_p100={:.6e} eval_ms={:.3} abi={}",
            asset.material_slots.len(),
            n_slab,
            eval.parity_p100,
            eval.eval_ms,
            asset.abi_digest,
        );
        (Some((asset, eval, n_slab)), Some(arm_r))
    };

    // ③.6 纹理采样生产接线（main ③.6 逐字同律 + texmeta mod × R_slot 预调制）。
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
        let premod = if let (Some(asset_eval), Some(arm_r)) = (slab_report.as_ref(), slab_arm.as_ref())
        {
            g34_slab_premod_texmeta(&mut assets, &asset_eval.0, arm_r)
        } else {
            0
        };
        eprintln!(
            "{GTAG}: [hzb_skin] B4 纹理接线 mapped={} tex_tris={} atlas={}x{} probes={} ssbo_p100={:.6e}（位级={} 双跑={}） sampler_max_lsb={} nonconstant_slots={} eval_ms={:.3} slab_premod_slots={}",
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
        "{GTAG}: [hzb_skin] 装配 scene={scene_id} tris={} quads={} points={} nodes={} output={out_w}x{out_h} eps={eps:.6} features=[tex=true slab=true dyn=true hzb=true skin=true]",
        scene.tri_count,
        scene.quads.len(),
        scene.points.len(),
        hzb_groups.len(),
    );

    // ③.7 环境面（三态裁决同律——无设备即 skip）。
    let caps = match rurix_rt::render_exec::probe_device_caps() {
        Ok(c) => c,
        Err(e) => dev_env_or_fail("device_caps", &e),
    };

    // ④ 真窗口 present 会话（--hzb on --skin 闭集已拒 headless ⇒ 窗口必建）。
    let mut window = match vk::ExternalImagePresent::create(
        out_w,
        out_h,
        "rurix g34 unified lane + hzb + skin (bistro-interior 1080p;G37 W3 五特性同开;ESC 退出)",
        !hidden,
    ) {
        Ok(w) => w,
        Err(e) => dev_env_or_fail("window_present", &e),
    };
    let bgra = window.channel_order() == "bgra8_unorm";
    eprintln!(
        "{GTAG}: [hzb_skin] 窗口就绪 {}x{} channel_order={} visible={}",
        window.extent().0,
        window.extent().1,
        window.channel_order(),
        !hidden
    );

    // ⑤ 初态（相机 = 契约位姿;fork B 动态轨迹原点 + B5 蒙皮角色原点）。
    let cam0 = G34Camera::from_spec(&scene.camera);
    let mut cam = cam0;
    let ev100 = f64::from(scene.ev100);
    let jitter_base = (seed % JITTER_WINDOW_MOD) as u32;
    let skin_org = skin_origin(&scene.camera);
    let dyn_origin = dyn_trajectory_origin(&scene.camera);
    let cube_tris_host = dyn_cube_tris(DYN_CUBE_HALF);

    // ⑥ era 循环状态（G34-2 同律 + 蒙皮核验面）。
    let total = warmup + frames;
    let mut fi = 0u32;
    let mut exit_reason = "frames_done";
    let mut resize_eras = 0u32;
    let mut render_ms: Vec<f64> = Vec::new();
    let mut present_ms: Vec<f64> = Vec::new();
    let mut digest_ms: Vec<f64> = Vec::new();
    let mut encode_gpu_ms: Vec<f64> = Vec::new();
    let mut scene_gpu_ms: Vec<f64> = Vec::new();
    let mut skin_gpu_ms: Vec<f64> = Vec::new();
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
    let mut dyn_verify_recs: Vec<DynVerifyFrame> = Vec::new();
    let mut skin_verify_recs: Vec<G34SkinVerifyFrame> = Vec::new();
    let mut hzb_tested: u64 = 0;
    let mut hzb_occluded: u64 = 0;
    let mut hzb_offscreen: u64 = 0;
    let mut hzb_retested: u64 = 0;
    let mut hzb_flipped: u64 = 0;
    let mut hzb_visible_sum: u64 = 0;
    let mut hzb_closure_frames: u64 = 0;
    let mut hzb_closure_submits: u64 = 0;
    let mut hzb_fallbacks: u64 = 0;
    let probe_fi = warmup.max(1);
    let mut hzb_pre_data: Option<(Vec<f32>, Vec<f32>)> = None;
    let mut hzb_parity: Option<(G34HzbWiredParity, u32)> = None;
    // era 创建期恒先赋值（'eras 为 loop 至少一轮,出口全在赋值后 ⇒ 定赋值
    // 分析成立,免初值——G34-2 同律）。
    let mut hzb_levels_meta: Vec<(u32, u32)>;
    let mut hzb_flat_offsets_meta: Vec<u32>;
    let mut hzb_meta_json: String;
    let mut char_tri_base_meta: usize;
    'eras: loop {
        let (ew, eh) = window.extent();
        let in_w = ((ew as u64 * u64::from(tier)) / 100).max(1) as u32;
        let in_h = ((eh as u64 * u64::from(tier)) / 100).max(1) as u32;
        // ── era 资产（三分区合并 SSBO:静态汤 + 动态立方体 + 蒙皮角色——
        //    g34skin_assets 逐字复用;静态段与节点分组位级同 buffer）──
        let assets = g34skin_assets(&scene, in_w, in_h, skin_org);
        let dyn_tri_base = assets.dyn_tri_base;
        char_tri_base_meta = assets.char_tri_base;
        // UnifiedLaneBits:mv 位装 g34_unified_mv（G34-3 独消费变体,load 内建
        // NoContraction 注入面继承）;scene 位 = 统一 mega kernel 装配期借用
        // （mega pass 不进本车道零消费,G34-2 同律）。
        let bits = UnifiedLaneBits::load(
            &spv_scene,
            &spv_skin_mv,
            &spv_resample,
            &spv_resolve,
            in_w,
            in_h,
            ew,
            eh,
            false,
        );
        // 蒙皮 SPV（B5 同律 NoContraction 注入——① 逐顶点位级对拍前提）。
        let skin_words = spv_inject_no_contraction(&load_spv(&spv_skin));
        let skin_spv_bytes: Vec<u8> = skin_words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let skin_dispatch = [assets.character.vertex_count as u32, 1, 1];
        let enc_words = load_spv(&spv_encode);
        let (ex, ey, _) = spv_local_size(&enc_words);
        let enc_dispatch = [ew.div_ceil(ex), eh.div_ceil(ey), 1];
        let enc_spv_bytes: Vec<u8> = enc_words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let enc_params = aces13_device_encode_params(ew, eh, bgra);
        let enc_params_bytes = bytes_f32(&enc_params);
        // 纹理侧表（三分区总三角数;动态/角色段 tritex −1 常量面）。
        let total_tris = assets.char_tri_base + assets.character.tri_count;
        let side = {
            let (t, _) = tex_report.as_ref().unwrap_or_else(|| {
                fail("HZB×蒙皮面缺 B4 纹理报告（--full 闭集破缺,内部防御性复核）")
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
        // ── HZB era 常量面（primary 位装 primary_skin——G37 W3 合并主射线腿;
        //    inst_base/实例计数带角色尾槽;pass 名字面改叙,遥测/评注同名）──
        let mut hz = G34HzbBits::load(
            &spv_primary_skin,
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
            Some(assets.char_tri_base),
        );
        hz.name_primary = "g34_unified_primary_skin".to_owned();
        hzb_levels_meta = hz.levels.clone();
        hzb_flat_offsets_meta = hz.flat_offsets.clone();
        {
            let dims: Vec<String> = hz
                .levels
                .iter()
                .map(|&(w, h)| format!("[{w},{h}]"))
                .collect();
            hzb_meta_json = format!(
                "{{\"instances\":{},\"static_nodes\":{},\"dyn_tail_slot\":1,\"char_tail_slot\":1,\"levels\":{},\"level_dims\":[{}],\"flat_texels\":{},\"conv\":\"standard_z\"}}",
                hzb_groups.len() + 2,
                hzb_groups.len(),
                hz.levels.len(),
                dims.join(","),
                hz.flat_texels
            );
        }
        // ── 描述组（G34Full 27 SSBO 四 pass 解构 + HZB 追加面 + 蒙皮七件）──
        let g34_tuple = unified_lane_descs_g34(&assets.base, &bits, &side, in_w, in_h, ew, eh);
        let (resources, passes, barriers, readbacks, ids) = g34hs_descs(
            g34_tuple,
            &enc_spv_bytes,
            enc_dispatch,
            &enc_params_bytes,
            &hz,
            &bits,
            &assets,
            &skin_spv_bytes,
            skin_dispatch,
            hzb_groups.len() + 2,
            in_w,
            in_h,
            ew,
            eh,
        );
        let bar_refs: Vec<&[(u32, TargetState)]> =
            barriers.iter().map(|b| b.as_slice()).collect();
        // ── BLAS 分解 + 双 TLAS（表 0 = 初剔后,表 1 = 全量;节点段 + 动态局部
        //    段 + 角色绑定姿态尾 BLAS——角色两副本创建期 updatable 打标,双桥
        //    refit 消费面）──
        let mut blas_refs: Vec<&[f32]> = hzb_groups
            .iter()
            .map(|g| {
                let lo = g.tri_offset as usize * 9;
                &assets.base.tris[lo..lo + g.tri_count as usize * 9]
            })
            .collect();
        blas_refs.push(&assets.base.tris[dyn_tri_base * 9..assets.char_tri_base * 9]);
        blas_refs.push(&assets.character.rest_tris);
        let n_inst = hzb_groups.len() + 2;
        let hs_insts: Vec<RayQueryInstanceDesc> = (0..n_inst as u32)
            .map(|i| RayQueryInstanceDesc {
                blas: i,
                custom_index: i,
                mask: 0xFF,
                sbt_record_offset: 0,
            })
            .collect();
        let updatable: [u32; 1] = [(hzb_groups.len() + 1) as u32];
        let hs_accel = [
            AccelStructDesc {
                scene: RayQuerySceneDesc {
                    blas_triangles: &blas_refs,
                    instances: &hs_insts,
                },
                transforms: None,
                updatable_blas: &updatable,
            },
            AccelStructDesc {
                scene: RayQuerySceneDesc {
                    blas_triangles: &blas_refs,
                    instances: &hs_insts,
                },
                transforms: None,
                updatable_blas: &updatable,
            },
        ];
        let mut lane = match G34HsLane::create(
            &resources,
            &passes,
            &bar_refs,
            &readbacks,
            &hs_accel,
            ids,
            hzb_groups.clone(),
            dyn_tri_base,
            assets.char_tri_base,
            assets.character.tri_count,
            hz.levels.len(),
        ) {
            Ok(l) => l,
            Err(e) => dev_env_or_fail("device_lane", &e),
        };
        eprintln!(
            "{GTAG}: [hzb_skin] era 就绪 extent={ew}x{eh} internal={in_w}x{in_h}（车道:g31_skin→[blas_refit 双桥]→g34_unified_primary_skin→g34_unified_shade→g34_unified_mv→tsr×2→display_encode→test_p1→reduce×{}+pack×{}→test_p2;instances={}〔静态节点 {} + 动态尾槽 1 + 角色尾槽 1〕 mips={} flat_texels={} skin_tris={} skin_verts={} char_tri_base={};resize_eras={resize_eras}）",
            hz.levels.len() - 1,
            hz.levels.len(),
            n_inst,
            hzb_groups.len(),
            hz.levels.len(),
            hz.flat_texels,
            assets.character.tri_count,
            assets.character.vertex_count,
            assets.char_tri_base,
        );
        let char_inst_f = (hzb_groups.len() + 1) as f32;
        let dyn_inst_f = hzb_groups.len() as f32;
        let mut resized = false;
        let mut era_first = true;
        let mut prev_pal: Option<[BoneTransform; 3]> = None;
        let mut prev_vp_host: Option<Mat4> = None;
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
            // ── 相机（auto-move 确定性轨迹）──
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
            // ── 逐帧面:fork B 动态变换 + B5 palette 双表 + 场景/蒙皮参数
            //    （[42]=dyn_tri_base;[43]/[44] 恒 0——合并车道角色分派走
            //    inst_base 前缀和表,gi_skin 参数扩面不消费如实登记）──
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
            let pal = skin_palette(fi, skin_org);
            let prev_p = prev_pal.unwrap_or(pal);
            let skin_params = pack_skin_params(
                assets.character.vertex_count,
                prev_pal.is_some(),
                assets.char_tri_base,
                assets.character.bone_count,
            );
            let last = fi + 1 == total;
            let verify = fi >= 1 && fi >= warmup && (fi - warmup) % DYN_VERIFY_EVERY == 0;
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
                &skin_params,
                &skin_palette_bytes(&pal),
                &skin_palette_bytes(&prev_p),
                j,
                &vp,
                &vp_j,
                exposure,
                reset,
                last,
                verify,
                true,
                hzb_pre_frame,
                xf,
            ) {
                Ok(r) => r,
                Err(e) => fail(&format!("帧 {fi} HZB×蒙皮车道: {e}")),
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

            // ── HZB 逐帧决策面记账 + probe 两帧成对接线态对拍（G34-2 逐字）──
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
                        let _ = std::fs::create_dir_all(".tmp/g34_gates/hzb_skin");
                        let dump = |name: &str, v: &[f32]| {
                            let b: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
                            let _ = std::fs::write(format!(".tmp/g34_gates/hzb_skin/{name}"), &b);
                        };
                        dump("probe_depth.bin", d);
                        dump("probe_flat.bin", f);
                        eprintln!(
                            "{GTAG}: [hzb_skin] probe 现场 dump → .tmp/g34_gates/hzb_skin/probe_{{depth,flat}}.bin"
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
                        "{GTAG}: [hzb_skin] 帧 {} 接线态对拍 mips={} 位级全等 + p1 判定 {} rect 逐字节全等 + 零假阳性（剔除 {}）+ digest {}",
                        fi + 1,
                        wp.mips,
                        wp.n_rects,
                        wp.occluded,
                        &wp.verdict_digest[..23]
                    );
                    hzb_parity = Some((wp, fi));
                }
            }

            // ── 核验帧三面（G34-2 dyn 位置核验 + G34-3 蒙皮三面——同帧同回读）──
            if verify {
                let scene_color = rec
                    .scene_color
                    .as_ref()
                    .unwrap_or_else(|| fail("帧核验面缺 scene color 回读（内部破缺）"));
                let mv_plane = rec
                    .mv_out
                    .as_ref()
                    .unwrap_or_else(|| fail("帧核验面缺 mv 回读（内部破缺）"));
                let depth_plane = rec
                    .scene_depth
                    .as_ref()
                    .unwrap_or_else(|| fail("帧核验面缺 scene depth 回读（内部破缺）"));
                let hit_plane = rec
                    .hit
                    .as_ref()
                    .unwrap_or_else(|| fail("帧核验面缺 hit 回读（内部破缺）"));
                let char_dev = rec
                    .char_tris
                    .as_ref()
                    .unwrap_or_else(|| fail("帧核验面缺蒙皮顶点回读（内部破缺）"));

                // ── fork B 动态实例位置核验（G34-2 同律:域界式质心容差）──
                {
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
                    let pred_diag = pred_area.sqrt();
                    let tol_centroid = if pred_diag <= 100.0 {
                        DYN_TOL_CENTROID_PX
                    } else {
                        DYN_TOL_CENTROID_PX.max(0.05 * pred_diag)
                    };
                    let pass = obs_count >= min_count
                        && centroid_delta <= tol_centroid
                        && aabb_delta <= DYN_TOL_AABB_PX;
                    dyn_verify_recs.push(DynVerifyFrame {
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

                // ── 蒙皮核验三面（G34-3 逐字同律;实例号 = 合并分解——
                //    char_inst=(N+1) / dyn_inst=N 参数化）──
                let host_cur = skin_host_verts(&assets.character, &pal);
                let host_prev_pos = skin_host_verts(&assets.character, &prev_p);
                let mut vertex_max_abs = 0.0f64;
                for (vi, hv) in host_cur.iter().enumerate() {
                    for c in 0..3 {
                        let d = (f64::from(char_dev[vi * 3 + c]) - f64::from(hv[c])).abs();
                        if d > vertex_max_abs {
                            vertex_max_abs = d;
                        }
                    }
                }
                let prev_vp_h = prev_vp_host.unwrap_or(vp_j);
                let mut host_mv: Vec<[f64; 2]> =
                    Vec::with_capacity(assets.character.vertex_count);
                for k in 0..assets.character.vertex_count {
                    let (u, v) = dyn_project(&vp_j, host_cur[k], in_w, in_h)
                        .unwrap_or_else(|| fail("蒙皮顶点投影在相机背面（动画规格破缺）"));
                    let (pu, pv) = dyn_project(&prev_vp_h, host_prev_pos[k], in_w, in_h)
                        .unwrap_or_else(|| fail("蒙皮 prev 顶点投影在相机背面（动画规格破缺）"));
                    host_mv.push([pu - u, pv - v]);
                    let _ = (u, v);
                }
                let (pred_cx, pred_cy, pred_aabb, pred_mask_count) = skin_pred_mask(
                    &host_cur,
                    assets.character.tri_count,
                    &vp_j,
                    in_w,
                    in_h,
                )
                .unwrap_or_else(|| fail("蒙皮掩码投影为空（动画规格破缺）"));
                let pred_c = [pred_cx, pred_cy];
                let obs = skin_detect_hit(hit_plane, in_w, in_h, char_inst_f);
                let (obs_px, obs_aabb, obs_count, obs_idx) = match obs {
                    Some((cx, cy, bb, n, idx)) => ([cx, cy], bb, n, idx),
                    None => ([f64::NAN; 2], [f64::NAN; 4], 0, Vec::new()),
                };
                let centroid_delta = if obs_count > 0 {
                    ((obs_px[0] - pred_c[0]).powi(2) + (obs_px[1] - pred_c[1]).powi(2)).sqrt()
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
                let min_count = 200.0f64.max(0.75 * pred_mask_count as f64) as usize;
                let (fw, fh) = (in_w as f64, in_h as f64);
                let mut dx: Vec<f64> = obs_idx
                    .iter()
                    .map(|&pi| f64::from(mv_plane[pi as usize * 2]) * fw)
                    .collect();
                let mut dy: Vec<f64> = obs_idx
                    .iter()
                    .map(|&pi| f64::from(mv_plane[pi as usize * 2 + 1]) * fh)
                    .collect();
                let dev_med = if dx.is_empty() {
                    [f64::NAN; 2]
                } else {
                    [median_f64(&mut dx), median_f64(&mut dy)]
                };
                let mut dmag: Vec<f64> = obs_idx
                    .iter()
                    .map(|&pi| {
                        let mx = f64::from(mv_plane[pi as usize * 2]) * fw;
                        let my = f64::from(mv_plane[pi as usize * 2 + 1]) * fh;
                        mx.hypot(my)
                    })
                    .collect();
                let mut hx: Vec<f64> = host_mv.iter().map(|m| m[0]).collect();
                let mut hy: Vec<f64> = host_mv.iter().map(|m| m[1]).collect();
                let host_med = [median_f64(&mut hx), median_f64(&mut hy)];
                let mut hmag: Vec<f64> = host_mv.iter().map(|m| m[0].hypot(m[1])).collect();
                let host_motion = median_f64(&mut hmag);
                let dev_motion = if dmag.is_empty() {
                    f64::NAN
                } else {
                    median_f64(&mut dmag)
                };
                let mv_delta = [
                    (dev_med[0] - host_med[0]).abs(),
                    (dev_med[1] - host_med[1]).abs(),
                ];
                let (sx0, sy0, sx1, sy1) = SKIN_STATIC_WIN;
                let inv_cur_host = vp_j
                    .inverse()
                    .unwrap_or_else(|| fail("jittered view-proj 必须可逆（静态 MV 参照面）"));
                let mut sdev_x: Vec<f64> = Vec::new();
                let mut sdev_y: Vec<f64> = Vec::new();
                let mut shost_x: Vec<f64> = Vec::new();
                let mut shost_y: Vec<f64> = Vec::new();
                for py in sy0..sy1 {
                    for px in sx0..sx1 {
                        let pi = (py * in_w + px) as usize;
                        sdev_x.push(f64::from(mv_plane[pi * 2]) * fw);
                        sdev_y.push(f64::from(mv_plane[pi * 2 + 1]) * fh);
                        let hm = g34skin_host_camera_mv(
                            px,
                            py,
                            depth_plane[pi],
                            &inv_cur_host,
                            &prev_vp_h,
                            in_w,
                            in_h,
                        );
                        shost_x.push(hm[0]);
                        shost_y.push(hm[1]);
                    }
                }
                let static_dev = [median_f64(&mut sdev_x), median_f64(&mut sdev_y)];
                let static_host = [median_f64(&mut shost_x), median_f64(&mut shost_y)];
                let static_delta = [
                    (static_dev[0] - static_host[0]).abs(),
                    (static_dev[1] - static_host[1]).abs(),
                ];
                let mut rigid_idx: Vec<u32> = Vec::new();
                for py in 0..in_h {
                    for px in 0..in_w {
                        let pi = (py * in_w + px) as usize;
                        if hit_plane[pi * 4] == dyn_inst_f {
                            rigid_idx.push(pi as u32);
                        }
                    }
                }
                let rigid_count = rigid_idx.len();
                let mut rigid_dev = [f64::NAN; 2];
                let mut rigid_host = [f64::NAN; 2];
                let mut rigid_delta = [f64::INFINITY; 2];
                if rigid_count >= G34S_RIGID_MIN_COUNT {
                    let prev_xf = lane.prev_dyn_xf.unwrap_or(xf);
                    let mut rdx: Vec<f64> = Vec::new();
                    let mut rdy: Vec<f64> = Vec::new();
                    let mut rhx: Vec<f64> = Vec::new();
                    let mut rhy: Vec<f64> = Vec::new();
                    for &pi in &rigid_idx {
                        let px = pi % in_w;
                        let py = pi / in_w;
                        let prim = hit_plane[pi as usize * 4 + 1] as usize;
                        let bu = hit_plane[pi as usize * 4 + 2];
                        let bv = hit_plane[pi as usize * 4 + 3];
                        if let Some(hm) = g34skin_host_rigid_mv(
                            px,
                            py,
                            prim,
                            bu,
                            bv,
                            &cube_tris_host,
                            &prev_xf,
                            &xf,
                            &vp_j,
                            &prev_vp_h,
                            in_w,
                            in_h,
                        ) {
                            rdx.push(f64::from(mv_plane[pi as usize * 2]) * fw);
                            rdy.push(f64::from(mv_plane[pi as usize * 2 + 1]) * fh);
                            rhx.push(hm[0]);
                            rhy.push(hm[1]);
                        }
                    }
                    if !rdx.is_empty() {
                        rigid_dev = [median_f64(&mut rdx), median_f64(&mut rdy)];
                        rigid_host = [median_f64(&mut rhx), median_f64(&mut rhy)];
                        rigid_delta = [
                            (rigid_dev[0] - rigid_host[0]).abs(),
                            (rigid_dev[1] - rigid_host[1]).abs(),
                        ];
                    }
                }
                let rigid_gate_active = rigid_count >= G34S_RIGID_MIN_COUNT
                    && rigid_delta[0].is_finite()
                    && rigid_delta[1].is_finite();
                let pass = vertex_max_abs == 0.0
                    && obs_count >= min_count
                    && centroid_delta <= SKIN_TOL_CENTROID_PX
                    && aabb_delta <= SKIN_TOL_AABB_PX
                    && mv_delta[0] <= SKIN_MV_TOL_MEDIAN_PX
                    && mv_delta[1] <= SKIN_MV_TOL_MEDIAN_PX
                    && (host_motion < SKIN_MV_HOST_MOTION_MIN_PX
                        || dev_motion >= SKIN_MV_DEV_RATIO_MIN * host_motion)
                    && static_delta[0] <= G34S_STATIC_MV_TOL_PX
                    && static_delta[1] <= G34S_STATIC_MV_TOL_PX
                    && (!rigid_gate_active
                        || (rigid_delta[0] <= G34S_RIGID_MV_TOL_PX
                            && rigid_delta[1] <= G34S_RIGID_MV_TOL_PX));
                skin_verify_recs.push(G34SkinVerifyFrame {
                    frame: fi,
                    vertex_max_abs,
                    pred_px: pred_c,
                    pred_aabb,
                    obs_px,
                    obs_aabb,
                    obs_count,
                    centroid_delta_px: centroid_delta,
                    aabb_delta_px: aabb_delta,
                    mv_dev_median_px: dev_med,
                    mv_host_median_px: host_med,
                    mv_median_delta_px: mv_delta,
                    mv_host_motion_px: host_motion,
                    mv_dev_motion_px: dev_motion,
                    static_mv_dev_px: static_dev,
                    static_mv_host_px: static_host,
                    static_mv_delta_px: static_delta,
                    rigid_count,
                    rigid_mv_dev_px: rigid_dev,
                    rigid_mv_host_px: rigid_host,
                    rigid_mv_delta_px: rigid_delta,
                    pass,
                });
                if !pass {
                    fail(&format!(
                        "帧 {fi} 蒙皮核验 fail（vertex_max_abs={vertex_max_abs:.3e} obs_count={obs_count}（min {min_count}）centroid_Δ={centroid_delta:.3}px aabb_Δ={aabb_delta:.3}px mv_Δ=[{:.3},{:.3}]px static_Δ=[{:.3},{:.3}]px rigid_Δ=[{:.3},{:.3}]px（n={rigid_count}））",
                        mv_delta[0], mv_delta[1], static_delta[0], static_delta[1],
                        rigid_delta[0], rigid_delta[1],
                    ));
                }
            }
            prev_pal = Some(pal);
            prev_vp_host = Some(vp_j);

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
                skin_gpu_ms.push(rec.skin_gpu_ns / 1e6);
                prod_gpu_ms.push(
                    (rec.skin_gpu_ns
                        + rec.scene_gpu_ns
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
                    "{GTAG}: [hzb_skin] 帧 {}/{total} render={render_el:.3}ms(gpu_skin={:.4}ms gpu_scene={:.3}ms gpu_hzb={:.4}ms gpu_encode={:.3}ms) tested={} occluded={} offscreen={} flip={} closure_extra={}{} visible={} present={pres_el:.3}ms",
                    fi + 1,
                    rec.skin_gpu_ns / 1e6,
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

    // ⑦ 核验汇总 + 多口径稳态统计 + evidence（证据保全先于判红）。
    let frames_done = fi;
    let motion_max = skin_verify_recs
        .iter()
        .map(|r| r.mv_host_motion_px)
        .fold(0.0f64, f64::max);
    let rigid_active_frames = skin_verify_recs
        .iter()
        .filter(|r| r.rigid_count >= G34S_RIGID_MIN_COUNT && r.rigid_mv_delta_px[0].is_finite())
        .count();
    let vertex_max_all = skin_verify_recs
        .iter()
        .map(|r| r.vertex_max_abs)
        .fold(0.0f64, f64::max);
    let skin_all_pass = !skin_verify_recs.is_empty()
        && skin_verify_recs.iter().all(|r| r.pass)
        && motion_max >= SKIN_MV_HOST_MOTION_MIN_PX
        && rigid_active_frames >= 1;
    let dyn_all_pass = !dyn_verify_recs.is_empty() && dyn_verify_recs.iter().all(|v| v.pass);
    eprintln!(
        "{GTAG}: [hzb_skin] 蒙皮核验 {}/{} 帧通过（① vertex_max_abs={vertex_max_all:.3e}（位级门 ==0）② 质心 ≤{SKIN_TOL_CENTROID_PX}px AABB ≤{SKIN_TOL_AABB_PX}px ③ MV 中位差 ≤{SKIN_MV_TOL_MEDIAN_PX}px 窗级真动 max={motion_max:.3}px ≥{SKIN_MV_HOST_MOTION_MIN_PX}px 类2刚性激活帧={rigid_active_frames}）;dyn 核验 {}/{} 帧通过",
        skin_verify_recs.iter().filter(|r| r.pass).count(),
        skin_verify_recs.len(),
        dyn_verify_recs.iter().filter(|v| v.pass).count(),
        dyn_verify_recs.len(),
    );
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
    let (sk_mean, sk_min) = if skin_gpu_ms.is_empty() {
        (0.0, 0.0)
    } else {
        let (a, _, _, d, _) = g34_stats(&skin_gpu_ms);
        (a, d)
    };
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
    let spv_sha = |p: &str| {
        format!(
            "{{\"path\":{},\"sha256\":{}}}",
            jstr(&p.replace('\\', "/")),
            jstr(&g34_file_sha(p).unwrap_or_else(|e| fail(&e)))
        )
    };

    let features_json =
        "{\"textures\":true,\"slab\":true,\"dyn\":true,\"full\":true,\"static_camera\":false,\"hzb\":true,\"skin\":true}"
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
        for (k, v) in dyn_verify_recs.iter().enumerate() {
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
            dyn_verify_recs.len(),
            dyn_all_pass,
        )
    };
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
        hzb_groups.len() + 2,
        hzb_levels_meta.len(),
    );
    // skin 块（G34-3 evidence 同形——门脚本 skin 口径判读面;实例号/BLAS 下标
    // = 合并分解字面）。
    let skin_json = {
        let jf = |v: f64| -> String {
            if v.is_finite() {
                format!("{v:.6}")
            } else {
                "\"inf\"".to_owned()
            }
        };
        let mut rows = String::new();
        for (ri, r) in skin_verify_recs.iter().enumerate() {
            if ri > 0 {
                rows.push(',');
            }
            rows.push_str(&format!(
                "{{\"frame\":{},\"vertex_max_abs\":{},\"pred_px\":[{},{}],\"pred_aabb\":[{},{},{},{}],\"obs_px\":[{},{}],\"obs_aabb\":[{},{},{},{}],\"obs_count\":{},\"centroid_delta_px\":{},\"aabb_delta_px\":{},\"mv_dev_median_px\":[{},{}],\"mv_host_median_px\":[{},{}],\"mv_median_delta_px\":[{},{}],\"mv_host_motion_px\":{},\"mv_dev_motion_px\":{},\"static_mv_dev_px\":[{},{}],\"static_mv_host_px\":[{},{}],\"static_mv_delta_px\":[{},{}],\"rigid_count\":{},\"rigid_mv_dev_px\":[{},{}],\"rigid_mv_host_px\":[{},{}],\"rigid_mv_delta_px\":[{},{}],\"pass\":{}}}",
                r.frame,
                jf(r.vertex_max_abs),
                jf(r.pred_px[0]), jf(r.pred_px[1]),
                jf(r.pred_aabb[0]), jf(r.pred_aabb[1]), jf(r.pred_aabb[2]), jf(r.pred_aabb[3]),
                jf(r.obs_px[0]), jf(r.obs_px[1]),
                jf(r.obs_aabb[0]), jf(r.obs_aabb[1]), jf(r.obs_aabb[2]), jf(r.obs_aabb[3]),
                r.obs_count,
                jf(r.centroid_delta_px),
                jf(r.aabb_delta_px),
                jf(r.mv_dev_median_px[0]), jf(r.mv_dev_median_px[1]),
                jf(r.mv_host_median_px[0]), jf(r.mv_host_median_px[1]),
                jf(r.mv_median_delta_px[0]), jf(r.mv_median_delta_px[1]),
                jf(r.mv_host_motion_px),
                jf(r.mv_dev_motion_px),
                jf(r.static_mv_dev_px[0]), jf(r.static_mv_dev_px[1]),
                jf(r.static_mv_host_px[0]), jf(r.static_mv_host_px[1]),
                jf(r.static_mv_delta_px[0]), jf(r.static_mv_delta_px[1]),
                r.rigid_count,
                jf(r.rigid_mv_dev_px[0]), jf(r.rigid_mv_dev_px[1]),
                jf(r.rigid_mv_host_px[0]), jf(r.rigid_mv_host_px[1]),
                jf(r.rigid_mv_delta_px[0]), jf(r.rigid_mv_delta_px[1]),
                r.pass,
            ));
        }
        let rigid_delta_max = skin_verify_recs
            .iter()
            .filter(|r| r.rigid_mv_delta_px[0].is_finite())
            .map(|r| r.rigid_mv_delta_px[0].max(r.rigid_mv_delta_px[1]))
            .fold(0.0f64, f64::max);
        let static_delta_max = skin_verify_recs
            .iter()
            .map(|r| r.static_mv_delta_px[0].max(r.static_mv_delta_px[1]))
            .fold(0.0f64, f64::max);
        let char_delta_max = skin_verify_recs
            .iter()
            .map(|r| r.mv_median_delta_px[0].max(r.mv_median_delta_px[1]))
            .fold(0.0f64, f64::max);
        format!(
            "{{\"character\":{{\"bone_count\":3,\"tri_count\":36,\"vertex_count\":108,\"origin\":[{:.6},{:.6},{:.6}],\"emission\":[{},{},{}],\"albedo\":[{},{},{}],\"char_tri_base\":{},\"char_inst\":{},\"blas_index\":{},\"spv_skin\":{{\"path\":{},\"sha256\":{}}},\"spv_mv\":{{\"path\":{},\"sha256\":{}}}}},\"tolerance\":{{\"vertex_max_abs\":0.0,\"centroid_px\":{},\"aabb_px\":{},\"mv_median_px\":{},\"min_count_ratio\":0.75,\"mv_host_motion_min_px\":{},\"mv_dev_ratio_min\":{},\"static_mv_consistency_px\":{},\"rigid_mv_px\":{},\"rigid_min_count\":{}}},\"vertex_parity\":{{\"frames\":{},\"max_abs_max\":{},\"all_bitexact\":{}}},\"verify_frames\":[{}],\"verify_count\":{},\"all_pass\":{},\"motion_gate\":{{\"host_motion_max_px\":{},\"threshold_px\":{}}},\"mv_gap\":{{\"class1_camera\":\"wired（相机臂 g14_mv 镜像;静态区一致性 ≤2px 核验面）\",\"class2_rigid\":\"wired+verified（g34_unified_mv 刚性臂;hit 通道 inst==dyn_inst 像素 dev/host 中位差核验——合并分解 dyn_inst=N 参数化）\",\"class3_skinned\":\"wired+verified（g31_skin_mv 镜像面:B5 prev 蒙皮顶点 bary 插值臂——char_inst=N+1 参数化）\",\"class1_delta_max_px\":{},\"class2_delta_max_px\":{},\"class3_delta_max_px\":{},\"rigid_active_frames\":{},\"note\":\"RD-041 三类速度设计在 G37 W3 合并车道全接线（kernels/g34_unified_mv.rx 0-byte 消费——实例号经参数面下发,kernel 零硬编码）;g14_mv 不进本车道\"}},\"skin_gpu_ms\":{{\"mean\":{:.6},\"min\":{:.6}}}}}",
            skin_org[0], skin_org[1], skin_org[2],
            SKIN_EMISSION[0], SKIN_EMISSION[1], SKIN_EMISSION[2],
            SKIN_ALBEDO[0], SKIN_ALBEDO[1], SKIN_ALBEDO[2],
            char_tri_base_meta,
            hzb_groups.len() + 1,
            hzb_groups.len() + 1,
            jstr(&spv_skin.replace('\\', "/")),
            jstr(&g34_file_sha(&spv_skin).unwrap_or_else(|e| fail(&e))),
            jstr(&spv_skin_mv.replace('\\', "/")),
            jstr(&g34_file_sha(&spv_skin_mv).unwrap_or_else(|e| fail(&e))),
            SKIN_TOL_CENTROID_PX,
            SKIN_TOL_AABB_PX,
            SKIN_MV_TOL_MEDIAN_PX,
            SKIN_MV_HOST_MOTION_MIN_PX,
            SKIN_MV_DEV_RATIO_MIN,
            G34S_STATIC_MV_TOL_PX,
            G34S_RIGID_MV_TOL_PX,
            G34S_RIGID_MIN_COUNT,
            skin_verify_recs.len(),
            jf(vertex_max_all),
            vertex_max_all == 0.0 && !skin_verify_recs.is_empty(),
            rows,
            skin_verify_recs.len(),
            skin_all_pass,
            jf(motion_max),
            SKIN_MV_HOST_MOTION_MIN_PX,
            jf(static_delta_max),
            jf(rigid_delta_max),
            jf(char_delta_max),
            rigid_active_frames,
            sk_mean,
            sk_min,
        )
    };

    let mut ev = String::with_capacity(16384);
    ev.push('{');
    ev.push_str(&format!("\"schema\":{},", jstr(G34HS_SCHEMA)));
    ev.push_str(&format!("\"gate\":{},", jstr(G34HS_GATE)));
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
        ",\"hzb_spv\":{{\"primary_skin\":{},\"shade\":{},\"pack\":{},\"reduce\":{},\"test\":{}}}",
        spv_sha(&spv_primary_skin),
        spv_sha(&spv_hzb_shade),
        spv_sha(&spv_hzb_pack),
        spv_sha(&spv_hzb_reduce),
        spv_sha(&spv_hzb_test)
    ));
    ev.push_str(&format!(
        ",\"skin_spv\":{{\"skin\":{},\"mv\":{}}}",
        spv_sha(&spv_skin),
        spv_sha(&spv_skin_mv)
    ));
    ev.push_str("},");
    ev.push_str("\"render_includes_forced_readback\":true,");
    ev.push_str(&format!(
        "\"spv\":{},",
        unified_provenance_json(&spv_primary_skin, &spv_skin_mv, &spv_resample, &spv_resolve)
    ));
    ev.push_str(&format!("\"features\":{features_json},"));
    ev.push_str(&format!("\"textures\":{textures_json},"));
    ev.push_str(&format!("\"slab\":{slab_json},"));
    ev.push_str(&format!("\"dyn\":{dyn_json},"));
    ev.push_str(&format!("\"hzb\":{hzb_json},"));
    ev.push_str(&format!("\"skin\":{skin_json},"));
    ev.push_str("\"host_parity\":null,");
    ev.push_str(&format!(
        "\"stats\":{{\"render_cv\":{r_cv:.6},\"render_min_ms\":{r_min:.6},\"render_max_ms\":{r_max:.6},\"scene_gpu_ms\":{sg_mean:.6},\"prod_gpu_ms\":{pg_mean:.6},\"skin_gpu_ms\":{sk_mean:.6},\"hzb_aux_gpu_ms\":{hza_mean:.6},\"closure_extra_gpu_ms\":{hzc_mean:.6},\"hzb_host_ms\":{hzh_mean:.6},\"encode_gpu_ms\":{eg_mean:.6},\"present_cv\":{p_cv:.6},\"present_min_ms\":{p_min:.6},\"present_max_ms\":{p_max:.6}}},",
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
        jstr("G37 W3 hzb_skin——HZB×蒙皮同车道合并面（G36 W4-W5 留窗「HZB×蒙皮同车道 = 新 kernel 合并面」兑现件）：HZB 两阶段遮挡剔除×蒙皮×纹理×slab×动态实例五特性同开真窗口生产车道。实例分解 = 静态逐 mesh 节点 BLAS + 动态尾槽 + 蒙皮角色尾槽（两尾槽恒可见不参剔——核验对象面,剔除计数 = 静态节点如实登记）;蒙皮通路 = g31_skin 预 pass 逐顶点 LBS 重写 tris 角色段 + blas_refit 双桥（表 0 = FrameUpdate::blas_refit,表 1 = render_exec G37 W3 加性 blas_refit_b——双 TLAS 双 manager 各持 BLAS 副本须同帧同内容,角色 TLAS 级 AABB 滞后一帧 = G34-3 单表在案语义逐字继承）;主射线腿 = kernels/g34_unified_primary_skin.rx（G34-2 primary 全字面 + out_hit [inst,prim,bu,bv] 加性扩面——母版 0-byte）;shade/reduce/test/pack/g31_skin/g34_unified_mv 六件 0-byte 消费（角色分派经 inst_base 前缀和表,MV 实例号经参数面下发 char_inst=N+1/dyn_inst=N,kernel 零硬编码）;核验 = HZB probe 三面（mips 位级/判定逐字节/零假阳性）∧ 蒙皮三面（① 逐顶点位级 max_abs==0 ② hit inst==char 位置核验 ③ MV 三类）∧ fork B 动态位置核验,全 fail-closed;确定性双跑/剔除像素中性归门脚本裁决。g31_window_present.rs/g34_unified_primary.rx/g34_unified_gi_skin.rx/g27 双 kernel/g31_hzb_pack.rx 0-byte——其门为回归锚。")
    ));
    ev.push('}');

    if evidence_path.is_empty() {
        println!("{ev}");
    } else {
        std::fs::write(&evidence_path, format!("{ev}\n"))
            .unwrap_or_else(|e| fail(&format!("evidence 写 {evidence_path}: {e}")));
        eprintln!("{GTAG}: [hzb_skin] evidence → {evidence_path}");
    }
    if exit_reason == "frames_done" && frames_done > probe_fi && hzb_parity.is_none() {
        fail("HZB 接线态对拍未执行（probe 帧覆盖窗内未完成,内部破缺）");
    }
    if !skin_all_pass {
        fail("蒙皮核验汇总 fail（逐帧门/窗级真动门/类2激活帧数——帧详情见 evidence skin.verify_frames）");
    }
    if !dyn_all_pass {
        fail("动态实例位置核验汇总 fail（帧详情见 evidence dyn.verify_frames）");
    }
    eprintln!(
        "{GTAG}: [hzb_skin] PASS frames={frames_done}/{total} real_render={r_mean:.3}ms present={p_mean:.3}ms tested={hzb_tested} occluded_p1={hzb_occluded} flipped_p2={hzb_flipped} closure_extra={hzb_closure_submits} fallback_frames={hzb_fallbacks} vertex_max_abs={vertex_max_all:.3e} all_visible_arm={all_visible_arm} exit={exit_reason}"
    );
    std::process::exit(0)
}
