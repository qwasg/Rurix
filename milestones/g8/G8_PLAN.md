# G8_PLAN — UE5 级前置能力完成期 主线分解（G8.0 文档集草案）

> **状态**：**评审修订中（有条件通过，暂不定稿）**——2026-08-02 owner 评审结论：方向合理，但因 RD-038 承接缺口、条件型 RD 触发依据、反向依赖与退出门假绿风险，**暂不标「计划定稿」**。本文落盘时**不**立 G8_CONTRACT / CI_GATES / `g8_budget.json`、**不** claim number_ledger 任何编号空间（RFC/RXS/RD/SG/CI/U/RX 零占用）、零 `src/` 语义修改。
> **立项前置（硬）**：① G7_CONTRACT status flip closed（含 G-G7-9）；② **RD-038 status = closed**（逐字审计通过）；③ owner 立项指令。若 G7 按 G-G7-9 证据边界收口而 RD-038 仍 open，则**不得立项**，除非先完成 §1.0「RD-038 遗留分项 → G8 接入表」并经 owner 书面 override。立项时本文升格为 G8 契约上游事实源（同 [G6_PLAN](../g6/G6_PLAN.md) 升格路径）。
> **上游**：[G8_CAPABILITY_MATRIX.md](G8_CAPABILITY_MATRIX.md)（能力缺口矩阵；行号 M##）· [research/R1](research/R1_UE5_RENDERER_PANORAMA.md) / [R2](research/R2_PHYSICS_CHAOS_JOLT.md) / [R3](research/R3_GPU_API_ASSET_PIPELINE.md) · [G7_CONTRACT](../g7/G7_CONTRACT.md)（G-G7-9 字面允许 RD-038 保持 open 时收口）· `registry/deferred.json` v1.73。
> **推进形态**：**严格波次**——G8.0 文档集 → G8.1 治理/RFC/候选决策表 → G8.2~G8.7 实施 → **G8.8a soak** → G8.8b close-out。波次内可蜂群并行，波次间串行；禁止 stub/mock/host substitution 抢跑。

---

## 0. 定位与成功判据

**G8 = UE5 级渲染器与物理引擎的前置能力完成期。**

owner 定义（2026-08-02 会话）：完成「当前项目能胜任 UE5 级别渲染器和物理引擎」的**所有前置工作**。正式建造归 G9+。

「UE5 级」可核对基线 = UE 5.8（Epic：最后一个*计划内* UE5 大版本，保留发布 5.9 的可能性——State of Unreal 2026 / R1 §2）。验收五层级：**核心等价**、**功能闭环**、**可降级**、**可生产化**、**Vulkan 主线**。

**成功判据**：矩阵 §11.3 十条硬化为验收门；**全部 18 个 P0 行**各映射至少一门 CI 硬门 + evidence schema（§2.9 覆盖表，G8.1 落盘）；P1 按波次消化；P2 经 §2.7 穷举决策表 go/no-go，未触发维持 open、禁止假绿 close。

---

## 1. 择优裁决与在树候选处置

### 1.0 RD-038 与 G7 收口的硬衔接（阻断项修复）

[G7_CONTRACT G-G7-9](../g7/G7_CONTRACT.md) 字面：*「RD-038 按 title/backfill_condition 逐字审计后才可 closed，否则 G7 可按证据边界收口但必须明确 RD 保持 open」*。因此 **G-G7-9 ≠ RD-038 closed**。

| 情形 | G8 动作 |
|---|---|
| G7 closed **且** RD-038 closed | 正常立项；「承 G7」行视为前置已兑现 |
| G7 closed 但 RD-038 仍 open | **阻断立项**。须先填下表，把每个仍 open 的 RD-038 字面分项映射到 G8.2 或 G8.5a/b，并登记 owner override（只追加 deferred history，不改写 G7 契约） |
| G7 仍 active | 本文档集只允许勘误级修订，零编号占用 |

