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
RXS-0250~0255);区间登记于 [README.md](README.md) §4 文件清单(主循环收)。G4.3 PR-E 追加 **RXS-0280~0283**
(RD-035 执行面三项 + const 容量接线,续 RXS-0277;0278~0279 burned 跳号)。

**首期不可表达面(§5 范围红线)**:UAV 读写合并 / storage image 资源 / bindless / mesh·RT pass kind / pass 重排 /
依赖驱动调度 / `rhi_on_vulkan` 均不在首期封闭枚举内——显式登记 §5(RD-031 / RD-035+),不静默。
**G4.3 PR-E 后**「pass 重排 / 依赖驱动调度」由 RXS-0281/0282 兑现(单 queue 批级,多 queue 仍 out-of-scope);
§5 既有 RD-035+ 登记**字面不动**,G4.3 PR-E 兑现面以 RXS-0281/0282 追加条款为准。

## 2. 条款(RXS-0256 ~ RXS-0265, G4.2 扩 RXS-0270~0277, G4.3 PR-E 扩 RXS-0280~0283)

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
- **嵌入面(EI1.4,`#[export(c)]` 导出根内建图)**:整张 RHI 图可完整封闭在一个 `#[export(c)]` host fn 体内,经
  `rurixc --emit=dll` 产 cdylib 供**异语言宿主**(C / C++)调用——GPU 上下文、图、资源的创建与销毁全部在**单次
  调用内**闭合,宿主只见 C 兼容子集 v1 的标量与裸指针(spec/export_c.md §RXS-0251),**不见任何 Rurix 类型**。
  该形态对本条执行语义**零特例**:seal → 推导 → 派发 → 收尾同步 → D2H 与 in-EXE 路逐段同一,差别仅在结果的
  出口是 `*mut T` 出参而非进程 stdout。错误面按 §RXS-0255「无 panic 面 by-construction」以 **i32 状态码**跨
  ABI 返回(RD-026 无 Result 面纪律),**不展开 unwind、不 panic 终止**。链接线扩展(GPU-using 导出面须链
  `rurix_rt_cabi` + 系统库固定集)见 spec/export_c.md §RXS-0252。
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
> **嵌入面见证**:apps/uc05-rhi/src/embed.rx(`#[export(c)] uc05_run_graph` 导出根内建同一张图,
> kernel 与闭式参考取自 `mod graph` 单一事实源)+ src/rurix-engine/harness/uc05_engine_host.cpp(engine_host **v2**,
> C++/D3D12 LUID 匹配 adapter + fence 锚点夹住图节点)+ 步骤 74 `ci/uc05_engine_embed_smoke.py` device 段
> (三方数值对照:device 求和 / 宿主闭式参考 / CI 脚本独立重算,见证 `UC05_EMBED_OK`)。

> **G4.3 PR-E 追加式修订(RXS-0281,既有承诺字面不动)**:本条「顺序调度 + 显式 sync」承诺**字面
> 不动**;G4.3 PR-E 后**依赖保持下的重排/批级调度**为执行模型升级——`derive_exec_plan` 拓扑分层
> (单 queue 批级提交,层间屏障;多 queue 仍 out-of-scope)。`PlannedSync` 在重排后序上重算,
> 执行器禁二次推导(P-11)。核验器独立重建依赖闭包逐边核(I11,RXS-0282)。既有 `derive_syncs`
> 声明全序路**0-byte 保留为兼容路**。

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

> **G4.3 PR-E 追加式修订(RXS-0283,既有承诺字面不动)**:本条「const 泛型定长数组 + 编译期越界拒的
> `.rx` 接线随后续期落地」**字面不动**;G4.3 PR-E 后**const 容量接线已兑现**——`rhi.graph::<CAP>()`
> lang-item 已知方法调用点 turbofish const 实参(字面量即时求值 → 普通 i64 cabi 实参,CAP 不进类型
> 参数表,无 RD-007 依赖)+ 编译期越界拒(typeck 单函数体 affine 单定义链前向扫描)+ non-static
> construction strict 拒(循环/条件/跨函数)。host 侧 `RhiGraph` 仍以 `Vec` 承载(runtime-bounded),
> const CAP 经 cabi i64 实参传入,host 侧记账核验(resource_count > cap → 装配期 Structure Err)。
> **I10 自 report_only 升 measured_local**(RXS-0280 别名复用 + 执行期峰值计数器,峰值 < 声明容量
> 可 device 见证,非平凡成立)。

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

### RXS-0270 RHI 图形 pass 类型面（raster / mesh pass，RT 条件臂，G4.2，RFC-0015 §4.A1）

**Syntax**(图形 pass 声明,lang-item 已知方法扩面,零新文法产生式):

```
g.raster_pass(vs, fs) -> GfxPass<C>    // raster pass:vertex + fragment 着色对(RXS-0153 阶段着色 / RXS-0159 io_sig 既有)
g.mesh_pass(ms, fs) -> GfxPass<C>      // mesh pass:mesh + fragment(RXS-0243 入口契约;task 前置条件臂首期不开放)
GfxPass<C>                             // pass 句柄族新成员:非 Copy affine,消费式声明链,与 Pass<C> 同族(RXS-0256)
```

**Legality**:

- `raster_pass` / `mesh_pass` 为 `Graph<C>` 的**编译器已知方法**(typeck 已知签名分支,RXS-0190 先例);元数 / 类型 /
  方法名不符 → **RX2003 / RX2001 / RX2004** 复用(零新码)。`GfxPass<C>` 与 `Pass<C>` 同属 pass 句柄族(非 Copy affine,
  借用 / move 违例复用 RXS-0054 族,零新借用码);brand `C` 与图同源,跨 brand → **RX3006**(I7 同点位)。
- **着色函数引用合法性**:raster_pass 的两实参须 `vertex` / `fragment` 阶段着色函数(RXS-0153,顺序 vertex 先 fragment 后);
  mesh_pass 的两实参须 `mesh` / `fragment` 阶段着色函数,且 mesh 函数满足 **RXS-0243 入口契约**(`#[numthreads]` +
  `#[outputs(topology="triangles", max_vertices, max_primitives)]`,违例 → **RX3017** 既有);错阶段 / 非阶段着色函数 →
  **RX3001 / RX3015** 复用(零新码)。**task 前置(task → mesh payload 链)首期不开放**:出现 `task fn` 引用 → 编译期拒
  (复用 RX3017 类别,条件臂评估留痕 RFC-0015 §9 Q-RTArm/Q-MeshScope)。
- **宿主 API 着色合法性(I8 扩展)**:`raster_pass` / `mesh_pass` 声明出现在 `kernel` / `device fn` 体内 → **RX3015**
  (与 RXS-0256 I8 同点位)。
- **RT pass 条件臂(G-EA1-3 / RXS-0249 先例)**:`rt_pass(raygen, miss, closesthit)` 类型面**仅在执行臂同序列可达时
  才立**(RT MIR lowering 最小集 + AccelStruct 资源面 + SBT;评估窗 = mesh lowering 落地后,RFC-0015 §9 Q-RTArm);
  不可达则**不立类型面、登记 RD-036+**——strict-only 拒半成品;验收门 G-G4-3 以 raster + mesh 满足,不依赖本臂。

**Dynamic Semantics**:

- 图形 pass 声明仅**建图**(记入图声明序,RXS-0258 同规则);执行语义与自动 barrier 见 RXS-0272;着色函数的 device 语义
  由既有阶段类型面(RXS-0153~0159/0223/0243)与 codegen(RXS-0204/0246/0275)承载,本条零新语义本体。

**Implementation Requirements**:

- lang-item 分支(resolve / typeck / mir_build 加性,镜像 compute `pass` 既有面);反射集提取见 RXS-0273;执行与导出见
  RXS-0272 / RXS-0277。**零新 RX 码、零新借用码**。

> 测试锚定:conformance/uc05/accept/gfx_raster_mesh.rx(0 诊断,raster + mesh pass 声明)+ reject/rhi_gfx_in_kernel.rx
> (kernel 体内 raster_pass → **RX3015**)+ reject/gfx_wrong_stage.rx(fragment+vertex 错序 / compute kernel 充 vs →
> 着色诊断)+ rurixc corpus 单测锚定。

### RXS-0271 RHI 图形资源面（color / depth target、texture2d、sampler、texture_table，G4.2，RFC-0015 §4.A2）

**Syntax**(图形资源构造,`Rhi<C>` 已知方法扩面):

```
rhi.color_target(w: u32, h: u32) -> Res<C>    // color 附件目标(transient)
rhi.depth_target(w: u32, h: u32) -> Res<C>    // depth 附件目标(transient)
rhi.texture2d(w: u32, h: u32) -> Res<C>       // 可采样纹理(SRV;内容经 host 上传或 storage 写)
rhi.sampler(desc: SamplerDesc) -> Res<C>      // 采样器状态对象(RXS-0225 宿主形态薄映射)
rhi.texture_table() -> Res<C>                 // 无界纹理注册表(RXS-0235 TextureTable 薄映射,bindless 库化,见 RXS-0276)
```

**Legality**:

- 五构造均为 `Rhi<C>` 的**编译器已知方法**(RXS-0190 分支),产 `Res<C>` 族**非 Copy affine** 句柄(brand `C` 与 `Rhi`
  同源;跨 brand → **RX3006**;kernel / device fn 体内 → **RX3015**;元数 / 类型 → RX2001~2004 族,零新码)。
