<!-- Assisted-by: Kimi-K3（G10.2 波） -->
# G10.2 环境画像与实测登记 — UE5 5.8 参考渲染环境建设

> **状态**：G10.2 实现波首日实测记录 v1.0（2026-08-15）。一切数字来自本机真实命令输出（PowerShell / py -3 / nvidia-smi / Launcher 日志），推测值一律标注。服务 M128 环境画像判据与 RFC-0027 §4.1.5 首日实测登记义务。
> **编号纪律**：本波不领取任何 RXS/CI_step/RD/U/RX 编号；不改 spec/、不改 .github/workflows/。harness 脚本一律未编号，落 `milestones/g10/harness/`。

## 1. 磁盘实测（2026-08-15，`Get-PSDrive -PSProvider FileSystem`，GB 十进制）

| 盘 | 已用 | 剩余 | G10.2 角色 |
|---|---|---|---|
| C: | 447.1 | 83.6 | 系统盘；Launcher 本体所在 |
| E: | 112.3 | 87.6 | UE 源码参照面（只读） |
| F: | 11.1 | 188.9 | **Launcher 安装落盘（实测，见 §3）** |
| H: | 493.1 | 6.9 | 工作区盘，仅小文件（本日志/harness 脚本） |
| K: | 27.6 | 3698.4 | 帧库/语料缓存盘；源码浅克隆所在 |

与 spike v1.0 磁盘表一致（各盘余量无显著漂移）。

## 2. Epic 登录态探测结论：**已登录（持久会话有效）**

- Launcher 安装：`C:\Program Files (x86)\Epic Games\Launcher` = True；版本实测 19.3.3-52955156+++UE5+Release-Distro-5.5（exe FileVersion 19.3.3-0+UE5，自更新完成后复查不变）。
- 历史会话证据（`Saved\Logs\EpicGamesLauncher.log`，2026-05-01 最后运行）：`LogInit: OnSignIn 1` / `AllCheckComplete SelfUpdate=1 SignedIn=1`。
- **本日实测（2026-08-15 11:25:54 UTC）**：启动后自动签到成功——`LogInit: AllCheckComplete SelfUpdate=1 SignedIn=1 RequiresRestart=0`；`FAccountService::OnLoginComplete - 0 TRUE 2171e19133a841228b2a19d308e8dcd1`（AccountId 与注册表 `HKCU\Software\Epic Games\Unreal Engine\Identifiers` 一致）。
- UE EULA 接受态：`EulaService::GetAgreement user has already accepted the agreement unreal_engine2`（HTTP 204，2026-08-15 11:58:23 UTC）——安装流程无 EULA 弹窗。
- **人工接管点结论：本回合未触发**（持久会话直接放行），凭据零接触命令行/日志/仓库（R-G10-2 纪律保持）。

## 3. Launcher 安装臂执行记录（首选路径，立项裁决 2）

### 3.1 机制实测（URI 协议触发安装）

- 官方协议激活面：`com.epicgames.launcher://apps/<AppName>?action=install`（Playnite `GameInstallUrlMask` 同构；dev.epicgames.com 协议激活文档确认 `apps/[SandboxID]:[CatalogID]:[ArtifactId]` 形态与弃用 bare-ArtifactId 兼容）。
- UE 版本 AppName 实证：Launcher 目录元数据行 `ue:18b3b415bc434c5b974f50488360ca31:UE_5.8:5.8.1-56057345+++UE5+Release-5.8-Windows`（`LogSelectiveDownload`，ue57V2.sdmeta）——**AppName = `UE_5.8`，当前可装版本 = 5.8.1**（5.8 线现行补丁版；spike 记 5.8 正式版 2026-06-17 发布，本日为 5.8.1）。
- **陷阱实证（关键工程事实）**：Trae 终端 `Start-Process` 拉起的 GUI 进程运行在 Windows Job 内（Launcher 日志字面「Process is running as part of a Windows Job with separate resource limits」），命令退出后进程被 Job 回收——表现为 Launcher"启动后数分钟内消失"。**解法实测有效**：`Invoke-CimMethod Win32_Process.Create`（WMI 创建于 Job 外）后进程跨命令持续存活。
- 自更新干扰：首次触发 URI 时 Launcher 正在自更新（BuildPatchServices StageFiles），URI 被 EOS 引导链丢弃（子进程以固定参数重启，URI 参数不转发）；自更新收敛后重发 URI 成功。
- **目录元数据时效**：自更新前 sdmeta 缓存仅列 UE_5.0~5.7；自更新完成后 UE_5.8 出现——「5.8 不可见」是缓存时效问题，非账号/区域门槛。

