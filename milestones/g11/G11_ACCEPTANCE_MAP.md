<!-- Assisted-by: Kimi-K3（G11.1 治理波起草） -->
# G11_ACCEPTANCE_MAP — P0 / 已 go P1 验收映射

> **性质**：G11.1 治理交付物（governance-only）；事实源为 [G11_CONTRACT.md](G11_CONTRACT.md) v1.0 front matter acceptance_gates（G-G11-1~10）与 §4.2 十三行 P0 独立断言表、[G11_PLAN.md](G11_PLAN.md) v1.0 §2 各波退出门草案与 §3 P0 建议清单、[G11_CANDIDATE_DECISIONS.md](G11_CANDIDATE_DECISIONS.md) v1.0（§1 法定输入 11 差距行 → M144~M154 映射，§4 新增候选 → M155/M156 映射，§2 G10-N10 → M157 映射）。
> **编号纪律**：本表只冻结 symbolic CI gate key 与稳定脚本名，不 claim 数字 CI step。数字步骤必须等 G-G11-3 实现互锁（§6）通过后，再从 `registry/number_ledger.json` 当时实测的 `CI_step.next_free` 顺位分配——**numeric_step 一律写 `post-interlock actual-next-free allocation`**（当前实测 `CI_step.next_free=196`，G10 已消费至 195，[G10 CI_GATES](../g10/CI_GATES.md) v1.10 / ledger v1.112）；禁止沿用任何草案建议值。
> **证据纪律**：表内脚本与 schema 是实现 PR 的强制目标路径；二者须与对应 RED→GREEN 实现同 PR 落地。路径尚未 materialize 只表示未开工，不能记 PASS；本波不预建空脚本、空 schema 壳或占位 workflow 步骤（只冻结路径）。

---

## 1. P0 硬门（精确 13 行）

- P0 精确集合（13 行）：`{M144,M145,M146,M147,M148,M149,M150,M151,M152,M153,M154,M155,M156}`。每一行的 symbolic key 同时是该能力唯一的 `assertion_id`，与 [G11_CONTRACT.md](G11_CONTRACT.md) §4.2 **逐字一致**（key 命名空间三方一致性机核面，禁止任何改写）；独立硬判据列同逐字。实现后的 evidence 顶层必须至少含：`schema_version`、`subject`、`milestone`、`wave`、`assertion_id`、`status`、`commands`、`environment`、`base_commit`、`run_url`、`timestamp`；其中 `assertion_id` 必须等于本表 key，`status` 必须为 `pass|fail`，不得以 `skip|estimated|advisory` 充绿（条件未触发只能登记 `not-triggered`、环境缺失只能登记 `dev_env_degrade`，见 §3）。
- 每个 symbolic key 最终一对一取得一个 numeric CI step。脚本可复用，但 workflow 必须按 `--gate <symbolic-key>` 独立调用、独立产 evidence、独立给结论；任一 P0 assertion 缺失或失败，只能使该 P0 为红，**不得用同脚本内另一 assertion 的绿色代替**。一次 smoke 可共享进程启动成本，但每行必须单独产出 `PASS|FAIL|SKIP|DEV_ENV_DEGRADE`；只有 `PASS` 满足 P0。
- **单一 key/脚本命名空间**：symbolic key 一律小写点分 `g11.p{0,1}.m<###>.<slug>`，脚本一律 `ci/g11_<slug>_smoke.py`，evidence schema 一律 `milestones/g11/g11_m<###>_<slug>_evidence_schema.json`（slug 与 key 末段同字面，只冻结路径不预建文件）。本表、[G11_CONTRACT.md](G11_CONTRACT.md) §4.2 与 [CI_GATES.md](CI_GATES.md) §4 引用同一份 key/脚本，由 `ci/check_g11_acceptance_map.py` 三向比对强制一致（见 §4）。
- device 判据统一要求真实路径、`RURIX_REQUIRE_REAL=1`、validation error 为 0；mock、stub、host substitution、仅检查非零输出、缺 provisioning 的 SKIP 均不满足能力门。host oracle、既有最小见证、人工截图均不能替代目标门。
- **修复闭环判据统一形态**（M147~M154 八行修复闭环门 + M144~M146 三行口径对齐闭环门共用字面）：修复落盘（只消费 G10.8b 锁定清单对应行 + 承接锚字面）+ 修复前后度量 delta 收敛 measured（复测 delta 相对锁定基线 delta 收敛，收敛阈值由 G11.2/G11.5 标定程序 measured 产出，禁手写）+ 契约参数 digest 0-byte + 不降级既有 48 门绿面。**G11 不设绝对画质通过线**——「已达 UE5 画质」判定归 G15 商用收口期（契约 §1/§5 字面）。
- 负例 RED 臂列：「内联」= 契约 §4.2 判据字面中已含的 RED 臂（逐字摘录，与判据列同源）；「PLAN §3 草案补充」= [G11_PLAN.md](G11_PLAN.md) §3 草案登记的额外臂，**不并入契约判据字面**，进 validator 时以契约字面为准、草案臂须经治理程序硬化后方可机核。

