# DLSS 5 NR 适配 — Phase 4 窗口面散臂(GO 前提)

> 承 Phase 3 裁决 = NO-GO(NR 硬件限定 Blackwell,本机 Ada 不可用)。Phase 4 为 **GO 前提**
> 阶段——GO 前提(NR 在本机可实例化)不满足,本阶段**不在本机执行**,登记 blocked + Blackwell
> 落地设计。

## 1. 状态:blocked(GO 前提不满足)

计划 Phase 4 = g31 窗口(Vulkan present)接 NR 散臂。前提 = NR 特性可用(Phase 3 GO)。本机
RTX 4070 Ti(Ada)Phase 3 = NO-GO(CreateFeature(18) 硬件不支持),故窗口散臂**无对象可接**,
本机不执行,登记 blocked(非跳过——GO 门未过)。full19 / RD-045 orbit 等窗口锚**不动**
(NR 未接入 g31_window_present,冻结面 0-byte)。

## 2. Blackwell 落地设计(GO 后执行面;Phase 0 重大发现驱动的计划优化)

原计划 Phase 4 设 **VK↔DX12 external memory interop**(因假设「NR 仅 DX12 面」)。**Phase 0
静态探测推翻此假设**:`nvngx_dlssnr.dll` 导出**完整裸 NGX Vulkan 面**(`NVSDK_NGX_VULKAN_Init_Ext2`
+ `CreateFeature1` + `EvaluateFeature` + `GetFeatureDeviceExtensionRequirements`),且驱动
`_nvngx.dll` 亦导出全套 VULKAN app-facing 面(Phase 1 dump 实证)。

⇒ **Blackwell 上窗口面最优路 = VK 裸 NGX 直连**(非 DX12 interop):
- g31 窗口 renderer 原生 Vulkan,可直接把自有 `VkInstance/VkPhysicalDevice/VkDevice` 交给
  NGX(`NVSDK_NGX_VULKAN_Init_Ext2`),NR 直写自有 VK image,**免掉 VK↔DX12 external memory
  interop 全部复杂度**(原计划 Phase 4 主要工作量)。
- 与项目现有 DLSS SR(Streamline interposer Vulkan 臂,`DlssVkSession`)是两条独立路:前者
  app 自建 device + NGX 协商 device/instance 扩展(`GetFeatureInstanceExtensionRequirements` /
  `GetFeatureDeviceExtensionRequirements`),后者 SL 代理 device——不冲突。
- 落地形态:仿 Phase 2 `NrDx12Session` 建 `NrVkSession`(VK 臂),散臂 flag 默认 off,
  full19 锚不动;窗口 present 后置一 pass NR 增强(输入=呈现前 color/depth/mv 的 VK image)。

## 3. 复活/执行条件

1. **换 Blackwell 硬件**(RTX 50 系):本机复跑 [g13_dlss5_nr_probe](../../src/rurix-rt/src/bin/g13_dlss5_nr_probe.rs)
   → verdict available ⇒ Phase 3 GO ⇒ 本 Phase 4 执行(优先 VK 裸 NGX 路)。
2. NVIDIA 正式 DLSS 5 SDK 发布 → 官方通道 SDK 替换泄露件 + G13 owner 许可清结(评估件即刻退役)。

## 4. 结论

Phase 4 = blocked(GO 前提硬件不满足);Blackwell 落地路径已凭 Phase 0/1 实证优化设计
(VK 裸 NGX 直连 > DX12 interop)。本机零执行、冻结窗口面零改动。
