# visual_comparison.md — 画面对标度量与差距登记语义面（G10 M130/M134~M137/M139/M140）

> **地位**：G10 画面对标与图像度量轴事实源——双端确定性契约（M130）、帧
> 捕获 HDR 容器语义（M134）、FLIP/SSIM/PSNR 口径（M135/M136）、逐像素
> diff 报告 schema（M137）、差距清单 schema（M140）（RFC-0026，Agent
> Approved 2026-08-15，§4.0~§4.6 逐字承接；G10_CONTRACT §4.2 M130/M134/
> M135/M136/M137/M139/M140 行 + G10_ACCEPTANCE_MAP §1/§2/§3.3〔判据逐字〕）。
> 本文件不承载外部参照编排与语料治理（许可白名单/缓存/清单冻结属
> spec/external_reference.md RXS-0380~0383 面，**字面 0-byte 不动**）；
> 帧容器 EXR 语义挂 spec/imageio.md 追加新章（RXS-0114~0117 字面 0-byte，
> 归 G10.4 spec-first 波）。
>
> **档位**：Full RFC / RFC-0026。
>
> **编号**：本卷首条 = RXS-0384（G10.2 spec-first，自合入时实测
> `registry/number_ledger.json` `RXS.next_free = 384` 顺位领取；编号永不
> 复用，10 §9.5）。其余条款（度量域契约 / FLIP 口径 / SSIM·PSNR 口径 /
> diff 报告 schema / 差距清单 schema / EXR 容器语义）归 G10.4/G10.5
> spec-first 波按 actual next_free 顺位，本波不预写推测号。
>
> **新建裁决留痕（G10.2 spec PR）**：RFC-0026 §5 目标文件裁决逐字承接——
> 新建本卷（「画面对标度量与差距登记」语义轴；候选既有卷
> display_pipeline.md = 帧图输出与着色专项轴、imageio.md = 图像 IO 轴、
> rendering_platform.md = reflection/capability 轴，均不同轴，本体 0-byte；
> 沿 G9.2 virtual_geometry.md / G9.4 global_illumination.md / G9.5 双卷 /
> G9.6 physics.md / G10.3 external_reference.md 新建先例；spec/README.md
> §4 登记 + 本头注留痕）。
>
> **编号纪律**：条款号按合入当时实测 `next_free` 顺位，**不保证连续、不预
> 留区间**（RFC-0026 §5 + RFC-0027 §5 沿 RFC-0020 §5/F15 先例字面）；本卷
> 不消费任何 RX 错误码 / RD / U / SG（诊断类别实现期按真实可达类别从
> actual next-free 追加，en/zh message-key 同步）。

---

## 1. 范围与体例

- 体例 = FLS 风格（spec/README.md §2）；本文件**严禁 UB 节**——契约解析、
  canonical 编码、digest 比对与门序阻断的所有失败均为 typed 拒绝 / 确定性
  fail-closed（schema 外字段、非法枚举值、非单位四元数、NaN/±Inf、digest
  不等仍出报告一律判 RED），不设未定义行为。
- 实现锚定（实现期命名）：M130 双端确定性契约的 UE 侧解析器落
  `milestones/g10/harness/ue_python/g10_param_contract.py`（UE 进程内嵌
  CPython 载体，RFC-0026 §4.6 钉死）；Rurix 侧骨架期参考解析器与门脚本落
  `ci/g10_dual_determinism_contract_smoke.py`（symbolic key
  `g10.p0.m130.dual_determinism_contract`，G10.1 冻结字面不动）；Rust 消费
  面归 G10.5 实现波（RFC-0026 §4.6「Rurix 内部世界系 ↔ 契约世界系换算归
  Rurix 侧消费面（G10.2 骨架期登记）」）。
- 每条款 ≥1 `//@ spec: RXS-####` 测试锚定（traceability 矩阵全锚定，10 §4）。
- **G10 零通过线**：本卷不冻结任何画质通过阈值与帧率通过线（立项裁决 5 /
  G-G10-7 字面）；容差/阈值数值一律 M138 measured 标定入 `g10_budget.json`
  （P-09），本卷只冻结口径与 schema。

## 2. 术语

- **契约世界系**：右手系、+Y up、长度单位 = 米（与 glTF 资产链同构，
  RFC-0026 §4.6 值约定立约）；`position` / `sun.direction` 均以契约世界系
  表达，`orientation_quat` = 契约世界系下主动旋转（q = (w,x,y,z)，列向量
  v' = q·v·q*，正方向右手定则）。
- **canonical preimage**：解析后值的**二进制**编码（不经十进制文本）——
  版本前缀 + 键规范排序（Unicode code point 序）+ 逐字段类型标签 +
  length-prefix 字符串 + f64 取 IEEE-754 binary64 小端位模式 + u32/u64
  小端；digest = SHA-256(preimage)（同构 RXS-0305 CanonW 律）。
- **双端解析一致**：同一参数 JSON 文本，Rurix 端与 UE5 端各自解析 → 各自
  重编码 canonical preimage → 各自产 digest；两端 digest 相等 ⟺ 双端解析
  一致（含浮点 round-to-nearest 同值性）。
- **双 phase**：M130 单 key 双 phase（`--phase g10.2` 骨架期 /
  `--phase g10.5` 双端核验期），不拆双 key（G10_ACCEPTANCE_MAP §3.3 字面）；
  骨架期绿不替双端核验期充绿。

### RXS-0384 双端确定性契约：参数 schema 四节闭集 / 二进制 canonical preimage + SHA-256 / 双端解析一致性（M130）

**Legality**

- L1 **参数 schema 四节闭集**（全字段必填；schema 外字段注入即 RED、缺字
  段即 RED——strict fail-closed；`null` 仅 `sky.cubemap_id` 一位合法；
  NaN / ±Inf 禁入值域；契约 M130 字面「相机/光照/时间」三节为最低断言集，
  `post` 节为 RFC-0026 §4.6 扩集、不收缩契约判据）：
  - `camera`：`position`（f64×3，世界系）· `orientation_quat`（f64×4，
    **w,x,y,z 序冻结**，unit-norm 断言，非单位四元数拒绝）· `fov_y_deg`
    （f64，垂直视场角）· `near` / `far`（f64，登记面）· `resolution`
    （`{w, h}` u32）；
  - `lighting`：`sun {direction f64×3（unit 断言）, intensity_lux f64,
    color_linear_rgb f64×3}` · `sky {intensity f64, cubemap_id
    string|null}` · `exposure {mode: "manual", ev100 f64}`——**自动曝光
    禁入**（histogram 自动曝光破坏双端确定性；`mode` 闭集 v1 仅
    `"manual"`）；
  - `time`：`fixed_dt_s`（f64 固定步长）· `warmup_frames`（u32，TSR/时
    域累积收敛协议，R-G10-7）· `capture_frame_index`（u32，warmup 后捕
    获帧序号）· `random_seed`（u64）· `jitter {sequence: "halton_2_3",
    index_base u32, scale f64}`——Halton(2,3) 序列与 UE5
    `HaltonUtilities.cpp` 同族，索引基冻结，双端逐样本一致；
  - `post`：`view_transform`（**v1 合法值仅 `"aces13"`**——`"aces20"` /
    `"agx"` / `"neutral"` 保留演进位）· `bloom: false` · `vignette: false`
    · `motion_blur: false` · `dof: false`（v1 最小闭集 = 全关基线；加性
    演进走修订行）。
- L2 **值约定（应用一致面）**：契约世界系右手系 / +Y up / 米（L1 术语行
  冻结）；UE 侧应用映射（UE 5.8 惯例 = 厘米 / 左手系 / Z-up / 相机**水平**
  FOV）冻结公式——位置 `p_ue = (−z, x, y)·100`（cm；循环置换加一次取负，
  det = −1，右手系→左手系翻转成立）；旋转四元数向量部经同一 M 变换、标量
  部不变（相似变换 R_ue = M·R·M⁻¹，转角保持）；FOV：
  `fov_h_ue = 2·atan(tan(fov_y_deg/2)·aspect)`（同角度单位）；
  `sun.direction` 同 M（方向向量无单位换算）。**unit-norm 判定式（冻结常
  量）**：`orientation_quat` 判定 `|‖q‖² − 1| ≤ 2^-40`，`sun.direction`
  判定 `|‖d‖² − 1| ≤ 2^-40`，越界 fail-closed 拒绝；该常量为 schema 合法
  性谓词（f64 表示论下合法值的固有浮动界），**非 measured 标定值，不走
  `g10_budget.json`**（RFC-0026 §4.6 逐字）。应用层探针（标定场景标志物
  双端投影像素位置断言 `pixel_delta ≤ 1e-3 px`，同为合法性谓词常量）归
  双端核验期（`--phase g10.5`）与 M139 evidence 机核，骨架期不启用。
- L3 **canonical preimage 字节布局（字节级单源冻结，双端按同字面实现并
  对拍——RFC-0026 §4.6「字节布局全部自由量由 §5 拟落 spec 条款单源冻结
  字节级字面」的兑现点）**：
  - **版本前缀** = ASCII `"G10DCP-1"` + NUL，即字节序列
    `47 31 30 44 43 50 2D 31 00`（9 字节，位于 preimage 最前）；
  - **类型标签字节**：`f64 = 0x01` · `u32 = 0x02` · `u64 = 0x03` ·
    `str = 0x04` · `bool = 0x05` · `null = 0x06` · `obj_begin = 0x07` ·
    `obj_end = 0x08` · `arr_begin = 0x09` · `arr_end = 0x0A`；
  - **对象编码**：`obj_begin`，随后成员按 **key 的 Unicode code point 升
    序**排列；每成员 = key 编码 + value 编码；最后 `obj_end`。key 编码 =
    key 的 UTF-8 字节长度（u32 小端 length-prefix）+ UTF-8 字节（无 NUL
    终止）；根对象前无 key；
  - **数组编码**：`arr_begin`，随后元素按序编码（**无 key**），最后
    `arr_end`；
  - **标量编码**：`f64` = 标签 + 8 字节 IEEE-754 binary64 **小端**位模式
    （NaN / ±Inf 禁入）；`u32` = 标签 + 4 字节小端；`u64` = 标签 + 8 字节
    小端——**宽度由 schema 字段类型单源决定**（u64 字段值 < 2^32 仍编码
    8 字节，禁值域分派）；`str` = 标签 + u32 小端长度前缀 + UTF-8 字节；
    `bool` = 标签 + 单字节 `0x00` / `0x01`；`null` = 仅标签（唯一合法位
    = `lighting.sky.cubemap_id`）；
  - **preimage** = 版本前缀 9 字节 ‖ 根对象编码；**digest =
    SHA-256(preimage)**，64 位小写 hex 文本。canonical 编码/digest 不含路
    径、mtime、随机量（RFC-0026 §4.0 不变量 3）。
- L4 **双端解析一致性**：同一参数 JSON 文本双端各自解析 → 各自重编码
  canonical preimage → 各自产 digest；两端 digest 相等 ⟺ 双端解析一致。
  双端解析器均须 correctly-rounded（round-to-nearest ties-to-even）为口
  径要求；**边界浮点差分语料**（−0.0、次正规、2^53 边界、长十进制最短表
  示、1e-310 等）跨端解析逐位一致断言入 M130 GREEN。UE 侧 digest 载体钉
  死 = UE 进程内嵌 CPython（PythonScriptPlugin；`json` / `hashlib` /
  `struct` 标准库，CPython 浮点解析 correctly-rounded 由构造保证）；蓝
  图否决，host 侧脚本代算否决（digest 必须证明 UE 进程内的解析结果——
  **骨架期（`--phase g10.2`）豁免该载体要求**：双端 = harness UE 侧解析
  器（同一将在 UE 内嵌 CPython 运行的源文件）+ Rurix 侧骨架参考解析器，
  双端 schema 各一份、digest 比对面对拍就位；UE 进程内实跑 digest 归双端
  核验期）。M130 evidence 登记 `param_digest_rurix` / `param_digest_ue5`
  与共同值 `param_digest`（相等时）。
- L5 **双 phase 与门序硬约束（语义）**：骨架期（`--phase g10.2`）=
  schema 解析面 + 双端参数面就位 + digest 比对面就位，evidence
  `phase_g10_2_pass=true`、`phase_g10_5_pass=false`（骨架期绿不替双端核验
  期充绿）；双端核验期（`--phase g10.5`）= 双端真实参数 digest 比对相等
  实测，`phase_g10_5_pass=true` 方为完整绿。**门序硬约束（三重绑定，双端
  核验期生效）**：digest 不等不得出 A/B 报告——M139 机器前置 = (a) 当次
  双端 digest 相等 ∧ (b) == M130 双端核验期最新 evidence 登记
  `param_digest` ∧ (c) 同 `base_commit` 同 `session_run_id`；陈旧 pass 不
  得冒充当次一致（RFC-0026 §4.0 不变量 4 / §4.6 逐字）。单端参数漂移注
  入即 RED；schema 外字段注入即 RED；digest 不等仍出 A/B 报告即 RED（契
  约 §4.2 M130 行字面）。

**Implementation Requirements**

- IR1 本条款挂接 `g10.p0.m130.dual_determinism_contract`（G10.2 骨架 →
  G10.5 双端核验）；测试锚定 = conformance/visual_comparison/ 语料 +
  `ci/g10_dual_determinism_contract_smoke.py` 门脚本。
