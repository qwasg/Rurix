# DXIL 生成路径双路 Spike — Round-2 取证报告（G2.2，RD-010，RFC-0003 §9 Q-D131=C）

| 字段 | 值 |
|---|---|
| 类型 | **Spike 取证报告 round-2**（机器事实汇总 + 复现清单；非立项、非实现、非性能基准、非常驻 CI 门）。A/B 最终路径裁决由 **owner 人工裁决**（AGENTS 硬规则 1）；本报告只摆证据，**不含 A/B 选择结论**。 |
| 承接 | 承 round-1 报告 [evidence/dxil_path_spike_report.md](dxil_path_spike_report.md)（两路均未取得 DXIL emit+validator 实测）。round-1 证据 [20260623.json](dxil_path_spike_20260623.json) 与报告保留不动（历史基线，evidence/ 不可篡改）；本 round-2 为新增证据文件。 |
| 机器证据 | [evidence/dxil_path_spike_20260624.json](dxil_path_spike_20260624.json)（schema：[milestones/g2/dxil_path_spike_evidence_schema.json](../milestones/g2/dxil_path_spike_evidence_schema.json)，经 `ci/check_schemas.py` PASS） |
| 探针 | [spike/dxil-path-probe/](../spike/dxil-path-probe/)（标 `// SPIKE(RD-010)`，不入 src/ 生产路径；round-2 扩展 `probe_b_spirv_to_dxil.py` 端到端实测 + 新增 `corpus/` 代表性语料） |
| 纪律 | measured-first / blocked-honest（硬规则 3/4）：工具/target 探到记实测，探不到如实 blocked + repro，**绝不杜撰数字**。 |
| Provenance | `Assisted-by: claude-code:claude-opus-4-8`（AI 代录机器可核对事实，非代决、非代签） |

---

## 1. Round-2 取证范围

round-1 实况：A 路 blocked（发行版 clang 22.1.7 未编入 experimental DirectX target；dxv unavailable），B 路工具链在位但端到端未实测 → 两路均未取得 DXIL emit+validator 合规实测，D-131 维持 C。round-2 按 round-1 报告 §3（A 路复现清单）/ §4（B 路复现清单）推进，取得当下成熟度的进一步机器事实。结论仍 **不含 A/B 选择**（硬规则 1）。

## 2. A 路 round-2 — 仍 blocked（决策卡点，最优先）

| 项 | round-2 实测 |
|---|---|
| 状态 | **blocked**（target 仍缺失） |
| clang | `clang version 22.1.7`（pin LLVM 在位，D-205 单栈） |
| DirectX/dxil target | **仍未编入**（`clang --print-targets` 无 directx/dxil 项） |
| dxil emit | blocked（无 target，未能 emit） |
| validator | blocked（dxv unavailable、dxil.dll 不在工具链 Bin） |

- 发行版 pin clang **22.1.7** 仍未编入 DirectX/dxil target，与 LLVM 官方 experimental 后端不随 release 二进制 ship 一致 —— **非环境疏漏**。
- 取得 A 路完整取证须源码自编带 DirectX target 的 LLVM（`cmake -DLLVM_EXPERIMENTAL_TARGETS_TO_BUILD=DirectX`，对齐 D-205 pin 22.1.x 保代表性）。该构建为**多 GB / 长耗时重活**，本 session 内执行会 destabilize 环境 → 按 **blocked-honest** 处置：**未启动 LLVM 源码克隆/构建**，A 路 emit/validator 维持 blocked，精确 recipe 见 round-1 报告 §3 复现清单（不变）。
- validator 侧补充实测事实：本环境 `dxv` unavailable、`dxil.dll` **不在工具链 Bin** —— 即便取得 DirectX target，A 路仍须另备独立 DXIL validator 方可完成合规验证。
- 达到状态：probe_a 环境探测复跑 = blocked（与 round-1 一致），未进行源码构建（规避 destabilize）。

## 3. B 路 round-2 — 端到端转译实测（measured_local）

代表性语料 [spike/dxil-path-probe/corpus/](../spike/dxil-path-probe/corpus/)（4 个：`cs_saxpy` / `cs_reduce_shared`（groupshared + barrier 归约）/ `vs_passthrough`（stage IO + 矩阵乘）/ `ps_texture`（Texture2D + SamplerState）），跑端到端链 `HLSL → dxc -spirv → spirv-val → spirv-cross → HLSL → dxc → DXIL`：

