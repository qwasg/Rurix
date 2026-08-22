<!-- Assisted-by: Kimi-K3（G14.1 治理波起草） -->
# G14_ACCEPTANCE_MAP — P0 验收映射

> **性质**：G14.1 治理交付物（governance-only）；事实源为 [G14_CONTRACT.md](G14_CONTRACT.md) v1.0 front matter acceptance_gates（G-G14-1~9）与 §4.2 五行 P0 独立断言表、G14 立项前调研（UE benchmark 臂测量口径 + UE 渲染确定性控制面两份调研报告，2026-08-19 主会话留痕）、[G14_CANDIDATE_DECISIONS.md](G14_CANDIDATE_DECISIONS.md) v1.0（§3 新增候选 → M-a~M-e 映射，§1 G13 defer 24 行处置〔G10-N11/G10-N16 go 兑现窗〕，§2 RD-041 FG/MFG 分项 G14 窗承接）。
> **编号纪律**：本表 P0 行只冻结 symbolic CI gate key 与稳定脚本名，不 claim 数字 CI step。数字步骤必须等 G-G14-3 实现互锁（§6）通过后，再从 `registry/number_ledger.json` 当时实测的 `CI_step.next_free` 顺位分配——**P0 行 numeric_step 一律写 `post-interlock actual-next-free allocation`**（当前实测 `CI_step.next_free=247`，G13 已消费至 246，ledger v1.141）；禁止沿用任何草案建议值。**例外面**：G14.1 治理三门（§5）按 2026-08-19 任务面明令本波即落盘真脚本真步骤——步骤按落盘前实测 actual next_free 顺位领取，ledger 校准同批。
> **M 行号纪律**：M-a~M-e 字母行号为治理期稳定身份；M### 数字在 G14.2+ 实现波 materialize 时按落盘前实测 M 命名空间实际顺位领取（沿 G13 M167~M171 先例），本表不预占 M 数字。
> **证据纪律**：表内脚本与 schema 是实现 PR 的强制目标路径；二者须与对应 RED→GREEN 实现同 PR 落地。路径尚未 materialize 只表示未开工，不能记 PASS；本波不预建空脚本、空 schema 壳或占位 workflow 步骤（只冻结路径——治理三门为例外，见上）。

---

## 1. P0 硬门（精确 5 行）

- P0 精确集合（5 行）：`{M-a, M-b, M-c, M-d, M-e}`。每一行的 symbolic key 同时是该能力唯一的 `assertion_id`，与 [G14_CONTRACT.md](G14_CONTRACT.md) §4.2 **逐字一致**（key 命名空间双方一致性机核面，禁止任何改写）；独立硬判据列同逐字。实现后的 evidence 顶层必须至少含：`schema_version`、`subject`、`milestone`、`wave`、`assertion_id`、`status`、`commands`、`environment`、`base_commit`、`run_url`、`timestamp`；其中 `assertion_id` 必须等于本表 key，`status` 必须为 `pass|fail`，不得以 `skip|estimated|advisory` 充绿（条件未触发只能登记 `not-triggered`、环境缺失只能登记 `dev_env_degrade`，见 §3）。
- 每个 symbolic key 最终一对一取得一个 numeric CI step。脚本可复用，但 workflow 必须按 `--gate <symbolic-key>` 独立调用、独立产 evidence、独立给结论；任一 P0 assertion 缺失或失败，只能使该 P0 为红，**不得用同脚本内另一 assertion 的绿色代替**。一次 smoke 可共享进程启动成本，但每行必须单独产出 `PASS|FAIL|SKIP|DEV_ENV_DEGRADE`；只有 `PASS` 满足 P0。
- **单一 key/脚本命名空间**：symbolic key 一律小写点分 `g14.p0.m_<a~e>.<slug>`，脚本一律 `ci/g14_<slug>_smoke.py`，evidence schema 一律 `milestones/g14/g14_m_<a~e>_<slug>_evidence_schema.json`（slug 与 key 末段同字面，只冻结路径不预建文件）。本表 §1 与 [G14_CONTRACT.md](G14_CONTRACT.md) §4.2 引用同一份 key/脚本，由 `ci/g14_acceptance_map_check.py` 双向比对强制一致（见 §4；G14 治理三件套无独立 CI_GATES——门冻结面 = 契约 §4.2 + 本表 §1/§2，沿 G13 体例精简）。
- device 判据统一要求真实路径、`RURIX_REQUIRE_REAL=1`、validation error 为 0；mock、stub、host substitution、仅检查非零输出、缺 provisioning 的 SKIP 均不满足能力门。host oracle、既有最小见证、人工截图均不能替代目标门。
- **统一判据形态**（M-a~M-e 共用纪律字面）：接入/落盘 + 冻结面 0-byte（UpscaleBackend trait 签名面与 temporal 底座历史接口面 / G13 锁定双差距登记表终态 / G11 GI 既有判据 / M96 golden 门序 D2-Q7）+ measured 面标定程序产阈禁手写（P-09）+ 不降级既有 76 门绿面。**G14 帧率通过线 = Rurix 三轮进程级独立运行 trimmed mean 帧率 ≥ UE 同口径 benchmark 臂 ×1.00（「略高」下限——用户 2026-08-19 指令字面）**；**G14 不设绝对画质通过线**——「已达 UE5 画质」判定归 G15 商用收口期（契约 §1/§5 字面），G14 画质面 = 零降级回归守护（G13 锁定基线带内不劣化）。
- 负例 RED 臂列：「内联」= 契约 §4.2 判据字面中已含的 RED 臂（逐字摘录，与判据列同源）。

