//! `VisibleClusterSet` — 当帧几何可见性的**唯一事实源**(G9.3 M93/M95;
//! spec/virtual_geometry.md RXS-0350/RXS-0352;RFC-0022 §4.0-1/§4.4;D-8)。
//!
//! ## M93(RXS-0350)面
//!
//! - **载荷**(L1):元素 = cluster stable id + LOD level + 蒙皮版本 + 变换 id
//!   ([`VisibleClusterEntry`];`page_id`/`visible` 为冻结载荷之外的运行时辅助
//!   字段,随页驻留联动与可见性标记产生,语义不重定冻结面)。
//! - **selection cut 覆盖性**(L2):cut 由**当帧**屏幕空间误差驱动产生
//!   ([`select_lod_cut`],与 [`super::cull::cluster_cull`] 同一投影判据、同一
//!   互补边界,非构建期静态 LOD cut,L5);[`verify_cut_coverage`] 机器核验
//!   「每条根→叶路径恰好一个选中簇」——空洞/重叠 = fail-closed typed `Err`
//!   ([`CutCoverageError`]),不静默、不 clamp(§4.0-2 strict-only)。
//! - **未驻留页父簇兜底**(L3):选中簇页未驻留 → 沿父链上行至首个驻留祖先
//!   (根为最后兜底),同组兄弟随祖先替换同步撤出(保 cut 性质),替换后重核
//!   覆盖性;全过程 evidence 记入 [`VisibleClusterSet::fallback`] 并可机核
//!   (进 provenance digest)。
//!
//! ## M95(RXS-0352)面
//!
//! - **一份三喂**(L1):同一 [`VisibleClusterSet`] 实例派生光栅
//!   ([`VisibleClusterSet::feed_raster`],VisBuffer cluster27 下标序)/ RT
//!   ([`VisibleClusterSet::feed_rt`],BLAS 拼装输入数组由 selection 输出直接
//!   派生,禁独立再算可见性)/ VSM([`VisibleClusterSet::feed_vsm`],深度光栅
//!   簇列表)三消费方;每份 feed 携带来源 provenance digest。
//! - **帧末 provenance 校验**(L3):digest = 内容寻址 sha256(frame serial ‖
//!   元素规范字节 ‖ 兜底 evidence ‖ 驻留 evidence);旁路 variant(独立再算
//!   可见性 ⇒ 实例 serial 不同 ⇒ digest 必异,即使内容全等)由
//!   [`verify_frame_provenance`] 判 RED——单源真相是**结构**判据,出图相似
//!   不充绿(L4)。
//! - 蒙皮簇 VisBuffer SW/HW diff=0 host 断言面在 [`super::visbuffer`]
//!   (`raster_visible_set` + `assert_visbuffer_diff_zero`;L2),device 双腿
//!   由 CI 门代理经 rurix-rt render_exec 骨架统一真跑。
//!
//! 多 mesh 场景边界(诚实声明):本波 `produce` 以「单 mesh DAG × 其实例组」
//! 为产物粒度(与 G8 host 参照 harness 同形);多 mesh 场景按 mesh 组逐次产出,
//! 跨 set 帧级归并留 CI 门 device 波裁决。

use rurix_pkg::sha256;

use crate::graph::types::ClusterRecord;

use super::cull::{CullCamera, VisibleCluster, transform_dir_normalized, world_sphere};
use super::gpu_scene::InstanceRecord;

// ---------------------------------------------------------------------------
// DAG 拓扑视图(host 运行时侧最小面;与 rurix-geom-build `ClusterDag` 同构,
// 转换器归 geom-build 消费侧——本 crate 不反向依赖离线构建器)
// ---------------------------------------------------------------------------

/// DAG 节点(运行时最小面:子簇区间 + 层号;`group` 等 builder 字段不带入运行时)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DagNodeRec {
    /// 子簇在 [`MeshDagView::children`] 中的起始下标。
    pub first_child: u32,
    /// 子簇数(0 = 叶)。
    pub child_count: u32,
    /// 层号(0 = 叶层;LOD level 载荷源)。
    pub level: u32,
}

/// 单 mesh 簇 DAG 拓扑视图(段内**局部**簇号;全局 stable id = 实例
/// `cluster_offset` + 局部号,由产出一侧映射)。
#[derive(Debug, Clone, Copy)]
pub struct MeshDagView<'a> {
    /// 扁平簇记录(冻结契约 64B `ClusterRecord`)。
    pub records: &'a [ClusterRecord],
    /// 与 `records` 等长层级表。
    pub nodes: &'a [DagNodeRec],
    /// 扁平子簇索引(局部号)。
    pub children: &'a [u32],
}

impl<'a> MeshDagView<'a> {
    /// 构造并校验拓扑(fail-closed;环/越界/表长不齐 = typed `Err`)。
    pub fn new(
        records: &'a [ClusterRecord],
        nodes: &'a [DagNodeRec],
        children: &'a [u32],
    ) -> Result<Self, VisibleSetError> {
        if records.is_empty() {
            return Err(VisibleSetError::Topology("空 DAG"));
        }
        if nodes.len() != records.len() {
            return Err(VisibleSetError::Topology("nodes 与 records 表长不齐"));
        }
        for n in nodes {
            let end = n.first_child as usize + n.child_count as usize;
            if end > children.len() {
                return Err(VisibleSetError::Topology("子簇区间越界"));
            }
            for &c in &children[n.first_child as usize..end] {
                if c as usize >= records.len() {
                    return Err(VisibleSetError::Topology("子簇 id 越界"));
                }
                if nodes[c as usize].level >= n.level {
                    return Err(VisibleSetError::Topology("子簇层号不小于父(环/错位)"));
                }
            }
        }
        Ok(Self {
            records,
            nodes,
            children,
        })
    }

    /// 簇的子簇(局部号切片;叶返回空)。
    pub fn children_of(&self, id: u32) -> &[u32] {
        let n = self.nodes[id as usize];
        &self.children[n.first_child as usize..(n.first_child + n.child_count) as usize]
    }

