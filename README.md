# Rurix

> 让 GPU 系统编程拥有自己的 Rust。

**Rurix** 是一门独立的、静态编译的 GPU 系统编程语言与工具链——把*资源所有权、地址空间、并行执行层级*做成类型系统的一等公民,让图形与 GPU 计算程序在不牺牲 CUDA 级底层控制的前提下,获得**可静态证明的安全性、可预测的性能与可长期治理的生态**。

CUDA 优先、Windows 原生、NVIDIA 单栈做深;后端产出 PTX,运行时直连 CUDA Driver API。

---

## 它解决什么

| 现状的痛 | Rurix 的回答 |
|---|---|
| GPU 代码内存/并发安全全靠人(CUDA C++)或设备侧全 `unsafe`(Rust-CUDA) | 宿主层 Rust 式所有权 + 设备层 execution resources / views / 地址空间类型;结构化并行静态证明无竞争,弱序协议显式 `unsafe` + 验证义务 |
| host/device 资源生命周期运行时炸(跨线程 `cuCtxDestroy`、流序分配 use-after-free) | Context/Stream/Event/Buffer 做成 **affine 类型**,生命周期错误变成**编译错误** |
| 工具链静默降级、permissive 编译 | **strict-only**:lowering 失败 = 结构化编译错误;能力位由真实设备探测驱动 |
| Windows 上 GPU 开发二等公民 | COFF/PE/PDB/Authenticode 原生工具链 + CUDA Driver API 一等运行时 |
| host C++ / shader / kernel 三套语言三套类型系统 | **单语言双层模型**:宿主与 kernel 共享类型系统、泛型与模块系统,编译器静态检查 launch 边界 |
| 生态混乱生长 + AI 幻觉 API | 规范条款编号 ↔ conformance 测试 ↔ PR 强制引用三角;包管理无任意构建脚本 |

完整论证见 [`01_VISION_AND_MISSION.md`](01_VISION_AND_MISSION.md) 与 [`03_POSITIONING_AND_LANDSCAPE.md`](03_POSITIONING_AND_LANDSCAPE.md)。

## 项目状态:MVP 完结(`m8-closed`)

第一层全量验收(01 §6)已达成。三大旗舰用例端到端真跑、性能判据达标、资源生命周期错误类别 100% 编译期拦截、全部预算阈值 `measured_local`(零 estimated):

- **UC-01 — PyTorch 算子替换**:`rx build --emit=pyd` 产 PYD(nanobind + scikit-build-core),经 `__cuda_array_interface__` v3 / DLPack 双协议零拷贝接入 PyTorch CUDA 张量;SAXPY/Reduction/GEMM 算子替换 **≥ 手写 CUDA C++ 90%**(measured_local)。
- **UC-02 — 三 stream 重叠流水线**:affine Context/Stream/Event/Buffer + 跨线程所有权转移 + 流序分配类型化;use-after-free / double-free / 跨线程 / 跨流未同步四类资源生命周期错误**编译期拦截**。
- **UC-03 — SPH 仿真 + compute 软光栅**:单可执行,粒子更新 + 空间哈希 + 光栅化 kernel + host 帧循环,确定性出图。
- **cublas 绑定包**:GEMM/GEMV 三层绑定(raw FFI / safe wrapper / 高层 API)。
- **发布链路**:rurixup + MSI + winget + Azure Artifact Signing(Authenticode)+ SBOM(SPDX/CycloneDX)+ NVIDIA 许可白名单审计。
- **诊断双语全量覆盖**(中/英)+ **文档站**(`rx doc`)。

> stable API 快照冻结评估:MVP 收口维持 `not_frozen`(公开面仍处收敛期),机制激活留首个 stable 发布([`RD-008`](registry/deferred.json))。

## 工作区

