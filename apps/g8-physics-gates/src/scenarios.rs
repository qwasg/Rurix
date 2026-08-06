//! M66 corpus 场景(10×300+ tick,job_threads=1)。

use std::collections::HashMap;

use rurix_physics::bridge::{PageKey, StreamingBridge};
use rurix_physics::capture::{
    apply_journal_pre, body_ids_bits, CaptureArtifact, CaptureError, CaptureRecorder,
    JournalCommand,
};
use rurix_physics::{
    BodyDesc, BodyId, BodyKind, MassProps, PhysicsTransform, PhysicsWorld, QueryRay, ShapeDesc,
    SyncBudget, WorldDesc,
};

use crate::util::{build_fingerprint, joltc_abi_digest, scenario_budget};

const DT: f32 = 1.0 / 60.0;
const IDENTITY: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

#[derive(Debug, Clone)]
pub struct InjectionSpec {
    pub tick: u64,
    pub body: BodyId,
    pub field: &'static str,
    pub bit: u8,
}

pub fn run_scenario(scenario_id: &str) -> Result<(CaptureArtifact, Option<InjectionSpec>), CaptureError> {
    match scenario_id {
        "box_stack_settle" => box_stack_settle(),
        "sphere_impulse_script" => sphere_impulse_script(),
        "create_destroy_churn" => create_destroy_churn(),
        "streaming_page_cycle" => streaming_page_cycle(),
        "ccd_bullet_thin_wall" => ccd_bullet_thin_wall(),
        "kinematic_platform" => kinematic_platform(),
        "joint_pendulum_motor" => joint_pendulum_motor(),
        "query_mid_replay" => query_mid_replay(),
        "contact_ring_saturation" => contact_ring_saturation(),
        "mixed_soup_72" => mixed_soup_72(),
        other => Err(CaptureError::Rejected(format!("unknown scenario {other}"))),
    }
}

struct Sim {
    world: PhysicsWorld,
    recorder: CaptureRecorder,
    streaming: StreamingBridge,
    constraints: HashMap<u64, u64>,
}

impl Sim {
    fn new(id: &str, desc: WorldDesc, ticks: u64) -> Result<Self, CaptureError> {
        Ok(Sim {
            world: PhysicsWorld::new(desc.clone())
                .map_err(|e| CaptureError::Backend(e.to_string()))?,
            recorder: CaptureRecorder::begin(
                id,
                ticks,
                &desc,
                &build_fingerprint(),
                &joltc_abi_digest(),
                scenario_budget(id),
            ),
            streaming: StreamingBridge::new(),
            constraints: HashMap::new(),
        })
    }


    fn add_batch(&mut self, descs: &[BodyDesc]) -> Result<Vec<BodyId>, CaptureError> {
        self.world
            .add_bodies_batch(descs)
            .map_err(|e| CaptureError::Backend(e.to_string()))
    }

    fn advance(&mut self, tick: u64, pre: Vec<JournalCommand>) -> Result<(), CaptureError> {
        let stats = self
            .world
            .step(DT)
            .map_err(|e| CaptureError::Backend(e.to_string()))?;
        let emitted = stats.contacts_emitted;
        let dropped = u64::from(stats.contacts_dropped);
        let Sim {
            world, recorder, ..
        } = self;
        recorder.seal_tick(world, tick, pre, emitted, dropped)?;
        Ok(())
    }

    /// 执行 journal 命令后 step+seal(标准 tick)。
    fn step_with(&mut self, tick: u64, pre: Vec<JournalCommand>) -> Result<(), CaptureError> {
        let budget = self.recorder.header().budget_profile.clone();
        let Sim {
            world,
            streaming,
            constraints,
            ..
        } = self;
        apply_journal_pre(world, streaming, constraints, &budget, &pre)?;
        self.advance(tick, pre)
    }

    /// 命令已在活世界执行,仅 step+seal(录制 tick0 批插等)。
    fn step_already_applied(&mut self, tick: u64, pre: Vec<JournalCommand>) -> Result<(), CaptureError> {
        self.advance(tick, pre)
    }

    fn done(self) -> Result<CaptureArtifact, CaptureError> {
        self.recorder.finish(&self.world)
    }
}

fn ground() -> BodyDesc {
    BodyDesc {
        kind: BodyKind::Static,
        shape: ShapeDesc::Box {
            half_extents: [20.0, 0.5, 20.0],
        },
        layer: 0,
        mass_props: MassProps::default(),
        ccd: false,
        transform: PhysicsTransform {
            translation: [0.0, -0.5, 0.0],
            rotation: IDENTITY,
        },
    }
}

