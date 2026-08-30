# W3 深水区 — 异步 compute 三件套判档件实施记录(TODO #88/#57/#59/#60)

> 日期:2026-08-30。性质:**判档件实施**(按同目录 [PLAN.md](PLAN.md) §2/§3 与
> [PATCH_PROPOSAL_vk_timeline.md](PATCH_PROPOSAL_vk_timeline.md) 十行表落地);
> GPU 真跑归主 agent(§5 命令)。纪律:零 GPU、零 release、零 target-night;
> 生产车道 / graph/compile.rs / CompiledGraph::execute / spec / rfcs / milestones /
> registry / ci 全部 0-byte。

---

## 1. 改动清单

| # | 文件 | 性质 |
|---|---|---|
| 1 | `src/rurix-rt/src/vk_g37_async_lanes.rs` | **新文件**(vk.rs 加性面本体,body-include,≈1600 行) |
| 2 | `src/rurix-rt/src/vk.rs` | **尾部追加 4 行**(注释 + `include!("vk_g37_async_lanes.rs")`;既有 27901 行 0 改写——PATCH_PROPOSAL「或镜像 vk_g31_*.rs body-include 先例独立文件」选项,同 `vk_g31_present/vk_g31_mesh_bench/vk_m50_rt_body/vk_g31_ser_body` 四个既有先例) |
| 3 | `src/rurix-render/src/bin/g31_async_lanes_probe.rs` | 骨架 → 三臂判档 harness 补全(骨架的图构建/段切分/3 金标准单测逐字保留;`#![forbid(unsafe_code)]` 维持) |
| 4 | `src/rurix-render/Cargo.toml` | `[[bin]] g31_async_lanes_probe` 显式注册 + `required-features = ["vulkan"]`(原为 Cargo 自动发现;加 rurix-rt 消费后须门控,default 构建零回归) |
| 5 | 本文件 | 实施记录 |

## 2. vk.rs 加性面(全部经 body-include;既有函数/结构/常量 0 改写)

### 2.1 新增(PATCH_PROPOSAL §A~§H 对照)

| PATCH 条目 | 落地 | 数量 |
|---|---|---|
| §A 常量 | `ST_SEMAPHORE_TYPE_CREATE_INFO=1_000_207_002` / `ST_TIMELINE_SEMAPHORE_SUBMIT_INFO=1_000_207_003` / `ST_SEMAPHORE_WAIT_INFO=1_000_207_004` / `SEMAPHORE_TYPE_TIMELINE=1` + `PIPELINE_STAGE_ALL_COMMANDS=0x10000`(wait dst stage,时间戳不被提前触发)/ `SHARING_MODE_CONCURRENT=1` / `QUERY_RESULT_WAIT_BIT=0x2` | 7 |
| §B 结构 | `SemaphoreTypeCreateInfo`(32B)/ `TimelineSemaphoreSubmitInfo`(48B)/ `SemaphoreWaitInfo`(40B),size/align/sType 锚单测 `g37_timeline_ffi_layout_anchors` | 3 |
| §C fn 指针 | `FnWaitSemaphores` / `FnGetSemaphoreCounterValue`(1.2 core,device 级取址) | 2 |
| §D 探测 | `pub fn probe_async_queue_caps() -> Result<AsyncQueueCaps>`:compute-only(COMPUTE 且非 GRAPHICS)/ distinct-compute(异族含 GRAPHICS,仅登记)/ timeline feature 单链(复用既有 `PhysicalDeviceTimelineSemaphoreFeatures`)/ 双族 `timestamp_valid_bits` / `timestampPeriod`@blob 720 / `dual_queue_eligible()` 判据(timeline ∧ compute-only ∧ api≥1.2) | 1 struct + 1 pub fn |
| §E device | `run_async_lanes` 内的 harness 专用创建路径:双队列臂 = 两条 `DeviceQueueCreateInfo`(graphics + compute-only)+ `DeviceCreateInfo.p_next` 挂 `PhysicalDeviceTimelineSemaphoreFeatures{1}`;单队列臂 = 一条队列 + p_next null(既有形态)。镜像 TIRT 并行上下文先例(vk.rs:1033-1241),**未新增平行入口之外的任何既有函数改写** | 含于 §G 入口 |
| §F 创建 | `create_timeline_semaphore(device, create_sem, initial)`(p_next 挂 SemaphoreTypeCreateInfo) | 1 fn |
| §G 提交器 | `pub fn run_async_lanes(&AsyncLanesPlan) -> Result<AsyncLanesReport>`:逐字消费 harness 段切分/合法化产物(计划类型 `AsyncLaneQueueKind/AsyncLaneDispatchSpec/AsyncLanePassSpec/AsyncLaneSubmitSegment/AsyncLanesPlan/AsyncLaneFrameSample/AsyncLanesReport`);每段一次 `vkQueueSubmit` 挂 `TimelineSemaphoreSubmitInfo`(值 = 帧 base + 合法化点,跨帧严格递增);帧末 host `vkWaitSemaphores` 终值(替代 QueueWaitIdle;单臂 QueueWaitIdle);段内逐 dispatch 全局 `MemoryBarrier`(compute→compute,兼同队列跨段序);提交前 fail-closed 结构核验(值回退/车道矛盾/越界拒,单测 `g37_run_plan_precheck_rejects`);帧循环 = 1 digest 帧 + warmup + frames;首帧/末帧双回读(竞态金丝雀);`vkGetSemaphoreCounterValue` 终值 evidence | 7 类型 + 1 pub fn |
| §H 时间戳 | 每队列独立 query pool(2×段数)、队列首段 `vkCmdResetQueryPool`、段首/段末 `vkCmdWriteTimestamp`(TOP/BOTTOM)、帧末 `vkGetQueryPoolResults`(64|WAIT)→ 逐帧 frame/busy/overlap ns;**query pool 四件 FFI 零重定义**(`VkQueryPool/QueryPoolCreateInfo/FnCreateQueryPool/FnCmdResetQueryPool/FnCmdWriteTimestamp/FnGetQueryPoolResults/QUERY_*` 复用 vk_g31_mesh_bench 既有定义——PLAN §1.4「时间戳基建为零」在 C16 mesh_bench 落地后已过时,本窗如实复用);`timestamp_valid_bits=0`/`period≤0` → `timestamps_valid=false` SKIP 不充 measured | 复用 + 消费逻辑 |

