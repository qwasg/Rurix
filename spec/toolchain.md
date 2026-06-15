# Rurix 语言规范 — 工具链语义(M6.1:rx CLI 总入口与核心子命令)

> 条款:RXS-0083 ~ RXS-0088(M6.1 rx CLI 子命令语义面首批)。体例见 [README.md](README.md)。
> 依据:07 §2 §6 §9(查询化与增量编译 D-203 / 编译性能预算 / LSP 与工具模式 D-210——单一前端,常驻 query 层);08 §4 §7(rx bench harness 工具化 / 开发者工具集 rx CLI D-239);milestones/m0/BENCH_PROTOCOL.md(基准协议 §2/§3);M6 契约 D-M6-1 / G-M6-3 / G-M6-4 / G-M6-5(spec 先行)。
> 本文为已选定决策(D-203/D-210/D-239)的初版条款化(档位 Direct);任何偏离 07/08 已锁定决策的语义动作须按 10 §3 升档。本文承载工具链语义条款,M6.2/M6.4 的包管理 `rurix.toml`/`rurix.lock` 格式条款与 LSP 能力面条款续写本文件(编号续号)。
> **M6.1 范围裁决(rx CLI 总入口 + 核心子命令优先)**:rx 经 rurixc query 层复用单一前端,不另起引擎(07 §2);本批条款化 rx CLI 总入口分发 + 退出码约定 + build/run/check/fmt/bench 的语义契约,收编 rx fmt(RD-005)与 rx bench(RD-003)。`rx test`(子进程隔离,M6.3)/ `rx doc`/`fix`/`watch`/`vendor`(后续小里程碑)的语义面随各自里程碑续写。错误码 `RX7003`(及按需 `RX7004`)为 7xxx 链接/工具链段位 rx CLI 诊断首批,**spec 先行引用,正式分配于 M6.1 实现 WP**(沿用 3xxx/5xxx 在实现 PR 落 registry 的节奏,registry revision_log 留痕,编号不复用)。

---

### RXS-0083 rx CLI 总入口与子命令分发

**Syntax**(命令行调用形态,08 §7 D-239):

```
RxInvocation ::= "rx" Subcommand SubcommandArgs
Subcommand   ::= "build" | "run" | "check" | "fmt" | "bench"
               | "test" | "doc" | "fix" | "watch" | "vendor"   // 后续里程碑承接
SubcommandArgs ::= <子命令各自定义的参数与 flag>
```

**Legality**(子命令分发与用法裁决):

- `rx` 总入口按**首位非 flag 实参**裁决子命令;缺子命令或子命令未识别 → 用法错误 `RX7003`(退出码 2,见退出码约定)。
- 子命令名是**保留分发位**:`build`/`run`/`check`/`fmt`/`bench` 为 M6.1 落地核心集;`test`/`doc`/`fix`/`watch`/`vendor` 为已登记的分发位,M6.1 期调用返回"未实现"用法诊断(退出码 2),其语义面随各自里程碑(M6.2~M6.5)条款化。
- **单一前端纪律(07 §2)**:涉及编译的子命令(build/run/check)经 rurixc query 层(`QueryCtx`)复用同一前端管线,**不另起编译引擎**;rx 是子命令分发与产物编排层,语义裁决归一到 rurixc。

**退出码约定**(全子命令统一,与 rurixc 驱动同口径):

| 退出码 | 含义 |
|---|---|
| 0 | 成功(子命令语义达成;`rx run` 为产物退出码 0 的情形) |
| 1 | 诊断错误(编译诊断 / fmt 非幂等或未格式化 / bench 正确性失败等"任务级失败") |
| 2 | 用法或 I/O 错误(未知子命令 / 缺参 / 文件不可读;`RX7003`) |

> `rx run` 的产物退出码透传是退出码约定的受控例外,见 RXS-0085。

**Implementation Requirements**:子命令分发在 rx 入口手写裁决(与 rurixc 驱动 arg 风格一致);用法诊断措辞允许保守粗糙(07 §4 先正确性后诊断);rx 不复制编译语义,经 rurixc 库面(driver)调用。

> 锚定测试:`src/rurixc/tests/toolchain_corpus.rs`(退出码约定常量契约 + 子命令集契约);rx CLI 子命令端到端冒烟(`ci/rx_cli_smoke.py`,M6.1 实现 WP)。

