<!-- Assisted-by: Kimi-K3（G10.1 治理波起草） -->
# G10 CI_GATES — UE5 画面对标基线期机器门

> 契约：[G10_CONTRACT.md](G10_CONTRACT.md) v1.0 · 计划：[G10_PLAN.md](G10_PLAN.md) v1.0 · 能力矩阵：[G10_CAPABILITY_MATRIX.md](G10_CAPABILITY_MATRIX.md) v1.0 · 验收映射：[G10_ACCEPTANCE_MAP.md](G10_ACCEPTANCE_MAP.md) v1.0。
> 当前状态（v1.0，2026-08-15）：**G10.1 governance-only，G10.2+ blocked**（`implementation_status: blocked`）。本文 §4 的 12 个 P0 key 与 §4A 的 2 个 P1 key 当前全部未 materialize——脚本、schema、workflow 步骤一件未落；任何「G10.2 开工」叙述都不得当作 PASS。治理 validator 落地后必须诚实输出 `BLOCKED`，直到 §6 互锁条件同时为真。

---

## 1. 互锁与编号纪律

### 1.1 实现互锁

稳定治理 validator 名为 `ci/check_g10_implementation_interlock.py`，属于 `check_*` 类未编号守卫。其实现后必须读取事实源并逐项输出：

1. `milestones/g9/G9_CONTRACT.md` §8.10 的有效 status 是否为 `closed`（2026-08-15，flip commit `6ff73830` + 收口批 `c0cdfddd`）；G10.0 不可变 ref `c0cdfddd` 是否已登记；
2. 两份 Full RFC（画面对标与度量语义 / 外部参照 harness 与许可边界）是否均经 D-409 独立 provenance 对抗性评审后 Agent Approved（编号按立项时实测 `registry/number_ledger.json` namespaces.RFC `next_free` 领取）；
3. `G10_CANDIDATE_DECISIONS.md` 是否无空行，`registry/deferred.json` history 是否只追加、无静默改判，`G10_ACCEPTANCE_MAP.md` §1/§2 是否无缺行；
4. 用户 G10.2 开工指令是否留痕；workflow 与 ledger 的实际末号/`next_free` 是否一致。

全假或任一为假时 `BLOCKED` 是唯一正确结论；禁止把 `--expect-blocked` 一类测试模式当成互锁 PASS——它只能证明 validator 能识别阻断。G10.2 起每个实现 PR 必须把 `--require-ready` 作为前置 required check。互锁全绿后才允许 `src/`/`spec/`/`conformance/` 改动，且 spec 条款 PR 先于实现 PR（G10_PLAN §2 spec-first + RED 先行）。

### 1.2 数字步骤延迟分配

- G10 的稳定身份是本文件中的 `symbolic_gate_key` 与 `script`。所有未来编号栏统一写 **`post-interlock actual-next-free allocation`**。
- 只有 §6 互锁全绿后，才可同时读取 `.github/workflows/pr-smoke.yml` 与 `registry/number_ledger.json`，按合入时实际 `next_free` 给即将 materialize 的脚本分配数字步骤，并在同一 PR 追加 ledger 校准。**当前实测 `CI_step.next_free=173`（G9 已消费至 172，[G9 CI_GATES](../g9/CI_GATES.md) v1.21 / ledger v1.101）；G10 编号自互锁后实测 `next_free` 顺位领取，禁预占、禁沿用任何草案建议值**。
- 不创建“预留” workflow step、空 YAML job、空脚本、永远 PASS 的 schema 壳或注释占位。脚本 + RED/GREEN 自检 + schema + workflow 真步骤 + ledger 校准同一实现 PR 落。

### 1.3 三层 CI 口径

