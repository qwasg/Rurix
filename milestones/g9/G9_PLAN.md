# G9_PLAN — UE5 级渲染器与物理引擎正式建造期 主线分解

> **状态**：**计划定稿（v1.1）——G9 未立项**。本文以 [G9_DOCSET_PLAN_DRAFT.md](G9_DOCSET_PLAN_DRAFT.md) 为蓝本起草，v1.0「评审修订中」经一轮独立对抗性评审（评审 provenance ≠ 起草 provenance，4 findings 全部处置）后升「计划定稿」；本文与 G9.0 文档集全部产物均不构成任何契约、验收承诺或编号 claim。G9 正式立项（用户指令 + D-406 裁决 + §5 治理门）前，零 `src/`/`spec/`/`conformance/` 语义实现、零编号消费。
> **蓝本与上游**：[G9_DOCSET_PLAN_DRAFT.md](G9_DOCSET_PLAN_DRAFT.md)（事实基线与波次假设）· [G9_CAPABILITY_MATRIX.md](G9_CAPABILITY_MATRIX.md)（能力缺口矩阵，M90~M127 顺延 G8）· [research/R4~R8](research/)（五路调研正式化）· [design/](design/) 五份模块设计草案（G9.0 冻结引用，内容以草案为准不回写）· [G8_PLAN](../g8/G8_PLAN.md)（范式模板）· [G8_P2_DECISIONS](../g8/G8_P2_DECISIONS.md)（十条 defer 承接锚，法定输入）· [G8_CONTRACT](../g8/G8_CONTRACT.md) §8.26（G8 closed 终态）。
> **推进形态**：**严格波次**——G9.0 文档集 → G9.1 治理/RFC/候选决策表/验收映射 → G9.2~G9.6 五轨道实施 → G9.7 P2 穷举 → G9.8a soak → G9.8b close-out。波次内可蜂群并行，波次间串行；spec-first + RED 先行；禁止 stub/mock/host substitution 抢跑。
> **本波边界（G9.0，governance-only）**：本波仅落 PLAN / 矩阵 / 调研 / 草案冻结引用四件套，作为独立纯文档提交——零编号、零 registry、零 `spec/src/conformance` 改动、零 CI 步骤 materialize（验收门只写判据草案）。

---

## 0. 定位与成功判据

**G9 = UE5 级渲染器与物理引擎的正式建造期**（[G8_PLAN](../g8/G8_PLAN.md):15 字面「正式建造归 G9+」；G8_CONTRACT:128「在 G9+ 正式建造前补齐…前置」）。G8 是前置能力完成期，已 closed；G9 在其冻结底座上建造五模块：D1 虚拟化几何×RT 合流、D2 全局光照、D3 GPU-driven 提交与着色系统、D4 大世界×专项渲染器×显示管线、D5 物理。

「UE5 级」可核对基线沿用 G8 口径 = **UE 5.8**（R1 §2；Epic 最后计划内 UE5 大版本）。验收五层级沿用 G8：**核心等价、功能闭环、可降级、可生产化、Vulkan 主线**。

**成功判据（草案）**：矩阵 §6.4 十条判据草案在 G9.1 硬化为契约验收门；全部 P0 行（建议值 15 行，矩阵 §6.1）各映射至少一门 CI 硬门 + evidence schema（G9.1 落 ACCEPTANCE_MAP，本波不 materialize CI 步骤）；P1 按波次消化；P2 与留档项经 G9.7 穷举决策表逐条裁决，未触发维持 open、禁止假绿 close。

---

## 1. 择优裁决与候选处置

### 1.0 法定输入：G8.7 十条 defer 承接锚（机核「defer 必有 G9+ 承接锚」）

