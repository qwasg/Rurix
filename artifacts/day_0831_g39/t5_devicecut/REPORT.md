# REPORT — G39 T5 段 2:P1 probe-only `--cut-source device` 决策码回读对拍臂(施工登记)

> 日期:2026-08-31。施工图 = 同目录 `DESIGN.md`(逐章消费;GO 圈定 = §6)。
> 事实链 = `artifacts/day_0831_g39/recon/R5_T5.md`。本段零 GPU(C0 selftest
> 为纯 host 腿例外)、零 commit;C1-C5 真跑归主 agent GPU 批(§4 命令清单)。

---

## 1. P1 范围遵守声明

- **文件闭集(两文件加性)**:`src/rurix-render/src/bin/g14_3_lane/g31_frame_cut_arm.rs` + `src/rurix-render/src/bin/g31_frame_cut_probe.rs`。以下面 **0-byte**:`src/rurix-asset/**`(kernel `g31_cluster_cull.rx` 331 行冻结原样 + harness)、`src/rurix-rt/**`、`g14_3_lane_body.rs`(T2 独占)、`g31_window_present.rs`(T1 独占)、rurix-render 侧 `kernels/**`、`ci/**`、`milestones/**`(evidence 走既有 sidecar json,零新 schema 注册)。
- **决策权零移交**:`--cut-source` 缺省 = `host`;缺省路径行为字面 0-byte(`FrameCutArmExtOpt::default_ext()` 补 `false/String::new()/false` ⇒ 窗口臂与既有 probe 调用零行为变;device 臂 = `Option<FrameCutDeviceCtx>` None 短路)。`device` 为加性对拍臂:host 决策权/差集-上传-refit 施加链/既有五判据(双跑 digest 位级/单调门/命中∈已施加 cut/哨兵 canary/零命中防伪)逐字不动,device 平行复算判定码后逐项对拍,mismatch/域外码确定性 FAIL(fail-closed)。
- **P2/P3 未预支**:无施加权移交面、无表驻留 SSBO、无 scatter kernel;`verify_cut_coverage` 生产链 0 改动。
- **kernel 消费形** = DESIGN §2.6:rurixc 现编 SPV 工件 `.tmp/g39_gates/t5_devicecut/g31_cluster_cull.spv`(已产,spirv-val 过)+ `--cull-spv` 运行时装载;NoContraction 注入在 bin 侧装载后进行,不落盘(SPV 文件保持 rurixc 原产字节)。

## 2. 关键落点

### 2.1 中和方案 A 三重防线(DESIGN §2.3/风险 #4)

| 防线 | 落点 |
|---|---|
| ① decisions∈{2,4} 闭集断言(fail-closed) | `frame_cut_device_cut_compare`:逐项先验闭集,域外码打印首破簇号 + 归因口径(0=平面非零/1=cutoff 未关/3=关 4 未短路) |
| ② selftest 中和式 host 复算(锁外常驻) | selftest ⑦:对装配出的 params/表逐簇复算 kernel 判式字面——零平面 `0<−r` 恒假不剔(6 平面×7 簇)/`cone_cutoff=1.0 < 1.0` 恒假关断/view 行全零 ⇒ `near_z=−r<znear(0.1)` 恒短路 |
| ③ red-arm C3(消费路径机核) | `--cut-red-arm tamper`:device 消费的 lod 表构造性篡改 ⇒ 对拍必红(§3 偏离登记:受害裁决期望码驱动) |

中和装配字面:关 1 `params[0..24)=0`;关 2 逐簇 `cluster_f32[fb+7]=1.0`(cone_axis 零填);关 4 `params[36..64)=0` + `params[33]=0.1`(znear 正值);hzb_data=[0.0] 1 texel + hzb_meta=[0,1,1] + levels=1 兜底绑定(短路不读);cap=n ⇒ 原子列表零 overflow;mode=0。

### 2.2 对拍口径钉死声明:**提升前 select 原输出**

