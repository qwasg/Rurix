//! M92 GPU 蒙皮 compute kernel 的手编 SPIR-V 构建器与缓冲编组面(G9.3 M92
//! device 腿;spec/virtual_geometry.md RXS-0353;门 `g9.p1.m92.gpu_skinning_lod_update`)。
//!
//! ## kernel 语义(与 [`super::skinning`] host Kerbl 参照逐 op 镜像)
//!
//! cluster 感知 LBS:逐顶点 `p′ = Σ_k w_k·(M_{b_k}·p)`(累加序 = 权重行序,
//! 仅 `+`/`×`,无 FMA 收缩面——手编 SPV 逐指令发射,host/device IEEE 序列
//! 逐一对齐 ⇒ 定点化输入域(1/256 栅格)逐顶点对拍**容差 0**),并同 kernel
//! 输出:
//!
//! - 蒙皮后顶点 + 蒙皮法向(线性部 LBS,不归一化)——skin cache 布局写回面;
//! - **保守包围体块**(14 f32:AABB lo/hi + 包围球 center/r + 法向锥
//!   axis/half-angle):δ = 簇骨集各骨在静止 AABB 8 角点的最大位移(位移凸
//!   函数角点定理,Kerbl et al. 2021),AABB/球半径外扩 `δ + bound_inflation`;
//!   法向锥半角 += 簇骨集最大旋转角(`min(·, π)` 封顶)——与 host
//!   [`super::skinning::conservative_skinned_aabb`] /
//!   [`super::skinning::conservative_skinned_sphere`] /
//!   [`super::skinning::conservative_skinned_cone`] 逐 op 同式。
//!
//! ## 输入面划分(诚实声明)
//!
//! 骨**旋转角提取**(3×3 线性部刚性判定 + `acos`)含超越函数,归 palette
//! 构建面 host 侧一次计算([`pack_palette`]),kernel 消费角度表做簇骨集
//! `max`——避免 acos 双侧舍入分歧;**全部逐顶点蒙皮、位移上界、包围体放大**
//! 计算在 device kernel 内真跑。单骨行列式/正交判定见
//! [`super::skinning::bone_rotation_angle`]。
//!
//! ## 布局(全 SSBO `BufferBlock`+`Uniform`,SPIR-V 1.0,沿 rurixc 计算路面)
//!
//! - binding 0 `rest_pos` f32[3n] / 1 `rest_nrm` f32[3n];
//! - binding 2 `wval` f32[2n] / 3 `wbone` u32[2n](顶点主序,行定长
//!   [`M92_INFLUENCES`],零权 padding 位级中性);
//! - binding 4 `palette` f32[12·B](行主 3×4)/ 5 `bone_angle` f32[B];
//! - binding 6 `cluster_bones` u32[[`M92_CLUSTER_BONES`]](末位首骨重复
//!   padding,max 语义不变);
//! - binding 7 `out_pos` f32[3n] / 8 `out_nrm` f32[3n] / 9 `out_bound`
//!   f32[[`M92_BOUND_WORDS`]];
//! - push constants 48B:`n_vertices:u32`、`bound_inflation:f32`、
//!   `rest_aabb` 6×f32、`rest_cone` axis 3×f32 + half_angle(标量成员,
//!   4B 顺排,无向量对齐面)。
//!
//! 双 dispatch 组:groups.x = n_vertices,LocalSize 1×1×1;`gid < n_vertices`
//! 者写蒙皮顶点/法向,`gid == 0` 者额外写包围体块(单写者,无竞争)。
//!
//! 本模块纯 host safe 数据构造(零 unsafe、零后端调用);device 真跑归
//! `bin/g9_m92_skinning_device.rs`(rurix-rt render_exec 骨架)。

use super::skinning::{NormalCone, SkinPalette, bone_rotation_angle};

/// kernel 行宽(每顶点影响骨数;fixture 全簇定长 2,零权 padding)。
pub const M92_INFLUENCES: u32 = 2;
/// kernel 簇骨集定长(末位首骨重复 padding;max 语义不变)。
pub const M92_CLUSTER_BONES: u32 = 2;
/// 包围体输出块字数(AABB 6 + 球 4 + 锥 4)。
pub const M92_BOUND_WORDS: usize = 14;
/// push constants 字节数(12 标量 × 4B)。
pub const M92_PUSH_BYTES: usize = 48;

/// device 包围体输出块解码形态(与 host 三参照函数的返回同构)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeviceBound {
    /// 保守蒙皮 AABB。
    pub aabb: ([f32; 3], [f32; 3]),
    /// 保守蒙皮包围球(center, radius)。
    pub sphere: ([f32; 3], f32),
    /// 保守蒙皮法向锥。
    pub cone: NormalCone,
}

// ---------------------------------------------------------------------------
// SPIR-V 手编(无外部汇编器;沿 bin/vk_clas_rt 先例)
// ---------------------------------------------------------------------------

fn inst(v: &mut Vec<u32>, op: u32, ops: &[u32]) {
    v.push(op | ((ops.len() as u32 + 1) << 16));
    v.extend_from_slice(ops);
}

