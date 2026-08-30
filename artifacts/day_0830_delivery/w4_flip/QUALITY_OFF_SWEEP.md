# G37 W4 默认翻转前置补扫对账表（QUALITY_OFF_SWEEP）

> **任务**：DEFAULT_FLIP_PLAN §2.5/§4 字面「CI 中消费互斥诊断臂的门全部要加 off 字面,清单外补扫」——
> `g31_window_present --quality` 默认 off→full 翻转前,全仓调用面逐点分类处置。
> 扫描范围：`ci/*.py`、`artifacts/day_0830_delivery/`、`milestones/`（只读）、`registry/`（只读）、全仓兜底（`tools/` 不存在）。
> 纪律：禁 GPU / 禁 cargo；只动 `ci/` 与 `artifacts/day_0830_delivery/`；锚字面/判据/其它参数零改动。
> 日期 2026-08-30。

## 判类依据（harness 源码字面复核,g31_window_present.rs）

- full 展开臂（bloom/AE/smooth-normals/textures 等）与 `--hzb on / --svt on / --slab-table / --cluster-lod / --wp-hlod` 硬 fail 互斥；`--fg` 仅豁免 `--quality full` 字面（两点式闭集 = all-off base ∪ full 预设）；显式画质旗标与 full 展开面重叠 = dup fail-closed（`--textures on` 等 22 旗标）。
- `--window-storm/--storm-soak/--fault-probe` 与 full 不互斥（E1 解除,e4_storm_summary 在案）,但 C4 门语义 = 生产五 pass 现状车道 + 「C4 前行为逐字节等价」off 基线——按任务书归 A。
- `--quality off` = 中性字面零展开零行为 ⇒ A 类补丁在翻转前后均与现行为逐字节等价（翻转免疫）。

## A 类：补 `--quality off`（18 调用点 / 11 文件,全部已改）

| 文件 | 调用点(现行号) | 臂语义 | 动作 |
|---|---|---|---|
| ci/g31_svt_smoke.py | L343（run_present 共用 argv;tex-only 基线腿 + svt 池腿全过此） | `--textures on` 显式(翻转后 dup fail) + `--svt on` 互斥 | 已补 off |
| ci/g31_hzb_wiring_smoke.py | L305（run_present 共用 argv;orbit on/off、静态、all-visible 腿全过此） | `--hzb on` 互斥;off 腿 = on≠off/像素中性判据基线 | 已补 off |
| ci/g31_slab_wiring_smoke.py | L344（run_present 共用 argv;device/host 臂 + 基线腿全过此） | `--slab-table` 互斥;基线腿 = 跨臂对拍基 | 已补 off |
| ci/g31_framegen_present_smoke.py | L602（run_harness 共用 argv;x2/x3 × orbit/dolly 腿全过此） | fg **base 点**门(两点式闭集之 all-off 基;不带则翻转后静默升级成 full 组合点,门语义漂移、g26 冻结容差口径脱钩) | 已补 off |
| ci/g31_texture_sampling_smoke.py | L452（run_present 共用 argv;off/on 双腿全过此） | on 腿 `--textures on` 显式(dup fail);off 腿 = on≠off 判据的 off 基线 | 已补 off |
| ci/g31_cluster_lod_smoke.py | L350（窗口统计臂） | `--cluster-lod on` 互斥 | 已补 off |
| ci/g31_wp_hlod_smoke.py | L405（三档窗口臂） | `--wp-hlod on` 互斥 | 已补 off |
| ci/g31_wp_hlod_smoke.py | L504（四 RED 臂） | `--wp-hlod on` 互斥;翻转后会经 full 互斥 fail 先退,WP_RED_ARM_DETECTED 检出路径失覆盖 | 已补 off |
| ci/g31_profiling_smoke.py | L326（g31_leg,profile on/off 双腿过此） | G31_PASSES 五段闭集判据钉死 off 形态(full 增 pass 即缺/多 pass 判红) | 已补 off |
| ci/g31_profiling_smoke.py | L406（dev-env 探针腿） | 与门主腿同 off 形态(统一口径) | 已补 off |
| ci/g31_profiling_smoke.py | L566（RenderDoc 捕获腿） | 同上 | 已补 off |
| ci/g31_robustness_smoke.py | L187（fault-probe 三探针臂） | fault 诊断臂(任务书 A 类字面;C4 登记面 = 生产五 pass 现状车道) | 已补 off |
| ci/g31_robustness_smoke.py | L255（基线腿） | 语义 = 「注入臂全默认关:与 C4 前行为逐字节等价」——off 语义基线 | 已补 off |
| ci/g31_robustness_smoke.py | L291（窗口风暴爆发臂） | storm 诊断臂(任务书 A 类字面) | 已补 off |
| ci/g31_robustness_smoke.py | L326（storm-soak 故障臂） | 同上(且 1010 帧墙钟阈按 off 帧时标定) | 已补 off |
| artifacts/day_0830_delivery/w0_baseline/w0_reverify.py | L112（alloff 步） | all-off 8f == 55e4a92d 锚复验——本役复验脚本,alloff 步语义 = 显式 off | 已补 off |
| artifacts/day_0830_delivery/w3_deep/fg_combo/accept_fg_combo.py | L88（alloff 臂） | 画质生效门(③ full≠alloff)对照基线——不带则翻转后 full==full 门必红 | 已补 off |
| artifacts/day_0830_delivery/w3_deep/fg_combo/accept_fg_combo.py | L116（互斥矩阵「散臂微调拒跑」臂） | 期望 exit=1 的两点式闭集卫兵路径——不带则翻转后经 full×`--textures` dup 冲突「碰巧」exit=1,卫兵路径失覆盖 | 已补 off |

