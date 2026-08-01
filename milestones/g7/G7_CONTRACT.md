---
# 里程碑契约(14 §1 四要素;g7 = 生产帧闭环期,承 TEMPLATE_CONTRACT.md 体例)
contract: G7
title: G7 生产帧闭环期——收口 RD-038:compute SPIR-V 1.4 + RayQuery、真实 TLAS descriptor 链、GI/RTAO/硬阴影 W3 设备核、VisBuffer SW/HW 光栅零容差对拍、RD-038 字面余项审计与 One True Device Frame 生产证据
status: active
version: v1.0
date: 2026-08-01
timebox: "约 10–14 周(G7.0→G7.7 严格波次推进,见 G7_PLAN.md;周为相对刻度,非日历承诺)"
rfc_required: RFC-0018
upstream_docs:
  - "milestones/g5/G5_CONTRACT.md §8.1(G5 host/reference 主体与 RD-038 blocked-honest device 腿事实边界)"
  - "registry/deferred.json RD-038(本期唯一主 deferred:W1/W2 部分兑现后剩余 W3、HW raster diff 与字面余项)"
  - "渲染器调研/rurix 渲染器设备化调研报告.md(2026-07-29,shader supply wave、W3a/W3b/W3c 与证据矩阵)"
  - "milestones/g6/G6_CONTRACT.md §8.2(G6 closed;物理桥仅作为代表性动态场景输入,本期不扩物理面)"
  - "spec/shader_stages.md RXS-0242~0245 / spec/vulkan_backend.md RXS-0246~0248 / src/rurixc/src/vulkan_codegen.rs / src/rurix-rt/src/render_exec.rs"
  - "01 §5 使命 / 02 §2 U5 / 04 P-01·P-04·P-07·P-09·P-12·P-14 / 10 §3·§7·§9.5 / 14 §1·§3·§4·§5"
in_scope:
  - g7_governance
  - umbrella_rfc_0018
  - rd038_literal_baseline_audit
  - compute_spirv14_ray_query_codegen
  - compute_tlas_descriptor_runtime
  - w3_device_effects
  - visbuffer_hw_sw_diff
  - rd038_residual_device_legs
  - one_true_device_frame
  - production_frame_evidence
  - g7_closeout
out_of_scope:
  - rd037_single_source_gfx_submit
  - rd039_rd040_rd041_p3_plus_features
  - rd044_physics_p3_plus
  - dxil_ray_tracing
  - new_rendering_effects
  - safe_gpu_operator_platform
  - tensor_tile_neural_language_surface
  - language_core_autodiff_or_fusion
  - webgpu_multi_gpu_or_package_registry
  - production_adoption
deferred_refs: [RD-034, RD-037, RD-038, RD-039, RD-040, RD-041, RD-044]
deliverables:
  - id: D-G7-1
    name: G7.0 治理与基线包——四件套 + number_ledger reserved_in_flight[G7] + G5/G6 主线集成/close tag/README 状态/Jolt 供应链复核/干净基线记录
  - id: D-G7-2
    name: G7.1 RFC-0018 与 RD-038 字面审计——Full RFC 对抗性评审后 Approved;冻结 RED 语料、能力矩阵、代表性场景与非空 measured 预算
  - id: D-G7-3
    name: G7.2 W3a——compute SPIR-V 1.4 + RayQuery 类型/MIR lowering/关键指令 golden/spirv-val/严格诊断
  - id: D-G7-4
    name: G7.3 W3b——BLAS/TLAS 复用、compute AS descriptor/import、生命周期与 KernelWave::W3 fail-closed 执行链
  - id: D-G7-5
    name: G7.4 W3c——gi_probe.rx、rtao.rx、hard_shadow.rx 共用真实 TLAS 的设备执行与 host oracle 对拍
  - id: D-G7-6
    name: G7.5 HW raster 与余项——VisBuffer SW/HW 整数域零容差 diff;VSM 深度、TSR 等 RD-038 字面余项逐项设备证据或保持 RD open
  - id: D-G7-7
    name: G7.6~G7.7 One True Device Frame + 生产证据 + close-out——连续 raster→compute 数据流、soak、非空预算、视觉证据与 RD-038 逐字审计
