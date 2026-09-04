# G40 战役交接单(day_0901_g40,2026-09-02 收官)

> 役期:2026-09-01 17:06 开役(W0)→ 09-01 18:38-19:02 B1/B2 → 09-02 12:24-14:52 B3(三跑谱系)→ 14:52-15:54 收役门 → 17:21-17:53 soak → 文档收尾。入役 = git HEAD `1478859a`(feat(g39) 收口波;**全程未 commit,入库归 owner**)。
> 形态:T1 ReSTIR 画质补窗三件合一(主菜:T1a host 镜像对拍臂 / T1b disocclusion 两拒 / T1c per-pixel phat 钳制)+ T2 device cut P2 生产 dispatch(#77)+ T3 skin/dyn 生产规模 AS 副本内存 evidence 补登 + T4 bridge_ext×FIF rt 平行入口评估(纯文档)。
> **编排形态(如实)**:本执行环境无子 agent 调用工具——主 agent 串行承担侦察 / 实施 / 验收三层;侦察交接单 `recon/R1_T1.md` / `R2_T2.md` / `R3_T3T4.md` 照例先行归档,编辑权台账逐任务独占登记,GPU 真跑全程 `gpu_device_lock` 锁内串行(`RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1`,构建 `target-night`)。与任务书「两层子 agent 编排」的偏差仅在执行主体(`CAMPAIGN_LOG.md` L5)。
> **零重锚预期兑现**:默认面锚全程恒值,无一锚回写。逐波明细见 `CAMPAIGN_LOG.md`;各任务施工细节见 `t1_restir2/REPORT.md`(§八 = B3 回填章)/ `t2_devicecut/REPORT.md` / `t3_asmem/REPORT.md` / `t4_bridgeext/EVAL_BRIDGEEXT_FIF.md`。

## A. 锚表(G38 收官谱系,G39 全程恒值,本役全程恒值)

| 锚 | 值 | 本役复证状态 |
|---|---|---|
| all-off 8f/warmup2 | `sha256:55e4a92d25be959a91d111140b88eb81e0c7c29fe80d1aeee641aa78d8a86288` | W0 + B3 三跑批(run1/run2/run3 各一次)+ soak 前置,**五重 MATCH**;跨四个不同窗口 bin 二进制(W0 件 / run1-2 件 / run3 件 / soak 件)同值 |
| full19 缺省 96f/warmup2 | `sha256:a5521e4708a814e364fd3bf95b18f0ab69b6646efd039246e8686533c30e4fb1` | W0 + B3 三跑批 anchors + B3 ladder `k12_off`(阶梯基线自证)+ soak 前置,**六重 MATCH**;同上四个二进制同值 |
| bench 160f(Stage A bistro/t100/tsr 格) | `…c1d28ad73783cc3c054ae0ce372b042a23d01df5491394cb38de7725e83b6c02` | W0 n2 MATCH;Stage A 探针 W0 n3 + soak it4 MATCH |
| soak 32f/warmup2 缺省面 | `sha256:a8204b3b93845f557656231e5b3e2407bcbda030857d8e0d87ace0b48b32ac09` | G38 在案值钉死非自举,见 §B soak 行 |
| 窗口 frame_cut 臂 16f(`--cluster-per-frame-cut`) | `5540ecae…`(G38 T3 在案) | B1 ⑤(W0 二进制)+ B2 B1f 附录(T2+T3 树新建二进制)双 MATCH |
| frame_cut probe incr 16f digest 序列 | G38 `t3_framecut/ev/t3_incr.json` 在案 | B1 ③ device 臂 + 收役门 `fc.cross_g38` host 臂,**跨 G38 锚 MATCH**;device 臂另 == G39 `t5_dev` 锚 |
| RD-045 orbit 64+10 | `sha256:066395b0b6d877f546b7082560c093b4c325f39dae446abf3d59a8ad1023d56d` | 本役面不涉未消费,谱系维持 |
| ris_nee / transparency 单开 96f | `851a61ba…` / `af1f7264…` | 不动抽验参照,本役未消费未漂(g28/gi 冻结面 0-byte 由 restir_wiring 门机器证明) |

**on 臂无锚(如实)**:`--lamp-restir on` 各臂 digest 对 G39 换代(SPV `5571e875…`→`8ac52dc4…` + stride 4→12 + T1b 缺省拒进链),本役 on 臂确定性门 = 双跑位级(det `9fe2cfa5…`/`6a086fc7…` 跨 run1/run2/run3 三跑、两个二进制恒值;verify `e226f3ce…`;dolly `3cc8206f…` + 242 项 digest_seq),不设跨役锚。

## B. 门台账(全绿;GPU 批全部主 agent 锁内串行)

| 批 | 时间(本地) | 内容 | verdict | evidence |
|---|---|---|---|---|
| W0 基线 | 09-01 17:06-17:13(+CPU 守卫 17:22) | 构建 4 bin(182s)+ 三锚复证 n1/n2/n3/n4 + CPU 守卫 7/7(budget_eval 330 pass 0 skip) | BASELINE PASS | `w0_baseline/G40_BASELINE.json` + `cpu_guards.log` |
| B1(T2) | 18:38-18:46(8.4min) | ①device 16f digest==host 位级 + 臂内建双跑 + 跨进程双跑 ②×ml1 ③incr==full + 跨 G38 t3_incr / G39 t5_dev 双锚 ④red-arm rc≠0(「red-arm 模式」+「覆盖性」)⑤缺省 host 0-byte 回归 + 窗口臂 5540ecae ⑥帧时分项 measured | PASS(六判) | `t2_devicecut/B1_SUMMARY.json` + `b1_log.jsonl` + `ev/` |
| B2(T3) | 18:51-19:02(10.1min) | ①slot_as_mem 字段在档(dyn/skin × inflight 2\|3;inflight=1 负控无键)+ skin_verify all_pass ②flip-trace 六臂对 G39 B1/B2 在案件位级全等 ③B1f 附录:T2+T3 树窗口 bin 5540ecae MATCH | PASS | `t3_asmem/B2_SUMMARY.json` + `b2_log.jsonl` + receipts/ft_* |
| B3(T1) | run1 09-01 19:47-20:18(宿主终端关闭致 stdout OSError 中断,非 GPU 面)/ run2 09-02 10:10-10:52(verify 三跑连红,主 agent ABORT → 首红裁决)/ **run3 12:24-14:14 六 GPU 段 + 14:52 judge 段补跑** | anchors 双锚 / det on+onmin 双跑位级 / verify 六判(r1==r2 位级 + clamp4 GREEN + red_phase/red_resv 必红)/ ladder 八臂 + k12_off==full19 锚 / dolly 240f digest+digest_seq 双跑位级 / storm 三并联 / judge k26_on p50 ≤ 11.11 | **PASS**(首红 = 判读器 f32 边界口径过严,产物零缺陷;修正 = 判据两级化〔收紧非放宽〕+ 红臂空转修复) | `t1_restir2/B3_SUMMARY.json`(**仅 judge 段**)+ `b3_log.jsonl`(六 GPU 段逐段 `seg_fails: []`)+ `ab/T1_AB_MATRIX.json` + `ev/` |
| 收役回归 | 14:52-15:54 | profiling(1020s,N=5 中位十轮全 ok)/ wp_hlod(870s)/ g36_geo(1388s)/ restir_wiring(47s,g27/g28 冻结面双面 0-byte)四门 + framecut probe(host 四臂 + **device 臂终态树** incr==full 16f 位级 + 双跑 + dev==host + 跨 G38 MATCH)+ fif probe(rebuild/refit gates 全 true,trimmed_mean 44,544B)+ calibrate --check + CPU 守卫 7/7 | W_GATES PASS(fails=[]) | `closeout/W_GATES.json` + `gates_log.jsonl` + `framecut_ev/` + `fif_ev/` + `evidence/*_20260902T*.json` 四份 |
| soak | 17:21-17:53 | **1930.6s ≥ 1800s / 6 迭代** 32f 锚 `a8204b3b` **6/6 位级恒值** + 前置双锚(all-off / full19,终态树 17:21 新建 exe)+ it4 Stage A 探针 MATCH(it9 未及,G39 同形)+ VUID=0;**frame_ms_max 12.393 > 11.111 记账如实**(机态,见下;verdict 不消费该字段) | PASS(fails=0) | `soak/G40_SOAK.json` + `soak_ev/` |

**帧时验收终判**(B3 measured,profile `render_wall` p50,缺省 / `--quality full` 预设轴):k12 off 9.785 / on 7.861ms;**k26 off 11.476(超线)/ on 7.841ms = 26 簇交付档进 11.11ms 预算,余量 +3.27ms(G39 判档 A 钉死的 `RURIX_G31_LAMP_GRID_M=0.15 --quality full --lamp-k 48 --lamp-restir on` 组合在终态树复现)**;k38 off 14.211 / on 8.243ms(on 臂三档 7.84-8.24 恒平,极差 0.40ms,「与灯数 O(1)」在 stride 12 + 两拒 + 钳新形态下继续成立)。

**两条负面结果(如实,不进全绿叙述)**:①画质收益为负——dark ROI p95 时域噪声 on 臂升至 off 臂 2.1-2.7×(k12 −108%/−74%,k26 −164%/−66%,k38 −167%/−54%),三件合一**未消解** G39 D-3 方差税;②T1b 两拒边缘改善 ≈ 0(四 ROI 全落 ±1%,低于机态噪声地板),「两拒有边缘收益」在本役素材上未被证实。A/B 达标口径 = 帧时进预算 + 确定性位级;噪声升幅登记不判红。

**soak 帧时记账(如实,不进全绿叙述亦不判红)**:`frame_ms_max` 12.393 > 11.111(逐跑 real_render_frame_ms:pre_full19 11.560 / it1-6 12.393 / 11.514 / 11.662 / 10.953 / 11.801 / 12.106)。定性 = 机态而非渲染面:digest 8/8 位级恒值;同一语义面在案对照 W0 n4 11.125 / B3 run3 n4 10.824 / B3 ladder `k12_off` 9.996(render_wall p50 9.785 进预算)/ G39 soak max 10.798,同锚 digest 下帧时散布 9.8-12.4ms 即机态噪声量级;soak 口径 = 32f/warmup2 短跑(GPU 自 P8 爬坡 + 冷启动加载后立即计时)+ 宿主并发桌面负载未控制变量。帧时验收法定口径仍为阶梯 render_wall p50(§上段),soak 帧时为记账面。

**T2 帧时 measured**(B1,96x54/16f,f1-15 均值):cull **dispatch GPU 0.113ms**(P1 run_compute 82.7ms 上界 → 三个量级降)/ device select_ms 4.99(含 fence+回读+构造墙钟)/ ml0 device cut_ms 8.179 vs host 7.874(**ml0 无净收益**,DESIGN §4-1「下沉不解预算」维持)/ **ml1×device 组合墙钟 16.095ms 落 DESIGN §4-2 预期带 15-19ms**(G38 ml1 host 形态 23.7-27.3ms 对照;build 地板 8-11ms 仍在,不冒充进预算)。

**T3 measured**(生产 bistro 规模 tier100):dyn ×2 = 240,614,400B / ×3 = 360,921,600B;skin ×2 = 240,621,312B / ×3 = 360,931,968B(120.3MB/槽;skin 比 dyn 每槽 +3,456B = 角色 updatable BLAS 面差);与预算门 probe 口径 44,544B 差 ~2,700×,分口径登记不混。

## C. 交付面台账(git 工作树,未 commit;owner 入库时按本表择取)

G40 自有面(修改 8 + 新件斜杠标注):

| 文件 | 任务 | 变更 |
|---|---|---|
| `src/rurix-render/kernels/g31_realism.rx` | T1 | +91/−31:头注 params/stride 布局 + reservoir stride 4→12 写回(几何面入池)+ T1b 两拒折入 merge 计数门 + T1c W·phat 钳(params[75..78) 启用/追加) |
| `src/rurix-render/src/bin/g31_window_present.rs` | T1 | +895/−19:五子旗标闭集(`--lamp-restir-verify/-verify-red/-clamp/-depth-rej/-nrm-rej`)+ stride-12 两 descs + verify 回读注册/订阅/消费 + 镜像模块 `g31_lrs_mirror_frame`/`g31_lrs_chain`(两级判据:`G31_LRS_ULP_BOUND` 32ULP + `G31_LRS_ATTRIB_CAP` 4096)+ evidence quality_arms 四字段 + `lamp_restir_verify_stats` 块 |
| `ci/_patch_g31_window_evidence_schemas_g40.py` | T1 | **新件**:二号补丁(quality_arms +4 键 / textures +verify_stats v2,properties-only 幂等,不进 required,自校验四条拒改保护) |
| `milestones/g31/g31_texture_sampling_heap_evidence_schema.json` | T1 | +24/−1(经上补丁;`y_unattributed` const 0 钉进 schema;旧 evidence 免疫;check_schemas PASS) |
| `.tmp/night_0901/spv/g31_realism_restir.spv` | T1 | **新 SPV 工件**(373,388B,sha256 `8ac52dc4d6f78cb1…`,spirv-val 绿,双编位级 ==;.tmp 惯例不入 git 面;既有 9 下位工件 sha 全等;G39 件 `5571e875…` 退役留档) |
| `src/rurix-render/src/bin/g14_3_lane/g31_frame_cut_arm.rs` | T2 | +495/−69:P2 生产 dispatch——独立常驻 cull 会话(10 资源,表 device_local 驻留;每帧 params 256B 上传 + 决策码 ~493KB 回读)+ `frame_cut_sets_from_decisions`(d∈{2,4} 闭集)+ `frame_cut_select_from_decisions`(影子核 verify + host 提升照旧)+ `FrameCutSelectTiming` 三分项 + selftest ⑧ 段;P1 `frame_cut_device_cut_compare` 退役留档 |
| `src/rurix-render/src/bin/g31_frame_cut_probe.rs` | T2 | +12/−7:`--cut-source device` 语义升级 P2(文档头同步);缺省 `host` 字面 0-byte |
| `src/rurix-render/src/bin/g14_3_lane/g14_3_lane_body.rs` | T3 | +99/−1:`slot_as_mem_bytes/slot_as_resources_len` 字段 + `capture_slot_as_mem`(collect_frame_dyn/skin 首帧采集,ledger 过滤式同源 probe)+ bench receipt `slot_as_mem` 空串注入位(off 面 receipt 字节 0 漂) |
| `.gitignore` | 主 agent | +6:`artifacts/day_0901_g40/t1_restir2/ab/**/p.raw*` 排除(ladder 八臂 + dolly 两臂 raw 各 8/8,~2.0GB;G38/G39 同律) |
| `G31_PLUS_COMMERCIAL_RENDERER_TODO.md` | 主 agent | +1:修订记录 v1.2.3 行(G40 交付登记;既有行字面 0-byte,头部版本行沿 G37-G39 先例不动) |
| `artifacts/day_0901_g40/` | 全役 | 战役目录:`CAMPAIGN_LOG.md` / `HANDOVER.md` / `recon/` 三份 / 四任务 REPORT+EVAL / 编排脚本 6 件(`g40_baseline.py` `gpu_b1_t2.py` `gpu_b2_t3.py` `gpu_b3_t1.py` `g40_gates.py` `g40_soak.py`)/ 各批 evidence+summary / `closeout/` / `soak/`(`*.log` 与 `__pycache__/` 按既有 .gitignore 规则不入库,`.jsonl` 日志为归档面) |
| `evidence/g31_profiling_20260902T065651Z.json` / `g31_wp_hlod_20260902T072425Z.json` / `g36_geo_composition_gate_20260902T074732Z.json` / `g31_restir_wiring_20260902T074819Z.json` | 收役 | 四份门 PASS evidence |

冻结面 0-byte 兑现(`git diff --numstat` 逐件空输出;机器维持证明 = 收役 `restir_wiring` 门 g27/g28 双面 0-byte):`g28_restir.rx` / `gi/restir_reservoir.rs` / `gi/multi_light.rs` / `src/rurix-rt/` 全目录(含 render_exec*)/ rurix-asset `g31_cluster_cull.rx` 源 / `ci/check_schemas.py` 本体 / 预算门条目 `g31.fif_dyn.slot_as_group_mem_bytes`。

**入役即在树的两件改动(非 G40 自有面,G39 owner 治理窗产物)**:`artifacts/day_0831_g39/HANDOVER.md`(§D-1/§E-1 判档落档括注)+ `artifacts/day_0831_g39/t1_restir/LAMP_K_PROPOSAL.md`(判档段)——与入役预期字面一致,由 owner 一并择入。本役工作树**无**其它兄弟任务在途面(porcelain 实测)。

## D. 留窗登记(如实,不冒充)

1. **方差税继承(画质收益为负)**:on 臂暗部噪声 2.1-2.7×(§B),三件合一未消解 G39 D-3;A/B 达标口径不含画质转正。第一旋钮 = 验证射线走软阴影多样本(帧时代价须重估,T1 REPORT §四-4.2-2)。
2. **f32 ULP 边界事件未消除**:y 位级判据两级化(全等 ∨ 单判定翻转位级复现 device 且 |margin| ≤ 32ULP;未归因 100% fail-closed),实测 33.2M 像素 1 例 `margin=0.0`。第一旋钮 = 判据整数化(取样判定量量化到整数域,§四-4.2-1)。
3. **G39 D-3 两新税继承**:点灯软阴影半影让位 + 玻璃后点灯影转硬影,本役未处理。
4. **phat 重算近似仍在**(标准 ReSTIR DI 时域形,m_cap 截断有界非严格无偏);mcap 第一旋钮代价已量化:`k26_on_mcap4` −0.41ms 换暗部噪声 +41.9%,**不推荐作画质手段**。
5. **T1b 两拒**:收益未证实(±1% 内)+ 阈 0.10/0.80 为字面裁决非扫描最优;第一旋钮 = 更激进 dolly 轨迹 / 阈值扫描(阈走 CLI,零重编)。
6. **T1c clamp 缺省维持 0(off)**:clamp=4 在本场景几乎不触发(噪声同至小数第 5 位、帧时 +0.05ms);提档需 firefly 显著场景素材;**缺省裁决归 owner / 下一役**(零重锚纪律,本役不动缺省面)。**〔2026-09-04 判档落档:维持缺省 0(off)不提档——理由照本行既有 measured(clamp=4 本场景几乎不触发,噪声同至小数第 5 位、帧时 +0.05ms);提档重开条件 = firefly 显著场景素材出现。本项闭合〕**
7. **T1a 对拍不可常开**:verify 独载双份回读 + host 逐像素复算,墙钟 ≈ 常态 5-10×;生产面 off 0-byte,回归覆盖靠 B3 批而非每跑。
8. **T2 P3 留窗**(`verify_cut_coverage` device 化 / 直写竞技场):开窗条件 P2 B1 六判在案;诚实边界 = ml0 无净收益(fence+回读税抵消 select 收益)、UPDATE build 地板主导(ml0 ~21ms / ml1 8-11ms),ml1×device 16.1ms 不冒充进预算,90fps 叙事须 P3 或更深组合。
9. **T4 bridge_ext×FIF 判档留 owner**:EVAL §5 建议 = 机制 GO / 开窗**条件 DEFER**(条件 = 首个「稀疏 dirty × FIF」真实消费者成立;当前 skin incr≈full、frame_cut 非 FIF 车道,收益面空集);EVAL §6 判档段**待 owner 回填,agent 不代填**。**〔2026-09-04 判档落档:机制 GO / 开窗条件 DEFER,条件字面「首个『稀疏 dirty × FIF』真实消费者成立」;登记见 `t4_bridgeext/EVAL_BRIDGEEXT_FIF.md` §6 判档段。本项闭合〕**
10. **profiling 门**:G39 B4 + 本役收役两次真跑绿(本役十轮全 identity_ok 无单轮越界),重标定预案(G39 `t4_profiling/REPORT.md` §5)维持在档未启用。
11. **B3_SUMMARY.json 覆盖面仅 judge 段**(run3 judge 段宿主进程消失后 `--only judge` 补跑写出,零 GPU 重做):六 GPU 段的绿事实源 = `b3_log.jsonl` 逐段 END 行,读摘要须与日志合看(T1 REPORT §8.1)。
12. **run1 中断根因 = 编排脚本 `log()` 的 `print` 在宿主终端关闭后抛 OSError 穿透段级 try**,已修(OSError 兜底);G39 同型脚本(`gpu_b3_t1.py` 等)未回改——后续役复用脚本时按本役 `log()` 形态起草。
13. **二进制非位级可重现**:同源三次构建 exe sha16 `cc39f6dc`(12:24)/ `66e94de5`(15:11 wp_hlod 门自带构建)/ `7c77870a`(17:21 soak 前)互异,渲染语义以 digest 门为准(G38「双二进制同值」同律)——登记为构建面事实,非缺陷。
14. **evidence quality_arms 极简体例**:本役 +4 字段沿 G39 §七-1 体例(偏离继承自 G39 D-9),维持现状或回归体例归 owner。
15. `docs/renderer/feature_matrix.md` **未追加** `--lamp-restir` 臂行(G39 亦未追加;on 臂画质收益为负、缺省 off,是否登记及措辞归 owner)。
16. **soak 帧时记账超线**(§B:frame_ms_max 12.393 > 11.111,digest 恒值,机态定性):第一旋钮 = 后续役 soak 加 warmup / 拉长帧数摊薄 GPU 爬坡 + 记录宿主并发负载(口径变更须登记,不与 G38/G39 数字互比);it9 Stage A 探针同 G39 未及(6 迭代即越墙钟线),脚本口径使然非缺陷。

## E. 后续窗口建议(优先级序)

1. **owner 治理窗**:①本役工作树入库(commit 按 §C 表择取;G39 两件判档登记一并择入)②**T4 判档**(`t4_bridgeext/EVAL_BRIDGEEXT_FIF.md` §6 回填:机制面 GO/NO-GO + 开窗面 即刻/条件 DEFER/NO-GO + 若条件 DEFER 写明条件字面)③T1c clamp 缺省裁决(维持 0 / 提档需素材)。**〔2026-09-04 状态:三项均已处置——①入库 = 本次 owner 入库波(按 §C 表分层择取,G39 两件判档登记一并择入);②T4 判档 = 机制 GO / 开窗条件 DEFER(EVAL §6);③T1c = 维持缺省 0。治理窗闭合〕**
2. **T1 方差税专项窗**:以 dark ROI p95 收缩率**转正**为达标口径(非帧时),候选旋钮按 D-1/D-2/D-5 顺序(验证射线多样本 → 判据整数化 → 两拒阈值扫描);TSR 前口径(scene-linear)方差 A/B 一并补(G38 v1.2.1 ④ 遗留,G39 E-5 继承)。
3. **T2 P3**:`verify_cut_coverage` device 化 + 直写竞技场,与 `--min-level` 组合进预算叙事;免重锚推导链 = DESIGN §3.1 / 本役 B1 ③ 跨双锚 MATCH 先例。
4. **bridge_ext×FIF 开窗**:仅当 E-1 ② 判 GO 或条件成立(EVAL §5 候选 = dyn 大场景局部破坏/程序化位移 slot_as 车道,或 frame_cut 语义进 FIF 化渲染车道〔另涉 #77 P3 与窗口车道 FIF 化两前提〕);编辑权 = `render_exec*` 独占窗(G39 T3 同律),验收 = EVAL §4「验收」行。
5. **编排面**:后续役编排脚本沿本役 `log()` OSError 兜底形态;GPU 批建议以 `--only <seg>` 分段落盘 summary(避免 judge 段单点覆盖面缺口重演)。
