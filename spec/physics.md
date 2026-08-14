# physics.md — 物理平台完整期语义面（G9.6 M121/M122 完整期 + M124/M125/M126 + M123 判档登记）

> **地位**：D5 物理「完整期语义」轴事实源——M121 统一 physics particle
> view 完整期（场求值与求解器耦合 + analytic-surface 基元闭集 + 场 journal
> 并入 M66 capture 主流 + World-Field GpuScene 只读扩面）、M122 Gameplay
> Field 完整期（`--phase g9.6` 语义与门序硬约束）、M124 解析浮力（走 Field
> 通道）、M125 Jolt 5.3→5.6 升级 A/B、M126 Rapier 深造对标基准、M123 双
> 通道判档 no-go 诚实登记（RFC-0024 §4.A~§4.E + v1.1 章 F，Agent Approved
> 2026-08-09 / v1.1 增补 2026-08-13；G9_ACCEPTANCE_MAP §2 M121/M122 行 +
> §3 M124/M125/M126 行〔G9.6 波 P1 全进裁决登记，G9_CONTRACT §8.1 裁决①〕
> + G9_CANDIDATE_DECISIONS v1.5 校准注）。G8 已冻结的物理平台面
> （RFC-0021：replay-first 平台、capture/replay、五时间域 identity、Jolt
> 5.3 基线、物理五纪律）与 G9.2 骨架期面（M121/M122 `--phase g9.2` 断言面）
> **字面 0-byte 不动**；本文件只承载 G9.6 物理波新增/完整期语义。
>
> **档位**：Full RFC / RFC-0024（v1.1 章 F 增补）。
>
> **编号**：RXS-0374~0379（G9.6 spec-first，自合入时实测
> `registry/number_ledger.json` `RXS.next_free = 374` 顺位领取，0374~0379
> 连续不跳号；编号永不复用，10 §9.5）。
>
> **新建裁决留痕（G9.6 spec PR）**：G9.6 物理语义面六轴（场求解器耦合 /
> Field 完整期 / 浮力 / Jolt A/B / Rapier 基准 / M123 判档登记）裁定合并
> 新建本卷（D5 物理独立语义轴，沿 G9.2 virtual_geometry.md / G9.4
> global_illumination.md / G9.5 world_partition.md+display_pipeline.md 新建
> 先例）。候选既有卷（rendering_platform.md 的 reflection/capability 面、
> shader_stages.md 的语言类型面、geometry_pages.md 的页 ABI 面）与物理
> 语义轴均不同轴，本体 0-byte（spec/README.md §4 登记 + 本头注留痕）。
>
> **GPU 主刚体禁止线**：Jolt 5.6 GPU compute 接口只评估不接权威（RD-043
> 观察维持，RFC-0024 §4.E1/§8 字面 0-byte）。
>
> **M123 判档边界**：双通道判档 = **no-go 不充绿**（判档硬前置 Jolt 单线程
> 成本 measured 未满足，RFC-0024 v1.1 F1 🔒 修订行落定）——lockstep 通道
> 维持唯一权威通道，`physics-async-decorative` feature 与
> `DecorativePhysicsTickId` 维持「仅判档 go 时生效」字面不启用；本卷
> RXS-0379 只登记 no-go 语义，不授权任何 async 通道建造。

---

## 1. 范围与体例

- 体例 = FLS 风格（spec/README.md §2）；本文件**严禁 UB 节**——物理求解、
  场求值、浮力、vendor 升级与基准测量所有失败均为 typed `Err` / 确定性
  拒绝（fail-closed），不设未定义行为。
- 实现锚定（实现期命名）：M121/M122 完整期落 `src/rurix-physics`（场求值
  器与求解器耦合面、`field/` 模块完整期扩写、`capture/` 主流并入）；M124
  浮力落 `field/` + 新浮力求值面（`physics-buoyancy` feature，RFC-0024
  R-7 🔒 冻结名）；M125 落 `src/rurix-physics-sys` 独立 vendor 线与 layout
  探针工具；M126 落基准 harness（不作 replay oracle）。FFI 确需新增时按
  当时 `U.next_free` 实测顺位登记 unsafe-audit（沿 U33~U42 审计模式）。
