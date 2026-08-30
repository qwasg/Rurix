# W3 深水区 — 异步 compute 三件套实施方案(#88/#57/#59/#60/#62 侦察 + 起草,不合入)

> 日期:2026-08-29。性质:**侦察 + 起草 + 可行性试做**;本目录为草案面,不进 spec/ rfcs/ milestones/ registry/,
> 生产车道(g31_window_present / g14_3_*)、graph/compile.rs 本体、rurix-rt vk.rs 本体零改动。
> 本窗新增代码仅 `src/rurix-render/src/bin/g31_async_lanes_probe.rs`(骨架,cargo check 过)。
> 配套:[RFC_DRAFT_RXS0239_amendment.md](RFC_DRAFT_RXS0239_amendment.md)(RXS-0239 修订行草案)、
> [PATCH_PROPOSAL_vk_timeline.md](PATCH_PROPOSAL_vk_timeline.md)(vk.rs timeline 最小 diff 提案文本)。

---

## 0. 结论速览

| # | 问题 | 结论 |
|---|---|---|
| ① | 断链点/消费者 | `CompiledGraph::execute()`(compile.rs:298-308)只线性回放闭包产 `CommandLog`,`fences`/`barriers`/`queue` 三产物零消费。全仓调 `execute()` 仅 2 处:uc09-taichi-spike(host.rs:129,单 pass copy 图)+ graph.rs 单测。uc06/uc08 持 `CompiledGraph` 只做审计对拍(fence 非空断言/pool 峰值/dump),生产 Mega 完全不消费 |
| ② | compute-only family / timeline | vk.rs 全部 ≈17 处 family 选择均「取首个含位 family」,**零 compute-only(COMPUTE 且非 GRAPHICS)探测**;timeline **探测面已在**(`DeviceCapabilityReport.timeline_semaphore`,G31+ 波 C),但创建/提交/等待 FFI 与 device feature 开启为**零**;GPU 时间戳基建(QueryPool/WriteTimestamp)为**零** |
| ③ | 判档载体 | **独立 harness bin `g31_async_lanes_probe`**(骨架已落)。生产 Mega 不消费 CompiledGraph(#88 未合流),uc06 是 host 参考波,uc09 是 taichi spike——均不可承载;G26~G29 device 化独立 probe 判档先例同律 |
| ④ | 最小 diff | 见 §3 与 PATCH_PROPOSAL:vk.rs 追加 4 组常量/3 结构/3 fn 指针 + `probe_async_queue_caps` + `create_timeline_semaphore` + harness 专用 device/提交器;graph 计划面 **0-byte**(FencePair 值域映射在执行器侧) |
| ⑤ | M59 重判两态判据建议 | 硬前置 = 单/双队列 digest 逐字节等价;go = GPU 帧时长中位改善 ≥3% 且 ≥0.15ms 且重叠率 ≥50%(噪声门:同臂双跑中位差 <1%);任一不满足 = 维持 no-go 补新鲜 evidence。最终阈值程序产(g31_budget 谱系),禁手写进硬门 |
| ⑥ | RFC 草案要点 | 五条,见 RFC_DRAFT §0:默认承诺面字面 0-byte 的 opt-in 加性臂 / 等价门硬前置 / 单 timeline+值域映射+成对 release-acquire / 扩写最小化清单 / M59 go 为落地前置 |

---

## 1. 侦察证据

### 1.1 断链点(#88):execute 与三产物零耦合

- 图编译四趟(`src/rurix-render/src/graph/compile.rs`):趟 4 `plan_lanes`(588-659 行)逐 `AsyncCompute` pass 求
  fence 弧——signal = 输入的最后图形生产者(RAW)与产出旧版本读写者(WAR/WAW)中的 max,wait = 产出的首个图形
  消费者与帧内覆写者中的 min;相同 `(signal, wait)` 弧经 `BTreeSet` 去重共享;`value` 按弧序 1 起单调。版本倒置
  (输入最后生产者晚于产出首个消费者)确定性拒(`AsyncDependencyCycle`,声明面不表达双缓冲)。
- `FencePair { signal_after: PassId, wait_before: PassId, value: u64 }`(types.rs:224-232),注释即写明
  「timeline semaphore 值」——类型面从 G5 起就是为 timeline 设计的。
- 趟 3 屏障推导按**有效车道**给 stage(compile.rs:374-412):`enable_async=true` 时异步 pass 的跨车道 RAW 屏障
  已产 `SyncStage::Graphics → SyncStage::Compute`(单测 `reference_frame_compiles_with_ao_fence`,compile.rs:961-1050
  逐字段锚)。屏障产物对双队列执行是**现成的**,缺的只是提交器。
- 断链本体:`execute()`(298-308)对 `self.passes` 线性 `for` 回放 `p.execute` 闭包进 `CmdRecorder`,产
  `CommandLog`(记录桩,graph.rs:127-148);`self.fences`、`p.barriers_before`、`p.queue()` 在 execute 路径
  **一次都没被读**。回落面已在:`CompileOptions { enable_async: false }` → 全 pass 降 Graphics、零 fence、屏障
  stage 单车道重推(compile.rs:1052-1074 单测锚)。

### 1.2 消费者盘点(谁消费 CompiledGraph)

| 消费点 | 形态 | 消费面 |
|---|---|---|
| `apps/uc09-taichi-spike/src/host.rs:123-159` | **唯一生产性 `execute()` 调用** | 单 pass copy 图(`enable_async=false`),数 `CommandKind::Copy` 意图 + 资源池化断言;与异步三件套无关 |
| `src/rurix-render/src/graph/graph.rs:342/372` | 单测 | 剔除后闭包不执行 / 双跑等值 |
| `apps/uc06-renderer`(pipeline.rs:118-122 持有;598-649、1337-1338;graph_setup.rs:332-369;main.rs:818) | **审计对拍,不 execute** | `fences().is_empty()` 非空断言、`passes().len()`/`barriers()` 计数进 receipt、`pool()` 峰值、`dump_json()`;pass 实际工作 = pipeline.rs 手驱 CPU 参考执行,device 腿(device_g75*/device_m19 等)手写 pass 列表走 `render_exec::execute_frame` |
| `apps/uc08-physics`(pipeline.rs:170,同构) | 同 uc06 | 同上 |
| 生产 Mega(`g31_window_present.rs` / `g14_3_lane_body.rs`) | **零消费** | 对 `rurix_render::graph` 只 import `types::ClusterRecord`(几何契约),CompiledGraph/FencePair 0 命中——TODO #88「与执行断链」逐字成立 |

### 1.3 vk.rs 队列建立/提交面(#57/#62)

- TIRT 并行上下文(vk.rs:1033-1241,feature `taichi-tirt`):`compute_qfi` = **首个**含 `QUEUE_COMPUTE_BIT` 的
  family(1154-1157),`graphics_qfi` = 首个含 GRAPHICS 位(1158-1161);同族则 device 只建一条 queue。NVIDIA 上
  family 0 即 graphics+compute 通用族 ⇒ 现 `compute_queue` 句柄**通常与 graphics 同族同 queue**(假异步形态)。
  compute 队列用途 = TIRT copy + `QueueWaitIdle`(TODO #57 表述成立)。
- 全仓 family 选择遍历(vk.rs 1422/2231/3638/5455/7525/8944/9383/11255/12258/14814/15876/18652/20199/22021/
  24155/26147):全部「首个含位」;18652 处取 `GRAPHICS|COMPUTE` 合位。**无一处区分「仅 compute、非 graphics」
  family,无任何 digest 等价门**——#62 现状逐字成立。
- device 创建 `p_next` 链:TIRT 路径为 null(1183-1193);主执行路径有各自 feature 链但**不含
  timelineSemaphore feature 开启**(见 1.4)。
- `PassSpec`(rurix-rt graph.rs:301-309 / rhi.rs:99-108)= `{ name, accesses, reflection }`,**无 queue 字段**
  ——RHI/语言面单 queue 全序(RXS-0239)与类型面一致。

### 1.4 timeline / 时间戳 capability 现状(#59)

- **探测面已在**(修正 TODO #59「rurix-rt 零 timeline_semaphore 命中」的过时表述;G31+ 波 C Task C3 后加):
  vk.rs:27305-27321 有 `ST_PHYSICAL_DEVICE_TIMELINE_SEMAPHORE_FEATURES = 1_000_207_000` +
  `PhysicalDeviceTimelineSemaphoreFeatures`(24B 锚,27836);27656-27785 feature 单链查询(扩展缺席恒 0,链式
  合法);`DeviceCapabilityReport.timeline_semaphore: bool`(27399-27400)进 `bin/vk_capability_report`
  聚合 JSON(非 stable,不进 canonical/golden,RXS-0351 L9 同律)。`synchronization2` 同链同报(27401-27402)。
- **创建/提交面为零**:全仓零 `SemaphoreTypeCreateInfo` / `TimelineSemaphoreSubmitInfo` / `vkWaitSemaphores` /
  `vkGetSemaphoreCounterValue`;device 创建从未在 `p_next` 挂 timeline feature(探测归探测,**探测结果未反哺
  device 创建**)。
- **GPU 时间戳基建为零**:vk.rs 零 `vkCreateQueryPool`/`vkCmdWriteTimestamp`/`vkGetQueryPoolResults` 命中
  (`timestamp_valid_bits` 仅是 `QueueFamilyProperties` 结构字段,查询后未消费);现 bench receipt 计时全部
  CPU 侧 wall-clock。重叠量 measured 需新增 FFI(§2.4 / PATCH_PROPOSAL §H)。

### 1.5 规范面:RXS-0239 与 RFC-0019 §4.8

- RXS-0239 条款体(`spec/render_graph.md`:158-192):Dynamic Semantics =「**单 queue;声明序 = 提交序 = pass
  粒度完成序**……每个 pass 边界是全序同步点」;严禁 UB 节明写「多 queue / async compute / split barrier 不在
  承诺面(§8),**其不存在性即由本条全序措辞封死——条款不为未来扩张预留弱化措辞**」(178-181)。§4 禁区留痕
  (330-333)重申其为 RFC-0013 全文批准对象(🔒 §4.D4)。⇒ 任何多队列执行**必须先走修订行**,禁静默扩面。
- 修订行程序先例:G9.2 RXS-0346 走「🔒 唯一显式修订行表」逐条落地 + 既有条款字面 0-byte 声明
  (render_graph.md:275-292 六行表;G4.3 PR-E「重排执行模型」段(187-192)为「字面不动 + 加性子节」先例)。
- RFC-0019 §4.8(`rfcs/0019-rendering-platform.md`:340-381)**语义已 Approved 未实施**:
  - §4.8.1 logical queue 闭集 Graphics/Compute/Transfer;pass 声明的是 capability requirement,物理 family
    进 execution evidence 不进语义 hash;
  - §4.8.2 五步 release/acquire 序(producer 写完 → release barrier → timeline signal 精确点 → consumer wait
    精确值 → acquire barrier 后才可访问);成对相等;wait 缺失/错值/双 owner/半对/值回退 = 提交前 validator RED;
    timeline 依赖图无环、同队列 signal 严格递增;
  - §4.8.3 单队列 fallback:无专用队列/无 timeline/gate 未开 → 显式 single-queue plan,**资源 EB 前后态、最终
    内容 digest、readback 与多队列计划一致**;evidence 标 `single_queue_fallback`,不充多队列绿,但它是
    portability correctness 硬门;
  - §4.8.4 G5 EB 三轴冻结面:ownership/timeline/queue mapping/release-acquire pairing 是 **companion plan
    metadata**,不是第四轴;EB 无法无损表达时必须停下先修 RFC。
  - §5 RP-MULTIQUEUE 行(410):materialize 目标 = `rhi.md` + `rendering_platform.md`;§6.1(431)M59 gate 归
    `g8.p0.m37.streaming_io` 的 `queue_mode=multi` 分支;§6.2(450)M59 RED/GREEN = 「wait/ownership pair 缺失、
    timeline cycle/value rollback」RED + 「dedicated transfer→graphics device 见证 + single-queue 相同 digest」GREEN。

### 1.6 uc06 AsyncCompute 标注(#60)

`apps/uc06-renderer/src/graph_setup.rs`:15 pass 线性序中 6 `gi_probe_trace` / 7 `rtao` / 8 `hard_shadow` 标
`QueueClass::AsyncCompute`(62-69 注释 + 195-200 `async_c` 构造),`ao_filter`(9)回图形;三条件(时长 ≥0.5ms /
无图形管线依赖 / 消费者距生产者足够远,types.rs:22-23 注释)声明式满足;`graph_setup` 自断言 fence 非空
(332-333)。uc08 同构。**标注是活的,进第二队列的路是死的**(§1.1/§1.2)。

### 1.7 M59 判档历史与重判锚

- `milestones/g8/G8_P2_DECISIONS.md`:32 — M59 no-go(G8.4 默认单队列;无 measured 收益证据);
- `milestones/g9/G9_P2_DECISIONS.md`:37 — no-go 维持(截至 G9.7 多队列 measured 收益证据零;RXS-0239 字面不动),
  **重判条件 = 多队列 measured 收益证据(D3-Q7)齐备时按只追加程序重判**;兜底 = 单 queue 全序维持;
- `milestones/g9/design/G9_D3_GPU_DRIVEN_SUBMISSION.md`:245 — D3-Q7 裁决行;`G9_CONTRACT.md`:211(RD-041)
  no-go 维持条款;`G8_CAPABILITY_MATRIX.md`:131 — M59 行(「语义须进 RFC-0019」已兑现,执行零)。

---

## 2. 设计:最小判档面

### 2.1 执行器消费 `CompiledGraph.fences` 的双队列提交路径(#57/#59)

**计划面 0-byte 原则**:graph/compile.rs、types.rs、plan_lanes 与 FencePair 一个字节不动;全部新逻辑在
**执行器/harness 侧**(判档窗)与 vk.rs 加性面(实施窗)。

1. **能力探测(硬前置,#62)**:`probe_async_queue_caps`(PATCH_PROPOSAL §D)返回
   `{ timeline_semaphore, compute_only_family: Option<u32>, distinct_compute_family: Option<u32> }`;
   - compute-only = `COMPUTE && !GRAPHICS`(真异步族);仅存在「与 graphics 不同但含 graphics 位」的第二族时
     如实登记 kind(共享族常假重叠,Khronos 口径);
   - `compute_only_family == None || !timeline_semaphore` → **显式单队列回落**:图按
     `enable_async=false` **重编译**(不是忽略 fences——趟 3 屏障 stage 随车道变,重编译才是干净的
     single-queue plan,对齐 RFC-0019 §4.8.3「planner 必须生成显式 single-queue plan」),receipt 记
     `fallback_reason`,evidence 标 `single_queue_fallback`。
2. **段切分(host 纯函数,骨架已试做)**:沿 `compiled.passes()` 线性序切**提交段**,切点 = ①车道翻转
   ②`signal_after` pass 之后 ③`wait_before` pass 之前。产物 `SubmissionSegment { queue, passes,
   wait_points, signal_points }`——见 `g31_async_lanes_probe.rs::plan_submission_segments`(纯 host、可单测、
   不触 GPU;执行器实施窗逐字消费该切分,禁二次推导,镜像 RXS-0240 纪律)。
3. **timeline 值域映射(执行器侧,确定性)**:一个 `FencePair.value = v` 隐含**两个方向**的跨队列依赖
   (graphics 生产 → 异步段;异步段 → graphics 消费),单值不够表达,映射为一条 timeline 上的两点:
   - graphics 段(含 `signal_after`)末 signal `2v-1`;
   - 异步段首 wait `2v-1`(timeline 允许 wait-before-signal,提交序不受约束),段末 signal `2v`;
   - graphics 段(自 `wait_before` 起)首 wait `2v`。
   弧序 v 单调 ⇒ 2v-1/2v 单调,同队列 signal 严格递增(RFC-0019 §4.8.2 判据);计划面 FencePair 0-byte。
4. **跨队列屏障折分(companion metadata,EB 三轴 0-byte)**:趟 3 产物中 `sync_before=Graphics` 的屏障不能整条
   录在 compute-only 队列(stage 须为队列所支持);执行器把跨车道屏障折成 release(生产队列段末,before 侧)+
   acquire(消费队列段首,after 侧),layout 前后态成对相等——即 RFC-0019 §4.8.2 的成对律,以 plan 侧
   validator 先行核验(半对/漏 wait/错值 = 提交前确定性 RED)。
   **判档窗简化**:资源建 `VK_SHARING_MODE_CONCURRENT`(规避 family ownership transfer,semaphore signal/wait
   自带全量 memory dependency,acquire 侧只补 layout transition),receipt 诚实登记 `sharing_mode=concurrent`;
   go 后实施窗再落 EXCLUSIVE + 真 release/acquire 对(RFC 草案修订行 3)。
5. **提交形态**:两条 `VkQueue`(graphics family + compute-only family 各一)、每队列独立 command pool/buffer;
   `TimelineSemaphoreSubmitInfo` 挂 `p_wait/p_signal_semaphore_values`;帧末 host `vkWaitSemaphores` 等 timeline
   终值(替代 QueueWaitIdle),CPU 可 `GetSemaphoreCounterValue` 做回收(#59 价值项,判档窗只登记不接)。

### 2.2 单/双队列 digest 等价门(#62,RFC-0019 §4.8.3 语义)

- 同图同输入三臂:`arm_single`(enable_async=false 重编译,现回放路径)/ `arm_dual`(双队列)/
  `arm_dual_rerun`(确定性重跑);
- 判据:全部输出资源 readback 的 sha256 **逐字节相等**(single vs dual)+ dual 双跑位级一致;
- 不等价 = RED,**整窗不判收益**(等价是硬前置,收益是 evidence);fallback 臂 evidence 标注不充多队列绿。
- 判档窗 workload:异步三 pass 用可确定性验证的 compute kernel 占位(如逐像素解析函数),先证调度正确性;
  真 AO/GI kernel 的接线属 go 后实施窗(#60 白名单接线)。

### 2.3 判档载体(#60):独立 harness bin

**决策:新独立 harness `g31_async_lanes_probe`(消费 CompiledGraph 的判档车道),不借 uc06、不动生产 Mega。**

| 候选 | 判定 | 理由 |
|---|---|---|
| 生产 Mega(g31/g14_3) | 否 | 不消费 CompiledGraph(#88 断链);先动生产车道 = 违反判档纪律与 W3 禁改清单 |
| uc06 车道 | 否 | host 参考波(pass 工作 = CPU 参考执行),device 腿手写 pass 列表走 render_exec;借它判档须先做 #88 合流,超窗 |
| uc09-taichi-spike | 否 | taichi interop spike,单 pass copy 图,`enable_async=false`,形状无关 |
| **独立 probe bin** | **是** | G26/G27/G28/G29 device 化全部独立 probe 判档后进车道(先例);图形状镜像 uc06 异步三 pass(gi_probe_trace/rtao/hard_shadow,#60 首批白名单),host 面消费 fences/barriers/queue 计划产物,device 腿 feature `vulkan` 后续接 |
| 候选白名单(首批) | — | uc06 已标三 pass:AO(rtao)/GI probe(gi_probe_trace)/硬阴影(hard_shadow);**禁列**:主光栅/阴影深度/indirect 准备(TODO #60 字面) |

骨架已落(本窗):建镜像图 → 双臂编译 → 消费 fences/queue/barriers 摘要 → 段切分参考实现 + timeline 点映射 →
JSON 摘要;device 腿全部 TODO 注释(见 bin 内 `TODO(#57)`/`TODO(#59)`/`TODO(#62)` 标记)。

### 2.4 重叠量 GPU 时间戳 measured 方案(evidence-only,不进硬门)

- 每队列段首/段末 `vkCmdWriteTimestamp`(query pool 每帧 reset;`timestampPeriod` 换算 ns;compute-only 族
  `timestamp_valid_bits` 须非零,探测面同批登记);
- `overlap_ms` = graphics 段区间 ∪ 与 compute 段区间的**交集时长**;`overlap_ratio` = 交集 / 异步段时长;
- 帧总量对比:同工作量 `async_on` vs `async_off`(单队列臂)GPU 帧时长中位(≥100 帧,丢 warm-up);
- CPU wall-clock 交叉校验(两队列提交窗时距);噪声地板 = 同臂双跑中位差;
- 写 evidence(receipt JSON:`overlap_ms`/`overlap_ratio`/`frame_ms_median_on/off`/`noise_floor`),
  **不进硬门**(TODO §6-4 字面:「重叠量/流送收益一律 measured 写 evidence,不进硬门冒充」)。

### 2.5 M59 重判两态与判据数字建议

**两态程序**(G9_P2_DECISIONS M59 行「只追加重判」口径):

- **go 留档**:等价门恒绿(硬前置)且收益判据达标 → 按只追加程序落 go 行,开 #59 timeline 提交器 +
  #60 白名单接线实施窗,RFC 修订行走正式登记(RFC_DRAFT §6 程序);
- **维持 no-go**:任一不达标 → no-go 行补**新鲜 measured 证据**(D3-Q7 数据从「证据零」变「证据在案,低于
  阈值」,判据字面可复审),harness 与 RFC 草案留档不废。

**判据数字建议**(供主 agent 裁决;最终阈值以程序产 budget 为准,禁手写进硬门):

| 判据 | 建议值 | 依据 |
|---|---|---|
| 正确性(硬前置) | single/dual digest 逐字节相等 100%;dual 双跑位级一致 | RFC-0019 §4.8.3/§6.2 M59 GREEN 字面 |
| 噪声门(测量有效性) | 同臂双跑 GPU 帧时长中位差 < 1% | 低于此测不出 3% 档收益;不满足 = 测量无效不判 |
| 收益(go 线) | GPU 帧时长中位改善 **≥3% 且 ≥0.15ms**,且 `overlap_ratio ≥50%` | 3% ≈ 业界 async compute 保守下界(典型报告 5~15%,取下沿防噪声假 go);0.15ms 绝对下限防高帧率百分比虚高;重叠率 50% 防「有 fence 无重叠」的机制绿冒充收益 |
| 样本 | ≥3 场景臂(bistro-interior tier100 ± 组合臂)× ≥100 帧中位,丢 warm-up | 现 bench receipt 帧数惯例 |
| 候选 pass 门槛 | 异步段时长 ≥0.5ms | 报告5 §2.4 三条件之一(types.rs:22-23 已冻结注释) |

### 2.6 与 #88/#89 的合流关系

判档窗独立 harness 自证;**go 后**收益要落到窗口,合流顺序 = #88(CompiledGraph 驱动生产执行器,P0′ 合流前置)
→ #57 双队列进 Mega;#89(FIF 进 present)正交并行。本窗对 #88 只登记不实施。

---

## 3. 最小 diff 提案(文件/函数清单;patch 文本见 PATCH_PROPOSAL)

| # | 落点 | 内容 | 性质 |
|---|---|---|---|
| 1 | `src/rurix-rt/src/vk.rs` 常量区 | `ST_SEMAPHORE_TYPE_CREATE_INFO=1_000_207_002` / `ST_TIMELINE_SEMAPHORE_SUBMIT_INFO=1_000_207_003` / `ST_SEMAPHORE_WAIT_INFO=1_000_207_004` / `SEMAPHORE_TYPE_TIMELINE=1`(+ sType assert 锚,27820 段先例) | 加性,实施窗 |
| 2 | 同上结构区 | `SemaphoreTypeCreateInfo`(32B)/ `TimelineSemaphoreSubmitInfo`(48B)/ `SemaphoreWaitInfo`(40B)+ size/align 锚 | 加性 |
| 3 | 同上 fn 指针区 | `FnWaitSemaphores` / `FnGetSemaphoreCounterValue`(1.2 core,device 级取址) | 加性 |
| 4 | vk.rs 新 pub fn | `probe_async_queue_caps(gipa) -> AsyncQueueCaps`(family 全枚举 + feature 链复用 §1.4 现成逻辑) | 加性 |
| 5 | vk.rs 新 fn | `create_timeline_semaphore(device, initial=0)`(p_next 挂 SemaphoreTypeCreateInfo) | 加性 |
| 6 | vk.rs 新入口(harness 专用) | `create_async_lanes_device`:p_next 链挂 `PhysicalDeviceTimelineSemaphoreFeatures{1}`,两条 `DeviceQueueCreateInfo`(graphics + compute-only);既有入口 0-byte(镜像 TIRT 并行上下文先例 1033-1241) | 加性 |
| 7 | vk.rs 新 fn | `submit_async_lanes(segments, timeline)`:逐段录制/`TimelineSemaphoreSubmitInfo` 挂值/帧末 `vkWaitSemaphores` | 加性 |
| 8 | vk.rs 新 fn(evidence) | timestamp query pool 四件(`ST_QUERY_POOL_CREATE_INFO=11`/`QUERY_TYPE_TIMESTAMP=2`/WriteTimestamp/GetQueryPoolResults + `timestampPeriod`) | 加性 |
| 9 | `src/rurix-render/src/bin/g31_async_lanes_probe.rs` | 判档 harness(骨架本窗已落;device 腿实施窗补,feature `vulkan`) | 新 bin |
| 10 | 计划面 | `graph/compile.rs` / `types.rs` / uc06/uc08 / 生产 Mega | **0-byte** |

纪律注:本窗 vk.rs 本体禁改,§3-1~8 全部以 PATCH_PROPOSAL 文本交付;实施窗落地时新 unsafe 按 U26/U27 既有
审计边界折叠(graphics FFI 边界内,预期 0 新号,确有新边界才领)。

---

## 4. 风险与回退

| 风险 | 缓解 |
|---|---|
| 共享族假重叠(compute 句柄与 graphics 同族) | compute-only family 探测为硬前置;无则显式回落,不测不判 |
| 跨车道屏障 stage 不为 compute 队列支持 | §2.1-4 折分律(release/acquire 成对);plan validator 先于提交 RED |
| wait-before-signal 死锁疑虑 | timeline 语义本身允许(binary 才禁);帧末 host wait 终值兜底 + 值域单调锚单测 |
| WDDM/驱动提交粒度污染 timestamp | 中位数 + 噪声门(§2.5);CPU wall-clock 交叉校验;calibrated timestamps 留待后续 |
| RXS-0239 未修订前被误接进生产 | harness 独立 bin 不进 CI 门/不进生产车道;`enable_async` 默认开仅编译面(现状不变),执行面默认单队列直至 RFC 修订行正式登记 + M59 go |
| uc06 fence 非空断言被回落破坏 | 回落=harness 局部重编译,不动 uc06 的 CompileOptions 默认 |

## 5. 交付物清单与验证记录(本窗)

- `PLAN.md`(本文档:侦察证据 §1 + 设计 §2 + diff 清单 §3)
- `RFC_DRAFT_RXS0239_amendment.md`(修订行草案,不进 spec/rfcs/,正式登记留主 agent)
- `PATCH_PROPOSAL_vk_timeline.md`(vk.rs 最小 diff 提案文本,不合入)
- `src/rurix-render/src/bin/g31_async_lanes_probe.rs`(骨架;host 段切分参考实现 + 单测)

验证记录(2026-08-29,纯 host,零 GPU):

- `cargo check -p rurix-render --bin g31_async_lanes_probe` → 绿(dev,exit 0);
- `cargo test -p rurix-render --bin g31_async_lanes_probe` → **3/3 过**:fence 弧 golden
  `(0→5, v=1)/(0→6, v=2)` 实测吻合 §1.1 对 plan_lanes 的推演;五段切分 + timeline 点
  `(2v-1, 2v)` golden;off 臂零 fence 单段;
- bin 实跑 JSON 摘要:on 臂 8 pass / **19 屏障** / 2 fence / 5 段,off 臂 8 pass /
  **18 屏障** / 0 fence / 1 段——两臂屏障批不同(stage 随车道重推)实证 §2.1-1
  「回落 = 重编译,不是忽略 fence」。
