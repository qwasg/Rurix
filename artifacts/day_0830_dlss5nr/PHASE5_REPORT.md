# DLSS 5 NR 适配 — Phase 5 v4.55 addon 复核 + 结论修正

> 触发:用户指出「40 系也可以开启」,提供 `ReShade_Setup_6.8.0_Addon.exe` +
> `renodx-dlss5.addon64 v4.55.zip`。本阶段复核 v4.55 机制,修正 Phase 1/3 过强的
> 「硬件限制 / NO-GO」框定,给出 40 系可用的真实路径。

## 1. 结论修正(用户是对的)

**RTX 40 系确实可以开启 DLSS 5 NR** ——通过 ReShade + renodx-dlss5 v4.55 addon 在 DX12
**游戏**里。我 Phase 1/3 说的「硬件限制」框定**不准确**:NR 神经内核**能在 Ada 硬件上执行**
(Uncle Burrito 的 Ada cubin 补丁已在 4090/4080 实证);真正挡路的是 **snippet host 侧的
架构门**(Init 阶段查 GPU 架构,对非 Blackwell 返 `FeatureNotSupported`)——这是**软件门**,
addon 用「运行时 patch + hook 游戏活跃 DLSS 上下文」绕过它。

## 2. v4.55 机制(比 v3.5 高明:1.6MB vs 0.5MB addon64)

pe_probe 字符串实证 v4.55 用的是 NVIDIA **签名 reference build** `nvngx_dlssnr.dll`(非破签
补丁),运行时做:
- **pre-load 签名 snippet**("signed NR runtime pre-loaded at device init"),**但不调其自身
  Init**("loaded but never initialized; leaving it mapped")。
- **IAT patch**:"make signed-feature **IAT writable**" + hook **GetModuleFileNameW**(snippet
  自路径解析重定向;VirtualProtect/VirtualProtectEx 导入实证)。
- **hook 游戏活跃 NGX**:`NVSDK_NGX_D3D12_EvaluateFeature`(+`_C`)+ `slEvaluateFeature`
  (EnableHooks=1 Streamline / =2 NGX-only)——"**replacing the game's DLSS output**"。
- 依赖:游戏已跑 DLSS(NGX 经**签名** nvngx_dlss.dll 建活跃 feature)+ 兼容驱动
  ("Update your NVIDIA driver"、"signed NR runtime failed to initialize (driver/runtime version)")。

⇒ **本质 = 游戏注入 hook**:piggyback 游戏已初始化并运行中的 DLSS/NGX 上下文,在其
EvaluateFeature 上叠 NR;feature-18 的创建走的是**被 patch 过的 core + 活跃游戏上下文**,
不是裸进程 snippet.Init 直路。

## 3. 本机新增 measured 证据(探针第三臂 signed-route)

在探针加了 addon 式「snippet 自身 Init + snippet.CreateFeature」臂,双 cubin 变体实测
([probe_dynamic/t100_v1_route.json](probe_dynamic/t100_v1_route.json) / t100_signed_route.json):

| 变体 | snippet.Init | CreateFeature(18) |
|---|---|---|
| 40系专用v1(Ada cubin,破签) | **FeatureNotSupported**(0xBAD00001) | PlatformError |
| root(Blackwell cubin,签名) | **FeatureNotSupported**(0xBAD00001) | PlatformError |

两变体 snippet.Init 均 **FeatureNotSupported** ——**host 侧架构门在 Init 阶段拒绝**(早于 cubin
执行,故换 cubin 不解),与 NIGos/dlss5-d3d12-fix 一手记录**逐字一致**(「0xBAD00001 =
FAIL_FeatureNotSupported ... a limit of the feature」)。addon 不走此路(不调 snippet.Init),
故不撞此门。

## 4. 为何 rurix 裸引擎 ≠ addon 可跑

| | addon(可跑 40 系) | rurix 裸引擎直集成(本机被拒) |
|---|---|---|
| 上下文 | DX12 **游戏**已跑 DLSS(NGX + 签名 nvngx_dlss.dll 活跃 feature) | 无游戏 DLSS;自建裸 NGX |
| snippet 驱动 | pre-load + IAT patch + hook 游戏活跃 EvaluateFeature | snippet.Init 直路 → FeatureNotSupported |
| feature-18 创建 | 被 patch 的 core + 活跃上下文 | core UnableToInitializeFeature / 直驱 PlatformError |

裸引擎要复现 addon,须**重写 addon 的 hook + 运行时 patch**(IAT/GetModuleFileNameW +
可能的 host 架构门 patch + 依附一个活跃 DLSS 上下文)——对 165MB 泄露专有 DLL 做深度
逆向内存手术,脆弱、许可越界(泄露件),超出干净引擎集成范围。

