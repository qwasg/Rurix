<!-- Assisted-by: Claude Fable 5（G14plus 波0 治理立项批 RFC 起草） -->
# RFC-0030 — G14plus 渲染管线结构性优化语义（G14.x 延续波伞形：mv GPU 化 temporal 底座演进 / 确定性协议缺陷修复〔RD-045〕/ ray query first-hit 语言内建 / TSR kernel 调度变体 / readback 内存型与 FIF 流水结构面 / 阴影与主可见性结构面条件条款 / digest 锚重收割程序）

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0030（4 位制，编号永不复用，10 §9.5；编号按 2026-08-22 实测 `registry/number_ledger.json` namespaces.RFC `next_free=30` 领取，非推测号；`reserved_in_flight[G14plus]` 登记由波0 治理批落） |
| 标题 | G14plus 渲染管线结构性优化语义（G14.8~G14.12 延续波伞形单章） |
| 档位 | **Full RFC**（① 触 temporal 底座历史接口面演进——G14_CONTRACT guardrail「UpscaleBackend/temporal 底座 0-byte 不接线：确需演进必须独立 Full RFC 显式修订行」字面；② 触 RXS-0357 L2 固定 seed 确定性协议面评估——RD-045 backfill_condition「Full RFC 评估」字面；③ 触 ray query 语言面内建扩展——设备子语言 API 加性语义面；④ 触 G14-N14 阴影结构面「触冻结面时独立 Full RFC」承接锚字面。MR 体例不承载新语义面 + 冻结面修订，判档争议向上取严，10 §3） |
| 状态 | **Agent Approved**——D-409 对抗性评审完成（findings 全部 disposition，§9.1）；主会话已核对契约 §8.8 立项记录 / MAP 附录 A M-h 行 / 本 RFC 三面一致（2026-08-22），翻 Agent Approved |
| 承接里程碑 | G14（G14.8~G14.12 延续波集——G14_CONTRACT §7 裁决 7「G15 前按只追加程序新建 G14.x 延续波」字面；验收面 = 契约 §4.2 M-c/M-d 判据字面复跑 + 附录 A M-h 收口门） |
| 关联条款 | 拟落 spec 条款号一律 **post-interlock actual-next-free allocation**（不预写推测号；候选落点见 §5：`spec/shader_stages.md` ray query 内建修订行 + `spec/global_illumination.md` RXS-0357 L2 附注行〔如 §4.2 评估结论触及〕） |
| 依据决策 | D-406 v2.0 · D-409 · P-09 · P-13 · 10 §3/§7/§9.5 · 用户 2026-08-22 授权字面（「一次性完成G14硬收尾，要求门禁严格全绿…本次任务可附加为G14plus作为文档记录…本次进程允许视为超越G类里程碑的超大项目纠正优化案，不需要考虑工作量，务必完成任务使项目达到预期」）+ 用户 2026-08-19 全期授权字面（G14_CONTRACT §7 裁决 2/裁决 7）· [G14_P2_DECISIONS](../milestones/g14/G14_P2_DECISIONS.md) §3 G14-N8~N14 行承接锚 + 表后事件登记（G14plus 立项条）· [`registry/deferred.json`](../registry/deferred.json) RD-045（backfill_condition 字面）· G14_CONTRACT §8.4 优化残留登记 / §8.5 结构性优化取证六条（a~f）/ §8.6 遥测裁决登记 · [G14PLUS_RECORD](../milestones/g14/G14PLUS_RECORD.md) §1 立项授权 |
| Provenance | `Assisted-by: Claude Fable 5（G14plus 波0 治理立项批起草）` |
| Agent 批准 | **已批准**（2026-08-22，主会话核对契约 §8.8 ↔ MAP 附录 A ↔ 本 RFC 三面一致后翻 Agent Approved；D-409 第 1 轮对抗性评审 findings 全部 disposition 落实修法批） |
| 对抗性评审 | **已完成**（D-409 第 1 轮，2026-08-22，评审轮次与起草轮次隔离；findings 逐条 disposition 见 §9.1；provenance 偏差如实登记：评审者与起草者同模型同会话族、独立性 = 评审轮次隔离 + 独立重读事实源，非跨工具/跨会话——与 RFC-0029 先例同族，效力自限声明见 §9.1 并留 G14.12 M-h 门与 closeout 终审复核锚；评审全文见 [rfc0030_adversarial_review.md](../milestones/g14/design/rfc0030_adversarial_review.md)） |

