// Assisted-by: Kimi-K3（G14.3 Rurix 管线性能波）
//! G14.3 M-c Rurix 生产管线性能面 harness（门 `g14.p0.m_c.rurix_pipeline_perf`；
//! G14_CONTRACT §4.2 M-c 行）。
//!
//! ## 职责闭集
//!
//! 1. **生产 GPU 场景车道**（架构裁决已定 = 持久 session 车道；G13.4 登记的
//!    host TriBvh 渲染 + 逐帧回读同步口径倒挂面的 device 化兑现）：场景装载
//!    = g13_4 同 crate 同型复制子集（bin-local 惯例——JSON 解析 / glTF 最小
//!    装载 / DDS 均值解码 / assemble_scene 场景装配 / 契约解析，消费
//!    milestones/g13/g13_ue_upscale_parity_contract.json 的 cornell-box
//!    （512×512）与 bistro-interior（1920×1080）双场景行，内容模型 = 逐三角
//!    albedo/emission + point/quad 灯 + 契约相机，与 G13.4 逐字同模——M-d
//!    画质守护可比性锚）→ 三角形汤一次性建 BLAS/TLAS（`DeviceFrameSession::
//!    new_with_accel_structs`，AS 常驻 + scene buffer 创建期一次上传，禁逐帧
//!    全量重传场景）→ 逐帧 `execute_with_frame_update`（192B 帧参数上传 +
//!    readback 子集）驱动 `kernels/g14_3_direct_gi.rx`（RayQuery compute：
//!    jittered 主射线 → 逐三角 albedo/emission → quad 灯 4×4 分层确定性采样
//!    + point 灯 delta + 逐灯阴影射线 + emissive 主命中——面片逻辑镜像
//!    g13_4 shade_pixel L1902 语义）→ 内部分辨率 color(3ch f32)/depth(1ch
//!    ZO NDC) GPU buffer 回读。
//! 2. **超分链挂接**：
//!    - **tsr_device 臂 = G14plus 统一四 pass 车道**（RFC-0030 §4.5 L2 +
//!      §4.3 L3 已批准终态）：单一 DeviceFrameSession，pass0=scene →
//!      pass1=mv（kernels/g14_mv.rx，`compute_camera_mv` 机械转写 + bin 侧
//!      NoContraction 注入）→ pass2/3=tsr resample/resolve，GPU 链内零 host
//!      往返（原两 session 的 scene 回读 + host mv + TSR 上传/回读中转税
//!      消除，bistro t50 过渡态 prod 156ms → 稳态 ~29ms）；bench 测量循环
//!      零回读仅末帧回读 digest；render 腿逐帧回读出 EXR。历史五元组 A/B
//!      parity SSBO 常驻同律轮换。**mv 数值面登记**：GPU mv 与 host mv 存在
//!      ULP 级运算差（Vulkan FDiv 规范容差 2.5 ULP，非正确舍入；FMA 收缩已
//!      经 NoContraction 消除），miss 像素病态反投影放大至 max ~1e-2（mean
//!      ~3e-6），TSR 输出 digest 与旧两 session 架构（host mv）锚不等——
//!      RFC-0030 §4.1 L3 预期内改图 L1 级，本 bin 双跑位级确定性不受影响。
//!    - **dlss_sr 臂 = G14.10e 驻留统一车道**（RFC-0030 §4.3 vendor 输入驻留
//!      接线）：单一 exportable DeviceFrameSession 三 pass（scene → mv →
//!      pack 手编 SPV 直写 RGBA32F/R32F/RG32F exportable image）→ OPAQUE_WIN32
//!      导入 → DLSS `upscale_resident_external` 驻留 evaluate（LUID 对拍
//!      fail-closed）；scene 逐帧回读 + host mv + vendor host pack 三段中转税
//!      全消；digest 语义 = DLSS 输出（`readback_output_into` 按需回读）。
//!      **数值面登记**：mv=GPU mv（ULP 级差 vs host）+ color f32 直通（vs 现状
//!      f16 pack）+ depth R32F（vs D32）——evaluate 输入位面变化，输出 digest
//!      相对旧锚预期 L1 漂移，双跑位级确定性为门檩。
//!    - **fsr_3_1_5 臂 = 现状结构**（场景 session 逐帧回读 + host
//!      `compute_camera_mv` 单源 + vendor host pack；G13.2 M-a adapter 同
//!      模式，FSR resident 归 external memory 波另判）。
//! 3. **双模式**：`--render` 产 32 帧 Halton 收敛序列 + converged.exr +
//!    render_receipt（converged_digest 固定 seed 位级——双跑位级一致面）；
//!    `--bench <scene> --tier <50|67|100> --backend <B> --frames 160
//!    --warmup 10` 持续帧循环（session 不销毁），逐帧 host Instant 墙钟 +
//!    `DeviceFrameTelemetry` 逐 pass GPU ns 双分项 + host 分项（scene render
//!    / mv / upscale）序列 + warmup 后稳态统计（mean/cv，程序产禁手写阈）。
//!
//! ## GI 臂评估登记（架构裁决面如实登记，不静默省略）
//!
//! `--gi off`（默认）= 直接光唯一臂（G13/G14/G15 位级锚，0-byte）。
//! `--gi on` = G16plus 加性车道（RFC-0031）：`kernels/g16_gi_multibounce.rx`
//! （主射线直接光同式 + 次级 NEE + ≥2 反弹）。不得改默认 off 臂 SPV/参数。
//!
//! ## 性能波登记（G14.3 优化波实测结论面）
//!
//! - **已消费**：① scene kernel `wg` 标注系 `#[numthreads(8, 8, 1)]` 2D 车道
//!   （编译器 compute `#[numthreads]` 提取 → MIR `Body::compute_numthreads` →
//!   `LocalSize` 发射；无标注 kernel 恒 (1,1,1) 零漂移——G13 TSR SPV 重编译
//!   字节一致机核）；② tsr_device 移植 DeviceFrameSession 常驻车道（见上；
//!   cornell t67 173.5→~26ms、bistro t67 725.5→~261ms）；③ vendor host 冗余
//!   消除（pack 缓冲 session 常驻 + DLSS readback 内存型 HOST_CACHED——原
//!   uncached/WC 逐元素读 ~325ms@1080p 输出 → ~13.7ms）。
//! - **已消费（G31+ 波 A Task A2）**：① fence/readback 重叠——
//!   `submit_with_frame_update(→ FrameTicket)`/`collect(FrameTicket →
//!   DeviceFrameOutput)` 分离面（G14plus RFC-0030 §4.3 L2 落地于
//!   render_exec.rs；per-slot cmd/timestamp/上传/回读 staging + G31 增
//!   per-slot descriptor override set 解开 parity binding_overrides 入流水）
//!   经 `--inflight <2|3>` 在本 bin tsr_device bench 臂消费：第 k 帧 submit
//!   后不等待，第 k+1−N 帧 FIFO collect；末帧 digest 延迟至 collect 且与
//!   同步模式位级一致（g31.waveA.pipelining 门机核 + flip-trace 帧序断言）；
//! - **未消费（登记）**：② `compute_camera_mv` 留 host
//!   （temporal 底座 0-byte 纪律；实测 ~5.5ms@bistro t67；kernel 移植可选位
//!   评估：数学面 transform_vec4 左结合可位级复制、收益 ~5ms/帧、风险中——
//!   未消费）；③ vendor evaluate/submit_wait 同步面 = vendor 固有 GPU 执行
//!   （FSR ~18~27ms、DLSS ~18ms@1080p 输出档实测构成），不可消。
//!
//! ## 三态
//!
//! 无 Vulkan loader/设备/ray query 能力链/vendor DLL/场景资产 → `SKIP
//! DEV_ENV_DEGRADE`（退 0，非 fake pass；`RURIX_REQUIRE_REAL=1` 下缺真实面即
//! FAIL 退 1，禁 mock 充真跑——G13 M-a/M-c 同语义）。契约解析违例/digest
//! 不等/双跑位级漂移 ⇒ FAIL 退 1。
//!
//! ## 用法
//!
//! ```text
//! g14_3_pipeline_perf --contract-digest [--contract <contract.json>]
//! g14_3_pipeline_perf --selftest-digest [--contract <contract.json>]
//! g14_3_pipeline_perf --render --scene <cornell-box|bistro-interior> \
//!     --tier <50|67|100> --backend <tsr_device|dlss_sr|fsr_3_1_5> [--frames 32] \
//!     [--calibration-seed] [--contract <c.json>] [--gltf <scene.gltf>] \
//!     [--spv-scene <g14_3.spv>] [--spv-resample <a.spv> --spv-resolve <b.spv>] \
//!     [--out-root <dir>] [--expect-digest <sha256:…>] [--gi off|on]
//! g14_3_pipeline_perf --bench --scene <…> --tier <…> --backend <…> \
//!     [--frames 160] [--warmup 10] [--inflight <1|2|3>] [同上选项]
//! g14_3_pipeline_perf --bench --scene bistro-interior --tier <…> \
//!     --backend tsr_device --dyn-demo <refit|rebuild> [--frames 160] [--warmup 10]
//!     （G31+ 波 A Task A4 动态场景更新通路：逐帧实例变换 + TLAS refit/rebuild
//!     策略臂；inflight 恒 1——FIF 流水面拒 tlas_update，A2 约束登记）
//! g14_3_pipeline_perf --bench --scene bistro-interior --tier <…> \
//!     --backend tsr_device --skin-demo [--frames 160] [--warmup 10]
//!     （G31+ 波 B Task B5 蒙皮/骨骼动画进生产帧：device LBS 蒙皮（骨骼
//!     palette 逐帧上传）+ BLAS 逐帧 UPDATE refit + 蒙皮 MV 通道进 TSR
//!     历史链；inflight 恒 1——FIF 流水面拒 blas_refit，A2 同律登记）
//! g14_3_pipeline_perf --bench --scene bistro-interior --tier <…> \
//!     --backend tsr_device [--presentation-profile night] --smooth-normals on
//!     （D2 平滑顶点法线加性臂：glTF NORMAL → 9 f32/tri trinrm 侧表 +
//!     kernels/g18_smooth_nrm.rx（g18 逐字 fork + params[43] 门重心插值）；
//!     默认 off = 既有面 0-byte；与 --gi on/--dyn-demo/--skin-demo/
//!     --cluster-lod/--wp-hlod fail-closed 互斥）
//! g14_3_pipeline_perf --bench --scene bistro-interior --tier <…> \
//!     --backend tsr_device [--presentation-profile night] \
//!     --smooth-normals on --ggx on
//!     （D6 GGX 高光材质加性臂：glTF pbrMetallicRoughness metallicFactor/
//!     roughnessFactor → 2 f32/tri tri_mr 侧表 + g18_smooth_nrm kernel
//!     params[48] 门 GGX 高光项〔D=Trowbridge-Reitz/G=Smith Schlick-GGX/
//!     F=Schlick，F0=mix(0.04,albedo,metallic)〕；默认 off = 既有面 0-byte；
//!     须随 --smooth-normals on〔flat 法线下高光无意义，fail-closed〕，
//!     互斥面与 --smooth-normals 同集）
//! g14_3_pipeline_perf --bench|--render --scene bistro-interior --tier <…> \
//!     --backend tsr_device --smooth-normals on --lamp-lights on \
//!     [--lamp-gain 1.0] [--lamp-k 12] [--lamp-contrib 0.0] \
//!     [--lamp-stats-out <json>]
//!     （画质战役 A1 灯光提取加性臂：emissive 三角〔任一通道 Le>0 且非 quad
//!     灯尾段〕确定性聚类〔0.6m 网格 + 26 邻域 union-find〕→ ≤K 代表点光
//!     append 进 points 面〔I_c = Φ_c·gain/(4π)，radius = 簇最大顶点距
//!     +0.02m 进 pack 槽 7〕+ params[49] 贡献剔除阈值；默认 off = 既有面
//!     0-byte；须随 --smooth-normals on〔kernel 消费面〕，互斥面同集）
//! g14_3_pipeline_perf --bench|--render --scene bistro-interior --tier <…> \
//!     --backend tsr_device --smooth-normals on --gi2 on \
//!     [--gi2-scale 1.0] [--gi2-clamp 4.0]
//!     （画质战役 Phase C GI2 加性臂：g31_texture_nrm_gi 统一质量 kernel +
//!     哑表五件〔tritex 全 −1 ⇒ tex_gate=0 恒走 mats 均值面〕+ params[51..55)
//!     〔[51] 门 [52] frame_idx [53] firefly clamp [54] scale〕——R2 低差异
//!     序列 1 反弹间接光〔余弦半球 + 反弹点单点光随机 NEE + emission 直取〕；
//!     默认 off = 既有面 0-byte；须随 --smooth-normals on + --inflight 1，
//!     互斥面随 --smooth-normals 同集）
//! g14_3_pipeline_perf --bench|--render --scene <…> --tier <…> \
//!     --backend tsr_device --tsr-quality on \
//!     [--tsrq-min-alpha 0.02] [--tsrq-clamp 0]
//!     （画质战役 Phase D TSR 降噪质量档加性臂：resolve pass 换载
//!     kernels/g31_tsr_resolve_q.rx 独立 SPV〔字节隔离——off 臂恒载
//!     .tmp/g14_gates/m_c 冻结字节〕：Karis 反亮度加权混合〔压 emissive
//!     亚像素弹出/萤火虫〕+ 稳态 alpha 档 tsr_params[19]〔默认 0.02，母版
//!     稳态实测 0.1——驻态残差 ∝ √(α/(2−α)) 按档兑现〕+ 深度验证 3×3 膨胀
//!     区间化〔深度边缘像素不再随 jitter 恒拒史〕+ 可选 3×3 邻域亮度 clamp
//!     〔[20]=K，0=关〕；默认 off = 既有面 0-byte；仅 tsr_device，不与
//!     --dyn-demo/--skin-demo 同跑〔demo 面 prepare 路未接线〕，与全部质量
//!     臂可组合）
//! g14_3_pipeline_perf --bench|--render --scene <…> --tier <…> \
//!     --backend tsr_device --quality <off|full>
//!     （画质战役 Phase E1 质量预设：full = 解析层一键展开 bench 质量腿
//!     子集〔--smooth-normals on --ggx on --lamp-lights on --lamp-gain 4
//!     --gi2 on --gi2-clamp 0.01 --tsr-quality on〕——窗口九臂中 --textures
//!     无 bench 臂〔texel heap 侧表 bench 车道未接线,Phase B 留窗〕、
//!     --bloom/--dither/--auto-exposure 为窗口 presented 显示链专属〔bench
//!     EXR 在 encode 上游无此三面〕,如实不展开；展开先于全部臂校验/换载 ⇒
//!     下游与显式旗标写法完全同路径 = 位级等价；展开面旗标显式重叠 =
//!     fail-closed；RURIX_G18_AMBIENT env 缺席时预设注入 0.004〔显式 env
//!     一律优先〕；off = 中性字面零展开）
//! ```

