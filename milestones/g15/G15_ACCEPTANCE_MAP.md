<!-- Assisted-by: Kimi-K3（G15.1 治理波） -->
# G15_ACCEPTANCE_MAP — P0 验收映射

> **性质**：G15.1 治理交付物（governance-only）；事实源为 [G15_CONTRACT.md](G15_CONTRACT.md) v1.0 front matter acceptance_gates（G-G15-1~9）与 §4.2 五行 P0 独立断言表、G15 法定输入面（[G14PLUS_RECORD.md](../g14/G14PLUS_RECORD.md) §6.3 G15 承接锚 + [G14_P2_DECISIONS.md](../g14/G14_P2_DECISIONS.md) §5 defer-to-G15+ 29 行承接锚 + G13 超分 8 行/Lumen 2 行/G12 PT 10 行三差距登记表终态 + RD-045 观察窗）、[G15_CANDIDATE_DECISIONS.md](G15_CANDIDATE_DECISIONS.md) v1.0（§1 G14 defer 29 行逐行处置〔M61/M52/M100-high 等 14 行 defer-to-G16+ 窗结论〕，§2 open RD 八条映射，§3 G15 新增候选 → M-a~M-e 映射）。
> **编号纪律**：本表 P0 行只冻结 symbolic CI gate key 与稳定脚本名，不 claim 数字 CI step。数字步骤必须等 G-G15-2 实现互锁（§6）通过后，再从 `registry/number_ledger.json` 当时实测的 `CI_step.next_free` 顺位分配——**P0 行 numeric_step 一律写 `post-interlock actual-next-free allocation`**；禁止沿用推测号与任何草案建议值。**例外面**：G15.1 治理三门（§5）按契约 §4.3/§7 立项裁决 4 明令本波即落盘真脚本真步骤——步骤 266/267/268 = 落盘前实测 `CI_step.next_free=266` 顺位领取，ledger 校准同批。
> **M 行号纪律**：M-a~M-e 字母行号为治理期稳定身份；M### 数字在 G15.2+ 实现波 materialize 时按落盘前实测 M 命名空间实际顺位领取（沿 G13 M167~M171 / G14 先例），本表不预占 M 数字。
> **证据纪律**：表内脚本与 schema 是实现 PR 的强制目标路径；二者须与对应 RED→GREEN 实现同 PR 落地。路径尚未 materialize 只表示未开工，不能记 PASS；本波不预建空脚本、空 schema 壳或占位 workflow 步骤（只冻结路径——治理三门为例外，见上）。

---

## 1. P0 硬门（精确 5 行）

