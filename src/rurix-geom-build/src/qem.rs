//! QEM(Quadric Error Metrics)组内简化器——G31+ #66 加性第二实现。
//!
//! 纪律面(TODO #66 字面「可先做参照器臂……禁静默替换事实源」):
//! - **默认 [`crate::build_dag`] 0-byte 不动**(最短边贪心 = 既有事实源;
//!   m90 DAG digest golden 锚不漂移);
//! - 本模块经 [`crate::build_dag_kind`]/[`crate::SimplifyKind::Qem`] 加性
//!   暴露,生产簇包 bake(rurix-asset g31_cluster_lod_bake)显式选用;
//! - 对照数据(层数/根层三角/达成率)由调用方登记,替换默认走 #66 立项。
//!
//! 算法(Garland–Heckbert 1997 位置 QEM;调研报告 §3 要点逐条):
//! - 逐顶点 4×4 二次型 `Q = Σ 面平面 K_p`(面积加权;f64 十系数对称存储);
//! - 边坍塌候选按 `v̄ᵀ(Qa+Qb)v̄` 升序(最优位置 3×3 求解,奇异回退
//!   {a, b, mid} 最小代价;f64 求解 → f32 输出);
//! - **裂缝保护与既有实现逐字同律**:双锁边永不坍塌、单锁新位置 = 锁定端
//!   逐位保持、共边面含组边界边禁坍塌——组边界折线粗细两侧逐位一致;
//! - **fold-over 拒绝**(调研 §3:meshopt `hasTriangleFlip` 同思路):坍塌
//!   后存活邻接面新旧法线 `dot ≤ FOLD_DOT_MIN` 或退化 → 拒绝该候选;
//! - **stuck 出口**:候选耗尽即止(不死循环);简化率经返回值如实上报,
//!   调用方按 [`STUCK_RETAIN_RATIO`] 判卡(计数登记,不虚报误差);
//! - **误差口径与既有实现同源**(单调性/保守性证明 0-语义漂移):QEM 只
//!   用于**选边与定新位置**(质量),上报误差仍为「顶点实际位移的保守累计
//!   上界」`err[v] = max(err_keep + |new−old_keep|, err_drop + |new−old_drop|)`
//!   ——cut 判据消费的是世界空间最大偏移上界,非二次型残差。
//!
//! 蒙皮资产注意(调用方裁决):QEM 移动内部顶点 ⇒ 粗簇顶点位置不再命中
//! 叶层反查表 ⇒ 蒙皮元数据推导(RXS-0345 §3.3)不可得。骨骼资产须走
//! 既有端点保持简化器(`build_asset_dag` 恒 ShortestEdge,见 dag.rs)。

use crate::dag::SubMesh;
use crate::vecmath::vdist;

/// stuck 判定阈(调研 §3:meshopt `simplify_threshold = 0.85`——留存
/// 超此比例视为「简化卡住」;仅统计登记面,不改误差语义)。
pub const STUCK_RETAIN_RATIO: f64 = 0.85;

/// fold-over 拒绝阈:坍塌后面法线与旧法线点积下限(> 0 防连续近 90°
/// 累计翻转——调研 §3 meshopt 教训;0.05 = 保守小正值)。
const FOLD_DOT_MIN: f32 = 0.05;

/// 对称 4×4 二次型(十系数 f64;`K = p·pᵀ`,p = (a,b,c,d) 单位法线平面)。
#[derive(Debug, Clone, Copy, Default)]
struct Quadric {
    a2: f64,
    ab: f64,
    ac: f64,
    ad: f64,
    b2: f64,
    bc: f64,
    bd: f64,
    c2: f64,
    cd: f64,
    d2: f64,
}

impl Quadric {
    fn from_plane(a: f64, b: f64, c: f64, d: f64, w: f64) -> Self {
        Self {
            a2: w * a * a,
            ab: w * a * b,
            ac: w * a * c,
            ad: w * a * d,
            b2: w * b * b,
            bc: w * b * c,
            bd: w * b * d,
            c2: w * c * c,
            cd: w * c * d,
            d2: w * d * d,
        }
    }