- IR2 harness `g10_param_contract.py` 的字节布局常量必须与 L3 字面逐字节
  一致（骨架期机器核验：版本前缀字节值、类型标签字节值、schema 驱动整数
  宽度）；任何漂移即 fail-closed。
- IR3 RED 语料（conformance/visual_comparison/reject/）：单端参数漂移 /
  schema 外字段注入 / 非单位四元数注入——转正路径为 M130 门脚本内注入臂
  （同参数双端 digest 不等检出 / 解析器确定性拒绝）。

### RXS-0386 度量域契约：HDR/LDR 双臂捕获点 / LDR 臂派生路径 / 帧域标签与度量域互证（M134）

**Legality**

- L1 **双臂捕获点（域闭集 `domain ∈ {"scene-linear-hdr","display-referred-ldr"}`）**：
  HDR 臂（`scene-linear-hdr`）= tonemap / view transform **之前**的
  scene-referred 线性帧（Rurix 侧 = 后处理骨架〔RXS-0370〕tonemap 节点之前
  的 HDR 线性域帧，RXS-0369~0373 字面 0-byte 消费）；LDR 臂
  （`display-referred-ldr`）= 显示域 sRGB `[0,1]` 编码帧。HDR 臂是画质
  差距主战场；LDR 臂服务显示域体感对照与 SSIM/PSNR 口径面（RXS-0387）。
- L2 **LDR 臂派生路径**：LDR 帧由**本端 HDR 帧派生**——HDR 帧为权威源、
  LDR 帧为派生产物；view transform 双端共用同一参数字面（v1 仅
  `"aces13"`，RXS-0384 L1 `post.view_transform` 字面）；view transform
  后的线性显示域帧经**双端共用同一 host 侧 sRGB 编码步骤**（编码器口径
  单源）产 sRGB 编码帧，编码差从构造上消除；UE 侧产出路径 = 派生路径
  （UE 官方文档明示 `.exr` 不应用 sRGB 编码曲线，RFC-0026 §4.1/Q13）。
  派生链元数据互证：LDR 帧 `rurix:derivation="derived:host-srgb-encoder-v1"`
  且 `rurix:source_frame_digest` = 派生源 HDR 帧 digest，缺失即 RED。
- L3 **帧域标签与度量域互证**：度量计算的域入参必须等于输入帧元数据
  `rurix:domain` 与 `rurix:transfer`（HDR 臂 ⟺ `linear`，LDR 臂 ⟺
  `srgb`）；域标签错配 / transfer 错配（sRGB-线性混标）注入即 RED；
  HDR 帧直算 SSIM/PSNR 即口径混用 RED（RXS-0387 L1）。度量域互证失败
  一律 fail-closed，不静默降级。

**Implementation Requirements**

- IR1 本条款挂接 `g10.p0.m134.frame_capture_pipeline`（G10.4）；测试锚定 =
  conformance/visual_comparison/ 语料 + `ci/g10_frame_capture_pipeline_smoke.py`
  门脚本（域标签错标注入臂）。
- IR2 帧元数据闭集（域/transfer/派生链字段字面）以 spec/imageio.md
  RXS-0385 L3 为单源；本条款只冻结度量域契约，不重述字段表。

### RXS-0387 SSIM/PSNR 口径闭集：Wang 2004 参数化 / LDR 域限定 / 恒等图对极值断言 / 参考实现对拍（M136）

**Legality**

- L1 **域限定（防口径混用）**：SSIM/PSNR **仅在 LDR 臂定义**——显示域
  sRGB `[0,1]`，`data_range = 1.0`；HDR 臂不定义 SSIM/PSNR（无界动态
  范围下 data_range 无公认取值，口径不适定；HDR 域差异由 HDR-FLIP 承担，
  RFC-0026 §4.2/§4.3）。**任何在 HDR 帧上直接计算 SSIM/PSNR 的请求即
  口径混用，fail-closed 拒绝**（HDR 直算注入即 RED）。
- L2 **SSIM 口径闭集（Wang et al. 2004 标准参数化，闭集外参数禁调）**：
  窗 = 11×11 高斯窗 σ = 1.5；常数 K1 = 0.01、K2 = 0.03，
  C1 = (K1·L)²、C2 = (K2·L)²，L = data_range = 1.0；协方差 = 总体协方差
  （不采样校正，`use_sample_covariance = false`）；聚合 = 逐通道 SSIM →
  RGB 三通道均值（mean-SSIM 均值聚合，**非** multi-scale MS-SSIM〔Wang
  2003〕，不对齐 multi-scale 变体）；返回值域 `[-1, 1]`。
- L3 **PSNR 口径闭集**：MSE = RGB 三通道联合均方误差；
  `PSNR = 10·log10(L²/MSE)`，L = 1.0。**恒等图对极值断言语义**：位级
  相同图对 → SSIM 恰为 `1.0`、PSNR 为 `+inf`（JSON 序列化约定：PSNR
  字段类型 = number 或字符串字面 `"inf"`——MSE = 0 时的闭集例外值；
  解析器对 `"inf"` 与有限值双形态均须接受，其余字符串拒绝）。恒等图对
  非极值即 RED。
- L4 **参考实现与对拍**：参考实现 = scikit-image
  `structural_similarity`（显式参数化 `gaussian_weights=True, sigma=1.5,
  win_size=11, use_sample_covariance=False, data_range=1.0, channel_axis`
  显式）与 `peak_signal_noise_ratio`（`data_range=1.0`），**版本 pin +
  digest 登记**随 evidence；自实现与参考实现在同一测试图集上逐图对拍。
  **对拍图集下界**：图集 ≥ 24 图对；内容类五类每类 ≥ 4——高频边缘 /
  平滑渐变 / 噪声 / 高亮截断（clip）/ 色彩孤立区；图集清单与每图
  digest 入 evidence；**不满足下界的对拍不构成有效标定**（稀释通道封
  堵）。**对拍容差**：自实现与参考实现逐图标量差（SSIM 与 PSNR 分列）
  的样本最大值（p100）× 安全系数 k（k ∈ [1.0, 3.0]），容差数值一律
  measured 标定（M138 正式入 `g10_budget.json`；G10.4 波以 provisional
  形态随 evidence 登记 provenance，禁手写阈值冒充标定——估计器形态 =
  p100 × k，样本集 = 对拍图集 digest 引用，RFC-0026 §4.2 F10 字面）。
  口径漂移注入（闭集外参数或值漂移）即 RED；参考输出扰动注入即 RED。

**Implementation Requirements**

- IR1 本条款挂接 `g10.p0.m136.ssim_psnr_metric`（G10.4）；测试锚定 =
  conformance/visual_comparison/ 语料 + `ci/g10_ssim_psnr_metric_smoke.py`
  门脚本。
- IR2 自实现面 = ci/ 工具链 Python/numpy 按 Wang 2004 原文逐字实现
  （高斯窗/总体协方差/逐通道均值聚合字面与 L2 逐字一致）；与 G5 既有
  SSIM 门禁 helper（`src/rurix-render/src/temporal/ssim.rs`，8×8 盒式窗）
  **不同属一套口径**——字面 0-byte 不动，两口径并存、各自登记、互不
  冒充（RFC-0026 §4.3 0-byte 声明）。
- IR3 RED 语料（conformance/visual_comparison/reject/）：HDR 直算注入 /
  口径漂移注入 / 恒等图对非极值注入 / 图集不满足下界冒充——转正路径为
  M136 门脚本内注入臂。

### RXS-0388 逐像素 diff 报告 schema：双层产物 / 区域统计字段闭集 / evidence JSON 闭集（M137）

**Legality**

- L1 **双层产物（同一误差缓冲的确定性投影，互不一致即 RED）**：
  ① 机器 canonical 面 = 逐像素误差 EXR——float32 **单通道 Y**、无损
  （spec/imageio.md RXS-0385 单通道形态）、域随输入帧；色彩映射前的
  标量场是唯一事实源（G10.5 A/B 期 FLIP 域误差图直接取 FLIP
  `error_map_output`，RFC-0026 §4.2/§4.4；G10.4 门内误差缓冲供给口径
  = 逐像素 RGB 通道最大绝对差 `e = max(|Ra−Rb|,|Ga−Gb|,|Ba−Bb|)` 钳制
  `[0,1]`，登记为门内供给口径、非 schema 语义本体）。
  ② 人读面 = 灰度热区图——误差 `e ∈ [0,1]` 经冻结色彩映射闭集 v1 =
  `{"gray"}`（`e → [e,e,e]`，零色表常量）映射后按 RXS-0116 确定量化
  口径（clamp + 就近取整）落 8-bit 灰度，经 image-io 既有无损通道编码
  （PPM P6；PNG 接通后同语义加性可用）。
- L2 **逐区域统计字段闭集**：固定网格 `region_grid = {nx, ny}`（v1 默认
  16×16；网格维度入 schema 字段登记，改值走修订行）；`regions[]` 每区域
  字段闭集 = `{x, y, w, h, pixel_count, err_max, err_mean, err_p95,
  over_threshold_count}`。**百分位口径（冻结）**：`err_p95` = nearest-
  rank——N 个样本升序排序取第 ceil(0.95·N) 个（1-based；ceil(0.95·N)
  < 1 时取 1；禁插值法）。**网格边缘规则（冻结）**：分辨率不被
  `region_grid` 整除时，末行/末列区域 `w`/`h` 取实际剩余像素
  （`pixel_count` = w·h 逐区域对账，禁漂移）。`over_threshold_count`
  的阈值 = M138 标定值（噪声底上方）；报告内嵌阈值数值 +
  `thresholds.source_digest` 引用（G10.4 波 thresholds 以 provisional
  形态登记 `source="provisional_pending_m138"` 与推导 provenance，M138
  正式入 `g10_budget.json` 后翻 `source="g10_budget.json"`，禁手写阈值
  冒充标定）。
- L3 **标量报告（全图聚合）**：`scalars` 字段闭集 = 域对应指标集
  （HDR 臂：`flip`；LDR 臂：`flip` / `ssim` / `psnr`——G10.4 门内
  FLIP 未接通，对应字段登记 `null` 演进位，G10.5 翻转实值）+ 误差全图
  统计 `{err_max, err_mean, err_p95, over_threshold_pixel_count,
  over_threshold_ratio}`。
- L4 **evidence JSON 字段闭集（闭集外字段拒收；空场景行即 RED）**：
  `schema_version`（报告 schema 版本，v1 起加性演进）· `scene_id` /
  `camera_id` / `frame_index`（场景/机位/帧定位三元组，空串/缺失即
  RED）· `end_pair`（双帧标识与各自 digest——`{frame_a, frame_b}` 各
  含 `source_end` / `frame_id` / `digest`；G10.5 A/B 期 frame_a = rurix
  帧、frame_b = ue5 帧，G10.4 门内双探针帧 `source_end` 均 `"rurix"`
  登记）· `domain`（与帧元数据互证）· `metric_caliber`（口径参数闭集
  的 digest，口径版本互证）· `thresholds`（`{value, source,
  source_digest}`）· `region_grid` / `regions[]`（字段闭集见 L2）·
  `scalars`（字段闭集见 L3）· `artifacts`（`{frame_a_digest,
  frame_b_digest, error_map_digest, heatmap_digest}` 四 digest 闭集）·
  `determinism_contract_digest`（M130 链 digest，G10.4 门内探针对登记
  探针描述符 digest、不冒充 M130 链）· `provenance`（环境画像引用）。
  **一致性判据**：误差 EXR / 热区图 / 区域统计三面由同一误差缓冲
  重算一致 golden；diff 图与标量报告不一致注入即 RED；空场景行注入即
  RED；闭集外字段注入即 RED。

**Implementation Requirements**

- IR1 本条款挂接 `g10.p0.m137.pixel_diff_report`（G10.4）；测试锚定 =
  conformance/visual_comparison/ 语料 + `ci/g10_pixel_diff_report_smoke.py`
  门脚本。
- IR2 实现面 = `src/rurix-render/src/bin/g10_m137_diff_report.rs`（host
  纯 safe 报告器：读两帧 EXR〔image-io RXS-0385 解码〕→ 误差缓冲 → 三
  投影产物 + evidence JSON）；门侧 Python 独立重算核验（ci/ 独立 EXR
  解析器，双实现互证——区域统计由误差 EXR 重算一致、热区图由误差 EXR
  重算逐字节一致）。
- IR3 RED 语料（conformance/visual_comparison/reject/）：diff 图与标量
  报告不一致注入 / 空场景行注入 / 闭集外字段注入——转正路径为 M137 门
  脚本内注入臂。

### RXS-0389 FLIP 口径闭集：参考实现 pin 五元组 / 双域口径 / 恒等图对极值断言 / 对拍容差两面分列（M135）

**Legality**

- L1 **参考实现选型与版本 pin（pin 五元组，缺一元即 RED）**：FLIP 参考
  实现 = NVIDIA FLIP 官方开源实现 NVlabs/flip（Andersson et al.,
  *FLIP: A Difference Evaluator for Alternating Images*, HPG 2020；
  BSD-3-Clause）。**pin 五元组**随 evidence 登记（R-G10-3 版本漂移对策）：
  ① commit digest（联网获取时 `git ls-remote` 实测登记，zip 快照 digest
  双记）；② 实现分支/后端（枚举闭集 `{"cpp-tool", "cpp-header-lib",
  "cuda", "python-nanobind"}`，登记采用形态）；③ OS/工具链（OS + 编译器
  + CMake + Python + nanobind/scikit-build-core 版本）；④ 构建配置
  （构建系统 + 产物 wheel digest）；⑤ 运行参数集（域/输入色彩空间旗标/
  色彩映射旗标/均值聚合旗标/参数字面）。上游明示跨 OS 输出可像素级不
  一致、C++ 与 CUDA 后端结果亦不同（仓库 `misc/precision.md` 精度声明），
  故分支/后端与 OS/工具链必须是显式 pin 维度；任一 pin 维度漂移即口径
  漂移 RED。
