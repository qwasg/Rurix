<!-- Assisted-by: Cursor Agent（G26.1 治理波） -->
# RFC-0043 — G26 时域/帧生成 device 化——FG/MFG device kernel 兑现 + RD-045 backfill 重判程序 + G17-MD-F1 重判窗程序

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0043（立项时实测 `registry/number_ledger.json` namespaces.RFC next_free=43 顺位领取） |
| 状态 | **Agent Approved**（2026-08-25；D-409 对抗性评审 11 findings〔0 blocker/6 major/5 minor〕全部 disposition，v0.2 修法批落档——评审全文 `milestones/g26/design/rfc0043_adversarial_review.md`） |
| 判档 | Full RFC（FG/MFG device kernel 兑现为时域呈现层 device 登记面；渲染器库面零新语言语义条款，G5 先例） |
| 承接 | G26.2 M-a/M-b + G26.3 M-c/M-d + G26.4 M-e（G13-N7 device kernel 车道重判锚兑现；RFC-0036 §1.5 out-of-scope 承接命中） |
| 上游 | `milestones/g25/g25_campaign_handover_registry.json`（G13-N7 / RD-045 / G17-MD-F1 三行 = G26 法定输入）、RFC-0036（host 参考臂 + §1.5 车道锚）、`registry/deferred.json` RD-045 |

## 1. FG/MFG device kernel 兑现语义（M-a）

1. **kernel 面**：`kernels/g26_framegen.rx` 单 kernel——输入 prev/cur（各 3 通道）+ mv（2 通道，prev→cur uv 位移场）+ params；输出 3 通道中间帧。公式面与 host `temporal/framegen.rs` `interpolate` **逐字同源**：双线性采样 `a = prev.sample(uv − t·mv)`、`b = cur.sample(uv + (1−t)·mv)`；一致性权重 `w = exp(−‖a−b‖²/σ²)`（host 以 `inv_sigma2 = 1/σ²` 预乘同式，device 同预乘形）；遮挡感知混合 `lin = a·(1−t) + b·t`、`near = (t < 0.5 ? a : b)`（device 以 min/max 算术门形选臂，t=0.5 取 b 与 host `<` 语义一致）、`out = lin·w + near·(1−w)`。**双线性采样器语义钉死（F1 disposition）**：host `image.rs::sample_bilinear` 字面——`xf = u·w − 0.5`、`x0 = floor(xf)`、**`fx = xf − x0` 用未 clamp 的 x0**、取样坐标 `(x0, x0+1)` 各自 clamp 到 `[0, w−1]`；device 实现 clamp 一律在 **f32 域 min/max 后再转 usize**（禁负值→usize 转换未定义面；`g13_tsr_resample.rx` 同模），公式操作序逐字对齐。
2. **MFG 三档**：×2/×3/×4（inserted_per_pair 1..=3 闭集），`t_i = i/(n+1)`（i = 1..=n）逐帧 dispatch——host `mfg_between` 同序，端点即真渲帧不派发；**device 侧 t 由 host bin 按同式逐帧算得后经 params 传入（f32 位级同值，kernel 内不重算）（F7 disposition）**。
3. **确定性协议**（`kernels/g18_light_transport_depth.rx` 头注 RXS-0357 L2 同律继承）：禁 atomic、逐像素独立顺序求值、输出直写无跨像素交互、全 f32、分支判定 min/max 算术门白名单形、固定输入双跑位级一致。
4. **判据面**（程序产禁手写）：
   - device vs host 同输入逐帧对拍，逐帧逐像素最大绝对差 p100 ≤ 标定容差——标定腿程序产（threshold = measured × 2.0 冻结 k，标定腿两跑位级一致，禁手写 P-09；`g13_tsr_device` 标定腿同模），超容差静默即 RED；**量化兜底（F4 disposition）：RED 偏置幅 `RED_BIAS = 0.05`（g13 同值）构成标定容差绝对上界——断言 `tol < RED_BIAS × 0.5`，标定值超上界即门 FAIL（封死「标定与判定同实现」循环论证：容差带永远小到足以检出注入偏置）**；
   - `SSIM(interp_i, GT_i) > SSIM(frame_hold_i, GT_i)` 继承 G19（RFC-0036 §1.2 程序产对照；frame-hold = 复制最近真渲帧零成本基线）——device 臂输出同样必须严格优于帧保持；
   - **device 双跑位级一致（F3 disposition，判据枚举正列）**：同设备同驱动窗口内固定输入两跑输出 digest 位级相等（契约 M-a 行同字面；host↔device 差走上条容差带，两判据面分离）；
   - **kernel-bias RED 臂**：device kernel 输出面加性偏置注入 → 对拍必超容差检出（「超容差静默即 RED」的机器兑现，`g13_tsr_device` RED 臂同模）；
   - **seed-change RED 臂（F10 disposition）**：合成场景相位扰动（输入流改动等价面）→ 末帧 digest 必异（确定性协议漂移检出面，`g13_tsr_device` seed-change 同模）；
   - spirv-val：kernel 编译产物 SPIR-V 验证通过。
