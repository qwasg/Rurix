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

## 项目状态:语言 1.0 已发行(`v1.0.0`),截至 G18 共 32 个里程碑收口(最新 tag `g18-closed`),G19+ 方向待立项

第一层全量验收(01 §6)已达成,使命判据第一期(11 §6)已落地——首个以 Rurix 为主语言的生产级渲染器/仿真系统(第一方);多后端新纪元第一期(MB1)、工业渲染期(G3)、引擎集成期(EI1)、引擎渲染期(G4)、分发与门面期(EA1)、原生渲染器期(G5)、渲染物理双轨期(G6)、生产帧收口期(G7)、UE5 级前置能力完成期(G8)、UE5 目标渲染/物理平台期(G9)、UE5 画面对标基线期(G10)、GI 与光照画质闭环期(G11)、路径追踪生产化期(G12)、超分采样与 Lumen 对照期(G13)、正式帧率对标与渲染管线性能期(G14)、画质量级收口与商用终审期(G15)、UE 参照臂修复与画质强制收口期(G16 含 G16plus)相继收口。从 MVP 到 1.0 再到 UE5 对标主线,30 个里程碑契约按各自验收门收口;当前 **G17(DLSS 性能缺口收口期,G15-MD-F1 字面兑现,`milestones/g17/`)** `status: active` / `implementation_status: unlocked`,G17.2 M-a 已验收,后续波次同日战役会话推进中——波次终态以契约 §8 只追加记录为准(详见下方 2026-08-24 G17 勘误行);既有性能阈值均以 `measured_local` 留证且零 estimated（G6 明确未设置性能硬门）,预设资源生命周期错误类别 100% 编译期拦截。下表为 G8 期历史快照,其后进展以下方「状态勘误」只追加行为准:

| 阶段 | 收口 | 交付 |
|---|---|---|
| M0–M8(MVP) | 2026-06-17 `m8-closed` | 编译器/运行时/工具链闭环 + UC-01/02/03 三旗舰 + cublas 绑定 + 发布链路 + 双语诊断/文档站 |
| G1 | 2026-06-22 `g1-closed` | CUDA–D3D12 interop 实时呈现、流序分配 `AsyncBuffer<'stream,T>`、引擎集成 DLL(C ABI)、fatbin 生产分发、geometry 库 |
| G2 | 2026-06-30 `g2-closed` | 着色阶段进类型系统、DXIL 第二后端(D-131 混合路线)、绑定布局推导(root signature)、D3D12 运行时 + UC-04 deferred 渲染器、语言 1.0 机制就绪(edition "2026" + stable 面快照冻结) |
| V1 | 2026-07-14 `v1-closed` | 语言 1.0 首个 stable 发行(tag `v1.0.0`):stabilization report、FCP-lite 公示、stable channel 清单(rurixup)、首个 GitHub Release |
| MS1 | 2026-07-15 `ms1-closed` | `std::gpu` 单源宿主编排(单源 `.rx` → 单 EXE)+ 首个全 `.rx` 应用 ruridrop(UC-07) |
| MB1 | 2026-07-16 `mb1-closed` | 单一 Vulkan/SPIR-V 跨端后端(RFC-0011;AMD 桌面 + Android,compute+graphics;Android 真机 on-device measured;AMD 真卡尾门 G-MB1-6 诚实维持 open 待硬件;preview、feature 默认关闭) |
| G3 | 2026-07-19 `g3-closed` | 工业渲染期:RD-027 毒径归因闸门 + 五特性面全量落地(采样超集 / bindless / render graph 自动 barrier / UC-04 窗口 present / mesh-task-RT 双后端) |
| EI1 | 2026-07-23 `ei1-closed` | 引擎集成期:UC-05 最小 RHI + render graph 核心(U5 旗舰用例)+ RD-009 `#[export(c)]` C ABI 导出 codegen 与内建头文件生成(D-113) |
| G4 | 2026-07-24 `g4-closed` | 引擎渲染期:图形 RHI 化 raster/mesh 库面 + 自动 barrier + engine_host v3 嵌入 + `.rx` 单源 Vulkan RHI 通道 + BLACKHOLE 生产档验收(RD-036 open 存续) |
| EA1 | 2026-07-28 `ea1-closed` | 分发与门面期:rurixup 真实分发(RD-025 兑现)+ 预编译工具链 bundle(`v1.0.1-dist` 系列,pre-release)+ 文档门面 + 冷启动验收 |
| G5 | 2026-07-29 收口(契约 §8.1) | 原生渲染器期:声明式 render graph(`rurix-render`)+ RHI 图形派发桥 + 虚拟化几何(meshlet/GPU 两级剔除/VisBuffer)+ VSM 阴影 + 屏幕探针 GI + 光追效果 + 材质流送 + 时域重建(TAA/TSR);UC-06 全管线 demo device 真跑(P3+ 长线项登记 RD-037+ 存续,RD-038 分波兑现推进) |
| G6 | 2026-08-01 收口(契约 §8.2) | 渲染物理双轨期:Jolt 生产默认物理库 + Rapier 默认关闭快路径 + Physics→GpuScene 单向桥 + uc08 合流 demo + Taichi Vulkan AOT 特效副轨;性能数字留 evidence 但不设硬门 |
| G7 | 2026-08-05 `g7-closed` | Production Frame Closure 收口:RD-038 closed(compute SPIR-V 1.4/RayQuery、W3 GI/RTAO/硬阴影、VisBuffer SW/HW diff=0、字面余项与 One True Device Frame + soak);见 [`milestones/g7/G7_CONTRACT.md`](milestones/g7/G7_CONTRACT.md) §8.1 |
| G8 | 2026-08-05 `status: active` / **`implementation_status: unblocked`**(2026-08-02 立项时为 `blocked`) | UE5 级前置能力完成期。G8.1 治理波已交付:契约四件套 + 候选决策表 + 18 个 P0 验收映射 + RFC-0019/0020/0021(均 Agent Approved,D-409 独立 provenance 评审)+ RTX 4070 Ti measured baseline + 三个治理 validator。**G8.2 实现门已开**:G7 `closed`(`g7-closed`)+ RD-038 `closed`,`py -3 ci/check_g8_implementation_interlock.py --require-ready` 输出 `VERDICT = READY`,凭据逐字见 [`G8_CONTRACT.md`](milestones/g8/G8_CONTRACT.md) §8.1。**门已开 ≠ 门已绿**:21 个 P0/P1 symbolic gate 当前一件未 materialize,零 G8 实现能力可宣称为绿 |

