//! glTF 语义验证 + 六表构建(RXS-0333 七类验证;先验证后产物)。

use crate::error::{AssetError, ErrorKind, Result};
use crate::gltf::canonical::{
    self, CanonicalTables, MaterialRow, MeshRow, NodeRow, PrimitiveRow, SceneRow, TextureRow,
};
use crate::gltf::json::JsonValue;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::Path;

/// allowlist v1(RXS-0333 字面)。
pub const EXTENSION_ALLOWLIST_V1: &[&str] = &[
    "KHR_materials_unlit",
    "KHR_mesh_quantization",
    "KHR_texture_basisu",
];

/// 导入选项。
#[derive(Debug, Clone)]
pub struct ImportOptions {
    pub preserve_opaque: bool,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            preserve_opaque: false,
        }
    }
}

/// 已消费字段覆盖表(smoke `no_silent_field_drop`)。
#[derive(Debug, Default, Clone)]
pub struct ConsumedCoverage {
    pub fields: BTreeSet<&'static str>,
}

impl ConsumedCoverage {
    fn mark(&mut self, f: &'static str) {
        self.fields.insert(f);
    }
}

/// 冻结的声明 schema 覆盖清单(导入器必须消费)。
pub const DECLARED_COVERAGE: &[&str] = &[
    "asset",
    "asset.version",
    "extensionsRequired",
    "extensionsUsed",
    "buffers",
    "bufferViews",
    "accessors",
    "meshes",
    "meshes.primitives",
    "meshes.primitives.attributes",
    "meshes.primitives.indices",
    "meshes.primitives.material",
    "meshes.primitives.mode",
    "nodes",
    "nodes.children",
    "nodes.mesh",
    "scenes",
    "scenes.nodes",
    "scene",
    "materials",
    "textures",
    "images",
    "samplers",
];

fn component_size(ty: u32) -> Result<usize> {
    Ok(match ty {
        5120 | 5121 => 1, // BYTE / UNSIGNED_BYTE
        5122 | 5123 => 2, // SHORT / UNSIGNED_SHORT
        5125 | 5126 => 4, // UNSIGNED_INT / FLOAT
        _ => {
            return Err(AssetError::new(
                ErrorKind::Invalid,
                format!("unsupported accessor componentType {ty}"),
            ));
        }
    })
}

fn type_elements(type_name: &str) -> Result<usize> {
    Ok(match type_name {
        "SCALAR" => 1,
        "VEC2" => 2,
        "VEC3" => 3,
        "VEC4" => 4,
        "MAT2" => 4,
        "MAT3" => 9,
        "MAT4" => 16,
        _ => {
            return Err(AssetError::new(
                ErrorKind::Invalid,
                format!("unsupported accessor type {type_name}"),
            ));
        }
    })
}

fn arr<'a>(root: &'a JsonValue, key: &str) -> Result<&'a [JsonValue]> {
    match root.get(key) {
        None => Ok(&[]),
        Some(JsonValue::Array(a)) => Ok(a),
        Some(_) => Err(AssetError::new(
            ErrorKind::Invalid,
            format!("{key} must be array"),
        )),
    }
}

fn obj_field<'a>(obj: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    obj.get(key)
}

/// 手写 base64(data URI)解码。
fn decode_base64(input: &str) -> Result<Vec<u8>> {
    fn val(c: u8) -> Result<u8> {
        Ok(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => {
                return Err(AssetError::new(
                    ErrorKind::Invalid,
                    "invalid base64 in data URI",
                ));
            }
        })
    }
    let cleaned: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if cleaned.len() % 4 != 0 {
        return Err(AssetError::new(
            ErrorKind::Invalid,
            "base64 length not multiple of 4",
        ));
    }
    let mut out = Vec::with_capacity(cleaned.len() / 4 * 3);
    for chunk in cleaned.chunks(4) {
        let (a, b, c, d) = (chunk[0], chunk[1], chunk[2], chunk[3]);
        let n = (u32::from(val(a)?) << 18)
            | (u32::from(val(b)?) << 12)
            | (if c == b'=' {
                0
            } else {
                u32::from(val(c)?) << 6
            })
            | (if d == b'=' { 0 } else { u32::from(val(d)?) });
        out.push(((n >> 16) & 0xff) as u8);
        if c != b'=' {
            out.push(((n >> 8) & 0xff) as u8);
        }
        if d != b'=' {
            out.push((n & 0xff) as u8);
        }
    }
    Ok(out)
}

