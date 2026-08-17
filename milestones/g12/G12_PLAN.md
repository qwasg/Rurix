<!-- Assisted-by: Kimi-K3（G12.1 治理波起草） -->
# G12_PLAN — 路径追踪生产化期 主线分解

> **状态**：**计划起草稿（v1.0）——G12 未立项**。本文以 [G11_PLAN.md](../g11/G11_PLAN.md) v1.0 与 [G10_PLAN.md](../g10/G10_PLAN.md) v1.0 为波次结构/退出门/风险表范式起草；本文与 G12.1 治理波全部产物均不构成任何契约、验收承诺或编号 claim。G12 正式立项（用户指令 + 治理裁决 + §5 治理门）前，零 `src/`/`spec/`/`conformance/` 语义实现、零编号消费。
> **蓝本与上游**：[G11_CONTRACT.md](../g11/G11_CONTRACT.md) §8.8（G11 closed 终态，2026-08-17，flip commit `51279d45` + 回归刷新批 `5ae83aa7`）· [G11_P2_DECISIONS.md](../g11/G11_P2_DECISIONS.md) v1.0（28 行闭集；**defer-to-G12+ 19 行承接锚 = G12 法定输入**——尤其 M52 SER 锚定 G12 重评窗、c1_ue_specular_ibl 上界、g11_5b_sun_through_glass_tail、G11-N5 度量口径修订评估面）· [G10_P2_DECISIONS.md](../g10/G10_P2_DECISIONS.md) v1.0（M52 承接锚原文：「G11+（锚定 G12）高分歧 RT workload 真实集成需求 + capability rt.ser 设备面实测可用」）· [`spec/global_illumination.md`](../../spec/global_illumination.md)（M96/RXS-0357 参照器冻结面：megakernel+NEE/MIS/RR 起步范围冻结〔焦散/体积/specular 链 out〕+ 固定 seed 位级确定性协议 + pbrt-v4 容差带 + golden 门序硬约束）· [`g9_m96_pbrt_tolerance_band.json`](../g9/g9_m96_pbrt_tolerance_band.json)（M96 冻结容差带 measured 基值）· [RFC-0026](../../rfcs/0026-visual-comparison-metrics.md)（度量口径冻结面）· [RFC-0028](../../rfcs/0028-g11-gi-quality-closure.md)（G11 GI 修订先例）· [g10_ue5_harness_spike](../g10/design/g10_ue5_harness_spike.md)（UE 出图面）· [渲染器调研](../../渲染器调研/) 七份报告（2026-07-28 快照；报告 2 = GI/路径追踪技术参照）· [14_ENGINEERING_DISCIPLINE](../../14_ENGINEERING_DISCIPLINE.md) §5（证据分级：measured_local 优先，estimated 占位不得超 2 期——P-09）。
> **推进形态**：**严格波次**——G12.1 治理波 → G12.2 生产化核心波 → G12.3 降噪波 → G12.4 UE Path Tracer 对标波 → G12.5 性能面波 → G12.6 P2 穷举 → G12.7a soak → G12.7b close-out。波次内可蜂群并行，波次间串行；spec-first + RED 先行；禁止 stub/mock/host substitution 抢跑。
> **本波边界（G12.1，governance-only）**：本波仅落 PLAN / 契约 / 候选决策 / 验收映射 / RFC / CI_GATES / measured baseline / validator 治理面——零 `spec/src/conformance` 改动、零 CI 步骤 materialize（验收门只写判据草案；数字 CI 步骤一律 `post-interlock actual-next-free allocation`，禁预占）。

---

## 1. 目标与口径

**G12 = 路径追踪生产化期**。G10~G15 总体分期（G10 立项裁决口径）：G10 = 画面对标基线期（closed）→ G11 = 画质修复期（closed）→ **G12 = 路径追踪生产化期** → G13 = 超分与 DLSS 期 → G14 = 性能优化期 → G15 = 商用收口期。既有存量：M96 参照器（`src/rurix-render/src/gi/path_trace.rs` + `src/rurix-render/src/rt/ref_tracer.rs` + `src/rurix-render/src/bin/g9_m96_path_tracer.rs`）已验收（G9.4，`g9.p0.m96.path_tracer_reference` 门绿——固定 seed 位级确定性 + pbrt-v4 收敛曲线容差带 + golden 门序硬约束 D2-Q7）；G10/G11 已交付双端出图链（UE 5.8.1 MRQ 臂 F:\UE_5.8，M128/M129 门绿）+ 度量基建（EXR/FLIP/SSIM/PSNR/diff，M134~M138）+ 修复闭环 11 行全绿（G11.5b 复测清单终审锁定）。**G12 把 M96 参照器提升为生产级路径追踪器，对标 UE5 Path Tracer**（本地源码 `Engine\Source\Runtime\Renderer\Private\PathTracing.cpp` 只读可参照——F:\UE_5.8 与 E:\Kimi_Agent_Taichi Engine 优化计划\references\UnrealEngine 双树均在位实测；UE 5.8.1 可出 Path Tracing 参考帧）。

