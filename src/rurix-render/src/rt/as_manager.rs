//! AS 管理器 host 模型(报告4 §2.2 BLAS/TLAS 生命周期;RFC-0016 章 F 策略单源)。
//!
//! 显式策略(报告4 §5「加速结构债」缓解:缓存键 / refit 决策树 / compaction 时机
//! 写成显式策略并配监控计数器):
//!
//! - **BLAS 缓存**:`BlasKey` = 网格内容哈希(positions+indices 的 FNV-1a),
//!   [`BlasCache::get_or_build`] 命中即复用不重建(`AsStats::blas_builds` 锚定);
//! - **动态分级**:`DynamicPolicy` = `Static`(不 refit)/ `Deformable`(refit 队列
//!   轮转摊销,`refit_budget_frames` 帧窗口)/ `FullRebuild`(拒绝 refit,走
//!   [`BlasCache::rebuild`]);变形超阈判据 = 任一叶 AABB 膨胀比 > 2
//!   ([`RefitReport::needs_rebuild`],报告4 refit 分级口径);
//! - **TLAS 快速重建**:[`TlasBuilder`] 每帧全量重建实例级 BVH(报告4:实例数 <10k
//!   亚毫秒级,不值得增量结构);实例变换更新标脏,`rebuild_if_dirty` 干净帧零成本;
//! - **compaction 留口**:[`BlasCache::evict`] 显式驱逐 + 槽位复用(host 模型对应
//!   device 的 VkCompaction;device 压缩/scratch 归下一波 device 腿)。
//!
//! 统计面 `AsStats`(blas_builds / refits / tlas_rebuilds)= evidence 埋点计数源。

use std::collections::HashMap;
use std::fmt;

use crate::rt::bvh::{
    BlasSet, InstanceDesc as BvhInstanceDesc, RefitError, RefitReport, Tlas, Transform3x4, TriBvh,
};

// ---------------------------------------------------------------------------
// 键与句柄
// ---------------------------------------------------------------------------

/// BLAS 缓存键:网格内容哈希(positions+indices 的 FNV-1a 64 位)。
///
/// 逐元素按位模式混合(f32 取 `to_bits`、u32 直取),与字节序无关、跨平台确定;
/// `+0.0` 与 `-0.0` 位模式不同视为不同内容(内容哈希 = 位级精确)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlasKey(pub u64);

impl BlasKey {
    /// 计算网格内容哈希(FNV-1a 64:offset 0xcbf29ce484222325,prime 0x100000001b3)。
    pub fn from_mesh(positions: &[[f32; 3]], indices: &[[u32; 3]]) -> Self {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut mix = |v: u64| {
            h = (h ^ v).wrapping_mul(0x0000_0100_0000_01b3);
        };
        for p in positions {
            for &c in p {
                mix(u64::from(c.to_bits()));
            }
        }
        for t in indices {
            for &vi in t {
                mix(u64::from(vi));
            }
        }
        BlasKey(h)
    }
}

/// BLAS 句柄(缓存槽位下标;evict 后失效,槽位可被后续构建复用)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlasId(pub u32);

// ---------------------------------------------------------------------------
// 动态策略分级(报告4 §2.2:refit 用于变形、rebuild 用于拓扑变化)
// ---------------------------------------------------------------------------

/// 动态 BLAS 更新策略(决策树显式化;首次 `get_or_build` 时的策略为准,变更需 evict
/// 后重建缓存项)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicPolicy {
    /// 静态:永不 refit(一次构建 + 后续 compaction;报告4:静态几何顶点还可压 16 位,
    /// host 模型不量化,留 device 腿)。
    Static,
    /// 可变形:refit 队列轮转摊销。`refit_budget_frames` = 摊销窗口(帧):窗口开启
    /// 时定型每帧份额 = ceil(队长 / 最紧预算),份额在窗口内恒定,保证窗口内全部脏
    /// BLAS 在 ≤ 预算帧内完成 refit(混合预算取最紧,保鲜优先);0 视同 1(次帧即清)。
    Deformable { refit_budget_frames: u32 },
    /// 拓扑可变(顶点数变化):拒绝 refit,一切更新走 [`BlasCache::rebuild`] 全量重建。
    FullRebuild,
}

// ---------------------------------------------------------------------------
// 统计面与错误
// ---------------------------------------------------------------------------

/// AS 管理统计计数器(evidence 埋点源;单调递增,快照语义)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AsStats {
    /// BLAS 全量构建次数(get_or_build 缓存未命中 + rebuild 拓扑重建)。
    pub blas_builds: u64,
    /// BLAS refit 次数(直接 refit + 队列摊销 refit)。
    pub refits: u64,
    /// TLAS 重建次数(TlasBuilder::rebuild)。
    pub tlas_rebuilds: u64,
}

/// AS 管理器错误(确定性拒绝;调用契约违例)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsError {
    /// 未知/已 evict 的 BLAS 句柄。
    UnknownBlas(BlasId),
    /// 未知/已移除的 TLAS 实例槽位。
    UnknownInstanceSlot(u32),
    /// 策略拒绝 refit 入队(仅 Deformable 可入队;Static 零更新、FullRebuild 走重建)。
    PolicyRejectsRefit(BlasId),
    /// 顶点数与 BLAS 构建时不符(拓扑变化必须走 rebuild)。
    VertexCountMismatch { expected: usize, got: usize },
}