- L2 **双域口径**（与 RXS-0386 L1 双臂一一对应）：**HDR-FLIP** 输入 =
  HDR 臂 scene-linear 帧（线性 HDR）；曝光参数面对齐参考实现实际面——
  `hdr_exposure_mode ∈ {"auto-from-reference", "fixed"}`：
  `auto-from-reference` = 由**参考图中位亮度**推导 start/stop 曝光
  （参考实现 v1.7 起 median=0 安全）；`fixed` 时
  `{hdr_exposure_start, hdr_exposure_stop, hdr_num_exposures}` 三参必填；
  单值 `hdr_exposure_value` 形态否决（与参考实现参数面不符，照抄即不可
  执行）。**LDR-FLIP** 输入 = LDR 臂显示域 sRGB `[0,1]` 帧。**域互证**：
  度量 `domain` 入参必须等于输入帧元数据 `rurix:domain`（RXS-0386 L3），
  错配拒绝（fail-closed）。
- L3 **口径参数闭集**（闭集外参数禁调；值随参考实现默认 pin，偏离默认
  须经 M138 标定程序登记理由）：

  | 参数 | 闭集/口径 |
  |---|---|
  | `domain` | `"hdr"` / `"ldr"`（与帧 `rurix:domain` 互证，错配拒绝） |
  | `ppd` | pixels-per-degree 正数；或由 viewing geometry 三参数（`viewing_distance_m` / `screen_width_m` / `resolution_x`）按参考实现公式 `ppd = dist · (res_x / mon_w) · π/180` 推导——两形态二选一，登记采用形态；**ppd 策略冻结：全语料单一值或单一推导几何**（采用形态与取值随 `metric_caliber` digest 登记；语料内逐场景漂移即口径漂移 RED，跨场景 FLIP 标量方可比）；变更走修订行 |
  | `hdr_exposure_mode` / `hdr_exposure_start` / `hdr_exposure_stop` / `hdr_num_exposures` | HDR 域曝光参数面（见 L2，对齐参考实现 start/stop/N + auto 语义） |
  | `colorspace_transform` | `"YCxCz"`（论文口径，冻结） |
  | `feature_filters` | 边缘/点检测参数集 = 参考实现默认（`gw = 0.082`、`gqf = 0.5`；pin 五元组覆盖） |
  | `spatial_pooling` | 加权均值聚合（全图算术均值），输出标量 ∈ `[0,1]`（0 = 不可区分） |
  | `error_map_output` | 必开（逐像素误差图，RXS-0388 机器 canonical 面的 FLIP 源） |

  颜色/特征常量闭集（论文口径冻结）：`gqc = 0.7`、`gpc = 0.4`、
  `gpt = 0.95`、`gw = 0.082`、`gqf = 0.5`；空间滤波常量
  `a1 = (1.0, 1.0, 34.1)`、`b1 = (0.0047, 0.0053, 0.04)`、
  `a2 = (0.0, 0.0, 13.5)`、`b2 = (1e-5, 1e-5, 0.025)`；色彩管道 =
  sRGB → linear RGB → XYZ(D65) → YCxCz → 空间滤波（分离卷积、边缘
  clamp）→ 回线性 RGB clamp [0,1] → CIELAB → Hunt 调整
  （`0.01·L·a` / `0.01·L·b`）→ HyAB 距离 → `^gqc` → cmax/pccmax
  分段重映射；特征差 = 边缘/点差大者经 `1/√2` 归一后 `^gqf`；最终
  误差 = `color_diff ^ (1 − feature_diff)`。
- L4 **恒等图对极值断言**：位级相同图对 → FLIP 标量**恰为 `0`**（误差
  图逐像素恰为 0）；非零即 RED。
- L5 **对拍与容差（两面分列，measured 标定）**：自实现与参考实现在同一
  测试图集上逐图对拍。**对拍图集下界**：图集 ≥ 24 图对；内容类五类每类
  ≥ 4——高频边缘 / 平滑渐变 / 噪声 / 高亮截断（clip）/ 色彩孤立区
  （RXS-0387 L4 同一图集与下界语义，两度量共用）；图集清单与每图 digest
  入 evidence；**不满足下界的对拍不构成有效标定**（稀释通道封堵）。
  **对拍容差两面分列**——标量对拍容差（逐图 FLIP 标量差）与**误差图
  对拍容差**（逐像素误差图差）分列 M138 标定、分列登记：上游明示跨
  OS/跨后端误差图可像素级漂移，而 RXS-0388 机器 canonical 误差 EXR
  直接取 FLIP 误差图，容差面必须覆盖误差图而非仅标量。**估计器语义
  （冻结）**：统计量 = 全图集逐图 |自实现 − 参考实现| 差（标量差与误差
  图逐像素差分列）的样本最大值（p100）；容差 = p100 × 安全系数 k，
  k ∈ [1.0, 3.0]（取值与选择理由随 `g10_budget.json` provenance 登记；
  估计器形态变更走修订行）；样本集 = 对拍图集 digest 引用。容差数值
  一律 M138 measured 标定（G10.4 波以 provisional 形态随 evidence 登记
  provenance，禁手写阈值冒充标定）。参考输出扰动注入即 RED；口径参数
  漂移（闭集外参数或值漂移）注入即 RED。

**Implementation Requirements**

- IR1 本条款挂接 `g10.p0.m135.flip_metric`（G10.4）；测试锚定 =
  conformance/visual_comparison/ 语料 + `ci/g10_flip_metric_smoke.py`
  门脚本。
- IR2 自实现面 = ci/ 工具链 Python/numpy 按 L3 口径管道逐字实现
  （YCxCz 变换 / 分离空间滤波 / Hunt-HyAB 色差 / 边缘·点特征滤波 /
  分段重映射 / 最终误差合成字面与 L3 逐字一致）；参考实现面 =
  NVlabs/flip 按 L1 pin 五元组落地（选臂与构建受阻回退臂如实登记），
  参考输出经 `flip_evaluator.evaluate(...)`（或选臂对应入口）逐图取得。
- IR3 RED 语料（conformance/visual_comparison/reject/）：参考输出扰动
  注入 / 口径参数漂移注入 / 恒等图对非零注入 / 图集不满足下界冒充——
  转正路径为 M135 门脚本内注入臂。

### RXS-0390 应用层探针：冻结标志物集与双端投影像素一致性断言（M130 双端核验期 / M139）

**Legality**

- L1 **探针语义（应用一致机核面）**：digest 证解析一致、探针证应用一致
  （RFC-0026 §4.6 值约定末节「应用层探针」逐字承接——M130 双端核验期与
  M139 evidence 各含 `application_probes[]`）。标定场景冻结标志物集
  （L2 逐值冻结）经**双端各自管线**按当次契约参数投影为像素坐标：
  Rurix 端 = 契约相机直接消费面（`g10_5_scene_render --project-landmarks`
  探针，契约四元数 → f64 view/proj 对账面，与渲染主路径同 look-at/
  针孔口径）；UE 端 = 契约 → RFC-0026 §4.6 冻结映射（含本卷 RXS-0384 L2
  errata 后的四元数共轭修订式）→ UE 相机视/投影链——视空间三分量
  （`rel·right` / `rel·up` / `rel·fwd`，fwd/right/up = 相机 actor 世界
  三轴）、针孔水平 FOV 投影、`px = (ndc.x/2 + 0.5)·w` /
  `py = (0.5 − ndc.y/2)·h` 像素映射（UE 5.8 源树一手锚定：
  `GameplayStatics::CalculateViewProjectionMatricesFromMinimalView`
  〔GameplayStatics.cpp〕+ `FReversedZPerspectiveMatrix`
  〔PerspectiveMatrix.h〕+ `FSceneView::ProjectWorldToScreen`
  〔SceneView.cpp〕）。**判定式（冻结常量）**：逐标志物双端像素差
  `pixel_delta = max(|Δx|, |Δy|) ≤ 1e-3 px`（**schema 合法性谓词常量**，
  与 RXS-0384 L2 unit-norm 判定式同登记口径——非 measured 标定值，
  **不走 `g10_budget.json`**；RFC-0026 §4.6 逐字）；超差即「应用不一致」
  RED。
- L2 **冻结标志物集（逐值字面冻结；改动走修订行）**：
  - `cornell-box`（毫米量级数值面，盒后墙平面 z=558.8 四角 + 中心，
    五点）：`(0.0, 0.0, 558.8)` · `(552.8, 0.0, 558.8)` ·
    `(552.8, 548.8, 558.8)` · `(0.0, 548.8, 558.8)` ·
    `(276.4, 274.4, 558.8)`；
  - `bistro-interior`（米，相机系合成标定标志物五点——深度 2.0 m、
    相机 NDC 面 `(0,0)` / `(±0.6, ±0.4)` 定格换算的世界常量）：
    `(2.0375248420941845, 1.3697032820278594, -1.6595583445401449)` ·
    `(2.1463398736291461, 1.6862064060565474, -0.82191749619001619)` ·
    `(1.9521887623639345, 1.6862064214520678, -2.4999157956664435)` ·
    `(2.1228609218244348, 1.053200142603651, -0.81920089341384617)` ·
    `(1.9287098105592226, 1.0532001579991714, -2.4971991928902737)`；
  - 入帧前提（条款作者 2026-08-15 实测登记）：cornell-box 五点
    |ndc| ≤ 0.573（512×512 帧内）；bistro-interior 五点 ndc = 设计定格
    值（1920×1080 帧内）。
- L3 **机核面**：M130 双端核验期（`--phase g10.5`）evidence 与 M139
  evidence 各含 `application_probes[]`——逐场景逐标志物 `pixel_rurix` /
  `pixel_ue5` / `pixel_delta` 实测值与逐点 pass 布尔；探针缺失或任一点
  超差即 RED；标志物集任一字面漂移（对照 L2 逐值 fail-closed）即 RED。

**Implementation Requirements**

- IR1 本条款挂接 `g10.p0.m130.dual_determinism_contract --phase g10.5`
  （G10.5）与 `g10.p0.m139.ab_comparison`（同波 evidence 面）；测试锚定
  = `conformance/visual_comparison/accept/application_probe_minimal.rx`
  + `ci/g10_dual_determinism_contract_smoke.py` 门脚本 g10.5 腿。
- IR2 UE 端探针载体 = **UE 进程内嵌 CPython**
  （`milestones/g10/harness/ue_python/g10_5_probe_landmarks.py`，契约映射
  经 `g10_param_contract.py` 单源消费，禁脚本内手写第二份）；host 侧
  代算否决（RXS-0384 L4 载体纪律同口径）；Rurix 端 =
  `g10_5_scene_render --project-landmarks`（Rust 消费面第三实现）。

### RXS-0391 差距清单 schema：字段闭集 / UE5 模块归属枚举闭集 / kind 两值分列 / measured_delta 可溯源 / 场景全集零空行对账（M140）

**Legality**

- L1 **清单 JSON 顶层字段闭集**（闭集外字段拒收——fail-closed）：
  `{schema_version, registry, generated_by, scene_set, items, scene_summary, not_ready_scenes}`；
  `schema_version` const `1`；`registry` const `"g10_gap_registry"`；
  `generated_by` 非空字符串（产出门脚本字面）；`scene_set` = 场景全集
  字符串数组（与 M133 冻结清单行集闭集对账面）。
- L2 **差距项字段闭集**（13 键 + 1 条件可选键）：
  `{gap_id, scene_id, camera_id, domain, kind, ue5_module_primary, ue5_module_secondary, measured_delta, suggested_priority, g11_anchor, title, description, attachments}`
  ——闭集外字段拒收；`attribution_note` 为唯一可选键，**当且仅当
  `ue5_module_primary` 取 Other 终值时必填非空**（Other 行计数进 evidence
  统计防滥用，RFC-0026 §4.5 终值行字面）。
- L3 **gap_id 派生（冻结字节规则，重跑可复现）**：
  `gap_id = sha256(utf8(scene_id) ‖ 0x00 ‖ utf8(camera_id) ‖ 0x00 ‖ utf8(ue5_module_primary) ‖ 0x00 ‖ utf8(kind) ‖ 0x00 ‖ utf8(title))`
  的全小写 hex **前 16 字符**（‖ 分隔符 = 单字节 `0x00`，字段序即本序；
  RFC-0026 §4.5「sha256(scene_id ‖ camera_id ‖ ue5_module_primary ‖ kind ‖ title)
  前 16 hex 派生」的字节级落地——分隔符与编码本条款单源冻结）。
- L4 **kind 两值分列**（RFC-0026 §3.3/§4.5）：`"quality_gap"`（画质差距）
  与 `"caliber_diff"`（口径差）——两值闭集外取值即 RED；口径差项与画质
  差距项不得互相冒充（G10 零通过线：两族全量登记即绿，均不设判线）。
