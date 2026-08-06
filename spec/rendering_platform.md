# rendering_platform.md — 渲染平台语义面(G8.2 起)

> **地位**:渲染平台级语义事实源之一(10 §4,D-403)。首发面 = M31 shader reflection
> schema 与 interface hash(RFC-0019 §4.4)。后续 RP-* diff key(RP-PERMUTATION /
> RP-TEMPORAL / RP-CLOSURE / RP-MULTIQUEUE 等,RFC-0019 §5)按各自实现波次在本文件
> 或既有 spec 文件顺位 materialize。
>
> **新建裁决留痕(G8.2 M31 实现 PR)**:RFC-0019 头部「关联条款」授权「G8.1 不领取
> RXS 数字号……并在 G8.2 实现门后按 §5 决定是否新建 `spec/rendering_platform.md`」;
> RFC-0019 §5 diff 计划表把 **RP-REFLECTION** 的目标 spec 冻结为本文件;评审 F16
> 留痕确认「最终归属由 G8.2 spec PR 按 07 §5 边界裁定」。本 PR 裁定:**新建本文件**,
> 理由是 RP-REFLECTION 的字段闭集 / canonical serialization / interface hash 规则
> 与既有 `shader_stages.md`(类型面)/`vulkan_backend.md`(编码腿)均不同轴,独立成文
> 可循 RXS-0236~0241 render_graph.md 新建先例(spec/README.md v1.65 行)。条款号自
> 合入时 `registry/number_ledger.json` 实测 `RXS.next_free = 304` 顺位领取
> (RXS-0304~0307),编号永不复用(10 §9.5)。

---

## 1. 范围与体例

本文件承载渲染平台**跨阶段、跨后端**的语义契约。首批条款冻结 M31
(`g8.p0.m31.reflection_hash`,G8_ACCEPTANCE_MAP §2)所需的 reflection v1 字段闭集、
canonical serialization、`interface_hash` 定义与装配期核验纪律。

- 体例 = FLS 风格(spec/README.md §2);**本文件严禁 UB 节**——reflection 面所有
  失败均为编译期/装配期确定性诊断或 typed `Err`,不设未定义行为。
- 实现锚定:`src/rurixc/src/reflection.rs`(模型 + canonical 序列化 + hash +
  pipeline key 组装 + 装配期核验)、`src/rurixc/src/iface_extract.rs`(AST 签名面
  无损提取,与 `mir_build` 图形阶段根收集同一提取律)、`--emit=reflection` CLI。
- 每条款 ≥1 `//@ spec: RXS-####` 测试锚定(conformance/reflection/ 语料 +
  rurixc 单测),traceability 矩阵全锚定(10 §4)。

## 2. 术语

- **entry**:着色入口函数——`kernel fn` / `compute fn` / `vertex fn` /
  `fragment fn` / `mesh fn`(RT 阶段见 RXS-0304 的诚实边界)。
- **entry identity**:entry 的**源级名称路径**(嵌套 `mod` 以 `::` 连接,如
  `vs_main`、`pipe::vs_main`)。不含编译单元文件名、不含绝对路径、不含链接符号
  (`rx_{名}_{DefId}` 等 mangle 产物属 codegen 细节,不是接口身份)。
- **canonical bytes**:按 RXS-0305 规则序列化 reflection 文档得到的唯一字节串。
- **interface hash**:RXS-0306 定义的 `SHA-256` 域分离 digest。

### RXS-0304 Reflection v1 schema 与字段闭集

**Legality**

reflection v1 文档(逻辑模型,序列化规则见 RXS-0305)由编译器 `--emit=reflection`
产出,字段为**闭集**——本节列出的字段即全部字段;实现不得增删。每个 entry 一条
entry 记录;entry 记录在文档内按 `(entry identity, stage_tag)` 规范键排序
(stage_tag 映射 = RXS-0290 单一事实源,0=Vertex / 1=Fragment / 2=Compute /
3=Mesh / 4=Task / 5=RayGen / 6=ClosestHit / 7=AnyHit / 8=Miss / 9=Intersection /
10=Callable)。

文档级字段(闭集):

| 字段 | 取值规则 |
|---|---|
| `schema` | 常量字符串 `"rurix.shader-reflection.v1"` |
| `schema_version` | 整数 `1` |
| `compiler` | 常量 `"rurixc"` |
| `compiler_version` | 本次构建的 rurixc 包版本(workspace 版本字串) |
| `edition` | 编译单元 edition,`"Rx0"`(MVP 期唯一 edition) |
| `target` | canonical 反射目标,常量 `"vulkan"`(见下「目标纪律」) |
| `backend` | 常量 `"spirv"` |
| `entries` | entry 记录列表(规范键排序,见上) |

