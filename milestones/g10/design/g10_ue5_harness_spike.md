<!-- Assisted-by: Kimi-K3（G10.1 治理波 spike） -->
# G10 UE5 出图环境 Spike 报告 — 本机建立可批量出图的 UE5 5.8 参考渲染环境可行路径

> **状态**：G10.1 治理波 spike 成果 v1.0（2026-08-15）。只读探测完成，**未启动任何大规模下载或编译**（按任务纪律留给实现波）。本报告服务 G10-N1「UE5 5.8 出图环境路径选择」裁决落地（见 `milestones/g10/G10_CANDIDATE_DECISIONS.md` §3 G10-N1 行）。
> **方法**：本机真实命令输出（PowerShell/git/nvidia-smi，全部只读）+ 本地 UE5 源码树实证（E:\Kimi_Agent_Taichi Engine 优化计划\references\UnrealEngine，ue5-main @4517329fa，Build.version=5.8.0）+ Epic/NVIDIA 官方文档联网核查。所有磁盘/体积数字均来自真实命令输出；推测值一律标注「需实现波验证」。

## 问题 1：磁盘空间实测

`Get-PSDrive -PSProvider FileSystem` 真实输出（2026-08-15，单位 GB，四舍五入 0.1）：

| 盘 | 已用 | 剩余 | 评估 |
|---|---|---|---|
| C: | 447.0 | **83.7** | 系统盘，不足以承载编译 |
| D: | 80.0 | 120.0 | 边缘不足 |
| E: | 112.3 | **87.6** | **UE5 源码浅克隆现居此盘——不足以原地编译** |
| F: | 11.1 | 188.9 | 可承载编译但有碎片风险 |
| G: | 97.4 | 102.6 | 不足 |
| H: | 493.1 | **6.9** | **工作区盘，极度紧张（仅够文档/小文件）** |
| I: | 379.0 | 121.0 | 边缘不足 |
| J: | 391.0 | 116.7 | 边缘不足 |
| K: | 27.6 | **3698.4** | **唯一充裕盘（≈3.6 TB），编译落盘唯一推荐位** |

**结论**：UE5 源码编译全量需求（依赖下载 + 解压 + 编译产物，业界口径约 150 GB+，需实现波实测复核）下，**K: 是唯一可行落盘位**；E: 原地编译不可行（87.6 GB < 需求）。H: 盘 6.9 GB 剩余为项目治理面风险（本报告本身即写于 H:），与出图环境无关但需登记。

## 问题 2：源码编译路径（本地树实证）

**证据源**：`E:\Kimi_Agent_Taichi Engine 优化计划\references\UnrealEngine`（下称 `$UE`）。

