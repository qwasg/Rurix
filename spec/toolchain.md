# Rurix 语言规范 — 工具链语义(M6.1~M6.3:rx CLI / 包管理 / rx test 与离线复现)

> 条款:RXS-0083 ~ RXS-0103(M6.1 rx CLI 子命令语义面首批 + M6.2 包管理 + M6.3 rx test / workspace / 离线重建复现门 + M6.4 LSP MVP)。体例见 [README.md](README.md)。
> 依据:07 §2 §6 §9(查询化与增量编译 D-203 / 编译性能预算 / LSP 与工具模式 D-210——单一前端,常驻 query 层);08 §4 §7(rx bench harness 工具化 / 开发者工具集 rx CLI D-239);milestones/m0/BENCH_PROTOCOL.md(基准协议 §2/§3);M6 契约 D-M6-1 / G-M6-3 / G-M6-4 / G-M6-5(spec 先行)。
> 本文为已选定决策(D-203/D-210/D-239)的初版条款化(档位 Direct);任何偏离 07/08 已锁定决策的语义动作须按 10 §3 升档。本文承载工具链语义条款,M6.2/M6.4 的包管理 `rurix.toml`/`rurix.lock` 格式条款与 LSP 能力面条款续写本文件(编号续号)。
> **M6.2 续号(RXS-0089 ~ RXS-0094,包管理 manifest/lock/vendor)**:`rurix.toml`(意图)+ `rurix.lock`(精确解析图 + 内容树 SHA-256)格式语义 + 依赖三来源 path/git/archive 解析规则 + workspace 单根锁 + feature additive-v1 + 无 build.rs 声明式(09 §7.1/§7.2 已锁定决策 D-308~D-311 的条款化,档位 Direct;registry sumdb D-312 不触碰)。错误码 `RX7005` ~ `RX7009`(7xxx 链接/工具链段位续接)随 M6.2 实现 WP 正式分配,registry revision_log 留痕、含义冻结。
> **M6.4 续号(RXS-0098 ~ RXS-0103,LSP MVP + 常驻 query 层)**:`rurixc --tooling-server` 常驻进程(stdio JSON-RPC)经单一前端 query 层服务 LSP MVP 六项能力(publishDiagnostics 直接消费 07 §5 诊断 JSON / completion / definition / references / documentHighlight / rename);进程内 memoization + 模块/函数级失效(RD-004 无损语法树通道接通,parser 事件流 → rowan 式绿树)。错误码 RX7012+(7xxx 续接)随 LSP 工具层诊断实现 WP 正式分配,registry revision_log 留痕、含义冻结。
> **M6.3 续号(RXS-0095 ~ RXS-0097,rx test / workspace / 离线重建复现门)**:`rx test` 内建 `#[test]`/`#[test(gpu)]` 发现与逐测试子进程隔离(14 §6);workspace members 激活入单根锁;三包 workspace(path/git/archive)在 `rx build --locked --offline` reproducible profile 下两次 host EXE SHA-256 逐字节一致(G-M6-1,09 §7.1/§7.2,14 §1/§6 契约机制)。错误码 `RX7010`/`RX7011`随 M6.3 实现 WP 正式分配,registry revision_log 留痕、含义冻结。
> **M6.1 范围裁决(rx CLI 总入口 + 核心子命令优先)**:rx 经 rurixc query 层复用单一前端,不另起引擎(07 §2);本批条款化 rx CLI 总入口分发 + 退出码约定 + build/run/check/fmt/bench 的语义契约,收编 rx fmt(RD-005)与 rx bench(RD-003)。`rx test`(子进程隔离,M6.3)/ `rx doc`/`fix`/`watch`/`vendor`(后续小里程碑)的语义面随各自里程碑续写。错误码 `RX7003`(及按需 `RX7004`)为 7xxx 链接/工具链段位 rx CLI 诊断首批,**spec 先行引用,正式分配于 M6.1 实现 WP**(沿用 3xxx/5xxx 在实现 PR 落 registry 的节奏,registry revision_log 留痕,编号不复用)。

---

### RXS-0083 rx CLI 总入口与子命令分发

**Syntax**(命令行调用形态,08 §7 D-239):

