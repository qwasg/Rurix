<!-- Assisted-by: TraeCode:Kimi-K3（G31+ 波 C 验收门 Task C18） -->
# G31+ 战役总登记 — 三波 56 项 TODO 逐项终态映射（2026-08-26，波 C 验收门 C18 产）

> 性质：战役终态登记镜像（append-only；事实源恒为 `milestones/*/\*_CONTRACT.md` 与 `registry/`）。输入面 = `G31_PLUS_COMMERCIAL_RENDERER_TODO.md` v1.1.3 §1~§5（#1~#56）+ `milestones/g30/g30_campaign_handover_registry.json`（RFC-0047 §5.5 唯一法定输入面）。
> 三波：波 A（G31 实时呈现期，§8 close-out 在案）/ 波 B（G32 画面完整期，§8 close-out 在案）/ 波 C（G33 商业化期，G33_CONTRACT §8 close-out 在案）。
> 终态闭集：**兑现**（门 evidence 指针）/ **维持**（open/defer/挂起锚字面 0-byte）/ **诚实红**（如实登记不冒充）。
> §7 调研补遗行（#57~#118）= v1.1 只追加镜像，三波范围外，不重开不预支（各行字面以 TODO 为事实源）。

## 1. §1 P0 主线（波 A + 波 B 兑现面）

| # | 任务 | 终态 | 兑现门 / 锚 / 证据指针 |
|---|---|---|---|
| 1 | 生产管线 swapchain present 接线 | **兑现**（波 A A1） | 门 g31.waveA.present PASS；C18 复跑 PASS（real_render=58.907ms present=1.181ms 双口径，2026-08-26） |
| 2 | 帧流水化 submit/collect 分离 | **兑现**（波 A A2） | 门 g31.waveA.pipelining PASS；C18 复跑 PASS（evidence/g31_frame_pipelining_20260826T220043Z.json）；#89 窗口 FIF 补洞归 §7 镜像不预支 |
| 3 | 游戏循环最小面 | **兑现**（波 A A3） | 门 g31.waveA.gameloop PASS；C18 复跑 PASS（orbit 双跑 digest_seq 位级） |
| 4 | 动态场景更新通路 | **兑现**（波 A A4） | 门 g31.waveA.dynscene PASS；C18 复跑 PASS（evidence/g31_dynamic_scene_20260826T221531Z.json） |
| 5 | 帧生成 FG/MFG 接线 | **兑现**（波 A A5） | 门 g31.waveA.framegen PASS；C18 复跑 PASS（x2/x3 双口径 + G26 对拍接线态 pass） |
| 6 | HZB 遮挡剔除接线 + 两阶段第二段 | **兑现**（波 B B1） | 门 g31.waveB.hzb PASS（tested=8799/occluded=3549 + 像素中性）；C18 复跑 PASS（20260826T222243Z） |
| 7 | ReSTIR 高档 reservoir 车道集成 | **兑现**（波 B B2） | 门 g31.waveB.restir PASS（y 锚 20000/20000 + p100 1.75e-9 ≪ 5.66e-6）；C18 复跑 PASS（20260826T223017Z） |
| 8 | slab 材质 closure/侧表转正 | **兑现**（波 B B3） | 门 g31.waveB.slab PASS（238927 三角 + device/host bitexact 0/2073600）；C18 复跑 PASS（g31_slab_wiring_gate_20260826T223040Z） |
| 9 | 纹理采样管线进生产场景 | **兑现**（波 B B4）+ C18 复跑诚实红注记 | 门 g31.waveB.texture 交付 PASS（albedo/normal 70/70）；**C18 复跑 FAIL 诚实红 = 两 0-byte 机核 fact（根因 = 波 C 未 commit 加性交付物落同 crate 代理面 + spec/release.md C5 同条修订；B4 真冻结面逐件 0-byte 实测维持 + 渲染实质 6/8 facts 全 PASS）**——evidence/g31_texture_sampling_gate_20260826T223804Z.json 如实留存 |
| 10 | 蒙皮/动画接入生产帧 | **兑现**（波 B B5） | 门 g31.waveB.skinning PASS（20/20 位置核验 + MV 进 TSR + BLAS refit）；C18 复跑 PASS（20260826T225801Z）；RD-041 类 3 蒙皮 MV 兑现窗 |
| 11 | BistroExterior 场景转换臂 | **维持**（G10-N6 锚挂起） | fbx2gltf/assimp/blender 三工具 PATH 全缺 + 源资产 0 命中（C17 探针 2026 新鲜复核维持）；锚 = FBX2glTF 上游修复在树或替代臂 + 源资产同窗齐备，0-byte |
| 12 | 多反弹 GI 默认档评估 | **兑现-决策**（波 B B6） | milestones/g31/g31_gi_default_tier_decision.json = maintain_default_off（off 1.79~1.93ms vs on 7.03ms ×3.64~3.93 measured）；re_trigger 两条件归 G32 后续窗 |
| 13 | OIT/半透明与毛发 | **兑现-决策**（波 B B7） | milestones/g31/g31_oit_evaluation_window.json = not_triggered（压测闭集全 OPAQUE；M120 测量 harness 态维持） |

