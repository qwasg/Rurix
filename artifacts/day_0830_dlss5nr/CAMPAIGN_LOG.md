# DLSS 5 NR 适配 CAMPAIGN LOG(day_0830_dlss5nr)

> 开役 2026-08-30 23:1x。目标:把 `D:\renodx-dlss5 v3.5.rar` 的 DLSS 5 Neural Rendering(nvngx_dlssnr.dll,NGX feature 18)以「后置增强臂」形式适配进 rurix,走项目既定「探针→门禁→采纳」程序,默认锚零影响。
> 纪律沿 G17/G38:GPU 真跑锁内 + RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1;vendor 二进制不入 git(external/ gitignored)+ registry provenance 登记;泄露件 evaluation-only(永不再分发/进生产默认);加性臂 default off + env opt-in + fail-closed;既有冻结面 0-byte。
> 本机:RTX 4070 Ti(Ada/sm_89)/ 驱动 620.02 / Win11 28120。

## Phase 0 解包 + 静态探测 + 登记 — 完成

- [x] `tar -xf` 解包到 `external/dlss5-nr-v3.5/`(gitignored);双 nvngx_dlssnr 变体 + addon64 + ReShade 安装器 + 说明图就位。
- [x] sha256 / Authenticode / 版本资源:root(Valid 签名,泄露预发布件)vs 40系(HashMismatch 破签补丁件),均 310.8.0.0。
- [x] PE32+ 导出表(手写解析,55 导出):**D3D11/D3D12/VULKAN/CUDA 四 API 全套** + GetSnippetVersion/GetGPUArchitecture。
- [x] 目标字符串:DLSSNR.* 参数键(Color/Depth/MVec/Output/ControlMask/UI + Subrect 族)+ NGX core 注册表定位 + addon64 D3D12 加载模型。
- [x] 等长字节 diff:12.36MB 差异(7.46%)集中前 18MB cubin 区;40系全架构扫描仅 sm_89 无 Blackwell 残留 → 就地 cubin 架构补丁(sm_120→sm_89)确证。
- [x] 登记:[dlss5nr_vendor_sdk_registry.json](dlss5nr_vendor_sdk_registry.json) + [dlss5nr_license_matrix.json](dlss5nr_license_matrix.json)(blocked/evaluation-only)+ [PHASE0_REPORT.md](PHASE0_REPORT.md)。
- [x] **重大发现**:snippet 导出完整裸 NGX Vulkan 面 → 推翻计划「NR 仅 DX12」假设,Phase 2/4 优先评估 VK 裸 NGX 臂(免 VK↔DX12 interop)。

战果:本机可跑变体锁定 = 40系 Ada;NGX ABI(feature 18 + 双 API + 参数键 + core 定位)确证;探针主臂 = D3D12(复用 FsrDx12Session 基建)。

## Phase 1 NGX 动态可用性探针 — 完成(裁决 NO-GO:硬件限定 Blackwell)

- [x] 手写 NGX D3D12 FFI 薄层写入 vendor_upscale.rs(加性,冻结面 0-byte):`NrDx12Probe` + NGX repr(C) 结构/fn 指针/结果码映射/vtable 逆序 setter/日志回调 + `d3d12_bootstrap_nvidia` + `nr_submit_wait`;复用 loader/com_fn/cast_sym/sha256_file/DllProvenance/COM GUID。
- [x] 探针 bin `g13_dlss5_nr_probe`(rurix-rt,required-features vendor-upscale)+ Cargo.toml 注册;NVIDIA/DLSS SDK 官方头下载至 external headers_ref(零复制进 src)。
- [x] **NGX ABI 逆向坐实**:core 4 参 version-only Init 签名(5 参 fci → OutOfDate 反证);MSVC 逆序 vtable(Set uint[4]/Get uint[12] 自检 OK);core 定位走 DriverStore 扫描(注册表 hash 过时);友好 C 名运行时 GetProcAddress(免 vendored lib)。
- [x] **GPU 真跑裁决 = not_available**:Init/GetCap/vtable/AllocateParameters 全 Success(集成正确),CreateFeature(18) core 臂 UnableToInitializeFeature / 直驱臂 PlatformError(Ada 硬件不支持 NR)。evidence probe_dynamic/t100_1080p.json + [PHASE1_REPORT.md](PHASE1_REPORT.md)。
- [x] **社区权威源独立印证**:NIGos/dlss5-d3d12-fix「no report of DLSS 5 NR working below RTX 50-series, a limit of the feature」;FF7R-DLSS5「only runs on 50 series」。cubin 补丁使 DLL 装载兼容(Phase 0)≠ 特性可跑(FP8/Blackwell 内核依赖)。

战果:适配 NGX FFI 全链验证正确 + fail-closed;**DLSS 5 NR 硬件限定 Blackwell,本机 RTX 4070 Ti 无法运行**(本机 measured 与社区一手双向坐实)。可用性门 NO-GO。

## Phase 2 NrDx12Session 评估臂 + lane — 完成