| M 行 | Symbolic CI gate key / 稳定脚本 | Evidence schema（目标路径，只冻结不预建） | 独立硬判据（契约 §4.2 逐字） | 负例 RED 臂 | device/host 性质 | 最晚波次 | numeric_step |
|---|---|---|---|---|---|---|---|
| **M144** | `g11.p0.m144.caliber_c1_indoor_luminance`<br>`py -3 ci/g11_caliber_c1_indoor_luminance_smoke.py --gate g11.p0.m144.caliber_c1_indoor_luminance` | `milestones/g11/g11_m144_caliber_c1_indoor_luminance_evidence_schema.json` | GI/天光遮蔽口径差 + 太阳 lux→辐射度链差逐行对齐（对齐后残余口径差显式登记）+ 对齐前后口径参数 provenance 齐备；未对齐口径消费复测 delta 即 RED；拟合冒充对齐即 RED；残余口径差未登记即 RED | 内联：未对齐口径消费复测 delta 即 RED；拟合冒充对齐即 RED；残余口径差未登记即 RED | host 纯 host | **G11.2** | post-interlock actual-next-free allocation |
| **M145** | `g11.p0.m145.caliber_c2_exposure_chain`<br>`py -3 ci/g11_caliber_c2_exposure_chain_smoke.py --gate g11.p0.m145.caliber_c2_exposure_chain` | `milestones/g11/g11_m145_caliber_c2_exposure_chain_evidence_schema.json` | 双端 EV100 同字面下派生尺度对齐（Rurix 臂 2^(−EV100) vs UE 臂 pipe 内手动曝光已施 ×1.0——统一或显式互证登记）+ 派生链元数据互证回归；派生尺度未对齐出 LDR 度量即 RED；互证链断裂即 RED | 内联：派生尺度未对齐出 LDR 度量即 RED；互证链断裂即 RED | host 纯 host | **G11.2** | post-interlock actual-next-free allocation |
| **M146** | `g11.p0.m146.caliber_c3_exr_bit_depth`<br>`py -3 ci/g11_caliber_c3_exr_bit_depth_smoke.py --gate g11.p0.m146.caliber_c3_exr_bit_depth` | `milestones/g11/g11_m146_caliber_c3_exr_bit_depth_evidence_schema.json` | UE EXR fp16→f32 提升口径（RXS-0385 strip-and-log）与 Rurix 原生 f32 度量域对齐登记 + 位深元数据闭集回归；位深截断注入即 RED；元数据缺字段即 RED | 内联：位深截断注入即 RED；元数据缺字段即 RED | host 纯 host | **G11.2** | post-interlock actual-next-free allocation |
| **M147** | `g11.p0.m147.fix_r1_material_subset`<br>`py -3 ci/g11_fix_r1_material_subset_smoke.py --gate g11.p0.m147.fix_r1_material_subset` | `milestones/g11/g11_m147_fix_r1_material_subset_evidence_schema.json` | R1 修复闭环：baseColorTexture/法线/metallic-roughness 采样接入（承接锚字面消费）+ 修复前后 LDR 臂度量 delta 收敛 measured（锁定基线 = bistro LDR SSIM delta 0.8328980787837229，收敛阈由标定程序产）+ 契约 digest 0-byte；未采样冒充修复即 RED；delta 未收敛冒充闭环即 RED；契约参数漂移即 RED | 内联：未采样冒充修复即 RED；delta 未收敛冒充闭环即 RED；契约参数漂移即 RED | host+device | **G11.3** | post-interlock actual-next-free allocation |
| **M148** | `g11.p0.m148.fix_r2_geometry_normals`<br>`py -3 ci/g11_fix_r2_geometry_normals_smoke.py --gate g11.p0.m148.fix_r2_geometry_normals` | `milestones/g11/g11_m148_fix_r2_geometry_normals_evidence_schema.json` | R2 修复闭环：winding 朝向 + 双面翻转消费（平滑法线面承接锚字面）+ 修复前后 cornell HDR 覆盖 delta 收敛 measured（锁定基线 −0.7451210021972656）+ 与 U1 同面对账；法线未消费冒充修复即 RED；delta 未收敛冒充闭环即 RED | 内联：法线未消费冒充修复即 RED；delta 未收敛冒充闭环即 RED | host+device | **G11.3** | post-interlock actual-next-free allocation |
| **M149** | `g11.p0.m149.fix_r5_json_u64_seed`<br>`py -3 ci/g11_fix_r5_json_u64_seed_smoke.py --gate g11.p0.m149.fix_r5_json_u64_seed` | `milestones/g11/g11_m149_fix_r5_json_u64_seed_evidence_schema.json` | R5 修复闭环：u64 顶格 seed 合法消费（i64 域 fail-closed 解除）+ 既有 seed=42 契约 digest 不变回归 + u64 边界语料锚定；顶格 seed 仍拒绝即 RED；既有 digest 漂移即 RED | 内联：顶格 seed 仍拒绝即 RED；既有 digest 漂移即 RED | host 纯 host | **G11.3** | post-interlock actual-next-free allocation |
| **M150** | `g11.p0.m150.fix_u1_cornell_shell_radiance`<br>`py -3 ci/g11_fix_u1_cornell_shell_radiance_smoke.py --gate g11.p0.m150.fix_u1_cornell_shell_radiance` | `milestones/g11/g11_m150_fix_u1_cornell_shell_radiance_evidence_schema.json` | U1 修复闭环：cornell 壳体（墙/顶/地板）零辐射修复（语料派生面走 M133 只追加修订程序或双端着色口径对齐面）+ 修复后 UE 帧覆盖收敛 measured（锁定基线 = UE 覆盖 18.39% vs Rurix 92.90%，HDR nonzero 比 delta −0.7451210021972656）+ Rurix 侧覆盖面不降级；语料静默改写即 RED；覆盖未收敛冒充闭环即 RED；Rurix 侧降级即 RED | 内联：语料静默改写即 RED；覆盖未收敛冒充闭环即 RED；Rurix 侧降级即 RED | host+device | **G11.3** | post-interlock actual-next-free allocation |
| **M151** | `g11.p0.m151.fix_u2_bistro_texture_dds`<br>`py -3 ci/g11_fix_u2_bistro_texture_dds_smoke.py --gate g11.p0.m151.fix_u2_bistro_texture_dds` | `milestones/g11/g11_m151_fix_u2_bistro_texture_dds_evidence_schema.json` | U2 修复闭环：DDS 纹理解码面落地（G10-N7 承接锚兑现，Direct PR 面不触语义冻结面）+ 材质实例 texture_parameter_values 非空回归 + 修复前后 LDR 臂度量 delta 收敛 measured（锁定基线 = bistro LDR 亮度中位 delta 0.7698879749655723）；纹理仍全缺冒充修复即 RED；未登记资产混入即 RED；delta 未收敛冒充闭环即 RED | 内联：纹理仍全缺冒充修复即 RED；未登记资产混入即 RED；delta 未收敛冒充闭环即 RED | host+device | **G11.3** | post-interlock actual-next-free allocation |
| **M152** | `g11.p0.m152.fix_u3_bistro_animation`<br>`py -3 ci/g11_fix_u3_bistro_animation_smoke.py --gate g11.p0.m152.fix_u3_bistro_animation` | `milestones/g11/g11_m152_fix_u3_bistro_animation_evidence_schema.json` | U3 修复闭环：Bistro 动画 Take 001 / glTF 相机节点消费或显式静态契约登记闭环 + 包内动画通道计数对账（锁定基线 = 消费 0 vs 包内 2 通道）+ 相机位姿契约 0-byte；动画通道静默丢弃冒充闭环即 RED；相机契约漂移即 RED | 内联：动画通道静默丢弃冒充闭环即 RED；相机契约漂移即 RED | host 纯 host | **G11.3** | post-interlock actual-next-free allocation |
| **M153** | `g11.p0.m153.fix_r3_light_subset`<br>`py -3 ci/g11_fix_r3_light_subset_smoke.py --gate g11.p0.m153.fix_r3_light_subset` | `milestones/g11/g11_m153_fix_r3_light_subset_evidence_schema.json` | R3 修复闭环：点/面光源 + glTF emissive 表达（bistro 包内 4+ 盏实测消费）+ 修复前后 HDR 亮度中位 delta 收敛 measured（锁定基线 2.664779790997505）+ cornell 契约 sun+sky 灯面 0-byte；点光源未表达冒充修复即 RED；delta 未收敛冒充闭环即 RED；契约灯面漂移即 RED | 内联：点光源未表达冒充修复即 RED；delta 未收敛冒充闭环即 RED；契约灯面漂移即 RED | host+device | **G11.4** | post-interlock actual-next-free allocation |
| **M154** | `g11.p0.m154.fix_r4_gi_multibounce_world_cache`<br>`py -3 ci/g11_fix_r4_gi_multibounce_world_cache_smoke.py --gate g11.p0.m154.fix_r4_gi_multibounce_world_cache` | `milestones/g11/g11_m154_fix_r4_gi_multibounce_world_cache_evidence_schema.json` | R4 + M99-clipmap 修复闭环：世界辐射缓存世界级 clipmap 级落地（G10.6 rejudged-go 承接锚字面 + RFC-0028 语义面 spec-first，RXS-0360 世界级登记翻转显式修订行）+ 修复前后 HDR 亮度 p90 delta 收敛 measured（锁定基线 4.697253086805343）+ 不以 g9.p1.m99 屏幕级绿色冒充世界级验收；世界级未落地冒充承接即 RED；屏幕级绿色冒充世界级即 RED；delta 未收敛冒充闭环即 RED | 内联：世界级未落地冒充承接即 RED；屏幕级绿色冒充世界级即 RED；delta 未收敛冒充闭环即 RED | host+device | **G11.4** | post-interlock actual-next-free allocation |
| **M155** | `g11.p0.m155.ab_retest_closure`<br>`py -3 ci/g11_ab_retest_closure_smoke.py --gate g11.p0.m155.ab_retest_closure` | `milestones/g11/g11_m155_ab_retest_closure_evidence_schema.json` | A/B 复测闭环：同契约双端复跑（契约参数 digest == G10.5 锁定值，不等仍出报告即 RED）+ 复测度量报告 + 复测差距清单 11 行闭集落盘（行集逐字对账；新差距项显式登记即 RED 评审面）+ 逐项闭环状态机核（修复前后 delta 收敛 measured，收敛阈由标定程序产）；清单缺行即 RED；单端缺帧聚合 PASS 即 RED | 内联：契约 digest 不等仍出报告即 RED；清单缺行/新项静默混入即 RED；单端缺帧聚合 PASS 即 RED | host+device | **G11.5** | post-interlock actual-next-free allocation |
| **M156** | `g11.p0.m156.regression_guard`<br>`py -3 ci/g11_regression_guard_smoke.py --gate g11.p0.m156.regression_guard` | `milestones/g11/g11_m156_regression_guard_evidence_schema.json` | 修复回归门：既有 48 门（G9 34 key + G10 14 key）最新 evidence 全绿只读汇总 + 修复触改面既有门重跑回归零降级；既有门降级即 RED；聚合遮蔽子断言 FAIL/SKIP/DEV_ENV_DEGRADE 即 RED | 内联：既有门降级即 RED；聚合遮蔽子断言 FAIL/SKIP/DEV_ENV_DEGRADE 即 RED | host 纯 host | **G11.5** | post-interlock actual-next-free allocation |

