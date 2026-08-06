//! G8.2 M85 shader/PSO manifest v1(RXS-0317~0318;RFC-0019 §4.1)。
//!
//! 纯 host/safe 模块:`schema = "rurix.shader-manifest.v1"`;shader 记录八字段
//! 来自 reflection v1(零第二事实源);PSO 记录四字段来自 M30 collector(RXS-0314)。
//! canonical bytes 沿 RXS-0305 CanonW 律;`manifest_digest =
//! SHA-256("rurix.shader-manifest.v1\0" || body)`(canonical 含版本前缀时等价于
//! `SHA-256(完整 canonical)`,与 M29/M32 digest 体例一致)。
//!
//! merge/dedup/冲突/coverage 律见 RXS-0318;冲突与缺口 = typed [`ManifestError`]
//! (零新 RX 数字码;CLI 非零退出)。

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use rurix_pkg::sha256;

use crate::tooling::json_util as ju;

/// JSON schema 字面(RXS-0317)。
pub const MANIFEST_SCHEMA: &str = "rurix.shader-manifest.v1";
/// canonical / digest 域前缀(含 NUL)。
pub const MANIFEST_DOMAIN: &[u8] = b"rurix.shader-manifest.v1\0";

/// coverage 声明表 schema(输入侧显式声明;禁输出自证)。
pub const COVERAGE_SCHEMA: &str = "rurix.shader-manifest-coverage.v1";

// ═══════════════════════ 错误面(零新 RX 码) ═══════════════════════

/// manifest 装配 / merge / coverage 失败(typed Err;CLI 映射非零退出)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    /// 同键异 payload(RXS-0318 fail-closed)。
    Conflict {
        kind: &'static str,
        key_hex: String,
        differing_fields: Vec<String>,
    },
    /// coverage 缺口或多出未声明键。
    Coverage {
        missing: Vec<String>,
        extra: Vec<String>,
    },
    /// JSON / 字段形态非法。
    Malformed(String),
    /// I/O。
    Io(String),
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManifestError::Conflict {
                kind,
                key_hex,
                differing_fields,
            } => write!(
                f,
                "manifest conflict: {kind} key={key_hex} differing_fields=[{}]",
                differing_fields.join(", ")
            ),
            ManifestError::Coverage { missing, extra } => write!(
                f,
                "manifest coverage: missing=[{}] extra=[{}]",
                missing.join(", "),
                extra.join(", ")
            ),
            ManifestError::Malformed(d) => write!(f, "manifest malformed: {d}"),
            ManifestError::Io(d) => write!(f, "manifest io: {d}"),
        }
    }
}

impl std::error::Error for ManifestError {}

// ═══════════════════════ 记录模型 ═══════════════════════

/// shader 记录(RXS-0317 八字段闭集;键 = `pipeline_key`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaderRecord {
    pub entry: String,
    pub stage: String,
    pub interface_hash: [u8; 32],
    pub source_digest: [u8; 32],
    pub selected_profile_digest: [u8; 32],
    pub permutation_domain_digest: [u8; 32],
    pub variant_key: String,
    pub pipeline_key: [u8; 32],
}

/// PSO stage digest 元素(对齐 M30 collector `stage_digests[]`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageDigestEntry {
    pub stage_tag: u32,
    pub digest: [u8; 32],
}

/// PSO 记录(RXS-0317 / RXS-0314 字段位;键 = `pso_key`)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PsoRecord {
    pub pso_key: [u8; 32],
    pub kind_tag: u32,
    pub stage_digests: Vec<StageDigestEntry>,
    pub fixed_function_digest: [u8; 32],
}

/// per-unit / merged manifest。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Manifest {
    /// shader 段(键 = pipeline_key 字节序)。
    pub shaders: BTreeMap<[u8; 32], ShaderRecord>,
    /// PSO 段(键 = pso_key 字节序)。
    pub psos: BTreeMap<[u8; 32], PsoRecord>,
}

/// coverage 输入侧声明表(RXS-0318)。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CoverageTable {
    /// 预期 `(entry, variant_key, pipeline_key_hex)`。
    pub shaders: Vec<CoverageShader>,
    /// 预期 `pso_key_hex`。
    pub pso_keys: Vec<String>,
}

/// coverage 声明的一条 shader 预期。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageShader {
    pub entry: String,
    pub variant_key: String,
    pub pipeline_key: String,
}

// ═══════════════════════ CanonW ═══════════════════════

struct CanonW {
    buf: Vec<u8>,
}

impl CanonW {
    fn new() -> Self {
        CanonW { buf: Vec::new() }
    }
    fn u32v(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    fn strv(&mut self, s: &str) {
        self.u32v(u32::try_from(s.len()).unwrap_or(u32::MAX));
        self.buf.extend_from_slice(s.as_bytes());
    }
    fn bytes(&mut self, b: &[u8]) {
        self.buf.extend_from_slice(b);
    }
}

// ═══════════════════════ hex ═══════════════════════

fn hex_of(d: &[u8; 32]) -> String {
    sha256::hex(d)
}

fn parse_hex32(s: &str, field: &str) -> Result<[u8; 32], ManifestError> {
    let t = s.trim();
    if t.len() != 64 || !t.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ManifestError::Malformed(format!(
            "字段 `{field}` 须为 64 位小写/大写 hex digest,得 `{t}`"
        )));
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        let byte = u8::from_str_radix(&t[i * 2..i * 2 + 2], 16).map_err(|_| {
            ManifestError::Malformed(format!("字段 `{field}` hex 解析失败"))
        })?;
        out[i] = byte;
    }
    Ok(out)
}