### 3.2 安装落盘事实

- 触发序列：WMI 启动 Launcher（已签到）→ `Start-Process 'com.epicgames.launcher://apps/UE_5.8?action=install'` → 约 60~70 s 后 `Pending\9B5FD42A4EC94136E655F7A72359FA62.item` 落盘，下载自动开始，**未停在人工确认对话框**（是否因 `[<account>_AutoInstall] Checked=True` 或既有偏好放行，机制未完全查明，如实登记）。
- Pending manifest 关键字段（实测）：`AppName=UE_5.8` / `AppVersionString=5.8.1-56057345+++UE5+Release-5.8-Windows` / `InstallSize=31,679,486,448 bytes` / `InstallTags=["","templates","engine_source"]`（含模板与引擎源码组件）/ `InstallLocation=F:\UE_5.8` / `StagingLocation=F:\UE_5.8/.egstore/bps` / `LaunchExecutable=Engine/Binaries/Win64/UnrealEditor.exe`。
- **安装完成（2026-08-15 22:49 本地实测）**：`LauncherInstalled.dat InstallationList` 登记 UE_5.8 @ F:\UE_5.8；`UnrealEditor-Cmd.exe` 在树；最终落盘体积 29.88 GB（目录实测）；下载窗口约 20:03→22:39（含一次 ~20 分钟 CDN 停滞后自愈）；`UnrealEditor-Cmd.exe -version` 冒烟触发 UBT 平台校验正常输出（Win64 VALID 10.0.22621.0）。
- **落盘偏差登记**：任务首选 K:（本波曾将 `GameUserSettings.ini [Launcher] DefaultAppInstallLocation` 由 `E:` 改为 `K:`——用户级非凭据配置，此修改如实留痕），实际 manifest 落 `F:\UE_5.8`（对话框默认路径来源未完全查明）。按 RFC-0027 §4.1.2「Launcher 安装落盘位置由实现波按 spike 磁盘事实选择（E:/F:/K: 均可）」口径，F:（余 188.9 GB ≫ InstallSize 31.7 GB）合规；如后续需迁 K:，按「移动目录 + Launcher 重新定位」程序另走，本波不阻塞。
- 下载速率实测：首段 ~9.5 MB/s（90 s 增 856 MB；20:10:15 本地已 2.68 GB）。长任务后台轮询中。
- LauncherInstalled.dat 安装前状态：`{"InstallationList": []}`——本机无任何既有 UE 安装（`C:\Program Files\Epic Games` 无引擎目录）。

### 3.3 RFC-0027 §4.1.5 F7 事实项①（出图运行时登录态需求）——**已实测**

2026-08-15 晚全部出图冒烟（臂 B HighResShot ×4 跑、MRQ Phase A/B 多跑、`-unattended` 全程）**零登录提示、零凭据交互**，进程正常出帧退出——**Launcher 安装的 UnrealEditor-Cmd 出图运行时不需登录态**（本机实证；Epic 接管点定性为「安装时一次」，非「每次运行前」）。

## 4. 源码编译臂首日实测（备选臂，spike 待验证清单）

证据源：`K:\moon_night_engine(update at May 1st)\references\UnrealEngine`（浅克隆 @4517329fa6e15ac7f4edc96f3a9c65f111745990，2026-04-30；Build.version = 5.8.0 UE5 快照；`GitDependencies.exe` 在树；`Commit.gitdeps.xml` = 36,964,065 bytes）。E: 参照面同版本并存（只读 0-byte 纪律保持）。

### 4.1 依赖规模（`GitDependencies.exe --dry-run --force`，只统计不下载）

