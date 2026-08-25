<!-- Assisted-by: Cursor Agent（G29.1 治理波） -->
# RFC-0046 — G29 材质 device 集成——slab device kernel 兑现 + 侧表供参加性臂 + SVT/KTX2 差距重判程序 + WG/DGC capability 复测程序

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0046（立项时实测 `registry/number_ledger.json` namespaces.RFC next_free=46 顺位领取） |
| 状态 | **Agent Approved**（2026-08-25；D-409 对抗性评审 11 findings〔1 blocker/3 major/7 minor〕全部 disposition，v0.2 修法批落档——评审全文 `milestones/g29/design/rfc0046_adversarial_review.md`；blocker F2 角点门形以修法 A 消除） |
| 判档 | Full RFC（slab device kernel 兑现为材质闭合 device 登记面；渲染器库面零新语言语义条款，G5 先例） |
| 承接 | G29.2 M-a/M-b + G29.3 M-c/M-d + G29.4 M-e（RD-041-slab「device kernel/侧表集成波」锚兑现 + RD-041-svt-ktx2-wg 重判锚兑现；RFC-0039 out-of-scope 承接命中） |
| 上游 | `milestones/g25/g25_campaign_handover_registry.json`（RD-041-slab / RD-041-svt-ktx2-wg 两行 = G29 法定输入）、`registry/deferred.json` RD-041、RFC-0039（host 参考臂 + out-of-scope 锚）、G22 锚源四件（`G22_P2_DECISIONS.md` §3 + `g22_svt_gap.json` + `g22_ktx2_disposition.json` + `g22_work_graphs_probe_results.json`） |

## 1. slab device kernel 兑现语义（M-a）

1. **kernel 面（逐样本单 invocation；F2 blocker disposition = 修法 A：删门 + 分母安全化）**：`kernels/g29_slab.rx`——host `material/slab.rs::total_reflectance` 的公式面 device 兑现（公式面同源 + 角点以分母安全化承载 host 分支）。公式字面（`slab.rs` L46-56 同源，host f64 内算 → device f32 承载见 §1.3）：`tc = 1 − rc`；`denom = 1 − rc·ab`；**`R = rc + tc·tc·ab / max(denom, 1e-30)`（直线代码零分支零门）**。角点位级一致论证：域内 denom ∈ {0} ∪ [1/128, 1]（rc<1 时 rc·ab ≤ rc < 1，全域最小正 denom = 1/128 ≈ 7.8e-3 ≫ 1e-30 ⇒ max 不扰动任何正 denom 样本）；唯一 denom=0 样本 = rc=ab=1 角点（129×129 网格 i=j=128 必踩），该点分子 `tc·tc·ab = 0`（tc=0 精确）⇒ `0/1e-30 = 0` 精确 ⇒ `R = rc = 1.0` **位级等于 host `denom ≤ 0 ⇒ 1.0` 分支值**——NaN/Inf 构造不可达宣称由此成立。输入 = 参数网格 SSBO（16641 样本 = 129×129 网格，rc = i/128、ab = j/128，i,j ∈ 0..129——**血缘钉死（F4）**：= `g22_slab_probe` GRID=128 经 `furnace_audit` (grid+1)² 格点口径〔probe 头注释「128×128」系段数/格点误述，勿按 16384 施工〕；host 单源生成一次原字节上传，device 不重算格点；域前提 = [0,1] 有限值闭集）+ params（[0]=n_samples [1]=red_bias [2..=7] 六槽 reserved 恒 0——F10c 区间写法钉死）；输出 = 逐样本 R（f32 SSBO）+ red_bias 加性注入位。
2. **白炉行 device 复现（F1 disposition：来源与覆盖论证补全）**：ab=1 列（129 样本）device R 值逐样本登记——host 白炉恒等 dev = **位级 0（可断言 == 0.0）**：网格值 rc=i/128 下 tc/tc²/denom=tc 逐步精确、数学商 tc²/tc = tc 可表示，f64 正确舍入下 R ≡ rc+tc ≡ 1.0 位级；device dev **如实登记**——dev 非零的真实来源 = **Vulkan FP32 `OpFDiv` 仅保证 ≤ 2.5 ULP（不要求正确舍入）+ 驱动 FMA 收缩/重结合可能性**（非「f32 舍入」泛指）；dev 值入 evidence 不冒充解析 0。
3. **精度承载（RFC-0045 F3 同律）**：rurixc device 路径 f64 为 RX6026 构造性拒绝（RXS-0203 L1）⇒ kernel 全 f32；host 参考值 = `total_reflectance()` f64 直调 ⇒ host↔device 走标定容差对拍协议（threshold = measured × 2.0 冻结 k 程序产禁手写；**实测 p100=0 方可登记零容差零条目**——纯四则运算下 f32 位级可达性存疑〔host f64 中间精度差〕，以标定腿实测定盘，禁先验宣称）。
4. **判据面**（程序产禁手写）：
   - ⓪ **输出有限性一等断言（F3 disposition）**：device 输出缓冲 16641 样本全量 `is_finite`，任一非有限 → **硬 FAIL（先于对拍聚合执行）**——封死 NaN 经 max 聚合静默吞掉的假绿路径（Rust `f64::max(NaN, x) = x` 陷阱）；
   - ① **逐样本对拍**：16641 样本逐样本 |device R − host R| p100 ≤ 标定容差（标定腿两跑位级一致程序产）；
   - ② **白炉行登记**：ab=1 列 device dev 最大值如实登记——**覆盖论证（F1）**：白炉行 ⊂ 16641 网格且 host 白炉行 R 位级 ≡ 1.0 ⇒ 白炉 dev ≡ 该列对拍差 ⇒ **已被判据①标定容差通过线传递覆盖**，登记面不另设线不构成无界；
   - ③ **能量上界 device 复核**：全样本 device R ≤ 1 + 容差（host `energy_never_exceeds_unity` 同律的 device 面）；
   - ④ **device 双跑位级一致**：固定输入两跑输出缓冲 digest 位级相等；
   - ⑤ **kernel-bias RED 臂**：red_bias = 0.05 注入 → 对拍必超容差；量化兜底断言 **tol < 0.025**（RED_BIAS × 0.5，RFC-0043 F4 同律）；**臂间判据归属（F11 disposition）**：RED 臂仅评判据①检出（对拍必超容差），判据⓪②③④不跨臂执行（注入后白炉行 R ≈ 1.05 触③属预期注入效果非门红——分支语义不得混淆），主臂判据面与 RED 臂判据面分列 evidence；
   - ⑥ **spirv-val**：编译产物验证通过。
