<!-- Assisted-by: Kimi-K3（G10.1 治理波起草） -->
# G10_ACCEPTANCE_MAP — P0 / 已 go P1 验收映射

> **性质**：G10.1 治理交付物（governance-only）；事实源为 [G10_CONTRACT.md](G10_CONTRACT.md) v1.0 front matter acceptance_gates（G-G10-1~11）与 §4.2 十二行 P0 独立断言表、[G10_PLAN.md](G10_PLAN.md) v1.0 §2 各波退出门草案与 §3 P0 建议清单、[G10_CAPABILITY_MATRIX.md](G10_CAPABILITY_MATRIX.md) v1.0。
> **编号纪律**：本表只冻结 symbolic CI gate key 与稳定脚本名，不 claim 数字 CI step。数字步骤必须等 G-G10-3 实现互锁（§6）通过后，再从 `registry/number_ledger.json` 当时实测的 `CI_step.next_free` 顺位分配——**numeric_step 一律写 `post-interlock actual-next-free allocation`**（当前实测 `CI_step.next_free=173`，G9 已消费至 172，[G9 CI_GATES](../g9/CI_GATES.md) v1.21）；禁止沿用任何草案建议值。
> **证据纪律**：表内脚本与 schema 是实现 PR 的强制目标路径；二者须与对应 RED→GREEN 实现同 PR 落地。路径尚未 materialize 只表示未开工，不能记 PASS；本波不预建空脚本、空 schema 壳或占位 workflow 步骤（只冻结路径）。

---

## 1. P0 硬门（精确 12 行）

- P0 精确集合（12 行）：`{M128,M129,M130,M131,M132,M134,M135,M136,M137,M139,M140,M141}`。每一行的 symbolic key 同时是该能力唯一的 `assertion_id`，与 [G10_CONTRACT.md](G10_CONTRACT.md) §4.2 **逐字一致**（key 命名空间三方一致性机核面，禁止任何改写）；独立硬判据列同逐字。实现后的 evidence 顶层必须至少含：`schema_version`、`subject`、`milestone`、`wave`、`assertion_id`、`status`、`commands`、`environment`、`base_commit`、`run_url`、`timestamp`；其中 `assertion_id` 必须等于本表 key，`status` 必须为 `pass|fail`，不得以 `skip|estimated|advisory` 充绿（条件未触发只能登记 `not-triggered`、环境缺失只能登记 `dev_env_degrade`，见 §3）。
- 每个 symbolic key 最终一对一取得一个 numeric CI step。脚本可复用，但 workflow 必须按 `--gate <symbolic-key>` 独立调用、独立产 evidence、独立给结论；任一 P0 assertion 缺失或失败，只能使该 P0 为红，**不得用同脚本内另一 assertion 的绿色代替**。一次 smoke 可共享进程启动成本，但每行必须单独产出 `PASS|FAIL|SKIP|DEV_ENV_DEGRADE`；只有 `PASS` 满足 P0。
- **单一 key/脚本命名空间**：symbolic key 一律小写点分 `g10.p{0,1}.m<###>.<slug>`，脚本一律 `ci/g10_<slug>_smoke.py`，evidence schema 一律 `milestones/g10/g10_m<###>_<slug>_evidence_schema.json`（slug 与 key 末段同字面，只冻结路径不预建文件）。本表、[G10_CONTRACT.md](G10_CONTRACT.md) §4.2 与 [CI_GATES.md](CI_GATES.md) §4 引用同一份 key/脚本，由 `ci/check_g10_acceptance_map.py` 三向比对强制一致（见 §4）。
- device 判据统一要求真实路径、`RURIX_REQUIRE_REAL=1`、validation error 为 0；mock、stub、host substitution、仅检查非零输出、缺 provisioning 的 SKIP 均不满足能力门。host oracle、既有最小见证、人工截图均不能替代目标门；**禁以截图/人工采集帧冒充 harness 出帧**（契约 G-G10-4 字面）。
- 负例 RED 臂列：「内联」= 契约 §4.2 判据字面中已含的 RED 臂（逐字摘录，与判据列同源）；「PLAN §3 草案补充」= [G10_PLAN.md](G10_PLAN.md) §3 草案登记的额外臂，**不并入契约判据字面**，进 validator 时以契约字面为准、草案臂须经治理程序硬化后方可机核。

