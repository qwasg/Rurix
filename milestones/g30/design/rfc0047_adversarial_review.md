<!-- Assisted-by: Cursor Agent（D-409 独立对抗评审，零共享上下文） -->
# RFC-0047 对抗性评审报告 — G30 战役商用终审收官程序

| 字段 | 值 |
|---|---|
| 被评审 RFC | RFC-0047（`rfcs/0047-campaign-final-review.md`） |
| RFC 版本 | v0.1（Draft，2026-08-25） |
| 评审程序 | D-409 独立对抗评审（与起草者零共享上下文；先例 = `milestones/g25/design/rfc0042_adversarial_review.md`） |
| 评审日期 | 2026-08-25 |
| 评审方法 | 逐条树内机器取证（文件存在性 / 字面逐字比对 / 数字核对）+ 程序设计对抗挑战；零推测，全部证据来自评审员亲读的树内文件 |
| **评审结论** | **approve-with-amendments**（blocker 0 / major 5 / minor 13，共 18 条 findings；F1~F5 为修法必办项） |

---

## 0. 取证通过面（核对一致，无 finding）

以下逐项机器取证与 RFC 字面一致，如实登记：

1. **六件尾锚锚字面**：§1.1~§1.6 六条锚字面 vs `milestones/g25/g25_campaign_handover_registry.json` 尾部七行 `g26_anchor` 逐字一致（M125-adopt3「需求证据三类任一命中（5.6 独有 API 引用/5.3 缺陷命中/A/B 超带）」、M127「corpus + PhysicsAsset residual 消费方出现（搜索面闭集只追加扩面）」、M114-strand「毛发资产入压测闭集」、M118-hdr-cal「显示链变化 + HDR 资产需求成立」、G10-N6「FBX2glTF 上游修复在树或替代臂+源资产同窗齐备」、SAFE-GPU「独立期资源窗 + 平台需求方（外部采纳生态）出现时立项评估」）。第七件 G17-MD-F1 锚「NGX 分解 profiling 或 UE 侧插桩（宿主差可分离 measured 证据，RFC-0032 重判条件同源）」逐字一致；`rfcs/0032-d3d12-host-ngx-lane.md` 在树。
2. **RD 八条现状**：`registry/deferred.json` RD-034/039/040/041/042/043/044/045 八条 `status` 全为 `open`，与 §4.1 rd_eight 编号闭集、G29_P2_DECISIONS §2 八条映射（全部「G30 尾锚窗/终审窗在案」）吻合。RD-042/043/044 三条 G25 归档锚字面与 §1.7 引号内字面吻合（RD-043「GPU 刚体 out_of_scope 翻转程序 + wgrapier 成熟度证据」逐字）。
3. **QUALITY_SURFACES 十项**：`ci/g25_quality_final_state_verification_smoke.py` L24-35 常量十项与 §2.1.1 清单逐项一致；PRODUCTION_BINS 三件（`g14_3_pipeline_perf.rs`/`g13_4_ue_upscale_parity_render.rs`/`g12_pt_production.rs`）与 §2.1.2 一致。
4. **战役加性面九件在树**：五 kernel（`src/rurix-render/kernels/g26_framegen.rx`、`g27_hzb_reduce.rx`、`g27_hzb_test.rx`、`g28_restir.rx`、`g29_slab.rx`）与四 device bin（`src/rurix-render/src/bin/g26_framegen_device.rs`、`g27_hzb_device.rs`、`g28_restir_device.rs`、`g29_slab_device.rs`）全部存在。
5. **性能面**：G14 M-d latest（`evidence/g14_m_d_dual_end_fps_parity_20260824T091444Z.json`）实测 `met_count=17`、18 格齐、焦点格 bistro-interior/t100/dlss_sr `fps_ratio=0.8563`——与 §2.2 的 17/18 + 焦点格口径吻合（G26_P2 §1 引 `ratio 0.856326` 同源）。性能面三文件在树（`src/rurix-render/src/bin/g14_3_pipeline_perf.rs`、`src/rurix-rt/src/render_exec.rs`、`src/rurix-rt/src/vendor_upscale.rs`），与先例 `ci/g25_fps_parity_final_verdict_smoke.py` L49-52 常量清单一致。`bench_receipt` 与 `RURIX_REQUIRE_REAL`（`g14_3_pipeline_perf.rs` 内 6 处）机制树内实存。
6. **确定性面**：`evidence/g30_baseline_stage_a_digest_guard.json` 实测 18.0（notes「anchors=18」）；`milestones/g30/g30_budget.json` 条目 `g30.baseline.stage_a_digest_guard.anchor_count` threshold=18.0/measured=18.0 同源。四 device 双跑绿件逐件在档且逐件含 `device_double_run_bitexact` PASS fact（g26 20260825T030005Z「三档固定输入双跑序列 digest 位级相等」/ g27 20260825T044714Z「双臂固定输入全链双跑 digest 位级相等」/ g28 20260825T063647Z「固定输入双跑输出缓冲 digest 位级相等」/ g29 20260825T083957Z）。
7. **前提 tag**：`git tag --list` 实测 g19-closed~g29-closed 恰 11 个（含 g26~g29 四 tag），`g30_budget.json` 条目 `g30.baseline.campaign_tags.count` threshold=11.0/measured=11.0 同源。
8. **门 key 五行**：§6 表五行（key/波次）与 `milestones/g30/harness/g30_gen_schemas.py` SLUGS 常量表逐字一致（`tail_anchor_rejudgment_closure`/G30.2、`commercial_final_review`/G30.2、`campaign_full_chain_no_regression`/G30.3、`campaign_handover_ledger`/G30.3、`closed_gate_no_regression`/G30.4），并与 G30_ACCEPTANCE_MAP §1 五行一致；m_a/m_b schema 侧 `skipped_dev_env` 合法态在 gen_schemas L38 实存。
9. **检索面闭集上游确证**：M127「corpus 四目录」= `evidence/g23_m_b_neural_deform_rejudgment_20260824T182511Z.json` 搜索面闭集 `['corpus', 'assets/corpus', 'assets/neural', 'conformance/neural']` 四项；M118 三 token = `milestones/g24/harness/g24_hdr_probe.py` HDR_TOKENS 常量（`VK_COLOR_SPACE_HDR10_ST2084_EXT`/`BT2020_LINEAR`/`HDR10_HLG`，g24 探针结果三 token 全 absent 在案）；G10-N6 三工具与源资产检索 = `milestones/g24/g24_bistro_exterior_recheck.json`（fbx2gltf/assimp/blender 三缺 + search_roots `["K:/rurix-ext", "assets", "external"]`）。
10. **同律引用实体核对**：RFC-0046 §4.2 三态闭集（absent/present/SKIP〔工具缺，如实登记 + 在案态兜底〕）、RFC-0046 F6 三件（常量表承载/锚字面派生/字段钉死，见其 §3.2）、RFC-0045 F10 门态映射（分支捕获非透传、维持/翻转均门绿、门 FAIL 只保留给程序未诚实执行，见其 §4.2）——被引条款全部实存且语义吻合。RFC-0042 §1.1「表面变化证据未命中显式登记」先例字面实存。「G25 收官期 M-e 同构」成立（`milestones/g25/g25_m_e_closed_gate_no_regression_evidence_schema.json` 在档；注意 RFC-0042 本体仅列 M-a~M-d，M-e 为 G25 合同层，引用仍成立）。
11. **编号纪律**：`registry/number_ledger.json` namespaces.RFC `on_tree_max=47`/`next_free=48`——RFC-0047 头表「next_free=47 顺位领取」与领取后台账现状自洽；G30_CONTRACT rfc_required 同字面。
12. **G19~G29 soak 累计面**：g25~g29 五期 `*_stabilization_soak_*.json` 绿件在档（G19~G24 六期轮次锚在 g25 registry RD-045 行字面）。
13. `milestones/g24/g24_legacy_rd_registry.json` 在树（§4.1 legacy 引用源实存）。

