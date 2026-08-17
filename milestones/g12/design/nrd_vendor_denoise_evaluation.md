<!-- Assisted-by: Kimi-K3（G12.3 降噪波） -->
# G12.3 M162 — NRD 类 vendor 降噪评估报告（评估不接线）

> **性质**：RD040-nrd 承接锚口径兑现面（G12_CONTRACT §6 RD-040 行 / §7 裁决 9 / G12_CANDIDATE_DECISIONS §2；RFC-0029 §4.5 L5；spec/global_illumination.md RXS-0402 L5）。**评估不接线**——本报告零 src/ 接线、零依赖新增、零 vendor 代码 vendoring；接入另判 G13+ 窗（RD-040 backfill_condition 字面 0-byte 维持：「NRD/vendor 降噪经 UpscaleBackend 同构输入契约接入（MV/深度/法线），接入时不改 temporal 底座」）。
> 取证时间：2026-08-17（联网实测 GitHub 仓与 GitHub API）；自研 measured 数字转引自 M162 门 evidence（本波真跑面，见 §5 溯源）。

---

## §1 承接锚（RD040-nrd 口径逐字）

- RD-040 backfill_condition nrd 分项（`registry/deferred.json`，0-byte 只引）：「逐项独立判档（10 §3）：……NRD/vendor 降噪经 UpscaleBackend 同构输入契约接入（MV/深度/法线），接入时不改 temporal 底座」。
- 本评估 = G12.3 窗的**评估面**兑现：接入面评估 + 许可/ABI 取证 + 与自研降噪面 measured 对照。**不判 go/no-go 接入**——接入裁决归 G13+ 窗（RD-040 总体 status open 维持，history 只追加登记本评估落盘）。

## §2 UpscaleBackend 同构输入契约接入面评估（MV/深度/法线）

冻结接口面（`src/rurix-render/src/temporal/upscale.rs`，RFC-0016 §4.0-3 冻结，0-byte 只读）：`UpscaleInputs` = color（3ch 预曝光）/ depth（1ch）/ mv（2ch，uv 位移，mv = prev_uv − cur_uv）/ reactive / exposure / jitter / output_size。M162 降噪管线的输入面（RXS-0402 L1）= PT 原生帧 + G-buffer（NDC 深度 + 世界几何法线）+ 相机 MV（`temporal::common::compute_camera_mv` 派生）。

NRD 输入契约（v4.17.5 master，2026-08-17 实测仓内 README/Integration 头文件）：REBLUR/RELAX 族消费 `IN_MV`（motion vectors——**物体运动 3D 世界空间或屏幕空间可选，相机运动不得含入**〔相机运动由矩阵携带〕，`CommonSettings::motionVectorScale` 缩放）/ `IN_VIEWZ`（**视空间线性 Z**，非 NDC）/ `IN_NORMAL_ROUGHNESS`（打包法线 + 粗糙度）/ 辐射度输入（demodulate albedo 口径族内变体）/ 可选 `IN_BASECOLOR_METALNESS` 等。

同构接入面差距逐项登记（评估面，不接线）：

