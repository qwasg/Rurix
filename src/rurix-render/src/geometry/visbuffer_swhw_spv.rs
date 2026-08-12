//! M95 蒙皮簇 VisBuffer SW/HW 双腿的手编 SPIR-V 构建器(G9.3 M95 device 腿;
//! spec/virtual_geometry.md RXS-0352 L2;判据体例 = G7.5b RXS-0303「SW 精确
//! 边函数 + top-left 为覆盖唯一权威,HW 保守光栅 OVERESTIMATE 超集 + FS 逐字
//! 复刻判定」)。
//!
//! ## 三内核(与 G7.5b 语料 `visbuffer_sw_u64.rx` / `vk_hw_raster_visbuffer_{vs,fs}.rx`
//! ## 判定段逐 op 同构;位格式/常量与冻结契约 `graph::types::visbuffer_pack` 一致)
//!
//! - **SW compute 腿**([`sw_visbuffer_u64_spv`]):`gid` 分解 (triangle, pixel),
//!   精确 f32 边函数 + top-left 补边 + reverse-Z 30 位量化(RoundEven)+ u64
//!   `OpAtomicUMax`(Device/Relaxed)写 `depth30|cluster27|tri7`;push consts =
//!   (tri_count, width, height)。
//! - **HW 图形腿**([`hw_visbuffer_vs_spv`] + [`hw_visbuffer_fs_spv`]):VS 纯
//!   passthrough(每三角形 3 顶点携全等 flat va/vb/vc/ids,provoking vertex /
//!   guard-band 裁剪免疫);FS 以 `frag_coord`(像素中心 k+0.5)为采样点**逐字
//!   复刻** SW 判定;保守光栅 OVERESTIMATE 派发 fragment 超集,`inside` 不成立
//!   不写(无 discard)⇒ HW 写集 = SW 精确集。
//! - 双腿同一判定表达式序列(逐指令同序同型)⇒ device 整数域 **diff = 0**
//!   (零容差);host `VisBufferCpu` 为覆盖集合 oracle(打包值受 FMA 限制不进
//!   判据,G7.5b 残差归因同构)。
//!
//! SSBO 全 `BufferBlock`+`Uniform`(SPIR-V 1.0 形态,沿 rurixc 已验证面);
//! u64 原子 capability = Int64 + Int64Atomics(W2 能力链,render_exec 探测
//! 启用;缺失 fail-closed)。本模块纯 host safe 数据构造(零 unsafe、零后端
//! 调用);device 真跑归 `bin/g9_m95_visbuffer_swhw.rs`。

// ---------------------------------------------------------------------------
// 手编基础设施(沿 bin/vk_clas_rt / geometry/skin_kernel 先例)
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

/// `(1 << 30) - 1` 的 f32 最近舍入值 = 2³⁰(`0x4E800000`;host
/// `DEPTH30_MAX as f32` 同值——round-to-nearest 跨端一致)。
const DEPTH30_SCALE_F32_BITS: u32 = 0x4E80_0000;

// ---------------------------------------------------------------------------
// SW compute 腿
// ---------------------------------------------------------------------------