**SKIP 登记（树外面，如实降级）**：① §2.2.3 焦点格帧目录 `K:\rurix-ext\g14-frames\rurix_prod\bistro-interior\tier100\dlss_sr` 与 ② §1.3 毛发资产面 `K:/rurix-ext/assets`——均为外部盘路径，本评审未访问，仅核验树内消费机制字面（bench_receipt 代码面 / g24 recheck search_roots 在案）。相关设计挑战见 F12/F15。

---

## 1. Findings（F1~F18）

### F1 [major] M125-adopt3 被错标为「UE 采纳档」，实为 Jolt 5.6 采纳档

- **RFC 位置**：§1.1「**M125-adopt3**（UE 采纳档）」；§5.1「尾锚六件任一实现（**UE 5.6 采纳施工**/M127 corpus 建设/SAFE-GPU 立项等）」。
- **证据（树内实测）**：`milestones/g23/g23_jolt_adoption_registry.json` L3-6：「M125-adopt3 采纳臂三件重判登记」`evaluation_arm.crate = "src/rurix-physics-sys56"`，`upstream = "JoltC@2982004 + JoltPhysics v5.6.0"`；G30_CONTRACT §4.2 M-a「M125-adopt3(**Jolt 5.6** 需求证据三类树内实测…)」；G30_CANDIDATE_DECISIONS §1 M125 行分项名「**Jolt 5.6 采纳窗**」；G23_CANDIDATE_DECISIONS「Jolt 5.6 采纳臂⑦三件」。树内四处法定文本一致为 Jolt，唯 RFC-0047 写 UE。
- **挑战陈述**：RFC 是终审程序法定文本。§1.1 的括号错标尚可由正确的锚字面兜住（pattern 从锚字面派生），但 §5.1 的「UE 5.6 采纳施工」是 out-of-scope 承接锚正文字面——按 §4/§5 设计，该字面将平移进 `g30_campaign_handover_registry.json`（G31+ 唯一法定输入面），把「UE 引擎 5.6 采纳」这一错误对象固化给 G31+ 立项程序。同时 §6「与 G30_CONTRACT §4.2 同构」声明在此点自破。
- **建议处置**：§1.1「UE 采纳档」→「Jolt 5.6 采纳档」；§5.1「UE 5.6 采纳施工」→「Jolt 5.6 采纳施工」。修法批一行字面改动，不触程序结构。