| M 行 | Symbolic CI gate key / 稳定脚本 | Evidence schema（目标路径，只冻结不预建） | 独立硬判据（契约 §4.2 逐字） | 负例 RED 臂 | device/host 性质 | 最晚波次 | numeric_step |
|---|---|---|---|---|---|---|---|
| **M-a** | `g14.p0.m_a.registry_variance_band_reconciliation`<br>`py -3 ci/g14_registry_variance_band_reconciliation_smoke.py --gate g14.p0.m_a.registry_variance_band_reconciliation` | `milestones/g14/g14_m_a_registry_variance_band_reconciliation_evidence_schema.json` | M-c/M-d 门登记表 UE 方差带结构化对账修订（G13 §8.7 承接锚兑现）：身份面（gap_id 集/场景集/metric/kind/模块归属/行数）逐字节 + Rurix 侧测量值位级一致 + UE 侧测量值程序产方差带（门内 UE 探针格双跑方差底 ×headroom 程序产禁手写 P-09，真实内容变更 ≫方差带检出面维持）+ 修订后 M-c/M-d 全门复跑双绿（登记表在树态复跑不再误报厂商随机方差）+ RED 双臂（UE 侧大方差注入检出 / 小方差带内吸收）+ G13 锁定双登记表 8+2 行终态 0-byte 不回写 + UE 确定性控制面调研结论登记（cvar/收敛面，压缩方差底）；方差带手写冒充程序产即 RED；身份面漂移静默即 RED；修订后 M-c/M-d 复跑仍误报即 RED | 内联：方差带手写冒充程序产即 RED；身份面漂移静默即 RED；修订后 M-c/M-d 复跑仍误报即 RED | host+device | **G14.2** | post-interlock actual-next-free allocation |
| **M-b** | `g14.p0.m_b.ue_benchmark_arm_measurement`<br>`py -3 ci/g14_ue_benchmark_arm_measurement_smoke.py --gate g14.p0.m_b.ue_benchmark_arm_measurement` | `milestones/g14/g14_m_b_ue_benchmark_arm_measurement_evidence_schema.json` | UE 侧 benchmark 臂正式帧率测量（G10-N11 承接锚兑现）：臂 B `-game -benchmark` 命令面闭集（RXS-0380 L2）双场景（cornell-box + bistro-interior）× 超分档三轮进程级独立运行 measured（进程冷启动逐轮独立，缓存冷热面登记）+ MRQ 开销剥离 measured 量化（同场景 MRQ 臂 frameRenderDuration vs benchmark 臂帧时差值 = 捕获合并开销 measured，G10-N11 口径字面兑现）+ 环境画像七元组 + 锁频/时钟面登记（provenance 闭集沿 RXS-0380 L3）+ 50×3 trimmed mean 统计协议（M141/M165 冻结口径）入 g14_budget（measured_local 零 estimated）；以 MRQ 含开销数据冒充 benchmark 臂即 RED；单轮冒充三轮即 RED；estimated 冒充 measured 即 RED | 内联：以 MRQ 含开销数据冒充 benchmark 臂即 RED；单轮冒充三轮即 RED；estimated 冒充 measured 即 RED | host+device | **G14.2** | post-interlock actual-next-free allocation |
| **M-c** | `g14.p0.m_c.rurix_pipeline_perf`<br>`py -3 ci/g14_rurix_pipeline_perf_smoke.py --gate g14.p0.m_c.rurix_pipeline_perf` | `milestones/g14/g14_m_c_rurix_pipeline_perf_evidence_schema.json` | Rurix 生产管线性能面：release 生产管线全链路帧时（G13.4 登记的 debug 构建 + 逐帧回读同步口径倒挂面〔tier67 > tier100 host 拷贝/同步主导〕消除——异步回读/提交面重叠 + 逐帧同步消除 + TSR device kernel 效率面）+ 双场景三后端三轮进程级独立运行 50×3 trimmed mean measured 入 g14_budget（零 estimated）+ 优化前后 measured 对照（G13 g13_budget 帧时基线条目为优化前锚）+ 固定 seed 位级确定性协议维持 + temporal 底座 0-byte；host 侧逐帧拷贝/同步主导倒挂未消除静默即 RED；estimated 冒充 measured 即 RED；确定性协议漂移即 RED | 内联：host 侧逐帧拷贝/同步主导倒挂未消除静默即 RED；estimated 冒充 measured 即 RED；确定性协议漂移即 RED | host+device | **G14.3** | post-interlock actual-next-free allocation |
| **M-d** | `g14.p0.m_d.dual_end_fps_parity`<br>`py -3 ci/g14_dual_end_fps_parity_smoke.py --gate g14.p0.m_d.dual_end_fps_parity` | `milestones/g14/g14_m_d_dual_end_fps_parity_evidence_schema.json` | 双端帧率正式对标 + 画质零降级守护（G10-N16/G11-N3 帧率面兑现 + 用户「帧率对标UE5略高（不降级画质）」字面兑现）：同场景同输出分辨率同超分档位 GPU 管线双端 A/B（UE 臂 = M-b benchmark 臂测量面；Rurix 臂 = M-c 生产管线路径）三轮进程级独立运行 + 通过线 = Rurix 三轮 trimmed mean 帧率 ≥ UE 同口径 ×1.00（略高下限，逐轮守护带登记）+ 画质零降级守护（G13 锁定对拍 deficit 基线——经 M-a 修订后方差带结构化对账面复跑核验不劣化；G14 不设绝对画质通过线归 G15）+ 对标差距/未达标项显式登记 g14 差距登记表（不静默混入）；以单轮/混合口径/MRQ 含开销数据冒充正式对标即 RED；画质劣化静默即 RED；未达标冒充达标即 RED（未达标如实登记不阻塞 G14.5a 穷举，商用收口判定归 G15+） | 内联：以单轮/混合口径/MRQ 含开销数据冒充正式对标即 RED；画质劣化静默即 RED；未达标冒充达标即 RED | host+device | **G14.4** | post-interlock actual-next-free allocation |
| **M-e** | `g14.p0.m_e.regression_drift_guard`<br>`py -3 ci/g14_regression_drift_guard_smoke.py --gate g14.p0.m_e.regression_drift_guard` | `milestones/g14/g14_m_e_regression_drift_guard_evidence_schema.json` | 回归门 + 漂移监控：既有 76 门（G9 34 key + G10 14 key + G11 14 key + G12 9 key + G13 5 key）最新 evidence 全绿只读汇总（聚合不遮蔽子断言 FAIL/SKIP/DEV_ENV_DEGRADE）+ G14 触改面既有门重跑回归零降级（M96 golden 门序面真跑抽检 + M-c/M-d 修订门复跑面）+ M165 漂移监控登记（G14 复跑面同型 digest 漂移检出计数/零检出字面入 evidence，FAIL 件 0-byte 保留纪律继承）；既有门降级即 RED；聚合遮蔽即 RED；漂移检出未登记即 RED | 内联：既有门降级即 RED；聚合遮蔽即 RED；漂移检出未登记即 RED | host 纯 host（抽检子进程自持 device 面） | **G14.5a** | post-interlock actual-next-free allocation |

