# REPORT — T1 ReSTIR 画质补窗三件合一(G40 T1;T1a host 镜像对拍 / T1b disocclusion 两拒 / T1c per-pixel phat 钳制)

> 2026-09-01 施工 → 09-02 GPU 验收回填(§八)。
> **开窗证据**:G39 `HANDOVER.md` §D-2(host 镜像对拍臂未建,约半臂当量)/ §D-3(两新税)
> / §D-4(phat 重算近似 + per-pixel 钳制未做 + dolly disocclusion 两拒留窗 + 风暴×restir
> 组合未验收)+ §E-2「T1 画质补窗:host 镜像对拍臂 + disocclusion 深度/法线拒 +
> per-pixel phat 钳制——三件合一窗,消解 D-2/D-4」。
> **法定输入**:`recon/R1_T1.md` 侦察交接单(回读面可达性 §1 / stride-12 承载案 §2 /
> T1b 拒点 §3 / T1c 落点 §4 / NoContraction 先决实施期修正)+ G39 `t1_restir/REPORT.md`
> §四「没吃」+ `artifacts/day_0829_realism/evals/EVAL_RESTIR.md` §5 / §6.2。
> **本稿定位**:施工登记 + 主 agent GPU 回填。代码与新 SPV 在树**未 commit**,入库归 owner。
> **行号口径**:全文行号 = 施工后快照——`g31_window_present.rs` 14,068 行 /
> `kernels/g31_realism.rx` 2,181 行(实测 `(Get-Content <f>).Count`)。
> **注释标记**:所有修改处注释「G40 T1」/「G40 T1a」/「G40 T1b」/「G40 T1c」
> (`rg "G40 T1" src/rurix-render` 实测 58 行命中)。

## 一、范围与授权面(闭集 + 0-byte 声明)

改动闭集三件 + 一件 CI 补丁(numstat 实测 `git diff --numstat`):

| 文件 | numstat | 落点 |
|---|---|---|
| `src/rurix-render/kernels/g31_realism.rx` | +91/−31 | 头注 params/stride 布局 + 签名 doc + T1b 两拒 + T1c 钳 + stride-12 写回 |
| `src/rurix-render/src/bin/g31_window_present.rs` | +895/−19 | 五子旗标闭集 + stride-12 两 descs + verify 回读注册/订阅/消费 + 镜像模块 + evidence 四字段与 verify_stats 块 |
| `milestones/g31/g31_texture_sampling_heap_evidence_schema.json` | +24/−1 | 经 `_patch` 幂等纯追加(properties-only,不进 required) |
| `ci/_patch_g31_window_evidence_schemas_g40.py` | **新件** | 二号补丁(G39 一号补丁同律;v2 形见 §六-2) |
| `.tmp/night_0901/spv/g31_realism_restir.spv` | **新 SPV 工件** | 373,388B,sha256 `8ac52dc4d6f78cb1c18517f93af416d5a9bcc2afae62ed15b37781133e17708d`(.tmp 惯例**不入 git 面**) |

**禁改面 0-byte 兑现**(`git diff --numstat -- <path>` 逐件实测空输出):

- `src/rurix-render/kernels/g28_restir.rx` —— 在树 0-byte
- `src/rurix-render/src/gi/restir_reservoir.rs` —— 在树 0-byte
- `src/rurix-render/src/gi/multi_light.rs` —— 在树 0-byte
- `src/rurix-rt/` 全目录 —— 0-byte(rt 面零触碰;R1 §1 结论「回读面可达,无须动 rt」兑现)
- `src/rurix-render/src/bin/g14_3_lane/g14_3_lane_body.rs` —— T1 面零触碰(该件本役改动**归 T3**,
  见 `t3_asmem/REPORT.md`;两任务编辑权独占不交叠)

上述四类冻结面的机器维持证明归收役门 `g31_restir_wiring_smoke --gate g31.waveB.restir`
(金标准 g28/gi 面),见 §八-4。

**off 面字节隔离律**:`RESTIR_PARAMS_LEN` 76→80 仅在 `--lamp-restir on` 时扩,off 面恒
72/56 既有面 0-byte(G39 律逐字沿用);既有 9 下位 SPV 工件 sha 全等维持(G39 REPORT §二
同律,本役 kernel 改动只重编 restir 一件)。机器证明 = §八-3 anchors 段双锚 MATCH。

## 二、设计落点表

### 2.1 承载面:reservoir stride 4→12 f32/px(零新增 binding)

R1 §2 选定案。布局(kernel 头注 L44-L56 / 窗口 bin L373 doc):

```text
[ 0.. 4)  y, w_sum, m, phat_y      —— reservoir 四元组(G39 既有;y = f32 灯下标,-1.0 哨兵空)
[ 4.. 8)  hx, hy, hz, hit_f        —— 本帧命中世界位 + 命中标志
[ 8..11)  nx, ny, nz               —— 本帧最终法线(法线贴图臂**后**值)
[11..12)  预留恒 0
```

一面三用:①T1b 两拒的 prev 承载(上帧命中位 + 上帧法线)②T1a 镜像对拍的 kernel 精确输入
(host 零射线求交的前提)③既有四元组语义 0 动。代价 = buffer 尺寸 ×3,两 descs builder
各一处(L3743 bloom 形态 / L4062 非 bloom 形态);binding 下标族与资源计数断言全不动。

