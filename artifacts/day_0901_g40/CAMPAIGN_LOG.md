# G40 战役日志 — ReSTIR 画质补窗 + device cut P2 + G39 两留窗收尾(day_0901_g40)

> 日期:2026-09-01。入役基线:git HEAD `1478859a`(feat(g39) day_0831 收口波),工作树仅含两件判档登记改动(`artifacts/day_0831_g39/HANDOVER.md` + 同役 `t1_restir/LAMP_K_PROPOSAL.md` 判档段)——与入役预期字面一致(porcelain 实测两行 M)。
> 纪律(全条硬性,沿 G37/G38/G39):GPU 真跑全程 gpu_device_lock 锁内串行;`RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1`;构建 `CARGO_TARGET_DIR=H:\rurix\target-night`;既有行字面 0-byte 只追加;加性臂 = off==锚 + on 双跑位级 + VUID=0 + 帧时记账;A/B 走无 AE 臂集(EXPLICIT_NOAE 基 = G39 t1_restir/REPORT §六 字面 + `RURIX_G18_AMBIENT=0.004`);evidence schema 经 `_patch` 幂等纯追加;阈值程序产禁手写;>100MB 逐帧转储 .gitignore 登记不入 git 面(本役 A/B 重跑 p.raw ~1.2GB,G38/G39 同律);首红先查判读器口径;CAMPAIGN_LOG 仅主 agent 写;零重锚;不 git commit 归 owner。
> **编排形态登记(如实)**:本执行环境无子 agent 调用工具——主 agent 串行承担侦察与实施两层角色;侦察交接单照例先行归档 `recon/`(实施前必备),编辑权台账照例逐任务独占登记。与任务书「两层子 agent 编排」的偏差仅在执行主体,交接单/独占编辑权/GPU 锁内串行三条纪律不变。

## 锚表(G38 收官谱系,G39 全程恒值,本役预期继续恒值)

| 锚 | 值 |
|---|---|
| all-off 8f/warmup2 | `sha256:55e4a92d25be959a91d111140b88eb81e0c7c29fe80d1aeee641aa78d8a86288` |
| full19 缺省 96f/warmup2 | `sha256:a5521e4708a814e364fd3bf95b18f0ab69b6646efd039246e8686533c30e4fb1` |
| bench 160f(Stage A bistro/t100/tsr 格) | `…c1d28ad73783cc3c054ae0ce372b042a23d01df5491394cb38de7725e83b6c02` |
| soak 32f/warmup2 | `sha256:a8204b3b93845f557656231e5b3e2407bcbda030857d8e0d87ace0b48b32ac09`(G38 在案值,钉死非自举) |
| RD-045 orbit 64+10 | `sha256:066395b0b6d877f546b7082560c093b4c325f39dae446abf3d59a8ad1023d56d` |
| ris_nee / transparency 单开 96f | `851a61ba…` / `af1f7264…`(不动抽验参照) |

lamp-k 判档 A 已落档本役消费(`artifacts/day_0831_g39/t1_restir/LAMP_K_PROPOSAL.md` 判档段):默认 12/0.6/off 不动不提档;26 簇交付形态 = `RURIX_G31_LAMP_GRID_M=0.15 --quality full --lamp-k 48 --lamp-restir on`;B 留窗不预支。

## 任务表

