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
| `required_capabilities` | **M32 未实现,确定性空编码**:恒空表(空集) |
| `selected_profile_digest` | **M32 未实现**:恒为「未选择 profile」的规范 digest = `SHA-256("rurix.profile-none.v1\0")`(hex) |
| `permutation_domain_digest` | **M29 未实现**:恒为空域的规范 digest = `SHA-256("rurix.permutation-domain-empty.v1\0")`(hex) |
| `variant_key` | **M29 未实现**:恒空串 |
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
| v1.0 | 2026-08-05 | 新建(G8.2 M31 实现 PR,RP-REFLECTION materialize):RXS-0304 reflection v1 schema 与字段闭集(含 M29/M32/M50 未实现字段的确定性空编码与 RT 枚举诚实边界)/ RXS-0305 canonical serialization 规则(版本前缀 + length-prefix + 规范键排序 + 禁用面 + 声明序扰动精确边界)/ RXS-0306 interface_hash 定义与 source/artifact digest 分离 + pipeline key 组成见证 / RXS-0307 装配期核验与 fail-closed。实现锚定 `src/rurixc/src/reflection.rs` + `--emit=reflection` + conformance/reflection 语料同 PR。依据 RFC-0019 §4.4/§5(RP-REFLECTION 行)/§6.2 M31 行。 | **Full RFC**(RFC-0019) |
