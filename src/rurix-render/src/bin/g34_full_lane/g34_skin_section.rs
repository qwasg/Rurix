// Assisted-by: Kimi-K3（G34-3 蒙皮角色进真窗口统一车道）
// G34-3 蒙皮段——独立 include 区段（`g34_full_lane.rs` 尾部 include! 拼接；
// 与 G34-2 HZB 同文件并行分区写零交叠：本段全部符号 `G34S*`/`g34skin*` 前缀
// 自持，主 bin 仅 `--skin` 旗标解析 + 早分支两行挂钩）。
//
// ## 结构（门 `g34.wave2.skin`）
// - 资产面 `G34SkinAssets`：静态场景 + 动态立方体（fork B）+ B5 蒙皮角色
//   三分区合并 tris/mats SSBO；3 BLAS（静态 / 动态立方体局部 / 角色绑定姿
//   态——创建期 updatable 打标）+ 3 实例 TLAS（创建期全 identity）。
// - 描述组 `g34skin_descs`：G34Full 27 SSBO（`unified_lane_descs_g34` 产物
//   解构——resample/resolve 双 pass 原件复用）+ 加性七件蒙皮资源
//   （27=hit 命中信息通道 4f32/px, 28=REST 绑定姿态, 29=WT 权重,
//   30/31=PAL_CUR/PAL_PREV palette 双表逐帧上传, 32=PREV 蒙皮 prev 顶点表,
//   33=SKIN_PARAMS 64B 逐帧）+ encode 两件（34=ACES 参数, 35=BGRA8）= 36
//   资源六 pass 图（g31_skin → [blas_refit 桥] → g34_unified_gi_skin →
//   g34_unified_mv → tsr 双 pass → display_encode）。
// - 车道 `G34SkinLane`：逐帧 tlas_update（3 实例全量表,槽位级 diff 仅动态
//   槽 64B 上传）+ blas_refit 桥（pass0 后 vkCmdCopyBuffer 角色段 → BLAS 2
//   顶点缓冲 + 原地 UPDATE build）同帧共存（render_exec 同槽同律记账面）；
//   顺序入口 inflight=1（FIF 流水面拒 tlas_update/blas_refit,A2 同律）。
// - 核验三面（每 DYN_VERIFY_EVERY 帧,fail-closed）：① 蒙皮 device/host 逐
//   顶点对拍（tris 角色段回读 vs host skin_vertex,max_abs == 0 位级门——
//   B5 在案口径）② 位置核验（host 蒙皮投影掩码 vs hit 通道 inst==2 地面
//   真值检测,质心 ≤4px / AABB ≤6px / 计数 ≥ max(200, 0.75×掩码)——B5 在案
//   口径）③ MV 通道（类 3 蒙皮：dev/host 逐分量中位差 ≤2px + 窗级聚合真动
//   门 + 高动帧条件 ratio 门——B5 在案口径;类 1 静态区相机 MV 一致性 ≤2px
//   ——auto-move 动相机下 B5「静态区 ≤1.5px 绝对门」的诚实重述,登记;
//   类 2 刚性实例：hit 通道 inst==1 像素 dev/host 逐分量中位差 ≤2px——A4
//   登记缺口顺手接通面的核验臂）。
// - MV 缺口推进：kernels/g34_unified_mv.rx = g31_skin_mv 镜像（相机臂 +
//   类 3 蒙皮臂逐字）+ 类 2 刚性实例臂（局部 bary 插值 → prev_dyn_xf 变换
//   → prev_vp 投影）——本车道类 2/类 3 双对象 MV 进 TSR 历史链;非蒙皮腿
//   维持 g14_mv 0-byte + A4 缺口登记（不冒充全局面接通）。

// ---------------------------------------------------------------------------
// G34-3 常量面（SPV 默认路径 / 门键 / schema 字面 / 资源下标）
// ---------------------------------------------------------------------------

/// G34-3 门键（evidence `gate` 字段字面）。
const G34S_GATE: &str = "g34.wave2.skin";
/// G34-3 harness evidence schema 字面（milestones/g34/
/// g34_skin_unified_evidence_schema.json 同字面）。
const G34S_SCHEMA: &str = "rurix.g34.skin_unified_evidence.v1";
/// 蒙皮 compute kernel 默认 SPV（kernels/g31_skin.rx 0-byte 复用——B5 在案
/// 蒙皮 LBS 求值面;CI 门脚本保障编译进 G34-3 隔离目录）。
const G34S_DEFAULT_SPV_SKIN: &str = ".tmp/g34_gates/skin/g31_skin.spv";
/// 蒙皮场景 kernel 默认 SPV（kernels/g34_unified_gi_skin.rx——G34-1 统一
/// kernel + out_hit 命中信息通道 + 角色实例分派扩面）。
const G34S_DEFAULT_SPV_SCENE: &str = ".tmp/g34_gates/skin/g34_unified_gi_skin.spv";
/// 统一 MV kernel 默认 SPV（kernels/g34_unified_mv.rx——g31_skin_mv 镜像 +
/// 类 2 刚性实例臂）。
const G34S_DEFAULT_SPV_MV: &str = ".tmp/g34_gates/skin/g34_unified_mv.spv";

/// G34-3 蒙皮区资源下标（G34Full 27 件 0..=26 逐字不动,蒙皮七件 27..=33
/// 加性追加,encode 两件 34/35——非蒙皮车道 27/28 = encode 面互不共享,车道
/// 各自独立描述组,下标域内自洽即可）。
const G34S_HIT: u32 = 27;
const G34S_REST: u32 = 28;
const G34S_WT: u32 = 29;
const G34S_PAL_CUR: u32 = 30;
const G34S_PAL_PREV: u32 = 31;
const G34S_PREV: u32 = 32;
const G34S_PARAMS: u32 = 33;
const G34S_ENC_PARAMS: u32 = 34;
const G34S_ENC_OUT: u32 = 35;
/// readback 表下标（0..=4 = G34Full 五件逐字同序：[out A/B, mv, depth,
/// scene];5 = hit 通道,6 = tris 角色段逐顶点对拍面,7 = BGRA8）。
const G34S_RB_HIT: u32 = 5;
const G34S_RB_TRIS: u32 = 6;
const G34S_RB_BGRA: u32 = 7;

/// 蒙皮车道逐 pass 屏障计划（保守 StorageReadWrite 超集逐字声明同律）。
const G34S_PLAN_SKIN: &[(u32, TargetState)] = &[
    (U_TRIS, TargetState::StorageReadWrite),
    (G34S_REST, TargetState::StorageReadWrite),
    (G34S_WT, TargetState::StorageReadWrite),
    (G34S_PAL_CUR, TargetState::StorageReadWrite),
    (G34S_PAL_PREV, TargetState::StorageReadWrite),
    (G34S_PREV, TargetState::StorageReadWrite),
    (G34S_PARAMS, TargetState::StorageReadWrite),
];
const G34S_PLAN_SCENE: &[(u32, TargetState)] = &[
    (U_TRIS, TargetState::StorageReadWrite),
    (U_MATS, TargetState::StorageReadWrite),
    (U_QUADS, TargetState::StorageReadWrite),
    (U_POINTS, TargetState::StorageReadWrite),
    (U_SCENE_PARAMS, TargetState::StorageReadWrite),
    (G34_U_TEX_UV, TargetState::StorageReadWrite),
    (G34_U_TEX_META, TargetState::StorageReadWrite),
    (G34_U_TEX_TRITEX, TargetState::StorageReadWrite),
    (G34_U_TEX_ATLAS, TargetState::StorageReadWrite),
    (G34_U_TEX_LINLUT, TargetState::StorageReadWrite),
    (U_SCENE_COLOR, TargetState::StorageReadWrite),
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
    (G34S_HIT, TargetState::StorageReadWrite),
];
const G34S_PLAN_MV: &[(u32, TargetState)] = &[
    (U_SCENE_DEPTH, TargetState::StorageReadWrite),
    (U_MV_PARAMS, TargetState::StorageReadWrite),
    (U_MV_OUT, TargetState::StorageReadWrite),
    (G34S_HIT, TargetState::StorageReadWrite),
    (G34S_PREV, TargetState::StorageReadWrite),
    (U_TRIS, TargetState::StorageReadWrite),
];
const G34S_PLAN_ENCODE: &[(u32, TargetState)] = &[
    (U_OUT_COLOR[0], TargetState::StorageReadWrite),
    (U_OUT_COLOR[1], TargetState::StorageReadWrite),
    (G34S_ENC_PARAMS, TargetState::StorageReadWrite),
    (G34S_ENC_OUT, TargetState::StorageReadWrite),
];

