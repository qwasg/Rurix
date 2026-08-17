<!-- Assisted-by: Kimi-K3（G12.1 治理波起草） -->
# G12_ACCEPTANCE_MAP — P0 / 已 go P1 验收映射

> **性质**：G12.1 治理交付物（governance-only）；事实源为 [G12_CONTRACT.md](G12_CONTRACT.md) v1.0 front matter acceptance_gates（G-G12-1~10）与 §4.2 八行 P0 独立断言表、[G12_PLAN.md](G12_PLAN.md) v1.0 §2 各波退出门草案与 §3 P0 建议清单、[G12_CANDIDATE_DECISIONS.md](G12_CANDIDATE_DECISIONS.md) v1.0（§3 新增候选 → M158~M166 映射，§1 G11 defer 19 行处置，§2 RD-040 nrd 评估窗承接）。
> **编号纪律**：本表只冻结 symbolic CI gate key 与稳定脚本名，不 claim 数字 CI step。数字步骤必须等 G-G12-3 实现互锁（§6）通过后，再从 `registry/number_ledger.json` 当时实测的 `CI_step.next_free` 顺位分配——**numeric_step 一律写 `post-interlock actual-next-free allocation`**（当前实测 `CI_step.next_free=217`，G11 已消费至 216，[G11 CI_GATES](../g11/CI_GATES.md) v1.9 / ledger v1.123）；禁止沿用任何草案建议值。
> **证据纪律**：表内脚本与 schema 是实现 PR 的强制目标路径；二者须与对应 RED→GREEN 实现同 PR 落地。路径尚未 materialize 只表示未开工，不能记 PASS；本波不预建空脚本、空 schema 壳或占位 workflow 步骤（只冻结路径）。

---

## 1. P0 硬门（精确 8 行）

- P0 精确集合（8 行）：`{M158,M159,M160,M161,M162,M163,M164,M165}`。每一行的 symbolic key 同时是该能力唯一的 `assertion_id`，与 [G12_CONTRACT.md](G12_CONTRACT.md) §4.2 **逐字一致**（key 命名空间三方一致性机核面，禁止任何改写）；独立硬判据列同逐字。实现后的 evidence 顶层必须至少含：`schema_version`、`subject`、`milestone`、`wave`、`assertion_id`、`status`、`commands`、`environment`、`base_commit`、`run_url`、`timestamp`；其中 `assertion_id` 必须等于本表 key，`status` 必须为 `pass|fail`，不得以 `skip|estimated|advisory` 充绿（条件未触发只能登记 `not-triggered`、环境缺失只能登记 `dev_env_degrade`，见 §3）。
- 每个 symbolic key 最终一对一取得一个 numeric CI step。脚本可复用，但 workflow 必须按 `--gate <symbolic-key>` 独立调用、独立产 evidence、独立给结论；任一 P0 assertion 缺失或失败，只能使该 P0 为红，**不得用同脚本内另一 assertion 的绿色代替**。一次 smoke 可共享进程启动成本，但每行必须单独产出 `PASS|FAIL|SKIP|DEV_ENV_DEGRADE`；只有 `PASS` 满足 P0。
- **单一 key/脚本命名空间**：symbolic key 一律小写点分 `g12.p{0,1}.m<###>.<slug>`，脚本一律 `ci/g12_<slug>_smoke.py`，evidence schema 一律 `milestones/g12/g12_m<###>_<slug>_evidence_schema.json`（slug 与 key 末段同字面，只冻结路径不预建文件）。本表、[G12_CONTRACT.md](G12_CONTRACT.md) §4.2 与 [CI_GATES.md](CI_GATES.md) §4 引用同一份 key/脚本，由 `ci/check_g12_acceptance_map.py` 三向比对强制一致（见 §4）。
- device 判据统一要求真实路径、`RURIX_REQUIRE_REAL=1`、validation error 为 0；mock、stub、host substitution、仅检查非零输出、缺 provisioning 的 SKIP 均不满足能力门。host oracle、既有最小见证、人工截图均不能替代目标门。
- **生产化判据统一形态**（M158~M162 五行生产化门共用字面）：生产化落盘（只消费 M96 参照器冻结面 + 候选决策表对应行）+ 正确性锚 0-byte（M96 既有判据/固定 seed 位级确定性协议/golden 门序 D2-Q7）+ 收敛/方差/噪声面 measured 不劣于参照器基线锚（容差由 G12.2 标定程序 measured 产出禁手写；或演进位显式登记即 RED 评审面）+ 不降级既有 62 门绿面。**G12 不设绝对 UE PT 画质通过线**——「已达 UE5 PT 画质」判定归 G15 商用收口期（契约 §1/§5 字面）。
- 负例 RED 臂列：「内联」= 契约 §4.2 判据字面中已含的 RED 臂（逐字摘录，与判据列同源）；「PLAN §3 草案补充」= [G12_PLAN.md](G12_PLAN.md) §3 草案登记的额外臂，**不并入契约判据字面**，进 validator 时以契约字面为准、草案臂须经治理程序硬化后方可机核。