5. **三态协议**：无 Vulkan → `SKIP DEV_ENV_DEGRADE` 退 0 如实登记不冒充（schema `skipped_dev_env` 合法态；`RURIX_REQUIRE_REAL=1` 下翻硬红）；host 腿恒跑。
6. **确定性协议**：禁 atomic、逐样本独立、输出直写、全 f32、固定输入双跑位级一致（RXS-0357 L2 同律继承）。
7. **host 参考臂冻结（F8 disposition：扩至 material/ 全目录）**：`src/rurix-render/src/material/` **整目录** vs `g28-closed` **0-byte**（git-diff 机核——slab.rs/closure/side_table.rs/table.rs 等生产面全部圈入；本 RFC 对 material/ 目录零合法改动，device 臂新增面全在 `kernels/` 与 bin-local adapter，全目录锁干净无误伤；reserved RED 守卫本体 `side_table.rs::check_closure_face_untouched` 因此同受机核保护）；device 臂 bin-local adapter（`g28_restir_device` 集成路径同模），不回写 host、不接线生产车道。

## 2. 侧表供参加性臂（M-b，bin-local）

1. **侧表形态（F5/F8/F10a disposition）**：N = 16 材质槽 slab 参数侧表（bin 内合成独立 SSBO：逐槽 [rc, ab] 对，参数跨槽确定性变化非退化——rc_k = k/15·0.95、ab_k = (15−k)/15；**0.95 上限系有意规避 denom→0 角点区**：角点语义覆盖由 §1 主网格 M-a 独担〔F2 修法后位级良定〕，侧表臂验证多槽寻址与逐槽求值面；**侧表 SSBO host 单源生成一次原字节上传，device 不重算槽参数**——k/15 非 2 幂分母求值序位级敏感，§1.1 网格同律）；**防混淆声明（F8）**：本 RFC「侧表」= bin-local slab 参数 SSBO（bin 内合成，不落资产），与 `material/side_table.rs` 生产资产侧表（RFC-0025 Burley/Marschner 通道 MaterialSideTable）**零关系零触碰**，禁挂接其编解码/digest 设施；**MaterialClosure 32B 冻结边界钉死**：`graph/types.rs` 32B 布局与 reserved 拓扑位**零触碰**（reserved 位机核 RED 守卫在案——守卫实体 = `material/side_table.rs::check_closure_face_untouched`〔reserved ≠ [0,0] → Err(FieldOverreach)〕+ RXS-0372 RED 单测 + `graph/types.rs::frozen_layout_sizes` 断言，效力形态 = cargo test 运行时校验 + 单测（F10a 指认入档）；侧表臂不写 reserved 位、不扩 closure 布局；拓扑位消费须显式修订行另立，本 RFC 不触）。
2. **device 逐槽求值**：kernel 复用 §1（输入换侧表 SSBO + 槽索引寻址）——逐槽 R 求值 + 逐槽 host `total_reflectance` 直调对拍（p100 同 §1.3 容差协议）+ **逐槽白炉互核**：每槽以 ab=1 变体重算一次（host/device 双端），白炉 dev 逐槽登记；
3. **判据面**：⓪ 输出有限性一等断言（全槽 is_finite 硬 FAIL 先行，F3 同律）①逐槽对拍 p100 ≤ 容差 ②逐槽白炉 dev 登记 ③双跑位级 ④`graph/types.rs` 与 `material/` 全目录 0-byte 机核（git diff vs g28-closed，§1.7 同面）。
4. **过述防线**：侧表臂 = 「device kernel/侧表集成波」锚的 bin-local 兑现登记——**不构成** closure 生产集成或多层 slab 生产化（closure 单层生产面 0-byte 维持，RFC-0039 兜底字面）。