    /// 展开到叶簇(局部号;不去重——覆盖性计数依赖重复暴露)。
    pub fn expand_to_leaves(&self, ids: &[u32]) -> Vec<u32> {
        let mut out = Vec::new();
        let mut stack: Vec<u32> = ids.to_vec();
        while let Some(id) = stack.pop() {
            if self.nodes[id as usize].child_count == 0 {
                out.push(id);
            } else {
                stack.extend_from_slice(self.children_of(id));
            }
        }
        out
    }

    /// 叶簇列表(局部号升序;`child_count == 0`)。
    pub fn leaves(&self) -> Vec<u32> {
        (0..self.records.len() as u32)
            .filter(|&i| self.nodes[i as usize].child_count == 0)
            .collect()
    }

    /// 父映射(局部号;同组多父取**最小 id**,确定性;根 = `None`)。
    fn min_parents(&self) -> Vec<Option<u32>> {
        let mut parent: Vec<Option<u32>> = vec![None; self.records.len()];
        for p in 0..self.records.len() as u32 {
            for &c in self.children_of(p) {
                let slot = &mut parent[c as usize];
                *slot = Some(match *slot {
                    None => p,
                    Some(q) => q.min(p),
                });
            }
        }
        parent
    }
}

// ---------------------------------------------------------------------------
// 覆盖性核验(RXS-0350 L2/L4;fail-closed typed Err)
// ---------------------------------------------------------------------------

/// selection cut 覆盖性违例(机器核验输出;RED 臂判据面)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CutCoverageError {
    /// 空洞:某叶簇未被任何选中簇覆盖(根→叶路径无选中簇)。
    Hole {
        /// 首个未被覆盖的叶簇(局部号)。
        leaf: u32,
    },
    /// 重叠:祖先与后代同选(或同簇重复选中)——同一 LOD 区域被表示两次。
    /// 注:同组多父簇(builder 组级共享子链接)同选**非**重叠——它们按
    /// 三角形划分同组几何,展开别名是链接粒度而非几何重叠(与离线
    /// `ClusterDag::expand_to_leaves` + dedup 参照语义对齐)。
    Overlap {
        /// 祖先簇(局部号;重复选中时 = 后代簇号)。
        ancestor: u32,
        /// 后代簇(局部号)。
        descendant: u32,
    },
    /// cut 引用了 DAG 外的簇号。
    UnknownCluster {
        /// 越界簇号。
        id: u32,
    },
}

impl std::fmt::Display for CutCoverageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            CutCoverageError::Hole { leaf } => {
                write!(f, "selection cut 空洞:叶簇 {leaf} 未被覆盖")
            }
            CutCoverageError::Overlap {
                ancestor,
                descendant,
            } => {
                write!(
                    f,
                    "selection cut 重叠:簇 {ancestor} 与其后代 {descendant} 同选"
                )
            }
            CutCoverageError::UnknownCluster { id } => {
                write!(f, "selection cut 引用未知簇 {id}")
            }
        }
    }
}

impl std::error::Error for CutCoverageError {}

