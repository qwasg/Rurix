<!-- Assisted-by: Cursor Agent（D-409 对抗性评审程序） -->
# RFC-0044 对抗性评审报告（D-409）

| 字段 | 值 |
|---|---|
| 评审对象 | `rfcs/0044-geometry-device-realization.md`（G27 几何 device 化，Draft v0.1） |
| 评审日期 | 2026-08-25 |
| 评审程序 | D-409 对抗性评审（独立评审会话，零共享上下文——本会话未接触 RFC 起草会话的任何中间产物，仅以树内文件为输入） |
| provenance 声明 | 评审员与起草方为**同环境单一模型家族**，可能存在同源盲区偏差，如实登记，本报告效力自限于该偏差面之外 |
| 已逐字核对锚 | RFC-0044 全文；`src/rurix-render/src/geometry/hzb.rs`（`HzbPyramid::build`/`test_rect`/`exact_rect_occluded`/`DepthConvention` 全量）；`src/rurix-render/src/bin/g20_hzb_probe.rs`（夹具/digest 口径全量） |
| 未在本会话逐字核对锚（效力自限清单） | `milestones/g25/g25_campaign_handover_registry.json` M61/M98-l4 行字面、`milestones/g20/g20_cluster_streaming_p4_gap.json` 四行字面、`milestones/g20/G20_P2_DECISIONS.md` §1、`rfcs/0034` 重判表、`gi/fallback_chain.rs`、`world/hlod.rs`、`registry/deferred.json` RD-039——凡 finding 依赖上述锚字面者，均标注「待核对」，其 disposition 须以逐字引锚方式兑现 |

**已核实为准确的字面转述**（正向登记，缩小争议面）：RFC §1.1/§1.2 对 host 参考臂的转述与 `hzb.rs` 实现逐字一致——`build` 的 `div_ceil(2).max(1)` 非 2 幂 ceil 减半、`min(·, 上级边长−1)` 越界 clamp 复采、`farther` 的 reverse-Z=min / standard-Z=max、`test_rect` 的 `x0=floor(clamp·w0)` clamp [0,w0−1]、`x1=ceil(clamp·w0)` clamp [1,w0]−1、`span=max(x1−x0+1, y1−y0+1)`、`while (span>>mip)>2` 逐字、mip 上界 `min(mips.len()−1)`、窗口 `x0>>mip` 起 `min(x1>>mip, mip_w−1)` 止、`is_farther` reverse-Z `<` / standard-Z `>`,均与 `hzb.rs` L67–123 吻合；§1.2 转述的 probe 夹具（193×117 非 2 幂、`det_rects(800)`、双约定臂、digest = sha256(判定序列 ‖ 金字塔字节)）与 `g20_hzb_probe.rs` L17–19/L99–110 吻合。

---

## Findings

### F1（major）「零容差/位级相等」的浮点语义面未完全钉死

**指认字面**：§1.1「**零容差协议**：纯 min/max 比较归约、零算术舍入 ⇒ device mips 与 host `HzbPyramid::build` 逐级**位级相等**」；§1.4「全 f32、分支判定 min/max 算术门白名单形」。

**挑战**：「纯比较归约无舍入面」对归约 kernel 本身基本成立,但位级相等宣称还静默依赖以下未写入 RFC 的前提,任一失守即位级漂移:

