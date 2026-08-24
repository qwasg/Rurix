<!-- Assisted-by: Cursor Grok 4.6（G16.1 治理波） -->
# G16_CANDIDATE_DECISIONS — G16.1 候选决策表（G16_CONTRACT D-G16-2；v1.0 2026-08-24）

> **状态**：G16.1 治理波定稿 v1.0（2026-08-24）。**穷举闭集 20 行零空行** = §1 G15 defer-to-G16+ 十四行 + G15-MC-F1/G15-MD-F1 承接（16 行）+ §3 G16 新增候选 4 行；另 §2 open RD 八条逐条映射（条目级 status 全维持 open 0-byte，不计入三值枚举行数）。
> **事实源**：[G16_CONTRACT.md](G16_CONTRACT.md) v1.0 · [G15_P2_DECISIONS.md](../g15/G15_P2_DECISIONS.md) v1.0 · [`registry/deferred.json`](../../registry/deferred.json)。
> **裁决枚举**：`go` / `no-go` / `defer-to-G17+` / `strategic_override`。G16 即本期，`defer-to-G16+` 不再合法。
> **0-byte 边界**：G15/G13/G12 契约与三冻结登记表 0-byte；`registry/deferred.json` 既有条目四字段 0-byte、history 只追加（本波零追加、零新 RD，max=RD-045 维持）；src/spec/conformance 0-byte、零 RFC 消费。

