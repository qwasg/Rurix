# UC-05 RHI 不变量对照报告(EI1.3,RXS-0264;RFC-0014 §4.B Part B)

> **historical counters unavailable in-repo, non-reproducible, no fabricated figures.**
> 对照口径(documented_historical,AGENTS 硬规则 3;redline 评审 F3 钉死):上一项目代码与
> H01~H07 交接档**不在仓库**(已核实事实,EI1_PLAN R3)。本报告的「上一项目靠运行期 Python
> 计数器事后观测」= **无数字的定性历史陈述**(纸面对照)——**不可复跑 A/B、零杜撰 Python
> 数字**。Rurix 侧证据全 measured / ci_checked;I9 / I10 的数值面归 device EI1.4 落地。

## 1. 核心论点

I1~I8 这组不变量,上一项目靠**运行期 Python 计数器事后捕获**(部分漏到生产);Rurix 由
**类型系统 / 图装配期 100% 确定性拦截**(编译期即不可构造违例,或 submit 装配期 host 侧确定性
strict 拒 + rxrt_trap 终止)。裁决 1 划界(消 EI1_CONTRACT §1「I1~I10」vs 门「I1~I8」不一致):

- **I1~I8 = 100% 确定性检测项**(编译期 **或** 装配期确定性,入门 G-EI1-3 / 步骤 73,漏拦即红);
- **I9~I10 = 仅报告 / 观测对照项**(本质动态,不可静态拦截,入 G-EI1-5,`documented_historical`)。

对照上一项目**运行期概率性计数器可漏**:Rurix 侧 I1~I8 无运行期概率窗口——三档全为确定性
(编译期 typeck / 装配期 host 确定性拦 / 库单测已证机制)。

## 2. 三档划界(裁决 1 措辞;诚实收窄)

| 档 | 判据 | 触点 | I |
|---|---|---|---|
| **编译期** | typeck / `--emit=check` 即拦(违例不可构造) | 编译期诊断 | I1, I2, I6, I7, I8 |
| **装配期(图装配期)** | `submit()` 时 host 侧确定性拦(`--emit=check` CLEAN;submit 确定性 rxrt_trap,pre-dispatch) | 装配期库层状态值 Err + 终止 | I3, I5 |
| **lib_tested** | 机制由 rhi.rs 库单测证(纯 host 无 GPU);`.rx` 反射喂入随 EI1.4 | 库层状态值 Err | I4 |
| **report_only** | 运行期观测对照(不可静态拦截) | device measured(EI1.4) | I9, I10 |

> **I3/I5 诚实标注**:装配期 = 图装配期(`submit()` 时确定性拦),`--emit=check` 不拦但 submit
> 时确定性 `rxrt_trap`。装配期确定性 ≠ 运行期概率性——纯 host、pre-dispatch、无需 GPU 的库层判定
> (rhi.rs `rejects_read_before_write_i3` / `rejects_write_write_conflict_i5` 库单测为纯 host 见证)。
> **I4 诚实收窄**:I4 机制(rhi.rs `with_reflection` 声明-反射相等核验)已实现 + 库测
> (`rejects_reflection_mismatch_i4`);`.rx` 编译器反射喂入(pass 绑 kernel)与 kernel 绑定 / compute
> dispatch 天然耦合,随 **EI1.4**(device 真跑)落地——EI1.3 **不宣称** I4 `.rx` 路 ci_checked。

## 3. 逐不变量对照(矩阵 ↔ 语料 ↔ 报告三方一致;步骤 73 机核)

| # | 不变量 | 档 | 条款 | 诊断 | 语料 / 库测 | 证据级 |
|---|---|---|---|---|---|---|
| I1 | resource use-after-free | 编译期 | RXS-0259 | RX4001 | `conformance/uc05/reject/res_use_after_move.rx` | ci_checked |
| I2 | resource double-free | 编译期 | RXS-0259 | RX4001 | `conformance/uc05/reject/res_double_move.rx` | ci_checked |
| I3 | pass 依赖环 | 装配期 | RXS-0258 | 库层 Structure(镜像 RX6029) | `conformance/uc05/assembly/graph_cycle.rx` + rhi.rs `rejects_read_before_write_i3` | ci_checked |
| I4 | 未声明访问 | lib_tested | RXS-0257 | 库层 ReflectionMismatch(镜像 RX6030) | rhi.rs `rejects_reflection_mismatch_i4`(.rx 接线 EI1.4) | lib_tested(EI1.3) / .rx_wiring:EI1.4 |
| I5 | 写写冲突 | 装配期 | RXS-0258 | 库层 Structure(镜像 RX6029) | `conformance/uc05/assembly/graph_write_write.rx` + rhi.rs `rejects_write_write_conflict_i5` | ci_checked |
| I6 | 1-submit 二次 submit | 编译期 | RXS-0260 | RX4001 | `conformance/uc05/reject/rhi_double_submit.rx` | ci_checked |
| I7 | 跨 brand 资源误用 | 编译期 | RXS-0256 | RX3006 | `conformance/uc05/reject/rhi_cross_brand.rx` | ci_checked |
| I8 | RHI 着色合法性 | 编译期 | RXS-0256 | RX3015 | `conformance/uc05/reject/rhi_in_kernel.rx` | ci_checked |
| I9 | compute pass 数值正确性 | report_only | RXS-0263 | —(无诊断码) | device measured(EI1.4);Python 侧无数字定性陈述 | report_only |
| I10 | transient 峰值 / 生命周期 | report_only | RXS-0263 | —(无诊断码) | device measured(EI1.4);Python 侧无数字定性陈述 | report_only |

## 4. 生成与机核

- 矩阵 json:`evidence/uc05_invariant_matrix.json`(schema `milestones/ei1/uc05_invariant_matrix_schema.json`
  硬门:invariant 字段全 string/null,**任何 number 值即 schema 违例** → by-construction 封死 I9/I10
  杜撰数字窗口)。
- 三方一致性(步骤 73,`ci/uc05_invariant_gate.py`):矩阵 json ↔ reject/assembly 语料实存 ↔ 本报告
  逐项对齐(条款号 / 语料路径 / 诊断码),任一漂移即红。
- I1~I8 逐条 reject 拦截:编译期组由 `cargo test -p rurixc --test uc05_corpus`(步骤 72 host 恒跑)+
  步骤 73 静态核;装配期组由 `ci/uc05_rhi_smoke.py`(步骤 72)编译成 EXE 真跑 red-green 见证(device
  段,需 GPU 运行 Context)+ rhi.rs 库单测纯 host 见证。