/// 蒙皮 MV 参数面长度（64 f32：g31_skin_mv 40 f32 面 + 类 2 刚性臂扩面
/// [40..54]——kernels/g34_unified_mv.rx 参数面逐字互锁）。
const G34S_MV_PARAMS_LEN: usize = 64;
/// 蒙皮角色实例下标（3 实例 TLAS：0=静态 / 1=动态立方体 / 2=蒙皮角色）。
const G34S_CHAR_INST: f32 = 2.0;
/// 刚性动态实例下标（类 2 MV 臂分派面）。
const G34S_DYN_INST: f32 = 1.0;
/// 静态区 MV 一致性容差（像素;auto-move 动相机下 = dev 静态区 MV vs host
/// 相机 MV 逐分量中位差——B5 静态相机 ≤1.5px 绝对门的诚实重述,登记口径）。
const G34S_STATIC_MV_TOL_PX: f64 = 2.0;
/// 类 2 刚性 MV 容差（像素;逐分量中位差——类 3 在案 2.0px 口径同值继承）。
const G34S_RIGID_MV_TOL_PX: f64 = 2.0;
/// 类 2 刚性核验像素数下限（核验帧刚性实例可见像素 ≥ 本值才激活逐帧门;
/// 低于 = 遮挡/出画面合法相位,逐帧门放空并如实登记——窗级要求 ≥1 帧激活）。
const G34S_RIGID_MIN_COUNT: usize = 50;

// ---------------------------------------------------------------------------
// G34-3 资产面（三分区合并 SSBO + 蒙皮侧表所有者）
// ---------------------------------------------------------------------------

/// G34-3 蒙皮资产面（静态场景汤 + 动态立方体局部段 + 蒙皮角色追加区;tris
/// 追加区**逐帧被 g31_skin pass 重写**为当帧蒙皮顶点〔创建期初值 = 绑定姿
/// 态,与 BLAS 2 初始 build 输入逐字节同〕;mats 追加区 = 常量材质行〔品红
/// 发射检测唯一谱〕;实例表 3 槽 = 静态 identity + 动态逐帧变换 + 角色恒
/// identity——角色形变全在 BLAS 2 顶点内,TLAS 实例变换对角色零触碰）。
struct G34SkinAssets {
    /// 合并面（tris/mats = 三分区;instances = 3 槽）——`unified_lane_descs_g34`
    /// 直消费形。
    base: LaneAssets,
    /// 蒙皮角色（绑定姿态 + 权重;host 参照核验臂同源）。
    character: SkinCharacter,
    /// 静态场景三角形数（kernel params[42] = dyn_tri_base）。
    dyn_tri_base: usize,
    /// 角色段全局三角形基底（= dyn_tri_base + 12;kernel params[43] /
    /// blas_refit src_offset = char_tri_base × 36B）。
    char_tri_base: usize,
    /// 绑定姿态字节（G34S_REST 创建期一次上传）。
    rest_bytes: Vec<u8>,
    /// 权重字节（8 f32/顶点;G34S_WT 创建期一次上传）。
    wt_bytes: Vec<u8>,
}

/// G34-3 资产装配（确定性纯函数;动态段 = fork B 立方体同式,角色段 = B5
/// skin_character 复用——两追加区互序 = [静态 | 动态立方体 | 蒙皮角色]）。
fn g34skin_assets(scene: &SceneData, iw: u32, ih: u32, origin: [f32; 3]) -> G34SkinAssets {
    let character = skin_character(origin);
    let mut tris = pack_tris(scene);
    let dyn_tri_base = tris.len() / 9;
    let dyn_tris = dyn_cube_tris(DYN_CUBE_HALF);
    tris.extend_from_slice(&dyn_tris);
    let char_tri_base = tris.len() / 9;
    tris.extend_from_slice(&character.rest_tris);
    let mut mats = pack_mats(scene);
    for _ in 0..dyn_tris.len() / 9 {
        // 动态立方体 = 纯发光体 albedo=0（fork B 简化面逐字继承）。
        mats.extend_from_slice(&[0.0, 0.0, 0.0]);
        mats.extend_from_slice(&DYN_EMISSION);
        mats.push(0.0);
        mats.push(0.0);
    }
    for _ in 0..character.tri_count {
        // 蒙皮角色 = 常量材质行（B5 在案：albedo 非零受光件 + 品红发射检测
        // 唯一谱;材质不随蒙皮变——位级常量）。
        mats.extend_from_slice(&SKIN_ALBEDO);
        mats.extend_from_slice(&SKIN_EMISSION);
        mats.push(0.0);
        mats.push(0.0);
    }
    let instances = vec![
        RayQueryInstanceDesc {
            blas: 0,
            custom_index: 0,
            mask: 0xFF,
            sbt_record_offset: 0,
        },
        RayQueryInstanceDesc {
            blas: 1,
            custom_index: 0,
            mask: 0xFF,
            sbt_record_offset: 0,
        },
        RayQueryInstanceDesc {
            blas: 2,
            custom_index: 0,
            mask: 0xFF,
            sbt_record_offset: 0,
        },
    ];
    let rest_bytes = bytes_f32(&character.rest_tris);
    let mut wt_flat: Vec<f32> = Vec::with_capacity(character.vertex_count * 8);
    for row in &character.weights {
        for &(b, _) in row {
            wt_flat.push(b as f32);
        }
        for &(_, w) in row {
            wt_flat.push(w);
        }
    }
    let wt_bytes = bytes_f32(&wt_flat);
    G34SkinAssets {
        base: LaneAssets {
            tris_bytes: bytes_f32(&tris),
            mats_bytes: bytes_f32(&mats),
            quads_bytes: bytes_f32(&pack_quads(scene)),
            points_bytes: bytes_f32(&pack_points(scene)),
            params0_bytes: vec![0u8; DYN_PARAMS_LEN * 4],
            out_color_size: (iw * ih * 12) as u64,
            out_depth_size: (iw * ih * 4) as u64,
            tris,
            instances,
        },
        character,
        dyn_tri_base,
        char_tri_base,
        rest_bytes,
        wt_bytes,
    }
}

/// G34-3 逐帧全量实例表（3 槽：静态 identity + 动态逐帧变换 + 角色恒
/// identity——write_transforms 槽位级 diff 保证仅动态槽 64B 上传;角色槽
/// 内容恒定 ⇒ 影子 diff 恒零触碰）。
fn g34skin_frame_instances(transform: [f32; 12]) -> Vec<RayQueryTransformedInstanceDesc> {
    vec![
        RayQueryTransformedInstanceDesc {
            blas: 0,
            custom_index: 0,
            mask: 0xFF,
            sbt_record_offset: 0,
            transform: vk::RAY_QUERY_IDENTITY_TRANSFORM,
        },
        RayQueryTransformedInstanceDesc {
            blas: 1,
            custom_index: 0,
            mask: 0xFF,
            sbt_record_offset: 0,
            transform,
        },
        RayQueryTransformedInstanceDesc {
            blas: 2,
            custom_index: 0,
            mask: 0xFF,
            sbt_record_offset: 0,
            transform: vk::RAY_QUERY_IDENTITY_TRANSFORM,
        },
    ]
}

// ---------------------------------------------------------------------------
// G34-3 描述组（36 资源 + 六 pass + 8 readback）
// ---------------------------------------------------------------------------