```
RxInvocation ::= "rx" Subcommand SubcommandArgs
Subcommand   ::= "build" | "run" | "check" | "fmt" | "bench" | "vendor" | "test"
               | "doc" | "fix" | "watch"   // 后续里程碑承接
SubcommandArgs ::= <子命令各自定义的参数与 flag>
```

**Legality**(子命令分发与用法裁决):

- `rx` 总入口按**首位非 flag 实参**裁决子命令;缺子命令或子命令未识别 → 用法错误 `RX7003`(退出码 2,见退出码约定)。
- 子命令名是**保留分发位**:`build`/`run`/`check`/`fmt`/`bench` 为 M6.1 落地核心集;`vendor` 为 M6.2 落地核心集;`test` 为 M6.3 落地核心集;`doc`/`fix`/`watch` 继续保留为后续分发位,调用返回"未实现"用法诊断(退出码 2),其语义面随各自里程碑(M6.4~M6.5 或后续)条款化。
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

### RXS-0089 rurix.toml 清单格式与字段(声明式,无 build.rs)

**Syntax**(`rurix.toml` 清单,TOML 子集;09 §7.1):

```
[package]
name    = "<crate-name>"          # 必填,非空,[A-Za-z0-9_-]
version = "<major.minor.patch>"   # 必填,语义化三段版本
build   = "declarative"           # 可选,缺省即 "declarative";唯一合法值(无 build.rs 红线)

[dependencies]
<dep> = { <source> }              # <source> 见 RXS-0090(path/git/archive 三选一)
<dep> = { <source>, features = ["..."], default-features = false }

[features]
default = ["<feat>", ...]         # 可选;feature → 启用的 feature/依赖 feature 列表
<feat>  = ["<feat>", "<dep>/<feat>", ...]

[workspace]                       # 可选;存在即为 workspace 根清单
members = ["<rel-dir>", ...]
```

**Legality**(清单合法性):