/// 覆盖性机器核验(RXS-0350 L2,fail-closed;首处违例即 typed `Err`):
/// - **无重叠**:无祖先-后代同选、无重复选中;
/// - **无空洞**:选中集叶展开**并集** = 全叶集(组级共享子链接别名不重复计,
///   与离线参照 `lod_cut_coverage_exact` 的 sort+dedup 语义逐字对齐)。
pub fn verify_cut_coverage(mesh: &MeshDagView<'_>, cut: &[u32]) -> Result<(), CutCoverageError> {
    let n = mesh.records.len();
    let mut in_cut = vec![false; n];
    for &c in cut {
        if c as usize >= n {
            return Err(CutCoverageError::UnknownCluster { id: c });
        }
        if in_cut[c as usize] {
            return Err(CutCoverageError::Overlap {
                ancestor: c,
                descendant: c,
            });
        }
        in_cut[c as usize] = true;
    }
    // 重叠:祖先-后代同选(可达性沿子链接;同组兄弟互不可达 ⇒ 不误判)。
    for &c in cut {
        let mut stack: Vec<u32> = mesh.children_of(c).to_vec();
        while let Some(d) = stack.pop() {
            if in_cut[d as usize] {
                return Err(CutCoverageError::Overlap {
                    ancestor: c,
                    descendant: d,
                });
            }
            stack.extend_from_slice(mesh.children_of(d));
        }
    }
    // 空洞:叶展开并集必须覆盖全部叶簇。
    let mut covered = vec![false; n];
    for &c in cut {
        for leaf in mesh.expand_to_leaves(&[c]) {
            covered[leaf as usize] = true;
        }
    }
    for leaf in mesh.leaves() {
        if !covered[leaf as usize] {
            return Err(CutCoverageError::Hole { leaf });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 当帧误差驱动 selection cut 生产(RXS-0350 L2/L5)
// ---------------------------------------------------------------------------

/// 当帧屏幕空间误差驱动 LOD cut(实例世界空间;与 `cluster_cull` 关 3 同一判据:
/// 自身误差投影 < 阈值 且 父级误差投影 ≥ 阈值,互补边界恰成 cut)。
///
/// **不含**视锥/背面锥关——覆盖性不变式定义在 LOD 维(可见性门由产出侧逐
/// 元素标记,见 [`VisibleClusterEntry::visible`])。输出局部簇号**升序**
/// (canonical 序,双跑逐位一致)。本函数即 L5「运行时误差驱动」证据面:输入
/// 相机逐帧变化 ⇒ cut 逐帧重算,静态 LOD cut 无法经本函数产生。
pub fn select_lod_cut(
    mesh: &MeshDagView<'_>,
    transform: &[[f32; 4]; 3],
    cam: &CullCamera,
) -> Vec<u32> {
    let mut out = Vec::new();
    for (i, c) in mesh.records.iter().enumerate() {
        let (center_w, _, scale) = world_sphere(transform, c);
        let d = dist3(center_w, cam.cam_pos);
        let self_px = cam.projected_error_px(c.error * scale, d);
        let parent_px = cam.projected_error_px(c.parent_error * scale, d);
        if self_px < cam.error_threshold_px && parent_px >= cam.error_threshold_px {
            out.push(i as u32);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// 未驻留页父簇兜底(RXS-0350 L3;沿 G8.4 迟到页降级语义不重定)
// ---------------------------------------------------------------------------

/// 父簇兜底 evidence(可机核;进 provenance digest)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FallbackRecord {
    /// 实例下标。
    pub instance: u32,
    /// 原始选中簇(局部号;页未驻留)。
    pub original: u32,
    /// 兜底簇(局部号;首个驻留祖先,或根最后兜底)。
    pub replacement: u32,
    /// 未驻留页 id。
    pub missing_page: u32,
    /// 上行步数(0 = 原簇即根,根最后兜底)。
    pub ascent_steps: u32,
    /// 是否以根最后兜底(祖先链上无驻留页)。
    pub hit_root: bool,
}

/// 页驻留 evidence(产出期实际裁决过的页;进 provenance digest)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PageResidency {
    /// 页 id。
    pub page: u32,
    /// 裁决时刻驻留性。
    pub resident: bool,
}

/// 对单实例 cut 应用页兜底。`resident` = 驻留判定(输入快照的纯函数,
/// 同输入同输出)。返回 (新 cut〔局部号升序去重〕, 兜底 evidence, 驻留 evidence)。
///
/// 不变式:输入 cut 已通过 [`verify_cut_coverage`] 时,输出 cut 亦通过
/// (同组兄弟随祖先替换同步撤出);调用方仍须复核(fail-closed 双保险)。
pub fn apply_page_fallback(
    mesh: &MeshDagView<'_>,
    instance: u32,
    cut: &[u32],
    resident: &dyn Fn(u32) -> bool,
) -> (Vec<u32>, Vec<FallbackRecord>, Vec<PageResidency>) {
    let parents = mesh.min_parents();
    let mut evidence = Vec::new();
    let mut residency: Vec<PageResidency> = Vec::new();
    let note = |page: u32, r: bool, residency: &mut Vec<PageResidency>| {
        let rec = PageResidency { page, resident: r };
        if !residency.contains(&rec) {
            residency.push(rec);
        }
    };
    let mut replacements: Vec<u32> = Vec::new();
    for &c in cut {
        let page = mesh.records[c as usize].page_id;
        let r = resident(page);
        note(page, r, &mut residency);
        if r {
            continue;
        }
        // 沿父链上行至首个驻留祖先;根 = 最后兜底。
        let mut cur = c;
        let mut steps = 0u32;
        let replacement = loop {
            match parents[cur as usize] {
                None => break cur, // 根,最后兜底
                Some(p) => {
                    let ppage = mesh.records[p as usize].page_id;
                    let pr = resident(ppage);
                    note(ppage, pr, &mut residency);
                    steps += 1;
                    if pr {
                        break p;
                    }
                    cur = p;
                }
            }
        };
        if replacement != c {
            evidence.push(FallbackRecord {
                instance,
                original: c,
                replacement,
                missing_page: page,
                ascent_steps: steps,
                hit_root: parents[replacement as usize].is_none(),
            });
            if !replacements.contains(&replacement) {
                replacements.push(replacement);
            }
        }
    }
    if replacements.is_empty() {
        return (cut.to_vec(), evidence, residency);
    }
    // 撤出全部替换祖先的后代成员(含原始未驻留簇与同组兄弟),保 cut 性质。
    let rep_leaves: Vec<Vec<u32>> = replacements
        .iter()
        .map(|&r| mesh.expand_to_leaves(&[r]))
        .collect();
    let mut out: Vec<u32> = Vec::new();
    for &c in cut {
        let dominated = rep_leaves.iter().any(|leaves| {
            let own = mesh.expand_to_leaves(&[c]);
            !own.is_empty() && own.iter().all(|l| leaves.contains(l))
        });
        if !dominated {
            out.push(c);
        }
    }
    for &r in &replacements {
        out.push(r);
    }
    out.sort_unstable();
    out.dedup();
    (out, evidence, residency)
}

// ---------------------------------------------------------------------------
// VisibleClusterSet 载荷与产出(RXS-0350 L1;RXS-0352 单源真相载体)
// ---------------------------------------------------------------------------

/// 可见簇元素(RXS-0350 L1 冻结载荷 = `cluster` + `lod_level` + `skin_version`
/// + `instance`〔变换 id = 实例变换表行〕;`page_id`/`visible` 为运行时辅助字段)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisibleClusterEntry {
    /// 全局簇 stable id(扁平 `ClusterRecord` 池下标 = 实例 `cluster_offset` + 局部号)。
    pub cluster: u32,
    /// 实例下标 = 变换 id(G8 场景模型:实例行即变换表行)。
    pub instance: u32,
    /// DAG 层号(0 = 叶层)。
    pub lod_level: u32,
    /// 蒙皮版本(skin cache 更新计数;0 = 非蒙皮簇)。
    pub skin_version: u32,
    /// 辅助:流送页 id(驻留联动 evidence 面)。
    pub page_id: u32,
    /// 辅助:可见性标记(视锥 + 背面锥两关;LOD cut 本身不含可见性门)。
    pub visible: bool,
}

/// 当帧可见性唯一事实源(M93 产物 + M95 载体)。provenance digest = 内容寻址
/// sha256(`frame_serial` ‖ 元素规范字节 ‖ 兜底 evidence ‖ 驻留 evidence)。
#[derive(Debug, Clone, PartialEq)]
pub struct VisibleClusterSet {
    /// 帧序列号(确定性输入;同输入序列双跑 digest 逐位一致,旁路重算必异)。
    pub frame_serial: u64,
    /// 选中簇集(可见实例序 × 局部簇号升序,canonical)。
    pub entries: Vec<VisibleClusterEntry>,
    /// 页驻留 evidence(产出期实际裁决的页;页号升序)。
    pub residency: Vec<PageResidency>,
    /// 父簇兜底链(evidence;空 = 全部驻留直选)。
    pub fallback: Vec<FallbackRecord>,
    /// provenance digest(三喂携带;帧末校验锚)。
    pub provenance_digest: [u8; 32],
}

/// 产出错误(fail-closed;无 UB)。
#[derive(Debug, Clone, PartialEq)]
pub enum VisibleSetError {
    /// DAG 拓扑非法。
    Topology(&'static str),
    /// 覆盖性破坏(含兜底后复核)。
    Coverage(CutCoverageError),
    /// 实例簇段与 mesh DAG 不匹配(`cluster_count` ≠ DAG 簇数)。
    InstanceMeshMismatch {
        /// 违例实例下标。
        instance: u32,
    },
    /// 蒙皮版本表短于 DAG 簇数(非空时)。
    SkinVersionTableShort {
        /// 表长。
        table: usize,
        /// DAG 簇数。
        clusters: usize,
    },
}

impl std::fmt::Display for VisibleSetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VisibleSetError::Topology(d) => write!(f, "DAG 拓扑非法:{d}"),
            VisibleSetError::Coverage(e) => write!(f, "selection cut 覆盖性破坏:{e}"),
            VisibleSetError::InstanceMeshMismatch { instance } => {
                write!(f, "实例 {instance} 簇段与 mesh DAG 不匹配")
            }
            VisibleSetError::SkinVersionTableShort { table, clusters } => {
                write!(f, "蒙皮版本表长 {table} < DAG 簇数 {clusters}")
            }
        }
    }
}

impl std::error::Error for VisibleSetError {}

impl From<CutCoverageError> for VisibleSetError {
    fn from(e: CutCoverageError) -> Self {
        VisibleSetError::Coverage(e)
    }
}

/// 产出 `VisibleClusterSet`(每可见实例:当帧误差 cut → 覆盖性核验 → 页兜底
/// → 复核 → 可见性标记;fail-closed,任一步破坏即 typed `Err`)。
///
/// - `mesh`:单 mesh DAG(实例 `cluster_count` 必须等于 DAG 簇数,共享拓扑);
/// - `resident_pages`:帧驻留页快照(线性判定;同输入同输出);
/// - `skin_versions`:局部簇号 → 蒙皮版本(空切片 = 全 0 非蒙皮);
/// - 覆盖性机器核验在产出内**强制执行**(RXS-0350 L2 fail-closed 字面)。
pub fn produce_visible_cluster_set(
    mesh: &MeshDagView<'_>,
    instances: &[InstanceRecord],
    visible_instances: &[u32],
    cam: &CullCamera,
    frame_serial: u64,
    resident_pages: &[u32],
    skin_versions: &[u32],
) -> Result<VisibleClusterSet, VisibleSetError> {
    if !skin_versions.is_empty() && skin_versions.len() < mesh.records.len() {
        return Err(VisibleSetError::SkinVersionTableShort {
            table: skin_versions.len(),
            clusters: mesh.records.len(),
        });
    }
    let frustum = cam.frustum();
    let resident = |page: u32| resident_pages.contains(&page);
    let mut entries = Vec::new();
    let mut fallback = Vec::new();
    let mut residency: Vec<PageResidency> = Vec::new();
    for &vi in visible_instances {
        let inst = instances
            .get(vi as usize)
            .ok_or(VisibleSetError::Topology("实例下标越界"))?;
        if inst.cluster_count as usize != mesh.records.len() {
            return Err(VisibleSetError::InstanceMeshMismatch { instance: vi });
        }
        let cut = select_lod_cut(mesh, &inst.transform, cam);
        verify_cut_coverage(mesh, &cut)?; // L2 fail-closed(兜底前)
        let (cut, fb, res) = apply_page_fallback(mesh, vi, &cut, &resident);
        verify_cut_coverage(mesh, &cut)?; // L2 fail-closed(兜底后复核)
        fallback.extend_from_slice(&fb);
        for r in res {
            if !residency.contains(&r) {
                residency.push(r);
            }
        }
        for local in cut {
            let c = &mesh.records[local as usize];
            let (center_w, radius_w, _) = world_sphere(&inst.transform, c);
            // 可见性标记 = 视锥球 + 背面锥(与 cluster_cull 关 1/2 同口径)。
            let mut visible = frustum.contains_sphere(center_w, radius_w);
            if visible && c.cone_cutoff < 1.0 {
                let to = sub3(center_w, cam.cam_pos);
                let d2 = dot3(to, to);
                if d2 > 1e-12 {
                    let d = d2.sqrt();
                    let view = [to[0] / d, to[1] / d, to[2] / d];
                    if let Some(axis_w) = transform_dir_normalized(&inst.transform, c.cone_axis)
                        && dot3(view, axis_w) >= c.cone_cutoff
                    {
                        visible = false;
                    }
                }
            }
            entries.push(VisibleClusterEntry {
                cluster: inst.cluster_offset + local,
                instance: vi,
                lod_level: mesh.nodes[local as usize].level,
                skin_version: skin_versions.get(local as usize).copied().unwrap_or(0),
                page_id: c.page_id,
                visible,
            });
        }
    }
    residency.sort_unstable();
    residency.dedup();
    let mut set = VisibleClusterSet {
        frame_serial,
        entries,
        residency,
        fallback,
        provenance_digest: [0u8; 32],
    };
    set.provenance_digest = compute_provenance_digest(&set);
    Ok(set)
}

/// 规范字节(content-addressed preimage;域分离 + 定长 LE 逐字段拼接)。
fn canonical_bytes(set: &VisibleClusterSet) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"VCS1\0");
    out.extend_from_slice(&set.frame_serial.to_le_bytes());
    out.extend_from_slice(&(set.entries.len() as u32).to_le_bytes());
    for e in &set.entries {
        out.extend_from_slice(&e.cluster.to_le_bytes());
        out.extend_from_slice(&e.instance.to_le_bytes());
        out.extend_from_slice(&e.lod_level.to_le_bytes());
        out.extend_from_slice(&e.skin_version.to_le_bytes());
        out.extend_from_slice(&e.page_id.to_le_bytes());
        out.push(u8::from(e.visible));
    }
    out.extend_from_slice(&(set.fallback.len() as u32).to_le_bytes());
    for f in &set.fallback {
        out.extend_from_slice(&f.instance.to_le_bytes());
        out.extend_from_slice(&f.original.to_le_bytes());
        out.extend_from_slice(&f.replacement.to_le_bytes());
        out.extend_from_slice(&f.missing_page.to_le_bytes());
        out.extend_from_slice(&f.ascent_steps.to_le_bytes());
        out.push(u8::from(f.hit_root));
    }
    out.extend_from_slice(&(set.residency.len() as u32).to_le_bytes());
    for r in &set.residency {
        out.extend_from_slice(&r.page.to_le_bytes());
        out.push(u8::from(r.resident));
    }
    out
}

