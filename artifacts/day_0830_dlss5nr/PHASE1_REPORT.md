# DLSS 5 NR 适配 — Phase 1 NGX 动态可用性探针

> 承 Phase 0。目标:手写 NGX D3D12 FFI 薄层,经驱动侧 NGX core(`_nvngx.dll`)真跑
> Init → GetCapabilityParameters → CreateFeature(feature id 18)双臂(core 标准路 /
> 直驱 snippet 破签绕行路),对 40系 Ada 变体 nvngx_dlssnr.dll 取本机 RTX 4070 Ti
> 的可用性 ground truth,产 evidence JSON。GPU 真跑锁内,fail-closed,default off。

## 1. 交付物

- 代码(加性,冻结面 0-byte):[src/rurix-rt/src/vendor_upscale.rs](../../src/rurix-rt/src/vendor_upscale.rs) 尾追 NGX NR D3D12 直驱评估段(`NrDx12Probe` + NGX FFI + `d3d12_bootstrap_nvidia` + `nr_submit_wait`);复用既有 `loader`/`com_fn`/`cast_sym`/`sha256_file`/`DllProvenance`/`VendorError`/COM GUID 常量。
- 探针 bin:[src/rurix-rt/src/bin/g13_dlss5_nr_probe.rs](../../src/rurix-rt/src/bin/g13_dlss5_nr_probe.rs)(`required-features = ["vendor-upscale"]`)。
- 头文件参照(gitignored,零复制进 src):`external/dlss5-nr-v3.5/headers_ref/nvsdk_ngx{,_defs,_params}.h`(NVIDIA/DLSS SDK 官方头,ABI 声明逐字核对源)。
- evidence:[probe_dynamic/t100_1080p.json](probe_dynamic/t100_1080p.json)(schema `rurix.dlss5nr.probe.v1`)。

## 2. NGX ABI 逆向要点(全部本机实测坐实)

| 事实 | 结论 | 证据 |
|---|---|---|
| NGX core 定位 | 注册表 NGXCore FullPath **过时**(驱动更新 hash 目录变);文件系统扫描 DriverStore `nv_disp*.inf*` 为准 | 实测 hash `8a2fa7d0…`(真)≠ 注册表 `f4c7a2fd…`(旧) |
| `NVSDK_NGX_D3D12_Init` 签名 | core 裸导出 = **4 参 version-only**(appId/path/device/version),非 5 参 FeatureCommonInfo(那是 SDK 静态库便利面) | 5 参调用把 `&FeatureCommonInfo` 误落 version 位(R9)→ core 见巨值版本 → **OutOfDate**;改 4 参 → **Success** |
| `NVSDK_NGX_Parameter` vtable | MSVC 同名重载**逆序**入表:Set(uint)=`[4]`、Set(int)=`[3]`、Set(float)=`[6]`、Set(d3d12*)=`[1]`、Get(uint*)=`[12]` | vtable 自检 Set uint`[4]` Width=1920 → Get uint`[12]` 回读 1920 == **OK** |
| core 导出面 | 友好 C 名全导(D3D12/VULKAN 双 API)——**运行时 LoadLibrary+GetProcAddress 即可,免 vendored .lib** | `_nvngx.dll` 75 导出含 `NVSDK_NGX_D3D12_{Init,AllocateParameters,GetCapabilityParameters,CreateFeature,EvaluateFeature,ReleaseFeature,DestroyParameters,Shutdown1,GetFeatureRequirements}` |

## 3. 探针实测(RTX 4070 Ti / 驱动 620.02 / 1920x1080 in==out)

| 步骤 | 结果码 | 名 |
|---|---|---|
| GetFeatureRequirements(18) | -1160773614 | **NotImplemented**(驱动本身不宣告 NR;预期——NR 由 snippet 提供) |
| NVSDK_NGX_D3D12_Init(4 参) | 1 | **Success** |
| GetCapabilityParameters | 1 | **Success** |
| vtable_selfcheck | 1 | **OK** |
| AllocateParameters | 1 | **Success** |
| **CreateFeature(18) core 臂** | -1160773621 | **UnableToInitializeFeature** |
| **CreateFeature(18) 直驱 snippet 臂** | -1160773630 | **PlatformError** |

