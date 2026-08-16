<!-- Assisted-by: Kimi-K3（G11.1 治理波起草） -->
# G11_PLAN — 画质修复期 主线分解

> **状态**：**计划起草稿（v1.0）——G11 未立项**。本文以 [G10_PLAN.md](../g10/G10_PLAN.md) v1.0 与 [G9_PLAN.md](../g9/G9_PLAN.md) v1.1 为波次结构/退出门/风险表范式起草；本文与 G11.1 治理波全部产物均不构成任何契约、验收承诺或编号 claim。G11 正式立项（用户指令 + 治理裁决 + §5 治理门）前，零 `src/`/`spec/`/`conformance/` 语义实现、零编号消费。
> **蓝本与上游**：[G10_CONTRACT.md](../g10/G10_CONTRACT.md) §8.10（G10 closed 终态，2026-08-16，flip commit `27e3b07c` + 幂等复跑批 `53eb3a28`）· [`g10_gap_registry.json`](../g10/g10_gap_registry.json)（**G11 法定输入**：11 行闭集终审锁定——quality_gap 8 行 R1~R5/U1~U3 + caliber_diff 3 行 C1~C3，每项带 UE5 模块归属 + measured delta + G11 承接锚）· [G10_P2_DECISIONS.md](../g10/G10_P2_DECISIONS.md) v1.0（27 行闭集；defer-to-G11+ 18 行承接锚）· [G10_DEFER_REEVALUATION.md](../g10/G10_DEFER_REEVALUATION.md) v1.0（M99-clipmap rejudged-go 承接锚字面）· [G10_ACCEPTANCE_MAP.md](../g10/G10_ACCEPTANCE_MAP.md) v1.0（验收映射范式）· [G10 CI_GATES](../g10/CI_GATES.md) v1.10（步骤编号已至 195；数字步骤纪律 = `post-interlock actual-next-free allocation`）· [RFC-0026](../../rfcs/0026-visual-comparison-metrics.md)（度量口径冻结面——修复后复测的对拍基准）· [RFC-0027](../../rfcs/0027-external-reference-harness-license.md)（外部参照 harness 与许可边界）· [渲染器调研](../../渲染器调研/) 七份报告（2026-07-28 快照；报告 2 = R4 GI 修复技术参照，报告 6 = 材质/纹理面参照）· [14_ENGINEERING_DISCIPLINE](../../14_ENGINEERING_DISCIPLINE.md) §5（证据分级：measured_local 优先，estimated 占位不得超 2 期——P-09）。
> **推进形态**：**严格波次**——G11.1 治理波 → G11.2 口径差对齐波 → G11.3 资产与场景面修复波 → G11.4 光照与 GI 修复波 → G11.5 A/B 复测波 → G11.6 P2 穷举 → G11.7a soak → G11.7b close-out。波次内可蜂群并行，波次间串行；spec-first + RED 先行；禁止 stub/mock/host substitution 抢跑。
> **本波边界（G11.1，governance-only）**：本波仅落 PLAN / 契约 / 候选决策 / 验收映射 / RFC / CI_GATES / measured baseline / validator 治理面——零 `spec/src/conformance` 改动、零 CI 步骤 materialize（验收门只写判据草案；数字 CI 步骤一律 `post-interlock actual-next-free allocation`，禁预占）。

---

## 1. 目标与口径

**G11 = 画质修复期**。G10~G15 总体分期（G10 立项裁决口径）：G10 = 画面对标基线期（已 closed）→ **G11 = 画质修复期** → G12 = 路径追踪生产化期 → G13 = 超分与 DLSS 期 → G14 = 性能优化期 → G15 = 商用收口期。G10 已交付：UE 5.8.1 出图环境（F:\UE_5.8，M128/M129 门绿）+ 压测语料（CornellBox + BistroInterior，许可白名单冻结，M131~M133 门绿）+ 度量基建（EXR/FLIP/SSIM/PSNR/diff，M134~M138 门绿，标定值入 g10_budget）+ 首轮 A/B（M139~M141 门绿）+ **measured 差距清单 11 行终审锁定（G11 法定输入）**。

**修复范围唯一法定来源**：`milestones/g10/g10_gap_registry.json` 11 行闭集（R1~R5/U1~U3/C1~C3）+ 每项 `g11_anchor` 承接锚字面。G11 不得无锚新立修复项；修复候选池 = 该闭集，不多一行、不少一行。G10 defer-to-G11+ 18 行（[G10_P2_DECISIONS](../g10/G10_P2_DECISIONS.md) §1/§3）逐行处置归 §2 G11.1 候选决策表与 G11.6 穷举，不构成新修复项。