entry 记录字段(闭集;RFC-0019 §4.4「至少含」清单逐字落地):

| 字段 | 取值规则 |
|---|---|
| `name` | entry identity(源级名称路径) |
| `stage` | 阶段名闭集:`"vertex"` / `"fragment"` / `"compute"` / `"mesh"`(`kernel fn` 与 `compute fn` 同归 `"compute"`) |
| `stage_visibility` | 该 entry 可见的阶段位掩码 = 本 entry 阶段对应的单 bit(`1 << stage_tag`);v1 每资源可见性 = 声明它的 entry 的阶段(见 `resources[].visibility`) |
| `io` | stage I/O 元素表,**声明序**(字段序是接口的一部分):每元素含 `name`(源字段名)、`dir`(`"in"`/`"out"`)、`kind`(`"builtin"` / `"interpolate"` / `"varying"`)、`annotation`(builtin 名或插值限定名;`"varying"` 时为空串)、`type`(已建模类型渲染,闭集见下)、`location`(非 builtin 元素按方向各自自 0 递增分配;builtin 元素记 `null`) |
| `resources` | 资源绑定表,**声明序**:每元素含 `name`、`class`(`"cbv"`/`"srv"`/`"uav"`/`"sampler"`/`"accel"`)、`set`、`binding`、`count`(`1` 或有界 `n`;无界 SRV 纹理表记 `0` 哨兵,见下)、`access`(`"read_only"`/`"read_write"`/`"sample"`/`"sample_cmp"`/`"accel"`)、`format`(纹理分量 / buffer 元素 prim 名,如 `"f32"`;`Sampler`/`SamplerCmp`/`AccelStruct` 等无元素类型的资源为空串)、`visibility`(= 本 entry 的 `stage_visibility`) |
| `push_constants` | compute 族入口的标量形参块:无标量形参时 `members` 为空表且 `size_bytes = 0`;否则 `members` 按声明序各含 `name`、`type`、`member`(自 0 递增)、`offset`、`size`(对齐律:64 位整型 `i64/u64` → `(align=8,size=8)`,其余标量 → `(4,4)`,偏移自 0 起按对齐向上取整累计——与 `vulkan_codegen` push-constant 布局同一律);`size_bytes` = 末成员 `offset + size` |
| `execution_modes` | mesh 入口的源衍生执行模式:`numthreads`(三正整数,声明序)、`max_vertices`、`max_primitives`(`#[numthreads]`/`#[outputs]` 标注,RXS-0243);其余阶段恒为空表编码(compute 的 workgroup 维度非源衍生——codegen 恒发 `LocalSize(1,1,1)`,故无接口字段) |
| `rt_payloads` / `rt_hit_attributes` / `rt_callable_data` / `rt_task_payloads` / `shader_records` | **M50 未实现,确定性空编码**:恒为空表。字段位保留,编码规则冻结(空表 = `count 0`),M50 落地时按同一序列化规则填充,不得改本闭集的既有字段语义 |
| `rt_group_membership` / `library_exports` | 同上:恒空表(M50 保留位) |
| `required_capabilities` | **M32 已实现(v1.2 真值化,RXS-0311)**:entry 有效 requirement 集的排序 ID 表(RXS-0311 调用图并集律);无任何 requirement 时恒空表——空集路径 0 字节漂移 |
| `selected_profile_digest` | **M32 已实现(v1.2 真值化,RXS-0312)**:`--profile` 给定时 = 该 profile 的规范 digest(RXS-0312);未给定恒为既有常量 `SHA-256("rurix.profile-none.v1\0")`(hex)——0 字节漂移 |
| `permutation_domain_digest` | **M29 已实现(v1.1 真值化,RXS-0309)**:entry 声明了非空 permutation 域时 = 该域的规范 digest(RXS-0309);无 `#[permutation]` 标注(空域)恒为既有常量 `SHA-256("rurix.permutation-domain-empty.v1\0")`(hex)——空域路径 0 字节漂移 |
| `variant_key` | **M29 已实现(v1.1 真值化,RXS-0310)**:`--permutation-select=KEY` 选中合法组合时 = 该组合的字符串形态 key(RXS-0309);未选择或空域恒空串——0 字节漂移 |
| `interface_hash` | RXS-0306 定义(不含 entry 函数体的接口 digest) |
| `source_digest` | RXS-0306 定义(含函数体的内容 digest) |
| `pipeline_key` | RXS-0306 定义的下游 key 组装见证(DDC/PSO/RT pipeline key 组成项) |

