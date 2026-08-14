# unsafe-audit: rurix-physics-sys(JoltC C API FFI 边界)

> 注册依据:AGENTS.md 硬规则 9 / 10 §7.6(无注册条目的 unsafe 是 CI 错误);
> 14 §2 常驻集 unsafe-audit 完整性。G6.2 PR-A 激活(RFC-0017 §4.C FFI 与 unsafe
> 纪律章,Agent Approved 2026-07-31;R-G6-1 裁决:自维护薄 FFI 子 crate 绑定
> SecondHalfGames/JoltC C API,rolt/jolt-rust 停滞否决)。
> 编号:number_ledger v1.28 `reserved_in_flight[G6]` claim——**U33 起续号**(main
> on_tree_max U32 时代基线;本文件已登记至 **U53**,U43~U46 跳号不回收;Gov materialize
> 时按实测 `U.next_free` 回填 ledger,本稿不预占)。
> vendor pin / 构建策略 / C-3 缺口审计:[`src/rurix-physics-sys/VENDOR.md`](../src/rurix-physics-sys/VENDOR.md)。

## 范围与豁免

- crate:`src/rurix-physics-sys`(`[lints.rust] unsafe_code = "allow"`;`undocumented_unsafe_blocks
  = "deny"` 维持——每个 unsafe 块强制 `// SAFETY:` 注释;镜像 rurix-rt 豁免模式,RFC-0017 §4.C2)。
- 全仓其余 crate 维持 workspace 默认 `unsafe_code = "deny"`;`rurix-physics`(safe 层)与
  `rurix-render` 维持 `#![forbid(unsafe_code)]`(§4.0-1;sys crate 之外零 `unsafe_code = "allow"` 新增)。
- 全部 unsafe 集中于:`src/rurix-physics-sys/src/ffi.rs`(手写 `extern "C"` 声明 +
  `#[repr(C)]` POD 镜像)+ `src/rurix-physics-sys/src/world.rs`(句柄所有权、回调、
  查询面),逐块 `// SAFETY:` 在位。对外只露 safe Rust 类型与 u64 token(§4.C3;
  不露原生 Jolt/JoltC 指针与类型名)。
- 布局可信链:JoltC 头内 C 结构 ↔ JPH C++ 结构由 `JoltCImpl/JoltC.cpp` 的
  `LAYOUT_COMPATIBLE` 静态断言在 vendor 构建期强制;Rust 镜像 ↔ JoltC C 结构由
  `ffi.rs` 的 `ffi_layout_anchors` 编译期断言锚定(U32 `ffi_layout_anchors` 模式;
  数值 = x86_64-pc-windows-msvc / 单精度 / OBJECT_LAYER_BITS=16 画像对 vendored 头
  `offsetof` 实测,pin 或画像变更必须重测,VENDOR.md §4)。

## 原语清单与验证义务(RustBelt 式)