/// SW 精确 VisBuffer compute SPIR-V(绑定:0 = triangles f32[9t],1 = ids
/// u32[2t],2 = vis u64[W·H];push consts u32×3 = tri_count/width/height;
/// LocalSize 1×1×1,groups.x = tri_count·W·H)。
pub fn sw_visbuffer_u64_spv() -> Vec<u32> {
    // id 布局:类型/常量/全局 < 100;函数体自 100。
    let id_ext = 1;
    let t_void = 2;
    let t_func = 3;
    let t_bool = 4;
    let t_u32 = 5;
    let t_u64 = 6;
    let t_f32 = 7;
    let t_vec3u = 8;
    let p_in_vec3u = 9;
    let v_gid = 10;
    let t_rt_f32 = 11;
    let t_st_f32 = 12;
    let p_uni_st_f32 = 13;
    let t_rt_u32 = 14;
    let t_st_u32 = 15;
    let p_uni_st_u32 = 16;
    let t_rt_u64 = 17;
    let t_st_u64 = 18;
    let p_uni_st_u64 = 19;
    let p_uni_f32 = 20;
    let p_uni_u32 = 21;
    let p_uni_u64 = 22;
    let t_pc = 23;
    let p_pc = 24;
    let v_pc = 25;
    let p_pc_u32 = 26;
    let v_tris = 27;
    let v_ids = 28;
    let v_vis = 29;
    let c_u32 = |x: u32| 30 + x; // u32 0..=9 → 30..=39
    let c_depth_max = 40; // u32 1073741823
    let c_u64_7 = 41;
    let c_u64_34 = 42;
    let c_half = 43; // f32 0.5
    let c_zero = 44; // f32 0.0
    let c_one = 45; // f32 1.0
    let c_scale = 46; // f32 2³⁰

    let mut pre = Vec::new();
    let mut ann = Vec::new();
    let mut typ = Vec::new();

    inst(&mut pre, 17, &[1]); // Shader
    inst(&mut pre, 17, &[11]); // Int64
    inst(&mut pre, 17, &[12]); // Int64Atomics
    let mut ext = vec![id_ext];
    ext.extend(words("GLSL.std.450"));
    inst(&mut pre, 11, &ext);
    inst(&mut pre, 14, &[0, 1]); // Logical GLSL450
    let mut ep = vec![5u32, 100];
    ep.extend(words("main"));
    ep.push(v_gid);
    inst(&mut pre, 15, &ep); // OpEntryPoint GLCompute
    inst(&mut pre, 16, &[100, 17, 1, 1, 1]); // LocalSize 1 1 1

    inst(&mut ann, 71, &[v_gid, 11, 28]); // GlobalInvocationId
    for (v, b) in [(v_tris, 0u32), (v_ids, 1), (v_vis, 2)] {
        inst(&mut ann, 71, &[v, 34, 0]);
        inst(&mut ann, 71, &[v, 33, b]);
    }
    for st in [t_st_f32, t_st_u32, t_st_u64] {
        inst(&mut ann, 71, &[st, 3]); // BufferBlock
        inst(&mut ann, 72, &[st, 0, 35, 0]); // member0 Offset 0
    }
    inst(&mut ann, 71, &[t_rt_f32, 6, 4]); // ArrayStride 4
    inst(&mut ann, 71, &[t_rt_u32, 6, 4]);
    inst(&mut ann, 71, &[t_rt_u64, 6, 8]); // u64[] ArrayStride 8
    inst(&mut ann, 71, &[t_pc, 2]); // Block
    for m in 0..3u32 {
        inst(&mut ann, 72, &[t_pc, m, 35, m * 4]);
    }

    inst(&mut typ, 19, &[t_void]);
    inst(&mut typ, 33, &[t_func, t_void]);
    inst(&mut typ, 20, &[t_bool]);
    inst(&mut typ, 21, &[t_u32, 32, 0]);
    inst(&mut typ, 21, &[t_u64, 64, 0]);
    inst(&mut typ, 22, &[t_f32, 32]);
    inst(&mut typ, 23, &[t_vec3u, t_u32, 3]);
    inst(&mut typ, 32, &[p_in_vec3u, 1, t_vec3u]);
    inst(&mut typ, 59, &[p_in_vec3u, v_gid, 1]);
    inst(&mut typ, 29, &[t_rt_f32, t_f32]);
    inst(&mut typ, 30, &[t_st_f32, t_rt_f32]);
    inst(&mut typ, 32, &[p_uni_st_f32, 2, t_st_f32]);
    inst(&mut typ, 29, &[t_rt_u32, t_u32]);
    inst(&mut typ, 30, &[t_st_u32, t_rt_u32]);
    inst(&mut typ, 32, &[p_uni_st_u32, 2, t_st_u32]);
    inst(&mut typ, 29, &[t_rt_u64, t_u64]);
    inst(&mut typ, 30, &[t_st_u64, t_rt_u64]);
    inst(&mut typ, 32, &[p_uni_st_u64, 2, t_st_u64]);
    inst(&mut typ, 32, &[p_uni_f32, 2, t_f32]);
    inst(&mut typ, 32, &[p_uni_u32, 2, t_u32]);
    inst(&mut typ, 32, &[p_uni_u64, 2, t_u64]);
    inst(&mut typ, 30, &[t_pc, t_u32, t_u32, t_u32]);
    inst(&mut typ, 32, &[p_pc, 9, t_pc]);
    inst(&mut typ, 59, &[p_pc, v_pc, 9]);
    inst(&mut typ, 32, &[p_pc_u32, 9, t_u32]);
    inst(&mut typ, 59, &[p_uni_st_f32, v_tris, 2]);
    inst(&mut typ, 59, &[p_uni_st_u32, v_ids, 2]);
    inst(&mut typ, 59, &[p_uni_st_u64, v_vis, 2]);
    for x in 0..=9u32 {
        inst(&mut typ, 43, &[t_u32, c_u32(x), x]);
    }
    inst(&mut typ, 43, &[t_u32, c_depth_max, (1 << 30) - 1]);
    inst(&mut typ, 43, &[t_u64, c_u64_7, 7, 0]);
    inst(&mut typ, 43, &[t_u64, c_u64_34, 34, 0]);
    inst(&mut typ, 43, &[t_f32, c_half, 0.5f32.to_bits()]);
    inst(&mut typ, 43, &[t_f32, c_zero, 0.0f32.to_bits()]);
    inst(&mut typ, 43, &[t_f32, c_one, 1.0f32.to_bits()]);
    inst(&mut typ, 43, &[t_f32, c_scale, DEPTH30_SCALE_F32_BITS]);

    let mut body = Vec::new();
    let mut nid = 100u32;
    macro_rules! alloc {
        () => {{
            let i = nid;
            nid += 1;
            i
        }};
    }
    macro_rules! ld_f32 {
        ($v:expr, $idx:expr) => {{
            let (a, r) = (alloc!(), alloc!());
            inst(&mut body, 65, &[p_uni_f32, a, $v, c_u32(0), $idx]);
            inst(&mut body, 61, &[t_f32, r, a]);
            r
        }};
    }
    macro_rules! ld_u32 {
        ($v:expr, $idx:expr) => {{
            let (a, r) = (alloc!(), alloc!());
            inst(&mut body, 65, &[p_uni_u32, a, $v, c_u32(0), $idx]);
            inst(&mut body, 61, &[t_u32, r, a]);
            r
        }};
    }
    macro_rules! pc_u32 {
        ($m:expr) => {{
            let (a, r) = (alloc!(), alloc!());
            inst(&mut body, 65, &[p_pc_u32, a, v_pc, c_u32($m)]);
            inst(&mut body, 61, &[t_u32, r, a]);
            r
        }};
    }
    macro_rules! fadd {
        ($x:expr, $y:expr) => {{
            let r = alloc!();
            inst(&mut body, 129, &[t_f32, r, $x, $y]);
            r
        }};
    }
    macro_rules! iadd {
        ($x:expr, $y:expr) => {{
            let r = alloc!();
            inst(&mut body, 128, &[t_u32, r, $x, $y]);
            r
        }};
    }
    macro_rules! imul {
        ($x:expr, $y:expr) => {{
            let r = alloc!();
            inst(&mut body, 132, &[t_u32, r, $x, $y]);
            r
        }};
    }

    let fn_id = alloc!(); // 100
    let l_entry = alloc!();
    let l_in = alloc!();
    let l_m1 = alloc!();
    inst(&mut body, 54, &[t_void, fn_id, 0, t_func]);
    inst(&mut body, 248, &[l_entry]);
    let gid3 = alloc!();
    inst(&mut body, 61, &[t_vec3u, gid3, v_gid]);
    let gid = alloc!();
    inst(&mut body, 81, &[t_u32, gid, gid3, 0]);
    let tc = pc_u32!(0);
    let width = pc_u32!(1);
    let height = pc_u32!(2);
    let pix_count = imul!(width, height);
    let total = imul!(tc, pix_count);
    let in_range = alloc!();
    inst(&mut body, 176, &[t_bool, in_range, gid, total]); // ULessThan
    inst(&mut body, 247, &[l_m1, 0]);
    inst(&mut body, 250, &[in_range, l_in, l_m1]);
    inst(&mut body, 248, &[l_in]);
    // gid 分解:tri_idx = gid / pix_count;pixel = gid % pix_count;
    // px = (pixel % width)+0.5;py = (pixel / width)+0.5。
    let tri_idx = alloc!();
    inst(&mut body, 134, &[t_u32, tri_idx, gid, pix_count]); // UDiv
    let pixel = alloc!();
    inst(&mut body, 137, &[t_u32, pixel, gid, pix_count]); // UMod
    let px_i = alloc!();
    inst(&mut body, 137, &[t_u32, px_i, pixel, width]);
    let py_i = alloc!();
    inst(&mut body, 134, &[t_u32, py_i, pixel, width]);
    let px = {
        let f = alloc!();
        inst(&mut body, 112, &[t_f32, f, px_i]); // UToF
        fadd!(f, c_half)
    };
    let py = {
        let f = alloc!();
        inst(&mut body, 112, &[t_f32, f, py_i]);
        fadd!(f, c_half)
    };
    let base = imul!(tri_idx, c_u32(9));
    let mut t9 = [0u32; 9];
    for (k, slot) in t9.iter_mut().enumerate() {
        let idx = if k == 0 { base } else { iadd!(base, c_u32(k as u32)) };
        *slot = ld_f32!(v_tris, idx);
    }
    let (ax, ay, az) = (t9[0], t9[1], t9[2]);
    let (bx0, by0, bz0) = (t9[3], t9[4], t9[5]);
    let (cx0, cy0, cz0) = (t9[6], t9[7], t9[8]);
    let cluster = {
        let i = imul!(tri_idx, c_u32(2));
        ld_u32!(v_ids, i)
    };
    let tri = {
        let i = imul!(tri_idx, c_u32(2));
        let i = iadd!(i, c_u32(1));
        ld_u32!(v_ids, i)
    };
    emit_decision_block(
        &mut body,
        &mut nid,
        DecisionIo {
            t_bool,
            t_u32,
            t_u64,
            t_f32,
            p_uni_u64,
            v_vis,
            id_ext,
            c_zero,
            c_one,
            c_scale,
            c_depth_max,
            c_u64_7,
            c_u64_34,
            c_u32_0: c_u32(0),
            c_u32_1: c_u32(1),
        },
        [ax, ay, az, bx0, by0, bz0, cx0, cy0, cz0],
        px,
        py,
        pixel,
        cluster,
        tri,
    );
    inst(&mut body, 249, &[l_m1]);
    inst(&mut body, 248, &[l_m1]);
    inst(&mut body, 253, &[]);
    inst(&mut body, 56, &[]);

    let mut v = vec![0x0723_0203u32, 0x0001_0000, 0, nid, 0];
    v.extend_from_slice(&pre);
    v.extend_from_slice(&ann);
    v.extend_from_slice(&typ);
    v.extend_from_slice(&body);
    v
}