- P0 精确集合（5 行）：`{M-a, M-b, M-c, M-d, M-e}`。每一行的 symbolic key 同时是该能力唯一的 `assertion_id`，与 [G15_CONTRACT.md](G15_CONTRACT.md) §4.2 **逐字一致**（key 命名空间双方一致性机核面，禁止任何改写）；独立硬判据列同逐字（0-byte 转引）。实现后的 evidence 顶层必须至少含：`schema_version`、`subject`、`milestone`、`wave`、`assertion_id`、`status`、`commands`、`environment`、`base_commit`、`run_url`、`timestamp`；其中 `assertion_id` 必须等于本表 key，`status` 必须为 `pass|fail`，不得以 `skip|estimated|advisory` 充绿（条件未触发只能登记 `not-triggered`、环境缺失只能登记 `dev_env_degrade`，见 §3）。
- 每个 symbolic key 最终一对一取得一个 numeric CI step。脚本可复用，但 workflow 必须按 `--gate <symbolic-key>` 独立调用、独立产 evidence、独立给结论；任一 P0 assertion 缺失或失败，只能使该 P0 为红，**不得用同脚本内另一 assertion 的绿色代替**。一次 smoke 可共享进程启动成本，但每行必须单独产出 `PASS|FAIL|SKIP|DEV_ENV_DEGRADE`；只有 `PASS` 满足 P0。
- **单一 key/脚本命名空间**：symbolic key 一律小写点分 `g15.p0.m_<a~e>.<slug>`，脚本一律 `ci/g15_<slug>_smoke.py`，evidence schema 一律 `milestones/g15/g15_m_<a~e>_<slug>_evidence_schema.json`（slug 与 key 末段同字面，只冻结路径不预建文件）。本表 §1 与 [G15_CONTRACT.md](G15_CONTRACT.md) §4.2 引用同一份 key/脚本/判据，由 `ci/g15_acceptance_map_check.py` 双向比对强制一致（见 §4；G15 治理三件套无独立 CI_GATES——门冻结面 = 契约 §4.2 + 本表 §1/§2，沿 G14 体例精简）。
- device 判据统一要求真实路径、`RURIX_REQUIRE_REAL=1`、validation error 为 0；mock、stub、host substitution、仅检查非零输出、缺 provisioning 的 SKIP 均不满足能力门。host oracle、既有最小见证、人工截图均不能替代目标门。**AI 读图强制门为 G15 新增法定面**（G14.10f 教训字面兑现：digest 双跑一致 ≠ 内容正确）——M-a 基线臂与 M-c 终审面每格出图必须经 AI 读图结构完整性审查，读图记录入 evidence，digest 面不替代内容面。
- **统一判据形态**（M-a~M-e 共用纪律字面，契约 §4.1 逐字转引）：接入/落盘 + 冻结面 0-byte（RXS-0357 起步范围与参照器面 / UpscaleBackend trait 签名面与 temporal 底座历史接口面 / G13 锁定双差距登记表终态 / G12 锁定 PT 差距登记表终态 / G11 GI 既有判据 / M96 golden 门序 D2-Q7 / RXS-0386~0393 锁定度量口径）+ measured 面标定程序产阈禁手写（P-09）+ 不降级既有 84 门绿面 + AI 读图强制门（digest 面不替代内容面）+ 性能零降级守护（G14 18/18 ×1.00 维持）。

| M 行 | symbolic gate key | 稳定脚本 | evidence schema 目标路径 | 判据（逐字） | 波次 | numeric_step |
|---|---|---|---|---|---|---|
| **M-a** | `g15.p0.m_a.dual_end_quality_reharvest` | `py -3 ci/g15_dual_end_quality_reharvest_smoke.py --gate g15.p0.m_a.dual_end_quality_reharvest` | `milestones/g15/g15_m_a_dual_end_quality_reharvest_evidence_schema.json` | 双端画质对拍链路全量复跑（G13 M-c ue_upscale_parity + G13 M-d ue_lumen_gi_parity + G12 M163 ue_pt_parity 三门同口径复跑，对拍契约 digest 0-byte 门序维持）+ 20 行登记表逐项重评（逐行 gap_id 逐字转引 + fresh measured_delta + 方向判定〔收敛/维持/劣化〕）+ G15 差距处置表 `milestones/g15/g15_quality_gap_disposition.json` 落盘零空行 + UE 方差带程序产（G14 M-a 双程序产面取严口径继承）+ AI 读图基线臂（双场景 × 三档 × 三后端出图结构完整性断言） | **G15.2** | post-interlock actual-next-free allocation |
| **M-b** | `g15.p0.m_b.gap_fix_closure_loop` | `py -3 ci/g15_gap_fix_closure_smoke.py --gate g15.p0.m_b.gap_fix_closure_loop` | `milestones/g15/g15_m_b_gap_fix_closure_loop_evidence_schema.json` | measured 主差修复闭环：处置表 20 行逐行终态处置 ∈ {closed-resolved（修复后 fresh delta 进容差带，RXS-0393 收敛判据两款）/ closed-caliber-registered（口径差显式登记不拟合，RXS-0392）/ open-defer-G16+（承接锚字面「重判条件 = …；兜底 = …」）} 零空行 + 修复项 RED 先行（失败测试先落 main 为 RED）+ 触冻结面独立 Full RFC 留痕（D-409 对抗评审）+ 材质链表达面立项评估结论登记（G11-N8/G11-N9/G12-N10 承接锚命中判定逐字：透射/焦散/镜面 IBL 类能量是否成为画质量级 measured 主差） | **G15.3** | post-interlock actual-next-free allocation |
| **M-c** | `g15.p0.m_c.absolute_quality_final_review` | `py -3 ci/g15_absolute_quality_review_smoke.py --gate g15.p0.m_c.absolute_quality_final_review` | `milestones/g15/g15_m_c_absolute_quality_final_review_evidence_schema.json` | 绝对画质通过线设立 + 严格画面审查：绝对通过线程序产标定（UE 参照 deficit 双 seed 方差底 p100×2.0 程序产，禁手写 P-09，标定链路入 evidence）+ 双场景 × 三档（t50/t67/t100）× 三后端（tsr_device/dlss_sr/fsr_3_1_5）18 格逐格判定 + 逐格 AI 读图严格画面审查记录（无乱序/无错位/无全黑/关键结构可见——cornell 盒体结构、bistro 吊灯/吧台/桌椅）+ 商用收口判定（达标格数/18 + 未达标格如实登记不冒充） | **G15.4** | post-interlock actual-next-free allocation |
| **M-d** | `g15.p0.m_d.perf_parity_zero_regression` | `py -3 ci/g15_perf_parity_guard_smoke.py --gate g15.p0.m_d.perf_parity_zero_regression` | `milestones/g15/g15_m_d_perf_parity_zero_regression_evidence_schema.json` | 性能零降级守护：G14 M-d 门同口径复跑（双场景 × 三档 × 三后端 18 格，三轮进程级独立运行 50×3 trimmed mean 跨轮中位数 + 逐轮守护带）逐格 ratio ≥ ×1.00 维持 + G14 M-c 画质锚带复核（SSIM deficit ≤ 0.010779849285388998 带内）+ G14 门产 budget 条目零 estimated 维持 + 画质修复致性能劣化静默即 RED | **G15.5** | post-interlock actual-next-free allocation |
| **M-e** | `g15.p0.m_e.regression_drift_guard` | `py -3 ci/g15_regression_drift_guard_smoke.py --gate g15.p0.m_e.regression_drift_guard` | `milestones/g15/g15_m_e_regression_drift_guard_evidence_schema.json` | 回归门 + 漂移监控：既有 84 门（G9 34 + G10 14 + G11 14 + G12 9 + G13 5 + G14 8）最新 evidence 全绿只读汇总不遮蔽 + 触改面真跑抽检零降级 + RD-045/M165 同型 digest 漂移监控登记（G15 复跑面检出计数/零检出字面入 evidence） | **G15.6a** | post-interlock actual-next-free allocation |