- 每条款 ≥1 `//@ spec: RXS-####` 测试锚定（traceability 矩阵全锚定，10 §4）。

## 2. 术语

- **场求解器耦合（力场积分）**：Field 求值输出（力/力矩）经 M121
  `ParticleAdapter` 写路径以 impulse/force 语义耦合进 lockstep 求解器输入；
  tick 内显式序 = 场求值 → impulse 施加 → 求解步进（RFC-0024 §4.A/§4.B）。
- **analytic-surface 基元**：场定义层解析曲面基元最小闭集 = sphere /
  plane / box，提供解析符号距离与梯度采样；浮力水面函数（M124）共用同一
  求值管线（RFC-0024 §4.B1 预留兑现）。
- **场 journal 并入 capture 主流**：persistent field 注册/注销/参数变更
  command journal 并入 M66 capture artifact 主流（同一 capture 目录与
  journal 流），参与 `semantic_state_hash`，replay 逐 tick hash 一致
  （RFC-0024 §4.B2 + RFC-0021 §4.A1）。
- **World-Field GpuScene 只读扩面**：RFC-0024 v1.1 F2 🔒 显式修订行授权
  的 GpuScene 加性只读 buffer 面——按 tick 经 Physics→GpuScene 桥提交场
  采样参数，渲染侧只读消费、零回写（RFC-0019 §8 `GpuScene` 冻结面加性
  修订）。
- **七步程序**：RFC-0021 §4.A4 Jolt 升级评估程序（冻结基线 → 独立 vendor
  → 各自 replay 一致 → canonical A/B → 实测写 budget → 失败臂钉 5.3 →
  采纳臂三件事），逐字不重述。
- **双通道判档**：M123 = lockstep-deterministic（永不异步化）vs
  async-decorative（零回写）架构的启用判档；判档硬前置 = Jolt 单线程成本
  measured（RFC-0024 R-6 🔒/Q1）。
- **`deterministic_profile`**：确定性画像断言面（RFC-0024 R-5 🔒 加性
  扩展 RFC-0021 §4.A1）——固定 seed、固定 dt 锁死、单线程
  （`job_threads` 与画像一致）、无睡眠（sleep 策略钉值）、无 IO、无浮点
  环境变量依赖；画像外运行 fail-closed。

---

## 3. 条款（RXS-0374，G9.6 M121 统一 particle view 完整期）

### RXS-0374 场求值与求解器耦合、analytic-surface 基元闭集、场 journal 并入 capture 主流与 World-Field GpuScene 只读扩面

**Legality**

1. **场求值与求解器耦合（力场积分）**（RFC-0024 §4.A/§4.B；判据逐字引
   G9_ACCEPTANCE_MAP §2 M121 行）：完整期场求值输出（力/力矩）必须经
   五域 `ParticleAdapter` 写路径以 **impulse/force 语义**耦合进
   lockstep-deterministic 求解器输入；tick 内显式序冻结 = 场求值 →
   impulse 施加 → 求解步进；**写路径仅 impulse/force** 结构性断言维持
   （骨架期字面 0-byte），任何直接改写 transform/速度/位置的旁路写注入
   即 RED；力场积分对同输入确定（同输入双运行逐位一致）。
2. **analytic-surface 基元最小闭集**（RFC-0024 §4.B1「为浮力水面函数
   预留」兑现）：场定义层基元加性扩展 `analytic-surface`，最小闭集 =
   **sphere / plane / box** 三形，提供解析符号距离与梯度解析采样；
   闭集外形状首期 fail-closed 拒绝（不静默退化采样）；该基元与 M124
   浮力水面函数**共用同一求值管线**（RXS-0376），禁第二套曲面采样
   实现；基元参数进场定义 digest（图 schema 版本化 + cook 确定性维持）。