fn json_escape(s: &str) -> String {
    ju::escape_json(s)
}

/// JSON 数组切片 → 顶层元素表。
fn split_json_top_level(slice: &str, open: char, close: char) -> Option<Vec<String>> {
    let body = slice.trim_start();
    let body = body.strip_prefix(open)?.strip_suffix(close)?;
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_str = false;
    let mut escaped = false;
    let mut depth = 0i32;
    for ch in body.chars() {
        if in_str {
            cur.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_str = false;
            }
            continue;
        }
        match ch {
            '"' => {
                in_str = true;
                cur.push(ch);
            }
            '{' | '[' => {
                depth += 1;
                cur.push(ch);
            }
            '}' | ']' => {
                depth -= 1;
                cur.push(ch);
            }
            ',' if depth == 0 => {
                out.push(cur.trim().to_owned());
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }
    let last = cur.trim();
    if !last.is_empty() {
        out.push(last.to_owned());
    }
    Some(out)
}

// ═══════════════════════ canonical / digest ═══════════════════════

impl Manifest {
    /// canonical bytes = 版本前缀 + shader 段(按 pipeline_key) + PSO 段(按 pso_key)。
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut w = CanonW::new();
        w.bytes(MANIFEST_DOMAIN);
        w.u32v(self.shaders.len() as u32);
        for r in self.shaders.values() {
            w.strv(&r.entry);
            w.strv(&r.stage);
            w.bytes(&r.interface_hash);
            w.bytes(&r.source_digest);
            w.bytes(&r.selected_profile_digest);
            w.bytes(&r.permutation_domain_digest);
            w.strv(&r.variant_key);
            w.bytes(&r.pipeline_key);
        }
        w.u32v(self.psos.len() as u32);
        for r in self.psos.values() {
            w.bytes(&r.pso_key);
            w.u32v(r.kind_tag);
            w.u32v(r.stage_digests.len() as u32);
            for sd in &r.stage_digests {
                w.u32v(sd.stage_tag);
                w.bytes(&sd.digest);
            }
            w.bytes(&r.fixed_function_digest);
        }
        w.buf
    }

    /// `manifest_digest = SHA-256(完整 canonical)` =
    /// `SHA-256("rurix.shader-manifest.v1\0" || body)`。
    pub fn digest(&self) -> [u8; 32] {
        sha256::digest(&self.canonical_bytes())
    }

    /// digest hex 展示面。
    pub fn digest_hex(&self) -> String {
        hex_of(&self.digest())
    }

    /// 确定性 JSON 产物(键序固定、UTF-8、LF;digest hex)。
    pub fn to_json(&self) -> String {
        let mut s = String::new();
        s.push_str("{\n");
        s.push_str(&format!("  \"schema\": \"{MANIFEST_SCHEMA}\",\n"));
        s.push_str(&format!(
            "  \"manifest_digest\": \"{}\",\n",
            self.digest_hex()
        ));
        s.push_str("  \"shaders\": [\n");
        let shaders: Vec<&ShaderRecord> = self.shaders.values().collect();
        for (i, r) in shaders.iter().enumerate() {
            s.push_str("    {\n");
            s.push_str(&format!(
                "      \"entry\": \"{}\",\n",
                json_escape(&r.entry)
            ));
            s.push_str(&format!(
                "      \"stage\": \"{}\",\n",
                json_escape(&r.stage)
            ));
            s.push_str(&format!(
                "      \"interface_hash\": \"{}\",\n",
                hex_of(&r.interface_hash)
            ));
            s.push_str(&format!(
                "      \"source_digest\": \"{}\",\n",
                hex_of(&r.source_digest)
            ));
            s.push_str(&format!(
                "      \"selected_profile_digest\": \"{}\",\n",
                hex_of(&r.selected_profile_digest)
            ));
            s.push_str(&format!(
                "      \"permutation_domain_digest\": \"{}\",\n",
                hex_of(&r.permutation_domain_digest)
            ));
            s.push_str(&format!(
                "      \"variant_key\": \"{}\",\n",
                json_escape(&r.variant_key)
            ));
            s.push_str(&format!(
                "      \"pipeline_key\": \"{}\"\n",
                hex_of(&r.pipeline_key)
            ));
            s.push_str(if i + 1 == shaders.len() {
                "    }\n"
            } else {
                "    },\n"
            });
        }
        s.push_str("  ],\n");
        s.push_str("  \"psos\": [\n");
        let psos: Vec<&PsoRecord> = self.psos.values().collect();
        for (i, r) in psos.iter().enumerate() {
            s.push_str("    {\n");
            s.push_str(&format!(
                "      \"pso_key\": \"{}\",\n",
                hex_of(&r.pso_key)
            ));
            s.push_str(&format!("      \"kind_tag\": {},\n", r.kind_tag));
            s.push_str("      \"stage_digests\": [\n");
            for (j, sd) in r.stage_digests.iter().enumerate() {
                s.push_str(&format!(
                    "        {{ \"stage_tag\": {}, \"digest\": \"{}\" }}{}\n",
                    sd.stage_tag,
                    hex_of(&sd.digest),
                    if j + 1 == r.stage_digests.len() {
                        ""
                    } else {
                        ","
                    }
                ));
            }
            s.push_str("      ],\n");
            s.push_str(&format!(
                "      \"fixed_function_digest\": \"{}\"\n",
                hex_of(&r.fixed_function_digest)
            ));
            s.push_str(if i + 1 == psos.len() {
                "    }\n"
            } else {
                "    },\n"
            });
        }
        s.push_str("  ]\n");
        s.push_str("}\n");
        s
    }

    /// 解析 manifest JSON。
    pub fn from_json(text: &str) -> Result<Self, ManifestError> {
        let schema = ju::json_str_field(text, "schema").ok_or_else(|| {
            ManifestError::Malformed("缺 `schema` 字段".to_owned())
        })?;
        if schema != MANIFEST_SCHEMA {
            return Err(ManifestError::Malformed(format!(
                "`schema` 须为 \"{MANIFEST_SCHEMA}\",得 `{schema}`"
            )));
        }
        let mut m = Manifest::default();
        let shaders_slice = ju::json_array_field(text, "shaders").ok_or_else(|| {
            ManifestError::Malformed("缺 `shaders` 数组".to_owned())
        })?;
        for elem in split_json_top_level(shaders_slice, '[', ']').unwrap_or_default() {
            let r = parse_shader_record(&elem)?;
            let key = r.pipeline_key;
            if let Some(prev) = m.shaders.get(&key) {
                if prev != &r {
                    return Err(conflict_shader(prev, &r));
                }
            }
            m.shaders.insert(key, r);
        }
        let psos_slice = ju::json_array_field(text, "psos").ok_or_else(|| {
            ManifestError::Malformed("缺 `psos` 数组".to_owned())
        })?;
        for elem in split_json_top_level(psos_slice, '[', ']').unwrap_or_default() {
            let r = parse_pso_record(&elem)?;
            let key = r.pso_key;
            if let Some(prev) = m.psos.get(&key) {
                if prev != &r {
                    return Err(conflict_pso(prev, &r));
                }
            }
            m.psos.insert(key, r);
        }
        Ok(m)
    }

    /// 排序后的 pipeline_key / pso_key hex 集合(golden 比对面)。
    pub fn key_set(&self) -> ManifestKeySet {
        ManifestKeySet {
            pipeline_keys: self.shaders.keys().map(hex_of).collect(),
            pso_keys: self.psos.keys().map(hex_of).collect(),
        }
    }
}

