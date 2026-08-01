# G6_PLAN — 渲染物理双轨架构与主线分解

> **状态**:**active**(G6.5 已完成 2026-07-31:G-G6-6 软门绿〔成功臂〕,CI 步骤 92 落地——TiRT FFI 边界 `src/rurix-rt/src/tirt.rs`(feature `taichi-tirt` 默认 off,13 符号动态装载 taichi_c_api.dll + `TiVulkanRuntimeInteropInfo` 并行设备上下文注入 + `run_particles_spike` 全链 + `TirtError` 六变体,U43 登记)+ spike demo `apps/uc09-taichi-spike`(host 8 断言 + device 五断言)+ AOT 资产三件(particles.tcm 3873B sha256 登记 + gen 脚本)+ 冒烟 `ci/taichi_vulkan_spike_smoke.py`(host 六段恒跑 + device gate real + --selftest);device 真跑一次即绿(RTX 4070 Ti:TiRT launch + 导出 VkBuffer 256B + graph external import 消费接线 + readback 64/64 非零,first_values=[1.0,2.5,4.0,5.5] 与 i*1.5+1.0 逐位相等),零 tirt 修复、RD-042 未消费;下一波 G6.6 close-out)——契约 [G6_CONTRACT.md](G6_CONTRACT.md) · 门 [CI_GATES.md](CI_GATES.md) · 预算 g6_budget.json 空壳 · 编号 claim `number_ledger` v1.28 `reserved_in_flight[G6]`(v1.29:RFC-0017 已 materialize,RFC on_tree_max 17 / next_free 18 校准;v1.30:U33~U42 已 materialize,U on_tree_max 42 / next_free 43 校准;v1.31:CI 步骤 88/89/91 已 materialize,CI_step on_tree_max 91 / next_free 92 校准,90 维持 G6.4 拟分配;v1.32:CI 步骤 90 已 materialize,CI_step on_tree_max 91 / next_free 92 不变;v1.33:CI 步骤 92 已 materialize,CI_step on_tree_max 92 / next_free 93 校准;U43 已 materialize,U on_tree_max 43 / next_free 44 校准)。本文升格为 G6 契约上游事实源(G6_CONTRACT upstream_docs 首条)。
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

### G6.1 治理包 + RFC · **已完成(2026-07-31:RFC-0017 Agent Approved)**

对齐 G5.0/G5.1 体例:

- 契约四件套:`G6_CONTRACT.md` / 本 PLAN(升格引用) / `CI_GATES.md` / `g6_budget.json` 空壳。**已落(2026-07-30)**。
- `number_ledger` `reserved_in_flight[G6]` claim(v1.28:RFC-0017 / 步骤 88 起 / RD-042 起 / U33 起,以开工时 ledger 实际 `next_free` 校准)。**已落**。
- RFC:**已落(2026-07-31)**——[RFC-0017](../../rfcs/0017-engine-physics.md) 伞形五章(A 物理库边界 / B 渲染同步契约 / C FFI 与 unsafe 纪律〔R-G6-1:自维护 JoltC FFI〕/ D Rapier 快路径 / E Taichi Vulkan AOT 副轨;预期零新语言语义条款,同 06 §8.3 / RFC-0016 口径)。
- D-409 对抗性评审 → Agent Approved 先于实现 PR:**已完成(2026-07-31)**——评审 provenance `kimi-cli:kimi-for-coding` 独立实例 ≠ 起草 `Kimi Code CLI (Kimi)`(首选 claude 403 不可得,跨工具/同模型族偏差如实登记 RFC-0017 §9.1 环境留痕,RFC-0015 先例),17 findings 全部采纳并修;§2 冻结接口草案经 RFC Approved 字面冻结(实现 PR 不得漂移,修订点以 RFC §9.1 disposition 为准)。

### G6.2 物理库底座(`rurix-physics`)· **已完成(2026-07-31:G-G6-3 物理底座门绿,CI 步骤 88 恒跑 PASS)**