5. **三态协议**：无 Vulkan loader/设备 → device 腿 `SKIP DEV_ENV_DEGRADE`（退 0，非 fake pass，如实登记不冒充；evidence schema `skipped_dev_env` 合法态；`RURIX_REQUIRE_REAL=1` 下 SKIP→硬红由 smoke 脚本层裁决）；host 腿恒跑；判据不符 / RED 臂失效 ⇒ FAIL 退 1。
6. **口径红线（G13-N7 字面 0-byte）**：FG/MFG 生成帧**禁计入**真实渲染帧率与 upscale ratio；`FgAccounting` 类型面分离 0-byte 不碰；`presented_fps` 独立登记面与 `real_render_fps` 并列输出、永不混算。
7. **host 参考臂冻结（F2 disposition：目录级扩面）**：`src/rurix-render/src/temporal/` **整目录** vs `g25-closed` 0-byte（git-diff 机核——对拍承重面 `framegen.rs`/`image.rs`/`ssim.rs` 全部圈入，g13 目录级冻结先例同模）；device 臂为 bin-local adapter 加性面（`g13_tsr_device` 集成路径同模，经 `rurix_rt::vk::run_compute` 派发），不回写 host 模块、不接线任何生产车道。

## 2. device 帧时登记面（M-b）

1. **协议**：warmup + timed 逐帧墙钟（host Instant 墙钟 around 逐帧 device 全链路：打包 + dispatch + 回读同步；`g13_tsr_device` bench 腿同模），×2/×3/×4 三档逐档登记；measured_local 零 estimated 入 budget。
2. **账目口径（F9 disposition：双恒等式机核）**：帧时以 `FgAccounting` 两口径并列登记——生成帧时归 `generation_seconds`，真渲口径零污染；机核两件 = ①`presented_frames == real_frames + generated_frames` 恒等式重算相等 ②`real_render_fps` 以登记面数值（real_frames/real_render_seconds）f64 重算相等且与 generated 计数无关（generated 扰动不改其值）。
3. **语义边界**：回归守护语义，**不构成帧率对标通过线**（G6 无性能硬门纪律沿用；正式帧率对标锚定 G14 车道，性能面 `src/rurix-render/src/bin/g14_3_pipeline_perf.rs` + `src/rurix-rt/src/render_exec.rs` + `src/rurix-rt/src/vendor_upscale.rs` 三文件 vs `g25-closed` 0-byte git-diff 机核——F11 disposition 契约 M-b 行同字面）。

## 3. RD-045 backfill 重判程序（M-c）

1. **输入面**：`registry/deferred.json` RD-045（status=open）backfill_condition 三件字面——①根因定位（候选面：首进程冷启动态/异步拷贝竞争/未初始化读取/浮点归约序）②生产化缺陷修复 ③Full RFC 评估（触 RXS-0357 L2 确定性协议面）；累计观察镜像 = G19.3 观察窗 12/12 中锚零漂移 + G19~G24 六期 soak 全零失败零漂移（`g25_campaign_handover_registry.json` rd_eight 行）。
2. **新鲜观察窗真跑**：焦点车道 `bistro-interior/t50/tsr_device` canonical 口径双跑 digest 轨迹多轮（G19.3 12 轮窗同模：逐轮 receipt last_frame_digest 对冻结锚对拍，`RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1`）；漂移复现 → 如实登记 + flip-trace 诊断臂逐帧 digest 轨迹取证（backfill_condition 字面动作）。
3. **三件逐项机器盘点决策树**：三件全齐 → close（deferred history 只追加 close 登记）；任一未齐 → **maintain-open 只追加扩窗登记**（不冒充 close；disposition 沿 G19.3 maintain-open-with-extended-zero-recurrence 先例）。本 RFC §3 只定重判程序，不自动构成③「Full RFC 评估」件的兑现。**防冒充硬线（F5 disposition）：新鲜观察窗零漂移证据永不构成①「根因定位」件——①件唯一合法证据形态 = 候选面四项（首进程冷启动态/异步拷贝竞争/未初始化读取/浮点归约序）之一的逐字确证记录（复现路径 + 定位机理）；观察性证据（零复现/长窗零漂移）只进累计观察面，盘点脚本对①件的判定输入面禁止引用观察窗结果。**
4. **只追加纪律**：RD-045 id/title/reason/backfill_condition 四字段 0-byte，history 只尾追加（`deferred.json` 对 AI 只追加纪律字面）。

