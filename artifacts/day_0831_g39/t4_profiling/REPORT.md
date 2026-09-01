# T4 — `identity_sum_matches_frame` 多轮中位鲁棒化处置登记（2026-08-31,G39）

处置对象：CI 门 `g31.waveC.profiling`（`ci/g31_profiling_smoke.py`）判据 ③
`identity_sum_matches_frame` —— G37 W6 以来唯一诚实红（三轮红形态见
`artifacts/day_0830_delivery/CAMPAIGN_LOG.md` L111:轮1 g14 residual −0.117433 /
轮2 g14 −0.288 / 轮3 g31 +2.250,两腿各有全绿轮 = 单轮抖动非系统性偏移）。
处置出路按 R4 交接单定案：**多轮中位**——判据规则与容差字面不动,输入从单轮值
换成 N 轮中位数。

## 1. 判据消费面变更说明

- **不变**：`identity_ok` 规则本体逐字节原样——
  `gs <= rw + IDENTITY_GPU_TOL_MS and -IDENTITY_GPU_TOL_MS <= hr <= IDENTITY_HOST_TOL_MS`
  （即 `gs ≤ rw+0.10 && −0.10 ≤ hr ≤ 2.00`）。
- **变**：输入。锁内腿编排 `g31 off ×1 → g31 on ×N → g14 off ×1 → g14 on ×N`
  （全在同一 `gpu_device_lock` 内;`N = IDENTITY_ROUNDS = 5`,`--rounds` 可覆盖,
  闭集 [1,9] 校验,越界拒跑）。各轮 profile JSON 独立路径后缀
  `.tmp/g31_gates/profiling/{g31,g14}_profile_<ts>_r<i>.json`。
- 判据 ③ 逐 bin 取 N 轮 `gpu_sum_mean_ms`/`render_wall_mean_ms`/
  `host_residual_mean_ms` 分量中位数（`statistics.median`,N 奇数取中值;新纯函数
  `median_identity`,任一轮判据分量缺失/非有限 → `{}` fail-closed 必红），再套用
  **不变的** `identity_ok`。`cpu_seg_sum_mean_ms` 非判据分量,可算则一并出中位
  仅供 evidence 登记,缺失不翻红。
- **其余 6 facts 口径不变**,消费首轮（r1）工件：①schema 合规 ②分解 measured
  ④debug labels（皆 r1 profile）⑥捕获兼容 ⑦工具探测（与轮数无关）。
- **fact ⑤ zero-drift 只加严不放松**：由「off × on(单轮)」双臂对拍改为
  「off 锚 × on 全 N 轮逐轮全等」（`drift_ok` 对每一 on 轮判 digest 位级一致,
  蕴含 on 腿各轮 digest 位级恒值断言）；逐轮 on digest 另在 evidence `notes`
  如实登记。
- evidence：`profiles/{g31,g14}` 现有块的 `gpu_sum_mean_ms`/`render_wall_mean_ms`/
  `cpu_seg_sum_mean_ms`/`host_residual_mean_ms` 填**中位值**,`identity_ok` 填中位
  判定——schema 钉死的 `identity_ok: const true` 语义自然作用于中位裁决;
  `path`/`frames_measured`/labels/`assembly_ms`/`scene_gpu_mean_ms` 仍为 r1 口径。
  另**追加**可选块 `identity_rounds`（逐轮明细,见 §3）。
- FAIL 诊断件仍落 `.tmp/g31_gates/profiling/gate_fail_<ts>.json`（含 identity_rounds
  逐轮明细）;PASS evidence 仍落 `evidence/g31_profiling_<ts>.json`。三态
  （DEV_ENV_DEGRADE/REQUIRE_REAL）逻辑未动。

## 2. 容差四面同源 0-byte 声明

容差 [−0.10, 2.00] 四面全部零触碰,git 面证明（工作树 diff）：

