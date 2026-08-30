# G37 W2 双子任务报告:post_chain 五级链差距分析与 LUT 臂(#79)+ PSO precache/warmup 守护(#82/#113)

- 日期:2026-08-29(night_0830 备料);产出 agent 纪律面:`g31_window_present.rs` **只读零触碰**(合入归主 agent),禁 GPU/禁 release/禁 target-night 全程遵守。
- 新工件:`kernels/g31_display_encode_lut.rx`(fork,母版 0-byte)、`.tmp/night_0830/spv/g31_display_encode_lut.spv`、`src/bin/g37_w2/g31_lut_assets.rs`、`src/bin/g37_w2/g31_pso_warmup.rs`、`src/bin/g37_w2_selfcheck.rs`(CPU 自检 harness,已跑通)。
- **注意:窗口 bin 正被主线并行演进(本报告侦察期间行号已漂一次,G37 W2 transparency 臂在途)。合入提案一律以「内容锚」为准,行号为侦察时快照仅供定位。**

---

## 1. 子任务 1:post_chain 五级链 device 接线差距(#79)

### 1.1 host 骨架事实源(`src/rurix-render/src/display/post_chain.rs`,M119/RXS-0370,只读)

五级显式排序冻结(`Stage::ORDER`,顺序闭集,交换/跳级即 RED):

1. **exposure**:histogram + EV 偏移;`ExposureState` 双缓冲帧间持久(丢帧即 RED),adapt **上/下双速率**(1.0/0.5)。host 运算 = `px × 2^ev`。
2. **bloom**:tonemap 前 **HDR 域多尺度 mip 链**(down/up 双 pass);host 骨架用 3×3 box blur 近似;HDR 探针防隐式 SDR clamp(RED 臂)。
3. **tonemap**:经 M118 `ViewTransform` 插件面(内置 neutral/aces13 等四插件,禁静默插级/跳级)。
4. **color grading**:**LUT 资产,tonemap 后**(显示域)。host 金标准形态 = **逐通道 1D 仿射** `apply_color_grading(px, slope, offset) = (px·slope + offset).max(0)`;模块注释明示「**完整 3D LUT 在 device 面**,host 骨架用 1D 逐通道映射维持 golden」——即 device 侧的金标准形态就是 3D LUT,host 1D 仿射是其 golden 参考子集。
5. **output transform**:`encode_display_linear`(RRT/ODT 或中性;`DisplayParams`{peak nits, OutputEncoding})。

### 1.2 窗口 bin 现状(只读侦察)