- **封闭格式集(首期为定,不泛化)**:color target = RGBA8(逐通道 u8 normalized);depth target = D32F;texture2d =
  RGBA8;readback 缓冲 = f32 / u32(沿 RXS-0256 / RXS-0236 资源面口径);超集请求 → 编译期拒(类型面封闭枚举,零新码)。
- `SamplerDesc` 复用 **RXS-0225** 宿主采样器状态面(同一状态空间;aniso 缺失确定性 `Err` 不占 RX 码);`texture_table`
  复用 **RXS-0235**(注册单调索引;feature chain 缺失 → 确定性 `Err`)。

**Dynamic Semantics**:

- transient 生命周期与 compute 资源同锚:**首写 → 末读** 区间记账(RXS-0262),`readback` / `rhi_destroy` 为释放锚;
  别名复用分配器对图形 transient 同样生效(RXS-0280,区间着色含尺寸 / 对齐三分量)。
- color / depth target 经**图外 readback**(copy 到 readback 缓冲)出图;纹理上传经 host → device copy;采样器无资源
  状态面(不参 barrier 状态推导,RXS-0272)。

**Implementation Requirements**:

- cabi 资源类参数追加式扩展(`rxrt_rhi_resource` 类枚举:0=buffer / 1=color / 2=depth / 3=texture / 4=sampler /
  5=table;既有 0 语义 0-byte);vk.rs 侧对象创建(image / image view / sampler / descriptor)沿 U27 审计模式登记;
  **零新 RX 码**。

> 测试锚定:conformance/uc05/accept/gfx_resources.rx(五构造 0 诊断 + 格式封闭枚举)+ reject/gfx_res_cross_brand.rx
> (跨 brand 图形资源 → **RX3006**)+ reject/gfx_res_in_kernel.rx(→ **RX3015**)。

### RXS-0272 图形 pass 访问声明集与自动 barrier（推导单源 graph.rs，G4.2，RFC-0015 §4.A3）

**Syntax**(访问声明封闭枚举,消费式声明链):

```
gfx.writes_rt(&Res<C>)           // color attachment 写
gfx.writes_depth(&Res<C>)        // depth attachment 写
gfx.reads(&Res<C>)               // shader read(含采样纹理与 texture_table)
gfx.reads_writes_uav(&Res<C>)    // UAV 读写(storage image / storage buffer)
gfx.binds_sampler(&Res<C>)       // 采样器绑定声明(非资源状态访问)
g.present(&Res<C>)               // 呈现终端 handoff(每图 ≤1,且必须为声明序末 pass)
pass.reads / pass.writes         // compute pass 访问声明 0-byte(RXS-0257)
```

**Legality**:

- 访问声明为**封闭枚举**(镜像 RXS-0236 `AccessKind` 五类 + PresentHandoff,**同一 graph.rs::AccessKind 单源**——RHI 侧
  不复制枚举定义);每 pass 每资源至多一条声明(读写反馈 → 装配期拒);`binds_sampler` 为**绑定声明非资源状态访问**
  (sampler 无 barrier 状态面);`present` 终端声明必须唯一且处声明序末位(违例 → 装配期拒,库层状态值零新码,镜像
  RX6029 口径)。
- **执行后端(strict 无回退,§G4.2 Q-A)**:含任一图形 pass 的图**仅经 Vulkan 后端执行**;compute-only 图维持 CUDA 既有
  路 **0-byte**;CUDA 后端遇图形 pass / Vulkan 不可用 → **装配期确定性拒**(非运行期炸、非静默换后端,RXS-0193 口径)。
- **compute pass 访问映射(语义钉死,RFC-0015 §4.0-1)**:混合图中 compute pass 的 `reads` → `ShaderRead`、
  `writes` → `UavReadWrite`(RXS-0238 映射表),同步由 `PlannedBarrier::BufferSync` 承载——其 pass 粒度全序
  happens-before(RXS-0239)**涵盖且等价于** CUDA 腿 `PlannedSync` 流序同步点语义(同为 pass 粒度全序);行为等价由
  G4.4 同图双腿数值对照 device 见证(步骤 80)。

**Dynamic Semantics**:

- **自动 barrier(推导单源 = G3.5 graph.rs,P-11)**:图 sealed 后,RHI 运行时(rhi.rs,与 graph.rs 同属 rurix-rt crate)
  将 pass / 资源记录**直接构造 graph.rs 的 `Graph` / `PassSpec`**(同 crate 函数调用,无 cabi marshalling)→
  `derive_barriers()`(RXS-0238 状态机,双后端映射同源)→ `PlannedBarrier` **逐字回放**(Vulkan 执行器:逐 pass
  render pass begin / end + 边界 `vkCmdPipelineBarrier`;执行器禁二次推导,镜像 RXS-0240 `run_graph` 先例)。
- **跨 pass happens-before = RXS-0239 既有承诺**(pass 粒度全序;G4.2 首期声明序 = 执行序,重排执行模型归 RXS-0281 /
  RXS-0282 追加式修订)。
- **图合法性**(read-before-write / 写写冲突 / 同 pass 同资源读写反馈 / 声明↔反射失配〔RXS-0273〕)→ 装配期确定性拒
  (库层状态值零新码,镜像 RX6029 / RX6030 口径)。
- **present 执行**:present 前布局迁移(COLOR_ATTACHMENT_OPTIMAL / RENDER_TARGET → PRESENT_SRC / PRESENT)+
  ① **headless readback 校验**(RXS-0222 纪律,CI 判据)② 窗口腿复用 RXS-0197 / 0198 typestate + C++ shim
  (D-130 0-byte;窗口腿 device 见证由 G4.6 BLACKHOLE 真实窗口路径承载,本条不以窗口为验收前提)。

**Implementation Requirements**:

- rhi.rs `RhiGraph` 扩 gfx pass / 资源记录与 `seal()` 桥接(纯 host safe 码,`#![forbid(unsafe_code)]` 面);vk.rs 新增
  RHI 图形执行入口(消费 `PlannedBarrier` + 经 artifacts v2 来的 .rx 源 SPIR-V + RHI 资源表;既有
  `run_graphics_offscreen` / `run_graphics_offscreen_v2` / `run_mesh_offscreen` / `run_graph_offscreen` 入口 **0-byte
  语义**,新 FFI 沿 U27 / U30 审计模式登记 U31+);推导产物 golden 锚定(同图同参推导逐字节一致);装配期拒红绿语料。

> 测试锚定:conformance/uc05/reject/gfx_undeclared_write.rx(漏声明写 → 装配期拒)+ reject/gfx_write_write.rx(写写冲突
> → 装配期拒)+ reject/gfx_present_not_last.rx(present 非末位 → 装配期拒)+ 推导 golden 单测 + 步骤 76 device 出图
> 像素判据。

### RXS-0273 图形 pass 声明↔反射相等（着色对并集规则，G4.2，RFC-0015 §4.A3 / C-F5 disposition）

**Legality**:

- **反射集 = 逐阶段函数签名资源形参的并集**:编译器(typeck,镜像 I4 既有机制 RXS-0257)自 raster / mesh 着色对的**两个**
  函数签名分别提取资源形参(`Texture2D` / `TextureRw2D` / storage buffer / `AccelStruct` 形参),按**资源身份**合并
  (vs 与 fs 引同一资源者计一);compute pass 反射机制(kind-2 槽)**0-byte**。
- **sampler 与 texture_table 计入反射并集但标「无状态访问」类**:barrier 相等域**只核资源状态访问**
  (color / depth / texture / uav / readback);sampler / table 另核**绑定完备性**(pass 着色函数用到而图未绑定 →
  装配期确定性拒,同库层状态值零新码)。
- **声明↔反射双向精确相等**(镜像 RX6030 口径):声明集与反射集不等(漏声明 / 声明未用)→ **装配期确定性拒**
  (库层状态值,**零新 RX 码**;拦截计入语言 / 编译器面,与 RXS-0257 I4 同口径)。

**Dynamic Semantics**:

- 反射喂入于 `raster_pass` / `mesh_pass` 声明时点由编译器完成(typeck 静态提取 → cabi `rxrt_rhi_gfx_bind` 反射集槽);
  seal 时双向相等核验一次。

**Implementation Requirements**:

- typeck 反射集提取(着色对并集)+ mir_build 反射集物化 + rhi.rs `with_reflection` 同面扩展;**零新 RX 码**。

> 测试锚定:conformance/uc05/reject/gfx_undeclared_texture.rx(着色函数采样未声明纹理 → 装配期拒)+
> reject/gfx_declared_unused.rx(声明未用 → 装配期拒)+ reject/gfx_sampler_unbound.rx(sampler 用到未绑定 → 装配期拒)。

### RXS-0274 present 面库化（终端 handoff 执行语义 + headless readback 判据 + RXS-0197 typestate 复用，G4.2 PR-C，RFC-0015 §4.A4）

**Legality**:

- **终端声明唯一且末位**:`g.present(&back)`（或 `present_handoff` 访问）每张图至多一次且必须为声明序末 pass——违例 → 装配期确定性拒(库层状态值零新码,RXS-0272 同口径)。
- **窗口腿 = RXS-0197 / 0198 typestate 复用 0-byte**:RHI 应用的窗口呈现走既有 `Present` / `Ready` / `Acquired` /
  `Presentable` 消费式帧状态机 + backbuffer 借用缓冲(语义本体 0-byte;窗/泵/交换链维持 C++ shim,**D-130 红线不动**);
  RHI 图产出的 color target 经 blit/拷贝进入 backbuffer 借用缓冲(RXS-0198 契约面),或经 handoff 直接成为 present 源。