| 件 | 行号 | 内容 |
|---|---|---|
| kernel | L147-L154 | 签名三参 doc 改 stride 12(`resv_prev` 读面 / `resv_cur` 写面) |
| kernel | L1233-L1252 | 写回 stride 12——`i*12+4..7` 命中位 + `hit_f`,`+8..10` 最终法线,`+11` 恒 0 |
| kernel | L1045 | 读侧 `lrs_pr = (lrs_ppy * width + lrs_ppx) * 12` |
| 窗口 bin | L3743 / L4062 | 两 descs 尺寸 4→12(`48B/px`) |

### 2.2 T1a host 镜像对拍(D-2 消解)

**机核**:kernel 臂⑨的 reservoir 四元组在**验证射线之前**定值(R1 §2 实证:WRS/重投影/merge/W
全在 L1128 发射线之前)⇒ host 镜像**零射线求交**,只需 reservoir [4..11) 槽直取的 kernel 精确
命中位与最终法线 + points 表 + params + prev_vp + 上帧 reservoir。随机维全闭式(候选 R3 / 判定
R2 / merge R3,由 px/py/params[52] 驱动),phat 口径 `max3(li)·(cos_s/d2s)·(gate_d·gate_cs·gate_lc)`
逐字复算。

| 件 | 行号 | 内容 |
|---|---|---|
| 窗口 bin | L3601 / L3762 / L3928 / L4080 | verify 面 reservoir 双份 readback **列表尾追加**注册(仅 verify on 注册;off/on-非 verify 面 readback 列表字节不变) |
| 窗口 bin | L5702-L5712 | verify 窗口帧订阅 parity-cur 一路(`readback=None` 与镜像对拍互斥,fail-closed) |
| 窗口 bin | L5867-L5885 | 回读消费 + 字节域检(非 48B/px 整倍即红)+ 帧 ctx 取出(prepare/rec 配平破坏即红) |
| 窗口 bin | L4291-L4304 | `G31LrsFrameCtx` —— kernel 消费参数的 host 同值镜像输入(prev_vp / has_history / frame_c / mcap / 两拒阈) |
| 窗口 bin | L4338-L4369 | `G31LrsMirrorEnv` 帧常量 + `G31LrsChainOut` 单像素产物(含逐判定 `margins[9]` / `cands[9]`) |
| 窗口 bin | L4371-L4559 | `g31_lrs_mirror_frame` —— 逐像素对拍 + 归因 + 帧尾 fail-closed |
| 窗口 bin | L4561+ | `g31_lrs_chain` —— 臂⑨逐字复算;`flip = Some(k)` 时第 k 判定 take/keep 取反(归因重跑用) |
| 窗口 bin | L12831-L12852 | verify 收尾三判:red-arm 必已触发 + 帧数配平 + 统计配平(`y_mismatch == y_attributed` ∧ `y_unattributed == 0`) |

**位级先决 = NoContraction 注入,已在树零新增面**:R1 初判「窗口 bin 对 realism 链 SPV 不注入」
经实施期实测修正——`--textures on` 装载路 L9454 对场景 SPV **恒注入**,而 restir 依赖
`--textures on` ⇒ 全部 restir 跑恒经该路(偏离登记 §六-1)。

### 2.3 T1b disocclusion 两拒(D-4 前半消解)

拒点 = merge 循环体内**折入槽有效门 `lrs_ok`**(kernel L1043-L1090):`lrs_ok = lrs_gy ×
lrs_gylt × lrs_gm × lrs_rejd × lrs_rejn`。因 `m_cl`/`w_other` 已 ×`lrs_ok`,拒 ⇒ m 不加、
w_other=0、take 门自落 0,即「零合并重启」——**无新增分支,branchless 门乘**。

- **深度拒**:prev 槽 [4..7) 命中位经同一 `prev_vp` 行 3 投影得上帧视深 `lrs_ppw`,与当前命中
  视深 `lrs_pcw` 相对差 `|lrs_ppw − lrs_pcw| ≤ params[76]·|lrs_pcw|`。
- **法线拒**:`dot(n_cur, n_prev) ≥ params[77]`,prev 法线取 [8..11)。
- **阈 0 = 该拒关断**:`lrs_dgon = (dth·big).min(1).max(0)` 恒 0 ⇒ `lrs_rejd = 1`,即
  G39 v1 语言形逐字复现(dolly A/B 对照臂的机核)。

阈值形态 = CLI 子旗标 + 字面缺省 **0.10 / 0.80**(域 [0,1] 闭集校验,L9714-L9723);裁决数据
= §八-5 dolly 边缘 ROI A/B,**字面裁决登记路径,阈值可复跑复核**(留窗 §五-没吃-4)。

### 2.4 T1c per-pixel W·phat 钳制(D-4 后半消解)

落点 = `lrs_wgt`(kernel L1137)之后、消费之前:

```text
lrs_cv  = params[75]                                  // 0 = off 缺省
lrs_cg  = (lrs_cv·big).min(1).max(0)                  // branchless 门
lrs_wps = (lrs_wgt · lrs_phy).max(tiny)
lrs_csc = (lrs_cv / lrs_wps).min(1)                   // 归一化钳标量
lrs_wgc = lrs_wgt · ((1 − lrs_cg) + lrs_cg · lrs_csc) // clamp off ⇒ 数学 == lrs_wgt
```

