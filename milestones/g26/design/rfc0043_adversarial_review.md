<!-- Assisted-by: Cursor Claude Fable 5（D-409 独立评审会话，与起草会话隔离） -->
# RFC-0043 对抗性评审报告（D-409 对抗性评审要求程序）

| 字段 | 值 |
|---|---|
| 评审对象 | `rfcs/0043-framegen-device-kernel-realization.md`（v0.1 Draft，2026-08-25，G26.1 治理波起草） |
| 评审日期 | 2026-08-25 |
| 评审轮次 | D-409 第 1 轮 |
| 评审 provenance | **独立评审会话零共享上下文**（与起草会话隔离，评审自行逐锚重读全部事实源）。同环境单一模型 provenance 偏差不静默处理，如实登记且效力自限——本评审无法派生跨工具/跨模型独立实例，偏差沿 RFC-0015 §9.1 / number_ledger v1.73 / v1.90 先例登记，留战役终审窗复核锚 |
| 结论速览 | **11 findings（0 blocker / 6 major / 5 minor）——建议 Agent Approved，条件 = 逐条 disposition 落档（v0.2 修法批）** |

## 0. 事实源逐锚核对结果（评审程序第 2 步）

全部引用锚实测存在且字面一致，无虚引：

| 锚 | 核对结果 |
|---|---|
| `src/rurix-render/src/temporal/framegen.rs` | ✅ `interpolate` 公式（双向 warp / `w_cons = (-d2 * inv_sigma2).exp()` / lin+near 混合）、`mfg_between` t_i = i/(n+1)、`assert!(t > 0.0 && t < 1.0)`、`FgAccounting` 两口径与 RFC §1.1/§1.2/§1.6 一致（细部差异见 F1） |
| `src/rurix-render/src/bin/g13_tsr_device.rs` | ✅ 标定腿 `--calibrate maxdiff` protocol 字面 "threshold = measured × 2.0 冻结 k"、`kernel-bias`/`seed-change` 双 RED 臂、三态（SKIP DEV_ENV_DEGRADE 退 0 / host 恒跑 / FAIL 退 1）、bin-local adapter 经 `vk::run_compute`——RFC §1.4/§1.5/§1.7 引用属实（差异见 F3/F10） |
| `src/rurix-render/kernels/g18_light_transport_depth.rx` 头注 | ✅ "确定性协议（RXS-0357 L2 同律继承）：逐像素独立顺序求值，禁 atomic；输出直写无跨像素交互；固定输入双跑位级一致。全 f32；分支判定一律 min/max 算术门" 与 RFC §1.3 字面一致 |
| `registry/deferred.json` RD-045 | ✅ status=open；backfill_condition 三件字面（含候选面四项枚举、"Full RFC 评估（触 RXS-0357 L2 确定性协议面）"）逐字一致；G19.3 history 行 disposition = maintain-open-with-extended-zero-recurrence 在案 |
| `milestones/g25/g25_campaign_handover_registry.json` | ✅ G13-N7（g26_anchor = "device kernel 车道重判（RFC-0036 §1.5）"）/ RD-045-window / G17-MD-F1（"非关闭性定论"）三行 + rd_eight RD-045 行字面一致 |
| `milestones/g19/g19_vendor_sdk_registry.json` | ✅ 存在，fsr3_fg=rejected / dlss_g=not_available / sl_310_6_0=not_available 三臂 disposition 在案（RFC §5.1 引用属实） |
| `evidence/g25_m_b_fps_parity_final_verdict_20260824T194143Z.json` | ✅ 存在；焦点格 ratio=0.856326323057633（RFC §4.1 "0.856326" 为树内先例截断写法，G25_P2_DECISIONS 同形）；"17/18 诚实红终判" / "非关闭性定论" 字面在案 |
| `rfcs/0036-frame-generation-realization.md` §1.5 | ✅ "device kernel 车道（本期 out-of-scope）……承接锚 = G25 全量终审窗重判或后续期"——G26 落于 "后续期"，承接命中成立；§1.2 SSIM 对照阈字面同源 |
| `rfcs/0032-d3d12-host-ngx-lane.md` v0.3 | ✅ "重判条件 = G18+ 宿主差可分离 measured 证据出现（NGX 分解 profiling 或 UE 侧插桩）"——RFC §4.2 两半同源字面属实 |
| `registry/number_ledger.json` | ✅ RFC on_tree_max=43 / next_free=44，"立项时实测 next_free=43 顺位领取" 与台账一致；D-409 在案 |
| `milestones/g26/G26_CONTRACT.md` §4.2 / `G26_ACCEPTANCE_MAP.md` | ✅ 五门 key/波次与 RFC §6 一致（同构宣称的字面分歧见 F3/F11） |