**G11 与 G10 的关键差异（验收口径）**：G10 不设画质通过线、差距全量登记即绿；**G11 设修复闭环判据**——每差距项的独立闭环门断言「修复前后度量 delta 收敛（measured）」：复测 delta 相对 G10.8b 锁定基线 delta 收敛，收敛阈值由 G11.2/G11.5 标定程序 measured 产出（禁手写，P-09）。**G11 仍不设绝对画质通过线**——「已达 UE5 画质」的绝对判定归 G15 商用收口期；G11 只断言修复闭环（delta 收敛 measured）+ 回归不降级，不断言绝对画质达标。

**成功判据（草案；G11.1 治理波硬化为契约验收门，本波不 materialize CI 步骤）**：

1. **口径差对齐**（G11.2）：C1 室内亮度口径差（GI/天光遮蔽口径 + 太阳 lux→辐射度链）/ C2 曝光链派生尺度 / C3 EXR 位深三行逐行对齐闭环——口径不对齐则修复无法被度量验证（R-G10-6 教训字面）；HDR-FLIP 独立标定（G10-N10 承接锚兑现，标定值按 M138 同程序入 g11_budget）。
2. **资产与场景面修复闭环**（G11.3）：R1 材质子集（baseColorTexture/法线/metallic-roughness 采样）/ R2 几何法线（winding 朝向 + 双面翻转）/ R5 JSON u64 seed 解析 / U1 cornell 壳体零辐射 / U2 bistro 纹理（DDS 面，G10-N7 承接锚兑现）/ U3 动画剥离，六行逐行修复闭环。
3. **光照与 GI 修复闭环**（G11.4）：R3 灯种子集（点/面光源 + glTF emissive 表达）/ R4 GI 多反弹 + **M99-clipmap 世界辐射缓存世界级承接**（G10.6 rejudged-go 字面：屏幕探针远场缺失已实证为画质 measured 问题，G11 承接世界 clipmap 级；语义面经 Full RFC-0028 冻结）。
4. **A/B 复测闭环**（G11.5）：同契约双端复跑（契约参数 digest == G10.5 锁定值，修复不得动契约参数）→ 复测度量报告 + 复测差距清单 → 11 行逐项闭环核验（修复前后 delta 收敛 measured）。
5. **回归不降级**：修复不得降级既有绿面（G9 34 key + G10 14 key = 48 门）；G5~G10 closed 判据 0-byte。
6. **measured 纪律**：`g11_budget.json` 非空 measured_local、零 estimated（P-09）；全部闭环阈值实测标定；环境画像随证据存档。
7. **UE 零 vendoring**：UE 源码仅外部参照只读；修复实现零 UE 片段复制。

**out-of-scope**：

| 项 | 依据 |
|---|---|
| 绝对画质通过线（「已达 UE5 画质」判定） | G15 商用收口期承接；本计划 §1 口径——G11 只断言修复闭环 delta 收敛 |
| 路径追踪生产化实施 | G12 承接；M143 档案已落 G10 |
| DLSS/超分接入实施 | G13 承接；G10-N5 方向登记维持（锚定 G13） |
| 性能优化实施与帧率通过线 | G14 承接；G10-N11/N16 帧率面锚定 G14 维持 |
| 无锚新立修复项 | 法定输入 = 锁定清单 11 行 + 承接锚（G-G10-11/MAP §7 字面） |
| GPU 管线双端 A/B 出图面 | G10-N16 锚定 G14；G11 复测臂 = 同 G10.5 host CPU 参考管线（复测对照口径一致） |
| UE 源码/二进制 vendoring | RFC-0027 许可边界；零 vendoring |
| 任何编号（RXS/RD/U/RX/CI step/RFC）推测性消费 | 立项时按实测 `next_free` 领取；数字 CI 步骤一律 `post-interlock actual-next-free allocation`（G10 CI_GATES §1.2 纪律继承） |
| G10 帧库/语料契约参数回写 | 复测对照口径：契约参数 digest == G10.5 锁定值；语料修订走 M133 只追加修订程序 |

---

## 2. 波次分解

```text
G11.1 治理波（本波，governance-only：契约四件套 + 候选决策 + 验收映射 + RFC + measured baseline + validator）
  → G11.2 口径差对齐波（C1/C2/C3 逐行对齐闭环 + HDR-FLIP 独立标定——先对齐口径否则修复无法被度量验证）
  → G11.3 资产与场景面修复波（R1 材质子集 / R2 几何法线 / R5 i64 / U1 壳体 / U2 纹理〔DDS 面〕/ U3 动画）
  → G11.4 光照与 GI 修复波（R3 灯种子集 / R4 多反弹 GI + M99-clipmap 世界级辐射缓存承接）
  → G11.5 A/B 复测波（同契约双端复跑 + 复测度量报告 + 复测差距清单 + 11 行逐项闭环核验）
  → G11.6 P2 穷举决策（G11 期新产生分项，defer 必有承接锚）
  → G11.7a soak → G11.7b close-out（复测差距清单终审锁定 → G12+ 法定输入候选面）
```