| M 行 | Symbolic CI gate key / 稳定脚本 | Evidence schema（目标路径，只冻结不预建） | 独立硬判据（契约 §4.2 逐字） | 负例 RED 臂 | device/host 性质 | 最晚波次 | numeric_step |
|---|---|---|---|---|---|---|---|
| **M128** | `g10.p0.m128.ue5_capture_environment`<br>`py -3 ci/g10_ue5_capture_environment_smoke.py --gate g10.p0.m128.ue5_capture_environment` | `milestones/g10/g10_m128_ue5_capture_environment_evidence_schema.json` | spike 裁决路径落地 + 固定场景 UE 5.8 侧出帧成功 + 环境画像（UE build digest/驱动/锁频）随证据存档；出帧进程非零退出冒充成功即 RED；预置假帧冒充真出帧即 RED | 内联：出帧进程非零退出冒充成功即 RED；预置假帧冒充真出帧即 RED。PLAN §3 草案补充：环境画像缺字段即 RED | host 编排 + device（UE 侧 GPU 渲染） | **G10.2** | post-interlock actual-next-free allocation |
| **M129** | `g10.p0.m129.ue5_reference_frames`<br>`py -3 ci/g10_ue5_reference_frames_smoke.py --gate g10.p0.m129.ue5_reference_frames` | `milestones/g10/g10_m129_ue5_reference_frames_evidence_schema.json` | 场景清单逐场景参考帧落盘 + 同参数双跑帧 digest 一致 + provenance（场景/相机/光照/build）登记闭集；双跑 digest 不等即 RED；provenance 缺行即 RED | 内联：双跑 digest 不等即 RED；provenance 缺行即 RED。PLAN §3 草案补充：帧文件篡改检测 RED | host+device | **G10.2** | post-interlock actual-next-free allocation |
| **M130** | `g10.p0.m130.dual_determinism_contract`<br>`py -3 ci/g10_dual_determinism_contract_smoke.py --gate g10.p0.m130.dual_determinism_contract --phase g10.2`<br>`py -3 ci/g10_dual_determinism_contract_smoke.py --gate g10.p0.m130.dual_determinism_contract --phase g10.5` | `milestones/g10/g10_m130_dual_determinism_contract_evidence_schema.json` | 相机/光照/时间参数同 schema 双端各一份 + digest 比对相等；单端参数漂移注入即 RED；schema 外字段注入即 RED；digest 不等仍出 A/B 报告即 RED（门序硬约束） | 内联：单端参数漂移注入即 RED；schema 外字段注入即 RED；digest 不等仍出 A/B 报告即 RED（门序硬约束） | host 纯 host | **G10.2 骨架 → G10.5 双端核验**（双阶段口径见 §3.3） | post-interlock actual-next-free allocation |
| **M131** | `g10.p0.m131.asset_license_registry`<br>`py -3 ci/g10_asset_license_registry_smoke.py --gate g10.p0.m131.asset_license_registry` | `milestones/g10/g10_m131_asset_license_registry_evidence_schema.json` | 逐资产 license 白名单闭集 + SPDX id + 来源 URL + attribution + 资产 digest；未登记资产混入即 RED；白名单外许可注入即 RED | 内联：未登记资产混入即 RED；白名单外许可注入即 RED。PLAN §3 草案补充：URL/digest 缺字段即 RED | host 纯 host | **G10.3** | post-interlock actual-next-free allocation |
| **M132** | `g10.p0.m132.corpus_loading`<br>`py -3 ci/g10_corpus_loading_smoke.py --gate g10.p0.m132.corpus_loading` | `milestones/g10/g10_m132_corpus_loading_evidence_schema.json` | 场景清单逐场景 Rurix 加载成功 + 三角形/材质/纹理计数非空 + 加载事件序列 golden；计数为零冒充成功即 RED；静默丢场景即 RED | 内联：计数为零冒充成功即 RED；静默丢场景即 RED | host+device | **G10.3** | post-interlock actual-next-free allocation |
| **M134** | `g10.p0.m134.frame_capture_pipeline`<br>`py -3 ci/g10_frame_capture_pipeline_smoke.py --gate g10.p0.m134.frame_capture_pipeline` | `milestones/g10/g10_m134_frame_capture_pipeline_evidence_schema.json` | HDR 帧捕获落盘 + 捕获→回读逐像素往返无损 + 分辨率/色彩空间元数据齐备；位深截断注入即 RED；sRGB/线性混标注入即 RED | 内联：位深截断注入即 RED；sRGB/线性混标注入即 RED。PLAN §3 草案补充：元数据缺字段即 RED | host+device | **G10.4** | post-interlock actual-next-free allocation |
| **M135** | `g10.p0.m135.flip_metric`<br>`py -3 ci/g10_flip_metric_smoke.py --gate g10.p0.m135.flip_metric` | `milestones/g10/g10_m135_flip_metric_evidence_schema.json` | 自实现与参考实现逐图对拍一致（容差 measured 标定）+ 恒等图对 FLIP=0 极值断言 + 参考实现版本 pin；参考输出扰动注入即 RED | 内联：参考输出扰动注入即 RED。PLAN §3 草案补充：恒等图对非零即 RED；口径参数漂移注入即 RED | host 纯 host | **G10.4** | post-interlock actual-next-free allocation |
| **M136** | `g10.p0.m136.ssim_psnr_metric`<br>`py -3 ci/g10_ssim_psnr_metric_smoke.py --gate g10.p0.m136.ssim_psnr_metric` | `milestones/g10/g10_m136_ssim_psnr_metric_evidence_schema.json` | 口径冻结进 spec + 参考实现逐图对拍一致 + 恒等图对 SSIM=1/PSNR=inf 极值断言；口径漂移注入即 RED | 内联：口径漂移注入即 RED。PLAN §3 草案补充：参考输出扰动注入即 RED；恒等图对非极值即 RED | host 纯 host | **G10.4** | post-interlock actual-next-free allocation |
| **M137** | `g10.p0.m137.pixel_diff_report`<br>`py -3 ci/g10_pixel_diff_report_smoke.py --gate g10.p0.m137.pixel_diff_report` | `milestones/g10/g10_m137_pixel_diff_report_evidence_schema.json` | diff 热区图 + 逐区域统计落盘 + evidence schema 闭集；diff 图与标量报告不一致注入即 RED；空场景行即 RED | 内联：diff 图与标量报告不一致注入即 RED；空场景行即 RED | host 纯 host | **G10.4** | post-interlock actual-next-free allocation |
| **M139** | `g10.p0.m139.ab_comparison`<br>`py -3 ci/g10_ab_comparison_smoke.py --gate g10.p0.m139.ab_comparison` | `milestones/g10/g10_m139_ab_comparison_evidence_schema.json` | 场景全集双端出图 + 度量报告 + 差距清单落盘；差距清单缺场景行即 RED；单端缺帧聚合 PASS 即 RED；M130 digest 不等仍出报告即 RED | 内联：差距清单缺场景行即 RED；单端缺帧聚合 PASS 即 RED；M130 digest 不等仍出报告即 RED | host+device | **G10.5** | post-interlock actual-next-free allocation |
| **M140** | `g10.p0.m140.gap_registry`<br>`py -3 ci/g10_gap_registry_smoke.py --gate g10.p0.m140.gap_registry` | `milestones/g10/g10_m140_gap_registry_evidence_schema.json` | 每差距项带 UE5 Renderer 模块归属（模块路径枚举闭集）+ measured delta + 建议 P 级 + G11 承接锚；缺归属/缺承接锚行即 RED；非 measured 叙述充差距即 RED | 内联：缺归属/缺承接锚行即 RED；非 measured 叙述充差距即 RED | host 纯 host | **G10.5** | post-interlock actual-next-free allocation |
| **M141** | `g10.p0.m141.perf_baseline`<br>`py -3 ci/g10_perf_baseline_smoke.py --gate g10.p0.m141.perf_baseline` | `milestones/g10/g10_m141_perf_baseline_evidence_schema.json` | 双端同场景帧率采样（14 §5 协议）+ 环境画像随证据存档 + 双端交替采样顺序登记；未锁频/环境画像缺字段即 RED；采样轮数不足冒充即 RED | 内联：未锁频/环境画像缺字段即 RED；采样轮数不足冒充即 RED | host+device | **G10.5** | post-interlock actual-next-free allocation |

