# Rurix 语言规范 — 着色阶段类型面（vertex / fragment / compute / mesh / task / RT 着色阶段作为 kernel 着色扩展；G2.1 起）

> 条款:RXS-0153 起续号预留(G2.1 着色阶段类型面:着色阶段函数着色规则（vertex/fragment/compute/mesh/task + RT raygen/closesthit/anyhit/miss 作为新 coloring，扩展 RXS-0066）/ 阶段专属 I/O 语义类型（插值限定 type-level + 内建变量类型化）/ 阶段间接口类型契约（vertex out → fragment in 兼容性编译期校验）/ 资源句柄·纹理采样器参数化类型的类型面（`Texture2D<F>`/`Sampler` 类型形态，平行 `View<space,T>`）)。体例见 [README.md](README.md)。
> 依据:**[RFC-0002](../rfcs/0002-shader-stages.md)**(着色阶段进语言的类型面 vertex/fragment/compute/mesh/task/RT 作为 kernel 着色扩展,**owner 已批准定稿**,2026-06-23);06 §8.2(着色阶段 = kernel 着色扩展,设计预留);06 §4.2(纹理路径内存模型禁区,🔒);05 §1(device⊂host 单向可达,kernel 子语言受限子集);05 §2.2(trait 单态化子集 D-104,无 dyn/特化/HKT/async);spec/device.md:RXS-0066(函数着色与跨着色调用合法性)/ RXS-0067(地址空间类型与一致性 `View<space,T>`)/ RXS-0074(launch 类型契约)/ RXS-0078(views 算子集)。授权:[../milestones/g2/G2_CONTRACT.md](../milestones/g2/G2_CONTRACT.md)(`in_scope: shader_stages_in_lang` / `spec_g2_clauses`,D-G2-1 / D-G2-6,G-G2-1 / G-G2-6)+ [../milestones/g2/G2_PLAN.md](../milestones/g2/G2_PLAN.md) G2.1 首子里程碑。
> 档位:**Full RFC**(RFC-0002;10 §3:本设计触 **新语法 + 类型系统扩张**——着色阶段函数着色、阶段 I/O type-level 标注、阶段间接口契约、纹理采样器参数化类型,AGENTS 硬规则 5 / 10 §3 Full RFC 触发面,只能人类经 Full RFC 落笔)。RFC-0002 已由 owner 于 2026-06-23 在工作会话明确裁决 §9 Q1~Q6 与 §4.5 🔒 禁区边界处置(代录,非 AI 代签);spec 条款 PR 与实现 PR 均门控于 RFC-0002 合入之后(本脚手架门控于 main 已含 RFC-0002 之后)。**AI 无权自判 Direct**,判档以 RFC-0002 与 G2_CONTRACT 授权为据,判档争议向上取严。任何偏离 RFC-0002 已批准设计、或触及 **G2.2 DXIL codegen(D-131)** / **G2.3 绑定布局推导(P-11)** / **🔒 纹理路径内存模型映射(06 §4.2)** / **多后端(D-008/SG-003)** / **Python 原生嵌入(红线 1,SG-008)** 的条款,必须停下标注「需人工升档」,不在本文件自行落笔。**严禁 UB 节**(10 §7.5):着色阶段误用 / 阶段间接口不匹配 / 资源句柄违例以 **编译期类型/着色/接口诊断(P-01 strict-only,无运行期回退)**定义,不以 UB 表述(RFC-0002 §3/§4)。
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

## 2. 条款（计划骨架 — 非裸条款头，随实现 PR 落地带编号条款体）

> 沿 README v1.32 / v1.33 先例,本脚手架**不落 `### RXS-####` 裸条款头**(避免未锚定条款,trace_matrix 维持全锚定 152/152);下列为计划骨架(照搬 RFC-0002 §5 条款与测试锚定计划表),实现 PR(PR-B2,步骤 45)落地带编号条款体 + 每条 ≥1 `//@ spec: RXS-####` 测试锚定。每条按需分 Syntax / Legality / Dynamic Semantics / Implementation Requirements 节,**严禁 UB 节**。

| 条款(计划) | 标题 | 测试锚定计划(每条 ≥1) | RFC-0002 来源 |
|---|---|---|---|
| RXS-0153 | 着色阶段函数着色规则(vertex/fragment/compute/mesh/task + RT raygen/closesthit/anyhit/miss 作为新 coloring,扩展 RXS-0066) | conformance accept(合法着色阶段声明 0 诊断)+ reject(着色阶段误用 / 跨着色非法调用)+ UI golden | §4.1 |
| RXS-0154 | 阶段专属 I/O 语义类型(插值限定 type-level + 内建变量类型化) | conformance accept(合法 I/O 标注)+ reject(内建变量类型/阶段错配)+ UI golden | §4.2 |
| RXS-0155 | 阶段间接口类型契约(vertex out → fragment in 兼容性编译期校验) | conformance reject(接口类型不匹配)+ accept(兼容接口)+ UI golden | §4.3 |
| RXS-0156 | 资源句柄 / 纹理采样器参数化类型的类型面(`Texture2D<F>` / `Sampler` 类型形态,平行 `View<space,T>`) | conformance accept(合法句柄签名)+ reject(句柄违例 / 非法位置)+ UI golden | §4.4 |

> 区间已锁定为 4 条 `RXS-0153 ~ RXS-0156`(RFC-0002 §9 Q5,owner 2026-06-23):网格/RT 阶段间接口(payload/attribute)并入 RXS-0155、纹理类型集合并入 RXS-0156,本里程碑**不拆条、不预留、不预造**。条款数与区间不随实现 PR 漂移(若确需调整须经 owner Full RFC 复议,README §4 区间与本节同步更新,修订行留痕)。

