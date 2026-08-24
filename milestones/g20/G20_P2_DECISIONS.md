<!-- Assisted-by: Cursor Agent（G20.5 P2 穷举） -->
# G20_P2_DECISIONS — G20.5 P2 穷举决策表

> **闭集**：§1 九行 + §3 五行 = 14 行零空行。

## 1. 候选表 9 行终态裁决

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| M61 | mesh shader 第三光栅路径 | G20.1 go → G20.3 M-c 重判 | G19+ HZB/cluster P4 触发条件齐备 → VS 光栅唯一 fallback 维持 | maintain-no-go | HZB 半边兑现 + cluster 半边差距闭集 4 行全 open + HW 性能差 measured 证据仍缺（RFC-0034 重判记录只追加） | evidence/g20_m_c_mesh_shader_rejudgment_*.json + rfcs/0034 重判记录 | 重判条件 = cluster P4 差距闭集清零 + HZB device 化落地后只追加再判；兜底 = VS 光栅唯一 fallback 维持（字面 0-byte） | 本表 §1 + G20_ACCEPTANCE_MAP M-c | maintain-no-go |
| M98-l4 | M98 L4 Far Field 档 | G20.1 go → G20.3 M-d 重判 | G20 HLOD 接口面就绪 + L4 计数可测窗 → L1/L2/L3 三级链维持 | maintain-no-go | 接口面就绪命中（world/hlod.rs + g9.p1.m111 门绿）+ L4 计数可测未命中（HLOD proxy 追踪 device 腿零实现）——维持三级链诚实登记 | evidence/g20_m_d_far_field_l4_disposition_*.json | 重判条件 = HLOD proxy 追踪 device 腿落地 + L4 计数器接入选档 evidence；兜底 = L1/L2/L3 三级链维持 | 本表 §1 + G20_ACCEPTANCE_MAP M-d | maintain-no-go |
| M52 | SER / hit-object 重排 | G20.1 defer-to-G21+ | G21 光照期高分歧 RT workload + rt.ser 设备面实测窗 → 语言层不加 SER 原语维持 | defer-to-G21+ | G21 光照 P3+ 深化期即窗 | milestones/g20/G20_CANDIDATE_DECISIONS.md §1 M52 | 重判条件 = G21 rt.ser 设备面实测窗；兜底 = 语言层不加 SER 原语维持（字面 0-byte） | 本表 §1 行；registry/deferred.json RD-040 | open-defer（G21 窗） |
| SAFE-GPU | Safe GPU Operator Platform | G20.1 defer-to-G21+ | G24 立项评估处置窗 → G9~G20 零交付维持 | defer-to-G21+ | G24 处置窗承接 | milestones/g20/G20_CANDIDATE_DECISIONS.md §1 SAFE-GPU | 重判条件 = G24 立项评估处置窗；兜底 = G9~G20 零交付维持 | 本表 §1 行 | open-defer（G24 窗） |
| M127 | 神经变形研究子轨 | G20.1 defer-to-G21+ | G23 corpus + 消费方条件实测窗 → 无主线门研究子轨维持 | defer-to-G21+ | G23 物理平台深化期窗承接 | milestones/g20/G20_CANDIDATE_DECISIONS.md §1 M127 | 重判条件 = G23 corpus + PhysicsAsset residual 消费方实测窗；兜底 = 无主线门研究子轨维持 | 本表 §1 行 | open-defer（G23 窗） |
| M114-strand | 毛发 strand 档精确 OIT | G20.1 defer-to-G21+ | G24 M120 精确档 benchmark 裁决数据窗 → card/mesh 档维持 | defer-to-G21+ | G24 呈现与尾门清理期窗承接 | milestones/g20/G20_CANDIDATE_DECISIONS.md §1 M114-strand | 重判条件 = G24 M120 精确档 benchmark 裁决数据窗；兜底 = card/mesh 档维持 | 本表 §1 行 | open-defer（G24 窗） |
| M118-hdr-cal | HDR 设备标定层 | G20.1 defer-to-G21+ | G24 HDR 设备面实测窗 → g9.p0.m118 门绿 SDR 面维持 | defer-to-G21+ | G24 呈现与尾门清理期窗承接 | milestones/g20/G20_CANDIDATE_DECISIONS.md §1 M118-hdr-cal | 重判条件 = G24 HDR 设备面实测窗；兜底 = g9.p0.m118 门绿 SDR 面维持 | 本表 §1 行 | open-defer（G24 窗） |
| M125-adopt3 | Jolt 5.6 采纳臂⑦三件 | G20.1 defer-to-G21+ | G23 Jolt 升级评估窗采纳臂重评 → 5.3 基线生产默认维持 | defer-to-G21+ | G23 物理平台深化期窗承接 | milestones/g20/G20_CANDIDATE_DECISIONS.md §1 M125-adopt3 | 重判条件 = G23 Jolt 升级评估窗采纳臂重评；兜底 = 5.3 基线生产默认维持 | 本表 §1 行 | open-defer（G23 窗） |
| G10-N6 | BistroExterior 未入压测清单 | G20.1 defer-to-G21+ | G24 FBX2glTF 上游复查或替代转换臂窗 → BistroInterior + CornellBox 维持 | defer-to-G21+ | G24 呈现与尾门清理期窗承接 | milestones/g20/G20_CANDIDATE_DECISIONS.md §1 G10-N6 | 重判条件 = G24 FBX2glTF 上游复查或替代转换臂窗；兜底 = BistroInterior + CornellBox 维持 | 本表 §1 行 | open-defer（G24 窗） |