- **headless readback 判据(RXS-0222 纪律)**:present 终端在 CI/无显示环境以 readback 像素断言兑现「呈现前内容正确」——
  三断言点(首帧 / 重建后 / 末帧)沿 RXS-0222;无显示环境 SKIP = dev-env degrade(`RURIX_REQUIRE_REAL=1` 翻硬红),
  **mock / SKIP 不充绿**。

**Dynamic Semantics**:

- present 执行的 barrier 语义 = `PresentHandoff`(COLOR_ATTACHMENT_OPTIMAL → PRESENT_SRC_KHR / RENDER_TARGET → PRESENT,
  RXS-0238 映射表既有锚)由自动推导承载;窗口腿 device 见证(可见窗口 flip-model present)由 **G4.6 BLACKHOLE** 真实窗口
  路径承载,本条不以窗口为验收前提(术语:handoff = 图内终端声明,present session = 宿主帧状态机,两者经 backbuffer 契约
  胶合)。

**Implementation Requirements**:

- handoff 迁移由 vk 执行器在末 pass 后执行;窗口腿经既有 `rxp_*` shim 面(RXS-0197 IR),`rurix-d3d12` real-shim 构建
  面见 G4.6 归因;**零新 RX 码**。

> 测试锚定:conformance/uc05/reject/gfx_present_not_last.rx(present 非末位 → 装配期拒)+ reject/gfx_present_twice.rx
> (双 present → 装配期拒)+ 步骤 76 present 迁移 + headless readback 像素判据。

### RXS-0276 RHI bindless 面（TextureTable 入 pass，descriptor-indexing 运行时面复用，G4.2 PR-C，RFC-0015 §4.A6）

**Syntax**(bindless 绑定声明):

```
let table = rhi.texture_table();          // RXS-0235 TextureTable 薄映射(RXS-0271 资源面)
table.register(&tex_a);                   // 注册 → 稳定单调索引 u32(宿主侧,图外)
table.register(&tex_b);
gfx.reads_table(&table);                  // pass 绑定无界纹理表(动态索引面,着色侧 RXS-0231/0232 0-byte)
```

**Legality**:

- `texture_table()` 产 `Res<C>` 族句柄(table 类);`register(&tex)` 为宿主侧注册(图外,单调索引);`reads_table(&table)`
  为 pass 级绑定声明——计入反射并集但标「无状态访问」类(RXS-0273:barrier 相等域不核,绑定完备性另核)。
- 着色侧:无界句柄数组 `[Texture2D<F>]` 形参 + 动态索引 + `nonuniform` 标注维持 **RXS-0231 / RXS-0232 0-byte**
  (缺标注 → RX3016 既有);表内纹理生命周期须覆盖 pass 执行(宿主义务,类型面不拦截——诚实标注)。
- **feature chain 探测**:descriptor-indexing 运行时能力缺失 → **确定性 `Err`**(RXS-0235 同口径,非 fake pass)。

**Dynamic Semantics**:

- 执行 = vk descriptor-indexing 面既有底座复用(`run_graphics_offscreen_bindless` 同族 descriptor 写,update-after-bind
  沿 G3.4 运行时面);barrier 推导对 table 内纹理按**集合整体 ShaderRead** 保守迁移(单一事实源 = graph.rs 映射表)。
- bindless 与有界 `reads(&tex)` 在同图共存合法;表内容与有界纹理重复注册不拒(宿主义务管理)。

**Implementation Requirements**:

- cabi 资源类 5=table 的 descriptor 装配(RXS-0230 底座);vk.rs 执行器 table 绑定路径沿 U27 扩注;**零新 RX 码**;
  像素判据 = 四象限动态索引四色 + 篡改注册序换位 RED(G3.4 步骤 64 同判据移植)。

> 测试锚定:conformance/uc05/accept/gfx_bindless.rx(表注册 + reads_table 声明 0 诊断)+ 步骤 76 bindless 动态索引像素
> 判据(四象限四色 + 篡改换位 RED)。

### RXS-0277 engine_host v3 嵌入面 + 三方数值精确相等判据(G4.2 PR-D,RFC-0015 §4.A7)

**Syntax**(图形导出 + 宿主嵌入 C ABI):

```
#[export(c)]
fn uc05_gfx_run_frame(out: *mut u32, w: i32, h: i32) -> i32
```

**Legality**:

- **图形嵌入面(RXS-0261 嵌入面的图形侧扩展)**:整张图形 RHI 图(≥1 raster + ≥1 mesh pass,含 color_target / 装配
  核验 / hazard 推导 / 派发 / 真像素读回)可完整封闭在一个 `#[export(c)]` host fn 体内经 `--emit=dll` 产 cdylib 供
  C/C++ 宿主调用,**EI1.4 同构**(subset v1:标量 + 裸指针,无 upcall、无外部固定 ABI;PR-G 判档输入)。GPU 上下文 /
  图 / 资源生命周期**单次调用内创建与销毁**;宿主只见 C ABI 标量与裸指针、不见任何 Rurix 类型。
- **engine_host v3 嵌入宿主(RXS-0261 嵌入面的宿主侧升级)**:`src/rurix-engine/harness/` 新增 C++/D3D12 文件,链接
  `rurix_rhi.lib` 图形导出面 device 真跑;**v1/v2 既有资产逐字节 0-byte**(两制共存,RXS-0254 §4.A5 同面)。宿主建立
  D3D12 device + queue + fence 上下文,与 Rurix 侧 Vulkan device 经 **LUID 匹配**(同 adapter,跨 API 共享 GPU 节点
  时间轴;v2 LUID 匹配为 CUDA↔D3D12,v3 升级为 Vulkan↔D3D12),Rurix 图形图节点夹在宿主 fence 锚点之间执行——证图
  节点在宿主时间轴上有确定位置(非「另起一条无关时间线」)。
- **三方数值精确相等判据(Q-PixelCriterion)**:**不设 ULP 浮点容差**。三方为:① .rx RHI(Vulkan)readback 像素
  (`uc05_gfx_run_frame` 出参 `*out`);② D3D12 宿主 raster/mesh pipeline readback 像素(D3D12 graphics pipeline
  与 mesh pipeline 同图对照);③ host 参考值(闭式参考公式,C 实现与 .rx 侧不同实现——对照非自证)。**相等域 =
  纯色/nearest RGBA8 整数 fetch 域**:无纹理过滤、无混合、无 depth 写入、无多采样(避免浮点不确定性来源),像素为
  RGBA8 整数 texel fetch 严格逐字节相等。**超域换用例不降判据**(不换到带过滤/混合用例后放宽到 ULP 容差;Q-
  PixelCriterion 是判据硬约束,P-12 不以「完整」为名放宽)。
- **生成头逐字节守卫(RXS-0254 同面)**:CI 再生成 `rurix_rhi.h` 与仓库 tracked 头逐字节比对,篡改翻红。仓库**零 tracked
  .h**(生成头由 `rurixc --emit=dll` 现场生成,RXS-0253);守卫与 v2 嵌入面同 RXS-0254 守卫同面。
- **subset v1 合规(RXS-0251/0255)**:签名仅 `*mut u32` + `i32` + `i32` 返回;导出体**无 panic 面 by-construction**——
  错误面经 **i32 状态码**返回(RD-026 无 Result 面纪律),不跨 ABI 展开、不 panic 终止。

**Dynamic Semantics**:

- 执行序沿用 RXS-0261 嵌入面零特例:seal → 推导 → 派发 → 收尾同步 → D2H,与 in-EXE 路逐段同一,差别仅在结果出口
  是 `*mut u32` 出参(像素缓冲逐 RGBA8 写回)。三方数值对照于宿主侧 CPU 比对(逐字节 memcmp)。
- engine_host v3 宿主侧 D3D12 pipeline:raster 对照(vs/ps,d3dcompiler 或预编 cso)、mesh 对照(ms_6_5/ps_6_5,
  dxc 预编,Vulkan SDK dxc 在 provisioning);LUID 匹配保证 Vulkan 与 D3D12 device 共同一 adapter。

**Implementation Requirements**:

- 新增文件 `src/rurix-engine/harness/engine_host_v3.cpp`;`engine_host.cpp`(v1)/ `uc05_engine_host.cpp`(v2)**逐字节
  0-byte**。cabi 图形导出槽位扩展(class / access tag 追加式,与 PR-C 同源;PR-D 不新增 cabi 符号面,图形导出经既有
  `rxrt_rhi_*` 族 + `#[export(c)]` 收集根)。零新 RX 码、零新借用码、零新 lang item。
- 步骤 78 `ci/uc05_engine_embed_v3_smoke.py`:host 段恒跑(零 .rs 审计 / 生成头幂等 / 篡改再生成 byte-diff RED /
  `--emit=dll` 产图形导出面);device 段 gate real(cl.exe 编 v3 harness 链 `rurix_rhi.lib` 真跑 + 三方数值精确相等 +
  RED 三路:篡改 .rx 侧参考 / 篡改 D3D12 侧参考 / 篡改 host 侧参考任一翻红)。RURIX_REQUIRE_REAL=1 翻硬红;缺
  provisioning 环境 SKIP = dev-env degrade(非 fake pass)。

