# G5 CI_GATES — 原生渲染器期机器门

> 契约:[G5_CONTRACT.md](G5_CONTRACT.md) · 计划:[G5_PLAN.md](G5_PLAN.md)
> 通用纪律:host 段恒跑(无 GPU 也绿);device 段 gate real(`RURIX_REQUIRE_REAL=1` 翻硬红,缺 provisioning SKIP = dev-env degrade,mock/SKIP 不充绿);evidence JSON 落 `evidence/<subject>_<ts>.json` 过 `ci/check_schemas.py`;budget counter 与 `ci/budget_eval.py` evaluator 分支同实现 PR 落。

---

## 1. 既有守卫(全程恒跑,零回归)

`cargo fmt --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo test --workspace` / `py -3 ci/{check_number_ledger,check_schemas,check_structure,check_guardrails,check_contribution,trace_matrix --check,budget_eval}.py`;既有步骤 41~81 判据 0-byte 只增(步骤 70 = G3 showcase 永久 gap;步骤 69 blocked 探针恒跑)。

## 2. 新步骤拟分配(步骤 82 起;数量随实现回填不预占,多余号作废声明 burned)

| 步骤(拟) | 脚本(拟) | host 段(恒跑) | device 段(gate real) | 对应门 |
|---|---|---|---|---|
| 82 | `ci/renderer_graph_smoke.py` | rurix-render graph 库单测(四趟编译/EB 屏障 golden/transient 别名峰值/校验 RED 自检/图 dump) | —(纯 host 门,check_* 风格) | G-G5-3 |
| 83 | `ci/renderer_draw_smoke.py` | 派发桥库单测 + corpus | .rx gfx 图真派发三角形 readback 像素断言 | G-G5-4 |
| 84 | `ci/renderer_visbuffer_smoke.py` | geom-build meshlet/CPU 参照剔除单测 | GPU 剔除对拍 + VisBuffer SW/HW diff | G-G5-5 |
| 85 | `ci/renderer_lighting_smoke.py` | VSM 页表/GI 参考器/AS 管理单测 | VSM/GI/RTAO device 对拍 | G-G5-6 |
| 86 | `ci/renderer_temporal_smoke.py` | 时域底座/TAA/TSR 单测 + SSIM 参考 | TAA/TSR 收敛 device 见证 | G-G5-7 |
| 87 | `ci/uc06_renderer_smoke.py` | demo host 装配核验 | 全管线 device 真跑 + readback + 时间戳 evidence | G-G5-8 |

## 3. evidence schema(随 smoke 同 PR 落 milestones/g5/)

`renderer_draw_smoke_evidence_schema.json` / `renderer_visbuffer_smoke_evidence_schema.json` / `renderer_lighting_smoke_evidence_schema.json` / `renderer_temporal_smoke_evidence_schema.json` / `uc06_renderer_smoke_evidence_schema.json`(镜像 g4 体例:schema_version/subject/step/host_section_pass/device_section_rc/checks/<subject>_ok/run_url/timestamp;性能数字入 checks 不进硬门)。

## 4. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-07-29 | 初版(G5 开工;步骤号拟分配,随实现回填) |
