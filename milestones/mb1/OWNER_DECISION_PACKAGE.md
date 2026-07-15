# MB1 Owner Decision Package — Vulkan/SPIR-V 跨端后端(红线 3 方向)

> **性质**:本文件把 mb1(多后端新纪元)所有 **owner 裁决闸口**摊清,并给出**精确的、可逐字应用的改动草案**。agent 起草、把待裁摊清、**停在此不自动执行状态翻转**(任务 §5);既有治理文件(`13_DECISION_LOG.md` / `registry/spike_gating.json`)在本分支**保持 pristine(零改动)**——下述改动为 owner 裁决后应用的独立 PR 内容,非本分支已落。
>
> **诚实前置(裁决须知)**:mb1 = Vulkan/SPIR-V 跨端后端,**正面触死亡路线红线 3**(多后端 AMD/Intel/Metal/Vulkan/SPIR-V;D-008/SG-003)。项目自己的记录(`spike_gating.json` SG-003,最近一条 **2026-07-14**)明确判定其解除前提『**NVIDIA 单栈纵深完成**』**未达**;`03 §4`/`11 §2` 将 WGSL/wgpu/SYCL/HIP 的「跨平台优先 → 性能/能力/provenance 全部让位」列为死亡路线本体。**agent 不自行宣布该前提已达成,不自签、不自翻。是否认定前提已满足、是否解除红线 3,是 owner 的主动决策(10 §9.2,一次一条)。**
>
> **与仓库默认治理的关系**:仓库默认(D-406 v2.0 / AGENTS v3.0)为 agent 完全自主(无 owner 批准门)。**本任务由 owner 明确覆盖该默认**,保留红线 3 解除 / SG-003 触发 / RFC-0011 批准 / milestone 激活为 owner 裁决闸口(任务 §5:「在未见明确授权前,一律 draft-and-surface」)。故本包 draft-and-surface,不自 flip。

---

## 0. 裁决摘要(owner 一次决定,四选一 / 组合)

| # | 裁决点 | agent 拟裁 | owner 裁决 |
|---|---|---|---|
| A | **是否解除红线 3(D-008)** | 摊清两侧实证(§1),**不预设**;若解除,承 RFC-0011 单一 Vulkan/SPIR-V 后端(explicit、单目标 per-build、无地址空间推断,不犯 WGSL/SYCL 之错) | ☐ 解除　☐ 维持　☐ 其他 |
| B | **SG-003 → triggered(RFC-0011)** | 随 A=解除 同步(§2) | ☐ 同 A |
| C | **RFC-0011 批准** | 随 A=解除 + B 同步(§3);内容层 agent 已尽力(§4 技术面全做) | ☐ Approved　☐ 退回修订 |
| D | **mb1 里程碑激活 + 里程碑命名** | 命名已定 **mb1**(owner 2026-07-15 裁);激活随 A/B/C | ☐ 激活　☐ 暂缓 |

> **若 owner 选「维持红线 3」**:mb1 全部实现面(Phase 1~4)不启;本分支 governance 草案存档(RFC-0011 标 Withdrawn/Deferred,编号已消费不复用),NVIDIA 单栈继续纵深。agent 已把工程面做到「墙前」(见 §4),owner 决策无信息缺口。

---

## 1. 红线 3 解除的两侧实证(供 owner 裁决,agent 不预设)

**维持红线的理据(须并陈)**:项目定位建立在红线 3 之上(`03 §4`:WGSL/wgpu/SYCL/HIP「跨平台优先牺牲性能/能力/provenance」= 死亡路线);D-002 否决 Vulkan 路线(Windows 驱动黑洞)、D-130 择 D3D12 而非 Vulkan;`11 §5` 明列解除前提 = 『NVIDIA 单栈纵深完成』,SG-003 四条记录(含 2026-07-14)均判定该前提未达。

