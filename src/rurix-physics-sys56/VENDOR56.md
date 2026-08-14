# VENDOR56 — JoltC + JoltPhysics v5.6.0(rurix-physics-sys56,Jolt 5.6 评估臂)

> G9.6 M125(RXS-0377;RFC-0024 §4.E1 + RFC-0021 §4.A4 七步程序第②步):Jolt
> 5.3→5.6 升级 A/B 的 **5.6 独立 vendor 线**——与 5.3 基线
> `src/rurix-physics-sys/vendor/JoltC` **并存不覆盖**(5.3 基线 VENDOR.md pin
> 字面 0-byte;覆盖/替换注入即 RED,门 `g9.p1.m125.jolt_56_ab_evaluation`)。
> vendor 落盘与构建验证日期:2026-08-13/14。

## 1. pin 与许可

| 仓库 | 角色 | pin | 日期 | 许可 |
|---|---|---|---|---|
| [SecondHalfGames/JoltC](https://github.com/SecondHalfGames/JoltC) | C ABI 面 | `2982004387a9e36ca89525a87d983709d3666da7`(main,与 5.3 基线**同一 commit**) | 2026-05-14 | MIT OR Apache-2.0 双许可(`LICENSE-MIT` + `LICENSE-APACHE`,均保留) |
| [jrouwe/JoltPhysics](https://github.com/jrouwe/JoltPhysics) | 物理引擎本体 | tag `v5.6.0` = commit `e77f175595e64cb44218cc9d9d56fc365ad0e36a` | 2025-07-11 | MIT(`JoltPhysics/LICENSE`,保留) |

- **JoltC 上游未跟进 5.6 的处置登记(重要)**:JoltC main HEAD(2982004)钉住的
  JoltPhysics submodule 仍为 5.3.0 线;上游无「对应 5.6 的 JoltC 版本」。本 vendor
  线裁定 = **JoltC@2982004(与 5.3 基线同一 commit)+ JoltPhysics v5.6.0 替换其
  submodule 内容 + 最小 5.6 适配补丁集(§3 五件)**——两臂 JoltC 包裹层同源,
  A/B 变量唯一化为 Jolt 引擎本体(5.3.0 ↔ 5.6.0),消除包裹层差异干扰。
- **Jolt 版本核实**:vendored `JoltPhysics/Build/CMakeLists.txt`
  `project(JoltPhysics VERSION 5.6.0)` + `Jolt/Core/Core.h` `JPH_VERSION_MAJOR 5 /
  MINOR 6 / PATCH 0` → **Jolt 5.6.0**(5.3→5.6 评估目标版,实测定案 tag v5.6.0)。
- vendor 副本相对上游的字节差异(三类,无功能性改动 + 一类机械重命名):
  1. **LF 归一 + 尾换行补齐**:上游两仓库为 CRLF 仓,按仓库 `.gitattributes
     * -text` LF byte-exact 纪律对全部文本文件做 `CRLF → LF` 归一,并为缺尾换行
     的文本文件补齐(本批实测 11 件);二进制文件 36 件(`Build/Android/gradle/
     wrapper/gradle-wrapper.jar`、`Build/macOS/icon.icns`、`Jolt/Shaders/*.{dxil,spv}`
     34 件预编译 GPU 毛发/测试 shader)保持上游字节不动。
  2. **裁剪**:JoltPhysics 仅保留构建所需 `Jolt/`、`Build/` 与 `LICENSE`、
     `README.md`;删去 `Assets/`、`Docs/`、`Samples/`、`UnitTests/`、
     `TestFramework/`、`JoltViewer/`、`PerformanceTest/`、`HelloWorld/` 等非构建面
     (裁剪安全性:`Build/CMakeLists.txt` 中 Samples/UnitTests/Viewer 等 target 均在
     `CMAKE_CURRENT_SOURCE_DIR STREQUAL CMAKE_SOURCE_DIR` 守卫内,作为子目录引入
     时恒假)。JoltC 本体全量保留(含 `HelloWorld/`、`generate/`),仅删
     `.git`/`.gitmodules`/`.github`/根 `.editorconfig`/根 `.gitignore`(与 5.3
     基线 vendor 文件集逐项一致)。
  3. **机械符号重命名(同进程并存前提,零功能改动)**:全部文本源码经词边界
     标识符重命名 `JPC_` → `JPC56_`(JoltC C API 全量函数/类型/宏)+
     `namespace JPH` → `namespace JPH56` / `JPH::` → `JPH56::`(Jolt 引擎全量
     C++ 符号)——5.3 基线静态库(`JPC_*`/`JPH::*`)与本线(`JPC56_*`/`JPH56::*`)
     同进程链接零符号冲突(dumpbin 实测:`joltc.lib` 导出全 `JPC56_` 前缀,
     `JPH56::` 命名空间 mangling 与 5.3 `JPH::` 全分离)。重命名为纯标识符替换,
     不改变任何函数语义/调用关系;LICENSE 文件不参与重命名。

## 2. 构建策略(沿 5.3 线 I-1 定案:vendor 内联)

- **定案 = vendor 内联**(与 5.3 线同构):两仓库源码 vendor 于
  `src/rurix-physics-sys56/vendor/JoltC/`(Jolt 在 `vendor/JoltC/JoltPhysics/`,
  保持 JoltC 的 submodule 相对路径布局),`build.rs` 经 `cmake` crate 内联构建
  静态库 `joltc` + `Jolt`(库文件名与 5.3 线相同,各自 OUT_DIR 隔离;链接符号
  全分离,§1-3),无外部依赖漂移。
- C++ 工具链画像(本机实测,2026-08-13):**VS2022 Community** + **cmake 4.3.0**;
  host = Windows 11 x64;Rust target = `x86_64-pc-windows-msvc`(与 5.3 线画像逐项
  一致——A/B 同画像前提)。
- cmake 配置关键项(`build.rs` 固化;与 5.3 线逐项一致 + GPU compute 四开关):
  - `USE_STATIC_MSVC_RUNTIME_LIBRARY=OFF` / `INTERPROCEDURAL_OPTIMIZATION=OFF` /
    `CMAKE_POLICY_VERSION_MINIMUM=3.5` / `DOUBLE_PRECISION=OFF` /
    `OBJECT_LAYER_BITS=16` / `CROSS_PLATFORM_DETERMINISTIC=OFF` — 逐条理由与 5.3
    线 VENDOR.md §2 字面同构(/MD 对齐、构建时间、防御钉、单精度 16 位层、
    确定性口径 (a) 同二进制同平台逐位)。
  - **`JPH_USE_DX12=OFF` / `JPH_USE_VK=OFF` / `JPH_USE_MTL=OFF` /
    `JPH_USE_CPU_COMPUTE=OFF`** — **GPU compute 只评估不接权威(RXS-0377 L4)
    的结构性断言**:Jolt 5.6 新增 GPU compute shader 接口(`Jolt/Compute/**`
    DX12/Vulkan/Metal/CPU 四实现)+ GPU strand 毛发(`Jolt/Shaders/**`,
    Cosserat 杆,上游自标 work-in-progress)在本 vendor 构建中**编译期整体
    排除**(`Jolt/Jolt.cmake` 源清单全部门控于这四开关),接口在本进程结构性
    不可达;评估报告留档(§4),接入须 RD-043 + 矩阵 §12 + 独立 Full RFC(GPU
    主刚体禁止线 0-byte,RD-043 观察维持)。
  - 构建 target = `install`(默认),产物 `<dst>/lib/joltc.lib` +
    `<dst>/lib/Jolt.lib`;C++ 侧固定 Release(与 5.3 线同画像理由)。
- **并存断言**:feature `jolt`(5.3)+ feature `jolt56`(本线)同开时,两后端
  同进程各自实例化(`BackendKind::Jolt` 与 `BackendKind::Jolt56` 各自
  `PhysicsWorld::new` + step;链接成功即符号隔离证明,rurix-physics `ab_eval`
  单测锚);5.3 基线默认后端面(`default = ["jolt"]`)0-byte 不动。
- **确定性口径**:与 5.3 线同一口径 (a)(同二进制同平台重放逐位一致;
  `CROSS_PLATFORM_DETERMINISTIC=OFF` 不启用)。双臂 determinism 画像逐项一致
  (固定 dt 1/60 锁死、单线程 ThreadPool(1)、睡眠策略钉值、零 IO)。

## 3. 5.6 适配补丁集(五件;上游 JoltC@2982004 → Jolt v5.6.0 端口)

上游 JoltC 静态断言(`Functions.h` ENSURE_* / `JoltC.cpp` `LAYOUT_COMPATIBLE`,
vendor 构建期强制 C ↔ C++ 布局对齐)实测枚举出 5.3→5.6 全部布局漂移;本补丁集
= 对该清单的逐项适配,**不改任何求解/查询语义**:

| # | 文件 | 补丁 | 上游 5.6 变更面 |
|---|---|---|---|
| 1 | `JoltC/Functions.h` | `JPC56_ShapeCastSettings` 插入 `float ExtraConvexRadius`(@32,尾部四字段 32~35→36~39,size 48 不变) | v5.6.0 新增 `ShapeCastSettings::mExtraConvexRadius`(query shape 额外凸半径膨胀) |
| 2 | `JoltC/Functions.h` | `JPC56_CollideShapeSettings` 追加 `float InternalEdgeRemovalVertexToleranceSq`(@40,占 5.3 尾垫,size 48 不变) | v5.6.0 新增 `CollideShapeSettings::mInternalEdgeRemovalVertexToleranceSq`(内部边去除顶点容差可配置;上游 ENSURE_SIZE 因尾垫吸收未触发——**不经补丁则 C 侧从尾垫读 0.0 偏离上游默认**,本补丁消除该静默偏差) |
| 3 | `JoltC/Functions.h` | `JPC56_BodyManager_DrawSettings` 插入 strand-hair 调试三字段(`mDrawSoftBodyRods/RodStates/RodBendTwistConstraints`) | v5.6.0 GPU 毛发调试绘制字段(debug-only,safe 层不消费) |
| 4 | `JoltC/Functions.h` | `JPC56_CollisionEstimationResult` 重排:删逐点 `JPC_Impulse{Contact,Friction1,Friction2}`,改聚合摩擦字段(`FrictionPoint/Tangent1/Tangent2` + `FrictionImpulse1/2` + `AngularFrictionImpulse`)+ 逐点 `float Impulses[64]`(size 384 align 16) | **v5.6.0 新摩擦模型(平均接触点)的 ABI 印记**(摩擦不再逐点施加;详见 §4 摩擦模型专项;safe 层不消费本 API,impulse 恒 0 收窄登记维持) |
| 5 | `JoltCImpl/JoltC.cpp` | `JPC56_ConstraintSettings_default` 内经派生 shim(`struct JPCConstraintSettingsDefaults : JPH56::ConstraintSettings {}`)取默认值 | v5.6.0 起 `ConstraintSettings` 基类 ctor protected(`Constraint.h`;零行为变化) |

- 补丁外零改动:`JoltC` 与 `JoltPhysics` 其余源码相对上游 = LF 归一 + 机械
  重命名(§1),无第二处功能面接触。
- 布局可信链:C 结构 ↔ JPH56 C++ 结构由 `JoltCImpl/JoltC.cpp` 的
  `LAYOUT_COMPATIBLE` 静态断言在 **vendor 构建期**强制对齐(五件补丁落盘后
  全量断言绿,2026-08-13 实测);Rust 侧 `#[repr(C)]` 镜像 ↔ JoltC C 结构由
  本 crate `ffi_layout_anchors` 单测锚定(数值 = `tools/layout_dump56.cpp`
  对 vendored 5.6 头 `offsetof` 实测,2026-08-13,x86_64-pc-windows-msvc /
  单精度 / OBJECT_LAYER_BITS=16 画像;pin 或画像变更时重测——探针源码入库
  `tools/`,RXS-0377 L5 检查单固定项)。
- 5.3 线 C-3 七面缺口审计结论在本线**逐项沿用**(同一 JoltC commit:contact
  listener impulse 恒 0 收窄、无 activation listener 轮询差分、单 broadphase
  layer、CastRay 仅最近命中 → Rust 排除循环、DestroyBodies WIP → 逐
  DestroyBody 循环;五处收窄对双臂同构,A/B 无包裹层差异变量)。

## 4. 5.6 特性处置登记(RFC-0024 §4.E1 分项表消费面)

| 5.6 特性 | 处置(本线落地) |
|---|---|
| **新摩擦模型(平均接触点)**(上游 v5.6.0 release notes:Pyramid 测试快 15%/省 40% 内存/消除首接触点序偏向;摩擦 = 2 线性约束〔上限 μ·Σ(contact_impulse)〕+ 1 角约束〔上限 μ·Σ(distance·contact_impulse)〕) | **A/B 重点项已实测**:canonical 场景(堆叠 4 层 + 滑块初速摩擦减速直射)双臂同输入 digest 逐位相等;跨版本偏差画像与逐字段 exact/tolerance/invariant 分类实测落 `milestones/g9/g9_m125_jolt56_ab.json`(contact_events exact;translation/rotation/linvel/angvel tolerance;world_chain invariant 分叉如实记录——求解器语义变化预期);ABI 印记 = 补丁 #4 |
| **GPU compute shader 接口**(DX12/Vulkan/Metal + CPU 参考;四 cmake 开关) | **只评估不接权威**:四开关 OFF 编译期整体排除(§2 结构性断言);JoltC C 面零 GPU 导出(机核);接权威提案一律 fail-closed typed Err(`connect_gpu_compute_authority`);接入须 RD-043 + 矩阵 §12 + 独立 Full RFC;评估留档 = 本节 + measured 报告 `gpu_compute` 段 |
| GPU strand 毛发(Cosserat 杆,WIP) | 非权威装饰副轨候选登记(async-decorative 通道维持 M123 判档门前不启用);随 GPU compute 四开关一并编译期排除 |
| HeightField 16bit(`mBitsPerSample` ≤16) | 与流送/地形页联动评估,独立分项判档(本批不消费) |
| glTF `KHR_physics_rigid_bodies` 马达(`ESpringMode::MassNormalizedStiffnessAndDamping` / `EMotorState::PositionAndVelocity`) | 资产管线候选增强,进 RFC-0020 面(本批不消费) |
| `Ragdoll::DriveToPoseUsingMotors`(位置+速度双通道) | 与 G8 M69 约束五件套路线对照评估;采纳需 JoltC C 面审计(当前 pin 无该导出,本批不消费) |
| `Body::ApplyBodyCreationSettings`/`ApplySoftBodyCreationSettings` | 评估留档(本批不消费) |
| 内部边去除容差可配置(`CollideShapeSettings::mInternalEdgeRemovalVertexToleranceSq` + `PhysicsSettings::mInternalEdgeRemovalVertexToleranceSq`) | C 面字段经补丁 #2 对齐上游默认(`cDefaultInternalEdgeRemovalVertexToleranceSq`);safe 层不覆写 |
| **layout 探针工具化** | `tools/layout_dump56.cpp` 入库(全量消费面 + 所有 `*Settings` sizeof/offsetof 重测);数值进 `src/ffi.rs` `ffi_layout_anchors` 编译期断言;二进制产物不入库 |

## 5. 与 5.3 基线的边界(独立并存纪律)

- 5.3 基线 `src/rurix-physics-sys/`(含其 vendor/VENDOR.md/build.rs/全部源码)
  **0-byte 不动**;本线为独立 crate `src/rurix-physics-sys56/`。
- 生产默认 = `BackendKind::Jolt`(5.3,feature `jolt` 默认 on)0-byte;本线
  `BackendKind::Jolt56`(feature `jolt56` 默认 off)= **评估用途,不升格生产
  默认**(采纳须⑦程序:corpus 显式迁移保留 5.3 基线 artifact + replay 门新
  版本重跑落 evidence + 判据字面经修订后才改版本号——本评估三件
  not-triggered 登记,verdict = `maintain_5_3_default`)。
- unsafe 注册:本 crate 为物理 FFI unsafe 集中地之二,`U33~U53` 审计模式
  同构镜像 + 本文件 §3 五件 delta 登记于 `unsafe-audit/rurix-physics-sys.md`
  §M125 追加段(U 命名空间 0-byte,复用既有审计边界)。
