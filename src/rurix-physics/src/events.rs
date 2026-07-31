//! 接触事件归一化与有界 ring(纯函数/纯类型,后端无关;§4.A5 C-2 评审修订)。
//!
//! Jolt `ContactListener` 回调多线程触发、顺序非确定——库内在 **step 结束边界
//! 归一化**:按 `(min(a,b), max(a,b), phase)` 规范序排序去重后入 ring;事件序列
//! 确定性 = 归一化后序列语义。ring 满 → 确定性丢弃最旧(归一化序列上定义)+
//! `StepStats.contacts_dropped` 计数上报(不 panic,P-01)。

use std::collections::VecDeque;

use crate::types::{ContactEvent, ContactPhase};

/// 规范序键:`(min(a,b), max(a,b), phase)`(§4.A5 字面;BodyId 序 = u64 位序)。
fn canonical_key(e: &ContactEvent) -> (crate::id::BodyId, crate::id::BodyId, ContactPhase) {
    let (lo, hi) = if e.a <= e.b { (e.a, e.b) } else { (e.b, e.a) };
    (lo, hi, e.phase)
}

/// 全序比较:规范序键优先,载荷(点/法线/冲量,位级 total order)决胜——
/// 使「同键重复保留首条」的「首条」本身确定。
fn total_cmp(a: &ContactEvent, b: &ContactEvent) -> std::cmp::Ordering {
    canonical_key(a)
        .cmp(&canonical_key(b))
        .then_with(|| cmp_arr(&a.contact_point, &b.contact_point))
        .then_with(|| cmp_arr(&a.normal, &b.normal))
        .then_with(|| a.impulse.total_cmp(&b.impulse))
}

fn cmp_arr(a: &[f32; 3], b: &[f32; 3]) -> std::cmp::Ordering {
    a[0].total_cmp(&b[0])
        .then_with(|| a[1].total_cmp(&b[1]))
        .then_with(|| a[2].total_cmp(&b[2]))
}

/// step 结束边界归一化:规范序排序 + 同 `(min,max,phase)` 去重(保留全序首条)。
pub(crate) fn normalize_contacts(mut events: Vec<ContactEvent>) -> Vec<ContactEvent> {
    events.sort_by(total_cmp);
    events.dedup_by(|next, kept| canonical_key(next) == canonical_key(kept));
    events
}

/// 有界接触事件 ring(容量随 `WorldDesc::contact_capacity`;0 = 全丢)。
#[derive(Debug, Default)]
pub(crate) struct ContactRing {
    buf: VecDeque<ContactEvent>,
    capacity: usize,
}

impl ContactRing {
    pub(crate) fn with_capacity(capacity: u32) -> Self {
        ContactRing {
            buf: VecDeque::with_capacity(capacity.min(1 << 20) as usize),
            capacity: capacity as usize,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.buf.len()
    }

    /// 推入**已归一化**序列;ring 满逐条丢最旧,返回本批丢弃计数。
    /// 容量 0 = 全丢(合法配置:本帧不消费事件,§4.A5 溢出语义恒确定)。
    pub(crate) fn push_normalized(&mut self, events: Vec<ContactEvent>) -> u32 {
        let mut dropped = 0u32;
        for ev in events {
            if self.capacity == 0 {
                dropped = dropped.saturating_add(1);
                continue;
            }
            if self.buf.len() >= self.capacity {
                self.buf.pop_front();
                dropped = dropped.saturating_add(1);
            }
            self.buf.push_back(ev);
        }
        dropped
    }

    /// 取走前 `n` 条(`n ≤ len`,由 budget 实发额决定,§4.A6 截断语义)。
    pub(crate) fn drain_n(
        &mut self,
        n: usize,
    ) -> std::collections::vec_deque::Drain<'_, ContactEvent> {
        debug_assert!(n <= self.buf.len());
        self.buf.drain(..n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::BodyId;

    fn ev(a: u64, b: u64, phase: ContactPhase, impulse: f32) -> ContactEvent {
        ContactEvent {
            a: BodyId::new(a as u32, 1),
            b: BodyId::new(b as u32, 1),
            phase,
            contact_point: [impulse, 0.0, 0.0],
            normal: [0.0, 1.0, 0.0],
            impulse,
        }
    }

    #[test]
    fn normalize_sorts_canonical_order() {
        let out = normalize_contacts(vec![
            ev(3, 1, ContactPhase::Begin, 1.0), // a/b 逆序 → 归一为 (1,3)
            ev(1, 2, ContactPhase::End, 1.0),
            ev(1, 2, ContactPhase::Begin, 1.0),
            ev(2, 3, ContactPhase::Begin, 1.0),
        ]);
        let keys: Vec<_> = out.iter().map(canonical_key).collect();
        let id = |i: u32| BodyId::new(i, 1);
        assert_eq!(
            keys,
            vec![
                (id(1), id(2), ContactPhase::Begin),
                (id(1), id(2), ContactPhase::End),
                (id(1), id(3), ContactPhase::Begin),
                (id(2), id(3), ContactPhase::Begin),
            ]
        );
    }

    #[test]
    fn normalize_dedups_same_pair_phase() {
        // 同 (min,max,phase) 两条(回调重复)→ 去重保留全序首条(impulse 小者)。
        let out = normalize_contacts(vec![
            ev(1, 2, ContactPhase::Persist, 9.0),
            ev(2, 1, ContactPhase::Persist, 3.0),
        ]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].impulse, 3.0);
        // 不同 phase 不去重。
        let out = normalize_contacts(vec![
            ev(1, 2, ContactPhase::Begin, 1.0),
            ev(2, 1, ContactPhase::End, 1.0),
        ]);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn ring_overflow_drops_oldest_and_counts() {
        let mut ring = ContactRing::with_capacity(2);
        assert_eq!(
            ring.push_normalized(vec![ev(1, 2, ContactPhase::Begin, 1.0)]),
            0
        );
        assert_eq!(
            ring.push_normalized(vec![ev(1, 3, ContactPhase::Begin, 2.0)]),
            0
        );
        // 满后再入 2 条 → 确定性丢最旧 2 条。
        assert_eq!(
            ring.push_normalized(vec![
                ev(1, 4, ContactPhase::Begin, 3.0),
                ev(1, 5, ContactPhase::Begin, 4.0),
            ]),
            2
        );
        let n = ring.len();
        let rest: Vec<_> = ring.drain_n(n).collect();
        assert_eq!(rest.len(), 2);
        assert_eq!(rest[0].impulse, 3.0);
        assert_eq!(rest[1].impulse, 4.0);
    }

    #[test]
    fn zero_capacity_ring_drops_everything() {
        let mut ring = ContactRing::with_capacity(0);
        assert_eq!(
            ring.push_normalized(vec![ev(1, 2, ContactPhase::Begin, 1.0)]),
            1
        );
        assert_eq!(ring.len(), 0);
    }

    #[test]
    fn drain_n_truncates() {
        let mut ring = ContactRing::with_capacity(4);
        ring.push_normalized(vec![
            ev(1, 2, ContactPhase::Begin, 1.0),
            ev(1, 3, ContactPhase::Begin, 2.0),
            ev(1, 4, ContactPhase::Begin, 3.0),
        ]);
        let first: Vec<_> = ring.drain_n(2).collect();
        assert_eq!(first.len(), 2);
        assert_eq!(ring.len(), 1);
    }
}
