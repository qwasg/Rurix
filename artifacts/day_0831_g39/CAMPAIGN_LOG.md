# G39 战役日志 — ReSTIR 生产接线 + G38 留窗收尾(day_0831_g39)

> 日期:2026-08-31。入役基线:git HEAD `82a59ae3`(G38 `b05cd4ef` + DLSS5NR `82a59ae3` 已入库),工作树干净(porcelain 空,开役实测)。
> 纪律(全条硬性,沿 G37/G38):GPU 真跑全程 gpu_device_lock 锁内串行;`RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1`;构建 `CARGO_TARGET_DIR=H:\rurix\target-night`;既有行字面 0-byte 只追加;加性臂 = off==锚 + on 双跑位级 + VUID=0 + 帧时记账;A/B 走无 AE 臂集;本役预期零重锚;evidence schema 经 `_patch` 幂等纯追加;阈值程序产禁手写;>100MB 逐帧转储不入 git 面;首红先查判读器口径;子 agent 两层编排(侦察交接单先行,实施独占编辑权);不 git commit 归 owner。

## 锚表(G38 收官谱系,本役全程恒值预期)

| 锚 | 值 |
|---|---|
| all-off 8f/warmup2 | `sha256:55e4a92d25be959a91d111140b88eb81e0c7c29fe80d1aeee641aa78d8a86288` |
| full19 缺省 96f/warmup2(法线 v2) | `sha256:a5521e4708a814e364fd3bf95b18f0ab69b6646efd039246e8686533c30e4fb1` |
| bench 160f(= Stage A bistro/t100/tsr 格) | `…c1d28ad73783cc3c054ae0ce372b042a23d01df5491394cb38de7725e83b6c02` |
| RD-045 orbit 64+10 | `sha256:066395b0b6d877f546b7082560c093b4c325f39dae446abf3d59a8ad1023d56d` |
| ris_nee 单开 96f(不动抽验参照) | `sha256:851a61baf989733817bc4880e96ba1ededbea428e22e27842d0f4dc995e2b9b2` |
| transparency 单开 96f(不动抽验参照) | `sha256:af1f72643be83bd8f683bb47ab9c7da5dfc06329618fe86f61004a984eddaff5` |

## 任务表