fn words(s: &str) -> Vec<u32> {
    let mut b = s.as_bytes().to_vec();
    b.push(0);
    while !b.len().is_multiple_of(4) {
        b.push(0);
    }
    b.chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// 蒙皮 kernel SPIR-V(SPIR-V 1.0;`influences`/`cluster_bones` 为构建期
/// 常量——影响行与簇骨集循环在 Rust 侧**完全展开**,kernel 本体零循环零
/// phi,控制流 = 两个 OpSelectionMerge 守卫段)。
pub fn m92_skin_spv(influences: u32, cluster_bones: u32) -> Vec<u32> {
    assert!(influences >= 1 && cluster_bones >= 1, "kernel 行宽/骨集非空");
    // ── id 布局(类型/常量/全局 < 100;函数体自 100 起)──
    let id_ext: u32 = 1; // GLSL.std.450 import
    let t_void = 2;
    let t_func = 3;
    let t_bool = 4;
    let t_u32 = 5;
    let t_f32 = 6;
    let t_vec3u = 7;
    let p_in_vec3u = 8;
    let v_gid = 9;
    let t_rt_f32 = 10;
    let t_struct_f32 = 11;
    let p_uni_struct_f32 = 12;
    let t_rt_u32 = 13;
    let t_struct_u32 = 14;
    let p_uni_struct_u32 = 15;
    let p_uni_f32 = 16;
    let p_uni_u32 = 17;
    let t_pc_struct = 18;
    let p_pc_struct = 19;
    let v_pc = 20;
    let p_pc_f32 = 21;
    let p_pc_u32 = 22;
    // SSBO 变量:0..=9 binding → id 23..=32
    let v_buf = |binding: u32| 23 + binding;
    let c_u32 = |x: u32| 40 + x; // u32 常量 0..=13 → id 40..=53
    let c_f32_zero = 60;
    let c_f32_half = 61;
    let c_f32_pi = 62;

    let mut pre: Vec<u32> = Vec::new(); // capabilities/import/memory model/entry/mode
    let mut ann: Vec<u32> = Vec::new(); // annotations
    let mut typ: Vec<u32> = Vec::new(); // types/constants/globals

    // ── capabilities / import / memory model / entry ──
    inst(&mut pre, 17, &[1]); // OpCapability Shader
    let mut ext = vec![id_ext];
    ext.extend(words("GLSL.std.450"));
    inst(&mut pre, 11, &ext); // OpExtInstImport
    inst(&mut pre, 14, &[0, 1]); // OpMemoryModel Logical GLSL450
    let mut ep = vec![5u32, 100]; // GLCompute %100(fn id 见下)
    ep.extend(words("main"));
    ep.extend_from_slice(&[v_gid]); // interface:全局输入变量
    inst(&mut pre, 15, &ep); // OpEntryPoint GLCompute %100 "main" %gid
    inst(&mut pre, 16, &[100, 17, 1, 1, 1]); // OpExecutionMode LocalSize 1 1 1

    // ── annotations ──
    inst(&mut ann, 71, &[v_gid, 11, 28]); // %gid BuiltIn GlobalInvocationId
    for b in 0..10u32 {
        inst(&mut ann, 71, &[v_buf(b), 34, 0]); // DescriptorSet 0
        inst(&mut ann, 71, &[v_buf(b), 33, b]); // Binding b
    }
    inst(&mut ann, 71, &[t_struct_f32, 3]); // f32 数组 struct BufferBlock(SSBO)
    inst(&mut ann, 72, &[t_struct_f32, 0, 35, 0]); // member0 Offset 0
    inst(&mut ann, 71, &[t_struct_u32, 3]); // u32 数组 struct BufferBlock
    inst(&mut ann, 72, &[t_struct_u32, 0, 35, 0]);
    inst(&mut ann, 71, &[t_rt_f32, 6, 4]); // f32[] ArrayStride 4
    inst(&mut ann, 71, &[t_rt_u32, 6, 4]); // u32[] ArrayStride 4
    inst(&mut ann, 71, &[t_pc_struct, 2]); // push const struct Block
    for m in 0..12u32 {
        inst(&mut ann, 72, &[t_pc_struct, m, 35, m * 4]); // member m Offset 4m
    }

    // ── types / constants / globals ──
    inst(&mut typ, 19, &[t_void]); // void
    inst(&mut typ, 33, &[t_func, t_void]); // fn()
    inst(&mut typ, 20, &[t_bool]); // bool
    inst(&mut typ, 21, &[t_u32, 32, 0]); // u32
    inst(&mut typ, 22, &[t_f32, 32]); // f32
    inst(&mut typ, 23, &[t_vec3u, t_u32, 3]); // uvec3
    inst(&mut typ, 32, &[p_in_vec3u, 1, t_vec3u]); // ptr Input uvec3
    inst(&mut typ, 59, &[p_in_vec3u, v_gid, 1]); // %gid Input
    inst(&mut typ, 29, &[t_rt_f32, t_f32]); // f32[]
    inst(&mut typ, 30, &[t_struct_f32, t_rt_f32]); // struct { f32[] }
    inst(&mut typ, 32, &[p_uni_struct_f32, 2, t_struct_f32]); // ptr Uniform
    inst(&mut typ, 29, &[t_rt_u32, t_u32]); // u32[]
    inst(&mut typ, 30, &[t_struct_u32, t_rt_u32]); // struct { u32[] }
    inst(&mut typ, 32, &[p_uni_struct_u32, 2, t_struct_u32]);
    inst(&mut typ, 32, &[p_uni_f32, 2, t_f32]); // ptr Uniform f32
    inst(&mut typ, 32, &[p_uni_u32, 2, t_u32]); // ptr Uniform u32
    // push const struct:member0 u32(n_vertices),member1..11 f32。
    let mut pc_members = vec![t_u32];
    pc_members.extend_from_slice(&[t_f32; 11]);
    let mut pc_ops = vec![t_pc_struct];
    pc_ops.extend_from_slice(&pc_members);
    inst(&mut typ, 30, &pc_ops);
    inst(&mut typ, 32, &[p_pc_struct, 9, t_pc_struct]); // ptr PushConstant
    inst(&mut typ, 59, &[p_pc_struct, v_pc, 9]); // %pc PushConstant
    inst(&mut typ, 32, &[p_pc_f32, 9, t_f32]);
    inst(&mut typ, 32, &[p_pc_u32, 9, t_u32]);
    // SSBO 变量:binding 0/1/2/4/5/7/8/9 = f32 数组;3/6 = u32 数组。
    for b in [0u32, 1, 2, 4, 5, 7, 8, 9] {
        inst(&mut typ, 59, &[p_uni_struct_f32, v_buf(b), 2]);
    }
    for b in [3u32, 6] {
        inst(&mut typ, 59, &[p_uni_struct_u32, v_buf(b), 2]);
    }
    // 常量:u32 0..=13;f32 0.0 / 0.5 / π。
    for x in 0..=13u32 {
        inst(&mut typ, 43, &[t_u32, c_u32(x), x]);
    }
    inst(&mut typ, 43, &[t_f32, c_f32_zero, 0.0f32.to_bits()]);
    inst(&mut typ, 43, &[t_f32, c_f32_half, 0.5f32.to_bits()]);
    inst(&mut typ, 43, &[t_f32, c_f32_pi, std::f32::consts::PI.to_bits()]);

    // ── 函数体 ──
    let mut body: Vec<u32> = Vec::new();
    // NoContraction(42)登记:全部 FAdd/FSub/FMul 结果 id——禁驱动 mul+add 融合
    // 收缩,device 浮点序列与 host 严格 IEEE 逐 op 对齐(容差 0 对拍的收缩免疫
    // 面;Vk 侧尊重 NoContraction)。
    let mut nc_ids: Vec<u32> = Vec::new();
    let mut nid = 100u32;
    let mut alloc = || {
        let i = nid;
        nid += 1;
        i
    };
    let fn_id = alloc(); // 100 = entry(OpEntryPoint/ExecutionMode 已引)
    let l_entry = alloc();
    let l_vertex = alloc();
    let l_merge1 = alloc();
    let l_bound = alloc();
    let l_merge2 = alloc();

    // 辅助:SSBO f32/u32 load/store 与 push const load(地址链 + load)。
    macro_rules! buf_load_f32 {
        ($b:expr, $idx:expr) => {{
            let (a, r) = (alloc(), alloc());
            inst(&mut body, 65, &[p_uni_f32, a, v_buf($b), c_u32(0), $idx]); // &buf[i]
            inst(&mut body, 61, &[t_f32, r, a]);
            r
        }};
    }
    macro_rules! buf_load_u32 {
        ($b:expr, $idx:expr) => {{
            let (a, r) = (alloc(), alloc());
            inst(&mut body, 65, &[p_uni_u32, a, v_buf($b), c_u32(0), $idx]);
            inst(&mut body, 61, &[t_u32, r, a]);
            r
        }};
    }
    macro_rules! buf_store_f32 {
        ($b:expr, $idx:expr, $val:expr) => {{
            let a = alloc();
            inst(&mut body, 65, &[p_uni_f32, a, v_buf($b), c_u32(0), $idx]);
            inst(&mut body, 62, &[a, $val]);
        }};
    }
    macro_rules! pc_load_f32 {
        ($m:expr) => {{
            let (a, r) = (alloc(), alloc());
            inst(&mut body, 65, &[p_pc_f32, a, v_pc, c_u32($m)]);
            inst(&mut body, 61, &[t_f32, r, a]);
            r
        }};
    }
    // 标量浮点算子(FMA 无;逐指令 IEEE + NoContraction 登记)。
    macro_rules! fmul {
        ($x:expr, $y:expr) => {{
            let r = alloc();
            inst(&mut body, 133, &[t_f32, r, $x, $y]); // OpFMul
            nc_ids.push(r);
            r
        }};
    }
    macro_rules! fadd {
        ($x:expr, $y:expr) => {{
            let r = alloc();
            inst(&mut body, 129, &[t_f32, r, $x, $y]); // OpFAdd
            nc_ids.push(r);
            r
        }};
    }
    macro_rules! fsub {
        ($x:expr, $y:expr) => {{
            let r = alloc();
            inst(&mut body, 131, &[t_f32, r, $x, $y]); // OpFSub
            nc_ids.push(r);
            r
        }};
    }
    macro_rules! iadd {
        ($x:expr, $y:expr) => {{
            let r = alloc();
            inst(&mut body, 128, &[t_u32, r, $x, $y]); // OpIAdd
            r
        }};
    }
    macro_rules! imul {
        ($x:expr, $y:expr) => {{
            let r = alloc();
            inst(&mut body, 132, &[t_u32, r, $x, $y]); // OpIMul
            r
        }};
    }
    macro_rules! fsqrt {
        ($x:expr) => {{
            let r = alloc();
            inst(&mut body, 12, &[t_f32, r, id_ext, 31, $x]); // GLSL.std.450 Sqrt
            r
        }};
    }
    // f32::max/min 同义(select(a>b,a,b) / select(a<b,a,b);非 NaN 域位级等)。
    macro_rules! fmax {
        ($x:expr, $y:expr) => {{
            let (c, r) = (alloc(), alloc());
            inst(&mut body, 186, &[t_bool, c, $x, $y]); // OpFOrdGreaterThan
            inst(&mut body, 169, &[t_f32, r, c, $x, $y]); // OpSelect
            r
        }};
    }
    macro_rules! fmin {
        ($x:expr, $y:expr) => {{
            let (c, r) = (alloc(), alloc());
            inst(&mut body, 184, &[t_bool, c, $x, $y]); // OpFOrdLessThan
            inst(&mut body, 169, &[t_f32, r, c, $x, $y]);
            r
        }};
    }

    // entry:OpFunction + entry block(gid 取出 + 范围守卫)。
    inst(&mut body, 54, &[t_void, fn_id, 0, t_func]); // OpFunction None
    inst(&mut body, 248, &[l_entry]); // %entry:
    let gid3 = alloc();
    inst(&mut body, 61, &[t_vec3u, gid3, v_gid]);
    let gid = alloc();
    inst(&mut body, 81, &[t_u32, gid, gid3, 0]); // gid.x
    let nv = {
        let (a, r) = (alloc(), alloc());
        inst(&mut body, 65, &[p_pc_u32, a, v_pc, c_u32(0)]);
        inst(&mut body, 61, &[t_u32, r, a]);
        r
    };
    let in_range = alloc();
    inst(&mut body, 176, &[t_bool, in_range, gid, nv]); // OpULessThan
    inst(&mut body, 247, &[l_merge1, 0]); // OpSelectionMerge
    inst(&mut body, 250, &[in_range, l_vertex, l_merge1]);

    // ── 顶点段:cluster 感知 LBS(逐顶点位置 + 法向)──
    inst(&mut body, 248, &[l_vertex]);
    let base3 = imul!(gid, c_u32(3));
    let base_k = imul!(gid, c_u32(influences));
    let px = buf_load_f32!(0, base3);
    let i1 = iadd!(base3, c_u32(1));
    let py = buf_load_f32!(0, i1);
    let i2 = iadd!(base3, c_u32(2));
    let pz = buf_load_f32!(0, i2);
    let nx = buf_load_f32!(1, base3);
    let ny = buf_load_f32!(1, i1);
    let nz = buf_load_f32!(1, i2);
    let mut acc = [c_f32_zero; 3];
    let mut nacc = [c_f32_zero; 3];
    for k in 0..influences {
        let wk = if k == 0 {
            base_k
        } else {
            iadd!(base_k, c_u32(k))
        };
        let w = buf_load_f32!(2, wk);
        let bone = buf_load_u32!(3, wk);
        let pb = imul!(bone, c_u32(12));
        // palette 行主 3×4:m[row][col] = palette[pb + row*4 + col]。
        let mut m = [0u32; 12];
        for (j, slot) in m.iter_mut().enumerate() {
            let idx = if j == 0 { pb } else { iadd!(pb, c_u32(j as u32)) };
            *slot = buf_load_f32!(4, idx);
        }
        // 位置行:t_i = m[4i]·px + m[4i+1]·py + m[4i+2]·pz + m[4i+3](host 同序)。
        for (i, a) in acc.iter_mut().enumerate() {
            let t0 = fmul!(m[4 * i], px);
            let t1 = fmul!(m[4 * i + 1], py);
            let t2 = fmul!(m[4 * i + 2], pz);
            let s = fadd!(t0, t1);
            let s = fadd!(s, t2);
            let s = fadd!(s, m[4 * i + 3]);
            let wt = fmul!(w, s);
            *a = fadd!(*a, wt);
        }
        // 法向行(线性部,无平移):u_i = m[4i]·nx + m[4i+1]·ny + m[4i+2]·nz。
        for (i, a) in nacc.iter_mut().enumerate() {
            let t0 = fmul!(m[4 * i], nx);
            let t1 = fmul!(m[4 * i + 1], ny);
            let t2 = fmul!(m[4 * i + 2], nz);
            let s = fadd!(t0, t1);
            let s = fadd!(s, t2);
            let wu = fmul!(w, s);
            *a = fadd!(*a, wu);
        }
    }
    for (j, &a) in acc.iter().enumerate() {
        let idx = if j == 0 { base3 } else { iadd!(base3, c_u32(j as u32)) };
        buf_store_f32!(7, idx, a);
    }
    for (j, &a) in nacc.iter().enumerate() {
        let idx = if j == 0 { base3 } else { iadd!(base3, c_u32(j as u32)) };
        buf_store_f32!(8, idx, a);
    }
    inst(&mut body, 249, &[l_merge1]);

    // ── 包围体段(gid == 0 单写者):δ 上界 + AABB/球/锥放大 ──
    inst(&mut body, 248, &[l_merge1]);
    let is_zero = alloc();
    inst(&mut body, 170, &[t_bool, is_zero, gid, c_u32(0)]); // OpIEqual
    inst(&mut body, 247, &[l_merge2, 0]);
    inst(&mut body, 250, &[is_zero, l_bound, l_merge2]);
    inst(&mut body, 248, &[l_bound]);
    let inflation = pc_load_f32!(1);
    let aabb_lo = [pc_load_f32!(2), pc_load_f32!(3), pc_load_f32!(4)];
    let aabb_hi = [pc_load_f32!(5), pc_load_f32!(6), pc_load_f32!(7)];
    let cone_axis = [pc_load_f32!(8), pc_load_f32!(9), pc_load_f32!(10)];
    let cone_half = pc_load_f32!(11);
    // δ = max over 簇骨集 × 8 角点(host max_bone_displacement 同式同序)。
    let mut delta = c_f32_zero;
    let mut theta = c_f32_zero;
    for j in 0..cluster_bones {
        let bone = buf_load_u32!(6, c_u32(j));
        let ang = buf_load_f32!(5, bone);
        theta = fmax!(theta, ang);
        let pb = imul!(bone, c_u32(12));
        let mut m = [0u32; 12];
        for (q, slot) in m.iter_mut().enumerate() {
            let idx = if q == 0 { pb } else { iadd!(pb, c_u32(q as u32)) };
            *slot = buf_load_f32!(4, idx);
        }
        // 角点:x 外、y 中、z 内(host 序);cx/cy/cz = lo/hi 二择(构建期)。
        for xi in 0..2usize {
            for yi in 0..2usize {
                for zi in 0..2usize {
                    let c = [aabb_lo, aabb_hi];
                    let (cx, cy, cz) = (c[xi][0], c[yi][1], c[zi][2]);
                    let mut d2 = c_f32_zero;
                    for (row, cc) in [(0, cx), (1, cy), (2, cz)] {
                        let t0 = fmul!(m[4 * row], cx);
                        let t1 = fmul!(m[4 * row + 1], cy);
                        let t2 = fmul!(m[4 * row + 2], cz);
                        let s = fadd!(t0, t1);
                        let s = fadd!(s, t2);
                        let s = fadd!(s, m[4 * row + 3]);
                        let t = fsub!(s, cc);
                        let tt = fmul!(t, t);
                        d2 = fadd!(d2, tt);
                    }
                    let d = fsqrt!(d2);
                    delta = fmax!(delta, d);
                }
            }
        }
    }
    let grow = fadd!(delta, inflation);
    // AABB:lo − grow / hi + grow(host 同序:先 grow 后逐轴 ±)。
    for k in 0..3usize {
        let lo = fsub!(aabb_lo[k], grow);
        buf_store_f32!(9, c_u32(k as u32), lo);
        let hi = fadd!(aabb_hi[k], grow);
        buf_store_f32!(9, c_u32(3 + k as u32), hi);
    }
    // 包围球:center = (lo+hi)·0.5;r = √(Σ((hi−lo)·0.5)²);r + grow。
    let mut center = [0u32; 3];
    let mut half = [0u32; 3];
    for k in 0..3usize {
        let s = fadd!(aabb_lo[k], aabb_hi[k]);
        center[k] = fmul!(s, c_f32_half);
        let e = fsub!(aabb_hi[k], aabb_lo[k]);
        half[k] = fmul!(e, c_f32_half);
    }
    let r2 = {
        let e0 = fmul!(half[0], half[0]);
        let e1 = fmul!(half[1], half[1]);
        let e2 = fmul!(half[2], half[2]);
        let s = fadd!(e0, e1);
        fadd!(s, e2)
    };
    let r = fsqrt!(r2);
    for (k, &c) in center.iter().enumerate() {
        buf_store_f32!(9, c_u32(6 + k as u32), c);
    }
    let rs = fadd!(r, grow);
    buf_store_f32!(9, c_u32(9), rs);
    // 法向锥:轴透传;half′ = min(rest_half + θ, π)。
    let half2 = fadd!(cone_half, theta);
    let half2 = fmin!(half2, c_f32_pi);
    for (k, &a) in cone_axis.iter().enumerate() {
        buf_store_f32!(9, c_u32(10 + k as u32), a);
    }
    buf_store_f32!(9, c_u32(13), half2);
    inst(&mut body, 249, &[l_merge2]);
    inst(&mut body, 248, &[l_merge2]);
    inst(&mut body, 253, &[]); // OpReturn
    inst(&mut body, 56, &[]); // OpFunctionEnd

    // NoContraction 注解(函数体构建后一次性补入 annotation 段)。
    for id in nc_ids {
        inst(&mut ann, 71, &[id, 42]); // OpDecorate %id NoContraction
    }

    // ── 组装(bound = 最大 id + 1)──
    let mut v = vec![0x0723_0203u32, 0x0001_0000, 0, nid, 0]; // SPIR-V 1.0
    v.extend_from_slice(&pre);
    v.extend_from_slice(&ann);
    v.extend_from_slice(&typ);
    v.extend_from_slice(&body);
    v
}

// ---------------------------------------------------------------------------
// 缓冲编组 / 解码(harness 与单测共用)
// ---------------------------------------------------------------------------

/// 逐簇 kernel 输入包(SSBO 字节 + push constants;binding 序固定)。
#[derive(Debug, Clone)]
pub struct ClusterKernelPack {
    /// binding 0:静止顶点 f32[3n]。
    pub rest_pos: Vec<u8>,
    /// binding 1:静止法向 f32[3n]。
    pub rest_nrm: Vec<u8>,
    /// binding 2:权重 f32[2n](顶点主序,行定长 [`M92_INFLUENCES`])。
    pub wval: Vec<u8>,
    /// binding 3:骨骼 u32[2n]。
    pub wbone: Vec<u8>,
    /// binding 6:簇骨集 u32[[`M92_CLUSTER_BONES`]]。
    pub cluster_bones: Vec<u8>,
    /// push constants(48B,布局见模块头)。
    pub push: Vec<u8>,
    /// 顶点数(dispatch groups.x)。
    pub n_vertices: u32,
}

fn push_f32(out: &mut Vec<u8>, xs: &[[f32; 3]]) {
    for v in xs {
        for &x in v {
            out.extend_from_slice(&x.to_le_bytes());
        }
    }
}

/// palette 编组:binding 4 骨骼矩阵字节(行主 3×4 顺排)+ binding 5 逐骨
/// 旋转角字节(host 单源提取,见模块头「输入面划分」)。
pub fn pack_palette(palette: &SkinPalette) -> (Vec<u8>, Vec<u8>) {
    let mut mat = Vec::with_capacity(palette.bones.len() * 48);
    for b in &palette.bones {
        for row in b {
            for &x in row {
                mat.extend_from_slice(&x.to_le_bytes());
            }
        }
    }
    let mut ang = Vec::with_capacity(palette.bones.len() * 4);
    for b in &palette.bones {
        ang.extend_from_slice(&bone_rotation_angle(b).to_le_bytes());
    }
    (mat, ang)
}

/// 簇输入编组(`input` 须经 host 校核;`normals` 与顶点等长;权重行定长
/// [`M92_INFLUENCES`],骨集定长 [`M92_CLUSTER_BONES`]——fixture 不变式,
/// 越界/短行 = panic,调用契约违例)。
pub fn pack_cluster(
    vertices: &[[f32; 3]],
    normals: &[[f32; 3]],
    weights: &[Vec<(u32, f32)>],
    bone_indices: &[u32],
    bound_inflation: f32,
    rest_aabb: ([f32; 3], [f32; 3]),
    rest_cone: &NormalCone,
) -> ClusterKernelPack {
    assert_eq!(vertices.len(), normals.len(), "法向/顶点表长不齐");
    assert_eq!(vertices.len(), weights.len(), "权重/顶点表长不齐");
    assert_eq!(bone_indices.len(), M92_CLUSTER_BONES as usize, "簇骨集定长");
    let mut rest_pos = Vec::new();
    push_f32(&mut rest_pos, vertices);
    let mut rest_nrm = Vec::new();
    push_f32(&mut rest_nrm, normals);
    let mut wval = Vec::new();
    let mut wbone = Vec::new();
    for row in weights {
        assert_eq!(row.len(), M92_INFLUENCES as usize, "权重行定长");
        for &(b, w) in row {
            wbone.extend_from_slice(&b.to_le_bytes());
            wval.extend_from_slice(&w.to_le_bytes());
        }
    }
    let mut cb = Vec::new();
    for &b in bone_indices {
        cb.extend_from_slice(&b.to_le_bytes());
    }
    let mut push = Vec::with_capacity(M92_PUSH_BYTES);
    push.extend_from_slice(&(vertices.len() as u32).to_le_bytes());
    push.extend_from_slice(&bound_inflation.to_le_bytes());
    for &x in &rest_aabb.0 {
        push.extend_from_slice(&x.to_le_bytes());
    }
    for &x in &rest_aabb.1 {
        push.extend_from_slice(&x.to_le_bytes());
    }
    for &x in &rest_cone.axis {
        push.extend_from_slice(&x.to_le_bytes());
    }
    push.extend_from_slice(&rest_cone.half_angle.to_le_bytes());
    assert_eq!(push.len(), M92_PUSH_BYTES);
    ClusterKernelPack {
        rest_pos,
        rest_nrm,
        wval,
        wbone,
        cluster_bones: cb,
        push,
        n_vertices: vertices.len() as u32,
    }
}

/// 蒙皮输出解码(f32×3/顶点;device 回读字节 → 位置/法向)。
pub fn decode_vec3s(bytes: &[u8], n: usize) -> Vec<[f32; 3]> {
    assert_eq!(bytes.len(), n * 12, "回读字节数失配");
    bytes
        .chunks_exact(12)
        .map(|c| {
            [
                f32::from_le_bytes([c[0], c[1], c[2], c[3]]),
                f32::from_le_bytes([c[4], c[5], c[6], c[7]]),
                f32::from_le_bytes([c[8], c[9], c[10], c[11]]),
            ]
        })
        .collect()
}

/// 包围体块解码(14 f32:AABB lo/hi + 球 center/r + 锥 axis/half)。
pub fn decode_bound(bytes: &[u8]) -> DeviceBound {
    assert_eq!(bytes.len(), M92_BOUND_WORDS * 4, "包围体块字节数失配");
    let f = |i: usize| f32::from_le_bytes([bytes[4 * i], bytes[4 * i + 1], bytes[4 * i + 2], bytes[4 * i + 3]]);
    DeviceBound {
        aabb: ([f(0), f(1), f(2)], [f(3), f(4), f(5)]),
        sphere: ([f(6), f(7), f(8)], f(9)),
        cone: NormalCone {
            axis: [f(10), f(11), f(12)],
            half_angle: f(13),
        },
    }
}

// ---------------------------------------------------------------------------
// 单测
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::skinning::{BoneTransform, m92_fixture};

    /// 指令流结构自洽:逐指令 word-count 推进必须恰好穷尽模块(手编 SPV
    /// 的最低结构锚;坏 wc 必乱序)。
    fn walk_instructions(spv: &[u32]) -> usize {
        let mut i = 5usize;
        let mut count = 0;
        while i < spv.len() {
            let wc = (spv[i] >> 16) as usize;
            assert!(wc >= 1, "偏移 {i} 指令 word-count 为 0");
            assert!(i + wc <= spv.len(), "偏移 {i} 指令越出模块尾");
            i += wc;
            count += 1;
        }
        assert_eq!(i, spv.len(), "指令流未恰好穷尽模块");
        count
    }

    //@ spec: RXS-0353
    #[test]
    fn skin_spv_header_and_structure() {
        let spv = m92_skin_spv(M92_INFLUENCES, M92_CLUSTER_BONES);
        assert_eq!(spv[0], 0x0723_0203, "SPIR-V magic");
        assert_eq!(spv[1], 0x0001_0000, "SPIR-V 1.0(BufferBlock+Uniform 形态)");
        let bound = spv[3] as usize;
        assert!(bound > 100, "bound 覆盖函数体 id 域");
        let n_inst = walk_instructions(&spv);
        assert!(n_inst > 100, "kernel 非平凡指令数: {n_inst}");
        // 全部结果 id < bound(粗扫:指令首操作数为结果 id 的族——此处抽查
        // OpFMul(133)/OpFAdd(129) 结果 id 界)。
        let mut i = 5usize;
        while i < spv.len() {
            let wc = (spv[i] >> 16) as usize;
            let op = spv[i] & 0xffff;
            if matches!(op, 129 | 133 | 131) {
                assert!((spv[i + 2] as usize) < bound, "结果 id 越出 bound");
            }
            i += wc;
        }
    }

    //@ spec: RXS-0353
    #[test]
    fn skin_spv_entry_point_and_capabilities() {
        let spv = m92_skin_spv(M92_INFLUENCES, M92_CLUSTER_BONES);
        // OpEntryPoint(15)存在且模型 = GLCompute(5);OpExecutionMode(16)
        // LocalSize(17) 1 1 1;OpCapability(17) Shader(1)。
        let (mut has_entry, mut has_mode, mut has_cap) = (false, false, false);
        let mut i = 5usize;
        while i < spv.len() {
            let wc = (spv[i] >> 16) as usize;
            match spv[i] & 0xffff {
                15 => {
                    assert_eq!(spv[i + 1], 5, "执行模型 = GLCompute");
                    has_entry = true;
                }
                16 => {
                    assert_eq!(&spv[i + 2..i + 6], &[17, 1, 1, 1], "LocalSize 1 1 1");
                    has_mode = true;
                }
                17 => {
                    assert_eq!(spv[i + 1], 1, "唯一 capability = Shader");
                    has_cap = true;
                }
                _ => {}
            }
            i += wc;
        }
        assert!(has_entry && has_mode && has_cap);
    }

    //@ spec: RXS-0353
    #[test]
    fn skin_spv_double_build_bit_identical() {
        let a = m92_skin_spv(M92_INFLUENCES, M92_CLUSTER_BONES);
        let b = m92_skin_spv(M92_INFLUENCES, M92_CLUSTER_BONES);
        assert_eq!(a, b, "SPV 构建确定性(同参数字节全等)");
        // 行宽参数化:不同行宽产不同字节(kernel 形状随构建参数)。
        let c = m92_skin_spv(4, 3);
        assert_ne!(a, c);
    }

    //@ spec: RXS-0353
    #[test]
    fn pack_cluster_push_constants_golden() {
        let f = m92_fixture();
        let c0 = &f.clusters[0];
        let pack = pack_cluster(
            &c0.vertices,
            &c0.normals,
            &c0.weights,
            &c0.bone_indices,
            c0.bound_inflation,
            c0.rest_aabb,
            &c0.rest_cone,
        );
        assert_eq!(pack.n_vertices, 4);
        assert_eq!(pack.push.len(), M92_PUSH_BYTES);
        // member0 = n_vertices(u32);member1 = inflation;member2..4 = aabb_min。
        assert_eq!(&pack.push[0..4], &4u32.to_le_bytes());
        assert_eq!(&pack.push[4..8], &0.125f32.to_le_bytes());
        assert_eq!(&pack.push[8..12], &0.0f32.to_le_bytes());
        // 锥轴 (0,0,1) @ 32..44;半角 0 @ 44..48。
        assert_eq!(&pack.push[32..36], &0.0f32.to_le_bytes());
        assert_eq!(&pack.push[40..44], &1.0f32.to_le_bytes());
        assert_eq!(&pack.push[44..48], &0.0f32.to_le_bytes());
        // 权重行定长 2 × 4 顶点:wval 32B / wbone 32B;骨集 2 × u32。
        assert_eq!(pack.wval.len(), 32);
        assert_eq!(pack.wbone.len(), 32);
        assert_eq!(pack.cluster_bones.len(), 8);
        assert_eq!(&pack.cluster_bones[0..4], &0u32.to_le_bytes());
        assert_eq!(&pack.cluster_bones[4..8], &1u32.to_le_bytes());
    }

    //@ spec: RXS-0353
    #[test]
    fn pack_palette_bytes_and_bone_angles() {
        let f = m92_fixture();
        for (pi, palette) in f.poses.iter().enumerate() {
            let (mat, ang) = pack_palette(palette);
            assert_eq!(mat.len(), 3 * 48);
            assert_eq!(ang.len(), 3 * 4);
            // 首 12 B = 骨 0 行 0 前三元 + 平移分量。
            let row0 = &mat[0..16];
            let expected: [u8; 16] = {
                let mut b = [0u8; 16];
                for (k, &x) in palette.bones[0][0].iter().enumerate() {
                    b[4 * k..4 * k + 4].copy_from_slice(&x.to_le_bytes());
                }
                b
            };
            assert_eq!(row0, &expected, "姿态 {pi} 骨 0 行 0 字节序");
            if pi == 0 {
                // 全恒等:旋转角全 0。
                assert!(ang.chunks_exact(4).all(|c| c == 0.0f32.to_le_bytes()));
            }
            if pi == 2 {
                // 骨 0 = rot_x_90 ⇒ π/2;骨 2 = 纯平移 ⇒ 0。
                assert_eq!(&ang[0..4], &std::f32::consts::FRAC_PI_2.to_le_bytes());
                assert_eq!(&ang[8..12], &0.0f32.to_le_bytes());
            }
        }
    }

    //@ spec: RXS-0353
    #[test]
    fn decode_roundtrip_and_shape_checks() {
        let vals = [[1.5f32, -2.0, 0.25], [0.0, 3.0, -4.5]];
        let mut bytes = Vec::new();
        push_f32(&mut bytes, &vals);
        assert_eq!(decode_vec3s(&bytes, 2), vals);
        // 包围体块:14 f32 逐位往返。
        let mut bound = Vec::new();
        for i in 0..M92_BOUND_WORDS {
            bound.extend_from_slice(&(i as f32).to_le_bytes());
        }
        let d = decode_bound(&bound);
        assert_eq!(d.aabb.0, [0.0, 1.0, 2.0]);
        assert_eq!(d.aabb.1, [3.0, 4.0, 5.0]);
        assert_eq!(d.sphere, ([6.0, 7.0, 8.0], 9.0));
        assert_eq!(d.cone.axis, [10.0, 11.0, 12.0]);
        assert_eq!(d.cone.half_angle, 13.0);
    }

    //@ spec: RXS-0353
    #[test]
    fn bone_transform_layout_matches_kernel_indexing() {
        // kernel 寻址:palette[b*12 + row*4 + col] = BoneTransform[row][col]
        // (行主 3×4)——与 pack_palette 字节序互锁。
        let m: BoneTransform = [
            [0.0, 1.0, 2.0, 3.0],
            [4.0, 5.0, 6.0, 7.0],
            [8.0, 9.0, 10.0, 11.0],
        ];
        let palette = SkinPalette { bones: vec![m] };
        let (mat, _) = pack_palette(&palette);
        for row in 0..3usize {
            for col in 0..4usize {
                let off = (row * 4 + col) * 4;
                let got = f32::from_le_bytes([
                    mat[off],
                    mat[off + 1],
                    mat[off + 2],
                    mat[off + 3],
                ]);
                assert_eq!(got, (row * 4 + col) as f32);
            }
        }
    }
}
