<!-- Assisted-by: Kimi-K3（G15.1 治理波） -->
# G15_PLAN — G15 期计划（定稿版；G15.1 治理波产出；v1.0 2026-08-23）

> **状态**：G15.1 治理波定稿 v1.0（2026-08-23）。**唯一事实源 = [G15_CONTRACT.md](G15_CONTRACT.md) v1.0**（front matter 双状态机：`status: active`〔G15.1 governance-only〕+ `implementation_status: blocked`〔G-G15-2 事实互锁未过前 G15.2+ 禁止开工〕）。本计划不新增事实，全部判据字面以契约为准。
> **事实源**：[G15_CONTRACT.md](G15_CONTRACT.md) v1.0（契约 §1~§8 逐字）· [G14PLUS_RECORD.md](../g14/G14PLUS_RECORD.md) §6.3（G15 承接锚法定输入逐字）· [G14_P2_DECISIONS.md](../g14/G14_P2_DECISIONS.md) v1.0 §5（defer-to-G15+ 29 行承接锚）· [`g13_ue_upscale_gap_registry.json`](../g13/g13_ue_upscale_gap_registry.json)（8 行终态只消费不回写）+ [`g13_ue_lumen_gap_registry.json`](../g13/g13_ue_lumen_gap_registry.json)（2 行终态只消费不回写）+ [`g12_ue_pt_gap_registry.json`](../g12/g12_ue_pt_gap_registry.json)（10 行终态只消费不回写）· [`registry/deferred.json`](../../registry/deferred.json)（RD-034/039/040/041/042/043/044/045 八条 open）· [`g14_budget.json`](../g14/g14_budget.json)（G14 measured 帧时/画质锚带基线）。
> **0-byte 边界**：G5~G14 closed 契约与判据 0-byte；G13/G12 三表 20 行终态只消费不回写；RD 条目级 id/title/reason/backfill_condition 四字段 0-byte、history 只追加；src/spec/conformance 0-byte；零 RFC 消费（RFC 命名空间 next_free=31 维持）。本表为 `milestones/g15/` 新文件，UTF-8 + LF + 尾换行。

## 1. 目标与法定输入

### 1.1 目标（契约 §1 字面转引）

> **目标（用户 2026-08-23 指令字面兑现面）**：「帮我一次性完成G15里程碑，积极使用并行智能体和workflow减少工期」+ 2026-08-19 全期授权面「最终交付产物需要真实可商用，否则不要停止优化，并在此时允许在G15后无限制新建里程碑继续优化」——G15 = 画质量级收口与商用终审期：① G13 超分差距登记表 8 行 + Lumen 差距登记表 2 行 + G12 PT 差距登记表 10 行逐项重评（承接锚字面兑现，fresh measured_delta 可溯源）；② 绝对画质通过线设立（G14 out_of_scope 锚定 G15 面）；③ 严格画面审查（AI 读图强制门——G14.10f 教训字面兑现）；④ 商用收口判定（达标/未达标如实登记不冒充）；⑤ 性能零降级守护（G14 M-d 18/18 ×1.00 定盘面维持——画质收口不以性能回退为代价）。「UE5 级」可核对基线沿用 G8 口径 = UE 5.8（G9_CONTRACT §1 字面；本机 UE 5.8.1-56057345 == M128 登记机核继承）。

### 1.2 G14PLUS_RECORD §6.3 承接锚（法定输入逐字面）