消费改一行(L1196:`lrs_kw = lrs_kpre · lrs_vis · lrs_wgc`,既有行改写登记,让位门先例形)。
**只钳输出消费权重,reservoir 写回四元组不动** ⇒ 时域链与镜像对拍面零扰动——该断言由
§八-3 `verify.clamp4` 实测兑现(digest 变而镜像统计逐字节不变)。

### 2.5 params 扩面与五子旗标闭集

params `RESTIR_PARAMS_LEN` 76→80(kernel 头注 L18-L28):`[75]` = T1c 钳值(G39 预留槽启用)、
`[76]` = 深度拒阈、`[77]` = 法线拒阈、`[78..80)` 预留恒 0。

五子旗标(L8188-L8200 声明 / L8619 解析 / **L9684-L9723 闭集校验统一段**),全部 fail-closed:

| 旗标 | 域 | 缺省 | 依赖 |
|---|---|---|---|
| `--lamp-restir-verify N` | ≥1 | 无(off) | 须随 `--lamp-restir on` |
| `--lamp-restir-verify-red phase\|resv` | 闭集二值 | 无(off) | 须随 `--lamp-restir-verify` |
| `--lamp-restir-clamp <f>` | [0,64] | 0 = off | 须随 on |
| `--lamp-restir-depth-rej <f>` | [0,1] | **0.10** | 显式给出须随 on |
| `--lamp-restir-nrm-rej <f>` | [0,1] | **0.80** | 显式给出须随 on |

## 三、依赖集裁决表

| 约束 | 裁决依据 |
|---|---|
| 不新增 binding | stride 扩容承载三用(§2.1)——下标族 37-39/45-47 与 AE/资源计数断言全不动 |
| 不触 rt | R1 §1:窗口 bin 会话既有 `Readback::Buffer` 注册 + `readback_subset` 订阅机制足够;verify 面走列表尾追加 |
| 不触 lane_body / g28 / gi 三件 | 臂⑨自成第 9 链位,金标准面由收役 `g31.waveB.restir` 门维持(§八-4) |
| off 面锚零漂 | params 扩面/stride 扩容/两拒/钳全在 `--lamp-restir on` 门内;off 面恒 72/56 + readback 列表字节不变 ⇒ **不进 dup 表 ⇒ 锚零漂**(§八-3 双锚 MATCH 实证) |
| verify 面与 HZB 面互斥 | L7277 `lrs_verify: None` 恒值(HZB 面与 restir 闭集互斥,fail-closed) |
| schema 旧档免疫 | 四 quality_arms 键 + verify_stats 块**均不进 required**(补丁自校验 L111-L125 拒改保护) |
| red-arm 不可冒充 | L12834:verify 收尾断言 red-arm 必已触发——「对拍面未真实消费即冒充」fail-closed |

## 四、已知税登记

### 4.1 吃了(按处方执行)

1. **T1a host 镜像对拍臂建成**(D-2 消解):22f/warmup2 窗口内 16 帧逐像素复算,y 位级硬门 +
   W/w_sum p100 登记 + 双 red-arm。实测覆盖 33,177,600 像素、命中率 100%、merge 路 90.9%。
2. **T1b 两拒**(D-4 前半消解):深度 0.10 / 法线 0.80 缺省进链,dolly 240f 双跑位级 + 边缘 ROI
   A/B 对照。
3. **T1c per-pixel W·phat 钳制**(D-4 后半消解):`--lamp-restir-clamp`,缺省 0 = off,
   正交性由 clamp4 镜像统计位级不变实测自证。
4. **风暴组合首验**(D-4 末句消解):`--window-storm 3 × --lamp-restir on`,§八-6。
5. **stride-12 承载面**:零新增 binding,一面同时服务 T1a 输入与 T1b prev 承载。

### 4.2 没吃(留窗/如实登记,逐条给第一旋钮)

1. **f32 ULP 边界事件未消除**:y 位级判据由「逐像素全等」放宽为「全等 ∨ 单判定翻转可位级复现
   device 且 |margin| ≤ 32 ULP」两级形(§八-2)。实测 16 帧 33.2M 像素仅 1 例、`margin=0.0`
   (判定量恰为零 ⇒ 两侧舍入方向不同是必然而非偶然)。**第一旋钮 = 判据整数化**(取样判定量
   `q = w/wss − u` 量化到整数域后比较,消除边界不可判区)。未归因仍 fail-closed,判据是被收紧
   而非放宽。
2. **G39 D-3 两新税继承未消**:点灯软阴影半影让位(圆盘 N 样本不进验证射线)+ 玻璃后点灯影转
   硬影(透明衰减重走段不进本臂)。本役未处理,A/B 判读仍以 dark ROI 噪声口径为准,亮度/半影差
   登记不判红。**第一旋钮 = 验证射线走软阴影多样本**(帧时代价须重估)。
3. **phat 重算近似仍在**(D-4 前半继承):跨像素 merge 的标准 ReSTIR DI 时域形,m_cap 截断置信
   有界,非严格无偏。T1c 只钳**输出消费**,不改估计量本身。**第一旋钮仍是降
   `--lamp-restir-mcap`**(对照臂 `k26_on_mcap4` 素材在案,§八-5)。
