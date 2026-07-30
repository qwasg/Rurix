//! EB 三轴映射与逐资源屏障推导内核(报告5 §2.2;RFC-0016 章 A)。
//!
//! 推导规则(Frostbite/Granite 参照,AnKi 简化 stage 集):
//! - **layout 变化或写冲突才发屏障**(WAW 真 flush 前写 / WAR 等前读 / RAW 消费前写);
//! - **首用预屏障**:纹理首用 `Undefined → 目标 layout`,`access_before=None`;
//! - **连续只读链 fake flush**:写背景对某 stage 已建立执行依赖后,同 stage 再读
//!   不发屏障;新 stage 首读只补执行依赖(`access_before=None`,不重复失效缓存);
//! - **别名交接**:新主首用屏障 before 侧取同槽前任末态,`layout_before=Undefined`
//!   入局丢弃旧数据(EB aliasing barrier 示例语义);
//! - **buffer 恒 Undefined layout**(契约;调用方在 compile 趟3 强制)。
//!
//! 同步纪律注记:单队列内提交序即执行序(AnKi 简化前提),`sync_before` 标注语义
//! 来源供后端映射保守展开;跨车道可见性由 fence 对承担(见 compile 趟4)。

use crate::graph::types::{
    AccessKind, AccessMask, Barrier, ImageLayout, PassId, QueueClass, ResourceId, SyncStage,
};

// ---------------------------------------------------------------------------
// AccessKind → EB 三轴映射(单一事实源;device 腿映射锚点)
// ---------------------------------------------------------------------------

/// 访问发生的 sync stage:copy 访问恒 [`SyncStage::Copy`];present 为终端 handoff
/// (无 GPU stage 消费,`None`);其余随 pass 车道(graphics / async compute)。
#[must_use]
pub fn sync_stage_of(kind: AccessKind, queue: QueueClass) -> SyncStage {
    match kind {
        AccessKind::CopySrc | AccessKind::CopyDst => SyncStage::Copy,
        AccessKind::Present => SyncStage::None,
        _ => match queue {
            QueueClass::Graphics => SyncStage::Graphics,
            QueueClass::AsyncCompute => SyncStage::Compute,
        },
    }
}

/// 访问的缓存可见性掩码(读侧 invalidate / 写侧 flush 的简化两侧组合;
/// `ShaderWrite` = UAV/storage 写,契约注明含读写 → `ReadWrite`)。
#[must_use]
pub fn access_mask_of(kind: AccessKind) -> AccessMask {
    match kind {
        AccessKind::ShaderRead
        | AccessKind::DepthRead
        | AccessKind::IndirectArgs
        | AccessKind::CopySrc
        | AccessKind::Present => AccessMask::Read,
        AccessKind::ColorTarget | AccessKind::DepthTarget | AccessKind::CopyDst => {
            AccessMask::Write
        }
        AccessKind::ShaderWrite => AccessMask::ReadWrite,
    }
}

/// 访问要求的 image layout(buffer 由 compile 趟3 强制覆写为 `Undefined`)。
#[must_use]
pub fn image_layout_of(kind: AccessKind) -> ImageLayout {
    match kind {
        AccessKind::ShaderRead | AccessKind::DepthRead => ImageLayout::ShaderReadOnly,
        AccessKind::ShaderWrite => ImageLayout::General,
        AccessKind::ColorTarget => ImageLayout::ColorAttachment,
        AccessKind::DepthTarget => ImageLayout::DepthAttachment,
        AccessKind::IndirectArgs => ImageLayout::Undefined,
        AccessKind::CopySrc => ImageLayout::TransferSrc,
        AccessKind::CopyDst => ImageLayout::TransferDst,
        AccessKind::Present => ImageLayout::Present,
    }
}

fn is_write_mask(m: AccessMask) -> bool {
    matches!(m, AccessMask::Write | AccessMask::ReadWrite)
}

