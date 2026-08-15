<!-- Assisted-by: Kimi-K3（G10.1 治理波 RFC 起草） -->
<!-- Assisted-by: Kimi-K3（D-409 修法批，2026-08-15，Draft v0.2） -->
# RFC-0026 — 画面对标与图像度量语义（G10 伞形：帧捕获 HDR 格式面 / FLIP·SSIM·PSNR 口径冻结 / 逐像素 diff 报告 schema / 差距清单 schema / 双端确定性契约）

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0026（4 位制，编号永不复用，10 §9.5；编号按 2026-08-15 实测 `registry/number_ledger.json` namespaces.RFC `next_free=26` 领取，非推测号；`reserved_in_flight[G10]` 登记由 G10.1 治理波落，本条目只登记 0026——同波第二份 Full RFC「外部参照 harness 与许可边界」领 RFC-0027，由并行登记条目自行 claim） |
| 标题 | 画面对标与图像度量语义（G10 伞形单章） |
| 档位 | **Full RFC**（图像度量口径与 diff 报告/差距清单/确定性契约 schema 冻结、帧捕获 HDR 容器语义、跨端 digest 协议——触运行时/工具链语义与 evidence ABI 面，下游 spec 新增条款，10 §3 / AGENTS 硬规则 5；判档争议向上取严） |
| 状态 | **Agent Approved（2026-08-15）**——D-409 对抗性评审已完成：独立隔离评审会话 17 findings（3 blocker F1~F3 / 7 major / 7 minor）全部 disposition 落实（v0.2 修法批），§9.1 评审记录段已回填；同环境单一模型 provenance 偏差按 RFC-0015 §9.1/v1.29/v1.73（G9 三 RFC）/v1.90（RFC-0025）先例如实登记于 §9.1，效力自限并留 G10.8b 终审复核锚。主会话核对契约/MAP/CI_GATES 三面一致后翻 Agent Approved。Approved 不构成实现许可，G10.2+ 由 G-G10-3 实现互锁硬门独立阻断 |
| 承接里程碑 | G10（G10.2 M130 骨架 → G10.4 度量基建 M134~M138 → G10.5 首轮 A/B M130 双端核验/M139/M140；验收门 G-G10-4 / G-G10-6 / G-G10-7） |
| 关联条款 | 拟落 spec **RXS-####~**（条款号一律 **post-interlock actual-next-free allocation**，不预写推测号；候选落点见 §5：新建 `spec/visual_comparison.md` + `spec/imageio.md` 追加新章） |
| 依据决策 | D-406 v2.0 · D-409 · P-09 · P-13 · 10 §7/§9.5 · G10 立项九项裁决（[G10_CONTRACT](../milestones/g10/G10_CONTRACT.md) §7：裁决 4 指标集 = FLIP+SSIM+PSNR 参考实现选型与版本 pin 由 RFC 冻结、裁决 5 零通过线）· G10_CONTRACT §4.2（M130/M134/M135/M136/M137/M139/M140 硬判据字面）· [G10_PLAN](../milestones/g10/G10_PLAN.md) §2 G10.4/G10.5 · R-G10-3/R-G10-6/R-G10-7 · [G10_ACCEPTANCE_MAP](../milestones/g10/G10_ACCEPTANCE_MAP.md) §1/§2/§3.3（M130 单 key 双 phase）· [G10_CAPABILITY_MATRIX](../milestones/g10/G10_CAPABILITY_MATRIX.md) §3/§4（M134~M140 行、UE5 模块枚举基线）· RXS-0305 canonical serialization 律（CanonW）· RXS-0369~0373（M118/M119 显示管线冻结面，0-byte 消费）· [G10.1 spike](../milestones/g10/design/g10_ue5_harness_spike.md)（UE5 出图面事实） |
| Provenance | `Assisted-by: Kimi-K3（G10.1 治理波 RFC 起草）` |
| Agent 批准 | 待 D-409 第 1 轮对抗性评审完成、findings 全部 disposition 后由主会话翻 Agent Approved（本文 Draft 不自行批准） |
| 对抗性评审 | **已完成**（D-409 第 1 轮，2026-08-15，独立隔离会话零共享上下文；17 条 findings 全部采纳并修，disposition 逐条见 §9.1；provenance 偏差如实登记：评审者与起草者同模型、独立性 = 会话隔离，非跨工具——跨工具评审者可得时建议补一轮） |

---

## 1. 摘要

本 RFC 冻结 G10「UE5 画面对标基线期」的图像度量语义面——六个子面一份冻结：

1. **帧捕获 HDR 格式面**（M134）：canonical 帧容器裁决 = **OpenEXR（.exr）float32**（无损压缩闭集、色彩空间/位深/元数据字段闭集、捕获→回读逐像素往返无损）；
2. **FLIP 口径**（M135）：参考实现选型 = NVIDIA FLIP 开源参考实现（NVlabs/flip），版本 pin 策略 = commit digest + 构建/运行参数集全登记；HDR/LDR 双域口径参数闭集；对拍容差一律 M138 measured 标定，禁手写；
3. **SSIM/PSNR 口径**（M136）：Wang 2004 标准参数化闭集（11×11 高斯窗 σ=1.5、K1/K2、data_range、逐通道均值聚合）；恒等图对极值断言语义（SSIM=1 / PSNR=inf / FLIP=0）；
4. **逐像素 diff 报告 schema**（M137）：机器 canonical 误差 EXR + 人读热区图双层产物、逐区域统计字段闭集、evidence JSON 字段闭集；
5. **差距清单 schema**（M140）：UE5 Renderer 模块归属枚举闭集（基于 `Engine\Source\Runtime\Renderer` 真实模块目录 2026-08-15 实测在树）、measured delta 字段、G11 承接锚字段；
6. **双端确定性契约**（M130）：相机/光照/时间/后处理参数 schema、二进制 canonical preimage + SHA-256 digest 算法、双端解析一致性语义、门序硬约束（**digest 不等不得出 A/B 报告**，机器阻断）。

```text
        双端确定性契约（M130，参数 JSON → canonical preimage → SHA-256）
           │  digest 相等（门序硬前置：不等不得出 A/B 报告）
           ▼
UE5 5.8 参考帧 ──┐                      ┌── Rurix 帧捕获（M134）
  （MRQ/HighResShot EXR）│  同场景同相机同光照  │（EXR float32 往返无损）
           ▼           ▼                      ▼
        HDR 臂（scene-linear）──────── HDR-FLIP ──────────┐
        LDR 臂（display-referred，固定单 view transform）  │
           └─ LDR-FLIP + SSIM + PSNR（M135/M136 口径闭集） ┤
                                                          ▼
                              逐像素 diff 报告（M137：误差 EXR + 热区图 + 区域统计）
                                                          ▼
                              差距清单（M140：模块归属枚举 + measured delta + G11 承接锚）
```

本 RFC 是 **G10.1 governance-only** 交付物。即使随后 Agent Approved，也只表示语义评审通过，**不会解锁任何 `src/`、`spec/`、`conformance/` 实现**；G10.2 互锁（G-G10-3）是独立硬门。§4 全部 schema/参数/算法为**拟议语义（Draft）**，批准前不构成契约。**G10 零通过线纪律**：本 RFC 不冻结任何画质通过阈值与帧率通过线——差距全量 measured 登记即绿，修复归 G11。

## 2. 动机、范围与治理门

### 2.1 为什么需要 Full RFC

G10 的主交付是「同场景同相机同光照双端出图 + 度量报告 + 差距清单」。这一链条的每一环都是跨端契约：帧容器格式决定双端帧可否无损互换；FLIP/SSIM/PSNR 的口径参数决定度量数字可否复核与复跑（R-G10-3：同一图对不同实现输出不一致即口径事故）；diff 报告与差距清单 schema 是 G10.8b 终审锁定的 **G11 法定输入**；双端参数 digest 是 A/B 对比成立与否的门序前提（R-G10-6：口径不对齐则差距清单被噪声淹没）。这些面涉及：图像编码容器语义（image-io 扩面）、度量算法口径冻结（工具链语义）、evidence/report JSON ABI（跨期消费面）、跨端确定性协议（digest/canonicalization）——都不是 Direct/Mini 可安全承载的局部实现选择。

法定输入字面（G10.1 立项已定案）：

- G10 立项裁决 4：「图像度量指标集 = FLIP + SSIM + PSNR 三指标；参考实现选型与版本 pin 由 RFC 冻结；HDR/LDR 域口径同冻」——本 RFC §4.2/§4.3 即该裁决的兑现面；
- G10 立项裁决 5：「G10 不设画质通过阈值与帧率通过线」——本 RFC 只冻结口径与 schema，全部容差/阈值数值经 M138 标定程序 measured 入 `g10_budget.json`；
- G10_CONTRACT §4.2 七行 P0 硬判据（M130/M134/M135/M136/M137/M139/M140）是本 RFC 语义面的下游机器消费者，判据字面不在本 RFC 重定。

### 2.2 双门互锁：RFC 批准不等于实现开工

| 门 | 允许动作 | 禁止动作 |
|---|---|---|
| G10.1 governance-only（本波） | 起草/评审/批准 RFC；冻结语义面与 §5 spec 映射计划；编号 claim 登记 | 不改 `src/`、`spec/`、`conformance/`；不 materialize 数字 CI 步骤；不预建空 schema 壳/空脚本占位；不领取 RXS/RD/U/RX 共享在途号 |
| G10.2+ implementation gate | G-G10-3 机器事实（validator READY + 用户开工指令 + actual `next_free` 重校）齐备后，spec-first 落条款与 RED | 互锁任一红时不得以 RFC Approved 或立项裁决替代机器事实 |

### 2.3 in-scope（语义面 ↔ P0 key 映射）

| 面 | 本 RFC 冻结内容 | P0 key（G10.1 已冻结字面，0-byte 引用） | 最晚波次 |
|---|---|---|---|
| 双端确定性契约 | 参数 schema 四节闭集 / 值约定（契约世界系与应用层探针）/ digest 算法 / 双端解析一致性 / 门序三重绑定 | `g10.p0.m130.dual_determinism_contract` | G10.2 骨架 → G10.5 双端核验 |
| 帧捕获格式面 | EXR float32 canonical / 压缩闭集 / 色彩空间·位深·元数据闭集 / 往返无损 | `g10.p0.m134.frame_capture_pipeline` | G10.4 |
| FLIP 口径 | 参考实现选型与 pin 策略 / HDR-LDR 双域参数闭集 / 恒等极值 | `g10.p0.m135.flip_metric` | G10.4 |
| SSIM/PSNR 口径 | Wang 2004 参数闭集 / data_range / 恒等极值 / 域限定 | `g10.p0.m136.ssim_psnr_metric` | G10.4 |
| 逐像素 diff 报告 | 双层产物 / 区域统计字段闭集 / evidence JSON 闭集 | `g10.p0.m137.pixel_diff_report` | G10.4 |
| 差距清单 | UE5 模块枚举闭集 / measured delta / G11 承接锚 / kind 分列 | `g10.p0.m140.gap_registry` | G10.5 |
| A/B 报告门序消费 | M130 digest 前置的机器核验语义 | `g10.p0.m139.ab_comparison`（消费面） | G10.5 |
| 阈值标定纪律 | 容差/阈值一律 measured 标定入 `g10_budget.json`（P1） | `g10.p1.m138.metric_threshold_calibration` | G10.4 |

key/脚本/evidence schema 三方逐字一致字面以 [G10_ACCEPTANCE_MAP](../milestones/g10/G10_ACCEPTANCE_MAP.md) §1/§2 为唯一事实源；本 RFC 只消费不新造。

## 3. 指导级解释（用户视角）

### 3.1 一帧的旅程