- **绝对画质通过线不在 G14 设立**（G14 契约 `out_of_scope.absolute_image_quality_pass_line` / MAP §7）——归 G15 设立面。
- **G13 超分登记表 8 行与 Lumen 登记表 2 行只消费不回写，逐项重评锚定 G15**：Lumen `gap_id=2f6331a41404dfcd`（cornell：`gi_energy_rel` delta=0.535625027781919，`indirect_ssim` b=0.033384483786469556，`indirect_flip` delta=0.6127988976249465）；Lumen `gap_id=b7527c980cdd1d46`（bistro：`gi_energy_rel` delta=2.964585170338064，`indirect_ssim` b=0.006566911636724374，`indirect_flip` delta=0.9671355491209283）。
- **G15 法定输入 = 上述 8+2 行 + G14PLUS_RECORD §5 18/18 定盘 + RD-045 仍 open 的观察窗，不得另起无锚差距面**（G14PLUS_RECORD §6.3 末段字面）。
- 18/18 定盘证据：`evidence/g14_m_d_dual_end_fps_parity_20260822T183532Z.json`（首判定 `parity.met_count=18`，最紧格 bistro t100 dlss 1.2096）+ soak 复跑确认 `evidence/g14_m_d_dual_end_fps_parity_20260823T051754Z.json`（最紧格 1.0831 仍 ≥1.00）——G15 性能零降级守护法定对照基线。

### 1.3 三表 20 行逐项重评清单（只消费不回写）

| 表 | 行数 | 内容面 | 重评承载波 |
|---|---|---|---|
| `milestones/g13/g13_ue_upscale_gap_registry.json` | 8 | UE 超分差距（DLSS 档 deficit 2 行 + 噪声谱 6 行，逐行 measured_delta 可溯源） | G15.2 M-a |
| `milestones/g13/g13_ue_lumen_gap_registry.json` | 2 | UE Lumen GI 能量/间接光双端差（gap_id 见 §1.2 字面） | G15.2 M-a |
| `milestones/g12/g12_ue_pt_gap_registry.json` | 10 | UE PT 对标差距（G12.7b 终审锁定面承载） | G15.2 M-a |

处置面另立 `milestones/g15/g15_quality_gap_disposition.json`（新文件，gap_id 逐字转引 + fresh measured_delta 可溯源），三表终态本体 0-byte 不回写（契约 guardrails 第 6 条字面）。

## 2. 波次计划

并行策略总纲（契约 §7 立项裁决 5 字面）：用户指令「积极使用并行智能体和workflow减少工期」字面 = G15 全波允许并行子 agent 实施面——**波内并行、波间不越级**，波次生命周期沿 skill §4 十阶段；workflow 接线面按 actual next_free 延迟分配。

