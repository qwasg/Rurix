# PR-C2 分片3 RXS-0159 签名映射真达产物补证（RD-011 patched llc 环境，round-8）

> 跟踪：`registry/deferred.json` **RD-011**（patched llc dev 偏差）/ **RD-013**（入口 body 数据流降级）。
> 关联：spec/dxil_backend.md **RXS-0159**（阶段 I/O → DXIL 签名/系统值语义，类型面 IR2）。
> Provenance：`Assisted-by: kiro:claude-opus-4-8`。
> 地位：**measured-first 诚实快照**。本文件 append-only 新增，不回改分片1/2/3 既有冻结 evidence。

---

## 0. 目的与结论（先给结论）

分片3 此前 dxc validation **SKIP**（无 patched llc），golden 只到 `.dxil-ll`（IR），
未证 RXS-0159 的 SV 签名映射**真达 DXIL 产物**。本轮在 RD-011 patched llc 环境补证。

**结论 =（B）真发现 → 需升档（blocked）**：

- vertex_io / fragment_io 经 patched llc `-filetype=obj` 产 DXContainer，
  **IDxcValidator ×25 = 25/25 accept（`{0x0:25}`）、dxv.exe ×20 = 20/20 `Validation succeeded.`**；
- **但产物签名 part（ISG1/OSG1）`elemcount = 0`**——**无 SV_Position / SV_Target / SV_VertexID
  任何 SV 元素**。RXS-0159 的 SV 语义名映射**未进 DXIL 产物**。
- 根因：Rurix 自有元数据 `!rurix.dxil.sig.in/.out` 被 LLVM DirectX 后端**完全忽略**；
  且后端 `addSignature()` 对图形着色器**无条件写空签名**（`// FIXME: support graphics
  shader`，上游 issue #90504）——**任何元数据形态**（`rurix.*` / `hlsl.*` / 标准
  `dx.entryPoints` 签名元素）当前都不会被 lower 成 ISG1/OSG1 元素。
- 让 SV 元素进产物需 patch 后端调 `Signature::addParam(... Register, Mask,
  ExclusiveMask ...)`，即定义**寄存器/分量 mask 二进制布局** = RFC-0003 §4.6 / §9 Q-Builtin
  🔒 **FFI ABI 禁区**，**越出 RXS-0159 类型面** → 按硬规则 5 停手标「需升档」。
- **不录 `.dxil-disasm` golden**（签名空，录入即伪造 SV 真达）；不 patch LLVM 后端；如实 blocked + 复现。

---

## 1. 环境（RD-011 patched llc + 2026 签名 validator）

| 项 | 值（真实命令输出） |
|---|---|
| patched llc | `H:\llvm-clean-82c5bce5-build\bin\llc.exe` |
| llc 版本 | `LLVM version 23.0.0git`，`Optimized build with assertions` |
| dxil target | `dxil - DirectX Intermediate Language`（Registered Targets 在位） |
| llc SHA256 | `BF6C0868AB875F664F18BC7190BABC2734FCF50B144C92D94CABDD069D745261` |
| validator 套件 | `H:\dxc-round7\extracted\bin\x64`（dxc.exe / dxcompiler.dll / dxil.dll / dxv.exe） |
| validator 版本 | 1.9.2602.24（2026 签名 validator，RD-011/round-7） |
| 定位方式 | `RURIX_LLC` / `RURIX_DXC_NEW_DIR` dev env（隔离不入库，不改 D-205 pin） |
| recipe | spike/dxil-path-probe/dxil_psv_patch_recipe.md |

## 2. 输入（分片3 既有 .dxil-ll golden，未改）

- `tests/dxil/vs_io.dxil-ll`：vertex 入口，`!rurix.dxil.sig.in={vid:SV_VertexID}`、
  `!rurix.dxil.sig.out={pos:SV_Position, uv:interp:linear}`，函数体 `define void @rx_vs_io_13(){ ret void }`。
- `tests/dxil/ps_io.dxil-ll`：fragment 入口，`!rurix.dxil.sig.in={coord:SV_Position, uv:interp:linear}`、
  `!rurix.dxil.sig.out={color:SV_Target}`，函数体 `define void @rx_fs_io_13(){ ret void }`。

> 两者签名均经 `!rurix.dxil.sig.*` 自有命名元数据 emit，入口为 `void` stub（RD-013 body deferred）。

## 3. emit + validator 真验证（真实命令输出）

```
# patched llc -filetype=obj
llc vs_io.dxil-ll -filetype=obj -o vs_io.obj   # exit 0, 1888 bytes
llc ps_io.dxil-ll -filetype=obj -o ps_io.obj   # exit 0, 1888 bytes

# DXContainer part 表（dxil_container.parse_dxbc）
vs_io True ['DXIL', 'SFI0', 'HASH', 'ISG1', 'OSG1', 'PSV0']
ps_io True ['DXIL', 'SFI0', 'HASH', 'ISG1', 'OSG1', 'PSV0']

# IDxcValidator ×25（r8_probe.py）
vs_io  "accepted": "25/25"  status_hist {"0x0": 25}
ps_io  "accepted": "25/25"  status_hist {"0x0": 25}

# dxv.exe ×20
vs_io  accept: 20/20  -> "Validation succeeded."
ps_io  accept: 20/20  -> "Validation succeeded."
```