类型渲染闭集(`io[].type`、`resources[].format`、push-constant `type`):标量 prim
名(`i8/i16/i32/i64/u8/u16/u32/u64/usize/f32/f64/bool/char/str`)与向量
`vecN<T>`(N∈{2,3,4},T 为 prim 名)。资源句柄类型渲染闭集:`Texture2D` /
`TextureRw2D` / `Sampler` / `SamplerCmp` / `View` / `ViewMut` / `Atomic` /
`AtomicView` / `AccelStruct` / 无界表 `[Texture2D]`。

绑定推导律(接口的确定性来源,与生产 codegen **同一事实源**):

- compute 族(`kernel fn` / `compute fn`,**含 mesh**——`lower_mesh` 镜像
  `lower_compute` 的形参分类,RXS-0275):资源形参(`View`/`ViewMut`→ buffer,
  `Atomic`/`AtomicView` → buffer,`TextureRw2D` → storage image,`AccelStruct` →
  accel,仅 compute 签名)`set = 0`,`binding` 按资源形参声明序自 0 起**全局**
  递增(标量形参与 `ThreadCtx` 不占 binding——`ThreadCtx` 是执行上下文形参,
  不进任何 ABI 字段);标量形参按声明序进 push-constant 块(布局见
  `push_constants`)。
- vertex / fragment:`set`/`binding` = `binding_layout::
  infer_spirv_bindings_vk_native` 的输出(Vk-native set-per-class 分配:
  `set = 类别轴`(0=CBV/1=SRV/2=UAV/3=Sampler),`binding` = 类内声明序递增;
  无界 SRV 纹理表独占 `set ≥ 4` 按声明序递增、表内 `binding = 0`)。
- 无界**非** SRV 纹理表(如无界 `Sampler` 数组)不可映射:反射推导失败 =
  编译期确定性诊断(复用既有类别 RX6013 `codegen.dxil_unmappable`,与 Vk-native
  图形编码路同一裁决),**不产**部分 reflection 产物(fail-closed)。
- compute 形参超出已建模闭集(非标量、非资源句柄、非 `ThreadCtx`/`AccelStruct`)
  → 编译期确定性诊断(复用 RX6026 `codegen.vulkan_unsupported`,与 canonical
  target 的 compute 降级模型同一口径)。

**目标纪律**:v1 的 canonical 反射目标恒为 `"vulkan"`(Vk-native 分配律,RXS-0230
E-3;生产运行时通道)。DXIL/B 链形态(set0 装饰)不是 v1 的反射目标;多目标反射
属 schema 演进,须升 `schema_version`。

**诚实边界(冻结)**:① RT 阶段函数(`raygen/closesthit/anyhit/miss/intersection/
callable fn`)在 v1 **不可枚举**(编译器尚无其产物收集路,RXS-0275/RD-034 口径);
其 schema 字段位已按上空编码冻结,枚举接线归 M50 实现 PR。② 泛型着色函数不产
entry 记录(与 `mir_build` device 根收集口径一致:泛型根不收集)。③ 资源/阶段
判定以 AST 类型头名匹配为准(承 RXS-0156/RXS-0245 `is_accel_struct` 先例,与
`mir_build` 提取层同一函数族);用户类型遮蔽 lang item 名属前端既有行为面,不为
reflection 单开裁决;同名 entry 跨 `mod` 重复出现时,反射推导**确定性失败**
(fail-closed,不猜测归属;v1 语料不覆盖该形态)。④ `compiler_version` 参与文档
但与路径/时间戳无关;canonical bytes 的禁用面见 RXS-0305。

**Implementation Requirements**

- 同一编译单元两次编译的 reflection 产物必须逐字节相等(确定性);任何实现侧
  迭代序(`HashMap` 遍历等)不得泄漏进产物。
- reflection 产物不得包含绝对路径、文件名、mtime、进程 ID、随机 seed、backend
  handle 或 driver query 值(RFC-0019 §4.4 逐字;机验见 RXS-0305)。

### RXS-0305 Canonical serialization 规则

**Legality**

canonical bytes 是 reflection 文档的**唯一字节表示**,规则如下(全部强制):

1. **版本前缀**:字节串以 `"rurix.reflection.v1\0"`(ASCII + NUL)起始。
2. **整数**:一律小端定宽——计数/序号/枚举 tag 用 `u32 LE`;`location` 的
   `null` 编码为 `0xFFFF_FFFF` 哨兵;`count` 的无界哨兵为 `0`。
3. **字符串**:UTF-8 字节,以 `u32 LE` 长度前缀;空串 = 长度 0;不允许 NUL
   内嵌(接口名字面保证,出现即实现 bug,反射 fail-closed)。
