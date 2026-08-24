<!-- Assisted-by: Cursor Agent（G23.1 治理波） -->
# RFC-0040 — 物理平台深化：Jolt 5.6 采纳臂重判 + 神经变形重判 + 研究轨/物理 P3+ 处置程序

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0040（落盘前实测 ledger RFC next_free=40 顺位领取） |
| 状态 | Agent Approved（经对抗评审 milestones/g23/design/rfc0040_adversarial_review.md，D-409 对抗性评审要求程序） |
| 判档 | Full RFC（生产默认切换程序面留档；物理为引擎库面零新语言语义条款，G6 先例） |
| 承接 | G23.2 M-a/M-b + G23.3 M-c/M-d |
| 上游 | M125-adopt3（G17~G22 六期 defer 链）、M127、RD-042/043/044 |

## 1. 摘要

1. **Jolt 5.6 采纳臂重判程序（M-a）**：采纳三件 = ①升格裁决（A/B 证据面：g9_m125 两臂诚实登记绿件只读盘点 + sys56 构建新鲜真跑 `cargo check -p rurix-physics-sys56`）②生产默认切换（rurix-physics 默认绑定 5.3→5.6 flip——需求证据面：生产 workload 对 5.6 独有特性/修复的依赖证据）③5.3 退役程序（VENDOR.md pin 退役 + U33~U53 审计面迁移）。三件成立条件核验：②的需求证据缺 ⇒ **maintain-5.3**（评估臂存续、不升格）；登记 `g23_jolt_adoption_registry.json`。
2. **神经变形重判程序（M-b）**：M127 两半分别实测——corpus 半边（离线工具链语料树内存在性）+ 消费方半边（PhysicsAsset residual 消费代码面存在性）；两半未命中 ⇒ maintain 研究子轨。
3. **研究轨处置（M-c）**：RD-042（Newton/Genesis/MuJoCo-Warp 可微物理观察）+ RD-043（wgrapier GPU 刚体观察）逐轨 disposition 闭集 `g23_research_track_registry.json` + 两条 history 只追加；观察存续/关闭均合法。
4. **物理 P3+ 分项处置（M-d）**：RD-044 三分项（Jolt 软体/布料/流体生产化、Taichi MPM 生产 import、Rapier 快路径〔M126 maintain-no-go measured 在案转引〕）disposition 闭集 `g23_rd044_subitem_registry.json` + history 只追加。
5. **out-of-scope**：无需求证据的生产默认 flip、软体/布料/流体实现、MPM 生产 import（各附承接锚）。

## 2. 不变量

- 5.3 基线生产默认面 0-byte（rurix-physics-sys VENDOR.md pin 不动）；sys56 评估臂隔离共存（JPC56_/JPH56 符号隔离）不升格。
- 旧门只读消费：g9_m125/g9_m126 绿件禁 --gate 重跑。

## 3. 终态程序

M-a~M-d 真跑 evidence 落档后本 RFC 终态 = 采纳臂重判记录（maintain-5.3/adopt 字面）+ M127 两半记录 + 三 RD 处置闭集；争议时按只追加程序重判。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | G23.1 起草；对抗评审后 Agent Approved。 |
