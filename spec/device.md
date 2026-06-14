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

> **M4.2 范围裁决(NVPTX codegen 与 ptxas 关卡续写)**:RXS-0070~0073 条款化 device codegen 链路(MIR→LLVM IR(NVPTX 约束子集)→PTX 文本)与 ptxas 干验证关卡(07 §7,D-205/D-207)。本批为已选定决策(D-120/D-121/D-123/D-205/D-207)的初版条款化(档位 Direct);LLVM pin 22.1.x、目标基线 `compute_89`/`sm_89` 为 r2 第一阶段范围,升级走季度评估(07 §7),不在 M4 期变动。codegen 作用面为 **SAXPY 雏形子集**(全局线程索引 + `View<global>`/`ViewMut<global>` 索引读写 + f32 算术 + 边界分支);`shared`/barrier、views 不相交、scoped atomics、libdevice 链接随 M4.3/M5(07 §4)。错误码 `RX6003`~`RX6005` 为 6xxx codegen/目标段位 device 首批(现有 RX6001/RX6002 为 M2.3 host 子集),**spec 先行引用,正式分配于 M4.2 实现 WP**(沿用 3xxx/4xxx/5xxx 在实现 PR 落 registry 的节奏,registry revision_log 留痕,编号不复用)。

### RXS-0070 NVPTX codegen 目标与调用约定

**Legality**(device codegen 目标与 `kernel fn` 调用约定,07 §7 / D-205·D-207):

- device codegen 目标三元组 `nvptx64-nvidia-cuda`,数据布局取 NVPTX 后端默认(64 位指针);目标基线 `compute_89`(PTX 虚拟 ISA)/ `sm_89`(ptxas 真实架构),LLVM pin 22.1.x(r2 第一阶段范围)。
- `kernel fn` 着色函数 codegen 为 **`ptx_kernel` 调用约定**的 LLVM 函数(`@func ... #N` 处 `define ptx_kernel void @...`),作为 GPU 入口经 launch API 发起(M4.3),无返回值(`void`);`device fn` codegen 为普通调用约定的内部函数(MVP 默认强制内联,06 §2.2)。
- device codegen 产物为 **PTX 文本**(开发期,PTX-only),经 pin 的 LLVM 工具链(`clang --target=nvptx64-nvidia-cuda -mcpu=sm_89 -S`,文本 IR 通道延续 M2.3 host 选型 D-209)由 NVPTX 后端汇编为 PTX;不在 device codegen 期产 cubin(cubin/fatbin 分发随 G1)。
- device codegen 作用面外的语言构造(host 子集 codegen 同款的 closure / 任意数组索引 / 区间 / fn 指针间接调用等,及 device 不支持构造)→ `RX6003`(device codegen 暂不支持构造)。

**Implementation Requirements**:device codegen 消费着色定型后的 device MIR(`kernel fn` 为收集根,沿 device 调用图收集 `device fn`,不依赖 host `main` 可达性);MIR `Body` 关联函数着色(`FnColor`)供 codegen 分叉 host/device 通道;host codegen 通道(07 §8,target `x86_64-pc-windows-msvc`)不受影响。

> 锚定测试:`tests/ptx/`(小 kernel 全管线产 PTX golden);device codegen 单测(`ptx_kernel` 调用约定 / target triple);`rurixc <kernel>.rx --emit=ptx`。

### RXS-0071 地址空间 codegen 建模

**Legality**(addrspace 在 LLVM IR / PTX 的显式建模,05 §5 / r2 NVPTX 五空间,与 RXS-0067 类型层映射对齐):

- 五地址空间在 LLVM IR 指针类型携带 addrspace 数:`global`→`addrspace(1)`、`shared`→`addrspace(3)`、`constant`→`addrspace(4)`、`local`→`addrspace(5)`、泛型/默认→`addrspace(0)`(05 §5 / r2)。
- `View<space, T, Shape>`(只读)/ `ViewMut<space, T, Shape>`(可变)作为 `kernel fn` / `device fn` 形参时,ABI 表示为对应 addrspace 的指针(`ptr addrspace(N)`);索引 `v[i]` codegen 为 `getelementptr` 偏移 + `load`(`View`)/ `store`(`ViewMut`),元素类型取 `T`。
- `ViewMut<space, T>` 的索引位置为可写 place;`View<space, T>`(只读)索引位置仅可读(写入按既有类型/可变性检查段裁决,RXS-0067)。
- 索引下标按 NVPTX 后端 64 位指针算术展开;越界为 device 侧 UB(MVP 不插桩边界检查,边界守卫由 kernel 作者经 `if i < n` 显式书写,06 §2.2)。

