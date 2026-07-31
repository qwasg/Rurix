//! 同步预算(§4.A6,冻结接口):宿主**每帧构造(重置)**,bridge/query/drain
//! 共享消耗;三轴字段即剩余额度,消耗 = 饱和递减,余量归零即停(对应面确定性
//! 截断),饱和计数由消费方上报(P-01 不 panic)。
//!
//! 并发纪律(§4.A4):cast 查询 step 外多线程并发,每线程持自己的 `SyncBudget`
//! (`&mut` 不跨线程共享);饱和计数汇总在 `PhysicsWorld` 原子计数器。

/// 每帧同步预算(防物理→渲染写爆,R-G6-4 竞态预算的一部分)。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SyncBudget {
    /// body 变换写剩余额度(消费方 = 同步桥,G6.3;`try_consume_body_write`)。
    pub max_body_writes: u32,
    /// 接触事件剩余额度(消费方 = `drain_contacts`)。
    pub max_contact_events: u32,
    /// 查询 cast 剩余额度(消费方 = `cast_ray`/`cast_shape`/`overlap`,每次 1)。
    pub max_query_casts: u32,
}

impl SyncBudget {
    /// 构造满额预算(宿主每帧调用 = 重置语义)。
    pub fn new(max_body_writes: u32, max_contact_events: u32, max_query_casts: u32) -> Self {
        SyncBudget {
            max_body_writes,
            max_contact_events,
            max_query_casts,
        }
    }

    /// 消耗 1 次 body 写额度;耗尽 → `false`(调用方确定性截断 + 计数)。
    pub fn try_consume_body_write(&mut self) -> bool {
        consume(&mut self.max_body_writes)
    }

    /// 消耗 1 次接触事件额度;耗尽 → `false`。
    pub fn try_consume_contact_event(&mut self) -> bool {
        consume(&mut self.max_contact_events)
    }

    /// 消耗 1 次查询 cast 额度;耗尽 → `false`。
    pub fn try_consume_query_cast(&mut self) -> bool {
        consume(&mut self.max_query_casts)
    }

    /// 批量申请接触事件额度,返回实发数 = `min(剩余, requested)`(drain 截断语义)。
    pub fn consume_contact_events(&mut self, requested: u32) -> u32 {
        let granted = self.max_contact_events.min(requested);
        self.max_contact_events -= granted;
        granted
    }
}

fn consume(credit: &mut u32) -> bool {
    if *credit == 0 {
        return false;
    }
    *credit -= 1;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_budget_grants_then_saturates_per_axis() {
        let mut b = SyncBudget::new(2, 1, 3);
        // 三轴独立:body_writes 2 次后饱和,不影响其余两轴。
        assert!(b.try_consume_body_write());
        assert!(b.try_consume_body_write());
        assert!(!b.try_consume_body_write());
        assert!(
            !b.try_consume_body_write(),
            "余量归零即停,恒 false 不 panic"
        );
        assert!(b.try_consume_contact_event());
        assert!(!b.try_consume_contact_event());
        assert!(b.try_consume_query_cast());
        assert!(b.try_consume_query_cast());
        assert!(b.try_consume_query_cast());
        assert!(!b.try_consume_query_cast());
        assert_eq!(b, SyncBudget::new(0, 0, 0));
    }

    #[test]
    fn consume_contact_events_grants_min() {
        let mut b = SyncBudget::new(0, 3, 0);
        assert_eq!(b.consume_contact_events(5), 3);
        assert_eq!(b.max_contact_events, 0);
        assert_eq!(b.consume_contact_events(1), 0);
    }

    #[test]
    fn reset_is_fresh_construction() {
        // 重置 = 宿主每帧重新构造(§4.A6),旧实例的消耗不携带。
        let mut frame1 = SyncBudget::new(1, 1, 1);
        assert!(frame1.try_consume_query_cast());
        assert!(!frame1.try_consume_query_cast());
        let mut frame2 = SyncBudget::new(1, 1, 1);
        assert!(frame2.try_consume_query_cast());
    }
}