| # | 契约轴 | Rurix 现有面 | NRD 需求面 | 差距评估 |
|---|---|---|---|---|
| 1 | MV | 相机 MV（uv 位移，相机运动含入）经 `compute_camera_mv`（底座 0-byte 消费面） | 物体 MV（相机运动除外）或屏幕空间 MV + `motionVectorScale`；首 pass 可全 0（`isMotionVectorInWorldSpace` 口径） | **可适配**——静态场景物体 MV=0 与相机 MV 互补；需 MV 语义转换层（相机 MV → NRD 口径），工程小但非零；几何/蒙皮/WPO 三类物体速度面归 G5.3 冻结面存续（本评估不展开） |
| 2 | 深度 | NDC [0,1] ZO（G-buffer 派生面，M162 门内在树） | 视空间线性 Z（`IN_VIEWZ`） | **可适配**——NDC→viewZ 为闭式换算（M162 派生面已有正变换，逆变同一公式）；零新底座面 |
| 3 | 法线 | 世界几何法线 3ch（绕向） | 打包法线 + 粗糙度（`IN_NORMAL_ROUGHNESS`） | **可适配但有范围边界**——M96 起步范围冻结 Lambert-only（RXS-0357 0-byte），粗糙度恒 1.0 常量槽即可；specular 链降噪不在 G12 面（锚定 G15 画质收口） |
| 4 | 辐射度口径 | 终色 HDR 直接降噪（M162 管线） | REBLUR_DIFFUSE 族 = albedo demodulate 口径（辐射度 ÷ albedo 降噪后再调制） | **结构性差距**——REBLUR 族效能依赖 demodulation；Rurix PT 面需新增 albedo 分离输出（G-buffer 扩面，非底座改动）；不扩则只能消费 REFERENCE/间接族，效能面打折 |
| 5 | 后端形态 | rurix-rt 手写 Vulkan（U30 审计面）+ .rx compute kernel | NRI 抽象层（D3D11/D3D12/VK wrapper）+ HLSL shader pack | **重**——NRD 经 NRI Vulkan 接入 = vendor 库 + NRI 包装层双依赖；与 rurix-rt 手写 vk 车道并存需装配层适配（render graph/资源状态面 G3.5 冻结面衔接评估归接入窗） |

## §3 许可取证（2026-08-17 联网实测）

- 仓库：`github.com/NVIDIA-RTX/NRD`（v4.17.5，master 活跃，2026-08-17 有提交；HLSL/C++；825 stars）。
- **许可 = 自定义「NVIDIA RTX SDKs LICENSE」**（仓内 `LICENSE.txt`；GitHub API `license.key = "other"` / `spdx_id = NOASSERTION`——**非 OSI 批准许可**，非 MIT/BSD）：协议许可文体（"legal agreement … governs the use of the NVIDIA RTX software development kits, including the DLSS SDK, NGX SDK, RTXGI SDK, RTXDI SDK and/or NRD SDK"）；`Integration/NRDIntegration.h` 头文件带专有 banner（Copyright (c) 2022 NVIDIA CORPORATION, All rights reserved + "without an express license agreement … strictly prohibited"）。
- 含义（诚实登记，不作法律结论）：接入 = 接受自定义 SDK 协议条款（使用/再分发/归属条款逐条须 owner 法律面审）；**与 Rurix 仓库许可同列评估是 G13+ 接入窗的硬前置**；本评估期零 vendoring、零代码片段复制（G12 立项裁决 10 / RFC-0027 字面同律）。
- 对照面：自研降噪管线（M162）零第三方依赖；FSR 类（RD-041 方向）为 MIT——vendor 选型时许可轴须并列（本评估不展开，RD-041 字面 0-byte）。

## §4 ABI / 集成形态取证（2026-08-17 实测）

- ABI 形态：C++ 库（NRD.dll/静态库）+ HLSL shader pack + `NRDIntegration`（NRI 基集成层，`NRD_INTEGRATION_VERSION 22`，2026-04-06）；纹理经 NRI wrapper 三后端（`TextureVK`/`TextureD3D12`/`TextureD3D11` union 面）。
- 集成形态（若 G13+ 接入）：vendor 库链入（C ABI 边界须 unsafe-audit 登记，沿 U 段审计模式）+ NRI Vulkan 后端 + 资源状态/管线装配与 rurix-rt 车道衔接 + shader pack 的 SPIR-V 路径（NRI 内置或 DXC 链——工具链 pinning 进接入窗评估）。
- **本评估零接线**：树内 grep 实测 `NrdIntegration` / `nrd::` / `IN_VIEWZ` / `NRDIntegration` / `REBLUR` / `RELAX` 在 `src/` 与 `Cargo.toml` 全树零命中（机核面由 M162 门 RED 臂承载——接线符号在树即 RED）。

