<!-- Assisted-by: Kimi-K3（G12.1 治理波 RFC 起草） -->
# RFC-0029 — G12 路径追踪生产化语义（G12 伞形：MIS 完整面 / 俄罗斯轮盘生产化 / 采样策略升级与低差异序列确定性协议扩展 / 收敛判据生产化 / 降噪管线与 TSR 底座联动 / UE Path Tracer 对标口径 / spec/global_illumination.md RXS-0357 参照器面演进显式修订行）

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0029（4 位制，编号永不复用，10 §9.5；编号按 2026-08-17 实测 `registry/number_ledger.json` namespaces.RFC `next_free=29` 领取，非推测号；`reserved_in_flight[G12]` 登记由 G12.1 治理波落） |
| 标题 | G12 路径追踪生产化语义（G12 伞形单章） |
| 档位 | **Full RFC**（路径追踪运行时语义面——触 `spec/global_illumination.md` 冻结面 RXS-0357 参照器起步范围/固定 seed 确定性协议/门序面演进 + 采样语义冻结面，G5~G11 冻结面改动必须 RFC 显式修订行；MR 体例不承载新语义面 + 冻结面修订，判档争议向上取严，10 §3 / AGENTS 硬规则 5） |
| 状态 | **Agent Approved**——D-409 对抗性评审完成（findings 全部 disposition，§9.1）；主会话已核对契约/MAP/CI_GATES 三面一致（2026-08-17），翻 Agent Approved |
| 承接里程碑 | G12（G12.2 生产化核心波 M158~M161/M166；G12.3 降噪波 M162；G12.4 UE PT 对标波 M163；验收门 G-G12-4 / G-G12-5 / G-G12-6） |
| 关联条款 | 拟落 spec **RXS-####~**（条款号一律 **post-interlock actual-next-free allocation**，不预写推测号；候选落点见 §5：`spec/global_illumination.md` 修订行 + 追加条款） |
| 依据决策 | D-406 v2.0 · D-409 · P-09 · P-13 · 10 §7/§9.5 · G12 立项十项裁决（[G12_CONTRACT](../milestones/g12/G12_CONTRACT.md) §7：裁决 3 对标判据形态、裁决 4 RFC 判档、裁决 5 M52 重评窗核验、裁决 6 M96 既有判据 0-byte）· G12_CONTRACT §4.2（M158~M163 硬判据字面）· [G12_PLAN](../milestones/g12/G12_PLAN.md) §2 G12.2/G12.3/G12.4 · R-G12-1/R-G12-2/R-G12-5/R-G12-7 · [G12_ACCEPTANCE_MAP](../milestones/g12/G12_ACCEPTANCE_MAP.md) §1 · RXS-0357~0362（spec/global_illumination.md 冻结面）· RXS-0395（多反弹能量守恒口径）· RXS-0384~0393（spec/visual_comparison.md 度量口径冻结面）· [`g9_m96_pbrt_tolerance_band.json`](../milestones/g9/g9_m96_pbrt_tolerance_band.json)（M96 冻结容差带 measured 基值）· [`registry/deferred.json`](../registry/deferred.json) RD-040（nrd 承接锚字面）· [调研报告 2](../渲染器调研/调研报告2-GI与Lumen类全局光照.md)（GI/路径追踪技术参照，2026-07-28 快照） |
| Provenance | `Assisted-by: Kimi-K3（G12.1 治理波 RFC 起草）` |
| Agent 批准 | **已批准**（2026-08-17，主会话核对契约/MAP/CI_GATES 三面一致后翻 Agent Approved——三面一致性核验：契约 §4.2 八行 P0 key/判据 ↔ MAP §1 逐字一致 ↔ CI_GATES §4 同构；D-409 第 1 轮对抗性评审 10 findings 全部 disposition 落实 v0.2 修法批） |
| 对抗性评审 | **已完成**（D-409 第 1 轮，2026-08-17，评审轮次与起草轮次隔离；findings 全部 disposition 落实（v0.2 修法批），disposition 逐条见 §9.1；provenance 偏差如实登记：评审者与起草者同模型同会话族、独立性 = 评审轮次隔离，非跨工具/跨会话——偏差大于 RFC-0024「同工具族独立实例」先例，与 RFC-0025 单实例先例同族，效力自限声明见 §9.1 并留 G12.7b 终审复核锚；评审全文见 [rfc0029_adversarial_review.md](../milestones/g12/design/rfc0029_adversarial_review.md)） |

---

## 1. 摘要

本 RFC 冻结 G12「路径追踪生产化期」把 M96 参照器提升为生产级路径追踪器的语义面——七个子面一份冻结：