**RD-038 遗留分项 → G8 接入表（模板；立项前必须填满实测状态）**：

| RD-038 字面分项 | G7 兑现状态 | 若仍 open → G8 接入 | 退出门锚 |
|---|---|---|---|
| compute RayQuery codegen / AS descriptor | （填：closed / open） | G8.2 硬前置（不得跳过） | 同 G-G7-4/5 判据迁 G8 CI |
| gi_probe / rtao / hard_shadow 三核 | （填） | G8.2 或 G8.5b | 同 G-G7-6 |
| VisBuffer SW/HW 逐像素 diff=0 | （填） | G8.5a | 同 G-G7-7 |
| VSM depth/sample 真实进 device | （填） | G8.5a（与 M19 合流） | device 对拍 |
| TAA/TSR 非 host-only | （填） | G8.5b（与 M24 合流） | device 对拍 |
| One True Device Frame + soak | （填） | G8.5b 末或 G8.8a | 同 G-G7-8 |

> 「承 G7」矩阵行在 RD-038 未 closed 前**不得**视为已交付；G8_CAPABILITY_MATRIX 立项刷新时把未兑现行从「承 G7」改标「G8.x 接入」。

### 1.1 无条件并入（不改写 backfill）

| 候选 | 现状 | G8 处置 | 理由 |
|---|---|---|---|
| **RD-037** | open，「候选 G8」 | **正式并入 G8.2**（M89，P0） | 单源 gfx 是 UE5 级「以 `.rx` 为主语言」硬前置；backfill 判据字面不改 |
| **RD-034** | open，步骤 69 探针 | **不强攻**；G8.2 RT 仅 Vulkan 主腿 | 双上游钳制未解 |
| **RD-042/043** | open | **维持观察，不进 G8** | GPU 主刚体否决线；可微/机器人批仿属研究轨（Differentiable 观察挂 RD-042，不挂 RD-044） |
| **Safe GPU Operator Platform** | G7「G8 战略候选」 | **改挂 G9+**（留痕不消失） | 与 UE5 渲染/物理前置无依赖 |
| **RD-036** | open | 不进主线；upcall 硬需求出现时按其 backfill 判档 | 条件未触发 |

### 1.2 条件型 RD：禁止「以 UE5 目标直接主线化」

RD-039/040/041/044 的 backfill 均为「逐项独立判档」，触发条件字面含 *measured 瓶颈 / 真实资产需求 / 上游成熟* 等。**G8 不得用「UE5 级目标」静默改写这些条件。**

两类处置（互斥，G8.1 决策表二选一）：

1. **go（有证据）**：附 measured workload / 资产需求证据路径 → 进对应波次硬门。
2. **strategic_override（战略覆盖）**：owner 书面登记「以 UE5 前置完成期战略覆盖原 backfill」→ deferred history 只追加 override 行 → 进硬门；**未 override 且无证据 = no-go**，维持 open，进 G8.7 穷举表留痕，**不得假绿 close**。

**默认倾向（计划建议，非已 override）**：

