# Rurix 语言规范 — const 求值语义(M3.4:const fn 子集 / const item 求值 / const 泛型 / 求值失败)

> 条款:RXS-0062 ~ RXS-0065(M3.4 const eval 首批)。体例见 [README.md](README.md)。
> 依据:05 §9(const eval 与 const 泛型 D-111);07 §1 §5(IR 四层 / 错误码段位 5xxx);M3 契约 D-M3-5(const eval MIR 解释器 + const 泛型可用)、D-M3-2(spec 先行)。
> 本文为已选定决策(D-111)的初版条款化(档位 Direct);任何偏离 05 §9 已锁定决策的修改须按 10 §3 升档。
> **范围裁决(M3.4 标量优先,D-111 最小面)**:const 求值在编译期对整数 / `bool` 子集 + 数组构造产出常量值;const 泛型值(数组长度 / const 参数)接入类型系统与单态化。**运行期数组 aggregate 的 codegen 不在本里程碑**(数组表达式按值出现仍受 `RX6001` 拦截,登记为已知限制随 M4+ 只追加扩展,07 §4 先正确性后诊断);堆分配 / trait 调度 const eval 永久排除(D-111)。

---

### RXS-0062 const 求值上下文与 const fn 子集

**Legality**:

- **const 求值上下文**(const context):要求其值在编译期确定的位置,限于——
  - `const` item 的初始化器(`const NAME: T = EXPR;`);
  - 类型中的数组长度表达式(`[T; LEN]` 的 `LEN`);
  - const 泛型实参(调用 / 类型实例化处对 `const` 参数的赋值,RXS-0064)。
- `static` item 初始化器**不**强制为 const 求值上下文(其求值时点与 const item 不同;M3.4 维持 `static` 既有处置,本条款不扩面)。
- **const fn 子集**:`const fn` 标注的函数可在 const 求值上下文被调用求值。其 body 仅允许以下构造,违反即非 const 操作 `RX5003`:
  - 局部 `let` 绑定、整数 / `bool` / 浮点字面量与 const 泛型参数引用;
  - 算术 / 比较 / 布尔运算(RXS-0043 运算面);
  - `if` / `else`、`loop` / `while` / `for`(条件与边界须 const 可求值)、`match`(臂体 const 可求值);
  - 对其他 `const fn` 的调用、对其他 `const` item 的引用;
  - 数组构造 `[a, b, ...]` / `[elem; LEN]`(求值为常量数组值,RXS-0063)。
- 以下构造在 const 求值上下文非法 `RX5003`:调用非 `const fn`、解引用裸指针、取可变借用后写入经引用的外部状态、I/O / 堆分配 / trait 方法调度(D-111 排除项)。
- const 求值上下文外,`const fn` 亦可作为普通函数被运行期调用(其 body 同时满足运行期 host 子集时)。

**Implementation Requirements**:const 求值在 MIR 解释器上实施(输入 = 该 body 的 MIR),作为 query 记忆化(纯函数纪律,D-203);求值时点早于单态化收集对 const 泛型实参的需求(RXS-0064)。

> 锚定测试:`conformance/consteval/const_fn_eval.rx`(正例);const_eval 单测。

### RXS-0063 const item 与表达式的求值规则

**Dynamic Semantics**(编译期求值,语义与等价运行期求值一致):

- **整数运算**:按操作数类型(RXS-0043)的位宽与有无符号实施二进制补码运算;运算结果**溢出**该类型可表示范围即求值失败 `RX5001`(const 上下文不做 wrapping,与运行期 UB 边界无关——const 求值溢出是确定的编译错误)。
- **布尔 / 比较运算**:`bool` 短路语义(`&&` / `||`)在 const 求值中保持;比较产出 `bool`。
- **分支与循环**:`if` / `match` 按求值后的判别值择臂;`loop` / `while` / `for` 按运行期等价语义迭代求值,直至终止。**不终止的 const 求值**(无法在有限步内收敛)为实现可施加步数上限的诊断面,M3.4 首版按 `RX5003`(非 const 可终止)保守报告,精度登记为已知限制(07 §4)。
- **数组构造**:`[a, b, ...]` 求值各元素为常量并组成定长常量数组值;`[elem; LEN]` 求值 `elem` 一次、`LEN` 为非负整数常量,产出 `LEN` 份副本的常量数组值。`LEN` 求值为负或超出 `usize` 范围即 `RX5001`。
- **const item 引用**:对 `const NAME` 的引用求值为其初始化器的常量值(经 query 记忆化,跨引用点共享);`const` item 间的引用不得构成求值环 → 环检测报 `RX5003`(非 const 可终止求值)。

**Legality**:

- const item 初始化器的求值结果类型必须与标注类型合一(RXS-0040),否则既有 `RX2001`(类型检查层,先于 const 求值)。
- const 求值产出的值经 MIR `Operand::Const` 落地;运行期使用该常量等价于直接写入字面量。

**Implementation Requirements**:求值器对每种 MIR rvalue / terminator 给出确定结果;失败经 `RX5001` / `RX5002` / `RX5003` 报告,span 指向触发求值失败的源构件。

> 锚定测试:`conformance/consteval/const_fn_eval.rx`、`conformance/consteval/const_arith_run.rx`(真跑);`tests/ui/consteval/` 溢出 snapshot;const_eval 单测(算术 / 分支 / 循环 / 数组求值)。

### RXS-0064 const 泛型参数与求值

**Legality**(const 泛型接入类型系统与单态化,RXS-0020 / RXS-0021 语法配套):