| 承接锚 | 分项 | 归属 | G9 矩阵行 |
|---|---|---|---|
| G9+ 虚拟几何评估窗 | M06 骨骼/植被虚拟几何 | D1 | M90/M92/M93 |
| G9+ RT×Nanite 合流窗 | M09 Mega Geometry 簇级 BLAS | D1 | M90/M94 |
| G9+ GI 建造期 | M12 Surface Cache | D2 | M97 |
| G9+ GI 档位 | M16 irradiance field 档位 | D2 | M101 |
| G9+ shader library 深化 | M33 shader library 组合链接 | D3 | M106/M107 |
| G9+ 大世界分区 | M43 World Partition/HLOD | D4 | M110/M111 |
| G9+ 大气特效 | M48 体积雾/云 | D4 | M112 |
| G9+ 专项渲染器 | M49 水体/毛发/皮肤/地形/贴花族 | D4 | M113~M117 |
| G9+ GPU-driven 提交 | M55 descriptor buffer/DGC | D3 | M102~M105 |
| G9+ gameplay Field | M74 Physics Field | D5 | M121/M122 |

**触发条件声明（立项时留痕）**：五份设计草案假定 G9 正式立项书即构成 RD-039 M06/M09 两分项（「动态资产面出现时」「RT 与虚拟几何合流需求出现时」）的触发证据；立项时须在 `registry/deferred.json` history **只追加**登记相应分项由 open-defer 转入 G9 承接，不得改写 G8.7 决策表原文、不得静默改判。

### 1.1 追加输入处置建议（均非定案；「立项待裁决」项见 §5 待裁决表）

| 分项 | 现状字面 | 建议默认（计划建议，非裁决） |
|---|---|---|
| M17 Path Tracer 参照器 | backfill「GI/材质画质门需要跨路径 golden 时（G9+ 建造期前置）」 | **字面已命中，建议判 go**；G9.4 波内第一顺位（M96） |
| M45 HDR / M46 后处理栈 | no-go，open-留 G9+ | M118 拆「管线/插件面（SDR 可验证）+ 设备标定（条件触发，未触发 SKIP 不充绿）」；M119 以立项书为产品需求证据候选 |
| M47 OIT | no-go「需 measured 对照」 | M120 benchmark 门先行，默认档由数据裁决不由论文偏好裁决 |
| M52 SER | no-go 留档 | **立项待裁决**：D3 建议改判「语言层原语 + capability 可选」；改判须 deferred history 只追加 override |
| M61 mesh shader | no-go 留档 | **立项待裁决**：D3 建议改判「可选 geometry pipeline」；程序同上 |
| M56 Work Graphs / M59 async compute / M62 task shader | no-go 留档 | 维持；预留字段带 `reserved_` 前缀不接线；RXS-0239/RXS-0270 字面不动 |
| M75 异步物理 tick | no-go「须独立判档」 | 条件制：Jolt 单线程成本 measured 为判档硬前置，不足维持 no-go（M123） |
| M77 浮力 | no-go 留档 | D5 建议 go：走 Field 通道（M124）；禁旁路 API |
| M65b Rapier 深造 | no-go 留档 | 条件制维持：对标基准先行（M126），RD-044 字面不变 |
| Safe GPU Operator Platform | G8「改挂 G9+」留痕 | **立项待裁决**：进 G9 独立轨道 or G10+；与 UE5 前置无依赖 |
| 神经变形 | rfcs/0021:122 G9+ 研究轨留痕 | D5 研究子轨承接（M127），无主线门；登记形式**立项待裁决** |

### 1.2 条件型 RD：禁止「以 UE5 目标直接主线化」（G8_PLAN §1.2 纪律继承）

RD-039/040/041/044 总体维持 open，为 G9 法定输入。其分项在 G9 建造期真实资产/画质需求出现时逐条判档：① **go（有证据）**——附 measured workload / 资产需求证据路径 → 进对应波次硬门；② **strategic_override**——书面登记 + deferred history 只追加 override 行 → 进硬门；③ 未 override 且无证据 = **no-go**，维持 open 进 G9.7 穷举表留痕，不得假绿 close。**任何分项的触发条件不得被「UE5 目标」静默改写**；G9 立项书作为触发证据的分项仅限 §1.0 已声明者，且须 history 只追加留痕。

### 1.3 G9.1 必交付：候选分项决策表（完整映射）

G9.1 落盘 `G9_CANDIDATE_DECISIONS.md`，每一分项一行：

`锚/RD-id / 分项名 / G9 M## / 原 backfill 字面 / 证明 workload / 证据路径 / go|no-go|strategic_override|defer / 承接波次 / 承接锚 / 最终期望状态`