| 波次 | 内容 | 退出判据（契约判据 id） | 依赖 | 并行策略 |
|---|---|---|---|---|
| **G15.1 治理波**（本波） | 契约三件套 + 候选决策表（G14 defer 29 行逐行承接 + open RD 八条映射 + G15 新增候选 6 行）+ 验收映射 5 P0 + 治理三门 materialize（`ci/g15_acceptance_map_check.py` / `ci/g15_candidate_decisions_check.py` / `ci/g15_interlock_check.py`，步骤 266/267/268 实测领取）+ 互锁按事实诚实输出（BLOCKED/READY 均为正确结论字面，不充绿） | D-G15-1~4 + G-G15-1（G15.1 完成门）；本门通过不自动开放实现 | G14 closed（tag `g14-closed`，flip commit `f061487e`） | 波内并行：契约/候选决策/验收映射/治理门脚本四类治理资产可并行子 agent 起草，主会话统一接线提交批 |
| **G15.2 测量重收割波**（M-a） | 双端画质对拍链路全量复跑（G13 M-c ue_upscale_parity + G13 M-d ue_lumen_gi_parity + G12 M163 ue_pt_parity 三门同口径复跑，契约 digest 0-byte 门序维持）+ 20 行登记表逐项重评 fresh measured_delta + G15 差距处置表落盘零空行 + UE 方差带程序产 + AI 读图基线臂结构完整性断言 | G-G15-2（实现互锁门：READY + 用户开工指令留痕 + 编号重校准）+ G-G15-3（G15.2 退出门：M-a P0 独立断言全绿） | G15.1 完成门真实通过；实现互锁开放；UE 5.8 环境可用 | 波内并行：三门对拍复跑 / 20 行重评 / AI 读图基线臂分腿并行；波间不越级（M-b 须待 M-a 处置表落盘） |
| **G15.3 修复闭环波**（M-b） | measured 主差修复闭环逐项处置（20 行逐行 closed-resolved / closed-caliber-registered / open-defer-G16+ 三态零空行）+ 修复项 RED 先行（失败测试先落 main 为 RED）+ 触冻结面独立 Full RFC 留痕（D-409 对抗评审）+ 材质链表达面立项评估结论登记（G11-N8/G11-N9/G12-N10 承接锚命中判定逐字） | G-G15-4（G15.3 退出门：M-b P0 独立断言全绿） | M-a 处置表落盘（修复对象 = 处置表 measured 主差行） | 波内并行：独立修复项可按 gap 分腿并行修复（同一冻结面触碰归并单线）；评估登记与修复闭环并行 |
| **G15.4 绝对画质终审波**（M-c） | 绝对画质通过线程序产标定（UE 参照 deficit 双 seed 方差底 p100×2.0 程序产口径沿 G13 标定链，禁手写 P-09，标定链路入 evidence）+ 双场景 × 三档（t50/t67/t100）× 三后端（tsr_device/dlss_sr/fsr_3_1_5）18 格逐格 AI 读图严格画面审查（无乱序/无错位/无全黑/关键结构可见——cornell 盒体结构、bistro 吊灯/吧台/桌椅）+ 商用收口判定（达标格数/18 + 未达标格如实登记不冒充） | G-G15-5（G15.4 退出门：M-c P0 独立断言全绿） | M-b 修复闭环终态（终审对象 = 修复后画质面）；AI 读图基线臂（M-a 产） | 波内并行：18 格审查按场景/后端分腿并行出图与读图；通过线标定单线先行（判据先行于逐格判定） |
| **G15.5 性能零降级波**（M-d） | G14 M-d 门同口径复跑（双场景 × 三档 × 三后端 18 格，三轮进程级独立运行 50×3 trimmed mean 跨轮中位数 + 逐轮守护带）逐格 ratio ≥ ×1.00 维持 + G14 M-c 画质锚带复核（SSIM deficit ≤ 0.010779849285388998 带内）+ G14 门产 budget 条目零 estimated 维持 + 画质修复致性能劣化静默即 RED | G-G15-6（G15.5 退出门：M-d P0 独立断言全绿） | M-c 终审完（画质面终态锁定后性能复跑）；G14 18/18 定盘基线（g14_budget） | 波内并行：帧率复跑与画质锚带复核分腿并行；GPU 资源面串行调度（见 §4 GPU 锁） |
| **G15.6a 决策+稳定波** | G15 期全部 P2/留档/未触发分项逐条 go/no-go/defer-to-G16+ 穷举（零空行；defer 必有承接锚；no-go/defer 如实保持 open，不阻塞 soak 且不得写进全绿叙述）+ M-e 回归门（既有 84 门零降级 + 触改面真跑抽检 + RD-045/M165 同型 digest 漂移监控登记）+ stabilization soak（量级沿 G14.5a 继承〔≥1800s〕或 measured 证明更短足够；strict budget 非空、零 estimated/skip） | G-G15-7（G15.6a 决策门）+ G-G15-8（G15.6a 稳定门） | G15.2~G15.5 全波退出门绿 | 波内并行：P2 穷举决策表起草与 soak 全量回归分腿并行；M-e 门单线机核 |
| **G15.6b close-out** | 验收映射、候选决策、RD 最终状态逐字一致终审 + 全部 P0 独立断言均 PASS + evidence/schema/预算终审 + 商用收口终审定盘（达标/未达标如实登记不冒充；未达标按用户 2026-08-19 授权新建 G16+ 里程碑继续优化，性能零降级守护面终态锁定）+ 契约 §8 只追加后 status active→closed + 独立 commit + `g15-closed` tag | G-G15-9（G15.6b 收口门） | G15.6a 双门绿 | 单线收口（终审八 facts + flip commit + tag，不并行） |

