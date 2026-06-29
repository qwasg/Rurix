# M3 执行计划 — 小里程碑分解

> 所属契约:[M3_CONTRACT.md](M3_CONTRACT.md)
> 版本:v1.0(2026-06-12)
> 粒度依据:11 §7(1–2 周小里程碑 + 阶段两级结构);本计划是工作分解,验收以契约 §4 为准,本文不重定义成功。

---

## 0. 总览与依赖

```mermaid
flowchart LR
    m31[M3.1 desugar收口与TBIR] --> m32[M3.2 move/init与drop]
    m32 --> m33[M3.3 NLL借用检查]
    m33 --> m34[M3.4 const_eval与预算回填]
```

| 小里程碑 | 时长(估) | 交付物映射 | 阻塞关系 |
|---|---|---|---|
| M3.1 | ~2 周 | D-M3-1 / D-M3-2(borrow 条款先行部分) | 依赖 M2 typeck/MIR/codegen 闭环(已交付,`m2-closed`) |
| M3.2 | ~2 周 | D-M3-3 | 依赖 M3.1(数据流跑在 TBIR 重排后定型的 MIR 上;drop scope 来自 TBIR) |
| M3.3 | ~2–3 周 | D-M3-4 / D-M3-2(borrow 条款主体) | 依赖 M3.2(NLL 流程中 move/init 数据流是前置 pass,07 §4) |
| M3.4 | ~1–2 周 | D-M3-5 / D-M3-6 | const eval 依赖 MIR 定型(M3.1/M3.2);预算回填依赖管线全量存在(回填口径含 borrowck 耗时) |

时长为 `estimated`(M2 实际节奏可作弱参考),仅作排程参考,不构成验收承诺。

## 1. M3.1 — desugar 收口与 TBIR 窄门(~2 周)

| # | 任务 | 验证方式 |
|---|---|---|
| 1 | guardrail 基准切换:`ci/check_guardrails.py` 本地/push 回退基准 `m1-closed → m2-closed`(PR 路径仍以 GITHUB_BASE_REF 为准),双基准核对后落地,留痕 [CI_GATES.md](CI_GATES.md) 修订行 | `py -3 ci/check_guardrails.py m2-closed` PASS |
| 2 | spec 条款先行:drop scope/模式穷尽性/desugar 语义入 `spec/borrow.md` 首批条款(RXS-0048 起)——**条款 PR 先于实现 PR** | spec 档位标记 guardrail + 修订行 |
| 3 | lang-item 最小面:Iterator/Result 的编译器内建识别(仅 desugar 所需,不开放用户自定义 lang-item);`for` → loop+match、`?` → match desugar 落地(M2_PLAN v1.1/v1.2 推迟项收口) | 单测(desugar 前后语义快照)+ conformance 正例 |
| 4 | TBIR 定义与管线重排:HIR→TBIR(模式穷尽性/方法糖显式化/drop scope)→MIR;TBIR 构造 MIR 后即释放(D-202 峰值内存纪律);MIR lowering"暂不支持"诊断面收窄并留痕剩余清单 | 单测 + hello-world 冒烟(CI 步骤 12/13/14)持续绿 |
| 5 | 模式穷尽性检查(match 非穷尽 → 诊断;错误码段位按 07 §5 归 2xxx 类型检查段或 4xxx,分配时留痕) | 单测 + UI snapshot |

**出口判据**:desugar 后 conformance 正例(含 for/`?` 用例)全管线 0 诊断且 hello-world 冒烟不回归;TBIR 在管线中可观测(self-profile 阶段计数器扩列或等价证据)。

## 2. M3.2 — move/init 数据流与 drop 语义(~2 周)

| # | 任务 | 验证方式 |
|---|---|---|
| 1 | spec 条款:move/初始化/drop 时点语义条款追加 `spec/borrow.md`(affine 闭环,05 §3.1/§4) | 同 M3.1 第 2 项 |
| 2 | MIR 数据流框架(前向/后向通用骨架,后续 borrowck/M4 扩展 pass 复用) | 单测(小 CFG 收敛性) |
| 3 | move/init 分析:use-after-move / use-before-init 检测;4xxx 错误码首批分配 + `borrowck.*` message-key(registry 只追加) | `py -3 ci/check_schemas.py` PASS + UI snapshot |
| 4 | drop elaboration:drop scope(TBIR)→ MIR drop 语句;条件初始化的 drop flag;Drop 类型的释放点确定性 | 单测(drop 顺序快照)+ conformance 正例 |
| 5 | MIR 文本 golden guardrail 化预评估:MIR 形态此阶段定型,准备 golden 基线与核对脚本(激活在 M3.3/M3.4,经红绿验证) | 评估记录入 CI_GATES 修订行 |

**出口判据**:move/init 反例(契约 §4 类别 1/2)全拦截;conformance 正例 0 诊断;hello-world 冒烟不回归。

## 3. M3.3 — NLL 借用检查(~2–3 周)

