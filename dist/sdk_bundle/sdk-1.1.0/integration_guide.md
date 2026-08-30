<!-- Assisted-by: TraeCode:Kimi-K3（G31+ 波 C Task C2 渲染器文档与示例） -->
# Rurix 渲染器集成指南

> 所属：G31+ 波 C Task C2（渲染器文档与示例，G31_PLUS_COMMERCIAL_RENDERER_TODO §5 #49）。
> 读者：把 Rurix 渲染器集成进自己引擎/应用的开发者。
> 纪律：本文所有性能数字一律引用在案 measured_local 证据（RTX 4070 Ti + Vulkan 本机真跑），不新造数字；
> 各数字的来源文件随文标注。事实源冲突时以 `milestones/*/*_CONTRACT.md` 与 `registry/` 为准。
> 姊妹篇：[feature_matrix.md](feature_matrix.md)（pass/特性矩阵）· [performance_tuning.md](performance_tuning.md)（性能调优）· [examples/minimal_host/](examples/minimal_host/)（最小 C++ 宿主示例）。

---

## 1. 渲染器形态总览

Rurix 渲染器当前对外可消费的有三种形态，按集成深度从浅到深：

| 形态 | 入口 | 适合谁 | 状态 |
|---|---|---|---|
| **A. C ABI 宿主嵌入** | `.rx` 源 → `rurixc --emit=dll` → `rurix_rhi.dll` + import lib + 生成头 | C/C++ 引擎宿主，把 Rurix GPU 图当库调用 | **生产**（EI1 UC-05 面已验收：G-EI1-4 / G4.2 PR-D；本文 §4） |
| **B. 真窗口呈现 harness** | `target/release/g31_window_present.exe` | 评估渲染器真实窗口画质/帧率、做游戏画面 demo | **生产**（G31 波 A 验收 + G32 波 B 验收在案；本文 §5） |
| **C. 离屏 bench/契约车道** | `target/release/g14_3_pipeline_perf.exe --bench` | 性能对标、确定性 digest 回归、CI 集成 | **生产**（G14~G32 各期锚在案；本文 §6） |

形态 A 是本文主线（最小集成五步）。形态 B/C 是 harness（Rust 二进制，非库），用于验证与测量。

> **C1 在飞声明**：渲染器 SDK 稳定 API 面（G31_PLUS §5 #48，波 C Task C1）正在并行推进、**尚未定型**。本文最小集成以 **EI1 UC-05 既有 C ABI 面**（`#[export(c)]` codegen + `rurix_rhi.dll`，spec/export_c.md RXS-0250~0255 + spec/rhi.md RXS-0261/0277）为基线；C1 API 面冻结后本文按修订程序只追加更新。

---

## 2. 系统要求

| 项 | 要求 | 说明 |
|---|---|---|
| 操作系统 | Windows 11（x64） | 原生 COFF/PE/PDB 工具链；win32 swapchain 真窗口面 |
| GPU | NVIDIA（开发对照机 **RTX 4070 Ti**，Ada / sm_89） | 全部在案 measured 数字的单卡口径；其余厂商见 §9 兼容矩阵缺口 |
| 图形 API | **Vulkan**（渲染器生产车道后端） | `Rhi::create_vk`；DLSS 超分臂另需 NVIDIA NGX/Streamline 运行库（`vendor-upscale` feature） |
| 计算栈 | CUDA Toolkit + CUDA Driver API | 语言面 compute kernel（UC-05 compute 图）需要；纯图形嵌入面走 Vulkan |
| C++ 工具链 | **MSVC 2022**（链接期）+ Windows SDK | 编译宿主 `.cpp` 并链接 import lib；D3D12 互操作宿主另需 d3d12/dxgi 头与库 |
| device 编译 | **clang 22.1.x**（pin，`C:\Program Files\LLVM\bin\clang.exe` 或 `RURIXC_CLANG` 指定） | `rurixc` 把 `.rx` device kernel 编到 PTX/SPIR-V 的固定工具链（D-205） |
| 构建宿主 | Rust 工具链（`rust-toolchain.toml` 钉版） | 从源码构建 rurixc/harness 需要 |
| CI/门脚本 | Python 3 + `jsonschema` | 跑 `ci/*_smoke.py` 门需要 |

> GPU/MSVC/CUDA/clang 为文档化系统级前提（rustup 同类口径）；三态纪律见 §8——缺 GPU/缺工具链时门脚本如实 SKIP，不冒充 PASS。

---

## 3. 获取与构建

