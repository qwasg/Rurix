<!-- Assisted-by: Cursor Agent（G19.5 P2 穷举） -->
# G19_P2_DECISIONS — G19.5 P2 穷举决策表

> **闭集**：§1 九行 + §3 五行 = 14 行零空行。

## 1. 候选表 9 行终态裁决

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G13-N7 | 帧生成 FG/MFG 独立层 | G18.8 defer-to-G19+ → G19.1 go | RFC-0035 终态落档后按只追加程序重判 → FG/MFG 零实现维持 | closed-go | M-a host 参考臂 implemented（三档全绿逐帧优于 frame-hold）+ M-b vendor 三臂 disposition 落档 | evidence/g19_m_a_frame_generation_host_realization_*.json + evidence/g19_frame_gen_probe_*.json | 重判条件 = device kernel 车道 G25 终审窗或后续期重判；兜底 = host 参考臂 + presented 独立口径维持 | 本表 §1 + G19_ACCEPTANCE_MAP M-a/M-b | closed-go |
| M52 | SER / hit-object 重排 | G19.1 defer-to-G20+ | G21 光照期高分歧 RT workload + rt.ser 设备面实测窗 → 语言层不加 SER 原语维持 | defer-to-G20+ | G19 主轨不交集；G21 光照 P3+ 深化期窗承接 | milestones/g19/G19_CANDIDATE_DECISIONS.md §1 M52 | 重判条件 = G21 rt.ser 设备面实测窗；兜底 = 语言层不加 SER 原语维持（字面 0-byte） | 本表 §1 行；registry/deferred.json RD-040 | open-defer（G21 窗） |
| SAFE-GPU | Safe GPU Operator Platform | G19.1 defer-to-G20+ | G24 立项评估处置窗 → G9~G19 零交付维持 | defer-to-G20+ | G19 非独立期；G24 处置窗承接 | milestones/g19/G19_CANDIDATE_DECISIONS.md §1 SAFE-GPU | 重判条件 = G24 立项评估处置窗；兜底 = G9~G19 零交付维持 | 本表 §1 行 | open-defer（G24 窗） |
| M127 | 神经变形研究子轨 | G19.1 defer-to-G20+ | G23 corpus + 消费方条件实测窗 → 无主线门研究子轨维持 | defer-to-G20+ | 与 G19 主轨零依赖；G23 物理平台深化期窗承接 | milestones/g19/G19_CANDIDATE_DECISIONS.md §1 M127 | 重判条件 = G23 corpus + PhysicsAsset residual 消费方实测窗；兜底 = 无主线门研究子轨维持 | 本表 §1 行 | open-defer（G23 窗） |
| M98-l4 | M98 L4 Far Field 档 | G19.1 defer-to-G20+ | G20 HLOD 接口面就绪 + L4 计数可测窗 → L1/L2/L3 三级链维持 | defer-to-G20+ | G20 虚拟化几何 P4 期窗承接（下一期即窗） | milestones/g19/G19_CANDIDATE_DECISIONS.md §1 M98-l4 | 重判条件 = G20 HLOD 接口面就绪 + L4 计数可测窗；兜底 = L1/L2/L3 三级链维持 | 本表 §1 行 | open-defer（G20 窗） |
| M114-strand | 毛发 strand 档精确 OIT | G19.1 defer-to-G20+ | G24 M120 精确档 benchmark 裁决数据窗 → card/mesh 档维持 | defer-to-G20+ | G24 呈现与尾门清理期窗承接 | milestones/g19/G19_CANDIDATE_DECISIONS.md §1 M114-strand | 重判条件 = G24 M120 精确档 benchmark 裁决数据窗；兜底 = card/mesh 档维持 | 本表 §1 行 | open-defer（G24 窗） |
| M118-hdr-cal | HDR 设备标定层 | G19.1 defer-to-G20+ | G24 HDR 设备面实测窗 → g9.p0.m118 门绿 SDR 面维持 | defer-to-G20+ | G24 呈现与尾门清理期窗承接 | milestones/g19/G19_CANDIDATE_DECISIONS.md §1 M118-hdr-cal | 重判条件 = G24 HDR 设备面实测窗；兜底 = g9.p0.m118 门绿 SDR 面维持 | 本表 §1 行 | open-defer（G24 窗） |
| M125-adopt3 | Jolt 5.6 采纳臂⑦三件 | G19.1 defer-to-G20+ | G23 Jolt 升级评估窗采纳臂重评 → 5.3 基线生产默认维持 | defer-to-G20+ | G23 物理平台深化期窗承接 | milestones/g19/G19_CANDIDATE_DECISIONS.md §1 M125-adopt3 | 重判条件 = G23 Jolt 升级评估窗采纳臂重评；兜底 = 5.3 基线生产默认维持 | 本表 §1 行 | open-defer（G23 窗） |
| G10-N6 | BistroExterior 未入压测清单 | G19.1 defer-to-G20+ | G24 FBX2glTF 上游复查或替代转换臂窗 → BistroInterior + CornellBox 维持 | defer-to-G20+ | G24 呈现与尾门清理期窗承接 | milestones/g19/G19_CANDIDATE_DECISIONS.md §1 G10-N6 | 重判条件 = G24 FBX2glTF 上游复查或替代转换臂窗；兜底 = BistroInterior + CornellBox 维持 | 本表 §1 行 | open-defer（G24 窗） |

