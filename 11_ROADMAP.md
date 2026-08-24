# 11 — 路线图

> 所属文档集：[00_MASTER_INDEX.md](00_MASTER_INDEX.md)
> 版本：v1.0（2026-06-11）
> 基准：单人 + AI 集群（D-004），MVP 12–18 个月。月份为相对刻度（M+n = 启动后第 n 个月），非日历承诺。
> 纪律：每个里程碑按 [14](14_ENGINEERING_DISCIPLINE.md) 走契约制；**每个里程碑必须含至少一项真实硬件证据交付物**（H06 §6 反"纯骨架阶段"规则）。

---

## 1. 总览

```
M0    M1-M2      M3-M4        M5         M6-M7        M8        MVP
基建&  前端&host  MIR&borrow   device     views&       工具链&    收口&
证据   编译闭环    检查         codegen    并行基元      包管理     验收
│──1──│────4─────│─────7──────│────9─────│─────12─────│───15────│──18
                                                      ↓
                              G0 软光栅 demo 穿插于 M7-M8
MVP 后：开源决策点 → G1 interop (+6mo) → 引擎集成 → G2 原生图形 (3年期)
```

## 2. MVP 范围红线（先于里程碑定义）

H06 §5 红线 + 本项目裁剪的合并清单。以下各项 **MVP 明确不做**，全部登记 spike gating（[14](14_ENGINEERING_DISCIPLINE.md) §7）：

| 不做项 | 来源 |
|---|---|
| 自动 kernel fusion / 稀疏数据结构 | H06（Taichi 复杂度黑洞前科） |
| autodiff / 可微渲染 | H06（上一项目 M14.12 只落 stub 的前科） |
| 多后端（AMD/Intel/Metal/Vulkan/SPIR-V） | H06 + 红线 3 |
| 过程宏、声明宏 | H06 + D-111 |
| 包 registry | H06 + D-312（lockfile+vendor+checksum 即 MVP） |
| Python 原生嵌入 | H06 + 红线 1（仅 C ABI/PYD 通道） |
| Tensor Core/WGMMA/TMA intrinsics、cluster、动态并行、cooperative groups | r2 MVP 收缩 |
| Graph API、VMM、多 GPU、流序分配类型（G1 任务） | r4/D-122 |
| 跨会话增量编译、并行前端、Polonius | r1 三大警告 |
| trait 对象、特化、HKT、async | D-104 |

## 3. MVP 里程碑序列

每个里程碑列出：目标、关键交付物、验收门（摘要——完整验收门在各里程碑契约中固化）。

### M0（M+1）：基础设施与证据通道（P-09 的兑现，排第一不可调换）

- **目标**：在写第一行编译器代码之前，建成真实硬件证据通道与工程纪律骨架。
- **交付物**：仓库 + CI（PR smoke 真实跑通，不是 YAML 语法检查——上一项目 D11.8-2 教训）；RTX 4070 Ti 上的 L0/L1 基准 harness（锁频/环境画像/统计协议，r11 协议实现）；**手写 PTX + Driver API 装载的 SAXPY/bandwidthTest 基线**（`measured_local`，这是后续一切性能阈值的锚点）；契约/预算 JSON/deferred 注册表模板（H05 资产改造）；`agents/AGENTS.md` v1。
- **验收门**：SAXPY 基线三次运行 trimmed mean 进预算 JSON；CI 在真实 PR 上绿过一次；NVML 探测器输出完整环境画像。

### M1（M+2 ~ M+3）：词法、语法与诊断地基

- **交付物**：Span/SourceMap/DiagCtxt/错误码注册表（先于 lexer，r1 顺序）；lexer + 手写递归下降 parser（错误恢复）；AST + feature gate 骨架；UI golden 测试框架跑通第一条黄金路径（解析错误）；`rx fmt` 雏形（语法定型即跟进，防风格漂移）。
- **验收门**：语法样例集 100% 解析；UI 测试通道全自动（bless 流程可用）。