任一行缺失、合并后不可区分、非 `PASS` 或无对应 evidence schema，均阻断 G10.8b（契约 §4.2 末段字面）。**G10 不设画质通过阈值与帧率通过线**——差距全量 measured 登记即绿（契约 G-G10-7 / 立项裁决 5），本表不设任何 FLIP/SSIM/PSNR/帧率通过线。

---

## 2. 已 go P1 硬门（两行：M133 / M138）

契约 §4.2 末段：「M133（清单冻结）/M138（阈值标定）为 P1，入验收映射随主门核验」——两行随主门（G10.3 / G10.4 波聚合门）核验，key/脚本/schema 命名空间与 §1 同构（本表冻结；契约 §4.2 不载 P1 行，三向比对不含 P1，MAP §2 ↔ [CI_GATES.md](CI_GATES.md) §4A 双向比对强制）。判据事实源 = [G10_PLAN.md](G10_PLAN.md) §2 G10.3/G10.4 退出门草案与 §3 建议清单口径 + 契约 acceptance_gates G-G10-5/G-G10-6 字面。`numeric_step` 一律 `post-interlock actual-next-free allocation`，待各门脚本/schema/workflow 步骤 materialize 时按落盘实测 `next_free` 顺位回填；本节不预建空脚本、空 schema 壳或占位 workflow 步骤。

