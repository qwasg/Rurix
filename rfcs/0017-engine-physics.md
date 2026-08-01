# RFC-0017 — G6 渲染物理双轨期伞形:rurix-physics 物理库边界 / 渲染同步契约 / FFI 与 unsafe 纪律 / Rapier 快路径 / Taichi Vulkan AOT 特效副轨

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0017(4 位制,编号永不复用,10 §9.5) |
| 标题 | G6 渲染物理双轨期伞形:rurix-physics 物理库边界 / 渲染同步契约 / FFI 与 unsafe 纪律 / Rapier 快路径 / Taichi Vulkan AOT 特效副轨 |
| 档位 | **Full RFC**(伞形,G4 RFC-0015 / G5 RFC-0016 单伞形先例:一份 RFC 承载全期各面,一次对抗性评审、一次 Approved 合入即满足各面「RFC Approved 前置」;触及 FFI ABI 与 unsafe 边界高敏面——Jolt C++ → C ABI → Rust 绑定 + U33 起 unsafe 审计面,AGENTS 硬规则 5;判档争议向上取严 = Full,硬规则 8) |
| 状态 | **Agent Approved**(2026-07-31;§9.1 对抗性评审〔评审 provenance `kimi-cli:kimi-for-coding` 独立实例 ≠ 起草 provenance `Kimi Code CLI (Kimi)`,三镜头 correctness/redline/implementability,D-409〕完成,17 findings 逐条 disposition〔2 blocker + 11 major + 4 minor,全部采纳并修、无驳回〕,先于任何实现合入,G-G6-2) |
| 承接里程碑 | G6.1(验收门 G-G6-2;下游 G6.2~G6.5 各面「RFC Approved 前置」由本伞形一次承载,G6_CONTRACT `rfc_required: RFC-0017` 单号伞形) |
| 关联条款 | **预期零新 spec 条款**(物理为引擎库,06 §8.3「库不进语言」,G5 先例);条件消费路径见 §5(合入时 number_ledger 实际 next_free 顺位,与 RD-038 兑现臂同源先合先得) |
| 依据决策 | D-406 v2.0(agent 完全自主)· D-409(对抗性评审,评审 provenance ≠ 起草)· 06 §8.3(render graph/ECS/物理为库不进语言)· 04 P-01(strict-only)/ P-09(证据压过进度:性能数字 measured 写 evidence 不进硬门)/ P-11(推导单源)/ P-12(克制压过完整性)· 13 D-130(窗口/输入不进语言红线)· RD-034(DXIL RT blocked 维持,本期物理不依赖 DXIL)· [G6_PLAN](../milestones/g6/G6_PLAN.md) §0.1 择优锁定(Jolt 主物理 / Rapier 快路径 / Taichi Vulkan AOT 特效副轨 / Newton 系研究隔离 / GPU 主刚体否决) |
| Provenance | `Assisted-by: Kimi Code CLI (Kimi)`(起草)。agent 自主决策;批准前置 = §9.1 对抗性评审完成 |
| Agent 批准 | **Agent Approved 2026-07-31**——§9.1 对抗性评审(评审 provenance `kimi-cli:kimi-for-coding` 独立实例 ≠ 起草 `Kimi Code CLI (Kimi)`,三镜头,D-409)完成,17 findings(2 blocker + 11 major + 4 minor)全部采纳并修、正文实改逐条 disposition(§9.1),先于任何实现合入(G-G6-2) |
| 对抗性评审 | **已完成 第 1 轮 2026-07-31**——见 §9.1;评审 provenance `kimi-cli:kimi-for-coding`(独立 kimi-cli 实例,独立进程/零共享上下文)≠ 起草 `Kimi Code CLI (Kimi)`(硬规则 2 可机验,`ci/check_contribution.py` 规则 4);首选 claude 跨模型评审 403 不可得,本轮为跨工具/同模型族评审,偏差如实登记 §9.1 环境留痕(RFC-0015 §9.1 先例) |

---

## 1. 摘要

本 RFC 是 G6 渲染物理双轨期的**单伞形 Full RFC**(G4 RFC-0015 / G5 RFC-0016 单伞形先例:一次对抗性评审、一次 Approved 合入即满足各面「RFC Approved 前置」)。G6 把「rurix 引擎拥有生产级物理主线」自选型调研结论(G6_PLAN §0.1 已锁定)推进到 measured 工程事实——上游事实源 = **[G6_PLAN](../milestones/g6/G6_PLAN.md) v1.1** + [G6_CONTRACT](../milestones/g6/G6_CONTRACT.md) v1.0。五章:

- **章 A 物理库边界**——新 crate `src/rurix-physics`:`PhysicsWorld` 固定步 `step(dt_fixed)`、`BodyId`/`ShapeId` 不透明句柄、`BodyDesc`/`PhysicsTransform`、`ContactEvent` 有界队列(step 边界归一化排序)、`QueryRay` step 外并发查询(结果规范序)、`SyncBudget` 每帧重置;Jolt 生产默认(世界/层/睡眠/批插体/CCD/job 系统适配)。
- **章 B 渲染同步契约**——G6_PLAN §2.2/§2.3 字面化:单向事实源(物理 → `GpuScene` 变换脏集,渲染不回写)、查询并行、流送同构(页驻留驱动 body 批插移除,`RemovalReceipt` 先卸后放)、特效隔离、库不进语言五条纪律 + G5 冻结面 0-byte 边界。
- **章 C FFI 与 unsafe 纪律**——R-G6-1 裁决落地:自维护薄 FFI 子 crate 绑定 JoltC C API(rolt/jolt-rust 停滞否决;JoltC 缺口审计为 PR-A 前置);unsafe 集中 `rurix-physics-sys`,U33 起续号;`rurix-physics` / `rurix-render` 维持 `#![forbid(unsafe_code)]`。
- **章 D Rapier 快路径**——feature `rapier`(默认 off)同 `PhysicsWorld` 抽象第二后端;同场景 host 对拍 = 变换/接触集合**容差**断言(判据形态冻结、阈值数字经实测标定,非跨引擎逐位,R-G6-2);CI 无 CMake 路径可跑。
- **章 E Taichi Vulkan AOT 特效副轨**——可选交付:AOT 模块 + TiRT 挂已有/并行 Vk 设备上下文,粒子/体积场 buffer 经 graph external import(buffer TiRT 拥有、graph 只读引用);失败/能力缺口诚实登记 RD(自 RD-042 顺位),不阻塞硬门。

```
streaming ──batch_body_prep──► rurix-physics (Jolt default / Rapier feature)
                                    │ PhysicsTransform / ContactEvent / MV hints
                                    ▼
                               GpuScene ──────────────────► rurix-render ──► render_exec (Vulkan)
                                    ▲                                   ▲
              Taichi Vulkan AOT (可选,章 E) ── particle/volume ── graph external import
主物理 ──X──► GPU sim on render queue(禁止:不与 VisBuffer/VSM/GI/RT 抢车道)
```

**预期零新语言语义条款**(06 §8.3):物理为引擎库,spec 面零修订承诺;条件消费路径保条款先行纪律(§5)。

## 2. 动机

G5 close-out([G5_CONTRACT](../milestones/g5/G5_CONTRACT.md) §8.1)已交付原生渲染器八面(graph/geometry/shadow/gi/rt/material/streaming/temporal),`GpuScene`/`PageRequest`/时域 MV/AS 管理器为本期合流消费面;**物理模块在仓库零存在**(`src/` 无 physics crate,2026-07-31 核实)。02 §2 U5(引擎旗舰用例)的完整形态要求「渲染 × 物理」双轨:G6_PLAN §0.1 跨域择优已锁定——**Jolt Physics** 主物理(AAA 刚体多核/睡眠/批插体/并发查询,MIT,与 Vulkan 渲染正交不占车道)、**Rapier** 快路径(纯 Rust,CI 无 CMake 路径)、**Taichi Vulkan AOT** 特效副轨(MPM/连续体粒子·体积场,不进刚体求解)、Newton/Genesis/MuJoCo Warp 研究隔离、GPU 主刚体否决(游戏刚体预算走 CPU 多核,Vulkan 车道留给 G5 效果面)。

