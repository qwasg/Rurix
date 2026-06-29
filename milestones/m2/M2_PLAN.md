# M2 执行计划 — 小里程碑分解

> 所属契约:[M2_CONTRACT.md](M2_CONTRACT.md)
> 版本:v1.0(2026-06-11)
> 粒度依据:11 §7(1–2 周小里程碑 + 阶段两级结构);本计划是工作分解,验收以契约 §4 为准,本文不重定义成功。

---

## 0. 总览与依赖

```mermaid
flowchart LR
    m21[M2.1 名称解析与HIR] --> m22[M2.2 query与typeck]
    m22 --> m23[M2.3 MIR与host codegen]
    m23 --> m24[M2.4 self-profile与close-out]
```

| 小里程碑 | 时长(估) | 交付物映射 | 阻塞关系 |
|---|---|---|---|
| M2.1 | ~2 周 | D-M2-1 / D-M2-2(names 部分) | 依赖 M1 parser/AST(已交付) |
| M2.2 | ~2–3 周 | D-M2-2(types 部分)/ D-M2-3 / D-M2-4 | 依赖 M2.1(typeck 消费解析后的 HIR;query 承载语义 API) |
| M2.3 | ~2–3 周 | D-M2-5 | 依赖 M2.2(codegen 消费已定型的 MIR 输入 = 类型完备的 HIR body) |
| M2.4 | ~1–2 周 | D-M2-6 | 依赖 M2.3(端到端管线存在才有阶段计时对象) |

时长为 `estimated`(无历史数据),仅作排程参考,不构成验收承诺。

## 1. M2.1 — 名称解析与 HIR(~2 周)

| # | 任务 | 验证方式 |
|---|---|---|
| 1 | spec 名称/模块语义条款首批(`spec/names.md`,RXS-0032 起:作用域规则/可见性 D-112/use 解析/重名裁决)——**规范先行,条款 PR 先于实现 PR** | spec 档位标记 guardrail + 修订行 |
| 2 | DefId/符号表 + 名称解析(模块树/作用域栈/`pub`/`pub(package)`/use 别名;上下文关键字 `global` 等在类型位置的重分类预留 M4 钩子) | 单测 + 条款号引用核对 |
| 3 | HIR 定义与 lowering:item/body 分离(D-202 增量依赖边界);desugar:`for` → loop+match 形态、`?` → match 形态(07 §1) | 单测(desugar 前后语义快照) |
| 4 | 错误码 1xxx 首批分配(未解析名称/重复定义/可见性违例)+ `resolve.*` message-key;registry/error_codes.json 只追加 | `py -3 ci/check_schemas.py` PASS |

**出口判据**:conformance 语料(正例)全量通过名称解析 0 诊断;新增条款全部锚定。

## 2. M2.2 — query 骨架与 typeck host 子集(~2–3 周)

| # | 任务 | 验证方式 |
|---|---|---|
| 1 | spec 类型语义条款首批(`spec/types.md`:原生类型/函数签名全标注规则/HM 局部推断范围/trait 单态化子集 D-104/泛型 D-111) | 同 M2.1 第 1 项 |
| 2 | query 骨架(D-203 第一天形态):query context + 进程内 memo;`type_of(def_id)`/`hir_body(body_id)` 等纯函数 API;provider 只经 context 互访,无全局可变状态 | 单测(memo 命中计数/重入纪律) |
| 3 | 类型收集 → HIR body 内 HM 局部推断 → 类型检查(host 子集:函数/struct/enum/泛型单态化雏形;trait 求解 = 单态化导向简化版,07 §3) | 单测 + conformance 正例 0 诊断跑批 |
| 4 | 错误码 2xxx 首批(类型不匹配/未知字段/实参数目/trait 未实现)+ 黄金路径 2 snapshot ≥10 入 `tests/ui/typeck/`(经 bless 审批流程,M1.4 guardrail) | G-M2-3 计数 + CI 绿 |

**出口判据**:契约 G-M2-3 达成;typeck 对语料正例 0 诊断。

## 3. M2.3 — MIR 雏形与 host codegen 闭环(~2–3 周)

| # | 任务 | 验证方式 |
|---|---|---|
| 1 | MIR 雏形:CFG 化、显式类型、locals/语句/终结子;单态化收集(D-111 全单态化) | 单测(hello-world body 的 MIR 文本快照) |
| 2 | LLVM 接入选型留痕:pin 22.1.x(D-205),绑定通道(inkwell/llvm-sys/文本 IR + llc)取舍记录入本文件修订行——**选型是 Mini-RFC 级动作,先留痕再实现** | 选型记录 + 自主批准 |
| 3 | host codegen:MIR → LLVM IR → x86-64 COFF .obj(Microsoft x64 ABI)→ link.exe → EXE;CodeView/PDB(D-209/D-237) | G-M2-1(运行验证 + PDB 存在) |
| 4 | cdb 断点脚本(`bp` 源行断点 + `g` + 栈打印),输出留痕 | G-M2-2 |
| 5 | CI 步骤 12/13 接入(hello-world 冒烟 + 断点核对,[CI_GATES.md](CI_GATES.md) §2) | CI run 输出 |