> **状态勘误(2026-08-08,只追加不回写)**:上方状态标题「G8 仅治理波立项」与 G8 行「门已开 ≠ 门已绿,零 G8 实现能力可宣称为绿」均为 2026-08-05 时快照——**G8 已于 2026-08-06 收口**(`status: closed`,close-out flip commit `b4189e79`);G8.2~G8.8 各波已实施并验收,各 P0/P1 门终态(含 no-go/defer/诚实降级留档)以 [`milestones/g8/G8_CONTRACT.md`](milestones/g8/G8_CONTRACT.md) 最新 close-out 段与 `registry/` 为准,旧行按只追加纪律不回写。

> **状态勘误(2026-08-09,只追加不回写)**:上方表格止于 G8 行为 2026-08-08 时快照——**G9(UE5 目标渲染/物理平台期,`milestones/g9/`)已于 2026-08-09 立项**:`status: active` / `implementation_status: blocked`,**只有 G9.1 治理波获授权并已交付**:契约四要素 + 候选决策表(RD-039/040/041/044 全 open 分项逐行 go/no-go/strategic_override)+ 15 个 P0 验收映射 + 伞形 RFC-0022/0023/0024(均 Agent Approved,D-409 独立 provenance 评审,RFC-0024 为 RFC-0021 修订)+ RTX 4070 Ti VRAM/AS measured baseline(`g9_budget.json` 非空)+ 三个治理 validator。**G9.2+ 由 G-G9-3 事实互锁硬阻断**:`py -3 ci/check_g9_implementation_interlock.py` 当前 `VERDICT = BLOCKED`(治理完成 ≠ 实现开工),门开≠门绿口径不变。里程碑事实源恒为 `milestones/g9/` 与 `registry/`,旧行按只追加纪律不回写。

> **状态勘误(2026-08-15,只追加不回写)**:上方 2026-08-09 勘误行状态止于 G9 治理波立项快照——**G9 已于 2026-08-15 收口**(`status: closed`,close-out flip commit 见 `milestones/g9/G9_CONTRACT.md` §8.10):G9.2~G9.6 五波、G9.7 P2 穷举决策(33 行闭集)、G9.8a stabilization soak(≥30min/≥10000 帧 honest 口径)与 G9.8b close-out 终审(VERDICT=READY)逐波验收,15 个 P0 与 19 个已 go P1 独立断言全绿;各门终态(含 no-go/defer-to-G10+/not-triggered 诚实留档,不回写为 PASS)以 [`milestones/g9/G9_CONTRACT.md`](milestones/g9/G9_CONTRACT.md) 最新 close-out 段、`G9_P2_DECISIONS.md` 与 `registry/` 为准,旧行按只追加纪律不回写。

> **状态勘误(2026-08-16,只追加不回写)**:上方 2026-08-15 勘误行状态止于 G9 收口快照——**G10(UE5 画面对标基线期,`milestones/g10/`)已于 2026-08-16 收口**(`status: closed`,close-out flip commit 见 [`milestones/g10/G10_CONTRACT.md`](milestones/g10/G10_CONTRACT.md) §8.10):G10.2~G10.5 四波(UE5 5.8 出图环境/压测语料/度量基建/首轮 A/B 对比)、G10.6 defer 重评窗(M99-clipmap 唯一 rejudged-go 指定 G11 画质修复期承接)、G10.7 P2 穷举决策(27 行闭集)、G10.8a stabilization soak(出图→捕获→度量→差距清单全链路连续复跑 ≥30min honest 口径)与 G10.8b close-out 终审(VERDICT=READY)逐波验收,12 个 P0 与 2 个已 go P1 独立断言全绿;**差距清单 11 行闭集(R1~R5/U1~U3/C1~C3)终审锁定为 G11 法定输入**(G11 修复范围只消费该清单 + 承接锚);各门终态(含 no-go/defer-to-G11+/not-triggered 诚实留档,不回写为 PASS)以 [`milestones/g10/G10_CONTRACT.md`](milestones/g10/G10_CONTRACT.md) 最新 close-out 段、`G10_P2_DECISIONS.md` 与 `registry/` 为准,旧行按只追加纪律不回写。

> **状态勘误(2026-08-17,只追加不回写)**:上方 2026-08-16 勘误行状态止于 G10 收口快照——**G11(GI 与光照画质闭环期,`milestones/g11/`)已于 2026-08-17 收口**(`status: closed`,close-out flip commit 见 [`milestones/g11/G11_CONTRACT.md`](milestones/g11/G11_CONTRACT.md) §8.8;G11.1 于 2026-08-16 立项):G11.2 口径差对齐(M144~M146/M157)、G11.3 资产与场景修复(M147 判据双 phase 修订/M148~M152)、G11.4 光照与 GI 修复(M153 R3 灯种子集/M154 R4 多反弹 GI + M99-clipmap 世界级承接)、G11.5/G11.5b A/B 复测(首跑 R1 不收敛整波 FAIL 停线 → 诊断修复 --sky-ibl〔RXS-0397〕→ R1 真实收敛 0.8328980787837229 → 0.6655959582429252 + 11 行全闭环)、G11.6 P2 穷举决策(28 行闭集)、G11.7a stabilization soak(19 门全量回归真跑 + 修复链路连续复跑 29 迭代 1859.9s ≥1800s honest 口径)与 G11.7b close-out 终审(VERDICT=READY)逐波验收,13 个 P0 与 1 个已 go P1 独立断言全绿;**复测差距清单 11 行终态(converged 8 + aligned_closed 3 + partial 0)终审锁定**——残余差距五元归属如实登记不冒充全闭环;各门终态(含 no-go/defer-to-G12+/not-triggered 诚实留档,不回写为 PASS)以 [`milestones/g11/G11_CONTRACT.md`](milestones/g11/G11_CONTRACT.md) 最新 close-out 段、`G11_P2_DECISIONS.md` 与 `registry/` 为准,旧行按只追加纪律不回写。

