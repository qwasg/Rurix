<!-- Assisted-by: Kimi-K3（D-409 独立评审会话，与起草会话隔离） -->
# RFC-0026 对抗性评审记录（D-409 第 1 轮）

| 字段 | 值 |
|---|---|
| 评审对象 | `rfcs/0026-visual-comparison-metrics.md` Draft v0.1（2026-08-15） |
| 评审者 provenance | `Assisted-by: Kimi-K3（D-409 独立评审会话，与起草会话隔离）` |
| 评审轮次 | 第 1 轮，2026-08-15 |
| provenance 偏差登记 | 评审者与起草者**同模型**（Kimi-K3），独立性 = 会话隔离 + 零共享上下文，不满足 D-409 首选「跨工具/跨模型」字面。按 RFC-0015 §9.1 / number_ledger v1.29/v1.73/v1.90 已登记先例如实偏差登记并效力自限：本评审不替代未来跨工具评审；主会话回填 RFC §9.1 时须保留本行，跨工具评审者可得时建议补一轮 |
| 评审镜头 | ①闭集是否真闭集 ②EXR 自研工程风险诚实度 ③pin 冻结空窗 ④digest 双端等价可实现性 ⑤门序机器可核性 ⑥与 G5~G9 冻结面一致性 ⑦模块枚举可维护性 ⑧伪绿通道 |

## 0. 独立事实核对记录（先于 findings）

本评审不复用起草会话任何结论，以下事实由本会话独立复核：

- **UE 模块枚举闭集**（§4.5）：对 `E:\Kimi_Agent_Taichi Engine 优化计划\references\UnrealEngine\Engine\Source\Runtime\Renderer\Private` 实际枚举——顶层目录 24 个（含 `Tests`），排除后恰为 RFC 所列 **23 值，逐一吻合**；文件级闭集 58 个 `.cpp` **逐一在树，无一虚构**。诚实度核查通过。
- **编号与台账**：`registry/number_ledger.json` namespaces.RFC `next_free=26`（`on_tree_max=25`）✓；`reserved_in_flight[G10]` 段已落（MR 行字面「G10.1 两份均用 Full RFC-0026/0027」）✓。
- **image-io 现状**（§4.1 诚实边界）：`src/image-io/src/lib.rs` 实测 = PPM P6 落地、PNG `UnsupportedFormat`、零外部依赖、零 EXR ✓，RFC 未粉饰。
- **G5 SSIM helper**（§4.3 0-byte 声明）：`src/rurix-render/src/temporal/ssim.rs` 实测 = 8×8 盒式窗、K1=0.01/K2=0.03、L=1.0、RFC-0016 §4.H3/G-G5-7 出处注记 ✓，与 RFC 描述一致。
- **契约判据对照**：G10_CONTRACT §4.2 M130/M134/M135/M136/M137/M139/M140 判据字面与 RFC §2.3/§4/§6.2 逐行比对，语义覆盖无漏项（一处字面差见 F15）。
- **外部事实（联网核查）**：UE 官方导出格式文档明确「**.exr 目标不应用 sRGB 编码曲线**；tone curve 启用时线性值压缩至约 [0,1]，禁用时记录 [0,100+]」，MRQ EXR 输出为 16-bit——支撑 F3；NVlabs/flip 仓库 = BSD 3-Clause（由 NVIDIA Source Code License 改颁），C++/CUDA/Python(nanobind 包 C++) 三分支，上游 README 明示「**输出跨 OS 可能略异……误差图像素并非全同**」——支撑 F5。

## 1. Findings