---

## 1. 摘要

本 RFC 冻结 G14plus（G14.8~G14.12 延续波集）「渲染管线结构性优化」的语义面——七个子面一份冻结，目标 = M-d 门 18 格帧率通过线 ×1.00 全部达标（用户 2026-08-22「门禁严格全绿」字面）且画质零降级（G13 锁定对拍 deficit 基线带内 + G14.3 车道锚带内）：

1. **mv GPU 化语义（temporal 底座演进显式修订行，G14-N11 承接锚兑现）**：运动矢量计算自 host 单源 `temporal::common::compute_camera_mv` 纯 CPU 双循环（1080p 实测 12.4~33.9ms）演进为同 session 第二 compute pass（`kernels/g14_mv.rx`）——**重投影公式单源不破**：GPU kernel 为 host 单源公式的机械转写（同式同序），host 版保留为对照 oracle；逆矩阵在 host 算（与 `Mat4::inverse` 同一实现）经帧参数上传，GPU 侧零求逆；`src/rurix-render/src/temporal/` 目录 0-byte 不动（M-c 门 `temporal_base_0byte` 机核维持绿——演进面 = bin 侧消费切换 + 新 kernel 文件，非 temporal 目录改写）。
2. **确定性协议缺陷修复语义（RD-045，RXS-0357 L2 评估面）**：bistro TSR 车道间歇单轮末帧 digest 漂移（~0.6%/run，根因候选 = 首进程冷启动态/异步拷贝竞争/未初始化读取/浮点归约序）的定位与修复。**评估结论（本 RFC 冻结）**：修复语义 = 使 RXS-0357 L2「固定 seed 两次运行位级一致」真正成立的实现缺陷修复——L2 协议字面 0-byte 不改；若根因定位揭示需浮点归约序语义变更，则按本 RFC §4.2 L3 显式修订行程序另落 spec 附注（post-interlock 领号），否则零 spec 改动。诊断臂 = flip-trace 扩展至 `g14_3_pipeline_perf` TSR 车道（逐帧 digest 轨迹，RD-045 backfill_condition 字面动作）。修复落地后 RD-045 维持 open（间歇缺陷长窗观察归 G15+，零检出≠根因闭环的过度承诺，P-09 同族）。
3. **ray query first-hit 语言内建（设备子语言加性扩展）**：rurixc 新增内建 `ray_query_initialize_first_hit`（签名同 `ray_query_initialize`，SPIR-V 发射 RayFlags = `OpaqueKHR|TerminateOnFirstHitKHR` = 0x5；既有 `ray_query_initialize` 恒 `OpaqueKHR` 面 0-byte）。语义边界：仅供**存在性消费**（`has_committed()` 布尔）——全 opaque 场景下 first-hit 与 closest-hit 的存在性等价，阴影臂切换后图像位级不变；`committed_t()`/`committed_primitive_index()` 在 first-hit query 上的返回值为**任意命中**（非最近），消费即语义错误——conformance reject 语料承载此边界。
4. **TSR kernel 调度变体（G13.3 kernel 演进授权行）**：新建 `kernels/g14_8_tsr_resample.rx` + `g14_8_tsr_resolve.rx` 变体——`#[numthreads(8,8,1)]` 2D 线程组替代原 `ThreadCtx<1>` 无 numthreads（LocalSize 1,1,1）形态；**数学面逐字不变**（每像素公式与 `g13_tsr_*.rx` 同式同序，逐像素独立无跨像素交互 → 调度重排位级不变，digest 机核）；原 `g13_tsr_resample.rx`/`g13_tsr_resolve.rx` 逐字节 0-byte 保留（G13 M-b 门消费面维持 + RD-045 归因链保护——「TSR 车道零触碰」对照臂可回溯）；M-c 门 SPV 消费路径切换 = 门脚本内部修订面（§8 只追加验收记录口径）。
5. **readback 内存型与 FIF 流水结构面（G14-N9/N10 承接锚兑现，联动登记面）**：`render_exec.rs` 内部实现面演进——① readback 按用途分路内存型选择器（Readback 路优选 `HOST_VISIBLE|HOST_COHERENT|HOST_CACHED`，缺型回退既有 + 登记；上传路维持 WC；G14.3 DLSS 同型修法先例 ~1.8GB/s 蓝本）；② submit/collect 分离 + per-slot cmd/params/descriptor/query + 输出双缓冲（FIF=2，加性 API `submit_persistent_frame`/`collect_persistent_frame`，既有 `execute_persistent_frame` 签名与行为 0-byte 保留）；③ 生产帧 `readback_subset=Some(vec![])` 关全帧回读 + 锚点帧机制。本面不触 UpscaleBackend trait 签名与 temporal 目录（render_exec 内部实现面，严格说无独立 RFC 义务；伞形内登记为 G14-N11「readback 内存型修复先行面」字面兑现证明 + 位级安全论证消费 G14.4 取证 e 条）。
6. **阴影与主可见性结构面（G14-N13/N14 承接锚条件条款——仅当 G14.10 后 M-d 复跑仍有未达格才实施）**：① bistro 光栅 G-buffer 主可见性 MVP（Vulkan VS+FS，G7.5b diff=0 生产可用先例；RXS-0171 冻结面不动；raster/RT 双车道 A/B 对拍 = M-c 位级/画质锚复核承载）；② 阴影结构替代形态闭集（候选：背光 keep 预判跳射线〔位级不变，非本节〕→ 保守光贡献剔除〔改图〕→ cornell 16 样本交错削减〔改图 + deficit 参照系重标定完整程序〕——实施波 measured 裁决按序启用）；③ **画质锚带复核程序为本节验收硬前置**（cornell t67 tsr converged deficit ≤ 0.010779849285388998 + G13 双对拍门复跑带内）——超带即本节改动回退（RED 字面），不存在「放宽带」分支（带值 = 首跑 ×2 程序产禁手写 + 2026-08-19「不降级画质」用户授权字面的机器化身）。
7. **digest 锚重收割程序（`g14_3_stage_a_digest_anchor.json` + M-c 双跑锚按渲染语义变更重收割）**：渲染语义变更（§4.1/§4.6 改图面）落地后，18 格锚按程序重收割（沿 G14.6 首建先例「M-d 复跑面程序收割禁手写」）；合法性三证 = ① 同格双收割位级同值（M-c `double_run_bitexact` 机核）② 画质带内（M-c 锚带 + G13 双门复跑 PASS）③ 出图人工复核登记（AI 读图结论字面入 evidence notes，附加登记面不充机器门）；收割前置门 = M-c 复跑绿一次 + RD-045 修复后复现测试零检出（锚不得锚定到漂移态）；旧锚由 git 历史承载不另存档；M-f/M-g 门复跑在新锚下语义退化为「当前码面自洽性守护」——如实登记不冒充仍是 Stage A 历史零漂移证明。

