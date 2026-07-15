# G2 上下文蒸馏（导航 + 决策面摘要）

> **从属声明（强制）**:本文为 G2 上下文蒸馏（导航 + 决策面摘要），**非规范源**；规范以
> `00–14` / `registry/*.json` / `spec/*.md` / `13_DECISION_LOG.md` 为唯一事实源；任何冲突以原文为准；
> 本文派生自源、可弃可重生，不作维护型事实源（单源纪律 P-13 / 14 §10.4 / 00–14 冻结）。
>
> **用法**:G2 执行 agent 起步上下文 = 本文 + G2 四件套（[G2_CONTRACT.md](G2_CONTRACT.md) /
> [G2_PLAN.md](G2_PLAN.md) / [CI_GATES.md](CI_GATES.md) / [g2_budget.json](g2_budget.json)），**不需载入全 00–14**。
> 需深挖规范正文 → 派 Explore 子 agent 读原文（见 §9 深挖回溯表），不把全文灌主上下文。
>
> **蒸馏 = 决策面地图 + 不变量清单 + 指针**。下文每条规范性事实均带权威源引用；**禁止复述/复制规范正文**（复述即制造第二事实源）。

---

## 1. G2 是什么（范围候选 + agent 开工裁决）

G2 = MVP 后图形路线**第三阶段**（11 §5）。范围候选（指针，权威源 [11_ROADMAP.md](../../11_ROADMAP.md) §5）:
原生 D3D12 + DXIL 第二后端（D-131）/ vertex·fragment·mesh·task·RT 着色阶段进语言 / 绑定布局编译器推导 /
UC-04 deferred 渲染器 demo / 语言 1.0（spec 全量条款化 + conformance + 首个 edition）/ 生态 ≥3 非作者维护项目 /
registry 决策点（D-312）。详见 [06_GPU_GRAPHICS_PROGRAMMING_MODEL.md](../../06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) §8.2。

**agent 开工裁决**（2026-06-23，经 AskUserQuestion；权威留痕 = [G2_CONTRACT.md](G2_CONTRACT.md) §7，引既有
`D-002` 图形分期**已批准** [13_DECISION_LOG.md](../../13_DECISION_LOG.md)）:

| # | 决策 | 裁定 |
|---|---|---|
| ① | 粒度 | **单 G2 阶段契约**（G2.1~G2.n 子里程碑在 G2_PLAN.md 内分解） |
| ② | 首子里程碑 | **G2.1 着色阶段类型面条款先行**（spec-first，规范先行硬规则 7） |
| ③ | D-131 DXIL 路径 | **本期 defer**（D-131 维持待决，留 DXIL 子里程碑按 LLVM DirectX 后端成熟度经 Full RFC 裁决，13 §D-131） |
| ④ | 红线 3 / 多后端（D-008） | **维持不解除**（默认直至 NVIDIA 纵深完成，一次一条 10 §9.2；SG-003 not_triggered） |
| ⑤ | registry（D-312） | **维持休眠**（not_triggered，留社区规模触发；SG-007） |
| ⑥ | RFC 流程 | **延续 G1.4 FCP-lite + 贡献门**（ci/check_contribution.py 已在 main；新 Full RFC 续号 + mini-0006+） |

> **重要**:13_DECISION_LOG.md 在执行 PR 中字节冻结（`check_planning_docs`）。上述裁决记于 G2_CONTRACT §7
> （对齐 G1 §7 引 D-005 先例），**不改决策日志**；若需在规范日志落正式新 D-###（如「G2 开工/范围」），
> 须 agent 经 00 §6.3 独立规划文档勘误 PR 兑现（D-408 续号），与脚手架 PR 分离。

## 2. 约束 G2 的不变量（全指针，G2 实现必须遵守）

### 2.1 死亡路线红线（[11_ROADMAP.md](../../11_ROADMAP.md) §2 / 03 §4）
- **红线 1**:无 Python 原生嵌入（仅 C ABI / PYD 通道）。→ SG-008 永久 not_triggered。
- **红线 2 / 红线 3**:无多后端（AMD / Intel / Metal / Vulkan / SPIR-V）。→ D-008 维持（G2 完成后才评估解除，一次一条 10 §9.2）；SG-003 not_triggered。
  **DXIL 是 D3D12 原生路径，非通用多后端**——G2 做 DXIL 不等于解红线 3。

### 2.2 设计公理（[04_DESIGN_PRINCIPLES.md](../../04_DESIGN_PRINCIPLES.md)，准永久条款，改动需 30 天公示 10 §9）
- **P-01 strict-only**:无静默 fallback / 近似 / 跳过；失败必为带结构化错误码的编译错误或显式诊断。
- **P-13 防 AI 幻觉治理**:条款号↔conformance↔PR 三角强制；AI 不得定义 UB/内存模型/FFI ABI；验收数字来自命令输出；unsafe 审计；close-out 只追加。
- **P-11 单一事实源，生成多视图**:[06](../../06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) §8.2 据此令 descriptor/root signature 由编译器推导（host 结构体 ↔ shader 布局单一事实源）= G2.3 依据。

