# DXIL compute 路径能力缺口判档意见书（GRX-013/014 三组阻断 → Mini-RFC/Full RFC 分档）

> 性质：**判档材料 + 草案随附**（零实现、零 registry 改动、零既有条款文件改动）。
> 出处锚点：`spike/godot-rurix/passes/cluster_store/PASS_CONTRACT.md` §5.3（三重阻断段）·
> `spike/godot-rurix/passes/particles_copy/PASS_CONTRACT.md` §5.3（两重阻断段）·
> `src/rurixc/src/dxil_codegen.rs`（HEAD `d9c438b`：`require_view_global_f32` ≈L1741、
> 二元运算白名单 ≈L1282-1289、`LoweredScalarTy{F32,I64,Bool}` ≈L752）·
> `src/rurixc/src/ast.rs` L577-581（`BitAnd/BitOr/BitXor/Shl/Shr` 已在 AST）。
> 判档基准：`10_GOVERNANCE.md` §3（三档门 D-402）· `spec/README.md` §3 ·
> `rfcs/README.md` §1/§5 · 先例 MR-0001/MR-0005（Mini 携新 RXS 条款）·
> RXS-0171（升档触发先例：显式布局触及即升 Full RFC）· RFC-0007（禁区面须 Full 先例）。
> Provenance：`Assisted-by: claude-code:claude-fable-5`。本文件为草案判档意见，
> 批准/合入/编号 reconcile 归 agent 后续流程；**不 git commit、不改 registry**。

---

## 0. 判档结论表（摘要）

| # | 缺口 | 语言面现状（实读核对） | 判档 | 载体 | 拟落条款 |
|---|---|---|---|---|---|
| A | 整型 raw buffer 视图 `View/ViewMut<global, u32\|i32>` | 类型面**已存在**：`View<space,T>` 元素类型泛化（device.md RXS-0066/0067/0071 口径「元素类型取 T」）；`PrimTy::U32/I32` 已在 typeck（`typeck.rs` L1935/1939）；仅 DXIL compute body lowering 以 `require_view_global_f32` 收窄到 f32（RX6007 strict 拒绝） | **Mini-RFC** | MR-0006 | RXS-0181 |
| B1 | 整型位运算符 DXIL 降级（`& \| ^ << >>`） | 语言语义**已存在**：types.md「算术与位运算」条款（位运算仅整数，违例 RX2006）；AST/MIR 已建模（ast.rs L577-581 / mir.rs L643-647，PASS_CONTRACT 引证）；仅 DXIL 白名单只 `+ - * / %`（dxil_codegen.rs L1282-1289） | **Mini-RFC** | MR-0006 | RXS-0182 |
| B2 | 位扫描/位计数 intrinsic（findLSB/findMSB/popcount） | 语言面**不存在**（任何后端均无，cluster_store PASS_CONTRACT §5.3 第 3 条）；为 RXS-0081 device 数学 intrinsic 集的整数位域镜像扩展，纯值运算、全定义、零内存模型面 | **Mini-RFC**（带升档触发） | MR-0006 | RXS-0183 |
| C1 | DeviceMathFn（sqrt/rsqrt/sin/cos）DXIL 降级 | 语言语义**已存在**（device.md RXS-0081/0082，f32/f64 浮点 intrinsic 集 + RX6006）；仅 NVPTX libdevice 有 lowering，DXIL 侧零处理（particles_copy PASS_CONTRACT §5.3 第 2 条） | **Mini-RFC** | MR-0007 | RXS-0184 |
| C2 | 聚合 buffer 元素类型（struct/vec4/mat4 SSBO 元素 + 布局规则） | 类型面不存在（视图元素仅标量）；**必然触及显式字节布局**（std430 式 offset/stride = 二进制布局面） | **Full RFC**（本轮不起草，登记 deferred 预览 RD-026） | 候选 RFC-0009 | —（随 Full RFC 定） |

