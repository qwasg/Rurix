<!-- Assisted-by: Cursor Agent（G30.1 治理波） -->
# RFC-0047 — G30 战役商用终审收官程序——尾锚重判闭集 + 三面商用终审 + 全链零降级 + 承接锚归档

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0047（立项时实测 `registry/number_ledger.json` namespaces.RFC next_free=47 顺位领取） |
| 状态 | Agent Approved（2026-08-25；D-409 对抗性评审 18 findings〔blocker 0/major 5/minor 13〕全处置为 v0.2——报告 `milestones/g30/design/rfc0047_adversarial_review.md`） |
| 判档 | Full RFC（五期串行战役收官终审程序留档；零新实现语义面——RFC-0042 收官先例同档） |
| 承接 | G30.2 M-a/M-b + G30.3 M-c/M-d + G30.4 M-e（五 P0；G26~G30「商用终审收官期」= 五期串行战役收官） |
| 上游 | `milestones/g25/g25_campaign_handover_registry.json`（七行尾锚 = G30 法定输入）、`registry/deferred.json` 八条（RD-034/039/040/041/042/043/044/045）、`milestones/g24/g24_legacy_rd_registry.json`（历史清册十一条引用源）、G26~G29 四期 P2 表（战役期期内条目锚）+ 六件尾锚原始锚经 g25 registry 镜像（原始行在 G23/G24 P2 表——F16） |

## 1. 尾锚重判闭集程序（M-a）

g25 handover 七行中六件尾锚的 G30 收官窗重判（第七件 G17-MD-F1 归 §2.2 性能面终判法定义务）——六件**全部机器取证**，manifest 必填；锚字面 = g25 registry `g26_anchor` 逐行原文：

1. **M125-adopt3**（Jolt 5.6 采纳档——F1）：需求证据三类树内检索——① 5.6 独有 API 引用 ② 5.3 缺陷命中 ③ A/B 超带（**逐类独立 manifest**）+ `sys56` 评估臂 cargo check 新鲜跑绿（评估臂可编译性 = 采纳评估的最小机器前提）+ `evidence/g9_m125_jolt_56_ab_evaluation_*` latest 只读盘点（禁 `--gate` 重跑——「在案三件条件 1/3」的 A/B 绿件锚，G23 M-a 检证面不缩面，F17）；三类全空 ⇒ 维持 **maintain-5.3**（在案三件条件 1/3 不变）。锚字面：「需求证据三类任一命中（5.6 独有 API 引用/5.3 缺陷命中/A/B 超带）」。
2. **M127**：corpus 四目录存在性实测 + PhysicsAsset residual 消费方 token 树内检索（搜索面闭集只追加扩面，禁缩面）；两半未命中 ⇒ 维持**研究子轨**。锚字面：「corpus + PhysicsAsset residual 消费方出现（搜索面闭集只追加扩面）」。
3. **M114-strand**：毛发资产入压测闭集检索（`K:/rurix-ext/assets` hair 资产面）——三态闭集：命中 / 未命中 / 检索根不可达 SKIP（外部盘缺失如实登记 + 在案态兜底，§1.4 三态律拉平——F15）；未命中或 SKIP ⇒ 维持 **card/mesh**。锚字面：「毛发资产入压测闭集」。
4. **M118-hdr-cal**：vulkaninfo HDR token 新鲜探针——`HDR10_ST2084`/`BT2020`/`HLG` 三 token；三态闭集 absent/present/SKIP（RFC-0046 §4.2 同律：SKIP = 工具缺如实登记 + 在案态兜底）；absent ⇒ 维持 **maintain-SDR**。锚字面：「显示链变化 + HDR 资产需求成立」。
5. **G10-N6**：`fbx2gltf`/`assimp`/`blender` 三工具 PATH 实测 + BistroExterior 源资产树内/外部资产根检索；任一缺 ⇒ 维持**双场景闭集**。锚字面：「FBX2glTF 上游修复在树或替代臂+源资产同窗齐备」。
6. **SAFE-GPU**：独立期资源窗核验（战役收官期无专属资源 = 判据字面直接不成立）+ 平台需求方文档树内检索；未出现 ⇒ 维持 defer，归档行改锚 **defer-to-G31+**。锚字面：「独立期资源窗 + 平台需求方（外部采纳生态）出现时立项评估」。
7. **RD-042/043/044 三条 G30 尾锚窗同批逐锚重判**：锚源钉死 = g25 registry `rd_eight` 行 `g26_anchor` 字面（非 `deferred.json` 条目级字段——F8），各锚 ≥2 pattern 树内检索——RD-042「可微仿真需求场景出现」/ RD-043「GPU 刚体 out_of_scope 翻转程序 + wgrapier 成熟度证据」/ RD-044 检索面显式展开为 `milestones/g23/g23_rd044_subitem_registry.json` 三分项 `reeval_anchor` 字面；零命中 ⇒ 维持 open + history 只追加 G30 行（`check_deferred_append_only` 同律，四不可变字段 0-byte）。**追加幂等（F9）**：同 event 字面已在案则不再追加；在案历史重复行如实登记为既往数据质量事件，不回写清理。
8. **忠实性纪律（RFC-0046 F6 同律三件全承接）**：pattern 表由重判脚本**字面承载**（常量表，禁运行时构造）；逐 pattern 须为对应锚字面的**派生关键词**（pattern↔锚映射表机核可比对）；evidence 逐件载 {pattern 表, 检索根, 逐 pattern 命中数}（字段钉死）。
9. **门态映射（RFC-0045 F10 同律）**：维持 / 命中重判启动均**合法门绿**（分支捕获非透传）；门 FAIL 只保留给「重判程序未诚实执行」。

