//! AP-CANON：deterministic CBOR 冻结子集 + RXAP envelope（RXS-0335）。
//!
//! //@ spec: RXS-0335

use crate::error::{AssetError, ErrorKind, Result};
use rurix_pkg::sha256;
use std::collections::BTreeMap;

/// Canonicalizer 版本（进 schema digest）。
pub const CANONICALIZER_VERSION: u32 = 1;

/// Envelope magic `RXAP`。
pub const RXAP_MAGIC: &[u8; 4] = b"RXAP";

/// 确定性值（禁浮点；字符串限 ASCII 可打印）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Bytes(Vec<u8>),
    Text(String),
    Array(Vec<Value>),
    /// map key = 非负整数 field-ID，按 ID 排序编码。
    Map(BTreeMap<u64, Value>),
}

impl Value {
    pub fn text_ascii(s: impl Into<String>) -> Result<Self> {
        let s = s.into();
        validate_ascii_printable(&s)?;
        Ok(Value::Text(s))
    }

    pub fn map_of(pairs: impl IntoIterator<Item = (u64, Value)>) -> Result<Self> {
        let mut m = BTreeMap::new();
        for (k, v) in pairs {
            if m.insert(k, v).is_some() {
                return Err(AssetError::new(
                    ErrorKind::CanonInvalid,
                    format!("duplicate field-id {k}"),
                ));
            }
        }
        Ok(Value::Map(m))
    }
}

fn validate_ascii_printable(s: &str) -> Result<()> {
    for b in s.bytes() {
        if !(0x20..=0x7e).contains(&b) {
            return Err(AssetError::new(
                ErrorKind::CanonInvalid,
                "string must be ASCII printable (Unicode/NFC not open)",
            ));
        }
    }
    Ok(())
}

/// 编码 Value → deterministic CBOR bytes。
pub fn encode_cbor(v: &Value) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    encode_into(v, &mut out)?;
    Ok(out)
}

fn encode_into(v: &Value, out: &mut Vec<u8>) -> Result<()> {
    match v {
        Value::Null => out.push(0xf6),
        Value::Bool(false) => out.push(0xf4),
        Value::Bool(true) => out.push(0xf5),
        Value::Int(n) => encode_int(*n, out),
        Value::Bytes(b) => {
            encode_len_head(2, b.len() as u64, out);
            out.extend_from_slice(b);
        }
        Value::Text(s) => {
            validate_ascii_printable(s)?;
            encode_len_head(3, s.len() as u64, out);
            out.extend_from_slice(s.as_bytes());
        }
        Value::Array(a) => {
            encode_len_head(4, a.len() as u64, out);
            for e in a {
                encode_into(e, out)?;
            }
        }
        Value::Map(m) => {
            encode_len_head(5, m.len() as u64, out);
            for (k, val) in m {
                encode_uint(*k, out);
                encode_into(val, out)?;
            }
        }
    }
    Ok(())
}

fn encode_len_head(major: u8, n: u64, out: &mut Vec<u8>) {
    let hi = major << 5;
    if n < 24 {
        out.push(hi | (n as u8));
    } else if n <= u8::MAX as u64 {
        out.push(hi | 24);
        out.push(n as u8);
    } else if n <= u16::MAX as u64 {
        out.push(hi | 25);
        out.extend_from_slice(&(n as u16).to_be_bytes());
    } else if n <= u32::MAX as u64 {
        out.push(hi | 26);
        out.extend_from_slice(&(n as u32).to_be_bytes());
    } else {
        out.push(hi | 27);
        out.extend_from_slice(&n.to_be_bytes());
    }
}

fn encode_uint(n: u64, out: &mut Vec<u8>) {
    encode_len_head(0, n, out);
}

fn encode_int(n: i64, out: &mut Vec<u8>) {
    if n >= 0 {
        encode_uint(n as u64, out);
    } else {
        // major 1: -1 - n
        let m = (-1i128 - n as i128) as u64;
        encode_len_head(1, m, out);
    }
}

/// 解码 CBOR（严格：禁 indefinite、禁非最短整数、禁浮点、禁重复 map key）。
pub fn decode_cbor(bytes: &[u8]) -> Result<Value> {
    let (v, rest) = decode_one(bytes)?;
    if !rest.is_empty() {
        return Err(AssetError::new(
            ErrorKind::CanonInvalid,
            "trailing bytes after CBOR value",
        ));
    }
    Ok(v)
}