- dry-run 输出 133,154 行（133,153 文件全量 Add——浅克隆零二进制，全量待下载；**默认参数不做平台裁剪**，osx/linux 目标文件同在清单内）。
- 维度复核（本波重算，修正 spike 口径）：
  - Blob（89,835 个）Size 求和 = **79,029,880,768 B（79.03 GB 十进制 / 73.6 GiB，全平台解压量上界）**；
  - Pack（11,440 个）Size 求和 = 81.59 GB；**Pack CompressedSize 求和 = 30,438,080,054 B（30.44 GB，全平台实际下载传输量口径）**；
  - spike「Blob 求和 ≈177.9 GB」按同法未能复现（177.9 ≈ Blob+Pack 混合口径 160.6 GB 亦不符），登记为口径偏差，以本波重算为准。
- **Windows 子集启发式实测**（路径关键词剔除非 Windows 平台文件；2 个含 `&` 的 .license 文件因 XML 转义未匹配，忽略）：**115,383 文件 / Blob 字节 49.68 GB（46.27 GiB，解压量）**；非 Windows 17,768 文件 / 34.07 GB。下载传输量按 Pack 压缩比折算约 ~19 GB 级（启发式折算，未逐 Pack 精确归属，标注为估计）。与业界「~40 GB 级」口径相容。
- 裁减面：`--exclude=<folder>` 可剔除非 Windows 文件夹（`--help` 实证），如走编译臂可省 ~32 GiB 解压量。

### 4.2 .NET 兼容性结论（实测 + 源码双重实证，关闭 spike 待验证项）

- 仓库钉死随仓 bundled **.NET SDK 10.0**（`GetDotnetPath.bat`：`UE_DOTNET_VERSION=10.0`，`Engine\Binaries\ThirdParty\DotNet\10.0\win-x64`，随 gitdeps 下载）。
- `UE_USE_SYSTEM_DOTNET=1` 校验路径要求**系统 SDK 主版本 ≥ 10**（同文件 `REQUIRED_MAJOR_VERSION=10`）——**本机系统 .NET SDK 8.0.417 会被拒（exit 1）**。
- 旁证（UE 论坛 2026-06）：UE 5.8 UBT 项目仍目标 net8.0，但引导层钉 SDK 10——「系统 .NET 8 可替代」不成立。
- **结论：源码编译臂必须使用随仓 .NET SDK 10（gitdeps 下载件）；系统 .NET 8.0.417 不被 5.8 编译链接纳。**spike 待验证项「UE_USE_SYSTEM_DOTNET=1 + .NET 8.0.417 是否被接受」答案 = **否**。
- 本机其余编译条件：VS2022 17.14.38 Community（vswhere 实测）；`dotnet --list-runtimes` 含 .NETCore.App 8.0.23 / 6.0.33。
- 编译链命令确认：`Setup.bat` → `GenerateProjectFiles.bat` → `Engine\Build\BatchFiles\Build.bat UnrealEditor Win64 Development`（K: 树批处理实证，与 spike 一致）。

### 4.3 编译臂当前状态

**未启动**（Launcher 首选臂已通路，按裁决 2 编译臂降为备选）。首日可行性结论：K: 空间与工具链无缺口；.NET 走随仓 SDK 10；依赖下载全平台口径 30.44 GB 压缩 / Windows 子集解压 46.27 GiB（启发式）。ue5-main 快照 vs 5.8.1-release 口径差维持 spike 风险登记。

## 5. UE 侧 harness 预制件清单（`milestones/g10/harness/`，未编号，DRAFT 占位可解析形态）

