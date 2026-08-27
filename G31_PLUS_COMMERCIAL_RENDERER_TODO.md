# G31+ 待办总表 — GPU 真实渲染实时游戏画面 × 渲染器商业化发布

> 版本：v1.1.3（2026-08-26）
> 性质：**待办汇总镜像**（非契约、非规划文档）。事实源恒为 `milestones/*/\*_CONTRACT.md`、各期 P2 表与 `registry/`；冲突时以事实源为准。
> 法定输入面：任何 G31+ 立项必须以 `milestones/g30/g30_campaign_handover_registry.json` 为唯一法定输入面（RFC-0047 §5.5），各期 P2 表原始锚 0-byte 不回写，deferred history 只追加。
> 汇总口径：本表 = ①G 里程碑已标明的待办（承接锚/defer/open/maintain/诚实红）+ ②里程碑体系**未覆盖的遗漏项**（标 `[遗漏]`）。目标判据 = 做完后 Rurix 可作为游戏引擎渲染器商业化发布。
> v1.1 注：G31 波 A 已验收 §1.1–1.2 #1–#5 的实时呈现最小面（`milestones/g31/G31_PLAN.md` §2）。§1 行字面作历史镜像不回写；本版从 #57 只追加。§7 调研与现表 #6/#20–26/#24/#25/#33–36/#40 **去重**，不重开已标明行。

---

## 0. 现状定盘（截至 g30-closed，2026-08-25）

**已成立面**（不在本表内，列出仅为对照）：

- 生产管线（`src/rurix-render/src/bin/g14_3_pipeline_perf.rs`，Mega 四 pass：primary → shadow_scatter → direct GI → shade_reduce + TSR/DLSS/FSR 三超分臂）**全 GPU 链内零 host 往返**，bistro-interior 1080p 直接光 t100 档 tsr 2.29ms / fsr 2.79ms / dlss 4.01ms。
- 帧率对标 **17/18 格 ≥ UE5**（G14 M-d 口径，G30 终判维持）；商用画质 **18/18 达标**（G16plus M-g，G15 历史 0/18 未改写）；渲染确定性 Stage A digest 锚 **18/18 零漂移**。
- G7 One True Device Frame 15-pass 帧链（`apps/uc06-renderer/src/device_frame.rs`，960×540→1080p TSR + Jolt 物理桥）。
- 26 个 `.rx` device kernel 在树；win32/Android swapchain present 底座已 discharge（RD-032 history，`vk::run_graphics_present`）。

**未成立面（= 本表的存在理由）**：

1. **生产管线是"没接屏幕的引擎"**——bench 形态（固定契约相机 + 每帧 fence 全同步 + readback + 末帧 digest），无实时窗口呈现、无交互；真窗口 demo 只有 ruridrop（720p ~67fps，老 CUDA–D3D12 interop 路）与 blackhole。
2. **G26–G29 四个 device kernel（帧生成/HZB/ReSTIR/slab 材质）已实现并验证，但全部"在树零接线"**（G30 M-b 两层机器核验在案），生产管线拿不到它们的收益。
3. 性能面 **1 格诚实红**（G17-MD-F1：bistro/t100/dlss_sr，G30 新鲜 ratio=0.960479 < 1.00）。
4. 动态场景/纹理内容管线/动画接线缺失——现生产场景为静态几何 + 逐三角 albedo/emission，无纹理采样管线，动态源仅 Jolt 物理桥。
5. RD 八条 open + 尾锚六件 maintain + 16 行长线分项差距（P4 四行 / SVT 四行 / KTX2 三行 / RD-040 五分项）。

---

## 1. P0 — "GPU 真实渲染出实时游戏画面"主线

> 本节做完的验收方向：**bistro 级场景在真窗口内以生产管线实时交互渲染**（相机可动、逐帧呈现、帧率 measured），画质/确定性门零降级。

### 1.1 生产管线接实时窗口 `[遗漏——从未立项]`

| # | 任务 | 现状与证据 | 交付判据方向 |
|---|---|---|---|
| 1 | **生产管线 swapchain present 接线**：把 `g14_3` Mega 车道（含 TSR/DLSS/FSR 臂）输出接 `vk::run_graphics_present` 同族 win32 swapchain 真窗口 | `display/swapchain.rs` 仅路径状态机（窗口腿 D-130 红线 C++ shim 0-byte）；present 底座只到三角形级 `vk_present.rs`；G14 契约把"present 路径口径"登记为未消费结构面 | bistro 1080p 真窗口逐帧 present，present 口径帧率独立登记（禁混入真实渲染帧率口径） |
| 2 | **帧流水化（submit/collect 分离）**：去掉每帧 fence 全同步，N 帧 in-flight | `render_exec.rs::execute_with_frame_update` 每帧 submit 即等待，`frame_slots≥2` 仅槽复用；submit/collect 分离 API 在 `g14_3_pipeline_perf.rs` 头注释登记为"并发会话大改面，本波不动"（未消费①） | 双缓冲/三缓冲 in-flight，帧时 A/B measured；确定性协议（固定 seed digest 锚）不破坏 |
| 3 | **游戏循环最小面**：输入 → 相机 → 逐帧参数更新（jitter/曝光/灯光）进生产车道 | 契约相机为冻结常量；ruridrop/blackhole 有各自泵但不通生产管线 | 可交互相机 + 逐帧 uniform 更新路径；退出/resize/alt-tab 不崩 |
| 4 | **动态场景更新通路**：逐帧实例变换/增删 → GPU scene/AS 增量更新 | 生产车道 AS 常驻 +"禁逐帧全量重传场景"；动态源仅 G7 帧链 Jolt 物理桥 | 动态实例（≥1 类运动物体）进生产帧，AS refit/rebuild 策略 measured |

### 1.2 四个 device kernel 生产接线（G30 承接锚已标明——`campaign_period_rows`）

> 统一纪律：接线经 G31+ 立项程序，以 `g30_campaign_handover_registry.json` 为唯一输入面；host 参考臂即金标准，接线后双臂对拍门维持。

| # | 任务 | kernel（已实现件） | 被冻结的生产面（现 0-byte 机核对象） | 承接锚（字面） |
|---|---|---|---|---|
| 5 | **帧生成 FG/MFG 接线**（×2/×3/×4 presented 帧率） | `kernels/g26_framegen.rx`（对拍 p100=3.576e-7 + SSIM 严格胜 frame-hold） | `temporal/framegen.rs` host 臂 + 默认渲染臂；生成帧禁入真实渲染帧率口径 | 生产接线窗（G13-N7 行） |
| 6 | **HZB 遮挡剔除接线 + 两阶段闭环第二段**（上一帧重投影初剔 + 误遮挡重测） | `kernels/g27_hzb_reduce.rx` + `g27_hzb_test.rx`（mips 9 级位级全等 + 800 rect 零假阳性） | `geometry/hzb.rs`、`geometry/cull.rs`、`geometry/visbuffer.rs` | 生产接线窗；两阶段第二段 = RFC-0044 评审 §5 增列项；P4-2 依赖解除仅事实登记 ≠ 兑现 |
| 7 | **ReSTIR 高档 reservoir 车道集成**（多灯 GI/DI 高档） | `kernels/g28_restir.rx`（y 整数锚 20000/20000 + 无偏 3σ + 空间重用加性臂） | `gi/restir_reservoir.rs`、`gi/multi_light.rs`；低档 MegaLights 仍默认档 | M100 车道集成窗（锚三件之第三件；RFC-0038 out-of-scope 锚） |
| 8 | **slab 材质 closure/侧表转正**（Substrate 类双层材质进生产资产面） | `kernels/g29_slab.rx` + 侧表 16 槽（对拍 p100=1.192e-7 恰一 ULP） | `material/` 整目录 + `graph/types.rs` MaterialClosure 32B；生产侧表零挂接、bin-local 不落资产 | 生产接线窗（RD-041-slab 行） |

### 1.3 游戏画面内容管线 `[部分标明 + 部分遗漏]`

