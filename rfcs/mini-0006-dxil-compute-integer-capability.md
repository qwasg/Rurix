# Mini-RFC MR-0006 — DXIL compute 整型能力包（整型 raw buffer 视图 + 位运算降级 + 位扫描/位计数 intrinsic）

| 字段 | 值 |
|---|---|
| Mini-RFC 标识 | **MR-0006**（Mini-RFC 序列；独立于 Full-RFC 的 `RFC-####` 命名空间，不复用 RFC 编号，10 §9.5。Mini-RFC = 单页提案 + 失败测试先行，10 §3。编号经实读核对取下一未用：rfcs/ 目录最高 mini-0005 + 全仓 grep 无 MR-0006 + G2_CONTEXT「mini-0006+」；`rfcs/README.md` §5 台账滞后于 MR-0005，批准 PR 一并补台账） |
| 标题 | DXIL compute 路径整型能力包：`View/ViewMut<global, u32\|i32>` 元素视图降级 + 整型位运算（`& \| ^ << >>`）DXIL 降级 + 位扫描/位计数 intrinsic（`find_lsb`/`find_msb`/`popcount`，双后端）+（O-4 待裁）`bitcast` 位重解释 |
| 档位 | **Mini-RFC**（10 §3：语言类型面/位运算语义**既有 0-byte**（View 元素类型泛化 RXS-0066/0067/0071；位运算 types.md 条款 + RX2006；AST/MIR 已建模）+ DXIL 后端降级覆盖扩展 + RXS-0081 镜像式 intrinsic 集扩充；**不触** UB / 内存模型映射（06 §4.2）/ FFI ABI 二进制布局（RFC-0003 §4.6）/ 显式布局（RXS-0171 升档触发）/ 安全包络禁区——见 §3。判档意见书：`spike/dxil-path-probe/dxil_compute_capability_gaps_adjudication_20260712.md`） |
| 状态 | **Approved — 2026-07-12**（agent 完全自主批准，承 owner 已签署的 agent 完全自主化治理基线（b17fd67）与 g2.6 Mini-RFC 自主签署先例；判档 O-1~O-7 裁决落档 + 执行期实测修正留痕见 §7；`rfcs/README.md` §5 台账回填待后续 PR——台账文件不在本轮改动面） |
| 承接里程碑 | GRX（GRX-014 cluster_store 原生化直接前置；GRX-015/016/018 GPU-driven 三件套原生化前置；验收随各 pass gate + 本 MR §5/§6 红绿） |
| 关联条款 | 拟落 spec **RXS-0181~0183**（`spec/dxil_backend.md` 续号，A/B1 落该文件；RXS-0183 intrinsic 集语言面同步在 `spec/device.md` 以镜像 RXS-0081 形态扩节或交叉引用——落点随条款 PR 定，编号经实读核对：现存最高 RXS-0180 @ edition.md） |
| 依据决策 | **D-131**（混合 compute=A，13 §D-131 v1.4）· **RFC-0003**（MIR→DXIL 第二后端 + RXS-0157 L2 子集边界/RX6007 strict）· types.md 位运算既有条款（RX2006）· device.md RXS-0066/0067/0071（View 元素类型泛化）/ RXS-0081（device intrinsic 集先例形态）· **RD-025**（上游 intrinsic 缺口的受控本地 llc patch 纪律先例）· 先例 **MR-0005**（Mini 携新 RXS 条款 + 升档触发条件） |
| Provenance | `Assisted-by: claude-code:claude-fable-5`。草案由判档任务产出；agent 批准前不推进下游实现 PR |
| 失败测试先行 | `conformance/dxil/accept/view_param_u32.rx` + `conformance/dxil/accept/bitops_word_manipulation.rx` + `conformance/dxil/accept/find_lsb_scan.rx`（三件均为**当前 main/分支上 RED** 的 accept 语料：现行 `require_view_global_f32` / 二元白名单 / 无 intrinsic 决议分别使其被 RX6007 或名称决议拒绝；实现 PR 落地后转绿，并新增 reject 侧有意义拦截，见 §5） |

---

## 1. 摘要

