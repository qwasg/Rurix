<!-- Assisted-by: Cursor Agent(G30.1 治理波) -->
# G30_CANDIDATE_DECISIONS — G30.1 候选决策表（v1.0 2026-08-25）

> **状态**：G30.1 治理波定稿。**候选闭集 12 行零空行** = §1 七行 + §3 五行。
> **裁决枚举**：`go` / `closed-go` / `no-go` / `defer-to-G31+` / `strategic_override`。**G30 即收官期：defer 合法值 = defer-to-G31+（归档锚承接）**。

## 1. G25 交接登记表行 = 7 行逐行转引终态裁决（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| M125-adopt3 | Jolt 5.6 采纳窗 | G23.3 maintain-5.3 → G25 M-d 归档 → G30.1 go | 需求证据三类任一命中(5.6 独有 API 引用/5.3 缺陷命中/A/B 超带)→ maintain-5.3 维持 | go | M-a 承载（六件尾锚重判之一：需求证据三类树内实测 + sys56 评估臂 cargo check 新鲜；采纳切换实现 out-of-scope——重判只核验） | milestones/g25/g25_campaign_handover_registry.json M125-adopt3 行 + milestones/g23/G23_P2_DECISIONS.md §1 | 重判条件 = 需求证据三类任一命中时启动采纳切换评估（切换实现归后续窗）；兜底 = maintain-5.3 维持诚实终态 + deferred history 只追加 | 本表 §1 + G30_ACCEPTANCE_MAP M-a | go（M-a 承载） |
| M127 | 神经变形研究子轨 | G23.3 maintain → G25 M-d 归档 → G30.1 go | corpus + PhysicsAsset residual 消费方出现 → maintain 研究子轨维持 | go | M-a 承载（corpus 目录 + PhysicsAsset residual 消费方检索两半实测） | milestones/g25/g25_campaign_handover_registry.json M127 行 + milestones/g23/G23_P2_DECISIONS.md §1 | 重判条件 = 两半任一命中时研究子轨重判启动；兜底 = maintain 研究子轨维持 + 搜索面闭集只追加扩面 | 本表 §1 + G30_ACCEPTANCE_MAP M-a | go（M-a 承载） |
| M114-strand | 毛发 strand 档 | G24.2 maintain card/mesh → G25 M-d 归档 → G30.1 go | 毛发资产入压测闭集 → maintain card/mesh 维持 | go | M-a 承载（毛发资产入压测闭集检索实测） | milestones/g25/g25_campaign_handover_registry.json M114-strand 行 + milestones/g24/G24_P2_DECISIONS.md §1 | 重判条件 = 毛发资产入压测闭集命中时 strand 档评估启动；兜底 = maintain card/mesh 维持诚实终态 | 本表 §1 + G30_ACCEPTANCE_MAP M-a | go（M-a 承载） |
| M118-hdr-cal | HDR 标定 | G24.2 maintain-SDR → G25 M-d 归档 → G30.1 go | 显示链变化 + HDR 资产需求成立 → maintain-SDR 维持 | go | M-a 承载（vulkaninfo HDR token 新鲜探针；hdr_display_chain_implementation out-of-scope——重判只核验） | milestones/g25/g25_campaign_handover_registry.json M118-hdr-cal 行 + milestones/g24/G24_P2_DECISIONS.md §1 | 重判条件 = HDR token present + 资产需求两半同窗命中时标定评估启动（显示链实现归后续窗）；兜底 = maintain-SDR 维持诚实终态 | 本表 §1 + G30_ACCEPTANCE_MAP M-a | go（M-a 承载） |
| G10-N6 | BistroExterior 转换臂 | G24.3 维持双场景闭集 → G25 M-d 归档 → G30.1 go | FBX2glTF 上游修复在树或替代臂+源资产同窗齐备 → 维持双场景闭集 | go | M-a 承载（fbx2gltf/assimp/blender 三工具 PATH 实测 + 源资产检索） | milestones/g25/g25_campaign_handover_registry.json G10-N6 行 + milestones/g24/G24_P2_DECISIONS.md §1 | 重判条件 = 工具链任一在树 + 源资产同窗齐备时转换臂重判启动；兜底 = 双场景闭集维持诚实终态 | 本表 §1 + G30_ACCEPTANCE_MAP M-a | go（M-a 承载） |
| SAFE-GPU | Safe GPU Operator Platform | G25.3 defer-to-G26+ 归档 → G30.1 go | 独立期资源窗 + 平台需求方(外部采纳生态)出现时立项评估 → G9~G29 零交付维持 | go | M-a 承载（独立期资源窗 + 平台需求方文档检索两半实测） | milestones/g25/g25_campaign_handover_registry.json SAFE-GPU 行 + milestones/g24/g24_legacy_rd_registry.json | 重判条件 = 资源窗 + 需求方两半同窗命中时立项评估启动；兜底 = 零交付维持诚实终态 + G31+ 归档锚（M-d） | 本表 §1 + G30_ACCEPTANCE_MAP M-a | go（M-a 承载） |
| G17-MD-F1 | fps_parity_deficit bistro/t100/dlss_sr 终判链 | G25.2 M-b 17/18 诚实红 → G26.3 M-d 重判 carry → G30.1 go | NGX 分解 profiling 或 UE 侧插桩(宿主差可分离 measured 证据)→ 17/18 诚实红 carry;G26 M-d 登记终判归 G30 | go | M-b 承载（性能面终判法定义务：最新 18 格定盘 + 性能面 0-byte 机核 + 焦点格新鲜单测真跑；≥1.00 → 18/18 或物理不可达维持 17/18 诚实红终判两态均合法收官态） | milestones/g25/g25_campaign_handover_registry.json G17-MD-F1 行 + milestones/g26/G26_P2_DECISIONS.md §1 | 重判条件 = M-b 真跑后争议时只追加程序重判；兜底 = 17/18 诚实红终判合法（G15 物理不可达定论同源） | 本表 §1 + G30_ACCEPTANCE_MAP M-b | go（M-b 承载） |