3. **场 journal 并入 M66 capture 主流**（RFC-0024 §4.B2 + RFC-0021
   §4.A1；判据逐字引 G9_ACCEPTANCE_MAP §2 M121/M122 行）：persistent
   field 注册/注销/参数变更 command journal 完整期并入 M66 capture
   artifact 主流（同一 capture 目录与 journal 流，不单开第二通道）；
   **capture ↔ field journal 格式往返兼容性断言**——合并后 journal 经
   encode→decode 往返无损，迁移前后逐 tick digest 与 golden 一致、
   journal 全消费无损；journal schema 版本化（承 RFC-0021 §5.1 共同头
   纪律），版本变化显式迁移而非静默重解释。
4. **World-Field GpuScene 只读扩面**（RFC-0024 v1.1 F2 🔒 显式修订行
   前置；判据逐字引 G9_ACCEPTANCE_MAP §2 M122 行）：World-Field 唯一
   出口 = GpuScene 只读 buffer——物理侧按 tick（`PhysicsTickId`）经既有
   Physics→GpuScene 桥提交场采样参数为 GpuScene 承载的只读 buffer；
   **渲染侧只读消费、零回写**；渲染侧对该 buffer 的任何写/回写通道注入
   即 RED，绕过桥的旁路提交注入即 RED（两 RED 臂独立有效）；时间域
   `WorldFieldSampleSet` 归属 `RenderFrameId` 经 `FrameDomainMap` 显式
   映射（R-4 🔒 字面不变）；GpuScene 既有面 0-byte（F2 修订行边界）。
5. **M68 首个 consumer 与单向事实源维持**（判据逐字引
   G9_ACCEPTANCE_MAP §2 M121 行）：M68 damage journal 迁移为首个
   consumer 的迁移前后 digest 一致断言完整期维持；单向事实源纪律
   0-byte；`PhysicsParticleRef` 名义类型编译期隔离断言全真维持。

**Implementation Requirements**

- 实现锚定（实现期命名）：`src/rurix-physics/src/field/`（求值器完整期
  + analytic-surface 基元闭集）与 `src/rurix-physics/src/capture/`（场
  journal 主流并入与往返兼容）；`rurix-physics` 维持 crate 纪律，FFI 零
  新增为本条款预期面。
- RED 锚定计划（实现 PR 落）：旁路写注入 → RED；闭集外 analytic-surface
  形状 → fail-closed；journal 往返篡改/不兼容 → RED；World-Field 渲染侧
  回写注入 → RED；旁路提交注入 → RED。
- 本 spec PR 先行落最小锚定占位语料
  `conformance/physics/accept/field_solver_coupling_minimal.rx`、
  `conformance/physics/reject/world_field_render_writeback.rx`、
  `conformance/physics/reject/field_journal_capture_roundtrip_break.rx`
  （条款锚定占位，inert 锚定口径与转正路径见各文件头注释）；锚点目标
  （实现 PR 转正）= `ci/g9_physics_particle_view_smoke.py --phase g9.6` 与
  `ci/g9_gameplay_field_smoke.py --phase g9.6` 门（symbolic key
  `g9.p0.m121.physics_particle_view` / `g9.p0.m122.gameplay_field`，G9.1
  冻结字面 0-byte 不动）。

---

## 4. 条款（RXS-0375，G9.6 M122 Gameplay Field 完整期）

### RXS-0375 `--phase g9.6` 完整期语义、phase_g9_6_pass 判据与门序硬约束

**Legality**

1. **双 phase 纪律**（判据逐字引 G9_ACCEPTANCE_MAP §2 M122 行）：M122 门
   为 `--phase g9.2`（骨架期）与 `--phase g9.6`（完整期）两条独立调用；
   evidence schema 同时要求 `phase_g9_2_pass=true` 与
   `phase_g9_6_pass=true`；**任一阶段绿色不能替另一阶段充绿**——骨架期
   绿色冒充完整期即 FAIL。