把 DXIL compute body lowering（D-131 compute=A 路，`src/rurixc/src/dxil_codegen.rs`）
的可表达子集从「f32 元素视图 + `+ - * / %` 标量算术」扩到 GPU-driven 渲染 kernel
的整型工作集：**(A)** `View/ViewMut<global, u32|i32>` 元素视图（语言类型面 0-byte
既有，仅补后端降级）；**(B1)** 整型位运算 `& | ^ << >>` 的 DXIL 降级（语言语义
0-byte 既有，仅补后端映射，含移位量全定义化）；**(B2)** 位扫描/位计数 intrinsic
`find_lsb`/`find_msb`/`popcount`（语言面新增，镜像 RXS-0081 device 数学 intrinsic
集形态，NVPTX + DXIL 双后端同落，纯值运算、总函数、零内存模型面）。动机实证：
GRX-014 cluster_store 全 kernel = `uint[]` SSBO 位模式运算 + findLSB/findMSB
word-scan（PASS_CONTRACT §5.3 三重阻断，全部落于 A/B1/B2）；GRX-015/016/018 同
形态。落地后 cluster_store 可 rurix_owned（退役 hlsl_bridge_workaround 实例），
texture_artifact_provenance_policy 的 rurix_owned 偏好得以兑现。最大化复用：
既有类型检查/借用/MIR 前沿 0-byte，PTX 路径 0-byte（B2 为 NVPTX 纯追加），
绑定布局推导 RXS-0163~0166 0-byte（u32 视图同种类轴/register/RTS0 形态）。

## 2. 设计（用户视角 + 形态）

用户视角（cluster_store 型 kernel 片段，落地后可编译到 `--target dxil`）：

```text
kernel fn cluster_store(
    cluster_render: View<global, u32>,      \ SRV t0（种类轴/RTS0 与 f32 视图同形,RXS-0163~0166 0-byte）
    elements:       View<global, u32>,      \ SRV t1
    cluster_out:    ViewMut<global, u32>,   \ UAV u0
    t: ThreadCtx<1>,
) {
    let w: u32 = cluster_render[t.global_id()];
    let z_range: u32 = (w >> 8u) & 0xFFu;          \ B1:位运算符（types.md 既有语义）
    if z_range != 0u {
        let from_z = find_lsb(z_range);            \ B2:新 intrinsic
        let to_z   = find_msb(z_range) + 1u;
        cluster_out[t.global_id()] = w | (from_z << 24u);
    }
}
```

### 2.A 整型 raw buffer 视图（RXS-0181 拟落）

- **类型规则**：`View<global, T>` / `ViewMut<global, T>`，`T ∈ {f32, u32, i32}`
  为 DXIL compute 形参可接受元素集（现行仅 f32）。类型合一/可变性/地址空间规则
  全部复用 RXS-0067（0-byte）；子集外元素（f64/bool/聚合等）维持 **RX6007**
  strict 拒绝（RXS-0157 L2 边界收窄，不新造码）。
- **IR emit 形态**：复用现行 A 路生产拼写（bless_log 2026-07-03 定型）——
  `@llvm.dx.resource.handlefrombinding` 建句柄 +
  `@llvm.dx.resource.load.rawbuffer.*` / `@llvm.dx.resource.store.rawbuffer.*`
  按元素类型重载（f32 现行 → 增发 i32 重载；u32 在 LLVM IR 层同为 `i32`，
  有/无符号语义由运算指令侧承载）。下游 DXIL op `rawBufferLoad(139)` /
  `rawBufferStore(140)` 为元素类型重载 op（dxc 对 `StructuredBuffer<uint>` /
  `ByteAddressBuffer` 即产 i32 重载 = 存在性证明）。
- **上游支持面论证（实现前置 probe，义务项）**：pinned llc（LLVM 23.0git +
  RD-025 texture-store patch 环境）对 i32 rawbuffer 重载的 `-filetype=obj`
  emit 稳定性（×8 字节一致）+ `dxv.exe` `Validation succeeded` 实测，镜像
  cs_texture 收口纪律；**实测通过前不落 codegen**。预期低风险：rawbuffer 族
  与 f32 同一 lowering 路径（`DXILResourceAccess`/`DXILOpLowering`），非 texture
  式缺口；若实测拒绝 → 按 §3 升档触发处置。