任一行缺失、合并后不可区分、非 `PASS` 或无对应 evidence schema，均阻断 G11.7b（契约 §4.2 末段字面）。**G11 不设绝对画质通过线**——修复闭环判据 = 修复前后度量 delta 收敛 measured（契约 §1/§5 / 立项裁决 3），本表不设任何绝对画质 FLIP/SSIM/PSNR 通过线。

---

## 2. 已 go P1 硬门（一行：M157）

契约 §4.2 末段：「M157（HDR-FLIP 独立标定）为 P1，入验收映射随主门核验」——本行随主门（G11.2 波聚合门）核验，key/脚本/schema 命名空间与 §1 同构（本表冻结；契约 §4.2 不载 P1 行，三向比对不含 P1，MAP §2 ↔ [CI_GATES.md](CI_GATES.md) §4A 双向比对强制）。判据事实源 = [G11_PLAN.md](G11_PLAN.md) §2 G11.2 退出门草案与 §3 建议清单口径 + 契约 acceptance_gates G-G11-4 字面 + [G10_P2_DECISIONS.md](../g10/G10_P2_DECISIONS.md) G10-N10 行承接锚。`numeric_step` 一律 `post-interlock actual-next-free allocation`，待门脚本/schema/workflow 步骤 materialize 时按落盘实测 `next_free` 顺位回填；本节不预建空脚本、空 schema 壳或占位 workflow 步骤。

