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
//!
//! G9.2 M90 深化(RXS-0345;RFC-0022 §4.1;spec/virtual_geometry.md):
//!   - [`build_dag_v2`] = [`build_dag`] 的 typed `Err` 变体——构建产物逐边单调性
//!     机器核验,破坏即 fail-closed typed `Err` 拒录(**不静默继续、不 clamp 修复**);
//!     资产级 builder 入口 [`build_asset_dag`] 额外把单调性破坏的**输入/中间态**映射为
//!     `DagError::NonMonotonicInput`(内部不变量断言经 typed Err 转译,非 panic 泄漏)。
//!   - 蒙皮元数据三字段(最大影响骨数/骨骼索引集/包围体膨胀系数)经
//!     [`SkinWeights`] 逐簇校核进入 [`ClusterDagV2::ext`];骨骼资产缺任一字段 =
//!     typed `Err` 拒录。
//!   - CLAS 离线烘焙输入(三角形簇 + 簇级 AABB)由 [`ClusterDagV2::clas_bake_input`]
//!     按簇导出(三角形簇几何 = 既有顶点/索引段视图,簇级 AABB = 逐位 min/max)。
//!   - [`build_dag`] / [`crate::serialize`] RXGB v1 面 **0-byte 不动**(v1 消费路径
//!     回归 digest 不变;v2 记录面走 [`ClusterDagV2`] 与 RXPL major=2,RXS-0344)。

use rurix_render::graph::types::ClusterRecord;
use std::collections::HashMap;

use crate::cluster::{backface_cone, bounding_sphere, clusterize_tris};
use crate::mesh::{AttrMeshError, AttrTriMesh, TriMesh, TriMeshAttrs};
use crate::vecmath::vdist;

/// 叶层网格顶点反查表(蒙皮元数据逐簇查权重的确定性源):
/// 顶点位置 bits → 叶层全局顶点下标(焊接同位取最小下标,确定性)。
/// 由 [`build_asset_dag`] 在构建期挂载于线程局部,导出前摘除。
#[derive(Debug)]
struct MeshLookup {
    map: HashMap<[u32; 3], u32>,
    positions_len: usize,
}

impl MeshLookup {
    fn of(mesh: &TriMesh) -> Self {
        let mut map = HashMap::with_capacity(mesh.positions.len());
        for (i, p) in mesh.positions.iter().enumerate() {
            map.entry(p.map(f32::to_bits)).or_insert(i as u32);
        }
        Self {
            map,
            positions_len: mesh.positions.len(),
        }
    }
}

thread_local! {
    /// 当前资产级构建的叶层反查表(`build_asset_dag` 挂入,导出前摘除;
    /// RAII 守卫保证 panic 路径同摘)。
    static ACTIVE_MESH_LOOKUP: std::cell::RefCell<Option<MeshLookup>> =
        const { std::cell::RefCell::new(None) };
}

/// 摘除资产级构建上下文(panic 安全:经 [`MeshContextGuard`] 自动调用)。
fn clear_dag_mesh_context() {
    ACTIVE_MESH_LOOKUP.with(|c| *c.borrow_mut() = None);
}

/// 资产级构建上下文 RAII 守卫(Drop 摘除,防 panic 泄漏上下文)。
struct MeshContextGuard;

impl MeshContextGuard {
    fn install(mesh: &TriMesh) -> Self {
        ACTIVE_MESH_LOOKUP.with(|c| *c.borrow_mut() = Some(MeshLookup::of(mesh)));
        Self
    }
}

impl Drop for MeshContextGuard {
    fn drop(&mut self) {
        clear_dag_mesh_context();
    }
}

/// 供 `derive_skin_metadata` 的 DAG 侧视图(查不到 = 字段不可得,
/// 由校核层 typed Err;不暴露线程局部态为公共可变面)。
impl ClusterDag {
    fn leaf_vertex_source_index(&self, position_bits: &[u32; 3]) -> Option<u32> {
        ACTIVE_MESH_LOOKUP.with(|c| {
            c.borrow()
                .as_ref()
                .and_then(|l| l.map.get(position_bits).copied())
        })
    }

    fn mesh_positions_len(&self) -> usize {
        ACTIVE_MESH_LOOKUP.with(|c| c.borrow().as_ref().map_or(0, |l| l.positions_len))
    }
}

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
    /// 叶层三角形 → 源网格三角形 id(与叶层三角形导出序平行,
    /// `len = levels[0].triangle_count`;叶簇第 t 个三角形的源 id =
    /// `leaf_source_tris[record.triangle_offset / 3 + t]`——叶层先导出、
    /// 偏移自 0 连续,故 `triangle_offset / 3` 即叶层三角序号)。
    ///
    /// G31+ #58 消费面(生产管线 LOD cut 属性回查:逐三角 albedo/emission/
    /// tri_mat 按源 id 继承)。**RXGB v1 序列化与 [`canonical_bytes`] 均不含
    /// 本表(0-byte 不动;m90 digest golden 不漂移)**;`read_dag` 产物本表
    /// 为空——属性回查仅内存直构面可用。
    pub leaf_source_tris: Vec<u32>,
}

// ———— G9.2 M90:v2 深化记录面(RXS-0345;v1 64B ClusterRecord 0-byte 不动)————

/// DAG builder fail-closed typed 错误(RXS-0345 §1;无 UB、无静默、无 clamp)。
#[derive(Debug, Clone, PartialEq)]
pub enum DagError {
    /// 逐边单调性破坏(构建产物核验):父簇 `parent` 误差 < 子簇 `child` 误差
    /// (浮点序;误差 NaN 不可比较同判破坏)。
    NonMonotonicEdge {
        /// 父簇 record id。
        parent: u32,
        /// 子簇 record id。
        child: u32,
        /// 父侧误差(parent 边方向 = `parent.error`)。
        parent_error: f32,
        /// 子侧误差。
        child_error: f32,
    },
    /// 资产级输入/中间态单调性破坏(由 `build_asset_dag` 自内部断言转译;
    /// 拒绝 panic 泄漏为构建期事故)。
    NonMonotonicInput {
        /// 破坏点自述(内部转译标签)。
        detail: &'static str,
    },
    /// 蒙皮元数据缺失:骨骼资产某簇缺三字段(最大影响骨数/骨骼索引集/包围体
    /// 膨胀系数)任一面(每簇骨骼权重行缺失或三字段长度不齐)。
    SkinMetadataMissing {
        /// 缺字段簇(record id)。
        cluster: u32,
        /// 缺失面自述。
        detail: &'static str,
    },
    /// 蒙皮元数据不一致:骨骼 id 越界(≥ joint_count)或越簇数。
    SkinMetadataInconsistent {
        /// 簇 record id。
        cluster: u32,
        /// 越界面自述。
        detail: &'static str,
    },
}

impl std::fmt::Display for DagError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DagError::NonMonotonicEdge {
                parent,
                child,
                parent_error,
                child_error,
            } => write!(
                f,
                "DAG 边单调性破坏: parent={parent} child={child} \
                 parent_error={parent_error} < child_error={child_error}"
            ),
            DagError::NonMonotonicInput { detail } => {
                write!(f, "DAG 输入/中间态单调性破坏:{detail}")
            }
            DagError::SkinMetadataMissing { cluster, detail } => {
                write!(f, "簇 {cluster} 蒙皮元数据缺失:{detail}")
            }
            DagError::SkinMetadataInconsistent { cluster, detail } => {
                write!(f, "簇 {cluster} 蒙皮元数据不一致:{detail}")
            }
        }
    }
}

impl std::error::Error for DagError {}

/// 蒙皮元数据(RXS-0345 §3.3 三字段冻结 schema,每簇随页烘焙)。
#[derive(Debug, Clone, PartialEq)]
pub struct ClusterSkinMeta {
    /// 最大影响骨数(0 = 非蒙皮簇;> 0 时 bone_indices/bound_inflation 必须为 Some)。
    pub max_influences: u32,
    /// 骨骼索引集(确定性升序;非蒙皮簇 = None)。
    pub bone_indices: Option<Vec<u32>>,
    /// 蒙皮包围体膨胀系数(Kerbl 保守界所需输入;非蒙皮簇 = None)。
    pub bound_inflation: Option<f32>,
}

impl ClusterSkinMeta {
    /// 非蒙皮簇(三字段零值面)。
    pub fn unskinned() -> Self {
        Self {
            max_influences: 0,
            bone_indices: None,
            bound_inflation: None,
        }
    }
}

/// CLAS 离线烘焙输入(RXS-0345 §3.4;三角形簇 = `ClusterDag::cluster_vertices` /
/// `cluster_triangle` 既有视图,簇级 AABB 见 [`ClusterDagV2::clas_bake_input`])。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClasBakeInput {
    /// 簇级 AABB min(逐位 min/max,非包围球收缩)。
    pub aabb_min: [f32; 3],
    /// 簇级 AABB max。
    pub aabb_max: [f32; 3],
}

