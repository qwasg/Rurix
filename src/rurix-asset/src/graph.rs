//! AP-GRAPH：声明式工具 DAG（RXS-0336）。
//!
//! //@ spec: RXS-0336

use crate::canon::{self, Value};
use crate::error::{AssetError, ErrorKind, Result};
use std::collections::{BTreeMap, HashMap, HashSet};

/// 已注册工具 ID（首批真工具）。
pub const TOOL_GLTF_IMPORT: &str = "rurix.gltf.import.v1";
pub const TOOL_GEOM_PAGES: &str = "rurix.geom.pages.v1";
pub const TOOL_TEXTURE_COOK: &str = "rurix.texture.cook.v1";

pub fn is_registered_tool(tool_id: &str) -> bool {
    matches!(
        tool_id,
        TOOL_GLTF_IMPORT | TOOL_GEOM_PAGES | TOOL_TEXTURE_COOK
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphNode {
    pub tool_id: String,
    pub tool_version: String,
    pub tool_digest: [u8; 32],
    pub typed_inputs: Vec<String>,
    pub typed_outputs: Vec<String>,
    pub canonical_params: Value,
}

impl GraphNode {
    pub fn node_id_bytes(&self) -> Result<Vec<u8>> {
        let v = Value::map_of([
            (1, Value::text_ascii(&self.tool_id)?),
            (2, Value::text_ascii(&self.tool_version)?),
            (3, Value::Bytes(self.tool_digest.to_vec())),
            (
                4,
                Value::Array(
                    self.typed_inputs
                        .iter()
                        .map(Value::text_ascii)
                        .collect::<Result<Vec<_>>>()?,
                ),
            ),
            (
                5,
                Value::Array(
                    self.typed_outputs
                        .iter()
                        .map(Value::text_ascii)
                        .collect::<Result<Vec<_>>>()?,
                ),
            ),
            (6, self.canonical_params.clone()),
        ])?;
        canon::encode_cbor(&v)
    }

    pub fn node_id_hex(&self) -> Result<String> {
        Ok(canon::hex_digest(&self.node_id_bytes()?))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolGraph {
    pub nodes: Vec<GraphNode>,
}

impl ToolGraph {
    pub fn validate(&self) -> Result<()> {
        let mut out_owners: HashMap<String, usize> = HashMap::new();
        for (i, n) in self.nodes.iter().enumerate() {
            if !is_registered_tool(&n.tool_id) {
                return Err(AssetError::new(
                    ErrorKind::GraphInvalid,
                    format!("unregistered tool: {}", n.tool_id),
                ));
            }
            for o in &n.typed_outputs {
                if out_owners.insert(o.clone(), i).is_some() {
                    return Err(AssetError::new(
                        ErrorKind::GraphInvalid,
                        format!("duplicate output id: {o}"),
                    ));
                }
            }
        }
        // edge: input → producer
        let mut adj: HashMap<usize, Vec<usize>> = HashMap::new();
        for (i, n) in self.nodes.iter().enumerate() {
            for inp in &n.typed_inputs {
                if let Some(&prod) = out_owners.get(inp) {
                    adj.entry(prod).or_default().push(i);
                }
                // sources (logical uris) need not be graph outputs
            }
        }
        // cycle detect
        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();
        fn dfs(
            u: usize,
            adj: &HashMap<usize, Vec<usize>>,
            visiting: &mut HashSet<usize>,
            visited: &mut HashSet<usize>,
        ) -> Result<()> {
            if visited.contains(&u) {
                return Ok(());
            }
            if !visiting.insert(u) {
                return Err(AssetError::new(ErrorKind::GraphInvalid, "DAG cycle"));
            }
            if let Some(ns) = adj.get(&u) {
                for &v in ns {
                    dfs(v, adj, visiting, visited)?;
                }
            }
            visiting.remove(&u);
            visited.insert(u);
            Ok(())
        }
        for i in 0..self.nodes.len() {
            dfs(i, &adj, &mut visiting, &mut visited)?;
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        self.validate()?;
        // stable order by node_id hex
        let mut items: Vec<(String, Vec<u8>)> = Vec::new();
        for n in &self.nodes {
            let id = n.node_id_hex()?;
            let bytes = n.node_id_bytes()?;
            items.push((id, bytes));
        }
        items.sort_by(|a, b| a.0.cmp(&b.0));
        let arr = Value::Array(items.into_iter().map(|(_, b)| Value::Bytes(b)).collect());
        canon::encode_cbor(&arr)
    }

    pub fn artifact_keys(&self) -> Result<BTreeMap<String, String>> {
        // placeholder keys: SHA-256(node_id||output_id) — cook 层会覆写为 payload digest
        let mut m = BTreeMap::new();
        for n in &self.nodes {
            let nid = n.node_id_bytes()?;
            for o in &n.typed_outputs {
                let mut buf = nid.clone();
                buf.extend_from_slice(o.as_bytes());
                m.insert(o.clone(), canon::hex_digest(&buf));
            }
        }
        Ok(m)
    }
}

/// 检测签名字节中是否含绝对路径/时间戳/PID 字面（粗扫描）。
pub fn signed_bytes_clean(bytes: &[u8]) -> bool {
    let s = String::from_utf8_lossy(bytes);
    if s.contains(":\\") || s.contains(":/") {
        return false;
    }
    if bytes.windows(4).any(|w| w == b"PID=" || w == b"pid=") {
        return false;
    }
    // ISO-ish timestamps
    if s.contains("T00:") || s.contains("T12:") {
        // allow digest hex containing T; require fuller pattern
        if s.contains("202") && s.contains('T') && s.contains('Z') {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_cycle() {
        let g = ToolGraph {
            nodes: vec![
                GraphNode {
                    tool_id: TOOL_GLTF_IMPORT.into(),
                    tool_version: "1".into(),
                    tool_digest: [1; 32],
                    typed_inputs: vec!["b".into()],
                    typed_outputs: vec!["a".into()],
                    canonical_params: Value::Null,
                },
                GraphNode {
                    tool_id: TOOL_GEOM_PAGES.into(),
                    tool_version: "1".into(),
                    tool_digest: [2; 32],
                    typed_inputs: vec!["a".into()],
                    typed_outputs: vec!["b".into()],
                    canonical_params: Value::Null,
                },
            ],
        };
        assert!(g.validate().is_err());
    }

    #[test]
    fn reject_unknown_tool() {
        let g = ToolGraph {
            nodes: vec![GraphNode {
                tool_id: "shell.evil".into(),
                tool_version: "1".into(),
                tool_digest: [0; 32],
                typed_inputs: vec![],
                typed_outputs: vec!["x".into()],
                canonical_params: Value::Null,
            }],
        };
        assert!(g.validate().is_err());
    }
}
