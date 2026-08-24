<!-- Assisted-by: Cursor Grok 4.6（G16plus RFC-0031 起草 + v0.2 修法批） -->
# RFC-0031 — G16plus GI 表达与绝对画质收口语义

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0031（按 2026-08-24 实测 `registry/number_ledger.json` namespaces.RFC `next_free=31` 领取，非推测号） |
| 标题 | G16plus GI 表达与绝对画质收口（生产加性 `--gi on` 车道 / cornell 面光次级 NEE / ≥2 反弹 / 18 格 M-g 再审） |
| 档位 | **Full RFC**（① 触 G14「GI 多反弹臂不接线」架构裁决；② 触 RXS-0395/0396 GPU 生产化锚定 G14；③ 触绝对画质 deficit 收口。判档向上取严，10 §3） |
| 状态 | **Agent Approved**——D-409 第 1 轮 findings 全部 disposition（§9.1）；主会话核对契约 §8.3 / MAP 附录 A / 本 RFC 三面一致（2026-08-24） |
| 承接里程碑 | G16（G16.6~G16.10 延续波；验收面 = 附录 A M-e/M-f/M-g/M-h） |
| 关联条款 | 拟落 spec 条款号 **post-interlock actual-next-free allocation**（现快照 RXS next_free=408；禁推测号）。RXS-0357 起步范围字面 0-byte（焦散/体积/specular **out** 维持） |
| 依据决策 | D-406 v2.0 · D-409 · P-09 · P-13 · 用户 2026-08-24「一次性完美完成G16」+「强制收口画质」· 用户 2026-08-19 可商用授权 · [G16PLUS_RECORD](../milestones/g16/G16PLUS_RECORD.md) |
| Provenance | `Assisted-by: Cursor Grok 4.6（G16plus 治理立项起草）` |
| Agent 批准 | **已批准**（2026-08-24） |
| 对抗性评审 | **已完成**（[rfc0031_adversarial_review.md](../milestones/g16/design/rfc0031_adversarial_review.md)） |

---

## 1. 摘要

G16.1~G16.5 已修好 UE cornell 参照臂；商用收口诚实 **0/18**。本 RFC 冻结 G16plus 把生产臂从直接光接到间接光、再对 UE Lumen-on 重跑 18 格的语义：

1. **加性 `--gi on` 车道**：新 kernel，不改 `g14_3_direct_gi.rx` / 默认 `--gi off` SPV。
2. **次级 NEE**：次级命中必须对 emissive 与 quad 面光再采样；禁止只走 `gi/tracer.rs` 的 sun+sky（cornell 契约 sun/sky=0）。
3. **≥2 反弹**：第一反弹可见色bleed；第二反弹经世界缓存或等价二次查询，只丢能量不漏光（RXS-0358 口径）。
4. **新 M-g 门**：`met_count==18` 才 PASS；不改已绿 M-c「x/18 如实」语义。
5. **阈仍 `p100×2.0` 程序产**，禁手写（P-09）。

Agent Approved ≠ 实现许可。实施按 G16PLUS_RECORD §2 波序 + 契约 §8.x 只追加。

## 2. 动机、范围与治理门

### 2.1 为什么需要 Full RFC

18 格对的是 UE Lumen 充盈帧。生产臂 `lighting_model = direct_only`。cornell 隐含 SSIM≈0.376、超阈≈247×。接线改变内容模型，且触 RXS-0395/0396 GPU 面。MR 不承载。

### 2.2 双门

| 门 | 允许 | 禁止 |
|---|---|---|
| G16.6 governance | 本 RFC / D-409 / MAP 附录 A / 步骤 288~292 materialize | 改 GI 冻结面 src |
| G16.7+ | 按波序兑现 §4 | 跳过 cornell 机核去充 bistro；Approved 前改 src |

### 2.3 退出条件 ≠ 承诺（P-09）