- L5 **UE5 模块归属枚举闭集**（`ue5_module_primary` / `ue5_module_secondary[]`
  同闭集；枚举值 = 规范化正斜杠路径字面，公共前缀
  `Engine/Source/Runtime/Renderer/Private/`；版本锚 = Launcher 5.8.0 正式版
  release 口径，快照复核风险注记沿 RFC-0026 §4.5 字面）：
  - **目录级 23 值**：`CompositionLighting` · `Froxel` · `HairStrands` ·
    `HeterogeneousVolumes` · `InstanceCulling` · `Lumen` · `MaterialCache` ·
    `MegaLights` · `Nanite` · `OIT` · `PostProcess` · `RayTracing` ·
    `Renderer` · `SceneCulling` · `Shadows` · `Skinning` ·
    `SparseVolumeTexture` · `StateStream` · `StochasticLighting` ·
    `Substrate` · `VariableRateShading` · `VirtualShadowMaps` · `VT`。
  - **文件级 57 值**（curated 子集 + 补收触发条件沿 RFC-0026 §4.5 字面）：
    `PathTracing.cpp` · `PathTracingSpatialTemporalDenoising.cpp` ·
    `SceneCaptureRendering.cpp` · `SkyAtmosphereRendering.cpp` ·
    `SkyPassRendering.cpp` · `VolumetricCloudRendering.cpp` ·
    `VolumetricFog.cpp` · `SingleLayerWaterRendering.cpp` ·
    `WaterInfoTextureRendering.cpp` · `SubsurfaceTiles.cpp` ·
    `DBufferTextures.cpp` · `TranslucentRendering.cpp` ·
    `TranslucentLighting.cpp` · `FrontLayerTranslucency.cpp` ·
    `ShadowRendering.cpp` · `ShadowSetup.cpp` · `ShadowDepthRendering.cpp` ·
    `CapsuleShadowRendering.cpp` · `DistanceFieldAmbientOcclusion.cpp` ·
    `DistanceFieldShadowing.cpp` · `DistanceFieldScreenGridLighting.cpp` ·
    `DistanceFieldLightingPost.cpp` · `GlobalDistanceField.cpp` ·
    `ReflectionEnvironment.cpp` · `ReflectionEnvironmentCapture.cpp` ·
    `ReflectionEnvironmentDiffuseIrradiance.cpp` ·
    `ReflectionEnvironmentRealTimeCapture.cpp` ·
    `PlanarReflectionRendering.cpp` · `ScreenSpaceReflectionTiles.cpp` ·
    `ScreenSpaceRayTracing.cpp` · `ScreenSpaceDenoise.cpp` ·
    `FogRendering.cpp` · `LocalFogVolumeRendering.cpp` ·
    `LightRendering.cpp` · `IndirectLightRendering.cpp` ·
    `LightShaftRendering.cpp` · `BasePassRendering.cpp` · `DepthRendering.cpp` ·
    `VelocityRendering.cpp` · `AnisotropyRendering.cpp` ·
    `DecalRenderingShared.cpp` · `GPUScene.cpp` · `HZB.cpp` ·
    `SceneVisibility.cpp` · `DeferredShadingRenderer.cpp` · `Renderer.cpp` ·
    `HaltonUtilities.cpp` · `BlueNoise.cpp` · `HdrCustomResolveShaders.cpp` ·
    `GPUBenchmark.cpp` · `ShadingEnergyConservation.cpp` ·
    `IESTextureManager.cpp` · `RectLightTextureManager.cpp` ·
    `LightFunctionRendering.cpp` · `VolumeLighting.cpp` ·
    `HeightfieldLighting.cpp` · `DistortionRendering.cpp`。
  - **终值**：`Other`（全路径 = 公共前缀 + `Other`）——须
    `attribution_note` 非空说明。
  - **演进纪律**：闭集**只追加修订行**；旧值永不删除（10 §9.5 同构）。
- L6 **measured_delta 可溯源**（≥1 项，纯叙述无测量即 RED——M140 RED 臂
  字面）：每项字段闭集 `{metric, a_value, b_value, delta, region_ref, evidence_digest}`
  （`region_ref` 为唯一可选键，diff 报告区域/场景引用）；`a_value` /
  `b_value` 为双端或对拍测量值（f64），**`delta == b_value − a_value`
  f64 精确相等**（机器重算面）；`evidence_digest` 必须可回溯到 M137 diff
  报告 / M139 A/B 报告 evidence 登记的 artifact digest（门侧机核：
  不在最新 M139 evidence `ab_report.artifact_digests[]` 登记集内即 RED）。
- L7 **建议 P 级与承接锚**：`suggested_priority` ∈ `{"P0","P1","P2"}`
  （建议值，G11 立项重裁，本字段不构成承诺——RFC-0026 §4.5 字面）；
  `g11_anchor` 非空字符串（G11 立项只消费 G10.8b 锁定清单 + 本锚，
  契约 G-G10-11 字面）；缺归属/缺承接锚行即 RED。
- L8 **场景全集零空行对账**（M139/M140「差距清单缺场景行即 RED」字面）：
  `scene_summary[]` 逐场景 `{scene_id, gap_count, no_gap_explicit}` 字段
  闭集；行集与 `scene_set` 精确全等（禁静默丢行）；`no_gap_explicit ==
  (gap_count == 0)`（无差距场景显式 `no_gap_explicit=true` 汇总行）；
  `not_ready_scenes` 显式在列（G10_ACCEPTANCE_MAP §3.2 not-ready 登记面，
  可空集但键必须存在）。
- L9 **domain 两值**（与 diff 报告/帧元数据互证面，RXS-0386 L1）：
  `{"scene-linear-hdr", "display-referred-ldr"}`；取值须与该项
  measured_delta 锚定的度量域一致。

**Implementation Requirements**

- IR1 本条款挂接 `g10.p0.m139.ab_comparison`（清单落盘产出门）与
  `g10.p0.m140.gap_registry`（登记核验门）；测试锚定 =
  `conformance/visual_comparison/accept/gap_registry_minimal.rx` +
  `conformance/visual_comparison/reject/gap_registry_missing_attribution.rx`
  + `conformance/visual_comparison/reject/gap_registry_unmeasured_narrative.rx`
  + `ci/g10_gap_registry_smoke.py` 门脚本。
- IR2 共享判定层 = `ci/g10_gap_registry_lib.py` 单一事实源（枚举闭集 /
  字段闭集 / gap_id 派生 / 校验器），M139 落盘侧与 M140 门侧同一实现
  消费，禁第二份手写（RXS-0384 L4 同构载体纪律）。

### RXS-0392 C1 口径对齐：天光/太阳辐照链参数化对齐 + 残余口径差显式登记 / 不拟合（M144，G11.2）

**Legality**

- L1 **不拟合原则**：C1 口径差对齐 = 双端天光/太阳辐照链**参数化**对齐
  + 残余口径差显式登记；禁止以拟合/反向调参使双端亮度 delta 人为缩小
  ——对齐 = 参数化口径一致或显式登记残余（RFC-0028 §4.5 冻结语义；
  G11_CONTRACT §4.2 M144 判据字面「拟合冒充对齐即 RED」）。
- L2 **天光链参数集枚举闭集**（逐项双端同参数登记，任一环节不对齐即
  该环节残余口径差）：`{ 天光模式（cubemap IBL / 探针采样常量天光）、
  强度（同单位链——UE SkyLight 指定 cubemap 强度 × cubemap 值 =
  scene-linear 辐射度 cd/m²；Rurix 常量天光辐射度同单位）、色温/光谱面
  （常量值或 cubemap 资产 digest）、采样档位（探针分辨率/光线数档位） }`；
  cubemap 资产若使用须过 M131 许可白名单登记面（SPDX/来源
  URL/attribution/资产 digest，未登记资产混入即 RED——RFC-0028 §4.5.1
  字面）；白色常量 cubemap 的资产 digest 与逐像素值核验（= 1.0 uniform）
  进门 evidence。
- L3 **太阳 lux→辐射度链**（EV100 同字面前提，C2 派生尺度面与本链解耦
  ——RXS-0386 L2 既有口径 0-byte）：双端链式逐项参数化对齐并登记
  provenance——UE 臂 `DirectionalLight.intensity = 契约 sun.intensity_lux`
  （lux = 面元辐照度 E），朗伯出射辐射度 `L = ρ·E·(n·l)/π`；Rurix 臂
  `sun_color = color_linear_rgb × intensity_lux`，
  `direct = sun_color × max(n·l,0) × albedo/π`——两式同构；UE 侧光色消费
  口径 = **线性直给**（`set_light_color` 第二参 `b_srgb = False`，契约
  字段名 `color_linear_rgb` 即线性域——sRGB 二次转换即口径违例）；链上
  每环节参数 provenance（契约值 / UE 侧应用值 / Rurix 侧应用值）进门
  evidence。
- L4 **残余口径差显式登记**（对齐后结构性残余逐环节分列，登记粒度 =
  逐环节——天光/太阳/曝光/位深/GI 结构面分列，RFC-0028 §9 Q4 倾向
  裁决）：残余项 = 参数化对齐后仍存在的结构面差异（全向 IBL vs 探针
  单反弹覆盖差 / 多反弹缺级 / 灯种子集缺类〔R3 承接面〕/ GI 结构差
  〔R4 承接面〕/ 源位深量化差〔C3 承接面〕等）；登记载体 =
  `milestones/g11/g11_2_residual_caliber_registry.json`（每残余项带环节 /
  场景 / 处置锚〔指向 R3/R4/C3 修复承接面或显式留档〕/ measured 影响量
  〔可测则记〕）；复测差距清单 caliber_diff 面消费本登记（RXS-0391
  schema 面 0-byte 消费）；**残余口径差未登记即 RED**。
- L5 **消费门序**：未对齐口径（L2/L3 任一环节未参数化一致且未登记残余）
  消费复测 delta 即 RED（G11_CONTRACT §4.2 M144 判据字面）；契约参数
  （相机/光照/seed/post）digest 三面绑定 0-byte——对齐不得改契约参数
  （M130 门序字面继承，RXS-0393 L4）。

**Implementation Requirements**

- IR1 本条款挂接 `g11.p0.m144.caliber_c1_indoor_luminance`
  （`ci/g11_caliber_c1_indoor_luminance_smoke.py`，G11.2）；测试锚定 =
  `conformance/visual_comparison/accept/caliber_alignment_minimal.rx` +
  `conformance/visual_comparison/reject/caliber_fitting_masquerade.rx`。
- IR2 天光/太阳链 provenance 机器形态 = 门 evidence `caliber_chain`
  闭集块（逐环节 `{chain, scene_id, contract_value, ue_applied,
  rurix_applied, aligned, residual_note}`）；残余登记机核 =
  `g11_2_residual_caliber_registry.json` 逐环节非空行 + 每行处置锚非空。

### RXS-0393 修复闭环判据：锁定基线锚消费 + 收敛判定两款 + 收敛阈标定程序产 + 契约 digest 0-byte（G11.2~G11.5 共用）

**Legality**

- L1 **锁定基线锚**：每修复/口径行的基线 delta 转引自
  `milestones/g10/g10_gap_registry.json` 对应行 `measured_delta[].delta`
  （0-byte 消费不回写）；G11.1 已转录为 `g11_budget.json`
  `g11.closure_baseline.*` 十一条基线锚（`direction = max`：同 row 重登记
  delta 不得大于本锚——防修复反向恶化冒充）。
- L2 **收敛判定分两款**（RFC-0028 §4.6.2 冻结语义）：
  - **quality_gap 行（R/U 族）**：收敛 = 复测 delta（G11.5 同契约双端
    复跑实测）**向 0 收敛**——|复测 delta| < |基线 delta| 且收敛幅度 ≥
    收敛幅度阈值；方向性注入（修复反向过冲冒充收敛 / 绝对值缩小但双端
    仍实质不一致冒充闭环）即 RED。
  - **caliber_diff 行（C 族）**：闭环 = 口径对齐完成（参数化一致或显式
    互证登记，RXS-0392 面）+ 残余口径差显式登记 + 复测 delta 与登记残余
    一致（口径差行不是「被修没」——残余 delta 全额归属登记残余项，无未
    归因余量）；不以 quality_gap 款收敛字面冒充口径对齐闭环；未对齐口径
    消费复测 delta 即 RED。
- L3 **收敛阈值标定程序产**（禁手写，P-09）：收敛幅度阈值由标定程序对
  修复前后度量数据实测标定产出（p100 × k，k ∈ [1.0, 3.0]——RXS-0389
  L5 / RFC-0026 §4.2 F10 估计器语义同程序纪律），标定值入
  `g11_budget.json`（measured_local，provenance 齐备：样本集 digest /
  标定程序 / k 取值与理由随档）；**收敛阈值缺失（标定未产）时闭环断言
  不成立**——不得以「delta 有变小」叙述冒充收敛判定；手写阈值冒充标定
  即 RED；estimated 冒充 measured 即 RED。
- L4 **契约 digest 0-byte**：修复不得改契约参数（相机/光照/seed/post）
  ——复测契约参数 digest == G10.5 锁定值（cornell
  `sha256:80305791a68ccc66c5b046efaf193244796b52570494cf00aa1c86efa55be118`
  / bistro
  `sha256:ad45951ba641106b24e7d91d49ebf5992fb6a42cb70a3082520e8de19a6cf514`，
  联合
  `sha256:64fd54df6e9be522d6dbb3bec8fac1eb30a0a421c7a5a8185a3452c381178aa4`）；
  锁定值机核事实源 = `evidence/g10_m130_dual_determinism_contract_20260815T233315Z.json`
  （M130 `--phase g10.5` 门实测登记，本条款字面为转引便利）；不等仍出
  报告即 RED（M130/M139 门序硬约束继承，RXS-0384 L5/§4.0 字面）。
