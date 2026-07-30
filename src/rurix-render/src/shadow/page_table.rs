//! 32 位页表项与页表(报告3 §2.1 机制二;RFC-0016 §4.D1,StratusGFX SVSM 参照)。
//!
//! 每方向光每 clipmap 级一张 128×128 页表(R32Uint 纹理/缓冲,KB 级,常驻跨帧;
//! 报告3 §5.2 缓冲清单)。位布局属 P0 冻结的「阶段不变量」(报告3 §4),本文件
//! 单测锁定,任何改动必须同步 device 侧解码(W3 接线)。
//!
//! 位分配(LSB→MSB):
//! - `[ 0..16)` 物理页索引 u16(共享物理页池下标;`0xFFFF` = [`PHYS_NONE`] 哨兵);
//! - `[16]`     驻留位:1 = 已分配物理页;
//! - `[17]`     脏位:1 = 内容失效,须重光栅(失效三源,报告3 §5.3);
//! - `[18..26)` 帧龄 u8(距上次被屏幕标记的帧数,饱和计数;LRU 驱逐键,
//!   本帧标记 = 0 且不可驱逐——帧龄延迟纪律,报告3 §6);
//! - `[26..32)` 保留 6 位,恒 0(W3 device 解码同步前不得启用)。

use crate::shadow::clipmap::PAGE_TABLE_DIM;

/// 物理页索引哨兵:未驻留。
pub const PHYS_NONE: u16 = u16::MAX;

const PHYS_MASK: u32 = 0xFFFF;
const RESIDENT_BIT: u32 = 1 << 16;
const DIRTY_BIT: u32 = 1 << 17;
const AGE_SHIFT: u32 = 18;
const AGE_MASK: u32 = 0xFF;
/// 帧龄饱和上限(8 位)。
pub const AGE_MAX: u8 = 0xFF;

/// 全局页标识:clipmap 级 + 窗口内页表槽位(toroidal 槽位,非世界页坐标;
/// 槽位↔世界页坐标换算见 [`crate::shadow::clipmap`])。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PageId {
    pub level: u8,
    pub x: u8,
    pub y: u8,
}

/// 32 位页表项(逻辑视图;位打包语义见模块文档)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageTableEntry {
    /// 物理页索引(共享池下标);未驻留时恒 [`PHYS_NONE`]。
    pub phys: u16,
    /// 驻留位。
    pub resident: bool,
    /// 脏位(内容失效,须重光栅后才可被采样信任)。
    pub dirty: bool,
    /// 帧龄:距上次屏幕标记的帧数(0 = 本帧标记,不可驱逐)。
    pub age: u8,
}

impl PageTableEntry {
    /// 空项(未驻留/净/龄 0)。
    pub const EMPTY: PageTableEntry = PageTableEntry {
        phys: PHYS_NONE,
        resident: false,
        dirty: false,
        age: 0,
    };

    /// 打包为 32 位页表项(保留位恒 0)。
    pub fn pack(self) -> u32 {
        debug_assert!(self.resident || self.phys == PHYS_NONE);
        let mut v = u32::from(self.phys) & PHYS_MASK;
        if self.resident {
            v |= RESIDENT_BIT;
        }
        if self.dirty {
            v |= DIRTY_BIT;
        }
        v | (u32::from(self.age) << AGE_SHIFT)
    }

    /// 解包 32 位页表项(保留位非零 = 数据腐败,debug 断言)。
    pub fn unpack(v: u32) -> Self {
        debug_assert_eq!(v >> 26, 0, "保留位 [26..32) 必须恒 0");
        Self {
            phys: (v & PHYS_MASK) as u16,
            resident: v & RESIDENT_BIT != 0,
            dirty: v & DIRTY_BIT != 0,
            age: ((v >> AGE_SHIFT) & AGE_MASK) as u8,
        }
    }
}

/// 单级页表:128×128 项,行主序 `entries[y*128+x]`,常驻跨帧。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageTable {
    pub entries: Vec<u32>,
}