分立/合并裁量：A + B1 + B2 **合并一份 MR-0006**（同一动机 GRX-014 cluster_store 整型
kernel 原生化；同一实现 PR 面 `dxil_codegen` compute 子集扩展；失败测试同族）；
C1 **独立 MR-0007**（动机独立于整型包 —— GRX-013 particles ALIGN_BILLBOARD；风险轴独立
—— DXIL 浮点超越函数 opcode 覆盖 + ULP 精度对照，可独立落地/独立回退）；C2 **不并入
任何 Mini**（升档论证见 §3.5）。

## 1. 背景与 GRX 侧动机

1. **rurix_owned 偏好**：`spike/godot-rurix/passes/luminance_reduction/
   texture_artifact_provenance_policy.json` 确立 `provenance=hlsl_bridge_workaround`
   为 owner 批准的**临时**例外，`rurix_owned=false` 且带四条 revert-to-rurix_owned
   条件。缺口消除是把 provenance 翻回 `rurix_owned=true` 的前置。
2. **可退役的 workaround 实例**：GRX-013 particles_copy 与 GRX-014 cluster_store
   两个 pass 的 offline kernel 目前只能走 hlsl_bridge（各自 PASS_CONTRACT §5.3 判定
   rurixc-native infeasible）。A+B 消除后 cluster_store 全 kernel 原生可表达；
   A+B+C1 消除后 particles_copy 除聚合元素外原生可表达（聚合仍卡 C2，见 §7 O-5）。
3. **在途 pass 的直接受益**：GRX-015 GPU culling / GRX-016 visible instance
   compaction / GRX-018 indirect draw argument generation（GRX_PLAN §6）全部是
   SSBO + 位运算/整型计数型 kernel。A+B 先行落地可让它们**直接原生**，不再各自
   复制 hlsl_bridge 例外。
4. **GRX-009 先例 = 最近一次同类能力扩展的完整形态**（本判档的形态基准）：
   texture compute lowering 扩展以「上游 intrinsic 拼写（`target("dx.Texture",…)` +
   `llvm.dx.resource.load.level`）+ conformance accept/reject（锚 `//@ spec: RXS-0157`）
   + tests/dxil 两层 golden（`.dxil-ll` + dxv 接受后 `.dxil-disasm`）+ bless_log 追加」
   收口（bless_log 2026-07-11 行）；上游缺 store intrinsic 时走 RD-025 受控本地
   llc patch 纪律。本件 A/B/C1 的实现形态全部镜像此先例。

## 2. 缺口事实核对（实读，可复核）

- **A**：`dxil_codegen.rs::require_view_global_f32`（≈L1741）对非 `<global, f32>`
  视图报「DXIL compute body lowering slice 1 要求 View/ViewMut 带 <global, f32>」；
  `ComputeParamKind` 仅 `ViewF32/ViewMutF32/Texture2DF32/…`（≈L759）。语言层
  `View<space,T>` 本就元素类型泛化（device.md RXS-0067「元素类型可合一」、RXS-0071
  「元素类型取 T」），u32/i32 primitive 已在（typeck.rs `PrimTy::U32/I32`）。
  故 A 是**后端降级覆盖收窄**，非类型系统缺口。
- **B1**：`dxil_codegen.rs` ≈L1282-1289 白名单
  `BinOp::Add|Sub|Mul|Div|Rem`，其余（含全部位运算）报「slice 3a 仅支持标量
  + - * / 与简单比较」。types.md 已定「位运算 `& | ^ << >>` 仅整数、同型、结果
  同型，违例 RX2006」。NVPTX 路径已降级这些 MIR BinOp。
- **B2**：全仓无 findLSB/findMSB/popcount intrinsic（任何后端）；cluster_store
  kernel 的 word-scan 循环与 z_range 解码依赖 `firstbitlow/firstbithigh`
  （PASS_CONTRACT §5.3 第 3 条 + §「kernel 结构」L92-96）。
- **C1**：`DeviceMathFn` 建模贯穿 `typeck/hir/tbir/mir_build`，lowering 仅在
  NVPTX（libdevice `__nv_*` + `-mlink-builtin-bitcode`）；`dxil_codegen.rs` 零处理。
- **C2**：`LoweredScalarTy{F32,I64,Bool}`（≈L752）——DXIL compute 值域只有标量；
  `ParticleData = mat4 + vec3 + uint + vec4 + vec4`（stride 112B，column-major
  mat4）无法以现有视图元素表达。