| M 行 | Symbolic CI gate key / 稳定脚本 | Evidence schema（目标路径，只冻结不预建） | 精确 PASS 判据（本行独立 assertion） | 负例 RED 臂 | device/host 性质 | 最晚波次 | numeric_step |
|---|---|---|---|---|---|---|---|
| **M157** | `g11.p1.m157.hdr_flip_calibration`<br>`py -3 ci/g11_hdr_flip_calibration_smoke.py --gate g11.p1.m157.hdr_flip_calibration` | `milestones/g11/g11_m157_hdr_flip_calibration_evidence_schema.json` | HDR-FLIP 独立标定：HDR 域正式对拍样本集（真实 HDR 帧双臂，样本集下界 + digest 入 evidence）+ 标定程序可复跑（两跑逐位一致）+ 标定值按 M138 同程序（p100×k measured）入 `g11_budget.json`（measured_local）且 provenance 齐备（P-09，禁手写阈值；G10-N10 承接锚兑现） | 手写阈值冒充标定即 RED；estimated 冒充 measured 即 RED；标定程序不可复跑即 RED；样本集低于下界冒充有效标定即 RED | host 纯 host | **G11.2** | post-interlock actual-next-free allocation |

---

## 3. 条件型 / not-triggered 登记面

### 3.1 异己并发工作树面（不混入 G11 车道）