## 4. 签名 part 实证（本轮证据核心）——ISG1/OSG1 元素数

解 DXContainer 的 ISG1（输入签名）/ OSG1（输出签名）part：头部 `u32 ParamCount`。

```
=== vs_io
  ISG1 size 8 elemcount 0
  OSG1 size 8 elemcount 0
=== ps_io
  ISG1 size 8 elemcount 0
  OSG1 size 8 elemcount 0
```

**ISG1/OSG1 均 size=8（仅 8 字节头：ParamCount=0 + ParamOffset=8）、elemcount=0**——
产物签名 part **不含任何 SV 元素**。validator 之所以 accept，是因 `void` 入口的**空签名结构合法**，
**非**因 SV 映射真达：accept ≠ 签名含 SV（正是本轮要识破的陷阱）。

## 5. 根因定位（到 LLVM 后端函数/行）

隔离 fork worktree `H:\llvm-clean-82c5bce5-src`（未动 D-205 pin/toolchain.rs/src/）：

`llvm/lib/Target/DirectX/DXContainerGlobals.cpp` `addSignature()`：

```cpp
void DXContainerGlobals::addSignature(Module &M,
                                      SmallVector<GlobalValue *> &Globals) {
  // FIXME: support graphics shader.
  //  see issue https://github.com/llvm/llvm-project/issues/90504.
  Signature InputSig;
  Globals.emplace_back(buildSignature(M, InputSig, "dx.isg1", "ISG1"));
  Signature OutputSig;
  Globals.emplace_back(buildSignature(M, OutputSig, "dx.osg1", "OSG1"));
}
```

- `InputSig` / `OutputSig` **构造即空、从不 `addParam`**，直接 `write` 出 8 字节空 part。
  后端**不读任何模块元数据**来填充图形签名（`// FIXME: support graphics shader`，issue #90504）。
- `llvm/include/llvm/MC/DXContainerPSVInfo.h` `class Signature::addParam(... uint32_t Register,
  uint8_t Mask, uint8_t ExclusiveMask ...)`——填一个签名元素**强制要求 Register / Mask /
  ExclusiveMask**（寄存器分配 + component mask = 二进制布局）。
- 经全后端 + mcdxbc 检索：DirectX 后端**无任何调用点**对图形 ISG1/OSG1 调 `addParam`（root signature
  的 `addParameter` 是另一回事）。即**当前后端无图形签名填充路径**。

**推论**：`!rurix.dxil.sig.*` 被忽略不是命名空间问题——即便改用 `hlsl.*` 属性或标准
`dx.entryPoints` 签名元素，当前后端**同样**不会 lower 进 ISG1/OSG1（消费代码不存在，#90504）。
分支 B「改 lowering 为后端可消费形态」**对当前后端不成立**。

## 6. 判定与处置（硬规则 5 边界 → 需升档）

让 SV 元素真达产物的**唯一**途径 = patch LLVM DirectX 后端 `addSignature` 实现图形签名
填充，必经 `Signature::addParam(Register, Mask, ExclusiveMask, ...)` 给出**寄存器/分量 mask
二进制布局** = RFC-0003 §4.6 / §9 Q-Builtin 🔒 **FFI ABI 禁区**，**越出 RXS-0159 类型面**
（类型面只裁 SV 语义名，不碰 ABI 布局）。

按任务分支 B 末路条款 + 硬规则 5：

- **停手标「需升档」**：签名真达产物耦合 ABI 二进制布局，需 agent 独立 Full RFC
  （承 RD-013 backfill_condition 已声明的「签名元素二进制布局…由 agent 后续独立 Full RFC 定义」）+
  上游 #90504 图形签名支持落地。
- **不录 `.dxil-disasm` golden**（签名空，录入即伪造）；**不 patch LLVM 后端**（禁区）；
  **不改 RXS-0159 语义 / 不动 PTX / 不动 committed D-205 pin / trace 维持 159/159**。
- 真发现留痕：bless_log 追加本轮补证结果（disasm 关卡由 SKIP 收紧为 measured「accept 但签名空」）；
  RD-011 history 追加；RD-013 已覆盖签名→产物的 ABI 耦合 deferred，本发现强化其 backfill 条件。
- 上游 #90504（图形签名支持）随 RD-011 history 留痕，为退役/接通前置。

## 7. 复现命令

```powershell
$env:RURIX_LLC="H:\llvm-clean-82c5bce5-build\bin\llc.exe"
$env:RURIX_DXC_NEW_DIR="H:\dxc-round7\extracted\bin\x64"
& $env:RURIX_LLC tests\dxil\vs_io.dxil-ll -filetype=obj -o vs_io.obj
py -3 H:\dxil-round8\r8_probe.py vs_io.obj          # 25/25 accept {0x0:25}
H:\dxc-round7\extracted\bin\x64\dxv.exe vs_io.obj    # Validation succeeded.
# 签名 part：解 ISG1/OSG1 头 u32 ParamCount -> 0（无 SV 元素）
```
