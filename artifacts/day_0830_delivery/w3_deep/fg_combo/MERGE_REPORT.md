# G37 W3 fg_combo 合入记录 — 16 锚接线 + FULL 下标族重推导

- 日期:2026-08-30(G37 商业化收官战役 W3 合入子任务;依据 = 本目录 REPORT.md 判档)
- 修改域:**仅 `src/rurix-render/src/bin/g31_window_present.rs`**(独占编辑权会话)+ 本文件
- 执行序:B 组(接线,死代码期)→ A 组(开闸)→ C 组(登记),每组后
  `cargo check -p rurix-render --features vendor-upscale --bin g31_window_present` 全绿
- 纪律核验:未跑 GPU;未 `cargo build --release`;未碰 target-night;lane_body/kernels/
  SPV/milestones/registry/ci 全部 0-byte;修改注释统一「G37 W3 fg_combo 合入」

---

## 1. FULL 下标族推导表(RIS/NEE 合入后按现文件真实终态重推——核心偏差)

报告 §3.2 基于制作时终态(TEXNRM_BLOOM_TRANSP=44 + AE 44..=46 ⇒ FG 47..=55)推导;
RIS/NEE 于 W2 并行合入后 full 终态漂移,按现文件资源计数逐段重推:

| 段 | 资源区间 | 计数依据(现文件内容锚) |
|---|---|---|
| unified 基座 + encode 两件 | 0..=23 | `G31_U_RESOURCE_COUNT = 24` |
| bloom 八件 | 24..=31 | `G31_U_RESOURCE_COUNT_BLOOM = 32`(comp_out = 31) |
| tex 五件 | 32..=36 | `G31_U_RESOURCE_COUNT_TEX_BLOOM = 37` |
| trinrm/tri_mr | 37/38 | `G31_U_RESOURCE_COUNT_TEXNRM_BLOOM = 39` |
| triem | 39 | `…_TEXNRM_BLOOM_EM = 40` |
| tri_base | 40 | `…_TEXNRM_BLOOM_REAL = 41` |
| trinm/tri_tan | 41/42 | `…_TEXNRM_BLOOM_NM = 43` |
| tri_transp | 43 | `…_TEXNRM_BLOOM_TRANSP = 44` |
| **lamp_tbl(W2 ris_nee 尾挂)** | **44** | `G31_U_RESOURCE_COUNT_TEXNRM_BLOOM_RIS = 45` |
| **AE 三件(_RIS 族)** | **45/46/47** | `G31_U_AE_{STATE,PARAMS,PARTIALS}_TEXNRM_BLOOM_RIS = 45/46/47` |
| ⇒ full+AE 终态资源数 | **48** | `g31_apply_fg_full` 施加期 `assert_eq!(len, 48)` 钉死 |

**FULL FG 下标族最终数字(报告 47..=55 → 实际 48..=56,整体 +1)**:

| 常量 | 报告值 | **合入值** | 内容 |
|---|---|---|---|
| `G31_U_BLOOM_COMP_HIST_FULL` | 47 | **48** | comp parity 伙伴缓冲(opc×12) |
| `G31_U_MVN_PARAMS_FULL` / `G31_U_MVN_FULL` | 48/49 | **49/50** | MV 取反参数/输出(opc×8) |
| `G31_U_FG1_PARAMS_FULL` / `G31_U_FG1_OUT_FULL` / `G31_U_FG1_BGRA_FULL` | 50/51/52 | **51/52/53** | FG1 三件 |
| `G31_U_FG2_PARAMS_FULL` / `G31_U_FG2_OUT_FULL` / `G31_U_FG2_BGRA_FULL` | 53/54/55 | **54/55/56** | FG2 三件(x3 才构造) |

comp parity 对 = **(31, 48)** = (`G31_U_BLOOM_COMP_OUT`, `G31_U_BLOOM_COMP_HIST_FULL`)。