| M 行 | Symbolic CI gate key / 稳定脚本 | Evidence schema（目标路径，只冻结不预建） | 独立硬判据（契约 §4.2 逐字） | 负例 RED 臂 | device/host 性质 | 最晚波次 | numeric_step |
|---|---|---|---|---|---|---|---|
| **M158** | `g12.p0.m158.mis_full_surface`<br>`py -3 ci/g12_mis_full_surface_smoke.py --gate g12.p0.m158.mis_full_surface` | `milestones/g12/g12_m158_mis_full_surface_evidence_schema.json` | MIS 完整面生产化：光源采样（NEE）× BSDF 采样 MIS 权重全路径覆盖 + 能量守恒（白炉 + 逐级能量增量单调不增，RXS-0395 口径继承）+ 同 spp 收敛曲线不劣于参照器基线锚（g12_budget pt.ref_curve 锚，容差标定程序产）+ 固定 seed 位级确定性协议继承 + M96 既有判据 0-byte；权重缺失冒充 MIS 即 RED；能量偏置注入即 RED；收敛劣化冒充升级即 RED；确定性协议漂移即 RED | 内联：权重缺失冒充 MIS 即 RED；能量偏置注入即 RED；收敛劣化冒充升级即 RED；确定性协议漂移即 RED | host+device | **G12.2** | post-interlock actual-next-free allocation |
| **M159** | `g12.p0.m159.russian_roulette_prod`<br>`py -3 ci/g12_russian_roulette_prod_smoke.py --gate g12.p0.m159.russian_roulette_prod` | `milestones/g12/g12_m159_russian_roulette_prod_evidence_schema.json` | 俄罗斯轮盘生产化：吞吐自适应 RR（路径吞吐权重驱动终止概率）+ 无偏补偿（补偿因子闭式）+ 最小反弹保障（低深度不早杀）+ RR 终止率/补偿计数非空 + 收敛曲线不劣于基线锚；早杀偏置注入即 RED；补偿缺失冒充无偏即 RED；跳 RR 偏移未检出即 RED（RXS-0357 三臂 RED 面继承） | 内联：早杀偏置注入即 RED；补偿缺失冒充无偏即 RED；跳 RR 偏移未检出即 RED | host+device | **G12.2** | post-interlock actual-next-free allocation |
| **M160** | `g12.p0.m160.sampling_lds_upgrade`<br>`py -3 ci/g12_sampling_lds_upgrade_smoke.py --gate g12.p0.m160.sampling_lds_upgrade` | `milestones/g12/g12_m160_sampling_lds_upgrade_evidence_schema.json` | 采样策略升级 + 低差异序列：分层/低差异序列生产化 + 确定性协议扩展（序列索引确定性 + 固定 seed 位级一致维持 + RNG 流布局 provenance）+ 收敛曲线 measured 不劣于独立 PCG 流锚；序列非确定冒充低差异即 RED；位级一致破坏未登记即 RED；收敛劣化冒充升级即 RED | 内联：序列非确定冒充低差异即 RED；位级一致破坏未登记即 RED；收敛劣化冒充升级即 RED | host+device | **G12.2** | post-interlock actual-next-free allocation |
| **M161** | `g12.p0.m161.convergence_criterion_prod`<br>`py -3 ci/g12_convergence_criterion_prod_smoke.py --gate g12.p0.m161.convergence_criterion_prod` | `milestones/g12/g12_m161_convergence_criterion_prod_evidence_schema.json` | 收敛判据生产化：逐像素方差驱动自适应 spp 终止 + 收敛报告（逐像素 spp 分布/方差/未收敛像素计数非空）+ 收敛误判率 ≤ 标定阈（标定程序产禁手写）+ 固定全 spp golden 对拍不偏离冻结带（measured×2.0 带继承）；早停冒充收敛即 RED；未收敛像素缺报即 RED；golden 偏离冻结带即 RED | 内联：早停冒充收敛即 RED；未收敛像素缺报即 RED；golden 偏离冻结带即 RED | host+device | **G12.2** | post-interlock actual-next-free allocation |
| **M162** | `g12.p0.m162.denoise_pipeline_tsr`<br>`py -3 ci/g12_denoise_pipeline_tsr_smoke.py --gate g12.p0.m162.denoise_pipeline_tsr` | `milestones/g12/g12_m162_denoise_pipeline_tsr_evidence_schema.json` | 降噪管线 + TSR 联动：时域/空域降噪管线落地 + 噪声谱高频能量下降 measured（标定阈）+ 帧均值能量守恒容差内（不引入系统性变暗/变亮偏置）+ temporal 底座 0-byte 断言 + NRD 类 vendor 降噪评估报告落盘（评估不接线）+ golden 对拍面不降级；降噪引入系统性偏置即 RED；temporal 底座接线即 RED；评估冒充接入即 RED；噪声底未降冒充降噪即 RED | 内联：降噪引入系统性偏置即 RED；temporal 底座接线即 RED；评估冒充接入即 RED；噪声底未降冒充降噪即 RED | host+device | **G12.3** | post-interlock actual-next-free allocation |
| **M163** | `g12.p0.m163.ue_pt_parity`<br>`py -3 ci/g12_ue_pt_parity_smoke.py --gate g12.p0.m163.ue_pt_parity` | `milestones/g12/g12_m163_ue_pt_parity_evidence_schema.json` | UE Path Tracer 对标：同场景同 spp 双端出图（UE build digest == M128 登记 ue_build_id 机核；契约 digest 独立冻结，不等仍出报告即 RED）+ 收敛曲线逐段 measured 对拍（容差标定程序产）+ 噪声谱对拍 + 能量守恒对拍 + UE PathTracing 模块归属差距登记表落盘（差距项显式登记即 RED 评审面）；不设绝对通过线；逐段对拍超容差静默即 RED；差距项静默混入即 RED；单端缺帧聚合 PASS 即 RED | 内联：契约 digest 不等仍出报告即 RED；逐段对拍超容差静默即 RED；差距项静默混入即 RED；单端缺帧聚合 PASS 即 RED | host+device | **G12.4** | post-interlock actual-next-free allocation |
| **M164** | `g12.p0.m164.regression_guard`<br>`py -3 ci/g12_regression_guard_smoke.py --gate g12.p0.m164.regression_guard` | `milestones/g12/g12_m164_regression_guard_evidence_schema.json` | 生产化回归门：既有 62 门（G9 34 key + G10 14 key + G11 14 key）最新 evidence 全绿只读汇总 + 生产化触改面既有门重跑回归零降级（M96 golden 门序面真跑抽检）；既有门降级即 RED；聚合遮蔽子断言 FAIL/SKIP/DEV_ENV_DEGRADE 即 RED | 内联：既有门降级即 RED；聚合遮蔽子断言 FAIL/SKIP/DEV_ENV_DEGRADE 即 RED | host 纯 host | **G12.4** | post-interlock actual-next-free allocation |
| **M165** | `g12.p0.m165.pt_throughput_baseline`<br>`py -3 ci/g12_pt_throughput_baseline_smoke.py --gate g12.p0.m165.pt_throughput_baseline` | `milestones/g12/g12_m165_pt_throughput_baseline_evidence_schema.json` | PT 吞吐优化基线：吞吐基线 measured（rays/sec + 帧时 at 固定 spp × 场景集，50×3 trimmed mean 协议）入 g12_budget provenance 齐备 + 不设通过线登记 + 优化前后正确性锚（固定 seed digest 0-byte 或演进位显式登记）；基线冒充帧率对标即 RED；digest 漂移未登记即 RED；estimated 冒充 measured 即 RED | 内联：基线冒充帧率对标即 RED；digest 漂移未登记即 RED；estimated 冒充 measured 即 RED | host+device | **G12.5** | post-interlock actual-next-free allocation |

