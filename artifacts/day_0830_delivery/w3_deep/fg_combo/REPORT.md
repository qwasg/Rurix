# G36 留窗「FG × 画质臂组合面」侦察 + 判档 + 合入提案 — G37 W3 交付

- 日期:2026-08-30(G37 商业化收官战役 W3 深化子任务;G36 五留窗之「FG 组合归后续波不预支」行)
- 修改域:**仅本交付物目录**(`artifacts/day_0830_delivery/w3_deep/fg_combo/`)——窗口 bin /
  lane_body / kernels / SPV / milestones / registry / ci 全部 0-byte(侦察只读)
- 判档:**可组合——需接线适配(轻量,零新 kernel,零共享面触碰),非纯解字面,非 no-go**
- 纪律核验:未跑 GPU;未 `cargo build --release`;未碰 target-night;`g31_window_present.rs` /
  `g14_3_lane/g14_3_lane_body.rs` 只读未改;验收脚本 GPU 步骤写好不跑(默认 plan 模式)

---

## 1. 侦察结论(锚)

### 1.1 FG 臂输入形态——任务书核心问题的答案

**FG kernel 吃的既不是 presented BGRA8,也不是 raw 场景 pass 输出——是 display encode 之前的
线性 f32 TSR 输出(`U_OUT_COLOR` parity 双缓冲,3 f32/px @输出分辨率)+ 相机 MV
(`U_MV_OUT` 经 `g31_mv_negate` glue 逐元素 IEEE 取反)。FG 输出 f32 中间帧再走独立的
`g31_display_encode_fg1/_fg2` pass(复用主 encode 同一 SPV 字节 + 同一 `G31_U_ENC_PARAMS`
buffer)打包 BGRA8 供 present。**

证据锚(`src/rurix-render/src/bin/g31_window_present.rs`,行号随并行会话漂移,以内容锚为准):

| 锚 | 事实 |
|---|---|
| `g31_lane_descs` fg 块 pass 绑定 | `g26_framegen_fg1` 绑 `[U_OUT_COLOR[1], U_OUT_COLOR[0], G31_U_MVN, G31_U_FG1_PARAMS, G31_U_FG1_OUT]`;`g31_display_encode_fg1` 绑 `[G31_U_FG1_OUT, G31_U_ENC_PARAMS, G31_U_FG1_BGRA]`(`spirv: enc_spv` 与主 encode 同字节) |
| `kernels/g26_framegen.rx` 签名 | `(tc, prev: View<f32>, cur: View<f32>, mv: View<f32>, params: View<f32>, out_color: ViewMut<f32>)`——**纯图像空间 compute,无 AccelStruct,不感知场景结构**;prev/cur 3 f32/px 行主序、mv 2 f32/px、params 16 f32(`[3]=t` host 逐帧算好传入) |
| prepare_update fg override | 逐帧换 `[U_OUT_COLOR[1−p], U_OUT_COLOR[p], G31_U_MVN, …]`(prev=上帧 parity 槽,cur=本帧;≡ host `interpolate(prev, cur, −mv, t)` 逐字同语义) |
| MV 来源 | `U_MV_OUT`(g14_mv 相机 MV pass,unified lane 基座)→ `g31_mv_negate` → `G31_U_MVN`;`--fg` 闭集钉 `--tier 100` 保证 MV 与 out_color 同栅格 |
| 双口径登记 | `real_render_frame_ms/real_render_fps` 只由真渲帧构成;`presented_fps = presented ÷ (real_render_seconds + present_seconds)` 独立口径;`stats.fg_gpu_ms` 与 `stats.render5_gpu_ms` telemetry 按 pass 名分列;`digest_seq` = **逐真渲帧** BGRA8 sha256,「fg on/off 同轨迹位级一致」为不回污染机核门(ci/g31_framegen_present_smoke.py 四面门在案) |