// ---------------------------------------------------------------------------
// 判定块(SW/HW 逐 op 共享:绕向归一 → 边函数 → top-left → 量化 → pack → 原子写)
// ---------------------------------------------------------------------------

/// 判定块发射所需的模块级 id 面(类型/常量/输出缓冲/ext import)。
struct DecisionIo {
    t_bool: u32,
    t_u32: u32,
    t_u64: u32,
    t_f32: u32,
    p_uni_u64: u32,
    v_vis: u32,
    id_ext: u32,
    c_zero: u32,
    c_one: u32,
    c_scale: u32,
    c_depth_max: u32,
    c_u64_7: u32,
    c_u64_34: u32,
    c_u32_0: u32,
    c_u32_1: u32,
}

/// 发射「area < 0 ⇒ inside ⇒ atomicMax」判定块(两级 OpSelectionMerge;
/// 与 G7.5b 语料判定段逐 op 同构:`area = (bx0−ax)(cy0−ay) − (by0−ay)(cx0−ax)`
/// 等表达式序冻结)。`v` = 9 f32 屏幕坐标(ax,ay,az,bx0,by0,bz0,cx0,cy0,cz0);
/// `px/py` = 采样点(像素中心);`pixel` = 线性像素下标;`cluster`/`tri` = 打包 id。
#[allow(clippy::too_many_arguments)]
fn emit_decision_block(
    body: &mut Vec<u32>,
    nid: &mut u32,
    io: DecisionIo,
    v: [u32; 9],
    px: u32,
    py: u32,
    pixel: u32,
    cluster: u32,
    tri: u32,
) {
    let (t_bool, t_u32, t_u64, t_f32) = (io.t_bool, io.t_u32, io.t_u64, io.t_f32);
    macro_rules! alloc {
        () => {{
            let i = *nid;
            *nid += 1;
            i
        }};
    }
    macro_rules! fmul {
        ($x:expr, $y:expr) => {{
            let r = alloc!();
            inst(body, 133, &[t_f32, r, $x, $y]);
            r
        }};
    }
    macro_rules! fadd {
        ($x:expr, $y:expr) => {{
            let r = alloc!();
            inst(body, 129, &[t_f32, r, $x, $y]);
            r
        }};
    }
    macro_rules! fsub {
        ($x:expr, $y:expr) => {{
            let r = alloc!();
            inst(body, 131, &[t_f32, r, $x, $y]);
            r
        }};
    }
    // 浮点比较/逻辑/选择(判定段算子)。
    macro_rules! flt {
        ($x:expr, $y:expr) => {{
            let r = alloc!();
            inst(body, 184, &[t_bool, r, $x, $y]); // FOrdLessThan
            r
        }};
    }
    macro_rules! fgt {
        ($x:expr, $y:expr) => {{
            let r = alloc!();
            inst(body, 186, &[t_bool, r, $x, $y]); // FOrdGreaterThan
            r
        }};
    }
    macro_rules! feq {
        ($x:expr, $y:expr) => {{
            let r = alloc!();
            inst(body, 180, &[t_bool, r, $x, $y]); // FOrdEqual
            r
        }};
    }
    macro_rules! land {
        ($x:expr, $y:expr) => {{
            let r = alloc!();
            inst(body, 167, &[t_bool, r, $x, $y]); // LogicalAnd
            r
        }};
    }
    macro_rules! lor {
        ($x:expr, $y:expr) => {{
            let r = alloc!();
            inst(body, 166, &[t_bool, r, $x, $y]); // LogicalOr
            r
        }};
    }
    macro_rules! bsel {
        ($c:expr, $a:expr, $b:expr) => {{
            let r = alloc!();
            inst(body, 169, &[t_f32, r, $c, $a, $b]); // OpSelect(f32)
            r
        }};
    }

    let [ax, ay, az, bx0, by0, bz0, cx0, cy0, cz0] = v;
    // area = (bx0−ax)·(cy0−ay) − (by0−ay)·(cx0−ax)。
    let area = {
        let d1 = fsub!(bx0, ax);
        let d2 = fsub!(cy0, ay);
        let d3 = fsub!(by0, ay);
        let d4 = fsub!(cx0, ax);
        let p1 = fmul!(d1, d2);
        let p2 = fmul!(d3, d4);
        fsub!(p1, p2)
    };
    let l_area = alloc!();
    let l_inside = alloc!();
    let l_m_inside = alloc!();
    let l_m_area = alloc!();
    let neg = flt!(area, io.c_zero);
    inst(body, 247, &[l_m_area, 0]);
    inst(body, 250, &[neg, l_area, l_m_area]);
    inst(body, 248, &[l_area]);
    // 绕向归一(交换 b/c):bx=cx0 … cx=bx0 …。
    let (bx, by, bz, cx, cy, cz) = (cx0, cy0, cz0, bx0, by0, bz0);
    // 边函数(正绕向):e_bc = (cx−bx)(py−by) − (cy−by)(px−bx) 等。
    let edge = |body: &mut Vec<u32>,
                nid: &mut u32,
                x1: u32,
                y1: u32,
                x2: u32,
                y2: u32,
                px: u32,
                py: u32|
     -> u32 {
        macro_rules! alloc {
            () => {{
                let i = *nid;
                *nid += 1;
                i
            }};
        }
        macro_rules! fm {
            ($x:expr, $y:expr) => {{
                let r = alloc!();
                inst(body, 133, &[t_f32, r, $x, $y]);
                r
            }};
        }
        macro_rules! fs {
            ($x:expr, $y:expr) => {{
                let r = alloc!();
                inst(body, 131, &[t_f32, r, $x, $y]);
                r
            }};
        }
        let d1 = fs!(x2, x1); // cx − bx
        let d2 = fs!(py, y1); // py − by
        let d3 = fs!(y2, y1); // cy − by
        let d4 = fs!(px, x1); // px − bx
        let p1 = fm!(d1, d2);
        let p2 = fm!(d3, d4);
        fs!(p1, p2)
    };
    let e_bc = edge(body, nid, bx, by, cx, cy, px, py);
    let e_ca = edge(body, nid, cx, cy, ax, ay, px, py);
    let e_ab = edge(body, nid, ax, ay, bx, by, px, py);
    // top-left:d = 边方向;tl ⟺ d.y > 0 || (d.y == 0 && d.x > 0)。
    let tl = |body: &mut Vec<u32>, nid: &mut u32, dx: u32, dy: u32| -> u32 {
        macro_rules! alloc {
            () => {{
                let i = *nid;
                *nid += 1;
                i
            }};
        }
        let g = alloc!();
        inst(body, 186, &[t_bool, g, dy, io.c_zero]); // dy > 0
        let e = alloc!();
        inst(body, 180, &[t_bool, e, dy, io.c_zero]); // dy == 0
        let gx = alloc!();
        inst(body, 186, &[t_bool, gx, dx, io.c_zero]); // dx > 0
        let both = alloc!();
        inst(body, 167, &[t_bool, both, e, gx]);
        let r = alloc!();
        inst(body, 166, &[t_bool, r, g, both]);
        r
    };
    let tl_bc = {
        let dx = fsub!(cx, bx);
        let dy = fsub!(cy, by);
        tl(body, nid, dx, dy)
    };
    let tl_ca = {
        let dx = fsub!(ax, cx);
        let dy = fsub!(ay, cy);
        tl(body, nid, dx, dy)
    };
    let tl_ab = {
        let dx = fsub!(bx, ax);
        let dy = fsub!(by, ay);
        tl(body, nid, dx, dy)
    };
    // inside = ∧(e > 0 || (e == 0 && tl))。
    let in_bc = {
        let g = fgt!(e_bc, io.c_zero);
        let e = feq!(e_bc, io.c_zero);
        let both = land!(e, tl_bc);
        lor!(g, both)
    };
    let in_ca = {
        let g = fgt!(e_ca, io.c_zero);
        let e = feq!(e_ca, io.c_zero);
        let both = land!(e, tl_ca);
        lor!(g, both)
    };
    let in_ab = {
        let g = fgt!(e_ab, io.c_zero);
        let e = feq!(e_ab, io.c_zero);
        let both = land!(e, tl_ab);
        lor!(g, both)
    };
    let inside = {
        let a = land!(in_bc, in_ca);
        land!(a, in_ab)
    };
    inst(body, 247, &[l_m_inside, 0]);
    inst(body, 250, &[inside, l_inside, l_m_inside]);
    inst(body, 248, &[l_inside]);
    // z = (e_bc·az + e_ca·bz + e_ab·cz) / (0.0 − area)(.rx 表达式序)。
    let z = {
        let p1 = fmul!(e_bc, az);
        let p2 = fmul!(e_ca, bz);
        let p3 = fmul!(e_ab, cz);
        let s = fadd!(p1, p2);
        let s = fadd!(s, p3);
        let neg_area = fsub!(io.c_zero, area);
        let r = alloc!();
        inst(body, 136, &[t_f32, r, s, neg_area]); // FDiv
        r
    };
    // z clamp [0,1](.rx 两个 if ⇒ OpSelect 同义)。
    let z = {
        let c = flt!(z, io.c_zero);
        bsel!(c, io.c_zero, z)
    };
    let z = {
        let c = fgt!(z, io.c_one);
        bsel!(c, io.c_one, z)
    };
    // depth = clamp(RoundEven((1−z)·2³⁰), 1, 2³⁰−1)。
    let depth = {
        let o = fsub!(io.c_one, z);
        let s = fmul!(o, io.c_scale);
        let r = alloc!();
        inst(body, 12, &[t_f32, r, io.id_ext, 2, s]); // RoundEven
        let u = alloc!();
        inst(body, 109, &[t_u32, u, r]); // FToU(截断;RoundEven 后整数无损)
        // UMax(depth, 1) / UMin(depth, 2³⁰−1)(OpSelect 整数同义:.rx 两 if 序)。
        let c1 = alloc!();
        inst(body, 176, &[t_bool, c1, u, io.c_u32_1]); // ULessThan(depth,1)
        let m1 = alloc!();
        inst(body, 169, &[t_u32, m1, c1, io.c_u32_1, u]);
        let c2 = alloc!();
        inst(body, 172, &[t_bool, c2, m1, io.c_depth_max]); // UGreaterThan
        let m2 = alloc!();
        inst(body, 169, &[t_u32, m2, c2, io.c_depth_max, m1]);
        m2
    };
    // packed = (u64(depth) << 34) | (u64(cluster) << 7) | u64(tri)。
    let packed = {
        let d64 = alloc!();
        inst(body, 113, &[t_u64, d64, depth]); // UConvert u32→u64
        let c64 = alloc!();
        inst(body, 113, &[t_u64, c64, cluster]);
        let t64 = alloc!();
        inst(body, 113, &[t_u64, t64, tri]);
        let s1 = alloc!();
        inst(body, 196, &[t_u64, s1, d64, io.c_u64_34]); // Shl
        let s2 = alloc!();
        inst(body, 196, &[t_u64, s2, c64, io.c_u64_7]);
        let o1 = alloc!();
        inst(body, 197, &[t_u64, o1, s1, s2]); // BitwiseOr
        let o2 = alloc!();
        inst(body, 197, &[t_u64, o2, o1, t64]);
        o2
    };
    // atomicUMax(&vis[pixel], packed, Device, Relaxed)。
    let addr = alloc!();
    inst(body, 65, &[io.p_uni_u64, addr, io.v_vis, io.c_u32_0, pixel]);
    let ares = alloc!();
    inst(
        body,
        239, // OpAtomicUMax
        &[t_u64, ares, addr, io.c_u32_1, io.c_u32_0, packed],
    );
    inst(body, 249, &[l_m_inside]);
    inst(body, 248, &[l_m_inside]);
    inst(body, 249, &[l_m_area]);
    inst(body, 248, &[l_m_area]);
}

