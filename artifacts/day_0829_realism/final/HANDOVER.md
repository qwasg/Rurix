# Day 0829 真实感收口战役 HANDOVER

## A. 战果总览

六个画质臂全部验收达标并入 `--quality full`(十臂 → **十六臂**),两篇接线评估交付。全程共享体 `g14_3_lane_body.rs` **0-byte**(六臂全部 host 面落窗口 bin 自有文件),母版 kernel 与 night_0828 三锚定 SPV 0-byte,bench/Stage A 全程零漂移,不碰他会话文件,未 git commit。

## B. 锚表(事实源)

- **窗口 all-off 8f**:`sha256:55e4a92d25be959a91d111140b88eb81e0c7c29fe80d1aeee641aa78d8a86288`(不变,跨重建稳定,本役复验 5+ 次全 MATCH)。
- **窗口 full 十六臂 96f(现行)**:`sha256:5db2e7d72e6b4f3c961d1acdd48d05c60df24e8803a26f4dfdb37665b79bf673`(96f/warmup2 presented digest,双跑位级 ×2,帧时 9.5-9.9ms;`final/F2_ANCHOR.json`)。
- **作废谱系**:9e5f6300(九臂)→ 78113d56(十臂 γ1)→ de342586(十臂 γ2.5)→ **5db2e7d7(十六臂)**。语义变更 = realism 六臂并入,de342586 作废。
- **bench 默认 160f**:`sha256:c1d28ad73783cc3c054ae0ce372b042a23d01df5491394cb38de7725e83b6c02`(永不动,本役复验全 MATCH)。
- **Stage A 18 格**:`milestones/g14/g14_3_stage_a_digest_anchor.json` 不变,本役两轮 18/18 MATCH。
- **各臂单开语义锚(96f,--quality full〔旧十臂展开〕+ 单臂)**:f0 `5fbafab8` / ao `d4d67354` / soft(2 样本)`e68f2561` / refl `c06ce663` / gitex `6c56d857` / nrm `dca78cbe`——**注意:并入后 --quality full 已含六臂,这些组合需全显式写法复现**。

## C. SPV/工件指纹清单

- 冻结不动(改后逐一 SAME 自证):`g31_texture_nrm_gi.spv` fd22cb19 / `*_gi2.spv` 75d08aec / `*_em.spv` bdd23a3a / m_c `g14_3_direct_gi.spv` 970e13b9;母版源 `kernels/g31_texture_nrm_gi.rx` 0-byte。
- realism 链(`.tmp/night_0829/spv/`,rurixc + spirv-val 双过;链式超集,换载"默认字面才换"取 on 集最高臂):`g31_realism_f0.spv`(b3dffbe6)→ `_ao` → `_soft` → `_refl` → `_gitex` → `_nrm`(17 buffer 最高链位)。源 = `kernels/g31_realism.rx`(母版 Phase F 后源码逐字 fork 演进,源码-下位工件 divergence 与 day_0828 同律如实登记)。
- 法线烘焙件:`artifacts/day_0829_realism/a4_normalmap/baked_normals_bin/`(70 × rgba8bin + manifest_bin.json;62 张 2048² 12 级全链 + 8 张 1² 占位)。烘焙链 `bake_normals.py`(BC5 标准表)+ `pack_normals_bin.py`(全 mip 零重采样)。

## D. 六臂机制与验收摘要(全绿)