**单点依赖声明**：G11.2 是全部修复波的硬前置——口径差不对齐，则修复前后 delta 的度量基准不可比（C1 ≈21× 亮度口径主差未对齐前，任何 GI/光照修复的闭环断言都会被口径噪声淹没）；G11.5 是全部修复闭环的统一核验面（各修复波内门先行断言修复落盘 + 局部度量，G11.5 同契约复跑统一产出复测差距清单并逐项闭环核验）。

### G11.1 — 治理波（governance-only，与 G10.1/G9.1 同构）

交付：

1. **用户立项指令留痕 + agent 治理裁决**；§5 治理裁决表项全部落定；G11 文档集不可变 ref 登记。
2. **`G11_CONTRACT.md`**：契约四要素 + front matter 状态机（`implementation_status: blocked` 起始）+ §8 只追加条款结构。
3. **`G11_CANDIDATE_DECISIONS.md`**：法定输入逐行映射——① G10.8b 锁定差距清单 11 行 → 修复波次/P 级判档；② G10 defer-to-G11+ 18 行 → G11 裁决；③ 存续 open RD（RD-034/039/040/041/042/043/044）逐条映射；④ G11 新增候选。缺行不得开工 G11.2。
4. **`G11_ACCEPTANCE_MAP.md`**：全部 P0（及 go 的 P1）的 `M## → symbolic key → 稳定脚本 → evidence schema → 判据`；缺行阻断 G11.2。
5. **Full RFC**（实际编号按立项时 `registry/number_ledger.json` 实测 `next_free` 领取，禁止推测号）：M99-clipmap 世界级辐射缓存承接触 `spec/global_illumination.md` 冻结面（RXS-0360 世界级 not-triggered 登记翻转 + RXS-0357 门序面）——判档 **Full RFC**（判档争议向上取严）；R4 多反弹 GI 语义与 R3 灯种子集表达同伞。材质/纹理面若触 spec 冻结面同判，否则 Direct PR 面；DDS 纹理解码不触语义冻结面走 Direct PR 面。
6. **RTX 4070 Ti measured baseline → `g11_budget.json` 非空**（P-09：沿 G10.1 baseline 锚复测重登记 + 新增 G11 修复闭环面基线锚——11 行锁定差距 measured delta 转录为闭环基线锚，零 estimated）。
7. **`CI_GATES`** + G11 专属 validator（implementation interlock 诚实 BLOCKED→READY 两态 + acceptance map 三向比对 + wave_exit_lib + p2_decisions 骨架——重评窗不需：G10 重评窗是 G9 十锚的承接窗，G11 法定输入为锁定清单直消费，无独立重评窗波次；数字 CI 步骤一律 `post-interlock actual-next-free allocation`）。

退出门（判据草案，防假绿）：§5 治理裁决表项全落定并留痕；四件套齐备且互锁 validator 诚实输出 BLOCKED→READY；候选决策表与验收映射无缺行；零数字 CI 步骤 materialize；registry/spec/src/conformance 0-byte（登记/翻转/history 追加归立项治理动作除外）。

### G11.2 — 口径差对齐波

| 面 | 内容 | 矩阵 |
|---|---|---|
| C1 亮度口径对齐 | GI/天光遮蔽口径差（UE SkyLight 指定 cubemap 全向 IBL vs Rurix 屏幕探针单反弹）+ 太阳 lux→辐射度链差逐行登记对齐口径——不拟合、只对齐：双端天光/太阳辐照链参数化对齐 + 残余口径差显式登记（修复后复测的可比基准） | M144 |
| C2 曝光链对齐 | 双端 EV100 同字面下派生尺度对齐（Rurix 臂 2^(−EV100) vs UE 臂 pipe 内手动曝光已施 ×1.0——派生链统一或显式互证登记） | M145 |
| C3 位深对齐 | UE EXR fp16→f32 提升口径（RXS-0385 strip-and-log）与 Rurix 原生 f32 的度量域对齐登记 | M146 |
| HDR-FLIP 独立标定 | HDR 域正式对拍样本集（真实 HDR 帧双臂）+ 标定值按 M138 同程序（p100×k measured）入 `g11_budget.json`（G10-N10 承接锚兑现；G10-N10 重判条件已含「G11+ HDR 域正式对拍样本集 + 标定值入 budget」字面） | M157（P1） |

退出门（判据草案）：`g11.p0.m144` / `g11.p0.m145` / `g11.p0.m146` 门绿——三行口径差逐行对齐闭环（对齐后残余口径差显式登记，不得以未对齐口径消费复测 delta）；`g11.p1.m157` 门绿——标定程序可复跑 + 标定值入 budget provenance 齐备（P-09，禁手写阈值冒充标定即 RED；estimated 冒充 measured 即 RED）。

### G11.3 — 资产与场景面修复波

