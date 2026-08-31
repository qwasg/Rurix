# DLSS 5 NR 适配 — HANDOVER

**一句话(经 v4.55 复核修正)**:适配工程**完整、正确、fail-closed、加性零漂移**。
**40 系可以开 DLSS 5 NR ——但要经 ReShade + renodx-dlss5 v4.55 addon 在 DX12 游戏里**
(社区实证)。rurix 裸引擎直集成本机 Ada 被 **snippet host 侧架构门**挡下
(snippet.Init=`FeatureNotSupported`);干净集成面已就绪,待官方 Ada 支持 / Blackwell /
或引擎内复现 addon 机制(泄露件逆向,超本次范围)。详见 [PHASE5_REPORT.md](PHASE5_REPORT.md)。

## 结论(修正:软件门,非硬件限制)

先前「硬件限制」框定**不准**。NR 神经内核**能在 Ada 执行**(Uncle Burrito Ada cubin 补丁在
4090/4080 实证);挡路的是 **snippet host 侧架构门**:探针第三臂实测两 cubin 变体的
`snippet.Init` 均返 **`FeatureNotSupported`(0xBAD00001)**(Init 阶段查架构即拒,早于 cubin
执行)——与 NIGos/dlss5-d3d12-fix 一手记录逐字一致。

- **裸进程直路(rurix)**:core 臂 `UnableToInitializeFeature` / 直驱臂 `PlatformError` /
  snippet.Init `FeatureNotSupported`——三臂皆被拒(NGX 集成前链 Init/GetCap/vtable/Alloc
  全 Success,证明集成正确;被拒是 snippet/core 的 feature-18 门,非集成缺陷)。
- **addon 路(可跑 40 系)**:pre-load 签名 snippet(**不调其 Init**)+ IAT patch
  (GetModuleFileNameW)+ hook **游戏活跃 DLSS** 的 NGX EvaluateFeature,piggyback 已初始化
  的 DLSS 上下文 —— 绕过架构门。本质是**游戏注入 hook**,裸引擎无法干净复现。

## 交付物

| 类 | 路径 |
|---|---|
| 手写 NGX D3D12 FFI + `NrDx12Probe` + `NrDx12Session`(create/report/evaluate/Drop) | [src/rurix-rt/src/vendor_upscale.rs](../../src/rurix-rt/src/vendor_upscale.rs)(尾部加性 1647 行 / 0 删除) |
| NGX 可用性探针 bin | [src/rurix-rt/src/bin/g13_dlss5_nr_probe.rs](../../src/rurix-rt/src/bin/g13_dlss5_nr_probe.rs) |
| tsr→nr / dlss_sr→nr lane harness bin | [src/rurix-render/src/bin/g13_dlss5_nr_lane.rs](../../src/rurix-render/src/bin/g13_dlss5_nr_lane.rs) |
| 静态探测工具(PE 导出/字节 diff) | tools/pe_probe.py, tools/bin_diff.py |
| vendor 登记 + 许可矩阵 | dlss5nr_vendor_sdk_registry.json, dlss5nr_license_matrix.json |
| 分阶段报告 | PHASE0/1/3/4_REPORT.md, CAMPAIGN_LOG.md |
| measured 证据 | probe_dynamic/t100_1080p.json, probe_dynamic/lane_chains.json, probe_static/*.json |

## 运行(本机复现 NO-GO)

```
$env:CARGO_TARGET_DIR="H:\rurix\target-night"
cargo build -p rurix-rt --features vendor-upscale --bin g13_dlss5_nr_probe
./target-night/debug/g13_dlss5_nr_probe.exe --size 1920x1080   # verdict=not_available
cargo build -p rurix-render --features vendor-upscale --bin g13_dlss5_nr_lane
./target-night/debug/g13_dlss5_nr_lane.exe --chain both        # 上采样真跑,NR 段 fail-closed
```

## 纪律遵守

- **加性零漂移**:vendor_upscale.rs 1647 插入 / 0 删除;既有 FSR/DLSS/TSR 会话字节不变;
  g14_3/g31 冻结车道字面零改动(NR lane 为独立 harness)。默认车道语义逐位同适配前。
- **fail-closed / default off**:NR 未接任何默认车道/窗口/发布 bundle;env opt-in
  (`RURIX_DLSS5NR_SDK_DIR`)显式启用,缺件/缺符号/CreateFeature 失败均确定性 Err。
- **许可**:泄露预发布件 + 社区破签补丁 = blocked_for_redistribution,evaluation-only,
  永不入 git / 再分发 / 进生产默认。

## 可用路径

**A. 40 系现在就想用(游戏,社区实证路)**:ReShade 6.8 Addon 版 + renodx-dlss5 v4.55 addon +
**NVIDIA 签名** nvngx_dlssnr.dll → 放进 DX12 游戏 exe 目录,`[ADDON] LoadFromDllMain` +
ReShade overlay 勾 "Enable DLSS Neural Rendering"(视驱动/游戏)。addon 靠 pre-load 签名 snippet
+ IAT patch + hook 游戏活跃 DLSS 上下文绕过架构门。**这是 addon 的设计用途,与 rurix 引擎解耦。**

**B. rurix 引擎面 GO 路径**:
1. **NVIDIA 正式 DLSS 5 SDK 发布**(Ada 官方档)或 **换 Blackwell(RTX 50)**:本机复跑探针 →
   snippet.Init 不再 FeatureNotSupported ⇒ NrDx12Session 直接可用;窗口面走 **VK 裸 NGX 直连**
   (`NVSDK_NGX_VULKAN_Init_Ext2`,Phase 0 实证 snippet+core 均导出 Vulkan 面 → 免 VK↔DX12
   interop,见 PHASE4_REPORT)+ 官方件替换泄露件走 G13 owner 许可清结。
2. **本机 Ada 引擎内强启用**(唯一途径):在 rurix 进程内复现 addon 的 hook + 运行时 patch
   机制(IAT/GetModuleFileNameW + host 架构门 patch + 依附活跃 DLSS 上下文)——**未纳入本次
   交付**:对 165MB 泄露专有 DLL 深度逆向内存手术,脆弱且许可越界(评估件)。