| # | 任务 | 现状 | 来源/锚 |
|---|---|---|---|
| 9 | **纹理采样管线进生产场景** `[遗漏]`：生产内容模型从"逐三角 albedo/emission"升级为贴图采样（albedo/normal/rough-metal），与 slab 材质接线（#8）联动 | G11.3 DDS 转码链在树且确定性锚在案（`milestones/g11/g11_3_dds_transcode_manifest.json`），但生产 bench 车道未走纹理采样 | 采样语言面已稳定（RFC-0007 系）；DDS 链可直接消费 |
| 10 | **蒙皮/动画接入生产帧** `[遗漏]`：M92 device 蒙皮件 + 骨骼动画驱动动态角色进画面 | `geometry/skinning.rs`、`skin_kernel.rs`、`g9_m92_skinning_device.rs` 双臂验证件在树，未进生产管线；蒙皮/WPO MV 通道接口已按三类速度设计（RD-041 backfill 字面） | RD-041 分项"蒙皮 WPO MV 在动态资产面出现时"——做游戏画面即触发动态资产面 |
| 11 | **BistroExterior 场景转换臂**（第三对标场景，植被/大世界面） | fbx2gltf/assimp/blender 三工具 PATH 全缺 + 源资产三根检索 0 命中；维持 BistroInterior+CornellBox 双场景闭集 | G10-N6（tail_six 行）：锚 = FBX2glTF 上游修复在树或替代臂 + 源资产同窗齐备 |
| 12 | **多反弹 GI 默认档评估**：现生产默认 `--gi off`（直接光位级锚），`g16_gi_multibounce.rx` 为 `--gi on` 加性车道 | G16/G18 多反弹与 light transport depth 链在树 | 游戏画面质感需要 GI 默认开启的 measured 画质/帧时权衡窗 |
| 13 | OIT/半透明与毛发进生产帧 `[遗漏]` | `oit/` 模块仅测量 harness（G9 M120）；毛发 card/mesh 档（`display/hair.rs`）；strand 档 maintain（M114-strand，锚 = 毛发资产入压测闭集） | 游戏画面含玻璃/粒子特效/头发时触发 |

---

## 2. P1 — 商用终审残余与稳定性（已标明）

| # | 任务 | 现状与终态 | 重评/兑现锚（字面） | 来源 |
|---|---|---|---|---|
| 14 | **G17-MD-F1 性能焦点格收口**（bistro-interior/t100/dlss_sr） | 17/18 诚实红终判定盘：G30 新鲜真跑 frame_ms=3.5767ms vs UE 暖态 3.4353ms → ratio=0.960479 < 1.00 物理不可达（NGX 宿主开销不可分离）；五期轨迹 0.856→0.960 | NGX 分解 profiling **或** UE 侧插桩（宿主差可分离 measured 证据，RFC-0032 同源）；或焦点格 ratio ≥1.00 新证出现时只追加重判 | `g30_campaign_handover_registry.json` G30 期行 |
| 15 | **RD-045 间歇 digest 漂移 backfill 三件**：①漂移确证定位 ②修复确证 ③主题 Full RFC 评估 | open/maintain-open：盘点 0/3；累计观察 G19.3 窗 12/12 + G19~G29 十一期 soak 零漂移（观察零漂移不充①件——F5 硬线） | backfill 三件全齐方可 close | rd_eight 行 |
| 16 | **RD-027 PT 毒径挂起修复**：pt_render 特定样本序号/弹射深度组合不终止（疑 PTX 发散重汇聚/工具链缺陷） | open | 生产档路径追踪要商用必须定位（毒径 = 潜在挂死面） | `registry/deferred.json` |
| 17 | **HDR 输出管线**（现 maintain-SDR） | vulkaninfo 三 token（HDR10_ST2084/BT2020_LINEAR/HDR10_HLG）全 absent（本机显示链不支持） | 显示链变化 + HDR 资产需求两半同窗成立；商业化建议主动备 HDR 显示设备开窗 | M118-hdr-cal（tail_six） |
| 18 | **AMD 真卡 present 验收**（G-MB1-6，MB1 唯一存续尾门）+ 其余平台 surface 余量 | RD-032 win32/Android 已 discharge；AMD 缺硬件 open | 获得 AMD 硬件后按 DoD 补证据；商业化需多厂商兼容矩阵（另见 #50） | RD-032 history / MB1 契约 |
| 19 | EA1 冷启动 A 段 measured（干净 Win11 VM 分发验证） | RD-033 open：缺 VM 环境待补测 | owner 备齐 VM 环境 | `registry/deferred.json` |

---

## 3. P2 — 渲染特性长线（RD-039/040/041 的 16 行分项 + 关联重判窗，已标明）

### 3.1 虚拟化几何（RD-039，open）

> v1.1：生产未消费簇 DAG、HLOD 抽面质量、VisBuffer 出帧见 §7 #58/#66–#68/#74，不在本表重复立项。

| # | 任务 | 差距行字面 | 锚 |
|---|---|---|---|
| 20 | cluster 流送 P4-1：cluster 页磁盘布局与驻留池（RXPL v2 页 ABI 的 cluster 载荷扩展） | open | 后续期 device 波（RD-039 长线） |
| 21 | cluster 流送 P4-2：GPU 请求反馈链（剔除 pass 产 cluster 缺页请求 → host 驻留调度） | open | 依赖 HZB device 化——**G27 已解除依赖**（事实登记在案），可开工 |
| 22 | cluster 流送 P4-3：LOD cut 与驻留联动（cut 选择受驻留集约束的回退语义） | open | 后续期；现全驻留假设维持 |
| 23 | cluster 流送 P4-4：异步 IO 优先级链（近处/大屏占比 cluster 优先） | open | 后续期；页式流送优先级面复用评估 |
| 24 | **mesh shader HW 光栅第三路径**（M61） | maintain-no-go（三项 1/3：HZB device 已命中；P4 未清零；HW 性能差 measured 证据零命中） | 三项闭集全齐方启动（RFC-0034 重判表只追加）——P4 清零（#20~23）是前置 |
| 25 | **HLOD L4 Far Field 档**（M98-l4） | maintain L1/L2/L3 三级链（两半 0/2：proxy 追踪 device 腿零实现 + L4 计数器未接，三处 fail-closed 入口在位） | HLOD proxy 追踪 device 腿 **+** L4 计数器接入（合取，两半全齐方改判） |
| 26 | Foliage/骨骼虚拟几何、曲面细分位移、Assemblies 全功能、Mega Geometry 簇级 BLAS | RD-039 backfill 长线字面 | 逐项独立判档（动态资产面/RT 合流需求出现时） |

### 3.2 光照（RD-040 五分项全 defer + SER）

| # | 任务 | disposition 依据 | 锚 |
|---|---|---|---|
| 27 | SMRT 阴影贴图射线追踪（软阴影完整版） | VSM clipmap 为现生产档；收益依赖多灯动态场景资产 | 多灯动态场景资产入压测清单 + shadow page 采样车道出现 |
| 28 | 世界辐射缓存演进（M99-clipmap 后续：持久化/跨帧失效精化） | G11 已兑现 clipmap 承接，演进无新需求证据 | 大世界流送 + GI 联动窗 |
| 29 | NRD vendor 降噪集成 | G18 已落自研降噪加性 profile；无 measured 对照证据 | 自研降噪画质差距 measured 检出 |
| 30 | OMM（Opacity Micro-Map） | 现压测闭集零 alpha-tested 主导面 | alpha-tested 植被场景入压测（与 #11 BistroExterior 联动） |
| 31 | **RT pipeline + SBT 宿主车道**（hit/miss 着色阶段进 kernel 子语言，Full RFC 级） | 生产车道 = RayQuery compute 单 kernel（RXS-0357 谱系） | hit/miss 语义需求成立（多材质 RT 分派）+ SER 收益 measured 预估窗 |
| 32 | SER workload 兑现（M52，capability 实测 available） | maintain-defer（单半命中：workload 零实现） | RT pipeline/SBT 宿主车道出现（#31 是前置） |

### 3.3 材质/流送/时域（RD-041）

| # | 任务 | 差距行字面 | 锚 |
|---|---|---|---|
| 33 | SVT-1：虚拟纹理页表（128K² 虚拟地址空间 → 物理瓦片间接寻址） | open | SVT 立项窗 |
| 34 | SVT-2：GPU 反馈 pass（采样 miss 记录 → host 请求队列） | open | 与 P4-2（#21）同族缺口 |
| 35 | SVT-3：瓦片边界过滤（border texel 复制/各向异性跨瓦片） | open | SVT-1 落地后 |
| 36 | SVT-4：地形/贴花消费方接线（`world/terrain.rs` 现零 SVT 依赖断言维持） | open | M116 地形 SVT 需求成立窗 |
| 37 | KTX2-1：容器解析（supercompression 元数据 + mip 布局） | open | KTX2 立项窗（vendor 源码 `src/rurix-basis-sys/vendor/` 已在树） |
| 38 | KTX2-2：BasisU 转码器集成（C++ vendor 桥 → BC7/ASTC） | open | vendor C++ 桥判档（fail-closed DEV_ENV 纪律） |
| 39 | KTX2-3：通用转码收益 A/B measured（分发体积 vs 转码耗时） | open | 资产分发面需求成立（商业化分发即触发——与 #52 联动） |
| 40 | Work Graphs GPU 侧调度 | not-available 实测维持（驱动扩展 absent）+ DGC available 互核 | WG present 翻转时复评启动 |