- L5 **不设绝对画质通过线**：闭环判据只断言 delta 收敛 measured，不断言
  绝对画质达标——「已达 UE5 画质」判定归 G15 商用收口期（G11_CONTRACT
  §1/§5 字面）。

**Implementation Requirements**

- IR1 本条款挂接 G11.2 口径对齐闭环三门
  （`g11.p0.m144.caliber_c1_indoor_luminance` /
  `g11.p0.m145.caliber_c2_exposure_chain` /
  `g11.p0.m146.caliber_c3_exr_bit_depth`）与 G11.3~G11.5 各修复闭环门
  （G11_ACCEPTANCE_MAP §1 行集 0-byte 引用）；测试锚定 =
  `conformance/visual_comparison/accept/fix_closure_criterion_minimal.rx` +
  `conformance/visual_comparison/reject/closure_handwritten_threshold.rx`。
- IR2 门 evidence 闭环节机器形态（CI_GATES §7 修复闭环节字段闭集
  materialize 硬化面）= `closure = { gap_row_id, baseline_delta,
  retest_delta, converged, threshold_provenance, contract_digest_unchanged }`
  ——`threshold_provenance` 须含标定程序与 budget 条目引用（L3），
  `contract_digest_unchanged` 机核 = 当次契约参数 digest == L4 锁定值。

### RXS-0403 UE Path Tracer 对标口径：对标契约独立冻结与 digest 门序 / 双端同场景同 spp 出图 / 收敛曲线逐段·噪声谱·能量守恒 measured 对拍 / UE PathTracing 模块归属差距登记（M163，G12.4）

**Legality**

- L1 **对标契约独立冻结**：G12.4 UE PT 对标契约 =
  `milestones/g12/g12_ue_pt_parity_contract.json`（schema
  `rurix.g12.ue_pt_parity_contract.v1`），字段闭集：`schema` const /
  `contract_id` / `version`（u32）/ `spp_sequence`（u32 数组，严格递增，
  末档 == `ref_spp`）/ `ref_spp`（u32）/ `max_bounces`（u32）/ `seed`
  （u64）/ `calibration_seed`（u64，≠ `seed`）/ `noise_probe_spp`（u32，
  ∈ `spp_sequence` 且 ≠ `ref_spp`）/ `scenes`（恰二行，场景闭集
  {`cornell-box`, `bistro-interior`}——M133 清单 digest 注册面转引只读
  不回写，RXS-0383 口径）/ `rendering_policy`（`ue_pathtracing` const
  true / `filter_width` f64 / `max_bounces` u32 / `mis_mode` u32 /
  `russian_roulette` bool / `denoiser` const `"off"` / `tonemap` const
  `"off"`）/ `provenance`（**不入 digest preimage**）；逐场景行字段闭集：
  `scene_id` / `m133_manifest_digest` / `gltf_product_digest` / `camera`
  （`position` f64×3 / `orientation_quat` f64×4〔unit-norm 2^-40 谓词
  常量，RXS-0384 L2 口径继承〕/ `fov_y_deg` / `near` / `far` /
  `resolution{w,h}` u32）/ `exposure`（`mode` enum 仅 `"manual"` /
  `ev100` f64）/ `lighting`（`quad_lights[]`{`p00`/`e1`/`e2`/
  `le_linear_rgb` f64×3} / `point_lights[]`{`id` str/`position`/
  `color_linear_rgb` f64×3/`intensity_cd` f64} / `emissive_materials[]`
  {`material_name` str/`material_index` u32/`le_linear_rgb` f64×3/
  `area_m2` f64} / `sun_intensity_lux` f64 / `sky_intensity` f64——后两
  者本契约 = 0.0 显式登记：PT 起步范围无方向光/天光链接口面，RXS-0357
  L1 起步范围冻结维持）/ `material_policy`（`texture_mean_albedo` bool
  / `white_tex_to_white` bool 闭集）。schema 外字段注入即拒
  （fail-closed）；null 禁入。**契约参数独立冻结：不动 G10.5/G11.5b
  锁定值**（G10/G11 closed 复测对照面 0-byte，RXS-0393 L4 锁定值不消
  费不承接）。
- L2 **canonical 字节布局与 digest 门序**：契约 digest =
  SHA-256(canonical preimage)——字节布局 = 版本前缀 ASCII `G12PTP-1`
  + NUL（`47 31 32 50 54 50 2D 31 00`）+ RXS-0384 L3 同构规则（类型
  标签 f64=0x01/u32=0x02/u64=0x03/str=0x04/bool=0x05/obj 起止
  0x07/0x08/arr 起止 0x09/0x0A；键 Unicode code point 升序 + u32
  length-prefix UTF-8；f64 binary64 小端；u32/u64 宽度 schema 驱动禁
  值域分派；NaN/±Inf 禁入）；digest 域 = L1 字段闭集（`provenance`
  块不入）。**三方独立实现 digest 全等机核**：① host python（门脚本
  内嵌解析器）② Rurix Rust harness（`--contract-digest` 面）③ UE 内
  嵌 CPython（harness 解析器）——三值全等且 == 门内冻结注册值（实现
  PR 落盘时实测回填）；**契约 digest 不等仍出报告即 RED**（M130/
  M139/M155 门序硬约束继承，RXS-0384 L5 / RXS-0393 L4 同族）。
- L3 **双端出图**：同场景同 spp 双端出图——UE 臂 = UE 5.8.1 Path
  Tracer MRQ 臂（F:\UE_5.8；**UE build digest == M128 登记 ue_build_id
  机核**〔`5.8.1-56057345`，ci/g10_ue5_lib.py `EXPECTED_UE_BUILD_ID`
  注册面消费〕；窗口模式主路臂，G10-N8/N9 口径继承；MRQ 逐 （场景 ×
  spp） 作业 EXR〔NONE 压缩 + tone curve 关闭捕获点，RXS-0386 L1〕)；
  Rurix 臂 = G12 生产化 PT megakernel device 真跑（固定 seed 双跑位级
  一致确定性协议继承 RXS-0357 L2 / RXS-0400；多灯 workload 面 =
  bistro-interior 4+ 点光 + emissive 表面双端 PT 对拍——RD-040
  M100-high 触发评估法定输入）。**单端缺帧聚合不得 PASS**：任一端任
  一场景任一 spp 档缺帧/非真 EXR/非新鲜出帧，聚合门必须 FAIL（单端
  缺帧聚合 PASS 即 RED）。
- L4 **measured 对拍三面**（容差一律标定程序 measured 产，禁手写
  P-09，入 `g12_budget.json` measured_local；**不设绝对通过线**——
  超容差段显式登记即 RED 评审面，**逐段对拍超容差静默即 RED**）：
  - **收敛曲线逐段对拍**：逐端收敛曲线 rel_err_e(s) = rel-MAE(
    frame_e(s), frame_e(ref_spp))（端内参照，曝光尺度链两端消去）；
    逐段对拍差 = |rel_err_ue(s) − rel_err_rurix(s)|，s ∈ spp_sequence
    ∖ {ref_spp}；超容差段必须有差距登记表对应行。
  - **噪声谱对拍**：`noise_probe_spp` 档残余帧（frame_e(probe) −
    frame_e(ref)）高频能量谱逐端 measured + 双端谱差 measured；超容
    差登记纪律同上。
  - **能量守恒对拍**：ref_spp 档帧均值能量双端相对差 measured（口径
    链对齐后消费——Rurix 帧 ×2^(−ev100) 派生尺度，RXS-0392 C1 口径
    继承）；超容差登记纪律同上。
- L5 **UE PathTracing 模块归属差距登记**：差距登记表落盘
  （`milestones/g12/g12_ue_pt_gap_registry.json`）——差距逐项登记
  UE5 模块归属（`Engine/Source/Runtime/Renderer/Private/PathTracing.cpp`
  及关联模块行集，**RXS-0391 归属枚举闭集口径继承**，只追加演进）；
  差距项显式登记，不冒充全闭环（**差距项静默混入即 RED**）；登记表
  行集与对拍报告**对账**——L4 全部超容差段/谱差/能量差行必须有对应
  登记表行，登记表每行 measured_delta 可溯源（delta == b−a f64 精确
  + evidence_digest 回溯，RXS-0391 口径）。
- L6 **口径对齐先行 + 不设绝对通过线**：曝光/位深口径沿 G11.2 对齐
  口径（RXS-0385 strip-and-log / EV100 派生链互证，RXS-0392 口径继
  承）；残余口径差逐环节显式登记（载体 = 门 evidence
  `residual_caliber_note` + 差距登记表 caliber_diff 行）；**未对齐口
  径消费对拍 delta 即 RED**（R-G12-5 / R-G11-1 同族纪律）；**不设绝
  对通过线**——「已达 UE5 PT 画质」叙述 G12 期内一律不成立（绝对判
  定归 G15 商用收口期，G12_CONTRACT §1/§5 字面）。

**Implementation Requirements**

- IR1 本条款挂接 G12.4 P0 门 `g12.p0.m163.ue_pt_parity`
  （`ci/g12_ue_pt_parity_smoke.py`，G12_CONTRACT §4.2 M163 行 +
  G12_ACCEPTANCE_MAP §1 判据逐字）；测试锚定 =
  `conformance/visual_comparison/accept/ue_pt_parity_contract_minimal.rx`
  + `conformance/visual_comparison/reject/parity_digest_mismatch_report.rx`
  + `conformance/visual_comparison/reject/residual_caliber_silent.rx`。
- IR2 门 evidence 对标节机器形态（G12 CI_GATES §7 对标节字段闭集
  materialize 硬化面）= `parity = { contract_digest, ue_build_id,
  curve_segments, noise_spectrum_delta, energy_conservation_delta,
  gap_registry_file, residual_caliber_note }`——`curve_segments` 逐段
  数组非空（每段 {spp, rel_err_ue, rel_err_rurix, delta, tolerance,
  over_tolerance, registered}）；`residual_caliber_note` 无残余须为
  null 字面；`gap_registry_file` 行集对账机核 = L5。
- IR3 RED 臂独立有效（契约 §4.2 M163 判据字面）：契约 digest 不等仍
  出报告 / 逐段对拍超容差静默 / 差距项静默混入 / 单端缺帧聚合 PASS
  / 残余口径差未登记消费 delta——各臂注入必检出，漏检即 FAIL。

### RXS-0405 UE 超分双端对拍口径：对拍契约独立冻结与 digest 门序 / 双端同场景同档位出图 / SSIM·FLIP·噪声谱 measured 对拍与帧率 zero_pass_line 基线 / UE DLSS·超分模块归属差距登记（M169，G13.4）

**Legality**

- L1 **对拍契约独立冻结**：G13.4 UE 超分对拍契约 =
  `milestones/g13/g13_ue_upscale_parity_contract.json`（schema
  `rurix.g13.ue_upscale_parity_contract.v1`），字段闭集：`schema` const /
  `contract_id` / `version`（u32）/ `tier_sequence`（u32 数组闭集
  `[50, 67, 100]`，渲染比例百分数，严格递增）/ `frame_count`（u32，
  Halton 静态收敛序列帧数 == 32）/ `seed`（u64）/ `calibration_seed`
  （u64，≠ `seed`）/ `noise_probe_tier`（u32，∈ `tier_sequence`）/
  `scenes`（恰二行，场景闭集 {`cornell-box`, `bistro-interior`}——M133
  清单 digest 注册面转引只读不回写，RXS-0383 口径）/ `rurix_backends`
  （字符串数组闭集 `[tsr_device, dlss_sr, fsr_3_1_5]`——M-a/M-b 三
  后端逐一出帧面）/ `ue_dlss_quality_map`（tier → UE DLSS 质量枚举
  映射闭集：`50 → Performance` / `67 → Quality` / `100 → DLAA`）/
  `rendering_policy`（`tonemap` const `"off"` / `denoiser` const `"off"`
  / `ue_temporal_upscaler` const `"dlss_plugin"` / `jitter` const
  `"halton_static"`）/ `provenance`（**不入 digest preimage**）；逐场景
  行字段闭集：`scene_id` / `m133_manifest_digest` / `gltf_product_digest`
  / `camera`（`position` f64×3 / `orientation_quat` f64×4〔unit-norm
  2^-40 谓词常量，RXS-0384 L2 口径继承〕/ `fov_y_deg` / `near` / `far` /
  `resolution{w,h}` u32——输出分辨率，内部分辨率 = 输出 × tier%）/
  `exposure`（`mode` enum 仅 `"manual"` / `ev100` f64）/ `lighting`
  与 `material_policy` 字段面同 RXS-0403 L1 逐场景行闭集转引。schema
  外字段注入即拒（fail-closed）；null 禁入。**契约参数独立冻结：不动
  G10.5/G11.5b/G12.4 锁定值**（G10/G11/G12 closed 复测对照面 0-byte，
  RXS-0393 L4 锁定值不消费不承接）。
