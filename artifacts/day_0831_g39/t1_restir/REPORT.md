# G39 T1 — `--lamp-restir` 点灯直接光高档时域 reservoir 窗口 bin 生产接线(施工稿)

> 2026-08-31。TODO #7(M100 车道集成窗;RFC-0047 §5.5 法定输入 = g30 注册表
> M100-high 行);开窗证据 = G38 lamp-k 阶梯 measured「16 簇贴线/26 簇超线——
> 提灯数须 ReSTIR」(day_0830_g38/CAMPAIGN_LOG L27)。形态 = EVAL_RESTIR §3.2
> **方案 A**(scene 单 pass megakernel 内嵌 temporal RIS,pass 数不变)。
> 本稿 = 子 agent 施工登记:代码 + 新 SPV 已就位(未 commit),**GPU 零跑**
> (off==锚/双跑/A-B 全归主 agent,§六命令清单)。所有修改处注释「G39 T1」。
> 行号 = 施工后快照(g31_window_present.rs 13,152 行 / g31_realism.rx 2,148 行)。

## 一、设计落点

### 1.1 kernel 链位(`kernels/g31_realism.rx`,就地 gate 化追加第 9 链位)

| 件 | 行号 | 内容 |
|---|---|---|
| 头注 params 登记 | L18-22 | `[72..76) 臂⑨ --lamp-restir 门/m_cap/has_history/预留` |
| 头注跨帧面登记 | L40-47 | prev_vp 布局 + reservoir 4 f32/px + parity 轮换 + 首帧门语义 |
| 签名扩展 | L139-144 | `prev_vp: View` / `resv_prev: View` / `resv_cur: ViewMut`(lamp_tbl 之后、out_color 之前 = 第 9 链位;SPIR-V 绑定 21→**24**) |
| 门 + 点灯循环让位 | L728-736 | `lrs_gate = gate(params[72])`;`while pi_ < point_count` → `while pi_ < lrs_ptn`,`lrs_ptn = (1−gate)·point_count`(**唯一被改写的既有行**;臂⑧ `gi_neen ×(1−ris_gate)` 同一先例形) |
| 臂⑨主块 | L929-1195 | 见下述五段 |
| ① M=8 候选 WRS | L968-1014 | 候选下标 = R3 闭式(`lrs_uc`)、判定 = R2 闭式(`lrs_uwr`);phat = `max3(li)·cos_s/d2s·gate_d·gate_cs·gate_lc`(既有点灯 keep_pre 同源,含 params[49] 贡献剔除门,A1 max3 口径);`w = phat·point_count`(host `estimate_ris` w=phat·L 同式);WRS 消费门 = `((w/w_ss − u)·big).min(1).max(0)` 乘法-free 除法比较形(L1456-1462 臂⑧/g28 冻结链同源,零新比较式) |
| ② prev_vp 重投影 | L1015-1035 | g14_mv.rx 机械同式:行 0/1/3 左结合 + `pcw>1e-8` 门 + `prev_u/v` 折算 + floor;界内四门(`(x+1)·big` / `(wf−1−x)·big+1` 整数域精确);读地址钳位,消费经计数门 |
| ③ 时域 merge | L1036-1092 | 计数门 = has_history(params[74])× pcw 门 × 界内 ⇒ 零迭代即零读(首帧/reset 不清 buffer 安全);槽有效门 y∈[0,point_count)∧m>0;`m_cl = min(m_prev, m_cap)`;当前点重算 phat(y) 后 `w_other = phat_new·(w_sum/(phat_prev·m_prev))·m_cl`,`m += m_cl`(host `restir_reservoir.rs::merge` m_cap 截断同义,含 phat_y≤0 ⇒ w_other=0、m 照加分支);merge WRS 判定同比较形 |
| ④ W + 验证射线 + 合成 | L1093-1181 | `W = w_sum/(m·phat_y)` g28 三算术门形(y 空/phat≤0/m=0 ⇒ 0,安全分母);选中灯几何/光度重取(循环逐字同形,A1 灯罩豁免 `t = d − max(2·ray_eps, lr)`);1 条 first_hit 验证射线(if 包 ray query 白名单形,灯循环同构;W=0/几何门=0 零射线);`dir_* += li·geo·kpre·vis·W`、GGX 高光项逐字同形 `spec_* += li·geo·kpre·vis·W·dgf·fresnel`(params[48] 均匀分支) |
| ⑤ reservoir 写回 | L1182-1195 | gate on **全像素恒写**(miss/空池写 y=−1 哨兵 ⇒ 下一帧任意重投影目标读安全);gate off 计数门零写(未来超集工件 off 面占位 buffer 零触达) |