**生产化范围唯一法定来源**：M96 参照器冻结面（RXS-0357）+ G12.1 候选决策表行集（生产化核心 4 行 + 降噪 1 行 + 对标 2 行 + 性能基线 1 行 + 标定 1 行）。G12 不得无锚新立生产化项；G11 defer-to-G12+ 19 行（[G11_P2_DECISIONS](../g11/G11_P2_DECISIONS.md) §1/§3）逐行处置归 §2 G12.1 候选决策表与 G12.6 穷举，不构成新生产化项。

**G12 与 G11 的关键差异（验收口径）**：G11 设修复闭环判据（修复前后 delta 收敛 measured）；**G12 设生产化判据**——每生产化项的独立门断言「生产化落盘 + 正确性锚 0-byte（M96 参照器既有判据/确定性协议/golden 门序）+ 收敛/方差/噪声面 measured 不劣于参照器基线锚（或演进位显式登记）」。**G12 不设绝对「已达 UE5 PT 画质」通过线**——UE PT 对标波（G12.4）断言收敛曲线逐段 measured 对拍 + 噪声谱 + 能量守恒 + UE PathTracing 模块归属差距登记，不断言绝对画质达标（归 G15 商用收口期）。

**成功判据（草案；G12.1 治理波硬化为契约验收门，本波不 materialize CI 步骤）**：

1. **生产化核心**（G12.2）：MIS 完整面（光源采样 × BSDF 采样 MIS 权重全路径覆盖）/ 俄罗斯轮盘生产化（吞吐自适应 + 无偏补偿 + 最小反弹保障）/ 采样策略升级 + 低差异序列（确定性协议扩展）/ 收敛判据生产化（逐像素方差驱动自适应终止 + 收敛报告），四面各自独立闭环——正确性锚 0-byte + 收敛/方差面 measured 不劣于参照器基线锚；PT 生产化标定（标定值按 M138 同程序入 g12_budget，P-09）。
2. **降噪**（G12.3）：时域/空域降噪管线 + 与 TSR 底座联动（**temporal 底座 0-byte 不接线**，RD040-nrd 承接锚口径）+ 降噪前后噪声底 measured 回归 + 均值能量守恒（不引入系统性变暗/变亮偏置）+ NRD 类 vendor 降噪评估报告落盘（评估不接线）。
3. **UE PT 对标**（G12.4）：同场景同 spp 双端出图（UE 5.8.1 Path Tracer MRQ 臂，UE build digest == M128 登记机核）→ 收敛曲线逐段 measured 对拍 + 噪声谱对拍 + 能量守恒对拍 + UE PathTracing 模块归属差距登记表落盘。
4. **性能基线**（G12.5）：路径追踪吞吐优化基线 measured（为 G14 备料；只建基线不设通过线，G10-N11/G10-N16 承接锚字面维持）+ 优化前后正确性锚（固定 seed digest 0-byte 或演进位显式登记）。
5. **回归不降级**：生产化不得降级既有绿面（G9 34 key + G10 14 key + G11 14 key = 62 门）；G5~G11 closed 判据 0-byte；M96 门序硬约束（D2-Q7）维持。
6. **measured 纪律**：`g12_budget.json` 非空 measured_local、零 estimated（P-09）；全部闭门槛值实测标定；环境画像随证据存档。
7. **UE 零 vendoring**：UE 源码（PathTracing.cpp 等）仅外部参照只读；生产化实现零 UE 片段复制（RFC-0027 字面）。

**out-of-scope**：

| 项 | 依据 |
|---|---|
| 绝对画质通过线（「已达 UE5 PT 画质」判定） | G15 商用收口期承接；G12.4 只断言逐段 measured 对拍 + 差距登记 |
| DLSS/超分接入实施 | G13 承接；G10-N5 方向登记维持（锚定 G13） |
| 正式帧率对标与帧率通过线 | G14 承接；G10-N11/N16 帧率面锚定 G14 维持；G12.5 只建吞吐基线不设通过线 |
| GPU 管线双端 A/B 出图面 | G10-N16 锚定 G14；G12 生产化面 = M96 device megakernel（Vulkan RayQuery，已验收面演进）+ host oracle 同构兑现面 |
| 焦散/体积/specular 材质链生产化 | M96 起步范围冻结维持（RXS-0357 L1，0-byte）；g11_5b_sun_through_glass_tail / c1_ue_specular_ibl 锚定 G15 维持 |
| SER 语言层原语实施 | M52 重评窗核验未命中维持 defer（G12.1 裁决 5）；复评点 = G12.2 高分歧 RT workload 集成面 materialize 时；若 go 需独立 Full RFC |
| NRD/vendor 降噪接入实施 | RD040-nrd 承接锚：G12.3 只评估不接线；接入经 UpscaleBackend 同构契约另判（G13+ 窗） |
| 无锚新立生产化项 | 法定输入 = M96 冻结面 + 候选决策表行集 |
| UE 源码/二进制 vendoring | RFC-0027 许可边界；零 vendoring |
| 任何编号（RXS/RD/U/RX/CI step/RFC）推测性消费 | 立项时按实测 `next_free` 领取；数字 CI 步骤一律 `post-interlock actual-next-free allocation`（G11 CI_GATES §1.2 纪律继承） |
| G11 复测清单/帧库/契约参数回写 | G11 closed 终审锁定 0-byte；G12 对标契约参数 digest 独立冻结（不动 G10.5/G11.5b 锁定值） |
| 异己会话 src/ 未提交面消费 | 立项裁决 1：hzb/restir/sdf_trace/smrt/ssr/ktx2_read 等异己面严禁消费/混入（含 untracked gi/restir.rs——ReSTIR 相关面，G12 车道零消费） |

