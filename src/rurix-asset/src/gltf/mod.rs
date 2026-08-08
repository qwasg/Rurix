//! glTF 2.0 严格导入(RXS-0333 / M81)。

pub mod canonical;
pub mod glb;
pub mod json;
pub mod validate;

use crate::error::{AssetError, ErrorKind, Result};
use canonical::CanonicalTables;
use std::path::Path;
use validate::{DECLARED_COVERAGE, ImportOptions, ImportResult};

pub use validate::{ConsumedCoverage, EXTENSION_ALLOWLIST_V1};

/// 从路径导入 `.gltf` / `.glb`。
pub fn import_path(path: &Path, opts: &ImportOptions) -> Result<ImportResult> {
    let bytes = std::fs::read(path)?;
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    if path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("glb"))
        == Some(true)
    {
        let doc = glb::parse_glb(&bytes)?;
        return validate::import_document(&doc.json, base, doc.bin.as_deref(), opts);
    }
    // .gltf: UTF-8 JSON
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| AssetError::new(ErrorKind::JsonStrict, "glTF text is not valid UTF-8"))?;
    let root = json::parse_str(text)?;
    validate::import_document(&root, base, None, opts)
}

/// 覆盖表是否盖住冻结声明清单。
pub fn coverage_complete(cov: &ConsumedCoverage) -> bool {
    DECLARED_COVERAGE.iter().all(|f| cov.fields.contains(f))
}

/// 便捷:导入并只取六表。
pub fn import_tables(path: &Path) -> Result<CanonicalTables> {
    Ok(import_path(path, &ImportOptions::default())?.tables)
}

#[cfg(test)]
mod corpus_tests {
    use super::*;
    use std::path::PathBuf;

    fn corpus_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/asset/gltf")
    }

    //@ spec: RXS-0333
    #[test]
    fn accept_fixtures_import_green() {
        let accept = corpus_root().join("accept");
        if !accept.is_dir() {
            // 语料尚未落盘时跳过(smoke 会硬要求)。
            return;
        }
        let mut n = 0;
        for ent in std::fs::read_dir(&accept).unwrap() {
            let p = ent.unwrap().path();
            let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "gltf" && ext != "glb" {
                continue;
            }
            let r = import_path(&p, &ImportOptions::default());
            assert!(r.is_ok(), "accept {} failed: {:?}", p.display(), r.err());
            let r = r.unwrap();
            assert!(
                coverage_complete(&r.coverage),
                "coverage incomplete for {}",
                p.display()
            );
            // golden
            let golden = p.with_extension("golden.json");
            if golden.is_file() {
                let got = r.tables.to_report_json();
                let exp = std::fs::read_to_string(&golden).unwrap();
                assert_eq!(got, exp, "golden mismatch for {}", p.display());
            }
            // 双导入稳定
            let r2 = import_path(&p, &ImportOptions::default()).unwrap();
            assert_eq!(r.tables, r2.tables);
            n += 1;
        }
        assert!(n >= 3, "need ≥3 accept fixtures, got {n}");
    }

    //@ spec: RXS-0333
    #[test]
    fn reject_fixtures_fail_closed() {
        let reject = corpus_root().join("reject");
        if !reject.is_dir() {
            return;
        }
        let mut n = 0;
        for ent in std::fs::read_dir(&reject).unwrap() {
            let p = ent.unwrap().path();
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            if !(name.ends_with(".gltf") || name.ends_with(".glb")) {
                continue;
            }
            let err = import_path(&p, &ImportOptions::default());
            assert!(err.is_err(), "reject {} unexpectedly OK", p.display());
            let e = err.unwrap_err();
            if name.contains("ext_outside") {
                assert_eq!(e.kind, ErrorKind::ExtensionNotAllowed);
            } else if name.contains("accessor_oob") {
                assert_eq!(e.kind, ErrorKind::AccessorOutOfBounds);
            } else if name.contains("missing_buffer") {
                assert_eq!(e.kind, ErrorKind::MissingBuffer);
            }
            n += 1;
        }
        assert!(n >= 3, "need ≥3 reject fixtures, got {n}");
    }
}
