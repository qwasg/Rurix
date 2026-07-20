# UC-05 最小 RHI + render graph 核心库面语义(EI1.3,RFC-0014 §4.B Part B)

> 条款:**RXS-0256 ~ RXS-0265**(EI1.3,验收门 G-EI1-3〔I1~I8 拦截〕/ G-EI1-5〔I9~I10 报告〕)。体例见 [README.md](README.md)。
> 承 [RFC-0014](../rfcs/0014-engine-integration.md)(Agent Approved 2026-07-19,§4.B Part B 参考级设计全文;§5 条款映射表 RXS-0256~0265)。**06 §8.3「它们是库」的库面兑现**——RHI / graph 零新语言机制,全为 std::gpu 薄映射(RXS-0189/0190/0197)+ 库层状态值,**Part B 零新 RX 码全复用**(§3 / §5.1)。

> 规范先行(AGENTS.md 硬规则第 7 条):**条款 commit 先于实现 commit**。`ci/trace_matrix.py --check` 要求每条
> `### RXS-####` ≥1 测试锚定(`//@ spec: RXS-####`);本文件条款的锚定测试(`conformance/uc05/{accept,reject}`
> 语料 + `apps/uc05-rhi/src/demo.rx` in-EXE device 真跑 + 步骤 72/73/75 门 + `evidence/uc05_*` 对照报告 +
> schema `check_schemas`)随实现 commit 同 PR 落(EI1.3 PR-B1/PR-B2,RFC-0014 §6.3)。stable 快照因条款计数
> 增长同 PR 重 bless(RXS-0180 L2 加性演进)。

> **禁区对照(RFC-0014 §7-2,Q-GraphReuse)**:本 RHI render graph 是 `apps/uc05-rhi` 内**新建 `.rx` 库**
> (compute-pass 面,主语言判据零 `.rs`,MS1 最严口径);G3.5 `src/rurix-rt/src/graph.rs`(Rust
> `#![forbid(unsafe_code)]` 图形面 render graph,RXS-0236~0241)**仅设计参照非代码复用**——状态推导 /
> 依赖建序 / 声明-反射相等思路镜像,码不进零 `.rs` 应用。两面概念重叠、定位不同(EI1_PLAN R6)。

---

## 1. 范围与编号区间

**RHI 库面 + compute-pass render graph,无新语法、无新语言机制**。`Rhi` / `Queue` / `Res` / `Pass`(+ 派生
`Graph` / `Buffer` / `Submitted`)为编译器已知签名的 lang-item 宿主类型,**薄映射 std::gpu**(RXS-0189 lang-item +
RXS-0190 已知签名分支先例,零新文法产生式);pass 以封闭枚举访问集(read / write)声明读写面;**声明序 = 提交序**
(不做重排,RFC-0010 确定性);graph 装配核验(依赖环 / 写写冲突 / 未声明访问 / 生命周期)于 `submit()` 装配期
确定性 strict 拒;资源 affine 生命周期 + 1-submit typestate 复用既有借用 / typestate 裁决。用户样例见 RFC-0014 §3.2。

- **RXS-0256**:RHI 类型面与 brand(`Rhi`/`Queue`/`Res`/`Pass` 薄映射 std::gpu lang items;per-instance 新鲜 opaque brand;方法所有权模式;I7 / I8)。
- **RXS-0257**:pass 声明与资源访问集(read / write 封闭枚举;未声明访问 I4——编译器喂反射集核验)。
- **RXS-0258**:graph 构建与依赖推导 + 依赖环(I3)/ 写写冲突(I5)构建期拒(纯库层定长数组状态值)。
- **RXS-0259**:资源生命周期 affine 拦截(I1 use-after-free / I2 double-free)。
- **RXS-0260**:submit typestate(`Graph → Submitted` 消费式,1-submit,镜像 RXS-0197;I6)。
- **RXS-0261**:执行语义(顺序调度 + 显式 sync + RXS-0193 诊断封口 + device 数值确定)。
- **RXS-0262**:transient 资源图内生命周期(const 泛型定长容量,RD-026;I10 峰值观测源)。
- **RXS-0263**:I1~I10 不变量矩阵与 100% 拦截判据(裁决 1 划界;I1~I8 拦截 / I9~I10 报告)。
- **RXS-0264**:对照报告证据形态(矩阵 json + schema 硬拦 + report.md,documented_historical 口径)。
- **RXS-0265**:采纳判据操作化(C ABI 成熟 + check <5s 双口径:冷全检 + 预热全量重析)。

**编号区间**:本文件条款自 **RXS-0256** 起(RFC-0014 earmark 段 0250~0269 的 Part B 段,续 Part A `spec/export_c.md`
RXS-0250~0255);区间登记于 [README.md](README.md) §4 文件清单(主循环收)。

**首期不可表达面(§5 范围红线)**:UAV 读写合并 / storage image 资源 / bindless / mesh·RT pass kind / pass 重排 /
依赖驱动调度 / `rhi_on_vulkan` 均不在首期封闭枚举内——显式登记 §5(RD-031 / RD-035+),不静默。

## 2. 条款(RXS-0256 ~ RXS-0265)

> 每条按需分 Syntax / Legality / Dynamic Semantics / Implementation Requirements 节,**严禁 UB 节**(UB 为经
> Full RFC 由 agent 自主落笔的高敏面,10 §7.5;本面无 UB 出口——承诺面外走编译期诊断 / 装配期库层状态值 strict 拒 /
> 运行期确定性失败 + 终止,P-01)。Legality 违例只**引用**错误码或库层状态(§3 引用汇总),不在此定义其含义。

### RXS-0256 RHI 类型面与 brand

**Syntax**(RHI 宿主库类型与方法集,lang items;薄映射 std::gpu):

```
Rhi / Queue<C> / Res<C> / Pass<C> / Graph<C> / Buffer<C, T> / Submitted   // 非 Copy affine 句柄结构
Rhi::create(&Context) -> Rhi                        // brand 化根句柄;每实例合成新鲜 opaque brand 类型 C(沿 RXS-0189 Context 底座)
rhi.queue() -> Queue<C>                             // 提交通道(薄映射 std::gpu Stream<C>)
rhi.resource(n: usize) -> Res<C>                    // owned affine 资源句柄(per-instance 新鲜 brand C)
rhi.graph() -> Graph<C>                             // 声明式图本体(affine;内部 const 泛型定长容量,RXS-0262)
rhi.readback(res: Res<C>, out: &mut PinnedBuffer<C, T>)   // 消费 res(Res move-out 点:I1 / I2 拦截锚)
g.pass(kernel) -> Pass<C>                           // pass builder(声明序 = 提交序)
pass.reads(&Res<C>) / pass.writes(&Res<C>) -> Pass<C>    // reads / writes 取 &Res 借用(非 move)→ 访问声明
g.submit(self) -> Submitted                         // 消费 g(Graph move-out,1-submit typestate,镜像 RXS-0197)
```

**Legality**:

