//! 页池(报告6 §5 `PagePool { slots: FixedVec<128KB>, lru, residency_bitmap }`;
//! RFC-0016 §4.G4「128KB 页槽位固定池 + LRU」)。

use std::collections::HashMap;

use crate::graph::types::STREAM_PAGE_SIZE;

/// 池槽元数据。
#[derive(Debug, Clone)]
struct Slot {
    resource: u32,
    page_index: u32,
    /// root 常驻钉住位:永不驱逐(报告6 §2.4)。
    pinned: bool,
    /// 上次触帧序号——池内单调时钟(每次触帧取唯一值,给 LRU 全序;同帧
    /// 多次触帧不并列,驱逐序确定性的来源)。
    touch_seq: u64,
    /// 入池 payload(≤128KB)。
    data: Vec<u8>,
}

/// [`PagePool::insert`] 结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertOutcome {
    /// 已入池:槽号 + 被驱逐页键(如有)。
    Inserted {
        slot: usize,
        evicted: Option<(u32, u32)>,
    },
    /// 全部槽位被钉住(root 高压),无法腾槽——调用方按停顿处理,不静默失败。
    PoolFull,
}

/// 固定预算 N 槽 × 128KB 页池。
///
/// - **LRU 驱逐**:触帧 = 命中刷新/插入/显式 touch,单调时钟给全序,驱逐时
///   逐槽扫描取最久未触(并列取低槽号)——驱逐序完全确定;
/// - **root 钉住永不驱逐**(「永远有可渲染的东西」,报告6 §2.4);全槽钉住
///   时插入返回 [`InsertOutcome::PoolFull`];
/// - 同页重复 insert = 触帧刷新,不重复占槽、不改写驻留数据(驻留页内容
///   自入池至驱逐不可变)。
#[derive(Debug)]
pub struct PagePool {
    slots: Vec<Option<Slot>>,
    /// (resource, page) → 槽号(报告6 §5 residency_bitmap 的宿主形式)。
    index: HashMap<(u32, u32), usize>,
    /// 池内单调触帧时钟。
    clock: u64,
}

impl PagePool {
    /// 固定槽数池(容量 ≥1;容量 × 128KB 即驻留预算语义,对照 UE
    /// StreamingPool 有界口径,报告6 #3)。
    pub fn new(capacity: usize) -> Self {
        assert!(capacity >= 1, "页池容量至少 1 槽");
        Self {
            slots: vec![None; capacity],
            index: HashMap::new(),
            clock: 0,
        }
    }

    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    /// 驻留页数。
    pub fn resident_count(&self) -> usize {
        self.index.len()
    }

    /// 空闲槽数。
    pub fn free_count(&self) -> usize {
        self.capacity() - self.resident_count()
    }

    /// 驻留查询(不触帧)。
    pub fn lookup(&self, resource: u32, page: u32) -> Option<usize> {
        self.index.get(&(resource, page)).copied()
    }

    /// 触帧刷新 LRU(命中返回槽号)。
    pub fn touch(&mut self, resource: u32, page: u32) -> Option<usize> {
        let &slot_idx = self.index.get(&(resource, page))?;
        let seq = self.next_seq();
        self.slots[slot_idx]
            .as_mut()
            .expect("index 指向的槽必占用")
            .touch_seq = seq;
        Some(slot_idx)
    }

    /// 插入页:已驻留 = 触帧刷新(丢弃新数据,驻留内容不可变);否则取最低
    /// 空闲槽,无空闲则驱逐最久未触的未钉住页。
    ///
    /// `data.len() ≤ STREAM_PAGE_SIZE` 由调用方(引擎)断言,本层
    /// debug_assert 复核。
    pub fn insert(
        &mut self,
        resource: u32,
        page: u32,
        data: Vec<u8>,
        pinned: bool,
    ) -> InsertOutcome {
        debug_assert!(
            data.len() <= STREAM_PAGE_SIZE as usize,
            "单页 ≤128KB 契约(resource {resource} page {page})"
        );
        if let Some(slot) = self.touch(resource, page) {
            return InsertOutcome::Inserted {
                slot,
                evicted: None,
            };
        }
        let slot_idx = match self.first_free_slot() {
            Some(i) => i,
            None => match self.lru_victim() {
                Some(i) => i,
                None => return InsertOutcome::PoolFull,
            },
        };
        let old = self.slots[slot_idx].take();
        let evicted = old.map(|s| {
            self.index.remove(&(s.resource, s.page_index));
            (s.resource, s.page_index)
        });
        let seq = self.next_seq();
        self.slots[slot_idx] = Some(Slot {
            resource,
            page_index: page,
            pinned,
            touch_seq: seq,
            data,
        });
        self.index.insert((resource, page), slot_idx);
        InsertOutcome::Inserted {
            slot: slot_idx,
            evicted,
        }
    }

    /// 槽数据(入池 payload;staging 上传源与测试锚定消费)。
    pub fn slot_data(&self, slot: usize) -> &[u8] {
        self.slots[slot]
            .as_ref()
            .expect("查询的槽必占用")
            .data
            .as_slice()
    }

    /// 槽钉住位。
    pub fn is_pinned(&self, slot: usize) -> bool {
        self.slots[slot].as_ref().expect("查询的槽必占用").pinned
    }

    /// 槽键(resource, page)。
    pub fn slot_key(&self, slot: usize) -> (u32, u32) {
        let s = self.slots[slot].as_ref().expect("查询的槽必占用");
        (s.resource, s.page_index)
    }

    fn next_seq(&mut self) -> u64 {
        self.clock += 1;
        self.clock
    }