其余全部复用 vk.rs 既有面(句柄/结构/fn 指针/`pick_mem_type`/validation messenger/`load_vulkan_loader`/`cast_fn`),零重定义、零改写。**是否全加性:是**——vk.rs 本体 diff 仅尾部 include 块;`cargo check -p rurix-rt`(default)与既有全部入口行为 0 变化(default 构建不含 feature vulkan,连编译面都不触)。

### 2.2 unsafe 审计口径

对上全 safe(`probe_async_queue_caps` / `run_async_lanes` 无 unsafe 签名);内部 unsafe 按 U26/U27 既有 graphics/compute FFI 边界折叠(模块头 SAFETY 声明,vk_g31_mesh_bench 同型),**0 新 U 号**;显式 `unsafe {}` 块两处均携 `// SAFETY:`(clippy `undocumented_unsafe_blocks=deny` 下本文件零告警)。

## 3. probe bin 三臂(g31_async_lanes_probe)

- **臂①单队列基线**:`enable_async=false` **重编译**(显式 single-queue plan,RFC-0019 §4.8.3)→ 单段 → `run_async_lanes(dual_queue=false)`(一条 graphics 队列、零 timeline、QueueWaitIdle)。双跑(噪声地板)。
- **臂②双队列**:`enable_async=true`(默认)编译 → `plan_submission_segments`(骨架逐字保留)→ **`legalize_submission` 值域合法化 + 提交前 validator**(§4)→ `run_async_lanes(dual_queue=true)`(graphics + compute-only 双队列、单条 timeline、CONCURRENT sharing 诚实登记)。双跑(位级重跑一致)。
- **臂③等价门(硬前置)**:全部输出资源(10 个图资源 buffer 拼接)sha256 —— single↔dual 逐字节相等 ∧ dual 双跑位级一致 ∧ 每跑首帧/末帧一致(帧内竞态金丝雀)∧ single 双跑一致;另附 CPU 参照 digest 交叉核(`kernel_ref` 与 SPIR-V 逐运算同式,纯 u32 wrapping ⇒ device/host 位级同值;evidence 登记不进硬门)。不等 = RED exit 1,整窗不判收益。
- **回落**:`dual_queue_eligible()` 不满足 → 只跑臂①,evidence `fallback: { single_queue_fallback: true, reason }`,judge 恒 no-go(新鲜 measured:能力缺失如实登记)。
- **workload**:uc06 异步三 pass 形状等价 compute 负载 —— 13 buffer(10 图资源 + 零 dummy + 2 shade 链稿)× 230400 u32;每 pass = 手编 SPIR-V 迭代 kernel(SPIR-V 1.0,`mesh_witness_fs_spv` 手编先例同法;3 SSBO + 12B push constant;LocalSize 256),dispatch 表镜像图读写声明(同步错误 ⇒ 读到未写值 ⇒ digest 必漂移,等价门物理有效)。`--scale N`:异步三 pass 各 `256×N` 迭代;`vsm_page_mark`(与异步段并行的图形 pass)`3×` 之,时长对齐异步段(重叠窗物理存在);轻 pass 64 迭代。
- **重叠率**:逐帧 `overlap_ns = Σ(gfx 段区间 ∩ compute 段区间)`,`overlap_ratio = overlap / async_busy`;`frame_ns = max(end)−min(begin)`(全段);wall-clock 副口径交叉。中位聚合,噪声 = 同臂双跑中位差。
- **--judge 两态**(PLAN §2.5 字面):硬前置 digest 等价;噪声门 <1%;异步段 ≥0.5ms(报告5 条件,不满足提示调 `--scale`);go = 中位改善 ≥3% ∧ ≥0.15ms ∧ 重叠率中位 ≥50%;任一不满足 → no-go + 新鲜 measured 全量在 evidence。verdict ∈ {go, no-go, red}。
- **三态纪律**:无 loader/无设备 → `skipped_dev_env` exit 0;`RURIX_REQUIRE_REAL=1` 翻硬红(g35 同律)。`RURIX_VK_VALIDATION=1` → ERROR 级校验翻 Err(messenger fail-closed,mesh_bench 同律)。
- **--selftest 纯 CPU(7 项)**:fence 弧 golden / 段切分 golden / 合法化 golden / 合法化 RED 四臂(漏 signal 半对、孤儿 signal、同点双签、签发非全序)/ 回落重编译判据(off 臂零 fence 单段 **且屏障批与 on 臂不同**——「回落必须重编译而非忽略 fence」的结构证据)/ kernel 参照确定性 / SPIR-V 流完整性。`#[cfg(test)]` 同判据 7 测双承载。