## B 类：保持不带,语义随翻转升级（5 调用点 / 4 文件,零改动,登记）

| 文件 | 调用点(行号) | 登记理由（语义随翻转升级） |
|---|---|---|
| ci/g31_blocked_probes_smoke.py | L525-526（RD-045 P02 device 腿,orbit 64+10） | 任务书钦定 B：默认臂语义,翻转后按 full 重收割;锚字面 L63（060e69a8,旧二进制绑定）由主线 W4 §2.3 收编后统一改写——本补扫零触碰锚与调用 |
| ci/g31_game_loop_smoke.py | L453（run_harness 共用 argv;A/B/C/D 四腿） | A3 默认臂门,无锚字面;四判据全为腿间比较（A==B 双跑确定性、C≠A 异轨迹、D≠A 异曝光）,翻转后四腿同升 full 仍自洽;`--ev100-ramp` 驱动 TSR 段曝光（evidence ev100_seq 直写坡值）,AE 增益在 encode 段正交不夺 D≠A;翻转后门语义升级 = 生产默认(full)形态的游戏循环最小面 |
| ci/g31_window_present_smoke.py | L361（单腿 3+1） | A1 默认臂门,无 digest 锚;判据 = 字段闭集/口径恒等式（present_overhead≥present_frame 等）,形态无关;翻转后语义升级 = 生产默认形态的真窗口 present 最小门 |
| ci/g31_wave_a_soak.py | L157（主腿 ≥10000 帧） | A6 soak 门,无锚;判据 = 计数恒等/leak/validation,形态无关;DEFAULT_FLIP_PLAN §2.5 复跑清单字面即「soak ≥1800s **@新默认**」——soak 语义就该随默认升级 |
| ci/g31_wave_a_soak.py | L193（确定性抽查短腿双跑） | 同上;digest_seq 双跑腿间自洽,full 双跑位级已由 F2/W0 锚（5db2e7d7 双跑）先证 |

## C 类：已显式带 `--quality`,零动作（12 调用点 / 3 文件）

- ci/g31_encode_parity_smoke.py L467：显式 `--quality off`（原注即「默认翻转免疫」,W1 已预防）——1 点。
- artifacts/day_0830_delivery/w3_deep/fg_combo/accept_fg_combo.py：full_a/full_b/combo_a/combo_b/combo_x3 五臂（L89-93）+ 互斥矩阵 fg×hzb/fg×slab/fg×lut/fg 无轨迹/fg×headless 五臂（L103-112）均显式 `--quality full`——10 点。
- artifacts/day_0830_delivery/w0_baseline/w0_reverify.py L114：full16 步显式 `--quality full`——1 点。

## 非调用点 / 只读登记（零动作）