// ---------------------------------------------------------------------------
// HW 图形腿:VS 纯 passthrough + FS 逐字复刻判定
// ---------------------------------------------------------------------------

/// HW 腿 VS SPIR-V(passthrough:loc0 pos vec4f → gl_Position;loc1..3
/// va/vb/vc vec4f 与 loc4 ids vec2u flat 透传)。
pub fn hw_visbuffer_vs_spv() -> Vec<u32> {
    let t_void = 2;
    let t_func = 3;
    let t_f32 = 7;
    let t_u32 = 5;
    let t_vec4f = 10;
    let t_vec2u = 11;
    let p_in_vec4f = 12;
    let p_in_vec2u = 13;
    let p_out_vec4f = 14;
    let p_out_vec2u = 15;
    // 输入变量:16..=20(loc0..4);输出变量:21..=25(Position + loc0..3)。
    let v_in = |l: u32| 16 + l;
    let v_out = |l: u32| 21 + l; // l=0..3 → loc0..3;Position = 26
    let v_pos_out = 26u32;

    let mut pre = Vec::new();
    let mut ann = Vec::new();
    let mut typ = Vec::new();
    inst(&mut pre, 17, &[1]); // Shader
    inst(&mut pre, 14, &[0, 1]);
    let mut ep = vec![0u32, 100]; // Vertex
    ep.extend(words("main"));
    for l in 0..5u32 {
        ep.push(v_in(l));
    }
    ep.push(v_pos_out);
    for l in 0..4u32 {
        ep.push(v_out(l));
    }
    inst(&mut pre, 15, &ep);

    for l in 0..5u32 {
        inst(&mut ann, 71, &[v_in(l), 30, l]); // Location l
    }
    inst(&mut ann, 71, &[v_pos_out, 11, 0]); // BuiltIn Position
    for l in 0..4u32 {
        inst(&mut ann, 71, &[v_out(l), 30, l]);
        inst(&mut ann, 71, &[v_out(l), 14]); // Flat(两阶段一致;u32 强制)
    }

    inst(&mut typ, 19, &[t_void]);
    inst(&mut typ, 33, &[t_func, t_void]);
    inst(&mut typ, 21, &[t_u32, 32, 0]);
    inst(&mut typ, 22, &[t_f32, 32]);
    inst(&mut typ, 23, &[t_vec4f, t_f32, 4]);
    inst(&mut typ, 23, &[t_vec2u, t_u32, 2]);
    inst(&mut typ, 32, &[p_in_vec4f, 1, t_vec4f]);
    inst(&mut typ, 32, &[p_in_vec2u, 1, t_vec2u]);
    inst(&mut typ, 32, &[p_out_vec4f, 3, t_vec4f]);
    inst(&mut typ, 32, &[p_out_vec2u, 3, t_vec2u]);
    for l in 0..4u32 {
        inst(&mut typ, 59, &[p_in_vec4f, v_in(l), 1]);
    }
    inst(&mut typ, 59, &[p_in_vec2u, v_in(4), 1]);
    inst(&mut typ, 59, &[p_out_vec4f, v_pos_out, 3]);
    for l in 0..3u32 {
        inst(&mut typ, 59, &[p_out_vec4f, v_out(l), 3]);
    }
    inst(&mut typ, 59, &[p_out_vec2u, v_out(3), 3]);

    let mut body = Vec::new();
    inst(&mut body, 54, &[t_void, 100, 0, t_func]);
    inst(&mut body, 248, &[101]);
    let mut nid = 102u32;
    // gl_Position = pos;va/vb/vc/ids 透传。
    for l in 0..5u32 {
        let (ty, pv, vo) = if l == 4 {
            (t_vec2u, p_in_vec2u, p_out_vec2u)
        } else {
            (t_vec4f, p_in_vec4f, p_out_vec4f)
        };
        let dst = if l == 0 {
            v_pos_out // loc0 输入 pos → Position 输出
        } else {
            v_out(l - 1) // loc1..4 输入 → loc0..3 输出
        };
        let r = nid;
        nid += 1;
        inst(&mut body, 61, &[ty, r, v_in(l)]);
        let _ = pv;
        let _ = vo;
        inst(&mut body, 62, &[dst, r]);
    }
    inst(&mut body, 253, &[]);
    inst(&mut body, 56, &[]);

    let mut v = vec![0x0723_0203u32, 0x0001_0000, 0, nid, 0];
    v.extend_from_slice(&pre);
    v.extend_from_slice(&ann);
    v.extend_from_slice(&typ);
    v.extend_from_slice(&body);
    v
}