- **内部形态**：`LoweredScalarTy` 增 `U32/I32`；`ComputeParamKind` 增
  `ViewU32/ViewMutU32/ViewI32/ViewMutI32`；索引/边界/动态索引语义复用 f32 路径
  既有实现形态（0-byte 语义，换元素宽度不换规则——u32/i32 与 f32 同为 4 字节
  天然对齐标量，**无任何新布局规则**）。

### 2.B1 整型位运算 DXIL 降级（RXS-0182 拟落）

映射表（MIR `BinOp` → LLVM 指令 → DXIL；位运算仅整数，types.md 既有 RX2006 拦非法类型）：

| Rurix / MIR | LLVM（DirectX 三元组 IR） | DXIL 层 | 备注 |
|---|---|---|---|
| `BitAnd` | `and` | 平凡指令（无 dx.op） | |
| `BitOr` | `or` | 平凡指令 | |
| `BitXor` | `xor` | 平凡指令 | |
| `Shl` | `shl`（前置显式掩码） | 平凡指令 | 移位量 emit `and amount, width-1` |
| `Shr`（u32） | `lshr`（前置显式掩码） | 平凡指令 | 逻辑右移 |
| `Shr`（i32/i64） | `ashr`（前置显式掩码） | 平凡指令 | 算术右移 |

- **移位量语义（全定义，无 UB 节，P-01）**：移位量按左操作数位宽取模
  （`amount & (width-1)`），编译器在**两个 device 后端均 emit 显式掩码**（LLVM
  `shl/lshr/ashr` 越界为 poison，掩码消除之；PTX 原生钳制语义被掩码前置统一）。
  host/consteval 侧对齐面为实现期核对项，分歧即停手（判档意见书 O-2）。
- NVPTX 侧位运算既有降级 0-byte 不动；本条只补 DXIL 白名单
  （dxil_codegen ≈L1282-1289 扩表）+ 掩码语义显式化。

### 2.B2 位扫描/位计数 intrinsic（RXS-0183 拟落，语言面新增，双后端）

签名与语义（镜像 RXS-0081 条款形态——「device 上下文可调用的编译器已知函数集」；
纯值运算、总函数、零输入显式定义、无内存/并发面）：

| intrinsic | 签名 | 语义（LSB=0 位序） | 零输入 | NVPTX lowering | DXIL lowering（须 probe 实测） |
|---|---|---|---|---|---|
| `find_lsb(x)` | `u32 -> u32` | 最低置位位下标 | `0xFFFF_FFFFu`（O-1 预设 HLSL 形） | `llvm.cttz.i32`（→ PTX 组合/`bfind` 族） | `llvm.cttz.i32` → dx.op `FirstbitLo(32)` |
| `find_msb(x)` | `u32 -> u32` | 最高置位位下标 | `0xFFFF_FFFFu` | `llvm.ctlz.i32` 派生（`31 - clz`） | `llvm.ctlz.i32` → dx.op `FirstbitHi(33)`（i32 变体 `FirstbitSHi(34)` 留后续；dxc 正规化形态 `31 - FirstbitHi` 仅 golden 锚定，不写死为语言承诺，O-7） |
| `popcount(x)` | `u32 -> u32` | 置位位计数 | `0u`（自然） | `llvm.ctpop.i32`（→ PTX `popc`） | `llvm.ctpop.i32` → dx.op `Countbits(31)` |
| （O-4 待裁）`bitcast_f32_to_u32` / `bitcast_u32_to_f32` | `f32 -> u32` / `u32 -> f32` | IEEE-754 位重解释（总函数，NaN 载荷逐位保留） | — | LLVM `bitcast` | LLVM `bitcast`（平凡，无 dx.op） |

- 类型不符（浮点实参喂位扫描等）→ 既有 **RX2001** 类型不匹配段裁决（对齐
  RXS-0081「混入非浮点 → RX2001」先例形态），零新码。
- **上游覆盖诚实边界**：pinned llc 对 `cttz/ctlz/ctpop` 的 DirectX 后端 lowering
  覆盖须 probe 实测（每 op ×8 字节稳定 + dxv 接受）；缺口走 RD-025 受控本地
  patch 纪律 + upstream issue 链接（10 §8），不自造拼写（GRX-009 教训）。

## 3. 为何 Mini-RFC（而非 Direct，亦非 Full RFC）