fn decode_one(bytes: &[u8]) -> Result<(Value, &[u8])> {
    if bytes.is_empty() {
        return Err(AssetError::new(ErrorKind::CanonInvalid, "truncated CBOR"));
    }
    let b0 = bytes[0];
    let major = b0 >> 5;
    let ai = b0 & 0x1f;
    if ai == 31 {
        return Err(AssetError::new(
            ErrorKind::CanonInvalid,
            "indefinite-length forbidden",
        ));
    }
    let (n, rest) = read_ai(ai, &bytes[1..])?;
    match major {
        0 => Ok((Value::Int(n as i64), rest)),
        1 => {
            let v = -1i128 - n as i128;
            if v < i64::MIN as i128 {
                return Err(AssetError::new(ErrorKind::CanonInvalid, "int underflow"));
            }
            Ok((Value::Int(v as i64), rest))
        }
        2 => {
            let len = n as usize;
            if rest.len() < len {
                return Err(AssetError::new(ErrorKind::CanonInvalid, "bytes truncated"));
            }
            Ok((Value::Bytes(rest[..len].to_vec()), &rest[len..]))
        }
        3 => {
            let len = n as usize;
            if rest.len() < len {
                return Err(AssetError::new(ErrorKind::CanonInvalid, "text truncated"));
            }
            let s = std::str::from_utf8(&rest[..len])
                .map_err(|_| AssetError::new(ErrorKind::CanonInvalid, "text not utf-8"))?;
            validate_ascii_printable(s)?;
            Ok((Value::Text(s.to_string()), &rest[len..]))
        }
        4 => {
            let mut items = Vec::with_capacity(n as usize);
            let mut cur = rest;
            for _ in 0..n {
                let (v, r) = decode_one(cur)?;
                items.push(v);
                cur = r;
            }
            Ok((Value::Array(items), cur))
        }
        5 => {
            let mut m = BTreeMap::new();
            let mut cur = rest;
            let mut last_key: Option<u64> = None;
            for _ in 0..n {
                let (k, r1) = decode_uint_key(cur)?;
                if let Some(prev) = last_key
                    && k <= prev
                {
                    return Err(AssetError::new(
                        ErrorKind::CanonInvalid,
                        "map keys must be strictly increasing field-ids",
                    ));
                }
                last_key = Some(k);
                let (v, r2) = decode_one(r1)?;
                if m.insert(k, v).is_some() {
                    return Err(AssetError::new(
                        ErrorKind::CanonInvalid,
                        format!("duplicate field-id {k}"),
                    ));
                }
                cur = r2;
            }
            Ok((Value::Map(m), cur))
        }
        7 => match ai {
            20 => Ok((Value::Bool(false), rest)),
            21 => Ok((Value::Bool(true), rest)),
            22 => Ok((Value::Null, rest)),
            25..=27 => Err(AssetError::new(
                ErrorKind::CanonInvalid,
                "float not open in AP-CANON v1",
            )),
            _ => Err(AssetError::new(
                ErrorKind::CanonInvalid,
                format!("unsupported simple value ai={ai}"),
            )),
        },
        _ => Err(AssetError::new(
            ErrorKind::CanonInvalid,
            format!("unsupported major {major}"),
        )),
    }
}

fn decode_uint_key(bytes: &[u8]) -> Result<(u64, &[u8])> {
    if bytes.is_empty() {
        return Err(AssetError::new(
            ErrorKind::CanonInvalid,
            "truncated map key",
        ));
    }
    let b0 = bytes[0];
    if b0 >> 5 != 0 {
        return Err(AssetError::new(
            ErrorKind::CanonInvalid,
            "map key must be unsigned int field-id",
        ));
    }
    let ai = b0 & 0x1f;
    if ai == 31 {
        return Err(AssetError::new(
            ErrorKind::CanonInvalid,
            "indefinite-length forbidden",
        ));
    }
    read_ai(ai, &bytes[1..])
}