| 探测项 | 实测结果 | 证据 |
|---|---|---|
| 版本 | 5.8.0，`BranchName=UE5`，`Changelist=0`（ue5-main 快照，**非 5.8.0-release 标签**） | `$UE\Engine\Build\Build.version` |
| 克隆形态 | 浅克隆 @4517329fa（`git rev-parse --is-shallow-repository` = true） | git 命令输出 |
| 源码树体积 | **2.7 GB**（仅源码；无 Engine\Content，无编辑器二进制） | `Get-ChildItem -Recurse` 求和 |
| Setup.bat 机制 | 调用 `Engine\Binaries\DotNET\GitDependencies\win-x64\GitDependencies.exe --prompt` 同步依赖 → 装 VC++ Redist/GameInput Redist → UnrealVersionSelector /register | `$UE\Setup.bat` 全文 |
| GitDependencies.exe | **本地已存在**（浅克隆内含，无需先编译） | `Test-Path` = True |
| 依赖清单 | `Commit.gitdeps.xml` = **35.25 MB / 234,437 行**；`BaseUrl=https://cdn.unrealengine.com/dependencies`；**11,440 个 Pack / 89,835 个 Blob / 133,153 个 File**；Blob 的 Size 属性求和 ≈ **177.9 GB（全平台上界，含 Linux/Mac，Windows 实际子集显著更小，需实现波实测）** | `Get-Item`/`Select-String` 统计 |
| Engine\Content | **不存在**（False）——引擎内容（缺省材质/着色器资源等）随 GitDependencies 下载补齐 | `Test-Path` |
| Engine\Binaries\Win64 | 仅 RadDebugger / UnrealBuildAccelerator / UnrealInstrumentation / ProcessSymbols.bat——**无任何编辑器二进制**，UnrealEditor.exe 必须编译产出 | 目录枚举 |
| 工具链期望 | `GetDotnetPath.bat` 钉死随仓库分发的 **.NET SDK 10.0**（`Engine\Binaries\ThirdParty\DotNet\10.0\win-x64`，当前缺失，随依赖下载）；支持 `UE_USE_SYSTEM_DOTNET=1` 走系统 SDK 校验路径（本机系统已装 .NET SDK 8.0.417，是否被 5.8 UBT 接受需实现波验证） | `$UE\Engine\Build\BatchFiles\GetDotnetPath.bat`；`dotnet --list-sdks` |
| 编译命令链 | `Setup.bat` → `GenerateProjectFiles.bat`（转发 `Engine\Build\BatchFiles\GenerateProjectFiles.bat`）→ `Engine\Build\BatchFiles\Build.bat UnrealEditor Win64 Development`（或 UE5.sln 内编译 UnrealEditor/UnrealEditor-Cmd 目标） | 批处理文件实证 |
| 本机编译条件 | VS2022 17.14.38 ✓；i5-13600KF 14C/20T + 32 GB RAM（全量编辑器编译估计 1.5–3 小时量级，32 GB 内存链接期偏紧，**需实现波实测**） | `Get-CimInstance` 输出 |

**步骤清单（实现波执行）**：① 将浅克隆迁移/重克隆至 K:（重克隆可用 keyring 中 qwasg 账号，其在 EpicGames 组织且 token 含 repo scope；浅克隆迁移后 GitDependencies 按 Commit.gitdeps.xml 当前提交清单下载，不依赖完整历史）→ ② `Setup.bat`（下载 Windows 依赖子集，业界口径 ~40 GB 级压缩包，本清单全平台上界 177.9 GB 仅供参考）→ ③ `GenerateProjectFiles.bat` → ④ `Build.bat UnrealEditor Win64 Development` → ⑤ 冒烟验证 `UnrealEditor-Cmd.exe -version`。**风险**：ue5-main 快照非 release 标签（稳定性弱于 Launcher 正式版）；E: 空间不足必须迁盘；32 GB RAM 链接时长风险；依赖 CDN 大下载的时长/中断风险。

## 问题 3：出图自动化路径（源码 + 官方文档双实证）

**源码实证（5.8 @4517329fa，`Engine\Source\Runtime\Launch\Private\LaunchEngineLoop.cpp`）**：
- `-benchmark` 开关存在：`:2451` `FApp::SetBenchmarking(FParse::Param(FCommandLine::Get(), TEXT("BENCHMARK")))`（非 Shipping 构建）。
- `-fps=N` 固定帧率：`:4845` `FParse::Value(FCommandLine::Get(),TEXT("FPS="),FixedFPS)`。
- `-seconds=N` 到时自退：`:4827` `FParse::Value(FCommandLine::Get(),TEXT("SECONDS="),FloatMaxTickTime)`。
- `-Deterministic` = `-UseFixedTimeStep -FixedSeed` 快捷方式（`:2457-2462`）；benchmarking 隐含 FixedSeed。
- 游戏态渲染使能：`:5049` `FViewport::SetGameRenderingEnabled(true, 3)`（非编辑器路径）。