- kernel 关 3 对应的 host 面 = `select_lod_cut_grouped` **原输出**;min-level 提升映射是表示层后处理(arm 内 select→verify→promote 序不动)。
- 落点:`frame_cut_select_ext` 返回四元组扩五元组,尾加**提升前逐块布尔集**(在 `verify_cut_coverage`(原 cut)之后、提升之前构造;`min_level==0` 时提升恒等 ⇒ 与提升后 set 同值)。调用点闭集 3 处机械补:`frame_cut_select`(丢弃第 5 元)/`frame_cut_run_session` 帧 0 先行/帧循环段 ①。
- 期望码 = `frame_cut_device_expected`:提升前集 canonical 全局序(块序×簇序,`frame_cut_arena_layout_ext` 同一遍历序)展平,`in_cut ? 4 : 2`。
- ⇒ `--min-level N>0` 组合任意档可对拍(C4);提升映射/二次 verify/竞技场施加链 0 改动。
- 双跑:两遍 session 各自构造 ctx 各自对拍(L1306+ 双跑结构自动覆盖)⇒ device 跨跑一致性经「两跑均与同一 host 金标准全等」传递性成立,免独立断言(DESIGN E4 登记)。

### 2.3 新旗标闭集(probe;全 fail-closed)

| 旗标 | 域 | 校验 |
|---|---|---|
| `--cut-source` | `host`(默认)\| `device` | 域外 FAIL;`device` 且 `--cull-spv` 缺失/文件不存在 = FAIL(显式请求误配置,非 dev_env 三态——vulkan 缺失 skip 三态在前置不动,已核 rc=1) |
| `--cull-spv` | 路径 | 见上;装载时再验 4 对齐 |
| `--cut-red-arm` | ``(默认)\| `tamper` | 非空须 `--cut-source device`;域外 FAIL(三条路径已实测 rc=1) |

stderr 登记行:`G38 T3 旗标 refit_copy=… min_level=… cut_source=…[ red_arm=tamper]`(C1 判据「stderr 登记 cut_source=device」消费此行)。

### 2.4 evidence sidecar 字段清单(schema `rurix.g31.frame_cut_probe.v1` 保持 + 加性)

| 字段 | 位置 | 口径 |
|---|---|---|
| `cut_source` | 顶层(恒出) | `"host"` / `"device"` |
| `device_cut_table_bytes` | 顶层(仅 device 臂) | cluster(10 f32)+lod(8 f32)= 72B/簇 × 消费中块切片簇数(bistro 全量 123,169 簇 = 8,868,168) |
| `device_cut_probe_ms` | 逐帧 | `vk::run_compute` 全程墙钟 measured;**证据税单列,不并入 cut_ms/exec_ms 判读口径**;host 臂 null |
| `device_cut_decisions_sha256` | 逐帧 | 判定码回读字节 sha256(跨跑/跨窗审计面);host 臂 null |

既有字段口径逐字不动;host 臂新字段恒 null/`"host"`(w4_verify.py 等既有消费方无 schema 断言,T3 §7-3 先例)。窗口臂经 `default_ext()` 同样只见 null(digest 面零漂)。

### 2.5 行数账(**超设计量级,登记**)

| 文件 | +/− | 设计估计 |
|---|---|---|
| `g31_frame_cut_arm.rs` | +561 / −12 | ~220(+selftest ~60) |
| `g31_frame_cut_probe.rs` | +52 / −2 | ~40 |
| **合计** | **+613 / −14** | **~300±80(上界 380)** |

**超出原因分解(零功能越界)**:①房规级 doc 注释(逐函数机制/口径/出处注释 ~180 行)②rustfmt 纵向展开(表 extend/元组解构/format 参数逐行)③selftest ⑦ 实做 ~150 行(设计估 60:表/sentinel/params 三关中和复算/期望码/注入器/red-arm 裁决六锚全落)④red-arm 受害裁决 ~35 行(§3 偏离,设计原案 1 行但不可用)。功能面与 DESIGN E1-E5 一一对应,无一项设计外特性;登记后即停手,不再增面。

## 3. 偏离登记(全部如实)

