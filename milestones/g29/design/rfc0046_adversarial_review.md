<!-- Assisted-by: Cursor Agent（G29.1 治理波 D-409 独立对抗性评审程序） -->
# RFC-0046 对抗性评审报告（D-409）

| 字段 | 值 |
|---|---|
| 评审对象 | `rfcs/0046-material-device-integration.md` v0.1（Draft，2026-08-25） |
| 评审日期 | 2026-08-25 |
| 评审程序 | D-409 独立对抗性评审（零共享上下文启动；先例格式 = `milestones/g26/design/rfc0043_adversarial_review.md` / `milestones/g28/design/rfc0045_adversarial_review.md`） |
| 评审范围 | RFC-0046 全文（§1~§6 + 修订记录）；只产出本报告，不改 RFC 与任何其他文件 |
| findings | **11 条：blocker 1（F2）/ major 3（F3、F6、F8）/ minor 7（F1、F4、F5、F7、F9、F10、F11）** |

## 1. 独立性与 provenance 声明

本评审以独立评审员程序启动：评审会话与 RFC 起草会话零共享上下文，评审过程未与起草方交互，全部判断以树内文件实测为准（逐锚重读事实源，不信任 RFC 自述）。**效力自限声明**：评审员与起草方共享同一仓库宿主与同一 agent 基础设施，非组织学意义上的独立第三方；本报告效力 = D-409 程序内对抗面留痕，不构成外部审计。与先例（RFC-0043/0045 评审报告尾部 provenance 偏差效力自限声明）同律。

## 2. 事实源核对面（评审程序第 2 步）

逐锚实测结果（RFC 引用 → 树内实况）：

