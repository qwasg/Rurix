// Assisted-by: Cursor Agent(G37 W3 深水区:FIF×动态共存判档实施窗,TODO #90)
//! G37 W3:FIF 流水 × 动态 AS 更新共存 **判档 harness**(G36 五留窗之一,
//! TODO #90;RFC 草案 = artifacts/day_0830_delivery/w3_deep/fif_dyn/
//! RFC_DRAFT_RFC0030_amendment.md,实施记录 = 同目录 REPORT.md)。
//!
//! ## 背景(侦察结论)
//!
//! 既有 FIF 流水入口(`submit_with_frame_update`,render_exec)fail-closed 拒
//! `tlas_update`/`blas_refit`:TLAS instance buffer / BLAS 顶点缓冲为**共享
//! host 写面**,submit 帧 N+1 时帧 N 在飞,host memcpy 与在飞帧 device 读竞争
//! ——动态(--dyn-demo)/蒙皮(--skin-demo)被迫 `--inflight 1` 顺序提交。
//! 真修复 = **每槽 AS 副本**(每槽实例缓冲 + BLAS 顶点副本 + 每槽 AS 描述符
//! 集):G37 W3 加性入口 `submit_with_frame_update_slot_as`(rurix-rt
//! render_exec_g37_fif_dyn.rs body-include;既有入口 0-byte)。
//!
//! ## 三臂判档
//!
//! 固定轨迹 N 帧动态场景(地面 quad 静止 + 立方体逐帧平移,2 BLAS × 2 实例,
//! 逐帧 `tlas_update`〔默认 Rebuild;`--action refit` 臂 = UPDATE 语义〕):
//!
//! - **臂 A(基线)**:单槽顺序提交(`execute_with_frame_update`;现行为——
//!   dyn/skin/HZB 车道同形,session `frame_slots=2` 但顺序语义);
//! - **臂 B**:inflight=2——session AS 表 2 份同构副本(组 [0,2)),逐帧
//!   `tlas_update` 落本槽副本 + `binding_overrides` 把 ray query pass 的 AS
//!   绑定轮换到本槽副本(per-slot descriptor override set,G31 A2 既有基建),
//!   `submit_with_frame_update_slot_as`/`collect` FIFO 真流水;
//! - **臂 C**:inflight=3 同构(组 [0,3))。
//!
//! **判据(fail-closed)**:① B/C 与 A 的逐帧 digest 序列**逐字节相等**(每槽
//! 副本消除跨槽写竞争 ⇒ 语义等价;Rebuild 下 AS 内容 = 纯函数(本帧实例))∧
//! ② 三臂各自双跑位级一致(重建会话重放)∧ ③ validation ERROR = 0 ∧
//! ④ 动态见证(逐帧立方体/地面命中皆 >0 + digest 序列非常量 + 哨兵 canary
//! 零残留)∧ ⑤ 错槽更新/跨槽绑定 device 腿 RED 必拒。帧时 A/B/C measured
//! **登记不设通过线**(FIF 收益 = CPU submit/fence 解耦,GPU 帧间守卫 barrier
//! 全序维持——RFC-0030 §4.3 L2 字面)。
//!
//! ## 用法
//!
//! ```text
//! g31_fif_dyn_probe [--selftest] [--frames N=24] [--rays WxH=64x48]
//!                   [--action rebuild|refit] [--out FILE]
//! ```
//!
//! - `--selftest`:纯 host 腿(槽纪律校验器红绿臂/槽环写面隔离模型/轨迹
//!   确定性/kernel 结构),零 GPU。
//! - 无 loader/无设备/无 ray query 扩展 → `skipped_dev_env` 三态退 0
//!   (`RURIX_REQUIRE_REAL=1` 翻硬红;g35/async 同律)。

#![forbid(unsafe_code)]

use rurix_rt::render_exec::{
    AccelStructDesc, Bindings, BufferDesc, BufferUsage, ComputePass, DeviceFrameSession,
    DeviceFrameOutput, DispatchSpec, FrameTicket, FrameUpdate, Pass, Readback, ResourceDesc,
    SlotAsGroup, TargetState, g37_validate_slot_as_frame,
};
use rurix_rt::vk::{
    self as rvk, RAY_QUERY_IDENTITY_TRANSFORM, RayQueryInstanceDesc, RayQuerySceneDesc,
    RayQueryTransformedInstanceDesc, TlasBuildAction,
};
use std::collections::VecDeque;

const TAG: &str = "[g31_fif_dyn_probe]";

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

struct Args {
    selftest: bool,
    frames: u32,
    rays_w: u32,
    rays_h: u32,
    action: TlasBuildAction,
    out: Option<String>,
}

fn parse_args() -> Result<Args, String> {
    let argv: Vec<String> = std::env::args().collect();
    let mut a = Args {
        selftest: false,
        frames: 24,
        rays_w: 64,
        rays_h: 48,
        action: TlasBuildAction::Rebuild,
        out: None,
    };
    let mut i = 1;
    let take = |argv: &[String], i: &mut usize| -> Result<String, String> {
        *i += 1;
        argv.get(*i)
            .cloned()
            .ok_or_else(|| format!("{} 缺参数值", argv[*i - 1]))
    };
    while i < argv.len() {
        match argv[i].as_str() {
            "--selftest" => a.selftest = true,
            "--frames" => {
                a.frames = take(&argv, &mut i)?
                    .parse()
                    .map_err(|_| "--frames 非 u32".to_owned())?
            }
            "--rays" => {
                let s = take(&argv, &mut i)?;
                let mut it = s.split('x');
                let w: u32 = it
                    .next()
                    .and_then(|t| t.parse().ok())
                    .ok_or("--rays 形如 64x48")?;
                let h: u32 = it
                    .next()
                    .and_then(|t| t.parse().ok())
                    .ok_or("--rays 形如 64x48")?;
                if it.next().is_some() || w == 0 || h == 0 {
                    return Err("--rays 形如 64x48(两正整数)".into());
                }
                (a.rays_w, a.rays_h) = (w, h);
            }
            "--action" => {
                a.action = match take(&argv, &mut i)?.as_str() {
                    "rebuild" => TlasBuildAction::Rebuild,
                    "refit" => TlasBuildAction::Refit,
                    other => return Err(format!("--action {other}:只接受 rebuild|refit")),
                }
            }
            "--out" => a.out = Some(take(&argv, &mut i)?),
            other => {
                return Err(format!(
                    "未知参数 {other}(--selftest/--frames/--rays/--action/--out)"
                ));
            }
        }
        i += 1;
    }
    // 判据④「动态见证」与槽环覆盖(每槽至少复用一轮)需要最少帧数:
    // frames ≥ 8 ⇒ inflight=3 下每槽 ≥2 次复用 + 轨迹可辨。
    if !a.selftest && a.frames < 8 {
        return Err("--frames 必须 ≥8(槽环至少复用一轮 + 动态见证)".into());
    }
    Ok(a)
}

