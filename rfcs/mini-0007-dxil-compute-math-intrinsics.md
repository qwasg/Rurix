# Mini-RFC MR-0007 — DeviceMathFn 的 DXIL 第二后端降级（sqrt/rsqrt/sin/cos → dx.op.unary，f32 首期）

| 字段 | 值 |
|---|---|
| Mini-RFC 标识 | **MR-0007**（Mini-RFC 序列；独立于 Full-RFC 的 `RFC-####` 命名空间，不复用 RFC 编号，10 §9.5。Mini-RFC = 单页提案 + 失败测试先行，10 §3。编号承 MR-0006（本轮同批草案），实读核对无占用） |
| 标题 | device 数学函数 intrinsic 集（RXS-0081 既有语义面）在 DXIL compute=A 路的 lowering 覆盖：`sqrt`/`rsqrt`/`sin`/`cos`（f32 首期）→ LLVM float intrinsic → dx.op.unary |
| 档位 | **Mini-RFC**（10 §3：语言语义**既有 0-byte**——RXS-0081 已定 intrinsic 集签名契约/元数/求值语义 + RXS-0082 链接流程 + RX6006 保守拒绝；本件纯第二后端 lowering 覆盖扩展，**不触** UB / 内存模型映射（06 §4.2）/ FFI ABI（RFC-0003 §4.6）/ 显式布局（RXS-0171 升档触发）/ 安全包络禁区——见 §3。判档意见书：`spike/dxil-path-probe/dxil_compute_capability_gaps_adjudication_20260712.md`） |
| 状态 | **Draft — 2026-07-12**（判档材料随附草案；**未批准**——批准/合入/台账回填归 agent 批准流程，批准前不推进任何实现 PR） |
| 承接里程碑 | GRX（GRX-013 particles_copy `ALIGN_BILLBOARD` 子集原生化前置之一——Rodrigues 需 sin/cos、normalize/cross-normalize 需 sqrt/rsqrt，PASS_CONTRACT §5.3 第 2 条；聚合元素另卡 RD-026 预览/候选 RFC-0009，本 MR 不解） |
| 关联条款 | 拟落 spec **RXS-0184**（`spec/dxil_backend.md` 续号；编号承 MR-0006 拟占 RXS-0181~0183，现存最高 RXS-0180 @ edition.md 实读核对） |
| 依据决策 | **D-131**（混合 compute=A）· **RFC-0003**（RXS-0157 L2 子集边界/RX6007）· device.md **RXS-0081**（device 数学函数 intrinsic 集与求值语义——签名契约/元数/RX6006）+ **RXS-0082**（libdevice 链接流程,NVPTX 侧 0-byte 对照面）· **RD-025**（上游缺口受控本地 patch 纪律先例）· 先例 **MR-0005/MR-0006**（Mini 携新 RXS + 升档触发） |
| Provenance | `Assisted-by: claude-code:claude-fable-5`。草案由判档任务产出；agent 批准前不推进下游实现 PR |
| 失败测试先行 | `conformance/dxil/accept/math_sqrt_rsqrt.rx` + `conformance/dxil/accept/math_sin_cos.rx`（两件均为**当前 RED** 的 accept 语料：`dxil_codegen` 对 `DeviceMathFn` 调用零处理 → RX6007「slice 3a 仅支持标量 + - * / 与简单比较」形拒绝；实现 PR 落地后转绿 + reject 侧有意义拦截，见 §5） |

---

## 1. 摘要

Rurix 的 device 数学函数 intrinsic 集（`sqrt`/`rsqrt`/`sin`/`cos`/…）语义已由
RXS-0081 定型（浮点 f32/f64、签名契约、按元数与类型合一裁决、超覆盖 → RX6006），
但 lowering **仅存在于 NVPTX 路**（libdevice `__nv_*` + `-mlink-builtin-bitcode`，
RXS-0082）；DXIL compute=A 路零处理（particles_copy PASS_CONTRACT §5.3 第 2 条
实证阻断）。本 MR 为 DXIL 第二后端补该 intrinsic 集的 lowering 覆盖：**f32 首期
四函数** `sqrt/rsqrt/sin/cos` → LLVM float intrinsic（`llvm.sqrt.f32` 等）→ 上游
DirectX 后端 → `dx.op.unary`；f64 与首期外函数维持 strict 拒绝（RX6006/RX6007，
不静默降级，P-01）。语言语义、NVPTX 路径、host 路径全部 **0-byte**；本件是
纯第二后端覆盖扩展 + 上游 opcode 支持面实证义务。落地后 GRX-013 particles
billboard 子集的数学面阻断消除（聚合元素阻断仍存，RD-026 预览承接——本 MR
**不**宣称 particles 全 kernel 原生可达）。

