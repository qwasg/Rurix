# M2 CI 门禁增量

> 所属契约:[M2_CONTRACT.md](M2_CONTRACT.md)
> 版本:v1.0(2026-06-11)
> 基线:[../m0/CI_GATES.md](../m0/CI_GATES.md) + [../m1/CI_GATES.md](../m1/CI_GATES.md)(全部沿用:runner 约定、PR Smoke 1–11 步、guardrail 八项含 M1.4 激活的 UI bless 审批);本文只规定 M2 期的**增量**。
> 铁律不变:任何新增门禁必须在真实 PR 上以真实失败/通过路径验证过(反 YAML-only,H06 D11.8-2)。

---

## 1. Runner

沿用 M0 §1(自托管 RTX 4070 Ti 开发机)。M2 新增步骤均为 CPU 任务,不占 GPU 队列;步骤 12/13 依赖 VS 构建环境(link.exe / cdb),runner 预置项随 M2.3 落地时在本文件修订行留痕。

## 2. PR Smoke 追加步骤(编号接 M1 §2 的 7–11)

| # | 步骤 | 失败即红 |
|---|---|---|
| 12 | hello-world 编译闭环冒烟:rurixc 全管线产出 EXE → 运行核对退出码/输出 → PDB 产物存在(契约 G-M2-1 通道;自 M2.3 存在起) | 是 |
| 13 | cdb 断点脚本核对:源行断点命中 + 栈打印,输出与基线比对(契约 G-M2-2 通道;自 M2.3 存在起) | 是 |

预算 evaluator(M0 步骤 6)自动合并加载 [m2_budget.json](m2_budget.json)(命名空间冲突即红)。**M2 期 PR Smoke 跑 normal 模式**:`m2.bench.*` 为 `estimated` 占位,SKIP + skip_reason 输出属预期(契约 §4 诚实声明);`--strict` 的全局零残留判定推迟到 M3 close-out(占位存活 ≤2 里程碑,14 §3)。M1 终审引用其 close-out 已留痕的 strict 输出(2026-06-11,见 [../m1/M1_CONTRACT.md](../m1/M1_CONTRACT.md) §8.1.2),不受本套件追加占位影响。

## 3. Nightly 追加

- 前端吞吐冒烟延续(lexer/parser 短协议,M1 期既有)。
- self-profile 输出归档:Nightly 编译 hello-world 一次,阶段计时 JSON 入归档目录(非证据,趋势参考;自 M2.4 存在起)。
- M0 GPU 基准回归继续跑(回归判定 M1 起已生效,BENCH_PROTOCOL §5)。
- Release 层仍不建(RD-001,承接 M8 不变)。

## 4. Guardrail(无新增激活项)

沿用 M0 五项 + M1 三项(spec 档位 / 错误码冻结 / UI bless,均已激活并经红绿验证或本地核对)。基准 ref:`m1-closed` tag 由人类随 M1 终审打出;落地前 `ci/check_guardrails.py` 仍以 PR base / `m0-baseline` 为基准,切换时在本表修订行留痕。

14 §2 常驻集其余项的 M2 期评估结论:

| 项 | 结论 |
|---|---|
| IR golden(MIR 文本快照) | M2.3 MIR 雏形以**单测快照**形态起步,正式 guardrail 化随 M3 MIR 定型评估 |
| stable API 快照 | M2 无 stable 面,不激活 |
| unsafe-audit 完整性 | rurixc 实现侧 `unsafe_code = deny` 维持;出现首个 unsafe 块时按 AGENTS 硬规则 9 激活 |
| NVIDIA 白名单 | device 路径 M4 起评估 |

## 5. 验证程序(对应契约 G-M2-1/G-M2-2 与步骤 12/13)

1. 步骤 12/13 落地后,构造一个**故意破坏 codegen 输出**(或篡改断点基线)的 PR → 必须红。
2. 修复后同 PR 转绿 → 合入;两次 run URL 随 close-out 归档。
3. close-out 附 cdb 命令与输出原文(契约 G-M2-2 真跑铁律)。

## 6. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-11 | 初版(M2 契约配套;步骤 12/13 为 M2.3 计划项,落地时回填实测命令) |
| v1.1 | 2026-06-12 | M2.3 落地回填:步骤 12 = `py -3 ci/hello_smoke.py compile-run`,步骤 13 = `py -3 ci/hello_smoke.py breakpoint`(均已入 pr-smoke.yml)。实测命令:rurixc 驱动 `conformance/syntax/hello_world.rx` → EXE+PDB(clang 22.1.7 + VS BuildTools link.exe);cdb 断点 = `bp `hello_world!hello_world.rx:6`; g; k; q`(基线不变量:Breakpoint 0 hit / hello_world!main / hello_world.rx @ 6),cdb 输出原文留痕 `evidence/cdb_hello_world_20260612.txt`。runner 预置项:LLVM 22.1.7(winget LLVM.LLVM)+ WinDbg(winget Microsoft.WinDbg,含 cdb)。§5 验证程序:脚本级红绿已本地真跑(篡改 EXE → breakpoint 红 exit 1;恢复 → 绿 exit 0);PR 级红绿(run URL)随本分支 PR 流程执行,URL 届时归档 close-out |
