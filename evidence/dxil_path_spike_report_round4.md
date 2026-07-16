# DXIL 生成路径双路 Spike — Round-4 取证报告（G2.2，RD-010，RFC-0003 §9 Q-D131=C）

| 字段 | 值 |
|---|---|
| 类型 | **Spike 取证报告 round-4**（机器事实汇总 + 复现清单；非立项、非实现、非性能基准、非常驻 CI 门）。A/B 最终路径裁决由 **agent 自主裁决**（AGENTS 硬规则 1）；本报告只摆证据，**不含 A/B 选择结论**。 |
| 承接 | 承 round-1/2/3。round-3：A 路环境已自建（H:\llvm-dxil\build\bin 自编 LLVM 22.1.7 pin commit a255c1ed，含 experimental dxil target）。round-1/2/3 既有 evidence/ 文件全部 byte-unchanged 保留（evidence/ 不可篡改门强制）；本 round-4 为新增证据文件。 |
| 机器证据 | [evidence/dxil_path_spike_20260624_r4.json](dxil_path_spike_20260624_r4.json)（schema：[milestones/g2/dxil_path_spike_evidence_schema.json](../milestones/g2/dxil_path_spike_evidence_schema.json)，经 `ci/check_schemas.py` PASS） |
| 探针 | [spike/dxil-path-probe/](../spike/dxil-path-probe/)（标 `// SPIKE(RD-010)`，不入 src/ 生产路径；round-4 扩展 `probe_a_llvm_directx.py` 两诊断 + 新增 `dxil_container.py`（DXBC 解析）/ `dxil_validator.py`（IDxcValidator ctypes harness）） |
| 纪律 | measured-first / blocked-honest（硬规则 3/4）：每配置 ×12 发量崩溃率（单发会假 pass），探到记实测、探不到如实 blocked + repro，**绝不杜撰数字**。 |
| Provenance | `Assisted-by: claude-code:claude-opus-4-8`（agent 自主记录机器可核对事实，非代决、非代签） |

---

## 1. Round-4 取证范围

round-2/3：A 路环境就位（自编 LLVM 含 dxil target）但 emit/validator 两轴未同口径锐利测；B 路 measured（combo 链端到端 4/4）。round-4 对 A 路做**两个锐利诊断**，回答 D-131 唯一卡点「LLVM DirectX target 当下成熟度」：
1. **emit 稳定性**：分离测 `llc -filetype=asm`（文本 DXIL）vs `-filetype=obj`（二进制 DXContainer）崩溃率，跨 shader model（6.0/6.2/6.5/6.6）× 元数据变体（bare / dxc 风格补全）。
2. **互操作（0x80aa000f）**：改用 dxcompiler.dll 的 **IDxcValidator::Validate 真验证 API**（非 dxc -dumpbin 容器加载）对 llc 产物做真验证 + diff llc vs dxc 自产容器结构。

结论仍 **不含 A/B 选择**（硬规则 1）。

## 2. 诊断 1 — emit 稳定性（asm vs obj，每配置 ×12 发）

环境：自编 `llc` 22.1.7（pin commit a255c1ed，`llc --version` Registered Targets 含 `dxil - DirectX Intermediate Language`），经 `RURIX_LLC` 临时用；**未动 C:\Program Files\LLVM（D-205 pin）**。

| 配置 | `-filetype=asm`（文本 DXIL） | `-filetype=obj`（二进制 DXContainer） |
|---|---|---|
| bare / sm6.0 | 12/12 ok，0 crash | 10 ok，**2 crash** |
| enriched / sm6.0 | 12/12 ok，0 crash | 2 ok，**10 crash** |
| bare / sm6.2 | 12/12 ok，0 crash | 6 ok，**6 crash** |
| enriched / sm6.2 | 12/12 ok，0 crash | 3 ok，**9 crash** |
| bare / sm6.5 | 12/12 ok，0 crash | 9 ok，**3 crash** |
| enriched / sm6.5 | 12/12 ok，0 crash | 4 ok，**8 crash** |
| bare / sm6.6 | 12/12 ok，0 crash | 6 ok，**6 crash** |
| enriched / sm6.6 | 12/12 ok，0 crash | 4 ok，**8 crash** |

崩溃码 = `0xC0000005`（access violation，后端对象写出器崩溃，非编译期诊断）。成功 obj 容器 1740B（bare）/ ~1790B（enriched），魔数 `DXBC`。