**F1（blocker，§4.6）世界系/单位/FOV 轴向约定整体缺失——digest 只证「解析一致」，不证「应用一致」。**
问题：参数 schema 冻结了字段名与类型（`position f64×3，世界系`、`orientation_quat w,x,y,z`、`fov_y_deg`、`sun.direction`），但**全 spec/ 无任何渲染世界系条款**（本会话 grep 全 spec：右手系/左手系/Y-up/Z-up/坐标系约定零命中），RFC 首次使用「世界系」却不定义。UE 侧惯例 = 厘米单位、左手系、Z-up、相机 FOV 为**水平** FOV；Rurix 侧惯例（米？右手系？Y-up？）本文未指名。同一组 f64 位模式在两端会被解释成**不同的相机与光照**——而 digest 相等对此完全不可见（这正是 R-G10-6「口径不对齐」的主要成因层，解析层对齐只是其子集）。四元数 w,x,y,z 序冻结只解决分量序，不解决手性下的旋转解释差异；`fov_y_deg` 到 UE 水平 FOV 的换算（含 aspect）无归属面。实现者可各自发明约定而全部通过 digest 门——这是质询①的最大空隙。
建议修法：§4.6 增设「值约定」小节，钉死：长度单位（建议米）、世界系轴向与手性（指名 Rurix 侧既有约定或显式立一条）、四元数旋转方向约定、`fov_y_deg` 垂直口径与 UE 水平 FOV 的确定性换算公式（含 resolution 导出 aspect）；UE 侧应用映射规则要么在此冻结、要么显式委托 RFC-0027 并写明绑定句；M130/M139 evidence 增加**应用层探针**（如标定场景已知标志物的投影像素位置断言），使「应用一致」有机器可核面，而非仅文本解析一致。

**F2（blocker，§4.6 门序 / §6.1）门序 validator 判据存在陈旧 evidence 旁路，且 digest 值无跨门绑定、「最新」未定义。**
问题：门序机器核验被操作化为「validator 核验 M130 最新 evidence `status=="pass"` 且 `phase_g10_5_pass==true`」。反例：T1 时 M130 以参数集 A 双跑 pass 留 evidence；T2 时参数漂移为 B（双端 digest 不等）但**未重跑 M130**，直接跑 M139——validator 所见「最新 M130 evidence」仍是 T1 的 pass，门序判据字面满足，digest 不等的 A/B 报告照样产出。契约 M139 RED 臂「M130 digest 不等仍出报告即 RED」要机器成立，必须在 M139 运行时刻比较**当次参数**的双端 digest，而非仅回看 M130 历史绿；RFC 从未规定这一绑定（§4.4 evidence 虽有 `determinism_contract_digest` 字段，但无任何条款要求它 == M130 evidence 内的 digest）。「最新 evidence」的排序键（mtime？run_id？）也未定义。
建议修法：门序语义改为三重绑定——(a) M139 报告内嵌当次双端 digest 且二者相等；(b) 该 digest == M130 双端核验期 evidence 登记的 digest 值；(c) M130 evidence 与 M139 evidence 同 `base_commit`/同会话链（或显式 freshness 窗）。「最新」按 run_id/时间戳显式定义。§6.2 M130 GREEN 行补「陈旧 pass 冒充当次一致注入即 RED」臂。

**F3（blocker，§4.1 双臂 / §3.1）LDR 臂 UE 侧产出路径与 UE 官方文档冲突，冻结语义无可执行载体。**
问题：§4.1 冻结 LDR 臂 = 「显示域 **sRGB 编码**帧」+ canonical 容器 = EXR float32 + `rurix:transfer="srgb"`。但 UE 官方导出格式文档明示：**EXR 目标不应用 sRGB 编码曲线**——tone curve 启用时 EXR 写的是 tonemap 后**线性** [0,1] 值（且为 fp16），禁用时是未压缩线性 HDR；UE 侧能得到 sRGB 编码帧的唯一在树路径是 MRQ/HighResShot 的 PNG/JPG（8-bit，触发 M134 位深截断 RED 臂）。spike 只实证了 HDR EXR 出图面（bCaptureHDR / MRQ 逐帧 EXR），**LDR 臂 UE 侧产出路径从未被实证、也未入 spike 待验证清单**。按现状冻结，M136（SSIM/PSNR 仅 LDR 域）与 M135 的 LDR-FLIP 半臂没有合法的 UE 侧输入帧。这是「把未验证能力当事实」对 P-09 纪律的破例（位深不对称尚诚实登记了，LDR 产出路径连登记都没有）。
建议修法：三选一并显式冻结——(a) 派生路径：UE 侧 MRQ EXR（tone curve 启用，fp16→f32 提升）+ **双端共用同一 host 侧 sRGB 编码步骤**（编码器进 spec 口径，Rurix 侧 LDR 帧同走该编码器以消除编码差），UE tone curve 与 ACES 1.3 的差照登 `caliber_diff`；元数据诚实标注派生链（`rurix:source_end` 之外需派生标记位）；(b) LDR 臂 UE 侧容器合法化 PNG-16，走修订行放宽 canonical 容器裁决（需同步修 §4.1 裁决句与 M134 判据解释）；(c) 验证 OCIO Color Output 路径产出能力后按实测登记。无论选哪条，先入 spike 待验证清单，未实测前不得在 §3.1 以既成事实口吻叙述。