---

## 2. 波次分解

```text
G12.1 治理波（本波，governance-only：契约四件套 + 候选决策 + 验收映射 + RFC + CI_GATES + measured baseline + validator）
  → G12.2 生产化核心波（MIS 完整面 / 俄罗斯轮盘生产化 / 采样策略升级 + 低差异序列 / 收敛判据生产化 + PT 生产化标定）
  → G12.3 降噪波（时域/空域降噪 + 与 TSR 底座联动〔底座 0-byte〕+ NRD 类 vendor 降噪评估报告）
  → G12.4 UE Path Tracer 对标波（同场景同 spp 双端出图 + 收敛曲线逐段/噪声谱/能量守恒 measured 对拍 + UE PathTracing 模块归属差距登记）
  → G12.5 性能面波（PT 吞吐优化基线 measured——为 G14 备料，不设通过线）
  → G12.6 P2 穷举决策（G12 期新产生分项，defer 必有承接锚）
  → G12.7a soak → G12.7b close-out（生产化差距清单终审锁定 → G13+ 法定输入候选面）
```

**单点依赖声明**：G12.2 是全部下游波的硬前置——生产化核心面（MIS/RR/采样/收敛判据）未落地，则降噪输入（噪声帧）与对标输入（生产级收敛曲线）均不存在；G12.4 是全部生产化闭环的统一核验面（各波内门先行断言落盘 + 局部度量，G12.4 同契约双端对拍统一产出对标差距登记并逐项核验）；G12.5 依赖 G12.4 的正确性锚（优化前必须先有对标核验后的正确性基线）。

### G12.1 — 治理波（governance-only，与 G11.1/G10.1/G9.1 同构）

交付：

1. **用户立项指令留痕 + agent 治理裁决**；§5 治理裁决表项全部落定；G12 文档集不可变 ref 登记。
2. **`G12_CONTRACT.md`**：契约四要素 + front matter 状态机（`implementation_status: blocked` 起始）+ §8 只追加条款结构。
3. **`G12_CANDIDATE_DECISIONS.md`**：法定输入逐行映射——① G11 defer-to-G12+ 19 行 → G12 裁决；② 存续 open RD（RD-034/039/040/041/042/043/044）逐条映射；③ G12 新增候选（生产化核心/降噪/对标/性能基线/标定）。缺行不得开工 G12.2。
4. **`G12_ACCEPTANCE_MAP.md`**：全部 P0（及 go 的 P1）的 `M## → symbolic key → 稳定脚本 → evidence schema → 判据`；缺行阻断 G12.2。
5. **Full RFC**（实际编号按立项时 `registry/number_ledger.json` 实测 `next_free` 领取，禁止推测号）：路径追踪生产化触 `spec/global_illumination.md` 冻结面（RXS-0357 起步范围/确定性协议/门序面演进 + 采样语义冻结面）——判档 **Full RFC**（判档争议向上取严）；SER（M52）若 G12.2 复评 go 需语言层原语 → 独立 Full RFC 评估（承接锚：capability rt.ser 设备面实测可用 + 真实集成需求——先核验后裁决，未命中维持 defer）。
6. **RTX 4070 Ti measured baseline → `g12_budget.json` 非空**（P-09：沿 G11.1 baseline 锚复测重登记 + PT 参照器收敛曲线基线锚——M96 冻结容差带 measured 曲线值转录为生产化回归锚，零 estimated）。
7. **`CI_GATES`** + G12 专属 validator（implementation interlock 诚实 BLOCKED→READY 两态 + acceptance map 三向比对 + wave_exit_lib + p2_decisions 骨架；数字 CI 步骤一律 `post-interlock actual-next-free allocation`）。