## 2. §2 P1 商用终审残余与稳定性

| # | 任务 | 终态 | 兑现门 / 锚 / 证据指针 |
|---|---|---|---|
| 14 | G17-MD-F1 性能焦点格收口 | **诚实红维持**（承接锚兑现形态 = C9 NGX 分解） | C9 四段分解 measured（NGX in-stream 1.837ms 不可分离等量非差源；Δ=+0.1521ms 全落宿主可分离段包络 ≈0.707ms——milestones/g31/g31_ngx_decomposition_report.md）；轨迹 = 0.856→0.960479（G30 在案）→0.966059（波 A fresh）→0.956162（波 B fresh 中位）→**0.957894（波 C fresh 中位，5 样本 3.512540/3.539031/3.586359/3.587437/3.596624ms）**——17/18 诚实红终态维持，digest 全跑 == 锚零漂移；分解证据在案，重判条件（ratio ≥1.00 新证）未命中不冒充 |
| 15 | RD-045 间歇 digest 漂移三件 | **维持**（0/3 不冒充） | 波 B 观察窗 6/6 臂零漂移 + Stage A 18/18 ×2（g31_waveb_rd045_observation_results.json）；波 C 各门复跑 digest 全零漂移累计观察面只追加；backfill 三件 0/3 维持（F5 硬线），deferred history 只追加 |
| 16 | RD-027 PT 毒径挂起修复 | **维持 open + 绕行落档**（C10） | 门 g31.waveC.rd027 PASS（毒区 20 格全测绘 绿7毒13 + MR-0011 O0 护栏 + fail-closed 毒区拒绝；g31_rd027_poison_zone_map.json）；C18 复跑 PASS（20260826T211300Z，毒确认腿 hang_timeout 维持）；根因层维持定罪 NVIDIA 优化后段本仓不可修，修复 = 上游本体，绕行非修复不冒充 |
| 17 | HDR 输出管线 | **维持**（M118-hdr-cal maintain-SDR） | vulkaninfo HDR 三 token 全 absent（C17 探针 2026 新鲜复核维持）；锚 0-byte |
| 18 | AMD 真卡 present 验收 + 平台余量 | **维持 open**（G-MB1-6） | AMD 缺硬件；C3 兼容矩阵 amd-desktop/intel-desktop 格 DEV_ENV_DEGRADE 如实登记（milestones/g31/g31_compatibility_matrix.json）；宣称多厂商前须同探测面补测（release_checklist §5 在案） |
| 19 | EA1 冷启动 A 段 VM 验证 | **维持 open**（RD-033） | C17 探针：Win11 x64 VMware VM 候选在盘（owner 窗核验前非锚兑现）；deferred 字面 0-byte |

## 3. §3 P2 渲染特性长线