### RXS-0084 rx build 语义

**Legality**(build 子命令,经 rurixc query 层产物):

- `rx build <input.rx> [-o <out>]` 经 rurixc 前端管线(lex→parse→resolve→typeck→着色/launch/穷尽性/const eval→MIR→move/borrow/views/shared)后产 codegen 产物:
  - host 默认目标:LLVM IR → clang → COFF `.obj` → link.exe → host EXE(+ PDB),与 rurixc 驱动同链路(07 §7)。
  - device 目标(`--emit` device 通道):以 `kernel fn` 为根产 NVPTX IR / PTX(经 ptxas 干验证关卡,RXS-0073)。
- 前端任一阶段有 error → 阶段化中止,渲染诊断,退出码 1(与 rurixc 驱动同口径)。
- 工具链定位失败(clang pin 22.1.x / link.exe / libdevice)→ 工具链诊断(`RX7001`/`RX7002`,RXS-0073/RXS-0082),退出码 1。

**Implementation Requirements**:`rx build` 复用 rurixc 库 driver 的端到端编译路径,**不另起引擎**(07 §2);产物路径与中间产物(`.ll`/`.obj`)落盘约定与 rurixc 驱动一致;build 行为相对既有 rurixc 驱动**零语义漂移**(既有 golden / hello-world 冒烟不变)。

> 锚定测试:`conformance/toolchain/hello.rx`(可 build 的最小 host 程序);rx build 端到端冒烟(`ci/rx_cli_smoke.py`,M6.1 实现 WP)。

### RXS-0085 rx run 语义

**Legality**(run 子命令 = build + 执行):

- `rx run <input.rx>` 先执行 `rx build`(host EXE)语义(RXS-0084);build 失败则停于 build 的退出码语义(诊断 → 1),不执行产物。
- build 成功后**执行产物 EXE**,并**透传产物进程退出码**作为 `rx run` 的退出码(退出码约定的受控例外,RXS-0083):产物退出 0 → `rx run` 退出 0;产物退出 N → `rx run` 退出 N。
- 产物无法启动(spawn 失败 / 产物缺失)→ 工具链/执行诊断 `RX7004`,退出码 1。

**Implementation Requirements**:`rx run` 执行产物经 `std::process::Command`;退出码透传须区分"build/启动失败"(rx 自身退出码 1/2)与"产物正常运行后非零退出"(透传产物退出码),诊断措辞需提示二者区别。

> 锚定测试:`conformance/toolchain/exit_code.rx`(产物退出码透传契约样例);rx run 端到端冒烟(`ci/rx_cli_smoke.py`,M6.1 实现 WP)。

### RXS-0086 rx check 语义

**Legality**(check 子命令,仅前端):

- `rx check <input.rx>` 跑全量静态检查闭环(resolve→typeck→着色/launch/穷尽性→const eval→MIR→move/borrow/views/shared),**不产 codegen/link 产物**(对齐 rurixc `--emit=check`,07 §6 check 延迟计时口径)。
- 诊断错误 → 退出码 1;全检查通过且无产物 → 退出码 0。

**Implementation Requirements**:`rx check` 经 rurixc driver 的 check 路径复用单一前端(07 §2);check 不得有 codegen 副作用(无 `.ll`/`.obj`/EXE 落盘),供编译性能预算 check 计时口径稳定(07 §6)。

> 锚定测试:`conformance/toolchain/check_ok.rx`(check 通过样例);rx check 端到端冒烟(`ci/rx_cli_smoke.py`,M6.1 实现 WP)。

### RXS-0087 rx fmt 语义(收编雏形 + 幂等承诺)

**Legality**(fmt 子命令,RD-005 收编):

- `rx fmt <file>`:格式化结果写 stdout(收编 M1 雏形格式器 `rurixc::fmt::format_source`,RD-005)。
- `rx fmt --check <file>`:已格式化(与格式化结果字节一致,经 CRLF 归一)→ 退出码 0;否则退出码 1。
- `rx fmt --check-idempotent <file>`:核对 `fmt(fmt(x)) == fmt(x)`(字节级幂等判据,G-M6-4 延续 G-M1-5);违例退出码 1。
- 输入词法不洁(format 源 lex 出错)→ 退出码 1(任务级失败);文件不可读 → 退出码 2(I/O 错误)。