| # | 原语 | 位置 | 验证义务(SAFETY 不变量) |
|---|---|---|---|
| U33 | 进程级 Jolt 初始化(`JPC_RegisterDefaultAllocator` / `JPC_FactoryInit` / `JPC_RegisterTypes`,`std::sync::Once` 一次注册、进程常驻、不配对 Unregister/FactoryDelete) | world.rs `ensure_jolt_initialized` | `Once` 保证全进程恰好一次、按此序调用;常驻语义镜像 U1 loader 不 `FreeLibrary` 纪律——多 world 反复创建/销毁不触发重复注册竞态;无参数、无返回,失败面 = Jolt 内部断言(Release 构建 USE_ASSERTS=OFF) |
| U34 | JoltC 世界句柄线性配对 create/delete(`TempAllocatorImpl` / `JobSystemThreadPool` / `BroadPhaseLayerInterface` / `ObjectVsBroadPhaseLayerFilter` / `ObjectLayerPairFilter` / `PhysicsSystem` / `ContactListener`)+ 创建失败路径 fail-closed | world.rs `Inner::create` / `CreateGuard::drop` / `Inner::drop` | 每句柄 create 成功即入守卫,null 校验失败 → `CreateGuard` 按**逆创建序**销毁已建句柄后返回确定性 `Err(BackendUnavailable)`(early-return 全路径销毁,无泄漏、无双释放);成功后句柄所有权移交 `Inner`(独占、非 Clone);`Inner::drop` 固定销毁序 = **摘除监听器**(`SetContactListener(null)`,此后回调不再触发,user_data 堆地址不再被引用)→ `PhysicsSystem_delete`(连带 body 管理器内全部 body)→ 逐 body `JPC_Shape_Release`(U38)→ 过滤器/层接口 delete → job delete → temp delete;drop 需 `&mut`(Rust 规则保证无 Update 在飞) |
| U35 | 手写 `extern "C"` 声明 + `#[repr(C)]` POD 镜像(函数 ~60 个、结构 ~25 个、回调函数表 8 个按值传) | ffi.rs 全文 + `ffi_layout_anchors` | 签名逐字段对齐 vendored `JoltC/Functions.h`/`Enums.h`(pin `2982004`,VENDOR.md §1):字段序/尺寸/对齐经 `ffi_layout_anchors` **编译期**断言(60+ 条 size/align/offset,数值 = layout_dump 对 vendored 头 `offsetof` 实测);C `bool` 一律 `u8` 过境(0/1);`JPC_XxxFns` 为函数指针 POD 按值传(MSVC x64 ABI,双方同签名);16 字节 float4 结构按值返回(`GetCenterOfMass` 等)= xmm0,LLVM/MSVC x64 ABI 一致;枚举宽度按 Enums.h 实测(u8/u16/u32) |
| U36 | contact listener 回调(`JPC_ContactListener_new(user_data, fns)`;`OnContactAdded`/`OnContactPersisted`/`OnContactRemoved` 经 `extern "C" fn` + user_data) | world.rs `on_contact_*` / `ContactSink` / `Inner::create` | user_data = `Inner` 持有的 `Box<ContactSink>` 堆稳定地址,生命周期**严格 ≥ 注册窗口**(`Inner::drop` 先 `SetContactListener(null)` 摘除,`Box` 字段在其后析构);回调在 `PhysicsSystem::Update` 内由 Jolt job 线程**多线程触发**(顺序非确定)——事件收集进 `Mutex<VecDeque>`(归一化排序在 safe 层,§4.A5 C-2);回调内**不回抛 panic**(FFI 边界):仅读 POD + 锁内 push,锁经 `into_inner` 兜底不中毒;body/manifold/pair 指针为 JoltC 桥内有效对象,仅在回调内读取 POD 字段,不持有出回调;ring 满 → 确定性丢最旧 + 计数(不 panic,P-01);impulse 首版恒 0(JoltC 回调不含求解后冲量,VENDOR.md §3 收窄登记) |
| U37 | 查询过滤器/收集器栈纪律(`ObjectLayerFilter` / `BodyFilter` / `CastShapeCollector` / `CollideShapeCollector` 的 user_data 指向**调用帧栈上** mask/`Vec`/ctx,`*_new`/`*_delete` 配对) | world.rs `cast_ray` / `cast_shape` / `overlap_shape` + `ol_should_collide` / `ray_body_should_collide*` / `*_add_hit` | user_data 指向栈上状态,生命周期**严格短于注册窗口**(过滤器/收集器对象在查询返回后即 delete,沿 U27 messenger `p_user_data` 栈纪律);`CastShape`/`CollideShape` 在**调用线程同步**执行(collector ctx 的 `&mut` 无跨线程别名);`CastRay` 排除循环的 `Vec<u32>` 在循环全程存活;回调内不 panic;delete 与 new 一一配对(null 分支亦覆盖) |
| U38 | shape 引用计数配对(`*_ShapeSettings_Create` 成功 = 引用计数 1 调用方持有;Jolt `Body` **不**对 shape AddRef——实测 `Body::SetShapeInternal`,VENDOR.md §4) | world.rs `build_shape` / `create_shape` / `add_bodies_batch` / `remove_bodies_batch` / `Inner::drop` / `cast_shape` / `overlap_shape` | world 为每个 body 持有其 shape 引用(`BodyRec.shape`),`remove_bodies_batch` 与 `Inner::drop` 各 `JPC_Shape_Release` **恰好一次**(不双释放);批插中途失败 → 已建形状全 Release + 已 Create 未 Add 的体 `DestroyBody` 回滚(零副作用,P-01);查询临时形状在调用末尾 Release(配对 `create_query_shape`);Create 失败路径读 `JPC_String` 错误串后 `JPC_String_delete`(配对);显式 mass 两遍创建:首遍读取 `GetVolume` 后即 Release 再二遍创建 |
| U39 | 批插 `AddBodiesPrepare` / `AddBodiesFinalize`(`AddState` 裸指针)+ 批移除 `RemoveBodies` + 逐 `DestroyBody`(JoltC `DestroyBodies` impl 被上游注释,WIP 缺口处置 (c) = Rust 循环,VENDOR.md §3) | world.rs `add_bodies_batch` / `remove_bodies_batch` | prepare 产出的 `AddState` 立即交回 finalize 消费,两调用间**无可失败操作**(故无 `AddBodiesAbort` 需求——abort 路径类型面消除);token 全量预校验后才 Remove/Destroy(任一无效 → 零移除,确定性 `Err(InvalidBody)`,P-01);移除序 = 先 `RemoveBodies`(批量)再逐 `DestroyBody`(Jolt 约定序);Jolt token(u32 BodyID)经 u64  widening 过境,`0xFFFFFFFF` 与 > u32::MAX 在 Rust 侧拦截 |
| U40 | `BodyLockRead` 配对(cast_ray 命中法线回填:`JPC_Body_GetWorldSpaceSurfaceNormal`;JoltC CastRay 结果不含法线,VENDOR.md §3 计划外缺口处置) | world.rs `surface_normal` | lock `new`/`delete` 配对;`GetBody` 仅在 `Succeeded` 后调用,返回指针锁期内有效、不持出锁;锁失败 → `[0,0,0]` 确定性回填(登记,不 panic);目标 body 刚被同射线查询命中(存活) |
| U41 | `SysWorld` 的 `unsafe impl Send` / `unsafe impl Sync`(相位门,§4.A4 Q-B / §4.C3) | lib.rs 尾部 | `SysWorld` 独占拥有全部 JoltC 句柄(所有权单向);**变更路径**(`step`/`add_*`/`remove_*`/`drain_contacts`/`set_kinematic_target`/`apply_impulse`)= `&mut self`,Rust 借用规则编译期保证独占,step 相位内 Jolt job 线程只活在 `Update` 调用内;**只读查询路径**(`cast_*`/`overlap_*`/`body_transform`/`active_transforms`/`is_active`)= `&self`,对应 Jolt `NarrowPhaseQuery`/`BodyInterface` 只读面(step 外多线程并发安全,Jolt 上游文档口径),与 step 相位类型面互斥;contact 回调经 `Mutex` 收集,与并发查询无共享可变状态;body 注册表(`HashMap`)在 `&self` 相位不可变 |
| U42 | `mem::zeroed` + `*_default` 初始化模式(settings 结构先 `zeroed::<T>()` 再由 JoltC `JPC_Xxx_default` 填充合法默认值,后覆写消费字段) | world.rs `build_shape` / `cast_shape` / `overlap_shape` / `make_body_settings` | 目标类型为全 POD `#[repr(C)]`(数值 + 裸指针 + u8,无 `bool`/引用/枚举陷阱——`bool` 已按 U35 纪律以 u8 过境),`zeroed` 位模式合法(指针 = null,u8 = 0);`zeroed` 后**必经**对应 `JPC_*_default` 填充再使用(不依赖 Rust 侧对默认值的假设,上游默认值变更随 pin 升级自动跟进);锚定布局见 U35 |
| U47 | `BodyInterface` 速度只读(`JPC_BodyInterface_GetLinearVelocity` / `GetAngularVelocity`) | world.rs `body_velocities` | token 经 `validate_token` 在册;只读路径 step 外线程安全(§4.A4 Q-B);返回 POD `JpcVec3` 分量拷贝进 Rust 数组,不持出 FFI 指针;无效 token → 确定性 `Err(InvalidBody)` |
| U48 | 线速度/角速度写入(`JPC_BodyInterface_SetLinearVelocity` / `SetAngularVelocity`) | world.rs `set_linear_velocity` / `set_angular_velocity` | `&mut self` step 相位独占;有限性 Rust 侧前置校验;不附带激活副作用(M66 注入/F-12 纪律);id 在册 |
| U49 | 位姿写入且不激活(`JPC_BodyInterface_SetPositionAndRotation` + `JPC_ACTIVATION_DONT_ACTIVATE`) | world.rs `set_position_rotation_dont_activate` | 同 U48;`SysTransform` POD 按值过境;DontActivate 禁止求解器激活副作用;M66 注入白名单入口 |
| U50 | 位姿+速度原子写入(`JPC_BodyInterface_SetPositionAndRotationAndVelocity`) | world.rs `set_position_rotation_and_velocity` | 同 U49;四元数/平移/速度均有限性校验;单 body 单锁窗口内连续调用,无中间可观测态泄漏 |
| U51 | `BodyLockWrite` 配对(铰链约束创建取 `Body*`) | world.rs `add_hinge_constraint` | `JPC_BodyLockWrite_new/delete` 严格配对;两 body 均 `Succeeded` 后才读 `GetBody`;锁内仅读 position/rotation 构造 `JPC_HingeConstraintSettings`;失败路径逐锁 delete,无泄漏 |
| U52 | 约束生命周期(`JPC_Constraint_AddRef` / `AddConstraint` / `RemoveConstraint` / `Release`) | world.rs `add_hinge_constraint` / `remove_constraint` / `Inner::drop` | Create 后 refcount=0;registry `AddRef` 持一份 + `AddConstraint` 经 `Ref<>` 再持一份;Remove/Drop 序 = `RemoveConstraint` 后 `Release`(双释放曾致堆损坏,已修);token 在册门禁 |
| U53 | 铰链 motor 面(`JPC_HingeConstraint_SetMotorState` / `SetTargetAngularVelocity`) | world.rs `set_hinge_motor` / `constraint_snapshot` | constraint token(u64) 在册;motor state u32 直映射 Jolt `MotorState`;snapshot 只读枚举已注册约束,不回写 Rust 可变别名 |