4. **列表**:`u32 LE` 计数 + 元素顺序排列。entry 列表按规范键
   `(name, stage_tag)` 字典序排序(与源文件声明序无关);entry 内的 `io` 与
   `resources` 保持**声明序**(字段序/形参序本身是接口语义);其余集合字段
   (capabilities、RT 字段、library exports)按规范键排序——v1 恒空,编码为
   计数 0。
5. **禁用面**(RFC-0019 §4.4 逐字):canonical bytes 不得包含绝对路径、mtime、
   进程 ID、随机 seed、backend handle、driver query 值;编译单元文件名/目录也
   不得进入(「无语义路径」扰动不变性的承载条款)。
6. **digest 字段**:`selected_profile_digest` / `permutation_domain_digest` 等
   32 字节 digest 以原始字节(非 hex 文本)进入 canonical bytes;hex 仅用于
   JSON 产物与日志展示面。

**声明序扰动不变性的精确边界**(M31 判据的语义裁决):

- **透明**(hash 不变):entry 之间与 entry 内非接口 item(helper 函数、非 I/O
  结构体声明等)的声明次序变化;编译单元文件改名/移动目录(路径不入产物);
  函数体改写(仅 `source_digest` 变,见 RXS-0306)。
- **可见**(hash 必须变):`io` 字段声明序(驱动 location 分配)、资源形参声明序
  (驱动 binding 分配)、任一资源/字段/阶段字段值变化。

**Implementation Requirements**

- 实现必须提供逐字节可比的产物面:`--emit=reflection` 的 JSON 产物为确定性
  canonical JSON(键序固定、UTF-8、LF 行尾、整数不浮点),且每 entry 携带
  `canonical_hex`(canonical bytes 的 hex 展示),供双构建逐字节对拍。
- 排序实现不得依赖哈希迭代序;规范键比较为字节序字典序。

### RXS-0306 interface_hash 定义与 source/artifact digest 分离

**Legality**

逐字承 RFC-0019 §4.4:

```text
interface_hash = SHA-256("rurix.shader-interface.v1\0" || canonical_interface_bytes)
```

其中 `canonical_interface_bytes` = 该 entry 记录的 canonical bytes(RXS-0305),
**不含**函数体内容。

分离规则(判据字面):

- **source digest**:`source_digest = SHA-256("rurix.shader-source.v1\0" ||
  canonical_source_bytes)`;`canonical_source_bytes` = 接口字节 + entry 源文本段
  (签名至函数体原文,按 entry identity 排序拼接)。仅改变函数体(含注释/字面量)
  → `interface_hash` **不变**、`source_digest` **必变**;改变资源、I/O、payload、
  record、capability 或 profile 兼容性 → `interface_hash` **必变**。
- **artifact digest 的归属**:后端模块字节(SPIR-V blob,RXS-0290/0291 artifacts
  v2)的 digest 是后端发射面的内容寻址键,不在 reflection v1 产物内重复定义;
  reflection v1 以 `source_digest` 承担「同接口不同内容」的编译期区分证据,DDC
  往返的 artifact digest 组成归 M85/M80 接线(G8_ACCEPTANCE_MAP §2 对应行)。
- **下游 key 组成**(RFC-0019 §4.4 逐字口径的 v1 落地):DDC/PSO/RT pipeline key
  的 preimage = 版本前缀 `"rurix.pipeline-key.v1\0"` 顺次拼接 entry identity、
  `interface_hash`、`source_digest`、`selected_profile_digest`、
  `permutation_domain_digest`、`variant_key`、`compiler`、`compiler_version`、
  `edition`、`target`、`backend`(各字段按 RXS-0305 编码);`pipeline_key` =
  该 preimage 的 SHA-256(hex)。reflection 产物携带 `pipeline_key` 作为「hash
  已被记录为后续 DDC key 组成项」的编译期见证;`interface_hash` 变化 ⇒
  `pipeline_key` 必变(域分离 + 全字段覆盖)。
- hash mismatch 绝不通过重反射或 host layout 猜测修复(承 RXS-0307)。

**Implementation Requirements**

- SHA-256 实现 = `rurix-pkg` 的零依赖手写实现(RXS-0093 系,rurixc 经 path 依赖
  复用,不引外部 crate、不复制第三份实现)。
- `interface_hash` / `source_digest` / `pipeline_key` 均为纯函数:同输入恒同输出,
  与编译进程/机器/路径无关。

### RXS-0307 装配期核验与 fail-closed

**Legality**

- 装配/装载侧消费 reflection 产物时,必须先核对 `schema` 常量与
  `schema_version`,再比对 `interface_hash`;任一不符 = **fail-closed**
  (typed `Err` / 诊断拒绝),禁止以「重新反射一次」「按 host 端布局猜测」修补
  mismatch(RFC-0019 §4.4 逐字)。
