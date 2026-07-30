//! transient 池:区间图贪心着色别名 + 峰值审计(报告5 §2.3/§5;RFC-0016 章 A)。
//!
//! 语义(RDG 照搬):create 只分配描述符,执行前才落物理分配;生命周期不相交
//! ([`LifeInterval::overlaps`] 为 false)的资源别名共享同一物理槽。imported 资源
//! 永不入池(报告5 §2.3 约束一,compile 趟2 已过滤)。
//!
//! 峰值审计(CI 对照口径):
//! - [`TransientPool::high_water`] = 别名后物理池峰值 = 全部槽位尺寸和;
//! - [`TransientPool::no_alias_peak`] = 无别名对照 = 逐 transient 独占分配的尺寸和。
//!   别名有效的非平凡图必有 high_water < no_alias_peak(注入单测锚定)。

use std::collections::BTreeMap;

use crate::graph::resources::pool_bucket;
use crate::graph::types::{LifeInterval, PassId, PoolSlot, ResourceId, ResourceKind};

/// transient 池别名结果(编译产物一部分;只读审计面)。
#[derive(Debug, Clone, Default)]
pub struct TransientPool {
    slots: BTreeMap<ResourceId, PoolSlot>,
    slot_count: u32,
    high_water: u64,
    no_alias_peak: u64,
}

impl TransientPool {
    /// 贪心区间着色:逐桶按 `(first_use, ResourceId)` 排序(确定性),复用
    /// `last_use < 本 first_use` 的首个槽(区间不相交即可共享),无则开新槽;
    /// 槽尺寸 = 共享者最大 `byte_size`。
    pub(crate) fn build(entries: &[(ResourceId, ResourceKind, LifeInterval)]) -> TransientPool {
        let mut by_bucket: BTreeMap<u32, Vec<(ResourceId, u64, LifeInterval)>> = BTreeMap::new();
        let mut no_alias_peak = 0u64;
        for &(id, kind, iv) in entries {
            by_bucket
                .entry(pool_bucket(&kind))
                .or_default()
                .push((id, kind.byte_size(), iv));
            no_alias_peak += kind.byte_size();
        }

        let mut slots = BTreeMap::new();
        let mut slot_count = 0u32;
        let mut high_water = 0u64;
        for (bucket, mut members) in by_bucket {
            members.sort_by_key(|&(id, _, iv)| (iv.first_use, id));
            // 槽车道:(末用, 尺寸);复用判据 = 车道末用严格早于本首用(区间不相交)。
            let mut lanes: Vec<(PassId, u64)> = Vec::new();
            let mut assign: Vec<(ResourceId, u32)> = Vec::with_capacity(members.len());
            for (id, size, iv) in members {
                let lane = match lanes.iter().position(|&(last, _)| last < iv.first_use) {
                    Some(i) => {
                        lanes[i].0 = iv.last_use;
                        lanes[i].1 = lanes[i].1.max(size);
                        i
                    }
                    None => {
                        lanes.push((iv.last_use, size));
                        lanes.len() - 1
                    }
                };
                assign.push((id, u32::try_from(lane).unwrap_or(u32::MAX)));
            }
            for (id, lane) in assign {
                let size = lanes[lane as usize].1;
                slots.insert(
                    id,
                    PoolSlot {
                        bucket,
                        slot: lane,
                        size,
                    },
                );
            }
            slot_count += u32::try_from(lanes.len()).unwrap_or(u32::MAX);
            high_water += lanes.iter().map(|&(_, size)| size).sum::<u64>();
        }

        TransientPool {
            slots,
            slot_count,
            high_water,
            no_alias_peak,
        }
    }

    /// 某 transient 的池槽位(未入池/imported → None)。
    #[must_use]
    pub fn slot_of(&self, res: ResourceId) -> Option<PoolSlot> {
        self.slots.get(&res).copied()
    }

    /// 物理槽总数(跨桶)。
    #[must_use]
    pub fn slot_count(&self) -> u32 {
        self.slot_count
    }

    /// 别名后物理池峰值字节数(全部槽位尺寸和)。
    #[must_use]
    pub fn high_water(&self) -> u64 {
        self.high_water
    }

