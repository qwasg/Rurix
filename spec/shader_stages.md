# Rurix 语言规范 — 着色阶段类型面（vertex / fragment / compute / mesh / task / RT 着色阶段作为 kernel 着色扩展；G2.1 起）

> 条款:RXS-0153 起续号预留(G2.1 着色阶段类型面:着色阶段函数着色规则（vertex/fragment/compute/mesh/task + RT raygen/closesthit/anyhit/miss 作为新 coloring，扩展 RXS-0066）/ 阶段专属 I/O 语义类型（插值限定 type-level + 内建变量类型化）/ 阶段间接口类型契约（vertex out → fragment in 兼容性编译期校验）/ 资源句柄·纹理采样器参数化类型的类型面（`Texture2D<F>`/`Sampler` 类型形态，平行 `View<space,T>`）)。体例见 [README.md](README.md)。
> 依据:**[RFC-0002](../rfcs/0002-shader-stages.md)**(着色阶段进语言的类型面 vertex/fragment/compute/mesh/task/RT 作为 kernel 着色扩展,**owner 已批准定稿**,2026-06-23);06 §8.2(着色阶段 = kernel 着色扩展,设计预留);06 §4.2(纹理路径内存模型禁区,🔒);05 §1(device⊂host 单向可达,kernel 子语言受限子集);05 §2.2(trait 单态化子集 D-104,无 dyn/特化/HKT/async);spec/device.md:RXS-0066(函数着色与跨着色调用合法性)/ RXS-0067(地址空间类型与一致性 `View<space,T>`)/ RXS-0074(launch 类型契约)/ RXS-0078(views 算子集)。授权:[../milestones/g2/G2_CONTRACT.md](../milestones/g2/G2_CONTRACT.md)(`in_scope: shader_stages_in_lang` / `spec_g2_clauses`,D-G2-1 / D-G2-6,G-G2-1 / G-G2-6)+ [../milestones/g2/G2_PLAN.md](../milestones/g2/G2_PLAN.md) G2.1 首子里程碑。
> 档位:**Full RFC**(RFC-0002;10 §3:本设计触 **新语法 + 类型系统扩张**——着色阶段函数着色、阶段 I/O type-level 标注、阶段间接口契约、纹理采样器参数化类型,AGENTS 硬规则 5 / 10 §3 Full RFC 触发面,经 Full RFC 由 agent 自主落笔)。RFC-0002 已由 agent 于 2026-06-23 在工作会话明确裁决 §9 Q1~Q6 与 §4.5 🔒 禁区边界处置(agent 自主);spec 条款 PR 与实现 PR 均门控于 RFC-0002 合入之后(本脚手架门控于 main 已含 RFC-0002 之后)。**agent 自主判档**,判档以 RFC-0002 与 G2_CONTRACT 授权为据,判档争议向上取严。任何偏离 RFC-0002 已批准设计、或触及 **G2.2 DXIL codegen(D-131)** / **G2.3 绑定布局推导(P-11)** / **🔒 纹理路径内存模型映射(06 §4.2)** / **多后端(D-008/SG-003)** / **Python 原生嵌入(红线 1,SG-008)** 的条款,必须停下标注「需升档」,不在本文件自行落笔。**严禁 UB 节**(10 §7.5):着色阶段误用 / 阶段间接口不匹配 / 资源句柄违例以 **编译期类型/着色/接口诊断(P-01 strict-only,无运行期回退)**定义,不以 UB 表述(RFC-0002 §3/§4)。
> 规范先行(AGENTS.md 硬规则第 7 条):**条款 PR 先于实现 PR**;缺条款的语义 PR 必须先补 spec。`ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 ≥1 测试锚定(`//@ spec: RXS-####`)。**本脚手架 PR 沿 README v1.32 interop_d3d12.md / v1.33 async_buffer.md 先例:仅登记新文件名 + 预留区间,不落带编号裸条款头**——条款体(RXS-0153 起)与每条 ≥1 测试锚定随 G2.1 实现 PR(PR-B2,步骤 45)同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定 152/152)。

---

## 1. 范围与编号区间

本文件承载 **着色阶段类型面**的语义条款(G2.1+,D-G2-1)。覆盖语义面(RFC-0002 §4):