任一行缺失、合并后不可区分、非 `PASS` 或无对应 evidence schema，均阻断 G15.6b（契约 §4.2 末段 / G-G15-9 字面）。**G15 新设通过线唯一面 = M-c 绝对画质通过线**（程序产标定禁手写 P-09，标定链路入 evidence）；**M-d 帧率面 = 维持 G14 已定盘 ×1.00 通过线的零降级守护**（不新设、不放宽、不以画质修复为代价）。

---

## 2. 已 go P1 硬门（零行）

G15.1 无 go 的 P1 行——候选决策 5 行实现门（M-a~M-e）全为 P0（契约 §4.2 字面）。后续波次若治理程序将新 P1 判为 go，须先按治理程序修订本表及覆盖集合（只追加进 §2）再开对应实现；不得把它静默并入现有 key。本节的机核面 = §2 零行声明与 `ci/g15_acceptance_map_check.py` 的 P1 空集断言（§5）。

---

## 3. 条件型 / not-triggered 登记面

### 3.1 异己并发工作树面（G15.0 洁净机核在案，后续异己面严禁混入）

立项裁决 3（契约 §7 逐字登记）：G15.0 工作树洁净面机核——G14 战后遗留面（soak 门产 budget/方差样本刷新 + 未提交 evidence）已经 commit `34f96ac3` 归档，零异己 src/ 未提交面；G15.0 不可变 ref = `f061487efaf7816684de18a6ef86554e5c392a75`（G14 close-out flip commit，tag `g14-closed`）。纪律：G15 全波 commit 按文件名显式择取，只含 G15 车道文件；后续异己面出现即按 G14-N6 同律严禁消费/混入（G10.8b §8.10/G11/G12/G13/G14 先例同模）；异己面 evidence 不充 G15 任何门绿；若 G15 实现波与异己面触及同一文件，按只追加程序登记冲突面并请治理裁决（不得静默合并）。

### 3.2 G14 defer 行 G15 重评窗登记面

