# Rurix 语言规范 — device 语义(M4.1:函数着色 / 地址空间 / barrier uniform 可达性)

> 条款:RXS-0066 ~ RXS-0069(M4.1 device 着色/地址空间首批)。体例见 [README.md](README.md)。
> 依据:05 §1(一门语言两个执行世界 / 函数着色 D-102)、05 §3.2(设备层执行资源与 views D-106)、05 §5(地址空间类型一等公民 D-108);06 §1 §2.2(执行模型 / kernel 抽象 D-120 / barrier 可达性保守检查 D-123);07 §3(着色与地址空间检查在 HIR 层,无需数据流);M4 契约 D-M4-1 / D-M4-2(spec 先行)。
> 本文为已选定决策(D-102/D-106/D-108/D-120/D-123)的初版条款化(档位 Direct);任何偏离 05/06/07/13 已锁定决策的修改须按 10 §3 升档。本文承载 device 语义全部条款,M4.2/M4.3 的 NVPTX codegen / launch 类型契约条款续写本文件(编号续号)。
> **M4.1 范围裁决(着色/地址空间优先,HIR 层无数据流)**:着色检查为符号属性的跨调用合法性;地址空间一致性在类型合一处裁决(`View` 族携带空间类型参数);barrier uniform 可达性本批为**保守骨架**(禁止 thread-id 依赖分支内调 barrier,违例须 unsafe),完整 uniform 控制流分析与 views 不相交证明(MIR 借用检查 device 扩展)随 M5。错误码 `RX3001`~`RX3003` 为 3xxx 着色/地址空间段位首批,**spec 先行引用,正式分配于 M4.1 实现 WP**(沿用 4xxx/5xxx 在实现 PR 落 registry 的节奏,registry revision_log 留痕,编号不复用)。

---

### RXS-0066 函数着色与跨着色调用合法性

**Legality**(05 §1 函数着色 D-102;07 §3 着色检查在 HIR 层):

- **函数着色(function coloring)**是函数的符号属性,四色:
  - `fn`(**host**,默认):宿主函数,可用全部宿主能力;
  - `kernel fn`(**kernel**):GPU 入口函数,只能经 launch API 发起(M4.3),不可被直接调用;
  - `device fn`(**device**):设备侧可调用函数,可被 kernel / device / host 调用(MVP 默认强制内联,06 §2.2);
  - `const fn`(**const**):编译期可求值函数,host / device 两侧上下文皆可调用(RXS-0062)。
- **调用上下文着色**:调用点所在函数体的着色决定其**可调用集**——
  - **host 上下文**(host fn 体 / const 与 static 初始化器):可调用 host / device / const 着色函数;直接调用 `kernel fn` 非法 → `RX3001`(kernel 须经 launch 发起,无设备侧 launch / 动态并行,05 §1)。
  - **device 上下文**(device fn 体 / kernel fn 体):仅可调用 device / const 着色函数;调用 host 着色函数非法 → `RX3001`(host-only 能力在设备不可达);直接调用 `kernel fn` 同样非法 → `RX3001`。
- **单向可达**:`device ⊂ host` 可调用集——需双侧可用的函数写成 `device fn` 且不使用宿主能力,host 上下文可直接调用之(避免 CUDA C++ `__host__ __device__` 双标注组合爆炸,05 §1)。
- **不级联**(RXS-0047 口径延续):调用目标着色不可判定(解析容忍区 / 内建函数 / 非函数项调用)时不触发 `RX3001`。

**Implementation Requirements**:着色检查在 HIR 层实施(07 §3,无需数据流),输入 = 各 body 的已解析调用目标(typeck `call_targets`:调用点 → 目标 DefId);检查时点 = typeck 之后、MIR 前;诊断 span 指向调用表达式本体,措辞允许保守粗糙(07 §4 先正确性后诊断)。

