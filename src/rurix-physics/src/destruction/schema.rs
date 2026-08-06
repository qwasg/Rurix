//! Destruction source/cooked schema + SchemaHeader(RFC-0021 §5.1 / §4.C1)。

use std::fmt;

use rurix_pkg::sha256::{digest, hex};

pub const DESTRUCTION_SCHEMA_ID: &str = "rurix.physics.destruction";
pub const DESTRUCTION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaError {
    UnknownVersion(u32),
    UnknownSchemaId(String),
    DanglingEdge(String),
    NonTreeCluster(String),
    IllegalAnchor(String),
    Parse(String),
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownVersion(v) => write!(f, "unknown schema_version {v}"),
            Self::UnknownSchemaId(s) => write!(f, "unknown schema_id {s}"),
            Self::DanglingEdge(e) => write!(f, "dangling edge {e}"),
            Self::NonTreeCluster(e) => write!(f, "non-tree cluster {e}"),
            Self::IllegalAnchor(e) => write!(f, "illegal anchor {e}"),
            Self::Parse(e) => write!(f, "parse: {e}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SchemaHeader {
    pub schema_id: String,
    pub schema_version: u32,
    pub producer_tool_version: String,
    pub source_digest: String,
    pub dependency_digests: Vec<String>,
    pub cook_profile_digest: Option<String>,
    pub payload_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChunkDesc {
    pub chunk_id: String,
    pub mass: f32,
    pub half_extents: [f32; 3],
    pub center: [f32; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionEdge {
    pub edge_id: String,
    pub chunk_a: String,
    pub chunk_b: String,
    pub strength: f32,
    pub contact_area: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClusterNode {
    pub cluster_id: String,
    pub parent: Option<String>,
    pub children: Vec<String>,
    pub leaf_chunks: Vec<String>,
    pub activation_depth: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Anchor {
    pub chunk_id: String,
    pub world_static: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InteriorFace {
    pub face_id: String,
    pub chunk_id: String,
    pub material: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FractureRecipe {
    ExplicitChunks,
    SeededAxisGrid { seed: u64, divisions: [u32; 3] },
}

#[derive(Debug, Clone, PartialEq)]
pub struct DestructionSourceAsset {
    pub header: SchemaHeader,
    pub asset_id: String,
    pub recipe: FractureRecipe,
    pub chunks: Vec<ChunkDesc>,
    pub edges: Vec<ConnectionEdge>,
    pub clusters: Vec<ClusterNode>,
    pub anchors: Vec<Anchor>,
    pub interior_faces: Vec<InteriorFace>,
    pub cook_profile: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DestructionCookedArtifact {
    pub header: SchemaHeader,
    pub asset_id: String,
    pub chunks: Vec<ChunkDesc>,
    pub edges: Vec<ConnectionEdge>,
    pub clusters: Vec<ClusterNode>,
    pub anchors: Vec<Anchor>,
    pub interior_faces: Vec<InteriorFace>,
    pub cook_profile: String,
}

impl DestructionCookedArtifact {
    pub fn canonical_bytes(&self) -> Vec<u8> {
        self.canonical_json().into_bytes()
    }

    pub fn canonical_json(&self) -> String {
        let mut s = String::new();
        s.push_str("{\"header\":{");
        s.push_str(&format!(
            "\"schema_id\":\"{}\",\"schema_version\":{},\"producer_tool_version\":\"{}\",\"source_digest\":\"{}\",\"dependency_digests\":[",
            esc(&self.header.schema_id),
            self.header.schema_version,
            esc(&self.header.producer_tool_version),
            esc(&self.header.source_digest),
        ));
        for (i, d) in self.header.dependency_digests.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str(&format!("\"{}\"", esc(d)));
        }
        s.push_str("],\"cook_profile_digest\":");
        match &self.header.cook_profile_digest {
            Some(d) => s.push_str(&format!("\"{}\"", esc(d))),
            None => s.push_str("null"),
        }
        s.push_str(&format!(
            ",\"payload_digest\":\"{}\"}},\"asset_id\":\"{}\",\"cook_profile\":\"{}\",",
            esc(&self.header.payload_digest),
            esc(&self.asset_id),
            esc(&self.cook_profile)
        ));
        s.push_str("\"chunks\":[");
        for (i, c) in self.chunks.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str(&format!(
                "{{\"chunk_id\":\"{}\",\"mass\":{:.6},\"half_extents\":[{:.6},{:.6},{:.6}],\"center\":[{:.6},{:.6},{:.6}]}}",
                esc(&c.chunk_id),
                c.mass,
                c.half_extents[0],
                c.half_extents[1],
                c.half_extents[2],
                c.center[0],
                c.center[1],
                c.center[2]
            ));
        }
        s.push_str("],\"edges\":[");
        for (i, e) in self.edges.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str(&format!(
                "{{\"edge_id\":\"{}\",\"chunk_a\":\"{}\",\"chunk_b\":\"{}\",\"strength\":{:.6},\"contact_area\":{:.6}}}",
                esc(&e.edge_id),
                esc(&e.chunk_a),
                esc(&e.chunk_b),
                e.strength,
                e.contact_area
            ));
        }
        s.push_str("],\"clusters\":[");
        for (i, c) in self.clusters.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            let parent = match &c.parent {
                Some(p) => format!("\"{}\"", esc(p)),
                None => "null".into(),
            };
            s.push_str(&format!(
                "{{\"cluster_id\":\"{}\",\"parent\":{},\"activation_depth\":{},\"children\":[",
                esc(&c.cluster_id),
                parent,
                c.activation_depth
            ));
            for (j, ch) in c.children.iter().enumerate() {
                if j > 0 {
                    s.push(',');
                }
                s.push_str(&format!("\"{}\"", esc(ch)));
            }
            s.push_str("],\"leaf_chunks\":[");
            for (j, ch) in c.leaf_chunks.iter().enumerate() {
                if j > 0 {
                    s.push(',');
                }
                s.push_str(&format!("\"{}\"", esc(ch)));
            }
            s.push_str("]}");
        }
        s.push_str("],\"anchors\":[");
        for (i, a) in self.anchors.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str(&format!(
                "{{\"chunk_id\":\"{}\",\"world_static\":{}}}",
                esc(&a.chunk_id),
                if a.world_static { "true" } else { "false" }
            ));
        }
        s.push_str("],\"interior_faces\":[");
        for (i, f) in self.interior_faces.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str(&format!(
                "{{\"face_id\":\"{}\",\"chunk_id\":\"{}\",\"material\":\"{}\"}}",
                esc(&f.face_id),
                esc(&f.chunk_id),
                esc(&f.material)
            ));
        }
        s.push_str("]}");
        s
    }

    pub fn digest(&self) -> String {
        hex(&digest(self.canonical_bytes().as_slice()))
    }
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

pub fn validate_graph(
    chunks: &[ChunkDesc],
    edges: &[ConnectionEdge],
    clusters: &[ClusterNode],
    anchors: &[Anchor],
) -> Result<(), SchemaError> {
    let chunk_ids: std::collections::BTreeSet<_> =
        chunks.iter().map(|c| c.chunk_id.as_str()).collect();
    for e in edges {
        if !chunk_ids.contains(e.chunk_a.as_str()) || !chunk_ids.contains(e.chunk_b.as_str()) {
            return Err(SchemaError::DanglingEdge(e.edge_id.clone()));
        }
    }
    for a in anchors {
        if !chunk_ids.contains(a.chunk_id.as_str()) {
            return Err(SchemaError::IllegalAnchor(a.chunk_id.clone()));
        }
    }
    // cluster 树:恰好一个 root(parent=None);parent 引用存在;无环
    let ids: std::collections::BTreeSet<_> =
        clusters.iter().map(|c| c.cluster_id.as_str()).collect();
    let roots: Vec<_> = clusters.iter().filter(|c| c.parent.is_none()).collect();
    if roots.len() != 1 {
        return Err(SchemaError::NonTreeCluster(format!(
            "expected 1 root, got {}",
            roots.len()
        )));
    }
    for c in clusters {
        if let Some(p) = &c.parent {
            if !ids.contains(p.as_str()) {
                return Err(SchemaError::NonTreeCluster(format!(
                    "missing parent {p}"
                )));
            }
        }
        for ch in &c.children {
            if !ids.contains(ch.as_str()) && !chunk_ids.contains(ch.as_str()) {
                // children 可为子 cluster;leaf_chunks 才指向 chunk
                if !ids.contains(ch.as_str()) {
                    return Err(SchemaError::NonTreeCluster(format!(
                        "missing child {ch}"
                    )));
                }
            }
        }
        for leaf in &c.leaf_chunks {
            if !chunk_ids.contains(leaf.as_str()) {
                return Err(SchemaError::NonTreeCluster(format!(
                    "missing leaf {leaf}"
                )));
            }
        }
    }
    // 简单环检测:从 root DFS
    let mut parent_of: std::collections::BTreeMap<&str, Option<&str>> =
        std::collections::BTreeMap::new();
    for c in clusters {
        parent_of.insert(
            c.cluster_id.as_str(),
            c.parent.as_deref(),
        );
    }
    for c in clusters {
        let mut seen = std::collections::BTreeSet::new();
        let mut cur = Some(c.cluster_id.as_str());
        while let Some(id) = cur {
            if !seen.insert(id) {
                return Err(SchemaError::NonTreeCluster(format!("cycle at {id}")));
            }
            cur = parent_of.get(id).copied().flatten();
        }
    }
    Ok(())
}

/// 极简 JSON 字段抽取(fixture 受控;非通用解析器)。
fn json_str_field(obj: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\"");
    let i = obj.find(&pat)?;
    let rest = &obj[i + pat.len()..];
    let colon = rest.find(':')?;
    let mut r = rest[colon + 1..].trim_start();
    if r.starts_with("null") {
        return None;
    }
    if !r.starts_with('"') {
        return None;
    }
    r = &r[1..];
    let mut out = String::new();
    let mut chars = r.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(n) = chars.next() {
                out.push(n);
            }
        } else if c == '"' {
            break;
        } else {
            out.push(c);
        }
    }
    Some(out)
}

fn json_num_field(obj: &str, key: &str) -> Option<f64> {
    let pat = format!("\"{key}\"");
    let i = obj.find(&pat)?;
    let rest = &obj[i + pat.len()..];
    let colon = rest.find(':')?;
    let r = rest[colon + 1..].trim_start();
    let end = r
        .find(|c: char| c == ',' || c == '}' || c == ']' || c.is_whitespace())
        .unwrap_or(r.len());
    r[..end].parse().ok()
}

fn json_bool_field(obj: &str, key: &str) -> Option<bool> {
    let pat = format!("\"{key}\"");
    let i = obj.find(&pat)?;
    let rest = &obj[i + pat.len()..];
    let colon = rest.find(':')?;
    let r = rest[colon + 1..].trim_start();
    if r.starts_with("true") {
        Some(true)
    } else if r.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

fn json_u64_field(obj: &str, key: &str) -> Option<u64> {
    json_num_field(obj, key).map(|v| v as u64)
}

fn json_u32_field(obj: &str, key: &str) -> Option<u32> {
    json_num_field(obj, key).map(|v| v as u32)
}

fn extract_array_objects<'a>(text: &'a str, key: &str) -> Result<Vec<&'a str>, String> {
    let pat = format!("\"{key}\"");
    let i = text
        .find(&pat)
        .ok_or_else(|| format!("missing array {key}"))?;
    let rest = &text[i + pat.len()..];
    let bracket = rest
        .find('[')
        .ok_or_else(|| format!("array {key} not found"))?;
    let mut depth = 0i32;
    let mut start = None;
    let bytes = rest.as_bytes();
    let mut outs = Vec::new();
    let mut j = bracket;
    while j < rest.len() {
        let c = bytes[j] as char;
        if c == '{' {
            if depth == 1 && start.is_none() {
                // inside array
            }
            if depth >= 1 && start.is_none() {
                start = Some(j);
            }
            depth += 1;
        } else if c == '}' {
            depth -= 1;
            if depth == 1 {
                if let Some(s) = start.take() {
                    outs.push(&rest[s..=j]);
                }
            }
        } else if c == '[' {
            depth += 1;
        } else if c == ']' {
            depth -= 1;
            if depth == 0 {
                break;
            }
        }
        j += 1;
    }
    Ok(outs)
}