- **着色阶段函数着色(扩展 RXS-0066 function coloring)**:`vertex` / `fragment` / `compute`(D3D12 语境,复用既有 `kernel` 着色) / `mesh` / `task` fn + RT `raygen` / `closesthit` / `anyhit` / `miss` fn,作为**新增 coloring 类别**接入既有着色格(RFC-0002 §9 Q1:前缀式 `<stage> fn`,否决属性式)。各着色阶段函数体复用 **kernel 子语言类型系统**(05 §1 设备受限子集)+ **views**(`View<space,T>` RXS-0067 / views 算子集 RXS-0078);不引入第二套设备子语言。遵守 **device⊂host 单向可达**(05 §1):着色阶段属设备侧着色,可调 `device fn`,host 不可直接进入着色阶段体;跨着色非法调用复用既有 RX3001 类别 + 着色阶段专属新类别。**trait 单态化子集**(D-104,05 §2.2):着色阶段选择/分发编译期静态,无 `dyn`/特化/HKT/async。
- **阶段专属 I/O 语义类型**:插值限定(perspective/linear/flat/centroid 等)与内建变量(顶点 `position` 输出、`vertex-id`/`instance-id` 输入、片元 `frag-coord`、计算 `thread-id` 等)以**属性式标注** `#[interpolate(..)]` / `#[builtin(..)]` 表达(RFC-0002 §9 Q2:属性式,否决 type-level 包裹);**无标注字段 = 编译期拒绝**(P-01 strict-only,不默认插值、不静默放行)。内建变量在 codegen 后端的寄存器/语义槽映射属 G2.2,不在本文件。
- **阶段间接口类型契约**:相邻着色阶段经类型契约连接,**vertex out → fragment in** 类型兼容性(字段、类型、插值限定一致)编译期校验;不兼容 → 阶段间接口不匹配新类别。网格管线(`task`→`mesh`→`fragment`)与 RT 管线(`raygen`↔`closesthit`/`anyhit`/`miss` 经 payload/attribute 类型)的阶段间接口**并入同条款**(RFC-0002 §9 Q5)。契约为编译期静态校验(HIR/typeck 层),无运行期协商(P-01 strict-only)。
- **资源句柄 / 纹理采样器参数化类型的类型面**:`Texture2D<F>`(格式参数化)与 `Sampler` 作为**参数化类型**进入着色阶段签名,平行于 `View<space,T>`(RXS-0067)——平行但不强制完全同构(RFC-0002 §9 Q4);**首批仅 `Texture2D<F>` + `Sampler`**,其余纹理维度(`Texture1D`/`Texture3D`/`TextureCube`/`Array`)**defer** 后续。资源句柄(descriptor 可绑定资源)在签名中作类型化句柄,其**绑定布局推导**属 G2.3(P-11),不在本文件——本文件仅定义句柄在签名中的类型表达供 G2.3 消费。资源句柄违例(句柄类型与着色阶段不相容、句柄非法位置)作编译期拦截新类别。

全部着色阶段语义维持**仅类型面/语法面**:**DXIL codegen**(G2.2,内建变量寄存器/语义槽映射、DXIL 文本 golden)、**绑定布局推导实现**(G2.3,descriptor/root signature 生成)、**🔒 纹理/采样器内存模型映射**(06 §4.2 禁区:tex proxy / 采样 opcode / 描述符编码 / 缓存一致性 / UB,留后续独立 Full RFC,RFC-0002 §4.5)均**不在本文件**;device 分发维持 **PTX-only**(07 §7;DXIL 第二后端无 MVP 期 PTX↔DXIL 对应,完全于 G2.2 重评估,D-131)。着色阶段误用 / 阶段间接口不匹配 / 资源句柄违例以 **编译期类型/着色/接口诊断(P-01 strict-only)**定义,**不以 UB 表述**(§4)。

**编号区间**:本文件条款自 **RXS-0153** 起续号(全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1;最高现存 RXS-0152 @ [release.md](release.md))。**区间已锁定 4 条 RXS-0153 ~ RXS-0156**(RFC-0002 §9 Q5,owner 2026-06-23 裁决:不预留、不预造;网格/RT 阶段间接口并入 RXS-0155、纹理类型集合并入 RXS-0156,本里程碑不拆条)。本轮(脚手架)**仅登记区间预留 RXS-0153 ~ RXS-0156**,**不落带编号裸条款头**;条款体与每条 ≥1 测试锚定随 G2.1 实现 PR(PR-B2,步骤 45)同落。区间登记于 [README.md](README.md) §4 文件清单。

