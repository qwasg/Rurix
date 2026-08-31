# G38 五任务推进 CAMPAIGN LOG(day_0830_g38)

> 开役 2026-08-30 18:4x。目标:①法线 v2 消费切换+重锚 ②FIF×动态生产接线(#90:RFC-0030 L2a 正式登记+slot_as 生产路由+预算门) ③frame_cut 增量 refit(27ms→预算内或降档) ④#96 消费面(bake 双变体+运行时 gather+tritex 接真值) ⑤RIS/NEE 方差收缩 A/B 定量+lamp-k 提档(ReSTIR 前置)。
> 纪律沿 G37:GPU 真跑锁内 + RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1;构建内联 CARGO_TARGET_DIR=H:\rurix\target-night;既有行字面 0-byte 只追加;加性臂 = off==锚 + on 双跑位级 + VUID=0 + 无 AE A/B + 帧时记账;**动 full19 锚的语义变更(法线 v2 切换、lamp-k 若 GO 提默认档)合并进 Wave 3 一次整批重锚**;窗口 bin 编辑权串行移交(T5 → T2 → Wave3)。
> 入役基线:git HEAD `0e605c34`(G37 收官提交,树干净)/ all-off `55e4a92d` / full19 `7636f72f` / bench `c1d28ad7` / Stage A 18 格锚不动。

## Wave 1 代码面(四路并行)

- [ ] **T2 FIF×动态生产接线(#90)**:RFC-0030 §4.3 L2a 子行+§9.2 修订行正式登记(草案 w3_deep/fif_dyn/RFC_DRAFT_RFC0030_amendment.md 转正);g14_3_pipeline_perf 解除动态×FIF 强制 inflight=1,路由 submit_with_frame_update_slot_as;窗口 bin 动态路径同法;每槽 AS 副本内存预算门(schema+budget_eval 先例)。
- [ ] **T3 frame_cut 增量 refit**:BlasRefitUpdate 多 region 化(桥 copy 只搬差集脏槽,加性,既有单 region 调用者 0 改写);桥接段独立 timestamp 拆 copy/build(不动逐 pass 冻结口径);--min-level 簇粒度降档臂(截断 DAG,竞技场总三角减半);probe --refit-copy full|incr 对照。
- [ ] **T4 #96 消费面**:RXCS dump 补 UV(cook 前置缺口);g31_cluster_lod_bake 切 build_dag_attrs+RXGB sidecar;hlod bake 切 simplify_free_mesh_attrs+RXHL v2(6×f32 corner UV,v1 0-byte 版本加性);运行时 gather 填 tri UV 表,geo_patch_proxy_tritex 接真槽号替 −1(仅 g34 车道消费面)。
- [ ] **T5 RIS/NEE 定量准备**:lamp-k 聚类参数化(lane_body GRID_M=0.6 加性新函数+窗口散臂 flag,默认字面不动=不动锚);A/B 跑批脚本(四 ROI std_p95 口径复用 judge_kladder,无 AE 纪律 EXPLICIT_NOAE,窗口 raw 路)。

## Wave 2 GPU 锁内串行

- [x] **批次 1 主体全绿**(gpu_batch1.py 锁内 33min,batch1_log.jsonl 在案;20:15~20:48):
    - **T3**:B1-B5 全绿——incr==full 16 帧 digest 逐字节 + 跨进程双跑位级 + min-level 1 档自洽;**帧时分解 measured:桥 copy 4.348→0.022ms(多 region 收益),UPDATE build 地板 21.6ms(2.08M tri,主导项),min-level 1 档 build 8.78ms/exec 9.54ms ≤ 11.11ms 预算**——增量 copy 拿到但 build 全扫是 API 地板,90fps 进预算形态 = --min-level 1 降档档位,如实定档。B6/B7 首跑红 = 判读器口径错(B6 硬编码 s09 host 上传字节 75,139,596 当桥窗全量,实为 arena 74,973,708;B7 误用 headless-smoke 130 帧口径,W4 锚是 frames24/warmup2 evidence 口径),脚本已修,t3 段重跑在途。
    - **T2**:fif probe v2 收割双绿(gates 全 true + slot_as_mem 账在位);**预算回填 PASS:measured=44,544B threshold=66,816(×1.5 程序产),budget_eval 330 pass 0 skip**;**dyn 生产接线双硬门全绿:rebuild 与 refit 均 inflight 1/2/3 三臂逐帧 digest 逐字节相等 + 各臂双跑位级**(refit 实测纯函数,无需 L2a 降档登记——比预期更强)。#90 生产面 GO 实证。
    - **T4**:锚①② 绿(base == leafxfull_v1 == leafxfull_v2 逐帧 digest——v1 回归 + v2 极限臂零语义);锚③ 判读修正裁决(W4 s09 先例同形):v1_patched=395,235>0 旧语义 ✓ / v2 无 patched 行 = 0(L1936 `if patched>0` 才打印,实现语义确证)= **tritex −1 退役面兑现** ✓ / host 对拍双臂 p100 2.6e-4 / 4.1e-4 ≤ tol 7.9e-4 ✓;c 判据 bbox 方差归质量登记面不设通过线。
    - 附带处置:首版 B7 无 --evidence 时窗口 bin 写默认路径 evidence/g31_game_loop.json(字段不全,未跟踪副产品)→ 删除,check_schemas 恢复 PASS。
- [x] **批次 1 尾:t3 段重跑全绿 = BATCH1 PASS**(21:08;B6 修正后 OK;**B7 W4 口径命中锚:fc==base==5540ecae 全串**——T3/T4/T5 全部改动对既有 off 面 presented digest 零漂移铁证)。
- [x] **wp_hlod 门全量回归 PASS**(rc=0,evidence g31_wp_hlod_20260830T134007Z.json)——T4 bake 改造零回归(double-build 确定+全 Full digest 锚位级+三档单调+四 RED 臂)。
- [ ] g36 组合门:锁队列排批次 2 之后(在途)。
- [x] **批次 2 A/B 段完成**(ris_nee_ab.json):四臂(base f2eb4a3c / ris efa14e89 / nee 6790940c / both da176a4e)全双跑位级 + VUID=0;**方差收缩 measured(presented u8+TSR 后口径,dark ROI):nee p95 +2.43%/mean +5.94%,both +2.09%/+4.88%,ris 单开 −0.58%(噪声级)——verdict = nee/both marginal、ris worse(阈 10% 未达)**;口径诚实登记:dark ROI std 0.25~0.6 u8 LSB 贴量化地板 + TSR 时域滤波已吸收输入噪声差,scene-linear TSR 前口径不可得(bench 无 ris flag);**帧时反直觉收获:RIS/NEE 更快——base 10.30ms → ris 8.63(−1.67)/ nee 8.99(−1.31)/ both 9.65(−0.65)**,方差边际收益 + 负帧时代价组合如实登记。
- [x] **s1 基线原位复证:digest == 7636f72f 全串**(重建 exe 上 full19 锚零漂——Wave1 全改动对缺省面零影响的最终铁证;13 簇 kept 12 字面同 W4)。
- [x] **批次 2 全 PASS**(BATCH2_RC=0,97min):阶梯六档 measured 落 lamp_k_ladder.json——s1(0.6/12,13 簇 kept12)p50 10.80 margin+0.31 **digest==7636f72f 原位复证**;s2 证伪(0.6/24)clusters_total 恒 13 ✓(kept 12→13 = k 上限放开捡回弃簇,min(13,24)=13 语义正确,judge note「预期外」人工核查裁定非异常);s3(0.3/24)16 簇 p50 10.70 margin+0.41;s4(0.3/48)**与 s3 同 digest**(同 16 簇语义自洽)但 p50 12.01——**同语义帧时差 1.3ms 暴露机态噪声与预算余量同量级**;s5/s6(0.15,26 簇)p50 12.96/13.06 确定超线 −1.9ms。
- [x] **lamp-k 裁决 = 默认不提档(维持 12/0.6),verdict=go_candidate 贴线不稳健**:16 簇档(s3 +0.41 vs 同 digest s4 −0.90)跨预算线,不以噪声级余量动交付默认;26 簇档确定超线。**ReSTIR 前置条件登记(measured):逐盏直接光 16 簇贴线、26 簇超线——提灯数须 ReSTIR(#6/G21/G28 开窗证据链)**。散臂旋钮(RURIX_G31_LAMP_GRID_M + --lamp-k)与阶梯曲线为本窗交付。
- [x] g36 门首跑 = **锁等待超时红**(3600s 上限,批次 2 持锁超时——编排排队问题非门语义红,如实登记);重跑在途。

## Wave 3 整批重锚(23:0x 开工)

- [x] **重锚面裁决**:lamp-k 不提档 ⇒ 本批语义变更 = **仅法线 v2 消费切换**。受影响锚 = full19(7636f72f→新)+ RD-045 orbit(ef2b5b19→新,默认 full 面);不动锚(off 基,机器抽验)= all-off/bench/Stage A/transparency/lut×2/ris_nee 单开/tex 臂。
- [x] v2 字面量切换:g31_window_present.rs L7192 normal_dir → baked_normals_bin_v2(注释登记语义变更即重锚)。
- [x] 双树重建(target-night 1m36s + target/release 1m53s 双 RC=0)。
- [x] **收割链全绿**(g38_reanchor.py 锁内 34min + retry;G38_ANCHORS.json verdict=PASS):
    - 负控:n1 all-off == 55e4a92d ✓ / n2 bench == c1d28ad7 ✓ / n3 Stage A 探针(bistro/t100/tsr 格)== 锚 ✓——v2 切换对 off/bench/g14_3 面零影响机器证明。
    - **full19 新锚 = `a5521e4708a814e364fd3bf95b18f0ab69b6646efd039246e8686533c30e4fb1`**(96f/warmup2 双跑位级;谱系 5db2e7d7 → 7636f72f → 本锚〔法线 v2 消费〕)。
    - **RD-045 新锚 = `066395b0b6d877f546b7082560c093b4c325f39dae446abf3d59a8ad1023d56d`**(orbit 64+10;target-night 与 target/release 双二进制同值 + 各自双跑位级;谱系 060e69a8 → ef2b5b19 → 本锚)。
    - 不动抽验:ris_nee 单开 == 851a61ba ✓;transparency 单开首跑 FAIL = 判读器臂形多拼 --ggx(W4 s04 原形无 ggx),正确臂形补验 == af1f7264 全串 ✓(--retry-v2 程序化修正,v2_correction 在案)——**v2 切换只动 nrm 面的机器证明成立**。
- [x] **blocked_probes 门锚字面回写 + 双 PASS**:ci/g31_blocked_probes_smoke.py L68 ef2b5b19→066395b0(谱系注释同步);selftest PASS + 真跑 GATE PASS(rd045_digest device 腿 target/release v2 exe 命中新锚;evidence g31_blocked_probes_20260830T161741Z.json)。
- [x] **登记面同步**:w6_final/HANDOVER.md(锚表 full19/RD-045 行+工件行 v2 已切+复现命令注释)/ feature_matrix.md L142(G38 默认消费 v2)/ TODO 修订表 v1.2.1 行(全战役登记)/ G38_ANCHORS.json(W4 结构照抄+不动锚誊录+作废谱系);day_0829 历史锚(nrm 单开 dca78cbe 等)不回写,谱系作废在 v1.2.1 行与本 LOG 登记。

## 终验(00:1x~)

- [x] **CPU 守卫 7/7 全绿**:check_schemas PASS / budget_eval 330 pass 0 skip / gpu_device_lock selftest / encode_parity selftest / texture_sampling selftest / vendor_license selftest / blocked_probes selftest+真跑 GATE PASS。
- [x] **受影响门全量回归**:wp_hlod 门 PASS(T4 bake 面)+ g36 组合门 PASS(十 facts;首跑锁超时红为编排排队非门语义,重跑绿)+ blocked_probes PASS(锚回写面)——v2 影响面唯一消费门收口;其余 GPU 门全 --quality off 显式面(W4 QUALITY_OFF_SWEEP)不受 v2 影响。
- [x] **G38 soak PASS**(soak/W6_FULL_SOAK.json):新默认 v2 面 7 迭代 1893.1s ≥ 1800s 位级恒值 + Stage A 探针 MATCH + VUID=0;**frame_ms_max 10.993 ≤ 11.11 预算**(G37 W6 短窗口径 12.55 的冷启动越线本役未现,32f 短窗口径也进预算,如实登记)。

## 收官终态(00:52)

- **五任务全部交付**:①法线 v2 消费切换+整批重锚(full19 `a5521e47…` / RD-045 `066395b0…` 双二进制,负控+不动抽验全 MATCH,blocked_probes 门回写双 PASS)②FIF×动态 #90 收口(RFC-0030 v1.1 L2a 正式登记+dyn 臂 slot_as 生产接线双硬门逐字节等+预算门 measured 回填 330 pass;skin 批次 B 留窗)③frame_cut 增量 refit(copy 4.348→0.022ms+build 地板 21.6ms measured+--min-level 1 档 9.54ms 进 90fps 预算,incr==full 位级)④#96 消费面(三格式 v2+bake 双变体+runtime gather,tritex −1 退役 patched=0,wp_hlod+g36 门全量回归 PASS)⑤RIS/NEE 定量(四臂 A/B 方差数字+帧时 measured;lamp-k 阶梯 16 簇贴线/26 簇超线,默认不提档,ReSTIR 前置条件 measured 登记)。
- 门台账:CPU 守卫 7/7 + wp_hlod/g36/blocked_probes 三门 GATE PASS + soak PASS;GPU 批次 1/2 + 收割链全绿(三处判读器口径错均裁决修正,产物零缺陷)。
- 谱系登记:full19 = 5db2e7d7 → 7636f72f(作废)→ **a5521e47**;RD-045 = 060e69a8 → ef2b5b19(作废)→ **066395b0**;day_0829 nrm 面历史锚(dca78cbe 等)作废不回写。lamp-k 默认 12/GRID 0.6 维持(散臂旋钮 RURIX_G31_LAMP_GRID_M + --lamp-k 交付,提档须 ReSTIR)。
- 留窗登记:#90 skin 臂批次 B(WIRING_PLAN §1-A6)/ slot_as 单源折叠(REPORT §7-3)/ #96 锚③ c 判据 bbox 视觉指标(质量登记面)/ RIS/NEE scene-linear TSR 前口径方差(bench 无 ris flag)/ frame_cut host cut 3-11.5ms(#77 device cut)。不 git commit(与 G37 同律,归 owner)。

## Wave 3 整批重锚

- [ ] 法线 v2 切换:g31_window_present.rs L7192 目录字面量 baked_normals_bin → baked_normals_bin_v2(唯一代码改动);lamp-k 若 GO 同批提默认档。
- [ ] 收割:full19 新锚(96f/warmup2 双跑位级)替 7636f72f 谱系登记;负控 all-off 55e4a92d / bench c1d28ad7 / Stage A 18 格全 MATCH;RD-045 orbit ef2b5b19 重收割+blocked_probes 门字面回写(照 W4 s10 程序);tex 臂对账值刷新;day_0829 历史锚(nrm 单开 dca78cbe 等)不回写只登记作废。
- [ ] 登记面:W4_ANCHORS(新表)/w6_final/HANDOVER L22+L67/CAMPAIGN_LOG 谱系/TODO v1.2.x 行/feature_matrix L142。
- [ ] 终验:受影响门子集重跑(parity/texsampling/blocked_probes/相关 GPU 门)。

## 子agent编排登记

| 波 | 任务 | agent | 状态 |
|---|---|---|---|
| W1 | T3 frame_cut 增量 refit 实施 | f48342be | **完成**(桥 copy 多 region 走新入口 execute_with_frame_update_bridge_ext〔窗口 bin/fif_dyn 字面量构造点确认加字段必崩,冻结面零字节〕;--refit-copy incr\|full 两态 incr 默认,窗口臂经旧签名转发自动受益;降档 = cut 后父链提升映射 + 生产 verify_cut_coverage 复核,visible_cluster_set.rs 零改动;时戳 3 点追加区 fail-soft 拆 copy/build;check EXIT=0 warning 零新增,rt 115+render 583 测全过;GPU 验收 B0~B7 在 t3_framecut/REPORT.md §5) |
| W1 | T2 实施交接单(侦察续) | 07ca64f5 | 完成(9 项事实链) |
| W1 | T4 实施交接单(侦察续) | 5de62783 | 完成(9 节事实链) |
| W1 | T5 实施交接单(侦察续) | 3e8f9217 | 完成(10 项事实链) |
| W1 | T4 #96 消费面实施(独占 lane_body/rurix-asset/geom-build/g34 车道) | 9d4316b4 | **完成**(RXCS/RXCP/RXHL 三格式 v2 UV 加性,v1 路径 fc /b 逐字节不变自证〔既有 rxcp 2 字节漂移三方对拍裁定为 HEAD 已入库 QEM 重构所致,非本窗〕;--attrs 双构建字节相等;gather_tri_uv_attrs/geo_patch_proxy_tritex_v2 接 g34 两车道,v2 资产代理保留槽号,kernel 0 动;geom-build 46+asset 56 全绿、m90 golden 零漂、warning 零新增;GPU 三验收锚命令在 t4_attr96/REPORT.md §5,消费工件 .tmp/t4_attr96/ 就位) |
| W1 | T2 段1:RFC-0030 L2a 登记+预算门(独占 rfcs/probe/ci 预算面);段2 WIRING_PLAN 只设计 | cf118e05 | **段1完成**(RFC-0030 v1.1 条款+修订行+草案头状态+TODO#90+G14PLUS_RECORD §7/v1.2;预算门五件套:probe v2 evidence slot_as_mem 账+第7门/v2 schema 首注册〔v1 从未注册如实登记〕/_patch 幂等×2/budget estimated 占位/calibrate 标定脚本;check_schemas PASS+budget_eval 329pass+1skip+probe 单测5/5+selftest 5/5;段2 WIRING_PLAN.md 前后文本级计划在案,待 lane_body 释放) |
| W1 | T5 脚本面:A/B 跑批+阶梯+判读四脚本(纯 py,零 .rs) | ab432e1b | **完成**(py_compile 4/4 + selftest 4/4;EXPLICIT 无 AE 十九臂集抄定〔transparency 在集,lut/visbuffer 不在〕;批次 2 = 14 条 GPU 跑估 30-45min;阶梯 s3-s6 依赖 lamp-grid env 旋钮接线) |

> 编辑权台账(19:5x 更新):T3/T4 完成释放;**lamp-grid env 旋钮已由主agent接入 lane_body**(RURIX_G31_LAMP_GRID_M,缺席=0.6 字面零漂移,在位 parse 失败即 fail;extract_lamp_lights 定义+3 消费点);合流终态全树 check 双 RC=0(rt/asset/geom-build + render vendor-upscale --bins);lane_body+g14_3_pipeline_perf.rs 编辑权移交 T2 段2(cf118e05 在途);g31_window_present.rs 仍无人占用(留 Wave3 v2 字面量)。
> 侦察修正两则(接线设计已按此收敛):①窗口 bin 唯一动态 AS 消费方 = HZB 车道,逐帧 host 决策在环、已登记「FIF 天然不适用」——#90 生产接线真实面 = g14_3 dyn/skin 臂,窗口面如实登记不适用;②现 0.6m 聚类网格下 bistro 只产 13 簇,--lamp-k>13 无效——lamp-k 提档真旋钮 = 聚类网格 GRID_M(env RURIX_G31_LAMP_GRID_M 散臂,EVAL_RESTIR §9.4 字面「改参数不改算法」)。

## 战果记录

(按波次追加)