任一行缺失、合并后不可区分、非 `PASS` 或无对应 evidence schema，均阻断 G14.5b（契约 §4.2 末段字面）。**M-d 帧率通过线为 G14 唯一新设通过线**（Rurix ≥ UE 同口径 ×1.00 略高下限字面）；**G14 不设绝对画质通过线**（归 G15 商用收口期）。

---

## 2. 已 go P1 硬门（零行）

G14.1 无 go 的 P1 行——调研与候选决策 5 行实现门（M-a~M-e）全为 P0（契约 §4.2 末段字面）。后续波次若治理程序将新 P1 判为 go，须先按治理程序修订本表及覆盖集合（只追加进 §2）再开对应实现；不得把它静默并入现有 key。本节的机核面 = §2 零行声明与 `ci/g14_acceptance_map_check.py` 的 P1 空集断言（§5）。

---

## 3. 条件型 / not-triggered 登记面

### 3.1 异己并发工作树面（不混入零消费 G14 车道）

立项裁决 1（契约 §7 逐字登记）：G14 带未提交项立项——工作树异己会话 src/ 未提交面（2026-08-19 `git status` 实测：apps/uc06-renderer、apps/uc08-physics、src/rurix-asset/src/lib.rs、src/rurix-render 多面、src/rurix-rt/src/render_exec.rs 改写面 + evidence/d3d12_interop_smoke.json、milestones/g12/g12_pt_sampler_selection.json 异己改写面及各 evidence/ 异己新件）保持不混入 G14 车道、严禁消费（G10.8b §8.10/G11/G12/G13 先例同模）。纪律：G14 车道 commit 只含 G14 车道文件；异己面 evidence 不充 G14 任何门绿；若 G14 实现波与异己面触及同一文件，按只追加程序登记冲突面并请治理裁决（不得静默合并）。