impl fmt::Display for AsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            AsError::UnknownBlas(id) => write!(f, "未知 BLAS 句柄 {}", id.0),
            AsError::UnknownInstanceSlot(slot) => write!(f, "未知实例槽位 {slot}"),
            AsError::PolicyRejectsRefit(id) => write!(f, "BLAS {} 策略拒绝 refit 入队", id.0),
            AsError::VertexCountMismatch { expected, got } => {
                write!(f, "顶点数失配:期望 {expected},实得 {got}")
            }
        }
    }
}

impl std::error::Error for AsError {}

impl From<RefitError> for AsError {
    fn from(e: RefitError) -> AsError {
        match e {
            RefitError::VertexCountMismatch { expected, got } => {
                AsError::VertexCountMismatch { expected, got }
            }
        }
    }
}

/// 直接 refit 的结果(策略决策树答案,非错误)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RefitOutcome {
    /// Static 策略:不 refit(零成本透传,BVH 不变)。
    StaticSkipped,
    /// 已 refit;携带膨胀比报告(`needs_rebuild` 判据在内)。
    Refitted(RefitReport),
    /// FullRebuild 策略:拒绝 refit,调用方应走 [`BlasCache::rebuild`]。
    PolicyRequiresRebuild,
}

// ---------------------------------------------------------------------------
// BlasCache
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct BlasEntry {
    key: BlasKey,
    bvh: TriBvh,
    policy: DynamicPolicy,
}

#[derive(Debug)]
struct QueuedRefit {
    id: BlasId,
    positions: Vec<[f32; 3]>,
    budget_frames: u32,
}

/// BLAS 构建缓存(网格哈希复用 + 动态 refit 分级 + 显式 evict/compaction)。
///
/// 槽位模型:`BlasId` = 槽位下标,evict 释放槽位、后续构建复用(句柄在存活期内
/// 稳定;evict 后旧句柄失效,`blas()` 产 `None`,TLAS 遍历确定性跳过)。
#[derive(Debug, Default)]
pub struct BlasCache {
    entries: Vec<Option<BlasEntry>>,
    free_slots: Vec<u32>,
    by_key: HashMap<BlasKey, BlasId>,
    refit_queue: Vec<QueuedRefit>,
    stats: AsStats,
    /// 帧计数(process_refit_queue 每调用一次 +1;观测/调试语义,不参与决策)。
    frame: u64,
    /// 摊销窗口:每帧处理份额(窗口开启时定型,窗口内恒定;0 = 无活动窗口)。
    refit_window_share: usize,
    /// 摊销窗口剩余帧数(0 = 无活动窗口;耗尽或队列清空即关窗)。
    refit_window_frames_left: u32,
}

impl BlasCache {
    /// 空缓存。
    pub fn new() -> Self {
        Self::default()
    }

    /// 网格内容哈希命中即复用(不重建);未命中构建新 BLAS 并入缓存。
    /// 同 key 重复调用的策略以首建为准。
    pub fn get_or_build(
        &mut self,
        positions: &[[f32; 3]],
        indices: &[[u32; 3]],
        policy: DynamicPolicy,
    ) -> BlasId {
        let key = BlasKey::from_mesh(positions, indices);
        if let Some(&id) = self.by_key.get(&key) {
            return id;
        }
        let entry = BlasEntry {
            key,
            bvh: TriBvh::build(positions, indices),
            policy,
        };
        let id = if let Some(slot) = self.free_slots.pop() {
            self.entries[slot as usize] = Some(entry);
            BlasId(slot)
        } else {
            self.entries.push(Some(entry));
            BlasId(self.entries.len() as u32 - 1)
        };
        self.by_key.insert(key, id);
        self.stats.blas_builds += 1;
        id
    }

    /// 存活 BLAS 数(evict 后收缩)。
    pub fn len(&self) -> usize {
        self.by_key.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.by_key.is_empty()
    }

    /// 统计计数器快照(evidence 埋点)。
    pub fn stats(&self) -> AsStats {
        self.stats
    }

    /// 取 BLAS 几何;未知/已 evict 产 `None`。
    pub fn blas(&self, id: BlasId) -> Option<&TriBvh> {
        self.entries
            .get(id.0 as usize)
            .and_then(Option::as_ref)
            .map(|e| &e.bvh)
    }

    /// 取 BLAS 策略;未知/已 evict 产 `None`。
    pub fn policy(&self, id: BlasId) -> Option<DynamicPolicy> {
        self.entries
            .get(id.0 as usize)
            .and_then(Option::as_ref)
            .map(|e| e.policy)
    }

    /// 当前 refit 队列长度(观测用)。
    pub fn refit_queue_len(&self) -> usize {
        self.refit_queue.len()
    }