## 3. 判档分析

### 3.1 判档基准复述

10 §3 三档门：Full RFC 触发面 =「新语法 / 类型系统变更 / 运行时语义 / unsafe 边界 /
FFI ABI / 稳定化 / edition / 设计原则修改 / 死亡路线触碰」；Mini-RFC =「规范内
bug fix、诊断措辞策略、内部开关、工具行为变更」+ 失败测试先行。先例扩展了 Mini 的
实务口径：MR-0005（fatbin）以 Mini 携**新 RXS 条款**（RXS-0150~0152）落「既定决策的
工程实现 + 执行期新决策面」，论证核心是「复用既有语义面 0-byte + 不触禁区（UB /
内存模型映射 / FFI ABI / 安全包络）+ 升档触发条件」。DXIL 侧的禁区红线由
spec/dxil_backend.md 头注 + RXS-0171 定型：「资源/纹理/采样/**显式布局**触及即升
agent Full RFC」；纹理内存模型（06 §4.2）走过 Full（RFC-0007）。

### 3.2 A（整型视图）→ Mini-RFC

- **非 Full**：零新语法（`View<global, u32>` 本就是合法类型拼写，syntax.md L230
  口径）；零类型系统变更（元素类型泛化与合一规则既有，RXS-0067）；零运行时语义
  （device 视图索引读写语义既有，RXS-0071/0078 口径，元素类型换 T 不换语义）；
  零内存模型/布局面（标量元素、天然对齐、无 offset/stride 新规则——u32/i32 与
  f32 同宽同对齐）；零 unsafe/FFI ABI。纯**后端降级覆盖扩展** + strict 边界收窄
  （RX6007 拒绝面缩小，接受面扩大，向后兼容纯追加）。
- **非 Direct**：接受面扩大 = 语言可观测行为变化（原 RX6007 拒绝的程序开始编译
  通过）+「上游 rawbuffer intrinsic 对 i32 元素的支持面」是执行期新决策面（须
  probe 实证，镜像 GRX-009 texture 先例），判档争议向上取严 → Mini。
- **上游支持面论证义务**（MR-0006 §2 承载）：现行 f32 A 路已在产
  `@llvm.dx.resource.handlefrombinding` + `@llvm.dx.resource.load.rawbuffer` /
  `store.rawbuffer`（tests/dxil/bless_log.md 2026-07-03 行）；该 intrinsic 族按
  元素类型重载（`.f32`/`.i32` 后缀），下游 DXIL op `rawBufferLoad(139)` /
  `rawBufferStore(140)` 同为元素类型重载（dxc 对 `StructuredBuffer<uint>` /
  `ByteAddressBuffer` 即产 i32 重载，存在性证明）。**但 pinned llc（23.0git +
  RD-025 本地 patch）对 i32 重载的 emit 稳定性 + dxv 接受须 probe 实测**（×8
  字节稳定 + `Validation succeeded`，镜像 cs_texture 收口纪律），实测前不落 codegen。

### 3.3 B1（位运算符）→ Mini-RFC；B2（位扫描 intrinsic）→ Mini-RFC（带升档触发）

- **B1 非 Full**：语言语义既有（types.md 位运算条款 + RX2006），MIR 已建模，
  NVPTX 已降级；DXIL 降级映射为**平凡 LLVM 指令**（`and/or/xor/shl/lshr|ashr`），
  不经任何 dx intrinsic，无上游缺口轴。唯一需语言层显式定义的是**移位量越界
  语义**（LLVM `shl` 越界 = poison；DXIL 语义按位宽掩码；PTX 语义钳制）——为
  避免 UB 表述与后端分歧，MR-0006 拟定「移位量按位宽取模（显式掩码），两后端
  emit 显式 mask」，全定义、无 UB 节（P-01），见 §7 O-2 owner 裁决点。
- **B2 是本包唯一的语言面新增**（新 intrinsic 函数）。向上取严逐面核对 Full
  触发：非新语法（intrinsic 为路径调用，形态同 RXS-0081 数学函数集）；非类型
  系统变更（普通函数类型 `u32 -> u32`）；非运行时语义禁区（纯值运算，总函数——
  零输入显式定义，无内存/并发/可见性面）；非 FFI ABI/unsafe。形态精确镜像
  RXS-0081「device 数学函数 intrinsic 集」既有条款（该集当年随里程碑条款化落地，
  未走 Full）。故裁 Mini，**升档触发**：若实现期发现须定义任何内存序/跨线程
  可见性/资源访问耦合语义（如 wave/subgroup 变体），停手升 Full。
- **DXIL lowering 依赖面（诚实边界）**：`llvm.cttz/ctlz/ctpop → dx.op
  Countbits(31)/FirstbitLo(32)/FirstbitHi(33)/FirstbitSHi(34)` 的上游 DirectX
  后端覆盖须 probe 实测；若上游缺 lowering，走 RD-025 式受控本地 patch 纪律
  （或停手留痕），不自造拼写（GRX-009 教训：自造拼写任何 llc 均拒）。

### 3.4 C1（数学 intrinsic DXIL 降级）→ Mini-RFC

语言语义既有（RXS-0081/0082：签名契约、元数、RX6006 保守拒绝）；本件仅为第二
后端补 lowering 覆盖（`llvm.sqrt/sin/cos → dx.op.unary`，f32 首期；f64 与覆盖外
函数维持 RX6006/RX6007 strict 拒绝）。精度以 Implementation Requirements 引 D3D
功能规范 ULP 界 + host 参考对照 harness（镜像 GRX math parity evidence 纪律）表述，
不发明新数值语义。与 B1 同理非 Direct（接受面扩大 + DXIL opcode 覆盖为执行期
新决策面）。独立成 MR-0007 的理由见 §0。

### 3.5 C2（聚合元素类型）→ Full RFC（本轮只登记 deferred 预览）

- 触发面一：**类型系统变更**——视图元素从标量集扩到 struct/vec4/mat4 聚合，
  需新类型规则（哪些聚合可作元素、字段访问经视图的 place 语义、mat 列序）。
- 触发面二：**显式字节布局**——std430 式 offset/stride/对齐规则是二进制布局
  承诺（`ParticleData` stride=112、mat4 column-major 与 Godot ABI 互操作），
  正中 RXS-0171 升档触发原文「资源/纹理/采样/显式布局触及即升 agent Full RFC」
  与 RFC-0003 §4.6 FFI ABI 二进制布局禁区的邻接面。跨语言（Godot GLSL std430 ↔
  Rurix 元素布局）互操作承诺一旦写下即冻结，须 Full RFC 留档 + conformance +
  stabilization 路径。
- **过渡路径**（不必等 C2）：A+B 落地后，聚合可用标量分解表达——`View<global,
  u32>` + 手工 stride 索引 + `bitcast`（asuint/asfloat 式位重解释，DXIL/NVPTX
  均为平凡 bitcast）。bitcast intrinsic 是否随 MR-0006 一并入册 = §7 O-4。
- 本轮交付：deferred 预览条目 **RD-026**（§6），正式登记随立项/实现 PR。

## 4. 与 GRX-022 bindless RFC 的边界（不重叠论证）

GRX-022（GRX_PLAN §7）=「bindless/resource-array 扩展」，其 compiler 语义面对应
RD-018（RFC-0005 §9 Q-Bindless defer）：**descriptor 维度**——unbounded descriptor
array、descriptor heap 直索引（SM6.6+）、资源数组上界与越界语义、绑定模型扩展。
本件三组缺口全部在**已绑定单资源内部**：元素标量类型（A）、ALU 运算覆盖（B/C1）、
元素聚合形状（C2）。正交性逐面：

1. 绑定布局推导条款 RXS-0163~0166 **0-byte**：u32/i32 视图与 f32 视图同种类轴
   （SRV `t#` / UAV `u#`）、同 register-space 分配、同 RTS0 形态；不增资源数、
   不变 descriptor 编码。
2. 本件不引入资源数组、不引入 heap 直索引、不触 RD-018 backfill 条件。
3. 唯一接缝：rurixc raw-buffer 视图（ByteAddressBuffer 形态描述符）与 GRX shim
   per-slot binding-kind gate 期望的 `structured_buffer`（stride 型描述符）之间的
   D3D12 SRV/UAV 视图种类对齐——这是 **pass/shim 侧描述符创建问题**，不进语言
   绑定模型，与 bindless 无关（§7 O-3）。

结论：MR-0006/0007 与 GRX-022/RD-018 无范围重叠；GRX-022 若立项引用本件为
「元素类型/ALU 覆盖前置」，反向不成立。

## 5. 续号核对（全部实读确认，HEAD `d9c438b`）

| 命名空间 | 已用最高 | 下一可用 | 核对方式 |
|---|---|---|---|
| Mini-RFC `MR-####` | MR-0005（`rfcs/mini-0005-fatbin-distribution.md`） | **MR-0006** | rfcs/ 目录实列 + 全仓 grep 无 MR-0006/mini-0006 文件；G2_CONTEXT §「Mini-RFC \| mini-0005 \| mini-0006+」佐证。**注意**：`rfcs/README.md` §5 台账停在「下一个未用 MR-0005」= 台账滞后（MR-0005 已落档未回填），批准 PR 须一并补台账行（本轮不改该文件） |
| Full RFC `RFC-####` | RFC-0008 | **RFC-0009**（C2 候选） | rfcs/README §5 + G2_CONTRACT L262「下一未用 RFC-0009」 |
| spec 条款 `RXS-####` | RXS-0180（edition.md） | **RXS-0181**（MR-0006 拟占 0181~0183，MR-0007 拟占 0184） | spec/README §4 全表 + 全仓 grep RXS-018x/019x 仅 0180 命中 |
| deferred `RD-###` | RD-025 | **RD-026**（C2 预览） | registry/deferred.json 实读（RD-016 已跳号永不复用，RD-022~025 在册） |
| 错误码 `RX####` | 6xxx 段最高 RX6023 | 零新码（备查 RX6024） | registry/error_codes.json 实读；本件复用 RX6007/RX6006/RX2006/RX2001 |

## 6. registry/deferred.json 预览 diff（**不实际改动**，正式登记随立项/实现 PR）

```diff
--- a/registry/deferred.json
+++ b/registry/deferred.json
@@ entries 数组末尾（RD-025 之后）追加 @@
+    {
+      "id": "RD-026",
+      "title": "DXIL compute 聚合 buffer 元素类型(struct/vec4/mat4 SSBO 元素 + 显式字节布局规则)——须 Full RFC(候选 RFC-0009)",
+      "reason": "GRX-013 particles_copy 的 ParticleData(mat4+vec3+uint+vec4+vec4,stride 112B,column-major)等聚合 SSBO 元素无法以现有标量视图元素表达(dxil_codegen LoweredScalarTy 仅 F32/I64/Bool;require_view_global_f32 收窄)。聚合元素必然触及显式字节布局(std430 式 offset/stride/对齐 = 二进制布局承诺,跨 Godot GLSL std430 互操作),正中 RXS-0171 升档触发(显式布局触及即升 Full RFC)与 RFC-0003 §4.6 二进制布局禁区邻接面,且需视图元素类型规则扩展(类型系统面),Mini-RFC 载不动;不永久/条件裁剪该方向",
+      "backfill_condition": "GRX-013 退役 hlsl_bridge_workaround 走 rurix_owned、或后续 GPU-driven pass 出现聚合元素硬需求时,经 Full RFC(候选 RFC-0009)落笔:聚合元素类型规则 + 显式布局规则(offset/stride/对齐,含 mat 列序)+ conformance accept/reject + DXIL golden + device 证据;过渡期可用 MR-0006 标量分解(u32/f32 视图 + 手工 stride + bitcast)表达,聚合元素构造维持 RX6007 strict 拒绝",
+      "owner_milestone": "GRX",
+      "status": "open",
+      "history": [
+        { "date": "<立项日>", "event": "<立项/实现 PR 执行期登记留痕>", "evidence": "spike/dxil-path-probe/dxil_compute_capability_gaps_adjudication_20260712.md / spike/godot-rurix/passes/particles_copy/PASS_CONTRACT.md §5.3 / spike/godot-rurix/passes/cluster_store/PASS_CONTRACT.md §5.3" }
+      ]
+    }
```

（`revision_log` 相应追加一行 vN 版次；本预览不代拟 revision 措辞——deferred.json
「本文件对 AI 只追加」纪律 + 换行/LF 细节由落地 PR 按 g2.2 教训逐字节核对。）

## 7. 开放问题清单（owner / agent 批准流程裁决点）

| # | 问题 | 草案预设（可推翻） |
|---|---|---|
| O-1 | 位扫描零输入语义：HLSL 形（u32 全一 0xFFFFFFFF）还是 GLSL 形（i32 -1）？ | HLSL 形：`find_lsb(0u)==0xFFFF_FFFFu`（u32→u32 闭合，免符号扩展歧义） |
| O-2 | 移位量越界语义：按位宽取模（DXIL/掩码）vs 钳制（PTX 原生）vs 编译期拒绝？host/consteval 是否同步定义？ | 语言层定义「取模（amount & (width-1)）」，两 device 后端 emit 显式 mask；host/consteval 对齐面须实现期核对，分歧即停手 |
| O-3 | rurixc u32 视图绑定形态：raw buffer（ByteAddressBuffer 描述符,现行 rawbuffer intrinsic 形态）vs structured buffer（stride 描述符,GRX shim binding-kind gate 现对 cluster_store 期望 `structured_buffer`）？ | 语言侧维持 rawbuffer intrinsic 形态；shim/pass 侧描述符与 gate 对齐为 GRX 集成项，不进语言（若须 structured 形态支持则回本表重裁） |
| O-4 | `bitcast`（asuint/asfloat 式 f32↔u32 位重解释）是否随 MR-0006 一并入册（C2 过渡路径所需）？ | 随 MR-0006 入册（纯位重解释、总函数、两后端平凡 bitcast、零布局面）；若裁剪则 C2 过渡路径受限 |
| O-5 | C2 聚合元素 Full RFC 立项时机：作为 GRX-013 退役 bridge 的必要条件立项,还是接受 GRX-013 长期 hlsl_bridge？ | 暂不立项，登记 RD-026 挂 backfill 条件；GRX-015/016/018 若实测无聚合硬需求则维持 defer |
| O-6 | 上游 DirectX 后端对 `llvm.cttz/ctlz/ctpop` 与 `llvm.sin/cos/sqrt` 的 lowering 覆盖若 probe 实测缺失：走 RD-025 式本地 llc patch 纪律,还是停手等上游？ | 先 probe 定量（每 op ×8 字节稳定 + dxv 接受）；单 op 级缺口走 RD-025 纪律并各自留 upstream issue 链接（10 §8/rfcs/README §4），整面缺失则停手留痕 |
| O-7 | findMSB 的 DXIL 规范化（dxc 对 firstbithigh 产 `31 - FirstbitHi` 形正规化）是否作为语言承诺写进条款,还是仅 golden 锚定？ | 条款只承诺 LSB=0 位序的语言语义；DXIL 侧正规化形态经 golden + 与 dxc 产物 parity 对照锚定，不写死 opcode 组合 |

## 8. 交付物清单（本轮，全部新建文件、零实现）

| 文件 | 内容 |
|---|---|
| `spike/dxil-path-probe/dxil_compute_capability_gaps_adjudication_20260712.md` | 本判档意见书（判档结论表 / GRX-022 边界 / 续号核对 / RD-026 预览 diff / 开放问题） |
| `rfcs/mini-0006-dxil-compute-integer-capability.md` | Mini-RFC 草案（Draft）：缺口 A + B1 + B2，拟落 RXS-0181~0183 |
| `rfcs/mini-0007-dxil-compute-math-intrinsics.md` | Mini-RFC 草案（Draft）：缺口 C1，拟落 RXS-0184 |

不在本轮：spec/ 条款正文落地（随批准后的条款 PR）、registry 任何改动（预览仅
§6）、`rfcs/README.md` §5 台账回填（随批准 PR，连带补 MR-0005 滞后行）、一切
实现/probe/golden/conformance 文件创建（随实现 PR，失败测试先行按各 MR §5 执行）。
