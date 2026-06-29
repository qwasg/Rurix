# DXIL 生成路径双路 Spike — 取证报告（G2.2，RD-010，RFC-0003 §9 Q-D131=C）

| 字段 | 值 |
|---|---|
| 类型 | **Spike 取证报告**（机器事实汇总 + 复现清单;**非立项、非实现、非性能基准、非常驻 CI 门**）。A/B 最终路径裁决由 **agent 自主裁决**(AGENTS 硬规则 1);本报告只摆证据,**不含 A/B 选择结论**。 |
| 承接 | G2.2 DXIL 第二后端(验收门 G-G2-2);RFC-0003 §9 Q-D131=C「暂不锁路径,留限时双路 spike 取证后由 agent 凭当时成熟度证据再裁 A/B」。 |
| 范围 | A 路(LLVM DirectX 后端直接 emit DXIL,结构首选)× B 路(SPIR-V→DXIL 转译,对照)在统一判据上的**当下成熟度实况**。 |
| 跟踪锚 | RD-010([registry/deferred.json](../registry/deferred.json)) · 裁决载体 RFC-0003 §9 Q-D131 / 13 §D-131 |
| 机器证据 | [evidence/dxil_path_spike_20260623.json](dxil_path_spike_20260623.json)(schema:[milestones/g2/dxil_path_spike_evidence_schema.json](../milestones/g2/dxil_path_spike_evidence_schema.json),经 `ci/check_schemas.py` PASS) |
| 探针 | [spike/dxil-path-probe/](../spike/dxil-path-probe/)(标 `// SPIKE(RD-010)`,不入 src/ 生产路径、不随产品编译、spike 结束可弃) |
| 纪律 | measured-first / blocked-honest(硬规则 3/4):工具/target 探到记实测,探不到如实 blocked + repro,**绝不杜撰数字**。 |
| Provenance | `Assisted-by: claude-opus-4-8`(agent 自主记录机器可核对事实,非代决、非代签) |

---

## 1. 取证范围与背景

13 §D-131 把「DXIL 生成路径（LLVM DirectX 后端 vs SPIR-V→DXIL 转译）」登记为待决，触发时机为「G2 启动时按当时后端成熟度评估，agent 批准」。RFC-0003 §9 Q-D131 经 agent 裁决为 **C**：暂不锁 A/B，留限时双路 spike 取证后由 agent 凭当时成熟度证据再裁。本 spike 即该取证动作，产出两路在统一判据上的当下实况，作为 agent 裁定最终 A/B 的证据基底。

- **A 路**：LLVM DirectX target 直接从文本 LLVM IR emit DXIL。与既有 NVPTX 后端同构（rurixc 产文本 LLVM IR → 外部 pin clang/llc 经 target 后端汇编，D-205 LLVM 单栈），无第二中间表示 —— **结构首选**。
- **B 路**：SPIR-V→DXIL 转译。引入第二中间表示（SPIR-V）+ 外部转译依赖 —— **对照**。
  - 注：此处 SPIR-V 仅作 DXIL 转译的内部中间表示，**≠** SPIR-V 作为对外通用目标（后者属死亡路线红线 3 / SG-003，不在本 spike 范围）。

**A/B 唯一卡点 = LLVM DirectX target 当下成熟度**（DXIL 合规性 / shader model 覆盖 / validator 兼容）。13 §D-131 明确须按当时后端成熟度评估、不冻结某时点判断。

## 2. 取证方法

- 探针 `spike/dxil-path-probe/`（`_common.py` + `probe_a_llvm_directx.py` + `probe_b_spirv_to_dxil.py` + `run_spike.py`），全部经 `subprocess`（`shell=False`、list 参数，禁字符串插值，防命令注入）调外部工具，带 timeout；工具缺失/超时/spawn 失败一律降级为结构化失败，不抛、不崩溃。
- clang 定位复刻 `toolchain.rs` 探测序（D-205）：`RURIXC_CLANG` > `C:\Program Files\LLVM\bin\clang.exe` > PATH。
- 工具不存在记 `unavailable`（对齐 schema 受限环境降级值）。探到工具记实测版本与可用性；未跑真实端到端转译的项**诚实标 blocked**，不杜撰 pass。
- 运行：`py -3 spike/dxil-path-probe/run_spike.py` → 写 `evidence/dxil_path_spike_<YYYYMMDD>.json`（二进制写 + 显式 LF，遵 `.gitattributes * -text`）。

本次取证环境：`nt / win32`，clang 定位来源 `default_llvm_path`。

## 3. A 路实测 — LLVM DirectX 后端直接 emit DXIL

| 项 | 实测 |
|---|---|
| 状态 | **blocked**（target 缺失） |
| clang | `clang version 22.1.7`（pin LLVM 在位，D-205 单栈） |
| llc | `unavailable` |
| DirectX/dxil target | **未编入**（`clang --print-targets` 无 directx/dxil 项） |
| 探测命令 | `C:\Program Files\LLVM\bin\clang.exe --print-targets` |
| dxil emit | blocked（无 target，未能 emit） |
| validator（dxc/dxv） | blocked（dxv `unavailable`） |
| shader model 覆盖 | blocked |

