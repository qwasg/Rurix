# G37 W2 --transparency 加性臂:玻璃透射真解(ray 穿透)

修「玻璃隔断雾状楔形」缺陷(day_0829 HANDOVER §H:TransparentGlass.DoubleSided,
slot00,130,792 tris,被无透明管线的渲染器按不透明渲染)。本报告 = 设计说明 +
修改清单 + SPV sha 前后自证 + 资源计数/AE 下标族表 + GPU 验收步骤(本会话不跑
GPU)。所有修改处注释「G37 W2 transparency」。

## 一、kernel 段设计(kernels/g31_realism.rx 就地追加,第 7 链位)

### 签名扩展
`tri_transp: View<global, f32>` 追加在 `tri_tan` 之后、`out_color` 之前(链式
超集律:新最高链位)。1 f32/tri:0 = 不透明,(0,1] = 透射率。SPIR-V binding 计数
19 → 20(含 tlas;spirv-dis 核对 nrm 19 / transp 20,+1 = tri_transp)。

### 主射线穿透段(params[68] 门;插在主命中提取后、几何提取前)
- while 计数门 `tp_n = (tp_gate × hit_f × 8) as usize`(最大 8 层:玻璃双面 +
  多板;gate off / 主射线 miss ⇒ 零迭代零读)。
- 每层:读 `tri_transp[prim]`;透明(>0)⇒ tint 逐通道累乘「透射率 ×
  **tri_base 未衰减 baseColorFactor**」,从 `near + d·(th+ray_eps)` 沿**原方向**
  重投(工程直线透射,不折射如实登记),`th` 保持「距 near 累积程」参数化 ⇒ 下游
  hx/hy/hz、mip lod 公式(`th·k_pix·k_tri·w`)、深度全链同式零改;不透明 ⇒ tint
  ×1 恒等 + branchless 计数器越上限跳出(`tp_i += 1 + ((1−tp_pass·hit_f)·9) as
  usize`,gate 乘积 ∈ {0,1} 精确整数域)。
- **tint 色调源改判登记(装配量化后的设计修正)**:任务建议形态「材质 albedo
  调制」若取 mats 均值面 = tex_mean(0.219 线性,TransparentGlass_BaseColor.dds
  16×16 中灰)× factor([1,1,1])× (1−metallic)(metal=0.4 ⇒ ×0.6)≈ 0.13
  ⇒ 每面 tint 0.11、双面板 ≈ **0.012 全黑**,比雾状楔形更糟(臂① F0 修伤同根:
  mats 面被 k_metal 与灰贴图双重衰减)。改取臂① tri_base 侧表 = 未衰减
  baseColorFactor(bistro 玻璃 = [1,1,1] ⇒ tint = 纯透射率 0.85/面、双面板
  0.7225 物理合理;有色玻璃资产色调仍生效);代价 = `--transparency on` 时
  tri_base 恒真表(host 条件 `metal_f0 || transparency`,--textures 已裁 ⇒
  tex_report 必在)。逐像素贴图色调透射留窗。
- 命中态覆写:th/prim/bu/bv/hit_f(既有 mut;臂④ nx/ny/nz mut 化同律)= 最终
  不透明命中 ⇒ 直接光/GGX/GI2/AO/反射/em/法线贴图全部在穿透后真实着色点求值;
  穿出场景 = miss 语义(th/prim 置 0 保底与母版 miss 面同形,天空底色经 tint 衰减)。
- 输出施加:三输出行 `out_color = tp_tint_c × (hit_f·(…) + sky_amb·w)`(臂②
  ao_vis 插乘同律;关臂 tint 恒 1.0 数学恒等——但不依赖恒等保锚,off 面根本不载
  本 SPV)。
- **一句话**:主射线命中透明三角就带 tint(透射率×未衰减 baseColorFactor)沿
  原方向重投直到打到不透明面/穿出,整像素辐亮(含天空)最后乘累积 tint。