/// merged key 集合(golden 比对)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestKeySet {
    pub pipeline_keys: Vec<String>,
    pub pso_keys: Vec<String>,
}

impl ManifestKeySet {
    pub fn to_json(&self) -> String {
        let mut s = String::new();
        s.push_str("{\n");
        s.push_str("  \"pipeline_keys\": [\n");
        for (i, k) in self.pipeline_keys.iter().enumerate() {
            s.push_str(&format!(
                "    \"{k}\"{}\n",
                if i + 1 == self.pipeline_keys.len() {
                    ""
                } else {
                    ","
                }
            ));
        }
        s.push_str("  ],\n");
        s.push_str("  \"pso_keys\": [\n");
        for (i, k) in self.pso_keys.iter().enumerate() {
            s.push_str(&format!(
                "    \"{k}\"{}\n",
                if i + 1 == self.pso_keys.len() {
                    ""
                } else {
                    ","
                }
            ));
        }
        s.push_str("  ]\n");
        s.push_str("}\n");
        s
    }
}

fn parse_shader_record(obj: &str) -> Result<ShaderRecord, ManifestError> {
    let entry = ju::json_str_field(obj, "entry")
        .ok_or_else(|| ManifestError::Malformed("shader 缺 `entry`".to_owned()))?;
    let stage = ju::json_str_field(obj, "stage")
        .ok_or_else(|| ManifestError::Malformed("shader 缺 `stage`".to_owned()))?;
    let interface_hash = parse_hex32(
        &ju::json_str_field(obj, "interface_hash")
            .ok_or_else(|| ManifestError::Malformed("shader 缺 `interface_hash`".to_owned()))?,
        "interface_hash",
    )?;
    let source_digest = parse_hex32(
        &ju::json_str_field(obj, "source_digest")
            .ok_or_else(|| ManifestError::Malformed("shader 缺 `source_digest`".to_owned()))?,
        "source_digest",
    )?;
    let selected_profile_digest = parse_hex32(
        &ju::json_str_field(obj, "selected_profile_digest").ok_or_else(|| {
            ManifestError::Malformed("shader 缺 `selected_profile_digest`".to_owned())
        })?,
        "selected_profile_digest",
    )?;
    let permutation_domain_digest = parse_hex32(
        &ju::json_str_field(obj, "permutation_domain_digest").ok_or_else(|| {
            ManifestError::Malformed("shader 缺 `permutation_domain_digest`".to_owned())
        })?,
        "permutation_domain_digest",
    )?;
    let variant_key = ju::json_str_field(obj, "variant_key").unwrap_or_default();
    let pipeline_key = parse_hex32(
        &ju::json_str_field(obj, "pipeline_key")
            .ok_or_else(|| ManifestError::Malformed("shader 缺 `pipeline_key`".to_owned()))?,
        "pipeline_key",
    )?;
    Ok(ShaderRecord {
        entry,
        stage,
        interface_hash,
        source_digest,
        selected_profile_digest,
        permutation_domain_digest,
        variant_key,
        pipeline_key,
    })
}