> **状态勘误(2026-08-17,只追加不回写)**:上方 2026-08-17 勘误行状态止于 G11 收口快照——**G12(路径追踪生产化期,`milestones/g12/`)已于 2026-08-17 收口**(`status: closed`,close-out flip commit 见 [`milestones/g12/G12_CONTRACT.md`](milestones/g12/G12_CONTRACT.md) §8.8;G12.1 于 2026-08-17 立项):G12.2 生产化核心波(M158~M161 四 P0 + M166 P1 标定入 budget)、G12.3 降噪波(M162 降噪管线 + TSR 联动)、G12.4 UE PT 对标波(M163 双端对拍 + M164 生产化回归门)、G12.5 性能面波(M165 PT 吞吐基线 50×3 协议)、G12.6 P2 穷举决策(33 行闭集:go 5 closed-go + no-go 6 + defer-to-G13+ 22)、G12.7a stabilization soak(14 门全量回归真跑 + PT 生产化链路连续复跑 33 迭代 1813.6s ≥1800s honest 口径)与 G12.7b close-out 终审(VERDICT=READY)逐波验收,8 个 P0 与 1 个已 go P1 独立断言全绿;**生产化差距清单 10 行终态(quality_gap 6 + caliber_diff 4)终审锁定为 G13 法定输入**——残余差距/未闭环行如实登记不冒充全闭环;各门终态(含 no-go/defer-to-G13+/not-triggered 诚实留档,不回写为 PASS)以 [`milestones/g12/G12_CONTRACT.md`](milestones/g12/G12_CONTRACT.md) 最新 close-out 段、`G12_P2_DECISIONS.md` 与 `registry/` 为准,旧行按只追加纪律不回写。

> **状态勘误(2026-08-23,只追加不回写)**:上方 2026-08-19 勘误行状态止于 G13 收口快照——**G14(正式帧率对标与渲染管线性能期,`milestones/g14/`)已于 2026-08-23 收口**(`status: closed`,close-out flip commit 见 [`milestones/g14/G14_CONTRACT.md`](milestones/g14/G14_CONTRACT.md) §8.13;G14.1 于 2026-08-19 立项,G14plus G14.8~G14.12 延续波完成硬收尾):5 个 P0(M-a~M-e)全绿,M-d 18/18 达标(通过线 ×1.00;soak 回归最紧格 bistro t100 dlss ratio 1.0831),帧率差距登记表空表终态,digest 锚 18 格程序重收割三证齐,G14.5a soak 58 迭代 1835.7s ≥1800s 零失败,M-h 6/6 PASS,G14.5b close-out `VERDICT=READY`;RD-045 维持 open(长窗归 G15+);G13 超分/Lumen 8+2 行只消费不回写(绝对画质通过线归 G15)。各门终态以 [`milestones/g14/G14_CONTRACT.md`](milestones/g14/G14_CONTRACT.md) §8.13、[`G14PLUS_RECORD.md`](milestones/g14/G14PLUS_RECORD.md) §5/§6 与 `registry/` 为准,旧行按只追加纪律不回写。
> **状态勘误(2026-08-24,只追加不回写)**:上方同日 G16.1~G16.5 勘误行状态止于 `status: active` / 商用 0/18——**G16plus 已收口**(`status: closed`,close-out flip 见 [`milestones/g16/G16_CONTRACT.md`](milestones/g16/G16_CONTRACT.md) §8.4;tag `g16-closed`):M-e GI 表达 8 facts 绿 + M-f Lumen 重收割绿 + M-g 绝对画质 18/18（阈程序产 p100×2.0，M-c 历史 0/18 未改写）+ soak 56 迭代 1835.136s ≥1800s 零失败 + close-out 八 facts `VERDICT=READY`。各门终态以 [`milestones/g16/G16_CONTRACT.md`](milestones/g16/G16_CONTRACT.md) §8.4、[`G16PLUS_RECORD.md`](milestones/g16/G16PLUS_RECORD.md) 与 `registry/` 为准,旧行按只追加纪律不回写。
> **状态勘误(2026-08-24,只追加不回写)**:上方同日 G16.1 勘误行状态止于 `implementation_status: blocked`——**G16 第一波实现(M-a~M-d,步骤 284~287)已兑现 G15-MC-F1**：UE cornell 参照臂不再死黑；商用收口仍未达标 0/18 如实；G13/G15 冻结表与已收口 evidence 0-byte；里程碑保持 `status: active` / `implementation_status: unlocked`，本波不做 G16 close-out。各门终态以 [`milestones/g16/G16_CONTRACT.md`](milestones/g16/G16_CONTRACT.md) §8.2 与 `registry/` 为准,旧行按只追加纪律不回写。
> **状态勘误(2026-08-24,只追加不回写)**:上方 2026-08-23 勘误行状态止于 G15 收口快照——**G16(UE cornell 参照臂修复与受影响门重测,`milestones/g16/`)已于 2026-08-24 立项 G16.1 治理波**：`status: active` 且 `implementation_status: blocked`；契约三件套 + 候选决策 20 行 + 4 个 P0 映射 + 治理三门步骤 281~283 落盘；G16.2+ 由 `ci/g16_interlock_check.py --require-ready` 硬阻断。本波只承接 G15-MC-F1。各门终态以 [`milestones/g16/G16_CONTRACT.md`](milestones/g16/G16_CONTRACT.md) 与 `registry/` 为准,旧行按只追加纪律不回写。
> **状态勘误(2026-08-23,只追加不回写)**:上方 2026-08-23 勘误行状态止于 G14 收口快照——**G15(画质量级收口与商用终审期,`milestones/g15/`)已于 2026-08-23 收口**(`status: closed`,close-out flip commit 见 [`milestones/g15/G15_CONTRACT.md`](milestones/g15/G15_CONTRACT.md) §8.9;G15.1 于 2026-08-23 立项):G15.2 测量重收割波(M-a 双端画质对拍链路全量复跑 + 20 行登记表逐项重评)、G15.3 修复闭环波(M-b 三态处置 0/4/16 + 材质链评估 not-triggered 未命中)、G15.4 绝对画质终审波(M-c 通过线程序产标定 + 18 格 AI 读图 + 商用收口 0/18 如实定盘)、G15.5 性能零降级波(M-d 复跑 17/18 诚实红不冒充 + G15plus 双延续波诊断攻坚)、G15.6a P2 穷举(40 行闭集:closed-go 24 + defer-to-G16+ 16)+ M-e 回归门(84 门零降级 + RD-045 零检出)+ soak(59 迭代 1852.5s ≥1800s 零失败)与 G15.6b close-out 终审(VERDICT=READY)逐波验收,5 个 P0 独立断言全绿(M-d 诚实红特判 = 未达标如实登记不充绿亦不充降级);**双未达标终审定盘如实登记不冒充**(商用收口 0/18 + 性能 17/18 单格环境事件面),G16+ 承接锚三面齐备(GI 表达面 + UE 参照臂修复 / DLSS NGX 版本与宿主车道 / 绝对画质 deficit 收口——用户 2026-08-19 授权面承接,G16+ 里程碑继续优化面开放);各门终态(含 no-go/defer-to-G16+/诚实红留档,不回写为 PASS)以 [`milestones/g15/G15_CONTRACT.md`](milestones/g15/G15_CONTRACT.md) §8.9、`G15_P2_DECISIONS.md` 与 `registry/` 为准,旧行按只追加纪律不回写。

