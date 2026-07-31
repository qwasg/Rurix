# VENDOR — JoltC + JoltPhysics(rurix-physics-sys)

> G6.2 PR-A 前置(RFC-0017 §4.C1):C-3 JoltC 缺口审计结论 + I-1 构建策略定案登记。
> 审计与 vendor 落盘日期:2026-07-31。

## 1. pin 与许可

| 仓库 | 角色 | 默认分支 | pin commit | 日期 | 许可 |
|---|---|---|---|---|---|
| [SecondHalfGames/JoltC](https://github.com/SecondHalfGames/JoltC) | C ABI 面 | `main`(非 master,克隆时注意) | `2982004387a9e36ca89525a87d983709d3666da7` | 2026-05-14 | MIT OR Apache-2.0 双许可(`LICENSE-MIT` + `LICENSE-APACHE`,均保留) |
| [jrouwe/JoltPhysics](https://github.com/jrouwe/JoltPhysics)(JoltC 递归 submodule) | 物理引擎本体 | submodule 钉住 | `0373ec0dd762e4bc2f6acdb08371ee84fa23c6db` | 2025-03-15 | MIT(`JoltPhysics/LICENSE`,保留) |

- **Jolt 版本核实(≥ 5.2 要求,C-5)**:vendored `JoltPhysics/Build/CMakeLists.txt` `project(JoltPhysics VERSION 5.3.0)` + `Jolt/Core/Core.h` `JPH_VERSION_MAJOR 5 / MINOR 3 / PATCH 0` → **Jolt 5.3.0**,满足 ≥ 5.2。
- vendor 副本相对上游的字节差异(仅两类,无功能性改动):
  1. **LF 归一 + 尾换行补齐**:上游两仓库为 CRLF 仓(494/505 文件带 CR),按仓库 `.gitattributes * -text` LF byte-exact 纪律对全部文本文件做 `CRLF → LF` 归一,并为 14 个上游缺尾换行的文本文件补齐尾换行;2 个二进制文件(`Build/Android/gradle/wrapper/gradle-wrapper.jar`、`Build/macOS/icon.icns`)保持上游字节不动。
  2. **裁剪**:JoltPhysics 仅保留构建所需 `Jolt/`、`Build/` 与 `LICENSE`、`README.md`;删去 `Assets/`(21M)、`Docs/`、`Samples/`、`UnitTests/`、`TestFramework/`、`JoltViewer/`、`PerformanceTest/`、`HelloWorld/` 等非构建面。裁剪安全性:`Build/CMakeLists.txt` 中 Samples/UnitTests/Viewer 等 target 均在 `CMAKE_CURRENT_SOURCE_DIR STREQUAL CMAKE_SOURCE_DIR` 守卫内(Jolt 作为子目录引入时恒假),不参与构建。JoltC 本体全量保留(含 `HelloWorld/`、`generate/`,体积极小),仅删 `.git`/`.gitmodules`。

## 2. 构建策略(I-1 定案:vendor 内联)

- **定案 = vendor 内联**(备选「外部 CMake 探测」未启用):两仓库源码 vendor 于 `vendor/JoltC/`(Jolt 在 `vendor/JoltC/JoltPhysics/`,保持 JoltC 的 submodule 相对路径布局),`build.rs` 经 `cmake` crate 内联构建静态库 `joltc` + `Jolt`,无外部依赖漂移。
- C++ 工具链画像(本机实测,2026-07-31):**VS2022 Community**(`C:\Program Files\Microsoft Visual Studio\2022\Community`)+ **cmake 4.3.0**;host = Windows 11 x64;Rust target = `x86_64-pc-windows-msvc`。CI default 档(= jolt)provisioning 责任 = 同画像。
- cmake 配置关键项(`build.rs` 固化,逐条理由):
  - `USE_STATIC_MSVC_RUNTIME_LIBRARY=OFF` — 上游默认 ON(/MT);Rust MSVC target 默认动态 CRT(/MD),混链必 LNK2038,强制 OFF 对齐。
  - `INTERPROCEDURAL_OPTIMIZATION=OFF` — 上游默认 ON(/GL+/LTCG),构建时间数倍膨胀且对链接器敏感;底座首版关闭(性能数字不进硬门,P-09)。
  - `DOUBLE_PRECISION=OFF`(默认)、`OBJECT_LAYER_BITS=16`(默认)、`CROSS_PLATFORM_DETERMINISTIC=OFF`(默认)。
  - 构建 target = `install`(默认),产物 `<dst>/lib/joltc.lib` + `<dst>/lib/Jolt.lib`;C++ 侧固定 Release 配置(Jolt Debug 配置对单测不可用地慢;Release CRT 与 Rust debug 二进制兼容)。
  - `CMAKE_POLICY_VERSION_MINIMUM=3.5` — cmake 4.x 移除 < 3.5 兼容的防御钉(JoltC 要求 3.16 / Jolt 要求 3.20,当前不触发,钉住防上游未来变动)。
- **确定性口径登记(§4.0-4)**:采用 (a) 默认口径 = 同二进制同平台重放逐位一致;可选口径 (b) `CROSS_PLATFORM_DETERMINISTIC=ON` 本切片**不启用**(MSVC 下该选项把 `/fp:fast` 换 `/fp:precise`,属独立构建画像,留待后续波次按需启用并写 evidence)。
- **object layer 位宽**:`OBJECT_LAYER_BITS=16`(Jolt `JPH_OBJECT_LAYER_BITS` 与 JoltC `JPC_OBJECT_LAYER_BITS` 一致,`ObjectLayer = uint16`);`layer_count` 上限 = **65535**(`0xFFFF` 为 Jolt 保留 `cObjectLayerInvalid`,sys 层 `create` 校验 > 65535 → `Err(InvalidDesc)`)。broadphase layer 位宽 8(Jolt 固定),本切片仅用 1 个 broadphase layer(见 §3 处置)。

## 3. C-3 缺口审计(七面函数清单,基于 pin commit 实际头文件 `JoltC/Functions.h` / `Enums.h`)

| 面 | 结论 | 可用函数(pin 版本实测) | 缺口与处置 |
|---|---|---|---|
| contact listener | ✅ 全覆盖 | `JPC_ContactListener_new(user_data, JPC_ContactListenerFns{OnContactValidate, OnContactAdded, OnContactPersisted, OnContactRemoved})` + `JPC_ContactListener_delete` + `JPC_PhysicsSystem_SetContactListener`;manifold 经 `JPC_ContactManifold`(build 期 `LAYOUT_COMPATIBLE` 静态断言对齐 JPH) | **子缺口**:回调不含求解后 impulse。处置 **(c) 收窄首版范围**:`SysContactEvent.impulse` 首版恒 0.0,Begin/Persist/End 相位与点/法线完整;升级路径 = `JPC_EstimateCollisionResponse` 逐回调估算(成本敏感,后置) |
| body activation listener | ❌ 缺口 | JoltC 无 `JPC_BodyActivationListener` C 面 | 处置 **(c) 收窄首版范围**:不注册激活监听器;`slept_this_step`/`active_bodies` 由 step 前后对受管 body 轮询 `JPC_BodyInterface_IsActive` 差分得到(契约面无激活事件,无损;`is_active` 查询不受影响) |
| broadphase layer interface | ✅ 全覆盖 | `JPC_BroadPhaseLayerInterface_new/delete`(GetNumBroadPhaseLayers/GetBroadPhaseLayer 回调)+ `JPC_ObjectVsBroadPhaseLayerFilter_new` + `JPC_ObjectLayerPairFilter_new` + `JPC_BroadPhaseLayerFilter_new` + `JPC_ObjectLayerFilter_new` + `JPC_BodyFilter_new` | 处置 **(c) 收窄首版范围**:首版**单 broadphase layer**(全部 object layer → BP layer 0,ObjectVs/ObjectLayerPair 过滤恒 true);正确性不受影响(Jolt 显式支持单树),moving/non-moving 双树优化后置。layer 碰撞对过滤本就不在冻结接口 |
| job system | ✅ 全覆盖 | `JPC_JobSystemThreadPool_new2/new3/delete`(new3 可指定线程数,`job_threads==0` → -1 = 硬件并行度)+ `JPC_JobSystemSingleThreaded_new/delete` | 无。单线程/多线程均可用(本切片实现走 ThreadPool,numThreads 显式映射) |
| shape cast | ✅ 全覆盖 | `JPC_NarrowPhaseQuery_CastShape` + `JPC_CastShapeCollector_new`(Rust 侧全命中 collector 回调)+ `JPC_ShapeCastSettings_default`;overlap 同型 `JPC_NarrowPhaseQuery_CollideShape` + `JPC_CollideShapeCollector_new` + `JPC_CollideShapeSettings_default` | 无 |
| CCD(MotionQuality) | ✅ 全覆盖 | `JPC_MotionQuality`(DISCRETE=0/LINEAR_CAST=1,`ENSURE_ENUM_EQ` 对齐 `JPH::EMotionQuality`)+ `JPC_BodyCreationSettings.MotionQuality` 字段 + `JPC_BodyInterface_SetMotionQuality/GetMotionQuality` | 无。`ccd: bool` → `LinearCast`/`Discrete` 直映射 |
| batch add(AddBodiesPrepare/Finalize) | ✅ 全覆盖 | `JPC_BodyInterface_AddBodiesPrepare`(返回 `void* AddState`)/`AddBodiesFinalize`/`AddBodiesAbort` + `JPC_BodyInterface_RemoveBodies`/`DestroyBodies` | 无。批插映射 prepare(step 外交替期)→ finalize 单点提交,失败路径 `AddBodiesAbort`(P-01 不泄漏) |
| (计划外)ray cast 全命中 | ⚠️ 部分缺口 | `JPC_NarrowPhaseQuery_CastRay` = **仅最近命中**(impl 内 `ClosestHitCollisionCollector`,实测 `JoltCImpl/JoltC.cpp:2893`);四组 filter 均可空(空 = 默认全通过) | 处置 **(c) 收窄实现路线**:Rust 侧排除循环 — 每轮 cast 取最近命中,把已命中 body 经 `JPC_BodyFilter` 回调排除后再 cast,直至无命中;零 C++ 补丁,契约「全命中返回(顺序未规范化)」保持。首轮 t 映射 `origin+dir*t_min` / `dir*(t_max-t_min)` |
| (计划外)batch destroy | ⚠️ 部分缺口 | `JPC_BodyInterface_DestroyBodies` 头有声明但 impl 被上游注释(实测 `JoltCImpl/JoltC.cpp:2580-2581` 注释块,WIP 缺口);`RemoveBodies` 与单体 `DestroyBody` 均可用 | 处置 **(c) 收窄实现路线**:批移除 = `RemoveBodies`(批量)+ 逐 `DestroyBody` 循环(Jolt `DestroyBodies` 语义等价,零 C++ 补丁) |

**总处置**:七面无 vendor 补丁、无转 amer-koleci/joltc;五处缺口全部走 (c) 收窄并已在 §4.A5 impulse 语义、§4.A7 睡眠统计、broadphase 优化面、查询与批移除实现路线上确认无损冻结契约。

## 4. 回调与 ABI 关键事实(实现依据)

- 过滤器/监听器构造签名统一为 `JPC_Xxx_new(const void *self, JPC_XxxFns fns)`,`self` 原样回传回调首参(实测 `JoltCImpl/JoltC.cpp` impl);`CastRay`/`CastShape`/`CollideShape` 的 filter 指针传 `nullptr` = 默认全通过。
- `JPC_ContactListener` 回调在 `PhysicsSystem::Update` 内由 **Jolt job 线程多线程触发**(顺序非确定)——Rust 侧事件收集进 `Mutex<Vec>`,归一化排序在 safe 层(§4.A5 C-2);回调内不 panic(FFI 边界)。
- `HandleShapeResult`(JoltCImpl):`*_ShapeSettings_Create` 成功时 shape 引用计数 = 1,调用方持有;Jolt `Body` **不**对 shape AddRef(实测 `Body::SetShapeInternal`)→ sys 层为每个 body 持有其 shape 引用,body 销毁后 `JPC_Shape_Release`。
- `BodyCreationSettings.Position` = 体坐标系原点(非质心,`BodyCreationSettings.h:80` 注释);`BodyInterface_GetPosition` 返回同系原点(`Body.h:268` `mPosition - mRotation * COM`)→ 变换 round-trip 一致。
- 布局可信链:JoltC 头内 C 结构 ↔ JPH C++ 结构由 `JoltCImpl/JoltC.cpp` 的 `LAYOUT_COMPATIBLE` 静态断言在 **vendor 构建期**强制对齐;Rust 侧 `#[repr(C)]` 镜像 ↔ JoltC C 结构由本 crate `ffi_layout_anchors` 单测锚定(U32 模式,数值 = x86_64 单精度画像实测;复测程序 `tools/layout_dump.cpp`:`cl /std:c++17 /I vendor/JoltC tools/layout_dump.cpp` 打印 vendored 头 `offsetof`,pin 或画像变更时重测)。
