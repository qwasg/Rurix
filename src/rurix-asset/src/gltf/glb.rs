//! GLB 容器解析(RXS-0333)——magic/version/chunk 布局全校验。

use crate::error::{AssetError, ErrorKind, Result};
use crate::gltf::json::{self, JsonValue};

const GLB_MAGIC: u32 = 0x4654_6C67; // 'glTF' LE
const GLB_VERSION: u32 = 2;
const CHUNK_JSON: u32 = 0x4E4F_534A; // 'JSON'
const CHUNK_BIN: u32 = 0x004E_4942; // 'BIN\0'

/// 已拆开的 GLB 内容。
#[derive(Debug)]
pub struct GlbDocument {
    pub json: JsonValue,
    pub bin: Option<Vec<u8>>,
}

fn read_u32(bytes: &[u8], off: usize) -> Result<u32> {
    let slice = bytes
        .get(off..off + 4)
        .ok_or_else(|| AssetError::new(ErrorKind::GlbContainer, "GLB truncated reading u32"))?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

/// 解析 GLB 字节 → JSON + 可选 BIN。
pub fn parse_glb(bytes: &[u8]) -> Result<GlbDocument> {
    if bytes.len() < 12 {
        return Err(AssetError::new(
            ErrorKind::GlbContainer,
            "GLB shorter than 12-byte header",
        ));
    }
    let magic = read_u32(bytes, 0)?;
    if magic != GLB_MAGIC {
        return Err(AssetError::new(
            ErrorKind::GlbContainer,
            format!("bad GLB magic 0x{magic:08X}"),
        ));
    }
    let version = read_u32(bytes, 4)?;
    if version != GLB_VERSION {
        return Err(AssetError::new(
            ErrorKind::GlbContainer,
            format!("unsupported GLB version {version}"),
        ));
    }
    let length = read_u32(bytes, 8)? as usize;
    if length != bytes.len() {
        return Err(AssetError::new(
            ErrorKind::GlbContainer,
            format!("GLB length field {length} != actual {}", bytes.len()),
        ));
    }

    let mut off = 12;
    let mut json_chunk: Option<Vec<u8>> = None;
    let mut bin_chunk: Option<Vec<u8>> = None;
    let mut saw_json = false;

    while off < bytes.len() {
        if off + 8 > bytes.len() {
            return Err(AssetError::new(
                ErrorKind::GlbContainer,
                "truncated chunk header",
            ));
        }
        let chunk_len = read_u32(bytes, off)? as usize;
        let chunk_ty = read_u32(bytes, off + 4)?;
        off += 8;
        if off + chunk_len > bytes.len() {
            return Err(AssetError::new(
                ErrorKind::GlbContainer,
                "chunk data exceeds file",
            ));
        }
        let data = &bytes[off..off + chunk_len];
        // 对齐:chunk 总长(含 8B 头)应对齐到 4;数据区本身长度已由标准填充保证。
        if chunk_len % 4 != 0 {
            return Err(AssetError::new(
                ErrorKind::GlbContainer,
                format!("chunk length {chunk_len} not 4-byte aligned"),
            ));
        }
        match chunk_ty {
            CHUNK_JSON => {
                if saw_json {
                    return Err(AssetError::new(
                        ErrorKind::GlbContainer,
                        "multiple JSON chunks",
                    ));
                }
                saw_json = true;
                // JSON chunk 以空格填充至 4 对齐;剥离尾部 0x20。
                let mut end = data.len();
                while end > 0 && data[end - 1] == b' ' {
                    end -= 1;
                }
                json_chunk = Some(data[..end].to_vec());
            }
            CHUNK_BIN => {
                if bin_chunk.is_some() {
                    return Err(AssetError::new(
                        ErrorKind::GlbContainer,
                        "multiple BIN chunks",
                    ));
                }
                // BIN 以 0 填充;保留声明长度(含填充)供 buffer.byteLength 对齐,
                // 但语义数据使用完整 chunk 字节(glTF 允许 padding 计入 buffer)。
                bin_chunk = Some(data.to_vec());
            }
            _ => {
                return Err(AssetError::new(
                    ErrorKind::GlbContainer,
                    format!("unknown GLB chunk type 0x{chunk_ty:08X}"),
                ));
            }
        }
        off += chunk_len;
    }

    if off != bytes.len() {
        return Err(AssetError::new(
            ErrorKind::GlbContainer,
            "trailing bytes after final chunk",
        ));
    }
    let json_bytes = json_chunk.ok_or_else(|| {
        AssetError::new(ErrorKind::GlbContainer, "GLB missing JSON chunk")
    })?;
    // JSON 必须是第一 chunk(glTF 2.0 §2.1)。
    let first_ty = read_u32(bytes, 12 + 4)?;
    if first_ty != CHUNK_JSON {
        return Err(AssetError::new(
            ErrorKind::GlbContainer,
            "first GLB chunk must be JSON",
        ));
    }

    let json = json::parse_bytes(&json_bytes)?;
    Ok(GlbDocument {
        json,
        bin: bin_chunk,
    })
}

/// 组装最小合法 GLB(测试/fixture 辅助)。
pub fn build_glb(json_text: &str, bin: Option<&[u8]>) -> Result<Vec<u8>> {
    let mut json_bytes = json_text.as_bytes().to_vec();
    while json_bytes.len() % 4 != 0 {
        json_bytes.push(b' ');
    }
    let mut bin_bytes = bin.unwrap_or(&[]).to_vec();
    let has_bin = bin.is_some();
    if has_bin {
        while bin_bytes.len() % 4 != 0 {
            bin_bytes.push(0);
        }
    }
    let mut out = Vec::new();
    let total = 12
        + 8
        + json_bytes.len()
        + if has_bin {
            8 + bin_bytes.len()
        } else {
            0
        };
    out.extend_from_slice(&GLB_MAGIC.to_le_bytes());
    out.extend_from_slice(&GLB_VERSION.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&CHUNK_JSON.to_le_bytes());
    out.extend_from_slice(&json_bytes);
    if has_bin {
        out.extend_from_slice(&(bin_bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(&CHUNK_BIN.to_le_bytes());
        out.extend_from_slice(&bin_bytes);
    }
    debug_assert_eq!(out.len(), total);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0333
    #[test]
    fn roundtrip_minimal_glb() {
        let json = r#"{"asset":{"version":"2.0"}}"#;
        let bytes = build_glb(json, Some(&[1, 2, 3])).unwrap();
        let doc = parse_glb(&bytes).unwrap();
        assert!(doc.json.get("asset").is_some());
        assert_eq!(doc.bin.as_ref().unwrap()[..3], [1, 2, 3]);
    }

    //@ spec: RXS-0333
    #[test]
    fn rejects_bad_magic() {
        let mut bytes = build_glb(r#"{"asset":{"version":"2.0"}}"#, None).unwrap();
        bytes[0] = b'X';
        assert_eq!(parse_glb(&bytes).unwrap_err().kind, ErrorKind::GlbContainer);
    }
}