| 面 | 文件 | 状态 |
| --- | --- | --- |
| ① 脚本常量 | `ci/g31_profiling_smoke.py` L92-94（`IDENTITY_GPU_TOL_MS = 0.10` / `IDENTITY_HOST_TOL_MS = 2.00`） | 文件有改但 `git diff -U0 \| rg "^[+-].*TOL_MS = "` 零命中——常量行字节原样 |
| ② profile 输出 schema | `milestones/g31/g31_profile_output_schema.json`（identity 容差 const 0.1/2.0） | 未修改（git 状态干净） |
| ③ 双 bin identity JSON 字面 | `src/rurix-render/src/bin/g31_window_present.rs` L2049 / lane_body L15890 | 未修改（src 下仅有 g35 粒子面三文件为兄弟任务在途改动,与本任务无关） |
| ④ docs | `docs/renderer/profiling_debugging.md` | 未修改 |

另:禁改面合规——`ci/check_schemas.py` 0-byte（profiling 路由 L1203/L2867/L5922
早已在树,无需动）、双 bin 0-byte、`milestones/g31/g31_budget.json` 0-byte、
未跑 `--gate` 真跑（GPU 归主 agent）、未 git commit。

## 3. evidence schema 追加字段清单

`milestones/g31/g31_profiling_evidence_schema.json`（git numstat **+72 −0 纯追加**;
`required` 15 字段闭集不变 ⇒ 存量 PASS evidence
`evidence/g31_profiling_20260826T{143523,212626}Z.json` 免疫,check_schemas 复核绿）：

- `identity_rounds`（**可选** object;required 三键）：
  - `rounds`: integer,[1,9]；
  - `g31` / `g14`: array（minItems 1 / maxItems 9）,逐轮行 required 四键
    `{gpu_sum_mean_ms: number, render_wall_mean_ms: number, host_residual_mean_ms: number, identity_ok: boolean}`
    ——逐轮 `identity_ok` 为 **boolean 可红**（如实登记单轮越界）,中位裁决落
    `profiles/*/identity_ok: const true` 不变。

落地载体：新建幂等补丁 `ci/_patch_g31_profiling_rounds_schemas.py`
（锚 = 在树字节文本 `  "zero_drift": {…` 唯一性机核,token `"identity_rounds": {`
驻留 0/1 判定,io.open newline="" 字节面保全,插入后 json.loads + 结构自检
〔含 profiles identity_ok const true 未动证明〕+ 重读验证;已实跑 2 遍:首遍插入
PASS,复跑「已驻留（幂等跳过插入,只核验）」PASS）。

> **命名偏差登记（归主 agent 知悉）**：交办单指名「新建
> `ci/_patch_g31_profiling_schemas.py`」,但该名已被 C7 期 check_schemas 三处注册
> 补丁占用（历史件,不可覆写销毁）;本件按 `_patch_g31_sdk_dist_v2_schemas.py`
> 同主题二号补丁先例另名 `_patch_g31_profiling_rounds_schemas.py`。范式逐条
> 兑现（幂等/锚机核/重读/自检）,无实质偏离。

## 4. selftest / check_schemas 结果

- `py -3 ci/g31_profiling_smoke.py --selftest` → **PASS**（72 断言全 ok;新增
  中位鲁棒化臂:5 轮中 2 轮越界〔取真红实测值 −0.288/+2.25〕但中位在带 ⇒ 绿、
  residual 中位 2.1 / gpu_sum 中位 3.15 / residual 中位 −0.2 三向越界 ⇒ 红、
  N=1 退化=单轮语义、空轮列/分量缺失/NaN fail-closed、偶数 N=4 线性插值、
  rounds 闭集 [1,9] 红绿臂、IDENTITY_ROUNDS=5 缺省核、identity_rounds schema
  互核 7 断言〔可选性/rounds 闭集/逐轮行四键/identity_ok boolean〕;既有
  required-15 互核与容差 const 互核原样保持绿）。
- `py -3 ci/check_schemas.py` → **PASS**（追加后 schema 可 load + 存量 evidence
  过 Draft7;profiling 路由未动）。