> **状态勘误(2026-08-19,只追加不回写)**:上方 2026-08-17 勘误行状态止于 G12 收口快照——**G13(超分采样与 Lumen 对照期,`milestones/g13/`)已于 2026-08-19 收口**(`status: closed`,close-out flip commit 见 [`milestones/g13/G13_CONTRACT.md`](milestones/g13/G13_CONTRACT.md) §8.9;G13.1 于 2026-08-18 立项):G13.2 vendor 超分接入波(M-a:DLSS SR 经 Streamline SDK 2.10.3 NGX Vulkan interop 臂真跑出帧 + FSR 3.1.5 同接口档 + 三后端运行时切换)、G13.3 TSR device 化波(M-b:tsr.rs host 金标准 → .rx kernel device 面双 kernel SPV + device vs host 逐帧对拍)、G13.4 UE 对拍波(M-c UE DLSS 插件 MRQ 臂 vs Rurix 三后端 18 格双端对拍 + M-d UE Lumen GI 双臂对照,双差距登记表 8+2 行落盘)、G13.5a P2 穷举决策(31 行闭集:go 6 closed-go + no-go 1 + defer-to-G14+ 24)+ M-e 回归门(71 门零降级 + M165 漂移零检出)、G13.5a stabilization soak(9 门全量回归真跑 + 超分链路连续复跑 409 迭代 1805.97s ≥1800s 零失败 honest 口径;UE 厂商随机运行间方差事件四跑取证 + 双登记表再锚定,结构性修复承接锚入 G14 法定输入)与 G13.5b close-out 终审(VERDICT=READY)逐波验收,5 个 P0 独立断言全绿;**超分/Lumen 双差距登记表 8+2 行终态(全 quality_gap/P2)终审锁定为 G14/G15 法定输入**——残余差距/未闭环行如实登记不冒充全闭环;各门终态(含 no-go/defer-to-G14+/not-triggered 诚实留档,不回写为 PASS)以 [`milestones/g13/G13_CONTRACT.md`](milestones/g13/G13_CONTRACT.md) 最新 close-out 段、`G13_P2_DECISIONS.md` 与 `registry/` 为准,旧行按只追加纪律不回写。

> **状态勘误(2026-08-24,只追加不回写)**:上方各勘误行状态止于 G16plus 收口(本区物理行序非时间序,时间序以各行日期为准)——**G17(DLSS 性能缺口收口期,G15-MD-F1 字面兑现,`milestones/g17/`)已于 2026-08-24 立项并解锁实现**:G17.1 治理波交付契约四件套 + 候选决策 19 行零空行 + 5 个 P0 验收映射 + RFC-0032(D3D12 宿主 NGX 车道 Full RFC,D-409 对抗评审后 Agent Approved,终态 disposition 待 G17.4 M-c)+ 治理三门步骤 293~295;互锁 `py -3 ci/g17_interlock_check.py` VERDICT=READY → `implementation_status: unlocked`(契约 §8.1);G17.0 measured baseline = G14 M-d 门同口径一轮 **17/18 诚实红**(焦点格 bistro-interior/t100/dlss_sr ratio 0.9810);实现面全门 materialize(步骤 296~308,五 P0 + 波聚合 + P2/soak/close-out);**G17.2 M-a 双端复测与暖态重标定波已验收**(十 facts 全绿 + wave2 聚合绿,复测窗四轮焦点格 ratio [0.981, 0.8157, 0.7966, 0.8086] 登记面——达标判定归 M-d 终判门,6 UE 格暖态包络条目程序产入 `g17_budget.json`,契约 §8.2)。**本行落笔(2026-08-24,G17.2 验收后)时 G17.3 M-b(NGX 310.6.0+ 演进对齐)/G17.4 M-c(RFC-0032 终态兑现)/G17.5 M-d(t100 终判 18 格)/G17.6 M-e(旧门零降级)/G17.7a P2 穷举 + soak/G17.7b close-out 尚未验收**(同日战役会话继续推进,后续波次终态以契约 §8 只追加记录为准),性能单格未达标维持如实登记不冒充;终判两态(≥×1.00 → 18/18 或物理不可达维持未达标登记)均为合法收口(契约 §7 立项裁决 5)。各门终态以 [`milestones/g17/G17_CONTRACT.md`](milestones/g17/G17_CONTRACT.md) §8 与 `registry/` 为准,旧行按只追加纪律不回写。