## 2. 条款（RXS-0153 ~ RXS-0156，带编号条款体）

> 区间锁定 4 条 `RXS-0153 ~ RXS-0156`(RFC-0002 §9 Q5,owner 2026-06-23:不拆条、不预留、不预造)。条款体随 G2.1 实现 PR(PR-B2,步骤 45)落地(条款 PR 先于实现 PR,trace_matrix 维持全锚定);每条 ≥1 `//@ spec: RXS-####` 测试锚定。各条按需分 **Syntax / Legality / Dynamic Semantics / Implementation Requirements** 节,**严禁 UB 节**(着色阶段误用 / 阶段间接口不匹配 / 资源句柄违例以编译期类型/着色/接口诊断定义,P-01 strict-only,无运行期回退;10 §7.5)。**本批仅类型面/语法面 + 编译期拦截**:DXIL codegen(G2.2)/ 绑定布局推导(G2.3)/ 🔒 纹理采样器内存模型映射(06 §4.2 禁区)均不在本文件。着色阶段类型面 gate 于 cargo feature `shader-stages`(RFC-0002 §6;未启用时着色阶段语法/类型面不参与编译)。

### RXS-0153 着色阶段函数着色规则

**Syntax**(前缀式 `<stage> fn`,RFC-0002 §9 Q1;着色阶段名为上下文关键字,词法层按标识符产出,仅在 item 起始位置且其后紧跟 `fn` 时识别为着色阶段前缀):

```
ShaderFn ::= Stage "fn" Ident GenericParams? "(" Params? ")" ("->" Type)? Block
Stage    ::= "vertex" | "fragment" | "compute" | "mesh" | "task"      // 图形 / 计算阶段
           | "raygen" | "closesthit" | "anyhit" | "miss"             // 光线追踪阶段
```

**Legality**(着色阶段作为 **kernel 着色扩展**接入既有着色格,扩展 RXS-0066;05 §1 device⊂host 单向可达;05 §2.2 trait 单态化子集 D-104):

- 着色阶段函数取 **kernel 入口着色**:与 `kernel fn` 平行,着色阶段是 GPU/管线入口,**不可被直接调用**(经管线/分发发起,运行期分发面不在本文件)——任意上下文直接调用着色阶段入口 → `RX3001`(跨着色非法调用,复用 RXS-0066 既有类别,RFC-0002 §5「复用既有 RX3001」)。
- **compute 阶段复用既有 kernel 着色**(RFC-0002 §9 Q1):D3D12 语境 `compute fn` 与 `kernel fn` 同享 kernel 入口着色,不另立独立 coloring。
- 着色阶段函数体为 **device 上下文**:复用 kernel 子语言类型系统(05 §1 设备受限子集)与 views(`View<space,T>` RXS-0067 / views 算子集 RXS-0078);可调用 `device fn` / `const fn`,调用 host 着色函数非法 → `RX3001`(device⊂host 单向可达,RXS-0066)。
- 着色阶段选择与分发为**编译期静态**(D-104):无 `dyn` 派发 / 特化 / HKT / async;着色在 HIR 层静态可判(07 §3 着色检查无数据流)。

**Implementation Requirements**:着色阶段前缀关键字在 parser 以上下文关键字识别(gate `shader-stages`);着色阶段函数置 `kernel` 着色 + 阶段标记(`FnColor::Kernel` + `stage`);跨着色调用合法性复用着色检查(RXS-0066,[`coloring`](../src/rurixc/src/coloring.rs))无新发码;着色阶段函数**不进 device codegen 收集根**(本里程碑仅类型面,PTX 后端不收集图形/RT 着色阶段;DXIL codegen 属 G2.2)。

> 锚定测试:`conformance/shader/accept/*.rx`(合法着色阶段声明 0 诊断)、`conformance/shader/reject/stage_misuse/*.rx`(直接调用着色阶段入口 `RX3001`);`tests/ui/shader/direct_call.rx`(`RX3001` snapshot);`shader_stages` 单测。

### RXS-0154 阶段专属 I/O 语义类型