/// HW 腿 FS SPIR-V(frag_coord 采样 + flat va/vb/vc/ids 逐字复刻 SW 判定;
/// 绑定 0 = vis u64[W·H];push const = width;loc0 color 占位写 (0,0,0,1))。
pub fn hw_visbuffer_fs_spv() -> Vec<u32> {
    let id_ext = 1;
    let t_void = 2;
    let t_func = 3;
    let t_bool = 4;
    let t_u32 = 5;
    let t_u64 = 6;
    let t_f32 = 7;
    let t_vec4f = 10;
    let t_vec2u = 11;
    let p_in_vec4f = 12;
    let p_in_vec2u = 13;
    let p_out_vec4f = 14;
    let v_frag = 15; // frag_coord builtin
    let v_in = |l: u32| 16 + l; // loc0..2 va..vc(vec4f),loc3 ids(vec2u)
    let v_color = 20;
    let t_rt_u64 = 21;
    let t_st_u64 = 22;
    let p_uni_st_u64 = 23;
    let p_uni_u64 = 24;
    let v_vis = 25;
    let t_pc = 26;
    let p_pc = 27;
    let v_pc = 28;
    let p_pc_u32 = 29;
    let c_u32_0 = 30;
    let c_u32_1 = 31;
    let c_depth_max = 32;
    let c_u64_7 = 33;
    let c_u64_34 = 34;
    let c_zero = 35;
    let c_one = 36;
    let c_scale = 37;

    let mut pre = Vec::new();
    let mut ann = Vec::new();
    let mut typ = Vec::new();
    inst(&mut pre, 17, &[1]); // Shader
    inst(&mut pre, 17, &[11]); // Int64
    inst(&mut pre, 17, &[12]); // Int64Atomics
    let mut ext = vec![id_ext];
    ext.extend(words("GLSL.std.450"));
    inst(&mut pre, 11, &ext);
    inst(&mut pre, 14, &[0, 1]);
    let mut ep = vec![4u32, 100]; // Fragment
    ep.extend(words("main"));
    ep.extend_from_slice(&[v_frag, v_in(0), v_in(1), v_in(2), v_in(3), v_color]);
    inst(&mut pre, 15, &ep);
    inst(&mut pre, 16, &[100, 7]); // OriginUpperLeft

    inst(&mut ann, 71, &[v_frag, 11, 15]); // BuiltIn FragCoord
    for l in 0..3u32 {
        inst(&mut ann, 71, &[v_in(l), 30, l]);
        inst(&mut ann, 71, &[v_in(l), 14]); // Flat
    }
    inst(&mut ann, 71, &[v_in(3), 30, 3]);
    inst(&mut ann, 71, &[v_in(3), 14]);
    inst(&mut ann, 71, &[v_color, 30, 0]); // Location 0 输出
    inst(&mut ann, 71, &[v_vis, 34, 0]);
    inst(&mut ann, 71, &[v_vis, 33, 0]);
    inst(&mut ann, 71, &[t_st_u64, 3]); // BufferBlock
    inst(&mut ann, 72, &[t_st_u64, 0, 35, 0]);
    inst(&mut ann, 71, &[t_rt_u64, 6, 8]); // ArrayStride 8
    inst(&mut ann, 71, &[t_pc, 2]);
    inst(&mut ann, 72, &[t_pc, 0, 35, 0]);

    inst(&mut typ, 19, &[t_void]);
    inst(&mut typ, 33, &[t_func, t_void]);
    inst(&mut typ, 20, &[t_bool]);
    inst(&mut typ, 21, &[t_u32, 32, 0]);
    inst(&mut typ, 21, &[t_u64, 64, 0]);
    inst(&mut typ, 22, &[t_f32, 32]);
    inst(&mut typ, 23, &[t_vec4f, t_f32, 4]);
    inst(&mut typ, 23, &[t_vec2u, t_u32, 2]);
    inst(&mut typ, 32, &[p_in_vec4f, 1, t_vec4f]);
    inst(&mut typ, 32, &[p_in_vec2u, 1, t_vec2u]);
    inst(&mut typ, 32, &[p_out_vec4f, 3, t_vec4f]);
    inst(&mut typ, 59, &[p_in_vec4f, v_frag, 1]);
    for l in 0..3u32 {
        inst(&mut typ, 59, &[p_in_vec4f, v_in(l), 1]);
    }
    inst(&mut typ, 59, &[p_in_vec2u, v_in(3), 1]);
    inst(&mut typ, 59, &[p_out_vec4f, v_color, 3]);
    inst(&mut typ, 29, &[t_rt_u64, t_u64]);
    inst(&mut typ, 30, &[t_st_u64, t_rt_u64]);
    inst(&mut typ, 32, &[p_uni_st_u64, 2, t_st_u64]);
    inst(&mut typ, 32, &[p_uni_u64, 2, t_u64]);
    inst(&mut typ, 59, &[p_uni_st_u64, v_vis, 2]);
    inst(&mut typ, 30, &[t_pc, t_u32]);
    inst(&mut typ, 32, &[p_pc, 9, t_pc]);
    inst(&mut typ, 59, &[p_pc, v_pc, 9]);
    inst(&mut typ, 32, &[p_pc_u32, 9, t_u32]);
    inst(&mut typ, 43, &[t_u32, c_u32_0, 0]);
    inst(&mut typ, 43, &[t_u32, c_u32_1, 1]);
    inst(&mut typ, 43, &[t_u32, c_depth_max, (1 << 30) - 1]);
    inst(&mut typ, 43, &[t_u64, c_u64_7, 7, 0]);
    inst(&mut typ, 43, &[t_u64, c_u64_34, 34, 0]);
    inst(&mut typ, 43, &[t_f32, c_zero, 0.0f32.to_bits()]);
    inst(&mut typ, 43, &[t_f32, c_one, 1.0f32.to_bits()]);
    inst(&mut typ, 43, &[t_f32, c_scale, DEPTH30_SCALE_F32_BITS]);

    let mut body = Vec::new();
    let mut nid = 100u32;
    macro_rules! alloc {
        () => {{
            let i = nid;
            nid += 1;
            i
        }};
    }
    inst(&mut body, 54, &[t_void, alloc!(), 0, t_func]); // fn 100
    let l_entry = alloc!();
    inst(&mut body, 248, &[l_entry]);
    // frag_coord 采样点(pixel center;extract x/y)。
    let fc = alloc!();
    inst(&mut body, 61, &[t_vec4f, fc, v_frag]);
    let px = alloc!();
    inst(&mut body, 81, &[t_f32, px, fc, 0]);
    let py = alloc!();
    inst(&mut body, 81, &[t_f32, py, fc, 1]);
    // flat 输入分量展开(va/vb/vc xyz;ids xy)。
    let comp = |var: u32, k: u32, body: &mut Vec<u32>, nid: &mut u32| -> u32 {
        let r = *nid;
        *nid += 1;
        inst(body, 61, &[t_vec4f, r, var]);
        let e = *nid;
        *nid += 1;
        inst(body, 81, &[t_f32, e, r, k]);
        e
    };
    let va = [
        comp(v_in(0), 0, &mut body, &mut nid),
        comp(v_in(0), 1, &mut body, &mut nid),
        comp(v_in(0), 2, &mut body, &mut nid),
    ];
    let vb = [
        comp(v_in(1), 0, &mut body, &mut nid),
        comp(v_in(1), 1, &mut body, &mut nid),
        comp(v_in(1), 2, &mut body, &mut nid),
    ];
    let vc = [
        comp(v_in(2), 0, &mut body, &mut nid),
        comp(v_in(2), 1, &mut body, &mut nid),
        comp(v_in(2), 2, &mut body, &mut nid),
    ];
    let ids_v = nid;
    nid += 1;
    inst(&mut body, 61, &[t_vec2u, ids_v, v_in(3)]);
    let cluster = nid;
    nid += 1;
    inst(&mut body, 81, &[t_u32, cluster, ids_v, 0]);
    let tri = nid;
    nid += 1;
    inst(&mut body, 81, &[t_u32, tri, ids_v, 1]);
    // pixel = FToU(py) * width + FToU(px)。
    let width = {
        let (a, r) = (nid, nid + 1);
        nid += 2;
        inst(&mut body, 65, &[p_pc_u32, a, v_pc, c_u32_0]);
        inst(&mut body, 61, &[t_u32, r, a]);
        r
    };
    let px_u = nid;
    nid += 1;
    inst(&mut body, 109, &[t_u32, px_u, px]); // FToU(k+0.5)=k(截断)
    let py_u = nid;
    nid += 1;
    inst(&mut body, 109, &[t_u32, py_u, py]);
    let pixel = {
        let p = nid;
        nid += 1;
        inst(&mut body, 132, &[t_u32, p, py_u, width]);
        let s = nid;
        nid += 1;
        inst(&mut body, 128, &[t_u32, s, p, px_u]);
        s
    };
    emit_decision_block(
        &mut body,
        &mut nid,
        DecisionIo {
            t_bool,
            t_u32,
            t_u64,
            t_f32,
            p_uni_u64,
            v_vis,
            id_ext,
            c_zero,
            c_one,
            c_scale,
            c_depth_max,
            c_u64_7,
            c_u64_34,
            c_u32_0,
            c_u32_1,
        },
        [va[0], va[1], va[2], vb[0], vb[1], vb[2], vc[0], vc[1], vc[2]],
        px,
        py,
        pixel,
        cluster,
        tri,
    );
    // color = (0,0,0,1) 占位(dummy attachment;VisBuffer 走 SSBO)。
    let col = nid;
    nid += 1;
    inst(&mut body, 80, &[t_vec4f, col, c_zero, c_zero, c_zero, c_one]);
    inst(&mut body, 62, &[v_color, col]);
    inst(&mut body, 253, &[]);
    inst(&mut body, 56, &[]);

    let mut v = vec![0x0723_0203u32, 0x0001_0000, 0, nid, 0];
    v.extend_from_slice(&pre);
    v.extend_from_slice(&ann);
    v.extend_from_slice(&typ);
    v.extend_from_slice(&body);
    v
}