fn read_ai(ai: u8, rest: &[u8]) -> Result<(u64, &[u8])> {
    if ai < 24 {
        return Ok((ai as u64, rest));
    }
    match ai {
        24 => {
            if rest.is_empty() {
                return Err(AssetError::new(ErrorKind::CanonInvalid, "ai24 truncated"));
            }
            let n = rest[0] as u64;
            if n < 24 {
                return Err(AssetError::new(
                    ErrorKind::CanonInvalid,
                    "non-shortest integer encoding",
                ));
            }
            Ok((n, &rest[1..]))
        }
        25 => {
            if rest.len() < 2 {
                return Err(AssetError::new(ErrorKind::CanonInvalid, "ai25 truncated"));
            }
            let n = u16::from_be_bytes([rest[0], rest[1]]) as u64;
            if n <= u8::MAX as u64 {
                return Err(AssetError::new(
                    ErrorKind::CanonInvalid,
                    "non-shortest integer encoding",
                ));
            }
            Ok((n, &rest[2..]))
        }
        26 => {
            if rest.len() < 4 {
                return Err(AssetError::new(ErrorKind::CanonInvalid, "ai26 truncated"));
            }
            let n = u32::from_be_bytes([rest[0], rest[1], rest[2], rest[3]]) as u64;
            if n <= u16::MAX as u64 {
                return Err(AssetError::new(
                    ErrorKind::CanonInvalid,
                    "non-shortest integer encoding",
                ));
            }
            Ok((n, &rest[4..]))
        }
        27 => {
            if rest.len() < 8 {
                return Err(AssetError::new(ErrorKind::CanonInvalid, "ai27 truncated"));
            }
            let n = u64::from_be_bytes(rest[..8].try_into().unwrap());
            if n <= u32::MAX as u64 {
                return Err(AssetError::new(
                    ErrorKind::CanonInvalid,
                    "non-shortest integer encoding",
                ));
            }
            Ok((n, &rest[8..]))
        }
        _ => Err(AssetError::new(
            ErrorKind::CanonInvalid,
            format!("reserved ai={ai}"),
        )),
    }
}

/// RXAP envelope（全 LE，设计案 §3.1）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    pub schema_id: u32,
    pub major: u16,
    pub minor: u16,
    pub canonicalizer_version: u32,
    pub schema_digest: [u8; 32],
    pub payload: Vec<u8>,
}

pub fn encode_envelope(env: &Envelope) -> Result<Vec<u8>> {
    let payload_digest = sha256::digest(&env.payload);
    let mut out = Vec::with_capacity(4 + 4 + 2 + 2 + 4 + 8 + 32 + 32 + env.payload.len());
    out.extend_from_slice(RXAP_MAGIC);
    out.extend_from_slice(&env.schema_id.to_le_bytes());
    out.extend_from_slice(&env.major.to_le_bytes());
    out.extend_from_slice(&env.minor.to_le_bytes());
    out.extend_from_slice(&env.canonicalizer_version.to_le_bytes());
    out.extend_from_slice(&(env.payload.len() as u64).to_le_bytes());
    out.extend_from_slice(&env.schema_digest);
    out.extend_from_slice(&payload_digest);
    out.extend_from_slice(&env.payload);
    Ok(out)
}

pub fn decode_envelope(bytes: &[u8]) -> Result<Envelope> {
    const HDR: usize = 4 + 4 + 2 + 2 + 4 + 8 + 32 + 32;
    if bytes.len() < HDR {
        return Err(AssetError::new(
            ErrorKind::CanonInvalid,
            "RXAP envelope truncated",
        ));
    }
    if &bytes[0..4] != RXAP_MAGIC {
        return Err(AssetError::new(ErrorKind::CanonInvalid, "bad RXAP magic"));
    }
    let schema_id = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    let major = u16::from_le_bytes(bytes[8..10].try_into().unwrap());
    let minor = u16::from_le_bytes(bytes[10..12].try_into().unwrap());
    let canonicalizer_version = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
    let payload_len = u64::from_le_bytes(bytes[16..24].try_into().unwrap()) as usize;
    let mut schema_digest = [0u8; 32];
    schema_digest.copy_from_slice(&bytes[24..56]);
    let mut payload_digest = [0u8; 32];
    payload_digest.copy_from_slice(&bytes[56..88]);
    if bytes.len() != HDR + payload_len {
        return Err(AssetError::new(
            ErrorKind::CanonInvalid,
            "RXAP payload_len mismatch",
        ));
    }
    let payload = bytes[HDR..].to_vec();
    let got = sha256::digest(&payload);
    if got != payload_digest {
        return Err(AssetError::new(
            ErrorKind::CanonInvalid,
            "RXAP payload_digest mismatch",
        ));
    }
    Ok(Envelope {
        schema_id,
        major,
        minor,
        canonicalizer_version,
        schema_digest,
        payload,
    })
}