| M 行 | Symbolic CI gate key / 稳定脚本 | Evidence schema（目标路径，只冻结不预建） | 精确 PASS 判据（本行独立 assertion） | 负例 RED 臂 | device/host 性质 | 最晚波次 | numeric_step |
|---|---|---|---|---|---|---|---|
| **M133** | `g10.p1.m133.corpus_list_freeze`<br>`py -3 ci/g10_corpus_list_freeze_smoke.py --gate g10.p1.m133.corpus_list_freeze` | `milestones/g10/g10_m133_corpus_list_freeze_evidence_schema.json` | 场景清单版本化冻结 + 清单 digest 注册在树 + 后续变更只追加修订行（清单全场景与 M131 许可登记、M132 加载门行集闭集对账） | 清单变更无只追加修订行即 RED；未注册 digest 冒充冻结即 RED；清单行集与许可/加载登记不对账即 RED | host 纯 host | **G10.3** | post-interlock actual-next-free allocation |
| **M138** | `g10.p1.m138.metric_threshold_calibration`<br>`py -3 ci/g10_metric_threshold_calibration_smoke.py --gate g10.p1.m138.metric_threshold_calibration` | `milestones/g10/g10_m138_metric_threshold_calibration_evidence_schema.json` | 度量阈值标定程序可复跑 + 标定值入 `g10_budget.json`（measured_local）且 provenance 齐备（P-09，禁手写阈值；契约 G-G10-6 字面「M138 标定值入 g10_budget 且 provenance 齐备」） | 手写阈值冒充标定即 RED；estimated 冒充 measured 即 RED；标定程序不可复跑即 RED | host 纯 host | **G10.4** | post-interlock actual-next-free allocation |

---

## 3. 条件型 / not-triggered 登记面

### 3.1 Epic 账号人工接管点（dev_env_degrade 不充绿）

立项裁决 2（契约 §7 逐字登记）：UE5 出图首选路径 = ②Launcher 安装 UE 5.8 正式版，**Epic 账号登录设人工接管点（用户交互一次）**；登录受阻则回退 ①源码编译臂（K: 盘承载，qwasg 凭据已核查在 EpicGames 组织可用）。纪律：人工接管点未完成 = M128/M129 对应 evidence 只能登记 `DEV_ENV_DEGRADE`（环境缺失），**不充绿、不阻塞其他 host 面**；禁在 CI 内嵌任何凭据（R-G10-2）；回退臂与首选臂诚实登记，禁伪绿（契约 G-G10-4「裁决路径不可行时回退备选臂并 §8 只追加修订」）。