fn parse_vec3(obj: &str, key: &str) -> Result<[f32; 3], String> {
    let pat = format!("\"{key}\"");
    let i = obj.find(&pat).ok_or_else(|| format!("missing {key}"))?;
    let rest = &obj[i + pat.len()..];
    let b = rest.find('[').ok_or("vec3 [")?;
    let e = rest[b..].find(']').ok_or("vec3 ]")?;
    let inner = &rest[b + 1..b + e];
    let parts: Vec<_> = inner.split(',').map(|s| s.trim()).collect();
    if parts.len() != 3 {
        return Err(format!("{key} len"));
    }
    Ok([
        parts[0].parse().map_err(|e| format!("{key}: {e}"))?,
        parts[1].parse().map_err(|e| format!("{key}: {e}"))?,
        parts[2].parse().map_err(|e| format!("{key}: {e}"))?,
    ])
}

fn parse_string_array(obj: &str, key: &str) -> Vec<String> {
    let pat = format!("\"{key}\"");
    let Some(i) = obj.find(&pat) else {
        return Vec::new();
    };
    let rest = &obj[i + pat.len()..];
    let Some(b) = rest.find('[') else {
        return Vec::new();
    };
    let Some(e) = rest[b..].find(']') else {
        return Vec::new();
    };
    let inner = &rest[b + 1..b + e];
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_str = false;
    let mut escape = false;
    for c in inner.chars() {
        if escape {
            cur.push(c);
            escape = false;
            continue;
        }
        match c {
            '\\' if in_str => escape = true,
            '"' => {
                if in_str {
                    out.push(cur.clone());
                    cur.clear();
                    in_str = false;
                } else {
                    in_str = true;
                }
            }
            _ if in_str => cur.push(c),
            _ => {}
        }
    }
    out
}

