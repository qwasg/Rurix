//! 层级 LOD DAG 构建(报告1 §3.1;meshoptimizer clusterlod 思路:分簇 → 邻簇
//! 分组 → 保边界简化 → 再分簇递归,处理过 16.4 亿三角形场景的工业路径)。
//!
//! 自叶层(原始网格簇)迭代:
//!   1) 簇分组(~4 簇/组;簇邻接共享边加权贪心生长,Morton 码定种子序——
//!      meshopt_partitionClusters 目标的简化版);
//!   2) 组内合并为子网格,**组边界顶点锁定**(被组外三角形引用者不许收缩);
//!   3) 最短边贪心收缩到约半数三角形(非 QEM——已知简化;端点保持:存活顶点
//!      坐标逐位不变,跨层顶点焊接用精确位置相等);
//!   4) 组结果重新簇化 → 父层簇;
//!      直至单簇或不再缩减。
//!
//! 裂缝保护的精确规则(报告1 §7「DAG 构建质量最高危」的针对性缓解):组边界
//! 边(被组外三角形共享)所依附的面永不消亡——凡「被收缩边的共边面含组边界
//! 边」的收缩一律禁止,且锁定顶点永不移动/合并。故组边界折线在粗细两侧逐位
//! 一致,任意 LOD cut 组合下无裂缝。
//!
//! 误差不变量(报告1 §3.1「简化误差可量化」+ LOD cut 判据成立的前提):
//!   - 簇 `error` = 所在组简化引入的最大顶点偏移(收缩累计保守上界:
//!     `err[keep] = max(err[keep], err[drop] + |keep-drop|)`);
//!   - 组误差 `gerr = max(自身偏移, 成员簇 error)`,成员簇 `parent_error = gerr`
//!     ⇒ **parent_error ≥ 子 error(单调)**,且同组父簇共享同一 error;
//!   - 顶层(根)`parent_error = f32::MAX`(任意阈值下恒可选,cut 非空)。

use rurix_render::graph::types::ClusterRecord;
use std::collections::HashMap;

use crate::cluster::{backface_cone, bounding_sphere, clusterize_tris};
use crate::mesh::TriMesh;
use crate::vecmath::vdist;

/// 每组目标簇数(报告1 §3.1「约 4 簇/组」)。
const GROUP_SIZE: usize = 4;
/// 防御性层数上限(正常 log₂ 收敛;防病态输入死循环)。
const MAX_DAG_ROUNDS: usize = 64;

/// DAG 节点(与 [`ClusterDag::records`] 等长平行表;层级关系表的一半)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DagNode {
    /// 子簇在 [`ClusterDag::children`] 中的起始下标。
    pub first_child: u32,
    /// 子簇数(0 = 叶,即原始网格簇)。
    pub child_count: u32,
    /// 层号(0 = 叶层)。
    pub level: u32,
    /// 所属简化组(跨层全局唯一;同组簇共享 error/parent_error,单调性不变量锚)。
    pub group: u32,
}

/// 层表条目(层级统计与 LOD 断面;`levels[0]` = 叶层)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DagLevel {
    /// 本层首条簇记录在 `records` 中的下标。
    pub record_start: u32,
    /// 本层簇数。
    pub record_count: u32,
    /// 本层三角形总数(单调递减不变量)。
    pub triangle_count: u32,
}

/// 簇层级 DAG 构建产物(报告1 §6.1 ClusterRecord/ClusterGroup 行)。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ClusterDag {
    /// 扁平簇记录(64B 定长契约,叶层在前;`page_id` 预留 = 0)。
    pub records: Vec<ClusterRecord>,
    /// 层级关系表:与 `records` 等长。
    pub nodes: Vec<DagNode>,
    /// 扁平子簇索引(record id;经 `DagNode::first_child/child_count` 切片)。
    pub children: Vec<u32>,
    /// 顶点数据段(簇局部顶点拼接;`ClusterRecord::vertex_offset` 以元素计)。
    pub vertices: Vec<[f32; 3]>,
    /// 三角形局部索引段(3×u8/三角形;`ClusterRecord::triangle_offset` 以 u8 元素计)。
    pub triangle_indices: Vec<u8>,
    /// 层表(0 = 叶层,末层 = 顶层/根)。
    pub levels: Vec<DagLevel>,
}

