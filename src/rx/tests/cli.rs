//! rx CLI 子命令分发与退出码集成测试(spec/toolchain.md RXS-0083 / RXS-0087 / RXS-0095)。
//!
//! 仅覆盖**不依赖工具链**(clang/link/CUDA)的路径:用法诊断 + 退出码约定 +
//! fmt 收编幂等 + rx test 发现错误。build/run/check/test/bench 的端到端真跑见 `ci/rx_cli_smoke.py`
//! (契约 G-M6-3,GPU/工具链 runner)。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

fn rx() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rx"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_ws() -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("rx_cli_pkg_{}_{nanos}_{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(base: &Path, rel: &str, content: &str) {
    let p = base.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, content).unwrap();
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

/// RXS-0083:已登记但未实现的分发位(fix/watch)→ 退出码 2。
/// vendor 于 M6.2 落地(见 vendor_offline_lock_red_green),test 于 M6.3 落地
/// (见 rx_test_bad_signature_is_rx7010),doc 于 M8.6 落地(见 doc_generates_deterministic_site)。
//@ spec: RXS-0083
#[test]
fn reserved_subcommands_exit_2() {
    for sub in ["fix", "watch"] {
        let out = rx().arg(sub).output().expect("spawn rx");
        assert_eq!(out.status.code(), Some(2), "`{sub}` 未实现应退出 2");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("RX7003"), "`{sub}` 应携带 RX7003:{stderr}");
    }
}

/// RXS-0083:rx doc(M8.6,D-M8-6 / G-M8-6)从既有单一事实源(spec/error_codes/traceability)
/// 确定性生成静态文档站:退出 0,关键页齐备(index/spec/errors/traceability)+ 条款锚点 + 错误码索引。
//@ spec: RXS-0083
#[test]
fn doc_generates_deterministic_site() {
    let ws = temp_ws();
    let out = rx()
        .args(["doc", "--root"])
        .arg(repo_root())
        .arg("--out")
        .arg(&ws)
        .output()
        .expect("spawn rx");
    assert_eq!(
        out.status.code(),
        Some(0),
        "rx doc 应成功:{}",
        String::from_utf8_lossy(&out.stderr)
    );
    for page in [
        "index.html",
        "spec.html",
        "errors.html",
        "traceability.html",
    ] {
        assert!(ws.join(page).is_file(), "缺关键页 {page}");
    }
    let spec = std::fs::read_to_string(ws.join("spec.html")).unwrap();
    assert!(
        spec.contains("id=\"RXS-0083\""),
        "spec.html 应含 RXS-0083 锚点"
    );
    let errors = std::fs::read_to_string(ws.join("errors.html")).unwrap();
    assert!(
        errors.contains("id=\"RX0001\""),
        "errors.html 应含错误码索引锚点"
    );
    let _ = std::fs::remove_dir_all(&ws);
}

/// RXS-0095:rx test 发现到非法测试签名 → RX7010,退出码 1。
//@ spec: RXS-0095
#[test]
fn rx_test_bad_signature_is_rx7010() {
    let ws = temp_ws();
    write(&ws, "bad_test.rx", "#[test]\nfn bad(x: i32) {}\n");
    let out = rx()
        .arg("test")
        .arg(ws.join("bad_test.rx"))
        .output()
        .expect("spawn rx");
    assert_eq!(out.status.code(), Some(1), "非法测试签名应退出 1");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("RX7010"), "应携带 RX7010:{stderr}");
    let _ = std::fs::remove_dir_all(&ws);
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

/// RXS-0094:rx vendor 离线写 lock+vendor → rx vendor --locked 校验真实红绿
/// (篡改 vendor 内容 → RX7008;篡改 lock → RX7007;复原 → 绿)。CPU-only 无 codegen。
//@ spec: RXS-0094
#[test]
fn vendor_offline_lock_red_green() {
    let ws = temp_ws();
    write(
        &ws,
        "rurix.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\n[dependencies]\nfoo = { path = \"foo\" }\n",
    );
    write(&ws, "src/main.rx", "fn main() {}\n");
    write(
        &ws,
        "foo/rurix.toml",
        "[package]\nname = \"foo\"\nversion = \"0.2.0\"\n",
    );
    write(&ws, "foo/src/lib.rx", "fn foo() {}\n");
    let manifest = ws.join("rurix.toml");

    // 写 lock + vendor。
    let out = rx()
        .args(["vendor", "--offline", "--manifest-path"])
        .arg(&manifest)
        .output()
        .expect("spawn rx");
    assert_eq!(
        out.status.code(),
        Some(0),
        "vendor 应成功:{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(ws.join("rurix.lock").is_file());
    assert!(ws.join("vendor/foo/rurix.toml").is_file());

    let verify = || {
        rx().args(["vendor", "--locked", "--offline", "--manifest-path"])
            .arg(&manifest)
            .output()
            .expect("spawn rx")
    };

    // 绿:locked 校验通过。
    assert_eq!(verify().status.code(), Some(0), "locked 校验应绿");

    // 红:篡改 vendor 内容 → digest 不符 RX7008。
    std::fs::write(ws.join("vendor/foo/src/lib.rx"), "fn foo() { /* x */ }\n").unwrap();
    let red = verify();
    assert_eq!(red.status.code(), Some(1), "篡改 vendor 应红");
    assert!(String::from_utf8_lossy(&red.stderr).contains("RX7008"));

    // 复原 → 转绿。
    std::fs::write(ws.join("vendor/foo/src/lib.rx"), "fn foo() {}\n").unwrap();
    assert_eq!(verify().status.code(), Some(0), "复原后应转绿");

    // 红:篡改 lock → 与重解析图不一致 RX7007。
    let lock_text = std::fs::read_to_string(ws.join("rurix.lock")).unwrap();
    std::fs::write(
        ws.join("rurix.lock"),
        lock_text.replace("content_sha256 = \"", "content_sha256 = \"0000"),
    )
    .unwrap();
    let red2 = verify();
    assert_eq!(red2.status.code(), Some(1), "篡改 lock 应红");
    assert!(String::from_utf8_lossy(&red2.stderr).contains("RX7007"));

    let _ = std::fs::remove_dir_all(&ws);
}