- 编译期反射推导失败(绑定不可映射 / 形参超出已建模闭集,RXS-0304 的复用码
  类别)= 确定性诊断,退出非零,**不产**部分产物;reflection 通道不提供任何
  「尽力而为」降级模式。
- runtime 只比较 hash 后,仍须在 debug/validation 路径核对 schema version 与
  关键字段,防止错误归因(RFC-0019 §4.4 逐字)。

**Implementation Requirements**

- 编译器提供装配期核验原语 `verify_interface_pair`(host/safe 纯函数):输入两份
  entry 接口事实(schema、schema_version、interface_hash),一致 → `Ok`,任一不符
  → 携带相异字段名的 typed `Err`;函数体内不存在任何修复/再反射路径
  (by construction)。
- 核验失败的归因信息只携带**字段名级**事实(如 `"interface_hash"`,
  `"schema_version"`),不携带布局猜测值。

### RXS-0308 Permutation 域声明闭集(M29)

**Legality**

permutation 域为 **entry 级**声明(附着于着色入口函数的 `#[permutation(...)]`
属性;不引入跨 entry 命名域)。属性实参闭集——以下三形态即全部合法形态:

1. `axis(NAME, 值域)`:声明一根轴。值域三类(闭集,RFC-0019 §4.3 冻结面):
   - `bool` —— 组合值 ∈ {`false`, `true`}(规范序);
   - `enum(id0, id1, ...)` —— identifier 枚举,≥1 个成员,组合值 = 成员名;
   - `int(LO, HI)` —— 闭区间整数枚举,`LO <= HI`,组合值 = 区间内每个整数。
2. `forbid(NAME = 值, ...)`:禁组合行 = **等式合取**——组合同时满足行内全部
   等式即被裁剪。这是「无副作用编译期布尔式」的封闭子集(第一期冻结;`!`/`&&`/
   `||` 一般式与 capability 引用为加性演进面,须走本条款加性修订)。
3. `budget(N)`:声明合法组合数上限,`N` 为正整数。每 entry 至多一条。

合法性校验(违例 = 编译期确定性诊断,symbolic key
`shader.permutation_domain_invalid`,typeck 段数字码按实现 commit 当时实测
`next_free` 顺位领取;fail-closed,不产部分报告/反射产物):

- axis 重名(同 entry 内 NAME 重复);
- 空值域(`enum()` 零成员 / `int(LO, HI)` 且 `LO > HI`);
- `forbid` 引用未知 axis 名或该 axis 值域外的值;
- `budget` 非正整数或重复声明;
- 属性形态非上述闭集(未知子句/实参形态错误)。

`#[permutation]` 附着于非着色入口函数 = 同类违例(编译期拒)。

**Implementation Requirements**

- 属性提取与 `#[numthreads]` 家族同一机械(AST attr 面),校验在 reflection /
  permutation 求解前完成;任何违例都不得进入组合枚举。
- 泛型着色函数不产 entry(RXS-0304 口径),其上的 `#[permutation]` 不参与求解。

### RXS-0309 Canonical key 与 domain digest(M29)

**Legality**

- **规范域字节** `canonical_domain_bytes`:`"rurix.permutation-domain.v1\0"`
  起始;axis 按**名字节序**排列,每 axis 编码为 `(name 长度前缀字符串, type_tag
  u32 LE, 值域规范编码)`(`bool`=0 / `enum`=1 / `int`=2;enum 成员按声明序
  length-prefix 逐一编码——成员序是值域语义的一部分;int 编码 `LO`/`HI` 各
  `i64 LE`);随后 forbid 行按「行内等式按 axis 名字节序排序后整行字节」字节序
  排列逐行编码;budget 以 `u32 LE` 编码(未声明 = `0xFFFF_FFFF` 哨兵)。整数/
  字符串编码沿 RXS-0305 CanonW 律。
- `permutation_domain_digest = SHA-256("rurix.permutation-domain.v1\0" ||
  canonical_domain_bytes 去前缀段)`;**空域**(无 `#[permutation]`)恒为既有常量
  `SHA-256("rurix.permutation-domain-empty.v1\0")`——RXS-0304 空编码 0 漂移。
- **组合的 canonical key**(二进制形态):`"rurix.permutation-key.v1\0"` +
  按 axis 名字节序的 `(name, type_tag u32 LE, 组合值规范编码)` 序列(`bool` =
  `u32 LE` 0/1;`enum` = 成员名 length-prefix 字符串;`int` = `i64 LE`)。
- **字符串形态 key**(展示/`variant_key`/golden 比对):按 axis 名字节序拼接
  `NAME=值`,以 `;` 连接(如 `FOG=false;QUALITY=high`);`bool` 渲染
  `false`/`true`,`int` 渲染十进制,`enum` 渲染成员名。二进制 key 与字符串 key
  一一对应(同一排序、同一组合)。