pub fn parse_source_json(text: &str) -> Result<DestructionSourceAsset, String> {
    let schema_id = json_str_field(text, "schema_id")
        .unwrap_or_else(|| DESTRUCTION_SCHEMA_ID.into());
    let schema_version = json_u32_field(text, "schema_version").unwrap_or(1);
    let asset_id = json_str_field(text, "asset_id").ok_or("asset_id")?;
    let cook_profile = json_str_field(text, "cook_profile").unwrap_or_else(|| "v1".into());
    let producer = json_str_field(text, "producer_tool_version").unwrap_or_else(|| "g8-m68".into());

    let mut chunks = Vec::new();
    for obj in extract_array_objects(text, "chunks")? {
        chunks.push(ChunkDesc {
            chunk_id: json_str_field(obj, "chunk_id").ok_or("chunk_id")?,
            mass: json_num_field(obj, "mass").unwrap_or(1.0) as f32,
            half_extents: parse_vec3(obj, "half_extents").unwrap_or([0.5, 0.5, 0.5]),
            center: parse_vec3(obj, "center").unwrap_or([0.0, 0.0, 0.0]),
        });
    }
    let mut edges = Vec::new();
    for obj in extract_array_objects(text, "edges")? {
        edges.push(ConnectionEdge {
            edge_id: json_str_field(obj, "edge_id").ok_or("edge_id")?,
            chunk_a: json_str_field(obj, "chunk_a").ok_or("chunk_a")?,
            chunk_b: json_str_field(obj, "chunk_b").ok_or("chunk_b")?,
            strength: json_num_field(obj, "strength").unwrap_or(1.0) as f32,
            contact_area: json_num_field(obj, "contact_area").unwrap_or(1.0) as f32,
        });
    }
    let mut clusters = Vec::new();
    for obj in extract_array_objects(text, "clusters")? {
        clusters.push(ClusterNode {
            cluster_id: json_str_field(obj, "cluster_id").ok_or("cluster_id")?,
            parent: json_str_field(obj, "parent"),
            children: parse_string_array(obj, "children"),
            leaf_chunks: parse_string_array(obj, "leaf_chunks"),
            activation_depth: json_u32_field(obj, "activation_depth").unwrap_or(0),
        });
    }
    let mut anchors = Vec::new();
    for obj in extract_array_objects(text, "anchors")? {
        anchors.push(Anchor {
            chunk_id: json_str_field(obj, "chunk_id").ok_or("anchor chunk")?,
            world_static: json_bool_field(obj, "world_static").unwrap_or(true),
        });
    }
    let mut interior_faces = Vec::new();
    for obj in extract_array_objects(text, "interior_faces")? {
        interior_faces.push(InteriorFace {
            face_id: json_str_field(obj, "face_id").ok_or("face_id")?,
            chunk_id: json_str_field(obj, "chunk_id").ok_or("face chunk")?,
            material: json_str_field(obj, "material").unwrap_or_else(|| "concrete".into()),
        });
    }

    let source_digest = hex(&digest(text.as_bytes()));
    Ok(DestructionSourceAsset {
        header: SchemaHeader {
            schema_id,
            schema_version,
            producer_tool_version: producer,
            source_digest,
            dependency_digests: vec![],
            cook_profile_digest: Some(hex(&digest(cook_profile.as_bytes()))),
            payload_digest: String::new(),
        },
        asset_id,
        recipe: FractureRecipe::ExplicitChunks,
        chunks,
        edges,
        clusters,
        anchors,
        interior_faces,
        cook_profile,
    })
}