| 面 | 内容 | 矩阵 |
|---|---|---|
| R1 材质子集修复 | baseColorTexture/法线/metallic-roughness 采样接入（A/B harness host 参考管线消费面；触 spec 冻结面则升级 Full RFC，否则 Direct PR 面——判档见 §5 表项 6） | M147 |
| R2 几何法线修复 | winding 朝向 + 双面翻转消费（与 U1 同面——cornell 壳体单面片绕向 × 双面口径交互差的双端双侧） | M148 |
| R5 JSON 整数解析修复 | u64 顶格 seed 解析（i64 域 fail-closed → u64 全域合法消费；契约 seed=42 既有面 digest 不变回归） | M149 |
| U1 cornell 壳体零辐射修复 | 壳体（墙/顶/地板）零辐射修复——语料派生面（M133 只追加修订程序）或双端着色口径对齐面，修复后 UE 帧覆盖回归 measured | M150 |
| U2 bistro 纹理修复 | DDS 纹理解码面（G10-N7 承接锚兑现：解码面形态重判——Direct PR 面不触语义冻结面；UE 侧 Interchange 不可消费面 = 派生链转码或解码接入，材质实例 texture_parameter_values 非空回归） | M151 |
| U3 动画剥离修复 | Bistro 动画 Take 001 / glTF 相机节点消费面（动画通道消费或显式静态契约登记闭环） | M152 |

退出门（判据草案）：六门各自独立绿——修复落盘（只消费锁定清单对应行 + 承接锚）+ 修复前后局部度量 delta 收敛 measured（收敛阈由标定程序产，禁手写）+ 修复不得动契约参数 digest（相机/光照/seed 锁定值 0-byte）+ 语料修订走 M133 只追加修订程序 + 不降级既有 48 门绿面。统一闭环核验归 G11.5（本波门为修复落盘 + 局部度量面，不以局部绿色冒充 G11.5 复测闭环）。

### G11.4 — 光照与 GI 修复波

| 面 | 内容 | 矩阵 |
|---|---|---|
| R3 灯种子集修复 | 点/面光源 + glTF emissive 表达（bistro 包内 pointLight1~N 实测 4+ 盏与 emissive surfaces 消费；cornell 语料契约 sun+sky 面 0-byte） | M153 |
| R4 GI 多反弹修复 + M99-clipmap 承接 | GI 自屏幕探针单反弹 → 多反弹：世界辐射缓存世界级 clipmap 级承接（G10.6 rejudged-go 字面——空间哈希世界缓存 + 辐射 LOD + 屏幕缓存失效回落，调研报告 2 P2 蓝本 GI-1.0/SHaRC 与 Lumen Radiance Cache 双级参照；spec/global_illumination.md RXS-0360 世界级 not-triggered 登记翻转经 RFC-0028 显式修订行，RXS-0357 门序面衔接） | M154 |

退出门（判据草案）：两门独立绿——修复落盘 + RFC-0028 语义面 spec-first 条款落地 + 修复前后 HDR 域度量 delta 收敛 measured（HDR-FLIP 标定值消费面）+ 不以屏幕级 SPG + Radiance Cache 既有绿色（g9.p1.m99 门绿）冒充世界级验收 + 不降级既有 48 门绿面。

### G11.5 — A/B 复测波

| 面 | 内容 | 矩阵 |
|---|---|---|
| 同契约双端复跑 | 契约参数 digest == G10.5 锁定值（cornell `sha256:80305791…` / bistro `sha256:ad45951b…`，修复不得动契约参数）；双端出图链路复跑（同 G10.5 host CPU 参考管线臂 + UE 5.8.1 MRQ 臂，GPU 管线面锚定 G14 不动） | M155 |
| 复测度量与差距清单 | 复测度量报告（FLIP/SSIM/PSNR + HDR 域指标 + diff 报告）+ 复测差距清单落盘（11 行逐项：修复后 delta vs 锁定基线 delta 收敛状态 + 行状态翻转登记；新差距项显式登记即 RED 评审面——不得静默混入） | M155 |
| 逐项闭环核验 | 11 行修复闭环门统一核验：每行复测 delta 收敛 measured 断言（收敛阈由标定程序产）+ 回归门（既有 48 门最新 evidence 全绿 + 修复触改面既有门重跑回归） | M155/M156 |

退出门（判据草案）：`g11.p0.m155` / `g11.p0.m156` 门绿——复测差距清单 11 行闭集零空行（行集 == G10.8b 锁定清单行集逐字对账）+ 每行闭环状态机核 + 契约 digest 不等仍出报告即 RED（门序硬约束继承 M130/M139）+ 单端缺帧聚合不得 PASS + 回归门既有 48 门绿面零降级。

**M155 门预备注记（G11.3 收口裁决只追加登记；契约 §8.3a 修订句为裁决事实源——本注不改动 M155 行字面，M155 门 G11.5 才 materialize）**：R1 行（M147）局部 SSIM delta 收敛断言经双 phase 修订后移至本波——M155 必须对复测差距清单 R1 行给出修复前后 SSIM delta 收敛断言（锁定基线 = bistro LDR SSIM delta 0.8328980787837229；阈值标定程序产，禁手写；definitive 测量面 = 本波同契约复跑，RXS-0393 L2 quality_gap 款字面）；**R1 行不收敛则整波 FAIL**（判据不弱化只后移——M147 门 `--phase g11.3` 登记面绿不替本断言充绿，`--phase g11.5` 收敛断言面在本波 materialize 时兑现；「锁定度量对正确修复结构性不友好」登记为 G11.6 P2 候选行，反向激励旁证 0.1624318277352612 > 0.009656442299775102 入证据链）。