- **确定性律**:axis/forbid 的**声明序**、编译单元路径/文件名、进程/机器因素
  不得影响任何 key 或 digest;两不同组合不得产生同一 key(单射,by
  construction——全轴覆盖 + 定界编码);同一组合跨 clean build 逐字节相等。

**Implementation Requirements**

- SHA-256 复用 `rurix-pkg` 手写实现(RXS-0306 同源);排序为字节序字典序,禁
  哈希迭代序泄漏。
- key/digest 计算为纯函数;digest 原始 32 字节进 canonical bytes,hex 仅展示面
  (RXS-0305 §6 同律)。

### RXS-0310 裁剪、预算与报告(M29)

**Legality**

- **求解律**:组合全集 = 各 axis 值域的笛卡尔积,`enumerated = ∏|axis|`;逐
  `forbid` 行裁剪,`pruned` = 被至少一行匹配的组合数;`emitted` = 余集(合法
  组合)。恒等式 `enumerated == pruned + emitted` 是结构保证,也是报告的强制
  断言字段。
- **预算律**:求解前先算 `enumerated`(整数算术,不物化组合表);`enumerated >
  budget` = **硬失败**——工具段确定性诊断(symbolic key
  `toolchain.permutation_budget_exceeded`,数字码按实现 commit 当时实测
  `next_free` 顺位领取),退出非零,且必须同时产出 **axis contribution
  report**(逐 axis 的 `|axis|` 与占比,JSON 报告面;供指认爆炸来源)。CLI
  `--permutation-budget=N` 覆盖 attr 声明值(RED 腿注入口)。`emitted == budget`
  为 GREEN(上限含等号)。
- **报告律**:`--emit=permutations` 产出确定性 JSON(无绝对路径/文件名/时间戳/
  进程因素,RXS-0305 禁用面同律):per-entry `{domain_digest, axes[],
  enumerated, pruned, emitted, keys[](字符串形态,字节序), axis_contribution[]}`。
  双次运行逐字节相等。
- **选择律**:`--permutation-select=KEY`(字符串形态):`KEY` ∉ 合法组合集 =
  确定性错误(同 `shader.permutation_domain_invalid` 类;**禁**「最接近」回退/
  模糊匹配);选中后该 entry 的 reflection `variant_key = KEY`、
  `permutation_domain_digest` 为真值化 digest(RXS-0304 v1.1 行),`pipeline_key`
  preimage 既含二字段(RXS-0306)故随之分裂——零新接缝。
- 空域 entry 与非空域 entry 可共存于同一编译单元;空域 entry 的 reflection
  产物必须与 M31 基线逐字节一致(0 漂移见证)。

**Implementation Requirements**

- 报告 JSON 键序固定、UTF-8、LF(RXS-0305 实现要求同律);`keys[]` 按字符串
  字节序排列。
- 预算判定在组合物化前完成;超预算路径不得有部分组合表泄漏进报告(报告只含
  axis 元数据与三计数)。
- per-variant body specialization codegen **不在本条款范围**(v1 判据不要求;
  capability 约束裁剪归 M32 条款族,二者合入后的交叉接线走加性修订)。

### RXS-0312 Profile 闭集、构建期选择律与 fallback(M32)

**Legality**

- **profile v1 模型**(版本化闭集,RFC-0019 §4.5.2 逐字;由项目/构建 manifest
  选择,**不从当前开发机自动生成**):JSON 文件,字段闭集 = `{schema:
  "rurix.profile.v1", name, version, required[], optional[], forbidden[],
  fallbacks: {逻辑 entry 名: fallback entry 名}}`。`required`/`optional`/
  `forbidden` 元素 ∈ RXS-0311 capability ID 闭集(未知 ID = 装载 profile 时
  确定性拒,`capability.unknown_id` 同类);三集两两不相交(交集非空 = profile
  非法,确定性拒)。
- **canonical bytes 与 digest**:`canonical_profile_bytes` 沿 RXS-0305 CanonW
  律(版本前缀 `"rurix.profile.v1\0"` + name/version length-prefix + 三集各
  按字节序排序编码 + fallbacks 按键字节序编码);`selected_profile_digest =
  SHA-256("rurix.profile.v1\0" || canonical_profile_bytes 去前缀段)`。
  **无 `--profile`** 时恒为既有常量 `SHA-256("rurix.profile-none.v1\0")`
  (RXS-0304 空编码 0 漂移,此时本条款其余判定全部不触发——行为与 M32 前
  逐字节一致)。
