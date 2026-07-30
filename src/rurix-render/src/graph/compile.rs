//! 编译四趟 + 编译期校验(报告5 §2/§5;RFC-0016 章 A)。
//!
//! 趟序(结构面校验在剔除前完成,与剔除开关无关):
//! 1. **剔除**:从「写 imported 资源 / Present 消费」反向可达,剔除无贡献 pass 与资源
//!    (写满足更晚需求的全覆盖语义;可经 [`CompileOptions::enable_culling`] 关闭);
//! 2. **生命周期**:逐 transient 资源求 [`LifeInterval`](首用..末用;imported 不参与);
//! 3. **屏障推导**:逐资源 [`AccessTracker`] 沿线性序推进,pass 边界发射 EB 三轴屏障
//!    (规则见 [`crate::graph::sync`]),产出每 pass 前屏障批;
//! 4. **车道划分**:AsyncCompute pass 沿依赖找最后图形生产者(signal)与首个图形消费者
//!    (wait)插 [`FencePair`],timeline 值单调;异步关闭时整体回落图形车道
//!    ([`CompileOptions::enable_async`])。
//!
//! 编译期校验(全部确定性拒,逐类注入单测见本文件 tests):
//! ① 读未写(transient 消费读无先前写;imported 外部初始化豁免)
//! ② 同 pass 写写/读写冲突(冲突未声明序)
//! ③ 越期句柄(资源 id 越界;句柄与生命周期绑定)
//! ④ 重复 Present(整图至多一处 handoff)
//! ⑤ 异步依赖环(帧内版本倒置:输入的最后图形生产者晚于产出的首个图形消费者,
//!    fence 弧无解,保守拒——声明面不表达双缓冲语义)

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::graph::graph::{CmdRecorder, CommandLog, PassExecute, PassNode, RenderGraph};
use crate::graph::resources::ResourceNode;
use crate::graph::sync::{AccessTracker, access_mask_of, image_layout_of, sync_stage_of};
use crate::graph::transient::TransientPool;
use crate::graph::types::{
    AccessKind, Barrier, FencePair, ImageLayout, LifeInterval, PassId, PoolSlot, QueueClass,
    ResAccess, ResourceId, ResourceKind,
};

// ---------------------------------------------------------------------------
// 编译开关
// ---------------------------------------------------------------------------

/// 编译开关(剔除与异步车道均可整体关闭——Godot「可整体禁用图」调试阀门同思想,
/// 报告5 §6)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompileOptions {
    /// 趟1 剔除开关(默认开)。
    pub enable_culling: bool,
    /// 趟4 异步车道开关(默认开;关闭 = AsyncCompute pass 整体回落图形车道,无 fence)。
    pub enable_async: bool,
}

impl Default for CompileOptions {
    fn default() -> CompileOptions {
        CompileOptions {
            enable_culling: true,
            enable_async: true,
        }
    }
}

// ---------------------------------------------------------------------------
// 编译期错误(确定性拒)
// ---------------------------------------------------------------------------

/// 图编译错误(编译期确定性拒;每类附注入单测)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    /// ① 读未写:transient 消费读无先前 pass 写入(imported 外部初始化豁免)。
    ReadBeforeWrite {
        /// 违例 pass 名。
        pass: String,
        /// 违例资源名。
        resource: String,
    },
    /// ② 同 pass 冲突:同资源多次声明(写写/读写冲突未声明序)。
    WriteWriteConflict {
        /// 违例 pass 名。
        pass: String,
        /// 违例资源名。
        resource: String,
    },
    /// ③ 越期句柄:资源 id 越界(句柄与生命周期绑定)。
    InvalidHandle {
        /// 违例 pass 名。
        pass: String,
        /// 越界句柄。
        resource: ResourceId,
    },
    /// ④ 重复 Present:整图至多一处 Present handoff。
    DuplicatePresent {
        /// 第二处 Present 所在 pass 名。
        pass: String,
    },
    /// ⑤ 异步依赖环/版本倒置:fence 弧无解,保守拒。
    AsyncDependencyCycle {
        /// 违例异步 pass 名。
        pass: String,
        /// 诊断详情。
        detail: String,
    },
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphError::ReadBeforeWrite { pass, resource } => write!(
                f,
                "read-before-write(读未写):pass `{pass}` 消费读 transient `{resource}`,但无先前 pass 写入(transient 须先写后读;跨帧资源应 import)"
            ),
            GraphError::WriteWriteConflict { pass, resource } => write!(
                f,
                "write conflict(冲突未声明序):pass `{pass}` 对资源 `{resource}` 多次声明访问(同 pass 写写/读写冲突;ShaderWrite 单条即读写合并语义)"
            ),
            GraphError::InvalidHandle { pass, resource } => write!(
                f,
                "invalid handle(越期句柄):pass `{pass}` 引用越界资源句柄 res#{}(句柄与生命周期绑定,越界/剔除后引用确定性拒)",
                resource.0
            ),
            GraphError::DuplicatePresent { pass } => write!(
                f,
                "duplicate present(重复 Present):pass `{pass}` 声明了第二处 Present handoff(整图至多一处)"
            ),
            GraphError::AsyncDependencyCycle { pass, detail } => write!(
                f,
                "async dependency cycle(异步依赖环/版本倒置):异步 pass `{pass}` fence 弧无解——{detail}"
            ),
        }
    }
}

impl std::error::Error for GraphError {}

// ---------------------------------------------------------------------------
// 编译产物
// ---------------------------------------------------------------------------

/// 编译产物 pass(幸存 pass;执行序 = 声明序保序子列)。
pub struct CompiledPass {
    id: PassId,
    name: String,
    queue: QueueClass,
    reads: Vec<ResAccess>,
    writes: Vec<ResAccess>,
    barriers_before: Vec<Barrier>,
    execute: Option<PassExecute>,
}

impl CompiledPass {
    /// pass 句柄(声明面原 id,剔除后仍锚定线性序)。
    pub fn id(&self) -> PassId {
        self.id
    }

    /// pass 诊断名。
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 有效车道(异步回落开启时,声明的 AsyncCompute 降为 Graphics)。
    pub fn queue(&self) -> QueueClass {
        self.queue
    }

    /// 声明读集。
    pub fn reads(&self) -> &[ResAccess] {
        &self.reads
    }

    /// 声明写集。
    pub fn writes(&self) -> &[ResAccess] {
        &self.writes
    }

    /// 本 pass 前屏障批(趟3 产物;与 [`CompiledGraph::barriers`] 一致)。
    pub fn barriers_before(&self) -> &[Barrier] {
        &self.barriers_before
    }
}

