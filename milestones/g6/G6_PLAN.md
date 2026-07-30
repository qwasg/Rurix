# G6_PLAN — 渲染物理双轨架构与主线分解

> **状态**:**active**(G6.1 治理包已开工,2026-07-30)——契约 [G6_CONTRACT.md](G6_CONTRACT.md) · 门 [CI_GATES.md](CI_GATES.md) · 预算 g6_budget.json 空壳 · 编号 claim `number_ledger` v1.28 `reserved_in_flight[G6]`。本文升格为 G6 契约上游事实源(G6_CONTRACT upstream_docs 首条)。
>
> **上游**:G5 closed([G5_CONTRACT.md](../g5/G5_CONTRACT.md) · [G5_PLAN.md](../g5/G5_PLAN.md) · [RFC-0016](../../rfcs/0016-native-renderer.md)) · 物理选型调研(会话调研 + Cursor canvas `physics-engine-match`) · 仓内后端纪律([`src/rurix-rt/src/backend.rs`](../../src/rurix-rt/src/backend.rs))
>
> **推进形态**:**波次(wave)推进**——G6.0 计划定稿 → G6.1 治理与 RFC(未开工)→ G6.2 物理库底座 → G6.3 与渲染合流 → G6.4 Rapier 快路径 → G6.5 Taichi 特效 spike → G6.6 close-out。波次内可并行、波次间严格串行(每波结束全 workspace build/test 绿才进下一波)。

---

## 0. 多项择优裁决与依赖图

### 0.1 择优表(已锁定)

跨域性能不可直接横比:游戏刚体天花板 ≠ 连续体/可微/机器人批仿天花板。下列裁决相对 **rurix 现状**(G5 原生渲染器 + compute `cuda|vulkan` + 渲染主线 Vulkan)加权。

| 轨 | 选型 | 裁决 | 理由(相对 rurix) |
|---|---|---|---|
| 主物理 | **Jolt Physics** | 生产默认 | AAA 刚体多核/睡眠/批插体/并发查询;MIT;与 Vulkan 渲染正交,不占 VisBuffer/VSM/GI/RT 车道 |
| 快路径 | **Rapier**(feature `rapier`) | 非默认 | 纯 Rust、CI/host 无 CMake;「多静少动」开放世界弱于 Jolt;同场景 host 对拍用 |
| 特效副轨 | **Taichi Vulkan AOT** | 后期波次 | 仓内 Vulkan 主线 + AOT 可挂 VkDevice;只做 MPM/连续体粒子·体积场,不进刚体求解 |
| 研究隔离 | Newton / Genesis / MuJoCo Warp | 库外 | CUDA/可微/机器人;不进引擎主环与 CI 硬门 |
| 否决作主物理 | PhysX-GPU、Havok、FleX、Avian、Bullet | 否决 | CUDA 绑定 / 商用许可 / 遗留粒子栈 / Bevy 绑 / 多核维护落后 |

**GPU 主刚体不做**(含 PhysX CUDA 刚体、wgrapier 生产依赖、Warp/Newton 主环):游戏刚体预算走 CPU 多核;Vulkan 车道优先留给 G5 已交付效果面。

### 0.2 仓内后端纪律(已核实,选型约束)

- **Compute 双后端**:[`BackendKind::Cuda | BackendKind::Vulkan`](../../src/rurix-rt/src/backend.rs);`RURIX_BACKEND` 显式选择;默认 CUDA;缺驱动确定性 `Err`,**绝不静默回退**(P-01 / RFC-0016 §4.0-2 同构)。
- **渲染主线 = Vulkan**(RFC-0016):`VK_KHR_ray_query` / `VK_KHR_shader_atomic_int64` / synchronization2;能力查询 fail-closed。
- **DXIL RT 腿**维持 blocked([G5_CONTRACT](../g5/G5_CONTRACT.md) `rd034_dxil_rt` / RD-034);本期物理不依赖 DXIL。
- **推论**:主物理必须与图形/compute 后端**正交**(CPU 库);特效副轨若上 GPU,优先 **Vulkan AOT**(与渲染同设备),不引入第二套 CUDA 物理 runtime 作主环。