**自洽锚**:M=1 无历史退化时 W = point_count,贡献 = `li·g·keep·point_count`
= 既有「均匀选 1 灯 ×point_count」估计器数学同式(臂⑧同律)。M=8 字面常量;
随机相位 = 黄金比共轭 4×/5×/6×`0.3819660112501051`(臂⑧已用 1×..3×,不撞)
+ 候选序号 ×`0.6180339887498949` 步进 + params[52] 帧旋转,闭式零跨帧 RNG
状态,固定输入双跑位级一致。

### 1.2 跨帧 buffer 布局

| buffer | 大小 | 形态 | 初值 |
|---|---|---|---|
| reservoir ×2 | iw×ih×16B(f32×4 = `[y, w_sum, m, phat_y]`,y=−1 哨兵空) | `device_local: true, data: None`,parity 轮换(纯 device 侧读写零上传) | 不清——首帧 params[74]=0 门零读 + gate on 每帧全像素写回;era 重建重置(TSR hist/AE state 先例) |
| prev_vp | 64B(16 f32 行主序 m[r][c],`pack_mv_params` prev_vp 同布局) | **`device_local: false`**(逐帧 buffer_uploads 目标须 host-visible——B3 首红修复,§八),`data: None` | 首帧/reset 上传当前 vp_j 且 [74]=0 跳过消费 |

@1080p 内部渲染分辨率:reservoir 双份 ≈ 2×31.6 MiB(EVAL §3.1 表字面,可承受)。

### 1.3 params 槽位分配表(纯追加;`G31_RESTIR_PARAMS_LEN = 76`)

| 槽 | 语义 | off 面 |
|---|---|---|
| `[72]` | `--lamp-restir` 门(0/1) | 不扩不写(buffer 恒 72/56 既有面,参数面 0-byte) |
| `[73]` | m_cap(`--lamp-restir-mcap`,默认 8,闭集 [1,64]) | 同上 |
| `[74]` | has_history(`!reset && has_history_state`,TSR tsr_params[8] 同式) | 同上 |
| `[75]` | 预留恒 0(长度取 76 对齐既有 8 倍数节奏) | 同上 |

`[52]`(帧序)既有槽:restir on 且 gi2 off 时由 realism 块补写(rt-ao/soft/refl
同律,条件并入 L4745/L11194)。

### 1.4 binding/资源下标分配表

| 资源 | tex_nrm 形态 | tex_nrm_bloom 形态 |
|---|---|---|
| prev_vp | 37(`G31_U_LRS_PREVVP_TEXNRM`) | 45(`_BLOOM`) |
| reservoir 对 | [38, 39](`G31_U_LRS_RESV_TEXNRM`) | [46, 47] |
| 资源计数 | 40(`G31_U_RESOURCE_COUNT_TEXNRM_RESTIR`) | 48 |
| AE 三件 | 40/41/42(`G31_U_AE_*_TEXNRM_RESTIR`) | 48/49/50 |

kernel 绑定序(= 签名序,22 路 storage + tlas):`[..., lamp_tbl, prev_vp,
resv_prev, resv_cur, out_color, out_depth]`。descs 静态绑定 = parity 0 形
(prev←resv[1]、cur←resv[0]);逐帧 `prepare_update` 对 **scene pass(pass 0)**
push binding_overrides 全量重述绑定列表(含 `accel_structs: vec![0]`),
cur=resv[p]、prev=resv[1−p],`self.parity` 复用 TSR 五对同律(翻转在 `frame()`
既有行 L5375,0-byte)。

### 1.5 依赖集裁决(fail-closed,臂⑧体例)