## 1. G15 defer-to-G16+ 十四行与 G15-MC-F1/G15-MD-F1 逐行转引终态裁决

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| M61 | mesh shader 第三光栅路径 | G15.6a defer → G16.1 窗 | 「G16+ 重评窗内多厂商扩展行为收敛 + 性能差 measured 证据齐备且真实消费方出现 → VS 光栅唯一 fallback 维持」 | defer-to-G17+（G16 重评窗结论 = 不交集） | G16 第一波 = 参照臂修复，与 mesh shader 触发条件不交集 | milestones/g15/G15_P2_DECISIONS.md §1 M61 行 | 重判条件 = G17+ 重评窗内多厂商扩展行为收敛 + 性能差 measured 证据齐备且真实消费方出现；兜底 = VS 光栅唯一 fallback 维持 | 本表 §1 行；registry/deferred.json RD-039 | open-defer（G17+） |
| M52 | SER / hit-object 重排 | G15.6a defer → G16.1 窗 | 「G16+ 高分歧 RT workload 真实集成需求 + capability rt.ser 设备面实测可用 → 独立 Full RFC 评估」 | defer-to-G17+（G16 重评窗结论 = 维持未命中） | 本波零高分歧 RT 新集成需求，rt.ser 仍未测 | milestones/g15/G15_P2_DECISIONS.md §1 M52 行 | 重判条件 = G17+ 高分歧 RT workload 真实集成需求 + capability rt.ser 设备面实测可用；兜底 = 语言层不加 SER 原语维持 | 本表 §1 行；registry/deferred.json RD-040 | open-defer（G17+） |
| M100-high | ReSTIR GI/DI 高档 reservoir | G15.6a defer → G16.1 窗 | 「G16+ 低档 MegaLights GPU 管线多灯场景 measured 对照齐备 → 低档默认档维持」 | defer-to-G17+（G16 窗登记 = 未齐备） | 本波不新增多灯压测场景 | milestones/g15/G15_P2_DECISIONS.md §1 M100-high 行 | 重判条件 = G17+ 低档 MegaLights GPU 管线多灯场景 measured 对照齐备；兜底 = 低档 MegaLights 默认档 + M15 open-留档维持 | 本表 §1 行；registry/deferred.json RD-040 | open-defer（G17+） |
| SAFE-GPU | Safe GPU Operator Platform | G15.6a defer → G16.1 窗 | 「G16+ Safe GPU Operator Platform 独立期立项 → G9~G15 零交付维持」 | defer-to-G17+（独立期） | G16 第一波非 Safe GPU 独立期 | milestones/g15/G15_P2_DECISIONS.md §1 SAFE-GPU 行 | 重判条件 = G17+ Safe GPU Operator Platform 独立期立项；兜底 = G9~G16 零交付维持 | 本表 §1 行 | open-defer（G17+ 独立期） |
| M127 | 神经变形研究子轨 | G15.6a defer → G16.1 窗 | 「G16+ 离线工具链 corpus 语料 + PhysicsAsset residual 消费方出现 → 无主线门研究子轨维持」 | defer-to-G17+ | 研究子轨与参照臂修复零依赖 | milestones/g15/G15_P2_DECISIONS.md §1 M127 行 | 重判条件 = G17+ 离线工具链 corpus 语料 + PhysicsAsset residual 消费方出现；兜底 = 无主线门研究子轨维持 | 本表 §1 行 | open-研究子轨（G17+） |
| M98-l4 | M98 L4 Far Field 档 | G15.6a defer → G16.1 窗 | 「G16+ HLOD 运行时接口面就绪 + L4 计数可测 → L1/L2/L3 三级链维持」 | defer-to-G17+ | 本波不扩 L4 | milestones/g15/G15_P2_DECISIONS.md §1 M98-l4 行 | 重判条件 = G17+ HLOD 运行时接口面就绪 + L4 计数可测；兜底 = L1/L2/L3 三级链维持 | 本表 §1 行 | open-defer（G17+） |
| M114-strand | 毛发 strand 档强制精确 OIT | G15.6a defer → G16.1 窗 | 「G16+ M120 精确档 benchmark 裁决数据落地 + 档选定程序解冻 → card/mesh 档维持」 | defer-to-G17+ | 本波产参照臂修复数据面非毛发 OIT 精确档裁决数据面 | milestones/g15/G15_P2_DECISIONS.md §1 M114-strand 行 | 重判条件 = G17+ M120 精确档 benchmark 裁决数据落地 + 档选定程序解冻；兜底 = card/mesh 档 + Marschner 三瓣维持 | 本表 §1 行 | open-defer（G17+） |
| M118-hdr-cal | HDR 设备标定层 | G15.6a defer → G16.1 窗 | 「G16+ HDR 显示设备资产/产品需求出现 → 管线/插件面维持」 | defer-to-G17+ | 本波出图面仍为 offscreen/对拍，零 HDR 显示设备资产 | milestones/g15/G15_P2_DECISIONS.md §1 M118-hdr-cal 行 | 重判条件 = G17+ HDR 显示设备资产/产品需求出现；兜底 = 管线/插件面 SDR 全量验证维持 | 本表 §1 行 | open-defer（G17+） |
| M125-adopt3 | Jolt 5.6 采纳臂三件 | G15.6a defer → G16.1 窗 | 「G16+ 后续 Jolt 升级评估窗采纳臂成立 → 5.3 基线生产默认维持」 | defer-to-G17+ | 本波物理面零交付 | milestones/g15/G15_P2_DECISIONS.md §1 M125-adopt3 行 | 重判条件 = G17+ 后续 Jolt 升级评估窗采纳臂成立；兜底 = 5.3 基线生产默认维持 | 本表 §1 行 | open-defer（G17+） |
| G10-N6 | BistroExterior 未入压测清单 | G15.6a defer → G16.1 窗 | 「G16+ FBX2glTF 上游修复或替代转换臂落地 → BistroInterior + CornellBox 首发清单维持」 | defer-to-G17+ | 本波双场景闭集维持，不扩 Exterior | milestones/g15/G15_P2_DECISIONS.md §1 G10-N6 行 | 重判条件 = G17+ FBX2glTF 上游修复或替代转换臂落地；兜底 = BistroInterior + CornellBox 首发清单维持 | 本表 §1 行 | open-defer（G17+） |
| G10-N8 | renderoffscreen UE 5.8 可用性未测 | G15.6a defer → G16.1 窗 | 「G16+ 无头出图需求出现时实测 renderoffscreen 可用性 → 窗口模式 MRQ 出图臂维持」 | defer-to-G17+ | 本波仍走窗口模式 MRQ，无头需求未出现 | milestones/g15/G15_P2_DECISIONS.md §1 G10-N8 行 | 重判条件 = G17+ 无头出图需求出现时实测 renderoffscreen 可用性；兜底 = 窗口模式 MRQ 出图臂维持 | 本表 §1 行 | open-defer（G17+） |
| G10-N17 | M137 scalars.flip 演进位 null 维持 | G15.6a defer → G16.1 窗 | 「G16+ diff 报告消费 FLIP 标量面真实需求出现时按 RXS-0388 L3 演进位程序翻转实值 → null 演进位维持」 | defer-to-G17+ | 本波 FLIP 消费 = 重测度量面，不消费 M137 scalars.flip | milestones/g15/G15_P2_DECISIONS.md §1 G10-N17 行 | 重判条件 = G17+ diff 报告消费 FLIP 标量面真实需求出现时按 RXS-0388 L3 演进位程序翻转实值；兜底 = null 演进位 + 三面重算一致口径维持 | 本表 §1 行 | open-defer（G17+） |
| G11-N5 | 锁定度量对正确修复结构性不友好 | G15.6a defer → G16.1 窗 | 「G16+ 度量口径修订评估窗（SSIM/FLIP 对低反照率暗帧稳健性对照数据集齐备）→ 现锁定度量口径维持」 | defer-to-G17+ | 本波消费锁定度量口径 0-byte，不新开稳健性评估窗 | milestones/g15/G15_P2_DECISIONS.md §1 G11-N5 行 | 重判条件 = G17+ 度量口径修订评估窗对照数据集齐备；兜底 = 现锁定度量口径维持 | 本表 §1 行 | open-defer（G17+） |
| G13-N7 | 帧生成 FG/MFG | G15.6a defer → G16.1 窗 | 「G16+ 帧生成独立层立项后按只追加程序重判 → FG/MFG 零实现维持」 | defer-to-G17+（不立项） | 本波契约 out_of_scope 字面在案，不立项 FG/MFG | milestones/g15/G15_P2_DECISIONS.md §1 G13-N7 行 | 重判条件 = G17+ 帧生成独立层立项后按只追加程序重判；兜底 = FG/MFG 零实现维持 | 本表 §1 行；registry/deferred.json RD-041 | open-defer（G17+） |
| G15-MC-F1 | ue_reference_arm_black_frames@cornell-box | G15.4 M-c → G16.1 | 「G16+ UE 项目侧 cornell 出图链诊断/修复落地 → M-c 同口径重标定重审」 | go（G16.2 M-a 承载） | 本波唯一 go 承接：修 G13 cornell RectLight 衰减使参照臂不再死黑并重测受影响门 | milestones/g15/G15_P2_DECISIONS.md §2 G15-MC-F1 行；milestones/g16/G16_CONTRACT.md §4.2 M-a 行 | 重判条件 = 五份末帧 HDR luma max 大于 1e-3 且读图非死黑后按 M-b/M-c 重测；兜底 = 仍死黑则维持未达标登记不冒充 | 本表 §1 行 + G16_ACCEPTANCE_MAP M-a 行 | go（G16.2 M-a 承载面） |
| G15-MD-F1 | fps_parity_deficit@bistro-interior/t100/dlss_sr | G15.5 M-d → G16.1 窗 | 「G16+ 立项窗本格双端复测 + UE 参照臂暖态基线程序产重标定 + Rurix DLSS 车道 t100 档优化面重估 → 维持未达标登记不冒充」 | defer-to-G17+（本波不交集） | 本波不拉性能 17/18 单格环境事件面，G15-MD-F1 顺延 | milestones/g15/G15_P2_DECISIONS.md §2 G15-MD-F1 行 | 重判条件 = G17+ 立项窗本格双端复测 + 暖态基线程序产重标定 + t100 档优化面重估；兜底 = 维持未达标登记不冒充 | 本表 §1 行 | open-defer（G17+） |

