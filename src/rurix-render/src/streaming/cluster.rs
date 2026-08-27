//! cluster 流送（G31+ 波 C Task C11；RD-039 cluster 流送 P4 四行承接面；
//! milestones/g20/g20_cluster_streaming_p4_gap.json + milestones/g27/
//! g27_cluster_p4_rejudgment.json 逐行差距闭集的消费面）。
//!
//! 四行落地（全部加性；本文件之外的 streaming/ 既有面 0-byte 不动）：
//!
//! - **P4-1 页磁盘布局与驻留池**：[`ClusterPageResource`] = RXPD major=2
//!   （`rurix_geom_pages::disk_v2`，加性新版本面）页文件目录的
//!   [`PagedResource`] 实现——`read_page` = 磁盘真读，`transcode` = 冻结
//!   `decode_disk_page_v2` 解码 + `encode_logical_page_v2` 重编码（入池
//!   payload = RXPL v2 逻辑页映像，≤128KB 页契约由引擎断言复核）；驻留池
//!   = 既有 [`super::PagePool`]（128KB 固定槽 + LRU 逐出 + root 钉住 + 容量
//!   预算）零改动复用。
//! - **P4-2 GPU 请求反馈链 host 半**：[`lod_cut_with_residency`] 产
//!   [`PageRequest`]（与 device 请求缓冲同语义）；[`PriorityIoPool`] 异步
//!   读页 → 完成事件交 [`super::StreamingEngine::tick`] 入池 → 页表更新
//!   （harness 侧 page_state/pool 镜像上传）。
//! - **P4-3 LOD cut 与驻留联动**：[`lod_cut_with_residency`] = 驻留约束
//!   回退语义的 **host 金标准**——剔除三关 + LOD cut 全量复用
//!   `geometry::cull`（0-byte 金标准消费），随后对选中而缺页的簇沿父链
//!   回退到最近驻留祖先渲染（root 页钉住保证终止；禁空洞/禁错图的结构性
//!   来源）；[`verify_cut_cover`] = 覆盖不变量机器核验（每条选中链恰一个
//!   渲染祖先，无洞无重复覆盖）。
//! - **P4-4 异步 IO 优先级链**：[`PriorityIoPool`] 固定工作线程 +
//!   优先级堆（priority 降序、同序 FIFO 按提交 seq）真实磁盘读；优先级 =
//!   类目基值 + 屏幕重要度（[`screen_importance`]：投影直径量化——大屏
//!   占比/近处簇优先）；出队日志供优先级倒置正确性 measured 断言。

use std::collections::{BinaryHeap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};

use rurix_geom_pages::{decode_disk_page_v2, encode_logical_page_v2};

use crate::geometry::cull::{self, CullCamera, VisibleCluster};
use crate::geometry::gpu_scene::{InstanceRecord, NO_PARENT};
use crate::graph::types::{ClusterRecord, PageRequest};

use super::feedback::{FEEDBACK_BASE_GEOMETRY_LOD, FeedbackBuilder};
use super::resource::PagedResource;

/// cluster 页文件名（资源/页 → 页文件；harness 与资源实现的单一约定源）。
pub fn cluster_page_file_name(resource: u32, page: u32) -> String {
    format!("cluster_page_r{resource:03}_p{page:05}.rxpd2")
}

// ---------------------------------------------------------------------------
// P4-1：RXPD v2 页文件的 PagedResource 实现
// ---------------------------------------------------------------------------

/// RXPD major=2 页文件目录的流送资源（P4-1）。
///
/// 确定性契约（沿 [`PagedResource`] 字面）：
/// - `read_page` 双模式：**直读**（默认；页文件真实磁盘读，同页同字节——
///   参考臂/单测面）或**异步缓存**（[`ClusterPageResource::with_cache`]；
///   只消费 [`PriorityIoPool`] 完成事件填入的字节，未就绪 = 配置错误
///   panic——M37 `ready_raw` 同律「IO 未完成却进入 tick」fail-closed，
///   异步 IO 链唯一磁盘读路径，IO 量不双计）；
/// - `transcode` = `decode_disk_page_v2`（冻结解码器：checksum/版本/映射行
///   全校验）→ `encode_logical_page_v2` 重编码——payload = RXPL v2 逻辑页
///   映像（≤128KB；同输入同输出，host 单测逐字节锚定）；
/// - 解码失败 = 配置错误 panic（fail-closed，不静默降级——镜像引擎配置
///   错误即 panic 口径）。
#[derive(Debug)]
pub struct ClusterPageResource {
    resource_id: u32,
    dir: PathBuf,
    page_count: u32,
    root_pages: Vec<u32>,
    /// 异步完成字节缓存（Some = 异步模式；page → RXPD 原字节）。
    cache: Option<Arc<Mutex<HashMap<u32, Vec<u8>>>>>,
}

impl ClusterPageResource {
    pub fn new(resource_id: u32, dir: &Path, page_count: u32, root_pages: Vec<u32>) -> Self {
        Self {
            resource_id,
            dir: dir.to_path_buf(),
            page_count,
            root_pages,
            cache: None,
        }
    }

