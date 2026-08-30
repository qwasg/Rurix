# Day 0829 真实感收口战役日志

## 纪律(day_0828 全数继承 + 本役追加)

- 冻结面红线:默认路径 kernel + night_0828 三锚定 SPV(fd22cb19/75d08aec/bdd23a3a)+ 母版 `g31_texture_nrm_gi.rx` 全程 0-byte;六臂演进只发生在 fork `kernels/g31_realism.rx`,每臂编译到 `.tmp/night_0829/spv/g31_realism_*.spv` 独立工件(链式超集,换载"默认字面才换"取 on 集最高臂)。
- **共享体 0-byte(本役追加强形态)**:`g14_3_lane_body.rs` 全程不动——六臂全部 host 面(装配/pack/desc/旗标)落 `g31_window_present.rs` 窗口 bin 自有文件(include! 同 crate 可调共享体私有 fn)。
- 每臂验收环:off==锚(all-off 55e4a92d + full de342586)→ on 双跑位级 + VUID=0 → A/B 指标 + 视觉 → 帧时增量记账(90fps=11.11ms 预算,full 基线 7.51ms,余量 ≈3.6ms)→ 变差回退,单臂 ≤2 次红修。
- 构建隔离 target-night(每条命令内联 CARGO_TARGET_DIR + exe mtime 核验);GPU 真跑过 ci/gpu_device_lock;RURIX_REQUIRE_REAL=1 必配 RURIX_VK_VALIDATION=1。
- 绝不碰他会话文件(00_MASTER_INDEX/11_ROADMAP/milestones/g35、g36/15_EXTERNAL_ADOPTION_REGISTER/G31_PLUS_COMMERCIAL_RENDERER_TODO);不 git commit。

## Phase 0 — 开工复验(fails=0)

- all-off 8f == `55e4a92d` MATCH(6.25ms);bench 默认 160f == `c1d28ad7` MATCH;full 96f == `de342586` MATCH(**基线帧时 7.510ms**)。
- Stage A 18/18 MATCH(6 tsr 隔离单跑先行 + 12 vendor 批跑;dlss 格 vuid=1 为 vendor 层既有噪声,digest 全 MATCH 如实登记)。
- γ2.5 烘焙件在位 = 4 件 `.rgba8bin` + manifest(判据首版误写 `*.png` 已修正 rerun——HANDOVER「4 张 PNG」为口语,真实形状 rgba8bin)。
- 证据:`anchors/P0_SUMMARY.json`。

## params 扩面登记([55..72),G31_REAL_PARAMS_LEN=72)

- [55] 臂① metal-f0 门 / [56..60) 臂② rt-ao 门+半径+强度+样本 / [60..62) 臂⑤ soft-shadows 门+样本 / [62..65) 臂③ rt-reflect 门+rough 上限+clamp / [65..67) 臂④ normal-maps 门+强度 / [67] 臂⑥ gi2-tex 门 / [68..72) 预留恒 0。
- 任一 realism 臂 on 才扩容并写槽;全 off = pack 产物与既有 56 槽逐位同值。

## 臂① --metal-f0 金属 F0 修伤(**收绿**,红修 ×2 额度用尽)