## 2. 三面商用终审程序（M-b）

### 2.1 画质面
1. **QUALITY_SURFACES 闭集 0-byte 机核**：`ci/g25_quality_final_state_verification_smoke.py` 常量十项字面沿用（`src/rurix-render/src/display`、`temporal/tsr.rs`、`temporal/taa.rs`、`temporal/upscale.rs`、`bin/g14_3_pipeline_perf.rs`、`kernels/g14_3_direct_gi.rx`、`kernels/g16_gi_multibounce.rx`、`kernels/g18_light_transport_depth.rx`、`g18_presentation_contract.json`、`g13_ue_upscale_parity_contract.json`）vs **`g25-closed`** 逐文件 git-diff 0-byte。
2. **战役加性面零接线核验（两层，只追加禁缩面——F2）**：① G25 四 token 模块检索字面沿用——`ADDITIVE_MODULES = ("framegen", "hzb", "restir_reservoir", "slab")` 按 `::{m}` / ` {m}::` 形态在 PRODUCTION_BINS 三件源码内零命中（`ci/g25_quality_final_state_verification_smoke.py` 先例字面，禁缩面）；② 战役九件名字面扩面——五 kernel 文件名（`g26_framegen.rx`/`g27_hzb_reduce.rx`/`g27_hzb_test.rx`/`g28_restir.rx`/`g29_slab.rx`；合同「四 kernel」= kernel 族计数，hzb 族含 reduce/test 两文件，文件级精化为五件——F7）与四 device bin 名（`g26_framegen_device`/`g27_hzb_device`/`g28_restir_device`/`g29_slab_device`）在 PRODUCTION_BINS 三件源码内**字符串/路径字面检索**零命中——`.rx` 的「引用」判定 = 文件名字面命中（bin 消费 kernel 的形态是路径字符串，import 检索对 .rx 无语义；device bin 间无 import 通路故 import 半判据恒真弃用，字面检索为唯一非平凡判据面）。
3. **G18 M-d 达标绿件只读盘点**：`g18_m_d_dual_end_commercial_quality_verdict` latest（AI 读图 + SSIM/FLIP 程序产阈绿件）+ `g25_m_a_quality_final_state_verification` latest 绿件只读盘点（G18→g25-closed 传递环在档面——F6）。
4. 表面 0-byte ∧ 加性零接线 ⇒ **G18 达标终态维持有效**——传递依据 = tag `g25-closed` 收官语义（vs g18-closed 0-byte 由 g25 M-a 绿件承载）∧ 本期 vs g25-closed 0-byte（重渲无信息增量；UE 全渲重跑触发条件 = 表面变化证据，未命中显式登记——RFC-0042 §1.1 同律）。