任一行缺失、合并后不可区分、非 `PASS` 或无对应 evidence schema，均阻断 G12.7b（契约 §4.2 末段字面）。**G12 不设绝对 UE PT 画质通过线**——生产化判据 = 正确性锚 0-byte + measured 不劣于参照器基线锚（契约 §1/§5 / 立项裁决 3），本表不设任何绝对画质 FLIP/SSIM/收敛通过线。

---

## 2. 已 go P1 硬门（一行：M166）

契约 §4.2 末段：「M166（PT 生产化标定）为 P1，入验收映射随主门核验」——本行随主门（G12.2 波聚合门）核验，key/脚本/schema 命名空间与 §1 同构（本表冻结；契约 §4.2 不载 P1 行，三向比对不含 P1，MAP §2 ↔ [CI_GATES.md](CI_GATES.md) §4A 双向比对强制）。判据事实源 = [G12_PLAN.md](G12_PLAN.md) §2 G12.2 退出门草案与 §3 建议清单口径 + 契约 acceptance_gates G-G12-4 字面 + [G12_CANDIDATE_DECISIONS.md](G12_CANDIDATE_DECISIONS.md) §3 G12-N9 行。`numeric_step` 一律 `post-interlock actual-next-free allocation`，待门脚本/schema/workflow 步骤 materialize 时按落盘实测 `next_free` 顺位回填；本节不预建空脚本、空 schema 壳或占位 workflow 步骤。

