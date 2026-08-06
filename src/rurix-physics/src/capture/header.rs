//! PhysicsCaptureHeader v1(RFC-0021 §4.A1)。

use super::canonical::CaptureError;
use crate::types::{BackendKind, WorldDesc};

pub const RECOVERY_LAYER_V1: &str = "semantic_journal_rebuild_v1";
pub const SCHEMA_ID: &str = "rurix.physics.capture";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedStepRational {
    pub num: u32,
    pub den: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeterminismProfile {
    pub job_threads: u32,
    pub job_system: String,
    pub cross_platform_deterministic: bool,
    pub double_precision: bool,
    pub object_layer_bits: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BudgetProfile {
    pub contact_capacity: u32,
    pub max_query_casts: u32,
    pub max_contact_events: u32,
    pub max_body_writes: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhysicsCaptureHeader {
    pub schema_id: String,
    pub schema_version: u32,
    pub jolt_version: String,
    pub joltc_commit: String,
    pub joltc_abi_digest: String,
    pub rurix_build_fingerprint: String,
    pub platform: String,
    pub fixed_step_rational: FixedStepRational,
    pub world_desc: WorldDescSnapshot,
    pub id_domain_start: u64,
    pub determinism_profile: DeterminismProfile,
    pub budget_profile: BudgetProfile,
    pub recovery_layer: String,
    pub scenario_id: String,
    pub tick_count: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorldDescSnapshot {
    pub backend: String,
    pub gravity: [f32; 3],
    pub layer_count: u32,
    pub max_bodies: u32,
    pub job_threads: Option<u32>,
    pub dt_fixed: f32,
    pub contact_capacity: u32,
}

impl WorldDescSnapshot {
    pub fn from_desc(d: &WorldDesc) -> Self {
        Self {
            backend: match d.backend {
                BackendKind::Jolt => "Jolt".into(),
                BackendKind::Rapier => "Rapier".into(),
            },
            gravity: d.gravity,
            layer_count: d.layer_count,
            max_bodies: d.max_bodies,
            job_threads: d.job_threads,
            dt_fixed: d.dt_fixed,
            contact_capacity: d.contact_capacity,
        }
    }

    pub fn to_desc(&self) -> Result<WorldDesc, CaptureError> {
        let backend = match self.backend.as_str() {
            "Jolt" => BackendKind::Jolt,
            "Rapier" => BackendKind::Rapier,
            other => {
                return Err(CaptureError::Parse(format!("unknown backend {other}")));
            }
        };
        Ok(WorldDesc {
            backend,
            gravity: self.gravity,
            layer_count: self.layer_count,
            max_bodies: self.max_bodies,
            job_threads: self.job_threads,
            dt_fixed: self.dt_fixed,
            contact_capacity: self.contact_capacity,
        })
    }
}

impl PhysicsCaptureHeader {
    pub fn new_jolt_53(
        scenario_id: &str,
        tick_count: u64,
        world: &WorldDesc,
        build_fingerprint: &str,
        abi_digest: &str,
        budget: BudgetProfile,
    ) -> Self {
        Self {
            schema_id: SCHEMA_ID.into(),
            schema_version: 1,
            jolt_version: "5.3.0".into(),
            joltc_commit: "2982004387a9e36ca89525a87d983709d3666da7".into(),
            joltc_abi_digest: abi_digest.into(),
            rurix_build_fingerprint: build_fingerprint.into(),
            platform: format!(
                "{}-{}-{}",
                std::env::consts::ARCH,
                std::env::consts::OS,
                if cfg!(target_env = "msvc") {
                    "msvc"
                } else if cfg!(target_env = "gnu") {
                    "gnu"
                } else {
                    "unknown"
                }
            ),
            fixed_step_rational: FixedStepRational { num: 1, den: 60 },
            world_desc: WorldDescSnapshot::from_desc(world),
            id_domain_start: 0,
            determinism_profile: DeterminismProfile {
                job_threads: world.job_threads.unwrap_or(1),
                job_system: "ThreadPool".into(),
                cross_platform_deterministic: false,
                double_precision: false,
                object_layer_bits: 16,
            },
            budget_profile: budget,
            recovery_layer: RECOVERY_LAYER_V1.into(),
            scenario_id: scenario_id.into(),
            tick_count,
        }
    }

    pub fn to_canonical_json(&self) -> Result<String, CaptureError> {
        use super::canonical::canon_f32_bits_at;
        let g0 = canon_f32_bits_at(self.world_desc.gravity[0], "g0")?;
        let g1 = canon_f32_bits_at(self.world_desc.gravity[1], "g1")?;
        let g2 = canon_f32_bits_at(self.world_desc.gravity[2], "g2")?;
        let dt = canon_f32_bits_at(self.world_desc.dt_fixed, "dt")?;
        let jt = match self.world_desc.job_threads {
            Some(n) => n.to_string(),
            None => "null".into(),
        };
        Ok(format!(
            concat!(
                "{{\n",
                "  \"schema_id\": \"{schema_id}\",\n",
                "  \"schema_version\": {schema_version},\n",
                "  \"jolt_version\": \"{jolt_version}\",\n",
                "  \"joltc_commit\": \"{joltc_commit}\",\n",
                "  \"joltc_abi_digest\": \"{joltc_abi_digest}\",\n",
                "  \"rurix_build_fingerprint\": \"{rurix_build_fingerprint}\",\n",
                "  \"platform\": \"{platform}\",\n",
                "  \"fixed_step_rational\": {{\"num\": {num}, \"den\": {den}}},\n",
                "  \"world_desc\": {{\n",
                "    \"backend\": \"{backend}\",\n",
                "    \"gravity\": [\"{g0:08x}\", \"{g1:08x}\", \"{g2:08x}\"],\n",
                "    \"layer_count\": {layer_count},\n",
                "    \"max_bodies\": {max_bodies},\n",
                "    \"job_threads\": {job_threads},\n",
                "    \"dt_fixed\": \"{dt:08x}\",\n",
                "    \"contact_capacity\": {contact_capacity}\n",
                "  }},\n",
                "  \"id_domain_start\": {id_domain_start},\n",
                "  \"determinism_profile\": {{\n",
                "    \"job_threads\": {dp_threads},\n",
                "    \"job_system\": \"{job_system}\",\n",
                "    \"cross_platform_deterministic\": {cpd},\n",
                "    \"double_precision\": {dp},\n",
                "    \"object_layer_bits\": {olb}\n",
                "  }},\n",
                "  \"budget_profile\": {{\n",
                "    \"contact_capacity\": {bp_cc},\n",
                "    \"max_query_casts\": {bp_qc},\n",
                "    \"max_contact_events\": {bp_ce},\n",
                "    \"max_body_writes\": {bp_bw}\n",
                "  }},\n",
                "  \"recovery_layer\": \"{recovery_layer}\",\n",
                "  \"scenario_id\": \"{scenario_id}\",\n",
                "  \"tick_count\": {tick_count}\n",
                "}}\n"
            ),
            schema_id = self.schema_id,
            schema_version = self.schema_version,
            jolt_version = self.jolt_version,
            joltc_commit = self.joltc_commit,
            joltc_abi_digest = self.joltc_abi_digest,
            rurix_build_fingerprint = self.rurix_build_fingerprint,
            platform = self.platform,
            num = self.fixed_step_rational.num,
            den = self.fixed_step_rational.den,
            backend = self.world_desc.backend,
            g0 = g0,
            g1 = g1,
            g2 = g2,
            layer_count = self.world_desc.layer_count,
            max_bodies = self.world_desc.max_bodies,
            job_threads = jt,
            dt = dt,
            contact_capacity = self.world_desc.contact_capacity,
            id_domain_start = self.id_domain_start,
            dp_threads = self.determinism_profile.job_threads,
            job_system = self.determinism_profile.job_system,
            cpd = self.determinism_profile.cross_platform_deterministic,
            dp = self.determinism_profile.double_precision,
            olb = self.determinism_profile.object_layer_bits,
            bp_cc = self.budget_profile.contact_capacity,
            bp_qc = self.budget_profile.max_query_casts,
            bp_ce = self.budget_profile.max_contact_events,
            bp_bw = self.budget_profile.max_body_writes,
            recovery_layer = self.recovery_layer,
            scenario_id = self.scenario_id,
            tick_count = self.tick_count,
        ))
    }

    pub fn parse_json(text: &str) -> Result<Self, CaptureError> {
        // 最小字段提取(手写;字段序不依赖反序列化器)。
        fn req<'a>(text: &'a str, key: &str) -> Result<&'a str, CaptureError> {
            let pat = format!("\"{key}\"");
            let i = text
                .find(&pat)
                .ok_or_else(|| CaptureError::Parse(format!("missing {key}")))?;
            let rest = &text[i + pat.len()..];
            let rest = rest.trim_start_matches(|c: char| c == ' ' || c == ':' || c == '\n');
            if let Some(rest) = rest.strip_prefix('"') {
                let end = rest
                    .find('"')
                    .ok_or_else(|| CaptureError::Parse(format!("bad string {key}")))?;
                Ok(&rest[..end])
            } else {
                let end = rest
                    .find(|c: char| c == ',' || c == '\n' || c == '}')
                    .unwrap_or(rest.len());
                Ok(rest[..end].trim())
            }
        }
        fn req_u64(text: &str, key: &str) -> Result<u64, CaptureError> {
            req(text, key)?
                .parse()
                .map_err(|e| CaptureError::Parse(format!("{key}: {e}")))
        }
        fn req_u32(text: &str, key: &str) -> Result<u32, CaptureError> {
            Ok(req_u64(text, key)? as u32)
        }
        fn hex_f32(s: &str) -> Result<f32, CaptureError> {
            let s = s.trim_matches('"');
            let bits = u32::from_str_radix(s, 16)
                .map_err(|e| CaptureError::Parse(format!("hex f32: {e}")))?;
            Ok(f32::from_bits(bits))
        }
        // gravity array
        let g_pat = "\"gravity\"";
        let gi = text
            .find(g_pat)
            .ok_or_else(|| CaptureError::Parse("missing gravity".into()))?;
        let g_rest = &text[gi..];
        let lb = g_rest
            .find('[')
            .ok_or_else(|| CaptureError::Parse("gravity [".into()))?;
        let rb = g_rest[lb..]
            .find(']')
            .ok_or_else(|| CaptureError::Parse("gravity ]".into()))?;
        let parts: Vec<&str> = g_rest[lb + 1..lb + rb]
            .split(',')
            .map(str::trim)
            .collect();
        if parts.len() != 3 {
            return Err(CaptureError::Parse("gravity len".into()));
        }
        let gravity = [hex_f32(parts[0])?, hex_f32(parts[1])?, hex_f32(parts[2])?];
        let jt_raw = req(text, "job_threads")?;
        // first job_threads in world_desc; may be null
        let world_job_threads = if jt_raw == "null" {
            None
        } else {
            Some(jt_raw.parse().map_err(|e| CaptureError::Parse(format!("jt: {e}")))?)
        };
        Ok(Self {
            schema_id: req(text, "schema_id")?.into(),
            schema_version: req_u32(text, "schema_version")?,
            jolt_version: req(text, "jolt_version")?.into(),
            joltc_commit: req(text, "joltc_commit")?.into(),
            joltc_abi_digest: req(text, "joltc_abi_digest")?.into(),
            rurix_build_fingerprint: req(text, "rurix_build_fingerprint")?.into(),
            platform: req(text, "platform")?.into(),
            fixed_step_rational: FixedStepRational {
                num: req_u32(text, "num")?,
                den: req_u32(text, "den")?,
            },
            world_desc: WorldDescSnapshot {
                backend: req(text, "backend")?.into(),
                gravity,
                layer_count: req_u32(text, "layer_count")?,
                max_bodies: req_u32(text, "max_bodies")?,
                job_threads: world_job_threads,
                dt_fixed: hex_f32(req(text, "dt_fixed")?)?,
                contact_capacity: {
                    // last contact_capacity in budget preferred; world_desc one appears first
                    req_u32(text, "contact_capacity")?
                },
            },
            id_domain_start: req_u64(text, "id_domain_start")?,
            determinism_profile: DeterminismProfile {
                job_threads: req_u32(text, "job_threads").unwrap_or(1),
                job_system: req(text, "job_system")?.into(),
                cross_platform_deterministic: req(text, "cross_platform_deterministic")? == "true",
                double_precision: req(text, "double_precision")? == "true",
                object_layer_bits: req_u32(text, "object_layer_bits")?,
            },
            budget_profile: BudgetProfile {
                contact_capacity: req_u32(text, "contact_capacity")?,
                max_query_casts: req_u32(text, "max_query_casts")?,
                max_contact_events: req_u32(text, "max_contact_events")?,
                max_body_writes: req_u32(text, "max_body_writes")?,
            },
            recovery_layer: req(text, "recovery_layer")?.into(),
            scenario_id: req(text, "scenario_id")?.into(),
            tick_count: req_u64(text, "tick_count")?,
        })
    }

    pub fn validate_complete(&self) -> Result<(), CaptureError> {
        if self.schema_id != SCHEMA_ID {
            return Err(CaptureError::Parse(format!(
                "schema_id {}",
                self.schema_id
            )));
        }
        if self.schema_version != 1 {
            return Err(CaptureError::Parse("schema_version".into()));
        }
        if self.recovery_layer != RECOVERY_LAYER_V1 {
            return Err(CaptureError::Parse(format!(
                "recovery_layer {}",
                self.recovery_layer
            )));
        }
        if self.jolt_version.is_empty() || self.joltc_commit.is_empty() {
            return Err(CaptureError::Parse("jolt pin incomplete".into()));
        }
        if self.scenario_id.is_empty() {
            return Err(CaptureError::Parse("scenario_id empty".into()));
        }
        Ok(())
    }
}
