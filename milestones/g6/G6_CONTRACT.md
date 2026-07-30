---
# 里程碑契约(14 §1 四要素;g6 = 渲染物理双轨期,承 TEMPLATE_CONTRACT.md 体例)
contract: G6
title: G6 渲染物理双轨期——依 G6_PLAN 择优裁决落地引擎物理主线:rurix-physics 物理库(Jolt 生产默认:固定步/睡眠/批插体/CCD/并发查询/接触事件)+ 与 G5 渲染器合流(GpuScene 单向变换桥 + 动态体 MV 供时域底座 + 流送页驻留驱动 body 批插移除 + TLAS/BLAS refit 复用)+ Rapier 快路径 feature 对拍 + Taichi Vulkan AOT 特效副轨 spike,合流 demo device 真跑
status: active            # active(2026-07-30 开工:G5 close-out(G5_CONTRACT §8.1)+ owner 立项确认〔G6_PLAN v1.0 定稿后的 G6.1 治理包开工指令,2026-07-30 会话下达〕,§7 ①)→ closed(close-out 只追加 §8,上方条款 0-byte)
version: v1.0
date: 2026-07-30
timebox: "约 8–12 周(主线 G6.1→G6.6 波次推进,见 G6_PLAN.md;周为相对刻度,非日历承诺)"
rfc_required: RFC-0017    # 单伞形 Full RFC(G4 RFC-0015 / G5 RFC-0016 单伞形先例):五章——A 物理库边界(PhysicsWorld 固定步 step(dt_fixed)/BodyDesc/BodyId·ShapeId 不透明句柄/ContactEvent 有界队列/QueryRay 并发查询/SyncBudget,G6_PLAN §2.1 冻结接口草案字面化)/ B 渲染同步契约(G6_PLAN §2.2/§2.3:单向事实源·查询并行·流送同构·特效隔离·库不进语言五条纪律 + G5 冻结面 0-byte 边界)/ C FFI 与 unsafe 纪律(Jolt 绑定选型裁决 R-G6-1:jolt-rust/rolt vs 自维护 JoltC FFI;unsafe 集中绑定 crate,U33 起续号;rurix-render 维持 forbid(unsafe_code))/ D Rapier 快路径(feature `rapier` 默认 off,同场景 host 对拍容差判据,CI 无 CMake 路径)/ E Taichi Vulkan AOT 特效副轨(粒子/体积场 buffer 经 graph external import,失败诚实登记 RD)。物理为引擎库(06 §8.3「库不进语言」),预期零新语言语义条款
upstream_docs:
  - "milestones/g6/G6_PLAN.md v1.0(2026-07-30 定稿,本期范围/择优裁决/波次/冻结接口草案的上游事实源;多项择优:Jolt 主物理 / Rapier 快路径 / Taichi Vulkan AOT 特效副轨 / Newton 系研究隔离 / GPU 主刚体否决)"
  - "milestones/g5/G5_CONTRACT.md §8.1(G5 close-out:rurix-render graph/geometry/shadow/gi/rt/material/streaming/temporal + GpuScene/PageRequest/temporal MV/AS 管理器全量落地;RD-037/038 条件臂存续)+ rfcs/0016-native-renderer.md(渲染主线 Vulkan / fail-closed 能力查询)"
  - "src/rurix-rt/src/backend.rs(compute 双后端纪律:BackendKind::Cuda|Vulkan,RURIX_BACKEND 显式选择,缺驱动确定性 Err 绝不静默回退——推论:主物理与图形/compute 后端正交,CPU 库)"
  - "物理选型调研(会话调研 + Cursor canvas physics-engine-match,G6_PLAN §0.1 择优表已锁定)"
  - "06 §8.3(库不进语言)/ 02 §2 U5(引擎旗舰用例)/ 13 D-130(窗口/输入不进语言红线)/ D-406 v2.0(agent 完全自主)/ D-409(Full RFC 跨模型对抗性评审)/ 04 P-01(strict-only)/ P-09(证据压过进度)/ P-12(克制压过完整性)"
  - "14 §1 §3 §4 §5(契约/预算零占位/deferred/证据分级)/ 10 §3(变更三档)§9.5(编号永不复用)/ agents/AGENTS.md(硬规则十条)"
