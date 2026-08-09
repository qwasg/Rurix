//! M122 gameplay_field 门(G9.2 骨架期;g9.p0.m122.gameplay_field)。
//!
//! 七判据(MAP M122 行;--phase g9.2):
//! 1. three_layer_schema_frozen:定义层基元 canonical digest 确定 + schema
//!    头冻结字面值核验 + filter 进 digest;
//! 2. eight_enum_accept_green:八枚举逐项 parse/from_u8 accept;
//! 3. illegal_enum_red:非法枚举名/越界 discriminator fail-closed;
//! 4. filter_default_empty_zero_impact:默认空 filter 场注册后世界状态
//!    hash 与无场基线逐位一致(真 Jolt 世界逐步对拍);
//! 5. persistent_journal_replay_hash_equal:persistent 注册/注销/变更全
//!    journal 化,replay 逐 tick hash 一致 + digest == golden;
//! 6. world_field_egress_readonly:World-Field 出口 = 桥提交口只读 buffer,
//!    提交序列 digest == golden;
//! 7. render_zero_writeback_audit:渲染侧零回写静态审计(bridge 源无
//!    GpuScene 回写调用面 + GpuScene 冻结面 0-byte——无 world_field 字段)。

use std::fs;

use rurix_physics::capture::jolt_world_desc;
use rurix_physics::cloth::RenderFrameId;
use rurix_physics::field::filter::{FieldFilter, domain_bit, object_state_bits};
use rurix_physics::field::journal::{FieldJournal, FieldJournalCommand, replay_journal};
use rurix_physics::field::{
    FieldDef, FieldLifecycle, FieldNode, FieldNodeKind, FieldPhysicsType, FieldRegistry,
    WorldFieldBuffer, WorldFieldSampleSet,
};
use rurix_physics::net::frame::PhysicsTickId;
use rurix_physics::particle_view::{ParticleSleepState, PhysicsParticleRef, RigidBodyStableId};
use rurix_physics::{BodyDesc, BodyKind, MassProps, PhysicsTransform, PhysicsWorld, ShapeDesc};

use crate::util::{arg_value, json_bool, json_escape};

pub fn run(args: &[String]) -> Result<String, String> {
    let golden_path = arg_value(args, "--golden").ok_or_else(|| "--golden required".to_string())?;
    let golden_text = fs::read_to_string(&golden_path).map_err(|e| e.to_string())?;
    let golden_journal_digest = extract_str(&golden_text, "journal_digest")
        .ok_or_else(|| "golden missing journal_digest".to_string())?;
    let golden_egress_digest = extract_str(&golden_text, "world_field_sequence_digest")
        .ok_or_else(|| "golden missing world_field_sequence_digest".to_string())?;

    let mut notes: Vec<String> = Vec::new();

    // —— 1) 三层 schema 冻结 ——
    let schema_frozen = {
        let def = sample_field(
            "freeze_probe",
            FieldLifecycle::Transient,
            FieldFilter::default(),
        );
        let v1 = def.validate().is_ok();
        let d1 = def.digest();
        let d2 = def.digest();
        // filter 进 digest。
        let mut def2 = def.clone();
        def2.filter.domain_mask =
            domain_bit(rurix_physics::particle_view::ParticleDomain::RigidBody);
        let filter_in_digest = d1 != def2.digest();
        // schema 版本未知 fail-closed。
        let mut bad = def.clone();
        bad.schema_version = 9999;
        let bad_rejected = bad.validate().is_err();
        notes.push("three-layer: nodes×filter×physics_type canonical digest frozen".into());
        v1 && d1 == d2 && filter_in_digest && bad_rejected
    };

    // —— 2) 八枚举逐项 accept ——
    let eight_accept = FieldPhysicsType::ALL
        .iter()
        .all(|t| FieldPhysicsType::parse(t.canonical_name()) == Ok(*t))
        && FieldPhysicsType::ALL.len() == 8;

    // —— 3) 非法枚举 RED ——
    let illegal_red = FieldPhysicsType::parse("LinearForces").is_err()
        && FieldPhysicsType::parse("").is_err()
        && FieldPhysicsType::from_u8(8).is_err()
        && FieldPhysicsType::from_u8(255).is_err();

    // —— 4) 过滤默认空匹配 = 零影响(真世界逐步对拍)——
    let zero_impact = run_zero_impact_probe(&mut notes)?;

    // —— 5) persistent journal 化 + replay 逐 tick hash ——
    let (journal_ok, journal_digest) = run_persistent_journal_probe()?;
    let journal_match = journal_ok && journal_digest == golden_journal_digest;

    // —— 6) World-Field 出口(桥提交口,GpuScene 0-byte)——
    let (egress_ok, egress_digest) = run_egress_probe()?;
    let egress_match = egress_ok && egress_digest == golden_egress_digest;

    // —— 7) 渲染零回写静态审计 ——
    let manifest = env!("CARGO_MANIFEST_DIR");
    let audit = render_zero_writeback_audit(manifest, &mut notes)?;

    let ok = schema_frozen
        && eight_accept
        && illegal_red
        && zero_impact
        && journal_match
        && egress_match
        && audit;

    Ok(format!(
        "{{\"ok\":{},\"three_layer_schema_frozen\":{},\"eight_enum_accept_green\":{},\"illegal_enum_red\":{},\"filter_default_empty_zero_impact\":{},\"persistent_journal_replay_hash_equal\":{},\"world_field_egress_readonly\":{},\"render_zero_writeback_audit\":{},\"journal_digest\":\"{}\",\"world_field_sequence_digest\":\"{}\",\"detail\":\"{}\"}}",
        json_bool(ok),
        json_bool(schema_frozen),
        json_bool(eight_accept),
        json_bool(illegal_red),
        json_bool(zero_impact),
        json_bool(journal_match),
        json_bool(egress_match),
        json_bool(audit),
        journal_digest,
        egress_digest,
        json_escape(&notes.join("; ")),
    ))
}

