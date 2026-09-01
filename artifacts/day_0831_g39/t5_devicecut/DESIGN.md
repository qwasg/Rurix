# DESIGN — frame_cut 臂 host cut(3–11.5ms)下沉 device(TODO #77;G39 T5 段 1;**设计文档,本窗零代码**)

> 日期:2026-08-31。任务行:`G31_PLUS_COMMERCIAL_RENDERER_TODO.md` L207「#77 实例/簇两级 GPU 剔除 kernel `[遗漏]` … P1」;留窗登记:`artifacts/day_0830_g38/CAMPAIGN_LOG.md` L54「frame_cut host cut 3-11.5ms(#77 device cut)」。
> 输入闭集:R5 交接单(`artifacts/day_0831_g39/recon/R5_T5.md`)/ G38 T3 报告与 evidence(`artifacts/day_0830_g38/t3_framecut/`)/ 臂源 `src/rurix-render/src/bin/g14_3_lane/g31_frame_cut_arm.rs` / kernel `src/rurix-asset/kernels/g31_cluster_cull.rx`(331 行全读)与 harness `src/rurix-asset/src/bin/g31_cluster_cull_device.rs` / WP 模板 `artifacts/day_0830_g38/t2_fifdyn/WIRING_PLAN.md` / 判档登记 `artifacts/day_0830_delivery/w3_deep/frame_cut_as/REPORT.md` L129-134。
> **行号快照声明**(WP 同律):本文行号 = 2026-08-31 晚本窗复核值;`g31_frame_cut_arm.rs`/`g31_frame_cut_probe.rs` 本役无并行编辑窗(台账核对:T1=窗口 bin,T2=lane_body/pipeline_perf,T3=render_exec*),但实施窗仍**以「替换前文本」字面锚为准**,行号仅助定位。数字全部引 evidence 实测值并标出处路径,不引口头值。

---

## 0. 目标形态总览

| 面 | 现状(host cut) | 目标(device cut) | 批次 |
|---|---|---|---|
| cut 决策 | `frame_cut_select_ext`(arm L458-518)逐块 `select_lod_cut_grouped` + `verify_cut_coverage`,host 串行 6.29–12.83ms/帧(t3_incr f1-15)| **P1(本判档段 2)**:host 决策不动,device 平行复算判定码 → 回读逐项对拍(等价证据臂);**P2**:device 决策码为源,host 回读施加;**P3**:device 直写竞技场零回读 | P1 = GO 圈定范围;P2/P3 留窗 |
| 判定 kernel | 无(host 直调生产金标准) | `g31_cluster_cull.rx` 三关超集 **0-byte 消费**(关 1/2/4 params/数据域中和,关 3 = `select_lod_cut_grouped` 字面同式,kernel 头注 L16-19)| P1 |
| 竞技场施加 | host 差集 → 增量上传 → `BlasRefitBridgeExt` 多 region copy → UPDATE build(arm L1034-1149) | P1/P2 逐字不动;P3 换 device scatter kernel(新加性 kernel,另窗 RFC 级) | P3 留窗 |
| 判据面 | 双跑 digest 位级 + 单调门 + 命中∈已施加 cut + 哨兵 canary + 零命中防伪(arm 头注 L21-26,双跑断言 L1306-1337) | **全部不动**;P1 加性追加:判定码逐项全等门 + decisions∈{2,4} 闭集断言 + red-arm | P1 |
| **不动面(0-byte 闭集)** | `visible_cluster_set.rs` / `g31_cluster_cull.rx`(冻结件,day_0828 C-1 登记)/ `g31_cluster_cull_device.rs` / `render_exec*.rs` / `g31_window_present.rs` / `g14_3_lane_body.rs` / `FrameCutArmOpt` 字段集(窗口 bin L8112 字面量构造)/ RXCP schema / 全部既有锚(all-off `55e4a92d…`、窗口臂 `5540ecae…`) | 同左,P1 全程维持 | — |

一句话机制:frame_cut 的 cut 是**相机纯函数**(帧 k 决策输入同帧已知),device kernel 关 3 与 host 金标准**字面同式**且已有 harness 判据①先例(v1.1.5 全绿)——先以 probe-only 对拍臂把「同设备判定码逐项全等」从合成夹具扩展到 bistro 生产资产域(P1),等价证据在手后再谈施加权移交(P2/P3)。

---

## 1. 决策语义分析(对照 WP §4 模板)

### 1.1 §4 先例原文(逐字引用,`artifacts/day_0830_g38/t2_fifdyn/WIRING_PLAN.md` §4)

> 机制根据:HZB 车道的逐帧 TLAS 掩码更新(`masks`/`uploaded_masks`,L5735-5738)由**上帧回读的可见性判定**驱动——host 在环反馈闭环。FIF 化会把决策延迟 S−1 帧(读到的是 k−S 帧判定),改变剔除语义本体,不是「每槽副本」能消解的写面竞争问题。

### 1.2 frame_cut 的决策结构(与 §4 场景逐项对比)

