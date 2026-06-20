# unsafe-audit: rurix-engine（引擎集成 C ABI 导出边界）

> 注册依据：AGENTS.md 硬规则 9 / 10 §7.6（无注册条目的 unsafe 是 CI 错误）；
> 14 §2 常驻集 unsafe-audit 完整性。G1.3 激活（D-G1-3 首个引擎集成落地，C ABI 导出边界）。
> 决策依据：D-113（FFI 战略：C ABI 唯一，`extern "C"` 导出，Windows x64 唯一 ABI）、
> 06 §8.3（引擎级工作流 U5 服务承诺，UC-05 前奏）、05 §11（C ABI 导出 = DLL 导出表条目）。
> G1 契约 `rfc_required: none`；新决策面引擎集成判档带档位标记 **Mini-RFC**（[MR-0002](../rfcs/mini-0002-engine-integration.md)，
> owner 2026-06-20 经 AskUserQuestion 裁决）——复用 M8.1 既有 C ABI（RXS-0125）不扩 ABI 表面，
> 直接实现 + C ABI 导出属性块级豁免（不另走 RFC）。spec 条款：RXS-0149。

## 范围与豁免

- crate：`src/rurix-engine`（`[lints.rust] unsafe_code = "allow"`；`undocumented_unsafe_blocks
  = "deny"` 维持）。
- 全仓其余新 crate 维持 `unsafe_code = "deny"`（根 workspace 默认），不受影响。
- **本 crate 无 unsafe 块**：`unsafe_code = "allow"` 仅为 C ABI 导出属性
  `#[unsafe(no_mangle)] pub extern "C" fn rurix_engine_*`（编译为 `cdylib` DLL 导出表条目）所需；
  `ffi.rs` 导出层**前向** `rurix-interop` safe API（`saxpy`/`reduce`，RXS-0125），设备指针为不透明
  `u64` 地址，**不在本层解引用**。实际借用外部设备指针的 unsafe 原语在 `rurix-interop`→`rurix-rt`
  （`from_device_ptr` 借用缓冲 Drop 不 free，见 `unsafe-audit/rurix-rt.md` U9/U10），本 crate
  不重复其义务。

## 原语清单与验证义务（RustBelt 式）

| # | 原语 | 位置 | 验证义务（SAFETY 不变量） |
|---|---|---|---|
| U21 | C ABI 导出属性 `#[unsafe(no_mangle)] pub extern "C" fn`（DLL 导出表条目） | `src/rurix-engine/src/ffi.rs` `rurix_engine_{abi_version,compute_saxpy,compute_reduce}` | 导出符号名以 `rurix_engine_` 前缀唯一、与既有 C ABI 导出（`rurix_uc01_*`，RXS-0125）**不冲突**（no_mangle 符号唯一性）；签名为 C ABI（`extern "C"` + 标量 / 不透明 `u64` 设备指针按值），与随附头文件 `include/rurix_engine.h` 声明逐一对应（RXS-0149，`crate::tests::c_abi_header_matches_exports` 守卫）；**本层无 `unsafe` 块、不解引用设备指针**——参数按值前向 `rurix-interop` safe API（`saxpy`/`reduce`，对上全 safe），实际设备指针借用义务在 rurix-rt U10（借用缓冲 Drop 不 free，所有权留外部/宿主，不双重释放） |

## 销毁纪律

本 crate 无所有权资源、无 Drop、无 unsafe 块；compute pass 入口为无状态前向。设备内存所有权在
**宿主 C++/D3D12 框架**（经 C ABI 传入的设备指针在 device primary context 内分配，借用期内宿主
持有不释放，对齐 UC-01 零拷贝设备指针约定）；引擎 DLL **不捆绑 NVIDIA 组件**（运行时经 rurix-rt
动态加载 `nvcuda.dll`），D3D12/DXGI 系 Windows SDK 系统组件（NVIDIA 再分发白名单审计延续，r6）。

## 测试

- `cargo test -p rurix-engine`：RXS-0149 单测锚定（`c_abi_header_matches_exports`：随附头文件声明集
  ↔ `EXPORTED_C_ABI` 导出集逐一对应 + 编译期 C ABI 签名引用；`compute_pass_reuses_interop_diagnostics`：
  复用 RXS-0125 诊断段位，设备指针非法 / 维度 0 先于 GPU 确定性拦截，host 上不触 GPU 的校验）。
- 引擎集成端到端数值对照 + 内建篡改红绿真跑见 `ci/engine_integration_smoke.py`（步骤 43，自建最小
  C++/D3D12 harness 经 C ABI 调 Rurix DLL compute pass）/ G1 CI_GATES §2 步骤 43 / close-out run URL。