1. **MIS 完整面语义**（G12.2 M158 消费面）：多重重要性采样自参照器起步面（单光源 NEE/MIS）演进为全路径覆盖——光源采样（NEE）× BSDF 采样双策略逐顶点 MIS 权重（balance heuristic）；多光源 MIS；能量守恒语义（白炉守恒 + 逐级能量增量单调不增，RXS-0395 口径继承）。
2. **俄罗斯轮盘生产化语义**（G12.2 M159 消费面）：RR 自参照器起步面（固定概率/固定深度）演进为吞吐自适应——终止概率由路径吞吐权重驱动、补偿因子闭式无偏、最小反弹保障（低深度不早杀）、终止率/补偿计数进 evidence。
3. **采样策略升级与低差异序列确定性协议扩展**（G12.2 M160 消费面）：采样自逐像素独立 PCG 流演进为分层/低差异序列（stratified/Sobol 类）；确定性协议**加性扩展**——低差异序列索引推导确定性（像素索引 × 采样索引 × 维度寻址）+ 固定 seed 位级一致维持 + RNG 流布局 provenance 进 evidence；RXS-0357 L2 既有字面 0-byte。
4. **收敛判据生产化语义**（G12.2 M161 消费面）：逐像素方差驱动自适应 spp 终止 + 收敛报告（逐像素 spp 分布/方差/未收敛像素计数非空）+ 收敛误判率标定（标定程序产禁手写）+ 固定全 spp golden 对拍不偏离冻结带——自适应帧不得冒充全 spp 参照。
5. **降噪管线与 TSR 底座联动语义面**（G12.3 M162 消费面）：时域累积消费既有 TAA/TSR 历史接口面（**temporal 底座 0-byte 不接线**，RD040-nrd 承接锚口径）+ 空域 A-trous 类滤波 + 噪声谱高频能量下降 measured + 帧均值能量守恒（不引入系统性变暗/变亮偏置）+ NRD 类 vendor 降噪评估报告（评估不接线，接入另判 G13+ 窗）。
6. **UE Path Tracer 对标口径面**（G12.4 M163 消费面）：同场景同 spp 双端出图（UE 5.8.1 Path Tracer MRQ 臂；契约参数独立冻结 digest 机核，不动 G10.5/G11.5b 锁定值）+ 收敛曲线逐段 measured 对拍 + 噪声谱对拍 + 能量守恒对拍 + UE PathTracing 模块归属差距登记表；**不设绝对通过线**；残余口径差显式登记（未对齐口径消费对拍 delta 即 RED）。
7. **spec/global_illumination.md 显式修订行**：RXS-0357 参照器面演进——起步范围冻结**维持**（焦散/体积/specular 链 out 0-byte），生产化语义经修订行 + 新条款承载（post-interlock actual-next-free allocation 领新 RXS 条款，RXS-0357 既有字面 0-byte）。

**生产化判据语义**（横切七面）：每生产化项闭环 = 生产化落盘 + 正确性锚 0-byte（M96 既有判据/固定 seed 确定性协议/golden 门序 D2-Q7）+ 收敛/方差/噪声面 measured 不劣于参照器基线锚（容差由标定程序 measured 产出禁手写；或演进位显式登记即 RED 评审面）。**本 RFC 不冻结任何绝对 UE PT 画质通过线**——「已达 UE5 PT 画质」判定归 G15 商用收口期。

```text
M96 参照器冻结面（RXS-0357：megakernel + NEE/MIS/RR 起步 + 确定性协议 + pbrt 容差带）
   │  0-byte 只消费不回写；golden 门序 D2-Q7 维持
   ▼
G12.2 生产化核心：MIS 完整面 → RR 生产化 → 采样升级+低差异 → 收敛判据生产化
   │  正确性锚 0-byte + 收敛/方差面 measured 不劣于基线锚（g9_m96 冻结带转录）
   ▼
G12.3 降噪：时域/空域降噪（temporal 底座 0-byte）+ NRD 评估（不接线）
   ▼
G12.4 UE PT 对标：同场景同 spp 双端对拍（逐段/噪声谱/能量守恒 measured）
   │  + UE PathTracing 模块归属差距登记 → G13+ 法定输入候选面
   ▼
G12.5 吞吐基线（G14 备料，不设通过线）
```

本 RFC 是 **G12.1 governance-only** 交付物。即使随后 Agent Approved，也只表示语义评审通过，**不会解锁任何 `src/`、`spec/`、`conformance/` 实现**；G12.2 互锁（G-G12-3）是独立硬门。§4 全部 schema/参数/算法为**拟议语义（Draft）**，批准前不构成契约。

## 2. 动机、范围与治理门

### 2.1 为什么需要 Full RFC

G12 把 M96 参照器（G9.4 验收：固定 seed 位级确定性 + pbrt-v4 收敛曲线容差带 + golden 门序 D2-Q7）提升为生产级路径追踪器。该提升面触及 `spec/global_illumination.md` 冻结面：RXS-0357 的起步范围（megakernel + NEE/MIS/RR 起步）、固定 seed 确定性协议（累加序/RNG 流冻结）与 golden 门序面是 G9.4 经 RFC-0022 冻结的语义；MIS 完整面、吞吐自适应 RR、低差异序列采样、自适应收敛判据、降噪管线均是对该冻结面的**演进**（不是推翻——起步范围冻结维持，语义面加性扩展），G5~G11 冻结面改动必须 RFC 显式修订行（G12_CONTRACT guardrails 字面）；采样确定性协议扩展、降噪与 TSR 底座联动、UE PT 对标口径均属运行时渲染语义面，MR（Mini-RFC）体例不承载新语义面 + 冻结面修订（RFC-0025/RFC-0028 判档先例），判档向上取严为 **Full RFC**。