| 维度 | WP §4 场景(HZB 掩码,NO-GO) | frame_cut 臂(本案) |
|---|---|---|
| 决策输入 | 上帧 GPU 回读的可见性判定(host 在环反馈闭环) | (RXCP 静态表, 帧 k 相机, threshold_px)——**帧号纯函数**。probe 相机 = 装配相机 + k×step 闭式(probe L205-232);窗口臂 = 真轨迹相机,同帧 host 已知 |
| 回读依赖 | 有(决策依赖上帧产物) | **无**。`frame_cut_select_ext` 输入零 GPU 产物;下沉后 device 同帧算同一纯函数,零延迟语义 |
| 下沉/流水化后果 | 决策延迟 S−1 帧,剔除语义本体改变 | 决策语义逐位不变(同帧同输入同函数)——**与 §4 NO-GO 场景结构相反** |
| 语义闭环形态 | 必须 host 在环 | 生产语义路 cut→竞技场写→refit→RQ **可全 device 闭环零回读**(P3 终态) |

### 1.3 probe 判据面回读与生产语义分离

现帧循环(arm L1010-1220)的同帧 CPU 消费仅三类:①差集/上传构造(L1034-1099;P3 由 device 写竞技场后消失)②copy_regions 收集(L1087-1095;P3 同上)③命中缓冲回读 sha256 + 命中∈cut 判据(L1151-1189)——**③是 probe 判据面,不是生产语义路**(REPORT.md §5 登记:窗口合入 = 循环后证据臂,presented 面 0-byte)。P1 新增的决策码回读同属判据面。回读基建在位:`Readback`(render_exec.rs L577)/`FrameUpdate.readback_subset`(arm L1111 已消费);免回读 GPU-driven 先例在位:`DispatchSpec::Indirect`(render_exec.rs L338-348)+ g35 device 组装 args→间接消费链(`src/rurix-render/kernels/g35_indirect_args.rx`/`g35_compact_u32.rx`);`acceleration_structure_indirect_build` 未启用(render_exec.rs L4900/L8961 恒 0)——P3 的 build 面不依赖它(UPDATE build 输入=vbuf 字节,device 写完 barrier 后照录)。

### 1.4 关 4 的语义呼应(为什么中和不是阉割)

`g31_cluster_cull.rx` 关 4(L209-310)= 上帧金字塔两遍语义(Haar & Aaltonen 2015,harness 两 dispatch)——恰是 §4 引文描述的「上帧产物驱动决策」结构。frame_cut 生产语义**不含遮挡关**(`select_lod_cut_grouped` L316-354 只有 LOD 谓词,注释明示「不含视锥/背面锥关」)。故中和关 1/2/4 是**语义对齐**而非降格:frame_cut 的 cut 语义 = 纯关 3。若未来要把遮挡驱动引入 frame_cut(cut 随可见性收缩),决策输入将含上帧回读,落入 §4 的 host-在环/延迟语义域——**须另立 RFC,不在 #77 本行**(与 #6 HZB 接线行分界,留窗登记 §5-8)。

### 1.5 判定式等价的机制根据(字面对照)

host `select_lod_cut_grouped`(visible_cluster_set.rs L316-354):逐簇独立谓词 `self_px < thr && parent_px >= thr`,投影 = `projected_error_px(error·scale, max(dist−r·scale, 0))`(cull.rs L134-146:`dist > error` 则 `error·projection_factor()/dist` 否则 `+∞`;`projection_factor() = view_proj[1][1]·screen_height_px·0.5`,cull.rs L120-122;`error ≤ 0 → 0`,`error = +∞ → +∞`)。
kernel 关 3(g31_cluster_cull.rx L149-208):`dsurf = max(√(d·d) − r, 0)`;`dsurf > e` 则 `e·proj_factor/dsurf` 否则 `1e9`;`e ≤ 0 → 0`,`e ≥ 1e9 → 1e9`;谓词 `self_px < err_thr && parent_px >= err_thr`(L199-207)。
两侧差异闭集:①`+∞` vs `1e9` sentinel——判定只做与 thr(有限小值,如 2.0)的比较,两者均 `≥ thr` 且均不 `< thr`,**判定语义等价**(harness 上传映射先例:非有限 parent_error → `2.0e9`,harness L204-209);②host 有 `scale`(变换列范数)——frame_cut 恒 `IDENTITY_3X4`(arm L469/L482-487),scale = √1.0 = 1.0 精确,`error·1.0`/`radius·1.0` 位级恒等,`transform_point(identity, c)` 位级恒等(harness identity 同参先例,kernel 头注 L18-19「合成夹具 identity 变换(scale = 1)——实例变换面归生产接线波,金标准同参对拍」);③f32 运算序(sub/mul/div/sqrt)——**字面同式,但等价是实证门不是数学证明**(§3.2 风险)。harness 判据①(判定码逐项全等 vs host 复算,含 `select_lod_cut_grouped` 生产直调)v1.1.5(2026-08-27)全绿 = 本机贴齐的构造性先例;P1 就是把该实证从 uv_sphere 合成夹具扩展到 bistro 123,169 簇真值域。

---

## 2. edit 计划(字面锚级;**本窗不实施**,供段 2 实施窗直接消费)

