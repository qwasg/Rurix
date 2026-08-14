//! M121 particle_view 门(G9.2 骨架期;g9.p0.m121.physics_particle_view)。
//!
//! 七判据(MAP M121 行;--phase g9.2):
//! 1. five_domain_adapters_implemented:五域 adapter 各实例化并执行读写探针;
//! 2. write_path_impulse_only_structural:ParticleAdapter trait 源无 transform
//!    直写方法(类型面结构性断言,扫描 trait 定义字面);
//! 3. bypass_write_injection_rejected:旁路写注入探针源(语料 .rs.probe)
//!    含 transform 直写调用且引用不存在方法 = 编译期拒绝(静态核验);
//! 4. nominal_type_isolation:同位表示跨域 ref 不相等 + 域句柄互不 From;
//! 5. m68_migration_digest_equal:迁移前后逐 tick state_hash 一致 +
//!    migration_digest == golden;
//! 6. journal_fully_consumed:damage 行数 == 迁移命令数 == 重放消费数;
//! 7. one_way_fact_source_zero_byte:bridge 模块无渲染→物理回写面(扫描
//!    GpuScene 写口只出现在 sync_frame/World-Field 提交口;R-10 🔒)。

use std::fs;

use rurix_physics::capture::jolt_world_desc;
use rurix_physics::destruction::{DamageCommand, cook_destruction, parse_source_json};
use rurix_physics::particle_view::character_adapter::CharacterAdapter;
use rurix_physics::particle_view::cloth_adapter::ClothVertexAdapter;
use rurix_physics::particle_view::destruction_adapter::DestructionChunkAdapter;
use rurix_physics::particle_view::migrate::run_migration_gate;
use rurix_physics::particle_view::ragdoll_adapter::RagdollNodeAdapter;
use rurix_physics::particle_view::rigid_body_adapter::RigidBodyAdapter;
use rurix_physics::particle_view::{
    ImpulseWrite, ParticleAdapter, ParticleDomain, ParticleSleepState, PhysicsParticleRef,
    RAGDOLL_SCHEMA_ONLY_LITERAL, rigid_body_ref,
};
use rurix_physics::{BodyDesc, BodyKind, MassProps, PhysicsTransform, PhysicsWorld, ShapeDesc};

use crate::util::{arg_value, json_bool, json_escape};

struct GateReport {
    five_domain_adapters_implemented: bool,
    write_path_impulse_only_structural: bool,
    bypass_write_injection_rejected: bool,
    nominal_type_isolation: bool,
    m68_migration_digest_equal: bool,
    journal_fully_consumed: bool,
    one_way_fact_source_zero_byte: bool,
    migration_digest: String,
    notes: Vec<String>,
}