退出门（判据草案，防假绿）：§5 治理裁决表项全落定并留痕；四件套齐备且互锁 validator 诚实输出 BLOCKED→READY；候选决策表与验收映射无缺行；零数字 CI 步骤 materialize；registry/spec/src/conformance 0-byte（登记/翻转/history 追加归立项治理动作除外）。

### G12.2 — 生产化核心波

| 面 | 内容 | 矩阵 |
|---|---|---|
| MIS 完整面 | 光源采样（NEE）× BSDF 采样 MIS 权重全路径覆盖生产化（参照器起步 MIS 面演进）——多光源 MIS + 权重闭式 + 能量守恒（白炉/逐级能量增量单调不增口径继承 RXS-0395）+ 方差削减 measured（同 spp 收敛曲线不劣于参照器基线锚） | M158 |
| 俄罗斯轮盘生产化 | 吞吐自适应 RR（路径吞吐权重驱动终止概率）+ 无偏补偿（补偿因子语义）+ 最小反弹保障（低深度不早杀）+ 跳 RR 偏移 RED 臂（RXS-0357 三臂 RED 面继承） | M159 |
| 采样策略升级 + 低差异序列 | 逐像素独立 PCG 流 → 分层/低差异序列（stratified/Sobol 类）生产化——确定性协议扩展（低差异序列索引确定性 + 固定 seed 位级一致维持 + RNG 流布局 provenance）+ 收敛曲线 measured 不劣于独立流锚 | M160 |
| 收敛判据生产化 | 逐像素方差驱动自适应 spp 终止 + 收敛报告（逐像素 spp 分布/方差/未收敛像素计数非空）+ 收敛误判率标定（标定程序产禁手写）+ 固定全 spp golden 对拍不偏离冻结带 | M161 |
| PT 生产化标定 | 生产化闭门槛值标定（方差削减比/收敛误判率/噪声底标定值）按 M138 同程序（p100×k measured）入 `g12_budget.json` provenance 齐备（P-09） | M166（P1） |

退出门（判据草案）：`g12.p0.m158` / `g12.p0.m159` / `g12.p0.m160` / `g12.p0.m161` 门绿——四面独立闭环（生产化落盘 + 正确性锚 0-byte〔M96 既有判据/确定性协议/golden 门序〕+ 收敛/方差面 measured 不劣于参照器基线锚〔或演进位显式登记即 RED 评审面〕）；`g12.p1.m166` 门绿——标定程序可复跑 + 标定值入 budget provenance 齐备（禁手写阈值冒充标定即 RED；estimated 冒充 measured 即 RED）。

### G12.3 — 降噪波

| 面 | 内容 | 矩阵 |
|---|---|---|
| 时域/空域降噪管线 | A-trous 类空域降噪 + 时域累积与 TSR 底座联动（**temporal 底座 0-byte 不接线**——只消费既有 TAA/TSR 历史接口面，RD040-nrd 承接锚口径）+ 降噪前后噪声底 measured 回归（噪声谱高频能量下降 measured，标定阈）+ 均值能量守恒（帧均值能量差 measured 容差，不引入系统性变暗/变亮偏置）+ 历史验证/去鬼影面 | M162 |
| NRD 类 vendor 降噪评估 | NRD 类 vendor 降噪评估报告落盘（RD040-nrd 承接锚口径：UpscaleBackend 同构输入契约〔MV/深度/法线〕接入面评估 + 许可/ABI 取证；**评估不接线**，接入另判 G13+ 窗） | M162（同门登记面） |

退出门（判据草案）：`g12.p0.m162` 门绿——降噪管线落盘 + 噪声底回归 measured + 均值能量守恒容差内 + temporal 底座 0-byte 断言 + NRD 评估报告落盘（评估结论 go/no-go 显式，不接线不冒充接入）+ golden 对拍面不降级。

### G12.4 — UE Path Tracer 对标波

| 面 | 内容 | 矩阵 |
|---|---|---|
| 同场景同 spp 双端出图 | Rurix 生产 PT 臂 vs UE 5.8.1 Path Tracer MRQ 臂（F:\UE_5.8；UE build digest == M128 登记 ue_build_id 机核继承）——场景契约独立冻结（digest 机核，不动 G10.5/G11.5b 锁定值）+ spp 序列同字面 + 曝光/位深口径沿 G11.2 对齐口径（残余口径差显式登记） | M163 |
| 收敛曲线/噪声谱/能量守恒对拍 | 收敛曲线逐段 measured 对拍（spp 序列逐段 rel-MAE 曲线差 measured，容差标定程序产）+ 噪声谱对拍（高频能量谱差 measured）+ 能量守恒对拍（帧均值能量差 measured）+ **UE PathTracing 模块归属差距登记表落盘**（PathTracing.cpp 归属行集，沿 RXS-0391 归属枚举口径） | M163 |
| 生产化回归门 | 既有 62 门（G9 34 + G10 14 + G11 14）最新 evidence 全绿只读汇总 + 生产化触改面既有门重跑回归零降级（M96 golden 门序面真跑抽检） | M164 |

