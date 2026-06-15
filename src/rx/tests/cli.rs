//! rx CLI 子命令分发与退出码集成测试(spec/toolchain.md RXS-0083 / RXS-0087)。
//!
//! 仅覆盖**不依赖工具链**(clang/link/CUDA)的路径:用法诊断 + 退出码约定 +
//! fmt 收编幂等。build/run/check/bench 的端到端真跑见 `ci/rx_cli_smoke.py`
//! (契约 G-M6-3,GPU/工具链 runner)。

use std::path::{Path, PathBuf};
use std::process::Command;

fn rx() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rx"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// RXS-0083:缺子命令 / 未知子命令 → 用法错误(退出码 2,RX7003)。
//@ spec: RXS-0083
#[test]
fn missing_and_unknown_subcommand_exit_2() {
    let out = rx().output().expect("spawn rx");
    assert_eq!(out.status.code(), Some(2), "缺子命令应退出 2");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("RX7003"), "应携带 RX7003:{stderr}");

    let out = rx().arg("frobnicate").output().expect("spawn rx");
    assert_eq!(out.status.code(), Some(2), "未知子命令应退出 2");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("RX7003"), "应携带 RX7003:{stderr}");
}

/// RXS-0083:已登记但未实现的分发位(test/doc/fix/watch/vendor)→ 退出码 2。
//@ spec: RXS-0083
#[test]
fn reserved_subcommands_exit_2() {
    for sub in ["test", "doc", "fix", "watch", "vendor"] {
        let out = rx().arg(sub).output().expect("spawn rx");
        assert_eq!(out.status.code(), Some(2), "`{sub}` 未实现应退出 2");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("RX7003"), "`{sub}` 应携带 RX7003:{stderr}");
    }
}

/// RXS-0083:rx fmt 缺输入 → 用法错误退出码 2。
//@ spec: RXS-0083
#[test]
fn fmt_missing_file_exit_2() {
    let out = rx().arg("fmt").arg("--check").output().expect("spawn rx");
    assert_eq!(out.status.code(), Some(2), "rx fmt 缺文件应退出 2");
}

/// RXS-0087:rx fmt --check-idempotent 对 well-formed 语料 → 幂等退出 0。
//@ spec: RXS-0087
#[test]
fn fmt_idempotent_on_wellformed() {
    let sample = repo_root().join("conformance/syntax/hello_world.rx");
    let out = rx()
        .arg("fmt")
        .arg("--check-idempotent")
        .arg(&sample)
        .output()
        .expect("spawn rx");
    assert_eq!(
        out.status.code(),
        Some(0),
        "well-formed 语料应幂等:{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
