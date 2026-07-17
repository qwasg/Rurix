# Rurix

> 让 GPU 系统编程拥有自己的 Rust。

[English](README.en.md) · [简体中文](README.md)

**Rurix** 是一门独立的、静态编译的 GPU 系统编程语言与工具链——把*资源所有权、地址空间、并行执行层级*做成类型系统的一等公民,让图形与 GPU 计算程序在不牺牲 CUDA 级底层控制的前提下,获得**可静态证明的安全性、可预测的性能与可长期治理的生态**。

CUDA 优先、Windows 原生、NVIDIA 单栈做深;三后端产出 PTX(运行时直连 CUDA Driver API)、DXIL(原生 D3D12 图形运行时)与 SPIR-V(MB1 起的单一 Vulkan/SPIR-V 跨端后端,AMD 桌面 + Android、compute+graphics;preview,feature 默认关闭)。

---

## 它解决什么

| 现状的痛 | Rurix 的回答 |
|---|---|
| GPU 代码内存/并发安全全靠人(CUDA C++)或设备侧全 `unsafe`(Rust-CUDA) | 宿主层 Rust 式所有权 + 设备层 execution resources / views / 地址空间类型;结构化并行静态证明无竞争,弱序协议显式 `unsafe` + 验证义务 |
| host/device 资源生命周期运行时炸(跨线程 `cuCtxDestroy`、流序分配 use-after-free) | Context/Stream/Event/Buffer 做成 **affine 类型**,生命周期错误变成**编译错误** |
| 工具链静默降级、permissive 编译 | **strict-only**:lowering 失败 = 结构化编译错误;能力位由真实设备探测驱动 |
| Windows 上 GPU 开发二等公民 | COFF/PE/PDB/Authenticode 原生工具链 + CUDA Driver API 一等运行时 |
| host C++ / shader / kernel 三套语言三套类型系统 | **单语言双层模型**:宿主与 kernel(含着色阶段)共享类型系统、泛型与模块系统,编译器静态检查 launch 边界 |
| 生态混乱生长 + AI 幻觉 API | 规范条款编号 ↔ conformance 测试 ↔ PR 强制引用三角;包管理无任意构建脚本 |

完整论证见 [`01_VISION_AND_MISSION.md`](01_VISION_AND_MISSION.md) 与 [`03_POSITIONING_AND_LANDSCAPE.md`](03_POSITIONING_AND_LANDSCAPE.md)。

## 项目状态:语言 1.0 已发行(`v1.0.0`),使命判据第一期 + 多后端第一期落地(`mb1-closed`)

第一层全量验收(01 §6)已达成,使命判据第一期(11 §6)已落地——首个以 Rurix 为主语言的生产级渲染器/仿真系统(第一方);多后端新纪元第一期(MB1)亦已收口。从 MVP 到 1.0 再到使命期与多后端期,14 个里程碑契约全部按验收门收口;性能与诊断预算全程 `measured_local`(零 estimated),预设资源生命周期错误类别 100% 编译期拦截:

| 阶段 | 收口 | 交付 |
|---|---|---|
| M0–M8(MVP) | 2026-06-17 `m8-closed` | 编译器/运行时/工具链闭环 + UC-01/02/03 三旗舰 + cublas 绑定 + 发布链路 + 双语诊断/文档站 |
| G1 | 2026-06-22 `g1-closed` | CUDA–D3D12 interop 实时呈现、流序分配 `AsyncBuffer<'stream,T>`、引擎集成 DLL(C ABI)、fatbin 生产分发、geometry 库 |
| G2 | 2026-06-30 `g2-closed` | 着色阶段进类型系统、DXIL 第二后端(D-131 混合路线)、绑定布局推导(root signature)、D3D12 运行时 + UC-04 deferred 渲染器、语言 1.0 机制就绪(edition "2026" + stable 面快照冻结) |
| V1 | 2026-07-14 `v1-closed` | 语言 1.0 首个 stable 发行(tag `v1.0.0`):stabilization report、FCP-lite 公示、stable channel 清单(rurixup)、首个 GitHub Release |
| MS1 | 2026-07-15 `ms1-closed` | `std::gpu` 单源宿主编排(单源 `.rx` → 单 EXE)+ 首个全 `.rx` 应用 ruridrop(UC-07) |
| MB1 | 2026-07-16 `mb1-closed` | 单一 Vulkan/SPIR-V 跨端后端(RFC-0011;AMD 桌面 + Android,compute+graphics;Android 真机 on-device measured;AMD 真卡尾门 G-MB1-6 诚实维持 open 待硬件;preview、feature 默认关闭) |