### M2（M+3 ~ M+5）：HIR、类型检查与 host 编译闭环

- **交付物**：名称解析、HIR lowering（item/body 分离）；类型推断/检查（host 子集：函数/struct/enum/泛型单态化雏形）；query 风格 API + 进程内 memo（第一天形态，D-203）；MIR 雏形 + LLVM host codegen → COFF → link.exe → **第一个 hello-world EXE + PDB 断点可命中**。
- **验收门**：UI 黄金路径 2（类型错误）上线；hello-world 在 WinDbg 断点验证；`-Z self-profile` 式阶段计时输出。

### M3（M+5 ~ M+7）：MIR 完整化与借用检查

- **交付物**：TBIR 窄门（模式/方法糖/drop scope）；move/init 数据流；NLL 借用检查（host 全量）；const eval MIR 解释器（const 泛型可用）；drop/affine 语义闭环。
- **验收门**：借用检查 conformance 初版（预设错误类别全拦截）；UI 黄金路径 3（借用错误）上线；编译器自身预算项（冷编译/check 延迟）首次实测回填。

### M4（M+7 ~ M+9）：device codegen 与运行时——第一个 Rurix kernel 上 GPU

- **交付物**：着色/地址空间检查；MIR→LLVM(NVPTX 子集)→PTX（`ptx_kernel`/addrspace/sreg intrinsics，r2 第一阶段范围）；ptxas 干验证关卡；运行时 Context/Stream/Buffer/launch（经典内存路径）+ 装载协商 + poisoned 状态机；嵌入 PTX 的单可执行产物。
- **验收门（MVP 中点的硬证据）**：**Rurix 写的 SAXPY 在 RTX 4070 Ti 上 `measured_local` 达到 M0 手写基线 ≥ 95%**；launch 类型契约 conformance；UI 黄金路径 4（目标后端错误）上线。

### M5（M+9 ~ M+11）：views、shared、同步——安全并行的核心交付

- **交付物**：views 算子集 + 不相交证明（borrow check 扩展）；`shared let` + barrier 一致性检查；scoped atomics + PTX 映射层（spec 条款先行，D-406 v2.0 高敏面由 agent 自主经 Full RFC 落笔）；libdevice 链接；gpu 库并行基元（reduce/scan/transpose/tiled GEMM 自研 kernel）。
- **验收门**：L1+L2 基准全量 `measured_local`：**reduce/scan/GEMM-tile ≥ 手写 CUDA C++ 对照 90%**（UC-01 判据）；Compute Sanitizer 全绿纳入 CI nightly。

### M6（M+11 ~ M+13）：工具链与包管理

- **交付物**：`rx` CLI 全核心子命令；`rx test`（GPU 子进程隔离）；manifest/lock/vendor/checksum 包管理（path/git/archive 三来源）；LSP MVP + VS Code 扩展（诊断/补全/跳转/重命名）；Natvis 首批。
- **验收门**：三包 workspace 离线重建逐字节可复现；LSP 在 10k 行样例工程交互延迟达标（预算项实测）。

### M7（M+13 ~ M+15）：标准库充实与 G0 图形演示

- **交付物**：core 数学库定型（Vec/Mat/swizzle/几何原语）；image-io 包；**G0 compute 软光栅**（binning/tile 光栅/深度/tonemap kernel 全 safe 代码目标）；**UC-03 验收 demo：SPH 仿真 + 软光栅出图**；kernel 热重载（`rx watch`）。
- **验收门**：demo 单 EXE 分发、输出图像序列；软光栅 L3 基准入库；safe 覆盖率报告（哪些 kernel 落了 unsafe 及原因→反哺 views 扩展清单）。

### M8（M+15 ~ M+18）：互操作、加固与 MVP 验收