**Syntax**(阶段 I/O 标注属性式,RFC-0002 §9 Q2;否决 type-level 包裹类型):

```
Field ::= ("#[" "interpolate" "(" InterpMode ")" "]" | "#[" "builtin" "(" BuiltinVar ")" "]")? Ident ":" Type
```

**Legality**(着色阶段 I/O 聚合类型字段标注合法性,P-01 strict-only):

- 着色阶段输入/输出聚合类型(着色阶段函数形参/返回位置出现的命名结构体)的**每个字段须携带** `#[interpolate(..)]`(插值限定:`perspective`/`linear`/`flat`/`centroid`/`noperspective`/`sample`)**或** `#[builtin(..)]`(内建变量:`position`/`vertex_id`/`instance_id`/`frag_coord`/`thread_id`/`front_facing`/`depth`/`primitive_id`)。
- **无标注字段 = 编译期拒绝** → `RX3011`(不默认插值、不静默放行,P-01 strict-only)。
- `#[builtin(name)]` 的 `name` 不在已知内建变量集 / `#[interpolate(mode)]` 的 `mode` 不在已知插值限定集 → `RX3011`(未知内建变量 / 未知插值限定)。
- 内建变量在 codegen 后端的寄存器/语义槽映射属 G2.2(DXIL codegen),不在本文件。

**Implementation Requirements**:I/O 字段标注检查在 AST 层实施([`shader_stages`](../src/rurixc/src/shader_stages.rs));检查面 = 着色阶段函数形参/返回位置可达的命名结构体(非资源句柄类型);诊断 span 指向违例字段。普通(非着色阶段 I/O)结构体不受本条约束。

> 锚定测试:`conformance/shader/reject/io_annotation/*.rx`(无标注字段 `RX3011`);`tests/ui/shader/unannotated_field.rx`(`RX3011` snapshot);`shader_stages` 单测(无标注 / 未知 builtin / 未知插值)。

### RXS-0155 阶段间接口类型契约

**Legality**(相邻着色阶段经类型契约连接,编译期静态校验,HIR/AST 层无运行期协商,P-01 strict-only):

- **vertex out → fragment in 兼容性**:`fragment` 输入的插值 varying 字段(`#[interpolate(..)]`)须与上游 `vertex` 输出的 varying **逐一兼容**——字段名、字段类型、插值限定一致;`fragment` 输入存在无对应上游 `vertex` 输出 varying 的字段 → `RX3012`(阶段间接口不匹配)。
- 当 `fragment` 输入与 `vertex` 输出引用**同一命名接口结构体**时同型兼容(契约满足);引用不同结构体时按上述 varying 逐一比对。
- **网格管线**(`task`→`mesh`→`fragment`)与 **RT 管线**(`raygen`↔`closesthit`/`anyhit`/`miss` 经 payload/attribute 类型)的阶段间接口**并入本条款**(RFC-0002 §9 Q5);其 payload/attribute 类型化契约形态本里程碑取保守上界(`mesh` 输出比照 vertex 输出参与契约),完整 payload/attribute 契约随后续里程碑细化(不拆条)。

**Implementation Requirements**:接口契约校验在 AST 层([`shader_stages`](../src/rurixc/src/shader_stages.rs)),消费 `vertex`/`mesh` 输出 varying 集与 `fragment` 输入 varying 集;上游无 `vertex`/`mesh` 输出时保守跳过(不误报);诊断 span 指向 `fragment` 输入接口类型。

> 锚定测试:`conformance/shader/reject/interface_mismatch/*.rx`(varying 不兼容 `RX3012`)、`conformance/shader/accept/*.rx`(共享接口结构体兼容 0 诊断);`tests/ui/shader/interface_mismatch.rx`(`RX3012` snapshot);`shader_stages` 单测。

### RXS-0156 资源句柄 / 纹理采样器参数化类型的类型面

**Syntax**(资源句柄/纹理采样器作参数化类型进入着色阶段签名,平行 `View<space,T>` RXS-0067 但不强制完全同构,RFC-0002 §9 Q4):

```
ResourceTy ::= "Texture2D" "<" Type ">"   // 格式参数化纹理(首批仅 2D)
             | "Sampler"                  // 采样器类型形态
```

