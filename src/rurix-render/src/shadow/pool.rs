//! 共享物理页池(报告3 §2.1 机制二;RFC-0016 §4.D3)。
//!
//! 固定预算 N 页、跨全部 clipmap 级共享的扁平深度存储(128² f32/页);
//! **非 sparse binding**——共享池起步驱动行为更一致(StratusGFX 双后端经验,
//! RFC-0016 §7 否决项 5)。页池为跨帧外部资源(render graph imported 语义,
//! RFC-0016 §4.0-3),分配/驱逐策略在 [`crate::shadow::vsm`] 的 page_alloc。

use crate::shadow::clipmap::PAGE_TEXELS;
use crate::shadow::page_table::{PHYS_NONE, PageId};

/// 单页纹素数 128²。
pub const PAGE_FLOATS: usize = (PAGE_TEXELS * PAGE_TEXELS) as usize;

/// 共享物理页池:固定预算,扁平 `Vec<f32>` 存储(等价 `Vec<[f32; 128*128]>`)。
#[derive(Debug, Clone, PartialEq)]
pub struct PhysicalPagePool {
    /// 预算(页数)。
    pub budget: u16,
    /// 扁平深度存储,页 `i` 占 `data[i*PAGE_FLOATS .. (i+1)*PAGE_FLOATS]`。
    data: Vec<f32>,
    /// 空闲物理页索引栈(弹出序 = 逆回收序,确定性)。
    free: Vec<u16>,
    /// 物理页 → 占用者(空闲为 None)。
    owner: Vec<Option<PageId>>,
}

impl PhysicalPagePool {
    /// 新建池:全部页空闲。
    pub fn new(budget: u16) -> Self {
        assert!(budget >= 1, "页池预算必须 ≥1");
        assert!(budget < PHYS_NONE, "预算不得触碰 PHYS_NONE 哨兵");
        Self {
            budget,
            data: vec![1.0; usize::from(budget) * PAGE_FLOATS],
            free: (0..budget).rev().collect(),
            owner: vec![None; usize::from(budget)],
        }
    }

    /// 空闲页数(池水位度量埋点,报告3 §6 页池颠簸画像)。
    pub fn free_count(&self) -> u32 {
        self.free.len() as u32
    }

    /// 占用中页数。
    pub fn used_count(&self) -> u32 {
        self.budget as u32 - self.free_count()
    }

    /// 从空闲栈取一页给 `page`;池满返回 None(驱逐决策归 page_alloc)。
    pub fn alloc(&mut self, page: PageId) -> Option<u16> {
        let phys = self.free.pop()?;
        self.owner[usize::from(phys)] = Some(page);
        Some(phys)
    }

    /// 回收物理页(原占用者失效由调用方在页表侧清除)。
    pub fn free_page(&mut self, phys: u16) {
        assert!(phys < self.budget, "物理页索引越界");
        self.owner[usize::from(phys)] = None;
        self.free.push(phys);
    }

    /// 物理页占用者。
    pub fn owner(&self, phys: u16) -> Option<PageId> {
        self.owner[usize::from(phys)]
    }

    /// 物理页深度数据(只读)。
    pub fn page(&self, phys: u16) -> &[f32] {
        let b = usize::from(phys) * PAGE_FLOATS;
        &self.data[b..b + PAGE_FLOATS]
    }

    /// 物理页深度数据(可写;深度光栅专用)。
    pub fn page_mut(&mut self, phys: u16) -> &mut [f32] {
        let b = usize::from(phys) * PAGE_FLOATS;
        &mut self.data[b..b + PAGE_FLOATS]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_free_reuse_deterministic() {
        let mut pool = PhysicalPagePool::new(2);
        assert_eq!(PAGE_FLOATS, 128 * 128);
        assert_eq!(pool.free_count(), 2);
        let p0 = PageId {
            level: 0,
            x: 1,
            y: 2,
        };
        let p1 = PageId {
            level: 1,
            x: 3,
            y: 4,
        };
        // 初始空闲栈 = [1, 0](rev 构造),先弹 0 后弹 1
        let a = pool.alloc(p0).expect("有空闲");
        let b = pool.alloc(p1).expect("有空闲");
        assert_eq!((a, b), (0, 1));
        assert_eq!(pool.free_count(), 0);
        assert_eq!(pool.used_count(), 2);
        assert_eq!(pool.owner(a), Some(p0));
        assert_eq!(pool.owner(b), Some(p1));
        // 池满
        assert_eq!(pool.alloc(p0), None);
        // 回收 0 → 再分配复用 0
        pool.free_page(a);
        assert_eq!(pool.owner(a), None);
        let c = pool.alloc(p1).expect("回收后有空闲");
        assert_eq!(c, 0);
    }

    #[test]
    fn page_storage_isolated_and_far_init() {
        let mut pool = PhysicalPagePool::new(2);
        // 初始深度 = 1.0(远平面,无遮挡)
        assert!(pool.page(0).iter().all(|&v| v == 1.0));
        let p0 = PageId {
            level: 0,
            x: 0,
            y: 0,
        };
        let a = pool.alloc(p0).expect("有空闲");
        let b = pool.alloc(p0).expect("有空闲");
        pool.page_mut(a)[0] = 0.25;
        pool.page_mut(a)[PAGE_FLOATS - 1] = 0.75;
        // 页间隔离
        assert!(pool.page(b).iter().all(|&v| v == 1.0));
        assert!((pool.page(a)[0] - 0.25).abs() < 1e-7);
        assert!((pool.page(a)[PAGE_FLOATS - 1] - 0.75).abs() < 1e-7);
    }
}