> **状态勘误(2026-08-24,只追加不回写)**:上方同日 G17 勘误行状态止于 G17 收口——**G18(全量方向一次性收口期:光线画质+性能+虚拟化几何+帧生成,`milestones/g18/`)已于 2026-08-24 收口**(`status: closed`,close-out flip 见 [`milestones/g18/G18_CONTRACT.md`](milestones/g18/G18_CONTRACT.md) §8.7;tag `g18-closed`):九 P0 全绿(M-a/M-b 加性 profile 光照纵深+双 profile 出图;M-c UE 臂;M-d 商业化画质终审;M-e SL **not-available**;M-f **17/18 诚实红** ratio 0.856326;M-g mesh shader **no-go**;M-h 帧生成 **defer-to-G19+**;M-i 旧门零降级)+ P2 穷举 **25 行零空行**+ soak(49 迭代 1821.0s ≥1800s 零失败)+ close-out 终审(**八 facts VERDICT=READY**);RD 八条 open 维持;G19+ 承接锚(defer 行字面)。各门终态以 [`milestones/g18/G18_CONTRACT.md`](milestones/g18/G18_CONTRACT.md) §8.7、`G18_P2_DECISIONS.md` 与 `registry/` 为准,旧行按只追加纪律不回写。

> **状态勘误(2026-08-24,只追加不回写)**:上方同日 G17 勘误行状态止于 G17.2 M-a 验收——**G17 已于 2026-08-24 收口**(`status: closed`,close-out flip 见 [`milestones/g17/G17_CONTRACT.md`](milestones/g17/G17_CONTRACT.md) §8.7;tag `g17-closed`):G17.3 M-b(NGX 310.6.0 换版评估——**310.6.0 在 Streamline 2.10.3 pin 下 DLSSContext 兼容性不可用,拒绝换版如实登记**,A 臂 X2 边际 in-stream 2.224ms 新鲜分解留档,新 finding G17-MB-F1)、G17.4 M-c(RFC-0032 §5 决策树终态 = **defer** 分支③归因无法分离,单 device 化结构性 no-go 留档,D3D12 宿主车道零实现 = 合法终态)、G17.5 M-d(终判轮 18 格全协议:**ratio 终值 0.856326,维持未达标登记不冒充 17/18**——期窗五轮 0.7966~0.9810 定盘,G15 物理不可达定论在本窗环境下维持,兜底字面逐字兑现)、G17.6 M-e(旧门零降级 18/18:G16 九门 + 六 latest + G15 M-d 诚实红终态维持面 + g17_ 前缀零抢占)、G17.7a P2 穷举(**21 行闭集零空行**:closed-go 5〔G15-MD-F1 承接锚三件套程序全要素兑现完结 + G17-N1~N4〕+ defer-to-G18+ 16)+ soak(49 迭代 1914.487s ≥1800s 零失败 active==wall)与 G17.7b close-out 终审(**八 facts 全绿 VERDICT=READY**)逐波验收;RD 八条 open 维持(RD-045 六轮 digest 守护零检出不判 closed);G18+ 承接锚三面齐备(SL 运行时升级换版程序〔G17-MB-F1〕/ 宿主差可分离证据〔RFC-0032 v0.3〕/ UE 暖态包络演进重标定〔M-a 包络条目〕——用户 2026-08-19 授权面逐字承接,画质面已于 G16plus 18/18 达标定盘,性能面单格承接锚顺延)。各门终态(含 defer/诚实红留档,不回写为 PASS)以 [`milestones/g17/G17_CONTRACT.md`](milestones/g17/G17_CONTRACT.md) §8.7、`G17_P2_DECISIONS.md` 与 `registry/` 为准,旧行按只追加纪律不回写。

> **状态勘误(2026-08-24,只追加不回写)**:上方同日 G18 勘误行状态止于 G18 收口——**G19(帧生成独立层兑现期,七期串行战役第一期,`milestones/g19/`)已于 2026-08-24 收口**(`status: closed`,close-out flip 见 [`milestones/g19/G19_CONTRACT.md`](milestones/g19/G19_CONTRACT.md) §8.7;tag `g19-closed`):五 P0 全绿(M-a FG/MFG host 参考臂 **implemented**——×2/×3/×4 三档逐帧 SSIM(interp)>SSIM(frame-hold) 程序产对照阈 + 双跑位级 + 真实渲染帧率口径 0-byte/presented 独立登记面,G13-N7 兑现即 G18 唯一 M 级 defer 消化;M-b vendor 三臂 disposition——fsr3_fg **rejected**/dlss_g **not_available**/sl_310_6_0 **not_available** 均有 rationale+证据锚,310.5.2 生产默认维持;M-c RD-045 长窗观察 **maintain-open**——canonical 160 帧 12/12 中锚零漂移,backfill 三件未全齐不冒充 close;M-d fps 重评窗登记 **17/18 诚实红 carry** 终判归 G25;M-e G18 链旧门零降级)+ P2 穷举 **14 行零空行**(closed-go 6 + defer-to-G20+ 8)+ soak(63 迭代 1832.1s ≥1800s 零失败,含 FG 车道穿插 12 次)+ close-out 终审(**八 facts VERDICT=READY**);cargo test -p rurix-render --lib 481 passed(framegen/hzb/restir_reservoir 新增 16 测全绿);G20 承接锚齐备(M61 重判窗 HZB 半边预制件已在树)。各门终态以 [`milestones/g19/G19_CONTRACT.md`](milestones/g19/G19_CONTRACT.md) §8.7、`G19_P2_DECISIONS.md` 与 `registry/` 为准,旧行按只追加纪律不回写。

