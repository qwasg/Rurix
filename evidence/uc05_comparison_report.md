# UC-05 RHI 不变量对照报告(EI1.3 + EI1.4,RXS-0264;RFC-0014 §4.B Part B)

> **historical counters unavailable in-repo, non-reproducible, no fabricated figures.**
> 对照口径(documented_historical,AGENTS 硬规则 3;redline 评审 F3 钉死):上一项目代码与
> H01~H07 交接档**不在仓库**(已核实事实,EI1_PLAN R3)。本报告的「上一项目靠运行期 Python
> 计数器事后观测」= **无数字的定性历史陈述**(纸面对照)——**不可复跑 A/B、零杜撰 Python
> 数字**。Rurix 侧证据全 measured / ci_checked;I9 数值面已于 **EI1.4** device 落地(真派发 +
> 真 D2H + host 参考比对),I10 的执行期峰值计数器**仍未实现**(见 §2 诚实标注)。

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
| **装配期(图装配期)** | `submit()` 时 host 侧确定性拦(`--emit=check` CLEAN;submit 确定性 rxrt_trap,pre-dispatch) | 装配期库层状态值 Err + 终止 | I3, **I4**, I5 |
| **lib_tested** | 机制由库单测证但 `.rx` 未接线 | 库层状态值 Err | —(EI1.4 起空集) |
| **report_only** | 运行期观测对照(不可静态拦截) | device measured | I9, I10 |

> **I3/I4/I5 诚实标注**:装配期 = 图装配期(`submit()` 时确定性拦),`--emit=check` 不拦但 submit
> 时确定性 `rxrt_trap`。装配期确定性 ≠ 运行期概率性——纯 host、pre-dispatch、无需 GPU 的库层判定
> (rhi.rs `rejects_read_before_write_i3` / `rejects_write_write_conflict_i5` /
> `rejects_reflection_mismatch_i4` 库单测为纯 host 见证)。
>
> **I4 兑现(EI1.3 收窄 → EI1.4 交付)**:EI1.3 期 I4 机制(rhi.rs `with_reflection` 声明-反射相等
> 核验)已实现 + 库测,但 `.rx` 反射喂入与 kernel 绑定 / compute dispatch 天然耦合,故诚实收窄为
> `lib_tested`、**不宣称** `.rx` 路 ci_checked。**EI1.4 已接线**:`rhi.pass(kernel, GridDim(..),
> BlockDim(..), (args..))` 绑 kernel 后,编译器自 kernel 签名与绑定实参**静态提取反射集**(实参中
> 的 `Res`,由 launch_check 核对其确落在 `View`/`ViewMut` 形参位)→ marshalling kind-2 槽 →
> `rxrt_rhi_bind` → `PassSpec::with_reflection`;`conformance/uc05/assembly/pass_undeclared_read.rx`
> **真触发** seal 的 I4 分支(声明 ⊊ 反射 → 库层 ReflectionMismatch → rxrt_trap)。故 I4 升入
> **装配期**档、证据级 `ci_checked`。
>
> **I9 兑现 / I10 未兑现(诚实分列)**:I9 = `apps/uc05-rhi` demo 两 pass 真派发 → readback 真
> D2H → host 侧求和 vs 闭式参考精确比对(见证 token `UC05_SUM` / `UC05_REF`,相等才打
> `UC05_RHI_OK`),**已 device measured**;但仍留 `report_only`——数值正确性本质动态(单机单驱动
> 一次观测,非全域证明)。I10 = 每个 transient `Res` 为一笔真设备分配、生命期 = 图生命期,故实际
> 峰值**恒等于**声明容量;「峰值 <= 声明容量」平凡成立而**非因 aliasing/复用收紧**——transient
> 资源别名复用与执行期峰值计数器**均未实现**,随后续期。

## 3. 逐不变量对照(矩阵 ↔ 语料 ↔ 报告三方一致;步骤 73 机核)

| # | 不变量 | 档 | 条款 | 诊断 | 语料 / 库测 | 证据级 |
|---|---|---|---|---|---|---|
| I1 | resource use-after-free | 编译期 | RXS-0259 | RX4001 | `conformance/uc05/reject/res_use_after_move.rx` | ci_checked |
| I2 | resource double-free | 编译期 | RXS-0259 | RX4001 | `conformance/uc05/reject/res_double_move.rx` | ci_checked |
| I3 | pass 依赖环 | 装配期 | RXS-0258 | 库层 Structure(镜像 RX6029) | `conformance/uc05/assembly/graph_cycle.rx` + rhi.rs `rejects_read_before_write_i3` | ci_checked |
| I4 | 未声明访问 | 装配期 | RXS-0257 | 库层 ReflectionMismatch(镜像 RX6030) | `conformance/uc05/assembly/pass_undeclared_read.rx` + rhi.rs `rejects_reflection_mismatch_i4` | ci_checked(EI1.4 接线兑现) |
| I5 | 写写冲突 | 装配期 | RXS-0258 | 库层 Structure(镜像 RX6029) | `conformance/uc05/assembly/graph_write_write.rx` + rhi.rs `rejects_write_write_conflict_i5` | ci_checked |
| I6 | 1-submit 二次 submit | 编译期 | RXS-0260 | RX4001 | `conformance/uc05/reject/rhi_double_submit.rx` | ci_checked |
| I7 | 跨 brand 资源误用 | 编译期 | RXS-0256 | RX3006 | `conformance/uc05/reject/rhi_cross_brand.rx` | ci_checked |
| I8 | RHI 着色合法性 | 编译期 | RXS-0256 | RX3015 | `conformance/uc05/reject/rhi_in_kernel.rx` | ci_checked |
| I9 | compute pass 数值正确性 | report_only | RXS-0263 | —(无诊断码) | `apps/uc05-rhi/src/demo.rx` device measured(EI1.4:`UC05_SUM` == `UC05_REF`);Python 侧无数字定性陈述 | report_only(measured_local) |
| I10 | transient 峰值 / 生命周期 | report_only | RXS-0263 | —(无诊断码) | host 容量记账 measured;**device 峰值计数器未实现**(诚实标注,见 §2);Python 侧无数字定性陈述 | report_only(部分未兑现) |