G10.5 的一次 A/B 对比看起来是这样：harness 读入场景清单中某场景（如 Sponza）与一份**参数 JSON**（相机位姿/光照/时间/后处理四节），双端各自解析该 JSON 并各自计算 **digest**——两端 digest 相等，才允许进入出图；不等则整条链 fail-closed，不出任何报告。digest 相等证明的是「解析一致」；「应用一致」（同一组参数被两端解释成同一个相机与光照）另有应用层探针断言——标定场景已知标志物经双端各自管线投影的像素位置一致性随 M130/M139 evidence 机核（§4.6 值约定）。UE5 侧经 MRQ/HighResShot 出 EXR 参考帧，Rurix 侧经帧捕获管线出 EXR 帧；两帧各自带元数据闭集（域、色彩空间、位深、来源端、view transform、参数 digest）。度量工具读入两帧：HDR 臂跑 HDR-FLIP（scene-linear 域）；LDR 帧由各自 HDR 帧派生——HDR 帧为权威源，view transform 双端共用同一参数字面（实现差登记 `caliber_diff`），随后双端共用同一 host 侧 sRGB 编码器（spec 口径单源）产 sRGB 显示域帧，跑 LDR-FLIP + SSIM + PSNR（UE 侧 LDR 产出路径为拟议语义，已入 spike 待验证清单，§4.1）。输出 = 标量集 + 逐像素误差 EXR + 灰度热区图 + 16×16 区域统计——同一份误差缓冲的三种确定性投影，任何两者不一致即 RED。最后差距清单登记：每条差距带 UE5 Renderer 模块归属（枚举闭集内）、measured delta（数值可溯源到 diff 报告 digest）、建议 P 级与 G11 承接锚。**G10 不判「过/不过」**——清单全量登记即绿，是否修复、修什么由 G11 立项裁决。

### 3.2 恒等图对自证

任何一次度量运行前，工具链先跑恒等图对（同一帧对同一帧）：FLIP 必须恰为 0、SSIM 必须恰为 1、PSNR 必须为 inf（evidence JSON 记字符串 `"inf"`）。任一不成立 = 工具链未接通或口径漂移，本场景度量结果整体作废（RED）——这是度量可信的自证锚，先于一切对拍容差。

### 3.3 口径差与画质差距分列

差距清单的 `kind` 字段把「口径差项」（`caliber_diff`，如 UE5 默认 ACES Filmic 与 Rurix ACES 1.3 插件的已知实现差、UE5 侧 fp16 帧量化）与「画质差距项」（`quality_gap`，如 GI 能量缺失、阴影走样）分列。口径差项登记后不构成修复项；画质差距项才携带 G11 承接锚进入修复候选池。两类分列防止 R-G10-6 的「口径差噪声淹没真实差距」。

## 4. 参考级设计（拟议语义，Draft）

### 4.0 跨面不变量

1. **measured-only**：全部容差与阈值（对拍容差、区域超阈计数阈值）经 M138 标定程序实测标定入 `g10_budget.json`（measured_local，provenance 齐备，P-09）；本 RFC 冻结口径、schema 与标定程序语义，**不冻结任何数值阈值**；手写阈值冒充标定即 RED。
2. **fail-closed / strict-only**：schema 外字段、未知枚举值、未知色彩空间/域标签、位深截断、sRGB/线性混标、参数 digest 不等、误差产物间不一致，均确定性拒绝，不得静默继续或静默降级。
3. **deterministic**：同参数双跑出帧 digest 一致（M129 既有字面）；同输入度量双跑逐位一致；canonical 编码/digest 不含路径、mtime、随机量。
4. **门序硬约束（三重绑定）**：M130 双端核验期 digest 不等 → 不得出 A/B 报告。M139 机器前置 = 三重绑定同时成立：(a) M139 当次 evidence 内嵌当次双端 digest 且二者相等；(b) 该 digest == M130 双端核验期**最新** evidence 登记的 digest 值；(c) M130 与 M139 evidence 同 `base_commit` 且同会话链（`session_run_id` 相等）。「最新」排序键 = evidence 顶层 `timestamp`（UTC）最大者，并列时以落盘文件名 UTC stamp 为次键，仍并列 fail-closed（拒绝，不猜）。validator 同时核验该 M130 evidence `status=="pass"` 且 `phase_g10_5_pass==true`（沿 G9 D2-Q7 / RXS-0375 门序阻断先例）；陈旧 pass 冒充当次一致不得通过——回看历史绿不构成当次一致。
5. **口径差与画质差距分列**：`kind ∈ {quality_gap, caliber_diff}`，禁混列（R-G10-6）。
6. **0-byte 边界**：`spec/display_pipeline.md` RXS-0369~0373、`spec/imageio.md` RXS-0114~0117、G5 既有 SSIM 门禁 helper（`src/rurix-render/src/temporal/ssim.rs`）、G8 M24 时域底座字面 0-byte 不动；UE 源码零 vendoring、零片段复制（R-G10-10）。

### 4.1 帧捕获 HDR 格式面（M134）

**裁决：canonical 帧容器 = OpenEXR（.exr），RGB 三通道（alpha 可选且不进入度量面），scanline 布局；Rurix 侧 canonical = float32 每通道（UE 侧实际位深以 harness 实测登记，Q11 不对称口径）；无损压缩闭集 `{NONE, ZIP}`——与首选自研解码子集逐一对齐，PIZ/RLE 为修订行演进位，DWAA/DWAB 及一切有损压缩禁入。**

**与等价格式的论证**（否决理由详见 §7）：

| 候选 | 结论 | 要点 |
|---|---|---|
| OpenEXR float32 | **采纳（基准）** | 工业标准 HDR 帧容器；UE5 5.8 MRQ 原生逐帧 EXR 序列、HighResShot `bCaptureHDR` 输出 EXR（[spike](../milestones/g10/design/g10_ue5_harness_spike.md) 问题 3 实证）；任意元数据属性；无损压缩族齐备；双端同容器消除容器口径差 |
| Radiance HDR（.hdr，RGBE） | 否决为基准 | 共享指数 8-bit mantissa 对 float32 源为有损量化，往返无损判据不成立；无任意元数据面 |
| PFM | 否决为基准 | float32 无损但零元数据、零压缩标准、工具链弱；元数据齐备判据（M134 字面）不成立 |
| PNG-16 | 否决 HDR 面 | LDR 位深截断（M134 RED 臂即「8-bit clamp 注入」）；仅可作派生预览图，不作 canonical |

**捕获点与域闭集**：`domain ∈ {"scene-linear-hdr", "display-referred-ldr"}`，双臂定义：

- **HDR 臂**（`scene-linear-hdr`）：tonemap / view transform **之前**的 scene-referred 线性帧。UE5 侧 = MRQ/HighResShot HDR 捕获面；Rurix 侧 = 后处理骨架（RXS-0370）tonemap 节点之前的 HDR 线性域帧。此臂是画质差距主战场。
- **LDR 臂**（`display-referred-ldr`）：显示域 sRGB `[0,1]` 编码帧，**由本端 HDR 帧派生**（HDR 帧为权威源、LDR 帧为派生产物）。view transform 双端共用同一参数字面（§4.6 `post.view_transform` v1 仅 `"aces13"`）：Rurix 侧 = ACES 1.3 内置插件（RXS-0369 四内置之一，本 RFC 消费不重定）；UE5 侧 = MRQ tone curve 启用（其默认 ACES Filmic tonemapper 配置），实现差登记为 `caliber_diff` 已知口径差项（§3.3）。view transform 后的线性显示域帧经**双端共用同一 host 侧 sRGB 编码步骤**（编码器口径进 §5 spec 条款单源冻结；Rurix 侧 LDR 帧同走该编码器，编码差从构造上消除）产出 sRGB 编码帧。**UE 侧产出路径裁决 = 派生路径**：UE 官方导出格式文档明示 `.exr` 目标不应用 sRGB 编码曲线——tone curve 启用时 EXR 写 tonemap 后**线性** [0,1] 值（fp16），禁用时写未压缩线性 HDR（[0,100+]）；故 UE 侧不存在原生 sRGB 编码 EXR 出图面（PNG/JPG 路径触发 M134 位深截断 RED 臂），LDR 帧只能由 MRQ EXR（tone curve 启用，fp16→f32 提升）派生——本裁决消除「M136 与 LDR-FLIP 半臂无合法 UE 输入」矛盾（否决候选见 §7）。UE 侧 tone curve 配置/位深/host 编码器接通的实测入 spike 待验证清单，未实测前本路径为拟议语义。LDR 臂服务显示域体感对照与 SSIM/PSNR 口径面（§4.3）。

**色彩空间闭集**：`color_primaries="rec709"`、`white_point="d65"`、`transfer ∈ {"linear", "srgb"}`（HDR 臂必 `"linear"`，LDR 臂必 `"srgb"`；错配 = 混标 RED）。闭集外取值拒绝。

**位深**：Rurix 侧 canonical = float32/通道；UE5 侧实际输出位深（fp16/fp32）以 harness 实测登记入 provenance（spike 待验证清单顺位）；**位深不对称是已知口径事实**——度量计算域统一提升到 float32，fp16 量化差登记 `caliber_diff`，不构成 `quality_gap`。M134「捕获→回读逐像素往返无损」判据约束 **Rurix 侧管线**：capture→encode→落盘→decode 后逐像素 float32 位级相等（NONE 平凡成立；ZIP 无损成立）。

**元数据字段闭集**（EXR header 标准属性 + 自定义属性 `rurix:*` 命名空间，闭集外禁写）：

| 属性 | 类型 | 必填 | 语义 |
|---|---|---|---|
| `dataWindow` / `displayWindow` | EXR 标准 | 必 | 分辨率（宽高） |
| `chromaticities` | EXR 标准 | 必 | Rec.709 primaries + D65 白点（与色彩空间闭集互证） |
| `rurix:schema_version` | string | 必 | 帧元数据 schema 版本（v1 起，加性演进） |
| `rurix:domain` | string | 必 | §4.1 域闭集 |
| `rurix:transfer` | string | 必 | `"linear"` / `"srgb"` |
| `rurix:bit_depth` | string | 必 | `"float32"`（canonical）/ `"float16"`（UE5 侧实测登记） |
| `rurix:source_end` | string | 必 | `"rurix"` / `"ue5"` |
| `rurix:view_transform` | string | LDR 臂必 | `"aces13"` / `"aces20"` / `"agx"` / `"neutral"` / `"ue5-default-aces-filmic"` |
| `rurix:capture_params_digest` | string | 必 | §4.6 参数 digest（M130 链，帧 ↔ 参数互证） |
| `rurix:derivation` | string | 必 | `"capture"`（直接捕获，HDR 臂）/ `"derived:host-srgb-encoder-v1"`（LDR 臂派生链标记位） |
| `rurix:source_frame_digest` | string | 派生帧必 | 派生源 HDR 帧 digest（派生链互证；`"capture"` 帧缺省合法） |
| `rurix:chromaticities_origin` | string | 条件必 | `"writer"` / `"harness-backfill"`（UE 帧 `chromaticities` 缺失补写时必填，见下） |

**EXR 标准属性白名单与读取侧策略**：允许的标准属性白名单 = 结构属性（`channels` / `compression` / `dataWindow` / `displayWindow` / `lineOrder`）+ 可选标准属性闭集 {`chromaticities`（必填，见上表）、`pixelAspectRatio`、`screenWindowCenter`、`screenWindowWidth`}。读取策略按 `rurix:source_end` 分列——`"rurix"` 帧 **strict**：白名单与 `rurix:*` 闭集外属性确定性拒绝；`"ue5"` 帧：闭集外属性 **strip-and-log**（剥离并逐属性随 provenance 登记，含属性名与值 digest），不得因 UE 写出器附带属性拒收真实帧；白名单内属性保留并参与互证。`chromaticities` 在 UE 写出器的落盘情况入 spike 待验证清单；处置条款计划（spec 条款落字面）：实测缺失 → harness 补写 Rec.709/D65 闭集值并以 `rurix:chromaticities_origin="harness-backfill"` 登记；实测存在但值 ≠ 闭集 → 拒绝。

**实现面登记（诚实边界）**：`src/image-io` 现状 = 仅 PPM P6 落地（PNG 为 stub 返回 `UnsupportedFormat`，2026-08-15 实测 [lib.rs](../src/image-io/src/lib.rs)）；EXR 编解码是 G10.4 新实现面。**首选自研最小子集**（scanline：NONE 编 + NONE/ZIP 解，零外部依赖纪律与确定性字节流最直白）——ZIP 解构成如实登记 = 手写 inflate（动态 Huffman / 存贮块 / 窗口边界）+ EXR 专属 predictor（差分预测）/reorder（字节重排）还原，这是 image-io 现状到 EXR 之间最硬的一段，工程量不作「最直白」低估（§9 Q2）；PIZ/RLE 支持为修订行演进位。引入第三方解码库须经依赖治理另判（§9 Q2）。**harness 必须将 UE 侧 EXR 压缩配置收窄至自研可解子集 {NONE, ZIP}**（MRQ 输出设置强制无损且限该子集），配置值以 M128/M134 evidence 字段登记（漂移即 RED）；UE5 侧输出位深/tone curve 配置同以实测登记（spike 待验证清单顺位）。