2. **完整期语义面**（RFC-0024 §4.B；判据逐字引 G9_ACCEPTANCE_MAP §2
   M122 行）：完整期 = 场求值实际驱动力学响应（经 RXS-0374 耦合面消费
   impulse/force）——三层解耦 schema 冻结维持；首期 `FieldPhysicsType`
   八枚举逐项 accept GREEN 与非法枚举 RED 维持；**过滤默认空匹配 = 零
   影响**显式断言完整期重验（field 注册但零匹配时世界状态 hash 与无
   field 基线逐位一致）；persistent 注册/注销/变更全 journal 化且 replay
   逐 tick hash 一致完整期重验（消费 RXS-0374 主流并入面）；World-Field
   唯一出口断言按 RXS-0374 L4 修订行面核验。
3. **门序硬约束**（沿 D2-Q7 门序先例的机器阻断）：**M121 完整期
   （`--phase g9.6`）未绿 → M122 完整期（`--phase g9.6`）不得验收**——
   M122 完整期门前置机器核验 M121 完整期最新 evidence 须 `status=="pass"`
   且 `assertion_id=="g9.p0.m121.physics_particle_view"` 且含
   `phase_g9_6_pass=true`；缺失/非 pass 即门 FAIL 退 1（harness 直出件
   不充绿）；本约束为硬门序，不得 waived。

**Implementation Requirements**

- 实现锚定（实现期命名）：`ci/g9_gameplay_field_smoke.py` 完整期腿 +
  门序前置核验（共享小库沿 `ci/g9_gi_interlock.py` 先例）；门 evidence
  双 phase 字段机器核验。
- RED 锚定计划（实现 PR 落）：骨架期 evidence 冒充完整期 → RED；M121
  完整期 evidence 缺失/非 pass 时 M122 完整期调用 → 门 FAIL；过滤默认
  空匹配零影响破坏注入 → RED。
- 本 spec PR 先行落最小锚定占位语料
  `conformance/physics/accept/gameplay_field_full_phase_minimal.rx`
  （条款锚定占位，inert 锚定口径与转正路径见文件头注释）；锚点目标
  （实现 PR 转正）= `ci/g9_gameplay_field_smoke.py --phase g9.6` 门
  （symbolic key `g9.p0.m122.gameplay_field`，G9.1 冻结字面 0-byte 不动）。

---

## 5. 条款（RXS-0376，G9.6 M124 解析浮力模型）

### RXS-0376 解析浮力走 Field 通道、禁旁路 API、corpus fixture 与确定性断言

**Legality**

1. **走 Field 通道（硬判据）**（RFC-0024 §4.D 逐字；判据逐字引
   G9_ACCEPTANCE_MAP §3 M124 行）：水体区域 = persistent field，解析水面
   函数为场定义层 `analytic-surface` 基元（RXS-0374 L2 求值管线共用）；
   `FieldPhysicsType::Buoyancy` 语义；浮力是 Field 统一抽象的**第二个
   真实用户**（第一个是 destruction damage）——浮力不得长成第二套空间
   影响管线；浮力求值器消费场密度/速度参数（水面函数与介质参数为场
   定义的一部分，进 digest）。
2. **求值语义**：每 tick 对落入 filter 的 `PhysicsParticleRef`（首期
   RigidBody 域）计算 clipped 浸入体积与浸没质心 → `buoyancy impulse +
   buoyancy torque + linear/angular drag impulse`，经既有 `AddForceAtPoint`
   类 FFI 施加（消费既有导出符号纪律不变）；**形状支持分层**——首期
   convex/primitive 解析 clip；任意 mesh 走离线预计算 voxelized volume
   table（cooked artifact，版本化）。
3. **旁路 API 注入即 RED（RED 臂独立有效）**（判据逐字引
   G9_ACCEPTANCE_MAP §3 M124 行）：任何不经 Field 通道直接写速度/位置/
   transform 的浮力旁路 API 注入即 RED（旁路即门红）；该负例臂独立于
   正例臂成立，臂失效（旁路注入不红）即漏检，本条款整体 FAIL。