- 类型为编译器 lang items(`Rhi` / `Queue` / `Res` / `Pass` 四件核心 + 派生 `Graph` / `Buffer` / `Submitted`,
  追加于既有 std::gpu lang items〔RXS-0189/0190/0197〕之后,DefId 编号稳定),用户同名定义优先遮蔽、语义不变
  (兜底纪律沿 RXS-0189)。全部句柄类型为**非 Copy affine**:move 后再用 / 重复 move / 借用冲突等违例**复用
  RXS-0054 与 RXS-0057~0061 既有裁决**(**零新借用码**)。
- **薄映射 std::gpu(库面零新语言机制)**:`Rhi::create(&Context)` 沿 RXS-0189 `Context` 底座(RHI 为库面薄壳,
  06 §8.3「它们是库」);`Queue<C>` 薄映射 `Stream<C>`、`Res<C>` / `Buffer<C, T>` 薄映射既有 `Buffer<C, T>`。方法集
  经 typeck 编译器已知签名分支表达(RXS-0190 口径);元数 / 类型 / 方法名不符 → **RX2003 / RX2001 / RX2004** 复用
  (零新码)。
- **per-instance 新鲜 opaque brand(I7)**:`Rhi::create` **每实例合成新鲜 opaque brand 类型 `C`**(per-instance
  新鲜 brand,沿 RXS-0189 opaque brand 类型面,**非「生命周期 brand `Res<'rhi>`」**);`Res<C>` / `Graph<C>` /
  `Pass<C>` / `Buffer<C, T>` 泛型签名以 `C` 钉资源归属。跨 `Rhi` 实例误用(brand A 的 `Res` 入 brand B 的 `Pass`)
  → **编译期 context-brand 不匹配 RX3006**(复用 RXS-0074 / RXS-0189 既有 brand 裁决,**非 RX2001**)。**显式排除**
  RXS-0189 line 61「单 brand + cabi 运行期 context-id 校验」降级路径——该路系运行期拦截,取之则 I7 落入 I9 类
  运行期观测项、无法满足 I1~I8「100% 编译期 / 构建期拦截」判据(G-EI1-3);UC-05 取 per-instance 新鲜 brand 类型,
  保 I7 ∈ 编译期集。
- **宿主 API 着色合法性(I8)**:RHI 类型的构造与方法调用**仅 host 着色上下文合法**;出现在 `kernel` / `device fn`
  体内 → **RX3015**(coloring 层,与 RXS-0189/0197 同点位)。
- **方法所有权模式(保 RFC-0014 §3.2 示例可编译)**:`rhi.resource(n) -> Res<C>` 产 **owned affine** 句柄;
  `pass.reads(&res)` / `pass.writes(&res)` 的 `reads` / `writes` **取 `&Res` 借用**(调用期短借用、不 move——故同一
  `&res` 可跨多 pass 复用、`.reads(&a).reads(&a)` 二次借用合法、非 use-after-move);`g.submit(self)` **move-out
  Graph**(I6);`Res` 的 move-out 点 = `rhi.readback(res, …)` / 显式释放(**I1 use-after-free / I2 double-free 的
  实际消费锚**——无此消费点则 I1 / I2 by-construction 不可达而非「被拦截」,故点名钉死)。
- **图内资源记账无堆**:`Graph<C>` 在**无堆定长数组**内以资源 id / 索引(非借用)记账多 pass 资源(RD-026 无堆约束,
  避自指借用结构);容量编译期有界见 RXS-0262。

**Dynamic Semantics**:

- affine 句柄 drop 按声明逆序发生;RHI 句柄自身无附加运行期语义(图装配 / 执行语义见 RXS-0258 / 0261);
  `Rhi::create` 求值 = 沿 RXS-0189 `Context` 底座建根句柄,失败语义封口 RXS-0193。

**Implementation Requirements**:

- 句柄为编译器合成布局(`handle: u64` + brand 幽灵参数);方法集经 typeck 编译器已知签名分支表达(RXS-0190 先例,
  `Stream::launch` / present typestate 分支);`Rhi::create` 关联构造镜像 `Context::create` 解析锚点。全部为
  `apps/uc05-rhi` 内 `.rx` 库面薄壳,**零 `.rs`**(主语言判据沿 MS1 最严口径,`ci/uc07_offline_golden_smoke.py`
  :95-113 零 .rs 审计先例)。方法名终形随实现 PR 在已知签名纪律内定案(RFC-0009 §4.7 先例),语义面以本章为准。

> 测试锚定:conformance/uc05/accept/rhi_min.rx(0 诊断,RHI 四件构造 + graph 最小声明,lowering 见证)+
> reject/rhi_cross_brand.rx(brand A `Res` 入 brand B `Pass` → **RX3006**,I7)+ reject/rhi_in_kernel.rx
> (kernel 体内 RHI 构造 / 方法 → **RX3015**,I8);cabi `rxrt_rhi_*` 符号面 doc + rhi.rs 库单测 +
> tests/uc05_corpus.rs 批跑锚定(RXS-0256~0260)。

### RXS-0257 pass 声明与资源访问集

**Syntax**(pass builder 访问声明链,lang-item 方法):

```
g.pass(kernel, GridDim(..), BlockDim(..), (args..)) -> Pass<C>   // 逐 pass builder;声明序 = 提交序
pass.reads(&Res<C, T>) -> Pass<C>              // read 访问声明
pass.writes(&Res<C, T>) -> Pass<C>             // write 访问声明
```

`pass` 的 **kernel 绑定形态与 `Stream::launch` 逐位同构**(kernel fn item 引用 + `GridDim` + `BlockDim` +
实参元组),复用同一 marshalling 契约(RXS-0191)与同一契约裁决体(`launch_check`:着色 RX3004 / 维度
RX3005 / 实参 RX2001 / brand RX3006,**零新码**)。`Res<C, T>` 与 `Buffer<C, T>` 平行,以 `View<space, T>` /
`ViewMut<space, T>` 形参承载。

**Legality**:

- `g.pass(kernel).reads(&res).writes(&res)` builder 方法链(逐方法即逐 typeck 已知签名分支,RXS-0190 先例;诊断
  span 精确到单条访问声明)。**访问种类首期封闭枚举**:`read` / `write`(本面「不支持即不可表达」——UAV 读写合并 /
  storage image 等不在首期枚举内,超界登 §5)。
- **未声明访问拦截(I4,编译器 / 语言面强制,非纯库层零新码)**:判「pass 实际触碰未在其声明集内的 `Res`」须把
  **kernel 实际访问集**与**声明集**精确相等比对——kernel 签名是**编译期知识**,`.rx` 无运行期反射(RD-026 无字符串 /
  无集合 / 无反射),故 **reflected 集由编译器在 typeck / 构建期喂入**(镜像 G3.5 `src/rurix-rt/src/graph.rs::with_reflection`
  由编译器 / 外部提供 kernel 签名反射集),再与声明集精确相等核验(漏声明 / 声明未用即失配,**镜像 RX6030 口径**)→
  **构建期确定性 Err**(库层状态值,**零新 RX 码**;拦截承载**计入语言 / 编译器面**,非「纯库层定长数组状态值」)。