/// v2 DAG 深化产物:v1 `ClusterDag` 字段面 0-byte + 逐簇平行扩展表。
#[derive(Debug, Clone, PartialEq)]
pub struct ClusterDagV2 {
    /// v1 DAG(64B ClusterRecord/层级表/数据段,字面不动)。
    pub base: ClusterDag,
    /// 与 `base.records` 等长平行:蒙皮元数据(三字段校核后形态)。
    pub skin: Vec<ClusterSkinMeta>,
    /// 与 `base.records` 等长平行:CLAS 烘焙输入(簇级 AABB)。
    pub clas: Vec<ClasBakeInput>,
}

/// 资产级 builder 输入(G9.2 M90;纯静态网格 = `skinned: None`)。
#[derive(Debug, Clone, Default)]
pub struct DagAsset {
    /// 三角网格(叶层事实源)。
    pub mesh: TriMesh,
    /// 骨骼资产蒙皮权重(仅骨骼资产 = Some;逐顶点 ≤ MAX_BONE_INFLUENCES 对)。
    pub skinned: Option<SkinWeights>,
}

/// 逐顶点骨骼权重(离线烘焙输入面;`joint_count` 为索引集越界校核上界)。
#[derive(Debug, Clone)]
pub struct SkinWeights {
    /// 与 `mesh.positions` 等长;每顶点 (bone_id, weight) ≤ 4 对。
    pub vertex_influences: Vec<Vec<(u32, f32)>>,
    /// 骨骼总数(骨骼索引集元素 < joint_count 才合法)。
    pub joint_count: u32,
}

impl DagAsset {
    /// 纯静态网格资产(非蒙皮;蒙皮三字段按零值面落)。
    pub fn static_mesh(mesh: TriMesh) -> Self {
        Self {
            mesh,
            skinned: None,
        }
    }
}

/// 每顶点最大影响骨数(RXS-0345 上游口径;meshopt 同值)。
pub const MAX_BONE_INFLUENCES: usize = 4;

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
    /// 簇的生成组号(G31+ #98 边界交替:上一层同组产物 = 同胞。叶层无生成
    /// 组——逐簇唯一哨兵(自身 id),任意对互为非同胞 ⇒ 偏置权对全部对同乘
    /// 常数,排序序不变 ≡ 纯共享边加权,legacy 语义 0-漂移)。
    source_group: Vec<u32>,
}

/// 组子网格(合并 + 边界锁定的载体;`pub(crate)`——QEM 加性简化器
/// [`crate::qem`] 同接口消费,G31+ #66 参照臂纪律)。
#[derive(Clone)]
pub(crate) struct SubMesh {
    pub(crate) positions: Vec<[f32; 3]>,
    pub(crate) tris: Vec<[u32; 3]>,
    /// 顶点被组外三角形引用 → 锁定(不许被收缩移除)。
    pub(crate) locked: Vec<bool>,
    /// 面含组边界边(该边被组外三角形共享)→ 收缩禁令标记。
    /// 收缩只改写内部顶点,边界边端点永不移动/合并,故标记全程有效。
    pub(crate) face_on_boundary: Vec<bool>,
}

/// 顶点属性平行表(G31+ #96 构建期中间载体;与所伴随的位置表等长——
/// 层全局属性 / 组局部属性 / 简化产物属性同一形态)。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SubMeshAttrs {
    pub(crate) uv: Vec<[f32; 2]>,
    pub(crate) normal: Option<Vec<[f32; 3]>>,
}

impl SubMeshAttrs {
    /// 空表(normal 在场性随模板;增量填充用)。
    pub(crate) fn empty_like(&self) -> Self {
        Self {
            uv: Vec::new(),
            normal: self.normal.as_ref().map(|_| Vec::new()),
        }
    }

    /// 按下标表 gather(组局部属性抽取:`ids[local] = global`)。
    fn gather(&self, ids: &[u32]) -> Self {
        Self {
            uv: ids.iter().map(|&v| self.uv[v as usize]).collect(),
            normal: self
                .normal
                .as_ref()
                .map(|nm| ids.iter().map(|&v| nm[v as usize]).collect()),
        }
    }

    /// 追加一个顶点的属性(与位置表 push 同步调用)。
    pub(crate) fn push_from(&mut self, src: &SubMeshAttrs, v: usize) {
        self.uv.push(src.uv[v]);
        if let (Some(dst), Some(nm)) = (self.normal.as_mut(), src.normal.as_ref()) {
            dst.push(nm[v]);
        }
    }
}

/// UV 接缝顶点旗标(G31+ #96 保守裂缝纪律):同位置 bits 出现于多个顶点 id
/// = 属性接缝顶点(上游按位置+属性预拆分的两侧拷贝)。两侧独立收缩会产生
/// 几何裂缝——属性链一律锁定接缝顶点(逐位保持;meshopt 位置重映射协动
/// 简化为后续质量档,分界见 #96 交付报告)。
pub(crate) fn attr_seam_flags(positions: &[[f32; 3]]) -> Vec<bool> {
    let mut count: HashMap<[u32; 3], u32> = HashMap::with_capacity(positions.len());
    for p in positions {
        *count.entry(p.map(f32::to_bits)).or_insert(0) += 1;
    }
    positions
        .iter()
        .map(|p| count[&p.map(f32::to_bits)] > 1)
        .collect()
}

/// 组内简化器选择(G31+ #66 加性闭集;默认 = 既有事实源)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SimplifyKind {
    /// 最短边贪心收缩(端点保持;既有事实源——m90 DAG digest golden 锚,
    /// [`build_dag`] 恒走本档,0-byte)。
    #[default]
    ShortestEdge,
    /// QEM 最优位置收缩([`crate::qem`];fold-over 拒绝 + 锁定端逐位保持。
    /// **移动内部顶点** ⇒ 粗簇顶点不再命中叶层蒙皮反查——骨骼资产禁用
    /// (见 [`build_asset_dag_kind`] fail-closed)。生产簇包 bake 消费面)。
    Qem,
}

/// DAG 构建参数(G31+ #66/#98 加性闭集;[`Default`] = 既有事实源逐位 0-byte)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DagBuildParams {
    /// 组内简化器。
    pub simplify: SimplifyKind,
    /// 每组目标簇数(既有事实源 = 4;Nanite 口径 8–32——组越大简化自由度
    /// 越高、stuck 越少,代价 = 流送/兜底粒度变粗)。
    pub group_size: usize,
    /// 非同胞边权偏置(Nanite「边界交替」:上一层同组(同胞)簇对合并权 ×1、
    /// 非同胞 ×16——上层组边界优先并入新组内部,锁定边逐层轮换获得简化
    /// 机会。false = 既有纯共享边加权)。
    pub sibling_bias: bool,
}

impl Default for DagBuildParams {
    fn default() -> Self {
        Self {
            simplify: SimplifyKind::ShortestEdge,
            group_size: GROUP_SIZE,
            sibling_bias: false,
        }
    }
}

impl DagBuildParams {
    /// 生产簇包质量档(G31+ #66/#98:QEM + 8 簇/组 + 边界交替;
    /// g31_cluster_lod_bake 默认档)。
    pub fn quality() -> Self {
        Self {
            simplify: SimplifyKind::Qem,
            group_size: 8,
            sibling_bias: true,
        }
    }
}

/// 网格 → 簇层级 DAG(报告1 §5 P0→P3 的离线半;验收:任意输入得 DAG +
/// 每簇误差包围球 + 层级统计)。**v1 既有面 0-byte 不动**;G9.2 typed `Err`
/// 拒录变体见 [`build_dag_v2`](同一产物,失败经 `DagError` 返回而非 panic)。
pub fn build_dag(mesh: &TriMesh) -> ClusterDag {
    build_dag_params(mesh, &DagBuildParams::default())
}

/// [`build_dag`] 的简化器参数化变体(G31+ #66 加性面:`ShortestEdge` 与
/// [`build_dag`] 逐位同产物;`Qem` = 质量升级臂,单调/覆盖/确定性不变量
/// 同一套导出与单测锚)。
pub fn build_dag_kind(mesh: &TriMesh, kind: SimplifyKind) -> ClusterDag {
    build_dag_params(
        mesh,
        &DagBuildParams {
            simplify: kind,
            ..DagBuildParams::default()
        },
    )
}

/// [`build_dag`] 的全参数化变体(G31+ #66/#98;`Default` 参数与 [`build_dag`]
/// 逐位同产物)。
pub fn build_dag_params(mesh: &TriMesh, params: &DagBuildParams) -> ClusterDag {
    build_dag_impl(mesh, None, params).0
}

/// 簇 DAG 属性链产物(G31+ #96):v1 [`ClusterDag`] 字段面不动 + 与
/// `base.vertices` 等长平行的顶点属性表——粗簇/代理三角自此带真 UV 供
/// 纹理采样消费(G36 侧表 gather 对代理三角 tritex=−1 常量回退的退役前提)。
///
/// **RXGB v1 序列化与 [`canonical_bytes`] 均不含本表**(0-byte 不动,m90 DAG
/// digest golden 不漂移)——属性表为内存直构面(与 [`ClusterDag::leaf_source_tris`]
/// 同待遇);资产化承载(RXGB 扩展段)= 后续窗分界,见 #96 交付报告。
#[derive(Debug, Clone, PartialEq)]
pub struct ClusterDagAttrs {
    /// v1 DAG(字段面/字节面与无属性链同构)。
    pub base: ClusterDag,
    /// 与 `base.vertices` 等长平行:顶点 UV(粗簇顶点 = 简化链插值产物)。
    pub vertex_uv: Vec<[f32; 2]>,
    /// 与 `base.vertices` 等长平行:顶点法线(输入在场才在场)。
    pub vertex_normal: Option<Vec<[f32; 3]>>,
}