| 分项 | 矩阵 | 建议默认 | 原 backfill 要点 | 证明 workload 候选 |
|---|---|---|---|---|
| 压缩/正式页格式 | M04 | go（格式 ABI 属基础设施，非优化项） | RD-039「场景超显存时」偏运行时流送；**格式定版**与超显存触发分离 | 格式 golden + 编解码往返 |
| HZB | M03 | no-go 除非 measured | RD-039「剔除效率成为 measured 瓶颈」 | uc06/动态场景 cull 计数器 |
| 世界辐射缓存 | M11 | no-go 除非 measured/画质门 | RD-040「屏幕探针远场缺失成为画质 measured 问题」 | GI 远场误差指标 |
| SMRT | M20 | no-go 直至 VSM device 化 | RD-040「VSM device 化后可独立 Mini」 | 依赖 RD-038/M19 |
| RT pipeline+SBT **增量面** | M50 | **strategic_override 候选**（须 G8.1 书面登记） | RD-040「命中点需多样化材质着色时」；但 Path Tracer/材质 hit lighting/SER·OMM 均阻塞于此 | 见 §2.2 增量清单（非 RXS-0248 最小见证） |
| SVT | M40 | no-go 除非资产面 | RD-041 标题含 SVT，但 backfill **无独立 SVT 门槛**——G8.1 须补登记「SVT 触发 = 真实大纹理资产管线出现」或 strategic_override | 大纹理 residency 证据 |
| 多层材质 slab | M28 | no-go 除非瓶颈/override | RD-041「单层闭合表达力成为真实资产瓶颈」 | MaterialClosure 表达力用例清单 |
| KTX2/Basis 转码 | M83 | go（cook 管线基础设施） | RD-041「真实纹理资产管线出现时」——与 G8.3 cook 同构触发 | cook 样例资产 |
| vendor 超分插件面 | M25 | go（接口已冻结，零底座改动） | RD-041 UpscaleBackend 留口 | 输入 ABI 契约测试 |
| Work Graphs / mesh 第三光栅 / Foliage 骨骼 / Mega Geometry / FG-MFG / SER·OMM / ReSTIR | 各 M | G8.7 穷举 | 各 backfill 字面 | 见 §2.7 |
| 布料 | M72 | go（Chaos 对照产品级缺口；R2 五门槛之一） | RD-044 布料分项 | 服装 schema+碰撞用例 |
| Rapier 深造 | （新标 M65b） | no-go 除非真实 workload 采用 | RD-044「快路径被真实 workload 采用时」 | parity 扩展场景 |
| Taichi 生产 external-import | M49a | G8.7 | RD-044 | — |
| Continuum/Fluid（软体·MPM·FLIP） | — | P3 观察 | RD-044 | — |
| Differentiable Physics | — | **挂 RD-042 观察**，不进 RD-044 四拆 | R2 §5.4 | — |

### 1.3 G8.1 必交付：候选分项决策表（完整映射）

G8.1 落盘 `milestones/g8/G8_CANDIDATE_DECISIONS.md`（或 CONTRACT 附录），**每一分项一行**：

`RD-id / 分项名 / M## / 原 backfill 字面 / 证明 workload / 证据路径 / go|no-go|strategic_override / 承接波次 / 最终期望状态（closed|open-留 G8.7|open-观察）`

并维护 **RD 分项 → M## → 波次 → 退出门 → 最终状态** 总表（覆盖 RD-037~044 全部分项 + RD-038 遗留）。缺行不得开工 G8.2。

### 1.4 RD-044 拆分（修正）

立项后建议登记的观察/承接面（**本文不动注册表**）：

| 子面 | 处置 |
|---|---|
| Cloth | G8.6d 主线（M72） |
| Continuum（软体/MPM） | P3 观察 |
| Fluid | P3 观察 |
| **Rapier 快路径深造** | G8.7 穷举（M65b）；默认 no-go |
| Differentiable | **不进 RD-044**；观察归 **RD-042** |

### 1.5 out-of-scope

| 项 | 依据 |
|---|---|
| 正式建造 UE5 级渲染器/物理引擎 | 归 G9+ |
| 编辑器 GUI / 完整网络引擎 / 音频 / 多 GPU / WebGPU | 超前置；SG-010 维持 |
| FG/MFG、coop-vector ABI 稳定化、AMDX enqueue、GPUDirect Storage | 矩阵 P3 / 不可验证 |
| GPU 主刚体、软体/流体进硬门 | G6 裁决 + 矩阵 §12 |
| Tensor Core / autodiff 语言核心 / fusion | SG-002/004/005 |
| 改写 G5/G6/G7 closed 契约与 00–14 | 只追加 |
| 无 measured baseline 的性能硬门 | P-09 |

---

## 2. 五轨道与波次分解