fn dyn_box(x: f32, y: f32, z: f32, half: f32) -> BodyDesc {
    BodyDesc {
        kind: BodyKind::Dynamic,
        shape: ShapeDesc::Box {
            half_extents: [half, half, half],
        },
        layer: 1,
        mass_props: MassProps::default(),
        ccd: false,
        transform: PhysicsTransform {
            translation: [x, y, z],
            rotation: IDENTITY,
        },
    }
}

fn dyn_sphere(x: f32, y: f32, z: f32, r: f32) -> BodyDesc {
    BodyDesc {
        kind: BodyKind::Dynamic,
        shape: ShapeDesc::Sphere { radius: r },
        layer: 1,
        mass_props: MassProps::default(),
        ccd: false,
        transform: PhysicsTransform {
            translation: [x, y, z],
            rotation: IDENTITY,
        },
    }
}

fn box_stack_settle() -> Result<(CaptureArtifact, Option<InjectionSpec>), CaptureError> {
    let desc = crate::util::scenario_world_desc("box_stack_settle");
    let mut sim = Sim::new("box_stack_settle", desc, 300)?;
    let mut descs = vec![ground()];
    for i in 0..8 {
        descs.push(dyn_box(0.0, 0.6 + i as f32 * 1.05, 0.0, 0.5));
    }
    let ids = sim.add_batch(&descs)?;
    sim.step_already_applied(
        0,
        vec![JournalCommand::CreateBodies {
            descs,
            assigned_ids: body_ids_bits(&ids),
        }],
    )?;
    for t in 1..300 {
        sim.step_with(t, vec![])?;
    }
    Ok((sim.done()?, None))
}

fn sphere_impulse_script() -> Result<(CaptureArtifact, Option<InjectionSpec>), CaptureError> {
    let desc = crate::util::scenario_world_desc("sphere_impulse_script");
    let mut sim = Sim::new("sphere_impulse_script", desc, 300)?;
    let descs = vec![ground(), dyn_sphere(0.0, 2.0, 0.0, 0.5)];
    let ids = sim.add_batch(&descs)?;
    let sphere = ids[1];
    sim.step_already_applied(
        0,
        vec![JournalCommand::CreateBodies {
            descs,
            assigned_ids: body_ids_bits(&ids),
        }],
    )?;
    for t in 1..300 {
        let mut pre = Vec::new();
        if t % 40 == 0 {
            pre.push(JournalCommand::ApplyImpulse {
                body: sphere.to_bits(),
                impulse: [0.0, 3.0, 0.0],
            });
        }
        sim.step_with(t, pre)?;
    }
    Ok((
        sim.done()?,
        Some(InjectionSpec {
            tick: 150,
            body: sphere,
            field: "linvel.y",
            bit: 5,
        }),
    ))
}

fn create_destroy_churn() -> Result<(CaptureArtifact, Option<InjectionSpec>), CaptureError> {
    let desc = crate::util::scenario_world_desc("create_destroy_churn");
    let mut sim = Sim::new("create_destroy_churn", desc, 300)?;
    let descs = vec![ground()];
    let ids = sim.add_batch(&descs)?;
    sim.step_already_applied(
        0,
        vec![JournalCommand::CreateBodies {
            descs,
            assigned_ids: body_ids_bits(&ids),
        }],
    )?;
    let mut spawned: Vec<BodyId> = Vec::new();
    for t in 1..300 {
        if t % 20 == 0 {
            let d = vec![dyn_box((t % 7) as f32 * 0.3, 2.5, 0.0, 0.3)];
            let new_ids = sim.add_batch(&d)?;
            spawned.push(new_ids[0]);
            let pre = vec![JournalCommand::CreateBodies {
                descs: d,
                assigned_ids: body_ids_bits(&new_ids),
            }];
            sim.step_already_applied(t, pre)?;
            continue;
        }
        if t % 20 == 10 && !spawned.is_empty() {
            let victim = spawned.remove(0);
            sim.step_with(
                t,
                vec![JournalCommand::RemoveBodies {
                    ids: vec![victim.to_bits()],
                }],
            )?;
            continue;
        }
        sim.step_with(t, vec![])?;
    }
    Ok((sim.done()?, None))
}