## 2. 设计（用户视角 + 形态）

用户视角：既有 RXS-0081 语法 0-byte——`kernel fn` / `device fn` 体内
`sqrt(x)`/`rsqrt(x)`/`sin(x)`/`cos(x)`（f32）在 `--target dxil` 下不再被拒。

映射表（首期 f32；LLVM intrinsic 均为既有 `DeviceMathFn` → NVPTX 时的同族拼写，
DXIL 侧走上游 DirectX 后端 lowering，**dx.op 编号以 DXIL.rst unary 段为准**）：

| Rurix intrinsic（RXS-0081 既有） | LLVM（DirectX 三元组 IR） | DXIL op（dx.op.unary.f32） | 备注 |
|---|---|---|---|
| `sqrt(f32)` | `llvm.sqrt.f32` | `Sqrt(24)` | |
| `rsqrt(f32)` | 组合或 `llvm.dx` 族（probe 定拼写） | `Rsqrt(25)` | 上游若无直达 intrinsic,以 `1.0/sqrt` 组合 emit 需精度评估——见 §3 升档触发③ |
| `sin(f32)` | `llvm.sin.f32` | `Sin(13)` | |
| `cos(f32)` | `llvm.cos.f32` | `Cos(12)` | |

- **首期收敛与 strict 边界**：f32 四函数之外（f64 任意、`pow`/`fma`/`exp`/`log`
  等其余 RXS-0081 集合成员在 DXIL 路）→ **RX6006**（device codegen 不支持的
  数学 intrinsic——RXS-0081 既有码语义「不支持的元素类型组合/超覆盖」精确适用，
  零新码），不静默近似、不 fallback（P-01）。后续函数按需以同条款追加覆盖
  （拒绝面收窄 = 纯追加）。
- **精度（Implementation Requirements 表述,不发明数值语义）**：DXIL 侧精度界
  引 D3D 功能规范对相应 op 的 ULP 界；**不承诺** NVPTX(libdevice) 与 DXIL 两路
  bit-exact（两路各自对 host 参考的 parity 以证据 harness 锚定，镜像 GRX
  math parity evidence 纪律——particles/cluster pass 侧已有
  `generate_math_parity_evidence.py` 同款先例）；golden 只锁 IR/DXIL 文本形态，
  不锁数值。
- **上游支持面论证（实现前置 probe，义务项）**：pinned llc 对
  `llvm.sqrt/sin/cos.f32`（及 rsqrt 拼写）在 DirectX 后端的 lowering 覆盖与
  `-filetype=obj` 稳定性（每函数 ×8 字节一致）+ dxv `Validation succeeded`
  实测；**实测通过前不落 codegen**；单函数缺口走 RD-025 受控本地 patch 纪律
  + upstream issue 链接（10 §8），整面缺失停手留痕。

## 3. 为何 Mini-RFC（而非 Direct，亦非 Full RFC）

- **非 Full RFC**：零语言面新增——intrinsic 集、签名契约、求值语义、错误码
  全部 RXS-0081 既有 0-byte；本件仅第二后端 lowering 覆盖（对齐「B1 位运算符
  仅补后端映射」同款判档轴，MR-0006 §3）。纯值运算浮点函数，不触内存模型
  （06 §4.2）、不触布局/FFI ABI（RXS-0171 升档面）、不触 unsafe/安全包络、
  不触绑定模型（资源面 0-byte）。
- **非 Direct**：接受面扩大（原 RX6007 拒绝程序转编译通过）+「上游 DirectX
  后端对超越函数/rsqrt 的 opcode 覆盖与拼写」为执行期新决策面 + 精度表述为
  规范新增段——向上取严走 Mini + 失败测试先行（MR-0005/0006 先例）。