---

## 4. P3 — 上游阻塞、物理与观察项（已标明；商业化间接相关）

| # | 任务 | 状态 | 锚 | 
|---|---|---|---|
| 41 | RD-034：DXIL ray-tracing 腿（spirv-cross 拒 raygen） | open/maintain-blocked（探针恒跑） | spirv-cross SPV_KHR_ray_tracing 消费路径 或 LLVM A 路解锁（探针 exit 语义反转时复评） |
| 42 | RD-011/012/014/015：DXIL 后端系列（LLVM PSV patch / mesh-task-RT 阶段降级 / B 路供应链跟踪 / B→A 迁回） | open | 上游 LLVM #90504/#57928 演进跟踪 |
| 43 | RD-026：std::gpu 首期外宿主编排面（AsyncBuffer/Event/多 stream 重叠等） | open | 单源 `.rx` 游戏应用需求成立时判档 |
| 44 | RD-030：launch marshalling ABI 字节回归守护 | open | 持续回归面 |
| 45 | 物理观察轨：RD-042 可微仿真 / RD-043 wgrapier GPU 刚体 / RD-044 三分项（Jolt 软体布料/Taichi MPM/Rapier 深造）+ M125 Jolt 5.6 采纳窗 + M127 神经变形 | 全 open/maintain（G30.2 尾锚窗零命中） | 各自 reeval_anchor 字面（`g23_rd044_subitem_registry` 等） |
| 46 | SAFE-GPU 平台立项评估 | defer-to-G31+（G30 改锚） | 独立期资源窗 + 平台需求方（外部采纳生态）两半同窗 |
| 47 | legacy 十一条历史清册跟踪 | 零 close（逐条 backfill 核验在案） | `milestones/g24/g24_legacy_rd_registry.json` 引用不复制 |

---

## 5. P0′ — 商业化发布工程面 `[遗漏——里程碑体系未覆盖]`

> "商业化发布"= 外部游戏引擎/项目能安全采纳 Rurix 渲染器。以下无一在 G 里程碑立项，属体系性遗漏。

| # | 任务 | 现有基础 | 交付判据方向 |
|---|---|---|---|
| 48 | **渲染器 SDK 稳定 API 面**：embedding API（初始化/帧循环/场景提交/资源句柄）语义化版本 + stable 快照守卫扩展到渲染器面 | UC-05 最小 RHI + C ABI 导出（EI1）；RD-008 stable 快照机制（语言面已 closed）；RD-036 C ABI v2 超界需求登记 open | SDK 头文件/文档冻结 + 破坏性变更走 RFC；≥1 个外部 C++/D3D12 或 Vulkan 引擎宿主集成 demo 真跑 |
| 49 | **渲染器文档与示例**：集成指南、pass/特性矩阵、性能调优指南、最小示例工程 | 语言侧 `rx doc` 文档站已有；渲染器面文档零 | 新用户按文档 <1 天完成最小集成 |
| 50 | **设备兼容矩阵与能力降级链**：NVIDIA（Ada 实测）之外 AMD/Intel 桌面 GPU 的 capability 探测 → 降级链（DLSS→FSR、RayQuery→屏幕空间、HW AS→SW 回退）系统化 | 现全部 measured 证据单卡（RTX 4070 Ti）；FSR/TSR 臂已有；G-MB1-6 AMD 尾门 open | ≥2 厂商真卡全链绿 + 降级链 fail-closed 单测 |
| 51 | **运行时健壮性**：device lost 恢复、窗口 resize/全屏切换、驱动 TDR 处理、显存超额（budget 违约）降级 | poisoned 状态机（语言运行时）已有；渲染器面零 | 故障注入测试集 + soak 含故障臂 |
| 52 | **渲染器 SDK 分发打包**：预编译 bundle 进 rurixup/MSI/winget 链 + 签名/SBOM 扩展 | EA1 分发链（`v1.0.1-dist` 系列）已有语言工具链面 | SDK bundle 一键安装 + 示例工程离线可建 |
| 53 | **vendor 许可合规终审**：DLSS/Streamline、FSR、Jolt、BasisU、Taichi AOT 等再分发许可矩阵复核（商用分发口径） | `milestones/g13/design/vendor_upscale_license_clearance.md` 已有超分面 | 全 vendor 面商用再分发许可矩阵 + SBOM 对账 |
| 54 | **性能剖析与调试工具面**：GPU 时间戳/pass 级 profiler 对外暴露、Nsight 标注、帧捕获兼容（RenderDoc） | 各 bench 已有内部 GPU 时间戳 evidence | 外部用户可自助定位帧内热点 |
| 55 | **支持渠道与版本政策**：issue 流程、LTS/release 节奏、安全响应（SECURITY.md 已有语言面） | 治理骨架（10_GOVERNANCE）成熟 | 渲染器面 support policy 文档化 |
| 56 | **外部采纳判据兑现**（使命判据）：≥1 个非作者维护的真实项目采用渲染器 | 05 年愿景 carve-out 维持未宣称 | 外部生产项目选择 Rurix 渲染器（商业化的最终验收） |

---

## 6. 建议的执行顺序（三条波次线）

1. **G31（实时呈现期）**：#1–#4（接窗口/帧流水/游戏循环/动态场景）+ #5 帧生成接线（presented 流畅度收益最大）→ 里程碑验收 = bistro 真窗口交互 60fps@1080p（真实渲染帧率口径 + presented 口径双登记）。
2. **G32（画面完整期）**：#6–#10、#12（HZB/ReSTIR/slab 接线 + 纹理管线 + 蒙皮动画 + GI 默认档）+ #11 BistroExterior → 验收 = 含动态角色/贴图材质/GI 的"游戏画面"demo；同窗攻 #14（DLSS 焦点格两半锚）与 #15（RD-045 三件）。
3. **G33+（商业化期）**：第 5 节全量（SDK/文档/兼容矩阵/健壮性/分发/许可）+ 第 3 节按锚触发逐个开窗（SVT/KTX2 随资产分发需求、P4 随大世界需求、RT pipeline 随多材质 RT 需求）→ 验收 = 外部引擎集成 demo + 商用发布件（#56 为最终判据）。
4. **G32/G33 穿插（v1.1 增补）**：#57/#59–#60（异步车道 device 执行，触 RXS-0239 须 RFC 修订行）与 #58/#66–#67（生产消费簇 DAG + 简化质量 + HLOD 质量烘焙）在画面完整期评估窗并行开；#74–#76（VisBuffer/软光栅/MDI 生产接线）接在 #6 HZB 接线之后。重叠量/流送收益一律 measured 写 evidence，不进硬门冒充。

---

## 7. v1.1 增补 — 异步车道、大网格/远处降复杂度、业界优化调研

> 性质：只追加镜像。会话两条（#57/#58）是本版立项理由；其余为 2026-08-26 并行调研补遗。
> 调研对照：UE5 Nanite 页流送 + World Partition HLOD、Khronos Async Compute / Timeline Semaphore 教程、Capcom RE Engine meshlet 管线（REAC 2025）、Remedy Alan Wake 2 mesh shader GPU-driven、meshoptimizer `clusterlod` / QEM、SIGGRAPH Advances 2024 VisBuffer+VRS、UE5 出货性能清单（PSO hitch / 后处理预算 / Reflex）。
> **明确不重开**：#6 HZB 两阶段、#20–23 cluster P4 四行、#24 mesh shader 第三路径、#25 HLOD L4 Far Field、#33–36 SVT、#40 Work Graphs。

### 7.1 会话两条（必须单列）