### 4.2 FLIP 口径（M135）

**参考实现选型（冻结）**：NVIDIA FLIP 开源参考实现——NVlabs/flip（Andersson et al., *FLIP: A Difference Evaluator for Alternating Images*, HPG 2020 的官方开源实现；许可以实现波核照登记为准，选型前提为开源可构建）。**版本 pin 策略**：以 **pin 五元组** 钉死并随 evidence 登记（R-G10-3 版本漂移对策）= **commit digest + 实现分支/后端（cpp tool / cpp header lib / CUDA / python-nanobind）+ OS/工具链 + 构建配置 + 运行参数集**——上游明示跨 OS 输出可像素级不一致、C++ 与 CUDA 后端结果亦不同（v1.4 中位数实现变更即改结果，仓库 `misc/precision.md` 精度声明），分支/后端与 OS/工具链必须是显式 pin 维度；G10.4 首日实测钉死具体取值，本 RFC 不预写（写死未经实测的 digest 违反 measured 纪律）。

**双域口径**（与 §4.1 双臂一一对应）：

- **HDR-FLIP**：输入 = HDR 臂 scene-linear 帧（线性 HDR）；曝光参数面对齐参考实现实际面——`hdr_exposure_mode ∈ {"auto-from-reference", "fixed"}`：`auto-from-reference` = 由**参考图中位亮度**推导 start/stop 曝光（参考实现 v1.7 起 median=0 安全）；`fixed` 时 `{hdr_exposure_start, hdr_exposure_stop, hdr_num_exposures}` 三参必填（曝光区间起点/终点/曝光数 N，对应参考实现 start/stop/numExposures 参数面）；单值 `hdr_exposure_value` 形态否决（与参考实现参数面不符，照抄即不可执行）。
- **LDR-FLIP**：输入 = LDR 臂显示域 sRGB `[0,1]` 帧。

**口径参数闭集**（闭集外参数禁调；值随参考实现默认 pin，偏离默认须经 M138 标定程序登记理由）：

| 参数 | 闭集/口径 |
|---|---|
| `domain` | `"hdr"` / `"ldr"`（与帧 `rurix:domain` 互证，错配拒绝） |
| `ppd` | pixels-per-degree 正数；或由 viewing geometry 三参数（`viewing_distance_m` / `screen_width_m` / `resolution_x`）按参考实现公式推导——两形态二选一，登记采用形态；**ppd 策略冻结：全语料单一值或单一推导几何**（采用形态与取值随 `metric_caliber` digest 登记；语料内逐场景漂移即口径漂移 RED，跨场景 FLIP 标量方可比）；变更走修订行 |
| `hdr_exposure_mode` / `hdr_exposure_start` / `hdr_exposure_stop` / `hdr_num_exposures` | HDR 域曝光参数面（见上，对齐参考实现 start/stop/N + auto 语义） |
| `colorspace_transform` | `"YCxCz"`（论文口径，冻结） |
| `feature_filters` | 边缘/点检测参数集 = 参考实现默认（pin 五元组覆盖） |
| `spatial_pooling` | 加权均值聚合，输出标量 ∈ `[0,1]`（0 = 不可区分） |
| `error_map_output` | 必开（逐像素误差图，§4.4 机器 canonical 面的 FLIP 源） |

**对拍与容差**：自实现与参考实现在同一测试图对上逐图输出一致，容差 = 实现差噪声底，**一律 M138 measured 标定入 `g10_budget.json`，禁手写**（契约 G-G10-6 / 立项裁决 5 字面）。**对拍容差两面分列**——标量对拍容差（逐图标量差）与**误差图对拍容差**（逐像素误差图差）分列 M138 标定、分列登记：上游明示跨 OS/跨后端误差图可像素级漂移，而 §4.4 机器 canonical 误差 EXR 直接取 FLIP 误差图，容差面必须覆盖误差图而非仅标量。**对拍图集下界（语义冻结，归属 §5 spec 条款）**：图集 ≥ 24 图对；内容类五类每类 ≥ 4——高频边缘 / 平滑渐变 / 噪声 / 高亮截断（clip）/ 色彩孤立区；图集清单与每图 digest 入 evidence；不满足下界的对拍不构成有效标定（「一张平色图满足字面」的稀释通道封堵）。**M138 标定估计器语义（冻结）**：统计量 = 全图集逐图 |自实现 − 参考实现| 差（标量差与误差图逐像素差分列）的样本最大值（p100）；容差 = 样本最大值 × 安全系数 k，k ∈ [1.0, 3.0] 为标定程序参数边界（取值与选择理由随 `g10_budget.json` provenance 登记；估计器形态变更走修订行）；样本集 = 对拍图集 digest 引用——「max×10 永不过载」式估计器自由由此封堵。**恒等图对极值断言**：位级相同图对 → FLIP 标量恰为 `0`；非零即 RED。参考输出扰动注入即 RED；口径参数漂移（闭集外参数或值漂移）注入即 RED（M135 RED 臂字面）。

### 4.3 SSIM/PSNR 口径（M136）

**域限定（防口径混用）**：SSIM/PSNR **仅在 LDR 臂定义**——显示域 sRGB `[0,1]`，`data_range = 1.0`。HDR 臂不定义 SSIM/PSNR：无界动态范围下二者口径不适定（data_range 无公认取值），HDR 域差异由 HDR-FLIP 承担（§4.2）。任何在 HDR 帧上直接计算 SSIM/PSNR 的报告行即口径混用 RED。

**SSIM 口径闭集（Wang et al. 2004 标准参数化）**：

| 参数 | 冻结值/口径 |
|---|---|
| 窗 | 11×11 高斯窗，σ = 1.5（Wang 原文口径） |
| 常数 | K1 = 0.01，K2 = 0.03；C1 = (K1·L)²，C2 = (K2·L)²，L = data_range = 1.0 |
| 协方差 | 总体协方差（不采样校正；`use_sample_covariance = false`，Wang 原文） |
| 聚合 | 逐通道 SSIM → RGB 三通道均值（MSSIM = **mean-SSIM** 均值聚合，**非** multi-scale MS-SSIM（Wang 2003），本 RFC 不对齐 multi-scale 变体）；返回值域 `[-1, 1]` |
| 参考实现 | scikit-image `structural_similarity`（显式参数化：`gaussian_weights=True, sigma=1.5, win_size=11, use_sample_covariance=False, data_range=1.0, channel_axis` 显式），**版本 pin + digest 登记**，G10.4 首日实测钉死；环境不可得时回退臂 = 独立第二实现按 Wang 原文逐字移植对拍（§9 Q4） |

**PSNR 口径闭集**：MSE = RGB 三通道联合均方误差；`PSNR = 10·log10(L²/MSE)`，L = 1.0。参考实现 = scikit-image `peak_signal_noise_ratio`（同 pin 纪律）。

**恒等图对极值断言语义**：位级相同图对 → SSIM 恰为 `1.0`、PSNR 为 `+inf`。JSON 序列化约定：PSNR 字段类型 = number 或字符串字面 `"inf"`（MSE = 0 时的闭集例外值；解析器对 `"inf"` 与有限值双形态均须接受，其余字符串拒绝）。恒等图对非极值即 RED（M136 RED 臂字面）。

**对拍图集与标定**：对拍图集下界（≥24 图对、五内容类每类 ≥4）与 M138 标定估计器语义同 §4.2 冻结字面——SSIM/PSNR 对拍共用同一图集与标定程序，不满足下界即标定无效。

**与 G5 既有 SSIM 门禁 helper 的关系（0-byte 声明）**：`src/rurix-render/src/temporal/ssim.rs`（8×8 盒式窗、L = 1.0，RFC-0016 §4.H3 / G-G5-7 已验收门禁 helper）字面 0-byte 不动；其窗型与本节 11×11 高斯口径**不同属一套口径**——G5 门禁不复用为本 RFC A/B 度量实现，本 RFC 口径不回写 G5 门禁；两口径并存、各自登记、互不冒充。

### 4.4 逐像素 diff 报告 schema（M137）

**双层产物**（同一误差缓冲的确定性投影，互不一致即 RED——M137「diff 图与标量报告不一致注入即 RED」字面）：

1. **机器 canonical 面**：逐像素误差 EXR——float32 单通道、无损、域随输入帧；FLIP 域误差图直接取 §4.2 `error_map_output`。色彩映射前的标量场是唯一事实源。
2. **人读面**：灰度热区图——误差 `e ∈ [0,1]` 经冻结色彩映射闭集 v1 = `{"gray"}`（`e → [e,e,e]`，零色表常量、确定性最直白）映射后，按 RXS-0116 确定量化口径（clamp + 就近取整）落 8-bit 灰度，经 image-io 既有无损通道编码（PPM P6 现状；PNG 接通后同语义加性可用）。色彩映射闭集加性演进走修订行。

**逐区域统计字段闭集**：固定网格 `region_grid = {nx, ny}`（v1 默认 16×16；网格维度入 schema 字段登记，改值走修订行）；`regions[]` 每区域字段闭集 = `{x, y, w, h, pixel_count, err_max, err_mean, err_p95, over_threshold_count}`。`over_threshold_count` 的阈值 = M138 标定值（噪声底上方），报告内嵌阈值数值 + `thresholds.source_digest` 引用 `g10_budget.json` 行（自含 + 可溯源双写）。**百分位口径（冻结）**：`err_p95` = nearest-rank——N 个样本升序排序取第 ceil(0.95·N) 个（1-based；ceil(0.95·N) < 1 时取 1；禁插值法，三面重算一致 golden 与跨组件复核依赖同一口径）。**网格边缘规则（冻结）**：分辨率不被 `region_grid` 整除时，末行/末列区域 `w`/`h` 取实际剩余像素（`pixel_count` = w·h 逐区域对账，禁漂移）。

**标量报告（全图聚合）**：`scalars` 字段闭集 = 域对应指标集（HDR 臂：`flip`；LDR 臂：`flip` / `ssim` / `psnr`）+ 误差全图统计 `{err_max, err_mean, err_p95, over_threshold_pixel_count, over_threshold_ratio}`。

**evidence JSON 字段闭集**（闭集外字段拒收；空场景行即 RED）：

| 字段 | 语义 |
|---|---|
| `schema_version` | 报告 schema 版本（v1 起，加性演进） |
| `scene_id` / `camera_id` / `frame_index` | 场景/机位/帧定位三元组 |
| `end_pair` | 双端帧标识与各自 digest（`rurix` / `ue5` 两端帧 digest 全登记） |
| `domain` | `"scene-linear-hdr"` / `"display-referred-ldr"`（与帧元数据互证） |
| `metric_caliber` | §4.2/§4.3 口径参数闭集的 digest（口径版本互证） |
| `thresholds` | `{value, source: "g10_budget.json", source_digest}` |
| `region_grid` / `regions[]` | 区域统计（字段闭集见上） |
| `scalars` | 全图标量（字段闭集见上） |
| `artifacts` | `{frame_a_digest, frame_b_digest, error_map_digest, heatmap_digest}` 四 digest 闭集 |
| `determinism_contract_digest` | §4.6 参数 digest（M130 链） |
| `provenance` | 环境画像引用（UE build digest/驱动/锁频，M128/M141 画像面登记） |

### 4.5 差距清单 schema（M140）

**每差距项字段闭集**（缺归属/缺承接锚行即 RED；非 measured 叙述充差距即 RED——M140 RED 臂字面）：