```text
G8.0 文档集草案
  → G8.1 治理+RFC+候选决策表+验收映射
  → G8.2 编译器/RHI/RT 增量 + RD-037
  → G8.3 资产闭环 + M01/M04 页格式 ABI（OMM baker 仅 go 时）
  → G8.4 多队列+流送（VT 门 ∥ 几何页门，禁止二选一充绿）
  → G8.5a 几何/阴影  →  G8.5b 材质/GI/时域/显示
  → G8.6a replay+Jolt A/B → G8.6b 网络+角色 → G8.6c 破坏 → G8.6d 布料+载具
  → G8.7 P2 穷举决策
  → G8.8a stabilization/soak → G8.8b close-out
```

### G8.0 — 文档集草案（本文档集）

- 落盘：本文 + 矩阵 + R1~R3。
- **不**定稿、**不**立契约四件套、**不** claim 编号。

### G8.1 — 治理包 + RFC + 决策表 + 验收映射（立项波）

前置：§1.0 硬条件满足。

交付：

1. 契约四件套 + ledger claim（next_free 校准，不预写号）。
2. **`G8_CANDIDATE_DECISIONS.md`**（§1.3）——条件型 RD 逐分项 go/no-go/override。
3. **`G8_ACCEPTANCE_MAP.md`**——全部 P0（及本波已 go 的 P1）的 `M## → CI step → evidence schema → 判据`；缺行阻断 G8.2。
4. 伞形 RFC（D-409）：
   - **RFC-α 渲染平台语义**（必含）：
     - RT pipeline **增量**能力（§2.2，超出 RXS-0248 最小见证）
     - RD-037 单源 gfx submit
     - capability/profile（M32）、permutation（M29）、reflection/hash（M31）
     - task shader 评估窗（M62）
     - TSR/WPO 材质时域语义（M24/M05）
     - **多层 closure 材质 IR 语义面（M28）**——即使实现波次在 5b，语义必须进 RFC-α
     - **多队列语义**（否则 G8.4 禁止启用 transfer queue）：queue-family ownership、timeline semaphore、跨队列 barrier、**无专用 transfer/compute 队列时的单队列 fallback**；与 G5 Barrier EB 三轴冻结面的修订边界
   - **RFC-β 资产管线**：schema/DDC/声明式工具/vendor 许可；**M01/M04 磁盘页格式版本与解码 ABI**（G8.3 冻结、G8.4 消费）
   - **RFC-γ 物理平台**：capture/replay、网络层、破坏资产模型、PhysicsAsset/布料；**replay corpus 先在 Jolt 5.3 建成**，再评估 5.6 升级
5. RTX 4070 Ti measured baseline → `g8_budget.json` 非空。

### G8.2 — 编译器 / RHI / RT 增量 + RD-037

| 面 | 内容 | 矩阵 |
|---|---|---|
| RT **增量**（不得仅重复 RXS-0248） | 多 hit group / 材质记录；SBT **用户数据**；any-hit / intersection / callable（按 RFC-α 冻结子集）；stack sizing；pipeline library；与 AsManager 单所有者 | M50 |
| 单源 | RD-037 三件全量；判据 = backfill 字面 | M89 |
| 编译/能力 | permutation + PSO precache/cache/binary + reflection schema/interface hash + capability/profile 语言面 + shader library 组合 | M29/M30/M31/M32/M33/M85 |
| RD-038 编译/执行遗留 | 若 §1.0 表指向本波：RayQuery/AS/三核等 | （接入表） |

**退出门（硬，防假绿）**：

- M50：上述增量面至少「多 hit group + SBT 用户数据 + stack sizing + pipeline library」device 真跑；any-hit/intersection/callable 按 RFC-α 子集有 RED-GREEN；**禁止**仅用现有 `vk_rt` 单 hit group 见证充绿。
- M89：零 Rust 宿主 `.rx` gfx 图 readback 像素断言。
- M29/M30：permutation 预算报告 + PSO precache 冒烟 + compile-stall 计数器非零采集路径。
- **M31/M32/M85：各有独立 CI 硬门**（reflection hash 稳定性 / capability 拒录 RED / manifest 进 DDC 往返），不得并入「冒烟三门」省略。