| M 行 | Symbolic CI gate key / 稳定脚本 | Evidence schema（目标路径，只冻结不预建） | 精确 PASS 判据（本行独立 assertion） | 负例 RED 臂 | device/host 性质 | 最晚波次 | numeric_step |
|---|---|---|---|---|---|---|---|
| **M166** | `g12.p1.m166.pt_production_calibration`<br>`py -3 ci/g12_pt_production_calibration_smoke.py --gate g12.p1.m166.pt_production_calibration` | `milestones/g12/g12_m166_pt_production_calibration_evidence_schema.json` | PT 生产化标定：生产化闭门槛值标定集（方差削减比/收敛误判率/噪声底标定值——标定样本集下界 + digest 入 evidence）+ 标定程序可复跑（两跑逐位一致）+ 标定值按 M138 同程序（p100×k measured）入 `g12_budget.json`（measured_local）且 provenance 齐备（P-09，禁手写阈值） | 手写阈值冒充标定即 RED；estimated 冒充 measured 即 RED；标定程序不可复跑即 RED；样本集低于下界冒充有效标定即 RED | host 纯 host | **G12.2** | post-interlock actual-next-free allocation |

---

## 3. 条件型 / not-triggered 登记面

### 3.1 异己并发工作树面（不混入零消费 G12 车道）

立项裁决 1（契约 §7 逐字登记）：G12 带未提交项立项——工作树异己会话 src/ 未提交面（rurix-asset/rurix-render geometry/gi/shadow/ssr/ktx2_read/hzb/restir/sdf_trace/smrt 声明面，含 untracked `src/rurix-render/src/gi/restir.rs`——ReSTIR 相关面）保持不混入 G12 车道、严禁消费（G10.8b §8.10/G11 先例同模）。纪律：G12 车道 commit 只含 G12 车道文件；异己面 evidence 不充 G12 任何门绿；M100-high 重评不消费异己 restir 面；若 G12 实现波与异己面触及同一文件，按只追加程序登记冲突面并请治理裁决（不得静默合并）。

### 3.2 触发评估登记面（四行 defer 的 G12 窗）