- **PR Smoke**（`.github/workflows/pr-smoke.yml`）：G10 各 P0/P1 门与波聚合门的常驻承载层；数字步骤按 §1.2 纪律 post-interlock materialize；device 门 env 双置 `RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1`（沿 G9 体例）。
- **Nightly / full-run**：G10.8a soak full-run 与全链路连续复跑的承载层（本地 / `workflow_dispatch` 产 evidence，pr-smoke 侧 `--verify-latest` 秒级核最新 full-run evidence，沿 G8.8a/G9.8a 体例）；soak 量级沿 G9.8a 继承或 measured 证明更短足够，阈值 G10.1 裁决 measured 标定。
- **Release**：不新增 G10 专属 release 门；Release 面只消费 G10.8b close-out 终审 evidence 与终审锁定的差距清单（G11 法定输入）。
- 三层接线形态一律 post-interlock materialize；本文只冻结口径与 symbolic 身份，不以文档表格冒充 workflow 接线。

## 2. 既有守卫与 0-byte 边界

G10.1 可运行且不得改弱的既有守卫：

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
py -3 ci/check_number_ledger.py
py -3 ci/check_schemas.py
py -3 ci/check_structure.py
py -3 ci/check_guardrails.py <g9-close-ref-or-pr-base>
py -3 ci/check_contribution.py
py -3 ci/trace_matrix.py --check
py -3 ci/budget_eval.py
```

G5~G9 已 materialize 的全部数字 CI 步骤判据 0-byte 只增；G5~G9 四件套/决策表/evidence schema/budget 0-byte，closed 判据不回写。G10.1 不改 `.github/workflows/pr-smoke.yml`，也不以文档表格冒充 workflow 接线；spec/conformance/registry 在 G10.1 期 0-byte（registry 登记/翻转/history 追加归立项治理动作，与本文件无关）。UE 源码仅外部参照只读（零 vendoring、零片段复制）；压测资产二进制不入 git（外部缓存 K: 盘 + 仓库内元数据登记）。

## 3. G10.1 governance-only 机器门

| Symbolic gate key | 稳定脚本/检查 | 数字步骤 | 判据 |
|---|---|---|---|
| `g10.gov.structure` | 既有 `ci/check_structure.py` + `ci/check_schemas.py` | 不编号（`check_*`） | CONTRACT/CI/decision/map/RFC 结构一致；map 中预定 schema 名唯一，实际 schema 只与对应脚本同 PR 落，不预建空壳 |
| `g10.gov.number_isolation` | 既有 `ci/check_number_ledger.py` | 不编号（`check_*`） | 两份 RFC claim 与既有命名空间隔离；RXS/RD/U/RX/数字 CI 零推测 claim、零草案建议号沿用 |
| `g10.gov.implementation_interlock` | `ci/check_g10_implementation_interlock.py` | 不编号（`check_*`） | 当前应诚实报 `BLOCKED`；仅 §6 互锁条件全绿时才输出 READY receipt |
| `g10.gov.acceptance_coverage` | `ci/check_g10_acceptance_map.py` | 不编号（`check_*`） | 12 个 P0 + 2 个已 go P1 key/script/schema/check 双向全覆盖；MAP §1 / CONTRACT §4.2 / 本文 §4 三向逐字一致（P1 行 MAP §2 ↔ 本文 §4A 双向比对）；候选决策表无缺行 |
| `g10.gov.measured_baseline` | 既有 `ci/budget_eval.py` + `ci/check_g10_budget_baseline.py` | 不编号（`check_*`） | `g10_budget.json` 非空 measured_local、零 estimated（P-09），counter 与 evaluator 同步；当前不得声称实现性能通过 |

这些 validator 可以在 G10.1 落地，但不得带数字“步骤 NN”，也不得把 G10.2 目标脚本接进 workflow。

## 4. 12 个 P0 独立机器断言

下表的 key 与脚本名冻结，与 [G10_ACCEPTANCE_MAP.md](G10_ACCEPTANCE_MAP.md) §1 逐字一致，由 `ci/check_g10_acceptance_map.py` 三向比对强制。每一行均须独立 evidence subject 和独立结果；同一 workflow 进程可以顺序调用多个脚本，但任一行 `FAIL`、`SKIP` 或 `DEV_ENV_DEGRADE` 都必须保持可见，聚合结果不得 PASS。`numeric_step` 一律为 `post-interlock actual-next-free allocation`。Evidence schema 只冻结目标路径不预建文件；schema 形态见 §7。

| symbolic_gate_key | M## | 最晚波次 | script | evidence schema（目标路径） | 判据摘要 |
|---|---:|---|---|---|---|
| `g10.p0.m128.ue5_capture_environment` | M128 | G10.2 | `ci/g10_ue5_capture_environment_smoke.py` | `milestones/g10/g10_m128_ue5_capture_environment_evidence_schema.json` | 裁决路径落地 + UE 5.8 侧出帧成功 + 环境画像随证据存档；非零退出冒充成功/预置假帧即 RED |
| `g10.p0.m129.ue5_reference_frames` | M129 | G10.2 | `ci/g10_ue5_reference_frames_smoke.py` | `milestones/g10/g10_m129_ue5_reference_frames_evidence_schema.json` | 逐场景参考帧落盘 + 双跑 digest 一致 + provenance 闭集；digest 不等/provenance 缺行即 RED |
| `g10.p0.m130.dual_determinism_contract` | M130 | G10.2 骨架 → G10.5 双端核验 | `ci/g10_dual_determinism_contract_smoke.py` | `milestones/g10/g10_m130_dual_determinism_contract_evidence_schema.json` | 相机/光照/时间参数同 schema 双端各一份 + digest 相等（双 phase：骨架期绿不替双端核验期充绿）；digest 不等仍出 A/B 报告即 RED（门序硬约束） |
| `g10.p0.m131.asset_license_registry` | M131 | G10.3 | `ci/g10_asset_license_registry_smoke.py` | `milestones/g10/g10_m131_asset_license_registry_evidence_schema.json` | 逐资产 license 白名单闭集 + SPDX + URL + attribution + digest；未登记混入/白名单外许可即 RED |
| `g10.p0.m132.corpus_loading` | M132 | G10.3 | `ci/g10_corpus_loading_smoke.py` | `milestones/g10/g10_m132_corpus_loading_evidence_schema.json` | 逐场景加载成功 + 三角形/材质/纹理计数非空 + 加载事件序列 golden；计数为零/静默丢场景即 RED |
| `g10.p0.m134.frame_capture_pipeline` | M134 | G10.4 | `ci/g10_frame_capture_pipeline_smoke.py` | `milestones/g10/g10_m134_frame_capture_pipeline_evidence_schema.json` | HDR 帧捕获落盘 + 捕获→回读逐像素往返无损 + 元数据齐备；位深截断/sRGB 混标注入即 RED |
| `g10.p0.m135.flip_metric` | M135 | G10.4 | `ci/g10_flip_metric_smoke.py` | `milestones/g10/g10_m135_flip_metric_evidence_schema.json` | 与参考实现逐图对拍一致（容差 measured 标定）+ 恒等图对 FLIP=0 + 版本 pin；参考扰动注入即 RED |
| `g10.p0.m136.ssim_psnr_metric` | M136 | G10.4 | `ci/g10_ssim_psnr_metric_smoke.py` | `milestones/g10/g10_m136_ssim_psnr_metric_evidence_schema.json` | 口径冻结进 spec + 参考对拍一致 + 恒等图对 SSIM=1/PSNR=inf；口径漂移注入即 RED |
| `g10.p0.m137.pixel_diff_report` | M137 | G10.4 | `ci/g10_pixel_diff_report_smoke.py` | `milestones/g10/g10_m137_pixel_diff_report_evidence_schema.json` | diff 热区图 + 逐区域统计落盘 + schema 闭集；diff 图与标量不一致/空场景行即 RED |
| `g10.p0.m139.ab_comparison` | M139 | G10.5 | `ci/g10_ab_comparison_smoke.py` | `milestones/g10/g10_m139_ab_comparison_evidence_schema.json` | 场景全集双端出图 + 度量报告 + 差距清单落盘；缺场景行/单端缺帧聚合 PASS/M130 digest 不等出报告即 RED |
| `g10.p0.m140.gap_registry` | M140 | G10.5 | `ci/g10_gap_registry_smoke.py` | `milestones/g10/g10_m140_gap_registry_evidence_schema.json` | 每差距项带 UE5 Renderer 模块归属 + measured delta + 建议 P 级 + G11 承接锚；缺归属/缺锚/非 measured 叙述即 RED |
| `g10.p0.m141.perf_baseline` | M141 | G10.5 | `ci/g10_perf_baseline_smoke.py` | `milestones/g10/g10_m141_perf_baseline_evidence_schema.json` | 双端同场景帧率采样（14 §5 协议）+ 环境画像存档 + 交替采样顺序登记；未锁频/画像缺字段/轮数不足即 RED |

> **单一命名空间**：本文件、`G10_CONTRACT.md` §4.2、`G10_ACCEPTANCE_MAP.md` §1/§2 必须引用同一份 key/脚本；`g10.p{0,1}.m###.<slug>` + `ci/g10_<slug>_smoke.py` 为唯一合法形态，由 `ci/check_g10_acceptance_map.py` 三向比对强制。**G10 不设画质通过阈值与帧率通过线**——差距全量 measured 登记即绿（契约 G-G10-7 / 立项裁决 5）。