立项裁决 1（契约 §7 逐字登记）：G11 带未提交项立项——工作树异己会话 src/ 未提交面（rurix-asset/rurix-render geometry/gi/shadow/ssr/ktx2/hzb/restir/sdf_trace/smrt 声明面）保持不混入 G11 车道（G10.8b §8.10 先例同模）。纪律：G11 车道 commit 只含 G11 车道文件；异己面 evidence 不充 G11 任何门绿；若 G11 实现波与异己面触及同一文件，按只追加程序登记冲突面并请治理裁决（不得静默合并）。

### 3.2 触发评估登记面（三行 defer 的 G11 窗）

- **M100-high（G11.6 触发评估）**：R3 修复（G11.4 点/面光源表达）落地后，若多灯 workload measured 对照面产出（低档 MegaLights GPU 管线多灯场景 measured 对照），G11.6 穷举按只追加程序重判；未产出维持 defer（承接锚字面 0-byte）。
- **G10-N6 BistroExterior（G11.3 触发评估）**：G11.3 语料面若消费替代转换臂（glTF 派生链），按 M133 只追加修订程序重判清单扩容；未消费维持 defer。
- **G10-N17 M137 scalars.flip（G11.5 触发评估）**：G11.5 复测闭环核验若消费 diff 报告 FLIP 标量面，按 RXS-0388 L3 演进位程序翻转实值并回归 M137 门；未消费维持 null 演进位。

三行登记 `SKIP=not-triggered` 只表示决策已记录，不是成功，不充任何门绿。

### 3.3 GPU 管线画质差距面（G11-N3 边界登记）

锁定清单 11 行均为 host CPU 参考管线臂实测（G10-N16 字面）；GPU 管线画质差距面未 measured、不在锁定清单内——G11 不得无锚新立修复项（契约 §5 / 立项裁决 2）；GPU 管线双端 A/B 面锚定 G14（G10-N16 承接锚字面）。

### 3.4 M147 双 phase 口径（G11.3 修复落盘+局部度量登记期 / G11.5 收敛断言期，沿 G10 M130 §3.3 双阶段先例）

M147 单 key 双 phase（不拆双 key，沿 [G10_ACCEPTANCE_MAP.md](../g10/G10_ACCEPTANCE_MAP.md) §3.3 M130 双阶段口径先例；[G11_CONTRACT.md](G11_CONTRACT.md) §8.3a 修订句为裁决事实源，§4.2 M147 判据行正文 0-byte 冻结——本节为只追加校准注登记面，§1 M147 行字面 0-byte 不动）：