**解除的理据(须并陈)**:① 使命判据「生产级」的受众——CUDA-only 触达不到 AMD 桌面与 Android(GPU 生态两个最大非 NVIDIA 面);② RFC-0011 的 Vulkan 后端在设计上**不犯 WGSL/SYCL 的错**——explicit、单目标 per-build(`--target vulkan` 显式、无隐式多目标、无静默 fallback,P-01)、无地址空间推断(binding/set 由 RFC-0005 绑定布局推导显式产出,非弱化 provenance)——它触红线 3 的**字面**(多后端),而非其**底层关切**(可移植抽象层牺牲控制);③ SPIR-V 单一 IR 覆盖 AMD+Android,不引入 per-vendor codegen 蔓延,纵深可控。

**关键诚实点**:『NVIDIA 单栈纵深完成』是否达成 = owner 的判定,不是 agent 的。RFC-0011 §2 完整并陈两侧,不预设结论。

---

## 2. 改动草案 A+B — D-008 解除 errata + SG-003 → triggered(**若 owner 裁决解除**)

> **应用纪律**:① `13_DECISION_LOG.md` + `registry/spike_gating.json` 均为 **CRLF** 文件——应用时**保 CRLF**(`\r\n`),既有行 0-byte,只改/追加下述行,逐字节核 CR。② D-008 解除为 **独立 errata PR**(00 §6.3 追加式修订,`check_guardrails check_planning_docs` 预期红,owner 裁决后合入);SG-003 为**独立 registry PR**(append-only)。③ 日期/裁决理由由 **owner 落笔**(agent 代录机器事实、不代签);下述 `<...>` 为占位。

### 2A. `13_DECISION_LOG.md`

**§7 待决清单 — D-008 行**(现:`| D-008 | 多后端红线解除（红线 3） | G2 完成后 | 维持红线直至 NVIDIA 纵深完成 |`)更新为(镜像 D-131 已裁行体例,保留结构、resolution 入末列):

```
| D-008 | 多后端红线解除（红线 3） | G2 完成后（已至） | 【owner 裁决 <YYYY-MM-DD>】<解除，承 RFC-0011 = mb1 单一 Vulkan/SPIR-V 跨端后端（AMD 桌面 + Android，compute+graphics）；SG-003 → triggered(RFC-0011) 同步>。<owner 一行理由：为何认定『NVIDIA 单栈纵深完成』前提已达/可解除> |
```

**§8 修订记录 — 追加新行**(接 v2.0 之后):

```
| v2.1 | <YYYY-MM-DD> | **D-008 多后端红线 3 解除裁决（owner 主动决策，10 §9.2 一次一条）**：owner 裁决<解除>红线 3——触发条件『G2 完成后』已至；解除承 [RFC-0011](rfcs/0011-vulkan-spirv-backend.md)（mb1 单一 Vulkan/SPIR-V 跨端后端，AMD 桌面 + Android，compute+graphics，explicit/单目标 per-build/无地址空间推断，不做通用可移植抽象层）。<owner 一行理由：前提认定>。同步 SG-003 → triggered(RFC-0011)（registry 独立 PR）+ 承接里程碑 mb1（milestones/mb1）。**本裁决为 owner 主动决策，agent 代录机器事实、不代签**。规划文档勘误（00 §6.3 追加式修订，独立 errata PR，check_guardrails check_planning_docs 预期红，owner 裁决后合入） |
```

### 2B. `registry/spike_gating.json` — SG-003 条目

三处改动(schema 允许:`trigger_condition`/`permanence` 0-byte,只追加 `decisions[]` + 翻 `current_verdict`):