## 1. 逐条 findings

### F1（major）——「公式面逐字同源」未钉双线性采样器语义字面，而这正是 device 化最大分歧源

**指认字面**：RFC §1.1 "公式面与 host `temporal/framegen.rs` `interpolate` **逐字同源**：双线性采样 `a = prev.sample(uv − t·mv)`……`w = exp(−‖a−b‖²/σ²)`"。

**挑战**：RFC 只钉了混合公式，未钉承重的采样器内部语义。host `ImageF32::sample_bilinear`（`temporal/image.rs` L85-99）的实际语义为：① uv 约定纹素中心 (x+0.5)/w、`xf = u·w − 0.5`；② `x0 = xf.floor() as i32`——**权重 fx 用未 clamp 的 x0 计算**，四 tap 逐轴 `clamp(0, w−1)` 到边；③ Rust `f32 as i32` 越界**饱和**，而 SPIR-V `OpConvertFToS` 越界为**未定义值**——对抗性 mv 场（大位移）下 device 若照抄 floor→cast→clamp 顺序即踩 UB，若改为浮点域先 clamp 则算术序与 host 不同。此外 RFC 公式写除法 `/σ²`，host 实现为倒数预乘 `(-d2 * inv_sigma2)`——位级不同构，"逐字同源" 宣称在公式记法层即与 host 操作序有出入。