/// provenance digest(内容寻址;`frame_serial` 混入 ⇒ 同内容旁路重算必异)。
pub fn compute_provenance_digest(set: &VisibleClusterSet) -> [u8; 32] {
    sha256::digest(&canonical_bytes(set))
}

impl VisibleClusterSet {
    /// 可见元素迭代(条目下标 = 光栅 cluster27 序)。
    pub fn visible_entries(&self) -> impl Iterator<Item = (u32, &VisibleClusterEntry)> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.visible)
            .map(|(i, e)| (i as u32, e))
    }

    /// 可见元素计数(evidence `visible_clusters` 埋点源)。
    pub fn visible_count(&self) -> u32 {
        self.entries.iter().filter(|e| e.visible).count() as u32
    }

    /// M95 光栅喂:VisBuffer cluster27 下标表(可见项位置序;provenance 随喂)。
    pub fn feed_raster(&self) -> RasterFeed {
        RasterFeed {
            source: self.provenance_digest,
            entry_indices: self.visible_entries().map(|(i, _)| i).collect(),
        }
    }

    /// M95 RT 喂:BLAS 拼装输入数组——selection 输出**直接派生**的
    /// (instance, cluster) 对(RXS-0352 L1:禁独立再算可见性)。
    pub fn feed_rt(&self) -> RtFeed {
        RtFeed {
            source: self.provenance_digest,
            blas_input: self
                .visible_entries()
                .map(|(_, e)| VisibleCluster {
                    instance: e.instance,
                    cluster: e.cluster,
                })
                .collect(),
        }
    }

    /// M95 VSM 喂:阴影深度光栅簇列表(同一可见集;灯光视角 selection 归 VSM
    /// 页标记既有面,簇列表不独立再算)。
    pub fn feed_vsm(&self) -> VsmFeed {
        VsmFeed {
            source: self.provenance_digest,
            depth_clusters: self
                .visible_entries()
                .map(|(_, e)| VisibleCluster {
                    instance: e.instance,
                    cluster: e.cluster,
                })
                .collect(),
        }
    }
}