| # | 锚 | 实测结果 |
|---|---|---|
| 1 | `material/slab.rs` L46-56 闭式公式 | **一致**：L46-56 = `total_reflectance`（`tc = 1.0 - rc`；`denom = 1.0 - rc * ab`；`denom <= 0.0 → 1.0`；`rc + tc * tc * ab / denom`）。§1.1 公式字面与乘除序（`tc*tc*ab/denom` 后加 rc）逐字符一致 |
| 2 | slab.rs 白炉恒等/能量上界/单测容差 | **一致**：`white_furnace_identity`（1e-9）、`energy_never_exceeds_unity`（`furnace_audit(64,64)`，1e-9）、恒等式 1e-9 在案。注意：host 单测网格为 64/32/24 档，**16641 非单测口径**（见 F4） |
| 3 | `g22_slab_probe.rs` 16641 口径 | **RFC 口径正确**：probe `GRID=128` → `furnace_audit` 循环 `0..=grid`、`rc = i/grid` → samples = (128+1)² = **16641 = 129×129**，与 §1.1「rc=i/128、i,j ∈ 0..129」位级同构。但 probe 头注释自称「128×128 参数网格」（段数/格点误述），构成在案漂移源（F4） |
| 4 | `g22_svt_gap.json` SVT 四行 | **一致**：SVT-1~SVT-4 四行，status 全 open，disposition=defer |
| 5 | `g22_ktx2_disposition.json` KTX2 三行 | **一致**：KTX2-1~KTX2-3 三行，disposition=defer，DDS 链维持字面在案 |
| 6 | `g22_work_graphs_probe_results.json` | **WG/DGC 一致**：`VK_AMDX_shader_enqueue: false` + `work_graphs_verdict: not-available`；`dgc_tokens` 三键 = `VK_EXT_device_generated_commands` / `VK_NV_device_generated_commands` / `VK_NV_device_generated_commands_compute` 全 true。§4.2 所列三名字面逐字符一致（仅列序不同，无实质）——**指定挑战⑤核验通过无洞**。但该 JSON **无 FSR 字段**，§4.1 把「FSR 3.1.5 maintain 在案」归入此文件系锚定过载（F7；FSR maintain 真实字面在 `G22_P2_DECISIONS.md` G22-N4 行） |
| 7 | `registry/deferred.json` RD-041 | **一致**：status=open；history 末行 = 2026-08-24 G22.2~G22.3 处置行（§3.4「止于 G22 处置行」宣称属实）；backfill_condition 含 KTX2 `PagedResource::transcode` 留口与 WG「pass 内部提交单元可替换」接缝字面（§5.2/§5.3 承接锚对得上） |
| 8 | MaterialClosure 32B 与 reserved 守卫 | **一致且守卫实存**：`graph/types.rs` L329-343 八槽 u32 = 32B + `frozen_layout_sizes` 单测断言 32；reserved RED 守卫实体 = `material/side_table.rs::check_closure_face_untouched`（`reserved != [0,0]` → `Err(FieldOverreach)`）+ RXS-0372 单测 `closure_32b_face_untouched_and_overreach_red`（`reserved=[1,0]` RED 试验在案）。§2.1「机核 RED 守卫在案」宣称**核实为真**，但守卫文件未被 RFC 指认、且该文件恰与本 RFC「侧表」撞名（F8、F10a） |
| 9 | RFC-0039 out-of-scope 锚 | **一致**：0039 §1.6「out-of-scope：slab device kernel/侧表集成……各附承接锚」+ §2「material/closure 单层生产面……0-byte」。承接命中宣称属实 |
| 10 | `g25_campaign_handover_registry.json` 两行 | **一致**：RD-041-slab（closed-go，g26_anchor=「device kernel/侧表集成波」）+ RD-041-svt-ktx2-wg（defer，g26_anchor=「各差距表 reeval_anchor 字面」） |
| 11 | 同律引用网络 | **编号全部对得上**：RFC-0045 评审 F3（f32 精度承载）/F8（复测三态）/F9（manifest pattern 忠实性）/F10（门态映射）与 RFC-0043 评审 F4（RED 偏置量化兜底）各自内容与 0046 引用语境匹配；但 **F9 同律承接不完整**（F6）。RXS-0203/RX6026（`spec/vulkan_backend.md` 等）、RXS-0357（`spec/global_illumination.md` 等）条款号实存 |
| 12 | 先例路径 | `kernels/g28_restir.rx` 实存（§1.7 g28 同模宣称成立）；`rurix-rt` `vendor_upscale` 面实存（§4.3 锚成立）；`G29_CONTRACT.md` §4.2 五行 P0 实存且与 §6 门表语义同构（但 `skipped_dev_env` 字面不在契约内，F10b；契约 M-a 行「同 host 单测口径」自身失准，F4） |

**核对面结论**：15+ 处锚中 13 处逐字面属实，2 处失准（F7 FSR 锚归属、F4 口径漂移源未设防）。无虚引条款号、无虚构文件。

## 3. Findings

### F2（blocker）角点 rc=ab=1 算术门形结构性 bug：denom=0 时 gate=0 走公式支 → 0/0 = NaN，与 host 分支语义割裂；且 gate 合成式缺失，算术 mix 无法屏蔽 NaN 臂

**指认字面**：§1.1「`denom ≤ 0 ⇒ R = 1`（rc=ab=1 极限全反射角点——device 以算术门形承载分支：`gate = ((0 − denom)·1e30).min(1).max(0)`，denom > 0 时 gate=0 精确）」；§1.1「NaN/Inf 构造上不可达」。

**挑战（确认成立，实弹）**：

