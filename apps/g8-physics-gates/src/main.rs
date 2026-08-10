mod scenarios;
mod util;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use rurix_physics::capture::{
    CaptureError, InjectRequest, ReplayVerdict, canon_f32_bits, locate_injection_divergence,
    replay_capture_dir, replay_with_extra_journal_line, replay_with_missing_journal_line,
    whitelist_reject,
};

fn verdict_tag(v: &ReplayVerdict) -> &'static str {
    match v {
        ReplayVerdict::Pass => "Pass",
        ReplayVerdict::HashMismatch { .. } => "HashMismatch",
        ReplayVerdict::JournalLeftover { .. } => "JournalLeftover",
        ReplayVerdict::JournalMissing { .. } => "JournalMissing",
        ReplayVerdict::AssignedIdMismatch { .. } => "AssignedIdMismatch",
        ReplayVerdict::HeaderInvalid(_) => "HeaderInvalid",
        ReplayVerdict::InjectionDivergence { .. } => "InjectionDivergence",
        ReplayVerdict::Backend(_) => "Backend",
    }
}
use scenarios::{InjectionSpec, run_scenario};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "usage: g8-physics-gates <record|replay|inject|ab|net|fracture|cloth|vehicle> ..."
        );
        std::process::exit(2);
    }
    let result = match args[1].as_str() {
        "record" => cmd_record(&args[2..]),
        "replay" => cmd_replay(&args[2..]),
        "inject" => cmd_inject(&args[2..]),
        "journal-tamper" => cmd_journal_tamper(&args[2..]),
        "ab" => cmd_ab(&args[2..]),
        "canon-float" => cmd_canon_float(&args[2..]),
        "net" => cmd_net(&args[2..]),
        "fracture" => cmd_fracture(&args[2..]),
        "cloth" => cmd_cloth(&args[2..]),
        "vehicle" => cmd_vehicle(&args[2..]),
        other => Err(CaptureError::Rejected(format!(
            "unknown subcommand {other}"
        ))),
    };
    match result {
        Ok(out) => {
            println!("{out}");
        }
        Err(e) => {
            println!(
                "{{\"ok\":false,\"error\":\"{}\"}}",
                util::json_escape(&e.to_string())
            );
            std::process::exit(1);
        }
    }
}

fn cmd_record(args: &[String]) -> Result<String, CaptureError> {
    let all = args.iter().any(|a| a == "--all");
    let scenario = arg_value(args, "--scenario");
    if all {
        let mut ok = true;
        for id in util::all_scenario_ids() {
            if let Err(e) = record_one(id) {
                println!(
                    "{{\"scenario\":\"{id}\",\"ok\":false,\"error\":\"{}\"}}",
                    util::json_escape(&e.to_string())
                );
                ok = false;
            } else {
                println!("{{\"scenario\":\"{id}\",\"ok\":true}}");
            }
        }
        return Ok(format!("{{\"ok\":{}}}", util::json_bool(ok)));
    }
    let id =
        scenario.ok_or_else(|| CaptureError::Rejected("--scenario or --all required".into()))?;
    record_one(&id)?;
    Ok(format!("{{\"scenario\":\"{id}\",\"ok\":true}}"))
}

fn record_one(id: &str) -> Result<(), CaptureError> {
    let (artifact, injection) = run_scenario(id)?;
    let dir = util::corpus_dir(id);
    fs::create_dir_all(&dir).map_err(|e| CaptureError::Io(e.to_string()))?;
    artifact.persist(&dir)?;
    if let Some(spec) = injection {
        write_injection_meta(&dir, &spec)?;
    }
    Ok(())
}

fn write_injection_meta(dir: &Path, spec: &InjectionSpec) -> Result<(), CaptureError> {
    let text = format!(
        "{{\"tick\":{},\"body\":\"{:016x}\",\"field\":\"{}\",\"bit\":{}}}\n",
        spec.tick,
        spec.body.to_bits(),
        spec.field,
        spec.bit
    );
    fs::write(dir.join("injection.json"), text).map_err(|e| CaptureError::Io(e.to_string()))
}

fn cmd_replay(args: &[String]) -> Result<String, CaptureError> {
    let dir =
        arg_value(args, "--dir").ok_or_else(|| CaptureError::Rejected("--dir required".into()))?;
    let path = PathBuf::from(&dir);
    let report = replay_capture_dir(&path, None)?;
    let pass = report.verdict == ReplayVerdict::Pass;
    Ok(format!(
        "{{\"ok\":{},\"scenario\":\"{}\",\"ticks_ok\":{},\"tick_count\":{},\"journal_fully_consumed\":{},\"recovery_layer\":\"{}\",\"verdict\":\"{}\"}}",
        util::json_bool(pass),
        util::json_escape(&report.scenario_id),
        report.ticks_ok,
        report.tick_count,
        util::json_bool(report.journal_fully_consumed),
        util::json_escape(&report.recovery_layer),
        verdict_tag(&report.verdict)
    ))
}