impl ClusterDagAttrs {
    /// 簇的局部顶点 UV 切片(与 [`ClusterDag::cluster_vertices`] 同切片口径)。
    pub fn cluster_uvs(&self, id: u32) -> &[[f32; 2]] {
        let r = self.base.records[id as usize];
        &self.vertex_uv[r.vertex_offset as usize..(r.vertex_offset + r.vertex_count) as usize]
    }

    /// 簇的局部顶点法线切片(法线输入缺席返回 None)。
    pub fn cluster_normals(&self, id: u32) -> Option<&[[f32; 3]]> {
        let nm = self.vertex_normal.as_ref()?;
        let r = self.base.records[id as usize];
        Some(&nm[r.vertex_offset as usize..(r.vertex_offset + r.vertex_count) as usize])
    }
}

/// [`build_dag_params`] 的属性保持变体(G31+ #96 加性入口;既有入口签名/
/// 默认行为 0-byte 不动)。UV(+可选法线)全链跟随简化:
/// - **位置面与无属性链同律**——属性不参与选边/定新位置/fold-over 判定,
///   无接缝输入下 `base` 与 [`build_dag_params`] 产物逐位一致(单测锚);
/// - 跨组焊接键扩为「位置 + 属性 bits」(接缝顶点不被误并);
/// - UV 接缝顶点([`attr_seam_flags`])保守锁定——接缝两侧逐位保持无裂缝;
/// - 退化输入 typed Err([`AttrMeshError`];字段公开可绕过 [`TriMesh::with_attrs`]
///   校验,本入口再次 fail-closed)。
pub fn build_dag_attrs(
    mesh: &AttrTriMesh,
    params: &DagBuildParams,
) -> Result<ClusterDagAttrs, AttrMeshError> {
    crate::mesh::validate_attr_input(
        &mesh.mesh.positions,
        &mesh.mesh.indices,
        &mesh.attrs.uv,
        mesh.attrs.normal.as_deref(),
    )?;
    let (base, tables) = build_dag_impl(&mesh.mesh, Some(&mesh.attrs), params);
    let (vertex_uv, vertex_normal) = tables.expect("属性链必产属性表");
    Ok(ClusterDagAttrs {
        base,
        vertex_uv,
        vertex_normal,
    })
}

/// DAG 构建单一实现(#96 起属性经 `Option` 线程化:`None` 路径与既有实现
/// 逐字同路——属性只在收缩执行后插值/焊接键扩位/接缝锁定三点生效,不触碰
/// 任何位置/拓扑决策,默认产物字节不变由 m90 golden 与单测锚双证)。
fn build_dag_impl(
    mesh: &TriMesh,
    attrs_in: Option<&TriMeshAttrs>,
    params: &DagBuildParams,
) -> (ClusterDag, Option<(Vec<[f32; 2]>, Option<Vec<[f32; 3]>>)>) {
    let tris = mesh.triangles();
    let raw = clusterize_tris(&mesh.positions, &tris);
    let n_raw = raw.len();
    let mut level = LevelMesh {
        positions: mesh.positions.clone(),
        tris,
        clusters: raw.iter().map(|c| c.tris.clone()).collect(),
        errors: vec![0.0; n_raw],
        source_group: (0..n_raw as u32).collect(),
    };
    // #96 属性链:与 level.positions 平行的层属性(叶层 = 输入属性)。
    let mut level_attrs: Option<SubMeshAttrs> = attrs_in.map(|a| SubMeshAttrs {
        uv: a.uv.clone(),
        normal: a.normal.clone(),
    });
    let mut levels: Vec<LevelMesh> = Vec::new();
    let mut levels_attrs: Vec<SubMeshAttrs> = Vec::new();
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
        let groups = group_clusters(&level, &edge_faces, params);
        // #96 属性链:UV 接缝顶点(同位置 bits 多 id)本层保守锁定旗标。
        let seam = level_attrs.as_ref().map(|_| attr_seam_flags(&level.positions));
        let mut next = LevelMesh::default();
        let mut next_attrs: Option<SubMeshAttrs> =
            level_attrs.as_ref().map(SubMeshAttrs::empty_like);
        let mut links: Vec<Vec<u32>> = Vec::new();
        let mut g_of = vec![(0u32, 0.0f32); level.clusters.len()];
        // 精确位置焊接(端点收缩保证坐标逐位来自本层已有顶点;QEM 腿新位置
        // 仅组内部顶点——组边界锁定逐位不动,跨组焊接键唯一性维持)。
        let mut weld: HashMap<[u32; 3], u32> = HashMap::new();
        // 属性链焊接键 = 位置 + UV + 法线 bits(法线缺席补定值 0;锁定顶点
        // 位置与属性均逐位保持 ⇒ 组边界跨组键一致,接缝拷贝不被误并)。
        let mut weld_attrs: HashMap<[u32; 8], u32> = HashMap::new();
        for (gi, g) in groups.iter().enumerate() {
            let (mut sub, local_to_global) = extract_group(&level, g, &edge_faces);
            let sub_attrs = level_attrs
                .as_ref()
                .map(|la| la.gather(&local_to_global));
            if let Some(seam) = &seam {
                for (l, &gv) in local_to_global.iter().enumerate() {
                    if seam[gv as usize] {
                        sub.locked[l] = true;
                    }
                }
            }
            let target = (sub.tris.len() / 2).max(1);
            let (sm, sm_attrs, own_err) = match params.simplify {
                SimplifyKind::ShortestEdge => simplify_group_impl(&sub, target, sub_attrs),
                SimplifyKind::Qem => {
                    crate::qem::simplify_group_qem_impl(&sub, target, sub_attrs)
                }
            };
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
                        gt[k] = match (&sm_attrs, next_attrs.as_mut()) {
                            (Some(sa), Some(na)) => {
                                let pb = pos.map(f32::to_bits);
                                let ub = sa.uv[v as usize].map(f32::to_bits);
                                let nb = sa
                                    .normal
                                    .as_ref()
                                    .map_or([0u32; 3], |nm| nm[v as usize].map(f32::to_bits));
                                let key =
                                    [pb[0], pb[1], pb[2], ub[0], ub[1], nb[0], nb[1], nb[2]];
                                let nid = next.positions.len() as u32;
                                *weld_attrs.entry(key).or_insert_with(|| {
                                    next.positions.push(pos);
                                    na.push_from(sa, v as usize);
                                    nid
                                })
                            }
                            _ => {
                                let key = pos.map(f32::to_bits);
                                let nid = next.positions.len() as u32;
                                *weld.entry(key).or_insert_with(|| {
                                    next.positions.push(pos);
                                    nid
                                })
                            }
                        };
                    }
                    tri_ids.push(next.tris.len() as u32);
                    next.tris.push(gt);
                }
                next.clusters.push(tri_ids);
                next.errors.push(gerr);
                next.source_group.push(gi as u32);
                links.push(g.clone());
            }
        }
        if next.tris.len() >= level.tris.len() {
            break; // 不再缩减 → 当前层为顶层(根)
        }
        group_counts.push(groups.len() as u32);
        group_of.push(g_of);
        child_links.push(links);
        if let Some(la) = level_attrs.take() {
            levels_attrs.push(la);
        }
        levels.push(level);
        level = next;
        level_attrs = next_attrs;
    }
    if let Some(la) = level_attrs.take() {
        levels_attrs.push(la);
    }
    levels.push(level);
    let attrs_slice = attrs_in.map(|_| levels_attrs.as_slice());
    export(&levels, &group_of, &group_counts, &child_links, attrs_slice)
}

// ———— G9.2 M90 深化 API(RXS-0345;纯追加,v1 面 0-byte)————

/// [`build_dag`] 的 typed `Err` 变体:构建(资产级输入经 [`DagAsset::static_mesh`]
/// 等价路径)+ 逐边单调性核验;产物破坏单调性即 fail-closed 拒录(RXS-0345 §1)。
pub fn build_dag_v2(mesh: &TriMesh) -> Result<ClusterDag, DagError> {
    let dag = build_dag(mesh);
    validate_monotonicity(&dag)?;
    Ok(dag)
}

/// 资产级 builder 入口:蒙皮元数据三字段校核 + 单调性核验 + CLAS 烘焙输入导出
/// (RXS-0345 §1/§3.3/§3.4)。骨骼资产(skinned = Some)任一簇缺三字段任一面 =
/// typed `Err` 拒录;内部不变量断言转译为 `DagError::NonMonotonicInput`——
/// builder 失败一律 typed `Err`,无 panic 泄漏(FLS:本 spec 无 UB 节)。
pub fn build_asset_dag(asset: &DagAsset) -> Result<ClusterDagV2, DagError> {
    build_asset_dag_kind(asset, SimplifyKind::ShortestEdge)
}

