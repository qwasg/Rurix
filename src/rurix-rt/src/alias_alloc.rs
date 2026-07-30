#![forbid(unsafe_code)]
//! RXS-0280 transient 别名复用分配器 + 执行期峰值计数器(G4.3 PR-E,RD-035 执行面①,
//! RFC-0014 §4.B8)。
//!
//! **定位**:sealed 图上每个 transient 资源生命期区间 = `[首写 pass 序位, 末读 pass 序位]`
//! (含端点);区间不重叠者共享同一设备分配(区间图着色)。尺寸/对齐三分量着色(同槽组按
//! `max(尺寸)` + `max(对齐)` 分配,逐成员核满足性)。**纯 host safe 码**(`#![forbid(unsafe_code)]`
//! 编译期封口),零后端调用、零 GPU 依赖,可 golden 锚。
//!
//! **执行期峰值计数器**([`PeakCounter`]):回放期随分配/释放事件记账并发存活字节峰值
//! (cabi 真实设备分配驱动,非静态推算)。I10 自 `report_only` 升 `measured_local`。
//!
//! **四序闭合**(RXS-0280/0281):seal → 调度(拓扑分层)→ 着色(在调度后序上算生命期区间)
//! → 回放(按调度序派发 + 峰值计数器记账)。**B1 分配器输入 = 最终执行计划,单一事实源**。
//!
//! **零新 RX 码、零新 lang item、零新借用码**;纯库层状态值,不占编译器段位。

use crate::rhi::ResourceId;

// ── 生命期区间与尺寸/对齐三分量 ────────────────────────────────────────────────────

/// transient 资源生命期区间(含端点)。`start` = 首写 pass 序位,`end` = 末读 pass 序位。
/// 无写者的资源不参别名复用(保守分配独立槽);仅读者归 RXS-0262 host 记账面。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LiveRange {
    /// 首写 pass 序位(含端点)。
    pub start: u32,
    /// 末读 pass 序位(含端点)。
    pub end: u32,
}

impl LiveRange {
    /// 新建生命期区间。`start` 须 ≤ `end`(空区间 `start == end` 合法 = 单 pass 写后即读)。
    #[must_use]
    pub fn new(start: u32, end: u32) -> LiveRange {
        LiveRange { start, end }
    }

    /// 无写者哨兵区间(`start = u32::MAX, end = 0`):`start > end` 标识该资源不参别名复用,
    /// 由 [`AliasAlloc::assign`] 保守分配独立槽。与 [`LiveRange::new`] 的 `start ≤ end` 契约
    /// 互斥——哨兵路径专用此构造,常规生命期用 [`LiveRange::new`]。
    #[must_use]
    pub fn no_writer_sentinel() -> LiveRange {
        LiveRange {
            start: u32::MAX,
            end: 0,
        }
    }

    /// 两区间是否**严格不重叠**(端点不含,可共享同一槽)。`a1 < b0 || b1 < a0`(RXS-0280)。
    /// 即 `a.end < b.start || b.end < a.start`(端点相邻 `a.end == b.start` 视为重叠——保守)。
    #[must_use]
    pub fn is_disjoint(self, other: LiveRange) -> bool {
        (self.end as i64) < (other.start as i64) || (other.end as i64) < (self.start as i64)
    }
}

/// 资源字节尺寸(RXS-0280 三分量着色之一)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Size(pub u64);

/// 资源对齐字节数(RXS-0280 三分量着色之一;须为 2 的幂)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Align(pub u64);

/// 单个 transient 资源的生命期描述(`assign` 输入元素)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lifetime {
    /// 资源标识。
    pub resource: ResourceId,
    /// 生命期区间。
    pub range: LiveRange,
    /// 字节尺寸。
    pub size: Size,
    /// 对齐。
    pub align: Align,
}

// ── 着色产物 ────────────────────────────────────────────────────────────────────────

/// 单个资源的槽分配结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotAssignment {
    /// 被分配的资源。
    pub resource: ResourceId,
    /// 槽索引(区间不重叠者共享同一槽)。
    pub slot: u32,
}

/// 单个槽的设备分配信息(同槽组按 `max(尺寸)` + `max(对齐)` 分配)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotInfo {
    /// 槽索引。
    pub slot: u32,
    /// 槽分配字节(= 成员 `max(尺寸)`)。
    pub size: u64,
    /// 槽对齐(= 成员 `max(对齐)`)。
    pub align: u64,
    /// 槽内复用成员表(monotone 追加)。
    pub members: Vec<ResourceId>,
}