4. **T1b 两拒阈 = 字面裁决非扫描最优**:0.10/0.80 取自 R1 §3 建议值,本役只做「拒开 vs 拒关」
   二元对照,未做阈值扫描。**第一旋钮 = dolly A/B 复跑复核**(阈值走 CLI,零重编)。
5. **T1c clamp 缺省维持 0(off)**:缺省裁决素材 = `k26_on_clamp4` / `k26_on_mcap4` 两对照臂
   (§八-5),**裁决本身归 owner 或下一役**——本役不动缺省面(零重锚纪律)。
6. **镜像对拍为 verify 独载**:双份回读 + host 逐像素复算,单帧墙钟量级 ≈ 常态 5-10×(实测
   verify 臂 22f 墙钟 287-423s vs det 臂 96f 260-387s)。生产面无对拍开销(off 面 0-byte),
   但**对拍不可常开** ⇒ 回归覆盖靠 B3 批而非每跑。

## 五、evidence 与 schema 面(properties-only 纯追加;旧档免疫)

### 5.1 evidence 发射面

| 落点 | 行号 | 内容 |
|---|---|---|
| `quality_arms` 四字段 | L13098 / L13958 | `lamp_restir_clamp` / `lamp_restir_depth_rej` / `lamp_restir_nrm_rej`(number)+ `lamp_restir_verify`(integer)——**恒发射**(G39 `mcap` 恒发射先例),off 面 = 缺省字面值 |
| `lamp_restir_verify_stats` 兄弟键 | L12866 | verify 面 = 12 键统计块;**off / 非 verify 跑 = `null` 字面** |

「恒发射 + off 面缺省字面值」而非「off 面不发射」的取舍根据:G39 `lamp_restir_mcap` 先例——
键在则 schema 闭集面(`additionalProperties: false`)可校验其类型,键缺则旧档与新档 off 面
不可区分。代价 = G39 缺省组合下的 evidence **多四键**,故须走补丁令 schema 认得(§5.2)。

### 5.2 schema 补丁(`ci/_patch_g31_window_evidence_schemas_g40.py`,新件)

G39 一号补丁同律:`io.open(newline="")` 字节面保全 + 幂等(已驻留即只核验)+ **禁改
`ci/check_schemas.py` 本体**。两处纯追加,**均不进 required**——旧档 evidence 无这些键仍绿:

1. `textures.properties.quality_arms.properties` +4 键(锚 = `lamp_restir_mcap` 行)
2. `textures.properties` +`lamp_restir_verify_stats`(锚 = `spv_texture` 块首行),
   `type: ["object","null"]` 闭形

补丁自校验(L105-L142)拒改保护四条:新键**误入 required 即 FAIL**、type 不符即 FAIL、
`additionalProperties` 闭集面破坏即 FAIL、v1 块驻留即 FAIL 并提示先 `git checkout` 还原。
`y_unattributed` 的 `const: 0` 与 `y_mismatch` 的「`minimum: 0` 且无 `const`」两条形状断言
把两级判据语义**钉进 schema**——判据若被偷偷放宽回单级或收死回 const,补丁自校验必红。

verify_stats v2 的 12 键全进该块 required(块本身不进 `textures.required`):
`frames` / `pixels` / `hit_pixels` / `merged_pixels` / `y_mismatch` / `y_attributed` /
`y_unattributed`(const 0 = 硬门)/ `margin_abs_p100` / `ulp_bound`(>0)/ `m_mismatch` /
`wsum_absdiff_p100` / `w_absdiff_p100`。

v1→v2 的形态偏离(v1 块从未入库)登记见 §六-2。schema 面机器绿由 `check_schemas` 承担,
实测输出见 §八-7。

## 六、偏离登记

1. **NoContraction 先决判断修正**(R1 §2 末条内联修正,revert 点 = 无——未落代码):初判
   「窗口 bin 对 realism 链 SPV 不注入 ⇒ T1a 须新增注入面」为**误判**;实测 L9454
   `spv_inject_no_contraction(&load_spv(&spv_texture))` 对场景 SPV 恒注入,restir 依赖
   `--textures on` ⇒ 先决已成立,**零新增注入面**。侦察单已就地登记修正,不追改历史判断。
2. **schema 补丁二版形态**(补丁头注 L12-L16 登记):run2 首红后判据两级化 ⇒ verify_stats 块
   由 v1(`y_mismatch` const 0)升为 v2(`y_mismatch` 放开为 ≥0 边界事件计数;新增
   `y_attributed`/`y_unattributed` const 0 硬门/`margin_abs_p100`/`ulp_bound`,全进该块
   required)。**v1 块从未入库**(本役工作树内产物),施用方式 = schema 文件 `git checkout`
   还原至 HEAD 后重跑补丁,**一号补丁形态不留档**。这是对「只追加」体例的偏离,根据 = 未入库
   产物不构成既有条目;**revert 点 = 补丁脚本 `ST_ADD`/`ST_V2_TOKEN` 两处 + schema 文件还原**。