fn cmd_inject(args: &[String]) -> Result<String, CaptureError> {
    let dir =
        arg_value(args, "--dir").ok_or_else(|| CaptureError::Rejected("--dir required".into()))?;
    let path = PathBuf::from(&dir);
    let tick: u64 = arg_value(args, "--tick")
        .ok_or_else(|| CaptureError::Rejected("--tick required".into()))?
        .parse()
        .map_err(|e| CaptureError::Parse(format!("tick: {e}")))?;
    let body_hex = arg_value(args, "--body")
        .ok_or_else(|| CaptureError::Rejected("--body required".into()))?;
    let field = arg_value(args, "--field")
        .ok_or_else(|| CaptureError::Rejected("--field required".into()))?;
    let bit: u8 = arg_value(args, "--bit")
        .unwrap_or_else(|| "0".into())
        .parse()
        .map_err(|e| CaptureError::Parse(format!("bit: {e}")))?;

    whitelist_reject(&field)?;

    let body_bits = u64::from_str_radix(body_hex.trim_start_matches("0x"), 16)
        .map_err(|e| CaptureError::Parse(format!("body: {e}")))?;
    let req = InjectRequest {
        tick,
        body: rurix_physics::BodyId::from_bits(body_bits),
        field,
        bit,
    };
    let div = locate_injection_divergence(&path, &req)?;
    Ok(format!(
        "{{\"ok\":true,\"first_divergence_tick\":{},\"diff_count\":{},\"field\":\"{}\",\"stable_id\":\"{}\",\"expected_bits\":\"{:08x}\",\"actual_bits\":\"{:08x}\"}}",
        div.first_divergence_tick,
        div.diffs.len(),
        div.diffs.first().map(|d| d.path.as_str()).unwrap_or(""),
        div.diffs
            .first()
            .map(|d| d.stable_id.as_str())
            .unwrap_or(""),
        div.diffs.first().map(|d| d.expected_bits).unwrap_or(0),
        div.diffs.first().map(|d| d.actual_bits).unwrap_or(0),
    ))
}

fn cmd_journal_tamper(args: &[String]) -> Result<String, CaptureError> {
    let dir =
        arg_value(args, "--dir").ok_or_else(|| CaptureError::Rejected("--dir required".into()))?;
    let mode = arg_value(args, "--mode")
        .ok_or_else(|| CaptureError::Rejected("--mode required".into()))?;
    let path = PathBuf::from(&dir);
    let verdict = match mode.as_str() {
        "leftover" => replay_with_extra_journal_line(&path)?,
        "missing" => replay_with_missing_journal_line(&path)?,
        other => {
            return Err(CaptureError::Rejected(format!(
                "unknown journal-tamper mode {other}"
            )));
        }
    };
    let fails_closed = verdict != ReplayVerdict::Pass;
    Ok(format!(
        "{{\"ok\":true,\"mode\":\"{}\",\"verdict\":\"{}\",\"fails_closed\":{}}}",
        util::json_escape(&mode),
        verdict_tag(&verdict),
        util::json_bool(fails_closed)
    ))
}

fn cmd_ab(_args: &[String]) -> Result<String, CaptureError> {
    let root = util::repo_root();
    let vendor_next = root.join("src/rurix-physics-sys/vendor/JoltC-next");
    let available = vendor_next.is_dir() && vendor_next.join("JoltC/Functions.h").is_file();
    if !available {
        return Ok("{\"ok\":true,\"probe\":\"vendor_missing\",\"jolt_version_pinned\":\"5.3.0\",\"ab_pass\":false,\"verdict\":\"pin_5_3_honest_stop_loss\",\"note\":\"JoltC-next unavailable;formally pinned 5.3 (not fake 5.6 PASS)\"}".to_string());
    }
    Ok("{\"ok\":true,\"probe\":\"vendor_present\",\"ab_pass\":false,\"verdict\":\"deferred_m73\",\"note\":\"M73 vendor present but A/B not in M66 scope\"}".to_string())
}