// ---------------------------------------------------------------------------
// 场景(2 BLAS × 2 实例;全 f32 闭式确定性)
// ---------------------------------------------------------------------------

/// BLAS 0:地面 quad(y=0,x/z ∈ [-4,4],2 三角;9 f32/tri 三角汤)。
fn ground_tris() -> Vec<f32> {
    vec![
        -4.0, 0.0, -4.0, 4.0, 0.0, -4.0, 4.0, 0.0, 4.0, //
        -4.0, 0.0, -4.0, 4.0, 0.0, 4.0, -4.0, 0.0, 4.0,
    ]
}

/// BLAS 1:单位立方体(中心原点,半边长 0.5,12 三角;世界放置经实例矩阵)。
fn cube_tris() -> Vec<f32> {
    let h = 0.5f32;
    let p = |x: f32, y: f32, z: f32| [x * h, y * h, z * h];
    // 8 顶点。
    let v = [
        p(-1.0, -1.0, -1.0),
        p(1.0, -1.0, -1.0),
        p(1.0, 1.0, -1.0),
        p(-1.0, 1.0, -1.0),
        p(-1.0, -1.0, 1.0),
        p(1.0, -1.0, 1.0),
        p(1.0, 1.0, 1.0),
        p(-1.0, 1.0, 1.0),
    ];
    // 12 三角(6 面 × 2;固定绕序——顶点序即 primitiveIndex 序,确定性协议面)。
    let idx: [[usize; 3]; 12] = [
        [0, 2, 1],
        [0, 3, 2], // -z
        [4, 5, 6],
        [4, 6, 7], // +z
        [0, 1, 5],
        [0, 5, 4], // -y
        [3, 6, 2],
        [3, 7, 6], // +y
        [0, 7, 3],
        [0, 4, 7], // -x
        [1, 2, 6],
        [1, 6, 5], // +x
    ];
    let mut out = Vec::with_capacity(12 * 9);
    for t in idx {
        for vi in t {
            out.extend_from_slice(&v[vi]);
        }
    }
    out
}

/// 固定轨迹:帧 k 的实例集(实例 0 = 地面 identity;实例 1 = 立方体沿 +x
/// 匀速平移,y=0.75 悬空防共面 tie)。实例数恒 2(write_transforms 合法域)。
fn insts_for_frame(k: u32) -> Vec<RayQueryTransformedInstanceDesc> {
    let x = -1.2f32 + 0.1f32 * k as f32;
    vec![
        RayQueryTransformedInstanceDesc {
            blas: 0,
            custom_index: 0,
            mask: 0xFF,
            sbt_record_offset: 0,
            transform: RAY_QUERY_IDENTITY_TRANSFORM,
        },
        RayQueryTransformedInstanceDesc {
            blas: 1,
            custom_index: 1,
            mask: 0xFF,
            sbt_record_offset: 0,
            transform: [
                1.0, 0.0, 0.0, x, //
                0.0, 1.0, 0.0, 0.75, //
                0.0, 0.0, 1.0, 0.0,
            ],
        },
    ]
}

/// 针孔光线网格(eye (0,1,3.5) 望 -z,fov_y 60°;8 f32/线 = origin.xyz,
/// dir.xyz, tmin, tmax;全 f32 闭式——双跑逐位同)。
fn gen_rays(w: u32, h: u32) -> Vec<f32> {
    let eye = [0.0f32, 1.0, 3.5];
    let tan_half = 0.577_350_3f32; // tan(30°)
    let aspect = w as f32 / h as f32;
    let mut out = Vec::with_capacity((w * h) as usize * 8);
    for j in 0..h {
        for i in 0..w {
            let u = (i as f32 + 0.5) / w as f32;
            let v = (j as f32 + 0.5) / h as f32;
            let px = (2.0 * u - 1.0) * tan_half * aspect;
            let py = (1.0 - 2.0 * v) * tan_half;
            let len = (px * px + py * py + 1.0).sqrt();
            out.extend_from_slice(&[
                eye[0],
                eye[1],
                eye[2],
                px / len,
                py / len,
                -1.0 / len,
                1.0e-3,
                1.0e9,
            ]);
        }
    }
    out
}