fn streaming_page_cycle() -> Result<(CaptureArtifact, Option<InjectionSpec>), CaptureError> {
    let desc = crate::util::scenario_world_desc("streaming_page_cycle");
    let mut sim = Sim::new("streaming_page_cycle", desc, 300)?;
    let descs = vec![ground()];
    let ids = sim.add_batch(&descs)?;
    sim.step_already_applied(
        0,
        vec![JournalCommand::CreateBodies {
            descs,
            assigned_ids: body_ids_bits(&ids),
        }],
    )?;
    let page = PageKey {
        resource: 1,
        page: 0,
    };
    for t in 1..300 {
        if t == 50 {
            let pd = vec![dyn_sphere(5.0, 3.0, 0.0, 0.4)];
            let pids = sim
                .streaming
                .insert_page(&mut sim.world, page, &pd)
                .map_err(|e| CaptureError::Backend(e.to_string()))?;
            sim.step_already_applied(
                t,
                vec![JournalCommand::PageResident {
                    page_resource: page.resource,
                    page: page.page,
                    descs: pd,
                    assigned_ids: body_ids_bits(&pids),
                }],
            )?;
            continue;
        }
        if t == 120 {
            let receipt = sim
                .streaming
                .remove_page(&mut sim.world, page)
                .map_err(|e| CaptureError::Backend(e.to_string()))?;
            sim.step_already_applied(
                t,
                vec![JournalCommand::PageUnload {
                    page_resource: page.resource,
                    page: page.page,
                    receipt_bodies: body_ids_bits(receipt.removed_bodies()),
                }],
            )?;
            continue;
        }
        sim.step_with(t, vec![])?;
    }
    Ok((sim.done()?, None))
}

fn ccd_bullet_thin_wall() -> Result<(CaptureArtifact, Option<InjectionSpec>), CaptureError> {
    let desc = crate::util::scenario_world_desc("ccd_bullet_thin_wall");
    let mut sim = Sim::new("ccd_bullet_thin_wall", desc, 300)?;
    let wall = BodyDesc {
        kind: BodyKind::Static,
        shape: ShapeDesc::Box {
            half_extents: [2.0, 0.05, 2.0],
        },
        layer: 0,
        mass_props: MassProps::default(),
        ccd: false,
        transform: PhysicsTransform {
            translation: [0.0, 1.0, 0.0],
            rotation: IDENTITY,
        },
    };
    let mut bullet = dyn_sphere(-3.0, 1.0, 0.0, 0.15);
    bullet.ccd = true;
    bullet.mass_props.mass = 0.2;
    let descs = vec![ground(), wall, bullet];
    let ids = sim.add_batch(&descs)?;
    sim.step_already_applied(
        0,
        vec![JournalCommand::CreateBodies {
            descs,
            assigned_ids: body_ids_bits(&ids),
        }],
    )?;
    sim.step_with(
        1,
        vec![JournalCommand::SetVelocity {
            body: ids[2].to_bits(),
            linear: [40.0, 0.0, 0.0],
            angular: [0.0; 3],
        }],
    )?;
    for t in 2..300 {
        sim.step_with(t, vec![])?;
    }
    Ok((sim.done()?, None))
}

fn kinematic_platform() -> Result<(CaptureArtifact, Option<InjectionSpec>), CaptureError> {
    let desc = crate::util::scenario_world_desc("kinematic_platform");
    let mut sim = Sim::new("kinematic_platform", desc, 300)?;
    let plat = BodyDesc {
        kind: BodyKind::Kinematic,
        shape: ShapeDesc::Box {
            half_extents: [2.0, 0.2, 1.0],
        },
        layer: 0,
        mass_props: MassProps::default(),
        ccd: false,
        transform: PhysicsTransform {
            translation: [0.0, 1.0, 0.0],
            rotation: IDENTITY,
        },
    };
    let descs = vec![ground(), plat, dyn_box(0.0, 2.5, 0.0, 0.4)];
    let ids = sim.add_batch(&descs)?;
    sim.step_already_applied(
        0,
        vec![JournalCommand::CreateBodies {
            descs,
            assigned_ids: body_ids_bits(&ids),
        }],
    )?;
    for t in 1..300 {
        let y = 1.0 + (t as f32 * 0.01).sin() * 0.5;
        sim.step_with(
            t,
            vec![JournalCommand::MoveKinematic {
                body: ids[1].to_bits(),
                transform: PhysicsTransform {
                    translation: [0.0, y, 0.0],
                    rotation: IDENTITY,
                },
            }],
        )?;
    }
    Ok((sim.done()?, None))
}

