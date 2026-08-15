<!-- Assisted-by: Kimi-K3（G10.1 治理波起草） -->
# G10_CAPABILITY_MATRIX — UE5 画面对标基线期能力 → Rurix 现状与缺口矩阵

> **所属**：G10 文档集（`milestones/g10/`；计划状态见 [G10_PLAN.md](G10_PLAN.md)——**G10 未立项，本文不构成契约/验收承诺**）。上游输入：[G10_PLAN.md](G10_PLAN.md)（波次与 P0 建议）· [G10_CANDIDATE_DECISIONS.md](G10_CANDIDATE_DECISIONS.md) v1.0（候选决策表起草稿）· [design/g10_ue5_harness_spike.md](design/g10_ue5_harness_spike.md) v1.0（G10.1 治理波 UE5 出图环境 spike 只读探测成果）· [G9_CAPABILITY_MATRIX](../g9/G9_CAPABILITY_MATRIX.md) v1.0（矩阵范式与顺延基线）· [G9_P2_DECISIONS](../g9/G9_P2_DECISIONS.md) v1.0（十项 defer-to-G10+ 承接锚）· [渲染器调研](../../渲染器调研/) 七份报告（2026-07-28 快照）+ 本文 §0 立项前事实基线。下游消费者（G10.1 治理波）：G10_CONTRACT / G10_CANDIDATE_DECISIONS / G10_ACCEPTANCE_MAP。
> **基准日**：2026-08-15（G9 closed 终态 + G9.7 P2 决策表 v1.0 + deferred.json 只读引用 + 本地 UE 5.8 源码树实测）。
> **行号纪律**：行号 `M##` **顺延 G9 矩阵**（G9 止于 M127，本文自 **M128** 起），保持映射连续性；M## 为文档内部定位标识，**非 ledger 编号**。本文零编号占用——不新设/不消费任何 RFC/RD/RXS/SG/CI/U 编号，仅只读引用。
> **优先级纪律**：「建议 P 级」与「拟承接」均为计划建议值，G10 立项时经 G10.1 候选决策表重新裁决后硬化；**UE5 出图路径、压测场景首发清单、度量口径冻结未经治理裁决，相关行标「待定（G10.1 治理波裁决）」处不擅自定案**。
> **defer 纪律**：G9 十项 defer-to-G10+ 维持 open 为法定输入；其重判不得以「对标 UE5 目标」静默改写——触发证据（G10.5 差距清单 measured 数据 / 其他 measured 举证）须逐行留痕，deferred history 只追加（G8.7/G9.7 纪律继承）。
> **图例**：✅ 已交付 · 🟡 部分 · ⬜ 缺失；档位 A 语言/编译器 · B 运行时/RHI · C 引擎库 · D 工具/资产；验收五层级缩写 = **核** 核心等价 · **环** 功能闭环 · **降** 可降级 · **产** 可生产化 · **主** Vulkan 主线；4070Ti = 可否真机验证。

---

## 0. 立项前事实基线（行证据的共同来源）

