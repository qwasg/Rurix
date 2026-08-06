//! AP-SCHEMA 六对象字段闭集与 logical_uri(RXS-0332)。
//!
//! 本模块首批只冻结字段名闭集校验与 `logical_uri` 语法;canonical CBOR
//! (AP-CANON)由后续 M79 条款落地。

use crate::error::{AssetError, ErrorKind, Result};
use std::collections::BTreeSet;

/// SourceAsset 字段闭集(RXS-0332)。
pub const SOURCE_ASSET_FIELDS: &[&str] = &[
    "schema_version",
    "logical_uri",
    "media_type",
    "content_digest",
    "byte_len",
    "dependency_ids",
];

/// ImportRecipe 字段闭集。
pub const IMPORT_RECIPE_FIELDS: &[&str] = &[
    "schema_version",
    "importer_id",
    "importer_version",
    "importer_digest",
    "input_schema",
    "output_artifact_kind",
    "coordinates",
    "units",
    "color_space",
    "extensions_allowlist",
    "params",
    "preserve_opaque",
];

/// CookProfile 字段闭集。
pub const COOK_PROFILE_FIELDS: &[&str] = &[
    "schema_version",
    "target_os",
    "target_arch",
    "gpu_api",
    "gpu_profile",
    "capability_set",
    "quality_level",
    "texture_formats",
    "float_mode",
    "compressor_profile",
    "geometry_disk_abi",
    "geometry_memory_abi",
    "endian",
    "packing_strategy",
];

/// DerivedArtifact 字段闭集。
pub const DERIVED_ARTIFACT_FIELDS: &[&str] = &[
    "schema_version",
    "artifact_kind",
    "artifact_key",
    "payload_digest",
    "payload_len",
    "producer_tool_id",
    "producer_tool_version",
    "producer_tool_digest",
    "recipe_digest",
    "profile_digest",
    "schema_digest",
    "input_keys",
    "abi_id",
    "abi_version",
    "license_refs",
    "sbom_refs",
    "renewable",
];

/// ToolManifest 字段闭集。
pub const TOOL_MANIFEST_FIELDS: &[&str] = &[
    "schema_version",
    "tool_id",
    "tool_version",
    "impl_digest",
    "impl_kind",
    "input_kinds",
    "output_kinds",
    "param_schema",
    "license_components",
    "deterministic_capability",
];

/// BuildManifest 字段闭集。
pub const BUILD_MANIFEST_FIELDS: &[&str] = &[
    "schema_version",
    "sources",
    "dependencies",
    "tools",
    "recipes",
    "profiles",
    "artifacts",
    "abis",
    "sbom_digest",
    "license_digest",
    "build_digest",
];

/// 校验字段名集合是否恰好等于闭集(无未知、无缺失)。
pub fn check_closed_fields(present: &BTreeSet<&str>, closed: &[&str]) -> Result<()> {
    let expected: BTreeSet<&str> = closed.iter().copied().collect();
    for f in present {
        if !expected.contains(f) {
            return Err(AssetError::new(
                ErrorKind::SchemaInvalid,
                format!("unknown required/optional field outside closed set: {f}"),
            ));
        }
    }
    for f in &expected {
        if !present.contains(f) {
            return Err(AssetError::new(
                ErrorKind::SchemaInvalid,
                format!("missing required field in closed set: {f}"),
            ));
        }
    }
    Ok(())
}

/// `logical_uri` 语法校验(RXS-0332)。
pub fn validate_logical_uri(uri: &str) -> Result<()> {
    if uri.is_empty() {
        return Err(AssetError::new(
            ErrorKind::SchemaInvalid,
            "logical_uri must be non-empty",
        ));
    }
    if uri.contains('\0') {
        return Err(AssetError::new(
            ErrorKind::SchemaInvalid,
            "logical_uri must not contain NUL",
        ));
    }
    for b in uri.bytes() {
        if !(0x20..=0x7e).contains(&b) {
            return Err(AssetError::new(
                ErrorKind::SchemaInvalid,
                "logical_uri must be ASCII printable",
            ));
        }
    }
    if uri.contains('\\') {
        return Err(AssetError::new(
            ErrorKind::SchemaInvalid,
            "logical_uri must not contain backslash",
        ));
    }
    if uri.starts_with('/') {
        return Err(AssetError::new(
            ErrorKind::SchemaInvalid,
            "logical_uri must not be absolute (leading '/')",
        ));
    }
    if uri.contains("://") || uri.contains('?') || uri.contains('#') {
        return Err(AssetError::new(
            ErrorKind::SchemaInvalid,
            "logical_uri must not contain scheme/query/fragment",
        ));
    }
    // Windows drive: "C:/..." or "C:..."
    let bytes = uri.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return Err(AssetError::new(
            ErrorKind::SchemaInvalid,
            "logical_uri must not contain Windows drive prefix",
        ));
    }
    for seg in uri.split('/') {
        if seg.is_empty() {
            return Err(AssetError::new(
                ErrorKind::SchemaInvalid,
                "logical_uri must not contain empty path segment",
            ));
        }
        if seg == "." || seg == ".." {
            return Err(AssetError::new(
                ErrorKind::SchemaInvalid,
                "logical_uri must not contain '.' or '..' segments",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0332
    #[test]
    fn logical_uri_accepts_stable_relative() {
        validate_logical_uri("assets/hero.glb").unwrap();
        validate_logical_uri("pkg/a/b/c.bin").unwrap();
    }

    //@ spec: RXS-0332
    #[test]
    fn logical_uri_rejects_absolute_dotdot_backslash() {
        assert!(validate_logical_uri("/abs").is_err());
        assert!(validate_logical_uri("a/../b").is_err());
        assert!(validate_logical_uri("a/./b").is_err());
        assert!(validate_logical_uri("a\\b").is_err());
        assert!(validate_logical_uri("C:/x").is_err());
        assert!(validate_logical_uri("a?q=1").is_err());
        assert!(validate_logical_uri("http://x").is_err());
        assert!(validate_logical_uri("a\0b").is_err());
    }

    //@ spec: RXS-0332
    #[test]
    fn closed_field_sets_reject_unknown_and_missing() {
        let mut s: BTreeSet<&str> = SOURCE_ASSET_FIELDS.iter().copied().collect();
        check_closed_fields(&s, SOURCE_ASSET_FIELDS).unwrap();
        s.insert("extra");
        assert!(check_closed_fields(&s, SOURCE_ASSET_FIELDS).is_err());
        let mut t: BTreeSet<&str> = SOURCE_ASSET_FIELDS.iter().copied().collect();
        t.remove("logical_uri");
        assert!(check_closed_fields(&t, SOURCE_ASSET_FIELDS).is_err());
        // 六对象闭集均非空且互不混淆关键字段名存在。
        assert!(IMPORT_RECIPE_FIELDS.contains(&"extensions_allowlist"));
        assert!(COOK_PROFILE_FIELDS.contains(&"geometry_disk_abi"));
        assert!(DERIVED_ARTIFACT_FIELDS.contains(&"artifact_key"));
        assert!(TOOL_MANIFEST_FIELDS.contains(&"deterministic_capability"));
        assert!(BUILD_MANIFEST_FIELDS.contains(&"build_digest"));
    }
}