- **交付物**：PYD 产出 + nanobind 模板 + DLPack/`__cuda_array_interface__`（**UC-01 demo：PyTorch 算子替换**）；cublas 包（GEMM/GEMV 三层绑定）；`rurixup` + MSI/winget + 签名/SBOM 链路；文档站（`rx doc` 生成）；全量 conformance/UI/基准回归冻结。
- **MVP 验收门**（= [01](01_VISION_AND_MISSION.md) §6 第一层全量）：UC-01/UC-02/UC-03 三大旗舰用例端到端；L1/L2 性能判据达标；预设资源生命周期错误类别 100% 编译期拦截；全部预算阈值 `measured_local`（**零 estimated 占位**——上一项目最大教训的硬性反转）。

### MVP 完成 → agent 决策点

开源执行（D-003 既定，时点与形式 agent 确认）；G1 启动优先级；是否引入协作者。

## 4. MVP 后 12 个月（G1 期）

| 里程碑 | 内容 |
|---|---|
| G1-1 | CUDA–D3D12 interop（ExternalBuffer/Semaphore 类型化）；软光栅 demo 升级为实时窗口呈现 |
| G1-2 | 流序分配 `AsyncBuffer` 类型契约 + Graph API 评估 |
| G1-3 | **首个引擎集成里程碑**：Rurix DLL 嵌入一个现存 C++/D3D12 渲染框架，承担 compute pass（UC-05 前奏） |
| G1-4 | 开源社区基建：贡献指南/FCP-lite 实体化/首批外部 RFC；生态包第二梯队（geometry/cudnn 评估） |
| 持续 | 生产分发 fatbin（按架构 cubin + PTX fallback）；LSP 中期特性；编译性能（常驻进程/PTX 缓存） |

## 5. 3 年愿景（G2 期）

- **G2 原生图形管线**：D3D12 + DXIL 后端（路径重评估点 D-131）；`vertex/fragment/mesh/task/RT` 着色阶段进语言；绑定布局编译器推导；**UC-04 deferred 渲染器 demo**。
- 语言 1.0：spec 全量条款化 + conformance 覆盖全部 stable 特性 + 首个 edition 机制就绪。**✓ 已正式发行**（2026-07-14,V1 期:三要件于 G2.5 达成 + RFC-0008 §6 stabilization 路径收尾——stabilization report + FCP-lite 公示 [#121](https://github.com/qwasg/Rurix/issues/121) + 最小 stable channel 清单 MR-0008/RXS-0185~0186 + 版号 1.0.0;tag `v1.0.0` 经 release 机器发布门全绿,首个 GitHub Release:<https://github.com/qwasg/Rurix/releases/tag/v1.0.0>;详见 milestones/v1/ 契约与 STABILIZATION_REPORT）。
- 生态成功判据（[01](01_VISION_AND_MISSION.md) §6 第二层）：≥3 个非作者维护的真实项目。
- registry 决策点（D-312）：社区规模驱动。

## 6. 5 年愿景

- 至少一个生产级渲染器/仿真系统以 Rurix 为主语言（使命判据）。**首个第一方候选已落地**（2026-07-15,MS1 期:single-source 宿主 GPU 编排 std::gpu（RFC-0009/RXS-0189~0199,host `.rx` 编排 + 同源 kernel PTX 嵌入单 EXE)+ UC-07 全 `.rx` 应用 ruridrop(RFC-0010,GPU SPH + DDA 路径追踪/光线投射二合一,应用层零 `.rs`、确定性三层 golden、实时真窗口 ~68fps@131k 粒子,判据操作化四条机器审计);详见 milestones/ms1/ 契约 §8。**「外部选择/采纳」维度未宣称达成**,维持 carve-out——本判据的完全达成仍以外部生产项目选择 Rurix 为准)。
- 多后端解禁评估（红线 3 的正式重审——仅当 NVIDIA 单栈纵深完成）。**✓ 红线 3 已解除**（2026-07-15 owner（白栀）主动裁决,D-008 / 13 §8 v2.1,SG-003 → triggered(RFC-0011);承 RFC-0011 = 单一 Vulkan/SPIR-V 跨端后端 mb1(AMD 桌面 + Android,compute+graphics),MB1 期已整体收官 2026-07-16(基准 mb1-closed),唯一存续尾门 G-MB1-6 AMD 真卡验收缺硬件 open。诚实留痕:前提「NVIDIA 单栈纵深完成」先前判定未达,本次系 owner 主动裁决解除,非 agent 宣布前提达成）。
- Tensor Core/Work Graph 级现代 GPU 能力的安全抽象成为研究与工程的参照系。
- 自举（rurixc 用 Rurix 重写）作为语言成熟度的试金石立项评估（非承诺）。