| 约束 | 裁决依据 |
|---|---|
| 须随 `--lamp-lights on` | 臂的语义对象 = lamp 簇代表灯(开窗证据「提灯数须 ReSTIR」);点灯循环本身无独立 gate,但无代表灯时换选无意义,如实登记为依赖而非放行 |
| 须随 `--smooth-normals on` 且 `--textures on` | g31_realism 链 kernel 仅存在于 tex_nrm 合流臂(EVAL §3.3「首版只接 tex_nrm(+gi2) 合流形态」) |
| 不与 `--fg` 同跑 | **FG_FULL 静态下标族(48..=56)按 TEXNRM_BLOOM_RIS+AE 终态 48 起钉死**(`g31_apply_fg_full` assert L1815);restir 三件尾挂 ⇒ bloom 形态资源数 45→48、AE 至 50,族错位即红修 #2 症状——组合面未接线,CLI 卫兵裁(L8988) |
| `--lamp-restir-mcap` 须随 on,域 [1,64] | 子参数零消费律 + 时域置信截断域 |
| **不进** `QUALITY_FULL_EXPANSION` dup 表 | full19 字面不含之 ⇒ full19 锚零漂;`--quality full --lamp-restir on` 可组合(--lamp-k 同律) |
| 与 `--gi2-ris/--gi2-nee` 正交组合 | restir 级工件 = ris 级超集(全 gate 段在内),restir on 换载 restir 工件,ris/nee gates 照常 params 驱动 |
| hzb/svt/slab/cluster/wp | 经 `--smooth-normals`/`--textures` 上行互斥已裁(传递覆盖,零新增卫兵) |
| `--gi2` **非**依赖 | 点灯直接光循环在主体(非 GI2 循环内),按 kernel 实况如实登记——与臂⑧(须随 gi2)判别 |
| `--soft-shadows`/`--transparency` 可组合不冲突 | restir on 时其点灯阴影效果不进验证射线(§四税表登记,非 fail-closed——full19 含两臂,阶梯矩阵须可跑) |

## 二、窗口 bin 九步逐项落点(`g31_window_present.rs`,行号 = 施工后)

| 步 | 内容 | 行号 |
|---|---|---|
| 1. flag 声明 + 解析 | `lamp_restir`/`lamp_restir_mcap` 声明;`--lamp-restir off\|on` + `--lamp-restir-mcap <usize>` 闭集解析 | L7534-7546 / L7951-7966 |
| 2. fail-closed 校验 | 依赖集(lamp-lights+smooth+tex)/fg 互斥/mcap 随臂 + 域 [1,64] | L8982-8996 |
| 3. SPV 链换载 | 常量 `G31_DEFAULT_SPV_REALISM_RESTIR`(L376);「默认字面才换」梯级追加臂⑧之后,match 集 = 全部 11 个下位默认字面(含 `_RIS`) | L8997-9012 |
| 4. host 侧表(era 外) | restir 并入 `realism_any`(L9026);trinm 回退对/tri_transp 零表/lamp_tbl 80B 哑表三占位条件并入(L9700/L9709/L9743);params buffer 再扩 76×4B(L9770-9773) | 同左 |
| 5. descs 尾挂 + 计数断言 + 屏障 | 两 builder +`lamp_restir: bool` 形参;lamp_tbl 块 assert 改判 + restir 三件尾挂 + `assert_eq!(len, COUNT_RESTIR)`;屏障头分支 `_RESTIR` 最先;两 scene 计划常量(双 parity 并集) | tex_nrm:L3593/L3706-3746/L3765-3770;bloom:L3896/L4007-4048/L4063-4068;计划 L695-718/L858-883 |
| 6. AE 三件族顺延 | `_RESTIR` 六常量(L1380-1385)+ reduce/state 四计划(L1559-1585);`g31_apply_autoexp` match guard 最先(L10424-10443);`set_autoexp` 选择块 guard 最先(L10786-10795) | 同左 |
| 7. 车道挂载 | 字段 `lamp_restir/lamp_restir_mcap`(L4382-4388)+ create 初值(L4482-4483)+ `set_lamp_restir`(L4574-4579)+ 挂载点(L10890-10895) | 同左 |
| 8. prepare_update 逐帧 | realism 扩面门并入(L4686-4687);params[72..75) 写 + [52] 补写(L4728-4746);prev_vp 逐帧上传(源 = `prev` = `prev_vp_j.unwrap_or(vp_j)` 既有变量,L4796-4816);scene pass 0 binding_overrides parity 轮换(L4865-4933) | 同左 |
| 9. kernel gate 化消费 | §1.1;贡献并入既有 `dir_*/spec_*` 合成行 → 帧尾 BGRA8 digest 自动覆盖 | kernel L728-1195 |
| evidence | `quality_arms` 尾追 `lamp_restir`/`lamp_restir_mcap` 两字段(格式串 + 实参 L12236-12241);PASS 行 combo 第 9 段 ` lamp_restir=on lamp_restir_mcap=N`(off = 空串 0-byte,L13089-13094);`spv_texture` path/sha 换载字段自动承载臂登记 | 同左 |
| 帧时记账 | 既有字段自动覆盖(`real_render_frame_ms`/`--profile-json` render_wall/scene_gpu 逐 pass),零新增 | — |

