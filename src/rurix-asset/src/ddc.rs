//! AP-DDC：内容寻址派生数据缓存（RXS-0343）。
//!
//! //@ spec: RXS-0343

use crate::canon::{self, Value};
use crate::error::{AssetError, ErrorKind, Result};
use rurix_pkg::sha256;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub const PREIMAGE_DOMAIN: &[u8] = b"rurix-ddc-artifact-v1\0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DdcKey(pub [u8; 32]);

impl DdcKey {
    pub fn hex(&self) -> String {
        sha256::hex(&self.0)
    }

    pub fn from_hex(s: &str) -> Result<Self> {
        if s.len() != 64 {
            return Err(AssetError::new(ErrorKind::Invalid, "ddc key hex len"));
        }
        let mut out = [0u8; 32];
        for i in 0..32 {
            out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16)
                .map_err(|_| AssetError::new(ErrorKind::Invalid, "ddc key hex parse"))?;
        }
        Ok(DdcKey(out))
    }
}

/// 九段 preimage 输入（顺序冻结）。
#[derive(Debug, Clone)]
pub struct PreimageSegments {
    pub source_set: Value,
    pub dependency_keys: Value,
    pub import_recipe: Value,
    pub cook_profile: Value,
    pub tool_chain: Value,
    pub schema_set: Value,
    pub abi_set: Value,
    pub artifact_kind: Value,
    pub output_id: Value,
}

impl PreimageSegments {
    pub fn as_array(&self) -> [&Value; 9] {
        [
            &self.source_set,
            &self.dependency_keys,
            &self.import_recipe,
            &self.cook_profile,
            &self.tool_chain,
            &self.schema_set,
            &self.abi_set,
            &self.artifact_kind,
            &self.output_id,
        ]
    }
}