| 字段 | 闭集/语义 |
|---|---|
| `gap_id` | stable 标识：`sha256(scene_id ‖ camera_id ‖ ue5_module_primary ‖ kind ‖ title)` 前 16 hex 派生（重跑可复现） |
| `scene_id` / `camera_id` / `domain` | 定位面（与 diff 报告互证） |
| `kind` | `"quality_gap"` / `"caliber_diff"`（口径差与画质差距分列，§3.3） |
| `ue5_module_primary` | UE5 模块归属枚举闭集（见下） |
| `ue5_module_secondary[]` | 0..n，同闭集（跨模块面登记） |
| `measured_delta[]` | ≥1 项：`{metric, a_value, b_value, delta, region_ref?, evidence_digest}`——数值必须可溯源到 M137/M139 evidence digest；纯叙述无测量即 RED |
| `suggested_priority` | `"P0"` / `"P1"` / `"P2"`（建议值，G11 立项重裁，本字段不构成承诺） |
| `g11_anchor` | 非空字符串，G11 承接锚字面；G11 立项只消费 G10.8b 锁定清单 + 本锚（契约 G-G10-11 字面） |
| `title` / `description` | 人读摘要（描述不替代 measured_delta） |
| `attachments[]` | digest 引用闭集（diff 报告/热区图/帧） |

**UE5 模块归属枚举闭集**（**版本锚 = Launcher 5.8.0 正式版 release 口径**——对标基线即官方 5.8 release，立项裁决 2；下列枚举 2026-08-15 实测自 ue5-main @4517329fa 快照树，**快照 ≠ release 标签的版本差风险如实标注**：G10.2 出图环境落地时按 5.8.0-release 标签树（GitHub 只读）全量复核并只追加登记差量；快照独有值不删除、标注 `snapshot_only` 风险注记——只追加演进纪律优先；[G10_CAPABILITY_MATRIX](../milestones/g10/G10_CAPABILITY_MATRIX.md) §0.5 + 本 RFC 起草期目录枚举复核；枚举值 = 规范化正斜杠路径字面，公共前缀 `Engine/Source/Runtime/Renderer/Private/`）：

- **目录级 23 值**（实测全部顶层目录；`Tests` 排除——测试基建非渲染特性归属面）：`CompositionLighting` · `Froxel` · `HairStrands` · `HeterogeneousVolumes` · `InstanceCulling` · `Lumen` · `MaterialCache` · `MegaLights` · `Nanite` · `OIT` · `PostProcess` · `RayTracing` · `Renderer` · `SceneCulling` · `Shadows` · `Skinning` · `SparseVolumeTexture` · `StateStream` · `StochasticLighting` · `Substrate` · `VariableRateShading` · `VirtualShadowMaps` · `VT`。
- **文件级闭集**（对标相关顶层单文件模块，逐一实测在树；全路径 = 公共前缀 + 下列字面。**「对标相关」筛选规则（字面化）**：`Renderer\Private` 顶层 `.cpp` 中，文件名语义可唯一归属某一渲染特性/子系统、且该特性落在 G10 对标场景语料渲染面内者收入；本闭集显式承认为 **curated 子集**（顶层实有 ~130 个 `.cpp`，`SceneRendering.cpp`/`ScreenPass.cpp` 等中枢未收）——**补收触发条件**：差距归属时无精确枚举值可用、只能落 `Other` 或近似条目，即按只追加程序补收对应文件级枚举值（以 5.8.0-release 树复核后登记））：`PathTracing.cpp` · `PathTracingSpatialTemporalDenoising.cpp` · `SceneCaptureRendering.cpp` · `SkyAtmosphereRendering.cpp` · `SkyPassRendering.cpp` · `VolumetricCloudRendering.cpp` · `VolumetricFog.cpp` · `SingleLayerWaterRendering.cpp` · `WaterInfoTextureRendering.cpp` · `SubsurfaceTiles.cpp` · `DBufferTextures.cpp` · `TranslucentRendering.cpp` · `TranslucentLighting.cpp` · `FrontLayerTranslucency.cpp` · `ShadowRendering.cpp` · `ShadowSetup.cpp` · `ShadowDepthRendering.cpp` · `CapsuleShadowRendering.cpp` · `DistanceFieldAmbientOcclusion.cpp` · `DistanceFieldShadowing.cpp` · `DistanceFieldScreenGridLighting.cpp` · `DistanceFieldLightingPost.cpp` · `GlobalDistanceField.cpp` · `ReflectionEnvironment.cpp` · `ReflectionEnvironmentCapture.cpp` · `ReflectionEnvironmentDiffuseIrradiance.cpp` · `ReflectionEnvironmentRealTimeCapture.cpp` · `PlanarReflectionRendering.cpp` · `ScreenSpaceReflectionTiles.cpp` · `ScreenSpaceRayTracing.cpp` · `ScreenSpaceDenoise.cpp` · `FogRendering.cpp` · `LocalFogVolumeRendering.cpp` · `LightRendering.cpp` · `IndirectLightRendering.cpp` · `LightShaftRendering.cpp` · `BasePassRendering.cpp` · `DepthRendering.cpp` · `VelocityRendering.cpp` · `AnisotropyRendering.cpp` · `DecalRenderingShared.cpp` · `GPUScene.cpp` · `HZB.cpp` · `SceneVisibility.cpp` · `DeferredShadingRenderer.cpp` · `Renderer.cpp` · `HaltonUtilities.cpp` · `BlueNoise.cpp` · `HdrCustomResolveShaders.cpp` · `GPUBenchmark.cpp` · `ShadingEnergyConservation.cpp` · `IESTextureManager.cpp` · `RectLightTextureManager.cpp` · `LightFunctionRendering.cpp` · `VolumeLighting.cpp` · `HeightfieldLighting.cpp` · `DistortionRendering.cpp`。
- **终值**：`Other`（全路径 = 公共前缀 + `Other`）——须 `attribution_note` 非空说明；`Other` 行计数进 evidence 统计防滥用。
- **演进纪律**：闭集**只追加修订行**；UE 版本迁移/新归属需求按只追加程序登记新枚举值，旧值永不删除（编号永不复用同构纪律，10 §9.5）。

**场景全集零空行**：清单带 per-scene 汇总节 `{scene_id, gap_count, no_gap_explicit}`——场景全集 × 行集闭集对账；无差距场景显式 `no_gap_explicit=true` 汇总行，禁静默空行（M139/M140「差距清单缺场景行即 RED」字面）。

### 4.6 双端确定性契约（M130）

**参数 schema 四节闭集**（全字段必填；schema 外字段注入即 RED、缺字段即 RED——strict fail-closed；`null` 仅 `sky.cubemap_id` 一位合法。**契约兼容性注明**：G10_CONTRACT §4.2 M130 行字面「相机/光照/时间」三节为**最低断言集**；`post` 节是本 RFC 为 LDR 臂 view transform 双端互证的**扩集**——不收缩契约判据，契约正文冻结，兼容性注明在本侧）：

| 节 | 字段闭集 |
|---|---|
| `camera` | `position`（f64×3，世界系）· `orientation_quat`（f64×4，**w,x,y,z 序冻结**，unit-norm 断言，非单位四元数拒绝）· `fov_y_deg`（f64，垂直视场角）· `near` / `far`（f64，登记面）· `resolution`（`{w, h}` u32） |
| `lighting` | `sun {direction f64×3（unit 断言）, intensity_lux f64, color_linear_rgb f64×3}` · `sky {intensity f64, cubemap_id string|null}` · `exposure {mode: "manual", ev100 f64}`——**自动曝光禁入**（histogram 自动曝光破坏双端确定性；`mode` 闭集 v1 仅 `"manual"`） |
| `time` | `fixed_dt_s`（f64 固定步长）· `warmup_frames`（u32，TSR/时域累积收敛协议，R-G10-7）· `capture_frame_index`（u32，warmup 后捕获帧序号）· `random_seed`（u64）· `jitter {sequence: "halton_2_3", index_base u32, scale f64}`——Halton(2,3) 序列与 UE5 `HaltonUtilities.cpp` 同族，索引基冻结，双端逐样本一致 |
| `post` | `view_transform`（**v1 合法值仅 `"aces13"`**——`"aces20"` / `"agx"` / `"neutral"` 保留演进位，同 `exposure.mode` v1 仅 `"manual"` 先例；LDR 臂双端共用该参数字面，UE5 侧映射其默认 ACES Filmic 配置的实现差登记 `caliber_diff`）· `bloom: false` · `vignette: false` · `motion_blur: false` · `dof: false`（v1 最小闭集 = 全关基线；加性演进走修订行） |

**值约定（契约世界系 / 单位 / 轴向 / FOV——digest 之外的应用一致面）**：全 spec/ 此前无任何渲染世界系条款（2026-08-15 grep 复核：右手系/左手系/Y-up/Z-up/坐标系约定零命中），本 RFC 为双端契约显式立约——

- **契约世界系**：右手系、+Y up、长度单位 = 米（与 glTF 资产链同构，Rurix 侧消费面换算成本最低）。`position` / `sun.direction` 均以契约世界系表达；`orientation_quat` = 契约世界系下主动旋转（q = (w,x,y,z)，列向量 v' = q·v·q*，正方向右手定则）；`fov_y_deg` = **垂直**视场角；aspect = `resolution.w / resolution.h`。
- **UE 侧应用映射（冻结公式）**：UE 5.8 惯例 = 厘米 / 左手系 / Z-up / 相机**水平** FOV。映射 M：位置 `p_ue = (−z, x, y)·100`（cm；循环置换加一次取负、det = −1，右手系→左手系翻转成立）；旋转四元数向量部经同一 M 变换、标量部不变（相似变换 R_ue = M·R·M⁻¹，转角保持）；FOV：`fov_h_ue = 2·atan(tan(fov_y_deg/2)·aspect)`（同角度单位）；`sun.direction` 同 M（方向向量无单位换算）。换算公式字节级字面归 §5 spec 条款单源冻结、双端同字面对拍；Rurix 内部世界系 ↔ 契约世界系换算归 Rurix 侧消费面（G10.2 骨架期登记）。同一组 f64 位模式在两端被解释成不同相机/光照的空隙由此封堵（R-G10-6「口径不对齐」成因层；digest 只证解析一致，不证应用一致）。
- **unit-norm 判定式（冻结常量）**：`orientation_quat` 判定 `|‖q‖² − 1| ≤ 2^-40`，`sun.direction` 判定 `|‖d‖² − 1| ≤ 2^-40`，越界 fail-closed 拒绝。**显式登记：该常量为 schema 合法性谓词（f64 表示论下合法值的固有浮动界），非 measured 标定值，不走 `g10_budget.json`**——P-09 禁手写阈值指 measured 容差，合法性谓词常量不在其列。
- **应用层探针（应用一致机核面）**：M130 双端核验期与 M139 evidence 各含 `application_probes[]`——标定场景冻结标志物（标志物世界坐标集进 spec 条款）经双端各自管线按当次参数投影的像素位置断言：`pixel_delta ≤ 1e-3 px`（同为 schema 合法性谓词常量，不走 budget；超差即「应用不一致」RED）。digest 证解析一致、探针证应用一致，两面缺一不可。

**digest 算法（冻结）**：canonical preimage = **解析后值的二进制编码**，不经十进制文本——版本前缀 + 键规范排序（字典序）+ 逐字段类型标签 + length-prefix 字符串 + f64 取 IEEE-754 binary64 小端位模式 + u32/u64 小端；digest = SHA-256(preimage)。同构 RXS-0305 CanonW 律（版本前缀/length-prefix/规范键排序/禁用面）；**NaN / ±Inf 禁入 schema 值域**。键排序规则冻结 = Unicode code point 序；**字节布局全部自由量（版本前缀具体值、类型标签字节值、嵌套对象与数组编码）由 §5 拟落 spec 条款单源冻结字节级字面，双端按同字面实现并对拍**——族约束不定字节值，单源防分叉。二进制位模式 preimage 消除跨实现十进制 shortest-repr 分歧面（否决文本 canonical JSON 的理由见 §7）。