### 3.2 触发评估 / 决策窗登记面（G13 defer 行的 G14 窗）

- **G10-N11 帧率对标口径（G14 兑现窗）**：承接锚「正式帧率对标三轮进程级独立运行 + MRQ 开销剥离口径」——G14 立项本身即「锚定 G14」条件命中；兑现面 = M-b 门判据字面（三轮进程级独立运行 + 开销剥离 measured 量化）。
- **G10-N16 GPU 管线双端 A/B（G14 兑现窗）**：承接锚「GPU 管线双端 A/B 出图与帧率 measured 对标需求成立」——兑现面 = M-d 门判据字面；G13 M-d（GI 模块级对照面）不冒充本面在案（G13.4 边界登记）。
- **G13-N7 FG/MFG（G14 重评窗）**：重评窗结论 = **不立项**——G14 帧率通过线 = 真实渲染帧率（用户「帧率对标UE5略高（不降级画质）」字面——生成帧不计入真实渲染帧率口径，FG 面混淆对标语义）；FG/MFG 独立层立项 = 商用收口后独立期面（G15+ 重评窗顺延）；`registry/deferred.json` RD-041 history 只追加登记。
- **M100-high（G14 窗登记 = 未齐备）**：G14 对标场景面 = cornell-box + bistro-interior 双场景闭集，不新增低档 MegaLights 多灯压测场景——齐备条件未命中，maintain-defer。
- **M114-strand（G14 窗登记 = 数据面部分落地）**：G14 产双端 benchmark measured 数据集但非 M120 精确档（毛发 OIT 档）裁决数据面——档选定程序解冻条件未命中，不以帧率对标数据冒充精确档裁决数据。
- **G11-N3（G14 部分兑现登记）**：M-d 产 GPU 管线双端 A/B 出图面（前半条件命中）；画质差距清单 measured 落地 = G15 画面终审绝对画质通过线期面（后半条件未命中）——锚定 G15 维持，不以帧率对标出图面冒充画质差距清单。
- **M52 / G10-N17 / G11-N5 / G10-N8（G14 窗维持）**：SER 双条件未命中维持；M137 scalars.flip 未消费维持（G14 FLIP 消费 = M-c/M-d 修订门复跑面 g10_flip_lib 直取口径 0-byte）；度量稳健性对照数据集未齐备维持；无头臂未测维持（benchmark/窗口双臂面）。

登记 `SKIP=not-triggered` 只表示决策已记录，不是成功，不充任何门绿。

### 3.3 帧率通过线 / 画质面 / 材质链边界登记面

- **帧率通过线（G14-N7 口径裁决登记）**：M-d 通过线 = Rurix 三轮进程级独立运行 trimmed mean 帧率 ≥ UE 同口径 benchmark 臂 ×1.00（「略高」最保守机器可核下限——用户字面兑现，向上取严无量化授权字面；超出量 measured 登记；逐轮守护带防单轮运气达标）；以单轮/混合口径/MRQ 含开销数据冒充正式对标即 RED（M-d 判据内联）。
- **画质零降级守护面（G13-N8/N9 联动）**：M-d 画质面 = G13 锁定双登记表 8+2 行 deficit 基线带内不劣化（经 M-a 修订后方差带结构化对账面复跑核验）；G14 不设绝对画质通过线（归 G15）；画质劣化静默即 RED。
- **材质链边界（G11-N8/G11-N9/G12-N10/G12-N12 锚定 G15）**：G14 帧率对标若实测透射/焦散/镜面 IBL 类能量为画质量级主差，进 G14 新差距登记表显式归属登记但不承接修复——锚定 G15 画质量级收口面维持；`g12_ue_pt_gap_registry.json` 10 行与 G13 双表 8+2 行终态只消费不回写（G14 新产差距另立 `milestones/g14/` 新表）。

### 3.4 对标测量口径面（G14.2 独立冻结口径）

G14 对标测量参数（场景/输出分辨率/超分档位/采样协议/统计口径）独立冻结——digest 机核，不动 G10.5/G13.4 锁定值（closed 复测对照面 0-byte）；UE 臂 = UE 5.8.1 benchmark 臂（臂 B `-game -benchmark`，RXS-0380 L2 命令面闭集，窗口化形态，G10-N8 口径继承），UE build digest == M128 登记 ue_build_id 机核；统计协议 = 50×3 trimmed mean（M141/M165 冻结口径转引）+ 三轮进程级独立运行（进程冷启动逐轮独立，缓存冷热面登记）；MRQ 开销剥离 = 同场景 MRQ 臂 frameRenderDuration vs benchmark 臂帧时差值 measured 量化（G10-N11 口径字面）。

