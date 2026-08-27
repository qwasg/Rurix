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
        // G31+ #58 步骤 1/3：装配产物 dump（RXCS v1）——簇 DAG 离线 bake
        //（rurix-asset g31_cluster_lod_bake）的唯一装配输入面（装配语义单源，
        // bake 侧禁复刻装配）。纯 host，GPU 非必需。
        "--dump-scene" => {
            let mut scene_id = String::new();
            let mut contract_path = DEFAULT_CONTRACT.to_owned();
            let mut gltf_path = String::new();
            let mut out_path = String::new();
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--scene" => scene_id = take_arg(&args, &mut i),
                    "--contract" => contract_path = take_arg(&args, &mut i),
                    "--gltf" => gltf_path = take_arg(&args, &mut i),
                    "--out" => out_path = take_arg(&args, &mut i),
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
            let scene = assemble_scene_ex(
                &contract.raw,
                &scene_id,
                Path::new(&gltf_path),
                Some(&mut groups),
                None,
            )
            .unwrap_or_else(|e| fail(&format!("场景装配: {e}")));
            dump_scene_rxcs(&scene, &groups, Path::new(&out_path))
                .unwrap_or_else(|e| fail(&e));
            println!(
                "{TAG}: dump-scene OK scene={scene_id} tris={} groups={} emissive_tris={} quads={} -> {out_path}",
                scene.tri_count,
                groups.len(),
                scene.emissive_tri_count,
                scene.quads.len(),
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
            if scene_id.is_empty() || tier == 0 || backend.is_empty() {
                fail("参数闭集缺行（scene/tier/backend）");
            }
            if gi != "off" && gi != "on" {
                fail(&format!(
                    "--gi {gi}：只接受 off|on（off=直接光默认臂 0-byte；on=RFC-0031 加性多反弹）"
                ));
            }
            if gi == "on" && spv_scene == DEFAULT_SPV_SCENE {
                spv_scene = DEFAULT_SPV_GI.to_owned();
            }
            if presentation_profile.is_some() && spv_scene == DEFAULT_SPV_SCENE && gi == "off" {
                spv_scene = DEFAULT_SPV_G18_LIGHT.to_owned();
            }
            if !(1..=3).contains(&inflight) {
                fail(
                    "--inflight 只接受 1|2|3（1 = 顺序全同步既有面 0-byte；2/3 = FIF 真流水深度）",
                );
            }
            if inflight != 1 && (!bench || backend != "tsr_device") {
                fail(&format!(
                    "--inflight {inflight} 仅 --bench --backend tsr_device 已接线（G31+ 波 A Task A2 消费面；其余臂/render 腿未消费,fail-closed）"
                ));
            }
            if inflight > 1 && warmup + 1 < inflight {
                fail(
                    "--inflight N 要求 --warmup ≥ N−1（填充段须落 warmup,防测量面混入无 collect 迭代）",
                );
            }
            // G31+ 波 A Task A4 --dyn-demo 闭集校验（fail-closed，不静默降级）：
            // ① 策略字面闭集 refit|rebuild；② 仅 --bench tsr_device（MegaDyn
            // 车道唯一接线面）；③ inflight 恒 1（A2 约束：FIF 流水公共入口拒
            // tlas_update——共享 instance buffer host 写面在飞帧不可改写；动态
            // 面走顺序入口，per-slot 实例缓冲归后续波）；④ bistro-interior
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
                if inflight != 1 {
                    fail("--dyn-demo 要求 --inflight 1（A2 约束：FIF 流水入口拒 tlas_update——共享 instance buffer host 写面在飞帧不可改写；动态场景走顺序入口，per-slot 实例缓冲归后续波）");
                }
                if scene_id != "bistro-interior" {
                    fail("--dyn-demo 仅 bistro-interior 已接线（cornell Split 六 pass 形态未接 MegaDyn 动态车道，fail-closed）");
                }
                if gi != "off" || presentation_profile.is_some() {
                    fail("--dyn-demo 不与 --gi on / --presentation-profile 同跑（dyn kernel = 直接光唯一内容模型，与 g14_3_direct_gi 逐字镜像 + 实例分派）");
                }
            }
            // G31+ 波 B Task B5 --skin-demo 闭集校验（fail-closed,不静默降级）：
            // ① 仅 --bench tsr_device（MegaSkin 车道唯一接线面）；② inflight
            // 恒 1（A2 同律约束：FIF 流水入口拒 blas_refit——BLAS 顶点缓冲为
            // 共享写面,在飞帧 ray query 读取中不可改写）；③ bistro-interior
            // 唯一场景（cornell Split 形态未接线）；④ 与 --dyn-demo 互斥、不
            // 与 --gi on / --presentation-profile 同跑（skin scene kernel =
            // g31_dyn_scene 镜像直接光唯一内容模型）。
            let skin_spec = if skin_demo {
                if !bench || backend != "tsr_device" {
                    fail("--skin-demo 仅 --bench --backend tsr_device 已接线（MegaSkin 蒙皮车道唯一消费面；其余臂 fail-closed）");
                }
                if inflight != 1 {
                    fail("--skin-demo 要求 --inflight 1（A2 同律约束：FIF 流水入口拒 blas_refit——BLAS 顶点缓冲为共享写面,在飞帧 ray query 读取中不可改写；蒙皮车道走顺序入口）");
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
            // ① 模式字面闭集 off|leaf|on；② leaf/on 要求 --cluster-pack；③ 与
            // --dyn-demo/--skin-demo 互斥（cut 重排三角汤 ⇒ 动态段基址/蒙皮源
            // 段假设破坏，合流归后续波）；④ 阈值必须为正有限。
            let cluster_opt = match cluster_lod_mode.as_str() {
                "off" => ClusterLodOpt::off(),
                m @ ("leaf" | "on") => {
                    if cluster_pack.is_empty() {
                        fail("--cluster-lod leaf|on 要求 --cluster-pack <RXCP>（g31_cluster_lod_bake 产物）");
                    }
                    if dyn_spec.is_some() || skin_spec.is_some() {
                        fail("--cluster-lod 不与 --dyn-demo/--skin-demo 同跑（cut 重排三角汤,动态段/蒙皮段基址假设破坏;合流归后续波,fail-closed）");
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
            // ① 模式字面闭集 off|full|on；② full/on 要求 --wp-pack；③ 与
            // --cluster-lod/--dyn-demo/--skin-demo 互斥（两套几何重组各自重排
            // 三角汤,叠加组合面归后续波）；④ 参数域校验。
            let wp_opt = match wp_hlod_mode.as_str() {
                "off" => WpHlodOpt::off(),
                m @ ("full" | "on") => {
                    if wp_pack.is_empty() {
                        fail("--wp-hlod full|on 要求 --wp-pack <RXWH>（g31_wp_hlod_bake 产物）");
                    }
                    if cluster_opt.mode != ClusterLodMode::Off {
                        fail("--wp-hlod 不与 --cluster-lod 同跑（两套几何重组各自重排三角汤;组合面归后续波,fail-closed）");
                    }
                    if dyn_spec.is_some() || skin_spec.is_some() {
                        fail("--wp-hlod 不与 --dyn-demo/--skin-demo 同跑（cell 重组三角汤,动态段/蒙皮段基址假设破坏;合流归后续波,fail-closed）");
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
                );
            }
        }
        other => fail(&format!("未知子模式 {other}")),
    }
}