**Implementation Requirements**(I4 反射喂入链路;**EI1.4 兑现**):

- **反射集提取(编译期)**:pass 绑 kernel 后,该 pass 的 reflected 集 = **绑定实参中类型为 `Res<C, T>` 者**——
  即 kernel 实际触碰的资源。这是**编译期知识**:实参类型由 typeck 定型,且 `launch_check` 已核对每个 `Res` 实参
  确落在 kernel 签名的 `View` / `ViewMut` 形参位(非该位 → RX2001),故「kernel 实际访问集」由 kernel 签名与绑定
  实参**静态确定**,不依赖任何运行期反射(RD-026 维持)。
- **下发链路**:mir_build 的 marshalling 物化把 `Res` 实参标为 **kind-2 槽**(0 = `Buffer` / 1 = 标量 / 2 = RHI
  资源;与 `rxrt_launch` 同一槽契约的只追加扩展)→ `rxrt_rhi_bind(pass, entry, 维度×6, slots, kinds, n)` →
  cabi 自 kind-2 槽还原资源下标集 → `PassSpec::with_reflection`。
- **核验点**:`submit()` 的 `seal()` 对**声明集 ↔ 反射集**双向精确相等核验(漏声明 / 声明未用即失配)→ 库层
  `ReflectionMismatch` Err → `RXRT_FAIL` → `rxrt_trap` 确定性终止。**装配期**判定(`--emit=check` CLEAN)。

**Dynamic Semantics**:

- pass 声明为 host 侧记账(资源 id / 索引入 `Graph<C>` 定长数组);`g.submit()` 触发装配核验(RXS-0258)——声明-反射
  相等核验于装配期确定性判定,strict-only,无运行期 fallback(P-01)。

**Implementation Requirements**:

- 声明集 ↔ 反射集相等核验**相等域 = 首期封闭枚举访问面**(read / write);编译器喂反射集锚点镜像 `graph.rs::with_reflection`
  (设计参照,非 `.rs` 代码复用);Err 携库层状态值(**不占编译器 RX 段位**),诊断可定位到违例 pass(对比 G3.5 RX6030
  编译期码,库层口径不弱化 strict-only 承诺)。

> 测试锚定(**I4:EI1.3 诚实收窄 → EI1.4 兑现**):conformance/uc05/accept/pass_declared.rx(pass 绑
> kernel 后声明集 ↔ 反射集精确相等,0 诊断 + 装配期通过)+ **conformance/uc05/assembly/pass_undeclared_read.rx**
> (kernel 实际触碰 {a, b} 而只声明 `writes(&b)` → **漏声明 a** → 装配期库层 `ReflectionMismatch` Err →
> `rxrt_trap`;编译期 CLEAN,步骤 72 device 段 EXE RED 真跑见证 stderr 含 `rhi_submit [reflection]`)+
> rurix-rt rhi.rs 库单测 `rejects_reflection_mismatch_i4` / `accepts_reflection_exact_match_i4`(纯 host
> 无 GPU 见证)+ tests/uc05_corpus.rs `rxrt_rhi_bind` lowering 见证(kind-2 槽下发)。
>
> **口径迁移**:EI1.3 期 I4 = `lib_tested`(机制已实现 + 库测,但 `.rx` 反射喂入未接线,不宣称 ci_checked);
> **EI1.4 起 I4 = `assembly_time` / `ci_checked`**——`.rx` 反射喂入已接线并对真实语料真触发。矩阵
> (RXS-0263 I4 行)与 evidence/uc05_invariant_matrix.json 同步迁档。

### RXS-0258 graph 构建与依赖推导 + 依赖环 / 写写冲突拒绝

**Legality**(装配期确定性核验,strict-only,**纯库层定长数组状态值零新码**):

- **边推导**:写后读(RAW)/ 写后写(WAW)按**声明序**建 pass 序(声明全序无重排,RFC-0010 确定性口径;pass 重排 /
  依赖驱动调度 out_of_scope,§5)。
- **依赖环拒绝(I3)**:use-before-write 可达形态的环——消费读(`reads`)的资源若无先前 pass 的写(`writes`),即
  「读未写」可达环 → **构建期(`submit()` / 装配期)确定性 strict 拒**(**纯库层定长数组状态值零新码**,镜像 G3.5
  RX6029 口径;无编译器喂反射,真零新码)。
- **写写冲突(I5)**:同资源同序位多写者 / 写序违例(同 pass 对同资源重复声明写)→ 构建期确定性 Err(纯库层状态值
  零新码,镜像 RX6029 口径)。跨 pass 顺序重写(ping-pong)合法(由声明全序覆盖)。
- **生命周期误用**:空图 `submit` / 已 `submit` 后追加 pass → 构建期确定性 Err(与 RXS-0260 typestate 联动,镜像
  RX6029 生命周期误用口径)。

**Dynamic Semantics**:

- 图装配 = host 侧纯记账 → `submit()` 触发装配核验(I3 / I4〔RXS-0257〕/ I5 + 声明-反射相等)+ 依赖序推导 → 顺序调度
  (RXS-0261)。**同图 → 逐字节相同装配产物**(确定性,golden 可锚)。

**Implementation Requirements**:

- I3 / I5 判定为 `apps/uc05-rhi` 内 `.rx` 库面纯函数(定长数组状态机,零 `.rs`、零后端调用);环检测锁 use-before-write
  可达形态(镜像 RXS-0237 可达性口径),不做声明全序以外的重排。库层状态值**不分配编译器 RX 段位**(spec/imageio.md
  库层 `Result` / 哨兵先例)。

> 测试锚定:conformance/uc05/accept/graph_three_pass.rx(三 compute pass RAW 建序,0 诊断)+
> **assembly/graph_cycle.rx**(读未写可达环,I3)+ **assembly/graph_write_write.rx**(同资源同序位多写,I5)
> + assembly/graph_empty.rx(空图生命周期误用)——**均编译期 CLEAN**(图装配期性质,`--emit=check` 不拦),违例
> 在 `submit()` **装配期(图装配期)** host 侧确定性拦(库层状态值 Structure Err → RXRT_FAIL → rxrt_trap):
> `--emit=check` 不拦但 submit 确定性终止。装配期确定性拦的**纯 host 无 GPU 见证** = rhi.rs 库单测
> `rejects_read_before_write_i3` / `rejects_write_write_conflict_i5` / `rejects_lifecycle_misuse`;EXE
> red-green e2e(编译成 EXE 真跑退非零 + stderr 含装配 Err)由 ci/uc05_rhi_smoke.py 步骤 72 device 段兑现。

### RXS-0259 资源生命周期 affine 拦截(I1 / I2)

**Legality**:

- `Res<C>` **非 Copy affine**——move 后再用(use-after-free 面,I1)/ 重复 move-out(double-free 面,I2)→
  **编译期 move 违例 RX4001**(复用 RXS-0054,**零新借用码、零新 RX 码**);经引用消费 → **RX4003**(复用 RXS-0053)。