并维护 **G8 锚 → G9 M## → 波次 → 退出门 → 最终状态** 总表（覆盖矩阵 §6.3 全 24 条映射 + §1.1 全部追加输入 + 存续 open RD 分项）。**缺行不得开工 G9.2**。defer 出 G9 的任何分项必须带承接锚（沿袭 G8.7 纪律：`ci/g8_p2_decisions_check.py` 同构机核进 G9 validator）。

### 1.4 out-of-scope

| 项 | 依据 |
|---|---|
| DMM / displacement micromap | 永久禁止（D1 D-7；NVIDIA 已归档被 Mega Geometry 取代） |
| 编辑器 GUI / 完整网络引擎 / 音频 / 多 GPU / WebGPU / USD / MaterialX | G8_PLAN §1.5 口径继承 |
| Work Graphs 实现 / async compute 第二腿 / task shader 开放 | G8.7 no-go 维持（§1.1） |
| GPU 主刚体（含「预算隔离的可选副求解器」）/ 可微物理 / 真双向流体耦合 / 通用软体 MPM/FLIP | G6 禁止线 + RFC-0021 §2.4 + D5 §2.2；Jolt 5.6 GPU compute 只评估不接权威 |
| NRC / 神经 radiance cache / FG-MFG / cooperative vector | SG-002 族禁止面 + RD-041 分项字面（矩阵 §7） |
| 改写 G5/G6/G7/G8 closed 契约与 00–14 | 只追加 |
| 无 measured baseline 的性能硬门 | P-09；`g9_budget.json` 非空前置 |
| 任何编号（RXS/RD/U/RX/CI step/RFC）推测性消费 | 立项时按实测 `next_free` 领取（§4 R-G9-7） |

---

## 2. 五轨道与波次分解

```text
G9.0 文档集不可变基线（本波，纯文档提交）
  → G9.1 治理包（契约/RFC/候选决策表/验收映射/measured baseline/validator）
  → G9.2 地基波：D1 cluster 数据格式/页格式 v2 冻结 + D3 descriptor 全局表/DGC 抽象 + D5 Field 骨架与统一 particle view
  → G9.3 几何×RT 合流波：D1 GPU 蒙皮/LOD/CLAS + D3 command build node/Execution Set
  → G9.4 GI 波：D2 M17 参照器先行（golden 前置）→ Surface Cache → SPG/Radiance Cache → IF 档位 → 多灯
  → G9.5 大世界×专项波：D4 分区骨架/OIT benchmark → 大气/地形/贴花 → 云/水体/皮肤/HDR → AVBOIT/毛发
  → G9.6 物理波：D5 Field 完整语义/浮力/双通道 tick/Jolt 5.6 A/B/Rapier 基准
  → G9.7 P2 穷举决策表（软门必穷尽，沿袭 G8.7：defer 必有承接锚）
  → G9.8a stabilization/soak（≥G7 量级：≥30min/≥10000 帧）→ G9.8b close-out
```

**分包形态声明**：上表为「五模块全进」形态。**G9 规模与分包属立项待裁决（§5 表项 4）**——若裁决 G9 只取 D1+D3+D5 地基，则 G9.4/G9.5 两波整体平移 G10 并各自带承接锚（「G10 GI 建造期」「G10 大世界×专项」），本计划波次结构不因此改写，只追加分包修订行。

### G9.0 — 文档集不可变基线（本波）

交付四件套，独立纯文档提交（零编号、零 registry、零 G8 在途改动）：

1. **本计划**（经 ≥1 轮评审修订循环升「计划定稿」）。
2. **`G9_CAPABILITY_MATRIX.md`**：M## 顺延 G8（M90~M127，38 行），每行带 G8 承接锚 / backfill 触发字面 / 建议 P 级 / 验收五层级；五条验收层级沿用 G8 口径。
3. **`research/R4~R8`**：五路调研正式化（虚拟几何×RT / GI / GPU-driven / 大世界×专项 / 物理），每条引用带 URL + 访问日期 2026-08-08。
4. **五份 `design/` 草案冻结引用**：头部追加「G9.0 冻结引用」行，正文 0-byte，后续只追加修订记录。

### G9.1 — 治理包（governance-only，与 G8.1 同构）

交付：