1. **角点在网格内必被求值**：16641 = 129×129 网格含 i=j=128 → rc=ab=1.0。f32 格点值下 `rc·ab` 尾数 ≤14 位精确、`denom = 1 − rc·ab ≥ 0` 精确，且 `denom = 0 ⇔ rc = ab = 1`（若 rc<1 则 rc·ab ≤ rc < 1）——角点是全域唯一 denom=0 样本，**不是不可达角落，是必踩样本**。
2. **门在唯一需要它的样本上失效**：denom=0 → `gate = ((0−0)·1e30).min(1).max(0) = 0` → 选公式支。公式支分子 `tc·tc·ab = 0`（tc=1−rc=0），分母 denom=0 → **`0/0 = NaN`**（IEEE-754 与 Vulkan FP32 同）→ `R = rc + NaN = NaN`。host 同点走 `denom <= 0.0` 分支返回 1.0。RFC 括注只保证「denom > 0 时 gate=0 精确」，对 denom=0 行为**只字未提**——正是分支存在的全部理由。
3. **后果二形，皆不可接受**：判据①聚合 |device − host| 时，(a) NaN 参与比较 → p100=NaN → 门炸；(b) 更险：若 harness 用 Rust `f64::max` 或同语义 GPU 归约聚合（`max(NaN, x) = x`，NaN 被吞），p100 貌似正常 → **角点错误静默假绿**（配套加固见 F3）。
4. **次级缺陷 a**：denom ∈ (0, 1e-30) 时 gate ∈ (0,1) 非饱和（部分混合）。本域不可达（全域最小正 denom = 1 − 127/128 = 1/128 ≈ 7.8e-3），不构成实弹，但门形对「denom ≤ 0 ⇒ 1」宣称语义不忠实。
5. **次级缺陷 b**：RFC 未给出 gate 与公式支的合成表达式（`R_final = ?`）。若按算术 mix `R = formula·(1−gate) + 1·gate`，即使 gate 修到角点=1，`NaN·0 = NaN` 仍污染结果——**算术门形结构上无法屏蔽 NaN 操作数**，必须 select 语义或消除 NaN 源。

**建议 disposition（修法，按优先序）**：

- **修法 A（推荐：删门 + 分母安全化，一行闭合）**：`R = rc + tc*tc*ab / max(denom, 1e-30)`（ε 亦可取 f32 最小正规数 ≈1.175e-38；1e-30 富余）。论证：域内 denom ∈ {0} ∪ [1/128, 1]，max 不扰动任何正 denom 样本（1/128 ≫ 1e-30，无一样本值改变）；角点分子=0 → `0/1e-30 = 0` 精确 → `R = rc = 1.0` **位级等于 host 分支值**。gate 整体删除：直线代码、无分支、NaN 构造不可达宣称由此成立。§1.1「逐字抄录」表述随批改为「公式面同源 + 角点以分母安全化承载 host 分支（角点值位级一致论证入 RFC 字面）」。
- 修法 B（保门形）：`gate = ((ε − denom)·1e30).min(1).max(0)`，ε 取 1e-4（< 最小正 denom 1/128 的 1/78）⇒ denom=0 → gate=1 饱和、denom ≥ 1/128 → gate=0 精确；**必须同时**分母安全化（否则 mix 的 NaN 臂污染，见缺陷 b）。两处改动，劣于 A。
- 修法 C（若 rurixc 有 select/条件表达式）：`select(denom <= 0, 1.0, 公式)`——OpSelect 未选臂不作算术污染；稳妥仍配分母安全化。RFC 以算术门形行文恰暗示语言面可能无此原语，采 C 前须先核语言面。

任一修法落地后，§1.2 白炉行、判据①③的角点行为方为良定。**未修不得开工**。

### F3（major）判据①缺「全样本有限性」一等断言——NaN 可被 max 聚合静默吞掉，对拍门存在假绿路径

**指认字面**：§1.4①「逐样本对拍：16641 样本逐样本 |device R − host R| p100 ≤ 标定容差」（全清单无 is_finite 断言）。

**挑战**：F2 展示了 device 产 NaN 的现实路径；即使 F2 修掉，未来任何 kernel 回归（驱动差异、编译器重结合、后续扩展改公式）再引入非有限值时，p100 聚合语义决定成败：Rust `f64::max(NaN, x) = x` **静默吞 NaN**，`partial_cmp`/`sort` 各有陷阱。判据面把「p100 ≤ tol」当唯一防线，等于把有限性外包给聚合函数的实现细节——这是结构性漏洞，不是实现细节。