1. **--metal-f0**(F0 修伤):装配期 albedo×(1−metallic) 令金属 F0 归零(F0=0 ⇒ Schlick 只剩掠射项,金属正视全黑)。修 = tri_base 3 f32/tri 未衰减 baseColor 侧表,kernel `al_f0 = tb·(tex_gate·raw + 1−tex_gate)`,diffuse 衰减面不动。A/B(无 AE):掩码 10.6%、高光 +13%、能量 +4.9%。红修 ×2(见 §F)。
2. **--rt-ao**(短程 RT AO):N 条余弦半球短射线(默认半径 0.5m/强度 0.85/样本 2),ao_vis 只乘 al·amb 常量环境光项。A/B:掩码 3.7% 接触区、变暗 −26%、全屏 −2.8%;帧时 +0.4ms。
3. **--soft-shadows**(点光软阴影,TODO #27 SMRT 简化形):点灯阴影射线 → 逐灯半径圆盘采样 N 条(R2+帧旋转+灯序号去相关,TSR 时域收敛半影);光度项仍灯心方向如实登记。A/B:掩码 7.0% 阴影边缘、能量守恒;帧时 2 样本 +3.9ms → **full 预设定档 1 样本**(F1:2 样本组合 12.96ms 超线,1 样本 9.54ms)。
4. **--rt-reflect**(光追反射):逐像素 1 条 GGX 半矢量重要性采样反射射线(rough>0.55 零射线),命中点 GI2 形着色,spec += w(rough)²·F(F0)·L_clamp(8)。**有偏近似如实登记**(单样本无 pdf 归一)。A/B:掩码 0.47%(bistro 多粗糙面)、反射面 +13%;帧时 +1.6ms。
5. **--normal-maps**(法线贴图接线):BC5 70/70 → heap 新槽 74..143(cap-1024 起级,+350MB,em append 同律);切线 = 装配期 UV 导数法(glTF 无 TANGENT);kernel TBN 扰动进全链。A/B:掩码 7.6%、能量守恒;**帧时 +0**;装配 +13s。
6. **--gi2-tex**(GI2 贴图反弹):反弹命中点贴图 albedo(重心 UV + 反弹程 lod)+ 逐像素 emission;mats 均值 while 门回退。A/B:掩码 0.08%(间接光占比小如实)、反弹色重分布;帧时 +1.0ms。

每臂验收环 = off==双锚 + on 双跑位级 + VUID=0 + 无 AE A/B 判据 + 帧时记账;证据各落 `a{1..6}_*/{ARM_RUNS,A1_RUNS}.json` + `ev/` + `png/`。

## E. 两篇接线评估结论(`evals/`)

- **EVAL_RESTIR.md**:不建议本窗直接接 G21/G28 ReSTIR 大件(单 pass megakernel 无储备缓冲结构、bistro quads=0 直接光已 12 点光聚类、M100 承接锚在案)。推荐两个 1 臂当量低垂果实:GI2 反弹点 RIS 选灯(M=4-8 候选,闭式无跨帧状态)与 44k 灯片 CDF 面光 NEE(修反弹 quad NEE 缺口本体)。第一步 = K 阶梯聚类 workload 证据实验(0.5 臂,零 kernel 改动)。
- **EVAL_DENOISE.md**:不建议本窗上 SVGF(90fps 预算不容、拆通道结构前提未备、无客户臂)。第 0 级 = tsrq clamp K 档阶梯实测(零代码,D 相后备旋钮定档);优先做方差源头收缩(反弹 RIS/CDF NEE)而非降噪器平台化。

## F. 红修登记(臂① ×2,额度用尽;其余臂零红修)

1. **红修 #1(判据面)**:首版 A/B 在 --quality full(含 AE)下失真——F0 能量增 → AE 反馈全屏压暗(diff 99.9%/mean 81→15)。修 = 无 AE 显式组合对照。**教训固化:所有臂 A/B 一律无 AE 对照**(tools/run_arm.py 定形)。
2. **红修 #2(绑定面)**:realism 形态 AE 下标族缺失——tri_base 尾挂 32/40 后 AE 三件须顺延(33..=35/41..=43),首版沿用 _EM 下标 ⇒ tri_base 被 AE reduce 覆写、真 params 被越界写(release 下 debug_assert 不生效未拦截);症状 = full+f0 与无 AE 组合 digest 位级相同(AE 链失效)。修 = _REAL/_NM 下标族 + 屏障计划 + builder/set_autoexp 双分支。

## G. 他区域既有缺口登记(不修,待主线批准)

1. **em+AE override 错位(day_0828 Phase F 遗留,de342586 锚内冻结)**:`set_autoexp` 选择块无 _EM 分支——em on 时逐帧 reduce override 传 TEXNRM(32,33) = (triem, 真 params),即十臂 full 的 AE 逐帧绑定错位(reduce 读 triem 当 params/写真 params 越界被 robustness 钳)。未见异常原因:确定性错乱 ⇒ digest 稳定 + 增益槽 enc_params[133] 未被有效写 ⇒ AE 实际近似恒等。**十六臂新锚 5db2e7d7 的 _REAL/_NM 分支已正确接线,AE 真生效**;旧十臂组合(全显式写法)仍带此缺口。修复 = 语义变更即重锚,归主线。
2. `g31_apply_autoexp` 的资源连号断言为 debug_assert,release 不生效(红修 #2 未被拦截的根因)——建议升 assert!(常数代价),归主线。
3. dlss vendor 格 vuid=1(Stage A 批跑,digest 全 MATCH)——vendor 层既有噪声,非本役引入,沿 day_0828 口径登记。
4. **G12 PT 门冻结卫兵陈旧**(收官后活性核验发现):`ci/g12_pt_prod_lib.py` 的 `m96_frozen_surface_unchanged` 以 G12.0 不可变 ref `5ae83aa7` 为 diff 基线,要求 `gi/path_trace.rs` 纯追加——但该文件此后已被三次主线合法提交演进(526d4c4e G12.2 / 5388c30f G12.3 / 058f8e68 G31+ 合流,距 HEAD 156 commit),卫兵必红。**PT 功能面完好**:selftest PASS + M158 门 device 腿 14/15(harness 全档 + no-mis/energy-bias 两 RED 臂 GPU 真跑,VUID=0),唯一红即此卫兵。修复 = 主线把卫兵基线 ref 升到 G12.3 后新不可变点(或改为"合法提交白名单"),归主线。

## H. 已知限制留窗(day_0828 全数继承 + 本役追加)

- 臂③反射为有偏加性近似(单样本 GGX 无 pdf 归一,能量 clamp+w 控);反射命中点用 mats 均值 albedo(非贴图采样)——与臂⑥机制可合流,留窗。
- 臂⑤光度项仍灯心方向(lr≪d 近似);面光/PCSS 真解留窗(TODO #27/#105 分档不变)。
- 臂② AO 挂在 amb 0.004 小环境光上,绝对幅度有限(判据以方向性收);amb 提档需全局重定曝光,留窗。
- 臂④法线 mip 链 XY box 平均未重归一化(kernel 归一化兜底);BC5 解码 8bit 量化 (byte−127)/127 近似,如实登记。
- 臂⑥反弹 emission 逐像素但反弹 NEE 仍单点光(quad NEE 缺口不变,EVAL_RESTIR §9.3 的 CDF NEE 是修复路径)。
- params[52] 帧旋转 f32 >100k 帧精度退化(GI2 同源,soak 32f 迭代口径不触)。
- **slot14 法线源件损坏**(烘焙侧车发现):`Paris_Table_cloth_01_Normal.dds` 整张常值 (53,53) ⇒ x=y≈−0.58、‖xy‖>1 非法法线(源资产占位/损坏件,BC5 解码忠实)。kernel 侧 `max(0,1−x²−y²)` + 归一化兜底不产 NaN,仅该桌布材质法线方向统一有偏;已在 5db2e7d7 锚内如实登记。修复留窗 = 烘焙侧把 ‖xy‖>1 常值图替换平坦 (127,127) 或 trinm 该材质置 −1——**任一修复即重锚**。
- **A/B raw 读取口径瑕疵登记**:presented raw 实为 `w:u32 + h:u32(8B 头)+ BGRA8`(tools/README.md 已核准),战役 A/B 脚本(a1_accept/run_arm)按 `len/4` 全体读入把头当 2 个假像素——on/off 同偏 diff 恒零,对判据影响 ~1e-6 量级(2/2073602),结论稳健不重跑;后续消费一律以 `tools/ab_metrics.py`(跳头版)为准。
- **玻璃隔断"两个三角形雾状区"(交互预览用户可见,消融定位收官后)**:吧台前方玻璃隔断(TransparentGlass.DoubleSided,slot00,130,792 tris)被无透明管线的渲染器按不透明渲染——淡灰 albedo + 光滑高光 + 薄面几何在 jitter/TSR 时域混合下与背景交替 ⇒ 呈半透明雾状楔形(一块玻璃板 = 2 三角形,屏幕中央"总是"可见)。**消融链证明与本役六臂无关**:关 refl/nrm/soft/tsrq 均不消除,**十臂基线(六新臂全关)同轨迹同在**(`final/png_triage/m_tenarm_full.png` vs `m_full16_full.png`)。资产级既有限制 = 透明材质管线缺失(alpha blend/排序或 ray 穿透),修复为独立立项;快速缓解(装配层跳过 Transparent 材质)= 语义变更即重锚,留窗待主线。玻璃类材质法线全 1×1 占位(§H slot 占位清单同源)为旁证。

## I. 复验命令形状(全部 GPU 锁内,RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1,构建内联 CARGO_TARGET_DIR=H:\rurix\target-night)

```powershell
# all-off 锚
target-night\release\g31_window_present.exe --frames 8 --warmup 2 --hidden --evidence ev.json   # == 55e4a92d...
# full 十六臂锚
target-night\release\g31_window_present.exe --frames 96 --warmup 2 --hidden --quality full --evidence ev.json   # == 5db2e7d7...
# 单臂全显式(例:仅 metal-f0,soft 等其余 realism 臂 off)
target-night\release\g31_window_present.exe --frames 96 --warmup 2 --hidden --smooth-normals on --ggx on --lamp-lights on --lamp-gain 4 --textures on --bloom on --dither on --auto-exposure on --tsr-quality on --gi2 on --gi2-clamp 0.01 --emissive-tex on --metal-f0 on --evidence ev.json
# 法线烘焙件重生成(K: 资产在位)
py -3 artifacts\day_0829_realism\a4_normalmap\bake_normals.py && py -3 artifacts\day_0829_realism\a4_normalmap\pack_normals_bin.py
# 三锚+18格一键复验
py -3 artifacts\day_0829_realism\final\f2_reanchor.py
```

## J. F3 风暴 + soak(全绿,`final/F3_SUMMARY.json`)

- **风暴**:--quality full(16 臂)--window-storm 3(dolly 30f):rc=0、resize_eras=1、exit_reason=frames_done、VUID=0——PASS。
- **soak**:1955.4s ≥ 1800s,9 迭代 32f 全位级稳定(首迭代自举 32f 口径锚,后续逐迭代 ==)+ VUID=0 全程;**帧时 9.54-10.70ms,峰值 10.70 ≤ 11.11ms(90fps 预算全程达标)**;Stage A 单格探针 ×2(it3/it6)全 MATCH。fails=0。

## K. 后续窗口建议(优先级序)

1. em+AE override 错位修复(§G.1)+ debug_assert 升级(§G.2)——修复即重锚,与主线协调窗口。
2. EVAL_RESTIR §9.3 两个 1 臂当量:GI2 反弹 RIS 选灯 + 灯片 CDF NEE(方差源头收缩,优先于降噪器)。
3. EVAL_DENOISE 第 0 级:tsrq clamp K 档阶梯实测(零代码定档)。
4. 反射命中点贴图采样(臂③×臂⑥机制合流)、soft 光度项面光化(#105 PCSS 方向)。
5. 默认翻转(day_0828 DEFAULT_FLIP_PLAN)如获批,realism 十六臂锚 5db2e7d7 为新翻转基线。