fn parse_pso_record(obj: &str) -> Result<PsoRecord, ManifestError> {
    let pso_key = parse_hex32(
        &ju::json_str_field(obj, "pso_key")
            .ok_or_else(|| ManifestError::Malformed("pso 缺 `pso_key`".to_owned()))?,
        "pso_key",
    )?;
    let kind_tag = ju::json_i64_field(obj, "kind_tag")
        .ok_or_else(|| ManifestError::Malformed("pso 缺 `kind_tag`".to_owned()))?
        as u32;
    let ff = parse_hex32(
        &ju::json_str_field(obj, "fixed_function_digest").ok_or_else(|| {
            ManifestError::Malformed("pso 缺 `fixed_function_digest`".to_owned())
        })?,
        "fixed_function_digest",
    )?;
    let stages_slice = ju::json_array_field(obj, "stage_digests").ok_or_else(|| {
        ManifestError::Malformed("pso 缺 `stage_digests`".to_owned())
    })?;
    let mut stage_digests = Vec::new();
    for elem in split_json_top_level(stages_slice, '[', ']').unwrap_or_default() {
        let stage_tag = ju::json_i64_field(&elem, "stage_tag").ok_or_else(|| {
            ManifestError::Malformed("stage_digests 元素缺 `stage_tag`".to_owned())
        })? as u32;
        let digest = parse_hex32(
            &ju::json_str_field(&elem, "digest")
                .ok_or_else(|| ManifestError::Malformed("stage_digests 元素缺 `digest`".to_owned()))?,
            "stage_digests.digest",
        )?;
        stage_digests.push(StageDigestEntry { stage_tag, digest });
    }
    Ok(PsoRecord {
        pso_key,
        kind_tag,
        stage_digests,
        fixed_function_digest: ff,
    })
}

fn conflict_shader(a: &ShaderRecord, b: &ShaderRecord) -> ManifestError {
    let mut differing = Vec::new();
    if a.entry != b.entry {
        differing.push("entry".to_owned());
    }
    if a.stage != b.stage {
        differing.push("stage".to_owned());
    }
    if a.interface_hash != b.interface_hash {
        differing.push("interface_hash".to_owned());
    }
    if a.source_digest != b.source_digest {
        differing.push("source_digest".to_owned());
    }
    if a.selected_profile_digest != b.selected_profile_digest {
        differing.push("selected_profile_digest".to_owned());
    }
    if a.permutation_domain_digest != b.permutation_domain_digest {
        differing.push("permutation_domain_digest".to_owned());
    }
    if a.variant_key != b.variant_key {
        differing.push("variant_key".to_owned());
    }
    ManifestError::Conflict {
        kind: "shader",
        key_hex: hex_of(&a.pipeline_key),
        differing_fields: differing,
    }
}

fn conflict_pso(a: &PsoRecord, b: &PsoRecord) -> ManifestError {
    let mut differing = Vec::new();
    if a.kind_tag != b.kind_tag {
        differing.push("kind_tag".to_owned());
    }
    if a.stage_digests != b.stage_digests {
        differing.push("stage_digests".to_owned());
    }
    if a.fixed_function_digest != b.fixed_function_digest {
        differing.push("fixed_function_digest".to_owned());
    }
    ManifestError::Conflict {
        kind: "pso",
        key_hex: hex_of(&a.pso_key),
        differing_fields: differing,
    }
}

// ═══════════════════════ from_parts / merge / coverage ═══════════════════════

/// 从 reflection JSON + optional permutations 报告 + M30 collector JSON 装配
/// per-unit manifest(RXS-0317)。shader 字段仅取自 reflection entries;permutations
/// 可选——若给定则核验同名 entry 的 `variant_key`/`domain_digest` 与 reflection
/// 一致(不一致 = Malformed,禁第二事实源覆盖)。collector 缺省 → 空 PSO 段。
pub fn from_parts(
    reflection_json: &str,
    permutations_json: Option<&str>,
    collector_json: Option<&str>,
) -> Result<Manifest, ManifestError> {
    let mut m = Manifest::default();
    let entries_slice = ju::json_array_field(reflection_json, "entries").ok_or_else(|| {
        ManifestError::Malformed("reflection 缺 `entries` 数组".to_owned())
    })?;
    for elem in split_json_top_level(entries_slice, '[', ']').unwrap_or_default() {
        let name = ju::json_str_field(&elem, "name")
            .ok_or_else(|| ManifestError::Malformed("reflection entry 缺 `name`".to_owned()))?;
        let stage = ju::json_str_field(&elem, "stage")
            .ok_or_else(|| ManifestError::Malformed("reflection entry 缺 `stage`".to_owned()))?;
        let interface_hash = parse_hex32(
            &ju::json_str_field(&elem, "interface_hash").ok_or_else(|| {
                ManifestError::Malformed("reflection entry 缺 `interface_hash`".to_owned())
            })?,
            "interface_hash",
        )?;
        let source_digest = parse_hex32(
            &ju::json_str_field(&elem, "source_digest").ok_or_else(|| {
                ManifestError::Malformed("reflection entry 缺 `source_digest`".to_owned())
            })?,
            "source_digest",
        )?;
        let selected_profile_digest = parse_hex32(
            &ju::json_str_field(&elem, "selected_profile_digest").ok_or_else(|| {
                ManifestError::Malformed(
                    "reflection entry 缺 `selected_profile_digest`".to_owned(),
                )
            })?,
            "selected_profile_digest",
        )?;
        let permutation_domain_digest = parse_hex32(
            &ju::json_str_field(&elem, "permutation_domain_digest").ok_or_else(|| {
                ManifestError::Malformed(
                    "reflection entry 缺 `permutation_domain_digest`".to_owned(),
                )
            })?,
            "permutation_domain_digest",
        )?;
        let variant_key = ju::json_str_field(&elem, "variant_key").unwrap_or_default();
        let pipeline_key = parse_hex32(
            &ju::json_str_field(&elem, "pipeline_key").ok_or_else(|| {
                ManifestError::Malformed("reflection entry 缺 `pipeline_key`".to_owned())
            })?,
            "pipeline_key",
        )?;
        let rec = ShaderRecord {
            entry: name,
            stage,
            interface_hash,
            source_digest,
            selected_profile_digest,
            permutation_domain_digest,
            variant_key,
            pipeline_key,
        };
        if let Some(prev) = m.shaders.get(&rec.pipeline_key) {
            if prev != &rec {
                return Err(conflict_shader(prev, &rec));
            }
        } else {
            m.shaders.insert(rec.pipeline_key, rec);
        }
    }

    if let Some(perm) = permutations_json {
        verify_permutations_agree(&m, perm)?;
    }

    if let Some(coll) = collector_json {
        let records_slice = ju::json_array_field(coll, "records").ok_or_else(|| {
            ManifestError::Malformed("collector 缺 `records` 数组".to_owned())
        })?;
        for elem in split_json_top_level(records_slice, '[', ']').unwrap_or_default() {
            let r = parse_pso_record(&elem)?;
            if let Some(prev) = m.psos.get(&r.pso_key) {
                if prev != &r {
                    return Err(conflict_pso(prev, &r));
                }
            } else {
                m.psos.insert(r.pso_key, r);
            }
        }
    }
    Ok(m)
}

