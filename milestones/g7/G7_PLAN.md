# G7_PLAN — Production Frame Closure 主线分解

> 契约：[G7_CONTRACT.md](G7_CONTRACT.md) · 门：[CI_GATES.md](CI_GATES.md) · 唯一主 deferred：[`RD-038`](../../registry/deferred.json)
> 推进形态：**严格波次**。G7.0 基线 → G7.1 RFC/审计/预算 → G7.2 codegen → G7.3 runtime → G7.4 effects → G7.5 raster/residuals → G7.6 frame/evidence → G7.7 close-out。前波未绿，后波不得用 stub/mock/host substitution 抢跑。

---

## 0. 成功路径与边界

```text
.rx compute RayQuery
  └─ rurixc MIR → SPIR-V 1.4 + RayQuery KHR
       └─ rurix-rt 复用 BLAS/TLAS → compute AS descriptor
            ├─ gi_probe.rx
            ├─ rtao.rx
            └─ hard_shadow.rx

真实帧：cull → VisBuffer(SW/HW) → classify/resolve → VSM/lighting → TAA/TSR → readback
                         ↑ 每一箭头必须传真实设备资源，不接受 isolated nonzero 拼装
```

Rust 运行时与 Vulkan FFI 可以继续作为驱动层；禁止的是 host reference 回填效果或替代设备阶段。host reference 只作 oracle。

## 1. 波次

### G7.0 — 治理、集成与不可变基线

- 落本四件套与 number ledger G7 claim。
- 把 G5/G6 在途历史集成到唯一主线，补不可变 close ref；生成 `g7-base`。
- README 状态校准；Jolt vendor/license/SBOM 复核。
- 全跑 fmt/clippy/test/trace/schema/structure/ledger/guardrail/budget，结果追加到契约开工记录或独立 evidence。
- 本波零 `src/` 语义修改。

退出门：G-G7-1。

### G7.1 — RFC-0018、RED 语料、RD-038 审计与预算激活

RFC-0018 至少冻结四章：

| 章 | 内容 |
|---|---|
| A | compute RayQuery 类型、状态机、builtins 与动态语义 |
| B | SPIR-V 1.4/KHR capability、extension 与 per-entry module policy |
| C | BLAS/TLAS descriptor、资源生命周期、同步和 fail-closed 能力协商 |
| D | renderer W3 使用约束、host oracle 对拍与禁止降级 |

同时完成：

- 先落 spec diff、conformance/UI RED 语料，再实现。
- 建 `RD-038` 字面矩阵：`分项 / host oracle / 当前 device / 缺口 / 目标 smoke / evidence schema / close 判据`。
- 冻结代表性 1080p 场景、固定相机、TLAS 和 W1/W2/W3 capability snapshot。
- 在目标 RTX 4070 Ti 上记录未优化 baseline；把 `g7_budget.json` 从空壳追加为非空 measured 预算。候选族：frame GPU p95、CPU submit p95、peak VRAM、pipeline stall、device pass count、validation error count。具体阈值只来自 baseline 命令输出，不在脚手架预造。

退出门：G-G7-2、G-G7-3。

### G7.2 — W3a：compute SPIR-V 1.4 + RayQuery codegen

- 扩展 device type/MIR 以表达 RayQuery 生命周期与查询结果。
- per-entry 模块按需升 SPIR-V 1.4；W1/W2 仍保持最低合法版本/能力集合。
- lowering 覆盖 initialize、proceed、candidate/committed intersection、object/primitive/instance/geometry index、barycentric/t 值等 RFC 冻结子集。
- capability/extension 只按真实使用声明；非法状态和缺能力走结构化诊断。
- `spirv-val` 与反汇编 golden；RED 语料复原转绿。

退出门：G-G7-4。主要落点：`src/rurixc`、`spec/`、`conformance/`、`tests/ui`、SPIR-V golden。

### G7.3 — W3b：AS 到 compute descriptor 的执行闭环

- 复用 G3/G5 已有 BLAS/TLAS/BDA/AsManager，不建第二所有者。
- 给 compute pipeline 增加最小 AS descriptor/import 通道与明确生命周期。
- KernelWave::W3 能力链逐项 fail-closed；capability snapshot 进入 evidence。
- validation layer 零错误；设备丢失/缺扩展/过期 TLAS/错误 barrier 有 RED 自检。

退出门：G-G7-5。主要落点：`src/rurix-rt` 与 `apps/uc06-renderer` 装配层；新增 unsafe 才消费 U44+。

### G7.4 — W3c：三个真实效果内核

- `gi_probe.rx`
- `rtao.rx`
- `hard_shadow.rx`