1. **min/max 的 NaN/±0 语义**：Rust `f32::min/max`（IEEE minNum 族,单 NaN 时返回另一操作数）与 SPIR-V `OpFMin/OpFMax`（操作数含 NaN 时结果未定义）以及 `±0.0` 平局的返回值选择,在 IEEE 层面均无跨实现唯一解。夹具深度场落在正规正数域（≈[0.08, 0.93+]）确实避开了 NaN/−0/非规格数,但 RFC 把「零容差」表述为协议性质而非**域条件性质**——协议成立的域假设（无 NaN、无 −0、无 denormal）未声明。
2. **kernel 侧比较形**：若 device kernel 以 `select(a<b, a, b)` 白名单形实现 farther,则 NaN/±0 行为由比较算子唯一决定,上述不确定面即被钉死——但 §1.4 只说「白名单形」,未指明 farther 必须落此形而非 builtin min/max。
3. **test kernel 并非零算术**：`test_rect` 含 `clamp(u,0,1)·w0` 的 f32 乘法。Vulkan 对 fp32 乘法要求 correctly-rounded,故与 host 一致——但前提是编译管线不做收缩/快速数学改写（NoContraction 等价纪律）与不启用 denormal flush 改变语义。§1.4 继承的 g18 头注纪律是否覆盖「禁收缩」未在本 RFC 字面出现。
4. **深度场生成含 `sin()`**（`g20_hzb_probe.rs` L26/L30 libm 超越函数）:必须 host 生成后上传、device 不得复算。§1.1「mip0 = 全分辨率拷贝，不经 kernel」隐含此意但未把「夹具数据 host 单点生成」写成硬条款。

**建议 disposition**：§1.4 增补「浮点语义钉死」条款——(a) farther 归约在 kernel 侧的白名单实现形逐字指定（比较+select 形,禁 builtin min/max 或证明其目标语义等价）;(b) 声明零容差协议的域前提（夹具深度域为正规正数,无 NaN/−0/denormal）并由 host 侧机核断言该域;(c) 禁浮点收缩/快速数学改写入确定性协议字面;(d)「夹具数据（含超越函数产物）host 单点生成上传,device 零复算」入 §1.2。

### F2（minor）rect 像素化 f32→u32 转换与退化 rect 行为未声明域保证

**指认字面**：§1.2「像素化 `x0 = floor(clamp(u_min,0,1)·w0)` clamp [0,w0−1]、`x1 = ceil(clamp(u_max,0,1)·w0)` clamp [1,w0] − 1」。

**挑战**：f32→u32 转换（SPIR-V `OpConvertFToU` 对负值/越界未定义）被转换前的 clamp 字面钉死,这一点成立——**前提是 kernel 逐字复刻 clamp-先-转换的顺序**,RFC 有字面,合格。但存在一个未声明的退化面:当 `u_min=u_max=u` 且 `u·w0` 恰为整数 k≥1 时,`x0=k`（clamp 后）而 `x1=k−1`,`x1−x0+1` 在 u32 上下溢——host debug 构建 panic、release 回绕,device 侧回绕（SPIR-V 定义为模回绕),两侧行为形不同。现行 `det_rects` 夹具半宽 ≥0.02 不会产生零宽 rect,故本期判据不受影响,但「逐 rect 复算 host `test_rect` 字面」的宣称在退化输入上并非良定义。

**建议 disposition**：§1.2 增一句域声明:「夹具保证 rect 非退化（uv_max−uv_min ≥ 正下界),x0≤x1/y0≤y1 由构造保证;退化 rect 行为不在对拍面内」。

### F3（minor）判据③与判据②的逻辑蕴含关系未说清（独立面 vs 冗余面）

**指认字面**：§1.2 判据③「**零假阳性硬不变量**：device 判 Occluded ⇒ `exact_rect_occluded`……必同判遮挡——机核逐 rect 复验」。

**挑战**：`exact_rect_occluded` 是 host 裁判函数,而 host `test_rect` 的零假阳性已由 g20 门在同一夹具上证得。故在正常臂上,判据②（device 判定序列与 host 逐 rect 全等）**逻辑蕴含**判据③——②绿则③不可能红,③作为独立判据面是冗余的。③的真实价值仅在两处:(a) 作为 RED 臂⑤「更远向扰动」的检出机构,证明裁判机器本身活着;(b) 诊断分离——②红且③绿 = 纯漂移无正确性损失,②红且③红 = 正确性破口。RFC 把③并列为六判据之一而未声明此蕴含结构,存在把「冗余绿」叙述成「独立证据面」的措辞风险。

**建议 disposition**：§1.2 判据③补一句:「③在正常臂上被②蕴含（host 零假阳性已证）,其独立价值 = RED 臂⑤的裁判机构活性面 + ②失败时的诊断分离面,不计为独立通过证据」。

### F4（major）篡改 RED 臂注入面未量化,「必异/必检出」是非构造性宣称;组合 digest 使检出空洞化