- **ci/check_schemas.py**：仅 schema 加载/路由字面（evidence 文件名前缀),非 bin 调用。
- **ci/g36_geo_composition_smoke.py**：WINDOWED_ITEMS 留窗登记文本 + 互斥维持字面,非调用。
- **ci/g31_ktx2_smoke.py**：B4_IN_FLIGHT_PATHS 冻结源路径列表（g31_window_present.rs）,非调用。
- **ci/g34_hzb_unified_smoke.py / g34_skin_unified_smoke.py / g34_unified_lane_smoke.py**：untracked 冻结面 sha256 快照路径列表（.rs 源文件）,非调用（其 bin = g34_full_lane）。
- **ci/g31_wave_a_anchor_check.py**：调用面 = `g14_3_pipeline_perf` bench 18 格（L87）,窗口 bin 零调用——bench `--quality` 默认永不翻转（DEFAULT_FLIP_PLAN §0）,Stage A c1d28ad7 系锚零影响,登记即可。
- **milestones/**（CI_GATES.md / G31_CONTRACT.md / G31_PLAN.md / g31_blocked_probes_2026.json / 各 schema）：文档与探针件命令字面,只读——复验命令字面若翻转后重跑,诊断臂须按本表 A 类形态显式 off（主线 W4 复跑清单口径）。
- **registry/deferred.json**：RD-045 观察窗 / A3 重判窗登记字面,只读。
- **artifacts/day_0829_realism/final/f2_reanchor.py** L111：历史交付件（冻结面,实测 7388B 非字面 0-byte——冻结承诺面语义）,alloff 步空参数依赖默认 off;登记：翻转后如复跑该历史脚本,alloff 步须显式 off 方可对 55e4a92d,**文件本体不动**。
- **artifacts/day_0830_delivery/ 各 REPORT.md / MERGE_REPORT.md / PLAN.md**：GPU 验收命令文本 = 文档,不改（任务书字面）。
- **artifacts/day_0830_delivery/w1_fixes/slot14_normal/{check_png_limit15,verify_v1_v2}.py**：不调用窗口 bin,零涉。

## 验证结果（本补扫零 GPU 零 cargo）

| 文件 | 验证 | 结果 |
|---|---|---|
| ci/g31_svt_smoke.py | --selftest | PASS（facts=5;红臂组+正例组+双 schema 互核） |
| ci/g31_hzb_wiring_smoke.py | --selftest | PASS（facts=9;5 红臂组+正例组+schema 互核） |
| ci/g31_slab_wiring_smoke.py | --selftest | PASS（facts=8;5 红臂组+正例组+双 schema 互核） |
| ci/g31_framegen_present_smoke.py | --selftest | PASS（2 GREEN + 10 RED + 比较器 4 象限 + schema 互核） |
| ci/g31_texture_sampling_smoke.py | --selftest | PASS（facts=8;含作废锚占位钉死） |
| ci/g31_cluster_lod_smoke.py | --selftest | PASS（3 正则 GREEN + schema 互核） |
| ci/g31_wp_hlod_smoke.py | --selftest | PASS（4 正则 GREEN + schema 互核） |
| ci/g31_profiling_smoke.py | --selftest | PASS（facts=7;6 红臂组+正例组+双 schema 互核） |
| ci/g31_robustness_smoke.py | --selftest | PASS（3 GREEN + 5 RED + schema 互核） |
| accept_fg_combo.py | --selftest + --plan 抽查 | PASS（1 GREEN + 10 RED 全检出）;plan 打印确认 alloff 臂/散臂互斥臂命令面已带 off |
| w0_reverify.py | py -3 -m py_compile | 绿（无 selftest 面） |
| 全部 11 改动文件 | py -3 -m py_compile 批跑 | rc=0 全绿 |

## 汇总

- **A 类 18 调用点已补 `--quality off`**（ci/ 九文件 15 点 + 本役两脚本 3 点）,插入均为参数列表两元素,锚字面/判据/其它参数零改动;off = 中性零展开 ⇒ 翻转前行为逐字节等价（selftest 全绿佐证）,翻转后免疫。
- **B 类 5 调用点登记不动**（RD-045 P02 / A1 / A3 / A6 soak ×2）——默认臂语义随翻转升级为 full,锚与复跑归主线 W4（§2.3-§2.5）。
- **C 类 12 调用点显式带档,零动作**。
- 翻转执行时本表 A 类无需再动;B 类 4 门在主线「全门复跑清单」内按新默认复跑。