### 3.2 UE 出帧时域未收敛场景行 not-ready 登记

R-G10-7（PLAN §4）：UE 出帧时域非确定性（TSR/时域累积初帧未收敛致帧 digest 不稳）。处置口径：固定 seed + warmup 帧数 + 收敛后捕获协议进 harness；M129 双跑 digest 一致门为硬判据；**时域未收敛的场景行登记 `not-ready` 不充绿**——该场景行从 M129/M139 当次 PASS 面剔除并如实登记，不得以未收敛帧冒充参考帧、不得以场景子集绿色冒充清单全集绿（M139「差距清单缺场景行即 RED」口径联动：not-ready 行必须在差距清单/A/B 报告中显式存在并标注，不得静默丢行）。

### 3.3 M130 双阶段口径（骨架期 / 双端核验期，同 G9 M121/M122 phase 范式）

M130 单 key 双 phase（不拆双 key，沿 [G9_ACCEPTANCE_MAP.md](../g9/G9_ACCEPTANCE_MAP.md) §2 M121/M122 行范式）：

- **骨架期（`--phase g10.2`，G10.2 波）**：相机/光照/时间参数 schema 冻结 + 双端各一份参数面就位 + digest 比对面就位；evidence `phase_g10_2_pass=true`、`phase_g10_5_pass=false`（骨架期绿不替双端核验期充绿）。
- **双端核验期（`--phase g10.5`，G10.5 波）**：双端真实参数 digest 比对相等实测；`phase_g10_5_pass=true` 方为完整绿；**门序硬约束**——digest 不等仍出 A/B 报告即 RED（M139 前置机器核验 M130 双端核验期最新 evidence 须 `status=="pass"` 且 `phase_g10_5_pass==true`，沿 `ci/g9_gi_interlock.py` D2-Q7 / `ci/g9_physics_interlock.py` RXS-0375 门序阻断先例，post-interlock materialize）。
- schema 同时要求 `phase_g10_2_pass=true` 与 `phase_g10_5_pass=true`（完整期形态承载，沿 G9 v1.14 anyOf 双支体例）；任一阶段绿色不能替另一阶段充绿。

---

## 4. 互斥与对账面（key 命名空间三方逐字一致机器可核声明）

1. **三方逐字一致**：本表 §1 十二行、[G10_CONTRACT.md](G10_CONTRACT.md) §4.2 十二行、[CI_GATES.md](CI_GATES.md) §4 十二行对同一 P0 M 行给出的 symbolic gate key 与稳定脚本名**必须逐字相等**，由 `ci/check_g10_acceptance_map.py` 三向比对机器强制；任一处漂移即 FAIL。已 go P1 两行做本表 §2 与 [CI_GATES.md](CI_GATES.md) §4A **双向**逐字比对（契约 §4.2 不载 P1 行）。本声明为机器可核面：比对以文件字面为准，不以叙述替代。
2. **唯一命名空间**：`g10.p{0,1}.m<###>.<slug>` + `ci/g10_<slug>_smoke.py` + `milestones/g10/g10_m<###>_<slug>_evidence_schema.json` 为唯一合法形态；G10 命名空间（`g10.*`）与 G9 已消费 34 key 命名空间（`g9.*`）互不包含；全部 key 全局唯一，匹配 `g10\.p[01]\.m\d{3}\.[a-z0-9_]+`；没有两个 M 行共享 key；M130 单 key 双 phase 不构成第二 key。
3. **互斥**：M## 与 key 一对一；`no-go`/`defer` 项（如 G10-N5 DLSS/Streamline 方向登记，defer-to-G11+ 锚定 G13）不产生 key、不入本表，不得冒充 PASS；G9 十锚全 defer-to-G11+（[G10_CANDIDATE_DECISIONS.md](G10_CANDIDATE_DECISIONS.md) §1）不入本表；P0 集合变更属于契约变更，不得以勘误处理。
4. **防混淆登记**：`g10.p0.m135.flip_metric` 的 FLIP = 图像感知度量，与 RD-044 族 FLIP（流体，[G9_P2_DECISIONS.md](../g9/G9_P2_DECISIONS.md) §1 RD044-fluid 行）同名不同物，互不构成触发（[G10_CANDIDATE_DECISIONS.md](G10_CANDIDATE_DECISIONS.md) §2 RD-044 行字面）。