### 0.3 与 G5 已交付面的合流点

```
G5 rurix-render
  graph / geometry / shadow / gi / rt / material / streaming / temporal
       ▲
       │ 变换·MV·接触(单向)
G6 rurix-physics (Jolt default)
       ▲
       │ body 批插/移除
G5 streaming PageRequest / 页驻留
       │
G5 GpuScene 实例表 ──► render_exec (Vulkan)
       │
G6.5 Taichi AOT(可选) ──► external import 粒子/体积场 ──► graph
```

| G5 面 | G6 消费方式 |
|---|---|
| `GpuScene` / `update_transform` / `flush_dirty`([`geometry/gpu_scene.rs`](../../src/rurix-render/src/geometry/gpu_scene.rs)) | 物理步后写实例变换;渲染只读 |
| `PageRequest` / `StreamingBudget`([`graph/types.rs`](../../src/rurix-render/src/graph/types.rs)) | 页驻留驱动 body 批插入;卸载驱动移除 |
| 时域底座 MV([`temporal/`](../../src/rurix-render/src/temporal/)) | 动态体上一拍→当前拍变换差分供 TAA/TSR;禁效果 pass 私写重投影(RFC-0016 §4.H) |
| AS / TLAS([`rt/`](../../src/rurix-render/src/rt/)) | 动态实例变换变更触发 BLAS refit / TLAS 重建分级(复用 G5 决策树,不新建第二套) |
| `MaterialClosure` 32B | 物理不改材质闭合;碰撞形状与材质槽解耦 |

### 0.4 最终架构

```
┌──────────────────────── Host Engine Libs (06 §8.3「库不进语言」) ────────────────────────┐
│  streaming ──batch_body_prep──► rurix-physics (Jolt default / Rapier feature)            │
│                                      │ transforms / contacts / MV hints                   │
│                                      ▼                                                    │
│                                 GpuScene ─────────────────────────────────► rurix-render │
│                                      │                                         │          │
└──────────────────────────────────────┼─────────────────────────────────────────┼──────────┘
                                       │                                         ▼
                                       │                              render_exec (Vulkan)
                                       │                                         ▲
                                       │    Taichi Vulkan AOT (optional G6.5)    │
                                       └────── particle/volume buffers ──────────┘
                                              (external import; 不进刚体求解)

主物理 ──X──► GPU sim on render queue   (禁止:不与 VisBuffer/VSM/GI/RT 抢车道)
```

---

## 1. 波次分解

### G6.0 计划定稿(本文)· 已完成

- 落盘本文件;择优裁决与双轨架构冻结为后续开工输入。
- **不**立 CONTRACT / CI_GATES / `g6_budget.json` / ledger claim;编号空间不占用。

### G6.1 治理包 + RFC · **开工(治理包已落,2026-07-30;RFC 在途)**

对齐 G5.0/G5.1 体例:

- 契约四件套:`G6_CONTRACT.md` / 本 PLAN(升格引用) / `CI_GATES.md` / `g6_budget.json` 空壳。**已落(2026-07-30)**。
- `number_ledger` `reserved_in_flight[G6]` claim(v1.28:RFC-0017 / 步骤 88 起 / RD-042 起 / U33 起,以开工时 ledger 实际 `next_free` 校准)。**已落**。
- RFC:物理库边界 + 同步契约 + FFI/unsafe 纪律(判档争议向上取严;预期零新语言语义条款——物理为引擎库,同 06 §8.3 / RFC-0016 口径)。
- D-409 对抗性评审 → Agent Approved 先于实现 PR。

### G6.2 物理库底座(`rurix-physics`)

| 面 | 内容 | 主要落点 |
|---|---|---|
| P | Jolt 集成(生产默认):世界/层/固定步、睡眠、批插体、CCD 开关、job 系统接宿主线程池 | `src/rurix-physics/`(+ JoltC/FFI 子 crate 若需) |
| Q | 查询面:ray/shape cast、overlap;强调**并发 query**(与渲染同帧可读上一拍) | 同上 |
| E | 事件面:接触开始/持续/结束(有界队列 + 每帧 `SyncBudget`) | 同上 |
| U | unsafe 集中:全部 FFI `// SAFETY:` + unsafe-audit 续号;对外 safe API | `unsafe-audit/` |
| T | host 单测:堆叠沉降、睡眠唤醒、批插体不锁死主步、query 与 step 并发烟测 | `tests` in-crate |

