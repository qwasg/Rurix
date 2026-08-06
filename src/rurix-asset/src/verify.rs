//! M79 双构建 / mutation verifier（RXS-0337）。
//!
//! //@ spec: RXS-0337

use crate::cook::{self, BuildResult, CookPlan};
use crate::error::{AssetError, ErrorKind, Result};
use crate::graph;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct VerifyReport {
    pub double_build_isolated_roots: bool,
    pub double_build_dag_byte_equal: bool,
    pub double_build_artifacts_byte_equal: bool,
    pub double_build_manifest_digest_equal: bool,
    pub mutate_dependency_flips_key: bool,
    pub mutate_recipe_flips_key: bool,
    pub mutate_profile_flips_key: bool,
    pub mutate_tool_version_flips_key: bool,
    pub unrelated_nodes_keys_stable: bool,
    pub no_env_time_path_in_signed_bytes: bool,
    pub left_manifest: String,
    pub right_manifest: String,
    pub notes: Vec<String>,
}

impl VerifyReport {
    pub fn all_pass(&self) -> bool {
        self.double_build_isolated_roots
            && self.double_build_dag_byte_equal
            && self.double_build_artifacts_byte_equal
            && self.double_build_manifest_digest_equal
            && self.mutate_dependency_flips_key
            && self.mutate_recipe_flips_key
            && self.mutate_profile_flips_key
            && self.mutate_tool_version_flips_key
            && self.unrelated_nodes_keys_stable
            && self.no_env_time_path_in_signed_bytes
    }

    pub fn to_checks_json(&self) -> String {
        format!(
            "{{\n  \"double_build_isolated_roots\": {},\n  \"double_build_dag_byte_equal\": {},\n  \"double_build_artifacts_byte_equal\": {},\n  \"double_build_manifest_digest_equal\": {},\n  \"mutate_dependency_flips_key\": {},\n  \"mutate_recipe_flips_key\": {},\n  \"mutate_profile_flips_key\": {},\n  \"mutate_tool_version_flips_key\": {},\n  \"unrelated_nodes_keys_stable\": {},\n  \"no_env_time_path_in_signed_bytes\": {}\n}}\n",
            self.double_build_isolated_roots,
            self.double_build_dag_byte_equal,
            self.double_build_artifacts_byte_equal,
            self.double_build_manifest_digest_equal,
            self.mutate_dependency_flips_key,
            self.mutate_recipe_flips_key,
            self.mutate_profile_flips_key,
            self.mutate_tool_version_flips_key,
            self.unrelated_nodes_keys_stable,
            self.no_env_time_path_in_signed_bytes,
        )
    }
}

fn temp_pair(base: &Path) -> Result<(PathBuf, PathBuf)> {
    let a = base.join("root_a");
    let b = base.join("root_b");
    if a.exists() {
        fs::remove_dir_all(&a)?;
    }
    if b.exists() {
        fs::remove_dir_all(&b)?;
    }
    fs::create_dir_all(&a)?;
    fs::create_dir_all(&b)?;
    // 强制不同绝对路径
    let abs_a = fs::canonicalize(&a)?;
    let abs_b = fs::canonicalize(&b)?;
    if abs_a == abs_b {
        return Err(AssetError::new(
            ErrorKind::VerifyFailed,
            "isolated roots collapsed to same path",
        ));
    }
    Ok((abs_a, abs_b))
}

fn artifacts_equal(a: &BuildResult, b: &BuildResult) -> bool {
    if a.artifacts.len() != b.artifacts.len() {
        return false;
    }
    for (k, va) in &a.artifacts {
        match b.artifacts.get(k) {
            Some(vb) if va == vb => {}
            _ => return false,
        }
    }
    true
}