**指认字面**：§1.2 判据⑤「mip 纹素扰动注入——向『更近』方向扰动 → 判定序列 digest 必异……向『更远』方向扰动制造假阳性候选 → ③ 裁判函数必检出」。

**挑战**（三层）:

1. **量化缺失**：扰动的目标 mip 级、纹素选择方式、幅度（1 ULP？固定 δ？）全部未定。任意选点的 1 ULP「更近」扰动,若该纹素不被任何 rect 在其选定 mip 上采样、或扰动未跨过任何 rect 的 `nearest_depth` 阈值,则**零判定翻转**——「判定序列必异」为假。
2. **digest 口径歧义使检出空洞化**：g20 digest = sha256(判定序列 ‖ **金字塔字节**)。若 RED 臂沿用组合 digest,则任何纹素扰动都经由金字塔字节面平凡地改变 digest——检出的是注入本身而非判定敏感性,RED 臂退化为「哈希函数在工作」的空证。§1.2 写「判定序列 digest 必异」暗示判定-only 口径,但未显式与 g20 组合口径切割。
3. **「更远」臂同病**：制造假阳性候选要求存在某 Visible rect 其 farthest 恰在阈值邻域,且扰动落在该 rect 的采样纹素窗内并跨阈——任意扰动不保证构造出假阳性,「③必检出」随之悬空。

**建议 disposition**：把 RED 臂改写为**构造性注入协议**:从判定 trace 反查——选定一个 Occluded rect（更近臂）/一个 Visible rect（更远臂）,定位其选定 mip 的 ≤2×2 采样纹素窗,施加跨越该 rect `nearest_depth` 阈值的定向扰动（幅度 = |farthest − nearest_depth| + margin）;检出口径显式钉为**判定序列-only digest**（更近臂）与③裁判假阳性计数 >0（更远臂);「必」字宣称限定于此构造性前提之下。

### F5（minor）M61 防冒充硬线结构足够,建议补机器可核字段与措辞纪律

**指认字面**：§2.3「①半边命中不得单独构成重判启动……盘点脚本对『启动』判定的输入面 = 三项布尔合取,任何单项/两项命中均落 maintain-no-go 分支」。

**挑战**：合取硬线 + 「必要非充分」字面 + §6 evidence schema 映射,结构上足以封死「HZB device 化半边命中被叙述成部分重判成立」。剩余薄弱面在叙述层:RFC-0034 尾追加的 G27.2 行若措辞含糊（如「重判条件部分满足」),仍可能被下游读作半启动。

**建议 disposition**：evidence schema 强制三个独立布尔（`m_a_green` / `p4_cleared` / `mesh_hw_measured_evidence`）+ 显式 `rejudgment_started: false` 字段;RFC-0034 追加行措辞钉为「①命中、②③未齐,不启动,维持 maintain-no-go」定式。

### F6（major）「三项合并闭集」相对 handover 锚字面的合法性未逐字引证【待核对】

**指认字面**：§2.1「重判条件字面 = 『cluster P4 差距闭集清零 + HZB device 化落地后只追加再判』两半 + RFC-0034 原判据……——合并为**三项机器盘点闭集**」。

**挑战**：handover 锚字面若只含两半,则把 RFC-0034 原判据升格为第三合取项是对锚的**收紧修订**——收紧方向虽然反冒充（更难宣称启动),但同样构成未经授权的锚改写:若未来两半真齐而 measured 证据缺位,三项合取会阻断锚字面本应触发的重判。RFC 未逐字引 handover 行原文,也未论证「RFC-0034 原判据属启动条件」还是「属重判启动后的审理内容」——这两种读法给出不同的合法决策树。本会话未核 `g25_campaign_handover_registry.json` 字面,此条以 RFC 自身转述为挑战对象,效力自限。

**建议 disposition**：§2.1 逐字引 handover M61 行与 RFC-0034 原判据原文;二选一并给出字面依据——(a) 锚本身已含三项 ⇒ 引文自证;(b) 锚为两半 ⇒ 决策树改为「两半齐 → 启动」,第三项降为启动后审理面（§5.1 承接锚窗内的判据),不得阻断启动。