**双端解析一致性语义**：同一参数 JSON 文本，Rurix 端与 UE5 端各自解析 → 各自重编码 canonical preimage → 各自产 digest；**两端 digest 相等 ⟺ 双端解析一致**（含浮点 round-to-nearest 同值性）。schema 解析器双端各一份、同 schema 版本互证；解析结果回显 digest 是 M130 evidence 的核心字段。**浮点解析口径（冻结）**：双端解析器均须 correctly-rounded（round-to-nearest ties-to-even）为口径要求；**边界浮点差分语料**（−0.0、次正规、2^53 边界、长十进制最短表示、1e-310 等）跨端解析逐位一致断言入 M130 GREEN（§6.2）。**UE 侧 digest 载体（钉死）= UE 进程内嵌 CPython**（PythonScriptPlugin；Launcher 5.8.0 正式版插件层可用，spike 问题 4 已确认最小工程零项目 C++——`json` / `hashlib` / `struct` 标准库齐备，CPython 浮点解析 correctly-rounded 由构造保证）：蓝图否决（无原生 SHA-256 与 f64 LE 位打包能力）；host 侧脚本代算否决（digest 必须证明 **UE 进程内**的解析结果，代算违反「双端各自解析」语义本身）。M130 evidence 登记 `param_digest_rurix` / `param_digest_ue5` 与共同值 `param_digest`（相等时），供门序三重绑定消费。

**门序硬约束（机器阻断，三重绑定）**：M130 双端核验期（`--phase g10.5`）digest 不等 → **不得出 A/B 报告**。M139 门的机器前置 = §4.0 不变量 4 三重绑定：(a) M139 当次 evidence 内嵌当次 `param_digest_rurix` / `param_digest_ue5` 且二者相等；(b) 该 digest == M130 双端核验期**最新** evidence 登记的 `param_digest`；(c) M130 与 M139 evidence 同 `base_commit` 且 `session_run_id` 相等（同一次 A/B 运行链标识，harness 生成并双写）；validator 另核验该 M130 evidence `status=="pass"` 且 `phase_g10_5_pass==true`（[G10_ACCEPTANCE_MAP](../milestones/g10/G10_ACCEPTANCE_MAP.md) §3.3 双阶段口径字面；沿 G9 D2-Q7 / RXS-0375 门序阻断先例）。「最新」排序键 = evidence 顶层 `timestamp`（UTC）最大者，并列以落盘文件名 UTC stamp 次键，仍并列 fail-closed。**参数漂移后未重跑 M130 即以陈旧 pass evidence 出报告 = 旁路，(b)(c) 从机器上封堵**。单端参数漂移注入即 RED；digest 不等仍出 A/B 报告即 RED；陈旧 pass 冒充当次一致注入即 RED（契约 §4.2 M130/M139 行字面，本三重绑定为其机器可核化，不收缩契约判据）。骨架期（G10.2）= schema 解析面 + 单端自洽 digest；双端核验归 G10.5。

## 5. 下游 spec 条款映射（spec diff 计划，G10.2 互锁后 materialize）

条款号一律 **RXS-####（post-interlock actual-next-free allocation）**——G10.2 互锁开放后按 actual `next_free` 逐条领取，本 RFC 不预写任何推测号（`reserved_in_flight[G10].RXS` 零数字 claim 字面）。**spec 条款 PR 先于实现 PR**（硬规则 7）；每条 materialize 时至少一个 `//@ spec: RXS-实际号` 锚点，trace_matrix 全锚定。

**目标文件裁决**：新建 **`spec/visual_comparison.md`** 承载「画面对标度量与差距登记」语义轴（候选既有卷均不同轴：`display_pipeline.md` = 帧图输出与着色专项轴、`imageio.md` = 图像 IO 轴、`rendering_platform.md` = reflection/capability 轴；新建沿 G9.2 `virtual_geometry.md` / G9.4 `global_illumination.md` / G9.5 双卷先例，spec/README §4 登记 + 文件头注留痕）；**帧容器语义挂 `spec/imageio.md` 追加新章**（EXR 面是 image-io 轴自然延伸；RXS-0114~0117 字面 0-byte，RXS-0115 LDR 无损优先序不动——EXR 为 HDR 域新类别加性登记）。

| 条款（拟） | 标题 | 目标 spec（候选） | 测试锚定计划（每条 ≥1） |
|---|---|---|---|
| RXS-#### | EXR HDR 帧容器语义：Rurix 侧 canonical float32 RGB / scanline / 无损压缩闭集 {NONE, ZIP}（PIZ/RLE 演进位）/ 元数据字段闭集（§4.1 表，含派生链字段）/ EXR 标准属性白名单与分端读取策略（rurix strict / ue5 strip-and-log）/ harness 压缩配置收窄登记 / 捕获→回读逐像素往返无损 / 位深截断与 sRGB-线性混标拒绝 | `spec/imageio.md` 追加新章（RXS-0114~0117 字面 0-byte） | 往返无损 golden（float32 位级相等）；8-bit clamp 注入 RED；sRGB/线性混标注入 RED；闭集外元数据写入 RED；rurix 帧白名单外属性注入 RED / ue5 帧 strip-and-log 登记 golden；渲染输出探针图案位级核验 golden / 缺失 RED |
| RXS-#### | 度量域契约：HDR/LDR 双臂捕获点（tonemap 前 scene-linear / 显示域 sRGB）、LDR 臂派生路径（HDR 帧权威源 + 双端共用 host 侧 sRGB 编码器口径单源 + 派生链元数据互证）、LDR 臂 view transform 参数字面 v1 单值 `aces13`、帧域标签与度量域互证 | 新建 `spec/visual_comparison.md` | 域标签错配注入 RED；LDR 臂 view transform 漂移注入 RED；双臂帧 digest 互证 golden；派生帧缺 `rurix:source_frame_digest` RED；双端编码器输出逐位对拍 golden |
| RXS-#### | FLIP 口径闭集（域 / ppd 全语料单一策略 / HDR 曝光 start-stop-N + auto / YCxCz / 特征滤波 / 空间聚合 / 误差图输出）+ 参考实现 pin 五元组登记面 + 恒等图对 FLIP=0 极值断言 + 对拍图集下界（≥24 图对、五内容类）与标定估计器语义（p100 × k，k∈[1,3]） | 同上 | 恒等图对非零 RED；口径参数漂移注入 RED；参考输出扰动注入 RED；图集不满足下界冒充有效标定 RED；对拍容差引 `g10_budget.json` digest（标量与误差图容差分列） |
| RXS-#### | SSIM/PSNR 口径闭集（11×11 高斯 σ=1.5 / K1·K2 / data_range=1.0 / 总体协方差 / 逐通道均值；PSNR 联合 MSE 与 `"inf"` 例外字面）+ 恒等图对 SSIM=1/PSNR=inf 极值断言 + LDR 域限定（HDR 帧计算即拒） | 同上 | 恒等图对非极值 RED；口径漂移注入 RED；HDR 域直算注入 RED；与参考实现逐图对拍 golden |
| RXS-#### | 逐像素 diff 报告 schema：双层产物（误差 EXR canonical + 灰度热区图）/ 区域统计字段闭集 / evidence JSON 字段闭集 / 产物互不一致 RED | 同上 | diff 图与标量报告不一致注入 RED；空场景行 RED；闭集外字段注入 RED；区域统计由误差 EXR 重算一致 golden |
| RXS-#### | 差距清单 schema：字段闭集 / UE5 模块归属枚举闭集（目录级 23 + 文件级 + Other 终值）/ kind 两值分列 / measured_delta 可溯源 / 场景全集零空行对账 | 同上 | 缺归属/缺承接锚行 RED；非 measured 叙述充差距 RED；闭集外模块值 RED；场景缺行 RED；Other 无 note RED |
| RXS-#### | 双端确定性契约：参数 schema 四节闭集 / 值约定（契约世界系右手系 +Y up 米、四元数旋转约定、fov_y 垂直口径与 UE 水平 FOV 换算公式、unit-norm 判定常量、应用层探针标志物集与判定常量）/ 二进制 canonical preimage + SHA-256（字节布局自由量字节级单源）/ 双端解析一致性（digest 相等 ⟺ 解析一致 + correctly-rounded 口径 + 边界浮点差分语料）/ UE 侧载体 = 内嵌 CPython / strict 未知字段拒绝 / 门序三重绑定（digest 不等不得出 A/B 报告，机器阻断） | 同上 | 单端参数漂移注入 RED；schema 外字段注入 RED；非单位四元数/NaN 注入 RED；digest 不等仍出报告 RED（门序）；陈旧 pass 冒充当次一致注入 RED；探针超差 RED；边界浮点语料跨端不一致 RED；同文本双端 digest golden |

- **错误码策略**：G10.1 零 RX claim。实现期运行期失败优先 typed `Err` / 库层错误值（image-io `ImageError` 先例，spec/imageio.md §3 口径）；只有出现新的、用户可行动、可独立到达的诊断类别时，才按当时各段 `next_free` 只追加并同步 en/zh message key。不为每个状态预造 RX。

## 6. feature gate / tracking / 实现序（G10.2 互锁后生效）

### 6.1 Gate 命名空间（G10.1 已冻结字面，0-byte 引用）

| 覆盖面 | canonical gate key | smoke 脚本 | evidence schema 目标路径（只冻结路径） |
|---|---|---|---|
| M130 双端确定性 | `g10.p0.m130.dual_determinism_contract` | `ci/g10_dual_determinism_contract_smoke.py` | `milestones/g10/g10_m130_dual_determinism_contract_evidence_schema.json` |
| M134 帧捕获 | `g10.p0.m134.frame_capture_pipeline` | `ci/g10_frame_capture_pipeline_smoke.py` | `milestones/g10/g10_m134_frame_capture_pipeline_evidence_schema.json` |
| M135 FLIP | `g10.p0.m135.flip_metric` | `ci/g10_flip_metric_smoke.py` | `milestones/g10/g10_m135_flip_metric_evidence_schema.json` |
| M136 SSIM/PSNR | `g10.p0.m136.ssim_psnr_metric` | `ci/g10_ssim_psnr_metric_smoke.py` | `milestones/g10/g10_m136_ssim_psnr_metric_evidence_schema.json` |
| M137 diff 报告 | `g10.p0.m137.pixel_diff_report` | `ci/g10_pixel_diff_report_smoke.py` | `milestones/g10/g10_m137_pixel_diff_report_evidence_schema.json` |
| M139 A/B 对比 | `g10.p0.m139.ab_comparison` | `ci/g10_ab_comparison_smoke.py` | `milestones/g10/g10_m139_ab_comparison_evidence_schema.json` |
| M140 差距清单 | `g10.p0.m140.gap_registry` | `ci/g10_gap_registry_smoke.py` | `milestones/g10/g10_m140_gap_registry_evidence_schema.json` |
| M138 阈值标定（P1） | `g10.p1.m138.metric_threshold_calibration` | `ci/g10_metric_threshold_calibration_smoke.py` | `milestones/g10/g10_m138_metric_threshold_calibration_evidence_schema.json` |

M130 单 key 双 phase（`--phase g10.2` 骨架 / `--phase g10.5` 双端核验），不拆双 key（ACCEPTANCE_MAP §3.3 字面）。**门序硬约束（三重绑定）**进 validator 机核：M139 消费 M130 双端核验期最新 evidence（「最新」= 顶层 `timestamp` UTC 最大、并列以落盘文件名 UTC stamp 次键、仍并列 fail-closed），`status=="pass"` 且 `phase_g10_5_pass==true` 缺失即阻断；另机核 (a) M139 当次双端 digest 相等 ∧ (b) == 该 M130 evidence `param_digest` ∧ (c) 同 `base_commit` 同 `session_run_id`——任一不成立即阻断，陈旧 pass 不得冒充。

### 6.2 真实 RED/GREEN