### 点光阴影衰减段(params[68] 门;soft-shadow 样本循环内,既有 first_hit 判遮后)
- 既有 first_hit 判遮块内追加 2 行提取 `committed_primitive_index()`(纯寄存器,
  blk 语义不变);判遮命中恰为透明三角(`tri_transp[first_hit_prim]>0`)且门 on
  才触发重走——被不透明遮挡面(绝大多数样本)零重走零射线,只 +1 buffer 读。
- 重走 = closest-hit 自着色点沿样本方向逐层(≤8):透明层 ×透射率续走并推进
  `tp_sadv += t+ray_eps`(重投 t_max = 剩余程钳 ray_eps),不透明层 ×0 全遮跳出,
  穿出保持累积;`blk` 覆写 = 1 − 累积透射率(全不透明重走 blk=1 与既有位同值语义;
  纯玻璃遮挡 = 部分遮 ⇒ 玻璃灰影)。soft on/off 正交(单射线灯心面同构生效)。
- **一句话**:只有 first_hit 判遮撞上玻璃时才 closest-hit 重走,玻璃层按透射率
  把硬黑影衰减成灰影,撞墙即全遮。
- **不接线登记(留窗)**:quad 灯阴影(bistro quads=0 零消费面)、GI2 反弹 NEE、
  反射 NEE、AO 射线仍视玻璃为不透明(次级路能量占比小;quad NEE 缺口同律)。

### 分支纪律
while 计数门 + branchless gate 乘法 + 「if 包 ray query(空体 proceed + if
committed)」白名单形(灯循环/GI2/refl 逐字同构);「if 包计数 while」禁形不触
(rurixc 该缺陷在另分支修复中,本臂不依赖)。既有段落零改写,除三处登记的 gate
化钩:①输出三行 tint 头乘(ao_vis 同律)②first_hit 判遮块内 prim 提取 2 行
③头注 params 表 [68] 登记行。

### 已知近似(如实登记)
1. tint 色调 = tri_base 未衰减 baseColorFactor(非逐像素贴图采样;bistro 玻璃
   factor=[1,1,1] ⇒ 纯透射率,双面板 0.7225);阴影重走衰减为纯透射率不带色调。
2. 玻璃面自身高光/Fresnel 反射零贡献(穿透绕过其着色);资产 alpha=0.2 语义为
   coverage,透射率统一取 --transp-alpha 工程值。
3. 玻璃像素深度 = 穿透后背景命中深度(TSR 重投随透射主导辐亮)。
4. 次级射线(GI2 反弹/反射/AO/quad 阴影)仍视玻璃为不透明。

## 二、tri_transp 判定规则与命中三角数

**判定**(装配期 glTF 二次解析,`g31_assemble_tri_transp` 窗口 bin 自有,
g31_assemble_tri_base 同律):材质 `alphaMode == "BLEND"` **或**
`pbrMetallicRoughness.baseColorFactor[3] < 1.0`。判定零命中 fail-closed(臂无
消费面拒跑);tri_mat 越界 fail-closed;`SLAB_TRI_NONE` 灯面 = 0(不透明)。

**bistro 静态核算**(BistroInterior.gltf,70 材质):alphaMode 全 OPAQUE;唯一
alpha<1 材质 = **mat7 TransparentGlass.DoubleSided(baseColorFactor.a=0.2)**,
命中 **130,792 tris**(与 HANDOVER §H 登记数一致;glTF mesh 三角总和 1,046,609,
运行时 tri_count 长度互核 fail-closed)。名字启发式不进判定(如实登记:名字含
"Glass" 的 9 材质中 8 个为实心/涂层玻璃形态——酒瓶×4/收银机/画框/外窗
MASTER_Glass_Exterior/Frozen_Glass,误伤面 ≫ 收益;缺陷本体已由 alpha<1 精确
覆盖)。透射率 = `--transp-alpha`(默认 0.85)常值写表;装配日志逐材质登记
(mat 号/名/tris/透射率/判定规则字面)。

