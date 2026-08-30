# 交接清单归集（HANDOVER）— 画质战役 2026-08-28 收官

> 战役六相（A1/A2/A2b/B/C/D）+ E1 合流全部验收在案（CAMPAIGN_LOG.md + 各相 ACCEPTANCE_SUMMARY.json + e_final/E_ACCEPTANCE_SUMMARY.json）。本清单归集全部**跨出战役边界的遗留项**：他会话/主线认领面、缺陷登记、留窗优化。每项含出处与建议处置。

## A. 他区域 bug（战役纪律禁改,登记待主线修）

1. **g34 三 kernel fx/fy 双线性同源 bug**（Phase B 侦察）：`g34_*` 三 kernel 双线性底行 G/B 通道误用 fy 做水平混合（同源传播自 g31_texture_gi.rx L223/L226,该处已修）。修法 = 逐处 fy→fx（B 相在自有面 5 处修复的同款）。位置线索：L288/291 等（B 相报告在案）。**风险：g34 全特性组合门的贴图对拍恒绿假象**（host 镜像同错时对拍互相抵消——B 相曾因此恒绿,修须 host+device 同步改）。
2. **母版 TSR `min_alpha` 地板构造性不可达**（Phase D）：`kernel/tsr.rs` 母版 α = 0.1·(1−0.5·score) ∈ [0.05,0.1]，min_alpha=0.04 地板永不生效。D 相走 tsr_params[19] 稳态 alpha 档绕行；母版冻结未触。主线若修须过冻结面协议。
3. **rurixc「if 包 while」codegen 缺陷**（Phase A1）：if 块内 while 循环生成非法/错误 SPIR-V，A1 kernel 以 branchless gate 绕行。编译器面修复 + 回归样例登记。

## B. CI/门面同步项

4. **`ci/g31_texture_sampling_smoke.py` 判读器同步**（Phase B）：现编面按旧 tex 形态（探针步幅 3、非 heap）判读；战役后形态 = texel heap 单 SSBO + 探针步幅 4 + v2 SPV（`g31_texture_gi_v2.spv`/`g31_texture_probe_v2.spv`）。同步前该门对新形态不可用；旧 tex 臂锚 `6fab598c` 已作废。
5. **`encode_parity_probe.py` 转正 CI 门候选**（Phase A2b）：device-vs-host encode 对拍（2,073,600 px、exact 99.9891%、p100=1 LSB、>1LSB=0）。夜巡只验确定性 digest 未验 host parity——ACES 转置 bug 即此漏网通道；建议转正为 encode 改动硬门。件在 `artifacts/day_0828/a2b_aces_fix/encode_parity_probe.py`。
6. **RD-045 P02 腿锚 `060e69a8` 处置**：见 DEFAULT_FLIP_PLAN.md §1.1（经旧二进制+共享 SPV;收编时重收割并改 `ci/g31_blocked_probes_smoke.py` L63 字面）。
7. **gpu_device_lock 解锁段偶发 PermissionError + stale holder**（Phase A2 观察,pid 6028 事例）：解锁写回竞态类；建议锁文件写回失败重试 + stale holder 超时回收。基建观察面,战役全程未复发但在案。

## C. 已知限制/缺陷登记（质量档语义内,留窗优化）

8. **R2 f32 frame_idx 精度**（Phase C）：GI2 帧旋转 u=fract(px·a1+py·a2+n·a1) 用 f32 frame_idx,>100k 帧粒度退化（soak 30 分钟 @~120fps ≈ 21.6 万帧已进入区间）。修法留窗 = 拆和（n·a1 分整数/小数段 host 侧 fract 后再入 kernel）。E1 soak 用 32f 短迭代不触发。
9. **Karis HDR 尖峰有偏**（Phase D）：反亮度加权混合令收敛值 HDR 尖峰偏暗（dolly f0240 远小灯略暗小,灯不消失）。质量档语义内如实登记；无偏改法（能量补偿项）留窗。
10. **地板尾像素噪 −18% 未达 30% 目标**（Phase D）：相关性低频 jitter 响应分量,EMA 档原理性不可滤——需采样密度/jitter 感知滤波面。
11. **GI2 反弹无 quad NEE**（Phase C）：反弹点仅单点光随机 NEE + emission 直取,44k quad 灯片无 NEE 通道（间接光对灯片贡献靠命中直取,方差高）。quad NEE 缺口留窗。
12. **--svt × texel heap fail-closed 互斥**（Phase B）：SVT 页表假设 2048 固定网格与 heap 寻址不同构,深修留窗;现为显式拒跑。
13. **12 点光多重硬影**（Phase A3）：lamp 提取点光近似的固有限制,真解 = 面光/PCSS 留窗;大合并簇 r=3.11m 遮蔽豁免球偏大（本视角无伪影）;--lamp-contrib >0 档未实测。
14. **tsrq 邻域 clamp K 档未实测**（Phase D）：K=0 评估默认,后备旋钮。
15. **A2 reduce/state 两 pass 0.11–0.19ms**（超 <0.1ms 期望）：单 workgroup 结构代价,两级归约优化留窗。
16. **presentation_night.png zlib 流损坏**（A2b 旁支）：--export-png 写出面独立缺陷,不追。
17. **AE resize 复位 ~12 帧半衰 / α=0.02 收敛 ~50 帧**：resize/场景切换后短暂适应过程,协议内预期行为（E1 风暴复验在案）。

