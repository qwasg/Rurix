//! Canonical 六表编码与 digest(RXS-0333)。

use crate::error::Result;
use rurix_pkg::sha256::{self, Sha256};

pub const TABLE_SCENES: u8 = 0;
pub const TABLE_NODES: u8 = 1;
pub const TABLE_MESHES: u8 = 2;
pub const TABLE_PRIMITIVES: u8 = 3;
pub const TABLE_MATERIALS: u8 = 4;
pub const TABLE_TEXTURES: u8 = 5;

const DOMAIN: &[u8] = b"rurix.gltf.table.v1\0";

#[derive(Debug, Clone, PartialEq)]
pub struct TableDigest {
    pub count: u32,
    pub digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CanonicalTables {
    pub scenes: TableDigest,
    pub nodes: TableDigest,
    pub meshes: TableDigest,
    pub primitives: TableDigest,
    pub materials: TableDigest,
    pub textures: TableDigest,
}

#[derive(Debug, Clone)]
pub struct SceneRow {
    pub id: u32,
    pub name: String,
    pub nodes: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct NodeRow {
    pub id: u32,
    pub name: String,
    pub mesh: Option<u32>,
    pub children: Vec<u32>,
    pub translation: Option<[f32; 3]>,
    pub rotation: Option<[f32; 4]>,
    pub scale: Option<[f32; 3]>,
    pub matrix: Option<[f32; 16]>,
}

#[derive(Debug, Clone)]
pub struct MeshRow {
    pub id: u32,
    pub name: String,
    pub primitive_ids: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct PrimitiveRow {
    pub id: u32,
    pub mesh_id: u32,
    pub material: Option<u32>,
    pub mode: u32,
    /// (attribute name, accessor id), sorted by name.
    pub attributes: Vec<(String, u32)>,
    pub indices: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct MaterialRow {
    pub id: u32,
    pub name: String,
    pub unlit: bool,
    pub base_color_factor: [f32; 4],
    pub metallic_factor: f32,
    pub roughness_factor: f32,
}

#[derive(Debug, Clone)]
pub struct TextureRow {
    pub id: u32,
    pub sampler: Option<u32>,
    pub source: Option<u32>,
}

struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    fn new() -> Self {
        Self { buf: Vec::new() }
    }
    fn u8(&mut self, v: u8) {
        self.buf.push(v);
    }
    fn u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    fn f32(&mut self, v: f32) {
        self.buf.extend_from_slice(&v.to_bits().to_le_bytes());
    }
    fn str(&mut self, s: &str) {
        self.u32(s.len() as u32);
        self.buf.extend_from_slice(s.as_bytes());
    }
    fn opt_u32(&mut self, v: Option<u32>) {
        match v {
            Some(x) => {
                self.u8(1);
                self.u32(x);
            }
            None => self.u8(0),
        }
    }
    fn f32s(&mut self, xs: &[f32]) {
        for &x in xs {
            self.f32(x);
        }
    }
}

fn digest_table(tag: u8, count: u32, body: &[u8]) -> TableDigest {
    let mut h = Sha256::new();
    h.update(DOMAIN);
    h.update(&[tag]);
    h.update(&count.to_le_bytes());
    h.update(body);
    TableDigest {
        count,
        digest: sha256::hex(&h.finalize()),
    }
}

pub fn encode_tables(
    mut scenes: Vec<SceneRow>,
    mut nodes: Vec<NodeRow>,
    mut meshes: Vec<MeshRow>,
    mut primitives: Vec<PrimitiveRow>,
    mut materials: Vec<MaterialRow>,
    mut textures: Vec<TextureRow>,
) -> Result<CanonicalTables> {
    scenes.sort_by_key(|r| r.id);
    nodes.sort_by_key(|r| r.id);
    meshes.sort_by_key(|r| r.id);
    primitives.sort_by(|a, b| (a.mesh_id, a.id).cmp(&(b.mesh_id, b.id)));
    materials.sort_by_key(|r| r.id);
    textures.sort_by_key(|r| r.id);

    let mut w = Writer::new();
    for r in &scenes {
        w.u32(r.id);
        w.str(&r.name);
        w.u32(r.nodes.len() as u32);
        for &n in &r.nodes {
            w.u32(n);
        }
    }
    let scenes_d = digest_table(TABLE_SCENES, scenes.len() as u32, &w.buf);

    w = Writer::new();
    for r in &nodes {
        w.u32(r.id);
        w.str(&r.name);
        w.opt_u32(r.mesh);
        w.u32(r.children.len() as u32);
        for &c in &r.children {
            w.u32(c);
        }
        match &r.translation {
            Some(t) => {
                w.u8(1);
                w.f32s(t);
            }
            None => w.u8(0),
        }
        match &r.rotation {
            Some(t) => {
                w.u8(1);
                w.f32s(t);
            }
            None => w.u8(0),
        }
        match &r.scale {
            Some(t) => {
                w.u8(1);
                w.f32s(t);
            }
            None => w.u8(0),
        }
        match &r.matrix {
            Some(t) => {
                w.u8(1);
                w.f32s(t);
            }
            None => w.u8(0),
        }
    }
    let nodes_d = digest_table(TABLE_NODES, nodes.len() as u32, &w.buf);

    w = Writer::new();
    for r in &meshes {
        w.u32(r.id);
        w.str(&r.name);
        w.u32(r.primitive_ids.len() as u32);
        for &p in &r.primitive_ids {
            w.u32(p);
        }
    }
    let meshes_d = digest_table(TABLE_MESHES, meshes.len() as u32, &w.buf);

    w = Writer::new();
    for r in &primitives {
        w.u32(r.id);
        w.u32(r.mesh_id);
        w.opt_u32(r.material);
        w.u32(r.mode);
        w.u32(r.attributes.len() as u32);
        for (name, acc) in &r.attributes {
            w.str(name);
            w.u32(*acc);
        }
        w.opt_u32(r.indices);
    }
    let prims_d = digest_table(TABLE_PRIMITIVES, primitives.len() as u32, &w.buf);

    w = Writer::new();
    for r in &materials {
        w.u32(r.id);
        w.str(&r.name);
        w.u8(u8::from(r.unlit));
        w.f32s(&r.base_color_factor);
        w.f32(r.metallic_factor);
        w.f32(r.roughness_factor);
    }
    let mats_d = digest_table(TABLE_MATERIALS, materials.len() as u32, &w.buf);

    w = Writer::new();
    for r in &textures {
        w.u32(r.id);
        w.opt_u32(r.sampler);
        w.opt_u32(r.source);
    }
    let tex_d = digest_table(TABLE_TEXTURES, textures.len() as u32, &w.buf);

    Ok(CanonicalTables {
        scenes: scenes_d,
        nodes: nodes_d,
        meshes: meshes_d,
        primitives: prims_d,
        materials: mats_d,
        textures: tex_d,
    })
}

impl CanonicalTables {
    /// 人类/机器可读汇总(smoke golden 形态)。
    pub fn to_report_json(&self) -> String {
        format!(
            "{{\n  \"scenes\": {{\"count\": {}, \"digest\": \"{}\"}},\n  \"nodes\": {{\"count\": {}, \"digest\": \"{}\"}},\n  \"meshes\": {{\"count\": {}, \"digest\": \"{}\"}},\n  \"primitives\": {{\"count\": {}, \"digest\": \"{}\"}},\n  \"materials\": {{\"count\": {}, \"digest\": \"{}\"}},\n  \"textures\": {{\"count\": {}, \"digest\": \"{}\"}}\n}}\n",
            self.scenes.count,
            self.scenes.digest,
            self.nodes.count,
            self.nodes.digest,
            self.meshes.count,
            self.meshes.digest,
            self.primitives.count,
            self.primitives.digest,
            self.materials.count,
            self.materials.digest,
            self.textures.count,
            self.textures.digest,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0333
    #[test]
    fn digest_stable_for_same_rows() {
        let scenes = vec![SceneRow {
            id: 0,
            name: String::new(),
            nodes: vec![0],
        }];
        let a = encode_tables(scenes.clone(), vec![], vec![], vec![], vec![], vec![]).unwrap();
        let b = encode_tables(scenes, vec![], vec![], vec![], vec![], vec![]).unwrap();
        assert_eq!(a.scenes.digest, b.scenes.digest);
        assert_eq!(a.scenes.count, 1);
    }
}