impl ClusterDag {
    pub fn cluster_count(&self) -> usize {
        self.records.len()
    }

    pub fn level_count(&self) -> usize {
        self.levels.len()
    }

    /// 叶层 record id 区间(叶层自 0 起始)。
    pub fn leaf_ids(&self) -> std::ops::Range<u32> {
        0..self.levels.first().map_or(0, |l| l.record_count)
    }

    /// 顶层(根)record id 区间。
    pub fn top_level_ids(&self) -> std::ops::Range<u32> {
        match self.levels.last() {
            Some(l) => l.record_start..l.record_start + l.record_count,
            None => 0..0,
        }
    }

    pub fn record(&self, id: u32) -> &ClusterRecord {
        &self.records[id as usize]
    }

    pub fn node(&self, id: u32) -> &DagNode {
        &self.nodes[id as usize]
    }

    /// 簇的子簇(record id 切片;叶返回空)。
    pub fn children_of(&self, id: u32) -> &[u32] {
        let n = self.nodes[id as usize];
        let (s, e) = (
            n.first_child as usize,
            (n.first_child + n.child_count) as usize,
        );
        &self.children[s..e]
    }

    /// 簇的局部顶点切片(顶点数据段视图;GPU 上传/剔除验证用)。
    pub fn cluster_vertices(&self, id: u32) -> &[[f32; 3]] {
        let r = self.records[id as usize];
        let (s, e) = (
            r.vertex_offset as usize,
            (r.vertex_offset + r.vertex_count) as usize,
        );
        &self.vertices[s..e]
    }

    /// 簇的第 t 个局部三角形(3×u8)。
    pub fn cluster_triangle(&self, id: u32, t: u32) -> [u8; 3] {
        let r = self.records[id as usize];
        let s = (r.triangle_offset + 3 * t) as usize;
        [
            self.triangle_indices[s],
            self.triangle_indices[s + 1],
            self.triangle_indices[s + 2],
        ]
    }

    /// 将任意簇集合展开为其覆盖的叶簇(LOD cut 覆盖性验证/流送用)。
    pub fn expand_to_leaves(&self, ids: &[u32]) -> Vec<u32> {
        let mut out = Vec::new();
        let mut stack: Vec<u32> = ids.to_vec();
        while let Some(id) = stack.pop() {
            let n = self.nodes[id as usize];
            if n.child_count == 0 {
                out.push(id);
            } else {
                stack.extend_from_slice(self.children_of(id));
            }
        }
        out
    }
}

/// 层网格(构建期中间表示:全局焊接顶点 + 三角形 + 簇划分 + 每簇误差)。
#[derive(Default)]
struct LevelMesh {
    positions: Vec<[f32; 3]>,
    tris: Vec<[u32; 3]>,
    /// 簇 → 本层三角形 id 列表(本层三角形集合的划分)。
    clusters: Vec<Vec<u32>>,
    /// 簇自身误差(叶层 = 0)。
    errors: Vec<f32>,
}

/// 组子网格(合并 + 边界锁定的载体)。
#[derive(Clone)]
struct SubMesh {
    positions: Vec<[f32; 3]>,
    tris: Vec<[u32; 3]>,
    /// 顶点被组外三角形引用 → 锁定(不许被收缩移除)。
    locked: Vec<bool>,
    /// 面含组边界边(该边被组外三角形共享)→ 收缩禁令标记。
    /// 收缩只改写内部顶点,边界边端点永不移动/合并,故标记全程有效。
    face_on_boundary: Vec<bool>,
}

