# G37 W5 渲染器文档面刷新 REPORT（docs/renderer 同步 G37 终态）

> 任务：G33 C2 交付的 docs/renderer 8 篇 + 产品说明，同步 G37 终态（默认翻转/新臂/产品说明/修复登记/判档登记）。
> 纪律执行：纯文档零 GPU 零 cargo；只动 docs/renderer/ 面；追加式修订（修订记录表 + 就地更新事实段，历史结论不篡改，过时字面标注「G37 W4 翻转后以 X 为准」）；根文档/milestones/registry 零触碰。
> 机核验证：`ci/g31_renderer_docs_smoke.py --selftest` PASS（三篇钉死节锚 + 在案数字逐字全在场）+ `ci/g31_support_policy_smoke.py --selftest` PASS（release_checklist 节锚/在飞标注/引用脚本 29 件全在树）——两门 host 判读面全绿，文档刷新未破任何机核字面。
> 日期 2026-08-30。

## 1. 八篇清单与改动量（git numstat）

| # | 篇 | 改动量 | 判定与处置 |
|---|---|---|---|
| 1 | integration_guide.md | **+18 −5**（v1.1） | 受影响·已刷新：§5 默认档翻转段 + 参数面重写（G37 新臂/可组合子参数/诊断臂 off 律）+ 命令示例修订；§7 追加 presented 锚谱系（二进制绑定锚 + W4_ANCHORS 占位）；§8 FG 在案数字标注 base 形态口径 |
| 2 | feature_matrix.md | **+52 −1**（v1.1） | 受影响·已刷新（主承载篇）：新增 §8 四小节（8.1 默认翻转十九臂与锚谱系 / 8.2 新臂七行 / 8.3 修复登记五件 / 8.4 判档登记三件）；§1/§3/§4/§5/§6 各追加 G37 注（历史字面不回写） |
| 3 | performance_tuning.md | **+31 −2**（v1.1） | 受影响·已刷新：§2 真窗口表标注 off 口径 + 追加 full 默认档帧时口径（90fps 预算 11.11ms/day_0829 在案带/十九臂终值 W4_ANCHORS 占位）；§3.4 FG 两点式注；§3.5 HZB off 注；新增 §3.8（full 钉死档 + 微调旋钮表 + PSO 账本）；§4 AE 预期收敛行为；§6 默认档测量纪律 |
| 4 | profiling_debugging.md | **+24 −6**（v1.1，新增修订记录节） | 受影响·已刷新：§1.5 默认档口径条；三处窗口命令示例补显式 `--quality off`（§2.1/§3/§6.1——门 g31.waveC.profiling 三腿同字面）；§2.1 full 形态全量直出注 |
| 5 | release_checklist.md | **+10 −0**（v1.1） | 受影响·已刷新（轻量追加）：§2 G37 注（encode_parity + license 两门进套件 / 诊断门 off 对账与默认臂门新默认复跑 / presented 锚整批重收割纪律）；C5/C6 在飞标注与历史表字面零回写（support 门钉死面） |
| 6 | compatibility_matrix.md | **0-touch** | 判定不受影响：探测面/六降级链/厂商格与 G37 终态无交集（新臂全在 full 预设内无能力链；bench 面不翻转）；且本篇 = JSON 镜像 append-only 面，无事实变更不动 |
| 7 | support_policy.md | **0-touch** | 判定不受影响：缺陷报告要素（bench canonical 命令）/版本政策/安全响应均不因翻转变化；且被 support 门重度钉死（版本五面/镜像四要素/待建立字面）。观察登记：§1.1/§5 的「C7 profiler 面待建立」字面已被 profiling_debugging.md（2026-08-26 交付）事实超越——属 G33 期陈旧非 G37 影响，改判归 support 门 owner（见 §4 遗留） |
| 8 | vendor_license_matrix.md | +20 −0（**非本子任务**） | 本任务 0-touch：+20 行为同战役 W5 许可子任务（GAP closure §6）已落的工作区改动，如实分列不冒领 |
| 附 | examples/minimal_host/README.md | **0-touch** | 判定不受影响：C ABI DLL 宿主面（rurixc/cl 命令），不引用窗口 bin 与 `--quality` 默认行为 |

## 2. 新增篇

