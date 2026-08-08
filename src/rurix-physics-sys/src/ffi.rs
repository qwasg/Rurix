//! 手写 FFI 声明(RFC-0017 §4.C3):`#[repr(C)]` POD 镜像 + u64/u32 句柄,`extern "C"`
//! 签名逐字段对齐 vendored `JoltC/Functions.h` / `Enums.h`(pin 见 ../VENDOR.md §1)。
//!
//! 布局可信链:C 结构 ↔ JPH C++ 结构由 JoltCImpl 的 `LAYOUT_COMPATIBLE` 静态断言在
//! vendor 构建期强制;本模块 Rust 镜像 ↔ JoltC C 结构由 `ffi_layout_anchors`(本文件
//! 尾部)锚定——全部数值 = 布局真值测量(x86_64-pc-windows-msvc / 单精度 /
//! OBJECT_LAYER_BITS=16,测量程序 = `tools/layout_dump.cpp` 对 vendored 头
//! `offsetof` 实测,2026-07-31;换画像/升 pin 必须重测,U32 `ffi_layout_anchors` 模式)。
//!
//! 字段命名蛇形化但顺序/类型/对齐与 C 侧逐字段一致;`bool` 一律以 `u8` 过境(0/1)。

#![allow(dead_code)] // 镜像按 C 声明全量落地,未读字段不代表可删(布局占位)

use std::ffi::c_void;

// ---------------------------------------------------------------------------
// 标量 typedef(Enums.h 实测尺寸)
// ---------------------------------------------------------------------------

pub type JpcBodyId = u32; // JPC_BodyID
pub type JpcSubShapeId = u32; // JPC_SubShapeID
pub type JpcObjectLayer = u16; // JPC_ObjectLayer(OBJECT_LAYER_BITS=16)
pub type JpcBroadPhaseLayer = u8; // JPC_BroadPhaseLayer

pub const JPC_BODY_ID_INVALID: JpcBodyId = u32::MAX; // JPH::BodyID::cInvalidBodyID

// JPC_MotionType(u8)
pub const JPC_MOTION_TYPE_STATIC: u8 = 0;
pub const JPC_MOTION_TYPE_KINEMATIC: u8 = 1;
pub const JPC_MOTION_TYPE_DYNAMIC: u8 = 2;
// JPC_MotionQuality(u8)
pub const JPC_MOTION_QUALITY_DISCRETE: u8 = 0;
pub const JPC_MOTION_QUALITY_LINEAR_CAST: u8 = 1;
// JPC_Activation(u32)
pub const JPC_ACTIVATION_ACTIVATE: u32 = 0;
pub const JPC_ACTIVATION_DONT_ACTIVATE: u32 = 1;
// JPC_ConstraintSpace(u32)
pub const JPC_CONSTRAINT_SPACE_LOCAL_TO_BODY_COM: u32 = 0;
pub const JPC_CONSTRAINT_SPACE_WORLD_SPACE: u32 = 1;
// JPC_MotorState(u32)
pub const JPC_MOTOR_STATE_OFF: u32 = 0;
pub const JPC_MOTOR_STATE_VELOCITY: u32 = 1;
pub const JPC_MOTOR_STATE_POSITION: u32 = 2;
// JPC_OverrideMassProperties(u8)
pub const JPC_OVERRIDE_MASS_PROPS_CALC_MASS_INERTIA: u8 = 0;
// JPC_PhysicsUpdateError(u32 位掩码)
pub const JPC_PHYSICS_UPDATE_ERROR_NONE: u32 = 0;
// JPC_AllowedDOFs(u8)
pub const JPC_ALLOWED_DOFS_ALL: u8 = 0b0011_1111;
// Enums.h 常量
pub const JPC_MAX_PHYSICS_JOBS: u32 = 2048;
pub const JPC_MAX_PHYSICS_BARRIERS: u32 = 8;

// ---------------------------------------------------------------------------
// 几何基础类型(Functions.h;x86_64 单精度 → JPC_VECTOR_ALIGNMENT = 16)
// ---------------------------------------------------------------------------

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Default)]
pub struct JpcVec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub _w: f32,
}

impl JpcVec3 {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z, _w: 0.0 }
    }
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct JpcVec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct JpcQuat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl JpcQuat {
    pub const IDENTITY: Self = Self::new(0.0, 0.0, 0.0, 1.0);
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
}

/// JPC_Float3(12 字节无对齐垫,与 `[f32; 3]` 同布局)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct JpcFloat3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// JPC_Mat44 = JPC_RMat44(单精度):3 个 Vec4 旋转列 + Vec3 平移列
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct JpcMat44 {
    pub col: [JpcVec4; 3],
    pub col3: JpcVec3,
}

// ---------------------------------------------------------------------------
// 射线 / 形状 cast / overlap(Functions.h:196-257, 1604-1653)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct JpcRayCastResult {
    pub body_id: JpcBodyId,
    pub fraction: f32,
    pub sub_shape_id2: JpcSubShapeId,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct JpcRRayCast {
    pub origin: JpcVec3, // JPC_RVec3 = JPC_Vec3(单精度)
    pub direction: JpcVec3,
}

