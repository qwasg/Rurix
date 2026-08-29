# 15 · 外部采纳登记

> 滚动只追加。登记外部项目对本仓（语言 + 渲染器）的真实采纳事实与上游补面回流 provenance。
> 性质：**事实登记件**，非验收门、非营销宣称；同作者跨仓采纳不冒充「外部采纳判据」（#56）。
> 本文件由 2026-08-28 双仓对账波建立（对账动机：引擎仓治理面此前单侧登记，本仓零记录）。

## A1 · RurixForge（D:\游戏引擎）——首个跨仓引擎宿主集成（2026-08-16 起，持续在用）

**项目**：RurixForge，「AI 主导的游戏制作引擎」（Electron + React + Rust axum workspace，crates 前缀 `forge-*`）。
其治理不变量 I-1（00_MASTER_INDEX §5）：运行时内核只用本仓已有实现，不自研第二渲染器/物理后端。

**消费面**（全部 Cargo path 依赖，指向 `H:/rurix/src/*`；引擎仓 master 6c8401e 时点）：

| 引擎 crate | 本仓面 | 用途 |
|---|---|---|
| engine-host | `rurix-rt`(features=["vulkan"]) `render_exec` | 视口实渲染：`DeviceFrameSession` 固定 pass 图 + 相机 UBO + 逐实体 push constants + `Readback::Texture` |
| engine-host | `new_with_imported_d3d12_textures`（`VK_KHR_external_memory_win32` import 档） | VK→D3D12 零拷贝帧通道（共享线性 buffer + fence，presenter 进程消费嵌入 Electron） |
| engine-host | `rurix-physics` | 固定步物理 + 接触事件 |
| engine-host | `soft-raster` | F0 期 CPU 光栅腿（保留） |
| assetd | `rurix-asset`(gltf 严格导入) + `rurix-geom-build`(build_dag/write_dag/read_dag) + `rurix-pkg`(SHA-256) | gltf → RXGB `.rxmesh` 内容寻址缓存构建链 |
| forge-logic / forge-index / forge-util | `rurix-pkg` | 手写 SHA-256 缓存键 |
| 工具链 | `rx.exe` / `rurixc.exe` CLI 子进程 | rx check/test/fmt（MCP 面）、`--emit=dll` 图解释执行运行时 |

**验收证据**（引擎仓 evidence/，与本仓门形态同构的诚实三态）：
- `f1-w2-viewport-smoke-20260819-164157.log`（2026-08-19）：**PASS**——RTX 4070 Ti，draws=3 nonzero=1524、两帧逐字节一致、移动帧变、点选命中/未命中、共享纹理 presenter presented=3/3 + share_close 幂等、OS 级截屏锚点与引擎帧逐字节一致。
- `f8-w4-matrix-2026-08-19T08-30-14Z.json`：gateGreen=true，含纯浏览器编辑器出帧 6/6、chat 建实体真 engine 链 11/11。
- 里程碑 F0–F11 全收口（F1 四波：场景编辑闭环 → rurix-rt 实渲染 → VK→D3D12 零拷贝 → H.264 流腿，各波验收记录见引擎仓 milestones/f1/F1_CONTRACT.md §6）。

**与 G31+ TODO 的关系（字面对账，不回写既有行）**：
- #48（渲染器 SDK 稳定 API 面）DoD「≥1 个外部 C++/D3D12 或 Vulkan 引擎宿主集成 demo 真跑」的**工程事实面**首次获得真实承载件（上表消费面 + PASS 证据）。如实登记：宿主为 Rust 非 C++、同作者跨仓非第三方；#48 行字面是否翻转留 owner 治理程序，本登记不预支。
- #56（外部采纳判据，使命级）要求「≥1 个**非作者维护**的真实项目采用渲染器」——RurixForge 为同作者维护，**不满足字面，#56 维持未宣称（0-byte）**。本文件不改变其状态。

## A2 · 上游补面回流 provenance（回溯登记；此前双仓均无来源记录）

**事实**：本仓 `src/rurix-rt/src/render_exec.rs` 的 VK external memory 档部分源于引擎仓 F1 wave.3（2026-08-17）的当日内上游补面，经 PowerShell 补丁脚本（引擎仓 `scripts/_f1w3_upstream_patch4~7.ps1`，现随引擎仓落账清理删除，repro 留档 `_f1w3_repro.ps1`）直接改写本仓文件：

- **R4**：`vkBindImageMemory` 返回值校验 + import 失败诚实 `Err`（现存于 render_exec.rs「vkBindImageMemory(import) 失败」分支，~L12401）——修复 F1 wave.3 实测的 960×540 首帧 `VK_ERROR_DEVICE_LOST`（bind 静默失败 → 未绑定图像参与渲染）。
- **R6/R7**：设备扩展 `VK_KHR_external_memory`(+`_win32`) 探测启用（缺扩展 fail-closed 不降级）+ 外部内存导入会话构造（`new_with_imported_d3d12_textures`，~L1177）+ pNext 结构体注入（VkExternalMemoryImageCreateInfo / VkImportMemoryWin32HandleInfoKHR 等，sType 经 SDK 1.3.296 头核对）。
- **后续演进**：F1 wave.3 时的图像探针（`probe_image_mem_req`）与 D3D12 共享堆腿随「共享体由纹理改为线性 buffer」重构在**引擎侧**退役（引擎仓 f1_zerocopy.rs:264 注释留痕）；本仓现存 import 面为 buffer/texture 两形态的 `new_with_imported_d3d12_textures`。
- **吸收路径**：上述面随后随本仓 HEAD 演进（含 2026-08-27 G34 合流批 058f8e68 等）按文件吸收提交；此前本仓治理面（决策日志/里程碑登记）无来源标注——**本节即为回溯来源登记**（只追加，不改写既有 commit 叙述）。

## A3 · 版本锚定事实

- 引擎侧依赖形态 = path 依赖随本仓 HEAD 漂移（引擎仓决策 D-016：开发期常态，tag 锚定推迟至发行形态裁决）。对账时点锚：**本仓 HEAD bece24e7（2026-08-28）**；引擎仓落账基线 master 6c8401e（F8，2026-08-19）。
- 本仓 tag 面：`v1.0.0` / `v1.0.1-dist.1` / `v1.0.1-dist.2` 为既有版本 tag；尚未产生面向宿主集成的渲染器稳定快照 tag（rurix-renderer-sdk v1 为 G33 Wave C 交付，feature-gated，RurixForge 未消费——其绑定的是更底层的 rurix-rt render_exec 库面，见 A1 表）。

## 修订

- 2026-08-28 建立本文件：A1/A2/A3 初版（双仓对账波；对账另一侧 = 引擎仓 14_DECISION_LOG D-027+ 与 F1 close-out 终审）。