    /// 无别名对照峰值字节数(逐 transient 独占分配的尺寸和)。
    #[must_use]
    pub fn no_alias_peak(&self) -> u64 {
        self.no_alias_peak
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::types::TextureFormat;

    const MB: u64 = 1024 * 1024;

    fn tex_1mb() -> ResourceKind {
        ResourceKind::Texture2d {
            width: 512,
            height: 512,
            format: TextureFormat::Rgba8Unorm,
            mip_levels: 1,
        }
    }

    fn iv(first: u32, last: u32) -> LifeInterval {
        LifeInterval {
            first_use: PassId(first),
            last_use: PassId(last),
        }
    }

    /// 非平凡别名:a[0,1] / b[1,2] / c[2,3]——a、c 区间不相交共享槽,b 另槽;
    /// 别名后峰值(2 槽 × 1MB)严格小于无别名峰值(3 × 1MB)。
    #[test]
    fn alias_peak_below_no_alias_peak() {
        let pool = TransientPool::build(&[
            (ResourceId(0), tex_1mb(), iv(0, 1)),
            (ResourceId(1), tex_1mb(), iv(1, 2)),
            (ResourceId(2), tex_1mb(), iv(2, 3)),
        ]);
        let sa = pool.slot_of(ResourceId(0)).expect("a 入池");
        let sb = pool.slot_of(ResourceId(1)).expect("b 入池");
        let sc = pool.slot_of(ResourceId(2)).expect("c 入池");
        assert_eq!(sa, sc, "a/c 区间不相交应共享槽");
        assert_ne!(
            (sa.bucket, sa.slot),
            (sb.bucket, sb.slot),
            "a/b 区间相交互斥"
        );
        assert_eq!(pool.slot_count(), 2);
        assert_eq!(pool.high_water(), 2 * MB);
        assert_eq!(pool.no_alias_peak(), 3 * MB);
        assert!(pool.high_water() < pool.no_alias_peak());
    }

    /// 端点相接即相交(闭区间语义):[0,1] 与 [1,2] 不可别名。
    #[test]
    fn touching_intervals_do_not_alias() {
        let pool = TransientPool::build(&[
            (ResourceId(0), tex_1mb(), iv(0, 1)),
            (ResourceId(1), tex_1mb(), iv(1, 2)),
        ]);
        assert_ne!(pool.slot_of(ResourceId(0)), pool.slot_of(ResourceId(1)));
        assert_eq!(pool.high_water(), 2 * MB);
    }

    /// 槽尺寸取共享者最大值(同桶异尺寸别名)。
    #[test]
    fn slot_size_takes_max_of_sharers() {
        let small = ResourceKind::Buffer { size: 700_000 };
        let big = ResourceKind::Buffer { size: 900_000 };
        let pool = TransientPool::build(&[
            (ResourceId(0), small, iv(0, 0)),
            (ResourceId(1), big, iv(1, 1)),
        ]);
        let s = pool.slot_of(ResourceId(1)).expect("big 入池");
        assert_eq!(s.size, 900_000);
        assert_eq!(pool.slot_of(ResourceId(0)), Some(s), "同桶不相交应共享");
        assert_eq!(pool.high_water(), 900_000);
    }

    /// 着色确定性:输入序打乱,槽位映射逐字节相同。
    #[test]
    fn coloring_is_deterministic() {
        let forward = TransientPool::build(&[
            (ResourceId(0), tex_1mb(), iv(0, 1)),
            (ResourceId(1), tex_1mb(), iv(1, 2)),
            (ResourceId(2), tex_1mb(), iv(2, 3)),
        ]);
        let shuffled = TransientPool::build(&[
            (ResourceId(2), tex_1mb(), iv(2, 3)),
            (ResourceId(0), tex_1mb(), iv(0, 1)),
            (ResourceId(1), tex_1mb(), iv(1, 2)),
        ]);
        for id in [ResourceId(0), ResourceId(1), ResourceId(2)] {
            assert_eq!(forward.slot_of(id), shuffled.slot_of(id));
        }
        assert_eq!(forward.high_water(), shuffled.high_water());
    }
}