- **M52 SER（G12.2 复评窗）**：G12.1 重评窗核验 = 双条件未命中 maintain-defer（真实集成需求未至〔治理波零实现〕+ capability rt.ser 设备面未实测〔树内零探针〕，契约 §7 裁决 5）；G12.2 生产化核心波 materialize 高分歧 RT workload 集成面时按只追加程序复评——命中则独立 Full RFC 评估（契约 §7 裁决 4），未命中维持 defer 顺延 G13+。
- **M100-high（G12.4 触发评估）**：G12.4 UE PT 对标波若产多灯 workload measured 对照面（bistro 4+ 点光 + emissive 双端 PT 对拍），G12.6 穷举按只追加程序重判；未产出维持 defer 锚定 G14（承接锚字面 0-byte）。
- **G10-N17 M137 scalars.flip（G12.4 触发评估）**：G12.4 对标若消费 diff 报告 FLIP 标量面，按 RXS-0388 L3 演进位程序翻转实值并回归 M137 门；未消费维持 null 演进位。
- **G11-N5 度量口径修订评估（G12.6 触发评估）**：SSIM/FLIP 标量面对低反照率暗帧场景稳健性 measured 对照数据集齐备则按只追加程序重判（演进须 RFC 显式修订行 + 既有门回归零降级）；未齐备维持 defer。

四行登记 `SKIP=not-triggered` 只表示决策已记录，不是成功，不充任何门绿。

### 3.3 材质链边界登记面（G12-N10 / G11-N8 / G11-N9）

M96 起步范围冻结维持（焦散/体积/specular 链 out，RXS-0357 L1 0-byte——契约 §7 裁决 6）：G12 生产化限方差削减/收敛/降噪/对标面，不扩材质链。G12.4 对标若实测透射/焦散/镜面 IBL 类能量为画质量级主差，差距登记表显式归属登记（UE PathTracing 模块归属面）但不承接修复——G11-N8/G11-N9/G12-N10 锚定 G15 画质量级收口面维持。

### 3.4 PT 对标契约面（G12.4 独立冻结口径）

G12.4 对标契约参数（场景/相机/光照/spp 序列/种子）独立冻结——digest 机核，不动 G10.5/G11.5b 锁定值（G10/G11 closed 复测对照面 0-byte）；UE 臂 = UE 5.8.1 Path Tracer MRQ 臂（窗口模式主路，G10-N8/N9 口径继承），UE build digest == M128 登记 ue_build_id 机核；曝光/位深口径沿 G11.2 对齐口径（RXS-0385 strip-and-log / EV100 派生链），残余口径差显式登记（未对齐口径消费对拍 delta 即 RED——R-G12-5）。

---

## 4. 互斥与对账面（key 命名空间三方逐字一致机器可核声明）

1. **三方逐字一致**：本表 §1 八行、[G12_CONTRACT.md](G12_CONTRACT.md) §4.2 八行、[CI_GATES.md](CI_GATES.md) §4 八行对同一 P0 M 行给出的 symbolic gate key 与稳定脚本名**必须逐字相等**，由 `ci/check_g12_acceptance_map.py` 三向比对机器强制；任一处漂移即 FAIL。已 go P1 行做本表 §2 与 [CI_GATES.md](CI_GATES.md) §4A **双向**逐字比对（契约 §4.2 不载 P1 行）。本声明为机器可核面：比对以文件字面为准，不以叙述替代。
2. **唯一命名空间**：`g12.p{0,1}.m<###>.<slug>` + `ci/g12_<slug>_smoke.py` + `milestones/g12/g12_m<###>_<slug>_evidence_schema.json` 为唯一合法形态；G12 命名空间（`g12.*`）与 G9 已消费 34 key（`g9.*`）、G10 已消费 14 key（`g10.*`）及 G11 已消费 14 key（`g11.*`）互不包含；全部 key 全局唯一，匹配 `g12\.p[01]\.m\d{3}\.[a-z0-9_]+`；没有两个 M 行共享 key。
3. **互斥**：M## 与 key 一对一；`no-go`/`defer` 项（如 G12-N10 材质链、G12-N11 异己面）不产生 key、不入本表，不得冒充 PASS；G11 defer 维持行（M61/SAFE-GPU/M127 等 19 行）不入本表；P0 集合变更属于契约变更，不得以勘误处理。
4. **防混淆登记**：`g12.p0.m163.ue_pt_parity` 等对拍门消费的 FLIP = 图像感知度量（RXS-0389），与 RD-044 族 FLIP（流体，[G9_P2_DECISIONS.md](../g9/G9_P2_DECISIONS.md) §1 RD044-fluid 行）同名不同物，互不构成触发（G10/G11 口径维持）。
5. **基线锚溯源**：M158~M161 各行判据内嵌的参照器基线锚转引自 [`g9_m96_pbrt_tolerance_band.json`](../g9/g9_m96_pbrt_tolerance_band.json)（M96 冻结批 measured 曲线值——cornell spp=16 `1.2422206054630583e-1` / cornell spp=64 `9.022782888026709e-2` / direct spp=16 `5.1394150300600565e-2` / direct spp=64 `2.8991059755816284e-2`，curve_rurix 列）——冻结带 0-byte 只消费不回写；G12.2 标定程序产出的生产化容差入 `g12_budget.json` 新条目（沿 G10.4 M138/G11.2 M157 追加先例，禁手写）。