- move-out 点 = `rhi.readback(res, …)` / 显式释放(RXS-0256 钉;`reads(&res)` / `writes(&res)` 取借用非 move,不构成
  消费点——故资源可跨多 pass 声明复用,消费仅在 readback / 释放发生)。

**Dynamic Semantics**:

- affine 句柄 drop 无附加运行期语义;实际 GPU 资源销毁经 RHI 底层 std::gpu 销毁纪律(RXS-0189/0194),失败封口 RXS-0193。

> 测试锚定:conformance/uc05/reject/res_use_after_move.rx(readback 消费后再用 `Res` → RX4001,I1)+
> reject/res_double_move.rx(二次 readback 同 `Res` = 重复 move-out → RX4001,I2);accept/graph_three_pass.rx
> readback 合法末次消费 + cabi `rxrt_rhi_readback` 符号锚定。**readback 消费实现**:mir_build 对资源实参
> 发射 `Operand::Move`(镜像 submit 消费式接收者纪律,唯此处 move 实参非接收者)→ move 检查裁决 RX4001。

### RXS-0260 submit typestate(I6,镜像 RXS-0197 present 消费式)

**Legality**:

- `Graph<C>` **消费式** `submit(self) -> Submitted`——接收者按值消费(镜像 RXS-0197 present `wait` / `signal` /
  `present` 消费式转移);**二次 submit = 编译期 move 违例 RX4001**(复用 RXS-0054;经引用消费 → RX4003)。
- 跳态 / 非本态方法(`Submitted` 上调 `pass` / `submit` 等图建面方法不存在)→ 走既有方法查找 **RX2004**(复用)。
  **零新借用码、零新 RX 码**(RXS-0197 同模)。

**Dynamic Semantics**:

- 消费式转移的 lowering 以 MIR move 表达(接收者按值 move 进 `Submitted`),move / init 数据流(RXS-0054)天然拦截
  编译期二次 submit;`submit` 触发装配核验(RXS-0258)+ 顺序调度(RXS-0261)。

**Implementation Requirements**:

- 消费式方法集经 typeck 编译器已知签名分支表达(RXS-0190 / RXS-0197 先例);`Submitted` 为终态句柄,无图建面方法。

> 测试锚定:conformance/uc05/accept/single_submit.rx(单次 submit → `Queue`,0 诊断)+
> reject/rhi_double_submit.rx(`submit` 后二次 `submit` → RX4001,I6)。

### RXS-0261 执行语义

**Dynamic Semantics**:

- **顺序调度 + 显式 sync**:`submit()` 后按**声明全序**单 queue 顺序提交 compute pass;跨 pass happens-before 由声明
  全序裁定(pass 粒度)。运行期失败(device 分配 / launch / sync 失败)走 **RXS-0193 确定性诊断封口**(操作名 + 原因 +
  context 序号,落 stderr 后进程终止)——**不占编译器 RX 段位**、无 UB、无静默降级(P-01)。
- **数值确定性**:device 真跑数值结果对照 host 参考(**I9 报告项**,RXS-0263);**同机两跑逐字节确定**。kernel 保持
  **编译期有界简单核**(saxpy / scale / reduce 级),避开 RD-027 深弹射毒径(G3.1 归因结论并读)。

**Implementation Requirements**:

- **EI1.4 兑现口径(真 compute dispatch + device 数值)**:`submit()` 的执行序为
  **① 装配核验(seal:I3/I4/I5)→ ② 纯函数 hazard 推导(`derive_syncs`)→ ③ 按推导序真派发**。三段**严格有序**:
  seal 失败则一个 kernel 也不派发(pre-dispatch fail-closed);派发**不做二次推导**,而是**逐字重放**②的计划——
  推导计划在第 `at_pass` 个 pass 边界产出的每条 `PlannedSync`,令执行器在**派发该 pass 之前**于本图 stream 上落
  一个显式同步点。单 queue 声明全序 ≙ 单 stream 顺序派发,故这些同步点是 hazard 计划驱动的**保守封口**而非重排
  依据。全部 pass 派发完毕后对本图 stream 收尾同步,使随后的 `readback` D2H 见到完整结果。
- **资源与读回**:`Res<C, T>` 为**真设备分配**(`n * sizeof(T)` 字节,`cuMemAlloc`;`rhi` 销毁时连带释放,
  释放前对 ctx sync 封口 in-flight 窗口);`queue.readback(res, &mut pinned)` 为**真 D2H**(`cuMemcpyDtoH`,长度
  须与资源分配字节数精确一致)。读回点归 `Queue<C>`——`submit(self) -> Queue<C>` 的消费式 typestate 使
  **「先派发、后读回」的执行序由类型强制**(submit 前无 `Queue` 可读回,submit 后 `Rhi` 已被消费)。
- **数值对照(I9)已 device 落地**:`apps/uc05-rhi/src/demo.rx` 两 pass 真算(pass1 `a[i] = i + 1`;pass2
  `b[i] = 2*a[i]`)→ readback `b` → host 侧逐元素求和 vs 闭式参考 `N*(N+1)` 精确比对,打印机器可核 token
  `UC05_SUM` / `UC05_REF`,相等才打 `UC05_RHI_OK`、不等退 2;步骤 72 device 段**独立复核**二者相等。
  kernel 保持**编译期有界简单核**,避开 RD-027 深弹射毒径。
- **仍诚实标注的未兑现面**:I9 虽已 device measured,**仍留 `report_only`**——数值正确性本质动态(单机单驱动
  一次观测,非全域证明)。**pass 重排 / 依赖驱动并行调度**(多 stream)未实现:声明全序即执行序,`PlannedSync`
  的作用是插同步点而非重排(§5)。**transient 资源别名复用**未实现(I10 峰值恒等于声明容量,RXS-0262)。
  RURIX_REQUIRE_REAL 纪律下 demo EXE 真跑(GPU Context)不许 SKIP 充绿(步骤 72 device 段);无 GPU / link
  工具链 → SKIP dev-env-degrade。

> 测试锚定:apps/uc05-rhi/src/demo.rx(两 pass 真派发 + 真 D2H + I9 数值对照 device 真跑,见证
> `UC05_SUM` == `UC05_REF` + `UC05_RHI_OK`;步骤 72 device 段 GREEN)+ conformance/uc05/accept/graph_three_pass.rx
> (`queue.readback(res, &mut pinned)` lowering 落 `rxrt_rhi_readback(i64, i64, ptr, i64)`)+ rhi.rs
> `accepts_linear_graph_derives_raw_syncs` / `derivation_is_deterministic`(声明全序执行序纯 host 确定性)+
> cabi `rhi_symbols_failure_path_and_assembly`(装配 → 声明全序 RAW 同步)。

### RXS-0262 transient 资源图内生命周期

**Legality**:

- graph 内生(transient)资源容量**编译期有界**——`Graph<C>` 内部以 **const 泛型定长数组**承载资源槽(RD-026 无堆
  集合对策,镜像 ruridrop 静态容量);声明资源数超容量 → **编译期拒**(const 泛型定长越界,复用既有 const / 类型诊断,
  **零新码**)。