---

## 4. 双向一致与互斥面（key 命名空间机器可核声明）

1. **双方逐字一致**：本表 §1 五行与 [G14_CONTRACT.md](G14_CONTRACT.md) §4.2 五行对同一 P0 M 行给出的 symbolic gate key 与稳定脚本名**必须逐字相等**，由 `ci/g14_acceptance_map_check.py` 双向比对机器强制；任一处漂移即 FAIL。本声明为机器可核面：比对以文件字面为准，不以叙述替代。（G14 无独立 CI_GATES——G12 三向比对体例精简为契约 ↔ MAP 双向，契约 §4.3 治理三门表为第三冻结面。）
2. **唯一命名空间**：`g14.p0.m_<a~e>.<slug>` + `ci/g14_<slug>_smoke.py` + `milestones/g14/g14_m_<a~e>_<slug>_evidence_schema.json` 为唯一合法形态；G14 命名空间（`g14.*`）与 G9 已消费 34 key（`g9.*`）、G10 已消费 14 key（`g10.*`）、G11 已消费 14 key（`g11.*`）、G12 已消费 9 key（`g12.*`）、G13 已消费 5 key（`g13.*`）互不包含；全部 key 全局唯一，匹配 `g14\.p0\.m_[a-e]\.[a-z0-9_]+`；没有两个 M 行共享 key。
3. **互斥**：M 行与 key 一对一；`no-go`/`defer` 项（如 G14-N6 异己面）不产生 key、不入本表，不得冒充 PASS；G13 defer 维持行（M61/SAFE-GPU/M127 等 22 行）不入本表；P0 集合变更属于契约变更，不得以勘误处理。
4. **防混淆登记**：`g14.p0.m_d.dual_end_fps_parity` 等对拍门消费的 FLIP = 图像感知度量（RXS-0389），与 RD-044 族 FLIP（流体）同名不同物，互不构成触发（G10/G11/G12/G13 口径维持）。
5. **基线锚溯源**：M-b/M-c/M-d 三轮独立运行统计协议（50×3 trimmed mean）转引自 M141/M165 冻结口径（BENCH_PROTOCOL §3）；M-d 画质零降级守护消费的 G13 双登记表 8+2 行与 deficit 标定带为 G13.5b 终审锁定面只消费不回写；`g12_ue_pt_gap_registry.json` 10 行只消费不回写（G14 新差距另立 `milestones/g14/` 新表）；M-a 修订锚 = G13_CONTRACT §8.7 承接锚字面。

---

## 5. G14.1 治理覆盖与空行门

G14.1 治理三门（本波 materialize，步骤按落盘前实测 actual next_free 顺位领取）：

```text
g14.wave.1.acceptance_map         步骤 <actual next_free>
  py -3 ci/g14_acceptance_map_check.py --gate g14.wave.1.acceptance_map

g14.wave.1.candidate_decisions    步骤 <actual next_free+1>
  py -3 ci/g14_candidate_decisions_check.py --gate g14.wave.1.candidate_decisions

g14.gov.implementation_interlock  步骤 <actual next_free+2>
  py -3 ci/g14_interlock_check.py --gate g14.gov.implementation_interlock
```

`ci/g14_acceptance_map_check.py` 的 PASS 判据（coverage + no-empty 两组断言分别独立报告）：

1. P0 行集合与 §1 的 5 项**集合全等**，无遗漏、无额外 P0、无重复；§2 P1 集合为空集（G14.1 零 go P1 字面）。
2. 全部 symbolic key 全局唯一，均匹配 `g14\.p0\.m_[a-e]\.[a-z0-9_]+`；每行只有一个 canonical `assertion_id`，没有两个 M 行共享 key；key 的 m 段字母与行号一致。
3. 每一行均有脚本命令（`--gate` 参数 == canonical key）、evidence schema、可机器求值的 PASS 判据、负例 RED 臂、device/host 性质、最晚波次。
4. **双向一致**：本表 §1 与 `G14_CONTRACT.md` §4.2 对同一 P0 M 行给出的 key 与脚本必须逐字相等；任一处漂移即 FAIL。

no-empty 组的 PASS 判据：

- 逐单元格拒绝空串、空白、`TBD`、`TODO`、`待定`、`待补`、`—`；表中全部行的必填列均非空。
- 所有 schema 路径必须唯一落到对应 M 行，且文件名含同一 `m_<a~e>` 与同一 slug；所有波次属于 `G14.2|G14.3|G14.4|G14.5a` 的非空集合。
- G14.1 只核映射完整性，不把尚未 materialize 的脚本/schema 误判为实现绿色；对应能力实现 PR 合入前或同 PR，`ci/check_schemas.py` 必须能校验该 schema 与实际 evidence。

