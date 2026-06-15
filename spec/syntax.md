# Rurix 语言规范 — 语法结构

> 条款:RXS-0011 ~ RXS-0031(首批,M1.3)。体例见 [README.md](README.md)。
> 依据:05 §12 语法基调(D-114:Rust 系、GPU 关键字最小化);05 §1/§5(函数着色 D-102、`shared let`);07 §1(手写递归下降、AST 贴近用户语法 D-202);10 §5(feature gate)。
> 本文为已选定决策的初版条款化(档位 Direct);任何偏离 05/13 已锁定决策的修改须按 10 §3 升档。
> 记法:产生式为 EBNF 式;终结符引用词法条款(RXS-0001 ~ RXS-0010)的 token;`?` 可选、`*` 零或多、`+` 一或多、`( … )` 分组、`|` 择一。

---

### RXS-0011 源文件与 item 序列

**Syntax**:

```
source_file = inner_attr* item*
item        = outer_attr* visibility? bare_item
bare_item   = fn_item | struct_item | enum_item | trait_item | impl_item
            | mod_item | use_item | static_item | const_item | type_alias
            | extern_block
```

**Legality**:

- 源文件顶层只允许 item;顶层出现表达式或语句是语法错误 → `RX0008`。
- 内部属性(`#!`,RXS-0012)只允许出现在源文件起始处(任何 item 之前);其他位置 → `RX0008`。

> 锚定测试:`conformance/syntax/hello_world.rx`;parser 单测(顶层非 item 报错)。

### RXS-0012 属性

**Syntax**:

```
inner_attr  = "#" "!" "[" meta_item "]"
outer_attr  = "#" "[" meta_item "]"
meta_item   = path
            | path "(" meta_seq? ")"
            | path "=" literal
meta_seq    = meta_inner ("," meta_inner)* ","?
meta_inner  = meta_item | literal
```

- 外部属性可前置于 item(RXS-0011)与表达式(RXS-0026 基本表达式之前)。
- 内部属性仅文件顶部(RXS-0011);MVP 已定语义的内部属性仅 `feature(...)`(RXS-0031)。
- 属性语义(`derive` / `repr` / `link` / `export` 等)由后续语义层条款裁决;语法层只规定 meta 形态。

**Legality**:meta 形态之外的属性内容(如裸运算符序列)是语法错误 → `RX0008`。

> 锚定测试:`conformance/syntax/struct_def.rx`(derive)、`ffi_extern.rx`(repr/link)、`export_c.rx`(export)、`attrs_meta.rx`。

### RXS-0013 路径与可见性

**Syntax**:

```
path         = path_segment ("::" path_segment)*
path_segment = identifier generic_args?            (类型位置)
             | identifier ("::" generic_args)?     (表达式位置,turbofish)
visibility   = "pub" ("(" "package" ")")?
```

- `Self`(标识符形态)是普通路径段,语法层不特殊化。
- 表达式位置的泛型实参必须经 turbofish(`::<…>`)引入;类型位置直接跟随 `<…>`(RXS-0021)。

**Legality**:`pub(…)` 括号内仅允许 `package`(05 §10 可见性层级首批);其他内容 → `RX0008`。

> 锚定测试:`conformance/syntax/modules_use.rx`(pub(package))、`fn_generics.rx`(turbofish)。

### RXS-0014 函数项与函数着色

**Syntax**:

```
fn_item      = fn_qualifier? "fn" identifier generic_params? "(" fn_params? ")"
               ret_ty? where_clause? (block_expr | ";")
fn_qualifier = "kernel" | "device" | "const"
fn_params    = fn_param ("," fn_param)* ","?
fn_param     = outer_attr* ("mut"? "self_param" | pattern ":" type)
self_param   = "self" 形态:identifier 文本为 "self",可前置 "&" lifetime? "mut"?
ret_ty       = "->" type
```

函数四色(D-102):无限定符 = 宿主函数;`kernel fn` = GPU 入口;`device fn` = 设备侧可调用;`const fn` = 编译期可求值。着色语义(可调用方向、执行形状参数等)随 M2+ 语义条款;本条款只规定语法形态。

**Legality**:

- 限定符至多一个;`kernel const fn` 等组合是语法错误 → `RX0008`。
- 函数体缺失(`;` 结尾)仅在 `extern` 块(RXS-0019)与 `trait` 体(RXS-0016)内合法;其他位置 → `RX0008`。
- `self` 参数只允许出现在参数列表首位 → 违例 `RX0008`。