**关键事实**：LLVM 官方 DirectX 后端为 **experimental**，不随发行版二进制 ship，须本地从源码 `cmake -DLLVM_EXPERIMENTAL_TARGETS_TO_BUILD=DirectX` 编译。本环境 pin clang 22.1.7 为发行版二进制，故不含 DirectX/dxil target —— 与官方文档预期一致，**非环境配置疏漏**。

**复现清单（取得 A 路完整取证所需）**：
1. 取得编入 DirectX 后端的 LLVM：源码编译 `cmake -DLLVM_EXPERIMENTAL_TARGETS_TO_BUILD=DirectX`。
2. 确认 `llc --version` 的 Registered Targets 含 `directx - DirectX` / `dxil`；`clang --print-targets` 同。
3. 安装 DXIL validator：dxc/dxv（DirectX Shader Compiler，含 dxil.dll 签名/验证）。
4. 设 `RURIX_DXC` / `RURIX_DXV` 指向 dxc/dxv 或置于 PATH；重跑 `probe_a_llvm_directx.py`。
5. 仍须核实：从任意 LLVM IR（非 HLSL→clang 路径）emit 合规 DXIL 的成熟度 + dxc validator 接受率 + shader model 覆盖。

## 4. B 路实测 — SPIR-V→DXIL 转译

| 项 | 实测 |
|---|---|
| 状态 | **measured_local**（工具链在位；端到端转译留实现 PR） |
| 转译链 | combo 链可用：SPIRV-Cross → dxc（direct 链 Mesa `spirv-to-dxil` `unavailable`） |
| spirv-cross | `vulkan-sdk-1.3.290.0-44-g65d73934`（2024-10-04） |
| dxc | `dxcompiler.dll: 1.8 - 1.8.0.4739` |
| SPIR-V producer | glslang `11:15.0.0` |
| dxil emit | blocked-honest（工具在位但未跑真实语料端到端，不杜撰成功率） |
| validator | blocked-honest（同上） |

**供应链成本（实记）**：引入第二中间表示 SPIR-V + 外部转译依赖。本环境探到 SPIRV-Cross、dxc、glslang(SPIR-V producer)。供应链长尾：Mesa（spirv-to-dxil）/ Khronos（SPIRV-Cross）/ Microsoft（dxc）三独立来源，各自版本/许可/合规性需独立 pin 与审计（对齐 D-205 LLVM 单栈 pin 纪律的对照成本）。

**确定性 / strict-only（P-01）**：工具在位；确定性与 strict-only 保真须以真实语料实测（转译层是否引入非确定性/静默降级未验，blocked-honest 不预判）。

**复现清单（完成 B 路端到端实测所需）**：
1. 准备代表性 SPIR-V 语料（经 `dxc -spirv` / `glslangValidator` 从 HLSL/GLSL 产）。
2. 跑 spirv-cross→dxc(combo) 转译为 DXIL。
3. dxc/dxv 验证 DXIL 合规性，记录通过率 + shader model 覆盖。
4. 评估转译层确定性（同输入同输出）与 strict-only 保真：无静默降级/回退。

## 5. A/B 统一判据对照（仅摆证据，不含选择结论）

| 判据 | A 路（LLVM DirectX 直接 emit） | B 路（SPIR-V→DXIL 转译） |
|---|---|---|
| target / 转译链可用 | ✗ DirectX target 未编入发行版 clang（experimental，须源码自编） | ✓ combo 链 SPIRV-Cross→dxc 在位（direct 链 Mesa 缺） |
| dxil emit | blocked（无 target） | blocked-honest（工具在位，未跑真实语料） |
| validator（dxc/dxv） | blocked（dxv 缺） | blocked-honest（dxv 缺；dxc 在位） |
| shader model 覆盖 | blocked | 留端到端实测 |
| 供应链成本 | 复用 D-205 LLVM 单栈、无第二中间 IR | 第二中间表示 SPIR-V + 三独立来源（Mesa/Khronos/MS）独立 pin/审计 |
| 确定性 / strict-only 保真 | n/a（未能 emit） | 未验（转译层非确定性/静默降级风险待实测） |
| 与 D-205 单栈契合 | 同构（与 NVPTX 后端一致） | 引入外部转译依赖，偏离单栈 |

**两路当下均未取得 DXIL emit + validator 合规实测**：A 路卡在发行版 LLVM 不含 experimental DirectX target；B 路工具链在位但端到端转译合规率/确定性须以真实语料实测（留实现 PR，本 spike 不杜撰）。完整 A/B 取证依赖上述两份复现清单的环境补齐。

## 6. 裁决归属与留痕

**A/B 最终路径裁决权属 agent**（RFC-0003 §9 Q-D131 / 13 §D-131 / AGENTS 硬规则 1）。本 spike 仅产证据基底，**agent 自主裁决**。C 不构成禁区 A/B 架构承诺，A/B 裁决权仍留 agent。

agent 凭本报告 + 机器证据裁定最终 A/B 后：
1. 回填 RFC-0003 §9 Q-D131 + 13 §D-131（经勘误 PR）；
2. close RD-010（registry/deferred.json，只追加 history）；
3. 裁决后方进 PR-C1 spec 脚手架（条款先于实现，硬规则 7）。

> 本 spike 纯取证：不落 codegen / 不创建 spec 条款 / 不造错误码 / 不入 golden / 不登 spike_gating（语义错位）。探针隔离于 `spike/dxil-path-probe/`，spike 结束可弃。