法定输入字面（G12.1 立项已定案）：

- G12 立项裁决 3：「UE PT 对标判据形态 = 收敛曲线逐段 measured 对拍 + 噪声谱 + 能量守恒 + UE PathTracing 模块归属差距登记；G12 不设绝对通过线」——本 RFC §4.6 即该裁决的语义兑现面；
- G12 立项裁决 5：「M52 SER 重评窗核验 = maintain-defer（双条件未命中）；复评点 = G12.2 集成面 materialize 时」——**SER 不在本 RFC 范围**；若 G12.2 复评 go，语言层原语面独立 Full RFC 评估（RFC-0023 冻结面衔接，本 RFC §6.4 边界声明）；
- G12 立项裁决 6：「M96 参照器既有判据 0-byte——起步范围冻结（焦散/体积/specular 链 out）/ 确定性协议 / pbrt-v4 容差带 / golden 门序 0-byte；生产化演进经 RFC-0029 显式修订行 + 新条款承载」——本 RFC §4.7 即该裁决的兑现面；
- G12_CONTRACT §4.2 六行 P0 硬判据（M158~M163）是本 RFC 语义面的下游机器消费者，判据字面不在本 RFC 重定。

### 2.2 双门互锁：RFC 批准不等于实现开工

| 门 | 允许动作 | 禁止动作 |
|---|---|---|
| G12.1 governance-only（本波） | 起草/评审/批准 RFC；冻结语义面与 §5 spec 映射计划；编号 claim 登记 | 不改 `src/`、`spec/`、`conformance/`；不 materialize 数字 CI 步骤；不预建空 schema 壳/空脚本占位；不领取 RXS/RD/U/RX 共享在途号 |
| G12.2+ implementation gate | G-G12-3 机器事实（validator READY + 用户开工指令 + actual `next_free` 重校）齐备后，spec-first 落条款与 RED | 互锁任一红时不得以 RFC Approved 或立项裁决替代机器事实 |

### 2.3 范围（in scope）

- M158 MIS 完整面语义（§4.1）；M159 RR 生产化语义（§4.2）；M160 采样策略升级与低差异序列确定性协议扩展（§4.3）；M161 收敛判据生产化语义（§4.4）。
- M162 降噪管线与 TSR 底座联动语义面 + NRD 评估口径（§4.5）。
- M163 UE PT 对标口径面（§4.6）。
- RXS-0357 参照器面演进显式修订行计划（§4.7 + §5 映射表）。

### 2.4 非范围（out of scope）

| 项 | 依据 |
|---|---|
| 焦散/体积/specular 材质链生产化 | M96 起步范围冻结维持（RXS-0357 L1 0-byte，G12 立项裁决 6）；G11-N8/G11-N9/G12-N10 锚定 G15 |
| SER 语言层原语 | M52 重评窗核验未命中维持 defer（G12.1 裁决 5）；若 G12.2 复评 go → 独立 Full RFC（本 RFC §6.4 边界声明，RFC-0023 冻结面衔接） |
| NRD/vendor 降噪接入实施 | RD040-nrd 承接锚：G12.3 只评估不接线；接入经 UpscaleBackend 同构契约另判 G13+ 窗 |
| 绝对 UE PT 画质通过线 | G15 商用收口期承接；§4.6 不设通过线字面 |
| 正式帧率对标与帧率通过线 | G14 承接；G12.5 只建吞吐基线不设通过线（G10-N11/N16 承接锚字面） |
| GPU 光栅管线双端 A/B 面 | G10-N16/G11-N3 锚定 G14；G12 生产化面 = M96 device megakernel + host oracle 同构兑现面 |
| temporal 底座改写 | RD040-nrd 承接锚口径：接入时不改 temporal 底座；G12.3 只消费既有历史接口面 |
| UE 源码/二进制 vendoring | RFC-0027 许可边界；PathTracing.cpp 只读外部参照 |

## 3. 术语