**Dynamic Semantics**:

- 执行期 transient 资源实际并发存活峰值 ≤ 声明容量(**I10 报告项**,RXS-0263;运行期观测,**不可静态全证实际峰值**)。
  transient 资源生命周期 = 图内声明区间(首次写 → 末次读)。

**Implementation Requirements**:

- **诚实收窄(EI1.3 落地口径)**:EI1.3 兑现面 = **host 侧容量记账**(`RhiGraph` 单调 `resource()` 分配 +
  `resource_count()` 精确追踪图内 transient 资源数;声明区间 = 首写→末读)。上文 Legality 的「const 泛型定长
  数组编译期越界拒」为 RD-026 无堆对策的**目标形态**——现 host 记账以 `Vec` 承载(runtime-bounded),**const
  泛型定长数组 + 编译期越界拒的 `.rx` 接线随后续期落地**(与 I4 `.rx` 反射喂入同批,EI1.4+);EI1.3 不锚
  不存在的 reject/transient_capacity_overflow.rx。实际并发存活峰值 evidence 经 device 执行期计数采集(I10
  报告项,measured_local,归 EI1.4 device 真跑)。

> 测试锚定:rurix-rt rhi.rs 库单测 `transient_resource_capacity_accounting`(host 侧容量记账本体,I10 静态源;
> 纯 host 无 GPU)+ apps/uc05-rhi/src/demo.rx 执行期峰值 evidence(I10,device EI1.4)。

### RXS-0263 I1~I10 不变量矩阵与 100% 拦截判据

**Legality / Dynamic Semantics**(裁决 1 划界,消除 EI1_CONTRACT §1「I1~I10」vs 门「I1~I8」内部不一致):

- **I1~I8 = 100% 确定性检测项**(逐条断言,入验收门 **G-EI1-3** / 步骤 73,漏拦即红)——三档确定性:
  - **编译期**(typeck / `--emit=check` 即拦,违例不可构造):**I1 / I2 / I6 / I7 / I8**;
  - **装配期(图装配期)**(`submit()` 时 host 侧确定性拦;`--emit=check` **不拦**但 submit 确定性 rxrt_trap,
    pre-dispatch):**I3 / I5**;
  - **lib_tested**(机制由 rhi.rs 库单测证纯 host 无 GPU;`.rx` 反射喂入随 EI1.4):**I4**。
- **I9~I10 = 仅报告 / 观测对照项**(对标上一项目 Python 计数器事后观测,**不可静态拦截**,入对照报告 **G-EI1-5**,
  `documented_historical` 口径)。

> **叙事口径(诚实收窄)**:所有 I1~I8 = 100% **确定性**检测(**编译期 OR 装配期确定性,或库测已证机制**),对照
> 上一项目**运行期概率性计数器可漏**。**「编译期即不可构造」仅对 I1 / I2 / I6 / I7 / I8 成立**;I3 / I5 = 装配期
> 确定性拦(图装配期性质非类型面性质,`--emit=check` CLEAN,submit 确定性 rxrt_trap——装配期确定性 ≠ 运行期概率性,
> 纯 host、pre-dispatch);I4 机制库测已证、`.rx` 接线随 EI1.4。裁决 1「编译期 / 构建期」措辞保留(**构建期 = 装配期**)。

**不变量矩阵**(逐条:不变量 / 档 / 拦截机制 / 条款或诊断码 / 语料或库测 / 期望诊断 / 证据级):

| # | 不变量 | 档 | 拦截机制 | 条款 / 诊断码 | 语料 / 库测 | 期望诊断 | 证据级 |
|---|---|---|---|---|---|---|---|
| **I1** | 资源 use-after-free(`Res` move 后再用) | 编译期 | affine 所有权(RXS-0189/0054;readback 按值消费) | RXS-0259 / **RX4001**(复用,零新码) | `conformance/uc05/reject/res_use_after_move.rx` | 编译期 move 违例 RX4001 | ci_checked(步骤 73) |
| **I2** | 资源 double-free(`Res` 重复 move-out) | 编译期 | affine(二次 readback = 重复 move) | RXS-0259 / **RX4001**(复用) | `conformance/uc05/reject/res_double_move.rx` | 编译期 move 违例 RX4001 | ci_checked |
| **I3** | pass 依赖环(use-before-write 可达) | 装配期 | graph 装配期确定性拒(纯库层状态值) | RXS-0258 / 库层状态 Err(镜像 RX6029 口径,零新码) | `conformance/uc05/assembly/graph_cycle.rx` + rhi.rs `rejects_read_before_write_i3` | 装配期确定性 Err → rxrt_trap | ci_checked |
| **I4** | 未声明访问(触碰未声明 `Res`) | **装配期**(EI1.4 迁档) | 声明-反射精确相等(编译器自 kernel 签名与绑定实参静态提取反射集 → kind-2 槽 → `with_reflection`) | RXS-0257 / 库层状态 Err(镜像 RX6030 口径,零新码) | `conformance/uc05/assembly/pass_undeclared_read.rx` + rhi.rs `rejects_reflection_mismatch_i4` | 库层确定性 Err(`rhi_submit [reflection]` → `rxrt_trap`) | **ci_checked**(EI1.3 = lib_tested,EI1.4 接线兑现) |
| **I5** | 写写冲突(同资源同序位多写 / 写序违例) | 装配期 | graph 装配期确定性拒(纯库层状态值) | RXS-0258 / 库层状态 Err(镜像 RX6029 口径) | `conformance/uc05/assembly/graph_write_write.rx` + rhi.rs `rejects_write_write_conflict_i5` | 装配期确定性 Err → rxrt_trap | ci_checked |
| **I6** | 1-submit typestate 二次 submit | 编译期 | 消费式 typestate(镜像 RXS-0197) | RXS-0260 / **RX4001**(复用,经引用 RX4003) | `conformance/uc05/reject/rhi_double_submit.rx` | 编译期 move 违例 RX4001 | ci_checked |
| **I7** | 跨 brand 资源误用(brand A `Res` 入 brand B `Pass`) | 编译期 | per-instance 新鲜 opaque brand 类型(镜像 RXS-0189) | RXS-0256 / **RX3006**(复用 RXS-0074/0189,**非 RX2001**) | `conformance/uc05/reject/rhi_cross_brand.rx` | 编译期 context-brand 不匹配 RX3006 | ci_checked |
| **I8** | RHI 着色合法性(RHI 构造 / 方法于 `kernel`/`device fn` 体内) | 编译期 | 着色合法性(RXS-0189/0197 同点位) | RXS-0256 / **RX3015**(复用) | `conformance/uc05/reject/rhi_in_kernel.rx` | 编译期 RX3015 | ci_checked |
| **I9** | compute pass 数值正确性(GPU 输出 vs host 参考) | report_only | 运行期 device 数值对照(本质动态,**不可静态全证**) | RXS-0263 报告项 / 无诊断码 | `apps/uc05-rhi/src/demo.rx`(`UC05_SUM` / `UC05_REF`)+ 步骤 72 device 段独立复核 | GPU 求和 == host 闭式参考 | **EI1.4 device measured_local**(仍留 report_only:一次观测非全域证明);Python 侧 = **无数字的定性历史陈述**(上一项目代码 / 交接档不在仓库,EI1_PLAN R3;非可复跑、零杜撰数字) |
| **I10** | transient 资源执行期峰值 / 生命周期(并发存活 vs 声明容量) | report_only | 运行期观测(**不可静态全证实际峰值**;host 侧容量记账 EI1.3 兑现) | RXS-0263 报告项 / 无诊断码 | rhi.rs `transient_resource_capacity_accounting`(host 记账) | 实际峰值 ≤ 声明容量(**平凡成立**,见右) | **诚实标注:未完全兑现**——EI1.4 每 transient `Res` = 一笔真设备分配、生命期 = 图生命期,故峰值**恒等于**声明容量;别名复用与执行期峰值计数器**均未实现**,随后续期。Python 侧 = 无数字的定性历史陈述(同 I9) |