| # | 任务 | 现状与证据 | 业界对照 | 交付判据方向 | 来源 |
|---|---|---|---|---|---|
| 57 | **异步 compute 第二腿 device 执行**（M59 镜像补登） | 图编译第 4 趟 `plan_lanes` 已算 `FencePair`（`graph/compile.rs`），`CompileOptions::enable_async` 默认开；`CompiledGraph::execute()` 只按线性序回放 host 记录桩，**不提交第二条 GPU 队列**。语言/RHI 面 RXS-0239 单 queue 全序，`PassSpec` 无 queue 字段。`vk.rs`/`tirt.rs` 有独立 `compute_queue`/`graphics_queue` 句柄——compute 队列现用于 TIRT **copy + QueueWaitIdle**，不服务渲染图。G8/G9 M59 = **no-go 留档**（缺多队列 measured 收益）。本镜像 v1.0 **漏登**此行 | UE RDG / Frostbite 异步车道；Khronos：专用 compute-only family + timeline + wait-before-signal；候选 = 深度预通过后的 light cull 与主不透明重叠、本帧后处理与下帧阴影/几何重叠 | 执行器消费 `CompiledGraph.fences` 双队列提交；无专用队列 / 无 timeline → 显式单队列回落（已有 `enable_async=false`）；开/关异步 GPU 时间戳重叠量 measured 写 evidence **不进硬门**；**触 RXS-0239 必须走 RFC 修订行**，禁静默扩承诺面 | 已标明：`G9_P2_DECISIONS.md` M59；`RFC-0019` §4.8；重判锚 = D3-Q7 多队列 measured 收益 |
| 58 | **生产管线消费簇 DAG + 屏幕误差 LOD 出帧** `[遗漏]`（大网格导入的兑现面） | cook 能 `build_dag`→`pack_cluster_dag`→`geom_pages.bin`（`write_dag` 的 RXGB **被丢掉**）。host cut / G9 探针 / UC06 程序化网格吃 DAG。**g14_3 与 g31 共享 `g14_3_lane_body.rs`：glTF 三角汤烘焙进顶点 + 全场景（或按 mesh 节点拆的）TLAS**；`SceneData` 无 DAG/簇/页；对 `ClusterDag\|VisibleClusterSet\|read_dag` **0 命中**。`g14_3_primary.rx` 吃 `committed_primitive_index`。`--hzb on` 剔除粒度 = **~1186 个 mesh 节点 BLAS**，不是簇。远处三角数 = 导入三角数 | UE5 Nanite / `vk_lod_clusters`：误差 cut → 可见簇集 → 光栅或 CLAS | 把 `geom_pages`/RXGB 装进 g14/g31 会话；每帧 `VisibleClusterSet`（或 device cut）替换全量三角汤；全量臂留 digest 对拍，禁静默双世界；流送仍归 #20–23 | `[遗漏]`；#20–23 的上游，不替代 P4 |

### 7.2 GPU 重叠与多队列（#57 族）

| # | 任务 | 现状 | 业界做法与价值 | 优先级 | 去重 |
|---|---|---|---|---|---|
| 59 | **Fence 对 → timeline 提交器** `[遗漏]` | `FencePair { signal_after, wait_before, value }` 只进 dump/单测；`rurix-rt` 零 `timeline_semaphore` 命中 | 一条 64-bit timeline 表达跨队列依赖，替代 binary semaphore 网；CPU 可 `GetSemaphoreCounterValue` 做资源回收 | P1（#57 的执行脊柱） | #57 判档 go 后开工 |
| 60 | **异步候选 pass 白名单接线** `[遗漏]` | 报告5 三条件只在类型注释。`apps/uc06-renderer` / `uc08-physics` 的 `graph_setup.rs` 已把 `gi_probe_trace`/`rtao`/`hard_shadow` 标 `AsyncCompute`（`ao_filter` 回图形），**生产 Mega 不消费 `CompiledGraph`，标了也不进第二队列**。降噪在 G12 加性车道，不在默认 Mega 链 | Khronos：Forward+ light cull 等深度预通过即可与主不透明重叠；首批候选 = 已声明的 AO/GI/硬阴影 + 降噪空间趟/bloom mip；禁主光栅/阴影深度/indirect 准备 | P1 | 勿并入 #12（#12=GI 默认档画质；本行=调度重叠） |
| 61 | **跨帧 async post** `[遗漏]` | 后处理五级链 host 骨架在（`display/post_chain.rs`），与下帧几何无重叠提交 | 本帧 bloom/tonemap 走 compute 队列，与下帧阴影/主几何重叠，填 GPU bubble | P2 | 依赖 #57+#79 |
| 62 | **compute-only queue family 探测 + 单队列等价** `[遗漏]` | `vk.rs` 取 compute/graphics 句柄；未区分「仅 compute、非 graphics」family，也无 digest 等价门 | 真异步要专用 compute family；共享 graphics family 常假重叠。RFC-0019 要求 single-queue plan 与 multi-queue 同 digest | P1 | RFC-0019 §4.8.3 已写语义，执行零 |
| 63 | **跨队列 release/acquire + 禁 split 承诺面扩写** | EB 三轴 host 推导在；split barrier / 跨 family ownership **不在 RXS-0239 承诺面** | 跨 queue 必须成对 release-acquire + timeline wait；半对即 UB。Khronos 强调过宽 barrier 会把异步打回串行 | P2 | 须 RFC 修订行；与 #57 同窗 |
| 64 | **多 timeline 分轨（geometry / compute / transfer）** `[遗漏]` | 单 FencePair 值域 | 全局一条 timeline 易误造全序瓶颈；Khronos 建议每「引擎」一条，只在真依赖处交叉 wait | P2 | #59 之后 |
| 65 | **并行命令录制 / 多线程 encode** `[遗漏]` | host 图编译单线程；device 录制随 `render_exec` 单 CB | Frostbite/UE 按 pass 或 view 并行录制，吃满 CPU 提交；与 GPU 异步正交（一边填 CPU bubble，一边填 GPU bubble） | P2 | 不替代 #2 帧流水（波 A 已验收最小面） |

### 7.3 大网格导入与远处降复杂度（#58 族）

| # | 任务 | 现状 | 业界做法与价值 | 优先级 | 去重 |
|---|---|---|---|---|---|
| 66 | **geom-build 简化质量升级（QEM / meshoptimizer clusterlod）** `[遗漏]` | `simplify_group` 最短边贪心、端点不移动；`Cargo.toml` 无 meshoptimizer；M82 无 gate。单测网格偏小（uv_sphere/plane_grid） | meshoptimizer `simplify*` / `clusterlod.h` 保边界 QEM。质量差会在远处裂缝/糊成一团 | P1（#58 的资产质量前置） | 不改 RXGB 簇上限；可先做 **meshopt 参照器臂**（同输入比误差/裂缝/三角数，低于阈值 RED），禁静默替换事实源 |
| 67 | **HLOD 质量烘焙（合批+简化+图集）** `[遗漏]` | `bake_hlod` 按 Component **分别** `stride = 2^level` 抽三角，无空间合并、无缝合、不是 QEM。运行时只 host 选层 | UE5 WP HLOD：cell 内多 Actor 先合并再简化/图集；对 Nanite 内容侧重合批 | P1 | #25 是 L4 Far Field / proxy GPU；本行是**烘焙质量**。跨组件合并细节见 #97 |
| 68 | **HLOD 代理 GPU 绘制腿** `[遗漏]` | G27 重判窗：proxy 追踪 device 腿零实现 | 选层不画出 = 远处仍空或仍吃全量。须互斥：同 cell 全量 XOR 代理，零双绘 | P1 | 与 #25 合取改判 L4；本行先让画面成立 |
| 69 | **Nanite 式两层内存：层次常驻 + 页按需** | 簇记录预留 `page_id`；页流送 P4-1 open | Nanite：Cluster Hierarchy 常驻做 cut，Streaming Pages GPU 常驻解压簇。只做 cut 不做页 = 大网格仍爆显存 | P1 | **兑现面 = #20**，本行只作 #58 的内存模型注释，不新开 P4-1 |
| 70 | **Impostor / 广告牌超远场** `[遗漏]` | 无 impostor 模块；`uc03` billboard 是粒子，不是资产 LOD | 八面体/卡片 impostor 比 HLOD 网格更远，树/道具远景常规 | P2 | 远于 #67/#68；与 #25 proxy **互斥不互相充数**；#11 植被窗触发 |
| 71 | **多尺度 World Partition 网格** `[遗漏]` | `world/partition.rs` 单网格 cell + 可选 HLOD 引用 | UE5 实践：地形/建筑/小道具分网格、cell size 与 loading range 递缩，避免整城随一颗道具加载 | P2 | M110 已有单网格；本行是多网格策略 |
| 72 | **远景 Runtime Virtual Texture / 地形代理** `[遗漏]` | 地形模块在；SVT 四行 #33–36 open；无 RVT bake | UE5 WP RVT Builder：远景低分辨率烘焙虚拟纹理，替代远 terrain 全量采样 | P2 | 消费方接线归 #36，本行是**远景烘焙代理** |
| 73 | **RT 友好空间簇化（SAH / meshopt Spatial）** `[遗漏]` | CLAS 烘焙吃 geom-build 簇 AABB；簇化按邻接贪心（光栅友好） | meshoptimizer `buildMeshletsSpatial`：光栅簇 ≠ RT 簇；Mega Geometry / CLAS 要 SAH 边界。一份三喂（#26/M95）在质量上仍吃亏 | P2 | 动态/RT 合流需求出现时与 #26 同窗 |

### 7.4 GPU-driven 生产接线（host 金标准已在、出帧未走）