## 3. P0 摘要表（契约 §4.2 五行逐字转引）

| M 行 | 判据（逐字） | 波次 |
|---|---|---|
| **M-a** | 双端画质对拍链路全量复跑（G13 M-c ue_upscale_parity + G13 M-d ue_lumen_gi_parity + G12 M163 ue_pt_parity 三门同口径复跑，对拍契约 digest 0-byte 门序维持）+ 20 行登记表逐项重评（逐行 gap_id 逐字转引 + fresh measured_delta + 方向判定〔收敛/维持/劣化〕）+ G15 差距处置表 `milestones/g15/g15_quality_gap_disposition.json` 落盘零空行 + UE 方差带程序产（G14 M-a 双程序产面取严口径继承）+ AI 读图基线臂（双场景 × 三档 × 三后端出图结构完整性断言） | G15.2 |
| **M-b** | measured 主差修复闭环：处置表 20 行逐行终态处置 ∈ {closed-resolved（修复后 fresh delta 进容差带，RXS-0393 收敛判据两款）/ closed-caliber-registered（口径差显式登记不拟合，RXS-0392）/ open-defer-G16+（承接锚字面「重判条件 = …；兜底 = …」）} 零空行 + 修复项 RED 先行（失败测试先落 main 为 RED）+ 触冻结面独立 Full RFC 留痕（D-409 对抗评审）+ 材质链表达面立项评估结论登记（G11-N8/G11-N9/G12-N10 承接锚命中判定逐字：透射/焦散/镜面 IBL 类能量是否成为画质量级 measured 主差） | G15.3 |
| **M-c** | 绝对画质通过线设立 + 严格画面审查：绝对通过线程序产标定（UE 参照 deficit 双 seed 方差底 p100×2.0 程序产，禁手写 P-09，标定链路入 evidence）+ 双场景 × 三档（t50/t67/t100）× 三后端（tsr_device/dlss_sr/fsr_3_1_5）18 格逐格判定 + 逐格 AI 读图严格画面审查记录（无乱序/无错位/无全黑/关键结构可见——cornell 盒体结构、bistro 吊灯/吧台/桌椅）+ 商用收口判定（达标格数/18 + 未达标格如实登记不冒充） | G15.4 |
| **M-d** | 性能零降级守护：G14 M-d 门同口径复跑（双场景 × 三档 × 三后端 18 格，三轮进程级独立运行 50×3 trimmed mean 跨轮中位数 + 逐轮守护带）逐格 ratio ≥ ×1.00 维持 + G14 M-c 画质锚带复核（SSIM deficit ≤ 0.010779849285388998 带内）+ G14 门产 budget 条目零 estimated 维持 + 画质修复致性能劣化静默即 RED | G15.5 |
| **M-e** | 回归门 + 漂移监控：既有 84 门（G9 34 + G10 14 + G11 14 + G12 9 + G13 5 + G14 8）最新 evidence 全绿只读汇总不遮蔽 + 触改面真跑抽检零降级 + RD-045/M165 同型 digest 漂移监控登记（G15 复跑面检出计数/零检出字面入 evidence） | G15.6a |

## 4. 风险与缓解

