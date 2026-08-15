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