/// permutations 报告与 reflection 装配结果一致性核验(禁第二事实源覆盖)。
fn verify_permutations_agree(m: &Manifest, perm_json: &str) -> Result<(), ManifestError> {
    let entries_slice = ju::json_array_field(perm_json, "entries").unwrap_or("[]");
    for elem in split_json_top_level(entries_slice, '[', ']').unwrap_or_default() {
        let name = match ju::json_str_field(&elem, "name") {
            Some(n) => n,
            None => continue,
        };
        let Some(domain_hex) = ju::json_str_field(&elem, "domain_digest") else {
            continue;
        };
        let domain = parse_hex32(&domain_hex, "domain_digest")?;
        let shader = m.shaders.values().find(|s| s.entry == name);
        let Some(shader) = shader else {
            continue;
        };
        if shader.permutation_domain_digest != domain {
            return Err(ManifestError::Malformed(format!(
                "permutations 与 reflection 对 entry `{name}` 的 permutation_domain_digest 不一致(禁第二事实源)"
            )));
        }
    }
    Ok(())
}

/// N 份 manifest → 按键归并(RXS-0318):同键同 payload dedup;同键异 payload
/// fail-closed;输出确定性(与输入次序无关)。
pub fn merge(manifests: &[Manifest]) -> Result<Manifest, ManifestError> {
    let mut out = Manifest::default();
    for m in manifests {
        for (k, r) in &m.shaders {
            match out.shaders.get(k) {
                None => {
                    out.shaders.insert(*k, r.clone());
                }
                Some(prev) if prev == r => {
                    // dedup:恰好保留一条
                }
                Some(prev) => return Err(conflict_shader(prev, r)),
            }
        }
        for (k, r) in &m.psos {
            match out.psos.get(k) {
                None => {
                    out.psos.insert(*k, r.clone());
                }
                Some(prev) if prev == r => {}
                Some(prev) => return Err(conflict_pso(prev, r)),
            }
        }
    }
    Ok(out)
}

/// coverage:声明表的 entry/variant/PSO 全集须在 merged 中恰好各现一次。
pub fn check_coverage(merged: &Manifest, table: &CoverageTable) -> Result<(), ManifestError> {
    let mut missing = Vec::new();
    let mut extra = Vec::new();

    let mut expected_pipe = std::collections::BTreeSet::new();
    for s in &table.shaders {
        expected_pipe.insert(s.pipeline_key.to_ascii_lowercase());
        let key = match parse_hex32(&s.pipeline_key, "coverage.pipeline_key") {
            Ok(k) => k,
            Err(e) => return Err(e),
        };
        match merged.shaders.get(&key) {
            None => missing.push(format!("shader:{}", s.pipeline_key)),
            Some(got) => {
                if got.entry != s.entry || got.variant_key != s.variant_key {
                    missing.push(format!(
                        "shader:{}(entry/variant mismatch want {}:{} got {}:{})",
                        s.pipeline_key, s.entry, s.variant_key, got.entry, got.variant_key
                    ));
                }
            }
        }
    }
    for k in merged.shaders.keys() {
        let hex = hex_of(k);
        if !expected_pipe.contains(&hex) {
            extra.push(format!("shader:{hex}"));
        }
    }

    let mut expected_pso = std::collections::BTreeSet::new();
    for pk in &table.pso_keys {
        expected_pso.insert(pk.to_ascii_lowercase());
        let key = parse_hex32(pk, "coverage.pso_key")?;
        if !merged.psos.contains_key(&key) {
            missing.push(format!("pso:{pk}"));
        }
    }
    for k in merged.psos.keys() {
        let hex = hex_of(k);
        if !expected_pso.contains(&hex) {
            extra.push(format!("pso:{hex}"));
        }
    }

    if missing.is_empty() && extra.is_empty() {
        Ok(())
    } else {
        Err(ManifestError::Coverage { missing, extra })
    }
}

