//! 反馈桥(报告6 §2.4:LOD cut 选中未驻留页 → 请求队列;§2.5 纹理 feedback
//! pass 的 miss 页同栈)。把渲染侧零散反馈聚合为去重 [`PageRequest`] 列表,
//! 交给 [`super::StreamingEngine::submit_requests`]。

use std::collections::HashMap;

use crate::graph::types::PageRequest;

/// 反馈类目基值:几何 LOD cut(报告6 §2.4 / 报告1——几何是「有可渲染的东西」
/// 的前提,同级竞争让几何页优先于纹理页)。
pub const FEEDBACK_BASE_GEOMETRY_LOD: u32 = 1 << 16;
/// 反馈类目基值:纹理采样 miss(报告6 §2.5)。
pub const FEEDBACK_BASE_TEXTURE_MISS: u32 = 1 << 15;

/// 渲染反馈聚合器。
///
/// **优先级公式**:`priority = category_base.saturating_add(screen_importance)`
/// ——类目基值决定量级(几何 > 纹理),屏幕重要度(簇投影误差 / 纹理 mip 逼
/// 近度,由效果侧估计)决定类目内次序;饱和加防溢出回绕,公式对任意输入确
/// 定性。
///
/// 去重口径与引擎一致:同 `(resource, page)` 多次反馈取最高优先级;输出序 =
/// 首次反馈序(引擎 tick 再按优先级重排,此序只保证确定,不承担调度语义)。
#[derive(Debug)]
pub struct FeedbackBuilder {
    frame: u32,
    /// (resource, page) → 聚合后优先级。
    priorities: HashMap<(u32, u32), u32>,
    /// 首次出现序。
    order: Vec<(u32, u32)>,
}

impl FeedbackBuilder {
    /// 以目标帧号开建(产出请求携带该帧,供引擎 pop-in/时效口径使用)。
    pub fn new(frame: u32) -> Self {
        Self {
            frame,
            priorities: HashMap::new(),
            order: Vec::new(),
        }
    }

    /// 登记一条反馈(类目基值 + 屏幕重要度)。
    pub fn add(&mut self, resource: u32, page: u32, category_base: u32, screen_importance: u32) {
        let priority = category_base.saturating_add(screen_importance);
        let key = (resource, page);
        match self.priorities.get_mut(&key) {
            Some(p) => *p = (*p).max(priority),
            None => {
                self.priorities.insert(key, priority);
                self.order.push(key);
            }
        }
    }

    /// 聚合产出:去重请求列表(首次反馈序)。
    pub fn build(&self) -> Vec<PageRequest> {
        self.order
            .iter()
            .map(|&(resource, page)| PageRequest {
                resource,
                page_index: page,
                priority: self.priorities[&(resource, page)],
                frame: self.frame,
            })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 聚合去重 + 优先级公式:类目基值 + 屏幕重要度;同页多次反馈取最高。
    #[test]
    fn aggregate_dedup_priority_formula() {
        let mut fb = FeedbackBuilder::new(7);
        fb.add(1, 3, FEEDBACK_BASE_GEOMETRY_LOD, 500);
        fb.add(2, 1, FEEDBACK_BASE_TEXTURE_MISS, 900);
        fb.add(1, 3, FEEDBACK_BASE_GEOMETRY_LOD, 800); // 同页更高重要度 → 取高
        fb.add(1, 3, FEEDBACK_BASE_GEOMETRY_LOD, 100); // 更低 → 不覆盖
        assert_eq!(fb.len(), 2);
        let reqs = fb.build();
        assert_eq!(
            reqs,
            vec![
                PageRequest {
                    resource: 1,
                    page_index: 3,
                    priority: FEEDBACK_BASE_GEOMETRY_LOD + 800,
                    frame: 7,
                },
                PageRequest {
                    resource: 2,
                    page_index: 1,
                    priority: FEEDBACK_BASE_TEXTURE_MISS + 900,
                    frame: 7,
                },
            ]
        );
    }

    /// 空构建产出空列表;饱和加防溢出(重要度拉满不回绕)。
    #[test]
    fn empty_and_saturating() {
        let fb = FeedbackBuilder::new(3);
        assert!(fb.is_empty());
        assert_eq!(fb.build(), Vec::new());
        let mut fb = FeedbackBuilder::new(0);
        fb.add(1, 0, FEEDBACK_BASE_GEOMETRY_LOD, u32::MAX);
        assert_eq!(fb.build()[0].priority, u32::MAX);
    }
}