仓库根（`H:\rurix` 布局）从源码构建：

```powershell
# 全 workspace（含编译器 rurixc、CLI rx、渲染器 crate 与全部 harness）
cargo build --workspace
# 关键产物的 release 形态（测量/演示用 release；debug 仅开发）
cargo build --release -p rurix-render --bin g14_3_pipeline_perf --bin g31_window_present --bin g31_restir_wiring
# DLSS/FSR 超分臂（vendor 库本机齐备时）
cargo build --release -p rurix-render --features vendor-upscale --bin g14_3_pipeline_perf
```

关键产物：

| 产物 | 作用 |
|---|---|
| `target/debug/rurixc.exe` | `.rx` 编译器（`--emit=dll` 产 C ABI DLL 的唯一入口，CI 门同口径；`rx build --emit` 透传集当前不含 `dll`，用 rurixc 直调） |
| `target/release/g14_3_pipeline_perf.exe` | 离屏 bench/契约车道（形态 C） |
| `target/release/g31_window_present.exe` | 真窗口呈现车道（形态 B） |
| `target/release/g31_restir_wiring.exe` | ReSTIR 高档/低档双臂 harness |

---

## 4. 最小集成：C ABI 宿主五步

对齐 EI1 UC-05 既有面（spec/export_c.md RXS-0250~0255、spec/rhi.md RXS-0261/0277）。五步每一步对应 [examples/minimal_host/minimal_host.cpp](examples/minimal_host/minimal_host.cpp) 中同号注释段，可直接对照走通。

### 步骤 1 · 初始化（构建 DLL + 宿主装载自检）

把含 `#[export(c)]` host fn 的 `.rx` 源编成 cdylib——`apps/uc05-rhi/src/embed.rx` 是在案见证源（compute 图 + 图形帧两导出面）：

```powershell
# 仓库根；产物 = rurix_rhi.dll + rurix_rhi.lib（import lib）+ rurix_rhi.h（编译器生成头）
# （-o 按词干处理：传 <目录>\rurix_rhi 或 <目录>\rurix_rhi.dll 产物同名——实测两形一致）
target\debug\rurixc.exe apps\uc05-rhi\src\embed.rx --emit=dll -o <工作目录>\rurix_rhi.dll
```

- 生成头**自始生成、不手写**（RXS-0253：单一事实源 = typeck C 映射；LF 行尾/无时间戳/幂等，两次生成逐字节一致；篡改一字节 CI 再生成比对即红，RXS-0254）。
- **预期内 stderr note**：embed.rx 首次 emit 会打印两条 `SPIR-V lowering failed … (RXS-0291)` 信息性 note（gfx 演示着色器 position builtin 形态，entry 从 SPIR-V artifacts 表略去；本导出面走真跑不受影响，uc05 门同口径容忍）——以退出码为准，note ≠ 失败。
- 导出符号保名不 mangle（`/EXPORT:` 由 driver 从 typeck 导出集拼参，RXS-0252）；每个头声明 ↔ 恰一 DLL 导出符号。
- 宿主侧：`#include "rurix_rhi.h"` + 链 `rurix_rhi.lib`；运行期 DLL 与 exe 同目录（或 PATH）。先调纯常量自检导出（如 `uc05_gfx_pass_count()` 应返回 2）确认头↔DLL 调用面通达——**不触 GPU**。

### 步骤 2 · 场景/图（声明封闭在导出体内）

GPU 上下文、render graph、资源生命周期**全部封闭在导出函数体内**（EI1.4 同构）：导出体内部建 `Context::create` → `Rhi::create_vk` → 声明资源与 pass（raster/mesh/compute）→ `submit()` 装配期确定性核验（I3/I4/I5）+ hazard 推导 + 真派发 + 真 D2H 读回。**宿主只见 C ABI 标量与裸指针，不见任何 Rurix 类型**——场景以导出函数的标量入参（尺寸/规模）参数化，图形状由 `.rx` 源决定。

### 步骤 3 · 帧循环（每帧一次导出调用）

宿主帧循环中每帧调用一次导出函数；每次调用内 GPU 资源创建与销毁成对（无跨调用泄漏面，soak 级 leak 账本纪律同 G31 波 A）。示例以 64×64 图形帧循环 4 帧演示（`uc05_gfx_run_frame(&px, 64, 64)`，raster + mesh 两 pass，纯色 RGBA8 代表值经 `*mut u32` 出参回写）。

### 步骤 4 · 参数与错误面（标量入参 + i32 状态码闭集）