- **非 Full RFC**：(A) 零新语法/零类型系统变更——`View<space,T>` 元素类型泛化、
  u32/i32 primitive、位运算类型规则全部既有条款 0-byte；(B1) 语言语义既有，仅
  后端映射 + 移位量全定义化（以编译期确定性掩码定义，非 UB、非内存模型）；(B2)
  新 intrinsic 为纯值运算总函数，精确镜像 RXS-0081 既有 intrinsic 集条款形态，
  不触 06 §4.2 内存模型映射、不触 RFC-0003 §4.6 FFI ABI/二进制布局（标量元素
  无 offset/stride 新规则，RXS-0171 升档触发面未触及）、不触 unsafe/安全包络、
  不触绑定模型（RXS-0163~0166 0-byte，与 GRX-022/RD-018 bindless 正交——判档
  意见书 §4）。
- **非 Direct**：接受面扩大 = 语言可观测行为变化（原 RX6007 拒绝程序转编译
  通过）；「上游 rawbuffer i32 重载 / cttz·ctlz·ctpop 后端覆盖」为执行期新决策
  面；B2 为语言 intrinsic 面新增。硬规则「判档争议向上取严」+ MR-0005 先例
  （对自身新决策面走 Mini 携新 RXS）→ 单页 Mini + 失败测试先行。
- **升档触发条件（实现期守卫）**：① probe 实测发现整型 rawbuffer/位扫描降级
  必须触及**显式字节布局 / descriptor 编码变更 / 新 FFI ABI 面**（如被迫改
  RTS0/绑定形态才能过 dxv）→ 停手升 Full RFC（RXS-0171 同款触发）；② B2 若需
  定义任何跨线程可见性/内存序/wave 级语义 → 停手升 Full；③ 移位量语义若无法
  在 host/consteval/NVPTX/DXIL 四面统一为掩码语义而须引入 per-target 分歧语义
  → 停手交裁（不落分歧语义）；④ 聚合元素类型任何形态不得借本 MR 夹带
  （RD-026 预览 / 候选 RFC-0009 专属）。

## 4. 错误码 / 影响 / 范围

- **零新 RX 码**：子集外构造维持 **RX6007**（RXS-0157 L2，拒绝面收窄措辞随条款
  PR 更新 message 文案不改语义）；位运算类型违例维持 **RX2006**（types.md）；
  intrinsic 实参类型违例复用 **RX2001**（RXS-0081 先例形态）。备查下一可用
  RX6024，不预造。
- **向后兼容**：纯追加——现行 f32 视图路径、全部既有 accept/reject 语料、
  `.dxil-ll`/`.dxil-disasm` golden 既有文件 **0-byte**（新能力全部新增语料/新
  golden 文件承载；`cs_noop`/`cs_texture` 等不重 bless）；PTX/NVPTX 路径对
  A/B1 0-byte（既有降级），对 B2 为纯追加 intrinsic；host 编译面 0-byte。
- **范围红线**：不做聚合元素类型（RD-026 预览/Full RFC 专属）；不做 bindless/
  资源数组（RD-018/GRX-022）；不改绑定布局推导条款与 RTS0 形态；不动 D-205 pin
  （llc 缺口走 RD-025 纪律 env 覆盖）；不触图形=B 路（本 MR 仅 compute=A）。

## 5. 失败测试先行（10 §3 Mini 硬性）

| 测试 | 编码的意图 | 当前为何 RED | 落地后 |
|---|---|---|---|
| `conformance/dxil/accept/view_param_u32.rx`（拟新增，`//@ spec: RXS-0181`） | `kernel fn k(src: View<global, u32>, dst: ViewMut<global, u32>, t: ThreadCtx<1>) { dst[t.global_id()] = src[t.global_id()]; }` 可过 DXIL 编译 | `require_view_global_f32` 报 RX6007 | 转绿（+ golden `cs_bitops` 系） |
| `conformance/dxil/accept/bitops_word_manipulation.rx`（拟新增，`//@ spec: RXS-0182`） | `(w >> 8u) & 0xFFu`、`w \| (x << 24u)` 等 word 运算可降级 | 二元白名单只 `+ - * / %` → RX6007 | 转绿 |
| `conformance/dxil/accept/find_lsb_scan.rx`（拟新增，`//@ spec: RXS-0183`） | `find_lsb/find_msb/popcount` 决议 + 降级 | intrinsic 不存在（名称决议失败） | 转绿 |
| reject 侧（拟新增，见 §6） | strict 边界仍有意义 | — | 落地后为有意义拦截（RX6007/RX2006/RX2001） |