- **G14 defer-to-G15+ 29 行承接锚**（G14_P2_DECISIONS §5 = G15 法定输入，契约 upstream_docs 字面）：逐行处置锚定 [G15_CANDIDATE_DECISIONS.md](G15_CANDIDATE_DECISIONS.md) §1，承接锚 0-byte 转引，本表不展开逐行清单。
- **M61（mesh shader 车道）/ M52（SER）/ M100-high（MegaLights 多灯压测）等 14 行**：G15 重评窗结论 = **defer-to-G16+**——G13.4/G14 双窗未命中在案，G15 重评窗只登记不立项（契约 out_of_scope `mesh_shader_ser_restir_high_end_lanes` 字面），承接锚字面 0-byte 维持；窗结论逐字以候选决策表 §1 登记为准。
- 登记 `SKIP=not-triggered` 只表示决策已记录，不是成功，不充任何门绿；defer 行如实保持 open，不写进全绿叙述（G-G15-7 字面）。

### 3.3 材质链表达面 / FG/MFG / 场景集边界登记面

- **材质链表达面（G11-N8/G11-N9 + G12-N10 锚定 G15 字面 = M-b 评估程序面）**：G15 承接 = M-b 判据内联的立项评估结论登记——G15.3 波按承接锚命中判定逐字登记「透射/焦散/镜面 IBL 类能量是否成为画质量级 measured 主差」；评估结论登记 ≠ 本波立项承诺，命中成立须按治理程序独立立项（触冻结面独立 Full RFC，D-409 对抗评审）。
- **FG/MFG（G13-N7）**：G14 重评窗不立项在案（生成帧不计入真实渲染帧率口径），G15 画质收口期不承接（契约 out_of_scope `frame_generation_fg_mfg_independent_layer` 字面）；FG/MFG 独立层立项 = 商用收口后独立期面；`registry/deferred.json` RD-041 history 只追加登记。
- **场景集边界（G10-N6）**：G15 双场景闭集 = cornell-box + bistro-interior **0-byte**，BistroExterior 未入清单维持（契约 out_of_scope `new_scene_set_expansion` 字面）；M133 清单 digest 注册在树；新场景集扩展不属 G15。

### 3.4 绝对画质通过线口径面（程序产标定，不 retroactive 改写）

M-c 绝对画质通过线 = **程序产标定**（UE 参照 deficit 双 seed 方差底 p100×2.0 程序产口径沿 G13 标定链，禁手写 P-09，标定链路入 evidence）；逐格判定逐字入 evidence，未达格如实登记不冒充；通过线设立**不 retroactive 改写 G13/G14 已 closed 判据（0-byte）**——G13/G14 门判据语义冻结面维持，G15 新增通过线只前向适用于 G15 终审面（契约 guardrails 第七条字面）。

---

## 4. 双向一致与互斥面（key 命名空间机器可核声明）