/// 编码 Value 并包 RXAP。
pub fn wrap_value(
    schema_id: u32,
    major: u16,
    minor: u16,
    schema_digest: [u8; 32],
    value: &Value,
) -> Result<Vec<u8>> {
    let payload = encode_cbor(value)?;
    encode_envelope(&Envelope {
        schema_id,
        major,
        minor,
        canonicalizer_version: CANONICALIZER_VERSION,
        schema_digest,
        payload,
    })
}

pub fn digest_bytes(bytes: &[u8]) -> [u8; 32] {
    sha256::digest(bytes)
}

pub fn hex_digest(bytes: &[u8]) -> String {
    sha256::hex_digest(bytes)
}

/// Schema digest = SHA-256(schema_id||major||minor||canonicalizer_version||name)。
pub fn schema_digest_for(name: &str, schema_id: u32, major: u16, minor: u16) -> [u8; 32] {
    let mut h = sha256::Sha256::new();
    h.update(&schema_id.to_le_bytes());
    h.update(&major.to_le_bytes());
    h.update(&minor.to_le_bytes());
    h.update(&CANONICALIZER_VERSION.to_le_bytes());
    h.update(name.as_bytes());
    h.finalize()
}

/// 校验 accept/*.rxap 可解码且 payload 可 roundtrip；reject/* 必须失败。
pub fn check_canon_corpus(
    accept_dir: &std::path::Path,
    reject_dir: &std::path::Path,
) -> Result<(usize, usize)> {
    let mut accept_n = 0usize;
    for ent in std::fs::read_dir(accept_dir)? {
        let p = ent?.path();
        if p.extension().and_then(|e| e.to_str()) != Some("rxap") {
            continue;
        }
        let bytes = std::fs::read(&p)?;
        let env = decode_envelope(&bytes)?;
        let v = decode_cbor(&env.payload)?;
        let again = encode_cbor(&v)?;
        if again != env.payload {
            return Err(AssetError::new(
                ErrorKind::CanonInvalid,
                format!("{} payload not canonical", p.display()),
            ));
        }
        accept_n += 1;
    }
    if accept_n < 6 {
        return Err(AssetError::new(
            ErrorKind::CanonInvalid,
            format!("expected >=6 accept fixtures, got {accept_n}"),
        ));
    }
    let mut reject_n = 0usize;
    for ent in std::fs::read_dir(reject_dir)? {
        let p = ent?.path();
        let bytes = std::fs::read(&p)?;
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let failed = if name.ends_with(".rxap") {
            decode_envelope(&bytes).is_err()
        } else {
            decode_cbor(&bytes).is_err()
        };
        if !failed {
            return Err(AssetError::new(
                ErrorKind::CanonInvalid,
                format!("reject fixture unexpectedly accepted: {name}"),
            ));
        }
        reject_n += 1;
    }
    if reject_n < 4 {
        return Err(AssetError::new(
            ErrorKind::CanonInvalid,
            format!("expected >=4 reject fixtures, got {reject_n}"),
        ));
    }
    Ok((accept_n, reject_n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_map_sorted() {
        let v = Value::map_of([(2, Value::Int(7)), (1, Value::text_ascii("hi").unwrap())]).unwrap();
        let enc = encode_cbor(&v).unwrap();
        let dec = decode_cbor(&enc).unwrap();
        assert_eq!(v, dec);
        // field 1 before 2 in encoding
        assert_eq!(enc[0] >> 5, 5);
    }

    #[test]
    fn reject_non_shortest() {
        // uint 1 encoded as 24+1 byte
        let bad = [0x18, 0x01];
        assert!(decode_cbor(&bad).is_err());
    }

    #[test]
    fn reject_non_ascii() {
        assert!(Value::text_ascii("你好").is_err());
    }

    #[test]
    fn envelope_roundtrip() {
        let v = Value::Int(42);
        let sd = schema_digest_for("test", 1, 1, 0);
        let bytes = wrap_value(1, 1, 0, sd, &v).unwrap();
        let env = decode_envelope(&bytes).unwrap();
        assert_eq!(env.schema_id, 1);
        assert_eq!(decode_cbor(&env.payload).unwrap(), v);
    }
}