### 1.2 互斥矩阵字面全集(9 处 fail 门,其中 5 处涉画质臂)

CLI 校验段含 `fg != G31Fg::Off` 的 fail 门全集(内容锚 = 讯息字面,文件内唯一):

| # | 门 | 拒跑字面 | 当时理由 | 本判档 |
|---|---|---|---|---|
| 1 | `--textures on` | `--textures on 与 --fg 互斥（B4 接线面 = 生产五 pass 现状车道;FG 组合面非本任务口径,如实拒跑不冒充）` | 未验(口径外) | **解除(随 full 预设)** |
| 2 | `--smooth-normals on` | `--smooth-normals on 不与 --fg/--hzb on/--svt on/--slab-table 同跑（组合面未接线,fail-closed）` | 未验 | **解除(随 full)** |
| 3 | `--bloom on` | `--bloom on 不与 --fg/--hzb on/--svt on/--slab-table 同跑（组合面未接线,fail-closed）` | 未验 | **解除(随 full,需 §3 适配)** |
| 4 | `--auto-exposure on` | `--auto-exposure on 不与 --fg/--hzb on/--svt on/--slab-table 同跑（组合面未接线,fail-closed）` | 未验 | **解除(随 full,零适配)** |
| 5 | `--tsr-quality on` | `--tsr-quality on 不与 --fg/--hzb on/--svt on/--slab-table 同跑（hzb 车道 prepare 路 tsr_params[19..21) 未接线;其余组合面未接线,fail-closed）` | 未验(fg 项) | **解除(随 full,零适配)** |
| 6 | `--hzb on` | `--hzb on 与 --fg 互斥（B1 接线面…）` | hzb 重排 pass 图 | 维持(§6) |
| 7 | `--slab-table` | `--slab-table 与 --fg 互斥（B3 接线面…）` | 未验且无消费方 | 维持留窗(§6) |
| 8 | `--lut 非 off` | `--lut 非 off 不与 --fg/--hzb on/--svt on/--slab-table 同跑…` | lut 不入 full | 维持留窗(§6) |
| 9 | storm/fault 族 | `{arm} 与 --fg/--hzb on/--slab-table/--svt on 互斥（C4 登记面…）` | 诊断臂口径隔离 | 维持 |

另:`--fg` 自身闭集(须 `--auto-move`、须 `--tier 100`、`frames+warmup ≥ 2`、与
`--headless-smoke` 互斥)全部**维持**;`--svt on` 已被 heap 形态整体拒跑(fg 项不可达);
realism 臂族(metal-f0/rt-ao/soft-shadows/rt-reflect/gi2-tex/normal-maps/transparency)与
gi2/emissive-tex/dither/lamp/ggx **无独立 fg 门**——由 textures/smooth-normals 上游门传递
覆盖(解除上游两门即全链放行,无隐藏第二道闸)。

**`--quality full` 预设展开 = 20 臂**(`QUALITY_FULL_EXPANSION` 数组字面,W2 后含
transparency,不含 gi2-ris/gi2-nee/lut),展开于解析层、先于全部臂校验 → 解除上表 #1-#5
五门的 fg 项即打通 `--quality full × --fg` 全路径。

### 1.3 组合矩阵当时的验收范围(milestones/g31/g31_waveb_combo_matrix.json,冻结 2026-08-26)

- 可组合臂 C0–C3(base/textures/slab/hzb)全部 **fg off**;fg 从未进过任何组合验收窗。
- `mutex_rejections` M1/M4/M5/M8 把 fg×hzb、fg×slab、fg 无轨迹、**fg×textures** 钉为
  exit-1 登记,理由字面全部为「**FG 组合面非本任务口径,如实拒跑不冒充**」。
- **结论:互斥是「当时没验」的保守闭集登记,不是结构性不可组合的判决。** 该 JSON 为冻结
  历史登记(只追加纪律),本提案不回改;新组合窗另立登记文件。