## 4. 本窗 measured 发现:`(2v-1, 2v)` 逐弧点映射在共享生产者弧形下不可直接提交(→ 值域合法化层)

PLAN §2.1-3 的逐弧两点映射假设「弧序单调 ⇒ 点序 = 执行序」。判档形状恰好证伪:两弧
`(0→5,v=1)/(0→6,v=2)` **signal_after 同为 pass 0**,段切分产 seg0 signal `{1,3}`、异步段
signal `{2,4}`。Vulkan timeline 语义 = ①wait 于 counter **≥ 值**即满足;②signal 值须
**全局(跨队列)严格递增**。直接提交则:seg0 签 3 后异步段签 2 = **值回退(非法)**,且
counter=3 会提前解锁 `wait(2)`(ao_filter 在 rtao 完成前放行 = 等价门必炸)。——这正是
RFC_DRAFT 修订行 3「错值/值回退 = 提交前 validator 确定性 RED」预设要拦的形态,骨架窗
把 validator 留给了实施窗,本窗补齐:

- **`legalize_submission`(bin,host 纯函数)**:配对核验(每 wait 有唯一签发段、每
  signal 有等待者、同点双签拒 = 半对/漏 wait/漏 signal RED)→ 签发段 happens-before
  全序核验(弧边 + 同队列提交序可达闭包;非全序 = 单 timeline 值域不可表达,拒)→
  链序赋值 1..n(段级信号事件归并;wait 取生产段值 max——≥ 语义 + 生产者全序 ⇒ max
  蕴含全部)→ 线性序尾段挂 frame-end 信号 n+1(host 帧末等待锚;尾段须可达自全部签发段)。
- 判档形状合法化 golden:seg0 签 1;异步段等 1 签 2;ao_filter 等 2;shade+blit 等 2 +
  frame-end 签 3;span=3(selftest/`legalize_golden` 钉死)。
- **原 (2v-1,2v) 点保留在 plan evidence(语义层弧标识)**,合法化值为提交层;两层映射
  同进 receipt(`plan.arm_async_on.segments[].wait/signal` vs
  `plan.arm_async_on_legalized`)。FencePair 与 plan_lanes **0-byte**(PLAN 计划面纪律
  维持);vk 侧 `run_async_lanes` 另有提交前单调性 fail-closed 复核(防绕过 bin
  validator 的调用者)。
- **登记**:RFC_DRAFT 修订行 3 的「`(2v-1,2v)` 确定性映射」措辞在正式登记前应改为
  「弧点 → 签发事件链序值的确定性合法化(值回退不可构造)」或等价表述;草案文本本窗
  不动(rfcs/ 禁改),留主 agent 定稿时并入。

## 5. 验证记录(2026-08-30,dev profile,纯 host,零 GPU)