| # | 任务 | 现状 | 业界做法与价值 | 优先级 | 去重 |
|---|---|---|---|---|---|
| 74 | **VisBuffer 生产接线** `[遗漏]` | host `geometry/visbuffer.rs` + SW/HW SPV 构建件在；生产 Mega = `primary → scatter → reduce` **compute 直接光**，不写 64-bit VisBuffer 出帧 | Nanite / RE Engine / Destiny：可见性与着色解耦，材质 classify 一次、像素着色率可另定。REAC 2025：meshlet + VisBuffer + 软光栅是一套 | P0′（#58 的出图脊柱） | 不替代现 Mega 直接光对标臂；加性车道 |
| 75 | **小三角软光栅生产 + SW/HW 分箱** `[遗漏]` | host `compact_draw_args` 32px 阈值；`src/soft-raster` 是 G0 CPU 软光栅，**不是** GPU compute 软光栅 | Nanite/RE：亚像素三角 HW 光栅亏，compute 软光栅 2–3×。Alan Wake 2：无剔除不可交货 | P1 | 接 #74；#24 mesh shader 是第三路径，本行是 compute 软光栅 |
| 76 | **GPU compact + multi-draw-indirect-count 零回读** `[遗漏]` | host `compact_draw_args`；DGC/M105 零回读在 GPU-driven 提交面，**几何剔除 compact 未进生产** | Pulsar/AAA：atomic 追加可见簇，一条 `multi_draw_indexed_indirect_count`，CPU 每帧 O(1) | P1 | 与 #21 反馈链同族「GPU 写请求/画命令」；DGC 行不重开 |
| 77 | **实例/簇两级 GPU 剔除 kernel** `[遗漏]` | `cull.rs` 写明是「未来 GPU kernel 金标准」 | RE/AW2：视锥+锥+Hi-Z+LOD 全 GPU。200k 实例必须 GPU-driven | P1 | **Hi-Z 两阶段 = #6**；本行是 instance/cluster cull device，#6 之后接线 |
| 78 | **VisBuffer + VRS 着色率解耦** `[遗漏]` | 零 VRS | Advances 2024：VisBuffer 的着色率可低于几何分辨率。远景/背景 VFX 降 2×2/4×4。与 DLSS 互斥需 fail-closed | P2 | 依赖 #74；能力探测进 #50 矩阵 |

### 7.5 后处理、呈现延迟、CPU 提交（调研补遗）

| # | 任务 | 现状 | 业界做法与价值 | 优先级 | 去重 |
|---|---|---|---|---|---|
| 79 | **后处理五级链 device 接线** `[遗漏]` | M119 host 骨架：曝光→bloom→tonemap→LUT→输出；`g14_3` 出图臂有一条 post 路径，**实时窗口未恒走完整五级**。#3 只保证 EV 能进 uniform，不是 histogram 自适应 | 出货立刻缺的三件：自动曝光 / mip bloom / ACES·AgX 进真窗口（#17 是交换链 HDR 元数据，本行是场景线性→显示编码） | P0′ | 与 #17 正交；bench 常把 tonemap 钉死 off 不充接线 |
| 80 | **运动模糊 / 景深生产** `[遗漏]` | 无 MB/DoF 模块；MV 通道已按三类速度设计（RD-041） | 电影感常规；缺 MV 必鬼影。须全动态源写速度（蒙皮/WPO/粒子） | P2 | 依赖 #4/#10 MV 完整；与 TSR/DLSS 互操作要测 | 
| 81 | **粒子/半透明/WPO 写速度** `[遗漏]` | 蒙皮 WPO MV 接口已设计未进生产（#10） | UE 超分调参铁律：Niagara CPU 粒子、WPO、程序动画漏写速度 → TSR/DLSS 鬼影 | P1 | #10 触发动态资产面时本行一起验收 |
| 82 | **PSO precache 生产接线** `[遗漏]` | `material/pso_cache.rs` host 预测/预编译告警在；生产车道现场编译面未接到窗口 demo | 出货清单：首见特效 hitch ≈ PSO stutter。加载期笛卡尔积 precache，运行期告警=0 | P1 | 与 #51 健壮性、#9 纹理/材质变体联动 |
| 83 | **NVIDIA Reflex / 低延迟绑定** `[遗漏]` | FG ×2/×3 已接线（#5 / G31 A5）；无 Reflex/帧步限 | FG + VSync = 顿挫；Reflex 限制队列深度换输入延迟。商用 FG 标配 | P2 | 生成帧禁入真实帧率口径维持；本行只绑延迟 |
| 84 | **FG 后 UI overlay 不进插帧** `[遗漏]` | 窗口 demo 无游戏 UI 层 | UE：HUD 须 UI-aware blit，否则插帧扭曲准星/文字 | P2 | 游戏 UI 进画面时触发 |
| 85 | **Sampler Feedback 纹理流送** `[遗漏]` | 页式流送是反馈驱动纹理页；无 DX/VK sampler feedback 硬件回读 | 采样 miss 由硬件记，比 UAV 反馈准。与 SVT-2（#34）同族 | P2 | **不新开 SVT-2**；#34 立项时评估硬件反馈 vs UAV |
| 86 | **流送/PSO 双预算防 hitch** `[遗漏]` | 流送三预算（io/transcode/upload）在；与 PSO 编译预算未合成帧预算 | 出货：区域切换 hitch = 流送或 GC；须「每帧上传字节 + 每帧新 PSO」双上限，超则延后 | P2 | 复用 `StreamingBudget`；与 #82/#23 联动 |
| 87 | **Clustered / tiled light cull 与主几何重叠** `[遗漏]` | 低档多灯 MegaLights 为默认档（#7 高档 ReSTIR 另线） | Khronos 点名 Forward+ cull 是 async 首选重叠件 | P2 | **算法本体 = #107/#108**；本行只调度重叠，勿并进 #7 |

### 7.2b 异步调度调研补洞

> 与 #57/#59–#65 去重后只追加：图执行合流、窗口 FIF 补洞、FIF×动态、transfer 队列、同队列 event、TSR async、重叠量自动回退。

| # | 任务 | 现状 | 业界做法与价值 | 优先级 | 去重 |
|---|---|---|---|---|---|
| 88 | **CompiledGraph 驱动生产执行器** `[遗漏]` | Mega 生产车道手写 pass 列表进 `DeviceFrameSession`；图编译的屏障/车道/`FencePair` **与执行断链**（`CompiledGraph::execute` 只产 `CommandLog`） | UE RDG：同一张图同时驱动剔除/别名/重叠。不断链则 #57 无法落到窗口 | P0′（#57 的合流前置） | #1–#8 是特性接线窗，不是图执行合流 |
| 89 | **真窗口/游戏循环消费 FIF**（#2 补洞，不重开 API） | `submit`/`collect`/`--inflight 2\|3` 仅 `g14_3 --bench --backend tsr_device`；`g31_window_present` / 动态 / 蒙皮仍走 `execute_with_frame_update` **当帧 fence**。§1 #2 文案仍写「本波不动」= 历史镜像过时，以本行为准 | 窗口去掉当帧 `QueueWaitIdle`，CPU 录制与 GPU 跨帧重叠 | P0′ | **不重立项 FIF API**（G31 A2 bench 臂已有）；#3 循环改走已有入口 |
| 90 | **FIF 与动态/蒙皮共存** `[遗漏]` | FIF 拒 `tlas_update`/`blas_refit`；动态/蒙皮被迫顺序提交 | 每槽实例缓冲 / BLAS 顶点，动态角色也能 N 帧在飞 | P1 | #2+#4+#10 约束面，勿另开「再做 FIF」 |
| 91 | **专用 transfer 队列上传重叠** `[遗漏]` | `queue.dedicated_transfer` 仅 capability ID；`vk.rs` 无 transfer 句柄。上传走 graphics/compute | Frostbite/Anvil：copy queue 做贴图/UBO/staging，与绘制重叠，藏 H2D | P1 | 勿并入 #43（#43=语言 AsyncBuffer/多 stream；本行=渲染上传队列）。#64 是多 timeline 分轨，本行是 **transfer family 真句柄** |
| 92 | **同队列细粒度重叠（VkEvent / 窄掩码）** `[遗漏]` | 全仓零 `VkEvent`；屏障偏保守整障 | 无第二队列时也能让无关 compute 与后续图形重叠，少 bubble；须不破 RXS-0239 happens-before | P2 | 单队列回落路径的收益面；与 #57 双队列正交 |
| 93 | **TSR/超分非关键 pass 进异步队列** `[遗漏]` | TSR resample/resolve 与 scene 同 session 串行 | 对齐 UE `r.TSR.AsyncCompute`：历史/拒绝等非关键趟进 compute 队列，与下一帧主视重叠 | P2 | 勿并入 #5（#5=帧生成接线，另一条时间线） |
| 94 | **重叠量测量驱动自动回退** `[遗漏]` | `enable_async` 是编译开关，无运行期时间戳回退；DGC preprocess 纪律维持单队列（M59） | 重叠低于阈值自动回落，避免假 async 加同步税；DGC 仍单 queue 全序直至 measured 收益齐 | P2 | #54=对外剖析面（可复用时间戳）；#40=Work Graphs，勿混立项 |