- L2 **canonical 字节布局与 digest 门序**：契约 digest =
  SHA-256(canonical preimage)——字节布局 = 版本前缀 ASCII `G13USP-1`
  + NUL（`47 31 33 55 53 50 2D 31 00`）+ RXS-0384 L3 同构规则（类型
  标签/键序/宽度 schema 驱动/NaN·±Inf 禁入全口径继承）；digest 域 =
  L1 字段闭集（`provenance` 块不入）。**三方独立实现 digest 全等机
  核**：① host python（门脚本内嵌解析器）② Rurix Rust harness
  （`--contract-digest` 面）③ UE 内嵌 CPython（harness 解析器）——三
  值全等且 == 门内冻结注册值（实现 PR 落盘时实测回填）；**契约
  digest 不等仍出报告即 RED**（M130/M139/M155/M163 门序硬约束继承，
  RXS-0384 L5 / RXS-0403 L2 同族）。
- L3 **双端出图**：同场景同档位双端出图——UE 臂 = UE 5.8.1 DLSS 插
  件 MRQ 臂（F:\UE_5.8；**UE build digest == M128 登记 ue_build_id
  机核**〔ci/g10_ue5_lib.py `EXPECTED_UE_BUILD_ID` 注册面消费〕；
  `MoviePipelineDLSSSetting` 逐档注入 MRQ PrimaryConfig，
  `DLSSQuality` ∈ `ue_dlss_quality_map` 值域闭集；EXR〔NONE 压缩 +
  tone curve 关闭捕获点，RXS-0386 L1〕)；Rurix 臂 = M-a vendor 超分面
  （DLSS SR 经 Streamline Vulkan interop 臂 + FSR 3.1.5）+ M-b 自研
  TSR device 面（.rx kernel SPV），经 UpscaleBackend 冻结接口面逐后
  端出帧（**trait 签名面与 temporal 底座 0-byte**，G13 裁决 6 字面）；
  逐（场景 × 档位 × 后端）内部分辨率 = 输出分辨率 × tier%，Halton
  jitter 32 帧静态收敛序列，固定 seed 位级一致确定性协议继承
  RXS-0357 L2 / RXS-0400。**单端缺帧聚合不得 PASS**：任一端任一场景
  任一档位（Rurix 臂任一后端）缺帧/非真 EXR/非新鲜出帧，聚合门必须
  FAIL（单端缺帧聚合 PASS 即 RED）。
- L4 **measured 对拍三面**（容差一律标定程序 measured 产，禁手写
  P-09，入 `g13_budget.json` measured_local；**不设绝对通过线**——
  超容差项显式登记即 RED 评审面，**超容差静默即 RED**）：
  - **SSIM/FLIP 逐格对拍**：逐（场景 × 档位 × 后端）收敛末帧 LDR 派
    生域 SSIM/FLIP 双端 measured（RXS-0387/RXS-0388 口径继承，LDR
    派生链 RXS-0386 L2）+ 双端度量差 measured。
  - **噪声谱对拍**：`noise_probe_tier` 档逐端残余帧（逐帧 − 32 帧均
    值收敛参照）亮度 2D FFT 高频能量份额（径向 |f|>Nyquist/4 带，
    RXS-0403 L4 噪声谱口径继承）逐端 measured + 双端谱差 measured。
  - **帧率 measured 基线登记 zero_pass_line**：逐（场景 × 档位）双端
    单帧渲染耗时 measured 登记（UE 臂 = MRQ 出帧耗时面；Rurix 臂 =
    50×3 trimmed mean 统计口径 M141/M165 字面继承），**不设通过
    线**——正式帧率对标锚定 G14（G10-N11/N16 承接锚字面 0-byte）；
    **以基线冒充帧率对标即 RED**。
- L5 **UE DLSS·超分模块归属差距登记**：差距登记表落盘
  （`milestones/g13/g13_ue_upscale_gap_registry.json`）——差距逐项登
  记 UE5 模块归属（`Engine/Source/Runtime/Renderer/Private/` 超分相
  关模块行集 + DLSS/Streamline 插件模块行集，**RXS-0391 归属枚举闭
  集口径继承**，插件模块归属只追加演进不改写既有枚举；Other 终值须
  attribution_note 非空）；差距项显式登记，不冒充全闭环（**差距项静
  默混入即 RED**）；登记表行集与对拍报告**对账**——L4 全部超容差
  项必有对应登记表行，每行 measured_delta 可溯源（delta == b−a f64
  精确 + evidence_digest 回溯，RXS-0391 口径）。
- L6 **口径对齐先行 + 不设绝对通过线**：曝光/位深口径沿 G11.2/G12.4
  对齐口径（RXS-0385/RXS-0392 继承）；残余口径差逐环节显式登记（载
  体 = 门 evidence `residual_caliber_note` + 差距登记表 caliber_diff
  行）；**未对齐口径消费对拍 delta 即 RED**；**不设绝对「已达 UE5
  DLSS/超分画质」通过线**——绝对判定归 G15 商用收口期（G13_CONTRACT
  §1/§5 字面，RXS-0403 L6 同族）。

**Implementation Requirements**

- IR1 本条款挂接 G13.4 P0 门 `g13.p0.m_c.ue_upscale_parity`
  （`ci/g13_ue_upscale_parity_smoke.py`，G13_CONTRACT §4.2 M-c 行 +
  G13_ACCEPTANCE_MAP §1 判据逐字）；测试锚定 =
  `conformance/visual_comparison/accept/ue_upscale_parity_contract_minimal.rx`
  + `conformance/visual_comparison/reject/upscale_parity_digest_mismatch_report.rx`
  + `conformance/visual_comparison/reject/upscale_fps_baseline_masquerade.rx`。
- IR2 门 evidence 对拍节机器形态 = `parity = { contract_digest,
  ue_build_id, cells, noise_spectrum_delta, fps_baseline,
  gap_registry_file, residual_caliber_note }`——`cells` 逐（场景 × 档
  位 × 后端）数组非空（每格 {scene, tier, backend, ssim_ue,
  ssim_rurix, flip_ue, flip_rurix, delta, tolerance, over_tolerance,
  registered}）；`fps_baseline` 逐（场景 × 档位）双端 measured 登记
  且 `zero_pass_line` const true；`residual_caliber_note` 无残余须为
  null 字面；`gap_registry_file` 行集对账机核 = L5。
- IR3 RED 臂独立有效（契约 §4.2 M-c 判据字面）：契约 digest 不等仍
  出报告 / 超容差静默 / 差距项静默混入 / 单端缺帧聚合 PASS / 帧率
  基线冒充帧率对标——各臂注入必检出，漏检即 FAIL。

### RXS-0406 UE Lumen GI 对照口径：对照契约独立冻结与 digest 门序 / 双端同场景 GI 出图 / GI 能量·间接光 measured 对拍 / UE Lumen 模块归属差距登记与 G11 GI 面 0-byte（M170，G13.4）

**Legality**

- L1 **对照契约独立冻结**：G13.4 UE Lumen GI 对照契约 =
  `milestones/g13/g13_ue_lumen_gi_parity_contract.json`（schema
  `rurix.g13.ue_lumen_gi_parity_contract.v1`），字段闭集：`schema`
  const / `contract_id` / `version`（u32）/ `seed`（u64）/
  `calibration_seed`（u64，≠ `seed`）/ `scenes`（恰二行，场景闭集
  {`cornell-box`, `bistro-interior`}——M133 清单 digest 转引只读，
  RXS-0383 口径；逐场景行字段闭集同 RXS-0405 L1 逐场景行转引）/
  `rendering_policy`（`ue_gi_method` const `"lumen"` /
  `ue_reflection_method` const `"lumen"` / `tonemap` const `"off"` /
  `denoiser` const `"off"` / `indirect_derivation` const
  `"gi_on_minus_gi_off"`——间接光贡献项 = 同场景同参数 GI 开帧 − GI 关
  帧逐像素差双端同构派生面）/ `rurix_gi_surface`（三锚闭集
  {`screen_probe_near_field`, `world_cache_far_field`,
  `multibounce_chain`}——M98/M99/M154 已验收面锚定只消费，G9.4/
  G11.4 evidence digest 注册面转引）/ `provenance`（**不入 digest
  preimage**）。schema 外字段注入即拒（fail-closed）；null 禁入。
  **契约参数独立冻结：不动 G9.4/G10.5/G11.4/G11.5b 锁定值**
  （RXS-0393 L4 口径继承）。
- L2 **canonical 字节布局与 digest 门序**：契约 digest =
  SHA-256(canonical preimage)——字节布局 = 版本前缀 ASCII `G13LGP-1`
  + NUL（`47 31 33 4C 47 50 2D 31 00`）+ RXS-0384 L3 同构规则全口径
  继承；**三方独立实现 digest 全等机核**（host python / Rurix Rust
  harness `--contract-digest` / UE 内嵌 CPython）三值全等且 == 门内
  冻结注册值（实现 PR 落盘时实测回填）；**契约 digest 不等仍出报告
  即 RED**（RXS-0403 L2 / RXS-0405 L2 同族）。
- L3 **双端出图**：同场景双端 GI 出图——UE 臂 = UE 5.8.1 deferred
  管线 + Lumen GI MRQ 臂（`r.DynamicGlobalIlluminationMethod=1` 等
  Lumen 设置面经 MRQ ConsoleVariableSetting 注入；**UE build digest
  == M128 登记 ue_build_id 机核**；EXR〔NONE 压缩 + tone curve 关闭，
  RXS-0386 L1〕)；Rurix 臂 = M98 屏幕探针近场 + M99 世界辐射缓存远
  场 + M154 多反弹链 GPU GI 面（G9.4/G11.4 已验收面**只消费不改写**
  ——既有判据 0-byte 机核，GI 实现面目录级 diff 对账）。**单端缺帧
  聚合不得 PASS**：任一端任一场景缺帧/非真 EXR/非新鲜出帧，聚合门
  必须 FAIL（单端缺帧聚合 PASS 即 RED）。
- L4 **measured 对拍两面**（容差一律标定程序 measured 产，禁手写
  P-09，入 `g13_budget.json` measured_local；**不设绝对通过线**——
  超容差项显式登记即 RED 评审面，**超容差静默即 RED**）：
  - **GI 能量对拍**：逐场景帧均值能量双端相对差 measured（口径链对
    齐后消费——曝光派生尺度链 RXS-0392 C1 / RXS-0403 L4 能量守恒口
    径继承）。
  - **间接光对拍**：逐场景间接光辐照面（GI 贡献项）LDR 派生域
    SSIM/FLIP 双端 measured（RXS-0387/RXS-0388 口径继承）+ 双端度
    量差 measured。
- L5 **UE Lumen 模块归属差距登记**：差距登记表落盘
  （`milestones/g13/g13_ue_lumen_gap_registry.json`）——差距逐项登
  记 UE5 模块归属（`Engine/Source/Runtime/Renderer/Private/Lumen/`
  模块行集，**RXS-0391 归属枚举闭集口径继承**，只追加演进）；差距
  项显式登记不冒充全闭环（**Lumen 差距项静默混入即 RED**）；登记表
  行集与对拍报告对账（L4 全部超容差项必有对应行 + measured_delta
  可溯源，RXS-0391 口径）。
- L6 **G11 GI 面 0-byte + 不设绝对通过线**：G11 GI 面既有判据
  0-byte（**GI 既有门降级即 RED**）；残余口径差逐环节显式登记（载
  体 = 门 evidence `residual_caliber_note` + 差距登记表 caliber_diff
  行；未对齐口径消费 delta 即 RED）；**不设绝对「已达 UE5 Lumen
  画质」通过线**——绝对判定归 G15 商用收口期（G13_CONTRACT §1/§5
  字面）。

**Implementation Requirements**

- IR1 本条款挂接 G13.4 P0 门 `g13.p0.m_d.ue_lumen_gi_parity`
  （`ci/g13_ue_lumen_gi_parity_smoke.py`，G13_CONTRACT §4.2 M-d 行 +
  G13_ACCEPTANCE_MAP §1 判据逐字）；测试锚定 =
  `conformance/visual_comparison/accept/ue_lumen_gi_parity_contract_minimal.rx`
  + `conformance/visual_comparison/reject/lumen_parity_digest_mismatch_report.rx`
  + `conformance/visual_comparison/reject/lumen_gap_silent.rx`。
- IR2 门 evidence 对照节机器形态 = `parity = { contract_digest,
  ue_build_id, cells, gap_registry_file, residual_caliber_note }`——
  `cells` 逐场景数组非空（每格 {scene, energy_ue, energy_rurix,
  energy_delta, indirect_ssim, indirect_flip, tolerance,
  over_tolerance, registered}）；`residual_caliber_note` 无残余须为
  null 字面；`gap_registry_file` 行集对账机核 = L5。
- IR3 RED 臂独立有效（契约 §4.2 M-d 判据字面）：契约 digest 不等仍
  出报告 / 超容差静默 / Lumen 差距项静默混入 / 单端缺帧聚合 PASS /
  GI 既有门降级——各臂注入必检出，漏检即 FAIL。

### RXS-0407 绝对画质通过线口径：UE 参照 deficit 双 seed 方差底 p100×2.0 程序产标定 / 18 格逐格判定与 AI 读图强制 / 商用收口诚实定盘（G15 M-c，G15.4）

**Legality**