> 锚定测试:`conformance/syntax/fn_basic.rx`、`kernel_fn.rx`、`device_fn.rx`、`const_generics.rx`(const fn)。

### RXS-0015 struct 与 enum

**Syntax**:

```
struct_item    = "struct" identifier generic_params? where_clause? struct_body
struct_body    = "{" field_defs? "}" | "(" tuple_fields? ")" ";" | ";"
field_defs     = field_def ("," field_def)* ","?
field_def      = outer_attr* visibility? identifier ":" type
tuple_fields   = tuple_field ("," tuple_field)* ","?
tuple_field    = outer_attr* visibility? type
enum_item      = "enum" identifier generic_params? where_clause? "{" variants? "}"
variants       = variant ("," variant)* ","?
variant        = outer_attr* identifier variant_body?
variant_body   = "{" field_defs? "}" | "(" tuple_fields? ")"
```

**Legality**:具名字段重名、变体重名不是语法错误(名称解析层裁决,M2)。

> 锚定测试:`conformance/syntax/struct_def.rx`、`enum_match.rx`。

### RXS-0016 trait 与 impl

**Syntax**:

```
trait_item   = "trait" identifier generic_params? where_clause? "{" assoc_item* "}"
impl_item    = "impl" generic_params? type ("for" type)? where_clause? "{" assoc_item* "}"
assoc_item   = outer_attr* visibility? (fn_item | assoc_type | const_item)
assoc_type   = "type" identifier (":" bounds)? ("=" type)? ";"
```

- `impl Trait for Type` 中 `Trait` 按类型文法解析(路径类型形态);trait/类型角色由名称解析层裁决。
- trait 体内的 `fn_item` 允许以 `;` 结尾(签名声明);impl 体内必须有函数体。

**Legality**:impl 体内出现无体函数 → `RX0008`。

> 锚定测试:`conformance/syntax/trait_impl.rx`。

### RXS-0017 mod 与 use

**Syntax**:

```
mod_item = "mod" identifier "{" item* "}"
use_item = "use" path ("as" identifier)? ";"
```

外部文件模块(`mod name;`)、use 组(`use a::{b, c}`)、glob(`use a::*`)延后追加(05 §10 模块面随 M2 定型,只追加扩展)。

**Legality**:`use` 路径中的段不携带泛型实参 → 违例 `RX0008`。

> 锚定测试:`conformance/syntax/modules_use.rx`、`use_alias.rx`。

### RXS-0018 static、const 与 type 别名

**Syntax**:

```
static_item = "static" "mut"? identifier ":" type "=" expr ";"
const_item  = "const" identifier ":" type "=" expr ";"
type_alias  = "type" identifier generic_params? "=" type ";"
```

**Legality**:`const` 项必须有显式类型与初始化器 → 违例 `RX0008`。

> 锚定测试:`conformance/syntax/static_const_items.rx`。

### RXS-0019 extern 块

**Syntax**:

```
extern_block = "extern" string_lit "{" extern_member* "}"
extern_member = outer_attr* visibility? fn_item    (必须以 ";" 结尾,无函数体)
```

**Legality**:

- ABI 字符串首批仅 `"C"`(05 §11,D-113);其他 ABI 名是语法层可接受、语义层延后裁决(本条款只追加)。
- extern 块内出现带函数体的 fn → `RX0008`。

> 锚定测试:`conformance/syntax/ffi_extern.rx`。

### RXS-0020 泛型参数与 where 子句

**Syntax**:

```
generic_params = "<" generic_param ("," generic_param)* ","? ">"
generic_param  = lifetime
               | identifier (":" bounds)? ("=" type)?
               | "const" identifier ":" type
bounds         = bound ("+" bound)*
bound          = lifetime | path(类型位置)
where_clause   = "where" where_pred ("," where_pred)* ","?
where_pred     = type ":" bounds
```

**Legality**:`const` 泛型参数必须带类型标注 → 违例 `RX0008`。

> 锚定测试:`conformance/syntax/fn_generics.rx`、`const_generics.rx`、`lifetimes.rx`。

### RXS-0021 泛型实参与 `>>` 拆分

**Syntax**:

```
generic_args = "<" generic_arg ("," generic_arg)* ","? ">"
generic_arg  = lifetime | type | const_arg
const_arg    = int_lit | "-" int_lit | "{" expr "}"
```

- 路径形态的 const 实参(如 `TILE`)经类型文法覆盖(路径类型),语法层不区分类型/const 角色(名称解析层裁决,M2)。