集成门(拟定,正式门编号随 G6.1 契约落):workspace 绿 + Jolt 路径固定步确定性烟测(同输入同输出,平台内)。

### G6.3 与渲染合流

| 面 | 内容 | 主要落点 |
|---|---|---|
| S | `PhysicsTransform` → `GpuScene::update_transform` / `flush_dirty` 单向同步 | physics ↔ render bridge |
| M | 动态体 MV 供给时域底座(上一拍→当前);静态/睡眠体零 MV | `temporal/` 消费侧 |
| L | 流送↔body:页驻留→批插入;卸载→移除;与 `PageRequest` 同帧预算 | `streaming/` + physics |
| A | TLAS/BLAS:变换脏实例走 G5 refit 分级,不新建加速结构所有者 | `rt/as_manager` |
| D | demo:扩展 `uc06-renderer` 或新 `uc0x-physics`——刚体场景 + 既有 VisBuffer/GI/VSM/TAA 管线真跑 | `apps/` |

集成门(拟定):host 恒跑同步正确性 + device gate real(Vulkan)像素/变换非平凡断言 + 既有 G5 步骤 82+ 零回归。

### G6.4 Rapier 快路径

- feature `rapier`:同 `PhysicsWorld` 抽象第二后端;默认 off。
- 同场景 host 对拍门:变换/接触集合容差断言(非跨引擎逐位);CI 无 CMake 路径可跑。
- **不**替换生产默认;文档与 demo 明确「快路径 ≠ 性能/稳定性默认」。

### G6.5 Taichi Vulkan AOT 特效 spike

- 可选交付:AOT 模块 + TiRT 挂已有/并行 Vk 设备上下文;输出粒子或体积场 buffer → graph **external import**。
- 失败或能力缺口 → 诚实登记 RD(编号随 G6.1 claim),**不**阻塞 G6.3/G6.4 硬门。
- 禁止:用 Taichi 替代主刚体、把确定性联网绑到粒子求解、在 CUDA 后端上另起「主物理」。

### G6.6 close-out

- 全量回归冻结 + 门终审表 + RD/SG 处置 + CONTRACT status flip(若 G6.1 已开工)。
- P3+ / 研究轨(Newton/Genesis/wgrapier 观察)登记 deferred,不进硬门。

---

## 2. 冻结接口(G6.2 开工前固化,波次内不得漂移)

下列为计划级契约草案;G6.1 RFC Approved 后字面冻结,实现 PR 不得漂移。

### 2.1 物理库面

- `PhysicsWorld` — 固定步 `step(dt_fixed)`(accumulator 在宿主);后端枚举 `Jolt`(default) / `Rapier`(`rapier` feature)。
- `BodyId` / `ShapeId` — 不透明句柄;禁止渲染器持有原生 Jolt/Rapier 指针。
- `PhysicsTransform { translation: [f32; 3], rotation: [f32; 4] /* xyzw quat */ }` — 与 `GpuScene` 实例 3×4 变换的唯一桥接输入(由 bridge 合成 3×4)。
- `BodyDesc { kind: Static|Kinematic|Dynamic, shape, layer, mass_props, ccd }`
- `ContactEvent { a: BodyId, b: BodyId, phase: Begin|Persist|End, ... }` — 每帧有界 drain。
- `QueryRay { origin, dir, t_min, t_max, layer_mask }` / `QueryHit` — 支持 step 外并发调用(Jolt 路径硬需求)。
- `SyncBudget { max_body_writes, max_contact_events, max_query_casts }` — 每帧重置,防物理→渲染写爆。

### 2.2 与 G5 冻结面的边界

- **只写**:`GpuScene` 变换脏集;可选 MV 缓冲(时域 import)。
- **不写**:`MaterialClosure`、VisBuffer 格式、`Barrier` EB 三轴、`PageRequest` 字段布局(G5 §2 0-byte)。
- **流送**:物理只订阅「页驻留/卸载」通知;不重新实现 `StreamingBudget` 计量。
- **跨帧**:物理世界状态在库内;渲染侧历史(TAA/VSM/GI)仍为 external import,与物理世界**分属**。