1. **G9 已收口**：34 key（15 P0 + 19 go P1）全绿（[G9 CI_GATES](../g9/CI_GATES.md) v1.21 步骤 172 `g9.wave.8b.closeout` 终审门 materialize；34 key = [G9_ACCEPTANCE_MAP](../g9/G9_ACCEPTANCE_MAP.md) §2/§3 实记，[G9_P2_DECISIONS](../g9/G9_P2_DECISIONS.md) §2.1 字面）。覆盖：虚拟几何 cluster/CLAS（M90~M95）、GI Surface Cache/SPG/降级链（M96~M101）、GPU-driven DGC/descriptor（M102~M107）、大世界分区/HLOD（M110/M111）、显示管线 ACES/AgX（M118）、物理 Field/Jolt（M121~M126）。
2. **十项 defer-to-G10+ 承接锚**（[G9_P2_DECISIONS](../g9/G9_P2_DECISIONS.md) §1/§3，逐行字面见本矩阵 §6.3）：M61 mesh shader · M52 SER · M99-clipmap 世界辐射缓存 · M100-high ReSTIR 高档 · SAFE-GPU · M127 神经变形 · M98-l4 Far Field · M114-strand 毛发精确 OIT · M118-hdr-cal HDR 标定 · M125-adopt3 Jolt5.6 三件。全部经 G10.6 重评窗逐行重判。
3. **G9/G8 交付底座（只消费不重定）**：M96 路径追踪参照器（pbrt-v4 对齐，`g9.p0.m96` 门绿）· G8 M24/M25 时域/超分底座（TSR 时域底座与 UpscaleBackend 抽象已存在，[G9_CAPABILITY_MATRIX](../g9/G9_CAPABILITY_MATRIX.md) §0.4 字面）· M118 显示管线四插件（ACES 1.3/2.0/AgX/中性）· G8 M81 glTF 导入通道 · M79~M83/M88 资产闭环。
4. **仓库缺口（G10 主攻面）**：无图像质量度量工具（FLIP/SSIM/PSNR）· 无画面对比 harness · 无压测场景资产（仅 fixture 生成器 `ci/_gen_m81_gltf_fixtures.py`）· 无 DLSS 接入（UpscaleBackend 抽象在、vendor 后端未接）。
5. **本地 UE5 参考环境**：`E:\Kimi_Agent_Taichi Engine 优化计划\references\UnrealEngine`（UE 5.8.0 源码，ue5-main @4517329fa）——`Engine\Source\Runtime\Renderer` 完整源码树经实测在树：`Private\Nanite\`、`Private\Lumen\`、`Private\MegaLights\`、`Private\VirtualShadowMaps\`、`Private\HairStrands\`、`Private\PostProcess\TemporalSuperResolution.cpp`、`Private\PathTracing.cpp`、`Private\SceneCaptureRendering.cpp`、`Private\GPUBenchmark.cpp` 等模块路径均可引用；**无 Content、无编译产物、不可直接出图**。本机无 UE5 编辑器二进制；VS2022 17.14 + Vulkan SDK 1.3.296 + RTX 4070 Ti（驱动 620.02）齐备；GitHub 账号 qwasg 在 EpicGames 组织。**G10.1 spike 已落**（[design/g10_ue5_harness_spike.md](design/g10_ue5_harness_spike.md) v1.0，只读探测）：出图路径三臂对比与裁决建议（②Launcher 安装 5.8 正式版首选 / ①源码编译增强备选仅 K: 盘可行 / ③公开参考图仅兜底）、出图自动化候选臂（MRQ 批量 / HighResShot 快速 / Python 编排）、glTF 导入 5.8 内置确认、DLSS/Streamline 事实核查（G13 服务面）；风险登记含 **H: 盘仅剩 6.9 GB**（治理面容量风险）。
6. **G10~G15 分期（立项裁决口径）**：G10 基线 → G11 修复 → G12 路径追踪生产化 → G13 超分与 DLSS → G14 性能优化 → G15 商用收口。G10 只交基线与差距清单，不承诺修复（[G10_PLAN](G10_PLAN.md) §1）。

---

## 1. D1 UE5 出图与参考帧 harness

| 行 | 能力（子面） | G9 承接锚 | backfill / 触发条件字面 | UE5 参照（Renderer 源码模块路径） | 现状 | 缺口要点 | 档位 | 建议 P 级 | 验收五层级 | 4070Ti | 拟承接 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| M128 | UE5 出图环境建立：路径裁决落地（spike 建议 = Launcher 5.8 正式版首选 / 源码编译增强备选 / 公开图兜底）+ 环境画像（UE build digest/驱动/锁频）随证据存档 | —（G10 新立） | 用户指令「完整渲染帧与 UE5 出图对比」——G10 法定输入；路径最终裁决归 G10.1 治理门（spike 报告为输入） | `Engine\Source\Runtime\Renderer`（整体出图面）· `Private\SceneCaptureRendering.cpp`（捕获参照） | 🟡 G10.1 spike 已落（只读探测 + 裁决建议 + 待验证清单）；本机无编辑器二进制；源码无编译产物 | 路径未裁决（**待定（G10.1 治理波裁决）**）；依赖下载/安装实积时长未实测（R-G10-1）；Epic 登录人工介入面（R-G10-2）；H: 盘容量风险（R-G10-11） | D | P0 | 环·产 | ✔ | G10.2 |
| M129 | UE 参考帧批量出图与 provenance 库：固定场景/相机/光照批出 HDR 帧 + 同参数双跑帧 digest 一致 + provenance 登记闭集 | M128 | TSR/时域累积收敛协议（固定 seed + warmup 帧，R-G10-7）；出图自动化候选臂 = MRQ 批量主路 / HighResShot 快速 / Python 编排（spike 问题 3） | `Private\SceneCaptureRendering.cpp` · `Private\Renderer.cpp` | ⬜（spike 已证 MRQ/HighResShot 文档与开关面在树，harness 未建） | 批量出图 harness；双跑 digest 一致协议；provenance（场景/相机/光照/build）schema；选臂依据实测登记 | D | P0 | 环·产 | ✔ | G10.2 |
| M130 | 双端场景确定性契约：相机/光照/时间参数同 schema 双端各一份 + digest 比对 + 双端解析一致 | M128/M129 | A/B 硬前置——digest 不等不得出报告（R-G10-6） | `Private\SceneRendering.h` · `Private\HaltonUtilities.cpp`（jitter 序列参照） | ⬜ | 参数 schema 冻结；digest 比对门；漂移/schema 外字段注入 RED 臂 | D | P0 | 核·环 | ✔ | G10.2 骨架 → G10.5 双端核验 |

## 2. D2 压测语料

| 行 | 能力（子面） | G9 承接锚 | backfill / 触发条件字面 | UE5 参照 | 现状 | 缺口要点 | 档位 | 建议 P 级 | 验收五层级 | 4070Ti | 拟承接 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| M131 | 压测场景获取与许可登记：Bistro/Sponza/CornellBox 等 glTF/FBX 场景与材质资产；许可白名单（CC0/CC-BY 族）+ SPDX + 来源 URL + attribution + 资产 digest | —（G10 新立；G15 商用收口前置） | 商用可再分发硬性约束（R-G10-4）；首发清单**待定（G10.1 治理波裁决）** | —（外部资产面） | ⬜ 仓库零真实压测资产（仅 `ci/_gen_m81_gltf_fixtures.py` fixture 生成器） | 联网获取与镜像多源（R-G10-9）；逐资产许可核验登记；白名单外注入 RED；禁以 fixture 冒充真实资产 | D | P0 | 环·产 | ✔ | G10.3 |
| M132 | Rurix 场景加载门：清单逐场景加载成功 + 三角形/材质/纹理计数非空 + 加载事件序列 golden | G8 M81 glTF 导入通道（[G9 矩阵](../g9/G9_CAPABILITY_MATRIX.md) §0.4 资产闭环字面） | A/B 语料可用性硬前置 | —（Rurix 侧加载面） | 🟡 glTF 导入通道已验收（G8 M81），真实大场景加载未验证 | 大场景加载计数/事件 golden；计数为零冒充/静默丢场景 RED | C/D | P0 | 核·环·产 | ✔ | G10.3 |
| M133 | 场景清单冻结与版本化：清单 digest 注册 + 只追加修订行 | M131/M132 | 双端语料一致性冻结点 | — | ⬜ | 清单 digest 注册面；变更只追加程序 | D | P1 | 环·产 | ✔ | G10.3 |

## 3. D3 度量基建

| 行 | 能力（子面） | G9 承接锚 | backfill / 触发条件字面 | UE5 参照 | 现状 | 缺口要点 | 档位 | 建议 P 级 | 验收五层级 | 4070Ti | 拟承接 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| M134 | 帧捕获 EXR/HDR 管线：Rurix HDR 帧捕获落盘（格式**待定（G10.1 治理波裁决）**）+ 捕获→回读逐像素往返无损 + 分辨率/色彩空间元数据 | G9 M118 显示管线 HDR 线性域面（只消费不重定） | A/B 帧面硬前置 | `Private\HdrCustomResolveShaders.cpp`（HDR 解析参照） | ⬜ 无 HDR 帧捕获落盘管线 | 捕获往返无损；位深截断（8bit clamp）/sRGB 混标注入 RED；元数据闭集 | B/C | P0 | 核·环·主 | ✔ | G10.4 |
| M135 | FLIP 度量：自实现与参考实现逐图对拍一致（容差 measured 标定）+ 恒等图对 FLIP=0 极值断言 + 参考实现版本 pin | —（G10 新立） | 画面审查法定工具；口径冻结 RFC（R-G10-3） | —（度量面；参考实现选型**待定（G10.1 治理波裁决）**） | ⬜ 仓库无图像质量度量工具 | 自实现 + 参考对拍 harness；口径参数漂移/参考输出扰动 RED | D | P0 | 核·环 | ✔ | G10.4 |
| M136 | SSIM/PSNR 度量：口径冻结进 spec + 参考对拍 + 恒等图对 SSIM=1/PSNR=inf 极值断言 | —（G10 新立） | 同 M135 | — | ⬜ | 同 M135 三臂 RED；口径进 spec | D | P0 | 核·环 | ✔ | G10.4 |
| M137 | 逐像素 diff 报告：diff 热区图 + 逐区域统计落盘（阈值 measured 标定）+ evidence schema 闭集 | M135/M136 | 差距定位与 G11 修复导航 | — | ⬜ | 热区图与标量报告一致性 RED；空场景行 RED | D | P0 | 环·产 | ✔ | G10.4 |
| M138 | 度量阈值标定程序：标定可复跑 + 标定值入 `g10_budget.json`（measured_local，禁手写，P-09） | M134~M137 | 14 §5/P-09：estimated 占位不得超 2 期 | — | ⬜ | 标定程序 + provenance；手写阈值注入 RED | D | P1 | 环·产 | ✔ | G10.4（baseline 于 G10.1） |

## 4. D4 A/B 对比与性能对标

| 行 | 能力（子面） | G9 承接锚 | backfill / 触发条件字面 | UE5 参照 | 现状 | 缺口要点 | 档位 | 建议 P 级 | 验收五层级 | 4070Ti | 拟承接 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| M139 | A/B 对比 harness：场景全集同场景同相机同光照双端出图 + 度量报告 + 差距清单落盘；单端缺帧不充绿不遮蔽 | M128~M137 全链 | G10 主交付面；M130 digest 核验前置 | `Private\GPUBenchmark.cpp`（bench 参照面） | ⬜ 无画面对比 harness | 双端出图编排；差距清单缺场景行 RED；单端帧缺失聚合 PASS 即 RED | D | P0 | 核·环·产 | ✔ | G10.5 |
| M140 | 差距清单登记：每差距项带 UE5 模块归属（枚举闭集）+ measured delta + 建议 P 级 + G11 承接锚 | M139 | G11 法定输入；非 measured 叙述充差距 RED | 归属枚举闭集（G10.1 冻结）：`Private\Nanite\` · `Private\Lumen\` · `Private\MegaLights\` · `Private\VirtualShadowMaps\` · `Private\PostProcess\TemporalSuperResolution.cpp` · `Private\PathTracing.cpp` · `Private\HairStrands\` · `Private\SubsurfaceTiles.cpp` · `Private\SingleLayerWaterRendering.cpp` · `Private\VolumetricCloudRendering.cpp` · `Private\SkyAtmosphereRendering.cpp` · `Private\DBufferTextures.cpp` · `Private\DistanceField*.cpp` · `Private\ReflectionEnvironment*.cpp` · `Private\TranslucentRendering.cpp` 等 | ⬜ | 差距项 schema；缺归属/缺承接锚行 RED；口径差项与画质差距项分列（R-G10-6） | D | P0 | 核·环·产 | ✔ | G10.5 |
| M141 | 性能对标基线：双端同场景帧率采样（14 §5 协议）+ 环境画像（驱动/锁频/WDDM-HAGS/TDR）+ 交替采样顺序登记 | —（G14 前置数据面） | 「帧率对标 UE5 略高不降级画质」= G14 目标；G10 只建基线不设通过线 | `Private\GPUBenchmark.cpp` | ⬜ 无双端帧率采样面（`rx bench` 协议底座在，14 §5） | 锁频硬前置（R-G10-5）；采样轮数不足冒充 RED；画像缺字段 RED | D | P0 | 环·产 | ✔ | G10.5 |

## 5. D5 后续期前置档案面（仅档案不接线）

| 行 | 能力（子面） | G9 承接锚 | backfill / 触发条件字面 | UE5 参照 | 现状 | 缺口要点 | 档位 | 建议 P 级 | 验收五层级 | 4070Ti | 拟承接 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| M142 | UpscaleBackend 抽象复核 + DLSS/Streamline 接入面档案：输入槽位（MV/深度/reactive mask）复核 + vendor 后端获取/许可路径调研；**仅档案，实施归 G13** | G8 M24/M25 时域/超分底座（[G9 矩阵](../g9/G9_CAPABILITY_MATRIX.md) §0.4）；G9_P2 M26-fg open-观察；G9_P2 RD040-nrd 行 | 调研报告7 §2.3/§2.4：reactive mask 为 vendor SDK 标准输入槽位；`ITemporalUpscaler` 接口思路 | `Private\PostProcess\TemporalSuperResolution.cpp`（TSR）· 第三方超分接口面 | 🟡 TSR 时域底座 + UpscaleBackend 抽象已存在（G9 closed 面）；无 DLSS 接入 | 接入面复核档案；DLSS/Streamline 许可与集成路径档案；NRD 重判条件字面不动（G9_P2 RD040-nrd） | B/C | P1 | 环（档案面） | ✔ | G10.6 伴随 |
| M143 | M96 参照器→生产化路径追踪差距档案：采样预算/降噪链/材质覆盖/性能预算四面差距；**仅档案，实施归 G12** | M96（`g9.p0.m96` 门绿，pbrt-v4 对齐） | G12 法定输入候选 | `Private\PathTracing.cpp` · `Private\PathTracingSpatialTemporalDenoising.cpp` | 🟡 参照器级在位（非生产化） | 生产化差距档案作为 M140 差距清单专项落盘 | C | P1 | 环（档案面） | ✔ | G10.5 差距清单内 |

---

## 6. 汇总

### 6.1 P0 地基清单（建议值，12 行；G10.1 决策表重裁后硬化）

| 波次 | P0 行 |
|---|---|
| G10.2 出图环境 | M128 出图环境可用 · M129 参考帧批量出图与 provenance 库 · M130 双端确定性（骨架） |
| G10.3 压测语料 | M131 许可登记 · M132 场景加载 |
| G10.4 度量基建 | M134 帧捕获 · M135 FLIP · M136 SSIM/PSNR · M137 逐像素 diff |
| G10.5 A/B 对比 | M139 A/B harness · M140 差距清单登记 · M141 性能对标基线（M130 双端核验同波收尾） |
| G10.8a soak | 全部 P0 硬门绿后 soak；**禁止**条件实现后跳过 soak 直接 close |

### 6.2 统计

- **行数**：16 行（M128~M143），覆盖 D1×3 / D2×3 / D3×5 / D4×3 / D5×2 全部子面。
- **优先级分布（建议值）**：P0 = 12 · P1 = 4（M133/M138/M142/M143）· P2 = 0 初始（G10 期新产生项归 G10.7 穷举）。
- **档位分布**：D 档工具/资产为主（12 行）· B/C 档 4 行（M134/M142/M143 等）· A 档 0 行——G10 无语言/编译器语义面。
- **4070 Ti 可验证性**：全部 16 行可真机验证——与 P-09 兼容。

### 6.3 承接锚映射总表（G9 defer / 底座锚 → G10 行）

| G9/G8 锚 | 决策字面（来源） | G10 承接 | 模块 |
|---|---|---|---|
| M61 mesh shader | defer-to-G10+「重评窗内多厂商扩展行为收敛 + 性能差 measured 证据齐备且真实消费方出现」（[G9_P2_DECISIONS](../g9/G9_P2_DECISIONS.md) §3） | G10.6 重判（G10.5 差距清单 measured 数据为证据输入之一） | D1→重评窗 |
| M52 SER | defer-to-G10+「高分歧 RT workload 真实集成需求 + capability rt.ser 设备面实测可用」（同上） | G10.6 重判 | D1→重评窗 |
| M99-clipmap 世界辐射缓存 | defer-to-G10+「远场画质 measured 举证落地」（同上） | G10.6 重判——A/B 差距清单远场项即候选 measured 举证 | D1→重评窗 |
| M100-high ReSTIR 高档 | defer-to-G10+「多灯 workload measured 证据齐备」（同上） | G10.6 重判——多灯场景 A/B 差距项即候选举证 | D1→重评窗 |
| SAFE-GPU | defer-to-G10+「G10+ Safe GPU Operator Platform 独立期立项」（同上 §1） | G10.1 治理裁决表项 7（G10 承接 or 续 defer） | 治理 |
| M127 神经变形 | defer-to-G10+「离线工具链 corpus 语料 + PhysicsAsset residual 消费方出现」（同上） | G10.6 重判 | D5→重评窗 |
| M98-l4 Far Field | defer-to-G10+「HLOD 运行时接口面就绪 + L4 计数可测」（同上） | G10.6 重判（M111 HLOD 已验收，接口面就绪度核验） | D2→重评窗 |
| M114-strand 毛发精确 OIT | defer-to-G10+「M120 精确档 benchmark 裁决数据落地 + 档选定程序解冻」（同上） | G10.6 重判 | D4→重评窗 |
| M118-hdr-cal HDR 标定 | defer-to-G10+「HDR 显示设备资产/产品需求出现」（同上） | G10.6 重判 | D4→重评窗 |
| M125-adopt3 Jolt5.6 三件 | defer-to-G10+「后续 Jolt 升级评估窗采纳臂成立」（同上） | G10.6 重判 | D5→重评窗 |
| G8 M81 glTF 导入 | 资产闭环底座（[G9 矩阵](../g9/G9_CAPABILITY_MATRIX.md) §0.4） | M132 只消费不重定 | D2 |
| G8 M24/M25 时域/超分 | 时域底座 + UpscaleBackend 抽象（同上） | M142 复核档案（实施 G13） | D5 |
| G9 M96 参照器 | `g9.p0.m96` 门绿（pbrt-v4 对齐） | M143 差距档案（实施 G12） | D5 |
| G9 M118 显示管线 | `g9.p0.m118` 门绿（四插件） | M134 帧捕获 HDR 域面只消费不重定 | D3 |

### 6.4 G10 成功判据草案（十条；G10.1 进 G10_CONTRACT 硬化为验收门；本矩阵只写判据草案，不 materialize CI 步骤）

1. **出图环境**：spike 裁决路径落地；固定场景 UE 5.8 侧出帧成功；环境画像随证据存档；假帧冒充即 RED。
2. **参考帧确定性**：场景全集参考帧批出 + 同参数双跑帧 digest 一致 + provenance 登记闭集零缺行。
3. **双端确定性**：相机/光照/时间参数双端 digest 相等；漂移/schema 外字段注入即 RED；digest 不等不得出 A/B 报告。
4. **压测语料**：场景清单落盘 + 逐资产许可登记零缺行（白名单/SPDX/URL/attribution/digest）+ 全场景 Rurix 加载门绿。
5. **帧捕获**：HDR 捕获→回读逐像素往返无损；位深截断/sRGB 混标注入即 RED。
6. **度量一致性**：FLIP/SSIM/PSNR 与参考实现逐图对拍一致（容差 measured 标定）；恒等图对极值断言；口径冻结进 spec。
7. **A/B 与差距清单**：场景全集双端出图 + 度量报告 + 差距清单零空行；每项带 UE5 模块归属 + measured delta + G11 承接锚；单端缺帧不充绿。
8. **性能基线**：双端帧率采样 14 §5 协议 + 环境画像齐备；G10 不设帧率通过线。
9. **defer 重评窗**：G9 十项逐行重判零空行；维持 defer 必带承接锚；deferred history 只追加。
10. **measured 纪律**：`g10_budget.json` 非空 measured_local 零 estimated（P-09）；全部阈值实测标定；G10 全域零修复 PR。

---

## 7. 门控维持与重审条件登记（只读引用，不改写既有注册表）

| 项 | 维持裁决 | 重审条件（字面来源） |
|---|---|---|
| DMM / displacement micromap | **永久禁止**（G9 矩阵 §7 只读引用） | 任何 micromap 字样提案进 RFC 即一票否决 |
| G10 不设画质通过阈值 / 帧率通过线 | 差距全量 measured 登记即绿；修复归 G11 | G11 立项（[G10_PLAN](G10_PLAN.md) §5 表项 5 裁决后硬化） |
| UE 源码零 vendoring | 只读外部参照；不进仓库、不复制片段（R-G10-10） | 永不重审（许可边界线）；监控形式**待定（G10.1 治理波裁决）** |
| 修复 PR | G10 全域 out-of-scope（R-G10-8） | G11 立项只消费 G10.8b 锁定差距清单 + 承接锚 |
| FG/MFG（M26-fg） | 不进 G10；open-观察维持（[G9_P2_DECISIONS](../g9/G9_P2_DECISIONS.md) §1） | 独立层立项（ABI + latency/pacing + vendor 能力 measured 证据齐备） |
| NRD vendor 降噪接入（RD040-nrd） | no-go 维持（同上） | adapter ABI 契约测试 + 画质/稳定性对照需求出现时按只追加程序重判；temporal 底座 0-byte |
| NRC / 神经 radiance cache | 观察项维持（G9 矩阵 §7） | SG-002 Tensor Core 族禁止面解除 |
| GPU 主刚体 | 否决线维持（G9 矩阵 §7） | 矩阵 §12 五条件同时成立 + RD-043 触发 + 独立 Full RFC |
| 调研引用时效 | 七份报告为 2026-07-28 快照 | G10.1 复核关键引用时效（沿 G9 R-G9-3 模式） |

---

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-15 | 初版（G10.1 治理波起草）：M## 顺延 G9（M128~M143，16 行）覆盖 D1 UE5 出图 harness / D2 压测语料 / D3 度量基建 / D4 A/B 对比与性能对标 / D5 后续期前置档案面；每行带 G9 承接锚 / 触发字面 / UE5 参照（Renderer 源码模块路径，经 2026-08-15 本地源码树实测在树）/ 现状 / 缺口 / 建议 P 级 / 验收五层级 / 波次归属；§0 立项前事实基线六条（G9 closed 34 key、十项 defer、底座、仓库缺口、本地 UE5 环境、G10~G15 分期）；§6 P0 建议 12 行 + 承接锚映射总表 + 成功判据草案十条；§7 门控维持登记。零编号占用、零 registry 改动、零 spec/src/conformance 改动。 |