### F7（major）M98-l4「任一半命中→启动」与 M61「三项全齐→启动」判定形状相反,锚字面依据未引证【待核对】

**指认字面**：§4.1「重判条件字面 = 『HLOD proxy 追踪 device 腿落地 **+** L4 计数器接入选档 evidence』两半」;§4.3「**任一半**命中 → 重判程序启动」;对照 §2.2「三项**全齐** → 重判程序启动」。

**挑战**：两处锚转述使用同一连接词「+」,却导出相反的判定形状——M61 取合取（全齐),M98-l4 取析取（任一半）。若 M98-l4 的 G20.5 终判字面实为合取,则 §4.3 比锚**宽松**,恰是 §2.3 所防的冒充风险镜像:未来任一半命中即可宣称「重判启动」并开启 L4 实现窗。本期两半均预期未命中,析取/合取殊途同归于 maintain,结果不受影响——故为 major 非 blocker;但决策树形状会被本 RFC 冻结给未来窗口,不能留待事后解释。本会话未核 handover 与 G20_P2_DECISIONS §1 字面,效力自限。

**建议 disposition**：§4.1 逐字引 G20.5 M98-l4 终判原文;若为合取,§4.3 改「两半全齐 → 启动」;若确有析取依据（如原判据把两半登记为各自独立的重判触发器),引文入 RFC 并加一行说明与 §2.2 形状差异的锚源依据,消除「同形连接词、异形决策树」的表观矛盾。

### F8（minor）P4-2「依赖解除事实登记」与「维持 open」并存自洽,但 M-c 重判的启动依据应显式

**指认字面**：§3.2「M-a 绿件 ⇒ reeval_anchor **半边命中**……（登记≠该行兑现……）」;§3.1「表级 reeval_anchor = 『HZB device 化落地 + 剔除 pass 反馈链出现』」。

**挑战**：「依赖解除≠现面兑现」的并存逻辑本身自洽——依赖解除是使能事实,open 反映实现缺位,二者正交。真正的缝隙在启动依据:表级 reeval_anchor 是两半合取,本期仅半命中,而 M-c 已然执行逐行 reeval。若不声明依据,这构成「锚未满足即重判」的先例,可被未来窗口引用来提前重判其他锚定表。实际上 M-c 是 campaign 程序性登记重判（G27 法定输入驱动),不是锚触发重判——RFC 未写这句。

**建议 disposition**：§3 开头加一句:「本重判为 G27 campaign 程序性登记重判（handover 法定输入驱动),非表级 reeval_anchor 触发;anchor 半命中状态仅作事实登记,不构成锚触发先例」。

### F9（minor）RD-039 尾追加方式合规,建议补 append-only 机核证据

**指认字面**：§3.5「本期按只追加纪律尾追加 G27.3 行……**断档口径注明**……id/title/reason/backfill_condition 四字段 0-byte」。

**挑战**：断档口径写入**新行内部**（指针式注明「G15~G26 承接留痕在各期 P2 表」）而不回填旧行,符合只追加纪律;四字段 0-byte 声明覆盖了字段级不动面。剩余缝隙:history 数组旧行的不动性只有「尾追加」一词承载,无机核。本会话未核 deferred.json 现行 RD-039 字面（RFC 称 history 止于 G14.1 2026-08-19 行),效力自限。

**建议 disposition**：M-c evidence 增登 RD-039 history 追加前后的旧行区间 digest（或 git-diff 断言仅尾部新增行),把「只追加」从措辞升格为机核面;追加行 schema 与既有 history 行同形。

### F10（major）out-of-scope 漏列 HZB 两阶段的第二阶段;§3.5「登记 M-a 兑现」措辞有过述风险

**指认字面**：RFC 头表承接列「RD-039『HZB 两阶段』device 化长线锚」;§3.5「（登记 M-a 兑现 + P4 重判 disposition……）」;§5 七项 out-of-scope 清单。