4. **corpus fixture 与确定性断言**（判据逐字引 G9_ACCEPTANCE_MAP §3
   M124 行）：**细长体/翻滚体 corpus fixture**（canonical 场景 + 输入
   参数锚定语料）入 capture/replay corpus（M66 设施挂接点）；
   **capture→replay 逐 tick hash 一致 + 变帧率输入同 tick 结果逐位
   一致**（determinism 断言）；固定 dt + 解析水面函数——**禁帧率相关
   插值、禁墙钟相位**；全部输入/输出进 command journal。
5. **M49 联动维持 defer**（RFC-0024 §4.D 逐字）：Taichi AOT 只产出
   粒子/体积场（纪律 4 字面不变），水面视觉可消费 World-Field 通道，
   但**浮力权威不经 Taichi**；真双向流体-刚体耦合排除主线（RFC-0024
   §8 红线 0-byte）。

**Implementation Requirements**

- 实现锚定（实现期命名）：`src/rurix-physics/src/field/` 浮力求值面
  （`physics-buoyancy` feature，RFC-0024 R-7 🔒 冻结名；功能未编译时
  返回 `FeatureNotCompiled` 类错误，不静默退化成视觉-only 成功）；
  voxelized volume table 离线预计算走资产管线 cooked artifact 通道。
- RED 锚定计划（实现 PR 落）：旁路 API 注入 → RED；帧率相关插值/墙钟
  相位注入 → 变帧率逐位一致破坏可检测 → RED；闭集外形状未走 voxel
  table → fail-closed。
- 本 spec PR 先行落最小锚定占位语料
  `conformance/physics/accept/buoyancy_field_channel_minimal.rx` 与
  `conformance/physics/reject/buoyancy_bypass_api_injection.rx`
  （条款锚定占位，inert 锚定口径与转正路径见各文件头注释）；锚点目标
  （实现 PR 转正）= `ci/g9_buoyancy_field_channel_smoke.py` 门
  （symbolic key `g9.p1.m124.buoyancy_field_channel`，G9.6 波 P1 全进
  裁决登记字面不动）。

---

## 6. 条款（RXS-0377，G9.6 M125 Jolt 5.3→5.6 升级 A/B）

### RXS-0377 七步程序逐字执行、独立 vendor 并存、新摩擦模型实测与两臂诚实登记

**Legality**

1. **七步程序逐字**（RFC-0021 §4.A4 + RFC-0024 §4.E1；判据逐字引
   G9_ACCEPTANCE_MAP §3 M125 行）：① 冻结 5.3 基线（corpus / 资产 cook
   digest / CCD / contact / query 结果 / measured baseline）→ ② 5.6
   **独立 vendor/ABI 构建，不覆盖 5.3 基线** → ③ 两版本各自证明同版本
   capture/replay 逐 tick 一致 → ④ 相同 canonical source asset / input
   journal A/B → ⑤ 性能阈值只从真实采样写入 budget、版本锚按实测
   tag/commit 登记 → ⑥ **失败臂**：任一硬门失败正式钉住 5.3、记录失败
   证据、不得伪写 5.6 PASS → ⑦ **采纳臂三件事**：corpus 显式迁移并保留
   5.3 基线 artifact + replay 门在新版本重跑落 evidence + 判据字面经
   修订后才改版本号。七步执行记录完整（逐步留痕）。
2. **独立 vendor 并存（RED 臂独立有效）**：5.6 vendor 线必须独立并存
   ——任何覆盖/替换 5.3 基线 vendor 的注入即 RED；5.3 基线 artifact
   （corpus digest 与 measured baseline）在评估全程保持可复算。
3. **新摩擦模型重点实测**：5.6 新摩擦模型（平均接触点）为 A/B 重点项
   ——消除接触点序偏向对确定性 corpus 有直接价值；求解器语义变化
   **逐字段 exact / tolerance / invariant 分类**（§4.A4 程序），未分类
   字段不得默认同性。
