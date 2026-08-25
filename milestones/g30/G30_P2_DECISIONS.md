<!-- Assisted-by: Cursor Agent（G30.5 P2 穷举波） -->
# G30_P2_DECISIONS — G30.5 P2 穷举决策表（v1.0 2026-08-25）

> **状态**：G30.5 收口前置定稿。**穷举闭集 12 行零空行** = §1 七行 + §3 五行；§2 open RD 八条映射（登记面）。
> **裁决枚举**：`go` / `closed-go` / `no-go` / `maintain-no-go` / `maintain-defer` / `maintain-open` / `maintain-blocked` / `defer-to-G31+` / `strategic_override`。
> **候选表 0-byte**：[G30_CANDIDATE_DECISIONS.md](G30_CANDIDATE_DECISIONS.md) 裁决字面不回写，本表为终态穷举。

## 1. 上游承接七行终态（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| M125-adopt3 | Jolt 5.6 采纳窗 | G30.1 go → G30.2 M-a 兑现 | 需求证据三类任一命中(5.6 独有 API 引用/5.3 缺陷命中/A/B 超带)→ maintain-5.3 维持 | closed-go | 重判窗兑现：三类需求证据逐类独立 manifest 全空（6 pattern 常量表 + 锚派生映射——F6）+ sys56 评估臂 cargo check 新鲜绿（可编译性硬前提）+ g9_m125 A/B latest 只读盘点（F17 检证面不缩面）→ maintain-5.3（在案三件条件 1/3 不变） | evidence/g30_m_a_tail_anchor_rejudgment_closure_20260825T102516Z.json + milestones/g23/g23_jolt_adoption_registry.json | 重判条件 = 三类任一命中时启动采纳切换评估（切换实现归后续窗）；兜底 = maintain-5.3 维持诚实终态（归档锚 M-d tail_six 行） | 本表 §1 + M-d 归档表 tail_six | closed-go（M-a 兑现：maintain-5.3 维持） |
| M127 | 神经变形研究子轨 | G30.1 go → G30.2 M-a 兑现 | corpus + PhysicsAsset residual 消费方出现 → maintain 研究子轨维持 | closed-go | 重判窗兑现：corpus 四目录存在性 NONE + neural_deform 消费方 token NONE（g23 检索面逐字沿用禁缩面，5 pattern）→ 维持研究子轨 | evidence/g30_m_a_tail_anchor_rejudgment_closure_20260825T102516Z.json | 重判条件 = 两半任一命中时研究子轨重判启动；兜底 = maintain 研究子轨维持 + 搜索面闭集只追加扩面（归档锚 M-d tail_six 行） | 本表 §1 + M-d 归档表 tail_six | closed-go（M-a 兑现：maintain 研究子轨） |
| M114-strand | 毛发 strand 档 | G30.1 go → G30.2 M-a 兑现 | 毛发资产入压测闭集 → maintain card/mesh 维持 | closed-go | 重判窗兑现：契约 strand token NONE + 外部盘 hair 面检索根不可达 SKIP 如实登记 + 在案态兜底（F15 三态闭集，不 FAIL 不冒充命中）→ 维持 card/mesh | evidence/g30_m_a_tail_anchor_rejudgment_closure_20260825T102516Z.json | 重判条件 = 毛发资产入压测闭集命中时 strand 档评估启动；兜底 = maintain card/mesh 维持诚实终态（归档锚 M-d tail_six 行） | 本表 §1 + M-d 归档表 tail_six | closed-go（M-a 兑现：maintain card/mesh，SKIP 兜底） |
| M118-hdr-cal | HDR 标定 | G30.1 go → G30.2 M-a 兑现 | 显示链变化 + HDR 资产需求成立 → maintain-SDR 维持 | closed-go | 重判窗兑现：vulkaninfo 新鲜探针三 token（HDR10_ST2084/BT2020_LINEAR/HDR10_HLG，g24 probe 常量逐字）全 absent（三态闭集 RFC-0046 §4.2 同律，全量 log 存档）→ 维持 maintain-SDR | evidence/g30_m_a_tail_anchor_rejudgment_closure_20260825T102516Z.json | 重判条件 = HDR token present + 资产需求两半同窗命中时标定评估启动；兜底 = maintain-SDR 维持诚实终态（归档锚 M-d tail_six 行） | 本表 §1 + M-d 归档表 tail_six | closed-go（M-a 兑现：maintain-SDR，absent 实测） |
| G10-N6 | BistroExterior 转换臂 | G30.1 go → G30.2 M-a 兑现 | FBX2glTF 上游修复在树或替代臂+源资产同窗齐备 → 维持双场景闭集 | closed-go | 重判窗兑现：fbx2gltf/assimp/blender 三工具 PATH 实测全缺（含 FBX2glTF 变体）+ BistroExterior 源资产三根检索 0 命中（K: 根态逐根如实登记）→ 维持双场景闭集（BistroInterior + CornellBox 兜底字面 0-byte） | evidence/g30_m_a_tail_anchor_rejudgment_closure_20260825T102516Z.json | 重判条件 = 工具链任一在树 + 源资产同窗齐备时转换臂重判启动；兜底 = 双场景闭集维持诚实终态（归档锚 M-d tail_six 行） | 本表 §1 + M-d 归档表 tail_six | closed-go（M-a 兑现：维持双场景闭集） |
| SAFE-GPU | Safe GPU Operator Platform | G30.1 go → G30.2 M-a 兑现 | 独立期资源窗 + 平台需求方(外部采纳生态)出现时立项评估 → G9~G29 零交付维持 | closed-go | 重判窗兑现：收官期专属资源窗不成立（判据字面直接不成立如实登记——RFC-0047 §1.6）+ 平台需求方文档树内 0 命中 → 维持 defer，归档行改锚 defer-to-G31+ | evidence/g30_m_a_tail_anchor_rejudgment_closure_20260825T102516Z.json | 重判条件 = 资源窗 + 需求方两半同窗命中时立项评估启动；兜底 = 零交付维持诚实终态 + 归档锚 defer-to-G31+（M-d tail_six 行） | 本表 §1 + M-d 归档表 tail_six | closed-go（M-a 兑现：defer 维持，改锚 defer-to-G31+） |
| G17-MD-F1 | fps_parity_deficit bistro/t100/dlss_sr 终判链 | G30.1 go → G30.2 M-b 兑现 | NGX 分解 profiling 或 UE 侧插桩(宿主差可分离 measured 证据)→ 17/18 诚实红 carry;G26 M-d 登记终判归 G30 | closed-go | 终判法定义务兑现：18 格定盘 met=17/18（G14 latest）+ 性能面三文件 vs g25-closed 0-byte + 两半锚 6 pattern G30 新鲜检索零命中（F3 断言升格机器取证）⇒ ratio 登记面即重判执行体 + 焦点格 canonical 160 帧新鲜真跑（RURIX_REQUIRE_REAL=1 + GPU 独占窗 + bench_receipt 新鲜）frame_ms=3.5767ms、新鲜 ratio=0.960479 < 1.00 物理不可达 → **维持 17/18 诚实红终判**（G15 兜底 + G25 M-b 两态同源，合法收官态零冒充） | evidence/g30_m_b_commercial_final_review_20260825T102813Z.json + evidence/g14_m_d_dual_end_fps_parity_20260824T091444Z.json | 重判条件 = 两半锚命中或焦点格 ratio ≥1.00 新证出现时只追加程序重判；兜底 = 17/18 诚实红终判合法（终态归档槽位 = M-d 归档表 campaign_period_rows G30 期行） | 本表 §1 + M-d 归档表 G30 期行 | closed-go（M-b 兑现：17/18 诚实红终判定盘，新鲜 ratio=0.960479 登记） |