## 6. conformance 用例计划 / golden 影响面

**accept**（`conformance/dxil/accept/`，各锚对应拟落条款）：
`view_param_u32.rx`（u32 视图 copy）· `view_param_i32.rx`（i32 视图 + ashr 路径）·
`bitops_word_manipulation.rx`（&、|、^、<<、>> 组合）· `shift_amount_masked.rx`
（移位量掩码语义可编译锚点）· `find_lsb_scan.rx`（cluster_store 型 word-scan 循环）·
`popcount_reduce.rx` ·（O-4 若入册）`bitcast_f32_u32_roundtrip.rx`。

**reject**（`conformance/dxil/reject/`）：
`view_param_f64.rx`（`//@ expect-error: RX6007`，子集外元素类型仍 strict 拒）·
`bitops_on_f32.rx`（`//@ expect-error: RX2006`，位运算浮点操作数——types.md 既有规则的 DXIL 语料化）·
`shl_mixed_width.rx`（`//@ expect-error: RX2006`，混宽移位）·
`find_lsb_on_f32.rx`（`//@ expect-error: RX2001`，intrinsic 实参类型违例）。

**golden**（`tests/dxil/`，两层纪律 + bless_log 追加，RXS-0157 IR3 0-byte 复用）：
新增 `cs_bitops.rx` + `.dxil-ll`（rurixc DirectX 三元组 IR 文本，锁 i32 rawbuffer
重载拼写 + and/or/xor/shl/lshr/ashr + 掩码形态）+ `.dxil-disasm`（pinned llc obj
→ dxv `Validation succeeded` → dumpbin，锁 `rawBufferLoad(139)`/`rawBufferStore(140)`
i32 重载 + `FirstbitLo(32)`/`FirstbitHi(33)`/`Countbits(31)` 文本）；**既有 golden
文件 0-byte**（不重 bless cs_noop/cs_texture/图形系）。GRX 侧另有 pass 级证据
（offline_compile_evidence / math parity）随 GRX-014 实现，不在本 MR 范围。

## 7. Agent 批准