fn bytes_f32(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

// ---------------------------------------------------------------------------
// 手编 SPIR-V(bin-local;g31_frame_cut_arm `frame_cut_*_spv` 逐字改置——
// vk_clas_rt m94 形制;冻结 kernels/*.rx 与 SPV 全 0-byte,无新 rurixc 编译面)
// ---------------------------------------------------------------------------

fn spv_inst(v: &mut Vec<u32>, op: u32, ops: &[u32]) {
    v.push(op | ((ops.len() as u32 + 1) << 16));
    v.extend_from_slice(ops);
}

fn spv_words(s: &str) -> Vec<u32> {
    let mut b = s.as_bytes().to_vec();
    b.push(0);
    while b.len() % 4 != 0 {
        b.push(0);
    }
    b.chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn spv_bytes(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|w| w.to_le_bytes()).collect()
}

/// 哨兵清写 kernel(set0 b0 = out u32 SSBO;每 invocation 清一条 4 u32 记录为
/// 0xFFFF_FFFF——RQ pass 随后必须整写覆盖,残留哨兵 = dispatch 覆盖缺陷 canary)。
fn clear_spv() -> Vec<u32> {
    let mut v = vec![0x0723_0203u32, 0x0001_0400, 0, 64, 0];
    spv_inst(&mut v, 17, &[1]); // OpCapability Shader
    spv_inst(&mut v, 14, &[0, 1]); // OpMemoryModel Logical GLSL450
    let mut ep = vec![5u32, 1];
    ep.extend(spv_words("main"));
    ep.extend_from_slice(&[10, 21]);
    spv_inst(&mut v, 15, &ep); // OpEntryPoint GLCompute %1 "main" %gid %out
    spv_inst(&mut v, 16, &[1, 17, 1, 1, 1]); // LocalSize 1 1 1
    spv_inst(&mut v, 71, &[10, 11, 28]); // %10 BuiltIn GlobalInvocationId
    spv_inst(&mut v, 71, &[21, 34, 0]); // %21 DescriptorSet 0
    spv_inst(&mut v, 71, &[21, 33, 0]); // %21 Binding 0
    spv_inst(&mut v, 71, &[19, 2]); // %19 Block
    spv_inst(&mut v, 72, &[19, 0, 35, 0]); // member0 Offset 0
    spv_inst(&mut v, 71, &[18, 6, 4]); // %18 ArrayStride 4
    spv_inst(&mut v, 19, &[2]); // %2 void
    spv_inst(&mut v, 33, &[3, 2]); // %3 fn
    spv_inst(&mut v, 21, &[4, 32, 0]); // %4 u32
    spv_inst(&mut v, 23, &[8, 4, 3]); // %8 uvec3
    spv_inst(&mut v, 32, &[9, 1, 8]); // %9 ptr Input uvec3
    spv_inst(&mut v, 59, &[9, 10, 1]); // %10 gid
    spv_inst(&mut v, 29, &[18, 4]); // %18 rtarray u32
    spv_inst(&mut v, 30, &[19, 18]); // %19 struct
    spv_inst(&mut v, 32, &[20, 12, 19]); // %20 ptr SB struct
    spv_inst(&mut v, 59, &[20, 21, 12]); // %21 out
    spv_inst(&mut v, 32, &[23, 12, 4]); // %23 ptr SB u32
    spv_inst(&mut v, 43, &[4, 26, 0]); // %26 = 0
    spv_inst(&mut v, 43, &[4, 27, 1]); // %27 = 1
    spv_inst(&mut v, 43, &[4, 28, 4]); // %28 = 4
    spv_inst(&mut v, 43, &[4, 30, 2]); // %30 = 2
    spv_inst(&mut v, 43, &[4, 31, 3]); // %31 = 3
    spv_inst(&mut v, 43, &[4, 32, 0xFFFF_FFFF]); // %32 = 哨兵
    spv_inst(&mut v, 54, &[2, 1, 0, 3]); // %1 = OpFunction
    spv_inst(&mut v, 248, &[40]); // %40 label
    spv_inst(&mut v, 61, &[8, 42, 10]); // %42 = load gid
    spv_inst(&mut v, 81, &[4, 43, 42, 0]); // %43 = gid.x
    spv_inst(&mut v, 132, &[4, 44, 43, 28]); // %44 = i*4
    let offs = [26u32, 27, 30, 31];
    let mut next_id = 45u32;
    for (j, off) in offs.iter().enumerate() {
        let idx = if j == 0 {
            44
        } else {
            let id = next_id;
            next_id += 1;
            spv_inst(&mut v, 128, &[4, id, 44, *off]);
            id
        };
        let addr = next_id;
        next_id += 1;
        spv_inst(&mut v, 65, &[23, addr, 21, 26, idx]);
        spv_inst(&mut v, 62, &[addr, 32]); // store 哨兵
    }
    spv_inst(&mut v, 253, &[]); // OpReturn
    spv_inst(&mut v, 56, &[]); // OpFunctionEnd
    v
}

/// ray query kernel(set0:b0 = TLAS / b1 = 光线 8 f32 SSBO / b2 = 输出
/// 4 u32/线 [committed, t_bits, instance_id, primitive];LocalSize 1×1×1,
/// groups.x = 光线数——instance_id = TLAS 槽位序,动态实例的 digest 见证位)。
fn rq_spv() -> Vec<u32> {
    let mut v = vec![0x0723_0203u32, 0x0001_0400, 0, 128, 0];
    spv_inst(&mut v, 17, &[1]); // OpCapability Shader
    spv_inst(&mut v, 17, &[4472]); // OpCapability RayQueryKHR
    let mut ext = vec![];
    ext.extend(spv_words("SPV_KHR_ray_query"));
    spv_inst(&mut v, 10, &ext);
    spv_inst(&mut v, 14, &[0, 1]); // OpMemoryModel Logical GLSL450
    let mut ep = vec![5u32, 1];
    ep.extend(spv_words("main"));
    ep.extend_from_slice(&[10, 13, 17, 21]);
    spv_inst(&mut v, 15, &ep);
    spv_inst(&mut v, 16, &[1, 17, 1, 1, 1]); // LocalSize 1 1 1
    spv_inst(&mut v, 71, &[10, 11, 28]); // gid BuiltIn
    spv_inst(&mut v, 71, &[13, 34, 0]); // TLAS set0
    spv_inst(&mut v, 71, &[13, 33, 0]); // TLAS b0
    spv_inst(&mut v, 71, &[17, 34, 0]); // rays set0
    spv_inst(&mut v, 71, &[17, 33, 1]); // rays b1
    spv_inst(&mut v, 71, &[21, 34, 0]); // out set0
    spv_inst(&mut v, 71, &[21, 33, 2]); // out b2
    spv_inst(&mut v, 71, &[15, 2]); // rays Block
    spv_inst(&mut v, 72, &[15, 0, 35, 0]);
    spv_inst(&mut v, 71, &[19, 2]); // out Block
    spv_inst(&mut v, 72, &[19, 0, 35, 0]);
    spv_inst(&mut v, 71, &[14, 6, 4]); // stride 4
    spv_inst(&mut v, 71, &[18, 6, 4]);
    spv_inst(&mut v, 19, &[2]); // void
    spv_inst(&mut v, 33, &[3, 2]); // fn
    spv_inst(&mut v, 21, &[4, 32, 0]); // u32
    spv_inst(&mut v, 22, &[5, 32]); // f32
    spv_inst(&mut v, 20, &[6]); // bool
    spv_inst(&mut v, 23, &[7, 5, 3]); // vec3f
    spv_inst(&mut v, 23, &[8, 4, 3]); // uvec3
    spv_inst(&mut v, 32, &[9, 1, 8]); // ptr Input uvec3
    spv_inst(&mut v, 59, &[9, 10, 1]); // gid
    spv_inst(&mut v, 5341, &[11]); // OpTypeAccelerationStructureKHR
    spv_inst(&mut v, 32, &[12, 0, 11]); // ptr UC
    spv_inst(&mut v, 59, &[12, 13, 0]); // TLAS var
    spv_inst(&mut v, 29, &[14, 5]); // rtarray f32
    spv_inst(&mut v, 30, &[15, 14]);
    spv_inst(&mut v, 32, &[16, 12, 15]);
    spv_inst(&mut v, 59, &[16, 17, 12]); // rays var
    spv_inst(&mut v, 29, &[18, 4]); // rtarray u32
    spv_inst(&mut v, 30, &[19, 18]);
    spv_inst(&mut v, 32, &[20, 12, 19]);
    spv_inst(&mut v, 59, &[20, 21, 12]); // out var
    spv_inst(&mut v, 32, &[22, 12, 5]); // ptr SB f32
    spv_inst(&mut v, 32, &[23, 12, 4]); // ptr SB u32
    spv_inst(&mut v, 4472, &[24]); // OpTypeRayQueryKHR
    spv_inst(&mut v, 32, &[25, 7, 24]); // ptr Function rq
    spv_inst(&mut v, 43, &[4, 26, 0]);
    spv_inst(&mut v, 43, &[4, 27, 1]); // flags Opaque / committed / 常量 1
    spv_inst(&mut v, 43, &[4, 28, 4]);
    spv_inst(&mut v, 43, &[4, 29, 8]);
    spv_inst(&mut v, 43, &[4, 30, 0xFF]); // cull mask
    spv_inst(&mut v, 43, &[4, 32, 2]);
    spv_inst(&mut v, 43, &[4, 33, 3]);
    spv_inst(&mut v, 43, &[4, 34, 5]);
    spv_inst(&mut v, 43, &[4, 35, 6]);
    spv_inst(&mut v, 43, &[4, 36, 7]);
    spv_inst(&mut v, 54, &[2, 1, 0, 3]); // OpFunction
    spv_inst(&mut v, 248, &[40]);
    spv_inst(&mut v, 59, &[25, 41, 7]); // rq var
    spv_inst(&mut v, 61, &[8, 42, 10]); // load gid
    spv_inst(&mut v, 81, &[4, 43, 42, 0]); // i = gid.x
    spv_inst(&mut v, 132, &[4, 44, 43, 29]); // base = i*8
    let offs = [26u32, 27, 32, 33, 28, 34, 35, 36];
    let mut next_id = 45u32;
    let mut val_ids = [0u32; 8];
    for (k, slot) in val_ids.iter_mut().enumerate() {
        let idx_id = if k == 0 {
            44
        } else {
            let id = next_id;
            next_id += 1;
            spv_inst(&mut v, 128, &[4, id, 44, offs[k]]);
            id
        };
        let addr_id = next_id;
        next_id += 1;
        spv_inst(&mut v, 65, &[22, addr_id, 17, 26, idx_id]);
        let val_id = next_id;
        next_id += 1;
        spv_inst(&mut v, 61, &[5, val_id, addr_id]);
        *slot = val_id;
    }
    let origin = next_id;
    next_id += 1;
    spv_inst(&mut v, 80, &[7, origin, val_ids[0], val_ids[1], val_ids[2]]);
    let dir = next_id;
    next_id += 1;
    spv_inst(&mut v, 80, &[7, dir, val_ids[3], val_ids[4], val_ids[5]]);
    let as_id = next_id;
    next_id += 1;
    spv_inst(&mut v, 61, &[11, as_id, 13]);
    spv_inst(
        &mut v,
        4473,
        &[41, as_id, 27, 30, origin, val_ids[6], dir, val_ids[7]],
    );
    let loop_lbl = next_id;
    next_id += 1;
    let cont_lbl = next_id;
    next_id += 1;
    let after_lbl = next_id;
    next_id += 1;
    spv_inst(&mut v, 249, &[loop_lbl]);
    spv_inst(&mut v, 248, &[loop_lbl]);
    let cond = next_id;
    next_id += 1;
    spv_inst(&mut v, 4477, &[6, cond, 41]); // OpRayQueryProceedKHR
    spv_inst(&mut v, 246, &[after_lbl, cont_lbl, 0]);
    spv_inst(&mut v, 250, &[cond, cont_lbl, after_lbl]);
    spv_inst(&mut v, 248, &[cont_lbl]);
    spv_inst(&mut v, 249, &[loop_lbl]);
    spv_inst(&mut v, 248, &[after_lbl]);
    let ty = next_id;
    next_id += 1;
    spv_inst(&mut v, 4479, &[4, ty, 41, 27]); // GetIntersectionType Committed
    let has = next_id;
    next_id += 1;
    spv_inst(&mut v, 171, &[6, has, ty, 26]);
    let hit_lbl = next_id;
    next_id += 1;
    let miss_lbl = next_id;
    next_id += 1;
    let merge_lbl = next_id;
    next_id += 1;
    spv_inst(&mut v, 247, &[merge_lbl, 0]);
    spv_inst(&mut v, 250, &[has, hit_lbl, miss_lbl]);
    spv_inst(&mut v, 248, &[hit_lbl]);
    let t_id = next_id;
    next_id += 1;
    spv_inst(&mut v, 6018, &[5, t_id, 41, 27]); // committed T
    let inst_id = next_id;
    next_id += 1;
    spv_inst(&mut v, 6020, &[4, inst_id, 41, 27]); // committed InstanceId
    let prim_id = next_id;
    next_id += 1;
    spv_inst(&mut v, 6023, &[4, prim_id, 41, 27]); // committed PrimitiveIndex
    let tbits = next_id;
    next_id += 1;
    spv_inst(&mut v, 124, &[4, tbits, t_id]);
    let o0 = next_id;
    next_id += 1;
    spv_inst(&mut v, 132, &[4, o0, 43, 28]); // o0 = i*4
    let store_vals = [27, tbits, inst_id, prim_id];
    for (j, val) in store_vals.iter().enumerate() {
        let idx = if j == 0 {
            o0
        } else {
            let id = next_id;
            next_id += 1;
            spv_inst(&mut v, 128, &[4, id, o0, offs[j]]);
            id
        };
        let addr = next_id;
        next_id += 1;
        spv_inst(&mut v, 65, &[23, addr, 21, 26, idx]);
        spv_inst(&mut v, 62, &[addr, *val]);
    }
    spv_inst(&mut v, 249, &[merge_lbl]);
    spv_inst(&mut v, 248, &[miss_lbl]);
    let m0 = next_id;
    next_id += 1;
    spv_inst(&mut v, 132, &[4, m0, 43, 28]);
    for j in 0..4u32 {
        let idx = if j == 0 {
            m0
        } else {
            let id = next_id;
            next_id += 1;
            spv_inst(&mut v, 128, &[4, id, m0, offs[j as usize]]);
            id
        };
        let addr = next_id;
        next_id += 1;
        spv_inst(&mut v, 65, &[23, addr, 21, 26, idx]);
        spv_inst(&mut v, 62, &[addr, 26]); // miss 全 0
    }
    spv_inst(&mut v, 249, &[merge_lbl]);
    spv_inst(&mut v, 248, &[merge_lbl]);
    spv_inst(&mut v, 253, &[]);
    spv_inst(&mut v, 56, &[]);
    v
}

// ---------------------------------------------------------------------------
// 臂执行(A = 顺序;B/C = slot-AS FIF)
// ---------------------------------------------------------------------------

/// 单帧回读的判据核验(哨兵 canary/命中见证)+ 命中计数;返回
/// (ground_hits, cube_hits)。
fn audit_frame(arm: &str, k: u32, rb: &[u8], n_rays: usize) -> Result<(u64, u64), String> {
    let w = |i: usize, j: usize| -> u32 {
        let o = (i * 4 + j) * 4;
        u32::from_le_bytes([rb[o], rb[o + 1], rb[o + 2], rb[o + 3]])
    };
    let (mut ground, mut cube) = (0u64, 0u64);
    for i in 0..n_rays {
        let committed = w(i, 0);
        if committed == 0xFFFF_FFFF {
            return Err(format!(
                "{arm} 帧 {k} 光线 {i} 残留哨兵(RQ dispatch 覆盖缺陷)"
            ));
        }
        if committed == 0 {
            continue;
        }
        match w(i, 2) {
            0 => ground += 1,
            1 => cube += 1,
            other => {
                return Err(format!(
                    "{arm} 帧 {k} 光线 {i} 命中实例 {other} 越域(实例集恒 2)"
                ));
            }
        }
    }
    if ground == 0 || cube == 0 {
        return Err(format!(
            "{arm} 帧 {k} 动态见证破缺(ground={ground} cube={cube};须双 >0——空接线防伪)"
        ));
    }
    Ok((ground, cube))
}

/// 单臂单跑结果。
struct ArmRun {
    digests: Vec<String>,
    wall_ms: f64,
    /// 逐帧 telemetry 聚合(中位;ms)。
    gpu_clear_ms: f64,
    gpu_rq_ms: f64,
    cpu_record_ms: f64,
    cpu_submit_ms: f64,
    cpu_fence_ms: f64,
    validation_errors: u64,
}

fn median(v: &mut Vec<f64>) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).expect("时序值非 NaN"));
    v[v.len() / 2]
}