## 2. open RD 逐条映射（八条口径）

| RD | title(摘要) | 条目级 status | G30.1 处置 | 联动面 | 裁决理由 | 留痕位置 |
|---|---|---|---|---|---|---|
| RD-034 | DXIL RT/mesh 腿 | open | 归档 G31+ 锚（M-d） | 无 | G28 复查在案（maintain-blocked 探针新鲜），上游仍拒 | 本表 §2 + M-d 归档表 |
| RD-039 | 虚拟化几何 P3+ | open | 归档 G31+ 锚（M-d） | 无 | G27 重判在案（HZB device 化 implemented + cluster P4 四行维持 open） | 本表 §2 + M-d 归档表 |
| RD-040 | 光照 P3+ | open | 归档 G31+ 锚（M-d） | 无 | G28 重判在案（M100-high 两件兑现 + M52 maintain-defer + 五分项维持 defer） | 本表 §2 + M-d 归档表 |
| RD-041 | 材质/流送/时域 P3+ | open | 归档 G31+ 锚（M-d） | 无 | G29 四面兑现在案（slab device kernel + 侧表 implemented + SVT/KTX2 七行 maintain-defer + WG not-available 维持，history G29.3 只追加） | 本表 §2 + M-d 归档表 |
| RD-042 | 可微物理研究轨 | open | M-a 尾锚窗承载（同批逐锚重判） | M-a | G25 归档锚在案（可微仿真需求场景未出现）——G30 尾锚窗兑现重判 | 本表 §2 + G30_ACCEPTANCE_MAP M-a + registry/deferred.json RD-042 history |
| RD-043 | wgrapier GPU 刚体 | open | M-a 尾锚窗承载（同批逐锚重判） | M-a | G25 归档锚在案（out_of_scope 翻转程序 + wgrapier 成熟度证据未出现）——G30 尾锚窗兑现重判 | 本表 §2 + G30_ACCEPTANCE_MAP M-a + registry/deferred.json RD-043 history |
| RD-044 | 物理 P3+ | open | M-a 尾锚窗承载（同批逐锚重判） | M-a | G25 归档锚在案（三分项 reeval_anchor）——G30 尾锚窗兑现重判 | 本表 §2 + G30_ACCEPTANCE_MAP M-a + registry/deferred.json RD-044 history |
| RD-045 | digest 漂移修复 | open | M-b 确定性面复核承载 | M-b | G26 M-c maintain-open 重判在案（新鲜窗 6/6 零漂移 + 三件盘点 0/3 只追加扩窗）——G30 确定性面 Stage A digest 锚在档复核 | 本表 §2 + G30_ACCEPTANCE_MAP M-b |

## 3. G30 期新增候选 5 行（零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G30-N1 | 尾锚六件重判协议 | G30.1 新增 | M-a 真跑后争议时重判 | go | M-a 判据承载 | G30_CONTRACT §4.2 M-a | 重判条件 = 六件检索面或 searched-paths manifest 闭集争议时只追加程序扩面；兜底 = 逐件维持诚实终态零冒充 + deferred history 只追加 | G30_ACCEPTANCE_MAP M-a | go（M-a） |
| G30-N2 | 三面商用终审协议 | G30.1 新增 | M-b 真跑后争议时重判 | go | M-b 判据承载 | G30_CONTRACT §4.2 M-b | 重判条件 = 画质/性能/确定性三面任一定盘面争议时只追加程序重判；兜底 = G18 达标维持 + 17/18 诚实红终判合法（G15 物理不可达定论同源）+ Stage A digest 锚维持 | G30_ACCEPTANCE_MAP M-b | go（M-b） |
| G30-N3 | 战役全链递归核验 | G30.1 新增 | M-c 真跑后争议时重判 | go | M-c 判据承载 | G30_CONTRACT §4.2 M-c | 重判条件 = 递归链集合争议时只追加程序扩表；兜底 = verify-latest 纪律维持 | G30_ACCEPTANCE_MAP M-c | go（M-c） |
| G30-N4 | G31+ 交接归档闭集 | G30.1 新增 | M-d 真跑后争议时重判 | go | M-d 判据承载 | G30_CONTRACT §4.2 M-d | 重判条件 = 归档完整性争议时只追加程序扩表；兜底 = 各期 P2 表原始锚 0-byte 维持 | G30_ACCEPTANCE_MAP M-d | go（M-d） |
| G30-N5 | G29 链回归守护 + 战役总结报告 | G30.1 新增 | M-e 真跑后争议时重判 | go | M-e 判据 + G30.6 收官承载 | G30_CONTRACT §4.2 M-e | 重判条件 = 受影响门集合争议时只追加程序扩表；兜底 = verify-latest 纪律维持 + 战役总结报告归 close-out 签署块 | G30_ACCEPTANCE_MAP M-e | go（M-e/收官承载） |

## 4. 汇总

go §1 七行 + §3 五行 = 12 go 承载面；零 defer 行（G30 即收官期，defer 合法值 = defer-to-G31+ 归档锚，本波零消费——G31+ 法定输入 = M-d 归档闭集）；§2 open RD 八条维持 open（RD-042/043/044 → M-a 尾锚窗承载 + RD-045 → M-b 确定性面复核承载 + 其余四条归档 G31+ 锚）。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-25 | 首版：12 行候选闭集。 |