**F4（major，§4.1 / §9 Q2）压缩合法闭集 {NONE, ZIP, PIZ} 超出自研解码子集 {NONE, ZIP}；ZIP 解码工程量被「最直白」低估。**
问题：合法 canonical 帧允许 PIZ 压缩，但首选自研子集只承诺 NONE/ZIP **解**。UE 侧压缩配置由 harness「强制无损」——{NONE, ZIP, PIZ} 皆无损，即闭集内合法配置即可产 Rurix 工具链读不了的帧；没有任何条款把 UE 侧压缩收窄到可解子集。另：EXR 的 ZIP = 每 16 scanline 块「预测器差分 + 字节重排 + DEFLATE」，自研 ZIP 解 = 手写 inflate（动态 Huffman/存贮块/窗口边界）+ EXR 专属 predictor/reorder 还原——这是 image-io 现状（仅 PPM）到 EXR 之间最硬的一段，RFC 以「零外部依赖纪律与确定性字节流最直白」带过，工程风险被低估（质询②正中）。
建议修法：闭集与能力对齐——压缩闭集收窄为 {NONE, ZIP}，或明文冻结「harness 必须将 UE 侧 EXR 压缩配置限制在自研可解子集内」并给该配置一个 M134/M128 evidence 登记字段；§9 Q2 登记 ZIP 解码工作项的真实构成（inflate + predictor/reorder），PIZ 支持留作修订行演进位。

**F5（major，§4.2 / §9 Q3）HDR-FLIP 曝光参数面与参考实现实际参数面不符；pin 维度漏实现分支/后端/OS；ppd 全语料策略未冻结。**
问题：(a) 参考实现的 HDR-FLIP 曝光参数面 = start/stop exposures + 曝光数 N（auto 模式由**参考图中位亮度**推导 start/stop，v1.7 刚修过 median=0 崩溃），RFC 冻结的 fixed 模式却是**单值** `hdr_exposure_value`——闭集表照抄即不可执行，实现者必须自行发明映射（口径空隙，R-G10-3 风险再生）。(b) 上游仓库明示跨 OS 输出像素级不一致、C++ 与 CUDA 后端亦不同（v1.4 中位数实现变更即改结果）——「pin = commit digest + 构建配置 + 运行参数集」未把**实现分支（cpp tool / cpp header lib / CUDA / python-nanobind）与 OS/工具链**列为显式 pin 维度；而 §4.4 机器 canonical 误差 EXR 直接取 FLIP 误差图，误差图像素级漂移意味着「逐图对拍」的容差面必须覆盖误差图而非仅标量，RFC 未声明。(c) `ppd` 两形态二选一且只要求「登记采用形态」——未冻结全语料单一 ppd 策略，逐场景漂移将使跨场景 FLIP 标量不可比，差距清单聚合进噪声。
建议修法：参数面对齐参考实现（`hdr_exposure_mode ∈ {"auto-from-reference","fixed"}`，fixed 时 `{start, stop, num_exposures}` 必填）；pin 三元组扩为五元组（commit digest + 分支/后端 + OS/工具链 + 构建配置 + 运行参数集）；声明误差图对拍容差与标量对拍容差分列入 M138；ppd 冻结「全语料单一值（或单一推导几何），变更走修订行」。

