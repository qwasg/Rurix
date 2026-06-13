# Rurix 语言规范 — 所有权与借用语义(M3 首批:desugar / 穷尽性 / drop scope)

> 条款:RXS-0048 ~ RXS-0056(RXS-0048 ~ 0052 = M3.1 首批;RXS-0053 ~ 0056 = M3.2 move/init/drop 执行语义;借用/生命周期主体随 M3.3 追加)。体例见 [README.md](README.md)。
> 依据:05 §3 §4(host 所有权 D-105 / affine 资源与 Drop)、05 §8(Result 错误处理 D-110);07 §1(IR 四层与 TBIR 窄门 D-202);M2_PLAN v1.1/v1.2/v1.3(for/`?` desugar 推迟留痕的 M3 收口);M3 契约 D-M3-1/D-M3-2。
> 本文为已选定决策的初版条款化(档位 Direct);任何偏离 05/07/13 已锁定决策的修改须按 10 §3 升档。
> desugar 条款(RXS-0049/RXS-0050)以**等价源形式**定义语义:实现可在任何 IR 层完成展开,但展开结果的静态与动态语义必须与给出的等价形式一致。

---

### RXS-0048 编译器已知项最小面

**Legality**:

- 实现必须内建识别以下**编译器已知项**,并注入 prelude 作用域(无需 `use` 即可用,变体名可不带路径前缀):
  - `enum Option<T> { None, Some(T) }`
  - `enum Result<T, E> { Ok(T), Err(E) }`
- 二者是普通 enum:构造、模式匹配、泛型单态化规则照常适用(RXS-0044 / RXS-0045),不附加特殊类型规则。
- 用户在模块或块作用域定义同名项时,按常规作用域规则遮蔽 prelude 项(对齐 RXS-0033 遮蔽语义),**不构成**重复定义 `RX1002`。
- 迭代器协议是**形状约定**而非 trait:类型 `I` 可被 `for` 迭代,当且仅当 `I` 具有 inherent 方法 `fn next(&mut self) -> Option<T>`(按 RXS-0046 查找)。不存在 `Iterator` trait 求解(D-104 单态化子集口径);trait 形态的迭代器协议随 trait 求解条款化(M4+)只追加扩展。
- **不开放用户自定义 lang-item 标注**;本最小面仅服务 desugar(RXS-0049 / RXS-0050),凡扩大该面的诉求按 10 §3 升档(M3 契约风险条款)。

**Implementation Requirements**:desugar 展开中对 `Option` / `Result` 变体的引用必须绑定到内建项本体,**不受用户同名遮蔽影响**(展开是实现内部行为,不经名称文本重解析)。

> 锚定测试:`conformance/desugar/option_result_prelude.rx`;resolve/lower 单测。

### RXS-0049 `for` 表达式的 desugar 语义

**Syntax**:`for PAT in EXPR BLOCK`(语法形式见 RXS-0028 控制流产生式;本条款定义其语义)。

**Dynamic Semantics**(以等价形式定义):

- **区间形态** `for p in lo..hi { body }`(`lo..hi` 为字面区间表达式)等价于:

```text
{
    let mut __i = lo;          // lo、hi 各求值恰好一次,先 lo 后 hi
    let __hi = hi;
    loop {
        match (if __i < __hi { let __v = __i; __i = __i + 1; Some(__v) } else { None }) {
            Some(p) => { body }
            None => break,
        }
    }
}
```

- **闭区间形态** `for p in lo..=hi { body }` 等价于:

```text
{
    let mut __i = lo;
    let __hi = hi;
    let mut __done = false;
    loop {
        match (if __done || __i > __hi { None } else {
            let __v = __i;
            if __i == __hi { __done = true; } else { __i = __i + 1; }
            Some(__v)
        }) {
            Some(p) => { body }
            None => break,
        }
    }
}
```

  (推进发生在 body 之前且 `__i == __hi` 时不递增,故 `hi` 为类型最大值时不产生越界递增。)

- **一般迭代器形态** `for p in it { body }`(`it` 非字面区间表达式)等价于:

```text
{
    let mut __it = it;
    loop {
        match __it.next() {
            Some(p) => { body }
            None => break,
        }
    }
}
```

- 三种形态中:`body` 内的 `break` / `continue` 绑定到展开引入的 `loop`,用户可见控制流语义不变(推进先于 body,`continue` 不会跳过推进);`__` 前缀的合成绑定不可被用户代码引用;`Some` / `None` 为内建 `Option` 变体(RXS-0048)。`for` 表达式整体类型为 `()`。

**Legality**:

- 区间形态:两端同整数型(RXS-0043),`p` 按该型绑定。
- 一般迭代器形态:`__it.next()` 按 RXS-0046 inherent 查找,失败 → `RX2004`;返回类型不是 `Option<T>` 形态 → `RX2001`;`p` 按 `T` 绑定。
- `p` 为可反驳模式时的合法性由展开后的 `match` 穷尽性裁决(`Some(p)` 臂的子模式参与 RXS-0051 判定)。

**Implementation Requirements**:类型诊断的 span 应指向用户源码中的 `for` 头部构件(迭代器表达式 / 模式),不暴露合成绑定名。

> 锚定测试:`conformance/desugar/for_range_desugar.rx`、`conformance/desugar/iterator_protocol.rx`;lower 单测(desugar 形状快照)。

### RXS-0050 `?` 操作符的 desugar 语义

**Syntax**:`e?`(后缀形式见 RXS-0027;本条款定义其语义)。

**Dynamic Semantics**:`e?` 等价于:

```text
match e {
    Ok(__v) => __v,
    Err(__e) => return Err(__e),
}
```

`Ok` / `Err` 为内建 `Result` 变体(RXS-0048),不受用户遮蔽影响;`__v` / `__e` 为合成绑定,不可被用户代码引用。

**Legality**:

- `e` 的类型必须为 `Result<T, E>` → 违例 `RX2001`(期待 `Result` 形态)。
- 所在函数返回类型必须为 `Result<U, E2>` 且 `E` 与 `E2` 合一 → 违例 `RX2001`(经展开式中 `return` 的返回一致性检查,RXS-0042)。**无 `From` 错误转换**:错误类型必须直接合一(trait 求解范围外;转换形态随 M4+ 只追加扩展)。
- `e?` 表达式的类型 = `T`。

**Implementation Requirements**:违例诊断的 span 指向 `?` 表达式本体,不暴露合成绑定名。

> 锚定测试:`conformance/desugar/question_mark_result.rx`;lower/typeck 单测。

### RXS-0051 `match` 模式穷尽性

**Legality**:

- `match` 的臂集合必须穷尽 scrutinee 类型的全部值 → 违例 `RX2007`(非穷尽 match)。
- M3.1 穷尽性判定域:
  - **enum**:全部变体被覆盖,变体载荷的子模式递归判定;
  - **bool**:`true` 与 `false` 均被覆盖;
  - **元组 / struct / 元组结构体**:逐字段递归判定;
  - **引用 `&T`**:对被引用类型递归判定;
  - **整数 / `char` / `str` / 浮点**:字面量与区间模式**不做值域完备性分析**——此类 scrutinee 必须存在通配或绑定臂兜底;
  - 通配 `_` 与(无歧义的)绑定模式覆盖任意值;or-pattern 覆盖域为各分支并集;`x @ p` 按子模式 `p` 判定;
  - **带 guard 的臂不计入穷尽性**(guard 真值静态不可知)。
- scrutinee 类型为名称/类型容忍区 `Err` 时不做穷尽性检查(RXS-0047 不级联口径)。
- 本条款仅约束 `match`;`let` 解构与函数参数模式的不可反驳性要求随 TBIR let-解构支持时条款化(M3.2 评估)。

**Implementation Requirements**:

- 检查时点:typeck 之后、MIR 构造之前(TBIR 窄门职责,D-202)。
- 诊断应给出至少一个未覆盖形态的保守描述(如未覆盖的变体名或 `_`);措辞允许保守粗糙(07 §4 先正确性后诊断)。