| 篇 | 行数 | 内容 |
|---|---|---|
| **docs/renderer/PRODUCT_NOTES.md**（v1.0，新增） | 约 90 行 | DEFAULT_FLIP_PLAN §4 产品说明条目的单一落点：①交付默认档 = full 十九臂（off 回退档律 + bench 分离）②AE 预期行为（resize ~12 帧半衰 / α=0.02 收敛 ~50 帧 / 场景切换 ~1s 完全收敛 = 协议内，含判缺陷界线）③帧时口径（90fps 预算 11.11ms + day_0829 9.54~10.70ms 在案 + 十九臂终值 W4_ANCHORS 占位）④FG 帧率口径承诺（生成帧不入真实渲染帧率）。引用不复制，工程细节指姊妹篇 |

选址理由：既有 8 篇均为工程面（集成/矩阵/调优皆自述「勿用绝对值做 SLA」），产品面承诺（预期行为/口径承诺/交付形态）无既有落点——按任务书「既有结构无处安放则新增」判定成立；AE/帧时的工程细节同时就地落进 performance_tuning §4/§2（各自本职事实段），两面互引不复制。

## 3. 命令示例一致性核对（逐条）

全仓 docs/renderer 内含窗口 bin（`g31_window_present`）的命令示例逐条与翻转后语义核对：

| 类 | 条数 | 明细 |
|---|---|---|
| **补 `--quality off`（诊断/在案数字口径示例，语义已随翻转变化）** | **4** | profiling §2.1 profile 快速上手例 / profiling §3 在案分解样例命令 / profiling §6.1 RenderDoc 捕获例 / integration §5 FG x2 例（在案 85.24/145.30 数字系 base 形态，改为显式 off） |
| 新增示例（翻转后新语义形态） | 2 | integration §5：显式回退档例（all-off 基线/诊断口径）+ fg×full 两点式第二点例 |
| 语义标注（命令不改，注明翻转后 = full 交付默认） | 2 | integration §5 交互例 + 非交互 CI 例（此二例本就应展示交付默认，flag 零改动，加注语义） |
| 核对后零改动（bench 车道/非窗口 bin） | 全部 | integration §6 bench 例、performance §6.1 bench 复现例、profiling §2.2 bench 例、support_policy §1.1 digest 要素命令（bench `--quality` 默认维持 off 不翻转）；minimal_host README（rurixc/cl）；release_checklist（py 门命令） |

窗口单臂/互斥臂写法（`--textures on`/`--hzb on`/`--slab-table` 等）在 feature_matrix §4/§6 与 performance_tuning §2/§3 以表格字面出现（非可执行示例）——以节级 G37 注统一声明「须显式 `--quality off`」，历史表字面不逐格改写（追加式纪律）。

## 4. 遗留（占位与归属）

**依赖 W4 终值的占位清单**（占位字面统一 =「见 W4_ANCHORS」，指 `artifacts/day_0830_delivery/w4_flip/W4_ANCHORS.json`；W4 整批重收割落值后按各篇修订程序追加，历史行不回写）：

| # | 占位项 | 落点 |
|---|---|---|
| 1 | 十九臂默认 full 帧时终值（frame_ms 带 + 预算达标复验） | performance_tuning §2 G37 块；PRODUCT_NOTES §3 |
| 2 | 十九臂默认 full presented 新锚（96f digest） | feature_matrix §8.1；integration_guide §7 锚谱系 |
| 3 | RD-045 P02 锚替换值（060e69a8 → W4 重收割值；ci/ L63 字面改写须专项授权） | feature_matrix §8.1（登记面） |
| 4 | fg × full 组合点在案双口径数字（x2/x3） | integration_guide §8 注；PRODUCT_NOTES §4 |

**其他遗留（非 W4 依赖，登记不动手）**：

- support_policy.md §1.1/§5「C7 profiler 面待建立」字面已陈旧（profiling_debugging.md 已交付、门 g31.waveC.profiling 在案）——本篇被 support 门钉死待建立字面（PENDING_POLICY_TOKENS 含 C7），改判须与门脚本判据同批修订，归 support 门 owner/主线。
- vendor_license_matrix.md 的 rowan 条目 conditional→cleared 正式改判 = 下一次矩阵版本化修订（W5 许可子任务残余登记，非本任务面）。
- 00_MASTER_INDEX 等根文档、milestones/、registry/ 按纪律零触碰；PRODUCT_NOTES.md 未挂任何机核门（如需防腐化钉死，归 docs 门 owner 后续追加 DOC_SPECS 条目）。