1. **`current_verdict`**:`"not_triggered"` → `"triggered(RFC-0011)"`
2. **`decisions[]` 追加一行**(接 2026-07-14 之后):
```json
        { "date": "<YYYY-MM-DD>", "verdict": "triggered(RFC-0011)", "rationale": "owner 主动决策(10 §9.2 一次一条):解除红线 3(D-008 同步 errata v2.1)。承 RFC-0011 = mb1 单一 Vulkan/SPIR-V 跨端后端(AMD 桌面 + Android,compute+graphics);explicit/单目标 per-build(--target vulkan 无隐式多目标·无静默 fallback P-01)/无地址空间推断(binding 经 RFC-0005 显式推导)——触红线 3 字面(多后端)非其底层关切(通用可移植抽象层牺牲控制)。<owner 一行理由:『NVIDIA 单栈纵深完成』前提认定>。trigger_condition 0-byte 不改。" }
```
3. **`revision_log` 追加一行**:
```json
    { "version": "v1.5", "date": "<YYYY-MM-DD>", "change": "SG-003 多后端 red-line 3 由 not_triggered → triggered(RFC-0011):owner 主动决策解除红线 3(D-008 同步 errata v2.1,10 §9.2 一次一条);承 mb1 单一 Vulkan/SPIR-V 跨端后端。只追加 decisions + 翻 current_verdict,trigger_condition/permanence 0-byte;其余 SG 条目不动。" }
```

---

## 3. 改动草案 C — RFC-0011 状态 Draft → Approved(**若 owner 批准**)

> `rfcs/0011-vulkan-spirv-backend.md` 为**新文件(LF)**,已在本分支落地(Draft)。owner 批准后改三处(agent 代录):

- 前置元数据 **状态** 行:`Draft（2026-07-15）...未获裁决前 agent 不自签...` → `**Owner Approved（<YYYY-MM-DD>）**。owner 裁决红线 3(D-008/SG-003)解除后批准;可推进 mb1 下游实现 PR`
- 前置元数据 **Agent 批准** 行:`未批准（Draft）...` → `**Owner Approved — <YYYY-MM-DD>**;批准范围含 🔒 §4.5(Backend trait FFI)/§4.7(launch marshalling)/§4.10(dlopen 移植缝);红线 3 解除为 owner 主动决策(区别于 D-406 常规 RFC 自主批准);记录于本文件与 MB1_CONTRACT §8`
- 末尾 **修订记录** 追加行:`| Owner approval | <YYYY-MM-DD> | owner 裁决红线 3 解除并批准 RFC-0011 全文(含 🔒 §4.5/§4.7/§4.10 FFI 面与 §9 八项裁决);批准后推进 mb1 Phase 1~4 实现 PR | Full RFC(Owner Approved) |`

> §9 表中 `Q-Binding`/`Q-Marshal`/`Q-ArchKey`/`Q-Trait`/`Q-Android`/`Q-Perf`/`Q-Stages` 各「拟裁」若 owner 有异议,可在批准时回填;无异议即以拟裁为准(实现 PR 定案回填)。

---

## 4. 已完成的工程面(裁决无信息缺口 — agent 做到「墙前」)

owner 裁决红线 3 时,工程可行性**不需推测**——agent 已(或将,随后续 turn)在本机 NVIDIA(RTX 4070 Ti,完整 Vulkan 1.4)上把除「AMD 真卡 / Android 真机」外的**全部工程与验证做完并留真实红绿**:

- **环境已证**:VULKAN_SDK 1.3.296.0 + glslang/spirv-val/spirv-cross/dxc 在位;`vulkan-1.dll` in System32;RTX 4070 Ti 经 Vulkan 1.4.351 枚举(driver 620.02)——完整实现,compute+graphics 本机真跑可行。
- **Phase 1~3**:本机 NVIDIA(+lavapipe 第二 ICD)真实红绿全绿并归档;host 四门 + 新 CI 步骤 `trace N/N` 全绿。
- **Phase 4**:android-arm64 交叉**构建**绿。
- **两道硬件尾门明确 open + DoD**:① AMD 真卡验收(G-MB1-6)② Android 真机 on-device(G-MB1-7)——缺硬件,不伪造、不签;NVIDIA(+lavapipe)跑通不充作 AMD/Android 已验证。

> 即:owner 只需就「**方向**(是否走多后端/解红线 3)」裁决,「**能不能做成**」已由真实红绿回答。

---