impl fmt::Debug for CompiledPass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompiledPass")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("queue", &self.queue)
            .field("reads", &self.reads)
            .field("writes", &self.writes)
            .field("barriers_before", &self.barriers_before)
            .finish_non_exhaustive()
    }
}

/// 编译产物资源(幸存资源;原 [`ResourceId`] 保留)。
#[derive(Debug, Clone)]
pub struct CompiledResource {
    id: ResourceId,
    name: String,
    kind: ResourceKind,
    imported: bool,
    lifetime: Option<LifeInterval>,
    slot: Option<PoolSlot>,
    last_writer: Option<PassId>,
}

impl CompiledResource {
    /// 资源句柄(声明面原 id)。
    pub fn id(&self) -> ResourceId {
        self.id
    }

    /// 资源诊断名。
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 资源类别与尺寸描述。
    pub fn kind(&self) -> ResourceKind {
        self.kind
    }

    /// true = 外部 import(图只管理状态转换;不入池、无生命周期)。
    pub fn imported(&self) -> bool {
        self.imported
    }

    /// transient 生命周期区间(imported → None)。
    pub fn lifetime(&self) -> Option<LifeInterval> {
        self.lifetime
    }

    /// transient 池槽位(imported → None)。
    pub fn slot(&self) -> Option<PoolSlot> {
        self.slot
    }

    /// 最后写入 pass(审计/dump;从未写入 → None)。
    pub fn last_writer(&self) -> Option<PassId> {
        self.last_writer
    }

    /// 保守字节尺寸(契约 [`ResourceKind::byte_size`])。
    pub fn byte_size(&self) -> u64 {
        self.kind.byte_size()
    }
}

/// 编译产物(下游唯一消费面:pass/资源/屏障批/fence 对/池审计 + 执行回放 + dump)。
#[derive(Debug)]
pub struct CompiledGraph {
    passes: Vec<CompiledPass>,
    resources: Vec<CompiledResource>,
    barriers: Vec<(PassId, Vec<Barrier>)>,
    fences: Vec<FencePair>,
    pool: TransientPool,
    culled_pass_count: usize,
    culled_resource_count: usize,
}

impl CompiledGraph {
    /// 幸存 pass(执行序)。
    pub fn passes(&self) -> &[CompiledPass] {
        &self.passes
    }

    /// 幸存资源。
    pub fn resources(&self) -> &[CompiledResource] {
        &self.resources
    }

    /// 按 id 查资源。
    #[must_use]
    pub fn resource(&self, id: ResourceId) -> Option<&CompiledResource> {
        self.resources.iter().find(|r| r.id == id)
    }

    /// 每 pass 前屏障批(趟3 产物;(PassId, 批) 按执行序)。
    pub fn barriers(&self) -> &[(PassId, Vec<Barrier>)] {
        &self.barriers
    }

    /// 异步车道 fence 对(趟4 产物;timeline 值单调)。
    pub fn fences(&self) -> &[FencePair] {
        &self.fences
    }

    /// transient 池审计。
    pub fn pool(&self) -> &TransientPool {
        &self.pool
    }

    /// 被剔除 pass 数。
    pub fn culled_pass_count(&self) -> usize {
        self.culled_pass_count
    }

    /// 未进编译产物的资源数(被剔除 / 从未被访问)。
    pub fn culled_resource_count(&self) -> usize {
        self.culled_resource_count
    }

    /// 依线性序回放各幸存 pass 的 execute 闭包,返回命令记录(记录桩;
    /// 被剔除 pass 的闭包已在编译期丢弃,不会执行)。
    pub fn execute(&mut self) -> CommandLog {
        let mut commands = Vec::new();
        for p in &mut self.passes {
            let mut rec = CmdRecorder::new(p.id);
            if let Some(f) = &mut p.execute {
                f(&mut rec);
            }
            commands.extend(rec.finish());
        }
        CommandLog { commands }
    }
}

// ---------------------------------------------------------------------------
// 编译本体
// ---------------------------------------------------------------------------

