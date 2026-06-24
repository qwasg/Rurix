# DXIL 生成路径双路 Spike — Round-5 取证报告（G2.2，RD-010，RFC-0003 §9 Q-D131=C）

| 字段 | 值 |
|---|---|
| 类型 | **Spike 取证报告 round-5**（机器事实汇总 + 复现清单；非立项、非实现、非性能基准、非常驻 CI 门）。A/B 最终路径裁决由 **owner 人工裁决**（AGENTS 硬规则 1）；本报告只摆证据，**不含 A/B 选择结论**，**不裁是否 bump D-205 pin**。 |
| 本轮命题 | round-4 定位 A 路当前 pin（LLVM 22.1.7 / commit a255c1ed）双轴 blocker 后，诊断 3「换更新 LLVM 能否让 A 打通」blocked-honest 跳过。**本轮即补诊断 3**：在隔离目录自编**更新于 pin 的 llvm-project commit**，对更新 llc 重跑 round-4 两诊断，判 A 打通 = 「pin bump」还是「fundamental（上游未成熟）」。 |
| 承接 | 承 round-1/2/3/4。复用 round-4 探针扩展（`dxil_validator.py` IDxcValidator harness / `dxil_container.py` DXBC 解析 / probe_a 两诊断 / run_spike `_rN` 后缀）。round-1~4 既有 evidence/ 文件全部 byte-unchanged 保留（evidence/ 不可篡改门强制）；本 round-5 为新增证据文件。 |
| 机器证据 | [evidence/dxil_path_spike_20260624_r5.json](dxil_path_spike_20260624_r5.json)（schema：[milestones/g2/dxil_path_spike_evidence_schema.json](../milestones/g2/dxil_path_spike_evidence_schema.json)，经 `ci/check_schemas.py` PASS） |
| 隔离纪律 | 更新 LLVM 仅进**隔离目录** `H:\llvm-upstream-test\`（自编产物不入库，digest/commit 写进证据 JSON）；**未动** `C:\Program Files\LLVM`（D-205 pin）、**未覆盖** `H:\llvm-dxil`（round-3 pin-matched 基线，留作对照）、**未改** `src/` / `toolchain.rs`。 |
| 纪律 | measured-first / blocked-honest（硬规则 3/4）：每配置 ×12 发量崩溃率（单发会假 pass），探到记实测、探不到如实 blocked + repro，**绝不杜撰数字**。 |
| Provenance | `Assisted-by: claude-code:claude-opus-4-8`（AI 代录机器可核对事实，非代决、非代签） |

---

## 1. 更新 LLVM commit（隔离自编，精确可复现）

| 项 | round-4 基线（D-205 pin） | round-5 更新 commit |
|---|---|---|
| LLVM 版本 | 22.1.7 | **23.0.0git** |
| commit hash | `a255c1ed36a1d06f79bd2633ba9f8d900153007c` | **`82c5bce5233f964da4f8086b2341067314d841d7`** |
| commit 日期 | （pin，约 6 月前） | **2026-06-24T16:07:05+08:00（取当日 llvm-project 最新 main）** |
| 目录 | `H:\llvm-dxil`（不动，对照基线） | `H:\llvm-upstream-test`（隔离，自编产物不入库） |

构建 recipe（同 round-3）：`cmake -DCMAKE_BUILD_TYPE=Release -DLLVM_ENABLE_PROJECTS=clang -DLLVM_TARGETS_TO_BUILD=X86 -DLLVM_EXPERIMENTAL_TARGETS_TO_BUILD=DirectX -DLLVM_ENABLE_ASSERTIONS=OFF ...`，`ninja -j6 llc llvm-as`。更新 llc `--version` Registered Targets 含 `dxil - DirectX Intermediate Language`（DirectX 后端在 LLVM 23 仍 experimental，须本地编入）。经 `RURIX_LLC` 临时用。

> 代表性说明：取 llvm-project **最新 main**（DirectX 后端修复最多、距 pin 约 6 个月跨度），是「pin bump 能否打通」最强证据点——若最新上游仍 broken，则 bump 到任何中间 commit 也不会打通。

## 2. 诊断 1 — emit 稳定性（asm vs obj，每配置 ×12 发；与 round-4 同口径）

| 配置 | asm（文本 DXIL）r4 → r5 | obj（二进制 DXContainer）r4 crash → r5 crash |
|---|---|---|
| bare / sm6.0 | 12/12 → 12/12 稳 | 2 → **7** crash |
| enriched / sm6.0 | 12/12 → 12/12 稳 | 10 → **3** crash |
| bare / sm6.2 | 12/12 → 12/12 稳 | 6 → **7** crash |
| enriched / sm6.2 | 12/12 → 12/12 稳 | 9 → **6** crash |
| bare / sm6.5 | 12/12 → 12/12 稳 | 3 → **2** crash |
| enriched / sm6.5 | 12/12 → 12/12 稳 | 8 → **5** crash |
| bare / sm6.6 | 12/12 → 12/12 稳 | 6 → **8** crash |
| enriched / sm6.6 | 12/12 → 12/12 稳 | 8 → **9** crash |

崩溃码仍 `0xC0000005`（access violation，后端 DXContainer 对象写出器）。

**结论（emit 轴）= still-broken**：
- 文本 DXIL（`-filetype=asm`）96/96 仍全稳定（与 round-4 一致）。
- 二进制容器化（`-filetype=obj`）**全配置仍非确定性崩溃**——逐配置崩溃数在 r4/r5 间上下波动（非确定性的典型表征），**无任何配置变为零崩溃**。补 dxc 风格元数据仍不稳。
- 更新到最新上游 main **未修复** DXContainer 写出器的非确定性崩溃。

## 3. 诊断 2 — 互操作（IDxcValidator::Validate 真验证；与 round-4 同口径）

对更新 llc 一次成功 emit 的 DXContainer（1872B bare / 1924B enriched），用 dxcompiler.dll 1.8.0.4739 的 IDxcValidator::Validate 真验证：

| 对象 | round-4（22.1.7） | round-5（23.0.0git） |
|---|---|---|
| llc 产容器（bare） | reject `0x80aa0009` / load dxil metadata failed | **reject `0x80aa0009` / load dxil metadata failed（同）** |
| llc 产容器（enriched） | reject `0x80aa0009` | **reject `0x80aa0009`（同）** |
| dxc 自产容器（对照） | accept `0x0` | accept `0x0`（同） |

容器结构 diff 亦不变：llc 仍缺 `STAT` part、part 顺序非规范、digest 全零（未签名）。

**结论（互操作轴）= still-broken**：更新到最新上游 main，IDxcValidator 在 metadata 加载阶段（`load dxil metadata failed`）**仍拒绝** llc 产物，与 round-4 完全一致；dxc 自产对照仍 accept（validator/工具本身可用，gap 在 llc↔dxc 互操作）。

### 3.1 DXIL 版本子轴（本轮新增，反驳「validator 太旧」假说）

任务要求记录更新 llc emit 的 DXIL 版本，判断是否因「新 DXIL 版本 > dxc 1.8 dxil.dll 支持」而被拒（若属此情，拒绝是 validator 版本问题、非 llc codegen 之过）。从 DXIL part 的 `DxilProgramHeader` 实测：

| 来源 | program_version（SM） | dxil_version | shader_kind |
|---|---|---|---|
| 更新 llc（23.0.0git） | **6.0** | **0x100（DXIL 1.0）** | 5（compute） |
| dxc 1.8 自产（对照） | 6.0 | 0x100（DXIL 1.0） | 5（compute） |

**子轴结论：更新 llc 与 dxc 1.8 产出的 DXIL 版本完全相同（均 DXIL 1.0 / SM 6.0）。** 故 IDxcValidator 的拒绝**不是**因为 llc emit 了 dxc 1.8 不支持的新版本 DXIL——dxc 1.8 完整支持 DXIL 1.0。这**反驳了「partial（validator 版本太旧）」假说**：拒绝发生在 metadata 加载阶段，是 LLVM DirectX 后端产的 DXIL **元数据编码不合规**，与 DXIL 版本号无关。无需再用更新 dxil.dll 复验（版本无 gap）。

## 4. 同口径对照表（更新 commit vs round-4 pin；逐轴 fixed / partial / still-broken）

| 轴 | round-4 pin（22.1.7 / a255c1ed） | round-5 更新（23.0.0git / 82c5bce5） | 判定 |
|---|---|---|---|
| asm 文本 DXIL emit | 96/96 稳定 | 96/96 稳定 | 稳定（不变） |
| obj DXContainer 写出器 | 全配置非确定崩溃 0xC0000005 | 全配置非确定崩溃 0xC0000005 | **still-broken** |
| IDxcValidator 接受 llc 产物 | reject 0x80aa0009（metadata） | reject 0x80aa0009（metadata） | **still-broken** |
| 容器结构（STAT/顺序/签名） | 缺 STAT + 顺序异 + 未签名 | 缺 STAT + 顺序异 + 未签名 | **still-broken** |
| DXIL 版本 vs dxc 1.8 | （未测） | llc DXIL 1.0 == dxc DXIL 1.0 | 无版本 gap（非 partial） |

## 5. Round-5 判定（产证据，不下裁决）

按任务判定逻辑（两轴皆 fixed → 可靠 bump pin 打通 / partial → 记差什么 / still-broken → 最新上游仍 blocked）：

- **两轴皆 still-broken**：更新到 llvm-project 最新 main（23.0.0git，距 D-205 pin 约 6 个月、含期间全部 DirectX 后端修复），A 路双轴 blocker（obj 写出器非确定崩溃 + IDxcValidator 拒绝 llc DXIL 元数据）**均未消失**，与 round-4 pin 完全同症。
- **DXIL 版本子轴排除 partial**：llc 与 dxc 1.8 均产 DXIL 1.0 / SM 6.0，拒绝非「DXIL 版本 > validator 支持」，而是元数据编码不合规 → 不属「换更新 validator 即可」的 partial 情形。

**→ A 打通 = fundamental，不是陈旧 pin 问题：bump D-205 pin（哪怕到最新上游）单独不足以打通 A。** blocker 在上游 LLVM DirectX 后端自身成熟度（DXContainer 写出器稳定化 + DXIL 元数据与 dxc validator 互操作合规），需等上游成熟 / 向上游贡献修复 / 用 B 桥接——此三选一由 **owner 裁决**（硬规则 1）。

- 上游复现引用：blocker 可在隔离目录按 §1 recipe 自编最新 llvm-project main 复现（probe_a 两诊断），崩溃码 `0xC0000005`（obj 写出器）+ validator `0x80aa0009 / load dxil metadata failed`（互操作）。

## 6. 红线与门（本轮严守）

- **未改真 D-205 pin / 未动 toolchain.rs / 未动 C:\Program Files\LLVM / 未动 src/**——更新 LLVM 只进隔离目录 `H:\llvm-upstream-test`，自编产物不入库。
- **AI 不裁 A/B**（硬规则 1）；本轮纯取证：**不**落 codegen / **不**创建 spec 条款 / **不**造错误码 / **不**入 golden / **不**登 spike_gating；trace 维持 156/156、零新 RXS。
- **不**覆盖任何 evidence/ 既有文件（round-1~4 全 byte-unchanged）；新证据 `dxil_path_spike_20260624_r5.json` + 新报告 `dxil_path_spike_report_round5.md`。
- **不**签 / **不**翻 G-G2-2：A 打通 = 工具链可行性，**≠** Rurix MIR→DXIL 实现、**≠** device 真跑 golden；验收门仍 **open**。

## 7. 裁决归属与留痕

**A/B 最终路径裁决权属 owner**（RFC-0003 §9 Q-D131 / 13 §D-131 / AGENTS 硬规则 1）。本 round-5 spike 仅产证据基底，**AI 不代决**。

- **A 打通结论**：最新上游 main 仍 still-broken（fundamental，非 pin 陈旧）→ **是否 bump D-205 pin 仍 owner 裁**（D-205 是 13 决策项，真 bump = owner 决策 + 独立 errata，**不在本 spike**）；本轮证据表明「单纯 bump pin」不解决 A，owner 选项收敛为：等上游成熟 / 贡献上游 / B 桥接。
- **D-131 仍 C**（不回填 RFC-0003 §9 / 13 §D-131）。
- **G-G2-2 未签、G2.2 验收门仍 open**（device 真跑 / DXIL golden / 独立签名 validator 三样仍缺，AI 不代签）。

> 本 round-5 纯取证：探针扩展隔离于 `spike/dxil-path-probe/`，自编 LLVM / harness 产物不入库（commit hash 写进证据 JSON），spike 结束可弃。
