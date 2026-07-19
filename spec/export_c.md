# Rurix 语言规范 — `#[export(c)]` C ABI 导出 codegen + 内建头文件生成面（EI1.2 Part A，`--emit=dll` cdylib 通道；RXS-0250 起）

> 条款：**RXS-0250 ~ RXS-0255**（EI1.2 Part A，验收门 G-EI1-2）。体例见 [README.md](README.md)。
> 承 [RFC-0014](../rfcs/0014-engine-integration.md)（Agent Approved 2026-07-19，§4.A 全文批准；评审 provenance
> `claude-code:claude-opus-4-8` ≠ 起草 `claude-code:claude-fable-5`，三镜头 correctness/redline/implementability，
> D-409）。**RD-009 兑现**：rurixc 把 `.rx` 源里 `#[export(c)]` 标注的 host `pub fn` 从 parser 桩
> （parsed-but-inert，[`parser.rs`](../src/rurixc/src/parser.rs)）转为**真实 C ABI 导出**——C 兼容签名子集 v1
> 合法性编译期校验 + 保名不 mangle + link.exe `/EXPORT:` 参数发射（driver 从 typeck 导出集拼参，非 obj
> `dllexport` 源标注）+ `--emit=dll` cdylib 通道 + 编译器内建头文件生成（cbindgen 内置化，D-113/P-11）。

> 规范先行（AGENTS.md 硬规则第 7 条）：**条款 commit 先于实现 commit**；缺条款的语义 PR 必须先补 spec。
> `ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 ≥1 测试锚定（`//@ spec: RXS-####`）；本文件条款的锚定
> 测试（`ci/export_c_smoke.py` 步骤 71 + `conformance/export_c/` accept|reject 语料）随 EI1.2 实现 PR
> （`PR-A`，**条款 commit 先于实现 commit，条款+前端+后端+CI 不可分**，EA1 #158/#159 先例，RFC-0014 §6.3）同落，
> trace_matrix 维持全锚定。**本 PR 落地 = codegen emit 全接线**（前端 attr 校验 `export_c.rs::collect_c_exports` +
> 签名子集 typeck + panic 面扫描 + driver `--emit=dll` + `/EXPORT:` 发射 + import lib + 内建头生成器）+ registry
> 新码（**RX6031** 子集/体违例 · **RX6032** 空导出集 · **RX6033** 属性挂载非法，error_codes.json v1.33，
> RFC-0014 §5.1「确定 ×2 + 条件 ×2」兑现:确定 = RX6031/RX6032,属性误用 materialize 专用码 RX6033,DLL 链接失败
> 复用 RX7001/RX7022）+ 双语 message-key + conformance/CI。stable 快照因条款计数增长同 PR 重 bless（RXS-0180 L2
> 加性演进）。

---

## 1. 范围与编号区间

本文件承载 **`#[export(c)]` C ABI 导出 codegen + 内建头文件生成面**（EI1.2 Part A，RFC-0014 §4.A）。语言面新增 =
`#[export(c)]` 属性由 parsed-but-inert 转正 + `--emit=dll` 目标；codegen 面新增 = 导出发射（link.exe `/EXPORT:`）
+ 头文件生成器。**两条 C ABI 出口分工**（RFC-0014 §4.0-2，EI1_CONTRACT guardrail）：手写路 RXS-0125
（[interop.md](interop.md)，`src/rurix-interop`）+ RXS-0149 守卫（[engine_integration.md](engine_integration.md)，
`src/rurix-engine`，步骤 43）**冻结覆盖 Rust crate 出口**（语义 0-byte 只增）；本文件生成路 `#[export(c)]` codegen
+ 内建头生成**覆盖 `.rx` 出口**——两制共存，判据升级与条款同 PR（§RXS-0254）。

覆盖语义面：

- **`#[export(c)]` 属性合法性**：仅 host `pub fn` 合法；挂 device/kernel fn、非 pub、非 fn item → 编译期
  strict-only 拒；`name = "…"` 覆写导出符号名（键/值非法拒）。
- **C 兼容签名子集 v1 与类型映射**：标量 + 裸指针（T ∈ 标量）+ unit；子集 v1 外类型（`repr(C)` struct 按值 /
  回调指针 / 数组按值 / 切片 / affine 句柄）出现在导出签名 → 编译期拒。
- **导出符号表与 cdylib 产物**：保名不 mangle + driver 从 typeck 导出集拼 link.exe `/EXPORT:` 序列 +
  `--emit=dll`（`/DLL` + import lib `.dll.lib`/`.exp`）；空导出集拒。