/// G34-3 蒙皮描述组装配：G34Full 27 SSBO 四 pass（`unified_lane_descs_g34`
/// 产物解构——资源逐项移动,resample/resolve 双 pass 原件复用零克隆）+
/// 蒙皮七件资源（27..=33）+ encode 两件（34/35）;pass 序 = g31_skin →
/// g34_unified_gi_skin（13 SSBO + AS;G34Full scene 绑定面 + out_hit）→
/// g34_unified_mv（6 SSBO）→ tsr 双 pass → display_encode;readback 8 件
/// （G34Full 五件逐字同序 + hit + tris 角色段 + BGRA8）。
#[allow(clippy::too_many_arguments)]
fn g34skin_descs<'x>(
    assets: &'x G34SkinAssets,
    bits: &'x UnifiedLaneBits,
    tex: &'x G34TexSideTable,
    skin_spv: &'x [u8],
    skin_dispatch: [u32; 3],
    enc_spv: &'x [u8],
    enc_dispatch: [u32; 3],
    enc_params_bytes: &'x [u8],
    iw: u32,
    ih: u32,
    ow: u32,
    oh: u32,
) -> G34Descs<'x> {
    let ipc = (iw * ih) as u64;
    let opc = (ow * oh) as u64;
    let storage = BufferUsage {
        storage: true,
        ..BufferUsage::default()
    };
    // G34Full 面解构（ResourceDesc/Pass 非 Copy——模式移动;resample/resolve
    // 双 pass 原件复用,scene/mv 两 pass 蒙皮变体重建,readback 前 5 项移动）。
    let (resources_g34, passes_g34, _barriers_g34, readbacks_g34) =
        unified_lane_descs_g34(&assets.base, bits, tex, iw, ih, ow, oh);
    let mut resources = resources_g34.to_vec();
    let [_p_scene, _p_mv, p_resample, p_resolve] = passes_g34;
    let [rb0, rb1, rb2, rb3, rb4] = readbacks_g34;
    // U_MV_PARAMS 扩容置换（G34Full 面 = host_buf(40×4) 静态/mv 参数;G34-3 =
    // 64 f32 = 256B——g34_unified_mv 参数面 [40..54] 刚性臂扩面消费;原位
    // 置换保下标语义,非蒙皮车道 40 f32 面 0-byte 不动）。
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
    // 蒙皮七件（27..=33;hit/prev = DEVICE_LOCAL GPU 链内面,rest/wt 创建期一
    // 次上传,palette 双表/params = host-visible 逐帧覆盖）。
    resources.push(ResourceDesc::Buffer(BufferDesc {
        size: ipc * 16,
        usage: storage,
        data: None,
        device_local: true,
    })); // G34S_HIT
    resources.push(ResourceDesc::Buffer(BufferDesc {
        size: assets.rest_bytes.len() as u64,
        usage: storage,
        data: Some(&assets.rest_bytes),
        device_local: true,
    })); // G34S_REST
    resources.push(ResourceDesc::Buffer(BufferDesc {
        size: assets.wt_bytes.len() as u64,
        usage: storage,
        data: Some(&assets.wt_bytes),
        device_local: true,
    })); // G34S_WT
    resources.push(host_buf(bone_bytes)); // G34S_PAL_CUR（逐帧上传）
    resources.push(host_buf(bone_bytes)); // G34S_PAL_PREV（逐帧上传）
    resources.push(ResourceDesc::Buffer(BufferDesc {
        size: (assets.character.vertex_count * 12) as u64,
        usage: storage,
        data: None,
        device_local: true,
    })); // G34S_PREV（pass0 写,MV 读）
    resources.push(host_buf((SKIN_PARAMS_LEN * 4) as u64)); // G34S_PARAMS（逐帧 64B）
    // encode 两件（34/35;与非蒙皮车道 27/28 面互不共享）。
    resources.push(ResourceDesc::Buffer(BufferDesc {
        size: enc_params_bytes.len() as u64,
        usage: storage,
        data: Some(enc_params_bytes),
        device_local: true,
    })); // G34S_ENC_PARAMS
    resources.push(ResourceDesc::Buffer(BufferDesc {
        size: opc * 4,
        usage: storage,
        data: None,
        device_local: true,
    })); // G34S_ENC_OUT
    let passes = vec![
        Pass::Compute(ComputePass {
            name: "g31_skin",
            spirv: skin_spv,
            entry: None,
            dispatch: DispatchSpec::Direct(skin_dispatch),
            bindings: Bindings {
                // 绑定序 = kernel 签名序:in_rest/in_wt/in_pal_cur/in_pal_prev/
                // params/out_tris/out_prev。
                storage_buffers: vec![
                    G34S_REST,
                    G34S_WT,
                    G34S_PAL_CUR,
                    G34S_PAL_PREV,
                    G34S_PARAMS,
                    U_TRIS,
                    G34S_PREV,
                ],
                ..Bindings::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "g34_unified_gi_skin",
            spirv: &bits.spv_scene,
            entry: None,
            dispatch: DispatchSpec::Direct(bits.scene_dispatch),
            bindings: Bindings {
                accel_structs: vec![0],
                // G34Full scene 绑定面（tris..depth）+ out_hit 第 13 件——
                // 与 kernels/g34_unified_gi_skin.rx 签名序逐字互锁。
                storage_buffers: vec![
                    U_TRIS,
                    U_MATS,
                    U_QUADS,
                    U_POINTS,
                    U_SCENE_PARAMS,
                    G34_U_TEX_UV,
                    G34_U_TEX_META,
                    G34_U_TEX_TRITEX,
                    G34_U_TEX_ATLAS,
                    G34_U_TEX_LINLUT,
                    U_SCENE_COLOR,
                    U_SCENE_DEPTH,
                    G34S_HIT,
                ],
                ..Bindings::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "g34_unified_mv",
            spirv: &bits.spv_mv,
            entry: None,
            dispatch: DispatchSpec::Direct(bits.mv_dispatch),
            bindings: Bindings {
                // 绑定序 = kernel 签名序:in_depth/params/in_hit/in_prev/
                // in_tris/out_mv。
                storage_buffers: vec![
                    U_SCENE_DEPTH,
                    U_MV_PARAMS,
                    G34S_HIT,
                    G34S_PREV,
                    U_TRIS,
                    U_MV_OUT,
                ],
                ..Bindings::default()
            },
        }),
        p_resample,
        p_resolve,
        Pass::Compute(ComputePass {
            name: "g31_display_encode",
            spirv: enc_spv,
            entry: None,
            dispatch: DispatchSpec::Direct(enc_dispatch),
            bindings: Bindings {
                storage_buffers: vec![U_OUT_COLOR[0], G34S_ENC_PARAMS, G34S_ENC_OUT],
                ..Bindings::default()
            },
        }),
    ];
    let barriers = vec![
        G34S_PLAN_SKIN,
        G34S_PLAN_SCENE,
        G34S_PLAN_MV,
        U_PLAN_RESAMPLE,
        U_PLAN_RESOLVE,
        G34S_PLAN_ENCODE,
    ];
    let readbacks = vec![
        rb0,
        rb1,
        rb2,
        rb3,
        rb4,
        // 命中信息通道回读（inst 地面真值检测/类 2 刚性像素分派/取证面）。
        Readback::Buffer {
            res: G34S_HIT,
            offset: 0,
            size: ipc * 16,
        },
        // 蒙皮顶点回读（① 逐顶点 device/host 对拍面——核验帧消费,B5 在案
        // max_abs == 0 位级口径）。
        Readback::Buffer {
            res: U_TRIS,
            offset: (assets.char_tri_base * 36) as u64,
            size: (assets.character.tri_count * 36) as u64,
        },
        Readback::Buffer {
            res: G34S_ENC_OUT,
            offset: 0,
            size: opc * 4,
        },
    ];
    G34Descs {
        resources,
        passes,
        barriers,
        readbacks,
    }
}

// ---------------------------------------------------------------------------
// G34-3 车道状态机（G34TsrLane 同律 + palette 双表/skin params 上传 +
// blas_refit 桥与 tlas_update 同帧共存）
// ---------------------------------------------------------------------------

/// G34-3 一帧产物（六 pass telemetry + 回读七路可选）。
struct G34SkinFrameRec {
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
    scene_depth: Option<Vec<f32>>,
    scene_color: Option<Vec<f32>>,
    hit: Option<Vec<f32>>,
    char_tris: Option<Vec<f32>>,
    readback_convert_ms: f64,
}

struct G34SkinLane<'a> {
    session: DeviceFrameSession<'a>,
    parity: usize,
    has_history_state: bool,
    prev_vp_j: Option<Mat4>,
    /// 上一帧动态实例变换（类 2 刚性 MV 臂 prev_dyn_xf 源;首帧 = 当帧变换
    /// 零 MV 语义,与 has_prev 门同字面）。
    prev_dyn_xf: Option<[f32; 12]>,
    /// 角色段在 tris SSBO 的字节偏移（blas_refit src_offset = char_tri_base
    /// × 36B;创建期由资产面带入,era 内恒定）。
    char_src_offset: u64,
}