### F2 [major] §2.1.2 战役加性零接线判据：对 device bin 恒真、对 .rx 无「import」语义、G25 四 token 检索面未显式保留

- **RFC 位置**：§2.1.2「战役 kernel 文件五件…与四 device bin…**不被任何生产 bin 引用——源码 import 检索**（PRODUCTION_BINS 三件…；G25 ADDITIVE_MODULES 纪律的战役全量扩面）」。
- **证据（树内实测）**：① 四 device bin 均为 `src/rurix-render/src/bin/*.rs` 独立 bin target——Rust 编译模型下 bin target 之间不存在 import 通路，「生产 bin 引用 device bin」在语言层不可构造，该半判据**恒真**；② `.rx` kernel 非 Rust 模块，bin 对其引用形态是路径/文件名字符串（运行时编译消费），「import 检索」检不出字符串引用；③ 先例 `ci/g25_quality_final_state_verification_smoke.py` L37-42 的实际判定是模块 token 检索（`ADDITIVE_MODULES = ("framegen", "hzb", "restir_reservoir", "slab")`，匹配 `::{m}` / ` {m}::`）——RFC 字面自称「扩面」，实为把检索对象从模块 token **替换**为文件件数，原四 token 检索面未显式保留。
- **挑战陈述**：恒真判据 = 结构性必绿，零信息量，属「可被设计冒充的绿」；若实现体照 RFC 字面只做 import 检索，则真实接线通路（生产 bin 源码内 `::framegen` 类模块引用、或 kernel 路径字符串消费）反而失察——与 RFC 自己在 §1.2 立的「搜索面闭集只追加扩面，禁缩面」纪律冲突。
- **建议处置**：判据改写为可机器执行且非平凡的两层：① 沿用 G25 四 token 模块检索字面（禁缩面）；② 追加战役九件名（五 kernel 文件名 + 四 device bin 名）在 PRODUCTION_BINS 三件源码内的**字符串/路径字面检索**，并显式声明 .rx 的「引用」判定 = 文件名字面命中。

### F3 [major] §2.2.3 第七件尾锚 G17-MD-F1 的重判锚条件在 G30 期零新鲜检索——「本期未出现」是断言而非机器取证