// ---------------------------------------------------------------------------
// M95 一份三喂与帧末 provenance 校验(RXS-0352 L1/L3)
// ---------------------------------------------------------------------------

/// 消费方标识(校验错误归属面)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Consumer {
    /// 光栅(VisBuffer)。
    Raster,
    /// RT(BLAS 拼装输入)。
    Rt,
    /// VSM(阴影深度光栅)。
    Vsm,
}

/// 光栅消费喂(VisBuffer cluster27 下标序;`source` = 来源 set 的 digest)。
#[derive(Debug, Clone, PartialEq)]
pub struct RasterFeed {
    /// 来源 provenance digest。
    pub source: [u8; 32],
    /// 可见元素下标(`VisibleClusterSet::entries` 位置;VisBuffer cluster27 序)。
    pub entry_indices: Vec<u32>,
}

/// RT 消费喂(当帧 BLAS 拼装输入数组;selection 输出直接派生)。
#[derive(Debug, Clone, PartialEq)]
pub struct RtFeed {
    /// 来源 provenance digest。
    pub source: [u8; 32],
    /// (instance, cluster) 对(可见项,canonical 序)。
    pub blas_input: Vec<VisibleCluster>,
}

/// VSM 消费喂(阴影深度光栅簇列表;同一可见集)。
#[derive(Debug, Clone, PartialEq)]
pub struct VsmFeed {
    /// 来源 provenance digest。
    pub source: [u8; 32],
    /// (instance, cluster) 对(可见项,canonical 序)。
    pub depth_clusters: Vec<VisibleCluster>,
}

/// provenance 校验违例(旁路单源真相 RED 臂判据面,硬门 R-G9-8)。
#[derive(Debug, Clone, PartialEq)]
pub enum ProvenanceError {
    /// 消费方输入 digest ≠ VisibleClusterSet digest(来源链断裂)。
    Mismatch {
        /// 违例消费方。
        consumer: Consumer,
        /// 期望(权威 set digest)。
        expected: [u8; 32],
        /// 实得(消费方携带)。
        got: [u8; 32],
    },
}

impl std::fmt::Display for ProvenanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProvenanceError::Mismatch {
                consumer,
                expected: _,
                got: _,
            } => write!(
                f,
                "消费方 {consumer:?} provenance digest 失配(旁路单源真相)"
            ),
        }
    }
}

impl std::error::Error for ProvenanceError {}