**官方文档实证**：
- 命令行参数总页（dev.epicgames.com/documentation/unreal-engine/command-line-arguments-in-unreal-engine）：`-game` 模式、URL 地图参数、`-ResX=/-ResY=` 键值对语法确认。
- 截屏文档（taking-screenshots-in-unreal-engine）：`HighResShot filename=PATH 3840x2160 ... bCaptureHDR` 控制台命令，输出至 `Saved\Screenshots\Windows`，支持 EXR/HDR 与 buffer dump。
- MRQ 命令行官方页（using-command-line-rendering-with-move-render-queue-in-unreal-engine）：三种模式——`-LevelSequence`+`-MoviePipelineConfig`(PrimaryConfig 预设)、`-MoviePipelineConfig`(Queue 资产批量)、Python Executor 自定义。官方示例形态：`UnrealEditor-Cmd.exe "<proj>.uproject" <map> -game -LevelSequence="/Game/..." -MoviePipelineConfig="/Game/..." -windowed -resx=1280 -resy=720 -log -notexturestreaming`。
- `-renderoffscreen` 无头离屏渲染开关见厂商/社区文档（UE5.6 实例，offworld.live 知识库），**5.8 可用性需实现波验证**。

**本地插件实证**：`Engine\Plugins\MovieScene\MovieRenderPipeline\MovieRenderPipeline.uplugin` 存在 ✓（MRQ 随引擎源码，无需外购）。

**可行命令形态（供实现波选用，标注验证义务）**：
1. **快速截屏臂（轻量）**：`UnrealEditor-Cmd.exe <proj>.uproject /Game/Maps/<Map> -game -benchmark -fps=30 -seconds=N -ResX=1920 -ResY=1080 -execcmds="r.ResetViewState; HighResShot 1920x1080" -unattended -log -FixedSeed` —— `-benchmark/-fps/-seconds/-FixedSeed` 源码已实证；`-execcmds` 触发 HighResShot 的时序（需等流送/PSO 编译稳定后再截）**需实现波验证**。
2. **MRQ 批量臂（推荐主路）**：队列资产 + `UnrealEditor-Cmd.exe <proj>.uproject <map> -game -MoviePipelineConfig="/Game/Cinematics/<Queue>" -windowed -resx=1280 -resy=720 -log -notexturestreaming -Unattended`（或 `-renderoffscreen` 无头）——官方文档实证；MRQ 原生支持逐帧 PNG/EXR 序列、warm-up 帧、控制台变量覆盖，最契合「确定性参考帧批量产出」。
3. **Python 编排臂**：`-ExecutePythonScript=<script>.py` + `MoviePipelineQueueSubsystem` 回调完成即 `quit_editor()`（社区教程实证模式），适合多场景多机位批处理编排——**需实现波验证**。

## 问题 4：场景工程与 glTF 导入（5.8 内置情况）

- **glTF 导入 5.8 内置确认**：`Engine\Plugins\Interchange\Runtime\Interchange.uplugin`（`EnabledByDefault: true`）模块清单声明 **GLTFCore** 运行时模块；同插件另有 InterchangeFbxParser（FBX，官方文档标注 Interchange 路径 FBX 为 Experimental，经典 FBX 管线仍在）。官方文档（importing-gltf-files-into-unreal-engine，5.5/5.6 版面）确认：glTF(.gltf/.glb) 经 Interchange 框架导入，支持单资产导入与 **File > Import Into Level 整场景导入**。另 `Engine\Plugins\Enterprise\GLTFExporter\GLTFExporter.uplugin` 存在（出口侧）。
- **最小工程方案**：内容型（Blueprint/纯资产）`.uproject` + 空白关卡 + Interchange 导入 glTF/FBX 压测资产——**无需编译任何项目 C++ 代码**；Launcher 版甚至无需 VS。批量化可用 Python（PythonScriptPlugin）做导入/布光/摆机位/注册 MRQ 队列（实现波工作量）。
- 结论：场景工程无许可/插件缺口；压测资产选型（glTF 样例库如 Khronos glTF-Sample-Assets、或自导出 FBX）归实现波。

## 问题 5：三路径对比与裁决建议