- **MIS / balance heuristic**：multiple importance sampling——多采样策略按权重混合的方差削减框架；balance heuristic 权重 w_i = n_i·p_i / Σ_j n_j·p_j（每策略样本数 × 该策略 PDF 占比）。
- **NEE**：next event estimation——显式光源采样（阴影射线直接求光源贡献），与 BSDF 采样构成双策略 MIS 对。
- **吞吐自适应 RR**：俄罗斯轮盘终止概率由路径吞吐权重（throughput，路径当前承载的通量权重估计）驱动——吞吐越低终止概率越高；补偿因子 1/(1−p_kill) 维持无偏。
- **低差异序列**：quasi-random 序列（Sobol 类）与分层采样（stratified）——以确定性低差异结构替代独立随机流的样本分布形态；**索引推导确定性** = 样本值完全由（像素索引， 采样索引， 维度）确定函数寻址。
- **自适应 spp 终止**：逐像素方差估计达阈即停（variance-driven termination）；**收敛误判率** = 判收敛像素中相对全 spp 参照偏差超带的比例。
- **噪声谱**：帧误差的高频能量谱（FFT/DCT 域高频段能量占比）——降噪有效性的 measured 面。
- **均值能量守恒**：降噪/滤波前后帧均值能量差 ≤ measured 容差——不引入系统性变暗/变亮偏置。
- **正确性锚**：M96 参照器既有判据集合（RXS-0357 起步范围/固定 seed 确定性协议/pbrt-v4 容差带/golden 门序 D2-Q7）+ `g9_m96_pbrt_tolerance_band.json` 冻结带——生产化演进的 0-byte 基准。
- **基线锚**：`g12_budget.json` 的 `g12.pt.ref_curve_*` 条目——M96 冻结带 measured 收敛曲线值转录的生产化回归锚（direction=max：重登记值不得大于锚）。
- **UE PathTracing 模块归属差距登记**：对标差距逐项登记 UE5 模块归属（`Engine/Source/Runtime/Renderer/Private/PathTracing.cpp` 等归属行集，RXS-0391 归属枚举口径继承）。

## 4. 拟议语义（Draft）

### 4.1 M158 — MIS 完整面语义

**L1（双策略逐顶点 MIS）**：路径每个非首顶点处，光源采样（NEE）与 BSDF 采样构成双策略 MIS 对，权重 = balance heuristic（w_nee = p_nee / (p_nee + p_bsdf)，w_bsdf = p_bsdf / (p_nee + p_bsdf)；p_* 为该顶点处两策略各自 PDF 换算到同一测度）。delta 光源（点光）NEE 策略 PDF 按冲激处理——BSDF 策略对 delta 光源概率为零，权重退化 w_nee=1 不产生除零；非 delta 光源（面光/发光面）双策略均可能非零，权重闭式计算。

**L2（多光源 MIS）**：多光源场景 NEE 先按光源分布采样光源（离散 PDF），再按光源采样点（连续 PDF）——联合 PDF = 离散 × 连续；MIS 分母含全部光源的 NEE 联合 PDF 之和（不重不漏）。光源分布构建确定性（同场景同分布 digest）。

**L3（能量守恒）**：白炉场景（全白 furnace，albedo=1 全反射域）MIS 渲染结果均值 = 入射能量（守恒容差 measured 标定）；逐级能量增量单调不增（RXS-0395 口径继承——每加一级反弹，路径能量增量不增）；**只丢能量不漏光**（RXS-0358 口径继承：能量损失只允许数值截断方向，不允许负辐射或漏光注入）。

**L4（正确性锚 0-byte）**：M96 既有判据 0-byte——起步范围冻结维持（MIS 完整面不改材质集合，Lambert/发光两类维持）；固定 seed 位级确定性协议维持（MIS 权重计算全 f32/f64 确定函数，无数据相关分支序差异）；golden 门序 D2-Q7 维持（M96 未绿任何下游门不得验收）。同 spp 收敛曲线不劣于基线锚（g12_budget `g12.pt.ref_curve_*` 锚，容差标定程序产）——MIS 是方差削减升级，收敛劣化即语义错误。

### 4.2 M159 — 俄罗斯轮盘生产化语义

**L1（吞吐自适应终止概率）**：自第 N_min 级反弹起（N_min ≥ 2 最小反弹保障——低深度不早杀），路径按 p_kill = clamp(1 − T/τ, 0, p_max) 概率终止（T = 当前路径吞吐权重估计，τ = 吞吐参考阈——标定程序产禁手写；p_max < 1 恒成立——任何深度保留非零续行概率，禁截断偏置）。

**L2（无偏补偿）**：续行路径权重乘以补偿因子 1/(1−p_kill)——RR 补偿闭式无偏（ estimator 期望不变）；补偿因子上界登记（防数值爆炸，钳制面显式登记）。**补偿缺失冒充无偏即 RED**。

**L3（计数非空）**：RR 终止率（终止路径数/总路径数）、补偿因子分布（p50/p90/max）逐场景进 evidence——无计数面不得验收。

**L4（RED 臂继承）**：跳 RR 偏移 RED 臂（RXS-0357 三臂面继承——关 RR 后同 seed 输出 digest 必须偏离 golden 臂对拍面；早杀偏置注入（N_min 违反/补偿缺失/p_max=1）独立 RED 臂。

### 4.3 M160 — 采样策略升级与低差异序列确定性协议扩展

**L1（低差异序列面）**：采样维度序列自逐像素独立 PCG 流演进为分层/低差异序列——候选面 = Sobol 类低差异序列（Owen  scramble 或确定性种子扰动）与分层采样（stratified per-dimension）；选型经 benchmark measured 裁决（收敛曲线对照），只测量不定档的纪律不适用（本条款冻结选型面——选型证据进 evidence）。