### G11.6 — P2 穷举决策（软门，必须穷尽）

对 G11 期内新产生的 P2/留档/未触发分项逐条 go/no-go/defer-to-G12+（不得遗漏；defer 必有承接锚，机核同构 `ci/g10_p2_decisions_check.py`）。候选行集 = G11.1 决策表校准后冻结 + G11.2~G11.5 期内新增分项。软门失败/未触发 → 诚实登记不阻塞 G11.7a；close-out 审计要求本表无空行。

### G11.7a — Stabilization / soak（close 前必经）

- 全 P0 硬门回归 + 已 go 的 P1 回归；G5~G10 既有判据 0-byte。
- 修复链路全量复跑 soak：复测出图/度量/差距清单装配全链路连续复跑（量级沿 G10.8a 继承〔≥1800s〕或 measured 证明更短足够，具体阈值 G11.1 裁决 measured 标定）。
- `budget_eval --strict` 非空、零 estimated/skip。
- 条件实现刚绿**不得**当日进 7b；**G9.8b/G10.8b 同日放行先例是否继承属治理裁决（§5 表项 7）**。

### G11.7b — Close-out

- 验收映射终审；RD 分项最终状态与 G11.1/G11.6 决策表逐字一致。
- **复测差距清单终审锁定 → G12+ 法定输入候选面**（残余差距/未闭环行如实登记，不冒充全闭环）。
- 契约 §8 只追加 + status flip。

### 2.9 P0 → 硬门覆盖（判据草案；G11.1 固化为 ACCEPTANCE_MAP 时方可 materialize CI 步骤）

任一 P0 无独立硬门 → **禁止** G11.7b status flip。数字 CI 步骤一律 `post-interlock actual-next-free allocation`（G10 CI_GATES v1.10 已至步骤 195；G11 编号自互锁后实测 `next_free` 顺位领取，禁预占）。

---

## 3. P0 建议清单（13 行，M144 起顺延 G10 矩阵 M128~M143；G11.1 决策表重裁后硬化）

