# M24 `g8.p0.m24.tsr_contract` — Gov 接线清单

实现 agent 已落地 host/device/smoke/schema/local freeze；下列治理热点由 **Gov** 接线（实现 agent 禁改）。

## 必接

1. **`registry/number_ledger.json`**：领取 `CI_step`（当前占位 `numeric_step=0`），回写 smoke/`NUMERIC_STEP` 与 evidence。
2. **`milestones/g8/CI_GATES.md` / `G8_CONTRACT.md`**：将 `g8.p0.m24.tsr_contract` 的 step 从 `post-G7 actual-next-free allocation` 改为实号。
3. **`.github/workflows/pr-smoke.yml`**：挂载  
   `py -3 ci/g8_tsr_contract_smoke.py --gate g8.p0.m24.tsr_contract`  
   （`RURIX_REQUIRE_REAL=1`，缺设备不得充绿）。
4. **`ci/check_schemas.py` / `ci/budget_eval.py`**：登记 `g8_m24_tsr_contract_evidence_schema.json`。
5. **RFC-0019 加性修订行**：冻结五 case golden digest + 逐 case tolerance（自 `tests/tsr_contract/freeze.json` 提升）。
6. **`milestones/g8/g8_budget.json`**：写入 measured 条目（含 `resurrection_age_max=K`）。
7. **`G8_CAPABILITY_MATRIX.md` / traceability**：M24 行 ⬜→实测指针。

## 两段式 tolerance

| 阶段 | 状态 | 谁 |
|---|---|---|
| `measured_local_freeze` | `tests/tsr_contract/freeze.json` | 实现（本 PR） |
| `rfc_budget_frozen` | RFC-0019 + g8_budget | Gov |

`checks.tolerance_frozen` 在 local freeze 在位时为真；`tolerance_stage.rfc_budget_gov_pending=true` 直至 Gov 完成 5–6。

## 勿做

- M28 维持 no-go。
- 不得以 TAA / 单帧 TSR / SKIP=dev-env 充绿。