| 风险 | 影响面 | 缓解（契约字面锚） |
|---|---|---|
| **UE 5.8 环境可用性** | M-a 三门对拍复跑与 M-c 终审的 UE 参照臂依赖本机 UE 5.8.1（M128 登记机核继承）；环境不可用则对拍面无法真跑 | 缺硬件/工具链仅可 dev_env_degrade 或 SKIP=not-triggered，两者均不充 P0 绿（契约 guardrails 第 5 条）；host oracle/mock/既有最小见证均不能替代目标门；环境异常如实登记不冒充 |
| **GPU 锁（单机单 GPU 资源互斥）** | M-a/M-c/M-d 均含 GPU 真跑腿；波内并行分腿若同抢 GPU 将致帧时测量污染 | 波内并行分腿按 GPU 资源面串行调度（测量腿独占 GPU 窗，非 GPU 腿〔登记表重评/文档面/脚本面〕方可真并行）；50×3 trimmed mean 跨轮中位数 + 逐轮守护带口径维持抗扰 |
| **RD-045 间歇漂移（M165 同型族）** | G15 复跑面可能再检出同型 digest 漂移（间歇 ~0.6%/run 量级，根因未逐字定位） | M-e 门漂移监控臂承接（G15 复跑面检出计数/零检出字面入 evidence）；检出即如实登记并升级评估（生产化缺陷修复项 + Full RFC 评估）；零检出维持 open-defer 不写进全绿叙述、不关闭条目（G14PLUS_RECORD §6.2 字面）；flip-trace 诊断臂在树维持 |
| **画质修复致性能回退** | M-b 修复闭环可能致 G14 M-d 18/18 ×1.00 定盘面跌破通过线 | 性能零降级守护纪律（契约 guardrails 第 9 条）：G15 全期任何画质修复/终审复跑不得致 18 格 ratio 跌破 ×1.00；M-d 门复跑核验 = 每波退出硬前置；优化致性能劣化静默即 RED |
| **AI 读图误判风险** | M-c 终审 18 格 AI 读图为强制门，误判（假绿/假红）将污染商用收口判定 | G14.10f 教训字面兑现（digest 双跑一致 ≠ 内容正确）：M-a 先落 AI 读图基线臂（结构完整性断言口径先定标）；逐格读图记录入 evidence 可复核；digest 面不替代内容面、内容面亦不替代 digest 面（双断言独立）；读图结论存疑格如实登记升级人工复核，不冒充判定 |

## 5. 工期纪律

1. **measured baseline 校准**：契约 timebox 字面——G15.1 治理波即刻执行；G15.2~G15.6b 严格波次，**工期在实现互锁开放后由 measured baseline 校准**（禁拍脑袋估时）；G14 measured 帧时/画质锚带基线（`g14_budget.json`）与 G13 标定三条目双 seed 方差底（`g13_budget.json`）为对照基线输入。
2. **数字步骤延迟分配**：契约 guardrails 第 3 条字面——G15 P0 实现门 CI 只冻结 symbolic gate key 与脚本名；**numeric_step 一律写 post-interlock actual-next-free allocation**，不得沿用推测号与草案建议值，不得预放空 workflow、空脚本或空 schema 壳（G15.1 治理三门为例外：本波即落盘真脚本真步骤，步骤 266/267/268 = 落盘前实测 `registry/number_ledger.json` namespaces.CI_step next_free=266 顺位领取）；共享编号按互锁开放时 actual next_free 重新校准。
3. **波间不越级**：G-G15-2 实现互锁未过前 G15.2+ 禁止开工（implementation_status=blocked 字面）；任一前置退出门未真实通过，后续波次不启动。
4. **诚实降级三档**：缺硬件/工具链仅可 dev_env_degrade 或 SKIP=not-triggered；两者均不充 P0 绿，不写入全绿叙述。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-23 | 首版（G15.1 治理波定稿）：§1 目标与法定输入（契约 §1 字面 + G14PLUS_RECORD §6.3 承接锚 + 三表 20 行清单）+ §2 七波计划（内容/退出判据 id/依赖/波内并行·波间不越级策略）+ §3 P0 摘要表（契约 §4.2 五行逐字转引）+ §4 风险与缓解五项（UE 5.8 环境/GPU 锁/RD-045 间歇漂移/画质修复致性能回退/AI 读图误判）+ §5 工期纪律（measured baseline 校准 + 数字步骤延迟分配）。 |