impl<'a> G34SkinLane<'a> {
    fn create(
        descs: &'a G34Descs<'a>,
        accel_structs: &[AccelStructDesc<'a>],
        char_tri_base: usize,
    ) -> Result<Self, String> {
        if !vk::vulkan_available() {
            return Err("vulkan loader 不可用".into());
        }
        // frame_slots=2（G34TsrLane 逐字同——顺序全同步口径;tlas_update +
        // blas_refit 同帧走顺序入口,FIF 流水面双拒,A2 同律）。
        let session = DeviceFrameSession::new_with_accel_structs(
            &descs.resources,
            &descs.passes,
            &descs.barriers,
            &descs.readbacks,
            2,
            accel_structs,
        )?;
        Ok(Self {
            session,
            parity: 0,
            has_history_state: false,
            prev_vp_j: None,
            prev_dyn_xf: None,
            char_src_offset: (char_tri_base * 36) as u64,
        })
    }

    /// 一帧：六小件参数上传（scene 240B + mv 256B + tsr 128B + skin 64B +
    /// palette 双表 2×144B）+ tlas_update（3 实例表,动态槽变换）+ blas_refit
    /// 桥（角色段 → BLAS 2 UPDATE）→ 六 pass GPU 链内执行 → 可选回读。
    /// readback 子集（下标升序 = 解析序）：[rb_out ⇒ out(p)] + [rb_verify ⇒
    /// mv(2)/depth(3)/scene(4)/hit(5)/tris(6)] + [rb_bgra ⇒ bgra(7)]。
    #[allow(clippy::too_many_arguments)]
    fn frame(
        &mut self,
        iw: u32,
        ih: u32,
        ow: u32,
        oh: u32,
        jitter: [f32; 2],
        vp_j: &Mat4,
        exposure: f32,
        reset: bool,
        scene_params: Vec<f32>,
        skin_params: Vec<f32>,
        pal_cur_bytes: Vec<u8>,
        pal_prev_bytes: Vec<u8>,
        dyn_xf: [f32; 12],
        dyn_tri_base: usize,
        char_tri_count: usize,
        rb_out: bool,
        rb_verify: bool,
        rb_bgra: bool,
    ) -> Result<G34SkinFrameRec, String> {
        // mv 参数面（64 f32）：前 40 = g31_skin_mv 逐字面（[35]=char_inst）;
        // [40..52] = prev_dyn_xf（上一帧动态实例变换,类 2 刚性臂消费;首帧 =
        // 当帧变换零 MV 语义）;[52] = dyn_tri_base;[53] = dyn_inst。
        let inv_cur = vp_j
            .inverse()
            .ok_or("jittered view-proj 必须可逆（mv 参数面）")?;
        let prev = self.prev_vp_j.unwrap_or(*vp_j);
        let mut mv_params = pack_mv_params(iw, ih, &inv_cur, &prev, self.prev_vp_j.is_some());
        mv_params[35] = G34S_CHAR_INST;
        let prev_xf = self.prev_dyn_xf.unwrap_or(dyn_xf);
        mv_params.resize(G34S_MV_PARAMS_LEN, 0.0);
        for k in 0..12 {
            mv_params[40 + k] = prev_xf[k];
        }
        mv_params[52] = dyn_tri_base as f32;
        mv_params[53] = G34S_DYN_INST;
        let has_history = !reset && self.has_history_state;
        let tsr_params = pack_tsr_params(iw, ih, ow, oh, jitter, exposure, has_history, false);
        let p = self.parity;
        let uploads: Vec<(StableResourceId, u64, Vec<u8>)> = vec![
            (
                StableResourceId(u64::from(U_SCENE_PARAMS) + 1),
                0,
                bytes_f32(&scene_params),
            ),
            (
                StableResourceId(u64::from(U_MV_PARAMS) + 1),
                0,
                bytes_f32(&mv_params),
            ),
            (
                StableResourceId(u64::from(U_TSR_PARAMS) + 1),
                0,
                bytes_f32(&tsr_params),
            ),
            (
                StableResourceId(u64::from(G34S_PARAMS) + 1),
                0,
                bytes_f32(&skin_params),
            ),
            (
                StableResourceId(u64::from(G34S_PAL_CUR) + 1),
                0,
                pal_cur_bytes,
            ),
            (
                StableResourceId(u64::from(G34S_PAL_PREV) + 1),
                0,
                pal_prev_bytes,
            ),
        ];
        // parity 轮换绑定（G34 同律;蒙皮车道 pass 下标顺移：resample=3 /
        // resolve=4 / encode=5）。
        let bindings_resample = Bindings {
            storage_buffers: vec![
                U_SCENE_COLOR,
                U_SCENE_DEPTH,
                U_TSR_PARAMS,
                U_CUR_RGB,
                U_LUMA[p],
                U_DEPTH_HI[p],
            ],
            ..Bindings::default()
        };
        let bindings_resolve = Bindings {
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
        };
        let bindings_encode = Bindings {
            storage_buffers: vec![U_OUT_COLOR[p], G34S_ENC_PARAMS, G34S_ENC_OUT],
            ..Bindings::default()
        };
        let binding_overrides = vec![
            (3, bindings_resample),
            (4, bindings_resolve),
            (5, bindings_encode),
        ];
        // readback 子集（下标升序 = 解析序;p ∈ {0,1} 恒小于 2 天然升序）。
        let mut subset: Vec<u32> = Vec::new();
        if rb_out {
            subset.push(p as u32);
        }
        if rb_verify {
            subset.extend_from_slice(&[2, 3, 4, G34S_RB_HIT, G34S_RB_TRIS]);
        }
        if rb_bgra {
            subset.push(G34S_RB_BGRA);
        }
        let update = FrameUpdate {
            // fork B 逐帧实例变换（3 实例全量表,槽位级 diff 仅动态槽上传）+
            // B5 blas_refit 桥同帧共存（render_exec 同槽同律记账面登记;
            // src_offset = char_tri_base × 36B,byte_len = 角色三角数 × 36B）。
            tlas_update: Some((0u32, g34skin_frame_instances(dyn_xf), TlasBuildAction::Refit)),
            buffer_uploads: uploads,
            binding_overrides,
            push_constant_overrides: vec![],
            readback_subset: Some(subset),
            blas_refit: Some(BlasRefitUpdate {
                as_index: 0,
                blas_index: 2,
                src: StableResourceId(u64::from(U_TRIS) + 1),
                src_offset: self.char_src_offset,
                byte_len: (char_tri_count * 36) as u64,
                after_pass: 0,
            }),
        };
        let prov = self.session.next_provenance_with_update(&update)?;
        let mut out = self.session.execute_with_frame_update(&prov, &update)?;
        // ── 产物解析（telemetry 六 pass 逐名提取;回读按子集构建序）──
        let gpu = |name: &str| -> Result<f64, String> {
            out.telemetry
                .passes
                .iter()
                .find(|pp| pp.name == name)
                .map(|pp| pp.gpu_ns)
                .ok_or_else(|| format!("telemetry 缺 {name} pass 行"))
        };
        let skin_gpu_ns = gpu("g31_skin")?;
        let scene_gpu_ns = gpu("g34_unified_gi_skin")?;
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
        let char_px = char_tri_count * 9;
        let mut idx = 0usize;
        let take_rb = |out: &mut DeviceFrameOutput, idx: &mut usize| -> Result<Vec<u8>, String> {
            if *idx >= out.readbacks.len() {
                return Err(format!(
                    "G34-3 回读路数 {} 少于子集消费序 {idx}",
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
                return Err("G34-3 f32 out_color 回读字节数与输出分辨率不符".into());
            }
            Some(c)
        } else {
            None
        };
        let (mv_out, scene_depth, scene_color, hit, char_tris) = if rb_verify {
            let m = read_f32(&take_rb(&mut out, &mut idx)?);
            if m.len() != mv_px {
                return Err("G34-3 mv 回读字节数与内部分辨率不符".into());
            }
            let d = read_f32(&take_rb(&mut out, &mut idx)?);
            if d.len() != depth_px {
                return Err("G34-3 scene depth 回读字节数与内部分辨率不符".into());
            }
            let s = read_f32(&take_rb(&mut out, &mut idx)?);
            if s.len() != scene_px {
                return Err("G34-3 scene color 回读字节数与内部分辨率不符".into());
            }
            let h = read_f32(&take_rb(&mut out, &mut idx)?);
            if h.len() != hit_px {
                return Err("G34-3 hit 回读字节数与内部分辨率不符".into());
            }
            let t = read_f32(&take_rb(&mut out, &mut idx)?);
            if t.len() != char_px {
                return Err("G34-3 蒙皮顶点回读字节数与角色段不符".into());
            }
            (Some(m), Some(d), Some(s), Some(h), Some(t))
        } else {
            (None, None, None, None, None)
        };
        let bgra8 = if rb_bgra {
            let b = take_rb(&mut out, &mut idx)?;
            if b.len() != bgra_px {
                return Err(format!("G34-3 BGRA8 回读字节 {} ≠ {ow}x{oh}x4", b.len()));
            }
            Some(b)
        } else {
            None
        };
        if idx != out.readbacks.len() {
            return Err(format!(
                "G34-3 回读消费序 {idx} ≠ 实到路数 {}",
                out.readbacks.len()
            ));
        }
        let readback_convert_ms = t_convert.elapsed().as_secs_f64() * 1000.0;
        self.prev_vp_j = Some(*vp_j);
        self.has_history_state = true;
        self.parity = 1 - self.parity;
        self.prev_dyn_xf = Some(dyn_xf);
        Ok(G34SkinFrameRec {
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
            scene_depth,
            scene_color,
            hit,
            char_tris,
            readback_convert_ms,
        })
    }
}

// ---------------------------------------------------------------------------
// G34-3 核验面（① 逐顶点对拍 / ② 位置核验 / ③ MV 三臂核验）
// ---------------------------------------------------------------------------

/// G34-3 蒙皮核验单帧记录（evidence skin.verify_frames 行面）。
struct G34SkinVerifyFrame {
    frame: u32,
    vertex_max_abs: f64,
    pred_px: [f64; 2],
    pred_aabb: [f64; 4],
    obs_px: [f64; 2],
    obs_aabb: [f64; 4],
    obs_count: usize,
    centroid_delta_px: f64,
    aabb_delta_px: f64,
    mv_dev_median_px: [f64; 2],
    mv_host_median_px: [f64; 2],
    mv_median_delta_px: [f64; 2],
    mv_host_motion_px: f64,
    mv_dev_motion_px: f64,
    static_mv_dev_px: [f64; 2],
    static_mv_host_px: [f64; 2],
    static_mv_delta_px: [f64; 2],
    rigid_count: usize,
    rigid_mv_dev_px: [f64; 2],
    rigid_mv_host_px: [f64; 2],
    rigid_mv_delta_px: [f64; 2],
    pass: bool,
}

/// host 参照相机 MV（逐像素;kernel 相机臂同式同序——深度反投影当帧世界点
/// → prev_vp_j 重投影;f64 域,2px 容差吸收 f32/f64 算术差与 jitter 亚像素
/// 偏移）。返回 px 域 mv（dev mv uv×w/h 同域）。
#[allow(clippy::too_many_arguments)]
fn g34skin_host_camera_mv(
    px: u32,
    py: u32,
    ndz: f32,
    inv_cur: &Mat4,
    prev_vp_j: &Mat4,
    iw: u32,
    ih: u32,
) -> [f64; 2] {
    let wf = iw as f64;
    let hf = ih as f64;
    let u = (px as f64 + 0.5) / wf;
    let v = (py as f64 + 0.5) / hf;
    let ndx = (2.0 * u - 1.0) as f32;
    let ndy = (1.0 - 2.0 * v) as f32;
    let w4 = inv_cur.transform_vec4([ndx, ndy, ndz, 1.0]);
    if w4[3].abs() < 1e-8 {
        return [0.0, 0.0];
    }
    let wx = w4[0] / w4[3];
    let wy = w4[1] / w4[3];
    let wz = w4[2] / w4[3];
    let pc = prev_vp_j.transform_vec4([wx, wy, wz, 1.0]);
    if pc[3] <= 1e-8 {
        return [0.0, 0.0];
    }
    let prev_u = 0.5 * (f64::from(pc[0] / pc[3]) + 1.0);
    let prev_v = 0.5 * (1.0 - f64::from(pc[1] / pc[3]));
    [prev_u * wf - (px as f64 + 0.5), prev_v * hf - (py as f64 + 0.5)]
}

/// 类 2 刚性实例 host 参照 MV（逐像素;hit 通道 prim/bary → 局部 bary 插值
/// → prev_xf 变换 → prev_vp_j/当帧 vp_j 双投影差——kernel 刚性臂同式;
/// B5 类 3 host 参照臂同构口径,f64 域 2px 容差吸收）。
#[allow(clippy::too_many_arguments)]
fn g34skin_host_rigid_mv(
    px: u32,
    py: u32,
    prim: usize,
    bu: f32,
    bv: f32,
    cube_tris: &[f32],
    prev_xf: &[f32; 12],
    cur_xf: &[f32; 12],
    vp_j: &Mat4,
    prev_vp_j: &Mat4,
    iw: u32,
    ih: u32,
) -> Option<[f64; 2]> {
    let xf_apply = |xf: &[f32; 12], p: [f32; 3]| -> [f32; 3] {
        [
            ((xf[0] * p[0] + xf[1] * p[1]) + xf[2] * p[2]) + xf[3],
            ((xf[4] * p[0] + xf[5] * p[1]) + xf[6] * p[2]) + xf[7],
            ((xf[8] * p[0] + xf[9] * p[1]) + xf[10] * p[2]) + xf[11],
        ]
    };
    let w0 = (1.0 - bu) - bv;
    let tb = prim * 9;
    let lx = ((w0 * cube_tris[tb] + bu * cube_tris[tb + 3]) + bv * cube_tris[tb + 6]);
    let ly = ((w0 * cube_tris[tb + 1] + bu * cube_tris[tb + 4]) + bv * cube_tris[tb + 7]);
    let lz = ((w0 * cube_tris[tb + 2] + bu * cube_tris[tb + 5]) + bv * cube_tris[tb + 8]);
    let prev_world = xf_apply(prev_xf, [lx, ly, lz]);
    let cur_world = xf_apply(cur_xf, [lx, ly, lz]);
    let (pu, pv) = dyn_project(prev_vp_j, prev_world, iw, ih)?;
    let (cu, cv) = dyn_project(vp_j, cur_world, iw, ih)?;
    // 与 dev 同域：dev mv px = prev_uv·w − (px+0.5);host 双投影差 = prev_cont −
    // cur_cont,cur_cont ≈ px+0.5+jitter——jitter 亚像素差经 2px 容差吸收（B5
    // 类 3 臂同口径）。
    Some([pu - cu, pv - cv])
}

// ---------------------------------------------------------------------------
// G34-3 主流程（--skin on 面：蒙皮×纹理×slab×动态四特性同开真窗口车道）
// ---------------------------------------------------------------------------

/// --skin on CLI 面（主 bin 早分支消费;全字段 = main 既有 CLI 同名面 +
/// 蒙皮 SPV 三件）。
struct G34SkinCli {
    frames: u32,
    warmup: u32,
    tier: u32,
    contract_path: String,
    g10_dir: String,
    gltf_path: String,
    spv_skin: String,
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
    headless: bool,
    auto_move: String,
    slab_table: String,
}

/// G34-3 蒙皮主流程（main() 早分支唯一消费面;装配段 = main ①..⑤ 同函复用
/// ——契约链/G10 转引/scene 装配/slab 接线/纹理接线/窗口创建逐字同律,host
/// 金标准全场景对拍面不建（蒙皮腿对拍 = ① 逐顶点臂承载,登记口径））。
fn g34_skin_main(cli: G34SkinCli) -> ! {
    let G34SkinCli {
        frames,
        warmup,
        tier,
        contract_path,
        g10_dir,
        mut gltf_path,
        spv_skin,
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
        headless,
        auto_move,
        slab_table,
    } = cli;
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
    eprintln!("{GTAG}: [skin] 契约链就绪 contract_digest={} g10 转引一致性=pass", contract.digest);

    // ③ 场景装配（UV sink 恒走——textures 消费面;main 同律）。
    if gltf_path.is_empty() {
        gltf_path = default_gltf(scene_id).to_owned();
    }
    let mut tri_uv: Vec<f32> = Vec::new();
    let mut scene = match assemble_scene_uv(&contract.raw, scene_id, Path::new(&gltf_path), &mut tri_uv) {
        Ok(s) => s,
        Err(e) => dev_env_or_fail("scene_assets", &e),
    };

    // ③.5 slab 侧表生产接线（main ③.5 逐字同律——蒙皮腿 = --full 面,slab
    //     资产必经;16 槽 device/host 双臂对拍 + 逐三角 albedo × R_slot 预调制）。
    let mut slab_report: Option<(SlabSideTableAsset, SlabEval, usize)> = None;
    let mut slab_arm: Option<[f32; SLAB_N_SLOTS]> = None;
    {
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
            "{GTAG}: [skin] slab 接线 arm=device slots=16 mapped_mats={} slab_tris={} parity_p100={:.6e} eval_ms={:.3} abi={}",
            asset.material_slots.len(),
            n_slab,
            eval.parity_p100,
            eval.eval_ms,
            asset.abi_digest,
        );
        slab_arm = Some(arm_r);
        slab_report = Some((asset, eval, n_slab));
    }

    // ③.6 纹理采样生产接线（main ③.6 逐字同律 + texmeta mod × R_slot 预调制）。
    let mut tex_report: Option<(G31TexAssets, G31TexProbeReport)> = None;
    let mut tex_premod_slots = 0usize;
    {
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
        if let (Some(asset_eval), Some(arm_r)) = (slab_report.as_ref(), slab_arm.as_ref()) {
            tex_premod_slots = g34_slab_premod_texmeta(&mut assets, &asset_eval.0, arm_r);
        }
        eprintln!(
            "{GTAG}: [skin] B4 纹理接线 mapped={} tex_tris={} atlas={}x{} probes={} ssbo_p100={:.6e}（位级={} 双跑={}） sampler_max_lsb={} nonconstant_slots={} eval_ms={:.3} slab_premod_slots={}",
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
            tex_premod_slots,
        );
        tex_report = Some((assets, report));
    }
    let eps = scene_eps(&scene.positions);
    eprintln!(
        "{GTAG}: [skin] 装配 scene={scene_id} tris={} quads={} points={} output={out_w}x{out_h} eps={eps:.6} features=[tex=true slab=true dyn=true skin=true]",
        scene.tri_count,
        scene.quads.len(),
        scene.points.len(),
    );

    // ④ 真窗口 present 会话（main ④ 同律;headless-smoke = 无窗口退化仅供
    //    自检逻辑用不计真门）。
    let mut window: Option<vk::ExternalImagePresent> = if headless {
        None
    } else {
        match vk::ExternalImagePresent::create(
            out_w,
            out_h,
            "rurix g34 unified lane + skin (bistro-interior 1080p;G34-3 蒙皮四特性同开;ESC 退出)",
            !hidden,
        ) {
            Ok(w) => Some(w),
            Err(e) => dev_env_or_fail("window_present", &e),
        }
    };
    let bgra = window
        .as_ref()
        .map(|w| w.channel_order() == "bgra8_unorm")
        .unwrap_or(true);
    if let Some(w) = window.as_ref() {
        eprintln!(
            "{GTAG}: [skin] 窗口就绪 {}x{} channel_order={} visible={}",
            w.extent().0,
            w.extent().1,
            w.channel_order(),
            !hidden
        );
    }

    // ⑤ 初态（相机 = 契约位姿;auto-move 轨迹基位;蒙皮角色原点 = B5
    //    skin_origin 同式——dyn 轨迹原点偏移面,开阔柱实测调定在案）。
    let cam0 = G34Camera::from_spec(&scene.camera);
    let mut cam = cam0;
    let ev100 = f64::from(scene.ev100);
    let jitter_base = (seed % JITTER_WINDOW_MOD) as u32;
    let skin_org = skin_origin(&scene.camera);
    let dyn_origin = dyn_trajectory_origin(&scene.camera);
    let cube_tris_host = dyn_cube_tris(DYN_CUBE_HALF);

    // ⑦ era 循环（main ⑦ 同律 + 蒙皮三面;resize → 车道按新 extent 重建,
    //    TSR 历史/palette prev 双 reset）。
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
    let mut digest_seq: Vec<String> = Vec::new();
    let mut ev100_seq: Vec<f64> = Vec::new();
    let mut pose_seq: Vec<[f64; 5]> = Vec::new();
    let mut render_digest = String::new();
    let mut presented_digest = String::new();
    let mut real_render_seconds: f64 = 0.0;
    let mut real_frames: u64 = 0;
    let mut dyn_verify_recs: Vec<DynVerifyFrame> = Vec::new();
    let mut skin_verify_recs: Vec<G34SkinVerifyFrame> = Vec::new();
    'eras: loop {
        let (ew, eh) = window
            .as_ref()
            .map(|w| w.extent())
            .unwrap_or((out_w, out_h));
        let in_w = ((ew as u64 * u64::from(tier)) / 100).max(1) as u32;
        let in_h = ((eh as u64 * u64::from(tier)) / 100).max(1) as u32;
        // ── 蒙皮车道资产（era 重建面;三分区合并 + palette prev 重置）──
        let assets = g34skin_assets(&scene, in_w, in_h, skin_org);
        let mut bits = UnifiedLaneBits::load(
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
        // G34-3 SPV 处置（textures on 恒真面）：scene = NoContraction 注入
        // （B4 同律 + 蒙皮链 host 参照对拍前提）;skin = 同注入（B5 同律——
        // ① 逐顶点位级对拍前提）;mv = UnifiedLaneBits::load 内建注入面继承。
        let scene_words = spv_inject_no_contraction(&load_spv(&spv_scene));
        bits.spv_scene = scene_words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let skin_words = spv_inject_no_contraction(&load_spv(&spv_skin));
        let skin_spv_bytes: Vec<u8> = skin_words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let skin_dispatch = [assets.character.vertex_count as u32, 1, 1];
        let enc_words = load_spv(&spv_encode);
        let (ex, ey, _) = spv_local_size(&enc_words);
        let enc_dispatch = [ew.div_ceil(ex), eh.div_ceil(ey), 1];
        let enc_spv_bytes: Vec<u8> = enc_words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let enc_params = aces13_device_encode_params(ew, eh, bgra);
        let enc_params_bytes = bytes_f32(&enc_params);
        // 纹理侧表（三分区总三角数;角色段恒 −1/0 不消费面）。
        let total_tris = assets.char_tri_base + assets.character.tri_count;
        let side = if let Some((t, _)) = tex_report.as_ref() {
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
        } else {
            G34TexSideTable::default_face(total_tris)
        };
        let descs = g34skin_descs(
            &assets,
            &bits,
            &side,
            &skin_spv_bytes,
            skin_dispatch,
            &enc_spv_bytes,
            enc_dispatch,
            &enc_params_bytes,
            in_w,
            in_h,
            ew,
            eh,
        );
        // 3 BLAS（静态段 / 动态立方体局部段 / 角色绑定姿态——角色创建期
        // updatable 打标 ALLOW_UPDATE）+ 3 实例 TLAS（创建期全 identity）。
        let blas_refs: [&[f32]; 3] = [
            &assets.base.tris[..assets.dyn_tri_base * 9],
            &assets.base.tris[assets.dyn_tri_base * 9..assets.char_tri_base * 9],
            &assets.character.rest_tris,
        ];
        const G34S_UPDATABLE_BLAS: [u32; 1] = [2];
        let accel_structs = [AccelStructDesc {
            scene: RayQuerySceneDesc {
                blas_triangles: &blas_refs,
                instances: &assets.base.instances,
            },
            transforms: None,
            updatable_blas: &G34S_UPDATABLE_BLAS,
        }];
        let mut lane = match G34SkinLane::create(&descs, &accel_structs, assets.char_tri_base) {
            Ok(l) => l,
            Err(e) => dev_env_or_fail("device_lane", &e),
        };
        eprintln!(
            "{GTAG}: [skin] era 就绪 extent={ew}x{eh} internal={in_w}x{in_h}（车道:g31_skin→[blas_refit 桥]→g34_unified_gi_skin→g34_unified_mv→tsr×2→display_encode 六 pass;skin_tris={} skin_verts={} char_tri_base={} bones=3;resize_eras={resize_eras}）",
            assets.character.tri_count,
            assets.character.vertex_count,
            assets.char_tri_base,
        );
        let mut resized = false;
        let mut era_first = true;
        let mut prev_pal: Option<[BoneTransform; 3]> = None;
        let mut prev_vp_host: Option<Mat4> = None;
        while fi < total {
            // ── 窗口事件面（main 同律）──
            if let Some(w) = window.as_mut() {
                let input = w.poll_input();
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
                        if let Err(e) = w.resize(nw, nh) {
                            fail(&format!("窗口 resize {nw}x{nh}: {e}"));
                        }
                        if w.extent() != (ew, eh) {
                            resized = true;
                            resize_eras += 1;
                            break;
                        }
                    }
                }
            }
            // ── 相机（auto-move 确定性轨迹;main 同律）──
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
            // ── 逐帧面：fork B 动态实例变换 + B5 骨骼 palette 双表 + 场景/蒙皮
            //    参数（[42]=dyn_tri_base [43]=char_tri_base [44]=char_inst）──
            let (pos, yaw) = dyn_trajectory(fi, dyn_origin);
            let xf = dyn_transform_3x4(pos, yaw);
            let mut scene_params = pack_frame_params_dyn(
                in_w,
                in_h,
                j,
                eps,
                scene.quads.len(),
                scene.points.len(),
                &inv_vp,
                &vp,
                assets.dyn_tri_base,
            );
            scene_params[43] = assets.char_tri_base as f32;
            scene_params[44] = G34S_CHAR_INST;
            let pal = skin_palette(fi, skin_org);
            let prev = prev_pal.unwrap_or(pal);
            let skin_params = pack_skin_params(
                assets.character.vertex_count,
                prev_pal.is_some(),
                assets.char_tri_base,
                assets.character.bone_count,
            );
            let last = fi + 1 == total;
            let verify = fi >= 1 && fi >= warmup && (fi - warmup) % DYN_VERIFY_EVERY == 0;
            let rb_out = last;
            let rb_verify = verify;
            // auto-move 必经面（digest_seq 逐帧登记）⇒ BGRA8 回读恒开（main
            // `window.is_some() || auto_move.is_some()` 在蒙皮闭集下恒真面）。
            let rb_bgra = true;
            let reset = fi == 0 || era_first;
            era_first = false;
            let t_render = std::time::Instant::now();
            let rec = match lane.frame(
                in_w,
                in_h,
                ew,
                eh,
                j,
                &vp_j,
                exposure,
                reset,
                scene_params,
                skin_params,
                skin_palette_bytes(&pal),
                skin_palette_bytes(&prev),
                xf,
                assets.dyn_tri_base,
                assets.character.tri_count,
                rb_out,
                rb_verify,
                rb_bgra,
            ) {
                Ok(r) => r,
                Err(e) => fail(&format!("帧 {fi} 蒙皮车道: {e}")),
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

                // ── fork B 动态实例位置核验（main 同律：A4 范式 host 投影 vs
                //    scene color 纯绿谱检测,fail-closed）──
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
                    let pass = obs_count >= min_count
                        && centroid_delta <= DYN_TOL_CENTROID_PX
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
                            "帧 {fi} 动态实例位置核验 fail（obs_count={obs_count}（min {min_count}）centroid_Δ={centroid_delta:.3}px aabb_Δ={aabb_delta:.3}px）"
                        ));
                    }
                }

                // ── 蒙皮核验三面（B5 范式 + 类 2 刚性臂 + 静态区相机 MV 一致
                //    性;host 参照臂 = skin_vertex 蒙皮全顶点 + 解析投影）──
                let host_cur = skin_host_verts(&assets.character, &pal);
                let host_prev_pos = skin_host_verts(&assets.character, &prev);
                // ① 逐顶点对拍（device tris 角色段回读 vs host skin_vertex;
                //    max_abs == 0 位级门——B5 在案口径,NoContraction 注入前提）。
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
                // ② 位置核验（pred = host 蒙皮投影并集掩码;obs = hit 通道
                //    inst==2 地面真值检测——B5 同律,角色实例下标 2 面）。
                let (pred_cx, pred_cy, pred_aabb, pred_mask_count) = skin_pred_mask(
                    &host_cur,
                    assets.character.tri_count,
                    &vp_j,
                    in_w,
                    in_h,
                )
                .unwrap_or_else(|| fail("蒙皮掩码投影为空（动画规格破缺）"));
                let pred_c = [pred_cx, pred_cy];
                let obs = skin_detect_hit(hit_plane, in_w, in_h, G34S_CHAR_INST);
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
                // ③a 类 3 蒙皮 MV 域统计（dev 检测像素域中位数 vs host 逐顶点
                //    中位数,逐分量;B5 同律）。
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
                // ③b 静态区相机 MV 一致性（SKIN_STATIC_WIN 背景窗;auto-move
                //    动相机下静态像素 MV = 相机 MV 非零真值——dev vs host 逐分
                //    量中位差 ≤2px;B5 静态相机 ≤1.5px 绝对门的诚实重述面）。
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
                // ③c 类 2 刚性实例 MV 核验（hit 通道 inst==1 像素;dev vs host
                //    逐像素 prev_xf·L 双投影差中位数,逐分量 ≤2px——A4 登记缺
                //    口顺手接通面的核验臂;低可见像素帧门放空如实登记）。
                let mut rigid_idx: Vec<u32> = Vec::new();
                for py in 0..in_h {
                    for px in 0..in_w {
                        let pi = (py * in_w + px) as usize;
                        if hit_plane[pi * 4] == G34S_DYN_INST {
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
                // 逐帧门（B5 在案口径 + ① 位级门 + ③b/③c 一致性门;低动相位
                //    ratio 门放空同律——真动判据归窗级聚合门）。
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

            // ── present（main 同律;device 已编码,host 仅拷贝/present）──
            let mut pres_el = 0.0f64;
            if let Some(w) = window.as_mut() {
                let Some(px) = rec.bgra8.as_ref() else {
                    fail(&format!("帧 {fi} 窗口面缺 BGRA8 回读"));
                };
                let t_one = std::time::Instant::now();
                if let Err(e) = w.present_rgba8(px) {
                    fail(&format!("帧 {fi} 窗口 present: {e}"));
                }
                let el = t_one.elapsed().as_secs_f64() * 1000.0;
                pres_el += el;
                if fi >= warmup {
                    present_ms.push(el);
                }
            }

            // ── digest（auto-move 逐帧序列;main 同律）──
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
                    fail("末帧缺 BGRA8 回读".into());
                };
                presented_digest = g34_bgra_digest(ew, eh, px);
                let Some(out_data) = rec.out_color.as_ref() else {
                    fail("末帧缺 f32 out_color 回读".into());
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
                real_frames += 1;
                real_render_seconds += render_el / 1000.0;
            }
            if fi == 0 || (fi + 1) % 20 == 0 || fi + 1 == total {
                eprintln!(
                    "{GTAG}: [skin] 帧 {}/{total} render={render_el:.3}ms(gpu_skin={:.4}ms gpu_scene={:.3}ms gpu_mv={:.3}ms gpu_encode={:.3}ms) present={pres_el:.3}ms digest={dig_el:.3}ms",
                    fi + 1,
                    rec.skin_gpu_ns / 1e6,
                    rec.scene_gpu_ns / 1e6,
                    rec.mv_gpu_ns / 1e6,
                    rec.encode_gpu_ns / 1e6,
                );
            }
            fi += 1;
        }
        if fi >= total || !resized {
            break 'eras;
        }
    }

    // ── 核验汇总（窗级聚合真动门 + 全帧全门 fail-closed;证据保全先于判红）──
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
        "{GTAG}: [skin] 蒙皮核验 {}/{} 帧通过（① vertex_max_abs={vertex_max_all:.3e}（位级门 ==0）② 质心 ≤{SKIN_TOL_CENTROID_PX}px AABB ≤{SKIN_TOL_AABB_PX}px ③ MV 中位差 ≤{SKIN_MV_TOL_MEDIAN_PX}px 窗级真动 max={motion_max:.3}px ≥{SKIN_MV_HOST_MOTION_MIN_PX}px 类2刚性激活帧={rigid_active_frames}）;dyn 核验 {}/{} 帧通过",
        skin_verify_recs.iter().filter(|r| r.pass).count(),
        skin_verify_recs.len(),
        dyn_verify_recs.iter().filter(|r| r.pass).count(),
        dyn_verify_recs.len(),
    );

    // ⑦ 多口径稳态统计 + evidence（main 同律 + skin 块;host_parity = null
    //    诚实登记——蒙皮腿对拍 = ① 逐顶点臂承载）。
    let frames_done = fi;
    let (r_mean, _, r_cv, r_min, r_max) = g34_stats(&render_ms);
    let (p_mean, _, p_cv, p_min, p_max) = if headless || present_ms.iter().all(|v| *v == 0.0) {
        (0.0, 0.0, 0.0, 0.0, 0.0)
    } else {
        g34_stats(&present_ms)
    };
    let (eg_mean, _, _, _, _) = if encode_gpu_ms.is_empty() {
        (0.0, 0.0, 0.0, 0.0, 0.0)
    } else {
        g34_stats(&encode_gpu_ms)
    };
    let (sg_mean, _, _, _, _) = if scene_gpu_ms.is_empty() {
        (0.0, 0.0, 0.0, 0.0, 0.0)
    } else {
        g34_stats(&scene_gpu_ms)
    };
    let (sk_mean, sk_min) = if skin_gpu_ms.is_empty() {
        (0.0, 0.0)
    } else {
        let (a, _, _, d, _) = g34_stats(&skin_gpu_ms);
        (a, d)
    };
    let (dg_mean, _, _, _, _) = if digest_ms.is_empty() {
        (0.0, 0.0, 0.0, 0.0, 0.0)
    } else {
        g34_stats(&digest_ms)
    };
    let encode_host_ms = 0.0f64;
    let overhead_mean = encode_host_ms + p_mean;
    let counts = window.as_ref().map(|w| w.counts());
    let (window_json, p_mean_json, overhead_json) = if headless {
        ("null".to_owned(), "null".to_owned(), "null".to_owned())
    } else {
        let c = counts.unwrap_or(rurix_rt::vk::ExternalPresentCounts {
            frames_presented: 0,
            swapchain_rebuilds: 0,
        });
        let (fw, fh) = window.as_ref().map(|w| w.extent()).unwrap_or((0, 0));
        (
            format!(
                "{{\"visible\":{},\"channel_order\":{},\"extent\":{{\"w\":{fw},\"h\":{fh}}},\"frames_presented\":{},\"swapchain_rebuilds\":{}}}",
                !hidden,
                jstr(if bgra { "bgra8_unorm" } else { "rgba8_unorm" }),
                c.frames_presented,
                c.swapchain_rebuilds
            ),
            format!("{p_mean:.6}"),
            format!("{overhead_mean:.6}"),
        )
    };
    let encode_spv_json = format!(
        "{{\"path\":{},\"sha256\":{}}}",
        jstr(&spv_encode.replace('\\', "/")),
        jstr(&g34_file_sha(&spv_encode).unwrap_or_else(|e| fail(&e)))
    );
    let real_render_fps = if real_render_seconds > 0.0 {
        real_frames as f64 / real_render_seconds
    } else {
        0.0
    };

    // ── features/textures/slab/dyn 块（main 同面;skin 块 = 本段新增）──
    let features_json =
        "{\"textures\":true,\"slab\":true,\"dyn\":true,\"full\":true,\"static_camera\":false,\"skin\":true}".to_owned();
    let textures_json = if let Some((t, rep)) = tex_report.as_ref() {
        let c = &t.census;
        format!(
            "{{\"census\":{{\"materials_total\":{},\"with_base_color_texture\":{},\"with_normal_texture\":{},\"with_metallic_roughness_texture\":{},\"primitives_total\":{},\"primitives_with_texcoord0\":{},\"primitives_with_tangent\":{}}},\"mapping_law\":\"逐材质三角数降序 top-12（并列时 material_index 升序;其余走常量面 0-byte）\",\"mapped_materials\":{},\"tex_tris\":{},\"atlas\":{{\"width\":{},\"height\":{},\"tile\":2048,\"format\":\"u32_packed_rgba8\",\"digest\":{}}},\"linlut_digest\":{},\"slab_premod_slots\":{},\"probe\":{{\"probe_count\":{},\"eval_ms\":{:.6},\"ssbo\":{{\"p100\":{:.15e},\"bitexact\":{},\"double_run_bitexact\":{},\"device_digest\":{},\"host_digest\":{}}},\"sampler_leg\":{{\"max_lsb_diff\":{},\"bound_lsb\":1,\"bitexact\":{}}}}},\"spv_scene\":{{\"path\":{},\"sha256\":{},\"no_contraction_injected\":true}}}}",
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
            jstr(&spv_scene.replace('\\', "/")),
            jstr(&g34_file_sha(&spv_scene).unwrap_or_else(|e| fail(&e))),
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
            "{{\"dyn_tris\":12,\"dyn_tri_base\":{},\"action\":\"refit\",\"verify_every\":{},\"tol_centroid_px\":{:.3},\"tol_aabb_px\":{:.3},\"min_count_area_ratio\":{:.4},\"verify_frames\":[{}],\"verify_count\":{},\"all_pass\":{}}}",
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
    // skin 块（①②③ 三面 + MV 缺口推进登记;数字真实 measured）。
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
        let char = skin_verify_recs
            .iter()
            .map(|r| r.mv_median_delta_px[0].max(r.mv_median_delta_px[1]))
            .fold(0.0f64, f64::max);
        format!(
            "{{\"character\":{{\"bone_count\":3,\"tri_count\":36,\"vertex_count\":108,\"origin\":[{:.6},{:.6},{:.6}],\"emission\":[{},{},{}],\"albedo\":[{},{},{}],\"char_tri_base\":{},\"char_inst\":2,\"blas_index\":2,\"spv_skin\":{{\"path\":{},\"sha256\":{}}},\"spv_mv\":{{\"path\":{},\"sha256\":{}}}}},\"tolerance\":{{\"vertex_max_abs\":0.0,\"centroid_px\":{},\"aabb_px\":{},\"mv_median_px\":{},\"min_count_ratio\":0.75,\"mv_host_motion_min_px\":{},\"mv_dev_ratio_min\":{},\"static_mv_consistency_px\":{},\"rigid_mv_px\":{},\"rigid_min_count\":{}}},\"vertex_parity\":{{\"frames\":{},\"max_abs_max\":{},\"all_bitexact\":{}}},\"verify_frames\":[{}],\"verify_count\":{},\"all_pass\":{},\"motion_gate\":{{\"host_motion_max_px\":{},\"threshold_px\":{}}},\"mv_gap\":{{\"class1_camera\":\"wired（相机臂 g14_mv 镜像;静态区一致性 ≤2px 核验面）\",\"class2_rigid\":\"wired+verified（g34_unified_mv 刚性臂:局部 bary 插值 → prev_dyn_xf 变换 → prev_vp 投影;hit 通道 inst==1 像素 dev/host 中位差核验）\",\"class3_skinned\":\"wired+verified（g31_skin_mv 镜像面:B5 prev 蒙皮顶点 bary 插值臂）\",\"class1_delta_max_px\":{},\"class2_delta_max_px\":{},\"class3_delta_max_px\":{},\"rigid_active_frames\":{},\"note\":\"RD-041 三类速度设计在 G34-3 统一车道蒙皮腿全接线——类 2 刚性实例 MV = A4 登记缺口顺手接通面（本腿 TSR 历史链生效）;非蒙皮腿维持 g14_mv 0-byte + 缺口登记（不冒充全局面接通）\"}},\"skin_gpu_ms\":{{\"mean\":{:.6},\"min\":{:.6}}}}}",
            skin_org[0], skin_org[1], skin_org[2],
            SKIN_EMISSION[0], SKIN_EMISSION[1], SKIN_EMISSION[2],
            SKIN_ALBEDO[0], SKIN_ALBEDO[1], SKIN_ALBEDO[2],
            scene.indices.len() + 12,
            jstr(&spv_skin.replace('\\', "/")),
            jstr(&g34_file_sha(&spv_skin).unwrap_or_else(|e| fail(&e))),
            jstr(&spv_mv.replace('\\', "/")),
            jstr(&g34_file_sha(&spv_mv).unwrap_or_else(|e| fail(&e))),
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
            jf(char),
            rigid_active_frames,
            sk_mean,
            sk_min,
        )
    };
    let spv_skin_json = format!(
        "{{\"scene\":{{\"path\":{},\"sha256\":{}}},\"mv\":{{\"path\":{},\"sha256\":{}}},\"skin\":{{\"path\":{},\"sha256\":{}}},\"resample\":{{\"path\":{},\"sha256\":{}}},\"resolve\":{{\"path\":{},\"sha256\":{}}},\"encode\":{{\"path\":{},\"sha256\":{}}}}}",
        jstr(&spv_scene.replace('\\', "/")),
        jstr(&g34_file_sha(&spv_scene).unwrap_or_else(|e| fail(&e))),
        jstr(&spv_mv.replace('\\', "/")),
        jstr(&g34_file_sha(&spv_mv).unwrap_or_else(|e| fail(&e))),
        jstr(&spv_skin.replace('\\', "/")),
        jstr(&g34_file_sha(&spv_skin).unwrap_or_else(|e| fail(&e))),
        jstr(&spv_resample.replace('\\', "/")),
        jstr(&g34_file_sha(&spv_resample).unwrap_or_else(|e| fail(&e))),
        jstr(&spv_resolve.replace('\\', "/")),
        jstr(&g34_file_sha(&spv_resolve).unwrap_or_else(|e| fail(&e))),
        jstr(&spv_encode.replace('\\', "/")),
        jstr(&g34_file_sha(&spv_encode).unwrap_or_else(|e| fail(&e))),
    );

    let mut ev = String::with_capacity(8192);
    ev.push('{');
    ev.push_str(&format!("\"schema\":{},", jstr(G34S_SCHEMA)));
    ev.push_str(&format!("\"gate\":{},", jstr(G34S_GATE)));
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
    ev.push_str(&format!("\"present_frame_ms\":{p_mean_json},"));
    ev.push_str(&format!("\"present_overhead_ms\":{overhead_json},"));
    ev.push_str(&format!("\"encode_frame_ms\":{encode_host_ms:.6},"));
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
    ev.push_str(&format!("\"headless\":{headless},"));
    ev.push_str(&format!("\"window\":{window_json},"));
    ev.push_str("\"contracts\":{\"production\":");
    ev.push_str(&format!(
        "{{\"path\":{},\"digest\":{}}},",
        jstr(&contract_path.replace('\\', "/")),
        jstr(&contract.digest)
    ));
    ev.push_str(&g10_fragment);
    ev.push_str(&format!(",\"encode_spv\":{encode_spv_json}"));
    ev.push_str("},");
    ev.push_str("\"render_includes_forced_readback\":true,");
    ev.push_str(&format!("\"spv\":{spv_skin_json},"));
    ev.push_str(&format!("\"features\":{features_json},"));
    ev.push_str(&format!("\"textures\":{textures_json},"));
    ev.push_str(&format!("\"slab\":{slab_json},"));
    ev.push_str(&format!("\"dyn\":{dyn_json},"));
    ev.push_str(&format!("\"skin\":{skin_json},"));
    ev.push_str("\"host_parity\":null,");
    ev.push_str(&format!(
        "\"stats\":{{\"render_cv\":{r_cv:.6},\"render_min_ms\":{r_min:.6},\"render_max_ms\":{r_max:.6},\"scene_gpu_ms\":{sg_mean:.6},\"encode_gpu_ms\":{eg_mean:.6},\"skin_gpu_ms\":{sk_mean:.6},\"present_cv\":{},\"present_min_ms\":{},\"present_max_ms\":{}}},",
        if headless { "null".to_owned() } else { format!("{p_cv:.6}") },
        if headless { "null".to_owned() } else { format!("{p_min:.6}") },
        if headless { "null".to_owned() } else { format!("{p_max:.6}") },
    ));
    ev.push_str(&format!(
        "\"notes\":{}",
        jstr("G34 全特性合流 G34-3 蒙皮角色进真窗口统一车道：蒙皮×纹理×slab×动态实例四特性同开——G34Full 27 SSBO 加性扩蒙皮七件（hit 通道/绑定姿态/权重/palette 双表/prev 顶点表/skin 参数）= 36 资源六 pass（g31_skin → FrameUpdate::blas_refit 桥〔角色 BLAS 2 逐帧 UPDATE〕→ g34_unified_gi_skin〔G34-1 统一 kernel + out_hit + 角色实例分派〕→ g34_unified_mv〔g31_skin_mv 镜像 + 类 2 刚性实例臂——A4 登记缺口本腿顺手接通〕→ TSR 双 pass → display_encode）+ 逐帧 tlas_update refit（3 实例表,动态槽位级增量）;核验三面 = ① 蒙皮 device/host 逐顶点对拍（max_abs == 0 位级门,B5 在案口径,NoContraction 注入前提）② 位置核验（host 蒙皮投影掩码 vs hit 通道 inst==2 地面真值,质心 ≤4px/AABB ≤6px/计数 ≥ max(200,0.75×掩码),B5 在案口径）③ MV 通道（类 3 dev/host ≤2px + 窗级真动门;类 1 静态区相机 MV 一致性 ≤2px——auto-move 动相机下 B5 静态绝对门的诚实重述;类 2 刚性 dev/host ≤2px）;确定性双跑 digest 位级 + skin≠静态/skin≠无skin全特性 digest 区分 + frame_ms measured（skin on/off）;host 金标准全场景对拍面 = null（蒙皮腿对拍 = ① 逐顶点臂承载,冻结容差标定面 = G34-1 非蒙皮腿在案——诚实登记不混口径）。g31_window_present.rs/g14_mv/g31_skin_mv/g34_unified_gi.rx 0-byte——其门为回归锚。")
    ));
    ev.push('}');

    if evidence_path.is_empty() {
        println!("{ev}");
    } else {
        std::fs::write(&evidence_path, format!("{ev}\n"))
            .unwrap_or_else(|e| fail(&format!("evidence 写 {evidence_path}: {e}")));
        eprintln!("{GTAG}: [skin] evidence → {evidence_path}");
    }
    if !skin_all_pass {
        fail("蒙皮核验汇总 fail（逐帧门/窗级真动门/类2激活帧数——帧详情见 evidence skin.verify_frames）");
    }
    if !dyn_all_pass {
        fail("动态实例位置核验汇总 fail（帧详情见 evidence dyn.verify_frames）");
    }
    eprintln!("{GTAG}: [skin] PASS frames={frames_done}/{total} real_render={r_mean:.3}ms present={p_mean:.3}ms exit={exit_reason}");
    std::process::exit(0)
}
