# Mini-RFC MR-0004 — 生态包第二梯队 geometry 库立项（网格/BVH 基础，复用 RXS-0104~0113）

| 字段 | 值 |
|---|---|
| Mini-RFC 标识 | **MR-0004**（Mini-RFC 序列；独立于 Full-RFC 的 `RFC-####` 命名空间，不复用 RFC 编号，10 §9.5。Mini-RFC = 单页提案 + 失败测试先行 + 负责人批准，10 §3）。**首条经新外部 RFC 通道走通的样例**（MR-0003 dogfood：一件交付 = 流程样例 + geometry 立项留痕） |
| 标题 | 生态包第二梯队 `geometry` 库立项：host 纯 safe 网格/BVH 基础，复用 spec/stdlib.md 几何原语/谓词语义 0-byte |
| 档位 | **Mini-RFC**（10 §3：**生态包选型**（09 §5 第二梯队 G0 后落地）+ 新官方库公共 API 面（`rurix-geometry`）量级；纯 safe 库**纯编排**复用既有 RXS-0104~0113，**零新 spec 条款 / 零新语义面 / 不触** UB / 内存模型 / FFI ABI / 安全包络禁区——见 §3）。owner 经 AskUserQuestion 裁为 **落地最小 crate + dogfood**（2026-06-20；生态包选型为执行期新决策面，向上取严，AI 不自判 Direct） |
| 状态 | **Approved — 2026-06-20**（owner 于本工作会话经 AskUserQuestion 明确裁决 geometry 范围 = **落地最小 crate + dogfood**。批准记录由 claude-code 代录，非 AI 代签 / 自判，硬规则 1。crate 命名 / 合入仍由 owner 人工签署） |
| 承接里程碑 | G1.4（验收门 **G-G1-4**），G1 第四子里程碑（生态包第二梯队） |
| 关联条款 | **零新 RXS**（复用 `spec/stdlib.md` RXS-0104~0113 几何原语/谓词语义 **0-byte**；BVH/triangle mesh 为纯 orchestration，不新增公共语义面，镜像 `uc03-demo` 0 条款先例）。零新错误码（纯 safe 误用落既有 2xxx 类型类诊断，stdlib.md §4） |
| 依据决策 | **09 §5**（生态包第二梯队 geometry「网格/BVH 基础，G0/仿真共用」，G0 后）· **D-005**（MVP 验收后 G1 优先级与协作者引入）· `spec/stdlib.md` RXS-0104~0113（core 数学库 + 几何原语/谓词）· **M7.3 soft-raster 先例**（Rust host 纯 safe 参考 crate 锚定 spec 条款）· **MR-0003**（外部 RFC 通道 + 贡献门） |
| Provenance | `Assisted-by: claude-code:claude-opus-4-8`。Human-in-the-loop：owner 批准前不推进合入 |
| 失败测试先行 | `cargo test -p rurix-geometry`（`src/rurix-geometry/src/lib.rs` 单测锚定 RXS-0110~0113）：当前 `origin/main` 上 `rurix-geometry` crate **不存在** → **RED**（crate 缺失）；本 Mini 落地后转绿（9/9 通过）。10 §3 Mini「必须先有失败测试」 |

---

## 1. 摘要

把 **09 §5 已锁的生态包第二梯队 `geometry`（网格/BVH 基础，G0/仿真共用，G0 后）** 落到最小工程实现：新建 `src/rurix-geometry` **host 纯 safe** 库，**复用** `spec/stdlib.md` 既有几何原语 / 谓词语义（RXS-0104~0113）**0-byte**，并在其上做**纯编排**的 BVH 加速结构 + triangle mesh 基础——**不新增任何公共语义面 / spec 条款**（镜像 `uc03-demo`「工程编排不新增语义面」、`soft-raster`「Rust host 参考锚定既有条款」先例）。

本 RFC 同时是 **MR-0003 外部 RFC 通道的首条 dogfood 样例**：经新 intake 通道提案、按 Mini-RFC 模板填写、走 `ci/check_contribution.py` 贡献门（provenance / 条款号 / 验证 CI 阻断真实红绿，见 §5）。

## 2. 设计（用户视角 + 形态）

`rurix-geometry` 是自包含 Rust host 库，无外部依赖（全仓供应链纪律，零外依），全 safe（`unsafe_code = "deny"`）、纯函数、确定性（IEEE-754 f32）：

| 面 | 内容 | 复用条款 |
|---|---|---|
| 几何向量类 | `Point3` / `Vector3` / `Normal3` 语义区分 + 互转（`sub`/`offset`/`as_vector`/`to_normal`（零向量确定性边界）/`to_vector`） | RXS-0110，0-byte |
| 包围盒 / 射线 | `Aabb` / `Ray` 构造 + 字段；`Aabb::union`/`centroid`/`point`（BVH 辅助） | RXS-0111，0-byte |
| 几何谓词 | `Aabb::contains` / `Aabb::distance`（点在盒内 → +0.0） | RXS-0112，0-byte |
| Ray–AABB | `Aabb::intersects`（slab 法，轴平行退化确定性） | RXS-0113，0-byte |
| **BVH（新编排）** | `Bvh::build`（确定性 median-split）/ `from_triangles` / `query_ray`（命中图元下标）/ `intersects_ray`；叶 ray 测试**委托** `Aabb::intersects` | 复用 RXS-0113，零新条款 |
| **Triangle mesh（新编排）** | `Triangle` + `aabb()` 包围盒投影，供 BVH 网格图元 | 复用 RXS-0111，零新条款 |