**横切纪律**：G14 帧率通过线 = Rurix 三轮 trimmed mean ≥ UE 同口径 ×1.00（M-d 判据字面 0-byte）；G14 不设绝对画质通过线（归 G15）；未达格如实登记不冒充；G5~G13 closed 判据与 76 门零降级；UE 源码只读；UpscaleBackend trait 签名 0-byte（vendor 驻留交接经加性默认方法路线，`upscale_ext` 先例同模）。

本 RFC 是 **G14plus 波0 governance-only** 交付物。Agent Approved 只表示语义评审通过，**不构成实现许可**——各波实施按 G14PLUS_RECORD §2 波序 + 契约 §8.x 只追加验收记录承载。§4 全部拟议语义批准前不构成契约。

## 2. 动机、范围与治理门

### 2.1 为什么需要 Full RFC

G14.4 M-d 门实测通过线 0/18 达标（cornell ratio 0.074~0.457 / bistro 0.019~0.060），结构性差距主因已由 G14.4~G14.7 取证闭环：RT 主射线车道 vs UE 光栅管线架构差 + host 面残余（readback WC 内存型 / fence 全串行 / mv 纯 CPU / TSR kernel LocalSize 1,1,1 调度灾难）。消除这些差距的改动面触及四处冻结/程序面：temporal 底座演进（guardrail 字面要求 Full RFC）、RXS-0357 L2 确定性协议评估（RD-045 backfill_condition 字面）、ray query 语言内建扩展（设备子语言语义面）、阴影结构面（G14-N14 承接锚字面）。判档向上取严为一份伞形 Full RFC（RFC-0028/0029 伞形先例；多个独立 RFC 产生多轮评审开销且条款联动面〔mv GPU 化依赖 readback 修复先行、阴影结构依赖画质锚程序〕在伞形内表达完整）。