| 维度 | ①源码编译 | ②Launcher 安装 | ③仅源码对照+公开参考图 |
|---|---|---|---|
| 版本口径 | ue5-main 快照 5.8.0 @4517329fa（非 release 标签） | **官方 5.8 正式版**（2026-06-17 发布，forums.unrealengine.com/t/unreal-engine-5-8-released/2729274；Launcher/GitHub/Linux 三渠道） | 不定（公开图版本参差） |
| 到首张参考帧时长 | 依赖下载（~40 GB 级）+ 编译 1.5–3 h + 迁盘成本 | **~40 GB 下载即得**（最快） | 零（但不可用） |
| 磁盘 | 150 GB+，仅 K: 可行（E: 87.6 GB 不足） | ~40–60 GB（视内容选项），E:/F:/K: 均可 | ≈0 |
| 账号依赖 | 依赖 CDN 无需 Epic 登录；重克隆需 Epic 关联 GitHub（qwasg 已具备） | **必须 Epic 账号交互登录**（官方安装文档字面「必须登录才能下载虚幻引擎」；Launcher 已安装 `C:\Program Files (x86)\Epic Games`=True，登录状态未知，需人工一次介入） | 无 |
| 可机核性 | 可批量出图 ✓ | 可批量出图 ✓ | **不可机核**（仅人工对照材料） |
| 渲染器可改性 | 可改源码/加探针（G11/G12 深改潜力） | 不可改（插件层可用） | — |
| 二进制出处 | 自编译（可复现性依赖本机工具链） | **官方签名正式版（对标基线出处最干净）** | — |
| 风险 | 快照稳定性、编译时长、内存链接风险 | 登录人工介入；下载体积 | 不满足 G10 验收证据要求 |

**裁决建议：首选 ②Launcher 安装 UE 5.8 正式版**。理由：(a) G10 目标是「批量出参考帧做 A/B 度量」，Launcher 正式版以最短路径给出出处最干净的 5.8 基线二进制，MRQ/HighResShot/glTF 导入全开箱可用；(b) 源码编译的全部增量价值（改渲染器、加探针）在 G10 画面对标期**无真实消费方**（G10-N1 裁决语义：出图环境形态，非引擎改造）；(c) 本机磁盘实测下源码编译必须迁盘 K: 且耗时显著更长。**①源码编译降为增强备选**：若 G11/G12 出现渲染器插桩/改造真实需求，或 Launcher 登录被阻断且人工介入不可得，再启动（K: 空间与 qwasg 账号已核查可行）。**③仅作兜底对照材料**，不进验收证据链。唯一人工门槛 = Epic 账号登录一次。

## 问题 6：DLSS 事实核查（服务 G13 规划）