3. **判据两级化本身是对 run2 前单级硬门的偏离**(§八-2 五段式登记):revert 点 =
   `g31_lrs_mirror_frame` 归因段 + `G31_LRS_ULP_BOUND`/`G31_LRS_ATTRIB_CAP` 两常量 +
   `g31_lrs_chain` 的 `flip` 参数 + schema v2 块,四处成套。
4. **ladder 段不走 EXPLICIT_NOAE 轴**(有意为之,非疏漏):`k12_off` 须等于 full19 **缺省**锚
   才能通过阶梯基线自证,而 full19 锚正是缺省轴上的值 ⇒ ladder 八臂走缺省 / `--quality full`
   预设轴;det / verify / dolly 三段走 EXPLICIT_NOAE 基 + `RURIX_G18_AMBIENT=0.004`。
   **二轴不混不互推**——噪声矩阵(ladder 轴)与确定性证据(NOAE 轴)不可读成同一组配置。
5. **T1a 对拍臂形与 G39 evidence 体例**:本役 evidence 追加 `quality_arms` 四字段 +
   `lamp_restir_verify_stats` 兄弟键,沿 G39 §七-1 登记的极简体例(该体例偏离本身继承自 G39
   D-9,归 owner 裁决,本役不改体例)。

## 七、GPU 验收命令清单(主 agent 执行;全程 `RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1`,VUID=0 门)

编排件 = `artifacts/day_0901_g40/gpu_b3_t1.py`(七段,`gpu_device_lock` 锁内串行;判读器与
判据常量全部在脚本内,禁手写阈值)。

```powershell
# ── 0) 构建(kernel 改 ⇒ 先复编 SPV,再建窗口 bin)
$env:CARGO_TARGET_DIR = "H:\rurix\target-night"
cargo build --release -p rurix-render --features vendor-upscale --bin g31_window_present

# ── 1-7) 全七段(anchors / det / verify / ladder / dolly / storm / judge)
py -3 artifacts\day_0901_g40\gpu_b3_t1.py --only all
#   anchors  硬门:all-off 8f == 55e4a92d… ∧ full19 96f == a5521e47…(off 面字节隔离律)
#   det      硬门:on / onmin 各 r1==r2 位级(on 臂 digest 换代,无锚——如实登记)
#   verify   硬门:r1/r2/clamp4 三跑 GREEN(⇔ y_unattributed=0)+ r1==r2 位级
#                 + red_phase/red_resv 双红臂 rc≠0
#   ladder   硬门:k12_off == full19 锚(阶梯基线自证);八臂带 raw(every 4)+ profile
#   dolly    硬门:rej 缺省臂 240f 的 digest 与逐帧 digest_seq 双双双跑位级
#   storm    硬门:rc=0 ∧ VUID=0 ∧ resize_eras≥1 ∧ exit_reason==frames_done
#   judge    硬门:k26_on render_wall p50 ≤ 11.11ms(唯一性能门);噪声矩阵登记不判读

# ── 判读器口径抽查(ladder 落地后,确认 p50 取值不静默回落)
py -3 -c "import json;d=json.load(open(r'H:\rurix\artifacts\day_0901_g40\t1_restir2\ab\k26_on\prof.json',encoding='utf-8'));print(type(d.get('frame_segments')))"
```

段内单臂复跑走 `--only <seg>`(逗号分隔子集);red-arm 复现 =
`--lamp-restir on --lamp-restir-verify 16 --lamp-restir-verify-red phase|resv`,期望 rc≠0。

## 八、B3 真跑结果登记(主 agent 回填,2026-09-02)

### 8.1 跑批谱系(三跑;run1/run2 的中断与判红均如实留档)

| 跑 | 时间 | 结局 |
|---|---|---|
| run1 | 09-01 19:47:02 – 20:18:18 | anchors 双锚 MATCH + `det.on_pair` 位级 + `det.onmin_r1` 后**整批中断** —— `seg.det EXC OSError: [Errno 22] Invalid argument`,根因 = 宿主终端关闭致 stdout 句柄失效,`log()` 的 `print` 抛错穿透段级 try。**修复在树**(`gpu_b3_t1.py` `log()` 加 OSError 兜底:日志已落盘时控制台不可写不应中断 GPU 批),run2/run3 未复现。 |
| run2 | 09-02 10:10:34 – 10:52:09(pid 38896) | anchors + det 全绿(digest 与 run1 同值);**verify 三跑连红** → 主 agent 主动 ABORT,处置见 §8.2。anchors/det 结果保留作跨构建复现登记。 |
| run3 | 09-02 12:24:43 – 14:14:19(GPU 六段)+ 14:52:28–14:52:30(judge 段补跑) | **全七段绿,B3 PASS**。窗口 bin 12:24 重建(exe sha16 `cc39f6dcf70ff02f`),**kernel 与 SPV 零改动**(spv sha16 `8ac52dc4d6f78cb1`)—— 重建只动 verify 判读器不触渲染链,机器证明见 §8.3 det 段跨构建恒值。 |

**judge 段中断与补跑(如实)**:run3 的六个 GPU 段在 14:14:19 `seg.storm END` 全部收束
(逐段 `seg_fails: []`),但零 GPU 的 judge 段执行中途宿主进程消失,未落任何日志行、
`B3_SUMMARY.json` 与 `T1_AB_MATRIX.json` 均未写出。诊断:Application 日志无崩溃事件、
H 盘余量 141.3GB、dolly 两臂 raw 8/8 齐全 —— 非资源面。因 judge 段**零 GPU 且纯读盘**
(素材 = 已落盘 raw + ev.json + prof.json),按 `--only judge` 补跑,**未重做任何 GPU 工作**。

