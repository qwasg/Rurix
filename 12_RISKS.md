# 12 — 风险与难题登记表

> 所属文档集：[00_MASTER_INDEX.md](00_MASTER_INDEX.md)
> 版本：v1.0（2026-06-11）
> 约定：概率/影响取 高/中/低；每项给出**早期消解手段**（多数已编入路线图或纪律）。风险编号永不复用；状态变化以追加方式记录。

---

## 1. 技术风险（R-1xx）

| 编号 | 风险 | 概率 | 影响 | 早期消解 |
|---|---|---|---|---|
| R-101 | **PTX 版本与驱动 JIT 错配**在用户机上炸（Numba `CUDA_ERROR_UNSUPPORTED_PTX_VERSION` 事故类别，r2） | 高 | 高 | 装载前协商 + 结构化指引（[08](08_RUNTIME_AND_TOOLING.md) §2.4）；ptx-floor 进 manifest；M0 起维护 PTX×驱动兼容矩阵测试 |
| R-102 | **LLVM NVPTX 边角 bug**（shfl 选择失败/sqrt 近似失效/ptxas 优化级正确性，r2） | 中 | 高 | pin LLVM 22.1.x；NVPTX 雷区回归集（[07](07_COMPILER_ARCHITECTURE.md) §7）；IR golden 锁形状；ptxas 干验证关卡 |
| R-103 | **LLVM 版本升级税**长期复利（Triton 实证的持续成本，r2/r3） | 高 | 中 | 季度评估制；vendored 构建 + fork 补丁须带 upstream 链接（[10](10_GOVERNANCE.md) §8）；升级走独立里程碑不夹带 |
| R-104 | **views 安全包络不够用**：真实 kernel 大量落 unsafe，安全卖点失效（Descend 边界，r5） | 中 | 高 | G0 软光栅/SPH 是包络压力测试（M7 验收要求 safe 覆盖率报告）；views 算子集按 deferred 清单迭代；包络外配 Sanitizer 纪律使 unsafe 仍可工程化 |
| R-105 | **PTX 弱内存模型映射出错**（morally strong 条件未满足导致 silent 错果，r5） | 低 | 极高 | 映射条款由 agent 自主经 Full RFC 落笔（D-406 v2.0）；conformance 锚定；litmus 测试集（GPUMC 风格）进 nightly |
| R-106 | **WDDM/TDR 现实**：长 kernel 超 2s 被杀、计时扭曲（r4/r11） | 高 | 中 | P-14 全套（环境画像/TDR lint/计时刷队列协议）；demo 设计避免单 kernel >100ms |
| R-107 | **D3D12 interop 未知数**（G1）：external memory/semaphore 在消费级 WDDM 的行为差异 | 中 | 中 | G1 启动前 spike：纯 C 验证 interop 通路再做语言化；上一项目"先探测后承诺"模式 |
| R-108 | **借用检查器+views 扩展的正确性**：soundness 漏洞 | 中 | 高 | 不相交证明走保守规则（拒绝可疑而非放行）；fuzz + conformance；RustBelt 式义务清单只锚定 unsafe 原语（不贪全栈证明，r5） |
| R-109 | **生产档路径追踪毒径挂起**（UC-07 `pt_render` 特定样本序号/弹射深度组合不终止；疑 rurixc PTX 分支重汇聚生成或驱动 JIT 层缺陷，RD-027；2026-07-15 RTX 4070 Ti/driver 620.02/CUDA 13.2 实测同配置必现） | 中 | 中 | 已切片锁定：ms1.bench.uc07_offline 降至 32spp/2-弹射可测切片、CI 冒烟档 160×120/bounces=4 golden 全绿隔离；kernel 源内全部循环编译期有界（spp 批宽/bounces/DDA max_steps/cell 段/拒绝采样 ≤16/Newton 30，非源级死循环）；最小化复现定位 rurixc PTX 生成或上报驱动后把切片升回完整生产档（256spp/4-弹射）重测回填 |
| R-110 | **上游 validation layer 在 Adreno/MTE 上自身崩溃**（VVL vulkan-sdk-1.4.350.1 处理非法 SPIR-V 的错误格式化路径踩已释放/错标指针 → 设备 MTE tagged-pointer 抓死 SIGSEGV/SEGV_ACCERR，VUID 未吐即崩；MB1 G-MB1-7 round-1 实测，layer 上游鲁棒性 bug 非本项目缺陷） | 低 | 中 | RED 自测改「合法 SPIR-V + 模块内假入口名」（pName-00707）天然规避非法字节路径（round-2 已绿）；崩溃逐字栈存档为独立上游证据（evidence/mb1-android-ondevice/round1_halt_excerpt.md）；上游报告 owner 复核门、未提报——提报前须补独立 MRP + 最新 SDK 重测 |
| R-111 | **多后端真硬件验收尾门长期悬置**（MB1 Vulkan/SPIR-V 后端 AMD 桌面真卡验收 G-MB1-6 缺 AMD 硬件无法关闭；NVIDIA+lavapipe 跑通 ≠ AMD 已验证） | 中 | 中 | 缺硬件不设 CI 硬门（SKIP 不充绿）、不伪造 device 绿、不签；DoD 写清（MB1_CONTRACT.md acceptance_gates G-MB1-6 + §8）；lavapipe/SwiftShader 软件 ICD 作过渡验证 SPIR-V 跨非-NVIDIA 驱动可消费 + 数值一致；获硬件后按 DoD 补 evidence + run URL |