**Implementation Requirements**:泛型实参闭合位置的 `>>`(RXS-0009 最长匹配产出的 Shr token)必须由 parser 拆分为两个 `>` 消费(`Vec<Vec<T>>`、`module.kernel::<tile_gemm<32>>()`);同理 `>=`、`>>=` 在该位置拆分出 `>`。拆分后剩余部分作为独立 token 继续参与解析。

> 锚定测试:`conformance/syntax/fn_generics.rx`(嵌套 `>>` / turbofish)、`launch_api.rx`(turbofish 内嵌套泛型)。

### RXS-0022 类型

**Syntax**:

```
type        = path(类型位置)                       (路径类型,含 Self / 泛型实参)
            | "&" lifetime? "mut"? type            (引用类型)
            | "*" ("const" | "mut") type           (裸指针类型)
            | "(" ")"                              (单元类型)
            | "(" type "," (type ("," type)*)? ","? ")"   (元组类型,单元素必须带尾逗号)
            | "(" type ")"                         (括号分组)
            | "[" type ";" expr "]"                (数组类型)
            | "[" type "]"                         (切片类型)
            | "fn" "(" (type ("," type)* ","?)? ")" ret_ty?   (fn 指针类型)
            | "_"                                  (推断占位)
            | int_lit                              (类型位置 const 实参形态,RXS-0021)
```

- 地址空间名 `global` / `constant` / `local` / `host` 是上下文关键字(RXS-0005):在类型位置按路径类型(标识符)解析,地址空间角色由语义层赋予(05 §5,M2+ 条款)。形如 `View<global, f32, (N,)>` 的实参在语法层一律按类型/const 实参解析。
- 类型位置的整数字面量仅为承载 shape 元组(`(1024,)`、`Grid<(64, 64)>`)等 const 实参形态;其在非 const 实参语境的合法性由语义层裁决。

**Legality**:`*` 后缺 `const` / `mut` 限定 → `RX0008`。

> 锚定测试:`conformance/syntax/kernel_fn.rx`(shape 元组/视图类型)、`ffi_extern.rx`(裸指针)、`operators.rx`(fn 指针)、`shared_let.rx`(嵌套数组)。

### RXS-0023 模式

**Syntax**:

```
pattern        = pattern_no_alt
pattern_no_alt = literal_pat ( ("..=" | "..") literal_pat )?   (字面量与范围模式)
               | identifier "@" pattern_no_alt                 (绑定模式)
               | "mut"? identifier                             (标识符绑定)
               | "_"                                           (通配)
               | "&" "mut"? pattern_no_alt                     (引用模式)
               | "(" (pattern ("," pattern)* ","?)? ")"        (元组模式)
               | "[" (pattern ("," pattern)* ","?)? "]"        (切片模式)
               | path                                          (单元变体/常量)
               | path "(" (pattern ("," pattern)* ","?)? ")"   (元组结构体模式)
               | path "{" field_pats? "}"                      (结构体模式)
field_pats     = field_pat ("," field_pat)* (","  "..")? ","? | ".."
field_pat      = identifier (":" pattern)?
literal_pat    = "-"? int_lit | "-"? float_lit | str_lit | char_lit | "true" | "false"
```

- 单段小写路径与标识符绑定的歧义按标识符绑定解析(名称解析层重分类,M2;与 Rust 一致)。
- `..`(rest)模式首批仅允许在结构体模式尾部(`Particle { pos, .. }`)。

**Legality**:范围模式两端必须是字面量形态 → 违例 `RX0008`。

> 锚定测试:`conformance/syntax/patterns_let.rx`、`enum_match.rx`(范围/`@`/guard 配套)。

### RXS-0024 语句与块

**Syntax**:

```
block_expr = "{" stmt* tail_expr? "}"
stmt       = item                                  (item 语句)
           | let_stmt
           | "shared" let_stmt                     (shared let,05 §5)
           | expr_stmt
           | ";"                                   (空语句)
let_stmt   = "let" pattern (":" type)? ("=" expr)? ";"
expr_stmt  = expr_with_block ";"?                  (块尾语句的分号可省)
           | expr_without_block ";"
tail_expr  = expr                                  (块值,无 ";")
```

- `expr_with_block` = 块、`if`、`while`、`for`、`loop`、`match`、`unsafe` 块(RXS-0026/0028/0029);其作语句时分号可省略。
- 非块表达式语句必须以 `;` 结尾;块内最后一个无 `;` 的非块表达式是尾表达式(块的值)。

**Legality**:`shared let` 的语义合法位置(仅 kernel/device 体内)由语义层裁决(M2+);语法层在任何块内接受。