- **签名子集 v1**（RXS-0251）：定宽标量 + 裸指针（T∈标量）+ unit 返回；子集外类型（struct 按值/回调/数组/切片/句柄）编译期拒（RX6031）。
- **错误面 = i32 状态码闭集**（RD-026 无 Result 面纪律）：`0` 成功；`2` 入参越界（不进 GPU 路）；`3` device 数值与 host 闭式参考不等（数值红，不静默）。负例先行验证（如 `w=0` 必须返回 2）是推荐集成顺序——先证错误面可判定，再证正路径。
- **裸指针 = 调用方前置条件**（documented unsafe FFI 边界，RXS-0251/0255）：非空/对齐/可写由宿主保证；codegen 不引入隐式解引用，**跨 ABI 无 panic 面 by-construction**（导出体含可 panic 面编译期拒，RXS-0255）。
- **无跨 ABI 堆/资源所有权转移**（RXS-0252 CRT 红线）：DLL 静态 libcmt，不分配-并-返回；异 CRT 宿主（如 `/MD`）内存安全 by-construction。

### 步骤 5 · 关闭（资源随调用销毁 + 宿主卸载）

导出体内 GPU 上下文/图/资源在单次调用返回前逆序拆除；宿主侧无需调任何 "shutdown" 导出——关闭面 = 不再调用 + `FreeLibrary`。运行期/环境失败（无 Vulkan/无 GPU/DLL 装载失败）不占 RX 码、不跨 ABI 展开：确定性诊断 + 终止（RXS-0193 口径，spec/export_c.md §3）。

### 构建与运行（完整命令序列）

```powershell
# 仓库根；MSVC 环境（x64 Native Tools 或已导入 vcvars64）
cl /std:c++17 /EHsc /I <生成头目录> docs\renderer\examples\minimal_host\minimal_host.cpp ^
   /link /LIBPATH:<工作目录> rurix_rhi.lib
# 运行（rurix_rhi.dll 与 exe 同目录；需 Vulkan 可用 GPU）
minimal_host.exe
# 预期输出：RURIX_MINIMAL_HOST_OK passes=2 frames=4 pixel=0x00000000（纯色清色不变量，RXS-0277 Q-PixelCriterion）
```

一键脚本见 [examples/minimal_host/build.ps1](examples/minimal_host/build.ps1)；逐步说明与该脚本实际走通记录见 [examples/minimal_host/README.md](examples/minimal_host/README.md) 与 `milestones/g31/g31_renderer_docs_walkthrough.json`。

---

## 5. 形态 B：真窗口呈现

`g31_window_present` = 生产五 pass 车道（scene `g14_3_direct_gi` → mv `g14_mv` → TSR `g14_8_tsr_resample`/`g14_8_tsr_resolve` → device 显示编码 `g31_display_encode`）输出接 win32 真 swapchain 逐帧 present；bistro-interior 1080p 契约。

> **G37 W4 默认档翻转（2026-08-30 执行在案）**：`--quality` 缺省 = **`full`（十九臂画质终态 = 交付形态）**：smooth-normals / ggx / lamp-lights（gain 4）/ textures / bloom / dither / auto-exposure / tsr-quality / gi2（clamp 0.01）/ emissive-tex / metal-f0 / rt-ao / soft-shadows（1 样本）/ rt-reflect / gi2-tex / normal-maps / transparency / gi2-ris / gi2-nee。`--quality off` = 显式回退档（all-off 基线，中性字面零展开）。**单臂显式写法与诊断/互斥臂**（fg base 点 / hzb / slab / svt / storm / fault / cluster-lod / wp-hlod）**须显式给 `--quality off`**——展开面旗标与 full 同给 = fail-closed 拒跑（双重指定即语义歧义）。bench 车道（`g14_3_pipeline_perf`）`--quality` 默认维持 off，Stage A 锚面永不动。十九臂清单、新臂语义与登记面见 [feature_matrix.md](feature_matrix.md) §8；交付默认档的产品口径见 [PRODUCT_NOTES.md](PRODUCT_NOTES.md)。