**Implementation Requirements**:addrspace 映射表与 RXS-0067 的 `View` 族类型层裁决同源(同一地址空间标记);M4.2 codegen 作用面为 `global`(addrspace 1)读写 + `constant`(addrspace 4)只读;`shared`(addrspace 3)经 `shared let` 的 view 收窄随 M5(07 §4)。

> 锚定测试:`tests/ptx/`(`View<global>` 索引读 / `ViewMut<global>` 索引写产 PTX `ld.global`/`st.global`);device codegen 单测(addrspace 指针形态)。

### RXS-0072 线程索引与 launch bounds

**Legality**(线程索引 intrinsics 与 launch bounds 属性,06 §2.2 / r2):

- `ThreadCtx<DIM>`(`DIM` ∈ {1,2,3})为 `kernel fn` 的线程上下文形参(零尺寸句柄,不占 ABI 槽位);其索引方法 codegen 为 NVPTX special-register intrinsics:
  - `thread_index()`(block 内线程索引)→ `llvm.nvvm.read.ptx.sreg.tid.{x,y,z}`;
  - `global_id()`(全局线程索引)→ `ctaid.{x} * ntid.{x} + tid.{x}`(`llvm.nvvm.read.ptx.sreg.{ctaid,ntid,tid}.{x,y,z}` 组合),返回 `usize`(NVPTX 64 位)。
- DIM=1 取 `.x` 维;DIM=2/3 维索引随 M4.3(launch 维度契约定型);M4.2 codegen 作用面为 DIM=1。
- **launch bounds**:`kernel fn` 的 block 维上界经 `nvvm.annotations` 元数据落地(`!{ptr @kernel, !"maxntidx", i32 N}` / `reqntidx`);M4.2 无显式 launch bounds 标注语法时,annotation 为可选(缺省由 ptxas 默认推导),标注语法与 `reqntid` 强约束随 M4.3。
- `block.sync()`(barrier,RXS-0068 着色层已设保守 uniform 骨架)codegen 为 `llvm.nvvm.barrier0`(`bar.sync 0`);M4.2 codegen 留扩展点(SAXPY 雏形不触发),完整随 M4.3/M5。

**Implementation Requirements**:线程索引方法为编译器已知 device intrinsic(resolve/typeck 兜底识别 `ThreadCtx` 类型与其方法,用户同名定义优先遮蔽,沿用 RXS-0067 `View` 族兜底纪律);intrinsic 声明在 device IR 模块头 `declare`。

> 锚定测试:`tests/ptx/`(`global_id()` 产 sreg intrinsic 序列);device codegen 单测(sreg intrinsic / `ptx_kernel` 入口)。

### RXS-0073 ptxas 干验证关卡与诊断要求(6xxx codegen/目标)

**Legality**(ptxas 干验证关卡,07 §7 strict-only,M4 契约 G-M4-4):

- device codegen 产出的 PTX **必须过 `ptxas -arch=sm_89` 干验证**(语法/语义校验,不产 cubin);ptxas 拒绝(退出非零)→ rurixc 报 `RX6004`(ptxas 拒绝 PTX)编译期诊断,携带 ptxas stderr 摘要,**对齐真跑铁律**(注入非法 PTX / 破坏 codegen 产出必须红)。
- 防御非 ASCII 路径:ptxas 对非 ASCII 路径有崩溃先例(r6 教训),驱动调用 ptxas 前对工作路径作 ASCII 校验/规避(临时 ASCII 路径或拒绝并诊断)。
- ptxas 工具定位经运行时探测(`CUDA_PATH` / NVML 枚举),**禁硬编码版本文件名**(r6 的 `CUDA 13.2.props` 教训,07 §10);ptxas 缺失(无 CUDA 工具链)→ 关卡 SKIP(开发环境降级),真实红绿在带 CUDA 的 CI runner(M4 CI_GATES §1 / 步骤 17),工具链定位失败归 `RX7001`(链接/工具链段,既有码)。
- **NVPTX 雷区回归集**:NVPTX 后端已知雷区(shfl 选择失败 / sqrt 近似约束类,r2/07 §7)遇雷登记雷区回归集并 pin 绕行;SAXPY 雏形不触发,机制就位备 M4.3+ 扩展。

