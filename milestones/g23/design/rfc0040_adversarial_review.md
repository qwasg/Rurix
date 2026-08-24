<!-- Assisted-by: Cursor Agent（G23.1 治理波） -->
# RFC-0040 对抗评审（D-4xx）

| Finding | Severity | Disposition |
|---|---|---|
| F1 采纳三件的「需求证据」判据可被主观化 | high | accepted — 需求证据面收窄为机器可核三类：5.6 独有 API 被生产代码引用 / 5.3 已知缺陷被生产 workload 命中 / A/B measured 性能差超带；三类全空 ⇒ maintain-5.3 唯一合法 |
| F2 sys56 构建新鲜真跑用 check 而非 test | medium | accepted — cargo check 验证 FFI 绑定与 vendor cmake 链完整性（本期判据 = 评估臂存续新鲜）；A/B 行为面已有 g9_m125 绿件（两臂诚实登记 checks 闭集），重跑归禁 --gate 纪律 |
| F3 M127 两半搜索可漏检 | medium | accepted — 搜索面闭集登记（corpus 目录模式 + residual 消费 token 模式）入 evidence detail；争议时只追加扩面重判 |
| F4 研究轨「观察存续」永续化风险 | low | accepted — 每期处置窗强制逐轨重判留痕（本期 M-c）；关闭条件字面 = 上游项目终止/被采纳两态 |
| F5 Rapier 分项转引 M126 而不重测 | low | accepted — M126 为 measured_local A/B 在案（40400ns vs 197900ns）；条件字面「快路径被真实 workload 采用时」未变 ⇒ 转引合法，重测触发条件未命中 |
