//! conformance/edition 语料驱动 corpus 测试(G2.5 edition 机制,RFC-0008)。
//!
//! 消费 `conformance/edition/{accept,reject}/*.toml` fixtures,断言 `Manifest::parse`
//! 的接受/拒绝结论与期望错误码(reject 文件名编码期望码:`*_rx7020*` → RX7020 /
//! `*_rx7005*` → RX7005)。条款锚定 RXS-0177~0179(spec/edition.md)。
//!
//! 纯 host/safe,无外部依赖、无 device、无网络;确定性。

use std::path::PathBuf;

use rurix_pkg::PkgError;
use rurix_pkg::manifest::{Edition, Manifest};

fn conformance_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR = <repo>/src/rurix-pkg → 上溯两级到 repo 根。
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("conformance")
        .join("edition")
}

fn toml_files(sub: &str) -> Vec<PathBuf> {
    let dir = conformance_dir().join(sub);
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("读取 conformance/edition/{sub} 失败: {e}"))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("toml"))
        .collect();
    files.sort();
    files
}

//@ spec: RXS-0177
//@ spec: RXS-0178
#[test]
fn edition_accept_corpus_parses_ok() {
    let files = toml_files("accept");
    assert!(
        !files.is_empty(),
        "conformance/edition/accept 语料为空(防空过)"
    );
    for f in files {
        let text = std::fs::read_to_string(&f).unwrap();
        let m = Manifest::parse(&text)
            .unwrap_or_else(|e| panic!("accept 语料 {f:?} 应解析成功,实得 {e}"));
        // 首期合法 edition 唯一,accept 语料均解析为首个 edition(显式或缺省)。
        assert_eq!(
            m.edition,
            Edition::FIRST,
            "accept 语料 {f:?} edition 应为首个 edition"
        );
    }
}

//@ spec: RXS-0179
#[test]
fn edition_reject_corpus_errors_with_expected_code() {
    let files = toml_files("reject");
    assert!(
        !files.is_empty(),
        "conformance/edition/reject 语料为空(防空过)"
    );
    for f in files {
        let name = f.file_name().unwrap().to_str().unwrap().to_lowercase();
        let expected = if name.contains("rx7020") {
            "RX7020"
        } else if name.contains("rx7005") {
            "RX7005"
        } else {
            panic!("reject 语料 {f:?} 文件名须编码期望错误码(*_rx7020* / *_rx7005*)");
        };
        let text = std::fs::read_to_string(&f).unwrap();
        let err = Manifest::parse(&text)
            .err()
            .unwrap_or_else(|| panic!("reject 语料 {f:?} 应解析失败(strict-only,无 fallback)"));
        assert_eq!(
            err.code(),
            expected,
            "reject 语料 {f:?} 期望错误码 {expected},实得 {}",
            err.code()
        );
        // 未知 edition 必为 EditionUnknown 变体(RXS-0179,不回退缺省)。
        if expected == "RX7020" {
            assert!(
                matches!(err, PkgError::EditionUnknown(_)),
                "reject 语料 {f:?} 期望 EditionUnknown 变体"
            );
        }
    }
}