### 2.2 双门互锁：RFC 批准不等于实现开工

| 门 | 允许动作 | 禁止动作 |
|---|---|---|
| 波0 governance-only（本波） | 起草/评审/批准 RFC；G14PLUS_RECORD 与 P2 表后事件登记；M-h 门 materialize（步骤 265 实测领取）；异己面 patch 存档清场 | 不改渲染语义 `src/` 面；不预跑重型 bench |
| G14.8+ 实施波 | 按 G14PLUS_RECORD §2 波序分批兑现 §4 条款；每波轻量验证（L0/L1 协议）+ §8.x 验收记录 | 跳过波序依赖（如 mv GPU 化先于 readback 修复）；以 RFC Approved 替代波退出验证 |

### 2.3 范围（in scope）

§4.1~§4.7 七个语义面；G14.8~G14.12 波序与验收面绑定见 G14PLUS_RECORD §2。

### 2.4 非范围（out of scope）

| 项 | 依据 |
|---|---|
| FG/MFG 帧生成 | G13-N7/G14 重评窗不立项维持（真实渲染帧率口径字面） |
| SER / RT pipeline 迁移 | M52 双窗未命中维持；调研实测阴影 first-hit 收益 ~1.0×，不值（G14plus 调研面登记） |
| 材质链扩展（透射/焦散/镜面 IBL） | 锚定 G15 维持（G11-N8/N9/G12-N10） |
| ReSTIR/MegaLights 完整实现 | 少灯场景（cornell 1 quad / bistro 4 point）Power/RIS 与剔除已够；`gi/restir.rs` host 参照为异己研究面零消费；确需时 G15+ 另判 |
| UE 源码/二进制 vendoring | RFC-0027 许可边界 |
| 绝对画质通过线 | G15 商用收口期承接 |
| UpscaleBackend trait 签名变更 | 0-byte guardrail 维持；vendor 驻留交接走加性默认方法 |

## 3. 术语

- **first-hit query**：`TerminateOnFirstHitKHR` 标志的 ray query——遍历在首个 committed 命中即终止；对全 opaque 几何,「存在命中」布尔与 closest-hit 等价，命中属性（t/primitive）为任意命中不保证最近。
- **FIF（frames in flight）**：submit 后不等当帧 fence、允许 CPU 录制下一帧与 GPU 执行当帧重叠的流水深度；FIF=2 = CPU 最多超前 1 帧。
- **锚点帧**：生产帧关全帧回读后，按固定间隔（或测量循环末）单次回读做 digest/画质守护的帧——digest 语义 = 同一状态机末态，与逐帧回读的末帧 digest 同值。
- **背光 keep 预判跳射线**：阴影采样循环内 `gate_d·gate_cs(·gate_cl)` 预判为 0 时跳过阴影 ray query 发射——被跳过的 vis 值被恒 0 的 keep 门乘掉，输出表达式逐字不变（严格位级不变）。
- **digest 锚重收割**：渲染语义合法变更后按程序从复跑收据重建 18 格 digest 锚（禁手写 P-09）；三证 = 双收割位级同值 + 画质带内 + 读图登记。
- **deficit 参照系连锁**：G13 超分对拍 t50/t67 deficit 的参照 = 本端 tier100 收敛帧——改 t100 图（如 cornell 样本削减）会动摇全部 deficit 度量参照系，须完整重标定程序。

## 4. 拟议语义（Draft）

### 4.1 mv GPU 化（temporal 底座演进显式修订行）