    /// 异步模式：`read_page` 只消费共享缓存（调用方以异步完成事件填充；
    /// 未就绪页进入 tick = 配置错误 panic，fail-closed）。
    pub fn with_cache(
        resource_id: u32,
        dir: &Path,
        page_count: u32,
        root_pages: Vec<u32>,
        cache: Arc<Mutex<HashMap<u32, Vec<u8>>>>,
    ) -> Self {
        Self {
            resource_id,
            dir: dir.to_path_buf(),
            page_count,
            root_pages,
            cache: Some(cache),
        }
    }

    /// 页文件路径（[`cluster_page_file_name`] 约定）。
    pub fn page_path(&self, page: u32) -> PathBuf {
        self.dir.join(cluster_page_file_name(self.resource_id, page))
    }

    /// 解码一页（冻结解码器直通；调用方自持校验语义用）。
    pub fn decode_page(
        &self,
        page: u32,
    ) -> Result<rurix_geom_pages::LogicalPageV2, rurix_geom_pages::DiskV2Error> {
        let raw = std::fs::read(self.page_path(page)).map_err(|_| {
            rurix_geom_pages::DiskV2Error::Truncated("file_missing")
        })?;
        decode_disk_page_v2(&raw)
    }
}

impl PagedResource for ClusterPageResource {
    fn resource_id(&self) -> u32 {
        self.resource_id
    }

    fn page_count(&self) -> u32 {
        self.page_count
    }

    fn root_pages(&self) -> &[u32] {
        &self.root_pages
    }

    fn read_page(&self, page: u32) -> Vec<u8> {
        if let Some(cache) = &self.cache {
            return cache
                .lock()
                .unwrap()
                .get(&page)
                .cloned()
                .unwrap_or_else(|| {
                    panic!(
                        "resource {} page {page} IO 未完成却进入 tick（异步链 fail-closed）",
                        self.resource_id
                    )
                });
        }
        let path = self.page_path(page);
        std::fs::read(&path)
            .unwrap_or_else(|e| panic!("resource {} page {page} 页文件读取失败 {path:?}: {e}", self.resource_id))
    }

    fn transcode(&self, page: u32, raw: &[u8]) -> Vec<u8> {
        let decoded = decode_disk_page_v2(raw).unwrap_or_else(|e| {
            panic!("resource {} page {page} RXPDv2 解码失败: {e}", self.resource_id)
        });
        encode_logical_page_v2(&decoded)
    }
}

// ---------------------------------------------------------------------------
// P4-3：驻留约束 LOD cut（host 金标准）+ 覆盖不变量
// ---------------------------------------------------------------------------

/// 全局簇 → 页归属与父链绑定（与全局簇表等长平行）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageBinding {
    /// 流送资源注册号（= 网格序）。
    pub resource: u32,
    /// 资源内页号（装箱产物）。
    pub page: u32,
    /// 父簇全局下标（根簇 = [`NO_PARENT`]；父链不跨网格）。
    pub parent: u32,
}

/// 渲染决定项（驻留约束后的实际渲染簇）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RenderEntry {
    /// 实例下标。
    pub instance: u32,
    /// 实际渲染簇（全局下标；= 选中簇或最近驻留祖先）。
    pub cluster: u32,
    /// LOD cut 原始选中簇（全局下标；回退诊断面）。
    pub selected: u32,
    /// 是否发生父级回退。
    pub fell_back: bool,
}

/// [`lod_cut_with_residency`] 产物。
#[derive(Debug, Clone, Default)]
pub struct CutOutcome {
    /// 渲染决定表（稳定序：可见实例序 × 簇段升序——`cluster_cull` 同律；
    /// 一致性 cut：同祖先多链合并，渲染集对 DAG 是合法 cut）。
    pub render: Vec<RenderEntry>,
    /// 缺页请求（去重后；优先级 = 类目基值 + 屏幕重要度）。
    pub requests: Vec<PageRequest>,
    /// LOD cut 选中簇数（全驻留语义下的可见集大小）。
    pub selected_count: u32,
    /// 选中页未驻留的链数（缺页率分子）。
    pub miss_selected_count: u32,
    /// 最终渲染 ≠ 选中簇的链数（回退率分子；含一致性合并被迫变粗的链）。
    pub fallback_count: u32,
}

/// 屏幕重要度量化（P4-4 优先级源）：投影直径 px → [0, 65535] 整数。
///
/// 直径 ∝ 半径/距离 ⇒ 大屏占比/近处簇重要度高（NaN/负值钳 0，+∞ 钳顶——
/// 确定性全序，禁浮点比较进调度）。
pub fn screen_importance(diameter_px: f32) -> u32 {
    if !diameter_px.is_finite() {
        return u16::MAX as u32;
    }
    diameter_px.clamp(0.0, u16::MAX as f32).round() as u32
}