| 面 | 内容 | 主要落点 |
|---|---|---|
| P | Jolt 集成(生产默认):世界/层/固定步、睡眠、批插体、CCD 开关、job 系统接宿主线程池 | `src/rurix-physics/`(+ JoltC/FFI 子 crate 若需) |
| Q | 查询面:ray/shape cast、overlap;强调**并发 query**(与渲染同帧可读上一拍) | 同上 |
| E | 事件面:接触开始/持续/结束(有界队列 + 每帧 `SyncBudget`) | 同上 |
| U | unsafe 集中:全部 FFI `// SAFETY:` + unsafe-audit 续号;对外 safe API | `unsafe-audit/` |
| T | host 单测:堆叠沉降、睡眠唤醒、批插体不锁死主步、query 与 step 并发烟测 | `tests` in-crate |

集成门(拟定,正式门编号随 G6.1 契约落):workspace 绿 + Jolt 路径固定步确定性烟测(同输入同输出,平台内)。

### G6.3 与渲染合流 · **已完成(2026-07-31:G-G6-4 合流门 + G-G6-7 demo 门绿,CI 步骤 89/91 落地)**

| 面 | 内容 | 主要落点 |
|---|---|---|
| S | `PhysicsTransform` → `GpuScene::update_transform` / `flush_dirty` 单向同步 | physics ↔ render bridge |
| M | 动态体 MV 供给时域底座(上一拍→当前);静态/睡眠体零 MV | `temporal/` 消费侧 |
| L | 流送↔body:页驻留→批插入;卸载→移除;与 `PageRequest` 同帧预算 | `streaming/` + physics |
| A | TLAS/BLAS:变换脏实例走 G5 refit 分级,不新建加速结构所有者 | `rt/as_manager` |
| D | demo:扩展 `uc06-renderer` 或新 `uc0x-physics`——刚体场景 + 既有 VisBuffer/GI/VSM/TAA 管线真跑 | `apps/` |

集成门(拟定):host 恒跑同步正确性 + device gate real(Vulkan)像素/变换非平凡断言 + 既有 G5 步骤 82+ 零回归。

**实施留痕(2026-07-31)**:

- **合流桥** `src/rurix-physics/src/bridge/`:`compose_transform_3x4` / `PhysicsBridge`(sync_frame 单向写 GpuScene、SyncBudget 截断、motion_hints 动态体 MV 提示、dirty_instances 供 AS 脏信号)+ `StreamingBridge`(insert_page/remove_page + `RemovalReceipt` 移动语义凭据,先卸 body 再放页,编译期不可伪造);`tests/bridge.rs` 七项行为测试全绿(one_way_sync_writes_active_dynamic_only / sleeping_body_zero_write_zero_mv / budget_truncation_deterministic / flush_dirty_ranges_match_dirty_instances / motion_hint_tracks_prev_cur / streaming_insert_on_residency_and_remove_receipt / unload_race_injection_no_dangling)。
- **sys 缺陷修复**(实施期发现):`src/rurix-physics-sys/src/world.rs` 批插入在 >32 体混 broadphase 层批下被 Jolt `AddBodiesPrepare` 内部 QuickSort 非稳定重排(≤32 回退插入排序恒等,故 G6.2 未暴露),返回序/kind 登记/激活三错位;修复 = prepare 前快照 `ids_orig` 三处按原始序配对;回归测试 `mixed_layer_batch_insert_order_preserved`(35 体混批)先红后绿;零新增 unsafe,SAFETY 注释增强。
- **合流 demo** `apps/uc08-physics`(RFC-0017 Q-D 定名,UC-07 已被 ruridrop 占用):地面 + 5 动态立方体落堆(60 帧入睡)+ 远场景第 6 立方体流送剧本(帧 10 驻留沿批插/帧 29 卸载 receipt 放页);host 15-pass 管线(VisBuffer/GI/VSM/TAA 全跑)新增 physics/bridge_sync/mv 阶段;16 断言全绿(物理步 measured 8.05ms/96 步、transform_landed max_err 5.5e-5、MV 动态区 0.01224 vs 睡眠后 3.4e-6、TLAS rebuild 61 次/BLAS 零 refit);device 腿 RURIX_REQUIRE_REAL=1 真跑 exit 0(RTX 4070 Ti,changed_pixels=168);cargo test 11 绿;clippy/fmt 绿。
- **CI 步骤**:89 `ci/physics_bridge_smoke.py`(门 G-G6-4)+ 91 `ci/uc08_physics_smoke.py`(门 G-G6-7)落地,evidence 落 `evidence/physics_bridge_smoke_*.json` / `evidence/uc08_physics_smoke_*.json` 过 `ci/check_schemas.py`;uc06-renderer 0-byte(G5 步骤 82~87 零回归)。

