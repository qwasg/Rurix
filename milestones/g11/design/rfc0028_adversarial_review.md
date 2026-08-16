<!-- Assisted-by: Kimi-K3（D-409 独立评审会话，与起草会话隔离） -->
# RFC-0028 D-409 第 1 轮对抗性评审记录（2026-08-16）

> **评审对象**：[`rfcs/0028-g11-gi-quality-closure.md`](../../../rfcs/0028-g11-gi-quality-closure.md) Draft v0.1（G11 GI 与光照画质闭环语义伞形）。
> **评审者 provenance**：`Assisted-by: Kimi-K3（D-409 独立评审会话，与起草会话隔离）`
> **会话形态**：独立评审轮次、不复用起草结论——评审者对以下事实源独立复核：① G10.8b 锁定清单（`milestones/g10/g10_gap_registry.json` R3/R4/C1 行 measured delta 与承接锚逐字）；② G10.6 重评窗 M99-clipmap rejudged-go 承接锚字面（`G10_DEFER_REEVALUATION.md` §1）；③ `spec/global_illumination.md` RXS-0357~0362 冻结面（门序/能量守恒/降级链/屏幕级登记逐条）；④ `spec/visual_comparison.md` RXS-0384~0391 度量口径面；⑤ G11_CONTRACT §4.2 M144/M153/M154 判据字面与 §7 立项裁决 3/4/5/8；⑥ 调研报告 2（GI-1.0/SHaRC/Lumen 蓝本面，含 DDGI 推荐度字面）；⑦ G10.5 锁定契约 digest 溯源 evidence（`evidence/g10_m130_dual_determinism_contract_20260815T233315Z.json`）。
> **provenance 偏差登记**：评审者与起草者**同模型**（Kimi-K3），独立性 = 评审轮次隔离 + 不复用起草结论，不满足 D-409 首选「跨工具/跨模型」字面。按 RFC-0015 §9.1 / number_ledger v1.29/v1.73/v1.90/v1.102 已登记先例如实偏差登记并效力自限：本评审不替代未来跨工具评审；跨工具评审者可得时建议补一轮；留 G11.7b 终审复核锚。

## 1. Findings（12 条：3 high + 5 med + 4 low）

| # | Finding | 严重度 | 位置 |
|---|---|---|---|
| F1 | **host 参考管线消费面与复测臂口径存在实现体量矛盾且形态悬空**：§4.1.4 要求 host CPU 参考管线消费「同一双级语义」（空间哈希世界缓存 + 多反弹），这是真 3D GI 求解器工程量，与「参考管线」定位冲突；若不落地，R4 delta（HDR p90 4.697253086805343）不可能收敛，M154 判据字面直接无法满足；§9 Q6 把形态（同构 vs 解析式）留为未决——判据依赖项不能悬空 | high | §4.1.4 / §4.2 / §9 Q6 |
| F2 | **收敛判定方向性缺陷 + C 族行闭环语义错位**：§4.6.2 以「\|复测 delta\| < \|基线 delta\|」统一判定——①收敛语义应为 delta → 0（双端一致），非绝对值单调缩小（反向过冲/部分修复仍 disagree 时绝对值可缩小而场景未一致）；②caliber_diff 行（C1/C2/C3）闭环 = 口径对齐完成 + 残余显式登记，与 quality_gap 行的 delta 收敛语义不同款，统一字面会误导演出「口径差也被修没」的伪闭环 | high | §4.6.2 |
| F3 | **世界级验收判定锚不可机核**：§4.2.4「远场探针集能量回归 measured 非零且与参考对拍一致」——「参考」未定义（UE Lumen 对拍 vs M96 golden 对拍混用则判定不可机核）；「非零」阈值过弱（任意噪声即非零，伪绿通道） | high | §4.2.4 |
| F4 | **R3 光源参数双通道未冻结**：§4.3.2「经契约光照参数面（M130 schema 面）或包内 glTF 字段消费」——「或」字双通道并存，Rurix harness 与 UE build_scenes 可各走一路，破坏 M130 契约 digest 一致性（门序硬约束面） | med | §4.3.2 |
| F5 | **C1 天光对齐「同参数」不可机核 + cubemap 资产面未登记**：§4.5.1 参数集未枚举；UE SkyLight 指定 cubemap 的具体资产与许可面（M131 白名单联动）未声明 | med | §4.5.1 |
| F6 | **R3 目标 spec 卷摇摆**：§5 映射表「global_illumination.md 或灯光语义面归属卷（实现波裁决）」——spec-first 落点应治理期裁决（G9 先例：目标卷治理期裁定，候选既有卷本体 0-byte） | med | §5 |
| F7 | **世界级缓存与 RXS-0359 L4 Far Field 档边界未声明**：M98-l4 维持 defer（L4 = 追踪降级链远场档）；世界级辐射缓存若与 L4 语义面混同，构成对 defer 行的静默兑现 | med | §4.4.2 |
| F8 | **契约 digest 锁定值缺机核验溯源**：§4.6.3 双 digest + 联合 digest 为转录字面，未标注溯源 evidence——字面转录错误则 G11.5 复测门序整个错位 | med | §4.6.3 |
| F9 | **漏光像素计数 = 0 断言的适用面未说明**：自 RXS-0358 Surface Cache 语境继承，双级 GI（屏幕探针 + 世界缓存）语境下漏光定义未给出 | low | §4.1.2 |
| F10 | **§7 备选 2 对 DDGI 表述与调研报告矛盾**：调研报告 2 §1.1 #12 载 DDGI 为「MVP 备选形态」高推荐度；本 RFC「滞后与泄漏控制成本高」无出处且与蓝本面冲突 | low | §7 备选 2 |
| F11 | **距离自适应量化函数族未冻结**：§4.2.1「近细远粗」无函数族语义（线性/对数/幂律），实现者可任意发明（口径漂移面） | low | §4.2.1 |
| F12 | **评审 provenance 与起草同模型，偏差须随 findings 一并回填**（RFC-0026 F17 先例） | low | §9.1 |