**F6（major，§4.6）unit-norm 断言无判定口径——实现者被迫发明阈值。**
问题：`orientation_quat`「unit-norm 断言，非单位四元数拒绝」与 `sun.direction`「unit 断言」未给容差。f64 下 ‖q‖² 恰好 == 1.0 几乎永不成立（任何经归一化或手工书写的四元数都在 1±1e-16 量级浮动）；无容差则合法参数被 fail-closed 全拒，有容差则容差由实现者手写——P-09「禁手写阈值」的边界情形（schema 合法性常量 vs M138 标定值）无人裁决。
建议修法：在 §4.6 写明判定式与常量（如 `|‖q‖²−1| ≤ 2^-40`），并显式登记「该常量为 schema 合法性谓词，非 measured 标定值，不走 `g10_budget.json`」；`sun.direction` 同例。

**F7（major，§4.6）UE 侧 digest 产出载体未指定；字节布局自由量未钉；浮点解析等价性要求未声明。**
问题（质询④正中）：(a) 裁决 2 首选 Launcher 版（不可改 C++）下，UE 侧可选载体 = 内嵌 CPython（PythonScriptPlugin：`json`+`hashlib`+`struct`，correctly-rounded 由 CPython 构造保证）、蓝图（无原生 SHA-256 与 f64 LE 位打包，不可行）、host 侧脚本代算（**违反「双端各自解析」语义本身**——digest 将不再证明 UE 进程内的解析结果）。RFC 只说「schema 解析器双端各一份」，载体不定则 M130 语义不定。(b) 二进制 preimage 的版本前缀具体值、类型标签字节值、键排序规则（code point/字节序）、嵌套对象与数组编码均未钉——双端各自实现即分叉，「同构 RXS-0305 CanonW 律」是族约束不是字节级单源。(c) 「浮点 round-to-nearest 同值性」依赖双端解析器均 correctly-rounded，未声明为口径要求，也无差分语料（-0.0、次正规、2^53 边界、长十进制、1e-310 等）。
建议修法：钉死 UE 侧载体 = UE 进程内嵌 CPython 实现（或显式接受语义降级并改写「双端解析一致」定义）；字节布局全部自由量指定由 §5 拟落 spec 条款单源冻结、双端同字面对拍；M130 GREEN 增「跨端解析器差分语料（边界浮点集）逐位一致」断言。

**F8（major，§4.1）EXR 元数据读取侧策略与标准属性白名单未定义——真实 UE 帧可能被自家 strict 纪律拒收。**
问题：闭集纪律写的是「闭集外**禁写**」，但 §4.0 不变量 2 是 fail-closed 通用「schema 外字段确定性拒绝」。UE5 写出的 EXR 会携带 OpenEXR 标准头属性（`pixelAspectRatio`、`screenWindowCenter`、`screenWindowWidth` 等）及可能的 UE 自有属性；这些不在 §4.1 表的必填清单内，「EXR 标准属性」作为类别未枚举边界。`chromaticities` 被列为必填，但 UE 写出器是否落该属性未验证（spike 未覆盖）。严格读入则真实 UE 帧大概率被拒；放行则闭集纪律名存实亡——两种实现都「合法」，口径空隙。
建议修法：枚举允许的 EXR 标准属性白名单（含上述三项）；按 `rurix:source_end` 分别定义读取策略（rurix 帧 strict 拒绝闭集外属性；ue5 帧闭集外属性 strip-and-log 并随 provenance 登记）；`chromaticities` 在 UE 侧的落盘情况入 spike 待验证清单，缺失时的处置（拒绝/补写并登记）写进条款计划。