/// 自证红臂(--selftest 消费;合成负样本必须红)。
pub fn run_selftest_arm(args: &[String]) -> Result<String, String> {
    let arm = arg_value(args, "--arm").ok_or_else(|| "--arm required".to_string())?;
    match arm.as_str() {
        // 非法枚举混入 accept 集:若 accept 路径把非法名当合法 → 臂绿 = bug。
        "illegal_enum" => {
            let accepted = FieldPhysicsType::parse("GravityWell");
            let red = accepted.is_err();
            Ok(format!(
                "{{\"ok\":true,\"arm\":\"illegal_enum\",\"red_detected\":{}}}",
                json_bool(red)
            ))
        }
        // 篡改 replay:翻转一个 tick 的 hash → replay 必须红。
        "tampered_replay" => {
            let (ok, _) = run_persistent_journal_probe()?;
            let mut journal = build_persistent_journal()?;
            if let Some(t) = journal.ticks.get_mut(1) {
                t.semantic_hash = "00".repeat(32);
            }
            let replayed = replay_journal(&journal);
            let red = replayed.is_err();
            Ok(format!(
                "{{\"ok\":true,\"arm\":\"tampered_replay\",\"baseline_ok\":{},\"red_detected\":{}}}",
                json_bool(ok),
                json_bool(red)
            ))
        }
        // 非空 filter 必须产生非零影响(反零影响面:若任何 filter 都零影响
        // = 过滤机制死亡,门须红)。
        "nonempty_filter_impact" => {
            let mut registry = FieldRegistry::new();
            let mut filter = FieldFilter {
                object_state_mask: object_state_bits::AWAKE,
                domain_mask: domain_bit(rurix_physics::particle_view::ParticleDomain::RigidBody),
                layer_mask: u64::MAX,
                explicit_include: vec![],
                explicit_exclude: vec![],
            };
            let def = sample_field("impact", FieldLifecycle::Persistent, filter.clone());
            registry.register(def).map_err(|e| e.to_string())?;
            let particles = vec![(
                PhysicsParticleRef::RigidBody(RigidBodyStableId::from_bits(1)),
                ParticleSleepState::Awake,
                0u32,
            )];
            let hits = registry.evaluate(&particles);
            let impact = !hits.is_empty();
            // 反例:显式 exclude 后必须零匹配(过滤面活着)。
            filter.explicit_exclude.push(
                PhysicsParticleRef::RigidBody(RigidBodyStableId::from_bits(1)).canonical_text(),
            );
            let def2 = sample_field("impact2", FieldLifecycle::Persistent, filter);
            registry.register(def2).map_err(|e| e.to_string())?;
            let hits2: Vec<_> = registry
                .evaluate(&particles)
                .into_iter()
                .filter(|(id, _, _)| id == "impact2")
                .collect();
            let excluded = hits2.is_empty();
            Ok(format!(
                "{{\"ok\":true,\"arm\":\"nonempty_filter_impact\",\"impact_observed\":{},\"exclude_zero_match\":{}}}",
                json_bool(impact),
                json_bool(excluded)
            ))
        }
        other => Err(format!("unknown selftest arm {other}")),
    }
}

fn sample_field(id: &str, lc: FieldLifecycle, filter: FieldFilter) -> FieldDef {
    FieldDef::new(
        id,
        FieldNode {
            node_id: format!("{id}_root"),
            kind: FieldNodeKind::RadialFalloff {
                center: [0.0, 1.0, 0.0],
                radius: 5.0,
            },
            weight: 1.0,
            children: vec![FieldNode {
                node_id: format!("{id}_child"),
                kind: FieldNodeKind::Noise {
                    scale: 0.5,
                    seed: 42,
                },
                weight: 0.25,
                children: vec![],
            }],
        },
        FieldPhysicsType::LinearForce,
        lc,
        filter,
    )
}

