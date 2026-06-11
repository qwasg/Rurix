# Rurix 语言规范 — 词法结构

> 条款:RXS-0001 ~ RXS-0010(首批,M1.2)。体例见 [README.md](README.md)。
> 依据:05 §12 语法基调(D-114:Rust 系、GPU 关键字最小化、不做 Python 亲和);07 §5 诊断架构(错误码段位 0xxx = 词法/语法)。
> 本文为已选定决策的初版条款化(档位 Direct);任何偏离 05/13 已锁定决策的修改须按 10 §3 升档。

---

### RXS-0001 源文本

**Syntax**:源文件是合法 UTF-8 编码的 Unicode 码点序列。词法分析的输入单位是码点,输出是 token 序列(每 token 携带字节区间 span)。

**Legality**:

- 源文本必须是合法 UTF-8;非法字节序列在源载入边界拒绝(由实现的输入类型保证;载入通道错误码随 CLI 落地分配)。
- 源文本中出现 BOM(U+FEFF)或 NUL(U+0000)是词法错误 → `RX0001`。
- 在字符串字面量、字符字面量与注释**之外**,出现任何不能开始 token 的码点(含全部非 ASCII 码点)是词法错误 → `RX0001`。

**Implementation Requirements**:连续的非法码点应合并为单条诊断(避免对一段非 ASCII 文本逐码点报错)。

> 锚定测试:`src/rurixc/src/lexer.rs` 单测(BOM/NUL/非法字符/合并诊断)。

### RXS-0002 空白

**Syntax**:空白码点集 = { U+0020 空格, U+0009 水平制表, U+000D 回车, U+000A 换行 }。空白仅用于分隔 token,无语义。

**Legality**:无(其他 Unicode 空白码点不在集合内,按 RXS-0001 处理)。

> 锚定测试:lexer 单测(空白分隔/CRLF);conformance/syntax 全量样例。

### RXS-0003 注释

**Syntax**:

- 行注释:`//` 至行尾(不含换行符)。
- 块注释:`/*` 至配对的 `*/`,**允许嵌套**,嵌套深度配平。
- 注释内容允许任意合法 UTF-8 码点(不受 RXS-0001 非 ASCII 限制)。
- 注释等价于空白。doc 注释形态(`///` 等)延后追加(`rx doc` 随 M8,本条款只追加扩展)。

**Legality**:EOF 前未配平的块注释是词法错误 → `RX0002`(span 指向最外层 `/*`)。

> 锚定测试:lexer 单测(嵌套/未终结);`conformance/syntax/comments.rx`。

### RXS-0004 标识符

**Syntax**:`identifier = [A-Za-z_] [A-Za-z0-9_]*`。单独的 `_` 不是标识符,是独立 token(通配/占位)。

**Legality**:

- **MVP ASCII-only**:非 ASCII 码点不能构成标识符 → `RX0001`(RXS-0001)。放宽到 Unicode 标识符是 additive 变更,须新条款修订。
- 与关键字表(RXS-0005)冲突的字串按关键字处理。

> 锚定测试:lexer 单测;`conformance/syntax/idents_keywords.rx`。

### RXS-0005 关键字

**Syntax**:首批保留关键字表(只追加;来源:05 各语法草图 + D-102 着色关键字):

```
as  break  const  continue  device  else  enum  extern  false  fn
for  if  impl  in  kernel  let  loop  match  mod  move
mut  pub  return  shared  static  struct  trait  true  type  unsafe
use  where  while
```

**Legality**:

- 关键字不可用作标识符。
- 地址空间名 `global` / `constant` / `local` / `host`(05 §5)是**上下文关键字**:词法层按标识符产出,类型位置语义由 parser/类型检查赋予(M1.3+ 条款化)。
- `true` / `false` 是关键字形态的布尔字面量。

> 锚定测试:lexer 单测(全表逐一 + 上下文关键字按 Ident 产出);`conformance/syntax/idents_keywords.rx`。

### RXS-0006 整数字面量

**Syntax**:

```
int_lit     = dec_lit | hex_lit | oct_lit | bin_lit
dec_lit     = [0-9] [0-9_]*
hex_lit     = "0x" [0-9a-fA-F_]*
oct_lit     = "0o" [0-7_]*
bin_lit     = "0b" [01_]*
int_suffix  = "i8"|"i16"|"i32"|"i64"|"u8"|"u16"|"u32"|"u64"|"usize"
```

`_` 为可读性分隔符,可出现在数字体任意位置(不可作首字符——首字符为 `_` 时按标识符 lex)。后缀紧跟数字体,无空白。

**Legality**:

- `0x`/`0o`/`0b` 后数字体为空(仅 `_` 亦视为空)→ `RX0006`。
- 进制外数字(如 `0b12`、`0o9`)→ `RX0006`。
- 后缀不在 `int_suffix` ∪ `float_suffix`(RXS-0007)内 → `RX0007`。整数体 + 浮点后缀(如 `1f32`)合法,产出浮点字面量。
- 数值超出后缀类型范围不是词法错误(由 const eval/类型检查层裁决,条款随 M2+)。