in_scope:
  - g6_governance           # G6.1 治理包:本契约四件套 + number_ledger reserved_in_flight[G6] 登记(v1.28);结构件,零语义实现
  - umbrella_rfc_0017       # G6.1 伞形 Full RFC-0017:Draft → D-409 跨模型对抗性评审(评审 provenance ≠ 起草 provenance)→ Agent Approved 先于实现 PR
  - physics_core_jolt       # G6.2 物理库底座:新 crate rurix-physics——Jolt 集成生产默认(世界/层/固定步/睡眠/批插体/CCD 开关/job 系统接宿主线程池)+ 查询面(ray/shape cast/overlap,并发 query)+ 事件面(接触 Begin/Persist/End 有界队列 + SyncBudget)+ FFI unsafe 集中审计 + host 单测(堆叠沉降/睡眠唤醒/批插体不锁死主步/query 与 step 并发烟测)
  - render_confluence       # G6.3 与渲染合流:PhysicsTransform → GpuScene::update_transform/flush_dirty 单向同步桥 + 动态体 MV 供时域底座(静态/睡眠体零 MV)+ 流送页驻留→批插入/卸载→移除(与 PageRequest 同帧预算)+ TLAS/BLAS 变换脏实例走 G5 refit 分级(不新建所有者)+ 合流 demo(uc06 扩展或 uc0x-physics)
  - rapier_fast_path        # G6.4 Rapier 快路径:feature `rapier` 同 PhysicsWorld 抽象第二后端(默认 off)+ 同场景 host 对拍门(变换/接触集合容差断言,非跨引擎逐位)+ CI 无 CMake 路径可跑;不替换生产默认
  - taichi_vulkan_spike     # G6.5 Taichi Vulkan AOT 特效 spike(可选交付):AOT 模块 + TiRT 挂 Vk 设备,粒子/体积场 buffer → graph external import;失败/能力缺口诚实登记 RD(自 RD-042 顺位),不阻塞 G6.3/G6.4 硬门
  - g6_closeout             # G6.6 close-out:全量回归冻结 + 门终审表 + RD/SG 处置 + status flip
out_of_scope:
  - gpu_rigid_body_main     # GPU 主刚体(PhysX CUDA 刚体/wgrapier 生产依赖/Warp-Newton 主环):游戏刚体预算走 CPU 多核,Vulkan 车道留给 G5 效果面(G6_PLAN §0.1 否决行 + §0.4 禁止线)
  - commercial_havok        # 商用 Havok:许可与可审计引擎库路线冲突
  - softbody_fluid_hard_gates # 软体/布料/流体进硬门:Jolt CPU 软体与 Taichi MPM 仅 spike/副轨,不进 G6 硬门
  - research_track_main_ci  # Newton / Genesis / MuJoCo Warp 合入主仓 CI:研究隔离,独立仓库或 feature 永不默认
  - dxil_window_input_language # DXIL RT 腿(RD-034 blocked 维持,本期物理不依赖 DXIL);窗口/输入进语言(D-130 红线)
  - perf_budget_hard_gates  # 性能数字进硬门:measured 写 evidence(P-09),预算条目随实现回填,机制正确性优先
  - production_adoption     # 引擎采纳/下载量/用户数宣称(carve-out 沿 MS1/EA1/EI1/G4/G5 先例)
deferred_refs: [RD-034, RD-036, RD-037, RD-038]    # 维护对象(本期不兑现,RD-038 分波兑现按其自身轨道演进不由 G6 承接)。执行期新 RD 自 RD-042 起(以合入时 deferred.json 实际为准)
deliverables:
  - id: D-G6-1
    name: G6.1 治理包四件(本契约 + G6_PLAN 升格 + CI_GATES + g6_budget.json 空壳)+ number_ledger reserved_in_flight[G6] 登记(v1.28,含 G5 期滞后字段校准)
  - id: D-G6-2
    name: G6.1 RFC-0017 伞形五章(Draft→跨模型对抗性评审→Agent Approved 先于实现)
  - id: D-G6-3
    name: G6.2 物理库底座——rurix-physics crate(Jolt 生产默认:世界/固定步/睡眠/批插体/并发查询/接触事件/SyncBudget)+ FFI unsafe 集中审计(U33 起)+ host 单测齐全 + 固定步确定性烟测
  - id: D-G6-4
    name: G6.3 渲染合流——GpuScene 单向变换桥 + 动态体 MV + 流送 body 批插移除 + TLAS refit 复用 + 合流 demo(刚体场景 + 既有 VisBuffer/GI/VSM/TAA 管线真跑)
  - id: D-G6-5
    name: G6.4 Rapier 快路径——feature `rapier` 第二后端(默认 off)+ 同场景 host 对拍门(容差断言,CI 无 CMake 路径)
  - id: D-G6-6
    name: G6.5 Taichi Vulkan AOT 特效 spike(可选交付:粒子/体积场 external import 或失败 RD 登记,不阻塞硬门)
  - id: D-G6-7
    name: G6.6 close-out 终审(全量回归冻结 + 门终审表 + RD/SG 处置 + status flip)