## 三、新增 flag / 常量 / 下标族清单

### CLI flag
| flag | 域 | 默认 | 说明 |
|---|---|---|---|
| `--transparency off\|on` | 闭集 | off | 须随 `--smooth-normals on && --textures on`(realism 链基座,fail-closed);并入 `--quality full` 展开集(19→20 项,十六臂→十七臂,full 语义变更重锚归 W4) |
| `--transp-alpha <f32>` | (0,1] | 0.85 | 须随 --transparency on;**不进** full dup 表(rt-ao 子参数同律,可与 full 组合微调) |

### kernel/params
- `params[68]` = transparency 门(原 [68..72) 预留槽头位;`G31_REAL_PARAMS_LEN=72`
  不变);lane `set_transparency()` + `prepare_update` 写 [68] + realism 扩面
  条件并入 `self.transparency`。
- `realism_any` 并入 `|| transparency`(triem 回退真表/tri_base 哑表/params
  扩容三机制自动覆盖)。

### SPV 换载
- `G31_DEFAULT_SPV_REALISM_TRANSP = ".tmp/night_0829/spv/g31_realism_transp.spv"`;
  「默认字面才换」:transparency on 且 spv_texture ∈ {TEXTURE_NRM, _GI2, _EM,
  REALISM_F0/AO/SOFT/REFL/GITEX/NRM 九默认字面} ⇒ 换 _TRANSP(链最高位,与
  realism 六臂正交组合——transp on 而 nrm off 也走 _transp,链式超集 gate 控制)。

### 资源下标 / 计数断言 / AE 下标族(红修 #2 律:双接线 + assert 连号保护网)
| 形态 | tri_transp 下标 | 资源计数 | AE state/params/partials | AE 计划 |
|---|---|---|---|---|
| tex+nrm(+em 回退+real+nm 回退)+transp | `G31_U_TRITRANSP_TEXNRM=35` | `G31_U_RESOURCE_COUNT_TEXNRM_TRANSP=36` | 36/37/38(`G31_U_AE_*_TEXNRM_TRANSP`) | `G31_U_PLAN_AE_{REDUCE,STATE}_TEXNRM_TRANSP` |
| ×bloom | `G31_U_TRITRANSP_TEXNRM_BLOOM=43` | `G31_U_RESOURCE_COUNT_TEXNRM_BLOOM_TRANSP=44` | 44/45/46(`…_BLOOM_TRANSP`) | `…_BLOOM_TRANSP` |

- scene 屏障计划:`G31_U_PLAN_SCENE_TEXNRM_TRANSP` / `…_BLOOM_TRANSP`(NM 计划
  超集 + tri_transp)。
- AE 双接线 guard 序:`transparency` 分支置于 **normal_maps/realism_any 之前**
  (transp 挂载序最尾即下标最高;set_autoexp 选择块与 g31_apply_autoexp 调用点
  match 逐字同构,W1 升级后的 assert 连号为错配 fail-fast 保护网)。
- 绑定占位律:transp on 而 --normal-maps off ⇒ `trinm` 绑 tri_count×(-1.0)
  回退真表(kernel `trinm[prim]` 无门保底读,triem 回退表同律)、`tri_tan` 绑
  16B 零哑表(nm while 零迭代零读);lane descs 断言 `tri_transp Some ⇒ nm 位
  Some` fail-closed。

### 装配
- `fn g31_assemble_tri_transp(gltf_path, tri_mat, alpha_v) -> (Vec<f32>,
  Vec<(mat_idx, name, tris)>)`(判定 + 命中登记);era 外一次打包字节面 +
  tri_count 长度互核 + 装配日志。

## 四、修改清单