## 2. open RD 八条映射

| RD | status | G20.5 处置 |
|---|---|---|
| RD-034 | open | 维持 open |
| RD-039 | open | 维持 open（HZB host 兑现 + P4 差距闭集 4 行落档；device 化长线存续） |
| RD-040 | open | 维持 open（G21 承接） |
| RD-041 | open | 维持 open（G22 承接） |
| RD-042 | open | 维持 open（G23 承接） |
| RD-043 | open | 维持 open（G23 承接） |
| RD-044 | open | 维持 open（G23 承接） |
| RD-045 | open | 维持 open（长窗存续） |

## 3. G20 期内行 5 行终态

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G20-N1 | HZB host 参考臂实现 | G20.1 新增 | M-a 真跑后争议时重判 | closed-go | M-a 全绿（双约定 800 rect 零假阳性 + 剔除率 231/800 + 双跑位级） | evidence/g20_m_a_hzb_occlusion_host_realization_*.json + evidence/g20_hzb_probe_*.json | 重判条件 = 保守性争议时只追加程序重判；兜底 = 既有两级剔除链维持 | G20_ACCEPTANCE_MAP M-a | closed-go |
| G20-N2 | cluster 流送 P4 评估两态 | G20.1 新增 | M-b 真跑后争议时重判 | closed-go | M-b 全绿（差距闭集四行全 open + disposition=defer 如实登记） | evidence/g20_m_b_cluster_streaming_p4_disposition_*.json | 重判条件 = 差距闭集争议时只追加程序扩表；兜底 = 现 streaming 页式面维持 | G20_ACCEPTANCE_MAP M-b | closed-go |
| G20-N3 | M61 重判两态合法 | G20.1 新增 | M-c 真跑后争议时重判 | closed-go | M-c 全绿（maintain-no-go 裁决 + RFC-0034 重判记录只追加） | evidence/g20_m_c_mesh_shader_rejudgment_*.json | 重判条件 = maintain-no-go/go 均合法留档；兜底 = VS 光栅 fallback 维持 | G20_ACCEPTANCE_MAP M-c | closed-go |
| G20-N4 | M98-l4 重判两态合法 | G20.1 新增 | M-d 真跑后争议时重判 | closed-go | M-d 全绿（维持三级链裁决 + 接口面/计数可测两半如实核验） | evidence/g20_m_d_far_field_l4_disposition_*.json | 重判条件 = 实现/维持三级链均合法留档；兜底 = L1/L2/L3 三级链维持 | G20_ACCEPTANCE_MAP M-d | closed-go |
| G20-N5 | 旧门零降级闭集 | G20.1 新增 | M-e 真跑后争议时重判 | closed-go | M-e 全绿（G19 链 verify-latest 零降级 + g20_ 前缀不抢 latest） | evidence/g20_m_e_closed_gate_no_regression_*.json | 重判条件 = 受影响门集合争议时只追加程序扩表；兜底 = verify-latest 纪律维持 | G20_ACCEPTANCE_MAP M-e | closed-go |

## 4. 版本

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | 首版：§1 九行 + §3 五行 = 14 行候选闭集零空行（closed-go 5 + maintain-no-go 2 + defer-to-G21+ 7）。 |