acceptance_gates:
  - id: G-G6-1
    check: "治理门:契约四件套合入(milestones/g6/ 四件,结构件零语义实现);number_ledger reserved_in_flight[G6] 登记(v1.28)且 `py -3 ci/check_number_ledger.py` PASS;check_schemas / check_structure PASS"
  - id: G-G6-2
    check: "RFC 门:RFC-0017(伞形五章)Agent Approved 合入先于实现 PR;D-409 对抗性评审完成——评审 provenance ≠ 起草 provenance,逐条 finding disposition 落 RFC 对抗性评审记录段;G6_PLAN §2 冻结接口草案经 RFC Approved 后字面冻结,实现 PR 不得漂移"
  - id: G-G6-3
    check: "物理底座门:rurix-physics Jolt 路径固定步确定性烟测(同输入同输出,平台内)+ host 单测齐全(堆叠沉降/睡眠唤醒/批插体不锁死主步/query 与 step 并发烟测/ContactEvent 有界 drain/SyncBudget 每帧重置);全 workspace cargo build/test 绿;FFI 全部 // SAFETY: + unsafe-audit U33 起登记,对外 API safe,渲染器不持有原生 Jolt/Rapier 指针(BodyId/ShapeId 不透明句柄,代码审计)"
  - id: G-G6-4
    check: "合流门:PhysicsTransform→GpuScene::update_transform/flush_dirty 单向同步正确性 host 恒跑(渲染器不回写物理,代码审计);动态体 MV 供时域底座、静态/睡眠体零 MV(禁效果 pass 私写重投影维持);页驻留→批插入/卸载→移除与 PageRequest 同帧预算单测(含 R-G6-4 卸载竞态注入:先卸 body 再放页);TLAS/BLAS 变换脏实例走 G5 refit 分级不新建第二套所有者(代码审计);device gate real(Vulkan)像素/变换非平凡断言(RURIX_REQUIRE_REAL=1);G5 步骤 82~87 与既有步骤 41~81 零回归"
  - id: G-G6-5
    check: "Rapier 对拍门:feature `rapier` 默认 off(cargo metadata 核验);同 PhysicsWorld 抽象第二后端同场景 host 对拍——变换/接触集合容差断言(非跨引擎逐位,R-G6-2 口径)全过;CI 无 CMake 路径可跑(纯 host 恒跑);文档明示「快路径 ≠ 性能/稳定性默认」"
  - id: G-G6-6
    check: "特效 spike 门(软门,不阻塞 G-G6-3/4/5):Taichi Vulkan AOT 粒子或体积场 buffer 经 graph external import 进管线 device 见证,或失败/能力缺口诚实登记 RD(自 RD-042 顺位)+ 契约 §8 留痕;禁止用 Taichi 替代主刚体/绑确定性联网/在 CUDA 后端另起主物理(代码审计)"
  - id: G-G6-7
    check: "demo 门:合流 demo(uc06 扩展或 uc0x-physics)刚体场景 + 既有 VisBuffer/GI/VSM/TAA 管线 device 真跑 exit 0(RURIX_REQUIRE_REAL=1)+ readback 像素非平凡断言 + 物理步耗时 measured 写 evidence(数字不进硬门,P-09);CI smoke 步骤 88 起 host 段恒跑全绿;evidence JSON 过 check_schemas"
  - id: G-G6-8
    check: "收口门:close-out `budget_eval.py --strict` 全局零 estimated;全量回归冻结真实输出追加 §8(fmt/clippy/test/trace/schemas/structure/number_ledger + 新步骤真跑 + 既有步骤 41~87 零回归);执行期 RD-042+ 登记齐全;status active→closed"
