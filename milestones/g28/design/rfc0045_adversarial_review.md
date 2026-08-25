<!-- Assisted-by: Cursor Agent（D-409 对抗性评审程序） -->
# RFC-0045 对抗性评审报告（D-409）

| 字段 | 值 |
|---|---|
| 评审对象 | `rfcs/0045-lighting-device-realization.md`（G28 光照 device 化，Draft v0.1） |
| 评审日期 | 2026-08-25 |
| 评审程序 | D-409 对抗性评审（独立评审会话，零共享上下文——本会话未接触 RFC 起草会话的任何中间产物，仅以树内文件为输入） |
| provenance 声明 | 评审员与起草方为**同环境单一模型家族**，可能存在同源盲区偏差，如实登记，本报告效力自限于该偏差面之外 |
| 已逐字核对锚 | RFC-0045 全文；`src/rurix-render/src/gi/restir_reservoir.rs` 全量（`Pcg32`/`update`/`merge`/`unbiased_weight`/`estimate_ris`/`fixture_lights`/`variance_experiment` 及五单测）；`src/rurix-render/src/bin/g21_restir_probe.rs` 全量；`milestones/g21/G21_P2_DECISIONS.md` §1 M100-high/M52 行；`milestones/g21/g21_rd040_subitem_registry.json` 五分项全量；`milestones/g21/g21_ser_capability_probe_results.json` 全量；`registry/deferred.json` RD-034 全条目 + RD-040 条目（title/reason/backfill_condition/status + history 首行、2026-08-06 M50 行、2026-08-18 NRD 行、尾三行含 G21.3 止点）；`ci/meshrt_probe_smoke.py` 全量；`milestones/g25/g25_campaign_handover_registry.json` M100-high/M52/rd_eight 行；`milestones/g28/g28_budget.json`；`registry/number_ledger.json` RFC namespace；RFC-0038/0043/0044 全文；`src/rurix-render/kernels/g18_light_transport_depth.rx` 头注；`G28_CONTRACT.md` §4.2 + `G28_ACCEPTANCE_MAP.md` §1 五行；`src/rurixc/src/vulkan_codegen.rs` f64 拒绝面（L576/L2111）；`gi/multi_light.rs` `check_restir_trigger`/`restir_serve` 存在性 |
| 未在本会话逐字核对锚（效力自限清单） | RD-040 history 中段（G13.5a~G16plus 行只抽核）；`evidence/g21_restir_probe_*.json` 与 G21 M-a~M-e 绿件 evidence 本体；`spec/global_illumination.md`/`spec/shader_stages.md` RXS-0357/RXS-0203 条款本体（仅核代码注释引用面）；RFC-0013 §4.E9/RFC-0019/0023 冻结面；`milestones/g24` 表（G10-N6 在案现状）；`G28_PLAN.md`/`G28_CANDIDATE_DECISIONS.md`/`CI_GATES.md`；`ci/g28_*_check.py` 三治理脚本——凡 finding 依赖上述锚字面者均标注「待核对」，其 disposition 须以逐字引锚方式兑现 |

**已核实为准确的字面转述**（正向登记，缩小争议面）：