/// 网格 → 簇层级 DAG(报告1 §5 P0→P3 的离线半;验收:任意输入得 DAG +
/// 每簇误差包围球 + 层级统计)。
pub fn build_dag(mesh: &TriMesh) -> ClusterDag {
    let tris = mesh.triangles();
    let raw = clusterize_tris(&mesh.positions, &tris);
    let mut level = LevelMesh {
        positions: mesh.positions.clone(),
        tris,
        clusters: raw.iter().map(|c| c.tris.clone()).collect(),
        errors: vec![0.0; raw.len()],
    };
    let mut levels: Vec<LevelMesh> = Vec::new();
    // 每层簇 → (组号, 组误差);顶层无此项(根 parent_error = MAX)。
    let mut group_of: Vec<Vec<(u32, f32)>> = Vec::new();
    // 每层组数(组号全局化用)。
    let mut group_counts: Vec<u32> = Vec::new();
    // 第 L+1 层簇 → 第 L 层孩子簇号(层内局部)。
    let mut child_links: Vec<Vec<Vec<u32>>> = Vec::new();

    for _round in 0..MAX_DAG_ROUNDS {
        if level.clusters.is_empty() {
            break;
        }
        // 本层无向边 → 面列表(组边界边判定与簇邻接权重:边同时被组内/组外面共享)。
        let mut edge_faces: HashMap<(u32, u32), Vec<u32>> = HashMap::new();
        for (f, t) in level.tris.iter().enumerate() {
            for e in 0..3 {
                let (a, b) = (t[e].min(t[(e + 1) % 3]), t[e].max(t[(e + 1) % 3]));
                edge_faces.entry((a, b)).or_default().push(f as u32);
            }
        }
        let groups = group_clusters(&level, &edge_faces);
        let mut next = LevelMesh::default();
        let mut links: Vec<Vec<u32>> = Vec::new();
        let mut g_of = vec![(0u32, 0.0f32); level.clusters.len()];
        // 精确位置焊接(端点收缩保证坐标逐位来自本层已有顶点)。
        let mut weld: HashMap<[u32; 3], u32> = HashMap::new();
        for (gi, g) in groups.iter().enumerate() {
            let sub = extract_group(&level, g, &edge_faces);
            let target = (sub.tris.len() / 2).max(1);
            let (sm, own_err) = simplify_group(&sub, target);
            let member_max = g
                .iter()
                .map(|&c| level.errors[c as usize])
                .fold(0.0f32, f32::max);
            // 单调不变量:parent_error = gerr ≥ 全部子簇 error。
            let gerr = own_err.max(member_max);
            for &m in g {
                g_of[m as usize] = (gi as u32, gerr);
            }
            for p in clusterize_tris(&sm.positions, &sm.tris) {
                let mut tri_ids = Vec::with_capacity(p.tris.len());
                for &f in &p.tris {
                    let t = sm.tris[f as usize];
                    let mut gt = [0u32; 3];
                    for (k, &v) in t.iter().enumerate() {
                        let pos = sm.positions[v as usize];
                        let key = pos.map(f32::to_bits);
                        let nid = next.positions.len() as u32;
                        gt[k] = *weld.entry(key).or_insert_with(|| {
                            next.positions.push(pos);
                            nid
                        });
                    }
                    tri_ids.push(next.tris.len() as u32);
                    next.tris.push(gt);
                }
                next.clusters.push(tri_ids);
                next.errors.push(gerr);
                links.push(g.clone());
            }
        }
        if next.tris.len() >= level.tris.len() {
            break; // 不再缩减 → 当前层为顶层(根)
        }
        group_counts.push(groups.len() as u32);
        group_of.push(g_of);
        child_links.push(links);
        levels.push(level);
        level = next;
    }
    levels.push(level);
    export(&levels, &group_of, &group_counts, &child_links)
}