> 测试锚定:apps/uc05-rhi/src/embed.rx(`uc05_gfx_run_frame` 图形导出,`//@ spec: RXS-0277`)+
> src/rurix-engine/harness/engine_host_v3.cpp(v3 嵌入宿主三方数值精确相等,`//@ spec: RXS-0277`)+
> ci/uc05_engine_embed_v3_smoke.py(步骤 78 device 段 cl.exe 编 v3 + 三方像素逐字节相等 + RED 三路)+
> g4.counter.engine_embed_v3(device 见证计数 ≥1,evaluator 分支同 PR)。

### RXS-0280 transient 别名复用分配器 + 执行期峰值计数器(G4.3 PR-E,RD-035 执行面①,RFC-0014 §4.B8)

**Syntax**(纯 host safe 码面,`#![forbid(unsafe_code)]`;区间图着色分配器 API):

```
AliasAlloc::new() -> AliasAlloc                                  // 空分配器(纯 host 状态)
alias_alloc.assign(lifetimes: &[(ResourceId, LiveRange, Size, Align)]) -> AliasPlan
                                                                 // 区间图着色 → 槽分配
AliasPlan { slots: Vec<SlotAssignment>, peak_bytes: u64 }        // 着色产物 + 峰值字节
PeakCounter::new(declared_capacity: u64) -> PeakCounter          // 执行期峰值计数器
peak_counter.on_alloc(bytes: u64)                                // 回放期分配事件记账
peak_counter.on_free(bytes: u64)                                 // 回放期释放事件记账
peak_counter.peak_bytes() -> u64                                 // 并发存活字节峰值
```

**Legality**(纯 host safe 码,`#![forbid(unsafe_code)]` 面;零新 RX 码):

- **生命期区间定义**:sealed 图上每个 transient 资源生命期区间 = `[首写 pass 序位, 末读 pass 序位]`
  (含端点;无写者的资源不参别名复用——保守分配独立槽;仅读者归 RXS-0262 host 记账面)。
- **区间不重叠者共享同一设备分配**(区间图着色,纯 host safe 码):两资源生命期区间 `[a0, a1]` 与
  `[b0, b1]` 满足 `a1 < b0 || b1 < a0`(严格不重叠,端点不含)→ 可共享同一槽;重叠则必异槽。
- **尺寸/对齐三分量着色**:同槽组按 `max(成员尺寸)` + `max(成员对齐)` 分配(逐成员核满足性:
  实际分配字节 ≥ 每成员尺寸、对齐 ≥ 每成员对齐)。槽内复用成员表 monotone 追加。
- **执行期峰值计数器**:回放期随分配/释放事件记账并发存活字节峰值(cabi 真实设备分配驱动,
  非静态推算)。`PeakCounter::on_alloc(bytes)` / `on_free(bytes)` 在 cabi 真实设备分配/释放时
  由执行器调用,`peak_bytes()` 返回观测到的最大并发存活字节。
- **I10 自 report_only 升 measured**:峰值 < 声明容量可 device 见证(`peak_bytes() < declared_capacity`,
  非平凡成立——别名复用使实际并发存活收紧)。**I10 矩阵档位**:自 `report_only` 升 `measured_local`
  (步骤 79 纯 host mock device 分配 + cabi 真实设备分配驱动两面)。

**Dynamic Semantics**:

- 别名着色在**重排后 DAG** 上重算(RXS-0281):seal → 调度(拓扑分层)→ 着色(在调度后序上算生命期区间)
  → 回放(按调度序派发 + 峰值计数器记账)。**B1 分配器输入 = 最终执行计划,单一事实源**。
- seal → 调度 → 着色 → 回放**四序固定闭合漂移窗口**:调度/着色/回放三段共享同一执行计划,
  执行器禁二次推导或重映射(P-11 单一事实源;镜像 RXS-0240 `run_graph` 先例)。
- 峰值计数器在回放期逐 alloc/free 事件记账;`on_alloc` 增当前存活、`on_free` 减之,`peak_bytes`
  为期间最大值。**cabi 真实设备分配驱动**:执行器在 `rxrt_rhi_resource` 真设备分配时调 `on_alloc`,
  释放时调 `on_free`;mock device 段(步骤 79 纯 host)用模拟分配事件驱动。

**Implementation Requirements**:

- 新增模块 `src/rurix-rt/src/alias_alloc.rs`(**纯 host safe 码**,`#![forbid(unsafe_code)]`);
  - 区间图着色算法(贪心着色,按生命期起点排序;O(n²) 可接受,无堆集合约束用 `Vec` 承载);
  - 三分量着色(尺寸/对齐 max 合并;逐成员核满足性);
  - `PeakCounter` 简单计数器(`current_bytes` + `peak_bytes`,`on_alloc`/`on_free` 更新)。
- rhi.rs `RhiGraph` 暴露 `derive_alias_plan(&ExecPlan) -> AliasPlan`(在调度后序上算生命期区间 → 着色);
  `execute()` 在派发期对每个 transient 资源的真设备分配/释放调 `PeakCounter::on_alloc/on_free`。
- **零新 RX 码、零新 lang item、零新借用码**;纯库层状态值,不占编译器段位。
- I10 矩阵 / evidence / 步骤 79 同步迁档(`report_only` → `measured_local`)。

> 测试锚定:rurix-rt `alias_alloc.rs` 库单测(重叠区间不共享 / 不重叠区间共享 / 尺寸对齐满足性 /
> 峰值计数器单调性 + 释放后回落)+ rhi.rs `derive_alias_plan` golden 单测(同图同参逐字节一致)+
> 步骤 79 `ci/uc05_exec_face_gate.py` 纯 host 恒跑(别名复用 + 峰值计数器 < 声明容量 mock device 断言)。

### RXS-0281 重排执行模型(G4.3 PR-E,RD-035 执行面②,RFC-0014 §4.B9)

**Syntax**(纯 host safe 码面,DAG 拓扑分层 + 批级提交):

```
ExecPlan { layers: Vec<Layer>, batch_submit: bool }              // 调度计划(纯 host 产物)
Layer { pass_indices: Vec<usize> }                               // 同层独立 pass(可换序/批级提交)
derive_exec_plan(sealed_graph: &RhiGraph) -> ExecPlan            // DAG 拓扑分层(纯函数)
```

**Legality**(纯 host safe 码,`#![forbid(unsafe_code)]` 面;零新 RX 码):

- **依赖 DAG**:sealed 图建依赖 DAG(RAW/WAW/WAR 边,复用 RXS-0258 hazard 推导的边集)→ 拓扑分层。
  同层 pass 互相独立(无跨 pass 资源依赖),可换序;层间须屏障(全序 happens-before,RXS-0239 既有承诺)。
- **同层独立 pass 可换序**:同层 pass 集合任意排列均保持依赖闭包(核验器独立重建闭包逐边核,
  RXS-0282 I11 拦截项);换序后 alias 着色在调度后序上重算(RXS-0280)。
- **批级提交**:单 queue 一次提交多 pass(同层 pass 批量录制到同一 command buffer,层间屏障);
  GPU 管线重叠(同层 pass 在硬件允许时可重叠执行,执行器不强制串行)。**多 queue 仍 out-of-scope**
  (单 queue 批级提交 = G4.3 兑现面,多 queue 调度归 RD-035+ 后续期)。
- **依赖保持性**:重排后执行计划的依赖闭包 ⊇ 原声明序的依赖闭包(核验器独立重建逐边核;
  丢边即 I11 拦截项红,RXS-0282)。**严禁丢边**:任一 RAW/WAW/WAR 边在重排后须仍由层间全序裁定。

**Dynamic Semantics**:

- 调度 = 纯函数 `derive_exec_plan(sealed_graph) -> ExecPlan`;输入 = sealed 图,输出 = 拓扑分层计划。
  同图 → 逐字节相同计划(golden 可锚;确定性,镜像 `derive_syncs` / `derive_barriers` 先例)。
- 执行器按 `ExecPlan` 逐层派发:同层 pass 批量录制(单次 `vkCmdBeginRenderPass`/`cuLaunchKernel` ×N),
  层间 `vkCmdPipelineBarrier` / `cuStreamSynchronize` 屏障;执行器禁二次推导(P-11)。
- **执行序 ≠ 声明序**(重排后):同层 pass 可换序,层间序由 DAG 拓扑裁定。`PlannedSync` 同步点
  在重排后序上重算(RXS-0280 别名着色同源)。

**Implementation Requirements**:

- 新增模块 `src/rurix-rt/src/scheduler.rs`(**纯 host safe 码**,`#![forbid(unsafe_code)]`);
  - DAG 建图(RAW/WAW/WAR 边,复用 `derive_syncs` 的 hazard 推导边集);
  - 拓扑分层(Kahn 算法或等价;同层 = 同拓扑深度 + 互独立);
  - 批级提交计划(`Layer` 数组,每层 pass 索引集合)。
- rhi.rs `RhiGraph::derive_exec_plan() -> ExecPlan`(纯函数,sealed 后可调);
  `execute()` 改用 `ExecPlan` 派发(层间屏障 + 同层批级;既有 `derive_syncs` 0-byte 保留为兼容路)。