---

## 5. G12.1 治理覆盖与空行门

G12.1 必须提供不占 numeric CI step 的 guardrail（脚本名与 [CI_GATES.md](CI_GATES.md) §3 同一份，属 `check_*` 未编号守卫）：

```text
g12.gov.acceptance_coverage
  py -3 ci/check_g12_acceptance_map.py

g12.gov.implementation_interlock
  py -3 ci/check_g12_implementation_interlock.py

g12.gov.measured_baseline
  py -3 ci/budget_eval.py
```

`ci/check_g12_acceptance_map.py` 的 PASS 判据（coverage + no-empty 两组断言分别独立报告）：

1. P0 行集合与 §1 的 8 项**集合全等**，无遗漏、无额外 P0、无重复；已 go P1 行集合与 §2 声明集合 `{M166}` 全等。
2. 全部 symbolic key 全局唯一，均匹配 `g12\.p[01]\.m\d{3}\.[a-z0-9_]+`；每行只有一个 canonical `assertion_id`，没有两个 M 行共享 key。
3. 每一行均有脚本命令、evidence schema、可机器求值的 PASS 判据、负例 RED 臂、device/host 性质、最晚波次；共享脚本必须使用不同的 `--gate` 参数。
4. **三向一致**：本表 §1、`G12_CONTRACT.md` §4.2 与 `CI_GATES.md` §4 对同一 P0 M 行给出的 key 与脚本必须逐字相等；任一处漂移即 FAIL。已 go P1 行做本表 §2 与 `CI_GATES.md` §4A **双向**逐字比对。

no-empty 组的 PASS 判据：

- 逐单元格拒绝空串、空白、`TBD`、`TODO`、`待定`、`待补`、`—`；表中全部行的必填列均非空。
- 所有 schema 路径必须唯一落到对应 M 行，且文件名含同一 `m###` 与同一 slug；所有波次属于 `G12.2|G12.3|G12.4|G12.5` 的非空集合。
- G12.1 只核映射完整性，不把尚未 materialize 的脚本/schema 误判为实现绿色；对应能力实现 PR 合入前或同 PR，`ci/check_schemas.py` 必须能校验该 schema 与实际 evidence。

`ci/check_g12_implementation_interlock.py` 的 PASS 判据：逐项读取事实源输出 §6 各条件真值；G12.1 期间必须诚实输出 `BLOCKED`（`--expect-blocked` 只证明 validator 能识别阻断，不算互锁 PASS）；仅全部条件为真时才输出 `READY`（`--require-ready` exit 0）。`ci/budget_eval.py` 的 PASS 判据：`g12_budget.json` 非空、`evidence_level=measured_local`、零 `estimated`，counter 与 evaluator 同步；baseline 只证明测量已建立，不得声称实现性能通过。

两个 validator 均把每组断言逐条打印，不以一个总 `all_pass=true` 掩盖具体缺行；`--selftest` 用受控负样本证明它们能红。治理 evidence schema 与实现期 evidence 同 PR 落，不预建空壳。

---

## 6. G12.2 硬互锁

`G12.GOV.G12_2.ENTRY_INTERLOCK` 是 G12.2 的前置 required check；它属于 `check_*` 治理守卫，不占 numeric CI step。以下条件必须**同时**为真（契约 front matter `implementation_unlock.required_all` 与 G-G12-3 字面展开）：

