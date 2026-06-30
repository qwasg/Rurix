# spec 全量条款化审计（G2.5 语言 1.0，2026-06-30）

> 地位：G2.5 语言 1.0「spec 全量条款化」审计结论。agent 完全自主记录机器事实（AGENTS v3.0 硬规则 1/3）。Provenance：`Assisted-by: cursor:claude-opus-4.8`。所有数字来自真实命令输出（硬规则 3）。本报告为 evidence/ 只增不改文件。

## 1. 审计范围与方法

- 范围：`spec/` 全部 22 份语义规范文件（`spec/README.md` 为体例/索引，非条款源，单列）。
- 方法：
  1. 统计每文件 `^### RXS-\d{4}` 条款头数量（FLS 体例三级标题，trace_matrix 同一正则 `CLAUSE_RE`）。
  2. 与 `ci/trace_matrix.py --check` 的「clauses anchored」计数比对。
  3. 核对无裸条款头（条款头数 == 锚定条款数 == 唯一条款数，无悬空/无 body-less 占位）。
  4. 核对无未锚定 RXS（trace_matrix unanchored 列表为空）、无幽灵锚定（ghost 列表为空）。
  5. 判定语言 1.0 面是否完整、edition 是否为唯一新增面。

## 2. 条款头分布（真实命令输出，`rg "^###\s+RXS-\d{4}" spec/ --count`）

| 文件 | 条款头数 | 文件 | 条款头数 |
|---|---|---|---|
| lexical.md | 10 | syntax.md | 21 |
| names.md | 7 | types.md | 9 |
| borrow.md | 14 | consteval.md | 4 |
| device.md | 17 | toolchain.md | 21 |
| stdlib.md | 10 | imageio.md | 4 |
| softraster.md | 4 | interop.md | 4 |
| cublas.md | 4 | pipeline.md | 5 |
| release.md | 8 | interop_d3d12.md | 4 |
| async_buffer.md | 5 | engine_integration.md | 1 |
| shader_stages.md | 5 | dxil_backend.md | 11 |
| binding_layout.md | 4 | d3d12_runtime.md | 4 |
| **合计** | **176** | | |

> 计：10+21+7+9+14+4+17+21+10+4+4+4+4+5+8+4+5+1+5+11+4+4 = **176**。

## 3. trace_matrix 全锚定核对（真实输出）

```
$ py -3 ci/trace_matrix.py --check
[trace_matrix] PASS (176/176 clauses anchored, 452 test files scanned)
```

- 唯一条款数 = 176；锚定条款数 = 176；条款头数 = 176 → **三者一致**。
- 未锚定条款（unanchored）= 0；幽灵锚定（ghost，引用不存在条款号）= 0。
- 条款号重复定义 = 0（trace_matrix `parse_clauses` 对重复定义抛异常，PASS 即证无重复）。

## 4. 裸条款头核对

- 「裸条款头」= 在 spec 中存在 `### RXS-####` 标题但无条款体（仅占位/预留区间）。
- 历史上 spec 脚手架阶段（README v1.32/v1.33/v1.37/v1.39/v1.45/v1.47）确曾「仅登记文件名 + 预留区间，**不落裸条款头**」——即预留期 spec 文件中**零** `### RXS-####` 标题，待实现 PR 同落条款体（trace 维持全锚定，无悬空锚点）。
- 现状（README v1.38/v1.42/v1.44/v1.46/v1.49/v1.50 升格）：全部预留区间已升格为带编号条款体。条款头数（176）== 锚定条款数（176）：**不存在「有头无体」或「有头无锚」的裸条款头**——任一裸条款头都会使「条款头数 > 锚定条款数」或触发 trace_matrix unanchored 红，二者均为 0。
- 结论：**spec 全量无裸条款头**。

## 5. 语言 1.0 语义面完整性核对（无遗漏面）

按 spec/README.md §4 文件清单逐面核对，语言 1.0 应覆盖的语义面均已条款化：

| 语义面 | 文件 | 状态 |
|---|---|---|
| 词法 / 语法 / 名称 / 类型 | lexical / syntax / names / types | 全量条款化（RXS-0001~0047） |
| 所有权 / 借用 / const 求值 | borrow / consteval | 全量条款化（RXS-0048~0065） |
| device 着色 / 地址空间 / codegen / 运行时 | device | 全量条款化（RXS-0066~0082） |
| 工具链 / 包管理 / rx CLI / LSP | toolchain | 全量条款化（RXS-0083~0103） |
| 标准库 / image-io / 软光栅 | stdlib / imageio / softraster | 全量条款化（RXS-0104~0121） |
| 互操作 / cublas / 流水线 / 发布 | interop / cublas / pipeline / release | 全量条款化（RXS-0122~0152） |
| 着色阶段进语言（G2.1） | shader_stages | 全量条款化（RXS-0153~0156, RXS-0174） |
| DXIL 第二后端（G2.2）+ body 降级 + 采样降级 | dxil_backend | 全量条款化（RXS-0157~0162, 0171~0173, 0175~0176） |
| 绑定布局推导（G2.3） | binding_layout | 全量条款化（RXS-0163~0166） |
| UC-04 deferred 运行时出图（G2.4） | d3d12_runtime | 全量条款化（RXS-0167~0170） |
| **edition 机制（G2.5）** | **edition（新增）** | **本里程碑新增，唯一新增面，RXS-0177~0180** |

- 经逐面核对，语言 1.0 既有语义面（词法→语法→类型→所有权→device→工具链→stdlib→互操作→着色/DXIL/绑定/UC-04）**全量条款化、无遗漏**。
- **edition 机制是语言 1.0 的唯一新增语义面**（11 §5 定义「语言 1.0 = spec 全量条款化 + conformance 覆盖 + 首个 edition」中「首个 edition」的兑现）。其余面均在 G2.1~G2.4 及更早里程碑条款化完毕。

## 6. 审计结论

1. spec 全量条款化达标：176 条款头 == 176 锚定条款 == trace_matrix 176/176，**零裸条款头、零未锚定、零幽灵锚定、零重复定义**。
2. 语言 1.0 既有语义面全量覆盖、无遗漏。
3. **edition 机制（RXS-0177~0180，新建 `spec/edition.md`）= 语言 1.0 的唯一新增语义面**。本里程碑落 edition 条款后，语言 1.0 spec 全量条款化即完整兑现（trace 176 → 180 全锚定）。

> 后续步骤：落 spec/edition.md RXS-0177~0180 + 实现 + conformance 后，trace_matrix 升至 180/180，本审计的「176」基线刷新为「180」，edition 面纳入全量。