struct TelemetryAcc {
    gpu_clear: Vec<f64>,
    gpu_rq: Vec<f64>,
    cpu_record: Vec<f64>,
    cpu_submit: Vec<f64>,
    cpu_fence: Vec<f64>,
    validation: u64,
}

impl TelemetryAcc {
    fn new() -> Self {
        TelemetryAcc {
            gpu_clear: Vec::new(),
            gpu_rq: Vec::new(),
            cpu_record: Vec::new(),
            cpu_submit: Vec::new(),
            cpu_fence: Vec::new(),
            validation: 0,
        }
    }
    fn push(&mut self, out: &DeviceFrameOutput) {
        let gpu = |pi: usize| -> f64 {
            out.telemetry
                .passes
                .get(pi)
                .map_or(0.0, |p| p.gpu_ns / 1e6)
        };
        self.gpu_clear.push(gpu(0));
        self.gpu_rq.push(gpu(1));
        self.cpu_record.push(out.telemetry.cpu_record_ns as f64 / 1e6);
        self.cpu_submit.push(out.telemetry.cpu_submit_ns as f64 / 1e6);
        self.cpu_fence
            .push(out.telemetry.cpu_fence_wait_ns as f64 / 1e6);
        self.validation = self.validation.max(out.telemetry.validation_error_count);
    }
    fn finish(mut self, digests: Vec<String>, wall_ms: f64) -> ArmRun {
        ArmRun {
            digests,
            wall_ms,
            gpu_clear_ms: median(&mut self.gpu_clear),
            gpu_rq_ms: median(&mut self.gpu_rq),
            cpu_record_ms: median(&mut self.cpu_record),
            cpu_submit_ms: median(&mut self.cpu_submit),
            cpu_fence_ms: median(&mut self.cpu_fence),
            validation_errors: self.validation,
        }
    }
}