## 2. 评审结论

**总评 = approve-with-changes**（修订后可批准，非现状可批准）。三条 high（F1~F3）触及 M154/M155 判据可满足性与收敛判定的机核性，必须正文实修；五条 med（F4~F8）为口径/边界/溯源冻结缺口；四条 low（F9~F12）同批 disposition。建议 Draft v0.2 修法批逐条落实后，由主会话核对契约/MAP/CI_GATES 三面一致（`ci/check_g11_acceptance_map.py` PASS）翻 Agent Approved。

## 3. 独立事实核对记录（评审者独立复核面）

| # | 核对项 | 结果 |
|---|---|---|
| 1 | R4 行 measured delta（bistro HDR p90 a=0.30276253819465637 / b=5.000015625 / delta=4.697253086805343）与 `g10_gap_registry.json` 逐字一致 | ✔ |
| 2 | C1 行双 measured delta（bistro HDR 中位 2.664779790997505 + cornell p90 ×2^(−EV100) 0.29024957587122924）与清单逐字一致 | ✔ |
| 3 | M99-clipmap 承接锚字面（rejudged-go → G11 画质修复期承接；兜底 = 屏幕级 SPG + Radiance Cache g9.p1.m99 门绿维持）与 `G10_DEFER_REEVALUATION.md` §1 逐字一致 | ✔ |
| 4 | RXS-0360「世界级 clipmap 未 measured 举证 not-triggered 不充绿」登记字面在树（spec/global_illumination.md §6）——翻转确需显式修订行 | ✔ |
| 5 | RXS-0357 L6 门序硬约束（M96 golden 未绿 GI 门不得验收）与 RXS-0358 能量守恒口径、RXS-0359 禁静默回退在树 | ✔ |
| 6 | G10.5 契约 digest 锁定值（cornell sha256:80305791… / bistro sha256:ad45951b… / 联合 sha256:64fd54df…）溯源 evidence `g10_m130_dual_determinism_contract_20260815T233315Z.json` 在树（F8 事实依据） | ✔ |
| 7 | 调研报告 2 §1.1 #12 DDGI「MVP 备选形态」高推荐度字面（F10 事实依据）与 §3.2 P2 世界辐射缓存行（空间哈希 + 辐射 LOD + 回落，蓝本面） | ✔ |
| 8 | G11_CONTRACT §7 裁决 3/4/5/8（闭环判据 / M99-clipmap 承接 / RFC 判档 / 复测臂口径）与本 RFC 引用逐字一致 | ✔ |
| 9 | M98-l4 维持 defer（G10.6 重判 maintain-defer，承接锚 0-byte）——世界级缓存与 L4 语义面边界声明必要性（F7 事实依据） | ✔ |
| 10 | 编号台账实测：RFC next_free=28（本 RFC 编号 RFC-0028 与实测一致，非推测号） | ✔ |