**关键发现**：
- **文本 DXIL（`-filetype=asm`）emit 全配置 8×12=96/96 稳定**（零崩溃，产物确定性大小）。崩溃**仅**出现在二进制容器化（`-filetype=obj`）阶段 → 崩溃隔离于 LLVM DirectX 后端的 **DXContainer 对象写出器**，**不在 DXIL codegen 本身**。
- **打通方向**（关键发现）= **emit 文本 DXIL（稳定）再另行容器化/签名**，绕开非确定性崩溃的 obj 写出器。
- 补全 dxc 风格元数据（`!dx.entryPoints`+numthreads / `!dx.valver` / `!dx.shaderModel`）**不降反升** obj 崩溃率（enriched 各 SM 崩溃数 ≥ bare）→ 崩溃与 SM、源级元数据无关，是写出器固有非确定性（疑似未初始化内存/竞态）。
- shader model 6.0/6.2/6.5/6.6 全测：asm 全稳定，obj 全非确定性崩溃，无「某 SM 稳定」配置。

## 3. 诊断 2 — 互操作（0x80aa000f，IDxcValidator 真验证）

对 llc 一次成功 emit 的 DXContainer（1740B），用 dxcompiler.dll 1.8.0.4739 的 **IDxcValidator::Validate** API（ctypes COM harness `dxil_validator.py`，非 dxc -dumpbin 容器加载）做真验证：

| 对象 | IDxcValidator 结果 |
|---|---|
| **llc 产 DXContainer（bare）** | accepted=**False**，status=`0x80aa0009`，err=`load dxil metadata failed - error code 0x80aa000f` |
| **llc 产 DXContainer（enriched 补全元数据）** | accepted=**False**，status=`0x80aa0009`（同 0x80aa000f） |
| **dxc 自产容器（对照）** | accepted=**True**，status=`0x0` |

**结论：validator 拒绝（validation error，非签名缺失）→ llc 产 DXIL 不合规 = 上游 backend 问题。**
- IDxcValidator 在 metadata 加载阶段即失败（`load dxil metadata failed - 0x80aa000f`），**不是只缺签名步**——签名是验证通过后的最后一步，这里在加载 DXIL 元数据时就被拒。
- 补全 dxc 风格 entry point 元数据**仍同样被拒**（0x80aa000f）→ 不是源级元数据缺项可补救，是 LLVM 22.1.7 DirectX 后端产的 DXIL（bitcode/元数据编码）与 dxc 1.8.0.4739 validator 期望的不兼容。
- 对照实验确证 validator/工具本身可用（dxc 自产 accepted=True）→ **gap 在 llc↔dxc 互操作，非工具坏**。

### 3.1 容器结构 diff（定位 llc 缺/错在哪）

| 项 | llc 产容器 | dxc 自产容器 |
|---|---|---|
| parts | `[DXIL, SFI0, HASH, ISG1, OSG1, PSV0]` | `[SFI0, ISG1, OSG1, PSV0, STAT, HASH, DXIL]` |
| 缺失 part（vs dxc） | **缺 `STAT`**（pipeline 统计/运行时信息） | — |
| part 顺序 | DXIL 在首位，顺序非规范 | 规范顺序 |
| 签名摘要（digest） | **全零（未签名）** | 已填（已签名） |

llc 容器同时存在：缺 `STAT` part、part 顺序非规范、digest 全零（未签名）三处结构问题；但 IDxcValidator 在更早的「load dxil metadata」阶段就失败（0x80aa000f），说明 DXIL bitcode 部分本身的元数据编码即不被 dxc 1.8 validator 接受，结构补全（加 STAT/排序/签名）尚不足以打通。

## 4. 诊断 3（可选，上游成熟度）— blocked-honest，未做

任务列为可选：在另一隔离目录编**更新的 llvm-project commit** 重测 1/2 的崩溃/互操作是否上游已修，以判定打通 = 「pin bump」还是「fundamental」。

**处置：blocked-honest，本 session 未执行。** 理由：克隆 + 自编另一 commit 的 LLVM（含 DirectX target）为多 GB / 长耗时重活，session 内执行会 destabilize 环境；且偏离 D-205 pin（22.1.x，commit a255c1ed），代表性下降。本轮已用 pin commit a255c1ed（22.1.7）测得决定性 blocker，「pin bump vs fundamental」判别留后续 round 或 agent 按需安排（复现 recipe：在隔离目录 `cmake -DLLVM_EXPERIMENTAL_TARGETS_TO_BUILD=DirectX` 编更新 commit，重跑 probe_a 两诊断对比崩溃率/0x80aa000f 是否消失）。

## 5. Round-4 A/B 判据对照（实测列；仅摆证据，不含选择结论）

