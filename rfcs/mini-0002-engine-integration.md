# Mini-RFC MR-0002 — 首个引擎集成（Rurix DLL 经 C ABI 嵌入 C++/D3D12 harness 承担 compute pass）

| 字段 | 值 |
|---|---|
| Mini-RFC 标识 | **MR-0002**（Mini-RFC 序列；独立于 Full-RFC 的 `RFC-####` 命名空间，不复用 RFC 编号，10 §9.5。Mini-RFC = 单页提案 + 失败测试先行 + agent 自主批准，10 §3） |
| 标题 | 首个引擎集成：Rurix DLL（C ABI）嵌入自建最小 C++/D3D12 渲染 harness 承担 compute pass（UC-05 前奏） |
| 档位 | **Mini-RFC**（10 §3：**复用** M8.1 既有手写 `extern "C"` C ABI（RXS-0125，语义 0-byte）+ `cdylib` DLL 打包 + 自建最小 C++/D3D12 harness「内部开关 / 工具行为」量级；**不扩 C ABI / ABI 表面、不实现 `#[export(c)]` 编译器 codegen、不触内存模型映射 / 新 FFI ABI 禁区**——见 §3）。agent 自主 裁为 Mini-RFC（2026-06-20；「C ABI 复用 vs `#[export(c)]` 实现」与「宿主 C++/D3D12 框架选型」为 G1 执行期新决策面，向上取严，agent 自主判档） |
| 状态 | **Approved — 2026-06-20**（agent 于本工作会话经 AskUserQuestion 明确裁决 §3 判档 = **Mini-RFC（复用 C ABI）** + §2 宿主 = **自建最小 C++/D3D12 harness**；§4 零新 RX 码 / §5 `#[export(c)] ` codegen defer / §6 范围沿 G1.1/G1.2 既有先例。批准记录由 claude-code **代录**，非 AI 代签 / 自判，AGENTS 硬规则 1。实现 PR 终审、RD-009 登记确认、crate 命名、device 真跑 / 证据回填 / 计数器兑现仍由 agent 自主签署） |
| 承接里程碑 | G1.3（验收门 **G-G1-3**），G1 第三子里程碑 |
| 关联条款 | 拟落 spec **RXS-0149**（区间随条款数定，§2）；新建 `spec/engine_integration.md` |
| 依据决策 | **D-113**（FFI：C ABI 唯一；导出走 `#[export(c)]` + 内建头文件生成，cbindgen 角色内置化，05 §11）· **RXS-0125**（M8.1 既有 C ABI 边界，`src/rurix-interop`）· 06 §8.3（引擎级工作流 U5 服务承诺，UC-05 前奏）· 02 §U5（图形引擎开发者画像 + UC-05 采纳判据：C ABI FFI 成熟 + 增量 check `<5s`）· **RFC-0001**（G1.1 interop 呈现通路复用）· **MR-0001**（Mini-RFC 先例） |
| Provenance | `Assisted-by: claude-code:claude-opus-4-8`。agent 自主：agent 批准前不推进下游实现 PR |
| 失败测试先行 | `ci/engine_integration_smoke.py` host 段「头↔ABI 1:1 闸门」（断言随附头文件声明集 == `cdylib` DLL 导出符号集）：实现 PR 落地前 `origin/main` 上 `rurix-engine` crate / 头文件 / 脚本均不存在 → **RED**；实现 PR 落地后步骤 43 host 段对不一致即红（应一致却漂移即红），证检查器能区分「一致 vs 漂移」（10 §3 Mini「必须先有失败测试」） |

---

## 1. 摘要

把 06 §8.3 已锁的「引擎级 compute pass 集成（U5 服务承诺，UC-05 前奏）」落到首个工程实现：**复用** M8.1 既有手写 `extern "C"` C ABI（`src/rurix-interop` `rurix_uc01_saxpy/reduce/gemm`，RXS-0125，**语义 0-byte**），把承担 compute pass 的 C ABI 编为 **`cdylib` DLL**（`rurix-cublas` 已证 cdylib 产 DLL 先例），并由一个**自建最小 C++/D3D12 渲染 harness** 经 C ABI 头文件 + import lib 链接该 DLL，在最小 render-graph 上下文中承担 **≥1 个 compute pass**，端到端**数值 / 呈现对照** + 对照 02 §U5 采纳判据（C ABI FFI 成熟 + 增量 check `<5s`）。

