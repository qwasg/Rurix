# G37 W6 门修复子任务：ci/g31_svt_smoke.py 互斥登记态改造

日期：2026-08-30 ｜ 波次：G37.W6 ｜ 门：`g31.waveC.svt` ｜ 处置定案 = **互斥登记态（MUTEX_REGISTERED）**

## 1. 背景与根因（前役遗留,非本役引入）

- day_0828 Phase B texel heap 化起,harness（`g31_window_present.rs` 约 L8706,g14_3 车道体共享 fail）对 `--svt on` **无条件 fail-closed**（CLI 校验期即拒,rc=1,零 GPU 消耗）,字面：
  `--svt on 与 day_0828 Phase B texel heap 纹理形态互斥（SVT 假设 = 2048 网格图集/texmeta origin/tritex 步幅 1,heap 化未适配——fail-closed 登记,SVT 深修归后续波）`
- 门真跑四腿（b4anchor/full/small_a/small_b）中 full/small×2 必红;红路径还把占位值 FAIL 裁决件写进 `evidence/`,令 `check_schemas` 连带红。W6 首跑实证：`../W6_GATES.json` svt 行（rc=1,wall 128.9s）。

## 2. 改造结构（ci/g31_svt_smoke.py,修改注释「G37 W6 svt mutex_registered」）

复用既有 dev-env probe 短跑（`--svt on --svt-pool-tiles 0`,frames=2/warmup=1,极小参数——互斥在 CLI 校验期必现,不新增 GPU 腿）,在其输出上**先探互斥字面**,再走既有 skipped_dev_env 判：

1. `detect_svt_mutex(out, rc)`：rc≠0 **且** 字面 `--svt on 与 day_0828 Phase B texel heap 纹理形态互斥` 在输出 ⇒ 返回捕获的完整字面行;其他 fail 字面（interference 不符）/rc=0 ⇒ 空串,**不入登记态**。判定只看 rc+字面,stale evidence 文件不参与（旧 probe 逻辑曾被 `.tmp` 残留件掩蔽）。
2. 命中 ⇒ `mutex_registered_exit(...)`（真跑四腿前拦截）：
   - **host 金标准腿硬门**（已在互斥探测前照跑照判,纯 host/CPU 面）：`cargo test streaming::svt`（SVT-1/2/3 页表/反馈闭环/border 的 host 金标准）+ SVT-4 维持 defer 面（terrain 零 SVT 断言字面 grep + `cargo test world::terrain`）。任一红 ⇒ **整体 FAIL 退 1 不产件**（host 面坏了不能靠登记态掩盖）。
   - 全绿 ⇒ 落盘 `evidence/g31_svt_mutex_registered_<utc>.json`（落盘前对新 schema Draft7 **自校验硬门**,红即退 1 不冒充）,print `GATE MUTEX_REGISTERED g31.waveC.svt（非 PASS 非 FAIL,三态之外的登记态…深修归后续波 TODO #33-#36）`,**退 0**。
3. 未命中（将来深修互斥解除）⇒ 回落**既有全量判读 0 改动**（四腿/facts/v1 双 schema 路径原样）。

登记件内容：互斥字面全行、probe 参数形状与 rc、host 金标准腿结果（含恒等页表 digest CI 独立重算 + 确定性双算）、跳过的 device 四腿清单、深修锚（TODO #33-#36 SVT 四行 open + day_0828 HANDOVER §12 + W6_GATES.json svt 行首证）、environment。

## 3. 新 schema 与注册

- **新 schema**：`milestones/g31/g31_svt_mutex_registered_schema.json`,id = `rurix.g31.svt_smoke.mutex_registered.v1`。要点：`state` const `MUTEX_REGISTERED`、`mutex_literal` pattern 钉互斥字面、`host_golden_legs.*` const true（产件前置 = host 全绿,冒充红腿必被 schema 拒）、`registered_wave` const `G37.W6`。既有 v1 双 schema（`g31_svt_evidence_schema.json` / `g31_svt_gate_evidence_schema.json`）**0-byte**。
- **注册**：新脚本 `ci/_patch_g31_svt_mutex_schemas.py`（`_patch_g31_encode_parity_schemas.py` 同法,幂等）,对 `ci/check_schemas.py` 三处纯追加（load / validator / `g31_svt_mutex_registered_` 前缀路由,锚 = C13 svt 族三处块;新前缀与 `g31_svt_gate_`/`g31_svt_harness_` 第九字符 g/h/m 分岔互不包含）。未注册则登记件落 gpu fallthrough 必红——已验证路由生效。

