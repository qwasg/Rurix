//! G9.3 M92 GPU 蒙皮 device 对拍 harness(RXS-0353;门
//! `g9.p1.m92.gpu_skinning_lod_update`)。
//!
//! ## 判据面(G9_ACCEPTANCE_MAP §3 M92 行)
//!
//! - **逐顶点对拍容差 0**(定点化输入域):确定性 fixture
//!   ([`skinning::m92_fixture`],两簇 × 三姿态,含 90° 旋转/大平移对抗姿态)
//!   经 device 蒙皮 compute kernel(手编 SPV,[`skin_kernel::m92_skin_spv`],
//!   render_exec 骨架真跑;全部 FAdd/FSub/FMul 挂 NoContraction,禁驱动收缩)
//!   → 回读蒙皮顶点/法向/保守包围体块,与 host Kerbl 参照(`skin_cluster`/
//!   `skin_normals`)**逐位一致**;**法向锥**同位级判据(无 sqrt 面);
//! - **包围体 100% 包含核验**:device 输出的 AABB/包围球/法向锥对 device 蒙皮
//!   顶点/法向逐姿态逐簇经 `verify_*_containment` 核验(含锥达半角边界的
//!   rot_x_90 对抗姿态);AABB/球字段的 device/host 分歧 = GLSL.std.450.Sqrt
//!   双侧舍入(实测 ulp 级),**measured 登记**进 evidence,不手写冻结容差
//!   (P-09)——判据 = 包含不变式(L2 逐字),非包围体字段位等;
//! - **档位切换双跑逐位一致**:全姿态序列双腿(两趟完整 device 运行)digest
//!   逐位一致;`SkinningDriver` 帧序列(近全速 → 静态 → 远 1/4 降级 → 更新点
//!   恢复)档位直方图/统计确定性;
//! - **静态帧零 AS 构建**:姿态 bit-equal 且档位不变帧,`AsStats` 构建/refit
//!   计数零增量(非零即 RED);更新帧 refit 计数非空可机核;
//! - **RED 臂**:人为缩小 device 包围体(AABB/球/锥三臂)必被包含核验检出;
//!   篡改回读顶点一子段必被逐顶点对拍检出(能红反证);
//! - **浮点输入域容差**:非定点变体实测 max abs/ulp 差**登记**于 evidence
//!   (`float_domain_measured`;P-09:先 measured 后冻结,本 harness 不手写
//!   冻结值、不作判据)。
//!
//! ## 三态
//!
//! 无 Vulkan loader/设备 → `G9_M92_SKIN: SKIP` + 显式 DEV_ENV_DEGRADE 登记
//! (退 0,非 fake pass;`RURIX_REQUIRE_REAL=1` 下的 SKIP→硬红由 smoke 脚本层
//! 裁决);判据不符 / RED 轴失效 → `G9_M92_SKIN: FAIL` 退 1。
//! `RURIX_VK_VALIDATION=1` 装载 validation messenger(render_exec lane 内
//! fail-closed;evidence 记 `validation_error_total`,必须 = 0)。

use rurix_render::geometry::skin_kernel::{
    self, M92_BOUND_WORDS, M92_CLUSTER_BONES, M92_INFLUENCES,
};
use rurix_render::geometry::skinning::{
    ClusterSkinInput, NormalCone, SkinPalette, SkinnedClusterFrame, SkinningDriver,
    conservative_skinned_aabb, conservative_skinned_cone, conservative_skinned_sphere, m92_fixture,
    skin_cluster, skin_normals, verify_bound_containment, verify_normal_cone_containment,
    verify_sphere_containment,
};
use rurix_render::rt::as_manager::{BlasCache, DynamicPolicy};
use rurix_rt::render_exec::{
    self, BufferDesc, BufferUsage, ComputePass, DispatchSpec, Pass, Readback, ResourceDesc,
    TargetState,
};
use rurix_rt::vk;

fn fail(msg: &str) -> ! {
    eprintln!("G9_M92_SKIN: FAIL {msg}");
    std::process::exit(1)
}

fn skip(msg: &str) -> ! {
    // 显式 DEV_ENV_DEGRADE 登记(非静默绿;REQUIRE_REAL 下的硬红归 smoke 层)。
    println!("G9_M92_SKIN: SKIP DEV_ENV_DEGRADE {msg}");
    std::process::exit(0)
}