- **RFC 位置**：§2.2.3「…（锚字面：『NGX 分解 profiling 或 UE 侧插桩…』；**宿主差可分离证据本期未出现** ⇒ 焦点格 ratio 登记面即为重判执行体）」。
- **证据（树内实测）**：G26_P2_DECISIONS §1 G17-MD-F1 行：G26.3 M-d 做过「两半证据树内闭集搜索实测 0+0 命中（searched-paths manifest 6 条 pattern 逐条登记，F6 非空清单硬线）」→「维持 17/18 诚实红 carry，**终判归 G30 商用终审**」。该 0+0 检索时点 = 2026-08-25 G26.3（`evidence/g26_m_d_g17_md_f1_rejudgment_window_20260825T030400Z.json`）。RFC-0047 §2.2 全节无任何两半锚 pattern 检索程序。
- **挑战陈述**：§1 对六件尾锚全部要求「机器取证 + manifest 必填」，第七件虽归 §2.2 承载终判，但其锚条件（两半证据出现与否）的现势判定被静默降格为起草时断言。G27~G29 三期窗内树内若新增 NGX 分解 profiling 或 UE 侧插桩证据，G30 程序无检出面——同一 RFC 内判据纪律双标，且「重判执行体 = 焦点格 ratio 登记面」的前提（两半未命中）失去机器支撑。
- **建议处置**：M-b 性能面追加一件 fact：两半锚 pattern 新鲜树内检索（沿 G26 M-d manifest 6 条 pattern 闭集，只追加不缩面），命中 → 如实登记并按锚启动重判分支（门态映射同 §1.9，门绿）；未命中 → 「焦点格 ratio 登记面即为重判执行体」字面成立。

### F4 [major] §3「递归涵盖 G13~G29 全链既有门」与 `--verify-latest` 的机器行为不符——静态读档链，非现势重验

- **RFC 位置**：§3「执行面：G29 受影响门 `--verify-latest` **递归全绿（递归涵盖 G13~G29 全链既有门）**」。
- **证据（树内实测）**：① `ci/g25_quality_final_state_verification_smoke.py` L98-100：`--verify-latest` 分支 = `load_latest_evidence(SUBJECT)` 后判 `host_section_pass`——**纯读档，不重新执行任何检查**；② `evidence/g29_m_e_closed_gate_no_regression_20260825T084226Z.json` facts：`verify_g28_closed_gate rc=0` + `verify_g28_closeout（VERDICT=READY）`——G29 M-e 的「递归」= 在 G29.4 时点读 G28 两门（M-e + closeout）的历史 evidence；G30 时点对 G29 门发 verify-latest，读到的是 G29 期时间戳的 evidence 快照。
- **挑战陈述**：该链证明的是「各期 latest evidence 在档且绿」（evidence 链完整性），不是「G30 现势下 G13~G29 门判据仍成立」。g25-closed 之后对 0-byte 闭集之外文件的任何回归性改动，M-c 全绿照常——「全链零回归」的现势检出实际全部压在 §2 的 0-byte 机核与 soak 上。字面夸大机器保证，属「可被绕过的判据」：改动非闭集文件 + M-c 依旧绿 = 名义全链零回归。`budget_eval --strict` 同为静态断言（读 `measured_value` 历史值 vs threshold，`ci/budget_eval.py` L290-296），不产生新鲜测量。
- **建议处置**：§3 字面如实化：「G29 受影响门 verify-latest 全绿 = **战役 evidence 链完整性定盘**（递归链条 = 各期 M-e/closeout 门在其收口时点的链式核验在档）；现势零回归由 §2 表面 0-byte 机核 + §2.2.3 焦点格新鲜真跑 + G30.5 soak 承载」。不建议为此重跑旧门 `--gate`（与既有禁令冲突），只修声明字面。

### F5 [major] §4 归档完整性机核字段闭集与三 section 实际字段集不匹配；G17-MD-F1 终判归档槽位未点名