- **零新 RX 码、零新 lang item、零新借用码**;纯库层状态值。
- RXS-0261 追加式修订:顺序调度 → **依赖保持下的重排/批级调度**(本条兑现;既有承诺字面不动)。

> 测试锚定:rurix-rt `scheduler.rs` 库单测(线性图单层 / 菱形依赖双层 / 独立 pass 同层 / 依赖保持性
> golden)+ rhi.rs `derive_exec_plan` 确定性单测 + 步骤 79 `ci/uc05_exec_face_gate.py` 纯 host 恒跑
> (DAG 重排依赖保持性 red_self_test 双向)。

### RXS-0282 I11 拦截项 + RXS-0239/0261 追加式修订行(G4.3 PR-E,RD-035 执行面③,RFC-0014 §4.B10)

**Syntax**(调度器与核验器两独立纯函数,互不导入;D6 互证先例):

```
verify_exec_plan(sealed_graph: &RhiGraph, plan: &ExecPlan) -> Result<()>  // 核验器(独立重建依赖闭包)
red_self_test_scheduler_drops_edge() -> bool   // 桩化调度器丢边 → 核验器检出(双向红)
red_self_test_verifier_dropped() -> bool       // 桩化核验器被门检出(调度器侧测试)
```

**Legality**(纯 host safe 码,`#![forbid(unsafe_code)]` 面;零新 RX 码):

- **调度器与核验器两独立纯函数(互不导入,D6 互证先例)**:`derive_exec_plan`(调度器,产 `ExecPlan`)
  与 `verify_exec_plan`(核验器,独立重建依赖闭包逐边核)为**两独立模块**,互不 import 对方推导逻辑
  (镜像 G3.5 `graph.rs` 禁 import `uc04-demo barrier.rs` 的 D6 互证纪律)。核验器**自 sealed 图独立
  重建**依赖闭包(RAW/WAW/WAR 边),逐边核 `ExecPlan` 是否保持(丢边即 Err)。
- **red_self_test 双向**:
  - **桩化调度器丢边被拦**:构造一个故意丢边的桩 `derive_exec_plan_faulty`,核验器须检出并 Err;
  - **桩化核验器被门检出**:构造一个不核边的桩 `verify_exec_plan_faulty`,调度器侧测试须检出
    (执行计划注入丢边,桩核验器不拦 → 测试门检红)。
- **demo 图手算期望调度 golden 锚**:`apps/uc05-rhi/src/demo.rx` 三 pass 线性图 → 期望 `ExecPlan`
  为单层(三 pass RAW 链,无独立 pass)或等价 golden(手算可核);golden 单测逐字节比对。
- **I11 入不变量矩阵(漏拦即红)**:I11 = 调度器/核验器丢边拦截项,入 RXS-0263 矩阵 I11 行
  (档 = 装配期/库测,机制 = 两独立纯函数 + red_self_test 双向,证据级 = ci_checked 步骤 79)。
- **RXS-0239 追加「重排执行模型」段**(严禁改写既有承诺字面):RXS-0239 既有「单 queue;声明序 =
  提交序 = pass 粒度完成序」承诺**字面不动**;追加段明记 G4.3 PR-E 后**单 queue 批级提交下**,
  pass 边界全序 happens-before 仍由层间屏障裁定(同层 pass 互独立无跨资源依赖,层间序 = 全序)。
- **RXS-0261 顺序调度 → 依赖保持下的重排/批级调度**(追加式修订,既有字面不动):RXS-0261 既有
  「顺序调度 + 显式 sync」承诺**字面不动**;追加段明记 G4.3 PR-E 后**依赖保持下的重排/批级调度**
  为执行模型升级(单 queue 批级,多 queue 仍 out-of-scope)。

**Dynamic Semantics**:

- 核验器在 `execute()` 派发前**严格先于**调用(pre-dispatch fail-closed):`verify_exec_plan` 失败
  则一个 kernel 也不派发(镜像 seal 严格先于派发的纪律)。
- red_self_test 为**纯 host 库单测**(步骤 79 恒跑,无 GPU 依赖);双向断言两独立纯函数互证。

**Implementation Requirements**:

- 新增模块 `src/rurix-rt/src/scheduler.rs` 内 `verify_exec_plan`(核验器,与 `derive_exec_plan`
  同模块但**独立函数**——禁共享推导辅助函数的内部状态,只读 sealed 图 + ExecPlan 入参);
  - 独立重建依赖闭包(不调 `derive_exec_plan` 的内部函数);
  - 逐边核 ExecPlan 是否保持(层间序覆盖所有 RAW/WAW/WAR 边);
  - Err = 库层状态值(镜像 RXS-0258 Structure 口径,零新码)。
- red_self_test 双向单测在 `scheduler.rs` `#[cfg(test)]` 内;
- I11 入 `evidence/uc05_invariant_matrix.json` 矩阵 I11 行(步骤 79 三方一致)。
- **零新 RX 码、零新 lang item、零新借用码**。

> 测试锚定:rurix-rt `scheduler.rs` 库单测(`verify_exec_plan` 拒丢边 + red_self_test 双向 +
> demo 图手算 golden)+ 步骤 79 `ci/uc05_exec_face_gate.py` I11 双向 red_self_test 断言 +
> 矩阵 I11 行三方一致(矩阵 ↔ 语料 ↔ report.md)。

### RXS-0283 const 容量接线 + RXS-0262 收窄段更新(G4.3 PR-E,RD-035 执行面 const 接线,RFC-0014 §4.B11)

**Syntax**(`rhi.graph::<CAP>()` lang-item 已知方法调用点 turbofish const 实参):

```
rhi.graph::<CAP>() -> Graph<C>        // CAP = const 泛型实参(turbofish 语法,字面量即时求值)
                                      // → 普通 i64 cabi 实参(CAP 不进类型参数表,无 RD-007 依赖)
```

**Legality**(编译期越界拒 + non-static construction 拒;零新 RX 码,复用既有 const/类型诊断):

- **turbofish const 实参**:`rhi.graph::<CAP>()` 的 `CAP` 须为**字面量即时求值**(const eval,
  单函数体 affine 单定义链前向扫描)→ 普通 i64 cabi 实参传 `rxrt_rhi_graph_create(rhi, cap: i64)`。
  **CAP 不进类型参数表**(无 RD-007 const 泛型依赖,零新类型面机制)。
- **编译期越界拒**(typeck/MIR 层有界局部分析):同函数体内 `rhi.resource(n)` 调用计数 > CAP →
  **编译期拒**(复用既有 const/类型诊断,**零新码**;单函数体 affine 单定义链前向扫描,
  不跨函数/不跨分支)。
- **循环/条件/跨函数构建 → strict 拒 non-static construction**:`rhi.graph::<CAP>()` 出现在
  循环/条件/跨函数体 → **strict 拒**(non-static construction,复用既有 const eval 拒诊断,零新码)。
  仅**单函数体直链构建**合法(affine 单定义链,RD-026 无堆集合对策)。
- **RXS-0262 收窄段更新**(Vec 承载 → const 容量接线兑现):RXS-0262 既有「const 泛型定长数组 +
  编译期越界拒的 `.rx` 接线随后续期落地」**字面不动**;追加段明记 G4.3 PR-E 后**const 容量接线
  已兑现**(`rhi.graph::<CAP>()` lang-item + 编译期越界拒 + non-static 拒)。host 侧 `RhiGraph`
  仍以 `Vec` 承载(runtime-bounded),const CAP 经 cabi i64 实参传入,host 侧记账核验
  (resource_count > cap → 装配期 Structure Err,库层状态值零新码)。

**Dynamic Semantics**:

- `rhi.graph::<CAP>()` 求值 = 调用 `rxrt_rhi_graph_create(rhi, cap: i64)`(cap = 字面量即时求值);
  返回 `Graph<C>` affine 句柄(brand `C` 与 `Rhi` 同源,跨 brand → RX3006 复用)。
- 编译期越界拒在 typeck 阶段判定(单函数体 resource() 计数 vs CAP);装配期 host 侧二次核验
  (resource_count > cap → Structure Err,防御 in-depth)。

**Implementation Requirements**:

- `src/rurixc/src/resolve.rs`:Rhi lang-item 已知方法 `graph` 注册(turbofish const 实参识别);
- `src/rurixc/src/typeck.rs`:`graph::<CAP>()` turbofish const 实参求值(字面量即时求值 → i64);
  单函数体 resource() 计数 vs CAP 越界拒(复用既有 const 诊断);循环/条件/跨函数 strict 拒;
- `src/rurixc/src/mir_build.rs`:`Op::RhiGraph` 物化(cap 作为 i64 cabi 实参下发 `rxrt_rhi_graph_create`);
- `src/rurix-rt/src/rhi.rs`:`RhiGraph` 增 `declared_capacity: Option<u64>` 字段(graph() 调用时记录);
  `resource()` 时核 `resource_count > declared_capacity → Structure Err`(装配期防御);
- `src/rurix-rt-cabi/src/lib.rs`:`rxrt_rhi_graph_create(rhi: u64, cap: i64) -> u64` 新符号(只追加);
- **零新 RX 码、零新 lang item**(`graph` 为 Rhi 已知方法扩面,镜像 RXS-0190 分支先例);
  零新借用码(CAP 不进类型参数表,无 brand/affine 新面)。