**挑战**：`hzb.rs` 头注（L1–2）字面为「本模块兑现 mod.rs 头注『HZB 两阶段 P3 预留』的**第一阶段** host 面」——RD-039 锚是**两阶段**遮挡剔除（第二阶段 = 上一帧金字塔重投影初剔 + 本帧误遮挡重测闭环）。M-a 交付的 build/reduce + test 双 kernel 只是第一阶段的 device 对拍面;第二阶段既未在 §1 交付,也未在 §5 out-of-scope 列项附承接锚——是清单漏项。更要害的是 §3.5 把 RD-039 追加行措辞写为「登记 M-a 兑现」:若不加限定,单阶段 kernel 会被叙述成「HZB 两阶段」锚的兑现,与本 RFC 处处强调的零冒充纪律自相冲突。次级漏项（严重度低,并列登记）:真实光栅深度源接入（夹具为合成深度场,可由 §5.7 生产接线间接覆盖,但宜点名）;reverse-Z/standard-Z 之外的深度域（如 [0,1] 之外的 D 域/无限远平面）——`DepthConvention` 双枚举即闭集,宜一句声明「深度域闭集 = ZO 双约定,其余域无锚不立项」。

**建议 disposition**：§5 增列「**HZB 两阶段闭环的第二阶段**（上一帧重投影初剔 + 误遮挡重测）:承接锚 = P4-2 反馈链/生产接线窗」;§3.5 追加行措辞改为「登记 M-a **第一阶段 device 对拍兑现**（两阶段锚余第二阶段,承接锚见 RFC-0044 §5)」;深度域闭集与合成深度源各加一句声明。

### F11（minor）digest 口径「判定位序列」的序列化字面未钉死

**指认字面**：§1.2 判据②「digest 口径 = sha256(判定位序列 ‖ 金字塔字节)（g20 同模）」。

**挑战**：g20 probe 实现（`g20_hzb_probe.rs` L84–103）实际是**每 rect 1 字节** u8（Occluded=1/Visible=0）拼接金字塔各 mip 依序 f32 **小端**字节。「位序列」措辞会诱导 device 侧实现为 bit-pack（100 字节 vs 800 字节),digest 必然错配且错配会被误读为判定漂移。「g20 同模」有救,但对拍承重面的序列化不应靠「同模」二字间接钉。

**建议 disposition**：§1.2 把 digest 序列化字面展开:「每 rect 1 字节 u8(1=Occluded/0=Visible) 按 rect 序拼接 ‖ mips 依 0..N 级序、级内行主序、f32 LE 字节」——与 probe 实现逐字对齐。

---

## 总评

**findings 合计 11 条:blocker 0 / major 5（F1、F4、F6、F7、F10）/ minor 6（F2、F3、F5、F8、F9、F11）。**

无 blocker:RFC 的总体架构——host 参考臂冻结 + bin-local device adapter 加性面、三态协议、三套重判程序的只追加纪律、防冒充硬线、RXS-0396/0359 不混同声明——结构成立,已核锚（`hzb.rs`/`g20_hzb_probe.rs`）的字面转述准确度高。

**不建议以现文本直接翻 Agent Approved。** 建议**有条件通过**,条件为五条 major 全部 disposition:

1. **F1/F4**（判据面硬度):零容差协议补浮点语义钉死条款与域前提;RED 臂改构造性注入协议并钉死检出 digest 口径——否则 §1 两条核心宣称（位级相等、「必异/必检出」）在对抗输入下不成立,门实现时会被迫在无 RFC 依据处自行立法。
2. **F6/F7**（锚字面依据):handover M61/M98-l4 行与 RFC-0034 原判据**逐字引文**入 RFC,消解「三项合并」的合法性缺口与两套决策树「同连接词、反判定形状」的表观矛盾——此二条依赖本会话未核锚,disposition 时引文即自证或自纠。
3. **F10**（零冒充自洽):§5 补 HZB 第二阶段 out-of-scope 项,§3.5 RD-039 追加行措辞改「第一阶段兑现」口径。

六条 minor 可与 major disposition 同批以文本增补方式处理,不单独构成翻态障碍。全部 disposition 落文后,本评审程序不反对 Draft → Agent Approved。

*（本报告为 D-409 独立评审会话产物;「待核对」标注条款的最终效力以 disposition 阶段逐字引锚为准。）*