/// 会话资源/pass/readback 装配(三臂共用;`slots` = AS 副本数,1 = 臂 A 单表项)。
struct SessionShape {
    ray_bytes: Vec<u8>,
    clear: Vec<u8>,
    rq: Vec<u8>,
    ground: Vec<f32>,
    cube: Vec<f32>,
    n_rays: usize,
}

impl SessionShape {
    fn new(w: u32, h: u32) -> Self {
        SessionShape {
            ray_bytes: bytes_f32(&gen_rays(w, h)),
            clear: spv_bytes(&clear_spv()),
            rq: spv_bytes(&rq_spv()),
            ground: ground_tris(),
            cube: cube_tris(),
            n_rays: (w * h) as usize,
        }
    }
}

/// dev-env degrade 判别(g35/async 三态纪律)。
fn is_dev_env_degrade(e: &str) -> bool {
    e.contains("不可用")
        || e.contains("物理设备")
        || e.contains("扩展")
        || e.contains("feature")
}

fn skip_dev_env(reason: &str) -> ! {
    if std::env::var("RURIX_REQUIRE_REAL").as_deref() == Ok("1") {
        eprintln!("{TAG}: FAIL RURIX_REQUIRE_REAL=1 但 device 面降级: {reason}");
        std::process::exit(1);
    }
    println!(
        "{{\n  \"probe\": \"g31_fif_dyn_probe\",\n  \"mode\": \"skipped_dev_env\",\n  \"reason\": \"{}\"\n}}",
        json_escape(reason)
    );
    std::process::exit(0);
}