acceptance_gates:
  - id: G-G7-1
    check: "治理/基线门:milestones/g7 四件套与 number_ledger reserved_in_flight[G7] 合入;G5/G6 十个在途提交进入唯一主线基准并落 g5-closed/g6-closed 或等价不可变 ref;README 状态校准;Jolt vendor/license/SBOM 复核;干净基线 fmt/clippy/test/trace/schemas/structure/number_ledger/guardrails/budget 全量记录;本门未绿前禁止语义实现"
  - id: G-G7-2
    check: "RFC 门:RFC-0018 Agent Approved 先于实现 PR;Full RFC 冻结 compute RayQuery 类型与动态语义、SPIR-V 1.4/capability 声明、AS descriptor/生命周期、缺能力诊断与 feature gate;D-409 对抗性评审 provenance ≠ 起草 provenance,findings 逐条 disposition;spec 条款与 RED conformance/UI 语料先于实现"
  - id: G-G7-3
    check: "基线/预算激活门:逐字审计 RD-038 title/backfill_condition/history,形成 pass×host×device×evidence 矩阵;冻结一个代表性 1080p 场景与 W1/W2/W3 能力快照;真实 GPU baseline 写 evidence;g7_budget.json 在首个语义实现 PR 前由空壳转为至少一项 measured_local 性能条目 + correctness counter,全程零 estimated"
  - id: G-G7-4
    check: "W3a 编译门:真实 .rx compute RayQuery 模块生成 SPIR-V 1.4,spirv-val 通过;反汇编 golden 锚定 OpTypeRayQueryKHR/OpRayQueryInitializeKHR/遍历与交点查询关键指令及所需 capability/extension;hit/miss/非法状态 RED-GREEN;W1/W2 产物与能力声明零回归"
  - id: G-G7-5
    check: "W3b 执行门:复用既有 BLAS/TLAS/AsManager 所有权,同一真实 TLAS 经 compute descriptor 进入 .rx kernel;KernelWave::W3 七项能力链缺一确定性拒绝,完整链 RURIX_REQUIRE_REAL=1 真跑;validation 零错误;禁止第二套伪 BVH、host 回填或隐式降级"
  - id: G-G7-6
    check: "W3c 效果门:gi_probe.rx/rtao.rx/hard_shadow.rx 三个内核共用真实 TLAS device 真跑;hit/miss/t/instance/primitive/barycentric 等几何语义在 RFC 冻结容差内与 host reference 一致;固定场景数值或感知门全过,host 仅为 oracle 不参与成功路径"
  - id: G-G7-7
    check: "光栅/余项门:同场景同投影同 VisBuffer ABI,真实 graphics raster 输出对真实 W2 software raster 输出逐像素整数域 diff=0;RD-038 中 VSM 深度、GI、RTAO/硬阴影、TAA-TSR 等每个字面分项逐项有真实 device evidence;未覆盖任一项则 RD-038 保持 open,禁止局部完成冒充全关"
  - id: G-G7-8
    check: "真实帧门:一个代表性动态场景中设备阶段输出真实成为下一设备阶段输入,至少覆盖 cull→VisBuffer→classify/resolve→VSM/lighting→TAA/TSR→readback;禁止 isolated nonzero 拼装充绿;RURIX_REQUIRE_REAL=1、零 mock/host substitution/SKIP;validation 零错误;完成≥30 分钟且≥10000 帧 soak,固定相机视觉证据与时间戳/显存/卡顿数据落 evidence"
  - id: G-G7-9
    check: "收口门:g7_budget.json 非空且 budget_eval.py --strict 全部 PASS、零 estimated/skip;全量回归冻结;新步骤 93 起 device 真跑;既有步骤 41~92 判据 0-byte 只增;RD-038 按 title/backfill_condition 逐字审计后才可 closed,否则 G7 可按证据边界收口但必须明确 RD 保持 open;status active→closed"
guardrails:
  - "milestones/m0~g6 的 *_CONTRACT.md 既有条款与 measured_local 预算条目 0-byte;g7_budget.json 经 glob 自动纳入且 id 强制 g7. 前缀;G7.0 空壳只允许存活到 G-G7-3,首个语义实现 PR 前必须非空 measured;全程零 estimated"
  - "registry/deferred.json/spike_gating.json/number_ledger 只追加;本期以关闭 RD-038 为唯一 deferred 主线,新阻塞自 RD-045 顺位登记;RD-016/028 永不复用;SG-010 软保留维持"
  - "evidence/ 只增不删不改;00~14 规划文档 0-byte;README 状态镜像可校准但不得冒充里程碑已通过"
  - "spec 先于实现;compute RayQuery 新语义必须 RFC-0018 + feature gate + RXS 条款 + conformance/UI 三角;稳定面未经 stabilization 不翻 stable"
  - "src/ 新 unsafe 全部 // SAFETY: + unsafe-audit U44 起顺位登记;优先复用 U30/U32 既有 AS/render_exec 边界;rurix-render 维持 #![forbid(unsafe_code)]"
  - "RURIX_REQUIRE_REAL=1;缺 provisioning SKIP=dev-env degrade,不充绿;mock/host substitution/isolated nonzero 不得满足 device/真实帧门"
  - "G5 冻结面 0-byte:MaterialClosure 32B、VisBuffer 位格式、Barrier EB 三轴、PageRequest 字段布局、host oracle 数值语义不得为迁就 device 实现而漂移"
  - "主线只做 RD-038 closure;RD-037、RD-039~041、RD-044、PyTorch/JAX operator、Tile/Neural、AD/fusion、WebGPU、多 GPU 全部 out-of-scope"
  - "既有零回归:dxil 套件恒定、vulkan 套件 grow-only、步骤 41~92 既有判据 0-byte 只增;步骤 69 RD-034 blocked 探针与步骤 70 永久 gap 维持"
  - "新文件 LF + 尾换行;本契约既有条款自合入起 0-byte,close-out 只追加 §8"
---