## 2. open RD 逐条映射（RD-034/039/040/041/042/043/044/045，条目级 status 全维持 open 0-byte）

| RD | title（摘要） | 条目级 status | G16.1 处置 | 联动面 | 裁决理由 | 留痕位置 |
|---|---|---|---|---|---|---|
| RD-034 | DXIL RT/mesh 腿（blocked-on-upstream） | open | 维持 open（blocked 维持） | 无 | 上游钳制二选一解锁证据未出现；本波仅 Vulkan 主腿 | 本表 §2 行；registry/deferred.json RD-034 |
| RD-039 | 虚拟化几何 P3+ | open | 维持 open | M61 见 §1（defer-to-G17+） | 本波参照臂修复面不产生 HZB 瓶颈/tess 资产证据 | 本表 §2 行 + §1 M61 行；registry/deferred.json RD-039 |
| RD-040 | 光照 P3+ | open | 维持 open | M52/M100-high 见 §1（defer-to-G17+） | SER/高档 ReSTIR 触发条件未命中维持 | 本表 §2 行 + §1 M52/M100-high 行；registry/deferred.json RD-040 |
| RD-041 | 材质/流送/时域 P3+ | open | 维持 open | G13-N7 见 §1（defer-to-G17+） | FG/MFG 本波不立项 | 本表 §2 行 + §1 G13-N7 行；registry/deferred.json RD-041 |
| RD-042 | 可微物理 / 机器人批仿研究轨 | open | 维持 open-观察 | 无 | 与参照臂修复无关；四项证据未齐 | 本表 §2 行；registry/deferred.json RD-042 |
| RD-043 | wgrapier GPU 刚体观察 | open | 维持 open-观察 | 无 | 与参照臂修复无关；五条件未同时成立 | 本表 §2 行；registry/deferred.json RD-043 |
| RD-044 | 物理 P3+ | open | 维持 open | 无 | M126 基准 verdict=maintain_no_go 字面维持 | 本表 §2 行；registry/deferred.json RD-044 |
| RD-045 | 间歇性 digest 漂移生产化缺陷修复项 | open | 维持 open | 无 | G15 全期零检出；本波不关闭，长窗归 G17+ | 本表 §2 行；registry/deferred.json RD-045 |