- **修复落盘+局部度量登记期（`--phase g11.3`，G11.3 波）**：材质子集采样接入消费核验 + 基线复现（锁定基线 = bistro LDR SSIM delta 0.8328980787837229）+ 契约 digest 0-byte + 标定/RED 臂全绿；收敛检 = verdict 显式登记形态——实测收敛（`converged` ∧ `convergence_pending=false`）或 `deferred_to_g11_5` 显式登记（∧ `convergence_pending=true`）皆合法；evidence 标 `phase=g11.3` + `g11_3_phase_pass`（当且仅当 12 检全绿）+ `convergence_pending`——**convergence_pending 缺登记冒充全闭环即 RED，不是 SKIP 充绿**（反向激励旁证 measured：ssim(ue_修,rurix_未修白帧)=0.1624318277352612 > ssim(ue_修,rurix_修)=0.009656442299775102，锁定度量对正确修复结构性不友好——G11.6 P2 候选行登记，契约 §8.3a）。
- **收敛断言期（`--phase g11.5`，G11.5 波）**：R1 行修复前后 SSIM delta 收敛断言（definitive 测量面 = G11.5 同契约复跑，RXS-0393 L2 quality_gap 款字面；阈值标定程序产禁手写；**不收敛则整波 FAIL**——契约 §8.3a 不弱化声明 + [G11_PLAN.md](G11_PLAN.md) §2 G11.5 节 M155 门预备注记）；当前 G11.5 未至，`--phase g11.5` **fail-closed 拒跑**（exit=2）。
- schema 同时承载 v1 legacy 支（双 phase 校准前既有 evidence 形态 0-byte）与 v2 g11.3 phase 支（沿 G9 v1.14 / G10 M130 anyOf 双支体例）；**g11.3 phase 绿不替 g11.5 收敛断言充绿**（wave3 聚合门 fact⑥ `m147_dual_phase_discipline` 两态机核，沿 G10.8a wave2 fact④ 两态校准先例，判据语义 0-byte）。
- **G11.5 落地注（v1.2 只追加；§1 M147 行与本节既有字面 0-byte 不动）**：`--phase g11.5` 收敛断言面已于 G11.5 波落地兑现——`ci/g11_fix_r1_material_subset_smoke.py --phase g11.5` fail-closed 拒跑面翻真跑（11 检集：契约 digest 0-byte / 材质消费维持 / 基线复现 / **definitive 面复测 delta 当次独立重算与复测差距清单 R1 行登记逐位互核**〔旧帧区值冒充即 RED〕/ 收敛断言〔RXS-0393 L2 quality_gap 款字面，收敛阈消费 g11_budget g11.fix.r1_ssim_shrink_tol 标定条目〕/ RED 四臂）；schema anyOf 扩 v3 支承载（wave=G11.5 + phase=g11.5 + g11_5_phase_pass + verdict enum converged|not_converged + if/then 反冒充条款，v1/v2 支 0-byte）；**不收敛则本门 FAIL、整波 FAIL**（契约 §8.3a 不弱化声明——G11.5 实测 verdict 与复跑数字以契约 §8.5 留痕为准）。

---

## 4. 互斥与对账面（key 命名空间三方逐字一致机器可核声明）

1. **三方逐字一致**：本表 §1 十三行、[G11_CONTRACT.md](G11_CONTRACT.md) §4.2 十三行、[CI_GATES.md](CI_GATES.md) §4 十三行对同一 P0 M 行给出的 symbolic gate key 与稳定脚本名**必须逐字相等**，由 `ci/check_g11_acceptance_map.py` 三向比对机器强制；任一处漂移即 FAIL。已 go P1 行做本表 §2 与 [CI_GATES.md](CI_GATES.md) §4A **双向**逐字比对（契约 §4.2 不载 P1 行）。本声明为机器可核面：比对以文件字面为准，不以叙述替代。
2. **唯一命名空间**：`g11.p{0,1}.m<###>.<slug>` + `ci/g11_<slug>_smoke.py` + `milestones/g11/g11_m<###>_<slug>_evidence_schema.json` 为唯一合法形态；G11 命名空间（`g11.*`）与 G9 已消费 34 key（`g9.*`）及 G10 已消费 14 key（`g10.*`）互不包含；全部 key 全局唯一，匹配 `g11\.p[01]\.m\d{3}\.[a-z0-9_]+`；没有两个 M 行共享 key。
3. **互斥**：M## 与 key 一对一；`no-go`/`defer` 项（如 G11-N3 GPU 管线画质差距面，defer-to-G12+ 锚定 G14）不产生 key、不入本表，不得冒充 PASS；G10 defer 维持行（M61/M52/M100-high 等 15 行）不入本表；P0 集合变更属于契约变更，不得以勘误处理。
4. **防混淆登记**：`g11.p0.m147.fix_r1_material_subset` 等修复闭环门消费的 FLIP = 图像感知度量（RXS-0389），与 RD-044 族 FLIP（流体，[G9_P2_DECISIONS.md](../g9/G9_P2_DECISIONS.md) §1 RD044-fluid 行）同名不同物，互不构成触发（G10 口径维持）。
5. **锁定基线 delta 溯源**：M147~M154 各行判据内嵌的锁定基线 delta 字面转引自 [`g10_gap_registry.json`](../g10/g10_gap_registry.json) 对应行 `measured_delta[].delta`（R1 ↔ SSIM 0.8328980787837229 / R2·U1 ↔ HDR nonzero 比 −0.7451210021972656 / R3·C1 ↔ HDR 亮度中位 2.664779790997505 / R4 ↔ HDR 亮度 p90 4.697253086805343 / U2 ↔ LDR 亮度中位 0.7698879749655723 / U3 ↔ 动画通道 2.0 / R5 ↔ u64 顶格探针）——锁定清单 0-byte 只消费不回写；复测时由 M155 门重算对账。

---

## 5. G11.1 治理覆盖与空行门

G11.1 必须提供不占 numeric CI step 的 guardrail（脚本名与 [CI_GATES.md](CI_GATES.md) §3 同一份，属 `check_*` 未编号守卫）：