| # | 偏离 | 根据 |
|---|---|---|
| 1 | **red-arm 受害裁决改期望码驱动**(DESIGN E3-4 原案「全局簇 0 self 球半径 +1.0」不落):生产包簇 0 = 叶——`dag.rs` records「叶层在前」、叶 `errors = vec![0.0]`;而 host `select_lod_cut_grouped` 对 `error ≤ 0` 恒 0px **不读球**(`visible_cluster_set.rs` L326-327 字面),kernel 关 3 同式(`if self_e > 0.0` 守卫)⇒ 原案篡改结构性空转,「必红」不可达,C3 恒绿假过 | 实现保持原案形状(lod 球篡改/上传前/施加于 device 消费面/host 期望码不动/fail-closed):模式甲 = 首个期望 4 且 parent_error*∈(0,1e9) 簇,parent 球半径→−f32::MAX(dsurf 饱和 ~3.4e38 ⇒ parent_px→~0 < thr ⇒ 必翻 2);模式乙(甲空)= 首个期望 2 且 self_error<1e9 且 parent_error*>0 簇,self→−MAX(仅 self_e>0)+ parent→+MAX(仅 parent_e<1e9)⇒ 必翻 4。翻转与相机/资产数值无关(结构性);无候选 = fail-closed。selftest ⑦ 两模式裁决锚已落(近帧全叶 cut ⇒ 甲簇 0;远帧根 cut〔parent=sentinel 不可翻〕⇒ 乙簇 0) |
| 2 | `frame_cut_device_cut_compare` 返回 `(f64, String)` 而非设计签名 `f64` | E5 evidence 要求逐帧 `device_cut_decisions_sha256`——sha 须由 compare 面产出,机械后果 |
| 3 | compare 收参收敛为 `&FrameCutDeviceCtx`(设计签名罗列 spv/tables) | 等价重排,表/SPV 会话级单一事实源 |
| 4 | `frame_cut_device_ctx` 加 `verbose` 参 | 双跑第二遍 session 静默(既有「会话就绪」行 collect 门控同律);red-arm 篡改登记行不受门控(必打) |
| 5 | 行数账超设计量级(+613 vs ~300±80) | §2.5 分解;零功能越界,登记停手 |
| 6 | NoContraction 注入器第三副本 | DESIGN §5-3 预登记(字面同式副本;单源折叠归治理窗);selftest ⑦ 结构锚(注入量 = FAdd/FSub/FMul 数、插入位 = 首 annotation 前、原指令序逐字保持) |
| 7 | ~~`frame_cut_device_tables` 域检(有限 parent_error ≥1e9 拒)~~ **该域检为 C1 首红根因,已整体撤除**(§6 裁决登记):在树根簇合法编码 = 有限 f32::MAX,被误判资产异常 | 修正后上传律 = harness 字面(有限原样透传/非有限→2e9),无域检拒项;见 §6 |

## 4. GPU 验收命令清单(C1-C5;主 agent 锁内消费)

先决(本段已产,可跳;重跑命令形):

```powershell
$env:CARGO_TARGET_DIR='H:\rurix\target-night'
cargo build -p rurix-render --features vendor-upscale --bin g31_frame_cut_probe --release   # 已在树,rc=0
cargo build -p rurixc --features vulkan-backend --bin rurixc --release                      # 已在树
$KDIR='H:\rurix\.tmp\g39_gates\t5_devicecut'; mkdir $KDIR -Force | Out-Null
& "H:\rurix\target-night\release\rurixc.exe" src/rurix-asset/kernels/g31_cluster_cull.rx --target vulkan -o $KDIR\g31_cluster_cull.spv   # 已产:rurixc 内嵌 spirv-val accepted
spirv-val $KDIR\g31_cluster_cull.spv                                                        # 已核 rc=0
$FCP='H:\rurix\target-night\release\g31_frame_cut_probe.exe'
$KSPV="$KDIR\g31_cluster_cull.spv"
$EV='H:\rurix\artifacts\day_0831_g39\t5_devicecut\ev'; mkdir $EV -Force | Out-Null
```

基参照(gpu_batch1 seg_t3 口径):`--cluster-pack .tmp/g36_gates/wave1_geo_composition/bistro.rxcp --error-px 2.0 --frames 16 --step-m 0.15 --res 96x54`。