- 根因坐实:装配期 per-tri albedo = (tex_mean×factor)×(1−metallic)(lane_body L1771-1789)、texmeta mod = factor×k_metal(L6212-6217)⇒ 金属 al_*≈0 ⇒ kernel F0 = mix(0.04, al, metal) 归零——注意 F0=0 时 Schlick 剩纯掠射项 (1−cosθ)^5,金属实症 =「只有掠射白边、正视全黑」。
- 修法:tri_base 3 f32/tri 未衰减 baseColor 侧表(tex 三角 = factor〔kernel 乘 ×mod 前原始采样 raw_*〕;常量三角 = tex_mean·factor;灯面/matless = [1,1,1] 零消费);kernel `al_f0 = tb·(tex_gate·raw + 1−tex_gate)`,F0 = mix(0.04, al_f0, metal),diffuse/环境光衰减面不动;门 off 地址钳 0 读 12B 零哑表。
- 接线:`--metal-f0 off|on`(须随 --ggx on && --textures on,fail-closed);SPV `g31_realism_f0.spv` = `b3dffbe6`(rurixc + spirv-val 双过);desc 尾挂 tri_base=32/40(em off 时 triem 绑 tri_count×(-1.0) 回退真表保持 kernel 签名序);params buffer 任一臂 on 扩 288B。
- 既有 SPV 指纹改后自证:fd22cb19/75d08aec/bdd23a3a/970e13b9 全 SAME。
- 首跑:off 双锚 MATCH + dump 侧车零扰(full 带 dump == de342586)+ on 双跑位级(87d7139f ×2)+ VUID=0 全绿;A/B 判据红。
- **红修 #1(判据面)**:--quality full 含 AE,F0 能量增 → AE 反馈全屏压暗 ⇒ 首版全屏 diff 判据失真(diff 99.9%/mean 81→15)。修 = A/B 换无 AE 显式组合(十臂去 --auto-exposure + env 环境光 0.004)。重跑绿:changed_frac 10.6%、掩码亮度 30.96→34.97(高光恢复 +13%)、全屏 mean 14.08→14.77(加性能量 +4.9%)。**教训固化:后续臂 A/B 一律无 AE 对照**(tools/run_arm.py 定形)。
- **红修 #2(绑定面)**:发现 full+f0(含 AE)digest 与无 AE 组合位级相同 = AE 链失效。根因 = realism 形态 AE 下标族缺失——tri_base 尾挂 32/40 后 AE 三件须顺延 +1(33..=35/41..=43),首版沿用 _EM 下标(32..=34)⇒ tri_base 被 AE reduce 当 state 写、真 params 被当 partials 越界写(release 下 debug_assert 不生效未拦截)。修 = _REAL 下标族 + 4 屏障计划 + builder/set_autoexp 双分支(guard 先于 em)。红修额度用尽,重验必须过。
- **同域既有缺口登记(不修,de342586 锚内冻结)**:em+AE 组合 set_autoexp 选择块无 _EM 分支——override 传 TEXNRM(32,33) = (triem, 真 params),即 day_0828 Phase F 起 full 十臂的 AE 逐帧 override 绑定已错位。为何未见异常:确定性错乱 ⇒ digest 稳定 + 增益槽未被有效写 ⇒ AE 实际近似恒等。修复 = 语义变更即重锚,归 HANDOVER 待主线批准。
- **红修 #2 重验全绿(收官)**:off 双锚 MATCH;on 双跑位级 `5fbafab8` ×2(≠ 错位时代 87d7139f,AE 链真生效自证);VUID=0;帧时 7.30/7.22ms(基线带内,F0 零射线成本)。臂① on 语义锚 = `5fbafab866e2430a...`(--quality full --metal-f0 on,96f)。

## 臂② --rt-ao 短程 RT AO(**收绿**,零红修)

- 机制:余弦半球 N 条短程遮蔽射线(t_max = 半径 0.5m 默认,first_hit;R2 + params[52] 帧旋转 + 样本 index 偏移),`ao_vis = 1 − strength·occ/N` 只乘 `al·amb` 常量环境光项(GI2 真可见性间接光不乘;sky_amb 与 hit_f 天然互斥)。默认 strength 0.85/samples 2。
- 接线:`--rt-ao off|on` + `--rt-ao-radius/-strength/-samples`(须随 --smooth-normals && --textures);SPV `g31_realism_ao.spv`;时序臂共用 params[52] 帧旋转(gi2 off 时 realism 块补写,set_gi2_frame 条件放宽)。
- 验收:off 双锚 MATCH;on 双跑位级 `d4d67354` ×2 + VUID=0;帧时 on 7.52/7.87 vs off 7.25ms(**+0.4ms**,预算内);A/B(无 AE 对照):changed_frac 3.7%(接触区空间选择性)、掩码亮度 12.65→9.41(**遮蔽变暗 −26%**)、全屏 mean −2.8%(遮蔽减能方向正确)。

## 臂⑤ --soft-shadows 点光软阴影(**收绿**,零红修;⚠帧时定档点)

- 机制:点灯阴影射线 1 条灯心 → 逐灯半径 `points[pb+6]` 圆盘采样 N 条(⊥灯心方向圆盘;R2 + 帧旋转 + 样本 index + 灯序号黄金比去相关;TSR 时域收敛半影);A1 灯罩豁免同律 t = d_s − max(2·eps, lr);光度项仍灯心方向(lr≪d 近似,SMRT 简化形如实登记)。默认 samples 2。
- 验收:off 双锚 MATCH;on 双跑 `e68f2561` ×2 + VUID=0;A/B:changed 7.0%(阴影边缘带)、掩码亮度 19.04→19.04 + 全屏 −0.14(**半影重分布能量守恒**,方向正确)。
- **⚠帧时:on 10.96/11.45ms(+3.9ms)踩 90fps=11.11ms 线**(2 样本 ×12 灯 = 24 条发散阴影射线)。终局定档:默认样本降 1 或不并入默认档,组合实测定夺。

## 臂③ --rt-reflect 光追镜面/glossy 反射(**收绿**,零红修)