/// 跑一臂(slots=1 ⇒ 臂 A 顺序入口;slots=2|3 ⇒ slot-AS FIF 真流水)。
/// `red_arm` = 首帧前注入错槽/跨槽 RED 双臂(仅 device 腿首跑消费一次)。
#[allow(clippy::too_many_lines)]
fn run_arm(
    shape: &SessionShape,
    slots: usize,
    frames: u32,
    action: TlasBuildAction,
    red_arm: bool,
) -> Result<ArmRun, String> {
    let n_rays = shape.n_rays;
    let resources = [
        // 0: 光线 SSBO(创建期一次上传,静态——动态性全部经 TLAS)。
        ResourceDesc::Buffer(BufferDesc {
            size: (n_rays * 32) as u64,
            usage: BufferUsage {
                storage: true,
                ..BufferUsage::default()
            },
            data: Some(&shape.ray_bytes),
            device_local: false,
        }),
        // 1: 命中输出 SSBO。
        ResourceDesc::Buffer(BufferDesc {
            size: (n_rays * 16) as u64,
            usage: BufferUsage {
                storage: true,
                ..BufferUsage::default()
            },
            data: None,
            device_local: false,
        }),
    ];
    let passes = [
        Pass::Compute(ComputePass {
            name: "fd_clear",
            spirv: &shape.clear,
            entry: Some("main"),
            dispatch: DispatchSpec::Direct([n_rays as u32, 1, 1]),
            bindings: Bindings {
                storage_buffers: vec![1],
                ..Bindings::default()
            },
        }),
        Pass::Compute(ComputePass {
            name: "fd_rq",
            spirv: &shape.rq,
            entry: Some("main"),
            dispatch: DispatchSpec::Direct([n_rays as u32, 1, 1]),
            bindings: Bindings {
                accel_structs: vec![0],
                storage_buffers: vec![0, 1],
                ..Bindings::default()
            },
        }),
    ];
    let plan0 = [(1u32, TargetState::StorageWrite)];
    let plan1 = [
        (0u32, TargetState::StorageReadWrite),
        (1u32, TargetState::StorageReadWrite),
    ];
    let barriers: [&[(u32, TargetState)]; 2] = [&plan0, &plan1];
    let readbacks = [Readback::Buffer {
        res: 1,
        offset: 0,
        size: (n_rays * 16) as u64,
    }];
    let instances = [
        RayQueryInstanceDesc {
            blas: 0,
            custom_index: 0,
            mask: 0xFF,
            sbt_record_offset: 0,
        },
        RayQueryInstanceDesc {
            blas: 1,
            custom_index: 1,
            mask: 0xFF,
            sbt_record_offset: 0,
        },
    ];
    let tris_ref: [&[f32]; 2] = [&shape.ground, &shape.cube];
    // AS 表:臂 A 1 份;臂 B/C `slots` 份同构副本(每份独立 instance buffer/
    // BLAS 顶点缓冲/BLAS/TLAS/scratch——vk.rs VkAsManager 每表项单所有者)。
    let entry_count = slots.max(1);
    let accel: Vec<AccelStructDesc<'_>> = (0..entry_count)
        .map(|_| AccelStructDesc {
            scene: RayQuerySceneDesc {
                blas_triangles: &tris_ref,
                instances: &instances,
            },
            transforms: None,
            updatable_blas: &[],
        })
        .collect();
    // 臂 A:frame_slots=2(API 下限)+ 顺序入口 = 现行 dyn/skin/HZB 车道形。
    let frame_slots = if slots <= 1 { 2 } else { slots };
    let mut session = DeviceFrameSession::new_with_accel_structs(
        &resources,
        &passes,
        &barriers,
        &readbacks,
        frame_slots,
        &accel,
    )
    .map_err(|e| format!("会话创建: {e}"))?;

    let mk_update = |k: u32, as_index: u32, with_override: bool| -> FrameUpdate {
        FrameUpdate {
            tlas_update: Some((as_index, insts_for_frame(k), action)),
            buffer_uploads: vec![],
            binding_overrides: if with_override {
                vec![(
                    1,
                    Bindings {
                        accel_structs: vec![as_index],
                        storage_buffers: vec![0, 1],
                        ..Bindings::default()
                    },
                )]
            } else {
                vec![]
            },
            push_constant_overrides: vec![],
            readback_subset: Some(vec![0]),
            blas_refit: None,
        }
    };

    let mut acc = TelemetryAcc::new();
    let mut digests: Vec<String> = Vec::with_capacity(frames as usize);
    let wall_start = std::time::Instant::now();

    if slots <= 1 {
        // ── 臂 A:单槽顺序提交(现行为基线)──
        for k in 0..frames {
            let update = mk_update(k, 0, false);
            let prov = session
                .next_provenance_with_update(&update)
                .map_err(|e| format!("A 帧 {k} provenance: {e}"))?;
            let out = session
                .execute_with_frame_update(&prov, &update)
                .map_err(|e| format!("A 帧 {k} 提交: {e}"))?;
            audit_frame("A", k, &out.readbacks[0], n_rays)?;
            digests.push(rurix_pkg::sha256::hex_digest(&out.readbacks[0]));
            acc.push(&out);
        }
    } else {
        // ── 臂 B/C:slot-AS FIF 真流水(submit/collect FIFO)──
        let group = SlotAsGroup {
            base: 0,
            len: slots as u32,
        };
        if red_arm {
            // device 腿 RED 双臂(fail-closed 必拒;校验全在提交前,session
            // 状态零污染):① 错槽 tlas ② 跨槽绑定。
            let slot = session.next_frame_slot() as u32;
            let wrong = (slot + 1) % slots as u32;
            let bad1 = mk_update(0, wrong, true);
            let p1 = session
                .next_provenance_with_update(&bad1)
                .map_err(|e| format!("RED① provenance: {e}"))?;
            match session.submit_with_frame_update_slot_as(&p1, &bad1, &group) {
                Err(e) if e.contains("非本槽副本") => {}
                Err(e) => return Err(format!("RED①(错槽 tlas)拒因漂移: {e}")),
                Ok(_) => return Err("RED①(错槽 tlas)未拒——槽纪律破缺".into()),
            }
            let mut bad2 = mk_update(0, slot, true);
            bad2.binding_overrides[0].1.accel_structs = vec![wrong];
            let p2 = session
                .next_provenance_with_update(&bad2)
                .map_err(|e| format!("RED② provenance: {e}"))?;
            match session.submit_with_frame_update_slot_as(&p2, &bad2, &group) {
                Err(e) if e.contains("跨槽绑定") => {}
                Err(e) => return Err(format!("RED②(跨槽绑定)拒因漂移: {e}")),
                Ok(_) => return Err("RED②(跨槽绑定)未拒——绑定纪律破缺".into()),
            }
        }
        let mut pending: VecDeque<(u32, FrameTicket)> = VecDeque::new();
        let collect_one =
            |session: &mut DeviceFrameSession<'_>,
             pending: &mut VecDeque<(u32, FrameTicket)>,
             digests: &mut Vec<String>,
             acc: &mut TelemetryAcc|
             -> Result<(), String> {
                let (fk, ticket) = pending.pop_front().expect("collect 配平(结构保证)");
                let out = session
                    .collect(ticket)
                    .map_err(|e| format!("FIF 帧 {fk} collect: {e}"))?;
                audit_frame(&format!("FIF{slots}"), fk, &out.readbacks[0], n_rays)?;
                debug_assert_eq!(digests.len() as u32, fk, "FIFO 序 = 帧序");
                digests.push(rurix_pkg::sha256::hex_digest(&out.readbacks[0]));
                acc.push(&out);
                Ok(())
            };
        for k in 0..frames {
            let slot = session.next_frame_slot();
            if slot != (k as usize) % slots {
                return Err(format!(
                    "FIF{slots} 帧 {k} slot {slot} ≠ k%S {}(轮转纪律破缺)",
                    (k as usize) % slots
                ));
            }
            let update = mk_update(k, slot as u32, true);
            let prov = session
                .next_provenance_with_update(&update)
                .map_err(|e| format!("FIF{slots} 帧 {k} provenance: {e}"))?;
            let ticket = session
                .submit_with_frame_update_slot_as(&prov, &update, &group)
                .map_err(|e| format!("FIF{slots} 帧 {k} submit: {e}"))?;
            pending.push_back((k, ticket));
            if pending.len() == slots {
                collect_one(&mut session, &mut pending, &mut digests, &mut acc)?;
            }
        }
        while !pending.is_empty() {
            collect_one(&mut session, &mut pending, &mut digests, &mut acc)?;
        }
    }
    let wall_ms = wall_start.elapsed().as_secs_f64() * 1e3;
    if digests.len() != frames as usize {
        return Err(format!(
            "帧配平破缺:digest {} 帧 ≠ frames {frames}",
            digests.len()
        ));
    }
    Ok(acc.finish(digests, wall_ms))
}

// ---------------------------------------------------------------------------
// selftest(纯 host;rt 单测 g37_fif_dyn_tests 同判据双承载)
// ---------------------------------------------------------------------------