**schema 补丁**:`ci/_patch_g31_window_evidence_schemas.py`(新建,幂等,CRLF
字节面保全)——`milestones/g31/g31_texture_sampling_heap_evidence_schema.json`
quality_arms(`additionalProperties:false` 闭集)**properties-only 纯追加**
`lamp_restir`(boolean)/`lamp_restir_mcap`(integer),**不进 required**
(evidence/ 旧归档件无此两键,check_schemas.py 全量复验保绿)。已跑:补丁
PASS + 二跑幂等 PASS + `py -3 ci/check_schemas.py` **PASS**。

## 三、冻结面 0-byte 声明

| 文件 | 状态 |
|---|---|
| `kernels/g28_restir.rx` | git diff 空(0-byte;WRS 比较形仅作同源形参照) |
| `src/rurix-render/src/gi/restir_reservoir.rs` | git diff 空(0-byte;merge/m_cap/unbiased_weight 语义金标准只读) |
| `src/rurix-render/src/gi/multi_light.rs` | git diff 空(0-byte;fail-closed 恒拒面不解除——本臂为窗口 bin 独立接线,低档 MegaLights 默认档不动) |
| `src/rurix-rt/**` | git diff 空 |
| `g14_3_lane_body.rs` / `g14_3_pipeline_perf.rs` | **本会话零触碰**(工作树中两文件的未提交改动属并行窗 T2/其它 agent,非本臂) |
| 既有 SPV 工件 | sha256 全等锚⑧报告表:`g31_realism_ris.spv` 622a1c33… / `_transp` 35983d0f… / `g31_texture_nrm_gi{,_gi2,_em}.spv` fd22cb19…/75d08aec…/bdd23a3a… |
| 新工件 | `.tmp/night_0831/spv/g31_realism_restir.spv` = **5571e8755357a1e9fbe01acfda952c5eee8b65f1f5d16c3758e4dfb379d38394**(365,724 B;rurixc 内嵌 spirv-val accepted;复编位级 == 自证) |

off 路径改动仅为加性分支闭合(git diff 复核:kernel 唯一被改写既有行 =
点灯循环界 `while pi_ < lrs_ptn`〔gate=0 ⇒ ptn=point_count 数学同值,且 off
面根本不载本 SPV——字节隔离律〕;host 全部被改写既有行 = `|| lamp_restir`
条件并入/assert 改判/guard 前插/格式串尾追,restir=false 时逐字面等价)。
源-工件 divergence 律照旧:g31_realism.rx 现源 = 第 9 链位超集,8 个下位
工件承载各自锚定字节不再复编(w2_wiring REPORT L130 先例)。

## 四、已知税登记(EVAL_RESTIR §9.2/§3.2/§5/§6/§7 对照)

**吃了(按 EVAL 处方执行)**:

1. **prev VP 走新小 SSBO**(§7「prev VP 16+ 槽建议走新小 SSBO」)——64B 逐帧上传;params 只追加 4 槽。
2. **SPV 第 10 工件字节隔离**(§7/C 相纪律)——off 各臂恒载既有锚定字节;新 ray query 站点(验证射线)只存在于新工件。
3. **首版只接 tex_nrm(+bloom/AE)合流形态**(§3.3)——fg 互斥、其余重臂经上行依赖传递互斥。
4. **g28 随机带协议不搬,换闭式 R2/R3**(§6.2:>132MB/帧不可行)——候选/判定/merge 三随机维闭式,黄金比共轭独立相位。
5. **m_cap 取小默认 8**(§5 缓解「8–16 取小」;`--lamp-restir-mcap` 闭集 [1,64] 留调参窗)。
6. **spatial reuse 不做**(§9.1;在案数据 min 0.899 不支持)。
7. **f32 w_sum 精度域**(§6.2)——单像素 m ≤ 8+m_cap 滚动窗有界,g28 device 腿同口径如实登记(非 host f64)。