- **内建头文件生成**：从同一 typeck C 映射确定性产 `.h`（LF 行尾 / 无时间戳 / 无绝对路径 / 两次逐字节一致 =
  幂等），单一事实源，每声明 ↔ 恰一 DLL 导出符号（承 RXS-0149 逐一对应口径）。
- **头↔ABI 守卫共存 + 跨 ABI 运行期契约**：生成路 CI 再生成逐字节比对；subset v1 无 panic 面 by-construction、
  裸指针 documented unsafe FFI 边界、**严禁 UB 节**（一切运行期语义 well-defined，RFC-0014 §4.A6）。

跨 ABI 运行期契约 **strict-only、well-defined、严禁 UB 节**（10 §7.5 高敏面：本 subset v1 首次容许
caller-responsibility 裸指针面，codegen 侧不引入额外 UB，框为 documented unsafe 边界非 UB，§RXS-0255）。

**编号区间**：本文件条款自 **RXS-0250** 起续号（全 spec 唯一、分配制递增、永不复用，见 [README.md](README.md)
§1；最高现存 RXS-0249 @ [dxil_backend.md](dxil_backend.md)，`registry/number_ledger.json` `next_free` = 250；earmark
区间 **RXS-0250 ~ RXS-0269** 由 RFC-0014 分配、G3_CONTRACT §7 v1.1 固化）。本轮 Part A 落 **RXS-0250 ~ RXS-0255**
条款体（每条 ≥1 测试锚定 `//@ spec: RXS-####`，随 EI1.2 实现 PR 同落）；**RXS-0256 ~ RXS-0265** = Part B UC-05 最小
RHI / render graph，落新建 [rhi.md](rhi.md)（EI1.3+）；**RXS-0266 ~ RXS-0269 预留不落裸条款头**（close-out 作废
声明留痕，burned 机制，MR-0006/0007 先例）。区间登记于 [README.md](README.md) §4 文件清单（主循环统一收）。

## 2. 条款（RXS-0250 ~ RXS-0255）

> 每条按需分 Syntax / Legality / Dynamic Semantics / Implementation Requirements 节，**严禁 UB 节**（UB 为经 Full
> RFC 由 agent 自主落笔的高敏面，10 §7.5；本面全条 well-defined，裸指针面框为 documented unsafe 边界非 UB）。
> Legality 违例只**引用**错误码（§3 汇总），新错误码一律以占位「6xxx 段续号」表述、不硬编具体号；跨 ABI 边界的
> 指针生命周期 / 所有权语义以 **caller-responsibility 前置条件 + 确定性诊断**定义，不以 UB 表述。

### RXS-0250 `#[export(c)]` 属性语法与合法性

**Syntax**（属性形态，挂载于 host fn item）：

```
ExportAttr ::= "#[" "export" "(" "c" [ "," "name" "=" StringLit ] ")" "]"
ExportItem ::= ExportAttr "pub" "fn" Ident "(" [ CAbiParams ] ")" [ "->" CAbiRet ] Block
```

**Legality**：

- **仅 host `pub fn` 合法**：`#[export(c)]` 挂载对象须 `FnColor::Host`（沿 coloring，与 RX3001/RX3015 同色判定
  底座）**且** `pub` 可见性。以下四类 → **编译期 strict-only 拒**（属性误用）：
  - 挂 `device fn` / `kernel fn`（device 着色对象无 host C ABI 出口）；
  - 挂非 `pub` fn（导出符号须外部可见）；
  - 挂非 fn item（`struct` / `const` / `impl` 块 / `mod` 等）；
  - `name = "…"` 键/值非法（键名非 `name` / 值非 `StringLit` / 空串 / 含非法符号名字符）。
- **诊断码 RX6033**（属性挂载对象非法，`export_c.attr_target`）：所有挂载对象误用（device/kernel/着色阶段 fn、
  非 pub、非 fn item、`name=` 非法）统一走**专用码 RX6033**。RFC-0014 §5.1 曾预测「device/kernel 复用 coloring
  RX3015」，实现期裁为 **materialize 专用码而非复用 RX3015**——属性挂载对象错误（挂载点合法性）与跨色调用
  （RX3015 coloring）语义不同，精确诊断优先（error_codes.json v1.33，introduced_in EI1.2）。

**Dynamic Semantics**：

- `#[export(c)]` 属性无运行期语义（编译期导出标记）；合法性判定于 resolve/typeck，不进 codegen 后新增运行期行为。