1. G11 已 closed（`milestones/g11/G11_CONTRACT.md` §8.8 `status: closed`，2026-08-17，flip commit `51279d45` + 回归刷新批 `5ae83aa7`），且 G12.0 文档集不可变 ref `5ae83aa7` 已登记。
2. Full RFC-0029（G12 路径追踪生产化伞形）经 D-409 独立 provenance 对抗性评审后 Agent Approved；RFC 编号按立项时实测 `registry/number_ledger.json` namespaces.RFC `next_free` 领取，登记与 README/ledger 一致。
3. `G12_CANDIDATE_DECISIONS.md` 分项映射无空行；`registry/deferred.json` history 只追加、无静默改判；本表 §1/§2 无缺行（D-G12-3）。
4. §5 的 `g12.gov.acceptance_coverage`（coverage + no-empty 两组）独立 PASS；`g12.gov.measured_baseline` PASS（RTX 4070 Ti measured baseline 非空、零 estimated）。
5. G12 的 numeric CI step claim 发生在上述互锁通过之后，并以 claim 当下 ledger 实测 `CI_step.next_free` 为起点；每个 symbolic key 一对一分配，未复用、未覆盖、未沿用任何草案建议值。
6. 用户 G12.2 开工指令已留痕（2026-08-15 指令全期授权面——「支持 dlss、超分采样、路径追踪等前沿技术」字面，契约 §7 逐字登记）。

任一条件为假时，互锁必须返回非零；此时禁止合入 G12.2 的 `spec/`、`conformance/`、`src/` 或 workflow 实现改动。不能用 owner override 绕过任何条件，也不能用本表存在本身当作 G12.2 开工许可。`check_g12_implementation_interlock --require-ready` 输出 READY 是机器事实，不以叙述替代（契约 G-G12-3 字面）。

---

## 7. Close-out 审计

- G12.7a 必须重跑全部 8 个 P0 与已 go P1（M166）的**各自 assertion**；evidence 的 `base_commit` 必须落在同一候选 close-out 基线，零 skip/estimated；soak 量级沿 G11.7a 继承（≥1800s）或 measured 证明更短足够（具体阈值 G12.1 裁决 measured 标定，PLAN §2 G12.7a 字面）；`budget_eval --strict` 非空全 PASS。
- G12.7b 只有在 8 个 P0 key 全 PASS、已 go P1 key 全 PASS、验收映射/候选决策/RD 最终状态逐字一致、**生产化差距清单终审锁定**（UE PT 对标残余差距/未闭环行如实登记不冒充全闭环）时才可 status flip；任一 P0 无独立硬门则禁止 flip（PLAN §2.9）。
- 同日放行先例继承（立项裁决 7）：7a full-run 先行完成后允许同日进 7b close-out；条件实现刚绿不得跳过 7a 直接 close。
- **G12 生产化判据 + 零绝对通过线口径维持到 close-out**：生产化判据 = 正确性锚 0-byte + measured 不劣于参照器基线锚；任何「已达 UE5 PT 画质」叙述在 G12 期内一律不成立（契约 §5 字面）。
- 后续若治理流程将新的 P1 判为 go，须先按治理流程修订本表及覆盖集合，再开对应实现；不得把它静默并入现有 key。

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-17 | G12.1 初版：冻结 8 个 P0 的 symbolic gate、目标脚本/schema、独立判据（契约 §4.2 逐字）与最晚波次——生产化核心 4 行（M158~M161，G12.2）+ 降噪 1 行（M162，G12.3）+ 对标与回归 2 行（M163/M164，G12.4）+ 性能基线 1 行（M165，G12.5）；已 go P1 一行（M166 pt_production_calibration）同构登记；§3 条件型/not-triggered 登记面（异己并发工作树面 / 四行触发评估 / 材质链边界 / PT 对标契约面）；§4 key 命名空间三方逐字一致机器可核声明 + 基线锚溯源表；单一命名空间 `g12.p{0,1}.m###.<slug>` + `ci/g12_<slug>_smoke.py` + `g12_m###_<slug>_evidence_schema.json` 由 `ci/check_g12_acceptance_map.py` 三向比对强制；§5 治理覆盖与空行门、§6 G12.2 硬互锁六条件、§7 Close-out 审计。数字 CI 步骤全部 `post-interlock actual-next-free allocation`（当前实测 CI_step next_free=217），零 workflow/script/schema 预放。 |