**出口判据**:契约 G-M2-1 + G-M2-2 达成。

## 4. M2.4 — self-profile、预算布点与 close-out(~1–2 周)

| # | 任务 | 验证方式 |
|---|---|---|
| 1 | self-profile:query 级计时 + 阶段计数器(parse/resolve/typeck/mir/codegen/link),机器可解析输出(07 §6) | G-M2-4(各计数器非零) |
| 2 | 编译性能预算占位核对:[m2_budget.json](m2_budget.json) `estimated` 条目的 skip_reason 与 M3 回填承接复核(占位存活 ≤2 里程碑,14 §3) | `py -3 ci/budget_eval.py` 输出 |
| 3 | traceability 矩阵再生成(`ci/trace_matrix.py`,含 names/types 新条款)+ 全锚定核对 | G-M2-5 |
| 4 | M2 close-out 草拟(验收记录 + guardrail 输出 + cdb 留痕追加契约 §8;关闭判定人工) | guardrail 全过 |

**出口判据**:契约 G-M2-4 / G-M2-5 达成,close-out 草案就绪。

## 5. 风险提示(引用,不另建登记)

- **LLVM 工具链体量与 Windows 链路**:link.exe 依赖 VS 构建环境(vcvars),CI runner 须预置;LLVM 22.1.x 的获取/构建/缓存策略在 M2.3 选型留痕时一并裁决——若链路超预算,退路是文本 LLVM IR + 外部 llc(形态自由,不破坏 D-205 的 pin 承诺)。
- **类型系统范围蔓延**:host 子集以 conformance 正例集为唯一锁定面(05 全文比 M2 范围大);凡超出"函数/struct/enum/泛型单态化雏形"的诉求一律走 M3+ 或 RFC,不临时扩 typeck。
- **M1 关闭顺序**:M1 的人工红绿验证/终审(M1_CONTRACT §8.2)与 M2.1 可并行;但 `m1-closed` tag 未打出前,guardrail 基准维持 PR base / `m0-baseline`(契约 §5)。
- **错误码段位纪律**:1xxx/2xxx 分配制递增、含义冻结(10 §6);诊断措辞策略变更属 Mini-RFC 档。

## 6. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-11 | 初版 |
| v1.1 | 2026-06-11 | M2.1 实施留痕(Direct)两项:(1)§1 出口判据作用面调整——"conformance 语料(正例)全量 0 诊断"改为新建 `conformance/resolve/` 语义正例集(自包含程序)全量 0 诊断,`conformance/syntax/` 维持 parse 门(其样例为含草图引用的语法正例,且 names_duplicates.rx 是故意的 resolve 反例);(2)§1 任务 3 的 desugar(for/`?`)时点推迟至 M2.2 与类型系统同步(依赖 Iterator/Result lang-item),HIR 暂保留 For/Try 一等节点 |
| v1.2 | 2026-06-12 | M2.2 实施留痕(Direct)三项:(1)§2 任务 3 "trait 求解单态化导向简化版"最小化口径——内建运算符直接按原生类型规则检查(不经运算符 trait)、泛型 bound 仅记录不求解(RXS-0045 条款化)、方法查找仅 inherent impl,完整求解器随 M2.3+ 评估;(2)desugar(for/`?`)继续推迟(v1.1 项延续,typeck 对应构造容忍);(3)UI 通道阶段化:前一阶段有 error 即停(parse→resolve→typeck),防级联污染 snapshot,与 rustc 阶段中止策略对齐 |
| v1.3 | 2026-06-12 | M2.3 选型留痕(§3 任务 2,Mini-RFC 级,自主批准已取得):LLVM 绑定通道裁决为**文本 LLVM IR + 外部工具**(rurixc 产 `.ll`,经 pin 的 clang 22.1.x 编为 COFF .obj,link.exe 链接)。理由:(1)rurixc 维持零外部 Rust 依赖(M1.1 纪律);(2)inkwell 对 LLVM 22 支持滞后、llvm-sys 在 Windows 需自建 LLVM 开发库(官方预编译包不含 .lib),链路成本均超预算——即 §5 风险节预留退路的正式启用,不破坏 D-205 pin 承诺(pin 核对面 = 外部工具版本断言 22.1.x);(3)文本 IR 通道与绑定库解耦,季度评估升级(D-205)只动工具不动代码。环境留痕:LLVM 22.1.7(winget)+ WinDbg/cdb;in-process 绑定迁移待 M3+ 按需重评(届时另行 Mini-RFC)。附带口径:M2.3 codegen 作用面 = hello-world 闭环所需 host 子集,`for`/`?`/closure/`match` 等构造在 MIR lowering 给出明确"暂不支持"诊断(desugar 推迟项 v1.1/v1.2 延续,随 M3 收口) |