1. **用户立项指令留痕 + agent D-406 裁决**；§5 待裁决表六项全部落定；G9.0 不可变 ref 登记；工作树未提交项处置完毕（§5 表项 1）。
2. **`G9_CONTRACT.md`**：契约四要素 + front matter 状态机（`implementation_status: blocked` 起始）+ §8 只追加条款结构。
3. **`G9_CANDIDATE_DECISIONS.md`**（§1.3）——含 M52/M61 改判提案的 override 登记（若裁决接受；每条改判 deferred.json history 只追加，禁静默改判）+ Safe GPU Operator Platform 归属 + 神经变形登记形式。
4. **`G9_ACCEPTANCE_MAP.md`**：全部 P0（及 go 的 P1）的 `M## → CI step → evidence schema → 判据`；缺行阻断 G9.2。
5. **伞形 Full RFC（D-409 对抗性评审）**，建议三份（实际编号按立项时 `registry/number_ledger.json` 实测 `next_free` 领取，禁止推测号）：
   - 虚拟几何与 GI 语义（cluster DAG/页格式 v2/CLAS/Surface Cache/probe 编码/材质时域；触 M28 多层 closure 条件扩展时显式修订行）；
   - GPU-driven 提交与着色系统（DGC 语义/Execution Set/descriptor 全局索引/SER 原语/mesh shader 可选路径；自动 barrier 新依赖边 = G5 Barrier EB 冻结面修订行）；
   - 物理平台修订（RFC-0021 修订：Field 系统/双通道 tick/浮力/Jolt 5.6 升级路径/神经变形研究轨边界）。
6. **RTX 4070 Ti measured baseline → `g9_budget.json` 非空**（P-09：无 measured baseline 不得设性能硬门；D1 验收含 VRAM/AS 构建耗时指标；D4 阈值全部实测标定禁手写）。
7. **`CI_GATES`** + G9 专属 validator（acceptance map 三向比对、implementation interlock、budget baseline 检查、决策表承接锚机核——脚本模式照 G8 五件套，编号 CI 步骤 G9.2 互锁后实测领取）。

### G9.2 — 地基波

| 面 | 内容 | 矩阵 |
|---|---|---|
| D1 离线 | cluster DAG 误差度量/簇对锁定/蒙皮元数据/CLAS 烘焙输入 + **页格式 v2 新 major ABI 冻结**（spec-first，M04 v1 0-byte） | M90/M91 |
| D3 提交底座 | descriptor buffer 全局表 + DGC 抽象层 + AccessKind 新边（RFC 先行） | M102/M103/M104 |
| D5 骨架 | 统一 particle view + Field 骨架（定义层/schema/三生命周期）；M68 journal 迁移首个 consumer | M121/M122 |

退出门（判据草案，防假绿）：页格式 v2 编解码 golden + 篡改 digest 页被拒；DGC/DgcBuffer 类型层无 host 读接口断言 + 装配期限制核验 RED 臂；M68 journal 迁移前后 digest 一致。

### G9.3 — 几何×RT 合流波

| 面 | 内容 | 矩阵 |
|---|---|---|
| D1 动态几何 | GPU 蒙皮/保守包围体/距离分级更新率 → 误差驱动 LOD 选择（`VisibleClusterSet`）→ CLAS 当帧拼装（NV 主腿 + 回退腿）→ VisBuffer/VSM 单源集成 | M92/M93/M94/M95 |
| D3 链路 | command build node 全链路零 CPU 回读 + Execution Set/PSO 衔接 + shader library IR 链接 + 变体预算工具 | M105/M106/M107 |

退出门（判据草案）：CLAS 腿与回退腿 ray query 逐命中一致；静态帧零 AS 构建（非零即 RED）；蒙皮簇 VisBuffer SW/HW diff=0 维持；IR 链接 interface hash 确定性；变体工程级总预算门硬失败有效。M108/M109 仅在 §5 裁决 go 后落本波或后续，未裁决前不得出现其实现痕迹。

### G9.4 — GI 波

**门序（硬）**：M96 M17 golden 门未绿 → M97~M101 任何画质门不得验收（D2-Q7）。