**建议 disposition**：§1.4 增列一等判据「⓪ 输出有限性：device 输出缓冲 16641 样本全量 `is_finite`，任一非有限 → 硬 FAIL（先于对拍聚合执行）」；§2.3 侧表臂同律增列。RED 臂/白炉行登记均在其覆盖下。一句字面，堵死全部 NaN 静默路径。

### F1（minor）§1.2 白炉行「预期 ≤ 数 ULP」来源未论证、登记面不设线的兜底逻辑未写出——诚实性成立，论证缺失

**指认字面**：§1.2「device dev 如实登记（预期 ≤ 数 ULP；dev 值入 evidence 不冒充解析 0）」；§1.4②「不设 device 通过线——登记面」。

**挑战（指定挑战①）**：用户挑战面提出 f32 求值 `tc²/tc` 有舍入故 dev 可能非零。评审逐位分析：白炉行网格值 rc=i/128 下 tc=1−rc 精确（≤8 位尾数）、tc² 精确（≤14 位）、denom=tc 精确、数学商 tc²/tc = tc **可表示**——若除法正确舍入（IEEE）则 device R ≡ rc+tc = 1.0 位级、dev ≡ 0。**dev 非零的真实来源不是「f32 舍入」泛指，而是 Vulkan FP32 `OpFDiv` 仅保证 ≤ 2.5 ULP（不要求正确舍入）+ 驱动可能 FMA 收缩/重结合**。RFC「预期 ≤ 数 ULP」量级碰巧合理，但来源未写，且三点缺失：

1. host 侧「白炉恒等 dev=0（f64 解析级）」实为**位级 0**（网格值下逐步精确，同上论证在 f64 全部成立），可断言 `== 0.0`，比「解析级」更强——宣称弱于事实，白给对抗面一个「解析级≠位级」的攻击口。
2. 「不设通过线」孤立读像裸奔，但白炉行（ab=1 列）⊂ 16641 网格，且 host 白炉行 R 位级 ≡ 1.0 ⇒ 白炉 dev ≡ 该列对拍差 ⇒ **已被判据①标定容差通过线传递覆盖**。这层覆盖论证 RFC 未写出——写出后「登记面不设线」才站得住（登记 ≠ 无界）。
3. 可选加固：由 Vulkan FP32 精度模型（OpFDiv ≤2.5 ULP、加/乘正确舍入）可解析推出白炉行 dev 保守上界 ~4e-7；设一条富余断言（如 dev ≤ 1e-5，比 RED 兜底 0.025 紧三个数量级）属「解析级程序产」谱系（推导入 RFC 字面），不违 RFC-0039「阈值零手写」纪律。

**建议 disposition**：§1.2 补「dev 非零来源 = Vulkan FP32 OpFDiv ≤2.5 ULP 非正确舍入 + 驱动收缩」半句；host 侧改「位级 0（可断言 ==0）」；§1.4② 补「白炉行已被判据①通过线传递覆盖（host 白炉行 R 位级 ≡1.0），登记面不另设线不构成无界」一句；解析 ULP 上界断言可选随批。

### F4（minor）16641=129×129 与 g22 probe 口径核实一致，但两处在案漂移源未设防（probe 注释「128×128」误述 + 契约「同 host 单测口径」失准）

**指认字面**：§1.1「16641 样本 = 129×129 网格，rc = i/128、ab = j/128，i,j ∈ 0..129——host 单源生成一次原字节上传」。

**挑战（指定挑战③）**：实测 `g22_slab_probe.rs`：`GRID=128` → `furnace_audit` 循环 `for i in 0..=grid`、`rc = i as f32 / grid as f32` → `samples = (grid+1)² = 16641`。**RFC 口径与 probe 实际执行位级同构，核实通过**。但漂移源两处在案：① probe 头注释自称「128×128 参数网格白炉审计」（实为 128 段/129 格点，16384 ≠ 16641）；② `G29_CONTRACT.md` §4.2 M-a 行写「16641 样本网格同 host 单测口径」——host 单测实际用 `furnace_audit(64,64)`/(32,96)/(24,48)，16641 是 **probe 口径**非单测口径。实现者或后续复核者按任一误述施工/质疑，互核即歪。