设计**最大化复用**——M8.1 C ABI（RXS-0125）+ G1.1 interop 呈现通路（`rurix-d3d12` shim / `rurix-rt::interop`）+ 既有 device kernel（saxpy/reduce 或 G0 软光栅 RXS-0118~0121，**语义 0-byte，仅经 C ABI 暴露**）——**不重新发明 C ABI、不实现 `#[export(c)]` 编译器 codegen**（D-113 的 cbindgen 内置化 codegen defer，见 §5 / RD-009）。

## 2. 设计（用户视角 + 形态）

宿主 C++/D3D12 框架是驱动方，Rurix DLL 是被调的 compute pass 提供方（与 G1.1「Rust 驱动 C++ shim」极性相反）：

```
自建最小 C++/D3D12 host harness（main，render-graph 上下文）
   → 经 include/rurix_engine.h + rurix_engine.dll.lib 链接 rurix_engine.dll
   → 设备 buffer（与 D3D12 同 adapter LUID，复用 G1.1 interop 路径）
   → 调 compute pass C ABI 入口（设备指针 u64 + 维度按值 → i32 错误码）
   → Rurix device kernel（saxpy/reduce 等，语义 0-byte）写输出 buffer
   → readback 数值对照（vs host 参考）+ 可选 D3D12 present 呈现对照
```

| 复用项 | 来源 | 形态 |
|---|---|---|
| C ABI compute（saxpy/reduce/gemm） | `rurix-interop` RXS-0125 | safe API 前向，**语义 0-byte** |
| device kernel（PTX 嵌入） | M5 自研 kernel / G0 软光栅 RXS-0118~0121 | build.rs 嵌入，**0-byte** |
| `cdylib` 产 DLL | `rurix-cublas` 先例 | crate-type `["cdylib","rlib"]` |
| D3D12/DXGI 呈现通路 | `rurix-d3d12` shim（G1.1，real-shim） | harness 复用 / 扩展 |
| interop 设备指针 / LUID | `rurix-rt::interop`（G1.1） | 设备 buffer 与 D3D12 同 adapter |

新增的语义面**仅**「引擎集成 DLL 打包约定 + C ABI 头文件与导出 ABI 逐一对应」（**RXS-0149**，§2 拟落）；compute pass 本身的 C ABI 形态、device kernel 语义、interop 类型面**全部既有、0-byte**。

## 3. 为何 Mini-RFC（而非 Direct，亦非 Full RFC）

- **非 Full RFC**：本设计**复用** RXS-0125 既有 C ABI，**不扩 ABI 表面、不新增导出语义、不引入跨边界新所有权语义**，故**不触** AGENTS 硬规则 5 / 10 §7.5「FFI ABI」禁区（区别于 G1.1：G1.1 因 D3D12/DXGI host shim 的**新** C ABI 面 + `cuImportExternal*` 绑定 + CUDA↔D3D12 内存模型映射裁 **Full RFC**）。`cdylib` 打包是稳定工具链能力（`rurix-cublas` 先例）；C++/D3D12 host harness 的 COM 复杂度**留在 C++ 不进语言**（对齐 D-130 shim 纪律）。
- **非 Direct**：`G1_CONTRACT` YAML 头 / CI_GATES §1 **显式**把「宿主 C++/D3D12 框架选型」列为 G1.3 执行期新决策面（agent 裁决留痕）；且「是否在本期实现 D-113 `#[export(c)]` codegen vs 复用 `extern "C"`」触及 C ABI / FFI 边界判档。AGENTS 硬规则 8「判档争议向上取严」+ MR-0001 对其自身新决策面（AsyncBuffer API 形态）走 Mini 的先例 → 走一页 Mini-RFC + 失败测试先行 + agent 批准。
- **升档触发条件（实现期守卫）**：若实现期发现 G-G1-3 **无法以复用既有 C ABI 达成**而确需 **扩 C ABI 表面 / 新导出语义 / 跨边界新所有权语义 / 实现 `#[export(c)]` 编译器 codegen**，则**停手升 Full RFC**（向上取严，镜像 G1.1 RFC-0001 因 FFI ABI 禁区裁 Full），不在 spec/impl 自行落笔。

## 4. 错误码

**零新 RX 码**（对齐 G1.1 RXS-0140~0143 / G1.2 RXS-0144~0148 零新码先例）：compute pass C ABI 入口返回 `i32`——`0` = 成功；复用既有互操作诊断段位 `RX7013`/`RX7014`/`RX7015`（RXS-0125，07 §5，只追加、含义冻结）；负 = 运行时 / 驱动失败。`registry/error_codes.json` 与 `en.messages` 零追加。若 device 段实现期某类别确需**新**运行期诊断段位码，则按 14 §4 + RX 段位制（7xxx 从 **RX7020** 起，M8.2 止于 RX7019）处置并停手标注，**不预造**。