> **状态勘误(2026-08-24,只追加不回写)**:上方同日 G19 勘误行状态止于 G19 收口——**G20(虚拟化几何 P4 期,七期串行战役第二期,`milestones/g20/`)已于 2026-08-24 收口**(`status: closed`,close-out flip 见 [`milestones/g20/G20_CONTRACT.md`](milestones/g20/G20_CONTRACT.md) §8.7;tag `g20-closed`):五 P0 全绿(M-a HZB 遮挡剔除 host 参考臂 **implemented**——geometry 头注「HZB 两阶段 P3 预留」第一阶段兑现,双约定 800 rect 保守零假阳性硬不变量 + 剔除率 231/800 + 双跑位级;M-b cluster 流送 P4 **defer**——差距闭集四行全 open 如实登记;M-c M61 mesh shader **maintain-no-go**——HZB 半边兑现 + cluster 半边未清零 + HW 性能差 measured 证据缺,RFC-0034 重判记录只追加;M-d M98-l4 **维持三级链**——HLOD 接口面就绪命中 + L4 计数可测未命中;M-e G19 链旧门零降级)+ P2 穷举 **14 行零空行**(closed-go 5 + maintain-no-go 2 + defer-to-G21+ 7)+ soak(67 迭代 1845.0s ≥1800s 零失败,含 HZB 车道穿插 13 次)+ close-out 终审(**八 facts VERDICT=READY**);cargo test --lib 486 passed;G21 承接锚齐备(M100-high 重判窗 ReSTIR 预制件在树 + SER capability vulkaninfo 预测得)。各门终态以 [`milestones/g20/G20_CONTRACT.md`](milestones/g20/G20_CONTRACT.md) §8.7、`G20_P2_DECISIONS.md` 与 `registry/` 为准,旧行按只追加纪律不回写。

> **状态勘误(2026-08-24,只追加不回写)**:上方同日 G20 勘误行状态止于 G20 收口——**G21(光照 P3+ 深化期,七期串行战役第三期,`milestones/g21/`)已于 2026-08-24 收口**(`status: closed`,close-out flip 见 [`milestones/g21/G21_CONTRACT.md`](milestones/g21/G21_CONTRACT.md) §8.7;tag `g21-closed`):五 P0 全绿(M-a ReSTIR 高档 reservoir **implemented**——M100-high「高档 reservoir 证据齐备」兑现,等验证预算方差收益 **15.955×** + 时域合并再收益 **7.27×** + 无偏 3σ + 双跑位级,低档 MegaLights 生产默认面 0-byte;M-b M52 SER 重判 **maintain-defer**——capability 半边 vulkaninfo 实测 **available**〔VK_NV/EXT_ray_tracing_invocation_reorder 三 token 取证〕+ workload 半边未命中〔RT pipeline/SBT 车道零实现〕;M-c RD-040 五分项全 defer 各附 basis+reeval_anchor;M-d RD-034 复查维持 blocked〔探针真跑 spirv-cross 仍拒 raygen〕;M-e G20 链旧门零降级)+ P2 穷举 **13 行零空行** + soak(69 迭代 1854.7s ≥1800s 零失败,含 ReSTIR 车道穿插 13 次)+ close-out 终审(**八 facts VERDICT=READY**);G22 承接锚齐备(slab 预制件在树)。各门终态以 [`milestones/g21/G21_CONTRACT.md`](milestones/g21/G21_CONTRACT.md) §8.7、`G21_P2_DECISIONS.md` 与 `registry/` 为准,旧行按只追加纪律不回写。

> **状态勘误(2026-08-24,只追加不回写)**:上方同日 G21 勘误行状态止于 G21 收口——**G22(材质/流送/时域期,七期串行战役第四期,`milestones/g22/`)已于 2026-08-24 收口**(`status: closed`,close-out flip 见 [`milestones/g22/G22_CONTRACT.md`](milestones/g22/G22_CONTRACT.md) §8.7;tag `g22-closed`):五 P0 全绿(M-a Substrate 类 slab 能量守恒闭合 host 参考臂 **implemented**——白炉恒等 dev=0/能量上界 16641 样本/单调/闭式↔级数+尾和恒等式 1e-15/lerp 连续/双跑位级,closure 单层生产面 0-byte;M-b SVT **defer** 四行差距闭集;M-c KTX2-BasisU **defer** 三行差距闭集 DDS 链维持;M-d Work Graphs **not-available 实测**〔VK_AMDX_shader_enqueue absent〕+ DGC 三扩展 **available 实测** + dgc.rs M102 现面 + FSR 3.1.5 maintain;M-e G21 链旧门零降级)+ P2 穷举 **11 行零空行** + soak(69 迭代 1846.2s ≥1800s 零失败,含 slab 车道穿插 13 次)+ close-out 终审(**八 facts VERDICT=READY**);G23 承接锚齐备(Jolt 5.6 评估臂 sys56 在树)。各门终态以 [`milestones/g22/G22_CONTRACT.md`](milestones/g22/G22_CONTRACT.md) §8.7、`G22_P2_DECISIONS.md` 与 `registry/` 为准,旧行按只追加纪律不回写。

> **状态勘误(2026-08-24,只追加不回写)**:上方同日 G22 勘误行状态止于 G22 收口——**G23(物理平台深化期,七期串行战役第五期,`milestones/g23/`)已于 2026-08-24 收口**(`status: closed`,close-out flip 见 [`milestones/g23/G23_CONTRACT.md`](milestones/g23/G23_CONTRACT.md) §8.7;tag `g23-closed`):五 P0 全绿(M-a M125-adopt3 重判 **maintain-5.3**——采纳三件条件 1/3〔sys56 评估臂 cargo check 新鲜绿 + g9_m125 A/B 绿件只读盘点命中,生产切换需求证据三类全空〕,5.3 生产默认 VENDOR.md pin 0-byte;M-b M127 重判 **maintain 研究子轨**——两半实测未命中〔corpus NONE + PhysicsAsset residual 消费方 NONE〕;M-c RD-042/043 四轨全 **maintain-observe**;M-d RD-044 三分项 defer 2 + Rapier **maintain-no-go**〔M126 measured 40400ns vs 197900ns 转引〕;M-e G22 链旧门零降级)+ P2 穷举 **11 行零空行** + soak(69 迭代 1836.9s ≥1800s 零失败,含 sys56 构建新鲜车道 13 次)+ close-out 终审(**八 facts VERDICT=READY**);G24 承接锚齐备(G18 承接池尾四行)。各门终态以 [`milestones/g23/G23_CONTRACT.md`](milestones/g23/G23_CONTRACT.md) §8.7、`G23_P2_DECISIONS.md` 与 `registry/` 为准,旧行按只追加纪律不回写。