### 7.3b 虚拟几何调研补洞

> 与 #58/#66–73/#74–77/#11/#20–26 去重：不重开生产消费 DAG、stride HLOD、impostor、双簇化器、GPU 簇剔除、P4 流送、mesh shader、proxy GPU、BistroExterior。

| # | 任务 | 现状 | 业界做法与价值 | 优先级 | 去重 |
|---|---|---|---|---|---|
| 95 | **World Partition 生产接线（cell 进 g14/g31）** `[遗漏]` | `partition.rs`/`hlod.rs` 是 host schema + 事件总线 + 选层；g14/g31 **0 引用** | UE WP：距离环 load/unload + 预算排队 + 与 HLOD 互斥。没有它，大世界 cell 永不进 Mega 车道 | P1 | #71=多网格策略；#11=Exterior 资产；#20–23=簇页。本行是 **cell 装载进生产会话** |
| 96 | **属性保持简化（UV / 法线 / 切线）** `[遗漏]` | `TriMesh` 只有 position+index；`mesh.rs` 写明「P0 不引入法线/UV」。生产贴图均值也不走簇 UV | `meshopt_simplifyWithAttributes`；Nanite 簇内插值属性。无 UV 的 DAG 无法喂 #9 纹理管线 | P1 | #66 的属性腿；#9 是采样接线，本行是 **cook 输入扩面** |
| 97 | **HLOD 跨组件先合并再简化** `[遗漏]` | `bake_hlod` 逐 Component 分别抽样，无空间合并 | UE：cell 内多 Actor 合成一块 proxy，减 draw。只抽面不合批 = 远处仍多 draw | P1 | #67 的合并细则；运行时零合并 RED 锚不动 |
| 98 | **Metis / meshopt_partitionClusters 图分区对照** `[遗漏]` | 分组 = 共享边加权贪心 + Morton 种子；树内无 Metis | 完整图分区减裂缝、改善父簇形状。贪心可留作确定性默认 | P2 | 不并进 #66 的 QEM 简化器；对照臂独立 |
| 99 | **Continuous LOD / geomorph / 时域淡入** `[遗漏]` | 离散 DAG cut（自身不可感知且父级可感知）；geomorph/dither **0 命中** | 切层淡入减轻 popping。固定轨迹 popping 指标进 evidence | P2 | 不改 #22 驻留回退语义 |
| 100 | **Data Layer 激活语义** `[遗漏]` | 掩码位可往返；`data_layer_active` **fail-closed**，v2 才接线 | UE WP Data Layers：与空间网格正交的 gameplay 加载维 | P2 | 大世界 gameplay 需要时再开；与 #95 距离环合取 |
| 101 | **link-condition / fold-over 防护** `[遗漏]` | `lib.rs` 写明未做拓扑校验；极端输入可能折翻，误差上界仍保守 | Hoppe link-condition；折翻输入 typed `Err` 拒录，不静默出 DAG | P2 | #66 的正确性护栏，不替代 QEM |
| 102 | **分块并行 cook（bistro 级确定性 DAG）** `[遗漏]` | `build_dag` 单线程；cook 默认 `tri_min.gltf` / `geom_segments=12`；**bistro 生产绕过 cook** | zeux clusterlod 多线程处理亿级三角。bistro 不出 DAG，#58 无资产 | P2 | 不含 #20 页驻留池；双构建 hash 跨线程不变 |

### 7.5b 渲染器全谱调研补洞

> 对照 #1–56 与 §7 已列：不重开 compact/MDI（#76）、VisBuffer/软光栅（#74/#75）、VRS（#78）、MB/DoF（#80）、Reflex（#83）、Sampler Feedback（#85）、PSO precache（#82）、多线程录制（#65）、SMRT（#27）、ReSTIR（#7）、clipmap 演进（#28）。

| # | 任务 | 现状 | 业界做法与价值 | 优先级 | 去重 |
|---|---|---|---|---|---|
| 103 | **Persistent threads 剔除/着色** `[遗漏]` | 无常驻 wave / kernel 内工作窃取 | 减少空 dispatch 与小 batch 启动开销 | P2 | 勿并入 #40（WG 是图调度；本行是 kernel 内窃取） |
| 104 | **VSM 页失效 / clipmap 生产接线** `[遗漏]` | host `shadow/vsm.rs` mark/alloc/invalidate 齐；g14 `shadow_scatter` **不是**这条页管线；`vsm_page_mark_project` 曾「编进 SPV 无人 dispatch」 | 动态物体/灯变只重绘脏页，否则每帧全量阴影不可扩展 | P1 | 勿并入 #27（SMRT 是光追软阴影） |
| 105 | **PCSS 软阴影** `[遗漏]` | 无 | 接触硬化的光栅软阴影，覆盖多数动态灯，成本远低于 #27 | P2 | 与 #27 分档，不互相充数 |
| 106 | **缓存阴影生产接线** `[遗漏]` | host `shadow/page_cache.rs`（M19 16 帧金标准）未进窗口车道 | 静态/缓动灯跨帧复用页 | P1 | 与 #104 同窗；#27 仍是 SMRT |
| 107 | **Clustered / Tiled / Forward+ 光照档** `[遗漏]` | 无光照实现；贴花 `DECAL_CLUSTER_DIM` 只占位「复用光照 cluster 结构」 | 屏幕 tile×深度切片挂灯，前向可带大量动态灯 | P1 | #87 是重叠调度；#7 是 ReSTIR；本行是 **算法档** |
| 108 | **GPU light culling** `[遗漏]` | M100 MegaLights = 固定随机选一灯，不是 per-tile cull | 每 tile/cluster 产出可见灯列表，避免逐像素扫全灯 | P1 | 与 #107 同窗；勿并入 M100/ #7 |
| 109 | **Probe volume / DDGI 生产接线** `[遗漏]` | `gi/if_tier.rs` L1 写「DDGI 基线」；文档写明 Resampling 未做；未进 g14/g31 | 室内/中距 GI 的光栅体积探针档 | P2 | 勿并入 #28（世界 clipmap 辐射缓存演进） |
| 110 | **Bindless 贴图表生产** `[遗漏]` | RHI `vk.rs` G3.4 无界表在；生产是定长 `MaterialTable` + mega kernel，**未走 bindless 贴图** | #9 贴图场景的换绑前提；否则 per-draw descriptor 撑不住 | P0′ | 语言 bindless 面已稳定；本行是 **生产车道消费** |
| 111 | **Classify / resolve 分箱生产** `[遗漏]` | host `geometry/material_pass.rs`（tile×材质 + 16 位窄缓冲）未进 Mega | VisBuffer 后同材质成批着色，降带宽 | P1 | 依赖 #74；不并进 #8 slab |
| 112 | **Shader permutation 爆炸治理** `[遗漏]` | host 变体键在 `pso_cache.rs`；G8 M29 曾登记缺口，本表未单列 | specialization / uber + 静态裁剪，控 PSO 数量与 hitch | P1 | 与 #82 precache 正交（一个少变体，一个预编译） |
| 113 | **Pipeline warmup 服务** `[遗漏]` | 无独立 warmup；bench `--warmup` 是测量窗，不是进关预热 | 启动/进关走一遍 PSO+资源，消首次遭遇卡 | P0′ | 勿并入 #82；勿并入 bench warmup |
| 114 | **Descriptor 池 / 按帧回收** `[遗漏]` | 仅 set-per-class 建面，无池化回收器 | 避免每帧创建与池耗尽 | P1 | 渲染器面；与 #51 device-lost 正交 |
| 115 | **统一显存预算回收账本** `[遗漏]` | 流送 PagePool LRU、分区 MemoryBudgetMB、VSM 页 LRU **各算各的**，无 renderer heap 总账 | 按堆类型记账、主动驱逐/整理 | P1 | #51 是超额后降级，不是日常回收 |
| 116 | **渲染 Job 图** `[遗漏]` | 无渲染 job DAG；Jolt job 只在物理 | 剔除/动画/上传/录制成 DAG，与物理线程池解耦 | P1 | 勿并入 #65（#65=录制并行；本行=CPU 工作编排） |
| 117 | **TBDR 友好 pass 合并** `[遗漏]` | 图编译不按 tile store/load 约束排 pass | Mali/Adreno 主收益：少 framebuffer fetch、合并 pass | P1 | 移动/低端面；与 #50 降级链联动 |
| 118 | **Forward+ 低端光照降级档** `[遗漏]` | 无主光照 Forward+ 档；贴花「前向回退」只覆盖 DBuffer | 低端关掉延迟/RT/mega，走 clustered 前向 | P1 | #50 的 RayQuery→屏幕空间不是这条；算法依赖 #107 |