改动闭集 = **两文件加性**:`g31_frame_cut_arm.rs`(probe/窗口 include 共享单源,但全部新面缺省 host = 窗口臂 0-byte)+ `g31_frame_cut_probe.rs`。kernel/rt/lane_body/窗口 bin/schema 0-byte。

### 2.1 E1 — probe 旗标 `--cut-source host|device` + `--cull-spv`(闭集 fail-closed)

- 用法文档(锚:probe L25-31 用法块,`[--refit-copy incr|full] [--min-level N]` 行后追加):`[--cut-source host|device] [--cull-spv <g31_cluster_cull.spv>] [--cut-red-arm tamper]`。
- 解析(锚:probe L108-113,`"--refit-copy" => refit_copy = take_arg(&args, &mut i),` 后追加三臂):`cut_source: String`(默认 `"host"`)/ `cull_spv: String`(默认空)/ `cut_red_arm: String`(默认空)。
- 闭集校验(锚:probe L139-144 `let copy_full = match refit_copy.as_str() {` 块后同式):`host|device` 闭集,其余 fail;**`device` 且 `--cull-spv` 缺失/文件不存在 = fail**(显式请求下的误配置,非 dev_env 三态——vulkan 缺失的 skip 三态在前置 L162-165 已覆盖,不动);`--cut-red-arm` 非空时须 `--cut-source device` 且值 ∈ {`tamper`} 闭集。
- ext 构造(锚:probe L247-250 `let ext = FrameCutArmExtOpt { copy_full, min_level, };`):补 `cut_source`/`cull_spv`/`red_arm` 字段;stderr 登记行(L251-254 同式)追加 `cut_source=…`。

### 2.2 E2 — `FrameCutArmExtOpt` 加性字段(锚:arm L79-102)

`FrameCutArmExtOpt` 为 G38 T3 新类型(**非冻结**;窗口 bin 不构造它,经 `run_frame_cut_arm` L1229-1246 转发 `default_ext()` 消费):加 `cut_source_device: bool` / `cull_spv: String` / `red_arm_tamper: bool`(`Clone, Copy` 因 String 须降为 `Clone`——复核两处 `ext` 传参均引用/克隆,`#[derive(Clone)]` 即可;或 `cull_spv` 存 `&'static`… 不引入生命周期复杂度,取 `Clone`)。`default_ext()` 补 `false / String::new() / false` ⇒ **窗口臂与既有 probe 调用 0 行为变**。

### 2.3 E3 — device 表构造 + params 装配 + 三关中和(arm 新增纯函数,置 L518 `frame_cut_select_ext` 尾后)

**中和方案裁决(两案对比)**:

| 案 | 机制 | 裁决 |
|---|---|---|
| **A(选定):params/数据域退化,kernel 0-byte** | 关 1:六平面全零 ⇒ `0 < −radius` 恒假(radius ≥ 0)恒不剔(kernel L86 判式字面);关 2:逐簇 `cone_cutoff = 1.0` ⇒ `if cutoff < 1.0` 恒假关断(kernel L126-127 字面);关 4:view 行(params[52..64))全零 ⇒ `viewz = 0`,`near_z = −radius ≤ 0 < znear(0.1)` ⇒ 恒走「近平面骑跨保守可见」分支,rect/金字塔逻辑**结构性短路不执行**(kernel L218-220 字面);hzb_data=[0.0] 1 texel + hzb_meta=[0,1,1] + levels=1 兜底绑定(短路下不读) | **选定**:冻结 kernel 0-byte(day_0828 C-1 登记件);判定码域收缩为 {2,4},与 host 期望码 `in_cut ? 4 : 2` 恰成对拍闭集;中和破坏可机核(§2.5 断言) |
| B:新加性 lod-only kernel 变体 | 新 `.rx` 文件只保关 3 | 否决:新 rurixc 编译面 + 新判据闭集须独立重证(harness 判据①锚在三关超集 SPV 上,换 kernel = 先例失效重锚);且违「cut 来源可插拔(判据不变)」登记精神(frame_cut_as REPORT L129-134) |

新增函数(全部 host 纯函数,selftest 直测):