**F9（major，§4.5 / §9 Q8）模块枚举的版本锚 ≠ 对标基线版本；「对标相关」筛选规则未冻结，闭集边界主观。**
问题：枚举实测自 **ue5-main @4517329fa 快照**，而立项裁决 2 的对标基线 = **Launcher 5.8.0 正式版**；spike 风险 4 已自认「快照 ≠ release 标签」，Launcher 版无源码树，枚举对 5.8.0-release 标签树的有效性从未复核。文件级闭集自称「对标相关顶层单文件」——筛选规则没有字面化：顶层实有 ~130 个 `.cpp`，闭集收 58 个，而 `SceneRendering.cpp`（场景渲染总入口）、`ScreenPass.cpp`（后处理框架中枢）、`SceneTextures.cpp`、`SceneViewState.cpp` 等**对标最相关的核心文件反在闭集外**，归属只能落 `Other` 或近似条目，M140 的归属质量被结构性稀释。（诚实面已核查：闭集内所列 23 目录 + 58 文件逐一在树，无虚构——问题不在事实，在版本锚与筛选规则。）
建议修法：G10.2 出图环境落地时按 5.8.0-release 标签树（GitHub 只读）复核枚举并只追加登记差量；把筛选规则字面化（或显式承认 curated 子集 + 触发条件：当 `Other` 行或近似归属行出现时，按只追加程序补收对应文件级枚举值）；§9 Q8 同步修订。

**F10（major，§4.2/§4.3/§6.2 + M138）对拍图集与标定程序语义无下界——measured 形式可满足、判别力可任意稀释（伪绿通道）。**
问题：「自实现与参考实现逐图对拍」的图集无最小规模与内容类约束——一张平色图即满足字面；M138 标定程序（估计器形态、安全系数、样本集、分位数口径）语义完全不冻结，「手写阈值冒充标定即 RED」管不住「用 max×10 估计器合法地标定出一个永不过载的容差」。P-09 的 measured-only 在此只剩类型正确，M135/M136 的 GREEN 判别力可由标定程序选择任意调节（质询⑧正中）。
建议修法：冻结对拍图集下界（最小张数 + 内容类清单：高频边缘/平滑渐变/噪声/高亮截断/色彩孤立区等）与标定估计器语义（统计量、安全系数、样本集引用 digest），可放 spec 条款但 RFC 须点名归属；§6.2 M135/M136 GREEN 行引用该下界字面。

**F11（minor，§4.1 vs §4.6 / §3.1）`post.view_transform` 四值枚举与「LDR 臂 Rurix 侧固定 = ACES 1.3」相互矛盾；§3.1「双端同一 view transform 字面」措辞与 caliber_diff 设计冲突。**
问题：§4.6 枚举允许 `aces13/aces20/agx/neutral` 四值，§4.1 却「固定 = ACES 1.3」——选 agx 合法还是不合法，两节各支持一种读法。§3.1「显示域、双端同一 view transform 字面」在 §4.1/§4.6 的设计（Rurix=ACES1.3、UE=默认 Filmic、差登 caliber_diff）下不成立；若本意是「参数字面统一、实现各异」，现措辞会被读成「双端变换相同」。
建议修法：仿 `exposure.mode` v1 仅 `"manual"` 先例，声明 v1 合法值仅 `aces13`（余值保留演进位）；§3.1 改为「双端共用同一参数字面，实现差登记 caliber_diff」。

**F12（minor，§4.1）「canonical = float32 每通道」裁决句全域绝对化，与 `rurix:bit_depth` 允许 `"float16"`（UE 帧）/ Q11 不对称登记存在表述张力。**
建议修法：裁决句限定为「Rurix 侧 canonical float32；UE 侧位深以 harness 实测登记（Q11）」，消除 strict 读者眼中的自相矛盾。

**F13（minor，§4.4）`err_p95` 百分位方法（nearest-rank / 插值）与区域网格边缘规则（分辨率不被 16 整除时末区域 w/h）未冻结。**
问题：三面重算一致 golden（§6.2 M137）与跨组件复核依赖同一 p95 口径；网格边缘不定则 `pixel_count` 对账漂移。
建议修法：指定百分位口径（建议 nearest-rank 并写公式）与末区域截断规则（`w/h` 取实际剩余像素）。

**F14（minor，§4.3）「（MSSIM）」术语歧义。**
问题：skimage 文档以 MSSIM 指 mean-SSIM（本 RFC 口径），但图形学文献 MSSIM 常指 Wang 2003 multi-scale SSIM——未加注易误导实现者去对齐 multi-scale 变体。
建议修法：注「mean-SSIM，非 multi-scale（MS-SSIM）」。