fn stage_bit(s: SyncStage) -> u8 {
    match s {
        SyncStage::None => 1 << 0,
        SyncStage::Graphics => 1 << 1,
        SyncStage::Compute => 1 << 2,
        SyncStage::Copy => 1 << 3,
        SyncStage::All => 1 << 4,
    }
}

// ---------------------------------------------------------------------------
// 逐资源访问追踪器
// ---------------------------------------------------------------------------

/// 逐资源访问追踪器(报告5 §5 `AccessTracker{last_sync,last_access,last_layout,
/// last_writer}` + 只读链 fake flush 所需的写背景:`write_sync` / `write_mask` /
/// `ordered_stages`——最近一次写各自已建立执行依赖的 stage 位集)。
#[derive(Debug, Clone)]
pub(crate) struct AccessTracker {
    last_sync: SyncStage,
    last_access: AccessMask,
    last_layout: ImageLayout,
    last_writer: Option<PassId>,
    write_sync: SyncStage,
    write_mask: AccessMask,
    ordered_stages: u8,
}

impl AccessTracker {
    pub(crate) fn new() -> AccessTracker {
        AccessTracker {
            last_sync: SyncStage::None,
            last_access: AccessMask::None,
            last_layout: ImageLayout::Undefined,
            last_writer: None,
            write_sync: SyncStage::None,
            write_mask: AccessMask::None,
            ordered_stages: 0,
        }
    }

    /// 最近一次访问的 (sync, access) 快照(别名交接屏障的 before 侧)。
    pub(crate) fn last_state(&self) -> (SyncStage, AccessMask) {
        (self.last_sync, self.last_access)
    }

    /// 最后写入者(产物资源审计/dump 用)。
    pub(crate) fn last_writer(&self) -> Option<PassId> {
        self.last_writer
    }