退出门（判据草案）：`g12.p0.m163` / `g12.p0.m164` 门绿——对标报告 + 差距登记表落盘（逐段对拍容差内或显式差距登记即 RED 评审面，不静默混入）+ 契约 digest 不等仍出报告即 RED（门序硬约束继承）+ 单端缺帧聚合不得 PASS + **不设绝对通过线**（「已达 UE5 PT 画质」叙述 G12 期内一律不成立）+ 62 门零降级。

### G12.5 — 性能面波

| 面 | 内容 | 矩阵 |
|---|---|---|
| PT 吞吐优化基线 | 路径追踪吞吐基线 measured（rays/sec 与帧时 at 固定 spp × 场景集，BENCH_PROTOCOL §3 同族协议：warmup 至稳态 + 50×3 timed + IQR + trimmed mean 0.2）+ 基线入 `g12_budget.json`（measured_local，**不设通过线**——G14 备料，G10-N11/N16 承接锚字面维持）+ 优化前后正确性锚（固定 seed digest 0-byte 或演进位显式登记） | M165 |

退出门（判据草案）：`g12.p0.m165` 门绿——吞吐基线入 budget provenance 齐备 + 正确性锚断言（digest 漂移未登记即 RED）+ 不设通过线登记（以基线冒充帧率对标即 RED——正式帧率对标锚定 G14 字面）。

### G12.6 — P2 穷举决策（软门，必须穷尽）

对 G12 期内新产生的 P2/留档/未触发分项逐条 go/no-go/defer-to-G13+（不得遗漏；defer 必有承接锚，机核同构 `ci/g11_p2_decisions_check.py`）。候选行集 = G12.1 决策表校准后冻结 + G12.2~G12.5 期内新增分项（含 M52 G12.2 复评结论行、M100-high G12.4 触发评估行、G10-N17/G11-N5 触发评估行）。软门失败/未触发 → 诚实登记不阻塞 G12.7a；close-out 审计要求本表无空行。

### G12.7a — Stabilization / soak（close 前必经）

- 全 P0 硬门回归 + 已 go 的 P1 回归；G5~G11 既有判据 0-byte。
- 生产化链路全量复跑 soak：PT 出图/降噪/对标装配全链路连续复跑（量级沿 G11.7a 继承〔≥1800s〕或 measured 证明更短足够，具体阈值 G12.1 裁决 measured 标定）。
- `budget_eval --strict` 非空、零 estimated/skip。
- 条件实现刚绿**不得**当日进 7b；**G8.8b/G9.8b/G10.8b/G11.8b 同日放行先例是否继承属治理裁决（§5 表项 7）**。

### G12.7b — Close-out

- 验收映射终审；RD 分项最终状态与 G12.1/G12.6 决策表逐字一致。
- **生产化差距清单终审锁定 → G13+ 法定输入候选面**（UE PT 对标残余差距/未闭环行如实登记，不冒充全闭环）。
- 契约 §8 只追加 + status flip。

### 2.9 P0 → 硬门覆盖（判据草案；G12.1 固化为 ACCEPTANCE_MAP 时方可 materialize CI 步骤）

任一 P0 无独立硬门 → **禁止** G12.7b status flip。数字 CI 步骤一律 `post-interlock actual-next-free allocation`（G11 CI_GATES v1.9 已至步骤 216；G12 编号自互锁后实测 `next_free` 顺位领取，禁预占）。

---

## 3. P0 建议清单（8 行，M158 起顺延 G11 矩阵 M144~M157；G12.1 决策表重裁后硬化）