/// 解析 coverage 声明表 JSON。
pub fn parse_coverage_table(text: &str) -> Result<CoverageTable, ManifestError> {
    if let Some(schema) = ju::json_str_field(text, "schema") {
        if schema != COVERAGE_SCHEMA {
            return Err(ManifestError::Malformed(format!(
                "coverage `schema` 须为 \"{COVERAGE_SCHEMA}\",得 `{schema}`"
            )));
        }
    }
    let mut table = CoverageTable::default();
    if let Some(arr) = ju::json_array_field(text, "shaders") {
        for elem in split_json_top_level(arr, '[', ']').unwrap_or_default() {
            let entry = ju::json_str_field(&elem, "entry").ok_or_else(|| {
                ManifestError::Malformed("coverage shader 缺 `entry`".to_owned())
            })?;
            let variant_key = ju::json_str_field(&elem, "variant_key").unwrap_or_default();
            let pipeline_key = ju::json_str_field(&elem, "pipeline_key").ok_or_else(|| {
                ManifestError::Malformed("coverage shader 缺 `pipeline_key`".to_owned())
            })?;
            table.shaders.push(CoverageShader {
                entry,
                variant_key,
                pipeline_key,
            });
        }
    }
    if let Some(arr) = ju::json_array_field(text, "pso_keys") {
        for elem in split_json_top_level(arr, '[', ']').unwrap_or_default() {
            let key = elem
                .trim()
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .ok_or_else(|| {
                    ManifestError::Malformed("coverage pso_keys 元素须为字符串".to_owned())
                })?;
            table.pso_keys.push(key.to_owned());
        }
    }
    Ok(table)
}

/// 读路径 → [`Manifest`]。
pub fn load_manifest(path: &Path) -> Result<Manifest, ManifestError> {
    let text = std::fs::read_to_string(path).map_err(|e| {
        ManifestError::Io(format!("cannot read {}: {e}", path.display()))
    })?;
    Manifest::from_json(&text)
}

/// 写 merged/assembled JSON。
pub fn write_manifest(path: &Path, m: &Manifest) -> Result<(), ManifestError> {
    std::fs::write(path, m.to_json()).map_err(|e| {
        ManifestError::Io(format!("cannot write {}: {e}", path.display()))
    })
}

/// CLI:`--merge-manifests -o out.json a.json b.json ...`(工具模式)。
pub fn merge_manifest_files(inputs: &[std::path::PathBuf], out: &Path) -> Result<Manifest, ManifestError> {
    if inputs.is_empty() {
        return Err(ManifestError::Malformed(
            "--merge-manifests 需要至少一份输入 manifest JSON".to_owned(),
        ));
    }
    let mut parts = Vec::with_capacity(inputs.len());
    for p in inputs {
        parts.push(load_manifest(p)?);
    }
    let merged = merge(&parts)?;
    write_manifest(out, &merged)?;
    Ok(merged)
}

/// CLI 装配 helper:`--assemble-manifest -o unit.json --reflection r.json
/// [--permutations p.json] [--collector c.json]`。
pub fn assemble_manifest_files(
    reflection: &Path,
    permutations: Option<&Path>,
    collector: Option<&Path>,
    out: &Path,
) -> Result<Manifest, ManifestError> {
    let refl = std::fs::read_to_string(reflection).map_err(|e| {
        ManifestError::Io(format!("cannot read {}: {e}", reflection.display()))
    })?;
    let perm = match permutations {
        Some(p) => Some(std::fs::read_to_string(p).map_err(|e| {
            ManifestError::Io(format!("cannot read {}: {e}", p.display()))
        })?),
        None => None,
    };
    let coll = match collector {
        Some(p) => Some(std::fs::read_to_string(p).map_err(|e| {
            ManifestError::Io(format!("cannot read {}: {e}", p.display()))
        })?),
        None => None,
    };
    let m = from_parts(&refl, perm.as_deref(), coll.as_deref())?;
    write_manifest(out, &m)?;
    Ok(m)
}