## 5. `#[export(c)]` 编译器 codegen defer（RD-009）

D-113 锁定「导出走 `#[export(c)]` + 编译器内建头文件生成（cbindgen 角色内置化，05 §11 / P-11）」为 FFI 战略方向。当前实现现实：`#[export(c)]` **仅**作为解析测试桩出现（`src/rurixc/src/parser.rs`，parsed-but-inert），**无** DLL 导出表 codegen、**无**内建头文件生成（`rurixc --emit` 无 `header`/`pyd`）；生产 C ABI 由 `rurix-interop` 以手写 `extern "C"` + `#[unsafe(no_mangle)]` 兑现。

本期裁决：**复用** `extern "C"` C ABI + 与导出 ABI **1:1** 的**随附**头文件兑现 D-113 的「头文件单一事实源对应」方向；**`#[export(c)]` 编译器 codegen + 内建头文件生成实现 defer**（**RD-009**，registry/deferred.json append-only）。头↔ABI 一致性由 **RXS-0149** + CI 步骤 43 host 段守卫（漂移即红）。后续里程碑出现「需经 `rurixc` 自动产 DLL 导出表 + 单一事实源内建头文件」硬需求时接通，届时按 10 §3 判档（FFI ABI codegen 触硬规则 5 则Full RFC）。

## 6. 影响 / 向后兼容 / 范围

- **向后兼容**：纯追加。`rurix-interop` RXS-0122~0125 / device kernel RXS-0118~0121 / G1.1·G1.2 既有语义面 **0-byte**（仅新增引擎集成缺口）。
- **常驻回归网不依赖 device 而绿**：引擎边界 crate 的 `cdylib` 为纯 Rust 编译（无需 MSVC/GPU）；C++/D3D12 host harness 编译 + device 真跑经段 / feature 门控，默认 `cargo build/clippy/test --workspace` 不参与（镜像 `rurix-d3d12` stub/real-shim 与 uc03-demo present 门控先例）。
- **unsafe 边界（§7）**：引擎边界**新 crate 默认 `unsafe_code=deny`**，C ABI 导出边界经裁决最小开 + `unsafe-audit/` 注册（U21），safe wrapper 对上全 safe。
- **NVIDIA 再分发白名单延续**：引擎 DLL 不捆绑 NVIDIA 组件（运行时动态加载 `nvcuda.dll`，对齐 rurix-rt）；D3D12/DXGI 系 Windows SDK 系统组件，不受 NVIDIA 再分发约束（check_redistribution 延续，r6）。
- **范围红线**：本期 Rurix 仅承担 **compute pass**，**不进图形着色阶段 / DXIL 第二后端**（G2，06 §8.2 / D-131）；不做 Graph API 立项（agent 已裁 defer，registry 0-byte）/ VMM / 多 GPU / 多后端 / Python 原生嵌入（红线 1，SG-008）。

## 7. unsafe 边界

引擎边界 crate（`rurix-engine`，名可由 agent 调整）的 C ABI 导出（`#[unsafe(no_mangle)] pub extern "C" fn …` 前向 `rurix-interop` safe API）经裁决最小开 `unsafe_code`（镜像 `rurix-interop` / `rurix-cublas` FFI 边界 crate 先例）+ `unsafe-audit/rurix-engine.md` 注册条目（**U21**，接 G1.2 U19/U20 续号）；`undocumented_unsafe_blocks = deny` 维持；safe wrapper 层对上全 safe（签名无 `unsafe`）。其余新代码维持 `unsafe_code=deny`。

## 8. Agent 批准

> **Approved — 2026-06-20**。agent 于本工作会话经 AskUserQuestion 明确裁决：§3 判档 = **Mini-RFC（复用既有 C ABI，不实现 `#[export(c)]` codegen）**；§2 宿主框架 = **自建最小 C++/D3D12 harness**。§4 零新 RX 码 / §5 `#[export(c)]` codegen defer（RD-009）/ §6 范围 / §7 unsafe 边界沿 G1.1/G1.2 既有先例。批准记录由 claude-code 代录，**非 AI 代签 / 自行裁决**（AGENTS 硬规则 1）。实现 PR 终审、RD-009 登记确认、crate 命名、device 真跑 / 证据回填 / `g1.counter.engine_integration` 兑现 / 增量 check `<5s` measured 回填 / close-out 仍由 agent 自主签署。