/// 簇分组(meshopt_partitionClusters 目标的简化版):簇邻接按**共享边计数**
/// 加权,贪心生长 ≤4 簇/组——共享边越多,合并后组内边越多、被锁定的组边界
/// 越少、简化越充分。种子顺序用 Morton 码(确定性空间序);同权按中心距离、
/// id 决胜(全确定性)。
fn group_clusters(level: &LevelMesh, edge_faces: &HashMap<(u32, u32), Vec<u32>>) -> Vec<Vec<u32>> {
    let n = level.clusters.len();
    if n == 0 {
        return Vec::new();
    }
    let centers: Vec<[f32; 3]> = level
        .clusters
        .iter()
        .map(|ctris| {
            let mut pts: Vec<[f32; 3]> = Vec::new();
            let mut seen: HashMap<u32, ()> = HashMap::new();
            for &f in ctris {
                for &v in &level.tris[f as usize] {
                    if seen.insert(v, ()).is_none() {
                        pts.push(level.positions[v as usize]);
                    }
                }
            }
            bounding_sphere(&pts).0
        })
        .collect();
    // 面 → 簇反查 + 簇邻接权重(共享边数)。
    let mut cluster_of = vec![u32::MAX; level.tris.len()];
    for (ci, ctris) in level.clusters.iter().enumerate() {
        for &f in ctris {
            cluster_of[f as usize] = ci as u32;
        }
    }
    let mut adj: Vec<HashMap<u32, u32>> = (0..n).map(|_| HashMap::new()).collect();
    for faces in edge_faces.values() {
        for i in 0..faces.len() {
            for j in (i + 1)..faces.len() {
                let (ca, cb) = (cluster_of[faces[i] as usize], cluster_of[faces[j] as usize]);
                if ca != cb && ca != u32::MAX && cb != u32::MAX {
                    *adj[ca as usize].entry(cb).or_insert(0) += 1;
                    *adj[cb as usize].entry(ca).or_insert(0) += 1;
                }
            }
        }
    }
    let order = morton_order(&centers);
    let mut assigned = vec![false; n];
    let mut groups = Vec::new();
    for &seed in &order {
        if assigned[seed as usize] {
            continue;
        }
        assigned[seed as usize] = true;
        let mut group = vec![seed];
        while group.len() < GROUP_SIZE {
            // (候选, 共享边数, 中心距离):权重降序 → 距离升序 → id 升序。
            let mut best: Option<(u32, u32, f32)> = None;
            for &m in &group {
                for (&cand, &w) in &adj[m as usize] {
                    if assigned[cand as usize] {
                        continue;
                    }
                    let d = vdist(centers[cand as usize], centers[m as usize]);
                    let better = match best {
                        None => true,
                        Some((bid, bw, bd)) => {
                            w > bw || (w == bw && (d < bd || (d == bd && cand < bid)))
                        }
                    };
                    if better {
                        best = Some((cand, w, d));
                    }
                }
            }
            match best {
                Some((cand, ..)) => {
                    assigned[cand as usize] = true;
                    group.push(cand);
                }
                None => break,
            }
        }
        groups.push(group);
    }
    groups
}

/// 簇中心 Morton 码排序(分组种子序;确定性空间顺序)。
fn morton_order(centers: &[[f32; 3]]) -> Vec<u32> {
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for c in centers {
        for k in 0..3 {
            lo[k] = lo[k].min(c[k]);
            hi[k] = hi[k].max(c[k]);
        }
    }
    let mut keyed: Vec<(u32, u32)> = centers
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let mut q = [0u32; 3];
            for k in 0..3 {
                let span = (hi[k] - lo[k]).max(1e-12);
                q[k] = (((c[k] - lo[k]) / span).clamp(0.0, 1.0) * 1023.0) as u32;
            }
            (morton3(q[0], q[1], q[2]), i as u32)
        })
        .collect();
    keyed.sort_unstable();
    keyed.into_iter().map(|(_, i)| i).collect()
}

/// 10 位 × 3 轴 Morton 间插码。
fn morton3(x: u32, y: u32, z: u32) -> u32 {
    fn spread(v: u32) -> u32 {
        let mut v = v & 0x3FF;
        v = (v | (v << 16)) & 0x0300_00FF;
        v = (v | (v << 8)) & 0x0300_F00F;
        v = (v | (v << 4)) & 0x030C_30C3;
        (v | (v << 2)) & 0x0924_9249
    }
    spread(x) | (spread(y) << 1) | (spread(z) << 2)
}