## 4.5 引擎内复现 addon 机制的尝试(用户选路)+ 精确阻断点

用户选「在 rurix 引擎内硬复现 addon 机制」。做了 dumpbin 导入表 + llvm-objdump 反汇编 +
探针加 addon 式臂(snippet 自身 Init + IAT hook)+ 两轮诊断,精确定位了阻断点:

- **addon 导入实证**:`VirtualProtect(Ex)` + `FlushInstructionCache` + `GetModuleFileNameW` +
  `LoadLibraryEx*` + `GetProcAddress` + `bcrypt`(sha256 校验 reference build)——即
  **运行时内存代码补丁**(FlushInstructionCache = patch 可执行代码)+ IAT hook,NGX 全动态解析。
- **GetModuleFileNameW hook 实测无效于 Init 门**:探针按真指针值扫 IAT 目录稳健命中并 hook
  了 snippet 的 GetModuleFileNameW(命中 1 槽),但 **`snippet.Init` 的 FeatureNotSupported
  发生在任何 GMFNW 调用之前**(GMFNW 首次调用在 Init 之后的 CreateFeature 阶段,查询宿主
  EXE 路径)——**故 Init 门不是基于 host exe 路径的 app 白名单门**。
- **snippet 无静态架构查询导入**:dumpbin 实测 nvngx_dlssnr.dll 仅导入 VERSION/ADVAPI32/
  USER32/KERNEL32——**无 nvcuda/nvapi/d3d12 静态导入**,GPU 架构查询走**动态解析**(GetProcAddress
  的 CUDA / 传入的 ID3D12Device 查询 / 或 NGX core 内)。**故无干净 IAT hook 点可谎报架构**。

**结论(精确阻断点)**:snippet.Init 的 `FeatureNotSupported(0xBAD00001)` = **动态 GPU 架构门**
(Init 期查物理 GPU 算力,见 Ada/sm_89 即拒;无静态 hook 点)。引擎内绕过唯二途径:①像 addon
一样 `VirtualProtect+FlushInstructionCache` **改 snippet host 代码**(patch 架构门 + 提供 Ada
cubin);②hook snippet 的 `GetProcAddress` 拦架构查询函数 + 用 v1 Ada cubin + 熬过内核启动。
二者皆 = 对 **165MB 泄露专有 DLL 的多步盲改**(un-annotated 40 万行反汇编抽 patch 字节 / 多 MB
in-memory cubin 补丁带 file→RVA 映射 / GPU 崩溃风险逐轮迭代),**且许可越界**(泄露件)。
**判定:不可在此可靠交付,超工程边界**——精确阻断点已如实登记,探针保留 3 臂 + IAT hook +
架构门诊断供未来(或有反汇编环境时)复用。evidence: probe_dynamic/t100_archgate_diag.json。

## 5. 40 系可用的真实路径(交付用户)

**A. 直接用(游戏,社区验证路)**:ReShade 6.8 Addon 版 + renodx-dlss5 v4.55 addon +
**NVIDIA 签名** nvngx_dlssnr.dll,放进 DX12 游戏 exe 目录,ReShade overlay 勾选 "Enable DLSS
Neural Rendering"(须 `[ADDON] LoadFromDllMain` + 兼容驱动)。这是 addon 的设计用途,40 系
可跑(视驱动/游戏)。

**B. rurix 引擎面(本项目)**:NrDx12Session / 规划中的 NrVkSession = 干净 NGX 集成(FFI 全链
验证正确、fail-closed、default off、evaluation-only),在 **NR 特性被官方支持**(NVIDIA 正式
DLSS 5 SDK,Ada 官方档)或 **Blackwell 硬件**上即直接可用;当前 Ada 被 snippet host 架构门
挡下(如实登记)。若确需本机 Ada 引擎内启用,唯一途径 = 在 rurix 进程内复现 addon 的
hook+patch 机制

## 6. 许可(v4.55 同 v3.5 纪律)

renodx-dlss5 v4.55 addon + 泄露 nvngx_dlssnr.dll = evaluation-only,blocked_for_redistribution,
不入 git、不再分发、不进生产默认(external/ gitignored;登记见 dlss5nr_vendor_sdk_registry.json)。

## 7. 一句话(修正版)

**40 系能开 NR(经 ReShade+v4.55 addon 在游戏里,社区实证)——我先前「硬件限制」说法不准,
实为 snippet host 架构门(软件门)被 addon 的游戏注入+运行时 patch 绕过。** rurix 裸引擎直
集成本机 Ada 被该门挡下(snippet.Init=FeatureNotSupported,一手实测 + NIGos 双证);干净集成
面已就绪,待官方 Ada 支持 / Blackwell / 或引擎内复现 addon 机制(泄露件逆向,超范围)。