> 锚定测试:`conformance/desugar/match_exhaustive.rx`(正例);`tests/ui/typeck/` 非穷尽 snapshot。

### RXS-0052 drop scope 结构

**Dynamic Semantics**:

- 每个函数 body 携带一棵 **drop scope 树**:body 是根 scope;每个块表达式构成嵌套 scope;块内每条语句构成语句 scope(界定该语句临时值的存活段)。
- 局部绑定归属其声明所在的块 scope。块退出时——无论正常落出、`break` / `continue` / `return` 跨块转移——该块内仍在作用域的局部按**声明逆序**离开作用域。
- 无绑定的中间值(临时值)归属其所在语句 scope,语句结束时离开作用域。
- 本条款仅固定 **scope 结构与离开顺序**(TBIR 显式化的对象,D-202);"离开作用域时发生什么"(Drop 调用时点 / move 后不 drop / 条件初始化 drop flag)的执行语义条款随 M3.2 追加(M3 契约 D-M3-3),本条款只追加扩展。

**Implementation Requirements**:TBIR 必须显式携带 drop scope 结构(scope 树 + 局部归属);TBIR 为临时层,逐 body 构造 MIR 后即释放,不得驻留全程(D-202 峰值内存纪律)。

> 锚定测试:`conformance/desugar/drop_scope_blocks.rx`;tbir 单测(scope 树快照)。

### RXS-0053 Copy 类型与 move 语义

**Legality**(Copy 判定,affine 闭环的复制例外面,05 §3.1):

- 以下类型为 **Copy**(按值使用产生复制,原值保持有效):
  - 整数 / 浮点 / `bool` / `char` 原生类型;单元类型 `()`;
  - 共享引用 `&T`(任意 `T`);裸指针 `*const T` / `*mut T`;fn 指针;
  - 元组 / 数组:当且仅当全部组件类型为 Copy;
  - struct / enum:当且仅当其定义携带 `#[derive(Copy)]` 标注(见下)。
- **非 Copy**:`&mut T`(独占借用不可复制);未标注 `#[derive(Copy)]` 的 struct / enum(默认 move,affine 基调);`str` / slice 仅经引用出现,不独立参与判定。
- `#[derive(Copy)]` 最小识别面(05 §9 内建 derive 的 M3.2 子集,**不开放用户自定义 derive**):
  - 仅允许标注于 struct / enum 定义;
  - 要求全部字段(enum 为全部变体的全部载荷字段)类型为 Copy → 违例 `RX2008`;
  - 字段类型引用泛型参数时**保守拒绝** `RX2008`(无 trait bound 求解,D-104 口径;按实例放宽随 trait 求解条款化只追加扩展);
  - 携带 `Drop` impl(RXS-0055)的类型不得标注 `#[derive(Copy)]` → 违例 `RX2008`(Copy 值无确定析构点)。
  - `#[derive(Clone)]` 等其余内建 derive 不在本条款作用面(随 M4+ 条款化);未知 derive 名的处置维持既有属性容忍口径。

**Dynamic Semantics**(move 时点):

- 非 Copy 类型的 place 在**按值使用**处发生 **move**:赋值 / `let` 初始化的右侧、函数实参、构造字段(struct / enum / 元组 / 数组元素)、`return` 值、match scrutinee 按值消耗等。move 后原 place 进入已移出状态(后续使用合法性由 RXS-0054 裁决);Copy 类型在相同位置产生复制,原 place 状态不变。
- move 的来源仅允许**局部及其字段投影**;经 `&T` / `&mut T` 解引用 move 出非 Copy 值非法 → `RX4003`(被借者所有权不经引用转移)。

**Implementation Requirements**:MIR operand 必须区分 copy / move 形态(数据流分析的输入);Copy 判定按单态化后类型实施。

> 锚定测试:`conformance/borrowck/accept/copy_types.rx`;`tests/ui/typeck/` derive(Copy) 违例 snapshot;ty 单测(Copy 判定矩阵)。

### RXS-0054 初始化与 move 检查