4. **GPU compute 只评估不接权威（RED 臂独立有效）**：Jolt 5.6 GPU
   compute shader 接口**只评估不接权威**（GPU 主刚体禁止线 0-byte，
   RD-043 观察维持）——任何把 GPU compute 接为权威求解路径的提案/
   接线注入即 RED；评估报告留档，接入须 RD-043 + 矩阵 §12 + 独立
   Full RFC。
5. **layout 探针工具化**：所有 `*Settings` 结构 sizeof/offsetof 静态
   断言重跑纳入 vendor 升级/新 FFI 检查单固定项；探针源码入库，不再
   散落工作树。
6. **两臂诚实登记**：采纳与失败都是正式终态——两臂（采纳三件事 /
   失败钉 5.3）诚实登记，**禁写 5.6 PASS 伪绿**；G9 契约判据字面若再
   钉「Jolt 5.3」，同样须修订后才可改字面。

**Implementation Requirements**

- 实现锚定（实现期命名）：`src/rurix-physics-sys` 独立 vendor 线
  （5.3 基线 `VENDOR.md` pin 字面不动）+ layout 探针工具入库；FFI 新增
  按当时 `U.next_free` 实测顺位登记 unsafe-audit（沿 U33~U42 审计
  模式）。
- RED 锚定计划（实现 PR 落）：5.6 vendor 覆盖 5.3 基线注入 → RED；
  GPU compute 接权威注入 → RED；失败臂伪写 5.6 PASS → 门 FAIL。
- 本 spec PR 先行落最小锚定占位语料
  `conformance/physics/accept/jolt_ab_seven_step_minimal.rx` 与
  `conformance/physics/reject/jolt_56_vendor_overwrite_baseline.rx`
  （条款锚定占位，inert 锚定口径与转正路径见各文件头注释）；锚点目标
  （实现 PR 转正）= `ci/g9_jolt_56_ab_evaluation_smoke.py` 门（symbolic
  key `g9.p1.m125.jolt_56_ab_evaluation`，G9.6 波 P1 全进裁决登记字面
  不动）。

---

## 7. 条款（RXS-0378，G9.6 M126 Rapier 深造对标基准）

### RXS-0378 同场景同输入同 determinism 画像 A/B 夹具、measured 报告与 RD-044 字面不变

**Legality**

1. **对标基准 A/B 夹具**（RFC-0024 §4.E2；判据逐字引
   G9_ACCEPTANCE_MAP §3 M126 行）：Rapier 深造对标面 = 新 Dynamic BVH /
   sparse voxel collider / persistent islands / manifold ≤4 / 简化摩擦
   模型，大堆叠场景建 A/B benchmark 夹具——**与 Jolt 同场景、同输入、
   同 determinism 画像**（同一 canonical 场景资产、同一输入 journal、
   同一 `deterministic_profile` 画像断言面）。
2. **measured 报告（evidence 非空）**：产出 measured 报告，含跨 solver
   **确定性偏差统计**——跨 solver 不承诺逐位，只作不变量/容差对拍
   （RFC-0021 §7 备选 D）；性能数字只从真实采样写入（measured_local
   纪律，禁 estimated）。
3. **基准不作 replay oracle（RED 臂独立有效）**：基准输出**不得**充当
   capture/replay 的逐位对拍 oracle——以基准输出冒充 replay oracle 的
   注入即 RED；replay 对拍唯一权威 = 同 solver 同版本 capture/replay
   逐 tick hash（RFC-0021 §4.A1 字面不变）。
4. **glam 迁移兼容留档**：Rapier 0.32+ glam 化对既有快路径封装的 API
   冲击评估与兼容层设计留档；不承诺 bitwise 不变。
5. **RD-044 字面不变**：「快路径被真实 workload 采用时」字面 0-byte——
   基准显示 D5 真实 workload 上 measured 优势才按 RD-044 程序申请深造
   判档（逐项独立判档 10 §3），否则维持 no-go 留档；本条款只产基准
   报告，**不升格深造、不作验收依赖与生产默认**。

**Implementation Requirements**

