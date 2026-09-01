# G39 战役交接单(day_0831_g39,2026-09-01 收官)

> 役期:2026-08-31 21:55 开役 → 09-01 03:16 GPU 面收官 + 文档收尾。入役 = 收官 git HEAD `82a59ae3`(全程未 commit,**入库归 owner**)。
> 形态:T1 ReSTIR 高档时域 reservoir 提灯臂生产接线(主菜)+ G38 四留窗收尾(T2 skin 批次 B / T3 slot_as 单源折叠 / T4 profiling 门鲁棒化 / T5 device cut 设计案+P1);两层子 agent 编排,GPU 真跑全程 gpu_device_lock 锁内串行(`RURIX_REQUIRE_REAL=1`+`RURIX_VK_VALIDATION=1`,构建 target-night)。
> **零重锚预期兑现**:默认面锚全程恒值,无一锚回写。逐波明细见 `CAMPAIGN_LOG.md`;各任务施工细节见 t1_restir/t2_skin/t3_fold/t4_profiling/t5_devicecut 各 `REPORT.md`。

## A. 锚表(G38 收官谱系,本役全程恒值)

| 锚 | 值 | 本役复证状态 |
|---|---|---|
| all-off 8f/warmup2 | `sha256:55e4a92d25be959a91d111140b88eb81e0c7c29fe80d1aeee641aa78d8a86288` | W0 + B3(off 面)+ soak 前置,三重 MATCH |
| full19 缺省 96f/warmup2 | `sha256:a5521e4708a814e364fd3bf95b18f0ab69b6646efd039246e8686533c30e4fb1` | W0 + B3(k12_off 阶梯基线自证)+ soak 前置,三重 MATCH |
| bench 160f(Stage A bistro/t100/tsr 格) | `…c1d28ad73783cc3c054ae0ce372b042a23d01df5491394cb38de7725e83b6c02` | W0 + B1 负控 MATCH;Stage A 探针 W0/soak it4 MATCH |
| soak 32f/warmup2 缺省面 | `sha256:a8204b3b93845f557656231e5b3e2407bcbda030857d8e0d87ace0b48b32ac09` | G38 在案值钉死非自举,6/6 迭代位级恒值 |
| RD-045 orbit 64+10 | `sha256:066395b0b6d877f546b7082560c093b4c325f39dae446abf3d59a8ad1023d56d` | 本役面不涉未消费,谱系维持 |
| ris_nee / transparency 单开 96f | `851a61ba…` / `af1f7264…` | 不动抽验参照,本役未消费未漂(冻结面 0-byte 证明在 B3/收役门) |

## B. 门台账(全绿;GPU 批全部主 agent 锁内串行)