## 3. 错误码引用汇总（新段位说明 — 脚手架不预造）

> 三类编译期拦截(着色阶段误用 / 阶段间接口不匹配 / 资源句柄违例)属 **Rurix 语义诊断**(编译期可检的着色/接口/句柄合法性,对齐 RXS-0066 着色诊断先例),归 **3xxx 着色/地址空间段位续号**(07 §5 语义分配;当前该段末号 **RX3010**,下一可用 **RX3011+**——**非全局 7xxx 段**,7xxx 为运行期/互操作段)。纯 Rust 通用错误(类型不符等)走 rustc 原生诊断(零新 RX)。

**不预留、不预造**:RX3011+ 随实现 PR(PR-B2)按**实现中真实可达、用户可行动**的错误类别**只追加**分配(脚手架不预留号码、不预造码数);含义冻结(10 §6,`check_error_codes` 延续),`registry/error_codes.json` 只追加并同时落 [../src/rurixc/src/messages/en.messages](../src/rurixc/src/messages/en.messages) + [../src/rurixc/src/messages/zh.messages](../src/rurixc/src/messages/zh.messages) 双语 message-key(`ci/bilingual_coverage.py` 覆盖门)。本表随实现 PR 落地回填。

## 4. 升档 / 禁区留痕

- **本文件档位 = Full RFC(RFC-0002)**:本设计触 **新语法 + 类型系统扩张**(着色阶段函数着色 / 阶段 I/O type-level 标注 / 阶段间接口契约 / 纹理采样器参数化类型,AGENTS 硬规则 5 / 10 §3 Full RFC 触发面),只能人类经 Full RFC 落笔。RFC-0002 已由 owner 于 2026-06-23 裁决 §9 Q1~Q6 与 §4.5 🔒 禁区边界处置(代录,非 AI 代签)。**AI 不自判 Direct**,判档争议向上取严。
- **🔒 纹理 / 采样器内存模型映射(06 §4.2 禁区)**:tex proxy / PTX·DXIL 采样 opcode 映射 / 采样器描述符编码 / 纹理缓存一致性 / 采样 UB 边界属内存模型禁区,本文件 `Texture2D<F>` / `Sampler` **仅类型面参数化形态**,不承诺任何采样语义、内存序或一致性保证;内存模型映射条款留**后续独立 Full RFC**(owner 落笔,RFC-0002 §4.5 / §9 Q6 维持占位「〈待 owner 后续 Full RFC〉」)。触及即停下标注「需人工升档」。
- **G2.2 DXIL codegen(D-131)**:MIR→DXIL 后端、内建变量寄存器/语义槽映射、DXIL 文本 golden → G2.2,不在本文件;触及即停下标注「需人工升档」。
- **G2.3 绑定布局推导(P-11)**:descriptor / root signature 编译器推导生成 → G2.3,不在本文件;本文件仅定义资源句柄在签名中的类型表达供 G2.3 消费。
- **多后端 / Python 原生嵌入 / device 高级 intrinsics**:分别为 D-008/SG-003、红线 1/SG-008、SG-001/SG-002,均不在本文件着色阶段类型面登记;触及即停下标注「需人工升档」。
- **UB 节禁区**:着色阶段误用 / 阶段间接口不匹配 / 资源句柄违例以 **编译期类型/着色/接口诊断(P-01 strict-only,无运行期回退)**定义,**严禁 UB 节**(UB 为人类经 Full RFC 落笔的禁区,10 §7.5)。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-23 | 新建 spec/shader_stages.md(G2.1 着色阶段类型面起始文件):登记编号区间 RXS-0153 起续号预留(**已锁定 4 条 RXS-0153 ~ RXS-0156**,RFC-0002 §9 Q5)+ 文件级前言 / 范围(着色阶段函数着色扩展 RXS-0066 / 阶段专属 I/O 语义类型属性式标注 / 阶段间接口类型契约 vertex out→fragment in / 资源句柄·纹理采样器参数化类型面 `Texture2D<F>`+`Sampler` 平行 View;复用 kernel 子语言+views、device⊂host 单向可达、trait 单态化子集、PTX-only、🔒 纹理内存模型映射禁区不落笔)/ 依据与授权(RFC-0002 owner 批准 + 06 §8.2/§4.2 + 05 §1/§2.2 + spec/device.md RXS-0066/0067/0074/0078;G2_CONTRACT D-G2-1/D-G2-6 / G-G2-1/G-G2-6 + G2_PLAN G2.1)/ 计划条款骨架(§2 预留,非裸条款头,照搬 RFC-0002 §5 表:RXS-0153 着色阶段函数着色规则 / RXS-0154 阶段专属 I/O 语义类型 / RXS-0155 阶段间接口类型契约 / RXS-0156 资源句柄·纹理采样器参数化类型面)/ 错误码新段位说明(§3:着色阶段语义诊断归 3xxx 段 RX3011+ 续号,脚手架不预造、不预留;纯 Rust 错误走 rustc 原生零新码)/ 升档·禁区留痕(§4:档位 Full RFC/RFC-0002、🔒 纹理内存模型映射禁区、G2.2 D-131、G2.3 P-11、多后端/红线1/SG、UB 节禁区)。**沿 README v1.32 / v1.33 先例:本轮不落带编号裸条款头**——条款体与每条 ≥1 测试锚定随 G2.1 实现 PR(PR-B2,步骤 45)同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定 152/152),无体例变更 | **Full RFC**(RFC-0002) |