- **RFC 位置**：§4.1（四 section 结构）；§4.2「必填字段闭集（**id/final/g31_anchor/source**）schema 校验」。
- **证据（树内实测）**：同构参照 `milestones/g25/g25_campaign_handover_registry.json`：`campaign_period_rows` 行字段 = `period/id/final/g26_anchor/source` 五键；`rd_eight` 行字段 = `id/status/g26_anchor` 三键（**无 final/source**）；legacy 引用字段名 = `legacy_eleven_source`。且 g25 表 `campaign_period_rows` 为 15 行（每期 1~3 行不等），非每期一行。G17-MD-F1 在 g25 表中占 campaign_period_rows G25 期一行（final =「17/18 诚实红终判…」）。
- **挑战陈述**：① 单一字段闭集「id/final/g31_anchor/source」对 rd_eight 不可校（g25 同构行无 final/source；若强加则破坏「g25 registry 同构」自我声明）；② §4.1 campaign_period_rows 字面「五期行：G26~G30 各期 defer/maintain 终态」——「五期行」歧义（恰五行还是五期若干行？），且 defer/maintain 枚举**装不下 G17-MD-F1 的两态终判**（18/18 达标或 17/18 诚实红均非 defer/maintain）；③ tail_six 明确只含 §1 六件——于是战役最重要的商用性能终判字面在归档表四 section 中**无点名槽位**，而该表是「G31+ 唯一法定输入面」。
- **建议处置**：分 section 钉死字段闭集（campaign_period_rows: period/id/final/g31_anchor/source；rd_eight: id/status/g31_anchor；tail_six: id/final/g31_anchor/source + evidence 引用），机核逐 section 校验；§4.1 显式点名「campaign_period_rows G30 期行承载 G17-MD-F1 终判行（g25 表 G25 期行先例同构）」；「五期行」改为「G26~G30 五期逐期 ≥1 行、期集合恰为五」。

### F6 [minor] 画质/性能 0-byte 基线由 g18-closed 换轨 g25-closed，传递前提未列入盘点面

- **RFC 位置**：§2.1.1「vs **g25-closed**」；§2.2.2「vs **g25-closed**」；§2.1.4「表面 0-byte ∧ 加性零接线 ⇒ G18 达标终态维持有效」。
- **证据**：先例脚本均 vs `g18-closed`（`ci/g25_quality_final_state_verification_smoke.py` L49、`ci/g25_fps_parity_final_verdict_smoke.py` L49）。「G18 终态经 g25-closed 传递」依赖隐式环：g25-closed 时点表面 ≡ g18-closed 时点表面，该环由 G25 M-a/M-b 门绿件承载，RFC 未要求盘点。
- **挑战陈述**：vs g25-closed 0-byte 单独只证明 g25-closed 以来无变化；不盘点 G25 期绿件则「G18 达标终态维持」的推理链留一环口头前提（tag 收官语义兜底，事实风险低，故 minor）。
- **建议处置**：§2.1.3 盘点清单追加 `g25.p0.m_a`（画质）与 g25 M-b fps 终判 latest 绿件只读盘点各一件，或在 §2.1.4/§2.2.2 显式声明传递依据 = tag g25-closed 收官语义（G25 五 P0 绿件在档）。

### F7 [minor] 「五件 kernel」vs 合同/验收映射「四 kernel」——§6 同构声明失真

- **RFC 位置**：§2.1.2「战役 kernel 文件五件」；§6「与 G30_CONTRACT §4.2 同构」。
- **证据**：G30_CONTRACT §4.2 M-b 与 G30_ACCEPTANCE_MAP §1 M-b 均写「战役期加性面(**四 kernel**/四 device bin)」；树内实测五件 .rx（g27 期两件：`g27_hzb_reduce.rx` + `g27_hzb_test.rx`）。RFC 是与树内事实一致的一方。
- **挑战陈述**：三份法定文本两个数字；验收映射 §4 又自称与合同「逐字相等」——机核实现以谁为准未钉死，验收争议时字面互指。
- **建议处置**：RFC 修订记录显式登记「五件 = 合同『四 kernel』的文件级精化（hzb 族含 reduce/test 两文件）」，并提请合同/验收映射随批勘误或加同一注records。

### F8 [minor] §1.7 锚字段指称混用：「reeval_anchor」非 deferred.json 条目级字段