| 批 | 时间(本地) | 内容 | verdict | evidence |
|---|---|---|---|---|
| W0 基线 | 08-31 21:55 | 构建双 bin + 三锚复证 + CPU 守卫 7/7 | BASELINE PASS | `w0_baseline/G39_BASELINE.json` |
| B1(T2) | 22:36-22:55 | skin 1\|2\|3 flip-trace x1≡x2≡x3 逐字节 + 双跑位级 + skin_verify all_pass + 真动门 + dyn/静态基线收割 + fif probe 双臂 + calibrate --check | PASS(refit 实测纯函数,免 L2a 降档) | `t2_skin/gpu/B1_SUMMARY.json` |
| B2(T3) | 23:24-23:40 | 折叠后 静态/dyn rebuild/refit/skin ×1\|2\|3 轨迹逐字节 == 折叠前 + fif probe 双臂 == B1 | PASS(零语义门闭合) | `t3_fold/B2_SUMMARY.json` |
| B5(T5 P1) | 23:40-23:45 | C1 判定码全等×16f×双跑 + C2 dev==host digest + 跨 G38 锚 + C3 red-arm 必红 + C4 ×ml1 + C5 缺省 0-byte | PASS(#77 P1 等价门收口) | `t5_devicecut/B5_SUMMARY.json` |
| B3(T1) | 23:46-01:52 | off==双锚 + on r1/r2 双跑位级 + dolly 240f digest_seq + 阶梯 12/26/38 簇 × on/off + 判读矩阵 | PASS(首红 = prev_vp device_local 接线 bug,修复重跑全绿) | `t1_restir/B3_SUMMARY.json` + `ab/T1_AB_MATRIX.json` |
| B4(T4) | 09-01 01:54-02:08 | profiling 门真跑(N=5 中位判据首真跑,863.9s) | **GATE PASS**(g14 r4 单轮越界被中位吸收,机制兑现) | `evidence/g31_profiling_20260831T175627Z.json` |
| 收役回归 | 01:54-02:43 | wp_hlod / g36_geo / restir_wiring 三门 + framecut probe(incr==full 16f 位级+双跑+ml1+跨 G38 MATCH)+ CPU 守卫 7/7(budget_eval 330 pass 0 skip) | W_GATES PASS(fails=[]) | `closeout/W_GATES.json` |
| soak | 02:44-03:16 | 1936.2s ≥ 1800s / 6 迭代 32f 锚 6/6 恒值 + 前置双锚 + it4 Stage A 探针 + frame_ms_max 10.798 ≤ 11.111 | PASS(VUID=0,fails=0) | `soak/G39_SOAK.json` |

帧时验收终判(B3 measured,render_wall p50):k12 off 9.560 / on 7.390ms;**k26 off 11.546(超线)/ on 7.526ms = 26 簇档进 11.11ms 预算,余量 +3.58ms,战役目标达成**;k38 off 13.755 / on 7.417ms(on 臂与簇数解耦 7.4-7.5ms 恒平)。方差如实登记:on 臂 dark ROI p95 时域噪声升 ~1.5-2.6×(1-spp 随机换选代价,TSR 部分吸收,不判红)。

## C. 交付面台账(git 工作树,未 commit;owner 入库时按本表择取)

G39 自有面(修改 11 + 新件斜杠标注):

| 文件 | 任务 | 变更 |
|---|---|---|
| `src/rurix-render/kernels/g31_realism.rx` | T1 | +~300:第 9 链位 gate 化时域段(WRS/重投影/merge/验证射线/写回) |
| `src/rurix-render/src/bin/g31_window_present.rs` | T1 | +~400:九步接线 + B3 修复(prev_vp `device_local` 两处 true→false) |
| `ci/_patch_g31_window_evidence_schemas.py` | T1 | **新件**:quality_arms 两键 properties-only 幂等补丁 |
| `milestones/g31/g31_texture_sampling_heap_evidence_schema.json` | T1 | ±4(经上补丁,不进 required,旧 evidence 免疫) |
| `.tmp/night_0831/spv/g31_realism_restir.spv` | T1 | **新 SPV 工件**(365,724B,sha `5571e875…`,.tmp 惯例不入 git 面;既有 9 工件 sha 全等) |
| `src/rurix-render/src/bin/g14_3_lane/g14_3_lane_body.rs` | T2 | +704/−120:skin 批次 B(§1-A6) |
| `src/rurix-render/src/bin/g14_3_pipeline_perf.rs` | T2 | +27/−13:CLI 解除(§2-B2/B3) |
| `src/rurix-rt/src/render_exec.rs` | T3 | +125/−14:`submit_pipelined_frame` 加 `Option<&SlotAsGroup>` 末参 |
| `src/rurix-rt/src/render_exec_g37_fif_dyn.rs` | T3 | +11/−389:复制体连 doc 整删 |
| `ci/g31_profiling_smoke.py` | T4 | +182/−42:N=5 多轮中位判据 + fact ⑤ 加固 |
| `milestones/g31/g31_profiling_evidence_schema.json` | T4 | +72/−0:可选块 identity_rounds 纯追加 |
| `ci/_patch_g31_profiling_rounds_schemas.py` | T4 | **新件**:幂等补丁(命名偏差登记在 T4 REPORT §3) |
| `src/rurix-render/src/bin/g14_3_lane/g31_frame_cut_arm.rs` | T5 | +594/−12:device 对拍臂(含 C1 域检修正) |
| `src/rurix-render/src/bin/g31_frame_cut_probe.rs` | T5 | +52/−2:`--cut-source device`/`--cull-spv` 闭集旗标 |
| `.gitignore` | 主 agent | +5:t1_restir/ab p.raw 排除(~1.2GB,G38 同律) |
| `artifacts/day_0831_g39/` | 全役 | 战役目录(日志/交接/五 REPORT/evidence/summary) |
| `evidence/g31_{profiling,wp_hlod,restir_wiring}_20260831T*.json` + `evidence/g36_geo_composition_gate_20260831T*.json` | B4+收役 | 四份门 PASS evidence |

冻结面 0-byte 兑现(git 双面证明在各 REPORT/门日志):`g28_restir.rx` / `gi/restir_reservoir.rs` / `gi/multi_light.rs` / rurix-asset `g31_cluster_cull.rx` 源 / `ci/check_schemas.py` 本体 / identity 容差四面 / 公共入口 `submit_with_frame_update_slot_as` 签名。

**兄弟任务在途面(非 G39,不混,按文件名显式择取先例留树)**:`src/rurix-render/kernels/g35_render_resolve.rx`(±31)/ `g35_render_splat.rx`(±21)/ `src/rurix-render/src/bin/g35_particle_lane.rs`(±253)/ `artifacts/day_0831_site/`。

## D. 留窗登记(如实,不冒充)

1. **lamp-k 提档提案判 GO,只登记留 owner**(`t1_restir/LAMP_K_PROPOSAL.md`):建议 A 维持(默认 12/0.6 零动作,26 簇进预算组合已钉死可交付)+ B 留窗(提档默认 = full19→full20 语义变更须整批重锚,与下一触锚变更合批);C 不推荐。
2. T1 **host 镜像对拍臂未建**(EVAL §6.2,约半臂当量):本窗验收走 digest 双跑位级 + A/B 方差,不含 per-pixel host 复算对拍。
3. T1 **两新税**:restir on ⇒ 点灯软阴影半影让位(圆盘 N 样本不进验证射线)+ 玻璃后点灯影转硬影(透明衰减重走段不进本臂);A/B 判读以 dark ROI 噪声口径为准,亮度/半影差登记不判红。
4. T1 跨像素 merge 的 phat 重算近似(标准 ReSTIR DI 时域形,m_cap 截断置信有界,非严格无偏)+ per-pixel phat 钳制未做(第一旋钮 = 降 `--lamp-restir-mcap`);dolly disocclusion 深度/法线拒留窗(v1 仅界内/pcw 门);风暴臂(window-storm)× restir 组合未验收。
5. **T5 P2/P3 留窗**(生产 dispatch 下沉 + `verify_cut_coverage` device 化):开窗条件 P1 C1-C5 已全绿;`device_cut_probe_ms` mean=82.7ms 为 P2 上界参考(probe 逐帧回读口径,不判读);诚实边界 = UPDATE build ~21ms 地板主导,收益叙事须与 `--min-level` 组合。
6. T4 若未来中位仍红 ⇒ **重标定预案**(`t4_profiling/REPORT.md` §5):≥20 轮分布重定容差 + 四面同源同步改,归 budget 程序窗,禁改判据凑绿;程序化旋钮 `--rounds 7|9` 可先行加采样。
7. T2 生产 bistro 规模 AS 副本内存 = receipt/evidence notes 登记面(预算门条目 `g31.fif_dyn.slot_as_group_mem_bytes` 锚 probe 场景,不混口径);skin bridge_ext×FIF 须 rt 平行入口,维持留窗。
8. soak it9 Stage A 探针未及(6 迭代即越 1800s 墙钟线,it4 MATCH 在档)——脚本口径使然,非缺陷。
9. T1 evidence quality_arms 登记超臂⑧极简体例(偏离登记 T1 REPORT §七-1,revert 点在案)——维持现状或回归体例归 owner。

## E. 后续窗口建议(优先级序)

1. **owner 治理窗**:本役工作树入库(commit 按 C 表择取,兄弟面不混)+ lamp-k 提案判档(A/B/C)。
2. **T1 画质补窗**:host 镜像对拍臂(y 整数锚/p100 对拍重建)+ disocclusion 深度/法线拒 + per-pixel phat 钳制——三件合一窗,消解 D-2/D-4。
3. **T5 P2**:`--cut-source device` 生产 dispatch(判定码免回读,消费 B5 等价门谱系),与 `--min-level` 组合进预算叙事。
4. **T2 补窗**:bistro 生产规模 slot_as AS 副本内存 evidence 正式登记 + bridge_ext×FIF rt 平行入口评估。
5. TSR 前口径(scene-linear)方差 A/B 留窗(G38 v1.2.1 ④ 遗留,restir 臂同样适用)。