```powershell
# 交互真窗口（默认 = --quality full 十九臂交付画质；WASD/QE 平移 + 鼠标视角 + -/= 曝光 ±0.25ev；ESC 退出）
target\release\g31_window_present.exe --frames 600 --warmup 10
# 非交互确定性轨迹（CI 口径；--hidden 不显示窗口仍真 swapchain present；G37 翻转后此形态 = full 交付默认）
target\release\g31_window_present.exe --frames 64 --warmup 10 --hidden --auto-move orbit
# 显式回退档（all-off 基线/诊断口径——单臂显式写法与诊断臂一律此形态）
target\release\g31_window_present.exe --frames 64 --warmup 10 --hidden --auto-move orbit --quality off
# FG x2 × all-off base（两点式闭集第一点；§8 在案双口径数字 85.24/145.30 fps 系此形态）
target\release\g31_window_present.exe --frames 100 --warmup 10 --hidden --auto-move orbit --quality off --fg x2
# FG x2 × full 交付默认（两点式闭集第二点，G37 W3 fg_combo 接线；fg 合法形态 = all-off base ∪ full 预设）
target\release\g31_window_present.exe --frames 100 --warmup 10 --hidden --auto-move orbit --fg x2
```

参数面（闭集，未知参数拒跑 exit=1；G37 终态）：

- **基础**：`--frames/--warmup/--tier <50|67|100>/--auto-move <orbit|dolly>/--ev100-ramp/--hidden/--headless-smoke/--evidence/--expect-digest/--contract/--gltf/--spv-*`。
- **画质预设**：`--quality <off|full>`（缺省 full；见上）。预设展开面之外的子参数可随 full 组合微调：`--lamp-k`（默认 12）/`--lamp-contrib`/`--autoexp-key|rate|min|max`/`--tsrq-min-alpha|clamp`/`--rt-ao-radius|strength|samples`/`--transp-alpha`（默认 0.85）/`--gi2-ris-m`（默认 6）。
- **G37 新臂**：`--transparency <off|on>`（玻璃 ray 穿透，入 full）/`--lut <off|neutral|warm|路径.cube>`（第 4 级色彩分级，可与默认 full 组合）/`--pso-report <json>`（PSO 变体账本 sidecar；账本本体默认开）/`--gi2-ris`+`--gi2-nee`（反弹 RIS 选灯 + 灯片 CDF 面光 NEE，入 full）/`--visbuffer`、`--cluster-per-frame-cut`（证据臂，须随 `--cluster-lod` ⇒ 须 `--quality off`）。
- **既有特性/诊断臂**：`--fg <off|x2|x3>/--textures/--slab-table/--slab-arm/--hzb/--svt/--cluster-lod/--wp-hlod/--fault-probe/--window-storm/--storm-soak` 及各单臂画质旗标——互斥闭集与两点式 FG 组合语义见 [feature_matrix.md](feature_matrix.md) §6（波 B 历史表）与 §8（G37 终态）；`--skin-demo/--dyn-demo` 在 bench 车道闭集、**不进**真窗口车道。

## 6. 形态 C：离屏 bench / 确定性 digest

```powershell
# canonical 口径：160 帧 warmup 10，bistro-interior t100 tsr_device
target\release\g14_3_pipeline_perf.exe --bench --scene bistro-interior --tier 100 --backend tsr_device --frames 160 --warmup 10 --out-root <out>
# receipt：<out>/bistro-interior/tier100/tsr_device/bench_receipt.json（含逐帧 frame_ms + last_frame_digest）
```

> 注：两形态 harness 的进度日志一律走 **stderr**（PowerShell 5.1 可能把首条 stderr 行显示为 `NativeCommandError` 样式）——以退出码与末行 `PASS`/`BENCH PASS` 为准。

digest 协议（Stage A 锚口径）：末帧 digest 与 `milestones/g14/g14_3_stage_a_digest_anchor.json` 在案锚逐格位级比对——跨日/跨机态零漂移是既有纪律（G31 波 A 锚检 18/18 ×多跑在案）。bench 车道参数闭集：`--scene <bistro-interior|cornell>/--tier <50|67|100>/--backend <tsr_device|dlss_sr|fsr_3_1_5>/--frames/--warmup/--gi <off|on>/--inflight <1|2|3>/--dyn-demo <refit|rebuild>/--skin-demo/--calibration-seed/--presentation-profile`（各自约束见 harness 拒跑文案与 feature_matrix §6）。

## 7. 确定性协议（固定 seed / digest 口径）