**诊断要求**(6xxx codegen/目标段位 device 首批):

- **device codegen 不支持构造** `RX6003`:device codegen 作用面外的语言构造(RXS-0070)。
- **ptxas 拒绝 PTX** `RX6004`:产出 PTX 过 `ptxas -arch=sm_89` 干验证被拒(携 ptxas stderr 摘要)。
- **device codegen 内部约束违例** `RX6005`:device MIR 形态超出 NVPTX codegen 约束子集(如不支持的 addrspace 组合 / 非 DIM=1 线程索引 / 不支持元素类型),保守拒绝(措辞允许粗糙,07 §4)。

**Implementation Requirements**:ptxas 干验证在 device codegen 产 PTX 后、嵌入 host 产物前(M4.3)实施;诊断 span 指向 kernel 定义 / 违例构件,措辞允许保守粗糙(07 §4 先正确性后诊断,M4 契约 §2.2 诊断打磨排除项)。

> 锚定测试:`tests/ui/codegen/`(RX6003/RX6004/RX6005 snapshot,黄金路径 4 的 6xxx 子集);device codegen 单测(ptxas 拒绝路径,缺 ptxas 时 SKIP)。

---

> **M4.3 范围裁决(launch 类型契约续写)**:RXS-0074~0075 条款化 host 侧 launch 类型契约(05 §6 / 06 §1 / 08 §2,已锁定决策 D-107/D-120 的条款化,档位 Direct)。本批作用面 = **编译期可检的 launch 类型契约**(着色/维度/参数/context-brand 四类反例,07 §3 HIR 层无数据流);运行时对象(Context/Stream/Buffer/launch 的 Driver API 实现)、装载协商(PTX `.version` 比对 → RX7xxx)、poisoned context 状态机随 `rurix-rt` 运行时实现 PR 落地(规范先行:与其实现同 PR,06 §5 / 08 §2.4/§2.5)。错误码 `RX3004`~`RX3006` 为 3xxx 着色/地址空间段位续接(launch 着色/维度/brand;arg 类型不符复用 `RX2001`,View 空间不符复用 `RX3002`),**spec 先行引用,正式分配于 M4.3 实现 WP**(沿用既有节奏,registry revision_log 留痕,编号不复用)。

### RXS-0074 launch 类型契约

**Syntax**(launch 调用形态,05 §6 / D-120 草图的可检收窄):

```
LaunchCall ::= StreamExpr "." "launch" "(" KernelRef "," GridDim "," BlockDim "," ArgTuple ")"
KernelRef  ::= PathExpr           // 解析到 `kernel fn` 的值引用
GridDim    ::= "GridDim" "(" Expr ("," Expr)* ")"
BlockDim   ::= "BlockDim" "(" Expr ("," Expr)* ")"
ArgTuple   ::= "(" (Expr ("," Expr)*)? ")"
```

`Stream`、`GridDim`、`BlockDim`、`Context`、`Module` 为编译器已知 host 运行时类型(resolve 兜底识别,用户同名定义优先遮蔽,沿用 RXS-0067 `View` 族兜底纪律);`GridDim`/`BlockDim` 的可变维数构造容忍(维数 = 实参个数)。`Stream<Ctx>`/`Buffer<Ctx, T>` 的首类型实参 `Ctx` 为 **context-brand**(资源归属的类型层编码,05 §4 D-107 affine 资源 brand 的 M4.3 可检形态;完整 affine 生命周期借用证明随 M5)。

**Legality**(launch 类型契约,编译期 HIR 层裁决,无数据流):

