# EVAL_DENOISE — 更强时空降噪接入窗口车道评估

> Day 0829 真实感战役 Phase 1 评估件。**纯文档,只读代码,零改动零 GPU 跑。**
> 评估对象:在窗口车道(`src/rurix-render/src/bin/g31_window_present.rs` 五 pass +
> TSR 质量档 `src/rurix-render/kernels/g31_tsr_resolve_q.rx`)之上引入更强时空降噪
> 的路线选型:SVGF 类(方差引导 + A-trous 多迭代)vs 轻量双边空间滤波 vs 现
> TSR+tsrq 增强。素材全部为在树文件与 day_0828 在案证据;引用格式 `路径:行号`。

---

## 0. 结论速览

**本窗不上任何新降噪 pass。推荐四级阶梯,按噪声源出现顺序逐级开窗:**

1. **第 0 级(零代码,本窗可做)**:实测 tsrq 邻域亮度 clamp K 档——kernel/CLI 全部
   已接线(`g31_tsr_resolve_q.rx:26-28` params[20],`--tsrq-clamp`),D 相以 K=0
   评估、登记为"后备旋钮"未实测(`artifacts/day_0828/e_final/HANDOVER.md:26`)。
2. **第 1 级(≈1 臂)**:tsrq v4 方差引导稳态档——SVGF 的**时域半段**(luma 二阶矩
   驱动 per-pixel α)嵌进现 resolve kernel,走 tsr_params [21..32) 11 个 reserved 槽
   (`g31_tsr_resolve_q.rx:41`),零新 pass、一个新历史缓冲(≈7.9 MiB)。
3. **第 2 级(≈1.5 臂)**:轻量双边单 pass(5×5,深度/法线/暗区门控)——**开窗锚 =
   新噪声源臂(PCSS 软阴影 / 光泽反射 / GI spp 升档)落地且其 conv std_p95 回到
   ≥1e-3 量级**。现在没有客户:tsrq 已把 gi2 微光点压 −87%~−97%
   (`artifacts/day_0828/d_tsr/d_metrics.json:486-487`),残余 4.9e-5~1.3e-4,
   距 gi2-off 基线 1.1×~6.7×(§2.1 表),D 相判词"已接近 gi2-off 量级"。