/// 别名着色产物 + 静态峰值字节(RXS-0280)。
///
/// `peak_bytes` = 所有槽分配字节之和(别名复用后的设备内存占用上界;非未别名声明容量)。
/// 运行期实测峰值由 [`PeakCounter`] 在回放期记账(cabi 真实设备分配驱动)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasPlan {
    /// 逐资源槽分配(与 `assign` 输入顺序对应)。
    pub slots: Vec<SlotAssignment>,
    /// 逐槽设备分配信息(下标 = 槽索引)。
    pub slot_info: Vec<SlotInfo>,
    /// 静态峰值字节(所有槽分配字节之和)。
    pub peak_bytes: u64,
}

// ── 别名复用分配器(区间图着色)──────────────────────────────────────────────────────

/// transient 别名复用分配器(纯 host 状态;区间图贪心着色)。**纯 host safe 码**。
///
/// 着色算法:按生命期起点排序,贪心为每个资源找第一个与之区间不重叠的既有槽;无可用槽则开新槽。
/// 同槽组按 `max(成员尺寸)` + `max(成员对齐)` 分配(逐成员核满足性:实际分配字节 ≥ 每成员
/// 尺寸、对齐 ≥ 每成员对齐)。O(n²) 可接受(无堆集合约束用 `Vec` 承载)。
#[derive(Debug, Clone, Default)]
pub struct AliasAlloc {
    /// 已分配槽的「当前末读序位」(用于区间不重叠判定的快速核;逐槽单调推进)。
    /// `slot_end[i]` = 槽 i 当前最后一个成员的 `range.end`。
    slot_end: Vec<u32>,
    /// 槽当前 max 尺寸。
    slot_size: Vec<u64>,
    /// 槽当前 max 对齐。
    slot_align: Vec<u64>,
    /// 槽内复用成员表。
    slot_members: Vec<Vec<ResourceId>>,
}

impl AliasAlloc {
    /// 新建空分配器(纯 host 状态)。
    #[must_use]
    pub fn new() -> AliasAlloc {
        AliasAlloc::default()
    }

    /// 区间图着色 → 槽分配(RXS-0280)。输入 = 逐资源生命期(含区间/尺寸/对齐);
    /// 输出 = [`AliasPlan`](着色产物 + 静态峰值字节)。同图同参 → 逐字节相同计划(golden 可锚)。
    ///
    /// **无写者资源不参别名复用**:生命期区间 `start > end`(哨兵 = 无写者,由调用方以
    /// `LiveRange { start: u32::MAX, end: 0 }` 表达)→ 保守分配独立槽。逐成员核满足性:
    /// 实际分配字节 ≥ 每成员尺寸、对齐 ≥ 每成员对齐。
    #[must_use]
    pub fn assign(&mut self, lifetimes: &[Lifetime]) -> AliasPlan {
        // 按生命期起点排序的索引序(稳定:同起点按资源 id 升序)。着色在排序后序上进行。
        // 注意:返回的 `slots` 须与输入顺序对应(逐资源),故先排序索引,着色后再回填。
        let mut order: Vec<usize> = (0..lifetimes.len()).collect();
        // 无写者哨兵(start=MAX)排在最后(独立槽,不影响有写者的复用)。
        order.sort_by(|&a, &b| {
            let la = &lifetimes[a];
            let lb = &lifetimes[b];
            (la.range.start, la.resource.0).cmp(&(lb.range.start, lb.resource.0))
        });

        // 逐资源槽分配(下标 = 输入索引)。
        let mut resource_slot: Vec<u32> = vec![u32::MAX; lifetimes.len()];

        for &idx in &order {
            let lt = &lifetimes[idx];
            let no_writer = lt.range.start == u32::MAX; // 哨兵:无写者 → 独立槽
            let mut chosen: Option<u32> = None;

            if !no_writer {
                // 贪心找第一个与之区间不重叠的既有槽(slot_end < lt.range.start)。
                for (i, &end) in self.slot_end.iter().enumerate() {
                    // 区间不重叠:既有槽末读 < 本资源首写(端点相邻视为重叠——保守)。
                    if (end as i64) < (lt.range.start as i64) {
                        chosen = Some(u32::try_from(i).unwrap_or(u32::MAX));
                        break;
                    }
                }
            }

            let slot = match chosen {
                Some(s) => {
                    let i = s as usize;
                    // 更新槽:max 尺寸 / max 对齐 / 末读序位 / 成员表。
                    if lt.size.0 > self.slot_size[i] {
                        self.slot_size[i] = lt.size.0;
                    }
                    if lt.align.0 > self.slot_align[i] {
                        self.slot_align[i] = lt.align.0;
                    }
                    if lt.range.end > self.slot_end[i] {
                        self.slot_end[i] = lt.range.end;
                    }
                    self.slot_members[i].push(lt.resource);
                    s
                }
                None => {
                    // 开新槽。
                    let i = self.slot_end.len();
                    self.slot_end
                        .push(if no_writer { u32::MAX } else { lt.range.end });
                    self.slot_size.push(lt.size.0);
                    self.slot_align.push(lt.align.0);
                    self.slot_members.push(vec![lt.resource]);
                    u32::try_from(i).unwrap_or(u32::MAX)
                }
            };
            resource_slot[idx] = slot;
        }

        // 构造产物。
        let slots: Vec<SlotAssignment> = lifetimes
            .iter()
            .enumerate()
            .map(|(i, lt)| SlotAssignment {
                resource: lt.resource,
                slot: resource_slot[i],
            })
            .collect();

        let slot_info: Vec<SlotInfo> = (0..self.slot_members.len())
            .map(|i| SlotInfo {
                slot: u32::try_from(i).unwrap_or(u32::MAX),
                size: self.slot_size[i],
                align: self.slot_align[i],
                members: self.slot_members[i].clone(),
            })
            .collect();

        let peak_bytes: u64 = self.slot_size.iter().sum();

        AliasPlan {
            slots,
            slot_info,
            peak_bytes,
        }
    }
}