`Point3.sub`/`offset`/谓词的语义与 `spec/stdlib.md` RXS-0110~0113 **逐字同义**（host 参考实现，对照 conformance/stdlib 的 `.rx` 语义）。BVH/Triangle 是这些谓词之上的工程编排，**不引入新公共语义面**。

## 3. 为何 Mini-RFC（而非 Direct，亦非 Full RFC）

- **非 Full RFC**：`rurix-geometry` 是**纯 safe host 库**，**不引入任何语言/语义面**（复用 RXS-0104~0113 0-byte），**不触** AGENTS 硬规则 5 / 10 §7.5 禁区（UB / 内存模型映射 / FFI ABI / 安全包络）；无 device kernel、无 unsafe、不附带 NVIDIA 组件。
- **非 Direct**：`G1_CONTRACT` §2 把「生态包第二梯队选型」列为执行期新决策面（09 §5 geometry 范围 = 评估-only vs 落地最小 crate，owner 裁决留痕）；新官方库的公共 API 面是新增稳定面候选（10 §6，stable 冻结随 RD-008 届时定义）。硬规则 8「判档争议向上取严」→ 走一页 Mini-RFC + 失败测试先行 + owner 批准。owner 经 AskUserQuestion 明确裁为「落地最小 crate + dogfood」。
- **升档触发条件（实现期守卫）**：若实现期 geometry 确需**新公共语义面**（如 device 几何 kernel / 新几何谓词进语言）则**停手**补 spec 条款 PR（RXS-0150 续号）先于实现（硬规则 7）；若需 device unsafe 则按硬规则 9 注册 unsafe-audit（U22）。本期均**不触**（host 纯 safe）。

## 4. 错误码 / 影响 / 范围

- **零新错误码**：纯 safe 库误用天然落既有 2xxx 类型类诊断（`spec/stdlib.md` §4 引用汇总）；`registry/error_codes.json` / en-zh message 零追加。
- **零新 RD / SG / budget counter / evidence**：host 纯 safe 库随 `cargo test --workspace` 覆盖（9 单测，trace 锚定 RXS-0110~0113），不接 `g1_budget.json`、不写 `evidence/*.json`。
- **cuDNN 维持 Phase 2+ 延后（留痕）**：09 §5 的另一第二梯队候选 **cuDNN 完整绑定包明确延后 Phase 2+**，**本期不落地**（仅本行留痕）；与 geometry 无依赖关系，不在本 crate 范围。
- **范围红线**：本期 geometry **不做 device 几何 kernel**（host 纯 safe 参考；device 路径 = 后续工程，非本期 / 非 RD）；不做可微/稀疏（SG-004/005 永久 gating）；不建包 registry（SG-007）。

## 5. 失败测试先行（10 §3 Mini 硬性）+ 真实红绿走通

- **失败测试先行**：`cargo test -p rurix-geometry` 在 `origin/main`（254d26f）上 **RED**（`rurix-geometry` crate 不存在）；本 Mini 落地 `src/rurix-geometry/`（含锚定 RXS-0110~0113 的 9 单测）后 **GREEN**（9/9 通过）。
- **贡献门真实红绿（反 YAML-only，dogfood）**：本 geometry PR 经 `ci/check_contribution.py`（MR-0003）守卫——构造一个**缺 provenance / 缺条款号**且触 `src/rurix-geometry/*.rs` 的 commit → 贡献门 **RED**（退出 1，3/3 缺项检出）→ 补 `Assisted-by` trailer + `RXS-####` 条款号 + 验证标记 → **GREEN**（退出 0）。本机 CPU-only 已演示；GitHub Actions run URL 由 owner 自托管 runner 上线回填（[`../milestones/g1/CI_GATES.md`](../milestones/g1/CI_GATES.md) §6 第 6 项，对齐 G1.1~G1.3 runner-offline 先例）。

## 6. 影响 / 向后兼容 / 范围

- **向后兼容**：纯追加。新 crate `rurix-geometry` 入 workspace members + `ci/trace_matrix.py` 扫描（对齐 rurix-engine 先例）；既有语义面 / RXS-0104~0113 / 回归网 **0-byte**。默认 `cargo build/clippy/test --workspace` 全覆盖且不依赖 device 而绿（host 纯 safe）。
- **范围红线**：见 §4；host-only、零外依、`unsafe_code=deny`。

## 7. Owner 批准

> **Approved — 2026-06-20**。owner 于本工作会话经 AskUserQuestion 明确裁决 geometry 范围 = **落地最小 crate + dogfood**（经新外部 RFC 通道走通流程样例）。批准范围：本 Mini 全文（§2 形态 / §3 判档 Mini / §4 零新码 + cuDNN Phase 2+ 延后留痕 / §6 范围）。批准记录由 claude-code 代录，**非 AI 代签 / 自行裁决**（硬规则 1）。**crate 命名 / 贡献门真实红绿 run URL 回填 / 栈式 PR 合入仍由 owner 人工签署**。