**L1（kernel 形态）**：新 `kernels/g14_mv.rx`，`#[numthreads(8,8,1)]`；输入 = depth SSBO（场景 session 资源 6 驻留直读）+ 帧参数（`inv_vp` 未抖逆矩阵 16f + `prev_vp` 16f + 尺寸——host 算逆，GPU 零求逆）；输出 = mv SSBO（2 f32/px，uv 偏移约定与 host 同）；数学 = `compute_camera_mv`（temporal/common.rs L193-227）逐字机械转写：uv 像素中心 → ndc(2u−1, 1−2v, z) → world = inv_vp·ndc → prev_uv ← prev_vp·world → mv = prev_uv − cur_uv；`|w|≤1e-8` 门经 min/max 算术门兑现与 host `continue`（留 0）一致；无原子、无跨像素交互、每像素独立。

**L2（单源纪律守恒）**：「禁私写重投影」纪律的守恒表述——重投影公式单源仍在 `temporal::common`（host 函数 0-byte 保留为对照 oracle），kernel 为其机械转写；`src/rurix-render/src/temporal/` 目录 git diff（vs G14.0 ref f4c8da0b）恒空（M-c `temporal_base_0byte` 机核字面维持）；bin 侧（g14_3_pipeline_perf）消费切换 = host 调用删除 + 矩阵经帧参数上传 + mv SSBO 直供 TSR/vendor 输入链。

**L3（数值面与验收）**：GPU float（FMA/除法）与 host 的 ULP 差 → mv 微差 → 时域重投影微差 → 图像微差（digest 必变，SSIM 预期 >0.9999）；对拍 = 锚点帧 GPU mv 回读 vs host `compute_camera_mv` 逐分量 max-abs 登记（容差带程序产）；图像面验收 = §4.7 锚重收割三证 + cornell t67 SSIM 锚带 + G13 双门复跑带内。**以 host mv 路径冒充 GPU 化（切换未生效静默）即 RED**。

### 4.2 确定性协议缺陷修复（RD-045）

**L1（诊断臂）**：flip-trace 扩展至 `g14_3_pipeline_perf`——env `RURIX_G14_FLIP_TRACE=<dir>` 时 bench 逐帧回读并计算 `frame_content_digest` 追加写 `<dir>/frame_digests.jsonl`（frame_index/digest），漂移帧自动 dump EXR（G12_5_BENCH_FLIP_TRACE 前例同模）；漂移定位分型 = 首帧漂（冷启/未初始化）/ 中途单帧漂（拷贝竞争/归约序）/ 漂后链式污染（进历史链）；diff 像素空间分布分型 = tile/行块聚簇（异步拷贝竞争）/ 全图散点（浮点归约序）/ 固定角落（未初始化读取）；海森堡效应对冲登记 = trace 模式零漂移而非 trace 模式漂移 → 时序敏感竞争强证据。

**L2（修复语义）**：RXS-0357 L2「固定 seed 两次运行位级一致」协议字面 0-byte——修复 = 使协议真正成立的实现缺陷修复（候选实施面：TSR 独立 session 消除〔§4.5 联动——第二 session 异步上传/回读竞争面移除〕/ 缓冲初始化置零 / 拷贝同步窗补齐）。

**L3（条件修订行）**：若根因定位揭示浮点归约序语义变更必要（当前无证据），按 post-interlock actual-next-free 领 RXS 附注条款显式修订；否则零 spec 改动。

**L4（验收与状态）**：修复后复现测试 N=20 快筛零检出（统计诚实：p≈1.9% 基础率下 N=20 置信 ~68%，为快筛非闭环证据）+ 全战役累计 M-c/M-d/soak 复跑 ≥150 轮零检出 → deferred.json history 只追加「缓解证据」登记；**RD-045 条目 status 维持 open**（根因即使定位，间歇型缺陷长窗观察归 G15+；closeout `RD_FINAL_OPEN_IDS` 8 条 open 面零改动）。**漂移检出未登记即 RED**（M-e/M-d 漂移监控面维持）。

### 4.3 readback 内存型与 FIF 流水结构面（联动登记）

**L1（内存型选择器）**：`render_exec.rs` `pick_mem_type` 演进为按用途分路：Readback 路 = 优选 `HOST_VISIBLE|HOST_COHERENT|HOST_CACHED`（次选 `VISIBLE|CACHED`+invalidate，末选 WC+登记缺陷字面）；Upload 路 = 维持 `VISIBLE|COHERENT`（WC 顺序写）；GpuOnly 路 = `DEVICE_LOCAL` 排除 HOST_VISIBLE。内存型不改数据内容——位级不变机核（DLSS 同型先例 vendor_upscale.rs L1811-1823 蓝本字面）。