    /// 最低空闲槽号(分配序确定性)。
    fn first_free_slot(&self) -> Option<usize> {
        self.slots.iter().position(Option::is_none)
    }

    /// LRU 牺牲者:未钉住占用槽中 touch_seq 最小者(并列取低槽号)。
    fn lru_victim(&self) -> Option<usize> {
        let mut best: Option<(usize, u64)> = None;
        for (i, slot) in self.slots.iter().enumerate() {
            let Some(s) = slot else { continue };
            if s.pinned {
                continue;
            }
            match best {
                Some((_, seq)) if s.touch_seq >= seq => {}
                _ => best = Some((i, s.touch_seq)),
            }
        }
        best.map(|(i, _)| i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(tag: u8, len: usize) -> Vec<u8> {
        vec![tag; len]
    }

    /// lookup 命中/未命中;insert 返回槽号与 lookup 一致;数据逐字节取回。
    #[test]
    fn lookup_hit_miss() {
        let mut pool = PagePool::new(2);
        assert_eq!(pool.lookup(1, 0), None);
        let InsertOutcome::Inserted { slot, evicted } = pool.insert(1, 0, data(0xAA, 100), false)
        else {
            panic!("空池插入必成功")
        };
        assert_eq!(evicted, None);
        assert_eq!(pool.lookup(1, 0), Some(slot));
        assert_eq!(pool.slot_data(slot), data(0xAA, 100).as_slice());
        assert_eq!(pool.slot_key(slot), (1, 0));
        // 未命中:页号不符 / 资源号不符。
        assert_eq!(pool.lookup(1, 1), None);
        assert_eq!(pool.lookup(2, 0), None);
        assert_eq!(pool.resident_count(), 1);
        assert_eq!(pool.free_count(), 1);
    }

    /// LRU 驱逐序确定性:触帧刷新区位次序,逐槽扫描 + 单调时钟给出唯一
    /// 牺牲者。
    #[test]
    fn lru_eviction_order_deterministic() {
        let mut pool = PagePool::new(3);
        pool.insert(1, 0, data(1, 10), false);
        pool.insert(1, 1, data(2, 10), false);
        pool.insert(1, 2, data(3, 10), false);
        // 触帧 (1,0) → LRU 序:(1,1) < (1,2) < (1,0)。
        assert!(pool.touch(1, 0).is_some());
        let InsertOutcome::Inserted { evicted, .. } = pool.insert(1, 3, data(4, 10), false) else {
            panic!("有未钉住页必能腾槽")
        };
        assert_eq!(evicted, Some((1, 1)));
        assert_eq!(pool.lookup(1, 1), None);
        // 再插入 → 驱逐 (1,2);被触帧的 (1,0) 始终幸免。
        let InsertOutcome::Inserted { evicted, .. } = pool.insert(1, 4, data(5, 10), false) else {
            panic!("有未钉住页必能腾槽")
        };
        assert_eq!(evicted, Some((1, 2)));
        assert!(pool.lookup(1, 0).is_some());
        assert!(pool.lookup(1, 3).is_some());
        assert!(pool.lookup(1, 4).is_some());
        assert_eq!(pool.resident_count(), 3);
    }

    /// 重复 insert 同页 = 触帧刷新:不驱逐、不占新槽、驻留数据不被改写。
    #[test]
    fn reinsert_same_page_touches() {
        let mut pool = PagePool::new(2);
        pool.insert(1, 0, data(1, 10), false);
        pool.insert(1, 1, data(2, 10), false);
        let InsertOutcome::Inserted { evicted, .. } = pool.insert(1, 0, data(9, 10), false) else {
            panic!("同页重复插入必成功")
        };
        assert_eq!(evicted, None);
        assert_eq!(pool.resident_count(), 2);
        // (1,0) 已刷新 → 新页驱逐 (1,1)。
        let InsertOutcome::Inserted { evicted, .. } = pool.insert(1, 2, data(3, 10), false) else {
            panic!("有未钉住页必能腾槽")
        };
        assert_eq!(evicted, Some((1, 1)));
        // 驻留页内容自入池至驱逐不可变(重复 insert 的新数据被丢弃)。
        let slot = pool.lookup(1, 0).expect("驻留");
        assert_eq!(pool.slot_data(slot), data(1, 10).as_slice());
    }

    /// root 钉住在池满高压下永不驱逐;全槽钉住 → PoolFull(确定性失败,不
    /// 静默丢页)。
    #[test]
    fn pinned_never_evicted_under_pressure() {
        let mut pool = PagePool::new(2);
        pool.insert(7, 0, data(7, 10), true); // root 钉住
        pool.insert(7, 1, data(1, 10), false);
        // 高压:连续插入 5 页,每页挤走上一页,root 始终驻留且钉住位不变。
        for i in 2..7u32 {
            let InsertOutcome::Inserted { evicted, .. } =
                pool.insert(7, i, data(i as u8, 10), false)
            else {
                panic!("有未钉住页必能腾槽")
            };
            assert_eq!(evicted, Some((7, i - 1)));
            let root_slot = pool.lookup(7, 0).expect("root 常驻");
            assert!(pool.is_pinned(root_slot));
            assert_eq!(pool.slot_data(root_slot), data(7, 10).as_slice());
        }
        assert_eq!(pool.resident_count(), 2);
        // 全槽钉住:插入确定性失败。
        let mut full = PagePool::new(1);
        full.insert(7, 0, data(7, 10), true);
        assert_eq!(
            full.insert(7, 1, data(1, 10), false),
            InsertOutcome::PoolFull
        );
    }
}