# G7 契约 — 生产帧闭环期

> 所属:[../../01_VISION_AND_MISSION.md](../../01_VISION_AND_MISSION.md) §5、[../../02_USERS_AND_USE_CASES.md](../../02_USERS_AND_USE_CASES.md) §2 U5；契约纪律见 [../../14_ENGINEERING_DISCIPLINE.md](../../14_ENGINEERING_DISCIPLINE.md) §1。
> 上游事实源:[../../registry/deferred.json](../../registry/deferred.json) RD-038 与 [../../渲染器调研/rurix 渲染器设备化调研报告.md](../../渲染器调研/rurix%20渲染器设备化调研报告.md)。
> 基准 ref:G7.0 落地的不可变 `g7-base`；在其生成前暂以当前 G6 close-out HEAD 为候选基准，不用过期 `g4-closed` 冒充。
> **定位口径:G7 不新增渲染效果，而是把 G5/G6 已有 host/reference 与孤立 device 证明收敛成一个连续、可测量、不可静默降级的真实设备帧。**
> **脚手架口径:本契约为 G7.0 开工结构件,不实现 RayQuery 或渲染语义;§8 close-out 开工时为空。**

---

## 1. 目标

G7 结束时应获得：① compute SPIR-V 1.4 RayQuery 完整编译与诊断通道；② BLAS/TLAS 到 `.rx` compute kernel 的真实 descriptor/lifetime 通道；③ GI、RTAO、硬阴影三个 W3 内核共用真实 TLAS device 真跑；④ VisBuffer software/hardware raster 整数域零容差对拍；⑤ RD-038 全部字面分项的逐项证据；⑥ 同一帧内前一设备阶段输出成为下一阶段输入的连续数据流；⑦ 非空 measured 预算、soak 与视觉证据。

## 2. 范围

### 2.1 in-scope

| 项 | 说明 | 波次 | 交付物 |
|---|---|---|---|
| g7_governance | 四件套、ledger、集成/供应链/基线冻结 | G7.0 | D-G7-1 |
| rfc_and_audit | RFC-0018、RED 语料、RD-038 字面矩阵、预算激活 | G7.1 | D-G7-2 |
| ray_query_codegen | compute SPIR-V 1.4 + RayQuery MIR lowering | G7.2 | D-G7-3 |
| as_compute_runtime | TLAS descriptor/import 与能力链 | G7.3 | D-G7-4 |
| w3_effects | GI/RTAO/硬阴影 `.rx` 设备核 | G7.4 | D-G7-5 |
| raster_and_residuals | HW/SW VisBuffer diff + VSM depth/TSR 等字面余项 | G7.5 | D-G7-6 |
| one_true_frame | 连续真实帧、预算、soak、close-out | G7.6~G7.7 | D-G7-7 |

### 2.2 out-of-scope

见 YAML `out_of_scope`。尤其明确：RD-037 单源 gfx submit 是强备选但不与 RD-038 混做；ReSTIR/MegaLights/帧生成/新材质层/软体流体不因“顺手”进入；Safe GPU Operator Platform 留作 G8 战略候选。

## 3. 交付物清单

见 YAML `deliverables`（D-G7-1 ~ D-G7-7）。

## 4. 验收门

见 YAML `acceptance_gates`（G-G7-1 ~ G-G7-9）。G7 不设 blocked-honest 即通过的设备门；缺硬件/工具链可在开发期如实 SKIP，但 close-out 的 device、预算与真实帧门必须真跑。

## 5. Guardrails

见 YAML `guardrails`。本期最关键的三条是：host reference 不为 device 漂移；isolated module 非零不等于真实帧；预算空壳不得越过 G-G7-3。

## 6. Deferred 处置

| 编号 | G7 处置 |
|---|---|
| RD-038 | 唯一主线。按 title、backfill_condition、history 逐字验收；全部兑现才 closed。 |
| RD-034 | DXIL RT upstream blocked，维持 open，不是 G7 依赖。 |
| RD-037 | `.rx` gfx submit 真派发，维持 open，候选 G8。 |
| RD-039~041 | 渲染 P3+，维持 open，不触发。 |
| RD-044 | 物理 P3+，维持 open；G6 physics 只作动态场景输入。 |
| RD-045+ | G7 新发现且本期做不完的真实阻塞，按合入时顺位登记。 |

## 7. 修订记录与开工裁决

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-01 | G7.0 四件套初版，status=active；零语义实现。 |

**开工裁决留痕**：用户 2026-08-01 指令“帮我把G7文档和脚手架落地”，承接同会话已裁定的 G7 首选“Production Frame Closure”。本期只收口 RD-038，不把外部趋势调研提出的 Safe GPU Operator Platform 混入单一期；编号 claim 见 `registry/number_ledger.json` reserved_in_flight[G7]；Full RFC-0018 在 G7.1 经独立 provenance 对抗性评审后方可批准与实现。

---

## 8. Close-out（只追加区 — 开工时为空）

<!-- 验收记录、真实命令输出、预算/evidence 路径、RD-038 逐字审计与 status flip 只追加于此。 -->