**Implementation Requirements**：

- parser 桩（[`parser.rs`](../src/rurixc/src/parser.rs) 现解析 `#[export(c)]` 但零 codegen，parsed-but-inert）
  转正为规范化校验入 resolve/typeck：属性收集 → color/可见性/item-kind 三重守门 → 合法者入 typeck 导出集
  （§RXS-0252 单一事实源起点）；违例 strict 拒、无 fallback（P-01）。诊断 span 精确到属性或 fn 签名。

> 测试锚定：`conformance/export_c/accept/attr_host_pub_fn`（host `pub fn` + `#[export(c)]` 合法，0 诊断）+
> `conformance/export_c/accept/attr_name_override`（`name = "rurix_store_out"` 覆写合法）+
> reject `conformance/export_c/reject/attr_on_kernel_fn`（挂 kernel → RX6033）+
> `conformance/export_c/reject/attr_on_device_fn`（挂 device fn → RX6033）+
> `conformance/export_c/reject/attr_on_non_pub_fn`（非 pub → 属性误用占位）+
> `conformance/export_c/reject/attr_on_non_fn_item`（挂非 fn item → 属性误用占位）+
> `conformance/export_c/reject/attr_bad_name_key`（`name=` 键/值非法 → 属性误用占位）+ UI golden。

### RXS-0251 C 兼容签名子集 v1 与类型映射

**Syntax / 类型映射表**（导出函数参数/返回类型全集，subset v1）：

| Rurix 类型 | C 映射 | 备注 |
|---|---|---|
| 标量 `{i8/i16/i32/i64, u8/u16/u32/u64, f32/f64, bool}` | `int8_t…/uint…/float/double/bool` | 定宽整型 + IEEE 浮点，宽度/符号性/位型为真 ABI 契约（§RXS-0253 类型层往返） |
| 裸指针 `*mut T` / `*const T`（T ∈ 标量） | `T*` / `const T*` | **documented unsafe FFI boundary**（§RXS-0255）：codegen 侧不引入隐式解引用；体内解引用属用户 `unsafe`，指针有效性/对齐/别名为调用方前置条件 |
| unit `()`（仅返回位） | `void` | 无返回值魔数；错误信号由用户显式 `i32` 返回码承载（应用契约面，非语言） |

**Legality**：

- 导出签名（`#[export(c)]` fn 的全部参数与返回类型）须落 subset v1 上表。**子集 v1 外类型出现在导出签名 →
  编译期 strict-only 拒**（签名不兼容），封闭越界形态至少覆盖：`repr(C)` struct 按值 / 回调函数指针 / 数组
  按值 / 切片 / affine 句柄（`Buffer`/`Res`/`Context` 等 brand 化不透明句柄）。
- **诊断码 RX6031**（`export_c.subset`）：子集 v1 违例（签名或体）走 **RX6031**。RFC-0014 §5.1「签名不兼容」确定
  新码兑现;RXS-0255 的导出体可 panic 面违例**折入同码 RX6031**（「超出子集 v1〔签名或体〕」单一类别，少一新码，
  error_codes.json v1.33）。此判据是 `#[export(c)]` 的类型面守门，防未定义 ABI 布局静默逃逸；边界锁 §4
  升档留痕，超界（扩签名支持）登 **RD-035+**（加性方向，后期升级）。
- 裸指针面为 **caller-responsibility 前置条件**（有效性/对齐/别名，等价任意 C 库裸指针面，对齐 RXS-0125 口径）；
  Rurix 侧 codegen 不引入额外 UB（§RXS-0255）。

**Dynamic Semantics**：

- 标量按定宽 C 类型逐位传递；`*mut T`/`*const T` 传裸地址（调用方自持缓冲，DLL 不分配-并-返回，§RXS-0255
  CRT-boundary 不变量）；unit 返回映射 `void`。跨 ABI 数值语义 well-defined，无 UB 节。

**Implementation Requirements**：

- 签名子集校验于 typeck，与 §RXS-0250 属性守门同管线；合法者的 C 映射（类型/宽度/符号性/调用约定）为
  §RXS-0252 `/EXPORT:` 发射与 §RXS-0253 头生成的**同一份单一事实源**（RFC-0014 §4.0-1）。越界类型走确定性
  Err（**RX6031**）、无 fallback。