**没吃(留窗/如实登记)**:

1. **host 镜像对拍臂未建**(§6.2「y 整数锚/p100 对拍证据在新形态下要重建,约半个臂当量」)——本窗验收走 digest 双跑位级 + A/B 方差判读,不含 per-pixel host 复算对拍;留窗登记。
2. **跨像素 merge 的 phat 重算近似**——host 金标准 merge 无偏证明覆盖**同点** merge;跨像素重投影 + 当前点重算 phat(y) 为标准 ReSTIR DI 时域形,m_cap 截断使置信有界,非严格无偏;如实登记(与 §5.1 离散跳变面同源)。
3. **per-pixel phat 归一化钳制未做**(§5 缓解建议第二半句)——v1 由 m_cap=8 小值 + 验证射线承担;若 A/B 出 firefly/闪烁,第一旋钮 = 降 m_cap,第二旋钮留窗。
4. **新税:restir on ⇒ 点灯软阴影让位**——`--soft-shadows` 的圆盘 N 样本不进验证射线(单射线换选形态);full19 组合 A/B 中点灯半影会变硬,判读以 dark ROI 噪声口径为准,亮度/半影差如实登记不判红。
5. **新税:验证射线视玻璃不透明**——`--transparency` 的阴影透明衰减重走段不进本臂(臂⑧ NEE 射线同律);玻璃后点灯影转硬影。
6. **三级反馈耦合(TSR/AE/reservoir)验收未跑**(§5)——归主 agent GPU 窗(§六含 dolly 预案)。

## 五、风险与首红预案

**dolly disocclusion 首红概率大**(EVAL §5.4 原文:重投影失效像素 reservoir
归零重启,1 spp 直接光新露面区裸奔,TSR passthrough 同时失效)。预案:

1. **先查判读器口径再改代码**:A/B 判读 = `ab_metrics.py noise` 尾段静态帧
   (f0064..f0092,默认无 `--auto-move`)——**静态相机下无 disocclusion**,
   主线判读不受 dolly 影响;dolly 属加跑观察臂(`--auto-move dolly`),其
   digest_seq 双跑位级仍必须绿(确定性与画质分离判读)。
2. 若 dolly 观察臂边缘噪声超观感线:登记如实(v1 无 disocclusion 特判——
   界外/pcw 门已拒,深度拒/法线拒留窗),不冒充修复;候选旋钮 = m_cap 降档
   (置信衰减更快)/`--lamp-restir off` 回退(off==锚全链保底)。
3. **era/reset 语义已闭合**:resize era 重建 ⇒ 车道重建(parity=0、
   has_history_state=false、buffer 重建)⇒ 首帧 [74]=0 零读;风暴臂
   (window-storm)与本臂组合未验收,如实登记不冒充。
4. **首帧亮度阶跃**:帧 0 无历史 M=8 单帧 RIS,方差高于稳态;warmup ≥2 帧
   后判读(既有口径已含)。

## 六、GPU 验收命令清单(主 agent 执行;全程 `RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1`,VUID=0 门)