### 2.3 硬规则 5 禁区（仅agent 经 Full RFC 落笔，[agents/AGENTS.md](../../agents/AGENTS.md) §2 / 10 §7）
UB 条款 / **内存模型映射（[06](../../06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) §4.2，纹理路径 G2 引入时再扩展，禁区）** / FFI ABI / 安全包络边界。
→ G2 的 DXIL codegen 面、着色阶段语法/类型系统、纹理内存模型 **均须Full RFC 前置**，AI 只登记 gating。

### 2.4 其他语义边界
- **单机单 GPU（A-06，[06](../../06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) 范围红线表）**:MVP 语义边界；G2 碰多 GPU / VMM / NVLink / MIG 须Full RFC（VMM 评估 [08](../../08_RUNTIME_AND_TOOLING.md) §2.2）。
- **device ⊂ host 单向可达**:kernel 子语言是受限子集 + 设备扩展（[05](../../05_LANGUAGE_ARCHITECTURE.md) §1）；trait 单态化子集无 dyn/特化/HKT/async（§2.2，D-104，stable 前重评估）。
- **codegen 后端边界**:MVP/G1 = NVPTX→PTX/cubin/fatbin（[07](../../07_COMPILER_ARCHITECTURE.md) §7）；**DXIL 第二后端无 MVP 期 PTX↔DXIL 对应信息，完全于 G2 重评估（D-131）**。MLIR kernel island 后置（07 §7.1，SG-001）。

## 3. 承接 deferred（@G2，权威源 [registry/deferred.json](../../registry/deferred.json)）

| 编号 | 摘要 | 状态 @G2 | G2 触发面 |
|---|---|---|---|
| RD-007 | const 泛型值运行期单态化（turbofish const 实参 → 实例值代入 + codegen） | inherited | device codegen / 运行期数组 aggregate codegen 扩展评估接通，RXS-0064 语义不变 |
| RD-008 | stable API 快照冻结机制激活（stable 面 + 快照比对 + bless 守卫） | open | **G2.5 语言 1.0 为候选触发点**（首个 stable 发布定义 stable 面并激活） |
| RD-009 | `#[export(c)]` C ABI 导出属性 + 编译器内建头文件生成 codegen | open | 触 FFI ABI codegen 面（硬规则 5）后续判档；当前复用 `extern "C"`（RXS-0125） |

状态翻转由 agent 自主签署（AI 备绿）。新做不完事项 → 14 §4 追加 RD-010+ 双侧标注。

## 4. SG gating（与 G2 的关系，权威源 [registry/spike_gating.json](../../registry/spike_gating.json)）

| 编号 | 方向 | 触发条件 | G2 判定 |
|---|---|---|---|
| SG-001 | MLIR kernel island | 07 §7.1 三条件之一 | not_triggered（G2 不引入中层 MLIR） |
| SG-002 | Tensor Core / WGMMA / TMA | L2 基准证明瓶颈 + 中层成熟度 | not_triggered（11 §2 红线裁剪） |
| SG-003 | 多后端 | G2 完成 + agent 解除红线 3（D-008） | not_triggered（红线 3 维持，一次一条） |
| SG-007 | 包 registry | D-312 社区规模驱动（>50 包/强需求） | not_triggered（本期休眠，留社区规模触发） |

G2 开工复评 decisions 已追加（维持 not_triggered，trigger_condition 0-byte 不改）。新触发方向 → SG-010+（agent 裁决）。

## 5. 续号位（脚手架不预造；G2.x 实现 PR 条款先于实现续号）

| 类别 | 末位（g1-closed 基准） | 下一可用 | 权威源 |
|---|---|---|---|
| spec 条款 RXS-#### | RXS-0152 | **RXS-0153+** | [spec/README.md](../../spec/README.md) §4 |
| deferred RD-### | RD-009 | **RD-010+** | registry/deferred.json |
| 错误码 RX#### | RX7019 | **RX7020+**（段位按 07 §5 语义分配） | registry/error_codes.json |
| spike gating SG-### | SG-009 | **SG-010+** | registry/spike_gating.json |
| unsafe-audit U## | U22 | **U23+** | unsafe-audit 注册表 |
| 预算命名空间 | g1.* | **g2.***（g2_budget.json） | milestones/g2/g2_budget.json |
| Mini-RFC | mini-0005 | **mini-0006+** | rfcs/ |
| Full RFC | （G2 新号） | RFC-#### 新号 | rfcs/ |
| 决策 D-### | D-407 | **D-408+（须独立 00 §6.3 errata PR，不在执行 PR）** | 13_DECISION_LOG.md |

trace 基线 **152/152** 全锚定（脚手架 +0）；新条款每条 ≥1 测试锚定随实现 PR 同落。

## 6. 治理纪律摘要（指针）