**建议 disposition**：§1.1 补半句钉死血缘：「= `g22_slab_probe` GRID=128 经 `furnace_audit` (grid+1)² 格点口径（probe 头注释『128×128』系段数/格点误述，勿按 16384 施工）」；契约行失准本 RFC 不改（0-byte 纪律），登记入候选决策表或修法批说明，提请治理波勘误。

### F5（minor）侧表 rc_k = k/15·0.95 的 0.95 上限规避 rc=1 角点——意图未声明；侧表参数「host 单源生成」纪律未从 §1.1 继承

**指认字面**：§2.1「rc_k = k/15·0.95、ab_k = (15−k)/15」。

**挑战（指定挑战④）**：k ∈ 0..16 → rc ∈ [0, 0.95]、ab ∈ [0, 1]；k=0 槽 (rc=0, ab=1) 踩白炉线但 **无任何槽踩 rc=1**，配合 §2.2 逐槽白炉互核（ab=1 变体，denom = 1−rc ≥ 0.05）——侧表臂全程规避 denom→0 病态区。这大概率是有意设计（侧表臂验证「多槽寻址 + 逐槽求值」而非角点语义，角点由 M-a 主网格独担），但 RFC 零声明，对抗面可读作「侧表臂靠参数挑选回避了 F2 角点雷」。另 §1.1 网格有「host 单源生成一次原字节上传，device 不重算格点」纪律，§2.1 侧表**无对应字面**——`k/15·0.95` 在 f32 下求值序位级敏感（k/15 先算再乘，vs k·0.95/15；15 非 2 幂必有舍入），若 host/device 各算一遍，参数本身位级不一致，对拍差混入参数差。

**建议 disposition**：§2.1 补两句：「0.95 上限系有意规避 denom→0 角点区（角点语义覆盖由 §1 主网格 M-a 独担，F2 修法后位级良定）」；「侧表 SSBO host 单源生成一次原字节上传，device 不重算槽参数（§1.1 网格同律）」。

### F6（major）§3.2 对 RFC-0045 F9 同律的承接不完整——缺「pattern 表常量承载 + 锚字面派生」两条，≥2 pattern 防单不防歪

**指认字面**：§3.2「逐行 ≥2 pattern 树内检索 + pattern↔锚关键词映射表入 evidence（manifest 忠实性纪律，RFC-0045 F9 同律）」。

**挑战（指定挑战⑥）**:0045 评审 F9 的原文挑战正是「**用一条与锚字面无关的狭窄 pattern 制造非空清单即可『合规』得出零命中，manifest 纪律对此零防御**」，其建议 disposition 有三件：① manifest 逐项字段钉死（pattern 表/检索根/逐 pattern 命中数）；② **pattern 表由盘点脚本字面承载（常量表）且须含该分项锚字面的派生关键词**；③ pattern 表随 evidence 落档。0046 §3.2 承接了 ③（映射表入 evidence）并新增「≥2 pattern」，但 **① 的字段钉死与 ② 的常量表承载 + 派生纪律双双缺失**。「≥2」只防单 pattern 敷衍，两条无关 pattern 同样合规——七行 reeval 的结论可信度（M-c 门核心）系于此。至于 pattern 闭集是否须 RFC 级钉死：先例（0045 修法）定盘在**脚本级常量表 + RFC 级纪律字面**，无须把七行 pattern 全文抄入 RFC；但纪律两条必须 RFC 级写死，否则实现自由度直接吃掉 F9 忠实性。

**建议 disposition**：§3.2 补「pattern 表由 reeval 脚本字面承载（常量表，禁运行时构造）；逐 pattern 须为对应 gap 行 `gap`/`anchor` 字面的派生关键词（映射表机核可比对）；evidence 逐行载 {pattern 表, 检索根, 逐 pattern 命中数}」——0045 F9 disposition 三件全承接，七行口径适配。