### 7.6 调研结论（给立项用，不充绿）

1. **两条会话缺口都不是「没设计」**：异步有编译规划、大网格有离线 DAG + host cut。缺的是 **device 提交 / 生产出帧 / 页流送 / 代理质量**。cook 出的 RXGB 被丢掉，bistro 绕过 cook，g14/g31 仍全量三角汤。
2. **Nanite 观感 = 四件套同时成立**：①簇 DAG 质量（#66）②生产 VisBuffer/软光栅（#58/#74/#75）③页流送（#20–23）④远处 HLOD/impostor（#67/#68/#70）。只做其中一件会假绿。**#20–23 没有 #58 就没有上游。**
3. **异步计算 = 三件套**：①RFC 修订 RXS-0239（#57）②timeline 提交器（#59）③有时长的候选 pass（#60）。没有 ③ 的重叠量，①② 只是机制正确性（G5 已做过 host 门）。合流前置 = #88（图驱动执行器）；窗口收益前置 = #89（FIF 进 present）。
4. **不要用 stride 抽面或全驻留 cut 冒充「远处自动缩小规模」。**
5. **真窗口「游戏画面」立刻缺的（全谱 P0′）**：bindless 贴图（#110）、曝光/bloom/tonemap 接线（#79）、PSO warmup（#113）。VisBuffer 三联与 VSM/缓存阴影跟在 #6/#27 之后开窗，不要并进那两行。