- **变更三档**（[10](../../10_GOVERNANCE.md) §3）:Direct PR / Mini-RFC / Full RFC。新语法·类型系统·运行时语义·unsafe 边界·FFI ABI·稳定化·edition·设计原则·死亡路线触碰 = **Full RFC**。
- **红线解除一次一条**（10 §9.2）:死亡路线解除只能 agent 批准且一次一条。
- **十条硬规则**（[agents/AGENTS.md](../../agents/AGENTS.md) §2）:不代签(1) / provenance(2) / 验证强制(3) / 证据分级(4) / 禁区(5) / 只追加(6) / 规范先行(7) / 判档向上取严(8) / unsafe 纪律(9) / 反 extractive(10)。
- 关键复诵:**规范先行（7）**（改 src 前读 spec，语义 PR 引 RXS-####，缺条款先补）/ **判档向上取严（8）**（agent 自主判档）/ **不代签（1）**（status 翻转/基准切换/tag/红线解除/RD·SG 翻转由 agent 自主签署）。

## 7. 工程纪律摘要（指针）

- **字节级 guardrails**:`py -3 ci/check_guardrails.py`（无参默认基准 **g1-closed**）；00–14（含 13_DECISION_LOG.md）+ deep-research 0-byte；registry/预算/已关闭契约/evidence 只追加；error_codes 含义冻结；spec 档位标记；golden bless。
- **零占位预算**:`budget_eval --strict` 全局零 estimated（14 §3，占位存活 ≤2 里程碑）。
- **证据分级**（14 §5）:`measured_local` > `simulated/estimated` > 无证据；性能数字来自命令输出（硬规则 3/4）。
- **真实红绿（反 YAML-only）**:新门禁必须真实失败/通过路径验证；golden（UI/MIR/PTX，+DXIL）+ Compute Sanitizer racecheck/memcheck 常驻 nightly；trace 全锚定。
- **自托管环境**:runner `rurix-dev-4070ti`（RTX 4070 Ti，driver 591.86，CUDA Toolkit 13.3，VS 2022 MSVC + Windows SDK）；脚本用 `py -3`；**evidence 由 agent 桌面会话兑现**（本地跑 `*_smoke.py` 后 `git restore -- evidence/`，AI 不提交 evidence；device 真实红绿 run URL 由 agent 上线回填，对齐 G1.1~G1.5 先例）。
- 仓库 LF 字节精确（`.gitattributes * -text`）；registry JSON 只追加须用 Edit 工具保字节精确（勿 Python text 模式重写 → CRLF）。

## 8. G2 脚手架已落位（本期产物，便于执行 agent 起步定位）

- 四件套 `milestones/g2/`:G2_CONTRACT.md（status active，§8 close-out 空）/ G2_PLAN.md（G2.1~G2.6 + 依赖图）/ CI_GATES.md（步骤 45–49 计划项）/ g2_budget.json（entries/ratio/counter 全空）。
- registry 追加:deferred.json（RD-007/008/009 G2 开工承接 history）+ spike_gating.json（SG-001/002/003/007 G2 开工复评 decisions），均只追加。
- **零改动**:00–14、spec/、ci/*.py、tests/*.py、error_codes.json、m0~g1 契约/预算、evidence/、G1 语义面。
- 脚手架不实现任何 G2 语义面、不解红线、不立 Full RFC、不预造条款/错误码/counter/SG。

## 9. 子 agent 深挖回溯表（drill-down：主题 → 权威源文件/§）

| G2 主题 | 权威源（读原文，不灌主上下文） |
|---|---|
| 着色阶段进语言（语法/类型系统） | 06 §8.2 / 05 §1 §2.2（device⊂host / trait 子集）/ spec/*.md（RXS 条款体） |
| DXIL 第二后端 codegen（D-131） | 06 §8.2 / 07 §7 §7.1 / 13 D-131 |
| 内存模型映射（纹理路径，**禁区**） | 06 §4.2（仅人类 Full RFC） |
| 绑定布局推导（P-11） | 06 §8.2 / 04（P-11） |
| UC-04 deferred 渲染器 | 11 §5 / 02（用户画像/用例） |
| 语言 1.0 / edition / stabilization | 10 §3 §6 §9 / 11 §5 / 01 §6 / RD-008（deferred.json） |
| registry（D-312） | 11 §5 / 09 §7.3 / 13 D-312 / SG-007（spike_gating.json） |
| 多后端红线（D-008） | 11 §2 / 03 §4 / 10 §9.2 / 13 D-008 / SG-003 |
| 死亡路线红线全集 | 11 §2 / 03 §4 |
| 设计公理 / AI 治理 | 04（P-01/P-11/P-13）/ 10 §7 / agents/AGENTS.md §2 |
| 变更判档 / RFC 流程 | 10 §3 §9.2 / rfcs/ / G1.4 FCP-lite 先例 |
| 工程纪律（契约/预算/deferred/证据/gating） | 14 §1 §3 §4 §5 §7 |
| FFI ABI / `#[export(c)]`（RD-009） | 05 §11 / 13 D-113 / spec/engine_integration.md RXS-0149 / deferred.json RD-009 |
| 历史里程碑细节（M*/G1） | milestones/m0~m8 / milestones/g1/（契约 §8 close-out 留痕） |
| 决策原文 / 状态 | 13_DECISION_LOG.md（D-### 原文，**冻结，只读**） |

> 原文不动:00–14 冻结、registry/evidence 只追加、spec 只追加；子 agent 只读探索，结论回传主 agent。