1. `frame_cut_device_tables(blocks: &[ClusterPackBlock]) -> (Vec<f32>, Vec<f32>)`——canonical 全局序(块序×簇序,= `frame_cut_arena_layout_ext` L199-215 同一遍历序):cluster_f32 10/簇 `[center 3 | radius | 0,0,0(cone_axis 零填) | 1.0(cone_cutoff 中和) | error | parent_error*]`,lod_f32 8/簇 `[cluster_self_lod[ci].{center,radius} | cluster_parent_lod[ci].{center,radius}]`(`frame_cut_select_ext` L483-486 同源平行表直取)。`parent_error*` 映射 = harness L204-209 同律:非有限 → `2.0e9`;**加强域检 fail-closed:有限值须 < 1e9**(真值撞 sentinel 域 = 资产异常,拒)。center/radius 照填真值(关 1/4 中和下不参与判定,表意保真;error 面为关 3 真输入)。
2. `frame_cut_device_params(spec, in_w, in_h, threshold_px, n) -> [f32; 64]`——kernel 头注 L30-43 布局字面:`[0..24) = 0.0`(关 1 中和)/ `[24..27) = spec.eye` / `[27] = build_vp(spec,in_w,in_h).m[1][1] · in_h · 0.5`(= `cluster_cull_camera`(lane_body L2910-2918)→`projection_factor()`(cull.rs L120-122)字面同式;harness L455 同式)/ `[28] = threshold_px` / `[29] = n` / `[30] = 0`(mode=pass1)/ `[31..33) = 0` / `[33] = 0.1`(znear 正值,关 4 短路判据消费;harness ZNEAR 同字面)/ `[34] = 1` / `[35] = n`(cap = n ⇒ 零 overflow)/ `[36..64) = 0.0`(VP/view 零填,关 4 短路)。**LOD 判据分辨率 = in_w/in_h(内部分辨率,prelude 供)非光线画布 res**——`frame_cut_select_ext` 消费同一 `cluster_cull_camera(spec, s.in_w, s.in_h, threshold_px)` 口径(arm L473/L1016-1026),两侧同源。
3. `fc_spv_inject_no_contraction(spv: &[u32]) -> Vec<u32>`——harness `spv_inject_no_contraction`(g31_cluster_cull_device.rs L87-119)**字面同式副本**(OpFMul/OpFDiv/OpFSub〔op 129/131/133〕result-id 收集 → 首 annotation/type 段前逐 id 注 `OpDecorate NoContraction`)。已有两副本先例(cluster_cull_device + g31_cluster_stream 同律 bin 侧注入,harness L86 注释);第三副本如实登记 + 单源折叠留窗(§5-3)。
4. `frame_cut_device_cut_compare(spv, tables, params, expected: &[u32], tag, frame) -> f64`——`vk::run_compute`(rurix-rt pub 面,harness L478-479 同式;rurix-render 已依赖 rurix-rt)10 buffer 布局 = harness L466-477 字面:params/cluster_f32/lod_f32/input_ids(0..n 恒等)/hzb_data([0.0])/hzb_meta([0,1,1])/counters(12B 零)/decisions(n×4)/vis_ids(n×4)/occ_ids(n×4);dispatch `[n,1,1]`。回读 decisions:①**闭集断言** `d ∈ {2,4}`(出现 0/1/3 = 中和破坏,fail-closed 打印首破簇号)②**逐项全等门** `d[g] == expected[g]`(不等 = fail-closed,打印全局簇号/两侧判定码/该簇 error/parent_error/lod 球值——归因素材);返回 dispatch 墙钟 ms(measured)。red_arm_tamper 时:上传前 `lod_f32[3] += 1.0`(全局簇 0 self 球半径篡改)⇒ 对拍**必须红**(构造性证明对拍面真实消费,harness 判据⑥同律)。

### 2.4 E4 — 期望码来源与帧循环集成(对拍口径 = **提升前** select 原输出)

- **口径裁决**:kernel 关 3 对应的 host 面是 `select_lod_cut_grouped` 原输出(提升映射是 min-level 的表示层后处理,arm L494-505 在 select+verify 之后)。对拍必须锚在提升前,否则 ml>0 时语义面错位。
- `frame_cut_select_ext` 签名加性尾参(锚:arm L459-468 签名,L517 返回):返回四元组扩为五元组,尾加 `Vec<Vec<bool>>`(提升前逐块布尔集;`min_level == 0` 时与 sets 同值克隆)。调用点闭集 = 3 处机械补:`frame_cut_select`(L445-448,丢弃第 5 元)、`frame_cut_run_session` 两分支(L880-890 帧 0 先行 + L1013-1027 段 ①,变量接收)。
- 帧循环集成(锚:arm L1028-1032 `let cut_ms = …;` 之后、段 ② 之前插入):`ext.cut_source_device` 时——表/SPV 帧无关,在 `frame_cut_run_session` 会话建立段(L875-905)一次性构造(`frame_cut_device_tables` + SPV 读取 + NoContraction 注入 + spirv-val 由验收环覆盖);逐帧构造 params + 期望码(提升前集展平:`expected[global(bi,ci)] = if pre_sets[bi][ci] {4} else {2}`)+ 调 `frame_cut_device_cut_compare`。**双跑两遍 session 各自对拍**(L1306-1337 双跑结构自动覆盖)⇒ device 决策码跨跑一致性经「两跑均与同一 host 金标准全等」传递性成立,免独立断言(登记)。
- `blocks_limit` 逃生阀组合:表按消费中的 `blocks` 切片构造(L1270-1279 之后的同一引用)——天然一致,零特判。
- `--min-level N>0` 组合:对拍点在提升前 ⇒ ml 任意档均可对拍;提升映射/二次 verify/竞技场施加链 **0 改动**。

### 2.5 E5 — 判据、evidence、selftest(全加性)