/// 驻留约束 LOD cut（P4-3 host 金标准；P4-2 请求语义源）。
///
/// 两阶段：
/// 1. **全驻留金标准**：`instance_cull` + `cluster_cull`（geometry::cull
///    0-byte 消费）产 LOD cut 选中集（稳定序）；
/// 2. **驻留约束一致性 cut**：每链（选中簇 → 根）取最近驻留祖先-or-自身
///    （root 页钉住 ⇒ 链上必有驻留点，回退必然终止），随后**回退级联合
///    并**——回退产物祖先的全体选中后代链并入该祖先（渲染集对 DAG 恒为
///    合法 cut：每条 root→叶路径恰一节点），禁空洞 ∧ 禁重复覆盖（禁错
///    图）的结构性保证；无回退时零合并，全驻留渲染集与生产
///    `cluster_cull` 原始选中逐字一致。对选中缺页链发 [`PageRequest`]
///    （优先级 = `FEEDBACK_BASE_GEOMETRY_LOD` +
///    [`screen_importance`]（选中簇投影直径））。
///
/// `is_resident(resource, page)` = 驻留查询（引擎 pool 镜像；调用方保证与
/// 本帧页表上传态一致——host/device 双臂共用同一查询语义即对拍面）。
pub fn lod_cut_with_residency(
    instances: &[InstanceRecord],
    clusters: &[ClusterRecord],
    bindings: &[PageBinding],
    cam: &CullCamera,
    frame: u32,
    is_resident: &mut dyn FnMut(u32, u32) -> bool,
) -> CutOutcome {
    debug_assert_eq!(clusters.len(), bindings.len());
    let visible_instances = cull::instance_cull(instances, cam);
    let selected = cull::cluster_cull(instances, &visible_instances, clusters, cam);
    let mut out = CutOutcome {
        selected_count: selected.len() as u32,
        ..CutOutcome::default()
    };
    let mut fb = FeedbackBuilder::new(frame);
    // 阶段 2a：每链最近驻留祖先-or-自身（稳定序平行表）。
    let mut chain_node: Vec<u32> = Vec::with_capacity(selected.len());
    for vc in &selected {
        let b = bindings[vc.cluster as usize];
        let node = if is_resident(b.resource, b.page) {
            vc.cluster
        } else {
            out.miss_selected_count += 1;
            // 屏幕重要度 = 选中簇包围球投影直径（近处/大屏占比优先）。
            let inst = &instances[vc.instance as usize];
            let c = &clusters[vc.cluster as usize];
            let (center_w, radius_w, _) = cull::world_sphere(&inst.transform, c);
            let d = [
                center_w[0] - cam.cam_pos[0],
                center_w[1] - cam.cam_pos[1],
                center_w[2] - cam.cam_pos[2],
            ];
            let dist = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            let importance = screen_importance(cam.projected_diameter_px(radius_w, dist));
            fb.add(b.resource, b.page, FEEDBACK_BASE_GEOMETRY_LOD, importance);
            nearest_resident_ancestor(bindings, vc.cluster, is_resident)
        };
        chain_node.push(node);
    }
    // 阶段 2b/2c：一致性归一（device 读回面与 host 金标准同律单源；
    // 回退产物掩码 = 链节点经回退步行产生——合并只级联自真实回退）。
    let chain_fallback: Vec<bool> = chain_node
        .iter()
        .zip(selected.iter())
        .map(|(&n, vc)| n != vc.cluster)
        .collect();
    let (render, fallback_count) =
        normalize_render_decisions(&selected, &chain_node, &chain_fallback, bindings);
    out.render = render;
    out.fallback_count = fallback_count;
    out.requests = fb.build();
    out
}

/// 一致性 cut 归一（P4-3；host 金标准与 device 读回归一共用单源）：
/// 输入 = 选中链（稳定序）+ 每链渲染节点（最近驻留祖先-or-自身）+ 回退
/// 产物掩码（链节点经回退步行产生 = true）；
/// **合并只级联自真实回退产物**：渲染节点 A 为回退产物时,A 的全体选中
/// 后代链并入 A（A 粗化覆盖整子树,后代链再渲染 = 重复覆盖禁错图）;
/// 无回退时**零合并**——全驻留渲染集与生产 `cluster_cull` 原始选中逐字
/// 一致（投影边界祖先/后代共选 = 生产语义既存边界态,不在本面重定;
/// bistro 实证:同误差值跨距离投跨界两边 → 共选,合并与否须以回退为条件,
/// 否则全驻留参考与生产 cut 偏离）。同实例内（父链不跨网格故跨实例无祖
/// 先关系）+ 稳定序首现去重;返回（渲染决定表, 渲染 ≠ 选中的链数）。
pub fn normalize_render_decisions(
    selected: &[VisibleCluster],
    chain_node: &[u32],
    chain_fallback: &[bool],
    bindings: &[PageBinding],
) -> (Vec<RenderEntry>, u32) {
    debug_assert_eq!(selected.len(), chain_node.len());
    debug_assert_eq!(selected.len(), chain_fallback.len());
    let mut chain_node = chain_node.to_vec();
    let mut chain_fallback = chain_fallback.to_vec();
    loop {
        let mut merged = false;
        'outer: for i in 0..selected.len() {
            for j in 0..selected.len() {
                if i == j || selected[i].instance != selected[j].instance {
                    continue;
                }
                if chain_fallback[j]
                    && chain_node[i] != chain_node[j]
                    && is_proper_ancestor(bindings, chain_node[j], chain_node[i])
                {
                    chain_node[i] = chain_node[j];
                    chain_fallback[i] = true;
                    merged = true;
                    break 'outer;
                }
            }
        }
        if !merged {
            break;
        }
    }
    let mut render = Vec::with_capacity(chain_node.len());
    let mut fallback_count = 0u32;
    let mut seen = std::collections::HashSet::new();
    for (i, vc) in selected.iter().enumerate() {
        let node = chain_node[i];
        if chain_fallback[i] {
            fallback_count += 1;
        }
        if seen.insert((vc.instance, node)) {
            render.push(RenderEntry {
                instance: vc.instance,
                cluster: node,
                selected: vc.cluster,
                fell_back: chain_fallback[i],
            });
        }
    }
    (render, fallback_count)
}

