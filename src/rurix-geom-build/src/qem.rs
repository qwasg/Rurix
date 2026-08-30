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

use crate::dag::{SubMesh, SubMeshAttrs, attr_seam_flags};
use crate::mesh::AttrMeshError;
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
    let (out, _, err) = simplify_group_qem_impl(sub, target, None);
    (out, err)
}

/// 收缩点属性插值(G31+ #96 属性保持策略——**位置 QEM 主导**:属性不参与
/// 选边/定新位置/fold-over,收缩执行后把新位置投影到被坍塌边线段取参数
/// `t = clamp(⟨new−keep, drop−keep⟩ / ‖drop−keep‖², 0, 1)`:
/// - `t = 0/1`(含锁定端收缩、端点候选、投影出端)= 端点属性**逐位拷贝**
///   ——锁定顶点/组边界属性裂缝保护与位置面同一律免费获得;
/// - 内点 = 线性插值(f64 求值;clamp ⇒ 属性恒在两端点凸包内,不外插飞出
///   原面片邻域);法线插值后归一化,近反向抵消退化时 keep 端逐位保持。
fn merge_attrs_at(
    attrs: &mut SubMeshAttrs,
    keep: usize,
    drop: usize,
    p_keep: [f32; 3],
    p_drop: [f32; 3],
    new_pos: [f32; 3],
) {
    let d = [
        p_drop[0] as f64 - p_keep[0] as f64,
        p_drop[1] as f64 - p_keep[1] as f64,
        p_drop[2] as f64 - p_keep[2] as f64,
    ];
    let w = [
        new_pos[0] as f64 - p_keep[0] as f64,
        new_pos[1] as f64 - p_keep[1] as f64,
        new_pos[2] as f64 - p_keep[2] as f64,
    ];
    let len2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
    let t = if len2 <= 1e-30 {
        0.0
    } else {
        ((d[0] * w[0] + d[1] * w[1] + d[2] * w[2]) / len2).clamp(0.0, 1.0)
    };
    if t == 0.0 {
        return; // keep 端属性逐位保持(锁定端裂缝保护)。
    }
    if t == 1.0 {
        attrs.uv[keep] = attrs.uv[drop];
        if let Some(nm) = attrs.normal.as_mut() {
            nm[keep] = nm[drop];
        }
        return;
    }
    let (ua, ub) = (attrs.uv[keep], attrs.uv[drop]);
    attrs.uv[keep] = [
        (ua[0] as f64 + t * (ub[0] as f64 - ua[0] as f64)) as f32,
        (ua[1] as f64 + t * (ub[1] as f64 - ua[1] as f64)) as f32,
    ];
    if let Some(nm) = attrs.normal.as_mut() {
        let (na, nb) = (nm[keep], nm[drop]);
        let mix = [
            na[0] as f64 + t * (nb[0] as f64 - na[0] as f64),
            na[1] as f64 + t * (nb[1] as f64 - na[1] as f64),
            na[2] as f64 + t * (nb[2] as f64 - na[2] as f64),
        ];
        let l = (mix[0] * mix[0] + mix[1] * mix[1] + mix[2] * mix[2]).sqrt();
        if l > 1e-12 {
            nm[keep] = [
                (mix[0] / l) as f32,
                (mix[1] / l) as f32,
                (mix[2] / l) as f32,
            ];
        }
        // 退化(近反向抵消)不改写:keep 端法线逐位保持。
    }
}