// ---------------------------------------------------------------------------
// 单测
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn walk(spv: &[u32]) -> usize {
        let mut i = 5usize;
        let mut n = 0;
        while i < spv.len() {
            let wc = (spv[i] >> 16) as usize;
            assert!(wc >= 1 && i + wc <= spv.len(), "偏移 {i} 指令流破坏");
            i += wc;
            n += 1;
        }
        assert_eq!(i, spv.len());
        n
    }

    fn caps(spv: &[u32]) -> Vec<u32> {
        let mut out = Vec::new();
        let mut i = 5usize;
        while i < spv.len() {
            let wc = (spv[i] >> 16) as usize;
            if (spv[i] & 0xffff) == 17 {
                out.push(spv[i + 1]);
            }
            i += wc;
        }
        out
    }

    fn has_op(spv: &[u32], op: u32) -> bool {
        let mut i = 5usize;
        while i < spv.len() {
            let wc = (spv[i] >> 16) as usize;
            if (spv[i] & 0xffff) == op {
                return true;
            }
            i += wc;
        }
        false
    }

    //@ spec: RXS-0352
    #[test]
    fn sw_spv_structure_and_capabilities() {
        let spv = sw_visbuffer_u64_spv();
        assert_eq!(spv[0], 0x0723_0203);
        assert_eq!(spv[1], 0x0001_0000, "SPIR-V 1.0(BufferBlock+Uniform 形态)");
        walk(&spv);
        // capability:Shader + Int64 + Int64Atomics(W2 链,与 rurixc 同值)。
        assert_eq!(caps(&spv), vec![1, 11, 12]);
        // 原子写在位(u64 OpAtomicUMax)与 RoundEven ext inst 在位。
        assert!(has_op(&spv, 239), "OpAtomicUMax 必须在位");
        assert!(has_op(&spv, 12), "OpExtInst(RoundEven)必须在位");
        // 双跑构建逐位一致。
        assert_eq!(spv, sw_visbuffer_u64_spv());
    }

    //@ spec: RXS-0352
    #[test]
    fn hw_vs_fs_spv_structure() {
        let vs = hw_visbuffer_vs_spv();
        let fs = hw_visbuffer_fs_spv();
        walk(&vs);
        walk(&fs);
        assert_eq!(caps(&vs), vec![1], "VS 仅 Shader");
        assert_eq!(caps(&fs), vec![1, 11, 12], "FS Shader+Int64+Int64Atomics");
        assert!(has_op(&fs, 239), "FS 原子写在位");
        // VS entry 模型 Vertex(0);FS Fragment(4) + OriginUpperLeft(7)。
        let entry_model = |spv: &[u32]| {
            let mut i = 5usize;
            while i < spv.len() {
                let wc = (spv[i] >> 16) as usize;
                if (spv[i] & 0xffff) == 15 {
                    return Some(spv[i + 1]);
                }
                i += wc;
            }
            None
        };
        assert_eq!(entry_model(&vs), Some(0), "VS Vertex 模型");
        assert_eq!(entry_model(&fs), Some(4), "FS Fragment 模型");
        let mut fs_origin = false;
        let mut i = 5usize;
        while i < fs.len() {
            let wc = (fs[i] >> 16) as usize;
            if (fs[i] & 0xffff) == 16 && fs[i + 2] == 7 {
                fs_origin = true;
            }
            i += wc;
        }
        assert!(fs_origin, "FS OriginUpperLeft 声明(Vulkan 必须)");
        // 双跑逐位一致。
        assert_eq!(vs, hw_visbuffer_vs_spv());
        assert_eq!(fs, hw_visbuffer_fs_spv());
    }

    //@ spec: RXS-0352
    #[test]
    fn depth30_scale_constant_matches_host() {
        // (2³⁰−1) as f32 最近舍入 = 2³⁰(0x4E800000)——与 host `DEPTH30_MAX as
        // f32`(quantize_depth30)位级一致(跨端对拍前提)。
        assert_eq!(
            DEPTH30_SCALE_F32_BITS,
            (((1u32 << 30) - 1) as f32).to_bits(),
            "DEPTH30 scale 位型"
        );
        assert_eq!(DEPTH30_SCALE_F32_BITS, 1073741824.0f32.to_bits());
    }
}