- L1 **适用面与不 retroactive**：本条款冻结 G15 商用收口期绝对画质
  通过线口径，**唯一挂接面 = G15.4 M-c 终审门**（G15_CONTRACT §4.2
  M-c 行 / G-G15-5）。RXS-0403 L6 / RXS-0405 L6 / RXS-0406 L6「不设
  绝对通过线」字面 **0-byte 维持**——G13/G14 closed 门判据语义不
  回写、不重审、不 retroactive 改写；本条款只前向适用于 G15 终审面
  （G15_CONTRACT guardrails 绝对画质通过线纪律字面）。G13.4 起
  端内参照 deficit delta 容差带面（RXS-0405 L4）与本条款跨端绝对
  deficit 通过线面 = **两套口径并存、各自登记、互不冒充**（RFC-0026
  §4.3 0-byte 声明同律）。
- L2 **判定对象与参照帧**：判定矩阵 = 场景闭集 {`cornell-box`,
  `bistro-interior`}（M133 清单 digest 转引只读，RXS-0383 口径）×
  档位闭集 {t50, t67, t100} × 后端闭集 {`tsr_device`, `dlss_sr`,
  `fsr_3_1_5`} = **18 格** Rurix **生产管线**出图（生产车道
  `--render` 32 帧 Halton jitter 静态收敛序列末帧 converged.exr，
  `RURIX_REQUIRE_REAL=1` + validation 零错误 + GPU 锁纪律；mock /
  host 替代 / 人工截图充数即 RED）。**UE 参照帧 = G15.2 M-a 复跑产出
  的 UE 臂同场景同档帧列末帧**：新鲜度机核 = receipt
  `started_epoch` ≥ M-a 波启动锚 ∧ 抽帧 canonical digest 重算 ==
  receipt 登记值——**陈旧参照注入即 RED**；参照内容有效性机核 =
  失败模式字面编码（HDR 亮度 max ≤ 1e-3 = 死黑退化面）——**参照
  退化格不得冒充达标亦不得静默消费**：显式登记 finding + 该格判定
  面如实标注参照退化态（digest 面不替代内容面，G14.10f 教训字面
  兑现）。
- L3 **度量域（RXS-0386 字面维持）**：度量唯一域 = display-referred
  LDR 臂，双端同一派生链单源——UE 帧（MRQ 管线内 ev100 手动曝光
  已施 + tone curve 关闭捕获点，RXS-0386 L1）派生尺度 = 1.0；Rurix
  生产出图（全后端管线内 ×2^(−ev100) 显示域转换已施，receipt
  `exposure == 2^(−ev100)` 机核）派生尺度 = 1.0；双端经同一 aces13
  view transform + 同一 host 侧 sRGB 编码步骤（RXS-0386 L2 单源）
  产 LDR 帧。**scene-linear 域直比 = G15-MA-F1 caliber 已登记面
  （RXS-0392 不拟合），不得混入本条款度量——混入即 RED**；HDR 帧
  直算 SSIM/FLIP 即口径混用 RED（RXS-0387 L1 / RXS-0389 L2 继承，
  度量实现 fail-closed）。
- L4 **绝对阈程序产标定（禁手写 P-09）**：逐格双度量 deficit——
  `deficit_ssim = 1 − SSIM(rurix_ldr, ue_ldr)`（Wang 2004 闭集，
  RXS-0387 L2）；`deficit_flip = FLIP_LDR(ref=ue_ldr, tst=rurix_ldr)`
  （RXS-0389 闭集）。绝对阈 **T(scene, metric) = 双 seed 标定腿方差
  底 p100 × 2.0 程序产**（沿 G13.4 标定三条目范式）：标定腿 = 18 格
  逐格双 seed（契约 `seed` vs `calibration_seed`）生产渲染，逐格逐
  度量取 `|deficit_main − deficit_calibration|` 为方差样本，场景内
  九格取 p100（= max），阈 = p100 × 2.0（冻结安全系数 k = 2.0，
  RXS-0387 L4 / RXS-0389 k ∈ [1.0, 3.0] 面内）；标定四条目（2 场景
  × 2 度量）入 `g15_budget.json` `measured_local` 零 estimated，
  标定链路全要素（双 seed 帧 digest、逐格 deficit、方差样本集、
  参数面）入 evidence，标定程序可复跑。**标定腿双跑核验**：同 seed
  双跑 converged_digest 位级一致（RXS-0357 固定 seed 位级确定性
  协议继承）+ 标定值自在档帧面重算 f64 精确相等——不等即 RED；
  **手写 / estimated 阈值冒充标定即 RED**（RXS-0393 L3 同族）。
- L5 **18 格逐格判定与 AI 读图强制**：逐格 verdict =
  `deficit_ssim ≤ T(scene, ssim)` ∧ `deficit_flip ≤ T(scene, flip)`
  ∧ **AI 读图 PASS**，逐格判定逐字入 evidence。AI 读图 = 逐格 PNG
  导出 + 逐格审查记录（18 格闭集零空行）：无乱序 / 无错位 / 无全黑 /
  关键结构可见（cornell 盒体结构〔左绿墙/右红墙/白后墙/顶部面光/
  双箱〕、bistro 吊灯群/吧台/桌椅/墙板）+ 斑块伪影有无 + 暗部态
  诚实区分（「暗但结构在」与「死黑无内容」分列——bistro 夜景固有
  暗态与无 GI 直接光口径边界态如实登记不冒充清晰）+ 三后端互一致
  性；读图记录与导出 PNG digest 逐格绑定（读图对象机核）；**读图
  记录缺格即 RED**；digest 双跑一致不替代内容审查（G14.10f 字面）。
- L6 **商用收口判定（诚实定盘）**：格达标 = L5 三条件全立；商用
  收口判定 = 达标格数 x/18 如实定盘——x = 18 → 「达标」；x < 18 →
  「未达标」如实登记不冒充 + 未达格逐格归因 + G16+ 承接锚字面
  （用户 2026-08-19 授权面「允许在G15后无限制新建里程碑继续优化」
  逐字承接）。**未达格报达标即 RED（判定冒充）**；判定结论以
  measured 面为准，不得受期望影响；判定面新发现缺陷显式登记
  G15-MC-F<n> 进 G15 处置面（法定来源唯一纪律），不得静默放过。

**Implementation Requirements**

- IR1 本条款挂接 G15.4 P0 门 `g15.p0.m_c.absolute_quality_final_review`
  （`ci/g15_absolute_quality_review_smoke.py`，G15_CONTRACT §4.2 M-c
  行 + G-G15-5 + G15_ACCEPTANCE_MAP §1 M-c 行判据逐字）；测试锚定 =
  `conformance/visual_comparison/accept/absolute_pass_line_minimal.rx`
  + `conformance/visual_comparison/reject/absolute_pass_handwritten_threshold.rx`
  + `conformance/visual_comparison/reject/absolute_pass_verdict_masquerade.rx`。
- IR2 门 evidence 终审节机器形态 = `parity = { wave_anchor,
  ue_reference_status, calibration, cells, ai_reading_manifest,
  findings, commercial_closure }`——`cells` 18 格数组非空（每格
  {cell, scene, tier, backend, ssim_deficit, flip_deficit,
  threshold_ssim, threshold_flip, metric_pass, ai_verdict,
  reference_state, verdict, attribution}）；`calibration` 逐场景逐
  度量 {measured_p100, threshold, variance_samples, budget_entry_id,
  evidence_file}；`commercial_closure` = {verdict ∈ {"达标",
  "未达标"}, met_count, total, unmet_cells, g16_anchor}。
- IR3 RED 臂独立有效（契约 §4.2 M-c 判据字面）：标定阈手写注入 /
  读图记录缺格 / 判定冒充（未达格报达标）/ 标定腿单跑无双跑 /
  UE 参照帧陈旧注入——各臂注入必检出，漏检即 FAIL。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.8 | 2026-08-23 | G15.4 绝对画质终审波 spec-first（硬规则 7 条款先行；G15 已解锁 implementation_status=unlocked，G15_CONTRACT §8.1 G-G15-2 互锁 READY + §8.2/§8.3 G-G15-3/G-G15-4 兑现）：**RXS-0407 单号 materialize 为条款头**——绝对画质通过线口径（适用面与不 retroactive〔唯一挂接面 = G15.4 M-c 终审门，RXS-0403/0405/0406 L6「不设绝对通过线」字面 0-byte 维持，端内参照容差带面与跨端绝对通过线面两套口径并存互不冒充〕+ 判定对象与参照帧〔双场景 × 三档 × 三后端 18 格 Rurix 生产管线 --render 真跑出图 + UE 参照帧 = G15.2 M-a 复跑产出 UE 臂同场景同档帧〔新鲜度机核 + 抽帧 digest 重算对账，陈旧注入即 RED〕+ 参照内容有效性失败模式字面编码〔死黑退化面显式登记不冒充不静默〕〕+ 度量域 RXS-0386 字面维持〔display-referred LDR 臂双端同一派生链单源，双端派生尺度均 1.0——UE 帧管线内 ev100 曝光已施 / Rurix 生产出图全后端管线内 ×2^(−ev100) 已施〔receipt exposure 机核〕；scene-linear 域直比 = G15-MA-F1 caliber 已登记面不得混入，混入即 RED〕+ 绝对阈程序产标定〔UE 参照 deficit 双 seed 方差底 p100 × 2.0 沿 G13.4 标定三条目范式，禁手写 P-09，四条目入 g15_budget measured_local，标定链路全要素入 evidence，双跑位级 + 重算 f64 精确核验，手写冒充即 RED〕+ 18 格逐格判定与 AI 读图强制〔逐格 verdict 逐字入 evidence + 读图记录 18 格闭集零空行与 PNG digest 逐格绑定，缺格即 RED，digest 面不替代内容面〕+ 商用收口诚实定盘〔达标格数 x/18 如实定盘，未达格如实登记不冒充 + 逐格归因 + G16+ 承接锚字面，未达格报达标即 RED，新发现显式登记 G15-MC-F<n>〕）。判档 = **加性 spec 条款**（G15_CONTRACT front matter rfc_required 触发面逐条未命中——RXS-0386~0393 锁定度量口径面/RXS-0357 面/UpscaleBackend trait 签名面/temporal 底座面/G13 双表 G12 单表终态/G12~G14 既有门判据语义全部 0-byte 不触，本条款 = G15 新设通过线唯一面 M-c 的口径登记，语义事实源 = G15_CONTRACT §4.2 M-c 行判据逐字 + G15_ACCEPTANCE_MAP §3.4 口径面 + RXS-0386/0387/0389/0392/0393/0403/0405 口径继承转引，条款只登记不加语义；零 RFC 消费，实测 RFC next_free=31 维持）。条款号自落盘前实测 `RXS.next_free=407` 顺位领取（0407 单号不跳号，0295/0296 burned 与 shadow_reserved 181~184 维持）。零新 RX 码；零新 U/RD/SG；conformance 锚定语料三件同 PR 落（accept absolute_pass_line_minimal.rx + reject absolute_pass_handwritten_threshold.rx + reject absolute_pass_verdict_masquerade.rx；inert + `//@ spec` 锚定 + 预期 RED 注释 + 转正路径旁注，G9.2~G13.4 spec 波先例）；trace_matrix 重生成（388→389 全锚定）；stable 快照因条款计数 388→389 同 PR 重 bless（RXS-0180 L2 加性演进，error_codes/editions/subcommands 三段 0 变化）。既有 spec 条款字面 0-byte（只追加新条款/修订记录行），不触红线/禁区。`Assisted-by: Kimi-K3（G15.4 绝对画质终审波）` | **加性条款**（G15.4 波新设通过线口径 spec 面登记；零 RFC 触发面） |