> 锚定测试:lexer 单测(四进制/分隔符/空体/坏数字/坏后缀);`conformance/syntax/literals_int.rx`。

### RXS-0007 浮点字面量

**Syntax**:

```
float_lit    = dec_lit "." dec_body? exponent? float_suffix?
             | dec_lit exponent float_suffix?
             | dec_lit float_suffix
dec_body     = [0-9] [0-9_]*
exponent     = ("e"|"E") ("+"|"-")? dec_body
float_suffix = "f32" | "f64"
```

消歧规则:`dec_lit` 后的 `.` 仅当后随码点**不是** `.`、不是标识符起始码点时才归入浮点字面量(`1..2` 是整数与区间运算符;`1.foo()` 是整数与方法调用;`1.` 是浮点)。

**Legality**:

- 指数部分缺数字(`1e`、`1e+`)→ `RX0006`。
- 浮点字面量带整数后缀(如 `1.5i32`)或未知后缀 → `RX0007`。
- `f16` / `bf16` 后缀延后追加(05 §2.1 一等类型,字面量后缀形态随库面定型,只追加)。

> 锚定测试:lexer 单测(消歧三例/指数/坏后缀);`conformance/syntax/literals_float.rx`。

### RXS-0008 字符、字符串字面量与生命周期标记

**Syntax**:

```
char_lit    = "'" (escape | 非 {'\', '\n', '''} 码点) "'"
string_lit  = '"' (escape | 非 {'\', '"'} 码点)* '"'
escape      = "\n"|"\r"|"\t"|"\\"|"\'"|"\""|"\0"
            | "\x" hex hex            (范围 0x00–0x7F)
            | "\u{" hex{1,6} "}"      (合法 Unicode 标量值)
lifetime    = "'" identifier
```

- 字符串/字符内容允许任意合法 UTF-8 码点(不受 RXS-0001 限制);字符串内允许字面换行。raw string 形态延后追加。
- 消歧:`'` 后随标识符起始码点、且其后不是 `'`,产出生命周期标记(`'ctx`);否则按字符字面量解析(`'c'`)。

**Legality**:

- EOF 前未闭合的字符串 → `RX0003`(span 自起始引号)。
- 空字符字面量(`''`)、多码点字符字面量、未终结字符字面量(含跨行)→ `RX0004`。
- 非法转义序列(未知转义名、`\x` 超界或缺位、`\u{}` 非法标量值或位数超限)→ `RX0005`(字面量整体仍产出 token,错误恢复见 RXS-0010)。

> 锚定测试:lexer 单测(转义全集/未终结/消歧);`conformance/syntax/literals_string.rx`、`literals_char.rx`、`lifetimes.rx`。

### RXS-0009 标点与运算符

**Syntax**:首批 token 表(只追加),按**最长匹配**lex:

```
( ) [ ] { } , ; : :: -> => . .. ..= ? @ # _
= == != < > <= >= + - * / % ! && || & | ^ << >>
+= -= *= /= %= &= |= ^= <<= >>=
```

**Implementation Requirements**:`>>` 等复合 token 按最长匹配产出;泛型嵌套闭合(`Vec<Vec<T>>`)的拆分是 parser 职责(M1.3 条款化),词法层不做上下文消歧。

> 锚定测试:lexer 单测(全表 + 最长匹配边界);`conformance/syntax/operators.rx`。

### RXS-0010 词法错误恢复

**Implementation Requirements**:

1. 词法错误**不终止**词法分析:报告诊断后跳过违例码点(或按字面量条款产出占位 token)继续,单文件可产出多条词法诊断。
2. 全部词法诊断必须携带精确 span 与 `RX000x` 错误码,经 DiagCtxt 产出(emit-or-cancel,07 §5)。
3. 出错文件仍产出可供 parser 消费的 token 流(以 EOF token 收尾)。

> 锚定测试:lexer 单测(单文件多错误恢复)。

---

## 错误码引用汇总

| 错误码 | 含义 | 条款 |
|---|---|---|
| RX0001 | 非法字符 | RXS-0001 / RXS-0004 |
| RX0002 | 未终结块注释 | RXS-0003 |
| RX0003 | 未终结字符串字面量 | RXS-0008 |
| RX0004 | 非法字符字面量 | RXS-0008 |
| RX0005 | 非法转义序列 | RXS-0008 |
| RX0006 | 非法数字字面量 | RXS-0006 / RXS-0007 |
| RX0007 | 非法字面量后缀 | RXS-0006 / RXS-0007 |

含义以 [../registry/error_codes.json](../registry/error_codes.json) 为唯一事实源,本表仅引用。

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-11 | 初版:RXS-0001 ~ RXS-0010(05 §12 / D-114 已选定决策的条款化,M1 契约 D-M1-2) | Direct |