| 文件 | 修改 |
|---|---|
| `src/rurix-render/kernels/g31_realism.rx` | 头注 params 表 [68] 登记 + G37 W2 侧表登记;签名 +tri_transp;主射线穿透段(gate 化插段);点光阴影衰减段(first_hit 块内 prim 提取 2 行 + gate 化重走插段);输出三行 tint 头乘。其余段落 0 改写 |
| `src/rurix-render/src/bin/g31_window_present.rs` | SPV 常量 + REAL_PARAMS_LEN 注释;TRANSP 资源/计数/AE 下标/4 屏障计划常量族;`g31_assemble_tri_transp`;两 lane descs 函数 +tri_transp 参数/挂载/断言/屏障选择;lane 字段/`set_transparency`/prepare_update [68];CLI 声明/parse/校验/换载/realism_any/quality full(20 项);era 外字节面(真表+回退表+哑表)+ **tri_base 真表条件扩 `metal_f0 \|\| transparency`**(透射色调消费面);调用点 nm_ref 回退分支 + transp_ref;AE 施加 match + set_autoexp 选择块各 +2 分支(guard 最先);挂载点 `set_transparency()` |
| 新工件 | `.tmp/night_0829/spv/g31_realism_transp.spv`(272,032 B) |

未触:`kernels/g31_texture_nrm_gi.rx` 母版、既有 .spv 全部、`g14_3_lane_body.rs`、
`g14_3_pipeline_perf.rs`、`milestones/`、`registry/`、`ci/`、其余共享体。

## 五、SPV sha256 前后自证(编译前 `.tmp/w2_transp_sha_before.txt` / 后 `_after.txt`,Compare-Object = ALL SAME)

| 文件 | sha256(前 = 后,SAME) |
|---|---|
| g31_realism_f0.spv | `b3dffbe6292f2ed7d837352ea4e0efb870aaf298d321f4444b65377b5edc4915` |
| g31_realism_ao.spv | `76fff402be5d07775f8c5d95fef84dd5ff444ab0ed101dd1a23f7f8bf46398ee` |
| g31_realism_soft.spv | `4eca2067f87fabca726185e6e1af29754b25beb3b41f0620c670293df20583bd` |
| g31_realism_refl.spv | `e418990c240570d4d6fe4fde0fa60184e31ca914a517b723627958df718b73be` |
| g31_realism_gitex.spv | `a0a3c821b8ec8aee6989b9068c19a59e44fc92c79f3a40c98298a591d8a32f1b` |
| g31_realism_nrm.spv | `0c68fc49456798890ad1680cbb2ea4d4be0d53e54ba20e42f4e88e4501f5f4b6` |
| g31_texture_nrm_gi.spv(冻结) | `fd22cb19c563efc7187b9ea61bcc27afdad56afbf880886af4ca9f541d14e6f7` |
| g31_texture_nrm_gi_gi2.spv(冻结) | `75d08aec5ec89f0d028f2753d8aecafc626383c2b34e5034d10adc41c17da7a4` |
| g31_texture_nrm_gi_em.spv(冻结) | `bdd23a3a14e01cdd325e020c8689adc26cf7db55e7088df70d0d1ca5ff870e25` |
| kernels/g31_texture_nrm_gi.rx(母版源,0-byte) | `9ec07050121611da424dcfbc2cc469a8ad39ced33f1db4bbb0780abe56504c9c` |

**新增**:`g31_realism_transp.spv` =
`35983d0f405169ec84bf222f4a12ec8bf8dfd7d471eefb12488eea7dd34c4f8b`(272,032 B;
tri_base tint 终版——首版 mats tint `0b9b7a91…` 经装配量化否决作废,谱系如实
登记)。spirv-dis binding 计数 nrm 19 → transp 20(+1 = tri_transp,含 tlas)。

## 六、编译/验证结果

- rurixc 现建:`cargo build -p rurixc --features vulkan-backend --bin rurixc`
  (dev;CARGO_TARGET_DIR=.tmp/w2_build 侧目录避免与并行会话默认 target 互覆写,
  不碰 target-night);编译形状 `rurixc <src.rx> --target vulkan -o <dst.spv>`
  (day_0830 w1 同形)。rurixc 内嵌 spirv-val accepted + PATH 独立 `spirv-val`
  exit=0 双过。