> 测试锚定：`conformance/export_c/accept/sig_scalars_ptr_unit`（标量 + `*mut T`/`*const T`(T∈标量) + unit 返回
> 全子集 v1 合法，0 诊断）+ reject `conformance/export_c/reject/sig_struct_by_value`（`repr(C)` struct 按值）+
> `conformance/export_c/reject/sig_callback_ptr`（回调函数指针）+ `conformance/export_c/reject/sig_array_by_value`
> （数组按值）+ `conformance/export_c/reject/sig_slice`（切片）+ `conformance/export_c/reject/sig_affine_handle`
> （affine 句柄逃逸）——全部子集 v1 违例 → **RX6031**。

### RXS-0252 导出符号表与 cdylib 产物（link.exe `/EXPORT:` + `--emit=dll`）

**Syntax**（CLI 形态）：

```
DllEmit ::= "rx" "build" <entry> "--emit=dll" [ "-o" <out> ]
```

**Legality**：

- `--emit` 目标须为已识别集合 `{check, mir, llvm-ir, nvptx-ir, ptx, pyd, dll}`（`dll` 入既有 device_emit 分支
  match，[`driver.rs`](../src/rurixc/src/driver.rs):206-216 免-main 先例）；未识别目标维持拒（driver.rs 现纪律，
  不静默落入 host EXE 路径）。
- **空导出集拒**：`--emit=dll` 但零 `#[export(c)]` 导出 → **编译期拒**（空导出集）。**诊断码 RX6032**
  （`export_c.empty`，driver 层 emit）：RFC-0014 §5.1「空导出集」确定新码兑现（error_codes.json v1.33）。
- **DLL 链接失败**（link.exe `/DLL` 退出码非零）→ 诊断。**诊断码占位（不硬编，复用评估）**：**优先复用 RX7001**
  （外部工具链失败，含 link.exe 退出非零）/ **RX7022** 同族（`#[link]` 失败事后归因）——仅当确需 export-table
  上下文专诊断才另立 **RX7023+ 工具段**（RFC-0014 §5.1「DLL 链接失败」条件新码，correctness 评审 F2）；实号以
  合并时 registry 为准。

**Dynamic Semantics**：

- **保名不 mangle**：导出符号名 = fn 名（或 `name=` 覆写值），不施 Rust name mangling（`dumpbin /EXPORTS` 见裸名，
  spike 实证 `rurix_add`/`rurix_store` ordinal 1/2 未 mangle）。
- **发射机制 = driver 从 typeck 导出集拼 `/EXPORT:name` 传 link.exe**（**非 obj 内 `dllexport` 源标注**）：与内建头
  生成**同源单一事实源**（typeck C 映射既产 `/EXPORT` 参数又产 `.h`，RFC-0014 §4.0-1；否决 obj dllexport 见
  RFC-0014 §7-1 两事实源漂移风险）。**单一事实源正确性不止名字层**：`/EXPORT:` 只承载符号名，`.h` 承载完整 C
  签名（类型/宽度/符号性/调用约定）——后者才是真正的 ABI 契约，其正确性由**类型层 ABI 往返 conformance**
  （§RXS-0253）端到端机验，非仅 `dumpbin` 名字集 bijection（redline 评审 F6）。
- **cdylib 通道**：`--emit=dll` 走 cdylib 语义（无 `main`，`/DLL`）——driver link 段（driver.rs:524-604 现产 EXE
  `/subsystem:console`）扩展为 `/DLL` + 拼 `/EXPORT:` 序列 + 生成 import lib（link.exe 副产 `<name>.dll.lib` +
  `.exp`）。C 调用方 `#include` 生成头 + 链 import lib 即用。

**Implementation Requirements**：

- **CRT + 跨堆所有权红线**：`--emit=dll` 用 libcmt（静态）；subset v1 **无任何堆/资源所有权跨 ABI 边界**（调用方
  自持全部缓冲，DLL 不分配-并-返回，§RXS-0251/§RXS-0255）——故 `rurix_rhi.dll`（静态 libcmt）与异宿主（可能
  `/MD` 动态 CRT）是否同源 CRT 对内存正确性**无影响**，跨堆 `malloc`/`free` 配对陷阱（[interop.md](interop.md)
  「Windows DLL 陷阱纪律」先例）在 v1 by-construction 不触发（红线登 §4，扩签名让 DLL 返回分配物前必先解，redline
  评审 F2）。
- **rurixc 集成路首验闸口**：既有 `build/spike-emit-dll/` rustc cdylib spike 仅证 DLL/import-lib/导出表工具链在位；
  rurixc 自身「clang obj（无 dllexport 标注）+ driver 拼 `/EXPORT:`」集成路系 EI1.2 首验（impl 评审 EI1-IMPL-01），
  实现 PR 首 commit 走 rurixc 产物取证（非再跑 rustc）。