| P0 | 名称 | 独立硬判据（草案） | 负例 RED 臂要求 | device/host 性质 | 最晚波次 |
|---|---|---|---|---|---|
| M144 | C1 室内亮度口径对齐闭环门 | GI/天光遮蔽口径差 + 太阳 lux→辐射度链差逐行对齐（对齐后残余口径差显式登记）+ 对齐前后口径参数 provenance 齐备 | 未对齐口径消费复测 delta 即 RED；拟合冒充对齐即 RED；残余口径差未登记即 RED | host 纯 host | G11.2 |
| M145 | C2 曝光链派生尺度对齐闭环门 | 双端 EV100 同字面下派生尺度对齐（Rurix 臂 2^(−EV100) vs UE 臂 ×1.0 统一或显式互证登记）+ 派生链元数据互证回归 | 派生尺度未对齐出 LDR 度量即 RED；互证链断裂即 RED | host 纯 host | G11.2 |
| M146 | C3 EXR 位深对齐闭环门 | UE fp16→f32 提升口径（RXS-0385 strip-and-log）与 Rurix 原生 f32 度量域对齐登记 + 位深元数据闭集回归 | 位深截断注入即 RED；元数据缺字段即 RED | host 纯 host | G11.2 |
| M147 | R1 材质子集修复闭环门 | baseColorTexture/法线/metallic-roughness 采样接入 + 修复前后 LDR 臂度量 delta 收敛 measured（bistro LDR SSIM 基线 0.8328980787837229）+ 契约 digest 0-byte | 未采样冒充修复即 RED；delta 未收敛冒充闭环即 RED；契约参数漂移即 RED | host+device | G11.3 |
| M148 | R2 几何法线修复闭环门 | winding 朝向 + 双面翻转消费 + 修复前后 cornell HDR 覆盖 delta 收敛 measured（基线 −0.7451210021972656）+ 与 U1 同面对账 | 法线未消费冒充修复即 RED；delta 未收敛冒充闭环即 RED | host+device | G11.3 |
| M149 | R5 JSON u64 seed 修复闭环门 | u64 顶格 seed 合法消费（i64 域 fail-closed 解除）+ 既有 seed=42 契约 digest 不变回归 + u64 边界语料锚定 | 顶格 seed 仍拒绝即 RED；既有 digest 漂移即 RED | host 纯 host | G11.3 |
| M150 | U1 cornell 壳体零辐射修复闭环门 | 壳体零辐射修复（M133 只追加修订程序或口径对齐面）+ 修复后 UE 帧覆盖回归 measured（基线 18.39% → 收敛）+ Rurix 侧 92.90% 面不降级 | 语料静默改写即 RED；覆盖未收敛冒充闭环即 RED；Rurix 侧降级即 RED | host+device | G11.3 |
| M151 | U2 bistro 纹理修复闭环门 | DDS 解码面落地（G10-N7 承接锚兑现）+ 材质实例 texture_parameter_values 非空回归 + 修复前后 LDR 臂度量 delta 收敛 measured（bistro LDR 亮度中位基线 0.7698879749655723） | 纹理仍全缺冒充修复即 RED；未登记资产混入即 RED；delta 未收敛冒充闭环即 RED | host+device | G11.3 |
| M152 | U3 动画剥离修复闭环门 | 动画通道消费或显式静态契约登记闭环 + 包内动画通道计数对账（基线 0 vs 2）+ 相机位姿契约 0-byte | 动画通道静默丢弃冒充闭环即 RED；相机契约漂移即 RED | host 纯 host | G11.3 |
| M153 | R3 灯种子集修复闭环门 | 点/面光源 + glTF emissive 表达（bistro 4+ 盏实测消费）+ 修复前后 HDR 亮度中位 delta 收敛 measured（基线 2.664779790997505）+ cornell sun+sky 契约面 0-byte | 点光源未表达冒充修复即 RED；delta 未收敛冒充闭环即 RED；契约灯面漂移即 RED | host+device | G11.4 |
| M154 | R4 GI 多反弹 + M99-clipmap 世界级辐射缓存修复闭环门 | 世界辐射缓存世界级 clipmap 级落地（RFC-0028 语义面 spec-first）+ 修复前后 HDR 亮度 p90 delta 收敛 measured（基线 4.697253086805343）+ 不以 g9.p1.m99 屏幕级绿色冒充世界级验收 | 世界级未落地冒充承接即 RED；屏幕级绿色冒充世界级即 RED；delta 未收敛冒充闭环即 RED | host+device | G11.4 |
| M155 | A/B 复测闭环门 | 同契约双端复跑（契约 digest == G10.5 锁定值）+ 复测度量报告 + 复测差距清单 11 行闭集（行集逐字对账）+ 逐项闭环状态机核 | 契约 digest 不等仍出报告即 RED；清单缺行/新项静默混入即 RED；单端缺帧聚合 PASS 即 RED | host+device | G11.5 |
| M156 | 修复回归门 | 既有 48 门（G9 34 + G10 14）最新 evidence 全绿只读汇总 + 修复触改面既有门重跑回归零降级 | 既有门降级即 RED；汇总遮蔽子断言 FAIL/SKIP 即 RED | host 纯 host | G11.5 |

P1 建议（go 1 行）：**M157 HDR-FLIP 独立标定门**（G11.2；G10-N10 承接锚兑现——HDR 域正式对拍样本集 + 标定值按 M138 同程序 p100×k measured 入 `g11_budget.json`，provenance 齐备；手写阈值冒充标定即 RED；estimated 冒充 measured 即 RED；标定程序不可复跑即 RED；host 纯 host）。

---

## 4. 风险与止损