- 判据追加(fail-closed 闭集):①判定码逐项全等(cluster_cull harness 判据①形)②decisions ∈ {2,4} ③red-arm 时对拍必红(进程 rc≠0)。既有五判据(文件头 L21-26)与单调门/双跑逐字不动。
- evidence(锚:arm L1401 起 `frame_cut_finish_ext`,schema `rurix.g31.frame_cut_probe.v1` 保持 + 加性,T3 §7-3 先例:w4_verify.py 无 schema 断言):顶层 `cut_source`;`cut_source=device` 时逐帧 `device_cut_probe_ms`(dispatch 墙钟 measured,**不并入 cut_ms/exec_ms 判读口径**)+ `device_cut_decisions_sha256`(决策码字节 sha256,跨跑/跨窗审计面)+ 顶层 `device_cut_table_bytes`(cluster+lod 表字节,123,169 簇 ≈ 4,926,760 + 3,941,408 B)。
- selftest 追加段(锚:arm selftest ⑥ 段尾,T3 同位):合成块上 ①`frame_cut_device_tables` 布局/域检/sentinel 映射锚 ②params 装配锚(零平面对任意球不剔/cutoff=1.0 关断/view 零 ⇒ near_z<znear 短路——host 复算三关中和式)③期望码构造锚 ④NoContraction 注入器结构锚(注入后 OpDecorate 计数 = FMul/FDiv/FSub 数)。纯 host,锁外可跑。

### 2.6 跨 crate 工件引用(kernel 在 rurix-asset,臂在 rurix-render)

- **源不迁、不复刻、无 Cargo 跨依赖**:kernel 源留 `src/rurix-asset/kernels/g31_cluster_cull.rx`(冻结 0-byte);消费形 = **rurixc 现编 SPV 文件工件 + `--cull-spv` 运行时显式传入**。先例字面:g35 渲染链 kernel 由 smoke 现编入 `.tmp/g35_gates/render` 后 bin 运行时装载(`g35_particle_lane.rs` L144 `G35L_SPV_DIR`;编译命令形 = `ci/g35_render_wiring_smoke.py` L396/L409:`cargo build -p rurixc --features vulkan-backend --bin rurixc` → `rurixc <src.rx> --target vulkan -o <dst.spv>` → `spirv-val`);harness 自身即 `--spv <path>` 运行时装载(harness L28-29 用法)。
- 本臂工件路径约定(验收环消费):`.tmp/g39_gates/t5_devicecut/g31_cluster_cull.spv`。NoContraction 注入在 bin 侧装载后进行(harness 同律,SPV 文件保持 rurixc 原产字节——注入不落盘,免第二工件口径)。

### 2.7 施加权移交两阶段路线(P2/P3;**不在段 2**,形态预登记)

| 阶段 | 机制 | 回读 | 前提/留窗 |
|---|---|---|---|
| **P2:device 决策 → host 施加** | 决策码回读(123,169×4 ≈ 493KB/帧)→ host 由 `d==4` 构造 cut 布尔集 → **既有**差集/上传/refit 链 0 改(cut 语义仍 host 写竞技场);`verify_cut_coverage` 直接跑在回读集上(host 影子核,fail-closed 语义逐字保持);min-level 提升映射照旧 host | 判据面回读转为生产路回读(过渡形态) | P1 等价门 16 帧全绿 + red-arm 红;dispatch 进 frame session 化(表驻留 SSBO,每帧仅 params 256B 上传——加 pass 面须动会话 resources/passes 数组,arm 内自有面) |
| **P3:device 直写竞技场** | 新加性 scatter kernel(cut 簇写包几何 9f32/tri、非 cut 写 [0;9] 折叠;槽互斥 ⇒ 写序无关确定性)→ vbuf barrier → UPDATE build 照录;零回读闭环 | 零(判据面按需保留) | 新 kernel = 新编译面/新判据闭集(RFC 级);`verify_cut_coverage` 等价 = device 机核(逐组计数/父子一致性 kernel)**或** host 影子核降频抽检(诚实降档);min-level 提升映射 device 化(DAG 上行 + 支配撤出,L267-328——图序算法,非平凡)或 P3 限 `--min-level 0` 首兑;与 #76(GPU compact+MDI 零回读,TODO L206)同族但独立行,不混 |

`verify_cut_coverage` fail-closed 等价方案裁决:P1/P2 取 **host 影子核**(生产校验器 L208-249 直调,输入换 device 产集——校验器语义零重写,T3 §4-3「生产校验器就是正确校验器」同一论证);device 机核归 P3 窗与 scatter kernel 同批设计。

### 2.8 验收环(GPU 批归主 agent 锁内;命令形完整可粘贴)

先决(构建 + kernel 现编):

```powershell
$env:CARGO_TARGET_DIR='H:\rurix\target-night'
cargo build -p rurix-render --features vendor-upscale --bin g31_frame_cut_probe --release
cargo build -p rurixc --features vulkan-backend --bin rurixc --release
$KDIR='H:\rurix\.tmp\g39_gates\t5_devicecut'; mkdir $KDIR -Force | Out-Null
& "H:\rurix\target-night\release\rurixc.exe" src/rurix-asset/kernels/g31_cluster_cull.rx --target vulkan -o $KDIR\g31_cluster_cull.spv
spirv-val $KDIR\g31_cluster_cull.spv
$FCP='H:\rurix\target-night\release\g31_frame_cut_probe.exe'
$EV='H:\rurix\artifacts\day_0831_g39\t5_devicecut\ev'; mkdir $EV -Force | Out-Null
```