**L2（确定性协议加性扩展）**：低差异序列索引推导确定性——样本值 = f（像素索引， 采样索引， 维度， seed）确定函数寻址，无任何数据相关状态；**固定 seed 两次运行位级一致维持**（同 seed ⇒ 序列位级一致 ⇒ 输出位级一致；canonical digest 口径沿 RXS-0357 L2 字面）；RNG 流布局 provenance 进 evidence（序列族/扰动面/寻址公式字面）。RXS-0357 L2 既有字面 0-byte——本条款为**加性扩展**（新条款承载，修订行衔接）。

**L3（收敛不劣于）**：同场景同 spp 收敛曲线 measured 不劣于独立 PCG 流锚（g12_budget `g12.pt.ref_curve_*` 锚，容差标定程序产）——低差异序列的卖点是收敛加速，劣化即语义错误；序列篡改/非确定注入 RED 臂。

### 4.4 M161 — 收敛判据生产化语义

**L1（方差驱动自适应终止）**：逐像素维护 Welford 类在线方差估计（Σx/Σx² 协议沿 RXS-0357 L2 out_stats 面）；像素方差（或相对误差界）达阈即停采——阈值标定程序产（p100×k measured 入 g12_budget，禁手写）。spp 下界保障（每像素最小采样数 ≥ N_floor——防早期方差欠估计早停）。

**L2（收敛报告）**：逐像素 spp 分布（min/p50/p90/max）、方差分布、**未收敛像素计数**（达 spp 上界仍未达阈）非空进 evidence——无报告面不得验收；未收敛像素缺报即 RED。

**L3（误判率标定）**：收敛误判率（判收敛像素中相对全 spp 参照偏差超带比例）≤ 标定阈——标定程序产（对照集 = 自适应帧 vs 全 spp 参照帧同 seed 同场景）；**早停冒充收敛即 RED**。

**L4（golden 不偏离）**：固定全 spp golden 对拍不偏离冻结带（`g9_m96_pbrt_tolerance_band.json` measured×2.0 带继承——自适应帧与全 spp golden 同场景对拍，偏差超带即 RED）；**自适应帧不得冒充全 spp 参照**（evidence 帧型标签闭集 {adaptive, full_reference}，混标即 RED）。

### 4.5 M162 — 降噪管线与 TSR 底座联动语义面

**L1（降噪管线形态）**：时域累积（消费既有 TAA/TSR 历史接口面——历史帧重投影 + 历史验证，**temporal 底座 0-byte 不接线**：不改 TAA/TSR 任何语义/代码面，只消费其历史输出）+ 空域 A-trous 类滤波（小波域多尺度，边缘停止函数消费法线/深度/亮度面）。降噪输入 = PT 原生帧（adaptive 或 full），输出 = 降噪帧；帧型标签闭集 {raw, denoised} 进 evidence。

**L2（噪声底回归）**：降噪帧噪声谱高频能量 < 原生帧高频能量（下降幅度 ≥ 标定阈——标定程序产）；噪声底未降冒充降噪即 RED。

**L3（均值能量守恒）**：降噪前后帧均值能量差 ≤ measured 容差（标定程序产）——不引入系统性变暗/变亮偏置；区域均值能量差分布（p90）进 evidence；偏置注入 RED 臂（人为 ±k 亮度注入必须检出）。

**L4（历史验证与去鬼影）**：时域累积带历史验证（深度/法线/运动一致性拒绝失效历史）——历史污染鬼影面登记；**temporal 底座接线即 RED**（任何 TAA/TSR 语义面改动判 RED——本条款只消费不接线）。

**L5（NRD 类 vendor 降噪评估）**：NRD 类 vendor 降噪评估报告落盘（RD040-nrd 承接锚口径：UpscaleBackend 同构输入契约〔MV/深度/法线〕接入面评估 + 许可/ABI/集成形态取证 + 与自研降噪面 measured 对照）；**评估不接线**——接入另判 G13+ 窗；评估冒充接入即 RED。

### 4.6 M163 — UE Path Tracer 对标口径面

**L1（双端出图口径）**：同场景（场景契约独立冻结——参数 digest 机核，不动 G10.5/G11.5b 锁定值）同 spp（spp 序列同字面）双端出图：Rurix 生产 PT 臂 vs UE 5.8.1 Path Tracer MRQ 臂（F:\UE_5.8；UE build digest == M128 登记 ue_build_id 机核继承；窗口模式主路臂，G10-N8/N9 口径继承）。**契约 digest 不等仍出报告即 RED**（门序硬约束继承 M130/M139/M155）。

**L2（逐段收敛对拍）**：收敛曲线逐段 measured 对拍——spp 序列逐段（如 1/4/16/64/256/1024 段）双端 rel-MAE 曲线差 measured，容差标定程序产；逐段对拍超容差**显式登记即 RED 评审面**（不得静默混入）；噪声谱对拍（高频能量谱差 measured）；能量守恒对拍（帧均值能量差 measured）。