- `[package].name` 与 `[package].version` 必填;`version` 须为三段点分非负整数(`major.minor.patch`),否则清单错误 `RX7005`。
- `[package].build` 若出现,唯一合法值为字符串 `"declarative"`;任何其他值(尤其试图引入构建脚本/逃生舱)→ `RX7005`(无 build.rs 红线,09 §7.1;硬需求按 14 §4 登记 RD-### 而非改此处)。
- TOML 子集仅支持:`[table]` / `[[array.table]]` 头、`key = value`(字符串 / 整数 / 布尔 / 字符串数组 / 内联表);未闭合定界符 / 非法键 / 重复键 / 不支持的标量 → `RX7005`。
- 清单缺失 / 不可读 → `RX7005`(I/O 与解析错误同归清单错误段)。

**Implementation Requirements**:清单解析由 `rurix-pkg::manifest` 手写 TOML 子集解析器实现(零外部依赖,与全仓零依赖纪律一致);解析器产出确定性模型(键有序),为 `rurix.lock` 与内容树哈希的逐字节复现铺底(RXS-0092/0093)。

> 锚定测试:`src/rurix-pkg/src/manifest.rs`(清单解析 + `build` 非法值拒绝单测);`conformance/pkg/`(可解析的样例 workspace 清单)。

### RXS-0090 依赖三来源 path/git/archive 解析规则

**Syntax**(依赖来源,三选一,互斥):

```
<dep> = { path = "<rel-dir>" }                              # 本地路径源
<dep> = { git = "<url>", rev = "<commit>" }                # git 源(rev 必填,精确提交)
<dep> = { archive = "<url>", sha256 = "<64-hex>" }         # 归档源(sha256 必填,内容指纹)
```

**Legality**(来源裁决):

- 每个依赖**恰好**一个来源键(`path` / `git` / `archive`);零个或多于一个 → `RX7005`。
- `git` 源必须携带 `rev`(精确提交,不接受可变 ref/分支名,供逐字节复现);`archive` 源必须携带 `sha256`(64 位十六进制内容指纹)。缺失 → `RX7005`。
- 来源在解析阶段不可达(`path` 目标目录缺失;`--offline` 下 `git`/`archive` 需要网络且无 vendor 缓存)→ 来源不可达 `RX7009`(见 RXS-0094 离线路径)。
- M6.2 范围内 `path` 源端到端可解析(离线、无网);`git`/`archive` 源的清单形态与 lock 记录形态条款化并落解析图节点,实际网络抓取与 vendor 落盘在带网络/缓存环境验证(三包逐字节复现门归 M6.3,G-M6-1)。

**Implementation Requirements**:三来源在解析图中统一为 `Source { kind, locator, pin }` 节点(`pin` = git rev / archive sha256 / path 无 pin);来源字符串在 `rurix.lock` 以稳定前缀编码(`path:` / `git:<url>#<rev>` / `archive:<url>#<sha256>`,RXS-0092),保证解析图稳定。

> 锚定测试:`src/rurix-pkg/src/resolve.rs`(三来源节点构造 + 互斥/缺 pin 拒绝单测);`conformance/pkg/`(path 源样例)。

### RXS-0091 依赖解析图与 feature 加性合一(additive-v1)

**Legality**(解析图与 feature 合并):

- 解析图以根清单(或 workspace 根)为根,广度遍历 `[dependencies]` 构建有向图;**workspace 单根锁**:整个 workspace 共享单一解析图与单一 `rurix.lock`(09 §7.2)。
- feature 统一为 **additive-v1**(`unification = "selected"`):同一依赖在图中被多个上游以不同 feature 集启用时,该依赖最终启用的 feature 集 = 各上游所选 feature 的**并集**(加性,启用不撤销);`default-features = false` 抑制该边对 `default` 的引入,但不撤销其他边已选的 feature。
- 冲突检测:同一依赖名解析到**不相容的来源/pin**(如两处 path 指向不同目录、或 git rev 不一致)→ 解析冲突 `RX7006`;feature 引用不存在的 feature/依赖 → `RX7006`。
- 解析图须为确定性(节点按依赖名排序、feature 集排序),保证 `rurix.lock` 逐字节稳定。

**Implementation Requirements**:`rurix-pkg::resolve` 产出 `ResolveGraph { nodes: BTreeMap<name, ResolvedPackage> }`;feature 合一为不动点的并集合并(加性单调,保证收敛);冲突即 `RX7006`,不做版本回溯求解(MVP 单根锁,无 registry,09 §7.2)。

> 锚定测试:`src/rurix-pkg/src/resolve.rs`(feature 加性合一 + 来源冲突 `RX7006` 拒绝单测)。

### RXS-0092 rurix.lock 格式(精确解析图 + 内容树 SHA-256)

**Syntax**(`rurix.lock`,生成物,确定性序列化):

```
# rurix.lock — generated by `rx vendor`; do not edit by hand.
lock_version = 1
root = "<root-package-name>"

[[package]]
name           = "<name>"
version        = "<version>"
source         = "path:<rel>" | "git:<url>#<rev>" | "archive:<url>#<sha256>"
content_sha256 = "<64-hex>"           # 内容树规范化哈希,RXS-0093
features       = ["<feat>", ...]      # 排序后的最终启用 feature 集
deps           = ["<name>", ...]      # 排序后的直接依赖名
```

**Legality**(lock 一致性):

- `rurix.lock` 是解析图(RXS-0091)的精确序列化:`[[package]]` 表按 `name` 排序,数组字段元素排序,保证逐字节确定性。
- `--locked` 模式(RXS-0094):由当前清单重新解析得到的图与入库 `rurix.lock` **不一致**(包集合 / source / features / deps 差异)→ lock 不一致 `RX7007`(不静默重写)。
- `content_sha256` 与 vendor 内容实测不符 → digest 不符 `RX7008`(见 RXS-0093)。

**Implementation Requirements**:`rurix-pkg::lock` 提供 `write`(确定性序列化)与 `parse`(读回)+ `check_consistent`(与重解析图比对);lock 写出与读回须 round-trip 稳定(`parse(write(g)) == g`)。

> 锚定测试:`src/rurix-pkg/src/lock.rs`(序列化 round-trip + `--locked` 不一致 `RX7007` 单测)。

### RXS-0093 内容树规范化 SHA-256

**Legality**(内容树哈希,逐字节复现根):

- 包内容树哈希在**规范化**输入上计算,消除非确定性源(09 §7.1 / M6_PLAN §6 风险):
  1. 仅纳入包内容文件(`rurix.toml` + `src/**` 等源文件);排除 `vendor/`、`target/`、VCS 元数据与生成物。
  2. 文件项按**相对路径字典序**排序(路径以 `/` 归一,平台无关)。
  3. **不纳入**文件时间戳 / 权限 / inode 等文件系统元数据;仅纳入相对路径与文件字节内容。
  4. 行结尾按字节原样纳入(不做 CRLF 归一;源文件的字节内容即事实)。
- 哈希算法为 SHA-256(自实现,确定性);最终 `content_sha256` = 对规范化序列(逐文件 `len(path) ‖ path ‖ len(content) ‖ content`,长度为定宽小端)的 SHA-256 十六进制小写。

**Implementation Requirements**:`rurix-pkg::sha256` 手写 SHA-256(FIPS 180-4,零依赖),附 known-answer 自测(空串 / `"abc"` 标准向量);`rurix-pkg::content_tree` 实现规范化序列化 + 哈希,保证同一内容树在不同机器/时刻哈希一致(M6.3 两次重建逐字节比对的判据来源)。

> 锚定测试:`src/rurix-pkg/src/sha256.rs`(known-answer 向量单测);`src/rurix-pkg/src/content_tree.rs`(排序/去元数据确定性单测)。

### RXS-0094 vendor/ 与离线解析路径(--locked/--offline)

**Legality**(vendor 与离线):

- `vendor/` 是**可提交**的依赖快照目录:`rx vendor` 解析图后将各依赖内容落 `vendor/<name>/`,并写 `rurix.lock`(含每包 `content_sha256`)。vendor 目录本身不纳入上层包的内容树哈希(RXS-0093 排除项)。
- `--offline`:解析只允许本地来源(`path` 源 + 已存在的 `vendor/` 缓存);任何需要网络的来源(无缓存的 `git`/`archive`)→ 来源不可达 `RX7009`。
- `--locked`:不重写 `rurix.lock`;以入库 lock 为准并校验(RXS-0092 不一致 `RX7007`)+ 校验 `vendor/` 内容树 digest 与 lock 记录一致(不符 `RX7008`)。
- `--locked --offline` 组合 = 默认离线可重建路径:不触网、不改 lock,逐包校验内容树 digest;为 M6.3 三包离线重建逐字节复现门(G-M6-1)铺底。

**Implementation Requirements**:`rx vendor` 与 `rx build` 的 manifest 解析前段共享 `rurix-pkg::{resolve,lock,vendor}`;`rx build` 在有清单时先跑解析前段(`--locked`/`--offline` 校验)再经 `rurixc::driver` 单一前端编译根入口(**不另起引擎**,07 §2);workspace 多包实际构建与三包逐字节复现门归 M6.3。

> 锚定测试:`src/rurix-pkg/src/vendor.rs`(离线 `RX7009` / locked digest `RX7008` 校验单测);`ci/pkg_resolve_smoke.py`(样例 workspace 离线解析 + lock + digest 复核,篡改即红)。

---

### RXS-0095 rx test 内建测试运行器与子进程隔离

**Syntax**(`rx test`,M6.3;14 §6 harness 隔离纪律工具化):

```
RxTest ::= "rx" "test" TestInput? TestFlag*
TestInput ::= <file.rx>
TestFlag ::= "--filter" <substring>
          | "--gpu"
          | "--manifest-path" <rurix.toml>
          | "--locked"
          | "--offline"

TestAttr ::= "#[test]" | "#[test(gpu)]"
TestFn   ::= TestAttr "fn" IDENT "(" ")" ("->" ("()" | "i32"))? Block
```

**Legality**(测试发现与签名):

- `rx test` 发现**顶层 free function** 上的 `#[test]` 与 `#[test(gpu)]`;trait/impl/extern 内关联函数不参与 M6.3 v1 发现。
- 测试函数必须为 host 普通函数,无参数,有函数体,返回 `()`(显式或省略)或 `i32`;其他签名 → `RX7010`(测试发现/签名错误,退出码 1)。
- M6.3 v1 为逐测试生成临时 `main` 的窄运行器:测试源文件不得同时定义顶层 `fn main`;存在顶层 `main` 且发现测试 → `RX7010`,避免生成 harness 与用户入口冲突。
- 默认 `rx test <file.rx>` 仅运行 `#[test]` host 测试;`--gpu` 仅运行 `#[test(gpu)]` 分类测试;`--filter` 对测试函数名做子串过滤。过滤后无可运行测试 → `RX7010`。
- `--locked`/`--offline` 仅在 `--manifest-path` 包上下文有效,先执行 RXS-0094 的 lock/vendor 校验;包前段失败按 RX7005~RX7009 返回,不进入测试执行。

**Dynamic Semantics**(隔离运行):

- 每个测试函数生成独立临时 harness 源文件,该 harness 只调用一个测试函数并以独立子进程执行。`()` 测试返回即成功;`i32` 测试返回 0 成功、非 0 失败。
- 单个测试编译失败、spawn 失败、被信号/异常终止或退出非 0 → `RX7011`(测试子进程执行失败,退出码 1);父进程继续收集其余测试结果,最终汇总失败。
- `#[test(gpu)]` 是分类与隔离契约:GPU 相关测试同样以独立子进程执行,使 GPU context poison / 进程崩溃不连坐 harness(14 §6)。完整 GPU 正确性仍由既有 self-hosted runner 与 Compute Sanitizer nightly 路径验证。

**Implementation Requirements**:`rx test` 使用 `rurixc::test_harness` 复用 lexer/parser/AST 进行发现与签名校验,不得以正则扫描代替语义解析;每个测试编译仍经 `rurixc::driver` 单一前端,不另起编译引擎。临时 harness 目录须位于系统临时目录或构建目录下并在运行后 best-effort 清理。

> 锚定测试:`src/rurixc/src/test_harness.rs`(测试发现/签名校验单测);`conformance/toolchain/rx_test_basic.rx` / `conformance/toolchain/rx_test_gpu.rx`(`#[test]` / `#[test(gpu)]` 样例);`ci/rx_cli_smoke.py`(rx test 端到端纳入核心子命令计数)。

### RXS-0096 workspace members 多包参与单根锁

**Syntax**(workspace 根清单,延续 RXS-0089):

```
[workspace]
members = ["<rel-dir>", ...]
```

**Legality**(workspace 多包):

- 若根清单含 `[workspace].members`,每个 member 必须指向一个含 `rurix.toml` 的本地包目录;缺失 → `RX7009`。
- workspace members 参与同一个解析图与同一个 `rurix.lock`:根包 + members + 递归依赖(path/git/archive 三来源)必须落入同一 lock 图,并按 RXS-0091 的单根锁冲突规则合一。
- member 包以其 `[package].name` 作为解析图节点名;若根 `[dependencies]` 已显式声明同名包且来源 locator 不同 → `RX7006`;相同 locator 视为同一节点。
- M6.3 v1 编译入口仍为 root `src/main.rx`;workspace members/deps 进入 lock/digest 校验与离线复现门,跨包链接不在本步引入。

**Implementation Requirements**:`rurix-pkg::vendor::resolve_workspace` 在调用解析器前把 workspace members 归一为根的 path 依赖边,使 `rx vendor` / `rx build --manifest-path` / `rx test --manifest-path` 共享同一解析图。远端 git/archive 源不新增网络抓取;离线重建使用已提交的 `vendor/<name>` 缓存。

> 锚定测试:`src/rurix-pkg/src/vendor.rs`(workspace members 注入单根锁单测);`conformance/workspace/repro/rurix.toml`(path/git/archive 三来源 workspace 样例)。

### RXS-0097 G-M6-1 离线重建逐字节复现门

**Syntax**(门禁命令形态):

```
ReproBuild ::= "rx" "build" "--manifest-path" <rurix.toml> "--locked" "--offline" ("-o" <exe>)?
```

**Legality**(可复现判据):

- G-M6-1 样例 workspace 必须至少包含三个依赖包,且 path/git/archive 三来源各 ≥1;git/archive 远端源在 M6.3 通过已提交 `vendor/<name>` 快照验证离线可重建,不触 registry sumdb(D-312)与网络抓取。
- `rx build --locked --offline` 在包上下文中必须先执行 RXS-0094:不触网、不改 `rurix.lock`,并校验 lock 解析图与 vendor 内容树 digest。lock 不一致 → `RX7007`;vendor digest 不符 → `RX7008`;远端无缓存或 path 不可达 → `RX7009`。
- 在 reproducible profile 下,同一干净路径中清输出后连续两次构建 host EXE,EXE 字节 SHA-256 必须一致;同时 `rurix.lock` SHA-256 与 vendor 内容树 digest 在两次构建前后不变。
- 普通 debug build 的 PDB/source path 语义不受 G-M6-1 约束;reproducible profile 可关闭 debug link/PDB 并启用链接器可复现开关,以保证门禁哈希稳定。

**Implementation Requirements**:CI 门 `ci/offline_rebuild_repro.py` 必须复制 fixture 到干净临时目录后真跑两次 `rx build --manifest-path ... --locked --offline`,比较 EXE SHA-256、lock SHA-256 与 vendor digest,并写 `evidence/offline_rebuild_*.json` 作为 `m6.counter.offline_rebuild_reproducible` 计数源。该门必须内建红绿验证:临时篡改 fixture 的 vendor 内容或 digest 后预期 `RX7008`/`RX7007` 红,复原后绿;应红却绿即脚本自身失败(反 YAML-only,H06 D11.8-2)。

> 锚定测试:`conformance/workspace/repro/src/main.rx`(G-M6-1 workspace 根入口样例);`ci/offline_rebuild_repro.py`(两次 build 逐字节一致 + 篡改红绿门);`milestones/m6/offline_rebuild_evidence_schema.json`(证据 schema)。

### RXS-0098 rurixc --tooling-server 常驻 query 层

**Syntax**(工具模式调用形态,07 §9 D-210):

```
ToolingServer ::= "rurixc" "--tooling-server" ("--stdio")?
```

**Legality**:

- LSP 语义必须全部来自 `rurixc` query 层(单一前端,07 §2/§9);`rx` 不得另起 language server 引擎。
- `--tooling-server` 以 stdio JSON-RPC 2.0 常驻进程模式运行;MVP 不做跨会话红绿增量(D-203 Phase 2+)。
- `ToolingSession` 维护已打开文档的源文本、版本号、无损语法树(RD-004)与 `QueryCtx`;文档变更触发 query memo 失效(模块/函数级粒度,MVP 可回退为整文档重建,但须保留模块/函数级失效 API 与单测)。

**Implementation Requirements**:`src/rurixc/src/tooling/session.rs` 实现 `ToolingSession`;`src/rurixc/src/query.rs` 暴露 `invalidate_bodies` / `invalidate_module` 与 memo 计量单测。CI 门 `ci/lsp_smoke.py` 真跑 `--tooling-server` 往返冒烟。

> 锚定测试:`src/rurixc/src/query.rs`(memo 命中与失效单测);`src/rurixc/src/tooling/session.rs`(session 生命周期);`ci/lsp_smoke.py`(server 往返)。

### RXS-0099 publishDiagnostics 与 07 §5 诊断 JSON

**Syntax**(诊断 JSON 字段,07 §5 第 4 条):

```
DiagJson ::= "{" "level" ":" Level "," "message" ":" string
           ("," "code" ":" "RX####")?
           ("," "spans" ":" SpanList)?
           ("," "labels" ":" LabelList)?
           ("," "suggestions" ":" SuggestionList)? "}"
```

**Legality**:

- `publishDiagnostics` 必须直接消费与 `--error-format=json` 同形的结构化诊断 JSON;禁止 LSP 侧二次渲染文本诊断。
- 诊断 span 必须为 UTF-8 字节偏移,经 `SourceMap` 映射为 LSP `Range`(UTF-16 code unit 列)。
- MVP 容忍保存/`didChange` 后全量 body 重查询(07 §9);增量细化随 RD-004 通道与 Phase 2。

**Implementation Requirements**:`src/rurixc/src/tooling/diag_json.rs` 序列化 `DiagData`;LSP server 在 `textDocument/didOpen` 与 `textDocument/didChange` 后 push `textDocument/publishDiagnostics`。`rurixc --emit=check --error-format=json` 输出同形 JSON 供非 LSP 回归。

> 锚定测试:`src/rurixc/src/tooling/diag_json.rs`(JSON 形状单测);`conformance/toolchain/lsp_mvp/sample.rx`(类型错误诊断锚点);`ci/lsp_smoke.py`(诊断往返)。

### RXS-0100 textDocument/completion

**Legality**:

- 补全请求必须在光标所在作用域内返回可见标识符与首批关键字(`fn`/`let`/`if`/`return` 等);MVP 限单文件,不跨 crate 索引。
- 补全项 `label` 必须为标识符文本;`kind` 映射 LSP `CompletionItemKind`(函数/变量/关键字)。

**Implementation Requirements**:`src/rurixc/src/tooling/ide_query.rs::completions_at`;LSP handler `textDocument/completion`;fixture `sample.rx` 在 `foo` 前缀处须命中局部绑定 `foo`。

> 锚定测试:`src/rurixc/src/tooling/ide_query.rs`(completion 单测);`conformance/toolchain/lsp_mvp/sample.rx`;`ci/lsp_smoke.py`。

### RXS-0101 textDocument/definition 与 textDocument/references

**Legality**:

- `definition` 必须解析光标处路径/绑定至 `DefId` 或 `LocalId`,返回定义点 span 映射的 LSP `Location`。
- `references` 必须在单文件内返回所有同目标引用 span(MVP 不跨文件);未解析路径不得伪造结果。
- 跳转失败返回空结果,不 ICE。

**Implementation Requirements**:`ide_query::definition_at` / `ide_query::references_at`;LSP handlers `textDocument/definition` / `textDocument/references`;fixture 对 `helper` 调用须至少 2 处引用(定义 + 使用)。

> 锚定测试:`src/rurixc/src/tooling/ide_query.rs`;`conformance/toolchain/lsp_mvp/sample.rx`;`ci/lsp_smoke.py`。

### RXS-0102 textDocument/documentHighlight

**Legality**:

- 高亮请求须返回光标处符号的所有出现 span(只读/写入区分 MVP 可统一为 Read);MVP 单文件。
- 结果映射 LSP `DocumentHighlight` 列表,range 与 RXS-0099 span 映射规则一致。

**Implementation Requirements**:`ide_query::highlights_at`;LSP handler `textDocument/documentHighlight`;与 RXS-0101 引用集一致。

> 锚定测试:`src/rurixc/src/tooling/ide_query.rs`;`ci/lsp_smoke.py`。

### RXS-0103 textDocument/rename 与 WorkspaceEdit

**Legality**:

- 重命名须校验新名为合法标识符;冲突(同作用域已有同名) → 空编辑 + 诊断(按需 RX7012+)。
- 成功时返回 `WorkspaceEdit` 覆盖单文件内全部引用 span 的文本替换;MVP 不跨文件。

**Implementation Requirements**:`ide_query::rename_at`;LSP handler `textDocument/rename`;fixture 将 `foo` 重命名为 `renamed_foo` 须更新所有出现点。

> 锚定测试:`src/rurixc/src/tooling/ide_query.rs`;`conformance/toolchain/lsp_mvp/sample.rx`;`ci/lsp_smoke.py`。

---

## 错误码引用汇总

| 错误码 | 含义 | 条款 |
|---|---|---|
| RX7001 | 工具链失败(clang pin 22.1.x / link.exe 定位 / 退出非零) | RXS-0084 |
| RX7002 | libdevice bitcode 链接失败 | RXS-0084 |
| RX7003 | rx CLI 子命令用法错误(未知子命令 / 缺参 / 未实现分发位) | RXS-0083 |
| RX7004 | rx run 产物执行失败(spawn 失败 / 产物缺失) | RXS-0085 |
| RX7005 | rurix.toml 清单解析/校验错误(缺字段 / 非法值 / build 非声明式 / TOML 子集违例) | RXS-0089 / RXS-0090 |
| RX7006 | 依赖解析冲突(来源/pin 不相容 / feature 引用不存在) | RXS-0091 |
| RX7007 | rurix.lock 不一致(--locked 下重解析图 ≠ 入库 lock) | RXS-0092 |
| RX7008 | 内容树 digest 不符(vendor 内容 SHA-256 ≠ lock 记录) | RXS-0093 |
| RX7009 | 依赖来源不可达(--offline 需网无缓存 / path 目标缺失) | RXS-0090 / RXS-0094 |
| RX7010 | rx test 测试发现/签名错误(无测试 / main 冲突 / 参数或返回类型不合法) | RXS-0095 |
| RX7011 | rx test 子进程执行失败(编译失败 / spawn 失败 / 测试进程非零或异常终止) | RXS-0095 |
| RX7012 | LSP rename 目标无效(新名非法 / 同作用域冲突 / 不可重命名符号) | RXS-0103 |

含义以 [../registry/error_codes.json](../registry/error_codes.json) 为唯一事实源,本表仅引用。RX7001/RX7002 已于 M4.2/M5.3 分配,RX7003/RX7004 于 M6.1 分配。RX7005 ~ RX7009 为 7xxx 链接/工具链段位包管理诊断(07 §5 段位语义,工具链类归 7xxx 续接),**spec 先行引用,正式分配于 M6.2 实现 WP**(registry revision_log 留痕,编号不复用、含义冻结)。RX7010/RX7011 为 7xxx 链接/工具链段位 `rx test` 诊断,**spec 先行引用,正式分配于 M6.3 实现 WP**。RX7012 为 7xxx 链接/工具链段位 LSP rename 诊断,**spec 先行引用,正式分配于 M6.4 实现 WP**。

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-15 | 初版:RXS-0083 ~ RXS-0088(M6.1 rx CLI 子命令语义面首批:总入口与子命令分发 + 退出码约定 / build / run / check / fmt 收编 RD-005 / bench 收编 RD-003;07 §2 §6 §9 单一前端 + 08 §7 D-239 rx CLI + BENCH_PROTOCOL §3 已锁定决策的条款化,M6 契约 D-M6-1 spec 先行)。错误码汇总表登记 RX7003/RX7004(spec 先行引用,实现 WP 正式分配,7xxx 续接);包管理 manifest/lock 格式条款(M6.2)与 LSP 能力面条款(M6.4)续写本文件 | Direct |
| v1.1 | 2026-06-15 | 续写 RXS-0089 ~ RXS-0094(M6.2 包管理 manifest/lock/vendor:rurix.toml 清单格式与声明式无 build.rs / 依赖三来源 path·git·archive 解析规则 / 依赖解析图与 feature additive-v1 加性合一(unification="selected")+ 冲突检测 / rurix.lock 精确解析图格式 / 内容树规范化 SHA-256 / vendor 与离线解析路径 --locked·--offline;09 §7.1/§7.2 已锁定决策 D-308~D-311 的条款化,M6 契约 D-M6-2 / G-M6-1 spec 先行)。错误码汇总表登记 RX7005 ~ RX7009(spec 先行引用,实现 WP 正式分配,7xxx 续接);registry sumdb D-312 不触碰。LSP 能力面条款(M6.4)续写本文件 | Direct |
| v1.2 | 2026-06-15 | 续写 RXS-0095 ~ RXS-0097(M6.3 rx test 子进程隔离 + workspace members 多包 + G-M6-1 三包离线重建逐字节复现门:顶层 `#[test]`/`#[test(gpu)]` 签名与逐测试子进程 harness / `[workspace].members` 进入单根 lock 图 / `rx build --locked --offline` reproducible profile 两次 host EXE SHA-256 一致且 lock/vendor 不改写;14 §6 / 09 §7.1§7.2 / M6 契约 D-M6-3·G-M6-1 的条款化)。错误码汇总表登记 RX7010/RX7011(spec 先行引用,实现 WP 正式分配,7xxx 续接) | Direct |
| v1.3 | 2026-06-15 | 续写 RXS-0098 ~ RXS-0103(M6.4 LSP MVP + 常驻 query 层 + RD-004 无损语法树通道接通:`rurixc --tooling-server` stdio JSON-RPC / publishDiagnostics 消费 07 §5 诊断 JSON / completion / definition+references / documentHighlight / rename;07 §9 D-210 单一前端 + M6 契约 D-M6-4·G-M6-2·G-M6-5 的条款化)。错误码汇总表登记 RX7012(spec 先行引用,实现 WP 正式分配,7xxx 续接) | Direct |