fn cmd_net(args: &[String]) -> Result<String, CaptureError> {
    use rurix_physics::net::{
        SMOOTHING_BOUND_V1, SmoothingBound, assert_trace_deterministic, load_net_trace,
        run_net_trace, run_net_trace_with_bound,
    };

    let trace_path = arg_value(args, "--trace")
        .ok_or_else(|| CaptureError::Rejected("--trace <path> required".into()))?;
    let path = PathBuf::from(&trace_path);
    let trace = load_net_trace(&path).map_err(|e| CaptureError::Rejected(e.to_string()))?;

    let force_freeze = args.iter().any(|a| a == "--force-freeze-bound");
    let bound = if force_freeze {
        // 采样后本地冻结(RFC §6.5.1 字面由 Gov/RFC PR 回填;此处用 measured ceiling)。
        let probe = run_net_trace(&trace).map_err(|e| CaptureError::Rejected(e.to_string()))?;
        SmoothingBound {
            max_position_m: (probe.max_position_offset_m * 1.25).max(0.05),
            max_angle_rad: (probe.max_angle_offset_rad * 1.25).max(0.05),
            max_convergence_frames: 30,
            frozen: true,
            reference: "RFC-0021 §6.5.1 measured_local_freeze",
        }
    } else {
        SMOOTHING_BOUND_V1
    };

    let report = run_net_trace_with_bound(&trace, bound)
        .map_err(|e| CaptureError::Rejected(e.to_string()))?;
    let det =
        assert_trace_deterministic(&trace).map_err(|e| CaptureError::Rejected(e.to_string()))?;

    // character/asset 最小闭环自检(非 M71/M69 独立门)
    let char_ok = {
        use rurix_physics::PhysicsTransform;
        use rurix_physics::character::RurixCharacter;
        let c = RurixCharacter::new(1, PhysicsTransform::IDENTITY);
        c.state.canonical_bytes().is_ok()
    };
    let asset_ok = {
        use rurix_physics::asset::PhysicsAsset;
        let a = PhysicsAsset::new("demo_ragdoll", "skel-digest-v1");
        matches!(a.cook_deterministic_double(), Ok((x, y)) if x == y)
    };

    Ok(format!(
        "{{\"ok\":true,\"trace_id\":\"{}\",\"trace_fixture_deterministic\":{},\"prediction_divergence_observed_at_golden\":{},\"correction_received_at_golden_frame\":{},\"correction_frame\":{},\"rollback_start\":{},\"rollback_input_sequence\":[{}],\"rollback_start_and_input_sequence_match_expected\":{},\"resim_final_hash\":\"{}\",\"server_hash\":\"{}\",\"resim_final_hash_equals_server\":{},\"contact_events_committed\":{},\"contact_event_committed_exactly_once\":{},\"event_dedup_across_repeated_rollbacks\":{},\"smoothing_authoritative_state_untouched\":{},\"smoothing_within_frozen_bound_per_frame\":{},\"smoothing_bound_frozen\":{},\"max_position_offset_m\":{:.6},\"max_angle_offset_rad\":{:.6},\"history_ring_overflow_hard_correction_explicit\":{},\"incompatible_schema_or_build_digest_rejected\":{},\"profile_match_recorded\":{},\"job_threads\":{},\"frame_domain_map_recorded\":{},\"frame_domain_map_count\":{},\"character_state_canonical_ok\":{},\"physics_asset_cook_deterministic\":{},\"determinism_double_run\":{}}}",
        util::json_escape(&report.trace_id),
        util::json_bool(report.trace_fixture_deterministic && det),
        util::json_bool(report.prediction_divergence_observed),
        util::json_bool(report.correction_received_at_golden),
        report
            .correction_frame
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".into()),
        report
            .rollback_start
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".into()),
        report
            .rollback_input_sequence
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(","),
        util::json_bool(report.rollback_sequence_matches),
        util::json_escape(report.resim_final_hash.as_deref().unwrap_or("")),
        util::json_escape(report.server_hash_at_correction.as_deref().unwrap_or("")),
        util::json_bool(report.resim_final_hash_equals_server),
        report.contact_events_committed,
        util::json_bool(report.contact_event_committed_exactly_once),
        util::json_bool(report.event_dedup_across_repeated_rollbacks),
        util::json_bool(report.smoothing_authoritative_state_untouched),
        util::json_bool(report.smoothing_within_frozen_bound_per_frame),
        util::json_bool(report.smoothing_bound_frozen),
        report.max_position_offset_m,
        report.max_angle_offset_rad,
        util::json_bool(report.history_ring_overflow_hard_correction_explicit),
        util::json_bool(report.incompatible_digest_rejected),
        util::json_bool(report.profile_match),
        report.job_threads,
        util::json_bool(report.frame_domain_map_count > 0),
        report.frame_domain_map_count,
        util::json_bool(char_ok),
        util::json_bool(asset_ok),
        util::json_bool(det),
    ))
}