**L3（模块归属差距登记）**：UE PathTracing 模块归属差距登记表落盘——差距逐项登记 UE5 模块归属（`Engine/Source/Runtime/Renderer/Private/PathTracing.cpp` 及关联模块行集，RXS-0391 归属枚举口径继承）；差距项显式登记，不冒充全闭环；登记表行集与对拍报告对账。

**L4（口径对齐先行）**：曝光/位深口径沿 G11.2 对齐口径（RXS-0385 strip-and-log / EV100 派生链互证）；残余口径差逐环节显式登记（未对齐口径消费对拍 delta 即 RED——R-G12-5/R-G11-1 同族纪律）；**不设绝对通过线**——「已达 UE5 PT 画质」叙述 G12 期内一律不成立。

### 4.7 RXS-0357 参照器面演进显式修订行

**L1（修订行形态）**：RXS-0357 既有字面 0-byte——起步范围冻结（焦散/体积/specular 链 out）维持、固定 seed 确定性协议维持、pbrt-v4 容差带维持、golden 门序 D2-Q7 维持；生产化语义（§4.1~§4.6）经**修订行 + 新条款**承载（post-interlock actual-next-free allocation 领新 RXS 条款；修订行显式声明「参照器面生产化演进（G12，RFC-0029）——起步范围冻结维持，MIS/RR/采样/收敛判据生产化语义见 RXS-####」）。

**L2（host oracle 同构兑现面）**：生产化语义在 device megakernel 与 host oracle 双面同构兑现（公式面逐字同源——RXS-0357 host oracle 纪律继承；仅 host 输出不能充绿，门绿由 device 腿承载；GPU 光栅管线面不在本 RFC 范围，锚定 G14）。

**L3（基线锚消费）**：`g12_budget.json` `g12.pt.ref_curve_*` 条目（M96 冻结带转录）为生产化回归锚——各生产化门消费锚断言不劣于；锚条目 0-byte 只消费不回写（新标定条目按 M138/M157 追加先例入 budget）。

## 5. spec 映射表（post-interlock actual-next-free allocation）

| 语义面 | 目标 spec（候选） | 形态 | conformance 锚定候选 |
|---|---|---|---|
| §4.1 MIS 完整面 | `spec/global_illumination.md` 追加条款 | 新条款（RXS-####，post-interlock 领取） | accept `mis_full_surface_minimal.rx` + reject `mis_weight_missing.rx` / `mis_energy_bias_inject.rx` |
| §4.2 RR 生产化 | `spec/global_illumination.md` 追加条款 | 新条款 | accept `rr_throughput_adaptive_minimal.rx` + reject `rr_early_kill_bias.rx` / `rr_compensation_missing.rx` |
| §4.3 采样升级+低差异 | `spec/global_illumination.md` 追加条款 + RXS-0357 L2 修订行 | 新条款 + 修订行（既有字面 0-byte） | accept `lds_deterministic_minimal.rx` + reject `lds_nondeterministic_inject.rx` |
| §4.4 收敛判据生产化 | `spec/global_illumination.md` 追加条款 | 新条款 | accept `adaptive_convergence_minimal.rx` + reject `early_stop_masquerade.rx` / `unconverged_pixel_underreport.rx` |
| §4.5 降噪+TSR 联动 | `spec/global_illumination.md` 追加条款（或 display_pipeline.md 候选——落点裁决 spec-first 波定，候选文件本体 0-byte） | 新条款 | accept `denoise_pipeline_minimal.rx` + reject `denoise_energy_bias.rx` / `temporal_base_rewire.rx` |
| §4.6 UE PT 对标口径 | `spec/visual_comparison.md` 追加条款（度量口径轴） | 新条款 | accept `ue_pt_parity_contract_minimal.rx` + reject `parity_digest_mismatch_report.rx` / `residual_caliber_silent.rx` |
| §4.7 参照器面演进修订行 | `spec/global_illumination.md` RXS-0357 修订行 | 修订行（既有字面 0-byte） | 随上述各条款行 |

目标 spec 新建/落点裁决沿 G9.4 global_illumination.md/G10.4 visual_comparison.md 先例——候选既有卷本体 0-byte，文件头注留痕；条款号一律 post-interlock actual-next-free allocation（不预写推测号）。

## 6. 兼容性、0-byte 边界与联动

### 6.1 既有面 0-byte

| 面 | 约束 |
|---|---|
| RXS-0357 参照器面 | 起步范围冻结/确定性协议/容差带/门序既有字面 0-byte；演进经 §4.7 修订行 + 新条款 |
| `g9_m96_pbrt_tolerance_band.json` | 冻结带只消费不回写；M96 门（g9.p0.m96）判据 0-byte |
| G9~G11 62 门绿面 | 回归门 M164 独立 P0；触改面重跑零降级 |
| G10.5/G11.5b 契约与帧库 | 对标契约独立冻结，锁定值 0-byte |
| temporal 底座 | TAA/TSR 语义面 0-byte（§4.5 L4 接线即 RED） |
| spec 其余各卷 | 候选落点外各卷 0-byte |