> **状态勘误(2026-08-24,只追加不回写)**:上方同日 G23 勘误行状态止于 G23 收口——**G24(呈现与尾门清理期,七期串行战役第六期,`milestones/g24/`)已于 2026-08-24 收口**(`status: closed`,close-out flip 见 [`milestones/g24/G24_CONTRACT.md`](milestones/g24/G24_CONTRACT.md) §8.7;tag `g24-closed`):五 P0 全绿(M-a M114-strand 重判 **maintain card/mesh**——M120 measured 数据半命中 + 毛发资产需求半未命中;M-b M118-hdr-cal 重判 **maintain-SDR**——设备半 vulkaninfo 实测 **not-available**〔HDR10_ST2084/BT2020/HLG token 全 absent〕;M-c G10-N6 复查 **维持双场景闭集**——fbx2gltf/assimp/blender 三工具 PATH 实测全缺 + BistroExterior 独立源资产缺;M-d SAFE-GPU **defer-to-G25+** + 历史 open RD 十一条清册逐条 backfill 核验 **零 close 诚实** + history 只追加;M-e G23 链旧门零降级)+ P2 穷举 **9 行零空行** + soak(69 迭代 1847.4s ≥1800s 零失败,含 HDR 探针车道 13 次)+ close-out 终审(**八 facts VERDICT=READY**);**G18 承接池本期全量消化清零**;G25 收官承接面就绪。各门终态以 [`milestones/g24/G24_CONTRACT.md`](milestones/g24/G24_CONTRACT.md) §8.7、`G24_P2_DECISIONS.md` 与 `registry/` 为准,旧行按只追加纪律不回写。

> **状态勘误(2026-08-24,只追加不回写;G19-G25 七期串行战役收官行)**:上方同日 G24 勘误行状态止于 G24 收口——**G25(全量商用终审收官期,七期串行战役收官,`milestones/g25/`)已于 2026-08-24 收口**(`status: closed`,close-out flip 见 [`milestones/g25/G25_CONTRACT.md`](milestones/g25/G25_CONTRACT.md) §8.7;tag `g25-closed`):五 P0 全绿(M-a 画质终态**维持达标**——画质表面闭集 10 项 vs g18-closed 0-byte 机核 + 战役加性四模块零接线 + G18 M-d 达标绿件,重渲无信息增量;M-b fps **17/18 诚实红终判**——焦点格 bistro/t100/dlss_sr ratio 0.856326 定盘 + 性能面全战役 0-byte + 焦点格 canonical 160 帧新鲜单测 frame_ms_production_mean=3.5520ms 真跑,G15 物理不可达兜底同源,顺延锚 = NGX 分解 profiling/UE 插桩;M-c 战役全链零降级 + strict 285;M-d 承接锚归档闭集〔七期 15 行 + RD 八条 + 清册十二行 = G26+ 唯一法定输入 `g25_campaign_handover_registry.json`〕;M-e 旧门零降级)+ P2 穷举 **7 行零空行** + soak(69 迭代 1840.4s ≥1800s 零失败,含战役四实现件探针轮换 13 次)+ close-out 终审(**八 facts VERDICT=READY**)。**七期战役合计**:七 tag 全在位(g19~g25-closed)、35 个 P0 全绿、P2 穷举 79 行零空行、七期 soak 累计 12888.7s 全零失败、cargo test 486 + pytest 163 全绿、四个真实实现件(framegen/hzb/restir_reservoir/slab)、G18 承接池全量消化清零;战役总结 = [`milestones/g25/G19_G25_CAMPAIGN_RECORD.md`](milestones/g25/G19_G25_CAMPAIGN_RECORD.md)。各门终态以各期契约 §8、P2 表与 `registry/` 为准,旧行按只追加纪律不回写。

> **状态勘误(2026-08-25,只追加不回写;G26-G30 五期串行战役第一期行)**:上方 G25 战役收官勘误行状态止于 G25 收口——**G26(时域/帧生成 device 化期,用户「帮我一次性完成G26-G30」五期串行战役第一期,`milestones/g26/`)已于 2026-08-25 收口**(`status: closed`,close-out flip 见 [`milestones/g26/G26_CONTRACT.md`](milestones/g26/G26_CONTRACT.md) §8.7;tag `g26-closed`):五 P0 全绿(M-a FG/MFG device kernel **implemented**——`kernels/g26_framegen.rx` 真跑对拍 p100=3.576e-7 ≤ 冻结容差 7.153e-7〔标定程序产 measured×2.0,F4 量化兜底 tol<0.025〕+ SSIM 全帧严格胜 frame-hold + device 双跑位级 + kernel-bias/seed-change 双 RED 臂 + spirv-val + temporal/ 目录 0-byte 机核,G13-N7 device 车道锚兑现;M-b device 帧时 measured 登记——×2/×3/×4 = 60.64/60.71/60.22ms 程序产入 budget ×2.0 回归守护 + FgAccounting 真渲/presented 双恒等式机核 + 性能面三文件 0-byte,不构成帧率对标通过线;M-c RD-045 backfill 重判 **maintain-open**——新鲜观察窗 6/6 零漂移 + 三件盘点 0/3 机核〔F5 观察窗零漂移不充根因定位件〕+ history 只追加;M-d G17-MD-F1 重判 **maintain 17/18 诚实红 carry**——两半证据树内闭集搜索 0+0〔F6 searched-paths manifest 必填〕终判归 G30;M-e 旧门零降级)+ P2 穷举 **8 行零空行**(closed-go 8)+ soak(67 迭代 1950.7s ≥1800s 零失败,五车道探针扩容穿插 13 次〔战役四实现件 + framegen device --probe 快车道〕,同窗独立第二实例 8/8 同绿双证)+ close-out 终审(**八 facts VERDICT=READY**);RFC-0043 经 D-409 对抗评审 11 findings 全 disposition(v0.2 修法批)Agent Approved;RD 八条 open 维持;cargo test 486 passed 维持。各门终态以 [`milestones/g26/G26_CONTRACT.md`](milestones/g26/G26_CONTRACT.md) §8.7、`G26_P2_DECISIONS.md` 与 `registry/` 为准,旧行按只追加纪律不回写。