- **构建期选择律**(每 entry 独立判定):
  1. entry 有效 requirement 集(RXS-0311)∩ `forbidden` ≠ ∅ → **编译期 RED**,
     symbolic key **`capability.forbidden_used`**;
  2. 有效集 ⊆ (`required` ∪ `optional`) → 合法,entry 照常发射;
  3. 有效集含 profile 未提供的 capability:
     - `fallbacks` 有该 entry 的映射且 fallback entry **接口契约兼容** →
       选中 fallback,**主 variant 不发射**(「只生成允许的 specialization」
       判据字面);fallback entry 自身有效集仍须满足本选择律(递归判定,
       fallback 链深度 1——fallback 的 fallback 不支持,v1 冻结);
     - 无映射 → **编译期 RED**,symbolic key **`capability.missing_required`**
       (消息携带缺失 ID + 首个引入 callee,RXS-0311);
     - 有映射但不兼容 → **编译期 RED**,symbolic key
       **`capability.fallback_incompatible`**(消息给出不兼容字段)。
- **接口契约兼容判定**(v1 从严,宽松化走加性修订):两 entry 的 `io`、
  `resources`、`push_constants`、`execution_modes` 四字段**结构相等**
  (reflection v1 同一提取律的内部比较;stage 必须相同)。
- **选择结果落报告**:`--emit=capabilities` 产出确定性 JSON(RXS-0305 禁用面
  同律):per-entry `{effective_requirements[], status(emitted/fallback/...),
  selected_entry, missing[], forbidden_hits[]}`——selection manifest 是
  runtime 装配的事实源之一(P-11)。
- codegen 根收集按 selection 过滤:`--profile` 未给时行为 **0-byte**。

**Implementation Requirements**

- profile 解析/判定/digest 为纯 host safe 函数;digest 原始 32 字节进
  canonical bytes,hex 仅展示面。
- fallback 选中时,reflection 文档内该逻辑 entry 记录的接口事实取 **fallback
  entry 的实体**(名字段 = fallback entry 的 entry identity;逻辑名→实体映射
  在 capabilities 报告中可查),`selected_profile_digest`/`required_capabilities`
  照 RXS-0304 v1.2 行真值化。

### RXS-0313 运行期 capability snapshot 核验与 fail-closed(M32)

**Legality**

- 运行时装载编译产物时,必须以 **device capability snapshot**(运行期实测)
  对照产物所选 profile 的 `required` 集:任一 required capability 在 snapshot
  中缺失 = **装载期 RED**(fail-closed,typed `Err`),symbolic key
  **`capability.runtime_snapshot_mismatch`**;**禁止**临时重编、**禁止**静默
  换 profile、**禁止**「尽力而为」降级(RFC-0019 §4.5.2 逐字)。
- 该失败为 rurix-rt 库层 typed `Err`(镜像 RX6029/6030 口径**不占 RX 数字码**;
  诊断文本携带 symbolic key 字面与缺失 ID 表)。
- 核验先于任何 pipeline 创建/资源绑定;失败后不产生部分装配状态。

**Implementation Requirements**

- 编译器侧提供装配期核验原语 **`verify_profile_snapshot`**(host/safe 纯函数,
  镜像 RXS-0307 `verify_interface_pair` 体例):输入 = 产物 profile 事实
  (digest + required 集)与 snapshot 事实(可用 capability 集),满足 → `Ok`,
  缺失 → 携带缺失 ID 表的 typed `Err`;函数体内不存在任何修复/重编/换
  profile 路径(by construction)。M32 门只落该 host 原语与其单测;device 腿
  消费(真实 snapshot 采集)归 M50/M89 device 门。

---

## 3. 与其他 spec 文件的关系

- `shader_stages.md`(RXS-0153~0156/0242~0245/0297~0299):着色阶段**类型面**与
  非法接口的编译期拒绝;本文件消费其类型面事实(stage 集合、I/O 标注、资源句柄
  类型),不改其判据。
- `vulkan_backend.md`(RXS-0230/0290/0291):Vk-native set 分配律与 artifacts v2
  blob;本文件的绑定推导律引用其为单一事实源,artifact digest 归属声明见其条款。
- `binding_layout.md`(RXS-0163~0166/0233):host 侧绑定推导;本文件复用
  `infer_spirv_bindings_vk_native` 为 graphics 绑定唯一来源。

