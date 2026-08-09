//! 最小声明式 cook 执行器（M79 DAG：gltf.import + geom.pages）。

use crate::canon::{self, Value};
use crate::error::{AssetError, ErrorKind, Result};
use crate::geom_build::{concatenate_pages, pack_cluster_dag};
use crate::gltf::validate::{ImportOptions, ImportedMesh};
use crate::gltf::{self};
use crate::graph::{GraphNode, TOOL_GEOM_PAGES, TOOL_GLTF_IMPORT, TOOL_TEXTURE_COOK, ToolGraph};
use crate::texture::{
    CookProfile as TexCookProfile, TextureSemantics, cook_texture, fixture_checker_rgba16,
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
    /// geom 细分参数（真实 mesh 时用于 LOD/簇打包调参，不再造几何）。
    pub geom_segments: u32,
    pub recipe_tag: String,
    pub profile_tag: String,
    /// 纹理工具版本（tool-version mutation 面之一）。
    pub texture_tool_version: String,
    /// 纹理 cook profile 标签（`win-vulkan-bcn-v1` / `mobile-astc-v1`）。
    pub texture_profile: String,
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
            texture_tool_version: "1.0.0".into(),
            texture_profile: "win-vulkan-bcn-v1".into(),
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
    let tex_params = Value::map_of([
        (1, Value::text_ascii(&plan.texture_profile)?),
        (2, Value::text_ascii("color")?),
    ])?;
    Ok(ToolGraph {
        nodes: vec![
            GraphNode {
                tool_id: TOOL_GLTF_IMPORT.into(),
                tool_version: plan.gltf_tool_version.clone(),
                tool_digest: tool_digest(TOOL_GLTF_IMPORT, &plan.gltf_tool_version),
                typed_inputs: vec!["source.gltf".into()],
                typed_outputs: vec!["artifact.gltf_tables".into(), "artifact.gltf_mesh".into()],
                canonical_params: gltf_params,
            },
            GraphNode {
                tool_id: TOOL_GEOM_PAGES.into(),
                tool_version: plan.geom_tool_version.clone(),
                tool_digest: tool_digest(TOOL_GEOM_PAGES, &plan.geom_tool_version),
                // 真实数据流边：geom 页消费 gltf 导入产出的 mesh，
                // 不再由 uv_sphere 程序化几何旁路（设计案 §3.1）。
                typed_inputs: vec!["artifact.gltf_mesh".into()],
                typed_outputs: vec!["artifact.geom_pages".into()],
                canonical_params: geom_params,
            },
            GraphNode {
                tool_id: TOOL_TEXTURE_COOK.into(),
                tool_version: plan.texture_tool_version.clone(),
                tool_digest: tool_digest(TOOL_TEXTURE_COOK, &plan.texture_tool_version),
                typed_inputs: vec!["source.texture_rgba".into()],
                typed_outputs: vec!["artifact.texture_cooked".into()],
                canonical_params: tex_params,
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
    let imported = gltf::import_path(&gltf_path, &ImportOptions::default())
        .map_err(|e| AssetError::new(ErrorKind::VerifyFailed, format!("gltf import: {e}")))?;
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

    // --- geom pages（消费 gltf 导入的真实 mesh；uv_sphere 旁路已移除）---
    let mesh = merge_imported_meshes(&imported.meshes)?;
    let dag = build_dag(&mesh);
    let _rxgb = write_dag(&dag);
    let pages = pack_cluster_dag(&dag)
        .map_err(|e| AssetError::new(ErrorKind::VerifyFailed, format!("pack: {e}")))?;
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

    // --- texture cook（真实执行 TOOL_TEXTURE_COOK，非占位注册）---
    let tex_profile = TexCookProfile::parse(&plan.texture_profile).ok_or_else(|| {
        AssetError::new(
            ErrorKind::VerifyFailed,
            format!("unknown texture profile: {}", plan.texture_profile),
        )
    })?;
    let (tw, th, rgba) = fixture_checker_rgba16();
    let tex_dir = art_dir.join("texture");
    let tex_report = cook_texture(
        &rgba,
        tw,
        th,
        TextureSemantics::Color,
        tex_profile,
        &tex_dir,
    )
    .map_err(|e| AssetError::new(ErrorKind::VerifyFailed, format!("texture cook: {e}")))?;
    // 签名载荷 = 四腿产物字节本身（不含任何路径/时间）。
    let ktx2_bytes = fs::read(&tex_report.ktx2_path)?;
    let bcn_bytes = fs::read(&tex_report.bcn_path)?;
    let astc_bytes = fs::read(&tex_report.astc_path)?;

    // fail-closed 真实性守卫：占位/桩产物不得充当已 cook 纹理（禁假绿）。
    // 三腿各自须有容器 magic + 非常量填充载荷。
    const KTX2_MAGIC: &[u8] = b"\xabKTX 20\xbb\r\n\x1a\n";
    let leg_ok = |bytes: &[u8], magic: &[u8], name: &str| -> Result<()> {
        if !bytes.starts_with(magic) {
            return Err(AssetError::new(
                ErrorKind::VerifyFailed,
                format!("texture leg {name}: 容器 magic 缺失（疑占位产物）"),
            ));
        }
        let body = &bytes[magic.len()..];
        if body.is_empty() || body.iter().all(|&b| b == body[0]) {
            return Err(AssetError::new(
                ErrorKind::VerifyFailed,
                format!("texture leg {name}: 载荷为常量填充（疑占位/桩）"),
            ));
        }
        Ok(())
    };
    leg_ok(&ktx2_bytes, KTX2_MAGIC, "ktx2")?;
    leg_ok(&bcn_bytes, b"RXBC", "bcn")?;
    leg_ok(&astc_bytes, b"RXAS", "astc")?;

    let mut tex_payload = Vec::new();
    tex_payload.extend_from_slice(&ktx2_bytes);
    tex_payload.extend_from_slice(&bcn_bytes);
    tex_payload.extend_from_slice(&astc_bytes);
    let tex_dig = canon::hex_digest(&tex_payload);
    artifacts.insert("artifact.texture_cooked".into(), tex_payload);
    artifact_digests.insert("artifact.texture_cooked".into(), tex_dig.clone());
    node_keys.insert(
        "artifact.texture_cooked".into(),
        artifact_key(
            &tex_dig,
            TOOL_TEXTURE_COOK,
            &plan.texture_tool_version,
            "", // recipe 不进 texture 节点 key
            &plan.texture_profile,
        ),
    );

    // BuildManifest digest（canonical map of digests；无路径）
    let mut manifest_map = BTreeMap::new();
    manifest_map.insert(
        1u64,
        Value::Bytes(sha256::digest(dag_digest.as_bytes()).to_vec()),
    );
    let mut arts = BTreeMap::new();
    for (i, (k, d)) in artifact_digests.iter().enumerate() {
        arts.insert(
            i as u64,
            Value::map_of([(1, Value::text_ascii(k)?), (2, Value::text_ascii(d)?)])?,
        );
    }
    manifest_map.insert(2, Value::Map(arts));
    let manifest_cbor = canon::encode_cbor(&Value::Map(manifest_map))?;
    let manifest_digest = canon::hex_digest(&manifest_cbor);
    fs::write(out_root.join("build_manifest.cbor"), &manifest_cbor)?;
    fs::write(out_root.join("dag.cbor"), &dag_bytes)?;

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

/// 把导入的 glTF 三角图元合并成单个 `TriMesh`（geom 上游真实输入）。
///
/// 顺序 = `extract_meshes` 的 meshes×primitives 稳定序；顶点按图元依次追加，
/// 索引整体偏移。空集合 = fail-closed（不退回程序化几何）。
fn merge_imported_meshes(meshes: &[ImportedMesh]) -> Result<TriMesh> {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    for m in meshes {
        let base = u32::try_from(positions.len()).map_err(|_| {
            AssetError::new(
                ErrorKind::VerifyFailed,
                "imported vertex count overflows u32",
            )
        })?;
        positions.extend_from_slice(&m.positions);
        for &ix in &m.indices {
            if ix as usize >= m.positions.len() {
                return Err(AssetError::new(
                    ErrorKind::VerifyFailed,
                    "imported index out of range for primitive",
                ));
            }
            indices.push(base + ix);
        }
    }
    if positions.is_empty() || indices.len() < 3 {
        return Err(AssetError::new(
            ErrorKind::VerifyFailed,
            "gltf import produced no triangles; geom upstream has no real input",
        ));
    }
    if !indices.len().is_multiple_of(3) {
        return Err(AssetError::new(
            ErrorKind::VerifyFailed,
            "imported index count is not a multiple of 3",
        ));
    }
    Ok(TriMesh::new(positions, indices))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 三个首批注册工具在 M79 DAG 中都必须是真实节点（非占位注册）。
    //@ spec: RXS-0336
    #[test]
    fn dag_declares_all_three_registered_tools() {
        let g = plan_graph(&CookPlan::default()).unwrap();
        let ids: Vec<&str> = g.nodes.iter().map(|n| n.tool_id.as_str()).collect();
        assert!(ids.contains(&TOOL_GLTF_IMPORT), "缺 gltf import 节点");
        assert!(ids.contains(&TOOL_GEOM_PAGES), "缺 geom pages 节点");
        assert!(
            ids.contains(&TOOL_TEXTURE_COOK),
            "缺 texture cook 节点（占位注册不算）"
        );
        g.validate().unwrap();
    }

    /// geom 节点必须以 gltf 导入的网格为上游输入（真实数据流边）。
    //@ spec: RXS-0336
    #[test]
    fn geom_node_consumes_gltf_mesh_output() {
        let g = plan_graph(&CookPlan::default()).unwrap();
        let gltf_node = g
            .nodes
            .iter()
            .find(|n| n.tool_id == TOOL_GLTF_IMPORT)
            .expect("gltf 节点");
        assert!(
            gltf_node
                .typed_outputs
                .iter()
                .any(|o| o == "artifact.gltf_mesh"),
            "gltf 节点未导出网格输出"
        );
        let geom_node = g
            .nodes
            .iter()
            .find(|n| n.tool_id == TOOL_GEOM_PAGES)
            .expect("geom 节点");
        assert!(
            geom_node
                .typed_inputs
                .iter()
                .any(|i| i == "artifact.gltf_mesh"),
            "geom 节点未消费导入网格（疑 uv_sphere 旁路）"
        );
    }

    /// 真实导入网格进入 geom：tri_min 是单三角形，与 uv_sphere 三角数量级不同。
    //@ spec: RXS-0336
    #[test]
    fn merge_uses_imported_triangles_not_procedural() {
        let one_tri = vec![ImportedMesh {
            mesh_id: 0,
            primitive_id: 0,
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            indices: vec![0, 1, 2],
        }];
        let mesh = merge_imported_meshes(&one_tri).unwrap();
        assert_eq!(mesh.indices.len(), 3, "导入三角未被采用");
        assert_eq!(mesh.positions.len(), 3);
        // uv_sphere(1.0,12,12) 的三角数远大于 1，用于反证未走旁路。
        let procedural = TriMesh::uv_sphere(1.0, 12, 12);
        assert!(
            procedural.indices.len() > mesh.indices.len(),
            "测试前提不成立"
        );
    }

    /// 空导入（无可用网格）必须 fail-closed，而非静默回落程序化几何。
    //@ spec: RXS-0336
    #[test]
    fn empty_import_fails_closed() {
        let err = merge_imported_meshes(&[]).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("no triangles") && msg.contains("no real input"),
            "空导入 fail-closed 诊断不符预期: {err}"
        );
    }
}