| 任务 | 内容 | 编辑面(独占) | 验收 |
|---|---|---|---|
| T1(主菜) | ReSTIR 画质补窗三件合一:T1a host 镜像对拍臂(`--lamp-restir-verify`)+ T1b disocclusion 深度/法线拒 + T1c per-pixel phat 钳制(消解 G39 HANDOVER §D-2/D-3/D-4) | `g31_window_present.rs` + `kernels/g31_realism.rx` + 复编 SPV;**禁改** g28_restir.rx / gi/restir_reservoir.rs / gi/multi_light.rs / lane_body / rt | B3:off==双锚 + on 双跑 + 镜像对拍批(≥16f×双跑+red-arm)+ dolly 240f + A/B 阶梯重跑 + 风暴组合首验 |
| T2 | device cut P2 生产 dispatch(#77,消费 G39 B5 等价门谱系;DESIGN §2.7 P2 行 + §4-2) | `g31_frame_cut_arm.rs` + `g31_frame_cut_probe.rs`;kernel 冻结/rt/lane_body/窗口 bin/schema 0-byte;缺省 `--cut-source host` 字面 0-byte | B1:digest 16f==host 位级 + 双跑 + ml1 + incr==full + 跨锚 MATCH + red-arm + 0-byte 回归 + 帧时分项 |
| T3 | skin/dyn 生产规模 AS 副本内存 evidence 补登(§D-7) | `g14_3_lane_body.rs`(receipt/evidence notes 加性字段);禁改 render_exec*;预算门条目 0 动不混 | B2:dyn/skin ×2\|3 字段在档 + flip-trace 对 G39 B1/B2 位级不漂 |
| T4 | bridge_ext×FIF rt 平行入口评估(纯文档,判档留 owner) | 仅 `t4_bridgeext/EVAL_BRIDGEEXT_FIF.md` | 形态案 + 决策语义分析(WP §4 对照)+ 工程量/风险 + GO/NO-GO 建议 |

## 编辑权台账(禁并行写同文件;GPU 真跑全部主 agent 锁内)

- T1:窗口 bin + g31_realism.rx + SPV(`.tmp/night_0901/spv/`)——独占。
- T2:frame_cut arm + probe——独占(与 T1/T3 cargo check 面交叠,收尾双绿,G39 T5 §5-7 先例)。
- T3:lane_body——独占(禁改 render_exec*)。
- T4:纯文档。
- GPU 批序:B1(T2)→ B2(T3)→ B3(T1,终态树 + 锚复证)→ 收役。

## Wave 0 — 开役基线复证(全绿,2026-09-01 17:13)

- [x] git HEAD == `1478859a`,porcelain 仅两件判档登记改动(实测,与预期一致)。
- [x] 构建 4 bin(g31_window_present / g14_3_pipeline_perf / g31_frame_cut_probe / rurixc,target-night,182s rc=0)。
- [x] 三锚复证 `g40_baseline.py` **BASELINE PASS**(388s 锁内):n1 all-off==55e4a92d MATCH / n2 bench==c1d28ad7 MATCH / n3 Stage A 格 MATCH / n4 full19==a5521e47 MATCH;VUID=0 全程。evidence = `w0_baseline/G40_BASELINE.json`。
- [x] CPU 守卫 7/7 全绿:check_schemas PASS / budget_eval **330 pass 0 skip** / gpu_device_lock selftest / encode_parity selftest / texture_sampling selftest / vendor_license selftest / blocked_probes selftest(`w0_baseline/cpu_guards.log`)。
- [x] 侦察交接单三份归档 `recon/`(R1_T1:回读面可达不触 rt + stride-12 承载案 + NoContraction 先决;R2_T2:独立常驻 cull 会话案 + 决策消费链 + 计时插桩点;R3_T3T4:ledger 账机制 + receipt 注入位 + T4 素材集)。

## Wave 1 — T2 实施 + GPU 批 B1(18:39-18:46,8.4min 锁内)**B1 PASS**

- [x] **T2 实施落地**:`--cut-source device` 语义升级 P1 对拍臂 → **P2 生产 dispatch**——独立常驻 cull 会话(10 资源 = kernel buffer 布局字面继承,表三件 device_local 驻留 staging 上传,每帧仅 params 256B 上传 + 决策码 ~493KB 回读);host 由 d==4 构造 cut 集(`frame_cut_sets_from_decisions` 闭集断言 d∈{2,4})→ `verify_cut_coverage` host 影子核直跑回读集(fail-closed 逐字保持)→ min-level 提升照旧 host → 既有差集/上传/refit 施加链 0 改;`frame_cut_select_ext` 加计时尾参,cut_ms 拆 select/verify/promote 三分项(host/device 双臂恒出,DESIGN §4-2 分项登记义务);red-arm 承接 = 篡改 ⇒ 决策翻转 ⇒ 覆盖性必破必红;P1 `frame_cut_device_cut_compare`(run_compute 逐帧,82.7ms 上界)退役留档;selftest ⑧ 段(决策码逆展平 + ml0/ml1 与 host select 后链同判)。缺省 host 字面 0-byte;kernel/rt/lane_body/窗口 bin/schema 0-byte。cargo check 双消费方绿 + selftest PASS。
- [x] **B1 PASS**(`gpu_b1_t2.py`,六判全绿):①device 16f digest == host 位级 + 臂内建双跑 + 跨进程双跑 ②×ml1 PASS ③incr==full + **跨 G38 t3_incr / G39 t5_dev 双锚 MATCH** ④red-arm rc≠0(「red-arm 模式」+「覆盖性」报文)⑤缺省 host 回归 + 窗口臂 5540ecae 锚(W0 二进制;T2 树复验归 B2 附录)⑥帧时 measured:**cull dispatch GPU 0.113ms**(P1 run_compute 82.7ms 上界 → 会话化后純 GPU 三个量级降)/dev select_ms 4.99(wall 含 fence+回读+构造)/**ml1×device 组合墙钟 16.095ms 落 DESIGN §4-2 预期带 ~15-19ms**(对照 G38 ml1 host 形态 23.7-27.3ms;ml0 墙钟 29.1ms 维持 build 地板主导不冒充)。evidence = `t2_devicecut/`(B1_SUMMARY + ev/)。

## Wave 2 — T3 实施 + GPU 批 B2(18:52-19:02,10.1min 锁内)**B2 PASS**

- [x] **T3 实施落地**:lane_body 加 `slot_as_mem_bytes/slot_as_resources_len` 字段 + `capture_slot_as_mem`(collect_frame_dyn/skin 双半程首帧采集;ledger 过滤式 = fif_dyn_probe `slot_as_mem_from_ledger` 同源)+ bench receipt 空串注入位 `slot_as_mem` 块(非 slot_as 臂 = 空串 ⇒ off 面 receipt 字节 0 漂);**预算门条目 0 动不混**。
- [x] **B2 PASS**(`gpu_b2_t3.py`):①字段在档 measured:**dyn ×2 = 240,614,400B(120.3MB/槽)/ ×3 = 360,921,600B;skin ×2 = 240,621,312B / ×3 = 360,931,968B**(生产 bistro 规模「数百 MB 级」预告兑现;inflight=1 负控 receipt 无键)+ skin_verify 全 all_pass ②flip-trace 六臂(dyn rebuild x1|2|3 + skin x1|2|3)对 G39 B1/B2 在案件**位级全等** ③B1f 附录:T2+T3 树新建窗口 bin 复验 5540ecae 锚 MATCH(B1 首验二进制时序缺口闭合)。evidence = `t3_asmem/`(B2_SUMMARY + receipts + ft_*)。

## Wave 3 — T1 实施(进行中)

- kernel:stride 4→12(几何面入池)+ T1b 两拒(merge 计数门折入)+ T1c W·phat 钳(params[75..78) 启用/追加);复编 SPV `.tmp/night_0901/spv/g31_realism_restir.spv` = **sha256 8ac52dc4…**(373,388B,spirv-val 绿,双编位级 ==;G39 件 5571e875… 退役留档)。
- 窗口 bin:五子旗标闭集 + verify 回读注册/订阅/消费 + 镜像模块(`g31_lrs_mirror_frame`,臂⑨逐字复算,y 位级硬门 + W/w_sum p100)+ evidence quality_arms 四字段 + verify_stats 块。**NoContraction 先决已在树**(B4 纹理接线 L9454 对场景 SPV 恒注入,restir 依赖 textures 恒经该路——R1 侦察修正)。cargo check 绿。

### Wave 3 收尾登记(2026-09-02;上方 Wave 3 标题「进行中」按**只追加纪律不改写**——以本节为准)

> 体例根据:本役纪律「既有行字面 0-byte 只追加」与「Wave 3 去『进行中』」两条在字面上冲突,
> **判档向上取严 = 保 0-byte,以追加节收口**(G39 「Wave N 落地登记」回填子节先例同构)。

- [x] **B3 PASS**(`gpu_b3_t1.py` 七段;终判 = `t1_restir2/B3_SUMMARY.json` `verdict: PASS` /
  `fails: []` + `b3_log.jsonl` 六 GPU 段逐段 `seg_fails: []`)。**T1 三件合一收口。**
- [x] **三跑谱系如实留档**:run1(09-01 19:47-20:18)anchors + det 部分绿后**整批中断** —
  `seg.det EXC OSError: [Errno 22]`(宿主终端关闭致 stdout 句柄失效,`log()` 的 `print` 抛错
  穿透段级 try);修复 = `log()` 加 OSError 兜底(日志已落盘时控制台不可写不应中断 GPU 批),
  run2/run3 未复现。run2(09-02 10:10-10:52,pid 38896)verify 三跑连红 → 主 agent ABORT,
  裁决见下节。run3(12:24:43 起,GPU 六段至 14:14:19;窗口 bin 12:24 重建 exe sha16
  `cc39f6dcf70ff02f`,**kernel/SPV 零改动** spv sha16 `8ac52dc4d6f78cb1`)全段绿。
- [x] **judge 段中断与补跑(如实,非掩饰)**:run3 六 GPU 段 14:14:19 全部收束后,零 GPU 的
  judge 段执行中途宿主进程消失,未落任何日志行、两产物均未写出。诊断:Application 日志无崩溃
  事件 / H 盘余量 141.3GB / dolly 两臂 raw 8/8 齐全 —— **非资源面**。因该段零 GPU 且纯读盘
  (素材 = 已落盘 raw + ev.json + prof.json),按 `--only judge` 补跑(14:52:28-14:52:30),
  **未重做任何 GPU 工作**。**由此 `B3_SUMMARY.json` 的 `segments` 仅 `["judge"]`——六 GPU 段
  的绿不在该摘要内,事实源 = `b3_log.jsonl` 逐段 END 行,读摘要须与日志合看。**

#### 首红裁决登记(09-02 10:33-10:52,T1a verify y 位级硬门)

- 症状:`verify.r1`/`r2`/`clamp4` **三跑同一签名**判红 ——
  `px=(1768,1044) host_y=7 dev_y=13 host_wsum=dev_wsum=2.038904e-1 host_m=dev_m=16`,
  帧 8(绝对序),帧 0-7 共 16.6M 像素 y 位级全等。
- **裁决 = 判读器口径过严,产物零缺陷**。排除假设四条:①镜像链算错(排除:w_sum 位级同值 +
  m 同值 ⇒ 链逐字正确,分歧只在最后一次取样判定)②随机维不同源(排除:同上,w_sum 为 8 候选
  权重累加,随机维有偏必分叉)③回读错位/parity 反(排除:错位应全帧分叉,实为单例)④机态随机
  (排除:三跑同像素同帧同值)。确诊 = **f32 取样判定边界事件**(WRS 判定量 `q = w/w_sum − u`
  落在 f32 可分辨精度内,两侧舍入方向不同致选择翻转;Vulkan FDiv ≤ 2.5 ULP 非正确舍入)。
- **同时暴露第二缺陷(代码审查发现,非跑出来)**:phase 红臂篡改常量 `0.618034` 与 kernel
  黄金比常量 f32 **同值** ⇒ 篡改等于没篡改,**红臂空转** —— 即该 fail-closed 证明链在 run2
  之前无效(run2 在 clamp4 即被终止,根本没跑到红臂)。
- 修正落地(12:24,四处成套):判据两级化(`g31_lrs_chain` 加 `flip` 参数 → 单判定 take/keep
  取反后 (y,m) **位级复现** device **且** |margin| ≤ `G31_LRS_ULP_BOUND` 32ULP ⇒ 计
  `y_attributed`,否则 `y_unattributed` ⇒ 帧尾 fail-closed)+ `G31_LRS_ATTRIB_CAP` 4096 +
  phase 常量 `0.618034→0.62` + resv 篡改改 `y+1, m+1` + schema `verify_stats` v2
  (`y_unattributed` const 0 钉进 schema,`check_schemas` PASS)。
- **等价论证 = 判据被收紧非放宽**:未归因仍 100% fail-closed;归因是构造性位级复现证明而非
  容差比较;ULP 界为独立第二条件取与;ATTRIB_CAP 令红臂全帧分叉走不进归因路,必红性不被吞掉。
- 复验(run3 同一像素走归因路):`判定 k=8 margin=0.000000e0(|margin| ≤ ULP 界 1.907349e-6)
  host_y=7 dev_y=13 flip_cand=13 m=16(单判定翻转位级复现 device,计 measured 不判红)`。
  **`margin = 0.0` 判定量恰等于零** —— 两侧舍入方向不同是必然而非偶然;k=8 = 时域 merge 判定,
  与「帧 0-7 无历史全等、帧 8 首次 merge 才分叉」自洽。

#### B3 measured(两条配置轴分标,不混不互推)

**轴 A = EXPLICIT_NOAE 基 + `RURIX_G18_AMBIENT=0.004`**(det / verify / dolly 三段):

- anchors 硬门:`55e4a92d…` MATCH(146.9s)/ `a5521e47…` MATCH(421.2s),**三跑批第三次复现**
  ⇒ off 面字节隔离律兑现(params 扩面/stride 12/两拒/钳全在 on 门内)。
- det:on `9fe2cfa5…7cca` / onmin `6a086fc7…0349`,各 r1==r2 位级,且**跨 run1/run2/run3 三跑、
  两个窗口 bin 二进制恒值** ⇒ run3 重建只动 verify 判读器未触渲染链。on 臂 digest 对 G39 换代
  (SPV `8ac52dc4` + stride 12 + T1b 缺省拒进链),**如实登记不判红,on 臂无锚**。
- verify 六判全绿:r1/r2 `e226f3ce…40ba` 双跑位级 + clamp4 `85d57adb…c913` +
  red_phase/red_resv 双红臂 rc=1 必红。镜像统计**三跑逐字节同值**:`frames=16
  pixels=33,177,600 hit=33,177,600 merged=30,175,964(90.9%)y_mismatch=1 y_attributed=1
  y_unattributed=0 margin_abs_p100=0.0 ulp_bound=1.907349e-6 m_mismatch=0
  wsum_absdiff_p100=1.220703e-4 w_absdiff_p100=1.079102`。
- **T1c 正交性实测自证**:clamp4 digest 与无钳 r1 **不同**(钳制确实改输出)而镜像统计**逐字节
  相同**(reservoir 链零扰动)⇒ 兑现「只钳输出消费、写回四元组不动」设计断言。
- dolly 硬门:rej 缺省臂(0.10/0.80)240f 双跑,`digest 3cc8206f…` 与逐帧 `digest_seq`
  (**242 项**)双双位级相等。

**轴 B = 缺省 / `--quality full` 预设轴**(ladder 八臂;k12_off 须等于 full19 缺省锚故不可走
NOAE 轴)。帧时 = profile `render_wall` p50(**字段实测在位,未回落** evidence
`real_render_frame_ms`;括注为后者):

| 档 | off p50 | on p50 |
|---|---|---|
| k12(缺省 12 簇) | 9.785(9.996) | **7.861**(8.010) |
| k26(grid 0.15/k48,26 簇) | **11.476 超线**(12.111) | **7.841**(7.991) |
| k38(grid 0.10/k96,38 簇) | **14.211 超线**(14.323) | **8.243**(8.470) |

- `ladder.k12_off_anchor` 硬门通过(k12_off == full19 锚,阶梯基线自证);**唯一性能硬门
  `judge.k26on_budget` 通过:7.8412 ≤ 11.11,余量 +3.27ms** —— G39 判档 A 钉死的 26 簇交付
  组合在本役终态树复现进预算。on 臂三档 7.84-8.24ms 恒平(极差 0.40ms),off 臂随簇数近线性
  上升 ⇒ 「与灯数 O(1)」机制在新形态(stride 12 + 两拒 + 钳)下继续成立。
- storm 三条并联硬门通过:`--window-storm 3 × --lamp-restir on` 30f/warmup4,rc=0 ∧ VUID=0 ∧
  `resize_eras=1` ∧ `exit_reason=frames_done` ⇒ **G39 D-4 末句「风暴×restir 组合未验收」消解**。
- 全段 **VUID=0**;raw 转储 ladder 八臂 + dolly 两臂**各 8/8**(205.7MB/臂,合计约 2.0GB),
  **零素材缺口**,`.gitignore` 已登记不入 git 面。

**两条负面结果(如实登记,不进全绿叙述)**:

1. **画质收益为负**。dark ROI p95 时域噪声 off→on 收缩率三档全负 —— k12 **−108.14%** /
   −74.49%,k26 **−164.21%** / −66.14%,k38 **−167.42%** / −54.07%,即 on 臂暗部噪声升至
   off 臂 **2.1-2.7 倍**,与 G39 登记 1.5-2.6× 同量级略差。**本役三件合一未消解 G39 D-3 的
   方差税**,该税继承留窗。A/B 达标口径 = 帧时进预算 + 确定性位级,噪声升幅登记不判红。
2. **T1b 两拒边缘改善 ≈ 0**。rej vs norej 同轨迹同切片:edge_l −0.06% / edge_r −0.83% /
   dark_arch +0.02% / dark_table −0.53%,四 ROI 全落 ±1% 内,低于机态噪声地板。判读:本 dolly
   轨迹 disocclusion 暴露面不足以让两拒产生可测收益,**「两拒有边缘收益」在本役素材上未被证实**;
   机制正确性由 `dolly.pair` 位级门 + kernel 门乘形态(阈 0 ⇒ 恒 1 逐字复现 G39 v1 语言形)承担。

**T1c 缺省裁决 = 维持 0(off)**:`k26_on_clamp4` 噪声与基准同至小数第 5 位、帧时 +0.05ms ⇒
clamp=4 在本场景几乎不触发(W·phat 典型值低于钳线),提档需 firefly 显著场景素材,本役不具备。
附带成果:mcap 第一旋钮代价首次量化 —— `k26_on_mcap4` 帧时 −0.41ms 换暗部噪声 **+41.9%**
(0.024788→0.035174),**不推荐作为画质手段**。

施工细节与八槽位登记见 `t1_restir2/REPORT.md`(§八 = 主 agent B3 回填章)。

## Wave 4 — 收役回归 + soak(2026-09-02;门段各自持锁 / probe 与 soak 段主 agent 锁内串行)

> 编排形态(如实,沿 L5):本波仍无子 agent 调用工具——主 agent 串行承担;唯一并行 = soak(GPU 锁内)与收官文档(零 GPU)两线并行,不涉同文件写。

- [x] **收役门全量回归 W_GATES PASS**(`g40_gates.py --only all`,14:52:54-15:54:10,`fails: []`,evidence = `closeout/W_GATES.json` + `gates_log.jsonl` + `gates_stdout.log`):
  - build 段:`g31_frame_cut_probe` / `g31_fif_dyn_probe` 终态树 cargo **Fresh**(0.2s×2——两 bin 已于 10:03 以 T2+T3 终态树构建,B3 run2/run3 期间 lane_body/arm/probe 三件零改动,故无重编)。
  - gates 段(四 ci 门各自持锁):**profiling GATE PASS**(1020.0s;G39 T4 N=5 中位判据**第二次真跑绿**——g31 腿五轮 residual 1.162~1.888 中位 1.370 / g14 腿五轮 −0.024~0.310 中位 0.109,**十轮全 identity_ok 无单轮越界**,重标定预案未启用;fact profiler_zero_render_drift:g31 digest+render_digest 双锚 off×on 全 5 轮位级 + g14 last_frame 全 5 轮位级)/ **wp_hlod PASS**(869.8s)/ **g36_geo PASS**(1387.7s)/ **restir_wiring PASS**(47.2s;G28 门维持复跑 y 锚 20000/20000 p100=2.831e-06 ≤ tol + 车道双臂 var 15.844× + off 静态锚 `c66e9f0e` 零漂 + **g27/g28 冻结面提交面+工作树双面 0-byte** = T1 禁改面〔g28_restir.rx / gi 金标准〕机器维持证明)。
  - framecut 段(锁内 15:48:20-15:54:01):selftest(含 T2 ⑧ 段)+ host incr/full/incr_r2/ml1 四臂 + **device 臂终态树回归**(`--cut-source device` P2 生产 dispatch,cull 会话驻留表 9,360,844B / 每帧 params 256B 上传 + 决策码 492,676B 回读)—— `fc.judge` incr==full ∧ 跨进程双跑位级 16f / `fc.judge_dev` **dev==host 位级** / `fc.cross_g38` **跨 G38 t3_incr 锚 MATCH**;bridge copy incr 均 0.58ms / build 均 20.1-20.4ms 与 G38/G39 同向(measured 登记不设线)。
  - fif 段(锁内 15:54:01-15:54:03):probe selftest + rebuild/refit 双臂 `gates` 全 true、`trimmed_mean` **44,544B**(== 预算门条目 `g31.fif_dyn.slot_as_group_mem_bytes` 在案 measured;与 T3 生产规模 240-361MB 登记面差 ~2,700×,「分口径不混」自证)+ `calibrate_fif_budget.py --check` OK。
  - guards 段 7/7:check_schemas PASS(**含 verify_stats v2 补丁后的 schema**)/ budget_eval **330 pass 0 skip**(normal mode)/ gpu_device_lock selftest / encode_parity / texture_sampling / vendor_license / blocked_probes。
  - 四份门 evidence:`evidence/g31_profiling_20260902T065651Z.json` / `g31_wp_hlod_20260902T072425Z.json` / `g36_geo_composition_gate_20260902T074732Z.json` / `g31_restir_wiring_20260902T074819Z.json`(UTC 文件名 = 本地 14:56 / 15:24 / 15:47 / 15:48)。
  - 二进制时序登记(如实):wp_hlod 门自带 cargo 步于 15:11:41 重建了 `g31_window_present.exe`(sha16 `66e94de5…`),与 B3 run3 的 `cc39f6dc…` 不同——差异归构建非位级可重现,渲染语义以 digest 门为准(G38「release+target-night 双二进制同值」同律)。源面核对:kernel / frame_cut_arm / frame_cut_probe / lane_body 四件 mtime ≤ 09-02 08:46(run3 前即定形);`g31_window_present.rs` mtime 显示 17:12:22(会话收尾时的同内容重存——`git diff --numstat` +895/−19 与 T1 REPORT §一/§8.8 登记值逐件同值,两级判据四处成套常量〔`G31_LRS_ULP_BOUND`/`G31_LRS_ATTRIB_CAP`/phase `0.62`/`y_unattributed`〕在树实测)。Wave 4 soak 不复用上述任一 exe,而以 17:21:10 终态树新建件(sha16 `7c77870a…`)起跑并先行双锚断言(见下)。
- [x] **soak PASS ≥1800s**(`g40_soak.py`,17:21:37-17:53:49,rc=0,`soak/G40_SOAK.json` + `soak_ev/`):wall **1930.6s** / **6 迭代**;前置双锚断言 all-off==`55e4a92d` + full19==`a5521e47` MATCH(终态树 17:21 新建 exe,与 W0/run1-2/run3 三代二进制同值);32f 迭代锚钉死 `a8204b3b`(G38 在案值非自举)**6/6 位级恒值**;it4 Stage A 探针(bench 160f/warmup10 bistro/t100/tsr)MATCH(it9 未及:6 迭代即越 1800s 墙钟线,G39 同形,如实登记非缺陷);全程 **VUID=0,fails=0**。
  - **帧时记账如实(不进全绿叙述,亦不判红——verdict 字面不消费该字段)**:`frame_ms_max` **12.393 > 11.111 预算**(`frame_ms_within_budget: false`);逐跑 real_render_frame_ms = pre_full19 11.560 / it1-6 12.393 / 11.514 / 11.662 / 10.953 / 11.801 / 12.106。定性 = **机态而非渲染面**:①digest 8/8 全程位级恒值,渲染语义零漂;②同一语义面本役在案对照 —— W0 n4 11.125 / B3 run3 n4 10.824 / B3 ladder `k12_off` 9.996(render_wall p50 9.785 进预算)/ G39 soak 前置 10.476 & max 10.798,同锚 digest 下帧时散布 9.8-12.4ms,量级即机态噪声(G38「同 digest 档帧时差 1.3ms 机态噪声」同律);③soak 口径 = 32f/warmup2 短跑(GPU 自 P8 空闲态爬坡 + 每次冷启动 ~2min 加载后立即计时)+ 宿主并发桌面负载(实测采样期间 Weixin/Edge WebView/L-Connect/Razer 等常驻进程在跑),两者本役未做控制变量;④阶梯口径(render_wall p50,96f)仍为帧时验收的法定口径(k12_off 9.785 / k26_on 7.841 进预算),soak 帧时为记账面。**第一旋钮** = 后续役 soak 加 `--warmup` 或帧数拉长以摊薄爬坡(口径变更须登记,不与 G38/G39 数字互比)。
  - 二进制/SPV 登记:soak exe = 17:21:10 终态树件(sha16 `7c77870a…`,2,702,336B;kernel/SPV 零改动,restir SPV sha16 `8ac52dc4d6f78cb1` 维持);bench exe = 10:03:14 件(sha16 `06b19b38…`,T3 终态树)。

## 收官登记(2026-09-02)

- 四任务全收口:T1 ReSTIR 画质补窗三件合一(B3 PASS;26 簇交付档 on 7.841ms 进预算 +3.27 余量;**画质收益为负 2.1-2.7× 与 T1b 边缘改善 ≈0 两条负面如实登记,方差税继承留窗**)/ T2 device cut P2 生产 dispatch(B1 六判 PASS,#77 P2 收口;cull dispatch GPU 0.113ms,ml1×device 16.095ms 落预期带不冒充进预算;P3 留窗)/ T3 AS 副本内存 evidence 补登(B2 PASS,G39 D-7 前半消解;生产规模 240.6/360.9MB 登记面与 probe 口径不混)/ T4 bridge_ext×FIF 评估(纯文档,建议机制 GO / 开窗条件 DEFER,**判档段留 owner 回填**)。
- **零重锚预期兑现**:all-off / full19 / bench / Stage A / soak 32f / 5540ecae 窗口 frame_cut 臂 / G38 t3_incr probe 序列全程未触,收役多重复证 MATCH(W0 / B1-B2 / B3 三跑 / 收役门 / soak 前置);on 臂无锚如实登记(digest 换代,确定性门 = 双跑位级)。
- 首红裁决一次(B3 run2 verify,判读器 f32 边界口径过严,产物零缺陷;修正 = 判据两级化收紧非放宽 + 红臂空转修复,五段式登记 `t1_restir2/REPORT.md` §8.2);编排面中断两次(run1 stdout OSError / run3 judge 段宿主进程消失)均非 GPU 面,处置如实留档。
- 文档:四任务 REPORT/EVAL 在档 + 侦察交接单三份 + `HANDOVER.md`(锚表/门台账/交付面台账/留窗 16 条/后续窗口 5 条)+ TODO 表 v1.2.3 修订行;t1_restir2 ab 逐帧 p.raw ~2.0GB gitignore 排除(G38/G39 同律)。
- **不 git commit,归 owner**;工作树 = G40 自有面(修改 8 + 新件)+ 入役即在树的 G39 两件判档登记改动(owner 治理窗产物,一并择入),**无兄弟任务在途面**(porcelain 实测)。