fn hex(d: &[u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

fn storage(size: usize, data: Option<&[u8]>) -> ResourceDesc<'_> {
    ResourceDesc::Buffer(BufferDesc {
        size: size as u64,
        usage: BufferUsage {
            storage: true,
            ..Default::default()
        },
        data,
        // G14.10d 加字段后的最小修复:保持既有 host-visible 行为(0-byte)。
        device_local: false,
    })
}

fn spv_bytes(spv: &[u32]) -> Vec<u8> {
    spv.iter().flat_map(|w| w.to_le_bytes()).collect()
}

/// 单姿态双腿(两簇)device 真跑:一个 execute_frame、两个 compute pass
/// (共享 palette/bone_angle SSBO;输出各自独立缓冲),六路回读。
/// 返回 [(C0 out_pos, out_nrm, out_bound), (C1 ...)](字节)。
fn run_pose_on_device(
    spv: &[u8],
    fixture: &rurix_render::geometry::skinning::M92Fixture,
    pose: &SkinPalette,
) -> Result<[[Vec<u8>; 3]; 2], String> {
    let (palette_b, angle_b) = skin_kernel::pack_palette(pose);
    let packs: Vec<_> = fixture
        .clusters
        .iter()
        .map(|c| {
            skin_kernel::pack_cluster(
                &c.vertices,
                &c.normals,
                &c.weights,
                &c.bone_indices,
                c.bound_inflation,
                c.rest_aabb,
                &c.rest_cone,
            )
        })
        .collect();
    // 资源布局:每簇 7 输入(0..7 = pos/nrm/wval/wbone/palette/angle/cbones)
    // + 3 输出(7..10);C0 = 资源 0..10,C1 = 10..20(palette/angle 复用 4/5 字节
    // 重建独立资源——readback 下标与资源一一对应,布局直白优先)。
    let mut resources: Vec<ResourceDesc> = Vec::new();
    for (ci, p) in packs.iter().enumerate() {
        resources.push(storage(p.rest_pos.len(), Some(&p.rest_pos)));
        resources.push(storage(p.rest_nrm.len(), Some(&p.rest_nrm)));
        resources.push(storage(p.wval.len(), Some(&p.wval)));
        resources.push(storage(p.wbone.len(), Some(&p.wbone)));
        resources.push(storage(palette_b.len(), Some(&palette_b)));
        resources.push(storage(angle_b.len(), Some(&angle_b)));
        resources.push(storage(p.cluster_bones.len(), Some(&p.cluster_bones)));
        resources.push(storage(p.rest_pos.len(), None)); // out_pos
        resources.push(storage(p.rest_nrm.len(), None)); // out_nrm
        resources.push(storage(M92_BOUND_WORDS * 4, None)); // out_bound
        let _ = ci;
    }
    let mut passes: Vec<Pass> = Vec::new();
    for (ci, p) in packs.iter().enumerate() {
        let base = (ci as u32) * 10;
        passes.push(Pass::Compute(ComputePass {
            name: if ci == 0 {
                "m92_skin_c0"
            } else {
                "m92_skin_c1"
            },
            spirv: spv,
            entry: Some("main"),
            dispatch: DispatchSpec::Direct([p.n_vertices, 1, 1]),
            bindings: render_exec::Bindings {
                storage_buffers: (base..base + 10).collect(),
                push_constants: p.push.clone(),
                ..Default::default()
            },
        }));
    }
    let barriers: [&[(u32, TargetState)]; 2] = [&[], &[]];
    let mut readbacks: Vec<Readback> = Vec::new();
    for (ci, p) in packs.iter().enumerate() {
        let base = (ci as u32) * 10;
        for o in [7u32, 8, 9] {
            readbacks.push(Readback::Buffer {
                res: base + o,
                offset: 0,
                size: if o == 9 {
                    (M92_BOUND_WORDS * 4) as u64
                } else {
                    (p.n_vertices * 12) as u64
                },
            });
        }
    }
    let out = render_exec::execute_frame(&resources, &passes, &barriers, &readbacks)?;
    Ok([
        [out[0].clone(), out[1].clone(), out[2].clone()],
        [out[3].clone(), out[4].clone(), out[5].clone()],
    ])
}

/// 逐顶点逐位对拍(容差 0):首个分叉详报。
fn bitexact_compare(tag: &str, a: &[[f32; 3]], b: &[[f32; 3]]) -> Result<(), String> {
    if a.len() != b.len() {
        return Err(format!("{tag} 长度失配 {} vs {}", a.len(), b.len()));
    }
    for (i, (x, y)) in a.iter().zip(b).enumerate() {
        if x.map(f32::to_bits) != y.map(f32::to_bits) {
            return Err(format!(
                "{tag} 顶点 {i} 分叉:device={x:?} vs host={y:?}(容差 0 判据)"
            ));
        }
    }
    Ok(())
}

/// 有序浮点 ulp 距离(浮点域 measured 面)。
fn ulp_diff(a: f32, b: f32) -> u64 {
    let key = |x: f32| -> i64 {
        let bits = x.to_bits() as i32;
        i64::from(if bits < 0 { i32::MIN - bits } else { bits })
    };
    key(a).abs_diff(key(b))
}

/// 位级全等(容差 0 判据面)。
fn bits_eq(a: &[[f32; 3]], b: &[[f32; 3]]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| x.map(f32::to_bits) == y.map(f32::to_bits))
}