| 顺位 | 内容 | 矩阵 |
|---|---|---|
| 1 | M17 Path Tracer 参照器（megakernel + 确定性协议 + pbrt-v4 对照） | M96 |
| 2 | Surface Cache 离线 Card + 运行时缓存（L3 追踪档先行） | M97/M98 |
| 3 | 追踪降级链 L1/L2/L4 + hit lighting 档 | M98 |
| 4 | SPG 自适应细分 + Radiance Cache 双级 | M99 |
| 5 | irradiance field 档位 L1→L2→L3 | M101 |
| 6 | 多灯直接光（低档默认 → 高档 ReSTIR 可选；RD-040 触发举证前置） | M100 |

退出门（判据草案）：各档按匹配深度对 M17 golden；漏光负例 RED 臂独立有效；验证射线零跳过统计性偏置门；每档 AS 更新预算行消费 AsStats；L4 Far Field 依赖 HLOD 接口未就绪时登记 SKIP=not-triggered 不充绿。

### G9.5 — 大世界×专项波

| 顺位 | 内容 | 矩阵 |
|---|---|---|
| 1 | 分区数据模型 + 流送预算契约 + HLOD 烘焙 + **OIT benchmark harness（仅测量不定档）** + 后处理骨架 + view transform 插件面（SDR 验证） | M110/M111/M120/M119/M118 |
| 2 | Froxel + 雾前端；地形（chunk ≡ cell 首发）；贴花 DBuffer 占位；DOF/分级完整化 | M112/M116/M117/M119 |
| 3 | 云前端；大洋水体；皮肤；HDR 设备标定（条件触发，否则 open-留痕不假绿） | M112/M113/M115/M118 |
| 4 | OIT 有界近似档 → 精确 linked-list 档；毛发；浅水水体 | M120/M114/M113 |

退出门（判据草案）：大世界 soak hitch p99 ≤ measured 阈值 + 预算违约注入必降级不静默超帧；HLOD 双构建 hash 相等 + 运行时零合并断言；AgX/ACES golden 对含已知差异记录；OIT 默认档选型必须引 benchmark 数据（无数据提交判 RED）；地形/贴花零 SVT 依赖断言。

### G9.6 — 物理波

| 面 | 内容 | 矩阵 |
|---|---|---|
| Field 完整 | 语义层八枚举/过滤/World-Field 通道 + persistent 全 journal replay hash | M121/M122 |
| 浮力 | 解析模型走 Field 通道 + corpus fixture（细长/翻滚回归） | M124 |
| 双通道 | `deterministic_profile` 断言 + async-decorative 通道（判档 go 才启用；P-6 测量硬前置） | M123 |
| Jolt A/B | 5.3→5.6 七步程序（新摩擦模型重点；GPU compute 只评估不接权威；layout 探针工具化） | M125 |
| Rapier 基准 | 对标 A/B 报告 → RD-044 判档申请或维持 | M126 |

退出门（判据草案）：浮力旁路 API 注入即 RED（必须走 Field 通道）+ capture→replay 逐 tick hash 与变帧率逐位一致对拍；双通道未经 Jolt 单线程成本 measured 判档则登记 no-go 不充绿；persistent field 注册/注销/变更全 journal 化且 replay hash 一致（§2.9 M121/M122 行）；Jolt A/B 两臂（采纳三件事/失败钉 5.3）诚实登记，禁写 5.6 PASS 伪绿。

神经变形研究子轨（M127）全程伴随，无硬门、产出不计主线绿。

### G9.7 — P2 穷举决策表（软门，必须穷尽）

对 G9 全部 P2/留档/未触发分项逐条填 go/no-go/defer-to-G10+（不得遗漏；**defer 必有承接锚**，机核同构 `ci/g8_p2_decisions_check.py`）。候选行集（G9.1 决策表校准后冻结）：M108/M109（若维持 no-go）、M114、M123/M126（若判档不成立）、M99 世界 clipmap 级（若未举证）、M100 高档、M127、Safe GPU Operator Platform（若裁决 G10+）、M118 设备标定层（若未触发）、G9 期内新产生的 defer 分项。软门失败/未触发 → 诚实登记不阻塞 G9.8a；close-out 审计要求本表无空行。

### G9.8a — Stabilization / soak（close 前必经）