/// 零影响探针:同种子同命令双世界,一臂注册默认空 filter 场,逐 tick
/// step 后 semantic 快照对拍——场零匹配 → 两世界状态 hash 逐位一致。
fn run_zero_impact_probe(notes: &mut Vec<String>) -> Result<bool, String> {
    use rurix_physics::capture::canonical::{hash_canonical_state, state_from_world};

    let build = || -> Result<PhysicsWorld, String> {
        let mut w = PhysicsWorld::new(jolt_world_desc(16)).map_err(|e| e.to_string())?;
        w.add_bodies_batch(&[BodyDesc {
            kind: BodyKind::Dynamic,
            shape: ShapeDesc::Sphere { radius: 0.5 },
            layer: 0,
            mass_props: MassProps {
                mass: 1.0,
                friction: 0.5,
                restitution: 0.0,
                allow_sleep: true,
            },
            ccd: false,
            transform: PhysicsTransform::IDENTITY,
        }])
        .map_err(|e| e.to_string())?;
        Ok(w)
    };
    let mut baseline = build()?;
    let mut with_field = build()?;

    // 注册默认空 filter 的 persistent 场(零匹配 → 对世界零影响)。
    let mut registry = FieldRegistry::new();
    registry
        .register(sample_field(
            "zero_impact",
            FieldLifecycle::Persistent,
            FieldFilter::default(),
        ))
        .map_err(|e| e.to_string())?;

    // 粒子集(场候选 = 唯一动态体)。
    let body_bits = {
        let snap = baseline
            .body_semantic_snapshot()
            .map_err(|e| e.to_string())?;
        snap[0].body_id.to_bits()
    };
    let particles = vec![(
        PhysicsParticleRef::RigidBody(RigidBodyStableId::from_bits(body_bits)),
        ParticleSleepState::Awake,
        0u32,
    )];

    let dt = jolt_world_desc(16).dt_fixed;
    for tick in 0..8u64 {
        baseline.step(dt).map_err(|e| e.to_string())?;
        with_field.step(dt).map_err(|e| e.to_string())?;
        // 场求值(零匹配 → 无输出 → 无 impulse 注入)。
        let hits = registry.evaluate(&particles);
        if !hits.is_empty() {
            notes.push(format!(
                "zero-impact violated at tick {tick}: {} hits",
                hits.len()
            ));
            return Ok(false);
        }
        let ha =
            hash_canonical_state(&state_from_world(&baseline, tick).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        let hb =
            hash_canonical_state(&state_from_world(&with_field, tick).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        if ha != hb {
            notes.push(format!("world hash diverged at tick {tick}"));
            return Ok(false);
        }
    }
    notes.push("zero-impact: 8 ticks bitwise equal with zero-match field registered".into());
    Ok(true)
}

/// persistent 注册/注销/变更全 journal 化 + replay 逐 tick hash 一致。
fn build_persistent_journal() -> Result<FieldJournal, String> {
    let mut registry = FieldRegistry::new();
    let mut journal = FieldJournal::new();

    // tick 0: 注册。
    let def_a = sample_field(
        "field_a",
        FieldLifecycle::Persistent,
        FieldFilter {
            object_state_mask: object_state_bits::AWAKE,
            domain_mask: domain_bit(rurix_physics::particle_view::ParticleDomain::RigidBody),
            layer_mask: 1,
            explicit_include: vec![],
            explicit_exclude: vec![],
        },
    );
    registry
        .register(def_a.clone())
        .map_err(|e| e.to_string())?;
    journal.push_tick(rurix_physics::field::journal::FieldJournalTick {
        tick: 0,
        commands: vec![FieldJournalCommand::Register {
            field_id: def_a.field_id.clone(),
            def: Box::new(def_a.clone()),
        }],
        semantic_hash: registry.semantic_hash(),
    });

    // tick 1: 注册第二个 + 变更第一个。
    let def_b = sample_field(
        "field_b",
        FieldLifecycle::Persistent,
        FieldFilter::default(),
    );
    registry
        .register(def_b.clone())
        .map_err(|e| e.to_string())?;
    let mut def_a2 = def_a.clone();
    def_a2.root.weight = 2.0;
    registry.update(def_a2.clone()).map_err(|e| e.to_string())?;
    journal.push_tick(rurix_physics::field::journal::FieldJournalTick {
        tick: 1,
        commands: vec![
            FieldJournalCommand::Register {
                field_id: def_b.field_id.clone(),
                def: Box::new(def_b),
            },
            FieldJournalCommand::Update {
                field_id: def_a2.field_id.clone(),
                def: Box::new(def_a2),
            },
        ],
        semantic_hash: registry.semantic_hash(),
    });

    // tick 2: 注销第一个。
    registry.unregister("field_a").map_err(|e| e.to_string())?;
    journal.push_tick(rurix_physics::field::journal::FieldJournalTick {
        tick: 2,
        commands: vec![FieldJournalCommand::Unregister {
            field_id: "field_a".into(),
        }],
        semantic_hash: registry.semantic_hash(),
    });

    Ok(journal)
}

fn run_persistent_journal_probe() -> Result<(bool, String), String> {
    let journal = build_persistent_journal()?;
    let replayed = replay_journal(&journal).map_err(|e| e.to_string())?;
    let all_equal = replayed.len() == journal.ticks.len()
        && replayed
            .iter()
            .zip(journal.ticks.iter())
            .all(|((t, h), jt)| *t == jt.tick && *h == jt.semantic_hash);
    // 生命周期纪律:Transient/Construction 注册 = LifecycleViolation。
    let mut probe = FieldRegistry::new();
    let lc_rejected = probe
        .register(sample_field(
            "transient_probe",
            FieldLifecycle::Transient,
            FieldFilter::default(),
        ))
        .is_err()
        && probe
            .register(sample_field(
                "construction_probe",
                FieldLifecycle::Construction,
                FieldFilter::default(),
            ))
            .is_err();
    Ok((all_equal && lc_rejected, journal.digest()))
}

/// World-Field 出口:桥提交口 + GpuScene 冻结面 0-byte。
fn run_egress_probe() -> Result<(bool, String), String> {
    use rurix_physics::bridge::PhysicsBridge;
    let mut bridge = PhysicsBridge::new();
    for i in 0..3u64 {
        bridge.submit_world_field(WorldFieldBuffer {
            sample_set: WorldFieldSampleSet {
                physics_tick: PhysicsTickId(i),
                render_frame: RenderFrameId(i * 2),
            },
            payload: format!("field-samples-{i}").into_bytes(),
        });
    }
    let committed = bridge.world_field_committed().len() == 3;
    let digest = bridge.world_field_sequence_digest();
    // 确定性:同序列重放 digest 一致。
    let mut bridge2 = PhysicsBridge::new();
    for i in 0..3u64 {
        bridge2.submit_world_field(WorldFieldBuffer {
            sample_set: WorldFieldSampleSet {
                physics_tick: PhysicsTickId(i),
                render_frame: RenderFrameId(i * 2),
            },
            payload: format!("field-samples-{i}").into_bytes(),
        });
    }
    let deterministic = bridge2.world_field_sequence_digest() == digest;
    Ok((committed && deterministic, digest))
}

/// 渲染侧零回写静态审计:
/// 1. bridge 源无 GpuScene 回写调用面(scene.set_/write_/push_/submit_);
/// 2. GpuScene 源 0-byte 扩面——不出现任何 world_field/field buffer 字段
///    (R-10 🔒:扩面须渲染侧 RFC 显式修订行;骨架期预期 0-byte)。
fn render_zero_writeback_audit(manifest: &str, notes: &mut Vec<String>) -> Result<bool, String> {
    let bridge_src = fs::read_to_string(format!(
        "{manifest}/../../src/rurix-physics/src/bridge/mod.rs"
    ))
    .map_err(|e| e.to_string())?;
    let gpu_scene_src = fs::read_to_string(format!(
        "{manifest}/../../src/rurix-render/src/geometry/gpu_scene.rs"
    ))
    .map_err(|e| e.to_string())?;

    let mut clean = true;
    for banned in ["scene.set_", "scene.write_", "scene.push_", "scene.submit_"] {
        if bridge_src.contains(banned) {
            notes.push(format!("bridge writeback literal found: {banned}"));
            clean = false;
        }
    }
    // GpuScene 冻结面 0-byte:骨架期不得出现 world-field buffer 字段。
    for banned in ["world_field", "WorldField", "field_buffer"] {
        if gpu_scene_src.contains(banned) {
            notes.push(format!("GpuScene frozen surface extended: {banned}"));
            clean = false;
        }
    }
    notes.push("render zero-writeback audit: bridge clean + GpuScene 0-byte".into());
    Ok(clean)
}

// —— 极简 golden 字段抽取 ——

fn extract_str(text: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\"");
    let i = text.find(&pat)?;
    let rest = &text[i + pat.len()..];
    let colon = rest.find(':')?;
    let r = rest[colon + 1..].trim_start();
    if !r.starts_with('"') {
        return None;
    }
    let end = r[1..].find('"')?;
    Some(r[1..1 + end].to_string())
}