impl PageTable {
    /// 全空页表。
    pub fn new() -> Self {
        Self {
            entries: vec![PageTableEntry::EMPTY.pack(); (PAGE_TABLE_DIM * PAGE_TABLE_DIM) as usize],
        }
    }

    fn index(x: u8, y: u8) -> usize {
        (usize::from(y) * PAGE_TABLE_DIM as usize) + usize::from(x)
    }

    /// 读槽位 (x, y)。
    pub fn get(&self, x: u8, y: u8) -> PageTableEntry {
        PageTableEntry::unpack(self.entries[Self::index(x, y)])
    }

    /// 写槽位 (x, y)。
    pub fn set(&mut self, x: u8, y: u8, e: PageTableEntry) {
        let i = Self::index(x, y);
        self.entries[i] = e.pack();
    }

    /// 脏项计数(单测/度量埋点)。
    pub fn dirty_count(&self) -> u32 {
        self.entries.iter().filter(|&&v| v & DIRTY_BIT != 0).count() as u32
    }
}

impl Default for PageTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_pack_unpack_roundtrip() {
        // 全字段组合往返(龄取边界与中间值)
        for &phys in &[0u16, 1, 4096, 65534, PHYS_NONE] {
            for &resident in &[false, true] {
                for &dirty in &[false, true] {
                    for &age in &[0u8, 1, 127, AGE_MAX] {
                        let e = PageTableEntry {
                            phys: if resident { phys } else { PHYS_NONE },
                            resident,
                            dirty,
                            age,
                        };
                        assert_eq!(PageTableEntry::unpack(e.pack()), e);
                    }
                }
            }
        }
    }

    #[test]
    fn entry_layout_locked() {
        // 布局锁定(P0 阶段不变量):精确 u32 值锚定
        let e = PageTableEntry {
            phys: 0x1234,
            resident: true,
            dirty: true,
            age: 0x5A,
        };
        // phys[0..16)=0x1234 | resident bit16 | dirty bit17 | age 0x5A << 18
        assert_eq!(e.pack(), 0x1234 | (1 << 16) | (1 << 17) | (0x5A << 18));
        assert_eq!(e.pack(), 0x016B_1234);
        // 空项 = phys 全 1,其余全 0
        assert_eq!(PageTableEntry::EMPTY.pack(), 0x0000_FFFF);
        // 最小驻留项:phys 0 + resident,脏/龄 0
        let min_resident = PageTableEntry {
            phys: 0,
            resident: true,
            dirty: false,
            age: 0,
        };
        assert_eq!(min_resident.pack(), 0x0001_0000);
        // 龄最大:age 0xFF << 18 = 0x03FC_0000
        let aged = PageTableEntry {
            phys: 7,
            resident: true,
            dirty: false,
            age: AGE_MAX,
        };
        assert_eq!(aged.pack(), 0x03FD_0007);
        // 位段互不串扰:unpack 后保留位语义
        let u = PageTableEntry::unpack(0x016B_1234);
        assert_eq!(u, e);
    }

    #[test]
    fn table_slot_index_and_dirty_count() {
        let mut t = PageTable::new();
        assert_eq!(t.entries.len(), 128 * 128);
        assert_eq!(t.dirty_count(), 0);
        assert_eq!(t.get(0, 0), PageTableEntry::EMPTY);
        assert_eq!(t.get(127, 127), PageTableEntry::EMPTY);
        // 行列主序锁定:slot (1,0) 是 entries[1],(0,1) 是 entries[128]
        let dirty = PageTableEntry {
            dirty: true,
            ..PageTableEntry::EMPTY
        };
        t.set(1, 0, dirty);
        t.set(0, 1, dirty);
        assert!(t.entries[1] & (1 << 17) != 0);
        assert!(t.entries[128] & (1 << 17) != 0);
        assert_eq!(t.dirty_count(), 2);
        assert!(t.get(1, 0).dirty);
        assert!(t.get(0, 1).dirty);
        assert!(!t.get(0, 0).dirty);
    }
}