三者共用同一真实 TLAS。逐项对拍 hit/miss、t、instance/primitive/geometry index、barycentric 与效果输出；冻结数值/感知容差前先 measured，不允许为过门修改 host oracle。

退出门：G-G7-6。

### G7.5 — HW raster diff 与 RD-038 余项

- 真实 graphics raster VisBuffer 对真实 `.rx` software raster VisBuffer，同场景同投影同 ABI，整数域逐像素 diff=0。
- 若 Vulkan top-left/edge coverage 与 software raster 规则存在规范差异，先经 RFC 修订裁定，不扩大容差。
- 逐字复查 RD-038：W1 的 VSM 是否只有 page-mark、TSR 是否仍只有 host reference、VSM depth/sample 是否真实进入 device。缺项必须在本波补齐或让 RD-038 保持 open。
- 保持 W1/W2 已有五 kernel 证据零回归。

退出门：G-G7-7。

### G7.6 — One True Device Frame 与生产证据

- 代表性动态场景消费 G6 physics transform，但不新增物理功能。
- 每一设备阶段的输出资源真实成为下一阶段输入；增加 provenance/resource-id 证据，阻断 isolated output 拼装。
- `RURIX_REQUIRE_REAL=1` 真跑：固定相机视觉证据、GPU timestamps、VRAM、pipeline stall/launch 数据。
- ≥30 分钟且 ≥10000 帧 soak；validation error、device loss、TDR、资源泄漏均为硬红。
- budget threshold 只能由 G7.1 measured baseline 追加式收紧，不得回改已有 measured 条目。

退出门：G-G7-8。

### G7.7 — Close-out

- `budget_eval.py --strict` 非空、零 estimated、零 skip。
- 全量回归与步骤 93+ 真跑，既有步骤 41~92 判据不改写。
- 按 RD-038 的 title、backfill_condition、history 逐字审计；全部兑现才翻 closed。
- 契约 §8 只追加终审表、真实输出和 evidence 路径，最后 status flip。

退出门：G-G7-9。

## 2. 主要 PR 栈

| PR | 形态 | 前置 | 主要内容 |
|---|---|---|---|
| PR-0 | Direct / governance | 无 | G7 四件套、ledger、README、基线记录 |
| PR-1 | Full RFC | PR-0 | RFC-0018 Draft→对抗性评审→Approved；spec/RED tests；预算 baseline |
| PR-2 | semantic implementation | PR-1 | W3a RayQuery codegen |
| PR-3 | runtime/unsafe as needed | PR-2 | W3b AS descriptor/lifetime |
| PR-4 | renderer device | PR-3 | 三个 W3 kernels |
| PR-5 | renderer device | PR-4 | HW raster diff + RD-038 residuals |
| PR-6 | integration/evidence | PR-5 | One True Device Frame + soak + budget |
| PR-7 | close-out | PR-6 | RD/预算/全量回归终审 |

## 3. Out-of-scope 触发规则

- RD-037 单源 gfx submit：G7 只记录接口依赖，不实现；若 W3 闭环事实证明缺它不可运行，登记 RD-045+ 并单独重立项，禁止偷偷并入。
- RD-039~041 新效果：只有现有帧的 measured 画质/性能缺口满足各自 backfill condition 才可后续提案。
- Tile/Neural、Tensor Core、AD/fusion：服从 SG-002/SG-004/SG-005，不因外部趋势抢跑。
- Safe GPU Operator Platform：候选 G8，不能占用 G7 RFC/CI/预算编号。

## 4. 风险与止损

| 风险 | 预警 | 止损 |
|---|---|---|
| RayQuery 语义面扩张 | RFC 出现完整 RT pipeline/新 shader stage 需求 | 退回最小 compute inline ray-query 子集；RT pipeline 沿既有 RXS-0242~0248 |
| AS 所有权重复 | renderer、runtime 各持独立 TLAS 生命周期 | 强制复用 AsManager；无法复用则先修所有权图，不推进效果核 |
| HW/SW edge rule 不一致 | 仅边界像素稳定 diff | RFC 冻结覆盖规则；不放宽整数域容差掩盖歧义 |
| “全帧”仍是模块拼装 | pass output 不被后继资源消费 | resource-id/provenance 机验，失败即 G-G7-8 红 |
| 预算再次为空 | 首个实现 PR 前 JSON 仍空 | G-G7-3 硬阻断；不允许以后补为由继续 |
| 范围膨胀 | 出现新特效/物理/operator/tile 代码 | P-12 拒绝并登记 deferred/后续候选 |

## 5. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-01 | G7.0 初版；单一主题 Production Frame Closure。 |