**Legality**(静态数据流判定,MIR/CFG 层,05 §3.1 / 07 §4):

- 局部(含合成临时)初始状态为**未初始化**;函数参数入口即初始化。对 place 的整体赋值使其(重新)进入已初始化状态。
- **使用**(读取 / 按值消耗 / 取引用 / 字段投影读)要求该 place 在**全部**到达路径上已初始化且未被 move:
  - 存在某条到达路径上已被 move(含 maybe-moved)→ `RX4001`(use after move);
  - 存在某条到达路径上未初始化(含 maybe-uninit)→ `RX4002`(use before init)。
  - 判定**保守**:路径汇合处取最弱状态;循环按不动点收敛。条件路径的精度问题登记为已知限制,不阻塞(07 §4 先正确性后诊断)。
- 字段投影赋值(`x.f = v`)要求 base place 已初始化(否则 `RX4002`);move 出字段使**整 local** 进入已移出状态(M3.2 保守粒度;字段级精度随 M3.3+ 只追加提升)。
- 已移出 place 经整体赋值后重新初始化,恢复可用。

**Implementation Requirements**:检查时点 = MIR 构造后、codegen 前,对全部单态化 body 强制;诊断 span 指向违例使用处,并尽可能附 move 发生点标注(措辞允许保守粗糙)。

> 锚定测试:`conformance/borrowck/reject/use_after_move/`、`conformance/borrowck/reject/use_before_init/` 反例;`conformance/borrowck/accept/` 正例;`tests/ui/borrowck/` snapshot。

### RXS-0055 Drop 执行语义

**Legality**(`Drop` 最小识别面,对齐 RXS-0048 编译器已知项口径):

- `Drop` 为编译器内建 trait 名,注入 prelude 作用域(用户同名定义按 RXS-0033 遮蔽,遮蔽后该模块内 impl 绑定到用户 trait,不参与本条款语义)。
- 类型获得析构钩子的唯一通道:`impl Drop for T { fn drop(&mut self) { ... } }`,其中 `T` 为本包 struct / enum(泛型形态与类型定义一致)。形状约束:impl 体内**恰好一个**关联函数,名为 `drop`、接收者 `&mut self`、无其余参数、返回 `()` → 违例 `RX2009`;同一类型重复 `Drop` impl → `RX2009`。
- **不开放 trait 求解**:`Drop::drop` 不可显式调用(`RX2004` 既有查找面自然拒绝,不引入显式禁止码);drop 仅由编译器在析构点注入。

**Dynamic Semantics**(drop 时点与顺序,承接 RXS-0052 预留的"离开作用域时发生什么";05 §4 affine 资源闭环):

- **needs-drop 判定**(传递):类型 `T` needs-drop 当且仅当 `T` 自身携带 `Drop` impl,或 `T` 为 struct / enum / 元组 / 数组且存在 needs-drop 组件。引用 / 裸指针 / fn 指针 / 原生类型恒不 needs-drop;Copy 类型恒不 needs-drop(RXS-0053 互斥约束的推论)。
- **drop 时点**:值离开作用域(RXS-0052 scope 结构,局部按声明逆序、临时按 RXS-0056)时,若该值 needs-drop 且**此刻持有所有权**(已初始化且未被 move 出),执行 drop。
- **drop 动作**(递归):先调用该类型自身的 `Drop::drop`(若有),再按**字段声明序**递归 drop 各 needs-drop 字段;enum 仅 drop 当前活动变体的载荷;数组按元素序。
- **move 后不 drop**:已 move 出的值不在原作用域 drop(所有权随 move 转移:函数实参由被调方负责,`return` 值由调用方负责)。
- **赋值覆盖**:对已初始化且 needs-drop 的 place 整体赋值时,先 drop 旧值再写入。
- **每个值至多 drop 一次**;静态不可判定持有状态的(条件初始化 / 条件 move)经隐藏 **drop flag** 在运行期裁决,可观测行为与上述语义一致。
- 本条款仅定义正常控制流(落出 / `break` / `continue` / `return`)路径;无 unwind 语义(05 §7 host 错误模型无 panic 展开)。