- 全 P0 硬门回归 + 已 go 的 P1 回归；G5~G8 既有判据 0-byte。
- 代表性场景 soak：**阈值 ≥ G7 量级（≥30min/≥10000 帧）**，除非 measured 证明更短足够（G8.8a 口径继承）；含动态几何/GI/大世界流送/物理同场的 One True Device Frame 连续 provenance。
- `budget_eval --strict` 非空、零 estimated/skip。
- 条件实现刚绿**不得**当日进 8b；**G8.8b 同日放行先例是否继承属立项待裁决（§5 表项 6）**。

### G9.8b — Close-out

- 验收映射终审；RD 分项最终状态与 G9.1/G9.7 决策表逐字一致。
- RD-039/040/041/044 分项逐条判档留痕（触发条件未被「UE5 目标」静默改写——§1.2）。
- 契约 §8 只追加 + status flip。

### 2.9 P0 → 硬门覆盖（判据草案；G9.1 固化为 ACCEPTANCE_MAP 时方可 materialize CI 步骤）

| P0 | 最晚波次 | 硬门要点（判据草案摘要） |
|---|---|---|
| M90 | G9.2 | DAG 误差 monotonic 逐边成立 + 双构建字节一致 + 破坏单调性 fixture 被拒 |
| M91 | G9.2 | 页格式 v2 编解码往返无损 + M04 v1 0-byte 兼容 + 篡改 digest 页被拒 |
| M102 | G9.2 | DGC token 限制装配期 fail-closed + layout 违规声明被拒 |
| M103 | G9.2 | 全局 descriptor 索引与 shader 实际索引双向精确相等 + ≥65536 条目出图正确 |
| M104 | G9.2 | 新 AccessKind 边 barrier 推导 golden + 漏声明 indirect 读边装配期 strict 拒 |
| M121 | G9.2/9.6 | 五域 adapter 全实现 + M68 journal 迁移无损（digest 一致 + golden） |
| M122 | G9.2/9.6 | 过滤默认空匹配零影响断言 + persistent 全 journal replay hash 一致 |
| M93 | G9.3 | selection cut 无重叠无空洞 + 未驻留页父簇兜底 + 空洞注入 RED |
| M94 | G9.3 | CLAS 腿与回退腿逐命中一致 + 可见集/BLAS 错开一簇即 RED |
| M95 | G9.3 | 蒙皮簇 VisBuffer SW/HW diff=0 + 旁路单源真相 variant provenance RED |
| M96 | G9.4 | 固定 seed 位级一致 + pbrt-v4 收敛曲线容差带 + 改 seed/跳 RR/关 MIS 三臂 RED |
| M97 | G9.4 | Card 空洞漏光检测臂 RED 有效 + 只丢能量不漏光断言 |
| M98 | G9.4 | 四级命中率/耗时计数非空 + 逐级强关回归可检测 + 禁静默回退 |
| M110 | G9.5 | 预算违约注入必排队降级 + hitch p99 soak + cell 事件序列逐字 golden |
| M118 | G9.5 | 四插件逐一 golden + 非 HDR 交换链携带 PQ 输出即 RED；设备标定未触发 SKIP 不充绿 |

任一 P0 无独立硬门 → **禁止** G9.8b status flip。

---

## 3. 与既有面的边界（0-byte 纪律）

| 面 | 约束 |
|---|---|
| G8 车道 | G8 四件套/决策表/evidence schema/budget 0-byte；G8 closed 判据不回写；G8.7 决策表原文不改写（承接走 deferred history 只追加） |
| G5/G6 冻结面 | `GpuScene`/`MaterialClosure` 32B/`Barrier` EB 三轴/`PageRequest`/物理五纪律经 RFC 修订方可动；AccessKind 新边（M104）、closure 扩展（M115/M114）、World-Field buffer（M122）触冻结面时**必须显式修订行** |
| 00–14 | 只勘误，独立提交 |
| spec/conformance | G9.0 与 G9.1 期均 0-byte；spec-first + RED 先行自 G9.2 实现门开放后起，spec 条款 PR 先于实现 PR |
| 注册表 | G9.0 期 0-byte；登记/翻转/history 追加归立项后；deferred history 只追加禁静默改判 |
| 编号 | G9.0 期零消费；D3 草案 §⑨ 建议区间（RXS-0322 起）与 M50 实际消费段（RXS-0322~0327）冲突——**一切编号以立项时实测 `next_free` 为准，禁止沿用草案建议值** |
| 设计草案 | 五份 `design/` 草案头部追加冻结引用行后正文 0-byte；内容以草案为准不回写 |
| unsafe | U 段按立项 next_free；`rurix-render` forbid(unsafe) 维持 |
| 调研引用 | R4~R8 为 2026-08-08 快照；G9.1 立项时复核关键引用时效（R-G9-3） |