#![forbid(unsafe_code)]

// ── 共享体(G31+ 波 A:实现逐字移入,include 回原位,语义零漂移;vk.rs
//    include vk_m50_rt_body.rs 同型先例)──
include!("g14_3_lane/g14_3_lane_body.rs");

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        fail("缺子模式（--contract-digest / --selftest-digest / --render / --bench / --dump-scene）");
    }
    match args[1].as_str() {
        // G31+ #58 步骤 1/3：装配产物 dump（RXCS v1|v2）——簇 DAG 离线 bake
        //（rurix-asset g31_cluster_lod_bake）的唯一装配输入面（装配语义单源，
        // bake 侧禁复刻装配）。纯 host，GPU 非必需。
        // G31+ #96：默认带 UV 段（v2,装配 TEXCOORD_0 sink——属性保持简化
        // bake 输入面;装配面对缺 TEXCOORD_0 的资产 fail-closed）;
        // `--uv off` = 无 UV 资产臂逃生口,产 v1 字节面逐位不变。
        "--dump-scene" => {
            let mut scene_id = String::new();
            let mut contract_path = DEFAULT_CONTRACT.to_owned();
            let mut gltf_path = String::new();
            let mut out_path = String::new();
            let mut uv_on = true;
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--scene" => scene_id = take_arg(&args, &mut i),
                    "--contract" => contract_path = take_arg(&args, &mut i),
                    "--gltf" => gltf_path = take_arg(&args, &mut i),
                    "--out" => out_path = take_arg(&args, &mut i),
                    "--uv" => {
                        uv_on = match take_arg(&args, &mut i).as_str() {
                            "on" => true,
                            "off" => false,
                            other => fail(&format!("--uv {other}：只接受 on|off")),
                        }
                    }
                    other => fail(&format!("未知参数 {other}")),
                }
                i += 1;
            }
            if scene_id.is_empty() || out_path.is_empty() {
                fail("--dump-scene 参数闭集缺行（--scene / --out）");
            }
            if gltf_path.is_empty() {
                gltf_path = default_gltf(&scene_id).to_owned();
            }
            let text = std::fs::read_to_string(&contract_path)
                .unwrap_or_else(|e| fail(&format!("契约读取: {e}")));
            let contract = parse_contract(&text).unwrap_or_else(|e| fail(&e));
            if contract.digest != FROZEN_CONTRACT_DIGEST {
                fail(&format!(
                    "契约 digest 不等: {} ≠ {FROZEN_CONTRACT_DIGEST}",
                    contract.digest
                ));
            }
            let mut groups: Vec<SceneNodeGroup> = Vec::new();
            let mut tri_uv: Vec<f32> = Vec::new();
            let scene = assemble_scene_ex(
                &contract.raw,
                &scene_id,
                Path::new(&gltf_path),
                Some(&mut groups),
                uv_on.then_some(&mut tri_uv),
            )
            .unwrap_or_else(|e| fail(&format!("场景装配: {e}")));
            dump_scene_rxcs(
                &scene,
                &groups,
                uv_on.then_some(tri_uv.as_slice()),
                Path::new(&out_path),
            )
            .unwrap_or_else(|e| fail(&e));
            println!(
                "{TAG}: dump-scene OK scene={scene_id} tris={} groups={} emissive_tris={} quads={} rxcs_v={} -> {out_path}",
                scene.tri_count,
                groups.len(),
                scene.emissive_tri_count,
                scene.quads.len(),
                if uv_on { 2 } else { 1 },
            );
        }
        "--contract-digest" => {
            let mut path = DEFAULT_CONTRACT.to_owned();
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--contract" => path = take_arg(&args, &mut i),
                    other if !other.starts_with("--") => path = other.to_owned(),
                    other => fail(&format!("未知参数 {other}")),
                }
                i += 1;
            }
            contract_leg(&path);
        }
        "--selftest-digest" => {
            let mut path = DEFAULT_CONTRACT.to_owned();
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--contract" => path = take_arg(&args, &mut i),
                    other => fail(&format!("未知参数 {other}")),
                }
                i += 1;
            }
            selftest_leg(&path);
        }
        "--render" | "--bench" => {
            let bench = args[1].as_str() == "--bench";
            let mut scene_id = String::new();
            let mut tier: u32 = 0;
            let mut backend = String::new();
            let mut frames: u32 = 0;
            let mut warmup: u32 = 10;
            let mut inflight: u32 = 1;
            let mut calibration = false;
            let mut gi = String::from("off");
            // day_0828 Phase C GI2 加性臂（off 默认 = 既有面 0-byte；on =
            // g31_texture_nrm_gi 统一质量 kernel + 哑表五件 + params[51..55)
            // ——R2 低差异 1 反弹间接光；须随 --smooth-normals on + inflight=1，
            // fail-closed）。
            let mut gi2 = String::from("off");
            let mut gi2_scale: Option<f32> = None;
            let mut gi2_clamp: Option<f32> = None;
            // day_0828 Phase D TSR 降噪质量档加性臂（off 默认 = 既有面 0-byte；
            // on = resolve pass 换载 g31_tsr_resolve_q.spv〔字节隔离〕+
            // tsr_params[19..21)；仅 tsr_device，fail-closed）。
            let mut tsr_quality = String::from("off");
            let mut tsrq_min_alpha: Option<f32> = None;
            let mut tsrq_clamp: Option<f32> = None;
            // 画质战役 Phase E1 --quality off|full 预设（默认 off = 零展开
            // 零行为；full = 解析层展开质量腿子集,见 parse loop 尾展开块）。
            let mut quality_full = false;
            // D2 平滑顶点法线臂（off 默认 = 既有面 0-byte；on = g18_smooth_nrm
            // kernel + trinrm 侧表 + params[43]=1.0）。
            let mut smooth_normals = String::from("off");
            // D6 GGX 高光臂（off 默认 = 既有面 0-byte；on = tri_mr 2 f32/tri
            // 侧表 + params[48]=1.0；须随 --smooth-normals on，fail-closed）。
            let mut ggx = String::from("off");
            // A1 灯光提取加性臂（off 默认 = 既有面 0-byte；on = emissive 三角
            // 确定性聚类 → ≤K 代表点光 append 进 points 面 + params[49]=
            // contrib；须随 --smooth-normals on，fail-closed）。
            let mut lamp_lights = String::from("off");
            let mut lamp_gain: Option<f32> = None;
            let mut lamp_k: Option<usize> = None;
            let mut lamp_contrib: Option<f32> = None;
            let mut lamp_stats_out: Option<String> = None;
            let mut contract_path = DEFAULT_CONTRACT.to_owned();
            let mut gltf_path = String::new();
            let mut spv_scene = DEFAULT_SPV_SCENE.to_owned();
            let mut spv_mv = DEFAULT_SPV_MV.to_owned();
            let mut spv_resample = DEFAULT_SPV_RESAMPLE.to_owned();
            let mut spv_resolve = DEFAULT_SPV_RESOLVE.to_owned();
            let mut out_root = DEFAULT_OUT_ROOT.to_owned();
            let mut expect_digest: Option<String> = None;
            let mut presentation_profile: Option<String> = None;
            // C7 profiler 输出面（--profile-json;None = 默认关全零消费）。
            let mut profile_json: Option<String> = None;
            let mut export_png = false;
            let mut dyn_demo: Option<String> = None;
            // G31+ 波 B Task B5:蒙皮/骨骼动画进生产帧 demo(device 蒙皮 LBS +
            // BLAS 逐帧 refit + 蒙皮 MV 通道;仅 --bench tsr_device inflight=1)。
            let mut skin_demo = false;
            // G31+ #58:簇 DAG LOD（off 默认 = 既有面 0-byte;leaf = 全叶逐位
            // 对拍锚;on = 屏幕误差 cut,--cluster-error-px 默认 1.0;
            // --cluster-resident-pages = E 驻留压力臂,0 = 全驻留）。
            let mut cluster_lod_mode = String::from("off");
            let mut cluster_pack = String::new();
            let mut cluster_error_px: f32 = 1.0;
            let mut cluster_resident_pages: u32 = 0;
            // G31+ #95/#68:WP cell + HLOD（off 默认 = 既有面 0-byte;full =
            // 全 Full 逐位对拍锚;on = screen-size 阈值互斥切换,--wp-threshold-l0
            // 默认 1.0 层间 ÷16 递减 = 切换距离逐层 ×4）。
            let mut wp_hlod_mode = String::from("off");
            let mut wp_pack = String::new();
            let mut wp_threshold_l0: f64 = 1.0;
            let mut wp_radius: f32 = 64.0;
            let mut wp_warmup: u32 = 4;
            let mut wp_budget_cells: u32 = 4;
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--scene" => scene_id = take_arg(&args, &mut i),
                    "--tier" => {
                        tier = take_arg(&args, &mut i)
                            .parse()
                            .unwrap_or_else(|_| fail("--tier 非 u32"))
                    }
                    "--backend" => backend = take_arg(&args, &mut i),
                    "--frames" => {
                        frames = take_arg(&args, &mut i)
                            .parse()
                            .unwrap_or_else(|_| fail("--frames 非 u32"))
                    }
                    "--warmup" => {
                        warmup = take_arg(&args, &mut i)
                            .parse()
                            .unwrap_or_else(|_| fail("--warmup 非 u32"))
                    }
                    "--inflight" => {
                        inflight = take_arg(&args, &mut i)
                            .parse()
                            .unwrap_or_else(|_| fail("--inflight 非 u32"))
                    }
                    "--gi" => gi = take_arg(&args, &mut i),
                    // Phase C GI2 加性臂三参数（off 默认 = 零消费）。
                    "--gi2" => gi2 = take_arg(&args, &mut i),
                    "--gi2-scale" => {
                        gi2_scale = Some(
                            take_arg(&args, &mut i)
                                .parse()
                                .unwrap_or_else(|_| fail("--gi2-scale 非 f32")),
                        )
                    }
                    "--gi2-clamp" => {
                        gi2_clamp = Some(
                            take_arg(&args, &mut i)
                                .parse()
                                .unwrap_or_else(|_| fail("--gi2-clamp 非 f32")),
                        )
                    }
                    // Phase D TSR 降噪质量档三参数（off 默认 = 零消费）。
                    "--tsr-quality" => tsr_quality = take_arg(&args, &mut i),
                    "--tsrq-min-alpha" => {
                        tsrq_min_alpha = Some(
                            take_arg(&args, &mut i)
                                .parse()
                                .unwrap_or_else(|_| fail("--tsrq-min-alpha 非 f32")),
                        )
                    }
                    "--tsrq-clamp" => {
                        tsrq_clamp = Some(
                            take_arg(&args, &mut i)
                                .parse()
                                .unwrap_or_else(|_| fail("--tsrq-clamp 非 f32")),
                        )
                    }
                    // Phase E1：--quality off|full 预设闭集（默认 off）。
                    "--quality" => {
                        quality_full = match take_arg(&args, &mut i).as_str() {
                            "off" => false,
                            "full" => true,
                            other => fail(&format!("--quality 档 {other} 越闭集(off|full)")),
                        }
                    }
                    "--smooth-normals" => smooth_normals = take_arg(&args, &mut i),
                    "--ggx" => ggx = take_arg(&args, &mut i),
                    // A1 灯光提取加性臂四参数（off 默认 = 零消费）。
                    "--lamp-lights" => lamp_lights = take_arg(&args, &mut i),
                    "--lamp-gain" => {
                        lamp_gain = Some(
                            take_arg(&args, &mut i)
                                .parse()
                                .unwrap_or_else(|_| fail("--lamp-gain 非 f32")),
                        )
                    }
                    "--lamp-k" => {
                        lamp_k = Some(
                            take_arg(&args, &mut i)
                                .parse()
                                .unwrap_or_else(|_| fail("--lamp-k 非 usize")),
                        )
                    }
                    "--lamp-contrib" => {
                        lamp_contrib = Some(
                            take_arg(&args, &mut i)
                                .parse()
                                .unwrap_or_else(|_| fail("--lamp-contrib 非 f32")),
                        )
                    }
                    "--lamp-stats-out" => lamp_stats_out = Some(take_arg(&args, &mut i)),
                    "--calibration-seed" => calibration = true,
                    "--contract" => contract_path = take_arg(&args, &mut i),
                    "--gltf" => gltf_path = take_arg(&args, &mut i),
                    "--spv-scene" => spv_scene = take_arg(&args, &mut i),
                    "--spv-mv" => spv_mv = take_arg(&args, &mut i),
                    "--spv-resample" => spv_resample = take_arg(&args, &mut i),
                    "--spv-resolve" => spv_resolve = take_arg(&args, &mut i),
                    "--out-root" => out_root = take_arg(&args, &mut i),
                    "--expect-digest" => expect_digest = Some(take_arg(&args, &mut i)),
                    "--presentation-profile" => {
                        presentation_profile = Some(take_arg(&args, &mut i))
                    }
                    "--export-png" => export_png = true,
                    // C7 profiler 输出面（逐 pass GPU/CPU 段 + mean/p50/p99 机器可读
                    // JSON;默认关 = 零收集零写盘零渲染语义变更）。
                    "--profile-json" => profile_json = Some(take_arg(&args, &mut i)),
                    // G31+ 波 A Task A4：动态场景更新通路 demo（逐帧实例变换 +
                    // TLAS refit/rebuild 策略臂；仅 --bench tsr_device inflight=1）。
                    "--dyn-demo" => dyn_demo = Some(take_arg(&args, &mut i)),
                    // G31+ 波 B Task B5：蒙皮角色进生产帧 demo（脚本化骨骼动画
                    // 驱动;无值标志）。
                    "--skin-demo" => skin_demo = true,
                    // G31+ #58：簇 DAG LOD 参数（off|leaf|on / 簇包路径 / 阈值
                    // / E 驻留压力臂）。
                    "--cluster-lod" => cluster_lod_mode = take_arg(&args, &mut i),
                    "--cluster-pack" => cluster_pack = take_arg(&args, &mut i),
                    "--cluster-error-px" => {
                        cluster_error_px = take_arg(&args, &mut i)
                            .parse()
                            .unwrap_or_else(|_| fail("--cluster-error-px 非 f32"))
                    }
                    "--cluster-resident-pages" => {
                        cluster_resident_pages = take_arg(&args, &mut i)
                            .parse()
                            .unwrap_or_else(|_| fail("--cluster-resident-pages 非 u32"))
                    }
                    // G31+ #95/#68：WP cell + HLOD 参数（off|full|on / cell 包
                    // 路径 / L0 阈值 / 距离环 / 预热帧 / 流送预算）。
                    "--wp-hlod" => wp_hlod_mode = take_arg(&args, &mut i),
                    "--wp-pack" => wp_pack = take_arg(&args, &mut i),
                    "--wp-threshold-l0" => {
                        wp_threshold_l0 = take_arg(&args, &mut i)
                            .parse()
                            .unwrap_or_else(|_| fail("--wp-threshold-l0 非 f64"))
                    }
                    "--wp-radius" => {
                        wp_radius = take_arg(&args, &mut i)
                            .parse()
                            .unwrap_or_else(|_| fail("--wp-radius 非 f32"))
                    }
                    "--wp-warmup" => {
                        wp_warmup = take_arg(&args, &mut i)
                            .parse()
                            .unwrap_or_else(|_| fail("--wp-warmup 非 u32"))
                    }
                    "--wp-budget-cells" => {
                        wp_budget_cells = take_arg(&args, &mut i)
                            .parse()
                            .unwrap_or_else(|_| fail("--wp-budget-cells 非 u32"))
                    }
                    other => fail(&format!("未知参数 {other}")),
                }
                i += 1;
            }
            // 画质战役 Phase E1 --quality full 预设展开（解析层——先于全部
            // 臂校验/换载,下游与显式写法完全同路径 ⇒ 位级等价）。bench full
            // = 质量腿子集：窗口九臂中 --textures（texel heap 侧表 bench
            // 车道未接线,Phase B 留窗）、--bloom/--dither/--auto-exposure
            // （窗口 presented 显示链专属,bench EXR 在 encode 上游无此三面）
            // 如实不展开。展开面旗标显式重叠 = fail-closed（双重指定无裁决
            // 面,拒跑不猜）。RURIX_G18_AMBIENT env 缺席时注入战役终态档
            // 0.004（lane_body OnceLock 预设槽,env 在位一律优先——
            // forbid(unsafe_code) + edition 2024 下 env::set_var 为 unsafe
            // 不可用;与 env 路径同字面同 parse ⇒ f32 位级同值）。
            if quality_full {
                const QUALITY_FULL_EXPANSION: [&str; 7] = [
                    "--smooth-normals",
                    "--ggx",
                    "--lamp-lights",
                    "--lamp-gain",
                    "--gi2",
                    "--gi2-clamp",
                    "--tsr-quality",
                ];
                let dup: Vec<&str> = QUALITY_FULL_EXPANSION
                    .into_iter()
                    .filter(|f| args.iter().any(|a| a == f))
                    .collect();
                if !dup.is_empty() {
                    fail(&format!(
                        "--quality full 与显式旗标 {} 冲突（预设 = 解析层一键展开,重叠指定即语义歧义;微调请弃预设走全显式写法,fail-closed）",
                        dup.join(" ")
                    ));
                }
                smooth_normals = "on".to_owned();
                ggx = "on".to_owned();
                lamp_lights = "on".to_owned();
                lamp_gain = Some(4.0);
                gi2 = "on".to_owned();
                gi2_clamp = Some(0.01);
                tsr_quality = "on".to_owned();
                let _ = G18_AMBIENT_PRESET.set(
                    "0.004"
                        .parse::<f32>()
                        .unwrap_or_else(|_| fail("--quality full 预设环境光字面解析失败（不可达）")),
                );
            }
            if scene_id.is_empty() || tier == 0 || backend.is_empty() {
                fail("参数闭集缺行（scene/tier/backend）");
            }
            if gi != "off" && gi != "on" {
                fail(&format!(
                    "--gi {gi}：只接受 off|on（off=直接光默认臂 0-byte；on=RFC-0031 加性多反弹）"
                ));
            }
            if smooth_normals != "off" && smooth_normals != "on" {
                fail(&format!(
                    "--smooth-normals {smooth_normals}：只接受 off|on（off=默认臂 0-byte；on=D2 平滑顶点法线加性臂）"
                ));
            }
            if smooth_normals == "on" && gi == "on" {
                fail("--smooth-normals on 与 --gi on 互斥（内容模型不同构：g16 多反弹 kernel 面无 trinrm 绑定面；闭集拒绝）");
            }
            if gi == "on" && spv_scene == DEFAULT_SPV_SCENE {
                spv_scene = DEFAULT_SPV_GI.to_owned();
            }
            if presentation_profile.is_some() && spv_scene == DEFAULT_SPV_SCENE && gi == "off" {
                spv_scene = DEFAULT_SPV_G18_LIGHT.to_owned();
            }
            // D2：平滑法线臂换 scene kernel（g18 逐字 fork + trinrm 第 8 路 +
            // params[43] 门；默认/g18 两档字面才换——用户显式 --spv-scene 面
            // 尊重不覆盖；on 时车道描述组走 MegaSmoothNrm，见 bench/render 腿）。
            if smooth_normals == "on"
                && (spv_scene == DEFAULT_SPV_SCENE || spv_scene == DEFAULT_SPV_G18_LIGHT)
            {
                spv_scene = DEFAULT_SPV_G18_SMOOTH_NRM.to_owned();
            }
            if !(1..=3).contains(&inflight) {
                fail(
                    "--inflight 只接受 1|2|3（1 = 顺序全同步既有面 0-byte；2/3 = FIF 真流水深度）",
                );
            }
            if inflight != 1 && (!bench || backend != "tsr_device") {
                fail(&format!(
                    "--inflight {inflight} 仅 --bench --backend tsr_device 已接线（G31+ 波 A Task A2 静态臂 + G38 L2a 动态臂〔--dyn-demo〕消费面；其余臂/render 腿未消费,fail-closed）"
                ));
            }
            if inflight > 1 && warmup + 1 < inflight {
                fail(
                    "--inflight N 要求 --warmup ≥ N−1（填充段须落 warmup,防测量面混入无 collect 迭代）",
                );
            }
            // G31+ 波 A Task A4 --dyn-demo 闭集校验（fail-closed，不静默降级）：
            // ① 策略字面闭集 refit|rebuild；② 仅 --bench tsr_device（MegaDyn
            // 车道唯一接线面）；③ inflight 1|2|3——1 = 顺序入口既有面 0-byte；
            // 2|3 = **G38（RFC-0030 v1.1 §4.3 L2a）每槽 AS 副本 opt-in FIF**
            // （session AS 表 ×inflight 同构副本组 + 平行入口
            // submit_with_frame_update_slot_as，逐帧更新/scene 绑定落 base+slot；
            // AS 面内存 ×S 显式代价，预算门 g31.fif_dyn.slot_as_group_mem_bytes；
            // --warmup ≥ inflight−1 通则已覆盖填充段）；④ bistro-interior
            // 唯一场景（cornell Split 形态未接线）；⑤ 不与 --gi on /
            // presentation-profile 同跑（dyn kernel = 直接光唯一内容模型）。
            let dyn_spec = match dyn_demo.as_deref() {
                None => None,
                Some("refit") => Some(DynDemoSpec {
                    refit: true,
                    spv_scene: DEFAULT_SPV_DYN_SCENE.to_owned(),
                }),
                Some("rebuild") => Some(DynDemoSpec {
                    refit: false,
                    spv_scene: DEFAULT_SPV_DYN_SCENE.to_owned(),
                }),
                Some(other) => fail(&format!(
                    "--dyn-demo {other}：只接受 refit|rebuild（refit = TLAS UPDATE 优先策略；rebuild = BUILD 回退策略）"
                )),
            };
            if dyn_spec.is_some() {
                if !bench || backend != "tsr_device" {
                    fail("--dyn-demo 仅 --bench --backend tsr_device 已接线（MegaDyn 动态车道唯一消费面；其余臂 fail-closed）");
                }
                // G38（RFC-0030 v1.1 §4.3 L2a）：原「--dyn-demo 要求 --inflight 1」
                // 强制解除——inflight 2|3 走 slot_as 每槽 AS 副本 FIF 路径（lane
                // 侧 create_with_slot_as 显式建组 + rt 入口槽纪律三判据提交前
                // fail-closed 复核）；既有拒绝面（submit_with_frame_update 对
                // tlas_update 的 fail-closed）字面 0-byte 不动，本臂走加性平行
                // 入口。inflight=1 顺序入口既有面 0-byte。
                if scene_id != "bistro-interior" {
                    fail("--dyn-demo 仅 bistro-interior 已接线（cornell Split 六 pass 形态未接 MegaDyn 动态车道，fail-closed）");
                }
                if gi != "off" || presentation_profile.is_some() {
                    fail("--dyn-demo 不与 --gi on / --presentation-profile 同跑（dyn kernel = 直接光唯一内容模型，与 g14_3_direct_gi 逐字镜像 + 实例分派）");
                }
            }
            // G31+ 波 B Task B5 --skin-demo 闭集校验（fail-closed,不静默降级）：
            // ① 仅 --bench tsr_device（MegaSkin 车道唯一接线面）；② inflight
            // 恒 1——**蒙皮 × slot_as 批次 B 留窗**（G38：RFC-0030 v1.1 §4.3
            // L2a 通路 rt 侧已支持 blas_refit 槽纪律同律，g14_3 接线精确计划 =
            // artifacts/day_0830_g38/t2_fifdyn/WIRING_PLAN.md §1-A6〔scene
            // pass=1 override / BlasRefitUpdate.as_index 逐帧 base+slot /
            // 与 T3 bridge_ext 加性面协调〕；接线前本拒绝面维持,不静默降级）；
            // ③ bistro-interior 唯一场景（cornell Split 形态未接线）；④ 与
            // --dyn-demo 互斥、不与 --gi on / --presentation-profile 同跑
            // （skin scene kernel = g31_dyn_scene 镜像直接光唯一内容模型）。
            let skin_spec = if skin_demo {
                if !bench || backend != "tsr_device" {
                    fail("--skin-demo 仅 --bench --backend tsr_device 已接线（MegaSkin 蒙皮车道唯一消费面；其余臂 fail-closed）");
                }
                if inflight != 1 {
                    fail("--skin-demo 要求 --inflight 1（蒙皮 × slot_as 批次 B 留窗：RFC-0030 v1.1 §4.3 L2a 通路 rt 侧已支持〔blas_refit 槽纪律同律〕,g14_3 接线计划 = artifacts/day_0830_g38/t2_fifdyn/WIRING_PLAN.md §1-A6;接线前 fail-closed 维持,蒙皮车道走顺序入口）");
                }
                if scene_id != "bistro-interior" {
                    fail("--skin-demo 仅 bistro-interior 已接线（cornell Split 六 pass 形态未接 MegaSkin 蒙皮车道，fail-closed）");
                }
                if dyn_spec.is_some() {
                    fail("--skin-demo 与 --dyn-demo 互斥（动态内容面各自独立,叠加无意义;闭集拒绝）");
                }
                if gi != "off" || presentation_profile.is_some() {
                    fail("--skin-demo 不与 --gi on / --presentation-profile 同跑（skin scene kernel = g31_dyn_scene 镜像直接光唯一内容模型）");
                }
                Some(SkinDemoSpec {
                    spv_skin: DEFAULT_SPV_SKIN.to_owned(),
                    spv_scene: DEFAULT_SPV_SKIN_SCENE.to_owned(),
                    spv_mv: DEFAULT_SPV_SKIN_MV.to_owned(),
                })
            } else {
                None
            };
            if gltf_path.is_empty() {
                gltf_path = default_gltf(&scene_id).to_owned();
            }
            // C7 --profile-json 闭集校验（fail-closed,不静默降级）：首接面 =
            // tsr_device 静态臂 inflight=1（生产 bench 口径）;vendor 双臂/FIF
            // 流水/动态面归后续,如实拒跑不冒充。
            if profile_json.is_some() {
                if !bench || backend != "tsr_device" {
                    fail("--profile-json 仅 --bench --backend tsr_device 已接线（C7 profiler 输出面首接臂;vendor 双臂/render 腿未接线,fail-closed）");
                }
                if inflight != 1 {
                    fail("--profile-json 要求 --inflight 1（C7 首接面 = 顺序静态臂;FIF 流水面归后续,如实拒跑）");
                }
                if dyn_spec.is_some() || skin_spec.is_some() {
                    fail("--profile-json 不与 --dyn-demo/--skin-demo 同跑（C7 首接面 = 静态生产臂;动态/蒙皮面归后续,如实拒跑）");
                }
            }
            // G31+ #58 --cluster-lod 闭集校验（fail-closed，不静默降级）：
            // ① 模式字面闭集 off|leaf|on；② leaf/on 要求 --cluster-pack；
            // ③ 阈值必须为正有限。
            // G36 W2 互斥解除：与 --dyn-demo/--skin-demo 组合面成立——动态/
            // 蒙皮尾接段基址在 apply_* 重建**之后**计算（lane_assets_dyn/
            // lane_assets_skin 消费重建后 scene.indices.len(),基址假设不再
            // 依赖装配序;原互斥字面留此注释存证,fail 行撤除）。
            let cluster_opt = match cluster_lod_mode.as_str() {
                "off" => ClusterLodOpt::off(),
                m @ ("leaf" | "on") => {
                    if cluster_pack.is_empty() {
                        fail("--cluster-lod leaf|on 要求 --cluster-pack <RXCP>（g31_cluster_lod_bake 产物）");
                    }
                    if !(cluster_error_px.is_finite() && cluster_error_px > 0.0) {
                        fail("--cluster-error-px 必须为正有限 f32");
                    }
                    if cluster_resident_pages > 0 && m != "on" {
                        fail("--cluster-resident-pages 仅 --cluster-lod on 消费（leaf 全叶对拍锚不与驻留压力臂混口径）");
                    }
                    ClusterLodOpt {
                        mode: if m == "leaf" {
                            ClusterLodMode::Leaf
                        } else {
                            ClusterLodMode::On
                        },
                        pack_path: cluster_pack.clone(),
                        threshold_px: cluster_error_px,
                        resident_pages: cluster_resident_pages,
                    }
                }
                other => fail(&format!("--cluster-lod {other}：只接受 off|leaf|on")),
            };
            // G31+ #95/#68 --wp-hlod 闭集校验（fail-closed，不静默降级）：
            // ① 模式字面闭集 off|full|on；② full/on 要求 --wp-pack；③ 参数
            // 域校验。
            // G36 W2 互斥解除：与 --cluster-lod 组合走 apply_geo_combined
            // （WP cell 互斥选层先行 → Full 域内簇 cut → 跨界粗簇叶级回退;
            // 零双绘/覆盖机核 fail-closed,leaf×full 极限 == off 逐位锚）;与
            // --dyn-demo/--skin-demo 组合同 --cluster-lod 行注释（尾接段基址
            // 后移成立;原互斥字面留此注释存证,fail 行撤除）。
            let wp_opt = match wp_hlod_mode.as_str() {
                "off" => WpHlodOpt::off(),
                m @ ("full" | "on") => {
                    if wp_pack.is_empty() {
                        fail("--wp-hlod full|on 要求 --wp-pack <RXWH>（g31_wp_hlod_bake 产物）");
                    }
                    if !(wp_threshold_l0.is_finite() && wp_threshold_l0 > 0.0) {
                        fail("--wp-threshold-l0 必须为正有限 f64");
                    }
                    if !(wp_radius.is_finite() && wp_radius > 0.0) {
                        fail("--wp-radius 必须为正有限 f32");
                    }
                    if wp_warmup == 0 {
                        fail("--wp-warmup 必须 ≥1（预热协议:切换请求 → 原子翻转间隔）");
                    }
                    WpHlodOpt {
                        mode: if m == "full" {
                            WpHlodMode::Full
                        } else {
                            WpHlodMode::On
                        },
                        pack_path: wp_pack.clone(),
                        threshold_l0: wp_threshold_l0,
                        loading_radius_m: wp_radius,
                        inner_radius_m: (wp_radius * 0.25).max(1.0),
                        budget_cells: wp_budget_cells.max(1),
                        warmup_frames: wp_warmup,
                    }
                }
                other => fail(&format!("--wp-hlod {other}：只接受 off|full|on")),
            };
            // D2 --smooth-normals 闭集校验（fail-closed，不静默降级）：
            // ① 仅 tsr_device（vendor 双臂车道无 trinrm 绑定面）；
            // ② 不与 --dyn-demo/--skin-demo 同跑（动态/蒙皮 scene kernel 面
            //    内容模型不同构）；
            // ③ 不与 --cluster-lod/--wp-hlod 同跑（几何重建后法线侧表
            //    gather 未接线——D2 登记留窗）。
            if smooth_normals == "on" {
                if backend != "tsr_device" {
                    fail("--smooth-normals on 仅 --backend tsr_device 已接线（vendor 双臂无 trinrm 绑定面，fail-closed）");
                }
                if dyn_spec.is_some() || skin_spec.is_some() {
                    fail("--smooth-normals on 不与 --dyn-demo/--skin-demo 同跑（动态/蒙皮 scene kernel 面无 trinrm 绑定面，fail-closed）");
                }
                if cluster_lod_mode != "off" || wp_hlod_mode != "off" {
                    fail("--smooth-normals on 暂不与 --cluster-lod/--wp-hlod 同跑（几何重建后法线侧表 gather 未接线——D2 登记留窗，fail-closed）");
                }
            }
            // D6 --ggx 闭集校验（fail-closed，不静默降级）：① 字面闭集
            // off|on；② 须随 --smooth-normals on（GGX 高光依赖平滑法线才
            // 正确——flat 面法线下高光逐三角不连续无意义；且 tri_mr 绑定面
            // 仅 MegaSmoothNrm 形态存在）；③ 互斥集与 --smooth-normals 同
            // （gi/dyn/skin/cluster/wp/vendor 双臂——上行校验已裁，本块登记
            // 不重复 fail）。
            if ggx != "off" && ggx != "on" {
                fail(&format!(
                    "--ggx {ggx}：只接受 off|on（off=默认臂 0-byte；on=D6 GGX 高光材质加性臂）"
                ));
            }
            if ggx == "on" && smooth_normals != "on" {
                fail("--ggx on 须随 --smooth-normals on（GGX 依赖平滑法线；tri_mr 绑定面仅 MegaSmoothNrm 形态存在，fail-closed）");
            }
            // A1 --lamp-lights 闭集校验（fail-closed，不静默降级）：① 字面
            // 闭集 off|on；② 须随 --smooth-normals on（提取灯的半径阴影
            // 截断/贡献剔除消费面仅 g18_smooth_nrm kernel 存在——母版/默认
            // kernel 不读 points 槽 7 与 params[49]，开臂无语义）；③ 子参数
            // （gain/k/contrib/stats-out）须随 on（off 面零消费）；④ 参数域
            // 校验（gain 正有限 / k ≥ 1 / contrib 非负有限）。互斥集随
            // --smooth-normals（gi/dyn/skin/cluster/wp/vendor 上行已裁）。
            if lamp_lights != "off" && lamp_lights != "on" {
                fail(&format!(
                    "--lamp-lights {lamp_lights}：只接受 off|on（off=默认臂 0-byte；on=A1 灯光提取加性臂）"
                ));
            }
            if lamp_lights == "on" && smooth_normals != "on" {
                fail("--lamp-lights on 须随 --smooth-normals on（半径阴影截断/贡献剔除消费面仅 g18_smooth_nrm kernel 存在，fail-closed）");
            }
            if lamp_lights != "on"
                && (lamp_gain.is_some()
                    || lamp_k.is_some()
                    || lamp_contrib.is_some()
                    || lamp_stats_out.is_some())
            {
                fail("--lamp-gain/--lamp-k/--lamp-contrib/--lamp-stats-out 须随 --lamp-lights on（off 面零消费，fail-closed）");
            }
            let lamp_gain_v = lamp_gain.unwrap_or(1.0);
            let lamp_k_v = lamp_k.unwrap_or(12);
            let lamp_contrib_v = lamp_contrib.unwrap_or(0.0);
            if lamp_lights == "on" {
                if !(lamp_gain_v.is_finite() && lamp_gain_v > 0.0) {
                    fail("--lamp-gain 必须为正有限 f32");
                }
                if lamp_k_v == 0 {
                    fail("--lamp-k 必须 ≥1");
                }
                if !(lamp_contrib_v.is_finite() && lamp_contrib_v >= 0.0) {
                    fail("--lamp-contrib 必须为非负有限 f32");
                }
            }
            let lamp_opt = if lamp_lights == "on" {
                LampOpt {
                    enabled: true,
                    gain: lamp_gain_v,
                    max_k: lamp_k_v,
                    contrib: lamp_contrib_v,
                    stats_out: lamp_stats_out.clone().unwrap_or_default(),
                }
            } else {
                LampOpt::off()
            };
            // Phase C --gi2 闭集校验（fail-closed，不静默降级）：① 字面闭集
            // off|on；② 须随 --smooth-normals on（GI2 段仅统一质量 kernel
            // 存在，反弹沿平滑法线半球——flat 法线下与母版语义漂移；互斥集
            // 〔gi/dyn/skin/cluster/wp/vendor〕随 --smooth-normals 上行已裁）；
            // ③ 须 --inflight 1（[52]=frame_idx 逐帧挂载走顺序循环——FIF
            // submit 面未接线）；④ 子参数须随 on（off 面零消费）；⑤ 参数域
            // （scale 正有限 / clamp 正有限）。
            if gi2 != "off" && gi2 != "on" {
                fail(&format!(
                    "--gi2 {gi2}：只接受 off|on（off=默认臂 0-byte；on=Phase C R2 低差异 1 反弹间接光加性臂）"
                ));
            }
            if gi2 == "on" && smooth_normals != "on" {
                fail("--gi2 on 须随 --smooth-normals on（GI2 段仅 g31_texture_nrm_gi 统一质量 kernel 存在，fail-closed）");
            }
            if gi2 == "on" && inflight != 1 {
                fail("--gi2 on 要求 --inflight 1（params[52]=frame_idx 逐帧挂载走顺序循环，FIF 流水面未接线，fail-closed）");
            }
            if gi2 != "on" && (gi2_scale.is_some() || gi2_clamp.is_some()) {
                fail("--gi2-scale/--gi2-clamp 须随 --gi2 on（off 面零消费，fail-closed）");
            }
            let gi2_scale_v = gi2_scale.unwrap_or(1.0);
            let gi2_clamp_v = gi2_clamp.unwrap_or(4.0);
            if gi2 == "on" {
                if !(gi2_scale_v.is_finite() && gi2_scale_v > 0.0) {
                    fail("--gi2-scale 必须为正有限 f32");
                }
                if !(gi2_clamp_v.is_finite() && gi2_clamp_v > 0.0) {
                    fail("--gi2-clamp 必须为正有限 f32");
                }
            }
            // GI2 scene SPV 换载（默认/g18/smooth_nrm 三档字面才换——用户显式
            // --spv-scene 面尊重不覆盖；须为 g31_texture_nrm_gi 14 路绑定面）。
            if gi2 == "on"
                && (spv_scene == DEFAULT_SPV_SCENE
                    || spv_scene == DEFAULT_SPV_G18_LIGHT
                    || spv_scene == DEFAULT_SPV_G18_SMOOTH_NRM)
            {
                spv_scene = DEFAULT_SPV_G31_TEXNRM_GI.to_owned();
            }
            let gi2_opt = if gi2 == "on" {
                Gi2Opt {
                    enabled: true,
                    scale: gi2_scale_v,
                    clamp: gi2_clamp_v,
                }
            } else {
                Gi2Opt::off()
            };
            // Phase D --tsr-quality 闭集校验（fail-closed，不静默降级）：
            // ① 字面闭集 off|on；② 仅 --backend tsr_device（vendor 双臂无本
            // resolve kernel 消费面）；③ 不与 --dyn-demo/--skin-demo 同跑
            // （demo 面 prepare 路 tsr_params[19..21) 未接线——skin 自有
            // prepare 不消费,如实拒跑不冒充）；④ 子参数须随 on（off 面零
            // 消费）；⑤ 参数域（min-alpha ∈ (0,1) 有限 / clamp 非负有限）。
            // 与 --smooth-normals/--ggx/--lamp-lights/--gi2 全可组合（resolve
            // 面在 scene 面下游正交）。
            if tsr_quality != "off" && tsr_quality != "on" {
                fail(&format!(
                    "--tsr-quality {tsr_quality}：只接受 off|on（off=默认臂冻结 resolve 字节 0-byte；on=Phase D 降噪质量档 resolve 变体换载）"
                ));
            }
            if tsr_quality == "on" && backend != "tsr_device" {
                fail("--tsr-quality on 仅 --backend tsr_device 已接线（vendor 双臂无 TSR resolve kernel 消费面，fail-closed）");
            }
            if tsr_quality == "on" && (dyn_spec.is_some() || skin_spec.is_some()) {
                fail("--tsr-quality on 不与 --dyn-demo/--skin-demo 同跑（demo 车道 prepare 路 tsr_params[19..21) 未接线，fail-closed）");
            }
            if tsr_quality != "on" && (tsrq_min_alpha.is_some() || tsrq_clamp.is_some()) {
                fail("--tsrq-min-alpha/--tsrq-clamp 须随 --tsr-quality on（off 面零消费，fail-closed）");
            }
            let tsrq_min_alpha_v = tsrq_min_alpha.unwrap_or(0.02);
            let tsrq_clamp_v = tsrq_clamp.unwrap_or(0.0);
            if tsr_quality == "on" {
                if !(tsrq_min_alpha_v.is_finite() && tsrq_min_alpha_v > 0.0 && tsrq_min_alpha_v < 1.0) {
                    fail("--tsrq-min-alpha 必须 ∈ (0,1) 有限 f32");
                }
                if !(tsrq_clamp_v.is_finite() && tsrq_clamp_v >= 0.0) {
                    fail("--tsrq-clamp 必须为非负有限 f32");
                }
            }
            // Phase D resolve SPV 换载（默认字面才换——用户显式 --spv-resolve
            // 面尊重不覆盖；字节隔离：off 臂恒载 m_c 冻结字节，on 臂独载
            // g31_tsr_resolve_q.spv——C 相纪律「保锚一律字节隔离」）。
            if tsr_quality == "on" && spv_resolve == DEFAULT_SPV_RESOLVE {
                spv_resolve = DEFAULT_SPV_RESOLVE_Q.to_owned();
            }
            let tsrq_opt = if tsr_quality == "on" {
                TsrqOpt {
                    enabled: true,
                    min_alpha: tsrq_min_alpha_v,
                    clamp: tsrq_clamp_v,
                }
            } else {
                TsrqOpt::off()
            };
            if bench {
                let frames = if frames == 0 { 160 } else { frames };
                bench_leg(
                    &scene_id,
                    tier,
                    &backend,
                    frames,
                    warmup,
                    inflight,
                    &contract_path,
                    &gltf_path,
                    &spv_scene,
                    &spv_mv,
                    &spv_resample,
                    &spv_resolve,
                    &out_root,
                    expect_digest.as_deref(),
                    dyn_spec.as_ref(),
                    skin_spec.as_ref(),
                    profile_json.as_deref(),
                    &cluster_opt,
                    &wp_opt,
                    smooth_normals == "on",
                    ggx == "on",
                    &lamp_opt,
                    &gi2_opt,
                    &tsrq_opt,
                );
            } else {
                render_leg(
                    &scene_id,
                    tier,
                    &backend,
                    frames,
                    calibration,
                    &contract_path,
                    &gltf_path,
                    &spv_scene,
                    &spv_mv,
                    &spv_resample,
                    &spv_resolve,
                    &out_root,
                    expect_digest.as_deref(),
                    &gi,
                    presentation_profile.as_deref(),
                    export_png,
                    &cluster_opt,
                    &wp_opt,
                    smooth_normals == "on",
                    ggx == "on",
                    &lamp_opt,
                    &gi2_opt,
                    &tsrq_opt,
                );
            }
        }
        other => fail(&format!("未知子模式 {other}")),
    }
}