- CI 消费面核验:无任何 ci 脚本把 fg×textures 拒跑钉为断言(`g31_hzb_wiring_smoke.py`
  仅文档串提及)——解除互斥不破坏既有 CI。

### 1.4 画质臂对 FG 输入形态的影响(逐臂判)

FG 输入形态 = ①`U_OUT_COLOR` parity 双缓冲(3 f32/px、逐帧轮换)②`U_MV_OUT`(2 f32/px
同栅格)③encode 参数面。逐臂核验画质臂是否破坏这三者:

| 臂 | 触碰面 | 对 FG 输入的影响 | 判 |
|---|---|---|---|
| textures / smooth-normals / ggx / lamp-lights / gi2 / gi2-clamp / emissive-tex / metal-f0 / rt-ao / soft-shadows / rt-reflect / gi2-tex / normal-maps / transparency | 仅 scene pass(pass[0] 换 kernel + 尾挂侧表资源) | 只改 out_color **内容**;parity 双缓冲结构、MV pass、栅格全部 0 改动 | **正交** |
| tsr-quality | 仅 tsr_params[19..21)(resolve 行为) | 同上,内容变化 | **正交** |
| dither | encode SPV/参数面 | enc_fg1/enc_fg2 复用同 SPV 同参数 → 生成帧同治;**现状已非互斥**(无 fg 门) | **正交(已开放)** |
| auto-exposure | 尾挂 reduce/state 两 pass;state 在 **device 侧把增益写 `G31_U_ENC_PARAMS[133]`**(`g31_apply_autoexp` 注释字面「增益经 params[133] 消费,零新增绑定」) | 主 encode 与 enc_fg1/enc_fg2 绑定**同一** ENC_PARAMS buffer,state pass 先于全部 encode pass 执行 → 生成帧自动继承本帧(cur)增益。语义登记:生成帧插值于 prev/cur 之间但用 cur 增益——AE 为慢收敛 EMA(α=0.02),帧间增益差可忽略,如实登记不适配 | **正交(零适配)** |
| **bloom** | encode 输入**重挂**:`bindings_encode` 从 `U_OUT_COLOR[p]` 换 `G31_U_BLOOM_COMP_OUT`(合成缓冲,**单缓冲静态绑定**) | **唯一形态耦合**:真实帧 presented = encode(comp),FG 若仍插值 out_color,生成帧 = encode(fg(pre-bloom)) **无辉光** → x2 下 50% presented 帧无 bloom 交替闪烁,不可接受;且 comp 单缓冲无上帧历史,FG 无法直接改吃 | **需适配(§3)** |

### 1.5 结构性障碍——为什么不是「解除字面」一行事

1. **资源下标正面冲突**:FG 追加资源写死 `24..=31`(base 变体 `G31_U_RESOURCE_COUNT=24`
   之后),而 full 终态变体(tex+nrm+bloom+em+real+nm+transp)资源到
   `G31_U_RESOURCE_COUNT_TEXNRM_BLOOM_TRANSP = 44`,AE 三件再占 `44..=46`。