1. §1.1/§1.2/§1.4 对冻结模块的全部转述与 `restir_reservoir.rs` 逐字吻合：`update` 的 `w_sum += w; m += 1; if w_sum > 0 && f64::from(next_f32()) < w/w_sum`（L114–121，**w_sum ≤ 0 时 && 短路不消费**属实）；`merge` 空 other（`y == usize::MAX`）早退零消费（L124–127）、`m_other = other.m.min(m_cap)`（L128）、头注「other 的等效权 = phat_y · W_other · m_other」（L129）与实现 `other.w_sum·m_other/other.m`（L130–134）的代数恒等关系（W_other = w_sum/(p̂_y·m) 代入即得）、除法比较形（L137）、`m = m_before + m_other`（L141）；`unbiased_weight` 三退化分支（L146）；`estimate = f64::from(phat_y) * unbiased_weight()`（L169，禁化简论断的舍入差异成立）；`w = f64::from(phat)·n as f64` 提升点（L166）；w_sum f64 顺序累加；`Pcg32` 头注「流为输入非结果」（L23）、`next_f32` 24 位尾数（L53–55，`>>8 × 2⁻²⁴` f32 精确可表论断成立）；`target_phat` 乘除序（L73–83）。
2. §1.4 五常量与 `g21_restir_probe.rs` 逐字吻合（SEED=0x0521_A011_2026_0824 / 20000 / 16 / 8 帧 / M-cap 60，L14–18）；`fixture_lights` 公式（L198–208）、单着色点 pos[0,0,0]/normal[0,1,0]（L219–222）、三流 `t·4+k` 分配 k=0 uniform / k=1 RIS / k=2 时域（L233–235）、3σ 检验同式（L38–41 与 probe/单测双源一致）逐字命中。
3. §2.2「异点直用即偏置」论断**独立验证成立**：host merge 捷径 `other.w_sum·m_other/other.m ≡ other.phat_y·W_other·m_other`，仅在 p̂_受点 ≡ other.phat_y（同目标函数）时与受点重评律 `p̂_dst(y)·W_other·m_other` 重合；异点直用时保留概率与 W 归一化以邻点目标为权，RIS 无偏贡献恒等式 E[g(y)·W_y] = Σ_x g(x) 仅对受点目标 g 成立——结构性偏置论断正确。§2.1 夹具几何独立复核：网格半径 √(1.75²+1.75²) ≈ 2.475 < 2.5（「内嵌灯环」成立）、灯高 1.5+0.8·sin(3a) ∈ [0.7, 2.3] ⇒ 逐点 ndotl > 0 全支撑成立——全支撑使受点重评版合并与带 Z 计数修正的 unbiased 合并（Bitterli 2020 Alg. 6，Z = m）恒等，无偏性在本夹具构造性成立（另见 F11③）。
4. §1.2 录制推断的位级正确性**独立验证成立**：`update` 内 w_sum 在消费判定前恰累加一次、判定后不再改写 ⇒ 「update 返回后 `r.w_sum > 0`」⟺「本次 update 消费了 next_f32」无例外；Copy 快照在 update 前 ⇒ 快照重放 `next_f32()` 与 update 内部消费值位级同值。「host bug 同时进带与参考 ⇒ 对拍不检出」的循环论证风险已由判据 ②（独立对 `exact_direct` 解析参考）阻断，§1.5 ② 的纵深防御声明成立。
5. §2.4 ① 族错误率数字复算正确：3σ 双侧 ≈ 0.0027 × 64 ≈ 0.17；5σ 双侧 ≈ 5.7e-7 × 64 ≈ 3.7e-5。
6. 治理锚全命中：G21_P2 §1 M100-high/M52 行承接锚逐字（含「device 化/空间重用/M100 车道集成窗」三件读法与「兜底 = 语言层不加 SER 原语维持（字面 0-byte）」）；五分项 reeval_anchor 五条逐字；SER probe 三 token + verdict=available + RTX 4070 Ti；RD-034 backfill 二选一 + 步骤 69 恒跑字面 + 「5319 = LaunchIdKHR」；RD-034 history 止于 G21.3（2026-08-24）、RD-040 history 止于 G21.3 M-c 行（2026-08-24）——§3.4/§4.3 断档口径与事实一致；g25 handover rd_eight 行存在；`g28_budget.json` 头注「若实测位级可达则零容差零条目」逐字；number_ledger RFC on_tree_max=45/next_free=46 与头表「实测 next_free=45 顺位领取」自洽；RFC-0038 §1.7（§1 列表第 7 条「device kernel 车道 out-of-scope」）、RFC-0043 §1.4 标定协议/F4 量化兜底/F6 manifest/§5.4、RFC-0044 §1.1 域前提/§1.2③ 冗余设计声明/§2.1③/§2.3 合取防冒充/§3.2 过述防线/§3.5 append-only 机核/§4.4 不混同——引用先例全部命中；§3.2 不混同声明的 G8.2 M50 行（2026-08-06）与「G21.2 M-b 终判在 M50 在树前提下作出」时间线属实；RFC §6 五门 key 与 G28_CONTRACT §4.2 = G28_ACCEPTANCE_MAP §1 逐字同构；`meshrt_probe_smoke.py` 退出码判定（非 grep）、三态 SKIP、`RURIX_REQUIRE_REAL=1` 硬红全部与脚本字面一致。

---

## Findings

### F1（major）随机带 offset 表形态未钉死；判定带消费计数锚在本夹具构造性平凡化未声明