guardrails:
  - "milestones/m0~g5 的 measured_local 既有预算条目 git diff 0-byte;g6_budget.json 经 *_budget.json glob 自动纳入 + 命名空间强制前缀 g6.(14 §3);counter/entries 不预造(与 ci/budget_eval.py evaluator 分支同实现 PR 落);全程零 estimated;永不立引擎采纳/下载量/用户数类条目"
  - "milestones/m0~g5 的 *_CONTRACT.md(closed)只追加不修改"
  - "registry/deferred.json 与 spike_gating.json 只追加;RD-016/028 跳号永不复用;SG-010 留续号维持;number_ledger 只追加纪律;严禁把 G6 earmark 写进 shadow_reserved"
  - "evidence/ 只增不删不改;00–14 共 15 份规划文档不被执行 PR 改写(check_planning_docs)"
  - "src/ 新 unsafe 全部 // SAFETY: + unsafe-audit U33 起续号登记;FFI 集中 rurix-physics(及绑定子 crate);rurix-render 维持 #![forbid(unsafe_code)];渲染器不持有原生物理指针"
  - "G5 冻结面 0-byte:MaterialClosure 32B / VisBuffer 位格式 / Barrier EB 三轴 / PageRequest 字段布局不改(G6_PLAN §2.2);物理只写 GpuScene 变换脏集 + 可选 MV 缓冲;流送只订阅页驻留/卸载通知不重实现 StreamingBudget 计量"
  - "主物理 CPU 正交纪律:禁 GPU 主刚体、禁物理 sim 上渲染队列与 VisBuffer/VSM/GI/RT 抢车道;特效副轨仅 Vulkan AOT external import,不引入第二套 CUDA 物理 runtime 作主环(G6_PLAN §0.2/§0.4/R-G6-5)"
  - "既有零回归不变量:dxil 套件恒定 / vulkan 套件 grow-only / 步骤 41~87 既有判据 0-byte 只增(步骤 70 = G3 showcase 永久 gap 维持;步骤 69 blocked 探针恒跑 RD-034;步骤 84~86 device 段 RD-038 分波探针按其自身轨道演进,G6 不改写)"
  - "device 见证纪律:RURIX_REQUIRE_REAL=1;缺 provisioning 环境 SKIP = dev-env degrade,mock/SKIP 不得充绿"
  - "仓库 LF byte-exact(* -text):新文件 LF + 尾换行;提交前逐文件字节核 CR"
  - "本契约既有条款 0-byte,close-out 只追加 §8;status 翻转/RD·SG 处置由 agent 自主签署;milestones/g5 既有条款 0-byte 只追加引用"
---

# G6 契约 — 渲染物理双轨期

> 所属:[../../02_USERS_AND_USE_CASES.md](../../02_USERS_AND_USE_CASES.md) §2 U5 + [../../06_GPU_GRAPHICS_PROGRAMMING_MODEL.md](../../06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) §8.3 / 契约机制见 [../../14_ENGINEERING_DISCIPLINE.md](../../14_ENGINEERING_DISCIPLINE.md) §1。
> 上游事实源:**[G6_PLAN.md](G6_PLAN.md) v1.0**(2026-07-30 定稿)——多项择优裁决(§0.1)/ 仓内后端纪律推论(§0.2)/ G5 合流点(§0.3)/ 冻结接口草案(§2)/ out-of-scope(§3)/ 风险登记(§4)。
> 基准 ref:**默认 `g4-closed`**(`g5-closed` tag 未落,G5 收口以 [G5_CONTRACT.md](../g5/G5_CONTRACT.md) §8.1 为准;PR 路径以 `GITHUB_BASE_REF` 为准)。
> **定位口径:G6 把「rurix 引擎拥有生产级物理主线」从选型调研结论推进到 measured 工程事实。**现状(G5 close-out 已核):渲染器八面(graph/geometry/shadow/gi/rt/material/streaming/temporal)已交付,`GpuScene`/`PageRequest`/时域 MV/AS 管理器为本期合流消费面;物理模块在仓库零存在。G6 新建 `src/rurix-physics` 引擎物理库(Jolt 生产默认 + Rapier feature 快路径)+ 合流桥 + demo,按 G6_PLAN 波次全量落地。「双轨」的诚实边界:主物理 CPU 库与渲染 Vulkan 车道正交;Taichi 特效副轨为可选交付,失败登记 RD 不算失败伪装;性能数字 measured 写 evidence 不进硬门。
> **治理口径:agent 完全自主(D-406 v2.0 / AGENTS v3.0 硬规则 1)。**
> **脚手架口径:本契约为 G6.1 开工结构件,不实现任何语义面;§8 close-out 开工时为空。**

---

## 1. 目标

G6 期结束时项目获得:① **物理库底座**——`rurix-physics` 引擎库(Jolt 生产默认:固定步/睡眠/批插体/CCD/job 系统接宿主线程池;查询面并发 ray/shape cast/overlap;接触事件有界队列 + `SyncBudget`),对外全 safe API;② **渲染合流**——`PhysicsTransform` → `GpuScene` 单向变换桥、动态体 MV 供时域底座、流送页驻留驱动 body 批插移除、TLAS/BLAS 走 G5 refit 分级;③ **Rapier 快路径**——feature `rapier` 第二后端(默认 off)+ 同场景 host 对拍门;④ **Taichi Vulkan AOT 特效副轨 spike**(可选交付);⑤ **合流 demo** device 真跑 + CI smoke + evidence;⑥ 收口。