fn selftest_slot_validator() -> Result<(), String> {
    let bind = |idx: &[u32]| Bindings {
        accel_structs: idx.to_vec(),
        storage_buffers: vec![0, 1],
        ..Bindings::default()
    };
    let g = SlotAsGroup { base: 0, len: 2 };
    // 绿:本槽更新 + 本槽绑定(slot 0/1)。
    for slot in 0..2usize {
        let expect = slot as u32;
        let got = g37_validate_slot_as_frame(2, 2, slot, &g, Some(expect), None, &[bind(&[expect])])
            .map_err(|e| format!("绿臂 slot {slot} 误拒: {e}"))?;
        if got != expect {
            return Err(format!("绿臂 slot {slot} 返回 {got} ≠ {expect}"));
        }
    }
    // 红:错槽更新。
    let e = g37_validate_slot_as_frame(2, 2, 0, &g, Some(1), None, &[bind(&[0])])
        .err()
        .ok_or("错槽更新未拒")?;
    if !e.contains("非本槽副本") {
        return Err(format!("错槽拒因漂移: {e}"));
    }
    // 红:跨槽绑定。
    let e = g37_validate_slot_as_frame(2, 2, 0, &g, Some(0), None, &[bind(&[1])])
        .err()
        .ok_or("跨槽绑定未拒")?;
    if !e.contains("跨槽绑定") {
        return Err(format!("跨槽绑定拒因漂移: {e}"));
    }
    // 红:组长 ≠ frame_slots / 组越界 / 组外更新。
    if g37_validate_slot_as_frame(3, 3, 0, &g, Some(0), None, &[]).is_ok() {
        return Err("组长≠槽数未拒".into());
    }
    if g37_validate_slot_as_frame(2, 1, 0, &g, Some(0), None, &[]).is_ok() {
        return Err("组越 AS 表界未拒".into());
    }
    let g2 = SlotAsGroup { base: 1, len: 2 };
    let e = g37_validate_slot_as_frame(2, 3, 0, &g2, Some(0), None, &[bind(&[1])])
        .err()
        .ok_or("组外更新未拒")?;
    if !e.contains("槽组") {
        return Err(format!("组外拒因漂移: {e}"));
    }
    Ok(())
}

/// 槽环写面隔离模型:FIFO 深度 S 的 submit/collect 交错下,帧 k(host 写槽
/// k%S)提交前同槽前帧 k−S 必已 collect(fence 语义)——写窗与在飞读窗不重叠。
fn selftest_slot_ring() -> Result<(), String> {
    for s in [2usize, 3] {
        let frames = 12usize;
        let mut collected_at = vec![usize::MAX; frames];
        let mut submit_at = vec![0usize; frames];
        let mut pending: VecDeque<usize> = VecDeque::new();
        let mut t = 0usize;
        for k in 0..frames {
            if pending.len() == s {
                let oldest = pending.pop_front().expect("配平");
                collected_at[oldest] = t;
                t += 1;
            }
            submit_at[k] = t;
            t += 1;
            pending.push_back(k);
        }
        while let Some(oldest) = pending.pop_front() {
            collected_at[oldest] = t;
            t += 1;
        }
        for k in s..frames {
            if collected_at[k - s] >= submit_at[k] {
                return Err(format!(
                    "S={s}: 帧 {k} 写槽 {} 时同槽前帧 {} 在飞(写读窗重叠)",
                    k % s,
                    k - s
                ));
            }
        }
    }
    Ok(())
}

fn selftest_trajectory() -> Result<(), String> {
    // 双跑逐位 + 相邻帧可辨 + 实例数恒定。
    for k in 0..32u32 {
        let a = insts_for_frame(k);
        let b = insts_for_frame(k);
        if a.len() != 2 || b.len() != 2 {
            return Err("实例数漂移(恒 2)".into());
        }
        for (x, y) in a.iter().zip(b.iter()) {
            if x.transform.map(f32::to_bits) != y.transform.map(f32::to_bits) {
                return Err(format!("帧 {k} 轨迹双跑位级破缺"));
            }
        }
        if k > 0 {
            let p = insts_for_frame(k - 1);
            if a[1].transform.map(f32::to_bits) == p[1].transform.map(f32::to_bits) {
                return Err(format!("帧 {k} 与 {} 动态实例不可辨(轨迹退化)", k - 1));
            }
            if a[0].transform.map(f32::to_bits) != p[0].transform.map(f32::to_bits) {
                return Err("静态实例漂移(应恒 identity)".into());
            }
        }
    }
    let r1 = gen_rays(64, 48);
    let r2 = gen_rays(64, 48);
    if r1.len() != 64 * 48 * 8 {
        return Err("光线流长度漂移".into());
    }
    if r1.iter().map(|x| x.to_bits()).ne(r2.iter().map(|x| x.to_bits())) {
        return Err("光线流双跑位级破缺".into());
    }
    Ok(())
}

fn selftest_kernels() -> Result<(), String> {
    for (name, words, need_rq) in [
        ("fd_clear", clear_spv(), false),
        ("fd_rq", rq_spv(), true),
    ] {
        if words[0] != 0x0723_0203 {
            return Err(format!("{name} magic 破缺"));
        }
        let has_rq_cap = words.windows(2).any(|w| w[0] == (17 | (2 << 16)) && w[1] == 4472);
        if has_rq_cap != need_rq {
            return Err(format!("{name} RayQueryKHR capability 面漂移"));
        }
        let main_words = spv_words("main");
        let found = words
            .windows(main_words.len())
            .any(|w| w == main_words.as_slice());
        if !found {
            return Err(format!("{name} 入口名 main 缺失"));
        }
    }
    // 几何面:立方体 12 tri × 9 f32,地面 2 tri × 9 f32。
    if cube_tris().len() != 108 || ground_tris().len() != 18 {
        return Err("几何流长度漂移".into());
    }
    Ok(())
}

fn run_selftest() -> i32 {
    let cases: [(&str, fn() -> Result<(), String>); 4] = [
        ("slot_validator(红绿臂,rt 事实源直调)", selftest_slot_validator),
        ("slot_ring(写面隔离模型)", selftest_slot_ring),
        ("trajectory(双跑位级/可辨性)", selftest_trajectory),
        ("kernels(SPIR-V 结构/几何流)", selftest_kernels),
    ];
    let mut ok = true;
    for (name, f) in cases {
        match f() {
            Ok(()) => eprintln!("{TAG} selftest {name}: PASS"),
            Err(e) => {
                eprintln!("{TAG} selftest {name}: FAIL — {e}");
                ok = false;
            }
        }
    }
    if ok {
        println!("{TAG}: PASS selftest 4/4(纯 host;device 判档归 GPU 验收窗)");
        0
    } else {
        1
    }
}

// ---------------------------------------------------------------------------
// evidence JSON(手拼;独立 sidecar,不动既有 schema)
// ---------------------------------------------------------------------------

fn json_escape(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect(),
            '\n' => "\\n".chars().collect(),
            '\r' => "\\r".chars().collect(),
            '\t' => "\\t".chars().collect(),
            c if (c as u32) < 0x20 => format!("\\u{:04x}", c as u32).chars().collect(),
            c => vec![c],
        })
        .collect()
}