- **RFC 位置**：§1.7「各 `reeval_anchor` 字面 ≥2 pattern 树内检索」。
- **证据**：`registry/deferred.json` RD-042/043/044 条目级字段为 `backfill_condition`（无条目级 `reeval_anchor`）；引号内三条锚字面实际来自 g25 registry rd_eight 行 `g26_anchor`；`reeval_anchor` 字段实存于分项登记表（如 `milestones/g23/g23_rd044_subitem_registry.json`、`g23_research_track_registry.json`）。RD-044 的锚「三分项 reeval_anchor（G23 闭集落档）」本身还是二级间接引用。
- **挑战陈述**：F6 忠实性纪律要求 pattern↔锚映射机核可比对——锚源字段指称不清，机核读不到法定锚字面的唯一落点；RD-044 若不展开 g23 三分项 reeval_anchor，「≥2 pattern」将从间接引用句派生，检索面失真。
- **建议处置**：钉死锚源 = g25 registry rd_eight 行 `g26_anchor` 字面；RD-044 检索面显式展开为 `g23_rd044_subitem_registry.json` 三分项 `reeval_anchor` 字面。

### F9 [minor] deferred history 在案重复行；G30 只追加程序无幂等/判重要求

- **RFC 位置**：§1.7「零命中 ⇒ 维持 open + history 只追加 G30 行（`check_deferred_append_only` 同律…）」。
- **证据**：`registry/deferred.json` RD-042 history 2026-08-24 G23.3 行**逐字重复两次**（L1448-1462）；RD-043 同（L1474-1488）；RD-044 M-d 行同（L1505-1519）——append-only 机核不判重的既往质量事件在案。
- **挑战陈述**：G30 行若被重复追加同样过机核；收官档携带重复行进入 G31+。
- **建议处置**：G30 追加程序声明幂等（同 event 字面已在案则不再追加）；在案重复行如实登记为既往数据质量事件（四不可变字段纪律维持，不回写清理）。

### F10 [minor] 「G29 受影响门」集合未闭集列举；M-c 与 M-e 判据重叠、分工未声明

- **RFC 位置**：§3 与 §4.3/§6 M-e 行。
- **证据**：G29 M-e 先例实测集合 = G28 两门（`verify_g28_closed_gate` + `verify_g28_closeout`，见 g29 M-e evidence facts）；RFC/合同/验收映射均未列 G30 应 verify 的 G29 门清单。M-c 与 M-e 均含「G29 受影响门 --verify-latest 全绿」，差异仅 budget --strict（M-c）与前缀不抢 latest（M-e）。
- **挑战陈述**：集合可被最小化解释（verify 一门即字面合规）；两门重复 verify 同一集合的分工语义（G30.3 定盘 vs G30.4 收官前复核）无声明，争议时无从裁决。
- **建议处置**：钉死清单字面（建议 = `g29.p0.m_e.closed_gate_no_regression` + `g29.wave.6b.closeout` 两门，G29 M-e 先例同构，争议时只追加扩表）；一句话声明 M-c/M-e 分工。

### F11 [minor] §2.2.2 性能面三文件仅给文件名，未给全路径（跨两 crate）

- **证据**：`g14_3_pipeline_perf.rs` 在 `src/rurix-render/src/bin/`，`render_exec.rs`/`vendor_upscale.rs` 在 `src/rurix-rt/src/`；先例 `ci/g25_fps_parity_final_verdict_smoke.py` L49-52 为全路径常量。
- **挑战陈述**：机核需全路径；文件名级字面在未来同名文件出现时二义。
- **建议处置**：补全路径或点名「沿 `ci/g25_fps_parity_final_verdict_smoke.py` 三文件常量字面（基线改 g25-closed）」。

### F12 [minor] 焦点格新鲜单测的外部依赖无 SKIP 分支字面（判据面两态 vs schema 三态）