// ── 执行期峰值计数器(I10 → measured_local)──────────────────────────────────────────

/// 执行期峰值计数器(RXS-0280)。回放期随分配/释放事件记账并发存活字节峰值。
///
/// **cabi 真实设备分配驱动**:执行器在 `rxrt_rhi_resource` 真设备分配时调 [`on_alloc`](Self::on_alloc),
/// 释放时调 [`on_free`](Self::on_free);mock device 段(步骤 79 纯 host)用模拟分配事件驱动。
/// `peak_bytes()` 返回观测到的最大并发存活字节(I10 自 `report_only` 升 `measured_local`)。
#[derive(Debug, Clone)]
pub struct PeakCounter {
    /// 声明容量(图内 transient 资源总字节上界;`on_alloc` 累计可超之,别名复用后实测峰值
    /// 通常收紧——`peak_bytes() < declared_capacity` 非平凡成立)。
    declared_capacity: u64,
    /// 当前并发存活字节。
    current_bytes: u64,
    /// 期间最大并发存活字节。
    peak: u64,
}

impl PeakCounter {
    /// 新建峰值计数器(传入声明容量用于 I10 见证比对)。
    #[must_use]
    pub fn new(declared_capacity: u64) -> PeakCounter {
        PeakCounter {
            declared_capacity,
            current_bytes: 0,
            peak: 0,
        }
    }

    /// 回放期分配事件记账(执行器在真设备分配时调用)。
    pub fn on_alloc(&mut self, bytes: u64) {
        self.current_bytes = self.current_bytes.saturating_add(bytes);
        if self.current_bytes > self.peak {
            self.peak = self.current_bytes;
        }
    }

    /// 回放期释放事件记账(执行器在真设备释放时调用)。
    pub fn on_free(&mut self, bytes: u64) {
        self.current_bytes = self.current_bytes.saturating_sub(bytes);
    }

    /// 并发存活字节峰值(期间最大值;I10 measured_local 观测源)。
    #[must_use]
    pub fn peak_bytes(&self) -> u64 {
        self.peak
    }

    /// 当前并发存活字节(回放期瞬时;调试 / 断言用)。
    #[must_use]
    pub fn current_bytes(&self) -> u64 {
        self.current_bytes
    }

    /// 声明容量(I10 见证比对基准;`peak_bytes() < declared_capacity` 非平凡成立)。
    #[must_use]
    pub fn declared_capacity(&self) -> u64 {
        self.declared_capacity
    }
}