### G8.3 — 资产闭环 + 页格式 ABI

| 面 | 内容 | 矩阵 |
|---|---|---|
| schema/DDC/确定性 | SourceAsset/Recipe/Artifact/CookProfile + 双构建 hash 相等 | M79/M80 |
| 导入/纹理 | glTF + meshoptimizer 交叉验证；BCn/ASTC/KTX2/Basis | M81/M82/M83 |
| **页格式（前移）** | **M01 builder 侧版本化 + M04 压缩磁盘/内存页格式与解码 ABI 冻结**；golden 编解码往返 | M01/M04 |
| 打包 | chunk/manifest；shader/PSO manifest 进 DDC | M88/M85 |
| VT tile baker | 仅当 §1.3 对 M40/SVT 为 go 或 override | M84（VT 腿） |
| OMM baker | **默认不做**；仅当 M53 go/override | M84（OMM 腿） |
| BLAS 派生烘焙 | 可与页格式同波，供 G8.4 几何页门消费 | M84（BLAS 腿） |

退出门：双构建确定性 CI；glTF→DAG→页格式 golden；**M01/M04 格式 ABI 文档+golden 硬门**（缺则阻断 G8.4）。

### G8.4 — 多队列与流送

**前置**：RFC-α 多队列章 Approved；否则本波**强制单队列**（I/O 仍可做，但 copy 与 graphics 同 queue，timeline 跨队列门 SKIP 且不充绿）。

| 面 | 内容 | 矩阵 |
|---|---|---|
| I/O 链 | 磁盘异步 I/O + 解压 + 上传分离 timeline；迟到页降级 | M37 |
| 解压 | GDeflate + CPU fallback；GPU 解压预算 | M38 |
| 驻留 | sparse/tiled + （若 M40 go）SVT 运行时 + feedback feedback | M39/M40/M41 |
| 几何页 | meshlet page streamer **消费 G8.3 已冻结的 M04 ABI**（禁止本波重定格式） | M44 |

**退出门（VT 与几何页独立，禁止二选一）**：

- **门-VT**：若 M40 go → VT 按需驻留 + 迟到页路径独立 evidence；若 M40 no-go → 本门登记 SKIP=not-triggered（不充绿），不得用几何页证据替代。
- **门-GeomPage**：几何页按需驻留 + 迟到页路径独立 evidence（P1 硬门，与 VT 无关）。
- **门-MQ**：若启用多队列 → ownership/barrier/fallback 三断言；若单队列回退 → 契约字面登记。

### G8.5a — 几何 / 阴影

| 面 | 矩阵 |
|---|---|
| HZB（仅 go/override）/ programmable raster / RT fallback / WPO·位移实现 | M03/M05/M07/M08 |
| VSM 完整页缓存；SMRT（仅 VSM device 后且 go） | M19/M20 |
| RD-038 光栅/VSM 遗留分项 | §1.0 表 |

退出门：VSM 页缓存对拍；已 go 的几何分项各有 evidence；未 go 的 HZB/SMRT 不得出现在「全绿」叙述中。

### G8.5b — 材质 / GI / 时域 / 显示

| 面 | 矩阵 |
|---|---|
| 多层材质实现（M28 仅 go/override；语义已在 RFC-α） | M28 |
| 世界辐射缓存 / SWRT DF（仅 go） | M11/M13 |
| TSR 生产契约 + vendor 超分 | M24/M25 |
| HDR / 后处理 / 透明·OIT / Path Tracer 参照器 | M45/M46/M47/M17 |
| RD-038 GI/TSR/真帧遗留 | §1.0 表 |