```text
g11.gov.acceptance_coverage
  py -3 ci/check_g11_acceptance_map.py

g11.gov.implementation_interlock
  py -3 ci/check_g11_implementation_interlock.py

g11.gov.measured_baseline
  py -3 ci/budget_eval.py
```

`ci/check_g11_acceptance_map.py` 的 PASS 判据（coverage + no-empty 两组断言分别独立报告）：

1. P0 行集合与 §1 的 13 项**集合全等**，无遗漏、无额外 P0、无重复；已 go P1 行集合与 §2 声明集合 `{M157}` 全等。
2. 全部 symbolic key 全局唯一，均匹配 `g11\.p[01]\.m\d{3}\.[a-z0-9_]+`；每行只有一个 canonical `assertion_id`，没有两个 M 行共享 key。
3. 每一行均有脚本命令、evidence schema、可机器求值的 PASS 判据、负例 RED 臂、device/host 性质、最晚波次；共享脚本必须使用不同的 `--gate` 参数。
4. **三向一致**：本表 §1、`G11_CONTRACT.md` §4.2 与 `CI_GATES.md` §4 对同一 P0 M 行给出的 key 与脚本必须逐字相等；任一处漂移即 FAIL。已 go P1 行做本表 §2 与 `CI_GATES.md` §4A **双向**逐字比对。

no-empty 组的 PASS 判据：

- 逐单元格拒绝空串、空白、`TBD`、`TODO`、`待定`、`待补`、`—`；表中全部行的必填列均非空。
- 所有 schema 路径必须唯一落到对应 M 行，且文件名含同一 `m###` 与同一 slug；所有波次属于 `G11.2|G11.3|G11.4|G11.5` 的非空集合。
- G11.1 只核映射完整性，不把尚未 materialize 的脚本/schema 误判为实现绿色；对应能力实现 PR 合入前或同 PR，`ci/check_schemas.py` 必须能校验该 schema 与实际 evidence。

`ci/check_g11_implementation_interlock.py` 的 PASS 判据：逐项读取事实源输出 §6 各条件真值；G11.1 期间必须诚实输出 `BLOCKED`（`--expect-blocked` 只证明 validator 能识别阻断，不算互锁 PASS）；仅全部条件为真时才输出 `READY`（`--require-ready` exit 0）。`ci/budget_eval.py` 的 PASS 判据：`g11_budget.json` 非空、`evidence_level=measured_local`、零 `estimated`，counter 与 evaluator 同步；baseline 只证明测量已建立，不得声称实现性能通过。

两个 validator 均把每组断言逐条打印，不以一个总 `all_pass=true` 掩盖具体缺行；`--selftest` 用受控负样本证明它们能红。治理 evidence schema 与实现期 evidence 同 PR 落，不预建空壳。

---

## 6. G11.2 硬互锁

`G11.GOV.G11_2.ENTRY_INTERLOCK` 是 G11.2 的前置 required check；它属于 `check_*` 治理守卫，不占 numeric CI step。以下条件必须**同时**为真（契约 front matter `implementation_unlock.required_all` 与 G-G11-3 字面展开）：

1. G10 已 closed（`milestones/g10/G10_CONTRACT.md` §8.10 `status: closed`，2026-08-16，flip commit `27e3b07c` + 幂等复跑批 `53eb3a28`），且 G11.0 文档集不可变 ref `53eb3a28` 已登记。
2. Full RFC-0028（G11 GI 与光照画质闭环伞形）经 D-409 独立 provenance 对抗性评审后 Agent Approved；RFC 编号按立项时实测 `registry/number_ledger.json` namespaces.RFC `next_free` 领取，登记与 README/ledger 一致。
3. `G11_CANDIDATE_DECISIONS.md` 分项映射无空行；`registry/deferred.json` history 只追加、无静默改判；本表 §1/§2 无缺行（D-G11-3）。
4. §5 的 `g11.gov.acceptance_coverage`（coverage + no-empty 两组）独立 PASS；`g11.gov.measured_baseline` PASS（RTX 4070 Ti measured baseline 非空、零 estimated）。
5. G11 的 numeric CI step claim 发生在上述互锁通过之后，并以 claim 当下 ledger 实测 `CI_step.next_free` 为起点；每个 symbolic key 一对一分配，未复用、未覆盖、未沿用任何草案建议值。
6. 用户 G11.2 开工指令已留痕（2026-08-15 指令全期授权面，契约 §7 逐字登记）。

任一条件为假时，互锁必须返回非零；此时禁止合入 G11.2 的 `spec/`、`conformance/`、`src/` 或 workflow 实现改动。不能用 owner override 绕过任何条件，也不能用本表存在本身当作 G11.2 开工许可。`check_g11_implementation_interlock --require-ready` 输出 READY 是机器事实，不以叙述替代（契约 G-G11-3 字面）。

