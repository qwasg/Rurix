# DLSS 5 NR 适配 — Phase 0 解包 + 静态探测 + 登记

> 开役 2026-08-30 23:1x。目标:解包 `D:\renodx-dlss5 v3.5.rar` 到 gitignored `external/`,对双 nvngx_dlssnr 变体 + addon64 做 sha256 / Authenticode / PE 导出表 / 目标字符串 / 等长字节 diff 静态取证,固化 vendor registry + license matrix 登记(泄露件 evaluation-only 纪律)。**本阶段不动 src/ 代码**。

## 1. 解包结果(external/dlss5-nr-v3.5,gitignored)

| 文件 | 大小 | 用途 |
|---|---|---|
| `renodx-dlss5 v3/nvngx_dlssnr.dll` | 165,840,496 B | root 变体(Blackwell/50系) |
| `renodx-dlss5 v3/nvngx_dlssnr 40系专用v1/nvngx_dlssnr.dll` | 165,840,496 B | **Ada/40系补丁变体(本机唯一可跑)** |
| `renodx-dlss5 v3/renodx-dlss5.addon64` | 518,656 B | ReShade 注入件(rurix 不消费,仅 ABI 参照) |
| `renodx-dlss5 v3/ReShade_Setup_6.8.0_Addon.exe` | 4,318,424 B | 第三方注入器(rurix 不用) |
| `renodx-dlss5 v3/bb8d51c...png` | 46,779 B | Lecram 发布贴说明图 |

`tar -xf`(系统 libarchive)解包成功;`external/` 已在 `.gitignore` 闭集内,二进制零入 git。

## 2. sha256 / Authenticode / 版本资源

| 件 | sha256 | 签名 | FileVersion | Desc |
|---|---|---|---|---|
| root | `e16bcf15…e1fc8e` | **Valid**(CN=NVIDIA Corporation) | 310.8.0.0 | NVIDIA DLSSNR - DVS PRODUCTION |
| 40系 | `28bdc080…2b9265` | **HashMismatch**(破签) | 310.8.0.0 | NVIDIA DLSSNR - DVS PRODUCTION |
| addon64 | `fa6b2f6d…22f5f` | NotSigned | 0.2026.0828.2110 | (RenoDX addon) |

root 签名 Valid 但属 **NBA 2K27 EA 泄露的预发布件**(签名有效 ≠ 授权分发);40系为社区就地补丁,破坏原签名 = HashMismatch(补丁件确证)。

## 3. PE 导出表(手写 PE32+ 解析,55 导出)

snippet 导出 **D3D11 / D3D12 / VULKAN / CUDA 四 API 全套** + meta 面。关键:

- **D3D12 面**(Phase 1 探针主臂):`Init_Ext / GetFeatureRequirements / GetScratchBufferSize / CreateFeature / EvaluateFeature / ReleaseFeature / Shutdown1 / PopulateParameters_Impl`
- **VULKAN 面**(重大发现,见 §6):`Init_Ext2 / CreateFeature1 / EvaluateFeature / GetFeatureInstanceExtensionRequirements / GetFeatureDeviceExtensionRequirements`
- **meta**:`GetAPIVersion / GetSnippetVersion / GetGPUArchitecture / GetDriverVersionEx`

## 4. 目标字符串证据

- **NR 参数键**(`DLSSNR.*` 40 组):`ScalingRatio / Style / Color / MVec / Depth / Output / ControlMask / UI / UIAlpha / Backbuffer`,每资源槽带 `Subrect{BaseX,BaseY,Width,Height}` 族——NR 契约面确证(Color/Depth/MVec 输入 + Output,可选 ControlMask/UI)。
- **NGX core 定位**:注册表 `SOFTWARE\NVIDIA Corporation\Global\NGXCore` FullPath(串:`Loaded NGXCore from path (%ls)` / `failed to open registry key ...NGXCore`)。
- **addon64 加载模型**(D3D12 路参照):`signed NR runtime (nvngx_dlssnr.dll) pre-loaded at device init` + `nvngx_dlssnr.dll was not found beside the addon`——snippet 于 device init 前预载,addon 走 `NVSDK_NGX_D3D12_{Init_Ext,AllocateParameters,CreateFeature,EvaluateFeature,EvaluateFeature_C,ReleaseFeature,Shutdown1}`。