## 2. open RD 八条映射

| RD | status | G19.5 处置 |
|---|---|---|
| RD-034 | open | 维持 open |
| RD-039 | open | 维持 open（G20 承接） |
| RD-040 | open | 维持 open（G21 承接） |
| RD-041 | open | 维持 open（FG 分项 G19 兑现留痕 history 不改条目；其余分项 G22 承接） |
| RD-042 | open | 维持 open（G23 承接） |
| RD-043 | open | 维持 open（G23 承接） |
| RD-044 | open | 维持 open（G23 承接） |
| RD-045 | open | 维持 open（G19.3 观察窗 12/12 零漂移 history 只追加；backfill 三件未全齐不冒充 close） |

## 3. G19 期内行 5 行终态

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G19-N1 | FG/MFG host 参考臂实现 | G19.1 新增 | M-a 真跑后争议时重判 | closed-go | M-a 全绿（×2/×3/×4 三档逐帧优于 frame-hold + 双跑位级 + 口径恒等式） | evidence/g19_m_a_frame_generation_host_realization_*.json | 重判条件 = 参考臂质量争议时只追加程序重判；兜底 = 默认臂 digest 零漂移维持 | G19_ACCEPTANCE_MAP M-a | closed-go |
| G19-N2 | vendor 三臂 disposition 三态 | G19.1 新增 | M-b 真跑后争议时重判 | closed-go | M-b 全绿（fsr3_fg rejected / dlss_g not_available / sl_310_6_0 not_available 均有 rationale + 证据锚） | evidence/g19_m_b_frame_generation_vendor_disposition_*.json + milestones/g19/g19_vendor_sdk_registry.json | 重判条件 = disposition 争议时只追加程序重判；兜底 = 310.5.2 生产默认 + FG vendor 零集成如实登记 | G19_ACCEPTANCE_MAP M-b | closed-go |
| G19-N3 | RD-045 长窗观察两态 | G19.1 新增 | M-c 真跑后争议时重判 | closed-go | M-c 全绿（canonical 160 帧 12 轮 12/12 中锚零漂移；maintain-open 诚实登记） | evidence/g19_m_c_rd045_drift_observation_window_*.json + milestones/g19/g19_rd045_observation_results.json | 重判条件 = 观察窗争议时只追加程序重判；兜底 = RD-045 maintain-open 诚实登记 | G19_ACCEPTANCE_MAP M-c | closed-go |
| G19-N4 | fps 重评窗登记两态合法 | G19.1 新增 | M-d 真跑后争议时重判 | closed-go | M-d 全绿（17/18 诚实红 carry 如实登记；焦点格 ratio 0.856326；终判归 G25） | evidence/g19_m_d_fps_parity_window_registration_*.json | 重判条件 = 物理不可达时维持未达标登记；兜底 = ×1.00 口径 0-byte，终判归 G25 | G19_ACCEPTANCE_MAP M-d | closed-go |
| G19-N5 | 旧门零降级闭集 | G19.1 新增 | M-e 真跑后争议时重判 | closed-go | M-e 全绿（G18 链 verify-latest 零降级 + g19_ 前缀不抢 latest） | evidence/g19_m_e_closed_gate_no_regression_*.json | 重判条件 = 受影响门集合争议时只追加程序扩表；兜底 = verify-latest 纪律维持 | G19_ACCEPTANCE_MAP M-e | closed-go |

## 4. 版本

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | 首版：§1 九行 + §3 五行 = 14 行候选闭集零空行（closed-go 6 + defer-to-G20+ 8）。 |