- **升档触发条件（实现期守卫）**：① 若某函数在 DXIL 侧只能以**改变可观测
  数值语义**的方式达成（如被迫引入与 RXS-0081 求值语义冲突的近似且无法以
  strict 拒绝替代）→ 停手交裁；② 若须触及 06 §4.2 内存模型面（不应发生——
  纯 ALU）→ 停手升 Full；③ rsqrt 若无直达 op/intrinsic 而组合 emit
  （`1.0/sqrt`）的精度界无法以 IR 段诚实表述 → 收窄首期集合（去 rsqrt）而非
  降级承诺；④ f64 任何形态不得借本 MR 夹带（DXIL double 覆盖面另行判档）。

## 4. 错误码 / 影响 / 范围

- **零新 RX 码**：首期覆盖外 → **RX6006**（RXS-0081 既有语义精确适用）；
  DXIL 目标级子集外构造维持 **RX6007**；类型违例维持 **RX2001**（RXS-0081
  既有裁决段）。
- **向后兼容**：纯追加。NVPTX/libdevice 路径 **0-byte**（RXS-0082 不动）；
  host 路径 0-byte；既有 conformance/golden 文件 0-byte（新语料/新 golden
  文件承载）；PTX golden 不动。
- **范围红线**：不做 f64（strict 拒）；不做 RXS-0081 集合外新数学函数（语言面
  扩集另行判档）；不做聚合元素/向量化形态（`vec3` normalize 等聚合值语义属
  RD-026 预览/候选 RFC-0009——本 MR 只覆盖标量 f32 调用形态）；不触图形=B 路
  （采样/导数语义归 RFC-0007/RD-022 体系,与本 MR 无关）；不动 D-205 pin。

## 5. 失败测试先行（10 §3 Mini 硬性）

| 测试 | 编码的意图 | 当前为何 RED | 落地后 |
|---|---|---|---|
| `conformance/dxil/accept/math_sqrt_rsqrt.rx`（拟新增，`//@ spec: RXS-0184`） | `kernel fn`（f32 视图 + `sqrt`/`rsqrt`）过 DXIL 编译 | `dxil_codegen` 对 `DeviceMathFn` 零处理 → RX6007 | 转绿（+ golden `cs_math`） |
| `conformance/dxil/accept/math_sin_cos.rx`（拟新增，`//@ spec: RXS-0184`） | Rodrigues 型 `sin/cos` 组合可降级 | 同上 | 转绿 |
| reject 侧（见 §6） | strict 边界仍有意义 | — | 有意义拦截（RX6006/RX2001） |

## 6. conformance 用例计划 / golden 影响面

**accept**（`conformance/dxil/accept/`）：`math_sqrt_rsqrt.rx` ·
`math_sin_cos.rx` · `math_normalize_scalar_form.rx`（`v/sqrt(dot)` 标量分解形，
billboard 子集代表）。

**reject**（`conformance/dxil/reject/`）：`math_f64_dxil.rx`
（`//@ expect-error: RX6006`，f64 首期外）· `math_pow_dxil.rx`
（`//@ expect-error: RX6006`，首期集合外函数）· `math_on_u32.rx`
（`//@ expect-error: RX2001`，类型违例——RXS-0081 既有裁决段语料化）。

**golden**（`tests/dxil/`，两层纪律 + bless_log 追加）：新增 `cs_math.rx` +
`.dxil-ll`（锁 `llvm.sqrt/sin/cos.f32` 拼写）+ `.dxil-disasm`（pinned llc →
dxv 接受 → dumpbin，锁 `dx.op.unary.f32` 的 `Sqrt(24)/Rsqrt(25)/Sin(13)/Cos(12)`
文本形态）；**既有 golden 0-byte**。数值 parity 证据（host 参考 vs DXIL device
实跑）归 GRX pass 侧 harness（math parity evidence 先例），不入语言 golden。

## 7. Agent 批准

> **Draft — 未批准**。本节留批准流程填写（镜像 MR-0005 §7 形态：批准日期 +
> §2 首期集合/精度表述 + §3 判档 + §4 错误码 + §6 范围确认 + rsqrt 拼写/
> 升档触发③ 裁决落点记录；批准后方可推进条款 PR → 实现 PR，条款先于实现，
> 硬规则 7）。