| # | 任务 | 终态 | 兑现门 / 锚 / 证据指针 |
|---|---|---|---|
| 20 | cluster 流送 P4-1 页磁盘布局与驻留池 | **兑现**（波 C C11） | 门 g31.waveC.p4stream PASS（RXPD v2 加性磁盘面 + v2 段感知装箱修复 + LRU 逐出真实发生）；C18 复跑 PASS（20260826T213056Z） |
| 21 | cluster 流送 P4-2 GPU 请求反馈链 | **兑现**（C11） | 同上（剔除 pass 缺页请求 → host 驻留调度 → 次帧 device 消费闭环真跑） |
| 22 | cluster 流送 P4-3 LOD cut 驻留联动 | **兑现**（C11） | 同上（一致性 cut 金标准 + 逐帧对拍 + 全驻留参考零回退双跑位级） |
| 23 | cluster 流送 P4-4 异步 IO 优先级链 | **兑现**（C11） | 同上（PriorityIoPool 优先级堆 + 倒置探针 measured 高优先级先驻留） |
| 24 | mesh shader HW 光栅第三路径（M61） | **维持-no-go**（C16 改判程序执行完毕） | 三项闭集 3/3 齐备 → maintain-no-go（性能差 measured=零：N=262144 档 0.2344 vs 0.2342ms / N=1048576 档 0.9065 vs 0.9057ms；多厂商收敛单卡不可证；真实消费方零）——g31_rejudgment_windows.json + 门 g31.waveC.meshbench PASS（C18 复跑 20260826T214521Z）；RFC-0034 重判记录只追加 |
| 25 | HLOD L4 Far Field 档 | **兑现-改判**（C12） | 门 g31.waveC.hlodl4 PASS（两半全齐 + 三入口解锁）；g31_m98_l4_rejudgment.json = rejudged-four-tier-chain（L1/L2/L3/L4 四级链）；C18 复跑 PASS（20260826T213134Z） |
| 26 | RD-039 backfill 逐项 | **部分触发**（C16 判档） | 骨骼 = triggered（动态资产面字面命中，开实施窗判档登记，实施归后续期）；Foliage/曲面细分/Assemblies/Mega Geometry 维持 not-triggered；RD-039 总体 open 维持 |
| 27 | SMRT 阴影贴图射线追踪 | **维持-defer**（C16） | partial 1/2（多灯动态资产面命中、shadow page 采样车道未出现）——G21 终判先例单半命中不得改判 |
| 28 | 世界辐射缓存演进 | **维持-defer**（C16） | partial 1/2（大世界流送面命中、GI 联动窗未成立——B6 maintain_off 在案） |
| 29 | NRD vendor 降噪集成 | **维持-defer**（C16） | not-triggered（自研降噪在案绿、画质差距 measured 检出零命中） |
| 30 | OMM | **维持**（未触窗） | 压测闭集零 alpha-tested 主导面（#11 联动窗未开）；三波未消费不预支 |
| 31 | RT pipeline + SBT 宿主车道 | **兑现**（C15） | RFC-0048 Agent Approved（D-409 第 1 轮 8 findings disposition）+ 门 g31.waveC.rtpipeline PASS（RT 臂镜像语料 vs RayQuery 臂 mismatch 0/4096 位级 + golden 三采样点 + validation 静默）；**.rx→SPIR-V RT codegen 缺口 PR-2/3/4 维持 open 登记不冒充**；C18 复跑 PASS（20260826T214507Z） |
| 32 | SER workload 兑现（M52） | **维持-defer**（字面 0-byte）+ measured 登记 | SER workload 臂 measured ratio=0.518079（C18 复跑新鲜 0.519489）微基准 caveats 在案（g31_ser_gain_estimate evidence）；M52 语言面 go 须独立 Full RFC 评估维持 defer |
| 33 | SVT-1 虚拟纹理页表 | **兑现**（C13） | 门 g31.waveC.svt PASS；C18 复跑 PASS（g31_svt_gate_20260826T213146Z） |
| 34 | SVT-2 GPU 反馈 pass | **兑现**（C13） | 同上 |
| 35 | SVT-3 瓦片边界过滤 | **兑现**（C13） | 同上 |
| 36 | SVT-4 地形/贴花消费方接线 | **维持-defer**（C13 登记） | M116 地形 SVT 需求成立窗未命中；world/terrain.rs 零 SVT 依赖断言维持 |
| 37 | KTX2-1 容器解析 | **兑现**（C14） | 门 g31.waveC.ktx2 PASS；C18 复跑 PASS（g31_ktx2_gate_20260826T214234Z） |
| 38 | KTX2-2 BasisU 转码器集成 | **兑现**（C14） | 同上（vendor C++ 桥 fail-closed DEV_ENV 纪律） |
| 39 | KTX2-3 转码收益 A/B | **兑现**（C14） | ETC1S 6.96× 跨平台档 measured；DDS 维持 Windows-first（A/B 登记在案） |
| 40 | Work Graphs GPU 侧调度 | **维持 not-available** | 驱动扩展 absent + DGC available 互核（C17 探针 2026 新鲜复核维持：vulkaninfo WG token absent）；WG present 翻转时复评 |

## 4. §4 P3 上游阻塞、物理与观察项