- `cargo check -p rurix-render` exit=0;窗口 bin 有 required-feature 门 ⇒ 追加
  `cargo check -p rurix-render --bin g31_window_present --features vendor-upscale`
  exit=0(bin 4 warnings 全为 hzb/svt 既有面:8485/8488 unused doc comment、
  8505/8506 svt_era 赋值未读——非本次引入)。默认 dev/默认 target 目录,未用
  --release/target-night。

## 七、GPU 验收步骤(主agent执行;全程 RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1,VUID=0 门)

```powershell
# 0) 构建窗口 bin(主agent自有构建纪律/target 目录)
# 1) all-off 锚零漂移(off 面不载新 SPV 字节,数字必须逐字):== 55e4a92d…
g31_window_present.exe --frames 8 --warmup 2 --hidden --evidence ev_alloff.json
# 2) transparency 单臂最小组合,双跑位级(digest run1 == run2;装配日志应见
#    「透明材质 mat7 TransparentGlass.DoubleSided(130792 tris,透射率 0.85)」)
g31_window_present.exe --frames 96 --warmup 2 --hidden --smooth-normals on --textures on --transparency on --evidence ev_transp_1.json
g31_window_present.exe --frames 96 --warmup 2 --hidden --smooth-normals on --textures on --transparency on --evidence ev_transp_2.json
# 3) full 十七臂(--quality full 已含 transparency),双跑位级 + 帧时记账
#    (real_render_frame_ms vs 十六臂基线 9.5-9.9ms,预算 11.11ms;新锚归 W4)
g31_window_present.exe --frames 96 --warmup 2 --hidden --quality full --evidence ev_full17_1.json
g31_window_present.exe --frames 96 --warmup 2 --hidden --quality full --evidence ev_full17_2.json
# 4) 无 AE A/B(day_0829 红修 #1 定形:全显式 16 臂无 AE 对照;掩码 = 玻璃楔形
#    区域〔final/png_triage/d_full16_full.png 屏幕中央〕;raw 判读一律
#    tools/ab_metrics.py 跳头版;dump 形状 = run_arm.py 定形)
#    off 臂:
g31_window_present.exe --frames 96 --warmup 2 --hidden --smooth-normals on --ggx on --lamp-lights on --lamp-gain 4 --textures on --bloom on --dither on --tsr-quality on --gi2 on --gi2-clamp 0.01 --emissive-tex on --metal-f0 on --rt-ao on --soft-shadows on --soft-shadow-samples 1 --rt-reflect on --gi2-tex on --normal-maps on --dump-present-raw ab_off.raw --dump-present-every 95 --evidence ab_off.json
#    on 臂 = 同上 + --transparency on(判据:玻璃掩码内显著 diff〔楔形消隐/透见
#    背景〕;掩码外 diff ≈ 0〔换载 SPV 驱动重编译 ULP 扰动预期,非位级 0——C 相
#    字节隔离教训,不以恒等为门〕;玻璃灰影 vs 全黑硬影对照点在吧台后地面/台面)
# 5) 组合正交抽查(transp on 而 nrm off 走 _transp 链位):
g31_window_present.exe --frames 96 --warmup 2 --hidden --smooth-normals on --textures on --gi2 on --transparency on --evidence ev_orth.json
# 6) 全程 VUID=0;帧时超预算时旋钮:--transp-alpha(透射率)/裁剪阴影重走面归档
```

预期成本形状:非玻璃像素 +1 次 tri_transp 读/帧(gate on);玻璃像素 +1..4 条
closest 重投;阴影重走仅 first_hit 判遮撞玻璃的样本触发(≤8 层)。亮度预期:
bistro 玻璃 tint = 0.85/面(baseColorFactor=[1,1,1]),单板双面透射 0.7225——
玻璃区应呈「清透略暗 + 灰影」;若观感需调,`--transp-alpha` ∈ (0,1] 为唯一旋钮。