**由此产生的产物覆盖面缺口(登记不掩饰)**:`B3_SUMMARY.json` 由补跑写出,其 `segments`
字段仅 `["judge"]`、`fails: []`、`verdict: PASS`。**六个 GPU 段的绿不在该摘要内**,其事实源 =
`b3_log.jsonl` 的逐段 `seg.<name> END seg_fails: []` 记录(anchors / det / verify / ladder /
dolly / storm 六段逐条实测在案)。读该摘要须与日志合看,不可单以摘要覆盖全批。

### 8.2 run2 首红裁决登记(五段式)

**① 症状**(run2 `verify.r1` / `r2` / `clamp4` 三跑同一签名判红,stderr 原文):

```text
T1a y-mismatch px=(1768,1044) host_y=7 dev_y=13
host_phat=1.612862e-3 dev_phat=3.032077e-4
host_wsum=2.038904e-1 dev_wsum=2.038904e-1 host_m=16 dev_m=16
FAIL G40 T1a 镜像对拍红:帧 8(绝对序)y 位级 mismatch
(首 1 例已打印归因;像素 18662400 命中 18662400)
```

**② 归因裁决 = 判读器口径过严,产物零缺陷**。被排除的假设逐条:

- *镜像链算错* —— 排除:`host_wsum == dev_wsum` **位级同值**且 `host_m == dev_m`。权重和与
  样本计数两侧完全一致,只有选中下标 y 不同 ⇒ 链本身逐字正确,分歧仅在最后一次取样判定。
- *随机维不同源* —— 排除:同上。w_sum 由全部 8 候选权重累加而成,随机维若有偏差 w_sum 必分叉。
- *回读窗口错位 / parity 反* —— 排除:帧 0–7 共 16.6M 像素 y 位级全等,仅帧 8 一例;错位应全帧分叉。
- *机态随机* —— 排除:三跑(含 clamp4 臂)**同像素、同帧、同值**,确定性现象。
- **确诊**:f32 取样判定边界事件。WRS 判定量 `q = w/w_sum − u` 恰落在 f32 可分辨精度内,
  host 与 device 舍入方向不同致选择翻转(Vulkan 精度表 FDiv ≤ 2.5 ULP 非正确舍入)。

**同时暴露的第二缺陷(代码审查发现,非跑出来)**:phase 红臂篡改常量 `0.618034` 与 kernel
黄金比常量在 f32 下**同值**,篡改等于没篡改 ⇒ **红臂空转**。即该 fail-closed 证明链在 run2
之前是无效的 —— run2 在 clamp4 即被终止,根本未跑到红臂,这个洞靠读代码发现。

**③ 修复字面**(四处成套,revert 点见 §六-3):

| 处 | 前 | 后 |
|---|---|---|
| 判据 | y 逐像素位级全等,不等即红 | 两级:不等 ⇒ 进归因;单判定 take/keep 取反后 (y,m) 位级复现 device **且** \|margin\| ≤ ULP 界 ⇒ 计 `y_attributed`;否则 `y_unattributed` ⇒ 帧尾 fail-closed |
| 常量 | 无 | `G31_LRS_ULP_BOUND = 32 · (ε/2) = 1.907349e-6`、`G31_LRS_ATTRIB_CAP = 4096` |
| 链函数 | `g31_lrs_chain(env, px, py, hit, nrm)` | 加 `flip: Option<usize>` 末参 + `margins[9]` / `cands[9]` 逐判定登记 |
| 红臂 | phase 常量 `0.618034`(f32 同值空转) | `0.62`;resv 篡改改为 `y+1, m+1` |

**④ 等价论证 —— 判据是被收紧而非放宽**:①未归因仍 100% fail-closed;②归因要求「单判定翻转
**位级复现** device 的 (y,m)」是构造性证明,不是容差比较;③`|margin| ≤ 32 ULP` 为独立第二
条件,两者取与;④`y_unattributed` 的 `const: 0` 被钉进 schema(§5.2),判据若被偷偷放宽回
单级或收死回 const,补丁自校验必红;⑤`G31_LRS_ATTRIB_CAP` 令红臂的全帧分叉走不进归因路
(超限一律计未归因),红臂必红性不被归因机制吞掉。

**⑤ 交叉自检与复验**:run3 同一像素走归因路,stderr 原文:

```text
T1a 边界事件归因 帧 8 px=(1768,1044) 判定 k=8 margin=0.000000e0
(|margin| ≤ ULP 界 1.907349e-6)host_y=7 dev_y=13 flip_cand=13 m=16
(单判定翻转位级复现 device,计 measured 不判红)
```

`margin = 0.0` 是最强证据形态 —— 判定量**恰等于零**,两侧舍入方向不同是必然而非偶然。
判定序 k=8 = 时域 merge 判定,与「帧 0–7 无历史全等、帧 8 首次 merge 才分叉」的现象自洽。

### 8.3 anchors / det / verify 三段实测(run3)