    /// 直接 refit(同步,绕过队列;策略决策树见 [`RefitOutcome`])。
    ///
    /// # Errors
    /// 未知句柄产 `UnknownBlas`;顶点数失配产 `VertexCountMismatch`(应改走 rebuild)。
    pub fn refit(
        &mut self,
        id: BlasId,
        new_positions: &[[f32; 3]],
    ) -> Result<RefitOutcome, AsError> {
        let entry = self
            .entries
            .get_mut(id.0 as usize)
            .and_then(Option::as_mut)
            .ok_or(AsError::UnknownBlas(id))?;
        match entry.policy {
            DynamicPolicy::Static => Ok(RefitOutcome::StaticSkipped),
            DynamicPolicy::FullRebuild => Ok(RefitOutcome::PolicyRequiresRebuild),
            DynamicPolicy::Deformable { .. } => {
                let report = entry.bvh.refit(new_positions)?;
                self.stats.refits += 1;
                Ok(RefitOutcome::Refitted(report))
            }
        }
    }

    /// 脏 BLAS 入队(仅 Deformable;同一句柄重复入队替换为最新位置,保持单条目)。
    ///
    /// # Errors
    /// 未知句柄产 `UnknownBlas`;Static/FullRebuild 产 `PolicyRejectsRefit`;
    /// 顶点数失配产 `VertexCountMismatch`。
    pub fn queue_refit(&mut self, id: BlasId, positions: Vec<[f32; 3]>) -> Result<(), AsError> {
        let entry = self
            .entries
            .get(id.0 as usize)
            .and_then(Option::as_ref)
            .ok_or(AsError::UnknownBlas(id))?;
        let DynamicPolicy::Deformable {
            refit_budget_frames,
        } = entry.policy
        else {
            return Err(AsError::PolicyRejectsRefit(id));
        };
        if positions.len() != entry.bvh.vertex_count() {
            return Err(AsError::VertexCountMismatch {
                expected: entry.bvh.vertex_count(),
                got: positions.len(),
            });
        }
        if let Some(q) = self.refit_queue.iter_mut().find(|q| q.id == id) {
            q.positions = positions;
        } else {
            self.refit_queue.push(QueuedRefit {
                id,
                positions,
                budget_frames: refit_budget_frames,
            });
        }
        Ok(())
    }

    /// 每帧 refit 队列轮转(摊销窗口语义):窗口开启时定型每帧份额
    /// `ceil(队长 / 最紧预算)` 并冻结,窗口内恒定——保证窗口内全部脏 BLAS 在
    /// ≤ 预算帧内完成(FIFO 轮转);窗口随队列清空或预算耗尽关闭,下帧按需重开。
    /// 返回本帧完成的 `(BlasId, RefitReport)`(含 `needs_rebuild` 判据,调用方决策
    /// 是否转 rebuild);已 evict/失配条目确定性丢弃。
    pub fn process_refit_queue(&mut self) -> Vec<(BlasId, RefitReport)> {
        self.frame += 1;
        if self.refit_queue.is_empty() {
            self.refit_window_share = 0;
            self.refit_window_frames_left = 0;
            return Vec::new();
        }
        if self.refit_window_frames_left == 0 {
            let min_budget = self
                .refit_queue
                .iter()
                .map(|q| q.budget_frames.max(1))
                .min()
                .unwrap_or(1);
            self.refit_window_share = self.refit_queue.len().div_ceil(min_budget as usize);
            self.refit_window_frames_left = min_budget;
        }
        let k = self.refit_window_share.min(self.refit_queue.len());
        self.refit_window_frames_left -= 1;
        let batch: Vec<QueuedRefit> = self.refit_queue.drain(..k).collect();
        if self.refit_queue.is_empty() || self.refit_window_frames_left == 0 {
            self.refit_window_share = 0;
            self.refit_window_frames_left = 0;
        }
        let mut done = Vec::with_capacity(batch.len());
        for q in batch {
            if let Some(entry) = self
                .entries
                .get_mut(q.id.0 as usize)
                .and_then(Option::as_mut)
                && let Ok(report) = entry.bvh.refit(&q.positions)
            {
                self.stats.refits += 1;
                done.push((q.id, report));
            }
        }
        done
    }

    /// 全量重建(拓扑变化:顶点数可不同;`FullRebuild` 策略的唯一更新路径,其余策略
    /// 遇 `needs_rebuild` 亦走此)。句柄/策略不变,缓存键随内容更新。
    ///
    /// # Errors
    /// 未知句柄产 `UnknownBlas`。
    pub fn rebuild(
        &mut self,
        id: BlasId,
        positions: &[[f32; 3]],
        indices: &[[u32; 3]],
    ) -> Result<(), AsError> {
        let entry = self
            .entries
            .get_mut(id.0 as usize)
            .and_then(Option::as_mut)
            .ok_or(AsError::UnknownBlas(id))?;
        let new_key = BlasKey::from_mesh(positions, indices);
        if new_key != entry.key {
            self.by_key.remove(&entry.key);
            self.by_key.insert(new_key, id);
            entry.key = new_key;
        }
        entry.bvh = TriBvh::build(positions, indices);
        self.refit_queue.retain(|q| q.id != id);
        self.stats.blas_builds += 1;
        Ok(())
    }

