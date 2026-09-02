# Rurix

> 让 GPU 系统编程拥有自己的 Rust。

[English](README.en.md) · [简体中文](README.md)

**Rurix** 是一门独立的、静态编译的 GPU 系统编程语言与工具链——把*资源所有权、地址空间、并行执行层级*做成类型系统的一等公民，让图形与 GPU 计算程序在不牺牲 CUDA 级底层控制的前提下，获得**可静态证明的安全性、可预测的性能与可长期治理的生态**。

CUDA 优先、Windows 原生、NVIDIA 单栈做深；三后端产出 PTX（运行时直连 CUDA Driver API）、DXIL（原生 D3D12 图形运行时）与 SPIR-V（MB1 起的单一 Vulkan/SPIR-V 跨端后端，AMD 桌面 + Android，compute + graphics；preview，feature 默认关闭）。

---

**目录**：[它解决什么](#它解决什么) · [项目状态](#项目状态) · [工作区](#工作区) · [上手](#上手) · [治理与质量门](#治理与质量门) · [克制声明](#克制声明) · [文档地图](#文档地图) · [贡献](#贡献) · [许可](#许可)

## 它解决什么

| 现状的痛 | Rurix 的回答 |
|---|---|
| GPU 代码内存/并发安全全靠人（CUDA C++）或设备侧全 `unsafe`（Rust-CUDA） | 宿主层 Rust 式所有权 + 设备层 execution resources / views / 地址空间类型；结构化并行静态证明无竞争，弱序协议显式 `unsafe` + 验证义务 |
| host/device 资源生命周期运行时炸（跨线程 `cuCtxDestroy`、流序分配 use-after-free） | Context/Stream/Event/Buffer 做成 **affine 类型**，生命周期错误变成**编译错误** |
| 工具链静默降级、permissive 编译 | **strict-only**：lowering 失败 = 结构化编译错误；能力位由真实设备探测驱动 |
| Windows 上 GPU 开发二等公民 | COFF/PE/PDB/Authenticode 原生工具链 + CUDA Driver API 一等运行时 |
| host C++ / shader / kernel 三套语言三套类型系统 | **单语言双层模型**：宿主与 kernel（含着色阶段）共享类型系统、泛型与模块系统，编译器静态检查 launch 边界 |
| 生态混乱生长 + AI 幻觉 API | 规范条款编号 ↔ conformance 测试 ↔ PR 强制引用三角；包管理无任意构建脚本 |

完整论证见 [`01_VISION_AND_MISSION.md`](01_VISION_AND_MISSION.md) 与 [`03_POSITIONING_AND_LANDSCAPE.md`](03_POSITIONING_AND_LANDSCAPE.md)。

## 项目状态

**语言 1.0 已发行（tag `v1.0.0`）。里程碑主线 M0 → G30 共 44 个契约收口（最新收口 tag `g30-closed`）；2026-08-27 起转入同工作区战役制推进（G31–G39），G31~G36 六契约 `status: active` / `implementation_status: unlocked` 如实维持，不冒充收口。**

- 第一层全量验收（01 §6）已达成；使命判据第一期（11 §6）已落地——ruridrop 是首个以 Rurix 为主语言的生产级渲染器/仿真系统（第一方）。
- 预设资源生命周期错误类别 100% 编译期拦截；既有性能阈值均以 `measured_local` 留证、零 `estimated`（G6 明确未设性能硬门）。
- 各门终态（含 no-go / defer / 诚实红）一律如实登记，不回写为 PASS。

> **事实源**：里程碑事实源恒为 `milestones/<id>/*_CONTRACT.md` 契约 §8 close-out、各期 `*_P2_DECISIONS.md` 与 [`registry/`](registry/)；战役制交付与门 evidence 见 `artifacts/day_*/`（交接单 `HANDOVER.md`）与 [`evidence/`](evidence/)。本节仅为索引镜像；历次逐条追加的「状态勘误」原文保留在 git 历史中（`git log -p -- README.md`）。

### 里程碑主线（已收口）

| 阶段 | 收口 | 主题 / 交付 |
|---|---|---|
| M0–M8（MVP） | 2026-06-17 `m8-closed` | 编译器 / 运行时 / 工具链闭环 + UC-01/02/03 三旗舰 + cublas 绑定 + 发布链路 + 双语诊断 / 文档站 |
| G1 | 2026-06-22 `g1-closed` | CUDA–D3D12 interop 实时呈现、流序分配 `AsyncBuffer<'stream,T>`、引擎集成 DLL（C ABI）、fatbin 生产分发、geometry 库 |
| G2 | 2026-06-30 `g2-closed` | 着色阶段进类型系统、DXIL 第二后端（D-131 混合路线）、绑定布局推导（root signature）、D3D12 运行时 + UC-04 deferred 渲染器、语言 1.0 机制就绪（edition "2026" + stable 面快照冻结） |
| V1 | 2026-07-14 `v1-closed` | 语言 1.0 首个 stable 发行（tag `v1.0.0`）：stabilization report、FCP-lite 公示、stable channel 清单（rurixup）、首个 GitHub Release |
| MS1 | 2026-07-15 `ms1-closed` | `std::gpu` 单源宿主编排（单源 `.rx` → 单 EXE）+ 首个全 `.rx` 应用 ruridrop（UC-07） |
| MB1 | 2026-07-16 `mb1-closed` | 单一 Vulkan/SPIR-V 跨端后端（RFC-0011；AMD 桌面 + Android，compute + graphics；Android 真机 measured；AMD 真卡尾门 G-MB1-6 诚实维持 open 待硬件；preview、feature 默认关闭） |
| G3 | 2026-07-19 `g3-closed` | 工业渲染期：RD-027 毒径归因闸门 + 五特性面全量落地（采样超集 / bindless / render graph 自动 barrier / UC-04 窗口 present / mesh-task-RT 双后端） |
| EI1 | 2026-07-23 `ei1-closed` | 引擎集成期：UC-05 最小 RHI + render graph 核心 + RD-009 `#[export(c)]` C ABI 导出 codegen 与内建头文件生成（D-113） |
| G4 | 2026-07-24 `g4-closed` | 引擎渲染期：图形 RHI 化 raster/mesh 库面 + 自动 barrier + engine_host v3 嵌入 + `.rx` 单源 Vulkan RHI 通道 + BLACKHOLE 生产档验收（RD-036 open 存续） |
| EA1 | 2026-07-28 `ea1-closed` | 分发与门面期：rurixup 真实分发（RD-025 兑现）+ 预编译工具链 bundle（`v1.0.1-dist` 系列，pre-release）+ 文档门面 + 冷启动验收 |
| G5 | 2026-07-29 `g5-closed` | 原生渲染器期：声明式 render graph（`rurix-render`）+ RHI 图形派发桥 + 虚拟化几何（meshlet / GPU 两级剔除 / VisBuffer）+ VSM 阴影 + 屏幕探针 GI + 光追效果 + 材质流送 + 时域重建（TAA/TSR）；UC-06 全管线 demo device 真跑 |
| G6 | 2026-08-01 `g6-closed` | 渲染物理双轨期：Jolt 生产默认物理库 + Rapier 默认关闭快路径 + Physics→GpuScene 单向桥 + UC-08 合流 demo + Taichi Vulkan AOT 特效副轨 |
| G7 | 2026-08-05 `g7-closed` | 生产帧闭环期：RD-038 closed（compute SPIR-V 1.4 / RayQuery、W3 GI/RTAO/硬阴影、VisBuffer SW/HW diff=0、One True Device Frame + soak） |
| G8 | 2026-08-06（契约 §8） | UE5 级前置能力完成期：RFC-0019/0020/0021 + 资产管线（`rurix-asset` / 几何页 / basis_universal 纹理 codec）+ G8.2~G8.8 各波验收，门终态含 no-go / defer 诚实留档 |
| G9 | 2026-08-15（契约 §8.10） | UE5 目标渲染 / 物理平台期：RFC-0022/0023/0024，G9.2~G9.6 五波 + P2 穷举 33 行 + soak ≥30min；15 个 P0 + 19 个 go P1 全绿 |
| G10 | 2026-08-16（契约 §8.10） | UE5 画面对标基线期：UE5 5.8 出图环境 / 压测语料 / 度量基建 / 首轮 A/B；差距清单 11 行锁定为 G11 法定输入 |
| G11 | 2026-08-17 `g11-closed` | GI 与光照画质闭环期：口径差对齐、资产与场景修复、灯种子集与多反弹 GI（含 M99-clipmap）；11 行复测终态 converged 8 + aligned_closed 3 |
| G12 | 2026-08-17 `g12-closed` | 路径追踪生产化期：降噪管线 + TSR 联动、UE PT 双端对拍、PT 吞吐基线（50×3 协议）；10 行差距清单锁定为 G13 法定输入 |
| G13 | 2026-08-19 `g13-closed` | 超分采样与 Lumen 对照期：DLSS SR（Streamline 2.10.3 NGX Vulkan）/ FSR 3.1.5 / TSR device 化三后端切换 + UE DLSS / Lumen 双臂对照；8+2 行登记表锁定为 G14/G15 法定输入 |
| G14（含 G14plus） | 2026-08-23 `g14-closed` | 正式帧率对标与渲染管线性能期：M-d 18/18 达标（通过线 ×1.00），帧率差距登记表空表终态；RD-045 维持 open |
| G15（含 G15plus） | 2026-08-23 `g15-closed` | 画质量级收口与商用终审期：**双未达标如实登记**——商用收口 0/18 + 性能 17/18（单格环境事件面），三面承接锚交 G16+ |
| G16（含 G16plus） | 2026-08-24 `g16-closed` | UE cornell 参照臂修复（不再死黑）+ 受影响门重测；M-g 绝对画质 18/18（程序产阈 p100×2.0，G15 M-c 历史 0/18 不改写） |
| G17 | 2026-08-24 `g17-closed` | DLSS 性能缺口收口期：NGX 310.6.0 换版**拒绝**（Streamline 2.10.3 pin 下不兼容）、RFC-0032 D3D12 宿主车道 **defer**、M-d 终判 ratio 0.856326 维持 17/18 诚实红 |
| G18 | 2026-08-24 `g18-closed` | 全量方向一次性收口期：九 P0 全绿含诚实终态（SL not-available / fps 17/18 / mesh shader no-go / 帧生成 defer-to-G19+），P2 穷举 25 行 |
| G19–G25 | 2026-08-24 `g19-closed` … `g25-closed` | 七期串行战役：帧生成独立层 / 虚拟化几何 P4 / 光照 P3+ / 材质·流送·时域 / 物理平台深化 / 呈现与尾门清理 / 全量商用终审收官；35 个 P0 全绿、P2 穷举 79 行、四个 host 参考实现件（framegen / hzb / restir_reservoir / slab）；总结见 [`G19_G25_CAMPAIGN_RECORD.md`](milestones/g25/G19_G25_CAMPAIGN_RECORD.md) |
| G26–G30 | 2026-08-25 `g26-closed` … `g30-closed` | 五期 device 化战役：上述四件落地为真 `.rx` device kernel（`g26_framegen` / `g27_hzb_reduce` / `g28_restir` / `g29_slab`，位级双跑 + 冻结容差对拍）+ 材质侧表臂；RFC-0043~0047 共 63 findings 全 disposition；G30 终审 fps 17/18 诚实红定盘（焦点格 ratio 0.960479），承接归档 `g30_campaign_handover_registry.json` = G31+ 唯一法定输入 |

### 战役制推进（2026-08-27 起）

契约 flip 与正式立项程序留 owner；交付与门 evidence 随役入库 `artifacts/day_*/`。

| 战役 | 日期 / commit | 摘要 |
|---|---|---|
| G31 / G32 / G33 + G34 | 2026-08-27 `058f8e68` | 实时呈现期（波 A）/ 画面完整期（波 B）/ 商业化期（波 C）三期交付一次性入库，§8 close-out 在案、契约保持 `active`；G34 全特性合流收口验收：统一车道地基八 facts + HZB 接统一车道六 facts + 蒙皮进统一车道九面判据 + Stage A digest 18/18 零漂移 + soak 5010/5010 帧零崩 |
| G35 / G36 | 2026-08-27 | G35 GPU 粒子系统期（RFC-0049）九波工件入树，G35-4 半透明 sort/OIT 双臂门 PASS；G36 互斥项修复与生产组合渲染 W1~W5 交付，`g36.wave1.geo_composition` 十 facts 门真跑 PASS |
| G37 商业化交付收官 | 2026-08-30 `0e605c34` | `g31_window_present --quality` 缺省 **off→full 十九臂翻转**（帧时 9.75/10.59ms ≤ 11.11ms 预算）+ 七组新臂（透明 / LUT / PSO 账本 / VisBuffer 证据臂 / RIS+NEE / 逐帧 cut / FG×full）+ 修复十件（含 rurixc if-while codegen 回边裁剪）+ 商业化 GAP-01~03 闭合 + **SDK bundle 候选 `sdk-1.1.0`**（24 组件、双 SBOM、四级校验）+ 双 soak 绿 |
| G38 五任务推进 | 2026-08-30 `b05cd4ef` | 法线 v2 消费切换整批重锚 + FIF×动态 #90 收口（RFC-0030 v1.1 L2a + slot_as 生产接线 + 每槽 AS 内存预算门）+ frame_cut 增量 refit（build 8.78ms 进 90fps 预算）+ #96 消费面收口 + RIS/NEE 画质定量与 lamp-k 阶梯（默认维持 12/0.6） |
| DLSS 5 NR 适配 | 2026-08-30 `82a59ae3` | NGX feature-18 手写 D3D12 FFI 集成 + 三臂可用性探针 + nr lane harness；本机 Ada verdict = **not_available** 如实登记（fail-closed / default off / env opt-in；评估件不入库） |
| G39 五任务推进 | 2026-09-01 `1478859a` | ReSTIR 高档时域 reservoir 提灯臂生产接线（26 簇档 off 11.546ms → **on 7.526ms 进 11.11ms 预算**）+ skin 批次 B + slot_as 单源折叠 + profiling 门 N=5 中位判据 + device cut P1 等价门；零重锚 + CPU 守卫 7/7 + soak 1936.2s 零失败。交接单：[`artifacts/day_0831_g39/HANDOVER.md`](artifacts/day_0831_g39/HANDOVER.md) |

### 如实登记的未达标与留档项

- **fps 焦点格 17/18**（bistro-interior / t100 / dlss_sr）：自 G15 起维持诚实红，G17 / G25 / G30 三次终判均未达 ×1.00（ratio 0.856326 → 0.960479，G34 复测 0.921836）；承接锚 = NGX 分解 profiling / UE 插桩。
- **商用收口 G15 M-c 0/18** 历史记录不改写；G16plus M-g 按程序产阈 18/18 另行登记。
- **RD 八条 open 维持**（含 RD-045 长窗观察 maintain-open、RD-034 blocked、RD-036 存续）；vendor / 硬件面 Streamline 310.6.0 与 DLSS 5 NR not-available、mesh shader（M61）no-go、Work Graphs 与 HDR 输出设备 not-available、异步三件套 M59 no-go，均留 measured 证据。

### 旗舰用例与关键交付

全部端到端真机验收：

- **UC-01 — PyTorch 算子替换**：`rx build --emit=pyd` 产 PYD（nanobind + scikit-build-core），经 `__cuda_array_interface__` v3 / DLPack 双协议零拷贝接入 PyTorch CUDA 张量；SAXPY/Reduction/GEMM 算子替换 **≥ 手写 CUDA C++ 90%**（measured_local）。
- **UC-02 — 三 stream 重叠流水线**：affine Context/Stream/Event/Buffer + 跨线程所有权转移 + 流序分配类型化；use-after-free / double-free / 跨线程 / 跨流未同步四类资源生命周期错误**编译期拦截**。
- **UC-03 — SPH 仿真 + compute 软光栅**：单可执行，确定性 SPH 仿真 + 软光栅 kernel（binning / tile 光栅 / 深度 / tonemap）+ host 帧循环，确定性出图。
- **UC-04 — deferred 渲染器（D3D12）**：DXIL 第二后端（D-131 混合路线：compute 直出 DXIL 最小子集通道，图形经 SPIR-V→HLSL→dxc 校验桥）+ 绑定布局推导（root signature RTS0）+ 多 pass 编排 / barrier 锚定；lighting pass 真采样 G-buffer，离屏 readback 像素比对真机验收。
- **UC-07 — ruridrop 全 `.rx` 应用**：`std::gpu` 单源宿主编排（单 `.rx` 入口 → 单 EXE，内嵌 PTX+cubin）；GPU SPH 溃坝仿真 + 球体光线追踪，离线 path-traced PPM 与实时 D3D12 present 共用同一 kernel 核；GPU 帧与 CPU 重放 golden **逐字节全等**（CI 冒烟档），实时 ~68fps@1280×720 / 131k 粒子（measured_local）。
- **cublas 绑定包**：GEMM/GEMV 三层绑定（raw FFI / 安全封装 / 高层 API）。
- **发布链路**：rurixup（stable channel 清单）+ Authenticode 签名 / 验签发布门（当前测试证书；of-record 生产签名后端 = Azure Artifact Signing，secret 门控）+ SBOM（SPDX/CycloneDX）+ NVIDIA 许可白名单审计。
- **诊断双语全量覆盖**（中 / 英）+ **文档站**（`rx doc`）。

> stable API 快照冻结已随语言 1.0 激活（[`RD-008`](registry/deferred.json) 已关闭）：stable 面（spec 条款 ID 全集 + 错误码含义 + edition 合法值 + `rx` CLI 命令面）经快照比对 + bless 审批守卫锚定，同一 edition 内只增不破坏，破坏性变更须经新 edition 隔离。

## 工作区

**语言与工具链**

| 组件 | 职责 |
|---|---|
| `src/rurixc` | 编译器（前端 + MIR + NVPTX/DXIL/SPIR-V 三后端 + 借用 / 资源检查 + 格式化器 + LSP 会话） |
| `src/rx` | 工具链 CLI（`build` / `check` / `run` / `fmt` / `bench` / `test` / `doc` / `vendor`） |
| `src/rurix-pkg` | 包管理（lockfile + vendor + checksum） |
| `src/rurixup` | 安装 / 引导器（发布链路、stable channel 清单） |

**运行时与互操作**

| 组件 | 职责 |
|---|---|
| `src/rurix-rt` | 运行时（CUDA Driver API 薄层：affine Context/Stream/Event/Buffer、launch、fatbin 装载协商、poisoned 状态机） |
| `src/rurix-rt-cabi` | 宿主编排 C ABI 运行时边界（`rxrt_*` / `rxp_*` / `rxio_*`：单源 `.rx` 应用 ↔ 运行时，fatbin 装载 / launch / present / 图像落盘） |
| `src/rurix-interop` | PyTorch 互操作（PYD / `__cuda_array_interface__` / DLPack 边界） |
| `src/rurix-cublas` | cublas v2 绑定包 |
| `src/rurix-d3d12` | D3D12/DXGI 呈现 shim（CUDA–D3D12 interop 实时呈现边界） |
| `src/rurix-engine` | 引擎集成 DLL（C ABI cdylib，嵌入 C++/D3D12 宿主承担 compute pass） |
| `src/rurix-android-present` | Android on-device present 胶水（MB1；零-Java NativeActivity cdylib 壳，桌面编译为空 lib） |

**渲染器、几何与资产**

| 组件 | 职责 |
|---|---|
| `src/rurix-render` | 原生引擎渲染器库（声明式 render graph / 虚拟化几何 / VSM / 探针 GI / 光追效果 / 材质流送 / 时域重建 / 帧生成 / ReSTIR；渲染器是库不进语言） |
| `src/rurix-renderer-sdk` | 渲染器 SDK C ABI 实现层（`rxsdk_*` 会话面，首个 stable 嵌入 ABI；G31+） |
| `src/rurix-geometry` | 几何库（mesh / BVH，零依赖全 safe） |
| `src/rurix-geom-build` | 离线几何构建器（网格 → meshlet 化 → 分组简化层级 DAG + CPU 参照剔除器，host 纯 safe 确定性） |
| `src/rurix-geom-pages` | 几何页格式（RXPL / RXPD / RXPM 编解码，`spec/geometry_pages.md`） |
| `src/rurix-asset` | 资产管线（RFC-0020：几何页构建、canon / graph / cook / verify、glTF 导入、纹理 codec） |
| `src/rurix-basis-sys` | basis_universal 纹理 codec FFI 边界（UASTC→KTX2 / ETC1S / BCn·ASTC transcode；`unsafe` 集中地） |
| `src/image-io` · `src/soft-raster` | 图像 I/O · 软光栅 host CPU 参考库（与 device kernel 数值语义同义） |

**物理**

| 组件 | 职责 |
|---|---|
| `src/rurix-physics` | 引擎物理库（RFC-0017：固定步 `PhysicsWorld`；Jolt 生产默认 / Rapier 默认关闭快路径） |
| `src/rurix-physics-sys` | JoltC FFI 边界（Jolt 5.3 基线；物理 `unsafe` 唯一集中地） |
| `src/rurix-physics-sys56` | Jolt 5.6 评估臂 FFI（与 5.3 并存不覆盖，A/B 用） |

**应用与演示**

| 组件 | 职责 |
|---|---|
| `apps/ruridrop` | UC-07 全 `.rx` 应用（渲染器 / 仿真二合一；非 Cargo crate，声明式 `rurix.toml` 包，零 `.rs`） |
| `apps/uc05-rhi` | UC-05 最小 RHI + render graph（`.rx` 包，`--emit=dll` 导出 C ABI；渲染器最小集成示例的基座） |
| `apps/uc06-renderer` | UC-06 全管线 demo（剔除 → VisBuffer → 延迟着色 → GI/VSM/RTAO → TAA/TSR → headless readback 像素断言） |
| `apps/uc08-physics` · `apps/uc09-taichi-spike` | UC-08 渲染×物理合流 demo · Taichi Vulkan AOT 特效副轨 spike |
| `apps/blackhole` | BLACKHOLE 生产档渲染演示（G4 验收） |
| `apps/g31-renderer-sdk` | 渲染器 SDK `.rx` 包 + 宿主示例（含 `API_VERSIONING.md`） |
| `apps/g8-physics-gates` · `apps/g9-physics-gates` | G8 / G9 物理验收门 harness |
| `src/uc02-demo` · `src/uc03-demo` · `src/uc04-demo` | 旗舰用例演示 |

## 上手

**环境**：Windows 11 + NVIDIA GPU（开发对照机 RTX 4070 Ti）、CUDA Toolkit、MSVC 2022。Rurix 工具链自身用 Rust 构建（D-201）。

预编译二进制（`rx.exe` / `rurixup.exe` + SBOM + `SHA256SUMS`）见 [GitHub Releases](https://github.com/qwasg/Rurix/releases)（自 v1.0.0 起；当前为测试证书 Authenticode 签名，SmartScreen 可能警示）。从源码构建：

```sh
# 构建工作区
cargo build --workspace

# 用 rx 工具链
cargo run -p rx -- build <input.rx>      # 编译（产 host EXE；--emit=ptx / pyd 等）
cargo run -p rx -- check <input.rx>      # 仅检查（借用 / 资源 / 类型）
cargo run -p rx -- bench saxpy           # 微基准（BENCH_PROTOCOL 协议化采样）
cargo run -p rx -- doc --root . --out target/doc   # 生成文档站
```

文档站（`rx doc`）从单一事实源（`spec/*.md`、`registry/error_codes.json`、`conformance/`）确定性生成：规范条款索引、错误码索引、traceability 矩阵。

- **想学怎么写 Rurix 代码**：见入门教程 [`guide/`](guide/)——从第一个 host 程序到第一个 kernel 的渐进式路径，可独立编译的示例均经 CI 门（`rx check` / `rx run`）真跑。
- **想把渲染器嵌进自己的引擎**：见 [`docs/renderer/integration_guide.md`](docs/renderer/integration_guide.md)（C ABI 宿主五步 + 最小示例工程 `docs/renderer/examples/minimal_host/`），配套 [特性矩阵](docs/renderer/feature_matrix.md)、[性能调优](docs/renderer/performance_tuning.md)、[兼容矩阵](docs/renderer/compatibility_matrix.md) 与 SDK bundle（`dist/sdk_bundle/`）。

## 治理与质量门

Rurix 从第一天把治理内建为产品力（AI 时代语言基础设施，见 [`10_GOVERNANCE.md`](10_GOVERNANCE.md)）：

- **规范 ↔ 测试 ↔ PR 三角**：每条 RXS 规范条款 ≥1 测试锚定（`ci/trace_matrix.py`，当前 278/278）。
- **measured_local 预算**：性能 / 诊断基线全部真机实测，零 estimated 占位（`ci/budget_eval.py --strict`）。
- **真实红绿**：每道 CI 门经「构造缺陷 → 红 → 复原 → 绿」验证（反 YAML-only），run URL 归档于 [`evidence/`](evidence/)。
- **字节级 guardrails**、schema 校验、结构校验、conformance 全绿、UI/MIR/PTX/DXIL golden 与 stable API 快照经 bless。
- **deferred / spike-gating 注册表**：延期项与扩张方向唯一事实源，只追加。
- **诚实终态**：门的 no-go / defer / 诚实红一律留档不改写；契约 close-out 只追加，已收口契约既有内容 0-byte 修改（`ci/check_guardrails.py` 机器守卫）。

里程碑契约与 close-out 留痕见 [`milestones/`](milestones/)；治理机制总览见 [`14_ENGINEERING_DISCIPLINE.md`](14_ENGINEERING_DISCIPLINE.md)。

## 克制声明

Rurix **不**取代 CUDA 生态（在其上提供安全编译前端与运行时）、**不**首发跨平台（NVIDIA 单栈做深）、**不**做 ML 框架（与 PyTorch 经 DLPack 零拷贝互操作）。每条克制对应一条已验证的死亡路线（[`03_POSITIONING_AND_LANDSCAPE.md`](03_POSITIONING_AND_LANDSCAPE.md) §4）。

## 文档地图

| 位置 | 内容 |
|---|---|
| [`00_MASTER_INDEX.md`](00_MASTER_INDEX.md) | 总索引：文档清单、阅读路径、术语表、文档维护规则 |
| `01`–`14` | 规划文档集（愿景 / 用户与用例 / 定位 / 设计原则 / 语言架构 / GPU 编程模型 / 编译器架构 / 运行时与工具链 / 标准库与生态 / 治理 / 路线图 / 风险 / 决策日志 / 工程纪律）；[`15_EXTERNAL_ADOPTION_REGISTER.md`](15_EXTERNAL_ADOPTION_REGISTER.md) 为外部采纳登记 |
| [`spec/`](spec/) | 可测试规范（FLS 体例，RXS 条款）；[`conformance/`](conformance/) 为唯一验收边界 |
| [`rfcs/`](rfcs/) | 语言演进 RFC / Mini-RFC 序列（模板、编号台账、FCP-lite 评审窗见 `rfcs/README.md`） |
| [`guide/`](guide/) | 入门教程（中 / 英） |
| [`docs/renderer/`](docs/renderer/) | 渲染器产品文档：集成指南、特性矩阵、性能调优、兼容矩阵、发布检查单、支持策略、vendor 许可矩阵 |
| [`milestones/`](milestones/) | 里程碑契约（四要素）、P2 决策表、close-out 终审签署 |
| [`registry/`](registry/) | 只追加注册表：`deferred.json` / `spike_gating.json` / `error_codes.json` / `number_ledger.json` |
| [`evidence/`](evidence/) | CI 门 evidence（PASS 才落盘；fail-closed） |
| `artifacts/day_*/` | 战役制交付工件：`CAMPAIGN_LOG.md` / `HANDOVER.md` / 各任务 REPORT / evidence |
| [`dist/`](dist/) | SDK bundle（`sdk_bundle/sdk-1.1.0/`）、SBOM、第三方声明、发布说明 |

**阅读路径**：*只有 15 分钟* → 01 → 04 → 13；*评估项目是否靠谱* → 01 → 03 → 12 → 11；*参与语言设计* → 04 → 05 → 06 → 13；*参与编译器实现* → 04 → 07 → 14 → 05。

## 贡献

欢迎贡献。请先读 [`CONTRIBUTING.md`](CONTRIBUTING.md)（规范↔测试↔PR 三角、变更分档、AI 贡献政策、`unsafe` 纪律）与 [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md)；安全问题见 [`SECURITY.md`](SECURITY.md)。

## 许可

双许可，任选其一（D-003）：

- Apache License 2.0（[`LICENSE-APACHE`](LICENSE-APACHE)）
- MIT License（[`LICENSE-MIT`](LICENSE-MIT)）

`SPDX-License-Identifier: MIT OR Apache-2.0`。除非你明确声明，否则你有意提交并纳入本项目的任何贡献，均按上述双许可授权，无附加条款。