> 测试锚定:conformance/uc05/reject/transient_capacity_overflow.rx(声明第 9 个资源 CAP=8 → 编译期拒,
> `//@ expect-error: RX2010` 或既有 const 诊断)+ reject/nonstatic_graph_construction.rx(循环构建图 →
> strict 拒 non-static construction)+ rurix-rt rhi.rs `rejects_transient_capacity_overflow` 库单测
> (host 侧装配期防御)+ 步骤 79 `ci/uc05_exec_face_gate.py` reject 语料逐条断言。

### RXS-0319 VB/IB 声明算子与布局推导律（G8.2 M89，RFC-0019 RP-GFX-SUBMIT）

#### Syntax

`rhi.vertex_data(&VERTS)` / `rhi.index_data(&INDICES)` / `pass.draw(&vb, vertex_count)` /
`pass.draw_indexed(&vb, &ib, index_count)`——RHI 库面已知方法（builder 风格，沿
`RhiRasterPass` 机械全套）。

#### Legality

- **`vertex_data`**:`VERTS` 为 host 侧定长数组（`[f32; N]` 等）；cabi
  `rxrt_rhi_vb_create(rhi, ptr, bytes, stride)` 拷贝入设备缓冲。
- **VB 布局单源律（P-11）**:`stride` 与顶点属性布局**不由用户声明**——由该 pass
  vertex shader 的 io **输入表**推导（`iface_extract::io_sig_for` 同一提取律）：声明序
  紧凑交错，location/format/offset 全部来自 reflection；与用户手写 stride 不符 =
  seal 拒（RXS-0326）。
- **`index_data`**:首期索引类型冻结为 **u32**；cabi `rxrt_rhi_ib_create`。
- **`draw` / `draw_indexed`**:登记进 `GfxPassRecord`；cabi
  `rxrt_rhi_gfx_draw(pass, vb, ib_or_0, count)`。
- **`rxrt_rhi_raster_pass` vs/fs 符号**:必须登记进 pass 记录并按名索引 artifacts v2
  入口表（`spirv_entries`）；**禁止忽略符号**（RD-037 历史洞关闭面）。
- builder sig 违例走既有 typeck（RX2010/RX4001 等），**零新 RX 码**。

#### Implementation Requirements

- 实现锚定：`hir`/`typeck`/`mir_build` 加性 Op + cabi 三符号只追加；每条款 ≥1
  `//@ spec: RXS-0319` 锚定。

### RXS-0320 装配核验与越界拒（G8.2 M89）

#### Legality

- **范围核验**（submit 前 fail-closed，库层状态值，零新 RX 码）：
  - `draw_indexed`：`index_count * 4 ≤ ib.bytes`；索引引用的最大顶点号 × stride
    `≤ vb.bytes`（host 可核部分）。
  - `draw`：`vertex_count * stride ≤ vb.bytes`。
- **声明集 ↔ reflection 双向精确相等**（既有 seal 骨架扩面）：VS io 输入表 ↔ VB
  stride/attrs 一致；FS 输出 ↔ color target 在位；任一失配 → 装配拒。
- **artifact 绑定**：vs/fs 名必须命中 artifacts v2 入口；篡改符号 → 装配拒 RED。
- 越界/失配在 **submit 前**拒；禁止提交部分装配状态。

#### Implementation Requirements

- reject 语料：`conformance/gfx_submit/reject/{draw_ib_oob,draw_vb_range_oob,
  draw_without_vb,vs_io_vb_mismatch}.rx`（EXE RED，库层 [structure]/[capacity]）。
- ≥1 `//@ spec: RXS-0320` 锚定。

### RXS-0321 gfx 派发臂与 readback provenance（G8.2 M89）

#### Dynamic Semantics

- **`rxrt_rhi_submit` gfx 派发臂**（Vk 分支）：gfx pass 若绑定了 draw + vs/fs →
  组扩展 raster 描述（含可选 IB）→ `run_rhi_graphics_offscreen_v2`（U31 扩注：IB 绑定 +
  `vkCmdBindIndexBuffer` + `vkCmdDrawIndexed`；barrier plan 逐字重放）→ 输出写回 host
  镜像 → `rxrt_rhi_readback` 消费。
- **未绑 draw** 的 gfx pass 维持「仅参 barrier 推导」（既有语料 0 回归）。
- **真实 artifacts 消费**：SPIR-V 必须来自 artifacts v2 按名索引；**禁止**固定最小
  SPIR-V 模块替身、**禁止** host 填像素替身（RD-037 字面）。
- **readback provenance**：仅在图完成点后、指向**同一 submit generation** 的 color
  target；device 不满足 profile → fail-closed（不静默降级）。
- **通用 dump**：运行时 env `RURIX_RHI_READBACK_DUMP=<path>`（通用特性，非
  workload 专用 `.rs`）供 smoke 与 checked-in golden 逐字节对拍。
- **零 Rust 宿主判据**：fixture/启动链 `rust_host_source_count == 0`；编译成功或仅
  非零像素**不算 PASS**（须像素 golden + validation=0）。

#### Implementation Requirements

- accept：`conformance/gfx_submit/accept/m89_two_tri_quad.rx`（两直角三角形拼满屏
  quad，flat 色，整数域零容差）+ `vb_only_draw.rx`。
