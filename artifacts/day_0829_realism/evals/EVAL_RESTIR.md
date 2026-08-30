# EVAL_RESTIR — 零件库 ReSTIR 直接光接入窗口车道成本评估

> Day 0829 真实感战役 Phase 1 评估件。**纯文档,只读代码,零改动零 GPU 跑。**
> 评估对象:把 `kernels/g28_restir.rx` 零件(已实现已验证零接线,TODO #7)接入窗口车道
> (`src/rurix-render/src/bin/g31_window_present.rs` + 统一质量 kernel
> `src/rurix-render/kernels/g31_texture_nrm_gi.rx`)的成本/收益/风险与开窗时机。
> 素材全部为在树文件与 day_0828 在案证据;引用格式 `路径:行号`。

---

## 0. 结论速览

**本窗不接。** 三条硬理由:

1. **收益前提不成立**:窗口车道直接光是**逐盏全算的确定性估计**(bistro 契约 quads=0,
   4 契约点光 + `--lamp-lights` 12 代表点光 = 16 盏逐盏各发 1 条门控阴影射线),
   **零随机方差**。ReSTIR 是"1 样本/px + 时空重用"的**帧时换方差**交易——在 16 盏灯
   规模上,它引入方差、储备内存与锚脆弱面,换回的射线预算(粗估 2–3ms)可由已在位的
   `params[49]` 贡献剔除门(A1 交付,至今未动用)更便宜地拿到。
2. **程序面 fail-closed 字面未解除**:`gi/multi_light.rs:784-796` 的
   `check_restir_trigger`/`restir_serve` 恒 typed Err——"高档 ReSTIR reservoir 须附
   **多灯 workload 证据**,证据不足登记 not-triggered 不充绿"。当前窗口车道 16 盏灯
   不构成该证据;TODO #7 的承接锚是"M100 车道集成窗"(`G31_PLUS_COMMERCIAL_RENDERER_TODO.md:51`),
   属 G32 波次(同文件 :156),不是 Day 0829 窗口。
3. **时域重用与 TSR/AE 双反馈耦合 + 二进制绑定锚教训**(§5/§6):reservoir 的离散
   样本跳变(`u < w/w_sum` 比较翻转即整像素换灯)比 TSR EMA 的连续衰减扰动更陡,
   E1 已定案的"窗口合流臂 presented 锚 = 二进制绑定锚"脆弱面
   (`artifacts/day_0828/e_final/HANDOVER.md:33`)会被放大。

**何时接**(可判定锚,详 §9):`--lamp-k` 提档到 ≥64–256 代表灯(消 12 点光多重硬影,
`HANDOVER.md:25`)或 BistroExterior/动态多灯需求成立,使逐盏全算帧时崩
(实测斜率 ≈0.16ms/灯,bench 口径)——即 multi_light fail-closed 所要求的
"多灯 workload 证据"成立之时,同窗对齐 TODO #87/#108(light cull 算法档,
`G31_PLUS_COMMERCIAL_RENDERER_TODO.md:222/264`)。

**本窗若要吃 ReSTIR 家族的近期收益**,推荐两个不动 reservoir 的低配替身(§9):
GI2 反弹点 kernel 内 RIS 选灯(替换均匀选灯,无跨帧状态),与 44k 灯片 CDF 面光 NEE
(修 `HANDOVER.md:23` 的 quad NEE 缺口)。

---

## 1. 零件库现状盘点(素材已读)

### 1.1 device kernel:`kernels/g28_restir.rx`(127 行)

- **形态 = 逐 trial 单 invocation 的 WRS/RIS 链验证件,不是 per-pixel 渲染 pass**:
  `#[numthreads(1,1,1)]`、dispatch   `[n_trials,1,1]`(`g28_restir.rx:54-55`);
  夹具 = 64 灯环形 `fixture_lights(64)` host 单源上传(`g28_restir.rx:13-14`)+
  单一着色点 `sp.pos=[0,0,0] normal=[0,1,0]` 字面(`g28_restir.rx:83`)。