/// `anc` 是否 `desc` 的真祖先（沿父链严格上行可达；同实例/同网格内）。
fn is_proper_ancestor(bindings: &[PageBinding], anc: u32, desc: u32) -> bool {
    let mut cur = desc;
    while cur != anc {
        let parent = bindings[cur as usize].parent;
        if parent == NO_PARENT {
            return false;
        }
        cur = parent;
    }
    true
}

/// 沿父链找最近驻留祖先（root 页钉住 ⇒ 必然命中；命中前断言不回绕）。
fn nearest_resident_ancestor(
    bindings: &[PageBinding],
    selected: u32,
    is_resident: &mut dyn FnMut(u32, u32) -> bool,
) -> u32 {
    let mut cur = selected;
    loop {
        let parent = bindings[cur as usize].parent;
        if parent == NO_PARENT {
            // 根簇：其页 = root 页（注册期钉住常驻）。root 未驻留 = 引擎契约
            // 破坏（配置错误），此处确定性 panic 不静默（禁错图红线）。
            let b = bindings[cur as usize];
            assert!(
                is_resident(b.resource, b.page),
                "root 簇 {cur} 页 (r{}, p{}) 未驻留——root 钉住契约破坏",
                b.resource,
                b.page
            );
            return cur;
        }
        let pb = bindings[parent as usize];
        if is_resident(pb.resource, pb.page) {
            return parent;
        }
        cur = parent;
    }
}

/// 覆盖不变量机器核验（P4-3 禁空洞/禁错图的结构判据；四联式）：
/// - **(a) 无洞**：每条选中链（选中簇 → 根）至少含一个渲染簇（祖先或自身）
///   ——比生产基线更强的保证（空洞零容忍）；
/// - **(b) 无凭空粗化**：每个非选中渲染节点（= 回退产物）必有选中真后代
///   （回退必有所本）；
/// - **(c) 无重复**：同一 (instance, cluster) 不出两条渲染决定；
/// - **(d) 流送引入重复覆盖零容忍**：回退产物与其任一真后代不同帧渲染
///   （级联合并的机器核验对偶面）。
/// 生产边界共选（祖先/后代同帧入选——投影边界态,`cluster_cull` 全驻留既
/// 存语义）不在本判据重定：两者皆为选中节点,(d) 只约束回退产物。
pub fn verify_cut_cover(
    selected: &[VisibleCluster],
    render: &[RenderEntry],
    bindings: &[PageBinding],
) -> bool {
    let mut rendered = std::collections::HashSet::new();
    for e in render {
        if !rendered.insert((e.instance, e.cluster)) {
            return false; // (c) 重复覆盖
        }
    }
    let selected_set: std::collections::HashSet<(u32, u32)> = selected
        .iter()
        .map(|vc| (vc.instance, vc.cluster))
        .collect();
    // (a) 无洞：每链 ≥1 渲染祖先-or-自身。
    for vc in selected {
        let mut cur = vc.cluster;
        let mut covered = false;
        loop {
            if rendered.contains(&(vc.instance, cur)) {
                covered = true;
                break;
            }
            let parent = bindings[cur as usize].parent;
            if parent == NO_PARENT {
                break;
            }
            cur = parent;
        }
        if !covered {
            return false;
        }
    }
    // (b)+(d)：非选中渲染节点（回退产物）双约束。
    for e in render {
        if selected_set.contains(&(e.instance, e.cluster)) {
            continue;
        }
        let has_selected_descendant = selected.iter().any(|vc| {
            vc.instance == e.instance
                && is_proper_ancestor(bindings, e.cluster, vc.cluster)
        });
        if !has_selected_descendant {
            return false; // (b) 凭空粗化
        }
        for other in render {
            if other.cluster != e.cluster
                && other.instance == e.instance
                && is_proper_ancestor(bindings, e.cluster, other.cluster)
            {
                return false; // (d) 回退产物与真后代同渲染
            }
        }
    }
    true
}

// ---------------------------------------------------------------------------
// P4-4：优先级异步 IO 池
// ---------------------------------------------------------------------------

/// 异步读页完成事件（真实磁盘读产物；`raw` = 页文件原字节）。
#[derive(Debug)]
pub struct IoCompletion {
    pub resource: u32,
    pub page: u32,
    pub priority: u32,
    pub frame: u32,
    pub raw: Vec<u8>,
}

/// 出队事件（优先级倒置正确性 measured 面；出队序 = 调度序的实证）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DequeueEvent {
    pub seq: u64,
    pub priority: u32,
    pub resource: u32,
    pub page: u32,
}

struct QueuedRead {
    priority: u32,
    seq: u64,
    resource: u32,
    page: u32,
    frame: u32,
    path: PathBuf,
}