| ID | 风险 | 预警 | 止损 |
|---|---|---|---|
| R-G11-1 | 口径差未对齐即修复——修复闭环断言被口径噪声淹没（C1 ≈21× 主差未对齐前 GI/光照修复无法被度量验证） | G11.3/G11.4 修复门 delta 异常 | G11.2 硬前置（单点依赖声明）；残余口径差显式登记；未对齐口径消费复测 delta 即 RED |
| R-G11-2 | 修复动契约参数——复测对照失效（相机/光照/seed 漂移则复测 delta 无对照意义） | 复测契约 digest ≠ G10.5 锁定值 | 契约 digest 门序硬约束（不等仍出报告即 RED）；修复门内嵌契约 digest 0-byte 断言 |
| R-G11-3 | 语料修订破坏 M133 冻结面（U1 修复需动 cornell 语料派生面） | 清单 digest 漂移 / 加载 golden 漂移 | M133 只追加修订程序（清单变更必有修订行）；Rurix 侧 92.90% 覆盖面不降级断言；未注册 digest 冒充冻结即 RED |
| R-G11-4 | 世界辐射缓存工程量失控（M99-clipmap 世界级 = 调研报告 2 P2 级工作量；Lumen 本体 140 pass 级不可照抄） | G11.4 波工期膨胀 | RFC-0028 冻结最小承接面（空间哈希世界缓存 + 辐射 LOD + 屏幕缓存回落，GI-1.0/SHaRC 蓝本）；兜底 = 屏幕级 SPG + Radiance Cache（g9.p1.m99 门绿）维持不冒充世界级；规模失控则按只追加程序登记 G12+ 承接 |
| R-G11-5 | DDS 解码面形态失控（UE Interchange 不消费 .dds——派生链转码 vs 解码接入选型；许可面复核） | U2 修复两臂漂移 | G10-N7 承接锚兑现 = G11.3 重判形态；Direct PR 面不触语义冻结面；未登记资产混入即 RED（M131 白名单面联动） |
| R-G11-6 | 修复降级既有绿面（48 门） | 修复波后既有门 FAIL | M156 回归门独立 P0（只读汇总 + 触改面重跑）；波聚合门不遮蔽子断言 FAIL/SKIP；G5~G10 closed 判据 0-byte |
| R-G11-7 | 闭环阈值手写（收敛判据失去 measured 根基） | 闭门槛值无标定 provenance | 收敛阈值一律标定程序 measured 产（M138 同程序 p100×k）；手写阈值冒充标定即 RED；estimated 冒充 measured 即 RED |
| R-G11-8 | 复测新差距项静默混入（修复引入新缺口未登记） | 复测清单行集 ≠ 锁定清单行集 | 行集逐字对账；新差距项显式登记即 RED 评审面；不得以修复名义删除锁定行 |
| R-G11-9 | 异己并发工作树面混入（立项时工作树带异己会话 src/ 未提交面） | G11 车道 commit 混入异己面 | 带未提交项立项登记（§5 表项 1）；G11 commit 只含 G11 车道文件；异己面保持不混入（G10.8b §8.10 先例同模） |
| R-G11-10 | UE 出图环境漂移（F:\UE_5.8 版本/许可变化；HighResShot/csvCaptureFrames 死路复活误判） | UE build digest ≠ M128 登记值 | 复测臂 UE build digest == M128 登记 ue_build_id 机核（G10.5b 门序面继承）；MRQ 主路臂维持（G10-N9 死路不复活作证据面） |
| R-G11-11 | HDR-FLIP 标定样本集判别力稀释（伪绿通道——样本集下界不足则标定值无判别力） | 标定值与复测 delta 判别力不符 | 样本集下界 + digest 入 evidence（RXS-0389 图集下界口径继承）；标定两跑逐位一致；provisional 标记消费登记 |

---

## 5. 治理裁决表项（供契约 §7 登记；本计划不定案，裁决权归用户/立项治理）

| # | 事项 | 候选口径 | 不定案原因 |
|---|---|---|---|
| 1 | G11 立项时机与工作树处置 | 带未提交项立项（G11.0 不可变 ref = `53eb3a28`；异己会话 src/ 未提交面〔rurix-asset/rurix-render geometry/gi/shadow/ssr/ktx2/hzb/restir/sdf_trace/smrt 声明面〕保持不混入 G11 车道）or 先处置再立项 | 处置方式影响 G11 文档集不可变 ref 的基线内容；异己面归属权不在 G11 车道 |
| 2 | 修复闭环判据形态 | 修复前后度量 delta 收敛 measured（复测 delta < 锁定基线 delta，收敛阈由标定程序产）；不设绝对画质通过线（归 G15） | 涉及 G11/G15 边界严肃性，须立项显式裁决 |
| 3 | M99-clipmap 承接口径确认 | G10.6 rejudged-go 逐字承接——G11.4 承接世界辐射缓存世界 clipmap 级（只消费 G10.8b 锁定清单 R4/C1 行 + 承接锚）；兜底 = 屏幕级 SPG + Radiance Cache 维持不冒充 | 承接面规模与语义冻结形态须立项确认 |
| 4 | G10 defer 18 行逐行处置 | 画质修复相关行承接（N7 DDS→U2 面 / N10 HDR-FLIP→G11.2 / N17 演进位 G11.5 触发评估 / N16 GPU 管线面锚定 G14 维持 / N6 Exterior 语料扩容触发评估 / N8 renderoffscreen 维持 defer / N11 锚定 G14）；十锚 M99-clipmap 承接确认、其余九锚维持 defer 承接锚字面 0-byte | 逐行裁决权须立项留痕 |
| 5 | RFC 判档 | GI 面 = Full RFC（M99-clipmap 触 spec/global_illumination.md 冻结面，向上取严）；R1 材质/R5/U1/U2/U3/C1~C3 修复 = Direct PR 面（不触 spec 语义冻结面；触则升级 Full RFC）；DDS 解码 = Direct PR 面 | 判档争议向上取严；升级触发条件须立项登记 |
| 6 | R1 材质修复触 spec 面升级条款 | 修复限 A/B harness host 参考管线消费面 → Direct PR；若波及 GPU 材质着色语义面（MaterialClosure 32B / display_pipeline 冻结面）→ 升级 Full RFC 显式修订行 | 实现波实际触及面不可预知，升级程序须立项登记 |
| 7 | G9.8b/G10.8b 同日放行先例继承 | 继承：7a full-run 先行完成后允许同日进 7b（G8.8b/G9.8b/G10.8b 先例链）or 不继承 | 涉及 soak 严肃性口径，须立项显式裁决 |
| 8 | 复测臂口径 | 同 G10.5 host CPU 参考管线臂 + UE 5.8.1 MRQ 臂（GPU 管线双端面锚定 G14 不动；G10-N16 承接锚字面） | 复测对照一致性须立项确认 |
| 9 | 资产入库形式与编号纪律确认 | 压测资产二进制不入 git（外部缓存 K: 盘 + 仓库内元数据登记，沿 G10 裁决 9）；数字 CI 步骤 `post-interlock actual-next-free allocation` 重申 | 编号纪律须立项重申 |