> 测试锚定：`conformance/export_c/accept/emit_dll_two_exports`（`.rx` fixture → DLL + import lib 产出单测 + `dumpbin
> /EXPORTS` 未 mangle 断言，名字集 ↔ 生成头声明集逐一对应）+ **per-type ABI 往返 conformance 哨兵值断言（步骤 71
> 硬门，redline F6）**：`conformance/export_c/accept/abi_roundtrip_i64_width`（i64 传 >2³² 验宽度）/
> `conformance/export_c/accept/abi_roundtrip_signext_i8_i32`（i8·i32 负值验符号扩展）/
> `conformance/export_c/accept/abi_roundtrip_f32_f64_bits`（f32 vs f64 验位型）/
> `conformance/export_c/accept/abi_roundtrip_ptr_rw`（`*mut`/`*const` 读写）——生成头声明宽度/符号性错误即**数值
> 红**（非仅 evidence）；+ reject `conformance/export_c/reject/emit_dll_empty_exports`（空导出集 → 拒，**RX6032**）。

### RXS-0253 内建头文件生成（确定性 + 单一事实源）

**Syntax**（产物形态）：

```
rx build <entry> --emit=dll -o <out>.dll
# 产出：<out>.dll + <out>.dll.lib（import lib）+ <out>.h（生成头，LF/无时间戳/幂等）
```

**Legality**：

- 生成头每个声明 ↔ 恰一个 DLL 导出符号（无悬空声明 / 无未声明导出，承 RXS-0149 逐一对应口径）；声明的 C 类型
  映射与 typeck 导出集（§RXS-0251 单一事实源）逐字段一致。

**Dynamic Semantics**：

- **确定性生成器**（cbindgen 内置化，D-113/P-11）：从 typeck 导出集确定性产 `<out>.h`——**LF 行尾、无时间戳、无
  绝对路径、两次逐字节一致**（幂等判据，RFC-0014 §9 Q-HeaderIdem）。头文本不含构建时刻/机器路径/工具版本等非
  确定性字段。

**Implementation Requirements**：

- 单一事实源 = typeck 的 C 映射（§RXS-0252 `/EXPORT:` 发射与本头生成同源）；生成器为纯函数 over 导出集，同源两次
  生成逐字节一致。**类型层 ABI 往返硬门**（redline F6）：生成头的类型映射（宽度/符号性/位型）经 §RXS-0252 per-type
  哨兵值穿头往返端到端机验——名字层 bijection 不足证 ABI 契约，错宽映射（如 `i64`→`int32_t`）在名字层 bijection +
  `cl.exe include` 编译下全绿却静默逃逸 = 边界 UB，故类型层往返为必需门。

> 测试锚定：`conformance/export_c/accept/header_idempotent`（同源两次生成 `<out>.h` 逐字节一致，byte-eq 幂等单测）+
> `conformance/export_c/accept/header_c_include_compile`（C 宿主 `#include` 生成头 + `cl.exe` 编译 + 链 import lib
> 真跑，步骤 71）+ 类型层 ABI 往返复用 §RXS-0252 `abi_roundtrip_*` 哨兵语料（i64 宽度 / i8·i32 符号扩展 / f32-f64
> 位型 / `*mut`·`*const` 读写穿生成头往返，宽度/符号性错即数值红）。

### RXS-0254 头↔ABI 守卫共存判据（手写路冻结 + 生成路 CI 再生成）

**Legality**（两制共存守卫）：

- **手写路 RXS-0149 冻结覆盖 Rust crate 出口**：既有 RXS-0149（[engine_integration.md](engine_integration.md)，
  `src/rurix-engine` `EXPORTED_C_ABI` 手写三符号 ↔ 随附手写 `rurix_engine.h`，步骤 43 host 段守卫）**语义 0-byte
  只增**——每 PR git diff 核 `src/rurix-engine` 三符号面 / 手写头 0-byte（EI1_CONTRACT §6.4 全期硬约束）。
- **生成路 `#[export(c)]` = CI 再生成逐字节比对覆盖 `.rx` 出口**：入库生成头被篡改一字节 → CI 再生成 byte-diff
  红（步骤 71 RED 路之一）。两守卫共存，判据升级与本条款同 PR。

**Implementation Requirements**：