### 6.2 与 RFC-0022/0023/0026/0028 的衔接

- RFC-0022（G9 GI 语义）：D2-Q7 门序面与 RXS-0357 冻结面 = 本 RFC 演进基准（0-byte）；D2-Q8 wavefront 阶段化接口面为 MIS/RR/采样演进的接缝面。
- RFC-0023（G9 GPU-driven）：SER 语义冻结面不接线（§6.4 边界声明）；M108 语言原语面与本 RFC 零交集。
- RFC-0026（G10 度量口径）：UE PT 对标度量面（FLIP/SSIM/PSNR/diff/归属枚举）沿用，不新定义度量。
- RFC-0028（G11 GI 闭环）：能量守恒口径（RXS-0395/0396）与闭环判据语义（RXS-0393）继承——本 RFC 生产化判据 = 正确性锚 0-byte + measured 不劣于基线锚，形态沿 RXS-0393 两款收敛判定同族。

### 6.3 与 RD-040 的衔接

- M52 SER：本 RFC 不触碰（§6.4）；M99-clipmap：承接兑现完结维持（G11.6 登记字面 0-byte）；M100-high：G12.4 触发评估登记（对标若产多灯 workload measured 对照面）；RD040-nrd：§4.5 L5 评估窗承接（评估不接线）。

### 6.4 SER 边界声明

SER（Shader Execution Reordering）语言层原语与渲染器集成**不在本 RFC 范围**：M52 重评窗核验 = maintain-defer（G12.1 双条件未命中——真实集成需求未至 + capability rt.ser 设备面未实测）；复评点 = G12.2 生产化核心波 materialize 高分歧 RT workload 集成面时；若复评 go，语言层原语面（HitObject/reorderThread/capability rt.ser）须独立 Full RFC 评估（RFC-0023 §4.7 冻结面衔接——语义面留 RFC-0023 冻结面不接线，本 RFC 不构成 SER 任何承诺）。本 RFC 的生产化语义（MIS/RR/采样/收敛判据）不依赖 SER——megakernel 起步面维持，wavefront 阶段化接口面（D2-Q8）为可选演进接缝不强制。

## 7. 风险与备选

### 7.1 风险

| 风险 | 缓解 |
|---|---|
| 生产化破坏位级确定性（低差异序列/自适应终止/降噪引入漂移） | §4.3 L2 确定性协议加性扩展 + 各门 digest 0-byte 断言（R-G12-1） |
| MIS/RR 语义错误引入能量偏置 | §4.1 L3 白炉守恒 + 逐级能量增量单调不增 + 偏置注入 RED（R-G12-2） |
| 自适应收敛过松冒充收敛 | §4.4 L3/L4 误判率标定 + 全 spp golden 不偏离冻结带 + 帧型标签闭集（R-G12-7） |
| 降噪系统性偏置/鬼影 | §4.5 L3 均值能量守恒 + L4 历史验证（R-G12-3） |
| UE PT 口径差未对齐即对拍 | §4.6 L4 口径对齐先行 + 残余口径差显式登记（R-G12-5） |
| 低差异序列选型争议（Sobol 族选型） | §4.3 L1 选型 benchmark measured 裁决 + 证据进 evidence |

### 7.2 备选方案（已否决）

| 备选 | 否决理由 |
|---|---|
| 直接扩 M96 参照器起步范围（焦散/体积/specular 链入 G12） | 起步范围冻结是 G9.4 经 RFC-0022 冻结的语义面；G11-N8/G11-N9 实测举证（高光尾 57.29 vs 4.88 / 镜面 IBL ≤0.031%）判定该类能量非当前画质量级主差——G12 投在方差削减/收敛/降噪/对标面的边际收益更高；材质链锚定 G15（G12 立项裁决 6） |
| wavefront 重写替代 megakernel 演进 | D2-Q8 阶段化接口面已留位；megakernel 起步是 RXS-0357 冻结面，重写 = 推翻冻结面而非演进；生产化可在 megakernel 内落 MIS/RR/采样/收敛判据 |
| SER 提前接线（借 G12 生产化顺带集成） | M52 承接锚纪律：双条件未命中（G12.1 核验在案）——抢跑即违反承接锚字面；§6.4 边界声明 |
| NRD 直接接入（跳过评估） | RD040-nrd 承接锚：接入经 UpscaleBackend 同构契约另判 G13+ 窗；许可/ABI/集成形态未取证即接入 = 无锚新立 |
| 复用 G10.5 契约 digest 作 PT 对标契约 | G10/G11 closed 复测对照面 0-byte；PT 对标场景/spp 序列面独立——独立冻结不污染既有对照面 |

## 8. 未决问题