pub fn parse_golden_json(text: &str) -> Result<super::FractureGolden, String> {
    Ok(super::FractureGolden {
        chunk_count: json_u64_field(text, "chunk_count").ok_or("chunk_count")? as usize,
        edge_count: json_u64_field(text, "edge_count").ok_or("edge_count")? as usize,
        interior_face_count: json_u64_field(text, "interior_face_count")
            .ok_or("interior_face_count")? as usize,
        anchor_count: json_u64_field(text, "anchor_count").ok_or("anchor_count")? as usize,
        cooked_digest: json_str_field(text, "cooked_digest").ok_or("cooked_digest")?,
        below_threshold_ticks: json_u64_field(text, "below_threshold_ticks").unwrap_or(5),
        below_damage_magnitude: json_num_field(text, "below_damage_magnitude").unwrap_or(0.1)
            as f32,
        above_damage_magnitude: json_num_field(text, "above_damage_magnitude").unwrap_or(10.0)
            as f32,
        damage_point: parse_vec3(text, "damage_point").unwrap_or([0.0, 1.0, 0.0]),
        damage_radius: json_num_field(text, "damage_radius").unwrap_or(1.5) as f32,
        break_tick: json_u64_field(text, "break_tick").unwrap_or(3),
        break_edge_id: json_str_field(text, "break_edge_id").ok_or("break_edge_id")?,
        activated_cluster_ids: parse_string_array(text, "activated_cluster_ids"),
    })
}