- 生成路守卫 = 再生成 `<out>.h` 与入库副本逐字节比对（差异即红）；手写路守卫复用 RXS-0149 步骤 43 既有 0-byte
  只增门（不改其条款、不改 `src/rurix-engine`）。两制事实源相互独立（手写 = crate 出口；生成 = `.rx` 出口），无
  交叉写。

> 测试锚定：`conformance/export_c/reject/header_tamper_byte_diff`（篡改入库生成头一字节 → 再生成 byte-diff 红，
> 步骤 71 RED）；手写路 RXS-0149 步骤 43 三符号面 / 手写头 0-byte 只增门复用既有守卫（不新增语料）。

### RXS-0255 跨 ABI 契约（subset v1 无 panic 面 by-construction + 裸指针 documented unsafe + 严禁 UB）

**Legality**（编译期结构性保证）：

- **无 panic 面 by-construction**（跨 ABI panic 契约的诚实兑现，redline 评审 F1）：subset v1 导出体仅 C 兼容算术
  （标量 + 裸指针 + unit），**结构上不含任何可 panic 面**。`#[export(c)]` 导出函数体**禁含可 panic 面**（整数
  溢出检查触发点 / 数组越界 / 显式 `panic!` 等），违者 **编译期 strict-only 拒**。**诊断码 RX6031**
  （`export_c.subset`，与签名子集违例同码——「超出子集 v1〔签名或体〕」单一类别，error_codes.json v1.33）：
  首期 panic 面检出 = 数组/切片索引 / `?` / `unwrap`·`expect`。「不 unwind 穿 C 帧」（C 侧 UB）由此 by-construction
  保证。
- **裸指针责任边界 = documented unsafe FFI boundary**（对齐 RXS-0125 口径，redline 评审 F5）：`*mut T`/`*const T`
  的有效性/对齐/别名为调用方（C 侧）前置条件，导出契约不承诺（等价任意 C 库裸指针面）——「不解引用语义承诺」
  确切含义 = **codegen 侧不引入隐式解引用；体内解引用属用户 `unsafe`**（§3.1 `store` 示例），其内存正确性为调用方
  责任。Rurix 侧 codegen 不引入额外 UB，故与「全条无 UB 表述」不冲突（框为 documented unsafe 边界，非 UB）；不
  引入「有界非确定」内存序断言。

**Dynamic Semantics**：

- 一切运行期语义 **well-defined**，全条无 UB 措辞：unit 返回映射 `void`，无返回值魔数；错误信号由用户显式 `i32`
  返回码承载。跨 ABI 边界无堆/资源所有权转移（§RXS-0252 CRT-boundary 不变量），调用方自持全部缓冲。
- **明确删去**无法构造的运行期「panic-across-ABI 确定性终止单测」（redline 评审 F1：subset v1 无 panic 面即无可
  测运行期路径，那是 YAML-only 空条款）——契约以编译期结构性保证（禁含可 panic 面）兑现，非运行期终止单测。

**Implementation Requirements**：

- 导出体可 panic 面检测于编译期（`collect_c_exports` AST 体扫描识别数组/切片索引 / `?` / `unwrap`·`expect`），命中
  即 strict 拒（**RX6031**）、无 fallback（P-01，反 YAML-only §6.1）。后期扩签名若引入 panic 面（RD-035+），届时
  另钉唯一终止机制（推荐 per-export `catch_unwind→abort` 边界 shim，避免全程 `-C panic=abort` 污染 EXE 路）+ 补 RED
  语料。10 §7.5 无 UB 口径。

> 测试锚定：reject `conformance/export_c/reject/export_body_int_overflow_check`（导出体含整数溢出检查可 panic 面 →
> 编译期拒，步骤 71 RED）+ `conformance/export_c/reject/export_body_array_index`（数组越界索引可 panic 面 → 拒）+
> `conformance/export_c/reject/export_body_explicit_panic`（显式 `panic!` → 拒）+ accept
> `conformance/export_c/accept/no_panic_surface_arith`（纯 C 兼容算术导出体，无可 panic 面，0 诊断）+ 无 UB 措辞核
> （**删去无法构造的运行期 panic-across-ABI 终止单测**，redline F1）。可 panic 面违例码 = **RX6031**（与签名子集同码）。

## 3. 错误码引用汇总

