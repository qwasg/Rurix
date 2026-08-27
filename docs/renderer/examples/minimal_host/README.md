<!-- Assisted-by: TraeCode:Kimi-K3（G31+ 波 C Task C2 渲染器文档与示例） -->
# minimal_host — Rurix 渲染器 C ABI 最小宿主示例

[integration_guide.md §4](../../integration_guide.md)「最小集成五步」的可执行见证：
`.rx` 导出面 → `rurix_rhi.dll` + import lib + 生成头 → 本目录 `minimal_host.cpp`
（C++ 宿主，仅 `<cstdint>/<cstdio>` + 生成头）编译链接 → 真 GPU 跑 4 帧图形图。

> C1 SDK API 面在飞未定型——本例以 **EI1 UC-05 既有 C ABI 面**（RXS-0250~0255 + RXS-0261/0277）为准。

## 文件

| 文件 | 作用 |
|---|---|
| `minimal_host.cpp` | 最小宿主源（五步同号注释段；退出码 0 = 全绿） |
| `build.ps1` | 一键构建+真跑（工具链定位 → emit DLL → cl 编译 → 运行 + 输出断言） |
| `build/` | 构建产物目录（脚本现场生成；`rurix_rhi.dll/.lib/.h` + `minimal_host.exe`）——**生成头自始生成不手写，不入库**（RXS-0253/0254） |

## 前提

Windows 11 + NVIDIA GPU（Vulkan 可用）+ clang 22.1.x（`C:\Program Files\LLVM` 或 `RURIXC_CLANG`）+ MSVC 2022 + Windows SDK + Rust 工具链（首跑需 `cargo build -p rurixc`，脚本自动）。详见集成指南 §2。

## 一键走通

```powershell
powershell -ExecutionPolicy Bypass -File docs\renderer\examples\minimal_host\build.ps1
```

预期输出（末两行）：

```text
RURIX_MINIMAL_HOST_OK passes=2 frames=4 pixel=0x00000000
[minimal_host] PASS 五步最小集成走通（产物：…\build）
```

`pixel=0x00000000` = 清色不变量（空着色器无颜色写入，RXS-0277 Q-PixelCriterion
纯色 RGBA8 整数 fetch 域；不设 ULP 容差）。

> **预期内 stderr note**（实测走通记录）：emit 阶段打印两条 `SPIR-V lowering failed …
> (RXS-0291)` 信息性 note（gfx 演示着色器 position builtin 形态，产物照常生成）；
> 进度日志走 stderr（PowerShell 可能显示为 NativeCommandError 样式）——均以退出码为准。

## 手动逐步（教学拆解）

```powershell
# 仓库根
# 1. 初始化：.rx 导出面 → DLL 三件（rurixc 自定位 link.exe/MSVC 库；clang 经 RURIXC_CLANG）
target\debug\rurixc.exe apps\uc05-rhi\src\embed.rx --emit=dll -o docs\renderer\examples\minimal_host\build\rurix_rhi
#    产物：rurix_rhi.dll + rurix_rhi.lib（import lib）+ rurix_rhi.h（生成头）

# 2. 宿主编译（x64 Native Tools 或已设 INCLUDE/LIB 的 MSVC 环境）
cl /std:c++17 /EHsc /I docs\renderer\examples\minimal_host\build `
   docs\renderer\examples\minimal_host\minimal_host.cpp `
   /Fe:docs\renderer\examples\minimal_host\build\minimal_host.exe `
   /link /LIBPATH:docs\renderer\examples\minimal_host\build rurix_rhi.lib

# 3. 真跑（rurix_rhi.dll 与 exe 同目录；需 Vulkan GPU）
docs\renderer\examples\minimal_host\build\minimal_host.exe
```

## 与五步映射（minimal_host.cpp 内同号注释）

1. **初始化**：`uc05_gfx_pass_count()` 自检（纯常量不触 GPU；头↔DLL 调用面通达核对）。
2. **场景**：图声明封闭在导出体内（宿主以标量 `w/h` 参数化，不见 Rurix 类型）。
3. **帧循环**：4 帧 `uc05_gfx_run_frame(&pixel, 64, 64)`（每帧装配核验 + 真派发 + 真 D2H）。
4. **参数/错误**：负例 `w=0 → rc 2` 先行；i32 状态码闭集；裸指针 = 调用方前置条件。
5. **关闭**：资源随单次调用拆除；无 shutdown 导出；宿主直接退出。

## 三态纪律

缺 rurixc/clang/MSVC/Vulkan GPU → 脚本打印 `SKIP DEV_ENV_DEGRADE: <原因>` 退 0
（**不冒充 PASS**）；`RURIX_REQUIRE_REAL=1` 下降级翻硬 FAIL 退 1。

## 实际走通记录

本示例由 Task C2 walkthrough 真机走通验证（RTX 4070 Ti + Vulkan，步骤数/耗时/卡点
逐字登记）：`milestones/g31/g31_renderer_docs_walkthrough.json`。