## 销毁纪律

`Inner::drop`(U34)为唯一销毁出口,固定序:摘除监听器 → 逐约束
`RemoveConstraint`+`Constraint_Release`(U52)→ `PhysicsSystem_delete`(连带
全部 body)→ 逐 body `JPC_Shape_Release`(U38)→ 过滤器/层接口 delete → job delete →
temp delete。创建失败路径由 `CreateGuard` 逆序兜底(U34)。body 个体移除经
`remove_bodies_batch`(U39)+ shape 引用单次释放(U38)。进程级 Jolt 注册不卸载(U33)。
Drop 无 panic(销毁调用均无失败返回;锁不中毒,U36)。

## 测试

- `cargo test -p rurix-physics-sys`:`ffi_layout_anchors`(60+ 编译期布局断言,U35)+
  in-crate 单测六面——世界创建/销毁(单/多线程 job + 非法描述确定性 Err)/ 球体重力下落
  沉降 / 批插 64 体 + 批移除 + 池耗尽·非法描述 Err / 射线命中球与地面(t 升序、法线朝上、
  layer_mask 过滤)/ 接触事件产生与 drain(Begin/Persist 关联 body 对、容量内零丢弃)/
  无效 token → `Err(InvalidBody)`(含移除后二次使用)。
- 验收门:`cargo clippy -p rurix-physics-sys --all-targets -- -D warnings` 零告警 +
  `cargo fmt --check -p rurix-physics-sys` 绿;G-G6-3 集成门见 milestones/g6/CI_GATES.md 步骤 88。