## 4. 验证（本子任务禁 GPU/禁 cargo;selftest/py_compile/check_schemas 面）

| 项 | 结果 |
| --- | --- |
| `py -3 ci/g31_svt_smoke.py --selftest` | **PASS 47 项全绿** = 既有 34 项 0 破坏 + 新 13 项（字面命中捕获正例、rc=0 红臂、字面不符红臂、host 腿红×2、登记件 Draft7 正例、冒充 PASS/字面不符/host 红字面三红臂、schema 在树 + const 互核×2、全绿判正例） |
| `py -3 -m py_compile`（门脚本 + patch 后 check_schemas） | 绿 |
| `py -3 ci/_patch_g31_svt_mutex_schemas.py` | 三处插入 + 重读核验 + py_compile 绿;复跑幂等 |
| `py -3 ci/check_schemas.py` | **PASS**（含哨兵登记件在树时亦 PASS = 新路由生效实证） |
| 登记态干跑 `dryrun_mutex_path.py`（真函数,无 GPU/cargo） | 正臂 rc=0 产件合法;红臂（svt host 单测红）rc=1 **不产件**。哨兵件验后挪本目录留样 `sample_mutex_registered_evidence.json`,evidence/ 不留伪造件 |

## 5. 连带处置：W6 首跑红裁决件归档

W6 首跑（改造前）把占位值 FAIL 裁决件写进 `evidence/g31_svt_gate_20260830T070732Z.json`（未跟踪本地产物）,令 check_schemas 全库红（svt1/p100_vs_direct=-1.0 越 min 0 等 10 项）。已归档至本目录 `w6_firstrun_gate_fail_20260830T070732Z.json`（前役遗留红实证留存;W6_GATES.json 亦留有完整 tail）,evidence/ 移出后 check_schemas 绿。改造后互斥在四腿前被拦截,该红路径不再触发。

## 6. 主 agent 复跑命令（GPU 机,验证真跑探测段）

```powershell
py -3 ci/g31_svt_smoke.py --gate g31.waveC.svt
py -3 ci/check_schemas.py
```

预期：构建 + host 单测（cargo test streaming::svt / world::terrain 照跑）→ probe 短跑命中互斥字面 → print `互斥字面命中（probe rc=1）→ 走 mutex_registered 登记态` 与 `GATE MUTEX_REGISTERED g31.waveC.svt（非 PASS 非 FAIL,…）` → **rc=0**,产 `evidence/g31_svt_mutex_registered_<utc>.json`;随后 check_schemas 绿（新路由消费）。若 host 单测红 ⇒ rc=1（登记态拒入,如实红）。

## 7. 纪律面

- 既有 facts/FACT_IDS/v1 双 schema/全量判读路径 0 改动;`milestones/` 既有文件 0-byte（仅新增 1 文件;diff 中他务 WIP 与本任务无涉）。
- 修改均注「G37 W6 svt mutex_registered」;check_schemas 仅三处纯追加(其余 diff 为 G37 前波在飞注册,他务)。

## 8. 文件清单

| 文件 | 动作 |
| --- | --- |
| `ci/g31_svt_smoke.py` | 修改（登记态判读器 + probe 拦截 + selftest 13 臂 + docstring） |
| `milestones/g31/g31_svt_mutex_registered_schema.json` | 新增 |
| `ci/_patch_g31_svt_mutex_schemas.py` | 新增（注册脚本,幂等） |
| `ci/check_schemas.py` | 三处纯追加（经 patch 脚本） |
| 本目录 `dryrun_mutex_path.py` / `sample_mutex_registered_evidence.json` / `w6_firstrun_gate_fail_20260830T070732Z.json` | 干跑复核件 / 登记件样例 / W6 首跑红裁决件归档 |
