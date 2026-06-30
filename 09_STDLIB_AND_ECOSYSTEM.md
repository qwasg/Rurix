# 09 — 标准库与生态

> 所属文档集：[00_MASTER_INDEX.md](00_MASTER_INDEX.md)
> 版本：v1.0（2026-06-11）
> 主要输入：r12（标准库与 CUDA 绑定）、r8（包管理与供应链）、r6（再分发合规）
> 关联决策：D-301 ~ D-313（见 [13](13_DECISION_LOG.md)）

---

## 1. 分层结构（D-302）

```
core      无 OS 依赖：原生类型操作、Option/Result、迭代器核心、Vec/Mat 数学、views
          （host/device 双侧可用的最大公共子集）
std       host-only：堆分配、集合、字符串、IO、文件、线程、时间
gpu       运行时绑定层：Device/Context/Stream/Buffer 家族/launch/telemetry
          （即 rurix-rt 的语言面，[08](08_RUNTIME_AND_TOOLING.md)）
ecosystem 外部包：cuBLAS 绑定、图像编解码、几何库、ndarray、cuDNN……
          （明确不进标准库的清单见 §9）
```

裁剪哲学：标准库做"语言能力的最小完备载体"，领域能力全部进生态包（红线 4，[03](03_POSITIONING_AND_LANDSCAPE.md) §4）。Rust 的"std 小而稳 + 生态繁荣"路线，反面教材是 Python std 的腐化区。

## 2. core：数值与基础

- 原生类型方法集（`i32::max` 等）、位操作、checked/wrapping/saturating 算术全family。
- `Option/Result` + 组合子；迭代器 trait 核心（device 侧可用其无分配子集）。
- f16/bf16 的转换与算术（device 原生、host 软件实现起步）。
- **无**：字符串处理进 core（device 无意义）、collections 进 core。

## 3. 数学库：Vec/Mat（语言内建，库面在 core，D-301）

r12 结论照搬并 reconcile：

- `Vec2/3/4<T>`、`Mat2/3/4<T>`（T ∈ {f16, f32, f64, i32, u32, bool}）；swizzle 语法（`v.xyz`、`v.xxyy`）编译器支持。
- 布局保证：`Vec4<f32>` 16 字节对齐；`Vec3<f32>` 提供 `Vec3A`（16B padding）变体（glam 先例）；布局是 spec 承诺（FFI/图形互操作依赖）。
- **列主序 + 列向量（`v' = M v`）为 canonical**（Eigen/glam/cuBLAS/LAPACK 惯例，r12）；图形互操作显式提供 `Mat4RowMajor` 转换类型而不是布局开关（Slang 社区的隐式转置 bug 教训——正交矩阵转置肉眼难察，r12 标注的张力点以"单一 canonical + 显式转换"裁决，D-303）。
- 几何原语（G0 需要）：`Ray`、`Aabb`、`Plane`、四元数 `Quat`、变换 `Transform3`——进 core 数学模块（软光栅与仿真的公共依赖）。
- SoA 支持：`derive(Record)` 自动生成 SoA 视图类型（GPU 批量场景 SoA 优于 AoS——warp coalesce 1–2 次事务 vs 32 次，r12）；AoS 仍是默认（固定小维度合法）。
- fast-math 纪律：库提供显式变体（`sin` / `sin_fast`），全局 `--fast-math` 只影响 libdevice NVVMReflect 分支；**库/中间包不得擅自启用 fast-math**（glam feature 先例，r12）。

## 4. gpu：Buffer 家族与运行时面（D-304）

[05](05_LANGUAGE_ARCHITECTURE.md) §4 与 [08](08_RUNTIME_AND_TOOLING.md) §2 的库面汇总：

- 统一 trait `Buffer<T: DeviceCopy>`；具体类型 `DeviceBuffer` / `PinnedBuffer` / `MappedBuffer`（opt-in）/ `ManagedBuffer`（opt-in + Windows 语义警示——r4/r12 的张力以"类型存在但非默认路径 + feature gate"reconcile，D-305）。
- `DeviceSlice` / `DeviceSliceMut`（host 侧对设备内存的区段句柄，类比 `&[T]`）；device 侧统一 views。
- FFI 边界：`DeviceBuffer` 不保证 FFI 安全；跨 C ABI 必须显式 `into_raw()/from_raw()` 转 `DevicePtr<T>`（RustaCUDA 教训，r12）。
- `BufferPool`（库级池化，分桶 next_power_of_two，r12 的 workspace 模式）——运行时不内置池化（P-05）。
- 并行基元：`reduce/scan/sort/histogram` 以 **Rurix 自研 kernel** 进 gpu 库（CUB/Thrust 是 C++ 模板库，FFI 不可行——r12 明确结论，D-306）。这些 kernel 同时是 views 表达力的 dogfood 与 L2 基准载体。