**Legality**(资源句柄位置合法性;**首批仅 `Texture2D<F>` + `Sampler`**,其余纹理维度 defer,RFC-0002 §9 Q4):

- `Texture2D<F>` / `Sampler` **仅可作着色阶段函数签名形参**(descriptor 可绑定资源的类型化句柄);出现在**返回位置** / **结构体字段** / **非着色阶段函数签名** → `RX3013`(资源句柄违例:句柄是输入参数,非可返回值/可聚合字段)。
- 其余纹理维度(`Texture1D` / `Texture3D` / `TextureCube` / `*Array` 等)**首批不支持(defer)** → `RX3013`(未支持纹理维度)。
- 资源句柄的**绑定布局推导**(host 结构体 ↔ shader 布局单一事实源,P-11)属 G2.3,不在本文件——本文件仅定义句柄在签名中的类型表达供 G2.3 消费。
- 🔒 `Texture2D<F>` / `Sampler` **仅类型面参数化形态**,**不承诺任何采样语义、内存序或一致性保证**;纹理/采样器内存模型映射(tex proxy / 采样 opcode / 描述符编码 / 缓存一致性 / 采样 UB)属 06 §4.2 禁区,留后续独立 Full RFC(RFC-0002 §4.5 / §9 Q6)。

**Implementation Requirements**:资源句柄类型按类型头名识别(`Texture2D`/`Sampler`;`Texture*` 其余维度归未支持)于 AST 层([`shader_stages`](../src/rurixc/src/shader_stages.rs));位置合法性逐 item 走查(返回/字段/形参);诊断 span 指向句柄类型。资源句柄类型为容忍未知名(resolve 类型位置容忍 `Res::Err`,RXS-0047 不级联),不引入名称解析新已知类型(codegen/绑定语义不在本 PR)。

> 锚定测试:`conformance/shader/reject/resource_handle/*.rx`(句柄返回位置违例 `RX3013`)、`conformance/shader/accept/*.rx`(`Texture2D<F>`+`Sampler` 作 fragment 形参 0 诊断);`tests/ui/shader/handle_return.rx`(`RX3013` snapshot);`shader_stages` 单测(返回位置 / 未支持维度)。

### RXS-0174 采样表达式类型面（`Texture2D<F>.sample(Sampler, vec2<f32>) → vec4<F>`，承 RXS-0156，RFC-0007）

> **编号续号说明**:本条 RXS-0174 由 **RFC-0007**(纹理采样语义本体)新增,**超出 G2.1/RFC-0002 锁定的 RXS-0153~0156 区间**(独立 RFC 续号,全 spec 唯一递增、永不复用)。本条把 RXS-0156 的 opaque 资源句柄类型面升级为**可在着色 body 求值的采样表达式**类型面,是 UC-04 lighting pass 真采样 G-buffer 的语言前置。

**Syntax**(采样表达式,复用既有方法调用产生式 `MethodCall`,无新 token):

```
SampleExpr ::= Expr "." "sample" "(" Expr "," Expr ")"
            // receiver.sample(sampler, coord):receiver = Texture2D<F> 句柄形参引用,
            // sampler = Sampler 句柄形参引用,coord = vec2<f32> 归一化 UV
```

**Legality**(采样表达式合法性;**首期收敛子集**,RFC-0007 §4.2):

- `tex.sample(samp, coord)` 合法当且仅当:`tex : Texture2D<F>`(F = 已建模标量分量类型,首期 `f32`)、`samp : Sampler`、`coord : vec2<f32>`、**且包含该表达式的函数为 `fragment` 着色阶段**。
- receiver(`tex`)与第一实参(`samp`)必须**直接是资源句柄形参引用**(句柄非值类型,承 RXS-0156:不可存入 `let`/结构体后再采样)。
- 违例(receiver 非 `Texture2D<F>` / `samp` 非 `Sampler` / `coord` 非 `vec2<f32>` / 元数不符 / 非 fragment 阶段采样) → `RX3014`(采样表达式类型/阶段违例,编译期 strict-only 拦截)。
- 结果类型 = `vec4<F>`(SPIR-V/DXIL 采样结果恒 4 分量;首期 `vec4<f32>`),可被后续白名单算术消费或作为输出 I/O 字段写出。