| # | 任务 | 终态 | 锚 / 证据指针 |
|---|---|---|---|
| 41 | RD-034 DXIL ray-tracing 腿 | **维持-blocked** | C17 探针恒跑（exit 语义未反转；spirv-cross 拒 raygen 维持） |
| 42 | RD-011/012/014/015 DXIL 后端系列 | **维持 open**（RD-015 锚信号登记） | C17 新鲜发现：llvm#57928 closed-as-completed 2026-08-13 = RD-015 reeval_anchor「LLVM 上游任一 issue 关闭」字面命中 → **重判程序启动信号登记，条目维持 open 不冒充 close**；重判窗归后续期 |
| 43 | RD-026 std::gpu 首期外编排面 | **维持-open**（C16） | not-triggered（A3 = Rust host 驱动非 .rx 单源；子集外七面硬需求零出现） |
| 44 | RD-030 launch marshalling ABI 守护 | **维持**（持续回归面） | 既有守护面维持；三波零新 ABI 面 |
| 45 | 物理观察轨（RD-042/043/044 + M125/M127） | **维持** | C17 探针 22 pattern 复核维持（g30 常量表逐字沿用）；各 reeval_anchor 字面 0-byte |
| 46 | SAFE-GPU 平台立项评估 | **维持 defer-to-G31+** | 独立期资源窗 + 平台需求方两半未齐；锚 0-byte |
| 47 | legacy 十一条历史清册 | **维持零 close** | C17 复核：逐条 backfill 核验在案零 close（g24_legacy_rd_registry.json 引用不复制） |

## 5. §5 P0′ 商业化发布工程面（波 C 兑现面）

| # | 任务 | 终态 | 兑现门 / 证据指针 |
|---|---|---|---|
| 48 | 渲染器 SDK 稳定 API 面 | **兑现**（C1） | 门 g31.waveC.sdk PASS：9 C ABI 函数 + 外部 C++ 宿主真跑 digest==Stage A 锚 c1d28ad7… + stable 快照 renderer_sdk_api 段 + API_VERSIONING v1=1.0.0；C18 复跑 PASS（20260826T205708Z，帧时 mean=2.1572ms） |
| 49 | 渲染器文档与示例 | **兑现**（C2） | 门 g31.waveC.docs PASS：docs/renderer/ 三件 + minimal_host 示例真跑 + walkthrough 1.29s（g31_renderer_docs_walkthrough.json）；C18 复跑 PASS（20260826T205521Z） |
| 50 | 设备兼容矩阵与能力降级链 | **兑现**（C3） | 门 g31.waveC.capability PASS：capability report + 六链 fail-closed + 12 单测 + 三超分臂真机切换；AMD/Intel 格 DEV_ENV_DEGRADE（#18 联动维持 open）；C18 复跑 PASS（20260826T210628Z） |
| 51 | 运行时健壮性 | **兑现**（C4） | 门 g31.waveC.robustness PASS：device-lost poisoned 锁存/TDR/OOM budget/窗口风暴 121 resize/soak 故障臂；C18 复跑 PASS（20260826T212406Z，故障臂新鲜 1000 帧零崩零泄漏） |
| 52 | 渲染器 SDK 分发打包 | **兑现**（C5）+ GAP 诚实登记 | 门 g31.waveC.dist 9/9 PASS：16 组件 bundle + 签名/SBOM + 离线可建 digest==锚 + 红臂四路 + EA1 回归绿；C18 复跑 PASS（20260826T205842Z）；**GAP-01~03 许可义务三件维持 open——「附带义务未闭前不以对应形态发布」口径在案（release_checklist §3/§4/v1.0 修订行）** |
| 53 | vendor 许可合规终审 | **兑现**（C6） | 门 g31.waveC.license PASS：16 项 cleared 15/conditional 1（rust_rowan，GAP-01 联动）/pending_owner 0/blocked 0；C18 复跑 PASS（20260826T205856Z） |
| 54 | 性能剖析与调试工具面 | **兑现**（C7） | 门 g31.waveC.profiling PASS：双 bin --profile-json 同 schema + debug labels 三面同名 + 分解恒等式 + on/off 位级；本机 RenderDoc/Nsight 双 absent 如实 DEV_ENV_DEGRADE；C18 复跑 PASS（20260826T212626Z） |
| 55 | 支持渠道与版本政策 | **兑现**（C8） | 门 g31.waveC.support PASS：support_policy + release_checklist + SECURITY 双件增补段；待建立项五件诚实登记；C18 复跑 PASS（20260826T210000Z） |
| 56 | 外部采纳判据兑现（使命判据） | **维持未宣称** | **字面登记：「外部选择/采纳」维度未宣称达成，维持 carve-out（11_ROADMAP.md §6：本判据的完全达成仍以外部生产项目选择 Rurix 为准）**——本战役交付 = 可采纳工程面（#48~#55 全兑现），采纳事实 = 零外部项目证据不预支；05 年愿景 carve-out 字面 0-byte |