pub fn compute_key(segs: &PreimageSegments) -> Result<DdcKey> {
    let mut h = sha256::Sha256::new();
    h.update(PREIMAGE_DOMAIN);
    for seg in segs.as_array() {
        let canon = canon::encode_cbor(seg)?;
        h.update(&(canon.len() as u64).to_le_bytes());
        h.update(&canon);
    }
    Ok(DdcKey(h.finalize()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GetMiss {
    Absent,
    Corruption { detail: String },
}

#[derive(Debug)]
pub enum PutError {
    KeyCollision,
    Io(AssetError),
}

impl From<AssetError> for PutError {
    fn from(e: AssetError) -> Self {
        PutError::Io(e)
    }
}

pub struct Ddc {
    root: PathBuf,
    seq: u64,
}

impl Ddc {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(root.join("objects"))?;
        fs::create_dir_all(root.join("meta"))?;
        fs::create_dir_all(root.join("tmp"))?;
        Ok(Self { root, seq: 0 })
    }

    fn object_path(&self, key: &DdcKey) -> PathBuf {
        let hx = key.hex();
        self.root.join("objects").join(&hx[..2]).join(&hx)
    }

    fn meta_path(&self, key: &DdcKey) -> PathBuf {
        let hx = key.hex();
        self.root
            .join("meta")
            .join(&hx[..2])
            .join(format!("{hx}.rxap"))
    }

    pub fn put(
        &mut self,
        key: &DdcKey,
        payload: &[u8],
        meta_envelope: &[u8],
    ) -> std::result::Result<(), PutError> {
        let obj = self.object_path(key);
        let meta = self.meta_path(key);
        if obj.exists() {
            let existing = fs::read(&obj).map_err(|e| PutError::Io(e.into()))?;
            if existing != payload {
                return Err(PutError::KeyCollision);
            }
            // idempotent same payload
            return Ok(());
        }
        if let Some(parent) = obj.parent() {
            fs::create_dir_all(parent).map_err(|e| PutError::Io(e.into()))?;
        }
        if let Some(parent) = meta.parent() {
            fs::create_dir_all(parent).map_err(|e| PutError::Io(e.into()))?;
        }
        self.seq += 1;
        let tmp = self
            .root
            .join("tmp")
            .join(format!("{}_{}", std::process::id(), self.seq));
        {
            let mut f = fs::File::create(&tmp).map_err(|e| PutError::Io(e.into()))?;
            f.write_all(payload).map_err(|e| PutError::Io(e.into()))?;
            f.sync_all().map_err(|e| PutError::Io(e.into()))?;
        }
        fs::rename(&tmp, &obj).map_err(|e| PutError::Io(e.into()))?;
        fs::write(&meta, meta_envelope).map_err(|e| PutError::Io(e.into()))?;
        Ok(())
    }

    pub fn get(&self, key: &DdcKey) -> std::result::Result<Vec<u8>, GetMiss> {
        let obj = self.object_path(key);
        let meta = self.meta_path(key);
        if !obj.exists() || !meta.exists() {
            return Err(GetMiss::Absent);
        }
        let payload = fs::read(&obj).map_err(|e| GetMiss::Corruption {
            detail: e.to_string(),
        })?;
        let meta_bytes = fs::read(&meta).map_err(|e| GetMiss::Corruption {
            detail: e.to_string(),
        })?;
        let env = canon::decode_envelope(&meta_bytes).map_err(|e| GetMiss::Corruption {
            detail: e.to_string(),
        })?;
        // meta payload encodes byte_len + payload_digest as CBOR map
        let v = canon::decode_cbor(&env.payload).map_err(|e| GetMiss::Corruption {
            detail: e.to_string(),
        })?;
        let Value::Map(m) = v else {
            return Err(GetMiss::Corruption {
                detail: "meta not map".into(),
            });
        };
        let Some(Value::Int(len)) = m.get(&1) else {
            return Err(GetMiss::Corruption {
                detail: "meta missing len".into(),
            });
        };
        let Some(Value::Bytes(dig)) = m.get(&2) else {
            return Err(GetMiss::Corruption {
                detail: "meta missing digest".into(),
            });
        };
        if *len as usize != payload.len() {
            return Err(GetMiss::Corruption {
                detail: "byte_len mismatch".into(),
            });
        }
        let got = sha256::digest(&payload);
        if dig.as_slice() != got.as_slice() {
            return Err(GetMiss::Corruption {
                detail: "payload digest mismatch".into(),
            });
        }
        Ok(payload)
    }

    pub fn evict(&self, key: &DdcKey) -> Result<()> {
        let obj = self.object_path(key);
        let meta = self.meta_path(key);
        if obj.exists() {
            fs::remove_file(obj)?;
        }
        if meta.exists() {
            fs::remove_file(meta)?;
        }
        Ok(())
    }
}

/// 构造 DerivedArtifact 风格 meta envelope（schema_id=80）。
pub fn make_meta_envelope(payload: &[u8]) -> Result<Vec<u8>> {
    let dig = sha256::digest(payload);
    let v = Value::map_of([
        (1, Value::Int(payload.len() as i64)),
        (2, Value::Bytes(dig.to_vec())),
    ])?;
    let sd = canon::schema_digest_for("derived_artifact_meta", 80, 1, 0);
    canon::wrap_value(80, 1, 0, sd, &v)
}

/// 测试/smoke 用最小九段。
pub fn demo_segments(tag: &str) -> Result<PreimageSegments> {
    Ok(PreimageSegments {
        source_set: Value::map_of([(1, Value::text_ascii("src")?), (2, Value::text_ascii(tag)?)])?,
        dependency_keys: Value::Array(vec![Value::text_ascii("dep0")?]),
        import_recipe: Value::map_of([(1, Value::text_ascii("recipe")?)])?,
        cook_profile: Value::map_of([(1, Value::text_ascii("profile")?)])?,
        tool_chain: Value::map_of([
            (1, Value::text_ascii("tool")?),
            (2, Value::text_ascii("1.0.0")?),
        ])?,
        schema_set: Value::Array(vec![Value::text_ascii("schema.v1")?]),
        abi_set: Value::Array(vec![Value::text_ascii("abi.v1")?]),
        artifact_kind: Value::text_ascii("demo.payload")?,
        output_id: Value::text_ascii("out0")?,
    })
}

pub fn mutate_segment(base: &PreimageSegments, which: usize) -> Result<PreimageSegments> {
    let mut s = base.clone();
    let flip = Value::text_ascii("MUTATED")?;
    match which {
        0 => s.source_set = Value::map_of([(1, flip)])?,
        1 => s.dependency_keys = Value::Array(vec![flip]),
        2 => s.import_recipe = Value::map_of([(1, flip)])?,
        3 => s.cook_profile = Value::map_of([(1, flip)])?,
        4 => s.tool_chain = Value::map_of([(1, flip)])?,
        5 => s.schema_set = Value::Array(vec![flip]),
        6 => s.abi_set = Value::Array(vec![flip]),
        7 => s.artifact_kind = flip,
        8 => s.output_id = flip,
        _ => {
            return Err(AssetError::new(ErrorKind::Invalid, "bad segment idx"));
        }
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::thread;

    #[test]
    fn same_preimage_same_key() {
        let s = demo_segments("a").unwrap();
        assert_eq!(compute_key(&s).unwrap(), compute_key(&s).unwrap());
    }

    #[test]
    fn nine_mutations_flip() {
        let base = demo_segments("a").unwrap();
        let k0 = compute_key(&base).unwrap();
        for i in 0..9 {
            let m = mutate_segment(&base, i).unwrap();
            assert_ne!(compute_key(&m).unwrap(), k0, "seg {i}");
        }
    }

    #[test]
    fn put_get_and_corruption() {
        let dir = std::env::temp_dir().join(format!("ddc_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let mut ddc = Ddc::open(&dir).unwrap();
        let segs = demo_segments("x").unwrap();
        let key = compute_key(&segs).unwrap();
        let payload = b"hello-ddc-payload";
        let meta = make_meta_envelope(payload).unwrap();
        ddc.put(&key, payload, &meta).unwrap();
        assert_eq!(ddc.get(&key).unwrap(), payload);

        // bitflip
        let p = ddc.object_path(&key);
        let mut bytes = fs::read(&p).unwrap();
        bytes[0] ^= 0xff;
        fs::write(&p, &bytes).unwrap();
        match ddc.get(&key) {
            Err(GetMiss::Corruption { .. }) => {}
            other => panic!("expected corruption, got {other:?}"),
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn concurrent_same_key() {
        let dir = std::env::temp_dir().join(format!("ddc_conc_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let ddc = Arc::new(Mutex::new(Ddc::open(&dir).unwrap()));
        let segs = demo_segments("c").unwrap();
        let key = compute_key(&segs).unwrap();
        let payload = b"same";
        let meta = make_meta_envelope(payload).unwrap();
        let mut handles = vec![];
        for _ in 0..4 {
            let ddc = Arc::clone(&ddc);
            let key = key.clone();
            let meta = meta.clone();
            handles.push(thread::spawn(move || {
                let mut g = ddc.lock().unwrap();
                g.put(&key, payload, &meta).unwrap();
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        let g = ddc.lock().unwrap();
        assert_eq!(g.get(&key).unwrap(), payload);
        let _ = fs::remove_dir_all(&dir);
    }
}