- **着色契约**:`launch` 的 `KernelRef` 必须解析到 **`kernel` 着色函数**;对 `host`/`device`/`const` 着色函数或非函数项发起 launch 非法 → `RX3004`(kernel fn 不可直接调用经 RXS-0066/RX3001 拦截,经 launch API 误用非 kernel 经本条 RX3004 拦截)。
- **维度契约**:`GridDim` 与 `BlockDim` 的维数(实参个数)必须一致(共同定义 launch 网格维度 N ∈ {1,2,3},须与 `kernel fn` 的 `ThreadCtx<DIM>` 一致);不一致 → `RX3005`。M4.3 保守检查 = grid/block 维数相等(`ThreadCtx<DIM>` 的 const 维度跨核对随 const 泛型在 HIR 留存的扩展,RD-007 系)。
- **参数契约**:`ArgTuple` 各元素按位置与 `kernel fn` 形参(剔除 `ThreadCtx` 句柄形参)对应。host 侧 `Buffer<Ctx, T>` 实参满足 device 侧 `View<space, T>`/`ViewMut<space, T>` 形参当且仅当元素类型 `T` 可合一(Buffer 提供 view,空间由形参声明);标量实参与标量形参类型须合一。类型不符 → `RX2001`(复用类型不匹配段,View 空间不符复用 `RX3002`,RXS-0067)。
- **context-brand 契约**:`ArgTuple` 中携带 brand 的资源实参(`Buffer<Ctx, T>`)的 brand `Ctx` 必须与发起 launch 的 `Stream<Ctx>` 的 brand 一致;不一致(跨 context 资源误用)→ `RX3006`。

**Implementation Requirements**:launch 类型契约检查在 HIR 层、typeck 之后、MIR 前实施(与着色检查同层,07 §3,无需数据流);输入 = 各 body 的 typeck 结果(实参定型 + 接收者类型)与 `kernel fn` 签名;诊断 span 指向违例构件(kernel 引用 / grid/block 维度 / 不符实参);措辞允许保守粗糙(07 §4 先正确性后诊断)。`Stream`/`Buffer` brand 的完整 affine 生命周期证明(跨 context 资源逃逸)随 M5 device 借用扩展;本批为类型层 brand 一致性(名义合一)。

> 锚定测试:`conformance/launch/accept/*.rx`(正例 0 诊断)、`conformance/launch/reject/<category>/*.rx`(四类反例全拦截);`tests/ui/launch/`(黄金路径 4 launch 子集 snapshot);launch_check 单测。

### RXS-0075 launch 诊断要求(3xxx 续接 + 复用)

**Legality**(launch 类型契约诊断码,3xxx 着色/地址空间段位续接 + 既有段复用):

- **launch 非 kernel 着色** `RX3004`:对非 `kernel` 着色函数发起 launch(RXS-0074 着色契约)。
- **launch 维度不匹配** `RX3005`:`GridDim`/`BlockDim` 维数不一致(RXS-0074 维度契约)。
- **launch context-brand 不匹配** `RX3006`:资源实参 brand 与 launch 所在 `Stream` brand 不一致(RXS-0074 context-brand 契约)。
- **launch 参数类型不符** `RX2001`(复用):`ArgTuple` 实参与 `kernel fn` 形参元素类型不可合一(RXS-0074 参数契约;View 地址空间不符复用 `RX3002`,RXS-0067)。

**Implementation Requirements**:

- 检查时点:launch 类型契约四类均在 HIR / typeck 层之后实施(typeck 后、MIR 前),不依赖数据流(07 §3)。
- 诊断 span 指向违例构件(kernel 引用表达式 / grid/block 维度构造 / 不符的实参元组元素);措辞允许保守粗糙(07 §4,M4 契约 §2.2 诊断打磨排除项)。
- **Err 容忍不级联**:launch 形态不完整(kernel 引用不可判定 / 接收者非 `Stream` / 参与类型为容忍区 `Err`)时不触发(防一错多报,与 RXS-0047/0069 同口径);每个 launch 调用按优先序(着色 → 维度 → 参数/brand)报首个违例。

> 锚定测试:`tests/ui/launch/`(黄金路径 4 的 launch 子集 snapshot:RX3004/RX3005/RX3006/RX2001);launch_check 单测(失败路径)。