## 3. SVT/KTX2 差距重判程序（M-c）

1. **输入面**：`milestones/g22/g22_svt_gap.json` SVT 四行 + `g22_ktx2_disposition.json` KTX2 三行（均 0-byte 只读）——七行逐行 reeval。
2. **逐行 reeval（manifest 忠实性纪律，RFC-0045 F9 同律三件全承接——F6 disposition）**：逐行 ≥2 pattern 树内检索——**pattern 表由 reeval 脚本字面承载（常量表，禁运行时构造）**；**逐 pattern 须为对应 gap 行 `gap`/`anchor` 字面的派生关键词**（pattern↔锚关键词映射表机核可比对）；**evidence 逐行载 {pattern 表, 检索根, 逐 pattern 命中数}**（字段钉死）。任一行现面兑现 → closed-go；零实现 → 维持 defer（预期七行全维持）。
3. **产物**：`milestones/g29/g29_svt_ktx2_rejudgment.json`（七行逐行 disposition + 检索清单 + 映射表）；g22 两表 0-byte 不回写。
4. **RD-041 history 只追加**：history 止于 G22 处置行（2026-08-24），G23~G28 承接留痕在各期 P2 表与 g25 handover——本期尾追加 G29.3 行（断档口径注明）；**不可变字段清单（F10d 点名）**：title/reason/raised_in/backfill_condition 四字段 0-byte + status 维持 open 另句 + owner_milestone 不回写；append-only 机核 = `check_deferred_append_only` 同律（vs G29.0 不可变 ref）。

## 4. WG/DGC capability 复测程序（M-d）

1. **输入面（F7 disposition：拆锚）**：WG absent + DGC available 实测 = `milestones/g22/g22_work_graphs_probe_results.json`（`VK_AMDX_shader_enqueue: false` → not-available 终态 + `dgc_tokens` 三键全 true）；**FSR 3.1.5 maintain 在案 = `milestones/g22/G22_P2_DECISIONS.md` §3 G22-N4 行字面**（probe JSON 无 FSR 字段，锚归属分列）。
2. **新鲜复测三态闭集（RFC-0045 F8 同律）**：vulkaninfo 新鲜复测 `VK_AMDX_shader_enqueue`——absent（与在案一致 → not-available 维持）/ **present（翻转 → 复评启动登记，门同样绿：Work Graphs 立项评估归承接锚窗）**/ SKIP（工具缺，如实登记 + 在案态兜底）；DGC 三扩展（`VK_NV_device_generated_commands` / `VK_NV_device_generated_commands_compute` / `VK_EXT_device_generated_commands`——以 g22 probe 结果字面为准）available 复测互核，漂移事件如实登记。
3. **FSR maintain 盘点**：`rurix-rt` vendor_upscale 面 0-byte 机核（git diff vs g28-closed）——FSR 3.1.5 生产档维持字面。
4. **门态映射（RFC-0045 F10 同律）**：分支捕获非透传——not-available 维持 / 翻转复评启动均**门绿**（合法诚实终态）；门 FAIL 只保留给「复测程序未诚实执行」。

## 5. out-of-scope（各附承接锚）