| # | 命令(s09 基线形,仅新旗标/evidence 路径异) | 判据 |
|---|---|---|
| C0 | `& $FCP --selftest`(锁外先跑) | selftest OK 含新 ⑦ 段 + PASS |
| C1 | `& $FCP --cluster-pack .tmp/g36_gates/wave1_geo_composition/bistro.rxcp --error-px 2.0 --frames 16 --step-m 0.15 --res 96x54 --cut-source device --cull-spv $KDIR\g31_cluster_cull.spv --evidence $EV\t5_dev.json` | PASS(= 内建逐帧逐项全等 ×16 帧 ×双跑 + decisions∈{2,4} + 既有五判据);stderr 登记 `cut_source=device` |
| C2 | `& $FCP …同参… --evidence $EV\t5_host.json`(缺省 host 臂) + python 比对两件 digest 序列 | **t5_dev == t5_host 逐帧逐字节**(cut 语义 0 动的结构性必然,机核之);t5_host 对照 `artifacts/day_0830_g38/t3_framecut/ev/t3_incr.json` digest 序列 = 跨窗参考锚(同机同驱动预期同;若异,先查驱动/build 面,不误红本臂) |
| C3 | C1 同参 + `--cut-red-arm tamper` | **rc≠0** 且 stderr 含判定码 mismatch(对拍面消费的构造性证明) |
| C4 | C1 同参 + `--min-level 1` → `$EV\t5_dev_ml1.json` | PASS(提升前对拍口径 × ml1 组合;digest 自洽双跑,不与 ml0 比——T3 B5 口径同律) |
| C5 | `& $FCP …s09 原参无新旗标… --evidence $EV\t5_default.json` | 缺省面 0-byte 回归:digest 序列 == t5_host.json(旗标不传 = 字面同路径) |

工程量当量估计:加性 ~300±80 行(arm ~220 / probe ~40 / selftest ~60),**两文件**;对标 G38 T3(跨 render_exec 贯穿链 + 桥/计时/降档三件)约 1/5–1/10 当量;实施 0.5–1 窗 + GPU 批 6 命令(C1/C2 各 ~4min 量级,16f×双跑×每帧 ~9MB 表重传;C0 锁外)。

---

## 3. digest 等价门方案

### 3.1 免重锚推导链

**P1(对拍臂)**:cut 源未替换,施加链 0 字节改动 ⇒ digest 结构性不变(C2/C5 机核)。**P2(源替换)**:判定码逐项全等(P1 门,harness 判据①形)⇒ device cut 集 == host cut 集(判定码 4 ⇔ in_cut 双射)⇒ host 施加同一集合 ⇒ 竞技场写内容逐字节同(`frame_cut_write_cluster` = pack 顶点位级 copy〔L344-360〕+ [0;9] 折叠 + 槽互斥无序写)⇒ vbuf 终态位级同(T3 §3 incr/full 归纳论证同构)⇒ UPDATE build 输入序列位级同 ⇒ 同设备同驱动 digest 序列位级同 ⇒ **既有 RQ digest 门全部免重锚**:臂内建双跑(arm L1306-1337)/ 跨进程双跑(T3 B4)/ incr==full(T3 B3,16 帧已核:`t3_incr.json` == `t3_full.json` digest 逐帧同值,本窗复读确认)/ 窗口臂加性回归锚 `5540ecae…`(T3 B7)。
链上每一跳都是位级论证,**无「近似等价」环节**——这是把等价门锚在判定码(cluster_cull harness 判据①形)而非 cut 列表 diff 或图像 diff 的理由:判定码是最上游的最小闭集,上游全等则下游免证。

### 3.2 f32 决定性风险(诚实登记)与红臂设计

- **风险本体**:§1.5 的「字面同式」不构成位级数学证明——host(x86 libm/rustc)与 device(SPIR-V)的 sqrt/div 在 Vulkan 精度模型下不保证正确舍入逐位一致;FMA 收缩会改变 mul+sub 舍入序。**处置**:①NoContraction 注入(harness L87-119 先例,挡 FMA 收缩;本臂 E3-3 同律)②等价门定性为**同设备实证门**——与既有 digest 协议完全同律(arm 头注 L28-31/`determinism_note`:「跨设备不作 golden——RT 遍历 tie-break 依设备」),不冒充跨设备承诺 ③实证先例:harness 判据① v1.1.5 全绿(合成夹具,含平移副本 lod 球真值);P1 = 扩展到 bistro 123,169 簇 ×16 相机 ×(可选 ml1)真值域。
- **若 P1 红**(mismatch ≠ 0):E3-4 打印归因素材(簇号/两侧码/error/lod 球值);预期归因域 = `|self_px − thr|` 或 `|parent_px − thr|` 亚 ULP 边界簇。处置预案(诚实,不留模糊):**P2 判 NO-GO**,留窗「判据整数化/误差带重设计」(RFC 级,动金标准口径面),P1 evidence 如实登记 mismatch 率——对拍臂本身仍是有效交付(风险被量化)。
- **红臂闭集**:①`--cut-red-arm tamper`(lod 球篡改 ⇒ 必红,C3 机核消费路径)②decisions∈{2,4} 闭集断言(中和面破坏检出:出现 0 = 平面非零/出现 1 = cutoff 未关/出现 3 = 关 4 未短路)③selftest ② 段 host 复算三关中和式(锁外常驻)。
- **驱动差异**:同机同驱动为门界;驱动升级后 C2 的跨窗参考锚(t3_incr digest)若漂,与本臂无关(RQ 遍历面),判定码门独立复跑即可再证——判定码不含 RT 遍历,预期比 digest 锚更稳。