---

## 7. Close-out 审计

- G11.7a 必须重跑全部 13 个 P0 与已 go P1（M157）的**各自 assertion**；evidence 的 `base_commit` 必须落在同一候选 close-out 基线，零 skip/estimated；soak 量级沿 G10.8a 继承（≥1800s）或 measured 证明更短足够（具体阈值 G11.1 裁决 measured 标定，PLAN §2 G11.7a 字面）；`budget_eval --strict` 非空全 PASS。
- G11.7b 只有在 13 个 P0 key 全 PASS、已 go P1 key 全 PASS、验收映射/候选决策/RD 最终状态逐字一致、**复测差距清单终审锁定**（残余差距/未闭环行如实登记不冒充全闭环）时才可 status flip；任一 P0 无独立硬门则禁止 flip（PLAN §2.9）。
- 同日放行先例继承（立项裁决 7）：7a full-run 先行完成后允许同日进 7b close-out；条件实现刚绿不得跳过 7a 直接 close。
- **G11 闭环判据 + 零绝对通过线口径维持到 close-out**：修复闭环判据 = 修复前后度量 delta 收敛 measured；任何「已达 UE5 画质」叙述在 G11 期内一律不成立（契约 §5 字面）。
- 后续若治理流程将新的 P1 判为 go，须先按治理流程修订本表及覆盖集合，再开对应实现；不得把它静默并入现有 key。

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-16 | G11.1 初版：冻结 13 个 P0 的 symbolic gate、目标脚本/schema、独立判据（契约 §4.2 逐字）与最晚波次——口径对齐闭环 3 行（M144~M146，G11.2）+ 资产场景修复闭环 6 行（M147~M152，G11.3）+ 光照 GI 修复闭环 2 行（M153/M154，G11.4）+ 复测与回归 2 行（M155/M156，G11.5）；已 go P1 一行（M157 hdr_flip_calibration，G10-N10 承接锚兑现）同构登记；§3 条件型/not-triggered 登记面（异己并发工作树面 / 三行触发评估 / GPU 管线画质差距面边界）；§4 key 命名空间三方逐字一致机器可核声明 + 锁定基线 delta 溯源表；单一命名空间 `g11.p{0,1}.m###.<slug>` + `ci/g11_<slug>_smoke.py` + `g11_m###_<slug>_evidence_schema.json` 由 `ci/check_g11_acceptance_map.py` 三向比对强制；§5 治理覆盖与空行门、§6 G11.2 硬互锁六条件、§7 Close-out 审计。数字 CI 步骤全部 `post-interlock actual-next-free allocation`（当前实测 CI_step next_free=196），零 workflow/script/schema 预放。 |
| v1.1 | 2026-08-16 | **G11.3 收口 M147 判据双 phase 校准注（只追加登记，§1/§2 行字面 0-byte）**：§3.4 新增 M147 双 phase 口径登记（沿 G10 M130 §3.3 双阶段先例；契约 §8.3a 修订句为裁决事实源）——`--phase g11.3` = 修复落盘+局部度量 verdict 显式登记面（deferred_to_g11_5 ∧ convergence_pending=true 或实测收敛；pending 缺登记冒充全闭环即 RED），`--phase g11.5` = 收敛断言面（RXS-0393 L2 definitive 测量面，阈值标定程序产禁手写，不收敛则整波 FAIL，当前 fail-closed 拒跑）；schema anyOf 双支承载 v1 legacy/v2 g11.3 phase；g11.3 phase 绿不替 g11.5 收敛断言充绿（wave3 fact⑥ `m147_dual_phase_discipline` 两态机核，沿 G10.8a wave2 fact④ 先例）。`Assisted-by: Kimi-K3（G11.3 收口）` |
| v1.2 | 2026-08-16 | **G11.5 波 M155/M156 materialize + M147 g11.5 phase 落地注（只追加登记，§1/§2 行字面 0-byte）**：§3.4 追加 G11.5 落地注（M147 `--phase g11.5` 收敛断言面落地——fail-closed 翻真跑，definitive 测量面 = G11.5 同契约复跑帧区，不收敛则整波 FAIL）；M155/M156 两门按 §1 冻结行 materialize（ci/g11_ab_retest_closure_smoke.py / ci/g11_regression_guard_smoke.py + 双 schema，数字步骤按落盘前实测 CI_step.next_free=211 顺位领取 211/212，wave5 聚合门 213——分配登记面 = CI_GATES v1.5 修订行 + ledger v1.119）；三向比对维持（§1 M155/M156 行字面 0-byte——check_g11_acceptance_map 三向 PASS）。`Assisted-by: Kimi-K3（G11.5 波）` |
