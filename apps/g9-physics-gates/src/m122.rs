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
pub(crate) fn build_persistent_journal() -> Result<FieldJournal, String> {
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

/// 渲染侧零回写静态审计(RFC-0024 v1.1 章 F2 🔒 修订行面;F2 RED 臂字面:
/// 骨架期「GpuScene 0-byte 扩面」断言面在完整期改按本修订行面核验——
/// world_field buffer 仅经本行授权面出现):
/// 1. bridge 源无 GpuScene 回写调用面(scene.set_/write_/push_/submit_);
/// 2. GpuScene world-field 面 = F2 授权加性面恰在位(commit_world_field
///    唯一写口 / world_field_slots 只读消费 / render_write_world_field
///    fail-closed 守卫),类型面无 &mut 访问器;
/// 3. GpuScene 既有冻结面字面 0-byte(update_transform/flush_dirty 签名在位);
/// 4. `.commit_world_field(` 生产代码调用点唯一 = Physics→GpuScene 桥
///    (旁路提交注入 RED 臂的静态面)。
fn render_zero_writeback_audit(manifest: &str, notes: &mut Vec<String>) -> Result<bool, String> {
    world_field_f2_authorized_audit(manifest, notes)
}

/// F2 授权面机器核验(完整期门与骨架期审计共用;见上四条)。
pub(crate) fn world_field_f2_authorized_audit(
    manifest: &str,
    notes: &mut Vec<String>,
) -> Result<bool, String> {
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
    // F2 授权加性面恰在位。
    for required in [
        "pub fn commit_world_field(",
        "pub fn world_field_slots(&self) -> &[WorldFieldSlot]",
        "pub fn render_write_world_field(",
        "RenderWriteRejected",
    ] {
        if !gpu_scene_src.contains(required) {
            notes.push(format!("GpuScene F2 authorized face missing: {required}"));
            clean = false;
        }
    }
    // 类型面无 world-field 可变访问器(渲染只读消费)。
    for banned in ["&mut WorldFieldSlot", "world_field_slots_mut"] {
        if gpu_scene_src.contains(banned) {
            notes.push(format!("GpuScene mutable world-field accessor found: {banned}"));
            clean = false;
        }
    }
    // GpuScene 既有冻结面 0-byte(签名字面在位)。
    for required in [
        "pub fn update_transform(&mut self, instance_id: u32, transform: [[f32; 4]; 3]) -> bool",
        "pub fn flush_dirty(&mut self) -> Vec<DirtyRange>",
    ] {
        if !gpu_scene_src.contains(required) {
            notes.push(format!("GpuScene legacy frozen surface drifted: {required}"));
            clean = false;
        }
    }
    // 唯一提交点:生产代码(剔除 #[cfg(test)] 段)内 `.commit_world_field(`
    // 只允许出现在 physics bridge。
    let gpu_prod = gpu_scene_src
        .split("#[cfg(test)]")
        .next()
        .unwrap_or(&gpu_scene_src);
    if gpu_prod.contains(".commit_world_field(") {
        notes.push("GpuScene production code self-commits world-field".into());
        clean = false;
    }
    let physics_src = format!("{manifest}/../../src/rurix-physics/src");
    let mut files = Vec::new();
    collect_rs_files(std::path::Path::new(&physics_src), &mut files);
    for f in files {
        let name = f.to_string_lossy().replace('\\', "/");
        let text = fs::read_to_string(&f).map_err(|e| e.to_string())?;
        let prod = text.split("#[cfg(test)]").next().unwrap_or(&text);
        if prod.contains(".commit_world_field(")
            && !name.ends_with("src/rurix-physics/src/bridge/mod.rs")
        {
            notes.push(format!("bypass commit site outside bridge: {name}"));
            clean = false;
        }
    }
    if !bridge_src.contains(".commit_world_field(") {
        notes.push("bridge world-field commit call site missing".into());
        clean = false;
    }
    notes.push("F2 authorized-face audit: bridge clean + GpuScene face exact + unique commit site".into());
    Ok(clean)
}

fn collect_rs_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten() {
        let p = entry.path();
        if p.is_dir() {
            collect_rs_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// World-Field GpuScene 承载探针(F2 修订行面):桥完整期提交口按 tick 提交
/// → GpuScene 只读 buffer 逐位承载;渲染侧只读消费可读;写尝试 typed Err;
/// 序列 digest 确定性双跑一致。
pub(crate) fn world_field_scene_probe() -> (bool, String, bool) {
    use rurix_physics::bridge::PhysicsBridge;
    use rurix_physics::cloth::RenderFrameId;
    use rurix_physics::field::{WorldFieldBuffer, WorldFieldSampleSet};
    use rurix_physics::net::frame::PhysicsTickId;
    use rurix_render::geometry::gpu_scene::{GpuScene, WorldFieldSlot, WorldFieldWriteError};

    let mk = |i: u64| WorldFieldBuffer {
        sample_set: WorldFieldSampleSet {
            physics_tick: PhysicsTickId(i),
            render_frame: RenderFrameId(i * 2),
        },
        payload: format!("g96-field-samples-{i}").into_bytes(),
    };
    let mut bridge = PhysicsBridge::new();
    let mut scene = GpuScene::new();
    for i in 0..3u64 {
        bridge.commit_world_field_to_scene(&mut scene, mk(i));
    }
    // GpuScene 承载 = 桥提交载荷逐位一致(渲染只读消费面);WorldFieldSampleSet
    // 时间域(physics_tick × render_frame)显式成对(R-4 🔒)。
    let slots = scene.world_field_slots();
    let carried = slots.len() == 3
        && slots
            .iter()
            .zip(bridge.world_field_committed())
            .all(|(s, b)| {
                s.physics_tick == b.sample_set.physics_tick.0
                    && s.render_frame == b.sample_set.render_frame.0
                    && s.payload == b.payload
            });
    // 确定性:同序列重放 digest 逐位一致。
    let mut bridge2 = PhysicsBridge::new();
    let mut scene2 = GpuScene::new();
    for i in 0..3u64 {
        bridge2.commit_world_field_to_scene(&mut scene2, mk(i));
    }
    let deterministic =
        bridge2.world_field_sequence_digest() == bridge.world_field_sequence_digest();
    // 渲染侧写/回写尝试 = fail-closed typed Err + 零状态变化。
    let before = scene.world_field_slots().to_vec();
    let rejected = scene.render_write_world_field(WorldFieldSlot {
        physics_tick: 9,
        render_frame: 18,
        payload: vec![9],
    });
    let render_err = matches!(rejected, Err(WorldFieldWriteError::RenderWriteRejected))
        && scene.world_field_slots() == before.as_slice();
    (
        carried && deterministic,
        bridge.world_field_sequence_digest(),
        render_err,
    )
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

// —— G9.6 完整期门(--phase g9.6;spec/physics.md RXS-0375,判据逐字引
// G9_ACCEPTANCE_MAP §2 M122 行;RFC-0024 §4.B + v1.1 章 F2)——
//
// 完整期判据面:
// 1. field_drives_dynamic_response_full:场求值实际驱动力学响应(经
//    RXS-0374 耦合面消费 impulse/force——活性场世界 digest ≠ 无场基线);
// 2. filter_default_empty_zero_impact_full:过滤默认空匹配 = 零影响完整期
//    重验(场注册但零匹配时世界状态 hash 与无场基线逐位一致);
// 3. persistent_journal_mainstream_replay_equal:persistent 注册/注销/变更
//    全 journal 化(并入 capture 主流)且 replay 逐 tick hash 一致完整期重验;
// 4. world_field_egress_unique_authorized_f2:World-Field 唯一出口 =
//    GpuScene 只读 buffer(F2 授权面恰在位 + 唯一提交点 + 既有面 0-byte);
// 5. render_write_injection_rejected:渲染侧写/回写注入 fail-closed
//    typed Err(RED 臂独立有效);
// 6. bypass_submit_detected:绕过桥的旁路提交注入被一致性对拍检出(RED
//    臂独立有效);
// 7. conformance_anchor_consumed:RXS-0374/0375 锚定语料消费核验;
// 8. measured_freeze_digest_match:measured 冻结带对拍(禁手写 golden)。

/// 完整期门(`field-full --freeze <path>`;`--emit-freeze` = measured 冻结
/// 带生成口)。门序硬约束(M121 完整期未绿 → 本门不得验收)由门脚本层
/// 机器核验(ci/g9_physics_interlock.py,沿 g9_gi_interlock 先例)。
pub fn run_full(args: &[String]) -> Result<String, String> {
    use rurix_physics::field::capture_merge::{
        FieldPresence, record_field_capture, replay_field_capture,
    };

    let emit_freeze = args.iter().any(|a| a == "--emit-freeze");
    let freeze_path = arg_value(args, "--freeze");

    let mut notes: Vec<String> = Vec::new();
    let spec_of = |presence: FieldPresence| {
        rurix_physics::field::capture_merge::FieldCaptureSpec {
            scenario_id: "g96_m122_gameplay_field_full".into(),
            ticks: 8,
            presence,
        }
    };

    // —— 1) 场求值实际驱动力学响应 ——
    let active = record_field_capture(&spec_of(FieldPresence::Active)).map_err(|e| e.to_string())?;
    let absent = record_field_capture(&spec_of(FieldPresence::Absent)).map_err(|e| e.to_string())?;
    let drives = active.applied_impulse_count > 0 && active.world_digest != absent.world_digest;
    notes.push(format!(
        "field drives motion: applied={} digest_diverged={}",
        active.applied_impulse_count,
        active.world_digest != absent.world_digest
    ));

    // —— 2) 过滤默认空匹配 = 零影响完整期重验 ——
    let nomatch =
        record_field_capture(&spec_of(FieldPresence::NoMatch)).map_err(|e| e.to_string())?;
    let zero_impact = nomatch.applied_impulse_count == 0
        && nomatch.world_digest == absent.world_digest
        && nomatch.field_chain_digest != absent.field_chain_digest;
    notes.push(format!(
        "zero-impact re-verify: nomatch==absent={}",
        nomatch.world_digest == absent.world_digest
    ));

    // —— 3) persistent 全 journal 化 + 主流 replay 逐 tick hash 一致重验 ——
    let cap_dir =
        std::env::temp_dir().join(format!("g9_m122_field_capture_{}", std::process::id()));
    let _ = fs::remove_dir_all(&cap_dir);
    active
        .artifact
        .persist(&cap_dir)
        .map_err(|e| e.to_string())?;
    let replay_active = replay_field_capture(&cap_dir);
    let _ = fs::remove_dir_all(&cap_dir);
    let replay_active = replay_active.map_err(|e| e.to_string())?;
    let nomatch_dir =
        std::env::temp_dir().join(format!("g9_m122_field_capture_nm_{}", std::process::id()));
    let _ = fs::remove_dir_all(&nomatch_dir);
    nomatch
        .artifact
        .persist(&nomatch_dir)
        .map_err(|e| e.to_string())?;
    let replay_nomatch = replay_field_capture(&nomatch_dir);
    let _ = fs::remove_dir_all(&nomatch_dir);
    let replay_nomatch = replay_nomatch.map_err(|e| e.to_string())?;
    let mainstream_replay = replay_active.journal_fully_consumed
        && replay_active.field_hash_matched
        && replay_active.impulses_recomputed_equal
        && replay_active.world_digest == active.world_digest
        && replay_active.field_chain_digest == active.field_chain_digest
        && replay_nomatch.journal_fully_consumed
        && replay_nomatch.field_hash_matched
        && replay_nomatch.world_digest == nomatch.world_digest;
    notes.push(format!(
        "mainstream replay: active {}/{} + nomatch {}/{}",
        replay_active.ticks_ok,
        replay_active.tick_count,
        replay_nomatch.ticks_ok,
        replay_nomatch.tick_count
    ));

    // —— 4/5) World-Field 唯一出口(F2)+ 渲染写拒绝 ——
    let manifest = env!("CARGO_MANIFEST_DIR");
    let (wf_ok, wf_digest, render_err_ok) = world_field_scene_probe();
    let f2_audit = world_field_f2_authorized_audit(manifest, &mut notes)?;
    let egress_f2 = wf_ok && f2_audit;

    // —— 6) 旁路提交注入检出(RED 臂的检测面)——
    let bypass_detected = {
        use rurix_render::geometry::gpu_scene::{GpuScene, WorldFieldSlot};
        // 旁路模拟:不经桥直接写 GpuScene(唯一出口纪律破坏)→ 桥↔场景
        // 一致性对拍必须检出。
        let mut scene_bypass = GpuScene::new();
        scene_bypass.commit_world_field(WorldFieldSlot {
            physics_tick: 0,
            render_frame: 0,
            payload: b"bypass".to_vec(),
        });
        let bridge_clean = rurix_physics::bridge::PhysicsBridge::new();
        let consistent = scene_bypass.world_field_slots().len()
            == bridge_clean.world_field_committed().len()
            && scene_bypass
                .world_field_slots()
                .iter()
                .zip(bridge_clean.world_field_committed())
                .all(|(s, b)| {
                    s.physics_tick == b.sample_set.physics_tick.0
                        && s.render_frame == b.sample_set.render_frame.0
                        && s.payload == b.payload
                });
        // 对照:经桥提交的一致性对拍为真(检测面不假红)。
        let (bridge_ok, _, _) = world_field_scene_probe();
        !consistent && bridge_ok
    };

    // —— 7) 锚定语料消费 ——
    let anchors_ok = crate::m121::conformance_anchors_consumed(manifest);

    // —— 8) measured 冻结带对拍 ——
    let freeze_values = format!(
        concat!(
            "{{\"world_digest_active\":\"{}\",",
            "\"journal_digest_active\":\"{}\",",
            "\"field_chain_digest_active\":\"{}\",",
            "\"world_field_sequence_digest\":\"{}\"}}"
        ),
        active.world_digest, active.journal_digest, active.field_chain_digest, wf_digest
    );
    if emit_freeze {
        return Ok(format!(
            "{{\"ok\":true,\"emit_freeze\":true,\"freeze\":{freeze_values}}}"
        ));
    }
    let freeze_path =
        freeze_path.ok_or_else(|| "--freeze required (or --emit-freeze)".to_string())?;
    let freeze_text = fs::read_to_string(&freeze_path).map_err(|e| e.to_string())?;
    let freeze_match = [
        "world_digest_active",
        "journal_digest_active",
        "field_chain_digest_active",
        "world_field_sequence_digest",
    ]
    .iter()
    .all(|k| {
        let want = extract_str(&freeze_text, k);
        let got = match *k {
            "world_digest_active" => Some(active.world_digest.clone()),
            "journal_digest_active" => Some(active.journal_digest.clone()),
            "field_chain_digest_active" => Some(active.field_chain_digest.clone()),
            "world_field_sequence_digest" => Some(wf_digest.clone()),
            _ => None,
        };
        want.is_some() && want == got
    });

    let ok = drives
        && zero_impact
        && mainstream_replay
        && egress_f2
        && render_err_ok
        && bypass_detected
        && anchors_ok
        && freeze_match;

    Ok(format!(
        "{{\"ok\":{},\"field_drives_dynamic_response_full\":{},\"filter_default_empty_zero_impact_full\":{},\"persistent_journal_mainstream_replay_equal\":{},\"world_field_egress_unique_authorized_f2\":{},\"render_write_injection_rejected\":{},\"bypass_submit_detected\":{},\"conformance_anchor_consumed\":{},\"measured_freeze_digest_match\":{},\"world_digest_active\":\"{}\",\"journal_digest_active\":\"{}\",\"field_chain_digest_active\":\"{}\",\"world_field_sequence_digest\":\"{}\",\"applied_impulse_count\":{},\"detail\":\"{}\"}}",
        json_bool(ok),
        json_bool(drives),
        json_bool(zero_impact),
        json_bool(mainstream_replay),
        json_bool(egress_f2),
        json_bool(render_err_ok),
        json_bool(bypass_detected),
        json_bool(anchors_ok),
        json_bool(freeze_match),
        active.world_digest,
        active.journal_digest,
        active.field_chain_digest,
        wf_digest,
        active.applied_impulse_count,
        json_escape(&notes.join("; ")),
    ))
}

/// 完整期自证红臂(门 --selftest 消费;两 RED 臂独立有效,臂失效 = 漏检)。
pub fn run_full_selftest_arm(args: &[String]) -> Result<String, String> {
    let arm = arg_value(args, "--arm").ok_or_else(|| "--arm required".to_string())?;
    match arm.as_str() {
        // 渲染侧写/回写注入:守卫必须 typed Err 拒绝(若放行 = 臂失效)。
        "render_write_injection" => {
            use rurix_render::geometry::gpu_scene::{
                GpuScene, WorldFieldSlot, WorldFieldWriteError,
            };
            let mut scene = GpuScene::new();
            let before = scene.world_field_slots().len();
            let rejected = scene.render_write_world_field(WorldFieldSlot {
                physics_tick: 1,
                render_frame: 2,
                payload: vec![0xEE],
            });
            let red = matches!(rejected, Err(WorldFieldWriteError::RenderWriteRejected))
                && scene.world_field_slots().len() == before;
            Ok(format!(
                "{{\"ok\":true,\"arm\":\"render_write_injection\",\"red_detected\":{}}}",
                json_bool(red)
            ))
        }
        // 旁路提交注入:不经桥直写 GpuScene → 一致性对拍必须检出。
        "bypass_submit" => {
            use rurix_render::geometry::gpu_scene::{GpuScene, WorldFieldSlot};
            let mut scene = GpuScene::new();
            scene.commit_world_field(WorldFieldSlot {
                physics_tick: 0,
                render_frame: 0,
                payload: b"bypass".to_vec(),
            });
            let bridge = rurix_physics::bridge::PhysicsBridge::new();
            let consistent = scene.world_field_slots().len()
                == bridge.world_field_committed().len();
            let red = !consistent;
            Ok(format!(
                "{{\"ok\":true,\"arm\":\"bypass_submit\",\"red_detected\":{}}}",
                json_bool(red)
            ))
        }
        other => Err(format!("unknown full selftest arm {other}")),
    }
}
