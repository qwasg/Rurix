# G6 CI_GATES — 渲染物理双轨期机器门

> 契约:[G6_CONTRACT.md](G6_CONTRACT.md) · 计划:[G6_PLAN.md](G6_PLAN.md)
> 通用纪律:host 段恒跑(无 GPU 也绿);device 段 gate real(`RURIX_REQUIRE_REAL=1` 翻硬红,缺 provisioning SKIP = dev-env degrade,mock/SKIP 不充绿);evidence JSON 落 `evidence/<subject>_<ts>.json` 过 `ci/check_schemas.py`;budget counter 与 `ci/budget_eval.py` evaluator 分支同实现 PR 落。

---

## 1. 既有守卫(全程恒跑,零回归)

`cargo fmt --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo test --workspace` / `py -3 ci/{check_number_ledger,check_schemas,check_structure,check_guardrails,check_contribution,trace_matrix --check,budget_eval}.py`;既有步骤 41~87 判据 0-byte 只增(步骤 70 = G3 showcase 永久 gap;步骤 69 blocked 探针恒跑 RD-034;步骤 84~86 device 段 RD-038 分波探针按其自身轨道演进,G6 不改写)。

## 2. 新步骤拟分配(步骤 88 起;数量随实现回填不预占,多余号作废声明 burned)

| 步骤(拟) | 脚本(拟) | host 段(恒跑) | device 段(gate real) | 对应门 |
|---|---|---|---|---|
| 88 | `ci/physics_core_smoke.py` | rurix-physics 库单测(固定步确定性/堆叠沉降/睡眠唤醒/批插体不锁死主步/query 与 step 并发/ContactEvent 有界 drain/SyncBudget) | —(纯 host 门,check_* 风格;Jolt 为 CPU 库无 GPU 依赖) | G-G6-3 |
| 89 | `ci/physics_bridge_smoke.py` | 合流桥单测(GpuScene 单向同步/MV 供给/流送 body 批插移除 + R-G6-4 竞态注入)+ 渲染器不回写物理·不持原生指针代码审计 | 合流 demo 物理驱动变换 → 像素/变换非平凡断言 | G-G6-4 |
| 90 | `ci/physics_rapier_parity_smoke.py` | feature `rapier` 同场景对拍(变换/接触集合容差断言,非逐位)+ 默认 off 核验;无 CMake 路径可跑 | —(纯 host 门) | G-G6-5 |
| 91 | `ci/uc0x_physics_smoke.py`(demo 落点定名后回填) | demo host 装配核验 | 刚体场景 + 既有 VisBuffer/GI/VSM/TAA 管线全跑 + readback + 物理步耗时 evidence | G-G6-7 |

Taichi Vulkan AOT spike(G6.5)不占步骤号:可选交付,成功则判档后顺位续号,失败登记 RD(自 RD-042 顺位)留痕于契约 §8。

## 3. evidence schema(随 smoke 同 PR 落 milestones/g6/)

`physics_core_smoke_evidence_schema.json` / `physics_bridge_smoke_evidence_schema.json` / `physics_rapier_parity_evidence_schema.json` / `uc0x_physics_smoke_evidence_schema.json`(镜像 g5 体例:schema_version/subject/step/host_section_pass/device_section_rc/checks/<subject>_ok/run_url/timestamp;性能数字入 checks 不进硬门)。

## 4. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-07-30 | 初版(G6.1 开工;步骤号拟分配,随实现回填) |