- **位级确定性**：同命令双跑 digest（及 digest_seq）位级一致是渲染器一等门（RXS-0357 L2 谱系；G31 波 A/波 B 各臂双跑在案）。相机/曝光逐帧 uniform 由帧号唯一事实源驱动（`--auto-move` f64 参数化）；`--calibration-seed` 固定 bench 标定 seed。
- **digest 口径不混**：`render_digest`（末帧 TSR 输出 f32 回读，G10EXRD-1）与 `digest`（末帧 device BGRA8 打包帧，G31BGRA-1）是两种口径，与 A1 历史 host f64 编码域**不冒充同值**；逐帧 `digest_seq`（auto-move 面）与末帧 digest 分列。
- **锚消费**：Stage A digest 锚 18 格只读消费、跨跑核验 MATCH；既有锚/注册表 0-byte 不回写（append-only 纪律）。
- **窗口 presented 锚谱系（G37 W4）**：presented 锚 = **二进制绑定锚**（重建后整批重收割，E1 治理律）；跨重建稳定面 = all-off `55e4a92d…` 与 bench `c1d28ad7…`。十六臂 full 锚 `5db2e7d7…`（day_0829）随 G37 十九臂并入作废；**十九臂默认 full 新锚 = W4 整批重收割，占位「见 W4_ANCHORS」**（`artifacts/day_0830_delivery/w4_flip/W4_ANCHORS.json`）。
- **已知观察面**：RD-045（间歇 digest 漂移 backfill 三件）maintain-open——观察窗累计零漂移不充确证；集成方若复现漂移请按 `registry/deferred.json` 字面回报。

## 8. 双帧率口径与三态纪律

**双口径分离（硬线）**：

| 口径 | 定义 | 禁则 |
|---|---|---|
| `real_render_fps` / `real_render_frame_ms` | 只由真渲帧构成（生产五 pass 墙钟；含 present 强制 BGRA8 回读段，`render_includes_forced_readback=true` 如实登记） | **生成帧禁入计数**；禁混 present 开销 |
| `presented_fps` | presented 帧 ÷（real_render_seconds + present_seconds）——含 FG 生成帧的呈现流畅度 | 独立登记，**禁冒充真实渲染帧率** |

FG 开启时 evidence 钉 `caliber_identities` 恒等式组（presented = real + generated、两 fps 各自重算、real fps 对 generated 扰动隔离）schema 层 const true。示例在案（G31 波 A A5 门）：`--fg x2` real 85.24 fps vs presented 145.30 fps（交付跑；复跑 65.83/116.91——两臂数字不冒充同值）。**G37 注**：该在案数字为 fg × all-off base 形态（当期默认 off）；W4 翻转后复现须显式 `--quality off --fg x2`，fg × full 组合点（两点式闭集第二点）数字待 W4 后在案登记。生成帧永不入真实渲染帧率口径的产品级承诺见 [PRODUCT_NOTES.md](PRODUCT_NOTES.md) §4。

**三态纪律**：无 Vulkan/无 GPU/资产缺失/工具链缺失 → harness/门脚本 `skipped_dev_env` **如实 SKIP 退 0**（非 PASS 非 FAIL）；`RURIX_REQUIRE_REAL=1` 下任何 dev-env 降级翻**硬 FAIL**（禁 mock 充真跑）。集成方 CI 接入时按此语义消费退出码。

## 9. 已知缺口（诚实登记）

- **设备兼容矩阵**：全部 measured 证据单卡（RTX 4070 Ti）；AMD/Intel 真卡面 open（G-MB1-6 尾门 + G31_PLUS §5 #50）；DLSS→FSR→TSR 降级链未系统化。
- **BistroExterior 场景**：维持 G10-N6 锚挂起（FBX2glTF 上游修复 + 源资产齐备）；场景闭集 = BistroInterior + CornellBox。
- **HDR 显示链**：maintain-SDR 字面维持（M118-hdr-cal；本机显示链 HDR token 全 absent）。
- **G17-MD-F1 性能焦点格**：bistro/t100/dlss_sr 17/18 诚实红在案（ratio 0.960479，轨迹见 performance_tuning.md §5）。

---

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-25 | 初版（G31+ 波 C Task C2）：形态总览/系统要求/构建/C ABI 五步/真窗口与离屏/确定性协议/双口径三态/已知缺口；C1 在飞声明以 EI1 UC-05 面为基线 |
| v1.1 | 2026-08-30 | G37 商业化收官同步（W5）：§5 默认档翻转（`--quality` 缺省 = full 十九臂、off 升显式回退档、单臂/诊断臂须显式 off）+ 参数面刷新（G37 新臂与可组合子参数）+ 命令示例按翻转后语义修订（CI 例增回退档形态、FG 例分列 base/full 两点）；§7 追加窗口 presented 锚谱系（二进制绑定锚 + W4_ANCHORS 占位）；§8 FG 在案数字标注 base 形态口径 + 指向 PRODUCT_NOTES |