/// 编译本体(RenderGraph::compile 的实现)。
pub(crate) fn compile(
    graph: RenderGraph,
    options: CompileOptions,
) -> Result<CompiledGraph, GraphError> {
    let RenderGraph { resources, passes } = graph;
    validate(&resources, &passes)?;

    // ── 趟1 剔除(反向可达;根 = 写 imported / Present 消费)──
    let keep = if options.enable_culling {
        cull(&resources, &passes)
    } else {
        vec![true; passes.len()]
    };
    let culled_pass_count = keep.iter().filter(|&&k| !k).count();
    let survivors: Vec<PassNode> = passes
        .into_iter()
        .zip(keep)
        .filter_map(|(node, k)| k.then_some(node))
        .collect();

    // 幸存 pass 访问的资源集(未被访问的资源不进产物——无生命周期可言)。
    let mut used: BTreeSet<u32> = BTreeSet::new();
    for p in &survivors {
        for a in p.desc.reads.iter().chain(p.desc.writes.iter()) {
            used.insert(a.res.0);
        }
    }
    let culled_resource_count = resources.len() - used.len();

    // ── 趟2 生命周期(逐 transient;原 PassId 保序,区间比较即幸存序比较)──
    let mut lifetimes: BTreeMap<u32, LifeInterval> = BTreeMap::new();
    for p in &survivors {
        for a in p.desc.reads.iter().chain(p.desc.writes.iter()) {
            lifetimes
                .entry(a.res.0)
                .and_modify(|iv| {
                    iv.first_use = iv.first_use.min(p.id);
                    iv.last_use = iv.last_use.max(p.id);
                })
                .or_insert(LifeInterval {
                    first_use: p.id,
                    last_use: p.id,
                });
        }
    }

    // ── transient 池(趟2 区间 → 区间着色别名)──
    let mut pool_entries: Vec<(ResourceId, ResourceKind, LifeInterval)> = Vec::new();
    for &rid in &used {
        let node = &resources[rid as usize];
        if node.desc.imported {
            continue;
        }
        pool_entries.push((node.id, node.desc.kind, lifetimes[&rid]));
    }
    let pool = TransientPool::build(&pool_entries);
    let handoff = alias_predecessors(&pool, &pool_entries);

    // ── 趟3 屏障推导(逐资源 AccessTracker;有效车道 = 回落后)──
    let effective_queue = |q: QueueClass| {
        if options.enable_async {
            q
        } else {
            QueueClass::Graphics
        }
    };
    let mut trackers: BTreeMap<u32, AccessTracker> = BTreeMap::new();
    let mut batches: Vec<Vec<Barrier>> = Vec::with_capacity(survivors.len());
    for p in &survivors {
        let q = effective_queue(p.desc.queue);
        let mut batch: Vec<Barrier> = Vec::new();
        for a in p.desc.reads.iter().chain(p.desc.writes.iter()) {
            let node = &resources[a.res.0 as usize];
            let req_sync = sync_stage_of(a.access, q);
            let req_access = access_mask_of(a.access);
            // buffer 恒 Undefined layout(契约)。
            let req_layout = if node.is_texture() {
                image_layout_of(a.access)
            } else {
                ImageLayout::Undefined
            };
            // 别名交接:本资源首用且同槽有前任 → before 侧取前任末态。
            let ho = if trackers.contains_key(&a.res.0) {
                None
            } else {
                handoff
                    .get(&a.res.0)
                    .and_then(|pred| trackers.get(pred))
                    .map(AccessTracker::last_state)
            };
            let tracker = trackers.entry(a.res.0).or_insert_with(AccessTracker::new);
            if let Some(b) = tracker.update(p.id, a.res, req_sync, req_access, req_layout, ho) {
                batch.push(b);
            }
        }
        batches.push(batch);
    }

    // ── 趟4 车道划分(异步 fence 对 + 依赖环拒)──
    let fences = plan_lanes(&survivors, &options)?;

    // ── 装配产物 ──
    let mut compiled_passes = Vec::with_capacity(survivors.len());
    for (node, batch) in survivors.into_iter().zip(batches) {
        compiled_passes.push(CompiledPass {
            id: node.id,
            name: node.desc.name,
            queue: effective_queue(node.desc.queue),
            reads: node.desc.reads,
            writes: node.desc.writes,
            barriers_before: batch,
            execute: node.execute,
        });
    }
    let barriers: Vec<(PassId, Vec<Barrier>)> = compiled_passes
        .iter()
        .map(|p| (p.id, p.barriers_before.clone()))
        .collect();
    let mut compiled_resources = Vec::with_capacity(used.len());
    for &rid in &used {
        let node = &resources[rid as usize];
        let imported = node.desc.imported;
        compiled_resources.push(CompiledResource {
            id: node.id,
            name: node.desc.name.clone(),
            kind: node.desc.kind,
            imported,
            lifetime: if imported {
                None
            } else {
                lifetimes.get(&rid).copied()
            },
            slot: if imported {
                None
            } else {
                pool.slot_of(node.id)
            },
            last_writer: trackers.get(&rid).and_then(AccessTracker::last_writer),
        });
    }

    Ok(CompiledGraph {
        passes: compiled_passes,
        resources: compiled_resources,
        barriers,
        fences,
        pool,
        culled_pass_count,
        culled_resource_count,
    })
}

/// 结构面校验(剔除前;①②③④)。
fn validate(resources: &[ResourceNode], passes: &[PassNode]) -> Result<(), GraphError> {
    // ③ 越期句柄(资源 id 越界)。
    for p in passes {
        for a in p.desc.reads.iter().chain(p.desc.writes.iter()) {
            if a.res.0 as usize >= resources.len() {
                return Err(GraphError::InvalidHandle {
                    pass: p.desc.name.clone(),
                    resource: a.res,
                });
            }
        }
    }
    // ② 同 pass 冲突(同资源多次声明 = 写写/读写冲突未声明序)。
    for p in passes {
        let mut seen: BTreeSet<u32> = BTreeSet::new();
        for a in p.desc.reads.iter().chain(p.desc.writes.iter()) {
            if !seen.insert(a.res.0) {
                return Err(GraphError::WriteWriteConflict {
                    pass: p.desc.name.clone(),
                    resource: resources[a.res.0 as usize].desc.name.clone(),
                });
            }
        }
    }
    // ① 读未写(transient 消费读须有先前写;imported 外部初始化豁免)。
    let mut written: BTreeSet<u32> = BTreeSet::new();
    for p in passes {
        for a in &p.desc.reads {
            let node = &resources[a.res.0 as usize];
            if !node.desc.imported && !written.contains(&a.res.0) {
                return Err(GraphError::ReadBeforeWrite {
                    pass: p.desc.name.clone(),
                    resource: node.desc.name.clone(),
                });
            }
        }
        for a in &p.desc.writes {
            if a.access.is_write() {
                written.insert(a.res.0);
            }
        }
    }
    // ④ 重复 Present(整图至多一处 handoff)。
    let mut presented = false;
    for p in passes {
        let has_present = p
            .desc
            .reads
            .iter()
            .chain(p.desc.writes.iter())
            .any(|a| a.access == AccessKind::Present);
        if has_present {
            if presented {
                return Err(GraphError::DuplicatePresent {
                    pass: p.desc.name.clone(),
                });
            }
            presented = true;
        }
    }
    Ok(())
}

/// 趟1 剔除:反向需求扫描。根 = 写 imported 资源 / 含 Present 访问的 pass;
/// 幸存 pass 的写满足更晚需求(全覆盖语义),其读向上游传播需求。
fn cull(resources: &[ResourceNode], passes: &[PassNode]) -> Vec<bool> {
    let mut needed = vec![false; passes.len()];
    let mut alive: BTreeSet<u32> = BTreeSet::new();
    for (i, p) in passes.iter().enumerate().rev() {
        let d = &p.desc;
        let is_root = d
            .writes
            .iter()
            .any(|a| a.access.is_write() && resources[a.res.0 as usize].desc.imported)
            || d.reads
                .iter()
                .chain(d.writes.iter())
                .any(|a| a.access == AccessKind::Present);
        let produces_demand = d.writes.iter().any(|a| alive.contains(&a.res.0));
        if is_root || produces_demand {
            needed[i] = true;
            for w in &d.writes {
                alive.remove(&w.res.0);
            }
            for r in &d.reads {
                alive.insert(r.res.0);
            }
        }
    }
    needed
}

/// 别名交接前任表:同槽链上按首用排序,紧邻前主即交接对象(其末用 < 本首用
/// 由着色复用判据保证)。
fn alias_predecessors(
    pool: &TransientPool,
    entries: &[(ResourceId, ResourceKind, LifeInterval)],
) -> BTreeMap<u32, u32> {
    let mut by_slot: BTreeMap<(u32, u32), Vec<(u32, LifeInterval)>> = BTreeMap::new();
    for (id, _, iv) in entries {
        let s = pool.slot_of(*id).expect("transient 必入池");
        by_slot
            .entry((s.bucket, s.slot))
            .or_default()
            .push((id.0, *iv));
    }
    let mut pred = BTreeMap::new();
    for members in by_slot.values_mut() {
        members.sort_by_key(|(_, iv)| iv.first_use);
        for w in members.windows(2) {
            pred.insert(w[1].0, w[0].0);
        }
    }
    pred
}