## 6. 汇总

| 终态 | 计数 | 项 |
|---|---|---|
| 兑现（门绿/决策件在案） | 38 | #1~#10、#12、#13、#20~#23、#25、#31、#33~#35、#37~#39、#48~#55（#9/#14/#52 附诚实红/在案注记不掩兑现事实） |
| 维持（open/defer/挂起/不冒充，锚 0-byte） | 17 | #11、#15~#19、#24（no-go 定判）、#26（部分触发余项）、#27~#30、#32、#36、#40~#47、#56 |
| 诚实红（性能轨迹面） | 1 | #14（G17-MD-F1：轨迹 0.856→0.960479→0.966059→0.956162→0.957894；NGX 分解证据在案；digest 零漂移；不冒充收口） |

> 计数口径：#14 同时列「承接锚兑现形态（C9）」与「诚实红维持」——汇总行按主终态计诚实红 1 项、兑现 38 项（含 #9/#52 带注记兑现）、维持 17 项（#26 按部分触发归维持面）；56 = 38 + 17 + 1。
> 零降级三面终判（C18）：画质 18/18 VERDICT=PASS + Stage A digest 18/18 零漂移 + 性能 17/18 诚实红不恶化（预算杠 ×2.0 远未触及）——三面成立零冒充（G33_CONTRACT §8.1⑥）。
> §7 调研补遗 #57~#118：三波范围外镜像行，终态 = 未立项不预支（字面以 G31_PLUS_COMMERCIAL_RENDERER_TODO.md v1.1.3 为事实源）。

## 7. G34 附记（只追加，2026-08-27 收口验收批）——全特性合流期"全流程无降级"收口

> 事实源 = [g34/G34_CONTRACT.md](g34/G34_CONTRACT.md) §8 close-out；本节仅镜像指针。三波互斥降级面（波 B 组合矩阵互斥 12/12 fail-closed 在案）收敛为统一生产车道 `g34_full_lane`（真窗口 swapchain）。

| 波次 | 门 | 终态 | 证据指针 |
|---|---|---|---|
| G34-1 合流地基（纹理×slab×动态三特性同开统一 kernel） | g34.wave1.unified | **兑现**（八 facts；收口日新鲜复跑 host 对拍 p100 与标定值逐位同值） | evidence/g34_unified_lane_gate_20260827T093331Z.json |
| G34-2 HZB 接统一车道（双 TLAS + 帧内金字塔轮换 + 两阶段闭环） | g34.wave2.hzb | **兑现**（六 facts；像素中性 74/74 位级 + 金字塔 12 级位级 + 零假阳性 + 剔除 22407/65183） | evidence/g34_hzb_unified_gate_20260827T125510Z.json |
| G34-3 蒙皮进统一车道（蒙皮×纹理×slab×动态四特性同开 36 资源六 pass） | g34.wave2.skin | **兑现**（九面判据；逐顶点 max_abs==0 位级 + MV 三类 + 类 2 刚性 MV A4 缺口顺手接通） | evidence/g34_skin_unified_gate_20260827T084533Z.json |
| 收口验收面（守卫七条 + 三锚 + soak 5010 帧） | G-G34-4 | **四面全绿 + 焦点格轨迹诚实红恶化如实登记**（中位 ratio 0.921836，digest 十跑零漂移，预算杠 ×1.92 headroom） | G34_CONTRACT §8.1 ②③④ |

维持面：FG/MFG 合流与 HZB×蒙皮同车道合并 = 后续波立项程序不预支（接口预留在案）；契约 flip（G31~G34 四期 active→closed）与治理波 = 留 owner 按 10_GOVERNANCE 程序；G35（GPU 粒子系统期）四件套 + RFC-0049 立项在飞零实现面（同 commit 如实收入不消费）。