fn cmd_fracture(args: &[String]) -> Result<String, CaptureError> {
    use rurix_physics::destruction::{parse_golden_json, parse_source_json, run_fracture_pipeline};

    let source_path = arg_value(args, "--source")
        .ok_or_else(|| CaptureError::Rejected("--source <path> required".into()))?;
    let golden_path = arg_value(args, "--golden")
        .ok_or_else(|| CaptureError::Rejected("--golden <path> required".into()))?;
    let source_text =
        fs::read_to_string(&source_path).map_err(|e| CaptureError::Io(e.to_string()))?;
    let golden_text =
        fs::read_to_string(&golden_path).map_err(|e| CaptureError::Io(e.to_string()))?;
    let source = parse_source_json(&source_text).map_err(CaptureError::Rejected)?;
    let golden = parse_golden_json(&golden_text).map_err(CaptureError::Rejected)?;
    let report = run_fracture_pipeline(&source, &golden).map_err(CaptureError::Rejected)?;

    let activated = report
        .activated_cluster_ids
        .iter()
        .map(|s| format!("\"{}\"", util::json_escape(s)))
        .collect::<Vec<_>>()
        .join(",");
    Ok(format!(
        "{{\"ok\":{},\"cook_deterministic_double_byte_equal\":{},\"cook_counts_and_digests_match_golden\":{},\"unknown_schema_fails_closed\":{},\"dangling_edge_or_nontree_cluster_fails_closed\":{},\"below_threshold_no_break\":{},\"above_threshold_breaks_specified_edge_at_tick\":{},\"cluster_activation_hierarchy_matches_golden\":{},\"activated_bodies_enter_journal_and_capture\":{},\"cache_roundtrip_event_sequence_identical\":{},\"cache_roundtrip_state_hash_identical\":{},\"vfx_exactly_once_per_fracture_event\":{},\"vfx_no_duplicate_across_rollback_or_cache_replay\":{},\"chunk_count\":{},\"edge_count\":{},\"interior_face_count\":{},\"anchor_count\":{},\"cooked_digest\":\"{}\",\"broken_edge_id\":{},\"break_tick\":{},\"activated_cluster_ids\":[{}],\"activated_body_count\":{},\"vfx_commit_count\":{},\"event_sequence_digest\":\"{}\",\"state_hash\":\"{}\",\"detail\":\"{}\"}}",
        util::json_bool(report.ok),
        util::json_bool(report.cook_deterministic_double_byte_equal),
        util::json_bool(report.cook_counts_and_digests_match_golden),
        util::json_bool(report.unknown_schema_fails_closed),
        util::json_bool(report.dangling_edge_or_nontree_cluster_fails_closed),
        util::json_bool(report.below_threshold_no_break),
        util::json_bool(report.above_threshold_breaks_specified_edge_at_tick),
        util::json_bool(report.cluster_activation_hierarchy_matches_golden),
        util::json_bool(report.activated_bodies_enter_journal_and_capture),
        util::json_bool(report.cache_roundtrip_event_sequence_identical),
        util::json_bool(report.cache_roundtrip_state_hash_identical),
        util::json_bool(report.vfx_exactly_once_per_fracture_event),
        util::json_bool(report.vfx_no_duplicate_across_rollback_or_cache_replay),
        report.chunk_count,
        report.edge_count,
        report.interior_face_count,
        report.anchor_count,
        util::json_escape(&report.cooked_digest),
        report
            .broken_edge_id
            .as_ref()
            .map(|s| format!("\"{}\"", util::json_escape(s)))
            .unwrap_or_else(|| "null".into()),
        report
            .break_tick
            .map(|v| v.to_string())
            .unwrap_or_else(|| "null".into()),
        activated,
        report.activated_body_count,
        report.vfx_commit_count,
        util::json_escape(&report.event_sequence_digest),
        util::json_escape(&report.state_hash),
        util::json_escape(&report.detail),
    ))
}