pass 图(RIS 不加 pass,报告数字维持有效):full+AE 11 pass(scene/mv/resample/resolve/
bright/blurH/blurV/composite/reduce/state/encode)→ mvn=11、fg1=12、enc_fg1=13、
(x3)fg2=14、enc_fg2=15。readback 布局(基座 0..=4 与 base 逐字同序,tex/nrm/bloom/AE
均不加逐帧 readback):x2 = comp0/comp1/fg1_bgra/fg1_out/mvn @5..=9;x3 =
comp0/comp1/fg1_bgra/fg2_bgra/fg1_out/fg2_out/mvn @5..=11——与报告 §3.2 表同构。

保护网:`g31_apply_fg_full` 施加期 `assert_eq!(d.resources.len(), 48)`(W1 升级律,
错位即红修 #2 症状)+ 末位 pass 名复核(`g31_display_encode`,施加序前提)+
`g31_apply_autoexp` 既有连号 assert(45/46/47 错位在 AE 施加期先红)。

## 2. 16 锚落点与偏差登记

### B 组(接线八锚,死代码期合入——A 组开闸前 fg×textures 组合 CLI 不可达,零行为)

| 锚 | 落点(内容锚) | 偏差 |
|---|---|---|
| B1 | `_RIS` AE 计划族常量之后插入 FULL 族 9 常量 + 屏障计划族 | ① 下标 +1(§1);② 计划族 **6→8 件**:报告六件之外补 `G31_U_PLAN_ENCODE_BLOOM_FULL_FG`/`G31_U_PLAN_AE_REDUCE_FULL_FG`——encode/AE reduce 读 comp[p] 逐帧轮换,&'static 静态计划须覆双 parity 并集(composite 超集同一律的一致延伸;漏槽 = comp[1] 帧读写无屏障);③ 插入点从报告的 `_TRANSP` 常量组后顺延至 `_RIS` 计划族后(W2 追加所致,语义同位) |
| B2 | `g31_apply_autoexp` 函数体之后插入 `g31_apply_fg_full(d, fga, enc_spv, enc_dispatch, opc)` | assert 值 47→**48**;签名较报告草案增 `enc_spv/enc_dispatch` 实参(enc_fg 复用主 encode SPV/dispatch,调用点在 scope 直传);comp 触碰面三 pass 计划替换按 **pass 名**定位(composite/reduce/encode),不按下标——防并行合入 pass 图漂移 |
| B3 | `let mut off_descs = …` 之前分流 `(fg_assets_base, fg_assets_full)`,调用实参换 `fg_assets_base` | 报告草案「`if textures { None } else { fg_assets }`」内联式改为解构双元组(fg_assets_full 供 B4 消费,move 语义下等价) |
| B4 | AE 施加链(`// A2:对将被选中的变体描述组施加 autoexp 变换` 块)闭括号后追加 `if let Some(fga) = fg_assets_full.as_ref() { g31_apply_fg_full(tex_descs…) }` | 零偏差(必在 AE 之后——摘出断言前提由施加序保证) |
| B5 | lane struct 增 `fg_full: bool` + `fg_pass_fg1/fg_pass_fg2: u32`;`create()` 按 pass 名(`g26_framegen_fg1/_fg2`)派生下标 + `fg_full = fg on && bloom`;`G31FgLayout` 增 `rb_comp0/rb_comp1` 字段与 `of_full` 构造 | 下标派生从报告「pass 数派生」改为**按名派生**(更强不变量,fg on 缺 pass 即 Err);fg_full 判定 = fg×bloom(两点式下 ⇔ full,CLI 卫兵已裁散臂) |
| B6 | bloom override 块:composite(7) 出口换 `comp_p`、encode(10) 入口换 `comp_p`(经 `bindings_encode`)、fg_full 面新增 reduce(8) override 读 `comp_p` | fg off 面 `comp_p` 恒 `G31_U_BLOOM_COMP_OUT` ⇒ composite/encode override 与既有静态语义逐字同值、reduce 不 override——三处 0-byte 达成 |
| B7 | fg override 块:pass 下标 `6`/`8` 字面 → `self.fg_pass_fg1`/`self.fg_pass_fg2`;fg_full 面 prev/cur = `(comp[1−p], comp[p])` + MVN/PARAMS/OUT 换 FULL 族 | 零偏差(base 面 U_OUT_COLOR 对 + base 族下标逐字维持) |
| B8 | probe 子集:fg_full 面 cur 换 `rb_comp0/1[p]`、prev 换 `rb_comp0/1[1−p]`;`rec_from_output` 解析面 0-byte(comp 回读尺寸 = out_color 同 opc×12);host `interpolate` 调用零改动 | 角例登记:fg_full 下 probe 帧恰为末帧时 `rec.out_color` 携带 comp[p](post-bloom)而非 U_OUT_COLOR[p],影响面仅 env 门控 f32 dump(`RURIX_G31_DUMP_F32`)与 `--dump-present-raw` 的末帧 f32 语义;验收窗 frames=64+10 下 probe(warmup 后首 gen 活跃帧)≠ 末帧,不触发 |

### A 组(判档字面六锚)

| 锚 | 落点 | 偏差 |
|---|---|---|
| A1 | `--textures on 与 --fg 互斥（B4 接线面…` 门条件加 `&& !quality_full`,讯息追加两点式判档字面 | 零偏差 |
| A2 | `--smooth-normals on 不与 --fg/…` 门 `fg != Off ||` → `(fg != Off && !quality_full) ||`,讯息同律 | 零偏差 |
| A3 | `--bloom on 不与 --fg/…` 同律 | 零偏差 |
| A4 | `--auto-exposure on 不与 --fg/…` 同律 | 零偏差 |
| A5 | `--tsr-quality on 不与 --fg/…` 同律 | 零偏差 |
| A6 | `--fg` 闭集块(`// A5 FG 闭集约束`)尾追加两点式卫兵 | **语义适配(报告原文不可用)**:报告草案 `if quality_full && (gi2_ris \|\| gi2_nee) { fail }` 制作于 ris/nee 未入 full 时;W2 已将两臂并入 `QUALITY_FULL_EXPANSION`(20→22 项)与 dup 表 ⇒ 原卫兵在 full 下恒触发,会误杀全部 fg×full 组合(语义反转)。适配为:`!quality_full` 面对「须随 smooth-normals/textures 传递覆盖」的 13 散臂(ggx/lamp-lights/gi2/emissive-tex/metal-f0/rt-ao/soft-shadows/rt-reflect/gi2-tex/normal-maps/transparency/gi2-ris/gi2-nee)全量 fail-fast 显式化——防后续臂解除上游门时静默放行;五门(textures/smooth-normals/bloom/auto-exposure/tsr-quality)有自身 !quality_full 豁免字面、lut/storm/hzb/slab/svt/headless 有自身 fg 门、dither 既有开放,均不入列不重复。原威胁(`--quality full --gi2-ris` 散臂越形态)现由 dup 检查拒跑(`--quality full 与显式旗标 … 冲突`);`--gi2-ris-m` 类子参数不改变体形态,随预设组合合法(E1 既有律) |

### C 组(登记面两锚)

| 锚 | 落点 | 偏差 |
|---|---|---|
| C1 | evidence 组装链 `if textures {` → `if textures && fg == G31Fg::Off {`;`evidence_path` 默认链同律;FG notes 字面追加组合面描述(comp parity/enc_params[133] 继承/不污染门/两点式/辉光 warp 近似登记) | 「fg 分支前移」以 textures 分支加 fg-off 限定实现(if-else 链上与物理前移语义等价——fg on ⇒ 非 svt/hzb/slab 已由闭集保证,最小 diff) |
| C2 | pass 链 eprintln:fg 臂按 bloom(⇔full)分流出「十四 pass/--quality full × fg x2」「十六 pass/x3」字面;文件头 usage `[--fg …]` 行追加组合说明 + A5 闭集注释段追加两点式/comp parity 登记 | 零偏差 |

## 3. cargo check 结果

```
cargo check -p rurix-render --features vendor-upscale --bin g31_window_present
```

- B 组后:绿(exit 0,Finished dev profile)
- A 组后:绿(exit 0)
- C 组后(终态):**绿(exit 0)**;bin 警告 4 条全为既有代码(hzb `unused_doc_comment`
  ×2、svt `unused_assignments` ×2,合入前后同集);IDE linter 零错误。
- dev/默认 target;未触 release/target-night。

## 4. 结构不变量自证(编译期/施加期,GPU 验收前置)

1. fg off 全路径 0-byte:FULL 族资源/pass/readback 仅 `g31_apply_fg_full` 构造(fg on ×
   textures 面才调用);`comp_p` 在 fg off/base fg 恒 `G31_U_BLOOM_COMP_OUT`,三 override
   与既有静态语义逐字同值,reduce 不新增 override。
2. base fg 面 0-byte:`fg_assets_base` 路径与既有 `g31_lane_descs` 内嵌逐字同参;override
   下标按名派生回落 6/8 同值;`G31FgLayout::of` 布局字段值不变(新增 rb_comp0/1 = MAX
   不消费)。
3. 真实帧位级不变(不污染门结构证):comp parity 仅换 composite 输出缓冲对象(偶帧写
   comp[0] = 既有 comp_out),encode/reduce 同帧同槽读——同 kernel 同输入,BGRA 输出与
   单 comp 静态位级同值;digest_seq = 逐真渲帧 BGRA sha256,结构上跨 fg on/off 一致。
4. 首帧/era 首帧:comp[1] 未定义内容零消费(`gen_active` false,FG 输出不读不 present
   ——base 面 U_OUT_COLOR 首帧同律)。
5. AE 继承:enc_fg1/enc_fg2 绑同一 `G31_U_ENC_PARAMS`,state pass 先于全部 encode pass
   ⇒ 生成帧自动继承本帧增益(慢收敛 EMA,帧间差可忽略,如实登记不适配)。

## 5. GPU 验收(独占 GPU 会话前置,本会话纪律禁跑)

```powershell
# 前置①:release 构建(验收窗会话执行;本会话禁 release)
cargo build --release -p rurix-render --features vendor-upscale --bin g31_window_present
# 前置②:.tmp SPV 构建产物在位(night_0828/night_0829/night_0830 各族 kernel;
#        缺件 = bin fail-closed 自报)
# 前置③:判读器自证(无 GPU)
py -3 artifacts/day_0830_delivery/w3_deep/fg_combo/accept_fg_combo.py --selftest
# 真跑五臂 + 互斥矩阵(独占 GPU;--plan 可先打印步骤)
py -3 artifacts/day_0830_delivery/w3_deep/fg_combo/accept_fg_combo.py --execute
# 复判(可重入)
py -3 artifacts/day_0830_delivery/w3_deep/fg_combo/accept_fg_combo.py --judge
```

判读关注点(§2.3 口径不变量):combo 双跑位级、`digest_seq(full+fg x2) ==
digest_seq(full, fg off)` 不污染门、`digest_seq(full) ≠ digest_seq(all-off)` 生效门、
presented 计数恒等式、real fps 口径隔离、`wired_parity.excess == 0` +
`mvn_max_abs_plus_mv == 0` + SSIM 胜 frame-hold(fg_full 下对拍输入 = comp 对,判据
公式零改动)、互斥矩阵六臂 exit=1(含散臂 `--textures on --fg x2` 两点式拒跑)。

- Assisted-by: Claude(G37 W3 fg_combo 合入子任务)