| # | 任务 | 验证方式 |
|---|---|---|
| 1 | spec 条款:借用/生命周期条款主体(`spec/borrow.md`:共享/独占借用规则、NLL 作用域、生命周期参数与 RXS-0041 预留的"子类型仅限生命周期"条款化) | 同 M3.1 第 2 项 |
| 2 | region 推断管线(D-204 流程照搬 r1):region 变量替换 → MIR type check 收集 constraints → region inference → 逐点 in-scope borrows → 报错 walk;**保守先行**,精度问题登记不阻塞 | 单测(逐 pass 中间产物快照) |
| 3 | `mir_borrowck(body_id)` query 化(D-203:经 query context 互访,memo 计量沿用) | 单测(memo 命中/纯函数纪律) |
| 4 | `conformance/borrowck/` 语料:`reject/<category>/`(契约 §4 七类)+ `accept/`(正例);CI 步骤 15 = borrowck conformance 批跑接入([CI_GATES.md](CI_GATES.md) §2),红绿真跑 | G-M3-1 计数 + CI run 输出 |
| 5 | 黄金路径 3:`tests/ui/borrowck/` snapshot ≥10(4xxx 错误码,经 bless 审批流程) | G-M3-2 计数 + CI 绿 |
| 6 | MIR golden guardrail 激活(M3.2 预评估的落地):基线入库 + 核对入 CI,真实红绿验证 | guardrail 红绿 run URL 留痕 |

**出口判据**:契约 G-M3-1 + G-M3-2 达成;borrowck 对全部既有 conformance 正例 0 诊断。

## 4. M3.4 — const eval、预算回填与 close-out(~1–2 周)

| # | 任务 | 验证方式 |
|---|---|---|
| 1 | spec 条款:`spec/consteval.md`(const fn 子集边界/const 泛型求值规则/求值失败语义) | 同 M3.1 第 2 项 |
| 2 | const eval MIR 解释器(算术/分支/循环/数组构造,05 §9);5xxx 错误码首批(求值溢出/越界/非 const 操作)+ `tests/ui/consteval/` snapshot | 单测 + UI snapshot |
| 3 | const 泛型接入:类型系统(数组长度/const 参数)+ 单态化收集;`conformance/consteval/` 真跑程序;CI 步骤 16 = const eval 冒烟接入,红绿真跑 | G-M3-4 + CI run 输出 |
| 4 | 预算实测回填(G-M3-3):冷编译 hello-world / 全量 check 延迟各三次进程级独立运行(trimmed mean,bench/stats.py),证据 JSON 入 evidence/;[../m2/m2_budget.json](../m2/m2_budget.json) estimated → measured_local(阈值数值经自主批准),revision_log 追加 | `py -3 ci/budget_eval.py --strict` 零 estimated 残留 |
| 5 | traceability 矩阵再生成(`ci/trace_matrix.py`,含 borrow/consteval 新条款)+ 全锚定核对 | G-M3-5 |
| 6 | M3 close-out 草拟(验收记录 + guardrail 输出 + 红绿 run URL 追加契约 §8;关闭判定人工) | guardrail 全过 |

**出口判据**:契约 G-M3-3 / G-M3-4 / G-M3-5 达成,close-out 草案就绪。

## 5. 风险提示(引用,不另建登记)

- **lang-item 范围蔓延**:desugar 只需要 Iterator/Result 的最小内建识别面;凡"顺手做泛型 trait 求解完整化/用户自定义 lang-item"的诉求一律走 M4+ 或 RFC(M2_PLAN v1.2 的最小化口径延续),不临时扩 trait 系统。
- **region 推断复杂度**:NLL 是 M3 最大的算法风险点。对策:保守先行(07 §4"先正确性后诊断";精度不足产生的误报登记为已知限制,不阻塞关闭)+ 逐 pass 中间产物快照单测(防黑盒化);Polonius 方向被 D-204 永久 gating,不要提案。
- **MIR 形态变动的回归面**:TBIR 重排与 drop elaboration 会动 M2 已交付的 codegen 输入。hello-world 冒烟(CI 步骤 12/13/14)是常驻回归网,每个 M3.x PR 必须保持绿;MIR golden guardrail 激活后(M3.3)形态变更显式化。
- **预算回填的测量噪声**:编译耗时受 Windows 文件系统/杀软扫描影响大;沿用 BENCH_PROTOCOL 的环境画像纪律,三次进程级独立运行 + trimmed mean,阈值设定留余量并经自主批准;回填是 14 §3 硬约束(占位 ≤2 里程碑到期),M3.4 内必须完成,不得顺延。
- **错误码段位纪律**:4xxx/5xxx 分配制递增、含义冻结(10 §6);模式穷尽性错误的段位归属(2xxx vs 4xxx)在分配 PR 中留痕裁决。

## 6. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-12 | 初版 |
| v1.1 | 2026-06-12 | §1 任务 4 留痕:TBIR 重排落地后 RX6001"暂不支持"剩余清单 = closure / 索引 / 数组与 repeat / 独立 range 表达式 / fn 指针间接调用 / 裸 fn·const 值引用 / 带值 break / 解构 `let` / 区间·slice·const 模式 / 字符串字面量模式 / 泛型 extern fn。两项实现取舍登记为已知限制(随 M4+ 或诊断打磨期评估):enum 扁平布局(tag + 全变体载荷顺排不重叠,空间换实现简单)、match or-pattern 臂体按候选重复 lowering(不共享块) |