---

## 5. G10.1 治理覆盖与空行门

G10.1 必须提供不占 numeric CI step 的 guardrail（脚本名与 [CI_GATES.md](CI_GATES.md) §3 同一份，属 `check_*` 未编号守卫）：

```text
g10.gov.acceptance_coverage
  py -3 ci/check_g10_acceptance_map.py

g10.gov.implementation_interlock
  py -3 ci/check_g10_implementation_interlock.py

g10.gov.measured_baseline
  py -3 ci/budget_eval.py
  py -3 ci/check_g10_budget_baseline.py
```

`ci/check_g10_acceptance_map.py` 的 PASS 判据（coverage + no-empty 两组断言分别独立报告）：

1. P0 行集合与 §1 的 12 项**集合全等**，无遗漏、无额外 P0、无重复；已 go P1 行集合与 §2 声明集合 `{M133,M138}` 全等。
2. 全部 symbolic key 全局唯一，均匹配 `g10\.p[01]\.m\d{3}\.[a-z0-9_]+`；每行只有一个 canonical `assertion_id`，没有两个 M 行共享 key。
3. 每一行均有脚本命令、evidence schema、可机器求值的 PASS 判据、负例 RED 臂、device/host 性质、最晚波次；共享脚本必须使用不同的 `--gate`（及 `--phase`）参数。
4. **三向一致**：本表 §1、`G10_CONTRACT.md` §4.2 与 `CI_GATES.md` §4 对同一 P0 M 行给出的 key 与脚本必须逐字相等；任一处漂移即 FAIL。已 go P1 行做本表 §2 与 `CI_GATES.md` §4A **双向**逐字比对。

no-empty 组的 PASS 判据：

- 逐单元格拒绝空串、空白、`TBD`、`TODO`、`待定`、`待补`、`—`；表中全部行的必填列均非空。
- 所有 schema 路径必须唯一落到对应 M 行，且文件名含同一 `m###` 与同一 slug；所有波次属于 `G10.2|G10.3|G10.4|G10.5` 的非空集合（M130 允许 `G10.2 骨架 → G10.5 双端核验`）。
- G10.1 只核映射完整性，不把尚未 materialize 的脚本/schema 误判为实现绿色；对应能力实现 PR 合入前或同 PR，`ci/check_schemas.py` 必须能校验该 schema 与实际 evidence。

`ci/check_g10_implementation_interlock.py` 的 PASS 判据：逐项读取事实源输出 §6 各条件真值；G10.1 期间必须诚实输出 `BLOCKED`（`--expect-blocked` 只证明 validator 能识别阻断，不算互锁 PASS）；仅全部条件为真时才输出 `READY`（`--require-ready` exit 0）。`ci/check_g10_budget_baseline.py` 的 PASS 判据：`g10_budget.json` 非空、`evidence_level=measured_local`、零 `estimated`，counter 与 evaluator 同步；baseline 只证明测量已建立，不得声称实现性能通过。

三个 validator 均把每组断言逐条打印，不以一个总 `all_pass=true` 掩盖具体缺行；`--selftest` 用受控负样本证明它们能红。治理 evidence schema 与实现期 evidence 同 PR 落，不预建空壳。

---

## 6. G10.2 硬互锁

`G10.GOV.G10_2.ENTRY_INTERLOCK` 是 G10.2 的前置 required check；它属于 `check_*` 治理守卫，不占 numeric CI step。以下条件必须**同时**为真（契约 front matter `implementation_unlock.required_all` 与 G-G10-3 字面展开）：