/// [`build_asset_dag`] 的简化器参数化变体(G31+ #66)。骨骼资产 + `Qem` =
/// typed `Err` 拒录(QEM 移动内部顶点 ⇒ 粗簇顶点位置不再命中叶层蒙皮
/// 反查表 ⇒ 三字段不可得——fail-closed 不静默降档)。
pub fn build_asset_dag_kind(
    asset: &DagAsset,
    kind: SimplifyKind,
) -> Result<ClusterDagV2, DagError> {
    build_asset_dag_params(
        asset,
        &DagBuildParams {
            simplify: kind,
            ..DagBuildParams::default()
        },
    )
}

/// [`build_asset_dag`] 的全参数化变体(G31+ #66/#98;拒录语义同
/// [`build_asset_dag_kind`])。
pub fn build_asset_dag_params(
    asset: &DagAsset,
    params: &DagBuildParams,
) -> Result<ClusterDagV2, DagError> {
    if params.simplify == SimplifyKind::Qem && asset.skinned.is_some() {
        return Err(DagError::SkinMetadataMissing {
            cluster: 0,
            detail: "QEM 简化器移动内部顶点,蒙皮叶层反查不可得——骨骼资产须走 ShortestEdge",
        });
    }
    // 叶层反查上下文(RAII;panic 路径同摘)。
    let _ctx = MeshContextGuard::install(&asset.mesh);
    // 内部 panic 级不变量断言(病态输入/中间态)→ typed Err 转译(不静默、不 clamp)。
    let built = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        build_dag_params(&asset.mesh, params)
    }));
    let dag = match built {
        Ok(d) => d,
        Err(payload) => {
            let s: String = match payload.downcast::<String>() {
                Ok(b) => *b,
                Err(p) => match p.downcast::<&'static str>() {
                    Ok(b) => (*b).to_string(),
                    Err(_) => String::new(),
                },
            };
            let detail: &'static str = match s.as_str() {
                "monotonic violation" => "monotonic violation",
                _ => "builder internal assertion",
            };
            return Err(DagError::NonMonotonicInput { detail });
        }
    };
    validate_monotonicity(&dag)?;
    let skin = derive_skin_metadata(&dag, asset.skinned.as_ref())?;
    let clas = (0..dag.records.len() as u32)
        .map(|id| clas_bake_input_of(&dag, id))
        .collect();
    Ok(ClusterDagV2 {
        base: dag,
        skin,
        clas,
    })
}

/// 属性资产链构建错误(G31+ #96 加性面:退化输入域([`AttrMeshError`])与
/// DAG 构建/单调性域([`DagError`])的并集;Display 逐域转发)。
#[derive(Debug, Clone, PartialEq)]
pub enum DagAttrsError {
    /// 属性输入退化(空网格/索引越界/属性表不齐/非有限)。
    Attr(AttrMeshError),
    /// DAG 构建/单调性破坏(资产级入口同域)。
    Dag(DagError),
}

impl std::fmt::Display for DagAttrsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DagAttrsError::Attr(e) => write!(f, "{e}"),
            DagAttrsError::Dag(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for DagAttrsError {}

/// [`build_asset_dag_params`] 的属性保持变体(G31+ #96 加性入口;静态网格
/// 专用——属性臂无蒙皮语义,蒙皮资产走既有入口)。增值面与资产级入口同律:
/// 内部 panic 级不变量断言 → typed Err 转译 + `base` 面逐边单调性核验;
/// 属性面语义 = [`build_dag_attrs`](位置面与无属性链同律,焊接键扩位 +
/// 接缝保守锁定)。**RXGB 冻结序列化/[`canonical_bytes`] 均不含属性表**
/// (m90 DAG digest golden 不漂移,见 [`ClusterDagAttrs`])。
pub fn build_asset_dag_attrs_params(
    mesh: &AttrTriMesh,
    params: &DagBuildParams,
) -> Result<ClusterDagAttrs, DagAttrsError> {
    // 内部 panic 级不变量断言(病态输入/中间态)→ typed Err 转译
    // (build_asset_dag_params 同一转译律;退化输入的 typed AttrMeshError
    // 在 build_dag_attrs 内先验,不进 panic 域)。
    let built = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        build_dag_attrs(mesh, params)
    }));
    let attrs = match built {
        Ok(Ok(a)) => a,
        Ok(Err(e)) => return Err(DagAttrsError::Attr(e)),
        Err(payload) => {
            let s: String = match payload.downcast::<String>() {
                Ok(b) => *b,
                Err(p) => match p.downcast::<&'static str>() {
                    Ok(b) => (*b).to_string(),
                    Err(_) => String::new(),
                },
            };
            let detail: &'static str = match s.as_str() {
                "monotonic violation" => "monotonic violation",
                _ => "builder internal assertion",
            };
            return Err(DagAttrsError::Dag(DagError::NonMonotonicInput { detail }));
        }
    };
    validate_monotonicity(&attrs.base).map_err(DagAttrsError::Dag)?;
    Ok(attrs)
}

/// 逐边单调性机器核验:DAG 每条 parent→child 边 `parent.error ≥ child.error`
/// (RXS-0345 §1 逐字面;首条破坏边即 typed `Err`,不聚合不静默)。
pub fn validate_monotonicity(dag: &ClusterDag) -> Result<(), DagError> {
    for parent in 0..dag.records.len() as u32 {
        let pe = dag.record(parent).error;
        for &child in dag.children_of(parent) {
            let ce = dag.record(child).error;
            // 浮点序:NaN 不可比较同判破坏(partial_cmp 显式区分,非静默 ≥ 简化)。
            if !matches!(
                pe.partial_cmp(&ce),
                Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
            ) {
                return Err(DagError::NonMonotonicEdge {
                    parent,
                    child,
                    parent_error: pe,
                    child_error: ce,
                });
            }
        }
    }
    Ok(())
}

/// 蒙皮元数据推导与校核(RXS-0345 §3.3 三字段:最大影响骨数/骨骼索引集/
/// 包围体膨胀系数)。骨骼资产缺任一字段面 = typed `Err` 拒录。
pub fn derive_skin_metadata(
    dag: &ClusterDag,
    skinned: Option<&SkinWeights>,
) -> Result<Vec<ClusterSkinMeta>, DagError> {
    let n = dag.records.len();
    let mut out = Vec::with_capacity(n);
    match skinned {
        None => {
            out.resize_with(n, ClusterSkinMeta::unskinned);
        }
        Some(sw) => {
            if sw.vertex_influences.len() != dag.mesh_positions_len() {
                // 权重表与网格顶点数不齐 = 三字段面残缺,拒录首簇定位。
                let id = dag.leaf_ids().next().unwrap_or(0);
                return Err(DagError::SkinMetadataMissing {
                    cluster: id,
                    detail: "vertex_influences 与网格顶点数不齐",
                });
            }
            for id in 0..n as u32 {
                let verts = dag.cluster_vertices(id);
                let mut bones: Vec<u32> = Vec::new();
                let mut max_inf = 0usize;
                let mut rows: Vec<&[(u32, f32)]> = Vec::with_capacity(verts.len());
                let mut missing = false;
                for v in verts {
                    let key = v.map(f32::to_bits);
                    match dag.leaf_vertex_source_index(&key) {
                        Some(gi) => rows.push(&sw.vertex_influences[gi as usize]),
                        None => missing = true,
                    }
                }
                if missing {
                    return Err(DagError::SkinMetadataMissing {
                        cluster: id,
                        detail: "簇顶点无叶层权重行(三字段不可得)",
                    });
                }
                for row in &rows {
                    if row.is_empty() {
                        return Err(DagError::SkinMetadataMissing {
                            cluster: id,
                            detail: "顶点骨骼权重行为空(最大影响骨数不可得)",
                        });
                    }
                    max_inf = max_inf.max(row.len());
                    for &(b, _) in row.iter() {
                        if !bones.contains(&b) {
                            bones.push(b);
                        }
                    }
                }
                bones.sort_unstable();
                bones.dedup();
                for &b in &bones {
                    if b >= sw.joint_count {
                        return Err(DagError::SkinMetadataInconsistent {
                            cluster: id,
                            detail: "骨骼索引越界(≥ joint_count)",
                        });
                    }
                }
                if max_inf > MAX_BONE_INFLUENCES {
                    return Err(DagError::SkinMetadataInconsistent {
                        cluster: id,
                        detail: "单顶点影响骨数 > MAX_BONE_INFLUENCES",
                    });
                }
                if max_inf == 0 {
                    // 全簇零权重 = 事实非蒙皮簇(字段面完整:零值三字段)。
                    out.push(ClusterSkinMeta::unskinned());
                    continue;
                }
                let bound_inflation = bound_inflation_of(&rows);
                out.push(ClusterSkinMeta {
                    max_influences: max_inf as u32,
                    bone_indices: Some(bones),
                    bound_inflation: Some(bound_inflation),
                });
            }
        }
    }
    Ok(out)
}

/// 包围体膨胀系数(Kerbl 保守界所需输入的离线界,确定性纯函数):
/// 顶点权重均摊偏移总量 = 蒙皮位移上界的保守代理(权重非负 ≤ 1)。
fn bound_inflation_of(rows: &[&[(u32, f32)]]) -> f32 {
    let total_w: f32 = rows
        .iter()
        .map(|r| r.iter().map(|&(_, w)| w).sum::<f32>())
        .sum();
    let n_inf: usize = rows.iter().map(|r| r.len()).sum();
    if n_inf == 0 {
        return 0.0;
    }
    let mean = total_w / n_inf as f32;
    rows.iter()
        .flat_map(|r| r.iter())
        .map(|&(_, w)| (w - mean).abs())
        .fold(0.0f32, f32::max)
}