```powershell
# ── 0) 构建窗口 bin(CPU 工序;并行窗占 target 目录会阻塞等待属正常)──
$env:CARGO_TARGET_DIR="H:\rurix\target-night"
cargo build --release -p rurix-render --features vendor-upscale --bin g31_window_present
$BIN = "H:\rurix\target-night\release\g31_window_present.exe"
$env:RURIX_REQUIRE_REAL="1"; $env:RURIX_VK_VALIDATION="1"

# ── 1) off == 锚两跑(全链默认字面零漂;期望锚 = w0_baseline/G39_BASELINE.json 字面)──
# 1a. all-off 8f:期望 presented digest == 55e4a92d25be959a91d111140b88eb81e0c7c29fe80d1aeee641aa78d8a86288
& $BIN --frames 8 --warmup 2 --hidden --quality off --evidence artifacts\day_0831_g39\t1_restir\ev\n1_alloff_8f.json
# 1b. full19 默认(--quality full 缺省即 full,零参数)96f:期望 == a5521e4708a814e364fd3bf95b18f0ab69b6646efd039246e8686533c30e4fb1
& $BIN --frames 96 --warmup 2 --hidden --evidence artifacts\day_0831_g39\t1_restir\ev\n4_full19_96f.json
# (二进制重建后先复验上两锚再消费组合臂——E1 锚纪律;若重建漂移,回滚判读以 w0 双跑为基)

# ── 2) on 臂 r1/r2 双跑位级(EXPLICIT_NOAE 基 = G38 run_ab.py EXPLICIT_NOAE_BASE
#      + ris/nee 两臂〔full19 现含〕+ 本臂;digest r1 == r2 且 VUID=0)──
$env:RURIX_G18_AMBIENT="0.004"
$NOAE = @("--quality","off","--smooth-normals","on","--ggx","on","--lamp-lights","on",
  "--lamp-gain","4","--textures","on","--bloom","on","--dither","on","--tsr-quality","on",
  "--gi2","on","--gi2-clamp","0.01","--emissive-tex","on","--metal-f0","on","--rt-ao","on",
  "--soft-shadows","on","--soft-shadow-samples","1","--rt-reflect","on","--gi2-tex","on",
  "--normal-maps","on","--transparency","on","--gi2-ris","on","--gi2-nee","on")
& $BIN @NOAE --lamp-restir on --frames 96 --warmup 2 --hidden --evidence artifacts\day_0831_g39\t1_restir\ev\on_r1.json
& $BIN @NOAE --lamp-restir on --frames 96 --warmup 2 --hidden --evidence artifacts\day_0831_g39\t1_restir\ev\on_r2.json
# 附加正交抽查:mcap 微调 + 无 ris/nee 最小组合(哑表占位链位)各一跑
& $BIN --quality off --smooth-normals on --textures on --lamp-lights on --lamp-restir on --lamp-restir-mcap 16 --frames 96 --warmup 2 --hidden --evidence artifacts\day_0831_g39\t1_restir\ev\on_min.json

# ── 3) A/B 阶梯矩阵(12/26/~48 簇 × off/on;R1 run_kladder 口径:提簇档 =
#      env RURIX_G31_LAMP_GRID_M + --quality full --lamp-k K〔均不进 dup 表可组合〕;
#      本臂 --lamp-restir 同不进 dup 表可与 full 直接组合)──
# 3a. ~48 簇档 grid 标定(先行;kept 随 grid 收细,0.15→26 在案,预扫 0.10/0.075/0.05,
#     取 stderr「lamp-lights 提取 emissive_tris=… clusters=… kept=…」kept≈48 之档)
foreach ($g in "0.10","0.075","0.05") {
  $env:RURIX_G31_LAMP_GRID_M=$g
  & $BIN --quality full --lamp-k 96 --frames 4 --warmup 0 --hidden --evidence artifacts\day_0831_g39\t1_restir\ev\cal_$g.json 2>&1 | Select-String "lamp-lights 提取"
}
Remove-Item Env:RURIX_G31_LAMP_GRID_M
# 3b. 矩阵六跑(每档 off/on 各一;dump 口径 = run_ab.py 同形:96f warmup2 dump-every 4;
#     GRID_48 = 3a 标定值)。off 臂 = full 裸(+env);on 臂 = + --lamp-restir on。
#     s1 12 簇:env 缺席 + 缺省 full19(off 臂即 1b 锚跑可复用)
& $BIN --lamp-restir on --frames 96 --warmup 2 --hidden --dump-present-raw artifacts\day_0831_g39\t1_restir\ab\k12_on\p.raw --dump-present-every 4 --evidence artifacts\day_0831_g39\t1_restir\ab\k12_on\ev.json
& $BIN --frames 96 --warmup 2 --hidden --dump-present-raw artifacts\day_0831_g39\t1_restir\ab\k12_off\p.raw --dump-present-every 4 --evidence artifacts\day_0831_g39\t1_restir\ab\k12_off\ev.json
#     s5 26 簇:grid 0.15 + k 48
$env:RURIX_G31_LAMP_GRID_M="0.15"
& $BIN --quality full --lamp-k 48 --frames 96 --warmup 2 --hidden --dump-present-raw artifacts\day_0831_g39\t1_restir\ab\k26_off\p.raw --dump-present-every 4 --evidence artifacts\day_0831_g39\t1_restir\ab\k26_off\ev.json
& $BIN --quality full --lamp-k 48 --lamp-restir on --frames 96 --warmup 2 --hidden --dump-present-raw artifacts\day_0831_g39\t1_restir\ab\k26_on\p.raw --dump-present-every 4 --evidence artifacts\day_0831_g39\t1_restir\ab\k26_on\ev.json
#     ~48 簇:GRID_48 + k 96(标定值代入)
$env:RURIX_G31_LAMP_GRID_M="<GRID_48>"
& $BIN --quality full --lamp-k 96 --frames 96 --warmup 2 --hidden --dump-present-raw artifacts\day_0831_g39\t1_restir\ab\k48_off\p.raw --dump-present-every 4 --evidence artifacts\day_0831_g39\t1_restir\ab\k48_off\ev.json
& $BIN --quality full --lamp-k 96 --lamp-restir on --frames 96 --warmup 2 --hidden --dump-present-raw artifacts\day_0831_g39\t1_restir\ab\k48_on\p.raw --dump-present-every 4 --evidence artifacts\day_0831_g39\t1_restir\ab\k48_on\ev.json
Remove-Item Env:RURIX_G31_LAMP_GRID_M
# 3c. 判读:artifacts/day_0829_realism/tools/ab_metrics.py noise 子命令(尾段
#     f0064..f0092 恰 8 张,BGRA→RGB /255 逐像素跨帧 std → mean/p95;四 ROI
#     wall/floor/dark_arch/dark_table;verdict = dark 两 ROI min 的 p95 shrink_pct)。
#     判据预期:12 簇档 on 臂方差可能持平或劣化(16 盏逐盏全算本零方差,EVAL §0
#     已预告——如实登记);26/~48 簇档 on 臂帧时显著低于 off 逐盏线(见 3d)且
#     dark ROI 噪声受 TSR 时域收敛控制。亮度口径:soft-shadows 半影让位 +
#     玻璃硬影两税(§四)致 |Δmean| 非零,登记幅度不判红。
# 3d. 帧时口径:每跑 --profile-json artifacts\...\profile.json(frame_segments
#     render_wall p50 = R1 交接单口径)或 evidence real_render_frame_ms;对照
#     G38 阶梯 measured:26 簇 off 臂 12.959ms(超线 −1.849),on 臂期望回到
#     11.11ms 预算内(换选 = 每像素 1 条点灯阴影射线,与簇数解耦)。

# ── 4) dolly 观察臂(disocclusion;digest_seq 双跑位级 + 观感登记,不进主判)──
& $BIN @NOAE --lamp-restir on --auto-move dolly --frames 240 --warmup 2 --hidden --evidence artifacts\day_0831_g39\t1_restir\ev\dolly_r1.json
& $BIN @NOAE --lamp-restir on --auto-move dolly --frames 240 --warmup 2 --hidden --evidence artifacts\day_0831_g39\t1_restir\ev\dolly_r2.json
```