---

## 4. 预算叙事(诚实)

90fps 预算 = 11.11ms/帧。实测分解(全部 evidence 出处;refit 帧 = 每帧,cut_every=1):

| 口径 | host cut_ms | exec_ms(≈fence) | 其中 UPDATE build GPU | 桥 copy GPU | 出处 |
|---|---|---|---|---|---|
| ml0 incr,f1-15 | **6.291–12.827** | 21.147–22.735 | 19.837–21.329 | 0.012–0.031(77KB–479KB) | `artifacts/day_0830_g38/t3_framecut/ev/t3_incr.json` |
| ml0 incr,帧 0 | 3.218 | 32.534 | 19.458 | 4.666(75MB 全量) | 同上 |
| ml0 full 对照,f1-15 | 6.006–9.442 | 26.100–29.122 | 20.134–22.952 | 4.405–5.274(恒 75MB) | `…/t3_full.json` |
| ml0 窗口真轨迹,26f | **5.992–10.100**(均 9.095) | 均 24.196 | 21.353–23.325(均 22.314) | — | `…/t3_window_fc.json` |
| ml1 incr,f1-15 | **14.015–15.246** | 9.182–12.403 | 8.087–11.252 | 0.006–0.016 | `…/t3_ml1.json` |

(留窗登记字面「3-11.5ms」出自 G38 CAMPAIGN_LOG L54,对应 t3_incr 帧 0 的 3.218 与逐帧上界 ~11.3/12.8 的口径;窗口真轨迹 5.99–10.10 同域。)

1. **ml0 下沉不解预算**:墙钟/帧 ≈ cut + delta + exec ≈ 27.8–35.1ms(t3_incr f1-15 三段和:min 帧 6 = 6.291+0.224+21.246,max 帧 14 = 12.827+0.552+21.715)。cut 全额下沉(P3 理想化,host 段 → 0)余 ≈ 21.5–23.2ms——**UPDATE build ~19.8–21.3ms GPU 地板主导,仍 ~2× 预算**。下沉在 ml0 是墙钟 −20%~−37% 的真实收益,但不是进预算杠杆。
2. **×`--min-level` 组合才有进预算叙事**:ml1 下 build 地板降至 8.1–11.3ms、exec 9.2–12.4ms,而 host cut 段反升至 **14.0–15.2ms(帧内最大单段)**——R5 口径:较 ml0 同帧 +~5ms 来自提升映射 + 二次 verify。此形态下沉收益测算:P2(select 下沉,提升/verify 留 host)省 ≈ ml0-cut_ms 当量 6–10ms,host 残段 ≈ 5–6ms + 决策码回读税(~493KB,预期亚 ms,measured 待 P1 登记)⇒ 墙钟 ≈ 23.7–27.3(t3_ml1 f1-15 三段和)→ **~15–19ms**;P3(全闭环 + 提升 device 化或 ml0 限定)⇒ **~10–13ms 贴预算线**。诚实结论:**device cut 是 ml1 交付形态进预算的必要件之一,非充分件**;select/verify/提升三段无分项计时 evidence,P1 应加分项登记(cut_ms 拆 `select_ms`/`verify_ms`/`promote_ms` 加性字段)供 P2 精算。
3. **fence_ms 同步会话形态 = 收益不折价的前提**:t3_incr f1-15 fence_ms 20.886–22.402 ≈ exec_ms(同帧差 < 0.4ms)——host cut 与 GPU 段严格串行,**省 1ms host 段 = 得 1ms 墙钟**。反面:该收益的替代路「FIF 流水遮蔽 host cut」不可用——FIF 拒 `blas_refit`(render_exec 既有纪律,WP §0 表/R5 同引),#90 L2a 亦不覆盖 refit×FIF;submit/collect 拆分的软件流水是另案(rt 面新入口),不在 #77 域,不预支。
4. **P1 自身零性能主张**:对拍臂加 dispatch + 表传 + 回读(~9MB/帧证据税),`device_cut_probe_ms` 单列 measured 不进 cut_ms/exec_ms 判读——P1 交付物是**等价证据**,不是帧时;P2 的生产 dispatch 成本(表驻留后仅 256B params/帧 + 123k invocation)由 P1 的 dispatch 计时给出上界参考(kernel `numthreads(1,1,1)` 单线程/invocation,占用率低但 harness 已真跑先例;bistro 规模 measured 待 C1)。
5. **与 #76 分界**:本行「零回读」指 cut→竞技场→refit 决策闭环;draw 提交面的 GPU compact + MDI 零回读是 TODO L206 #76 独立 P1 行,不混不预支。