**F15（minor，§4.6 vs 契约 §4.2 M130 行）契约字面为「相机/光照/时间」三节，RFC 冻结四节（增 `post`），扩集未注明与契约字面的关系。**
问题：扩集本身合理（view_transform 双端互证必需入 digest），但三方对账纪律下，「判据字面不在本 RFC 重定」（§2.1）与「实际多冻一节」之间缺一句解释，易被审出「契约三节 vs RFC 四节」的漂移嫌疑。
建议修法：§4.6 注明「契约 M130 字面三节为最低断言集；`post` 节为本 RFC 为 LDR 臂 view transform 互证的扩集，不收缩契约判据」。

**F16（minor，§6.2 M134）M134 GREEN 无活性绑定：恒定合成帧即可满足「往返无损 + 元数据齐备」字面。**
问题：M128 有「预置假帧冒充真出帧即 RED」，M134 无对称臂——捕获帧与真实渲染输出（tonemap 前 HDR 抽头）之间无探针互证；M139 虽兜底，但 M134 作为 G10.4 独立 P0 门可单独伪绿。
建议修法：§6.2 M134 增 RED 臂「渲染输出探针图案未位级出现于捕获 EXR 即 RED」（注入已知像素图案经管线后核验）。

**F17（minor，§9.1 / 流程）本轮评审 provenance 与起草同模型，偏差须随 findings 一并回填。**
问题：D-409 首选跨工具/跨模型评审者；本评审为同模型独立会话。仓库已有同工具族偏差登记先例（ledger v1.73/v1.90、RFC-0025 §9.1），按先例办理即可，但 RFC §9.1 空段回填时不得把本评审写成「跨工具」。
建议修法：主会话回填时逐字登记本表「provenance 偏差登记」行；跨工具评审者可得时补一轮。

## 2. 核查通过项（未见问题的面，防 findings 比例误读）

- 质询③（pin 冻结空窗）：**不构成 blocker**。§10 已把「参考实现具体 pin 值」列为明确非 stable，「策略冻结 + G10.4 实测钉死」符合 P-09（预写未实测 digest 才违规）；选型前提（开源可构建）经本会话核查 = BSD 3-Clause + CMake 可构建，风险低。真正缺陷是 F5 的参数面错误，不是 pin 时序。
- 门序「不得 waived」（Q10）、恒等图对自证（§3.2）、M130 单 key 双 phase（与 ACCEPTANCE_MAP §3.3 逐字一致）、`kind` 两值分列、`Other` 计数防滥用、场景全集零空行对账、0-byte 声明（RXS-0369~0373 / RXS-0114~0117 / `temporal/ssim.rs`）、错误码零预造策略、TEMPLATE-RFC 结构完整性（全节齐备、§9.1 空段合规待回填）——逐项核查通过。
- SSIM/PSNR 的 LDR 域限定与 HDR 不适定论证（§4.3/§7）技术正确；`"inf"` 字符串例外避开 JSON 非法 `Infinity` 是正确设计。

## 3. 总评

**approve-with-changes**（修订后可批准，非现状可批准）。

- F1/F2/F3 为 **blocker**：三者都位于本 RFC 的核心语义面（双端契约的值约定与门序绑定、LDR 臂 UE 侧产出路径），且都属于「补一段冻结文字即可修」的缺口而非架构性错误——但语义冻结 RFC 的批准门槛恰是这类空隙清零，故 Approved 前必须改正文并同步 §9 Q 表。
- F4~F10 为 **major**：闭集/能力对齐、参数面与参考实现对齐、判定常量、载体与字节布局钉死、读取策略、枚举版本锚、标定/对拍下界——均应在本次修订一并落实（多数可与 blocker 同段合写）。
- F11~F17 为 **minor**：可同批修，或按只追加修订行处置，不阻断批准。
- 若主会话对 blocker 判定有异议，最低可接受的替代处置：F1/F2/F3 逐条给出显式 disposition（驳回须附理由，且 F3 涉及与 UE 官方文档冲突的事实面，驳回前须先拿出 5.8 实测反证入 spike 待验证清单）。
- 严重度映射建议（供 §9.1 表 high/med/low 口径）：blocker→high，major→high/med 由主会话酌定，minor→low。