退出门：各 go 分项独立对拍；Path Tracer 依赖 G8.2 RT 增量面。

### G8.6 — 物理平台（拆子波）

| 子波 | 内容 | 矩阵 |
|---|---|---|
| **G8.6a** | **先在 Jolt 5.3 建成 replay corpus** + capture/replay + 状态哈希 + divergence 定位；**再**做 5.3↔5.6 A/B（perf/CCD/determinism）；升级失败则钉 5.3 继续后续子波 | M66/M73 |
| **G8.6b** | 网络物理层 + CharacterVirtual + PhysicsAsset/ragdoll/physical animation | M67/M69/M71 |
| **G8.6c** | 破坏生产链（fracture cook 可复用 G8.3 资产图式） | M68 |
| **G8.6d** | 布料 + 载具产品层 | M72/M70 |

纪律：RFC-0017 五纪律 0-byte；GPU 主刚体禁止线维持。  
退出门：矩阵 §11.3 物理五条；6a corpus 未建成不得宣称 5.6 升级完成。

### G8.7 — P2 穷举决策表（软门，必须穷尽）

对下列**全部** P2 行逐条填 go/no-go/defer-to-G9+（不得遗漏）：

M06, M09, M12, M14, M15, M16, M22, M33（若 G8.2 未完）, M34, M41, M42, M43, M48, M49, M49a, M49b, M52, M53, M54, M55, M56, M59（async compute 第二腿）, M61, M62, M63, M65b（Rapier 深造）, M74, M75, M77, M86, M87

软门：失败/未触发 → 诚实登记，**不**阻塞进入 G8.8a；但 close-out 审计要求本表无空行。

### G8.8a — Stabilization / soak（close 前必经）

- 全 P0 硬门回归 + 已 go 的 P1 回归；既有步骤 41~92 及 G7 步骤判据 0-byte。
- 代表性场景 soak（时长/帧数阈值由 G8.1 baseline 追加，不得低于 G-G7-8 量级除非 measured 证明更短足够）。
- budget_eval --strict 非空、零 estimated/skip。
- 条件实现刚绿**不得**当日进 8b。

### G8.8b — Close-out

- 验收映射表终审；RD 分项最终状态与 §1.3 决策表逐字一致。
- 契约 §8 只追加 + status flip。

### 2.9 P0 → 硬门覆盖（G8.1 固化为 ACCEPTANCE_MAP）

| P0 | 最晚波次 | 硬门要点（摘要） |
|---|---|---|
| M50 | G8.2 | RT **增量**面（非 RXS-0248 重复） |
| M89 | G8.2 | RD-037 像素断言 |
| M29 | G8.2 | permutation 预算/裁剪 CI |
| M30 | G8.2 | PSO precache/cache CI |
| M31 | G8.2 | reflection hash 稳定 CI |
| M32 | G8.2 | capability 拒录 RED CI |
| M85 | G8.2/3 | manifest↔DDC 往返 CI |
| M79 | G8.3 | 双构建 hash |
| M80 | G8.3 | DDC 内容寻址 |
| M81 | G8.3 | glTF 导入烟测 |
| M01 | G8.3 | builder 版本化/golden |
| M04 | G8.3 | 页格式编解码 golden（**前移**） |
| M37 | G8.4 | 磁盘→GPU I/O 链 |
| M19 | G8.5a | VSM 完整页缓存对拍 |
| M24 | G8.5b | TSR 生产契约对拍 |
| M66 | G8.6a | capture 重演+divergence |
| M67 | G8.6b | 网络回滚链 |
| M68 | G8.6c | 破坏全链 |

任一 P0 无独立硬门 → **禁止** G8.8b status flip。

---

## 3. 与既有面的边界（0-byte 纪律）