## 3. G16 新增候选 4 行（G16.1 立项裁决行集，零空行）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G16-N1 | UE cornell 参照臂修复与内容有效性 | G16.1 新增（G15-MC-F1 承接） | 「探针否定默认 10 m 衰减时改查 intensity units / barn door / game 可见性 / 源面上限」 | go（G16.2 M-a 承载） | 本波主修面：衰减半径覆盖房间+相机 far，Candela 单位，只重建 cornell | milestones/g16/G16_CONTRACT.md §4.2 M-a 行 | 重判条件 = 探针否定默认 10 m 后改查强度单位与可见性；兜底 = 不改坐标尺度 | 本表 §3 行 + G16_ACCEPTANCE_MAP M-a 行 | go（G16.2 M-a 承载面） |
| G16-N2 | 双端度量重收割写 G16 处置表 | G16.1 新增 | 「fresh measured_delta 方向判定劣化行按只追加程序重判」 | go（G16.3 M-b 承载） | 历史 1.0/0.0 是黑对黑假完美；必须 G16 另立处置表 | milestones/g16/G16_CONTRACT.md §4.2 M-b 行 | 重判条件 = fresh delta 方向判定劣化行按只追加程序重判；兜底 = 不写 G13 两张登记表 | 本表 §3 行 + G16_ACCEPTANCE_MAP M-b 行 | go（G16.3 M-b 承载面） |
| G16-N3 | 18 格绝对画质重审与 cornell 重标定 | G16.1 新增 | 「商用收口判定口径争议时按只追加程序重判判定形态」 | go（G16.4 M-c 承载） | G15 cornell 绝对阈对着死黑参照标定，对有效参照无效 | milestones/g16/G16_CONTRACT.md §4.2 M-c 行 | 重判条件 = 商用收口判定口径争议时按只追加程序重判；兜底 = 达标/未达标如实登记不冒充 | 本表 §3 行 + G16_ACCEPTANCE_MAP M-c 行 | go（G16.4 M-c 承载面） |
| G16-N4 | 已收口门 --verify-latest 零降级 | G16.1 新增 | 「新 g16_ 前缀件抢 latest 致旧门红时按只追加程序重判前缀纪律」 | go（G16.5 M-d 承载） | 84 门绿面不得因新 g16 件被抢 latest；旧脚本禁 --gate | milestones/g16/G16_CONTRACT.md §4.2 M-d 行 | 重判条件 = 新前缀件抢 latest 致旧门红时按只追加程序重判前缀纪律；兜底 = 旧门 --verify-latest 仍绿 | 本表 §3 行 + G16_ACCEPTANCE_MAP M-d 行 | go（G16.5 M-d 承载面） |

## 4. 承接锚清单（defer-to-G17+ 十五行）

| ID | 承接锚（重判条件到兜底） | 目标重评期 |
|---|---|---|
| M61 | G17+ 重评窗内多厂商扩展行为收敛 + 性能差 measured 证据齐备且真实消费方出现 | G17+ |
| M52 | G17+ 高分歧 RT workload 真实集成需求 + capability rt.ser 设备面实测可用 | G17+ |
| M100-high | G17+ 低档 MegaLights GPU 管线多灯场景 measured 对照齐备 | G17+ |
| SAFE-GPU | G17+ Safe GPU Operator Platform 独立期立项 | G17+ |
| M127 | G17+ 离线工具链 corpus 语料 + PhysicsAsset residual 消费方出现 | G17+ |
| M98-l4 | G17+ HLOD 运行时接口面就绪 + L4 计数可测 | G17+ |
| M114-strand | G17+ M120 精确档 benchmark 裁决数据落地 + 档选定程序解冻 | G17+ |
| M118-hdr-cal | G17+ HDR 显示设备资产/产品需求出现 | G17+ |
| M125-adopt3 | G17+ 后续 Jolt 升级评估窗采纳臂成立 | G17+ |
| G10-N6 | G17+ FBX2glTF 上游修复或替代转换臂落地 | G17+ |
| G10-N8 | G17+ 无头出图需求出现时实测 renderoffscreen 可用性 | G17+ |
| G10-N17 | G17+ diff 报告消费 FLIP 标量面真实需求出现 | G17+ |
| G11-N5 | G17+ 度量口径修订评估窗对照数据集齐备 | G17+ |
| G13-N7 | G17+ 帧生成独立层立项 | G17+ |
| G15-MD-F1 | G17+ 本格双端复测 + 暖态基线程序产重标定 + t100 档优化面重估 | G17+ |