## 4A. 已 go P1 独立机器断言（两行：M133 / M138）

契约 §4.2 末段「M133（清单冻结）/M138（阈值标定）为 P1，入验收映射随主门核验」只追加登记：下表两行与 [G10_ACCEPTANCE_MAP.md](G10_ACCEPTANCE_MAP.md) §2 逐字一致，由 `ci/check_g10_acceptance_map.py` 双向比对强制（§4 P0 三向比对 0-byte 不改弱）。`numeric_step` 一律为 `post-interlock actual-next-free allocation`，待各门脚本/schema/workflow 步骤 materialize 时按 §1.2 纪律落盘实测回填；本节不预建空脚本、空 schema 壳或占位 workflow 步骤。

| symbolic_gate_key | M## | 最晚波次 | script | evidence schema（目标路径） | 判据摘要 |
|---|---:|---|---|---|---|
| `g10.p1.m133.corpus_list_freeze` | M133 | G10.3 | `ci/g10_corpus_list_freeze_smoke.py` | `milestones/g10/g10_m133_corpus_list_freeze_evidence_schema.json` | 场景清单版本化冻结 + 清单 digest 注册在树 + 变更只追加修订行；无修订行/未注册 digest 冒充/行集不对账即 RED |
| `g10.p1.m138.metric_threshold_calibration` | M138 | G10.4 | `ci/g10_metric_threshold_calibration_smoke.py` | `milestones/g10/g10_m138_metric_threshold_calibration_evidence_schema.json` | 标定程序可复跑 + 标定值入 `g10_budget.json`（measured_local）provenance 齐备（P-09）；手写阈值/estimated 冒充/不可复跑即 RED |