---

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.1.7 | 2026-08-27 | **G36 W1-W4 互斥项修复交付登记**（门 `g36.wave1.geo_composition`，evidence/g36_geo_composition_gate_*.json；milestones/g36/ 四件套 + ci/g36_geo_composition_smoke.py；v1.1.5/v1.1.6 已知限制"闭集互斥（组合面归后续波）"行兑现）：①**W1 逐三角 provenance 地基**（g14_3_lane_body 加性）——TriProvenance（Src\|簇粗代理\|cell 代理）+ geo_rebuild 统一重建 + 侧表 gather（UV 位保真/代理 tritex 强制 −1 常量面回退〔#96 属性保持简化留窗〕）+ regroup_nodes 节点段重导出（AABB 自重建几何精确重算,三角 ⊆ AABB 精确包含维持）+ 恒等排列逐位锚;选层机核抽取共用（cluster_lod_select/wp_hlod_select），单开路径 0-语义漂移（off==leaf==wpfull 三臂 digest 位级机核,== 在案锚 sha256:f39e9808… 同值）②**W2 cluster×wp 互斥解除**：apply_geo_combined 四态分派（WP cell 互斥选层先行 → Full 域内簇 cut → 组共享多父 DAG 语义下粗簇集合化判定〔⊆F 出帧/跨界叶级回退差集化防双绘/全外域归 cell 代理〕）+ 覆盖机核 fail-closed（identity 恰一次 + identity×粗簇域零交叠 + ≡ WP Full 域恰等）+ leaf×full 极限 == off 逐位锚;g14_3 三条互斥撤除（cluster×dyn/skin、wp×dyn/skin、wp×cluster——尾接段基址在重建后计算,组合面成立;bench 混合臂 5 Full/15 Hlod：identity=236894+粗簇 11524〔1901 簇〕+跨界回退 3270〔79 簇〕+cell 代理 388975=637393 出帧,双跑位级）③**g34 全特性组合接线**：统一主车道五特性（cluster×wp×纹理×slab×动态——五特性 leaf×full digest_seq 逐帧 == --full 基线;混合组合 host 金标准对拍 p100=2.599e-4 ≤ 冻结容差 7.937e-4〔光线/材质/质感两臂一致机核〕）+ HZB 区段六特性（重导出 337 节点段进逐节点 BLAS 分解,金字塔 12 级位级全等 + 判定逐字节全等 + 零假阳性 + 真实剔除 measured）;geo on 时 evidence schema/gate 切 g36 字面（G34 注册面 0-byte）④**#13/#81 联动**：g35 粒子车道 × geo 组合（--particles on --oit wboit\|sorted × cluster×wp 双跑 presented digest 位级;见证/RED 臂语义互斥维持——标定夹具构型如实拒跑）⑤**留窗如实登记（不冒充）**：FIF×动态（#90——RFC-0030 §4.3 L2 共享 host 写面语义,真修复 = 每槽实例/TLAS 副本 + 每槽 AS 描述符集,触冻结确定性协议须 RFC 修订行）/ FG 组合（G34 契约 out-of-scope「归后续波不预支」字面管辖）/ HZB×蒙皮同车道（新 kernel 合并面）/ #96 代理属性保持简化 / 逐帧 device cut→AS 更新（#77/#89 合流窗）/ g31_window_present 冻结 bin 互斥字面维持（五门回归锚,组合面由 g34/g14_3/g35 车道承载）。既有行字面 0-byte 不回写 |
| v1.1.6 | 2026-08-27 | **#95/#68/#99 WP cell + HLOD 生产接线交付登记**（门 `g31.wave95.wp_hlod` PASS，evidence/g31_wp_hlod_*.json；v1.1.5 已知限制三行留窗兑现 + 计划 F 阶段收口〔F1 质量烘焙已于 v1.1.5 ⑦ 交付〕）：①三步资产链——`g14_3 --dump-scene`（RXCS 装配 dump 复用）→ `g31_wp_hlod_bake`（XZ 正方形 cell 网格〔边长 = 资产属性,bistro 4m × 30 cell/20 非空〕→ 逐 cell **跨组件〔节点段〕先合并再简化**：`bake_hlod_merged` QEM 链事实源直调〔#67/#97 字面,L0 全量 + L1/L2/L3 = 100.2万→50.1万→25.1万→12.5万精确减半〕→ RXHL 资产字节 + digest 寻址 → RXWH cell 包,double-build 字节相等）→ 生产车道 `--wp-hlod off\|full\|on` 加性开关。②**#95 生产机核直调（cell 进 g14/g31 会话）**：`world::partition::PartitionRuntime` 距离环 load/unload + 三项预算排队（预算 4 cell/帧下 stall_frames=7 真实记账进 evidence）+ `world::hlod::HlodRuntime` 事件总线消费/实载资产 digest 核验/screen-size 阈值互斥选层——**零复刻全走 M110/M111 冻结机核**；`full` 全量臂与 off 三角汤**末帧 digest 位级一致**（bin 内嵌逐三角位级断言 + GPU 端到端双证）。③**#68 HLOD 代理 GPU 绘制腿 + 互斥切换协议**：代理三角随互斥重建进 BLAS 出帧（选层真画出）；三档 t0=0.25/4.0/8.0 out_tris 62.8%/33.5%/29.3% 阈值单调下降（远景三角数下降 measured,阈值层间 ÷16 = 切换距离逐层 ×4〔UE 经验字面〕）+ 混合臂 5 Full/15 Hlod 同帧互斥并存；同 cell 全量 XOR 代理（互斥机核 + 源三角零重复断言 = 零双绘 fail-closed）；切换 = warmup 预热 N 帧后同帧原子翻转（UE bRequireWarmup 模式,flip−request==warmup 逐事件机核）；四 RED 臂子进程独立检出（tamper-digest 读取期拒篡改/event-order 状态机拒乱序/double-draw 互斥机核拒双绘/runtime-merge RXS-0364 L3 零合并锚恒拒）。④**#99 popping 指标进 evidence**：g31 窗口 headless dolly 固定轨迹逐帧 tick/选层/切换统计 sidecar（切换事件表〔cell 27 L2→L1 请求帧 10→翻转帧 14〕+ 逐帧翻转数/三角跳变 delta_max=7,warmup_protocol_verified 机核）；geomorph/dither 过渡评估留窗。⑤画质差 off vs on(4.0) 收敛帧 EXR diff err_p95≈2.2e-3/err_max 1.0/零超阈区域 + frame_ms 11.78→10.79ms measured 如实登记；bench on 双跑 digest 位级 + 选层序列 digest 一致。schema 注册 `ci/_patch_g31_wp_hlod_schemas.py` 三处纯追加 + `milestones/g31/g31_wp_hlod_evidence_schema.json` + `ci/g31_wp_hlod_smoke.py`（selftest 4 正则 GREEN + schema 互核）。**已知限制如实登记（不冒充）**：出帧几何冻结于装配期选层（逐帧 AS 更新归 #77/#89 合流窗）；`--wp-hlod` 与 --cluster-lod/--hzb/--textures/--slab-table 闭集互斥（两套几何重组各自重排三角汤,组合面归后续波）；HLOD 图集烘焙依赖纹理管线后置（#67 图集半留窗）；多级 cell size 层级链（粗层大 cell 网格）单场景不触发留窗；#70 impostor 与 #25 L4 far field 各自独立不互相充数。既有行字面 0-byte 不回写 |
| v1.1.5 | 2026-08-27 | **#58 生产消费簇 DAG 首波交付登记**（门 `g31.wave58.cluster_lod` PASS，evidence/g31_cluster_lod_*.json）：①三步资产链——`g14_3 --dump-scene`（RXCS 装配 dump）→ `g31_cluster_lod_bake`（bistro 104.6 万三角全量 DAG，187 块并行 ~0.7s + double-build 字节相等）→ 生产车道 `--cluster-lod off\|leaf\|on` 加性开关：leaf 全叶臂与 off 三角汤**末帧 digest 位级一致**（bin 内嵌逐三角位级断言 + GPU 端到端双证），on 臂 1px/4px 三角 90.7%/54.7% 阈值单调下降、off vs on(4px) 收敛帧 EXR diff err_p95≈1.1e-4/err_max 0.72/零超阈区域，g31 窗口 dolly 轨迹逐帧 host cut 统计 sidecar（cut_tris 794k↔809k 相机真实驱动）；`cook.rs` RXGB 算即弃修复（落 artifacts/geom_dag.rxgb 进 manifest）。②**组共享 LOD 判定球**（B4 提前兑现，Nanite "same input→same output" 语义）：自心判据在 bistro 近距**实证**祖先-后代同选（verify_cut_coverage fail-closed 拒帧在案），新增 `select_lod_cut_grouped` 生产金标准（球面最近点投影）+ `rurix-geom-build::lod_bounds::derive_lod_bounds`（叶几何球→组并集球沿链单调嵌套 + 嵌套机核 typed Err）；剔除紧球/LOD 并集球分离经 RXCP 平行表承载，**64B ClusterRecord 契约 0-byte**（原计划 RFC 修订面免动）。③ **#66/#98 加性质量档**：QEM 简化器（`qem.rs`：属性四次型前置的位置 QEM + 最优点求解 + fold-over 拒绝(dot≤0.05) + stuck 出口(>0.85 登记)；上报误差口径 = 既有位移保守累计上界 **0-语义漂移**，单调性证明不变）+ `DagBuildParams::quality()`（8 簇/组 + 非同胞边权 ×16 边界交替，Nanite ClusterDAG.cpp 字面）——默认 `build_dag` 0-byte（m90 DAG digest golden 零漂移），bake 默认质量档（uv_sphere 误差上界 2.13→1.30、stuck −52%、bistro 层数 17→13）；#66 meshopt 参照器臂立项字面维持（事实源未静默替换）。④ **#77 簇剔除 device kernel 化 + #6 两阶段第二段簇级下沉**（`kernels/g31_cluster_cull.rx` + `g31_cluster_cull_device` harness）：三关 + 组共享球 cut + 簇级两阶段 HZB（上帧金字塔初剔 + 本帧重测 disocclusion）device 真跑——判定序列逐项全等/最终可见集全等/exact_rect_occluded 裁判零假阳性/双跑位级/disocclusion 存在性/tamper RED 臂全绿。⑤ **#74/#75/#111 机制链**（harness `--visbuffer` 臂）：cut→`compact_draw_args` 32px SW/HW 分箱→SW compute 软光栅 device 真跑（M95 u64 原子腿转正消费,覆盖集合与 host oracle 全等+双跑位级）→合并→`classify`/`resolve` 材质分箱;HW device 腿复用 M95 diff=0 锚登记;生产 pass 序接线与 MDI count 变体留窗。⑥ **#20–23 驻留回退 cut 进生产出帧**：bake 页分配（页 0 = 各块顶层钉住 + 64 簇/页，bistro 1712 页,64B 记录 page_id 预留字段兑现）+ `--cluster-resident-pages` 压力臂（`apply_page_fallback` RXS-0350 L3 生产金标准直调 + 兜底后覆盖复核；200/1712 页→8.7% 三角回退出帧成功,饱和驻留(≥总页)与全驻留 **digest 位级一致**）；GPU 缺页请求闭环维持 C11 `g31.waveC.p4stream` 锚不重做。⑦ **#67/#97 HLOD 质量烘焙**：`bake_hlod_merged`（跨 Component 合并 + 位置 bits 焊接 + 逐层 QEM 减半：uv_sphere 4 象限 1104→552/276/138 精确减半 + 球面邻域质量粗判；L0 与既有 stride 烘焙器**逐位同值**——运行时 Full/HLOD 互斥切换协议(RXS-0364 三态)零改动,RXHL v1 结构兼容(合并层单 `__merged__` proxy)；双构建/声明序免疫/几何扰动分叉三判据同锚）——stride 抽面事实源 0-byte（M111 golden 维持）。schema 注册 `ci/_patch_g31_cluster_lod_schemas.py` 三处纯追加 + `milestones/g31/g31_cluster_lod_evidence_schema.json` + `ci/g31_cluster_lod_smoke.py`。**已知限制如实登记（不冒充）**：出帧几何冻结于装配相机 cut（逐帧 device cut→AS/出帧更新归 #77 生产接线与 #89 FIF 合流窗）；`--cluster-lod` 与 --hzb/--textures/--slab-table/--dyn-demo/--skin-demo 闭集互斥（cut 重排三角汤,组合面归后续波）；蒙皮资产 × QEM typed Err 拒录（叶层位置反查约束）；#68 HLOD 代理 GPU 绘制腿/#95 WP cell 进生产/#99 popping 指标留窗。既有行字面 0-byte 不回写 |
| v1.1.4 | 2026-08-27 | G35 立项（GPU 粒子系统期，milestones/g35/）：消费 #13 OIT 评估窗 re-trigger 条件命中（粒子特效进画面，milestones/g31/g31_oit_evaluation_window.json conditional_wiring_sketch ① 字面——消费 M120 冻结测量启动 WBOIT 起步选型，G35-4 兑现载体）+ #81 粒子写速度同窗验收（粒子 MV 进 MV 通道，G35-3 兑现载体）+ #80 依赖登记（运动模糊/景深依赖 MV 完整性——依赖登记不兑现）；既有行字面 0-byte 不回写，去重不重开 |
| v1.1.3 | 2026-08-26 | §7.5b 只追加全谱调研补洞 #103–#118（persistent threads / VSM·PCSS·缓存阴影 / clustered·light cull·DDGI / bindless·classify·permutation / warmup·descriptor·显存账本 / job 图 / TBDR·Forward+ 降级）；#79 升为 P0′ 并写明自适应曝光≠#3 uniform；#87 指向 #107/#108。不重开 #74–76/#78/#80/#82/#83/#85/#65/#27/#7/#28 |
| v1.1.2 | 2026-08-26 | §7.3b 只追加虚拟几何调研补洞 #95–#102（WP 进生产 / 属性简化 / HLOD 跨组件合并 / 图分区对照 / geomorph / Data Layer / fold-over / 并行 cook）；#58 收紧三角汤/TLAS/HZB 实例粒度/RXGB 丢弃证据；#66/#67/#70 补 meshopt 参照器、逐 Component 抽面、uc03 粒子 billboard。不重开 #20–26/#11/#74–77 |
| v1.1.1 | 2026-08-26 | §7.2b 只追加异步调度调研补洞 #88–#94（图驱动执行器 / 窗口 FIF / FIF×动态 / transfer 队列 / VkEvent / TSR async / 重叠量回退）；#57/#60 补 TIRT 句柄用途与 uc06 已标 AsyncCompute 证据；§1 #2 文案过时不回写，以 #89 为准 |
| v1.1 | 2026-08-26 | 只追加 §7：会话两条 #57（M59 异步车道 device 执行，镜像补登）+ #58（生产消费簇 DAG/LOD 出帧，遗漏）+ 并行调研 #59–65 / #66–73 / #74–78 / #79–87；§6 增第 4 条波次穿插；§0 头注 G31 波 A 已验收 #1–#5 最小面（§1 字面不回写）。调研对照 Nanite/WP HLOD、Khronos async+timeline、RE Engine meshlet、AW2 GPU-driven、meshoptimizer clusterlod、Advances 2024 VRS、UE 出货 hitch 清单。与 #6/#20–26/#24/#25/#33–36/#40 去重 |
| v1.0 | 2026-08-25 | 初版：汇总 g30-closed 时点全部已标明待办（承接注册表 9+8+6+legacy 11、16 行长线分项、尾锚六件）+ 遗漏项补全（实时呈现 #1–4、内容管线 #9–10/13、商业化工程面 #48–56） |
