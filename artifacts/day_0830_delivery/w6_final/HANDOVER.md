# G37 商业化交付收官战役 HANDOVER(day_0830_delivery)

> 战役窗口 2026-08-29 23:1x ~ 2026-08-30(通宵);目标 = 剩余零散模块接线 + 已知问题可修面清零 + G36 五留窗/异步深水判档 + 默认翻转(获批执行)+ 许可义务闭合 + SDK bundle 重打包。
> 编排:主agent + 23 个子agent(两层,含并行侦察/实现/合入/判读),GPU 真跑全程锁内串行。
> 事实源:CAMPAIGN_LOG.md(逐波)、W4_ANCHORS.json(锚表)、各 w*/REPORT.md(逐件)。

## A. 战果总览

- **默认翻转已执行**:`g31_window_present --quality` 缺省 off→**full 十九臂**(交付形态);off 升显式回退档;CI 调用面 18 点补扫;bench Stage A 面零动作自证。
- **七组新臂接线**(全部 off==锚 + on 双跑位级 + VUID=0):`--transparency`(玻璃 ray 穿透真解)/`--lut`(第 4 级色彩分级,#79 收口)/PSO 变体账本(#82/#113)/`--visbuffer` 证据臂(#74/#111)/`--gi2-ris`+`--gi2-nee`(#6 修复路径)/`--cluster-per-frame-cut`(逐帧 cut→AS 竞技场)/FG×full 两点式组合。
- **G36 五留窗全部处置**:FIF×动态 GO(每槽 AS 副本判档件 rebuild+refit 双 PASS,RFC-0030 修订草案)/FG 组合 GO(验收 PASS)/HZB×蒙皮 GO(合并面 ACCEPT PASS 七 facts)/#96 crate 面完成(45/45,消费面分界登记)/逐帧 cut→AS GO(refit 竞技场,probe 全绿)。
- **异步三件套 M59 判档 = 维持 no-go + 新鲜 measured**:digest 等价硬前置全过(机制正确性证明),重叠率 48.54%<50% 阈值;机制件(timeline 提交器/合法化层/validator/compute-only 探测)入库为基建。
- **已知问题修复十件**:G12 PT 卫兵陈旧(CI 必红→绿)/gpu_device_lock 三处健壮性/em+AE override 错位/11 断言 debug→assert/rurixc「if 包 while」codegen(526+108 测试绿,90/90 生产 kernel 零漂移)/slot14 法线损坏件(v2 烘焙)/g34 fx-fy 六处+host 镜像+探针 SPV 三面同源修/encode 共享件收编 v2/parity 硬门转正(防 ACES 复发红臂)/纹理判读器 heap 同步(77 臂 selftest)。
- **商业化面**:GAP-01~03 许可义务闭合翻绿(门 7/7 PASS)/SDK bundle 候选 sdk-1.1.0(24 组件 2.26MiB,SBOM 双视图,install 四级校验;签名如实降级登记)/docs 5 篇刷新+PRODUCT_NOTES 新增/release notes/RD-036 判档维持 open。
- **判档如实登记二件**:VSM 页管线 no-go/maintain-defer(ray 车道无 shadow map 生成成本可摊销;判档件 g31_vsm_device_probe 三腿 GPU PASS 补 dirty_depth 轴消费缺口)/K 档关死(tsrq clamp K∈[1.5,3] 旋钮近惰性,降噪投资转第 1 级)。

## B. 锚表(W4 整批重收割终态,W4_ANCHORS.json 为事实源)

| 面 | 锚 | 口径 |
|---|---|---|
| 窗口 all-off | `55e4a92d…`(不变) | 8f/warmup2,--quality off 显式回退档;跨重建稳定 |
| **窗口 full 十九臂(现行交付默认)** | **`a5521e4708a814e364fd3bf95b18f0ab69b6646efd039246e8686533c30e4fb1`**(G38 法线 v2 重锚) | 96f/warmup2 缺省参数;谱系 5db2e7d7(十六臂)→ 7636f72f(十九臂+翻转)→ 本锚(法线 v2 消费切换,G38_ANCHORS.json 在案,双跑位级) |
| bench 默认 160f | `c1d28ad7…`(永不动) | 复验 MATCH |
| Stage A 18 格 | 全 18 格 MATCH(W0) | g14_3_stage_a_digest_anchor.json 不变 |
| RD-045 P02 腿 | `066395b0…`(G38 法线 v2 重锚,L68 已回写,门复跑 PASS;谱系 060e69a8→ef2b5b19→本锚) | orbit 64+10 缺省(=full);release/target-night 双二进制同值 |
| transparency 单开 | `af1f7264…` | --quality off + snrm+tex+transp 96f |
| RIS/NEE 单开 | `851a61ba…` | --quality off + snrm+tex+gi2+ris+nee 96f |
| LUT neutral/warm | `7b6856df…`/`c6cd1152…`(neutral == off 位级 = 恒等 LUT 精度证明) | --quality off + lut 32f |
| tex 臂(heap) | `ac2e5ff5…`(登记面;判读器"不消费固定锚"为长期形态) | --quality off + textures v2 SPV orbit 64+10 |

## C. 关键工件指纹

- 新 SPV:`g31_realism_transp.spv` 35983d0f(272,032B)/`g31_realism_ris.spv` 622a1c33(315,048B)/`g31_display_encode_lut.spv` 9087b743/`g34_unified_primary_skin.spv` 7d3ae216——全部 spirv-val 绿;母版/既有冻结件 sha 前后 ALL SAME 自证在各 REPORT。
- encode 共享件 `.tmp/g14_gates/m_c/g31_display_encode.spv`:43b0c255→e7291c79(v2 收编,.pre_g37.bak 备份);g34/g35/g36 消费面门全绿复跑。
- B4 探针原形态 `.tmp/g31_gates/texture/g31_texture_probe.spv`:8B272122→753C4E83(fx/fy 同源修,sanity = 未修源复编==部署件位级)。
- 法线烘焙 v2:`artifacts/day_0829_realism/a4_normalmap/baked_normals_bin_v2/`(69 张与 v1 逐字节相等 + slot14 平坦件 77bc6c00);**G38 Wave3 已切默认消费 v2**(g31_window_present.rs normal_dir 字面,语义变更即重锚已执行——full19 新锚 a5521e47/RD-045 新锚 066395b0,day_0829 历史 nrm 单开锚 dca78cbe 谱系作废不回写;G38 CAMPAIGN_LOG 在案)。
- SDK bundle:`dist/sdk_bundle/sdk-1.1.0/`(24 组件+发布七件+CANDIDATE_MANIFEST;重打脚本 ci/g37_sdk_bundle_repack.py)。

## D. 门台账(本役新增/复跑,全部 evidence 在 evidence/ 或战役目录)

新增门:`g31.g37w1.encode_parity`(PASS)/`g31.g37w5.dist` v2(候选面 selftest 67 臂)/`g37.wave3.hzb_skin`(ACCEPT PASS)。复跑绿:texsampling(heap)/blocked_probes(新锚)/g34×3/g36/vendor_license(closure)/VSM probe/fgcombo/FIF probe×2/frame_cut probe/async probe(no-go 判档)。

**W6 终态**:CPU 守卫 7/7 绿;GPU 门 10 绿(present/gameloop/dynscene/framegen/pipelining/hzb/slab/cluster_lod/wp_hlod/robustness,前四门 A 类化补 off 后——B 类补扫误判修正:evidence schema 闭集钉死 off 形态)+ svt **MUTEX_REGISTERED 登记态**(day_0828 heap 化起结构互斥,host 金标准腿全绿,深修锚 #33-36)+ profiling **轨迹面诚实红**(identity 恒等式判据机态敏感:三轮三形态越界且换腿、两腿各有全绿轮、历史绿值贴边、其余 facts 三轮恒绿;容差 [−0.10,2.00] 冻结面不放水,重标定/多轮中位鲁棒化归 budget 程序窗)。双 soak 绿:wave_a(off 面 10000 帧)+ **full19 @新默认**(7 迭代 2000.4s 位级恒值 + Stage A 探针 ×2;frame_ms_max 12.55 为 32f 短窗冷启动口径,96f 持续口径 9.75/10.59 ≤ 预算 W4 双证,产品口径以持续运行为准)。K 阶梯判读 = **K 档关死**(旋钮近惰性,tsrq_clamp_ladder.json;#30 闭,降噪投资转第 1 级)。

## E. 治理登记(须 owner 程序的项)

1. **RFC 草案二件待正式登记**:RXS-0239 多队列修订(建议并入 RFC-0019 修订记录;M59 no-go 下留档)/RFC-0030 §4.3 L2a(FIF×动态 opt-in,判档 GO,生产接线窗落地时走 RFC 程序)。
2. G36 正式立项程序(契约为事实登记面)与 G31~G35 契约 flip 维持留 owner(前役既有)。
3. `submit_pipelined_frame_slot_as` 复制适配体正式化时折叠单源(fif_dyn REPORT §6)。
4. bundle LICENSE 双件落位 bin/(install.rs 后缀律,非理想)归主线酌处;支撑生产签名走 release.yml 门控。
5. rowan 字面改判 conditional→cleared 须矩阵版本化修订(GAP 残余);sbom.rs Cargo.lock 自动展开归后续 src 批次。

## F. 已知限制留窗(day_0828/0829 继承项中本役未动的 + 新增)

- 继承未动(锚/条件未变):母版 TSR min_alpha 构造性不可达(绕行在位)/Karis HDR 尖峰有偏/地板尾噪原理性(采样密度面)/12 点光多重硬影(面光真解 #27/#105)/R2 f32 frame_idx >100k 帧精度/AE 两 pass 0.11-0.19ms/svt×heap 与 svt×风暴 fail-closed/emissive 无条件采样/mat59 壁灯 0 顶点方法限制/E2 低危观察三则/presentation_night.png zlib(导出工具)。
- 本役新增留窗:transparency 的 GI2/AO/反射 NEE 仍视玻璃不透明(如实登记)/反射命中点贴图采样(臂③×⑥合流)/#96 消费面(bake 双变体+运行时 gather)/frame_cut 全量 refit 27ms(生产实时需增量 refit/簇粒度降档)/fg×{hzb,slab,svt,lut} 维持互斥(结构证据)/PRODUCT_NOTES 未挂机核门/support_policy「C7 待建立」陈旧字面归门 owner。

## G. 复验命令形状(全 GPU 锁内,RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1;构建内联 CARGO_TARGET_DIR=H:\rurix\target-night)

```powershell
# 三锚+18格(W0 形态)
py -3 artifacts\day_0830_delivery\w0_baseline\w0_reverify.py   # 注:full16 步字面已被十九臂取代,以 w4 为准
# W4 全臂验收+锚表(主链 12 段)
py -3 artifacts\day_0830_delivery\w4_flip\w4_verify.py         # (s09 判读修正版见 w4_resume.py)
# all-off / full 十九臂
target-night\release\g31_window_present.exe --frames 8 --warmup 2 --hidden --quality off --evidence ev.json   # == 55e4a92d
target-night\release\g31_window_present.exe --frames 96 --warmup 2 --hidden --evidence ev.json                # == a5521e47(缺省=full;G38 法线 v2 重锚,旧值 7636f72f 作废)
# 门矩阵 + soak
py -3 artifacts\day_0830_delivery\w6_final\w6_gates.py; py -3 ci\g31_wave_a_soak.py
# bundle 重打(幂等)
py -3 ci\g37_sdk_bundle_repack.py
```

## H. 不可为项

见 IMPOSSIBLE_ITEMS.md(12 项硬件/外部锁死 + 本役判档 no-go 行:异步 M59/VSM 页管线/K 档,全部带重启锚字面)。

## I. 后续窗口建议(优先级序)

1. 法线 v2 消费切换 + 重锚(一次小重锚窗,slot14 修复真正生效)。
2. FIF×动态生产接线(判档 GO 已证,RFC-0030 L2a 正式登记 + 窗口/g14_3 消费 slot_as 入口 + 预算门)。
3. frame_cut 增量 refit(只更新 cut 差集顶点段)与簇粒度档,把 27ms 压进帧预算。
4. #96 消费面(bake 带 UV 双变体 + 运行时 gather 摘 tritex=−1 回退)。
5. RIS/NEE 的 A/B 画质定量(方差收缩数字面)与 lamp-k 提档评估(ReSTIR 开窗条件)。
6. 外部条件项按锚开窗:AMD 卡/HDR 显示/干净 VM/BistroExterior 资产/上游 LLVM。