**Dynamic Semantics**(采样语义本体的**类型面投影**;完整内存模型映射见 spec/dxil_backend.md RXS-0176 🔒):采样在纹理基础 mip 层(**显式 LOD 0**,规避 fragment 隐式导数,RFC-0007 §4.6)、按绑定 sampler 过滤模式、在归一化坐标 `coord ∈ [0,1]²` 处读取,产 `vec4<F>`。**非 fragment 阶段采样 / 隐式 LOD / 任意 mip 层 / texel fetch / 比较采样 / 多分量纹理 / 可配置 sampler 状态** = 首期**规避项**,登记 deferred **RD-022~RD-024**(RFC-0007 §8),触及即 `RX3014`(类型面)或 `RX6023`(codegen 面),**不静默降级**。

**Implementation Requirements**:typeck 于 AST 层([`shader_stages`](../src/rurixc/src/shader_stages.rs))识别 `method == "sample"` 且 receiver 类型为 `Texture2D<F>` 时按本条规则核对(receiver/`samp` 解析回资源句柄形参、`coord` 类型、阶段);MIR 降级为 `Rvalue::ResourceSample`(spec/dxil_backend.md RXS-0175),SPIR-V `OpImageSampleExplicitLod`(Lod 0)。诊断 span 指向采样表达式。

> 锚定测试:`conformance/shader/reject/sample/*.rx`(采样违例 `RX3014`:非 fragment / 类型不符)+ `tests/ui/shader/*.stderr`(`RX3014` snapshot);accept 经 `conformance/dxil/graphics/accept/uc04_lighting_fs.rx`(fragment 真采样 0 诊断);`shader_stages` 单测(采样类型规则 accept/reject)。

## 3. 错误码引用汇总（RX3011 ~ RX3014）

> 三类编译期拦截(着色阶段误用 / 阶段间接口不匹配 / 资源句柄违例)属 **Rurix 语义诊断**(编译期可检的着色/接口/句柄合法性,对齐 RXS-0066 着色诊断先例),归 **3xxx 着色/地址空间段位续号**(07 §5 语义分配;接 RX3010 之后 **RX3011+**——**非全局 7xxx 段**,7xxx 为运行期/互操作段)。纯 Rust 通用错误(类型不符等)走 rustc 原生诊断(零新 RX)。

| 错误码 | 含义 | 条款 |
|---|---|---|
| RX3001(复用) | 着色阶段误用:直接调用着色阶段入口 / 着色阶段体内调 host 着色函数(跨着色非法调用,着色阶段取 kernel 入口着色) | RXS-0153, RXS-0066 |
| RX3011 | 着色阶段 I/O 标注违例:着色阶段 I/O 字段无 `#[interpolate(..)]`/`#[builtin(..)]` 标注 / 未知 builtin 名 / 未知插值限定 | RXS-0154 |
| RX3012 | 阶段间接口不匹配:fragment 输入 varying 与上游 vertex 输出名/类型/插值限定不兼容 | RXS-0155 |
| RX3013 | 资源句柄违例:`Texture2D`/`Sampler` 出现在返回位置 / 结构体字段 / 非着色阶段签名,或未支持纹理维度(defer) | RXS-0156 |
| RX3014 | 采样表达式违例:`tex.sample(samp, coord)` receiver 非 `Texture2D<F>` / `samp` 非 `Sampler` / `coord` 非 `vec2<f32>` / 元数不符 / 非 fragment 阶段采样(首期收敛子集外) | RXS-0174 |

**只追加、不预造**:RX3011~3014 按**实现中真实可达、用户可行动**的错误类别只追加(着色阶段误用复用既有 RX3001,无新码);含义冻结(10 §6,`check_error_codes` 延续),`registry/error_codes.json` 只追加并同时落 [../src/rurixc/src/messages/en.messages](../src/rurixc/src/messages/en.messages)(`shader.stage_io_invalid` / `shader.stage_interface_mismatch` / `shader.resource_handle_invalid` / `shader.sample_expr_invalid`)+ [../src/rurixc/src/messages/zh.messages](../src/rurixc/src/messages/zh.messages) 双语 message-key(`ci/bilingual_coverage.py` 覆盖门)。RX3014 由 RFC-0007 分配(采样表达式类型面)。

## 4. 升档 / 禁区留痕