18/18 是 M-g / close-out 退出条件。未进带则保持 `active`，不伪造 close-out，不放宽 k。

### 2.4 范围 / 非范围

**in**：§4.1~§4.4；cornell 然后 bistro；G16 处置表重测。

**out**：FG/MFG；ReSTIR/M100-high（异己 restir 零消费）；DLSS NGX / G15-MD-F1；改坐标尺度；回写 G13/G15 冻结表；焦散/体积/specular；`--gi off` 默认臂改图。

## 3. 术语

- **加性 GI 车道**：`--gi on` 选新 kernel；`--gi off` 仍 `g14_3_direct_gi`。
- **次级 NEE**：间接命中点对 quad/emissive/point 再做可见性采样。
- **M-g**：新绝对画质收口门，与历史 M-c 分列。

## 4. 拟议语义

### 4.1 生产加性 `--gi on`（F1/F2）

**L1**：新 `kernels/g16_gi_multibounce.rx`（或同职责新文件）。主射线与 off 臂同式（jitter unproject + closest-hit + 双面 Lambert 直接光 + 4×4 quad + point + emissive 主命中）。**加性**：cosine 半球（或等价）次级射线 ≥1；次级命中累加 emission + 对该点再做 quad/point NEE。禁止只接 `gi/pipeline.rs`/`RadianceTracer` sun+sky。

**L2**：`g14_3_direct_gi.rx` 与默认 `--spv-scene` 路径 0-byte。`--gi off`（默认）行为与 G16.5 基线位级同模。`--gi on` 不再 fail-closed。

**L3**：固定 seed 双跑位级一致（RXS-0357 L2 继承，协议字面 0-byte）。

### 4.2 ≥2 反弹与世界缓存（RXS-0395/0396 GPU 接线）

第二反弹：世界缓存查询或第二次次级射线。失效必须回落（禁静默零辐射）。只丢能量不漏光。屏幕探针近场可选，不得替代 cornell 面光 NEE。

### 4.3 绝对画质收口门 M-g（F3/F4）

新 `g16.p0.m_g.absolute_quality_closure`。生产臂 **GI on** vs UE Lumen-on。双 seed 重标定写入 `g16_budget` **新**条目。`commercial_closure.verdict==达标` ∧ `met_count==18` 才 PASS。M-c 历史 0/18 门 0-byte。

### 4.4 Lumen 差分重收割 M-f

新脚本 import G13 函数，不写 G13 两表。`indirect_ssim` / `gi_energy_rel` 入 `g16_quality_gap_disposition.json`。

## 5. 波序与 soak（F5/F7）

G16.7 诊断 → G16.8 cornell（间接光能量非近零 + 读图色bleed）→ G16.9 bistro → G16.10 M-g。soak≥1800s 与 close-out **仅当 M-g 绿**。

## 6. RED 臂

- `--gi on` 仍走 off kernel / 或 sun+sky-only → RED
- 手写阈 / 改 k → RED
- M-c evidence 被改写成达标 → RED
- M-g 未绿跑 close-out READY → RED
- 默认 off 臂 digest 相对 G16.5 基线漂移 → RED

## 7. 兼容

G5~G15 closed 判据 0-byte。84 门 `--verify-latest`。旧脚本禁 `--gate`。RD-040/045 等八条 open 维持。

## 8. 测试与验收

M-e/M-f/M-g/soak/closeout 独立 evidence。MAP 附录 A 不进 §1 四行闭集。

## 9. 修订与评审

### 9.1 D-409

评审全文见 `milestones/g16/design/rfc0031_adversarial_review.md`。F1~F7 已落入 §4.1 L1/L2、§4.3、§2.3、§5、§7。单模型会话族 provenance 偏差如实登记，留 M-h 复核锚。

| 版本 | 日期 | 变更 |
|---|---|---|
| v0.1 | 2026-08-24 | 起草 |
| v0.2 | 2026-08-24 | D-409 修法批；翻 Agent Approved |