| # | 命令 | 期望判据 |
|---|---|---|
| C0 | `& $FCP --selftest`(锁外) | **已过 rc=0**(§5);stderr 含「device 对拍臂 host 面〔表+sentinel 映射+params 三关中和+期望码+NoContraction 注入器+red-arm 裁决〕」+ `PASS selftest` |
| C1 | `& $FCP --cluster-pack .tmp/g36_gates/wave1_geo_composition/bistro.rxcp --error-px 2.0 --frames 16 --step-m 0.15 --res 96x54 --cut-source device --cull-spv $KSPV --evidence $EV\t5_dev.json` | rc=0 PASS = 判定码逐项全等 ×16 帧 ×双跑(内建 fail-closed,红则进程死)+ decisions∈{2,4} + 既有五判据;stderr 含 `cut_source=device` 与「device cut 对拍臂就绪 n=123169 表字节=8868168」;evidence 逐帧 `device_cut_probe_ms`/`device_cut_decisions_sha256` 非 null 且 16 帧 sha 双跑传递性在案(单文件即证——两 session 均全等于同一 host 金标准) |
| C2 | `& $FCP …同基参… --evidence $EV\t5_host.json`(缺省 host 臂)+ `python -c "import json;a=json.load(open(r'$EV\t5_dev.json'));b=json.load(open(r'$EV\t5_host.json'));da=[f['digest'] for f in a['frames_data']];db=[f['digest'] for f in b['frames_data']];assert da==db,'digest 漂移';print('C2 OK digest 16 帧逐字节等')"` | **t5_dev == t5_host digest 序列逐帧逐字节**(P1 施加链 0 字节改动的结构性必然,机核之);另对照 `artifacts/day_0830_g38/t3_framecut/ev/t3_incr.json` digest 序列 = 跨窗参考锚(同机同驱动预期同;若异先查驱动/build 面,不误红本臂) |
| C3 | C1 同参 + `--cut-red-arm tamper`(evidence 可 `$EV\t5_red.json`,预期不落盘——进程先死) | **rc≠0** 且 stderr 含 `red-arm 模式甲 受害全局簇`(或乙)+ `判定码 mismatch 全局簇 …: device=… host=…`(对拍面真实消费的构造性证明;bistro 帧 0 cut 非空且中层簇 parent_error 有限 ⇒ 模式甲预期命中) |
| C4 | C1 同参 + `--min-level 1` → `--evidence $EV\t5_dev_ml1.json` | rc=0 PASS(提升前对拍口径 × ml1 组合;digest 自洽双跑,**不与 ml0 比**——T3 B5 口径同律) |
| C5 | `& $FCP --cluster-pack .tmp/g36_gates/wave1_geo_composition/bistro.rxcp --error-px 2.0 --frames 16 --step-m 0.15 --res 96x54 --evidence $EV\t5_default.json`(s09 原参,零新旗标)+ C2 同式 python 比对 t5_default vs t5_host | 缺省面 0-byte 回归:digest 序列 == t5_host.json(旗标不传 = 字面同路径;stderr `cut_source=host`) |

预期量级(DESIGN §2.8/§4):C1/C2 各 ~4min(16f×双跑×~9MB 表重传/帧);`device_cut_probe_ms` 为 P2 生产 dispatch 上界参考,单列不判读。**若 C1 红(mismatch)**:归因素材已在 stderr(簇号/两侧码/error/parent_error*/两球),按 DESIGN §3.2 预案 P2 判 NO-GO、P1 evidence 以风险量化形态收账。

## 5. 本段验证结果

| 项 | 结果 |
|---|---|
| `cargo check --release -p rurix-render --features vendor-upscale --bin g31_frame_cut_probe` | **rc=0,0 error**;仅 rurix-rt 既有 17 warning(非本段文件) |
| rurixc 现编 SPV | `.tmp/g39_gates/t5_devicecut/g31_cluster_cull.spv` 产出,rurixc 内嵌 spirv-val accepted + 独立 `spirv-val` rc=0 |
| C0 selftest(零 GPU) | **PASS rc=0**,⑦ 段全锚过(stderr 同时出 red-arm 甲/乙裁决登记两行 = 裁决函数在测) |
| 旗标 fail-closed 三路径 | `device` 缺 `--cull-spv` / `--cut-source bogus` / `--cut-red-arm` 无 device:均 rc=1 正确报文 |
| 窗口 bin 交叠面(`--bin g31_window_present` check) | 首查红——错误在 T1 独占域(`g31_window_present.rs:12185` format 串 53 占位 vs 51 实参,纹理 evidence 组装行 T1 在途编辑;与 frame_cut 臂共享面无涉,本臂错误会以 `g31_frame_cut_arm.rs` 路径报出)。按台账纪律未代修,等 ~100s 重试 **rc=0 绿**(T1 已自修);随后 probe bin 复确认 rc=0——**收尾双绿在手** |

---

## 6. C1 首红裁决登记(判读器口径修正,G38 B6/B7/transparency 同律)

### 6.1 定性:对拍臂域检口径错,产物零缺陷

C1 真跑首红(`b5_log.jsonl` C1_device stderr 尾):

```
FAIL device cut 表: 块 0 簇 604 有限 parent_error 340282350000000000000000000000000000000 撞 sentinel 域(≥1e9;资产异常,fail-closed)
```