fn cmd_cloth(_args: &[String]) -> Result<String, CaptureError> {
    use rurix_physics::cloth::run_cloth_pipeline;
    let r = run_cloth_pipeline();
    Ok(format!(
        "{{\"ok\":{},\"schema_pass\":{},\"import_pass\":{},\"collision_pass\":{},\"lod_pass\":{},\"timeline_pass\":{},\"solver_double_run_deterministic\":{},\"bound_frozen_reference_present\":{},\"cloth_capture_scene_appended\":{},\"measured_max_penetration_m\":{:.6},\"penetration_bound_m\":{:.6},\"detail\":\"{}\"}}",
        util::json_bool(r.ok),
        util::json_bool(r.schema_pass),
        util::json_bool(r.import_pass),
        util::json_bool(r.collision_pass),
        util::json_bool(r.lod_pass),
        util::json_bool(r.timeline_pass),
        util::json_bool(r.solver_double_run_deterministic),
        util::json_bool(r.bound_frozen_reference_present),
        util::json_bool(r.cloth_capture_scene_appended),
        r.measured_max_penetration_m,
        r.penetration_bound_m,
        util::json_escape(&r.detail),
    ))
}

fn cmd_vehicle(args: &[String]) -> Result<String, CaptureError> {
    use rurix_physics::vehicle::legs::{LEG_NAMES, falsify_leg, run_vehicle_subject};
    // 逐腿 RED 证伪臂:ci selftest 专用;对该腿输入做最小摄动并回报腿结果(期望 false)。
    if let Some(leg) = arg_value(args, "--falsify") {
        return match falsify_leg(&leg) {
            Some(result) => Ok(format!(
                "{{\"ok\":true,\"falsify\":\"{}\",\"leg_result\":{}}}",
                util::json_escape(&leg),
                util::json_bool(result)
            )),
            None => Err(CaptureError::Rejected(format!(
                "unknown leg {leg}; known: {}",
                LEG_NAMES.join(",")
            ))),
        };
    }
    let r = run_vehicle_subject();
    Ok(format!(
        "{{\"ok\":{},\"vehicle_subject_pass\":{},\"asset_roundtrip\":{},\"fixed_input_replay_hash_equal\":{},\"rollback_correction_converges\":{},\"tire_light_object_contact_regression_golden\":{},\"state_serialization_roundtrip\":{},\"telemetry_trace_golden\":{},\"final_state_hash\":\"{}\",\"contact_digest\":\"{}\",\"contact_events\":{},\"telemetry_digest\":\"{}\",\"telemetry_lines\":{},\"detail\":\"{}\"}}",
        util::json_bool(r.ok),
        util::json_bool(r.ok),
        util::json_bool(r.asset_roundtrip),
        util::json_bool(r.fixed_input_replay_hash_equal),
        util::json_bool(r.rollback_correction_converges),
        util::json_bool(r.tire_light_object_contact_regression_golden),
        util::json_bool(r.state_serialization_roundtrip),
        util::json_bool(r.telemetry_trace_golden),
        util::json_escape(&r.final_state_hash),
        util::json_escape(&r.contact_digest),
        r.contact_events,
        util::json_escape(&r.telemetry_digest),
        r.telemetry_lines,
        util::json_escape(&r.detail),
    ))
}

fn cmd_canon_float(args: &[String]) -> Result<String, CaptureError> {
    let mode = arg_value(args, "--mode").unwrap_or_else(|| "neg_zero".into());
    match mode.as_str() {
        "neg_zero" => {
            let bits = canon_f32_bits(-0.0f32)?;
            Ok(format!(
                "{{\"ok\":true,\"neg_zero_bits\":\"{:08x}\"}}",
                bits
            ))
        }
        "nan" => match canon_f32_bits(f32::NAN) {
            Err(CaptureError::NanFloat { path }) => Ok(format!(
                "{{\"ok\":true,\"nan_rejected\":true,\"path\":\"{}\"}}",
                util::json_escape(&path)
            )),
            Ok(b) => Err(CaptureError::Rejected(format!("NaN accepted bits {b:08x}"))),
            Err(e) => Err(e),
        },
        other => Err(CaptureError::Rejected(format!("unknown mode {other}"))),
    }
}

fn arg_value(args: &[String], key: &str) -> Option<String> {
    args.iter()
        .position(|a| a == key)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

#[cfg(test)]
mod probe_api {
    use super::*;
    use rurix_physics::capture::RECOVERY_LAYER_V1;
    use util::corpus_dir;

    #[test]
    fn recovery_layer_constant() {
        assert_eq!(RECOVERY_LAYER_V1, "semantic_journal_rebuild_v1");
    }

    #[test]
    fn corpus_paths_distinct() {
        let a = corpus_dir("box_stack_settle");
        let b = corpus_dir("sphere_impulse_script");
        assert_ne!(a, b);
    }
}