> 本表**引用**本文件条款涉及的错误码。含义以 [registry/error_codes.json](../registry/error_codes.json) 为唯一事实
> 源；message-key 落 [`en.messages`](../src/rurixc/src/messages/en.messages) / [`zh.messages`](../src/rurixc/src/messages/zh.messages)
> （en/zh 成对，bilingual 门）。**新错误码一律占位「6xxx 段续号」，不硬编具体号**（避免与主循环 wiring 撞号；
> 6xxx 段自 RX6031 续〔main 现最高 RX6030 active，RXS-0237〕，7xxx 工具段自 RX7023 续〔`number_ledger` `next_free`
> = 7023〕；实号以合并时 registry 为准，registry 只追加，10 §6）。RFC-0014 §5.1 预测**需新码 ≤4**（确定 ×2 = 签名
> 不兼容 / 空导出集；条件 ×2 = 属性误用〔复用 RX3015 或另立〕/ DLL 链接失败〔复用评估 RX7001/RX7022〕）。

| 码（拟） | 段 | 语义 | 复用/新码 | 条款 |
|---|---|---|---|---|
| RX3015（复用候选） | 3xxx 着色 | `#[export(c)]` 挂 device/kernel fn（宿主/设备着色边界，与 RXS-0189 同点位，复用口径） | 复用（实现期判是否独立可达） | RXS-0250 |
| 6xxx 段续号（占位） | 6xxx 装配/codegen | `#[export(c)]` 属性误用（非 pub / 非 fn item / `name=` 键值非法） | 条件新码 ×1（或并入 RX3015） | RXS-0250 |
| 6xxx 段续号（占位） | 6xxx 装配/codegen | C 兼容签名子集 v1 违例（子集外类型出现在导出签名） | 确定新码 ×1 | RXS-0251 |
| 6xxx 段续号（占位） | 6xxx 装配/codegen | 空导出集（`--emit=dll` 零 `#[export(c)]` 导出） | 确定新码 ×1 | RXS-0252 |
| RX7001 / RX7022（复用候选） | 7xxx 工具链 | DLL 链接失败（link.exe `/DLL` 退出码非零，外部工具链 / `#[link]` 失败同族） | 复用优先，仅需专诊断才另立 RX7023+ | RXS-0252 |
| 6xxx 段续号（占位） | 6xxx 装配/codegen | 导出体含可 panic 面（整数溢出检查 / 数组越界 / 显式 `panic!`） | 条件新码 ×1 | RXS-0255 |

**运行期/环境失败**（panic-across-ABI 终止〔subset v1 by-construction 不可达〕、device 分配失败、DLL 装载失败）
一律**不占 RX 码**（06 §8.2 / RXS-0193 口径，确定性诊断 + 终止）。

## 4. 升档 / 禁区留痕

- **C 兼容签名子集 v1 边界锁（超界登 RD-035+）**：`export_c_extended_signatures`（`repr(C)` struct 按值 / 回调函数
  指针 / 数组按值 / 切片）为首期范围红线——struct 按值 ABI 布局（对齐/填充/寄存器分类）与回调指针（跨 ABI 调用
  约定）面显著扩风险，超首期 correctness-only 目标（RFC-0014 §8）；硬需求出现按 10 §3 判档，合入时 deferred.json
  续号 **RD-035+**（加性方向，不预留）。
- **跨堆/资源所有权红线（`cross_boundary_heap_ownership`）**：subset v1 **no cross-boundary heap/resource ownership
  transfer**——调用方自持全部缓冲、DLL 不分配-并-返回（§RXS-0252/§RXS-0255），静态 libcmt 与异宿主 CRT 不同源在 v1
  无害，跨堆 `malloc`/`free` 配对陷阱 by-construction 不触发（[interop.md](interop.md)「Windows DLL 陷阱纪律」先例，
  redline 评审 F2）。**扩签名让 DLL 返回分配物前必先解此不变量**（RD-035+ 前置守门，合入时 deferred.json 续号）。
- **ABI 稳定面对账 P-10（RFC-0014 §10，redline 评审 F4）**：区分两层——（i）**单个 DLL 的符号字节布局 / 生成头
  ABI / 工具版本 = 非稳定**，维持 **RXS-0180 L3** 口径（[edition.md](edition.md)；符号面是工具链内部实现要求，非
  用户 stable ABI，镜像 RXS-0149/0162/0165 先例）；（ii）**导出约定本身（命名不 mangle 规则 + subset v1 类型映射 +
  调用约定）= 10 §6 P-10「C ABI 导出约定」1.0 stable 面候选**（用户会依赖），其稳定化经 **RD-008** 定型——非把整个
  C ABI 导出面一并判为非稳定。`abi_stability_promise` carve-out（RFC-0014 §8）仅冻结 (i) 层字节布局不作语言 ABI
  承诺，不否认 (ii) 层约定属 P-10 stable 面候选。FCP-lite（advisory 公开等待窗）下公开，agent 自主裁决合入。