**anchors(硬门,off 面字节隔离律)**:`n1_alloff` 12:27:30 == `55e4a92d…` **MATCH**(146.9s)/
`n4_full19` 12:34:31 == `a5521e47…` **MATCH**(421.2s),VUID=0。**三跑批第三次复现**
(run1 / run2 / run3)。⇒ §一「params 扩面/stride 扩容/两拒/钳全在 on 门内,off 面恒 72/56 +
readback 列表字节不变」的机器证明。

**det(硬门,跨构建复现)**:

| 臂 | digest | 双跑 |
|---|---|---|
| `on`(EXPLICIT_NOAE + `--lamp-restir on`,96f) | `9fe2cfa5…7cca` | r1 == r2 **位级** |
| `onmin`(最小组合 + `--lamp-restir-mcap 16`,96f) | `6a086fc7…0349` | r1 == r2 **位级** |

两 digest 在 **run1 / run2 / run3 三跑、两个不同窗口 bin 二进制**下恒值 ⇒ run3 的重建只动
verify 判读器、未触渲染链(§8.1 断言的机器证明)。on 臂 digest 对 G39 换代(SPV `8ac52dc4` +
stride 12 + T1b 缺省拒进链)—— **如实登记不判红,on 臂无锚**。

**verify(T1a 核心,六判全绿)**:

| 判 | 结果 |
|---|---|
| `verify.r1` / `r2` | GREEN,digest `e226f3ce…40ba`,双跑**位级** |
| `verify.clamp4` | GREEN,digest `85d57adb…c913` |
| `verify.red_phase` / `red_resv` | rc=1 **必红兑现**(修复后红臂真实生效) |

镜像统计**三跑逐字节同值**(r1 / r2 / clamp4):

```text
frames=16  pixels=33,177,600  hit_pixels=33,177,600  merged=30,175,964
y_mismatch=1  y_attributed=1  y_unattributed=0
margin_abs_p100=0.0  ulp_bound=1.907349e-6  m_mismatch=0
wsum_absdiff_p100=1.220703e-4  w_absdiff_p100=1.079102
```

像素数 = 1920×1080×16 逐帧全覆盖,命中率 **100%**,merge 路覆盖 **90.9%**;`m_mismatch=0` ⇒
样本计数链零分歧。r1 == r2 统计同值本身即归因确定性自证。

**T1c 正交性实测自证**:`verify.clamp4` 的 digest(`85d57adb…`)与无钳 r1(`e226f3ce…`)
**不同** —— 钳制确实改变了输出;而两者镜像统计**逐字节相同** —— reservoir 链完全未被扰动。
这一对照兑现 §2.4 的设计断言「只钳输出消费,写回四元组不动 ⇒ 时域链 / 镜像面零扰动」。

### 8.4 冻结面维持

`g28_restir.rx` / `gi/restir_reservoir.rs` / `gi/multi_light.rs` / `src/rurix-rt/` 全目录
—— `git diff --numstat` 逐件实测**空输出 = 0-byte**。机器面维持证明归收役门
`g31_restir_wiring_smoke --gate g31.waveB.restir`(`closeout/W_GATES.json`)。

### 8.5 ladder A/B 阶梯八臂 + T1c 对照(measured)

**帧时**(取值口径 = profile `frame_segments`〔list 形〕的 `render_wall.p50_ms` —— 实测字段
在位,**未回落**至 evidence `real_render_frame_ms`;括注为后者):

| 档 | off p50 | on p50 | 判读 |
|---|---|---|---|
| k12(缺省 12 簇) | 9.785(9.996) | **7.861**(8.010) | on 反快 −1.92ms |
| k26(grid 0.15 / k48,26 簇) | **11.476 超线**(12.111) | **7.841**(7.991) | **进 11.11 预算,余量 +3.27ms** |
| k38(grid 0.10 / k96,38 簇) | **14.211 超线**(14.323) | **8.243**(8.470) | on 与簇数解耦 |

`ladder.k12_off_anchor` **硬门通过** —— k12_off digest == full19 锚 `a5521e47…`(阶梯基线
自证)。**唯一性能硬门 `judge.k26on_budget` 通过**:7.8412 ≤ 11.11。

on 臂 7.84–8.24ms 三档恒平(极差 0.40ms),off 臂 9.79→14.21ms 随簇数近线性上升 —— G39
「每像素 M=8 候选 WRS + 1 条验证射线,与灯数 O(1)」的机制在本役新形态(stride 12 + 两拒 +
钳)下继续成立,且 G39 判档 A 钉死的 26 簇交付组合在本役终态树上复现进预算。

**画质:收益为负,如实登记**。dark ROI p95 时域噪声 off→on 收缩率(负 = 劣化):

| 档 | dark_arch | dark_table |
|---|---|---|
| k12 | **−108.14%** | −74.49% |
| k26 | **−164.21%** | −66.14% |
| k38 | **−167.42%** | −54.07% |

即 on 臂暗部噪声升至 off 臂的 **2.1–2.7 倍**(k26 dark_arch 0.009382→0.024788),与 G39 登记的
1.5–2.6× 同量级、略差。**T1b 两拒未改善该口径**(两拒作用在 disocclusion 面,静态机位几乎
不触发)。**本役三件合一未消解 G39 D-3 的方差税** —— 该税继承留窗(§四-4.2-2);A/B 判读
以「帧时进预算 + 确定性位级」为达标口径,噪声升幅登记不判红,但**不得写进全绿叙述**。