### 2.2 性能面
1. **G14 M-d 18 格最新 evidence 定盘**：`g14_m_d_dual_end_fps_parity` latest（met 计数 + 全格 ratio + 焦点格 `fps_ratio` 摘出）。
2. **性能面三文件 0-byte**：`src/rurix-render/src/bin/g14_3_pipeline_perf.rs`/`src/rurix-rt/src/render_exec.rs`/`src/rurix-rt/src/vendor_upscale.rs` vs **`g25-closed`**（全路径沿 `ci/g25_fps_parity_final_verdict_smoke.py` 三文件常量字面、基线由 g18-closed 换轨 g25-closed——F11；传递依据 = `g25_m_b_fps_parity_final_verdict` latest 绿件只读盘点 + tag 收官语义——F6；ratio 定盘的机器前提）。
3. **焦点格新鲜单测**：`g14_3_pipeline_perf --bench --scene bistro-interior --tier 100` dlss_sr 160 帧真跑（`RURIX_REQUIRE_REAL=1` + GPU 独占窗 + `bench_receipt.json` 新鲜性校验〔`K:\rurix-ext\g14-frames\rurix_prod\bistro-interior\tier100\dlss_sr`〕），ratio 对 UE 暖态包络登记——**G17-MD-F1 终判法定义务**（锚字面：「NGX 分解 profiling 或 UE 侧插桩（宿主差可分离 measured 证据，RFC-0032 重判条件同源）」）。**两半锚 pattern G30 新鲜树内检索（F3）**：沿 G26 M-d manifest 6 条 pattern 闭集只追加不缩面（`evidence/g26_m_d_g17_md_f1_rejudgment_window_*` 在案 pattern 表字面），命中 ⇒ 如实登记并按锚启动重判分支（门态映射同 §1.9，门绿）；零命中 ⇒ 「焦点格 ratio 登记面即为重判执行体」字面成立（断言升格为机器取证）。
4. **终判两态 + SKIP 第三分支（F12）**：焦点格 ratio ≥ 1.00 → **18/18 达标**；物理不可达 → **维持 17/18 诚实红终判**——两态均为合法收官态零冒充（G15「物理不可达维持未达标登记」+ G25 M-b 兜底同源）。环境/资产面不可得（GPU 独占窗或 `K:` 帧目录缺）→ **SKIP 如实登记（`skipped_dev_env`）+ 在案 17/18 维持**，`RURIX_REQUIRE_REAL=1` 下翻硬红（RFC-0046 §5 三态协议同律，schema 合法态与判据字面对齐）。

### 2.3 确定性面
1. **Stage A 18 格 digest 锚在档 18/18**（`g30.baseline.stage_a_digest_guard.anchor_count` 预算行同源，G30.0 baseline 在档）。
2. **战役四 device kernel 双跑位级绿件只读盘点**：G26~G29 M-a evidence 四件（`g26_m_a_framegen_device_kernel`/`g27_m_a_hzb_device_kernel`/`g28_m_a_restir_device_kernel`/`g29_m_a_slab_device_kernel`——各含固定输入双跑输出 digest 位级一致判据）。
3. **RD-045 累计观察面复核（机器判定面钉死——F14）**：g25~g29 五期 `*_stabilization_soak_*` latest evidence 逐期只读盘点（零失败字段逐件核验；G19~G24 六期轮次锚 = g25 registry RD-045 行字面在案）+ `deferred.json` RD-045 `status=open` 与 backfill 三件维持 open 核验（锚归 §4 归档行）。

## 3. 全链零降级（M-c）