    /// 显式驱逐(compaction 留口):移除缓存项、释放槽位(后续构建复用)、
    /// 丢弃其队列条目。句柄自此失效(TLAS 遍历确定性跳过)。
    pub fn evict(&mut self, id: BlasId) -> bool {
        let Some(slot) = self.entries.get_mut(id.0 as usize) else {
            return false;
        };
        let Some(entry) = slot.take() else {
            return false;
        };
        self.by_key.remove(&entry.key);
        self.refit_queue.retain(|q| q.id != id);
        self.free_slots.push(id.0);
        true
    }

    /// TLAS 重建计数 +1(TlasBuilder 调用)。
    pub(crate) fn note_tlas_rebuild(&mut self) {
        self.stats.tlas_rebuilds += 1;
    }
}

impl BlasSet for BlasCache {
    fn blas(&self, id: u32) -> Option<&TriBvh> {
        self.entries
            .get(id as usize)
            .and_then(Option::as_ref)
            .map(|e| &e.bvh)
    }
}

// ---------------------------------------------------------------------------
// M95 RT 消费面(RXS-0352 L1/L3/L4):BLAS 拼装输入由 selection 输出直接派生
// ---------------------------------------------------------------------------

use crate::geometry::cull::VisibleCluster;
use crate::geometry::visible_cluster_set::{
    Consumer, ProvenanceError, RtFeed, VisibleClusterSet,
};

/// RT 腿消费锚(G9.3 M95):当帧 BLAS 拼装的输入数组 = [`RtFeed`]
/// ([`VisibleClusterSet::feed_rt`] 产物)**直接派生**——本函数是 RT 消费方
/// 进入 AS 管理层前的**结构性断言**:`feed.source` 必须与权威
/// `VisibleClusterSet` 的 provenance digest **精确一致**,随后透传 feed 的
/// (instance, cluster) 输入切片(零拷贝 = 直接派生字面,层内不再重算可见性)。
///
/// 旁路双世界否决(L4,硬门 R-G9-8):光栅/RT 各自独立再算可见性的 variant
/// ⇒ 实例 serial 必异 ⇒ digest 必异 ⇒ fail-closed `Err`(即使内容全等、
/// 出图相似也判 RED——单源真相是**结构**判据)。
///
/// # Errors
/// digest 失配产 `ProvenanceError::Mismatch { consumer: Consumer::Rt, .. }`。
pub fn rt_blas_input_from_feed<'a>(
    set: &VisibleClusterSet,
    feed: &'a RtFeed,
) -> Result<&'a [VisibleCluster], ProvenanceError> {
    if feed.source != set.provenance_digest {
        return Err(ProvenanceError::Mismatch {
            consumer: Consumer::Rt,
            expected: set.provenance_digest,
            got: feed.source,
        });
    }
    Ok(&feed.blas_input)
}

// ---------------------------------------------------------------------------
// TlasBuilder:实例列表管理 + 每帧快速重建(增量标脏)
// ---------------------------------------------------------------------------

/// TLAS 实例描述(manager 层;`blas` 为缓存句柄)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TlasInstance {
    /// BLAS 缓存句柄。
    pub blas: BlasId,
    /// 对象空间 → 世界空间仿射变换。
    pub transform: Transform3x4,
    /// 光线掩码(Vulkan mask 语义;0 = 对光线不可见)。
    pub mask: u8,
    /// 实例旗标(保留字段,device 腿对齐留口;host 遍历不消费)。
    pub flags: u32,
}

impl TlasInstance {
    /// 构造实例(默认 mask = 0xFF 全可见,flags = 0)。
    pub fn new(blas: BlasId, transform: Transform3x4) -> Self {
        Self {
            blas,
            transform,
            mask: 0xFF,
            flags: 0,
        }
    }

    /// 设光线掩码。
    pub fn with_mask(mut self, mask: u8) -> Self {
        self.mask = mask;
        self
    }

    /// 设实例旗标(保留)。
    pub fn with_flags(mut self, flags: u32) -> Self {
        self.flags = flags;
        self
    }
}

#[derive(Debug, Clone, Copy)]
struct SlotState {
    blas: BlasId,
    transform: Transform3x4,
    mask: u8,
    flags: u32,
}

/// TLAS 构建器:实例列表(BlasId + 3×4 变换 + mask/flags)的增删改 + 每帧快速
/// 重建。重建本身廉价(报告4:实例 <10k 亚毫秒),故增量语义只做**标脏**——
/// `rebuild_if_dirty` 在干净帧产 `None`(零成本跳过),脏帧全量重建。
#[derive(Debug, Default)]
pub struct TlasBuilder {
    instances: Vec<Option<SlotState>>,
    free_slots: Vec<u32>,
    dirty: bool,
    built: bool,
}

impl TlasBuilder {
    /// 空构建器。
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加实例,返回槽位(移除过的槽位复用)。标脏。
    pub fn add_instance(&mut self, inst: TlasInstance) -> u32 {
        let state = SlotState {
            blas: inst.blas,
            transform: inst.transform,
            mask: inst.mask,
            flags: inst.flags,
        };
        self.dirty = true;
        if let Some(slot) = self.free_slots.pop() {
            self.instances[slot as usize] = Some(state);
            slot
        } else {
            self.instances.push(Some(state));
            self.instances.len() as u32 - 1
        }
    }