4. **第 3 级(≥3 臂,车道结构改动)**:SVGF 全家桶 + separate GI 通道——**90fps
   预算下不可行**(估算 2.3–4.3ms,余量仅 ~3.5ms,§3),开窗锚 = GI 多反弹默认档
   (TODO #12)或光泽反射臂立项且第 1/2 级 measured 不够用;NRD vendor 维持
   TODO #29 锚字面"自研降噪画质差距 measured 检出"
   (`G31_PLUS_COMMERCIAL_RENDERER_TODO.md:101`)。

**两条在案遗留的正解不在降噪器**(§4):"地板尾像素噪 −18% 未达 30%"是相关性低频
jitter 响应分量,EMA 与空间滤波原理性都不可及(`HANDOVER.md:22` 字面即写明"需采样
密度/jitter 感知滤波面");"GI2 反弹无 quad NEE 方差高"(`HANDOVER.md:23`)的正解是
**修估计器**(反弹点 RIS 选灯 / 灯片 CDF NEE,见姊妹篇 `EVAL_RESTIR.md` §9.3),
方差从源头收缩比末端滤波便宜且无糊像素代价。

---

## 1. 预算与车道现状

- **帧预算**:90fps = 11.11ms;`--quality full` 窗口 real_render ≈7.35–7.58ms
  (run 噪声带,`HANDOVER.md:53`,含 8.3MB 强制回读税)⇒ **余量 ≈3.5ms**。
  60fps 口径(16.67ms)余量 ≈9ms。
- **现有 post 链开销参照**(全部在案 measured):
  - TSR resample+resolve 合计 ≈0.539–0.542ms(bench 160f upscale 口径,
    `artifacts/day_0828/d_tsr/ACCEPTANCE_SUMMARY.json:67`)⇒ 单个 1080p 全屏
    3×3 邻域 pass ≈0.3ms 量级——后文新 pass 帧时估算的标尺;
  - tsrq on 增量 ≈0(0.5388→0.5422ms,同 :67-68);
  - AE 两微 pass 0.11–0.19ms(单 workgroup 结构代价,`HANDOVER.md:27`);
  - bloom 四 pass(半分辨率)在 full 档常开。
- **时域降噪现状**(`g31_tsr_resolve_q.rx`,D 相交付,312 行 fork,字节隔离换载):
  1. Karis 反亮度加权混合(:11-16,压 HDR 尖峰注入,收敛值偏暗有偏已登记);
  2. 稳态 alpha 档 params[19](:17-25,默认 0.02,驻态残差 ∝ √(α/(2−α)));
  3. 邻域亮度 clamp params[20](:26-28,**K=0 关,未实测**);
  4. 深度验证 3×3 膨胀区间化(:29-36,v3 决定性红修——深度边缘像素不再随 jitter
     恒拒史)。
  参数余量:tsr_params [21..32) 全 reserved(:41)。
- **scene 输出形态**:单缓冲合成 HDR——`out_color = hit_f·(em + al·π⁻¹·dir +
  al·amb + spec + al·gi) + sky`(`g31_texture_nrm_gi.rx:775-777`),GI2 贡献
  `al·gi` 已乘 albedo 并入;深度另一缓冲。**没有独立的间接光/albedo 通道**——
  这是 §5 拆通道讨论的地形。

---

## 2. 在案噪声画像(谁还在响,响多大,什么形状)

### 2.1 幅值:tsrq 之后还剩多少(conv 协议 std_p95,绝对幅值)

四臂在案(`d_tsr/ACCEPTANCE_SUMMARY.json:42-47`,单位 = scene-linear luma):

| ROI | ①snrm 基线 | ③gi2 c001 | ④gi2+tsrq | ④ vs ③ | ④ vs ①(残余倍数) |
|---|---|---|---|---|---|
| wall | 1.93e-5 | 5.38e-4 | 1.30e-4 | −75.8% | 6.7× |
| floor | 3.00e-5 | 2.09e-3 | 5.69e-5 | −97.3% | 1.9× |
| dark_arch | 1.29e-5 | 1.66e-3 | 4.94e-5 | −97.0% | 3.8× |
| dark_table | 5.89e-5 | 5.25e-4 | 6.64e-5 | −87.3% | 1.1× |

判读:**gi2 的 1 spp 微光点已被 tsrq 压到接近关臂基线**(D 相原话"已接近 gi2-off
量级",:87)。e-5 量级在 ACES ×16 曝光域后视觉阈下——**现有九/十臂组合里没有值得
再上一个降噪 pass 的噪声源**。

### 2.2 未达标残余的"形状"(决定哪种滤波器有效)

- **地板尾像素 −18%(目标 30%)**:D 相判词 = "相关性低频 jitter 响应分量,EMA 档
  原理性不可滤——需采样密度/jitter 感知滤波面"(`d_tsr/ACCEPTANCE_SUMMARY.json:48`,
  `HANDOVER.md:22`)。空间自相关佐证:gi2_c001 时域 std 图的 lag-1 自相关
  wall x=0.993/y=0.879(`artifacts/day_0828/c_gi_r2/c_spatial_char.json:26-28`)
  ——方差场高度空间平滑 = 低频结构,**3×3/5×5 空间核与 A-trous 小迭代都够不着**;
  强行大核滤会先糊掉 D 相刚赢回来的真 AA(收敛帧高通能量 −27%~−57%,
  `d_metrics.json:488-509`)。这条残余的正解在采样侧,不在滤波侧。
- **dark_table/dark_arch 稀疏微光点**:lag-1 x=0.482/y=0.383(table)、
  x=0.381(arch x 向)(`c_spatial_char.json:31-42`)——接近白噪,**空间滤波可及**
  (√N 平均收益成立)。但幅值已在 e-5(2.1 表),暂无动机。
- **rel_p95 比值门教训**:temporal_rel_p95 对 1 spp 随机估计器不可达(off 基线
  近零、比值尺度不变——gi2_on/off 比值 14×~78×,
  `c_gi_r2/c_metrics.json:425-446`;判据改绝对幅值,
  `c_gi_r2/ACCEPTANCE_SUMMARY.json:51`)。**未来任何降噪臂的验收判据沿用
  conv 协议 std_p95 绝对幅值 + 高通能量不升(锐度门)**,不再踩比值门。

---

## 3. 维度①:三路线的 pass 数 / 带宽 / 帧时对比

标尺:1080p(2,073,600 px)全屏 3×3 邻域 pass ≈0.3ms(§1 resolve 实测推算);
f32 RGB 全屏读或写一遍 ≈23.7 MiB 流量。

| 路线 | 新 pass 数 | 新缓冲 | 每帧新增带宽(读+写) | 帧时估算 | 90fps 余量(3.5ms)判定 |
|---|---|---|---|---|---|
| **A. tsrq 增强**(K 档实测 + v4 方差引导 α) | **0**(嵌现 resolve) | +1 luma² 历史 ≈7.9 MiB | ≈+16 MiB | **≈0**(tsrq 先例 :67-68) | 无压力 |
| **B. 轻量双边空间滤波**(resolve 后单 pass 5×5,深度/法线/luma 引导 + 暗区门控) | +1 | ping 缓冲 23.7 MiB | ≈24 MiB×25 tap 加权读 ≈ 有效 ~70–120 MiB | **≈0.4–0.8ms** | 可容纳 |
| **C. SVGF 类**(moments/方差估计 1 pass + A-trous 5 迭代 + 时域 moments 历史) | +6 | 方差/moments ×2 + ping-pong ≈40–60 MiB | ≈300–500 MiB | **≈2.3–4.3ms**(moments ~0.3 + 5×[0.4–0.8]) | **吃满或超出余量;不可行**(60fps 档才可谈) |

补充成本(表外):

- C 路线还有 §5 的**结构前提**(separate GI 通道 + demodulate albedo),否则
  A-trous 的 edge-stopping 会把 B 相纹理战果(色块 −97.84%)与阴影边缘一起当噪声
  抹掉——SVGF 论文形态就是对 demodulated irradiance 滤波。把这个前提算进去,
  C 路线真实成本是"scene 输出拆分 + TSR 双通道历史 + 滤波链"三件套(§5)。
- B 路线的隐藏代价:糊纹理/糊 AA 风险,必须窄门控(仅暗区 luma < 阈 + 深度/法线
  edge-stopping + 强度上限);对 2.2 判读的低频相关分量无效——**它只对"未来白噪
  新臂"有客户价值**。

---

## 4. 维度②:与噪声源臂的匹配度(谁受益,谁不受益)

| 噪声源 | 形状(在案) | tsrq 现状 | A(增强) | B(双边) | C(SVGF) | 正解 |
|---|---|---|---|---|---|---|
| gi2 1 spp 微光点(现网) | 稀疏白噪,e-5 残余(§2.1) | 已 −87~−97% | K 档可再压萤火虫 | 可及但无动机 | 杀鸡用牛刀 | 已解决;残余走第 0 级 K 档 |
| 地板尾低频 jitter 响应(现网,−18% 未达标) | 低频、帧间相关(lag-1 ~0.99) | 原理性不可滤(:48) | 方差引导也不可及(不是方差问题) | 不可及 | 不可及(小核);大核糊 AA | **采样密度 / jitter 感知**(resolve 内按 jitter 相位补偿)——独立留窗,不算降噪器 |
| GI2 反弹无 quad NEE(现网,`HANDOVER.md:23`) | 高方差通道(灯片贡献靠命中直取) | clamp 0.01 压峰但有偏 | — | — | — | **修估计器**:反弹点 RIS 选灯 / 44k 灯片 CDF NEE(`EVAL_RESTIR.md` §9.3,各 ≈1 臂) |
| PCSS 软阴影(未来臂,`HANDOVER.md:25` 面光/PCSS 留窗) | penumbra 随机采样 → 白噪 | α 档可部分吸收 | 方差引导受益 | **主要客户**(penumbra 空间平滑先验强) | 过度 | B 级开窗触发器之一 |
| 光泽 GGX 反射 1 spp(未来臂) | 高方差 + 各向异性斑 | 时域拖尾风险 | 不够 | 部分 | **主要客户**(需方差引导 + 多迭代) | C 级开窗触发器 |
| GI spp 升档 / 多反弹默认档(TODO #12) | 宽谱方差 | 不够 | 部分 | 部分 | **主要客户** | C 级开窗触发器 |
| ReSTIR 直接光(若接,姊妹篇) | 离散换灯阶跃 | 闪烁通道对撞(EVAL_RESTIR §5) | score/relax 已有 | 无效(阶跃非白噪) | 无效 | m_cap/钳制在 ReSTIR 侧解决 |

结论:**降噪器投资必须跟着噪声源臂走**。现网臂全部已被 tsrq 覆盖或不属于滤波问题;
B/C 两级的客户(PCSS/反射/GI 升档)都还没立项——先建平台就是空转。

---

## 5. 维度③:separate GI 通道降噪 vs 合成后降噪

### 5.1 现状约束

scene megakernel 单缓冲合成输出(`g31_texture_nrm_gi.rx:775-777`),GI2 贡献在
kernel 内是独立累加器 `gi_r/g/b`(:748-753),**乘 albedo 之前可低成本另写一份**
——即"半分离"改法:kernel 尾加 `out_gi` 输出(+1 SSBO,23.7 MiB),direct/emissive/
spec 仍走主缓冲。kernel 侧改动小;贵的在下游:

### 5.2 两形态对比

| | 合成后降噪 | separate GI 通道降噪 |
|---|---|---|
| scene kernel | 0-byte(读现输出) | 尾加 out_gi 输出(新 SPV,字节隔离) |
| TSR | 不动 | **双通道历史**:GI 通道独立 resample/resolve(或滤波后再合成进主通道)——resolve fork 双通道版 + hist 缓冲 ×2 |
| descs/组合面 | +1 pass 尾挂 | 全链扩(scene/tsr/encode 三段都动),≈方案 B 级结构改动 |
| 滤波质量 | edge-stopping 只能靠深度/法线/luma,纹理会被当边缘保下来但**阴影边缘与纹理噪声不可分**,须暗区窄门控 | demodulated irradiance 上滤波,纹理零伤害,SVGF 标准形态 |
| 工期当量 | ≈1.5 臂 | ≈4–6 臂(含全锚重验) |

### 5.3 判定

- **B 级(轻量双边)按合成后形态落**,窄门控换低成本——它的客户(PCSS penumbra)
  本来就在暗区/半影区,门控代价可接受。
- **C 级(SVGF)必须拆通道**,所以 C 级的真实开窗条件 = "值得为一个噪声源臂做
  车道结构改动"——这只在 GI 多反弹默认档(#12)这种长驻大方差源上成立,与其
  G32 波次归属一致(`G31_PLUS_COMMERCIAL_RENDERER_TODO.md:156`)。
- 若将来拆,**半分离改法优先**(out_gi 尾加),不要走"G-buffer 全拆"——后者是
  `EVAL_RESTIR.md` §3.2 方案 B 同一个大坑。

---

## 6. 维度④:双跑位级与字节隔离纪律下的落地形状

任何一级开工都套用 day_0828 已固化的纪律,零新发明:

1. **SPV 字节隔离**:新 kernel(或 resolve fork v4)独立编译新工件,off 臂恒载
   既有锚定字节——C 相纪律修订原文:"kernel 演进含新 ray query 站点/超越函数时
   gate=0 恒等不可依赖,保锚一律走字节隔离"
   (`artifacts/day_0828/c_gi_r2/ACCEPTANCE_SUMMARY.json:100`)。纯 ALU 尾加
   (tsrq v4 属此类)理论上可依赖 A1 门乘先例,但按 D 相实操仍走独立 SPV 换载
   (`d_tsr/ACCEPTANCE_SUMMARY.json:8`)。
2. **参数走 reserved 槽,零新绑定优先**:tsr_params [21..32) 11 槽
   (`g31_tsr_resolve_q.rx:41`);逼不得已加缓冲时 descs 致密尾挂 + 首版与重臂
   fail-closed 互斥(`g31_window_present.rs:912-927` 下标顺延纪律)。
3. **验收判据模板**(D 相 9/9 全套):全锚零漂移(all-off 55e4a92d / full 十臂
   78113d56 / bench c1d28ad7 系,`HANDOVER.md:48` + `e_final/E_ACCEPTANCE_SUMMARY.json:20-28`)
   + on 双跑位级 + validation 静默 + **conv 协议 std_p95 绝对幅值**(≥30% 类目标)
   + **收敛帧高通能量不升**(锐度门,防糊)+ dolly 240f 拖影三帧对照。
4. **锚管理**:降噪臂属组合臂,按 E1"二进制绑定锚"最保守纪律执行——重建后先复验
   再消费(`HANDOVER.md:33`)。
5. **跨帧状态臂的双跑口径**:方差引导 α 引入 luma² 历史 = 又一跨帧反馈,双跑验收
   口径与 AE/tsrq 同律(固定输入位级一致;resize era 重建状态归零再适应,
   `g31_window_present.rs:188-189` AE 先例)。

---

## 7. 维度⑤:结论——推荐路线与开窗时机

| 级 | 内容 | 帧时 | 工期 | 开窗时机 |
|---|---|---|---|---|
| 0 | tsrq K 档实测定档 | 0 | ~1 小时,零代码 | **本窗即可**(§8) |
| 1 | tsrq v4 方差引导稳态档(时域 moments → per-pixel α) | ≈0 | ≈1 臂 | 下一个噪声臂立项同窗;或 K 档实测显示萤火虫残余仍碍眼 |
| 2 | 轻量双边单 pass(合成后,暗区窄门控) | 0.4–0.8ms | ≈1.5 臂 | **PCSS 软阴影臂**(`HANDOVER.md:25` 留窗)或任何新臂 conv std_p95 回到 ≥1e-3 |
| 3 | SVGF(拆 GI 通道 + 方差引导 + A-trous) | 2.3–4.3ms | ≥4–6 臂 | GI 多反弹默认档(#12)/光泽反射臂立项,且 1/2 级 measured 不够;**只在 60fps 档预算下考虑** |
| — | NRD vendor(#29) | — | — | 维持锚字面:自研降噪画质差距 measured 检出(`G31_PLUS_COMMERCIAL_RENDERER_TODO.md:101`) |

配套但**不算降噪器**的两条(优先级高于 2/3 级,因为它们收缩方差源头):

- GI2 估计器缺口修复(反弹点 RIS 选灯 / 灯片 CDF NEE)→ `EVAL_RESTIR.md` §9.3;
- 地板尾 jitter 响应 → jitter 感知 resolve / 采样密度,独立留窗
  (`HANDOVER.md:22` 字面)。

**不推荐**:本窗直接上 SVGF(无客户 + 90fps 预算不容 + 结构前提未备);
把降噪当"平台"提前铺(第 2/3 级在客户臂出现前是纯负债——多一个组合臂锚要养)。

---

## 8. 若开工,第一步做什么

**第 0 级 K 档实测(零代码,一小时级,GPU 窗口另批)**——把 D 相留下的后备旋钮
变成定档数据:

1. 跑 bench 质量腿 K 阶梯(CLI 全在,`d_tsr/ACCEPTANCE_SUMMARY.json:16`):
   `--smooth-normals on --ggx on --lamp-lights on --lamp-gain 4 --gi2 on
   --gi2-clamp 0.01 --tsr-quality on --tsrq-clamp {0 | 3 | 2 | 1.5}` × `--render` 128f;
2. 判据(全部在案模板):conv 协议四 ROI std_p95 vs K=0 基线(arm4 数据在
   `d_tsr/d_metrics.json` 可直接对照);**远处小灯保真 ROI**(dolly f0240 远灯,
   D 相已登记 Karis 偏暗面,K 过小会误杀合法孤立小灯——
   `g31_tsr_resolve_q.rx:28` 原注)必须亮度不降超阈;
3. K=0 位级恒等由 branchless 门保证(:265-268 `k_on` 门),全锚零漂移免重验代价低;
4. 产出 `tsrq_clamp_ladder.json`:若某档在"微光点再降"与"小灯保真"间存在正区间,
   入 full 预设复评;若不存在,登记"K 档关死,降噪投资转第 1 级"——
   两种结果都直接喂 §7 路线表的下一格。

---

## 附:本评估读过的素材清单

| 文件 | 用途 |
|---|---|
| `src/rurix-render/kernels/g31_tsr_resolve_q.rx` | tsrq 四质量面 + params[19]/[20]/[21..32) 余量 |
| `artifacts/day_0828/d_tsr/{ACCEPTANCE_SUMMARY.json,d_metrics.json}` | 四臂 A/B 在案(std_p95/高通/dolly/帧时) |
| `artifacts/day_0828/c_gi_r2/{ACCEPTANCE_SUMMARY.json,c_metrics.json,c_spatial_char.json,c_noise_metrics_tuned.py}` | GI2 噪声幅值/空间特征/rel_p95 门教训 |
| `artifacts/day_0828/e_final/{HANDOVER.md,E_ACCEPTANCE_SUMMARY.json}` | 地板尾 −18% / quad NEE 缺口 / K 档留窗 / 锚纪律 / full 帧时 |
| `src/rurix-render/kernels/g31_texture_nrm_gi.rx` :748-777 | 单缓冲合成输出(拆通道地形) |
| `src/rurix-render/src/bin/g31_window_present.rs` | 五 pass 结构 / bloom/AE 插入形态 / descs 下标纪律 |
| `G31_PLUS_COMMERCIAL_RENDERER_TODO.md` :101/:156 | #29 NRD 锚 / #12 GI 默认档波次 |