前提：上游十一期收口 tag 计数 11/11（`g19-closed`~`g29-closed`，G30.0 baseline 在档）。执行面：**G29 受影响门 `--verify-latest` 全绿 = 战役 evidence 链完整性定盘（F4 字面如实化）**——verify-latest 为静态读档核验（读各门 latest evidence 判绿，不重新执行检查）；「递归」语义 = 各期 M-e/closeout 门在其收口时点的链式核验在档（G29 M-e 先例 = verify G28 两门），非 G30 现势重验 G13~G29 判据；**现势零回归由 §2 表面 0-byte 机核 + §2.2.3 焦点格新鲜真跑 + G30.5 soak 承载**。verify 清单钉死（F10）= `g29.p0.m_e.closed_gate_no_regression` + `g29.wave.6b.closeout` 两门（G29 M-e 先例同构，争议时只追加扩表）。预算面：**`budget_eval --strict` 全量零 skip 零 estimated（合同 §4.2 字面同构），禁 `--allow-pending`（F18）**。禁 `--gate` 旧脚本重跑；`g30_` 前缀不抢占既有门 latest（RFC-0046 M-e 同律）。

## 4. 承接锚归档（M-d，含归档完整性机核）

1. **归档闭集**：`milestones/g30/g30_campaign_handover_registry.json` = **G31+ 唯一法定输入面**——
   - `campaign_period_rows`：G26~G30 **五期逐期 ≥1 行、期集合恰为五**（g25 表 15 行先例同构，非每期恰一行——F5）；行终态枚举 = defer/maintain/**两态终判字面**（G30 期行承载 G17-MD-F1 终判行——18/18 达标或 17/18 诚实红，g25 表 G25 期行先例同构）+ `g31_anchor`（各期 P2 表原始锚 **0-byte 不回写**，本表为汇总镜像）；
   - `rd_eight`：RD-034/039/040/041/042/043/044/045 八条 `g31_anchor`（§1.7 重判终态回填 RD-042/043/044 三行，其余五条锚随各期落档态平移）；
   - `legacy_eleven_source`：`milestones/g24/g24_legacy_rd_registry.json` 引用（字段名沿 g25 registry 字面——F13；十一条清册引用不复制，G25 归档同律）；
   - `tail_six`：§1 六件尾锚重判终态（维持/翻转字面 + G30 evidence 文件引用）。
2. **归档完整性机核（M-d 同门承载）**：行数/字段闭集机核**分 section 钉死（F5）**——`campaign_period_rows` 行字段 = period/id/final/g31_anchor/source；`rd_eight` 行字段 = id/status/g31_anchor（恰 8 行）；`tail_six` 行字段 = id/final/g31_anchor/source + G30 evidence 引用（恰 6 行）；逐 section schema 校验；机核绿 = 战役收官法定输出面成立（tag `g30-closed` 收官前置）。
3. **M-e = G29 受影响门零降级**（G25 收官期 M-e 同构）：verify 清单同 §3 钉死两门；禁 `--gate` 旧脚本；`g30_` 前缀不抢 latest。**M-c/M-e 分工（F10）**：M-c = G30.3 定盘（附 budget --strict 全量），M-e = G30.4 收官前复核（附前缀不抢 latest 核验）——同集合两时点核验，非重复判据。

## 5. out-of-scope（各附承接锚 = §4 归档锚字面）

1. **尾锚六件任一实现**（Jolt 5.6 采纳施工/M127 corpus 建设/SAFE-GPU 立项等——F1）：承接锚 = `tail_six` 各行 `g31_anchor` 字面。
2. **HDR 显示链接入与 HDR 资产**：承接锚 = M118-hdr-cal 行 `g31_anchor`（显示链变化 + HDR 资产需求成立）。
3. **Jolt 切换/GPU 刚体翻转**：承接锚 = RD-043 `g31_anchor` 字面（翻转程序 + wgrapier 成熟度证据）。
4. **毛发资产制作与 strand 压测**：承接锚 = M114-strand 行 `g31_anchor`（毛发资产入压测闭集）。
5. **战役外新优化/新特性/重构**：承接锚 = G31+ 立项程序（g30 归档表为唯一法定输入面，绕开归档表的立项无效）。

## 6. 验收门映射

| P0 | 门 key | 波次 | 判据面 |
|---|---|---|---|
| M-a | `g30.p0.m_a.tail_anchor_rejudgment_closure` | G30.2 | §1（六件机器取证 + RD 三条同批重判 + manifest 忠实性 + 门态映射） |
| M-b | `g30.p0.m_b.commercial_final_review` | G30.2 | §2（画质 0-byte + 加性零接线 / 焦点格真跑两态终判 / 确定性三件盘点） |
| M-c | `g30.p0.m_c.campaign_full_chain_no_regression` | G30.3 | §3（verify-latest 递归 + budget --strict 全量） |
| M-d | `g30.p0.m_d.campaign_handover_ledger` | G30.3 | §4.1~§4.2（五期行 + rd_eight + legacy_source + tail_six + 归档完整性机核同门承载） |
| M-e | `g30.p0.m_e.closed_gate_no_regression` | G30.4 | §4.3（G29 受影响门 verify-latest 全绿零降级 + g30_ 前缀不抢 latest） |

与 G30_CONTRACT §4.2 同构。**implemented / maintain-defer / 诚实红终判均为合法终态，零冒充**——终审以机器事实定盘：表面 0-byte 为维持终态的必要条件；阈值零手写（×1.00 口径与画质阈程序产面 0-byte——RFC-0042 §2 不变量同律）；探针/工具缺 SKIP 如实登记不冒充。M-a~M-e 真跑 evidence 落档后本 RFC 终态 = 战役收官五面记录字面（tag `g30-closed` 收官）；争议按只追加程序重判。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v0.1 | 2026-08-25 | G30.1 起草（Draft）：§1 六件尾锚重判闭集（逐件机器取证 + RD-042/043/044 同批 + F6 忠实性/F10 门态同律）；§2 三面商用终审（画质 QUALITY_SURFACES vs g25-closed + 战役加性五 kernel/四 bin 零接线、性能焦点格 G17-MD-F1 终判两态、确定性 Stage A + 四 device 双跑绿件 + RD-045 累计）；§3 全链零降级；§4 归档闭集 + 完整性机核（G31+ 唯一法定输入面）；待 D-409 对抗性评审。 |
| v0.2 | 2026-08-25 | D-409 对抗评审 18 findings 全处置（blocker 0/major 5/minor 13，报告 `milestones/g30/design/rfc0047_adversarial_review.md`）：**F1** M125-adopt3「UE」错标改「Jolt 5.6」（§1.1/§5.1）；**F2** §2.1.2 零接线判据重写为两层（四 token 模块检索沿用禁缩面 + 战役九件名字面检索，弃恒真 import 半判据）；**F3** §2.2.3 补两半锚 pattern G30 新鲜检索（G26 M-d manifest 6 条闭集只追加）；**F4** §3「递归涵盖全链」如实化为 evidence 链完整性定盘 + 现势零回归承载面点名；**F5** §4 归档字段闭集分 section 钉死 + G17-MD-F1 终判行归档槽位点名（campaign_period_rows G30 期行）+「五期行」歧义消除；**F6** G18/G14 终态传递环补 g25 M-a/M-b latest 只读盘点；**F7**「五件 = 四 kernel 族文件级精化」登记；**F8** RD 三条锚源钉死 g25 registry `g26_anchor` + RD-044 展开三分项；**F9** history 追加幂等 + 在案重复行如实登记；**F10** verify 清单钉死两门 + M-c/M-e 分工声明；**F11** 性能面三文件补全路径；**F12** 焦点格 SKIP 第三分支（`skipped_dev_env`）；**F13** `legacy_eleven_source` 字段名对齐 g25；**F14** RD-045 复核判定面钉死（五期 soak latest 盘点 + status=open 核验）；**F15** M114 检索三态闭集；**F16** 头表上游行溯源改写（六件原始行在 G23/G24 P2 表经 g25 镜像）；**F17** M125 补 g9_m125 A/B latest 盘点；**F18** 预算面对齐合同「零 skip 零 estimated」+ 禁 `--allow-pending`。状态 → **Agent Approved**。 |