// ── 单测(RXS-0280 测试锚定)──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rhi::ResourceId;

    fn rid(n: u32) -> ResourceId {
        ResourceId(n)
    }
    fn lt(n: u32, start: u32, end: u32, size: u64, align: u64) -> Lifetime {
        Lifetime {
            resource: rid(n),
            range: LiveRange::new(start, end),
            size: Size(size),
            align: Align(align),
        }
    }

    /// 重叠区间不共享同一槽(RXS-0280 Legality 核心)。
    //@ spec: RXS-0280
    #[test]
    fn overlapping_intervals_do_not_share() {
        let mut alloc = AliasAlloc::new();
        // a: [0, 2], b: [1, 3] — 重叠 → 异槽。
        let plan = alloc.assign(&[lt(0, 0, 2, 1024, 4), lt(1, 1, 3, 1024, 4)]);
        assert_ne!(plan.slots[0].slot, plan.slots[1].slot, "重叠区间须异槽");
        assert_eq!(plan.slot_info.len(), 2, "两重叠资源 → 两槽");
        assert_eq!(plan.peak_bytes, 2048, "静态峰值 = 两槽尺寸和");
    }

    /// 不重叠区间共享同一槽(RXS-0280 别名复用核心)。
    //@ spec: RXS-0280
    #[test]
    fn disjoint_intervals_share_slot() {
        let mut alloc = AliasAlloc::new();
        // a: [0, 1], b: [2, 3] — 严格不重叠(a.end=1 < b.start=2)→ 共享槽。
        let plan = alloc.assign(&[lt(0, 0, 1, 1024, 4), lt(1, 2, 3, 1024, 4)]);
        assert_eq!(plan.slots[0].slot, plan.slots[1].slot, "不重叠区间须共享槽");
        assert_eq!(plan.slot_info.len(), 1, "两不重叠资源 → 一槽");
        assert_eq!(plan.peak_bytes, 1024, "别名复用后静态峰值 = 单槽尺寸");
        // 槽内复用成员表 monotone 追加。
        assert_eq!(plan.slot_info[0].members, vec![rid(0), rid(1)]);
    }

    /// 端点相邻视为重叠(保守:a.end == b.start 不共享)。
    //@ spec: RXS-0280
    #[test]
    fn adjacent_endpoints_treated_as_overlap() {
        let mut alloc = AliasAlloc::new();
        // a: [0, 2], b: [2, 3] — a.end == b.start → 保守视为重叠 → 异槽。
        let plan = alloc.assign(&[lt(0, 0, 2, 1024, 4), lt(1, 2, 3, 1024, 4)]);
        assert_ne!(plan.slots[0].slot, plan.slots[1].slot, "端点相邻保守异槽");
        assert_eq!(plan.slot_info.len(), 2);
    }

    /// 尺寸/对齐三分量着色:同槽组按 max(尺寸) + max(对齐) 分配,逐成员核满足性。
    //@ spec: RXS-0280
    #[test]
    fn size_align_three_component_coloring() {
        let mut alloc = AliasAlloc::new();
        // a: [0,1] size=512 align=4; b: [2,3] size=1024 align=16 — 不重叠共享槽。
        let plan = alloc.assign(&[lt(0, 0, 1, 512, 4), lt(1, 2, 3, 1024, 16)]);
        assert_eq!(plan.slots[0].slot, plan.slots[1].slot, "不重叠共享槽");
        let slot = &plan.slot_info[0];
        assert_eq!(slot.size, 1024, "槽尺寸 = max(成员尺寸)");
        assert_eq!(slot.align, 16, "槽对齐 = max(成员对齐)");
        // 逐成员核满足性:实际分配字节 ≥ 每成员尺寸、对齐 ≥ 每成员对齐。
        assert!(slot.size >= 512);
        assert!(slot.size >= 1024);
        assert!(slot.align >= 4);
        assert!(slot.align >= 16);
        assert_eq!(plan.peak_bytes, 1024);
    }

    /// 无写者资源不参别名复用(保守分配独立槽)。
    //@ spec: RXS-0280
    #[test]
    fn no_writer_resource_gets_independent_slot() {
        let mut alloc = AliasAlloc::new();
        // a: 有写者 [0,2]; b: 无写者哨兵 [MAX, 0]。
        let plan = alloc.assign(&[
            lt(0, 0, 2, 1024, 4),
            Lifetime {
                resource: rid(1),
                range: LiveRange::no_writer_sentinel(),
                size: Size(2048),
                align: Align(8),
            },
        ]);
        assert_ne!(plan.slots[0].slot, plan.slots[1].slot, "无写者须独立槽");
        assert_eq!(plan.slot_info.len(), 2);
    }

    /// 确定性:同图同参 → 逐字节相同计划(golden 可锚)。
    //@ spec: RXS-0280
    #[test]
    fn deterministic_same_plan() {
        let mk = || {
            let mut a = AliasAlloc::new();
            a.assign(&[
                lt(0, 0, 1, 1024, 4),
                lt(1, 2, 3, 2048, 8),
                lt(2, 0, 3, 512, 16),
            ])
        };
        let p1 = mk();
        let p2 = mk();
        assert_eq!(p1, p2, "同图同参 → 逐字节相同计划");
    }

    /// 菱形复用:a 与 c 不重叠共享,d 与 b 不重叠共享(多槽交错)。
    //@ spec: RXS-0280
    #[test]
    fn diamond_reuse_two_slots() {
        let mut alloc = AliasAlloc::new();
        // a:[0,1] c:[2,3] 共享槽 0;b:[0,1] d:[2,3] 共享槽 1(a/b 重叠,c/d 重叠)。
        let plan = alloc.assign(&[
            lt(0, 0, 1, 1024, 4), // a → 槽 0
            lt(1, 0, 1, 1024, 4), // b → 槽 1(与 a 重叠)
            lt(2, 2, 3, 1024, 4), // c → 槽 0(与 a 不重叠)
            lt(3, 2, 3, 1024, 4), // d → 槽 1(与 b 不重叠)
        ]);
        assert_eq!(plan.slots[0].slot, plan.slots[2].slot, "a 与 c 共享");
        assert_eq!(plan.slots[1].slot, plan.slots[3].slot, "b 与 d 共享");
        assert_ne!(plan.slots[0].slot, plan.slots[1].slot, "a 与 b 异槽");
        assert_eq!(plan.slot_info.len(), 2);
        assert_eq!(plan.peak_bytes, 2048);
    }

    // ── PeakCounter(I10 → measured_local)────────────────────────────────────────── //

    /// 峰值计数器单调性 + 释放后回落(RXS-0280)。
    //@ spec: RXS-0280
    #[test]
    fn peak_counter_monotonic_then_fallback() {
        let mut pc = PeakCounter::new(4096);
        assert_eq!(pc.peak_bytes(), 0, "初始峰值 0");
        pc.on_alloc(1024);
        assert_eq!(pc.current_bytes(), 1024);
        assert_eq!(pc.peak_bytes(), 1024);
        pc.on_alloc(2048);
        assert_eq!(pc.current_bytes(), 3072);
        assert_eq!(pc.peak_bytes(), 3072, "峰值单调递增");
        pc.on_free(2048);
        assert_eq!(pc.current_bytes(), 1024, "释放后当前回落");
        assert_eq!(pc.peak_bytes(), 3072, "峰值不随释放回落");
        pc.on_alloc(512);
        assert_eq!(pc.current_bytes(), 1536);
        assert_eq!(pc.peak_bytes(), 3072, "未超旧峰则峰不变");
        pc.on_alloc(4096);
        assert_eq!(pc.current_bytes(), 5632);
        assert_eq!(pc.peak_bytes(), 5632, "超旧峰则峰更新");
    }

    /// 别名复用使实测峰值 < 声明容量(I10 measured_local 非平凡成立)。
    //@ spec: RXS-0280
    #[test]
    fn aliasing_peak_below_declared_capacity() {
        // 两不重叠资源各 1024B,声明容量 = 2048(未别名);别名后单槽 1024B。
        let mut alloc = AliasAlloc::new();
        let plan = alloc.assign(&[lt(0, 0, 1, 1024, 4), lt(1, 2, 3, 1024, 4)]);
        let declared = 2048u64; // 未别名声明容量
        let mut pc = PeakCounter::new(declared);
        // 回放:槽 0 在 [0,1] 分配 1024,释放后再在 [2,3] 复用同槽 1024。
        // 实测峰值 = 1024(单槽并发)< 声明容量 2048。
        pc.on_alloc(plan.slot_info[0].size); // 槽 0 分配
        pc.on_free(plan.slot_info[0].size); // 槽 0 释放(生命期结束)
        // (同槽复用:第二次分配仍在同槽,峰值不增)
        pc.on_alloc(plan.slot_info[0].size);
        pc.on_free(plan.slot_info[0].size);
        assert_eq!(pc.peak_bytes(), 1024, "别名复用后实测峰值 = 单槽尺寸");
        assert!(
            pc.peak_bytes() < pc.declared_capacity(),
            "实测峰 < 声明容量(I10 非平凡)"
        );
    }

    /// 空分配器 → 空计划(边界)。
    //@ spec: RXS-0280
    #[test]
    fn empty_allocator_empty_plan() {
        let mut alloc = AliasAlloc::new();
        let plan = alloc.assign(&[]);
        assert!(plan.slots.is_empty());
        assert!(plan.slot_info.is_empty());
        assert_eq!(plan.peak_bytes, 0);
    }
}