| Crate | 职责 |
|---|---|
| `src/rurixc` | 编译器(前端 + MIR + NVPTX 后端 + 借用/资源检查 + 格式化器 + LSP 会话) |
| `src/rurix-rt` | 运行时(CUDA Driver API 绑定、execution resources) |
| `src/rx` | 工具链 CLI(`build`/`check`/`run`/`fmt`/`bench`/`test`/`doc`/`watch`/`vendor`) |
| `src/rurix-pkg` | 包管理(lockfile + vendor + checksum) |
| `src/rurix-interop` | PyTorch 互操作(PYD / `__cuda_array_interface__` / DLPack 边界) |
| `src/rurix-cublas` | cublas v2 绑定包 |
| `src/rurixup` | 安装/引导器(发布链路) |
| `src/image-io` · `src/soft-raster` | 图像 I/O · compute 软光栅库 |
| `src/uc02-demo` · `src/uc03-demo` | 旗舰用例演示 |

## 上手

**环境**:Windows 11 + NVIDIA GPU(开发对照机 RTX 4070 Ti)、CUDA Toolkit、MSVC 2022。Rurix 工具链自身用 Rust 构建(D-201)。

```sh
# 构建工作区
cargo build --workspace

# 用 rx 工具链
cargo run -p rx -- build <manifest>      # 编译(产 PTX / PYD)
cargo run -p rx -- check <manifest>      # 仅检查(借用/资源/类型)
cargo run -p rx -- bench saxpy           # 微基准(BENCH_PROTOCOL 协议化采样)
cargo run -p rx -- doc --root . --out target/doc   # 生成文档站
```

文档站(`rx doc`)从单一事实源(`spec/*.md`、`registry/error_codes.json`、`conformance/`)确定性生成:规范条款索引、错误码索引、traceability 矩阵。

## 治理与质量门

Rurix 从第一天把治理内建为产品力(AI 时代语言基础设施,见 [`10_GOVERNANCE.md`](10_GOVERNANCE.md)):

- **规范 ↔ 测试 ↔ PR 三角**:每条 RXS 规范条款 ≥1 测试锚定(`ci/trace_matrix.py`,当前 139/139)。
- **measured_local 预算**:性能/诊断基线全部真机实测,零 estimated 占位(`ci/budget_eval.py --strict`)。
- **真实红绿**:每道 CI 门经「构造缺陷 → 红 → 复原 → 绿」验证(反 YAML-only),run URL 归档于 [`evidence/`](evidence/)。
- **字节级 guardrails**、schema 校验、结构校验、conformance 全绿、UI/MIR/PTX golden 经 bless。
- **deferred / spike-gating 注册表**:延期项与扩张方向唯一事实源,只追加。

里程碑契约与 close-out 留痕见 [`milestones/`](milestones/);治理机制总览见 [`14_ENGINEERING_DISCIPLINE.md`](14_ENGINEERING_DISCIPLINE.md)。

## 克制声明

Rurix **不**取代 CUDA 生态(在其上提供安全编译前端与运行时)、**不**首发跨平台(NVIDIA 单栈做深)、**不**做 ML 框架(与 PyTorch 经 DLPack 零拷贝互操作)。每条克制对应一条已验证的死亡路线([`03_POSITIONING_AND_LANDSCAPE.md`](03_POSITIONING_AND_LANDSCAPE.md) §4)。

## 文档地图

`00_MASTER_INDEX.md` 为总索引;`01`–`14` 为规划文档集(愿景 / 定位 / 设计原则 / 语言与编译器架构 / GPU 编程模型 / 运行时与工具链 / 标准库与生态 / 治理 / 路线图 / 工程纪律)。`spec/` 为可测试规范(FLS 体例,RXS 条款),`conformance/` 为唯一验收边界。

## 贡献

欢迎贡献。请先读 [`CONTRIBUTING.md`](CONTRIBUTING.md)(规范↔测试↔PR 三角、变更分档、AI 贡献政策、`unsafe` 纪律)与 [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md);安全问题见 [`SECURITY.md`](SECURITY.md)。

## 许可

双许可,任选其一(D-003):

- Apache License 2.0([`LICENSE-APACHE`](LICENSE-APACHE))
- MIT License([`LICENSE-MIT`](LICENSE-MIT))

`SPDX-License-Identifier: MIT OR Apache-2.0`。除非你明确声明,否则你有意提交并纳入本项目的任何贡献,均按上述双许可授权,无附加条款。