## D. E1 新增治理教训

18. **窗口纹理合流臂 presented 锚 = 二进制绑定锚**（E1 归因定案,e2_reanchor_registry.json）：`--textures×--smooth-normals` 及上叠组合的 presented digest 对宿主二进制重建敏感（ULP×TSR/AE 反馈放大;C 相 d89848b9 事件在 D 终态重建原样复现,E1 编辑在位/摘除两个二进制位级同值自证无罪）。**律：重建后组合臂锚一律先复验再消费;可跨重建对锚面仅 all-off + bench 全系。** 细化：E1 期间三个二进制形态（E1 编辑/de-E1 归因/风暴互斥解除）全部稳定复现 9e5f6300/d89848b9——扰动为 D 终态重建 delta 的一次性效应,非「任意重建必漂」;但纪律仍按最保守形态执行。
21. **storm×textures 组合已接线验收**（E1）：`--window-storm/--storm-soak/--fault-probe` 与 --textures 互斥解除（era 重建走完整四形态变体描述组重建）,--quality full × 风暴 3 次真 resize 干净退出在案;--svt × 风暴由隐式转显式 fail-closed（流送状态 × era 重建未验收,留窗）。
19. **Stage A vendor→tsr 测序脆弱面**（夜巡教训,E1 沿用）：同批 vendor（dlss/fsr）格之后跑 tsr 格 rc=1;18 格锚检脚本固定「6 tsr 隔离先行 + 12 vendor 批跑」测序（e3_stagea18.py 在案可复用）。
20. **RURIX_REQUIRE_REAL=1 必配 RURIX_VK_VALIDATION=1**（A3 教训,E1 全程执行）。

## E. 战役产出索引（新会话快速上手）

- 预设：`--quality off|full`（窗口九臂/bench 质量腿,解析层展开,fail-closed 冲突;RURIX_G18_AMBIENT 缺席自注入 0.004,显式 env 优先）。**Phase F 追加：窗口 full = 十臂（+--emissive-tex）,见 §F。**
- 终态锚总表：`e_final/E_ACCEPTANCE_SUMMARY.json`;重锚谱系:`e_final/e2_reanchor_registry.json`;默认翻转:`e_final/DEFAULT_FLIP_PLAN.md`。
- 对照图：`e_final/hero_campaign_before_after.png` + 四特写 `closeup_*.png`。
- 工具可复用：`e_final/e2_equivalence.py`（等价批跑）/`e3_stagea18.py`（18 格锚检）/`e4_storm.py`/`e5_soak.py`/`e6_hero.py`;`a2b_aces_fix/encode_parity_probe.py`;`night_0828/regression_probe.py`（6 格快检,锚为 Stage A 冻结系跨重建可用）。

## F. Phase F 追加（灯具 emissive 贴图臂,2026-08-28 晚）