    fn add(&mut self, o: &Quadric) {
        self.a2 += o.a2;
        self.ab += o.ab;
        self.ac += o.ac;
        self.ad += o.ad;
        self.b2 += o.b2;
        self.bc += o.bc;
        self.bd += o.bd;
        self.c2 += o.c2;
        self.cd += o.cd;
        self.d2 += o.d2;
    }

    fn sum(&self, o: &Quadric) -> Quadric {
        let mut s = *self;
        s.add(o);
        s
    }

    /// 二次型误差 `vᵀQv`(v 齐次 (x,y,z,1);数值下限 0——浮点抵消防负)。
    fn error(&self, p: [f32; 3]) -> f64 {
        let (x, y, z) = (p[0] as f64, p[1] as f64, p[2] as f64);
        let e = self.a2 * x * x
            + 2.0 * self.ab * x * y
            + 2.0 * self.ac * x * z
            + 2.0 * self.ad * x
            + self.b2 * y * y
            + 2.0 * self.bc * y * z
            + 2.0 * self.bd * y
            + self.c2 * z * z
            + 2.0 * self.cd * z
            + self.d2;
        e.max(0.0)
    }

    /// 最优位置(∇(vᵀQv) = 0 的 3×3 线性系统;克莱姆法则,|det| 过小 =
    /// 奇异(平面/直线退化)返回 None——调用方回退候选集)。
    fn optimal(&self) -> Option<[f32; 3]> {
        let (a11, a12, a13) = (self.a2, self.ab, self.ac);
        let (a22, a23) = (self.b2, self.bc);
        let a33 = self.c2;
        let (b1, b2, b3) = (-self.ad, -self.bd, -self.cd);
        let det = a11 * (a22 * a33 - a23 * a23) - a12 * (a12 * a33 - a23 * a13)
            + a13 * (a12 * a23 - a22 * a13);
        // 尺度感知奇异阈(系数量级 ~L⁴,det ~L⁶;取迹立方的相对阈)。
        let scale = (a11 + a22 + a33).abs().max(1e-30);
        if det.abs() <= 1e-12 * scale * scale * scale {
            return None;
        }
        let inv = 1.0 / det;
        let x = (b1 * (a22 * a33 - a23 * a23) - a12 * (b2 * a33 - a23 * b3)
            + a13 * (b2 * a23 - a22 * b3))
            * inv;
        let y = (a11 * (b2 * a33 - a23 * b3) - b1 * (a12 * a33 - a13 * a23)
            + a13 * (a12 * b3 - b2 * a13))
            * inv;
        let z = (a11 * (a22 * b3 - b2 * a23) - a12 * (a12 * b3 - b2 * a13)
            + b1 * (a12 * a23 - a22 * a13))
            * inv;
        let out = [x as f32, y as f32, z as f32];
        if out.iter().all(|v| v.is_finite()) {
            Some(out)
        } else {
            None
        }
    }
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// 面平面(单位法线 + d)与 2×面积;退化面返回 None(不入 quadric)。
fn face_plane(p0: [f32; 3], p1: [f32; 3], p2: [f32; 3]) -> Option<([f64; 4], f64)> {
    let a = [p0[0] as f64, p0[1] as f64, p0[2] as f64];
    let b = [p1[0] as f64, p1[1] as f64, p1[2] as f64];
    let c = [p2[0] as f64, p2[1] as f64, p2[2] as f64];
    let n = cross(
        [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
        [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
    );
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if len <= 1e-30 {
        return None;
    }
    let un = [n[0] / len, n[1] / len, n[2] / len];
    let d = -(un[0] * a[0] + un[1] * a[1] + un[2] * a[2]);
    Some(([un[0], un[1], un[2], d], len))
}

/// f32 面法线(fold-over 检测用;退化返回 None——退化面按拒绝处理)。
fn face_normal_f32(p0: [f32; 3], p1: [f32; 3], p2: [f32; 3]) -> Option<[f32; 3]> {
    let e1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
    let e2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
    let n = [
        e1[1] * e2[2] - e1[2] * e2[1],
        e1[2] * e2[0] - e1[0] * e2[2],
        e1[0] * e2[1] - e1[1] * e2[0],
    ];
    let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if l <= 1e-20 {
        return None;
    }
    Some([n[0] / l, n[1] / l, n[2] / l])
}

/// QEM 组内简化(接口与 [`crate::dag`] `simplify_group` 逐位同形:输入组
/// 子网格 + 目标三角数,输出简化子网格 + **顶点位移保守累计上界**误差)。
///
/// 裂缝保护与既有实现同律(锁定端逐位保持/双锁禁/边界面禁);新增
/// fold-over 拒绝与 QEM 最优内点。整组塌光退回原网格(误差 0 = 无进展)。
pub(crate) fn simplify_group_qem(sub: &SubMesh, target: usize) -> (SubMesh, f32) {
    use std::cmp::Ordering;
    use std::collections::BinaryHeap;

    #[derive(Clone, Copy, PartialEq)]
    struct Cand {
        cost: f64,
        a: u32,
        b: u32,
        /// 候选生成时的坍塌纪元(顶点位置/拓扑变化后失效——懒惰失效)。
        epoch_a: u32,
        epoch_b: u32,
    }
    impl Eq for Cand {}
    impl Ord for Cand {
        fn cmp(&self, o: &Self) -> Ordering {
            // 反转 → 最小堆(最低代价优先);id 决胜全确定。
            o.cost
                .total_cmp(&self.cost)
                .then_with(|| o.a.cmp(&self.a))
                .then_with(|| o.b.cmp(&self.b))
        }
    }
    impl PartialOrd for Cand {
        fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
            Some(self.cmp(o))
        }
    }

    let nv = sub.positions.len();
    let input_tris = sub.tris.len();
    let mut positions = sub.positions.clone();
    let mut tris = sub.tris.clone();
    let mut vert_err = vec![0.0f32; nv];
    let mut alive_v = vec![true; nv];
    let mut alive_f = vec![true; tris.len()];
    let mut epoch = vec![0u32; nv];
    let mut adj: Vec<Vec<u32>> = vec![Vec::new(); nv];
    for (f, t) in tris.iter().enumerate() {
        for &v in t {
            adj[v as usize].push(f as u32);
        }
    }
    // 逐顶点 quadric(面积权重 area/3 分给三顶点——Garland 面积加权惯例)。
    let mut quadric = vec![Quadric::default(); nv];
    for t in tris.iter() {
        if let Some((p, area2)) = face_plane(
            positions[t[0] as usize],
            positions[t[1] as usize],
            positions[t[2] as usize],
        ) {
            let w = area2 * 0.5 / 3.0;
            let k = Quadric::from_plane(p[0], p[1], p[2], p[3], w);
            for &v in t {
                quadric[v as usize].add(&k);
            }
        }
    }

    // 候选评估:合法性(锁/边界面) + 新位置 + QEM 代价。返回 None = 永久非法
    //(双锁/边界面);Some = 可入堆。
    let eval = |a: usize,
                b: usize,
                positions: &[[f32; 3]],
                quadric: &[Quadric]|
     -> Option<(f64, [f32; 3])> {
        let q = quadric[a].sum(&quadric[b]);
        let cand_pos: [f32; 3] = match (sub.locked[a], sub.locked[b]) {
            (true, true) => return None,
            (true, false) => positions[a],
            (false, true) => positions[b],
            (false, false) => {
                let mid = [
                    (positions[a][0] + positions[b][0]) * 0.5,
                    (positions[a][1] + positions[b][1]) * 0.5,
                    (positions[a][2] + positions[b][2]) * 0.5,
                ];
                match q.optimal() {
                    Some(opt) => {
                        // 最优点与回退候选集取最小代价(奇异/漂移防护:最优点
                        // 若劣于端点/中点则不用——数值稳健)。
                        let mut best = (q.error(opt), opt);
                        for c in [positions[a], positions[b], mid] {
                            let e = q.error(c);
                            if e < best.0 {
                                best = (e, c);
                            }
                        }
                        best.1
                    }
                    None => {
                        let mut best = (q.error(positions[a]), positions[a]);
                        for c in [positions[b], mid] {
                            let e = q.error(c);
                            if e < best.0 {
                                best = (e, c);
                            }
                        }
                        best.1
                    }
                }
            }
        };
        Some((q.error(cand_pos), cand_pos))
    };

    let mut heap: BinaryHeap<Cand> = BinaryHeap::new();
    let seed_edges = |heap: &mut BinaryHeap<Cand>,
                          tris: &[[u32; 3]],
                          f: usize,
                          positions: &[[f32; 3]],
                          quadric: &[Quadric],
                          epoch: &[u32]| {
        let t = tris[f];
        for e in 0..3 {
            let (a, b) = (t[e].min(t[(e + 1) % 3]), t[e].max(t[(e + 1) % 3]));
            if a == b {
                continue;
            }
            if let Some((cost, _)) = eval(a as usize, b as usize, positions, quadric) {
                heap.push(Cand {
                    cost,
                    a,
                    b,
                    epoch_a: epoch[a as usize],
                    epoch_b: epoch[b as usize],
                });
            }
        }
    };
    for f in 0..tris.len() {
        seed_edges(&mut heap, &tris, f, &positions, &quadric, &epoch);
    }

    let mut alive_count = tris.len();
    'outer: while alive_count > target {
        // 取合法最低代价候选(懒惰失效:纪元不符 = 陈旧,丢弃)。
        let (keep, drop, new_pos) = loop {
            let Some(c) = heap.pop() else { break 'outer };
            let (a, b) = (c.a as usize, c.b as usize);
            if !alive_v[a]
                || !alive_v[b]
                || c.epoch_a != epoch[a]
                || c.epoch_b != epoch[b]
            {
                continue; // 陈旧候选
            }
            // 共存活面 + 边界面禁令(裂缝保护第二半,与既有实现逐字同律)。
            let mut shares_face = false;
            let mut touches_boundary = false;
            for &f in &adj[a] {
                let f = f as usize;
                if alive_f[f] && tris[f].contains(&(b as u32)) {
                    shares_face = true;
                    if sub.face_on_boundary[f] {
                        touches_boundary = true;
                        break;
                    }
                }
            }
            if !shares_face || touches_boundary {
                continue;
            }
            let Some((_, new_pos)) = eval(a, b, &positions, &quadric) else {
                continue;
            };
            // keep/drop 方向:锁定端保留;双开取小号端(确定性,与既有同律)。
            let (keep, drop) = match (sub.locked[a], sub.locked[b]) {
                (true, true) => continue,
                (true, false) => (a, b),
                (false, true) => (b, a),
                (false, false) => {
                    if a < b {
                        (a, b)
                    } else {
                        (b, a)
                    }
                }
            };
            // fold-over 拒绝:坍塌后存活面(不含消亡面)法线翻转/退化即拒。
            let mut folds = false;
            'fold: for &vv in &[keep, drop] {
                for &f in &adj[vv] {
                    let f = f as usize;
                    if !alive_f[f] {
                        continue;
                    }
                    let t = tris[f];
                    // 消亡面(同含 keep 与 drop)跳过。
                    if t.contains(&(keep as u32)) && t.contains(&(drop as u32)) {
                        continue;
                    }
                    let old = [
                        positions[t[0] as usize],
                        positions[t[1] as usize],
                        positions[t[2] as usize],
                    ];
                    let Some(n_old) = face_normal_f32(old[0], old[1], old[2]) else {
                        continue; // 已退化面不参与判定
                    };
                    let pick = |v: u32| -> [f32; 3] {
                        if v as usize == keep || v as usize == drop {
                            new_pos
                        } else {
                            positions[v as usize]
                        }
                    };
                    let Some(n_new) = face_normal_f32(pick(t[0]), pick(t[1]), pick(t[2]))
                    else {
                        folds = true; // 坍塌产生退化面 → 拒
                        break 'fold;
                    };
                    let dot = n_old[0] * n_new[0] + n_old[1] * n_new[1] + n_old[2] * n_new[2];
                    if dot <= FOLD_DOT_MIN {
                        folds = true;
                        break 'fold;
                    }
                }
            }
            if folds {
                continue;
            }
            break (keep, drop, new_pos);
        };

        // 误差累计(保守上界,与既有实现同口径——单调性证明 0-语义漂移)。
        let d_keep = vdist(new_pos, positions[keep]);
        let d_drop = vdist(new_pos, positions[drop]);
        let merged_err = (vert_err[keep] + d_keep).max(vert_err[drop] + d_drop);
        vert_err[keep] = merged_err;
        // 执行坍塌:drop 面转 keep、quadric 合并、位置更新、纪元推进。
        let q_drop = quadric[drop];
        quadric[keep].add(&q_drop);
        positions[keep] = new_pos;
        epoch[keep] = epoch[keep].wrapping_add(1);
        let moved = std::mem::take(&mut adj[drop]);
        for &f in &moved {
            let f = f as usize;
            if !alive_f[f] {
                continue;
            }
            for v in &mut tris[f] {
                if *v == drop as u32 {
                    *v = keep as u32;
                }
            }
            adj[keep].push(f as u32);
        }
        alive_v[drop] = false;
        // 退化面消亡 + 重复面去重(与既有实现同律)。
        let affected = adj[keep].clone();
        for &f in &affected {
            let f = f as usize;
            if !alive_f[f] {
                continue;
            }
            let t = tris[f];
            let degenerate = t[0] == t[1] || t[1] == t[2] || t[0] == t[2];
            let dup = !degenerate
                && affected.iter().any(|&g| {
                    let g = g as usize;
                    g < f && alive_f[g] && same_tri_set(tris[g], t)
                });
            if degenerate || dup {
                alive_f[f] = false;
                alive_count -= 1;
            }
        }
        // 邻域候选再播种(位置/quadric 已变;纪元过滤旧候选)。
        let new_edges = adj[keep].clone();
        for &f in &new_edges {
            if alive_f[f as usize] {
                seed_edges(&mut heap, &tris, f as usize, &positions, &quadric, &epoch);
            }
        }
    }

    // 压缩输出;整组塌光退回原网格(粗层不得出洞,误差 0 = 无进展)。
    let mut remap = vec![u32::MAX; nv];
    let mut out_pos = Vec::new();
    let mut out_locked = Vec::new();
    for v in 0..nv {
        if alive_v[v] {
            remap[v] = out_pos.len() as u32;
            out_pos.push(positions[v]);
            out_locked.push(sub.locked[v]);
        }
    }
    let mut out_tris = Vec::new();
    for (f, t) in tris.iter().enumerate() {
        if alive_f[f] {
            out_tris.push(t.map(|v| remap[v as usize]));
        }
    }
    if out_tris.is_empty() {
        return (sub.clone(), 0.0);
    }
    let retain = out_tris.len() as f64 / input_tris.max(1) as f64;
    if retain > STUCK_RETAIN_RATIO {
        // stuck 登记面(统计不改语义;误差如实上报,不虚报 MAX 哨兵)。
        crate::qem::note_stuck();
    }
    let max_err = vert_err
        .iter()
        .enumerate()
        .filter(|(v, _)| alive_v[*v])
        .map(|(_, &e)| e)
        .fold(0.0f32, f32::max);
    let face_on_boundary = vec![false; out_tris.len()];
    (
        SubMesh {
            positions: out_pos,
            tris: out_tris,
            locked: out_locked,
            face_on_boundary,
        },
        max_err,
    )
}

/// 两三角形顶点集合相等(绕序无关;与 dag.rs 同名私有函数同式)。
fn same_tri_set(a: [u32; 3], b: [u32; 3]) -> bool {
    a.iter().all(|v| b.contains(v))
}

/// 自由网格 QEM 简化公共入口(G31+ #67 HLOD 质量烘焙消费面:跨 Component
/// 合并后的整块简化——无组边界锁、无边界面禁令,fold-over 拒绝与最优位置
/// 求解全套生效)。返回 (顶点, 索引, 位移保守上界)。
///
/// 确定性:同输入同输出(与组内简化同一实现路径);输入应已按位置焊接
/// (调用方保证——未焊接的重复位置顶点会被视为孤立,简化自由度受限)。
pub fn simplify_free_mesh(
    positions: &[[f32; 3]],
    indices: &[u32],
    target_tris: usize,
) -> (Vec<[f32; 3]>, Vec<u32>, f32) {
    assert!(indices.len().is_multiple_of(3), "索引数须为 3 的倍数");
    let tris: Vec<[u32; 3]> = indices
        .chunks_exact(3)
        .map(|c| [c[0], c[1], c[2]])
        .collect();
    let sub = SubMesh {
        positions: positions.to_vec(),
        locked: vec![false; positions.len()],
        face_on_boundary: vec![false; tris.len()],
        tris,
    };
    let (out, err) = simplify_group_qem(&sub, target_tris.max(1));
    let mut flat = Vec::with_capacity(out.tris.len() * 3);
    for t in &out.tris {
        flat.extend_from_slice(t);
    }
    (out.positions, flat, err)
}

/// stuck 组计数(进程级统计面;跨线程可见——块间并行 bake 各线程累加,
/// `take_stuck_count` 读后清零。仅登记用途,不参与任何判定语义)。
static STUCK_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn note_stuck() {
    STUCK_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

/// 读取并清零 stuck 组计数(调用方在 build 后取,登记进对照 evidence)。
pub fn take_stuck_count() -> u64 {
    STUCK_COUNT.swap(0, std::sync::atomic::Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::TriMesh;

    fn open_submesh(mesh: &TriMesh) -> SubMesh {
        SubMesh {
            positions: mesh.positions.clone(),
            tris: mesh.triangles(),
            locked: vec![false; mesh.positions.len()],
            face_on_boundary: vec![false; mesh.triangle_count()],
        }
    }

    #[test]
    fn qem_halves_sphere_and_reports_error() {
        let mesh = TriMesh::uv_sphere(1.0, 24, 24);
        let sub = open_submesh(&mesh);
        let target = sub.tris.len() / 2;
        let (out, err) = simplify_group_qem(&sub, target);
        assert!(
            out.tris.len() <= target + 8,
            "未接近减半: {} / target {target}",
            out.tris.len()
        );
        assert!(err > 0.0 && err.is_finite(), "误差须为正有限: {err}");
        // 球面简化的位移上界应远小于半径(质量粗判)。
        assert!(err < 0.5, "单位球减半位移上界异常大: {err}");
        // 简化产物顶点仍近似在单位球面(QEM 最优点不飞出;容差宽松)。
        for p in &out.positions {
            let r = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
            assert!((r - 1.0).abs() < 0.2, "顶点飞出球面邻域: r={r}");
        }
    }

    #[test]
    fn qem_locked_vertices_bitexact_preserved() {
        // 锁定顶点(模拟组边界)逐位不动——裂缝保护第一半。
        let mesh = TriMesh::plane_grid(8, 1.0);
        let mut sub = open_submesh(&mesh);
        // 锁定网格外圈顶点。
        for (i, p) in sub.positions.iter().enumerate() {
            if p[0].abs() >= 1.0 - 1e-6 || p[1].abs() >= 1.0 - 1e-6 {
                sub.locked[i] = true;
            }
        }
        let locked_pos: Vec<[u32; 3]> = sub
            .positions
            .iter()
            .zip(&sub.locked)
            .filter(|&(_, &l)| l)
            .map(|(p, _)| p.map(f32::to_bits))
            .collect();
        let (out, _) = simplify_group_qem(&sub, sub.tris.len() / 2);
        for pb in &locked_pos {
            assert!(
                out.positions.iter().any(|p| p.map(f32::to_bits) == *pb),
                "锁定顶点丢失/移动(裂缝保护破坏)"
            );
        }
    }

    #[test]
    fn qem_double_run_deterministic() {
        let mesh = TriMesh::uv_sphere(1.0, 16, 16);
        let sub = open_submesh(&mesh);
        let (a, ea) = simplify_group_qem(&sub, sub.tris.len() / 2);
        let (b, eb) = simplify_group_qem(&sub, sub.tris.len() / 2);
        assert_eq!(ea.to_bits(), eb.to_bits(), "误差双跑漂移");
        assert_eq!(a.tris, b.tris, "拓扑双跑漂移");
        assert_eq!(
            a.positions
                .iter()
                .map(|p| p.map(f32::to_bits))
                .collect::<Vec<_>>(),
            b.positions
                .iter()
                .map(|p| p.map(f32::to_bits))
                .collect::<Vec<_>>(),
            "位置双跑漂移"
        );
    }

    #[test]
    fn qem_boundary_face_edges_never_collapse() {
        // 全部面标边界(极端):零坍塌 → 原样返回(裂缝保护第二半)。
        let mesh = TriMesh::plane_grid(4, 1.0);
        let mut sub = open_submesh(&mesh);
        sub.face_on_boundary = vec![true; sub.tris.len()];
        let (out, err) = simplify_group_qem(&sub, 1);
        assert_eq!(out.tris.len(), sub.tris.len(), "边界面禁令被绕过");
        assert_eq!(err, 0.0);
    }
}