`ci/g14_candidate_decisions_check.py` 的 PASS 判据：G14_CANDIDATE_DECISIONS 候选行闭集全等（§1 G13 defer 24 行 + §2 open RD 7 行 + §3 G14 新增候选行）+ 裁决枚举合法（go/no-go/defer-to-G15+/strategic_override）+ 零空行（全列非空）+ 承接锚纪律（§1 行承接锚 = G13.5a 字面 0-byte 转引，含「→」分节与 G14+ 承接源字面；§3 行含「重判条件 + 兜底」字面）+ defer-to-G15+ 裁决行 G15+ 重评窗字面（裁决/最终状态列承载，转引列不回写）+ go 行验收映射锚义务（登记留痕位置含 G14_ACCEPTANCE_MAP）+ §2 RD 行条目级 status==open + 与本表 5 key 互斥（候选行 ID 不得命中已 go 门裸 token）。

`ci/g14_interlock_check.py` 的 PASS 判据：逐项读取事实源输出 §6 各条件真值；G14.1 期间必须诚实输出 `BLOCKED`（validator 能识别阻断，不算互锁 PASS 实现面）；仅全部条件为真时才输出 `READY`（`--require-ready` exit 0）；`--gate` 模式产 evidence（VERDICT 字面入档，BLOCKED 不充绿）。

三个 validator 均把每组断言逐条打印，不以一个总 `all_pass=true` 掩盖具体缺行；`--selftest` 用受控负样本证明它们能红。

---

## 6. G14.2 硬互锁

`G14.GOV.G14_2.ENTRY_INTERLOCK` 是 G14.2 的前置 required check（`ci/g14_interlock_check.py --require-ready`，治理三门之一同脚本）。以下条件必须**同时**为真（契约 front matter `implementation_unlock.required_all` 与 G-G14-3 字面展开）：

1. G13 已 closed（`milestones/g13/G13_CONTRACT.md` front matter `status: closed`，2026-08-19，flip commit `f4c8da0b` + tag `g13-closed`），且 G14.0 文档集不可变 ref `f4c8da0b` 已登记。
2. `G14_CANDIDATE_DECISIONS.md` 分项映射无空行；`registry/deferred.json` history 只追加、无静默改判（vs G14.0 base 条目四字段 0-byte）；本表 §1/§2 无缺行（D-G14-3）。
3. §5 的 `g14.wave.1.acceptance_map`（coverage + no-empty 两组）与 `g14.wave.1.candidate_decisions` 独立 PASS。
4. G14 的 P0 实现门 numeric CI step claim 发生在上述互锁通过之后，并以 claim 当下 ledger 实测 `CI_step.next_free` 为起点；每个 symbolic key 一对一分配，未复用、未覆盖、未沿用任何草案建议值（治理三门步骤为 G14.1 实测领取的合法面）。
5. 用户 G14.2 开工指令已留痕（2026-08-19 指令全期授权面——「帧率对标UE5略高（不降级画质）」字面，契约 §7 逐字登记）。

任一条件为假时，互锁必须返回非零；此时禁止合入 G14.2 的 `spec/`、`conformance/`、`src/` 或 workflow 实现改动。不能用 owner override 绕过任何条件，也不能用本表存在本身当作 G14.2 开工许可。`ci/g14_interlock_check.py --require-ready` 输出 READY 是机器事实，不以叙述替代（契约 G-G14-3 字面）。

---

## 7. Close-out 审计

- G14.5a 必须重跑全部 5 个 P0 与所有 go 的 P1 的**各自 assertion**；evidence 的 `base_commit` 必须落在同一候选 close-out 基线，零 skip/estimated；soak 量级沿 G13.5a 继承（≥1800s）或 measured 证明更短足够；`budget_eval --strict` 非空全 PASS；既有 76 门（G9 34 + G10 14 + G11 14 + G12 9 + G13 5）零降级（M-e 门承载）。
- G14.5b 只有在 5 个 P0 key 全 PASS、所有 go 的 P1 key 全 PASS、验收映射/候选决策/RD 最终状态逐字一致、**帧率对标结果终审定盘**（达标/未达标如实登记不冒充；画质零降级守护面终态锁定）时才可 status flip；任一 P0 无独立硬门则禁止 flip。
- 同日放行先例继承（立项裁决 3）：5a full-run 先行完成后允许同日进 close-out；条件实现刚绿不得跳过 soak 直接 close。
- **G14 零绝对画质通过线口径维持到 close-out**：不设绝对画质通过线（归 G15）；任何「已达 UE5 画质」叙述在 G14 期内一律不成立（契约 §5 字面）。**M-d 帧率通过线未达标 = 如实登记**（不冒充达标；按用户授权继续优化面承接——G15 前可按只追加程序新建延续波/G16+ 里程碑）。
- 后续若治理流程将新的 P1 判为 go，须先按治理流程修订本表及覆盖集合，再开对应实现；不得把它静默并入现有 key。