**在树事实链(据实复核)**:
- 构建侧:`dag.rs` L1665-1666 `let parent_error = if li == top { f32::MAX } else { group_of[li][ci].1 }` + 单测断言 L1779-1781「根 parent_error 须为 MAX 哨兵」——**根簇(无父)合法编码 = 有限 `f32::MAX`(3.4028235e38)**,`canonical_bytes` 逐位序列化进 RXCP。C1 报文值即此(块 0 簇 604 = 该块顶层簇)。
- host 消费侧:`projected_error_px`(cull.rs L134-146)`error ≤ 0 → 0;is_infinite → +∞;dist > error 则 e·pf/dist 否则 +∞`。对 f32::MAX:**非**无穷分支,走 `d_surface > e` 判——场景距离 ≪ 3.4e38 恒假 ⇒ **+∞**。
- kernel 消费侧:`parent_e >= 1e9` 分支 ⇒ parent_px = 1e9(f32::MAX ≥ 1e9)。
- **harness 先例即透传**:`g31_cluster_cull_device.rs` L204-209 `if is_finite { 原值 } else { 2.0e9 }`——dag 产物根 = f32::MAX(有限)**一直被原样上传**,判据① v1.1.5 全绿即锚在此路径上。段 2 的「加强域检(有限 ≥1e9 拒)」与已证先例矛盾,DESIGN E3-1 该行设计错,REPORT 原偏离 #7 未察觉在树合法形态。

### 6.2 修正字面(前后对照;`frame_cut_device_tables`)

```rust
// 前(C1 红):
let parent_e = if r.parent_error.is_finite() {
    if r.parent_error >= 1.0e9 { fail("…撞 sentinel 域(≥1e9;资产异常,fail-closed)"); }
    r.parent_error
} else { 2.0e9 };
// 后(harness 字面律):
let parent_e = if r.parent_error.is_finite() {
    r.parent_error          // 含根 f32::MAX 原样透传
} else { 2.0e9 };           // +∞/NaN → sentinel(NaN 必须映射,见 6.3)
```

### 6.3 等价论证

- **有限 e ≥ 1e9(根 f32::MAX)透传等价**:parent_px 的唯一消费 = 谓词 `parent_px ≥ thr`。kernel 走 `e ≥ 1e9` 分支 ⇒ parent_px = 1e9;host 走 `d_surface > e` 恒假分支(d_surface = 相机到判定球面距离,场景量级 ≪ 1e9)⇒ parent_px = +∞——**两侧对谓词同向饱和恒真,判定码逐位不变**;与非有限→2e9 映射(kernel 1e9 / host is_infinite +∞)同一饱和形态,故两路编码同判(selftest ⑦ 新锚机核:+∞ 根与 f32::MAX 根期望码逐项等)。残余角例(d_surface > e ≥ 1e9,须相机距球面 >1e9 m)由逐项全等门 fail-closed 兜底,不静默。
- **NaN 不可透传(保留非有限映射的理由)**:kernel `parent_e > 0` 对 NaN 假 ⇒ 0px;host `dist > error` 对 NaN 假 ⇒ +∞——直传会分叉,映射 2e9 后双侧饱和同判。
- **域检收窄至零**:负有限 → kernel `> 0` 假 0px / host `≤ 0` 分支 0px,双侧等价(叶 error≤0 对偶);无剩余「真域异常」类——判读最终仲裁 = 逐项全等门本体。

### 6.4 同步复核(修正要求 #2)

- `frame_cut_device_expected`:仅消费提升前布尔集,无 parent_error 依赖——**0 改动**。
- red-arm 受害裁决:条件字面不变,语义在新域下仍正确——模式甲 `pe ∈ (0,1e9)` = 球参与域(f32::MAX 根落 ≥1e9 饱和域,谓词恒真不可翻,甲正确跳过);模式乙条件 `pe > 0` 含饱和域(免篡即已 ≥thr),`pe < 1e9` 门控使饱和域 parent 不被篡。doc 注释已按「饱和域(含有限 f32::MAX 透传与非有限 2e9)」对齐。selftest ⑦ 新锚:f32::MAX 根块上 red-arm 裁决与 +∞ 根块逐项同(甲空→乙簇 0,同一篡改)。

### 6.5 修正后验证

| 项 | 结果 |
|---|---|
| `cargo build --release -p rurix-render --features vendor-upscale --bin g31_frame_cut_probe` | rc=0 |
| C0 selftest | **PASS rc=0**,⑦ 段含新锚(f32::MAX 根透传 + 期望码同判 + red-arm 裁决同判;stderr 出现两次模式乙登记 = 两种根编码同一裁决在测) |
| 行数账增量 | arm +561→+594(映射简化 + doc 对齐 + selftest 新锚);probe 0 改动 |

C1-C5 命令清单(§4)不变,可直接重跑 B5 全批。

*(交付:代码 + SPV 就位,未 commit;本文件 = 施工登记。)*
