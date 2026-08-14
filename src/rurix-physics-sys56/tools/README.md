# rurix-physics-sys56 / tools —— JoltC(5.6 线)ABI 布局复测探针

本目录放 **Jolt 5.6 评估臂**的布局真值测量程序(RXS-0377 L5「layout 探针工具化」:
所有消费面 `*Settings` 结构 sizeof/offsetof 静态断言重跑纳入 vendor 升级/新 FFI
检查单固定项;探针源码入库,不再散落工作树),唯一用途是为 `src/ffi.rs` 的
`ffi_layout_anchors` 单测提供 `sizeof / alignof / offsetof` 实测数值。
非构建产物、不参与 `cargo build`(`build.rs` 不引用),也不是 CI 门。
二进制(`*.exe` / `*.obj`)不入库,仅源文件入库(见根 `.gitignore`)。

画像:`x86_64-pc-windows-msvc` / 单精度(`DOUBLE_PRECISION=OFF`)/
`OBJECT_LAYER_BITS=16`(与 5.3 线逐项一致)。pin 或画像变更时重测,详见
`../VENDOR56.md` §2「布局可信链」。

## 编译与运行

```
cl /std:c++17 /I vendor/JoltC tools/layout_dump56.cpp
```

在 crate 根(`src/rurix-physics-sys56/`)执行;`ENSURE_*` 宏在无 `ENSURE_TESTS` 时为空,
故只需 vendored JoltC(5.6 线)公共头,无需链接 Jolt 静态库。

## 探针清单

| 探针 | 引入 | 覆盖面 | 状态 |
| --- | --- | --- | --- |
| `layout_dump56.cpp` | G9.6 M125 | 5.3 线 `layout_dump.cpp` + `layout_hinge.cpp` + `layout_hinge2.cpp` 三面合并(全量消费结构 + 所有 `*Settings`)+ 5.6 delta 四面(`ShapeCastSettings.ExtraConvexRadius` / `CollideShapeSettings.InternalEdgeRemovalVertexToleranceSq` / `CollisionEstimationResult` 新摩擦模型聚合重排 / `BodyManager_DrawSettings` strand-hair 三字段) | 已消费(2026-08-13 实测进 `ffi_layout_anchors`) |

## 结论(2026-08-13 实测,已落 `ffi_layout_anchors`;相对 5.3 线的 delta)

```
ShapeCastSettings 48 align 16(不变):ExtraConvexRadius 32(5.6 新增),
  BackFaceModeTriangles/Convex 32/33 → 36/37,UseShrunken/ReturnDeepest 34/35 → 38/39
CollideShapeSettings 48 align 16(不变):InternalEdgeRemovalVertexToleranceSq 40(5.6 新增,占 5.3 尾垫)
CollisionEstimationResult 384 align 16(5.6 新摩擦模型聚合重排;Rust 侧不镜像):
  FrictionPoint 64 / Tangent1 80 / Tangent2 96 / FrictionImpulse1 112 /
  FrictionImpulse2 116 / AngularFrictionImpulse 120 / NumImpulses 124 / Impulses 128
BodyManager_DrawSettings 36 align 4(Rust 侧不镜像):mDrawSoftBodyRods 25 /
  RodStates 26 / RodBendTwistConstraints 27(5.6 新增)
ConstraintSettings 32 align 8 / SpringSettings 12 / MotorSettings 28 /
  HingeConstraintSettings 208 align 16(全字段 offset 与 5.3 线逐字一致——
  5.6 仅 ctor 转 protected,vendor56 适配补丁 #5 派生 shim)
```

其余结构与 5.3 线实测逐字一致(Vec/Mat/查询 args/CastResult/Body 主干/形状设置/
函数表/标量宽度全量重测零漂移)。探针与 `ffi.rs` 断言任一侧漂移即为 vendor pin
或画像变更信号,须按上表重测。