/// 趟4 车道划分:逐 AsyncCompute pass 求 fence 弧(signal = 最后图形生产者,
/// wait = 首个图形消费者;WAR/WAW 冲突方按声明序归侧),并判 ⑤ 版本倒置环。
/// 相同 (signal, wait) 弧去重共享,timeline 值按弧序单调分配(1 起)。
fn plan_lanes(passes: &[PassNode], options: &CompileOptions) -> Result<Vec<FencePair>, GraphError> {
    if !options.enable_async {
        return Ok(Vec::new());
    }
    let declares_write = |p: &PassNode, r: ResourceId| p.desc.writes.iter().any(|a| a.res == r);
    let declares_read = |p: &PassNode, r: ResourceId| p.desc.reads.iter().any(|a| a.res == r);
    let mut arcs: BTreeSet<(u32, u32)> = BTreeSet::new();
    for (i, p) in passes.iter().enumerate() {
        if p.desc.queue != QueueClass::AsyncCompute {
            continue;
        }
        let mut before: BTreeSet<usize> = BTreeSet::new();
        let mut after: BTreeSet<usize> = BTreeSet::new();
        let mut last_prod: Option<usize> = None;
        let mut first_cons: Option<usize> = None;
        for (j, g) in passes.iter().enumerate() {
            if j == i || g.desc.queue != QueueClass::Graphics {
                continue;
            }
            // 我的输入被图形写:j<i = RAW 生产者(signal 侧);j>i = 帧内覆写
            // (WAR,须等我读完 → wait 侧)。
            for a in &p.desc.reads {
                if declares_write(g, a.res) {
                    last_prod = Some(last_prod.map_or(j, |p0| p0.max(j)));
                    if j < i {
                        before.insert(j);
                    } else {
                        after.insert(j);
                    }
                }
            }
            // 我的产出被图形读/写:j>i = 消费者/后继写(wait 侧);j<i = 旧版本
            // 读/写(WAR/WAW,signal 侧)。
            for a in &p.desc.writes {
                if declares_read(g, a.res) || declares_write(g, a.res) {
                    if j > i {
                        first_cons = Some(first_cons.map_or(j, |c| c.min(j)));
                        after.insert(j);
                    } else {
                        before.insert(j);
                    }
                }
            }
        }
        // ⑤ 版本倒置:输入的最后图形生产者晚于产出的首个图形消费者 → fence 弧无解。
        if let (Some(prod), Some(cons)) = (last_prod, first_cons)
            && prod > cons
        {
            return Err(GraphError::AsyncDependencyCycle {
                pass: p.desc.name.clone(),
                detail: format!(
                    "输入的最后图形生产者 pass#{prod} 晚于产出的首个图形消费者 pass#{cons}(声明面不表达双缓冲语义,保守拒)"
                ),
            });
        }
        if let (Some(&s), Some(&w)) = (before.iter().max(), after.iter().min()) {
            arcs.insert((
                u32::try_from(s).unwrap_or(u32::MAX),
                u32::try_from(w).unwrap_or(u32::MAX),
            ));
        }
    }
    Ok(arcs
        .into_iter()
        .enumerate()
        .map(|(v, (s, w))| FencePair {
            signal_after: passes[s as usize].id,
            wait_before: passes[w as usize].id,
            value: v as u64 + 1,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::types::{AccessMask, PassDesc, ResourceDesc, SyncStage, TextureFormat};

    const MB: u64 = 1024 * 1024;

    fn tex(name: &str, format: TextureFormat) -> ResourceDesc {
        ResourceDesc {
            name: name.to_owned(),
            kind: ResourceKind::Texture2d {
                width: 512,
                height: 512,
                format,
                mip_levels: 1,
            },
            imported: false,
        }
    }

    fn buf(name: &str, size: u64) -> ResourceDesc {
        ResourceDesc {
            name: name.to_owned(),
            kind: ResourceKind::Buffer { size },
            imported: false,
        }
    }

    fn ra(res: ResourceId, access: AccessKind) -> ResAccess {
        ResAccess { res, access }
    }

    fn pd(
        name: &str,
        queue: QueueClass,
        reads: Vec<ResAccess>,
        writes: Vec<ResAccess>,
    ) -> PassDesc {
        PassDesc {
            name: name.to_owned(),
            queue,
            reads,
            writes,
        }
    }

    fn gfx(name: &str, reads: Vec<ResAccess>, writes: Vec<ResAccess>) -> PassDesc {
        pd(name, QueueClass::Graphics, reads, writes)
    }

    fn bar(
        res: u32,
        sb: SyncStage,
        sa: SyncStage,
        ab: AccessMask,
        aa: AccessMask,
        lb: ImageLayout,
        la: ImageLayout,
    ) -> Barrier {
        Barrier {
            res: ResourceId(res),
            sync_before: sb,
            sync_after: sa,
            access_before: ab,
            access_after: aa,
            layout_before: lb,
            layout_after: la,
        }
    }

    // ── golden:deferred 三 pass 图(gbuffer→lighting→post)────────────────────

    fn deferred_graph() -> RenderGraph {
        let mut g = RenderGraph::new();
        let a = g.create(tex("gbuf:Albedo", TextureFormat::Rgba8Unorm));
        let n = g.create(tex("gbuf:Normal", TextureFormat::Rgba16Float));
        let d = g.create(tex("gbuf:Depth", TextureFormat::Depth32Float));
        let hdr = g.create(tex("lighting:Hdr", TextureFormat::Rgba16Float));
        let bb = g.import(tex("backbuffer", TextureFormat::Rgba8Unorm));
        g.add_pass(gfx(
            "gbuffer",
            vec![],
            vec![
                ra(a, AccessKind::ColorTarget),
                ra(n, AccessKind::ColorTarget),
                ra(d, AccessKind::DepthTarget),
            ],
        ));
        g.add_pass(gfx(
            "lighting",
            vec![
                ra(a, AccessKind::ShaderRead),
                ra(n, AccessKind::ShaderRead),
                ra(d, AccessKind::DepthRead),
            ],
            vec![ra(hdr, AccessKind::ColorTarget)],
        ));
        g.add_pass(gfx(
            "post",
            vec![ra(hdr, AccessKind::ShaderRead)],
            vec![ra(bb, AccessKind::ColorTarget)],
        ));
        g
    }

    /// golden:deferred 三 pass 屏障序列逐字段锚定(首用预屏障 ×4 / RAW ×4 /
    /// 首用 imported 写 ×1,共 9 条)。
    #[test]
    fn golden_deferred_barriers() {
        let c = deferred_graph()
            .compile(CompileOptions::default())
            .expect("合法图");
        let expected: Vec<(PassId, Vec<Barrier>)> = vec![
            (
                PassId(0),
                vec![
                    bar(
                        0,
                        SyncStage::None,
                        SyncStage::Graphics,
                        AccessMask::None,
                        AccessMask::Write,
                        ImageLayout::Undefined,
                        ImageLayout::ColorAttachment,
                    ),
                    bar(
                        1,
                        SyncStage::None,
                        SyncStage::Graphics,
                        AccessMask::None,
                        AccessMask::Write,
                        ImageLayout::Undefined,
                        ImageLayout::ColorAttachment,
                    ),
                    bar(
                        2,
                        SyncStage::None,
                        SyncStage::Graphics,
                        AccessMask::None,
                        AccessMask::Write,
                        ImageLayout::Undefined,
                        ImageLayout::DepthAttachment,
                    ),
                ],
            ),
            (
                PassId(1),
                vec![
                    bar(
                        0,
                        SyncStage::Graphics,
                        SyncStage::Graphics,
                        AccessMask::Write,
                        AccessMask::Read,
                        ImageLayout::ColorAttachment,
                        ImageLayout::ShaderReadOnly,
                    ),
                    bar(
                        1,
                        SyncStage::Graphics,
                        SyncStage::Graphics,
                        AccessMask::Write,
                        AccessMask::Read,
                        ImageLayout::ColorAttachment,
                        ImageLayout::ShaderReadOnly,
                    ),
                    bar(
                        2,
                        SyncStage::Graphics,
                        SyncStage::Graphics,
                        AccessMask::Write,
                        AccessMask::Read,
                        ImageLayout::DepthAttachment,
                        ImageLayout::ShaderReadOnly,
                    ),
                    bar(
                        3,
                        SyncStage::None,
                        SyncStage::Graphics,
                        AccessMask::None,
                        AccessMask::Write,
                        ImageLayout::Undefined,
                        ImageLayout::ColorAttachment,
                    ),
                ],
            ),
            (
                PassId(2),
                vec![
                    bar(
                        3,
                        SyncStage::Graphics,
                        SyncStage::Graphics,
                        AccessMask::Write,
                        AccessMask::Read,
                        ImageLayout::ColorAttachment,
                        ImageLayout::ShaderReadOnly,
                    ),
                    bar(
                        4,
                        SyncStage::None,
                        SyncStage::Graphics,
                        AccessMask::None,
                        AccessMask::Write,
                        ImageLayout::Undefined,
                        ImageLayout::ColorAttachment,
                    ),
                ],
            ),
        ];
        assert_eq!(c.barriers(), expected.as_slice(), "屏障序列 golden 漂移");
        // 每 pass 前屏障批与全局表一致。
        for p in c.passes() {
            assert_eq!(p.barriers_before(), &c.barriers()[p.id().0 as usize].1[..]);
        }
        // 生命周期:albedo/normal/depth [0,1],hdr [1,2];imported 无区间无槽位。
        let res = |id: u32| c.resource(ResourceId(id)).expect("资源幸存");
        let liv = |f: u32, l: u32| {
            Some(LifeInterval {
                first_use: PassId(f),
                last_use: PassId(l),
            })
        };
        assert_eq!(res(0).lifetime(), liv(0, 1));
        assert_eq!(res(3).lifetime(), liv(1, 2));
        assert_eq!(res(4).lifetime(), None);
        assert_eq!(res(4).slot(), None);
        assert_eq!(res(4).last_writer(), Some(PassId(2)));
        // 生命周期相交互斥别名:1MB 桶 albedo/depth 异槽,2MB 桶 normal/hdr 异槽。
        let (sa, sn, sd, sh) = (
            res(0).slot().expect("slot"),
            res(1).slot().expect("slot"),
            res(2).slot().expect("slot"),
            res(3).slot().expect("slot"),
        );
        assert_eq!(sa.bucket, sd.bucket);
        assert_ne!(sa.slot, sd.slot);
        assert_eq!(sn.bucket, sh.bucket);
        assert_ne!(sn.slot, sh.slot);
        assert_ne!(sa.bucket, sn.bucket);
        // 无 fence(全图形车道)。
        assert!(c.fences().is_empty());
    }

    // ── 参考帧图:GBuffer→(异步 AO)→lighting→TAA→blit→present ─────────────

    fn reference_frame() -> RenderGraph {
        let mut g = RenderGraph::new();
        let a = g.create(tex("gbuf:Albedo", TextureFormat::Rgba8Unorm));
        let n = g.create(tex("gbuf:Normal", TextureFormat::Rgba16Float));
        let d = g.create(tex("gbuf:Depth", TextureFormat::Depth32Float));
        let ao = g.create(tex("ao:Raw", TextureFormat::Rgba8Unorm));
        let hdr = g.create(tex("lighting:Hdr", TextureFormat::Rgba16Float));
        let hist = g.import(tex("taa:History", TextureFormat::Rgba16Float));
        let out = g.create(tex("taa:Out", TextureFormat::Rgba16Float));
        let bb = g.import(tex("backbuffer", TextureFormat::Rgba8Unorm));
        g.add_pass(gfx(
            "gbuffer",
            vec![],
            vec![
                ra(a, AccessKind::ColorTarget),
                ra(n, AccessKind::ColorTarget),
                ra(d, AccessKind::DepthTarget),
            ],
        )); // 0
        g.add_pass(pd(
            "ao",
            QueueClass::AsyncCompute,
            vec![ra(n, AccessKind::ShaderRead), ra(d, AccessKind::DepthRead)],
            vec![ra(ao, AccessKind::ShaderWrite)],
        )); // 1
        g.add_pass(gfx(
            "lighting",
            vec![
                ra(a, AccessKind::ShaderRead),
                ra(n, AccessKind::ShaderRead),
                ra(d, AccessKind::DepthRead),
                ra(ao, AccessKind::ShaderRead),
            ],
            vec![ra(hdr, AccessKind::ColorTarget)],
        )); // 2
        g.add_pass(gfx(
            "taa",
            vec![
                ra(hdr, AccessKind::ShaderRead),
                ra(hist, AccessKind::ShaderRead),
            ],
            vec![ra(out, AccessKind::ColorTarget)],
        )); // 3
        g.add_pass(gfx(
            "blit",
            vec![ra(out, AccessKind::ShaderRead)],
            vec![ra(bb, AccessKind::ColorTarget)],
        )); // 4
        g.add_pass(gfx("present", vec![ra(bb, AccessKind::Present)], vec![])); // 5
        g
    }

    /// 参考帧图:编译成功;AO 车道 fence 对 = gbuffer 后 signal / lighting 前 wait,
    /// timeline 值 1 起;关键屏障(跨车道 RAW / fake flush / present)逐字段锚定。
    #[test]
    fn reference_frame_compiles_with_ao_fence() {
        let c = reference_frame()
            .compile(CompileOptions::default())
            .expect("参考帧图应编译通过");
        assert_eq!(
            c.fences(),
            &[FencePair {
                signal_after: PassId(0),
                wait_before: PassId(2),
                value: 1,
            }],
            "AO 车道 fence 对"
        );
        assert_eq!(c.passes()[1].name(), "ao");
        assert_eq!(c.passes()[1].queue(), QueueClass::AsyncCompute);
        // AO 的跨车道 RAW:graphics 写 → compute 读;aoTex 首写 ShaderWrite → General。
        let ao_batch = &c.barriers()[1].1;
        assert!(ao_batch.contains(&bar(
            1,
            SyncStage::Graphics,
            SyncStage::Compute,
            AccessMask::Write,
            AccessMask::Read,
            ImageLayout::ColorAttachment,
            ImageLayout::ShaderReadOnly
        )));
        assert!(ao_batch.contains(&bar(
            2,
            SyncStage::Graphics,
            SyncStage::Compute,
            AccessMask::Write,
            AccessMask::Read,
            ImageLayout::DepthAttachment,
            ImageLayout::ShaderReadOnly
        )));
        assert!(ao_batch.contains(&bar(
            3,
            SyncStage::None,
            SyncStage::Compute,
            AccessMask::None,
            AccessMask::ReadWrite,
            ImageLayout::Undefined,
            ImageLayout::General
        )));
        // lighting 二度消费 normal/depth = fake flush(access_before=None,layout 不变)。
        let light_batch = &c.barriers()[2].1;
        assert!(light_batch.contains(&bar(
            1,
            SyncStage::Graphics,
            SyncStage::Graphics,
            AccessMask::None,
            AccessMask::Read,
            ImageLayout::ShaderReadOnly,
            ImageLayout::ShaderReadOnly
        )));
        assert!(light_batch.contains(&bar(
            2,
            SyncStage::Graphics,
            SyncStage::Graphics,
            AccessMask::None,
            AccessMask::Read,
            ImageLayout::ShaderReadOnly,
            ImageLayout::ShaderReadOnly
        )));
        // AO 产出 → lighting:compute 写真 flush(sync_before=Compute)。
        assert!(light_batch.contains(&bar(
            3,
            SyncStage::Compute,
            SyncStage::Graphics,
            AccessMask::ReadWrite,
            AccessMask::Read,
            ImageLayout::General,
            ImageLayout::ShaderReadOnly
        )));
        // present 终端屏障:layout → Present,sync_after=None。
        let present_batch = &c.barriers()[5].1;
        assert_eq!(
            present_batch.as_slice(),
            &[bar(
                7,
                SyncStage::Graphics,
                SyncStage::None,
                AccessMask::Write,
                AccessMask::Read,
                ImageLayout::ColorAttachment,
                ImageLayout::Present
            )]
        );
    }

    /// 异步回落:enable_async=false → 无 fence,全部有效车道 = Graphics。
    #[test]
    fn async_fallback_demotes_to_graphics_lane() {
        let c = reference_frame()
            .compile(CompileOptions {
                enable_async: false,
                ..CompileOptions::default()
            })
            .expect("合法图");
        assert!(c.fences().is_empty());
        assert!(c.passes().iter().all(|p| p.queue() == QueueClass::Graphics));
        // 回落后 AO 访问 stage 也是 Graphics(单车道推导)。
        let ao_batch = &c.barriers()[1].1;
        assert!(ao_batch.contains(&bar(
            1,
            SyncStage::Graphics,
            SyncStage::Graphics,
            AccessMask::Write,
            AccessMask::Read,
            ImageLayout::ColorAttachment,
            ImageLayout::ShaderReadOnly
        )));
    }

    // ── 五类错误注入(RED 自检)─────────────────────────────────────────────

    /// ① 读未写:transient 消费读无先前写 → ReadBeforeWrite。
    #[test]
    fn rejects_read_before_write() {
        let mut g = RenderGraph::new();
        let a = g.create(tex("a", TextureFormat::Rgba8Unorm));
        let out = g.import(tex("out", TextureFormat::Rgba8Unorm));
        g.add_pass(gfx(
            "lighting",
            vec![ra(a, AccessKind::ShaderRead)],
            vec![ra(out, AccessKind::ColorTarget)],
        ));
        match g.compile(CompileOptions::default()) {
            Err(GraphError::ReadBeforeWrite { pass, resource }) => {
                assert_eq!(pass, "lighting");
                assert_eq!(resource, "a");
            }
            other => panic!("读未写应确定性拒,实得 {:?}", other.map(|_| ())),
        }
    }

    /// ① 对照:imported 外部初始化豁免读未写(跨帧历史纪律),首读发预屏障。
    #[test]
    fn accepts_imported_read_without_inframe_write() {
        let mut g = RenderGraph::new();
        let hist = g.import(tex("hist", TextureFormat::Rgba8Unorm));
        let out = g.import(tex("out", TextureFormat::Rgba8Unorm));
        g.add_pass(gfx(
            "taa",
            vec![ra(hist, AccessKind::ShaderRead)],
            vec![ra(out, AccessKind::ColorTarget)],
        ));
        let c = g
            .compile(CompileOptions::default())
            .expect("imported 读应豁免");
        assert!(c.barriers()[0].1.contains(&bar(
            0,
            SyncStage::None,
            SyncStage::Graphics,
            AccessMask::None,
            AccessMask::Read,
            ImageLayout::Undefined,
            ImageLayout::ShaderReadOnly
        )));
    }

    /// ② 同 pass 冲突:两次写同资源 / 同 pass 读写同资源 → WriteWriteConflict。
    #[test]
    fn rejects_write_write_conflict() {
        let mut g = RenderGraph::new();
        let a = g.create(tex("a", TextureFormat::Rgba8Unorm));
        g.add_pass(gfx(
            "geo",
            vec![],
            vec![
                ra(a, AccessKind::ColorTarget),
                ra(a, AccessKind::ColorTarget),
            ],
        ));
        assert!(matches!(
            g.compile(CompileOptions::default()),
            Err(GraphError::WriteWriteConflict { .. })
        ));
        // 同 pass 读写同资源(feedback)。
        let mut g2 = RenderGraph::new();
        let a2 = g2.create(tex("a", TextureFormat::Rgba8Unorm));
        let out = g2.import(tex("out", TextureFormat::Rgba8Unorm));
        g2.add_pass(gfx("mk", vec![], vec![ra(a2, AccessKind::ColorTarget)]));
        g2.add_pass(gfx(
            "bad",
            vec![ra(a2, AccessKind::ShaderRead)],
            vec![
                ra(a2, AccessKind::ColorTarget),
                ra(out, AccessKind::ColorTarget),
            ],
        ));
        match g2.compile(CompileOptions::default()) {
            Err(GraphError::WriteWriteConflict { pass, .. }) => assert_eq!(pass, "bad"),
            other => panic!("同 pass 读写冲突应拒,实得 {:?}", other.map(|_| ())),
        }
    }

    /// ③ 越期句柄:资源 id 越界 → InvalidHandle。
    #[test]
    fn rejects_invalid_handle() {
        let mut g = RenderGraph::new();
        let out = g.import(tex("out", TextureFormat::Rgba8Unorm));
        g.add_pass(gfx(
            "p",
            vec![ra(ResourceId(99), AccessKind::ShaderRead)],
            vec![ra(out, AccessKind::ColorTarget)],
        ));
        match g.compile(CompileOptions::default()) {
            Err(GraphError::InvalidHandle { pass, resource }) => {
                assert_eq!(pass, "p");
                assert_eq!(resource, ResourceId(99));
            }
            other => panic!("越界句柄应拒,实得 {:?}", other.map(|_| ())),
        }
    }

    /// ④ 重复 Present:第二处 handoff → DuplicatePresent。
    #[test]
    fn rejects_duplicate_present() {
        let mut g = RenderGraph::new();
        let bb = g.import(tex("backbuffer", TextureFormat::Rgba8Unorm));
        g.add_pass(gfx("blit", vec![], vec![ra(bb, AccessKind::ColorTarget)]));
        g.add_pass(gfx("present_a", vec![ra(bb, AccessKind::Present)], vec![]));
        g.add_pass(gfx("present_b", vec![ra(bb, AccessKind::Present)], vec![]));
        match g.compile(CompileOptions::default()) {
            Err(GraphError::DuplicatePresent { pass }) => assert_eq!(pass, "present_b"),
            other => panic!("重复 Present 应拒,实得 {:?}", other.map(|_| ())),
        }
    }

    /// ⑤ 异步依赖环:帧内更晚图形写覆写异步输入(版本倒置),而产出在更早图形
    /// pass 被消费 → fence 弧无解,确定性拒。
    #[test]
    fn rejects_async_dependency_cycle() {
        let mut g = RenderGraph::new();
        let x = g.create(tex("x", TextureFormat::Rgba8Unorm));
        let y = g.create(tex("y", TextureFormat::Rgba8Unorm));
        let o1 = g.import(tex("o1", TextureFormat::Rgba8Unorm));
        let o2 = g.import(tex("o2", TextureFormat::Rgba8Unorm));
        g.add_pass(gfx("g0", vec![], vec![ra(x, AccessKind::ColorTarget)])); // 0:x 生产者
        g.add_pass(pd(
            "a1",
            QueueClass::AsyncCompute,
            vec![ra(x, AccessKind::ShaderRead)],
            vec![ra(y, AccessKind::ShaderWrite)],
        )); // 1:异步 x→y
        g.add_pass(gfx(
            "g2",
            vec![ra(y, AccessKind::ShaderRead)],
            vec![ra(o1, AccessKind::ColorTarget)],
        )); // 2:y 消费者
        g.add_pass(gfx(
            "g3",
            vec![],
            vec![
                ra(x, AccessKind::ColorTarget),
                ra(o2, AccessKind::ColorTarget),
            ],
        )); // 3:帧内覆写 x → 版本倒置
        match g.compile(CompileOptions::default()) {
            Err(GraphError::AsyncDependencyCycle { pass, .. }) => assert_eq!(pass, "a1"),
            other => panic!("版本倒置应判异步依赖环,实得 {:?}", other.map(|_| ())),
        }
    }

    // ── 剔除 ────────────────────────────────────────────────────────────────

    fn culling_graph() -> RenderGraph {
        let mut g = RenderGraph::new();
        let keep = g.create(tex("keep", TextureFormat::Rgba8Unorm));
        let dead = g.create(tex("dead", TextureFormat::Rgba8Unorm));
        let _lonely = g.create(tex("lonely", TextureFormat::Rgba8Unorm)); // 从未被访问
        let out = g.import(tex("out", TextureFormat::Rgba8Unorm));
        g.add_pass(gfx(
            "produce",
            vec![],
            vec![ra(keep, AccessKind::ColorTarget)],
        ));
        g.add_pass(gfx(
            "dead_pass",
            vec![],
            vec![ra(dead, AccessKind::ColorTarget)],
        ));
        g.add_pass(gfx(
            "consume",
            vec![ra(keep, AccessKind::ShaderRead)],
            vec![ra(out, AccessKind::ColorTarget)],
        ));
        g
    }

    /// 趟1 剔除:无贡献 pass(dead_pass)与无访问资源(dead/lonely)不进产物;
    /// 幸存 pass 保留原 id 与线性序。
    #[test]
    fn culls_unproductive_passes_and_resources() {
        let c = culling_graph()
            .compile(CompileOptions::default())
            .expect("合法图");
        assert_eq!(c.passes().len(), 2);
        assert_eq!(c.passes()[0].name(), "produce");
        assert_eq!(c.passes()[1].name(), "consume");
        assert_eq!(c.passes()[1].id(), PassId(2), "原 PassId 保留");
        assert_eq!(c.culled_pass_count(), 1);
        assert_eq!(c.resources().len(), 2);
        assert_eq!(c.culled_resource_count(), 2);
        assert_eq!(c.barriers().len(), 2, "屏障批只覆盖幸存 pass");
    }

    /// 剔除可整体关闭(调试阀门):dead_pass 幸存,dead 入池分配。
    #[test]
    fn culling_can_be_disabled() {
        let c = culling_graph()
            .compile(CompileOptions {
                enable_culling: false,
                ..CompileOptions::default()
            })
            .expect("合法图");
        assert_eq!(c.passes().len(), 3);
        assert_eq!(c.culled_pass_count(), 0);
        assert_eq!(c.resources().len(), 3, "lonely 无访问仍不进产物");
        assert_eq!(c.culled_resource_count(), 1);
    }

    // ── 别名交接 / 峰值审计 ────────────────────────────────────────────────

    /// 别名交接:a[0,1] / b[1,2] / c[2,3] → a、c 同槽;c 首用屏障 before 侧取
    /// 前任 a 末态(Graphics/Read),layout_before=Undefined;别名后峰值 < 无别名峰值。
    #[test]
    fn alias_handoff_and_peak_audit() {
        let mut g = RenderGraph::new();
        let a = g.create(tex("a", TextureFormat::Rgba8Unorm));
        let b = g.create(tex("b", TextureFormat::Rgba8Unorm));
        let cc = g.create(tex("c", TextureFormat::Rgba8Unorm));
        let out = g.import(tex("out", TextureFormat::Rgba8Unorm));
        g.add_pass(gfx("mk_a", vec![], vec![ra(a, AccessKind::ColorTarget)])); // 0
        g.add_pass(gfx(
            "use_a",
            vec![ra(a, AccessKind::ShaderRead)],
            vec![ra(b, AccessKind::ColorTarget)],
        )); // 1
        g.add_pass(gfx(
            "use_b",
            vec![ra(b, AccessKind::ShaderRead)],
            vec![ra(cc, AccessKind::ColorTarget)],
        )); // 2
        g.add_pass(gfx(
            "use_c",
            vec![ra(cc, AccessKind::ShaderRead)],
            vec![ra(out, AccessKind::ColorTarget)],
        )); // 3
        let c = g.compile(CompileOptions::default()).expect("合法图");
        let slot = |id: ResourceId| c.resource(id).expect("幸存").slot().expect("入池");
        assert_eq!(slot(a), slot(cc), "a/c 区间不相交应别名同槽");
        assert_ne!(
            (slot(a).bucket, slot(a).slot),
            (slot(b).bucket, slot(b).slot)
        );
        // 峰值审计:别名后 2 槽 × 1MB < 无别名 3 × 1MB。
        assert_eq!(c.pool().high_water(), 2 * MB);
        assert_eq!(c.pool().no_alias_peak(), 3 * MB);
        assert!(c.pool().high_water() < c.pool().no_alias_peak());
        // c 首用屏障 = 别名交接(前任 a 末态 Graphics/Read;Undefined 入局)。
        let use_b_batch = &c.barriers()[2].1;
        assert!(use_b_batch.contains(&bar(
            2,
            SyncStage::Graphics,
            SyncStage::Graphics,
            AccessMask::Read,
            AccessMask::Write,
            ImageLayout::Undefined,
            ImageLayout::ColorAttachment
        )));
    }

    // ── 只读链 fake flush / buffer layout 纪律 ─────────────────────────────

    /// 连续只读链:同 stage 静默;跨 stage(copy)首读 fake flush(access_before=None)。
    #[test]
    fn read_chain_uses_fake_flush_across_stages() {
        let mut g = RenderGraph::new();
        let b0 = g.create(buf("b0", 4096));
        let rb = g.import(buf("rb", 4096));
        let sink = g.import(tex("sink", TextureFormat::Rgba8Unorm));
        g.add_pass(gfx(
            "produce",
            vec![],
            vec![ra(b0, AccessKind::ShaderWrite)],
        )); // 0
        g.add_pass(gfx(
            "read1",
            vec![ra(b0, AccessKind::ShaderRead)],
            vec![ra(sink, AccessKind::ColorTarget)],
        )); // 1 真 flush
        g.add_pass(gfx(
            "read2",
            vec![ra(b0, AccessKind::ShaderRead)],
            vec![ra(sink, AccessKind::ColorTarget)],
        )); // 2 同 stage 静默
        g.add_pass(gfx(
            "readback",
            vec![ra(b0, AccessKind::CopySrc)],
            vec![ra(rb, AccessKind::CopyDst)],
        )); // 3 跨 stage fake flush
        let c = g.compile(CompileOptions::default()).expect("合法图");
        let b1 = &c.barriers()[1].1;
        assert!(
            b1.contains(&bar(
                0,
                SyncStage::Graphics,
                SyncStage::Graphics,
                AccessMask::ReadWrite,
                AccessMask::Read,
                ImageLayout::Undefined,
                ImageLayout::Undefined
            )),
            "首个读者真 flush"
        );
        let b2 = &c.barriers()[2].1;
        assert!(
            b2.iter().all(|x| x.res != b0),
            "同 stage 只读链不应重复失效"
        );
        let b3 = &c.barriers()[3].1;
        assert_eq!(
            b3.as_slice(),
            &[bar(
                0,
                SyncStage::Graphics,
                SyncStage::Copy,
                AccessMask::None,
                AccessMask::Read,
                ImageLayout::Undefined,
                ImageLayout::Undefined
            )],
            "跨 stage 只读链应 fake flush(buffer 首写读无屏障)"
        );
    }

    /// buffer 恒 Undefined layout(契约);buffer 首写无前态无屏障。
    #[test]
    fn buffers_stay_undefined_layout() {
        let mut g = RenderGraph::new();
        let b0 = g.create(buf("b0", 4096));
        let out = g.import(tex("out", TextureFormat::Rgba8Unorm));
        g.add_pass(gfx("fill", vec![], vec![ra(b0, AccessKind::ShaderWrite)]));
        g.add_pass(gfx(
            "use",
            vec![ra(b0, AccessKind::ShaderRead)],
            vec![ra(out, AccessKind::ColorTarget)],
        ));
        let c = g.compile(CompileOptions::default()).expect("合法图");
        assert!(
            c.barriers()[0].1.iter().all(|x| x.res != b0),
            "buffer 首写无屏障"
        );
        for (_, batch) in c.barriers() {
            for x in batch {
                if x.res == b0 {
                    assert_eq!(x.layout_before, ImageLayout::Undefined);
                    assert_eq!(x.layout_after, ImageLayout::Undefined);
                }
            }
        }
    }

    /// 空图 → 空产物(宽容面;校验不拒)。
    #[test]
    fn empty_graph_compiles_to_empty_product() {
        let c = RenderGraph::new()
            .compile(CompileOptions::default())
            .expect("空图宽容");
        assert!(c.passes().is_empty());
        assert!(c.resources().is_empty());
        assert!(c.barriers().is_empty());
        assert!(c.fences().is_empty());
        assert_eq!(c.pool().high_water(), 0);
        assert_eq!(c.pool().no_alias_peak(), 0);
    }
}
