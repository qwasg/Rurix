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