- **UB 节禁区**：跨 ABI 边界的指针生命周期 / 所有权语义以 **caller-responsibility 前置条件 + 确定性诊断**定义，
  **严禁 UB 节**（UB 为经 Full RFC 由 agent 自主落笔的高敏面，10 §7.5）；subset v1 首次容许 caller-responsibility
  裸指针面，Rurix 侧 codegen 不引入额外 UB，框为 documented unsafe 边界（§RXS-0255），全条 well-defined。
- **新错误码不预造具体号**：本文件条款体先落、错误码占位「6xxx 段续号 / RX7023+」，落码 + message-key（en/zh 成对）
  + conformance reject 语料 + CI 步骤 71 归 EI1.2 实现 PR（`PR-A`，条款+前端+后端+CI 不可分，RFC-0014 §6.3）；实号以
  合并时 registry 为准（不预留不预造，07 §5）。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-07-19 | 新建 spec/export_c.md（EI1.2 Part A，`PR-A` 条款 commit 先于实现 commit、单 PR 不可分）：带编号条款体 `### RXS-0250 ~ ### RXS-0255`（FLS 体例，按需分 Syntax/Legality/Dynamic Semantics/Implementation Requirements，**严禁 UB 节**）——RXS-0250 `#[export(c)]` 属性语法与合法性（仅 host `pub fn`；挂 device/kernel/着色阶段 fn / 非 pub / 非 fn item / `name=` 键值非法 → 编译期 strict-only 拒 **RX6033**）/ RXS-0251 C 兼容签名子集 v1 与类型映射（标量 + 裸指针〔T∈标量，documented unsafe 边界〕+ unit；子集外类型编译期拒 **RX6031**，边界锁超界登 RD-035+）/ RXS-0252 导出符号表与 cdylib 产物（保名不 mangle + driver 从 typeck 导出集拼 link.exe `/EXPORT:` + `--emit=dll` `/DLL` + import lib；空导出集拒 **RX6032**；DLL 链接失败复用 RX7001/RX7022；per-type ABI 往返哨兵值步骤 71 硬门 redline F6）/ RXS-0253 内建头文件生成（确定性 LF/无时间戳/无绝对路径/两次逐字节一致幂等；单一事实源 = typeck C 映射；每声明↔恰一 DLL 导出符号承 RXS-0149）/ RXS-0254 头↔ABI 守卫共存（手写路 RXS-0149 冻结覆盖 Rust crate 出口 0-byte 只增；生成路 CI 再生成逐字节比对覆盖 `.rx` 出口，篡改一字节 byte-diff 红）/ RXS-0255 跨 ABI 契约（subset v1 无 panic 面 by-construction = 编译期结构性保证〔导出体禁含可 panic 面 → **RX6031**，与签名子集同码〕+ 裸指针 documented unsafe 边界 + 严禁 UB；删去无法构造的运行期 panic-across-ABI 终止单测 redline F1）。§1 编号区间登记 RXS-0250 起（earmark 0250~0269，Part A 落 0250~0255，Part B → rhi.md 0256~0265，0266~0269 预留 burned）；§4 升档留痕（subset v1 边界锁 RD-035+ / 跨堆所有权红线 RD-035+ 前置守门 / P-10 对账 RXS-0180 L3+RD-008 / UB 禁区）。**本 PR 全接线**（条款 + 前端 attr 校验 `export_c.rs::collect_c_exports` + 签名子集 + panic 面 AST 扫描 + driver `--emit=dll` 免-main 分支 + `/EXPORT:` 发射 + import lib + 内建头生成器 + MIR 导出根收集 `build_export_crate` + registry 新码 RX6031/RX6032/RX6033〔error_codes.json v1.33，RFC-0014 §5.1「确定 ×2 + 条件 ×2」兑现:确定 = RX6031/RX6032,属性误用 materialize 专用码 RX6033〕+ 双语 message-key + conformance/export_c accept·reject 语料 + corpus 测试 + 步骤 71 `ci/export_c_smoke.py` + 每条 ≥1 `//@ spec:` 锚定，EA1 #158/#159 单 PR 条款先行先例，RFC-0014 §6.3）。承 [RFC-0014](../rfcs/0014-engine-integration.md)（Agent Approved 2026-07-19，§4.A 全文批准，RD-009 兑现；评审 provenance ≠ 起草，D-409）。 | **Full RFC**（RFC-0014 / EI1.2 / PR-A） |