**指认字面**：§1.1「③随机带双 SSBO + 逐 trial offset 表（§1.2）」；§1.2「offset 表 ≤ 320000 < 2²⁴……两带、offset 表与计数输出全 f32 精确承载」；§1.5 ①「逐 trial 判定带消费计数 device vs host 模拟全等」。

**挑战**（两层）：

1. **offset 表是 device 消费定位的承重面，其语义未钉死**：候选带定长（16/trial，偏移 = t·16 可算术定位，t·16 ≤ 319984 < 2²⁴ f32 精确）；判定带变长才需要表。RFC 未写明：表覆盖哪条带（双带各一表？仅判定带？）、表元素语义（起始偏移前缀和 vs 逐 trial 长度）、是否含尾哨兵、表本身是否入 evidence/digest。「逐 trial 单 invocation」下 offset 错位一格即整段 desync——这一自由度留给实现波即埋下「实现即立法」缺口。
2. **消费计数锚在 §1.4 夹具下恒等于 16，检出力平凡化**：单着色点 [0,0,0]/normal [0,1,0] 对 `fixture_lights(64)` 全灯 ndotl > 0（灯高 ≥ 0.7）⇒ 每个候选 phat > 0 ⇒ w > 0 ⇒ 首次 update 起 w_sum > 0 恒成立 ⇒ 16 次 update **全部消费**、短路分支构造性不可达、判定带恒满长 16、逐 trial 消费计数恒 16。于是「消费计数全等」锚对任何 desync/offset 错位**零检出力**（计数恒 16 不因错位而变），真实检出承重完全落在保留样本 y 全等锚上。RFC 把计数锚列为前置整数锚之一而未声明这一退化事实，存在把平凡真断言叙述成有效防线的措辞风险；「变长判定带 + offset 表」机制在本夹具实为死代码路径（防御性一般形态设计——合法，但须自知并声明）。

**建议 disposition**：§1.2 钉死 offset 表字面——「判定带逐 trial 起始偏移表（前缀和，含末位总长哨兵，f32 精确域已证）；候选带零表（偏移 = t·16 算术定位）」，offset 表随行入 evidence；§1.5 ① 补声明「本夹具下消费计数恒 16（全支撑构造性成立），计数锚的活性检出承重在 y 全等锚；变长机制为一般形态防御设计非本夹具活性面」。

### F2（major）录制器循环骨架复写面与 host 对拍参考值产出路径缺自检锚

**指认字面**：§1.2「随机带由 bin-local 以 `Pcg32` Copy 快照 + 冻结模块 `update` 本体驱动录制……候选抽取与 w 提升两行字面同源复写——禁在 bin 内复刻 update/merge 判定逻辑的第二份消费点实现」；§1.5 ①「device vs host 逐 trial estimate 对拍」。

**挑战**：消费点判定逻辑零复刻已达成（正向登记 4），但录制器必然在 bin 内重建 `estimate_ris` 的**循环骨架**（候选抽取 + `target_phat` + w 提升 + update 调用序）——RFC 已诚实标注「两行字面同源复写」。残余缺口：**host 对拍参考值（estimate/y/w_sum/m）从哪条路径产出未钉死**。若取自录制循环的 Reservoir 终态，则「host 参考」本身是复写骨架的产物，骨架漂移（如循环次序/流误用）时带与参考同时漂移、对拍面自洽地错——①判据检不出，只能靠 ②（3σ 对解析参考）以统计方式兜底，检出力远弱于位级锚。若直调 `estimate_ris`（冻结 API），复写骨架与直调之间又无一致性验证锚。

**建议 disposition**：§1.2 增**录制自检锚**（机核硬断言）：逐 trial 以同 (SEED, t·4+1) 重建 rng 直调 `estimate_ris`，其返回 (est, r) 与录制循环终态在 {estimate f64 位级, y, w_sum 位级, m} 上全等，任一不等即录制器 FAIL；host 对拍参考值钉死 = `estimate_ris` 直调产物（录制器只产带与 offset 表，不产参考）。

### F3（major）精度态 A 在现行语言面构造性不可达；「升态 A」修复途径为幻影路径；与判档声明存在张力

**指认字面**：§1.3「态 A（rurixc kernel f64 可用）——w/w_sum/比较左值 `f64::from(u)`/W/estimate 全 f64……两态均合法，以实现波 rurixc 实测定盘」；§1.5 ①「修复途径 = 升态 A 或判定算术 f64 等效承载」；头表判档「渲染器库面零新语言语义条款」。