| P0 | 名称 | 独立硬判据（草案） | 负例 RED 臂要求 | device/host 性质 | 最晚波次 |
|---|---|---|---|---|---|
| M158 | MIS 完整面生产化门 | 光源采样 × BSDF 采样 MIS 权重全路径覆盖 + 能量守恒（白炉 + 逐级能量增量单调不增）+ 同 spp 收敛曲线不劣于参照器基线锚（g12_budget pt.ref_curve 锚，容差标定程序产）+ 固定 seed 位级确定性协议继承 + M96 既有判据 0-byte | 权重缺失冒充 MIS 即 RED；能量偏置注入即 RED；收敛劣化冒充升级即 RED；确定性协议漂移即 RED | host+device | G12.2 |
| M159 | 俄罗斯轮盘生产化门 | 吞吐自适应 RR + 无偏补偿（补偿因子闭式）+ 最小反弹保障 + RR 终止率/补偿计数非空 + 收敛曲线不劣于基线锚 + 跳 RR 偏移 RED 臂（RXS-0357 三臂面继承） | 早杀偏置注入即 RED；补偿缺失冒充无偏即 RED；跳 RR 未检出即 RED | host+device | G12.2 |
| M160 | 采样策略升级 + 低差异序列门 | 分层/低差异序列生产化 + 确定性协议扩展（序列索引确定性 + 固定 seed 位级一致 + RNG 流布局 provenance）+ 收敛曲线 measured 不劣于独立 PCG 流锚 + 序列篡改 RED 臂 | 序列非确定冒充低差异即 RED；位级一致破坏未登记即 RED；收敛劣化冒充升级即 RED | host+device | G12.2 |
| M161 | 收敛判据生产化门 | 逐像素方差驱动自适应 spp 终止 + 收敛报告（spp 分布/方差/未收敛像素计数非空）+ 收敛误判率 ≤ 标定阈（标定程序产）+ 固定全 spp golden 对拍不偏离冻结带（measured×2.0 带继承） | 早停冒充收敛即 RED；未收敛像素缺报即 RED；golden 偏离冻结带即 RED | host+device | G12.2 |
| M162 | 降噪管线 + TSR 联动门 | 时域/空域降噪管线落地 + 噪声谱高频能量下降 measured（标定阈）+ 帧均值能量守恒容差内（不引入系统性偏置）+ temporal 底座 0-byte 断言 + NRD 类 vendor 降噪评估报告落盘（评估不接线）+ golden 对拍面不降级 | 降噪引入系统性变暗/变亮即 RED；temporal 底座接线即 RED；评估冒充接入即 RED；噪声底未降冒充降噪即 RED | host+device | G12.3 |
| M163 | UE Path Tracer 对标门 | 同场景同 spp 双端出图（UE build digest == M128 登记机核；契约 digest 独立冻结，不等仍出报告即 RED）+ 收敛曲线逐段 measured 对拍（容差标定程序产）+ 噪声谱对拍 + 能量守恒对拍 + UE PathTracing 模块归属差距登记表落盘；不设绝对通过线 | 契约 digest 不等仍出报告即 RED；逐段对拍超容差静默即 RED；差距项静默混入即 RED；单端缺帧聚合 PASS 即 RED | host+device | G12.4 |
| M164 | 生产化回归门 | 既有 62 门（G9 34 + G10 14 + G11 14）最新 evidence 全绿只读汇总 + 生产化触改面既有门重跑回归零降级（M96 golden 门序面真跑抽检） | 既有门降级即 RED；汇总遮蔽子断言 FAIL/SKIP/DEV_ENV_DEGRADE 即 RED | host 纯 host | G12.4 |
| M165 | PT 吞吐优化基线门 | 吞吐基线 measured（rays/sec + 帧时，50×3 trimmed mean 协议）入 g12_budget provenance 齐备 + 不设通过线登记 + 优化前后正确性锚（固定 seed digest 0-byte 或演进位显式登记） | 基线冒充帧率对标即 RED；digest 漂移未登记即 RED；estimated 冒充 measured 即 RED | host+device | G12.5 |

P1 建议（go 1 行）：**M166 PT 生产化标定门**（G12.2；生产化闭门槛值标定——方差削减比/收敛误判率/噪声底标定值按 M138 同程序 p100×k measured 入 `g12_budget.json` provenance 齐备；手写阈值冒充标定即 RED；estimated 冒充 measured 即 RED；标定程序不可复跑即 RED；host 纯 host）。

---

## 4. 风险与止损