- smoke：`ci/g8_single_source_gfx_smoke.py`；device 段 `RURIX_REQUIRE_REAL=1`。
- ≥1 `//@ spec: RXS-0321` 锚定。RD-037 三件套齐绿后一次 close。

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
| v1.8 | 2026-08-06 | **G8.2 M89 spec-first:落带编号条款体 `### RXS-0319` ~ `### RXS-0321`**(RP-GFX-SUBMIT,硬规则 7 条款先行;设计案规划参考号 0325~0327 **不预占**,按 ledger 实测 next_free=319 顺位领取)。RXS-0319 VB/IB 声明算子与布局推导律(`vertex_data`/`index_data`/`draw(_indexed)`;VB 布局 = VS io 输入表单源推导 P-11;u32 索引首期冻结;raster_pass vs/fs 符号必须登记)/ RXS-0320 装配核验与越界拒(draw↔VB/IB 范围、声明↔reflection 双向相等、artifact 按名绑定,submit 前 fail-closed,库层状态值零新 RX 码)/ RXS-0321 gfx 派发臂与 readback provenance(真实 artifacts v2 消费禁固定最小 SPIR-V/host 像素替身;readback 同 submit generation;`RURIX_RHI_READBACK_DUMP` 通用 dump;零 Rust 宿主;`compile_only_not_pass`)。依据 [RFC-0019](../rfcs/0019-rendering-platform.md) RP-GFX-SUBMIT + G8_ACCEPTANCE_MAP §2 M89 行 + 设计案 §6 + RD-037。既有 RXS-0256~0283 0-byte;零新 RX 码零新 U(U31 扩注方向)。 | **Full RFC**(RFC-0019) |
| v1.2 | 2026-07-20 | EI1.4 实现落地:**RHI compute dispatch + I4 编译器反射喂入**(兑现 EI1.3 诚实归到 EI1.4 的两项收窄)。①**pass 绑 kernel**(RXS-0257):`rhi.pass(kernel, GridDim(..), BlockDim(..), (args..))`,形态与 `Stream::launch` **逐位同构**——tbir `RhiPassBind` 与 `GpuLaunch` 共用形态判据 `kernel_binding_form`、mir_build 共用 marshalling 物化 `gpu_marshal_args`、契约裁决共用 `launch_check::check_kernel_binding`(着色 RX3004 / 维度 RX3005 / 实参 RX2001 / brand RX3006,**零新码**);`launch_check::ty_compat` 补 `Ty::Const` 自反臂(缺之则同 `Rhi` 实例资源被误判跨 brand,镜像 typeck unify 同修)。②**I4 反射喂入接线兑现**(RXS-0257):编译器自 kernel 签名与绑定实参**静态提取**反射集(实参中的 `Res<C, T>`,由 launch_check 核对确落在 `View`/`ViewMut` 形参位)→ marshalling **kind-2 槽**(0=Buffer/1=标量/2=RHI 资源,`rxrt_launch` 槽契约只追加扩展)→ 新符号 `rxrt_rhi_bind` → `PassSpec::with_reflection` → `seal()` 双向精确相等核验;新语料 `conformance/uc05/assembly/pass_undeclared_read.rx`(kernel 触碰 {a,b} 只声明 `writes(&b)`)**真触发**库层 `ReflectionMismatch` → `rxrt_trap`(device EXE RED 真跑见证 `rhi_submit [reflection]`)。**I4 自 `lib_tested` 迁 `assembly_time` / 证据级 `ci_checked`**(矩阵 / 报告 / 步骤 73 门同步迁档;LIB_TESTED 档遂为空集)。③**真 compute dispatch**(RXS-0261):`rxrt_rhi_submit` 在 seal + `derive_syncs` **之后**按推导序真派发——推导计划**逐字重放**(每条 `PlannedSync` 令执行器在派发该 pass 前于本图 stream 落显式同步点,执行器禁二次推导),全部派发后收尾同步;派发本体复用 `rxrt_launch` 抽出的单一事实源 `launch_prepared`。`Res<C, T>` 获元素类型参数(镜像 `Buffer<C, T>`,元素经使用点推断 + RX2010 定型)并为**真设备分配**(`n * sizeof(T)`,`rxrt_rhi_resource(r, bytes)`;`rhi_destroy` 连带释放,释放前 ctx sync 封口)。④**真 D2H + 读回点迁 `Queue`**(RXS-0259):`queue.readback(res, &mut pinned)`(`rxrt_rhi_readback(r, src, dst, bytes)`)——readback 自 `Rhi` 迁 `Queue<C>` 使**「先派发、后读回」执行序由类型强制**(submit 前无 `Queue`,submit 后 `Rhi` 已消费),move-out affine 语义(I1/I2 → RX4001)不变。⑤**I9 device 落地**(RXS-0263):`apps/uc05-rhi/src/demo.rx` 升为两 pass 真算 + readback 求和 vs 闭式参考,机器可核 token `UC05_SUM` / `UC05_REF`,步骤 72 独立复核相等;**I9 仍留 report_only**(一次观测非全域证明)。⑥**诚实标注未兑现面**:pass 重排 / 依赖驱动并行调度(多 stream)未实现——声明全序即执行序,`PlannedSync` 作用是插同步点非重排;**I10 未完全兑现**——每 transient `Res` 生命期 = 图生命期故峰值恒等于声明容量(「≤ 声明容量」平凡成立而非因复用收紧),别名复用与执行期峰值计数器均未实现,随后续期。⑦步骤 72 RED 断言改为**按语料头 `//@ assembly-reject: <category>` 逐例核类别**(`structure` / `reflection`),GREEN 增 I9 数值对照;evidence schema 增 `demo_numeric`。零新 RX 码维持(全复用 + 库层状态值);零新 lang item。 | **Full RFC**(RFC-0014 / §4.B / EI1.4) |
| v1.1 | 2026-07-19 | EI1.3 PR-B2 实现落地 + **对抗性验证 disposition 诚实收窄**:①**readback 接线兑现**(RXS-0259):hir `Op::RhiReadback` + typeck(`rhi.readback(res)` 资源实参按值消费)+ mir_build(实参 `Operand::Move` → move 检查 RX4001)+ cabi `rxrt_rhi_readback(r, src)` affine 消费;readback 后再用 / 二次 readback → **RX4001**(I1/I2 真兑现,conformance/uc05/reject/res_use_after_move.rx + res_double_move.rx)。②**I4 诚实收窄**(RXS-0257/0263):I4 未声明访问核验机制(`with_reflection` 声明-反射相等)已实现 + 库测(`rejects_reflection_mismatch_i4`);`.rx` 编译器反射喂入(pass 绑 kernel)与 compute dispatch 耦合、随 **EI1.4** 落地——EI1.3 不宣称 I4 `.rx` 路 ci_checked,不锚 pass_undeclared_read.rx;矩阵 I4 证据级 = `lib_tested(EI1.3) / .rx_wiring:EI1.4`。③**I3/I5 装配期分档**(RXS-0263):I1/I2/I6/I7/I8 = **编译期**(typeck / --emit=check 即拦);I3/I5 = **装配期(图装配期)**(`submit()` 时 host 侧确定性拦,--emit=check CLEAN,submit 确定性 rxrt_trap);I4 = lib_tested;叙事改「I1~I8 = 100% **确定性**检测(编译期 OR 装配期确定性 / 库测机制),对照上一项目运行期概率性计数器可漏」,删对 I3/I4/I5 的「编译期即不可构造」过强表述。④**RXS-0262 诚实收窄**:EI1.3 兑现 host 侧容量记账(rhi.rs `transient_resource_capacity_accounting`),const 泛型定长数组编译期越界拒的 `.rx` 接线随后续期落地(Vec 承载 runtime-bounded)。⑤**RXS-0261 诚实收窄**:EI1.3 demo host 图 submit 装配核验通过 device 真跑(`UC05_RHI_OK`),pass 绑 kernel compute dispatch + 数值对照(I9)归 EI1.4。⑥语料迁 `conformance/uc05/{accept,reject,assembly}`(4 accept + 5 编译期 reject + 3 装配期,tests/uc05_corpus.rs 批跑)+ apps/uc05-rhi 零 .rs demo + 步骤 72(ci/uc05_rhi_smoke.py:host 恒跑 corpus/审计/--emit=check + device 段 EXE red-green)+ 步骤 73(ci/uc05_invariant_gate.py:I1~I8 逐条 + 三方一致)+ evidence/uc05_invariant_matrix.json(schema 字段全 string/null 硬拦 I9/I10 数值)+ comparison_report.md。零新 RX 码维持;trace_matrix 全锚定;stable 快照重 bless(spec_clauses 251→261,RXS-0180 L2 加性)。budget `ei1.counter.uc05_invariant_cases`(≥8)。 | **Full RFC**(RFC-0014 / §4.B / PR-B2) |
| v1.0 | 2026-07-19 | 新建 spec/rhi.md(EI1.3,PR-B1 条款先行):带编号条款体 `### RXS-0256 ~ ### RXS-0265`(FLS 体例,按需分 Syntax / Legality / Dynamic Semantics / Implementation Requirements,**严禁 UB 节**;镜像 spec/render_graph.md 体例)——RXS-0256 RHI 类型面与 brand(Rhi / Queue / Res / Pass 薄映射 std::gpu lang items,per-instance 新鲜 opaque brand,方法所有权 reads / writes 取 &Res 借用、submit move-out、readback 为 Res move-out 锚,跨 brand → RX3006〔非 RX2001〕,kernel 体内 → RX3015,显式排除 RXS-0189 line 61 单-brand 运行期降级)/ RXS-0257 pass 声明与资源访问集(read / write 封闭枚举,未声明访问 I4 由编译器喂反射集核验、库层状态 Err 镜像 RX6030)/ RXS-0258 graph 构建与依赖推导(RAW / WAW 建序,依赖环 I3 / 写写冲突 I5 纯库层定长数组状态值构建期拒、镜像 RX6029)/ RXS-0259 资源生命周期 affine 拦截(I1 / I2 → RX4001 复用)/ RXS-0260 submit typestate(Graph → Submitted 消费式 1-submit,镜像 RXS-0197,二次 submit → RX4001)/ RXS-0261 执行语义(顺序调度 + 显式 sync + RXS-0193 诊断封口 + device 数值确定 I9)/ RXS-0262 transient 资源(const 泛型定长容量 RD-026,超界编译期拒,I10 峰值观测源)/ RXS-0263 I1~I10 不变量矩阵(裁决 1 划界:I1~I8 编译 / 构建期 100% 拦截入 G-EI1-3 步骤 73、I9 / I10 报告项入 G-EI1-5 步骤 75,documented_historical 无数字定性历史陈述、schema 禁无 in-repo 出处数值)/ RXS-0264 对照报告证据形态(uc05_invariant_matrix.json + schema check_schemas 硬拦 + comparison_report.md 顶部标注、三方一致性机核)/ RXS-0265 采纳判据操作化(C ABI 成熟 G-EI1-4 + check <5s 双口径 cold / warm、warm = 全量重析非 LSP 增量,阈 5000ms)。**Part B 零新 RX 码全复用**(RX4001 / RX4003 / RX3006 / RX3015 / RX2001 / RX2003 / RX2004 + 库层状态值镜像 RX6029 / RX6030 口径,§3 / §5.1);零新借用码。每条 ≥1 `//@ spec` 测试锚定(conformance/uc05/{accept,reject} + apps/uc05-rhi/src/demo.rx + 步骤 72 / 73 / 75 + evidence/uc05_* + schema)随实现 commit 同 PR 落,trace_matrix 全锚定;stable 快照同 PR 重 bless(RXS-0180 L2)。承 [RFC-0014](../rfcs/0014-engine-integration.md)(Agent Approved 2026-07-19,§4.B Part B 参考级设计)。 | **Full RFC**(RFC-0014 / §4.B / PR-B1) |
| v1.3 | 2026-07-20 | EI1.4 引擎嵌入落地(§4 任务 2 / G-EI1-4):§RXS-0261 新增**嵌入面**实现要求一条 —— 整张 RHI 图可完整封闭在一个 `#[export(c)]` host fn 体内经 `--emit=dll` 产 cdylib 供 C/C++ 宿主调用,GPU 上下文/图/资源在**单次调用内**创建与销毁,宿主只见 C 子集 v1 标量与裸指针、不见任何 Rurix 类型;对执行语义**零特例**(seal → 推导 → 派发 → 收尾同步 → D2H 与 in-EXE 路逐段同一,差别仅在结果出口是 `*mut T` 出参),错误面以 **i32 状态码**跨 ABI 返回(§RXS-0255 无 panic 面 by-construction,不展开 unwind)。落地物:`apps/uc05-rhi/src/embed.rx`(全 .rx 导出面,`uc05_run_graph(*mut i32, i32) -> i32` + `uc05_graph_pass_count()`)+ `apps/uc05-rhi/src/graph.rx`(kernel 与闭式参考公式抽为模块 = demo/embed 单一事实源,demo.rx 改为 `mod graph` 复用,语义 0-byte)+ `src/rurix-engine/harness/uc05_engine_host.cpp`(engine_host **v2**:C++/D3D12 驱动方,LUID 匹配 adapter + queue fence 锚点夹住 Rurix 图节点,**新增文件**——G1.3 v1 三符号面/手写头/RXS-0149 守卫逐字节 0-byte,两制共存 spec/export_c.md §RXS-0254)+ 步骤 74 `ci/uc05_engine_embed_smoke.py`(host 恒跑:生成头自始生成不手写审计 + 两制共存审计 + 零 .rs 审计 + `--emit=dll` 三件 + 生成头幂等 + 篡改再生成 byte-diff RED;device:cl.exe 编 v2 harness 链 rurix_rhi.lib 真跑 + **三方数值对照**)+ evidence schema + `ei1.counter.uc05_engine_embed`(≥1,device 见证计数)。**零新 RX 码**;条款语义无收窄或扩张,§RXS-0256~0260/0262~0265 全部 0-byte。 | **Full RFC**(RFC-0014 / §4.A+§4.B / EI1.4) |
| v1.4 | 2026-07-23 | **G4.2 PR-B 图形 RHI 化主面条款先行:落带编号条款体 `### RXS-0270` ~ `### RXS-0273`(spec-first;编号自 RXS-0270 claim 段,0266~0269 burned 跳号,number_ledger v1.13)**。承 RFC-0015(Agent Approved 2026-07-23,G4 伞形章 A;G4_CONTRACT G-G4-3)。**RXS-0270**(RHI 图形 pass 类型面:`g.raster_pass(vs, fs)` / `g.mesh_pass(ms, fs)` → `GfxPass<C>` 句柄族;着色函数引用合法性〔vertex/fragment 阶段 + mesh 须 RXS-0243 入口契约〕;task 前置条件臂首期不开放;**RT pass 条件臂**——执行臂不可达则不立类型面登记 RD-036+,G-EA1-3/RXS-0249 先例;kernel 体内声明 → RX3015 I8 扩展)。**RXS-0271**(RHI 图形资源面:`color_target`/`depth_target`/`texture2d`/`sampler`/`texture_table` 五构造已知方法,封闭格式集 RGBA8/D32F;SamplerDesc 复用 RXS-0225、TextureTable 复用 RXS-0235;cabi 资源类枚举追加式 0~5)。**RXS-0272**(图形 pass 访问声明集与自动 barrier:封闭枚举镜像 RXS-0236 **同一 graph.rs::AccessKind 单源**;**推导单源 = G3.5 graph.rs `derive_barriers`**——rhi.rs 同 crate 构造 Graph/PassSpec 无 cabi marshalling,PlannedBarrier 逐字回放禁二次推导;compute pass reads/writes → ShaderRead/UavReadWrite BufferSync 映射钉死〔RFC-0015 §4.0-1 R-F5〕;含图形 pass 的图仅 Vulkan 后端 strict 无回退,compute-only 图 CUDA 既有路 0-byte;RXS-0239 pass 边界 happens-before 既有承诺,重排归 RXS-0281/0282;`present(&back)` 终端 handoff 唯一且末位 + headless readback 校验 RXS-0222 纪律,窗口腿 D-130 0-byte 归 G4.6)。**RXS-0273**(图形 pass 声明↔反射相等:**反射集 = 逐阶段函数签名资源形参并集**〔按资源身份合并〕;sampler/table 计入并集但标「无状态访问」类——barrier 相等域只核资源状态访问,sampler/table 另核绑定完备性;双向精确相等装配期拒,库层状态值零新码,与 RXS-0257 I4 同口径)。FLS 分节 **严禁 UB 节**;**零新 RX 码、零新借用码**(§3 引用汇总维持)。每条 ≥1 `//@ spec` 测试锚定(conformance/uc05/{accept,reject} gfx_* 语料 + rurixc corpus 单测 + 推导 golden + 步骤 76/77)随实现 commit 同 PR 落;stable 快照因条款增长同 PR 重 bless(RXS-0180 L2)。档位 **Full RFC**(RFC-0015) | **Full RFC**（RFC-0015） |
| v1.5 | 2026-07-23 | **G4.2 PR-C 库化补齐条款先行:落带编号条款体 `### RXS-0274` / `### RXS-0276`(spec-first)**。承 RFC-0015(§4.A4/A6)。**RXS-0274**(present 面库化:终端 handoff 唯一且末位 + present 执行 barrier 语义〔COLOR_ATTACHMENT → PRESENT_SRC,RXS-0238 映射表既有锚〕+ headless readback 三断言点判据〔RXS-0222 纪律〕+ 窗口腿 = RXS-0197/0198 typestate 复用 0-byte〔D-130 不动,窗口 device 见证归 G4.6〕)。**RXS-0276**(RHI bindless 面:`texture_table()` + `register(&tex)` 单调索引 + `reads_table(&table)` pass 绑定;标「无状态访问」类另核绑定完备性;着色侧 RXS-0231/0232 0-byte;feature chain 缺失 → 确定性 Err;descriptor-indexing 运行时面复用,table 整体按 ShaderRead 保守迁移;像素判据 = 四象限动态索引四色 + 篡改注册序换位 RED)。FLS 分节 **严禁 UB 节**;零新 RX 码零新借用码。每条 ≥1 `//@ spec` 锚定随实现 commit 同 PR 落。档位 **Full RFC**(RFC-0015) | **Full RFC**（RFC-0015） |
| v1.6 | 2026-07-24 | **G4.2 PR-D engine_host v3 嵌入条款先行:落带编号条款体 `### RXS-0277`(spec-first;编号续 RXS-0276,PR-C 已落 0274/0276)**。承 RFC-0015(§4.A7)。**RXS-0277**(engine_host v3 嵌入面 + 三方数值精确相等判据:整张图形 RHI 图封闭在 `#[export(c)]` host fn `uc05_gfx_run_frame(out: *mut u32, w: i32, h: i32) -> i32` 体内经 `--emit=dll` 产 cdylib〔EI1.4 同构,subset v1 标量+裸指针,无 upcall 无外部固定 ABI〕;宿主 `src/rurix-engine/harness/engine_host_v3.cpp`(C++/D3D12,**新增文件**,v1/v2 既有资产逐字节 0-byte)链接 `rurix_rhi.lib` device 真跑,LUID 匹配升级为 Vulkan↔D3D12〔v2 = CUDA↔D3D12〕;**三方数值精确相等判据 Q-PixelCriterion** = .rx RHI Vulkan readback ↔ D3D12 raster/mesh pipeline readback ↔ host 闭式参考,**不设 ULP 容差**,相等域 = 纯色/nearest RGBA8 整数 fetch 域〔无过滤/混合/depth/多采样〕,超域换用例不降判据;生成头 CI 再生成逐字节守卫〔RXS-0254 同面,仓库零 tracked .h〕;步骤 78 `ci/uc05_engine_embed_v3_smoke.py` host 恒跑 + device gate real〔cl.exe 编 v3 + 三方像素逐字节相等 + RED 三路〕,RURIX_REQUIRE_REAL=1 翻硬红,缺 provisioning SKIP=dev-env degrade)。FLS 分节 **严禁 UB 节**;零新 RX 码零新借用码零新 lang item。每条 ≥1 `//@ spec` 锚定随实现 commit 同 PR 落。档位 **Full RFC**(RFC-0015) | **Full RFC**(RFC-0015) |
| v1.7 | 2026-07-24 | **G4.3 PR-E RD-035 执行面三项条款先行:落带编号条款体 `### RXS-0280` ~ `### RXS-0283`(spec-first;编号续 RXS-0277,0278~0279 burned 跳号)**。承 RFC-0014(§4.B8~B11)。**RXS-0280**(transient 别名复用分配器 + 执行期峰值计数器:区间图着色〔纯 host safe `#![forbid(unsafe_code)]`〕,生命期区间 = [首写 pass 序位, 末读 pass 序位],区间不重叠者共享同一设备分配,尺寸/对齐三分量着色;执行期峰值计数器 cabi 真实设备分配驱动;I10 自 report_only 升 measured_local)。**RXS-0281**(重排执行模型:sealed 图建依赖 DAG〔RAW/WAW/WAR 边〕→ 拓扑分层,同层独立 pass 可换序/批级提交,层间屏障;多 queue out-of-scope)。**RXS-0282**(I11 拦截项 + RXS-0239/0261 追加式修订行:调度器与核验器两独立纯函数互不导入〔D6 互证先例〕,核验器自 sealed 图独立重建依赖闭包逐边核,red_self_test 双向,demo 图手算 golden,I11 入不变量矩阵;RXS-0239 追加「重排执行模型」段〔严禁改写既有承诺字面〕,RXS-0261 顺序调度 → 依赖保持下的重排/批级调度)。**RXS-0283**(const 容量接线 + RXS-0262 收窄段更新:`rhi.graph::<CAP>()` lang-item 已知方法调用点 turbofish const 实参〔字面量即时求值 → 普通 i64 cabi 实参,CAP 不进类型参数表,无 RD-007 依赖〕,编译期越界拒〔typeck 单函数体 affine 单定义链前向扫描〕,循环/条件/跨函数 strict 拒 non-static construction;RXS-0262 收窄段更新〔Vec 承载 → const 容量接线兑现〕)。**追加式修订**(既有条款字面不动):RXS-0261 追加 PR-E 修订行,RXS-0262 追加 PR-E 修订行,spec/render_graph.md RXS-0239 追加「重排执行模型」段。FLS 分节 **严禁 UB 节**;零新 RX 码零新借用码零新 lang item(`graph` 为 Rhi 已知方法扩面,镜像 RXS-0190)。每条 ≥1 `//@ spec` 锚定随实现 commit 同 PR 落。档位 **Full RFC**(RFC-0014 / §4.B / PR-E) | **Full RFC**(RFC-0014 / §4.B / PR-E) |