---

## 5. 风险与留窗表(WP §5 体例)

| # | 项 | 处置 |
|---|---|---|
| 1 | f32 等价是实证门非证明(sqrt/div 精度域/驱动差异) | NoContraction 注入(harness 先例)+ 同设备门界(digest 协议同律)+ mismatch 归因打印;P1 红则 P2 判 NO-GO 留窗判据整数化(§3.2 预案),P1 evidence 仍有效交付 |
| 2 | sentinel 域碰撞(有限 parent_error ≥ 1e9) | 上传域检 fail-closed(E3-1);bistro 误差米级,预期零命中,防御性拒 |
| 3 | NoContraction 注入器第三副本(双源纪律) | 字面同式副本 + 本表登记;留窗:与 cluster_cull_device/cluster_stream 三处并折至共享 helper(rt/asset lib 面,归后续治理窗,不阻本兑) |
| 4 | 中和面破坏(params/表装配 bug 静默放行) | 三重:decisions∈{2,4} 闭集断言(fail-closed)+ selftest 中和式 host 复算 + red-arm C3 |
| 5 | `select_lod_cut_grouped` 未来演进致谓词性漂移(集合级后处理引入) | 对拍门本身即哨兵(漂移 ⇒ C1 红);`verify_cut_coverage` 生产链 0 改动继续兜底 |
| 6 | P1 每帧 ~9MB 表重传 + `run_compute` 会话开销 | 证据臂性质,单列 measured 不进帧时判读;P2 表驻留 SSBO 设计在案(§2.7) |
| 7 | arm 为 probe/窗口 include 共享单源,实施窗编辑权 | 全部新面缺省 host = 窗口 bin 0 行为变(E2);实施窗须主 agent 台账登记 arm+probe 编辑权,与 T1(窗口 bin 本体)文件不交叠但 `cargo check` 面交叠(include 重编),收尾双绿即可 |
| 8 | 遮挡驱动 cut(关 4 解除中和)误入本行 | 结构性 NO(§1.4):决策输入将含上帧回读,落 WP §4 host-在环域;须另立 RFC,与 #6/#90 分界,本设计不预支 |
| 9 | min-level 提升映射 device 化(P3) | 图序算法(L267-328)非平凡;P3 首兑限 `--min-level 0` 或提升留 host(混合形态),归 P3 RFC 窗裁决 |
| 10 | `verify_cut_coverage` device 机核 | P1/P2 host 影子核(生产校验器直调,零重写);device 机核归 P3 窗(§2.7) |

---

## 6. GO/NO-GO 判档建议

**判档:GO(段 2 最小面实施)——范围精确圈定如下,超出即越界:**

- **形态**:probe-only `--cut-source device` 决策码回读对拍臂(P1)——先证等价再谈施加。cut 决策权、竞技场施加链、全部既有判据与锚 **0 字节移交**。
- **文件闭集**:`g31_frame_cut_arm.rs` + `g31_frame_cut_probe.rs` 两文件加性(E1–E5);kernel/rt/lane_body/窗口 bin/schema/ci 门 0-byte;kernel 消费 = rurixc 现编 `.tmp/g39_gates/t5_devicecut/g31_cluster_cull.spv` + `--cull-spv` 运行时装载(E3/§2.6),源冻结面不触。
- **判据闭集**:判定码逐项全等(×16 帧 ×双跑)+ decisions∈{2,4} + red-arm 必红 + 缺省面 digest 0 漂(C0–C5,§2.8);全绿 = #77 的「cut 来源可插拔(判据不变)」登记(frame_cut_as REPORT L129-134)从登记升级为机核证据。
- **工程量当量**:~300±80 行加性 / 0.5–1 实施窗 / GPU 批 6 命令(C0 锁外);T3 当量的 1/5–1/10。
- **GO 的三条机制根据**:①决策语义与 WP §4 NO-GO 场景结构相反(相机纯函数,零回读反馈环,§1)②kernel 关 3 与生产金标准字面同式 + harness 判据①先例全绿,中和方案纯 params/数据域(§1.5/§2.3)③等价门锚在判定码最上游闭集,P1 对 digest 面结构性零风险(§3.1)。
- **P2/P3 不在本 GO 内**(施加权移交/直写竞技场):开窗条件 = P1 C1–C5 全绿 + `device_cut_probe_ms`/分项计时 measured 在档;owner 留窗登记于 §2.7/§5-9/§5-10。若 P1 出 mismatch,P2 自动 NO-GO(§3.2 预案),P1 evidence 以「风险量化」形态收账,不冒充。
- **预算叙事纪律**(随 GO 登记,防误读):本 GO 不承诺帧时收益——ml0 下 build ~19.8–21.3ms 地板主导,下沉不解预算;进预算路径 = device cut(P2/P3)× `--min-level`(t3_ml1 实测 build 8.1–11.3ms)组合叙事,数字见 §4。

---

*(本文档为 G39 T5 段 1 交付;零代码/零 schema/零 GPU/零 commit。评估中读过的全部源锚与 evidence 路径已在文中逐处标注。)*