---

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-19 | G14.1 初版：冻结 5 个 P0 的 symbolic gate、目标脚本/schema、独立判据（契约 §4.2 逐字）与最晚波次——登记表 UE 方差带结构化对账修订 1 行（M-a，G14.2，G13 §8.7 承接锚兑现）+ UE benchmark 臂正式帧率测量 1 行（M-b，G14.2，G10-N11 兑现窗）+ Rurix 生产管线性能面 1 行（M-c，G14.3）+ 双端帧率正式对标 + 画质零降级守护 1 行（M-d，G14.4，G10-N16 兑现窗 + 通过线 ×1.00 略高下限）+ 回归门+漂移监控 1 行（M-e，G14.5a）；§2 零 go P1 声明；§3 条件型/not-triggered 登记面（异己并发工作树面 / 八行触发评估与决策窗 / 帧率通过线·画质面·材质链边界 / 对标测量口径面）；§4 key 命名空间双方逐字一致机器可核声明（G14 无独立 CI_GATES，契约 ↔ MAP 双向）+ 基线锚溯源；单一命名空间 `g14.p0.m_<a~e>.<slug>` + `ci/g14_<slug>_smoke.py` + `g14_m_<a~e>_<slug>_evidence_schema.json` 由 `ci/g14_acceptance_map_check.py` 双向比对强制；§5 治理三门（步骤按落盘前实测 actual next_free 领取）、§6 G14.2 硬互锁五条件、§7 Close-out 审计。P0 行数字 CI 步骤全部 `post-interlock actual-next-free allocation`（当前实测 CI_step next_free=247），零 P0 workflow/script/schema 预放。 |

---

## 附录 A. 延续波门（G14.x 只追加程序面——G14_CONTRACT §7 裁决 7 字面；§1 冻结 5 行闭集 0-byte 不动，本附录行不进 G14.5b 阻断闭集，其绿面为其所属延续波退出门输入；非数字节首不参与 §1/§2 行集机核）