1. **双方逐字一致**：本表 §1 五行与 [G15_CONTRACT.md](G15_CONTRACT.md) §4.2 五行对同一 P0 M 行给出的 symbolic gate key、稳定脚本名与独立硬判据**必须逐字相等**（判据列 0-byte 转引），由 `ci/g15_acceptance_map_check.py` 双向比对机器强制；任一处漂移即 FAIL。本声明为机器可核面：比对以文件字面为准，不以叙述替代。（G15 无独立 CI_GATES——契约 ↔ MAP 双向，契约 §4.3 治理三门表为第三冻结面，沿 G14 体例精简。）
2. **唯一命名空间**：`g15.p0.m_<a~e>.<slug>` + `ci/g15_<slug>_smoke.py` + `milestones/g15/g15_m_<a~e>_<slug>_evidence_schema.json` 为唯一合法形态；G15 命名空间（`g15.*`）与 G9 已消费 34 key（`g9.*`）、G10 已消费 14 key（`g10.*`）、G11 已消费 14 key（`g11.*`）、G12 已消费 9 key（`g12.*`）、G13 已消费 5 key（`g13.*`）、G14 已消费 8 key（`g14.*`）互不包含；全部 key 全局唯一，匹配 `g15\.p0\.m_[a-e]\.[a-z0-9_]+`；没有两个 M 行共享 key。
3. **互斥**：M 行与 key 一对一；`no-go`/`defer` 项不产生 key、不入本表，不得冒充 PASS；G14 defer 维持行（M61/M52/M100-high 等 14 行 defer-to-G16+ 窗结论行与其余承接行）不入本表；P0 集合变更属于契约变更，不得以勘误处理。**本表 §1 五 key 与 [G15_CANDIDATE_DECISIONS.md](G15_CANDIDATE_DECISIONS.md) 35 行候选行 ID 命名空间互斥**（候选行 ID 不得命中已 go 门裸 token）——由 `ci/g15_candidate_decisions_check.py` 机器承载（§5）。
4. **防混淆登记**：M-b 判据内联的 RXS-0392/RXS-0393 处置口径 = spec 锁定度量口径面（只消费不回写）；M-e 漂移监控消费的 RD-045/M165 同型 digest 漂移族与 M-a fresh measured_delta 重评面不同面——检出即升级评估（生产化缺陷修复项 + Full RFC 评估），零检出维持 open-defer 不写进全绿叙述（契约 §6 字面）。
5. **基线锚溯源**：M-a 三门复跑锚 = G13 M-c `ue_upscale_parity` + G13 M-d `ue_lumen_gi_parity` + G12 M163 `ue_pt_parity` 对拍契约 digest 0-byte 门序维持；20 行重评面 = `g13_ue_upscale_gap_registry.json` 8 行 + `g13_ue_lumen_gap_registry.json` 2 行 + `g12_ue_pt_gap_registry.json` 10 行终态只消费不回写（G15 处置面另立 `milestones/g15/g15_quality_gap_disposition.json` 新文件，gap_id 逐字转引 + fresh measured_delta 可溯源）；M-d 锚 = G14 M-d 18/18 ×1.00 定盘（G14 §8.13 closed 终态，flip commit `f061487e` + tag `g14-closed`）+ G14 M-c 画质锚带（SSIM deficit ≤ 0.010779849285388998）+ `g14_budget.json` 基线；三轮进程级独立运行统计协议（50×3 trimmed mean）转引自 M141/M165 冻结口径（BENCH_PROTOCOL §3）。

---

## 5. G15.1 治理覆盖与空行门

G15.1 治理三门（本波 materialize，步骤按契约 §4.3/§7 立项裁决 4 落盘前实测 `CI_step.next_free=266` 顺位领取；**白名单声明**：本波除下列治理三门外，零 workflow 步骤、零脚本、零 schema 壳预放）：

```text
g15.wave.1.acceptance_map         步骤 266（落盘前实测 CI_step.next_free=266 顺位领取）
  py -3 ci/g15_acceptance_map_check.py --gate g15.wave.1.acceptance_map

g15.wave.1.candidate_decisions    步骤 267（同批顺位领取）
  py -3 ci/g15_candidate_decisions_check.py --gate g15.wave.1.candidate_decisions

g15.gov.implementation_interlock  步骤 268（同批顺位领取）
  py -3 ci/g15_interlock_check.py --gate g15.gov.implementation_interlock
```

`ci/g15_acceptance_map_check.py` 的 PASS 判据（coverage + no-empty 两组断言分别独立报告）：

1. P0 行集合与 §1 的 5 项**集合全等**，无遗漏、无额外 P0、无重复；§2 P1 集合为空集（G15.1 零 go P1 字面）。
2. 全部 symbolic key 全局唯一，均匹配 `g15\.p0\.m_[a-e]\.[a-z0-9_]+`；每行只有一个 canonical `assertion_id`，没有两个 M 行共享 key；key 的 m 段字母与行号一致。
3. 每一行均有脚本命令（`--gate` 参数 == canonical key）、evidence schema、可机器求值的 PASS 判据、最晚波次。
4. **双向一致**：本表 §1 与 `G15_CONTRACT.md` §4.2 对同一 P0 M 行给出的 key、脚本与判据必须逐字相等；任一处漂移即 FAIL。

no-empty 组的 PASS 判据：