---

## 4. 风险与止损

| ID | 风险 | 预警 | 止损 |
|---|---|---|---|
| R-G9-1 | 五模块范围爆炸，G9 变成不可收口巨兽 | 波次退出证据不齐想并行抢跑 | §5 表项 4 分包裁决；P0 最小集先行，P1/P2 按波次消化与穷举 |
| R-G9-2 | NV-only 扩展（CLAS/DGC 部分）绑死单厂商 | Khronos EXT 标准化长期无草案 | 每 NV 扩展主腿配跨厂商回退腿作正确性基线；capability profile 门控；回退腿升主交付 |
| R-G9-3 | 调研引用失效/版本漂移（UE 5.5-5.8 快速演进） | R4~R8 关键结论与新公开材料冲突 | G9.1 立项时复核 R4~R8 关键引用时效；SIGGRAPH/Unreal Fest 2026 新材料复审窗 |
| R-G9-4 | 防假绿纪律在更大范围下执行成本上升 | 人工评审替代机器判读 | 验收门全部机器判读 + 负例 RED 臂 + evidence schema，沿袭 G8 机核模式 |
| R-G9-5 | M17 参照器工作量被低估（golden 是 GI 各档前置） | G9.4 波内 M96 超预算 50% | M96 波内第一顺位；megakernel 起步控制范围；焦散/体积/specular 链明确 out |
| R-G9-6 | 条件型 RD 静默主线化 / defer 无承接锚 | 决策表空行或无证据 | §1.2/1.3 缺行阻断 G9.2；承接锚机核进 G9 validator |
| R-G9-7 | 草案编号建议值漂移/冲突（D3 §⑨ 区间撞 M50 已消费段） | 实现 PR 沿用草案 RXS 建议号 | 立项时一律实测 `next_free` 领取；ACCEPTANCE_MAP 三向比对锁定 |
| R-G9-8 | 双世界错配（RT 与光栅各算可见性）悄然引入 | 帧末一致性校验缺位 | 单源真相负例 RED 臂为硬门；provenance 校验进 CI |
| R-G9-9 | 条件实现刚绿即 close / 同日放行先例滥用 | 跳过 G9.8a soak | 8a 为 8b 前置硬门；先例继承与否 §5 表项 6 显式裁决 |
| R-G9-10 | HDR 设备/真实骨骼植被资产/多灯 workload 不可得 | 用 SDR 截图/程序资产/无证据叙述充绿 | SKIP=not-triggered 登记不充绿；G9.1 冻结资产制作/采购计划；证据不足则分项维持 open-留档 |

---

## 5. 立项条件与四件套落位清单

**G9.0 文档集基线（本波，已按本计划 §2 G9.0 执行）**：

1. 本计划经 ≥1 轮评审修订循环升「计划定稿」；
2. `G9_CAPABILITY_MATRIX.md`（M90~M127）落盘；
3. `research/R4~R8` 五份落盘（引用带 URL + 访问日期 2026-08-08）；
4. 五份 `design/` 草案头部「G9.0 冻结引用」行追加（正文 0-byte）；
5. 全部作为独立纯文档提交：零编号、零 registry、零 `spec/src/conformance` 改动。

**立项待裁决表（本计划不定案，裁决权归用户/立项治理）**：