> **对照口径(documented_historical,硬规则 3;redline 评审 F3 钉死)**:上一项目代码与 H01~H07 交接档**不在仓库**
> (已核实事实,EI1_PLAN R3)——`文件:行号` 伪引文会指向仓外不存在文件、其数字永不可由命令输出复核(正面顶撞硬规则 3
> 「所有数字必须来自命令输出」),**取消对仓外源的伪引文格式**(防「看似可机验」的杜撰窗口)。I9 / I10 的 Python 侧
> 「计数器事后观测」= **无数字的定性历史陈述**(纸面对照)——`evidence/uc05_comparison_report.md` **顶部醒目标注**
> 「historical counters unavailable in-repo, non-reproducible, no fabricated figures」,报告显式声明不可复跑 A/B、
> **零杜撰 Python 数字**;**schema 层(`check_schemas` 硬拦)禁止 I9 / I10 出现无 in-repo 出处的数值字段**(RXS-0264
> 测试锚定已落)。Rurix 侧证据全 measured / ci_checked。对照核心论点:I1~I8 这组不变量上一项目靠运行期 Python
> 计数器事后捕获(部分漏到生产),Rurix 由类型系统 / 图装配期 **100% 确定性拦截**(**编译期即不可构造** I1/I2/I6/I7/I8,
> **装配期确定性拦** I3/I5,**lib_tested 机制已证** I4);I9 / I10 本质动态(数值 / 执行期峰值),两侧同为观测面,
> Rurix 侧以 device measured 兑现(EI1.4)。**删去对 I3/I4/I5 的「编译期即不可构造」过强表述**——I3/I5 装配期确定性、
> I4 库测机制 + `.rx` 接线 EI1.4,均确定性(非运行期概率),但非「编译期不可构造」。

> 测试锚定:conformance/uc05/{reject,assembly}/ I1~I8 逐条语料 + rhi.rs 库单测(I3/I4/I5 纯 host 见证)+ 步骤 73
> 不变量拦截门逐条断言 + 矩阵 ↔ 语料 ↔ report.md 三方一致性互查(`ci/uc05_invariant_gate.py`,漏拦 / 漂移即红)+
> **schema 禁 I9 / I10 无 in-repo 出处数值字段**(`check_schemas` 硬拦,字段全 string/null,任何 number 值即违例)。

### RXS-0264 对照报告证据形态(镜像 RXS-0134 / 0148 体例)

**Implementation Requirements**:

- **矩阵 json**:`evidence/uc05_invariant_matrix.json`——逐不变量记 {拦截机制 / 条款号 / reject 语料路径 / 期望诊断 /
  CI 结果 / 证据级};**I9 / I10 Python 侧为无数字定性历史陈述,schema 禁止无 in-repo 出处的数值字段**(redline 评审 F3)。
- **schema 硬拦**:`milestones/ei1/uc05_invariant_matrix_schema.json` 入 `check_schemas` 硬门——schema 层禁止 I9 /
  I10 字段含无 in-repo 出处数值(防杜撰窗口)。
- **叙事报告**:`evidence/uc05_comparison_report.md`——**顶部醒目标注**「historical counters unavailable in-repo,
  non-reproducible, no fabricated figures」,纸面对照口径显式声明不可复跑 A/B、零杜撰 Python 数字。
- **三方一致性机核(步骤 73,防 YAML-only)**:矩阵 json ↔ reject/assembly 语料实存 ↔ report.md 三方一致性互查
  (条款号 / 语料路径 / 诊断码逐项对齐),任一漂移即红(`ci/uc05_invariant_gate.py` + `tests/uc05_corpus.rs`
  `invariant_matrix_three_way_consistency`)。

> 测试锚定:步骤 73 三方一致性互查(矩阵 ↔ 语料 ↔ report.md,`ci/uc05_invariant_gate.py` +
> `tests/uc05_corpus.rs::invariant_matrix_three_way_consistency`)+ `check_schemas` 校验
> `uc05_invariant_matrix.json`(字段全 string/null,任何 number 值即违例——无 in-repo 出处数值字段硬拦)。

### RXS-0265 采纳判据操作化

**Implementation Requirements**:

- **C ABI 成熟** = `#[export(c)]` 端到端(DLL + 生成头 + C 宿主真跑,**G-EI1-4**,EI1.2 / EI1.4 落;Part A
  `spec/export_c.md` RXS-0250~0255)。
- **增量 check <5s = 双口径 measured**:
  - `ei1.bench.uc05_check_cold_ms`——`apps/uc05-rhi` 全包 `--emit=check` **冷全检**(含磁盘 `mod` 解析,BENCH_PROTOCOL
    三次 trimmed mean);
  - `ei1.bench.uc05_check_warm_ms`——**进程 / 缓存预热后的全包 `--emit=check` 重跑**(**诚实标注全量重析、非 LSP 增量**:
    现 tooling session〔`src/rurixc/src/tooling/session.rs::analyze`〕只对单个内存文件 lex + parse + check_crate、无
    `mod` 解析 / 磁盘加载,无法「增量」检全包 `apps/uc05-rhi`,故 warm 口径**不用** didChange → publishDiagnostics 增量
    路、去「增量 / incremental」措辞;若坚持 LSP 增量则须把 tooling server 扩为整 crate 分析 = net-new 工作量,本期不取)。
  - 阈 **5000ms** measured_local 回填。
- evidence 面**不进 CI 硬门**(计时波动,EA1 冷启动先例),**SKIP 不充绿**。

> 测试锚定:`ei1.bench.uc05_check_cold_ms` / `ei1.bench.uc05_check_warm_ms` measured 回填(阈 5000ms,warm =
> 全量重析口径,非 LSP 增量)。

## 3. 错误码引用汇总(**Part B 零新 RX 码全复用**)