| ID | 风险 | 预警 | 止损 |
|---|---|---|---|
| R-G12-1 | 生产化破坏固定 seed 确定性协议（低差异序列/自适应终止/降噪引入位级漂移） | digest 双跑不一致 / golden 漂移 | 确定性协议扩展经 RFC-0029 显式条款；digest 0-byte 断言内嵌各门；漂移未登记即 RED |
| R-G12-2 | MIS/RR 语义错误引入能量偏置（权重/补偿因子错误 → 白炉不守恒、系统性变暗） | 白炉 measured 偏移 / 能量增量非单调 | 能量守恒判据内联（白炉 + 逐级能量增量单调不增，RXS-0395 口径继承）；偏置注入 RED 臂独立有效 |
| R-G12-3 | 降噪引入系统性偏置/鬼影（时域累积历史污染、空域过模糊） | 帧均值能量漂移 / 历史验证面 FAIL | 均值能量守恒容差 measured + 历史验证/去鬼影面 + 降噪前后 golden 对拍不降级 |
| R-G12-4 | 降噪动 temporal 底座（TSR 底座被接线改写，RD040-nrd 锚被静默翻转） | temporal 底座文件 diff 非空 | temporal 底座 0-byte 断言（M162 门内嵌）；NRD 评估不接线登记；接入另判 G13+ 窗 |
| R-G12-5 | UE PT 口径差未对齐即对拍（spp/采样/曝光/位深口径差淹没收敛曲线对拍——R-G10-6/R-G11-1 教训同族） | 对拍 delta 异常大且不可归因 | 口径对齐先行（spp 序列同字面 + 契约 digest 机核 + 曝光/位深沿 G11.2 对齐口径）；残余口径差显式登记；未对齐口径消费对拍 delta 即 RED |
| R-G12-6 | UE 出图环境漂移（F:\UE_5.8 版本/许可变化；Path Tracer 设置面漂移） | UE build digest ≠ M128 登记值 | UE build digest == M128 登记 ue_build_id 机核（G10.5b/G11.5 门序面继承）；MRQ 主路臂维持（G10-N9 死路不复活作证据面） |
| R-G12-7 | 收敛判据过松冒充收敛（自适应 spp 早停造假收敛，生产化名义降级画质） | 未收敛像素计数异常 / golden 偏离 | 固定全 spp golden 对拍不偏离冻结带（measured×2.0 带继承）+ 收敛误判率标定 + 未收敛像素计数非空；不以自适应帧冒充全 spp 参照 |
| R-G12-8 | 性能优化降级正确性（吞吐提升改写语义/RNG/累加序） | 优化后 digest 漂移 / 62 门回归 FAIL | 正确性锚（digest 0-byte 或演进位显式登记）+ M164 回归门独立 P0；G5~G11 closed 判据 0-byte |
| R-G12-9 | 异己并发工作树面混入（立项时工作树带异己会话 src/ 未提交面，含 untracked gi/restir.rs——ReSTIR 相关面） | G12 车道 commit 混入异己面 | 带未提交项立项登记（§5 表项 1）；G12 commit 只含 G12 车道文件；异己面零消费零混入（G10.8b §8.10/G11 先例同模） |
| R-G12-10 | 参照器既有判据被降级（RXS-0357 起步范围/门序面改写冒充生产化） | RXS-0357 字面 diff / 门序阻断失效 | M96 既有判据 0-byte 断言；spec-first 显式修订行（新条款承载演进，既有条款字面不动）；golden 门序机器阻断（D2-Q7）维持 |
| R-G12-11 | SER 重评抢跑（未核验 capability/真实需求即接线语言层原语） | rt.ser 面出现未经复评的实现 | G12.1 核验登记（需求未至 + 设备面未实测 → maintain-defer）；复评点 = G12.2 集成面 materialize 时；若 go 需独立 Full RFC（契约 §7 裁决 4/5） |

---

## 5. 治理裁决表项（供契约 §7 登记；本计划不定案，裁决权归用户/立项治理）

| # | 事项 | 候选口径 | 不定案原因 |
|---|---|---|---|
| 1 | G12 立项时机与工作树处置 | 带未提交项立项（G12.0 不可变 ref = 立项时实测 HEAD；异己会话 src/ 未提交面〔rurix-asset/rurix-render geometry/gi/shadow/ssr/ktx2_read/hzb/restir/sdf_trace/smrt 声明面〕保持不混入 G12 车道，严禁消费）or 先处置再立项 | 处置方式影响 G12 文档集不可变 ref 的基线内容；异己面归属权不在 G12 车道 |
| 2 | 生产化范围法定来源 | M96 参照器冻结面（RXS-0357）+ 候选决策表行集（生产化核心 4 + 降噪 1 + 对标 2 + 性能基线 1 + 标定 1）；不得无锚新立生产化项 | 涉及 G12/G13/G14/G15 边界严肃性，须立项显式裁决 |
| 3 | UE PT 对标判据形态 | 收敛曲线逐段 measured 对拍 + 噪声谱 + 能量守恒 + UE PathTracing 模块归属差距登记；不设绝对「已达 UE5 PT 画质」通过线（归 G15） | 对标判据形态须立项显式裁决 |
| 4 | RFC 判档 | 生产化核 + 降噪 + 对标口径 = Full RFC（触 spec/global_illumination.md RXS-0357 冻结面 + 采样语义冻结面，向上取严）；SER（M52）若 G12.2 复评 go → 独立 Full RFC 评估（语言层原语面，RFC-0023 冻结面衔接） | 判档争议向上取严；升级触发条件须立项登记 |
| 5 | M52 SER 重评窗核验 | G12.1 治理波先核验：真实集成需求未至（治理波零实现）+ capability rt.ser 设备面未实测（树内零探针）→ maintain-defer；复评点 = G12.2 生产化核心波 materialize 高分歧 RT workload 集成面时；承接锚字面 0-byte 维持 | RD-040 history 重评窗兑现须立项留痕 |
| 6 | 降噪与 TSR 底座联动口径 | temporal 底座 0-byte 不接线（RD040-nrd 承接锚口径：NRD/vendor 降噪经 UpscaleBackend 同构输入契约接入，接入时不改 temporal 底座；G12.3 只评估不接线） | 底座边界严肃性须立项确认 |
| 7 | G8.8b/G9.8b/G10.8b/G11.8b 同日放行先例继承 | 继承：7a full-run 先行完成后允许同日进 7b（先例链）or 不继承 | 涉及 soak 严肃性口径，须立项显式裁决 |
| 8 | measured/编号/资产纪律确认 | g12_budget 非空 measured_local 零 estimated（G11 baseline 锚复测重登记 + PT 参照器收敛曲线基线锚）；数字 CI 步骤 `post-interlock actual-next-free allocation`；压测资产二进制不入 git（外部缓存 K: 盘 + 仓库内元数据登记，沿 G10/G11 裁决） | 编号纪律须立项重申 |