- **本文件档位 = Full RFC(RFC-0002)**:本设计触 **新语法 + 类型系统扩张**(着色阶段函数着色 / 阶段 I/O type-level 标注 / 阶段间接口契约 / 纹理采样器参数化类型,AGENTS 硬规则 5 / 10 §3 Full RFC 触发面),经 Full RFC 由 agent 自主落笔。RFC-0002 已由 agent 于 2026-06-23 裁决 §9 Q1~Q6 与 §4.5 🔒 禁区边界处置(agent 自主)。**agent 自主判档**,判档争议向上取严。
- **🔒 纹理 / 采样器内存模型映射(06 §4.2 禁区)**:tex proxy / PTX·DXIL 采样 opcode 映射 / 采样器描述符编码 / 纹理缓存一致性 / 采样 UB 边界属内存模型禁区。RXS-0156 的 `Texture2D<F>` / `Sampler` **仅类型面参数化形态**;**采样语义本体首期收敛映射已由 RFC-0007(agent 2026-06-30 自主批准)落笔**——采样表达式类型面见 RXS-0174(本文件),SPIR-V/DXIL 采样 opcode 降级 + 06 §4.2 内存模型条款见 spec/dxil_backend.md RXS-0175/RXS-0176(🔒 禁区子节)。规避子能力(隐式 LOD/导数 / 任意 mip / texel fetch / 比较采样 / 可配置 sampler / 非 fragment 阶段 / 多分量纹理)登记 deferred RD-022~RD-024(RFC-0007 §8),后续 Full RFC 增补,**不一次落全**。触及收敛子集外即停下标注「需升档」或 strict-only 拒(RX3014/RX6023)。
- **G2.2 DXIL codegen(D-131)**:MIR→DXIL 后端、内建变量寄存器/语义槽映射、DXIL 文本 golden → G2.2,不在本文件;触及即停下标注「需升档」。
- **G2.3 绑定布局推导(P-11)**:descriptor / root signature 编译器推导生成 → G2.3,不在本文件;本文件仅定义资源句柄在签名中的类型表达供 G2.3 消费。
- **多后端 / Python 原生嵌入 / device 高级 intrinsics**:分别为 D-008/SG-003、红线 1/SG-008、SG-001/SG-002,均不在本文件着色阶段类型面登记;触及即停下标注「需升档」。
- **UB 节禁区**:着色阶段误用 / 阶段间接口不匹配 / 资源句柄违例以 **编译期类型/着色/接口诊断(P-01 strict-only,无运行期回退)**定义,**严禁 UB 节**(UB 为经 Full RFC 由 agent 自主落笔的高敏面,10 §7.5)。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-23 | 新建 spec/shader_stages.md(G2.1 着色阶段类型面起始文件):登记编号区间 RXS-0153 起续号预留(**已锁定 4 条 RXS-0153 ~ RXS-0156**,RFC-0002 §9 Q5)+ 文件级前言 / 范围(着色阶段函数着色扩展 RXS-0066 / 阶段专属 I/O 语义类型属性式标注 / 阶段间接口类型契约 vertex out→fragment in / 资源句柄·纹理采样器参数化类型面 `Texture2D<F>`+`Sampler` 平行 View;复用 kernel 子语言+views、device⊂host 单向可达、trait 单态化子集、PTX-only、🔒 纹理内存模型映射禁区不落笔)/ 依据与授权(RFC-0002 agent 批准 + 06 §8.2/§4.2 + 05 §1/§2.2 + spec/device.md RXS-0066/0067/0074/0078;G2_CONTRACT D-G2-1/D-G2-6 / G-G2-1/G-G2-6 + G2_PLAN G2.1)/ 计划条款骨架(§2 预留,非裸条款头,照搬 RFC-0002 §5 表:RXS-0153 着色阶段函数着色规则 / RXS-0154 阶段专属 I/O 语义类型 / RXS-0155 阶段间接口类型契约 / RXS-0156 资源句柄·纹理采样器参数化类型面)/ 错误码新段位说明(§3:着色阶段语义诊断归 3xxx 段 RX3011+ 续号,脚手架不预造、不预留;纯 Rust 错误走 rustc 原生零新码)/ 升档·禁区留痕(§4:档位 Full RFC/RFC-0002、🔒 纹理内存模型映射禁区、G2.2 D-131、G2.3 P-11、多后端/红线1/SG、UB 节禁区)。**沿 README v1.32 / v1.33 先例:本轮不落带编号裸条款头**——条款体与每条 ≥1 测试锚定随 G2.1 实现 PR(PR-B2,步骤 45)同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定 152/152),无体例变更 | **Full RFC**(RFC-0002) |
| v1.1 | 2026-06-23 | **G2.1 实现 PR(PR-B2):§2 计划骨架升格为带编号条款体 `### RXS-0153 ~ ### RXS-0156`**(FLS 体例,按需分 Syntax / Legality / Dynamic Semantics / Implementation Requirements 节,**严禁 UB 节**;Legality 引用对应 RX 码):RXS-0153 着色阶段函数着色规则(前缀式 `<stage> fn`,着色阶段取 kernel 入口着色,直接调用入口 / 跨着色非法调用复用 `RX3001`;compute 复用 kernel;device⊂host 单向可达;trait 单态化子集)/ RXS-0154 阶段专属 I/O 语义类型(`#[interpolate(..)]`/`#[builtin(..)]` 属性式,无标注字段编译期拒绝 → `RX3011`)/ RXS-0155 阶段间接口类型契约(vertex out → fragment in varying 兼容 → 不兼容 `RX3012`,网格/RT 并入本条)/ RXS-0156 资源句柄·纹理采样器参数化类型面(`Texture2D<F>`+`Sampler` 仅着色阶段签名形参,返回/字段/非阶段位置或未支持维度 → `RX3013`,纹理仅类型形态无采样/内存语义)。§3 错误码表回填 RX3011~3013(3xxx 段续号,着色阶段误用复用 RX3001;en/zh message-key 同落,bilingual_coverage 覆盖)。配套 rurixc 着色阶段前端(parser 上下文关键字 `<stage> fn` + AST 层着色阶段类型面检查,gate `cargo feature shader-stages`)+ conformance accept/reject(`conformance/shader/`)+ UI golden(`tests/ui/shader/*.stderr`,经 bless)+ 每条 ≥1 `//@ spec: RXS-####` 锚定(trace_matrix 152→156 全锚定)。**仅类型面/语法面 + 编译期拦截**:不碰 DXIL codegen(G2.2)/ 绑定布局推导(G2.3)/ 🔒 纹理内存模型映射(06 §4.2 禁区);区间锁定 4 条不拆条 | **Full RFC**(RFC-0002) |
| v1.2 | 2026-06-30 | **RFC-0007 采样语义本体落库 + RXS-0174 采样表达式类型面条款(spec-first,G2.4 严格面)**。承 RFC-0007(agent 2026-06-30 自主批准,废止 G2_CONTRACT §8.5 选项 B「不采样」折中、关闭 RD-021):把 RXS-0156 的 opaque 资源句柄类型面升级为**可在着色 body 求值的采样表达式**类型面。新增 `### RXS-0174`(采样表达式类型面,**超出 G2.1/RFC-0002 锁定的 RXS-0153~0156 区间,独立 RFC 续号**):`tex.sample(samp, coord)` 复用 `MethodCall` 产生式(无新 token);合法当且仅当 `tex : Texture2D<F>`、`samp : Sampler`、`coord : vec2<f32>` 且包含函数为 `fragment` 阶段;结果 `vec4<F>`;首期收敛子集(显式 LOD 0,规避隐式导数;receiver/`samp` 须直接句柄形参引用),违例 `RX3014`;规避项登记 RD-022~RD-024。§3 错误码表回填 RX3014(3xxx 段续号,`shader.sample_expr_invalid` en/zh message-key)。§4 🔒 纹理内存模型映射禁区留痕更新:采样语义本体首期收敛映射已由 RFC-0007 落笔(类型面 RXS-0174 本文件 + codegen/内存模型 RXS-0175/0176 @ dxil_backend.md),不再「占位待后续」。**条款先于实现**(硬规则 7),测试锚定随实现 commit 同落(trace_matrix 维持全锚定)。🔒 完整内存模型映射(采样 opcode/坐标/LOD/寻址/越界/缓存可见性·memory-order)落 dxil_backend.md RXS-0176 | **Full RFC**(RFC-0007) |