**挑战**：树内事实与「态 A 可用」的可能性直接矛盾——`src/rurixc/src/vulkan_codegen.rs` L576「mesh 路径仍拒绝 64 位整数，**f64 在两条路径均为 RX6026**」、L2111「调用方的标量类型映射**先拒 F64**（RXS-0203 L1 …）」（条款本体待核对，代码注释引用面已核）；`src/rurix-render/kernels/*.rx` 全部 kernel 零 f64；`spec/shader_stages.md` 零 f64 命中。即 rurixc kernel 子语言 device 面**构造性拒绝 f64（诊断码 RX6026）**，「实测定盘」必然落态 B——态 A 不是环境探测结果的分支，而是**语言语义扩展**（f64 device 类型/算术/SSBO 布局 + SPIR-V Float64 capability）之后才存在的世界线，这与本 RFC「零新语言语义条款」判档字面冲突。连锁后果：§1.5 ① 把「升态 A」列为态 B 整数锚失败（判定边界翻转）的两条修复主路之一——该路在本 RFC 权限内不可走，翻转真实发生时（f32 除法比较相对 f64 的翻转窗口 ~1e-7/判定 × 320000 判定 ≈ 期望 0.003~0.03 次，小概率非零）程序卡死在幻影分支上。RFC 起草未核实语言面现状即写入两态对称叙述，属事实源核对缺口。

**建议 disposition**：§1.3 增语言面现状引证（RX6026 / RXS-0203 L1 拒 f64 字面），态 A 改写为「语言面 f64 能力出现后的承接态（激活须语言面程序另立，超本 RFC 判档）——本期实现波以 RX6026 拒绝实测为态 B 定盘证据」；§1.5 ① 修复途径改写为「f64 等效承载（double-single 软件算术，bin/kernel 加性面）或语言面 f64 扩展另立 Full RFC——二者均另立只追加修订」；evidence schema 增态选择依据字段。

### F4（minor）§1.2「kernel 维持全 f32 SSBO 纪律」与 §1.3 态 A「estimate 输出 f64 SSBO」字面冲突

**指认字面**：§1.2「两带、offset 表与计数输出全 f32 精确承载，kernel 维持**全 f32 SSBO 纪律**，u64 零下发」；§1.3 态 A「……estimate 输出 **f64 SSBO**」。

**挑战**：前者以全称形式陈述（「kernel 维持全 f32 SSBO 纪律」），后者在态 A 下要求 f64 输出缓冲——两条款字面打架。若严格全 f32 则态 A 输出面被禁（位级对拍 p100=0 不可达，host estimate 为 f64）；若态 A 豁免则 §1.2 全称字面须限定辖域。F3 接受后态 A 移出主文，本条随动消解；若 F3 不接受则本条独立成立。

**建议 disposition**：§1.2 全 f32 纪律限定辖域为「输入带/offset 表/整数锚输出面」，态 A 的 estimate 输出面显式豁免（或随 F3 一并改写）。

### F5（major）§2.1 API 闭集与 §2.2 受点重评律的实现路径矛盾——直调冻结 merge 无法承载受点重评，重实现即消费点第二实现

**指认字面**：§2.1「全部 bin-local 只消费冻结模块公开 API（`fixture_lights`/`ShadePoint`/`estimate_ris`/`merge`/`exact_direct`）」；§2.2「w_other = target_phat(sp_受点, lights[other.y])·(other.w_sum/(other.phat_y·other.m))·m_other……保留样本换为邻样本时 phat_y 槽位同步写受点重评值」。

**挑战**：host `merge` 字面硬编码同目标捷径（`other.w_sum·m_other/other.m`，保留时 `phat_y ← other.phat_y`）——**按 other 原样直调冻结 merge 得到的恰是 §2.2 自己论证为偏置的「异点直用」形态**。要实现受点重评律，bin 要么改传入、要么重实现 merge 判定链——后者复刻了 RNG 消费点（与 §1.2「禁第二份消费点实现」纪律精神冲突），前者 RFC 未写。且 §2.1 API 闭集**遗漏 `target_phat`**（§2.2 受点重评的必需函数）与 `unbiased_weight`。矛盾未解则 M-b 实现波面临「违 §2.1 重实现」或「违 §2.2 直用偏置」的二选一。