2. **屏障计划编译期定死**:`G31Descs.barriers: Vec<&'static [(u32, TargetState)]>` 强制
   计划为 `&'static` 数组 → FG×full 需**新静态下标族常量 + 计划族**(repo 既有「变体族
   静态下标 + assert 资源表连号」模式;AE 红修 #2 后 debug_assert 已升 assert,G37 W1)。
3. **parity override pass 下标写死**:fg1/fg2 override 钉 pass 6/8(base 六/八 pass 图);
   full+AE 图为 11 pass(scene/mv/resample/resolve/bright/blurH/blurV/composite/reduce/
   state/encode),FG 尾挂后 fg1 在 12、fg2 在 14 → 需变体感知字段。
4. **AE 摘出断言的顺序约束**:`g31_apply_autoexp` 断言「变体末位 pass ==
   `g31_display_encode`」(注释明说「fg/hzb 组合 CLI 已裁不达——防御性复核」)→ FG 必须
   在 AE 变换**之后**尾挂(先 AE 重挂 encode,再 FG 追加;顺序天然可满足,断言零触碰)。
5. **evidence 分支序**:evidence 组装与 `evidence_path` 默认值均为 if-else 链,`textures`
   分支在 `fg` 之前 → 组合跑会落 textures 面、丢失 FG 双口径登记 → fg 分支须前移。
6. **wired-parity 对拍面**:probe 帧 host 复算读 out_color 对(readback 0/1);bloom-comp
   适配后 host `interpolate(prev, cur, −mv, t)` 的 prev/cur 须换 comp 对回读。
7. **readback 布局**:`G31FgLayout::of` 按 base 变体 readback 序(BGRA=4 后 FG 块从 5 起)
   写死——full 变体恰好同序(tex/nrm/bloom/AE 均不加逐帧 readback,仅 svt 加且已拒),
   但 comp 对回读进 FG 块后布局仍需 full 族版本。

---

## 2. 判档:可组合——需接线适配

### 2.1 判据链

1. `g26_framegen.rx` 是**纯图像空间 kernel**(五绑定,场景无关):输入形态由车道保证,
   §1.4 证明 20 臂中 19 臂不破坏形态 → 结构上可组合,当年互斥 = 保守闭集(§1.3 字面自证)。
2. 唯一语义耦合 = bloom 的 encode 输入重挂。适配 = **comp 缓冲 parity 双缓冲化**(§3.1),
   零新 kernel、零共享面触碰、真实帧数值逐位不变 → 「fg on/off digest_seq 位级一致」不
   污染机核门**保持可证**。
3. 商业语义对齐:适配后 FG 插值 post-bloom(post-process 之后、display encode 之前)图像
   ——与 DLSS-G/FSR-FG 的「对最终合成帧插值」同形态;screen-space 辉光被场景 MV warp 属
   商业 FG 已知近似,如实登记。
4. AE/tsr-quality/dither 零适配即正确(§1.4)。

### 2.2 组合闭集裁决:两点式,不开全矩阵

fg 合法形态 = **{全画质 off(既有 base 面,0-byte)} ∪ {`--quality full` 预设字面}**。

- 散臂混搭(如 `--textures on --fg x2` 不带 full、`--quality full --fg x2 --gi2-ris`)
  **维持 fail-closed**:每个变体形态需独立 FG 静态下标族,开放 2^N 组合 = 下标族爆炸,
  正是 AE 红修 #2(release 下下标错位把 tri_base 当 state 写)的事故几何;full 是唯一
  生产预设,散臂组合无消费方。
- 判别式用 `quality_full` 解析层布尔(预设与显式旗标重叠已被 dup 检查拒跑,布尔即闭集
  证明);微调需求走「弃 fg 或弃预设」,与既有 `--quality full 与显式旗标冲突` 纪律同律。

### 2.3 组合面的口径不变量(验收臂断言集,§4 脚本兑现)

1. **双跑位级**:combo 臂两跑 `digest` + `digest_seq` 逐帧位级一致。
2. **不回污染**:`digest_seq(full+fg x2) == digest_seq(full, fg off)`(真渲帧位级;
   comp parity 适配只换缓冲对象不换数值 → 结构上可证,真跑复核)。
3. **画质真实生效**:`digest_seq(full) ≠ digest_seq(all-off)`(防「组合绿但画质没开」冒充)。
4. **presented 计数恒等式**(独立重算不信旗标):`presented == real + generated`、
   `generated == real×inserted − (warmup==0 ? inserted : 0)`、
   `window.frames_presented == 1 + (total−1)×factor`、`len(digest_seq) == frames+warmup`。
5. **real fps 口径隔离**:`real_render_fps == real/real_render_seconds` 重算、
   `presented_fps == presented/(real_render_seconds+present_seconds)` 重算、
   `stats.fg_gpu_ms`/`stats.render5_gpu_ms` 分列存在;`caliber_identities` 五旗标恒 true。
6. **接线态对拍**:`wired_parity.excess == 0`(逐像素 L1 结构界)、`mvn_max_abs_plus_mv == 0`、
   `ssim_beats_frame_hold == true`、`frozen_floor` == G26 budget
   `g26.framegen_device.host_device_maxdiff_tol` 程序读(禁手写)。
7. **presented 帧率独立登记维持**:FG evidence schema v1 字段闭集不变(组合面零新字段,
   `ci/g31_framegen_present_smoke.py` 的 REQUIRED_KEYS 互核继续绿)。

---

## 3. 适配层设计(bloom-comp parity;组合接线的唯一新机制)

### 3.1 comp parity 双缓冲(fg-on 面才构造,off 面 0-byte)

- fg×full 面为 `G31_U_BLOOM_COMP_OUT` 追加同尺寸伙伴缓冲 `comp[1]`(opc×12 B),二者组
  parity 对(下标见 §3.2 表);**composite 写 `comp[p]`、encode 读 `comp[p]`、AE reduce 读
  `comp[p]`、FG 读 `(comp[1−p], comp[p])`**——四处均为 prepare_update 逐帧 binding
  override(composite/encode 两处 override 已存在,改绑定内容;reduce 原静态绑定,fg 面
  新增 override;FG 两 pass override 已存在,换 prev/cur 槽位)。
- **零新 kernel**:全部既有 SPV 换绑定即可(bright/blur/composite/encode/reduce/state/
  mvn/fg/enc_fg 九类 pass 字节 0-byte)。
- **真实帧数值逐位不变**:同 kernel 同输入,仅输出缓冲对象逐帧轮换 → 真渲帧 BGRA 位级
  与 fg-off(单 comp 静态)一致 ⇒ §2.3-2 不污染门结构上成立。
- 首帧/era 首帧:`gen_active == false`(无 prev 真渲帧对),FG pass 照跑但输出不读不
  present(既有纪律「FG pass 仍随固定图执行但输出面不读不 present」),comp[1−p] 未定义
  内容零消费——与 base 面 U_OUT_COLOR 首帧同律,零新风险。
- probe 帧对拍:host `interpolate(prev, cur, −mv, t)` 的 prev/cur 换 comp 对回读
  (readback 布局 §3.2);判据面(L1 结构界/SSIM/MVN 位级)公式零改动——对拍的是
  「FG kernel 的输入→输出契约」,输入是什么缓冲不改变契约本身。

### 3.2 FULL FG 静态下标族(TEXNRM_BLOOM_TRANSP + AE 终态之后连号)

资源(`assert_eq!(d.resources.len(), 47)` 施加期钉死,红修 #2 律):

| 常量 | 值 | 内容 |
|---|---|---|
| `G31_U_BLOOM_COMP_HIST_FULL` | 47 | comp parity 伙伴缓冲(opc×12) |
| `G31_U_MVN_PARAMS_FULL / G31_U_MVN_FULL` | 48 / 49 | MV 取反参数 / 输出(opc×8) |
| `G31_U_FG1_PARAMS_FULL / G31_U_FG1_OUT_FULL / G31_U_FG1_BGRA_FULL` | 50 / 51 / 52 | FG1 三件 |
| `G31_U_FG2_PARAMS_FULL / G31_U_FG2_OUT_FULL / G31_U_FG2_BGRA_FULL` | 53 / 54 / 55 | FG2 三件(x3 才构造) |

pass 图(full+AE 11 pass 之后尾挂):mvn=11、fg1=12、enc_fg1=13、(x3)fg2=14、enc_fg2=15。
屏障计划族六件(`_FULL` 后缀;FG1/FG2 计划读 `COMP_OUT`+`COMP_HIST` 对而非 U_OUT_COLOR 对;
composite 计划换含 comp 两槽的超集版)。

readback 布局(base 5 件后连号;`G31FgLayout` 增 full 族构造):

| 序 | x2 | x3 |
|---|---|---|
| 5/6 | comp[0] / comp[1](probe 帧才入子集) | 同左 |
| 7 | fg1_bgra | fg1_bgra |
| 8 | fg1_out | fg2_bgra |
| 9 | mvn | fg1_out |
| 10/11 | — | fg2_out / mvn |

### 3.3 evidence 归属

组合跑落 **FG evidence 面(schema `rurix.g31.framegen_present_evidence.v1` 原样)**:
evidence 组装链与 `evidence_path` 默认链中 fg 分支前移至 textures 之前(fg on ⇒ 非
hzb/svt/slab,前移安全)。理由:组合验收的机核 = FG 双口径与不污染门;textures/realism 各
臂 parity 门已由各自单臂窗绿件在案,组合窗不重验。schema 零新字段 ⇒ milestones 0-byte、
`ci/g31_framegen_present_smoke.py` 0-byte(其 REQUIRED_KEYS/schema 互核继续成立)。

---

## 4. 窗口 bin 修改提案(内容锚 + 插入文本;共 16 锚)

> 本任务不改 bin;下表为获批执行会话的施工单。行号一律不给(并行会话在改此文件,侦察期间
> 已观测行号漂移)——**锚 = 文件内唯一内容串**。A 组 6 锚为判档字面(独立可合);B 组 8 锚
> 为接线(依赖 A);C 组 2 锚为登记面。

### A 组:判档字面(6 锚)

| # | 锚(唯一检索串) | 操作 |
|---|---|---|
| A1 | `--textures on 与 --fg 互斥（B4 接线面` | 条件 `if fg != G31Fg::Off {` → `if fg != G31Fg::Off && !quality_full {`;讯息追加`（fg×画质组合面经 --quality full 预设字面开放——两点式闭集,散臂微调组合仍拒,G37 W3 fg_combo 判档）` |
| A2 | `--smooth-normals on 不与 --fg/--hzb on/--svt on/--slab-table 同跑` | 条件 `fg != G31Fg::Off \|\|` → `(fg != G31Fg::Off && !quality_full) \|\|`;讯息同律追加 |
| A3 | `--bloom on 不与 --fg/--hzb on/--svt on/--slab-table 同跑` | 同 A2 律 |
| A4 | `--auto-exposure on 不与 --fg/--hzb on/--svt on/--slab-table 同跑` | 同 A2 律 |
| A5 | `--tsr-quality on 不与 --fg/--hzb on/--svt on/--slab-table 同跑` | 同 A2 律 |
| A6 | `// A5 FG 闭集约束（fail-fast 如实拒跑,不静默降级）。` | 块内追加两点式闭集卫兵:`if quality_full && (gi2_ris \|\| gi2_nee) { fail("--fg × --quality full 组合面闭集 = full 预设终态形态（W2 ris/nee 臂不入 full;FG_FULL 下标族按 TEXNRM_BLOOM_TRANSP+AE 终态定死,越形态拒跑不冒充）"); }`(lut/storm/hzb/slab/svt/headless 各有自身 fg 门,不重复) |

### B 组:接线(8 锚)

| # | 锚 | 操作(结构草案) |
|---|---|---|
| B1 | `const G31_U_AE_PARTIALS_TEXNRM_BLOOM_TRANSP` 常量组之后 | 插入 §3.2 下标族 9 常量 + 屏障计划族 6 件(`_FULL`;FG1/FG2 计划读 comp 对;composite 超集计划含 comp[0]/comp[1] 两槽) |
| B2 | `g31_apply_autoexp` 函数体之后 | 插入 `fn g31_apply_fg_full(d: &mut G31Descs, fga: &G31FgAssets, opc: u64)`:`assert_eq!(d.resources.len(), 47, "g31_apply_fg_full: FG_FULL 下标族须 == TEXNRM_BLOOM_TRANSP+AE 终态资源数（错位即红修 #2 症状）")` → 追加 comp[1] + mvn/fg 资源(x3 才加 FG2 三件)→ 尾挂 mvn/fg1/enc_fg1(/fg2/enc_fg2) pass(初始绑定 parity 0 形,enc_fg 绑 `G31_U_ENC_PARAMS` 复用主 encode SPV)→ 屏障计划族 → readbacks 按 §3.2 序 |
| B3 | `let mut off_descs = if hzb != G31Hzb::On {` 调用中的 `fg_assets,` 实参 | 分流:`if textures { None } else { fg_assets }`(full 面 base descs 不再内嵌 FG;base 面 0-byte) |
| B4 | `// A2:对将被选中的变体描述组施加 autoexp 变换` 块尾 | 追加施加点:`if fg != G31Fg::Off && quality_full { g31_apply_fg_full(tex_descs.as_mut().unwrap_or_else(...), fg_full_assets, opc); }`(必在 AE 之后——AE 摘出断言前提「末位 pass = encode」由顺序保证) |
| B5 | `fg_layout: G31FgLayout::of(fg),`(lane struct 构造) | struct 增 `fg_full: bool` + `fg_pass_fg1: u32 / fg_pass_fg2: u32`(base = 6/8,full = 12/14,构造期由 descs pass 数派生);`G31FgLayout` 增 full 族构造(§3.2 readback 表) |
| B6 | `// D3 九 pass 图:4=bright/5=blurH/6=blurV/7=composite/8=encode` override 块 | fg_full 面:composite override 出口换 `comp[p]`、encode override 入口换 `comp[p]`、新增 reduce(idx 8)override 入口 `comp[p]`(fg off 面三处零改动,静态 `G31_U_BLOOM_COMP_OUT` 现状 0-byte) |
| B7 | `// A5:fg pass parity override——取反 glue 直通馈入` 块 | pass 下标 `6`/`8` 字面 → `self.fg_pass_fg1`/`self.fg_pass_fg2`;fg_full 面 prev/cur 绑定 = `comp[1−p]`/`comp[p]`(base 面维持 U_OUT_COLOR 对,0-byte) |
| B8 | probe 子集段 `if probe {`(fg_layout 消费处)+ `rec.probe_prev_color` 消费点 | fg_full 面 probe 子集追加 comp 对下标(5/6 按 parity 选),`probe_prev_color/probe_cur_color` 从 comp 回读解析;host `interpolate` 复算调用零改动 |

### C 组:登记面(2 锚)

| # | 锚 | 操作 |
|---|---|---|
| C1 | evidence 组装链 `} else if fg != G31Fg::Off {`(A5 分支)与 `evidence_path` 默认链 `"evidence/g31_framegen_present.json"` | fg 分支前移至 textures 分支之前(两处);FG notes 字面追加组合面描述(「fg×--quality full 组合面:FG 插值 post-bloom comp parity 对,AE 增益经 enc_params[133] 同读继承,digest_seq 不污染门跨 full on/off 维持」) |
| C2 | `"→mv_negate→fg1→enc_fg1（八 pass,fg x2）"` eprintln 字面 + 文件头 usage `[--fg <off\|x2\|x3>` 行与互斥登记注释段 | pass 链字符串扩组合形态(`full+fg x2 = 十四 pass`/`x3 = 十六 pass`);usage/头注登记两点式闭集与 comp parity 语义 |

**执行序**:B1→B2→B5→B3→B4→B6→B7→B8→A1..A6→C1→C2(先备好接线再开闸,任一中间态
`cargo check` 必绿;A 组开闸前 fg×full 不可达,接线死代码期零行为变化)。

---

## 5. 合入提案

1. **PR 形状**:单 PR 三 commit——①B 组接线(死代码期,`cargo check` + clippy 绿)
   ②A 组开闸 + C 组登记 ③验收窗产物(evidence + 本目录判读输出)。
2. **验收窗(获批后独占 GPU 会话)**:跑 §7 命令;绿件登记
   `artifacts/day_0830_delivery/w3_deep/fg_combo/ACCEPTANCE_SUMMARY.json`(本脚本
   `--judge` 产出)+ 组合窗登记文件(新文件,不回改 g31_waveb_combo_matrix.json)。
3. **锚治理**:combo 臂 presented 锚 = 二进制绑定锚(E1 教训「任何重建后整批重收割」);
   验收窗与 DEFAULT_FLIP_PLAN §1.2 encode v2 收编同窗执行可省一次重收割(推荐,非阻塞)。
4. **消费面同步**:DEFAULT_FLIP_PLAN §2.2「残余互斥 = fg/hzb/slab/svt」在 fg 项解除后
   过时——翻转执行会话把 fg 移出残余集(历史文档不回改,翻转登记面新写)。
   `ci/g31_framegen_present_smoke.py` 与 milestones schema **0-byte**(§3.3)。
5. **回滚**:一级 = A 组 6 锚逆向(fg×full 回到拒跑,B 组死代码留树零行为);二级 = 整 PR
   revert。comp parity 为 fg-on 面才构造 → fg off 生产路径任何时刻 0-byte,回滚零锚漂移。

## 6. 不做面(留窗,结构证据)

| 面 | 证据 | 处置 |
|---|---|---|
| fg×hzb | hzb 变体重排前四 pass 为剔除闭环多 pass 图,`G31FgLayout`/override 下标全不同族,且 hzb on ≈ 4.7×base 帧时(combo matrix C3)叠 FG 无商业收益 | 维持互斥 |
| fg×slab | slab 改装配期材质侧表,形态其实正交,但 slab 臂无 full 预设消费方 | 维持留窗(后续波一并入 full 时再判) |
| fg×svt | svt 与 heap 纹理形态本身互斥(`--svt on` 整体拒跑),fg 项不可达 | 随 svt 深修留窗 |
| fg×lut | lut 不入 full 预设;lut 段在 encode 链工件内,若并入 full 则 enc_fg 同 SPV 自动同治——届时仅解字面 | 留窗登记 |
| fg×散臂微调 | §2.2 两点式闭集裁决(下标族爆炸 = 红修 #2 事故几何) | fail-closed 维持 |
| 运动物体 MV 缺口 | A4 已登记项(MV 仅相机运动+静态深度重投影);full 预设不引入动态实例,组合面继承原语义 | 如实登记不冒充 |

## 7. 验收命令

```powershell
# 判读器自证(无 GPU,本会话可跑):
py -3 artifacts/day_0830_delivery/w3_deep/fg_combo/accept_fg_combo.py --selftest
# 打印 GPU 步骤(默认 plan 模式,不执行——本任务纪律禁跑 GPU):
py -3 artifacts/day_0830_delivery/w3_deep/fg_combo/accept_fg_combo.py --plan
# 获批验收窗(独占 GPU + release 构建后)真跑五臂 + 互斥矩阵:
py -3 artifacts/day_0830_delivery/w3_deep/fg_combo/accept_fg_combo.py --execute
# 对已在案 evidence 单独判读(可重入):
py -3 artifacts/day_0830_delivery/w3_deep/fg_combo/accept_fg_combo.py --judge
```

- 交付物:本报告 + `accept_fg_combo.py`(GPU 步骤写好不跑;判读含两跑 digest 位级/
  presented 计数恒等式/real fps 口径隔离/不污染门/画质生效门/wired_parity/互斥矩阵)。
- Assisted-by: Claude(G37 W3 fg_combo 侦察+判档子任务)