    /// 移除实例(槽位释放)。标脏。
    ///
    /// # Errors
    /// 未知槽位产 `UnknownInstanceSlot`。
    pub fn remove_instance(&mut self, slot: u32) -> Result<(), AsError> {
        let cell = self
            .instances
            .get_mut(slot as usize)
            .ok_or(AsError::UnknownInstanceSlot(slot))?;
        if cell.take().is_none() {
            return Err(AsError::UnknownInstanceSlot(slot));
        }
        self.free_slots.push(slot);
        self.dirty = true;
        Ok(())
    }

    /// 更新实例变换(增量标脏的核心理由:变换更新只标脏,不即刻重建)。
    ///
    /// # Errors
    /// 未知槽位产 `UnknownInstanceSlot`。
    pub fn update_transform(&mut self, slot: u32, transform: Transform3x4) -> Result<(), AsError> {
        let state = self
            .instances
            .get_mut(slot as usize)
            .and_then(Option::as_mut)
            .ok_or(AsError::UnknownInstanceSlot(slot))?;
        state.transform = transform;
        self.dirty = true;
        Ok(())
    }

    /// 更新实例光线掩码。标脏。
    ///
    /// # Errors
    /// 未知槽位产 `UnknownInstanceSlot`。
    pub fn set_mask(&mut self, slot: u32, mask: u8) -> Result<(), AsError> {
        let state = self
            .instances
            .get_mut(slot as usize)
            .and_then(Option::as_mut)
            .ok_or(AsError::UnknownInstanceSlot(slot))?;
        state.mask = mask;
        self.dirty = true;
        Ok(())
    }

    /// 是否有未重建的变更。
    pub fn is_dirty(&self) -> bool {
        self.dirty || !self.built
    }

    /// 存活实例数。
    pub fn instance_count(&self) -> usize {
        self.instances.iter().flatten().count()
    }

    /// 每帧快速重建:全量重建实例级 BVH(`AsStats::tlas_rebuilds` +1),清脏。
    /// 引用已 evict BLAS 的实例在 TLAS 内确定性禁用(见 [`Tlas::build`])。
    pub fn rebuild(&mut self, cache: &mut BlasCache) -> Tlas {
        let descs: Vec<BvhInstanceDesc> = self
            .instances
            .iter()
            .flatten()
            .map(|s| BvhInstanceDesc {
                blas: s.blas.0,
                transform: s.transform,
                mask: s.mask,
                flags: s.flags,
            })
            .collect();
        let tlas = Tlas::build(&descs, cache);
        cache.note_tlas_rebuild();
        self.dirty = false;
        self.built = true;
        tlas
    }

    /// 脏则重建(产 `Some`),干净帧零成本跳过(产 `None`)。
    pub fn rebuild_if_dirty(&mut self, cache: &mut BlasCache) -> Option<Tlas> {
        if self.is_dirty() {
            Some(self.rebuild(cache))
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// 单测
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rt::bvh::{Ray, Vec3};

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1e-5
    }

    /// 单位四边形(z=0,x,y ∈ [0,1];两三角形)。
    fn unit_quad() -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
        (
            vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            vec![[0, 1, 2], [0, 2, 3]],
        )
    }

    /// 平移 quad 的顶点(纯平移保持叶面积,膨胀比 1.0)。
    fn translated(pos: &[[f32; 3]], d: [f32; 3]) -> Vec<[f32; 3]> {
        pos.iter()
            .map(|p| [p[0] + d[0], p[1] + d[1], p[2] + d[2]])
            .collect()
    }

    #[test]
    fn blas_key_content_hash_semantics() {
        let (pos, idx) = unit_quad();
        let k1 = BlasKey::from_mesh(&pos, &idx);
        let k2 = BlasKey::from_mesh(&pos, &idx);
        assert_eq!(k1, k2, "同内容同哈希(确定性)");
        // 任一位置分量变化 → 键不同。
        let mut pos2 = pos.clone();
        pos2[0][0] = 2.0;
        assert_ne!(k1, BlasKey::from_mesh(&pos2, &idx));
        // 索引变化 → 键不同。
        let idx2 = vec![[0, 1, 2], [0, 3, 2]];
        assert_ne!(k1, BlasKey::from_mesh(&pos, &idx2));
    }