### F7（minor）§4.1 输入面锚定过载：「FSR 3.1.5 maintain 在案」不在 g22_work_graphs_probe_results.json 内

**指认字面**：§4.1「输入面：`milestones/g22/g22_work_graphs_probe_results.json`——WG `VK_AMDX_shader_enqueue` absent 实测（not-available 终态）+ DGC 三扩展 available 实测 + FSR 3.1.5 maintain 在案」。

**挑战**：实测该 JSON 仅含 `work_graphs_tokens`/`dgc_tokens`/两 verdict/`dgc_host_surface`/`log_path`，**无任何 FSR 字段**。FSR 3.1.5 maintain 的真实字面在 `milestones/g22/G22_P2_DECISIONS.md` G22-N4 行（「+ FSR maintain」「兜底 = DGC 现面 + FSR 3.1.5 维持」）与 RFC-0039 §1.5。§4.3 的 FSR 盘点若按 §4.1 锚去 probe JSON 里找「maintain 在案」字面，必空手而归；复核者亦会以此指控虚引。RFC 上游行「G22 锚源四件」含 G22_P2_DECISIONS.md §3，正确锚就在自家清单里。

**建议 disposition**：§4.1 拆锚：「WG absent + DGC available 实测 = probe JSON；FSR 3.1.5 maintain 在案 = `G22_P2_DECISIONS.md` §3 G22-N4 行字面」。

### F8（major）「侧表」与在树生产面 material/side_table.rs 撞名零防线；0-byte 机核清单不含 side_table.rs/table.rs——生产面误触无机核兜底

**指认字面**：§2.1「侧表形态：N = 16 材质槽 slab 参数侧表（bin 内合成独立 SSBO……）」；§1.7 冻结清单（slab.rs + material/closure）；§2.3④（仅 graph/types.rs 0-byte）。

**挑战**：树内已有 `material/side_table.rs` = RFC-0025 生产资产面 **MaterialSideTable**（Burley 扩散 profile/Marschner 参数集，「按材质槽 ID 索引」接入 closure 求值，自带 canonical 编解码 + digest 签名设施）。本 RFC 的 bin-local「slab 参数侧表」与之**同名同形**（都叫侧表、都按材质槽索引），RFC 全文零防混淆声明。误触诱因是现实的：实现者见生产面有现成「材质槽侧表 + 编解码 + digest」设施，把 slab [rc,ab] 槽挂进 MaterialSideTable 资产通道即触碰生产面。而机核面：§1.7 锁 `slab.rs` + `material/closure`，§2.3④ 锁 `graph/types.rs`——**`side_table.rs`、`table.rs` 等 material/ 其余生产面全在机核清单之外**，「顺手」改动无 git-diff 门兜底（reserved RED 守卫只防 closure 字段越权，不防 side_table.rs 文件本身被改）。另注意：F10a 指出 §2.1 宣称的 reserved RED 守卫本体恰好就住在 side_table.rs 里——把守卫文件留在机核清单外，等于允许实现波改写守卫本身。

**建议 disposition**：① §2.1 补防混淆声明：「本 RFC『侧表』= bin-local slab 参数 SSBO（bin 内合成，不落资产），与 `material/side_table.rs` 生产资产侧表（RFC-0025 Burley/Marschner 通道）零关系零触碰，禁挂接其编解码/digest 设施」；② §1.7 与 §2.3④ 机核清单扩为「`src/rurix-render/src/material/` 全目录 vs g28-closed 0-byte git-diff 机核」（device 臂新增面已按 §1.7 钉死在 `kernels/` 与 bin-local adapter，material/ 目录本 RFC 零合法改动，全目录锁干净且无误伤）。

### F9（minor）out-of-scope 漏项：slab 方向依赖/各向异性、层数 >2 语义扩展与生产集成混列、时域材质参数动画、bin-local 侧表资产化转正

**指认字面**：§5 全节（七项）。

**挑战（指定挑战⑦）**：§5 覆盖 SVT/KTX2/WG/多层生产集成/host 重写/性能/吸收档七项，但：