**Implementation Requirements**:`rx fmt` 复用 `rurixc::fmt::format_source` 库函数(单一事实源,与 `fmt_corpus` cargo test 通道同判据);收编后 M1 雏形二进制形态退役(RD-005 close 留痕);`ci/check_fmt_idempotent.py` 既有全语料二次格式化 0 diff 幂等门路由到 `rx fmt --check-idempotent`(G-M6-4)。格式行为变更须经审查(防风格漂移,gofmt 哲学)。

> 锚定测试:`src/rurixc/tests/fmt_corpus.rs`(format_source 幂等 + 输出可解析,字节级判据);`ci/check_fmt_idempotent.py`(全 `conformance/syntax` 语料经 rx fmt 二次格式化 0 diff,M6.1 实现 WP)。

### RXS-0088 rx bench 语义(BENCH_PROTOCOL 收编)

**Legality**(bench 子命令,RD-003 收编):

- `rx bench` 是 M5 bench harness(`bench/*.py`)的**统一工具链入口**(RD-003 收编):复用 [../milestones/m0/BENCH_PROTOCOL.md](../milestones/m0/BENCH_PROTOCOL.md) §3 协议——L0 锁频前置(§2.1)/ 三次进程级独立运行 / trimmed mean(去头尾 20%);既有 `measured_local` 证据口径不变(`evidence/` 只增不删不改)。
- `rx bench --smoke`:跑既有基准 smoke 正确性路径(产物正确性核对,非计时回填),退出码 0 = 正确性 PASS;正确性失败 → 退出码 1。
- measured 路径(nightly L1/L2 回填)经 rx bench 入口驱动既有三次进程级聚合协议,协议与证据格式不变(BENCH_PROTOCOL §3)。

**Implementation Requirements**:`rx bench` 作统一入口编排既有 BENCH_PROTOCOL 协议实现(`bench/*.py`),口径与 `evidence/*.json` 证据格式完全不变(证据连续性,RD-003);收编后既有 harness 脚本降级为"被 rx bench 调用的协议库"并在 RD-003 close 时留痕;L0 锁频前置纪律(未锁频不得回填预算)延续。

> 锚定测试:`src/rurixc/tests/toolchain_corpus.rs`(BENCH_PROTOCOL §3 协议参数契约 + bench smoke 入口存在性);rx bench `--smoke` 正确性(`ci/rx_cli_smoke.py`,M6.1 实现 WP)。

---

## 错误码引用汇总

| 错误码 | 含义 | 条款 |
|---|---|---|
| RX7001 | 工具链失败(clang pin 22.1.x / link.exe 定位 / 退出非零) | RXS-0084 |
| RX7002 | libdevice bitcode 链接失败 | RXS-0084 |
| RX7003 | rx CLI 子命令用法错误(未知子命令 / 缺参 / 未实现分发位) | RXS-0083 |
| RX7004 | rx run 产物执行失败(spawn 失败 / 产物缺失) | RXS-0085 |

含义以 [../registry/error_codes.json](../registry/error_codes.json) 为唯一事实源,本表仅引用。RX7001/RX7002 已于 M4.2/M5.3 分配。RX7003/RX7004 为 7xxx 链接/工具链段位 rx CLI 诊断首批(07 §5 段位语义,工具链类归 7xxx 续接),**spec 先行引用,正式分配于 M6.1 实现 WP**(registry revision_log 留痕,编号不复用、含义冻结)。

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-15 | 初版:RXS-0083 ~ RXS-0088(M6.1 rx CLI 子命令语义面首批:总入口与子命令分发 + 退出码约定 / build / run / check / fmt 收编 RD-005 / bench 收编 RD-003;07 §2 §6 §9 单一前端 + 08 §7 D-239 rx CLI + BENCH_PROTOCOL §3 已锁定决策的条款化,M6 契约 D-M6-1 spec 先行)。错误码汇总表登记 RX7003/RX7004(spec 先行引用,实现 WP 正式分配,7xxx 续接);包管理 manifest/lock 格式条款(M6.2)与 LSP 能力面条款(M6.4)续写本文件 | Direct |