**Implementation Requirements**:drop elaboration 在 MIR 上显式化(drop 语句 + drop flag),输入 = TBIR drop scope(RXS-0052)与 move/init 数据流结果(RXS-0054);drop 顺序须可经真实运行观测验证。

> 锚定测试:`conformance/borrowck/accept/drop_order_run.rx`(真跑顺序核对);`tests/ui/typeck/` Drop impl 形状违例 snapshot;mir 单测(drop 顺序快照)。

### RXS-0056 语句级临时值的 drop 时点

**Dynamic Semantics**(RXS-0052 语句 scope 的执行语义补全):

- 语句求值期间物化的无绑定中间值(临时值)中 needs-drop 且语句结束时仍持有所有权的,在**语句末尾**按**创建逆序** drop——先于下一语句开始。
- `let` 语句:初始化器产生的值经 move 进入绑定,不作为临时 drop;初始化器求值期间的其余临时按上款于 `let` 语句末 drop。
- 块尾表达式:其值 move 出至外层(块的值),求值临时于块尾表达式所在语句 scope 结束时 drop(块退出序列:先语句临时,后块内局部逆序,RXS-0052)。

**Legality**(RXS-0051 预留项的 M3.2 评估留痕):`let` 解构模式(非绑定 / 非通配)维持 MIR 作用面外(`RX6001`),本里程碑不引入;`let` 不可反驳性条款随解构支持时追加(只追加扩展)。

> 锚定测试:`conformance/borrowck/accept/temp_drop_stmt.rx`;mir 单测(临时 drop 时点快照)。

---

## 错误码引用汇总

| 错误码 | 含义 | 条款 |
|---|---|---|
| RX2001 | 类型不匹配(引用) | RXS-0049, RXS-0050 |
| RX2004 | 无此方法或关联项(引用) | RXS-0049 |
| RX2007 | 非穷尽 match | RXS-0051 |
| RX2008 | 非法 derive(Copy)(字段非 Copy / 泛型字段保守拒绝 / 与 Drop impl 冲突) | RXS-0053 |
| RX2009 | 非法 Drop impl(形状违例 / 重复 impl) | RXS-0055 |
| RX4001 | 使用已移出的值(use after move,含 maybe-moved) | RXS-0054 |
| RX4002 | 使用未初始化的值(use before init,含 maybe-uninit) | RXS-0054 |
| RX4003 | 经引用 move 出非 Copy 值 | RXS-0053 |

含义以 [../registry/error_codes.json](../registry/error_codes.json) 为唯一事实源,本表仅引用。RX2007 段位裁决(2xxx 类型检查段而非 4xxx):穷尽性是类型驱动的静态检查(输入 = typeck 结果),与借用/生命周期无关;裁决留痕于 error_codes.json revision_log(M3_PLAN §1 任务 5 / §5 风险条款)。RX2008/RX2009 段位裁决同理(derive(Copy) 合法性与 Drop impl 形状是定义处类型检查,非数据流);RX4001 ~ RX4003 为 4xxx 借用/生命周期段首批(move/init 数据流诊断,M3_PLAN §2 任务 3)。

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-12 | 初版:RXS-0048 ~ RXS-0052(M3.1 首批:lang-item 最小面 / for-`?` desugar / match 穷尽性 / drop scope 结构;05 §3 §4 §8 D-105/D-110、07 §1 D-202 已选定决策的条款化,M3 契约 D-M3-2 borrow 先行部分) | Direct |
| v1.1 | 2026-06-13 | 追加 RXS-0053 ~ RXS-0056(M3.2:Copy/move 语义、init/move 静态检查、Drop 执行语义(最小识别面 + drop flag)、语句级临时 drop 时点;05 §3.1 §4 §9 D-105 已选定决策的条款化,M3 契约 D-M3-3 spec 先行;RXS-0052/RXS-0051 预留项的承接,既有条款 0-byte);错误码汇总表追加 RX2008/RX2009/RX4001~4003 引用 | Direct |