/// 组内三角形 → 子网格;**组边界顶点锁定 + 组边界边标记**:被组外三角形引用
/// 的顶点锁定;含「组内外共享边」的面打上 `face_on_boundary`(报告1 §3.1 裂缝
/// 保护的第一半;另一半见 simplify_group 的收缩禁令)。
fn extract_group(
    level: &LevelMesh,
    group: &[u32],
    edge_faces: &HashMap<(u32, u32), Vec<u32>>,
) -> SubMesh {
    let mut in_group = vec![false; level.tris.len()];
    for &c in group {
        for &f in &level.clusters[c as usize] {
            in_group[f as usize] = true;
        }
    }
    let mut used_outside = vec![false; level.positions.len()];
    for (f, t) in level.tris.iter().enumerate() {
        if !in_group[f] {
            for &v in t {
                used_outside[v as usize] = true;
            }
        }
    }
    let mut map: HashMap<u32, u32> = HashMap::new();
    let mut local_to_global: Vec<u32> = Vec::new();
    let mut positions = Vec::new();
    let mut locked = Vec::new();
    let mut tris = Vec::new();
    for &c in group {
        for &f in &level.clusters[c as usize] {
            let t = level.tris[f as usize];
            let mut st = [0u32; 3];
            for (k, &v) in t.iter().enumerate() {
                st[k] = *map.entry(v).or_insert_with(|| {
                    local_to_global.push(v);
                    positions.push(level.positions[v as usize]);
                    locked.push(used_outside[v as usize]);
                    (positions.len() - 1) as u32
                });
            }
            tris.push(st);
        }
    }
    // 组边界边 ⇒ 面标记:边的共享面存在组外面。
    let mut face_on_boundary = vec![false; tris.len()];
    for (lf, st) in tris.iter().enumerate() {
        for e in 0..3 {
            let (ga, gb) = {
                let (a, b) = (
                    local_to_global[st[e] as usize],
                    local_to_global[st[(e + 1) % 3] as usize],
                );
                (a.min(b), a.max(b))
            };
            let boundary = edge_faces
                .get(&(ga, gb))
                .is_some_and(|faces| faces.iter().any(|&g| !in_group[g as usize]));
            if boundary {
                face_on_boundary[lf] = true;
                break;
            }
        }
    }
    SubMesh {
        positions,
        tris,
        locked,
        face_on_boundary,
    }
}

