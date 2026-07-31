# unsafe-audit: rurix-physics-sys(JoltC C API FFI 边界)

> 注册依据:AGENTS.md 硬规则 9 / 10 §7.6(无注册条目的 unsafe 是 CI 错误);
> 14 §2 常驻集 unsafe-audit 完整性。G6.2 PR-A 激活(RFC-0017 §4.C FFI 与 unsafe
> 纪律章,Agent Approved 2026-07-31;R-G6-1 裁决:自维护薄 FFI 子 crate 绑定
> SecondHalfGames/JoltC C API,rolt/jolt-rust 停滞否决)。
> 编号:number_ledger v1.28 `reserved_in_flight[G6]` claim——**U33 起续号**(main
> on_tree_max U32,next_free 33;U29 = EA1 预留显式跳让不回收)。
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

## 销毁纪律

`Inner::drop`(U34)为唯一销毁出口,固定序:摘除监听器 → `PhysicsSystem_delete`(连带
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