| 任务 | 内容 | 编辑面(独占) | 验收 |
|---|---|---|---|
| T1 | ReSTIR 时域 reservoir 提灯臂 `--lamp-restir`(TODO #7,M100 车道集成窗) | `g31_window_present.rs` + `kernels/g31_realism.rx` + 新 host 模块 | off==锚 + on 双跑位级 + VUID=0 + A/B 阶梯(12/26/~48 簇 × on/off) |
| T2 | #90 skin 臂批次 B(WIRING_PLAN §1-A6 + §2-B2) | `g14_3_lane_body.rs` + `g14_3_pipeline_perf.rs` | skin 1\|2\|3 逐帧 digest 逐字节等 + 双跑 + skin_verify all_pass + 真动门 |
| T3 | slot_as 单源折叠(fif_dyn REPORT §7-3) | `render_exec.rs` + `render_exec_g37_fif_dyn.rs`(T2 后开窗) | 零语义门:静态 FIF + dyn + skin + fif probe 全量位级复证 |
| T4 | profiling 门 identity 判据多轮中位鲁棒化 | `ci/g31_profiling_smoke.py` + schema `_patch` | 门真跑绿或诚实红;容差 [−0.10,2.00] 不动 |
| T5 | #77 device cut 设计案(段 2 条件实施) | 段 1 仅 `t5_devicecut/DESIGN.md` | WIRING_PLAN 级设计案 + GO/NO-GO 判档 |

## 编辑权台账(禁并行写同文件;GPU 真跑全部归主 agent 锁内)

- 窗口 bin `g31_window_present.rs` / `g31_realism.rx` / 新 host include 件 → T1 子 agent。
- `g14_3_lane_body.rs` / `g14_3_pipeline_perf.rs`(+ 共享签名机械补参涉及的 include 消费方)→ T2 子 agent;禁改 `render_exec*.rs`。
- `render_exec.rs` / `render_exec_g37_fif_dyn.rs` → T3 子 agent,T2 GPU 批全绿后开窗。
- `ci/g31_profiling_smoke.py` + 新 `_patch` → T4 子 agent。
- CAMPAIGN_LOG.md 仅主 agent 写;各任务 REPORT.md 归各实施 agent。

## Wave 0 — 开役基线复证(全绿,2026-08-31 21:55)

- [x] git HEAD == `82a59ae3`,`git status --porcelain` 空(实测)。
- [x] 侦察交接单五份归档 `recon/`(T1 资产/T1 接线/T2T3/T4 运维/T5)。
- [x] 构建双 bin(target-night,1m51s,rc=0)。
- [x] 三锚复证 `g39_baseline.py` **BASELINE PASS**:n1 all-off==55e4a92d MATCH / n2 bench==c1d28ad7 MATCH(65s)/ n3 Stage A 格 MATCH / n4 full19==a5521e47 MATCH(267s);VUID=0 全程。evidence = `w0_baseline/G39_BASELINE.json`。
- [x] CPU 守卫 7/7 全绿:check_schemas PASS / budget_eval **330 pass 0 skip** / gpu_device_lock selftest / encode_parity selftest / texture_sampling selftest / vendor_license selftest / blocked_probes selftest。

## Wave 1 — 并行实施窗(发射登记)

- T2 子 agent:skin 批次 B(编辑权 = lane_body + pipeline_perf;禁 render_exec*/GPU)。
- T4 子 agent:profiling 门多轮中位(编辑权 = ci/g31_profiling_smoke.py + 新 _patch;禁 check_schemas.py 本体/容差字面/GPU)。
- T5 子 agent:device cut 设计案(纯文档 t5_devicecut/DESIGN.md)。
- T1 子 agent:--lamp-restir 时域 reservoir 臂(编辑权 = 窗口 bin + g31_realism.rx + 新 SPV 工件;禁 lane_body/g28/gi 金标准/GPU)。
- GPU 验收批全部归主 agent 锁内串行(B1 T2 → T3 开窗 → B2 → B3 T1 → B4 T4)。

## Wave 2 — GPU 批 1(T2 验收 + 折叠前基线)**B1 PASS**(22:36-22:55,19.4min 锁内)

- [x] **skin 三臂等价环全绿(硬门)**:`--skin-demo --inflight 1|2|3` × 双跑六跑全 OK;flip-trace **x1≡x2≡x3 逐字节 + 各臂双跑位级**同时成立 ⇒ **skin refit 实测纯函数,无需 L2a 降档**(G38 dyn refit 同结论);skin_verify 全 all_pass(窗级真动门过,motion_gate 数值在 `t2_skin/gpu/sv_*.json`);VUID=0。
- [x] dyn rebuild/refit ×1|2|3 基线收割 + eq_across 复证 OK(**含 G38 旧轨迹 x1 跨役字节对照 MATCH** = T2 对 dyn 面加性 0-byte 实证);静态 FIF ×1|2|3 eq_across OK;轨迹落 `t3_fold/baseline/`(T3 零语义门基线)。
- [x] bench 缺省 160f 负控 == `c1d28ad7` MATCH;fif probe 双臂 gates 全 true + trimmed_mean 在位(`t2_skin/gpu/evidence_fif_dyn_*_g39.json`);`calibrate_fif_budget.py --check` PASS(预算条目位级互核绿)。
- 产物:`t2_skin/gpu/`(B1_SUMMARY.json verdict=PASS + b1_log.jsonl + sv/receipt/evidence)。**T2 验收收口;T3 render_exec 独占窗现在开启。**

- [x] **T1 实施落地**(23:00):`--lamp-restir off|on` + `--lamp-restir-mcap`(默认 8,[1,64])九步全链——kernel 第 9 链位(+~300 行:M=8 闭式 WRS〔g28 同源比较形〕+ prev_vp 重投影〔g14_mv 同式〕+ m_cap 截断时域 merge〔host merge 同义〕+ 1 条验证射线 + `W=w_sum/(m·phat)` 三算术门形;唯一被改写既有行 = 点灯循环让位门,臂⑧先例形);窗口 bin +~400 行(params [72..76) 纯追加/prev_vp 64B 逐帧上传/reservoir ×2 parity 轮换 TSR 同律/binding 37-39〔bloom 45-47〕/AE `_RESTIR` 族/fail-closed 校验);新 SPV `g31_realism_restir.spv`(365,724B sha `5571e875…`,spirv-val 绿,复编位级 ==,既有 9 工件 sha 全等)。依赖集裁决:须随 lamp-lights+smooth+tex;`--gi2` 非依赖(kernel 实况);`--fg` 互斥(FG 静态下标族撞位,新增卫兵);不进 full dup 表(full19 锚零漂)。cargo check 0 error;check_schemas PASS(quality_arms 两键 properties-only 补丁);冻结面 g28/gi 三件/lane_body/rt 审计 0-byte。已知税 6 收 6 留如实登记(host 镜像对拍臂留窗/软阴影半影让位/玻璃硬影两新税)。REPORT = `t1_restir/REPORT.md`。B3 = `gpu_b3_t1.py`(锚/双跑/标定/阶梯/dolly/判读),排 T3 落地树稳后执行;raw 转储 gitignore 已登记。

- [x] **T3 实施落地**(23:21):`submit_pipelined_frame` 加 `Option<&SlotAsGroup>` 末参吸收三插入一换向;复制体 `g37_submit_pipelined_frame_slot_as` 连 doc 整删 378 行(render_exec.rs +125/−14,render_exec_g37_fif_dyn.rs +11/−389);公共入口/`SlotAsGroup`/`g37_validate_slot_as_frame` 0-byte;报错前缀经 `err_pfx` 局部变量承载(两路求值产物与折叠前逐字节相等,4 处 `.into()`→`format!` 构造形式差登记);L5805 跨文件字面量构造冻结注记改写为现状陈述。两消费方 cargo check rc=0 + 0 新增 warning;g37 单测 5/5(vulkan 特性门控下按 fif_dyn 窗先例只跑目标模块,登记);双向机械 diff 四件存档(`t3_fold/`)。B2 零语义门接跑。

### 首红裁决登记(23:00,T5 段 2 C1)

- **C1 device 臂真跑首红** rc=1:`FAIL device cut 表: 块 0 簇 604 有限 parent_error 3.4028235e38 撞 sentinel 域(≥1e9)`。**裁决 = 对拍臂自身域检口径错,产物零缺陷**(首红先查判读器口径纪律;G38 B6/B7/transparency 三先例同律):生产簇包根簇(无父)合法编码 = 有限 `f32::MAX`,新臂域检把「有限且 ≥1e9」一律判资产异常 fail-closed,设计只覆盖了非有限→sentinel 映射。已回递 T5 子 agent 修域检 + 期望码/red-arm 裁决条件同步 + selftest 补根簇锚;B5 批已终止待修后全量重跑。
- **修正落地**(23:05):撤除「有限 ≥1e9 拒」域检,上传律回归 harness 字面(有限原样透传含根 `f32::MAX`;非有限→2e9 sentinel;NaN 保留映射防 kernel/host 分叉)。**等价论证**:有限 ≥1e9 时 kernel `≥1e9` 分支得 1e9px、host `d_surface>e` 恒假分支得 +∞px,唯一消费谓词 `parent_px ≥ thr` 两侧同向饱和恒真,判定码逐位不变。事实链 = `dag.rs` L1665-66 根簇合法编码为有限 f32::MAX(单测钉死)且 harness v1.1.5 全绿本就锚在原样上传路径。red-arm 裁决条件字面 0 改动(复核仍正确);selftest 补 f32::MAX 根锚 PASS;build rc=0。行数账刷新 arm +594/−12。REPORT §6 裁决登记在案。B5 待 T3 落地树稳后与 B2 连跑。

## Wave 3 — GPU 批 2 + 批 5(终态树,23:24-23:45,连跑 21min 锁内)

- [x] **B2 零语义门 PASS(T3 折叠位级闭合)**:静态 FIF ×3 / dyn rebuild ×3 / dyn refit ×3 / skin ×3 折叠后单跑轨迹**逐字节 == 折叠前基线**(B1 收割件);fif probe 双臂 gates 全 true + 逐臂 digest 序列 == B1 evidence。T3 收口。
- [x] **B5 T5 段 2 验收 PASS(域检修正后全绿)**:C1 device 判定码逐项全等 ×16 帧 ×双跑 + decisions∈{2,4} 闭集;C2 dev==host digest 16 帧逐字节 + **跨 G38 t3_incr 参考锚 MATCH**;C3 red-arm rc≠0 且 mismatch 报文正确(对拍面真实消费的构造性证明);C4 ×--min-level 1 组合 PASS;C5 缺省面 digest == host(0-byte 回归)。device 证据税 `device_cut_probe_ms` mean=82.733ms 单列登记(P2 生产 dispatch 上界参考,不判读)。**#77 P1 等价门交付收口;P2/P3 留窗。**

## Wave 4 — GPU 批 3(T1 验收)

- **首跑(23:46-00:52)**:off 面全绿——**all-off/full19 双锚 MATCH(终态树,零重锚预期成立)**、off 阶梯三档 OK、grid 标定完成(0.10→38 簇/0.075→61/0.05→78,取 0.10;「~48 档」如实登记为 **38 簇可达档**);off 帧时 measured:k12 9.11ms / k26 **11.38ms 超线** / k38 13.30ms(26 簇超预算现象复现,G38 方向一致)。**全部 on 臂帧 0 fail-closed 红**(rc=1,VUID=0)。
- **首红裁决(00:57)**:报文 `buffer_uploads: StableResourceId(38/46) 为 DEVICE_LOCAL 驻留不可 map`——按 rt 编号规则(槽位 = id−1)解码恰为 **prev_vp 37/45 本尊**(非下标错位):prev_vp 小 buffer 创建时误带 `device_local: true`(复制 reservoir 创建形所致),rt G14.10d 上传校验帧 0 fail-closed。**产物判定 = T1 接线 bug,非判读器口径**(判读器两假设排查在案)。修复 = 两 descs builder 各一处 `device_local: true→false`(均在 `if lamp_restir` 分支内,off 路径零触碰);kernel 零改动 SPV sha 不变;cargo check rc=0;REPORT §八登记。
- **重跑(00:57-01:52)全绿收官 B3 PASS**:anchors 复证 MATCH ×2 → det on/onmin 双跑位级 ×2 → 阶梯六跑全 OK → dolly 240f ×2 digest_seq 位级 → judge 矩阵落盘。**验收终判(measured)**:
  - 帧时 render_wall p50:k12 off 9.560/on 7.390ms;**k26 off 11.546(超线)/on 7.526ms——26 簇档 restir on 进 11.11ms 预算,余量 +3.58ms,战役目标达成**;k38 off 13.755/on 7.417ms。on 臂帧时与簇数解耦(7.4-7.5ms 恒平,机制兑现)。
  - 方差如实登记(不判红):on 臂 dark ROI p95 时域噪声升 ~1.5-2.6×(dark_arch 0.0093→0.0191/0.0241/0.0245 @u8 归一;shrink −105%/−157%/−162%)——1-spp 随机换选代价,EVAL §0 预告形,TSR 时域部分吸收;dark_table −52~−72% 同向。
  - k12_off digest == full19 锚 `a5521e47…`(阶梯基线自证);VUID=0 全程。evidence = `t1_restir/ab/T1_AB_MATRIX.json` + `ev/` + B3_SUMMARY。
- **lamp-k 提档提案判 GO,只登记留 owner**:`t1_restir/LAMP_K_PROPOSAL.md`(A 维持+B 留窗建议;默认翻转/重锚不预支)。**T1 收口。**

### Wave 1 落地登记

- [x] **T5 段 2 实施落地**(22:44):P1 对拍臂两文件加性(`g31_frame_cut_arm.rs` +561/−12、`g31_frame_cut_probe.rs` +52/−2,合计 +613 超设计 300±80 量级——分解登记为 doc/rustfmt/selftest 实做/red-arm 裁决,零功能越界);SPV 现编 `.tmp/g39_gates/t5_devicecut/g31_cluster_cull.spv`(rurix-asset 源 0-byte,spirv-val rc=0);缺省 `--cut-source host` 字面 0-byte;cargo check 0 error + C0 selftest PASS(⑦ 段六锚)。**偏离登记 7 项**(REPORT §3),最重要:red-arm 原案「簇 0 self 球篡改」经事实链证明在生产包上结构性空转(叶 error≤0 不读球)——改期望码驱动受害裁决(模式甲/乙,构造性必红)。C1-C5 GPU 批 = `gpu_b5_t5.py`,排 B1 后进锁。
- [x] **T5 段 1 设计案交付 + 判档 GO**(22:19):`t5_devicecut/DESIGN.md`(七章,零代码)。判档根据:cut = 相机纯函数、零上帧回读反馈环(与 WP §4 NO-GO 场景结构相反);等价门锚在判定码最上游闭集。段 2 圈定 = **P1 probe-only `--cut-source device` 决策码回读对拍臂**:三关超集 kernel 0-byte 消费(params/数据域中和:关1 六平面全零/关2 cone_cutoff=1.0/关4 view 零行短路,判定码域收缩 {2,4} 恰成对拍闭集 + decisions 闭集断言 + red-arm);kernel 源留 rurix-asset 冻结,rurixc 现编 SPV 到 `.tmp/g39_gates/t5_devicecut/` + `--cull-spv` 运行时装载;仅 `g31_frame_cut_arm.rs`+`g31_frame_cut_probe.rs` 两文件 ~300±80 行加性。**预算纪律随 GO 登记防误读**:本 GO 不承诺帧时收益(ml0 下 UPDATE build 19.8-21.3ms 地板主导);进预算路径 = P2/P3 × --min-level 留窗,开窗条件 = P1 C1-C5 全绿。段 2 实施子 agent 已发射。

- [x] **T4 实施落地**(22:16):`ci/g31_profiling_smoke.py` +182/−42——`IDENTITY_ROUNDS=5`(+`--rounds` 闭集 [1,9]),锁内腿编排 4→12(off ×2 + on ×10,各轮 profile `_r<i>`),identity 判据消费逐分量**中位数**后套用不变规则;fact ⑤ zero-drift 加固为 off 锚 × on 全 N 轮 digest 全等(只加严)。evidence schema 纯追加可选块 `identity_rounds`(经新建幂等补丁 `ci/_patch_g31_profiling_rounds_schemas.py` 落地,复跑幂等;**命名偏差登记**:交办名被 C7 期历史补丁占用,按二号补丁先例另名)。**容差 [−0.10,2.00] 四面同源 0-byte(git 面证明)**;check_schemas.py 本体 0-byte。selftest 72 断言 PASS(含 5 轮 2 越界中位在带⇒绿 / 中位越界⇒红 红绿臂);check_schemas PASS。REPORT = `t4_profiling/REPORT.md`。门真跑(预期 ~15-30min)排 B4,在 T1/T3 落地后与收役回归合并执行(门内自建 window bin,须树稳定)。

## Wave 5 — GPU 批 4 + 收役回归 + soak(2026-09-01 01:54-03:16,锁内串行)**全绿收官**

- [x] **B4 profiling 门真跑 GREEN(T4 多轮中位后首真跑)**:`g31_profiling_smoke.py --gate g31.waveC.profiling` **GATE PASS**(wall 863.9s)。**鲁棒化机制当场兑现**:g14 腿第 4 轮 host_residual −0.249892 单轮越界(G37 W6 历史红轮 2 形态 −0.288 同向同量级),五轮中位 +0.091333 在带 ⇒ 判据③中位绿;g31 腿五轮全绿(residual 0.835~0.987,中位 0.883587)。fact ⑤ 加固面实跑绿:off 锚 × on 全 5 轮 digest 位级全等(g31 presented+render 双锚 / g14 last_frame)。容差 [−0.10,2.00] 字面 0 动;T4 REPORT §5 重标定预案**未启用**(两腿各有全绿轮 + 越界轮孤立 = 单轮抖动定性成立)。evidence = `evidence/g31_profiling_20260831T175627Z.json`(identity_rounds 逐轮明细在档)。
- [x] **收役门全量回归 W_GATES PASS**(`g39_gates.py`,fails=[],`closeout/W_GATES.json`):wp_hlod 门 PASS(635.5s)/ g36_geo 门 PASS(1189.5s,十 facts 全绿)/ restir_wiring 门 PASS(23.0s,G28 门维持复跑全绿 + g27/g28 冻结面提交面+工作树双面 0-byte——T1 维持证明字面兑现)/ frame_cut probe 回归全绿(selftest + incr==full 16 帧位级 + incr 跨进程双跑 + ml1 降档臂 + **跨 G38 t3_incr 锚 MATCH**;bridge copy incr 均 0.33-0.55ms / full 8.05ms 与 G38 同向)/ CPU 守卫 7/7 全绿(budget_eval **330 pass 0 skip**)。fif probe 回归由 B2 终态树承载不重复(g39_gates.py 文档串登记;B2 后唯一代码变更 = T1 window bin prev_vp device_local 两行修复,fif 消费面〔rt+lane〕字节不涉)。
- [x] **soak PASS ≥1800s**(`g39_soak.py`,02:44-03:16,`soak/G39_SOAK.json`):wall **1936.2s** / 6 迭代;前置双锚断言 all-off==`55e4a92d` + full19==`a5521e47` MATCH;32f 迭代锚钉死 `a8204b3b`(G38 在案值非自举)**6/6 位级恒值**;it4 Stage A 探针 MATCH(it9 未及:6 迭代即越 1800s 墙钟线,如实登记非缺陷);frame_ms_max **10.798 ≤ 11.111 预算**;VUID=0,fails=0。

## 收官登记(2026-09-01)

- 五任务全收口:T1 ReSTIR 臂(B3 PASS,26 簇档 on 7.526ms 进预算 +3.58 余量)/ T2 skin 批次 B(B1 PASS,#90 收口)/ T3 slot_as 折叠(B2 零语义 PASS,复制体 −378 行)/ T4 profiling 鲁棒化(B4 GREEN)/ T5 device cut 段 1 GO + 段 2 P1 等价门(B5 PASS,#77 P1 收口)。
- **零重锚预期兑现**:all-off/full19/bench/RD-045 全程未触,收役三重复证 MATCH(W0 基线 / B3 终态树 / soak 前置);G38_ANCHORS 谱系恒值,ris_nee/transparency 不动抽验参照未消费未漂。
- 文档:五任务 REPORT.md 在档 + `t1_restir/LAMP_K_PROPOSAL.md`(判 GO,只登记留 owner)+ `HANDOVER.md`(锚表/门台账/留窗/后续窗口)+ TODO 表 v1.2.2 修订行;t1 ab 逐帧 p.raw ~1.2GB gitignore 排除(G38 t5_risnee 同律)。
- **不 git commit,归 owner**;工作树另有兄弟任务在途面(`g35_render_resolve.rx`/`g35_render_splat.rx`/`g35_particle_lane.rs` + `artifacts/day_0831_site/`)不混,按文件名显式择取先例留树。