## 5. 生态包第一梯队（官方维护，MVP±）

| 包 | 内容 | 时点 |
|---|---|---|
| `cublas-sys` / `cublas` | 手写薄封装（**非 bindgen 全量**，r12）：GEMM/GEMV 起步；handle-per-stream + workspace 预分配（CuPy #4676 教训）；三层 sys→safe→api | MVP 后期（M8） |
| `image-io` | PNG/BMP/EXR 读写（G0 出图依赖） | MVP（G0 前） |
| `rx-python` | nanobind 工程模板 + DLPack 工具（§6） | MVP 验收前 |
| `geometry` | 网格/BVH 基础（G0/仿真共用） | G0 后 |
| `cudnn` | 完整绑定 | Phase 2+（明确延后，r12） |

## 6. Python 互操作（D-307）

r12 分期方案照搬：

- **MVP**：`rx build --emit=pyd` 产出 PYD；绑定层走 **nanobind + scikit-build-core**（非 PyO3/maturin——nanobind 对 C ABI 扩展约 4× 编译速度 / 5× 更小二进制 / 10× 更低开销，且 Rurix 不是 Rust 不受益于 PyO3 生态，r12）；数据通道 `__cuda_array_interface__` v3 + DLPack（`__dlpack__`/`from_dlpack`）双协议——零拷贝接入 PyTorch/CuPy（UC-01 的实现路径）。
- **Phase 2**：buffer protocol、stream context manager、type stubs 生成。
- **Phase 3**：装饰器式 kernel 调用糖、PyTorch mempool 共享。
- **Windows DLL 陷阱纪律**（r12 全部收录进文档与 `rx doc` 指南）：wheel 内捆绑 CUDA runtime DLL 至 `lib/`、`os.add_dll_directory()`、torch/lib 优先于 `CUDA_PATH\bin` 的搜索顺序事故（unsloth #5491）、delvewheel 处理 OpenMP。
- **永不**：Python 原生嵌入/解释器宿主（红线 1）。

## 7. 包管理与供应链（D-308 ~ D-312）

r8 的混合设计照搬：**Cargo 的工程体验骨架 + Zig 的声明式抓取 + Go 的透明完整性方向 + 拒绝任意构建脚本**。

### 7.1 MVP 形态

- `rurix.toml`（意图）+ `rurix.lock`（精确解析图 + 内容树 SHA-256）+ 可提交 `vendor/` + 默认完全离线可重建（`--locked/--offline`）。
- 依赖来源仅三类：`path` / `git` / `archive`（无 registry）。
- **无 build.rs**：`build.model = "declarative"`——native/GPU 元数据全部声明式（r8；npm Shai-Hulud 事故 + Cargo 沙箱 2026 仍未落地的双重论证）。逃生舱（受限 runner/allowlist）按需后置。
- workspace 单根锁；feature 模型 additive-v1 + `unification = "selected"`（预防 Cargo resolver v1 的 feature 泄漏教训，r8）。
- `-sys` 包 + `links` 唯一性（防重复符号）；Windows SDK/MSVC 经 vswhere 探测，不当包分发。

### 7.2 GPU 元数据（manifest 草案，r8）

```toml
[package.gpu]
toolkit    = "13.0..14"     # 兼容窗
min-driver = ">=560"
sm         = ["89"]          # 预编 cubin 覆盖（G1 起）
ptx-floor  = "compute_89"    # PTX 前向兼容基线
```

lockfile `[[artifact]]` 记录每个 GPU 产物变体（ptx/cubin/fatbin）与 digest。

### 7.3 演进路线

- 阶段二：受限 runner + SBOM precursor JSON。
- 阶段三（registry，**agent决策点 D-312**）：sparse index + lockfile 内容哈希 + **sumdb 式透明日志**（registry 不是唯一信任根，Go 模型优于 Cargo index-only，r8）；scopes/OIDC trusted publishing/Sigstore；typosquat 防御上线首日设计。

## 8. NVIDIA 再分发合规（D-313）

[08](08_RUNTIME_AND_TOOLING.md) §9 的生态侧细则：生态包附带 NVIDIA DLL 时只允许 Attachment A 白名单（cudart/nvrtc/cublas/cusparse/curand/npp/nvJitLink/nvvm/libdevice 等，r6 清单）；cuDNN 仅 runtime DLL（开发头/lib 不得再分发）；CI 维护白名单审计（防 AI 随手打包 SDK——r6 对 AI 协作的专门警示）。

## 9. 明确不进标准库的清单（永久登记）

ndarray 全功能（生态包）/ autograd（红线）/ 网络栈与异步运行时（非领域）/ GUI / 序列化框架（`Record` derive 提供地基，框架进生态）/ 正则（host 工具场景由生态包覆盖）。

## 10. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-11 | 初版 |