- 机制:逐像素 1 条 GGX 半矢量重要性采样反射射线(tan²θ_h = α²u/(1−u),R2 + 帧旋转黄金比共轭偏移;rough > 上限 0.55 零射线零成本);命中点 GI2 形着色(mats 均值 + emission + 单灯 NEE);合成 spec += w(rough)²·F(F0,cos_vh)·L_clamp(8.0)。**有偏近似如实登记**(单样本无 pdf 归一,能量由 clamp+w 控)。F0 = 臂① alf 并入后 f0_*(金属映场景主消费面)。
- 验收:off 双锚 MATCH;on 双跑 `c06ce663` ×2 + VUID=0;帧时 +1.6ms(8.93/9.39);A/B:changed 0.47%(光滑面/金属小区域——bistro 多粗糙面,rough_max 0.55 门下如实)、掩码亮度 43.0→48.6(**反射加性 +13%**)、全屏 +0.2(能量增方向正确)。

## 臂⑥ --gi2-tex GI2 贴图反弹(**收绿**,零红修)

- 机制:GI2 反弹命中点 albedo 从 mats 均值直读升级为 heap 采样(反弹命中重心 UV〔committed_barycentric〕+ 主命中同 lod 公式〔距离 = 反弹程 gi_th〕+ 双线性 + linlut × mod);emission 逐像素(triem 槽号,slot 钳 0 + branchless 选择,主命中 em 段逐字同形);while 计数门包采样,off/无槽/miss 零采样零读 mats 逐字回退。
- 验收:off 双锚 MATCH;on 双跑 `6c56d857` ×2 + VUID=0;帧时 +1.0ms(8.49/8.11);A/B:changed 0.08%(间接光在 lamp-gain 4 直接光下占比小,gi_scale 1.0 幅度如实)、掩码亮度 86.4→82.5(反弹色从均值→贴图色重分布)、全屏 −0.06 守恒。

## 臂④ --normal-maps 法线贴图接线(**收绿**,零红修)

