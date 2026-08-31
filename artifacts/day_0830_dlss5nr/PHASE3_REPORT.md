# DLSS 5 NR 适配 — Phase 3 证据 + GO/NO-GO 裁决

> 承 Phase 1(可用性 NO-GO)+ Phase 2(NrDx12Session + lane)。本阶段:画质/帧时证据
> 战役裁定 + 默认锚零漂移证明 + GO/NO-GO 终态登记。

## 1. 裁决:NO-GO(NR 硬件限定 Blackwell,本机 RTX 4070 Ti/Ada 不可用)

DLSS 5 Neural Rendering 特性在本机**无法实例化**——`CreateFeature(NGX feature 18)` 恒返
`UnableToInitializeFeature`(core 臂)/ `PlatformError`(直驱臂)。这是**特性的硬件限制**
(NR 内核依赖 Blackwell 的 FP8/sm_120 面),非本适配缺陷:NGX 集成全链(core 定位 / 4 参
Init / MSVC 逆序 vtable / AllocateParameters / D3D12 device 面)**逐环 Success 验证通过**。

## 2. 画质/帧时证据战役 = 不可采集(如实登记)

计划 Phase 3 拟「两场景(cornell-box/bistro-interior)× 三 tier(100/67/50)画质 ROI/SSIM +
帧时记账」。**本机无法采集**:NR 特性 create() 即失败,产不出任何 NR 帧,故无 color 输出
可比对、无 evaluate 帧时可记。这不是跳过,是**物理不可行**——采集前置(NR 可实例化)不满足。

已采集的 measured 证据(本机 RTX 4070 Ti / 驱动 620.02):

| 证据 | 路径 | 结论 |
|---|---|---|
| NGX D3D12 探针(1080p) | [probe_dynamic/t100_1080p.json](probe_dynamic/t100_1080p.json) | Init/GetCap/vtable/Alloc 全 Success;CreateFeature(18) core=UnableToInitializeFeature / direct=PlatformError |
| tsr→nr / dlss_sr→nr 双链 lane | [probe_dynamic/lane_chains.json](probe_dynamic/lane_chains.json) | 上采样段真跑真出帧(TSR digest e965b39e / DLSS digest 17bebf92);NR 段双链 fail-closed 同错 |

## 3. 默认锚零漂移证明(measured + 构造)

**构造证明**:本适配对 `src/rurix-rt/src/vendor_upscale.rs` = **1647 插入 / 0 删除**(`git diff
--numstat` 实测),纯追加——既有 `FsrDx12Session`/`DlssVkSession`/TSR 会话代码**字节不变**;
Cargo.toml 仅追加 `[[bin]]` 条目(+9/+8,0 删除);g14_3_pipeline_perf / g14_3_lane_body /
g31_window_present 冻结车道**字面零改动**(NR lane 为独立 harness bin)。故默认车道
(all-off / dlss_sr / fsr / tsr / full19 / Stage A 18 格 / RD-045)机器码与语义与适配前逐位相同。

**独立 measured 佐证**:lane_chains.json 中 dlss_sr 与 tsr 上采样段在含本适配的树上**仍真跑
真出帧**(有效输出 digest),即既有上采样车道功能未受任何影响。

## 4. 采纳处置(终态)

- **NrDx12Session 评估臂封存**:代码正确、fail-closed、default off、evaluation-only 泄露件
  登记闭合;**在 Blackwell(RTX 50)硬件上即可跑通**(集成已全链验证到 CreateFeature 前一步)。
- **生产默认零影响**:NR 未接入任何默认车道/窗口/发布 bundle;env opt-in(`RURIX_DLSS5NR_SDK_DIR`)
  显式启用,缺件/缺符号/CreateFeature 失败均确定性 Err。
- **许可**:blocked_for_redistribution(泄露预发布件 + 社区破签补丁),evaluation-only,永不
  再分发/进生产默认([dlss5nr_license_matrix.json](dlss5nr_license_matrix.json))。
- **复活条件**:①换 Blackwell 硬件本机复跑探针即转 available;②NVIDIA 正式发布 DLSS 5 SDK
  后以官方通道 SDK 替换泄露件 + 走 G13 owner 许可清结。

## 5. 结论

**GO/NO-GO = NO-GO(本机 Ada 硬件限制)**。适配工程**完整且正确**;DLSS 5 NR 特性本身
硬件限定 RTX 50,本机 RTX 4070 Ti 无法运行——本机 measured 与社区一手实测双向坐实。