| v1.7 | 2026-08-19 | G13.4 UE 对拍波 spec-first（硬规则 7 条款先行；G13 已解锁 implementation_status=unlocked，G13_CONTRACT §8.2 G-G13-3 互锁 READY）：**RXS-0405 / RXS-0406 双号 materialize 为条款头**——RXS-0405 UE 超分双端对拍口径（对拍契约独立冻结〔schema `rurix.g13.ue_upscale_parity_contract.v1` 字段闭集 + tier_sequence=[50,67,100] ↔ ue_dlss_quality_map {50:Performance, 67:Quality, 100:DLAA} 名义档映射 + rurix_backends=[tsr_device, dlss_sr, fsr_3_1_5] M-a/M-b 三后端面 + 场景闭集 M133 清单 digest 转引 + 不动 G10.5/G11.5b/G12.4 锁定值〕+ canonical 字节布局与 digest 门序〔版本前缀 `G13USP-1\0` + RXS-0384 L3 同构 + 三方独立实现 digest 全等机核 + 不等仍出报告即 RED〕+ 双端同场景同档位出图〔UE 5.8.1 DLSS 插件 MRQ 臂 MoviePipelineDLSSSetting 逐档注入 + ue_build_id == M128 机核 + Rurix 超分面经 UpscaleBackend 冻结接口逐后端出帧 + 单端缺帧聚合不得 PASS〕+ measured 对拍三面〔SSIM/FLIP 逐格 LDR 派生域 + 噪声谱 + 帧率 measured 基线 zero_pass_line 不设通过线锚定 G14〕+ UE DLSS·超分模块归属差距登记 + 不设绝对通过线归 G15）；RXS-0406 UE Lumen GI 对照口径（对照契约独立冻结〔schema `rurix.g13.ue_lumen_gi_parity_contract.v1` 字段闭集 + rurix_gi_surface 三锚闭集 M98/M99/M154 已验收面只消费 + indirect_derivation=gi_on_minus_gi_off 双端同构派生 + 不动 G9.4/G10.5/G11.4/G11.5b 锁定值〕+ digest 门序〔`G13LGP-1\0` 前缀同构〕+ 双端同场景 GI 出图〔UE 5.8.1 deferred + Lumen GI MRQ 臂 + Rurix GPU GI 面只消费不改写 + 单端缺帧聚合不得 PASS〕+ GI 能量·间接光 measured 对拍 + UE Lumen 模块归属差距登记 + G11 GI 面既有判据 0-byte + 不设绝对通过线归 G15）。判档 = **加性 spec 条款**（G13_CONTRACT §7 裁决 4 Full RFC 触发面——UpscaleBackend trait 签名面/temporal 底座历史接口面/RXS-0357 参照器面/M137 scalars.flip 演进位——逐条未命中：双条款零冻结面消费，语义事实源 = G13_CONTRACT §4.2 M-c/M-d 行判据逐字 + RXS-0384/0386/0387/0388/0391/0392/0403 口径继承转引，条款只登记不加语义）。条款号自落盘前实测 `RXS.next_free=405` 顺位领取（0405/0406 双号连续不跳号，0295/0296 burned 与 shadow_reserved 181~184 维持）。零新 RX 码；零新 U/RD/SG；零 RFC 消费（RFC 命名空间 0-byte，实测 next_free=30 维持）；conformance 锚定语料六件同 PR 落（accept ue_upscale_parity_contract_minimal.rx + ue_lumen_gi_parity_contract_minimal.rx；reject upscale_parity_digest_mismatch_report.rx + upscale_fps_baseline_masquerade.rx + lumen_parity_digest_mismatch_report.rx + lumen_gap_silent.rx；inert + `//@ spec` 锚定 + 预期 RED 注释 + 转正路径旁注，G9.2~G13.3 spec 波先例）；symbolic key `g13.p0.m_c.ue_upscale_parity` / `g13.p0.m_d.ue_lumen_gi_parity`（G13.1 冻结字面，G13_ACCEPTANCE_MAP §1）0-byte 不动；trace_matrix 重生成（386→388 全锚定）；stable 快照因条款计数 386→388 同 PR 重 bless（RXS-0180 L2 加性演进，error_codes/editions/subcommands 三段 0 变化）。既有 spec 条款字面 0-byte（只追加新条款/修订记录行），不触红线/禁区。`Assisted-by: Kimi-K3（G13.4 UE 对拍波）` | **加性条款**（G13.4 波冻结判据 spec 面登记；零 RFC 触发面） |
| v1.1 | 2026-08-15 | **errata（RXS-0384 L2 四元数共轭公式勘误；零既有字面改写，本行 = 唯一生效勘误）**：L2 冻结公式行「旋转四元数向量部经同一 M 变换、标量部不变（相似变换 R_ue = M·R·M⁻¹，**转角保持**）」对 det(M) = −1 的反射矩阵 M **数学上不成立**——正交共轭的一般律为 R_ue = M·R(axis, θ)·M⁻¹ = **R(M·axis, det(M)·θ)**，det(M) = −1 时转角反号：**R_ue = R(M·axis, −θ)**，四元数向量部应为 **−M·v**、标量部不变，即 q = (w, x, y, z) ⇒ **q_ue = (w, z, −x, −y)**（harness 缺陷实现 (w, −z, x, y) = R(M·axis, +θ) 为镜像朝向）。**实证**（G10.5a 波，2026-08-15）：共轭恒等式 R(q_ue)·(M·v) == M·(R(q)·v) 随机对拍——缺陷式最大偏差 6.35e0（2000 组）/ 1.39e0（pytest 5000 组首例），修订式偏差 0.0；黄金个案（契约绕 +Y 转 +90° ⇒ 正确 UE 映射 = 绕 +Z 转 −90°，缺陷式给 +90°）镜像成立；`tests/test_g10_param_contract.py` RED 先行 commit 后修复转 GREEN。cornell-box 相机（绕 +Y 180°）为该缺陷不变量特例（R(a,180°) ≡ R(a,−180°)），bistro-interior 一般旋转取景全暴露。**生效面**：harness `g10_param_contract.py quat_contract_to_ue` 按修订式修复（G10.5a 实现批）；L2 既有字面 0-byte 不回改，RFC-0026 §4.6 同文理勘误 = RFC 章 E1 errata 段（只追加）。`Assisted-by: Kimi-K3（G10.5a 波续）` | **Full RFC**（RFC-0026 errata） |
| v1.2 | 2026-08-15 | G10.5a 双端出图波 spec-first（硬规则 7 条款先行）：**RXS-0390 单号 materialize 为条款头**——应用层探针（冻结标志物集〔cornell-box 后墙五点毫米数值面 / bistro-interior 相机系合成五点米，逐值字面冻结〕+ 双端各自管线投影像素一致性断言〔`pixel_delta ≤ 1e-3 px` 合法性谓词常量不走 budget〕+ M130 `--phase g10.5` 与 M139 evidence `application_probes[]` 机核面 + UE 端内嵌 CPython 载体纪律），依据 RFC-0026（Agent Approved 2026-08-15）§4.6 应用层探针末节（「标志物世界坐标集进 spec 条款」兑现点）+ G10_ACCEPTANCE_MAP §1 M130 行 + §3.3（判据逐字）；条款号自落盘前实测 `RXS.next_free=390` 顺位领取（0390 单号，0295/0296 burned 与 shadow_reserved 181~184 维持）；conformance 锚定 accept 一件（application_probe_minimal.rx）同 PR 落；trace_matrix 371→372 全锚定 + stable 快照 371→372 重 bless（RXS-0180 L2 加性演进，error_codes/editions/subcommands 三段 0 变化）。`Assisted-by: Kimi-K3（G10.5a 波续）` | **Full RFC**（RFC-0026） |
| v1.3 | 2026-08-15 | G10.5b 首轮 A/B 对比波 B 段 spec-first（硬规则 7 条款先行）：**RXS-0391 单号 materialize 为条款头**——差距清单 schema（顶层/差距项字段闭集〔13 键 + attribution_note 条件可选键〕+ gap_id 派生冻结字节规则〔sha256 五节 0x00 分隔 utf8 拼接前 16 hex〕+ kind 两值分列〔quality_gap / caliber_diff〕+ UE5 模块归属枚举闭集〔目录级 23 + 文件级 57 + Other 终值，公共前缀 `Engine/Source/Runtime/Renderer/Private/`，Other 须 attribution_note 非空〕+ measured_delta 可溯源〔≥1 项、delta == b−a f64 精确、evidence_digest 须回溯 M137/M139 evidence 登记 artifact digest，纯叙述无测量即 RED〕+ 建议 P 级三值 + g11_anchor 非空 + 场景全集零空行对账〔scene_summary 全等 + no_gap_explicit 显式 + not_ready_scenes 显式在列〕+ domain 两值互证），依据 RFC-0026（Agent Approved 2026-08-15）§4.5 + §3.3 + G10_ACCEPTANCE_MAP §1 M140 行（判据逐字）+ G10_CONTRACT G-G10-7；条款号自落盘前实测 `RXS.next_free=391` 顺位领取（0391 单号，0295/0296 burned 与 shadow_reserved 181~184 维持）；conformance 锚定 accept 一件 + reject 两件（gap_registry_minimal.rx / gap_registry_missing_attribution.rx / gap_registry_unmeasured_narrative.rx）同 PR 落；trace_matrix 372→373 全锚定 + stable 快照 372→373 重 bless（RXS-0180 L2 加性演进，error_codes/editions/subcommands 三段 0 变化）。`Assisted-by: Kimi-K3（G10.5b 波）` | **Full RFC**（RFC-0026） |
| v1.4 | 2026-08-16 | G11.2 口径差对齐波 spec-first（硬规则 7 条款先行）：**RXS-0392 单号 materialize 为条款头**——C1 口径对齐（不拟合原则 + 天光链参数集枚举闭集〔天光模式/强度同单位链/色温或 cubemap 资产 digest/采样档位，cubemap 资产 M131 白名单联动 + 白色常量 cubemap digest 与逐像素值核验〕+ 太阳 lux→辐射度链〔UE DirectionalLight lux → L=ρ·E·(n·l)/π 与 Rurix sun_color=rgb·lux → direct=·ndl·albedo/π 同构登记；UE 侧光色线性直给口径 b_srgb=False〕+ 残余口径差显式登记〔逐环节粒度，载体 milestones/g11/g11_2_residual_caliber_registry.json，未登记即 RED〕+ 消费门序〔未对齐口径消费复测 delta 即 RED；契约 digest 三面绑定 0-byte〕），依据 RFC-0028（Agent Approved 2026-08-16）§4.5 + G11_CONTRACT §4.2 M144 行（判据逐字）+ G11_ACCEPTANCE_MAP §1 M144 行；条款号自落盘前实测 `RXS.next_free=392` 顺位领取（0392 单号，0295/0296 burned 与 shadow_reserved 181~184 维持）；conformance 锚定 accept 一件 + reject 一件（caliber_alignment_minimal.rx / caliber_fitting_masquerade.rx）同 PR 落；trace_matrix 373→374 全锚定 + stable 快照 373→374 重 bless（RXS-0180 L2 加性演进，error_codes/editions/subcommands 三段 0 变化）。`Assisted-by: Kimi-K3（G11.2 波）` | **Full RFC**（RFC-0028） |
| v1.5 | 2026-08-16 | G11.2 口径差对齐波 spec-first：**RXS-0393 单号 materialize 为条款头**——修复闭环判据（锁定基线锚消费〔g10_gap_registry 0-byte 转引 + g11.closure_baseline.* direction=max 十一条基线锚〕+ 收敛判定两款〔quality_gap 行 delta 向 0 收敛且幅度 ≥ 标定阈，方向性注入 RED；caliber_diff 行 = 口径对齐完成 + 残余显式登记 + 复测 delta 与登记残余一致〕+ 收敛阈标定程序产〔p100 × k，k∈[1,3]，入 g11_budget measured_local，标定缺失时闭环断言不成立，手写/estimated 冒充 RED〕+ 契约 digest 0-byte〔锁定值机核事实源 = evidence/g10_m130_dual_determinism_contract_20260815T233315Z.json，不等仍出报告即 RED〕+ 不设绝对画质通过线〔归 G15〕），依据 RFC-0028（Agent Approved 2026-08-16）§4.6 + G11_CONTRACT §4.2/§5（判据字面）+ G11_ACCEPTANCE_MAP §1；条款号自落盘前实测 `RXS.next_free=393` 顺位领取（0393 单号，0295/0296 burned 与 shadow_reserved 181~184 维持）；conformance 锚定 accept 一件 + reject 一件（fix_closure_criterion_minimal.rx / closure_handwritten_threshold.rx）同 PR 落；trace_matrix 374→375 全锚定 + stable 快照 374→375 重 bless（RXS-0180 L2 加性演进，error_codes/editions/subcommands 三段 0 变化）。`Assisted-by: Kimi-K3（G11.2 波）` | **Full RFC**（RFC-0028） |
| v1.6 | 2026-08-17 | G12.4 UE Path Tracer 对标波 spec-first（硬规则 7 条款先行）：**RXS-0403 单号 materialize 为条款头**——UE PT 对标口径（对标契约独立冻结〔schema `rurix.g12.ue_pt_parity_contract.v1` 字段闭集 + 场景闭集 {cornell-box, bistro-interior} M133 清单 digest 转引 + sun/sky=0.0 显式登记 PT 起步范围面 + 不动 G10.5/G11.5b 锁定值〕+ canonical 字节布局与 digest 门序〔版本前缀 `G12PTP-1\0` + RXS-0384 L3 同构规则 + 三方独立实现 digest 全等机核 + 不等仍出报告即 RED〕+ 双端同场景同 spp 出图〔UE 5.8.1 PT MRQ 臂 + UE build digest == M128 登记 ue_build_id 机核 + Rurix 生产化 PT megakernel device 真跑 + 单端缺帧聚合不得 PASS〕+ measured 对拍三面〔收敛曲线逐段端内参照 rel-MAE 对拍 + 噪声谱对拍 + 能量守恒对拍，容差标定程序产禁手写，超容差显式登记即 RED 评审面、静默即 RED〕+ UE PathTracing 模块归属差距登记〔RXS-0391 归属枚举闭集口径继承 + 行集与对拍报告对账 + 差距项静默混入即 RED〕+ 口径对齐先行〔残余口径差逐环节显式登记，未对齐口径消费 delta 即 RED〕+ 不设绝对通过线〔归 G15〕），依据 RFC-0029（Agent Approved 2026-08-17，D-409 评审后）§4.6/§5 + G12_CONTRACT §4.2 M163 行（判据逐字）+ G12_ACCEPTANCE_MAP §1 M163 行 + §3.4 PT 对标契约面；条款号自落盘前实测 `RXS.next_free=403` 顺位领取（0403 单号不跳号，0295/0296 burned 与 shadow_reserved 181~184 维持）；conformance 锚定 accept 一件 + reject 两件（ue_pt_parity_contract_minimal.rx / parity_digest_mismatch_report.rx / residual_caliber_silent.rx）同 PR 落；trace_matrix 384→385 全锚定 + stable 快照 384→385 重 bless（RXS-0180 L2 加性演进，error_codes/editions/subcommands 三段 0 变化）。`Assisted-by: Kimi-K3（G12.4 UE PT 对标波）` | **Full RFC**（RFC-0029） |