/// 双构建 + 四类 mutation。
pub fn verify_double_build(workspace: &Path, scratch: &Path) -> Result<VerifyReport> {
    let mut report = VerifyReport::default();
    let plan = CookPlan::default();
    let (root_a, root_b) = temp_pair(scratch)?;
    report.double_build_isolated_roots = true;

    let left = cook::cook_to(&root_a, workspace, &plan)?;
    let right = cook::cook_to(&root_b, workspace, &plan)?;
    report.left_manifest = left.manifest_digest.clone();
    report.right_manifest = right.manifest_digest.clone();

    report.double_build_dag_byte_equal = left.dag_bytes == right.dag_bytes;
    report.double_build_artifacts_byte_equal = artifacts_equal(&left, &right);
    report.double_build_manifest_digest_equal = left.manifest_digest == right.manifest_digest;

    let signed = [
        left.dag_bytes.as_slice(),
        left.manifest_digest.as_bytes(),
    ]
    .concat();
    report.no_env_time_path_in_signed_bytes = graph::signed_bytes_clean(&signed)
        && graph::signed_bytes_clean(&right.dag_bytes);

    let base_keys = left.node_keys.clone();

    // mutate dependency: change geom segments (affects geom pages only)
    let mut mut_dep = plan.clone();
    mut_dep.geom_segments = 16;
    let dep_root = scratch.join("mut_dep");
    let _ = fs::remove_dir_all(&dep_root);
    let dep = cook::cook_to(&dep_root, workspace, &mut_dep)?;
    let geom_flipped = base_keys.get("artifact.geom_pages")
        != dep.node_keys.get("artifact.geom_pages");
    let gltf_stable = base_keys.get("artifact.gltf_tables")
        == dep.node_keys.get("artifact.gltf_tables");
    report.mutate_dependency_flips_key = geom_flipped;

    // mutate recipe
    let mut mut_recipe = plan.clone();
    mut_recipe.recipe_tag = "alt".into();
    let recipe_root = scratch.join("mut_recipe");
    let _ = fs::remove_dir_all(&recipe_root);
    let rec = cook::cook_to(&recipe_root, workspace, &mut_recipe)?;
    report.mutate_recipe_flips_key = base_keys.get("artifact.gltf_tables")
        != rec.node_keys.get("artifact.gltf_tables");

    // mutate profile
    let mut mut_profile = plan.clone();
    mut_profile.profile_tag = "alt-profile".into();
    let profile_root = scratch.join("mut_profile");
    let _ = fs::remove_dir_all(&profile_root);
    let prof = cook::cook_to(&profile_root, workspace, &mut_profile)?;
    report.mutate_profile_flips_key = base_keys.get("artifact.geom_pages")
        != prof.node_keys.get("artifact.geom_pages");

    // mutate tool version
    let mut mut_tool = plan.clone();
    mut_tool.gltf_tool_version = "1.0.1".into();
    let tool_root = scratch.join("mut_tool");
    let _ = fs::remove_dir_all(&tool_root);
    let tool = cook::cook_to(&tool_root, workspace, &mut_tool)?;
    report.mutate_tool_version_flips_key = base_keys.get("artifact.gltf_tables")
        != tool.node_keys.get("artifact.gltf_tables");

    // unrelated stable across mutations where expected
    report.unrelated_nodes_keys_stable = gltf_stable
        && base_keys.get("artifact.geom_pages") == rec.node_keys.get("artifact.geom_pages")
        && base_keys.get("artifact.gltf_tables") == prof.node_keys.get("artifact.gltf_tables")
        && base_keys.get("artifact.geom_pages") == tool.node_keys.get("artifact.geom_pages");

    if !report.all_pass() {
        report.notes.push(format!("report={:?}", report));
    }
    Ok(report)
}

/// 供 smoke：写出 JSON 总报告。
pub fn verify_and_emit_json(workspace: &Path, scratch: &Path) -> Result<String> {
    let r = verify_double_build(workspace, scratch)?;
    let mut map = BTreeMap::new();
    map.insert(
        "ok".to_string(),
        if r.all_pass() { "true" } else { "false" }.to_string(),
    );
    Ok(format!(
        "{{\n  \"ok\": {},\n  \"checks\": {},\n  \"left_manifest\": \"{}\",\n  \"right_manifest\": \"{}\"\n}}\n",
        if r.all_pass() { "true" } else { "false" },
        r.to_checks_json().trim().trim_end_matches('\n'),
        r.left_manifest,
        r.right_manifest
    ))
}
