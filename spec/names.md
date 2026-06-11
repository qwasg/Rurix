# Rurix 语言规范 — 名称与模块语义

> 条款:RXS-0032 ~ RXS-0038(首批,M2.1)。体例见 [README.md](README.md)。
> 依据:05 §2/§9/§10(trait 与泛型边界、模块与包系统 D-112、Result/`?` 的语义载体);07 §1/§2(HIR 与 query 边界 D-202/D-203);10 §4(spec 领导实现)。
> 本文为已选定决策的初版条款化(档位 Direct);任何偏离 05/07/13 已锁定决策的修改须按 10 §3 升档。

---

### RXS-0032 名称引入与定义域

**Syntax**:名称引入形态沿用 [syntax.md](syntax.md) 的 item / `use` / 泛型参数 / 模式绑定语法;本条款仅规定这些形态在何处向名称环境引入绑定。

**Legality**:

- 下列构造会引入名称:
  - module item: `fn` / `struct` / `enum` / `trait` / `impl` 关联项 / `mod` / `type` / `const` / `static`
  - `use` 声明引入的导入名(默认取末段名,显式别名取 `as` 后名称)
  - 泛型参数(`T`, `const N`, 生命周期留待后续条款)
  - 局部绑定:函数参数、`let` 模式中的具名绑定、`match`/`if let`/`while let` 模式绑定
- module item 与 `use` 在其所属模块作用域内引入名称;局部绑定在最内层块作用域内引入名称。
- `_` 不是可引用名称,不会向任何作用域引入绑定。

**Implementation Requirements**:

- 名称解析实现必须把 item 作用域与 body 内局部作用域分离建模,为 HIR 的 item/body 分离提供稳定边界(D-202)。

> 锚定测试:`conformance/syntax/names_module_scope.rx`。

### RXS-0033 词法作用域与遮蔽

**Legality**:

- 块作用域按词法嵌套生效:内层块新引入的局部绑定可遮蔽外层局部绑定。
- 函数参数与函数体内 `let` 绑定处于同一 body 名称体系;内层块可遮蔽参数名,同一层块内不可重复定义同名具名绑定 → `RX1002`。
- 模块 item 与 `use` 引入名在同一模块作用域内自声明点起可见;解析不依赖源码先后顺序,但循环引用的可接受性由后续类型/值语义条款决定。

**Implementation Requirements**:

- 单段路径在表达式位置解析时,局部绑定优先于同名模块级 item / import。

> 锚定测试:`conformance/syntax/names_module_scope.rx`、`conformance/syntax/names_path_priority.rx`。

### RXS-0034 路径解析优先级

**Legality**:

- 单段路径在表达式位置按如下顺序解析:
  1. 最内层局部绑定;
  2. 当前函数/项的泛型参数;
  3. 当前模块作用域中的 item 或 `use` 引入名。
- 单段路径在类型位置优先解析为泛型参数,其次为当前模块作用域中的类型级 item / `use` 引入名。
- 多段路径以首段名称定位起始模块项或导入名;后续段逐层在对应模块或关联项命名空间中继续解析。局部绑定不能作为多段路径前缀。
- 语法层允许的歧义形态(如单段小写路径)按本条款在名称解析层重分类;无法归类时 → `RX1001`。

**Implementation Requirements**:

- 解析器不得提前固化单段路径的“局部变量/项路径”角色;该裁决属于名称解析层职责。

> 锚定测试:`conformance/syntax/names_path_priority.rx`。

### RXS-0035 `use` 声明与别名

**Legality**:

- `use p::q::r;` 在当前模块作用域内引入名称 `r`;`use p::q::r as alias;` 引入名称 `alias`。
- `use` 的被导入目标必须解析到模块项命名空间中的公开名称;解析失败或目标类别不合法 → `RX1004`。
- `use` 只向所在模块引入绑定,不会把导入目标“重新注入”到局部块作用域。
- 同一 `use` 声明中显式别名优先于默认末段名;别名参与与普通 item 相同的冲突裁决(RXS-0037)。

**Implementation Requirements**:

- 名称解析实现必须在 HIR 中保留 `use` 的已解析目标与最终导出名,避免后续 query 重新做文本级回溯。

> 锚定测试:`conformance/syntax/names_use_visibility.rx`、`conformance/syntax/use_alias.rx`。

### RXS-0036 可见性与模块边界

**Legality**:

- 可见性默认私有。
- `pub` 名称可在包外被导出;`pub(package)` 名称仅在当前 package 内可见。
- 可见性适用于模块 item 与 struct/tuple field;其精确导出面由名称解析与后续元数据导出阶段共同实现。
- 解析到存在但不可见的名称时,报可见性违例 → `RX1003`,而不是“未解析名称”。

**Implementation Requirements**:

- 名称解析必须区分“名称不存在”与“名称存在但不可见”两类失败路径,以支撑结构化诊断与后续 IDE 查询。

> 锚定测试:`conformance/syntax/names_use_visibility.rx`、`conformance/syntax/visibility_levels.rx`。

### RXS-0037 重名与冲突裁决

**Legality**:

- 同一模块作用域内,两个非 `_` 名称若向同一命名类引入相同文本名,则构成重复定义 → `RX1002`。
- 同一层块作用域内,两个具名局部绑定若文本名相同,则构成重复定义 → `RX1002`;内层块遮蔽外层块不视为错误。
- `use` 引入名与现有 item / import / 局部绑定在同一作用域内同名时,按重复定义处理 → `RX1002`。
- struct 具名字段、enum 变体、trait / impl 关联项的同名冲突在其各自声明域内按重复定义处理 → `RX1002`。

**Implementation Requirements**:

- 重复定义诊断至少指出“先定义处”和“再次定义处”两个 span。

> 锚定测试:`conformance/syntax/names_duplicates.rx`。

### RXS-0038 名称解析诊断要求

**Legality**:

- 名称解析阶段的首批结构化错误码为:
  - `RX1001`: 未解析名称
  - `RX1002`: 重复定义
  - `RX1003`: 可见性违例
  - `RX1004`: 非法 `use` 目标

**Implementation Requirements**:

- `RX1001` 诊断应携带未解析文本名;对单段路径优先建议同作用域相近拼写。
- `RX1002` 诊断应携带至少两个主/辅 span(首次定义与冲突处)。
- `RX1003` 诊断应优先引用被访问实体的定义处与其声明的可见性。
- `RX1004` 诊断应区分“目标不存在”和“目标类别不允许导入”。

> 锚定测试:`conformance/syntax/names_duplicates.rx`、`conformance/syntax/names_use_visibility.rx`。

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-11 | 初版:RXS-0032 ~ RXS-0038(05 §10 D-112、07 §1/§2 D-202/D-203 已锁定决策的条款化,M2.1 names 部分) | Direct |