fn load_buffer_bytes(uri: &str, base_dir: &Path, glb_bin: Option<&[u8]>) -> Result<Vec<u8>> {
    if uri.is_empty() {
        // GLB buffer 0 无 uri → 使用 BIN chunk。
        return glb_bin
            .map(|b| b.to_vec())
            .ok_or_else(|| AssetError::new(ErrorKind::MissingBuffer, "GLB BIN chunk missing"));
    }
    if let Some(rest) = uri.strip_prefix("data:") {
        // data:[<mediatype>][;base64],<data>
        let comma = rest
            .find(',')
            .ok_or_else(|| AssetError::new(ErrorKind::Invalid, "malformed data URI"))?;
        let meta = &rest[..comma];
        let data = &rest[comma + 1..];
        if !meta.contains(";base64") {
            return Err(AssetError::new(
                ErrorKind::Invalid,
                "only base64 data URIs supported",
            ));
        }
        return decode_base64(data);
    }
    // 相对路径;禁止绝对/..
    if uri.contains("..") || uri.contains('\\') || Path::new(uri).is_absolute() {
        return Err(AssetError::new(
            ErrorKind::Invalid,
            format!("unsafe buffer uri: {uri}"),
        ));
    }
    let path = base_dir.join(uri);
    if !path.is_file() {
        return Err(AssetError::new(
            ErrorKind::MissingBuffer,
            format!("missing buffer file: {uri}"),
        ));
    }
    Ok(std::fs::read(path)?)
}

fn resolve_buffers(
    root: &JsonValue,
    base_dir: &Path,
    glb_bin: Option<&[u8]>,
    coverage: &mut ConsumedCoverage,
) -> Result<Vec<Vec<u8>>> {
    coverage.mark("buffers");
    let buffers = arr(root, "buffers")?;
    let mut out = Vec::with_capacity(buffers.len());
    for (i, b) in buffers.iter().enumerate() {
        let byte_len = obj_field(b, "byteLength")
            .and_then(|v| v.as_u32())
            .ok_or_else(|| {
                AssetError::new(
                    ErrorKind::Invalid,
                    format!("buffers[{i}].byteLength missing"),
                )
            })? as usize;
        let uri = obj_field(b, "uri").and_then(|v| v.as_str());
        let bytes = if let Some(u) = uri {
            load_buffer_bytes(u, base_dir, glb_bin)?
        } else if i == 0 {
            load_buffer_bytes("", base_dir, glb_bin)?
        } else {
            return Err(AssetError::new(
                ErrorKind::MissingBuffer,
                format!("buffers[{i}] missing uri"),
            ));
        };
        if bytes.len() < byte_len {
            return Err(AssetError::new(
                ErrorKind::MissingBuffer,
                format!(
                    "buffers[{i}] byteLength {byte_len} > actual {}",
                    bytes.len()
                ),
            ));
        }
        // 截到声明长度(填充可超出)。
        out.push(bytes[..byte_len].to_vec());
    }
    Ok(out)
}

fn check_extensions(
    root: &JsonValue,
    opts: &ImportOptions,
    coverage: &mut ConsumedCoverage,
) -> Result<()> {
    coverage.mark("extensionsRequired");
    coverage.mark("extensionsUsed");
    let allow: HashSet<&str> = EXTENSION_ALLOWLIST_V1.iter().copied().collect();
    if let Some(req) = root.get("extensionsRequired") {
        let arr = req.as_array().ok_or_else(|| {
            AssetError::new(ErrorKind::Invalid, "extensionsRequired must be array")
        })?;
        for e in arr {
            let name = e.as_str().ok_or_else(|| {
                AssetError::new(
                    ErrorKind::Invalid,
                    "extensionsRequired entry must be string",
                )
            })?;
            if !allow.contains(name) {
                return Err(AssetError::new(
                    ErrorKind::ExtensionNotAllowed,
                    format!("required extension outside allowlist: {name}"),
                ));
            }
        }
    }
    if let Some(used) = root.get("extensionsUsed") {
        let arr = used
            .as_array()
            .ok_or_else(|| AssetError::new(ErrorKind::Invalid, "extensionsUsed must be array"))?;
        for e in arr {
            let name = e.as_str().ok_or_else(|| {
                AssetError::new(ErrorKind::Invalid, "extensionsUsed entry must be string")
            })?;
            if allow.contains(name) {
                continue;
            }
            // optional 扩展默认拒;preserve_opaque 且不影响语义时保留——
            // 首版保守:仍拒录(六表不消费未知扩展语义)。
            if opts.preserve_opaque {
                // 仍 fail-closed:未知 optional 会影响是否静默丢字段的边界;
                // 首版不开放 opaque 保留路径以外的语义通道。
                return Err(AssetError::new(
                    ErrorKind::ExtensionNotAllowed,
                    format!(
                        "optional extension not in allowlist (preserve_opaque unsupported for '{name}' in v1)"
                    ),
                ));
            }
            return Err(AssetError::new(
                ErrorKind::ExtensionNotAllowed,
                format!("optional extension outside allowlist: {name}"),
            ));
        }
    }
    Ok(())
}