fn main() {
    println!(
        "[g9_m92_skinning_device] G9.3 M92 GPU 蒙皮 device 对拍 harness(RXS-0353;门 g9.p1.m92.gpu_skinning_lod_update)"
    );
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut evidence_path: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--evidence" => {
                i += 1;
                evidence_path = Some(args.get(i).expect("--evidence path").clone());
            }
            other => fail(&format!("unknown arg {other}")),
        }
        i += 1;
    }

    // ── 步骤 0:device 门(三态)──
    if !vk::vulkan_available() {
        skip("无 Vulkan loader(dev-env degrade)");
    }
    let caps = match render_exec::probe_device_caps() {
        Ok(c) => c,
        Err(e) => skip(&format!("无 Vulkan 物理设备({})", e.trim())),
    };
    if let Err(e) = render_exec::require_wave(&caps, render_exec::KernelWave::W1) {
        skip(&format!("W1 能力链缺失({e})"));
    }
    let validation_on = std::env::var("RURIX_VK_VALIDATION").as_deref() == Ok("1");
    println!(
        "G9_M92_SKIN: device=`{}` validation={}",
        caps.device_name,
        if validation_on { "on" } else { "off" }
    );

    let fixture = m92_fixture();
    let spv = spv_bytes(&skin_kernel::m92_skin_spv(
        M92_INFLUENCES,
        M92_CLUSTER_BONES,
    ));
    let mut failures: Vec<String> = Vec::new();

    // ── 步骤 1:双腿(两趟完整运行)× 全姿态:device kernel 真跑 + host 参照对拍 ──
    let mut run_digests: Vec<[u8; 32]> = Vec::new();
    let mut per_pose_cluster_digest: Vec<String> = Vec::new();
    let mut containment_aabb = true;
    let mut containment_sphere = true;
    let mut containment_cone = true;
    // AABB/包围球字段级 device/host 分歧 measured 面(run 0 采集;双跑 digest
    // 逐位一致 ⇒ run 1 同值)。
    let mut bound_max_ulp = 0u64;
    let mut bound_max_abs = 0.0f64;
    let mut device_outputs: Vec<[[Vec<u8>; 3]; 2]> = Vec::new(); // 供 driver 交叉锚与 RED 臂
    for run in 0..2u32 {
        let mut preimage: Vec<u8> = Vec::new();
        for (pi, pose) in fixture.poses.iter().enumerate() {
            let out = match run_pose_on_device(&spv, &fixture, pose) {
                Ok(o) => o,
                Err(e) => fail(&format!("姿态 {pi} device 执行(run {run}): {e}")),
            };
            if run == 0 {
                device_outputs.push(out.clone());
            }
            for (ci, c) in fixture.clusters.iter().enumerate() {
                let input = c.cluster_input();
                let host_pos = match skin_cluster(&input, pose) {
                    Ok(v) => v,
                    Err(e) => fail(&format!("姿态 {pi} 簇 {ci} host 蒙皮: {e}")),
                };
                let host_nrm = match skin_normals(&input, &c.normals, pose) {
                    Ok(v) => v,
                    Err(e) => fail(&format!("姿态 {pi} 簇 {ci} host 法向: {e}")),
                };
                let host_aabb = conservative_skinned_aabb(&input, pose).expect("host AABB");
                let host_sphere = conservative_skinned_sphere(&input, pose).expect("host 球");
                let host_cone =
                    conservative_skinned_cone(&input, pose, &c.rest_cone).expect("host 锥");
                let dev_pos = skin_kernel::decode_vec3s(&out[ci][0], c.vertices.len());
                let dev_nrm = skin_kernel::decode_vec3s(&out[ci][1], c.vertices.len());
                let dev_bound = skin_kernel::decode_bound(&out[ci][2]);
                // 逐顶点对拍(定点域容差 0 = 位级全等;RXS-0353 L1 核心句)。
                if let Err(e) = bitexact_compare(&format!("P{pi}C{ci} 位置"), &dev_pos, &host_pos)
                {
                    fail(&e);
                }
                if let Err(e) = bitexact_compare(&format!("P{pi}C{ci} 法向"), &dev_nrm, &host_nrm)
                {
                    fail(&e);
                }
                // 法向锥逐位对拍(无 sqrt 面:角度表 host 单源 + max/add/min 精确
                // 整数式操作 ⇒ 可位级判据)。
                if dev_bound.cone != host_cone {
                    fail(&format!(
                        "P{pi}C{ci} 法向锥分叉:device={:?} vs host={host_cone:?}",
                        dev_bound.cone
                    ));
                }
                // AABB/包围球:判据 = 包含不变式(L2 逐字;下方核验)。device/host
                // 字段级差异经 GLSL.std.450.Sqrt 双侧舍入(本机实测 ≤1 ulp 级)
                // 产生——**measured 登记**(进 evidence,不手写冻结容差,P-09)。
                if run == 0 {
                    for (d, h) in dev_bound
                        .aabb
                        .0
                        .iter()
                        .chain(dev_bound.aabb.1.iter())
                        .zip(host_aabb.0.iter().chain(host_aabb.1.iter()))
                    {
                        bound_max_ulp = bound_max_ulp.max(ulp_diff(*d, *h));
                        bound_max_abs = bound_max_abs.max(f64::from((d - h).abs()));
                    }
                    for (d, h) in dev_bound
                        .sphere
                        .0
                        .iter()
                        .chain(std::iter::once(&dev_bound.sphere.1))
                        .zip(host_sphere.0.iter().chain(std::iter::once(&host_sphere.1)))
                    {
                        bound_max_ulp = bound_max_ulp.max(ulp_diff(*d, *h));
                        bound_max_abs = bound_max_abs.max(f64::from((d - h).abs()));
                    }
                }
                // 包围体 100% 包含核验(device 输出自封:AABB/球 ⊇ 顶点;锥 ⊇ 法向)。
                if verify_bound_containment(&dev_bound.aabb, &dev_pos).is_err() {
                    containment_aabb = false;
                    failures.push(format!("P{pi}C{ci} AABB 包含破坏"));
                }
                if verify_sphere_containment(dev_bound.sphere.0, dev_bound.sphere.1, &dev_pos)
                    .is_err()
                {
                    containment_sphere = false;
                    failures.push(format!("P{pi}C{ci} 球包含破坏"));
                }
                if verify_normal_cone_containment(&dev_bound.cone, &dev_nrm).is_err() {
                    containment_cone = false;
                    failures.push(format!("P{pi}C{ci} 锥覆盖破坏"));
                }
                // digest 面:回读三缓冲依序混合。
                let mut d_pre = Vec::new();
                d_pre.extend_from_slice(&(pi as u32).to_le_bytes());
                d_pre.extend_from_slice(&(ci as u32).to_le_bytes());
                d_pre.extend_from_slice(&out[ci][0]);
                d_pre.extend_from_slice(&out[ci][1]);
                d_pre.extend_from_slice(&out[ci][2]);
                let d = rurix_pkg::sha256::digest(&d_pre);
                if run == 0 {
                    per_pose_cluster_digest.push(hex(&d));
                }
                preimage.extend_from_slice(&d);
            }
        }
        run_digests.push(rurix_pkg::sha256::digest(&preimage));
    }
    let double_run_bitexact = run_digests[0] == run_digests[1];
    if !double_run_bitexact {
        failures.push("档位序列双跑 digest 分叉".to_string());
    }
    println!(
        "G9_M92_SKIN: 逐顶点对拍容差 0 全过(3 姿态 × 2 簇 × 2 趟);run digests [{} {}]",
        hex(&run_digests[0]),
        hex(&run_digests[1])
    );

    // ── 步骤 2:驱动帧序列(档位切换/静态帧/降级恢复)+ AS 计数面 ──
    // BLAS:两簇 Deformable(更新帧 refit 记账)。
    let mut blas = BlasCache::new();
    let blas_c0 = blas.get_or_build(
        &fixture.clusters[0].vertices,
        &[[0u32, 1, 2], [0, 2, 3]],
        DynamicPolicy::Deformable {
            refit_budget_frames: 1,
        },
    );
    let blas_c1 = blas.get_or_build(
        &fixture.clusters[1].vertices,
        &[[0u32, 1, 2]],
        DynamicPolicy::Deformable {
            refit_budget_frames: 1,
        },
    );
    let builds0 = blas.stats().blas_builds;
    let mut driver = SkinningDriver::new(2);
    let in0 = fixture.clusters[0].cluster_input();
    let in1 = fixture.clusters[1].cluster_input();
    let pose1 = fixture.poses[1].clone();
    let pose2 = fixture.poses[2].clone();
    let mut static_frame_zero_build = true;
    let mut f0_cache: Option<[Vec<[f32; 3]>; 2]> = None;
    // (frame, pose, dist C0, dist C1):近全速 → 静态 → 远 1/4 降级/半速更新 →
    // 降级续帧 → 更新点恢复。
    let frames: [(u64, &SkinPalette, f32, f32); 5] = [
        (0, &pose1, 5.0, 5.0),
        (1, &pose1, 5.0, 5.0),
        (2, &pose2, 50.0, 15.0),
        (3, &pose2, 50.0, 15.0),
        (4, &pose2, 50.0, 15.0),
    ];
    for &(f, pose, d0, d1) in &frames {
        let (rb, bb) = (blas.stats().refits, blas.stats().blas_builds);
        driver
            .drive_frame(
                f,
                &[
                    SkinnedClusterFrame {
                        input: &in0,
                        distance_m: d0,
                        blas: blas_c0,
                    },
                    SkinnedClusterFrame {
                        input: &in1,
                        distance_m: d1,
                        blas: blas_c1,
                    },
                ],
                pose,
                0.5,
                &mut blas,
            )
            .expect("驱动帧");
        if f == 1 && (blas.stats().refits != rb || blas.stats().blas_builds != bb) {
            static_frame_zero_build = false;
        }
        if f == 0 {
            f0_cache = Some([
                driver.cache.slots[0].positions.clone(),
                driver.cache.slots[1].positions.clone(),
            ]);
        }
    }
    let as_stats = blas.stats();
    // AS 更新计数非空可机核:f0 双簇 + f2 C1(半速更新点)+ f4 C0(恢复)= 4 refit。
    let as_update_counted = as_stats.refits == 4 && as_stats.blas_builds == builds0;
    if !as_update_counted {
        failures.push(format!(
            "AS 计数面漂移:refits={} builds={}(期望 4/{builds0})",
            as_stats.refits, as_stats.blas_builds
        ));
    }
    if !static_frame_zero_build {
        failures.push("静态帧 AS 构建/refit 计数非零增量(RED)".to_string());
    }
    // 档位直方图锚:Full×4 + Half×3 + Quarter×3。
    let tier_hist_ok = driver.stats.tier_histogram == [4, 3, 0, 3];
    if !tier_hist_ok {
        failures.push(format!("档位直方图漂移:{:?}", driver.stats.tier_histogram));
    }
    // 驱动产物 ↔ device 输出交叉锚(更新帧 skin cache 内容 = device kernel 输出,
    // 位级):f0 快照 = P1 双簇;f2 后 C1 / f4 后 C0 = P2。
    let dev_p1 = &device_outputs[1];
    let dev_p2 = &device_outputs[2];
    let f0_cache = f0_cache.expect("f0 快照");
    for (ci, c) in fixture.clusters.iter().enumerate() {
        let dev = skin_kernel::decode_vec3s(&dev_p1[ci][0], c.vertices.len());
        if !bits_eq(&f0_cache[ci], &dev) {
            failures.push(format!("driver f0 簇 {ci} 产物 ≠ device P1 输出"));
        }
    }
    let c0p = skin_kernel::decode_vec3s(&dev_p2[0][0], fixture.clusters[0].vertices.len());
    let c1p = skin_kernel::decode_vec3s(&dev_p2[1][0], fixture.clusters[1].vertices.len());
    if !bits_eq(&driver.cache.slots[0].positions, &c0p) {
        failures.push("driver C0 恢复帧产物 ≠ device P2 输出".to_string());
    }
    if !bits_eq(&driver.cache.slots[1].positions, &c1p) {
        failures.push("driver C1 更新帧产物 ≠ device P2 输出".to_string());
    }

    // ── 步骤 3:RED 臂(人为缩小包围体必被检出;篡改顶点必被对拍检出)──
    // 取 P2/C0 device 输出(锥半角 = π/2 对抗位)。
    let red_pos = skin_kernel::decode_vec3s(&dev_p2[0][0], fixture.clusters[0].vertices.len());
    let red_nrm = skin_kernel::decode_vec3s(&dev_p2[0][1], fixture.clusters[0].vertices.len());
    let red_bound = skin_kernel::decode_bound(&dev_p2[0][2]);
    // AABB 缩小臂:收缩量 = grow(δ+inflation,由 device AABB 与静止盒反推)+ 0.25
    // ⇒ 缩小盒落在静止盒内部;P2 对抗姿态把顶点旋出静止盒(v2 x = −1)⇒ 必违例。
    let rest0 = fixture.clusters[0].rest_aabb;
    let grow = rest0.0[0] - red_bound.aabb.0[0];
    let s = grow + 0.25;
    let shrunk_aabb = (
        [
            red_bound.aabb.0[0] + s,
            red_bound.aabb.0[1] + s,
            red_bound.aabb.0[2] + s,
        ],
        [
            red_bound.aabb.1[0] - s,
            red_bound.aabb.1[1] - s,
            red_bound.aabb.1[2] - s,
        ],
    );
    let red_aabb = verify_bound_containment(&shrunk_aabb, &red_pos).is_err();
    let red_sphere =
        verify_sphere_containment(red_bound.sphere.0, red_bound.sphere.1 * 0.5, &red_pos).is_err();
    let red_cone = verify_normal_cone_containment(
        &NormalCone {
            axis: red_bound.cone.axis,
            half_angle: 0.0,
        },
        &red_nrm,
    )
    .is_err();
    for (name, ok) in [
        ("red_shrunk_aabb", red_aabb),
        ("red_shrunk_sphere", red_sphere),
        ("red_shrunk_cone", red_cone),
    ] {
        if !ok {
            failures.push(format!("RED 臂 {name} 失效:缩小包围体未被检出"));
        }
    }
    // 篡改回读一顶点(位翻转)⇒ 与 host 参照对拍必分叉(能红反证)。
    let mut tampered = red_pos.clone();
    tampered[0][0] = f32::from_bits(tampered[0][0].to_bits() ^ 0x8000_0000);
    let host_p2c0 = skin_cluster(&in0, &pose2).expect("host P2C0");
    let red_tamper = bitexact_compare("tampered", &tampered, &host_p2c0).is_err();
    if !red_tamper {
        failures.push("RED 臂 red_vertex_tamper 失效:位翻转未被对拍检出".to_string());
    }
    println!(
        "G9_M92_SKIN: RED 臂[aabb={red_aabb} sphere={red_sphere} cone={red_cone} tamper={red_tamper}] 全部检出"
    );

    // ── 步骤 4:浮点输入域容差 measured(登记,非判据;P-09 先 measured 后冻结)──
    let float_vertices: Vec<[f32; 3]> = vec![
        [0.1, 0.2, 0.3],
        [1.1, -0.7, 0.333],
        [0.4, 0.9, 0.125],
        [0.25, 0.11, 0.47],
    ];
    let float_normals: Vec<[f32; 3]> = vec![
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.1, 0.2, 0.970_142_5],
        [0.0, 0.0, 1.0],
    ];
    let float_weights: Vec<Vec<(u32, f32)>> = vec![
        vec![(0, 1.0 / 3.0), (1, 2.0 / 3.0)],
        vec![(0, 0.7), (1, 0.3)],
        vec![(1, 1.0), (1, 0.0)],
        vec![(0, 0.125), (1, 0.875)],
    ];
    let float_bones = [0u32, 1];
    let float_input = ClusterSkinInput {
        max_influences: 2,
        bone_indices: &float_bones,
        bound_inflation: 0.1,
        rest_aabb_min: [0.1, -0.7, 0.125],
        rest_aabb_max: [1.1, 0.9, 0.47],
        vertices: &float_vertices,
        weights: &float_weights,
    };
    let float_cone = NormalCone {
        axis: [0.0, 0.0, 1.0],
        half_angle: 0.05,
    };
    let mut float_max_abs = 0.0f64;
    let mut float_max_ulp = 0u64;
    {
        // 单簇单 pass device 真跑(同 kernel;输入域非定点)。
        let (palette_b, angle_b) = skin_kernel::pack_palette(&pose2);
        let pack = skin_kernel::pack_cluster(
            &float_vertices,
            &float_normals,
            &float_weights,
            &float_bones,
            0.1,
            ([0.1, -0.7, 0.125], [1.1, 0.9, 0.47]),
            &float_cone,
        );
        let resources = [
            storage(pack.rest_pos.len(), Some(&pack.rest_pos)),
            storage(pack.rest_nrm.len(), Some(&pack.rest_nrm)),
            storage(pack.wval.len(), Some(&pack.wval)),
            storage(pack.wbone.len(), Some(&pack.wbone)),
            storage(palette_b.len(), Some(&palette_b)),
            storage(angle_b.len(), Some(&angle_b)),
            storage(pack.cluster_bones.len(), Some(&pack.cluster_bones)),
            storage(pack.rest_pos.len(), None),
            storage(pack.rest_nrm.len(), None),
            storage(M92_BOUND_WORDS * 4, None),
        ];
        let passes = [Pass::Compute(ComputePass {
            name: "m92_skin_float",
            spirv: &spv,
            entry: Some("main"),
            dispatch: DispatchSpec::Direct([pack.n_vertices, 1, 1]),
            bindings: render_exec::Bindings {
                storage_buffers: (0..10).collect(),
                push_constants: pack.push.clone(),
                ..Default::default()
            },
        })];
        let barriers: [&[(u32, TargetState)]; 1] = [&[]];
        let readbacks = [
            Readback::Buffer {
                res: 7,
                offset: 0,
                size: (pack.n_vertices * 12) as u64,
            },
            Readback::Buffer {
                res: 8,
                offset: 0,
                size: (pack.n_vertices * 12) as u64,
            },
            Readback::Buffer {
                res: 9,
                offset: 0,
                size: (M92_BOUND_WORDS * 4) as u64,
            },
        ];
        match render_exec::execute_frame(&resources, &passes, &barriers, &readbacks) {
            Ok(out) => {
                let dev_pos = skin_kernel::decode_vec3s(&out[0], float_vertices.len());
                let host_pos = skin_cluster(&float_input, &pose2).expect("host 浮点蒙皮");
                for (d, h) in dev_pos.iter().zip(host_pos.iter()) {
                    for k in 0..3 {
                        float_max_abs = float_max_abs.max(f64::from((d[k] - h[k]).abs()));
                        float_max_ulp = float_max_ulp.max(ulp_diff(d[k], h[k]));
                    }
                }
            }
            Err(e) => fail(&format!("浮点域 device 执行: {e}")),
        }
    }
    println!(
        "G9_M92_SKIN: 浮点域 measured max_abs={float_max_abs:.9e} max_ulp={float_max_ulp}(登记,非判据)"
    );

    // ── 步骤 5:validation 计数面(render_exec messenger 实数)──
    let validation_error_total = if validation_on {
        let n = render_exec::validation_error_total();
        let installed = render_exec::validation_messenger_installed();
        if !installed {
            failures.push("validation=on 但 messenger 未安装(计数不可信)".to_string());
        }
        if n != 0 {
            failures.push(format!("validation error {n} ≠ 0"));
        }
        Some(n)
    } else {
        None
    };

    // ── 步骤 6:evidence JSON(hand-rolled;零依赖纪律)──
    // vertex_bitexact/cone_bitexact:逐顶点/法向/法向锥位级对拍在上方
    // fail-fast(到此即全过)。
    let checks: [(&str, bool); 14] = [
        ("vertex_bitexact", true),
        ("cone_bitexact", true),
        ("containment_aabb", containment_aabb),
        ("containment_sphere", containment_sphere),
        ("containment_cone", containment_cone),
        ("tier_switch_double_run_bitexact", double_run_bitexact),
        ("static_frame_zero_as_build", static_frame_zero_build),
        ("as_update_counted", as_update_counted),
        ("tier_histogram_golden", tier_hist_ok),
        ("red_shrunk_aabb", red_aabb),
        ("red_shrunk_sphere", red_sphere),
        ("red_shrunk_cone", red_cone),
        ("red_vertex_tamper", red_tamper),
        (
            "validation_error_zero",
            validation_error_total.is_none_or(|n| n == 0),
        ),
    ];
    let checks_json: Vec<String> = checks
        .iter()
        .map(|(n, ok)| format!("\"{n}\": {ok}"))
        .collect();
    let failures_json: Vec<String> = failures.iter().map(|f| format!("\"{f}\"")).collect();
    let pose_digest_json: Vec<String> = per_pose_cluster_digest
        .iter()
        .map(|d| format!("\"{d}\""))
        .collect();
    let json = format!(
        "{{\n  \"schema\": \"rurix.g9m92.skinning_device.v1\",\n  \
         \"subject\": \"g9_m92_skinning_device\",\n  \
         \"device_state\": {{\"device_name\": \"{}\", \"validation\": \"{}\", \
         \"validation_error_total\": {}, \"require_real\": {}}},\n  \
         \"checks\": {{{}}},\n  \
         \"digests\": {{\"run_a\": \"{}\", \"run_b\": \"{}\", \
         \"per_pose_cluster\": [{}]}},\n  \
         \"driver_stats\": {{\"refits\": {}, \"blas_builds\": {}, \"static_skips\": {}, \
         \"stale_skips\": {}, \"tier_histogram\": {:?}}},\n  \
         \"bound_field_divergence_measured\": {{\"max_ulp\": {}, \"max_abs\": {:.9e}, \
         \"note\": \"AABB/球字段 device/host 分歧 = GLSL.std.450.Sqrt 双侧舍入(实测 ulp 级);\
         判据 = 包含不变式(RXS-0353 L2 逐字)+ 法向锥位级;不手写冻结容差(P-09)\"}},\n  \
         \"float_domain_measured\": {{\"max_abs_diff\": {:.9e}, \"max_ulp_diff\": {}, \
         \"note\": \"measured 登记;未经 spec 冻结(P-09),非判据\"}},\n  \
         \"failures\": [{}]\n}}",
        caps.device_name,
        if validation_on { "on" } else { "off" },
        validation_error_total
            .map(|n| n.to_string())
            .unwrap_or_else(|| "null".to_string()),
        std::env::var("RURIX_REQUIRE_REAL").as_deref() == Ok("1"),
        checks_json.join(", "),
        hex(&run_digests[0]),
        hex(&run_digests[1]),
        pose_digest_json.join(", "),
        as_stats.refits,
        as_stats.blas_builds,
        driver.stats.static_skips,
        driver.stats.stale_skips,
        driver.stats.tier_histogram,
        bound_max_ulp,
        bound_max_abs,
        float_max_abs,
        float_max_ulp,
        failures_json.join(", "),
    );
    match &evidence_path {
        Some(p) => std::fs::write(p, &json).expect("写 evidence"),
        None => {}
    }
    println!("{json}");
    if failures.is_empty() {
        println!(
            "G9_M92_SKIN: PASS run_digest={} validation={}",
            hex(&run_digests[0]),
            if validation_on { "on(error=0)" } else { "off" }
        );
        std::process::exit(0);
    }
    fail(&format!("{failures:?}"));
}