## 5. 双变体等长字节 diff(补丁范围量化)

等长 165,840,496 B;差异 **12,364,628 B(7.46%)**,集中 **4 个区段**,首偏移 94,966 / 末偏移 18,083,536——**补丁全部落于文件前 ~18MB 的 cubin fatbin 区**,host 代码面(PE 头/导出表/后段)零差异。

全架构扫描:40系变体只命中 `sm_89`(Ada),**无 `sm_120`/`sm_90` 残留**;root 变体 `sm_120`(Blackwell)cubin kernel 9 组(`cuda_capture_kernel` 等)。⇒ **就地 cubin 架构替换补丁确证(sm_120 → sm_89),纯净**。本机 RTX 4070 Ti(Ada/sm_89)必须用 40系变体。

## 6. 重大发现:snippet 导出完整 Vulkan 面 → 计划落点优化

原计划假设「NR 只有 DX12 面」,故设计 DX12 直驱 + Phase 4 VK↔DX12 external memory interop。**静态证据推翻此假设**:`nvngx_dlssnr.dll` 导出完整裸 NGX Vulkan 面(`NVSDK_NGX_VULKAN_Init_Ext2` + `CreateFeature1` + `EvaluateFeature` + device/instance extension 协商)。

含义:
- **Phase 1 探针**仍先走 D3D12(复用 `FsrDx12Session` 现成 d3d12/dxgi/queue/fence/committed-resource/upload/readback 基建,最快拿 ground truth,与社区/addon64 验证路一致)。
- **Phase 2 评估臂 / Phase 4 窗口面**:若 D3D12 探针 GO,优先评估 **Vulkan 裸 NGX 臂**——项目 renderer 原生 Vulkan,可直接把自有 `VkInstance/VkPhysicalDevice/VkDevice` 交给 NGX(`Init_Ext2`),NR 直写自有 VK image,**免掉 VK↔DX12 interop**。这比原计划 Phase 4 大幅简化。裸 NGX VK 与项目现有 Streamline interposer 臂(`DlssVkSession`)是两条独立路(前者 app 自建 device + NGX 协商扩展,后者 SL 代理 device),不冲突。

## 7. 许可处置(evaluation-only,登记面)

源件 = 泄露 NVIDIA 预发布件(root)+ 社区破签补丁件(40系),**无 owner 可接受的再分发许可路径**。登记:

- `redistribution_status = blocked`,`commercial_redistribution = prohibited`
- evaluation-only 本机 `external/` 缓存,永不 vendoring / 再分发 / 进生产默认 / 入 bundle
- 升格生产须 owner 法律面动作(等 NVIDIA 官方 DLSS 5 SDK 发布 + 官方通道清结);agent 不冒充 owner 接受泄露件(G13 范式)

登记文件:[dlss5nr_vendor_sdk_registry.json](dlss5nr_vendor_sdk_registry.json) / [dlss5nr_license_matrix.json](dlss5nr_license_matrix.json)。

## 8. Phase 0 结论

解包 + 静态取证全绿,Phase 1 前置事实链齐备:
- 本机可跑变体锁定 = 40系 Ada(sm_89 纯净补丁)
- NGX ABI 面确证 = feature id 18 + D3D12/VULKAN 双 API + DLSSNR.* 参数键 + NGX core 注册表定位
- 探针主臂 = D3D12(复用 FSR 基建),备选 = 直驱 snippet;VK 裸 NGX 记为 Phase 2/4 优化候选
- 许可 = evaluation-only blocked,登记闭合

## 工具(artifacts/day_0830_dlss5nr/tools)

- `pe_probe.py` — 纯标准库 PE32+ 导出表解析 + 目标 token ASCII 上下文扫描
- `bin_diff.py` — 等长二进制分块字节 diff 量化(补丁范围取证)

## 证据(artifacts/day_0830_dlss5nr/probe_static)

`nr_root.json` / `nr_40series.json` / `addon64.json` / `variant_diff.json` / `arch_40.json`
