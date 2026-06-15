//! LSP tooling 层(RXS-0098~0103;07 §9 D-210)。

pub mod diag_json;
pub mod ide_query;
pub mod json_util;
pub mod lsp;
pub mod session;

pub use lsp::{SmokeResult, run_smoke, run_stdio_server};
pub use session::ToolingSession;
