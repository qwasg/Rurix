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

/// 写一份最小三角 glTF，`apex` 为第三顶点坐标（依赖内容单变量的唯一差异）。
///
/// 采用相对路径 `.bin` 外部缓冲：顶点字节改变而 JSON 结构逐字节不变，
/// 因此六表 digest 稳定、仅 geom 上游内容变化——正是「依赖内容」单变量。
fn write_gltf_variant(dir: &Path, apex: [f32; 3]) -> Result<PathBuf> {
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir)?;
    let verts: [[f32; 3]; 3] = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], apex];
    let mut bin = Vec::with_capacity(36);
    for v in &verts {
        for c in v {
            bin.extend_from_slice(&c.to_le_bytes());
        }
    }
    fs::write(dir.join("mesh.bin"), &bin)?;
    // JSON 对两个变体逐字节相同（差异只在外部 .bin）。
    let json = concat!(
        r#"{"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0]}],"#,
        r#""nodes":[{"mesh":0}],"#,
        r#""meshes":[{"primitives":[{"attributes":{"POSITION":0},"mode":4}]}],"#,
        r#""accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3"}],"#,
        r#""bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36}],"#,
        r#""buffers":[{"byteLength":36,"uri":"mesh.bin"}]}"#,
    );
    let path = dir.join("model.gltf");
    fs::write(&path, json.as_bytes())?;
    Ok(path)
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

    let signed = [left.dag_bytes.as_slice(), left.manifest_digest.as_bytes()].concat();
    report.no_env_time_path_in_signed_bytes =
        graph::signed_bytes_clean(&signed) && graph::signed_bytes_clean(&right.dag_bytes);

    let base_keys = left.node_keys.clone();

    // mutate dependency **内容**（设计案 §3.1「依赖内容」单变量）：
    // 两份结构逐字节相同、仅顶点缓冲字节不同的 glTF 源。
    //
    // 这一腿同时是「glTF 网格真实流入 geom 下游」的守门断言：
    // 若 geom 由程序化 uv_sphere 旁路造几何（旧行为），源内容变化不会
    // 传导到 geom 产物，geom key 不翻 → 本腿必红。
    let dep_a = scratch.join("dep_src_a");
    let dep_b = scratch.join("dep_src_b");
    let src_a = write_gltf_variant(&dep_a, [0.0, 1.0, 0.0])?;
    let src_b = write_gltf_variant(&dep_b, [0.0, 2.0, 0.0])?;

    let mut plan_a = plan.clone();
    plan_a.gltf_source = src_a;
    let mut plan_b = plan.clone();
    plan_b.gltf_source = src_b;

    let root_dep_a = scratch.join("mut_dep_a");
    let root_dep_b = scratch.join("mut_dep_b");
    let _ = fs::remove_dir_all(&root_dep_a);
    let _ = fs::remove_dir_all(&root_dep_b);
    let cook_a = cook::cook_to(&root_dep_a, workspace, &plan_a)?;
    let cook_b = cook::cook_to(&root_dep_b, workspace, &plan_b)?;

    // 受影响节点：geom 页 key 必翻（真实 mesh 内容流入）。
    let geom_content_flipped =
        cook_a.node_keys.get("artifact.geom_pages") != cook_b.node_keys.get("artifact.geom_pages");
    // 无关节点：文档结构未变 → 六表 digest 不变 → gltf_tables key 必稳。
    let gltf_stable = cook_a.node_keys.get("artifact.gltf_tables")
        == cook_b.node_keys.get("artifact.gltf_tables");
    // 无关节点：纹理与 glTF 源无依赖 → key 必稳。
    let tex_stable_dep = cook_a.node_keys.get("artifact.texture_cooked")
        == cook_b.node_keys.get("artifact.texture_cooked");
    report.mutate_dependency_flips_key = geom_content_flipped;
    if !geom_content_flipped {
        report.notes.push(
            "dependency content mutation did not flip geom key: \
             geom 下游未消费真实 glTF 网格(疑 uv_sphere 旁路复活)"
                .into(),
        );
    }

    // mutate recipe
    let mut mut_recipe = plan.clone();
    mut_recipe.recipe_tag = "alt".into();
    let recipe_root = scratch.join("mut_recipe");
    let _ = fs::remove_dir_all(&recipe_root);
    let rec = cook::cook_to(&recipe_root, workspace, &mut_recipe)?;
    report.mutate_recipe_flips_key =
        base_keys.get("artifact.gltf_tables") != rec.node_keys.get("artifact.gltf_tables");

    // mutate profile
    let mut mut_profile = plan.clone();
    mut_profile.profile_tag = "alt-profile".into();
    let profile_root = scratch.join("mut_profile");
    let _ = fs::remove_dir_all(&profile_root);
    let prof = cook::cook_to(&profile_root, workspace, &mut_profile)?;
    report.mutate_profile_flips_key =
        base_keys.get("artifact.geom_pages") != prof.node_keys.get("artifact.geom_pages");

    // mutate tool version
    let mut mut_tool = plan.clone();
    mut_tool.gltf_tool_version = "1.0.1".into();
    let tool_root = scratch.join("mut_tool");
    let _ = fs::remove_dir_all(&tool_root);
    let tool = cook::cook_to(&tool_root, workspace, &mut_tool)?;
    report.mutate_tool_version_flips_key =
        base_keys.get("artifact.gltf_tables") != tool.node_keys.get("artifact.gltf_tables");

    // unrelated stable across mutations where expected
    report.unrelated_nodes_keys_stable = gltf_stable
        && tex_stable_dep
        && base_keys.get("artifact.geom_pages") == rec.node_keys.get("artifact.geom_pages")
        && base_keys.get("artifact.gltf_tables") == prof.node_keys.get("artifact.gltf_tables")
        && base_keys.get("artifact.geom_pages") == tool.node_keys.get("artifact.geom_pages")
        // 纹理节点与 recipe / geom profile / gltf tool version 均无依赖 → 三腿全程稳。
        && base_keys.get("artifact.texture_cooked") == rec.node_keys.get("artifact.texture_cooked")
        && base_keys.get("artifact.texture_cooked") == prof.node_keys.get("artifact.texture_cooked")
        && base_keys.get("artifact.texture_cooked")
            == tool.node_keys.get("artifact.texture_cooked");

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