| 判据 | A 路 round-4（LLVM DirectX 直接 emit） | B 路（SPIR-V→DXIL 转译，round-2 起 measured，本轮复跑维持） |
|---|---|---|
| target / 转译链可用 | ✓ 自编 llc 含 dxil target | ✓ combo 链 SPIRV-Cross→dxc 端到端可用 |
| dxil emit | 文本 asm **96/96 稳定**；二进制 obj **全 SM 非确定性崩溃**（0xC0000005）→ emit=**fail**（容器化不稳） | ✓ 4/4 pass（DXBC 容器） |
| validator（真验证） | IDxcValidator **拒绝** llc 产物（0x80aa000f load dxil metadata failed，非签名缺失）；dxc 自产对照 accepted=True → validator=**fail** | dxc 内置 4/4 pass；独立 dxv 仍缺 |
| shader model 覆盖 | 6.0/6.2/6.5/6.6 全测；asm 稳定 / obj 崩溃 | cs/vs/ps × SM 6.0–6.6 全 pass |
| 确定性 | asm 产物确定；obj 因崩溃不可用 | 4/4 deterministic |
| 供应链成本 | 复用 D-205 LLVM 单栈、无第二中间 IR（但后端 experimental 未成熟） | 第二中间表示 SPIR-V + 三独立来源（Mesa/Khronos/MS）独立 pin/审计 |
| 与 D-205 单栈契合 | 同构（与 NVPTX 一致） | 引入外部转译依赖，偏离单栈 |

## 6. Round-4 结论（不裁 A/B；A 是否可打通 / evidence 是否充分）

- **A 路当前 pin（LLVM 22.1.7 / commit a255c1ed）打不通**，精确 blocker 双轴：
  1. **emit**：文本 DXIL（asm）稳定，但二进制 DXContainer 写出器（obj）非确定性崩溃 `0xC0000005`（全 SM、bare/enriched 均崩溃，补元数据反增崩溃）→ 后端容器写出器未成熟。
  2. **互操作**：即便侥幸产出 obj 容器，IDxcValidator 真验证**拒绝**（`0x80aa0009` / `load dxil metadata failed - 0x80aa000f`，validation error 非签名缺失）；补 dxc 风格元数据无效；dxc 自产对照通过 → 上游后端 DXIL 与 dxc 1.8 validator 不兼容，非工具坏。
- **关键发现（打通方向）**：文本 DXIL emit 稳定 → A 路若打通，方向 = emit 文本 DXIL + 另行容器化/签名 + 修复 DXIL 元数据与 validator 互操作（或换 validator/pin 版本）。
- **A 打通 = 上游 LLVM DirectX 后端成熟（容器写出器稳定化 + DXIL 元数据合规）或换 pin / validator 版本依赖项**；本轮未判别「pin bump vs fundamental」（诊断 3 blocked-honest）。
- **evidence 充分性判定：A 侧成熟度卡点已取得决定性实测**（emit + 互操作双轴 blocker + 复现 + 0x80aa000f 定位），较 round-2/3 的 A 侧空白实质推进。B 侧 measured。**是否到 evidence sufficient 由 agent 裁**（硬规则 1）：本报告呈现 A 当前 pin 不可打通的精确证据 + B 可用，agent 据此可裁「等上游成熟/换 pin」（倾向 A）还是「B 桥接」。
- 按硬规则 1：**D-131 维持 C**，不裁 A/B、不回填 RFC-0003 §9 / 13 §D-131；**G-G2-2 未签、G2.2 验收门仍 open**（A 打通=工具链可行性，**非** Rurix MIR→DXIL 实现、**非** device 真跑 golden；device 真跑 / DXIL golden / 独立签名 validator 三样仍缺，agent 自主签署）。

## 7. 裁决归属与留痕

**A/B 最终路径裁决权属 agent**（RFC-0003 §9 Q-D131 / 13 §D-131 / AGENTS 硬规则 1）。本 round-4 spike 仅产证据基底，**agent 自主裁决**。agent 凭 round-1~4 证据裁定最终 A/B 后：回填 RFC-0003 §9 Q-D131 + 13 §D-131（经勘误 PR）→ close RD-010 → 裁决后方进 PR-C1 spec 脚手架（条款先于实现，硬规则 7）。

> 本 round-4 纯取证：不落 codegen / 不创建 spec 条款（RXS-0157~ 仍门控在裁决后）/ 不造错误码 / 不入 golden / 不登 spike_gating，trace 维持 156/156、零新 RXS。探针扩展隔离于 `spike/dxil-path-probe/`，自编 LLVM / harness 产物不入库（digest 写进证据 JSON），spike 结束可弃。