> 锚定测试:`conformance/syntax/control_flow.rx`(尾表达式/语句混排)、`shared_let.rx`。

### RXS-0025 表达式运算符与优先级

**Syntax**:二元/一元运算符按下表优先级解析(自高到低;同级按左结合,除注明者):

| 级 | 运算符 | 结合性 |
|---|---|---|
| 1 | 后缀:调用 `()`、索引 `[]`、字段/方法 `.`、`?` | 左 |
| 2 | 一元前缀:`-` `!` `*`(解引用)`&` `&mut` | — |
| 3 | `as` 类型转换 | 左 |
| 4 | `*` `/` `%` | 左 |
| 5 | `+` `-` | 左 |
| 6 | `<<` `>>` | 左 |
| 7 | `&` | 左 |
| 8 | `^` | 左 |
| 9 | `\|` | 左 |
| 10 | `==` `!=` `<` `>` `<=` `>=` | **不可链式** |
| 11 | `&&` | 左 |
| 12 | `\|\|` | 左 |
| 13 | `..` `..=`(区间) | **不可链式**;两操作数均可缺省(`a..`、`..b`、`..`)延后追加,首批要求双操作数或 for 头部使用 |
| 14 | `=` `+=` `-=` `*=` `/=` `%=` `&=` `\|=` `^=` `<<=` `>>=` | 右 |

**Legality**:

- 比较与区间运算符不可链式:`a < b < c`、`a .. b .. c` 是语法错误 → `RX0008`(诊断应提示加括号)。
- 赋值左侧的"位置表达式"合法性由语义层裁决;语法层接受任意表达式。

> 锚定测试:`conformance/syntax/operators.rx`、`expr_precedence.rx`。

### RXS-0026 基本表达式

**Syntax**:

```
primary_expr = outer_attr* primary_core
primary_core = literal                            (int/float/str/char/true/false)
             | path(表达式位置)                    (变量/常量/单元变体;turbofish 见 RXS-0013)
             | path "{" struct_fields? "}"        (结构体字面量)
             | "(" ")"                            (单元值)
             | "(" expr ")"                       (分组)
             | "(" expr "," (expr ("," expr)*)? ","? ")"  (元组)
             | "[" (expr ("," expr)* ","?)? "]"   (数组)
             | "[" expr ";" expr "]"              (重复数组)
             | block_expr                         (块)
             | "unsafe" block_expr                (unsafe 块)
             | "move"? closure_expr               (闭包,feature gate 后,RXS-0031)
             | if_expr | while_expr | for_expr | loop_expr | match_expr
             | "return" expr? | "break" expr? | "continue"
struct_fields = struct_field ("," struct_field)* ","?
struct_field  = identifier (":" expr)?            (缺 ":" 为简写)
closure_expr  = "|" (closure_param ("," closure_param)*)? "|" expr
closure_param = pattern (":" type)?
```

**Legality**:

- **结构体字面量限制**:`if` / `while` 条件、`for` 迭代器、`match` 被匹配表达式位置不允许裸结构体字面量(`if x == S { … }` 的 `{` 归属条件体);需括号包裹。违例按块解析,产生的错误 → `RX0008`。
- `return` / `break` 的操作数表达式可缺省;`continue` 不带操作数。标签(`'label:`)延后追加。

> 锚定测试:`conformance/syntax/struct_def.rx`(结构体字面量/简写)、`literals_int.rx` 等字面量族、`unsafe_block.rx`、`operators.rx`(属性前缀表达式)。

### RXS-0027 调用、字段访问与后缀表达式

**Syntax**:

```
postfix_expr = primary_expr postfix*
postfix      = "(" (expr ("," expr)* ","?)? ")"            (调用)
             | "[" expr "]"                                 (索引)
             | "." identifier ("::" generic_args)? call_args?  (字段访问 / 方法调用,turbofish 可选)
             | "." int_lit                                  (元组字段访问)
             | "?"                                          (错误传播)
call_args    = "(" (expr ("," expr)* ","?)? ")"
```

- `.` 后随标识符且紧跟实参表(或 turbofish + 实参表)为方法调用;无实参表为字段访问。
- `expr as type` 转换见 RXS-0025 优先级表(级 3)。

> 锚定测试:`conformance/syntax/closures_and_calls.rx`(链式调用/索引/as)、`vec_mat_swizzle.rx`(swizzle 字段/方法)、`error_handling.rx`(`?` 链)、`views_ops.rx`(方法 turbofish)。

### RXS-0028 控制流表达式

**Syntax**:

```
if_expr    = "if" expr block_expr ("else" (if_expr | block_expr))?
while_expr = "while" expr block_expr
for_expr   = "for" pattern "in" expr block_expr
loop_expr  = "loop" block_expr
```

- 条件/迭代器位置受结构体字面量限制(RXS-0026)。
- `if` 作语句不强制 `else` 分支;`if` 作值(尾表达式/let 初始化器)缺 `else` 的类型合法性由类型层裁决(M2),语法层接受。
- `while let` / `if let` 形态延后追加(05 草图未含,只追加扩展)。

> 锚定测试:`conformance/syntax/control_flow.rx`。

### RXS-0029 match 表达式

**Syntax**:

```
match_expr = "match" expr "{" match_arms? "}"
match_arms = match_arm ("," match_arm)* ","?
match_arm  = outer_attr* arm_pats guard? "=>" expr
arm_pats   = pattern ("|" pattern)*               (顶层 or-模式)
guard      = "if" expr
```

- 被匹配表达式位置受结构体字面量限制(RXS-0026)。
- 臂体为块表达式时,臂间 `,` 可省略;非块臂体之间必须以 `,` 分隔(末臂尾逗号可选)。

**Legality**:空臂列表(`match e {}`)语法合法;穷尽性由语义层裁决(M2+)。

> 锚定测试:`conformance/syntax/enum_match.rx`(guard/`@`/范围)、`match_or_patterns.rx`。

### RXS-0030 语法错误恢复

**Implementation Requirements**:

1. 语法错误**不终止**解析:报告诊断后跳至最近同步点继续,单文件可产出多条语法诊断,并仍产出部分 AST(出错子树以错误节点占位)。
2. 同步点(anchor 集)至少包含:item 起始 token(`fn` / `kernel` / `device` / `const` / `struct` / `enum` / `trait` / `impl` / `mod` / `use` / `static` / `type` / `extern` / `pub` / `#`)、语句终止符 `;`、块闭合 `}` 与 EOF。块/括号内恢复不得越过当前闭合定界符所在层级。
3. 全部语法诊断必须携带精确 span 与 `RX00xx` 错误码,经 DiagCtxt 产出(emit-or-cancel,07 §5)。
4. EOF 前未闭合的 `(` / `[` / `{` → `RX0009`(span 指向未闭合的开定界符)。
5. parser 内部以事件流(节点开始/结束/token 消费)驱动 AST 构造;**RD-004(M6.4)已接通**:事件流经 `lossless` 模块组装 rowan 式无损语法树,供 LSP offset 映射与 IDE 查询消费(07 §9)。

> 锚定测试:parser 单测(单文件多错误恢复/未闭合定界符);`src/rurixc/src/lossless.rs`(事件流 → 树 → offset round-trip,RXS-0030 第 5 条)。

### RXS-0031 feature gate

**Syntax**:`#![feature(name (, name)*)]`(内部属性,RXS-0012;仅文件顶部)。

**Legality**:

- gate 名必须在实现的 gate 注册表内;未知 gate 名 → `RX0011`。
- 使用 gated 语法而未启用对应 gate → `RX0010`(诊断须给出 gate 名与启用方式)。
- 首批 gate 注册表(只追加):`closures`——闭包表达式(RXS-0026 `closure_expr`)。
- gate 的生命周期(实验 → 稳定化 → 移除 gate)遵循 10 §5;稳定化时本条款追加记录,gate 名永不复用。

> 锚定测试:`conformance/syntax/feature_gate_closures.rx`(启用后正例);feature gate 单测(未启用报错/未知 gate)。

---

## 错误码引用汇总

| 错误码 | 含义 | 条款 |
|---|---|---|
| RX0008 | 语法错误(期待 X 实得 Y) | RXS-0011 ~ RXS-0029 |
| RX0009 | 未闭合定界符 | RXS-0030 |
| RX0010 | 使用未启用的 gated 语法 | RXS-0031 |
| RX0011 | 未知 feature gate 名 | RXS-0031 |

含义以 [../registry/error_codes.json](../registry/error_codes.json) 为唯一事实源,本表仅引用。

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-11 | 初版:RXS-0011 ~ RXS-0031(05 §12 / D-114、05 §1 / D-102、07 §1 / D-202 已选定决策的条款化,M1 契约 D-M1-3) | Direct |
| v1.1 | 2026-06-15 | RXS-0030 第 5 条更新:RD-004 无损语法树通道 M6.4 接通(`src/rurixc/src/lossless.rs`);锚定测试扩列 lossless offset round-trip | Direct |