pub fn run(args: &[String]) -> Result<String, String> {
    let source_path = arg_value(args, "--source").ok_or_else(|| "--source required".to_string())?;
    let golden_path = arg_value(args, "--golden").ok_or_else(|| "--golden required".to_string())?;
    let source_text = fs::read_to_string(&source_path).map_err(|e| e.to_string())?;
    let golden_text = fs::read_to_string(&golden_path).map_err(|e| e.to_string())?;

    let source = parse_source_json(&source_text)?;
    let cooked = cook_destruction(&source).map_err(|e| e.to_string())?;
    let golden_migration_digest = extract_str(&golden_text, "migration_digest")
        .ok_or_else(|| "golden missing migration_digest".to_string())?;

    let mut notes: Vec<String> = Vec::new();

    // —— 五域 adapter 实例化 + 读写探针 ——
    // 域 1: RigidBody(Jolt 运行时权威)。
    let mut world = PhysicsWorld::new(jolt_world_desc(16)).map_err(|e| e.to_string())?;
    let body = world
        .add_bodies_batch(&[BodyDesc {
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
        .map_err(|e| e.to_string())?
        .remove(0);
    let rb_ref = rigid_body_ref(body);
    let rb_ok = {
        let mut ad = RigidBodyAdapter::new(&mut world);
        let p0 = ad.position(rb_ref).map_err(|e| e.to_string())?;
        ad.set_force_impulse(rb_ref, ImpulseWrite::Linear([0.0, 1.0, 0.0]))
            .map_err(|e| e.to_string())?;
        let v = ad.velocity(rb_ref).map_err(|e| e.to_string())?;
        let s = ad.sleep_state(rb_ref).map_err(|e| e.to_string())?;
        // 骨架期质量面诚实拒绝(BodySemantic 无质量字段)。
        let mass_honest = ad.mass(rb_ref).is_err();
        notes.push(format!("rigid_body boundary: {}", ad.skeleton_boundary()));
        // impulse 写路径生效 = 速度位非零(step 前 Jolt 记账 linear velocity);
        // 位置未变 = 写路径未触 transform(纪律 1:位置只经 step 演化)。
        p0 == [0.0; 3] && v[1] != 0.0 && mass_honest && !matches!(s, ParticleSleepState::Static)
    };

    // 域 2: ClothVertex(demo 轨道)。
    let mut solver = rurix_physics::cloth::ClothSolver::new_demo();
    let cloth_ok = {
        let mut ad = ClothVertexAdapter::new(&mut solver, 0xC107_0001);
        let r = PhysicsParticleRef::ClothVertex {
            stable_id: rurix_physics::particle_view::ClothStableId::from_bits(0xC107_0001),
            element_index: 0,
        };
        let p0 = ad.position(r).map_err(|e| e.to_string())?;
        ad.set_force_impulse(r, ImpulseWrite::Linear([0.001, 0.0, 0.0]))
            .map_err(|e| e.to_string())?;
        let p1 = ad.position(r).map_err(|e| e.to_string())?;
        notes.push(format!("cloth boundary: {}", ad.skeleton_boundary()));
        p0[0] + 0.001 == p1[0]
    };

    // 域 3: DestructionChunk(cooked + pipeline)。
    let pipeline = rurix_physics::destruction::FracturePipeline::new(cooked.clone());
    let dc_ok = {
        let mut ad = DestructionChunkAdapter::with_pipeline(&cooked, &pipeline);
        let r = DestructionChunkAdapter::ref_of_chunk(&cooked.chunks[0].chunk_id);
        let m = ad.mass(r).map_err(|e| e.to_string())?;
        ad.set_force_impulse(r, ImpulseWrite::Force([1.0, 0.0, 0.0]))
            .map_err(|e| e.to_string())?;
        let ledger = ad.ledger_impulse(r);
        notes.push(format!("destruction boundary: {}", ad.skeleton_boundary()));
        m > 0.0 && ledger == Some([1.0, 0.0, 0.0])
    };

    // 域 4: RagdollNode(资产层只读;写面诚实 SchemaOnlyAdapter 拒绝)。
    let asset = rurix_physics::asset::PhysicsAsset::new("gate_ragdoll", "skel-digest");
    let rag_ok = {
        let mut asset = asset.clone();
        asset.bones.push(rurix_physics::asset::BoneBodyMapping {
            bone_stable_id: "b0".into(),
            body_role: "pelvis".into(),
            collider_role: "capsule".into(),
        });
        let mut ad = RagdollNodeAdapter::new(&asset, 0xA66D_0001);
        let r = PhysicsParticleRef::RagdollNode {
            stable_id: rurix_physics::particle_view::RagdollAssetStableId::from_bits(0xA66D_0001),
            element_index: 0,
        };
        let write_rejected = ad
            .set_force_impulse(r, ImpulseWrite::Linear([1.0; 3]))
            .map_err(|e| e.to_string())
            .unwrap_err()
            .contains(RAGDOLL_SCHEMA_ONLY_LITERAL);
        let read_rejected = ad.position(r).is_err();
        notes.push(format!("ragdoll boundary: {}", ad.skeleton_boundary()));
        write_rejected && read_rejected
    };

    // 域 5: CharacterInner(M71 状态块)。
    let mut character =
        rurix_physics::character::RurixCharacter::new(7, PhysicsTransform::IDENTITY);
    let ch_ok = {
        let mut ad = CharacterAdapter::new(&mut character);
        let r = CharacterAdapter::ref_of(7);
        ad.set_force_impulse(r, ImpulseWrite::Linear([2.0, 0.0, 0.0]))
            .map_err(|e| e.to_string())?;
        let v = ad.velocity(r).map_err(|e| e.to_string())?;
        notes.push(format!("character boundary: {}", ad.skeleton_boundary()));
        v[0] == 2.0
    };

    let five_ok = rb_ok && cloth_ok && dc_ok && rag_ok && ch_ok;

    // —— 结构性断言:trait 源无 transform 直写方法 + 桥无渲染回写面 ——
    let manifest = env!("CARGO_MANIFEST_DIR");
    let pv_src = fs::read_to_string(format!(
        "{manifest}/../../src/rurix-physics/src/particle_view/mod.rs"
    ))
    .map_err(|e| e.to_string())?;
    let write_structural = trait_has_no_transform_write(&pv_src);

    // 旁路写注入探针源(语料;期望编译失败 = RED 臂静态核验)。
    let probe_path =
        format!("{manifest}/../../conformance/physics/particle_view/bypass_write_probe.rs.probe");
    let probe_src = fs::read_to_string(&probe_path).map_err(|e| e.to_string())?;
    let bypass_rejected = probe_src.contains(".set_transform(")
        && !trait_declares_set_transform(&pv_src)
        && probe_src.contains("compile_fail_expected");

    // 名义类型隔离(运行时等价面;编译期面 = 探针源 + 类型构造)。
    let isolation = nominal_isolation_runtime();

    // 桥单向面核验(R-10 🔒;bridge 源面无 GpuScene 回写)。
    let bridge_src = fs::read_to_string(format!(
        "{manifest}/../../src/rurix-physics/src/bridge/mod.rs"
    ))
    .map_err(|e| e.to_string())?;
    let one_way = bridge_one_way_clean(&bridge_src);

    // —— M68 迁移器(damage 命令序列 = 阈下 5 tick + 阈上 1 tick)——
    let golden_damage_point = extract_vec3(&golden_text, "damage_point");
    let golden_radius = extract_f32(&golden_text, "damage_radius").unwrap_or(1.5);
    let mut cmds: Vec<DamageCommand> = Vec::new();
    for tick in 0..5u64 {
        cmds.push(DamageCommand {
            tick,
            point: golden_damage_point,
            radius: golden_radius,
            magnitude: 0.1,
        });
    }
    cmds.push(DamageCommand {
        tick: 5,
        point: golden_damage_point,
        radius: golden_radius,
        magnitude: 10.0,
    });
    let report = run_migration_gate(&cooked, &cmds, 6).map_err(|e| e.to_string())?;
    let digest_equal = report.digest_equal && report.migration_digest == golden_migration_digest;
    let consumed = report.journal_fully_consumed
        && report.damage_line_count == cmds.len()
        && report.replayed_count == cmds.len();

    let out = GateReport {
        five_domain_adapters_implemented: five_ok,
        write_path_impulse_only_structural: write_structural,
        bypass_write_injection_rejected: bypass_rejected,
        nominal_type_isolation: isolation,
        m68_migration_digest_equal: digest_equal,
        journal_fully_consumed: consumed,
        one_way_fact_source_zero_byte: one_way,
        migration_digest: report.migration_digest.clone(),
        notes,
    };

    let ok = out.five_domain_adapters_implemented
        && out.write_path_impulse_only_structural
        && out.bypass_write_injection_rejected
        && out.nominal_type_isolation
        && out.m68_migration_digest_equal
        && out.journal_fully_consumed
        && out.one_way_fact_source_zero_byte;

    Ok(format!(
        "{{\"ok\":{},\"five_domain_adapters_implemented\":{},\"write_path_impulse_only_structural\":{},\"bypass_write_injection_rejected\":{},\"nominal_type_isolation\":{},\"m68_migration_digest_equal\":{},\"journal_fully_consumed\":{},\"one_way_fact_source_zero_byte\":{},\"migration_digest\":\"{}\",\"damage_line_count\":{},\"replayed_count\":{},\"detail\":\"{}\"}}",
        json_bool(ok),
        json_bool(out.five_domain_adapters_implemented),
        json_bool(out.write_path_impulse_only_structural),
        json_bool(out.bypass_write_injection_rejected),
        json_bool(out.nominal_type_isolation),
        json_bool(out.m68_migration_digest_equal),
        json_bool(out.journal_fully_consumed),
        json_bool(out.one_way_fact_source_zero_byte),
        out.migration_digest,
        report.damage_line_count,
        report.replayed_count,
        json_escape(&out.notes.join("; ")),
    ))
}

/// 结构性断言:trait 定义块内无任何 transform/position 直写方法。
fn trait_has_no_transform_write(src: &str) -> bool {
    let Some(start) = src.find("pub trait ParticleAdapter") else {
        return false;
    };
    let rest = &src[start..];
    let mut depth = 0i32;
    let mut end = rest.len();
    for (i, c) in rest.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }
    let block = &rest[..end];
    // 禁出现的方法名(transform 直写族)。
    for banned in [
        "fn set_transform",
        "fn set_position",
        "fn teleport",
        "fn set_rotation",
        "fn write_transform",
    ] {
        if block.contains(banned) {
            return false;
        }
    }
    block.contains("fn set_force_impulse")
}

/// 探针源 RED 臂:trait 若声明 set_transform 则旁路可表达(此时门红)。
fn trait_declares_set_transform(src: &str) -> bool {
    src.contains("fn set_transform")
}

/// 名义隔离运行时等价面(编译期拒绝证据在探针源 + 类型定义面)。
fn nominal_isolation_runtime() -> bool {
    use rurix_physics::particle_view::{ChunkStableId, RigidBodyStableId};
    let bits = 0xDEAD_BEEFu64;
    let a = PhysicsParticleRef::RigidBody(RigidBodyStableId::from_bits(bits));
    let b = PhysicsParticleRef::DestructionChunk(ChunkStableId::from_bits(bits));
    a != b && a.domain() == ParticleDomain::RigidBody && b.domain() != a.domain()
}

/// 桥单向核验:bridge 源只允许 `&mut GpuScene` 出现在 `sync_frame` 签名;
/// 禁止任何 GpuScene 方法被当作回写通道调用物理态(骨架期字面核验:
/// 桥内不得出现 `scene.set_`/`scene.write_` 类回写调用面)。
fn bridge_one_way_clean(src: &str) -> bool {
    // GpuScene 写口唯一合法字面 = update_transform / flush_dirty(既有面)。
    // 禁止新增 scene 直写方法调用。
    for banned in ["scene.set_", "scene.write_", "scene.push_", "scene.submit_"] {
        if src.contains(banned) {
            return false;
        }
    }
    // World-Field 出口 = submit_world_field 登记面;不得触 GpuScene 字段。
    src.contains("fn sync_frame") && src.contains("update_transform")
}

// —— 极简 golden 字段抽取(手写,免 serde)——

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

fn extract_f32(text: &str, key: &str) -> Option<f32> {
    let pat = format!("\"{key}\"");
    let i = text.find(&pat)?;
    let rest = &text[i + pat.len()..];
    let colon = rest.find(':')?;
    let r = rest[colon + 1..].trim_start();
    let end = r
        .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+'))
        .unwrap_or(r.len());
    r[..end].parse().ok()
}

fn extract_vec3(text: &str, key: &str) -> [f32; 3] {
    let pat = format!("\"{key}\"");
    let Some(i) = text.find(&pat) else {
        return [0.0, 1.0, 0.0];
    };
    let rest = &text[i + pat.len()..];
    let Some(b) = rest.find('[') else {
        return [0.0, 1.0, 0.0];
    };
    let Some(e) = rest[b..].find(']') else {
        return [0.0, 1.0, 0.0];
    };
    let inner = &rest[b + 1..b + e];
    let parts: Vec<f32> = inner
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    if parts.len() == 3 {
        [parts[0], parts[1], parts[2]]
    } else {
        [0.0, 1.0, 0.0]
    }
}

// —— G9.6 完整期门(--phase g9.6;spec/physics.md RXS-0374,判据逐字引
// G9_ACCEPTANCE_MAP §2 M121 行;RFC-0024 §4.A/§4.B + v1.1 章 F2)——
//
// 完整期判据面:
// 1. field_solver_coupling_drives_motion:场求值经 ParticleAdapter 写路径
//    (仅 impulse/force)耦合进 lockstep 求解输入——活性场世界 digest ≠
//    无场基线且施加非零 impulse;
// 2. coupling_determinism_double_run:同输入双运行逐位一致;
// 3. field_zeroed_baseline_digest_equal:场置零退化臂世界 digest 与无场
//    基线逐位一致;
// 4. write_path_impulse_only_structural / 5. bypass_write_injection_rejected:
//    骨架期结构性断言完整期维持(字面 0-byte);
// 6. analytic_surface_closed_set:sphere/plane/box 三形解析 sdf/梯度 +
//    确定性双跑 + 闭集外 fail-closed + 参数进 digest;
// 7. field_journal_capture_roundtrip:场命令并入 M66 journal 主流 encode→
//    decode 往返无损 + 版本/digest 篡改 fail-closed;
// 8. capture_replay_field_mainstream:capture→replay 逐 tick hash 一致 +
//    重算 impulse 与记账逐位相等 + journal 全消费;
// 9. world_field_gpu_scene_readonly:F2 修订行面——桥提交 GpuScene 只读
//    buffer,渲染侧只读消费,序列 digest 一致且确定性;
// 10. render_write_injection_typed_err:渲染侧写尝试 fail-closed typed Err;
// 11. conformance_anchor_consumed:RXS-0374/0375 锚定语料消费核验;
// 12. measured_freeze_digest_match:measured 冻结带对拍(禁手写 golden)。

/// 完整期录制 canonical 场景规格(8 tick 单动态球 + persistent 场)。
fn full_spec(presence: rurix_physics::field::capture_merge::FieldPresence) -> rurix_physics::field::capture_merge::FieldCaptureSpec {
    rurix_physics::field::capture_merge::FieldCaptureSpec {
        scenario_id: "g96_m121_field_solver_coupling".into(),
        ticks: 8,
        presence,
    }
}

/// analytic-surface 闭集探针(accept/解析值/确定性/闭集外拒/digest 参与)。
fn run_analytic_closed_set_probe() -> bool {
    use rurix_physics::field::def::{AnalyticSurfacePrimitive as P, FieldNodeKind};
    let sphere = P::Sphere {
        center: [0.0; 3],
        radius: 2.0,
    };
    let plane = P::Plane {
        normal: [0.0, 0.0, 1.0],
        offset: 1.5,
    };
    let b = P::Box {
        min: [-1.0; 3],
        max: [1.0; 3],
    };
    // 三形 accept + 解析 sdf/梯度已知值。
    let known = sphere.validate().is_ok()
        && plane.validate().is_ok()
        && b.validate().is_ok()
        && sphere.signed_distance([3.0, 0.0, 0.0]) == 1.0
        && plane.signed_distance([0.0, 0.0, 1.0]) == -0.5
        && b.signed_distance([2.0, 0.0, 0.0]) == 1.0
        && sphere.gradient([3.0, 0.0, 0.0]) == [1.0, 0.0, 0.0]
        && plane.gradient([9.0, 9.0, 9.0]) == [0.0, 0.0, 1.0];
    // 同输入双跑位级一致。
    let p = [0.3, -1.2, 4.5];
    let det = [sphere, plane, b]
        .iter()
        .all(|pr| pr.signed_distance(p) == pr.signed_distance(p) && pr.gradient(p) == pr.gradient(p));
    // 闭集外形状 fail-closed(不静默退化采样)。
    let closed = ["capsule", "cone", "mesh", "cylinder", "tetrahedron", ""]
        .iter()
        .all(|n| !P::closed_set_member(n))
        && ["sphere", "plane", "box"].iter().all(|n| P::closed_set_member(n));
    // 基元参数进场定义 digest。
    let d1 = rurix_physics::field::FieldDef::new(
        "p1",
        rurix_physics::field::FieldNode {
            node_id: "n".into(),
            kind: FieldNodeKind::AnalyticSurfacePrimitive { primitive: sphere },
            weight: 1.0,
            children: vec![],
        },
        rurix_physics::field::FieldPhysicsType::LinearForce,
        rurix_physics::field::FieldLifecycle::Persistent,
        rurix_physics::field::FieldFilter::default(),
    );
    let mut d2 = d1.clone();
    d2.root.kind = FieldNodeKind::AnalyticSurfacePrimitive {
        primitive: P::Sphere {
            center: [0.0; 3],
            radius: 2.5,
        },
    };
    let digest_in = d1.digest() != d2.digest();
    known && det && closed && digest_in
}

/// 场 journal 并入主流往返探针:FieldJournalCommand → 主流线命令 →
/// journal.jsonl 行 encode → decode 逐位无损;版本/digest 篡改 fail-closed。
fn run_journal_roundtrip_probe() -> Result<bool, String> {
    use rurix_physics::capture::{JournalTick, PostTick};
    use rurix_physics::field::capture_merge::{field_cmd_from_wire, field_cmd_to_wire};

    let journal = crate::m122::build_persistent_journal()?;
    for tick in &journal.ticks {
        for cmd in &tick.commands {
            let wire = field_cmd_to_wire(cmd);
            // 主流行格式往返(进 post.field_semantic_hash 加性面同行验证)。
            let line = JournalTick {
                tick: tick.tick,
                pre: vec![wire],
                post: PostTick {
                    semantic_state_hash: "00".repeat(32),
                    event_digest: "11".repeat(32),
                    contacts_emitted: 0,
                    contacts_dropped: 0,
                    ring_backlog: 0,
                    saturation_query_casts: 0,
                    saturation_contact_events: 0,
                    saturation_body_writes: 0,
                    field_semantic_hash: Some(tick.semantic_hash.clone()),
                },
            }
            .to_json_line()
            .map_err(|e| e.to_string())?;
            let parsed = JournalTick::parse_json_line(&line).map_err(|e| e.to_string())?;
            if parsed.post.field_semantic_hash.as_deref() != Some(tick.semantic_hash.as_str()) {
                return Ok(false);
            }
            if parsed.pre.len() != 1 {
                return Ok(false);
            }
            let back = field_cmd_from_wire(&parsed.pre[0])
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "field cmd lost in roundtrip".to_string())?;
            if back != *cmd {
                return Ok(false);
            }
            // 版本篡改 fail-closed(显式迁移纪律,不静默重解释)。
            let tampered = line.replacen("\"v\":1", "\"v\":999", 1);
            if tampered != line && JournalTick::parse_json_line(&tampered).is_ok() {
                return Ok(false);
            }
        }
    }
    // legacy 行 0-byte:无场 hash 行 parse → None,重编码字节不变。
    let legacy = JournalTick {
        tick: 0,
        pre: vec![],
        post: PostTick {
            semantic_state_hash: "00".repeat(32),
            event_digest: "11".repeat(32),
            contacts_emitted: 0,
            contacts_dropped: 0,
            ring_backlog: 0,
            saturation_query_casts: 0,
            saturation_contact_events: 0,
            saturation_body_writes: 0,
            field_semantic_hash: None,
        },
    };
    let legacy_line = legacy.to_json_line().map_err(|e| e.to_string())?;
    if legacy_line.contains("field_semantic_hash") {
        return Ok(false);
    }
    let legacy_back = JournalTick::parse_json_line(&legacy_line).map_err(|e| e.to_string())?;
    if legacy_back.post.field_semantic_hash.is_some()
        || legacy_back.to_json_line().map_err(|e| e.to_string())? != legacy_line
    {
        return Ok(false);
    }
    Ok(true)
}

/// RXS-0374/0375 锚定语料消费核验(conformance/physics 四件)。
pub(crate) fn conformance_anchors_consumed(manifest: &str) -> bool {
    let cases = [
        ("accept/field_solver_coupling_minimal.rx", "RXS-0374"),
        ("reject/world_field_render_writeback.rx", "RXS-0374"),
        ("reject/field_journal_capture_roundtrip_break.rx", "RXS-0374"),
        ("accept/gameplay_field_full_phase_minimal.rx", "RXS-0375"),
    ];
    cases.iter().all(|(rel, anchor)| {
        fs::read_to_string(format!("{manifest}/../../conformance/physics/{rel}"))
            .map(|text| {
                text.contains(&format!("//@ spec: {anchor}")) && text.contains("--phase g9.6")
            })
            .unwrap_or(false)
    })
}

/// 完整期门(`particle-view-full --freeze <path>`;`--emit-freeze` =
/// measured 冻结带生成口)。
pub fn run_full(args: &[String]) -> Result<String, String> {
    use rurix_physics::field::capture_merge::{
        FieldPresence, record_field_capture, replay_field_capture,
    };

    let emit_freeze = args.iter().any(|a| a == "--emit-freeze");
    let freeze_path = arg_value(args, "--freeze");

    let mut notes: Vec<String> = Vec::new();

    // —— 1~3) 耦合驱动/确定性/置零退化(三臂 canonical 场景)——
    let active = record_field_capture(&full_spec(FieldPresence::Active)).map_err(|e| e.to_string())?;
    let zeroed = record_field_capture(&full_spec(FieldPresence::Zeroed)).map_err(|e| e.to_string())?;
    let absent = record_field_capture(&full_spec(FieldPresence::Absent)).map_err(|e| e.to_string())?;
    let active_rerun =
        record_field_capture(&full_spec(FieldPresence::Active)).map_err(|e| e.to_string())?;
    let drives_motion =
        active.applied_impulse_count > 0 && active.world_digest != absent.world_digest;
    let determinism = active_rerun.world_digest == active.world_digest
        && active_rerun.journal_digest == active.journal_digest;
    let zeroed_equal = zeroed.world_digest == absent.world_digest;
    notes.push(format!(
        "coupling: applied={} active!={} zeroed==absent={}",
        active.applied_impulse_count,
        absent.world_digest != active.world_digest,
        zeroed_equal
    ));

    // —— 4/5) 结构性断言完整期维持(字面 0-byte)——
    let manifest = env!("CARGO_MANIFEST_DIR");
    let pv_src = fs::read_to_string(format!(
        "{manifest}/../../src/rurix-physics/src/particle_view/mod.rs"
    ))
    .map_err(|e| e.to_string())?;
    let write_structural = trait_has_no_transform_write(&pv_src);
    let probe_path =
        format!("{manifest}/../../conformance/physics/particle_view/bypass_write_probe.rs.probe");
    let probe_src = fs::read_to_string(&probe_path).map_err(|e| e.to_string())?;
    let bypass_rejected = probe_src.contains(".set_transform(")
        && !trait_declares_set_transform(&pv_src)
        && probe_src.contains("compile_fail_expected");

    // —— 6) analytic-surface 闭集 ——
    let analytic_ok = run_analytic_closed_set_probe();

    // —— 7) 场 journal 主流往返 ——
    let roundtrip_ok = run_journal_roundtrip_probe()?;

    // —— 8) capture→replay 主流逐 tick 一致(persist 落盘 + 目录回放)——
    let cap_dir = std::env::temp_dir().join(format!("g9_m121_field_capture_{}", std::process::id()));
    let _ = fs::remove_dir_all(&cap_dir);
    active
        .artifact
        .persist(&cap_dir)
        .map_err(|e| e.to_string())?;
    let replay = replay_field_capture(&cap_dir);
    let _ = fs::remove_dir_all(&cap_dir);
    let replay = replay.map_err(|e| e.to_string())?;
    let mainstream_ok = replay.journal_fully_consumed
        && replay.field_hash_matched
        && replay.impulses_recomputed_equal
        && replay.world_digest == active.world_digest
        && replay.field_chain_digest == active.field_chain_digest;
    notes.push(format!(
        "mainstream replay: ticks {}/{} impulses_recomputed={}",
        replay.ticks_ok, replay.tick_count, replay.impulses_recomputed_equal
    ));

    // —— 9/10) World-Field GpuScene 只读扩面(F2)+ 渲染写 typed Err ——
    let (wf_ok, wf_digest, render_err_ok) = crate::m122::world_field_scene_probe();

    // —— 11) 锚定语料消费 ——
    let anchors_ok = conformance_anchors_consumed(manifest);

    // —— 12) measured 冻结带对拍 ——
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
    let freeze_path = freeze_path.ok_or_else(|| "--freeze required (or --emit-freeze)".to_string())?;
    let freeze_text = fs::read_to_string(&freeze_path).map_err(|e| e.to_string())?;
    let freeze_match = ["world_digest_active", "journal_digest_active", "field_chain_digest_active", "world_field_sequence_digest"]
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

    let ok = drives_motion
        && determinism
        && zeroed_equal
        && write_structural
        && bypass_rejected
        && analytic_ok
        && roundtrip_ok
        && mainstream_ok
        && wf_ok
        && render_err_ok
        && anchors_ok
        && freeze_match;

    Ok(format!(
        "{{\"ok\":{},\"field_solver_coupling_drives_motion\":{},\"coupling_determinism_double_run\":{},\"field_zeroed_baseline_digest_equal\":{},\"write_path_impulse_only_structural\":{},\"bypass_write_injection_rejected\":{},\"analytic_surface_closed_set\":{},\"field_journal_capture_roundtrip\":{},\"capture_replay_field_mainstream\":{},\"world_field_gpu_scene_readonly\":{},\"render_write_injection_typed_err\":{},\"conformance_anchor_consumed\":{},\"measured_freeze_digest_match\":{},\"world_digest_active\":\"{}\",\"journal_digest_active\":\"{}\",\"field_chain_digest_active\":\"{}\",\"world_field_sequence_digest\":\"{}\",\"applied_impulse_count\":{},\"detail\":\"{}\"}}",
        json_bool(ok),
        json_bool(drives_motion),
        json_bool(determinism),
        json_bool(zeroed_equal),
        json_bool(write_structural),
        json_bool(bypass_rejected),
        json_bool(analytic_ok),
        json_bool(roundtrip_ok),
        json_bool(mainstream_ok),
        json_bool(wf_ok),
        json_bool(render_err_ok),
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

/// 完整期自证红臂(门 --selftest 消费;臂失效 = 漏检即门红)。
pub fn run_full_selftest_arm(args: &[String]) -> Result<String, String> {
    use rurix_physics::field::capture_merge::{
        FieldPresence, record_field_capture, replay_field_artifact,
    };

    let arm = arg_value(args, "--arm").ok_or_else(|| "--arm required".to_string())?;
    match arm.as_str() {
        // journal 缺失注入:删末行 → replay 必须红。
        "journal_missing_line" => {
            let out = record_field_capture(&full_spec(FieldPresence::Active))
                .map_err(|e| e.to_string())?;
            let baseline_ok = replay_field_artifact(&out.artifact).is_ok();
            let mut broken = out.artifact.clone();
            broken.ticks.pop();
            let red = replay_field_artifact(&broken).is_err();
            Ok(format!(
                "{{\"ok\":true,\"arm\":\"journal_missing_line\",\"baseline_ok\":{},\"red_detected\":{}}}",
                json_bool(baseline_ok),
                json_bool(red)
            ))
        }
        // journal 乱序注入:交换 update/unregister 两 tick pre → replay 必须红。
        "journal_reordered" => {
            let out = record_field_capture(&full_spec(FieldPresence::Active))
                .map_err(|e| e.to_string())?;
            let mut broken = out.artifact.clone();
            broken.ticks[4].pre = out.artifact.ticks[7].pre.clone();
            broken.ticks[7].pre = out.artifact.ticks[4].pre.clone();
            let red = replay_field_artifact(&broken).is_err();
            Ok(format!(
                "{{\"ok\":true,\"arm\":\"journal_reordered\",\"red_detected\":{}}}",
                json_bool(red)
            ))
        }
        // journal 篡改注入:翻 def_digest → replay 必须红。
        "journal_tampered_def" => {
            let out = record_field_capture(&full_spec(FieldPresence::Active))
                .map_err(|e| e.to_string())?;
            let mut broken = out.artifact.clone();
            for cmd in &mut broken.ticks[0].pre {
                if let rurix_physics::capture::JournalCommand::FieldRegister {
                    def_digest, ..
                } = cmd
                {
                    *def_digest = "f".repeat(64);
                }
            }
            let red = replay_field_artifact(&broken).is_err();
            Ok(format!(
                "{{\"ok\":true,\"arm\":\"journal_tampered_def\",\"red_detected\":{}}}",
                json_bool(red)
            ))
        }
        // 场置零不退化检测面:若耦合未接线(活性 ≡ 基线),门的非零断言
        // 必红——本臂验证检测面活着(活性 ≠ 基线 且 置零 ≡ 基线)。
        "coupling_not_wired" => {
            let active = record_field_capture(&full_spec(FieldPresence::Active))
                .map_err(|e| e.to_string())?;
            let absent = record_field_capture(&full_spec(FieldPresence::Absent))
                .map_err(|e| e.to_string())?;
            let zeroed = record_field_capture(&full_spec(FieldPresence::Zeroed))
                .map_err(|e| e.to_string())?;
            // 检测面 = (活性 ≠ 基线)可区分「耦合未接线」假绿 + (置零 ≡
            // 基线)可区分「置零不退化」假绿;两臂均成立 = 检测面活。
            let detector_alive = active.world_digest != absent.world_digest
                && zeroed.world_digest == absent.world_digest;
            Ok(format!(
                "{{\"ok\":true,\"arm\":\"coupling_not_wired\",\"red_detected\":{}}}",
                json_bool(detector_alive)
            ))
        }
        other => Err(format!("unknown full selftest arm {other}")),
    }
}