- a) **方向依赖/各向异性 slab**：slab.rs 模型自述「方向-半球反照率层级」（标量，无角度分辨）；真实 Substrate 类 coating 的 Fresnel 角度依赖/粗糙度方向分布是最近邻扩展面，未列承接锚——它既不属「多层生产集成」也不属「吸收档」，落在 §5 缝隙里。
- b) **§5.4「多层 slab 生产集成」双义**：「双层 slab 的生产集成（closure/侧表转正）」与「层数 >2 语义扩展（N 层栈传输矩阵/递归闭式 = 新 host 参考面）」是两个承接对象，现字面混列一行，后者的锚（何时立 N 层参考臂）实际缺位。
- c) **时域材质参数动画/参数流送**：slab.rs `lerp` 自述「参数域连续性载体」，时变材质动画是其显式下游，未列。
- d) **bin-local 侧表资产化转正**：§2 侧表若来日入资产管线（接 MaterialSideTable 通道），是独立立项面，未列（与 F8 防混淆声明互补：现在零关系，来日转正须显式立项）。

**建议 disposition**：§5 补 3~4 行，各附承接锚（a=真实分层材质资产需求 + host 参考臂先行；b=拆分 §5.4 为两行；c=动态材质资产面出现；d=slab 参数资产化需求 + Full RFC）。

### F10（minor）锚精度杂项四件：reserved 守卫未指认文件、skipped_dev_env 挂错宣称主体、params 区间写法、「四字段」未点名

**指认字面与挑战**：

- a) §2.1「reserved 位机核 RED 守卫在案」——核实为真（见核对面 #8），但守卫实体（`material/side_table.rs::check_closure_face_untouched` + RXS-0372 RED 单测 + `graph/types.rs::frozen_layout_sizes`）RFC 未指认，复核定位成本高；且「机核」效力经 cargo test 达成（运行时校验 + 单测），非独立 CI 脚本，宜写明效力形态。
- b) §6「与 G29_CONTRACT §4.2 同构（M-a/M-b schema 含 `skipped_dev_env` 合法态）」——契约 §4.2 实测无 `skipped_dev_env` 字面（仅「SKIP 如实登记不冒充」），该字段名系 RFC/schema 侧新钉；现行文易误读为契约字面,拆句可免。
- c) §1.1 params「[2..8] reserved」——8 槽数组下 2..8 为排他区间（6 槽），与「[0][1]」下标记法混排有一字之歧，写「[2..8) 六槽」或「2..=7」免歧。
- d) §3.4「四字段 0-byte」——未点名哪四字段（deferred 条目七字段：id/title/reason/raised_in/owner_milestone/status/backfill_condition）；对照 RD-040 先例行惯写「status 维持 open，backfill_condition 0-byte 不回写」，宜写全清单（如 title/reason/raised_in/backfill_condition 四不可变字段 + status 维持 open 另句）。

**建议 disposition**：四处各一句字面修补，随修法批落。

### F11（minor）RED 臂与判据③交互未钉：red_bias=0.05 注入后白炉行 R≈1.05，若能量上界跨臂执行必假红

**指认字面**：§1.4③「全样本 device R ≤ 1 + 容差」与 §1.4⑤「red_bias = 0.05 注入 → 对拍必超容差」。

**挑战**：RED 臂注入后白炉列 R ≈ 1.05 > 1 + 容差（容差 ~1e-7 量级）。若实现把判据③放进公共校验路径跨臂执行，RED 臂必触③假红（RED 臂的预期红是「对拍超容差」，不是「能量上界破」——红的理由错了，F10 同律的「门态映射」精神即分支语义不得混淆）。RFC 未写臂间判据归属。

**建议 disposition**：§1.4⑤ 补一句：「RED 臂仅评判据①检出（对拍必超容差 + tol<0.025 兜底），判据②③④不跨臂执行；主臂判据面与 RED 臂判据面分列 evidence」。

## 4. 指定挑战覆盖映射