    #[test]
    fn blas_cache_hit_avoids_rebuild() {
        let (pos, idx) = unit_quad();
        let mut cache = BlasCache::new();
        let a = cache.get_or_build(&pos, &idx, DynamicPolicy::Static);
        let b = cache.get_or_build(
            &pos,
            &idx,
            DynamicPolicy::Deformable {
                refit_budget_frames: 1,
            },
        );
        // 同内容命中:同一 BlasId,不重建,策略以首建为准。
        assert_eq!(a, b);
        assert_eq!(cache.stats().blas_builds, 1);
        assert_eq!(cache.policy(a), Some(DynamicPolicy::Static));
        // 不同内容:新构建。
        let pos2 = translated(&pos, [0.0, 0.0, 1.0]);
        let c = cache.get_or_build(&pos2, &idx, DynamicPolicy::Static);
        assert_ne!(a, c);
        assert_eq!(cache.stats().blas_builds, 2);
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn static_policy_refit_is_noop() {
        let (pos, idx) = unit_quad();
        let mut cache = BlasCache::new();
        let id = cache.get_or_build(&pos, &idx, DynamicPolicy::Static);
        let before = cache.blas(id).and_then(|b| b.bounds());
        let moved = translated(&pos, [10.0, 0.0, 0.0]);
        let outcome = cache.refit(id, &moved).expect("Static refit 非错误");
        assert_eq!(outcome, RefitOutcome::StaticSkipped);
        assert_eq!(cache.stats().refits, 0, "Static 零成本透传,不计 refit");
        assert_eq!(cache.blas(id).and_then(|b| b.bounds()), before);
    }

    #[test]
    fn deformable_refit_updates_bounds() {
        let (pos, idx) = unit_quad();
        let mut cache = BlasCache::new();
        let id = cache.get_or_build(
            &pos,
            &idx,
            DynamicPolicy::Deformable {
                refit_budget_frames: 1,
            },
        );
        let moved = translated(&pos, [10.0, 0.0, 0.0]);
        let outcome = cache.refit(id, &moved).expect("Deformable refit 合法");
        let RefitOutcome::Refitted(report) = outcome else {
            panic!("Deformable 应执行 refit")
        };
        // 纯平移:叶面积不变,膨胀比 1.0,不建议重建。
        assert!(approx(report.max_inflation, 1.0));
        assert!(!report.needs_rebuild());
        assert_eq!(cache.stats().refits, 1);
        // 几何跟随新位置。
        let blas = cache.blas(id).expect("存活");
        let hit_ray = Ray {
            origin: Vec3::new(10.25, 0.25, 1.0),
            dir: Vec3::new(0.0, 0.0, -1.0),
        };
        assert!(blas.intersect(&hit_ray).is_some());
        let old_ray = Ray {
            origin: Vec3::new(0.25, 0.25, 1.0),
            dir: Vec3::new(0.0, 0.0, -1.0),
        };
        assert_eq!(blas.intersect(&old_ray), None);
    }

    #[test]
    fn full_rebuild_policy_rejects_refit() {
        let (pos, idx) = unit_quad();
        let mut cache = BlasCache::new();
        let id = cache.get_or_build(&pos, &idx, DynamicPolicy::FullRebuild);
        let moved = translated(&pos, [1.0, 0.0, 0.0]);
        let outcome = cache.refit(id, &moved).expect("策略答案非错误");
        assert_eq!(outcome, RefitOutcome::PolicyRequiresRebuild);
        assert_eq!(cache.stats().refits, 0);
        // 入队同样被拒(确定性拒绝)。
        assert_eq!(
            cache.queue_refit(id, moved),
            Err(AsError::PolicyRejectsRefit(id))
        );
    }

    #[test]
    fn rebuild_replaces_geometry_and_key() {
        let (pos, idx) = unit_quad();
        let mut cache = BlasCache::new();
        let id = cache.get_or_build(&pos, &idx, DynamicPolicy::FullRebuild);
        assert_eq!(cache.stats().blas_builds, 1);
        // 拓扑变化:三角形数量不同(单三角替换 quad)。
        let pos2 = vec![[5.0, 0.0, 0.0], [6.0, 0.0, 0.0], [5.0, 1.0, 0.0]];
        let idx2 = vec![[0, 1, 2]];
        cache.rebuild(id, &pos2, &idx2).expect("rebuild 合法");
        assert_eq!(cache.stats().blas_builds, 2, "rebuild 计入全量构建");
        // 句柄不变,几何与键更新:get_or_build 新内容命中同一句柄,不再新建。
        let again = cache.get_or_build(&pos2, &idx2, DynamicPolicy::Static);
        assert_eq!(again, id);
        assert_eq!(cache.stats().blas_builds, 2);
        assert_eq!(cache.len(), 1);
        // 旧内容不再命中该句柄(键已换)——重新构建为新句柄。
        let old = cache.get_or_build(&pos, &idx, DynamicPolicy::Static);
        assert_ne!(old, id);
        assert_eq!(cache.stats().blas_builds, 3);
    }

    #[test]
    fn refit_queue_round_robin_amortizes() {
        let (pos, idx) = unit_quad();
        let mut cache = BlasCache::new();
        // 4 个 Deformable{budget:2}:每帧处理队列一半,两帧摊完。
        let ids: Vec<BlasId> = (0..4)
            .map(|i| {
                let p = translated(&pos, [i as f32, 0.0, 0.0]);
                cache.get_or_build(
                    &p,
                    &idx,
                    DynamicPolicy::Deformable {
                        refit_budget_frames: 2,
                    },
                )
            })
            .collect();
        for (i, &id) in ids.iter().enumerate() {
            let p = translated(&pos, [i as f32, 100.0, 0.0]);
            cache.queue_refit(id, p).expect("Deformable 入队合法");
        }
        assert_eq!(cache.refit_queue_len(), 4);
        let frame1 = cache.process_refit_queue();
        assert_eq!(frame1.len(), 2, "帧1 摊销一半");
        assert_eq!(frame1[0].0, ids[0]);
        assert_eq!(frame1[1].0, ids[1]);
        assert_eq!(cache.refit_queue_len(), 2);
        let frame2 = cache.process_refit_queue();
        assert_eq!(frame2.len(), 2, "帧2 摊完余量");
        assert_eq!(frame2[0].0, ids[2]);
        assert_eq!(cache.refit_queue_len(), 0);
        assert_eq!(cache.stats().refits, 4);
        // 空队列:平凡帧。
        assert!(cache.process_refit_queue().is_empty());
        assert_eq!(cache.stats().refits, 4);
    }

    #[test]
    fn queue_refit_replaces_positions_and_validates() {
        let (pos, idx) = unit_quad();
        let mut cache = BlasCache::new();
        let id = cache.get_or_build(
            &pos,
            &idx,
            DynamicPolicy::Deformable {
                refit_budget_frames: 1,
            },
        );
        // 重复入队:替换为最新位置,保持单条目。
        cache
            .queue_refit(id, translated(&pos, [0.0, 1.0, 0.0]))
            .expect("合法");
        cache
            .queue_refit(id, translated(&pos, [0.0, 2.0, 0.0]))
            .expect("重复入队替换");
        assert_eq!(cache.refit_queue_len(), 1);
        let done = cache.process_refit_queue();
        assert_eq!(done.len(), 1);
        // 生效的是最新位置(y+2):y+1 处不命中,y+2.5 处命中。
        let blas = cache.blas(id).expect("存活");
        let stale_ray = Ray {
            origin: Vec3::new(0.25, 1.25, 1.0),
            dir: Vec3::new(0.0, 0.0, -1.0),
        };
        assert_eq!(blas.intersect(&stale_ray), None);
        let fresh_ray = Ray {
            origin: Vec3::new(0.25, 2.25, 1.0),
            dir: Vec3::new(0.0, 0.0, -1.0),
        };
        assert!(blas.intersect(&fresh_ray).is_some());
        // 顶点数失配:入队即拒(拓扑变化必须 rebuild)。
        let bad = vec![[0.0, 0.0, 0.0]; 3];
        assert_eq!(
            cache.queue_refit(id, bad),
            Err(AsError::VertexCountMismatch {
                expected: 4,
                got: 3
            })
        );
        // 未知句柄:拒绝。
        assert_eq!(
            cache.queue_refit(BlasId(99), vec![]),
            Err(AsError::UnknownBlas(BlasId(99)))
        );
    }

    #[test]
    fn tlas_builder_dirty_flag_and_stats() {
        let (pos, idx) = unit_quad();
        let mut cache = BlasCache::new();
        let blas = cache.get_or_build(&pos, &idx, DynamicPolicy::Static);
        let mut builder = TlasBuilder::new();
        assert!(builder.is_dirty(), "未构建过视为脏");
        let s0 = builder.add_instance(TlasInstance::new(blas, Transform3x4::IDENTITY));
        builder.rebuild(&mut cache);
        assert_eq!(cache.stats().tlas_rebuilds, 1);
        assert!(!builder.is_dirty());
        // 干净帧:零成本跳过。
        assert!(builder.rebuild_if_dirty(&mut cache).is_none());
        assert_eq!(cache.stats().tlas_rebuilds, 1);
        // 变换更新标脏 → 下一帧重建。
        builder
            .update_transform(s0, Transform3x4::from_translation([3.0, 0.0, 0.0]))
            .expect("槽位合法");
        assert!(builder.rebuild_if_dirty(&mut cache).is_some());
        assert_eq!(cache.stats().tlas_rebuilds, 2);
        // 移除实例标脏;移除未知槽位确定性拒绝。
        builder.remove_instance(s0).expect("槽位合法");
        assert!(builder.rebuild_if_dirty(&mut cache).is_some());
        assert_eq!(cache.stats().tlas_rebuilds, 3);
        assert_eq!(builder.instance_count(), 0);
        assert_eq!(
            builder.remove_instance(s0),
            Err(AsError::UnknownInstanceSlot(s0))
        );
    }

    #[test]
    fn tlas_builder_end_to_end_traversal() {
        let (pos, idx) = unit_quad();
        let mut cache = BlasCache::new();
        let blas = cache.get_or_build(&pos, &idx, DynamicPolicy::Static);
        let mut builder = TlasBuilder::new();
        builder.add_instance(TlasInstance::new(
            blas,
            Transform3x4::from_translation([5.0, 0.0, 0.0]),
        ));
        let tlas = builder.rebuild(&mut cache);
        // 平移实例命中(t 参数与相对位置一致)。
        let hit_ray = Ray {
            origin: Vec3::new(5.25, 0.25, 1.0),
            dir: Vec3::new(0.0, 0.0, -1.0),
        };
        let hit = tlas.intersect(&cache, &hit_ray).expect("应命中");
        assert!(approx(hit.t, 1.0));
        assert_eq!(hit.instance, 0);
        let miss_ray = Ray {
            origin: Vec3::new(0.25, 0.25, 1.0),
            dir: Vec3::new(0.0, 0.0, -1.0),
        };
        assert_eq!(tlas.intersect(&cache, &miss_ray), None);
        assert!(tlas.any_hit(&cache, &hit_ray, 2.0));
    }

    #[test]
    fn evict_frees_slot_and_hides_blas() {
        let (pos, idx) = unit_quad();
        let mut cache = BlasCache::new();
        let id = cache.get_or_build(&pos, &idx, DynamicPolicy::Static);
        let mut builder = TlasBuilder::new();
        builder.add_instance(TlasInstance::new(id, Transform3x4::IDENTITY));
        let tlas = builder.rebuild(&mut cache);
        // 驱逐后:句柄失效,缓存收缩;既有 TLAS 遍历确定性跳过(evict 后查询)。
        assert!(cache.evict(id));
        assert!(!cache.evict(id), "重复驱逐幂等 false");
        assert!(cache.is_empty());
        assert!(cache.blas(id).is_none());
        let ray = Ray {
            origin: Vec3::new(0.25, 0.25, 1.0),
            dir: Vec3::new(0.0, 0.0, -1.0),
        };
        assert_eq!(tlas.intersect(&cache, &ray), None, "失效 BLAS 永不命中");
        // 槽位复用:同内容重建占用释放的槽位,计入新建。
        let reused = cache.get_or_build(&pos, &idx, DynamicPolicy::Static);
        assert_eq!(reused, id, "释放槽位被复用");
        assert_eq!(cache.stats().blas_builds, 2);
        assert_eq!(cache.len(), 1);
        // 复用后 TLAS 恢复命中(同一句柄再次有效)。
        assert!(tlas.intersect(&cache, &ray).is_some());
    }

    // -----------------------------------------------------------------------
    // G9.3 M95(RXS-0352):RT 腿 as_manager 消费锚(结构断言 + RED)
    // -----------------------------------------------------------------------

    use crate::geometry::visible_cluster_set::{
        VisibleClusterEntry, compute_provenance_digest,
    };

    /// 手构两元素可见集(与 visbuffer 单测同型;cluster 0 静态 + cluster 1 蒙皮)。
    fn two_entry_set(serial: u64) -> crate::geometry::visible_cluster_set::VisibleClusterSet {
        let mut set = crate::geometry::visible_cluster_set::VisibleClusterSet {
            frame_serial: serial,
            entries: vec![
                VisibleClusterEntry {
                    cluster: 0,
                    instance: 0,
                    lod_level: 0,
                    skin_version: 0,
                    page_id: 0,
                    visible: true,
                },
                VisibleClusterEntry {
                    cluster: 1,
                    instance: 0,
                    lod_level: 0,
                    skin_version: 1,
                    page_id: 0,
                    visible: true,
                },
            ],
            residency: vec![],
            fallback: vec![],
            provenance_digest: [0; 32],
        };
        set.provenance_digest = compute_provenance_digest(&set);
        set
    }

    //@ spec: RXS-0352
    #[test]
    fn rt_feed_consumed_by_as_manager_anchor() {
        let set = two_entry_set(7);
        let feed = set.feed_rt();
        // 正例:provenance 一致 ⇒ 消费放行,输入切片 = feed 载荷直接派生
        // (零拷贝同址 = 直接派生字面;内容 = 可见元素 (instance, cluster) 对)。
        let input = rt_blas_input_from_feed(&set, &feed).expect("权威 feed 必须放行");
        assert_eq!(
            input,
            &[
                VisibleCluster {
                    instance: 0,
                    cluster: 0
                },
                VisibleCluster {
                    instance: 0,
                    cluster: 1
                },
            ]
        );
        assert!(
            std::ptr::eq(input.as_ptr(), feed.blas_input.as_ptr()),
            "直接派生 = 零拷贝透传 feed 载荷(禁独立再算可见性)"
        );
        // 双跑逐位一致(确定性)。
        let set2 = two_entry_set(7);
        let feed2 = set2.feed_rt();
        assert_eq!(
            rt_blas_input_from_feed(&set2, &feed2).expect("run B"),
            input
        );
    }

    //@ spec: RXS-0352
    #[test]
    fn rt_feed_bypass_recompute_red_at_consumption() {
        // RED 臂(消费锚面):RT 腿旁路独立再算可见性 ⇒ serial 异 ⇒ digest 必异,
        // 即使内容逐元素全等也 fail-closed(双世界结构否决,L4/R-G9-8)。
        let authoritative = two_entry_set(7);
        let bypass = two_entry_set(8);
        assert_eq!(authoritative.entries, bypass.entries, "内容全等");
        assert_ne!(
            authoritative.provenance_digest, bypass.provenance_digest,
            "serial 混入 ⇒ 旁路 digest 必异"
        );
        let bypass_feed = bypass.feed_rt();
        let err = rt_blas_input_from_feed(&authoritative, &bypass_feed)
            .expect_err("旁路 feed 必须在消费锚判 RED");
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
        // 篡改 source 字节的 feed(伪装同源)同样判 RED。
        let mut forged = authoritative.feed_rt();
        forged.source[0] ^= 0xFF;
        assert!(rt_blas_input_from_feed(&authoritative, &forged).is_err());
    }
}