- 实现锚定（实现期命名）：基准 harness（`rapier` feature 默认 off 纪律
  维持；rapier3d `=0.33.0` pin 变更须随 A/B 程序留痕）；报告 evidence
  落 `evidence/` 新文件不覆盖既有件（只增不删不改）。
- RED 锚定计划（实现 PR 落）：基准输出充当 replay oracle 注入 → RED；
  determinism 画像不一致（两臂画像漂移）注入 → RED；无 measured 数据
  的深造判档申请 → fail-closed。
- 本 spec PR 先行落最小锚定占位语料
  `conformance/physics/accept/rapier_benchmark_ab_fixture_minimal.rx` 与
  `conformance/physics/reject/rapier_benchmark_as_replay_oracle.rx`
  （条款锚定占位，inert 锚定口径与转正路径见各文件头注释）；锚点目标
  （实现 PR 转正）= `ci/g9_rapier_benchmark_ab_smoke.py` 门（symbolic
  key `g9.p1.m126.rapier_benchmark_ab`，G9.6 波 P1 全进裁决登记字面
  不动）。

---

## 8. 条款（RXS-0379，G9.6 M123 双通道判档 no-go 诚实登记）

### RXS-0379 双通道判档 no-go 登记语义、deterministic_profile 断言面维持与承接锚

**Legality**

1. **判档 no-go 诚实登记**（RFC-0024 v1.1 F1 🔒 修订行落定；Q1「测量
   不足 → 维持 M75 no-go 留档」字面）：M123 双通道判档 = **no-go 不充
   绿**——判档硬前置（Jolt 单线程成本 measured）未满足：树内零
   measured artifact（evidence/ 物理件零单线程成本字段、
   `g9_budget.json` 无物理段 counter），测量归实现波 D5 先行任务 P-6
   （measured_local 真跑，禁 estimated）。本条款**证据非空但
   `counts_as_green=false`**——判档留痕与画像断言面登记不构成 M123
   绿色，no-go 项不入 G9_ACCEPTANCE_MAP §3（「no-go/defer 项不入本表」
   纪律）。
2. **通道生效面**：lockstep-deterministic 维持**唯一**权威通道（永不
   异步化字面不变）；async-decorative 通道不建造、不启用——
   `physics-async-decorative` feature 与 `DecorativePhysicsTickId` 维持
   「仅 M123 判档 go 时生效」字面（R-4/R-7 🔒）；**判档 go 前任何
   async 通道启用/feature 接线注入即 RED**（诚实登记期的负例臂，
   独立有效）。
3. **`deterministic_profile` 断言面维持**（R-5 🔒 加性扩展 RFC-0021
   §4.A1，不随 no-go 撤销）：lockstep 通道启动与 corpus 运行前断言
   画像五件——**固定 seed、固定 dt 锁死、单线程（`job_threads` 与画像
   一致）、无睡眠（sleep 策略钉值）、无 IO、无浮点环境变量依赖**；
   画像外运行 fail-closed；负例 RED 臂维持——**跳 seed / 多线程 /
   睡眠注入**三臂必须被 corpus 拒绝（fail-closed 断言独立布尔）。
4. **承接锚与重判纪律**：承接锚 = **G9.7 P2 穷举**（G9_PLAN §2 G9.7
   候选行集已列「M123/M126（若判档不成立）」字面）；实现波 Jolt 单
   线程成本 measured 数据落地后按只追加程序重判（RFC-0024 修订行 +
   G9_ACCEPTANCE_MAP §1 程序），本条款字面不改写；测量显示主线程物理
   超预算 → 采纳双通道（`physics-async-decorative` 生效、
   `DecorativePhysicsTickId` 生效），测量不足 → 本登记维持。

**Implementation Requirements**

- 实现锚定（实现期命名）：`deterministic_profile` 并入 capture header
  determinism 画像（RFC-0021 §4.A1 扩展面，`src/rurix-physics/src/
  capture/header.rs` `DeterminismProfile` 既有面加性）；画像断言与三
  RED 臂归实现波 corpus 门接线；async 通道零建造。