> **状态勘误(2026-08-25,只追加不回写;五期串行战役第二期行)**:上方 G26 勘误行状态止于 G26 收口——**G27(几何 device 化期,五期串行战役第二期,`milestones/g27/`)已于 2026-08-25 收口**(`status: closed`,close-out flip 见 [`milestones/g27/G27_CONTRACT.md`](milestones/g27/G27_CONTRACT.md) §8.7;tag `g27-closed`):五 P0 全绿(M-a HZB device 化 **implemented**——`kernels/g27_hzb_reduce.rx` + `g27_hzb_test.rx` 真跑:金字塔 mips 9 级双臂**位级全等零容差**〔纯 min/max 选择运算〕+ 800 rect × 双约定判定序列逐字节全等 + 零假阳性独立复核 + device 双跑位级 + tamper 构造性双臂检出 + 剔除数 231/800 与 host 一致 + geometry 三文件 0-byte 机核,RD-039 HZB device 化分项兑现;M-b M61 重判 **maintain-no-go**——三项盘点 1/3〔HZB device 半边命中 + P4 未清零 + HW measured 证据零命中,防冒充硬线:①命中不得单独启动〕RFC-0034 重判表 G27.2 行只追加;M-c cluster P4 四行重判**全维持 open**——cluster 专属检索零实现 + P4-2 依赖解除事实登记≠兑现 + RD-039 history G27.3 只追加断档口径注明;M-d M98-l4 重判 **maintain 三级链**——两半 0/2〔device 腿零实现 + 三处 fail-closed 入口在位实测〕RXS-0396/0359 不混同;M-e 旧门零降级)+ P2 穷举 **8 行零空行**(closed-go 8)+ soak(67 迭代 1836.0s ≥1800s 零失败,六车道探针扩容穿插 13 次〔四实现件 + framegen/hzb 两 device 快车道〕)+ close-out 终审(**八 facts VERDICT=READY**);RFC-0044 经 D-409 对抗评审 11 findings 全 disposition(v0.2 修法批)Agent Approved;RD 八条 open 维持;cargo test 486 passed 维持。各门终态以 [`milestones/g27/G27_CONTRACT.md`](milestones/g27/G27_CONTRACT.md) §8.7、`G27_P2_DECISIONS.md` 与 `registry/` 为准,旧行按只追加纪律不回写。

> **状态勘误(2026-08-25,只追加不回写;五期串行战役第三期行)**:上方 G27 勘误行状态止于 G27 收口——**G28(光照 device 化期,五期串行战役第三期,`milestones/g28/`)已于 2026-08-25 收口**(`status: closed`,close-out flip 见 [`milestones/g28/G28_CONTRACT.md`](milestones/g28/G28_CONTRACT.md) §8.7;tag `g28-closed`):五 P0 全绿(M-a ReSTIR device kernel **implemented**——`kernels/g28_restir.rx` 真跑:随机带单源录制〔PCG32 u64 留 host + 录制自检锚:录制终态 vs estimate_ris 直调逐 trial 位级〕+ y 整数锚 20000/20000 全等零翻转 + 对拍 p100=2.831e-6 ≤ 冻结容差 5.66e-6〔程序产 measured×2.0〕+ 无偏 3σ + 双跑位级 + RED 臂 + gi/ 冻结 0-byte,M100-high「device 化」件兑现;M-b 空间重用加性臂 **implemented**——受点重评快照变换后直调冻结 merge〔零复刻,异点偏置经评审 F5 代数验证闭环〕+ 聚合 3σ dev=3.54e-3 + 逐点 5σ 全过 + 方差再收益 min/mean/max=0.899/2.063/2.733 如实登记含负收益点不设通过线,M100-high「空间重用」件兑现〔登记≠车道锚整体兑现,M100 车道集成窗维持〕;M-c M52 重判 **maintain-defer**——capability 现势 available〔新鲜 vulkaninfo 复测三 token 零漂移〕+ workload 零实现〔manifest 5 条 + M50 库面底座不混同〕两半 1/2 单半不改判 + RD-040 五分项逐锚重判全维持 defer〔逐分项 manifest + 锚映射表〕;M-d RD-034 复查 **maintain-blocked**——探针真跑 rc=0〔spirv-cross 仍拒 raygen,HLSL builtin 5319 谱系〕blocked 证据新鲜 + ②分支零检测声明;M-e 旧门零降级)+ P2 穷举 **8 行零空行**(closed-go 8)+ soak(61 迭代 1809.2s ≥1800s 零失败,七车道探针扩容穿插 12 次〔四实现件 + framegen/hzb/restir 三 device 快车道〕)+ close-out 终审(**八 facts VERDICT=READY**);RFC-0045 经 D-409 对抗评审 12 findings 全 disposition(v0.2 修法批——含 F3 f64 幻影态删除)Agent Approved;RD 八条 open 维持;cargo test 486 passed 维持。各门终态以 [`milestones/g28/G28_CONTRACT.md`](milestones/g28/G28_CONTRACT.md) §8.7、`G28_P2_DECISIONS.md` 与 `registry/` 为准,旧行按只追加纪律不回写。

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
| `src/rurix-android-present` | Android on-device present 胶水(MB1;零-Java NativeActivity cdylib 壳,桌面编译为空 lib) |
| `src/rurix-render` | 原生引擎渲染器库(G5:声明式 render graph / 虚拟化几何 / VSM / 探针 GI / 光追效果 / 材质流送 / 时域重建;渲染器是库不进语言) |
| `src/rurix-geom-build` | 离线几何构建器(G5:网格 → meshlet 化 → 分组简化层级 DAG + CPU 参照剔除器,host 纯 safe 确定性) |
| `apps/uc06-renderer` | UC-06 全管线 demo(G5:剔除 → VisBuffer → 延迟着色 → GI/VSM/RTAO → TAA/TSR → headless readback 像素断言) |

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

- **规范 ↔ 测试 ↔ PR 三角**:每条 RXS 规范条款 ≥1 测试锚定(`ci/trace_matrix.py`,当前 278/278)。
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
