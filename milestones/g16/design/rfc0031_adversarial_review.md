<!-- Assisted-by: Cursor Grok 4.6（G16plus D-409 对抗性评审轮次——与起草轮次隔离） -->
# RFC-0031 D-409 对抗性评审记录（第 1 轮，2026-08-24）

> **性质**：D-409 对抗性评审交付物（G16plus 治理批）。评审对象 = `rfcs/0031-g16plus-gi-expression-quality-closure.md` v0.1。
> **Provenance 偏差如实登记**：评审者与起草者同模型同会话族；独立性 = 评审轮次隔离 + 独立重读事实源。效力自限：跨工具评审者可得时建议补一轮；留 M-h / close-out 终审复核锚。与 RFC-0030/0029 先例同族。

## 1. 事实源独立核对清单

| 事实源 | 核对结论 |
|---|---|
| `G16_CONTRACT.md` out_of_scope `gi_expression_rfc` / `absolute_quality_deficit_closure_beyond_honest_review` / `g16_closeout_and_soak` | v0.1 要求只追加修订把三面移入 G16plus in_scope——与用户 2026-08-24 强制收口画质指令一致；须 §8 只追加记录，禁止回写 §8.2 0/18 历史门 |
| `g14_3_pipeline_perf.rs` `--gi on` fail-closed | 加性车道必须保持 `--gi off` 默认臂 0-byte；v0.1 §4.1 成立 |
| `kernels/g14_3_direct_gi.rx` | 已有 quad 4×4 NEE + point + emissive 主命中；**无次级反弹**。`gi/tracer.rs` 仅 sun+sky，cornell 契约 sun/sky=0——接现成探针管线不够（F1 已预见表述，须写死「必须扩 NEE 到次级命中」） |
| `g16_m_c_review_matrix.json` | cornell SSIM deficit≈0.624 / 阈 0.00252（≈247×）；bistro≈0.059 / 0.00397（≈15×）。18/18 是退出条件不是事前承诺（P-09）——v0.1 §2.3 成立 |
| `spec/global_illumination.md` RXS-0357 起步范围 | 焦散/体积/specular **out** 维持——v0.1 不得打开 |
| RXS-0395/0396 | GPU 面「锚定 G14」——本 RFC 才接线；host 绿不充 GPU 18 格（G11-N3） |
| `g13_ue_lumen_gap_registry.json` | 两行 0-byte 不回写；重测只进 `g16_quality_gap_disposition.json` |
| `registry/deferred.json` RD-040 | 条目 status 维持 open；history 只追加「G16plus GI 表达窗」 |
| 异己 `gi/restir.rs` 等 | 已移 `.tmp/g16plus_alien_archive/`；v0.1 零消费成立 |

## 2. Findings（分级 + disposition）

| # | 级别 | finding 字面 | disposition（v0.2 落实） |
|---|---|---|---|
| F1 | high | 只写「打开 --gi on」会被实施成接 `gi/pipeline.rs` 单反弹 sun+sky，cornell 天花 RectLight 仍无次级 NEE | §4.1 L1 写死：次级命中必须对 emissive/quad 再做 NEE；禁止只走 sun+sky |
| F2 | high | 若改 `g14_3_direct_gi.rx` 本体，G13/G14/G15 `--gi off` digest 锚全毁 | §4.1 L2：新 kernel 文件；off 臂源/默认 SPV 路径 0-byte |
| F3 | high | 把 M-c「0/18 如实也绿」改成 18/18 会 retroactive 红已绿 evidence | §4.3：新 M-g 门；M-c 历史语义 0-byte |
| F4 | med | 18/18 事前承诺违反 P-09 | §2.3 / §6：退出条件 ≠ 承诺；未进带保持 active |
| F5 | med | soak/close-out 与 M-g 并行会伪造收口 | §5：6a/6b 硬前置 M-g 绿 |
| F6 | low | RD-040 被误关 | history 只追加，status 维持 open |
| F7 | low | 伞形偏大，cornell 未证明间接光就开 bistro | 波序硬依赖：G16.8 机核非近零才进 G16.9 |

## 3. 评审结论

v0.1 与在树事实源零冲突（核对清单 9 项）。F1~F7 全部于 v0.2 修法批落实。建议翻 Agent Approved 的前置：主会话核对 RFC ↔ 契约 §8.3 ↔ MAP 附录 A 三面一致。

签署：白栀（依 10 §7 / P-13 / D-406 v2.0；D-409 评审轮次隔离；provenance 偏差见头注）。