## 5. 波聚合门与收口机器门清单

以下门脚本名/ symbolic key 一并冻结（同 §1.2：数字步骤一律 `post-interlock actual-next-free allocation`，不预占）。波聚合门为薄壳只读聚合（沿 `ci/g9_wave{N}_exit_check.py` + `ci/g9_wave_exit_lib.py` 同构体例）：聚合不代绿、不重跑 smoke、不设 `RURIX_REQUIRE_REAL`，聚合 PASS 不遮蔽任一子断言 FAIL/SKIP/DEV_ENV_DEGRADE；`required_gates` 闭集与 `aggregate_read_only const true` 进各自 evidence schema。G10.6 重评窗门/G10.7 决策门/G10.8a soak/G10.8b closeout 沿 `ci/g9_p2_decisions_check.py` / `ci/g9_stabilization_soak.py` / `ci/g9_closeout_check.py` 同构体例。

| symbolic_gate_key | 波次 | script | evidence schema（目标路径） | 判据摘要 | numeric_step |
|---|---|---|---|---|---|
| `g10.wave.2.exit` | G10.2 | `ci/g10_wave2_exit_check.py` | `milestones/g10/g10_wave2_exit_evidence_schema.json` | M128/M129 + M130 骨架期最新 evidence 只读汇总；裁决路径与回退臂诚实登记；Epic 人工接管点未完成只登记 dev_env_degrade 不充绿 | post-interlock actual-next-free allocation |
| `g10.wave.3.exit` | G10.3 | `ci/g10_wave3_exit_check.py` | `milestones/g10/g10_wave3_exit_evidence_schema.json` | M131/M132/M133 汇总：许可登记零缺行、清单全场景加载绿、清单 digest 在树 | post-interlock actual-next-free allocation |
| `g10.wave.4.exit` | G10.4 | `ci/g10_wave4_exit_check.py` | `milestones/g10/g10_wave4_exit_evidence_schema.json` | M134/M135/M136/M137/M138 汇总：四条 RED 臂独立有效、标定值入 g10_budget 且 provenance 齐备（P-09） | post-interlock actual-next-free allocation |
| `g10.wave.5.exit` | G10.5 | `ci/g10_wave5_exit_check.py` | `milestones/g10/g10_wave5_exit_evidence_schema.json` | M139/M140/M141 + M130 双端核验期（`phase_g10_5_pass==true`）汇总；门序硬约束留痕（digest 不等不得出报告）；差距清单场景全集零空行、not-ready 行显式在列 | post-interlock actual-next-free allocation |
| `g10.wave.6.reevaluation` | G10.6 | `ci/g10_wave6_reevaluation_check.py` | `milestones/g10/g10_wave6_reevaluation_evidence_schema.json` | G9 十项 defer 逐行重判核验零空行（G10.5 measured 数据为法定证据输入）；命中者只追加程序重判 go 并指定 G11+ 承接波次，未命中者维持 defer 承接锚字面 0-byte；deferred history 只追加禁静默改判 | post-interlock actual-next-free allocation |
| `g10.wave.7.decisions` | G10.7 | `ci/g10_p2_decisions_check.py` | `milestones/g10/g10_p2_decisions_evidence_schema.json` | G10 期全部 P2/留档/未触发分项逐条 go/no-go/defer-to-G11+ 零空行；defer 必有承接锚（机核同构 `ci/g9_p2_decisions_check.py`）；no-go/defer 如实保持 open，不阻塞 soak 且不得写进全绿叙述 | post-interlock actual-next-free allocation |
| `g10.wave.8a.soak` | G10.8a | `ci/g10_stabilization_soak.py` | `milestones/g10/g10_stabilization_soak_evidence_schema.json` | 全部 P0 与 go 的 P1 全量回归（M130 走 `--phase g10.5` 腿）；G5~G9 既有判据 0-byte；出图/捕获/度量/差距清单全链路连续复跑 soak（量级沿 G9.8a 继承或 measured 证明更短足够，阈值 G10.1 裁决 measured 标定）；`budget_eval --strict` 非空、零 estimated/skip | post-interlock actual-next-free allocation |
| `g10.wave.8b.closeout` | G10.8b | `ci/g10_closeout_check.py` | `milestones/g10/g10_wave8b_closeout_evidence_schema.json` | 验收映射、候选决策、RD 最终状态逐字一致；全部 P0 独立断言均 PASS；evidence/schema/预算终审；差距清单终审锁定为 G11 法定输入；§8 只追加后 status active→closed 前置 | post-interlock actual-next-free allocation |