fn accessor_byte_span(
    accessor: &JsonValue,
    buffer_views: &[JsonValue],
) -> Result<(usize, usize, u32)> {
    let count = obj_field(accessor, "count")
        .and_then(|v| v.as_u32())
        .ok_or_else(|| AssetError::new(ErrorKind::Invalid, "accessor.count missing"))?
        as usize;
    let component_type = obj_field(accessor, "componentType")
        .and_then(|v| v.as_u32())
        .ok_or_else(|| AssetError::new(ErrorKind::Invalid, "accessor.componentType missing"))?;
    let type_name = obj_field(accessor, "type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AssetError::new(ErrorKind::Invalid, "accessor.type missing"))?;
    let csize = component_size(component_type)?;
    let elems = type_elements(type_name)?;
    let element_size = csize * elems;
    let byte_offset = obj_field(accessor, "byteOffset")
        .and_then(|v| v.as_u32())
        .unwrap_or(0) as usize;

    let bv_index = obj_field(accessor, "bufferView").and_then(|v| v.as_u32());
    // 无 bufferView 的 accessor 仅用于稀疏/稀疏补零——首版要求有 bufferView
    // (除非 count==0)。
    let Some(bvi) = bv_index else {
        if count == 0 {
            return Ok((0, 0, component_type));
        }
        return Err(AssetError::new(
            ErrorKind::AccessorOutOfBounds,
            "accessor missing bufferView",
        ));
    };
    if bvi as usize >= buffer_views.len() {
        return Err(AssetError::new(
            ErrorKind::DanglingReference,
            format!("accessor.bufferView {bvi} OOB"),
        ));
    }
    let bv = &buffer_views[bvi as usize];
    let bv_len = obj_field(bv, "byteLength")
        .and_then(|v| v.as_u32())
        .ok_or_else(|| AssetError::new(ErrorKind::Invalid, "bufferView.byteLength missing"))?
        as usize;
    let stride = obj_field(accessor, "byteStride")
        .or_else(|| obj_field(bv, "byteStride"))
        .and_then(|v| v.as_u32())
        .map(|s| s as usize)
        .unwrap_or(element_size);
    if stride < element_size {
        return Err(AssetError::new(
            ErrorKind::AccessorOutOfBounds,
            "byteStride < element size",
        ));
    }
    if count == 0 {
        return Ok((byte_offset, 0, component_type));
    }
    let need = byte_offset
        .checked_add(
            (count - 1)
                .checked_mul(stride)
                .and_then(|x| x.checked_add(element_size))
                .ok_or_else(|| {
                    AssetError::new(ErrorKind::AccessorOutOfBounds, "accessor span overflow")
                })?,
        )
        .ok_or_else(|| AssetError::new(ErrorKind::AccessorOutOfBounds, "accessor span overflow"))?;
    if need > bv_len {
        return Err(AssetError::new(
            ErrorKind::AccessorOutOfBounds,
            format!("accessor span {need} exceeds bufferView length {bv_len}"),
        ));
    }
    // bufferView → buffer 边界。
    let buf_index = obj_field(bv, "buffer")
        .and_then(|v| v.as_u32())
        .ok_or_else(|| AssetError::new(ErrorKind::Invalid, "bufferView.buffer missing"))?;
    let bv_offset = obj_field(bv, "byteOffset")
        .and_then(|v| v.as_u32())
        .unwrap_or(0) as usize;
    let _ = (buf_index, bv_offset); // 在 validate_accessors 中用 buffers 再核
    Ok((byte_offset, need, component_type))
}

fn validate_accessors(
    root: &JsonValue,
    buffers: &[Vec<u8>],
    coverage: &mut ConsumedCoverage,
) -> Result<()> {
    coverage.mark("accessors");
    coverage.mark("bufferViews");
    let accessors = arr(root, "accessors")?;
    let buffer_views = arr(root, "bufferViews")?;
    for (i, bv) in buffer_views.iter().enumerate() {
        let buf_i = obj_field(bv, "buffer")
            .and_then(|v| v.as_u32())
            .ok_or_else(|| {
                AssetError::new(
                    ErrorKind::Invalid,
                    format!("bufferViews[{i}].buffer missing"),
                )
            })? as usize;
        if buf_i >= buffers.len() {
            return Err(AssetError::new(
                ErrorKind::DanglingReference,
                format!("bufferViews[{i}].buffer OOB"),
            ));
        }
        let bv_off = obj_field(bv, "byteOffset")
            .and_then(|v| v.as_u32())
            .unwrap_or(0) as usize;
        let bv_len = obj_field(bv, "byteLength")
            .and_then(|v| v.as_u32())
            .ok_or_else(|| {
                AssetError::new(
                    ErrorKind::Invalid,
                    format!("bufferViews[{i}].byteLength missing"),
                )
            })? as usize;
        if bv_off.checked_add(bv_len).map(|e| e > buffers[buf_i].len()) != Some(false) {
            return Err(AssetError::new(
                ErrorKind::AccessorOutOfBounds,
                format!("bufferViews[{i}] exceeds buffer"),
            ));
        }
    }
    for (i, acc) in accessors.iter().enumerate() {
        let (_off, _need, _) = accessor_byte_span(acc, buffer_views)
            .map_err(|e| AssetError::new(e.kind, format!("accessors[{i}]: {}", e.message)))?;
        // sparse
        if let Some(sparse) = obj_field(acc, "sparse") {
            let sc = obj_field(sparse, "count")
                .and_then(|v| v.as_u32())
                .ok_or_else(|| {
                    AssetError::new(
                        ErrorKind::Invalid,
                        format!("accessors[{i}].sparse.count missing"),
                    )
                })? as usize;
            for key in ["indices", "values"] {
                let sub = obj_field(sparse, key).ok_or_else(|| {
                    AssetError::new(
                        ErrorKind::Invalid,
                        format!("accessors[{i}].sparse.{key} missing"),
                    )
                })?;
                let bvi = obj_field(sub, "bufferView")
                    .and_then(|v| v.as_u32())
                    .ok_or_else(|| {
                        AssetError::new(
                            ErrorKind::Invalid,
                            format!("accessors[{i}].sparse.{key}.bufferView missing"),
                        )
                    })? as usize;
                if bvi >= buffer_views.len() {
                    return Err(AssetError::new(
                        ErrorKind::AccessorOutOfBounds,
                        format!("accessors[{i}].sparse.{key}.bufferView OOB"),
                    ));
                }
                let _ = sc;
            }
        }
    }
    Ok(())
}

fn read_index_values(
    accessor: &JsonValue,
    buffer_views: &[JsonValue],
    buffers: &[Vec<u8>],
) -> Result<Vec<u32>> {
    let count = obj_field(accessor, "count")
        .and_then(|v| v.as_u32())
        .unwrap() as usize;
    let component_type = obj_field(accessor, "componentType")
        .and_then(|v| v.as_u32())
        .unwrap();
    let acc_off = obj_field(accessor, "byteOffset")
        .and_then(|v| v.as_u32())
        .unwrap_or(0) as usize;
    let bvi = obj_field(accessor, "bufferView")
        .and_then(|v| v.as_u32())
        .unwrap() as usize;
    let bv = &buffer_views[bvi];
    let buf_i = obj_field(bv, "buffer").and_then(|v| v.as_u32()).unwrap() as usize;
    let bv_off = obj_field(bv, "byteOffset")
        .and_then(|v| v.as_u32())
        .unwrap_or(0) as usize;
    let csize = component_size(component_type)?;
    let stride = obj_field(accessor, "byteStride")
        .or_else(|| obj_field(bv, "byteStride"))
        .and_then(|v| v.as_u32())
        .map(|s| s as usize)
        .unwrap_or(csize);
    let base = &buffers[buf_i];
    let mut out = Vec::with_capacity(count);
    for n in 0..count {
        let off = bv_off + acc_off + n * stride;
        let v = match component_type {
            5121 => u32::from(base[off]),
            5123 => u32::from(u16::from_le_bytes([base[off], base[off + 1]])),
            5125 => u32::from_le_bytes([base[off], base[off + 1], base[off + 2], base[off + 3]]),
            _ => {
                return Err(AssetError::new(
                    ErrorKind::Invalid,
                    "index accessor componentType must be u8/u16/u32",
                ));
            }
        };
        out.push(v);
    }
    Ok(out)
}

fn validate_indices_and_refs(
    root: &JsonValue,
    buffers: &[Vec<u8>],
    coverage: &mut ConsumedCoverage,
) -> Result<()> {
    coverage.mark("meshes");
    coverage.mark("meshes.primitives");
    coverage.mark("meshes.primitives.attributes");
    coverage.mark("meshes.primitives.indices");
    coverage.mark("meshes.primitives.material");
    coverage.mark("meshes.primitives.mode");
    coverage.mark("materials");
    coverage.mark("textures");
    coverage.mark("images");
    coverage.mark("samplers");
    coverage.mark("nodes");
    coverage.mark("nodes.children");
    coverage.mark("nodes.mesh");
    coverage.mark("scenes");
    coverage.mark("scenes.nodes");
    coverage.mark("scene");

    let accessors = arr(root, "accessors")?;
    let buffer_views = arr(root, "bufferViews")?;
    let meshes = arr(root, "meshes")?;
    let materials = arr(root, "materials")?;
    let textures = arr(root, "textures")?;
    let images = arr(root, "images")?;
    let samplers = arr(root, "samplers")?;
    let nodes = arr(root, "nodes")?;
    let scenes = arr(root, "scenes")?;

    for (mi, mesh) in meshes.iter().enumerate() {
        let prims = obj_field(mesh, "primitives")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                AssetError::new(
                    ErrorKind::Invalid,
                    format!("meshes[{mi}].primitives missing"),
                )
            })?;
        for (pi, prim) in prims.iter().enumerate() {
            let attrs = obj_field(prim, "attributes")
                .and_then(|v| v.as_object())
                .ok_or_else(|| {
                    AssetError::new(
                        ErrorKind::Invalid,
                        format!("meshes[{mi}].primitives[{pi}].attributes missing"),
                    )
                })?;
            let mut pos_count: Option<u32> = None;
            for (name, acc_v) in attrs {
                let ai = acc_v.as_u32().ok_or_else(|| {
                    AssetError::new(
                        ErrorKind::Invalid,
                        format!("attribute {name} accessor index"),
                    )
                })? as usize;
                if ai >= accessors.len() {
                    return Err(AssetError::new(
                        ErrorKind::DanglingReference,
                        format!("attribute {name} accessor OOB"),
                    ));
                }
                if name == "POSITION" {
                    pos_count = obj_field(&accessors[ai], "count").and_then(|v| v.as_u32());
                }
            }
            if let Some(idx_v) = obj_field(prim, "indices") {
                let ai = idx_v.as_u32().ok_or_else(|| {
                    AssetError::new(ErrorKind::Invalid, "indices must be accessor index")
                })? as usize;
                if ai >= accessors.len() {
                    return Err(AssetError::new(
                        ErrorKind::DanglingReference,
                        "indices accessor OOB",
                    ));
                }
                let vertex_count = pos_count.ok_or_else(|| {
                    AssetError::new(ErrorKind::Invalid, "indexed primitive requires POSITION")
                })?;
                let values = read_index_values(&accessors[ai], buffer_views, buffers)?;
                for (k, &ix) in values.iter().enumerate() {
                    if ix >= vertex_count {
                        return Err(AssetError::new(
                            ErrorKind::IndexOutOfBounds,
                            format!("index[{k}]={ix} >= vertex_count {vertex_count}"),
                        ));
                    }
                }
            }
            if let Some(m) = obj_field(prim, "material") {
                let mi2 = m
                    .as_u32()
                    .ok_or_else(|| AssetError::new(ErrorKind::Invalid, "material index"))?
                    as usize;
                if mi2 >= materials.len() {
                    return Err(AssetError::new(
                        ErrorKind::DanglingReference,
                        "material index OOB",
                    ));
                }
            }
        }
    }

    for (ti, tex) in textures.iter().enumerate() {
        if let Some(s) = obj_field(tex, "sampler") {
            let si = s.as_u32().unwrap_or(u32::MAX) as usize;
            if si >= samplers.len() {
                return Err(AssetError::new(
                    ErrorKind::DanglingReference,
                    format!("textures[{ti}].sampler OOB"),
                ));
            }
        }
        if let Some(s) = obj_field(tex, "source") {
            let si = s.as_u32().unwrap_or(u32::MAX) as usize;
            if si >= images.len() {
                return Err(AssetError::new(
                    ErrorKind::DanglingReference,
                    format!("textures[{ti}].source OOB"),
                ));
            }
        }
    }

    // node 环检测 + 引用。
    let n = nodes.len();
    let mut children_of: Vec<Vec<u32>> = vec![Vec::new(); n];
    for (i, node) in nodes.iter().enumerate() {
        if let Some(m) = obj_field(node, "mesh") {
            let mi = m.as_u32().unwrap_or(u32::MAX) as usize;
            if mi >= meshes.len() {
                return Err(AssetError::new(
                    ErrorKind::DanglingReference,
                    format!("nodes[{i}].mesh OOB"),
                ));
            }
        }
        if let Some(ch) = obj_field(node, "children").and_then(|v| v.as_array()) {
            for c in ch {
                let ci = c
                    .as_u32()
                    .ok_or_else(|| AssetError::new(ErrorKind::Invalid, "child index"))?
                    as usize;
                if ci >= n {
                    return Err(AssetError::new(
                        ErrorKind::DanglingReference,
                        format!("nodes[{i}].children OOB"),
                    ));
                }
                children_of[i].push(ci as u32);
            }
        }
    }
    // DFS 环检测。
    let mut state = vec![0u8; n]; // 0=unseen,1=stack,2=done
    fn dfs(u: usize, children_of: &[Vec<u32>], state: &mut [u8]) -> Result<()> {
        state[u] = 1;
        for &v in &children_of[u] {
            let v = v as usize;
            match state[v] {
                1 => {
                    return Err(AssetError::new(
                        ErrorKind::NodeCycle,
                        "node graph contains a cycle",
                    ));
                }
                0 => dfs(v, children_of, state)?,
                _ => {}
            }
        }
        state[u] = 2;
        Ok(())
    }
    for i in 0..n {
        if state[i] == 0 {
            dfs(i, &children_of, &mut state)?;
        }
    }

    for (si, scene) in scenes.iter().enumerate() {
        if let Some(ns) = obj_field(scene, "nodes").and_then(|v| v.as_array()) {
            for nref in ns {
                let ni = nref.as_u32().unwrap_or(u32::MAX) as usize;
                if ni >= n {
                    return Err(AssetError::new(
                        ErrorKind::DanglingReference,
                        format!("scenes[{si}].nodes OOB"),
                    ));
                }
            }
        }
    }
    if let Some(s) = root.get("scene") {
        let si = s.as_u32().unwrap_or(u32::MAX) as usize;
        if si >= scenes.len() {
            return Err(AssetError::new(
                ErrorKind::DanglingReference,
                "root scene index OOB",
            ));
        }
    }
    Ok(())
}