| # | 事项 | 候选口径 | 不定案原因 |
|---|---|---|---|
| 1 | G9 立项时机与工作树处置 | 先处置（8b closeout evidence 落库提交 + `.phys_baseline`/`.wip_evidence_hold` 归档 + `layout_hinge` 探针入库或纳入 G9 物理波次）再立项，或带未提交项立项 | 处置方式影响 G9.0 不可变 ref 的基线内容 |
| 2 | Safe GPU Operator Platform 归属 | 进 G9（独立轨道，与渲染/物理无依赖）or G10+ | 用户未裁决；G8 留痕仅「改挂 G9+」 |
| 3 | M52 SER / M61 mesh shader 改判 | 接受 D3 建议（语言层支持 / 可选路径）or 维持 G8.7 no-go 留档 | 改判须 deferred history 只追加 override，属治理裁决非计划可定 |
| 4 | G9 规模与分包 | 五模块全进 or G9 = D1+D3+D5 地基、GI/大世界归 G10 | 五模块全进为项目史上最大里程碑，工期与 soak 成本需 measured 评估 |
| 5 | 神经变形研究轨登记形式 | 独立 RD 登记 or 维持 rfcs/0021:122 无归属留痕 | 注册表动作归立项后，本波 registry 0-byte |
| 6 | G8.8b 同日放行先例继承 | 继承：8a full-run 先行完成后允许同日进 8b close-out（G8.8b 先例字面，G8_CONTRACT §8.26 放行条款）or 不继承：恢复 G8_PLAN「条件实现刚绿不得当日进 8b」 | 涉及 soak 严肃性口径，须立项时显式裁决；注：蓝本 §G9.2+ 段末「不得当日进 8a」与 G8_PLAN 字面不同，本计划采 G8_PLAN 字面 |

**G9.1 治理门（立项波）**：

1. 用户立项指令留痕 + agent D-406 裁决；待裁决表六项全部落定并留痕。
2. G9.0 文档集有不可变 ref；工作树处置完毕。
3. 落 CONTRACT / CI_GATES / 非空 measured `g9_budget.json` + 本 PLAN 升格为契约上游事实源。
4. 三份伞形 RFC 经 D-409 对抗性评审后 Approved；编号按实测 `next_free` 领取。
5. `G9_CANDIDATE_DECISIONS.md` + `G9_ACCEPTANCE_MAP.md` 齐备；条件型 RD history 只追加；决策表缺行阻断 G9.2。
6. G9 validator 五件套落盘（acceptance map 三向比对 / implementation interlock / budget baseline / 决策表承接锚机核 / 编号冲突检查）。
7. README 状态镜像；00_MASTER_INDEX errata 独立提交。

**G9.2 实现门（后续波）**：

1. G9.1 治理门全绿 + interlock validator 输出 READY。
2. 重新校准共享命名空间 actual `next_free`，materialize 数字 CI 步骤；不得沿用 G9.0/G9.1 期间的推测号与草案建议值。
3. 互锁 validator 全绿后才允许 `src/spec/conformance` 改动；spec 条款 PR 先于实现 PR。

---

## 6. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-08 | 初版起草（状态「评审修订中」）：以 G9_DOCSET_PLAN_DRAFT 为蓝本，按 G8_PLAN 范式落七节结构；十条承接锚法定输入表；追加输入处置建议（M17 建议 go；M52/M61/Safe GPU/分包/神经变形/同日放行全部标「立项待裁决」）；五轨道波次 G9.0→G9.8b；P0→硬门覆盖表仅判据草案不 materialize CI 步骤；0-byte 边界与十条风险止损。零编号、零 registry、零 spec/src/conformance 改动。 |
| v1.1 | 2026-08-08 | **评审修订循环 → 计划定稿**：独立对抗性评审（explore 子代理，评审 provenance ≠ 起草 provenance）结论「有条件通过」，4 findings 全部处置——F-1（major）五份 `design/` 草案头部「G9.0 冻结引用」行实际落盘（正文 0-byte）；F-2（major）§5 待裁决表项 6 候选臂改写为先例真实对象（8a full-run 先行完成后同日进 8b），并备注蓝本「不得当日进 8a」与 G8_PLAN 字面不同、本计划采 G8_PLAN 字面；F-3（minor）§0 G8_CONTRACT:128 引用修正为原义；F-4（minor）§2 G9.6 补退出门判据草案（浮力旁路 RED / 判档不成立不充绿 / Jolt A/B 两臂诚实登记）。波次结构、P0 覆盖表、十锚表、矩阵行号映射 0-byte。状态升「计划定稿」。 |
