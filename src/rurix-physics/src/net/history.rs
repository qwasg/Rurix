//! 有界 input/state/snapshot history ring;耗尽 → 显式 hard correction。

use std::collections::VecDeque;

use super::frame::NetworkPhysicsFrameId;
use super::{HardCorrectionReason, NetError};

#[derive(Debug, Clone, PartialEq)]
pub struct HistoryRing<T> {
    capacity: usize,
    items: VecDeque<(NetworkPhysicsFrameId, T)>,
}

impl<T> HistoryRing<T> {
    pub fn new(capacity: usize) -> Result<Self, NetError> {
        if capacity == 0 {
            return Err(NetError::Rejected(
                "history ring capacity must be > 0".into(),
            ));
        }
        Ok(Self {
            capacity,
            items: VecDeque::with_capacity(capacity),
        })
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 推入;若将挤出仍被需要的窗口帧 → Err(HardCorrection)。
    pub fn push(
        &mut self,
        frame: NetworkPhysicsFrameId,
        value: T,
        retain_from: Option<NetworkPhysicsFrameId>,
    ) -> Result<(), NetError> {
        if self.items.len() >= self.capacity {
            if let Some(oldest) = self.items.front()
                && let Some(need) = retain_from
                && oldest.0 <= need
            {
                return Err(NetError::HardCorrection {
                    reason: HardCorrectionReason::HistoryRingOverflow,
                    detail: format!(
                        "ring capacity={} would drop frame {} still needed (>= {})",
                        self.capacity, oldest.0.0, need.0
                    ),
                });
            }
            self.items.pop_front();
        }
        self.items.push_back((frame, value));
        Ok(())
    }

    pub fn get(&self, frame: NetworkPhysicsFrameId) -> Option<&T> {
        self.items.iter().find(|(f, _)| *f == frame).map(|(_, v)| v)
    }

    pub fn iter(&self) -> impl Iterator<Item = &(NetworkPhysicsFrameId, T)> {
        self.items.iter()
    }

    pub fn frames_from(&self, start: NetworkPhysicsFrameId) -> Vec<&T> {
        self.items
            .iter()
            .filter(|(f, _)| *f >= start)
            .map(|(_, v)| v)
            .collect()
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn replace_through(
        &mut self,
        through: NetworkPhysicsFrameId,
        replacements: Vec<(NetworkPhysicsFrameId, T)>,
    ) -> Result<(), NetError>
    where
        T: Clone,
    {
        self.items.retain(|(f, _)| *f > through);
        for (f, v) in replacements {
            if f.0 > through.0 {
                return Err(NetError::Rejected(
                    "replace_through got frame beyond through".into(),
                ));
            }
            if self.items.len() >= self.capacity {
                return Err(NetError::HardCorrection {
                    reason: HardCorrectionReason::HistoryRingOverflow,
                    detail: "replace_through overflow".into(),
                });
            }
            self.items.push_back((f, v));
        }
        let slice = self.items.make_contiguous();
        slice.sort_by_key(|(f, _)| f.0);
        Ok(())
    }
}