## 4. 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.2 | 2026-08-06 | §2 追加 **RXS-0312 ~ RXS-0313**(G8.2 M32 capability_profile 硬门 `g8.p0.m32.capability_profile`,RP-CAP-PROFILE materialize,spec-first 条款先行,硬规则 7;`#[requires]`/ID 闭集/调用图并集律 = spec/shader_stages.md v1.7 RXS-0311 同 PR):RXS-0312 profile 闭集、构建期选择律与 fallback(profile v1 JSON 闭集 schema=rurix.profile.v1 + required/optional/forbidden 两两不相交 + fallbacks 映射;canonical bytes/digest 沿 CanonW 律,无 --profile 恒 rurix.profile-none.v1 常量 0 漂移;选择律四分支绑定 RFC-0019 §4.5.1 冻结 symbolic key `capability.forbidden_used`/`capability.missing_required`/`capability.fallback_incompatible`;fallback 接口契约兼容 = io/resources/push_constants/execution_modes 结构相等 v1 从严,选中 fallback 主 variant 不发射,fallback 链深度 1;`--emit=capabilities` selection manifest 确定性 JSON;--profile 未给 0-byte)/ RXS-0313 运行期 capability snapshot 核验 fail-closed(`capability.runtime_snapshot_mismatch` 装载期 RED,禁临时重编/静默换 profile/尽力而为;库层 typed Err 不占 RX 码;`verify_profile_snapshot` host 纯函数原语镜像 RXS-0307 体例,device 腿归 M50/M89)。**RXS-0304 加性修订**:`required_capabilities`/`selected_profile_digest` 两行由「M32 未实现空编码」真值化(空集/无 profile 路径恒既有常量,0 字节漂移见证进 smoke)。编号自 ledger 实测 next_free 顺位领取(RXS-0311~0313,v1.50 校准 310/311→313/314);typeck 段数字错误码按实现 commit 实测领取(条款以 RFC 四键 + `capability.unknown_id` 五个 symbolic key 冻结)。依据 [RFC-0019](../rfcs/0019-rendering-platform.md)(§4.5.1/§4.5.2/§5 RP-CAP-PROFILE 行)+ G8_ACCEPTANCE_MAP §2 M32 行 + G8.2 设计案 §2。既有条款 RXS-0304 其余行/0305~0310 0-byte。 | **Full RFC**(RFC-0019) |
| v1.1 | 2026-08-06 | §2 追加 **RXS-0308 ~ RXS-0310**(G8.2 M29 shader permutation 硬门 `g8.p0.m29.shader_permutation`,RP-PERMUTATION materialize,spec-first 条款先行):RXS-0308 permutation 域声明闭集(entry 级 `#[permutation(axis/forbid/budget)]`,axis 三类值域,forbid = 等式合取封闭子集,违例编译期确定性拒)/ RXS-0309 canonical key 与 domain digest(axis 名字节序 + 带类型标签规范编码,CanonW 律,声明序/路径不影响 key,组合→key 单射,空域恒既有常量 0 漂移)/ RXS-0310 裁剪·预算·报告·选择律(`enumerated == pruned + emitted` 恒等式,预算先算不物化、超限硬失败 + axis contribution report,`--emit=permutations` 确定性 JSON,`--permutation-select` 缺 variant 确定性错误禁最接近回退)。**RXS-0304 加性修订**:`permutation_domain_digest`/`variant_key` 两行由「M29 未实现恒空编码」真值化(非空域→真 digest/真 variant_key;空域路径恒既有常量,0 字节漂移见证进 smoke)。编号自 ledger 实测 `RXS.next_free` 顺位领取(RXS-0308~0310;同 PR 校准 M31 滞后 on_tree_max 303→310/next_free 304→311,v1.48);typeck/工具段数字错误码按实现 commit 实测领取,条款先以 symbolic key 冻结(`shader.permutation_domain_invalid` / `toolchain.permutation_budget_exceeded`)。依据 [RFC-0019](../rfcs/0019-rendering-platform.md)(Agent Approved 2026-08-02,§4.3/§5 RP-PERMUTATION 行)+ G8_ACCEPTANCE_MAP §2 M29 行 + G8.2 设计案 §1。既有条款 RXS-0304 其余行/RXS-0305~0307 0-byte。 | **Full RFC**(RFC-0019) |
| v1.0 | 2026-08-05 | 新建(G8.2 M31 实现 PR,RP-REFLECTION materialize):RXS-0304 reflection v1 schema 与字段闭集(含 M29/M32/M50 未实现字段的确定性空编码与 RT 枚举诚实边界)/ RXS-0305 canonical serialization 规则(版本前缀 + length-prefix + 规范键排序 + 禁用面 + 声明序扰动精确边界)/ RXS-0306 interface_hash 定义与 source/artifact digest 分离 + pipeline key 组成见证 / RXS-0307 装配期核验与 fail-closed。实现锚定 `src/rurixc/src/reflection.rs` + `--emit=reflection` + conformance/reflection 语料同 PR。依据 RFC-0019 §4.4/§5(RP-REFLECTION 行)/§6.2 M31 行。 | **Full RFC**(RFC-0019) |