/// 簇级 CLAS 烘焙输入(三角形簇几何 = `cluster_vertices`/`cluster_triangle` 视图;
/// 簇级 AABB = 簇局部顶点逐位 min/max,RXS-0345 §3.4)。
pub fn clas_bake_input_of(dag: &ClusterDag, id: u32) -> ClasBakeInput {
    let verts = dag.cluster_vertices(id);
    let mut lo = [f32::INFINITY; 3];
    let mut hi = [f32::NEG_INFINITY; 3];
    for v in verts {
        for k in 0..3 {
            lo[k] = lo[k].min(v[k]);
            hi[k] = hi[k].max(v[k]);
        }
    }
    if verts.is_empty() {
        lo = [0.0; 3];
        hi = [0.0; 3];
    }
    ClasBakeInput {
        aabb_min: lo,
        aabb_max: hi,
    }
}

/// 蒙皮簇运行时数据(G9.3 M92 RXS-0353 消费桥;builder 蒙皮三字段产物 +
/// 逐顶点权重 → rurix-render host 蒙皮求值器输入的**拥有型**载体)。
///
/// 运行时(host/device)只消费本结构与骨骼 palette;页内 skin_hdr/bone_idx/
/// clas_aabb 段 ABI 与本结构字段一一对应(RXS-0344 段序不重发)。
#[derive(Debug, Clone, PartialEq)]
pub struct SkinnedClusterData {
    /// 蒙皮元数据:最大影响骨数(skin_hdr.max_influences)。
    pub max_influences: u32,
    /// 蒙皮元数据:骨骼索引集(bone_idx 段,升序)。
    pub bone_indices: Vec<u32>,
    /// 蒙皮元数据:包围体膨胀系数(skin_hdr.bound_inflation)。
    pub bound_inflation: f32,
    /// 簇级静止 AABB min(clas_aabb 段)。
    pub aabb_min: [f32; 3],
    /// 簇级静止 AABB max。
    pub aabb_max: [f32; 3],
    /// 簇局部静止顶点(v1 顶点段视图拷贝)。
    pub vertices: Vec<[f32; 3]>,
    /// 逐顶点权重行(与 `vertices` 等长;叶层源顶点反查自 `SkinWeights`)。
    pub weights: Vec<Vec<(u32, f32)>>,
}

impl SkinnedClusterData {
    /// 借出为 rurix-render host 蒙皮求值器输入(零拷贝视图)。
    pub fn as_input(&self) -> rurix_render::geometry::skinning::ClusterSkinInput<'_> {
        rurix_render::geometry::skinning::ClusterSkinInput {
            max_influences: self.max_influences,
            bone_indices: &self.bone_indices,
            bound_inflation: self.bound_inflation,
            rest_aabb_min: self.aabb_min,
            rest_aabb_max: self.aabb_max,
            vertices: &self.vertices,
            weights: &self.weights,
        }
    }
}

/// 蒙皮簇运行时数据导出(M92 消费侧最小追加;RXS-0353 实现锚定)。
///
/// 反查规则与 builder 校核侧同源(顶点位置 bits → 叶层源顶点,同位取最小
/// 下标,确定性)。非蒙皮簇(`max_influences == 0`)/ 缺权重行 = typed `Err`
/// 拒录(fail-closed,与 RXS-0345 §3.3 同口径)。
pub fn skinned_cluster_runtime_data(
    v2: &ClusterDagV2,
    mesh: &TriMesh,
    skin: &SkinWeights,
    id: u32,
) -> Result<SkinnedClusterData, DagError> {
    let meta = v2.skin.get(id as usize).ok_or(DagError::SkinMetadataMissing {
        cluster: id,
        detail: "簇号越出蒙皮元数据表",
    })?;
    let (Some(bones), Some(inflation)) = (&meta.bone_indices, meta.bound_inflation) else {
        return Err(DagError::SkinMetadataMissing {
            cluster: id,
            detail: "非蒙皮簇无蒙皮运行时输入",
        });
    };
    if skin.vertex_influences.len() != mesh.positions.len() {
        return Err(DagError::SkinMetadataInconsistent {
            cluster: id,
            detail: "权重表与网格顶点数不齐",
        });
    }
    let mut map: HashMap<[u32; 3], u32> = HashMap::with_capacity(mesh.positions.len());
    for (i, p) in mesh.positions.iter().enumerate() {
        map.entry(p.map(f32::to_bits)).or_insert(i as u32);
    }
    let vertices = v2.base.cluster_vertices(id).to_vec();
    let mut weights = Vec::with_capacity(vertices.len());
    for v in &vertices {
        let Some(&gi) = map.get(&v.map(f32::to_bits)) else {
            return Err(DagError::SkinMetadataMissing {
                cluster: id,
                detail: "簇顶点无叶层权重行(运行时输入不可得)",
            });
        };
        weights.push(skin.vertex_influences[gi as usize].clone());
    }
    let clas = &v2.clas[id as usize];
    Ok(SkinnedClusterData {
        max_influences: meta.max_influences,
        bone_indices: bones.clone(),
        bound_inflation: inflation,
        aabb_min: clas.aabb_min,
        aabb_max: clas.aabb_max,
        vertices,
        weights,
    })
}

/// DAG canonical 字节流(双构建 byte-equal golden 的比对面;RXS-0345 §5):
/// v1 全字段面逐位拼接(records/nodes/children/vertices/indices/levels),
/// 与 RXGB 序列化同信息集、本结构内联布局(manifest 落 digest 不进本流)。
pub fn canonical_bytes(dag: &ClusterDag) -> Vec<u8> {
    let mut out = Vec::new();
    for r in &dag.records {
        for &x in &r.center {
            out.extend_from_slice(&x.to_bits().to_le_bytes());
        }
        out.extend_from_slice(&r.radius.to_bits().to_le_bytes());
        for &x in &r.cone_axis {
            out.extend_from_slice(&x.to_bits().to_le_bytes());
        }
        out.extend_from_slice(&r.cone_cutoff.to_bits().to_le_bytes());
        out.extend_from_slice(&r.error.to_bits().to_le_bytes());
        out.extend_from_slice(&r.parent_error.to_bits().to_le_bytes());
        out.extend_from_slice(&r.vertex_offset.to_le_bytes());
        out.extend_from_slice(&r.triangle_offset.to_le_bytes());
        out.extend_from_slice(&r.vertex_count.to_le_bytes());
        out.extend_from_slice(&r.triangle_count.to_le_bytes());
        out.extend_from_slice(&r.page_id.to_le_bytes());
        out.extend_from_slice(&r.reserved.to_le_bytes());
    }
    for n in &dag.nodes {
        out.extend_from_slice(&n.first_child.to_le_bytes());
        out.extend_from_slice(&n.child_count.to_le_bytes());
        out.extend_from_slice(&n.level.to_le_bytes());
        out.extend_from_slice(&n.group.to_le_bytes());
    }
    for &c in &dag.children {
        out.extend_from_slice(&c.to_le_bytes());
    }
    for v in &dag.vertices {
        for &x in v {
            out.extend_from_slice(&x.to_bits().to_le_bytes());
        }
    }
    out.extend_from_slice(&dag.triangle_indices);
    for l in &dag.levels {
        out.extend_from_slice(&l.record_start.to_le_bytes());
        out.extend_from_slice(&l.record_count.to_le_bytes());
        out.extend_from_slice(&l.triangle_count.to_le_bytes());
    }
    out
}