## 6. G10.2 互锁

`G10.GOV.G10_2.ENTRY_INTERLOCK` 条件与判据字面见 [G10_ACCEPTANCE_MAP.md](G10_ACCEPTANCE_MAP.md) §6（G9 closed + G10.0 不可变 ref `c0cdfddd` 登记 + 两份 Full RFC 经 D-409 评审 Agent Approved + 决策表/验收映射无缺行且 deferred history 只追加 + acceptance_coverage 与 measured_baseline 双 PASS + 数字步骤按互锁后 actual next_free 分配 + 用户 G10.2 开工指令留痕）。互锁未输出 READY 前：禁止合入 G10.2 的 `spec/`、`conformance/`、`src/` 或 workflow 实现改动；禁止 claim 任何数字 CI step；spec-first + RED 先行自互锁通过后才启动。`check_g10_implementation_interlock --require-ready` 输出 READY 是机器事实，不以叙述替代（契约 G-G10-3）。

## 7. Evidence 形态（沿 G9 schema 范式）

- 每门 evidence 顶层至少含：`schema_version` / `subject` / `milestone`（`G10`）/ `wave` / `assertion_id`（必须等于对应 `symbolic_gate_key`）/ `status`（`pass|fail`；`skip|estimated|advisory` 不充绿）/ `commands` / `environment` / `base_commit` / `run_url` / `timestamp`（UTC）。
- 治理与聚合门形态：`symbolic_gate_key`（const 钉死）/ `host_section_pass`（boolean）/ `device_section_state`（enum：`not_applicable|executed|dev_env_degrade`）/ `checks`（键集闭集全 boolean，逐条打印不以总 `all_pass` 掩盖）/ 聚合门加 `required_gates`（闭集 minItems=maxItems）与 `aggregate_read_only const true`；`numeric_step` materialize 时 const 钉死实测真号。
- evidence 落盘 `evidence/g10_<slug>_<UTC>.json` 新文件不覆盖既有件（只增不删不改）；文件名 UTC stamp 机核新鲜度。
- 条件语义：`SKIP=not-triggered` 只表示决策已记录，`DEV_ENV_DEGRADE` 只表示环境缺失（含 Epic 账号人工接管点未完成、UE 出帧时域未收敛场景行 not-ready），两者均不充 P0 绿、不反向否决其他可在当前环境全量验证的面（MAP §3）。

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.2 | 2026-08-15 | G10.2 UE5 出图环境波 materialize（G-G10-3 互锁 READY 后实现波；落盘前实测 `CI_step.next_free=177` 顺位领取 177~180、M130 spec 条款实测 `RXS.next_free=384` 顺位领取 RXS-0384）：**步骤 177** = `ci/g10_ue5_capture_environment_smoke.py --gate g10.p0.m128.ue5_capture_environment`（§4 M128 行兑现：②Launcher UE 5.8.1-56057345 @ F:\UE_5.8 Build.version 实测 + Entry 静态空图 MRQ 臂真出帧〔gpu_device_lock 串行、mtime 新鲜度机核、EXR magic+体积下限真帧判据〕+ 环境画像七元组随 evidence 存档 + red_fixtures/m128/ 三 RED 臂 + live 非零退出探针全检出）/ **步骤 178** = `ci/g10_ue5_reference_frames_smoke.py --gate g10.p0.m129.ue5_reference_frames`（§4 M129 行兑现：暂定场景集〔RFC-0027 §4.4 F8 形态，entry-empty-static 单场景闭集 + deviation_note 如实登记〕逐场景参考帧落盘 + 同参数双跑 canonical digest 一致〔harness g10_determinism 14 属性剥离单一事实源〕+ provenance 七元组闭集 + red_fixtures/m129/ 三 RED 臂〔真实不等帧对/缺行/像素区篡改〕全检出）/ **步骤 179** = `ci/g10_dual_determinism_contract_smoke.py --gate g10.p0.m130.dual_determinism_contract --phase g10.2`（§4 M130 行骨架期兑现：双端 schema 各一份〔Rurix 侧骨架参考解析器按 RXS-0384 L3 字节布局独立实现 + UE 侧 harness g10_param_contract.py〕+ digest 比对面就位 + 边界浮点差分语料跨端逐位一致 + 漂移/schema 外字段/非单位四元数/NaN 四 RED 臂；--phase g10.5 本波 fail-closed 拒跑留 G10.5，evidence 标 phase_g10_2_pass=true、phase_g10_5_pass=false）/ **步骤 180** = `ci/g10_wave2_exit_check.py --gate g10.wave.2.exit`（§5 wave2 聚合门兑现：三门只读汇总 + RXS-0380/RXS-0384 条款头 + RFC-0026/0027 Approved + 场景集登记 + M130 phase 纪律，聚合不代绿）。同批落：spec/visual_comparison.md 新建（RXS-0384，spec-first 先行）+ 四 evidence schema（g10_m128/g10_m129/g10_m130/g10_wave2_exit）+ ci/check_schemas.py 三处纯追加（load/validator/前缀路由，与既有全族互不包含，既有路由 0-byte）+ ci/g10_ue5_lib.py 共享判定层 + milestones/g10/g10_2_provisional_scene_set.json + harness g10_param_contract.py DRAFT 布局随 spec 冻结替换（SPEC_BYTE_LAYOUT）+ harness/examples/contract_params_entry_smoke.json + conformance/visual_comparison/ 锚定语料四件 + pr-smoke.yml 步骤 177~180（步骤 176 块后追加）+ trace_matrix 365→366 全锚定 + stable 快照 365→366 重 bless。§4/§4A/§5 表体 0-byte（numeric_step 经本校准回填，不回写表体）；数字领取以 registry/number_ledger.json revision_log v1.104 为据。 |
| v1.1 | 2026-08-15 | G10.3 压测语料波 materialize（G-G10-3 互锁 READY 后首波之一；落盘前实测 `CI_step.next_free=173` 顺位领取 173~176）：**步骤 173** = `ci/g10_asset_license_registry_smoke.py --gate g10.p0.m131.asset_license_registry`（§4 M131 行兑现：白名单闭集 {CC0-1.0, CC-BY-3.0, CC-BY-4.0} + 按类登记零缺行 + attribution 子字段闭集 + 清单级 canonical digest 缓存复算 + git 零二进制守卫 + red_fixtures/m131/ 五 RED 臂全检出）/ **步骤 174** = `ci/g10_corpus_loading_smoke.py --gate g10.p0.m132.corpus_loading`（§4 M132 行兑现：逐场景 rxcook 真实加载 + 计数非空 + 计数/六表全等 golden + 事件序列 golden + 静默丢场景零 + red_fixtures/m132/ 三 RED 臂全检出）/ **步骤 175** = `ci/g10_corpus_list_freeze_smoke.py --gate g10.p1.m133.corpus_list_freeze`（§4A M133 行兑现：清单 digest 注册在树 + 只追加修订程序 + M131/M132 行集对账 + ready 下界 vacuous 拦截 + red_fixtures/m133/ 三 RED 臂全检出）/ **步骤 176** = `ci/g10_wave3_exit_check.py --gate g10.wave.3.exit`（§5 wave3 聚合门兑现：三门只读汇总 + RXS-0380~0383 条款头 + RFC-0027 Approved + 注册表零缺行 + 清单 digest 在树，聚合不代绿）。同批落：spec/external_reference.md 新建（RXS-0380~0383，spec-first 先行）+ 四 evidence schema（g10_m131/g10_m132/g10_m133/g10_wave3_exit）+ ci/check_schemas.py 三处纯追加（load/validator/前缀路由，与既有全族互不包含，既有路由 0-byte）+ ci/g10_corpus_lib.py 共享判定层 + ci/_gen_g10_cornell_box.py 生成器 + 治理面登记件（g10_asset_license_registry.json / g10_corpus_scene_manifest.json / g10_corpus_loading_golden.json / license_snapshots/ / corpus/ / red_fixtures/）+ .gitignore G10 守卫块只追加 + pr-smoke.yml 步骤 173~176（步骤 172 块后追加）。§4/§4A/§5 表体 0-byte（numeric_step 经本校准回填，不回写表体）；数字领取以 registry/number_ledger.json revision_log v1.103 为据。 |
| v1.0 | 2026-08-15 | G10.1 初版：冻结治理/实现双门、12 个 P0 独立 key 与脚本（与 G10_CONTRACT §4.2 / G10_ACCEPTANCE_MAP §1 逐字一致）+ 2 个已 go P1 key（§4A，与 MAP §2 逐字一致）；§5 波聚合门与收口门清单（wave2~wave5 exit + wave6 重评窗 + wave7 p2 决策 + wave8a soak + wave8b closeout）脚本名冻结；`g10.gov.*` 五个 governance-only 机器门全不编号；三层 CI（PR Smoke/Nightly/Release）口径冻结；§7 evidence 形态沿 G9 schema 范式；全部 numeric_step 延迟为 `post-interlock actual-next-free allocation`（当前实测 CI_step next_free=173，G9 已消费至 172）；零 workflow/script/schema 预放，当前实现门诚实 blocked。 |