| 面 | RED（必须先可复现） | GREEN（不得以较弱见证替代） |
|---|---|---|
| M130 | 单端参数漂移注入；schema 外字段注入；非单位四元数/NaN 注入；digest 不等仍出 A/B 报告；陈旧 pass 冒充当次一致注入（M139 digest ≠ M130 最新 evidence digest 或会话链断裂仍出报告）；边界浮点差分语料跨端逐位不一致；应用层探针超差 | 同文本双端解析 digest golden + 门序阻断实测（digest 人为不等 → M139 被拒）+ 三重绑定机核实测（陈旧 evidence 注入被拒）+ 边界浮点差分语料逐位一致 + 应用层探针一致 |
| M134 | 8-bit clamp 截断注入；sRGB/线性混标注入；元数据缺字段；渲染输出探针图案未位级出现于捕获 EXR 即 RED（注入已知像素图案经管线后核验，防恒定合成帧伪绿） | float32 捕获→落盘→回读逐像素位级相等 golden + 元数据闭集齐备 + 探针图案位级核验 golden + UE 侧压缩配置收窄 {NONE, ZIP} 登记 |
| M135 | 参考输出扰动注入；恒等图对非零；口径参数漂移注入；对拍图集不满足下界冒充有效标定 | 自实现 vs pin 参考实现逐图对拍（图集满足 §4.2 下界：≥24 图对、五内容类每类 ≥4；标量容差与误差图容差分列，均 M138 标定值）+ 恒等图对 FLIP=0 |
| M136 | 恒等图对非极值；口径漂移注入；HDR 帧直算注入；对拍图集不满足下界冒充有效标定 | 逐图对拍 golden（图集满足 §4.2 下界字面）+ 恒等图对 SSIM=1/PSNR="inf" + LDR 域限定断言 |
| M137 | diff 图与标量报告不一致注入；空场景行；闭集外字段注入 | 误差 EXR/热区图/区域统计三面重算一致 golden + evidence schema 闭集校验 |
| M140 | 缺归属/缺承接锚行；非 measured 叙述充差距；闭集外模块值 | 场景全集零空行对账 + measured_delta 溯源链核验 + G11 承接锚字面非空 |

### 6.3 栈式实现序

1. **PR-Gate**：G-G10-3 互锁 validator READY + 用户 G10.2 开工指令 + 重读 ledger actual `next_free`。红即停止。
2. **PR-Spec**：按 §5 materialize 实际 RXS 与 RED 语料（spec-first，条款 commit 先于实现 commit）。
3. **PR-Contract**（G10.2 骨架腿）：M130 参数 schema 解析 + canonical/digest + 单端自洽（与 G10.2 出图环境波同波，Rurix 侧消费面骨架）。
4. **PR-ExrIO**（G10.4）：EXR 最小编解码（§9 Q2 裁决执行）+ 元数据闭集 + 往返无损。
5. **PR-Metrics**（G10.4）：FLIP/SSIM/PSNR 自实现 + 参考实现 pin + 逐图对拍 + 恒等极值；M138 标定程序同波（容差入 `g10_budget.json`）。
6. **PR-DiffReport**（G10.4）：双层产物 + 区域统计 + evidence schema。
7. **PR-AB**（G10.5）：M130 双端核验 + M139 A/B 出图编排 + M140 差距清单落盘。
8. **PR-Evidence**：evidence schema 落盘、RTX 4070 Ti 实跑，禁止 YAML-only 与 host substitution。

## 7. 备选方案

| 方案 | 裁决 | 理由 |
|---|---|---|
| Radiance HDR / PFM / PNG-16 作 canonical 帧容器 | 否决 | §4.1 表：RGBE 有损量化、PFM 零元数据、PNG-16 位深截断——M134 往返无损与元数据齐备判据不成立 |
| 只上 SSIM/PSNR，不上 FLIP | 否决 | 感知度量缺失；PSNR/SSIM 对结构/色彩感知差异钝感，不满足「严格画面审查」的感知定位需求（G10-N3 裁决三件套字面） |
| 只上 FLIP，不上 SSIM/PSNR | 否决 | SSIM/PSNR 是既有先例（G5 SSIM 门禁字面，RD-038 history）与社区通用 sanity 面；三指标互证防单度量失效/误标定 |
| HDR 帧上直接定义 SSIM/PSNR | 否决 | 无界动态范围下 data_range 无公认取值，口径不适定；HDR 域由 HDR-FLIP 承担（§4.3 域限定） |
| vendoring OpenEXR C++ 库 | 否决（首选自研最小子集） | 零外部依赖纪律与 vendoring 治理面；scanline NONE/ZIP 子集自研可控且确定性字节流最直白；确需第三方库时经依赖治理另判（§9 Q2） |
| 自研 FLIP 不经参考实现对拍 | 否决 | R-G10-3 口径风险无对照锚；对拍容差 M138 measured 标定是防口径漂移唯一可机核面 |
| digest 走十进制文本 canonical JSON | 否决 | 跨实现 shortest round-trip 文本表示存在分歧面；二进制位模式 preimage（f64 LE）从构造上消除（§4.6） |
| 差距模块归属用自由文本 | 否决 | 机核不可行（枚举闭集校验是 M140 RED 臂基础）；闭集基于 2026-08-15 真实源码树枚举，带 Other 终值与只追加演进纪律 |
| 相机朝向用 yaw/pitch/roll | 否决 | 欧拉角序跨端歧义面（UE 与 Rurix 默认序不同）；四元数 w,x,y,z 序冻结 + unit-norm 断言无歧义 |
| 自动曝光入契约 | 否决 | histogram 自动曝光的状态性破坏双端确定性；`exposure.mode` 闭集 v1 仅 `"manual"`（§4.6） |
| UE 侧 digest 由 host 侧脚本代算 / 蓝图实现 | 否决 | 代算违反「双端各自解析」语义本身（digest 不再证明 UE 进程内解析结果）；蓝图无原生 SHA-256 与 f64 LE 位打包能力；载体钉死 = UE 进程内嵌 CPython（§4.6） |
| LDR 臂 UE 侧容器合法化 PNG-16 | 否决 | 触发 M134 位深截断 RED 臂（8-bit clamp 注入）；派生路径（HDR 权威源 + 双端共用 host 侧 sRGB 编码器）已消除「无合法 UE 输入」矛盾且编码差构造性消除（§4.1） |
| LDR 臂走 OCIO Color Output 路径 | 否决（未实测，留重评位） | 产出能力未验证；派生路径优先（编码器口径单源最直白）。若派生路径 spike 实测受阻，按修订行重评本候选 |
| 压缩闭集保留 PIZ/RLE | 否决（演进位保留） | 闭集必须与首选自研解码子集 {NONE, ZIP} 逐一对齐，否则闭集内合法配置即可产 Rurix 工具链读不了的帧；PIZ/RLE 解码支持落地后经修订行加性开放（§9 Q2） |
| 门序仅回看 M130 历史绿 evidence | 否决 | 陈旧 evidence 旁路（参数漂移后不重跑 M130 仍字面满足判据）；三重绑定（当次 digest ∧ == M130 最新登记 ∧ 同会话链）为机器可核唯一形态（§4.0/§4.6） |
| 契约世界系采用 UE 惯例（厘米/左手系/Z-up） | 否决 | Rurix 资产链与 glTF 同构（右手系/+Y up/米），UE 侧映射为确定性换算公式（§4.6 值约定）；立约侧选换算成本最低端 |

## 8. 不做（范围红线）

- **不冻结任何画质通过阈值与帧率通过线**（G10 零通过线，立项裁决 5 / G-G10-7 字面）：M138 标定的是**度量工具一致性容差与噪声底**（自实现 vs 参考实现的实现差），与「画质通过线」严格两物；任何「已达 UE5 画质」叙述在 G10 期内一律不成立。
- **不接线 DLSS/超分/NRD**（G13 承接；RD-041/RD-040 backfill 字面；temporal 底座 0-byte）：本 RFC 零 vendor SDK 面。
- **UE 零 vendoring、零片段复制**（R-G10-10）：`E:\Kimi_Agent_Taichi Engine 优化计划\references\UnrealEngine` 只读外部参照；模块枚举闭集是路径字面登记，不是源码消费；许可边界与 harness 编排面归并行 RFC-0027 章。
- **不做任何画质修复**（G11 承接）：差距清单只登记不修复；G10.5 后任何画质修复 PR 判 out-of-scope（R-G10-8）。
- **不改 G5~G9 冻结面**：`spec/display_pipeline.md` RXS-0369~0373（M118/M119 显示管线）、`spec/imageio.md` RXS-0114~0117、G5 SSIM 门禁 helper（`temporal/ssim.rs`）、G8 M24 TSR 时域底座字面 0-byte；触任一冻结面必须显式 RFC 修订行。
- **M141 帧率基线协议不在本 RFC**：14 §5 采样协议面与画像字段归 G10.5 实现波 harness/evidence 面，本 RFC 不冻结。
- **压测资产许可登记面不在本 RFC**（M131/M133；归并行 RFC-0027 章）。
- **不在 G10.1 改 `src/`、`spec/`、`conformance/`、`.github/workflows/`**；不 materialize 数字 CI 步骤；不预建空 schema 壳/空脚本占位；不领取 RXS/RD/U/RX 共享在途号；Draft/Approved 状态均不构成实现许可。

## 9. 未决问题 / 关键裁决

下表是本 Draft 的明确裁决提案；Agent Approved 时逐行冻结。若对抗性评审推翻任一项，必须先改正文和本表，再批准。

| ID | 问题 | Draft 裁决 |
|---|---|---|
| Q1 | 帧容器 | OpenEXR RGB scanline；Rurix 侧 canonical float32，UE 侧位深 harness 实测登记（Q11）；无损压缩闭集 {NONE, ZIP}（PIZ/RLE 演进位），有损禁入；色彩空间/位深/元数据闭集按 §4.1 表 |
| Q2 | EXR 编解码实现选型 | 首选自研最小子集（NONE 编 + NONE/ZIP 解，零外部依赖）；ZIP 解构成如实登记 = inflate（动态 Huffman/存贮块/窗口边界）+ EXR predictor/reorder 还原（image-io 现状到 EXR 最硬段，工程量不低估）；PIZ/RLE 为演进位；harness 强制 UE 侧压缩配置收窄至 {NONE, ZIP} 并 evidence 登记；自研不可行时第三方纯 Rust 库经依赖治理另判（本 RFC 不预点名） |
| Q3 | FLIP 参考实现与 pin | NVlabs/flip 开源参考实现（BSD 3-Clause，v1.7 起 HDR auto 曝光 median=0 安全）；pin = 五元组（commit digest + 实现分支/后端 + OS/工具链 + 构建配置 + 运行参数集），G10.4 首日实测钉死并登记（不预写未实测 digest）；上游明示跨 OS/后端像素级差异，对拍容差标量与误差图两面分列 M138 |
| Q4 | SSIM/PSNR 参考实现 | scikit-image 显式 Wang 2004 参数化（§4.3 表），版本 pin + digest 登记；环境不可得回退 = 独立第二实现按 Wang 原文逐字移植对拍，回退触发如实登记 |
| Q5 | HDR 域是否定义 SSIM/PSNR | 不定义（口径不适定）；HDR 域 = HDR-FLIP 承担；HDR 帧直算 SSIM/PSNR 即口径混用 RED |
| Q6 | LDR 臂 view transform 固定 | 参数字面 v1 仅 `"aces13"`（余值演进位）；Rurix 侧 = ACES 1.3 内置插件（RXS-0369 消费不重定）；UE5 侧默认 ACES Filmic 实现差登记 `caliber_diff` |
| Q7 | digest preimage 形态 | 二进制 canonical（版本前缀 + 键排序 + 类型标签 + length-prefix + f64 binary64 LE 位模式）+ SHA-256；NaN/±Inf 禁入 |
| Q8 | 模块归属枚举 | 目录级 23 值 + 文件级闭集 + `Other` 终值（须 note，计数防滥用）；只追加演进，旧值永不删；版本锚 = Launcher 5.8.0 release 口径（现枚举实测自 ue5-main 快照，版本差风险标注，G10.2 按 5.8.0-release 标签树复核只追加登记差量）；「对标相关」筛选规则字面化 + curated 子集补收触发条件（§4.5） |
| Q9 | 曝光与时间参数 | 手动 EV100 固定，自动曝光禁入；固定步长 + warmup + 捕获帧序号 + seed + Halton(2,3) jitter 入契约四节 |
| Q10 | 门序 | M130 双端 digest 不等不得出 A/B 报告；validator 机器阻断（`status=="pass"` 且 `phase_g10_5_pass==true` + 三重绑定：当次双端 digest 相等 ∧ == M130 最新 evidence `param_digest` ∧ 同 `base_commit` 同 `session_run_id`；「最新」= `timestamp` UTC 最大、并列文件名 UTC stamp 次键），不得 waived |
| Q11 | 位深不对称 | Rurix canonical float32；UE5 侧实测位深登记 provenance；计算域统一 float32；fp16 量化差登记 `caliber_diff` 不充 `quality_gap` |
| Q12 | RFC Approved 是否解锁实现 | 不；G-G10-3 互锁是独立硬门，不得以 RFC 状态替代机器事实 |
| Q13 | LDR 臂 UE 侧产出路径 | 派生路径：HDR 帧权威源（MRQ EXR，tone curve 启用，fp16→f32 提升）+ 双端共用 host 侧 sRGB 编码器（spec 口径单源）派生 LDR 帧；UE 官方文档明示 .exr 不应用 sRGB 编码曲线，原生 sRGB 编码 EXR 路径不存在；PNG-16 合法化否决、OCIO 候选未实测留重评位（§7）；实测项入 spike 待验证清单 |
| Q14 | UE 侧 digest 载体 | 钉死 = UE 进程内嵌 CPython（PythonScriptPlugin，Launcher 5.8.0 插件层可用；`json`/`hashlib`/`struct`，correctly-rounded 构造保证）；蓝图与 host 代算否决；字节布局自由量由 §5 spec 条款单源冻结；边界浮点差分语料入 M130 GREEN |
| Q15 | 契约世界系值约定 | 右手系 / +Y up / 米（glTF 同构立约）；UE 映射公式冻结（p_ue = (−z,x,y)·100 cm、四元数相似变换、fov_h = 2·atan(tan(fov_y/2)·aspect)）；unit-norm 判定常量 2^-40 与探针判定常量 1e-3 px 登记为 schema 合法性谓词（不走 budget）；应用层探针入 M130/M139 evidence |