- RED 锚定计划（实现 PR 落）：判档 go 前 async 通道启用注入 → RED；
  跳 seed / 多线程 / 睡眠注入 → fail-closed（三臂独立）；以画像断言
  登记冒充 M123 绿色 → 门 FAIL。
- 本 spec PR 先行落最小锚定占位语料
  `conformance/physics/reject/async_decorative_channel_without_verdict.rx`
  （条款锚定占位，inert 锚定口径与转正路径见文件头注释）；本条款不立
  symbolic gate key（no-go 不入表），锚点目标（重判后实现 PR 转正）
  = G9.7 穷举决策行 + RFC-0024 修订行只追加程序。

---

## 9. 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-08-13 | G9.6 物理波 spec-first 新建（硬规则 7 条款先行）：RXS-0374（M121 完整期场求解器耦合 + analytic-surface 基元闭集 + 场 journal 并入 M66 capture 主流 + World-Field GpuScene 只读扩面〔RFC-0024 v1.1 F2 🔒 修订行前置〕）/ RXS-0375（M122 完整期 `--phase g9.6` 语义 + phase_g9_6_pass 判据 + 门序硬约束〔M121 完整期未绿 M122 完整期不得验收〕）/ RXS-0376（M124 解析浮力走 Field 通道 + 旁路 API 注入即 RED + 细长/翻滚 corpus fixture + capture→replay 逐 tick hash + 变帧率逐位一致）/ RXS-0377（M125 Jolt 5.3→5.6 A/B 七步逐字 + 独立 vendor 并存不覆盖 5.3 基线 + 新摩擦模型重点实测 + GPU compute 只评估不接权威 + 两臂诚实登记禁伪绿）/ RXS-0378（M126 Rapier 对标基准同场景同输入同 determinism 画像 A/B + measured 报告含确定性偏差统计 + 不作 replay oracle + RD-044 字面不变）/ RXS-0379（M123 双通道判档 no-go 诚实登记：证据非空但 counts_as_green=false + deterministic_profile 断言面维持 + 三 RED 臂 + 承接锚 G9.7 穷举），条款号自 ledger 实测 `RXS.next_free=374` 顺位领取（0374~0379 连续不跳号）。依据 [RFC-0024](../rfcs/0024-physics-platform-revision.md)（Agent Approved 2026-08-09；v1.1 章 F 增补 2026-08-13）§4.A~§4.E + 章 F + G9_ACCEPTANCE_MAP §2 M121/M122 行 + §3 M124/M125/M126 行（判据逐字；G9.6 波 P1 全进裁决，G9_CONTRACT §8.1 裁决①）+ G9_CANDIDATE_DECISIONS v1.5 校准注。零新 RX 码；零 src/ 改动、零 workflow 步骤、零新 U/RD/SG/MR/RFC/CI_step；conformance 最小锚定语料十一件（conformance/physics/accept 五件：field_solver_coupling_minimal.rx / gameplay_field_full_phase_minimal.rx / buoyancy_field_channel_minimal.rx / jolt_ab_seven_step_minimal.rx / rapier_benchmark_ab_fixture_minimal.rx；reject 六件：world_field_render_writeback.rx / field_journal_capture_roundtrip_break.rx / buoyancy_bypass_api_injection.rx / jolt_56_vendor_overwrite_baseline.rx / rapier_benchmark_as_replay_oracle.rx / async_decorative_channel_without_verdict.rx；inert + `//@ spec` 锚定 + 预期诊断注释 + 转正路径旁注，G9.2~G9.5 spec 波先例）同 PR 落；symbolic key `g9.p0.m121/m122.*`（G9.1 冻结字面）与 `g9.p1.m124/m125/m126.*`（G9.6 波 P1 登记）0-byte 不动；trace_matrix 重生成 CRLF 字节纪律维持；stable 快照因条款计数 355→361 同 PR 重 bless（RXS-0180 L2 加性演进）。既有 spec 条款字面 0-byte（只追加新卷），不触红线/禁区 | **Full RFC**（RFC-0024） |