## 5. 改动草案 D(路由) — 索引/台账登记(随 mb1 首 PR 合入,非独立)

> 以下为 **CRLF** 文件的路由登记,**gated on 红线 3 解除 + RFC-0011 批准**,与 vulkan_backend.md/RFC-0011 同 PR 合入(非本分支已落——保 CRLF 应用)。

### 5A. `spec/README.md`

- **§4 文件清单**(edition.md 行之后、`## 5.` 之前)追加一行:
```
| [vulkan_backend.md](vulkan_backend.md) | Vulkan/SPIR-V 跨端第三后端语义面（MIR→SPIR-V，AMD 桌面 + Android，compute+graphics，承 RFC-0011：codegen target 分发与 Vulkan 后端分叉 / MIR→SPIR-V compute·graphics 编码 / 数学 intrinsic→GLSL.std.450 / 运行时 Backend trait 抽象 / Vulkan compute·graphics+present 运行时 / launch marshalling / artifact 泛化 / Android 移植缝 / toolchain 定位·供应链；🔒 FFI ABI 二进制布局 / 纹理内存模型 / 通用可移植抽象层承诺均不在本文件；档位 Full RFC/RFC-0011） | RXS-0200 ~ RXS-0213（mb1：脚手架仅登记区间预留、不落裸条款头，条款体随 mb1 各 Phase 实现 PR 同落；跳号 RXS-0189~0199=MS1.2/MS1.2b 承接避撞；gated on 红线 3 解除 + RFC-0011 批准） | mb1 |
```
- **§5 修订记录**(v1.53 行之后)追加一行:
```
| v1.54 | 2026-07-15 | §4 文件清单追加 vulkan_backend.md（RXS-0200~0213 预留区间，起始里程碑 mb1：Vulkan/SPIR-V 跨端第三后端语义面——单一 MIR→SPIR-V codegen + Vulkan 运行时后端同覆盖 AMD 桌面 + Android，compute+graphics）。承 [RFC-0011](../rfcs/0011-vulkan-spirv-backend.md)（**Draft**，gated on owner 裁决红线 3(D-008/SG-003)解除 + RFC 批准）。**脚手架仅登记文件名 + 预留区间 RXS-0200~0213，不落带编号裸条款头**——vulkan_backend.md 零 `### RXS-####` 条款头，trace_matrix 维持全锚定不变。跳号 RXS-0189~0199（MS1.2/MS1.2b 承接，feat/ms1.2b 在途）避撞。gated on 红线 3 解除 + RFC-0011 批准，未获裁决前不合入 main。 | **Full RFC**（RFC-0011） |
```

### 5B. `rfcs/README.md` — §5 编号台账

- Full RFC 「已用」单元在 `RFC-0010（...）` 之后追加:
```
 · RFC-0011（[`0011-vulkan-spirv-backend.md`](0011-vulkan-spirv-backend.md)，mb1 Vulkan/SPIR-V 跨端第三后端 AMD 桌面 + Android compute+graphics，**Draft 2026-07-15——gated on owner 裁决红线 3(D-008/SG-003)解除 + RFC 批准**）
```
- 「下一个未用」单元:`RFC-0011` → `RFC-0012`

---

## 6. 应用顺序(owner 裁决「解除」后)

1. **独立 errata PR**：`13_DECISION_LOG.md` §7+§8（草案 2A，CRLF 保形，check_planning_docs 预期红）。
2. **独立 registry PR**：`registry/spike_gating.json` SG-003（草案 2B，append-only，CRLF 保形）。
3. **RFC-0011 Approved**（草案 3）+ **mb1 governance PR**（本分支新文件 + 索引登记草案 5A/5B），合入先于任何 mb1 实现 PR（条款先于实现,硬规则 7）。
4. mb1 Phase 1~4 实现 PR 栈式落地（各自真实红绿，见 MB1_PLAN.md）。

> **owner 裁决「维持」**：以上全不执行;RFC-0011 标 Withdrawn（编号已消费不复用）,本分支存档。