fn joint_pendulum_motor() -> Result<(CaptureArtifact, Option<InjectionSpec>), CaptureError> {
    let desc = crate::util::scenario_world_desc("joint_pendulum_motor");
    let mut sim = Sim::new("joint_pendulum_motor", desc, 300)?;
    let anchor = BodyDesc {
        kind: BodyKind::Static,
        shape: ShapeDesc::Box {
            half_extents: [0.3, 0.3, 0.3],
        },
        layer: 0,
        mass_props: MassProps::default(),
        ccd: false,
        transform: PhysicsTransform {
            translation: [0.0, 4.0, 0.0],
            rotation: IDENTITY,
        },
    };
    let bar = dyn_box(0.0, 2.0, 0.0, 0.25);
    let descs = vec![ground(), anchor, bar];
    let ids = sim.add_batch(&descs)?;
    let cid = sim
        .world
        .add_hinge_constraint(ids[1], ids[2], [0.0, 4.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0])
        .map_err(|e| CaptureError::Backend(e.to_string()))?;
    sim.constraints.insert(1, cid);
    sim.step_already_applied(
        0,
        vec![
            JournalCommand::CreateBodies {
                descs,
                assigned_ids: body_ids_bits(&ids),
            },
            JournalCommand::AddConstraint {
                ctype: 1,
                body_a: ids[1].to_bits(),
                body_b: ids[2].to_bits(),
                point: [0.0, 4.0, 0.0],
                hinge_axis: [0.0, 1.0, 0.0],
                normal_axis: [1.0, 0.0, 0.0],
                assigned_id: 1,
            },
        ],
    )?;
    for t in 1..300 {
        let mut pre = Vec::new();
        if t == 100 {
            pre.push(JournalCommand::SetMotor {
                id: 1,
                state: 1, // velocity
                target: 0.5,
            });
        }
        if t == 200 {
            pre.push(JournalCommand::SetMotor {
                id: 1,
                state: 0,
                target: 0.0,
            });
        }
        sim.step_with(t, pre)?;
    }
    Ok((sim.done()?, None))
}

fn query_mid_replay() -> Result<(CaptureArtifact, Option<InjectionSpec>), CaptureError> {
    let desc = crate::util::scenario_world_desc("query_mid_replay");
    let mut sim = Sim::new("query_mid_replay", desc, 300)?;
    let descs = vec![ground(), dyn_sphere(0.0, 1.0, 0.0, 0.5)];
    let ids = sim.add_batch(&descs)?;
    sim.step_already_applied(
        0,
        vec![JournalCommand::CreateBodies {
            descs,
            assigned_ids: body_ids_bits(&ids),
        }],
    )?;
    for t in 1..300 {
        let mut pre = Vec::new();
        if t == 75 {
            let mut budget = SyncBudget::new(65536, 4096, 4096);
            let hits = sim
                .world
                .cast_ray(
                    &QueryRay {
                        origin: [0.0, 10.0, 0.0],
                        dir: [0.0, -1.0, 0.0],
                        t_min: 0.0,
                        t_max: 100.0,
                        layer_mask: u64::MAX,
                    },
                    &mut budget,
                )
                .map_err(|e| CaptureError::Backend(e.to_string()))?;
            let expected: Vec<(u64, u32)> = hits
                .iter()
                .map(|h| (h.body.to_bits(), h.t.to_bits()))
                .collect();
            pre.push(JournalCommand::QueryRay {
                origin: [0.0, 10.0, 0.0],
                dir: [0.0, -1.0, 0.0],
                t_min: 0.0,
                t_max: 100.0,
                layer_mask: u64::MAX,
                expected_hits: expected,
            });
        }
        sim.step_with(t, pre)?;
    }
    Ok((sim.done()?, None))
}

fn contact_ring_saturation() -> Result<(CaptureArtifact, Option<InjectionSpec>), CaptureError> {
    let desc = crate::util::scenario_world_desc("contact_ring_saturation");
    let mut sim = Sim::new("contact_ring_saturation", desc, 120)?;
    // 4 盒落体:接触序列可重复;小 ring(8)仍可能 backlog,预算截断在 header 登记。
    let mut descs = vec![ground()];
    for i in 0..4 {
        descs.push(dyn_box(i as f32 * 1.2 - 1.8, 1.0 + i as f32 * 0.05, 0.0, 0.45));
    }
    let ids = sim.add_batch(&descs)?;
    sim.step_already_applied(
        0,
        vec![JournalCommand::CreateBodies {
            descs,
            assigned_ids: body_ids_bits(&ids),
        }],
    )?;
    for t in 1..120 {
        sim.step_with(t, vec![])?;
    }
    Ok((sim.done()?, None))
}

fn mixed_soup_72() -> Result<(CaptureArtifact, Option<InjectionSpec>), CaptureError> {
    let desc = crate::util::scenario_world_desc("mixed_soup_72");
    let mut sim = Sim::new("mixed_soup_72", desc, 300)?;
    let mut descs: Vec<BodyDesc> = (0..71)
        .map(|i| {
            dyn_box(
                (i % 11) as f32 * 1.2 - 6.0,
                6.0 + (i / 11) as f32 * 0.4,
                (i / 11) as f32 * 1.2 - 3.0,
                0.35,
            )
        })
        .collect();
    descs.insert(3, ground());
    let ids = sim.add_batch(&descs)?;
    sim.step_already_applied(
        0,
        vec![JournalCommand::CreateBodies {
            descs,
            assigned_ids: body_ids_bits(&ids),
        }],
    )?;
    for t in 1..300 {
        sim.step_with(t, vec![])?;
    }
    Ok((sim.done()?, None))
}