> 锚定测试:`conformance/coloring/accept/*.rx`(正例 0 诊断)、`conformance/coloring/reject/<category>/*.rx`(反例全拦截);`tests/ui/coloring/`(RX3001 snapshot);coloring 单测。

### RXS-0067 地址空间类型与一致性

**Syntax**:设备引用与容器类型携带地址空间参数(05 §5,D-108)。`View` 族容器的首类型实参为地址空间标记:

```
ViewType    ::= ("View" | "ViewMut") "<" AddrMark "," Type ("," Shape)? ">"
AddrMark    ::= "global" | "constant" | "local" | "host"
```

`global` / `constant` / `local` / `host` 为上下文关键字(RXS-0005,词法层按标识符产出),仅在 `View` 族首类型实参位置作地址空间标记裁决。`shared`(addrspace 3)是保留关键字(`shared let` 声明,05 §5),其地址空间不经 `View<...>` 类型实参直接书写——shared 空间 view 由对 `shared let` 的借用收窄获得(M5 device 借用扩展),故本批 `AddrMark` 不含 `shared`。

**Legality**(地址空间映射,05 §5 / r2 NVPTX 五空间):

- 五空间与 NVPTX addrspace 一一对应:`global`→addrspace(1,默认设备内存)、`shared`→(3,block 作用域,经 `shared let`)、`constant`→(4,只读广播)、`local`→(5,寄存器/栈)、`host`→宿主引用(device 不可达)。
- `View<space, T, Shape>`(只读)与 `ViewMut<space, T, Shape>`(可变)的地址空间是**类型组成**:两个 `View`(同可变性)当且仅当地址空间相同(且元素类型可合一)时为相容类型;类型合一处(调用实参 ↔ 形参、`let` 标注、返回类型)地址空间不一致 → `RX3002`(无需数据流,07 §3)。
- `local`(addrspace 5)不暴露指针/引用(05 §5,Slang 同款取舍):取局部地址即落 unsafe 或编译错误;safe 层 `View<local, ...>` 仅作受限只读形态,可变写经 views 收窄(M5)。
- 地址空间一致性是类型层裁决,与可变性(`View` vs `ViewMut`)正交:可变性不符按类型不匹配(`RX2001`)裁决,地址空间不符按 `RX3002` 裁决。

**Implementation Requirements**:`View` / `ViewMut` 为编译器已知容器类型(resolve 兜底识别,用户同名定义优先遮蔽),地址空间标记为编译器已知类型标记;一致性在类型检查层完成。`DeviceBuffer<T>` 等宿主侧资源句柄的地址空间隐含为 `global`(无显式空间参数,不参与本条款的空间合一裁决)。容忍区 `Err` 参与时不触发 `RX3002`(不级联)。

> 锚定测试:`conformance/addrspace/accept/*.rx`、`conformance/addrspace/reject/<category>/*.rx`;`tests/ui/addrspace/`(RX3002 snapshot);typeck 地址空间单测。

### RXS-0068 barrier uniform 可达性(保守骨架)

**Legality**(06 §2.2 barrier 可达性 D-123,MVP 保守版本):

- `block.sync()`(block barrier)的调用点必须对 block 内**全部线程一致可达**(uniform control flow):同一 barrier 要么所有线程执行,要么都不执行——否则部分线程在 barrier 等待而其余线程永不到达,构成 divergence deadlock(r5 列入 MVP 静态保证)。
- **保守骨架判定(M4.1)**:禁止在**依赖 thread id 的分支**内调用 barrier——即 `block.sync()` 出现在条件依赖线程索引(`thread_index` / `global_id` 等线程局部量)的 `if` / `match` / 循环体内时违例 → `RX3003`;此类构造须显式 `unsafe`(承担 P-03 验证义务)。
- 本批为**保守上界**:完整 uniform 控制流分析(精确判定分支条件是否 block-uniform)随 device 借用检查扩展(M5,07 §4);M4.1 骨架允许误拒边界情形(保守安全),措辞允许粗糙(07 §4)。