| 文件 | 职责 | host 侧自测 |
|---|---|---|
| `ue_project/G10RefRender.uproject` | 最小内容型工程模板（EngineAssociation 5.8；Interchange + PythonScriptPlugin + MovieRenderPipeline 启用） | JSON 可解析 |
| `ue_python/g10_param_contract.py` | RFC-0026 §4.6 四节闭集 schema 解析 + canonical preimage + SHA-256 + 契约世界系→UE 映射冻结公式；DRAFT_BYTE_LAYOUT 块显式标注「spec 单源冻结后替换」 | digest 实算通过；unit-norm 2^-40 谓词拦截实测（手写非单位四元数被拒） |
| `ue_python/g10_mrq_render.py` | 臂 A/C 复合：MRQ 队列程序化构建（EXR scene-linear 输出、tone curve 关闭 HDR 臂、分辨率/帧率字段化、warm-up 帧、TSR、后处理全关基线）+ provenance 七元组逐帧登记 | 语法可解析（unreal API 待引擎实测） |
| `ue_python/g10_mrq_smoke.py` | MRQ 冒烟 Phase A（LevelSequence + PrimaryConfig 资产程序化生成，含 5.8.1 API 校准记录） | **引擎内实测通过**（资产建/存/出图链路全通） |
| `ue_python/g10_determinism.py` | 确定性协议：warmup 后捕获计划纯函数、帧 SHA-256、双跑 digest 比对、臂 B execcmds 白名单模板、**EXR canonical digest（易变属性闭集剥离 + 偏移表归零）** | 计划/模板函数实测；canonical digest 双跑 4/4 MATCH（§7.3） |
| `ue_python/g10_scene_cornell_box.py` | Cornell Box 程序化搭建草案（契约世界系数值 → 冻结公式映射；标定探针位） | 语法可解析（unreal API 待引擎实测） |
| `ue_python/g10_capture_command.py` | 三臂命令面闭集结构化生成器（参数 token 白名单校验，禁 shell 拼接；命令面 digest 供 capture_arm） | 臂 A/B 生成实测；注入测试 3 例全 RED（schema 外字段 / 分号注入 / 分辨率注入） |
| `examples/contract_params_cornell_box.json` | 契约参数示例（Cornell Box 最小机位，DRAFT 值） | 解析+digest 通过 |
| `examples/mrq_job_cornell_box.json` | MRQ 作业字段化模板（scene_id/map/参数路径/输出目录/臂 id） | JSON 可解析 |
| `probe_gitdeps_dryrun_stats.py` | §4.1 统计脚本（留档可复算） | 本日志全部 gitdeps 数字来源 |

自测样例：示例参数 `param_digest = c6ebe3f60b4d4a1ea9dc4bbd90cf53bb7710546944429aa07b2c9b2ab44c0790`（DRAFT 布局，spec 冻结后失效重算）；UE 映射 `position (0,1,3.2)m → (-320,0,100)cm`、`fov_y 39.6°@16:9 → fov_h 65.2417°`。全部源文件 LF + 尾换行实测（crlf=0 逐文件核验）；`__pycache__` 二进制缓存已清除。

## 6. 环境画像补充

- GPU：NVIDIA GeForce RTX 4070 Ti，驱动 **620.02**（`nvidia-smi`）；时钟未锁（idle 210 MHz / max 3105 MHz；锁频状态随 M141 evidence 另行登记，G10.1 baseline 未锁频边界已知）。
- OS：Windows 11 26H1（10.0.28120.2630，Launcher 日志实测）；CPU i5-13600KF 14C；RAM 31.8 GB。
- 渲染帧/语料缓存目标盘 K:（`RURIX_G10_CACHE_ROOT` 解析序归下游 spec，本波不预建）。

## 7. 首帧出图与双跑 digest 实测（2026-08-15 晚，UE 5.8.1 Launcher 版）

### 7.1 臂 B（HighResShot 快速臂）实测结论

- **分隔符勘误（源码实证）**：`-execcmds` 按**逗号**分隔（`Engine\Source\Runtime\Engine\Private\ParseExecCommands.cpp` ParseExecCmds），分号形态会被解析为单条非法命令静默丢弃（run1 零产出实证）；改逗号后出图成功。
- 首帧出图成功：`UnrealEditor-Cmd.exe <proj> /Engine/Maps/Templates/Template_Default -game -benchmark -fps=30 -seconds=20 -ResX=1920 -ResY=1080 -execcmds="r.ResetViewState, HighResShot 1920x1080" -unattended -log -FixedSeed` → `Saved\Screenshots\WindowsEditor\HighresScreenshot00000.png`（2,198,499 B，目检为真实渲染帧：地板/天空/海洋）。
- **臂 B 双跑 digest 不一致**（三跑三异：DB0FE7A2… / 3BF6E456… / AF18FE69…）——DeferredCommands 首 tick 触发，时域累积/云动画状态未收敛，spike 待验证项「-execcmds 时序稳定性」答案 = **不稳定，臂 B 不能作 M129 证据面**。
- **`-csvCaptureFrames` 死路实证**：附带该参数即静默早退（exit 0、引擎日志未开、零产出），帧钉触发路线（csvExecCmds）此形态不可用。

### 7.2 臂 A/C（MRQ 主路）实测结论——**主路打通**