## 7. 节奏与产能纪律（D-004 落地）

- 里程碑两级结构：1–2 周小里程碑 + 6–10 周阶段（上一项目验证粒度，H06 §6）。
- 单人 + AI 的瓶颈在**验证通道与文档税**而非写代码（H06 §6）——因此 M0 前置全部验证基建，文档走单一事实源生成（P-11），AI 产出强制走 [10](10_GOVERNANCE.md) §7 验证规则。
- 时间弹性：12 个月为激进线、18 个月为承诺线；M4（GPU 闭环）与 M5（views）是关键路径，延期优先压缩 M6-M8 的范围而不是质量门。

## 8. 执行期实况补登（G3~G17，2026-08-24 勘误，只作索引镜像）

§3~§6 为规划期原文（按 00 §6.4 转历史档案不回写）。实际执行主线在 G2 之后按「里程碑契约 + 验收门」滚动立项，与规划期的 G1/G2/3 年期代号不再一一对应。截至本勘误日的实况（事实源恒为 `milestones/*/\*_CONTRACT.md` 与 `registry/`，本节只作索引镜像）：

| 里程碑 | 收口 | 主题一句话 |
|---|---|---|
| G1 | 2026-06-22 `g1-closed` | CUDA–D3D12 interop 呈现 + 流序分配 + 引擎集成 DLL |
| G2 | 2026-06-30 `g2-closed` | 着色阶段进类型系统 + DXIL 第二后端（D-131 混合路线）+ 语言 1.0 机制就绪 |
| V1 | 2026-07-14 `v1-closed` | 语言 1.0 首个 stable 发行（tag `v1.0.0`） |
| MS1 | 2026-07-15 `ms1-closed` | `std::gpu` 单源宿主编排 + 首个全 `.rx` 应用 ruridrop |
| MB1 | 2026-07-16 `mb1-closed` | Vulkan/SPIR-V 跨端后端（AMD 桌面 + Android；G-MB1-6 AMD 真卡尾门诚实 open） |
| G3 | 2026-07-19 `g3-closed` | 工业渲染期（采样超集/bindless/render graph 自动 barrier 等五特性面） |
| EI1 | 2026-07-23 `ei1-closed` | 引擎集成期（UC-05 最小 RHI + C ABI 导出 codegen） |
| G4 | 2026-07-24 `g4-closed` | 引擎渲染期（图形 RHI 化 + engine_host v3 + BLACKHOLE 生产档验收） |
| EA1 | 2026-07-28 `ea1-closed` | 分发与门面期（rurixup 真实分发 + `v1.0.1-dist` 预编译 bundle） |
| G5 | 2026-07-29（契约 §8.1） | 原生渲染器期（声明式 render graph + 虚拟化几何 + VSM + GI + 时域重建） |
| G6 | 2026-08-01（契约 §8.2） | 渲染物理双轨期（Jolt 生产默认 + Rapier 快路径 + Taichi AOT 副轨） |
| G7 | 2026-08-05 `g7-closed` | Production Frame Closure（RD-038 closed + One True Device Frame） |
| G8 | 2026-08-06（契约 close-out） | UE5 级前置能力完成期 |
| G9 | 2026-08-15（契约 §8.10） | UE5 目标渲染/物理平台期 |
| G10 | 2026-08-16（契约 §8.10） | UE5 画面对标基线期（差距清单 11 行闭集 → G11 法定输入） |
| G11 | 2026-08-17（契约 §8.8） | GI 与光照画质闭环期（复测清单 converged 8 + aligned_closed 3） |
| G12 | 2026-08-17（契约 §8.8） | 路径追踪生产化期（差距清单 10 行终态 → G13 法定输入） |
| G13 | 2026-08-19（契约 §8.9） | 超分采样与 Lumen 对照期（DLSS/FSR/TSR 三后端 + 双差距登记表 8+2 行） |
| G14 | 2026-08-23（契约 §8.13） | 正式帧率对标与渲染管线性能期（M-d 18/18 达标，通过线 ×1.00） |
| G15 | 2026-08-23（契约 §8.9） | 画质量级收口与商用终审期（商用 0/18 + 性能 17/18 双未达标如实定盘，承接锚三面 → G16+） |
| G16 | 2026-08-24 `g16-closed` | UE 参照臂修复与画质强制收口期（含 G16plus：M-g 绝对画质 18/18，M-c 历史 0/18 未改写） |
| G17 | **进行中**（active/unlocked） | DLSS 性能缺口收口期（G15-MD-F1 字面兑现；2026-08-24 落笔快照：G17.2 M-a 已验收，后续波次同日推进中，波次终态以契约 §8 为准；终判两态均为合法收口） |

