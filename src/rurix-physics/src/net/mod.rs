//! M67 network physics(`network-physics` feature;RFC-0021 §4.B1 / §6.2)。
//!
//! 进程内双世界 + NetTrace 确定性模拟;rollback = semantic_journal_rebuild_v1。

pub mod client;
pub mod events;
pub mod frame;
pub mod history;
pub mod rollback;
pub mod server;
pub mod smoothing;
pub mod trace;

pub use client::{ClientStepReport, ClientWorld, CorrectionReport};
pub use events::{
    EventCommitBridge, PhysicsEventId, PhysicsEventKind, derive_physics_event_id,
    event_id_for_contact,
};
pub use frame::{FrameDomainMap, NetworkPhysicsFrameId, PhysicsTickId};
pub use history::HistoryRing;
pub use rollback::{RollbackPlan, RollbackResult, TickInput, rebuild_and_resim};
pub use server::{AuthoritativeSnapshot, ServerWorld};
pub use smoothing::{
    PresentationOffset, PresentationTransform, SMOOTHING_BOUND_V1, SmoothingBound, hard_snap,
    soft_snap, within_bound,
};
pub use trace::{
    DeliveryKind, NetTrace, NetTraceReport, TraceActorImpulse, TraceFrame,
    assert_trace_deterministic, load_net_trace, parse_net_trace, run_net_trace,
    run_net_trace_with_bound,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HardCorrectionReason {
    HistoryRingOverflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetError {
    Io(String),
    Rejected(String),
    Backend(String),
    HardCorrection {
        reason: HardCorrectionReason,
        detail: String,
    },
    IncompatibleDigest {
        schema_ok: bool,
        build_ok: bool,
    },
}

impl std::fmt::Display for NetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetError::Io(m) | NetError::Rejected(m) | NetError::Backend(m) => write!(f, "{m}"),
            NetError::HardCorrection { reason, detail } => {
                write!(f, "hard_correction {reason:?}: {detail}")
            }
            NetError::IncompatibleDigest {
                schema_ok,
                build_ok,
            } => {
                write!(
                    f,
                    "incompatible digest schema_ok={schema_ok} build_ok={build_ok}"
                )
            }
        }
    }
}

impl std::error::Error for NetError {}