| 项 | 结果 |
|---|---|
| `cargo check -p rurix-rt`(default) | 绿(exit 0;default 不含 vulkan,加性面零编译影响) |
| `cargo check -p rurix-rt --features vulkan` | 绿(本窗文件零警告;存量警告在 vk_m50_rt_body/vk_g31_ser_body/vk.rs 22xxx 段,未触) |
| `cargo check -p rurix-render`(default) | 绿(exit 0;bin 经 required-features 门控不入 default) |
| `cargo check -p rurix-render --features vulkan --bin g31_async_lanes_probe` | 绿(bin 零警告) |
| `cargo test -p rurix-rt --features vulkan --lib g37` | **2/2 过**(FFI 布局/sType 锚 + 提交前核验拒);**登记**:rurix-rt 整 lib 测试面(207 测)未跑——按任务许可只跑新增模块;相邻面(vk 其余测试)未触改动 |
| `cargo test -p rurix-render --features vulkan --bin g31_async_lanes_probe` | **7/7 过**(骨架 3 金标准逐字保留 + 合法化 golden/RED 臂/回落重编译/kernel+SPIR-V 自检 4 新增) |
| `g31_async_lanes_probe --selftest` | **7/7 PASS,exit 0**(纯 CPU) |
| clippy(补充) | 本窗两文件在 `cargo clippy --features vulkan` 下零告警;rurix-rt/rurix-render 在该 feature 组合下有**存量** clippy 错误(vk.rs:22057/22123-22126/22155、vk_m50_rt_body.rs:65/85 缺 SAFETY 注释;rurix-render lib 1 错 100 警)——非本窗引入,未触(禁区外亦不顺手改,W3 纪律) |

## 6. 主 agent GPU 判档命令(本机 RTX 4070 Ti,PowerShell;dev profile,禁 release)

```powershell
# ① 校验层冒烟(小规模;ERROR 级即翻红,fail-closed)
$env:RURIX_VK_VALIDATION = "1"
cargo run -p rurix-render --features vulkan --bin g31_async_lanes_probe -- --frames 10 --warmup 3 --scale 4
Remove-Item Env:RURIX_VK_VALIDATION

# ② 判档跑(--judge 两态;validation 关闭防计时污染)
cargo run -p rurix-render --features vulkan --bin g31_async_lanes_probe -- --judge --frames 120 --warmup 20 --scale 8 --out artifacts/day_0830_delivery/w3_deep/async/evidence_async_lanes.json
```

- 判读:`judge.verdict` = go / no-go(digest 破缺 = red + exit 1);`measured.*` 为新鲜
  measured(no-go 时按 G9_P2 M59 行补进 no-go 证据)。
- 调参:`measured.async_busy_ms_median < 0.5` → 提高 `--scale`(异步段迭代 = 256×scale,
  时长近线性)后重跑②;噪声门 ≥1% → 关后台负载/提高 `--frames`。
- 预期形态:双队列臂 `arms.dual[].overlap_ms_median > 0`(vsm_page_mark 与异步三 pass
  并行窗);单队列臂同工作量全序执行。两臂 digest 逐字节相等为硬前置。

## 7. 已知风险与边界(诚实登记)

| # | 风险 | 状态 |
|---|---|---|
| 1 | 跨队列 timestamp 时基:Vulkan 未承诺双队列 tick 域对齐(VK_KHR_calibrated_timestamps 未接);同物理设备实践一致 | evidence 口径注记;PLAN §2.4 既有登记,中位 + 噪声门 + wall 交叉兜底 |
| 2 | CONCURRENT sharing 简化臂(非 EXCLUSIVE release/acquire 成对) | receipt `sharing_mode` 如实登记;go 后实施窗落成对律(RFC_DRAFT 修订行 3) |
| 3 | WDDM 提交粒度/后台负载污染计时 | 噪声门 <1% 不满足即「测量无效不判」;帧循环全同步(无 FIF)测的是单帧内重叠收益下界 |
| 4 | workload 为合成等价负载(整数迭代 kernel),非真 AO/GI kernel | PLAN §2.2 判档窗字面(先证调度正确性);量级经 --scale 满足 ≥0.5ms 条件;真 kernel 接线 = go 后 #60 白名单实施窗 |
| 5 | `(2v-1,2v)` 直提交不可行(§4) | 已由合法化层 + 双侧 validator 封死;RFC 草案措辞修订留主 agent 定稿 |
| 6 | 交错弧形(弧窗内异步 run 被图形 pass 割裂)未一般化 | 骨架既有边界注记维持;签发非全序时合法化确定性拒(不静默),一般化留 go 后实施窗 |
| 7 | 帧循环逐帧全同步,不测跨帧流水收益 | 判档口径 = 帧内重叠(PLAN §2.4 帧时长中位对比);跨帧 FIF 属 #89 正交 |
| 8 | rurix-rt 整 lib 测试未全跑(207 测滤除) | §5 登记;触改面(vk 模块加性)有专属锚测 + bin 侧 7 测覆盖消费面 |