- lints：0。

## 5. 若中位仍红——重标定提案预案（预登记,不启用）

若主 agent 真跑后中位仍越带（意味着偏移是系统性而非单轮抖动）,**如实维持红**,
禁改判据凑绿;处置归 **budget 程序窗**走重标定,预案要点：

1. **取证**：以 FAIL 件 `identity_rounds` 逐轮明细 + 各轮 profile JSON 判形态——
   单腿系统性偏移（如 g14 residual 恒负 ⇒ production_wall 与 cpu 段计时口径
   漂移）还是双腿散布变宽（⇒ 机况/驱动变化）。
2. **重标定路径**：新采样窗（≥20 轮分布,登记 p01/p99）→ 依分布重定容差字面 →
   **四面同源同步改**（脚本常量 L92-94 + `g31_profile_output_schema.json`
   identity const + 双 bin `"rule"` 字面 + docs）,经专用 `_patch` 幂等脚本 +
   selftest 容差互核同步 + 本 REPORT 续档;属判据字面变更,须主 agent 裁决立项,
   非本 T4 权限。
3. **禁走**：直接放宽容差凑绿、绕过四面同步单点改、把 identity fact 移出闭集。
4. 备用程序化旋钮（不动字面）：`--rounds 7/9` 加大采样（仍在闭集内,纯输入面）。

## 6. 改动文件清单

| 文件 | 变更 |
| --- | --- |
| `ci/g31_profiling_smoke.py` | +182 −42:IDENTITY_ROUNDS=5 / ROUNDS_MIN,MAX=[1,9] 常量、`rounds_valid`/`median_identity` 纯函数、锁内 off×1+on×N 编排（_r<i> 独立 profile 路径/逐轮日志/逐腿 rc 核）、判据 ③ 中位消费、fact ⑤ off 锚×N 轮全等加固、evidence identity_rounds 块 + profiles 中位填充 + notes 逐轮 digest 登记、`--rounds` CLI（闭集校验）、selftest 新臂 |
| `milestones/g31/g31_profiling_evidence_schema.json` | +72 −0 纯追加可选块 `identity_rounds`（经 _patch 落地） |
| `ci/_patch_g31_profiling_rounds_schemas.py` | 新建幂等补丁（已实跑 2 遍证幂等） |

主 agent 真跑命令：`py -3 ci/g31_profiling_smoke.py --gate g31.waveC.profiling`
（缺省 N=5;锁内腿数 4 → 12〔off×2 + on×10〕,另有 build/双 dev-env 探针不变,
预期门总时长 ≈ 旧口径 ~2.5–3×,量级 15–30 分钟,单锁一次持有）。

## 7. B4 真跑结果登记(主 agent 回填,2026-09-01)

- `py -3 ci/g31_profiling_smoke.py --gate g31.waveC.profiling` → **GATE PASS**
  (wall 863.9s ≈ 14.4min,单锁一次持有;evidence =
  `evidence/g31_profiling_20260831T175627Z.json`,identity_rounds 逐轮明细在档)。
- **中位鲁棒化当场兑现**:g14 腿第 4 轮 `host_residual_mean = −0.249892`
  单轮越界(与本报告头部所引 G37 W6 历史红轮 2 形态 −0.288 同向同量级),
  五轮中位 `+0.091333` 在带 ⇒ 判据 ③ 中位裁决绿;g31 腿五轮全绿(residual
  0.835~0.987,中位 0.883587)。旧单轮口径下本跑 g14 腿若恰采到 r4 即诚实红
  ——鲁棒化把「单轮抖动」与「系统性偏移」在判据面分离的设计目标实证成立。
- fact ⑤ 加固面实跑绿:off 锚 × on 全 5 轮 digest 位级全等(g31
  presented+render 双锚,g14 last_frame)。
- §5 重标定预案**未启用**(中位在带;两腿各有全绿轮 + 越界轮孤立 = 非系统性
  偏移之证);容差 [−0.10,2.00] 四面同源 0-byte 维持。