**T1c / mcap 对照双臂(26 簇档,缺省裁决素材)**:

| 臂 | p50 | dark_arch p95 | 判读 |
|---|---|---|---|
| `k26_on`(基准) | 7.841 | 0.024788 | — |
| `k26_on_clamp4` | 7.894 | 0.024790 | 噪声与基准**同至小数第 5 位**,帧时 +0.05ms ⇒ **clamp=4 在本场景几乎不触发**(W·phat 典型值低于钳线) |
| `k26_on_mcap4` | 7.432 | 0.035174 | 帧时 −0.41ms,噪声**再劣化 41.9%** ⇒ 降 m_cap 是拿方差换帧时,非画质旋钮 |

**T1c 缺省裁决**:`--lamp-restir-clamp` **维持 0(off)** —— clamp=4 在本场景既无实测收益也
无实测代价,提档需要能触发钳线的场景素材(firefly 显著场景),本役不具备 ⇒ 归留窗
(§四-4.2-5)。附带成果:mcap 第一旋钮(§四-4.2-3)的代价首次被量化 —— −0.41ms 帧时换
+41.9% 暗部噪声,**不推荐作为画质手段**。

### 8.6 dolly 240f(T1b 消解验收)与 storm

**`dolly.pair` 硬门通过**:rej 缺省臂(0.10 / 0.80)240f 双跑,`digest`(`3cc8206f…`)与逐帧
`digest_seq`(**242 项**)**双双位级相等** —— 两拒进链后时序确定性无损。

**T1b 边缘改善实测 ≈ 0(如实登记)**:rej vs norej 同轨迹同切片对照 —— edge_l **−0.06%**、
edge_r **−0.83%**、dark_arch **+0.02%**、dark_table **−0.53%**,四 ROI 全部落在 ±1% 内,
量级低于机态噪声地板。判读:本 dolly 轨迹的 disocclusion 暴露面不足以让两拒产生可测收益,
**「两拒有边缘收益」这一命题在本役素材上未被证实**;机制正确性由 `dolly.pair` 位级门与
kernel 门乘形态(§2.3:阈 0 ⇒ 恒 1,逐字复现 G39 v1 语言形)承担,收益面归留窗
(§四-4.2-4,第一旋钮 = 更激进的 dolly 轨迹或阈值扫描)。

**storm 三条并联硬门通过**:`--window-storm 3 × --lamp-restir on` 30f/warmup4,
rc=0 ∧ VUID=0 ∧ `resize_eras=1`(≥1)∧ `exit_reason="frames_done"`。⇒ G39 D-4 末句
「风暴臂 × restir 组合未验收」**消解**;era 重建首帧 `has_history=0` 语义由 params[74] 门承载。

### 8.7 全段 VUID 与素材完整性

- **VUID = 0 全程**(run3 六个 GPU 段逐跑 `vuid: 0`;`RURIX_VK_VALIDATION=1` 开启校验层)。
- raw 转储完整:ladder 八臂 + dolly 两臂**各 8/8**(`p.raw.f0064`–`f0092` 步长 4),每臂
  205.7MB,合计约 2.0GB,**零素材缺口**;`.gitignore` 已登记不入 git 面。
- 磁盘余量 141.3GB(判读前实测,非事后推断)。
- schema 面:补丁 v2 施用后 `check_schemas` PASS(收役门 guards 段复证)。

### 8.8 签署

- **Assisted-by**: `trae:claude-opus`(主 agent 串行承担侦察 / 实施 / 验收三层;本执行环境无
  子 agent 调用工具,偏差如实登记同 `CAMPAIGN_LOG.md` L5)
- **影响范围**:`kernels/g31_realism.rx`(+91/−31)/ `g31_window_present.rs`(+895/−19)/
  `milestones/g31/g31_texture_sampling_heap_evidence_schema.json`(+24/−1,经补丁)/
  `ci/_patch_g31_window_evidence_schemas_g40.py`(新件)/ `.tmp/night_0901/spv/` 新 SPV(不入 git 面)
- **验证方式**:`gpu_b3_t1.py --only all` 七段(GPU 六段 `gpu_device_lock` 锁内串行,judge 段
  零 GPU 补跑);硬门 = anchors 双锚 / det 两 pair / verify 六判 / ladder.k12_off_anchor /
  dolly.pair / storm 三并联 / judge.k26on_budget,**逐条实测在案**
- **evidence**:`t1_restir2/B3_SUMMARY.json`(judge 段)+ `t1_restir2/b3_log.jsonl`(六 GPU 段
  逐段 `seg_fails: []`)+ `t1_restir2/ab/T1_AB_MATRIX.json` + `t1_restir2/ev/`(逐跑 evidence)
- **诚实边界(三条,均不进全绿叙述)**:①画质收益为负 —— 暗部噪声 2.1–2.7×(§8.5)
  ②T1b 边缘改善 ≈0,收益命题未被证实(§8.6)③`B3_SUMMARY.json` 覆盖面仅 judge 段,
  六 GPU 段绿须查 `b3_log.jsonl`(§8.1)