**L2（FIF=2）**：加性 API `submit_persistent_frame`（至 vkQueueSubmit 止，含 slot-reuse bounded wait）+ `collect_persistent_frame`（当帧 wait + query + readback 后移）；per-slot cmd/params/descriptor/query/readback 双缓冲；既有 `execute_persistent_frame` = submit+collect 顺序调用等价形态 0-byte 保留（既有消费方零漂移）；数据依赖正确性 = 逐帧 digest 序列与 FIF=1 全等（500 帧压测机核）。

**L2a(FIF×动态,每槽 AS 副本 opt-in;G37 W3 #90 修订行)**:L2 的 per-slot 双缓冲枚举扩展到 AS 面——调用方在 session AS 表显式声明 `frame_slots` 份同构副本组,经加性平行入口(`submit_with_frame_update_slot_as`)逐帧把 `tlas_update`/`blas_refit` 与组内 AS 绑定轮换到 `base + slot` 表项(每表项独立 instance buffer/BLAS 顶点缓冲/BLAS/TLAS/scratch;每槽 AS 描述符集经 per-slot override set 既有基建)。写面按槽分离:host 实例写序于本槽 fence 等待之后,错槽更新/组外更新/跨槽绑定 = 提交前确定性 RED。缺省流水形态的 `tlas_update`/`blas_refit` 拒绝面与守卫 barrier 帧间全序**字面不动**;L2 确定性协议对本臂维持——固定轨迹逐帧 digest 序列与单槽顺序提交逐字节相等(`Rebuild`;判档机核 = `g31_fif_dyn_probe` 三臂等价门),`Refit` 非纯实测时按槽稳定判据显式降档登记。副本内存成本(AS 面 ×frame_slots)为 opt-in 显式代价,evidence 登记;GPU 帧间重叠仍不在承诺面。(加性修订行;2026-08-30)

**L3（生产帧关回读）**：测量循环内 `readback_subset=Some(vec![])`；TSR/vendor 输入直接消费 GPU 驻留数据（§4.5 并 session 后）；循环末锚点帧单次回读做 `last_frame_digest`/is_finite/EXR——digest 语义 = 同一状态机末态不变；生产口径 v2 机核（0<production≤full）维持。

### 4.4 阴影与主可见性结构面（条件条款——G14.10 后 M-d 复跑仍有未达格才实施）

**L1（光栅 G-buffer 主可见性，bistro 专属）**：`Pass::Raster`（render_exec 既有面）+ VS/FS（G7.5b 生产可用先例）；G-buffer = depth（未抖 vp ZO NDC，与 kernel ④ 同式）+ 命中三角索引（或 normal/albedo/emission 展开）；阴影/着色 pass 消费 G-buffer；jitter 经投影矩阵注入；cornell 不启用（36 三角主射线占比 <6% 不值改图风险）。图像面 = 光栅化插值 vs ray cast 亚像素边缘差（SSIM 预期 >0.999），锚重收割 + G13 双门复跑带内验收；**超带即回退本面**。

**L2（阴影结构阶梯，按序启用禁跳级）**：① 背光 keep 预判跳射线（位级不变，恒启用不属本节）→ ② first-hit 切换（§4.6 内建，位级不变，恒启用）→ ③ 保守光贡献剔除（tile 级 `max_contrib=I/d²_min` 低于程序产阈剔除；f32 位级变/视觉不可感级；锚重收割 + 带内验收）→ ④ cornell 16 样本交错削减（16→8→4 阶梯实验：逐帧确定性旋转采样防条带 + 每档 SSIM/FLIP/读图三通道 + **deficit 参照系连锁重标定完整程序**——t100 是 t50/t67 deficit 参照帧，本级为最后手段）。每级启用前置 = 上一级实测后该格差距 >3%。

**L3（画质锚带复核硬前置）**：本节任何改图级启用后必跑——cornell t67 tsr converged deficit ≤ 0.010779849285388998（M-c 锚带机核）+ G13 超分/Lumen 双门复跑带内 + AI 读图 C1~C6 清单登记；超带处置树 = 调参重试 → 回退本级 → 极端时停线呈报用户裁决（agent 不得自行取舍「全绿」vs「不降级画质」两个用户授权字面）。

### 4.5 TSR kernel 调度变体与并 session

**L1（变体 kernel）**：`g14_8_tsr_resample.rx`/`g14_8_tsr_resolve.rx`——`#[numthreads(8,8,1)]` + `ThreadCtx<2>` + 越界门（px<ow ∧ py<oh），逐像素数学全式与 `g13_tsr_*.rx` 逐字同源（i = py·ow+px 索引换算）；`//@ spec: RXS-0404` 锚保留 + 变体身份注释；原 g13 kernel 逐字节 0-byte（G13 M-b 门消费面 + RD-045 归因对照臂）。位级验收 = 变体输出 digest == 原 kernel 输出 digest（同输入同 seed）。

**L2（并 session）**：TSR 资源并入场景 DeviceFrameSession（passes = [scene(, mv), resample, resolve]），resample 直读场景 out_color/out_depth 驻留 SSBO（消输入再上传）；历史 A/B parity 轮换 binding_overrides 既有机制沿用；`TsrDeviceBackend` 独立 session 删除——RD-045 候选根因面（第二 session 异步竞争）消除动作。dispatch 改 `[ceil(ow/8), ceil(oh/8), 1]`。

### 4.6 ray query first-hit 语言内建

**L1（语言面）**：新内建 `ray_query_initialize_first_hit(accel, origin, tmin, dir, tmax) -> RayQuery`——签名/类型/三态协议（RXS-0298/0299 初始化-遍历-消费）与 `ray_query_initialize` 全同；唯一差异 = SPIR-V `OpRayQueryInitializeKHR` 的 RayFlags 操作数 = `OpaqueKHR|TerminateOnFirstHitKHR`（0x5，既有恒 `OpaqueKHR`=0x1 面 0-byte）。rurixc 全链（resolve/typeck/hir/mir/mir_build/tbir/lower/vulkan_codegen + dxil 腿按既有 RT blocked 面同态处理）+ `ray_query_check.rs` 三态协议把新内建列入初始化集。

**L2（消费边界）**：first-hit query 的合法消费 = `proceed()` 空循环 + `has_committed()` 存在性布尔（opaque 场景与 closest-hit 等价 → 阴影臂切换图像位级不变）；`committed_t()`/`committed_primitive_index()` 返回任意命中——语义警示入 spec 条款字面，conformance reject/accept 语料各 ≥1 承载；主射线（需最近命中）**禁用**本内建。

**L3（spec 落点）**：`spec/shader_stages.md` ray query 条款修订行 + 新条款（post-interlock actual-next-free 领号）；conformance trace_matrix 锚同批。

### 4.7 digest 锚重收割程序

见 §1 摘要第 7 条（程序字面完整）；补充机核细节：收割脚本为一次性 `.tmp` 工具（不入 commit）或直接消费 M-c/M-d 复跑收据程序抓取；锚文件 `harvested_utc`/`source_gate_run`/码面 commit 字段更新；新旧 18 格对照表入读图台账与 §8.12 登记；M-h 门（附录 A）承载三证机核。

### 4.8 AS build flags（登记面，无冻结面触碰）

BLAS/TLAS `AccelBuildGeometryInfo.flags: 0 → PREFER_FAST_TRACE(|ALLOW_COMPACTION)`；相交语义不变（closest-hit 唯一命中集不变、阴影 has_committed 存在性序无关）——理论位级不变，实测 digest 复核；若实测漂移 >0.1% 像素或 SSIM<0.9999 则放弃本项（收益小于验证成本）。compaction = 创建期一次 query compacted size + copy + handle swap。本面无 RFC 义务，伞形内登记备案。

## 5. spec 映射计划

| 语义面 | spec 落点 | 条款号纪律 |
|---|---|---|
| §4.6 first-hit 内建 | `spec/shader_stages.md` ray query 节修订行 + 新条款 | post-interlock actual-next-free（禁推测号） |
| §4.2 L3 条件修订行 | `spec/global_illumination.md` RXS-0357 L2 附注（仅当根因触归约序语义） | 同上；未触发则零落点 |
| §4.1/§4.3/§4.4/§4.5/§4.7/§4.8 | 无 spec 落点（运行时实现面/程序面/kernel 变体，经契约 §8.x + G14PLUS_RECORD 承载） | — |

## 6. 边界声明

1. **G13 M-b 门零漂移**：原 g13_tsr kernel 0-byte + 该门自跑重编对拍——本 RFC 全部面不触其判据。
2. **M-f/M-g 语义退化如实登记**：锚重收割后两门复跑退化为当前码面自洽守护（§4.7 字面），§8.12 登记。
3. **异己面零消费**：`gi/restir.rs` 等异己研究面（已 patch 存档清场）零消费；G14plus 全部实现独立起草。
4. **UE 臂零改动**：M-b/M-d 的 UE benchmark harness 命令面闭集 0-byte；锁频 = 环境画像登记面（RXS-0380 L3 既有登记位），非命令面变更。

## 9. 治理记录

### 9.1 D-409 对抗性评审（第 1 轮，2026-08-22）

评审全文见 [milestones/g14/design/rfc0030_adversarial_review.md](../milestones/g14/design/rfc0030_adversarial_review.md)。findings 汇总与 disposition：

| # | 级别 | finding | disposition |
|---|---|---|---|
| F1 | high | 初稿 §4.1 未显式声明「temporal/ 目录 git diff 恒空」与 M-c `temporal_base_0byte` 机核的对应关系——实施者可能误改 temporal 目录 | **已修法**：§4.1 L2 增补机核字面与 diff 范围（vs f4c8da0b） |
| F2 | high | 初稿 §4.6 未界定 first-hit query 上 `committed_t()` 消费的语义错误面——静默误用返回任意命中即错图 | **已修法**：§4.6 L2 消费边界 + conformance reject 语料义务 + 主射线禁用字面 |
| F3 | med | 初稿 §4.2 L4 N=20 快筛未标注统计置信（p≈1.9% 下 68%）——可能被误读为修复闭环证据 | **已修法**：L4 统计诚实字面 + 累计 ≥150 轮口径 + RD-045 维持 open 裁决 |
| F4 | med | 初稿 §4.4 阴影阶梯未写「每级启用前置 = 上级实测差距 >3%」——存在跳级直上样本削减的执行风险 | **已修法**：L2 阶梯禁跳级字面 + t100 deficit 参照系连锁警示 |
| F5 | med | 初稿 §4.7 未写收割前置门（M-c 绿 + RD-045 零检出）——锚可能锚定到漂移态 | **已修法**：§1 第 7 条与 §4.7 前置门字面 |
| F6 | low | 初稿 §4.3 L2 未声明既有 `execute_persistent_frame` 0-byte 保留——既有消费方（G13 门等）回归面不明 | **已修法**：L2 等价形态保留字面 |
| F7 | low | 伞形范围偏大（七面）——单面回退时 RFC 状态语义不清 | **裁决登记**：各面条款独立、分波兑现；单面回退按 §4.4 L3 处置树登记不撤销 RFC（RFC-0029 七面先例同模） |

Provenance 偏差如实登记：评审者与起草者同模型同会话族，独立性 = 评审轮次隔离 + 逐条独立重读事实源（P2 表承接锚字面 / temporal 代码面 / RD-045 登记 / G14.4 取证条款）；效力自限——跨工具评审者可得时建议补一轮；留 G14.12 M-h 门与 closeout 终审复核锚。

### 9.2 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v0.1 | 2026-08-22 | 初稿（波0 治理批起草） |
| v0.2 | 2026-08-22 | D-409 第 1 轮修法批（F1~F7 disposition 落实） |
| v1.0 | 2026-08-22 | 主会话三面一致核对后翻 **Agent Approved** |
| v1.1 | 2026-08-30 | **§4.3 加性**:L2a 每槽 AS 副本 opt-in 子行正式登记(G37 W3 #90 判档双 PASS 前置已兑——`g31_fif_dyn_probe` 三臂等价门 Rebuild/Refit GPU 双 PASS,evidence = artifacts/day_0830_delivery/w3_deep/fif_dyn/ 双件 gates 全 true;既有 L2 字面与 `submit_with_frame_update` 拒绝面 0-byte,实现 = 加性 body-include `render_exec_g37_fif_dyn.rs` 平行入口 `submit_with_frame_update_slot_as`)。零既有语义改动。 |