1. **SVT 四行实现**：承接锚 = 各行 reeval_anchor 字面（真实纹理资产管线出现/场景规模超显存窗）。
2. **KTX2-BasisU 真转码器接入**：承接锚 = 真实纹理资产管线出现经 `PagedResource::transcode` 留口接入（RD-041 backfill 字面）；DDS 链维持。
3. **Work Graphs 立项**：承接锚 = `VK_AMDX_shader_enqueue`（或 Vulkan 跨厂商对应物）present 翻转 + 「pass 内部提交单元可替换」接缝消费方出现。
4. **多层 slab 生产集成 / MaterialClosure 拓扑位消费**：closure 单层生产面 0-byte；拓扑位消费须显式修订行独立立项（32B 布局机核 RED 守卫维持）。
5. **host 参考臂重写**：`material/slab.rs` 冻结面；device 对拍暴露 host 语义缺陷时按只追加程序另立重判。
6. **kernel 性能优化 / 多 GPU**：承接锚同 RFC-0043/0044/0045 §5 各期字面。
7. **slab 吸收档扩展**：host 参考臂显式「无损 coating 档」（吸收档 = 后续波扩展位）；承接锚 = 有损材质资产需求出现。
8. **方向依赖/各向异性 slab**（F9a 补项）：slab.rs 模型为方向-半球反照率标量层级（无角度分辨）——Fresnel 角度依赖/粗糙度方向分布扩展承接锚 = 真实分层材质资产需求 + host 参考臂先行。
9. **层数 >2 语义扩展**（F9b 拆分，与 §5.4 生产集成分列）：N 层栈传输矩阵/递归闭式 = 新 host 参考面；承接锚 = 多层（>2）材质资产需求出现时先立 host 参考臂。
10. **时域材质参数动画/参数流送**（F9c 补项）：slab.rs `lerp` 为参数域连续性载体的显式下游；承接锚 = 动态材质资产面出现。
11. **bin-local 侧表资产化转正**（F9d 补项）：§2 侧表来日入资产管线（接 MaterialSideTable 通道）为独立立项面；承接锚 = slab 参数资产化需求 + Full RFC（与 §2.1 防混淆声明互补：现在零关系，转正须显式立项）。

## 6. 验收门映射

| P0 | 门 key | 波次 | 判据面 |
|---|---|---|---|
| M-a | `g29.p0.m_a.slab_device_kernel` | G29.2 | §1（逐样本对拍 + 白炉行登记 + 能量上界 + 双跑位级 + RED 臂 + spirv-val + 三态） |
| M-b | `g29.p0.m_b.slab_side_table_arm` | G29.2 | §2（侧表逐槽对拍 + 逐槽白炉互核 + 双跑位级 + graph/types.rs 0-byte） |
| M-c | `g29.p0.m_c.svt_ktx2_gap_rejudgment` | G29.3 | §3（七行逐行 reeval + manifest 映射表 + RD-041 只追加 + g22 表 0-byte） |
| M-d | `g29.p0.m_d.wg_dgc_capability_recheck` | G29.3 | §4（WG 三态复测 + DGC 互核 + FSR 盘点 + 门态映射） |
| M-e | `g29.p0.m_e.closed_gate_no_regression` | G29.4 | G28 受影响门 `--verify-latest` 全绿零降级；禁 `--gate` 旧脚本；`g29_` 前缀不抢 latest |

与 G29_CONTRACT §4.2 同构；M-a/M-b 的 evidence schema 侧另含 `skipped_dev_env` 合法态（**F10b 拆句**：该字段名为 RFC/schema 侧钉死，契约 §4.2 对应字面 = 「SKIP 如实登记不冒充」）；implemented / maintain-defer / not-available 维持均为合法终态，**零冒充**。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v0.1 | 2026-08-25 | G29.1 起草（Draft）：§1 slab 闭式 device 化（公式 L46-56 逐字 + 角点算术门 + f32 容差协议〔F3 同律〕+ 白炉行如实登记）；§2 侧表供参（MaterialClosure 32B 冻结边界钉死 + bin-local 独立 SSBO）；§3 七行重判（manifest 忠实性）；§4 WG/DGC 三态复测（门态映射）；待 D-409 对抗性评审。 |
| v0.2 | 2026-08-25 | D-409 对抗评审修法批：**F2 blocker 修法 A**（删角点算术门 → `max(denom, 1e-30)` 分母安全化，角点 R=rc=1.0 位级还原 host 分支值，NaN 构造不可达论证入字面）/ F3 输出有限性一等断言 ⓪（先于聚合硬 FAIL，§1.4/§2.3 双列）/ F1 白炉行 dev 来源钉死（OpFDiv ≤2.5 ULP 非正确舍入）+ host 位级 0 + 判据①传递覆盖论证 / F4 网格血缘钉死（probe GRID=128 (grid+1)² 口径 + 注释误述防线；契约「同 host 单测口径」失准登记于本行提请勘误——16641 实为 probe 口径）/ F5 侧表 0.95 规避意图声明 + host 单源纪律继承 / F6 F9 同律三件全承接（常量表承载 + 锚字面派生 + 字段钉死）/ F7 FSR 锚拆分（G22_P2 G22-N4 行）/ F8 侧表防混淆声明 + material/ 全目录 0-byte 机核扩面（守卫本体同受保护）/ F9 out-of-scope 补 §5.8~5.11 四项 / F10 锚精度四件（守卫实体指认/skipped_dev_env 拆句/params 区间/四字段点名）/ F11 RED 臂判据归属分列；状态 Draft → **Agent Approved**（评审 `milestones/g29/design/rfc0046_adversarial_review.md`，11 findings 全 disposition，F2+F3 必修先于开工已落）。 |