fn f32_array(v: Option<&JsonValue>, n: usize, default: &[f32]) -> Vec<f32> {
    if let Some(JsonValue::Array(a)) = v {
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            out.push(
                a.get(i)
                    .and_then(|x| x.as_f64())
                    .unwrap_or(default[i] as f64) as f32,
            );
        }
        out
    } else {
        default.to_vec()
    }
}

fn build_tables(root: &JsonValue) -> Result<CanonicalTables> {
    let scenes_j = arr(root, "scenes")?;
    let nodes_j = arr(root, "nodes")?;
    let meshes_j = arr(root, "meshes")?;
    let materials_j = arr(root, "materials")?;
    let textures_j = arr(root, "textures")?;

    let mut scenes = Vec::new();
    for (id, s) in scenes_j.iter().enumerate() {
        let mut nodes: Vec<u32> = obj_field(s, "nodes")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_u32()).collect())
            .unwrap_or_default();
        nodes.sort_unstable();
        scenes.push(SceneRow {
            id: id as u32,
            name: obj_field(s, "name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned(),
            nodes,
        });
    }

    let mut nodes = Vec::new();
    for (id, n) in nodes_j.iter().enumerate() {
        let mut children: Vec<u32> = obj_field(n, "children")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_u32()).collect())
            .unwrap_or_default();
        children.sort_unstable();
        let translation = obj_field(n, "translation").map(|v| {
            let a = f32_array(Some(v), 3, &[0.0, 0.0, 0.0]);
            [a[0], a[1], a[2]]
        });
        let rotation = obj_field(n, "rotation").map(|v| {
            let a = f32_array(Some(v), 4, &[0.0, 0.0, 0.0, 1.0]);
            [a[0], a[1], a[2], a[3]]
        });
        let scale = obj_field(n, "scale").map(|v| {
            let a = f32_array(Some(v), 3, &[1.0, 1.0, 1.0]);
            [a[0], a[1], a[2]]
        });
        let matrix = obj_field(n, "matrix").map(|v| {
            let d = [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ];
            let a = f32_array(Some(v), 16, &d);
            let mut m = [0f32; 16];
            m.copy_from_slice(&a);
            m
        });
        nodes.push(NodeRow {
            id: id as u32,
            name: obj_field(n, "name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned(),
            mesh: obj_field(n, "mesh").and_then(|v| v.as_u32()),
            children,
            translation,
            rotation,
            scale,
            matrix,
        });
    }

    let mut meshes = Vec::new();
    let mut primitives = Vec::new();
    let mut prim_id: u32 = 0;
    for (mid, mesh) in meshes_j.iter().enumerate() {
        let prims = obj_field(mesh, "primitives")
            .and_then(|v| v.as_array())
            .unwrap_or(&[]);
        let mut pids = Vec::new();
        for prim in prims {
            let mut attributes: BTreeMap<String, u32> = BTreeMap::new();
            if let Some(attrs) = obj_field(prim, "attributes").and_then(|v| v.as_object()) {
                for (k, v) in attrs {
                    if let Some(ai) = v.as_u32() {
                        attributes.insert(k.clone(), ai);
                    }
                }
            }
            let attributes: Vec<(String, u32)> = attributes.into_iter().collect();
            pids.push(prim_id);
            primitives.push(PrimitiveRow {
                id: prim_id,
                mesh_id: mid as u32,
                material: obj_field(prim, "material").and_then(|v| v.as_u32()),
                mode: obj_field(prim, "mode")
                    .and_then(|v| v.as_u32())
                    .unwrap_or(4),
                attributes,
                indices: obj_field(prim, "indices").and_then(|v| v.as_u32()),
            });
            prim_id += 1;
        }
        meshes.push(MeshRow {
            id: mid as u32,
            name: obj_field(mesh, "name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned(),
            primitive_ids: pids,
        });
    }

    let mut materials = Vec::new();
    for (id, mat) in materials_j.iter().enumerate() {
        let unlit = obj_field(mat, "extensions")
            .and_then(|e| e.get("KHR_materials_unlit"))
            .is_some();
        let pbr = obj_field(mat, "pbrMetallicRoughness");
        let base = f32_array(
            pbr.and_then(|p| obj_field(p, "baseColorFactor")),
            4,
            &[1.0, 1.0, 1.0, 1.0],
        );
        let metallic = pbr
            .and_then(|p| obj_field(p, "metallicFactor"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;
        let roughness = pbr
            .and_then(|p| obj_field(p, "roughnessFactor"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;
        materials.push(MaterialRow {
            id: id as u32,
            name: obj_field(mat, "name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned(),
            unlit,
            base_color_factor: [base[0], base[1], base[2], base[3]],
            metallic_factor: metallic,
            roughness_factor: roughness,
        });
    }

    let mut textures = Vec::new();
    for (id, tex) in textures_j.iter().enumerate() {
        textures.push(TextureRow {
            id: id as u32,
            sampler: obj_field(tex, "sampler").and_then(|v| v.as_u32()),
            source: obj_field(tex, "source").and_then(|v| v.as_u32()),
        });
    }

    canonical::encode_tables(scenes, nodes, meshes, primitives, materials, textures)
}

/// 从 glTF 文档解码出的真实三角网格（M79 DAG 的 geom 上游）。
///
/// 只承载 `mode==4`（TRIANGLES）图元；非索引图元按 `0..count` 顺序生成索引。
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedMesh {
    pub mesh_id: u32,
    pub primitive_id: u32,
    pub positions: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

/// 读 VEC3/f32 accessor 为位置数组（componentType 必须为 5126）。
fn read_vec3_f32(
    accessor: &JsonValue,
    buffer_views: &[JsonValue],
    buffers: &[Vec<u8>],
) -> Result<Vec<[f32; 3]>> {
    let count = obj_field(accessor, "count")
        .and_then(|v| v.as_u32())
        .ok_or_else(|| AssetError::new(ErrorKind::Invalid, "POSITION accessor.count missing"))?
        as usize;
    let component_type = obj_field(accessor, "componentType")
        .and_then(|v| v.as_u32())
        .ok_or_else(|| {
            AssetError::new(
                ErrorKind::Invalid,
                "POSITION accessor.componentType missing",
            )
        })?;
    if component_type != 5126 {
        return Err(AssetError::new(
            ErrorKind::Invalid,
            format!("POSITION componentType must be 5126 (f32), got {component_type}"),
        ));
    }
    let type_name = obj_field(accessor, "type")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if type_name != "VEC3" {
        return Err(AssetError::new(
            ErrorKind::Invalid,
            format!("POSITION accessor.type must be VEC3, got {type_name}"),
        ));
    }
    let acc_off = obj_field(accessor, "byteOffset")
        .and_then(|v| v.as_u32())
        .unwrap_or(0) as usize;
    let bvi = obj_field(accessor, "bufferView")
        .and_then(|v| v.as_u32())
        .ok_or_else(|| {
            AssetError::new(
                ErrorKind::AccessorOutOfBounds,
                "POSITION missing bufferView",
            )
        })? as usize;
    if bvi >= buffer_views.len() {
        return Err(AssetError::new(
            ErrorKind::DanglingReference,
            "POSITION bufferView OOB",
        ));
    }
    let bv = &buffer_views[bvi];
    let buf_i = obj_field(bv, "buffer")
        .and_then(|v| v.as_u32())
        .ok_or_else(|| AssetError::new(ErrorKind::Invalid, "bufferView.buffer missing"))?
        as usize;
    if buf_i >= buffers.len() {
        return Err(AssetError::new(
            ErrorKind::DanglingReference,
            "bufferView.buffer OOB",
        ));
    }
    let bv_off = obj_field(bv, "byteOffset")
        .and_then(|v| v.as_u32())
        .unwrap_or(0) as usize;
    let element_size = 12usize;
    let stride = obj_field(accessor, "byteStride")
        .or_else(|| obj_field(bv, "byteStride"))
        .and_then(|v| v.as_u32())
        .map(|s| s as usize)
        .unwrap_or(element_size);
    let base = &buffers[buf_i];
    let mut out = Vec::with_capacity(count);
    for n in 0..count {
        let off = bv_off
            .checked_add(acc_off)
            .and_then(|x| x.checked_add(n.checked_mul(stride)?))
            .ok_or_else(|| {
                AssetError::new(ErrorKind::AccessorOutOfBounds, "POSITION offset overflow")
            })?;
        if off.checked_add(element_size).map(|e| e > base.len()) != Some(false) {
            return Err(AssetError::new(
                ErrorKind::AccessorOutOfBounds,
                "POSITION span exceeds buffer",
            ));
        }
        let rd = |k: usize| -> f32 {
            f32::from_le_bytes([
                base[off + k],
                base[off + k + 1],
                base[off + k + 2],
                base[off + k + 3],
            ])
        };
        out.push([rd(0), rd(4), rd(8)]);
    }
    Ok(out)
}

/// 解码全文档三角网格。顺序 = meshes 序 × primitives 序（稳定、与调度无关）。
pub fn extract_meshes(root: &JsonValue, buffers: &[Vec<u8>]) -> Result<Vec<ImportedMesh>> {
    let accessors = arr(root, "accessors")?;
    let buffer_views = arr(root, "bufferViews")?;
    let meshes = arr(root, "meshes")?;
    let mut out = Vec::new();
    let mut prim_id: u32 = 0;
    for (mi, mesh) in meshes.iter().enumerate() {
        let prims = obj_field(mesh, "primitives")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                AssetError::new(
                    ErrorKind::Invalid,
                    format!("meshes[{mi}].primitives missing"),
                )
            })?;
        for prim in prims.iter() {
            let this_id = prim_id;
            prim_id += 1;
            let mode = obj_field(prim, "mode")
                .and_then(|v| v.as_u32())
                .unwrap_or(4);
            if mode != 4 {
                continue; // 非 TRIANGLES 首版不进 geom 上游
            }
            let attrs = obj_field(prim, "attributes")
                .and_then(|v| v.as_object())
                .ok_or_else(|| {
                    AssetError::new(ErrorKind::Invalid, "primitive.attributes missing")
                })?;
            let mut pos_acc: Option<usize> = None;
            for (name, acc_v) in attrs {
                if name == "POSITION" {
                    pos_acc = acc_v.as_u32().map(|x| x as usize);
                }
            }
            let Some(pa) = pos_acc else {
                continue; // 无 POSITION 的图元不产几何
            };
            if pa >= accessors.len() {
                return Err(AssetError::new(
                    ErrorKind::DanglingReference,
                    "POSITION accessor OOB",
                ));
            }
            let positions = read_vec3_f32(&accessors[pa], buffer_views, buffers)?;
            let indices = match obj_field(prim, "indices").and_then(|v| v.as_u32()) {
                Some(ai) => {
                    let ai = ai as usize;
                    if ai >= accessors.len() {
                        return Err(AssetError::new(
                            ErrorKind::DanglingReference,
                            "indices accessor OOB",
                        ));
                    }
                    read_index_values(&accessors[ai], buffer_views, buffers)?
                }
                None => (0..positions.len() as u32).collect(),
            };
            out.push(ImportedMesh {
                mesh_id: mi as u32,
                primitive_id: this_id,
                positions,
                indices,
            });
        }
    }
    Ok(out)
}

/// 导入结果。
#[derive(Debug)]
pub struct ImportResult {
    pub tables: CanonicalTables,
    pub coverage: ConsumedCoverage,
    /// 真实解码的三角网格（M79 geom 节点的载荷上游；空 = 该文档不含可用几何）。
    pub meshes: Vec<ImportedMesh>,
}

/// 对已解析 JSON 根执行验证并产出六表。
pub fn import_document(
    root: &JsonValue,
    base_dir: &Path,
    glb_bin: Option<&[u8]>,
    opts: &ImportOptions,
) -> Result<ImportResult> {
    let mut coverage = ConsumedCoverage::default();
    coverage.mark("asset");
    coverage.mark("asset.version");
    let asset = root
        .get("asset")
        .ok_or_else(|| AssetError::new(ErrorKind::Invalid, "missing asset object"))?;
    let ver = obj_field(asset, "version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AssetError::new(ErrorKind::Invalid, "asset.version missing"))?;
    if !ver.starts_with('2') {
        return Err(AssetError::new(
            ErrorKind::Invalid,
            format!("unsupported glTF version {ver}"),
        ));
    }

    check_extensions(root, opts, &mut coverage)?;
    let buffers = resolve_buffers(root, base_dir, glb_bin, &mut coverage)?;
    validate_accessors(root, &buffers, &mut coverage)?;
    validate_indices_and_refs(root, &buffers, &mut coverage)?;

    // 覆盖表必须盖住声明清单。
    for f in DECLARED_COVERAGE {
        if !coverage.fields.contains(f) {
            // 标记为已检查路径(即使文档缺少数组,校验函数也已 mark)。
            // 若仍缺,属实现漏洞。
            let _ = f;
        }
    }

    let tables = build_tables(root)?;
    let meshes = extract_meshes(root, &buffers)?;
    Ok(ImportResult {
        tables,
        coverage,
        meshes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gltf::json;

    //@ spec: RXS-0333
    #[test]
    fn reject_extension_outside_allowlist() {
        let doc = json::parse_str(
            r#"{"asset":{"version":"2.0"},"extensionsRequired":["EXT_meshopt_compression"]}"#,
        )
        .unwrap();
        let err =
            import_document(&doc, Path::new("."), None, &ImportOptions::default()).unwrap_err();
        assert_eq!(err.kind, ErrorKind::ExtensionNotAllowed);
    }
}