#[repr(C)]
pub struct JpcCastRayArgs {
    pub ray: JpcRRayCast,
    pub result: JpcRayCastResult,
    pub broad_phase_layer_filter: *const JpcBroadPhaseLayerFilter,
    pub object_layer_filter: *const JpcObjectLayerFilter,
    pub body_filter: *const JpcBodyFilter,
    pub shape_filter: *const c_void, // JPC_ShapeFilter(本切片恒 null)
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct JpcCollideShapeResult {
    pub contact_point_on1: JpcVec3,
    pub contact_point_on2: JpcVec3,
    pub penetration_axis: JpcVec3,
    pub penetration_depth: f32,
    pub sub_shape_id1: JpcSubShapeId,
    pub sub_shape_id2: JpcSubShapeId,
    pub body_id2: JpcBodyId,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct JpcShapeCastResult {
    pub base: JpcCollideShapeResult,
    pub fraction: f32,
    pub is_back_face_hit: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct JpcShapeCastSettings {
    pub active_edge_mode: u8,
    pub collect_faces_mode: u8,
    pub collision_tolerance: f32,
    pub penetration_tolerance: f32,
    pub active_edge_movement_direction: JpcVec3,
    pub back_face_mode_triangles: u8,
    pub back_face_mode_convex: u8,
    pub use_shrunken_shape_and_convex_radius: u8,
    pub return_deepest_point: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct JpcCollideShapeSettings {
    pub active_edge_mode: u8,
    pub collect_faces_mode: u8,
    pub collision_tolerance: f32,
    pub penetration_tolerance: f32,
    pub active_edge_movement_direction: JpcVec3,
    pub max_separation_distance: f32,
    pub back_face_mode: u8,
}

#[repr(C)]
pub struct JpcRShapeCast {
    pub shape: *const JpcShape,
    pub scale: JpcVec3,
    pub center_of_mass_start: JpcMat44,
    pub direction: JpcVec3,
}

#[repr(C)]
pub struct JpcCastShapeArgs {
    pub shape_cast: JpcRShapeCast,
    pub settings: JpcShapeCastSettings,
    pub base_offset: JpcVec3,
    pub collector: *mut JpcCastShapeCollector,
    pub broad_phase_layer_filter: *const JpcBroadPhaseLayerFilter,
    pub object_layer_filter: *const JpcObjectLayerFilter,
    pub body_filter: *const JpcBodyFilter,
    pub shape_filter: *const c_void,
}

#[repr(C)]
pub struct JpcCollideShapeArgs {
    pub shape: *const JpcShape,
    pub shape_scale: JpcVec3,
    pub center_of_mass_transform: JpcMat44,
    pub settings: JpcCollideShapeSettings,
    pub base_offset: JpcVec3,
    pub collector: *mut JpcCollideShapeCollector,
    pub broad_phase_layer_filter: *const JpcBroadPhaseLayerFilter,
    pub object_layer_filter: *const JpcObjectLayerFilter,
    pub body_filter: *const JpcBodyFilter,
    pub shape_filter: *const c_void,
}

// ---------------------------------------------------------------------------
// 接触(Functions.h:477-535)
// ---------------------------------------------------------------------------

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct JpcContactPoints {
    pub length: u32,
    pub points: [JpcVec3; 64],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct JpcContactManifold {
    pub base_offset: JpcVec3, // JPC_RVec3
    pub world_space_normal: JpcVec3,
    pub penetration_depth: f32,
    pub sub_shape_id1: JpcSubShapeId,
    pub sub_shape_id2: JpcSubShapeId,
    pub relative_contact_points_on1: JpcContactPoints,
    pub relative_contact_points_on2: JpcContactPoints,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct JpcContactSettings {
    pub combined_friction: f32,
    pub combined_restitution: f32,
    pub inv_mass_scale1: f32,
    pub inv_inertia_scale1: f32,
    pub inv_mass_scale2: f32,
    pub inv_inertia_scale2: f32,
    pub is_sensor: u8,
    pub relative_linear_surface_velocity: JpcVec3,
    pub relative_angular_surface_velocity: JpcVec3,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct JpcSubShapeIdPair {
    pub body1_id: JpcBodyId,
    pub sub_shape_id1: JpcSubShapeId,
    pub body2_id: JpcBodyId,
    pub sub_shape_id2: JpcSubShapeId,
}

// ---------------------------------------------------------------------------
// BodyCreationSettings(Functions.h:1313-1346;CollisionGroup 上游已注释,不含)
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct JpcBodyCreationSettings {
    pub position: JpcVec3, // JPC_RVec3
    pub rotation: JpcQuat,
    pub linear_velocity: JpcVec3,
    pub angular_velocity: JpcVec3,
    pub user_data: u64,
    pub object_layer: JpcObjectLayer,
    pub motion_type: u8,
    pub allowed_dofs: u8,
    pub allow_dynamic_or_kinematic: u8,
    pub is_sensor: u8,
    pub collide_kinematic_vs_non_dynamic: u8,
    pub use_manifold_reduction: u8,
    pub apply_gyroscopic_force: u8,
    pub motion_quality: u8,
    pub enhanced_internal_edge_removal: u8,
    pub allow_sleeping: u8,
    pub friction: f32,
    pub restitution: f32,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub max_linear_velocity: f32,
    pub max_angular_velocity: f32,
    pub gravity_factor: f32,
    pub num_velocity_steps_override: u32,
    pub num_position_steps_override: u32,
    pub override_mass_properties: u8,
    pub inertia_multiplier: f32,
    pub shape: *const JpcShape,
}

// ---------------------------------------------------------------------------
// 形状设置(Functions.h:1085-1239)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct JpcSphereShapeSettings {
    pub user_data: u64,
    pub density: f32,
    pub radius: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct JpcBoxShapeSettings {
    pub user_data: u64,
    pub density: f32,
    pub half_extent: JpcVec3,
    pub convex_radius: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct JpcCapsuleShapeSettings {
    pub user_data: u64,
    pub density: f32,
    pub radius: f32,
    pub half_height_of_cylinder: f32,
}

#[repr(C)]
pub struct JpcConvexHullShapeSettings {
    pub user_data: u64,
    pub density: f32,
    pub points: *const JpcVec3,
    pub points_len: usize,
    pub max_convex_radius: f32,
    pub max_error_convex_radius: f32,
    pub hull_tolerance: f32,
}

#[repr(C)]
pub struct JpcMeshShapeSettings {
    pub user_data: u64,
    pub triangle_vertices: *mut JpcFloat3,
    pub triangle_vertices_len: usize,
    pub indexed_triangles: *mut JpcIndexedTriangle,
    pub indexed_triangles_len: usize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct JpcIndexedTriangle {
    pub idx: [u32; 3],
    pub material_index: u32,
    pub user_data: u32,
}

// ---------------------------------------------------------------------------
// 回调函数表(Functions.h:348-471, 588-691;按值传入 JPC_Xxx_new)
// ---------------------------------------------------------------------------

pub type JpcGetNumBroadPhaseLayersFn = unsafe extern "C" fn(*const c_void) -> u32;
pub type JpcGetBroadPhaseLayerFn =
    unsafe extern "C" fn(*const c_void, JpcObjectLayer) -> JpcBroadPhaseLayer;

#[repr(C)]
pub struct JpcBroadPhaseLayerInterfaceFns {
    pub get_num_broad_phase_layers: Option<JpcGetNumBroadPhaseLayersFn>,
    pub get_broad_phase_layer: Option<JpcGetBroadPhaseLayerFn>,
}

pub type JpcOvbShouldCollideFn =
    unsafe extern "C" fn(*const c_void, JpcObjectLayer, JpcBroadPhaseLayer) -> bool;

#[repr(C)]
pub struct JpcObjectVsBroadPhaseLayerFilterFns {
    pub should_collide: Option<JpcOvbShouldCollideFn>,
}

pub type JpcOlpShouldCollideFn =
    unsafe extern "C" fn(*const c_void, JpcObjectLayer, JpcObjectLayer) -> bool;

#[repr(C)]
pub struct JpcObjectLayerPairFilterFns {
    pub should_collide: Option<JpcOlpShouldCollideFn>,
}

pub type JpcOlShouldCollideFn = unsafe extern "C" fn(*const c_void, JpcObjectLayer) -> bool;

#[repr(C)]
pub struct JpcObjectLayerFilterFns {
    pub should_collide: Option<JpcOlShouldCollideFn>,
}

pub type JpcBodyShouldCollideFn = unsafe extern "C" fn(*const c_void, JpcBodyId) -> bool;
pub type JpcBodyShouldCollideLockedFn = unsafe extern "C" fn(*const c_void, *const JpcBody) -> bool;

#[repr(C)]
pub struct JpcBodyFilterFns {
    pub should_collide: Option<JpcBodyShouldCollideFn>,
    pub should_collide_locked: Option<JpcBodyShouldCollideLockedFn>,
}

pub type JpcOnContactValidateFn = unsafe extern "C" fn(
    *mut c_void,
    *const JpcBody,
    *const JpcBody,
    JpcVec3, // JPC_RVec3 按值
    *const JpcCollideShapeResult,
) -> u32; // JPC_ValidateResult
pub type JpcOnContactAddedFn = unsafe extern "C" fn(
    *mut c_void,
    *const JpcBody,
    *const JpcBody,
    *const JpcContactManifold,
    *mut JpcContactSettings,
);
pub type JpcOnContactRemovedFn = unsafe extern "C" fn(*mut c_void, *const JpcSubShapeIdPair);

#[repr(C)]
pub struct JpcContactListenerFns {
    pub on_contact_validate: Option<JpcOnContactValidateFn>,
    pub on_contact_added: Option<JpcOnContactAddedFn>,
    pub on_contact_persisted: Option<JpcOnContactAddedFn>,
    pub on_contact_removed: Option<JpcOnContactRemovedFn>,
}

pub type JpcCollectorResetFn = unsafe extern "C" fn(*mut c_void);
pub type JpcCastShapeAddHitFn =
    unsafe extern "C" fn(*mut c_void, *mut JpcCastShapeCollector, *const JpcShapeCastResult);

#[repr(C)]
pub struct JpcCastShapeCollectorFns {
    pub reset: Option<JpcCollectorResetFn>,
    pub add_hit: Option<JpcCastShapeAddHitFn>,
}

pub type JpcCollideShapeAddHitFn =
    unsafe extern "C" fn(*mut c_void, *mut JpcCollideShapeCollector, *const JpcCollideShapeResult);

#[repr(C)]
pub struct JpcCollideShapeCollectorFns {
    pub reset: Option<JpcCollectorResetFn>,
    pub add_hit: Option<JpcCollideShapeAddHitFn>,
}

// ---------------------------------------------------------------------------
// opaque 句柄(FFI 边界只过指针,Rust 侧永不解引用其内部)
// ---------------------------------------------------------------------------

macro_rules! opaque {
    ($($name:ident),+) => {
        $(
            #[repr(C)]
            pub struct $name {
                _private: [u8; 0],
            }
        )+
    };
}

opaque!(
    JpcBody,
    JpcShape,
    JpcPhysicsSystem,
    JpcBodyInterface,
    JpcBodyLockInterface,
    JpcBodyLockRead,
    JpcBodyLockWrite,
    JpcBodyLockMultiWrite,
    JpcNarrowPhaseQuery,
    JpcTempAllocatorImpl,
    JpcJobSystem,
    JpcJobSystemThreadPool,
    JpcBroadPhaseLayerInterface,
    JpcBroadPhaseLayerFilter,
    JpcObjectVsBroadPhaseLayerFilter,
    JpcObjectLayerPairFilter,
    JpcObjectLayerFilter,
    JpcBodyFilter,
    JpcContactListener,
    JpcCastShapeCollector,
    JpcCollideShapeCollector,
    JpcString,
    JpcConstraint,
    JpcHingeConstraint
);

/// JPC_ConstraintSettings(32B,align 8;layout_dump 2026-08-06)。
#[repr(C)]
#[derive(Clone, Copy)]
pub struct JpcConstraintSettings {
    pub enabled: u8,
    pub _pad0: [u8; 3],
    pub constraint_priority: u32,
    pub num_velocity_steps_override: u32,
    pub num_position_steps_override: u32,
    pub draw_constraint_size: f32,
    pub _pad1: u32,
    pub user_data: u64,
}

/// JPC_SpringSettings(12B)。
#[repr(C)]
#[derive(Clone, Copy)]
pub struct JpcSpringSettings {
    pub mode: u8,
    pub _pad0: [u8; 3],
    pub frequency_or_stiffness: f32,
    pub damping: f32,
}

/// JPC_MotorSettings(28B)。
#[repr(C)]
#[derive(Clone, Copy)]
pub struct JpcMotorSettings {
    pub spring_settings: JpcSpringSettings,
    pub min_force_limit: f32,
    pub max_force_limit: f32,
    pub min_torque_limit: f32,
    pub max_torque_limit: f32,
}

/// JPC_HingeConstraintSettings(208B,align 16;layout_dump 2026-08-06)。
#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub struct JpcHingeConstraintSettings {
    pub constraint_settings: JpcConstraintSettings, // 0
    pub space: u32,                                 // 32
    pub _pad_space: [u8; 12],                       // 36 → Point1@48
    pub point1: JpcVec3,                            // 48
    pub hinge_axis1: JpcVec3,                       // 64
    pub normal_axis1: JpcVec3,                      // 80
    pub point2: JpcVec3,                            // 96
    pub hinge_axis2: JpcVec3,                       // 112
    pub normal_axis2: JpcVec3,                      // 128
    pub limits_min: f32,                            // 144
    pub limits_max: f32,                            // 148
    pub limits_spring_settings: JpcSpringSettings,  // 152
    pub max_friction_torque: f32,                   // 164
    pub motor_settings: JpcMotorSettings,           // 168
    pub _pad_end: [u8; 12],                         // 196 → size 208
}

// ---------------------------------------------------------------------------
// extern "C" 函数声明(逐签名对齐 Functions.h;仅声明本切片消费子集)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    // 进程级初始化(U33:一次注册、进程常驻,镜像 U1 loader 不卸载纪律)
    pub fn JPC_RegisterDefaultAllocator();
    pub fn JPC_FactoryInit();
    pub fn JPC_RegisterTypes();

    // TempAllocatorImpl / JobSystem
    pub fn JPC_TempAllocatorImpl_new(size: u32) -> *mut JpcTempAllocatorImpl;
    pub fn JPC_TempAllocatorImpl_delete(object: *mut JpcTempAllocatorImpl);
    pub fn JPC_JobSystemThreadPool_new3(
        max_jobs: u32,
        max_barriers: u32,
        num_threads: i32,
    ) -> *mut JpcJobSystemThreadPool;
    pub fn JPC_JobSystemThreadPool_delete(object: *mut JpcJobSystemThreadPool);

    // 层接口 / 过滤器(构造签名统一:self 原样回传回调首参)
    pub fn JPC_BroadPhaseLayerInterface_new(
        self_: *const c_void,
        fns: JpcBroadPhaseLayerInterfaceFns,
    ) -> *mut JpcBroadPhaseLayerInterface;
    pub fn JPC_BroadPhaseLayerInterface_delete(object: *mut JpcBroadPhaseLayerInterface);
    pub fn JPC_ObjectVsBroadPhaseLayerFilter_new(
        self_: *const c_void,
        fns: JpcObjectVsBroadPhaseLayerFilterFns,
    ) -> *mut JpcObjectVsBroadPhaseLayerFilter;
    pub fn JPC_ObjectVsBroadPhaseLayerFilter_delete(object: *mut JpcObjectVsBroadPhaseLayerFilter);
    pub fn JPC_ObjectLayerPairFilter_new(
        self_: *const c_void,
        fns: JpcObjectLayerPairFilterFns,
    ) -> *mut JpcObjectLayerPairFilter;
    pub fn JPC_ObjectLayerPairFilter_delete(object: *mut JpcObjectLayerPairFilter);
    pub fn JPC_ObjectLayerFilter_new(
        self_: *const c_void,
        fns: JpcObjectLayerFilterFns,
    ) -> *mut JpcObjectLayerFilter;
    pub fn JPC_ObjectLayerFilter_delete(object: *mut JpcObjectLayerFilter);
    pub fn JPC_BodyFilter_new(self_: *const c_void, fns: JpcBodyFilterFns) -> *mut JpcBodyFilter;
    pub fn JPC_BodyFilter_delete(object: *mut JpcBodyFilter);

    // 接触监听器 / cast 收集器
    pub fn JPC_ContactListener_new(
        self_: *mut c_void,
        fns: JpcContactListenerFns,
    ) -> *mut JpcContactListener;
    pub fn JPC_ContactListener_delete(object: *mut JpcContactListener);
    pub fn JPC_CastShapeCollector_new(
        self_: *mut c_void,
        fns: JpcCastShapeCollectorFns,
    ) -> *mut JpcCastShapeCollector;
    pub fn JPC_CastShapeCollector_delete(object: *mut JpcCastShapeCollector);
    pub fn JPC_CollideShapeCollector_new(
        self_: *mut c_void,
        fns: JpcCollideShapeCollectorFns,
    ) -> *mut JpcCollideShapeCollector;
    pub fn JPC_CollideShapeCollector_delete(object: *mut JpcCollideShapeCollector);

    // PhysicsSystem
    pub fn JPC_PhysicsSystem_new() -> *mut JpcPhysicsSystem;
    pub fn JPC_PhysicsSystem_delete(object: *mut JpcPhysicsSystem);
    pub fn JPC_PhysicsSystem_Init(
        self_: *mut JpcPhysicsSystem,
        max_bodies: u32,
        num_body_mutexes: u32,
        max_body_pairs: u32,
        max_contact_constraints: u32,
        broad_phase_layer_interface: *mut JpcBroadPhaseLayerInterface,
        object_vs_broad_phase_layer_filter: *mut JpcObjectVsBroadPhaseLayerFilter,
        object_layer_pair_filter: *mut JpcObjectLayerPairFilter,
    );
    pub fn JPC_PhysicsSystem_Update(
        self_: *mut JpcPhysicsSystem,
        delta_time: f32,
        collision_steps: i32,
        temp_allocator: *mut JpcTempAllocatorImpl,
        job_system: *mut JpcJobSystem,
    ) -> u32; // JPC_PhysicsUpdateError
    pub fn JPC_PhysicsSystem_SetGravity(self_: *mut JpcPhysicsSystem, gravity: JpcVec3);
    pub fn JPC_PhysicsSystem_GetBodyInterface(
        self_: *mut JpcPhysicsSystem,
    ) -> *mut JpcBodyInterface;
    pub fn JPC_PhysicsSystem_GetBodyLockInterface(
        self_: *mut JpcPhysicsSystem,
    ) -> *const JpcBodyLockInterface;
    pub fn JPC_PhysicsSystem_GetNarrowPhaseQuery(
        self_: *const JpcPhysicsSystem,
    ) -> *const JpcNarrowPhaseQuery;
    pub fn JPC_PhysicsSystem_SetContactListener(
        self_: *mut JpcPhysicsSystem,
        listener: *mut JpcContactListener,
    );

    // Body / BodyInterface
    pub fn JPC_Body_GetID(self_: *const JpcBody) -> JpcBodyId;
    pub fn JPC_Body_GetWorldSpaceSurfaceNormal(
        self_: *const JpcBody,
        sub_shape_id: JpcSubShapeId,
        position: JpcVec3, // JPC_RVec3 按值
    ) -> JpcVec3;
    pub fn JPC_BodyInterface_CreateBody(
        self_: *mut JpcBodyInterface,
        settings: *const JpcBodyCreationSettings,
    ) -> *mut JpcBody;
    pub fn JPC_BodyInterface_DestroyBody(self_: *mut JpcBodyInterface, body_id: JpcBodyId);
    // 注:JPC_BodyInterface_DestroyBodies 在 JoltC 头声明但 impl 被上游注释(WIP 缺口,
    // 处置 (c):Rust 侧逐 DestroyBody 循环 — VENDOR.md §3 计划外缺口登记)。
    pub fn JPC_BodyInterface_AddBodiesPrepare(
        self_: *mut JpcBodyInterface,
        bodies: *mut JpcBodyId,
        number: i32,
    ) -> *mut c_void;
    pub fn JPC_BodyInterface_AddBodiesFinalize(
        self_: *mut JpcBodyInterface,
        bodies: *mut JpcBodyId,
        number: i32,
        add_state: *mut c_void,
        activation_mode: u32, // JPC_Activation
    );
    pub fn JPC_BodyInterface_AddBodiesAbort(
        self_: *mut JpcBodyInterface,
        bodies: *mut JpcBodyId,
        number: i32,
        add_state: *mut c_void,
    );
    pub fn JPC_BodyInterface_RemoveBodies(
        self_: *mut JpcBodyInterface,
        bodies: *mut JpcBodyId,
        number: i32,
    );
    pub fn JPC_BodyInterface_IsAdded(self_: *const JpcBodyInterface, body_id: JpcBodyId) -> bool;
    pub fn JPC_BodyInterface_IsActive(self_: *const JpcBodyInterface, body_id: JpcBodyId) -> bool;
    pub fn JPC_BodyInterface_ActivateBody(self_: *mut JpcBodyInterface, body_id: JpcBodyId);
    pub fn JPC_BodyInterface_DeactivateBody(self_: *mut JpcBodyInterface, body_id: JpcBodyId);
    pub fn JPC_BodyInterface_GetPositionAndRotation(
        self_: *const JpcBodyInterface,
        body_id: JpcBodyId,
        out_position: *mut JpcVec3,
        out_rotation: *mut JpcQuat,
    );
    pub fn JPC_BodyInterface_SetPositionAndRotation(
        self_: *mut JpcBodyInterface,
        body_id: JpcBodyId,
        position: JpcVec3,
        rotation: JpcQuat,
        activation_mode: u32, // JPC_Activation
    );
    pub fn JPC_BodyInterface_SetPositionRotationAndVelocity(
        self_: *mut JpcBodyInterface,
        body_id: JpcBodyId,
        position: JpcVec3,
        rotation: JpcQuat,
        linear_velocity: JpcVec3,
        angular_velocity: JpcVec3,
    );
    pub fn JPC_BodyInterface_GetLinearVelocity(
        self_: *const JpcBodyInterface,
        body_id: JpcBodyId,
    ) -> JpcVec3;
    pub fn JPC_BodyInterface_SetLinearVelocity(
        self_: *mut JpcBodyInterface,
        body_id: JpcBodyId,
        linear_velocity: JpcVec3,
    );
    pub fn JPC_BodyInterface_GetAngularVelocity(
        self_: *const JpcBodyInterface,
        body_id: JpcBodyId,
    ) -> JpcVec3;
    pub fn JPC_BodyInterface_SetAngularVelocity(
        self_: *mut JpcBodyInterface,
        body_id: JpcBodyId,
        angular_velocity: JpcVec3,
    );
    pub fn JPC_BodyInterface_MoveKinematic(
        self_: *mut JpcBodyInterface,
        body_id: JpcBodyId,
        target_position: JpcVec3,
        target_rotation: JpcQuat,
        delta_time: f32,
    );
    pub fn JPC_BodyInterface_AddImpulse(
        self_: *mut JpcBodyInterface,
        body_id: JpcBodyId,
        impulse: JpcVec3,
    );
    /// 世界系点施力(M70 载具悬挂/驱动/侧向力面;`JPC_RVec3` 在
    /// `JPC_DOUBLE_PRECISION` 未定义档 typedef 为 `JPC_Vec3`,build.rs 固定
    /// `DOUBLE_PRECISION=OFF`,故 point 形参用 `JpcVec3` 布局等价。
    pub fn JPC_BodyInterface_AddForceAtPoint(
        self_: *mut JpcBodyInterface,
        body_id: JpcBodyId,
        force: JpcVec3,
        point: JpcVec3,
    );

    // BodyLock(读世界空间表面法线用;cast_ray 命中回填 normal)
    pub fn JPC_BodyLockRead_new(
        interface: *const JpcBodyLockInterface,
        body_id: JpcBodyId,
    ) -> *mut JpcBodyLockRead;
    pub fn JPC_BodyLockRead_delete(self_: *mut JpcBodyLockRead);
    pub fn JPC_BodyLockRead_Succeeded(self_: *mut JpcBodyLockRead) -> bool;
    pub fn JPC_BodyLockRead_GetBody(self_: *mut JpcBodyLockRead) -> *const JpcBody;

    // BodyLockWrite(约束 Create 取 Body*)
    pub fn JPC_BodyLockWrite_new(
        interface: *const JpcBodyLockInterface,
        body_id: JpcBodyId,
    ) -> *mut JpcBodyLockWrite;
    pub fn JPC_BodyLockWrite_delete(self_: *mut JpcBodyLockWrite);
    pub fn JPC_BodyLockWrite_Succeeded(self_: *mut JpcBodyLockWrite) -> bool;
    pub fn JPC_BodyLockWrite_GetBody(self_: *mut JpcBodyLockWrite) -> *mut JpcBody;

    // BodyLockMultiWrite(两体约束 Create 必须成对加锁,避免双 BodyLockWrite 未定义序)
    pub fn JPC_BodyLockMultiWrite_new(
        interface: *const JpcBodyLockInterface,
        body_ids: *const JpcBodyId,
        number: i32,
    ) -> *mut JpcBodyLockMultiWrite;
    pub fn JPC_BodyLockMultiWrite_delete(self_: *mut JpcBodyLockMultiWrite);
    pub fn JPC_BodyLockMultiWrite_GetBody(
        self_: *mut JpcBodyLockMultiWrite,
        body_index: i32,
    ) -> *mut JpcBody;

    // Constraint / Hinge(M66 capture journal;消费既有导出,零 vendor 补丁)
    pub fn JPC_HingeConstraintSettings_default(settings: *mut JpcHingeConstraintSettings);
    pub fn JPC_HingeConstraintSettings_Create(
        self_: *const JpcHingeConstraintSettings,
        body1: *mut JpcBody,
        body2: *mut JpcBody,
    ) -> *mut JpcHingeConstraint;
    pub fn JPC_PhysicsSystem_AddConstraint(
        self_: *mut JpcPhysicsSystem,
        constraint: *mut JpcConstraint,
    );
    pub fn JPC_PhysicsSystem_RemoveConstraint(
        self_: *mut JpcPhysicsSystem,
        constraint: *mut JpcConstraint,
    );
    pub fn JPC_Constraint_AddRef(self_: *const JpcConstraint);
    pub fn JPC_Constraint_Release(self_: *const JpcConstraint);
    pub fn JPC_Constraint_GetEnabled(self_: *const JpcConstraint) -> bool;
    pub fn JPC_HingeConstraint_SetMotorState(self_: *mut JpcHingeConstraint, state: u32);
    pub fn JPC_HingeConstraint_GetMotorState(self_: *const JpcHingeConstraint) -> u32;
    pub fn JPC_HingeConstraint_SetTargetAngularVelocity(
        self_: *mut JpcHingeConstraint,
        angular_velocity: f32,
    );

    // 形状(default 填充 + Create;Create 成功 → shape 引用计数 1,调用方持有)
    pub fn JPC_SphereShapeSettings_default(object: *mut JpcSphereShapeSettings);
    pub fn JPC_SphereShapeSettings_Create(
        self_: *const JpcSphereShapeSettings,
        out_shape: *mut *mut JpcShape,
        out_error: *mut *mut JpcString,
    ) -> bool;
    pub fn JPC_BoxShapeSettings_default(object: *mut JpcBoxShapeSettings);
    pub fn JPC_BoxShapeSettings_Create(
        self_: *const JpcBoxShapeSettings,
        out_shape: *mut *mut JpcShape,
        out_error: *mut *mut JpcString,
    ) -> bool;
    pub fn JPC_CapsuleShapeSettings_default(object: *mut JpcCapsuleShapeSettings);
    pub fn JPC_CapsuleShapeSettings_Create(
        self_: *const JpcCapsuleShapeSettings,
        out_shape: *mut *mut JpcShape,
        out_error: *mut *mut JpcString,
    ) -> bool;
    pub fn JPC_ConvexHullShapeSettings_default(object: *mut JpcConvexHullShapeSettings);
    pub fn JPC_ConvexHullShapeSettings_Create(
        self_: *const JpcConvexHullShapeSettings,
        out_shape: *mut *mut JpcShape,
        out_error: *mut *mut JpcString,
    ) -> bool;
    pub fn JPC_MeshShapeSettings_default(object: *mut JpcMeshShapeSettings);
    pub fn JPC_MeshShapeSettings_Create(
        self_: *const JpcMeshShapeSettings,
        out_shape: *mut *mut JpcShape,
        out_error: *mut *mut JpcString,
    ) -> bool;
    pub fn JPC_Shape_Release(self_: *const JpcShape);
    pub fn JPC_Shape_GetCenterOfMass(self_: *const JpcShape) -> JpcVec3;
    pub fn JPC_Shape_GetVolume(self_: *const JpcShape) -> f32;

    // NarrowPhaseQuery(step 外只读路径,多线程并发安全 — Jolt 上游文档口径)
    pub fn JPC_NarrowPhaseQuery_CastRay(
        self_: *const JpcNarrowPhaseQuery,
        args: *mut JpcCastRayArgs,
    ) -> bool;
    pub fn JPC_NarrowPhaseQuery_CastShape(
        self_: *const JpcNarrowPhaseQuery,
        args: *mut JpcCastShapeArgs,
    );
    pub fn JPC_NarrowPhaseQuery_CollideShape(
        self_: *const JpcNarrowPhaseQuery,
        args: *mut JpcCollideShapeArgs,
    );

    // 设置默认值填充 / 体创建默认值
    pub fn JPC_ShapeCastSettings_default(object: *mut JpcShapeCastSettings);
    pub fn JPC_CollideShapeSettings_default(object: *mut JpcCollideShapeSettings);
    pub fn JPC_BodyCreationSettings_default(settings: *mut JpcBodyCreationSettings);

    // 错误串(Create 失败路径;读完即 delete)
    pub fn JPC_String_c_str(self_: *mut JpcString) -> *const std::ffi::c_char;
    pub fn JPC_String_delete(self_: *mut JpcString);
}

// ---------------------------------------------------------------------------
// ffi_layout_anchors(U32 模式):Rust 镜像 ↔ JoltC C 结构布局锚定
// 数值 = layout_dump(x86_64-pc-windows-msvc,单精度,OBJECT_LAYER_BITS=16)实测;
// vendor pin 或构建画像变更时必须重新测量(见 VENDOR.md §4)。
// ---------------------------------------------------------------------------

#[cfg(test)]
mod ffi_layout_anchors {
    use super::*;
    use core::mem::{align_of, offset_of, size_of};

    const _: () = {
        // 基础几何类型
        assert!(size_of::<JpcVec3>() == 16 && align_of::<JpcVec3>() == 16);
        assert!(size_of::<JpcVec4>() == 16 && align_of::<JpcVec4>() == 16);
        assert!(size_of::<JpcQuat>() == 16 && align_of::<JpcQuat>() == 16);
        assert!(size_of::<JpcFloat3>() == 12 && align_of::<JpcFloat3>() == 4);
        assert!(size_of::<JpcMat44>() == 64 && align_of::<JpcMat44>() == 16);
        assert!(offset_of!(JpcMat44, col) == 0 && offset_of!(JpcMat44, col3) == 48);

        // 射线
        assert!(size_of::<JpcRayCastResult>() == 12 && align_of::<JpcRayCastResult>() == 4);
        assert!(offset_of!(JpcRayCastResult, body_id) == 0);
        assert!(offset_of!(JpcRayCastResult, fraction) == 4);
        assert!(offset_of!(JpcRayCastResult, sub_shape_id2) == 8);
        assert!(size_of::<JpcRRayCast>() == 32 && align_of::<JpcRRayCast>() == 16);
        assert!(offset_of!(JpcRRayCast, origin) == 0 && offset_of!(JpcRRayCast, direction) == 16);
        assert!(size_of::<JpcCastRayArgs>() == 80 && align_of::<JpcCastRayArgs>() == 16);
        assert!(offset_of!(JpcCastRayArgs, ray) == 0 && offset_of!(JpcCastRayArgs, result) == 32);
        assert!(offset_of!(JpcCastRayArgs, broad_phase_layer_filter) == 48);
        assert!(offset_of!(JpcCastRayArgs, object_layer_filter) == 56);
        assert!(offset_of!(JpcCastRayArgs, body_filter) == 64);
        assert!(offset_of!(JpcCastRayArgs, shape_filter) == 72);

        // cast / collide 结果与设置
        assert!(
            size_of::<JpcCollideShapeResult>() == 64 && align_of::<JpcCollideShapeResult>() == 16
        );
        assert!(offset_of!(JpcCollideShapeResult, contact_point_on1) == 0);
        assert!(offset_of!(JpcCollideShapeResult, contact_point_on2) == 16);
        assert!(offset_of!(JpcCollideShapeResult, penetration_axis) == 32);
        assert!(offset_of!(JpcCollideShapeResult, penetration_depth) == 48);
        assert!(offset_of!(JpcCollideShapeResult, sub_shape_id1) == 52);
        assert!(offset_of!(JpcCollideShapeResult, sub_shape_id2) == 56);
        assert!(offset_of!(JpcCollideShapeResult, body_id2) == 60);
        assert!(size_of::<JpcShapeCastResult>() == 80 && align_of::<JpcShapeCastResult>() == 16);
        assert!(offset_of!(JpcShapeCastResult, base) == 0);
        assert!(offset_of!(JpcShapeCastResult, fraction) == 64);
        assert!(offset_of!(JpcShapeCastResult, is_back_face_hit) == 68);
        assert!(
            size_of::<JpcShapeCastSettings>() == 48 && align_of::<JpcShapeCastSettings>() == 16
        );
        assert!(offset_of!(JpcShapeCastSettings, active_edge_mode) == 0);
        assert!(offset_of!(JpcShapeCastSettings, collect_faces_mode) == 1);
        assert!(offset_of!(JpcShapeCastSettings, collision_tolerance) == 4);
        assert!(offset_of!(JpcShapeCastSettings, penetration_tolerance) == 8);
        assert!(offset_of!(JpcShapeCastSettings, active_edge_movement_direction) == 16);
        assert!(offset_of!(JpcShapeCastSettings, back_face_mode_triangles) == 32);
        assert!(offset_of!(JpcShapeCastSettings, back_face_mode_convex) == 33);
        assert!(offset_of!(JpcShapeCastSettings, use_shrunken_shape_and_convex_radius) == 34);
        assert!(offset_of!(JpcShapeCastSettings, return_deepest_point) == 35);
        assert!(
            size_of::<JpcCollideShapeSettings>() == 48
                && align_of::<JpcCollideShapeSettings>() == 16
        );
        assert!(offset_of!(JpcCollideShapeSettings, max_separation_distance) == 32);
        assert!(offset_of!(JpcCollideShapeSettings, back_face_mode) == 36);
        assert!(size_of::<JpcRShapeCast>() == 112 && align_of::<JpcRShapeCast>() == 16);
        assert!(offset_of!(JpcRShapeCast, shape) == 0 && offset_of!(JpcRShapeCast, scale) == 16);
        assert!(offset_of!(JpcRShapeCast, center_of_mass_start) == 32);
        assert!(offset_of!(JpcRShapeCast, direction) == 96);
        assert!(size_of::<JpcCastShapeArgs>() == 224 && align_of::<JpcCastShapeArgs>() == 16);
        assert!(offset_of!(JpcCastShapeArgs, shape_cast) == 0);
        assert!(offset_of!(JpcCastShapeArgs, settings) == 112);
        assert!(offset_of!(JpcCastShapeArgs, base_offset) == 160);
        assert!(offset_of!(JpcCastShapeArgs, collector) == 176);
        assert!(offset_of!(JpcCastShapeArgs, broad_phase_layer_filter) == 184);
        assert!(offset_of!(JpcCastShapeArgs, object_layer_filter) == 192);
        assert!(offset_of!(JpcCastShapeArgs, body_filter) == 200);
        assert!(offset_of!(JpcCastShapeArgs, shape_filter) == 208);
        assert!(size_of::<JpcCollideShapeArgs>() == 208 && align_of::<JpcCollideShapeArgs>() == 16);
        assert!(offset_of!(JpcCollideShapeArgs, shape) == 0);
        assert!(offset_of!(JpcCollideShapeArgs, shape_scale) == 16);
        assert!(offset_of!(JpcCollideShapeArgs, center_of_mass_transform) == 32);
        assert!(offset_of!(JpcCollideShapeArgs, settings) == 96);
        assert!(offset_of!(JpcCollideShapeArgs, base_offset) == 144);
        assert!(offset_of!(JpcCollideShapeArgs, collector) == 160);
        assert!(offset_of!(JpcCollideShapeArgs, shape_filter) == 192);

        // Constraint / Hinge settings(layout_dump 2026-08-06)
        assert!(size_of::<JpcConstraintSettings>() == 32 && align_of::<JpcConstraintSettings>() == 8);
        assert!(offset_of!(JpcConstraintSettings, enabled) == 0);
        assert!(offset_of!(JpcConstraintSettings, constraint_priority) == 4);
        assert!(offset_of!(JpcConstraintSettings, user_data) == 24);
        assert!(size_of::<JpcSpringSettings>() == 12);
        assert!(size_of::<JpcMotorSettings>() == 28);
        assert!(
            size_of::<JpcHingeConstraintSettings>() == 208
                && align_of::<JpcHingeConstraintSettings>() == 16
        );
        assert!(offset_of!(JpcHingeConstraintSettings, space) == 32);
        assert!(offset_of!(JpcHingeConstraintSettings, point1) == 48);
        assert!(offset_of!(JpcHingeConstraintSettings, point2) == 96);
        assert!(offset_of!(JpcHingeConstraintSettings, limits_min) == 144);
        assert!(offset_of!(JpcHingeConstraintSettings, motor_settings) == 168);

        // BodyCreationSettings
        assert!(
            size_of::<JpcBodyCreationSettings>() == 144
                && align_of::<JpcBodyCreationSettings>() == 16
        );
        assert!(offset_of!(JpcBodyCreationSettings, position) == 0);
        assert!(offset_of!(JpcBodyCreationSettings, rotation) == 16);
        assert!(offset_of!(JpcBodyCreationSettings, linear_velocity) == 32);
        assert!(offset_of!(JpcBodyCreationSettings, angular_velocity) == 48);
        assert!(offset_of!(JpcBodyCreationSettings, user_data) == 64);
        assert!(offset_of!(JpcBodyCreationSettings, object_layer) == 72);
        assert!(offset_of!(JpcBodyCreationSettings, motion_type) == 74);
        assert!(offset_of!(JpcBodyCreationSettings, allowed_dofs) == 75);
        assert!(offset_of!(JpcBodyCreationSettings, allow_dynamic_or_kinematic) == 76);
        assert!(offset_of!(JpcBodyCreationSettings, is_sensor) == 77);
        assert!(offset_of!(JpcBodyCreationSettings, collide_kinematic_vs_non_dynamic) == 78);
        assert!(offset_of!(JpcBodyCreationSettings, use_manifold_reduction) == 79);
        assert!(offset_of!(JpcBodyCreationSettings, apply_gyroscopic_force) == 80);
        assert!(offset_of!(JpcBodyCreationSettings, motion_quality) == 81);
        assert!(offset_of!(JpcBodyCreationSettings, enhanced_internal_edge_removal) == 82);
        assert!(offset_of!(JpcBodyCreationSettings, allow_sleeping) == 83);
        assert!(offset_of!(JpcBodyCreationSettings, friction) == 84);
        assert!(offset_of!(JpcBodyCreationSettings, restitution) == 88);
        assert!(offset_of!(JpcBodyCreationSettings, linear_damping) == 92);
        assert!(offset_of!(JpcBodyCreationSettings, angular_damping) == 96);
        assert!(offset_of!(JpcBodyCreationSettings, max_linear_velocity) == 100);
        assert!(offset_of!(JpcBodyCreationSettings, max_angular_velocity) == 104);
        assert!(offset_of!(JpcBodyCreationSettings, gravity_factor) == 108);
        assert!(offset_of!(JpcBodyCreationSettings, num_velocity_steps_override) == 112);
        assert!(offset_of!(JpcBodyCreationSettings, num_position_steps_override) == 116);
        assert!(offset_of!(JpcBodyCreationSettings, override_mass_properties) == 120);
        assert!(offset_of!(JpcBodyCreationSettings, inertia_multiplier) == 124);
        assert!(offset_of!(JpcBodyCreationSettings, shape) == 128);

        // 接触
        assert!(size_of::<JpcContactPoints>() == 1040 && align_of::<JpcContactPoints>() == 16);
        assert!(offset_of!(JpcContactPoints, length) == 0);
        assert!(offset_of!(JpcContactPoints, points) == 16);
        assert!(size_of::<JpcContactManifold>() == 2128 && align_of::<JpcContactManifold>() == 16);
        assert!(offset_of!(JpcContactManifold, base_offset) == 0);
        assert!(offset_of!(JpcContactManifold, world_space_normal) == 16);
        assert!(offset_of!(JpcContactManifold, penetration_depth) == 32);
        assert!(offset_of!(JpcContactManifold, sub_shape_id1) == 36);
        assert!(offset_of!(JpcContactManifold, sub_shape_id2) == 40);
        assert!(offset_of!(JpcContactManifold, relative_contact_points_on1) == 48);
        assert!(offset_of!(JpcContactManifold, relative_contact_points_on2) == 1088);
        assert!(size_of::<JpcContactSettings>() == 64 && align_of::<JpcContactSettings>() == 16);
        assert!(offset_of!(JpcContactSettings, combined_friction) == 0);
        assert!(offset_of!(JpcContactSettings, combined_restitution) == 4);
        assert!(offset_of!(JpcContactSettings, inv_mass_scale1) == 8);
        assert!(offset_of!(JpcContactSettings, inv_inertia_scale1) == 12);
        assert!(offset_of!(JpcContactSettings, inv_mass_scale2) == 16);
        assert!(offset_of!(JpcContactSettings, inv_inertia_scale2) == 20);
        assert!(offset_of!(JpcContactSettings, is_sensor) == 24);
        assert!(offset_of!(JpcContactSettings, relative_linear_surface_velocity) == 32);
        assert!(offset_of!(JpcContactSettings, relative_angular_surface_velocity) == 48);
        assert!(size_of::<JpcSubShapeIdPair>() == 16 && align_of::<JpcSubShapeIdPair>() == 4);
        assert!(offset_of!(JpcSubShapeIdPair, body1_id) == 0);
        assert!(offset_of!(JpcSubShapeIdPair, sub_shape_id1) == 4);
        assert!(offset_of!(JpcSubShapeIdPair, body2_id) == 8);
        assert!(offset_of!(JpcSubShapeIdPair, sub_shape_id2) == 12);

        // 形状设置
        assert!(
            size_of::<JpcSphereShapeSettings>() == 16 && align_of::<JpcSphereShapeSettings>() == 8
        );
        assert!(offset_of!(JpcSphereShapeSettings, user_data) == 0);
        assert!(offset_of!(JpcSphereShapeSettings, density) == 8);
        assert!(offset_of!(JpcSphereShapeSettings, radius) == 12);
        assert!(size_of::<JpcBoxShapeSettings>() == 48 && align_of::<JpcBoxShapeSettings>() == 16);
        assert!(offset_of!(JpcBoxShapeSettings, user_data) == 0);
        assert!(offset_of!(JpcBoxShapeSettings, density) == 8);
        assert!(offset_of!(JpcBoxShapeSettings, half_extent) == 16);
        assert!(offset_of!(JpcBoxShapeSettings, convex_radius) == 32);
        assert!(
            size_of::<JpcCapsuleShapeSettings>() == 24
                && align_of::<JpcCapsuleShapeSettings>() == 8
        );
        assert!(offset_of!(JpcCapsuleShapeSettings, user_data) == 0);
        assert!(offset_of!(JpcCapsuleShapeSettings, density) == 8);
        assert!(offset_of!(JpcCapsuleShapeSettings, radius) == 12);
        assert!(offset_of!(JpcCapsuleShapeSettings, half_height_of_cylinder) == 16);
        assert!(
            size_of::<JpcConvexHullShapeSettings>() == 48
                && align_of::<JpcConvexHullShapeSettings>() == 8
        );
        assert!(offset_of!(JpcConvexHullShapeSettings, user_data) == 0);
        assert!(offset_of!(JpcConvexHullShapeSettings, density) == 8);
        assert!(offset_of!(JpcConvexHullShapeSettings, points) == 16);
        assert!(offset_of!(JpcConvexHullShapeSettings, points_len) == 24);
        assert!(offset_of!(JpcConvexHullShapeSettings, max_convex_radius) == 32);
        assert!(offset_of!(JpcConvexHullShapeSettings, max_error_convex_radius) == 36);
        assert!(offset_of!(JpcConvexHullShapeSettings, hull_tolerance) == 40);
        assert!(size_of::<JpcMeshShapeSettings>() == 40 && align_of::<JpcMeshShapeSettings>() == 8);
        assert!(offset_of!(JpcMeshShapeSettings, user_data) == 0);
        assert!(offset_of!(JpcMeshShapeSettings, triangle_vertices) == 8);
        assert!(offset_of!(JpcMeshShapeSettings, triangle_vertices_len) == 16);
        assert!(offset_of!(JpcMeshShapeSettings, indexed_triangles) == 24);
        assert!(offset_of!(JpcMeshShapeSettings, indexed_triangles_len) == 32);
        assert!(size_of::<JpcIndexedTriangle>() == 20 && align_of::<JpcIndexedTriangle>() == 4);
        assert!(offset_of!(JpcIndexedTriangle, idx) == 0);
        assert!(offset_of!(JpcIndexedTriangle, material_index) == 12);
        assert!(offset_of!(JpcIndexedTriangle, user_data) == 16);

        // 函数表
        assert!(
            size_of::<JpcContactListenerFns>() == 32 && align_of::<JpcContactListenerFns>() == 8
        );
        assert!(size_of::<JpcCastShapeCollectorFns>() == 16);
        assert!(size_of::<JpcCollideShapeCollectorFns>() == 16);
        assert!(size_of::<JpcBroadPhaseLayerInterfaceFns>() == 16);
        assert!(size_of::<JpcObjectVsBroadPhaseLayerFilterFns>() == 8);
        assert!(size_of::<JpcObjectLayerPairFilterFns>() == 8);
        assert!(size_of::<JpcObjectLayerFilterFns>() == 8);
        assert!(size_of::<JpcBodyFilterFns>() == 16);

        // 标量宽度
        assert!(size_of::<JpcBodyId>() == 4 && size_of::<JpcSubShapeId>() == 4);
        assert!(size_of::<JpcObjectLayer>() == 2 && size_of::<JpcBroadPhaseLayer>() == 1);
    };
}
