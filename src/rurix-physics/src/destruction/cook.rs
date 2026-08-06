//! Deterministic destruction cook(RFC-0021 §4.C1)。

use std::fmt;

use rurix_pkg::sha256::{digest, hex};

use super::schema::{
    validate_graph, DestructionCookedArtifact, DestructionSourceAsset, SchemaError,
    SchemaHeader, DESTRUCTION_SCHEMA_ID, DESTRUCTION_SCHEMA_VERSION,
};

#[derive(Debug)]
pub enum CookError {
    Schema(SchemaError),
}

impl fmt::Display for CookError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Schema(e) => write!(f, "cook schema: {e}"),
        }
    }
}

impl std::error::Error for CookError {}

pub fn cook_destruction(
    source: &DestructionSourceAsset,
) -> Result<DestructionCookedArtifact, CookError> {
    if source.header.schema_id != DESTRUCTION_SCHEMA_ID {
        return Err(CookError::Schema(SchemaError::UnknownSchemaId(
            source.header.schema_id.clone(),
        )));
    }
    if source.header.schema_version != DESTRUCTION_SCHEMA_VERSION {
        return Err(CookError::Schema(SchemaError::UnknownVersion(
            source.header.schema_version,
        )));
    }

    // 稳定排序:chunk/edge/cluster/anchor/face 按 id 字典序
    let mut chunks = source.chunks.clone();
    chunks.sort_by(|a, b| a.chunk_id.cmp(&b.chunk_id));
    let mut edges = source.edges.clone();
    edges.sort_by(|a, b| a.edge_id.cmp(&b.edge_id));
    let mut clusters = source.clusters.clone();
    for c in &mut clusters {
        c.children.sort();
        c.leaf_chunks.sort();
    }
    clusters.sort_by(|a, b| a.cluster_id.cmp(&b.cluster_id));
    let mut anchors = source.anchors.clone();
    anchors.sort_by(|a, b| a.chunk_id.cmp(&b.chunk_id));
    let mut interior_faces = source.interior_faces.clone();
    interior_faces.sort_by(|a, b| a.face_id.cmp(&b.face_id));

    validate_graph(&chunks, &edges, &clusters, &anchors).map_err(CookError::Schema)?;

    let mut cooked = DestructionCookedArtifact {
        header: SchemaHeader {
            schema_id: DESTRUCTION_SCHEMA_ID.into(),
            schema_version: DESTRUCTION_SCHEMA_VERSION,
            producer_tool_version: source.header.producer_tool_version.clone(),
            source_digest: source.header.source_digest.clone(),
            dependency_digests: source.header.dependency_digests.clone(),
            cook_profile_digest: Some(hex(&digest(source.cook_profile.as_bytes()))),
            payload_digest: String::new(),
        },
        asset_id: source.asset_id.clone(),
        chunks,
        edges,
        clusters,
        anchors,
        interior_faces,
        cook_profile: source.cook_profile.clone(),
    };
    // payload_digest 覆盖除自身外的 canonical 前像
    cooked.header.payload_digest = String::new();
    let pre = cooked.canonical_json();
    cooked.header.payload_digest = hex(&digest(pre.as_bytes()));
    Ok(cooked)
}

/// 同输入独立双 cook 逐字节相等。
pub fn cook_deterministic_double(
    source: &DestructionSourceAsset,
) -> Result<(DestructionCookedArtifact, DestructionCookedArtifact), CookError> {
    let a = cook_destruction(source)?;
    let b = cook_destruction(source)?;
    if a.canonical_bytes() != b.canonical_bytes() {
        return Err(CookError::Schema(SchemaError::Parse(
            "cook not byte-stable".into(),
        )));
    }
    Ok((a, b))
}