## 七、偏离登记

1. **quality_arms/PASS 行登记超出臂⑧字面体例**:臂⑧实际零登记(grep 证明
   ——其臂态经 `spv_texture` path/sha 承载);本臂按任务书字面补齐了
   quality_arms 两字段(恒发射,off 跑 evidence JSON 形状加性变化——digest
   锚不涉)+ PASS combo 段(off = 空串 0-byte)+ schema properties-only
   补丁。若主 agent 判定回归臂⑧极简体例,revert 点:evidence 格式串/实参
   两处 + combo 段 + 补丁脚本 + schema 两行。
2. **params 长度取 76 非 75**:任务书三槽(门/mcap/has_history),追加 [75]
   预留恒 0 对齐既有 8 的倍数节奏(56/72/76——76 非 8 倍数但 4 槽整;kernel
   不读 [75])。
3. **kernel 一行既有字面被改写**(`while pi_ < point_count` → `lrs_ptn`):
   「0-byte 只追加」的既定豁免形 = 臂⑧ `gi_neen ×(1−ris_gate)` 让位先例;
   off 臂行为由字节隔离律(不载本 SPV)承载,非 gate=0 恒等。
4. **`--gi2` 不入依赖集**(任务书预期集之外的减项):点灯直接光循环在
   kernel 主体、非 GI2 段,按 kernel 实况如实登记(任务书授权「若点灯直接光
   循环在别的 gate 内,如实按依赖登记」——它不在任何 gate 内)。