| M 行 | Symbolic CI gate key / 稳定脚本 | Evidence schema（目标路径） | 独立硬判据（逐字） | 负例 RED 臂 | device/host 性质 | 所属波次 | numeric_step |
|---|---|---|---|---|---|---|---|
| **M-f** | `g14.p0.m_f.production_caliber_stage_a`<br>`py -3 ci/g14_production_caliber_stage_a_smoke.py --gate g14.p0.m_f.production_caliber_stage_a` | `milestones/g14/g14_m_f_production_caliber_stage_a_evidence_schema.json` | 生产口径双列 + vendor Stage A 位级零漂移（G14.4 调研取证面契约 §8.5 a/d 兑现）：bench receipt 双列口径落盘（frame_ms 全量 G14.3 兼容 + frame_ms_production = 全量 − bench 测量面 tail〔is_finite 全帧校验 + frame_content_digest payload 重建+sha256 = 非生产路径固有面〕）逐格不变量机核 production ≤ full + DLSS 臂 pack 直写 mapped staging（消 ~px·21B 二次 memcpy）与 DLSS/FSR 双臂输出驻留写（消逐帧 ~out_px·12B 分配）位级零漂移——三探针格（cornell-box t67 × 三后端）末帧 digest == g14_3_stage_a_digest_anchor.json 冻结锚逐字一致 + RURIX_VENDOR_TIMING=1 分解遥测六段（pack/sl_book/upload/evaluate/submit_wait/readback）measured 非空（evaluate 黑盒段裁决登记——G14.4 调研 R1 动作）+ Stage A 前后全量口径对照行（pre 锚 = M-d 012652Z evidence）+ 三探针格 production 口径 measured 入 g14_budget（阈 = 实测 ×1.5 守护带程序产禁手写 P-09）+ budget_eval 全 PASS；以全量口径冒充生产口径即 RED；Stage A 输出漂移静默即 RED；estimated 冒充 measured 即 RED | 内联：以全量口径冒充生产口径即 RED；Stage A 输出漂移静默即 RED；estimated 冒充 measured 即 RED | host+device | **G14.6** | 257（落盘前实测 actual next_free 顺位领取） |
| **M-g** | `g14.p0.m_g.vendor_parallel_conversion`<br>`py -3 ci/g14_vendor_parallel_conversion_smoke.py --gate g14.p0.m_g.vendor_parallel_conversion` | `milestones/g14/g14_m_g_vendor_parallel_conversion_evidence_schema.json` | vendor 转换并行化位级零漂移 + 同码 A/B measured（G14.4 调研取证面契约 §8.5 d 条兑现——vendor host 转换面）：vendor_upscale.rs 四区打包（color f16/depth/mv/reactive）与双臂输出回读转换（DLSS 连续 RGBA / FSR 行距对齐）改像素带并行（std::thread::scope 带切分，元素零依赖，带内逐值同式同序——输出字节面与单带串行逐位一致）+ fsr-dbg 逐帧诊断打印门控（FSR_DBG_STAGE 置位才打印，CI 零消费面）+ 位级零漂移三机核（三探针格〔bistro-interior t67 dlss_sr/fsr_3_1_5 并行面 + cornell-box t67 dlss_sr 阈下单带面〕末帧 digest == g14_3_stage_a_digest_anchor.json 冻结锚 + RURIX_VENDOR_PAR=0 串行对照臂 bistro t67 dlss digest 同锚〔并行 ≡ 串行 ≡ 锚 三角机核〕+ Rust 函数面 g14_7_parallel_conversion_bitexact 单测真跑绿）+ 同码 A/B measured（bistro t67 dlss RURIX_VENDOR_TIMING=1 交错四跑〔PAR=0×2 + 缺省并行×2〕30 帧 warmup 10 稳态 mean——pack 与 readback 双段并行臂 mean < 串行臂 mean 方向机核，改善量 measured 登记不设先验阈值 P-09）+ 双探针格（bistro t67 dlss/fsr）production 口径 measured 入 g14_budget（阈 = 实测 ×1.5 守护带程序产禁手写 P-09）+ budget_eval 全 PASS；并行输出漂移静默即 RED；以串行臂冒充并行改善（direction 伪报）即 RED；estimated 冒充 measured 即 RED | 内联：并行输出漂移静默即 RED；以串行臂冒充并行改善（direction 伪报）即 RED；estimated 冒充 measured 即 RED | host+device | **G14.7** | 259（落盘前实测 actual next_free 顺位领取） |
| **M-h** | `g14.p0.m_h.continuation_closeout`<br>`py -3 ci/g14_continuation_closeout_smoke.py --gate g14.p0.m_h.continuation_closeout` | `milestones/g14/g14_m_h_continuation_closeout_evidence_schema.json` | G14plus 延续波收口门（G14PLUS_RECORD §2 波序/RFC-0030 §4.7 锚重收割程序/G14_P2_DECISIONS 表后事件登记〔G14plus 立项条〕兑现）：digest 锚重收割合法性三证（`g14_3_stage_a_digest_anchor.json` 顶层 reharvest 字段完备〔harvested_utc/source_gate_run/base_commit/double_harvest_bitexact=true——同格双收割位级同值证〕+ 收割前置最新 M-c evidence status=pass ∧ double_run_bitexact=true + 新锚下最新 M-d evidence 时间戳晚于重收割时点 ∧ stage_a_digest_drift_guard=true〔18 格 × 3 轮全矩阵对新锚零漂移由 M-d 门本体承载，本门只读消费〕）+ M-d 18/18 达标（最新 M-d evidence status=pass ∧ met_count=18 ∧ unmet_count=0——通过线 ×1.00 全达标）+ fps 差距登记表空表终态（items=[] ∧ 双场景 no_gap_explicit=true 显式登记）+ RD-045 修复/缓解登记完备（history 只追加含 G14plus 修复/缓解条目 ∧ 条目 status=open 维持——长窗观察归 G15+）+ G14plus 波验收记录在树（契约 §8.8/§8.9/§8.10/§8.11 标题机核 = 波0 治理立项/G14.8/G14.9/G14.10 四恒发生波，§8.12 = G14.11 结构条件波有无如实登记不阻断，§8.13 = G14.12 收口记录本门之后才写不自锚）+ RED 双臂（anchor-tamper 手写 digest 形态注入必检出 / unmet-masquerade 达标伪报必拒绝，函数面真跑）；锚手写/篡改冒充程序收割即 RED；达标伪报即 RED；重收割未发生冒充新锚即 RED | 内联：锚手写/篡改冒充程序收割即 RED；达标伪报（unmet 非空冒充 18/18）即 RED；重收割未发生（reharvest 字段缺失）冒充新锚即 RED | host 只读（消费 M-c/M-d device 真跑面） | **G14.12** | 265（落盘前实测 actual next_free 顺位领取） |

- 附录 A 行纪律：与 §1 同构（独立 symbolic key + 独立 evidence subject + 独立布尔断言）；§1 冻结 5 行集合机核面（`ci/g14_acceptance_map_check.py` EXPECTED_P0）0-byte 不扩——延续波门由契约 §8 波验收记录与本附录双向引用承载一致性，不混入 G14.5b 阻断闭集；后续延续波门只追加进本附录。