**Implementation Requirements**:barrier 可达性检查与着色检查同层(HIR 层,RXS-0066 实现要求同时点);骨架判定不需整数算术求解器(Descend 已证,r5);unsafe 块内的 barrier 调用豁免本检查。

> 锚定测试:`conformance/coloring/reject/barrier_non_uniform/*.rx`(骨架反例);`tests/ui/coloring/`(RX3003 snapshot);coloring 单测(barrier 骨架)。

### RXS-0069 着色/地址空间诊断要求(3xxx 首批)

**Legality**(3xxx 着色/地址空间段位首批,07 §5 段位语义):

- **跨着色非法调用** `RX3001`:device 上下文调用 host 着色函数;直接调用 `kernel fn`(host 或 device 上下文)。
- **地址空间不匹配** `RX3002`:类型合一处 `View` 族容器的地址空间不一致(RXS-0067)。
- **barrier 非 uniform 可达** `RX3003`:`block.sync()` 出现在依赖 thread id 的分支内(保守骨架,RXS-0068),且未置于 `unsafe`。

**Implementation Requirements**:

- 检查时点:着色 / barrier(RX3001 / RX3003)与地址空间(RX3002)均在 HIR / typeck 层实施(typeck 之后或之内,MIR 前),不依赖数据流(07 §3)。
- 诊断 span 指向违例构件(调用表达式 / 类型不符的实参或绑定 / barrier 调用点);措辞允许保守粗糙(07 §4 先正确性后诊断,M4 契约 §2.2 诊断打磨排除项)。
- **Err 容忍不级联**:目标着色不可判定或参与类型为容忍区 `Err`(RXS-0047)时不触发 3xxx(防一错多报,与 RXS-0047/0065 同口径)。

> 锚定测试:`tests/ui/coloring/`、`tests/ui/addrspace/`(黄金路径 4 的 3xxx 子集 snapshot);coloring / typeck 单测(失败路径)。

---

## 错误码引用汇总

| 错误码 | 含义 | 条款 |
|---|---|---|
| RX3001 | 跨着色非法调用(device 上下文调 host-only / 直接调用 kernel fn) | RXS-0066, RXS-0069 |
| RX3002 | 地址空间不匹配(View 族容器空间不一致) | RXS-0067, RXS-0069 |
| RX3003 | barrier 非 uniform 可达(thread-id 依赖分支内调 barrier,保守骨架) | RXS-0068, RXS-0069 |
| RX2001 | 类型不匹配(引用:View 可变性不符与元素类型不符走既有类型检查段) | RXS-0067 |

含义以 [../registry/error_codes.json](../registry/error_codes.json) 为唯一事实源,本表仅引用。RX3001 ~ RX3003 为 3xxx 着色/地址空间段位首批(07 §5 段位语义),**spec 先行引用,正式分配于 M4.1 实现 WP**(沿用 4xxx/5xxx 在实现 PR 落 registry 的节奏,registry revision_log 留痕,编号不复用)。views 不相交证明 / shared+barrier 一致性数据流 / 完整 uniform 分析为 M5 device 借用检查扩展(07 §4 / 11 §3),本批不覆盖。

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-13 | 初版:RXS-0066 ~ RXS-0069(M4.1 device 着色/地址空间首批:函数着色与跨着色调用合法性 / 地址空间类型与一致性 / barrier uniform 可达性保守骨架 / 着色与地址空间诊断要求;05 §1/§3.2/§5、06 §2.2、07 §3 已锁定决策 D-102/D-106/D-108/D-120/D-123 的条款化,M4 契约 D-M4-2 spec 先行)。错误码汇总表登记 RX3001~RX3003(spec 先行引用,实现 WP 正式分配);barrier 完整 uniform 分析与 views 不相交证明排除(M5) | Direct |