- **RFC 位置**：§2.2.3（GPU 独占窗 + `K:\rurix-ext\...` 帧目录 + bench_receipt 新鲜性）；§2.2.4「终判两态」。
- **证据**：帧目录为外部盘（本评审 SKIP 未核）；`g30_gen_schemas.py` L38 为 m_b schema 配了 `skipped_dev_env` 合法态；§6 尾句有「探针/工具缺 SKIP 如实登记不冒充」总则——但 §2.2 判据正文只给两态（≥1.00 / 物理不可达），环境不可得时执行体无法定分支。
- **挑战陈述**：外部盘缺失/GPU 窗不可得时，两态字面逼迫执行体要么硬凑真跑（冒充）要么无据 SKIP；schema 有态而判据无字面 = 字面与 schema 面不齐。
- **建议处置**：§2.2 补第三分支：「环境/资产面不可得 → SKIP 如实登记（skipped_dev_env）+ 在案 17/18 维持 + `RURIX_REQUIRE_REAL=1` 下翻硬红」（RFC-0046 §5 三态协议同律）。

### F13 [minor] §4.1 `legacy_source` 字段名与 g25 registry 实际字段 `legacy_eleven_source` 漂移

- **证据**：`g25_campaign_handover_registry.json` L31 字段名为 `"legacy_eleven_source"`；RFC 写 `legacy_source`。
- **挑战陈述**：自称「g25 registry 同构」「G25 归档同律」的表，字段名静默改名——归档机核 schema 与同构声明二者必失其一。
- **建议处置**：钉死字段名（沿用 `legacy_eleven_source`，或显式声明 G30 schema 改名及理由）。

### F14 [minor] §2.3.3 RD-045 累计观察复核的机器判定面未钉死

- **证据**：g25 registry RD-045 行锚字面「G19.3 观察窗 12/12 中锚零漂移 + G19~G24 六期 soak（63+67+69+69+69+G24 轮次）全零失败零漂移事件」在案；g25~g29 五期 soak 绿件在档。RFC 只写「复核」与「累计」，未定义读哪些文件的哪些字段判「零失败零漂移」。
- **建议处置**：钉死复核 = 逐期 soak latest evidence 只读盘点（零失败字段）+ `deferred.json` RD-045 `status=open` 与 backfill 三件维持 open 核验。

### F15 [minor] §1.3 M114-strand 检索根为外部盘，不可达时无 SKIP 分支

- **证据**：§1.3 检索面字面 = 「`K:/rurix-ext/assets` hair 资产面」（外部盘，本评审 SKIP）；§1.4 HDR 探针有三态闭集，§1.3 无。
- **挑战陈述**：盘符不可达时「未命中 ⇒ 维持 card/mesh」与「无法检索」不可区分——absent 与 SKIP 混同，恰是 RFC-0046 §4.2 三态律要防的冒充形态。
- **建议处置**：§1.3 补三态（命中/未命中/检索根不可达 SKIP + 在案态兜底），与 §1.4 拉平。

### F16 [minor] 头表上游行把「G26~G29 四期 P2 表」列为 defer/maintain 原始锚源——对六件尾锚不成立

- **证据**：G29_P2 穷举 7 行（RD-041 两行 + G29-N1~5）、G26_P2 穷举 8 行（G13-N7/RD-045-window/G17-MD-F1 + G26-N1~5）——四期 P2 表均**无六件尾锚行**；六件的期内原始锚在 G23/G24 P2 表（M125/M127 于 G23，M114/M118/G10-N6 于 G24，SAFE-GPU 于 g24 清册 + g25 归档行）。
- **挑战陈述**：§4.1「各期 P2 表原始锚 0-byte 不回写」的溯源指针若照头表指向 G26~G29 四期表，六件尾锚将溯空。
- **建议处置**：上游行改写为「G26~G29 四期 P2 表（战役期期内条目锚）+ 六件尾锚原始锚经 g25 registry 镜像（原始行在 G23/G24 P2 表）」。

### F17 [minor] §1.1 M125 重判检证面较 G23 先例缩面：缺 g9_m125 A/B 绿件只读盘点

- **证据**：G23 M-a 判据（G23_ACCEPTANCE_MAP §1）= 「…+ **g9_m125 A/B 最新绿件只读盘点** + 评估臂构建新鲜真跑（cargo check）+ 采纳三件成立条件核验」；`g23_jolt_adoption_registry.json` ADOPT-1 basis 明依 A/B 绿件在案。RFC §1.1 只保留 cargo check 一件。
- **挑战陈述**：「在案三件条件 1/3 不变」的 1/3（ADOPT-1 met）本身以 A/B 绿件在案为据——绿件不盘点则 1/3 断言失锚；检证面对上游先例缩面，与「只追加扩面」纪律方向相反。
- **建议处置**：§1.1 补「`evidence/g9_m125_jolt_56_ab_evaluation_*` latest 只读盘点（禁 --gate 重跑）」一件。