/// 帧末 provenance 一致性校验(RXS-0352 L3/L6):三方消费喂的 source digest
/// 必须与权威 `VisibleClusterSet` digest **精确一致**;任一失配 = typed `Err`
/// (旁路 variant 判 RED;fail-closed,首违例即报)。
pub fn verify_frame_provenance(
    set: &VisibleClusterSet,
    raster: &RasterFeed,
    rt: &RtFeed,
    vsm: &VsmFeed,
) -> Result<(), ProvenanceError> {
    for (consumer, got) in [
        (Consumer::Raster, raster.source),
        (Consumer::Rt, rt.source),
        (Consumer::Vsm, vsm.source),
    ] {
        if got != set.provenance_digest {
            return Err(ProvenanceError::Mismatch {
                consumer,
                expected: set.provenance_digest,
                got,
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 内部工具
// ---------------------------------------------------------------------------

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn dist3(a: [f32; 3], b: [f32; 3]) -> f32 {
    dot3(sub3(a, b), sub3(a, b)).sqrt()
}

// ---------------------------------------------------------------------------
// 单测
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::gpu_scene::NO_PARENT;

    /// 精确系数相机(与 cull.rs 单测同型:90° 视锥、m11 = 1、投影系数 H/2)。
    fn exact_cam(screen_h: f32, threshold: f32) -> CullCamera {
        CullCamera {
            view_proj: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, -1.0, -0.2],
                [0.0, 0.0, -1.0, 0.0],
            ],
            cam_pos: [0.0, 0.0, 0.0],
            screen_height_px: screen_h,
            error_threshold_px: threshold,
        }
    }

    fn inst_at(t: [f32; 3], cluster_offset: u32, cluster_count: u32) -> InstanceRecord {
        InstanceRecord {
            transform: [
                [1.0, 0.0, 0.0, t[0]],
                [0.0, 1.0, 0.0, t[1]],
                [0.0, 0.0, 1.0, t[2]],
            ],
            cluster_offset,
            cluster_count,
            material_id: 0,
            flags: 0,
            aabb_min: [t[0] - 2.0, t[1] - 2.0, t[2] - 2.0],
            mesh_id: 0,
            aabb_max: [t[0] + 2.0, t[1] + 2.0, t[2] + 2.0],
            reserved: NO_PARENT,
        }
    }

    fn cluster(error: f32, parent_error: f32, page_id: u32) -> ClusterRecord {
        ClusterRecord {
            center: [0.0; 3],
            radius: 0.5,
            cone_axis: [0.0; 3],
            cone_cutoff: 2.0, // 锥剔禁用(隔离可见性门,聚焦 LOD cut 面)
            error,
            parent_error,
            vertex_offset: 0,
            triangle_offset: 0,
            vertex_count: 0,
            triangle_count: 0,
            page_id,
            reserved: 0,
        }
    }

    /// conformance/virtual_geometry/accept/visible_cluster_set_valid_cut.rx 数据面
    /// DAG:R → {A, B};A → {A0, A1}。局部号:A0=0, A1=1, B=2, A=3, R=4。
    /// 误差:叶 0;A 0.5;R 2.0;parent_error:A0/A1=0.5、B=2.0、A=2.0、R=+∞。
    fn corpus_dag() -> (Vec<ClusterRecord>, Vec<DagNodeRec>, Vec<u32>) {
        let records = vec![
            cluster(0.0, 0.5, 1),
            cluster(0.0, 0.5, 1),
            cluster(0.0, 2.0, 2),
            cluster(0.5, 2.0, 3),
            cluster(2.0, f32::INFINITY, 4),
        ];
        let nodes = vec![
            DagNodeRec {
                first_child: 0,
                child_count: 0,
                level: 0,
            },
            DagNodeRec {
                first_child: 0,
                child_count: 0,
                level: 0,
            },
            DagNodeRec {
                first_child: 0,
                child_count: 0,
                level: 0,
            },
            DagNodeRec {
                first_child: 0,
                child_count: 2,
                level: 1,
            },
            DagNodeRec {
                first_child: 2,
                child_count: 2,
                level: 2,
            },
        ];
        let children = vec![0, 1, 3, 2];
        (records, nodes, children)
    }

    fn corpus_mesh<'a>(
        r: &'a [ClusterRecord],
        n: &'a [DagNodeRec],
        c: &'a [u32],
    ) -> MeshDagView<'a> {
        MeshDagView::new(r, n, c).expect("corpus DAG 拓扑合法")
    }

    /// 生成 DAG(平衡二叉树,`leaf_count` 叶,误差逐级 ×2,同心;确定性)。
    fn gen_binary_dag(leaf_count: usize) -> (Vec<ClusterRecord>, Vec<DagNodeRec>, Vec<u32>) {
        assert!(leaf_count >= 2);
        let mut records: Vec<ClusterRecord> = Vec::new();
        let mut nodes: Vec<DagNodeRec> = Vec::new();
        let mut children: Vec<u32> = Vec::new();
        // 叶层:error 0,parent_error 0.5。
        for _ in 0..leaf_count {
            records.push(cluster(0.0, 0.5, 0));
            nodes.push(DagNodeRec {
                first_child: 0,
                child_count: 0,
                level: 0,
            });
        }
        let mut level_start = 0u32;
        let mut level_count = leaf_count;
        let mut level = 1u32;
        let mut error = 0.5f32;
        while level_count > 1 {
            let parent_count = level_count.div_ceil(2);
            let parent_start = records.len() as u32;
            let parent_error = if parent_count == 1 {
                f32::INFINITY
            } else {
                error * 2.0
            };
            for p in 0..parent_count {
                let first_child = children.len() as u32;
                let c0 = level_start + (2 * p) as u32;
                children.push(c0);
                if 2 * p + 1 < level_count {
                    children.push(c0 + 1);
                }
                let child_count = children.len() as u32 - first_child;
                records.push(cluster(error, parent_error, 0));
                nodes.push(DagNodeRec {
                    first_child,
                    child_count,
                    level,
                });
            }
            level_start = parent_start;
            level_count = parent_count;
            level += 1;
            error *= 2.0;
        }
        (records, nodes, children)
    }

    // ———— M93 RXS-0350 ————

    //@ spec: RXS-0350
    #[test]
    fn valid_cut_accept_corpus_dag() {
        let (r, n, c) = corpus_dag();
        let mesh = corpus_mesh(&r, &n, &c);
        // d=100:0.5·500/100 = 2.5 ≥ 1 ⇒ 叶父检过 ⇒ 合法 cut = {A0, A1, B}。
        let cam = exact_cam(1000.0, 1.0);
        let inst = [inst_at([0.0, 0.0, -100.0], 0, 5)];
        let set = produce_visible_cluster_set(&mesh, &inst, &[0], &cam, 7, &[1, 2, 3, 4], &[])
            .expect("合法 cut 必须产出");
        let ids: Vec<u32> = set.entries.iter().map(|e| e.cluster).collect();
        assert_eq!(ids, vec![0, 1, 2], "语料合法 cut = {{A0, A1, B}}");
        // 载荷:lod level + 蒙皮版本(0) + 变换 id(= 实例)。
        assert_eq!(
            set.entries.iter().map(|e| e.lod_level).collect::<Vec<_>>(),
            vec![0, 0, 0]
        );
        assert!(set.entries.iter().all(|e| e.skin_version == 0));
        assert!(set.entries.iter().all(|e| e.instance == 0 && e.visible));
        assert!(set.fallback.is_empty());
        // 中距 d=500:0.5·500/500 = 0.5 < 1、2·500/500 = 2 ≥ 1 ⇒ cut = {A, B}。
        let inst2 = [inst_at([0.0, 0.0, -500.0], 0, 5)];
        let set2 = produce_visible_cluster_set(&mesh, &inst2, &[0], &cam, 7, &[1, 2, 3, 4], &[])
            .expect("中距 cut 必须产出");
        let ids2: Vec<u32> = set2.entries.iter().map(|e| e.cluster).collect();
        assert_eq!(ids2, vec![2, 3], "中距 cut = {{A, B}}(canonical 升序)");
        assert_eq!(
            set2.entries.iter().map(|e| e.lod_level).collect::<Vec<_>>(),
            vec![0, 1],
            "B 为叶层、A 为层 1(载荷 lod_level 源)"
        );
    }

    //@ spec: RXS-0350
    #[test]
    fn hole_and_overlap_injected_red() {
        // conformance/virtual_geometry/reject/selection_cut_hole_injected.rx 数据面。
        let (r, n, c) = corpus_dag();
        let mesh = corpus_mesh(&r, &n, &c);
        // 合法对照:{A0, A1, B} 通过。
        verify_cut_coverage(&mesh, &[0, 1, 2]).expect("合法 cut");
        // 空洞注入:{A0, B} —— 路径 R→A→A1 无选中簇。
        assert_eq!(
            verify_cut_coverage(&mesh, &[0, 2]),
            Err(CutCoverageError::Hole { leaf: 1 })
        );
        // 重叠注入:{A, A0, B} —— A 与 A0 父子同选。
        assert_eq!(
            verify_cut_coverage(&mesh, &[0, 2, 3]),
            Err(CutCoverageError::Overlap {
                ancestor: 3,
                descendant: 0
            })
        );
        // 重复选中同簇 = 重叠。
        assert!(matches!(
            verify_cut_coverage(&mesh, &[0, 0, 1, 2]),
            Err(CutCoverageError::Overlap { .. })
        ));
        // 未知簇引用。
        assert_eq!(
            verify_cut_coverage(&mesh, &[0, 1, 9]),
            Err(CutCoverageError::UnknownCluster { id: 9 })
        );
        // 空 cut = 全空洞。
        assert!(matches!(
            verify_cut_coverage(&mesh, &[]),
            Err(CutCoverageError::Hole { .. })
        ));
    }

    //@ spec: RXS-0350
    #[test]
    fn cut_coverage_generated_dag_sweep() {
        // 生成 DAG(8 叶平衡树):任意相机/阈值下 produce 内嵌核验必须通过,
        // 且 cut 恰为完整层带(同心 + 误差逐级 ×2 的构造性质)。
        let (r, n, c) = gen_binary_dag(8);
        let mesh = corpus_mesh(&r, &n, &c);
        for &d in &[50.0f32, 100.0, 250.0, 500.0, 1000.0, 2000.0, 100000.0] {
            let cam = exact_cam(1000.0, 1.0);
            let inst = [inst_at([0.0, 0.0, -d], 0, r.len() as u32)];
            let set = produce_visible_cluster_set(&mesh, &inst, &[0], &cam, 1, &[0], &[])
                .unwrap_or_else(|e| panic!("d={d} 产出失败:{e}"));
            assert!(!set.entries.is_empty(), "d={d} cut 为空");
            let levels: Vec<u32> = set.entries.iter().map(|e| e.lod_level).collect();
            assert!(
                levels.windows(2).all(|w| w[0] == w[1]),
                "d={d} 同心生成 DAG cut 应为单一层带:{levels:?}"
            );
        }
    }

    //@ spec: RXS-0350
    #[test]
    fn parent_fallback_on_missing_page_and_restore() {
        let (r, n, c) = corpus_dag();
        let mesh = corpus_mesh(&r, &n, &c);
        let cam = exact_cam(1000.0, 1.0);
        let inst = [inst_at([0.0, 0.0, -100.0], 0, 5)];
        // 页 1(A0/A1)强制未驻留 ⇒ 命中父簇 A(页 3)兜底;B(页 2)直选。
        let set = produce_visible_cluster_set(&mesh, &inst, &[0], &cam, 7, &[2, 3, 4], &[])
            .expect("兜底 cut 必须产出");
        let ids: Vec<u32> = set.entries.iter().map(|e| e.cluster).collect();
        assert_eq!(ids, vec![2, 3], "兜底后 cut = {{B, A}}");
        // evidence 可机核:两条兜底记录(A0→A、A1→A),missing_page = 1,ascent = 1。
        assert_eq!(set.fallback.len(), 2);
        for f in &set.fallback {
            assert_eq!(f.replacement, 3);
            assert_eq!(f.missing_page, 1);
            assert_eq!(f.ascent_steps, 1);
            assert!(!f.hit_root);
        }
        assert!(set.fallback.iter().map(|f| f.original).eq([0, 1]));
        // 驻留 evidence 覆盖裁决过的页(1 未驻留、2/3 驻留)。
        assert!(set.residency.contains(&PageResidency {
            page: 1,
            resident: false
        }));
        assert!(set.residency.contains(&PageResidency {
            page: 2,
            resident: true
        }));
        assert!(set.residency.contains(&PageResidency {
            page: 3,
            resident: true
        }));
        // 页到达(1 转驻留)⇒ 转正为正确内容 {A0, A1, B},兜底链清空。
        let set2 = produce_visible_cluster_set(&mesh, &inst, &[0], &cam, 8, &[1, 2, 3, 4], &[])
            .expect("页到达后转正");
        let ids2: Vec<u32> = set2.entries.iter().map(|e| e.cluster).collect();
        assert_eq!(ids2, vec![0, 1, 2]);
        assert!(set2.fallback.is_empty());
    }

    //@ spec: RXS-0350
    #[test]
    fn root_fallback_last_resort() {
        let (r, n, c) = corpus_dag();
        let mesh = corpus_mesh(&r, &n, &c);
        let cam = exact_cam(1000.0, 1.0);
        let inst = [inst_at([0.0, 0.0, -100.0], 0, 5)];
        // 仅根页 4 驻留:全链上行至根兜底。
        let set = produce_visible_cluster_set(&mesh, &inst, &[0], &cam, 7, &[4], &[])
            .expect("根兜底必须产出");
        let ids: Vec<u32> = set.entries.iter().map(|e| e.cluster).collect();
        assert_eq!(ids, vec![4], "全未驻留 ⇒ 根 R 兜底");
        assert_eq!(set.fallback.len(), 3, "三叶各自登记兜底 evidence");
        assert!(set.fallback.iter().all(|f| f.replacement == 4));
        assert!(set.fallback.iter().any(|f| f.hit_root));
    }

    //@ spec: RXS-0350
    #[test]
    fn double_run_digest_deterministic() {
        let (r, n, c) = corpus_dag();
        let mesh = corpus_mesh(&r, &n, &c);
        let cam = exact_cam(1000.0, 1.0);
        let inst = [inst_at([0.0, 0.0, -100.0], 0, 5)];
        let a = produce_visible_cluster_set(&mesh, &inst, &[0], &cam, 7, &[1, 2, 3, 4], &[])
            .expect("run A");
        let b = produce_visible_cluster_set(&mesh, &inst, &[0], &cam, 7, &[1, 2, 3, 4], &[])
            .expect("run B");
        assert_eq!(a, b, "双跑产物全等(确定性)");
        assert_eq!(a.provenance_digest, b.provenance_digest);
        // 输出 digest golden 全等(RXS-0350 L2 判据面;与 probe evidence 同值)。
        let hex: String = a
            .provenance_digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(
            hex,
            "4001bef36ab04af4f0cf65c59315fe9fc7eeea6cf70a41aba4fd2f78d0276f67",
            "provenance digest golden 漂移"
        );
    }

    // ———— M95 RXS-0352 ————

    //@ spec: RXS-0352
    #[test]
    fn three_feeds_digest_consistent() {
        // conformance/virtual_geometry/accept/single_source_three_consumers.rx 数据面。
        let (r, n, c) = corpus_dag();
        let mesh = corpus_mesh(&r, &n, &c);
        let cam = exact_cam(1000.0, 1.0);
        let inst = [inst_at([0.0, 0.0, -100.0], 0, 5)];
        let set = produce_visible_cluster_set(&mesh, &inst, &[0], &cam, 7, &[1, 2, 3, 4], &[])
            .expect("产出");
        let raster = set.feed_raster();
        let rt = set.feed_rt();
        let vsm = set.feed_vsm();
        verify_frame_provenance(&set, &raster, &rt, &vsm).expect("三喂 digest 精确一致");
        // 三喂内容派生自同一可见集(禁独立再算)。
        assert_eq!(raster.entry_indices, vec![0, 1, 2]);
        let expect: Vec<VisibleCluster> = (0..3u32)
            .map(|cluster| VisibleCluster {
                instance: 0,
                cluster,
            })
            .collect();
        assert_eq!(rt.blas_input, expect);
        assert_eq!(vsm.depth_clusters, expect);
    }

    //@ spec: RXS-0352
    #[test]
    fn bypass_single_source_variant_red() {
        // conformance/virtual_geometry/reject/bypass_single_source_variant.rx 数据面:
        // RT 腿独立再算可见性 ⇒ 即使内容与权威集全等,serial 不同 ⇒ digest 必异。
        let (r, n, c) = corpus_dag();
        let mesh = corpus_mesh(&r, &n, &c);
        let cam = exact_cam(1000.0, 1.0);
        let inst = [inst_at([0.0, 0.0, -100.0], 0, 5)];
        let authoritative =
            produce_visible_cluster_set(&mesh, &inst, &[0], &cam, 7, &[1, 2, 3, 4], &[])
                .expect("权威 set");
        let bypass = produce_visible_cluster_set(&mesh, &inst, &[0], &cam, 8, &[1, 2, 3, 4], &[])
            .expect("旁路重算 set(内容全等)");
        // 出图相似(甚至元素全等)不是判据:内容逐元素相等。
        assert_eq!(authoritative.entries, bypass.entries);
        assert_ne!(
            authoritative.provenance_digest, bypass.provenance_digest,
            "serial 混入 ⇒ 同内容旁路 digest 必异"
        );
        let raster = authoritative.feed_raster();
        let vsm = authoritative.feed_vsm();
        let rt_bypass = bypass.feed_rt(); // RT 腿旁路
        let err = verify_frame_provenance(&authoritative, &raster, &rt_bypass, &vsm)
            .expect_err("旁路 variant 必须判 RED");
        match err {
            ProvenanceError::Mismatch {
                consumer,
                expected,
                got,
            } => {
                assert_eq!(consumer, Consumer::Rt);
                assert_eq!(expected, authoritative.provenance_digest);
                assert_eq!(got, bypass.provenance_digest);
            }
        }
    }

    //@ spec: RXS-0352
    #[test]
    fn provenance_double_run_deterministic() {
        let (r, n, c) = corpus_dag();
        let mesh = corpus_mesh(&r, &n, &c);
        let cam = exact_cam(1000.0, 1.0);
        let inst = [inst_at([0.0, 0.0, -100.0], 0, 5)];
        let run = || {
            let set = produce_visible_cluster_set(&mesh, &inst, &[0], &cam, 7, &[1, 2, 3, 4], &[])
                .expect("产出");
            (
                set.provenance_digest,
                set.feed_raster(),
                set.feed_rt(),
                set.feed_vsm(),
            )
        };
        let (d1, ra1, rt1, vs1) = run();
        let (d2, ra2, rt2, vs2) = run();
        assert_eq!(d1, d2);
        assert_eq!((ra1, rt1, vs1), (ra2, rt2, vs2), "三喂双跑逐位一致");
    }
}