- `const` 泛型参数(`fn f<const N: usize>(...)` / `struct S<const N: usize>`)在签名内作为该类型的**值**参数;其类型必须为整数原生类型(M3.4 子集:`usize` 及整数型)。
- **实参求值**:调用 / 类型实例化处对 const 参数的实参(`f::<EXPR>()` / `S<EXPR>`)是 const 求值上下文(RXS-0062),按 RXS-0063 求值为常量;求值失败按 5xxx 报告。
- **数组长度作为类型组成**:`[T; N]` 的长度参与类型同一性——`[T; A]` 与 `[T; B]` 当且仅当 `A`、`B` 求值为同一常量时为同一类型;长度不一致即 `RX2001`(类型检查层)。const 泛型参数 `N` 在单态化实例内替换为其实参常量。
- **单态化收集**:含 const 泛型的 item 按 (类型实参, const 实参) 元组单态化收集;不同 const 实参产生不同单态实例(对齐 RXS-0045 类型实参单态化口径)。

**Implementation Requirements**:const 泛型实参在单态化前经 RXS-0062 query 求值;替换后的 const 值进入实例的类型表示(数组长度 / 值参数),供后续检查与(标量路径)codegen 使用。运行期数组实例化路径维持 `RX6001` 拦截(范围裁决:标量优先),不阻塞 const 求值与类型检查正确性。

**实现进度(M3.4 范围裁决)**:本条款语义为 D-111 已锁定决策的条款化(规范先行)。M3.4 交付 const eval MIR 解释器核心(RXS-0062/0063/0065,含 const item / const fn 标量求值真跑),const 泛型**值的运行期单态化**(`f::<N>()` turbofish 实参 → 实例值代入 + codegen)随 M4+ 接入,登记 [registry/deferred.json](../registry/deferred.json) **RD-007**(理由:turbofish 实参在 HIR 降级处丢弃、无 const 值的类型级表示、单态化 substs 为纯类型向量,跨层改造与 M3.4 标量优先预算不成比例,07 §4 保守先行)。本条款语义不随实现进度变更。

> 锚定测试:`conformance/consteval/const_generic_value.rx`(const 泛型值驱动标量结果,真跑);const_eval / typeck 单测(数组长度类型同一性 / const 实参单态化)。

### RXS-0065 const 求值失败语义

**Legality**(5xxx const eval 段位首批,07 §5 段位语义):

- **求值溢出** `RX5001`:const 求值期间整数运算结果超出操作数类型可表示范围,或数组长度求值为负 / 超 `usize`。
- **求值越界** `RX5002`:const 求值期间对常量数组 / 聚合的索引访问越界(常量索引 ≥ 常量长度)。
- **非 const 操作** `RX5003`:const 求值上下文出现 RXS-0062 子集外的操作(调用非 `const fn`、堆 / trait / I/O 操作、不可终止 / 环求值)。

**Implementation Requirements**:

- 检查时点:const 求值在 typeck 之后实施(类型已定),对全部 const 求值上下文强制;失败即停该上下文的求值并报告,不级联到无关上下文。
- 诊断 span 指向触发失败的源构件(溢出的运算 / 越界的索引 / 非 const 的调用);措辞允许保守粗糙(07 §4 先正确性后诊断,M3 契约 §2.2 诊断打磨排除项)。
- 操作数 / 类型为容忍区 `Err`(RXS-0047)时不触发 5xxx(不级联口径)。

> 锚定测试:`tests/ui/consteval/`(RX5001 溢出 / RX5002 越界 / RX5003 非 const snapshot);const_eval 单测(失败路径)。

---

## 错误码引用汇总

| 错误码 | 含义 | 条款 |
|---|---|---|
| RX2001 | 类型不匹配(引用:const 类型合一 / 数组长度同一性) | RXS-0063, RXS-0064 |
| RX5001 | const 求值溢出(整数运算溢出 / 数组长度越界) | RXS-0063, RXS-0065 |
| RX5002 | const 求值越界(常量索引 ≥ 常量长度) | RXS-0065 |
| RX5003 | 非 const 操作(子集外操作 / 不可终止 / 环求值) | RXS-0062, RXS-0063, RXS-0065 |
| RX6001 | codegen 暂不支持的语言构造(引用:运行期数组实例化,标量优先裁决) | RXS-0064 |

含义以 [../registry/error_codes.json](../registry/error_codes.json) 为唯一事实源,本表仅引用。RX5001 ~ RX5003 为 5xxx const eval 段位首批(07 §5 段位语义),**spec 先行引用,正式分配于 M3.4 实现 WP**(沿用 4xxx 在实现 PR 落 registry 的节奏,registry revision_log 留痕,编号不复用)。运行期数组的 `RX6001` 拦截是 M3.4 标量优先范围裁决的产物(D-111 最小面),登记为已知限制随 M4+ 只追加扩展。

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-13 | 初版:RXS-0062 ~ RXS-0065(M3.4 const eval 首批:const fn 子集与求值上下文 / const item 与表达式求值规则 / const 泛型参数与求值 / 求值失败语义;05 §9 const eval 与 const 泛型 D-111 已锁定决策的条款化,M3 契约 D-M3-5 spec 先行)。范围裁决标量优先:运行期数组 codegen 排除,登记已知限制(RX6001);错误码汇总表登记 RX5001~RX5003(spec 先行,实现 WP 正式分配) | Direct |
| v1.1 | 2026-06-13 | RXS-0064 追加"实现进度"注:M3.4 交付 const eval 解释器核心(RXS-0062/0063/0065 真跑),const 泛型值运行期单态化随 M4+ 接入(登记 RD-007);条款语义 0-byte(仅补实现进度留痕,非语义变更) | Direct |
