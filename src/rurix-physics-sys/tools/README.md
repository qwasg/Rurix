# rurix-physics-sys / tools —— JoltC ABI 布局复测探针

本目录放**手工编译的一次性布局真值测量程序**,唯一用途是为 `src/ffi.rs` 的
`ffi_layout_anchors` 单测提供 `sizeof / alignof / offsetof` 实测数值。
非构建产物、不参与 `cargo build`(`build.rs` 不引用),也不是 CI 门。
二进制(`*.exe` / `*.obj`)不入库,仅源文件入库(见根 `.gitignore`)。

画像(全部探针共用):`x86_64-pc-windows-msvc` / 单精度(`DOUBLE_PRECISION=OFF`)/
`OBJECT_LAYER_BITS=16`。pin 或画像变更时重测,详见 `../VENDOR.md` §4「布局可信链」。

## 编译与运行

```
cl /std:c++17 /I vendor/JoltC tools/<probe>.cpp
```

在 crate 根(`src/rurix-physics-sys/`)执行;`ENSURE_*` 宏在无 `ENSURE_TESTS` 时为空,
故只需 vendored JoltC 公共头,无需链接 Jolt 静态库。

## 探针清单

| 探针 | 引入 | 覆盖面 | 状态 |
| --- | --- | --- | --- |
| `layout_dump.cpp` | G6.2 PR-A | Vec/Mat/查询 args/CastResult/Shape 及 Body 主干 | 已消费 |
| `layout_hinge.cpp` | G8 M66 | `JPC_ConstraintSettings` / `JPC_SpringSettings` / `JPC_MotorSettings` / `JPC_HingeConstraintSettings` 主干 | 已消费 |
| `layout_hinge2.cpp` | G8 M66 | Hinge 轴/极限/极限弹簧 + ConstraintSettings 与 SpringSettings 余下字段 | 已消费 |

## 结论(2026-08-06 实测,已落 `ffi_layout_anchors`)

`layout_hinge.cpp`:

```
ConstraintSettings 32 align 8   Enabled 0   Priority 4   UserData 24
SpringSettings 12               MotorSettings 28
HingeConstraintSettings 208 align 16
  Space 32   Point1 48   Point2 96   LimitsMin 144   MotorSettings 168
```

`layout_hinge2.cpp`:

```
HingeAxis1 64   NormalAxis1 80   HingeAxis2 112   NormalAxis2 128
LimitsMax 148   LimitsSpring 152   MaxFriction 164
ConstraintSettings: DrawConstraintSize 16   NumVelocityStepsOverride 8   NumPositionStepsOverride 12
SpringSettings:     Mode 0   FrequencyOrStiffness 4   Damping 8
```

两段数值逐字进入 `src/ffi.rs`:`JpcConstraintSettings`(32B/align 8)、
`JpcSpringSettings`(12B)、`JpcMotorSettings`(28B)、
`JpcHingeConstraintSettings`(208B/align 16,含 `_pad_space` 36→48 与 `_pad_end` 196→208),
并由 `ffi_layout_anchors` 的 `size_of` / `align_of` / `offset_of!` 断言在编译期锚定。
探针与 `ffi.rs` 断言任一侧漂移即为 vendor pin 或画像变更信号,须按上表重测。