/// 簇分组(meshopt_partitionClusters 目标的简化版):簇邻接按**共享边计数**
/// 加权,贪心生长 ≤ `params.group_size` 簇/组——共享边越多,合并后组内边
/// 越多、被锁定的组边界越少、简化越充分。种子顺序用 Morton 码(确定性空间
/// 序);同权按中心距离、id 决胜(全确定性)。
///
/// G31+ #98 边界交替(`params.sibling_bias`,Nanite `ClusterDAG.cpp` 边权
/// `NumSharedEdges × (bSiblings ? 1 : 16) + 4` 逐字对齐):非同胞(上一层
/// 不同组产物)簇对权 ×16——上层组边界(非同胞相邻处)优先并入新组内部,
/// 锁定边逐层轮换获得简化机会,根治「树形方案边界永锁积累碎面」。
fn group_clusters(
    level: &LevelMesh,
    edge_faces: &HashMap<(u32, u32), Vec<u32>>,
    params: &DagBuildParams,
) -> Vec<Vec<u32>> {
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
    // 边界交替权重(Nanite 字面):同胞 ×1、非同胞 ×16,+4 常数底;
    // legacy(sibling_bias = false)= 纯共享边计数,序语义 0-漂移。
    let eff_weight = |m: u32, cand: u32, shared: u32| -> u64 {
        if params.sibling_bias {
            let siblings =
                level.source_group[m as usize] == level.source_group[cand as usize];
            u64::from(shared) * if siblings { 1 } else { 16 } + 4
        } else {
            u64::from(shared)
        }
    };
    let order = morton_order(&centers);
    let mut assigned = vec![false; n];
    let mut groups = Vec::new();
    for &seed in &order {
        if assigned[seed as usize] {
            continue;
        }
        assigned[seed as usize] = true;
        let mut group = vec![seed];
        while group.len() < params.group_size {
            // (候选, 有效权, 中心距离):权重降序 → 距离升序 → id 升序。
            let mut best: Option<(u32, u64, f32)> = None;
            for &m in &group {
                for (&cand, &w) in &adj[m as usize] {
                    if assigned[cand as usize] {
                        continue;
                    }
                    let we = eff_weight(m, cand, w);
                    let d = vdist(centers[cand as usize], centers[m as usize]);
                    let better = match best {
                        None => true,
                        Some((bid, bw, bd)) => {
                            we > bw || (we == bw && (d < bd || (d == bd && cand < bid)))
                        }
                    };
                    if better {
                        best = Some((cand, we, d));
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
/// 同时返回「局部顶点 → 层全局顶点」映射(#96 属性链按此 gather 组局部属性)。
fn extract_group(
    level: &LevelMesh,
    group: &[u32],
    edge_faces: &HashMap<(u32, u32), Vec<u32>>,
) -> (SubMesh, Vec<u32>) {
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
    (
        SubMesh {
            positions,
            tris,
            locked,
            face_on_boundary,
        },
        local_to_global,
    )
}

/// 最短边贪心收缩(非 QEM——已知简化;误差上界保守、端点保持保焊接)。
///
/// 裂缝保护第二半(精确规则):被收缩边 (u,v) 的任一共边存活面**含组边界边**
/// (`face_on_boundary`)则禁止该收缩——边界边依附的面不死、边界边端点(锁定
/// 顶点)永不移动/合并,组边界折线在简化前后逐位一致,任意 LOD cut 组合无
/// 裂缝。方向:锁定端保留,双开取小号端(确定性)。
///
/// 生产路径经 [`simplify_group_impl`] 直调(属性 `None`);本包装保持既有
/// 二元签名供既有单测锚消费。
#[cfg(test)]
fn simplify_group(sub: &SubMesh, target: usize) -> (SubMesh, f32) {
    let (out, _, err) = simplify_group_impl(sub, target, None);
    (out, err)
}

/// [`simplify_group`] 单一实现(#96 属性经 `Option` 线程化;`None` 路径逐字
/// 同路)。端点保持收缩的属性面平凡:keep 端位置不动 ⇒ keep 端属性逐位
/// 保持,drop 端属性随顶点消亡——属性只参与末端压缩重映射。
pub(crate) fn simplify_group_impl(
    sub: &SubMesh,
    target: usize,
    attrs: Option<SubMeshAttrs>,
) -> (SubMesh, Option<SubMeshAttrs>, f32) {
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
    let mut out_attrs = attrs.as_ref().map(SubMeshAttrs::empty_like);
    for v in 0..nv {
        if alive_v[v] {
            remap[v] = out_pos.len() as u32;
            out_pos.push(sub.positions[v]);
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
        // 端点保持收缩不改写属性表,原属性即入参原值。
        return (sub.clone(), attrs, 0.0);
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
        out_attrs,
        max_err,
    )
}

/// 两三角形顶点集合相等(绕序无关;收缩后重复面判定)。
fn same_tri_set(a: [u32; 3], b: [u32; 3]) -> bool {
    a.iter().all(|v| b.contains(v))
}

/// 汇总导出:扁平 ClusterRecord + 层级关系表 + 顶点/索引数据段 + 层表。
/// #96 属性链(`levels_attrs = Some`)同步导出与顶点数据段平行的属性表
/// (RXGB 序列化不消费——内存直构面)。
fn export(
    levels: &[LevelMesh],
    group_of: &[Vec<(u32, f32)>],
    group_counts: &[u32],
    child_links: &[Vec<Vec<u32>>],
    levels_attrs: Option<&[SubMeshAttrs]>,
) -> (ClusterDag, Option<(Vec<[f32; 2]>, Option<Vec<[f32; 3]>>)>) {
    if let Some(las) = levels_attrs {
        debug_assert_eq!(las.len(), levels.len(), "层属性表与层表不齐");
    }
    let mut dag = ClusterDag::default();
    let mut out_uv: Vec<[f32; 2]> = Vec::new();
    let mut out_normal: Option<Vec<[f32; 3]>> = levels_attrs
        .and_then(|las| las.first())
        .and_then(|la| la.normal.as_ref().map(|_| Vec::new()));
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
                        if let Some(las) = levels_attrs {
                            let la = &las[li];
                            out_uv.push(la.uv[v as usize]);
                            if let (Some(dst), Some(nm)) =
                                (out_normal.as_mut(), la.normal.as_ref())
                            {
                                dst.push(nm[v as usize]);
                            }
                        }
                    }
                }
            }
            debug_assert!(map.len() <= crate::cluster::MAX_VERTS);
            let t_off = dag.triangle_indices.len() as u32;
            for &f in ctris {
                for &v in &lv.tris[f as usize] {
                    dag.triangle_indices.push(map[&v]);
                }
                // 叶层(li == 0):本层三角 id 即源网格三角 id(clusterize 划分元素),
                // 按导出序平行登记(粗层三角为简化产物,无源 id)。
                if li == 0 {
                    dag.leaf_source_tris.push(f);
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
    // saturating_add 防御 u32 溢出(极端场景:超 4G 组 → 饱和不回绕,保单调性不破)。
    let mut group_seq = 0u32;
    for li in 0..top {
        let base = dag.levels[li].record_start as usize;
        for (ci, &(g, _)) in group_of[li].iter().enumerate() {
            dag.nodes[base + ci].group = group_seq.saturating_add(g);
        }
        group_seq = group_seq.saturating_add(group_counts[li]);
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
        group_seq = group_seq.saturating_add(1);
    }
    let attr_tables = levels_attrs.map(|_| (out_uv, out_normal));
    (dag, attr_tables)
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
        let n_raw = raw.len();
        let level = LevelMesh {
            positions: mesh.positions.clone(),
            tris,
            clusters: raw.iter().map(|c| c.tris.clone()).collect(),
            errors: vec![0.0; n_raw],
            source_group: (0..n_raw as u32).collect(),
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
        let (s0, _) = extract_group(&level, &g0, &edge_faces);
        let (s1, _) = extract_group(&level, &g1, &edge_faces);
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

    /// G31+ #66:QEM 加性简化器全 DAG 不变量(与既有事实源同一套锚:叶覆盖/
    /// 单调/层递减/根哨兵/双构建确定性)+ 两简化器压缩力对照登记。
    #[test]
    fn qem_full_dag_invariants_and_comparison() {
        let mesh = TriMesh::uv_sphere(1.0, 32, 32);
        let dag = build_dag_kind(&mesh, SimplifyKind::Qem);
        // 叶覆盖恰一次。
        let total: u32 = dag.leaf_ids().map(|i| dag.record(i).triangle_count).sum();
        assert_eq!(total as usize, mesh.triangle_count());
        // 单调机核(逐边 typed 核验直调)。
        validate_monotonicity(&dag).expect("QEM DAG 单调");
        // 层三角数严格递减 + 根哨兵。
        for w in dag.levels.windows(2) {
            assert!(w[1].triangle_count < w[0].triangle_count, "层三角形数未递减");
        }
        for id in dag.top_level_ids() {
            assert_eq!(dag.record(id).parent_error.to_bits(), f32::MAX.to_bits());
        }
        // 双构建确定性(canonical 字节相等)。
        let dag2 = build_dag_kind(&mesh, SimplifyKind::Qem);
        assert_eq!(canonical_bytes(&dag), canonical_bytes(&dag2), "QEM 双构建漂移");
        // 默认面 0-语义:build_dag == build_dag_kind(ShortestEdge) 逐位。
        let se = build_dag(&mesh);
        let se2 = build_dag_kind(&mesh, SimplifyKind::ShortestEdge);
        assert_eq!(
            canonical_bytes(&se),
            canonical_bytes(&se2),
            "ShortestEdge 参数化包装破坏默认面 0-byte"
        );
        // 压缩力对照登记(#66 参照臂数据面;不设通过线,打印如实)。
        let root_tris = |d: &ClusterDag| -> u32 {
            d.top_level_ids().map(|i| d.record(i).triangle_count).sum()
        };
        println!(
            "[qem_vs_shortest] uv_sphere_32: qem levels={} root_tris={} max_err={:.6}; shortest levels={} root_tris={} max_err={:.6}; qem_stuck_groups={}",
            dag.level_count(),
            root_tris(&dag),
            dag.records.iter().map(|r| r.error).fold(0.0f32, f32::max),
            se.level_count(),
            root_tris(&se),
            se.records.iter().map(|r| r.error).fold(0.0f32, f32::max),
            crate::qem::take_stuck_count(),
        );
    }

    /// G31+ #98:质量档(QEM + 8 簇/组 + 边界交替)全 DAG 不变量 + stuck
    /// 对照登记(组更大 + 边界交替 ⇒ 简化自由度更高,stuck 应显著下降)。
    #[test]
    fn quality_params_dag_invariants_and_stuck_comparison() {
        let mesh = TriMesh::uv_sphere(1.0, 32, 32);
        let _ = crate::qem::take_stuck_count();
        let legacy_qem = build_dag_kind(&mesh, SimplifyKind::Qem);
        let stuck_legacy = crate::qem::take_stuck_count();
        let quality = build_dag_params(&mesh, &DagBuildParams::quality());
        let stuck_quality = crate::qem::take_stuck_count();
        // 全套不变量。
        let total: u32 = quality
            .leaf_ids()
            .map(|i| quality.record(i).triangle_count)
            .sum();
        assert_eq!(total as usize, mesh.triangle_count());
        validate_monotonicity(&quality).expect("质量档 DAG 单调");
        for w in quality.levels.windows(2) {
            assert!(w[1].triangle_count < w[0].triangle_count);
        }
        // 双构建确定性。
        let quality2 = build_dag_params(&mesh, &DagBuildParams::quality());
        let _ = crate::qem::take_stuck_count();
        assert_eq!(canonical_bytes(&quality), canonical_bytes(&quality2));
        // 对照登记(#98 参照臂数据面;stuck 下降为方向性断言——组 8 + 边界
        // 交替的简化自由度收益)。
        println!(
            "[quality_vs_legacy] uv_sphere_32: quality(qem/8/bias) levels={} stuck={} ; legacy(qem/4) levels={} stuck={}",
            quality.level_count(),
            stuck_quality,
            legacy_qem.level_count(),
            stuck_legacy,
        );
        assert!(
            stuck_quality < stuck_legacy,
            "质量档 stuck 未下降: {stuck_quality} ≥ {stuck_legacy}"
        );
    }

    /// G31+ #66:骨骼资产 + QEM = typed Err 拒录(fail-closed 不静默降档)。
    #[test]
    fn qem_skinned_asset_rejected() {
        let mesh = TriMesh::uv_sphere(1.0, 8, 8);
        let n = mesh.positions.len();
        let asset = DagAsset {
            mesh,
            skinned: Some(SkinWeights {
                vertex_influences: (0..n).map(|_| vec![(0u32, 1.0f32)]).collect(),
                joint_count: 2,
            }),
        };
        assert!(matches!(
            build_asset_dag_kind(&asset, SimplifyKind::Qem),
            Err(DagError::SkinMetadataMissing { .. })
        ));
        // 同资产 ShortestEdge 照常通过(拒录面仅 QEM×蒙皮组合)。
        build_asset_dag_kind(&asset, SimplifyKind::ShortestEdge).expect("蒙皮资产走既有简化器");
    }

    /// G31+ #96:属性链全 DAG——①v1 字节面与无属性链逐位一致(fixture 锚:
    /// 属性不反哺位置/拓扑决策;本 fixture 位置唯一无接缝,属性焊接键不裂位置)
    /// ②属性表与顶点数据段平行 ③投影 UV(位置的仿射函数)全层偏差有界、
    /// 叶层逐位精确 ④双构建确定性(含属性位)。默认参数与质量档两臂全跑。
    #[test]
    fn attr_dag_zero_position_drift_and_uv_bounded() {
        let mesh = TriMesh::uv_sphere(1.0, 24, 24);
        let proj_uv = |p: &[f32; 3]| [(p[0] + 1.0) * 0.5, (p[1] + 1.0) * 0.5];
        let uv: Vec<[f32; 2]> = mesh.positions.iter().map(proj_uv).collect();
        let amesh = mesh
            .clone()
            .with_attrs(crate::mesh::TriMeshAttrs { uv, normal: None })
            .expect("合法属性网格");
        for params in [DagBuildParams::default(), DagBuildParams::quality()] {
            let attr = build_dag_attrs(&amesh, &params).expect("属性链构建");
            // ① v1 字节面 0-漂移。
            let plain = build_dag_params(&mesh, &params);
            assert_eq!(
                canonical_bytes(&attr.base),
                canonical_bytes(&plain),
                "属性链改动了 v1 字节面(simplify={:?})",
                params.simplify
            );
            // ② 平行表 + 簇切片口径。
            assert_eq!(attr.vertex_uv.len(), attr.base.vertices.len());
            assert!(attr.vertex_normal.is_none());
            for id in 0..attr.base.records.len() as u32 {
                assert_eq!(
                    attr.cluster_uvs(id).len(),
                    attr.base.cluster_vertices(id).len()
                );
            }
            // ③ 投影 UV 偏差:叶层逐位精确(误差 0 层);全层有界(收缩点
            //   线段插值随位移上界累计,uv 梯度 = 0.5/位置单位;凸包内插值
            //   ⇒ 恒在 [0,1] 盒内)。
            let leaf = &attr.base.levels[0];
            let leaf_vert_end = {
                let last = attr.base.record(leaf.record_start + leaf.record_count - 1);
                (last.vertex_offset + last.vertex_count) as usize
            };
            let mut max_err = 0.0f32;
            for (i, (v, uvv)) in attr
                .base
                .vertices
                .iter()
                .zip(&attr.vertex_uv)
                .enumerate()
            {
                let want = proj_uv(v);
                let e = (uvv[0] - want[0]).abs().max((uvv[1] - want[1]).abs());
                if i < leaf_vert_end {
                    assert_eq!(
                        uvv.map(f32::to_bits),
                        want.map(f32::to_bits),
                        "叶层 UV 须逐位等于输入"
                    );
                }
                assert!((-1e-6..=1.0 + 1e-6).contains(&uvv[0]), "UV 出凸包: {uvv:?}");
                assert!((-1e-6..=1.0 + 1e-6).contains(&uvv[1]), "UV 出凸包: {uvv:?}");
                max_err = max_err.max(e);
            }
            println!(
                "[attr_dag] simplify={:?} levels={} max_uv_err={max_err:.6}",
                params.simplify,
                attr.base.level_count()
            );
            assert!(max_err < 0.30, "UV 偏离投影仿射超界: {max_err}");
            // ④ 双构建确定性(v1 字节 + UV 位)。
            let attr2 = build_dag_attrs(&amesh, &params).expect("二跑");
            assert_eq!(canonical_bytes(&attr.base), canonical_bytes(&attr2.base));
            assert_eq!(
                attr.vertex_uv
                    .iter()
                    .map(|u| u.map(f32::to_bits))
                    .collect::<Vec<_>>(),
                attr2
                    .vertex_uv
                    .iter()
                    .map(|u| u.map(f32::to_bits))
                    .collect::<Vec<_>>(),
                "UV 双构建漂移"
            );
        }
        // 退化输入 typed Err(绕过 with_attrs 的字面构造由本入口自校)。
        let bad = AttrTriMesh {
            mesh: TriMesh::default(),
            attrs: crate::mesh::TriMeshAttrs::default(),
        };
        assert_eq!(
            build_dag_attrs(&bad, &DagBuildParams::default()).unwrap_err(),
            AttrMeshError::EmptyMesh
        );
    }

    /// G31+ #96:资产级属性包装(bake 消费面)——产物与直调 build_dag_attrs
    /// 全等(v1 字节 + UV 位),退化输入走 Attr 域 typed Err。
    #[test]
    fn asset_attrs_wrapper_matches_direct_and_rejects_bad() {
        let mesh = TriMesh::uv_sphere(1.0, 12, 12);
        let uv: Vec<[f32; 2]> = mesh
            .positions
            .iter()
            .map(|p| [(p[0] + 1.0) * 0.5, (p[2] + 1.0) * 0.5])
            .collect();
        let amesh = mesh
            .with_attrs(crate::mesh::TriMeshAttrs { uv, normal: None })
            .expect("合法属性网格");
        let params = DagBuildParams::quality();
        let a = build_asset_dag_attrs_params(&amesh, &params).expect("包装构建");
        let b = build_dag_attrs(&amesh, &params).expect("直调构建");
        assert_eq!(canonical_bytes(&a.base), canonical_bytes(&b.base));
        assert_eq!(
            a.vertex_uv
                .iter()
                .map(|u| u.map(f32::to_bits))
                .collect::<Vec<_>>(),
            b.vertex_uv
                .iter()
                .map(|u| u.map(f32::to_bits))
                .collect::<Vec<_>>(),
            "包装与直调 UV 漂移"
        );
        assert_eq!(a.base.leaf_source_tris, b.base.leaf_source_tris);
        // 退化输入 → Attr 域 typed Err(fail-closed 不 panic)。
        let bad = AttrTriMesh {
            mesh: TriMesh::default(),
            attrs: crate::mesh::TriMeshAttrs::default(),
        };
        assert_eq!(
            build_asset_dag_attrs_params(&bad, &params).unwrap_err(),
            DagAttrsError::Attr(AttrMeshError::EmptyMesh)
        );
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

    // ———— G9.2 M90(RXS-0345)————

    /// 破坏单调性 fixture:合法 DAG 副本上把某非根父簇 `error` 压到其孩子以下
    /// (模拟「破坏单调性的输入/中间态」;RXS-0345 §1 fail-closed 拒录臂)。
    fn make_nonmonotonic_fixture() -> (ClusterDag, u32, u32) {
        let mut dag = build_dag(&TriMesh::uv_sphere(1.0, 24, 24));
        for parent in 0..dag.records.len() as u32 {
            let children = dag.children_of(parent).to_vec();
            if children.is_empty() {
                continue;
            }
            let child = children[0];
            let child_err = dag.record(child).error;
            if child_err > 0.0 {
                dag.records[parent as usize].error = child_err * 0.5;
                return (dag, parent, child);
            }
        }
        panic!("fixture 构造失败:无可用边");
    }

    //@ spec: RXS-0345
    #[test]
    fn nonmonotonic_edge_rejected_typed_err() {
        let (dag, parent, child) = make_nonmonotonic_fixture();
        let err = validate_monotonicity(&dag).expect_err("破坏单调性必须 typed Err 拒录");
        match err {
            DagError::NonMonotonicEdge {
                parent: p,
                child: c,
                parent_error,
                child_error,
            } => {
                assert_eq!(p, parent);
                assert_eq!(c, child);
                assert!(parent_error < child_error);
            }
            other => panic!("错误变体不符: {other:?}"),
        }
        // NaN 误差不可比较 → 同判破坏(浮点序保守侧)。
        let mut nan_dag = build_dag(&TriMesh::uv_sphere(1.0, 24, 24));
        let pid = nan_dag.levels[1].record_start;
        nan_dag.records[pid as usize].error = f32::NAN;
        assert!(matches!(
            validate_monotonicity(&nan_dag),
            Err(DagError::NonMonotonicEdge { .. })
        ));
    }

    //@ spec: RXS-0345
    #[test]
    fn build_dag_v2_accepts_monotonic_and_rejects_broken() {
        let dag = build_dag_v2(&TriMesh::uv_sphere(1.0, 16, 16)).expect("合法 mesh 必须过");
        validate_monotonicity(&dag).expect("产物逐边单调");
        // 双构建 canonical 字节相等(RXS-0345 §5 双构建确定性)。
        let a = build_dag_v2(&TriMesh::uv_sphere(1.0, 16, 16)).unwrap();
        let b = build_dag_v2(&TriMesh::uv_sphere(1.0, 16, 16)).unwrap();
        assert_eq!(canonical_bytes(&a), canonical_bytes(&b));
    }

    //@ spec: RXS-0345
    #[test]
    fn asset_level_builder_catches_panic_as_typed_err() {
        // 资产级入口:病态输入的内部断言必须转译 typed Err,不 panic 泄漏。
        let bad = DagAsset {
            mesh: TriMesh {
                positions: vec![[0.0, 0.0, 0.0]],
                indices: vec![0, 1, 2], // 索引越界(positions 仅 1 顶点)
            },
            skinned: None,
        };
        let err = build_asset_dag(&bad).expect_err("病态输入必须 typed Err");
        assert!(matches!(err, DagError::NonMonotonicInput { .. }));
    }

    //@ spec: RXS-0345
    #[test]
    fn skin_metadata_three_fields_roundtrip() {
        let mesh = TriMesh::uv_sphere(1.0, 12, 12);
        let n = mesh.positions.len();
        // 全顶点半权重绑骨 0/1(权重和 1.0;骨骼索引集应恰 {0,1})。
        let influences: Vec<Vec<(u32, f32)>> = (0..n)
            .map(|_| vec![(0u32, 0.5f32), (1u32, 0.5f32)])
            .collect();
        let asset = DagAsset {
            mesh: mesh.clone(),
            skinned: Some(SkinWeights {
                vertex_influences: influences,
                joint_count: 4,
            }),
        };
        let v2 = build_asset_dag(&asset).expect("蒙皮资产合法构建");
        assert_eq!(v2.skin.len(), v2.base.records.len());
        assert_eq!(v2.clas.len(), v2.base.records.len());
        for id in v2.base.leaf_ids() {
            let meta = &v2.skin[id as usize];
            assert_eq!(meta.max_influences, 2, "最大影响骨数");
            assert_eq!(
                meta.bone_indices.as_deref(),
                Some(&[0u32, 1u32][..]),
                "骨骼索引集升序去重"
            );
            let infl = meta.bound_inflation.expect("包围体膨胀系数在场");
            assert!(infl.is_finite() && infl >= 0.0);
        }
        // CLAS 输入:簇级 AABB ⊇ 簇顶点(逐位)。
        for id in 0..v2.base.records.len() as u32 {
            let c = &v2.clas[id as usize];
            for v in v2.base.cluster_vertices(id) {
                for (k, &x) in v.iter().enumerate() {
                    assert!(c.aabb_min[k] <= x && x <= c.aabb_max[k]);
                }
            }
            // 三角形簇几何视图 = v1 数据段(不重发)。
            assert!(v2.base.record(id).triangle_count > 0);
        }
        let _ = mesh;
    }

    //@ spec: RXS-0353
    #[test]
    fn skinned_cluster_runtime_bridge_end_to_end() {
        use rurix_render::geometry::skinning::{
            SkinPalette, conservative_skinned_aabb, skin_cluster, verify_bound_containment,
        };
        // 与 skin_metadata_three_fields_roundtrip 同资产(全顶点半权重绑骨 0/1)。
        let mesh = TriMesh::uv_sphere(1.0, 12, 12);
        let n = mesh.positions.len();
        let influences: Vec<Vec<(u32, f32)>> = (0..n)
            .map(|_| vec![(0u32, 0.5f32), (1u32, 0.5f32)])
            .collect();
        let asset = DagAsset {
            mesh: mesh.clone(),
            skinned: Some(SkinWeights {
                vertex_influences: influences,
                joint_count: 4,
            }),
        };
        let v2 = build_asset_dag(&asset).expect("蒙皮资产合法构建");
        let skinned = asset.skinned.as_ref().expect("在场");
        // 叶簇桥接:三字段 + 逐顶点权重 + 簇级 AABB 全部在场。
        let leaf = v2.base.leaf_ids().next().expect("叶层非空");
        let data = skinned_cluster_runtime_data(&v2, &asset.mesh, skinned, leaf).expect("桥接");
        assert_eq!(data.max_influences, 2);
        assert_eq!(data.bone_indices, vec![0, 1]);
        assert_eq!(data.vertices.len(), data.weights.len());
        assert!(!data.vertices.is_empty());
        // 运行时消费:骨骼 palette(骨 0 平移、骨 1 恒等)→ 蒙皮 + 包围体包含。
        let palette = SkinPalette {
            bones: vec![
                [
                    [1.0, 0.0, 0.0, 0.5],
                    [0.0, 1.0, 0.0, -0.25],
                    [0.0, 0.0, 1.0, 1.0],
                ],
                [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                ],
            ],
        };
        let input = data.as_input();
        let out = skin_cluster(&input, &palette).expect("蒙皮求值");
        assert_eq!(out.len(), data.vertices.len());
        let bound = conservative_skinned_aabb(&input, &palette).expect("保守包围体");
        verify_bound_containment(&bound, &out).expect("包含不变式(任意姿态 100% 包含)");
        // 确定性:同输入双跑逐位一致。
        let out2 = skin_cluster(&input, &palette).expect("二跑");
        for (a, b) in out.iter().zip(out2.iter()) {
            assert_eq!(a.map(f32::to_bits), b.map(f32::to_bits));
        }
        // 权重表不齐 ⇒ typed Err(fail-closed)。
        let short = SkinWeights {
            vertex_influences: vec![vec![(0u32, 1.0f32)]; n - 1],
            joint_count: 2,
        };
        assert!(matches!(
            skinned_cluster_runtime_data(&v2, &asset.mesh, &short, leaf),
            Err(DagError::SkinMetadataInconsistent { .. })
        ));
        let _ = mesh;
    }

    //@ spec: RXS-0345
    #[test]
    fn skin_metadata_missing_fields_rejected() {
        let mesh = TriMesh::uv_sphere(1.0, 8, 8);
        let n = mesh.positions.len();
        // 缺臂 1:权重表行数与顶点数不齐(三字段面残缺)。
        let short = DagAsset {
            mesh: mesh.clone(),
            skinned: Some(SkinWeights {
                vertex_influences: vec![vec![(0u32, 1.0f32)]; n - 1],
                joint_count: 2,
            }),
        };
        assert!(matches!(
            build_asset_dag(&short),
            Err(DagError::SkinMetadataMissing { .. })
        ));
        // 缺臂 2:某顶点权重行为空(最大影响骨数不可得)。
        let mut rows: Vec<Vec<(u32, f32)>> = (0..n).map(|_| vec![(0u32, 1.0f32)]).collect();
        rows[0] = Vec::new();
        let empty_row = DagAsset {
            mesh: mesh.clone(),
            skinned: Some(SkinWeights {
                vertex_influences: rows,
                joint_count: 2,
            }),
        };
        assert!(matches!(
            build_asset_dag(&empty_row),
            Err(DagError::SkinMetadataMissing { .. })
        ));
        // 不一致臂:骨骼 id 越界(≥ joint_count)。
        let oob = DagAsset {
            mesh,
            skinned: Some(SkinWeights {
                vertex_influences: (0..n).map(|_| vec![(7u32, 1.0f32)]).collect(),
                joint_count: 2,
            }),
        };
        assert!(matches!(
            build_asset_dag(&oob),
            Err(DagError::SkinMetadataInconsistent { .. })
        ));
    }
}