## §M125 追加段(2026-08-14):rurix-physics-sys56 —— Jolt 5.6 评估臂同构镜像登记

G9.6 M125(RXS-0377;RFC-0024 §4.E1 + RFC-0021 §4.A4 七步②)新建
`src/rurix-physics-sys56` crate(Jolt 5.6 评估臂 FFI 边界,**与 5.3 基线并存
不覆盖**;feature `jolt56` 默认 off,评估不升格生产默认)。**U 命名空间
0-byte——复用 U33~U53 既有审计边界**(G9.3 M105 device 腿复用 U54 先例):
本 crate 的 unsafe 面 = U33~U53 各条原语的**同构镜像**(同一 JoltC@2982004
FFI 面、同一 Rust 绑定结构、同一 SAFETY 不变量逐条成立),差异面 = 下列
5.6 delta,逐项登记:

| 镜像面 | 对应 5.3 登记 | 5.6 delta 与不变量维持 |
|---|---|---|
| 进程级 Jolt 初始化(`JPC56_RegisterDefaultAllocator`/`JPC56_FactoryInit`/`JPC56_RegisterTypes`,`Once` 一次注册、进程常驻) | U33 | 符号重命名 `JPC_→JPC56_`;**两线各自 `Once` 注册各自 namespace 的全局态**(5.3 `JPH` / 5.6 `JPH56` 为两套独立静态库全局符号,互不覆盖——符号隔离 dumpbin 实测);其余不变量逐字同 U33 |
| 句柄线性配对 create/delete + `CreateGuard` 逆序兜底 + `Inner::drop` 固定销毁序 | U34 | 同构;销毁序不变;5.6 线句柄为 JPH56 侧对象,与 5.3 线互不可见 |
| 手写 `extern "C"` 声明 + `#[repr(C)]` POD 镜像 + `ffi_layout_anchors` 编译期断言 | U35 | 函数集与签名同构(符号 `JPC56_` 前缀);**布局 delta 仅两件**:`JpcShapeCastSettings` 插入 `extra_convex_radius`(@32,尾部 32~35→36~39,size 48 不变)+ `JpcCollideShapeSettings` 追加 `internal_edge_removal_vertex_tolerance_sq`(@40,占 5.3 尾垫,size 48 不变)——数值 = `tools/layout_dump56.cpp` 对 vendored 5.6 头实测(2026-08-13,画像同 5.3 线);`CollisionEstimationResult`/`BodyManager_DrawSettings` 5.6 重排面 Rust 侧不镜像(safe 层不消费,C 侧由 LAYOUT_COMPATIBLE 锚定) |
| contact listener 回调(`Mutex` 收集、回调内不 panic、user_data 生命周期 ≥ 注册窗口) | U36 | 同构;5.6 回调面签名未变(JoltC 同一 commit) |
| 查询过滤器/收集器栈纪律 | U37 | 同构 |
| shape 引用计数配对 | U38 | 同构 |
| 批插 prepare/finalize + 批移除(逐 DestroyBody 循环) | U39 | 同构(JoltC `DestroyBodies` impl 上游注释 WIP 缺口在 5.6 线同样存在,处置 (c) 沿用) |
| `BodyLockRead` 配对(cast_ray 法线回填) | U40 | 同构 |
| `SysWorld` 的 `unsafe impl Send`/`Sync`(相位门) | U41 | 同构;两线 `SysWorld` 类型独立,相位门纪律逐字同 |
| `mem::zeroed` + `*_default` 初始化模式 | U42 | 同构;新增字段均经对应 `JPC56_*_default` 填上游默认(ExtraConvexRadius=0.0、InternalEdgeRemovalVertexToleranceSq=cDefault…)——**杜绝从尾垫读未定值的静默偏差**(5.6 适配补丁 #2 动因) |
| `BodyInterface` 速度只读/写入、位姿写入(DontActivate/原子)、`BodyLockWrite`、约束生命周期、铰链 motor 面 | U47~U53 | 同构;`JPC56_ConstraintSettings_default` 内经派生 shim 取默认值(5.6 起基类 ctor protected,补丁 #5;零行为变化) |

- crate lint 面同 5.3 线:`[lints.rust] unsafe_code = "allow"`(本 crate 块级豁免)
  + `undocumented_unsafe_blocks = "deny"`(每块 `// SAFETY:` 强制);`rurix-physics`
  (safe 层)与 `rurix-render` 维持 `#![forbid(unsafe_code)]` 0-byte。
- vendor pin/裁剪/重命名/五件适配补丁/构建画像/GPU compute 编译期排除(只评估
  不接权威结构性断言)全字段登记 = `src/rurix-physics-sys56/VENDOR56.md`;
  5.3 基线 vendor 与本文件既有条目 0-byte。
- 验收门:`cargo test -p rurix-physics-sys56`(ffi_layout_anchors 编译期断言 +
  in-crate 单测六面同构)+ `cargo clippy -p rurix-physics-sys56 --all-targets --
  -D warnings` 零告警;G9.6 集成门 = `ci/g9_jolt_56_ab_evaluation_smoke.py`
  (步骤 168,milestones/g9/CI_GATES.md §4A M125 行)。