### G6.4 Rapier 快路径 · **已完成(2026-07-31:G-G6-5 门绿,CI 步骤 90 落地)**

- feature `rapier`:同 `PhysicsWorld` 抽象第二后端;默认 off。
- 同场景 host 对拍门:变换/接触集合容差断言(非跨引擎逐位);CI 无 CMake 路径可跑。
- **不**替换生产默认;文档与 demo 明确「快路径 ≠ 性能/稳定性默认」。

**实施留痕(2026-07-31)**:

- **第二后端** `src/rurix-physics/src/rapier.rs`(rapier3d =0.33.0 pin,optional,默认 off;纯 Rust 零 CMake CI 面):`PhysicsWorld` 同抽象 Rapier 实现,方法面镜像 `SysWorld` 消费形态(token u64 出/入),safe 层自维护机制两后端共享零分叉;`src/world.rs` 后端枚举三档分派(Jolt 默认 / Rapier feature / 无后端确定性 `Err(BackendNotCompiled)`),Rapier 变体 Box 化;`tests/behavior.rs` 双后端循环(测试名逐字未变)、`tests/api.rs` cfg 翻转;crate 维持 `#![forbid(unsafe_code)]`,零新 unsafe。
- **两缺陷随波修复**(集成轮探针定位,先红后绿):① **宽相 pair 注册闭环**——rapier BVH `AddPair` 事件仅向本次 `update` 调用方报告而窄相 `register_pairs` 为 crate 私有,手动 `broad_phase.update` 吞事件致初始即相交 pair(缝 ≤ prediction distance 0.002m)永不注册窄相、该对穿透(探针实测箱底缝 0.001 穿透地面);修复 = add 时录 `pending_reinsert`,下一拍 `step` 开头先从 BVH 删除再交 `pipeline.step` 自闭环重建,AddPair 重报、窄相注册完整。② **睡眠接触语义对齐 Jolt**——睡眠即接触约束移除(任一体入睡 → 该对视同移除发 End,唤醒再接触 = 新 Begin),弃用 `has_any_active_contact` 口径(入睡对 solver_contacts 为空会误发 End),改几何接触点 `dist ≤ 0` 判定 + 入睡对差分发 End。
- **对拍门** `tests/parity.rs`(§4.D3 全判据:变换容差逐拍逐体 / Begin·End 接触集合重叠 ≥0.99 / 相位等价类 / 各后端进程内重放逐位):场景 = 箱塔沉降 + 球滚动 + 批插移除脚本,N=300 固定步 dt=1/60;阈值实测标定 pos 0.82m / rot 93° / 重叠 ≥0.99(= 实测 max 0.541314125m / 61.774520874° × 1.5,跨后端偏差为确定性常量、6 次跨进程复跑逐位恒定方差零,jolt hash bfa449a7a5515449 / rapier hash 7e7ddda2f21e23d0),evidence `evidence/physics_rapier_parity_20260731T120019.json`(PARITY_JSON 落盘)。
- **CI 步骤 90** `ci/physics_rapier_parity_smoke.py`(纯 host 门七判据:默认 off 核验 / rapier-only 无 CMake 路径 / cargo test 两腿 / rapier 两 tier clippy / parity 进程级重放一致 / 文档口径 grep「快路径 ≠ 性能/稳定性默认」/ evidence 过 schema)+ `physics_rapier_parity_evidence_schema.json`(单 schema 双形态)+ pr-smoke.yml 真步骤替换占位注释;步骤 88 v1.4 收窄(rapier3d 原生类型名仅许 `src/rapier.rs` 单模块,RFC-0017 v1.4 修订行 + §9.1 留痕,冻结条文 0-byte);四档矩阵(default / `--no-default-features` / rapier-only / `--features rapier`)test+clippy 全绿。

### G6.5 Taichi Vulkan AOT 特效 spike · **已完成(2026-07-31:G-G6-6 软门绿〔成功臂〕,CI 步骤 92 落地)**