### 2.3 同步纪律(五条)

1. **单向事实源**:动态/运动学变换仅由 `rurix-physics` → `GpuScene`;渲染器不回写物理。
2. **查询并行**:角色/AI/拾取走并发 query;与 `render_exec` 同帧可读上一拍变换。
3. **流送同构**:几何页驻留/卸载驱动 body 批插入/移除(对齐 Jolt batch insert + G5 `PageRequest`)。
4. **特效隔离**:Taichi AOT 只产出粒子/体积场,经 external import 进 graph;不进刚体求解、不承担确定性联网。
5. **库不进语言**:同 06 §8.3;FFI 集中在 `rurix-physics`(及绑定 crate);[`rurix-render`](../../src/rurix-render/src/lib.rs) 维持 `#![forbid(unsafe_code)]`。

### 2.4 建议 crate 落点(实现期)

| Crate | 职责 |
|---|---|
| `src/rurix-physics` | safe 公共 API、`PhysicsWorld`、同步 bridge、Rapier feature |
| Jolt FFI 子 crate(若拆) | `joltc`/`rolt` 类绑定 + unsafe-audit |
| `apps/uc0x-physics` 或 uc06 扩展 | 合流 demo |
| **不**进 | `rurixc` 语言语义、`spec/` 新条款(除非 G6.1 条件消费判档成立) |

---

## 3. Out of scope

| 项 | 说明 |
|---|---|
| GPU 主刚体 | PhysX CUDA 刚体、wgrapier 作验收依赖、Warp/Newton 主环 |
| 商用 Havok | 许可与可审计引擎库路线冲突 |
| 软体/布料/流体进硬门 | Jolt CPU 软体与 Taichi MPM 仅 spike/副轨;不进 G6 硬门 |
| Newton / Genesis 合入主仓 CI | 研究隔离;独立仓库或 feature 永不默认 |
| DXIL RT、窗口/输入进语言 | D-130 / RD-034 维持 |
| 性能数字进硬门 | measured 写 evidence(P-09);预算条目随 G6.1 budget 空壳开工 |
| 改写 G5 closed 契约正文 | 只追加引用;milestones/g5 既有条款 0-byte |
| 改写 00–14 规划文档 | `check_planning_docs` 纪律 |

---

## 4. 风险与诚实边界

| ID | 风险 | 处置 |
|---|---|---|
| R-G6-1 | Jolt Rust 绑定(jolt-rust/rolt)未臻生产完备 | G6.2 允许自维护 JoltC FFI;审计面集中;不阻塞择优裁决 |
| R-G6-2 | Rapier 与 Jolt 同场景数值不可逐位对拍 | G6.4 门禁用逐位;改用容差/接触集合不变量 |
| R-G6-3 | Taichi AOT 与现有 `render_exec` 设备共享摩擦 | G6.5 失败 → RD;不挡主物理 |
| R-G6-4 | 物理写 `GpuScene` 与流送卸载竞态 | `SyncBudget` + 先卸载 body 再放页;单测注入 |
| R-G6-5 | 误把 CUDA compute 默认当成「应上 CUDA 物理」 | §0.2 纪律钉死:主物理 CPU;特效优先 Vulkan AOT |

---

## 5. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-07-30 | 初版(G6.0 计划定稿):多项择优锁定 Jolt 主物理 / Rapier 快路径 / Taichi Vulkan AOT 特效副轨 / Newton 系研究隔离;双轨架构与 G5 GpuScene·streaming·temporal 合流;波次 G6.0–G6.6;冻结接口草案;治理包未开工、编号未 claim |
| v1.1 | 2026-07-30 | G6.1 治理包开工升格(owner 同日会话立项指令):状态行翻 active + 契约四件套/ledger v1.28 claim 已落标注;§1 G6.0/G6.1 波次状态刷新;§0/§2/§3/§4 既有裁决与冻结接口草案 0-byte 不动 |