| 挑战 | 结论 | finding |
|---|---|---|
| ① 白炉行 device dev「≤ 数 ULP」诚实性/上界断言 | 部分成立：登记面诚实性靠判据①传递覆盖成立但论证缺失；「数 ULP」来源（Vulkan OpFDiv 2.5 ULP 非正确舍入）未写；host 侧宣称弱于事实（位级 0） | F1（minor） |
| ② 角点 rc=ab=1 门形 vs host 分支 | **确认成立，结构性 blocker**：denom=0 → gate=0 → 0/0=NaN，host 同点 =1.0；修法 A = 删门 + `max(denom, 1e-30)` 分母安全化（角点位级还原 1.0） | F2（blocker）+ F3（major 配套） |
| ③ 16641 与 g22 probe 实际口径 | 核实一致（GRID=128 → (grid+1)²=129²）；但 probe 注释「128×128」与契约「同 host 单测口径」两处漂移源未设防 | F4（minor） |
| ④ 侧表 0.95 上限规避角点 | 规避属实且大概率有意，但零声明；另侧表参数 host 单源纪律未继承 | F5（minor） |
| ⑤ DGC 三扩展名与 probe 字面 | **核验通过无洞**：三名逐字符一致（仅列序异），「以 g22 probe 结果字面为准」自防线在案 | 无 finding |
| ⑥ 七行 reeval pattern 集钉死层级 | RFC 级 pattern 闭集不必（先例 = 脚本级常量表），但 0045 F9 修法的「常量表承载 + 锚字面派生 + 字段钉死」未承接——同律引用不完整 | F6（major） |
| ⑦ out-of-scope 漏项 | 四件：方向依赖/各向异性、>2 层语义扩展锚缺位、时域参数动画、侧表资产化转正 | F9（minor） |
| 评审自查（挑战面之外） | FSR 锚过载（F7）、侧表撞名 + 机核清单缺口（F8）、锚精度四件（F10）、RED 臂×判据③（F11） | F7/F8/F10/F11 |

## 5. 总评

**结构面评价**：RFC-0046 的承接网络（RD-041 两行 → 五门映射 → 同律谱系 F3/F4/F8/F9/F10）与冻结面纪律（host 参考臂 0-byte/32B 布局零触碰/g22 表只读/RD-041 只追加）扎实；事实源 15+ 锚仅 2 处失准且均可一句修正；标定容差协议（measured×2 + p100=0 方登零容差）与 RED 量化兜底（tol < RED_BIAS×0.5）完整承接乃至强化了 0043 F4 修法。判档（Full RFC、渲染器库面零新语言语义）与三态/门态映射纪律无可挑剔。

**但 §1.1 角点算术门形是一颗实弹 blocker**：129×129 网格必踩 rc=ab=1 角点，现行 gate 字面在该点选中公式支产生 0/0=NaN，与 host 分支值 1.0 语义割裂；且 NaN 经 max 聚合存在静默假绿路径（F3）。这不是理论边角——是判据①第一次真跑就会命中的样本。所幸修法极小（推荐修法 A：删门 + 分母安全化一行，角点位级还原 1.0，全部次级缺陷连带消除）。

**结论：建议 Agent Approved，条件 = F1~F11 逐条 disposition 落档（v0.2 修法批，修订记录只追加）**，其中：

- **F2 + F3（角点门形 + 全样本有限性一等断言）为 blocker 级必修，v0.2 未落此二条不得翻 Approved、不得开工 G29.2**；
- F6（F9 同律补全）、F8（侧表防混淆 + material/ 全目录机核）为实现波开工前必须落的字面；
- F1/F4/F5/F7/F9/F10/F11 可随同批落档。

翻 Agent Approved 由治理会话按 D-409 程序执行；本报告 provenance 偏差效力自限声明（§1）随档。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-25 | D-409 独立对抗性评审报告落档：11 findings（blocker 1 / major 3 / minor 7），建议条件 Agent Approved（F2+F3 必修先于开工） |