---

## 6. 与既有面的边界（0-byte 纪律）

| 面 | 约束 |
|---|---|
| G10 车道 | G10 四件套/决策表/重评窗表/evidence schema/budget/差距清单 0-byte（G10 closed）；锁定清单只消费不回写；G10_P2_DECISIONS 27 行裁决字面 0-byte 不回写 |
| G5~G9 冻结面 | G5~G9 closed 契约与判据 0-byte；触冻结面必须显式 RFC 修订行（G10 纪律继承）；spec/global_illumination.md RXS-0360 世界级登记翻转只经 RFC-0028 修订行 |
| 00–14 | 只勘误，独立提交 |
| spec/conformance | G11.1 期 0-byte；spec-first + RED 先行自 G11.2 实现门开放后起，spec 条款 PR 先于实现 PR |
| 注册表 | G11.1 期仅立项治理动作（deferred history 只追加 / number_ledger reserved_in_flight 登记与命名空间校准）；既有条目四字段 0-byte，禁静默改判 |
| 编号 | G11.1 期零数字 claim（RFC 号按立项实测 `next_free` 领取为例外面——Full RFC 治理波 materialize 先例沿 G9.1/G10.1）；数字 CI 步骤一律 `post-interlock actual-next-free allocation`；一切编号以领取时实测 `next_free` 为准 |
| 契约参数与帧库 | 复测契约参数（相机/光照/seed/post）digest == G10.5 锁定值 0-byte；G10 帧库（K: 盘）只读消费；语料修订走 M133 只追加修订程序 |
| UE 源码 | `E:\Kimi_Agent_Taichi Engine 优化计划\references\UnrealEngine` 只读外部参照；零 vendoring、零片段复制（RFC-0027 字面） |
| 修复范围 | 只消费 G10.8b 锁定清单 11 行 + 承接锚；不得无锚新立修复项；新发现差距进复测清单显式登记 + G11.6 穷举 |
| 调研引用 | [渲染器调研](../../渲染器调研/) 七份为 2026-07-28 快照；G11.1 复核关键引用时效（沿 G9 R-G9-3/G10 模式） |
| unsafe | U 段按立项 next_free；`rurix-render` forbid(unsafe) 维持 |

---

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-16 | 初版起草（G11.1 治理波）：以 G10_PLAN v1.0 / G9_PLAN v1.1 为范式落七节结构——§1 目标与口径（G11 = 画质修复期；法定输入 = G10.8b 锁定差距清单 11 行 + 承接锚；设修复闭环判据〔delta 收敛 measured，阈值标定程序产〕不设绝对画质通过线〔归 G15〕）；§2 八波结构（G11.1 治理 → G11.2 口径差对齐〔C1/C2/C3 + HDR-FLIP 标定，硬前置〕 → G11.3 资产与场景面修复〔R1/R2/R5/U1/U2/U3〕 → G11.4 光照与 GI 修复〔R3/R4 + M99-clipmap 世界级承接〕 → G11.5 A/B 复测〔同契约复跑 + 逐项闭环核验〕 → G11.6 P2 穷举 → G11.7a soak → G11.7b close-out），每波退出门判据草案 + 单点依赖声明；§3 P0 建议清单 13 行（M144~M156，含独立硬判据/负例 RED 臂/device-host 性质/锁定基线 delta 字面）+ go P1 1 行（M157 HDR-FLIP 独立标定）；§4 风险表 R-G11-1~11；§5 治理裁决表 9 项（供契约 §7 登记）；§6 0-byte 边界。全文零写死数字阈值（闭环阈值一律 measured 标定）；数字 CI 步骤一律 `post-interlock actual-next-free allocation`。零编号、零 registry、零 spec/src/conformance 改动。 |
| v1.1 | 2026-08-16 | **G11.3 收口 M147 判据双 phase 修订登记（只追加）**：§2 G11.5 节追加 M155 门预备注记（R1 行修复前后 SSIM delta 收敛断言后移本波——锁定基线 0.8328980787837229、阈值标定程序产禁手写、**不收敛则整波 FAIL**；M155 门 G11.5 才 materialize，本注只登记不动行字面；「锁定度量对正确修复结构性不友好」登记 G11.6 P2 候选行，反向激励旁证 0.1624318277352612 > 0.009656442299775102 入证据链）——契约 §8.3a 修订句为裁决事实源；既有八波结构/判据草案/风险表/裁决表 0-byte。`Assisted-by: Kimi-K3（G11.3 收口）` |