旗舰用例与关键交付(全部端到端真机验收):

- **UC-01 — PyTorch 算子替换**:`rx build --emit=pyd` 产 PYD(nanobind + scikit-build-core),经 `__cuda_array_interface__` v3 / DLPack 双协议零拷贝接入 PyTorch CUDA 张量;SAXPY/Reduction/GEMM 算子替换 **≥ 手写 CUDA C++ 90%**(measured_local)。
- **UC-02 — 三 stream 重叠流水线**:affine Context/Stream/Event/Buffer + 跨线程所有权转移 + 流序分配类型化;use-after-free / double-free / 跨线程 / 跨流未同步四类资源生命周期错误**编译期拦截**。
- **UC-03 — SPH 仿真 + compute 软光栅**:单可执行,确定性 SPH 仿真 + 软光栅 kernel(binning / tile 光栅 / 深度 / tonemap)+ host 帧循环,确定性出图。
- **UC-04 — deferred 渲染器(D3D12)**:DXIL 第二后端(D-131 混合路线:compute 直出 DXIL 最小子集通道,图形经 SPIR-V→HLSL→dxc 校验桥)+ 绑定布局推导(root signature RTS0)+ 多 pass 编排/barrier 锚定;lighting pass 真采样 G-buffer,离屏 readback 像素比对真机验收。
- **UC-07 — ruridrop 全 `.rx` 应用**:`std::gpu` 单源宿主编排(单 `.rx` 入口 → 单 EXE,内嵌 PTX+cubin);GPU SPH 溃坝仿真 + 球体光线追踪,离线 path-traced PPM 与实时 D3D12 present 共用同一 kernel 核;GPU 帧与 CPU 重放 golden **逐字节全等**(CI 冒烟档),实时 ~68fps@1280×720 / 131k 粒子(measured_local)。
- **cublas 绑定包**:GEMM/GEMV 三层绑定(raw FFI / 安全封装 / 高层 API)。
- **发布链路**:rurixup(stable channel 清单)+ Authenticode 签名/验签发布门(当前测试证书;of-record 生产签名后端 = Azure Artifact Signing,secret 门控)+ SBOM(SPDX/CycloneDX)+ NVIDIA 许可白名单审计。
- **诊断双语全量覆盖**(中/英)+ **文档站**(`rx doc`)。

> stable API 快照冻结已随语言 1.0 激活([`RD-008`](registry/deferred.json) 已关闭):stable 面(spec 条款 ID 全集 + 错误码含义 + edition 合法值 + `rx` CLI 命令面)经快照比对 + bless 审批守卫锚定,同一 edition 内只增不破坏,破坏性变更须经新 edition 隔离。

## 工作区

| 组件 | 职责 |
|---|---|
| `src/rurixc` | 编译器(前端 + MIR + NVPTX/DXIL/SPIR-V 三后端 + 借用/资源检查 + 格式化器 + LSP 会话) |
| `src/rurix-rt` | 运行时(CUDA Driver API 薄层:affine Context/Stream/Event/Buffer、launch、fatbin 装载协商、poisoned 状态机) |
| `src/rurix-rt-cabi` | 宿主编排 C ABI 运行时边界(`rxrt_*`/`rxp_*`/`rxio_*`:单源 `.rx` 应用 ↔ 运行时,fatbin 装载/launch/present/图像落盘) |
| `src/rx` | 工具链 CLI(`build`/`check`/`run`/`fmt`/`bench`/`test`/`doc`/`vendor`) |
| `src/rurix-pkg` | 包管理(lockfile + vendor + checksum) |
| `src/rurix-interop` | PyTorch 互操作(PYD / `__cuda_array_interface__` / DLPack 边界) |
| `src/rurix-cublas` | cublas v2 绑定包 |
| `src/rurixup` | 安装/引导器(发布链路、stable channel 清单) |
| `src/rurix-d3d12` | D3D12/DXGI 呈现 shim(CUDA–D3D12 interop 实时呈现边界) |
| `src/rurix-engine` | 引擎集成 DLL(C ABI cdylib,嵌入 C++/D3D12 宿主承担 compute pass) |
| `src/rurix-geometry` | 几何库(mesh/BVH,零依赖全 safe) |
| `src/image-io` · `src/soft-raster` | 图像 I/O · 软光栅 host CPU 参考库(与 device kernel 数值语义同义) |
| `src/uc02-demo` · `src/uc03-demo` · `src/uc04-demo` | 旗舰用例演示 |
| `apps/ruridrop` | UC-07 全 `.rx` 应用(渲染器/仿真二合一;非 Cargo crate,声明式 `rurix.toml` 包,零 .rs) |