## 2. open RD 逐条映射（八条口径；登记面）

| RD | title（摘要） | 条目级 status | G30.5 处置 | 联动面 | 裁决理由 | 留痕位置 |
|---|---|---|---|---|---|---|
| RD-034 | DXIL RT/mesh 腿 | open | 维持 open（G28 复查 maintain-blocked 在案；归档 G31+ 锚〔M-d rd_eight〕） | 无 | 上游仍拒 | 本表 §2 + M-d 归档表 |
| RD-039 | 虚拟化几何 P3+ | open | 维持 open（G27 HZB 分项兑现在案；归档 G31+ 锚〔M-d rd_eight〕） | 无 | G31+ 承接（归档锚字面） | 本表 §2 + M-d 归档表 |
| RD-040 | 光照 P3+ | open | 维持 open（G28 M100-high 两件兑现在案；归档 G31+ 锚〔M-d rd_eight〕） | 无 | G31+ 承接（归档锚字面） | 本表 §2 + M-d 归档表 |
| RD-041 | 材质/流送/时域 P3+ | open | 维持 open（G29 四面兑现在案；归档 G31+ 锚〔M-d rd_eight〕） | 无 | G31+ 承接（归档锚字面） | 本表 §2 + M-d 归档表 |
| RD-042 | 可微物理研究轨 | open | M-a 尾锚窗承载兑现（锚 2 pattern 零命中维持 open + history G30.2 只追加〔幂等 F9〕） | M-a | 锚源钉死 g25 rd_eight g26_anchor 字面（F8），零命中不改判 | 本表 §2 + registry/deferred.json RD-042 history |
| RD-043 | wgrapier GPU 刚体 | open | M-a 尾锚窗承载兑现（锚 3 pattern 零命中维持 open + history G30.2 只追加〔幂等 F9〕） | M-a | 锚源钉死 g25 rd_eight g26_anchor 字面（F8），零命中不改判 | 本表 §2 + registry/deferred.json RD-043 history |
| RD-044 | 物理 P3+ | open | M-a 尾锚窗承载兑现（三分项 reeval_anchor 展开 6 pattern 零命中维持 open + history G30.2 只追加〔幂等 F9〕） | M-a | 检索面显式展开 = g23_rd044_subitem_registry 三分项字面（F8），零命中不改判 | 本表 §2 + registry/deferred.json RD-044 history |
| RD-045 | digest 漂移修复 | open | M-b 确定性面复核承载兑现（g25~g29 五期 soak latest 逐期只读盘点全绿 + Stage A 18/18 + status=open 核验——F14 判定面钉死） | M-b | 三件未齐不冒充 close（累计观察锚归 M-d rd_eight 行） | 本表 §2 + M-d 归档表 |