impl PartialEq for QueuedRead {
    fn eq(&self, other: &Self) -> bool {
        (self.priority, self.seq) == (other.priority, other.seq)
    }
}
impl Eq for QueuedRead {}
impl PartialOrd for QueuedRead {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for QueuedRead {
    /// 堆顶 = 最高优先级；同级取最小 seq（提交 FIFO）——调度序对任意入队
    /// 时序确定性（优先级倒置正确性的结构来源）。
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority
            .cmp(&other.priority)
            .then(other.seq.cmp(&self.seq))
    }
}

struct Pending {
    heap: BinaryHeap<QueuedRead>,
    closed: bool,
}

/// 优先级异步读页线程池（P4-4）。
///
/// 形态（M37 `StreamIoPool` 先例加性演进——FIFO mpsc → 优先级堆）：
/// 固定 N 工作线程；`submit` 入堆（priority 降序 + seq FIFO tie-break）；
/// 工作线程弹堆顶做真实 `std::fs::read` 并经 mpsc 回完成事件。调度序与
/// 墙钟无关（同 pending 集 ⇒ 同弹出序）；真实读字节计量进
/// [`PriorityIoPool::bytes_read_total`]。
///
/// 生命周期：构造即起线程（阻塞待 [`PriorityIoPool::start`]）；Drop 关闭
/// 并 join（确定性停机，无线程泄漏）。
pub struct PriorityIoPool {
    pending: Arc<(Mutex<Pending>, Condvar)>,
    started: Arc<(Mutex<bool>, Condvar)>,
    done_rx: Receiver<IoCompletion>,
    workers: Vec<JoinHandle<()>>,
    bytes_read_total: Arc<AtomicU64>,
    submitted: Arc<AtomicU64>,
    completed: Arc<AtomicU64>,
    dequeue_log: Arc<Mutex<Vec<DequeueEvent>>>,
    next_seq: AtomicU64,
}

impl PriorityIoPool {
    pub fn new(worker_count: usize) -> Self {
        assert!(worker_count >= 1, "IO 工作线程至少 1");
        let pending = Arc::new((
            Mutex::new(Pending {
                heap: BinaryHeap::new(),
                closed: false,
            }),
            Condvar::new(),
        ));
        let started = Arc::new((Mutex::new(false), Condvar::new()));
        let (done_tx, done_rx) = mpsc::channel::<IoCompletion>();
        let bytes_read_total = Arc::new(AtomicU64::new(0));
        let completed = Arc::new(AtomicU64::new(0));
        let dequeue_log = Arc::new(Mutex::new(Vec::new()));
        let mut workers = Vec::with_capacity(worker_count);
        for _ in 0..worker_count {
            let pending = Arc::clone(&pending);
            let started = Arc::clone(&started);
            let done_tx: Sender<IoCompletion> = done_tx.clone();
            let bytes = Arc::clone(&bytes_read_total);
            let completed = Arc::clone(&completed);
            let log = Arc::clone(&dequeue_log);
            workers.push(thread::spawn(move || {
                // 开工闸：start() 前阻塞（探针确定性入队窗）。
                {
                    let (lock, cvar) = &*started;
                    let mut s = lock.lock().unwrap();
                    while !*s {
                        s = cvar.wait(s).unwrap();
                    }
                }
                loop {
                    let job = {
                        let (lock, cvar) = &*pending;
                        let mut p = lock.lock().unwrap();
                        loop {
                            if let Some(j) = p.heap.pop() {
                                break Some(j);
                            }
                            if p.closed {
                                break None;
                            }
                            p = cvar.wait(p).unwrap();
                        }
                    };
                    let Some(job) = job else { break };
                    log.lock().unwrap().push(DequeueEvent {
                        seq: job.seq,
                        priority: job.priority,
                        resource: job.resource,
                        page: job.page,
                    });
                    let raw = std::fs::read(&job.path).unwrap_or_default();
                    bytes.fetch_add(raw.len() as u64, Ordering::Relaxed);
                    completed.fetch_add(1, Ordering::Relaxed);
                    let _ = done_tx.send(IoCompletion {
                        resource: job.resource,
                        page: job.page,
                        priority: job.priority,
                        frame: job.frame,
                        raw,
                    });
                }
            }));
        }
        Self {
            pending,
            started,
            done_rx,
            workers,
            bytes_read_total,
            submitted: Arc::new(AtomicU64::new(0)),
            completed,
            dequeue_log,
            next_seq: AtomicU64::new(0),
        }
    }

    /// 开工（构造后一次性；此前 submit 的请求按优先级整体调度——优先级
    /// 倒置探针的确定性入队窗）。
    pub fn start(&self) {
        let (lock, cvar) = &*self.started;
        *lock.lock().unwrap() = true;
        cvar.notify_all();
    }

    /// 提交读页请求（path = 页文件；priority 大者先读，同级先提交先读）。
    pub fn submit(&self, path: PathBuf, resource: u32, page: u32, priority: u32, frame: u32) {
        let seq = self.next_seq.fetch_add(1, Ordering::Relaxed);
        let (lock, cvar) = &*self.pending;
        lock.lock().unwrap().heap.push(QueuedRead {
            priority,
            seq,
            resource,
            page,
            frame,
            path,
        });
        cvar.notify_one();
        self.submitted.fetch_add(1, Ordering::Relaxed);
    }

    /// 非阻塞取一件完成事件。
    pub fn try_recv(&self) -> Option<IoCompletion> {
        self.done_rx.try_recv().ok()
    }