## 2. 范围

### 2.1 in-scope

| 项 | 说明 | 对应计划波次 | 对应交付物 |
|---|---|---|---|
| g6_governance | G6.1 治理包 | G6.1 | D-G6-1 |
| umbrella_rfc_0017 | G6.1 伞形 Full RFC 五章 | G6.1 | D-G6-2 |
| physics_core_jolt | 物理库底座(Jolt 默认) | G6.2 | D-G6-3 |
| render_confluence | 渲染合流(桥/MV/流送/AS/demo) | G6.3 | D-G6-4 |
| rapier_fast_path | Rapier 快路径 feature | G6.4 | D-G6-5 |
| taichi_vulkan_spike | Taichi Vulkan AOT 特效 spike(可选) | G6.5 | D-G6-6 |
| g6_closeout | close-out | G6.6 | D-G6-7 |

### 2.2 out-of-scope(显式排除)

见 YAML 头 `out_of_scope` 与 [G6_PLAN.md](G6_PLAN.md) §3:GPU 主刚体、商用 Havok、软体/布料/流体进硬门、Newton/Genesis 合入主仓 CI、DXIL RT 与窗口/输入进语言(RD-034 / D-130 维持)、性能数字进硬门、引擎采纳类宣称。

## 3. 交付物清单

见 YAML 头 `deliverables`(D-G6-1 ~ D-G6-7)。

## 4. 验收门(完整版,YAML 头为可提取摘要)

见 YAML 头 `acceptance_gates`(G-G6-1 ~ G-G6-8)。G-G6-6 为软门(spike 失败诚实登记 RD 即满足,不阻塞硬门);性能类数字全部 measured_local 写 evidence 不进硬门(out_of_scope perf_budget_hard_gates)。

## 5. Guardrails(字节级,机器核对)

见 YAML 头 `guardrails` 字段。核对方式:`ci/check_guardrails.py`(基准 g4-closed;`g5-closed` tag 未落,G5 面回归以步骤 82~87 判据 0-byte 只增核验)。

## 6. Deferred 引用

| 编号 | 内容摘要 | 承接 |
|---|---|---|
| RD-034 | DXIL RT 腿 blocked-on-upstream | 维护对象(本期物理不依赖 DXIL) |
| RD-036 | C ABI v2 超界硬需求存续 | 维护对象(本期不兑现) |
| RD-037 | .rx gfx submit 真派发条件臂 | 维护对象(本期不兑现) |
| RD-038 | 渲染器效果 kernel device 化条件臂 | 维护对象(分波兑现走自身轨道,G6 不承接不改写) |
| RD-042+ | G6 执行期新登记(spike 失败/P3+ 研究轨存续) | 执行期登记 |

详情以 [../../registry/deferred.json](../../registry/deferred.json) 为唯一事实源,本表仅引用。

## 7. 修订记录与开工裁决

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-07-30 | 初版契约固化(G6.1 治理包开工) |

**开工裁决留痕**:① owner 2026-07-30 会话下达 G6.1 治理包开工指令(「帮我干完 G6.1 脚手架工作」,承 G6_PLAN v1.0 同日定稿——多项择优与双轨架构已冻结为开工输入),范围裁决 = G6_PLAN 全文升格为契约上游事实源,流程裁决 = 完整工程纪律(G6 四件套 + Full RFC-0017 + ledger claim + smoke/evidence/budget 随实现 PR 落);② 编号 claim 全文见 registry/number_ledger.json reserved_in_flight[G6](v1.28):RFC-0017 单号伞形 / RXS 预期零消费(确需按合入时实际 next_free 顺位)/ 步骤 88 起 / RD-042 起 / U33 起 / RX_error 预期零新码 / MR-0012 按需;③ 同 revision 完成 G5 期滞后字段校准(RFC 14→16、CI_step 81→87、RD 36→41、U 31→32 各 on_tree_max,next_free 随动;G4.0 v1.13 校准先例,全部经命令核实);④ 物理为引擎库不进语言(06 §8.3),预期零新语言语义条款——确需时按 ledger 实际 next_free 顺位消费(与 RD-038 兑现臂同源顺位,先合先得,后合校准);⑤ 基准 ref 口径:`g5-closed` tag 未落,check_guardrails 默认基准维持 `g4-closed`,G5 面零回归由步骤 82~87 判据只增纪律托底。

---

## 8. Close-out(只追加区 — 开工时为空)

<!-- 验收记录、guardrail 核对输出、deferred 继承/关闭记录追加于此;上方条款 0-byte 修改。 -->