## 5. 裁决口径说明与汇总

1. **§1 十六行**：十四行 defer-to-G17+ + G15-MC-F1 go + G15-MD-F1 defer-to-G17+。
2. **§3 四行**：G16-N1~N4 全 go，分别锚定 M-a~M-d。
3. **open RD 八条全维持 open**，本波零追加，零新 RD（max=RD-045）。
4. **行数汇总**：go 5 行（G15-MC-F1 + G16-N1~N4）· defer-to-G17+ 15 行 · no-go 0 行 · 维持 open 8 行（§2）。穷举闭集 20 行零空行。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | 首版：§1 十六行 + §2 八条 RD + §3 四行新增候选。 |
| v1.1 | 2026-08-24 | G16plus 表后事件登记只追加（§1/§3 二十行闭集 0-byte）。 |

## 6. G16plus 表后事件登记（只追加；不入 FROZEN_IDS 20 行机核）

| ID | 分项名 | 来源波次 | 原触发条件字面 | 裁决 | 裁决理由 | 依据/证据路径 | 承接锚 | 登记留痕位置 | 最终状态 |
|---|---|---|---|---|---|---|---|---|---|
| G16-N5 | GI 表达生产加性车道 | G16plus | 「G16+ GI 多级反弹/表面缓存表达面立项 → 间接光残余如实登记」 | go（G16.8 M-e 承载） | 用户强制收口画质；RFC-0031 Approved | rfcs/0031-g16plus-gi-expression-quality-closure.md；G16_CONTRACT §8.3 | 重判条件 = `--gi on` 次级 NEE + ≥2 反弹机核非近零；兜底 = 未进带保持 active 不伪造 18/18 | 本表 §6 行 + G16_ACCEPTANCE_MAP 附录 A M-e | go（G16.8 M-e） |
| G16-N6 | Lumen 差分重收割 | G16plus | 「fresh measured_delta 入 G16 处置表 → 不写 G13 两表」 | go（G16.9 M-f 承载） | 直接光复绿后须重测 GI 差分 | G16_CONTRACT G-G16-9 | 重判条件 = 同口径重算可溯源；兜底 = 不写 G13 两张登记表 | 本表 §6 行 + G16_ACCEPTANCE_MAP 附录 A M-f | go（G16.9 M-f） |
| G16-N7 | 绝对画质 18/18 收口 | G16plus | 「商用收口判定口径争议时按只追加程序重判」 | go（G16.10 M-g 承载） | 新门，不改 M-c 0/18 历史语义 | G16_CONTRACT G-G16-10 | 重判条件 = met_count==18 且阈 p100×2.0 程序产；兜底 = 未达标保持 active | 本表 §6 行 + G16_ACCEPTANCE_MAP 附录 A M-g | go（G16.10 M-g） |
| G16-N8 | soak 与 close-out | G16plus | 「后续收口须另立治理程序」 | go（M-h 承载，前置 M-g） | G16plus 另立治理程序兑现 | G16_CONTRACT G-G16-11 | 重判条件 = M-g 已绿后 soak≥1800s + READY；兜底 = M-g 未绿不做 close-out | 本表 §6 行 + G16_ACCEPTANCE_MAP 附录 A M-h | go（前置 M-g） |
| 2f6331a41404dfcd | lumen_gi_parity@cornell-box | G15 M-b → G16plus | 「G16+ GI 多级反弹/表面缓存立项 → 间接光残余如实登记」 | go（M-e/M-f 承载） | G16plus 唯一画质主修面之一 | milestones/g15/g15_gap_fix_closure_registry.json | 重判条件 = 生产化落地后 fresh delta 进带；兜底 = 残余如实登记不冒充 | 本表 §6 行 + G16_ACCEPTANCE_MAP 附录 A M-e | go（G16plus） |
| b7527c980cdd1d46 | lumen_gi_parity@bistro-interior | G15 M-b → G16plus | 「G16+ GI 多级反弹/表面缓存立项 → 间接光残余如实登记」 | go（M-e/M-f 承载） | 与 cornell 同行 | milestones/g15/g15_gap_fix_closure_registry.json | 重判条件 = 生产化落地后 fresh delta 进带；兜底 = 残余如实登记不冒充 | 本表 §6 行 + G16_ACCEPTANCE_MAP 附录 A M-e | go（G16plus） |