## 9.1 对抗性评审记录（对抗性评审要求，10 §3 / §7 · [`../13_DECISION_LOG.md`](../13_DECISION_LOG.md) D-409）

**评审记录**：D-409 第 1 轮对抗性评审已完成，评审全文与独立事实核对记录见 [rfc0026_adversarial_review.md](../milestones/g10/design/rfc0026_adversarial_review.md)。

| 字段 | 值 |
|---|---|
| 评审者 provenance | `Assisted-by: Kimi-K3（D-409 独立评审会话，与起草会话隔离）` |
| 评审轮次 | 第 1 轮，2026-08-15 |
| 评审会话形态 | 独立隔离会话、零共享上下文——评审者不复用起草会话任何结论；UE 模块枚举、编号台账、image-io 现状、G5 SSIM helper、契约判据逐行、外部事实（UE 官方导出格式文档 / NVlabs-flip 许可与跨 OS 精度声明）均由评审会话独立复核 |
| provenance 偏差登记（逐字登记评审记录对应行） | 评审者与起草者**同模型**（Kimi-K3），独立性 = 会话隔离 + 零共享上下文，不满足 D-409 首选「跨工具/跨模型」字面。按 RFC-0015 §9.1 / number_ledger v1.29/v1.73/v1.90 已登记先例如实偏差登记并效力自限：本评审不替代未来跨工具评审；跨工具评审者可得时建议补一轮 |

**Findings 与 disposition**（17 条全部**采纳并修**，Draft v0.2 同批落实；严重度映射按评审建议：blocker→high，major→high/med 酌定，minor→low）：

| # | Finding（评审者提出） | 严重度 | Disposition |
|---|---|---|---|
| F1 | 世界系/单位/FOV 轴向约定整体缺失——digest 只证「解析一致」，不证「应用一致」 | high | **采纳并修**：§4.6 新增「值约定」小节（契约世界系右手系/+Y up/米立约、UE 厘米/左手系/Z-up/水平 FOV 映射公式冻结、四元数旋转约定、fov_y→水平 FOV 换算公式）；应用层探针（标定场景标志物投影位置断言）入 M130/M139 evidence；§9 Q15 登记 |
| F2 | 门序 validator 判据存在陈旧 evidence 旁路，digest 值无跨门绑定、「最新」未定义 | high | **采纳并修**：§4.0 不变量 4 与 §4.6 门序升级为三重绑定（M139 当次双端 digest 相等 ∧ == M130 最新 evidence 登记 `param_digest` ∧ 同 `base_commit` 同 `session_run_id`）；「最新」= `timestamp` UTC 最大、并列以落盘文件名 UTC stamp 次键、仍并列 fail-closed；§6.1/§6.2 M130 行同步并补「陈旧 pass 冒充当次一致注入即 RED」臂；§9 Q10 同步 |
| F3 | LDR 臂 UE 侧产出路径与 UE 官方文档冲突（.exr 不应用 sRGB 编码曲线），冻结语义无可执行载体 | high | **采纳并修**：§4.1 LDR 臂改派生路径（HDR 帧权威源 + UE MRQ EXR tone curve 启用 fp16→f32 提升 + 双端共用 host 侧 sRGB 编码器、编码口径 spec 单源）；元数据增派生链字段（`rurix:derivation` / `rurix:source_frame_digest`）；UE tone curve 与 ACES 1.3 差照登 `caliber_diff`；§3.1 改拟议口吻并登记 spike 待验证清单；§7 增 PNG-16 否决 / OCIO 留重评位；§9 Q13 登记 |
| F4 | 压缩合法闭集 {NONE, ZIP, PIZ} 超出自研解码子集 {NONE, ZIP}；ZIP 解码工程量被低估 | high | **采纳并修**：压缩闭集收窄 {NONE, ZIP}（PIZ/RLE 演进位）；harness 强制 UE 侧压缩配置收窄子集并以 M128/M134 evidence 字段登记；§9 Q2 如实登记 ZIP 解构成（inflate + predictor/reorder 还原）；§7 增否决行 |
| F5 | HDR-FLIP 曝光参数面与参考实现实际面不符；pin 漏实现分支/后端/OS 维度；ppd 全语料策略未冻结 | high | **采纳并修**：§4.2 曝光面改 start/stop/N + auto（auto 由参考图中位亮度推导，v1.7 起 median=0 安全），单值 `hdr_exposure_value` 形态否决；pin 三元组→五元组（+实现分支/后端 +OS/工具链）；ppd 全语料单一策略冻结（语料内漂移即口径漂移 RED）；标量与误差图对拍容差分列 M138；§9 Q3 同步 |
| F6 | unit-norm 断言无判定口径，实现者被迫发明阈值 | med | **采纳并修**：§4.6 值约定冻结判定式 `\|‖q‖²−1\| ≤ 2^-40`（`sun.direction` 同式），显式登记为 schema 合法性谓词常量、非 measured 标定值、不走 `g10_budget.json` |
| F7 | UE 侧 digest 产出载体未指定；字节布局自由量未钉；浮点解析等价性未声明 | high | **采纳并修**：载体钉死 = UE 进程内嵌 CPython（PythonScriptPlugin，Launcher 5.8.0 插件层可用，spike 问题 4 实证零项目 C++）；蓝图/host 代算否决入 §7；字节布局自由量归 §5 spec 条款单源冻结字节级字面；correctly-rounded 口径声明 + 边界浮点差分语料入 M130 GREEN；§9 Q14 登记 |
| F8 | EXR 元数据读取侧策略与标准属性白名单未定义，真实 UE 帧可能被自家 strict 纪律拒收 | med | **采纳并修**：§4.1 增标准属性白名单（`pixelAspectRatio`/`screenWindowCenter`/`screenWindowWidth`）+ 分端读取策略（rurix 帧 strict 拒绝 / ue5 帧 strip-and-log 随 provenance 登记）；`chromaticities` UE 侧落盘入 spike 待验证清单，处置条款计划 = 缺失补写并 `rurix:chromaticities_origin` 登记 / 值异拒绝 |
| F9 | 模块枚举版本锚 ≠ 对标基线版本；「对标相关」筛选规则未冻结，闭集边界主观 | med | **采纳并修**：§4.5 版本锚统一为 Launcher 5.8.0 release 口径，快照枚举标注版本差风险 + G10.2 按 5.8.0-release 标签树复核只追加登记差量（快照独有值注 `snapshot_only` 不删）；筛选规则字面化 + curated 子集承认 + `Other`/近似归属触发的只追加补收程序；§9 Q8 同步 |
| F10 | 对拍图集与标定程序语义无下界——measured 形式可满足、判别力可任意稀释（伪绿通道） | high | **采纳并修**：§4.2 冻结图集下界（≥24 图对；五内容类——高频边缘/平滑渐变/噪声/高亮截断/色彩孤立区——每类 ≥4；清单与 digest 入 evidence）与 M138 标定估计器语义（p100 × 安全系数 k∈[1.0,3.0] 登记、样本集 digest 引用）；§4.3 共用；§6.2 M135/M136 行引用下界字面并补「不满足下界冒充有效标定即 RED」臂 |
| F11 | `post.view_transform` 四值枚举与「LDR 臂固定 ACES 1.3」矛盾；§3.1 措辞与 caliber_diff 设计冲突 | low | **采纳并修**：§4.6 post 节 v1 合法值仅 `"aces13"`（余值演进位，同 `exposure.mode` 先例）；§3.1 改「双端共用同一参数字面，实现差登记 `caliber_diff`」；§9 Q6 同步 |
| F12 | 「canonical = float32 每通道」裁决句全域绝对化 | low | **采纳并修**：裁决句与位深段限域为「Rurix 侧 canonical float32；UE 侧位深 harness 实测登记（Q11）」 |
| F13 | `err_p95` 百分位方法与区域网格边缘规则未冻结 | low | **采纳并修**：§4.4 冻结 nearest-rank 公式（第 ceil(0.95·N) 个，1-based，禁插值）与末区域 w/h 取实际剩余像素规则 |
| F14 | 「（MSSIM）」术语歧义（mean-SSIM vs multi-scale） | low | **采纳并修**：§4.3 注 mean-SSIM 均值聚合、非 multi-scale MS-SSIM（Wang 2003），不对齐 multi-scale 变体 |
| F15 | 契约字面「相机/光照/时间」三节 vs RFC 四节，扩集未注明关系 | low | **采纳并修**：§4.6 契约兼容性注明——契约 M130 字面三节为最低断言集，`post` 节为本 RFC 扩集、不收缩契约判据（契约正文冻结，兼容性注明在本侧） |
| F16 | M134 GREEN 无活性绑定，恒定合成帧即可满足字面 | low | **采纳并修**：§6.2 M134 增 RED 臂「渲染输出探针图案未位级出现于捕获 EXR 即 RED」，GREEN 补探针图案位级核验 golden；§5 EXR 条款锚定同步 |
| F17 | 评审 provenance 与起草同模型，偏差须随 findings 一并回填 | low | **采纳**：本段「provenance 偏差登记」行逐字回填留痕；本评审不写成「跨工具」；跨工具评审者可得时建议补一轮 |

**总评回填**：评审总评 = **approve-with-changes**（修订后可批准，非现状可批准）。本批修订（Draft v0.2，2026-08-15）将 F1~F17 全部采纳并修：三条 blocker 的语义空隙已在正文冻结（§4.6 值约定与应用层探针 / 门序三重绑定 / §4.1 LDR 派生路径），七条 major 同批落实，七条 minor 同批 disposition，§9 Q 表与 §5 映射同步。**本 RFC 状态维持 Draft**——翻 Agent Approved 由主会话核对本批修订与契约三面（G10_CONTRACT / G10_ACCEPTANCE_MAP / CI_GATES）一致性后执行。

## 10. 稳定化与 provenance

- **特性生命周期**（10 §5）：RFC Agent Approved 只是语义评审完成；随后仍需 G-G10-3 互锁 → spec-first/RED → gated implementation → tracking evidence → 至少两个里程碑无重大语义修订 → stabilization report → FCP-lite。
- **稳定面候选**：EXR 帧元数据闭集与 digest 规则、度量口径参数闭集、diff 报告/差距清单 schema 字段闭集、双端契约 schema 与 canonical/digest 算法、门序硬约束语义；是否 stable 由未来 stabilization report 裁决。
- **明确非 stable**：全部容差/阈值数值（M138 标定值，`g10_budget.json` measured 行）、参考实现具体 pin 值、UE5 侧实测位深与压缩配置、区域网格维度默认值、枚举 `Other` 行计数阈值。
- **Provenance**：`Assisted-by: Kimi-K3（G10.1 治理波 RFC 起草）`；Draft v0.2 修法批 `Assisted-by: Kimi-K3（D-409 修法批）`。D-409 第 1 轮评审记录已回填 §9.1；翻 Agent Approved 由主会话核对后执行。