> **Approved — 2026-07-12**。agent（完全自主，承 owner 已签署的 agent 完全自主化治理基线（b17fd67 在 origin/main）与 g2.6 Mini-RFC 自主签署先例）批准本 Mini-RFC 全文；判档裁决点（判档意见书 §7）落档如下：
>
> - **O-1** 位扫描零输入语义 = **HLSL 形 `0xFFFF_FFFF`**（u32→u32 闭合，免符号扩展歧义；DXIL 目标 + dxc golden 锚；条款（RXS-0183 L3）注明与 GLSL findLSB/findMSB 的 `-1`（i32）约定同位模式、不同类型视角）。
> - **O-2** 移位越界 = **按位宽取模**（`amount & (width-1)`），**双 device 后端 emit 显式掩码**（DXIL `dxil_codegen` + NVPTX `device_codegen` 均落地）。实现期核对结论：consteval 越界移位维持既有 checked overflow **编译期拒绝**（与算术 overflow checked 语义同款先例，const 语境先行拒绝，非运行期分歧语义）；host 编译面 0-byte（§4 承诺维持）。四面无 per-target 分歧语义引入，升档触发③ 未触发。
> - **O-3** rurixc u32 视图维持 rawbuffer intrinsic 形态；shim/pass 侧描述符种类对齐属 GRX 集成项，不进语言绑定模型（维持草案预设，本 MR 不阻塞）。
> - **O-4** `bitcast` 位重解释**不随本 MR 入册**（§2.B2 表中 O-4 行不落条款、不落实现；C2 过渡路径受限如实接受）。
> - **O-5** C2 聚合元素 Full RFC 立项时机 defer（RD-026 正式登记随 C2 立项/实现 PR；本 MR registry 0-byte）。
> - **O-6** 上游覆盖缺口授权走 RD-025 patch 纪律——**实测未触发**：probe（2026-07-12，pinned llc `H:\llvm-clean-82c5bce5-build\bin\llc.exe`）实测 `llvm.cttz/ctlz.i32` 被 DXILBitcodeWriter 拒（「Unsupported intrinsic … for DXIL lowering」），但 `llvm.dx.firstbitlow` / `llvm.dx.firstbituhigh` / `llvm.ctpop.i32` 全部上游在位（各 `-filetype=obj` ×8 sha256 单值 + dxv `Validation succeeded`）→ 零本地 patch、零新 RD。
> - **O-7** findMSB 正规化**仅 golden 锚定**：emit dxc 同款 `select(raw == -1, -1, 31 - raw)` 形，与 pinned dxc 对 HLSL `firstbithigh` 产物逐形对照锚定，不写死为语言 opcode 组合承诺（RXS-0183 IR1）。
>
> **§2 形态确认**：A（RXS-0181）/ B1（RXS-0182）/ B2（RXS-0183）条款体落 `spec/dxil_backend.md`；**RXS-0183 语言面即落该文件、device.md 0-byte**（落点由本批准依「关联条款」栏授权裁定，交叉引用形态）。§2.A/§2.B2 probe 义务已**前置达成**（i32 rawbuffer 重载 + 混合 f32+i32 模块 ×8 字节稳定 + dxv `Validation succeeded`；`tests/dxil/bless_log.md` 2026-07-12 行）。**执行期实测修正三项（如实留痕，语义面不变）**：① 下游 DXIL op 在 SM6.0 实测归 `bufferLoad.i32(68)` / `bufferStore.i32(69)`（§2.A 预期的 rawBufferLoad(139)/rawBufferStore(140) 为 SM6.2+ 形；元素类型重载轴一致，golden 锚实测形）；② `handlefrombinding` 第 4 实参修正为 **range 内相对 index 0**（probe 实测 llc 以 lowerBound+index 计算 createHandle 绝对 register，register>0 传 register 越 range 被 dxv 拒；register 0 两种拼写字节一致 → 既有 golden 0-byte）；③ 移位首期收窄为 **u32/i32**（slice 3a I64 lowering 把 usize/u64/i64 归并同底，`>>` 的 lshr/ashr 符号性不可判 → strict RX6007 拒绝优于错码，P-01；`& | ^` 对 I64 归并域照常降级——§2.B1 表 i64 移位行按此收窄）。
> **§4 错误码确认**：零新 RX 码（RX6007/RX2006 如案）。§6 反例码两处实测修正留痕：`shl_mixed_width` 实测落 **RX2001**（混宽经类型合一失败，先于运算数裁决，计划 RX2006 修正）；`find_lsb_on_f32` 实测落 **RX2004**（方法接收者不符走 unknown-method 段，实参不符才走 RX2001，计划 RX2001 修正）。拦截均仍有意义（strict 边界不变）。
> **§5 失败测试先行确认**：三件 RED 语料（view_param_u32 / bitops_word_manipulation / find_lsb_scan）落地后转绿；accept 共 6 件 + reject 4 件（本 MR 侧）全链留痕，golden `cs_bitops` 两层 bless（`.dxil-ll` + llc→dxv→dumpbin `.dxil-disasm`），新 accept 语料各经 `rurixc --target dxil` 全链（llc→DXIL 容器→dxv）实测 accepted。**§3 升档触发确认**：①（显式布局/descriptor/FFI 面）②（跨线程/内存序/wave）③（移位四面分歧）④（聚合元素夹带）全部未触发。
> 批准与落档由 agent 完全自主签署；provenance `Assisted-by: claude-code:claude-fable-5`（D-406）。

## 8. 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-07-12 | 初版 Mini-RFC Draft（判档任务产出，随判档意见书 `spike/dxil-path-probe/dxil_compute_capability_gaps_adjudication_20260712.md`） | Mini-RFC（MR-0006） |
| v1.1 | 2026-07-12 | agent 完全自主批准（§7 批准段：O-1~O-7 裁决落档 + probe 实测矩阵 + 执行期实测修正三项留痕 + 反例码两处修正留痕）；状态 Draft → Approved；条款 RXS-0181~0183 + 实现（dxil_codegen/device_codegen/typeck/hir/tbir/mir_build）+ conformance/golden 同轮落地 | Mini-RFC（MR-0006） |