## 9. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-11 | 初版 |
| v1.1 | 2026-07-14 | §5「语言 1.0」行追加式勘误(00 §6.3 独立规划文档 PR;数字可追溯 00 §6.6):标注已正式发行——V1 期收尾(stabilization report + FCP-lite #121 + MR-0008 channel 清单 + 版号 1.0.0),tag `v1.0.0` 经 release 机器发布门全绿,首个 GitHub Release 链接与 milestones/v1/ 指针。既有行 0-byte,不新增里程碑正文、不改 §6 5 年愿景(post-G2 期规划仍前瞻未动) |
| v1.2 | 2026-07-15 | 规划文档勘误(00 §6.3,独立 PR):§6 使命判据行追加首个第一方候选落地标注(MS1 期,RFC-0009/RFC-0010,ruridrop;外部采纳维度未宣称达成,carve-out 维持,MS1_CONTRACT §8.5 口径);既有行 0-byte |
| v1.3 | 2026-07-17 | 规划文档勘误(00 §6.3,独立 errata PR,check_planning_docs advisory 不阻断):§6「多后端解禁评估」行追加式标注红线 3 已解除(2026-07-15 owner 主动裁决,D-008 / 13 §8 v2.1,SG-003 → triggered(RFC-0011));承 RFC-0011 = 单一 Vulkan/SPIR-V 跨端后端 mb1(AMD 桌面 + Android,compute+graphics),MB1 期已整体收官(2026-07-16,基准 mb1-closed),唯一存续尾门 G-MB1-6 AMD 真卡 open;当前 EA1 期。既有行 0-byte,§2 MVP 红线表历史记录不改,只刷状态不改决策语义 |
| v1.4 | 2026-08-24 | 规划文档勘误(00 §6.3 追加式修订,独立勘误 commit):新增 §8「执行期实况补登(G3~G17)」索引镜像表(G1~G16 共 21 个执行期里程碑收口实况 + G17 进行中,数字与日期取自各契约 close-out 段与 git tag 命令输出);原 §8 修订记录顺延为 §9(承 00 v1.1「顺延」先例)。§1~§7 规划期正文 0-byte,不改任何决策语义;里程碑事实源恒为 `milestones/` 与 `registry/`,本表只作索引镜像 |