## 4. 生成与机核

- 矩阵 json:`evidence/uc05_invariant_matrix.json`(schema `milestones/ei1/uc05_invariant_matrix_schema.json`
  硬门:invariant 字段全 string/null,**任何 number 值即 schema 违例** → by-construction 封死 I9/I10
  杜撰数字窗口)。
- 三方一致性(步骤 73,`ci/uc05_invariant_gate.py`):矩阵 json ↔ reject/assembly 语料实存 ↔ 本报告
  逐项对齐(条款号 / 语料路径 / 诊断码),任一漂移即红。
- I1~I8 逐条 reject 拦截:编译期组由 `cargo test -p rurixc --test uc05_corpus`(步骤 72 host 恒跑)+
  步骤 73 静态核;装配期组由 `ci/uc05_rhi_smoke.py`(步骤 72)编译成 EXE 真跑 red-green 见证(device
  段,需 GPU 运行 Context)+ rhi.rs 库单测纯 host 见证。
- **报告面一致性(步骤 75,`ci/uc05_report_check.py`,纯 host 恒跑)**:矩阵 schema 校验 + 矩阵 ↔
  `conformance/uc05/{reject,assembly}` 磁盘语料集**双向**互查(漏登 / 悬空各自判红)+ 本报告 §3 表
  ↔ 矩阵 I 集合全等与逐条条款 / 档位一致 + `documented_historical` **字面核**(I9/I10 对照侧陈述段
  零数字字面)+ §5 采纳判据数字 ↔ evidence `results.trimmed_mean` 逐位相等。内建 red_self_test。

## 5. 采纳判据(RXS-0265)—— measured

C ABI 成熟腿由 **G-EI1-4**(EI1.2 `--emit=dll` + 生成头 + EI1.4 C++/D3D12 宿主真跑)兑现;本节记
**增量 check <5s 双口径**的 measured 结果。采集器 `ci/uc05_check_bench.py`(操作者工具,**evidence
面不进 CI 硬门** —— 计时波动,EA1 冷启动先例;SKIP 不充绿),BENCH_PROTOCOL §3 三次进程级独立 trial
→ trimmed mean(0.2);计时 = 进程端到端墙钟,**纯 host、零 GPU**(`--emit=check` 不 codegen、不 link、
不建 CUDA Context,故 §2.1 锁频规程不适用,理由落 evidence `environment.clock_lock_applicability`)。

| 预算条目 | 口径 | trimmed_mean (ms) | 逐 trial (ms) | 阈 (ms) | 余量 | 证据 |
|---|---|---|---|---|---|---|
| `ei1.bench.uc05_check_cold_ms` | 冷全检(全新进程 + 全新临时路径的 exe 与包源,零预热,含磁盘 `mod` 解析) | 107.468 | 98.213 / 85.543 / 138.649 | 5000 | ~46× | `evidence/uc05_check_cold_20260720.json` |
| `ei1.bench.uc05_check_warm_ms` | 预热后全包重跑(**全量重析,非 LSP 增量**;每 trial 预热 10 次后 5 次计时取中位数) | 18.033 | 18.955 / 17.552 / 17.590 | 5000 | ~277× | `evidence/uc05_check_warm_20260720.json` |

两口径均**远低于** 5000ms 阈:采纳判据的风险从来在**口径**而非数值(EI1_PLAN R4)。故口径逐条钉死:

- **warm ≠ LSP 增量**(RXS-0265 锁,RFC-0014 §9.1 `I-EI1-IMPL-04` disposition):现 tooling session
  (`src/rurixc/src/tooling/session.rs::analyze`)只对单个内存文件 lex + parse + check_crate、**无
  `mod` 解析 / 磁盘加载**,无法「增量」检全包,故本口径**不走** didChange → publishDiagnostics 路,
  措辞去「增量」。冷 / 热差 = 进程启动 + 文件 IO + 镜像装载缓存,**非**编译器内部增量缓存(rurixc
  无跨进程增量,07 §9)。
- **冷口径控制到什么**:全新进程 + 该路径首读(exe 与包源各拷全新临时路径);**未控制到**操作系统
  standby 缓存中刚写入的字节(Windows 无管理员级 drop-cache)——如实声明,不假装是绝对冷启。

> **诚实缺口(不折算进上表数字)**:`apps/uc05-rhi/src/embed.rx` 是 `#[export(c)]` cdylib 根、**无
> `main`**,而 `--emit=check` **不在** driver.rs `device_emit` 免 `main` 豁免集内(RXS-0252 的导出根
> 免 `main` 只覆盖 `--emit=dll`)→ `rurixc apps/uc05-rhi/src/embed.rx --emit=check` 实测 **exit 1 /
> `error[RX6002]: no \`main\` function found for executable target`**(真实探测,记于两份 evidence 的
> `uncheckable_roots[].probe`,schema `required` 强制披露)。故「全包 check」当前**实测覆盖
> `demo.rx` + `graph.rx`**(`mod graph` 磁盘解析真发生),**不覆盖 `embed.rx`**。该缺口是
> `--emit=check` 与 `--emit=dll` 的**豁免集不对称**,属编译器面口径缺口、非本期 bench 取数问题;
> 处置(扩 `--emit=check` 导出根豁免,并令 move/borrow 覆盖导出体)归主循环判档,本节只如实登记。