- 逐单元格拒绝**占位单元格闭集**：空串 `""`、空白、`TBD`、`TODO`、`待定`、`待补`、`—`；表中全部行的必填列均非空。
- 所有 schema 路径必须唯一落到对应 M 行，且文件名含同一 `m_<a~e>` 与同一 slug；所有波次属于 `G15.2|G15.3|G15.4|G15.5|G15.6a` 的非空集合。
- G15.1 只核映射完整性，不把尚未 materialize 的脚本/schema 误判为实现绿色；对应能力实现 PR 合入前或同 PR，`ci/check_schemas.py` 必须能校验该 schema 与实际 evidence。

`ci/g15_candidate_decisions_check.py` 的 PASS 判据：G15_CANDIDATE_DECISIONS 候选行 **35 行闭集全等**（§1 G14 defer-to-G15+ 29 行承接锚逐行处置 + §2 open RD 八条逐条映射 + §3 G15 新增候选行）+ 裁决枚举合法（go/no-go/defer-to-G16+/strategic_override）+ **零空行门**（全列非空，占位单元格闭集同上逐单元格拒绝）+ 承接锚纪律（§1 行承接锚 0-byte 转引；defer 行含「重判条件 = …；兜底 = …」字面）+ defer-to-G16+ 裁决行 G16+ 重评窗字面（裁决/最终状态列承载，转引列不回写）+ go 行验收映射锚义务（登记留痕位置含 G15_ACCEPTANCE_MAP）+ §2 RD 行条目级 status==open + 与本表 5 key 互斥（候选行 ID 不得命中已 go 门裸 token）。

`ci/g15_interlock_check.py` 的 PASS 判据：逐项读取事实源输出 §6 各条件真值；G15.1 期间必须诚实输出 `BLOCKED`（validator 能识别阻断，不算互锁 PASS 实现面）；仅全部条件为真时才输出 `READY`（`--require-ready` exit 0）；`--gate` 模式产 evidence（VERDICT 字面入档，BLOCKED 不充绿）。

三个 validator 均把每组断言逐条打印，不以一个总 `all_pass=true` 掩盖具体缺行；`--selftest` 用受控负样本证明它们能红。

---

## 6. G15.2 硬互锁

`G15.GOV.G15_2.ENTRY_INTERLOCK` 是 G15.2 的前置 required check（`ci/g15_interlock_check.py --require-ready`，治理三门之一同脚本）。`implementation_status: blocked` 解锁须以下条件**同时**为真（契约 front matter `implementation_unlock.required_all` 与 G-G15-2 字面展开）：

1. G15.1 治理门全部完成且有真实验证记录——§5 的 `g15.wave.1.acceptance_map`（coverage + no-empty 两组）与 `g15.wave.1.candidate_decisions` 独立 PASS；G15_CANDIDATE_DECISIONS 分项映射零空行；`registry/deferred.json` history 只追加、无静默改判（vs G15.0 base 条目四字段 0-byte）；本表 §1/§2 无缺行（D-G15-1~4）。
2. `ci/g15_interlock_check.py --require-ready` 输出 **READY**（互锁 validator 机器事实，不以叙述替代）。
3. 用户 G15.2 开工指令已留痕——2026-08-23 指令全期授权面「**一次性完成G15里程碑**，积极使用并行智能体和workflow减少工期」字面 + 2026-08-19 授权面「最终交付产物需要真实可商用，否则不要停止优化，并在此时允许在G15后无限制新建里程碑继续优化」字面（契约 §7 立项裁决 2 逐字登记，G14-N7 先例：用户明示即生效）。
4. 共享编号按互锁开放时 **actual next_free 重新校准**——G15 的 P0 实现门 numeric CI step claim 发生在上述互锁通过之后，并以 claim 当下 ledger 实测 `CI_step.next_free` 为起点；每个 symbolic key 一对一分配，未复用、未覆盖、未沿用任何推测号与草案建议值（治理三门步骤 266/267/268 为 G15.1 实测领取的合法面）。

任一条件为假时，互锁必须返回非零；此时禁止合入 G15.2 的 `spec/`、`conformance/`、`src/` 或 workflow 实现改动。不能用 owner override 绕过任何条件，也不能用本表存在本身当作 G15.2 开工许可。`ci/g15_interlock_check.py --require-ready` 输出 READY 是机器事实，不以叙述替代（契约 G-G15-2 字面）。

---

## 7. Close-out 审计