- **随机流单源纪律**:PCG32 u64 状态整体留 host,kernel 零 RNG 算术;host 预生成
  "已对齐消费序"随机带两条(候选带 = 每 trial 16 个 [0,63] 整数;判定带 = 仅
  w_sum>0 消费点的变长 next_f32 序列)+ 逐 trial offset 三元组表
  (`g28_restir.rx:8-12`)。
- **WRS 链逐字同源 host 金标准**:`estimate_ris/update/unbiased_weight` 逐字复算,
  `u < w/w_sum` 除法比较形不可改写(`g28_restir.rx:104`),unbiased weight 走算术门
  + 安全分母(`g28_restir.rx:113-121`);f32 唯一精度承载(host `w_sum` 是 f64,
  `gi/restir_reservoir.rs:98`;device f64 为 RX6026 构造性拒绝面,`g28_restir.rx:18-19`)。
- **验证在案**(TODO #7 行字面 + evidence):y 整数锚 20000/20000 全等、estimate
  对拍实测 p100=0 零容差零条目(`evidence/g28_restir_device_calibration.json:7`
  protocol 字面)、无偏 3σ、双跑位级、kernel-bias RED 臂。

### 1.2 host 金标准:`gi/restir_reservoir.rs`(334 行)

- `Reservoir { y: usize, phat_y: f32, w_sum: f64, m: u32 }`(:92-101);
  `update`(:114-121)/`merge` 带 m_cap 截断(:124-142)/`unbiased_weight`(:145-150)。
- 方差证据(G21 M-a,`src/rurix-render/src/bin/g21_restir_probe.rs:14-18`,
  64 灯 × 20k trial × M=16 × 时域 8 帧 × m_cap 60):RIS 方差收益 >2×、时域合并再
  >1.2×、无偏 3σ、双跑位级(单测门 `restir_reservoir.rs:291-310`)。
- 空间重用加性臂(纯 host,G28 M-b):8×8 着色点 × von Neumann 4 邻接固定序,
  受点重评快照变换后直调冻结 merge(`g28_restir_device.rs:39-50`);在案数据
  `evidence/g28_restir_spatial_arm.json:1`——**方差再收益 min 0.899 / mean 2.06 /
  max 2.73,不设通过线如实登记**。注意 min<1:平坦区(网格中心 p28/p36)空间重用
  反而增方差,这是接入生产后"空间重用不是白吃"的直接证据。

### 1.3 被冻结的生产面:`gi/multi_light.rs`

- M100 低档 MegaLights(固定随机选 1 灯 NEE×MIS)是生产默认档;
  高档 ReSTIR 服务接口 **恒 fail-closed**:`check_restir_trigger` 恒
  `NotTriggered{reason: "…须附多灯 workload 证据…不充绿"}`(:784-788),
  `restir_serve()` 恒 `Err(RestirNotTriggered)`(:790-796)。
- 接入程序含义:接线不只是写 kernel,还要按 RFC-0038/M100-high 程序解除该登记面
  ——输入 = **多灯 workload 证据**(见 §9 第一步)。

### 1.4 TODO 表锚(`G31_PLUS_COMMERCIAL_RENDERER_TODO.md`)

- **#7 行(:51)**:"ReSTIR 高档 reservoir 车道集成(多灯 GI/DI 高档)|
  `kernels/g28_restir.rx`(y 整数锚 20000/20000 + 无偏 3σ + 空间重用加性臂)|
  被冻结面 `gi/restir_reservoir.rs`、`gi/multi_light.rs`;低档 MegaLights 仍默认档 |
  承接锚 = **M100 车道集成窗**(锚三件之第三件;RFC-0038 out-of-scope 锚)"。
- **G32 波次建议(:156)**:#7 归"G32 画面完整期"(#6–#10、#12 同窗),不在 G31/日窗。
- **#87 行(:222)**:clustered/tiled light cull 与主几何重叠——"算法本体 = #107/#108,
  勿并进 #7";**#108 行(:264)**:GPU light culling——"M100 MegaLights = 固定随机选
  一灯,不是 per-tile cull;勿并入 M100/#7"。三行共同划界:**ReSTIR(#7)、light cull
  算法档(#107/#108)、调度重叠(#87)是三条独立线**,多灯规模成立时三者同窗互补,
  不互相充数。

---

## 2. 窗口车道现状(接入面的地形)

- **pass 结构**:生产五 pass——scene(`g31_texture_nrm_gi.rx` 合流臂换载)→ mv
  (`g14_mv`)→ TSR `g14_8_tsr_{resample,resolve}`(tsrq on 换载
  `g31_tsr_resolve_q.rx`)→ `g31_display_encode`(`g31_window_present.rs:45-57`);
  可选 bloom 4 pass(:138-145)、AE 两微 pass(:179-195)。fg on 扩 8/10 pass(:11-15)。
  **scene 是单 pass megakernel**:主射线 + 直接光 + GGX + GI2 一反弹全部在
  `g31_texture_nrm_gi.rx` 一个 dispatch 内,输出即合成 HDR 单缓冲
  (`g31_texture_nrm_gi.rx:775-777`)+ 深度。
- **直接光形态**(kernel 能力面 vs bistro 实际):
  - quad 面光循环 = 4×4 分层 16 样本/灯、每样本 1 条门控阴影射线
    (`g31_texture_nrm_gi.rx:378-486`)——但 **bistro 契约 quads=0**
    (`g14_3_lane_body.rs:1900-1902` 注释字面"bistro quads=0 ⇒ 空尾段不产组"),
    该循环零迭代。
  - point 灯循环 = 逐盏 delta 灯,每盏 1 条阴影射线,A1 半径截断 + `params[49]`
    贡献剔除门(`g31_texture_nrm_gi.rx:488-566`)。bistro = 4 契约点光;
    `--lamp-lights on` 时 host 聚类 44,024 emissive 三角 → 13 簇 → top-12 代表点光
    append 进 points 面(`artifacts/day_0828/a1_lamp_lights/ACCEPTANCE_SUMMARY.json:19-24`),
    共 16 盏。帧时:bench scene_gpu 0.943→2.876ms(+1.93ms,12 盏增量 ⇒ **≈0.16ms/盏**,
    `ACCEPTANCE_SUMMARY.json:30`)。
  - **关键定性:主命中直接光无随机采样**(唯一随机性 = 相机 jitter)。方差为零,
    代价是灯数线性帧时 + 点光近似伪影(12 点光多重硬影 + 大簇 r=3.11m 遮蔽豁免球,
    `HANDOVER.md:25`)。
- **GI2 加性臂**(`g31_texture_nrm_gi.rx:586-755`):1 反弹余弦半球(R2 序列 +
  frame_idx 旋转)+ 反弹点**均匀随机选 1 盏点光** NEE(R3 Weyl 第三维,:689-746)
  + emission 直取 + firefly clamp。**这是车道内唯一的随机选灯站点**,也是唯一
  "RIS 语义天然适配"的位置。
- **44k 灯片的两条贡献通道**:主命中 = 12 代表点光近似(有损);GI2 反弹 = 反弹射线
  恰好命中灯片 emission 直取,**无 NEE 通道,方差高**(`HANDOVER.md:23` 留窗字面)。
- **params 预算**:scene params 56 f32,[51..55) 已被 GI2 用掉,**仅剩 [55] 一个空槽**
  (`g31_texture_nrm_gi.rx:33-37` + C 相 `artifacts/day_0828/c_gi_r2/ACCEPTANCE_SUMMARY.json:14`)。
- **帧时基线**:`--quality full` 窗口 real_render ≈7.35–7.58ms(run 噪声带,
  `HANDOVER.md:53`;含 8.3MB 强制回读税);90fps 预算 11.11ms ⇒ 余量 ≈3.5ms。

---

## 3. 维度①:reservoir 内存与 pass 结构改动量

### 3.1 reservoir 缓冲(1080p = 2,073,600 px)

| 形态 | 布局/px | 单份 | 说明 |
|---|---|---|---|
| DI 点灯集(16 盏) | `[y, phat_y, w_sum, m]` 4 f32 = 16 B | **31.6 MiB** | g28/host 金标准四元组直译 |
| DI 灯片样本集(44k 三角面光) | `[tri_id, u, v, phat_y, w_sum, m]` 6 f32 = 24 B | **47.5 MiB** | 面光样本须携 uv |
| temporal 双缓冲(prev/cur) | ×2 | 63–95 MiB | TSR history 同律 parity 轮换 |
| + spatial ping-pong | ×3 | 95–142 MiB | 若做空间重用 pass |

对照:texel heap 单 SSBO 282.7 MiB 已在位(B 相),TSR hist_color ≈23.7 MiB。
**显存可承受,非阻断项**;真正贵的是下面的结构改动。

### 3.2 pass 结构:两个方案

**方案 A(最小侵入,kernel 内嵌 temporal RIS,无 spatial pass)**——scene 仍单 pass:

- kernel 内:M 候选 RIS(闭式 R2/R3 随机,C 相先例)→ 读 prev reservoir(上帧
  世界坐标重投影:kernel 已有命中点 `hx/hy/hz`,需 **prev 帧 VP 矩阵 16 f32**)→
  merge(m_cap)→ 对保留样本发 1 条验证阴影射线 → 写 cur reservoir + 着色。
- pass 数不变(5 pass 维持);新增 2 个 reservoir SSBO(parity 轮换)+ prev VP 参数。
- 改动清单:kernel fork(≈+150–250 行,GI2 段当量)、`pack_frame_params_*` 链环
  加 prev VP(**params 56 槽只剩 [55],必扩容**——扩容属新形态另路,off 臂维持旧
  56 f32 上传保锚,C 相 SPV 隔离同律)、lane_body 装配 + descs 组合面、双 bin CLI。

**方案 B(标准三段式:initial/temporal → spatial ×1–2 → final shade)**:

- scene megakernel 必须拆:G-buffer pass(输出 pos/nrm/albedo/prim ≈40 B/px
  ≈79 MiB)→ ReSTIR passes → shade pass(GGX/GI2/环境光/emissive 回填)。
- 5 pass → 8–9 pass;emissive 贴图/纹理采样要么进 G-buffer(带宽翻倍)要么 shade
  pass 重采样(重复 fetch)。**这是"车道结构改动大"档**,等价于把 day_0828 六相
  合流成果重排一遍——B/C/F 相的合流臂(tex/nrm/em/gi2)全部要在新 pass 序下重验锚。

**判定:若接,只考虑方案 A。** 方案 B 的收益(spatial reuse)在案数据本身存疑
(§1.2:方差再收益 min 0.899,平坦区负收益),不值 3+ pass 的结构税。

### 3.3 descs 组合面税(两方案共有)

窗口 scene descs 已有 ≥7 个形态(base/nrm/tex/tex_nrm/tex_bloom/tex_nrm_bloom/
tex_nrm_em(+bloom)),AE 三件下标随形态顺延(`g31_window_present.rs:912-927`);
新增 2 个 reservoir SSBO = 每个可组合形态都要扩下标矩阵。战役先例是新臂以
fail-closed 互斥起步(gi2 与 fg/hzb/svt/slab/cluster/wp 互斥),ReSTIR 臂同律:
**首版只接 tex_nrm(+gi2) 合流形态,其余互斥**,组合面归后续波。

---

## 4. 维度②:与现 quad NEE / lamp-lights 的替换或共存

### 4.1 替换关系逐条

| 现有件 | ReSTIR 接入后 | 判定 |
|---|---|---|
| point 灯逐盏循环(16 盏 ×1 阴影射线) | 被 ReSTIR DI 替换(M 候选 phat ALU + 1–2 验证射线) | 16 盏规模**负收益**(见 4.2) |
| quad 4×4 分层循环 | bistro quads=0 零迭代,无交互 | 不动 |
| `--lamp-lights` host 聚类(44k→12) | **共存且是前置**:ReSTIR 的灯集仍需 host 产出(代表灯几百盏,或灯片 CDF/alias 表) | 聚类粒度从 0.6m 收细即产多灯 workload |
| `params[49]` 贡献剔除门(未动用) | 被 ReSTIR 的 phat 重要性隐式吸收 | ReSTIR 前先动用它,是更便宜的帧时旋钮 |
| GI2 反弹点均匀选灯(:689-746) | 可独立升级为 kernel 内 RIS(不需要 reservoir) | **推荐的低配替身**(§9) |

### 4.2 "44k 灯片直接光方差收益"的量化判断

- **主命中通道**:现在 12 代表点光是零方差近似,误差表现为**偏差**(多重硬影/大簇
  遮蔽豁免,`HANDOVER.md:25`),不是方差。ReSTIR 以 44k 灯片为样本集才能消这些偏差
  (真面光软影),但那是"灯规模 16 → 44k(或数百代表灯)"的换代,收益随灯数增长:
  - 逐盏全算帧时斜率实测 ≈0.16ms/盏(bench tier100 口径,§2)⇒ K=64 ≈ +10ms,
    K=256 完全不可行——**K≥~32 起逐盏全算崩,MegaLights/ReSTIR 才成为唯一解**。
  - K=16 现状下,RIS M=16 候选 ≈ 全灯扫描,ReSTIR 除引入方差外无信息增益。
- **GI2 反弹通道**:`HANDOVER.md:23` 在案——反弹点对 44k 灯片零 NEE,贡献靠命中
  直取,方差高。这里的正解优先序:①反弹点 RIS 选灯(12→16 盏内做 phat 加权,
  kernel 内闭式,无状态)②灯片 CDF 面光 NEE(host 建 44k 三角通量 CDF 一次上传,
  kernel 二分采样 1 样本)③full ReSTIR(样本跨帧重用)。①②各约一个昨日臂当量,
  ③才需要本评估的全部成本。**方差收益的 80% 在①②就能拿到**(C 相同类换代先例:
  旧 sin hash→R2 已把 GI 噪声 std_p95 压 −77~−90%,
  `artifacts/day_0828/c_gi_r2/ACCEPTANCE_SUMMARY.json:52`)。

### 4.3 与 TSR 质量档的既有战果对照

D 相已把 gi2 微光点压掉 −87~−97%(`artifacts/day_0828/d_tsr/d_metrics.json:486-487`:
dark_arch std_p95 drop 97.03%、dark_table 87.35%)。ReSTIR 若接在主命中直接光上,
反而**新增**一路 1 spp 方差源喂给 TSR——它必须自带时域重用把方差压回去,这正是
§5 的风险面。净画质收益在 16 盏灯规模下无法为正。

---

## 5. 维度③:时域重用 × TSR 反馈的交互风险

车道已有两级跨帧反馈:TSR EMA(α=0.02 稳态档)+ AE 曝光 EMA(encode params[133])。
ReSTIR temporal reuse 是**第三级**,且性质更陡:

1. **离散 vs 连续**:TSR 扰动是连续衰减(EMA 指数遗忘,ULP 扰动随 α 衰减);
   reservoir merge 的保留样本 y 是**离散跳变**——`u < w/w_sum`(除法比较形,
   `g28_restir.rx:104`)一次翻转 = 整像素直接光换灯源 = 帧间亮度阶跃,再被 TSR
   历史记忆 ~50 帧(α=0.02 时间常数,`d_tsr/ACCEPTANCE_SUMMARY.json:95`)。
   m_cap 截断置信(`restir_reservoir.rs:124-142`)限制历史权重但**不消除离散性**。
2. **闪烁通道对撞**:TSR resolve 的闪烁时域分析(死区符号翻转 EMA score,
   `g31_tsr_resolve_q.rx:106-121`)会把 reservoir 换灯的阶跃判为闪烁 → score 升 →
   α 收紧 + AABB 松弛(relax,:252-255)→ 降噪能力反被让渡。tsrq 的邻域亮度 clamp
   (params[20])若为压 ReSTIR 闪烁而启用,又会误杀合法孤立小灯(D 相已登记该权衡,
   `g31_tsr_resolve_q.rx:26-28`)。
3. **AE 三级耦合**:直接光换源引起全帧均值波动 → AE 增益 EMA 跟随 → 全帧亮度呼吸。
   A2 验收的"零振荡"曲线(`CAMPAIGN_LOG.md` Round 2)在新增随机直接光下需要重验。
4. **disocclusion 语义**:重投影失效像素(出屏/深度拒)reservoir 必须归零重启,
   1 spp 直接光在新露面区域裸奔——TSR 对 disocclusion 也正好走 passthrough
   (`g31_tsr_resolve_q.rx:290-291` 同律),两层同时失效 = 运动边缘直接光噪声完全
   无时域保护。dolly 轨迹验收(D 相 :59-63 协议)大概率首轮红。

**缓解设计**(若开工时采用):反馈解耦——首版 temporal merge 的 m_cap 取小(8–16,
host 金标准实验域 60 偏大),并给 ReSTIR 输出加 per-pixel phat 归一化钳制;
验收协议必须含 dolly 240f 曲线 + 静态 conv 双协议(D 相 :41 同律)。

---

## 6. 维度④:双跑位级一致性

### 6.1 可保面

- **同二进制同输入双跑位级一致可以保住**:随机性全走闭式序列(R2/R3 + frame_idx,
  C 相先例位级验收在案)或 host 预生成带;reservoir 演化 = 确定性状态机;digest
  序列(逐帧 BGRA8)仍确定。GI2/AE 两个跨帧反馈臂的双跑验收先例直接套用
  (`c_gi_r2/ACCEPTANCE_SUMMARY.json:36-42`)。

### 6.2 不可迁移面

- **g28 的随机协议不可搬**:host 预生成带按窗口口径 = 2,073,600 px × M16 候选带
  + 判定带 + offset 表 ≈ **>132 MB/帧 host 生成 + 上传**,不可行。必然换 R2/R3
  闭式 ⇒ **g28 的"零接线已验证"只覆盖 WRS 链本体,不覆盖接入后的随机形态**;
  y 整数锚/p100=0 对拍那套证据在新形态下要重建(host 镜像臂重写,约半个臂当量)。
- **f32 w_sum 精度域**:host 金标准 w_sum 是 f64(`restir_reservoir.rs:98`),
  device 是 f32(RX6026);g28 夹具 M=16 单帧无碍,但 temporal merge 跨帧累加
  w_sum 的 f32 舍入随 m_cap 窗口滚动——比较形 `u < w/w_sum` 对 ULP 敏感,
  舍入差可翻转 y(§5.1 同一敏感点的确定性侧面)。

### 6.3 锚脆弱面(核心风险)

E1 定案:窗口纹理合流臂 presented 锚 = **二进制绑定锚**——宿主二进制重建的 ULP
扰动经 TSR/AE 反馈放大,曾致 presented digest 漂移(d89848b9 事件;
`HANDOVER.md:33`,律 = "重建后组合臂锚一律先复验再消费;可跨重建对锚面仅 all-off +
bench 全系")。reservoir 状态跨帧演化把这个面**从"连续放大"升级为"离散雪崩"**:
ULP → y 翻转 → 换灯 → TSR 记忆 → 后续每帧 digest 全变。结论:

- 双跑门可绿,但 **ReSTIR 臂锚一律按"最脆弱组合臂"管理**(E1 纪律的最保守形态);
- soak/回归矩阵里 ReSTIR 臂的跨重建复验成本 ≈ 现九臂 full 锚的复验成本再翻一档;
- C 相已确立的纪律直接适用:**新增 ray query 站点(ReSTIR 验证射线即是)必走 SPV
  字节隔离,gate=0 恒等不可依赖**(`c_gi_r2/ACCEPTANCE_SUMMARY.json:21-25`)。

---

## 7. 维度⑤:SPV / params / 侧表预算

| 预算面 | 现状 | ReSTIR 方案 A 需求 | 处置 |
|---|---|---|---|
| SPV 工件 | scene 三工件并存(基线 `g31_texture_nrm_gi.spv` fd22cb19 / `*_gi2.spv` 75d08aec / `*_em.spv` bdd23a3a,`HANDOVER.md:49`) | 第 4 工件 `*_restir.spv`(仅 on 臂换载) | C/F 相字节隔离纪律照抄,off 臂 0-byte |
| scene params | 56 f32,仅 [55] 空(`g31_texture_nrm_gi.rx:33-37`) | prev VP 16 + M/m_cap/门/clamp ≈ 21+ 槽 | **必须扩容 params 或新开小 SSBO**;新形态另路上传,off 臂维持 56 f32 保锚 |
| 新 SSBO | — | reservoir ×2(parity)+ 可选灯片 CDF 表 | 31.6–47.5 MiB ×2;descs 尾挂,首版互斥起步(§3.3) |
| 灯表 | points 8 f32/盏(槽 6 radius/槽 7 pack,A1) | 复用;灯片集需 44k×CDF 表(≈0.7 MB) | host 装配一次,双跑确定性由聚类先例承载 |
| tsr_params | [21..32) 11 槽 reserved(`g31_tsr_resolve_q.rx:38-41`) | 无需求(ReSTIR 不动 TSR 参数面) | 留给 EVAL_DENOISE 的方差引导档 |
| encode params | [133] AE 已用 | 无需求 | — |

---

## 8. 维度⑥:预估工期与红修风险

尺度:昨日单臂(kernel 段 + 双车道 CLI + 全锚验收)= 数小时——A1 ≈50min、
C 相 GI2 ≈1.5h、D 相 tsrq ≈2h **且吃掉两轮红修额度**(`CAMPAIGN_LOG.md` Round 5/6,
`d_tsr/ACCEPTANCE_SUMMARY.json:18-21`)。

| 工作项(方案 A) | 当量 | 红修风险 |
|---|---|---|
| workload 证据实验(lamp-K 阶梯,不写 kernel) | 0.5 臂 | 低 |
| kernel fork:RIS + prev VP 重投影 + merge + 验证射线 | 1.5 臂 | 中(branchless 纪律 + rurixc「if 包 while」缺陷绕行,A1/C 先例) |
| host 侧:params 扩容/reservoir 装配/descs/CLI/host 镜像对拍臂 | 1–1.5 臂 | 中 |
| 验收:全锚零漂移 + on 双跑 + dolly + conv 指标 + soak 抽检 | 1 臂 | **高**——三级反馈耦合(§5),D 相单反馈都两轮红修;dolly disocclusion 首轮红概率大 |
| **合计** | **4–4.5 臂 ≈ 1–1.5 个满日** | 整体偏高 |

方案 B(拆 pass 标准 ReSTIR)≈ 8–10 臂当量 ≈ 3 日 + 全部合流臂锚重排,本评估直接排除。

---

## 9. 维度⑦:结论——接 / 不接 / 何时接

### 9.1 判定

- **Day 0829 窗口:不接 full ReSTIR。** 收益前提(直接光方差)在 16 盏灯 + 逐盏
  全算形态下不存在;成本(4+ 臂当量、三级反馈耦合、锚脆弱面升档)确定存在;
  程序面(multi_light fail-closed + TODO #7 M100 承接锚 + G32 波次归属)也不在本窗。
- **对照 TODO #7 承接锚**:M100 车道集成窗的输入 = 多灯 workload 证据。**先造证据,
  再开集成**——顺序不可倒。

### 9.2 何时接(可判定开窗条件,三选一即开)

1. `--lamp-k` 需求提档 ≥64(消多重硬影/大簇伪影的画质需求,`HANDOVER.md:25`),
   且 lamp-K 阶梯实测证明逐盏全算超帧预算(斜率 0.16ms/盏 ⇒ K=64 约 +10ms);
2. BistroExterior / 动态多灯场景入压测闭集(TODO #27 SMRT 同窗条件,:99);
3. GI 默认档(#12)把反弹 NEE 升到多样本,反弹点选灯成本成为热点。

开窗时的形态推荐:方案 A(kernel 内嵌 temporal RIS,m_cap 8–16,spatial 不做——
在案数据 min 0.899 不支持)、首版与 bloom/AE 之外的重臂互斥、SPV 第 4 工件隔离、
锚按最脆弱组合臂管理。

### 9.3 本窗低配替身(若想现在就吃收益)

1. **GI2 反弹点 RIS 选灯**(1 臂当量):`g31_texture_nrm_gi.rx:694-695` 的均匀选灯
   `gi_psel = u3·point_count` 升级为 M=4–8 候选 phat 加权 RIS(闭式 R3 驱动,
   无跨帧状态、无新 SSBO、params 用 [55] 一槽装 M)——直接压 `HANDOVER.md:23`
   登记的反弹 NEE 方差,与 host `estimate_ris` 语义同源可对拍。
2. **44k 灯片 CDF 面光 NEE**(1 臂当量):host 按通量建三角 CDF(一次装配),
   反弹点(或主命中作为第 17 "虚拟灯")二分采样 1 样本——修 quad NEE 缺口本体,
   软影方向也顺带受益。

### 9.4 若开工,第一步做什么

**不写 kernel。先跑 workload 证据实验(≈0.5 臂,纯既有旗标 + host 聚类参数)**:

1. 把 `extract_lamp_lights` 聚类网格 0.6m 收细(0.3m/0.15m)产出 K=32/64/128 档
   代表灯(`g14_3_lane_body.rs` LampOpt 面,改参数不改算法);
2. bench 口径跑 K 阶梯 scene_gpu 曲线(对照 A1 的 0.16ms/盏斜率),窗口口径跑
   `--quality full` + K 档 real_render;
3. 视觉对照:多重硬影随 K 的消退 vs 帧时曲线交点;
4. 产出 `restir_workload_evidence.json`(K 阶梯帧时表 + 伪影 ROI 对照)——它同时是:
   multi_light fail-closed 解除的程序输入、TODO #7 M100 集成窗的立项依据、以及
   "本评估不接判定"的复核面(若 K=16 已满足画质需求且帧时平,则 ReSTIR 继续搁置)。

---

## 附:本评估读过的素材清单

| 文件 | 用途 |
|---|---|
| `src/rurix-render/kernels/g28_restir.rx` | 零件本体形态 |
| `src/rurix-render/src/gi/restir_reservoir.rs` / `gi/multi_light.rs` | host 金标准 + 冻结生产面 |
| `src/rurix-render/src/bin/g28_restir_device.rs`、`evidence/g28_restir_{device_calibration,spatial_arm}.json` | device 对拍/空间臂在案数据 |
| `src/rurix-render/src/bin/g21_restir_probe.rs`(G21 M-a 门件) | 方差收益证据形态 |
| `G31_PLUS_COMMERCIAL_RENDERER_TODO.md` :51/:156/:222/:264 | #7 承接锚 / G32 波次 / #87/#108 划界 |
| `src/rurix-render/src/bin/g31_window_present.rs`、`src/rurix-render/src/bin/g14_3_lane/g14_3_lane_body.rs:1897-1943` | 车道 pass 结构 / bistro quads=0 |
| `src/rurix-render/kernels/g31_texture_nrm_gi.rx` | 单 pass megakernel 直接光/GI2 形态 |
| `artifacts/day_0828/a1_lamp_lights/ACCEPTANCE_SUMMARY.json`、`c_gi_r2/ACCEPTANCE_SUMMARY.json`、`d_tsr/{ACCEPTANCE_SUMMARY,d_metrics}.json`、`e_final/{HANDOVER.md,E_ACCEPTANCE_SUMMARY.json}` | 帧时/噪声/锚纪律在案数据 |