---

> **M4.3 运行时裁决(rurix-rt 装载协商与 poisoned 状态机)**:RXS-0076~0077 条款化 host 运行时**动态语义**(06 §5 / 08 §2.4/§2.5,已锁定决策 D-230/D-234 的条款化,档位 Direct)。本批为运行时行为(`rurix-rt` 实现:CUDA Driver API 薄层),`nvcuda.dll` 运行时动态加载(无 CUDA Toolkit 依赖,PTX-only,14 §2);装载协商失败与 poisoned 为**运行时结构化错误**(`CudaError`,`Result` 返回),保留原始 `CUresult`,**不占编译期 RX#### 段位**(registry = 编译诊断,07 §5)。GPU 真跑子进程隔离(14 §6),无 GPU 环境降级 SKIP(真红绿在带 GPU runner)。

### RXS-0076 PTX 装载协商(运行时)

**Dynamic Semantics**(模块装载与版本协商,08 §2.4 / D-234):

- 运行时装载嵌入/给定的 PTX 前解析其 `.version`(协商起点)与 `.target`;经 `cuModuleLoadDataEx` **驱动内 JIT** 装载(MVP 链路,r2;JIT info/error 日志缓冲常开)。
- 驱动 JIT 返回 `CUDA_ERROR_UNSUPPORTED_PTX_VERSION`(PTX ISA 超出驱动能力)时,按降版阶梯(高→低,如 `8.0 → 7.8 → 7.0`)改写 `.version` 重试;阶梯耗尽 → 结构化 `LoadNegotiation` 错误,携尝试过的版本序列 + 末次 JIT error log + **可执行指引**(升级 NVIDIA 驱动 / 以更低 `--ptx-floor` 重编)。
- **明确边界**:Windows 不支持 CUDA Minor Version Compatibility(r6),协商逻辑不照搬 Linux 假设。
- 装载协商失败为运行时结构化诊断(`rurix-rt` 的 `CudaError::LoadNegotiation`),非编译期 `RX####`(registry 段位仅编译诊断);原始 `CUresult` 经错误保留。

**Implementation Requirements**:`rurix-rt` 装载入口(`Context::load_module`)实现协商序列;PTX `.version` 解析/改写零外部依赖;`nvcuda.dll` 经 `LoadLibraryA`/`GetProcAddress` 运行时加载(不链接 Toolkit `nvcuda.lib`)。

> 锚定测试:`src/rurix-rt`(`.version` 解析/改写单测 + SAXPY 全链路真跑装载协商,子进程隔离;无 GPU SKIP)。

### RXS-0077 poisoned context 状态机(运行时)

**Dynamic Semantics**(错误模型与 poisoned,08 §2.5 / 06 §5 / D-230):

- 全部 Driver API `CUresult` 映射为结构化 `CudaError`(非穷尽枚举 + 原始码保留;异步错误现实:检测点 ≠ 起因点,r4)。
- `CUDA_ERROR_ASSERT`(device 侧断言)/ `CONTEXT_IS_DESTROYED` → context 进入 **poisoned** 状态;poisoned 后该 context 上的**全部后续操作返回确定性 `Err(Poisoned)`**(携触发函数 + 原始码),而非 UB 级联——重建路径由 affine 类型引导(06 §5,"整块重建"语义类型化)。
- poisoned 为运行时确定性错误(`CudaError::Poisoned`),非编译期 `RX####`。

**Implementation Requirements**:`rurix-rt` `Context` 维持 poisoned 状态机;每次 Driver API 调用后对 poisoning 码置位,后续 API 入口先查 poisoned 守卫返回确定性错误;GPU 测试子进程隔离(14 §6,崩溃不连坐 harness)。

> 锚定测试:`src/rurix-rt`(poisoning 码分类单测 + Context poisoned 守卫;SAXPY 全链路真跑覆盖正常路径)。

---