## 2. 生态风险（R-2xx）

| 编号 | 风险 | 概率 | 影响 | 早期消解 |
|---|---|---|---|---|
| R-201 | **NVIDIA 许可/再分发条款变化**（CUDA EULA Attachment A 收紧） | 低 | 高 | 最小再分发面（MVP 仅 libdevice）；白名单 CI 审计随条款版本更新；SBOM 全程可追溯 |
| R-202 | **供应链事故**（依赖投毒/typosquat——npm Shai-Hulud 类别，r8） | 中 | 中 | MVP 无 registry、无构建脚本、vendor+checksum 默认；registry 上线首日带透明日志与 typosquat 防御 |
| R-203 | **闭门期生态冷启动失败**：开源时无人问津 | 中 | 高 | MVP 验收即三个可演示旗舰用例（UC-01/02/03）；开源时附完整规范+conformance（可信度差异化）；G0 demo 的视觉传播力是刻意投资 |
| R-204 | **单一硬件厂商依赖**：NVIDIA 战略转向（如自家安全语言） | 低 | 高 | 无法对冲，接受为定位成本；语言核心（所有权/views/诊断）厂商中立，后端可迁移性由 MIR 边界保留 |

## 3. 性能风险（R-3xx）

| 编号 | 风险 | 概率 | 影响 | 早期消解 |
|---|---|---|---|---|
| R-301 | **生成代码达不到手写 CUDA 90%**（安全检查/抽象税） | 中 | 极高 | M4/M5 验收门硬性绑定性能判据，不达标不关闭；views 设计保证零运行时开销（纯类型层）；L1/L2 基准从 M0 锚定 |
| R-302 | **编译时间失控**（单态化爆炸/借用检查开销） | 中 | 中 | 编译性能预算从 M3 实测（P-09）；单 CGU 起步但保留拆分通道；query 化为后续优化留接口 |
| R-303 | **基准证据通道衰退**（机器环境漂移/锁频失效——上一项目最大教训的复发形态） | 中 | 高 | L0 环境验证强制前置每次采样；evidence 标注制；占位阈值 2 里程碑硬限（P-09） |

## 4. 编译器复杂度风险（R-4xx）

| 编号 | 风险 | 概率 | 影响 | 早期消解 |
|---|---|---|---|---|
| R-401 | **借用检查工程量超估**（rustc NLL 是十人年级，r1） | 高 | 高 | 范围裁剪是主对策：无 Polonius/HRTB/异步/trait 对象，NLL 核心规则 + views 扩展；"先正确后诊断打磨"分期；M3 是关键路径上的第一个核对点 |
| R-402 | **trait 求解复杂度蔓延**（rustc 长期成本中心） | 中 | 中 | D-104 裁剪（无特化/HKT/对象）；一致性规则从严；新 trait 特性一律 Full RFC |
| R-403 | **诊断质量债**：功能先行、诊断滞后，口碑受损 | 中 | 高 | UI golden 四黄金路径与功能同步（M1-M4 各上线一条）；诊断是验收门组成部分而非附件 |
| R-404 | **自研语言的规范债**：spec 滞后于实现，实现事实成为语义（FLS 反向教训，r7） | 高 | 高 | "规范领导实现"制度化（[10](10_GOVERNANCE.md) §4）；语义 PR 无条款号即 CI 阻断（开源后） |

## 5. 采纳与竞争风险（R-5xx）

