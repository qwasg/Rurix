//! 最小声明式 cook 执行器（M79 DAG：gltf.import + geom.pages）。

use crate::canon::{self, Value};
use crate::error::{AssetError, ErrorKind, Result};
use crate::geom_build::{concatenate_pages, pack_cluster_dag};
use crate::gltf::{self, validate::ImportOptions};
use crate::graph::{
    TOOL_GEOM_PAGES, TOOL_GLTF_IMPORT, TOOL_TEXTURE_COOK, GraphNode, ToolGraph,
};
use rurix_geom_build::{TriMesh, build_dag, write_dag};
use rurix_pkg::sha256;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// 一次构建产物摘要。
#[derive(Debug, Clone)]
pub struct BuildResult {
    pub out_root: PathBuf,
    pub dag_bytes: Vec<u8>,
    pub dag_digest: String,
    pub artifacts: BTreeMap<String, Vec<u8>>,
    pub artifact_digests: BTreeMap<String, String>,
    pub manifest_digest: String,
    pub node_keys: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct CookPlan {
    pub gltf_source: PathBuf,
    pub gltf_tool_version: String,
    pub geom_tool_version: String,
    pub geom_segments: u32,
    pub recipe_tag: String,
    pub profile_tag: String,
}

impl Default for CookPlan {
    fn default() -> Self {
        Self {
            gltf_source: PathBuf::from("conformance/asset/gltf/accept/tri_min.gltf"),
            gltf_tool_version: "1.0.0".into(),
            geom_tool_version: "1.0.0".into(),
            geom_segments: 12,
            recipe_tag: "default".into(),
            profile_tag: "win-vulkan-v1".into(),
        }
    }
}

fn tool_digest(tool_id: &str, version: &str) -> [u8; 32] {
    let mut h = sha256::Sha256::new();
    h.update(tool_id.as_bytes());
    h.update(b"\0");
    h.update(version.as_bytes());
    h.finalize()
}

pub fn plan_graph(plan: &CookPlan) -> Result<ToolGraph> {
    let gltf_params = Value::map_of([
        (1, Value::text_ascii(&plan.recipe_tag)?),
        (2, Value::text_ascii("gltf")?),
    ])?;
    let geom_params = Value::map_of([
        (1, Value::text_ascii(&plan.profile_tag)?),
        (2, Value::Int(plan.geom_segments as i64)),
    ])?;
    Ok(ToolGraph {
        nodes: vec![
            GraphNode {
                tool_id: TOOL_GLTF_IMPORT.into(),
                tool_version: plan.gltf_tool_version.clone(),
                tool_digest: tool_digest(TOOL_GLTF_IMPORT, &plan.gltf_tool_version),
                typed_inputs: vec!["source.gltf".into()],
                typed_outputs: vec!["artifact.gltf_tables".into()],
                canonical_params: gltf_params,
            },
            GraphNode {
                tool_id: TOOL_GEOM_PAGES.into(),
                tool_version: plan.geom_tool_version.clone(),
                tool_digest: tool_digest(TOOL_GEOM_PAGES, &plan.geom_tool_version),
                typed_inputs: vec!["source.mesh_param".into()],
                typed_outputs: vec!["artifact.geom_pages".into()],
                canonical_params: geom_params,
            },
        ],
    })
}

/// 在 `out_root` 执行 cook（空目录约定由调用方保证）。
pub fn cook_to(out_root: &Path, workspace: &Path, plan: &CookPlan) -> Result<BuildResult> {
    fs::create_dir_all(out_root)?;
    let art_dir = out_root.join("artifacts");
    fs::create_dir_all(&art_dir)?;

    let graph = plan_graph(plan)?;
    graph.validate()?;
    let dag_bytes = graph.canonical_bytes()?;
    let dag_digest = canon::hex_digest(&dag_bytes);

    let mut artifacts = BTreeMap::new();
    let mut artifact_digests = BTreeMap::new();
    let mut node_keys = BTreeMap::new();

    // --- gltf import ---
    let gltf_path = if plan.gltf_source.is_absolute() {
        plan.gltf_source.clone()
    } else {
        workspace.join(&plan.gltf_source)
    };
    let imported = gltf::import_path(&gltf_path, &ImportOptions::default()).map_err(|e| {
        AssetError::new(ErrorKind::VerifyFailed, format!("gltf import: {e}"))
    })?;
    let tables_json = imported.tables.to_report_json();
    let tables_bytes = tables_json.into_bytes();
    let tables_dig = canon::hex_digest(&tables_bytes);
    fs::write(art_dir.join("gltf_tables.json"), &tables_bytes)?;
    artifacts.insert("artifact.gltf_tables".into(), tables_bytes);
    artifact_digests.insert("artifact.gltf_tables".into(), tables_dig.clone());
    node_keys.insert(
        "artifact.gltf_tables".into(),
        artifact_key(
            &tables_dig,
            TOOL_GLTF_IMPORT,
            &plan.gltf_tool_version,
            &plan.recipe_tag,
            "", // profile 不进 gltf 节点 key
        ),
    );

    // --- geom pages ---
    let dag = build_dag(&TriMesh::uv_sphere(1.0, plan.geom_segments, plan.geom_segments));
    let _rxgb = write_dag(&dag);
    let pages = pack_cluster_dag(&dag).map_err(|e| {
        AssetError::new(ErrorKind::VerifyFailed, format!("pack: {e}"))
    })?;
    let pages_bytes = concatenate_pages(&pages);
    let pages_dig = canon::hex_digest(&pages_bytes);
    fs::write(art_dir.join("geom_pages.bin"), &pages_bytes)?;
    artifacts.insert("artifact.geom_pages".into(), pages_bytes);
    artifact_digests.insert("artifact.geom_pages".into(), pages_dig.clone());
    node_keys.insert(
        "artifact.geom_pages".into(),
        artifact_key(
            &pages_dig,
            TOOL_GEOM_PAGES,
            &plan.geom_tool_version,
            "", // recipe 不进 geom 节点 key
            &plan.profile_tag,
        ),
    );

    // BuildManifest digest（canonical map of digests；无路径）
    let mut manifest_map = BTreeMap::new();
    manifest_map.insert(1u64, Value::Bytes(sha256::digest(dag_digest.as_bytes()).to_vec()));
    let mut arts = BTreeMap::new();
    for (i, (k, d)) in artifact_digests.iter().enumerate() {
        arts.insert(
            i as u64,
            Value::map_of([
                (1, Value::text_ascii(k)?),
                (2, Value::text_ascii(d)?),
            ])?,
        );
    }
    manifest_map.insert(2, Value::Map(arts));
    let manifest_cbor = canon::encode_cbor(&Value::Map(manifest_map))?;
    let manifest_digest = canon::hex_digest(&manifest_cbor);
    fs::write(out_root.join("build_manifest.cbor"), &manifest_cbor)?;
    fs::write(out_root.join("dag.cbor"), &dag_bytes)?;

    let _ = TOOL_TEXTURE_COOK; // reserved registration

    Ok(BuildResult {
        out_root: out_root.to_path_buf(),
        dag_bytes,
        dag_digest,
        artifacts,
        artifact_digests,
        manifest_digest,
        node_keys,
    })
}

fn artifact_key(
    payload_digest_hex: &str,
    tool_id: &str,
    tool_version: &str,
    recipe: &str,
    profile: &str,
) -> String {
    let mut h = sha256::Sha256::new();
    h.update(b"rurix-artifact-key-v1\0");
    h.update(payload_digest_hex.as_bytes());
    h.update(b"\0");
    h.update(tool_id.as_bytes());
    h.update(b"\0");
    h.update(tool_version.as_bytes());
    h.update(b"\0");
    h.update(recipe.as_bytes());
    h.update(b"\0");
    h.update(profile.as_bytes());
    sha256::hex(&h.finalize())
}