## 11. 规范与实现依据

- 仓库内：[G10_CONTRACT](../milestones/g10/G10_CONTRACT.md) §4.2/§7（P0 硬判据字面、立项九项裁决）· [G10_PLAN](../milestones/g10/G10_PLAN.md) §2 G10.4/G10.5、§4 R-G10-3/6/7 · [G10_ACCEPTANCE_MAP](../milestones/g10/G10_ACCEPTANCE_MAP.md) §1/§2/§3.3（key 字面、M130 双阶段）· [G10_CAPABILITY_MATRIX](../milestones/g10/G10_CAPABILITY_MATRIX.md) §0.5/§3/§4 · [G10.1 spike](../milestones/g10/design/g10_ue5_harness_spike.md)（UE5 出图面事实：MRQ 逐帧 EXR / HighResShot `bCaptureHDR` / 待验证清单）· [spec/display_pipeline.md](../spec/display_pipeline.md)（RXS-0369~0373，M118/M119 冻结面 0-byte 消费）· [spec/imageio.md](../spec/imageio.md)（RXS-0114~0117）· `src/image-io/src/lib.rs`（2026-08-15 实测现状：PPM P6 落地、PNG stub、零 EXR）· `src/rurix-render/src/temporal/ssim.rs`（G5 SSIM 门禁 helper，8×8 盒式窗，0-byte 声明面）· [RFC-0019](0019-rendering-platform.md) §4.5 · [RFC-0025](0025-world-and-specialty-renderers.md) §4.I~§4.J · RXS-0305 canonical serialization 律 · UE 5.8 源码树 `E:\Kimi_Agent_Taichi Engine 优化计划\references\UnrealEngine`（ue5-main @4517329fa，只读外部参照；`Engine\Source\Runtime\Renderer\Private` 目录与顶层文件 2026-08-15 实测枚举）。
- 外部一手来源：Andersson et al., *FLIP: A Difference Evaluator for Alternating Images*, HPG 2020（FLIP 论文与 NVlabs/flip 开源参考实现）；Wang et al., *Image Quality Assessment: From Error Visibility to Structural Similarity*, IEEE TIP 2004（SSIM 标准口径）；OpenEXR 格式文档（scanline/压缩族/属性面）；scikit-image `structural_similarity` / `peak_signal_noise_ratio` API 文档（参考实现参数化面）。**D-409 修法批联网核查（2026-08-15）**：UE 官方导出格式文档「EXR Sequence：No sRGB encoding curve is applied to .exr targets；Tone Curve 启用时线性值压缩至约 [0,1]，禁用时写 [0,100+] 线性 HDR」——https://dev.epicgames.com/documentation/unreal-engine/cinematic-rendering-export-formats-in-unreal-engine （F3 派生路径事实依据）；NVlabs/flip 仓库 README 与 `misc/precision.md`（BSD 3-Clause、v1.7 HDR auto 曝光 median=0 修复、跨 OS/跨后端输出像素级差异声明）——https://github.com/NVlabs/flip （F5 pin 五元组与双容差分列事实依据）。
- 口径标注：FLIP 参数闭集默认值以参考实现 pin 五元组实测登记为准，本 RFC 只冻结参数名与语义面；UE5 侧压缩/位深/出图配置均以 G10.4 首日实测登记为准（spike 待验证清单顺位），本 RFC 不预写未实测事实。

---

## 12. 章 E — G10.5a 契约四元数共轭公式勘误（errata 纯加性段）

> **增补性质**（errata 体例；先例 = RFC-0024 v1.1 章 F 纯加性章增补 / RFC-0025 §4.L 显式修订行）：本章为 **v1.1 纯加性 errata 段**，§1~§11 既有冻结文本 **0-byte 不动**；被勘误的 §4.6 值约定公式行原文保留，生效语义以本章 E1 为准。缺陷由 G10.5a 双端出图波实证暴露，处置纪律 = spec-first 修订行（spec/visual_comparison.md v1.1 errata）+ RED 先行测试（tests/test_g10_param_contract.py）+ 本章登记。

### E1 🔒 §4.6 UE 映射四元数共轭公式勘误（det(M) = −1 反射矩阵的转角符号）

- **缺陷定位**：§4.6「UE 侧应用映射（冻结公式）」行原文「旋转四元数向量部经同一 M 变换、标量部不变（相似变换 R_ue = M·R·M⁻¹，**转角保持**）」对 det(M) = −1 的反射 M **数学上不成立**。正交共轭一般律：R_ue = M·R(axis, θ)·M⁻¹ = **R(M·axis, det(M)·θ)**；det(M) = −1 时**转角反号**——R_ue = R(M·axis, −θ)，四元数向量部 = **−M·v**、标量部不变。
- **修订式（生效）**：q = (w, x, y, z)（契约，w,x,y,z 序）⇒ **q_ue = (w, z, −x, −y)**。缺陷实现 (w, −z, x, y) = R(M·axis, +θ) 为镜像朝向（左右翻转取景）。位置映射 `p_ue = (−z, x, y)·100`、`sun.direction` 同 M、`fov_h_ue` 换算三式核验无缺陷，维持不变。
- **实证（2026-08-15，G10.5a 波）**：共轭恒等式 R(q_ue)·(M·v) == M·(R(q)·v) 随机对拍——缺陷式最大偏差 6.35e0（2000 组对拍）/ pytest 5000 组首例偏差 1.39e0，修订式偏差 0.0；黄金个案（契约绕 +Y 转 +90° ⇒ 正确 = UE 绕 +Z 转 −90°）缺陷式镜像成立；`tests/test_g10_param_contract.py` RED（commit 先行）→ 修复 GREEN。cornell-box 取景（绕 +Y 180°）为该缺陷**不变量特例**（R(a,180°) ≡ R(a,−180°)，q ≡ −q），bistro-interior 一般旋转取景全暴露——G10.2 骨架期未暴露原因如实登记（骨架期门面只核 schema/digest，应用一致面归双端核验期，缺陷在应用面）。
- **生效面**：harness `milestones/g10/harness/ue_python/g10_param_contract.py quat_contract_to_ue` 按修订式修复；spec 侧规范落点 = spec/visual_comparison.md RXS-0384 L2 之 v1.1 errata 修订行（条款字面 0-byte，errata 追加纪律维持）；应用一致机核 = RXS-0390 应用层探针（pixel_delta ≤ 1e-3 px，G10.5a spec-first 顺位领取）兜底防线——同类缺陷再现即探针 RED。
- **零新号消费**：本章 errata 不消费 RXS/RD/U/RX/MR/CI_step 任何新号（RXS-0390 探针条款消费登记于 number_ledger v1.107 spec-first 批，非本章）。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v0.1 | 2026-08-15 | AI 起草初版（G10.1 治理波）：六面语义冻结——§4.1 帧捕获 EXR float32 格式面（压缩/色彩空间/位深/元数据四闭集 + 往返无损）/ §4.2 FLIP 口径（NVlabs/flip 选型 + pin 三元组策略 + HDR-LDR 双域参数闭集 + 恒等极值）/ §4.3 SSIM/PSNR 口径（Wang 2004 参数闭集 + LDR 域限定 + 恒等极值 + G5 helper 0-byte 声明）/ §4.4 逐像素 diff 报告 schema（双层产物 + 区域统计 + evidence JSON 闭集）/ §4.5 差距清单 schema（UE5 模块枚举闭集 = 目录级 23 + 文件级 + Other 终值，measured delta 溯源，G11 承接锚）/ §4.6 双端确定性契约（参数四节闭集 + 二进制 canonical + SHA-256 + digest 不等不得出 A/B 报告门序）；§5 目标 spec 裁决 = 新建 `spec/visual_comparison.md` + `imageio.md` 追加新章，条款号一律 post-interlock actual-next-free allocation；§9.1 空段待 D-409 回填；零 `src/`、`spec/`、`conformance/`、workflows 改动；零画质/帧率通过线 | Full RFC（Draft） |
| Draft v0.2 | 2026-08-15 | D-409 第 1 轮对抗性评审修法批（17 findings 全部采纳并修）：F1 §4.6 新增「值约定」小节（契约世界系右手系/+Y up/米立约 + UE 厘米/左手系/Z-up/水平 FOV 映射公式 + unit-norm 判定常量 2^-40 + 应用层探针入 M130/M139 evidence）；F2 门序升级三重绑定（当次双端 digest 相等 ∧ == M130 最新 evidence `param_digest` ∧ 同 `base_commit` 同 `session_run_id`；「最新」= `timestamp` UTC + 文件名 UTC stamp 次键）+ M130 补陈旧 pass RED 臂；F3 LDR 臂改派生路径（HDR 帧权威源 + UE MRQ EXR tone curve 启用 fp16→f32 提升 + 双端共用 host 侧 sRGB 编码器 spec 口径单源 + 派生链元数据字段，消「M136/LDR-FLIP 无合法 UE 输入」矛盾；UE 官方 .exr 不应用 sRGB 编码曲线为依据，URL 见 §11）；F4 压缩闭集收窄 {NONE, ZIP}（PIZ/RLE 演进位）+ harness 收窄 evidence 登记 + ZIP 解构成如实登记；F5 HDR 曝光面 start/stop/N + auto + pin 五元组 + ppd 全语料冻结 + 标量/误差图对拍容差分列；F6 unit-norm 判定常量登记为 schema 合法性谓词（不走 budget）；F7 UE 侧载体钉死内嵌 CPython + 字节布局自由量 spec 单源 + correctly-rounded 口径与边界浮点差分语料；F8 EXR 标准属性白名单 + 分端读取策略（rurix strict / ue5 strip-and-log）+ chromaticities 处置条款计划；F9 枚举版本锚 5.8.0 release + 版本差风险标注 + 筛选规则字面化与补收触发条件；F10 对拍图集下界（≥24 图对、五内容类每类 ≥4）+ M138 标定估计器语义（p100 × k，k∈[1.0,3.0]）；F11 `post.view_transform` v1 单值 `aces13` + §3.1 措辞；F12 裁决句限域 Rurix 侧 canonical；F13 err_p95 nearest-rank + 末区域截断规则；F14 MSSIM=mean-SSIM 注；F15 契约三节兼容性注明（post 为扩集不收缩契约判据）；F16 M134 探针图案 RED 臂；F17 同模型评审偏差如实登记；§9.1 回填评审 provenance 与 17 条 disposition 表与总评；§9 增 Q13/Q14/Q15；§5/§6.1/§6.2/§7 同步；状态维持 Draft，翻 Agent Approved 由主会话核后执行。`Assisted-by: Kimi-K3（D-409 修法批）` | Full RFC（Draft） |
| v1.1 | 2026-08-15 | **增补 §12（章 E）：G10.5a 契约四元数共轭公式勘误（errata 纯加性，§1~§11 既有冻结文本 0-byte）**——E1 🔒 §4.6 UE 映射四元数共轭公式勘误：原文「向量部经同一 M 变换、标量部不变（转角保持）」对 det(M) = −1 反射矩阵不成立，正交共轭一般律 R_ue = R(M·axis, det(M)·θ)，修订式 **q_ue = (w, z, −x, −y)**（向量部 −M·v、标量部不变）；实证 = 共轭恒等式随机对拍（缺陷式最大偏差 6.35e0 / 修订式 0.0）+ 黄金个案镜像 + RED 先行测试（tests/test_g10_param_contract.py）修复转 GREEN；cornell-box 180° 取景为缺陷不变量特例、bistro 一般旋转全暴露、G10.2 骨架期未暴露原因如实登记；位置/方向/FOV 三式核验无缺陷维持；生效面 = harness `quat_contract_to_ue` 修复 + spec/visual_comparison.md v1.1 errata 修订行（规范落点）+ RXS-0390 应用层探针兜底防线；本章零新号消费（RXS-0390 消费登记于 number_ledger v1.107 spec-first 批）。`Assisted-by: Kimi-K3（G10.5a 波续）` | Full RFC（Agent Approved 增补 errata） |