// ═══════════════════════ 单测(≥8;RXS-0317/0318) ═══════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    const H1: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    const H2: &str = "2222222222222222222222222222222222222222222222222222222222222222";
    const H3: &str = "3333333333333333333333333333333333333333333333333333333333333333";
    const H4: &str = "4444444444444444444444444444444444444444444444444444444444444444";
    const H5: &str = "5555555555555555555555555555555555555555555555555555555555555555";
    const H6: &str = "6666666666666666666666666666666666666666666666666666666666666666";
    const H7: &str = "7777777777777777777777777777777777777777777777777777777777777777";
    const H8: &str = "8888888888888888888888888888888888888888888888888888888888888888";
    const H9: &str = "9999999999999999999999999999999999999999999999999999999999999999";
    const HA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HB: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HC: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const HD: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const HE: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    const HF: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
    const H0: &str = "0000000000000000000000000000000000000000000000000000000000000000";

    fn refl_entry(
        name: &str,
        stage: &str,
        iface: &str,
        src: &str,
        prof: &str,
        perm: &str,
        variant: &str,
        pipe: &str,
    ) -> String {
        format!(
            r#"{{
      "name": "{name}",
      "stage": "{stage}",
      "interface_hash": "{iface}",
      "source_digest": "{src}",
      "selected_profile_digest": "{prof}",
      "permutation_domain_digest": "{perm}",
      "variant_key": "{variant}",
      "pipeline_key": "{pipe}"
    }}"#
        )
    }

    fn reflection_doc(entries: &[&str]) -> String {
        format!(
            "{{\n  \"schema\": \"rurix.reflection.v1\",\n  \"entries\": [\n    {}\n  ]\n}}\n",
            entries.join(",\n    ")
        )
    }

    fn collector_doc(records: &[&str]) -> String {
        format!(
            "{{\n  \"schema\": \"rurix.pso-keys.v1\",\n  \"records\": [\n    {}\n  ]\n}}\n",
            records.join(",\n    ")
        )
    }

    fn pso_rec(key: &str, kind: u32, stage_tag: u32, dig: &str, ff: &str) -> String {
        format!(
            r#"{{
      "name": "x",
      "pso_key": "{key}",
      "kind_tag": {kind},
      "stage_digests": [
        {{ "stage_tag": {stage_tag}, "digest": "{dig}" }}
      ],
      "fixed_function_digest": "{ff}"
    }}"#
        )
    }

    /// //@ spec: RXS-0317
    #[test]
    fn schema_and_digest_stable() {
        let refl = reflection_doc(&[&refl_entry(
            "vs_main", "vertex", H1, H2, H3, H4, "", H5,
        )]);
        let coll = collector_doc(&[&pso_rec(H6, 0, 2, H7, H8)]);
        let m = from_parts(&refl, None, Some(&coll)).unwrap();
        assert_eq!(m.shaders.len(), 1);
        assert_eq!(m.psos.len(), 1);
        let d1 = m.digest();
        let d2 = m.digest();
        assert_eq!(d1, d2);
        let canon = m.canonical_bytes();
        assert!(canon.starts_with(MANIFEST_DOMAIN));
        assert_eq!(sha256::digest(&canon), d1);
        let json = m.to_json();
        assert!(json.contains(MANIFEST_SCHEMA));
        assert!(json.contains(&hex_of(&d1)));
        let m2 = Manifest::from_json(&json).unwrap();
        assert_eq!(m2.digest(), d1);
    }

    /// //@ spec: RXS-0317
    #[test]
    fn from_parts_copies_reflection_fields() {
        let refl = reflection_doc(&[&refl_entry(
            "fs_main",
            "fragment",
            H1,
            H2,
            H3,
            H4,
            "FOG=true",
            H5,
        )]);
        let m = from_parts(&refl, None, None).unwrap();
        let r = m.shaders.values().next().unwrap();
        assert_eq!(r.entry, "fs_main");
        assert_eq!(r.stage, "fragment");
        assert_eq!(hex_of(&r.interface_hash), H1);
        assert_eq!(hex_of(&r.source_digest), H2);
        assert_eq!(hex_of(&r.selected_profile_digest), H3);
        assert_eq!(hex_of(&r.permutation_domain_digest), H4);
        assert_eq!(r.variant_key, "FOG=true");
        assert_eq!(hex_of(&r.pipeline_key), H5);
    }

    /// //@ spec: RXS-0317
    #[test]
    fn digest_flips_on_interface_hash_or_pso_key() {
        let refl_a = reflection_doc(&[&refl_entry(
            "vs_main", "vertex", H1, H2, H3, H4, "", H5,
        )]);
        let refl_b = reflection_doc(&[&refl_entry(
            "vs_main", "vertex", H9, H2, H3, H4, "", H5,
        )]);
        let ma = from_parts(&refl_a, None, None).unwrap();
        let mb = from_parts(&refl_b, None, None).unwrap();
        assert_ne!(ma.digest(), mb.digest(), "改 interface_hash → digest 必变");

        let coll_a = collector_doc(&[&pso_rec(H6, 0, 2, H7, H8)]);
        let coll_b = collector_doc(&[&pso_rec(HA, 0, 2, H7, H8)]);
        let pa = from_parts(&refl_a, None, Some(&coll_a)).unwrap();
        let pb = from_parts(&refl_a, None, Some(&coll_b)).unwrap();
        assert_ne!(pa.digest(), pb.digest(), "改 pso_key → digest 必变");
    }

    /// //@ spec: RXS-0318
    #[test]
    fn merge_dedup_identical() {
        let refl = reflection_doc(&[&refl_entry(
            "vs_main", "vertex", H1, H2, H3, H4, "", H5,
        )]);
        let coll = collector_doc(&[&pso_rec(H6, 0, 2, H7, H8)]);
        let a = from_parts(&refl, None, Some(&coll)).unwrap();
        let b = from_parts(&refl, None, Some(&coll)).unwrap();
        let merged = merge(&[a.clone(), b]).unwrap();
        assert_eq!(merged.shaders.len(), 1);
        assert_eq!(merged.psos.len(), 1);
        assert_eq!(merged.digest(), a.digest());
    }

    /// //@ spec: RXS-0318
    #[test]
    fn merge_conflict_fail_closed() {
        let refl_a = reflection_doc(&[&refl_entry(
            "vs_main", "vertex", H1, H2, H3, H4, "", H5,
        )]);
        let refl_b = reflection_doc(&[&refl_entry(
            "vs_main", "vertex", H9, H2, H3, H4, "", H5,
        )]);
        let a = from_parts(&refl_a, None, None).unwrap();
        let b = from_parts(&refl_b, None, None).unwrap();
        let err = merge(&[a, b]).unwrap_err();
        match err {
            ManifestError::Conflict {
                kind,
                key_hex,
                differing_fields,
            } => {
                assert_eq!(kind, "shader");
                assert_eq!(key_hex, H5);
                assert!(differing_fields.iter().any(|f| f == "interface_hash"));
            }
            other => panic!("expected Conflict, got {other}"),
        }
    }

    /// //@ spec: RXS-0318
    #[test]
    fn merge_order_invariant_and_double_run() {
        let refl_a = reflection_doc(&[&refl_entry(
            "vs_main", "vertex", H1, H2, H3, H4, "", H5,
        )]);
        let refl_b = reflection_doc(&[&refl_entry(
            "fs_main", "fragment", HA, HB, HC, HD, "", HE,
        )]);
        let coll_a = collector_doc(&[&pso_rec(H6, 0, 2, H7, H8)]);
        let coll_b = collector_doc(&[&pso_rec(HF, 1, 0, H0, H1)]);
        let a = from_parts(&refl_a, None, Some(&coll_a)).unwrap();
        let b = from_parts(&refl_b, None, Some(&coll_b)).unwrap();
        let m1 = merge(&[a.clone(), b.clone()]).unwrap();
        let m2 = merge(&[b.clone(), a.clone()]).unwrap();
        assert_eq!(m1.digest(), m2.digest());
        assert_eq!(m1.to_json(), m2.to_json());
        let m3 = merge(&[a, b]).unwrap();
        assert_eq!(m1.digest(), m3.digest());
        assert_eq!(m1.canonical_bytes(), m3.canonical_bytes());
    }

    /// //@ spec: RXS-0318
    #[test]
    fn coverage_exact_and_gap() {
        let refl_a = reflection_doc(&[&refl_entry(
            "vs_main", "vertex", H1, H2, H3, H4, "", H5,
        )]);
        let refl_b = reflection_doc(&[&refl_entry(
            "fs_main", "fragment", HA, HB, HC, HD, "", HE,
        )]);
        let coll = collector_doc(&[&pso_rec(H6, 0, 2, H7, H8)]);
        let merged = merge(&[
            from_parts(&refl_a, None, Some(&coll)).unwrap(),
            from_parts(&refl_b, None, None).unwrap(),
        ])
        .unwrap();
        let ok = CoverageTable {
            shaders: vec![
                CoverageShader {
                    entry: "vs_main".into(),
                    variant_key: String::new(),
                    pipeline_key: H5.into(),
                },
                CoverageShader {
                    entry: "fs_main".into(),
                    variant_key: String::new(),
                    pipeline_key: HE.into(),
                },
            ],
            pso_keys: vec![H6.into()],
        };
        assert!(check_coverage(&merged, &ok).is_ok());

        let gap = CoverageTable {
            shaders: vec![
                CoverageShader {
                    entry: "vs_main".into(),
                    variant_key: String::new(),
                    pipeline_key: H5.into(),
                },
                CoverageShader {
                    entry: "missing".into(),
                    variant_key: String::new(),
                    pipeline_key: H0.into(),
                },
            ],
            pso_keys: vec![H6.into()],
        };
        let err = check_coverage(&merged, &gap).unwrap_err();
        match err {
            ManifestError::Coverage { missing, extra } => {
                assert!(!missing.is_empty());
                assert!(!extra.is_empty() || missing.iter().any(|m| m.contains(H0)));
            }
            other => panic!("expected Coverage, got {other}"),
        }
    }

    /// //@ spec: RXS-0318
    #[test]
    fn key_set_union_sorted() {
        let refl_a = reflection_doc(&[&refl_entry(
            "vs_main", "vertex", H1, H2, H3, H4, "", H5,
        )]);
        let refl_b = reflection_doc(&[&refl_entry(
            "fs_main", "fragment", HA, HB, HC, HD, "", HE,
        )]);
        let coll_a = collector_doc(&[&pso_rec(H6, 0, 2, H7, H8)]);
        let coll_b = collector_doc(&[&pso_rec(HF, 1, 0, H0, H1)]);
        let merged = merge(&[
            from_parts(&refl_b, None, Some(&coll_b)).unwrap(),
            from_parts(&refl_a, None, Some(&coll_a)).unwrap(),
        ])
        .unwrap();
        let ks = merged.key_set();
        let mut want_pipe = vec![H5.to_owned(), HE.to_owned()];
        want_pipe.sort();
        let mut want_pso = vec![H6.to_owned(), HF.to_owned()];
        want_pso.sort();
        assert_eq!(ks.pipeline_keys, want_pipe);
        assert_eq!(ks.pso_keys, want_pso);
    }

    /// //@ spec: RXS-0317
    #[test]
    fn permutations_disagree_fail_closed() {
        let refl = reflection_doc(&[&refl_entry(
            "vs_main", "vertex", H1, H2, H3, H4, "", H5,
        )]);
        let perm = format!(
            r#"{{
  "schema": "rurix.permutations.v1",
  "entries": [
    {{ "name": "vs_main", "domain_digest": "{H9}" }}
  ]
}}"#
        );
        let err = from_parts(&refl, Some(&perm), None).unwrap_err();
        assert!(matches!(err, ManifestError::Malformed(_)));
    }
}