- G15.6a 稳定门（G-G15-8）必须重跑全部 5 个 P0 与所有 go 的 P1 的**各自 assertion**；evidence 的 `base_commit` 必须落在同一候选 close-out 基线，零 skip/estimated；画质对拍与帧率对标链路连续复跑 soak（量级沿 G14.5a 继承〔≥1800s〕或 measured 证明更短足够）；`budget_eval --strict` 非空全 PASS、零 estimated/skip；既有 84 门（G9 34 + G10 14 + G11 14 + G12 9 + G13 5 + G14 8）零降级（M-e 门承载）；G5~G14 既有判据 0-byte。
- G15.6b 收口门（G-G15-9）**终审八 facts 面**齐备才可 status flip：① 验收映射最终状态、② 候选决策最终状态、③ RD 最终状态三面逐字一致；④ 全部 P0 独立断言均 PASS（任一 P0 无独立硬门则禁止 flip）；⑤ evidence 终审、⑥ schema 终审、⑦ 预算终审（strict budget 非空全 PASS）；⑧ 商用收口终审定盘——达标/未达标如实登记不冒充，未达标按用户 2026-08-19 授权新建 G16+ 里程碑继续优化，性能零降级守护面终态锁定；§8 只追加后 status active→closed，flip 独立 commit + `g15-closed` tag。
- 同日放行先例继承（沿 G14 立项裁决同模）：6a full-run 先行完成后允许同日进 close-out；条件实现刚绿不得跳过 soak 直接 close。
- **性能零降级守护口径维持到 close-out**：G15 全期任何画质修复/终审复跑不得致 G14 M-d 18 格 ratio 跌破 ×1.00 通过线（逐轮守护带口径）；M-d 门复跑核验 = 每波退出硬前置；优化致性能劣化静默即 RED（契约 guardrails 第九条字面）。
- 后续若治理流程将新的 P1 判为 go，须先按治理流程修订本表及覆盖集合，再开对应实现；不得把它静默并入现有 key。

---

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-23 | G15.1 初版：冻结 5 个 P0 的 symbolic gate、目标脚本/schema、独立判据（契约 §4.2 逐字 0-byte 转引）与最晚波次——双端画质对拍链路全量复跑 + 20 行登记表逐项重收割 1 行（M-a，G15.2，G14PLUS_RECORD §6.3 承接锚兑现）+ measured 主差修复闭环逐项处置 1 行（M-b，G15.3，材质链表达面立项评估程序面内联）+ 绝对画质通过线程序产标定 + 18 格 AI 读图严格画面审查 + 商用收口判定 1 行（M-c，G15.4，G14 out_of_scope 锚定 G15 面兑现）+ 性能零降级守护 1 行（M-d，G15.5，G14 M-d 18/18 ×1.00 定盘承接 + G14 M-c 画质锚带复核）+ 回归门 + 漂移监控 1 行（M-e，G15.6a，既有 84 门零降级 + RD-045/M165 承接）；§2 零 go P1 声明；§3 条件型/not-triggered 登记面（异己并发工作树面〔G15.0 洁净机核 commit 34f96ac3 归档在案〕/ G14 defer 行 G15 重评窗〔14 行 defer-to-G16+ 锚候选决策表 §1〕/ 材质链·FG/MFG·场景集边界 / 绝对画质通过线口径面）；§4 key 命名空间双方逐字一致 + 与候选决策 35 行 ID 命名空间互斥机器可核声明（G15 无独立 CI_GATES，契约 ↔ MAP 双向）+ 基线锚溯源；单一命名空间 `g15.p0.m_<a~e>.<slug>` + `ci/g15_<slug>_smoke.py` + `g15_m_<a~e>_<slug>_evidence_schema.json` 由 `ci/g15_acceptance_map_check.py` 双向比对强制；§5 治理三门（步骤 266/267/268 = 落盘前实测 CI_step.next_free=266 顺位领取，白名单声明）+ 候选决策表 35 行闭集零空行门 + 占位单元格闭集；§6 G15.2 硬互锁（implementation_status 解锁条件面）；§7 Close-out 审计（G-G15-9 终审八 facts 面）。P0 行数字 CI 步骤全部 `post-interlock actual-next-free allocation`，零 P0 workflow/script/schema 预放。 |