    pub fn bytes_read_total(&self) -> u64 {
        self.bytes_read_total.load(Ordering::Relaxed)
    }

    pub fn submitted_count(&self) -> u64 {
        self.submitted.load(Ordering::Relaxed)
    }

    pub fn completed_count(&self) -> u64 {
        self.completed.load(Ordering::Relaxed)
    }

    /// 出队日志快照（提交/优先级序对照 = 倒置正确性 measured 面）。
    pub fn dequeue_log(&self) -> Vec<DequeueEvent> {
        self.dequeue_log.lock().unwrap().clone()
    }
}

impl Drop for PriorityIoPool {
    fn drop(&mut self) {
        {
            let (lock, cvar) = &*self.pending;
            lock.lock().unwrap().closed = true;
            cvar.notify_all();
        }
        self.start(); // 未开工亦放行停机路径
        for w in self.workers.drain(..) {
            let _ = w.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::gpu_scene::IDENTITY_3X4;
    use crate::graph::types::StreamingBudget;
    use crate::streaming::StreamingEngine;
    use rurix_geom_pages::logical_v2::{LogicalPageV2, V2ClusterExt};
    use rurix_geom_pages::{
        FLAG_ROOT, LogicalPage, PageClusterRecord, encode_disk_page_v2, quantize_center,
    };

    fn write_page(dir: &Path, resource: u32, page_id: u64, root: bool, cluster: u32) {
        let bounds = [-1.0, -1.0, -1.0, 1.0, 1.0, 1.0];
        let center = [0.1, 0.2, 0.3];
        let (qx, qy, qz) = quantize_center(center, bounds);
        let page = LogicalPageV2 {
            base: LogicalPage {
                page_id,
                flags: if root { FLAG_ROOT } else { 0 },
                lod_level_min: 0,
                lod_level_max: 0,
                bounds,
                clusters: vec![PageClusterRecord {
                    cluster_id: cluster,
                    qx,
                    qy,
                    qz,
                    center,
                    radius: 1.0,
                    cone_axis: [0.0, 1.0, 0.0],
                    cone_cutoff: 0.0,
                    error: 0.0,
                    parent_error: 1.0,
                    vertex_offset: 0,
                    triangle_offset: 0,
                    vertex_count: 3,
                    triangle_count: 1,
                    level: 0,
                    group: 0,
                }],
                vertices: vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                indices: vec![0, 1, 2],
                dependency_page_ids: vec![],
                dag_links: vec![],
            },
            ext: vec![V2ClusterExt::unskinned([-1.0; 3], [1.0; 3])],
        };
        std::fs::write(
            dir.join(cluster_page_file_name(resource, page_id as u32)),
            encode_disk_page_v2(&page),
        )
        .unwrap();
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "rurix_p4_cluster_test_{}_{}_{}",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// P4-1：资源读页/transcode 逐字节锚定（payload = RXPL v2 映像重编码）。
    #[test]
    fn resource_read_transcode_byte_anchor() {
        let dir = temp_dir("res");
        write_page(&dir, 1, 0, true, 0);
        write_page(&dir, 1, 1, false, 1);
        let res = ClusterPageResource::new(1, &dir, 2, vec![0]);
        assert_eq!(res.resource_id(), 1);
        assert_eq!(res.page_count(), 2);
        assert_eq!(res.root_pages(), &[0]);
        let raw = res.read_page(0);
        let on_disk = std::fs::read(res.page_path(0)).unwrap();
        assert_eq!(raw, on_disk);
        let payload = res.transcode(0, &raw);
        let expect = encode_logical_page_v2(&decode_disk_page_v2(&on_disk).unwrap());
        assert_eq!(payload, expect);
        assert!(payload.len() <= crate::graph::types::STREAM_PAGE_SIZE as usize);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// P4-1：引擎注册 root 钉住 + LRU 高压逐出（驻留池行为经 cluster 资源复核）。
    #[test]
    fn engine_pool_lru_under_pressure() {
        let dir = temp_dir("pool");
        for p in 0..6u32 {
            write_page(&dir, 1, u64::from(p), p == 0, p);
        }
        let mut engine = StreamingEngine::new(3);
        engine.register_resource(Box::new(ClusterPageResource::new(1, &dir, 6, vec![0])));
        assert!(engine.is_resident(1, 0));
        let b = StreamingBudget {
            io_bytes: u64::MAX,
            transcode_bytes: u64::MAX,
            upload_bytes: u64::MAX,
        };
        // 逐页请求：池 3 槽（1 root 钉住 + 2 流动）⇒ 第 3 页起逐出。
        for (f, p) in (1..6u32).enumerate() {
            engine.submit_requests(&[PageRequest {
                resource: 1,
                page_index: p,
                priority: 1,
                frame: f as u32,
            }]);
            engine.tick(f as u32, &b);
            assert!(engine.is_resident(1, p));
            assert!(engine.is_resident(1, 0), "root 永不逐出");
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    /// P4-3：两级合成 DAG 的驻留回退 + 覆盖不变量（禁空洞）。
    #[test]
    fn lod_cut_residency_fallback_and_cover() {
        // 实例单网格：簇 0/1 = 叶（error 0），簇 2 = 根（parent_error +∞）。
        let mk = |id_error: (u32, f32, f32)| ClusterRecord {
            center: [0.0, 0.0, -5.0],
            radius: 0.5,
            cone_axis: [0.0, 1.0, 0.0],
            cone_cutoff: 1.0, // 锥剔禁用
            error: id_error.1,
            parent_error: id_error.2,
            vertex_offset: 0,
            triangle_offset: 0,
            vertex_count: 3,
            triangle_count: 1,
            page_id: 0,
            reserved: 0,
        };
        let clusters = vec![mk((0, 0.0, 4.0)), mk((1, 0.0, 4.0)), mk((2, 4.0, f32::INFINITY))];
        let bindings = vec![
            PageBinding { resource: 1, page: 1, parent: 2 },
            PageBinding { resource: 1, page: 2, parent: 2 },
            PageBinding { resource: 1, page: 0, parent: NO_PARENT },
        ];
        let inst = InstanceRecord {
            transform: IDENTITY_3X4,
            cluster_offset: 0,
            cluster_count: 3,
            material_id: 0,
            flags: 0,
            aabb_min: [-1.0, -1.0, -6.0],
            mesh_id: 0,
            aabb_max: [1.0, 1.0, -4.0],
            reserved: NO_PARENT,
        };
        let cam = CullCamera {
            view_proj: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, -1.0, -0.2],
                [0.0, 0.0, -1.0, 0.0],
            ],
            cam_pos: [0.0, 0.0, 0.0],
            screen_height_px: 1080.0,
            error_threshold_px: 1.0,
        };
        // 全驻留：渲染 = cut 自身（叶 0/1），零回退零请求。
        let sel = cull::cluster_cull(&[inst], &cull::instance_cull(&[inst], &cam), &clusters, &cam);
        let out = lod_cut_with_residency(&[inst], &clusters, &bindings, &cam, 0, &mut |_, _| true);
        assert_eq!(out.selected_count, sel.len() as u32);
        assert_eq!(out.fallback_count, 0);
        assert_eq!(out.miss_selected_count, 0);
        assert!(out.requests.is_empty());
        assert!(verify_cut_cover(&sel, &out.render, &bindings));
        // 叶 0 页缺：链 0 回退根 2；一致性 cut 合并 ⇒ 链 1 同渲染根 2（禁
        // 重复覆盖：根 2 覆盖叶 0 区域，叶 1 不得再渲染）；仅缺页链发请求。
        let out2 = lod_cut_with_residency(&[inst], &clusters, &bindings, &cam, 1, &mut |r, p| {
            (r, p) != (1, 1)
        });
        assert_eq!(out2.miss_selected_count, 1);
        assert_eq!(out2.fallback_count, 2, "链 0 缺页回退 + 链 1 一致性合并变粗");
        assert_eq!(out2.render.len(), 1, "同祖先合并渲染一次");
        assert_eq!(out2.render[0].cluster, 2);
        assert!(out2.render[0].fell_back);
        assert_eq!(out2.requests.len(), 1);
        assert_eq!((out2.requests[0].resource, out2.requests[0].page_index), (1, 1));
        assert!(out2.requests[0].priority >= FEEDBACK_BASE_GEOMETRY_LOD);
        assert!(verify_cut_cover(&sel, &out2.render, &bindings));
        // 叶 0/1 全缺：两链都回退根 2（同祖先合并 = 一条渲染决定）。
        let out3 = lod_cut_with_residency(&[inst], &clusters, &bindings, &cam, 2, &mut |r, p| {
            (r, p) == (1, 0)
        });
        assert_eq!(out3.miss_selected_count, 2);
        assert_eq!(out3.fallback_count, 2);
        let roots = out3.render.iter().filter(|e| e.cluster == 2).count();
        assert_eq!(roots, 1, "同祖先回退须合并渲染（禁重复覆盖）");
        assert_eq!(out3.requests.len(), 2);
        assert!(verify_cut_cover(&sel, &out3.render, &bindings));
    }

    /// P4-3 语义锚：无回退产物时归一**零合并**——全驻留渲染集与生产
    /// `cluster_cull` 原始选中逐字一致（投影边界祖先/后代共选 = 生产语义
    /// 既存边界态,bistro 实证帧 f5 锚;合并只级联自真实回退）。
    #[test]
    fn normalize_no_fallback_no_merge_boundary_coselection() {
        let bindings = vec![
            PageBinding { resource: 1, page: 1, parent: 2 },
            PageBinding { resource: 1, page: 2, parent: 2 },
            PageBinding { resource: 1, page: 0, parent: NO_PARENT },
        ];
        // 手工共选：叶 0 与根 2 同帧入选（投影边界态,全驻留 → 掩码全 false）。
        let selected = vec![
            VisibleCluster { instance: 0, cluster: 0 },
            VisibleCluster { instance: 0, cluster: 2 },
        ];
        let nodes = vec![0, 2];
        let mask = vec![false, false];
        let (render, fallback) = normalize_render_decisions(&selected, &nodes, &mask, &bindings);
        assert_eq!(fallback, 0, "无回退产物 ⇒ 零合并（生产 cut 逐字）");
        assert_eq!(render.len(), 2, "边界共选两簇均渲染（生产语义既存边界态）");
        assert!(render.iter().any(|e| e.cluster == 0) && render.iter().any(|e| e.cluster == 2));
        // 对照：同共选但链 0 为回退产物（掩码 true；链 0 回退到根 2,链 1
        // 本身选中根 2）⇒ 同节点去重单渲染,回退仅计链 0（链 1 渲染 = 选中）。
        let nodes2 = vec![2, 2];
        let mask2 = vec![true, false];
        let (render2, fallback2) = normalize_render_decisions(&selected, &nodes2, &mask2, &bindings);
        assert_eq!(render2.len(), 1, "同祖先去重单渲染（禁重复覆盖）");
        assert_eq!(render2[0].cluster, 2);
        assert_eq!(fallback2, 1);
        // 级联面：链 1 选中叶 1（根 2 后代）而链 0 回退根 2 ⇒ 链 1 并入回退
        // 产物（重复覆盖消除）,回退计 2。
        let selected3 = vec![
            VisibleCluster { instance: 0, cluster: 0 },
            VisibleCluster { instance: 0, cluster: 1 },
        ];
        let nodes3 = vec![2, 1];
        let mask3 = vec![true, false];
        let (render3, fallback3) = normalize_render_decisions(&selected3, &nodes3, &mask3, &bindings);
        assert_eq!(render3.len(), 1, "回退产物后代链并入");
        assert_eq!(render3[0].cluster, 2);
        assert_eq!(fallback3, 2);
    }

    /// P4-4：重要度单调（近处 > 远处）+ 钳制确定性。
    #[test]
    fn screen_importance_monotonic() {
        let near = screen_importance(400.0);
        let far = screen_importance(12.0);
        assert!(near > far);
        assert_eq!(screen_importance(f32::NAN), u16::MAX as u32);
        assert_eq!(screen_importance(f32::INFINITY), u16::MAX as u32);
        assert_eq!(screen_importance(-3.0), 0);
    }

    /// P4-4：优先级倒置正确性 measured——开工闸前入队 [低×3, 高×1]，
    /// 单 worker 出队序 = 高优先级先行 + 同级 FIFO；读字节计量真实。
    #[test]
    fn priority_pool_inversion_order_measured() {
        let dir = temp_dir("io");
        for p in 0..4u32 {
            write_page(&dir, 1, u64::from(p), p == 0, p);
        }
        let pool = PriorityIoPool::new(1);
        for p in 1..4u32 {
            pool.submit(
                dir.join(cluster_page_file_name(1, p)),
                1,
                p,
                10,
                0,
            );
        }
        pool.submit(dir.join(cluster_page_file_name(1, 0)), 1, 0, 999, 0);
        pool.start();
        let mut got = Vec::new();
        for _ in 0..4 {
            for _ in 0..1000 {
                if let Some(c) = pool.try_recv() {
                    got.push(c);
                    break;
                }
                thread::sleep(std::time::Duration::from_millis(1));
            }
        }
        assert_eq!(got.len(), 4);
        assert_eq!(got[0].page, 0, "高优先级先驻留（倒置校正）");
        assert_eq!(got[1].page, 1, "同级 FIFO");
        assert_eq!(got[2].page, 2);
        assert_eq!(got[3].page, 3);
        let log = pool.dequeue_log();
        assert_eq!(log.len(), 4);
        assert_eq!(log[0].priority, 999);
        assert!(pool.bytes_read_total() > 0);
        assert_eq!(pool.completed_count(), 4);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// P4-2 host 闭环：请求 → 异步读 → tick 入池 → 次帧零请求（驻留）。
    #[test]
    fn request_residency_closed_loop_host() {
        let dir = temp_dir("loop");
        for p in 0..4u32 {
            write_page(&dir, 1, u64::from(p), p == 0, p);
        }
        let mut engine = StreamingEngine::new(4);
        engine.register_resource(Box::new(ClusterPageResource::new(1, &dir, 4, vec![0])));
        let pool = PriorityIoPool::new(2);
        pool.start();
        let b = StreamingBudget {
            io_bytes: u64::MAX,
            transcode_bytes: u64::MAX,
            upload_bytes: u64::MAX,
        };
        // 帧 0：提交缺页请求 → 派读。
        engine.submit_requests(&[PageRequest {
            resource: 1,
            page_index: 2,
            priority: FEEDBACK_BASE_GEOMETRY_LOD + 100,
            frame: 0,
        }]);
        let r0 = engine.tick(0, &b);
        assert_eq!(r0.pages_loaded, 1, "同步腿引擎直读入池（资源 read_page 真读）");
        assert!(engine.is_resident(1, 2));
        // 异步腿：派读页 3 → 完成事件 → 次帧 tick 前已由 submit 触驻留查询。
        pool.submit(dir.join(cluster_page_file_name(1, 3)), 1, 3, 5, 1);
        let mut arrived = None;
        for _ in 0..2000 {
            if let Some(c) = pool.try_recv() {
                arrived = Some(c);
                break;
            }
            thread::sleep(std::time::Duration::from_millis(1));
        }
        let c = arrived.expect("异步读页完成");
        assert_eq!(c.page, 3);
        let decoded = decode_disk_page_v2(&c.raw).unwrap();
        assert_eq!(decoded.base.page_id, 3);
        std::fs::remove_dir_all(&dir).ok();
    }
}