- 可选交付:AOT 模块 + TiRT 挂已有/并行 Vk 设备上下文;输出粒子或体积场 buffer → graph **external import**。
- 失败或能力缺口 → 诚实登记 RD(编号随 G6.1 claim),**不**阻塞 G6.3/G6.4 硬门。
- 禁止:用 Taichi 替代主刚体、把确定性联网绑到粒子求解、在 CUDA 后端上另起「主物理」。

**实施留痕(2026-07-31)**:

- **TiRT FFI 边界** `src/rurix-rt/src/tirt.rs`(feature `taichi-tirt` 默认 off):13 符号运行时动态装载 taichi_c_api.dll(免导入库、免构建期 Taichi 依赖)+ `TiVulkanRuntimeInteropInfo` 注入并行 Vulkan 设备上下文(§4.E2 明示「挂已有渲染设备或并行设备上下文」两径,本臂取并行)+ `run_particles_spike` 全链(runtime → AOT module → kernel launch → `ti_export_vulkan_memory` 导出 VkBuffer)+ `TirtError` 六变体 fail-closed;`vk.rs` pub(crate) 三件套 cfg gate(既有面 0-byte);**U43 登记**(unsafe-audit/rurix-rt.md,沿 U26/U27/U31/U32 审计模式)。
- **spike demo** `apps/uc09-taichi-spike` + AOT 资产三件(`assets/particles.tcm` 3873B + `.sha256` 登记 f37398d3…0f05e4 + `gen_particles_aot.py` 生成脚本在树):host 腿 8 断言全绿;device 腿五断言(TiRT launch 成功 / NdArray 导出 VkBuffer 256B / graph external import 消费接线 byte_size=256 / readback 64/64 非零 / first_values=[1.0,2.5,4.0,5.5] 与 i*1.5+1.0 逐位相等)真跑一次即绿(RTX 4070 Ti,RURIX_REQUIRE_REAL=1),零 tirt 修复。
- **判定依据 §4.E3 成功最低判据四段闭合**:① TiRT Vulkan AOT kernel 在渲染设备上下文 launch 成功 ✓(并行设备上下文路径,§4.E2 明示允许);② NdArray 导出 VkBuffer ✓(256B = 64 粒子 × f32);③ graph external import 消费 ✓(只读引用、不拥有、不入 transient 池,copy 记录接线);④ 产出非零 buffer device 见证 ✓(readback 64/64 非零 + first_values 逐位)。**RD-042 未消费**(成功臂无需失败登记);§4.E4 三条禁止审计全绿(rurix-physics/rurix-render 零 taichi 引用、render `#![forbid(unsafe_code)]` 在位、零 CUDA 主物理路径)。
- **CI 步骤 92** `ci/taichi_vulkan_spike_smoke.py`(门 G-G6-6 软门):host 段六判据恒跑(AOT 资产核验〔在位非空 + 实测 sha256==登记 + gen 脚本在树〕/ feature `taichi-tirt` 双包默认 off cargo metadata 机验 / §4.E4 三条禁止审计 / U43 登记核验 / cargo test -p uc09-taichi-spike / host 腿 --json 8 断言全 true,反 YAML-only)+ device 段 gate real(RURIX_REQUIRE_REAL=1 且 RURIX_TAICHI_C_API_DLL provisioning 设位 → `--features taichi-tirt` 真跑;缺 DLL SKIP=dev-env degrade 退 0 不充绿)+ --selftest 红绿自检;`taichi_vulkan_spike_evidence_schema.json` + ci/check_schemas.py 注册 + pr-smoke.yml 步骤 92 真步骤落地(步骤 88~91 0-byte);evidence 两份如实(taichi_vulkan_spike_20260731T160536.json 真跑 ok=true / T160608.json host 态 SKIP ok=false);g6_budget.json 空壳维持(步骤 88~91 无 budget counter 体例,92 不加 counter)。

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
| v1.2 | 2026-07-31 | G6.1 完成(agent 完全自主 D-406 v2.0,Assisted-by: Kimi Code CLI (Kimi)):RFC-0017 伞形五章 Draft → D-409 第 1 轮对抗性评审(评审 provenance `kimi-cli:kimi-for-coding` 独立实例 ≠ 起草 `Kimi Code CLI (Kimi)`;claude 403 环境留痕,RFC-0015 先例)→ **Agent Approved 2026-07-31**(17 findings 全部采纳并修);状态行 + §1 G6.1 波次状态翻「已完成」,§2 冻结接口草案经 RFC Approved 字面冻结(实现期修订点以 RFC §9.1 disposition 为准);number_ledger v1.29 校准(RFC on_tree_max 17 / next_free 18)+ rfcs/README §5 台账行追加同步;§0/§2/§3/§4 既有裁决与冻结接口条文 0-byte 不动 |
| v1.3 | 2026-07-31 | G6.2 完成(agent 完全自主蜂群实施):`src/rurix-physics`(safe API:PhysicsWorld 固定步/BodyId·ShapeId generation arena/批插/并发查询规范序/ContactEvent 归一化有界 ring/SyncBudget)+ `src/rurix-physics-sys`(JoltC `29820043`+Jolt 5.3.0 `0373ec0d` vendor 内联 cmake 构建,unsafe-audit U33~U42,缺口处置 C-3 (c) 五处登记 VENDOR.md §3 + RFC-0017 §9.1 留痕)全量落地;A7 行为测试 7 项(确定性 N=100 逐位〔job_threads=1 钉住,MT 实测亦逐位不冻结〕/堆叠沉降/睡眠唤醒/批插不锁死主步〔阈值 16.667ms 标定〕/query 与 step 并发/ContactEvent 有界 drain/SyncBudget 饱和)全绿;CI 步骤 88 `ci/physics_core_smoke.py` host 恒跑全绿(cargo 三档 6/31/36 + §4.C4 审计四项)+ evidence;RFC-0017 v1.2 修订行(§4.C4 grep 门自相矛盾收窄:sys 消费收敛 world.rs 单模块,契约层判据 0-byte);number_ledger v1.30 校准(U on_tree_max 42 / next_free 43);全 workspace build/clippy/test 绿(1137 passed 零回归);§0/§2/§3/§4 既有裁决与冻结接口条文 0-byte 不动 |
| v1.4 | 2026-07-31 | G6.3 完成(agent 完全自主蜂群实施):合流桥 `src/rurix-physics/src/bridge/` 落地——`compose_transform_3x4`/`PhysicsBridge`(sync_frame 单向写 GpuScene/SyncBudget 截断/motion_hints 动态体 MV 提示/dirty_instances 供 AS 脏信号)+ `StreamingBridge`(insert_page/remove_page;`RemovalReceipt` 与 `PageKey` 绑定、移动语义不可 Clone、编译期不可伪造,先卸 body 再放页兑现 R-G6-4 类型纪律),`tests/bridge.rs` 七项行为测试全绿;sys 混批重排缺陷随波修复(实施期发现:Jolt `AddBodiesPrepare` 内部 QuickSort 在 >32 体混 broadphase 层批下非稳定重排致返回序/kind 登记/激活三错位,≤32 回退插入排序恒等故 G6.2 未暴露;修复 = prepare 前快照 `ids_orig` 三处按原始序配对,零新增 unsafe,SAFETY 注释增强;回归测试 `mixed_layer_batch_insert_order_preserved` 35 体混批先红后绿);合流 demo `apps/uc08-physics`(RFC-0017 Q-D 定名,UC-07 已被 ruridrop 占用)落地——host 15-pass 管线(VisBuffer/GI/VSM/TAA 全跑)+ physics/bridge_sync/mv 阶段,16 断言全绿(物理步 measured 8.05ms/96 步、transform_landed max_err 5.5e-5、MV 动态区 0.01224 vs 睡眠后 3.4e-6、TLAS rebuild 61 次/BLAS 零 refit),device 腿 RURIX_REQUIRE_REAL=1 真跑 exit 0(RTX 4070 Ti,changed_pixels=168),cargo test 11 绿;CI 步骤 89(`ci/physics_bridge_smoke.py`,G-G6-4)/ 91(`ci/uc08_physics_smoke.py`,G-G6-7)+ evidence schema 两件落地;number_ledger v1.31 校准(CI_step on_tree_max 87→91 / next_free 88→92,90 维持 G6.4 拟分配);RFC-0017 v1.3 修订行(§4.B4 RemovalReceipt 落点解释 + sys 混批缺陷留痕,正文冻结条文 0-byte);全 workspace 绿(cargo fmt --check / clippy --workspace --all-targets -D warnings 零警告 / cargo test --workspace 1166 passed 0 failed〔98 测试二进制;G6.2 基线 1137 + 本波 29〕);uc06-renderer 0-byte;§0/§2/§3/§4 既有裁决与冻结接口条文 0-byte 不动 |
| v1.5 | 2026-07-31 | G6.4 完成(agent 完全自主实施,T1~T11 兑现):Rapier 第二后端 `src/rurix-physics/src/rapier.rs` 落地(rapier3d =0.33.0 pin optional 默认 off,纯 Rust 零 CMake CI 面;方法面镜像 `SysWorld` token u64 出入,safe 层自维护机制两后端共享零分叉;`src/world.rs` 后端枚举三档分派,Rapier 变体 Box 化;`tests/behavior.rs` 双后端循环测试名逐字未变 + `tests/api.rs` cfg 翻转;`#![forbid(unsafe_code)]` 维持,零新 unsafe);rapier.rs 两缺陷随波修复(集成轮探针定位先红后绿:① 宽相 pair 注册闭环——BVH `AddPair` 仅向本次 `update` 调用方报告而窄相 `register_pairs` crate 私有,手动 update 吞事件致初始即相交 pair〔缝 ≤ 0.002m〕永不注册窄相穿透,修复 = `pending_reinsert` 下一拍 step 开头先删 BVH 再交 `pipeline.step` 自闭环重建;② 睡眠接触语义对齐 Jolt——睡眠即接触约束移除,弃 `has_any_active_contact` 口径〔入睡对 solver_contacts 空会误发 End〕,改几何接触点 `dist ≤ 0` 判定 + 入睡对差分发 End);对拍门 `tests/parity.rs` §4.D3 全判据(变换容差逐拍逐体 / Begin·End 接触集合重叠 ≥0.99 / 相位等价类 / 各后端进程内重放逐位,禁跨引擎逐位),阈值实测标定 pos 0.82m / rot 93° / 重叠 ≥0.99(= 实测 max 0.541314125m / 61.774520874° × 1.5;跨后端偏差为确定性常量,6 次跨进程复跑逐位恒定方差零,jolt hash bfa449a7a5515449 / rapier hash 7e7ddda2f21e23d0,evidence `physics_rapier_parity_20260731T120019.json`);CI 步骤 90 `ci/physics_rapier_parity_smoke.py` 七判据(默认 off 核验 / rapier-only 无 CMake 路径 / cargo test 两腿 / rapier 两 tier clippy / parity 进程级重放一致 / 文档口径 grep / evidence 过 schema)+ `physics_rapier_parity_evidence_schema.json`(单 schema 双形态)+ ci/check_schemas.py 注册 + pr-smoke.yml 真步骤替换占位注释;步骤 88 v1.4 收窄(§4.C4 grep 判据:rapier3d 原生类型名仅许 `src/rapier.rs` 单模块,RFC-0017 v1.4 修订行 + §9.1 G6.4 留痕段,冻结条文 0-byte);number_ledger v1.32 校准(CI 步骤 90 materialize,CI_step on_tree_max 91 / next_free 92 不变)+ CI_GATES v1.3 步骤 90 回填;四档矩阵(default〔=jolt〕/ `--no-default-features` / rapier-only / `--features rapier`)test+clippy 全绿;全 workspace 绿(cargo fmt --check / clippy --workspace --all-targets -D warnings 零警告 / cargo test --workspace 1166 passed 0 failed〔99 测试二进制;G6.3 基线 1166 passed/98 二进制,+1 二进制 = parity.rs 默认档 cfg 空壳 0 测试,passed 数不变〕;rurix-physics 四档 test default 54 / no-default 41 / rapier-only 49 / dual 55 + 四档 clippy 全绿;步骤 88/89/90/91 全段绿——90 纯 host 七判据 PASS + --selftest 红绿自检 PASS,89/91 device 腿 RURIX_REQUIRE_REAL=1 真跑 RTX 4070 Ti changed=168 无 degrade,91 host 16 断言全 true 物理步 measured 8.604ms 留证不进硬门);不替换 Jolt 生产默认、不做性能宣称(P-09);uc06-renderer 0-byte;§0/§2/§3/§4 既有裁决与冻结接口条文 0-byte 不动 |
| v1.6 | 2026-07-31 | G6.5 完成(agent 完全自主实施):Taichi Vulkan AOT 特效 spike **成功臂**落地——技术侦查定案 `evidence/taichi_spike_recon_20260731.md`(UC-09 编号自由无撞号 / taichi 1.7.4 C API 面 13 符号核实〔interop/runtime/module/ndarray/`ti_export_vulkan_memory`〕/ `ti.aot.Module(ti.vulkan)` AOT 链通真跑 / vk 设备参数与 `TiVulkanRuntimeInteropInfo` 兼容);AOT 资产三件入仓(`apps/uc09-taichi-spike/assets/particles.tcm` 3873B + sha256 登记 f37398d3…0f05e4 + `gen_particles_aot.py` 可复跑);TiRT FFI 边界 `src/rurix-rt/src/tirt.rs`(feature `taichi-tirt` 默认 off 依赖 `vulkan`,零新外部依赖;13 符号运行时动态装载 `taichi_c_api.dll`〔`RURIX_TAICHI_C_API_DLL` 绝对路径,免导入库/免构建期 Taichi 依赖,未设/缺符号确定性 `TirtError` 六变体 fail-closed〕+ `TiVulkanRuntimeInteropInfo` 注入并行 Vk 设备上下文〔§4.E2 明示两径之一,非能力缺口〕+ `run_particles_spike` 全链〔runtime→AOT module→kernel launch→`ti_export_vulkan_memory` 导出 VkBuffer 256B〕+ 同 device `vkCmdCopyBuffer` readback;vk.rs pub(crate) 三件套 cfg gate 既有面 0-byte;**U43 登记** unsafe-audit/rurix-rt.md,沿 U26/U32 审计模式);spike demo `apps/uc09-taichi-spike`(host 8 断言〔资产核验/graph import 只读引用不入 transient 池/copy 记录接线〕+ device 五断言〔launch 成功/导出 VkBuffer 256B/graph external import 消费接线 byte_size=256/readback 64/64 非零/first_values=[1.0,2.5,4.0,5.5] 与 i*1.5+1.0 逐位相等〕,RTX 4070 Ti `RURIX_REQUIRE_REAL=1` 一次真跑即绿零 tirt 修复);§4.E3 成功最低判据**四段闭合**,**RD-042 未消费**;§4.E4 三条禁止机器审计全绿(rurix-physics/rurix-render 零 taichi 引用、render `#![forbid(unsafe_code)]` 在位、零 CUDA 主物理路径);CI 步骤 92 `ci/taichi_vulkan_spike_smoke.py`(门 G-G6-6 软门:host 六判据恒跑 + device gate real〔缺 DLL SKIP=dev-env degrade 退 0 不充绿〕+ --selftest 红绿自检)+ `taichi_vulkan_spike_evidence_schema.json` + check_schemas 注册 + pr-smoke.yml 真步骤(步骤 88~91 0-byte);RFC-0017 v1.5 修订行(§9.1 G6.5 留痕段,§4.E 冻结条文 0-byte 未漂移)+ CI_GATES v1.4 步骤 92 回填 + number_ledger v1.33 校准(CI_step on_tree_max 91→92/next_free 93,U on_tree_max 42→43/next_free 44);g6_budget.json 空壳维持(步骤 88~91 无 counter 体例,92 不加);全量回归绿(cargo fmt --check / clippy --workspace --all-targets -D warnings 零警告 / cargo test --workspace **1174 passed 0 failed〔100 测试二进制;G6.4 基线 1166/99,+8 = uc09 测试、+1 二进制〕** / 步骤 88~91 全段绿〔89/91 device 腿 RTX 4070 Ti changed=168 与 v1.5 逐位一致〕+ 步骤 92 host+device 真跑全绿 / check_{number_ledger,schemas,structure,guardrails,contribution} / trace_matrix --check 278/278 / budget_eval 96 pass 全 PASS);零新 RXS/RD/SG/error-code/budget counter;uc06-renderer 0-byte;§0/§2/§3/§4 既有裁决与冻结接口条文 0-byte 不动 |