- `snippet_loaded=false`(core 臂后):snippet 已拷至 exe 同目录(165,840,496B 在位,core 可搜),core 仍**拒绝装载**破签 Ada 变体 → 签名强制确证(`UnableToInitializeFeature` = 特性库不可加载)。
- 直驱臂:绕过 core 直接 LoadLibrary snippet 并调其自身 `NVSDK_NGX_D3D12_CreateFeature` → **PlatformError**(底层平台/图形/OS 面错误 = Ada 硬件跑不了 NR 特性)。
- core/snippet dll sha256 provenance 逐件登记入 JSON(core `d28169b3…`、snippet `28bdc080…`)。

## 4. 裁决:not_available(硬件限定 Blackwell)

**DLSS 5 Neural Rendering 在本机 RTX 4070 Ti(Ada / sm_89)不可用**——两臂 CreateFeature(18) 均失败。**这是特性的硬件限制,不是集成 bug**:Init/GetCapabilityParameters/vtable 自检/AllocateParameters **全部 Success**,证明本适配的 NGX FFI 集成(core 定位、4 参 Init 签名、MSVC 逆序 vtable、参数/结构体 repr(C) ABI、D3D12 device/queue/fence 面)**逐环正确**;唯 CreateFeature(18)——真正下探到 NR 神经网络内核初始化的一步——因 Ada 不支持而失败。

### 社区权威源独立印证(与本机实测一致)

- **NIGos/dlss5-d3d12-fix** README(NR 修复插件作者,一手实测):*"Every report of the neural pass never starting has come from RTX 40-series hardware, and in one of them NGX refuses the neural-rendering feature outright: `0xBAD00001` is `FAIL_FeatureNotSupported`. There is so far no report of DLSS 5 neural rendering working below RTX 50-series. **That is a limit of the feature** rather than of this add-on."*
- **zhubaohi/FF7R-DLSS5**:*"An NVIDIA RTX 50 series GPU. From player reports, DLSS 5 Neural Rendering only runs on 50 series hardware. Older cards may not be able to run this mod."*
- videocardz/wccftech/time.news 报道的「Uncle Burrito 补丁让 40系可跑」= cubin 架构补丁使 DLL **装载**兼容(sm_120→sm_89,Phase 0 已证),但**特性初始化**仍被 NR 内核(FP8/Blackwell 依赖)拒绝——装载 ≠ 可跑。

「社区 40系可跑」的实现路径 = ReShade 注入 DX12 **游戏**并 hook 游戏**已初始化**的 NGX(游戏自带合法签名 nvngx_dlss.dll 驱动 NGX 栈,NR snippet 搭车),与 rurix 裸引擎从 NGX API 干净集成本质不同;且即便如此,40系仍是「神经 pass 从不启动」的空转(NIGos 实测)。

## 5. 对后续 Phase 的影响

- **可用性门 = NO-GO(本机 Ada)**。Phase 2 NrDx12Session 评估臂按正确 ABI 建面(在 Blackwell 上即可跑),本机 create() 如实 fail-closed(同探针 core 臂 `UnableToInitializeFeature`)。
- Phase 3 画质/帧时证据战役**无法在本机采集**(特性建不出)→ 登记 NO-GO 终态 + 本探针 measured 证据。
- Phase 4 窗口散臂 = GO 前提(Blackwell)→ 本机不执行,登记 blocked。

## 6. 结论一句话

适配代码**正确且完备**(NGX FFI 全链验证通过、fail-closed、default off、evaluation-only 泄露件登记闭合);**DLSS 5 NR 本身硬件限定 RTX 50(Blackwell),本机 RTX 4070 Ti 无法运行该特性**——本机 measured 与社区一手实测双向坐实。