## §5 与自研降噪面 measured 对照

自研降噪管线（M162 门 evidence measured 面；RTX 4070 Ti，RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1，m96_cornell/m96_direct 64×64，raw spp4 → 参照 spp64）：

| 面 | m96_cornell | m96_direct | 溯源 |
|---|---|---|---|
| 噪声谱高频能量下降（低梯度半幅掩码口径，1 − hf(den)/hf(raw)） | 7.697348e-1 | 9.740259e-1 | M162 门 evidence measurements（本波真跑） |
| 帧均值能量相对差 | 2.494836e-3 | 2.736881e-3 | 同上（容差 = 标定 p100×2.0 = 1.271257e-2） |
| 区域均值能量差 p90（8×8 分块） | 8.901973e-2 | 1.051987e-1 | 同上 |
| 历史验证拒绝计数（移动帧） | 96/4096 | 2108/4096 | 同上（活性面 ∈ (0,N) 开区间） |
| 管线形态 | firefly 预钳位（μ±2σ）→ 时域累积（重投影 + 三判据验证 + YCoCg 邻域裁剪 + α=0.1）→ A-trous 3 级（边缘停止：亮度/深度/法线） | 同左 | RXS-0402 / prod_denoise.rs / g12_pt_denoise.rx |

与 NRD 的对照登记（诚实边界）：

1. **形态对照**：NRD REBLUR/RELAX 族 = 时域累积 + 空域多 pass（A-trous 族同源思路）+ albedo demodulation + hit-distance 引导；自研管线同族形态（时域 + A-trous 空域）但**无 demodulation/无 hit-dist 引导**（结构性差距见 §2 #4/#5）。
2. **数字对照**：NRD 公开材料未提供与本门同场景同 spp 同相机的可比对数字——**任何「NRD 降噪优于/同于自研 X%」的定量叙述在本评估期一律不成立**（无 measured 对照面禁凭空引述，P-09）；定量对拍归 G13+ 接入窗（同场景同 spp 双端出图 + 噪声谱/能量守恒对拍口径可复用 M162/M163 门面）。
3. **自研 measured 面结论**：本门标定阈下噪声谱高频能量下降 measured 达标（cornell 0.77 / direct 0.97 vs 阈 0.343）且帧均值能量差 ≤0.27%（无系统性变暗/变亮偏置面）——自研管线在 M96 起步范围冻结面内成立；NRD 类接入的预期增量 = demodulation + hit-dist 引导 + 族变体（occlusion/SH 面），接入裁决须以 G13+ 窗 measured 对拍为法定证据。

## §6 结论（评估不接线）

- RD040-nrd 承接锚**评估面兑现完结**：接入面五轴差距逐项登记（§2）、许可取证（§3，自定义 RTX SDKs 协议非 OSI 许可——owner 法律面审为接入硬前置）、ABI/集成形态取证（§4）、自研 measured 对照面落盘（§5）。
- **不接线维持**：接入另判 G13+ 窗（重判条件 = 接入真实需求 + owner 法律面许可清结 + measured 对拍面接入裁决）；RD-040 总体 status open 维持、backfill_condition 字面 0-byte；本报告落盘经 deferred.json history 只追加登记。
- 评估冒充接入即 RED（RXS-0402 L5）：本报告只登记评估面；树内出现 vendor 降噪接线符号/依赖即 RED（M162 门机核面承载）。

---

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-17 | 初版（G12.3 降噪波 M162；RD040-nrd 承接锚评估面兑现：接入面五轴评估 + 许可/ABI 取证〔2026-08-17 联网实测：NRD v4.17.5 master，自定义 NVIDIA RTX SDKs LICENSE / NOASSERTION〕+ 自研 measured 对照〔M162 门 evidence 转引〕；评估不接线，接入另判 G13+ 窗） |