| 面 | 约束 |
|---|---|
| G7 车道 | 四件套/RFC-0018/G7 claim 段/g7_budget 0-byte；立项等 §1.0 硬条件 |
| G5/G6 冻结面 | `GpuScene`/`MaterialClosure` 32B/`Barrier` EB 三轴/`PageRequest`/物理五纪律经 RFC 修订方可动；**多队列若触 Barrier 冻结面必须在 RFC-α 显式修订行** |
| 00–14 | 只勘误 |
| spec/conformance | G8.0 期 0-byte；G8.1 起 spec-first + RED 先行 |
| 注册表 | G8.0 期 0-byte；登记/翻转归立项后 |
| unsafe | U 段按立项 next_free；不碰 G7 在途 claim；`rurix-render` forbid(unsafe) 维持 |

---

## 4. 风险与止损

| ID | 风险 | 预警 | 止损 |
|---|---|---|---|
| R-G8-1 | RT 语义扩张 / 假绿重复 RXS-0248 | 退出门无「增量」字样 | ACCEPTANCE_MAP 增量清单硬核 |
| R-G8-2 | 资产管线爆炸 | USD/GUI 代码进 G8.3 | 最小闭环 glTF+BCn+DDC；USD/MaterialX→G8.7 |
| R-G8-3 | 条件型 RD 静默主线化 | 决策表空行或无证据 | §1.2/1.3 阻断 G8.2 |
| R-G8-4 | M04/M44 反向依赖 | G8.4 重定格式 | 格式 ABI 锁在 G8.3 |
| R-G8-5 | 多队列无 RFC 抢跑 | G8.4 直接用 transfer queue | 无 RFC-α → 强制单队列 |
| R-G8-6 | RD-038 无人接 | G7 收口后「承 G7」空窗 | §1.0 阻断立项或接入表 |
| R-G8-7 | 物理/渲染争用 | GPU 刚体提案 | G6 禁止线 + 矩阵 §12 |
| R-G8-8 | 预算空壳 | 首实现 PR 前 JSON 空 | G8.1 baseline 硬阻断 |
| R-G8-9 | 条件实现后立即 close | 跳过 8a soak | 8a 为 8b 前置硬门 |

---

## 5. 立项条件与四件套落位清单

1. G7 status closed **且** RD-038 closed（或 §1.0 接入表 + owner override 齐备）。
2. owner 立项指令留痕。
3. 落 CONTRACT / CI_GATES / budget 空壳 + 本 PLAN 升格（届时才可将状态改为「计划定稿/active」）。
4. ledger `reserved_in_flight[G8]` claim。
5. RFC-α/β/γ Approved；**α 含多队列与 M28 语义**。
6. `G8_CANDIDATE_DECISIONS.md` + `G8_ACCEPTANCE_MAP.md` 齐备。
7. 00/README 状态行 errata。
8. RD 承接 history 只追加（037 并入；039/040/041/044 按决策表；044 含 Rapier 深造面；Differentiable→042）。

---

## 6. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-02 | 初版文档集草案（当时误标「计划定稿」）。 |
| v1.1 | 2026-08-02 | **评审修订（有条件通过→暂不定稿）**：① 立项前置改为 G7 closed **且** RD-038 closed，否则强制遗留接入表；② 条件型 RD 禁止以 UE5 目标静默改写 backfill，G8.1 必交决策表（go/no-go/strategic_override）；③ RD-044 四拆改为 Cloth/Continuum/Fluid/**Rapier 深造**，Differentiable 归 RD-042；补 SVT 触发门槛说明；M28 纳入 RFC-α + G8.5b；④ M01/M04 页格式 ABI 前移 G8.3，消除 G8.4←M04 反向依赖；⑤ 多队列语义强制进 RFC-α，否则 G8.4 单队列；⑥ P0 全量硬门覆盖表，M50 退出门改为 RXS-0248 **增量**面；⑦ 波次重划：5a/5b、6a–6d、7 穷举 P2、8a soak→8b close；⑧ 状态降为「评审修订中」。 |