| 指标 | round-2 实测 |
|---|---|
| DXIL emit（DXBC 容器 + rc=0） | **4/4 pass** |
| spirv-val（中间 SPIR-V 合规） | **4/4 pass** |
| dxc 内置验证（dxcompiler.dll 1.8.0.4739） | **4/4 pass** |
| 确定性（同输入二次编译 SHA256 一致） | **4/4 deterministic** |
| shader model 覆盖 | cs 6.0/6.2/6.6、vs 6.0/6.6、ps 6.0/6.6 全 pass |
| 工具层静默降级警告 | **0/4**（spirv-cross + dxc 零 stderr） |

**validator 范围诚实声明**：独立 validator `dxv` 缺失、`dxil.dll` 不在工具链 Bin。measured 的是 **dxc（dxcompiler.dll 1.8.0.4739）内置 validator** —— 默认路径（无 `-Vd`）产物与 `-Vd`（关验证）产物**字节不同**，证内置验证 + 签名摘要确实执行（4/4 接受）。完整外部签名验证（dxv / 带 dxil.dll 的 dxc）仍 **blocked-honest**，未杜撰。

**strict-only（P-01）**：工具层零降级警告 + 确定性 4/4 为实测；语义级行为等价（运行期无静默降级/回退）须 device 真跑 golden 验证，超出取证 spike 范围（留实现 PR）。

## 4. Round-2 A/B 判据对照（实测列；仅摆证据，不含选择结论）

| 判据 | A 路 round-2（LLVM DirectX 直接 emit） | B 路 round-2（SPIR-V→DXIL 转译） |
|---|---|---|
| target / 转译链可用 | ✗ DirectX target 仍未编入发行版 clang（源码自编未在 session 内做） | ✓ combo 链 SPIRV-Cross→dxc 端到端实测可用 |
| dxil emit | blocked（无 target） | ✓ **4/4 pass**（DXBC 容器） |
| validator（dxc/dxv） | blocked（dxv / dxil.dll 缺） | dxc 内置 **4/4 pass**；独立 dxv 仍缺 |
| shader model 覆盖 | blocked | cs/vs/ps × SM 6.0–6.6 全 pass |
| 确定性 | n/a（未能 emit） | **4/4 deterministic** |
| 供应链成本 | 复用 D-205 LLVM 单栈、无第二中间 IR | 第二中间表示 SPIR-V + 三独立来源（Mesa/Khronos/MS）独立 pin/审计 |
| 与 D-205 单栈契合 | 同构（与 NVPTX 后端一致） | 引入外部转译依赖，偏离单栈 |

## 5. Round-2 结论（不裁 A/B；evidence 是否充分）

- **B 路**转译链在本环境取得 measured_local 端到端合规实测（emit 4/4 + 确定性 4/4 + SM 6.0–6.6 覆盖 + dxc 内置验证 4/4，零工具层降级警告），较 round-1 的 blocked-honest 实质推进。
- **A 路**（结构首选）仍卡在发行版 LLVM 不含 experimental DirectX target，源码自编为 session 外重活；其 **emit 合规性这一 A/B 决策卡点仍空白**。
- **evidence 充分性判定：尚不充分**。A/B 唯一卡点 = A 路 LLVM DirectX target 当下成熟度（round-1 报告 §1）；round-2 强化了 B 侧实测，但 A 侧 emit 合规性仍未测，两路无法同口径对比，owner 凭单侧 B 证据裁 A/B 信息不足。
- 按硬规则 1：**D-131 维持 C**，不裁 A/B、不回填 RFC-0003 §9 / 13 §D-131；**G-G2-2 未签、G2.2 验收门仍 open**（device 真跑 / DXIL golden / 独立 validator 三样仍缺，AI 不代签）。round-3 解锁条件（取得带 DirectX target 的自编 LLVM + 独立 validator，测 A 路 emit 合规率）仍为后续 round 的 DoD。

## 6. 裁决归属与留痕

**A/B 最终路径裁决权属 owner**（RFC-0003 §9 Q-D131 / 13 §D-131 / AGENTS 硬规则 1）。本 round-2 spike 仅产证据基底，**AI 不代决**。owner 凭 round-1 + round-2 证据裁定最终 A/B 后：回填 RFC-0003 §9 Q-D131 + 13 §D-131（经勘误 PR）→ close RD-010 → 裁决后方进 PR-C1 spec 脚手架（条款先于实现，硬规则 7）。

> 本 round-2 纯取证：不落 codegen / 不创建 spec 条款 / 不造错误码 / 不入 golden / 不登 spike_gating。探针扩展隔离于 `spike/dxil-path-probe/`，spike 结束可弃。