| 码 / 状态面 | 段 | 语义 | 条款 |
|---|---|---|---|
| RX4001 / RX4003 | 4xxx 借用 | `Res` / `Graph` affine move 后再用 / 经引用消费(I1 / I2 / I6;复用 RXS-0054 / RXS-0053,零新借用码) | RXS-0259 / RXS-0260 |
| RX3006 | 3xxx 着色 | 跨 brand 资源误用(brand A `Res` 入 brand B `Pass`,I7;复用 RXS-0074 / RXS-0189,**非 RX2001**) | RXS-0256 |
| RX3015 | 3xxx 着色 | RHI 构造 / 方法出现在 `kernel` / `device fn` 体内(I8;复用,与 RX3001 同点位) | RXS-0256 |
| RX2001 / RX2003 / RX2004 | 2xxx 类型 | 方法实参类型 / 元数 / 方法名不符(编译器已知签名核验,复用,零新码) | RXS-0256 / RXS-0260 |
| 库层状态 Err(镜像 RX6029 口径) | —(库层) | graph 依赖环 / 写写冲突 / 生命周期误用(I3 / I5;**纯库层定长数组状态值,零新 RX 码**,不占编译器段位,spec/imageio.md 先例) | RXS-0258 |
| 库层状态 Err(镜像 RX6030 口径) | —(库层 / 编译器喂反射) | 声明-反射失配 / 未声明访问(I4;编译器喂反射集核验,**零新 RX 码**,拦截计入语言 / 编译器面) | RXS-0257 |

**Part B 零新 RX 码全复用(RFC-0014 §5.1 明记)**:affine / typestate 违例复用 **RX4001 / RX4003**、brand 误用复用
**RX3006**(RXS-0074/0189,非 RX2001)、着色违例复用 **RX3015**、类型 / 元数 / 方法查找复用 **RX2001 / RX2003 /
RX2004**;graph 构建期错误(I3 / I4 / I5)走**库层状态值**(镜像 G3.5 RX6029 / RX6030 口径,**不新造 RX 码**——I3 / I5
纯库层定长数组状态值真零新码,I4 由编译器喂反射集核验、拦截计入编译器面但仍零新 RX 码);transient 容量越界复用既有
const / 类型诊断;运行期 / 环境失败(device 分配 / launch / sync)走 **RXS-0193** 确定性诊断 + 终止,**不占 RX 段位**
(06 §8.2 口径)。**本文件零新 RX 码、零新借用码。**

## 4. 首期不可表达面 / 范围红线留痕

- **访问封闭枚举首期只 read / write**:UAV 读写合并 / storage image 资源 / bindless 表 / mesh·RT pass kind 出封闭
  枚举——凡含此类的 pass 首期不可经 UC-05 RHI graph 表达,显式登记(RD-035+),不静默。
- **声明全序、无重排**:pass 重排 / 依赖驱动调度 out_of_scope(§2 RXS-0258);声明序 = 提交序措辞封死重排面,不为
  未来扩张预留弱化措辞。
- **`rhi_on_vulkan` out_of_scope**:首期 CUDA std::gpu 底座(rxrt_* PTX)+ engine_host v2(C++ / D3D12 嵌入侧);
  `.rx` 单源 Vulkan RHI 通道归 **RD-031 open**(激活时复评 G3 vk descriptor 底座影响,RFC-0014 §8 / Q-A)。
- **transient 容量 const 有界(RD-026)**:图内生资源无堆集合,const 泛型定长承载;超界即编译期拒,不静默扩容。
- **无 UB 节**:本面承诺面外一切构造走编译期诊断(复用)/ 装配期库层状态值 strict 拒 / 运行期确定性失败 + 终止 +
  poisoned 传播(RXS-0193),无静默降级(P-01),无实现自由竞争窗口。