22. **新臂登记：`--emissive-tex off|on`（+`--emissive-dir`,默认 off）**：生产质量 kernel emission 逐材质均值 Le → 逐像素 emissive 贴图采样（能量守恒标定 scale = 契约 Le/贴图线性均值——本场景四材质 scale ≡ 1.0 精确,契约 Le 即由贴图均值派生）。修复灯具整体全白：吊灯罩显编织纹质感/吊扇叶显红木/灯泡与灯笼玻璃罩保持亮。须随 `--textures on && --smooth-normals on`（fail-closed）;4 张 PNG 烘焙件走 `f_emissive/bake_emissive.py` 侧车（仓内无 PNG 解码器）,缺件拒跑。验收全绿：`f_emissive/ACCEPTANCE_SUMMARY.json`（F4 8 步 + F5 预设并入 + 风暴 + soak）。
23. **--quality full 语义变更 + 锚谱系**：窗口 full 展开 11→12 旗标（+--emissive-tex）⇒ **旧九臂 full 锚 `9e5f6300` 作废**（em 并入为语义变更,非漂移）;新十臂 full 锚 = `78113d56c6ed…`（F4/F5 跨两个二进制 ×5 跑 + soak 位级恒值,`f_emissive/F5_ANCHOR.json`）。九臂形态仍可显式旗标复现（== 9e5f6300,F4 nine_explicit 复验在案）。bench full 预设不变（bench 无本臂）。
24. **SPV 字节隔离三件套**：em off 各臂恒载既有锚定字节（`g31_texture_nrm_gi.spv` fd22cb19…/`*_gi2.spv` 75d08aec… 两件 0-byte 复核在案）;em on 独载 `g31_texture_nrm_gi_em.spv`（bdd23a3a…,gi2 on/off 都用 em 工件——GI2 段 params[51] 门控在内）。源码-锚字节 divergence 谱系随 A2b/C 相同律登记。
25. **texel heap 扩容形态**：em on 时 heap 槽 70→74（emissive 槽 70..73）,头表 910→962 项全 heap 重排布（既有偏移 +52）,+22.4 MB（cap-1024 起级律同 DDS 槽）;triem 侧表 1 f32/tri（4.2 MB,em_tris=44,024）;B4 探针双臂自动覆盖 74 槽（SSBO p100=0 位级 + sampler ≤1LSB,emissive 槽过同一硬门）。
26. **已知限制（Phase F 增量）**：GI2 反弹点 emission 仍 mats 均值直读（逐像素反弹采样留窗）;emissive 采样无条件执行 + branchless select（非灯具像素亦付 4 fetch,帧时增量实测 ≈0）;mat 59 壁灯契约相机 0 顶点在框（视觉走探针替补,Phase B curtainB1 同律）。
27. **构建基建教训：CARGO_TARGET_DIR 会话丢失面**：后台化命令令后续 shell 丢 env ⇒ cargo 静默落 `target/`（默认）而 `target-night/` 旧 exe 被误当新鲜——F5 首跑冒烟因此假红。律：**每条构建命令内联 `$env:CARGO_TARGET_DIR`,构建后核验 exe mtime**。（另:g34_full_lane 编译红 = Phase B 既有 heap 前形态 API divergence,本相未触;g35_particle_lane 编译绿复核。→ §F.28 F6 已修）
28a. **F7 emissive γ 对比度重映射定档（小灯全白微调,2026-08-29 晨）**：吧台小吊灯（灯笼 mat 38）F 相后仍全白——根因非 mip（诊断件 `f_emissive/small_lamp_probe.py`：L0≈L4）,是其 emissive 贴图玻璃罩区 0.05–0.16 vs 显示链总增益 ×52.8（>0.019 全裁白）。修 = 烘焙期 γ 重映射（linear 域 tex^γ,`bake_emissive.py --gamma`,均值 manifest 重标定 ⇒ 可见均值仍==契约 Le;投光面解耦零影响;γ=1 位级恒等旧烘焙）。**定档 γ=2.5**：小灯饱和白 77-79%→37-38%（阶梯图 `png/g_ladder_bells.png`）,灯泡仍白;**运行时零代码差,性能代价=0**（7.35↔7.58ms run 噪声带）。**锚谱系：full 十臂 78113d56（γ1 资产）→ `de342586`（γ2.5 资产,双跑位级 ×2,g25_96_ev{,2}.json）**——γ 是资产字节参数,重烘即重锚,soak/风暴为代码面验证不随资产重跑（架构隔离:烘焙件仅 em 臂加载,all-off/bench 零影响）。大灯罩区 82→79% 仅微降（其亮板 0.65+ 本意常亮,如实登记）。
29. **F6 双形态回正（Phase B in-place 改形违规如实登记 + 修复,2026-08-28/29）**：Phase B 在共享 include 体 `g14_3_lane_body.rs` **就地改形**共享纹理装配 API（grid 图集→texel heap/tritex 步幅 1→2/探针 3→4 元组/`g31_tex_host_sample` 6→7 参/`g31_tex_probes` usize→&[slots]/`G31TexSlot`·`G31TexAssets` 扩字段/`G31_TEX_N_MAPPED` 12→70/`g31_dds_decode_rgba8` 除名）⇒ 他会话已提交的 `g34_full_lane` 编译红 4 处 + 运行时读错形态（其 kernel `g34_unified_gi.rx` 按原 grid/stride-1 冻结）——**加性纪律违规,如实登记**。修 = **双形态并存**：原形态原名原签名从 HEAD 逐字恢复（机核 `f_emissive/f6_verbatim_check.json` 20/20 位级;含 g34 冻结 kernel 同源的 G/B 底行 fy 原式——§A.1 同错抵消面维持,不越权代修）;heap 形态另名 `*_heap`/`*_mip`（`g31_tex_load_heap`/`g31_tex_probes_mip`/`g31_tex_host_sample_mip`/`g31_tex_probe_{device,evaluate}_mip`/`g31_tex_sampler_leg_mip`/`geo_patch_proxy_tritex_heap`/`G31TexSlotHeap`/`G31TexAssetsHeap`/`G31_TEX_N_MAPPED_HEAP`）;我们的调用点（window_present 9 处 + lane_body 内链 + SVT/emissive 臂随 heap struct）全切 heap 命名;g34 三文件 + g34 系 kernel 零字节。**编译器全 bin 绿为界**（--bins rc=0 首轮,4 exe mtime 核验）;我们的锚零漂移复验（all-off 8f 55e4a92d / bench 160f c1d28ad7 / full 32f 78113d56,VUID=0）;g34 过锁真跑健康证明（--full orbit 74f rc=0 + VUID=0 ×2,digest/render_digest/digest_seq 84 帧双跑位级 4a06301c/f7deccc1;f39e9808 系在案锚属 night_baseline EXR 逐帧面不可比对,按双跑一致性登记）。件：`f_emissive/F6_SUMMARY.json`（改名映射表/编译矩阵/锚复验表/g34 探针）+ `F6_RUNS.json`。**律：战役自有文件 ≠ 自有符号——共享 include 体内的符号改形即跨会话破坏,一律双形态加性。**