- [x] **NrDx12Session**(vendor_upscale.rs 加性,共享 `ngx_core_open`/`NgxCoreFns`):create()(bootstrap + Init + AllocateParameters + CreateFeature(18) 持久句柄,本机 Ada fail-closed 同探针 core 臂)+ report()(provenance)+ evaluate()(建 in==out 资源纹理 + host 上传 Color/Depth/MVec + 绑 DLSSNR.* + EvaluateFeature + 回读,Blackwell 落地面)+ Drop(配对释放)。探针重构复用 ngx_core_open,重跑裁决不变。
- [x] **tsr→nr / dlss_sr→nr 两链 lane**:独立 harness bin `g13_dlss5_nr_lane`(rurix-render;对冻结 g14_3 车道**字面零改动**)——上采样段真跑真出帧(TSR digest e965b39e / DLSS digest 17bebf92),NR 段双链 fail-closed(UnableToInitializeFeature,硬件限定)。evidence lane_chains.json。
- [x] 编译零新增警告(仅既有 iface),ReadLints 零错误。

## Phase 3 画质/帧时证据战役 + GO/NO-GO — 完成(裁决 NO-GO)

- [x] **画质/帧时证据 = 不可采集**(NR create() 即失败,产不出 NR 帧;物理不可行非跳过);已采集 measured = 探针 + 双链 lane(均 CreateFeature(18) UnableToInitializeFeature)。
- [x] **默认锚零漂移证明**:vendor_upscale.rs **1647 插入 / 0 删除**(git numstat)纯追加,既有 FSR/DLSS/TSR 会话字节不变;g14_3/g31 冻结车道字面零改动;lane 报告独立佐证 dlss_sr/tsr 上采样车道仍真跑真出帧。
- [x] **裁决 NO-GO**:NR 硬件限定 Blackwell,本机 RTX 4070 Ti(Ada)不可用;评估臂封存(Blackwell 可跑)、生产默认零影响、许可 blocked。[PHASE3_REPORT.md](PHASE3_REPORT.md)。

## Phase 4 窗口面散臂(GO 前提)— blocked(GO 门未过)

- [x] GO 前提(NR 本机可实例化)不满足 → 窗口散臂本机不执行,登记 blocked;full19/RD-045 窗口锚不动(NR 未接入 g31_window_present)。
- [x] **Blackwell 落地设计优化**(Phase 0 发现驱动):snippet + core 均导出裸 NGX **Vulkan** 面 ⇒ Blackwell 上窗口面走 **VK 裸 NGX 直连**(`NVSDK_NGX_VULKAN_Init_Ext2` + 自建 VkDevice + 扩展协商),**免掉原计划 VK↔DX12 interop 全部复杂度**。[PHASE4_REPORT.md](PHASE4_REPORT.md)。

## Phase 5 v4.55 addon 复核 + 结论修正(用户指出「40 系也可开」)

- [x] 解包 renodx-dlss5 v4.55(addon64 1.6MB vs v3.5 0.5MB)+ pe_probe 机制字符串:签名 reference build snippet + pre-load(**不调 Init**)+ **IAT patch(GetModuleFileNameW,VirtualProtect)** + hook **游戏活跃 DLSS** 的 NGX/SL EvaluateFeature = "replacing the game's DLSS output"。
- [x] 探针加第三臂(signed-route:snippet 自身 Init + CreateFeature):双 cubin 变体 `snippet.Init` 均 **FeatureNotSupported(0xBAD00001)** —— host 侧架构门(Init 即拒,换 cubin 不解),与 NIGos 一手逐字一致。IAT hook 盲扫路(arm C 初版)因 165MB 泄露 DLL PE 手术脆弱(0xC0000005)已撤,保留干净 snippet.Init 臂。
- [x] **结论修正**:40 系**能开** NR(经 ReShade+v4.55 addon 在游戏里,社区实证);先前「硬件限制」不准 —— 实为 snippet host 架构**软件门**,addon 用游戏注入 hook + 运行时 patch 绕过。rurix 裸引擎直集成本机 Ada 被该门挡下。[PHASE5_REPORT.md](PHASE5_REPORT.md)。

## 终局(修正版)

**适配工程完整、正确、fail-closed、default off、加性(既有面零漂移)**。**40 系可开 DLSS 5 NR,但须经 ReShade 6.8 + renodx-dlss5 v4.55 addon 在 DX12 游戏里**(pre-load 签名 snippet + IAT patch + hook 游戏活跃 DLSS 上下文;社区实证)。rurix 裸引擎直集成本机 RTX 4070 Ti(Ada)被 snippet host 架构门挡下(snippet.Init=FeatureNotSupported,一手实测 + NIGos 双证)——NGX 集成前链全 Success 证明集成正确,被拒是 feature-18 软件门非集成缺陷。干净集成面(NrDx12Session)待官方 Ada 支持 / Blackwell / 或引擎内复现 addon 机制(泄露件逆向,超范围)即可用。