## 上手

**环境**:Windows 11 + NVIDIA GPU(开发对照机 RTX 4070 Ti)、CUDA Toolkit、MSVC 2022。Rurix 工具链自身用 Rust 构建(D-201)。

预编译二进制(`rx.exe`/`rurixup.exe` + SBOM + `SHA256SUMS`)见 [GitHub Releases](https://github.com/qwasg/Rurix/releases)(自 v1.0.0 起;当前为测试证书 Authenticode 签名,SmartScreen 可能警示)。从源码构建:

```sh
# 构建工作区
cargo build --workspace

# 用 rx 工具链
cargo run -p rx -- build <input.rx>      # 编译(产 host EXE;--emit=ptx / pyd 等)
cargo run -p rx -- check <input.rx>      # 仅检查(借用/资源/类型)
cargo run -p rx -- bench saxpy           # 微基准(BENCH_PROTOCOL 协议化采样)
cargo run -p rx -- doc --root . --out target/doc   # 生成文档站
```

文档站(`rx doc`)从单一事实源(`spec/*.md`、`registry/error_codes.json`、`conformance/`)确定性生成:规范条款索引、错误码索引、traceability 矩阵。

**想学怎么写 Rurix 代码**,见入门教程 [`guide/`](guide/)——从第一个 host 程序到第一个 kernel 的渐进式路径,可独立编译的示例均经 CI 门(`rx check`/`rx run`)真跑。

## 治理与质量门

Rurix 从第一天把治理内建为产品力(AI 时代语言基础设施,见 [`10_GOVERNANCE.md`](10_GOVERNANCE.md)):

- **规范 ↔ 测试 ↔ PR 三角**:每条 RXS 规范条款 ≥1 测试锚定(`ci/trace_matrix.py`,当前 195/195)。
- **measured_local 预算**:性能/诊断基线全部真机实测,零 estimated 占位(`ci/budget_eval.py --strict`)。
- **真实红绿**:每道 CI 门经「构造缺陷 → 红 → 复原 → 绿」验证(反 YAML-only),run URL 归档于 [`evidence/`](evidence/)。
- **字节级 guardrails**、schema 校验、结构校验、conformance 全绿、UI/MIR/PTX/DXIL golden 与 stable API 快照经 bless。
- **deferred / spike-gating 注册表**:延期项与扩张方向唯一事实源,只追加。

里程碑契约与 close-out 留痕见 [`milestones/`](milestones/);治理机制总览见 [`14_ENGINEERING_DISCIPLINE.md`](14_ENGINEERING_DISCIPLINE.md)。

## 克制声明

Rurix **不**取代 CUDA 生态(在其上提供安全编译前端与运行时)、**不**首发跨平台(NVIDIA 单栈做深)、**不**做 ML 框架(与 PyTorch 经 DLPack 零拷贝互操作)。每条克制对应一条已验证的死亡路线([`03_POSITIONING_AND_LANDSCAPE.md`](03_POSITIONING_AND_LANDSCAPE.md) §4)。

## 文档地图

`00_MASTER_INDEX.md` 为总索引;`01`–`14` 为规划文档集(愿景 / 定位 / 设计原则 / 语言与编译器架构 / GPU 编程模型 / 运行时与工具链 / 标准库与生态 / 治理 / 路线图 / 工程纪律)。`spec/` 为可测试规范(FLS 体例,RXS 条款),`conformance/` 为唯一验收边界,`rfcs/` 为语言演进 RFC / Mini-RFC 序列。

## 贡献

欢迎贡献。请先读 [`CONTRIBUTING.md`](CONTRIBUTING.md)(规范↔测试↔PR 三角、变更分档、AI 贡献政策、`unsafe` 纪律)与 [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md);安全问题见 [`SECURITY.md`](SECURITY.md)。

## 许可

双许可,任选其一(D-003):

- Apache License 2.0([`LICENSE-APACHE`](LICENSE-APACHE))
- MIT License([`LICENSE-MIT`](LICENSE-MIT))

`SPDX-License-Identifier: MIT OR Apache-2.0`。除非你明确声明,否则你有意提交并纳入本项目的任何贡献,均按上述双许可授权,无附加条款。