## 4. G17-MD-F1 重判窗程序（M-d）

1. **输入面**：G25 M-b 17/18 诚实红终判（焦点格 bistro/t100/dlss_sr ratio 0.856326，`evidence/g25_m_b_fps_parity_final_verdict_*.json`）——合法收官态、**非关闭性定论**。
2. **两半证据树内闭集搜索（F6 disposition：搜索路径清单为 evidence 必填）**：①NGX 分解 profiling（vendor 臂宿主差可分离 measured 证据）②UE 侧插桩（对标侧帧时分解 measured 证据）——RFC-0032 重判条件同源字面；搜索面为树内闭集登记（禁开放式外采），**M-d evidence 必须逐条登记实际检索的路径/模式清单（searched-paths manifest），清单为空或缺失即门 FAIL——「均未命中」结论只能建立在非空搜索清单之上**。
3. **决策树**：任一命中 → 重判程序启动（本期只登记启动事实与证据路径，重判执行与终审归 G30 战役终审窗）；均未命中 → 维持 17/18 诚实红 carry（G15 物理不可达兜底同源，重判锚字面不变只追加）。

## 5. out-of-scope（各附承接锚；F8 disposition 扩列）

1. **vendor FG（DLSS-G/FSR-FG）接入**：G19 vendor 三臂 disposition 终态在案（`milestones/g19/g19_vendor_sdk_registry.json`），重判锚沿该表字面。
2. **presented 链路上屏集成**：swapchain/present 集成面不碰，承接锚 = presented 消费方车道出现。
3. **空间重用**：归 G28 ReSTIR 窗（RD-040 分项 reeval_anchor）。
4. **host 参考臂重写**：`temporal/` 冻结面；device 对拍若暴露 host 语义缺陷，按只追加程序另立重判。
5. **kernel 性能优化**（超对拍需求面的调度/占用率/共享内存优化）：承接锚 = device 车道进生产集成窗时按 measured 瓶颈立项。
6. **多 GPU / 跨设备一致性**：承接锚 = 多设备环境出现；本期双跑位级判据限同设备同驱动窗口。
7. **外推 FG（extrapolation，零未来帧）**：承接锚 = 延迟敏感消费方出现；本期仅插值形态（prev/cur 双端已知）。
8. **生产 mv 来源接线**：本期 mv 为 harness 合成场；承接锚 = temporal MV 链生产集成窗。

## 6. 验收门映射

| P0 | 门 key | 波次 | 判据面 |
|---|---|---|---|
| M-a | `g26.p0.m_a.framegen_device_kernel` | G26.2 | §1（对拍 + SSIM 对照 + RED 臂 + spirv-val + 三态） |
| M-b | `g26.p0.m_b.framegen_device_bench_accounting` | G26.2 | §2（帧时登记 + FgAccounting 口径） |
| M-c | `g26.p0.m_c.rd045_backfill_rejudgment` | G26.3 | §3 决策树 |
| M-d | `g26.p0.m_d.g17_md_f1_rejudgment_window` | G26.3 | §4 决策树 |
| M-e | `g26.p0.m_e.closed_gate_no_regression` | G26.4 | G25 受影响门 `--verify-latest` 全绿零降级；禁 `--gate` 旧脚本；`g26_` 前缀不抢 latest（契约 M-e 行同字面，F11 disposition） |

与 G26_CONTRACT §4.2 同构（M-a/M-b evidence schema 含 `skipped_dev_env` 合法态）；implemented / maintain / defer 均为合法终态，**零冒充**。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v0.1 | 2026-08-25 | G26.1 起草（Draft）。 |
| v0.2 | 2026-08-25 | D-409 对抗评审修法批：F1 双线性采样器语义钉死（f32 域 clamp + 未 clamp floor 的 fx）/ F2 冻结面扩至 temporal/ 目录级 / F3 device 双跑位级判据正列 / F4 RED_BIAS=0.05 量化兜底断言 tol < RED_BIAS×0.5 / F5 RD-045 ①件防冒充硬线 / F6 M-d searched-paths manifest 必填 / F7 device t 传参钉死 / F8 out-of-scope 扩列 4 项 / F9 M-b 双恒等式机核 / F10 seed-change RED 臂 / F11 契约字面对齐三处；状态 Draft → **Agent Approved**（评审 `milestones/g26/design/rfc0043_adversarial_review.md`，11 findings 全 disposition）。 |