## 3. G30 期新增候选 5 行终态（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G30-N1 | 尾锚六件重判协议 | G30.1 新增 → G30.2 兑现 | M-a 真跑后争议时重判 | closed-go | M-a 29 facts 全绿（九组常量 pattern 表 + 锚源钉死机核 F8 + 三态 SKIP 兜底 F15 + deferred 追加幂等 F9 + F6 三件 + F10 门态映射） | evidence/g30_m_a_tail_anchor_rejudgment_closure_20260825T102516Z.json | 重判条件 = 六件检索面或 searched-paths manifest 闭集争议时只追加程序扩面；兜底 = 逐件维持诚实终态零冒充 + deferred history 只追加 | G30_ACCEPTANCE_MAP M-a | closed-go（M-a） |
| G30-N2 | 三面商用终审协议 | G30.1 新增 → G30.2 兑现 | M-b 真跑后争议时重判 | closed-go | M-b 16 facts 全绿（画质十项 0-byte + 两层零接线 F2 + G18/g25 传递环盘点 F6 + 性能 18 格定盘 + 三文件全路径 0-byte F11 + 两半锚新鲜检索 F3 + 焦点格 160 帧真跑 + 17/18 诚实红终判 + Stage A 18/18 + 四 device 双跑绿件 + RD-045 复核 F14） | evidence/g30_m_b_commercial_final_review_20260825T102813Z.json | 重判条件 = 画质/性能/确定性三面任一定盘面争议时只追加程序重判；兜底 = G18 达标维持 + 17/18 诚实红终判合法 + Stage A digest 锚维持 | G30_ACCEPTANCE_MAP M-b | closed-go（M-b） |
| G30-N3 | 战役全链递归核验 | G30.1 新增 → G30.3 兑现 | M-c 真跑后争议时重判 | closed-go | M-c 7 facts 全绿（tag 11/11 逐 tag 列举 + verify g29 两门 rc=0 + budget --strict 301 pass 0 skip 禁 --allow-pending F18 + verify-latest 语义如实化字面 F4） | evidence/g30_m_c_campaign_full_chain_no_regression_20260825T102838Z.json | 重判条件 = 递归链集合争议时只追加程序扩表；兜底 = verify-latest 纪律维持 | G30_ACCEPTANCE_MAP M-c | closed-go（M-c） |
| G30-N4 | G31+ 交接归档闭集 | G30.1 新增 → G30.3 兑现 | M-d 真跑后争议时重判 | closed-go | M-d 8 facts 全绿（顶层七键闭集 + 9 期行分 section 字段闭集 F5 + G17-MD-F1 归档槽位 + rd_eight 与 deferred 实测一致 + tail_six 逐行 evidence/g30_ 引用 + legacy_eleven_source 字面 F13 + 上游 g25 registry 0-byte） | evidence/g30_m_d_campaign_handover_ledger_20260825T103120Z.json + milestones/g30/g30_campaign_handover_registry.json | 重判条件 = 归档完整性争议时只追加程序扩表；兜底 = 各期 P2 表原始锚 0-byte 维持 | G30_ACCEPTANCE_MAP M-d | closed-go（M-d） |
| G30-N5 | G29 链回归守护 + 战役总结报告 | G30.1 新增 → G30.4/G30.6 兑现 | M-e 真跑后争议时重判 | closed-go | M-e 7 facts 全绿（verify g29 两门 rc=0 + g30_ 前缀零抢占 + M-c/M-e 分工声明 F10 + Stage A 锚零漂移登记）；战役总结报告归 close-out 签署块（G30.6 承载） | evidence/g30_m_e_closed_gate_no_regression_20260825T103138Z.json | 重判条件 = 受影响门集合争议时只追加程序扩表；兜底 = verify-latest 纪律维持 + 战役总结报告归 close-out 签署块 | G30_ACCEPTANCE_MAP M-e | closed-go（M-e/收官承载） |

## 4. 汇总

closed-go §1 七行 + §3 五行 = 12 行穷举闭集零空行；§2 open RD 八条维持 open（RD-042/043/044 M-a 尾锚窗承载 + RD-045 M-b 复核承载 + 其余四条归档 G31+ 锚）；零 no-go、零 defer-to-G31+ 新消费（归档锚承接面 = M-d 归档表唯一法定输入面）。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-25 | G30.5 定稿：12 行穷举闭集。 |