> **M5.1 范围裁决(views 不相交证明,MIR 借用检查 device 扩展)**:RXS-0078 条款化 views 算子集(`split_at`/`chunks`/`windows`)语义与子 view 不相交证明,作为 M3 host 借用检查(MIR 层)的 **device 扩展 pass**(07 §4),消费 M4 着色(RXS-0066)/ 地址空间(RXS-0067)边界信息(05 §3.2 设备层 views D-106)。本批为已锁定决策(D-106 / 07 §4 保守先行)的条款化(档位 Direct);判定取**保守上界**——能证不相交才放行,证不出保守拒绝(可经 `unsafe` 逃生),避免假阴性(漏报竞争)。错误码 `RX3007`~`RX3008` 为 3xxx 着色/地址空间段位续接(接 M4.3 的 `RX3006`),**spec 先行引用,正式分配于 M5.1 实现 WP**(沿用 3xxx/4xxx/5xxx/6xxx 在实现 PR 落 registry 的节奏,registry revision_log 留痕,编号不复用)。shared+barrier 一致性数据流 / scoped atomics / 完整 uniform 分析随 M5.2+(07 §4),本批不覆盖。

### RXS-0078 views 算子集语义与子 view 不相交证明

**Syntax**(views 算子集,05 §3.2 设备层 views D-106):`View<space, T, Shape>`(只读)/ `ViewMut<space, T, Shape>`(可变)提供子 view 划分算子(device view 形态,语义对齐 Rust slice 的 `split_at`/`split_at_mut`、`chunks`/`chunks_mut`、`windows`):

```
view.split_at(mid)   // -> (sub[0..mid], sub[mid..len));ViewMut 产两个可变子 view
view.chunks(n)       // -> 块大小 n 的不重叠子 view 序列(尾块容许 < n)
view.windows(n)      // -> 大小 n、步长 1 的滑动窗口序列(相邻窗口重叠)
```

**Legality**(子 view 不相交证明,MIR 借用检查 device 扩展层裁决,07 §4 保守先行):

- **不相交并存规则**:同时持有的多个**可变**子 view 必须两两**不相交**(disjoint),其可变借用方可并存(对齐 host 借用检查的可变借用唯一性,RXS-0048~0061 的 device 扩展:把"同一父 view 的不同区间"纳入别名分析)。
- **结构性不相交**:`split_at(mid)` 产出的 `[0, mid)` 与 `[mid, len)` 静态可证不相交 → 放行两个可变子 view 并存;`chunks(n)` 产出的不同块静态可证不相交 → 放行。
- **重叠拒绝**:`windows(n)` 相邻窗口重叠(步长 1),对其同时发起可变借用 → 拒绝(`RX3007`);别名可变 view 之间的写冲突同理(消费 M4 着色/地址空间边界信息判定别名)。
- **保守上界(07 §4)**:能证不相交才放行;证不出(下标非常量、复杂区间/别名)保守拒绝(误拒边界情形,措辞容许粗糙,可经 `unsafe` 逃生)。MVP 判定取可静态判定的区间关系(常量端点 / split·chunks 的结构性不相交),不引入完整区间/别名求解器(随真实 kernel 需求扩展,扩展经 conformance 类别留痕)。
- **越界规则**:子 view 划分点静态可判越界(`split_at` 的 `mid > len` / `chunks(0)` / 窗口大小 > 父 view 长度)→ 拒绝(`RX3008`);动态下标越界为 device 侧 UB,边界守卫由 kernel 作者经 `if i < n` 显式书写(MVP 不插桩,对齐 RXS-0071)。

**诊断要求**(3xxx 着色/地址空间段位续接,接 M4.3 的 `RX3006`):

- **重叠/别名子 view 同时可变借用** `RX3007`:重叠子 view(`windows` 相邻窗口 / 可证相交的区间)被同时可变借用,或别名可变 view 写冲突;**证不出不相交的保守拒绝复用本通道**(措辞标注"无法证明子 view 不相交")。
- **view 划分越界** `RX3008`:静态可判的子 view 划分越界(split 点 / chunk 大小 / 窗口大小 超出父 view 长度)。

**Implementation Requirements**:

- views 不相交证明作为 **MIR 借用检查的 device 扩展 pass**(07 §4),在 host 借用检查之后、着色(RXS-0066)与地址空间(RXS-0067)检查之上运行;pass 结构沿用 M4 已保证的可扩展点(子 view 继承父 view 的 space 与着色)。
- `split_at`/`chunks`/`windows` 为编译器已知 device 方法(resolve/typeck 兜底识别 `View` 族方法,用户同名定义优先遮蔽,沿用 RXS-0067 `View` 族兜底纪律)。
- `views.*` message-key 随 `RX3007`/`RX3008` 在 M5.1 实现 WP 落 registry(只追加)。host 回归网(hello-world 冒烟 + MIR golden + SAXPY 回归)持续绿(本扩展不退化 host 借用检查)。

> 锚定测试:`conformance/views/accept/*.rx`(正例 0 诊断:`split_at`/`chunks` 不相交并存可变借用)、`conformance/views/reject/<category>/*.rx`(反例全拦截:重叠 `windows` 可变 / 别名可变 view 写冲突 / view 划分越界);`tests/ui/views/`(`RX3007`/`RX3008` 黄金路径 5 子集 snapshot);views 不相交 device 借用检查单测。

---

## 错误码引用汇总

| 错误码 | 含义 | 条款 |
|---|---|---|
| RX3001 | 跨着色非法调用(device 上下文调 host-only / 直接调用 kernel fn) | RXS-0066, RXS-0069 |
| RX3002 | 地址空间不匹配(View 族容器空间不一致) | RXS-0067, RXS-0069 |
| RX3003 | barrier 非 uniform 可达(thread-id 依赖分支内调 barrier,保守骨架) | RXS-0068, RXS-0069 |
| RX3004 | launch 非 kernel 着色函数(对 host/device/const 着色函数发起 launch) | RXS-0074, RXS-0075 |
| RX3005 | launch 维度不匹配(GridDim/BlockDim 维数不一致) | RXS-0074, RXS-0075 |
| RX3006 | launch context-brand 不匹配(资源实参与 Stream brand 不一致) | RXS-0074, RXS-0075 |
| RX2001 | 类型不匹配(引用:View 可变性不符与元素类型不符走既有类型检查段;launch 参数元素类型不符复用) | RXS-0067, RXS-0074 |
| RX6003 | device codegen 暂不支持构造(NVPTX codegen 作用面外) | RXS-0070, RXS-0073 |
| RX6004 | ptxas 拒绝 PTX(`ptxas -arch=sm_89` 干验证被拒) | RXS-0073 |
| RX6005 | device codegen 内部约束违例(超出 NVPTX 约束子集) | RXS-0071, RXS-0072, RXS-0073 |
| RX3007 | 重叠/别名子 view 同时可变借用(含证不出不相交的保守拒绝) | RXS-0078 |
| RX3008 | view 划分越界(split 点 / chunk 大小 / 窗口大小 静态可判超界) | RXS-0078 |
| RX7001 | 外部工具链失败(ptxas 定位失败归此段,既有码) | RXS-0073 |