| 编号 | 风险 | 概率 | 影响 | 早期消解 |
|---|---|---|---|---|
| R-501 | **Mojo 提前补齐 Windows 原生 + 开源**，窗口期压缩（r10） | 中 | 高 | 差异化纵深：图形方向（Mojo 是 AI-first）+ 可测试规范治理 + Descend 式设备安全（Mojo 未做）；不打正面战 |
| R-502 | **CUDA C++ 惯性**：目标用户"够用就不换"（r10 采纳阈值） | 高 | 高 | 采纳判据驱动设计（UC-01 单 kernel 迁移 <1 天、零工程结构改动）；互操作优先于替换；安全卖点用真实事故类别叙事 |
| R-503 | **NVIDIA 官方推出同定位语言/大幅升级 CUDA C++ 安全性** | 低 | 极高 | 无法阻止；速度与治理开放性是仅有筹码；保持与 NVIDIA 生态合作姿态而非对抗 |
| R-504 | **Slang 向通用计算扩张**（G2 期正面相遇） | 低 | 中 | Slang 无 host 层/所有权是结构性差距；G2 前完成单语言全栈叙事 |

## 6. 过程与组织风险（R-6xx）

| 编号 | 风险 | 概率 | 影响 | 早期消解 |
|---|---|---|---|---|
| R-601 | **单人 bus factor = 1** | 确定 | 极高 | 全部决策/纪律/规范文档化为机器可执行形态（本文档集 + 14 号纪律即对策）；开源时点是 bus factor 的结构性缓解；关键期身体/节奏管理列为正式约束 |
| R-602 | **AI 语义漂移**：验收放宽/占位说成完成/文档超出事实（上一项目实证三类，H06 §4） | 高 | 高 | [10](10_GOVERNANCE.md) §7 全套 + [14](14_ENGINEERING_DISCIPLINE.md) 契约只追加/数字来自命令输出/guardrail 脚本化 |
| R-603 | **范围蔓延**（单人项目无人踩刹车） | 高 | 高 | spike gating 永久清单 + 死亡路线 + MVP 红线三层防御；任何新方向必须先有 not_triggered 决策记录才能立项 |
| R-604 | **文档税复发**（上一项目 CLAUDE.md 72KB 教训） | 中 | 中 | P-11 单一事实源 + [00](00_MASTER_INDEX.md) §6 尺寸纪律 + deferred/决策走结构化注册表 |
| R-605 | **12–18 个月节奏断档**（动力衰减） | 中 | 高 | 每里程碑硬件证据交付物 = 可见进展；G0 出图节点（M7）刻意置于士气低谷期；阶段切换设回顾点允许范围再裁剪 |
| R-606 | **nightly 子进程无 timeout 僵尸锁 runner**（CI 冒烟脚本多处 `subprocess.run` 无 `timeout=`，挂死子进程变僵尸 exe 持锁工作区，占死自托管 GPU runner 串行队列；nightly 自 2026-06-14 长期非绿，2026-07-17 晨 run 29530318038 经 gh run cancel 止血再证；R-601 单点 bus factor 的运维变体） | 高 | 中 | 止血：gh run cancel + 隔离僵尸 exe（Move-Item 解锁）+ rerun --failed；根治（未落地）= 统一子进程包装强制 timeout= + 超时杀进程树；job 级 `timeout-minutes: 60` 只杀 GitHub job 不杀自托管 runner 内核态僵尸 |

## 7. 难题清单（无完整解，只有姿态）

正面承认的三个开放难题（与风险区分——它们没有"消解手段"，只有长期姿态）：

1. **GPU 安全包络的理论边界**：动态索引/弱序协议的静态安全在学术上未解（r5 全景）。姿态：包络内做绝、包络外工程化（Sanitizer + 验证义务），不承诺超出 Descend 已证明的范围。
2. **单人造语言的历史先例稀少**：现代语言几乎全部是团队产物。姿态：AI 集群是本项目的非对称变量，但其有效性本身是实验——12 个月评审点诚实评估，必要时调整 D-004。
3. **图形 API 的演进不确定性**（Work Graphs/Neural shading 等正在重塑 D3D12）：G2 设计预留可能落空。姿态：G2 前只定方向不定细节（[06](06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) §8 的"预留不承诺"措辞即此风险的对冲）。

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-11 | 初版 |
| v1.1 | 2026-07-17 | §1 追加 R-109（RD-027 生产档 PT 毒径挂起）/R-110（VVL Adreno-MTE 自崩上游 bug）/R-111（MB1 AMD 真卡尾门 G-MB1-6 悬置）；§6 追加 R-606（nightly 子进程无 timeout 僵尸锁 runner，R-601 运维变体）。依本表头列 5「风险编号永不复用；状态变化以追加方式记录」自授权表尾追加：§1–§7 既有风险行与编号 0-byte，不新增章节、不重排 §8。规划文档勘误（00 §6.3 追加式修订，独立 errata PR，check_planning_docs advisory 不阻断）。 |