/// 最短边贪心收缩(非 QEM——已知简化;误差上界保守、端点保持保焊接)。
///
/// 裂缝保护第二半(精确规则):被收缩边 (u,v) 的任一共边存活面**含组边界边**
/// (`face_on_boundary`)则禁止该收缩——边界边依附的面不死、边界边端点(锁定
/// 顶点)永不移动/合并,组边界折线在简化前后逐位一致,任意 LOD cut 组合无
/// 裂缝。方向:锁定端保留,双开取小号端(确定性)。
fn simplify_group(sub: &SubMesh, target: usize) -> (SubMesh, f32) {
    use std::cmp::Ordering;
    use std::collections::BinaryHeap;

    #[derive(Clone, Copy, PartialEq)]
    struct Cand {
        len2: f32,
        a: u32,
        b: u32,
    }
    impl Eq for Cand {}
    impl Ord for Cand {
        fn cmp(&self, o: &Self) -> Ordering {
            // 反转 → 最小堆(最短边优先)
            o.len2
                .total_cmp(&self.len2)
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
    let mut tris = sub.tris.clone();
    let mut vert_err = vec![0.0f32; nv];
    let mut alive_v = vec![true; nv];
    let mut alive_f = vec![true; tris.len()];
    let mut adj: Vec<Vec<u32>> = vec![Vec::new(); nv];
    for (f, t) in tris.iter().enumerate() {
        for &v in t {
            adj[v as usize].push(f as u32);
        }
    }
    let mut heap: BinaryHeap<Cand> = BinaryHeap::new();
    let push_edges = |heap: &mut BinaryHeap<Cand>, tris: &[[u32; 3]], f: usize| {
        let t = tris[f];
        for e in 0..3 {
            let (a, b) = (t[e].min(t[(e + 1) % 3]), t[e].max(t[(e + 1) % 3]));
            if a != b {
                let len2 = {
                    let d0 = sub.positions[a as usize][0] - sub.positions[b as usize][0];
                    let d1 = sub.positions[a as usize][1] - sub.positions[b as usize][1];
                    let d2 = sub.positions[a as usize][2] - sub.positions[b as usize][2];
                    d0 * d0 + d1 * d1 + d2 * d2
                };
                heap.push(Cand { len2, a, b });
            }
        }
    };
    for f in 0..tris.len() {
        push_edges(&mut heap, &tris, f);
    }

    let mut alive_count = tris.len();
    while alive_count > target {
        let mut chosen: Option<(usize, usize)> = None; // (keep, drop)
        while let Some(c) = heap.pop() {
            let (a, b) = (c.a as usize, c.b as usize);
            if !alive_v[a] || !alive_v[b] {
                continue; // 陈旧候选
            }
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
            // 裂缝保护:共边面含组边界边 → 禁收缩(保组边界折线不死)。
            if !shares_face || touches_boundary {
                continue;
            }
            chosen = match (sub.locked[a], sub.locked[b]) {
                (true, true) => continue, // 双锁边界边:永不收缩
                (true, false) => Some((a, b)),
                (false, true) => Some((b, a)),
                (false, false) => Some(if a < b { (a, b) } else { (b, a) }),
            };
            break;
        }
        let Some((keep, drop)) = chosen else { break };
        // 收缩 drop → keep:位移 |keep-drop|,误差保守累计。
        let d = vdist(sub.positions[keep], sub.positions[drop]);
        vert_err[keep] = vert_err[keep].max(vert_err[drop] + d);
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
        // 退化面消亡 + 重复面去重(收缩只经 keep 的面变化)。
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
        let new_edges = adj[keep].clone();
        for &f in &new_edges {
            if alive_f[f as usize] {
                push_edges(&mut heap, &tris, f as usize);
            }
        }
    }

    // 压缩输出;整组塌光时退回原网格(粗层不得出洞,误差 0 = 无进展)。
    let mut remap = vec![u32::MAX; nv];
    let mut out_pos = Vec::new();
    let mut out_locked = Vec::new();
    for v in 0..nv {
        if alive_v[v] {
            remap[v] = out_pos.len() as u32;
            out_pos.push(sub.positions[v]);
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
    let max_err = vert_err
        .iter()
        .enumerate()
        .filter(|(v, _)| alive_v[*v])
        .map(|(_, &e)| e)
        .fold(0.0f32, f32::max);
    // face_on_boundary 对本层简化之后不再消费(下一层由 extract_group 重算)。
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

/// 两三角形顶点集合相等(绕序无关;收缩后重复面判定)。
fn same_tri_set(a: [u32; 3], b: [u32; 3]) -> bool {
    a.iter().all(|v| b.contains(v))
}

/// 汇总导出:扁平 ClusterRecord + 层级关系表 + 顶点/索引数据段 + 层表。
fn export(
    levels: &[LevelMesh],
    group_of: &[Vec<(u32, f32)>],
    group_counts: &[u32],
    child_links: &[Vec<Vec<u32>>],
) -> ClusterDag {
    let mut dag = ClusterDag::default();
    let top = levels.len() - 1;
    for (li, lv) in levels.iter().enumerate() {
        let record_start = dag.records.len() as u32;
        let mut tri_total = 0u32;
        for (ci, ctris) in lv.clusters.iter().enumerate() {
            // 局部顶点重映射(本层全局 → 簇内 u8),同时写入顶点数据段。
            let mut map: HashMap<u32, u8> = HashMap::new();
            let mut local_verts: Vec<[f32; 3]> = Vec::new();
            let v_off = dag.vertices.len() as u32;
            for &f in ctris {
                for &v in &lv.tris[f as usize] {
                    if !map.contains_key(&v) {
                        map.insert(v, map.len() as u8);
                        local_verts.push(lv.positions[v as usize]);
                        dag.vertices.push(lv.positions[v as usize]);
                    }
                }
            }
            debug_assert!(map.len() <= crate::cluster::MAX_VERTS);
            let t_off = dag.triangle_indices.len() as u32;
            for &f in ctris {
                for &v in &lv.tris[f as usize] {
                    dag.triangle_indices.push(map[&v]);
                }
            }
            let (center, radius) = bounding_sphere(&local_verts);
            let tri_list: Vec<[u32; 3]> = ctris.iter().map(|&f| lv.tris[f as usize]).collect();
            let (cone_axis, cone_cutoff) = backface_cone(&lv.positions, &tri_list);
            let error = lv.errors[ci];
            let parent_error = if li == top {
                f32::MAX
            } else {
                group_of[li][ci].1
            };
            dag.records.push(ClusterRecord {
                center,
                radius,
                cone_axis,
                cone_cutoff,
                error,
                parent_error,
                vertex_offset: v_off,
                triangle_offset: t_off,
                vertex_count: map.len() as u32,
                triangle_count: ctris.len() as u32,
                page_id: 0, // P0 单页常驻(报告1 §5:格式 v0 预留页表字段)
                reserved: 0,
            });
            dag.nodes.push(DagNode {
                first_child: 0,
                child_count: 0,
                level: li as u32,
                group: 0,
            });
            tri_total += ctris.len() as u32;
        }
        dag.levels.push(DagLevel {
            record_start,
            record_count: lv.clusters.len() as u32,
            triangle_count: tri_total,
        });
    }
    // 组号全局化(同组簇共享 parent_error 的不变量锚)。
    let mut group_seq = 0u32;
    for li in 0..top {
        let base = dag.levels[li].record_start as usize;
        for (ci, &(g, _)) in group_of[li].iter().enumerate() {
            dag.nodes[base + ci].group = group_seq + g;
        }
        group_seq += group_counts[li];
    }
    // 父子链接:第 L 层簇的孩子 = 第 L-1 层组内成员(record id 全局化)。
    for li in 1..=top {
        let prev_base = dag.levels[li - 1].record_start;
        let cur_base = dag.levels[li].record_start as usize;
        for (pci, children) in child_links[li - 1].iter().enumerate() {
            let first = dag.children.len() as u32;
            dag.children.extend(children.iter().map(|c| prev_base + c));
            let node = &mut dag.nodes[cur_base + pci];
            node.first_child = first;
            node.child_count = children.len() as u32;
        }
    }
    // 顶层:每簇独立组(根,parent_error = MAX 哨兵)。
    let top_base = dag.levels[top].record_start;
    for ci in 0..dag.levels[top].record_count {
        dag.nodes[(top_base + ci) as usize].group = group_seq;
        group_seq += 1;
    }
    dag
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaf_coverage_partition() {
        let mesh = TriMesh::plane_grid(8, 1.0);
        let dag = build_dag(&mesh);
        let total: u32 = dag.leaf_ids().map(|i| dag.record(i).triangle_count).sum();
        assert_eq!(total as usize, mesh.triangle_count());
        assert_eq!(dag.levels[0].triangle_count as usize, mesh.triangle_count());
    }

    #[test]
    fn error_monotonic() {
        let dag = build_dag(&TriMesh::uv_sphere(1.0, 16, 16));
        for (i, r) in dag.records.iter().enumerate() {
            assert!(r.parent_error >= r.error, "簇 {i} parent_error < error");
        }
        for li in 1..dag.level_count() {
            for id in dag.levels[li].record_start
                ..dag.levels[li].record_start + dag.levels[li].record_count
            {
                let pe = dag.record(id).error;
                for &c in dag.children_of(id) {
                    let cr = dag.record(c);
                    assert_eq!(
                        cr.parent_error.to_bits(),
                        pe.to_bits(),
                        "子簇 parent_error ≠ 父组 error"
                    );
                    assert!(pe >= cr.error, "父 error < 子 error(单调破坏)");
                }
            }
        }
    }

    #[test]
    fn levels_decrease_and_roots() {
        let dag = build_dag(&TriMesh::uv_sphere(1.0, 16, 16));
        assert!(dag.level_count() >= 3, "层数过少:{}", dag.level_count());
        for w in dag.levels.windows(2) {
            assert!(
                w[1].triangle_count < w[0].triangle_count,
                "层三角形数未递减"
            );
        }
        for id in dag.top_level_ids() {
            assert_eq!(
                dag.record(id).parent_error.to_bits(),
                f32::MAX.to_bits(),
                "根 parent_error 须为 MAX 哨兵"
            );
            assert_eq!(dag.record(id).page_id, 0);
            assert_eq!(dag.record(id).reserved, 0);
        }
    }

    #[test]
    fn boundary_lock_shared_vertices() {
        // 相邻两组各自简化:共享边界顶点须在两侧父层逐位存活(裂缝保护断言)。
        let mesh = TriMesh::plane_grid(8, 1.0);
        let tris = mesh.triangles();
        let raw = clusterize_tris(&mesh.positions, &tris);
        assert!(raw.len() >= 2, "测试需要 ≥2 簇");
        let level = LevelMesh {
            positions: mesh.positions.clone(),
            tris,
            clusters: raw.iter().map(|c| c.tris.clone()).collect(),
            errors: vec![0.0; raw.len()],
        };
        let g0 = vec![0u32];
        let g1: Vec<u32> = (1..raw.len() as u32).collect();
        let mut edge_faces: HashMap<(u32, u32), Vec<u32>> = HashMap::new();
        for (f, t) in level.tris.iter().enumerate() {
            for e in 0..3 {
                let (a, b) = (t[e].min(t[(e + 1) % 3]), t[e].max(t[(e + 1) % 3]));
                edge_faces.entry((a, b)).or_default().push(f as u32);
            }
        }
        let s0 = extract_group(&level, &g0, &edge_faces);
        let s1 = extract_group(&level, &g1, &edge_faces);
        let shared: Vec<[u32; 3]> = s0
            .positions
            .iter()
            .map(|p| p.map(f32::to_bits))
            .filter(|pb| s1.positions.iter().any(|q| q.map(f32::to_bits) == *pb))
            .collect();
        assert!(!shared.is_empty(), "相邻组无共享顶点(测试前提不成立)");
        assert!(
            s0.locked.iter().any(|&l| l) && s1.locked.iter().any(|&l| l),
            "边界未锁定"
        );
        assert!(
            s0.face_on_boundary.iter().any(|&b| b) && s1.face_on_boundary.iter().any(|&b| b),
            "边界边未标记"
        );
        let (r0, _) = simplify_group(&s0, (s0.tris.len() / 2).max(1));
        let (r1, _) = simplify_group(&s1, (s1.tris.len() / 2).max(1));
        for pb in &shared {
            assert!(
                r0.positions.iter().any(|p| p.map(f32::to_bits) == *pb),
                "组0 丢失共享顶点"
            );
            assert!(
                r1.positions.iter().any(|p| p.map(f32::to_bits) == *pb),
                "组1 丢失共享顶点"
            );
        }
    }

    #[test]
    fn uv_sphere_64_full_dag() {
        // 报告1 §6.4 压力量级替代:64×64 UV 球(8064 三角形)全 DAG 秒级构建。
        let mesh = TriMesh::uv_sphere(1.0, 64, 64);
        assert_eq!(mesh.triangle_count(), 8064);
        let dag = build_dag(&mesh);
        let leaf_tris: u32 = dag.leaf_ids().map(|i| dag.record(i).triangle_count).sum();
        assert_eq!(leaf_tris, 8064);
        assert!(dag.level_count() >= 4, "层数过少:{}", dag.level_count());
        println!("[uv_sphere_64] 层数 = {}", dag.level_count());
        for (li, l) in dag.levels.iter().enumerate() {
            println!(
                "[uv_sphere_64]   L{li}: 簇 {} 三角形 {}",
                l.record_count, l.triangle_count
            );
        }
        println!("[uv_sphere_64] 总簇数 = {}", dag.cluster_count());
    }
}