5. **`--fg` 互斥为新增卫兵**(任务书未列):FG_FULL 静态下标族 48 起与
   restir bloom 形态资源数 48 撞位,`g31_apply_fg_full` assert 必炸——
   fail-closed 优于运行期 panic;quality_full 对 fg 的上行豁免使传递互斥
   不覆盖此组合,故显式裁。
6. **rurixc 编译旗标实况**:`rurixc <src> --target vulkan -o <out>` 一次过
   (内嵌 spirv-val accepted;与任务书预期字面一致,无偏差)。
7. **验收范围内验证结果**:`cargo check --release -p rurix-render --features
   vendor-upscale --bin g31_window_present` **exit 0、0 error**(warnings 全
   为既有面);SPV 编译 + spirv-val 绿 + 复编位级 ==;`py -3 ci/check_schemas.py`
   **PASS**;git diff 冻结面/禁区零改动(工作树中 lane_body/pipeline_perf/g35
   等未提交改动属并行窗其它 agent,本会话零触碰)。

## 八、B3 首红修复登记(2026-09-01)

**症状**:off 面全绿(all-off/full19 双锚 MATCH、off 阶梯三档 OK);全部
`--lamp-restir on` 跑帧 0 fail-closed(rc=1,VUID=0):
`FrameUpdate.buffer_uploads: StableResourceId(46〔bloom〕/38〔非 bloom〕) 为
DEVICE_LOCAL 驻留(不可 map;上传目标须 host-visible)`。

**报文 id 语义裁决(两假设排查)**:rt 编号规则 = `StableResourceId(n)` 的
资源表槽位 = **n−1**(`render_exec.rs` L2850-2857:`index == 0` 拒、
`resources.get(index − 1)`;上传侧全库惯例 `StableResourceId(idx + 1)`)。
故失败 id 38/46 解码 = 资源下标 **37/45 = prev_vp 本尊**(非 resv[0]):

| 报文 id | −1 解码 | 资源 | 形态 |
|---|---|---|---|
| 38 | 37 | `G31_U_LRS_PREVVP_TEXNRM`(prev_vp) | tex_nrm(五 pass) |
| 46 | 45 | `G31_U_LRS_PREVVP_TEXNRM_BLOOM`(prev_vp) | tex_nrm_bloom(九 pass) |

- 假设 1(上传目标 +1 错位打到 resv[0])**排除**:`+1` 为 rt 编号规则本身
  (U_SCENE_PARAMS 上传同式),目标下标正确;
- 假设 2 前半 **命中**:prev_vp 创建时误带 `device_local: true`(复制
  reservoir 创建形所致——reservoir 纯 device 侧读写,true 正确;prev_vp 为
  逐帧上传目标,须 host-visible,U_SCENE_PARAMS `host_init` 同律
  `device_local: false`);后半(分配表与实况差一)**排除**——§1.4 表与在树
  实况一致,原判读误将报文 id 当资源下标。

**修复字面(最小;两处,均在 `if lamp_restir` 分支内——off 路径字节语义
零触碰)**:`g31_lane_descs_tex_nrm` L3729 与 `g31_lane_descs_tex_nrm_bloom`
L4030 的 prev_vp `BufferDesc` `device_local: true` → **`false`**(reservoir
双缓冲两件 `true` 不动)。kernel 零改动 ⇒ SPV 不重编
(`g31_realism_restir.spv` sha 5571e875… 不变)。

**交叉自检(四面对齐,B3 后复核)**:①下标常量 prev_vp 37/45、resv
[38,39]/[46,47](§1.4 表无修正项);②descs push 序 = prev_vp→resv0→resv1
(L3746-3749/L4045-4048,sb = [pvp, resv[1], resv[0]] parity 0 形);
③binding_overrides 轮换目标 = [pvp, resv[1−p], resv[p]](L4894-4896/
L4920-4922,parity 0 与静态绑定同形);④kernel 签名序 prev_vp→resv_prev→
resv_cur→out_color(L140-145)。四面一致。

**验证**:`cargo check --release -p rurix-render --features vendor-upscale
--bin g31_window_present` **rc=0**;git diff 增量仅两处 `device_local` 字面
+ 注释。