    /// pass 边界屏障决策:按推导规则发射至多一条屏障,并推进追踪器状态。
    ///
    /// `handoff` = 别名交接前任的 [`last_state`](仅本资源首用时非 None)。
    pub(crate) fn update(
        &mut self,
        pass: PassId,
        res: ResourceId,
        req_sync: SyncStage,
        req_access: AccessMask,
        req_layout: ImageLayout,
        handoff: Option<(SyncStage, AccessMask)>,
    ) -> Option<Barrier> {
        // 首用判定:尚未有任何访问(last_access == None 且无写背景)。
        let untouched = self.last_access == AccessMask::None && self.write_mask == AccessMask::None;
        let req_write = is_write_mask(req_access);
        let layout_changed = req_layout != self.last_layout;

        let barrier = if untouched {
            // 首用预屏障:纹理 layout 迁移或别名交接才发;buffer 首写无前态无屏障。
            if layout_changed || handoff.is_some() {
                let (sb, ab) = handoff.unwrap_or((SyncStage::None, AccessMask::None));
                Some(Barrier {
                    res,
                    sync_before: sb,
                    sync_after: req_sync,
                    access_before: ab,
                    access_after: req_access,
                    layout_before: ImageLayout::Undefined,
                    layout_after: req_layout,
                })
            } else {
                None
            }
        } else if req_write {
            // 写冲突(WAW 真 flush 前写 / WAR 等前读)与 layout 迁移合一,必发屏障。
            Some(Barrier {
                res,
                sync_before: self.last_sync,
                sync_after: req_sync,
                access_before: self.last_access,
                access_after: req_access,
                layout_before: self.last_layout,
                layout_after: req_layout,
            })
        } else {
            let behind_write = is_write_mask(self.write_mask);
            let stage_covered = self.ordered_stages & stage_bit(req_sync) != 0;
            if layout_changed || (behind_write && !stage_covered) {
                // RAW / 迁移:写背景未 flush 过(stage 位集为空)则真 flush,
                // 其后只补执行依赖(fake flush,access_before=None 不重复失效);
                // 无写背景的纯迁移沿用前访问态。
                let (sb, ab) = if behind_write {
                    let ab = if self.ordered_stages == 0 {
                        self.write_mask
                    } else {
                        AccessMask::None
                    };
                    (self.write_sync, ab)
                } else {
                    (self.last_sync, self.last_access)
                };
                if behind_write {
                    self.ordered_stages |= stage_bit(req_sync);
                }
                Some(Barrier {
                    res,
                    sync_before: sb,
                    sync_after: req_sync,
                    access_before: ab,
                    access_after: req_access,
                    layout_before: self.last_layout,
                    layout_after: req_layout,
                })
            } else {
                None
            }
        };

        // 追踪器推进(写重建写背景;读保留写背景供后续 fake flush 判定)。
        self.last_sync = req_sync;
        self.last_access = req_access;
        self.last_layout = req_layout;
        if req_write {
            self.last_writer = Some(pass);
            self.write_sync = req_sync;
            self.write_mask = req_access;
            self.ordered_stages = 0;
        }
        barrier
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(
        res: u32,
        sb: SyncStage,
        sa: SyncStage,
        ab: AccessMask,
        aa: AccessMask,
        lb: ImageLayout,
        la: ImageLayout,
    ) -> Barrier {
        Barrier {
            res: ResourceId(res),
            sync_before: sb,
            sync_after: sa,
            access_before: ab,
            access_after: aa,
            layout_before: lb,
            layout_after: la,
        }
    }

    /// 首用预屏障:纹理写 → Undefined→ColorAttachment,access_before=None。
    #[test]
    fn first_use_texture_write_emits_prebarrier() {
        let mut t = AccessTracker::new();
        let got = t
            .update(
                PassId(0),
                ResourceId(0),
                SyncStage::Graphics,
                AccessMask::Write,
                ImageLayout::ColorAttachment,
                None,
            )
            .expect("首用纹理写应发预屏障");
        assert_eq!(
            got,
            b(
                0,
                SyncStage::None,
                SyncStage::Graphics,
                AccessMask::None,
                AccessMask::Write,
                ImageLayout::Undefined,
                ImageLayout::ColorAttachment
            )
        );
    }

    /// buffer 首写:无 layout 无前态 → 不发屏障。
    #[test]
    fn first_use_buffer_write_no_barrier() {
        let mut t = AccessTracker::new();
        assert!(
            t.update(
                PassId(0),
                ResourceId(0),
                SyncStage::Graphics,
                AccessMask::ReadWrite,
                ImageLayout::Undefined,
                None,
            )
            .is_none()
        );
    }

    /// RAW 真 flush 一次;同 stage 只读链静默;跨 stage 首读 fake flush。
    #[test]
    fn raw_flushes_once_then_read_chain_fake_flush() {
        let mut t = AccessTracker::new();
        t.update(
            PassId(0),
            ResourceId(0),
            SyncStage::Graphics,
            AccessMask::Write,
            ImageLayout::General,
            None,
        );
        let first = t
            .update(
                PassId(1),
                ResourceId(0),
                SyncStage::Graphics,
                AccessMask::Read,
                ImageLayout::General,
                None,
            )
            .expect("RAW 应发屏障");
        assert_eq!(first.access_before, AccessMask::Write); // 真 flush
        // 同 stage 再读:静默(不重复失效)。
        assert!(
            t.update(
                PassId(2),
                ResourceId(0),
                SyncStage::Graphics,
                AccessMask::Read,
                ImageLayout::General,
                None,
            )
            .is_none()
        );
        // 跨 stage 首读:fake flush(access_before=None,只补执行依赖)。
        let fake = t
            .update(
                PassId(3),
                ResourceId(0),
                SyncStage::Copy,
                AccessMask::Read,
                ImageLayout::General,
                None,
            )
            .expect("跨 stage 首读应 fake flush");
        assert_eq!(fake.sync_before, SyncStage::Graphics);
        assert_eq!(fake.access_before, AccessMask::None);
        assert_eq!(fake.sync_after, SyncStage::Copy);
    }

    /// WAW 真 flush 前写;WAR 屏障等前读(access_before=Read)。
    #[test]
    fn waw_flushes_and_war_waits() {
        let mut t = AccessTracker::new();
        t.update(
            PassId(0),
            ResourceId(0),
            SyncStage::Graphics,
            AccessMask::Write,
            ImageLayout::ColorAttachment,
            None,
        );
        let waw = t
            .update(
                PassId(1),
                ResourceId(0),
                SyncStage::Graphics,
                AccessMask::Write,
                ImageLayout::ColorAttachment,
                None,
            )
            .expect("WAW 应发屏障");
        assert_eq!(waw.access_before, AccessMask::Write);
        // 先读(RAW 真 flush),再写(WAR)。
        t.update(
            PassId(2),
            ResourceId(0),
            SyncStage::Graphics,
            AccessMask::Read,
            ImageLayout::ColorAttachment,
            None,
        );
        let war = t
            .update(
                PassId(3),
                ResourceId(0),
                SyncStage::Graphics,
                AccessMask::Write,
                ImageLayout::ColorAttachment,
                None,
            )
            .expect("WAR 应发屏障");
        assert_eq!(war.access_before, AccessMask::Read);
        assert_eq!(war.sync_before, SyncStage::Graphics);
    }

    /// 别名交接:before 侧取前任末态,layout_before=Undefined 入局。
    #[test]
    fn alias_handoff_uses_predecessor_tail_state() {
        let mut t = AccessTracker::new();
        let got = t
            .update(
                PassId(2),
                ResourceId(1),
                SyncStage::Graphics,
                AccessMask::Write,
                ImageLayout::ColorAttachment,
                Some((SyncStage::Graphics, AccessMask::Read)),
            )
            .expect("别名交接应发屏障");
        assert_eq!(got.sync_before, SyncStage::Graphics);
        assert_eq!(got.access_before, AccessMask::Read);
        assert_eq!(got.layout_before, ImageLayout::Undefined);
        assert_eq!(got.layout_after, ImageLayout::ColorAttachment);
    }

    /// 写后只读链上的 layout 迁移:写背景已 flush → access_before=None(纯迁移 +
    /// 执行依赖);无写背景的迁移 → access_before=Read。
    #[test]
    fn layout_transition_after_read_chain() {
        let mut t = AccessTracker::new();
        t.update(
            PassId(0),
            ResourceId(0),
            SyncStage::Graphics,
            AccessMask::Write,
            ImageLayout::ColorAttachment,
            None,
        );
        t.update(
            PassId(1),
            ResourceId(0),
            SyncStage::Graphics,
            AccessMask::Read,
            ImageLayout::ShaderReadOnly,
            None,
        );
        let tr = t
            .update(
                PassId(2),
                ResourceId(0),
                SyncStage::Copy,
                AccessMask::Read,
                ImageLayout::TransferSrc,
                None,
            )
            .expect("layout 迁移应发屏障");
        assert_eq!(tr.access_before, AccessMask::None); // 写背景已 flush,fake flush 迁移
        assert_eq!(tr.layout_before, ImageLayout::ShaderReadOnly);
        assert_eq!(tr.layout_after, ImageLayout::TransferSrc);
    }

    /// 最后写入者审计:写更新,读不动。
    #[test]
    fn tracks_last_writer() {
        let mut t = AccessTracker::new();
        assert_eq!(t.last_writer(), None);
        t.update(
            PassId(0),
            ResourceId(0),
            SyncStage::Graphics,
            AccessMask::Write,
            ImageLayout::ColorAttachment,
            None,
        );
        t.update(
            PassId(1),
            ResourceId(0),
            SyncStage::Graphics,
            AccessMask::Read,
            ImageLayout::ShaderReadOnly,
            None,
        );
        assert_eq!(t.last_writer(), Some(PassId(0)));
    }
}