- 5.8.1 API 校准：`MoviePipelineImageSequenceOutput_EXR`（+`EXRCompressionFormat.NONE`）、`MoviePipelineQueueSubsystem` 为 EditorSubsystem、warmup = `MoviePipelineAntiAliasingSetting.engine_warm_up_count`、**必须含渲染通道（`MoviePipelineDeferredPassBase`），否则 "Shot has 0 Passes" 零输出**；5.8.1 无 `MoviePipelineQueueFactoryNew`（队列资产不可脚本建）；`-ExecutePythonScript` 仅编辑器模式可用（-game 下字面报错拒绝）；编辑器 cmd 模式脚本结束即 QUIT_EDITOR（异步 InProcessExecutor 无法完成渲染）。
- **出图形态 = 两阶段**：Phase A 编辑器 Python 建 LevelSequence + PrimaryConfig 资产（harness `g10_mrq_smoke.py`）→ Phase B 官方命令行 `-game -LevelSequence=… -MoviePipelineConfig=… -windowed -resx/-resy -log -notexturestreaming -Unattended -FixedSeed`（spike 实证形态）——产出 4 帧 EXR（1920×1080 float，NONE 压缩，16.6 MB/帧，`K:\rurix-ext\g10-frames\`）。

### 7.3 双跑 digest 一致性首测（M129 面）

| 场景 | 原始字节 digest | canonical digest（剥易变面） | 像素区（偏移表后） |
|---|---|---|---|
| Template_Default（动画天空/海洋） | 4/4 不等 | — | **全帧像素级不等**（动画内容时域状态漂移） |
| Entry（静态空图） | 4/4 不等 | **4/4 相等** | **4/4 逐字节零差（16,597,440 B/帧）** |

- **UE 5.8.1 EXR 易变属性闭集（82 属性中 14 个跨跑必变，实测归纳）**：`unreal/frameRenderDuration`、`unreal/frameRenderStartTimeUTC`/`EndTimeUTC`、`unreal/jobDate`/`jobDay`/`jobMonth`/`jobTime`/`jobYear`、`unreal/stats/memory/{availablePhysicalMB,availableVirtualMB,peakUsedPhysicalMB,peakUsedVirtualMB}`、`unreal/stats/outputDirectory{TotalFreeMB,TotalSizeMB}`；另扫描线偏移表（1080×u64）随元数据长度差级联漂移。canonical 口径 = 剥离该闭集 + 偏移表归零（harness `g10_determinism.py::exr_canonical_digest`，4/4 MATCH 实测）。
- **结论**：RFC-0026「ue5 strip-and-log」策略实证必要且可行；**MRQ + 静态场景像素级双跑确定性成立**（M129 机器判据实现面打通）；定帧出图必须静态场景（Cornell Box 程序生成路线正确性旁证）。
- 影像证据：帧文件在 `K:\rurix-ext\g10-frames\smoke_entryA\` 与 `…\smoke\`（K: 帧库，不入 git）。

### 7.4 缺项与待验证登记（诚实状态）

| # | 项 | 状态 |
|---|---|---|
| 1 | UE 5.8.1 安装 + 冒烟 | **完成**（§3.2） |
| 2 | 出图运行时登录态需求 | **已实测：不需要**（§3.3） |
| 3 | 首帧出图 + 双跑 digest 首测 | **完成**：臂 B 出图成功/双跑不等（时序不稳实证）；MRQ 主路出图成功/canonical digest 4/4 相等（静态场景面） |
| 4 | `-renderoffscreen` 5.8 可用性 | 未测（本轮出图走窗口模式） |
| 5 | Launcher 装期权实测 | 已登记（InstallSize 31.68 GB manifest / 落盘 29.88 GB 实测） |
| 6 | 落盘 F: vs 任务首选 K: | 偏差已登记 §3.2（RFC 口径合规） |
| 7 | Cornell Box 程序生成场景 + 契约参数相机/光照出图 | harness 草案就位（`g10_scene_cornell_box.py`），引擎内实测待后续会话 |
| 8 | M128/M129 正式 evidence（provenance 七元组随帧落盘 + CI 门） | 编号步骤归后续串行 materialize，本波不领取 |

**DEV_ENV_DEGRADE：无**——Epic 登录接管点未触发（持久会话有效）；Launcher 臂通路；引擎可用；两臂出图实证完成。