- **零 `.rs` 主语言判据边界**:`apps/uc05-rhi` 全 `.rx`(RHI 库 + demo);engine_host v2(C++)为**嵌入宿主**、在
  应用主语言判据审计边界之外(RFC-0014 §9.2 B-5),不混入零 `.rs` 判定。

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.2 | 2026-07-20 | EI1.4 实现落地:**RHI compute dispatch + I4 编译器反射喂入**(兑现 EI1.3 诚实归到 EI1.4 的两项收窄)。①**pass 绑 kernel**(RXS-0257):`rhi.pass(kernel, GridDim(..), BlockDim(..), (args..))`,形态与 `Stream::launch` **逐位同构**——tbir `RhiPassBind` 与 `GpuLaunch` 共用形态判据 `kernel_binding_form`、mir_build 共用 marshalling 物化 `gpu_marshal_args`、契约裁决共用 `launch_check::check_kernel_binding`(着色 RX3004 / 维度 RX3005 / 实参 RX2001 / brand RX3006,**零新码**);`launch_check::ty_compat` 补 `Ty::Const` 自反臂(缺之则同 `Rhi` 实例资源被误判跨 brand,镜像 typeck unify 同修)。②**I4 反射喂入接线兑现**(RXS-0257):编译器自 kernel 签名与绑定实参**静态提取**反射集(实参中的 `Res<C, T>`,由 launch_check 核对确落在 `View`/`ViewMut` 形参位)→ marshalling **kind-2 槽**(0=Buffer/1=标量/2=RHI 资源,`rxrt_launch` 槽契约只追加扩展)→ 新符号 `rxrt_rhi_bind` → `PassSpec::with_reflection` → `seal()` 双向精确相等核验;新语料 `conformance/uc05/assembly/pass_undeclared_read.rx`(kernel 触碰 {a,b} 只声明 `writes(&b)`)**真触发**库层 `ReflectionMismatch` → `rxrt_trap`(device EXE RED 真跑见证 `rhi_submit [reflection]`)。**I4 自 `lib_tested` 迁 `assembly_time` / 证据级 `ci_checked`**(矩阵 / 报告 / 步骤 73 门同步迁档;LIB_TESTED 档遂为空集)。③**真 compute dispatch**(RXS-0261):`rxrt_rhi_submit` 在 seal + `derive_syncs` **之后**按推导序真派发——推导计划**逐字重放**(每条 `PlannedSync` 令执行器在派发该 pass 前于本图 stream 落显式同步点,执行器禁二次推导),全部派发后收尾同步;派发本体复用 `rxrt_launch` 抽出的单一事实源 `launch_prepared`。`Res<C, T>` 获元素类型参数(镜像 `Buffer<C, T>`,元素经使用点推断 + RX2010 定型)并为**真设备分配**(`n * sizeof(T)`,`rxrt_rhi_resource(r, bytes)`;`rhi_destroy` 连带释放,释放前 ctx sync 封口)。④**真 D2H + 读回点迁 `Queue`**(RXS-0259):`queue.readback(res, &mut pinned)`(`rxrt_rhi_readback(r, src, dst, bytes)`)——readback 自 `Rhi` 迁 `Queue<C>` 使**「先派发、后读回」执行序由类型强制**(submit 前无 `Queue`,submit 后 `Rhi` 已消费),move-out affine 语义(I1/I2 → RX4001)不变。⑤**I9 device 落地**(RXS-0263):`apps/uc05-rhi/src/demo.rx` 升为两 pass 真算 + readback 求和 vs 闭式参考,机器可核 token `UC05_SUM` / `UC05_REF`,步骤 72 独立复核相等;**I9 仍留 report_only**(一次观测非全域证明)。⑥**诚实标注未兑现面**:pass 重排 / 依赖驱动并行调度(多 stream)未实现——声明全序即执行序,`PlannedSync` 作用是插同步点非重排;**I10 未完全兑现**——每 transient `Res` 生命期 = 图生命期故峰值恒等于声明容量(「≤ 声明容量」平凡成立而非因复用收紧),别名复用与执行期峰值计数器均未实现,随后续期。⑦步骤 72 RED 断言改为**按语料头 `//@ assembly-reject: <category>` 逐例核类别**(`structure` / `reflection`),GREEN 增 I9 数值对照;evidence schema 增 `demo_numeric`。零新 RX 码维持(全复用 + 库层状态值);零新 lang item。 | **Full RFC**(RFC-0014 / §4.B / EI1.4) |
| v1.1 | 2026-07-19 | EI1.3 PR-B2 实现落地 + **对抗性验证 disposition 诚实收窄**:①**readback 接线兑现**(RXS-0259):hir `Op::RhiReadback` + typeck(`rhi.readback(res)` 资源实参按值消费)+ mir_build(实参 `Operand::Move` → move 检查 RX4001)+ cabi `rxrt_rhi_readback(r, src)` affine 消费;readback 后再用 / 二次 readback → **RX4001**(I1/I2 真兑现,conformance/uc05/reject/res_use_after_move.rx + res_double_move.rx)。②**I4 诚实收窄**(RXS-0257/0263):I4 未声明访问核验机制(`with_reflection` 声明-反射相等)已实现 + 库测(`rejects_reflection_mismatch_i4`);`.rx` 编译器反射喂入(pass 绑 kernel)与 compute dispatch 耦合、随 **EI1.4** 落地——EI1.3 不宣称 I4 `.rx` 路 ci_checked,不锚 pass_undeclared_read.rx;矩阵 I4 证据级 = `lib_tested(EI1.3) / .rx_wiring:EI1.4`。③**I3/I5 装配期分档**(RXS-0263):I1/I2/I6/I7/I8 = **编译期**(typeck / --emit=check 即拦);I3/I5 = **装配期(图装配期)**(`submit()` 时 host 侧确定性拦,--emit=check CLEAN,submit 确定性 rxrt_trap);I4 = lib_tested;叙事改「I1~I8 = 100% **确定性**检测(编译期 OR 装配期确定性 / 库测机制),对照上一项目运行期概率性计数器可漏」,删对 I3/I4/I5 的「编译期即不可构造」过强表述。④**RXS-0262 诚实收窄**:EI1.3 兑现 host 侧容量记账(rhi.rs `transient_resource_capacity_accounting`),const 泛型定长数组编译期越界拒的 `.rx` 接线随后续期落地(Vec 承载 runtime-bounded)。⑤**RXS-0261 诚实收窄**:EI1.3 demo host 图 submit 装配核验通过 device 真跑(`UC05_RHI_OK`),pass 绑 kernel compute dispatch + 数值对照(I9)归 EI1.4。⑥语料迁 `conformance/uc05/{accept,reject,assembly}`(4 accept + 5 编译期 reject + 3 装配期,tests/uc05_corpus.rs 批跑)+ apps/uc05-rhi 零 .rs demo + 步骤 72(ci/uc05_rhi_smoke.py:host 恒跑 corpus/审计/--emit=check + device 段 EXE red-green)+ 步骤 73(ci/uc05_invariant_gate.py:I1~I8 逐条 + 三方一致)+ evidence/uc05_invariant_matrix.json(schema 字段全 string/null 硬拦 I9/I10 数值)+ comparison_report.md。零新 RX 码维持;trace_matrix 全锚定;stable 快照重 bless(spec_clauses 251→261,RXS-0180 L2 加性)。budget `ei1.counter.uc05_invariant_cases`(≥8)。 | **Full RFC**(RFC-0014 / §4.B / PR-B2) |
| v1.0 | 2026-07-19 | 新建 spec/rhi.md(EI1.3,PR-B1 条款先行):带编号条款体 `### RXS-0256 ~ ### RXS-0265`(FLS 体例,按需分 Syntax / Legality / Dynamic Semantics / Implementation Requirements,**严禁 UB 节**;镜像 spec/render_graph.md 体例)——RXS-0256 RHI 类型面与 brand(Rhi / Queue / Res / Pass 薄映射 std::gpu lang items,per-instance 新鲜 opaque brand,方法所有权 reads / writes 取 &Res 借用、submit move-out、readback 为 Res move-out 锚,跨 brand → RX3006〔非 RX2001〕,kernel 体内 → RX3015,显式排除 RXS-0189 line 61 单-brand 运行期降级)/ RXS-0257 pass 声明与资源访问集(read / write 封闭枚举,未声明访问 I4 由编译器喂反射集核验、库层状态 Err 镜像 RX6030)/ RXS-0258 graph 构建与依赖推导(RAW / WAW 建序,依赖环 I3 / 写写冲突 I5 纯库层定长数组状态值构建期拒、镜像 RX6029)/ RXS-0259 资源生命周期 affine 拦截(I1 / I2 → RX4001 复用)/ RXS-0260 submit typestate(Graph → Submitted 消费式 1-submit,镜像 RXS-0197,二次 submit → RX4001)/ RXS-0261 执行语义(顺序调度 + 显式 sync + RXS-0193 诊断封口 + device 数值确定 I9)/ RXS-0262 transient 资源(const 泛型定长容量 RD-026,超界编译期拒,I10 峰值观测源)/ RXS-0263 I1~I10 不变量矩阵(裁决 1 划界:I1~I8 编译 / 构建期 100% 拦截入 G-EI1-3 步骤 73、I9 / I10 报告项入 G-EI1-5 步骤 75,documented_historical 无数字定性历史陈述、schema 禁无 in-repo 出处数值)/ RXS-0264 对照报告证据形态(uc05_invariant_matrix.json + schema check_schemas 硬拦 + comparison_report.md 顶部标注、三方一致性机核)/ RXS-0265 采纳判据操作化(C ABI 成熟 G-EI1-4 + check <5s 双口径 cold / warm、warm = 全量重析非 LSP 增量,阈 5000ms)。**Part B 零新 RX 码全复用**(RX4001 / RX4003 / RX3006 / RX3015 / RX2001 / RX2003 / RX2004 + 库层状态值镜像 RX6029 / RX6030 口径,§3 / §5.1);零新借用码。每条 ≥1 `//@ spec` 测试锚定(conformance/uc05/{accept,reject} + apps/uc05-rhi/src/demo.rx + 步骤 72 / 73 / 75 + evidence/uc05_* + schema)随实现 commit 同 PR 落,trace_matrix 全锚定;stable 快照同 PR 重 bless(RXS-0180 L2)。承 [RFC-0014](../rfcs/0014-engine-integration.md)(Agent Approved 2026-07-19,§4.B Part B 参考级设计)。 | **Full RFC**(RFC-0014 / §4.B / PR-B1) |