**建议 disposition**：钉死**重评快照变换 + 冻结 merge 直调**形态（零消费点复刻，本评审已代数验证闭环）：构造 `other' = Reservoir { y: other.y, phat_y: p̂_dst, w_sum: p̂_dst_f64 · other.unbiased_weight() · f64::from(other.m), m: other.m }`（p̂_dst = `target_phat(sp_受点, &lights[other.y])`），再 `self.merge(&other', rng, m_cap)`——merge 内部 `w_other = other'.w_sum·m_other/other'.m = p̂_dst·W_other·m_other` 与 §2.2 律逐字等价；退化分支自动闭合（other.phat_y ≤ 0 或 m = 0 ⇒ `unbiased_weight()` = 0 ⇒ w_sum' = 0 ⇒ w_other = 0）；保留时 `phat_y ← other'.phat_y = p̂_dst` 即「槽位同步写受点重评值」；m_cap 截断语义原样保持。§2.1 API 闭集补 `target_phat` + `unbiased_weight` 两项。

### F6（minor）§2 夹具参数闭集钉死度不足（「规范基线 N=8」留口）

**指认字面**：§2.1「N×N 着色点网格（**规范基线 N=8**：……）」。

**挑战**：「规范基线」措辞暗示存在非基线变体，N 未闭集化；trials=20000 靠 §2.3/§2.4 的「20000-trial」间接出现，M=16/M-cap=60/SEED 靠对 §1.4 的引用继承——§2 无一处集中的参数闭集表。对拍/复现面的参数应一处钉死。

**建议 disposition**：§2.1 集中钉死参数闭集（N=8 唯一、trials=20000、M=16、M-cap=60、SEED 同 §1.4、流 k=3），删「规范基线」措辞或改「唯一夹具」。

### F7（minor）3σ 统计功效未做量化声明；k=3 新流族独立性未被 G21 绿件覆盖

**指认字面**：§2.4 ①（聚合硬门 + 5σ 兜底）；§2.1 流纪律（k=3 残差类）。

**挑战与独立判断**：评审员独立估算——3σ_mean = 3·σ_trial/√20000 ≈ 0.021·σ_trial（聚合门在网格均值序列上），5σ 逐点兜底检出限 ≈ 0.035·σ_point；受点重评类结构性偏置（邻点归一化常数混入）量级由 p̂ 场跨点相对差决定，本夹具网格角点—中心的几何差异使其为 percent 级，相对检出限裕度约一个数量级——**64 点 × 20000 trial 的功效对本判据面的目标缺陷类充分**（独立推断，非 measured）。但 RFC 未写任何功效论证；且空间臂 k=3 流族为新增（G21 绿件只覆盖 k∈{0,1,2}），PCG32 不同 inc 流的独立性对 k=3 × 64 点族未经实测检验，仅由判据 ① 统计兜底。

**建议 disposition**：§2.4 ① 补一句功效注（检出限 ~0.02σ_trial 相对结构偏置 percent 级的裕度声明，σ_mean 实测值入 evidence）；§2.1 补「k=3 流族独立性以判据 ① 3σ 检验为机器验证面」声明。

### F8（major）M52 capability 新鲜复测 not-available（环境漂移）分支缺失

**指认字面**：§3.2 ①「capability 半边 = G21 三 token available 只读盘点……+ 新鲜 vulkaninfo 复测二次取证（同三 token 枚举重跑入档；工具不可定位 → SKIP 态如实登记『新鲜复测未跑』，只读盘点半边独立成立但新鲜件缺席显式入 evidence，不冒充复测绿）」。

**挑战**：新鲜复测的结果空间被写成 {复测绿, SKIP} 两态——**「复测跑了且 token 缺失/verdict not-available」（驱动更新移除扩展、换 GPU 等环境漂移）无判定分支、无登记口径**。该场景下 capability 半边的合取输入取「在案 available」还是「现势 not-available」未定优先级：对本期预期结论（workload 半边未命中 ⇒ maintain-defer）无实质影响，但若 workload 半边意外命中，「两半全齐方改判」的判定在 available-在案 + not-available-现势下悬空——这恰是决策树必须预先钉死的分支。另 fresh verdict 的生成规则（三 token 全 true 才 available？部分命中如何判）亦未写。

**建议 disposition**：§3.2 ① capability 现势口径三态钉死 {fresh-available, fresh-not-available, fresh-skip}，verdict 规则 = 三 token 全 true（G21 probe 同构）；合取判定的 capability 输入 = **现势优先**——fresh-available 方计命中；fresh-not-available ⇒ 半边不齐（maintain-defer）+ 环境漂移显式登记（G21 在案件 0-byte 不改写，两代取证并列入 evidence）；fresh-skip ⇒ 沿用在案件 + 新鲜件缺席显式登记（既有字面）。

### F9（major）RD-040 五分项 manifest 纪律只防「空」不防「歪」——检索 pattern 忠实性无约束

**指认字面**：§3.3「逐项 reeval_anchor 字面树内实测（逐项检索路径清单入 evidence，manifest 纪律同 §3.2）」。

**挑战**：逐项清单 + 非空强制（经「同 §3.2」继承「清单为空或缺失即门 FAIL」）已达先例（RFC-0043 F6 / RFC-0044 §2.1 ③）强度，但五分项批量场景下的敷衍经济性更高，且 WORLD-RC（「需求场景/联动窗痕迹」）与 NRD（「树内 measured artifact」）两项的检索对象无天然文件锚、语义模糊度显著高于 M52 workload 半边（有 RXS-0357 谱系锚）——**用一条与锚字面无关的狭窄 pattern 制造非空清单即可「合规」得出零命中**，manifest 纪律对此零防御。另五分项检索形态实为两类：盘点型（SMRT/OMM 读压测清单闭集）与检索型（WORLD-RC/NRD/RT-PIPELINE-SBT 模式搜索），manifest 应载字段不同，RFC 未区分。

**建议 disposition**：§3.3 补两条：① manifest 逐项字段钉死——检索型载 {pattern 表, 检索根, 逐 pattern 命中数}，盘点型载 {清单文件路径, 读得闭集内容}；② pattern 忠实性纪律——逐分项 pattern 表由盘点脚本字面承载（常量表）且须含该分项 reeval_anchor 字面的派生关键词（如 NRD：降噪/画质差距/measured 对拍面；RT-PIPELINE-SBT：hit/miss 着色/SBT/多材质分派），pattern 表本身随 evidence 落档供复核。

### F10（major）RD-034 探针意外成功（exit 1）时 M-d 门态映射未钉死；backfill ② 分支零检测未声明

**指认字面**：§4.2「exit 0 = **意外成功翻红**（探针 FAIL 退 1）= 上游消费能力出现 → 复评启动登记」；§4.3「**维持 blocked / 解锁复评启动二态均如实**……解锁/维持均为合法终态，零冒充」。

**挑战**（两层）：

1. **退出码语义冲突未解**：`meshrt_probe_smoke.py` 在意外成功时返回 1（已核 L155–168）——若 M-d smoke 层把探针退出码直通门态，则「解锁复评启动」这一 §4.3 明文合法终态**恒以门红呈现**，与「二态均合法」直接矛盾；契约 M-d 行同构地并存此张力。且脚本退 1 有三种不可区分于退出码的原因（red 自检失败 / 步骤 68 mesh B 链转红 / 步骤 69 意外成功），门层判定须按 stdout 标记分流，RFC 零字面。
2. **backfill ② 分支零检测未声明**：backfill_condition 为二选一（① spirv-cross 消费路径；② RD-015 上游 LLVM #90504/#57928 merge），探针只覆盖 ①；② 属上游 PR 状态、树内不可实测（外采面）——本期不检测是合理的，但 RFC 未显式声明该定界，「上游复查程序」的覆盖面存在无声收窄。

**建议 disposition**：§4.2 钉死门态映射——M-d 门判据 = 程序完整性（探针真跑 + 对应二态登记落档 + RD-034 history G28.3 行追加 + append-only 机核绿），探针退出码仅为证据输入：exit≠0 → 维持 blocked 登记路径；exit=0（以 stdout「步骤 69 …意外成功」标记判别）→ 复评启动登记路径——两路登记齐备均判 M-d 门绿；探针因 red 自检/步骤 68 转红而退 1 时如实门红（非二态场景）。§4.1 补「② 分支属上游仓库状态面，树内闭集纪律下本期不检测，复查覆盖面 = ① 分支探针 + ② 分支维持在案字面」。

### F11（minor）out-of-scope 漏列四项

**指认字面**：§5 七项闭集。

**挑战**：以下四面在正文有据但未入 §5 承接锚闭集：① **空间 merge 的 device 化**——§5.2 只写「时域 merge device 化随本窗联动」，M-b 空间臂为 host 形态，其 device 化归属窗零字面；② **ReSTIR GI（间接光/路径域）扩展**——M100 锚题字与 RD-040 title 均含「ReSTIR GI-DI」，本 RFC 全程 DI 域（`target_phat` 无阴影直接贡献），GI 域定界未显式声明（§2.5 过述防线只防「车道锚整体兑现/低档更替」两面，未防 GI/DI 混同读法）；③ **部分支撑/遮挡场景的无偏修正**（Z 计数/MIS 权，Bitterli Alg. 6）——本夹具全支撑使其不需要（正向登记 3），生产场景（遮挡/朝向差异致 supp(p̂_dst) 不覆盖）必现，未列承接锚；④ **时域+空间联合重用**（时空联合臂，host 形态算法面）——§1.4 只处置了时域 device 化归属。

**建议 disposition**：§5.2 补「空间 merge device 化随本窗联动」；§5 增列 GI 域扩展（承接锚 = RD-040 ReSTIR 分项 GI 需求证据窗）、非全支撑无偏修正（承接锚 = 遮挡/一般化场景入夹具窗，与 §5.2 联动）、时空联合臂（承接锚 = 生产集成窗）三项，或于 §2.5 并入定界声明。

### F12（minor）引用面三处小刺

**指认字面与挑战**：① `kernels/g28_restir.rx`/`kernels/g18_light_transport_depth.rx`——仓库根无 `kernels/` 目录，实际路径 `src/rurix-render/kernels/`（RFC-0043/0044 同简写且实现已落该路径，惯例已立但字面不精确）；② §1.8「（RFC-0038 §2 不变量字面）」——`check_restir_trigger`/`restir_serve` fail-closed 登记面字面实际在 RFC-0038 §1.3，§2 仅有「M100 低档面/multi_light.rs 0-byte」概括（引证节号偏移，语义无损）；③ §3.3「『M-c 唯一消费面』字面沿用」——原文为 G21 M-c 语境，G28 M-c 再消费后「唯一」已不成立，宜注明双期消费事实（五分项表本体 0-byte 前提下的读法澄清）。

**建议 disposition**：修法批顺手统一——kernel 路径写全或加简写口径声明；§1.8 引证改「RFC-0038 §1.3/§2」；§3.3 补「（G21 语境字面；G28 M-c 为第二消费方，原表 0-byte）」。

---

## 总评

**结论：建议「有条件 Agent Approved」——7 项 major（F1/F2/F3/F5/F8/F9/F10）全部以修法批（v0.2）落档后方可翻状态；0 blocker。**

分布：**12 findings = 0 blocker / 7 major / 5 minor**。

核心方案经逐锚核对与独立数学验证**结构成立**：随机流单源纪律（PCG32 状态整体留 host + 已对齐消费序双带）与语言面现状（device 拒 64 位整数）互恰；「冻结 update 本体驱动录制」的消费判定推断（update 后 w_sum > 0）位级无懈可击，循环论证风险由判据 ② 独立解析参考阻断（§1.5 ② 声明成立）；受点重评律的偏置论断与全支撑夹具的无偏构造均独立验证正确；M52 两半合取防冒充硬线、五分项闭集、RD-034 退出码判定均与事实源逐字吻合；治理锚（G21 三件/g25 handover/budget 头注/先例 RFC 四份/契约-验收图同构）全命中，正向登记面大。

major 的共性是**程序分支完备性与实现自由度收口**，不是方案性缺陷：F3 是唯一的事实源核对缺口（态 A 撞上 RX6026 语言面拒绝，两态叙述失真），但态 B 路径完整自洽，M-a 可落地；F1/F2/F5 把「实现即立法」的三个自由度（offset 表、参考值产出路径、受点重评实现形态——F5 已附代数闭环的零复刻方案）钉死即可；F8/F10 是决策树漏分支（环境漂移、意外成功门态），本期预期路径不受影响但对抗场景下判定悬空；F9 是 manifest 纪律的忠实性升级。全部 disposition 均为只追加修法批可承载的量级，无需推翻任何章节。

**条件**：v0.2 修法批逐项 disposition 七 major（minor 可合批处理或显式 no-fix 留档），其中 F3 须引语言面字面（RX6026/RXS-0203 L1，条款本体逐字核对后引）、F5 须落变换公式字面、F8/F10 须落决策树分支字面。落档后本报告效力支持翻 **Agent Approved**。
