//! M66 physics capture/replay(`physics-capture` feature;RFC-0021 §4.A1 / §6.2)。

pub mod canonical;
pub mod divergence;
pub mod header;
pub mod inject;
pub mod journal;
pub mod recorder;
pub mod replayer;

pub use canonical::{
    canon_f32_bits, event_digest, hash_canonical_state, state_from_world, CaptureError,
    CanonicalPhysicsState, ConstraintSemantic,
};
pub use divergence::{locate_divergence, DivergenceLocate, FieldDiff};
pub use header::{
    BudgetProfile, DeterminismProfile, FixedStepRational, PhysicsCaptureHeader, RECOVERY_LAYER_V1,
};
pub use inject::{inject_before_tick, whitelist_reject, InjectRequest};
pub use journal::{body_ids_bits, JournalCommand, JournalTick, PostTick};
pub use recorder::{default_budget, default_budget_profile, CaptureArtifact, CaptureRecorder};
pub use replayer::{
    apply_journal_pre, jolt_world_desc, locate_injection_divergence, replay_capture_dir,
    replay_with_extra_journal_line, replay_with_missing_journal_line, ReplayReport, ReplayVerdict,
};