### F18 [minor] §3 预算面字面「零违例」弱于合同「零 skip 零 estimated」；`--allow-pending` 逃逸口未封

- **证据**：G30_CONTRACT §4.2 M-c = 「budget_eval --strict 全量**零 skip 零 estimated**」；RFC §3 = 「`budget_eval --strict` 全量零违例」。`ci/budget_eval.py` L6-7/L400-401 实测：--strict 下 estimated 即 FAIL，但 `--allow-pending <id>` 可使未达标计数器保留 SKIP 且整体 exit 0。
- **挑战陈述**：「零违例」字面下带 --allow-pending 跑 strict 可产 SKIP 仍自称合规——收官期预算面留了一个白名单逃逸口，且 RFC 与合同字面不同构。
- **建议处置**：§3 对齐合同字面并加「禁 `--allow-pending`」一句。

---

## 2. 结论汇总表

| Finding | 等级 | RFC 位置 | 一句话结论 | 处置类型 |
|---|---|---|---|---|
| F1 | major | §1.1 / §5.1 | M125-adopt3 错标「UE」，树内四处法定文本均为 Jolt 5.6 | 修字面（必办） |
| F2 | major | §2.1.2 | 零接线判据对 device bin 恒真、.rx 无 import 语义、四 token 面未保留 | 重写判据（必办） |
| F3 | major | §2.2.3 | G17-MD-F1 两半锚条件 G30 期零新鲜检索，断言替代取证 | 补检索程序（必办） |
| F4 | major | §3 | 「递归涵盖全链」与 verify-latest 静态读档行为不符 | 字面如实化（必办） |
| F5 | major | §4.1 / §4.2 | 归档字段闭集与 section 字段集不匹配；G17-MD-F1 归档槽位缺 | 分 section 钉死（必办） |
| F6 | minor | §2.1 / §2.2 | 0-byte 基线换轨传递前提未盘点 | 补盘点件 |
| F7 | minor | §2.1.2 / §6 | 五件 vs 四 kernel，「同构」声明失真 | 登记精化注 |
| F8 | minor | §1.7 | 锚字段指称 reeval_anchor/backfill_condition/g26_anchor 混用 | 钉死锚源 |
| F9 | minor | §1.7 | history 在案重复行；追加无幂等要求 | 声明幂等 |
| F10 | minor | §3 / §4.3 | 「G29 受影响门」集合未列举；M-c/M-e 分工未声明 | 钉死清单 |
| F11 | minor | §2.2.2 | 性能面三文件缺全路径 | 补路径 |
| F12 | minor | §2.2.3~4 | 焦点格真跑外部依赖缺 SKIP 分支字面 | 补三态 |
| F13 | minor | §4.1 | legacy_source 字段名与 g25 实际 legacy_eleven_source 漂移 | 钉死字段名 |
| F14 | minor | §2.3.3 | RD-045 复核机器判定面未钉死 | 钉死盘点面 |
| F15 | minor | §1.3 | 外部资产根不可达无 SKIP 分支 | 补三态 |
| F16 | minor | 头表上游 | 四期 P2 表非六件尾锚原始锚源 | 改溯源字面 |
| F17 | minor | §1.1 | M125 缺 g9_m125 A/B 绿件盘点件（对先例缩面） | 补盘点件 |
| F18 | minor | §3 | 「零违例」弱于合同「零 skip 零 estimated」；--allow-pending 未禁 | 对齐合同字面 |

**计数**：blocker 0 / major 5 / minor 13，共 18 条。
**结论**：**approve-with-amendments** —— 程序骨架（六件机器取证重判 + 三面终审两态定盘 + 归档闭集）与树内事实兼容且先例（RFC-0042/0045/0046 同律链）引用全部实存；F1~F5 五条 major 修法落字面后可转 Agent Approved。本报告零改 RFC 本体；所有证据均为评审员亲读树内文件实测字面，树外面（K: 盘两处）已如实 SKIP 登记。
