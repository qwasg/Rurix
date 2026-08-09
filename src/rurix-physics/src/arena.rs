//! generation arena(纯类型,后端无关,§4.A2 generation 纪律的执行体)。
//!
//! - `alloc`:优先复用空闲槽位(复用时 generation +1,单调递增),否则顺发新
//!   index;index 池耗尽(`capacity` 封顶且无空闲槽)→ `Err(PoolExhausted)`。
//! - `remove`:generation 已达 `u32::MAX` 的槽位**退休不再分配**(回绕复活路径
//!   类型面消灭,I-6);其余槽位回空闲表。
//! - 失效句柄(未创建/已移除/generation 失配)`get`/`remove` 一律 `None`,由
//!   调用方映射为确定性 `Err(InvalidBody)`(§4.C3 不悬垂)。

use crate::error::PhysicsError;

/// 单个槽位:`generation` 初始为 0,首次 `alloc` 后从 1 起单调递增。
#[derive(Debug)]
struct Slot<T> {
    generation: u32,
    value: Option<T>,
}

/// 定容 generation arena(`T` 为槽位负载;body/shape 各持一个实例)。
#[derive(Debug)]
pub(crate) struct GenArena<T> {
    slots: Vec<Slot<T>>,
    /// 空闲可复用 index(LIFO;同操作序列下顺序确定)。
    free: Vec<u32>,
    capacity: u32,
    retired: u32,
}

impl<T> GenArena<T> {
    pub(crate) fn with_capacity(capacity: u32) -> Self {
        GenArena {
            slots: Vec::new(),
            free: Vec::new(),
            capacity,
            retired: 0,
        }
    }

    /// 存活占用数。
    pub(crate) fn live(&self) -> u32 {
        self.slots.len() as u32 - self.free.len() as u32 - self.retired
    }

    /// 距 `PoolExhausted` 还可分配的槽位数(空闲 + 未顺发)。
    pub(crate) fn remaining_capacity(&self) -> u32 {
        self.capacity - self.live() - self.retired
    }

    /// 分配一个槽位,返回 `(index, generation)`;池耗尽 → `Err(PoolExhausted)`。
    pub(crate) fn alloc(&mut self, value: T) -> Result<(u32, u32), PhysicsError> {
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            debug_assert!(slot.value.is_none());
            // 不回绕:generation 达 u32::MAX 的槽位已在 remove 时退休,不会进 free。
            slot.generation += 1;
            slot.value = Some(value);
            return Ok((index, slot.generation));
        }
        if self.slots.len() as u32 >= self.capacity {
            return Err(PhysicsError::PoolExhausted);
        }
        let index = self.slots.len() as u32;
        self.slots.push(Slot {
            generation: 1,
            value: Some(value),
        });
        Ok((index, 1))
    }

    /// 按句柄部件取负载;失效句柄 → `None`。
    pub(crate) fn get(&self, index: u32, generation: u32) -> Option<&T> {
        let slot = self.slots.get(index as usize)?;
        if slot.generation != generation {
            return None;
        }
        slot.value.as_ref()
    }

    /// 按句柄部件取可变负载(add 批回填 token 用);失效句柄 → `None`。
    pub(crate) fn get_mut(&mut self, index: u32, generation: u32) -> Option<&mut T> {
        let slot = self.slots.get_mut(index as usize)?;
        if slot.generation != generation {
            return None;
        }
        slot.value.as_mut()
    }

    /// 遍历存活槽位`(index, generation, &T)`(槽位序 = 确定性面)。
    pub(crate) fn iter_live(&self) -> impl Iterator<Item = (u32, u32, &T)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| slot.value.as_ref().map(|v| (i as u32, slot.generation, v)))
    }

    /// 移除并取回负载;失效句柄 → `None`。generation 达 `u32::MAX` 的槽位退休。
    pub(crate) fn remove(&mut self, index: u32, generation: u32) -> Option<T> {
        let slot = self.slots.get_mut(index as usize)?;
        if slot.generation != generation {
            return None;
        }
        let value = slot.value.take()?;
        if slot.generation == u32::MAX {
            self.retired += 1;
        } else {
            self.free.push(index);
        }
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_monotonic_on_slot_reuse() {
        let mut arena: GenArena<u32> = GenArena::with_capacity(4);
        let (i1, g1) = arena.alloc(10).unwrap();
        assert_eq!((i1, g1), (0, 1));
        assert_eq!(arena.remove(i1, g1), Some(10));
        // 槽位复用:同 index,generation 单调递增。
        let (i2, g2) = arena.alloc(20).unwrap();
        assert_eq!(i2, i1);
        assert_eq!(g2, g1 + 1);
        // 旧句柄失效,新句柄可用。
        assert_eq!(arena.get(i1, g1), None);
        assert_eq!(arena.get(i2, g2), Some(&20));
    }

    #[test]
    fn slot_retires_at_max_generation() {
        let mut arena: GenArena<u32> = GenArena::with_capacity(1);
        let (index, _) = arena.alloc(1).unwrap();
        // 直接把槽位 generation 推到 32b 上限(等价于长期复用后的终态)。
        arena.slots[index as usize].generation = u32::MAX;
        assert_eq!(arena.remove(index, u32::MAX), Some(1));
        // 槽位退休不再分配:capacity=1 且无空闲槽 → PoolExhausted。
        assert_eq!(arena.alloc(2), Err(PhysicsError::PoolExhausted));
        assert_eq!(arena.remaining_capacity(), 0);
    }

    #[test]
    fn pool_exhausted_when_index_pool_full() {
        let mut arena: GenArena<u32> = GenArena::with_capacity(2);
        arena.alloc(1).unwrap();
        arena.alloc(2).unwrap();
        assert_eq!(arena.alloc(3), Err(PhysicsError::PoolExhausted));
        // 移除一个后恢复可分配。
        let (index, generation) = (0, 1);
        assert_eq!(arena.remove(index, generation), Some(1));
        assert!(arena.alloc(3).is_ok());
    }

    #[test]
    fn stale_handle_rejected_after_remove() {
        let mut arena: GenArena<u32> = GenArena::with_capacity(2);
        let (index, generation) = arena.alloc(7).unwrap();
        assert_eq!(arena.remove(index, generation), Some(7));
        // 二次使用(重复移除 / 读)→ None(调用方映射 Err(InvalidBody),§4.C3)。
        assert_eq!(arena.remove(index, generation), None);
        assert_eq!(arena.get(index, generation), None);
        // 越界 index / 错位 generation 同样拒绝。
        assert_eq!(arena.get(99, 1), None);
        assert_eq!(arena.get(index, generation + 1), None);
    }
}