| 事实项 | 核查结论 | 证据 |
|---|---|---|
| Streamline SDK 获取 | **GitHub 开源**（github.com/NVIDIA-RTX/Streamline，即原 NVIDIAGameWorks/Streamline）；仓库最新 release 2.9.0（2025-08-26）；developer.nvidia.com/rtx/streamline/get-started 列 2.10.3 为最新下载 | GitHub 仓库页 + NVIDIA 官网 |
| Streamline 许可 | **MIT**（license.txt 全文为 MIT 授权条文；仅 sl_nvperf.h/.dll 两件受 NSight Perf SDK License 2023.3 约束） | 仓库 license.txt 直读 |
| DLSS 插件二进制 | Streamline 仓库提供**签名预编译 DLL**；DLSS-G 仅预编译无源码，其余插件源码可自编译 | 仓库 README |
| 独立 DLSS SDK | developer.nvidia.com 下载**需 NVIDIA Developer Program 会员（免费注册）**；DLSS EULA 禁止再分发 SDK 本体，但签名 nvngx_dlss.dll 等**可随应用再分发**（标准做法） | kajiya 项目 using-dlss.md + NVIDIA 论坛 |
| Vulkan 支持面 | DLSS SR on Vulkan：官方样例 **nvpro-samples/vk_streamline**（Apache-2.0）实证可行；DLSS-G on Vulkan 默认 interop 模式，原生 Vulkan 需驱动 ≥527.64 + VK_API_VERSION_1_1+；DLSS RR on Vulkan 经 Streamline 支持（ProgrammingGuideDLSS_RR，**版本矩阵需 G13 前复核**） | GitHub nvpro-samples + Streamline docs |
| 驱动门槛 | Streamline 要求 NVIDIA 驱动 ≥512.15；**本机 620.02 ✓** | nvidia-smi 实测 + NVIDIA 官网 |
| RTX 4070 Ti 能力 | Ada 架构：DLSS SR（含 DLSS 4 transformer 模型，驱动侧可升级）、DLSS RR、DLSS FG（40 系单帧生成）均支持；Multi-Frame-Gen 为 50 系独占（与本卡无关项） | NVIDIA DLSS 4 FAQ（论坛 kb） |
| UE5 侧 DLSS | 引擎源码树**不含 DLSS 插件**（`Engine\Plugins` 全树 glob `*DLSS*` 零命中——实证）；NVIDIA 以独立 UE 插件形式分发（developer.nvidia.com DLSS get-started 页） | 本地 glob + NVIDIA 官网 |
| 对 Rurix（Vulkan 渲染器）的意义 | G13 可行路径 = Streamline（MIT）+ 签名 DLSS DLL 集成本机 Vulkan 后端；4070 Ti 硬件面与驱动面无缺口 | 综合上行 |

## 风险清单（登记，实现波前必读）

1. **H: 盘仅剩 6.9 GB**——治理/证据文件持续增长的系统性风险，与出图环境无关但优先级最高。
2. **Epic 账号登录状态未知**——Launcher 路径唯一人工门槛；若登录受阻，回退路径①（qwasg GitHub 凭据已核查可用）。
3. **E: 盘不可原地编译**（87.6 GB < 150 GB+）——路径①必须先迁盘 K:，迁移/重克隆本身有成本。
4. **ue5-main 快照 ≠ 5.8.0-release**——路径①产出的基线与官方正式版可能存在渲染差异；若 G10 验收钉「官方 5.8 正式版」口径，路径①需先 checkout release 标签（依赖清单随标签变化，需重新 Setup）。
5. **依赖下载规模不确定**——gitdeps 全平台 Blob 上界 177.9 GB；Windows 子集实际规模未实测（业界口径 ~40 GB 压缩级），实现波首日即实测登记。
6. **32 GB RAM 链接风险**——UnrealEditor Development 全量链接在 32 GB 上偏紧，可能需要 `/p:MaxCpuCount` 降并发或加大分页文件（实现波预案）。
7. **`-execcmds` + HighResShot 时序**、**`-renderoffscreen` 在 5.8 的可用性**、**系统 .NET 8 替代随仓 .NET 10 的 UBT 兼容性**——三项均已标注需实现波验证，不在本 spike 下结论。
8. **DLSS DLL 再分发边界**——G13 集成前需逐字核 DLSS SDK EULA「随应用分发签名 DLL」条款字面（本 spike 仅确认通行做法与获取路径）。

## 实现波待验证清单（本 spike 不下结论项汇总）

- Windows 依赖子集实际下载/解压体积与时长（K: 盘实测）。
- `Build.bat UnrealEditor Win64 Development` 在 i5-13600KF/32GB 上的实测编译时长。
- `UE_USE_SYSTEM_DOTNET=1` + 系统 .NET 8.0.417 是否被 5.8 UBT 接受（否则用随仓 .NET 10）。
- `-renderoffscreen` 在 5.8 正式版/源码版的无头出图可用性。
- `-execcmds="HighResShot ..."` 的触发时序稳定性 vs MRQ 臂的逐帧确定性对比（选臂依据）。
- Launcher 5.8 安装选项（目标平台/内容勾选）对磁盘占用的实测值。