- **曝光双分量**:静态 EV(契约 ev100 + 交互 ±0.25 + `--ev100-ramp`)走 tsr_params 面,施加于 TSR 输出**之前**(bloom 上游);A2 自适应分量 = `g31_autoexp_reduce`(256 线程单 workgroup log-luma 归约)→ `g31_autoexp_state`(几何均值 → target=key/avg 钳 [min,max] → EMA → device 写增益进 enc_params[133]),施加点 = encode kernel ACES IDT 前 = **bloom 之后**。状态 16B device buffer 跨帧持久,era 重建归零再适应。
- **bloom(D3)**:resolve 后 bright(软膝阈值+2×降采样)→ blurH → blurV(半分辨率 9-tap 可分离高斯)→ composite(双线性上采样 × strength 加性合成);encode in_color 改读 comp_out。**单级半分辨率**,非多尺度 mip 链。
- **tonemap**:`g31_display_encode`(v2,A2b ACES 样条转置修复)——ACES 1.3 RRT+ODT 固定,params[0..136) host `aces13_device_encode_params_ex` 单源上传([133]=AE 增益,**[134..=135] reserved 恒 0**)。
- **色彩分级**:**零**。窗口 bin 内全部 "LUT" 命中(`G31_U_TEX_LINLUT` 28/36 号)均为纹理 srgb→linear 解码表,与分级无关。
- **输出**:BT.1886 γ2.4 逆 EOTF + D1 TPDF dither(params[3] 门)+ 8-bit 量化 + BGRA8 打包 + `ExternalImagePresent`(SDR 恒定,无 HDR 元数据 = #17 正交)。

### 1.3 五级差距表(交付①)

| 级 | 骨架定义 | 窗口现状 | 判定 | 形态差(如实登记) |
|---|---|---|---|---|
| 1 exposure | histogram+EV,双速率 adapt,状态帧间持久 | 静态 EV(tsr 面,bloom 前)+ A2 log-luma EMA 增益(enc_params[133],bloom 后) | **已接** | ①测光 = log-luma 几何均值(非 histogram)②单一 EMA 速率(非上/下双速率)③**自适应分量施加点在 bloom 后**——相对骨架序 exposure↔bloom 半交换(静态分量仍在 bloom 前);④状态持久 ✓(era 重建归零属 resize 语义,已在 A2 口径登记) |
| 2 bloom | HDR 域**多尺度 mip 链** | bright→blurH→blurV→composite(单级半分辨率高斯) | **已接** | 单级降采样非 mip 链(强于 host box 近似,弱于多尺度;大半径辉光截断)——留窗项 |
| 3 tonemap | M118 ViewTransform 插件面(四内置可换) | ACES 1.3 RRT+ODT 固定(v2 SPV,单源参数上传) | **已接** | 无插件选择臂(neutral/AgX 不可切);host 单源纪律 ✓ |
| 4 color grading (LUT) | LUT 资产,tonemap 后;device 金标准 = 3D LUT | **无** | **缺(唯一缺级)** | → 本役 `--lut` 臂设计(§1.4);host golden(slope/offset 1D 仿射)是 3D 表的线性子集 |
| 5 output transform | encode_display_linear(RRT/ODT 或中性→编码) | BT.1886 γ2.4 + dither + 8bit 量化 + BGRA8 + present | **已接** | 恒 SDR BT.1886;无 HDR(scRGB/PQ)腿 = #17 正交不混 |

**结论:#79 不能登记已闭**——五级缺第 4 级;其余四级已接但三处形态差需在 TODO 行如实附注(测光形态/单级 bloom/无插件面)。缺级修复 = 本役 `--lut` 加性臂(工件已备,合入归主 agent)。

### 1.4 `--lut` 臂设计(交付②)

**CLI 面**:`--lut off|neutral|warm|<path.cube>`(默认 off = 全 0-byte)。

- `neutral` = 内嵌 17³ 恒等格点(A/B 科学对照臂:隔离采样机械与分级内容);
- `warm` = 内嵌 17³ 分级 preset(白平衡暖移 R×1.06/B×0.94 → Rec.709 luma 保持饱和度 ×1.12 → γ0.96 轻抬,f64 闭式求值收窄 f32 一次,字节跨构建稳定);
- `<path>` = .cube 3D LUT 文件(Adobe/Resolve 惯例子集:`LUT_3D_SIZE` ∈ [2,64],DOMAIN 钉死 0..1,R 最快序,fail-closed 拒 1D/非单位域/行数不符)。

**传输面(反红修 #2 地雷的核心决策)**:LUT 表**内嵌 encode 参数 buffer(22 号资源)尾部**,不走独立 SSBO——

- `[134]` = lut_gate(host 打包 1.0;kernel ≤0.5 直通守卫)
- `[135]` = lut_dim N
- `[136..136+3N³)` = 表体(R 最快序 `idx = r + g·N + b·N²`,每格点 out RGB 3 f32;17³ ≈ 57.6 KB)

⇒ **零新资源/零新绑定/零新屏障/零下标族/零 prepare_update override 改动**。AE 变体族 match(现已 10+ 臂,红修 #2 事故面)完全不扩;与 bloom/AE/dither/textures/realism 全臂的组合**形态无关**(encode pass 绑定面 [in_color, ENC_PARAMS, ENC_OUT] 逐字不变,FG 变体的 encode_fg 共用同 buffer 自动一致)。A2 的 [133] 增益槽 device 写与新增槽零冲突(state kernel 只写 [133])。

**kernel 面**:`kernels/g31_display_encode_lut.rx` = 母版(ACES 已修版)逐字 fork,唯一新增 = LUT 段,插入点 = ODT Rec.709 色域钳 `fr/fg/fb ∈ [0,1]` 之后、BT.1886 逆 EOTF 之前(= 骨架第 3/4 级之间,显示线性域,与 host 金标准同域);trilinear 8 角直线展开(零循环),输出钳 [0,1];量化行改用 fr2/fg2/fb2。逐字 diff 已核(代码级差异仅:kernel 名 + LUT 段 + 三行量化变量名)。

**换载律(字节隔离,day_0828 C 相纪律)**:off 臂恒载 `.tmp/night_0828/spv/g31_display_encode_v2.spv` 锚定字节(**不载新 SPV**,55e4a92d/5db2e7d7 等锚零风险);on 臂独载新 SPV。CLI 面「默认字面才换」(tsrq 先例);`--lut on` 与显式 `--spv-encode` 同给 = fail-closed(显式 SPV 无 LUT 段即静默失效冒充)。

**互斥集**:与 `--auto-exposure` 同律(fg/hzb/svt/slab/cluster-lod/wp-hlod fail-closed);与 dither/smooth-normals/ggx/lamp/textures/bloom/AE/gi2/realism 全臂可组合(传输面形态无关)。`--quality full` **不展开 --lut**(预设语义变更即重锚 5db2e7d7,归主线决策)。

**新 SPV/资产锚**:

| 工件 | 锚 |
|---|---|
| `.tmp/night_0830/spv/g31_display_encode_lut.spv` | sha256 `9087b743a6fc426e065f2d673b38a04abe0abc2b65da09a3cd89ccb43f335a4b`(112,480 B;rurixc --target vulkan 内置 spirv-val accepted + 独立 spirv-val rc=0) |
| neutral 17³ 表体(f32 LE) | sha256 `aeafdd92a6160e27c03e803c91e96cd59bdd71c98beb819fb12bf7c4ccc73074`(双生成 == 断言过) |
| warm 17³ 表体(f32 LE) | sha256 `7cdf6afb3671a7ea8021db582a6ba3642be772f3f8d689ffd10e7c5ab9b0cc73`(双生成 == 断言过) |

**host 模块**(`src/bin/g37_w2/g31_lut_assets.rs`,mod 包裹可 `include!`,day_0829「host 面落窗口 bin 自有文件」律的模块化形):`neutral`/`preset_warm`/`parse_cube`/`from_arg`(CLI 闭集)/`extend_encode_params`(断言既有参数面恰 136 f32——kernel 基址字面双侧同步卫兵)/`sample_trilinear_f32`(与 kernel 逐操作同序的 host 参考,device/host 对拍面)/`table_sha256`。

**selfcheck 已验**(`cargo run -p rurix-render --bin g37_w2_selfcheck`,退 0):中性表 trilinear 恒等界 ≤5e-7(≪ 8-bit quantum 3.9e-3)、格点采样位级取角、.cube round-trip 表体位级相等、四 fail-closed 拒臂、参数尾挂布局断言。

**如实登记**:on(neutral) 与 off **不承诺位级相等**——新 SPV 字节即驱动重编扰动面(GI2 教训:字节隔离强于数学恒等),且 trilinear f32 舍入 ~1-2 ULP;验收口径见 §5。

### 1.5 LUT 合入提案(交付⑤ 前半;锚点 = 内容锚,行号为 2026-08-29 晚快照)

| # | 内容锚(行号快照) | 插入/修改 |
|---|---|---|
| L1 | `include!("g14_3_lane/g14_3_lane_body.rs");`(L218)之后 | 追加 `include!("g37_w2/g31_lut_assets.rs");` |
| L2 | `const G31_DEFAULT_SPV_ENCODE: …`(L248)之后 | 追加 `const G31_DEFAULT_SPV_ENCODE_LUT: &str = ".tmp/night_0830/spv/g31_display_encode_lut.spv";`(+ 换载律 doc 注释,realism 常量注释同款) |
| L3 | CLI 变量区 `let mut spv_encode = …`(L6443)邻域 | 追加 `let mut lut = "off".to_owned();` |
| L4 | `"--dither" =>` 解析臂(L6730)邻域 | 追加 `"--lut" => lut = take_arg(&args, &mut i),`(值域校验延后到 from_arg,路径臂自由字面) |
| L5 | tsrq 换载块 `if tsr_quality && spv_resolve == DEFAULT_SPV_RESOLVE {…}`(L7799)之后 | 追加校验+换载+资产构建块(见下代码) |
| L6 | era 循环内 `let enc_params = aces13_device_encode_params_ex(ew, eh, bgra, dither);`(L8557) | 改 `let mut enc_params = …;` 并紧随其后插 `if let Some(l) = lut_asset.as_ref() { g31_lut_assets::extend_encode_params(&mut enc_params, l); }` |
| L7 | evidence:`jstr(&spv_encode…)` + sha(L10182-10183) | **0-byte**——spv_encode 已换载,path+sha 自动如实流入;lut 臂字面建议进 PASS 行/notes(主 agent 裁量,主 evidence schema additionalProperties:false 禁新顶层字段) |

L5 块参考实现(变量名与 A2 互斥块 L7207-7214 逐字对齐):

```rust
// G37 W2:--lut 色彩分级臂(M119 五级链第 4 级;TODO #79 缺级收口)。
// 传输面 = enc_params 尾挂([134] 门/[135] dim/[136..) 表体),零新资源/
// 绑定/屏障/下标族;换载「默认字面才换」+ 字节隔离(off 恒载 v2 锚定字节)。
let lut_asset = match g31_lut_assets::from_arg(&lut) {
    Ok(a) => a,
    Err(e) => fail(&format!("--lut: {e}")),
};
if lut_asset.is_some() {
    if fg != G31Fg::Off || hzb == G31Hzb::On || svt_on || slab_table.is_some() {
        fail("--lut 非 off 不与 --fg/--hzb on/--svt on/--slab-table 同跑（组合面未接线,fail-closed）");
    }
    if cluster_lod_mode != "off" || wp_hlod_mode != "off" {
        fail("--lut 非 off 不与 --cluster-lod/--wp-hlod 同跑（组合面未接线,fail-closed）");
    }
    if spv_encode == G31_DEFAULT_SPV_ENCODE {
        spv_encode = G31_DEFAULT_SPV_ENCODE_LUT.to_owned();
    } else {
        fail("--lut 非 off 与显式 --spv-encode 同给（LUT 段归默认链工件;显式 SPV 无 LUT 段即静默失效冒充,fail-closed）");
    }
}
```

(`lut_asset` 为 era 不变量,建于 era 循环外;buffer 尺寸随 `enc_params_bytes.len()` 自动变长,resize 随车道重建自然重挂。)

---

## 2. 子任务 2:PSO precache + pipeline warmup(#82/#113)

### 2.1 `material/pso_cache.rs` API 事实(冻结,只读)

- `PsoDesc{vs_entry, fs_entry, color_formats, depth_format, blend, cull}` + `stable_hash()`(FNV-1a 64,字符串 NUL 分隔防歧义,枚举显式 tag);
- `PsoCache<P>`:`precache(descs, compile_fn)`(加载期,幂等,**不计告警**)/`get_or_compile(desc, compile_fn)`(运行期,未命中现场编译且 `runtime_compile_warnings +=1`)/`warnings()`(**验收归零**)/`contains`/`len`;
- `predict_precache_list(closures, passes)`:材质 flags(ALPHA_BLEND/DOUBLE_SIDED)× pass 模板笛卡尔积 → blend/cull 变体。

### 2.2 窗口 pipeline 创建时机侦察(交付③)

**结论:窗口 pipeline 全部启动期(session 构造期)创建;运行期唯一新建点 = era 重建(全量同变体集重建,非惰性首遭遇)。**

证据链:

1. `G31TsrLane::create` / `G31HzbLane::create`(窗口 bin,快照 L9261/L9227)把**全部 pass 一次性**交给 `DeviceFrameSession::new_with_accel_structs`;
2. `rurix-rt/src/render_exec.rs` `create_persistent_frame`(L8330 起;compute 分支 L7913-7975)在**构造期逐 pass 循环**调 `vkCreateComputePipelines`/`vkCreateGraphicsPipelines`,会话内 `ComputePipelineKey{spv_hash(fnv64), entry}` / `gfx_pipe_cache` 去重;
3. 运行期逐帧面 `FrameUpdate`(render_exec.rs L466-482)字段闭集 = tlas_update/buffer_uploads/binding_overrides/push_constant_overrides/readback_subset/blas_refit——**无管线创建能力**(布局键漂移即确定性 Err);
4. SPV 字节全部在 era 头 `load_spv`(快照 L8553 起),era 循环 `'eras: loop`(L8532);
5. **运行期新建点清单**:仅 era 重建三入口——WM_SIZE resize / `--window-storm`/`--storm-soak` 程序化 resize / 最小化恢复。每次 = 车道 drop + 新 session 构造(同 flag 集合 ⇒ 同 SPV 集合 ⇒ 同变体集);era 内逐帧零管线事件。present 腿 `ExternalImagePresent` 为拷贝面无管线;两车道(Tsr/Hzb)CLI 互斥不并存。
6. `bench --warmup` 语义核对:测量窗口(post-warmup 统计起点),**非**管线预热——但管线本就在构造期建齐,warmup 帧实际预热的是驱动/cache/时钟面。#113 原文口径如实。

### 2.3 #82/#113 差距表(交付③ 附)

| 项 | 现状 | 判定 |
|---|---|---|
| #82 预测/precache/告警 API | host 完备(冻结) | **窗口 demo 零消费** → 本役接「变体账本」消费面 |
| #82「现场编译面」 | 窗口不存在惰性现场编译(构造期全建) | 守护化:era≥1 `get_or_compile`,miss = 运行期新变体遭遇,验收 `pso_runtime_creates == 0` |
| #82 材质×pass 笛卡尔积(`predict_precache_list`) | compute mega kernel 单管线全材质,材质不引入 shader 排列(pso_cache.rs 自述语义) | **无消费面,如实登记不冒充**(变体数与材质数解耦;raster 化/#74 VisBuffer 波再接) |
| #113 启动 warmup | session 构造期全建 = **天然满足** | 实质 = 事实变受门保护的断言(账本 + evidence + strict 臂)——按任务预期口径「已天然满足,本役加守护」 |
| #113 进关预热 | 单场景 bin 无「进关」事件 | N/A 登记(场景切换臂出现时账本 `begin_session` 即预热登记点) |
| 跨 era Vulkan 管线重建税 | session 级 pipe cache 不跨 era(era 重建真重付 vkCreate) | **留窗**:`VkPipelineCache` 跨 era 复用/磁盘序列化 = rurix-rt 面改动,归主线(#86 双预算行联动) |

### 2.4 warmup/守护设计(交付④)

**默认开的变体账本**(零新必选 flag):

- era 0(启动)= **precache 面**:车道 descs 定型后逐 pass `ledger.register(name, spv)` → 冻结 `PsoCache::precache`(幂等,不告警),预测集 = 本次 flag 集合静态决定的 SPV 集(≤ 每 era ~5-11 pass,去重后更少;同 SPV 多 pass〔encode/encode_fg〕判同变体,与 rurix-rt 会话级去重同判);
- era ≥1(resize/风暴重建)= **守护面**:同一 ledger `get_or_compile`,命中零开销;未命中 = 运行期新 PSO 变体遭遇 → `pso_runtime_creates +=1` + stderr 单行告警 + 报告登记行;
- **验收 = `pso_runtime_creates == 0`**(风暴臂 `--window-storm 3` 下同变体集重建应全命中);
- strict 臂:`RURIX_G31_PSO_STRICT=1` → miss 即 `fail`(fail-closed;默认告警不断跑,镜像 pso_cache「告警,验收归零」语义);
- **evidence = sidecar**:`--pso-report <path>`(默认 off = 0-byte)落 `rurix.g31.pso_warmup_report.v1` 单行 JSON{sessions, unique_variants, pso_precache_count, **pso_runtime_creates**, planned[{pass, spv_fnv64, spv_sha256, spv_bytes}], runtime_create_rows[{pass, spv_fnv64, session_index}]}。主 evidence schema `additionalProperties:false` 冻结(milestones/ 禁改),新字段一律 sidecar——day_0829 战役证据外置同律。

**变体键映射约定**(compute mega 车道 → 冻结 `PsoDesc`):`vs_entry = "spv:<fnv1a64>"`(SPV 字节内容哈希,与 rurix-rt `ComputePipelineKey.spv_hash` 同算法)/`fs_entry = "compute"` 常量/color_formats=[]/depth=None/blend=Opaque/cull=None(compute 管线状态自由度为空,取闭集哨值);pass 诊断名只落报告行不进键。

**胶水模块**(`src/bin/g37_w2/g31_pso_warmup.rs`,mod 包裹可 include;仅依赖冻结 `material::pso_cache` + `rurix_pkg::sha256`):`G31PsoLedger::{new, begin_session, register(name, spv) -> was_miss, runtime_creates, unique_variants, sessions, report_json}`。selfcheck 已验:era0 precache 幂等零告警、同 SPV 双 pass 判同变体、era1 同集全命中、新变体 miss 告警 +1 且登记行、报告字段自洽。

### 2.5 PSO 合入提案(交付⑤ 后半)

| # | 内容锚(行号快照) | 插入内容 |
|---|---|---|
| P1 | 同 L1(`include!` 行后) | 追加 `include!("g37_w2/g31_pso_warmup.rs");` |
| P2 | CLI 变量区(L3 邻域)+ 解析臂(L4 邻域) | `let mut pso_report: Option<String> = None;` + `"--pso-report" => pso_report = Some(take_arg(&args, &mut i)),` |
| P3 | `'eras: loop {`(L8532)之前 | `let mut pso_ledger = g31_pso_warmup::G31PsoLedger::new();` + `let pso_strict = std::env::var("RURIX_G31_PSO_STRICT").is_ok_and(|v| v == "1");` |
| P4a | `match G31TsrLane::create(descs, …)`(L9261)**之前**(descs 已定型处) | 见下 P4 代码块(descs.passes 版) |
| P4b | `match G31HzbLane::create(…)`(L9227)之前 | 同款,遍历 hzb 的 `hz_pass` 切片 |
| P5 | 'eras 循环结束后、evidence 组装区(L10182 邻域之前任一收尾点) | 见下 P5 代码块 |

P4 参考实现(Pass 类型解构在 bin 侧,胶水模块零 lane_body 依赖):

```rust
// G37 W2 #82/#113:PSO 变体账本登记(era0 = precache 面/era≥1 = 守护面;
// 运行期新变体遭遇告警,验收 pso_runtime_creates == 0)。
pso_ledger.begin_session();
for p in descs.passes.iter() {
    let (pname, pspv) = match p {
        Pass::Compute(cp) => (cp.name, cp.spirv),
        Pass::Raster(rp) => (rp.name, rp.vs_spirv), // 现车道纯 compute;raster 出现时 vs/fs 双注册归后续
    };
    if pso_ledger.register(pname, pspv) {
        eprintln!("{GTAG}: [PSO] 运行期新 PSO 变体遭遇 pass={pname}（era≥1 未预测,#82 告警口径）");
        if pso_strict {
            fail("PSO strict:运行期新变体遭遇（RURIX_G31_PSO_STRICT=1,fail-closed）");
        }
    }
}
```

P5 参考实现:

```rust
// G37 W2:PSO 账本收口(sidecar 报告默认 off = 0-byte;计数恒登 stderr 单行)。
eprintln!(
    "{GTAG}: [PSO] sessions={} unique_variants={} pso_runtime_creates={}",
    pso_ledger.sessions(), pso_ledger.unique_variants(), pso_ledger.runtime_creates()
);
if let Some(path) = pso_report.as_deref() {
    std::fs::write(path, pso_ledger.report_json())
        .unwrap_or_else(|e| fail(&format!("--pso-report 写 {path}: {e}")));
}
```

(细节归主 agent 裁量:①Raster 臂 match 分支若 `Pass` 枚举名不同按 lane_body 实名对齐;②hzb 车道传的是 `hz_pass: &[Pass]` 切片,同款循环;③若希望账本含 headless 早退路径,P5 放在 evidence 写出函数入口处。)

---

## 3. 纪律遵守与验证结果(交付⑥)

- **0-byte 清单**(全程未触碰):`g31_window_present.rs`、`g14_3_lane_body.rs`、`g14_3_pipeline_perf.rs`、`kernels/g31_display_encode.rx` 原件、既有 .spv 字节、`post_chain.rs`、`pso_cache.rs`、milestones/、registry/、ci/、target-night。
- **cargo check**(dev,`-p rurix-render` 默认特性):**通过,零警告零错误**(新 selfcheck bin 编译面;edition 2024 autobins 自动发现,Cargo.toml 0-byte)。
- **selfcheck 真跑**(CPU,dev):`cargo run -p rurix-render --bin g37_w2_selfcheck` 退 0,单行 JSON `rurix.g37.w2_selfcheck.v1` 全断言过(LUT 恒等界/位级取角/.cube round-trip/拒臂;PSO 账本 precache/era 重建/新变体 RED 行为)。
- **kernel 编译**:rurixc(debug,`--features vulkan-backend`)`--target vulkan` 一次通过,内置 spirv-val accepted + 独立 spirv-val rc=0。fork 与母版 git diff 核对:代码级差异仅 kernel 名/LUT 段/三行量化变量。
- **禁 GPU/禁 release**:全程零 GPU 会话、零 `--release`。

## 4. 留窗与如实登记

1. LUT on(neutral) 对 off 非位级(新 SPV 字节 + trilinear f32 舍入);tetrahedral 插值(色彩 LUT 工业更优形)留窗,v1 = trilinear(host 参考逐操作同序,对拍面简单)。
2. bloom 多尺度 mip 链差距、AE histogram/双速率差距、tonemap 插件面差距——#79 收口附注三行,不在本役修。
3. `predict_precache_list` 材质×pass 面无消费(mega kernel 形态);raster/VisBuffer 波(#74)出现真 blend/cull 变体时账本键直接扩展(PsoDesc 全字段已在)。
4. 跨 era `VkPipelineCache` 复用/序列化 = rurix-rt 面,归主线(#86 联动)。
5. 账本是「变体新颖性」守护(UE precache 口径),不消除 era 重建的 Vulkan 重建墙钟税(该税已在 resize 语义内如实登记)。
6. 主 evidence schema 冻结 ⇒ PSO 计数走 sidecar;若主线愿升 schema 版本,`pso_runtime_creates` 建议进主 evidence(验收字段)。

## 5. 合入后 GPU 验收协议(主 agent 用)

1. **LUT off 锚**:`--frames 8 --warmup 2 --hidden` == `55e4a92d…`(all-off)+ `--quality full` 96f == `5db2e7d7…`(十六臂)——off 不载新 SPV,必须零漂移。
2. **LUT on 双跑**:`--lut neutral` 与 `--lut warm` 各双跑位级一致;`neutral` vs `warm` digest 必不同;`warm` 与 off 的 A/B(无 AE 对照,day_0829 教训)呈暖移方向(R 均值升/B 降)。
3. **device/host 对拍**(可选加严):`--dump-present-raw` + host `sample_trilinear_f32` 复算若干探针像素(γ 前域一致到 1 LSB)。
4. **PSO 守护**:任意臂 + `--window-storm 3 --pso-report pso.json` → `pso_runtime_creates == 0`、`sessions == 1 + resize_eras`;RED 臂 = 临时向 era≥1 注入异 SPV(或 strict 下人为换载)须告警/fail。
5. VUID=0 全程;帧时记账照旧(LUT 段预期 ≪1ms,8 角 SSBO 读 L2 驻留)。