1. G9 已 closed（`milestones/g9/G9_CONTRACT.md` §8.10 `status: closed`，2026-08-15，flip commit `6ff73830` + 收口批 `c0cdfddd`），且 G10.0 文档集不可变 ref `c0cdfddd` 已登记。
2. 两份 Full RFC（画面对标与度量语义 / 外部参照 harness 与许可边界）均经 D-409 独立 provenance 对抗性评审后 Agent Approved；RFC 编号按立项时实测 `registry/number_ledger.json` namespaces.RFC `next_free` 领取，登记与 README/ledger 一致。
3. `G10_CANDIDATE_DECISIONS.md` 分项映射无空行；`registry/deferred.json` history 只追加、无静默改判；本表 §1/§2 无缺行（D-G10-3）。
4. §5 的 `g10.gov.acceptance_coverage`（coverage + no-empty 两组）独立 PASS；`g10.gov.measured_baseline` PASS（RTX 4070 Ti measured baseline 非空、零 estimated）。
5. G10 的 numeric CI step claim 发生在上述互锁通过之后，并以 claim 当下 ledger 实测 `CI_step.next_free` 为起点；每个 symbolic key 一对一分配，未复用、未覆盖、未沿用任何草案建议值。
6. 用户 G10.2 开工指令已留痕（仓内可引用记录）。

任一条件为假时，互锁必须返回非零；此时禁止合入 G10.2 的 `spec/`、`conformance/`、`src/` 或 workflow 实现改动。不能用 owner override 绕过任何条件，也不能用本表存在本身当作 G10.2 开工许可。`check_g10_implementation_interlock --require-ready` 输出 READY 是机器事实，不以叙述替代（契约 G-G10-3 字面）。

---

## 7. Close-out 审计

- G10.8a 必须重跑全部 12 个 P0（M130 含 `--phase g10.5` 双端核验腿）与全部已 go P1 的**各自 assertion**；evidence 的 `base_commit` 必须落在同一候选 close-out 基线，零 skip/estimated；soak 量级沿 G9.8a 继承或 measured 证明更短足够（具体阈值 G10.1 裁决 measured 标定，PLAN §2 G10.8a 字面）；`budget_eval --strict` 非空全 PASS。
- G10.8b 只有在 12 个 P0 key 全 PASS、全部已 go P1 key 全 PASS、验收映射/候选决策/RD 最终状态逐字一致、**差距清单终审锁定为 G11 法定输入**（G11 修复范围只能消费该清单 + 其承接锚）时才可 status flip；任一 P0 无独立硬门则禁止 flip（PLAN §2.9）。
- 同日放行先例继承（立项裁决 8）：8a full-run 先行完成后允许同日进 8b close-out；条件实现刚绿不得跳过 8a 直接 close。
- **G10 零修复 + 零通过线口径维持到 close-out**：G10 全域不提交任何画质修复 PR；差距清单只登记不修复；任何「已达 UE5 画质/帧率」叙述在 G10 期内一律不成立（契约 §5 字面）。
- 后续若治理流程将新的 P1 判为 go，须先按治理流程修订本表及覆盖集合，再开对应实现；不得把它静默并入现有 key。

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-15 | G10.1 初版：冻结 12 个 P0 的 symbolic gate、目标脚本/schema、独立判据（契约 §4.2 逐字）与最晚波次；已 go P1 两行（M133 corpus_list_freeze / M138 metric_threshold_calibration）同构登记；§3 条件型/not-triggered 登记面（Epic 账号人工接管点 dev_env_degrade 不充绿 / UE 出帧时域未收敛场景行 not-ready / M130 双阶段口径沿 G9 M121/M122 phase 范式）；§4 key 命名空间三方逐字一致机器可核声明；单一命名空间 `g10.p{0,1}.m###.<slug>` + `ci/g10_<slug>_smoke.py` + `g10_m###_<slug>_evidence_schema.json` 由 `ci/check_g10_acceptance_map.py` 三向比对强制；§5 治理覆盖与空行门、§6 G10.2 硬互锁六条件、§7 Close-out 审计。数字 CI 步骤全部 `post-interlock actual-next-free allocation`，零 workflow/script/schema 预放。 |