| # | 问题 | 处置 |
|---|---|---|
| U1 | 低差异序列具体选型（Sobol-Owen / 分层 / 蓝噪声面） | G12.2 实现波 benchmark measured 裁决后进 evidence；本 RFC 只冻结确定性协议扩展面不冻结选型 |
| U2 | 吞吐参考阈 τ 与误判率阈的具体数值 | G12.2 标定程序 measured 产（M166 门，p100×k 入 g12_budget）；本 RFC 不预写数字（P-09 禁手写） |
| U3 | PT 对标场景集（双场景闭集沿用 or 扩场景） | G12.4 波前裁决——默认沿用 BistroInterior + CornellBox 双场景闭集（M133 清单）；扩容走 M133 只追加修订程序 |
| U4 | 降噪空域滤波具体形态（A-trous 族参数面） | G12.3 实现波定，均值能量守恒与噪声底回归判据不受选型影响（本 RFC 冻结判据面不冻结算法选型） |

## 9. 评审记录

### 9.1 对抗性评审记录（D-409 第 1 轮，2026-08-17）

评审轮次与起草轮次隔离（同会话族、独立评审视角重读全文，zero shared context 声明按单会话先例如实登记）；findings 与 disposition：

| # | 级别 | finding | disposition |
|---|---|---|---|
| F1 | high | §4.1 L1 delta 光源 MIS 权重退化路径未定义——点光（delta）BSDF 策略 PDF 为零时 w_nee 计算除零风险 | **采纳并修 §4.1 L1**：delta 光源 NEE 策略 PDF 按冲激处理、BSDF 策略概率为零 → 权重退化 w_nee=1 不产生除零；非 delta 光源双策略闭式 |
| F2 | high | §4.2 L1 p_max=1 或 p_max 未界定时 RR 截断引入偏置（任何深度路径被杀即系统性变暗） | **采纳并修 §4.2 L1**：p_max < 1 恒成立（任何深度保留非零续行概率，禁截断偏置）+ N_min ≥ 2 最小反弹保障 |
| F3 | high | §4.4 自适应终止若无 spp 下界，早期方差欠估计导致大面积早停假收敛 | **采纳并修 §4.4 L1**：spp 下界保障（N_floor——防早期方差欠估计早停）+ L3 误判率标定 + L4 帧型标签闭集 |
| F4 | med | §4.3「只测量不定档的纪律不适用」与 G9 M120 先例的关系未说明——采样选型是否构成「不定档」豁免 | **采纳并修 §4.3 L1**：本条款冻结选型面（选型证据进 evidence）——与 M120「仅测量不定档」（OIT 档选定程序未解冻）不同族：采样选型是语义面选择非档位承诺，benchmark 裁决证据进 evidence 即闭环 |
| F5 | med | §4.5 L1「消费既有历史输出」与「temporal 底座 0-byte」的机核边界未定义——何种改动算接线 | **采纳并修 §4.5 L4**：temporal 底座接线即 RED——任何 TAA/TSR 语义面/代码面改动判 RED；消费 = 只读历史输出接口面 |
| F6 | med | §4.6 L2 spp 序列「如 1/4/16/64/256/1024 段」像写死的数字 | **采纳并修 §4.6 L2**：序列为示例形态（「如」字面）——实际序列进场景契约 digest（G12.4 波裁决）；容差标定程序产不预写 |
| F7 | med | §5 降噪条款落点（global_illumination.md vs display_pipeline.md）未定 | **采纳并修 §5**：落点裁决 spec-first 波定（候选文件本体 0-byte，沿 G9.4/G10.4 新建/落点先例）——本 RFC 不预决 |
| F8 | low | §4.1 L2 多光源 MIS「联合 PDF = 离散 × 连续」未明确光源分布确定性 | **采纳并修 §4.1 L2**：光源分布构建确定性（同场景同分布 digest） |
| F9 | low | §6.2 与 RFC-0028 衔接未提 RXS-0393 两款收敛判定同族关系 | **采纳并修 §6.2**：生产化判据形态沿 RXS-0393 两款收敛判定同族 |
| F10 | low | §8 U3 场景集问题与 G10-N6（BistroExterior defer）联动未登记 | **采纳并修 §8 U3**：扩容走 M133 只追加修订程序（G10-N6 承接锚字面维持——FBX2glTF 缺口不随 PT 对标静默扩容） |

**provenance 偏差如实登记**：本环境为单模型会话族——评审者与起草者同模型（Kimi-K3）、独立性 = 评审轮次隔离（独立评审视角重读 + findings 驱动修法批），非跨工具/跨模型/跨会话独立实例；偏差大于 RFC-0024「同工具族独立实例」先例，与 RFC-0025「单实例偏差」先例同族——按 RFC-0015 §9.1 / number_ledger v1.29/v1.73/v1.90/v1.102 先例如实登记，**效力自限**：本评审不主张跨工具独立性等效，留 G12.7b 终审复核锚（close-out 时若有跨工具评审者可得，补一轮评审再 flip status）。评审全文（findings 详述与修法 diff）见 [rfc0029_adversarial_review.md](../milestones/g12/design/rfc0029_adversarial_review.md)。

评审会话 provenance：`Assisted-by: Kimi-K3（D-409 独立评审轮次，与起草轮次隔离）`。