fn arm_json(r: &ArmRun) -> String {
    let ds: Vec<String> = r.digests.iter().map(|d| format!("\"{d}\"")).collect();
    format!(
        "{{ \"wall_ms\": {:.3}, \"ms_per_frame\": {:.4}, \"gpu_clear_ms_median\": {:.4}, \"gpu_rq_ms_median\": {:.4}, \"cpu_record_ms_median\": {:.4}, \"cpu_submit_ms_median\": {:.4}, \"cpu_fence_ms_median\": {:.4}, \"validation_errors\": {}, \"digests\": [{}] }}",
        r.wall_ms,
        r.wall_ms / r.digests.len().max(1) as f64,
        r.gpu_clear_ms,
        r.gpu_rq_ms,
        r.cpu_record_ms,
        r.cpu_submit_ms,
        r.cpu_fence_ms,
        r.validation_errors,
        ds.join(", ")
    )
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{TAG} 参数错误: {e}");
            std::process::exit(2);
        }
    };
    if args.selftest {
        std::process::exit(run_selftest());
    }
    if !rvk::vulkan_available() {
        skip_dev_env("vulkan loader 不可用");
    }
    let shape = SessionShape::new(args.rays_w, args.rays_h);
    let action_name = match args.action {
        TlasBuildAction::Rebuild => "rebuild",
        TlasBuildAction::Refit => "refit",
    };
    eprintln!(
        "{TAG}: 三臂判档启动 frames={} rays={}x{} action={action_name}(A=顺序基线 / B=FIF2 每槽 AS 副本 / C=FIF3 同构;各双跑)",
        args.frames, args.rays_w, args.rays_h
    );

    // 各臂双跑(重建会话重放;首个 B 跑携 device RED 双臂)。
    let run = |name: &str, slots: usize, red: bool| -> ArmRun {
        eprintln!("{TAG}: [{name}] …");
        match run_arm(&shape, slots, args.frames, args.action, red) {
            Ok(r) => r,
            Err(e) if is_dev_env_degrade(&e) => skip_dev_env(&e),
            Err(e) => {
                eprintln!("{TAG}: FAIL [{name}] {e}");
                std::process::exit(1);
            }
        }
    };
    let a1 = run("A#1 顺序基线", 1, false);
    let a2 = run("A#2 顺序基线重放", 1, false);
    let b1 = run("B#1 FIF2 每槽副本(含 RED 双臂)", 2, true);
    let b2 = run("B#2 FIF2 重放", 2, false);
    let c1 = run("C#1 FIF3 每槽副本", 3, false);
    let c2 = run("C#2 FIF3 重放", 3, false);

    // ── 判据 ──
    let mut failures: Vec<String> = Vec::new();
    let pairs: [(&str, &ArmRun, &ArmRun); 3] =
        [("A", &a1, &a2), ("B", &b1, &b2), ("C", &c1, &c2)];
    for (name, r1, r2) in pairs {
        if r1.digests != r2.digests {
            failures.push(format!("{name} 双跑位级破缺(重建会话重放 digest 漂移)"));
        }
    }
    if b1.digests != a1.digests {
        let first_bad = a1
            .digests
            .iter()
            .zip(b1.digests.iter())
            .position(|(x, y)| x != y)
            .map_or("长度不等".to_owned(), |i| format!("首异帧 {i}"));
        failures.push(format!("B ≠ A 逐帧 digest({first_bad})——每槽副本语义等价破缺"));
    }
    if c1.digests != a1.digests {
        let first_bad = a1
            .digests
            .iter()
            .zip(c1.digests.iter())
            .position(|(x, y)| x != y)
            .map_or("长度不等".to_owned(), |i| format!("首异帧 {i}"));
        failures.push(format!("C ≠ A 逐帧 digest({first_bad})——每槽副本语义等价破缺"));
    }
    let mut uniq: Vec<&String> = a1.digests.iter().collect();
    uniq.dedup();
    if uniq.len() < 2 {
        failures.push("digest 序列常量(动态见证破缺——轨迹未生效)".into());
    }
    for (name, r) in [("A", &a1), ("A2", &a2), ("B", &b1), ("B2", &b2), ("C", &c1), ("C2", &c2)] {
        if r.validation_errors != 0 {
            failures.push(format!(
                "{name} validation ERROR {}(须 0)",
                r.validation_errors
            ));
        }
    }
    let verdict = if failures.is_empty() { "PASS" } else { "RED" };

    // ── evidence sidecar(rurix.g31.fif_dyn_probe.v1)──
    let fail_json: Vec<String> = failures
        .iter()
        .map(|f| format!("\"{}\"", json_escape(f)))
        .collect();
    let json = format!(
        "{{\n  \"schema\": \"rurix.g31.fif_dyn_probe.v1\",\n  \"probe\": \"g31_fif_dyn_probe\",\n  \"todo\": 90,\n  \"args\": {{ \"frames\": {}, \"rays\": \"{}x{}\", \"action\": \"{action_name}\" }},\n  \"gates\": {{\n    \"b_eq_a_bytewise\": {},\n    \"c_eq_a_bytewise\": {},\n    \"double_run_bitlevel\": {},\n    \"validation_zero\": {},\n    \"dynamic_witness\": {},\n    \"red_arms_rejected\": true\n  }},\n  \"verdict\": \"{verdict}\",\n  \"failures\": [{}],\n  \"measured_note\": \"帧时为 measured 登记不设通过线;FIF 收益 = CPU record/submit/fence 解耦(GPU 帧间守卫 barrier 全序维持,RFC-0030 §4.3 L2 字面);微场景下 GPU 段近零,收益读数以 cpu_fence_ms 中位与 wall_ms 对照为准\",\n  \"arms\": {{\n    \"a_seq\": {},\n    \"a_seq_rerun\": {},\n    \"b_fif2\": {},\n    \"b_fif2_rerun\": {},\n    \"c_fif3\": {},\n    \"c_fif3_rerun\": {}\n  }}\n}}",
        args.frames,
        args.rays_w,
        args.rays_h,
        b1.digests == a1.digests,
        c1.digests == a1.digests,
        a1.digests == a2.digests && b1.digests == b2.digests && c1.digests == c2.digests,
        [&a1, &a2, &b1, &b2, &c1, &c2]
            .iter()
            .all(|r| r.validation_errors == 0),
        uniq.len() >= 2,
        fail_json.join(", "),
        arm_json(&a1),
        arm_json(&a2),
        arm_json(&b1),
        arm_json(&b2),
        arm_json(&c1),
        arm_json(&c2),
    );
    if let Some(path) = &args.out {
        if let Some(dir) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(e) = std::fs::write(path, &json) {
            eprintln!("{TAG}: FAIL evidence 写盘 {path}: {e}");
            std::process::exit(1);
        }
        eprintln!("{TAG}: evidence → {path}");
    } else {
        println!("{json}");
    }

    if failures.is_empty() {
        println!(
            "{TAG}: PASS frames={} action={action_name}(B/C≡A 逐帧 digest 逐字节 + 三臂双跑位级 + validation=0 + 动态见证 + RED 双臂必拒;帧时 A/B/C measured 已登记)",
            args.frames
        );
    } else {
        for f in &failures {
            eprintln!("{TAG}: RED {f}");
        }
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// 单测(--selftest 同判据 cargo test 承载)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_validator_arms() {
        selftest_slot_validator().expect("校验器红绿臂");
    }

    #[test]
    fn slot_ring_isolation() {
        selftest_slot_ring().expect("槽环写面隔离");
    }

    #[test]
    fn trajectory_deterministic() {
        selftest_trajectory().expect("轨迹确定性");
    }

    #[test]
    fn kernel_streams() {
        selftest_kernels().expect("kernel 结构");
    }
}