/// [`simplify_group_qem`] 单一实现(#96 属性经 `Option` 线程化;`None` 路径
/// 逐字同路——属性只在收缩执行点插值 + 末端压缩重映射,不触碰任何位置/
/// 拓扑决策,QEM 档产物字节不变)。
pub(crate) fn simplify_group_qem_impl(
    sub: &SubMesh,
    target: usize,
    mut attrs: Option<SubMeshAttrs>,
) -> (SubMesh, Option<SubMeshAttrs>, f32) {
    use std::cmp::Ordering;
    use std::collections::BinaryHeap;

    // 整组塌光退回原网格时的属性原值(QEM 收缩过程会插值改写属性表)。
    let pristine_attrs = attrs.clone();

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
        // #96 属性链:收缩点属性插值(位置决策已定,属性不反哺;须在
        // positions[keep] 改写前取旧端点)。
        if let Some(a) = attrs.as_mut() {
            merge_attrs_at(a, keep, drop, positions[keep], positions[drop], new_pos);
        }
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
    let mut out_attrs = attrs.as_ref().map(SubMeshAttrs::empty_like);
    for v in 0..nv {
        if alive_v[v] {
            remap[v] = out_pos.len() as u32;
            out_pos.push(positions[v]);
            out_locked.push(sub.locked[v]);
            if let (Some(oa), Some(a)) = (out_attrs.as_mut(), attrs.as_ref()) {
                oa.push_from(a, v);
            }
        }
    }
    let mut out_tris = Vec::new();
    for (f, t) in tris.iter().enumerate() {
        if alive_f[f] {
            out_tris.push(t.map(|v| remap[v as usize]));
        }
    }
    if out_tris.is_empty() {
        return (sub.clone(), pristine_attrs, 0.0);
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
        out_attrs,
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

/// 属性保持自由网格简化产物(G31+ #96;HLOD 合并简化 bake 腿的带 UV
/// 代理三角事实源——`indices` 每三连为一个代理三角,corner UV 经
/// `uv[indices[k]]` 取用,`normal` 输入在场才在场)。
#[derive(Debug, Clone, PartialEq)]
pub struct AttrSimplifyOutput {
    pub positions: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    /// 与 `positions` 等长平行。
    pub uv: Vec<[f32; 2]>,
    /// 与 `positions` 等长平行(输入在场才在场)。
    pub normal: Option<Vec<[f32; 3]>>,
    /// 顶点位移保守累计上界(口径与 [`simplify_free_mesh`] 同源)。
    pub max_error: f32,
}

/// [`simplify_free_mesh`] 的属性保持变体(G31+ #96 加性入口,既有函数签名
/// 不动;HLOD 跨组件合并简化的带 UV 腿——未来 `bake_hlod_merged` UV 变体
/// 直调本函数,输出代理三角带真 UV 供 G36 侧表 gather 消费,分界登记见
/// #96 交付报告)。
///
/// - **位置面与 [`simplify_free_mesh`] 同律**:属性不参与选边/定位/fold-over
///   ——无接缝输入(位置 bits 全唯一)下位置/拓扑产物逐位一致(单测锚);
/// - 属性面 = 收缩点线段投影插值(端点逐位拷贝;见 [`merge_attrs_at`]);
/// - **UV 接缝保守锁定**:同位置 bits 多顶点 id(上游按位置+属性预拆分)
///   一律锁定——接缝两侧位置与属性逐位保持,无几何裂缝(压缩自由度损失
///   如实计入 stuck 统计,不静默);
/// - 退化输入 typed Err([`AttrMeshError`];既有入口 assert/panic 面不动);
/// - 输入应已按「位置+属性」焊接(调用方保证,与既有入口同注)。
pub fn simplify_free_mesh_attrs(
    positions: &[[f32; 3]],
    indices: &[u32],
    uv: &[[f32; 2]],
    normal: Option<&[[f32; 3]]>,
    target_tris: usize,
) -> Result<AttrSimplifyOutput, AttrMeshError> {
    crate::mesh::validate_attr_input(positions, indices, uv, normal)?;
    let tris: Vec<[u32; 3]> = indices
        .chunks_exact(3)
        .map(|c| [c[0], c[1], c[2]])
        .collect();
    // 接缝顶点即锁定集(自由网格无组边界;与 build_dag_attrs 同律)。
    let locked = attr_seam_flags(positions);
    let sub = SubMesh {
        positions: positions.to_vec(),
        locked,
        face_on_boundary: vec![false; tris.len()],
        tris,
    };
    let attrs = SubMeshAttrs {
        uv: uv.to_vec(),
        normal: normal.map(<[[f32; 3]]>::to_vec),
    };
    let (out, out_attrs, err) = simplify_group_qem_impl(&sub, target_tris.max(1), Some(attrs));
    let out_attrs = out_attrs.expect("属性链必产属性表");
    let mut flat = Vec::with_capacity(out.tris.len() * 3);
    for t in &out.tris {
        flat.extend_from_slice(t);
    }
    Ok(AttrSimplifyOutput {
        positions: out.positions,
        indices: flat,
        uv: out_attrs.uv,
        normal: out_attrs.normal,
        max_error: err,
    })
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

    /// G31+ #96:仿射 UV(位置的仿射函数)在平面简化下逐点近精确——QEM
    /// 平面退化候选恒落在被坍塌边线段上,线段插值对仿射场无损(fp 舍入内)。
    #[test]
    fn attr_simplify_plane_affine_uv_exact() {
        let mesh = TriMesh::plane_grid(8, 1.0);
        let affine = |p: &[f32; 3]| [(p[0] + 1.0) * 0.5, (p[1] + 1.0) * 0.5];
        let uv: Vec<[f32; 2]> = mesh.positions.iter().map(affine).collect();
        let target = mesh.triangle_count() / 2;
        let out =
            simplify_free_mesh_attrs(&mesh.positions, &mesh.indices, &uv, None, target)
                .expect("属性简化");
        assert!(out.normal.is_none());
        assert!(
            out.indices.len() / 3 <= target + 8,
            "未接近减半: {}",
            out.indices.len() / 3
        );
        for (p, t) in out.positions.iter().zip(&out.uv) {
            let want = affine(p);
            assert!(
                (t[0] - want[0]).abs() < 1e-4 && (t[1] - want[1]).abs() < 1e-4,
                "UV 偏离仿射场: {t:?} vs {want:?} @ {p:?}"
            );
        }
    }

    /// G31+ #96:属性链位置面与无属性入口**逐位同律**(属性不反哺选边/
    /// 定位/fold-over)+ UV 偏差 ≤ 0.5×位移上界(投影 UV 梯度 0.5,单次
    /// 简化内插值漂移被误差累计上界支配)+ 法线插值单位性/径向对齐。
    #[test]
    fn attr_simplify_sphere_bitmatch_plain_and_uv_bounded() {
        let mesh = TriMesh::uv_sphere(1.0, 24, 24);
        let proj_uv = |p: &[f32; 3]| [(p[0] + 1.0) * 0.5, (p[1] + 1.0) * 0.5];
        let uv: Vec<[f32; 2]> = mesh.positions.iter().map(proj_uv).collect();
        let normal: Vec<[f32; 3]> = mesh
            .positions
            .iter()
            .map(|p| {
                let l = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
                [p[0] / l, p[1] / l, p[2] / l]
            })
            .collect();
        let target = mesh.triangle_count() / 2;
        let out = simplify_free_mesh_attrs(
            &mesh.positions,
            &mesh.indices,
            &uv,
            Some(&normal),
            target,
        )
        .expect("属性简化");
        let (pp, pi, pe) = simplify_free_mesh(&mesh.positions, &mesh.indices, target);
        assert_eq!(out.indices, pi, "属性链改动了拓扑产物");
        assert_eq!(out.max_error.to_bits(), pe.to_bits(), "属性链改动了误差上界");
        assert_eq!(
            out.positions
                .iter()
                .map(|p| p.map(f32::to_bits))
                .collect::<Vec<_>>(),
            pp.iter().map(|p| p.map(f32::to_bits)).collect::<Vec<_>>(),
            "属性链改动了位置产物"
        );
        // UV 偏差 ≤ 0.5×err + fp 松量;恒在 [0,1] 凸包盒内。
        let mut max_uv = 0.0f32;
        for (p, t) in out.positions.iter().zip(&out.uv) {
            let want = proj_uv(p);
            max_uv = max_uv.max((t[0] - want[0]).abs().max((t[1] - want[1]).abs()));
            assert!((-1e-6..=1.0 + 1e-6).contains(&t[0]), "UV 出凸包: {t:?}");
            assert!((-1e-6..=1.0 + 1e-6).contains(&t[1]), "UV 出凸包: {t:?}");
        }
        println!(
            "[attr_free_sphere] max_uv_err={max_uv:.6} pos_err_bound={:.6}",
            out.max_error
        );
        assert!(
            max_uv <= 0.5 * out.max_error + 1e-4,
            "UV 漂移超出位移上界支配: {max_uv} vs 0.5×{}",
            out.max_error
        );
        // 法线:插值重归一化 → 单位;球面径向对齐(粗判)。
        let nm = out.normal.as_ref().expect("法线在场");
        for (p, nv) in out.positions.iter().zip(nm) {
            let l = (nv[0] * nv[0] + nv[1] * nv[1] + nv[2] * nv[2]).sqrt();
            assert!((l - 1.0).abs() < 1e-3, "法线非单位: {l}");
            let pl = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
            let cos = (nv[0] * p[0] + nv[1] * p[1] + nv[2] * p[2]) / (l * pl);
            assert!(cos > 0.9, "法线偏离径向: cos={cos}");
        }
    }

    /// G31+ #96:UV 接缝(同位置 bits、不同 UV 的预拆分拷贝)保守锁定——
    /// 接缝两侧 (位置, UV) 全部逐位存活,无几何裂缝、无图集串扰。
    #[test]
    fn attr_simplify_uv_seam_locked_bitexact() {
        // 双图集平面:5×5 栅格沿 x=0 竖线接缝拆分,右半 cell 引用 +0.5 图集
        // 偏移的接缝列拷贝顶点。
        let base = TriMesh::plane_grid(4, 1.0);
        let n = 5u32;
        let vid = |i: u32, j: u32| i * n + j;
        let mut positions = base.positions.clone();
        let mut uv: Vec<[f32; 2]> = base
            .positions
            .iter()
            .map(|p| [(p[0] + 1.0) * 0.25, (p[1] + 1.0) * 0.5])
            .collect();
        let mut dup_ids = [0u32; 5];
        for i in 0..5u32 {
            let src = vid(i, 2) as usize;
            dup_ids[i as usize] = positions.len() as u32;
            positions.push(positions[src]);
            uv.push([uv[src][0] + 0.5, uv[src][1]]);
        }
        let mut indices = Vec::new();
        for i in 0..4u32 {
            for j in 0..4u32 {
                // 右半 cell(j ≥ 2)把接缝列(全局列号 2)换成拷贝顶点。
                let m = |v: u32| -> u32 {
                    if j >= 2 && v % n == 2 {
                        dup_ids[(v / n) as usize]
                    } else {
                        v
                    }
                };
                let (v00, v10, v11, v01) =
                    (vid(i, j), vid(i, j + 1), vid(i + 1, j + 1), vid(i + 1, j));
                indices.extend_from_slice(&[m(v00), m(v10), m(v11)]);
                indices.extend_from_slice(&[m(v00), m(v11), m(v01)]);
            }
        }
        let out = simplify_free_mesh_attrs(&positions, &indices, &uv, None, 8)
            .expect("属性简化");
        let out_pairs: Vec<([u32; 3], [u32; 2])> = out
            .positions
            .iter()
            .zip(&out.uv)
            .map(|(p, t)| (p.map(f32::to_bits), t.map(f32::to_bits)))
            .collect();
        for i in 0..5u32 {
            let src = vid(i, 2) as usize;
            let pb = positions[src].map(f32::to_bits);
            let left = uv[src].map(f32::to_bits);
            let right = uv[dup_ids[i as usize] as usize].map(f32::to_bits);
            let variants: Vec<&[u32; 2]> = out_pairs
                .iter()
                .filter(|(p, _)| *p == pb)
                .map(|(_, t)| t)
                .collect();
            assert_eq!(variants.len(), 2, "接缝顶点 {i} 拷贝数漂移: {variants:?}");
            assert!(
                variants.contains(&&left) && variants.contains(&&right),
                "接缝顶点 {i} UV 变体丢失/串扰"
            );
        }
    }

    /// G31+ #96:退化输入 typed Err 六臂(空网格/非 3 倍数/越界/UV 不齐/
    /// 法线不齐/非有限)——不 panic、不静默钳制。
    #[test]
    fn attr_simplify_degenerate_typed_err() {
        let mesh = TriMesh::plane_grid(2, 1.0);
        let n = mesh.positions.len();
        let uv = vec![[0.0f32; 2]; n];
        assert_eq!(
            simplify_free_mesh_attrs(&[], &[], &[], None, 1).unwrap_err(),
            AttrMeshError::EmptyMesh
        );
        assert!(matches!(
            simplify_free_mesh_attrs(&mesh.positions, &mesh.indices[..4], &uv, None, 1)
                .unwrap_err(),
            AttrMeshError::IndicesNotTriples { len: 4 }
        ));
        let oob = vec![0u32, 1, n as u32];
        assert!(matches!(
            simplify_free_mesh_attrs(&mesh.positions, &oob, &uv, None, 1).unwrap_err(),
            AttrMeshError::IndexOutOfBounds { .. }
        ));
        assert!(matches!(
            simplify_free_mesh_attrs(&mesh.positions, &mesh.indices, &uv[..n - 1], None, 1)
                .unwrap_err(),
            AttrMeshError::UvLengthMismatch { .. }
        ));
        let short_nm = vec![[0.0f32, 0.0, 1.0]; n - 1];
        assert!(matches!(
            simplify_free_mesh_attrs(&mesh.positions, &mesh.indices, &uv, Some(&short_nm), 1)
                .unwrap_err(),
            AttrMeshError::NormalLengthMismatch { .. }
        ));
        let mut bad_pos = mesh.positions.clone();
        bad_pos[0][2] = f32::INFINITY;
        assert!(matches!(
            simplify_free_mesh_attrs(&bad_pos, &mesh.indices, &uv, None, 1).unwrap_err(),
            AttrMeshError::NonFinite {
                table: "positions",
                index: 0
            }
        ));
    }

    /// G31+ #96:属性链双跑逐位确定(位置/拓扑/UV/误差全部 bit 级)。
    #[test]
    fn attr_simplify_double_run_deterministic() {
        let mesh = TriMesh::uv_sphere(1.0, 16, 16);
        let uv: Vec<[f32; 2]> = mesh
            .positions
            .iter()
            .map(|p| [(p[0] + 1.0) * 0.5, (p[2] + 1.0) * 0.5])
            .collect();
        let target = mesh.triangle_count() / 2;
        let a = simplify_free_mesh_attrs(&mesh.positions, &mesh.indices, &uv, None, target)
            .expect("一跑");
        let b = simplify_free_mesh_attrs(&mesh.positions, &mesh.indices, &uv, None, target)
            .expect("二跑");
        assert_eq!(a.indices, b.indices, "拓扑双跑漂移");
        assert_eq!(a.max_error.to_bits(), b.max_error.to_bits(), "误差双跑漂移");
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
        assert_eq!(
            a.uv.iter().map(|t| t.map(f32::to_bits)).collect::<Vec<_>>(),
            b.uv.iter().map(|t| t.map(f32::to_bits)).collect::<Vec<_>>(),
            "UV 双跑漂移"
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