**为何需要 Full RFC(而非 Direct/Mini)**:触及 FFI ABI(Jolt C++ → JoltC C ABI → Rust,#[repr(C)] POD 镜像 / 回调 user_data 纪律 / 所有权跨边界)与 unsafe 边界(U33 起 unsafe-audit 新条目)高敏面(AGENTS 硬规则 5);判档争议向上取严 = Full(硬规则 8)。**为何伞形单 RFC**:五章共享一套跨章一致性约定(§4.0:crate 布局与 unsafe 分布 / 后端正交 / 冻结接口 / 确定性口径 / 证据口径 / 编号消费 / feature 总裁决),一次对抗性评审覆盖全文(D-409),各面失败测试先行判据不变(RFC 合入时点各面 CI 脚本与 crate 在 main 不存在 = RED,§6.1);物理为引擎库预期零新语言语义条款,伞形 Full 一并承载判档(G6_CONTRACT §7 ④)。

## 3. 指导级解释(宿主视角)

宿主(引擎 app,如 `apps/uc08-physics`)每帧的物理使用形态:

```rust
// 装配期:世界 + 静态场景批插入(流送页驻留驱动)
let mut world = PhysicsWorld::new(WorldDesc { backend: BackendKind::Jolt, gravity: [0.0, -9.81, 0.0], ..Default::default() })?;
let bodies = world.add_bodies_batch(&descs)?;          // 批插体不锁死主步(Jolt AddBodiesPrepare/Finalize 映射)

// 帧循环:accumulator 在宿主,库只收固定步
acc += dt_frame;
while acc >= DT_FIXED { world.step(DT_FIXED)?; acc -= DT_FIXED; }   // 固定步确定性:同二进制同平台重放逐位一致

// 步后:单向桥写 GpuScene 变换脏集(渲染只读;渲染器不回写物理)
for (inst, body) in &dyn_bodies {
    let t = world.transform(*body)?;                   // PhysicsTransform { translation, quat xyzw }
    bridge.push(*inst, t);                             // → GpuScene::update_transform + MV hints(静态/睡眠体零 MV)
}
gpu_scene.flush_dirty();

// 同帧 step 外交替期:角色/AI/拾取走并发 query(多线程并发,结果规范序;与 render_exec 同帧)
let hit = world.cast_ray(&QueryRay { origin, dir, t_min: 0.0, t_max: 50.0, layer_mask: PICK_MASK })?;

// 事件:每帧有界 drain(step 边界归一化序列),SyncBudget 每帧重置防物理→渲染写爆
for ev in world.drain_contacts(&mut budget) { /* Begin/Persist/End */ }
```

要点:`PhysicsWorld` 是 safe API(`rurix-physics` crate `forbid(unsafe_code)`);宿主只握 `BodyId`/`ShapeId` 不透明句柄,永不见原生 Jolt/Rapier 指针;物理世界状态在库内,渲染侧历史(TAA/VSM/GI)仍为 external import,两相分属。

## 4. 参考级设计

### 4.0 跨章一致性约定(汇装层裁决,五章共同事实源)

1. **crate 布局与 unsafe 分布**:`src/rurix-physics` = safe 公共 API + 同步 bridge + Rapier feature,`#![forbid(unsafe_code)]`;`src/rurix-physics-sys` = JoltC FFI 边界(unsafe 唯一集中地,`[lints.rust] unsafe_code = "allow"` + `undocumented_unsafe_blocks = "deny"`,镜像 rurix-rt 豁免模式,unsafe-audit U33 起);`src/rurix-render` 维持 `#![forbid(unsafe_code)]` 0-byte([lib.rs](../src/rurix-render/src/lib.rs) line 19 现状)。渲染器不持有原生 Jolt/Rapier 指针(代码审计判据 §4.C4)。
2. **后端正交纪律**:主物理 = CPU 库,与 compute 双后端(`BackendKind::Cuda | Vulkan`,[backend.rs](../src/rurix-rt/src/backend.rs))及渲染 Vulkan 车道**正交**;禁 GPU 主刚体(PhysX CUDA 刚体 / wgrapier 生产依赖 / Warp-Newton 主环);禁物理 sim 上渲染队列与 VisBuffer/VSM/GI/RT 抢车道(G6_PLAN §0.2/§0.4,R-G6-5)。特效副轨若上 GPU 仅 Vulkan AOT(章 E),不引入第二套 CUDA 物理 runtime 作主环。
3. **冻结接口**:§4.A/§4.B 接口面 = [G6_PLAN](../milestones/g6/G6_PLAN.md) §2.1/§2.2 字面化(评审修订点以 §9.1 disposition 为准,含 C-2/C-4/R-3/R-4/I-2 收窄);本 RFC Approved 后**字面冻结**,实现 PR 不得漂移;修订只能经 RFC 修订行 + §9.1 留痕。`PhysicsError` 枚举成员不属冻结面(§5.1,I-7 评审修订)。
4. **确定性口径**(C-1 评审修订,分两档):(a) **默认口径 = 同二进制同平台重放逐位一致**——G-G6-3 烟测判据(§4.A7,N=100 步全量 `PhysicsTransform` 逐位相等);(b) **可选口径 = 跨平台 bit 级一致**——Jolt 上游提供 `CROSS_PLATFORM_DETERMINISTIC` CMake 选项 + 统一编译标志可达(评审核实的上游能力),是否启用随 G6.2 实现 PR 定案并登记 evidence。跨引擎(Jolt ↔ Rapier)**不承诺逐位**——对拍走容差/集合不变量(§4.D3,R-G6-2)。
5. **证据口径**:性能数字(物理步耗时 / 同步桥每帧写量)measured_local 写 evidence,数字不进硬门(P-09;G6_CONTRACT out_of_scope `perf_budget_hard_gates`);budget counter/entries 不预造,随实现 PR 与 `ci/budget_eval.py` evaluator 分支同落。
6. **编号消费口径**(number_ledger v1.28 `reserved_in_flight[G6]`):RXS 预期零消费(确需按合入时 ledger 实际 next_free 顺位,与 RD-038 兑现臂同源先合先得、后合校准,未消费不占号)/ CI 步骤 88 起(数量随实现回填不预占,多余号作废声明 burned)/ RD-042 起 / U33 起 / RX_error 预期零新码 / MR-0012 按需 / SG 与共享 D 段零消费(SG-010 软保留维持,D-408 earmark 不动)。
7. **feature 总裁决**(R-2 评审修订:删「唯一新 feature」绝对化措辞):新增**功能开关** feature 仅 `rapier`(默认 off,cargo metadata 可机验);`jolt` 为**默认后端构建 gate**(非功能面开关——隔离 C++/CMake 构建依赖,默认 on,经 `rurix-physics-sys` vendor 构建 JoltC;C++ 工具链 = 构建面依赖如实登记,CI/host 构建环境画像随 G6.2 PR-A 记录,I-1);`--no-default-features` 构建零 C++ 依赖绿(纯 API 骨架 + 无后端编译,`PhysicsWorld::new(Jolt)` → 确定性 `Err`「backend not compiled」,P-01 不静默回退)。三档构建矩阵:no-default(零 C++ 依赖,纯 Rust,恒绿)/ default(= jolt,需 C++ 工具链,CI provisioning 责任随 PR-A 落实)/ default+rapier(双后端对拍;G-G6-5 无 CMake 路径 = rapier-only 构建)。
8. **每 PR 不变量**:既有步骤 41~87 判据 0-byte 只增(步骤 69 blocked 探针恒跑 RD-034 / 步骤 70 永久 gap / 步骤 84~86 RD-038 分波探针按自身轨道);LF byte-exact;`RURIX_REQUIRE_REAL=1` 贯穿 device 段(缺 provisioning SKIP = dev-env degrade,mock/SKIP 不充绿);evidence/ 只增不删;G5 冻结面 0-byte(§4.B6)。

### 4.A 物理库边界章(G6.2;验收门 G-G6-3;CI 步骤 88 拟)

#### A1. 世界与固定步(→ 冻结接口 §4.0-3)

- `PhysicsWorld::new(WorldDesc) -> Result<PhysicsWorld, PhysicsError>`:`WorldDesc { backend: BackendKind /* Jolt(default) | Rapier(feature `rapier`) */, gravity: [f32; 3], layer_count: u32, max_bodies: u32, job_threads: Option<u32> }`;`PhysicsError` = 库层错误枚举(零新 RX 码,§5.1)。
- `step(&mut self, dt_fixed: f32) -> Result<StepStats, PhysicsError>`:**固定步**,accumulator 在宿主(§3);库内拒绝变步长(`dt_fixed` 与 world 配置不一致 → 确定性 `Err`,P-01)。`StepStats { active_bodies, slept_this_step, contacts_emitted, contacts_dropped, step_time }`(step_time 仅供 evidence,不进硬门)。
- 后端枚举 `BackendKind::Jolt`(default)/ `BackendKind::Rapier`(feature `rapier`);未编译后端 → `Err(BackendNotCompiled)`(§4.0-7)。

#### A2. 句柄、描述与变换(→ 冻结接口 §4.0-3)

- `BodyId(u64)` / `ShapeId(u64)`:**不透明句柄**(index 32b + generation 32b;FFI 边界只过 u64,不过原生指针,§4.C3);禁止渲染器持有原生 Jolt/Rapier 指针(审计 §4.C4)。**generation 纪律**(I-6 评审修订):body 移除后槽位复用时 generation 单调递增;32b generation 空间耗尽的槽位**退休不再分配**(回绕复活路径类型面消灭);world 生命周期内 index 池耗尽 → `Err(PoolExhausted)`(P-01)。
- `BodyDesc { kind: Static | Kinematic | Dynamic, shape: ShapeDesc, layer: u32, mass_props: MassProps, ccd: bool }`;`ShapeDesc = Sphere | Box | Capsule | ConvexHull | StaticMesh`(StaticMesh 仅 Static 体,动态 mesh 不支持 → `Err`,P-01)。
- `PhysicsTransform { translation: [f32; 3], rotation: [f32; 4] /* xyzw quat */ }`——与 `GpuScene` 实例 3×4 变换的**唯一桥接输入**(bridge 合成 3×4,§4.B2);库内不出现 4×4/3×4 矩阵类型(单源,P-11)。

#### A3. Jolt 面映射(生产默认)

- 世界/层:Jolt `PhysicsSystem` + `BroadPhaseLayerInterface` / `ObjectLayerPairFilter` 层对过滤;`layer_count` 上限随 `WorldDesc` 定(Jolt ObjectLayer 位宽约束,绑 `object-layer-u32` 与否随 sys crate 定案,实现 PR 登记)。
- 睡眠:Jolt 内建睡眠(默认开);睡眠体零 MV(§4.B3)、零变换脏写(bridge 跳过)。
- 批插体:`add_bodies_batch(&[BodyDesc]) -> Result<Vec<BodyId>, PhysicsError>` 映射 Jolt `BodyInterface::AddBodiesPrepare/AddBodiesFinalize`(批插不锁死主步;量化判据与单测锚 §4.A7)。
- CCD:`ccd: bool` 映射 Jolt `MotionQuality`(Discrete/LinearCast)+ `PhysicsSettings` 穿透恢复参数(pin 随实现 PR)。
- job 系统:Jolt `JobSystem` 适配层(Q-E)——默认实现 = 库内线程池(线程数 = `job_threads` 或可用并行度),接口允许宿主注入外部池;engine 侧通用线程池仓内现不存在(2026-07-31 核实 `src/rurix-engine` 仅 ffi.rs/lib.rs),「接宿主线程池」落实为**适配抽象 + 默认自带池**,宿主池接入为后续增量,不阻塞 G6.2。

#### A4. 查询面(并发 query,Jolt 路径硬需求;C-2/C-4/I-2 评审修订收窄)

- `QueryRay { origin: [f32; 3], dir: [f32; 3], t_min: f32, t_max: f32, layer_mask: u64 }`;`QueryHit { body: BodyId, t: f32, position, normal, shape: ShapeId }`;shape cast / overlap 同型(`QueryShape` + `OverlapHit`)。
- **相位纪律(Q-B)**:cast 类查询(ray/shape cast/overlap)= **step 外并发调用**(G6_PLAN §2.1 字面「支持 step 外并发调用(Jolt 路径硬需求)」——Jolt `PhysicsSystem::Update` 期间禁读写 body,cast 查询天然属 step 外交替期;多线程 query 之间全并发,Jolt `NarrowPhaseQuery` 只读路径线程安全);step 相位内直读世界的路径**类型面不暴露**(不靠文档约定)。
- **变换读(上一拍快照,仅数组)**:渲染同帧读上一拍变换 = step 结束边界提交的变换数组副本(active 动态/运动体 `PhysicsTransform` 浅拷贝,内存增量 = 体数 × 32B 级,预算可忽略);**不复制加速结构**——起草稿「快照双缓冲(含查询加速结构)」方案经评审否决为过度设计(C-4/I-2):cast 查询一律 step 外相位,加速结构无跨相位读需求。
- **顺序确定性(C-2)**:Jolt `NarrowPhaseQuery` 结果一致但**返回顺序可变**、`BroadPhaseQuery` 非确定——对外契约:cast 结果返回前按 `(t, BodyId)` 规范序排序(全命中模式)或取最近命中(单命中模式),**排序后序列 = 确定性面**;`BroadPhaseQuery` 不作默认面(确需时自定义 collector + 同序排序,实现 PR 登记)。
- 与 `render_exec` 同帧可读上一拍变换(G6_PLAN §2.3-2);query 吞吐受 `SyncBudget.max_query_casts` 每帧约束(§4.A6)。**「query 与 step 并发烟测」机验判据**(C-4):step 外 ≥2 线程并发 cast 结果(排序后)与单线程一致 + step 相位内变换快照读与 step 完成后一致——真并发经线程注入断言,非相位串行伪装。

#### A5. 事件面(接触 Begin/Persist/End;C-2 评审修订)

- `ContactEvent { a: BodyId, b: BodyId, phase: Begin | Persist | End, contact_point, normal, impulse }`;库内有界 ring(容量随 `WorldDesc`),`drain_contacts(&mut self, budget: &mut SyncBudget) -> impl Iterator<Item = ContactEvent>` 每帧 drain。
- **事件序列确定性**:Jolt `ContactListener` 回调**多线程触发、顺序非确定**——库内在 **step 结束边界归一化**:收集本步全部接触回调,按 `(min(a,b), max(a,b), phase)` 规范序排序去重后入 ring;事件序列确定性 = 归一化后序列语义(回放/对拍只面向归一化序列,§4.D3)。
- 溢出语义:ring 满 → 确定性丢弃最旧(**归一化序列上定义**)+ `StepStats.contacts_dropped` 计数上报(不 panic,P-01;计数进 evidence 不进硬门)。

#### A6. SyncBudget(每帧重置,防物理→渲染写爆)

- `SyncBudget { max_body_writes: u32, max_contact_events: u32, max_query_casts: u32 }`;宿主每帧构造(重置),bridge/query/drain 共享消耗;任一项耗尽 → 对应面确定性截断(余量归零即停)+ 饱和计数上报(P-01 不 panic;R-G6-4 竞态预算的一部分)。

#### A7. 确定性与 host 单测锚(G-G6-3)

- **固定步确定性烟测**:同输入序列(初始变换 + 外力脚本)同二进制同平台重放 N=100 步,`PhysicsTransform` 全量逐位相等(§4.0-4(a) 口径;跨平台选项 (b) 启用与否随实现 PR 登记)。
- host 单测清单(随 G6.2 PR,`tests/` in-crate):堆叠沉降(箱塔静置收敛)/ 睡眠唤醒(静置入睡 + 冲量唤醒)/ **批插体不锁死主步**(量化判据,C-6 评审修订:prepare 在 step 外交替期执行、finalize 单点提交,批插期间主步延迟 ≤ 1 帧)/ query 与 step 并发烟测(§4.A4 机验判据)/ ContactEvent 有界 drain(归一化排序 + 溢出计数)/ SyncBudget 每帧重置与饱和截断。

### 4.B 渲染同步契约章(G6.3;验收门 G-G6-4;CI 步骤 89 拟)

#### B1. 五条纪律(G6_PLAN §2.3 字面化,→ 冻结接口 §4.0-3)

1. **单向事实源**:动态/运动学变换仅由 `rurix-physics` → `GpuScene`;渲染器不回写物理(代码审计 + 类型面:bridge 只持 `&mut GpuScene` 的变换写口,物理 API 不接受渲染侧输入)。
2. **查询并行**:角色/AI/拾取走并发 query(§4.A4);与 `render_exec` 同帧可读上一拍变换。
3. **流送同构**:几何页驻留/卸载驱动 body 批插入/移除(对齐 Jolt batch insert + G5 `PageRequest`);物理只订阅「页驻留/卸载」通知,不重新实现 `StreamingBudget` 计量。
4. **特效隔离**:Taichi AOT 只产出粒子/体积场,经 external import 进 graph;不进刚体求解、不承担确定性联网(章 E)。
5. **库不进语言**:同 06 §8.3;FFI 集中在 `rurix-physics-sys`;`rurix-render` 维持 `#![forbid(unsafe_code)]`。

#### B2. 变换桥(PhysicsTransform → GpuScene)

- bridge 将 `PhysicsTransform`(§4.A2)合成 3×4 行主变换,调 `GpuScene::update_transform(instance_id, transform)`([gpu_scene.rs](../src/rurix-render/src/geometry/gpu_scene.rs) line 204)+ 帧末 `flush_dirty()`(line 237);渲染只读消费。
- 只写面 = `GpuScene` 变换脏集 + 可选 MV 缓冲(§4.B3);静态/睡眠体零脏写(bridge 按 `StepStats`/body 状态跳过)。

#### B3. 动态体 MV 供给(时域底座消费侧;R-4 评审修订)

- 动态体:bridge 记上一拍 `PhysicsTransform`,与当前拍差分供给 motion 提示;**静态/睡眠体零 MV**。
- MV 缓冲为**可选**;**缓冲格式(2D 屏幕空间 vs 3D 世界空间 motion vector)不列入冻结接口**——格式由 `temporal/` 消费侧与实现 PR 共同标定,不修改 G5 时域底座数据结构;存在时经 graph external import 供 [src/rurix-render/src/temporal/](../src/rurix-render/src/temporal/) 消费;**禁效果 pass 私写重投影维持**(RFC-0016 §4.H 口径 0-byte 延续);物理不改 TAA/TSR 内部状态。

#### B4. 流送 ↔ body(页驻留驱动,同帧预算;R-3 评审修订)

- 页驻留(`PageRequest` 满足,[graph/types.rs](../src/rurix-render/src/graph/types.rs) line 355)→ `add_bodies_batch` 批插入;页卸载 → `remove_bodies_batch`;与 `PageRequest` 同帧预算(同步桥写量计入 `SyncBudget.max_body_writes`)。
- **R-G6-4 竞态纪律(文档纪律 → 类型/所有权纪律)**:**先卸 body 再放页**——`remove_bodies_batch` 返回 `RemovalReceipt`(移除完成凭据,与页 id 绑定),流送层**须持 receipt 方能释放对应几何页**(无 receipt 的放页路径编译期不可构造 + 运行时断言双保险);形状数据所有权:静态 mesh 形状引用流送页驻留几何,卸载前 body 必已移除;单测注入「卸载与物理写并发」时序脚本(驻留 → 批插 → 卸载并发物理写 → 断言无悬挂 body 引用已释放页形状)。

#### B5. AS 分级(TLAS/BLAS)

- 变换脏实例走 G5 既有 refit 决策树([src/rurix-render/src/rt/](../src/rurix-render/src/rt/) AS 管理器),**不新建加速结构所有者**;物理只提供「变换脏」信号(与 §4.B2 脏集同帧同源),不直接触碰 AS API。

#### B6. G5 冻结面 0-byte 边界(机器可核)

- **不改**:`MaterialClosure` 32B 定长 / VisBuffer 位格式 / `Barrier` EB 三轴 / `PageRequest` 字段布局(G5 §2;G6_PLAN §2.2);碰撞形状与材质槽解耦(物理不改材质闭合)。
- 跨帧分属:物理世界状态在库内;渲染侧历史(TAA/VSM/GI)仍为 external import,与物理世界分属。

### 4.C FFI 与 unsafe 纪律章(G6.2 横切;G-G6-3 代码审计面;unsafe-audit U33 起)

#### C1. R-G6-1 裁决:Jolt 绑定选型(2026-07-31 调研 + 评审核实)

| 候选 | 现状(调研事实) | 裁决 |
|---|---|---|
| `rolt` / `jolt-rust`(SecondHalfGames) | 最新 release `0.3.1+Jolt-5.0.0`(2024-05-19);自述「early work in progress」「best-effort and incomplete」;pin Jolt 5.0(上游已至 5.5 系,2026-07-31 评审核实);下载量 ~212/月 | **否决**(停滞 2 年 + 不完备 + 版本滞后;R-G6-1 原文「未臻生产完备」坐实) |
| zig-gamedev `zphysics` 内嵌 JoltC | Zig 生态自用 C API,非独立维护 C ABI 面 | **否决**(非 Rust 可直消费形态) |
| amer-koleci `joltc`(.NET 生态) | JoltPhysicsSharp / Unity 绑定生产使用,Jolt 5.3 跟进 | **候选 C ABI 面**(缺口处置备选,见下) |
| SecondHalfGames `JoltC` | 独立 C wrapper,目标明示「消除主要 UB 来源」;自述 work-in-progress(「bindings contain functions that we've needed」),**完整覆盖面不承诺**(C-3) | **首选 C ABI 面**(消 UB 口径与仓内审计纪律最契合;覆盖面缺口经前置审计处置) |

**裁决**:自维护薄 FFI 子 crate `src/rurix-physics-sys`,绑定 **JoltC C API**(首选 SecondHalfGames/JoltC 消 UB 口径)。两个前置硬任务(评审修订):

- **JoltC 缺口审计(C-3,G6.2 PR-A 前置,不得留到 PR-A 中段)**:contact listener / body activation listener / broadphase layer interfaces / job system / shape cast / CCD / batch add 各面在目标 JoltC 版本的可用函数清单落 PR 描述;**缺口处置三选一并登记**:(a) vendor 补丁(仓内 vendor 副本上补 C 面函数,优先 upstream 提 issue 并留链接,10 §8)/(b) 转 amer-koleci/joltc 候选面/(c) 收窄该面首版范围(如事件面降级为轮询模型)。
- **构建策略(I-1)**:优先 **vendor 内联**——JoltC + Jolt 源码 vendor 入 `src/rurix-physics-sys/`(许可 MIT 兼容,上游 commit pin 登记),经 `cmake`/`cc` crate 内联构建(可重现、CI 无外部依赖漂移);备选 = 外部 CMake 探测 + 缺失时确定性 `Err`(P-01)。PR-A 二选一落定并登记;CI default 档(= jolt)C++ 工具链 provisioning 责任同 PR 落实。

Jolt 版本 pin ≥ 5.2(上游 5.5 系,评审核实;C-5),具体版本随 PR-A pin 并登记。`rurix-physics-sys` 内部可对 JoltC 头文件 bindgen 或手写声明,**对外只露 safe Rust 类型与 u64 句柄**。G6_PLAN §4 R-G6-1(「G6.2 允许自维护 JoltC FFI」)就此兑现,不阻塞择优裁决。

#### C2. unsafe 集中与 lint 豁免(镜像 rurix-rt 模式)

- `rurix-physics-sys`:`[lints.rust] unsafe_code = "allow"` + `undocumented_unsafe_blocks = "deny"`(每块强制 `// SAFETY:`);其余全仓 crate 维持 workspace 默认 `deny`(根 Cargo.toml `[workspace.lints.rust] unsafe_code = "deny"` 现状);`rurix-physics` 与 `rurix-render` `#![forbid(unsafe_code)]`(§4.0-1)。
- unsafe-audit 登记:新文件 `unsafe-audit/rurix-physics-sys.md`,**U33 起续号**(number_ledger v1.28 claim),逐原语登记验证义务(沿 U26/U27/U31/U32 审计模式:loader/句柄线性配对 create-destroy/early-return 全路径销毁/fail-closed)。

#### C3. ABI 纪律(🔒 FFI ABI 高敏面)

- C ABI 面只过 `#[repr(C)]` POD 与 u64 句柄;`extern "C"` 签名逐字段与 JoltC 头对齐(sType/字段序/尺寸,布局锚定单测 = U32 `ffi_layout_anchors` 模式)。
- 所有权单向:world 拥有 body/shape;移除 body 后其 `BodyId` generation 失效,二次使用 → 确定性 `Err`(不悬垂,§4.A2)。
- 回调(contact listener / broadphase filter)经 `extern "C"` 函数指针 + `user_data` 指针:`user_data` 指向调用帧栈上状态,生命周期严格短于注册窗口(沿 U27 messenger `p_user_data` 栈纪律);回调内不回抛 panic(FFI 边界 catch + 错误码上岸);回调多线程触发的事件归一化纪律见 §4.A5。
- 线程纪律:Jolt job 线程只活在 step 相位内;FFI 边界对象跨线程移动经 `Send`/`Sync` 显式实现 + SAFETY 论证(Jolt `PhysicsSystem` 非线程安全面 = 库内相位门,§4.A4)。

#### C4. 审计判据(机器可核)

- CI grep 门(随 G6.2 PR):`src/rurix-render/` 与 `src/rurix-physics/`(非 sys)零 `rurix_physics_sys` 引用、零原生 Jolt/Rapier 类型名;`rurix-physics-sys` 之外零 `unsafe_code = "allow"` 新增。
- 渲染器不持有原生指针:bridge/同步层类型面只过 `BodyId`/`ShapeId`(§4.A2)。

### 4.D Rapier 快路径章(G6.4;验收门 G-G6-5;CI 步骤 90 拟)

#### D1. feature `rapier`(默认 off)

- `rurix-physics` feature `rapier` → 依赖 `rapier3d`(`default-features = false`,纯 Rust,无 CMake;dimforge 活跃维护,v0.33.0 2026-06,2026-07-31 调研);**默认 off**——核验判据:`cargo metadata` 中 `rurix-physics` 的 `default` feature 集合不含 `rapier`(CI 步骤 90 host 段断言)。

#### D2. 同 PhysicsWorld 抽象第二后端

- `BackendKind::Rapier` 走与 Jolt 同一 `PhysicsWorld` API(§4.A1~A6 全面:固定步/睡眠/批插/查询/事件/budget);Rapier 侧映射(`RigidBodySet`/`ColliderSet`/`QueryPipeline`/`EventHandler` 收集)在 `rurix-physics` crate 内(`forbid(unsafe_code)` 不破)。
- 能力差诚实登记:Rapier 无 Jolt 等价 batch-prepare/finalize 语义的,`add_bodies_batch` 以逐插 + 单测锚「不锁死主步」判据兜(§4.A7 同判据);CCD 映射 Rapier `CcdEnabled`;事件序列归一化排序与 Jolt 路径同契约(§4.A5,对拍可比性的前提)。

#### D3. 对拍判据(判据形态冻结、阈值数字不冻结;R-G6-2 + I-3 评审修订)

- 同场景(箱塔沉降 + 球滚动 + 批插移除脚本)双后端各跑 N=300 固定步。门判据形态:① **变换容差断言**(位置/旋转)——阈值**不写死于 RFC**,实现 PR 以重复对拍(≥5 次)实测噪声分布标定并写 evidence,阈值 ≤ 标定噪声包络(3σ);② **接触集合不变量**——Begin/End 事件对的 body 对集合重叠率 ≥ 99%、事件相位序列等价类一致(归一化序列上面向,§4.A5);③ **禁用跨引擎逐位相等**(§4.0-4)。
- 各自平台内确定性分别断言(§4.A7 烟测双后端各跑,(a) 口径)。

#### D4. 文档口径

- crate 文档与 demo 明示「**快路径 ≠ 性能/稳定性默认**」:Rapier 路径价值 = 纯 Rust/无 CMake CI 面与第二实现交叉验证;生产默认 = Jolt(G6_PLAN §0.1);不替换默认、不做性能宣称(P-09:实测数字写 evidence)。

### 4.E Taichi Vulkan AOT 特效副轨章(G6.5;验收门 G-G6-6 软门,不阻塞 G-G6-3/4/5)

#### E1. 定位(可选交付)

- 只做 **MPM/连续体粒子·体积场**特效(雪/沙/流体积观感面);**不进刚体求解**(刚体 = 章 A Jolt/Rapier 唯一面);不承担确定性联网。

#### E2. 设备与通路(2026-07-31 调研锚;I-4 评审修订)

- Taichi AOT:Python 侧 `ti.aot.Module` 落 `.tcm` 产物;宿主侧 **TiRT**(Taichi C 运行时)装载并 launch。
- 设备共享:`TiVulkanRuntimeInteropInfo`(Taichi ≥ 1.4.0 stable C API)支持注入外部 `VkInstance`/`VkPhysicalDevice`/`VkDevice`/queue——TiRT 挂**已有渲染设备**或并行设备上下文(首选同设备,避免跨设备拷贝;能力缺口走 §4.E3)。
- 产出通路与**所有权接缝**:粒子/体积场 buffer(TiRT NdArray,`ti_export_vulkan_memory` 导出 VkBuffer)→ graph **external import**(G5 external import 机制复用,不新建通路)→ 渲染消费(材质/体积 pass 读,物理世界不写)。**所有权图**:buffer 由 TiRT 端分配并拥有;graph external import **只读引用、不拥有、不入 transient 池**(TiRT buffer 生命周期跨帧持久,与 transient 每帧回收语义分属);同步 = TiRT kernel 完成 fence → 渲染帧消费 → 下一帧 TiRT 复用前 fence 等待;释放顺序 = graph 引用解除 → fence 完成 → TiRT 复用/释放。

#### E3. 失败路径(诚实边界;I-5 评审修订)

- **spike 成功最低判据**:TiRT Vulkan AOT kernel 在渲染设备上下文 launch 成功 + NdArray 导出 VkBuffer + graph external import 消费产出**非零 buffer/像素**(device 见证,`RURIX_REQUIRE_REAL=1`)。
- 未达判据(设备共享摩擦 R-G6-3 / interop 面不足 / 构建链过重)→ **诚实登记 RD**(自 RD-042 顺位)+ G6_CONTRACT §8 留痕;**不阻塞** G-G6-3/4/5 硬门;spike 不占 CI 步骤号(CI_GATES §2,成功判档后顺位续号)。

#### E4. 三条禁止(代码审计)

- 禁用 Taichi 替代主刚体;禁把确定性联网绑到粒子求解;禁在 CUDA 后端另起「主物理」(G6_PLAN §1 G6.5 / §0.2 纪律)。

## 5. 下游 spec 条款映射(spec diff,10 §3 要件)

**预期零新语言语义条款**(物理为引擎库,06 §8.3;G6_CONTRACT §7 ④;G5 RFC-0016 §5 先例)。五章全部为引擎库内部面(crate 内部契约,非 spec 条款面)。**条款先行纪律对条件消费路径保持**(硬规则 7):实现期确需新条款时,spec commit 先于实现 commit,按合入时 number_ledger 实际 `next_free` 顺位消费(现 RXS-0297,与 RD-038 兑现臂『条款按需自 RXS-0297 顺位』同源,先合先得、后合校准);**未消费不占号、不落裸条款头**。

| 章 | 既有条款复用面(零修订承诺) | 条件消费路径(确需时) |
|---|---|---|
| A~E | 无(spec 零修订;冻结接口 = crate 内部契约) | 确需语言语义时先判档(争议向上取严)→ 合入时实际 next_free 顺位 + RFC 修订行留痕 |

### 5.1 新错误码策略(预测;合并时以 registry 实号为准)

**预期零新 RX 码**(number_ledger v1.28 claim):物理库面违例走**库层错误枚举**(`PhysicsError`,Rust `Result`,镜像 RX6029/6030「图违例走库面诊断」口径的非 RX 码侧);sys crate 违例走 `Err` + 诊断文本(fail-closed,P-01)。确需时:codegen 自 RX6034 续 / 工具类自 RX7023(en/zh message-key 成对,registry/error_codes.json 只追加)。**`PhysicsError` 冻结范围**(I-7 评审修订):`PhysicsError` 为库层诊断枚举,**不属 §4.0-3 冻结接口**;实现 PR 可追加非 RX 变体(向后兼容),不得引入新 RX_error 码(number_ledger RX_error 预期零消费维持)。

## 6. feature gate / tracking / 实现序(10 §3 要件)

### 6.1 前置与失败测试先行

- 本 RFC **Approved 合入先于任何实现 PR**(G-G6-2,10 §3 硬性);**失败测试先行**(反 YAML-only):RFC 合入时点,`src/rurix-physics`、`src/rurix-physics-sys`、`ci/physics_core_smoke.py`(步骤 88 拟)、`ci/physics_bridge_smoke.py`(89 拟)、`ci/physics_rapier_parity_smoke.py`(90 拟)、`ci/uc08_physics_smoke.py`(91 拟,demo 定名后回填)、`apps/uc08-physics`、evidence schema 四件在 main **均不存在 = RED**(脚本名为拟名,随实现 PR 定案;步骤号一旦占用不复用,多余号作废声明 burned)。

### 6.2 feature gate 总裁决(R-2 评审修订)

新增**功能开关** feature 仅 `rapier`(默认 off,§4.0-7);`jolt` 为**默认后端构建 gate**(隔离 C++/CMake 构建依赖,非功能面开关;I-1 构建策略 vendor 优先,§4.C1);两 feature 组合矩阵 clippy/test 双验沿 G3/EI1/G4/G5 惯例——no-default(零 C++ 依赖,恒绿)/ default(= jolt,需 C++ 工具链,CI provisioning 随 PR-A 落实)/ default+rapier(双后端对拍);零语言 gate(物理为库)。

### 6.3 波次 PR 计划(照 G6_PLAN §1 波次;G3/EI1/G4/G5 结构先例)

- **G6.2 物理库底座**——PR-A:`rurix-physics-sys` FFI 边界(**前置**:JoltC 缺口审计 + 构建策略定案,§4.C1)+ unsafe-audit U33 起 + `rurix-physics` 世界/固定步/睡眠/批插体 + host 单测 + 步骤 88(`physics_core_smoke.py`,纯 host 门)→ 集成门 G-G6-3。PR-B:查询面(§4.A4 相位与排序纪律)+ 事件面(§4.A5 归一化)+ SyncBudget + 确定性烟测补强(若未随 PR-A 全落)。
- **G6.3 与渲染合流**——PR-C:同步桥(§4.B2/B3)+ 流送批插移除(§4.B4 `RemovalReceipt` 含 R-G6-4 竞态注入)+ AS 脏信号(§4.B5)+ 步骤 89(`physics_bridge_smoke.py`;device 段 = 合流 demo 物理驱动变换像素/变换非平凡断言)→ 集成门 G-G6-4。PR-D:`apps/uc08-physics` 合流 demo(刚体场景 + 既有 VisBuffer/GI/VSM/TAA 管线真跑)+ 步骤 91 + evidence schema 四件 + `g6_budget` counter(evaluator 分支同 PR)→ 门 G-G6-7。
- **G6.4 Rapier 快路径**——PR-E:feature `rapier` 第二后端 + 同场景对拍(阈值标定 evidence,§4.D3)+ 步骤 90(纯 host 门,无 CMake 路径)→ 门 G-G6-5。
- **G6.5 Taichi spike**——PR-F:AOT + TiRT 设备共享 + external import(成功判据 §4.E3)或 RD-042+ 登记(失败)→ 软门 G-G6-6。
- **G6.6 close-out**——PR-G:全量回归冻结 + 门终审表 + RD/SG 处置 + status flip → 门 G-G6-8。

### 6.4 每 PR 不变量核验(全期硬约束)

既有零回归:步骤 41~87 判据 0-byte 只增(步骤 69 blocked 探针恒跑 / 步骤 70 永久 gap / 步骤 84~86 RD-038 分波探针按自身轨道)/ dxil 套件恒定 / vulkan 套件 grow-only。LF byte-exact;counter/entries 不预造(与 evaluator 分支同 PR);device measured + run URL 归 G6_CONTRACT §8 面;`RURIX_REQUIRE_REAL=1` 贯穿 device 段(缺 provisioning SKIP = dev-env degrade,mock/SKIP 不充绿);新 unsafe U33+ 登记;GPU 实验全经 proc_guard;evidence/ 只增不删;G5 冻结面 0-byte(§4.B6)。

## 7. 备选方案

| 方案 | 否决理由 |
|---|---|
| 直接依赖 `rolt`/`jolt-rust` | 停滞(0.3.1+Jolt-5.0.0,2024-05)+ 自述不完备 + Jolt 5.0 滞后(§4.C1);R-G6-1 坐实 |
| PhysX-GPU 主物理 | CUDA 绑定,违后端正交(§4.0-2);Vulkan 车道留给 G5 效果面 |
| 商用 Havok | 许可与可审计引擎库路线冲突(G6_CONTRACT out_of_scope) |
| FleX / Avian / Bullet 主物理 | 遗留粒子栈 / Bevy 绑 / 多核维护落后(G6_PLAN §0.1) |
| wgrapier(GPU Rapier)生产依赖 | 研究观察项;GPU 主刚体否决(§4.0-2) |
| Rapier 作生产默认 | 「多静少动」开放世界弱于 Jolt(G6_PLAN §0.1);降为快路径第二后端(章 D) |
| Taichi 进刚体求解 / 主物理 | 特效副轨定位(章 E);禁替代主刚体(§4.E4) |
| 物理 sim 上渲染队列(async compute) | 与 VisBuffer/VSM/GI/RT 抢车道(G6_PLAN §0.4 禁止线) |
| 查询快照含加速结构双缓冲 | 过度设计(C-4/I-2 评审否决):cast 查询 step 外相位即可满足,加速结构无跨相位读需求;仅保变换数组快照 |

## 8. 不做(范围红线)

- GPU 主刚体(PhysX CUDA 刚体 / wgrapier 验收依赖 / Warp-Newton 主环);商用 Havok;软体/布料/流体进硬门(Jolt CPU 软体与 Taichi MPM 仅 spike/副轨)。
- Newton / Genesis / MuJoCo Warp 合入主仓 CI(研究隔离;独立仓库或 feature 永不默认)。
- DXIL RT 腿(RD-034 blocked 维持,本期物理不依赖 DXIL);窗口/输入进语言(D-130 红线)。
- 性能数字进硬门(measured 写 evidence,P-09);引擎采纳/下载量/用户数宣称(carve-out 沿 MS1/EA1/EI1/G4/G5 先例)。
- 改写 G5 closed 契约正文(只追加引用);改写 00–14 规划文档(`check_planning_docs` 纪律);新语言语义条款(预期零,§5)。
- `MaterialClosure` 32B / VisBuffer 位格式 / `Barrier` EB 三轴 / `PageRequest` 字段布局修订(G5 冻结面 0-byte,§4.B6)。

## 9. 未决问题 / 关键裁决

编号规则:`Q-<名>`。全部为 agent 拟裁(D-406 v2.0,Approved 即定案);对抗性评审 disposition 可修订,修订落 §9.1 与修订记录。

| # | 问题 | 裁决 |
|---|---|---|
| Q-A | JoltC 来源与版本 pin? | 首选 SecondHalfGames/JoltC(消 UB 口径),Jolt ≥ 5.2(上游 5.5 系);**PR-A 前置**:JoltC 缺口审计(contact listener/activation/broadphase/job system/shape cast/CCD/batch add)+ 缺口三选一处置(vendor 补丁 / 转 amer-koleci/joltc / 收窄首版范围);构建策略 vendor 内联优先(备选外部 CMake 探测)(C-3/C-5/I-1 评审修订,§4.C1) |
| Q-B | 并发 query 与 step 的相位纪律? | cast 类查询 = **step 外并发**(G6_PLAN §2.1 字面;Jolt Update 期间禁读写,类型面不暴露相位内路径);变换读 = step 边界提交的上一拍变换数组(仅数组快照,不复制加速结构);cast 结果与 ContactEvent 均经**规范序归一化**后对外(确定性面);证伪回退 = 全量 step 外串行 + 相位锁(C-2/C-4/I-2 评审修订,§4.A4/A5) |
| Q-C | MV 供给形态? | bridge 差分上一拍→当前拍 `PhysicsTransform`;静态/睡眠体零 MV;可选缓冲经 graph external import 供 temporal;**缓冲格式不冻结**,由 temporal 消费侧与实现 PR 标定(R-4 评审修订,§4.B3) |
| Q-D | 合流 demo 落点? | 新 `apps/uc08-physics`(02 文档 UC 序列 UC-07 已由 ruridrop 占用,UC-08 顺位;实现期若有占用按实际顺延),uc06 既有资产 0-byte;定名回填 CI_GATES 步骤 91 |
| Q-E | Jolt job 系统与宿主线程池? | 适配层抽象(宿主可注入池)+ 默认库内自带池;engine 侧通用线程池仓内不存在(2026-07-31 核实),宿主池接入为后续增量不阻塞 G6.2(§4.A3) |
| Q-F | Jolt 默认 feature 与「全 feature off 零依赖绿」不变量冲突? | 不冲突:`jolt` 为默认后端构建 gate(隔离 C++ 工具链依赖,构建面如实登记);`--no-default-features` 构建零 C++ 依赖绿(无后端编译 → 确定性 Err,P-01);三档矩阵 clippy/test 双验(R-2/I-1 评审修订,§4.0-7/§6.2) |

## 9.1 对抗性评审记录(D-409)

**已完成 第 1 轮 2026-07-31**——由与起草者 Provenance **不同**的工具执行三镜头(correctness / redline / implementability)批判性(对抗性)评审,**评审 provenance `kimi-cli:kimi-for-coding`(独立 kimi-cli 实例,独立进程/零共享上下文,仅持评审提示词与仓库访问)≠ 起草 provenance `Kimi Code CLI (Kimi)`**(硬规则 2 可机验,`ci/check_contribution.py` 规则 4)。17 findings(2 blocker / 11 major / 4 minor)逐条 disposition:**17 条全部采纳并修**(2 blocker 正文实改 + 11 major 正文实改 + 4 minor 正文实改),无驳回、无空过。状态 Draft → Agent Approved(先于任何实现 PR,G-G6-2)。

**环境留痕(诚实边界,不冒充跨模型)**:首选跨模型评审者 `claude` CLI(Claude Code 2.1.215)执行失败——返回 `403 Request not allowed`(账号级不可用,本会话不可修;失败输出存 `.tmp/g6_rfc0017_review_claude.md` 现场;RFC-0015 §9.1 同款 403 先例)。改用独立 kimi-cli 实例执行(评审提示词存 `.tmp/g6_rfc0017_review_prompt.md`,完整转录存 `.tmp/g6_rfc0017_review_kimi.md`):**工具级 provenance 相异成立**(`kimi-cli:kimi-for-coding` ≠ `Kimi Code CLI (Kimi)`),**但模型同族(kimi-for-coding 系)**——本轮为「跨工具、同模型族」评审,非 D-409 理想形态的「跨模型」。D-409 状态 = Proposed(13 §6),本偏差如实登记;claude 可用后欢迎追加跨模型第 2 轮(修订行追加,不重开 RFC)。评审者自报 provenance 字符串误为 `Kimi Code CLI (Kimi)`(其对自身执行上下文的误判),本段以实际执行上下文(独立 kimi-cli 实例)为准更正(RFC-0015「以实际枚举为准更正」先例);评审者并将此列为 R-1 finding,disposition 见下表。

| 字段 | 值 |
|---|---|
| 评审者 provenance | `Assisted-by: kimi-cli:kimi-for-coding`(独立实例;≠ 起草 `Kimi Code CLI (Kimi)`;同模型族偏差如实登记,见上环境留痕) |
| 评审轮次 | 第 1 轮,2026-07-31 |
| 结论 | **2 blocker / 11 major / 4 minor**;全部采纳并修,无驳回 |

**Findings 与 disposition**(每条一行;镜头前缀 C=correctness / R=redline / I=implementability):

| # | Finding(评审者提出) | 严重度 | Disposition |
|---|---|---|---|
| C-1 | §4.0-4 将 Jolt 确定性窄化为「平台内同输入同输出」并称「Jolt 官方口径」——事实上 Jolt 提供 `CROSS_PLATFORM_DETERMINISTIC` 选项可达跨平台 bit 级一致;窄化与上游事实不符且削弱烟测覆盖 | major | **采纳并修 §4.0-4**:确定性分两档——(a) 默认口径 = 同二进制同平台重放逐位一致(G-G6-3 烟测判据);(b) 可选口径 = 跨平台 deterministic 构建选项,启用与否随实现 PR 登记 evidence |
| C-2 | 未处理 Jolt 查询/事件面非确定性来源:`BroadPhaseQuery` 非确定、`NarrowPhaseQuery` 结果一致但顺序可变、`ContactListener` 回调多线程顺序非确定;「ContactEvent ring 确定性丢弃最旧」缺序列确定性前提 | major | **采纳并修 §4.A4/§4.A5**:cast 结果按 `(t, BodyId)` 规范序排序(或取最近命中)后对外,BroadPhaseQuery 不作默认面;ContactEvent 在 step 结束边界按 `(min(a,b), max(a,b), phase)` 归一化排序去重后入 ring,事件序列确定性 = 归一化序列语义,溢出丢弃在归一化序列上定义 |
| C-3 | RFC 假设 JoltC C API 已覆盖 contact listener/broadphase filter/job system 回调,但 SecondHalfGames/JoltC 自述 WIP(「functions that we've needed」),覆盖面不承诺;若实现期发现缺口将被迫维护 C++ 补丁,范围与风险未估 | blocker | **采纳并修 §4.C1/Q-A**:JoltC 缺口审计钉为 G6.2 PR-A **前置**(七面函数清单落 PR 描述:contact listener / activation / broadphase layer / job system / shape cast / CCD / batch add);缺口处置三选一登记(vendor 补丁〔upstream issue 链接〕/ 转 amer-koleci/joltc / 收窄首版范围) |
| C-4 | 快照双缓冲(Q-B)在 G6_PLAN §2.1 冻结接口草案中未出现,属 RFC 相对上游事实源的扩展设计;「query 与 step 并发烟测」如何证明真并发而非相位串行未定义 | major | **采纳并修 §4.A4/Q-B**:加速结构快照方案否决为过度设计——cast 查询收窄为 step 外并发(G6_PLAN §2.1 字面),仅保变换数组快照;机验判据钉死(≥2 线程并发 query 排序后一致 + step 相位内快照读一致) |
| C-5 | 「上游已 5.2/5.3」过时:Jolt 最新 release v5.5.0(2025-12,评审核实);时效性描述误导版本 pin 决策 | minor | **采纳并修 §4.C1**:改「上游已至 5.5 系(评审核实),pin ≥ 5.2,具体版本随 PR-A pin 并登记」 |
| C-6 | 「批插体不锁死主步」缺量化判据(延迟有界 = 多少?) | minor | **采纳并修 §4.A7**:量化判据 = prepare 在 step 外交替期、finalize 单点提交,批插期间主步延迟 ≤ 1 帧,host 单测断言 |
| R-1 | 评审 provenance 与起草 provenance 同为 `Kimi Code CLI (Kimi)`(评审者自报),违反 D-409 provenance 不等式,机验不过 | blocker | **采纳并修 §9.1**:评审实际执行上下文 = 独立 kimi-cli 实例(独立进程/零共享上下文),正式记录以执行上下文签署 `kimi-cli:kimi-for-coding` ≠ 起草 `Kimi Code CLI (Kimi)`,工具级相异成立;评审者自报字符串误差以实际枚举为准更正(RFC-0015 先例);同模型族偏差如实登记环境留痕,不冒充跨模型 |
| R-2 | 全文多次称「唯一新 cargo feature = rapier」,同节又引入默认 feature `jolt`——两 feature 皆新增,「唯一」表述自相矛盾,误导 G-G6-5 默认 off 核验范围 | major | **采纳并修 §4.0-7/§6.2**:统一口径 = 新增功能开关 feature 仅 `rapier`(默认 off);`jolt` 为默认后端构建 gate(隔离 C++/CMake 依赖,非功能面开关);两 feature 组合矩阵明示 |
| R-3 | R-G6-4「先卸 body 再放页」靠「类型面 + 运行时断言」但类型面如何强制未具体化,跨线程场景易退化为文档约定 | major | **采纳并修 §4.B4**:`remove_bodies_batch` 返回 `RemovalReceipt`(与页 id 绑定),流送层须持 receipt 方能放页(编译期不可构造无 receipt 路径 + 运行时断言双保险);并发时序脚本单测注入具体化 |
| R-4 | MV 缓冲格式(2D 屏幕空间 vs 3D 世界空间)未钉却列入冻结接口,实质把实现期决策提前锁定,违 G5 冻结面只写「可选 MV 缓冲」精神 | minor | **采纳并修 §4.B3/Q-C**:MV 缓冲格式移出冻结接口,由 temporal 消费侧与实现 PR 标定,不改 G5 时域底座数据结构 |
| I-1 | 默认 feature `jolt` 引入 C++/CMake 构建链,与「默认构建零重依赖」惯例摩擦;vendored vs 外部构建策略未定,CI provisioning 责任未落实 | blocker | **采纳并修 §4.C1/§6.2/Q-F**:构建策略裁决 = vendor 内联优先(JoltC+Jolt 源码 vendor 入 sys crate,commit pin 登记,`cmake`/`cc` 内联构建),备选外部 CMake 探测 + 确定性 Err;CI default 档 provisioning 责任随 PR-A 落实 |
| I-2 | 快照双缓冲实现成本未分析:step 前复制全部 body 变换 + 加速结构(内存/CPU 开销)或依赖 Jolt 内部只读接口线程安全保证,二者皆未论证 | major | **采纳并修 §4.A4**:同 C-4——加速结构快照否决;快照收窄为变换数组浅拷贝(体数 × 32B 级,预算可忽略);证伪回退 = 全量 step 外串行 query |
| I-3 | 对拍容差阈值(1e-2 m / 1°)为起草侧先验,过松形同虚设、过紧永不绿;标定方法未进硬门 | major | **采纳并修 §4.D3**:阈值数字移出 RFC——判据形态冻结(容差 ≤ 实测噪声包络 3σ,经 ≥5 次重复对拍标定并写 evidence;接触集合重叠率 ≥ 99%),阈值随实现 PR 标定 |
| I-4 | Taichi TiRT external import 与 G5 graph external import 接缝未分析:TiRT buffer 生命周期跨帧持久 vs graph transient 池每帧回收,ownership/同步/释放顺序未定 | major | **采纳并修 §4.E2**:所有权图钉死——buffer TiRT 拥有,graph 只读引用不拥有不入 transient 池;帧末 fence 排序;释放顺序 = graph 引用解除 → fence 完成 → TiRT 复用/释放 |
| I-5 | spike 成功判据未定义(「失败 → 登记 RD」的反面不明) | minor | **采纳并修 §4.E3**:成功最低判据 = TiRT AOT kernel 在渲染设备 launch 成功 + NdArray 导出 VkBuffer + graph external import 消费产出非零 buffer/像素(device 见证) |
| I-6 | `BodyId` generation 32b 回绕面自暴于 §9.2 但正文无处理:回绕后旧句柄复活指向新 body,破坏「二次使用 → 确定性 Err」 | minor | **采纳并修 §4.A2**:generation 单调递增不回绕,32b 空间耗尽槽位退休不再分配;index 池耗尽 → `Err(PoolExhausted)`;回绕复活路径类型面消灭 |
| I-7 | `PhysicsError` 是否属冻结接口未明确;若冻结,后续新增错误变体需走 RFC 修订,灵活性差 | minor | **采纳并修 §5.1/§4.0-3**:`PhysicsError` 为库层诊断枚举,不属冻结面;实现 PR 可追加非 RX 变体(向后兼容),零新 RX_error 码维持 |

**实现期留痕(G6.2 S1/R2-2,2026-07-31,修订记录 v1.2;§4.0-3 口径:修订只经 RFC 修订行 + §9.1 留痕,不重开 RFC)**:① **§4.C4 矛盾发现与收窄**——G6.2 实现发现 §4.C4 首条 grep 判据字面(「`src/rurix-render/` 与 `src/rurix-physics/`(非 sys)零 `rurix_physics_sys` 引用」)与 §4.0-1/§4.0-7 自相矛盾:safe crate 消费 sys crate 是 §4.0-1 Approved 架构本身(`rurix-physics` = safe API 封装,`jolt` 默认后端构建 gate 经 `dep:rurix-physics-sys` 实现,§4.0-7),字面执行会使架构不可构建。经修订记录 v1.2 收窄为原意:`src/rurix-render` 零 `rurix_physics_sys` 引用 + 零原生 Jolt/Rapier 类型名(0-byte 不变);`src/rurix-physics` 公共 API 不透出 sys/原生类型(代码审计),crate 内部 sys 消费收敛于 `src/world.rs` 单一模块(grep 判据:rurix-physics 内除 `src/world.rs` 外零 `rurix_physics_sys` 引用;原生类型名 `JoltPhysics|JPC_|JPH::|rapier3d` 两 crate 全禁维持)。收窄理由 = §4.0-1 架构一致性(判据须可机验且不得判红 Approved 架构本身);**评审口径不受影响**:G-G6-3 契约层判据「对外 API safe(`rurix-physics`/`rurix-render` 维持 `#![forbid(unsafe_code)]`)、渲染器不持有原生 Jolt/Rapier 指针」(§4.0-1/§4.C3)0-byte 维持,收窄只动 crate 内部消费面的 grep 落点。② **S1 JoltC 缺口处置登记**(C-3 缺口处置三选一 sanctioned 兑现;五处缺口全部走 (c) 收窄首版范围/实现路线,零 vendor 补丁、未转 amer-koleci/joltc):contact 回调不含求解后 impulse → `impulse` 首版恒 0.0(升级路径 = `JPC_EstimateCollisionResponse` 逐回调估算,成本敏感后置);body activation listener 缺 C 面 → 不注册监听器,`slept_this_step`/`active_bodies` 经 step 前后 `JPC_BodyInterface_IsActive` 轮询差分;broadphase 首版单 layer(全部 object layer → BP layer 0,Jolt 显式支持单树,moving/non-moving 双树优化后置);`JPC_NarrowPhaseQuery_CastRay` 仅最近命中(impl 内 `ClosestHitCollisionCollector`)→ Rust 侧排除循环(逐轮将已命中 body 经 `JPC_BodyFilter` 排除后再 cast,直至无命中)实现全命中契约,零 C++ 补丁;`JPC_BodyInterface_DestroyBodies` impl 被上游注释(WIP 缺口)→ `RemoveBodies`(批量)+ 逐 `DestroyBody` 绕行(Jolt `DestroyBodies` 语义等价)。明细与七面函数清单 = [src/rurix-physics-sys/VENDOR.md](../src/rurix-physics-sys/VENDOR.md) §3;五处收窄均已确认无损 §4.A 冻结契约(§4.A5 impulse 语义 / §4.A7 睡眠统计 / 层过滤 / 查询全命中 / 批移除)。

**实现期留痕(G6.3 合流,2026-07-31,修订记录 v1.3;§4.0-3 口径:修订只经 RFC 修订行 + §9.1 留痕,不重开 RFC)**:① **§4.B4(R-3)「`remove_bodies_batch` 返回 `RemovalReceipt`」落点解释**——落地解释为**合流层类型**:`StreamingBridge::remove_page` 返回 `RemovalReceipt`(与 `PageKey` 绑定,移动语义不可 Clone,编译期不可伪造;无 receipt 的放页路径类型面不可构造,先卸 body 再放页),`PhysicsWorld::remove_bodies_batch` 签名维持 G6.2 落地形态 `Result<(), PhysicsError>` 0-byte——页 id 为合流层概念,物理世界不感知页(G6.2 world.rs 注释留痕一致:「`RemovalReceipt` 流送纪律」归 bridge 层);§4.B4 冻结纪律(R-G6-4:先卸 body 再放页 / receipt 与页绑定 / 无 receipt 放页编译期不可构造)语义全额兑现,仅凭据类型持有点由 world 层解释为合流 streaming bridge 层。② **sys 混批重排缺陷修复**(实施期发现,与冻结接口无涉,纯缺陷修复 + 回归测试):`src/rurix-physics-sys/src/world.rs` 批插入在 >32 体混 broadphase 层批下被 Jolt `AddBodiesPrepare` 内部 QuickSort 非稳定重排(≤32 回退插入排序恒等,故 G6.2 未暴露),致返回序/kind 登记/激活三错位;修复 = prepare 前快照 `ids_orig` 三处按原始序配对;回归测试 `mixed_layer_batch_insert_order_preserved`(35 体混批)先红后绿;零新增 unsafe,SAFETY 注释增强。

**实现期留痕(G6.4 落地,2026-07-31,修订记录 v1.4;§4.0-3 口径:修订只经 RFC 修订行 + §9.1 留痕,不重开 RFC)**:① **§4.C4 grep 判据收窄**——G6.4 Rapier 快路径落地发现 §4.C4 原生类型名判据(v1.2 口径「`JoltPhysics|JPC_|JPH::|rapier3d` 两 crate 全禁」)与 §4.D2 自相矛盾:Rapier 侧映射(`RigidBodySet`/`ColliderSet`/`QueryPipeline` 等)在 `rurix-physics` crate 内实现是 §4.D2 Approved 架构本身,字面执行使 `rapier` feature 档不可构建(v1.2 时 `rapier3d` 仅作未来名占位入正则,G6.2/G6.3 工作树无命中故未暴露)。经修订记录 v1.4 收窄为原意:`rapier3d` 原生类型名收敛 `src/rurix-physics/src/rapier.rs` 单一 sanctioned 消费模块(镜像 v1.2 sys 消费收敛 `src/world.rs` 先例);rurix-physics 其余文件与 `src/rurix-render` 全 crate 对 `rapier3d` 零命中(render 0-byte 维持);Jolt 原生名(`JoltPhysics|JPC_|JPH::`)两 crate 全禁维持(src/rapier.rs 亦不例外);公共 API 不透出 sys/原生类型由代码审计面兜(不变);G-G6-3 契约层判据(对外 API safe / 渲染器不持有原生 Jolt/Rapier 指针)0-byte 不受影响,收窄只动 crate 内部消费面的 grep 落点;CI 步骤 88 grep 门同步收窄(ci/physics_core_smoke.py,旧逻辑对工作树 rapier.rs 误红、新逻辑绿 = 红绿验证本身)。② **§4.D1 feature 面解释性留痕**——「`default-features = false`」原意 = 纯 Rust 无 CMake、不启用 parallel·simd·serde 重面;实现期发现 rapier3d 0.33 lib target `required-features = ["dim3", "f32"]` 且 math/parry 依赖链需 `std` 启用(零 feature 字面不可编译,84 个 E0432),故 Cargo.toml 落 `default-features = false` + 显式最小集 `features = ["dim3", "f32", "std"]`;`parallel`/`simd-stable`/`serde-serialize`/`enhanced-determinism` 维持 off(单线程标量面,不静默宣称并行/确定性增强);§4.D1「纯 Rust,无 CMake」与「默认 off」核验判据(cargo metadata default 集合不含 `rapier`)0-byte 维持。③ **rapier.rs 能力差诚实登记摘要**(§4.D2 判据;与 [src/rurix-physics/src/rapier.rs](../src/rurix-physics/src/rapier.rs) 模块头一致,详情以该文件为准):批插逐插(Rapier 无 Jolt AddBodiesPrepare/Finalize 等价语义,`add_bodies_batch` 以逐插实现,「批插不锁死主步」判据由 behavior 测试双后端同锚兜底,§4.A7 同判据);事件 Begin/Persist/End = step 结束边界对窄相 `contact_pairs()` 与上一拍对集差分**单源合成**(不依赖 `EventHandler`,事件载荷需求超 CollisionEvent 面;Begin = 本拍有上拍无、End = 上拍有本拍无、Persist = 交集;归一化排序去重走 world.rs 共享面,与 Jolt 路径同契约 §4.A5;body 移除即接触关系随世界消亡,移除时不发 End——Jolt 侧同名不可得事件经 unmapped 丢弃,两出口语义一致);**impulse 不比对**(§4.D3:Jolt 侧首版恒 0.0 系 JoltC 缺口,修订记录 v1.2 已登记;Rapier 侧取 manifold 求解冲量点最大值);CCD → `CcdEnabled`,睡眠 → `can_sleep`,运动学 → position-based;层 → `InteractionGroups` 32 位(memberships = 1<<layer,filter = ALL),`layer_count > 32` → 世界创建确定性 `Err(BackendUnavailable)`;单线程标量(feature 面 dim3/f32/std),`WorldDesc::job_threads` 为 Jolt 专用、Rapier 后端忽略(文档留痕,不静默宣称并行);`cast_ray` `t_min` 后过滤(`intersect_ray` 仅收 max_toi,t ≥ t_min 过滤在映射层,solid = true 游戏查询惯例);`cast_shape` 全命中 = 排除循环(最近命中 → 排除已命中 collider 重查至无新命中;Jolt 侧 CastRay 同法,修订记录 v1.2 先例;witness/normal 局部系,上岸前转世界系);宽相新鲜度 step 内自动维护,add/remove 后手动 `update` **只增改不删**(stale leaf 经 `colliders.get_unknown_gen` 自然过滤,查询不触达已删体);`is_active` 语义对齐 Jolt(静态恒 false,动态/运动学 = !is_sleeping)。

**实现期留痕(G6.5 落地〔成功臂〕,2026-07-31,修订记录 v1.5;§4.0-3 口径:修订只经 RFC 修订行 + §9.1 留痕,不重开 RFC;§4.E 冻结条文 0-byte 未漂移)**:① **成功臂判档(§4.E3 四段闭合)**——TiRT Vulkan AOT kernel 在渲染设备上下文 launch 成功 + NdArray 导出 VkBuffer(256B = 64 粒子 × f32)+ graph external import 消费接线(byte_size=256;只读引用、不拥有、不入 transient 池,§4.E2 所有权图按字面兑现)+ readback 64/64 非零且 first_values=[1.0,2.5,4.0,5.5] 与 i*1.5+1.0 逐位相等(RURIX_REQUIRE_REAL=1,RTX 4070 Ti,一次真跑即绿,零 tirt 修复);**RD-042 未消费**(成功臂无需失败登记)。② **设备路径解释性留痕**——§4.E2 明示「挂已有渲染设备**或并行设备上下文**」两径(首选同设备避免跨设备拷贝;能力缺口走 §4.E3),本臂取**并行设备上下文**:`TiVulkanRuntimeInteropInfo` 注入 spike 侧自建 VkInstance/VkPhysicalDevice/VkDevice/queue(RTX 4070 Ti);此为 §4.E2 明文允许路径、非能力缺口,首选同设备路径留后续波次。③ **U43 登记**——`src/rurix-rt/src/tirt.rs`(feature `taichi-tirt` 默认 off;13 符号运行时动态装载 taichi_c_api.dll,免导入库/免构建期 Taichi 依赖;`run_particles_spike` 全链 runtime → AOT module → kernel launch → `ti_export_vulkan_memory`;`TirtError` 六变体 fail-closed)= G6.5 TiRT FFI 边界,沿 U26/U27/U31/U32 审计模式登记 unsafe-audit/rurix-rt.md;`vk.rs` pub(crate) 三件套 cfg gate(既有面 0-byte);rurix-render `#![forbid(unsafe_code)]` 维持、U 面零消费(§4.0-5/§4.C3 0-byte)。④ **CI 步骤 92 落地**——`ci/taichi_vulkan_spike_smoke.py`(门 G-G6-6 软门:host 六判据恒跑〔AOT 资产核验/feature `taichi-tirt` 双包默认 off 机验/§4.E4 三条禁止审计/U43 登记核验/cargo test -p uc09-taichi-spike/host 腿 --json 8 断言〕+ device gate real〔RURIX_REQUIRE_REAL=1 + RURIX_TAICHI_C_API_DLL provisioning,缺则 SKIP=dev-env degrade 退 0 不充绿〕+ --selftest 红绿自检)+ `taichi_vulkan_spike_evidence_schema.json` + check_schemas 注册 + pr-smoke.yml 真步骤(步骤 88~91 0-byte);§4.E4 三条禁止经 smoke host 段审计机器闭合(rurix-physics/rurix-render 零 taichi 引用、零 CUDA 主物理路径);g6_budget.json 空壳维持(步骤 88~91 无 budget counter 体例,92 不加 counter);evidence 两份如实(taichi_vulkan_spike_20260731T160536.json 真跑 ok=true / T160608.json host 态 SKIP ok=false)。

## 9.2 已知风险与评审攻击面(起草侧自暴,供 §9.1 评审镜头用)

> **评审已消化(2026-07-31 第 1 轮)**:本节攻击面①~⑭已由评审逐条覆盖或另发现(C-1~C-6/R-1~R-4/I-1~I-7),disposition 见 §9.1;下列条目为起草稿原状留痕,正文实改以 §9.1 disposition 与修订记录 v1.1 为准。

- **章 A**:① 快照双缓冲(Q-B)是 §4.A4 新引入机制,G6_PLAN §2.1 未明示——攻击点:是否过度设计?「query 与 step 并发烟测」(G-G6-3)在快照路径下如何算真并发而非相位串行?② `add_bodies_batch` 映射 Jolt AddBodiesPrepare/Finalize 的断言「不锁死主步」缺量化判据(延迟有界 = 多少?)。③ 确定性口径「平台内」未定义平台边界(OS/arch/编译器版本/SIMD 面)。
- **章 B**:④ MV「差分供给」未钉缓冲格式(逐实例 2D 屏幕空间 vs 3D 世界空间 motion vector)——攻击点:§4.B3 是否把应属实现期的格式决策提前冻结?⑤ R-G6-4「先卸 body 再放页」在流送与物理不同线程时如何强制(类型面?运行时断言?)。
- **章 C**:⑥ JoltC 两候选(SecondHalfGames vs amer-koleci)推给实现 PR 定案——攻击点:R-G6-1 裁决是否实质未落地?⑦ JoltC C API 覆盖面未知(contact listener / broadphase filter / job system 回调是否全在 C 面)——若缺口存在,自维护补丁面多大?⑧ 默认 feature `jolt` 引入 C++/CMake 构建链,与仓内「默认构建零重依赖」惯例的摩擦是否低估?
- **章 D**:⑨ 容差阈值(1e-2 m / 1°)为起草侧先验,未标定——攻击点:阈值过松形同虚设、过紧永不绿;标定方法(实测噪声分布)是否该进硬门?
- **章 E**:⑩ TiRT external import 与 G5 graph external import 的语义缝(Taichi buffer 生命周期 vs graph transient 池)未分析;⑪ spike 成功判据未定义(什么算「成功」vs「能力缺口」)。
- **横切**:⑫ `BodyId` generation 32b 回绕面;⑬ `PhysicsError` 枚举面是否该在 RFC 冻结(§4.0-3 冻结范围是否含错误枚举成员);⑭ 步骤 88~91 拟名与 evidence schema 四件未落,RED 判据是否充分。

## 10. 稳定化与 provenance

- **稳定化**(10 §5):本 RFC 为引擎库面,不进语言稳定面;crate API 稳定化随 G6 close-out 后两个里程碑无重大修订 → stabilization report → FCP-lite(10 §2.2,advisory 公开等待窗)。
- **Provenance**:`Assisted-by: Kimi Code CLI (Kimi)`(起草)。agent 自主决策;§9.1 对抗性评审已完成(评审 provenance ≠ 起草,D-409/硬规则 2),批准后推进 §6.3 下游实现 PR。

## 11. 规范与实现依据

- [G6_PLAN](../milestones/g6/G6_PLAN.md) v1.1(择优裁决 §0.1 / 冻结接口草案 §2 / 波次 §1 / 风险 §4)· [G6_CONTRACT](../milestones/g6/G6_CONTRACT.md) v1.0(rfc_required / G-G6-1~8 / guardrails)· [CI_GATES](../milestones/g6/CI_GATES.md)(步骤 88~91 拟分配)
- [RFC-0015](0015-engine-rendering.md)(G4 单伞形先例 / §9.1 跨工具同模型族评审留痕先例)· [RFC-0016](0016-native-renderer.md)(G5 伞形体例 / §4.H 时域底座 / G5 冻结面)
- Jolt Physics([jrouwe/JoltPhysics](https://github.com/jrouwe/JoltPhysics),[文档 5.2.0](https://jrouwe.github.io/JoltPhysicsDocs/5.2.0/index.html);上游 release 5.5 系,评审核实)· JoltC([SecondHalfGames/JoltC](https://github.com/SecondHalfGames/JoltC) · [amer-koleci/joltc](https://github.com/amerkoleci/joltc))· rolt/jolt-rust([SecondHalfGames/jolt-rust](https://github.com/SecondHalfGames/jolt-rust),[crates.io rolt 0.3.1+Jolt-5.0.0](https://crates.io/crates/rolt))
- Rapier([dimforge/rapier](https://github.com/dimforge/rapier),v0.33.0 2026-06)· Taichi AOT/TiRT([Taichi C++ 部署教程](https://docs.taichi-lang.org/docs/tutorial) · [Vulkan backend `TiVulkanRuntimeInteropInfo`](https://docs.taichi-lang.org/docs/master/taichi_vulkan))
- 仓内:[backend.rs](../src/rurix-rt/src/backend.rs)(compute 双后端纪律)· [gpu_scene.rs](../src/rurix-render/src/geometry/gpu_scene.rs)(update_transform/flush_dirty)· [graph/types.rs](../src/rurix-render/src/graph/types.rs)(PageRequest)· [unsafe-audit/](../unsafe-audit/)(U26~U32 审计模式)· registry/number_ledger.json v1.28(reserved_in_flight[G6])

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v0.1 | 2026-07-31 | AI 起草初版(伞形五章:§4.0 跨章约定 + A 物理库边界 / B 渲染同步契约 / C FFI 与 unsafe 纪律〔R-G6-1 裁决:自维护 JoltC FFI,rolt 停滞否决〕/ D Rapier 快路径 / E Taichi Vulkan AOT 副轨;预期零新语言语义条款;§9 Q-A~Q-F 拟裁;§9.2 攻击面自暴 14 项) | Full RFC(Draft) |
| v1.1 | 2026-07-31 | **D-409 对抗性评审完成 + 状态翻 Agent Approved**(评审 provenance `kimi-cli:kimi-for-coding` 独立实例 ≠ 起草 `Kimi Code CLI (Kimi)`;首选 claude 403 不可得,跨工具/同模型族偏差如实登记 §9.1 环境留痕):17 findings(2 blocker + 11 major + 4 minor)全部采纳并修无驳回——C-1 确定性分两档(§4.0-4)/ C-2 查询与事件规范序归一化(§4.A4/A5)/ C-3 JoltC 缺口审计钉为 PR-A 前置(§4.C1)/ C-4+I-2 加速结构快照否决收窄为变换数组快照(§4.A4)/ C-5 Jolt 上游 5.5 系(§4.C1)/ C-6 批插量化判据 ≤1 帧(§4.A7)/ R-1 以执行上下文签署评审 provenance(§9.1)/ R-2 feature 口径统一(§4.0-7/§6.2)/ R-3 RemovalReceipt 类型纪律(§4.B4)/ R-4 MV 格式移出冻结(§4.B3)/ I-1 vendor 内联构建优先(§4.C1)/ I-3 对拍阈值移出 RFC 改判据形态冻结(§4.D3)/ I-4 TiRT buffer 所有权图(§4.E2)/ I-5 spike 成功最低判据(§4.E3)/ I-6 generation 退休纪律(§4.A2)/ I-7 PhysicsError 不属冻结面(§5.1)。字段表状态/批准/评审三行同步;§9.2 加消化指针 | Full RFC(**Agent Approved**) |
| v1.2 | 2026-07-31 | **G6.2 实现期矛盾收窄(§4.C4 grep 门)+ S1 JoltC 缺口处置登记**(发现经过与评审口径留痕 §9.1 末段;§4.0-3 口径:修订只经修订行 + §9.1 留痕,不重开 RFC):§4.C4 首条判据字面(「`src/rurix-physics/`(非 sys)零 `rurix_physics_sys` 引用」)与 §4.0-1/§4.0-7 自相矛盾——safe crate 消费 sys crate 是 Approved 架构本身(`jolt` 默认后端构建 gate 经 `dep:rurix-physics-sys` 实现),字面执行使架构不可构建;收窄为原意——`src/rurix-render` 零 `rurix_physics_sys` 引用 + 零原生 Jolt/Rapier 类型名(0-byte 不变);`src/rurix-physics` 公共 API 不透出 sys/原生类型(代码审计),crate 内部 sys 消费收敛于 `src/world.rs` 单一模块(grep 判据:除 `src/world.rs` 外零 `rurix_physics_sys` 引用;原生类型名 `JoltPhysics|JPC_|JPH::|rapier3d` 两 crate 全禁维持);G-G6-3 契约层判据(对外 API safe / 渲染器不持有原生指针)0-byte 不受影响,CI 步骤 88 grep 门同步收窄(ci/physics_core_smoke.py)。S1 JoltC 缺口处置(C-3 三选一,五处全走 (c) 收窄,零 vendor 补丁、未转 amer-koleci/joltc,明细 [VENDOR.md](../src/rurix-physics-sys/VENDOR.md) §3):impulse 首版恒 0(`JPC_EstimateCollisionResponse` 后置)/ activation listener 缺 C 面 → `IsActive` 轮询差分 / 单 broadphase layer 收窄 / `CastRay` 仅最近命中 → 排除循环实现全命中契约 / `DestroyBodies` 上游注释 → `RemoveBodies` + 逐 `DestroyBody` 绕行 | Full RFC(**Agent Approved**) |
| v1.3 | 2026-07-31 | **G6.3 实现期解释性留痕(§4.B4 RemovalReceipt 落点)+ sys 混批重排缺陷修复登记**(留痕 §9.1 末段;§4.0-3 口径:修订只经修订行 + §9.1 留痕,不重开 RFC;正文冻结条文 0-byte):§4.B4(R-3)「`remove_bodies_batch` 返回 `RemovalReceipt`」落地解释为合流层类型——`StreamingBridge::remove_page` 返回 `RemovalReceipt`(与 `PageKey` 绑定,移动语义不可 Clone,编译期不可伪造,先卸 body 再放页),`PhysicsWorld::remove_bodies_batch` 签名维持 G6.2 落地形态 `Result<(), PhysicsError>` 0-byte(页 id 为合流层概念,物理世界不感知页;G6.2 world.rs 注释留痕一致);R-G6-4 纪律语义全额兑现,仅凭据持有点由 world 层解释为合流层。sys 缺陷修复(实施期发现,纯缺陷修复与冻结接口无涉):Jolt `AddBodiesPrepare` 内部 QuickSort 在 >32 体混 broadphase 层批下非稳定重排致返回序/kind 登记/激活三错位(≤32 回退插入排序恒等,G6.2 未暴露);修复 = prepare 前快照 `ids_orig` 按原始序配对,回归测试 `mixed_layer_batch_insert_order_preserved`(35 体混批)先红后绿,零新增 unsafe | Full RFC(**Agent Approved**) |
| v1.4 | 2026-07-31 | **G6.4 实现期收窄(§4.C4 grep 门:rapier3d 原生名收敛 src/rapier.rs 单模块)+ §4.D1 feature 面解释性留痕**(留痕 §9.1 末段;§4.0-3 口径:修订只经修订行 + §9.1 留痕,不重开 RFC;正文冻结条文 0-byte):① §4.C4 grep 判据收窄——`rapier3d` 原生类型名收敛 `src/rurix-physics/src/rapier.rs` 单一 sanctioned 消费模块(镜像 v1.2 sys 消费收敛 `src/world.rs` 先例:Rapier 侧映射在 crate 内实现是 §4.D2 Approved 架构本身,v1.2 字面全禁使 `rapier` feature 档不可构建);Jolt 原生名(`JoltPhysics|JPC_|JPH::`)两 crate 全禁维持(src/rapier.rs 亦不例外),`src/rurix-render` 零原生 Jolt/Rapier 类型名 0-byte 维持;G-G6-3 契约层判据(对外 API safe / 渲染器不持有原生指针)0-byte 不受影响,CI 步骤 88 grep 门同步收窄(ci/physics_core_smoke.py)。② §4.D1「`default-features = false`」解释性留痕——原意 = 纯 Rust 无 CMake、不启用 parallel·simd·serde 重面;实现期发现 rapier3d 0.33 lib target `required-features = ["dim3", "f32"]` 且 math/parry 依赖链需 `std`(零 feature 字面不可编译,84 个 E0432),故落 `default-features = false` + 显式最小集 `dim3`/`f32`/`std`;`parallel`/`simd-stable`/`serde-serialize`/`enhanced-determinism` 维持 off;§4.D1「纯 Rust,无 CMake」与默认 off 核验判据(cargo metadata default 集合不含 `rapier`)0-byte 维持。③ G6.4 实现期留痕(含 rapier.rs 能力差诚实登记摘要:批插逐插 / 事件窄相差分单源合成 / impulse 不比对 / CCD→`CcdEnabled` / 层→`InteractionGroups`≤32 / `job_threads` Jolt 专用被忽略 / `cast_ray` t_min 后过滤 / `cast_shape` 排除循环 / 宽相手动刷新只增改不删)= §9.1 末段 | Full RFC(**Agent Approved**) |
| v1.5 | 2026-07-31 | **G6.5 实现期留痕(Taichi Vulkan AOT spike 成功臂判档 + 并行设备上下文路径解释 + U43 + 步骤 92)**(留痕 §9.1 末段;§4.0-3 口径:修订只经修订行 + §9.1 留痕,不重开 RFC;**§4.E 冻结条文 0-byte 未漂移**):① 成功臂判档——§4.E3 成功最低判据四段闭合(TiRT Vulkan AOT kernel launch 成功 + NdArray 导出 VkBuffer 256B〔64 粒子 × f32〕+ graph external import 消费接线 byte_size=256〔只读引用、不拥有、不入 transient 池,§4.E2 所有权图按字面兑现〕+ readback 64/64 非零、first_values=[1.0,2.5,4.0,5.5] 与 i*1.5+1.0 逐位相等;RTX 4070 Ti 一次真跑即绿,零 tirt 修复),**RD-042 未消费**(成功臂无需失败登记);② 设备路径解释——§4.E2 明示「挂已有渲染设备或并行设备上下文」两径,本臂取并行设备上下文(`TiVulkanRuntimeInteropInfo` 注入 spike 侧自建 Vk 设备上下文),明文允许路径、非能力缺口,同设备首选路径留后续波次;③ U43 = G6.5 TiRT FFI 边界(`src/rurix-rt/src/tirt.rs`,feature `taichi-tirt` 默认 off,13 符号动态装载 taichi_c_api.dll + `run_particles_spike` 全链 + `TirtError` 六变体 fail-closed;`vk.rs` pub(crate) 三件套 cfg gate 既有面 0-byte;rurix-render `#![forbid(unsafe_code)]` 维持零消费)登记 unsafe-audit/rurix-rt.md(沿 U26/U27/U31/U32 审计模式),number_ledger U on_tree_max 42→43 / next_free 43→44;④ CI 步骤 92 `ci/taichi_vulkan_spike_smoke.py`(门 G-G6-6 软门:host 六判据恒跑 + device gate real〔RURIX_REQUIRE_REAL=1 + RURIX_TAICHI_C_API_DLL provisioning,缺则 SKIP=dev-env degrade〕+ --selftest 红绿自检)+ `taichi_vulkan_spike_evidence_schema.json` + check_schemas 注册 + pr-smoke.yml 真步骤(步骤 88~91 0-byte),number_ledger CI_step on_tree_max 91→92 / next_free 92→93;§4.E4 三条禁止经 smoke host 段审计机器闭合(rurix-physics/rurix-render 零 taichi 引用、零 CUDA 主物理路径);g6_budget.json 空壳维持(步骤 88~91 无 counter 体例,92 不加);evidence 两份如实(T160536 真跑 ok=true / T160608 host 态 SKIP ok=false) | Full RFC(**Agent Approved**) |