含义以 [../registry/error_codes.json](../registry/error_codes.json) 为唯一事实源,本表仅引用。RX3001 ~ RX3003 为 3xxx 着色/地址空间段位首批(07 §5 段位语义),**spec 先行引用,正式分配于 M4.1 实现 WP**(沿用 4xxx/5xxx 在实现 PR 落 registry 的节奏,registry revision_log 留痕,编号不复用)。RX6003 ~ RX6005 为 6xxx codegen/目标段位 device 首批(现有 RX6001/RX6002 为 M2.3 host 子集),**spec 先行引用,正式分配于 M4.2 实现 WP**(同上节奏)。RX3004 ~ RX3006 为 3xxx 段位续接(launch 着色/维度/brand,RXS-0074/0075),**spec 先行引用,正式分配于 M4.3 实现 WP**(同上节奏;launch 参数类型不符复用 RX2001/RX3002)。RX3007 ~ RX3008 为 3xxx 段位续接(views 不相交,RXS-0078),**spec 先行引用,正式分配于 M5.1 实现 WP**(同上节奏;证不出不相交的保守拒绝复用 RX3007)。shared+barrier 一致性数据流 / scoped atomics / 完整 uniform 分析仍随 M5.2+ device 借用检查扩展(07 §4 / 11 §3),本批不覆盖。

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-13 | 初版:RXS-0066 ~ RXS-0069(M4.1 device 着色/地址空间首批:函数着色与跨着色调用合法性 / 地址空间类型与一致性 / barrier uniform 可达性保守骨架 / 着色与地址空间诊断要求;05 §1/§3.2/§5、06 §2.2、07 §3 已锁定决策 D-102/D-106/D-108/D-120/D-123 的条款化,M4 契约 D-M4-2 spec 先行)。错误码汇总表登记 RX3001~RX3003(spec 先行引用,实现 WP 正式分配);barrier 完整 uniform 分析与 views 不相交证明排除(M5) | Direct |
| v1.1 | 2026-06-13 | 续写 RXS-0070 ~ RXS-0073(M4.2 NVPTX codegen 与 ptxas 关卡:codegen 目标与 `ptx_kernel` 调用约定 / 地址空间 codegen 建模 / 线程索引与 launch bounds / ptxas 干验证关卡与 6xxx 诊断要求;06 §1/§2.2、07 §7 已锁定决策 D-120/D-121/D-123/D-205/D-207 的条款化,M4 契约 D-M4-2/D-M4-3 spec 先行)。错误码汇总表登记 RX6003~RX6005(spec 先行引用,M4.2 实现 WP 正式分配);codegen 作用面 = SAXPY 雏形子集(DIM=1 线程索引 + `global` 索引读写 + f32 算术);shared/barrier 完整 codegen、launch 维度契约、cubin 分发排除(M4.3/M5/G1) | Direct |
| v1.3 | 2026-06-13 | 续写 RXS-0076 ~ RXS-0077(M4.3 运行时:PTX 装载协商 / poisoned context 状态机;06 §5、08 §2.4/§2.5 已锁定决策 D-230/D-234 的条款化,M4 契约 D-M4-4 运行时落地)。运行时动态语义(rurix-rt CUDA Driver API 薄层,nvcuda.dll 运行时动态加载,PTX-only 无 Toolkit 依赖);装载协商失败/poisoned 为运行时结构化 `CudaError`(Result),保留原始 CUresult,不占编译期 RX#### 段位;GPU 真跑子进程隔离,无 GPU SKIP。锚定 src/rurix-rt 单测 + SAXPY 全链路真跑 | Direct |
| v1.2 | 2026-06-13 | 续写 RXS-0074 ~ RXS-0075(M4.3 launch 类型契约:着色/维度/参数/context-brand 四类编译期可检契约 + launch 诊断要求;05 §6/§4、06 §1、08 §2 已锁定决策 D-107/D-120 的条款化,M4 契约 D-M4-2/D-M4-6 spec 先行)。错误码汇总表登记 RX3004~RX3006(spec 先行引用,M4.3 实现 WP 正式分配;launch 参数类型不符复用 RX2001/RX3002)。作用面 = 编译期 launch 类型契约;运行时对象/装载协商/poisoned 状态机随 rurix-rt 实现 PR(规范先行同 PR);ThreadCtx const 维度跨核对、views 不相交、完整 affine brand 借用证明排除(M5/RD-007) | Direct |
| v1.4 | 2026-06-14 | 续写 RXS-0078(M5.1 views 算子集语义与子 view 不相交证明:`split_at`/`chunks`/`windows` 划分语义 + 子 view 不相交并存规则 + 越界规则 + 诊断要求;05 §3.2 设备层 views D-106、07 §4 保守先行已锁定决策的条款化,M5 契约 D-M5-1 / G-M5-2 spec 先行,**条款 PR 先于实现 PR**)。作为 MIR 借用检查 device 扩展 pass,消费 M4 着色/地址空间边界信息;判定取保守上界(能证不相交才放行,证不出保守拒绝,可 unsafe 逃生)。错误码汇总表登记 RX3007~RX3008(3xxx 段位续接 M4.3 RX3006,spec 先行引用,M5.1 实现 WP 正式分配,编号不复用);`views.*` message-key 随实现 WP 落 registry。shared+barrier 一致性数据流 / scoped atomics / 完整 uniform 分析排除(M5.2+) | Direct |