- 装配链:烘焙侧车 `a4_normalmap/bake_normals.py`(BC5 标准 D3D/bcdec 表解码——并登记 lane_body 休眠 bc4_alpha 系数族偏差警告,槽号 = heap top-70 同律)→ `pack_normals_bin.py`(DDS 全 mip 链逐级解码零重采样 → rgba8bin 容器,62 张 2048²×12 级 + 8 张 1² 占位,70/70 零异常)→ 运行时 `g31_normals_append`(em append 同律:头表 74→144×13 全重排 + cap-1024 起级尾接,heap 增量 ≈350MB 在 2GiB 界内,+13s 装配)+ `g31_assemble_tri_tan`(UV 导数切线 4 f32/tri + 手性 w,tangent_ref.py 参考对拍)。
- kernel:trinm/tri_tan 双 buffer 签名(_nrm 17 路,最高链位);XY 线性解码((byte−127)/127,不过 sRGB LUT)→ Z 重建 → 逐像素 Gram-Schmidt + TBN 扰动(强度 params[66]),扰动后法线进直接光/GGX/环境光/AO/GI2/反射全链;退化切线 branchless 保原法线。
- 接线:`--normal-maps off|on` + `--normal-strength/--normal-dir`;desc 尾挂 trinm=33/tri_tan=34(41/42 bloom 形态)+ AE _NM 下标族(35..=37/43..=45,红修 #2 律前置吸收零红修)。
- 验收:off 双锚 MATCH;on 双跑 `dca78cbe` ×2 + VUID=0;**帧时 +0**(7.58/7.73 vs 7.63——采样成本被 cache 吸收);A/B:changed 7.6%(法线细节面)、掩码亮度 29.1→26.8(高频明暗重分布)、全屏 −0.16 守恒。

## 帧时账本(单臂增量,no-AE 对照口径;基线 7.2-7.5ms)

- f0 +0 / ao +0.4 / soft **+3.9** / refl +1.6 / gitex +1.0 / nrm +0。
- 全臂线性和 ≈ 14+ms > 11.11ms 预算 ⇒ F1 组合定档实测。

## 终局 F1 — 组合定档(fails=0)

- combo_s2(soft 2 样本):12.96ms **超预算** ✗;combo_s1(soft 1 样本):**9.54ms ✓**(余量 1.57ms);combo_ao1(再降 ao 1):9.55ms 无额外收益。
- **定档:六臂全并入,--soft-shadow-samples 预设 1,其余臂默认**(TSR 帧旋转时域收敛半影仍成立)。

## 终局 F2 — --quality full 十六臂升档重锚(fails=0)

- 展开面 12→19 字面(+ 七 realism 字面,fail-closed 重叠检查同步);soft samples 预设 1。
- **新锚 = `5db2e7d72e6b4f3c961d1acdd48d05c60df24e8803a26f4dfdb37665b79bf673`**(96f/warmup2,双跑位级 ×2,VUID=0,帧时 9.92/9.51ms 预算内;与 F1 combo_s1 显式组合 digest 位级相同 = 展开语义精确等价自证)。
- 零漂移复验:all-off 55e4a92d MATCH / bench c1d28ad7 MATCH / Stage A 18/18 MATCH。
- **锚谱系:9e5f6300(九臂)→ 78113d56(十臂 γ1)→ de342586(十臂 γ2.5)→ 5db2e7d7(十六臂 realism,前锚作废)**。证据 `final/F2_ANCHOR.json`。

## 终局 F3 — 风暴 + soak(fails=0,战役收官)

- 风暴:full 十六臂 --window-storm 3(dolly 30f)rc=0 + resize_eras=1 + VUID=0。
- soak:**1955.4s ≥ 1800s**,9 迭代 32f 全位级稳定 + VUID=0;**帧时峰值 10.70ms ≤ 11.11ms(90fps 全程达标)**;Stage A 探针 ×2 MATCH。证据 `final/F3_SUMMARY.json` + `final/soak/`。

## 评估交付(evals/)

- `EVAL_RESTIR.md`:不接大件(单 pass megakernel 结构 + M100 承接锚);推荐 GI2 反弹 RIS 选灯 + 灯片 CDF NEE 两个 1 臂当量;第一步 = K 阶梯 workload 证据实验。
- `EVAL_DENOISE.md`:不上 SVGF(预算/结构/需求三缺);第 0 级 tsrq K 档实测;方差源头收缩优先于降噪器平台化。

## 战役收官声明

六臂全绿并入 + 升档重锚 + 风暴 + soak 全过;共享体/母版 kernel/锚定 SPV 全程 0-byte;bench 与 Stage A 面零漂移;红修总计 2 次(均臂①,额度内);证据链完整落 `artifacts/day_0829_realism/`。交接文档 `final/HANDOVER.md`。

## 终局自证(收官核验)

- 禁区零触碰:00_MASTER_INDEX/11_ROADMAP/G31_TODO/milestones/g35 的 git M 均为他会话既有(开工快照在案);未 git commit。
- 共享体 0-byte:`g14_3_lane_body.rs` git diff 中 day_0829 字样命中 = 0(其 M 为 day_0828 未提交遗留)。
- 母版 kernel 0-byte:`g31_texture_nrm_gi.rx` sha256 = 9ec07050(与 fork 时点同值;其 ?? 为 day_0828 untracked 既有状态)。
- 锚定 SPV SAME:fd22cb19 / 75d08aec / bdd23a3a(收官时点复核)。

## 子代理回报终审补登(收官后)

- 臂④烘焙侧车确认:BC5 全 70 张 ATI2(61×2048² 12 级链 + 9×1² 占位);em 臂 mip 律 = 烘焙侧全链 + 运行时零重采样(pack_normals_bin 已同律);lane_body 休眠 bc4_alpha 系数族与标准 BC4 表差 +e0/7——臂④烘焙链用标准表,kernel 消费已解码字节不触该面。
- **slot14(Paris_Table_cloth_01)法线源件损坏**:整张常值 (53,53) 非法法线,kernel max(0,·)+归一化兜底无 NaN,仅该材质方向有偏——5db2e7d7 锚内如实登记,修复(烘焙替平坦/trinm 置 −1)即重锚,留窗(HANDOVER §H)。
- raw dump 格式核准(工具子代理):8B w/h 头 + BGRA8;战役 A/B 脚本按 len/4 读入含头 2 假像素,on/off 同偏 ⇒ 判据影响 ~1e-6 稳健不重跑;后续以 tools/ab_metrics.py(跳头)为准(HANDOVER §H)。

## 收官后追加:交互预览"两个三角形雾状区"消融定位(用户报告)

- 现象:交互窗口屏幕中央两块三角形/楔形半透明模糊区,"总是"可见。
- 消融链(orbit 运动口径 dump ×6,`final/png_triage/`):关 rt-reflect 不消除 → 关 normal-maps 不消除 → 关 soft-shadows 不消除 → 关 tsr-quality 不消除 → **十臂基线(六新臂全关)同在** ⇒ 与本役全部改动无关。
- 根因:吧台玻璃隔断(TransparentGlass.DoubleSided)被无透明管线按不透明渲染,薄面 + jitter/TSR 时域混合呈半透明雾状楔形(一块玻璃板 = 2 三角形)。资产级既有限制,登记 HANDOVER §H 留窗(透明管线为独立立项;装配层跳过 = 重锚)。
- 附带发现:full 十六臂交互/可见窗口首帧前装配约 2-3 分钟同步阻塞消息泵(窗口期间"未响应"为预期形态,hidden 验收面从未暴露)——装配移后台线程 + 加载画面留窗。