**建议 disposition（修法）**：§1.1 增列采样器语义字面闭集（uv 约定/−0.5 offset/floor 后未 clamp fx/逐轴 clamp-to-edge/**浮点域越界护栏先于 int 转换**——保证 device 无 UB 且结果落 host 饱和语义等价域），并声明 host 实现操作序（含 `inv_sigma2` 倒数预乘）为 normative、RFC 数学记法为非规范注记。host-device 残差走标定容差带不变。

### F2（major）——host 冻结面只圈 `framegen.rs`，承重的 `image.rs`/`ssim.rs` 未入冻结，g13 先例为目录级

**指认字面**：RFC §1.7 "`temporal/framegen.rs` 本期 0-byte（git-diff 机核）"；对照 g13 先例（`g13_tsr_device.rs` 头注）："`temporal/` 底座与 trait 签名面 0-byte（**目录级** git diff……机核）"。

**挑战**：对拍锚的采样语义在 `temporal/image.rs`（`sample_bilinear`），SSIM 判据在 `temporal/ssim.rs`。若本期有人"修正" image.rs 的 −0.5 偏移或 clamp 语义，host 臂输出漂移、冻结容差 k 失效、"逐字同源"锚静默漂移——而 §1.7 的 git-diff 机核抓不到。G26_CONTRACT front matter 同样只圈了 framegen.rs，双文件同窄。

**建议 disposition（修法）**：冻结面扩为 `temporal/` 目录级 0-byte（g13 同模），或至少 `framegen.rs + image.rs + ssim.rs` 三件闭集入 git-diff 机核。

### F3（major）——双判据面区分成立，但「device 自身双跑位级一致」未列入 §1.4 判据枚举/§6 摘要，与契约 M-a 行不齐

**指认字面**：RFC §1.4 判据面四条（对拍容差 / SSIM 对照 / kernel-bias RED 臂 / spirv-val）+ §6 M-a "判据面 §1（对拍 + SSIM 对照 + RED 臂 + spirv-val + 三态）"；对照 G26_CONTRACT §4.2 M-a 行显式含 "**+ device 双跑位级一致**"。

**挑战**：评审确认 RFC 把两个判据面写开了——§1.3 "固定输入双跑位级一致"（device 自身两跑，exp() 同驱动同输入确定）与 §1.4 容差带（host libm 与 GPU exp 的 ULP 差走 threshold 吸收），判据结构上经得起 exp ULP 挑战。但 device 双跑位级一致只存在于 §1.3 协议叙述，未作为 §1.4/§6 的一等硬判据枚举——g13 harness 中它是逐档硬 FAIL 项（`problems.push("双跑非位级一致")`），契约 M-a 行也已列。实现者按 §1.4 清单施工可能漏掉 harness 级双跑检查。另 "双跑位级一致" 的效力范围（同设备同驱动，不承诺跨 GPU/跨驱动）未限定，有被误读为跨设备承诺的口子。

**建议 disposition（修法）**：§1.4 增列 "device 双跑位级一致（同设备同驱动范围；逐档终帧 digest）" 为一等判据，§6 摘要同步；与契约 M-a 行对齐。

### F4（major）——threshold = measured × 2.0 的循环论证实在，RED 臂量化兜底机制成立但 RFC 未钉偏置幅度

**指认字面**：RFC §1.4 "标定腿程序产（threshold = measured × 2.0 冻结 k……）"、"kernel-bias RED 臂：device kernel 输出面加性偏置注入 → 对拍必超容差检出"。

**挑战**：标定与判定同一实现——系统性错误的 kernel 会把 measured 撑大，threshold = measured × 2.0 恒容纳自身，对拍门形同虚设。g13 先例的反循环机制是**量化的**：`red_arm_kernel_bias` 检出条件 `tampered_p100 > tol`，tampered ≈ honest + RED_BIAS(0.05)，tol = 2×measured ⇒ RED 臂通过隐含 **measured < RED_BIAS，即偏置幅度构成容差带的绝对上界**（measured ≥ 0.05 时 RED 臂必漏检 → 门 FAIL）。RFC-0043 写了 "同模" 但未钉偏置幅度——若实现取超大 bias（如 1e3），该绝对上界即虚化，循环论证复活。SSIM(interp) > SSIM(frame-hold) 对照（§1.4 第二条）作为独立于 host 的正确性地板已在，是第二重兜底。

**建议 disposition（修法）**：§1.4 明写量化耦合——"RED 偏置幅度冻结常量（g13 同模 0.05 量级，须满足 bias > 标定 measured × k 方能构成检出），RED 臂通过 ⇒ 容差带绝对上界 = 偏置幅度"；SSIM 对照地板维持。

### F5（major）——RD-045 决策树未成文封死「零漂移观察冒充根因定位」，且 ③ 件防冒充 guard 只圈 §3

**指认字面**：RFC §3.2 "漂移复现 → 如实登记 + flip-trace 诊断臂……取证"；§3.3 "三件全齐 → close；任一未齐 → maintain-open 只追加扩窗登记……**本 RFC §3 只定重判程序，不自动构成③「Full RFC 评估」件的兑现**"。

**挑战**：两个口子。①件（根因定位）：新鲜窗零漂移路径下,执行者可援引 G14.10 "候选根因面结构性消除" + 多窗零复现宣称 "根因已定位为被消除面"——而 deferred.json G14.10 history 行的标准恰是 "**根因未逐字定位**……不冒充 close"，RFC 未把这条标准成文（零漂移只支持 maintain-open 扩窗，不得计为①兑现；①须以漂移复现 flip-trace 取证或静态缺陷定位证据为要件）。③件：guard 字面只圈 "本 RFC §3"——对抗性读法可主张 RFC-0043 整体（Full RFC、§1.3 触 RXS-0357 L2 面）构成③兑现登记；虽三件合取下 ①② 未齐仍无法 close，但分项冒充登记的口子应封。

**建议 disposition（修法）**：§3.3 增两句——"新鲜窗零漂移仅支持 maintain-open 扩窗登记，不构成①兑现（①要件 = 复现取证定位或静态缺陷定位证据，沿 G14.10『根因未逐字定位不冒充』字面）"；③件 guard 由 "本 RFC §3" 扩为 "本 RFC 全文"。

### F6（major）——G17-MD-F1「树内闭集搜索」可被空搜索敷衍：搜索路径清单未要求为 evidence 必填

**指认字面**：RFC §4.2 "搜索面为树内闭集登记（禁开放式外采）"；§4.3 "均未命中 → 维持 17/18 诚实红 carry"。契约 M-d 行有 "（evidence/ 检索面登记）" 但同样未定粒度。

**挑战**：判 "均未命中" 的前提是搜索确实做了。现字面下，一次 token 式空搜索（如仅 grep evidence/ 目录零命中）即可合规触发 carry 分支。闭集本身（哪些目录/registry/文件族构成 NGX 分解 profiling 与 UE 侧插桩两半的搜索域）也未枚举出处。

**建议 disposition（修法）**：§4.2 增 "搜索路径清单为 M-d evidence 必填字段：闭集定义（枚举目录/registry/文件族 + 出处 = RFC-0032 v0.3 重判条件字面）+ 逐路径 hit/miss 判定 + 检索式字面登记"——使 "均未命中" 可证伪。

### F7（minor）——MFG 三档 t 端点与 host assert 一致性成立；device 侧 t 产生方式未钉

**指认字面**：RFC §1.2 "t_i = i/(n+1)（i = 1..=n）逐帧 dispatch——host `mfg_between` 同序，端点即真渲帧不派发"。

**核验**：n ∈ 1..=3 ⇒ t ∈ {1/2} ∪ {1/3, 2/3} ∪ {1/4, 1/2, 3/4}，全部严格落 (0,1)，host `assert!(t > 0.0 && t < 1.0)` 不可触发；"端点不派发" 与 host "端点即真渲帧本身" 同义——**挑战⑥判据面无洞**。残余小口：RFC 未规定 device 的 t 必须 = host `i as f32 / (n + 1) as f32` 位级计算并经 params 传入；若 kernel 内部重算或经 f64 中转，位差可破坏 "标定腿两跑位级一致" 前提。

**建议 disposition（修法一句）**：§1.2 注明 "t 由 host 侧 f32 同式计算、经 params 位级传入，kernel 不重算"。

### F8（minor）——out-of-scope 列表漏项：kernel 性能优化、多 GPU、外推式 FG、生产 mv 来源

**指认字面**：RFC §5 仅四项（vendor FG / presented 上屏 / 空间重用 / host 参考臂重写）。

**挑战**：① device kernel 性能优化（tiling/subgroup/共享内存）——§2.3 只说了不设通过线，未列 out-of-scope 承接锚；② 多 GPU / 跨设备位级一致——与 F3 范围限定联动，应显式 out-of-scope；③ 外推式帧生成（extrapolation，vendor FG 实际形态）——本 RFC 只覆盖插值；④ 生产 mv 场来源（本期合成输入，渲染器 mv 导出面不碰）。

**建议 disposition（修法）**：§5 增 3~4 行各附承接锚（性能优化归 G14 车道锚 / 多 GPU 归独立立项窗 / 外推归 vendor FG 重判锚同窗 / mv 来源归 presented 消费方车道）。

### F9（minor）——口径红线在 M-b 判据里可机核件不足：仅 presented_frames 恒等式一件

**指认字面**：RFC §2.2 "`presented_frames = real + generated` 恒等式核验"；§1.6 "FgAccounting 类型面分离 0-byte 不碰"。

**挑战**：「生成帧禁入真渲帧率」现靠类型面信任（FgAccounting 冻结）+ 单一恒等式。evidence 字段级机核可再加两件低成本断言：`real_render_fps × real_render_seconds ≈ real_frames`、`presented_fps × (real_render_seconds + generation_seconds) ≈ presented_frames` 重算恒等式，及跨档不变式（×2/×3/×4 三档 real_frames 计数恒同——生成帧数变化不得改变真渲帧计数）。类型面 + 冻结 + 恒等式已构成基本护栏，故 minor。

**建议 disposition（修法）**：M-b evidence schema 增上述重算恒等式与跨档不变式字段。

### F10（minor）——RED 臂单臂：缺 g13 seed-change 同模（digest 判别面防退化）

**指认字面**：RFC §1.4 仅 "kernel-bias RED 臂"；g13 先例为双臂（kernel-bias + seed-change "jitter 序列改 seed……终帧 digest 与诚实跑必异检出——确定性协议漂移检出面"）。

**挑战**：双跑位级一致判据若实现退化（digest 函数恒等/未覆盖全输出），双跑恒 pass 且静默——kernel-bias 臂测的是容差检出面，兜不住 digest 判别面。framegen 的 seed-change 同模低成本可得（t 或 mv 扰动 → 终帧 digest 必异）。

**建议 disposition（修法）**：§1.4 增 input-change RED 臂（g13 seed-change 同模）；或维持单臂并成文说明由对拍 p100 独立兜输出正确性、接受确定性判据退化面残余风险——建议前者。

### F11（minor）——「与 G26_CONTRACT §4.2 同构」宣称下的三处字面分歧

**指认字面与分歧**：① RFC §6 M-e "既有 closed 门**全链**零降级（链 verify-latest + **budget --strict 全量**，G25 M-c 同模）" vs 契约 M-e "G25 **受影响门** --verify-latest 全绿零降级"——范围语义不同（全链全量 vs 受影响门），close-out 时构成双重标准口子；② 契约 M-b 行含 "性能面 g14_3_pipeline_perf 0-byte 机核 vs g25-closed" 机核件，RFC §2 只有 "本面 0-byte 不碰" 语义句未列为判据件；③ §4.1 焦点格写 "bistro/t100/dlss_sr"（短形，G25_P2_DECISIONS 有先例）而 §3.2 用全称 "bistro-interior/t50/tsr_device"——同文混用，机核脚本 grep 字面时有失配风险。

**建议 disposition（修法）**：M-e 判据面以契约行为事实源对齐（或契约扩为全链并双向改）；§2 补列 g14_3_pipeline_perf 0-byte 机核件；焦点格统一全称 "bistro-interior/t100/dlss_sr"。

## 2. 总评

**findings 分布：11 条 = 0 blocker + 6 major（F1~F6）+ 5 minor（F7~F11）。**

事实源核对面（评审程序第 2 步）全绿：RFC-0043 引用的十一处锚全部实测存在且字面一致，无虚引、无字面漂移；引用先例（g13 标定腿/RED 臂/三态、g18 确定性协议、RD-045 三件、G17-MD-F1 重判条件、RFC-0036 §1.5 承接锚）均属实。对抗面上，八项指定挑战中两项（⑥ MFG t 端点、② 双判据面区分的结构本身）经核验成立无洞，其余六项各产出可修法的 finding；另有三条评审自查 finding（F2 冻结面范围、F10 RED 臂单臂、F11 同构分歧）。

全部 findings 均为可 disposition 的 major/minor，无未解 blocker，修法均为 RFC 文字面增补/对齐，不动架构：

> **建议 Agent Approved，条件 = F1~F11 逐条 disposition 落档（v0.2 修法批，修订记录只追加）**；其中 F1（采样器语义字面）、F2（冻结面扩围）、F4（RED 偏置幅度钉死）、F5（RD-045 两句封口）、F6（搜索清单必填）为实现波开工前必须落的字面，F3/F7~F11 可随同批修法落。翻 Agent Approved 由治理会话按 D-409 程序执行，本报告 provenance 偏差效力自限声明随档。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-25 | D-409 第 1 轮对抗性评审报告落档（11 findings：0 blocker / 6 major / 5 minor；建议 Agent Approved 条件 = 逐条 disposition 落档） |