---

## 6. 与既有面的边界（0-byte 纪律）

| 面 | 约束 |
|---|---|
| G11 车道 | G11 四件套/决策表/evidence schema/budget/复测清单 0-byte（G11 closed）；复测清单只消费不回写；G11_P2_DECISIONS 28 行裁决字面 0-byte 不回写（defer-to-G12+ 19 行承接锚转引消费） |
| G5~G10 冻结面 | G5~G10 closed 契约与判据 0-byte；触冻结面必须显式 RFC 修订行（G11 纪律继承） |
| M96 参照器面 | RXS-0357 起步范围冻结（焦散/体积/specular 链 out）/ 固定 seed 确定性协议 / pbrt-v4 容差带 / golden 门序（D2-Q7）0-byte；生产化演进经 RFC 显式修订行 + 新条款承载；`g9_m96_pbrt_tolerance_band.json` 冻结带只消费不回写 |
| 00–14 | 只勘误，独立提交 |
| spec/conformance | G12.1 期 0-byte；spec-first + RED 先行自 G12.2 实现门开放后起，spec 条款 PR 先于实现 PR |
| 注册表 | G12.1 期仅立项治理动作（deferred history 只追加 / number_ledger reserved_in_flight 登记与命名空间校准）；既有条目四字段 0-byte，禁静默改判 |
| 编号 | G12.1 期零数字 claim（RFC 号按立项实测 `next_free` 领取为例外面——Full RFC 治理波 materialize 先例沿 G9.1/G10.1/G11.1）；数字 CI 步骤一律 `post-interlock actual-next-free allocation`；一切编号以领取时实测 `next_free` 为准 |
| UE 源码 | `F:\UE_5.8` 与 `E:\Kimi_Agent_Taichi Engine 优化计划\references\UnrealEngine` 只读外部参照（PathTracing.cpp 只读可参照）；零 vendoring、零片段复制（RFC-0027 字面）；压测资产二进制不入 git（外部缓存 K: 盘 + 仓库内元数据登记） |
| temporal 底座 | TAA/TSR 时域底座 0-byte 不接线（RD040-nrd 承接锚口径）；G12.3 降噪只消费既有历史接口面 |
| 异己并发面 | 异己会话 src/ 未提交面（hzb/restir/sdf_trace/smrt/ssr/ktx2_read 等）严禁消费/混入；G12 车道 commit 只含 G12 车道文件（G10.8b §8.10/G11 先例同模） |
| 调研引用 | [渲染器调研](../../渲染器调研/) 七份为 2026-07-28 快照；G12.1 复核关键引用时效（沿 G9 R-G9-3/G10/G11 模式） |
| unsafe | U 段按立项 next_free；`rurix-render` forbid(unsafe) 维持 |

---

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-17 | 初版起草（G12.1 治理波）：以 G11_PLAN v1.0 / G10_PLAN v1.0 为范式落七节结构——§1 目标与口径（G12 = 路径追踪生产化期；法定输入 = M96 参照器冻结面 + 候选决策表行集；设生产化判据〔正确性锚 0-byte + measured 不劣于基线锚〕不设绝对 UE PT 画质通过线〔归 G15〕）；§2 八波结构（G12.1 治理 → G12.2 生产化核心〔MIS 完整面/RR/采样策略升级+低差异/收敛判据生产化 + PT 标定，硬前置〕 → G12.3 降噪〔时域/空域 + TSR 底座 0-byte + NRD 评估〕 → G12.4 UE PT 对标〔同场景同 spp 双端出图 + 逐段/噪声谱/能量守恒 measured 对拍 + 模块归属差距登记〕 → G12.5 性能基线〔吞吐基线为 G14 备料不设通过线〕 → G12.6 P2 穷举 → G12.7a soak → G12.7b close-out），每波退出门判据草案 + 单点依赖声明；§3 P0 建议清单 8 行（M158~M165，含独立硬判据/负例 RED 臂/device-host 性质）+ go P1 1 行（M166 PT 生产化标定）；§4 风险表 R-G12-1~11；§5 治理裁决表 8 项（供契约 §7 登记）；§6 0-byte 边界（含 M96 参照器面与 temporal 底座专项）。全文零写死数字阈值（闭门槛值一律 measured 标定）；数字 CI 步骤一律 `post-interlock actual-next-free allocation`。零编号、零 registry、零 spec/src/conformance 改动。 |
