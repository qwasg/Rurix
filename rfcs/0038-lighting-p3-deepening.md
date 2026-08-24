<!-- Assisted-by: Cursor Agent（G21.1 治理波） -->
# RFC-0038 — 光照 P3+ 深化：ReSTIR 高档 reservoir host 参考臂 + SER 重判 + RD-040/RD-034 处置程序

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0038（落盘前实测 ledger RFC next_free=38 顺位领取） |
| 状态 | Agent Approved（经对抗评审 milestones/g21/design/rfc0038_adversarial_review.md，D-409 对抗性评审要求程序） |
| 判档 | Full RFC（光照采样新登记面；渲染器库面零新语言语义条款，G5 先例） |
| 承接 | G21.2 M-a/M-b + G21.3 M-c/M-d |
| 上游 | G18 M100-high closed-go 重判锚、G20_P2 §1 M52、RD-040、RD-034 |

## 1. 摘要

1. **ReSTIR 高档 reservoir host 参考臂（本期实现）**：`rurix_render::gi::restir_reservoir`——流式加权蓄水池采样（WRS）实现 RIS 估计子（无偏权 `W_y = w_sum/(p̂(y)·m)`，Bitterli 2020 §3）+ 时域 reservoir 合并（历史以 m 计数入池 + M-cap 截断防置信漂移）。M100-high 重判条件「高档 reservoir 证据齐备」的证据产出面。
2. **程序产判据（禁手写）**：① 无偏性 = 三估计子（uniform/RIS/RIS-时域）对解析全灯和参考的 3σ 检验；② 方差收益 = 等验证预算下 `var(uniform)/var(RIS) > 2` measured；③ 时域再收益 `var(RIS)/var(temporal) > 1.2` measured；④ 固定 seed 双跑位级。
3. **0-byte 纪律**：M100 低档 MegaLights 生产默认档维持（multi_light.rs 与其 `check_restir_trigger`/`restir_serve` fail-closed 登记面不接线不改写）；本模块为独立加性算法面，接线归后续 device/集成波。
4. **SER 重判程序（M-b）**：M52 重判条件两半分别实测——capability 半边 = vulkaninfo 扩展枚举取证（`VK_NV_ray_tracing_invocation_reorder`/`VK_EXT_ray_tracing_invocation_reorder` + ReorderingHint 字面入档）；workload 半边 = 高分歧 RT workload 宿主车道存在性核验（Rurix 生产车道 = RayQuery compute 单 kernel，RT pipeline/SBT 车道零实现 = RD-040 分项 open）。capability-hit + workload-miss ⇒ maintain-defer 合法；语言层不加 SER 原语兜底 0-byte。
5. **RD-040 五分项处置闭集（M-c）**：SMRT / 世界辐射缓存演进 / NRD 降噪 / OMM / RT pipeline+SBT 逐分项 disposition 登记 `milestones/g21/g21_rd040_subitem_registry.json` + RD-040 history 只追加。
6. **RD-034 上游复查程序（M-d）**：blocked 恒跑探针（`ci/meshrt_probe_smoke.py`）真跑复查 + RD-034 history 只追加；解锁/维持 blocked 均合法。
7. **device kernel 车道（本期 out-of-scope）**：reservoir device 化 + 空间重用（spatial reuse）+ M100 车道集成显式登记 out-of-scope，承接锚 = 后续 device 波。

## 2. 不变量

- M100 低档面/multi_light.rs 0-byte；`rurix-render` 维持 `#![forbid(unsafe_code)]`。
- 阈值零手写：无偏 3σ + 方差收益对照均程序产 measured。

## 3. 终态程序

M-a~M-d 真跑 evidence 落档后本 RFC 终态 = approved-implemented（reservoir host 参考臂）+ SER 两半重判记录 + RD-040 分项闭集 + RD-034 复查记录字面；争议时按只追加程序重判。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | G21.1 起草；对抗评审后 Agent Approved。 |
