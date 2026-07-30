# G5 独立后置代码审计报告(G5_REVIEW)

> 审计对象:G5 原生渲染器期——声明式 render graph + RHI 图形派发桥 + 虚拟化几何(meshlet/VisBuffer)+ VSM 阴影 + 屏幕探针 GI + 光追效果 + 材质场景流送 + 时域重建;含 RD-038 W1/W2 device 侧变更
> 审计基线:HEAD(2026-07-30;G5_CONTRACT status=closed,§8.1 close-out 终审已签署)
> 审计 provenance:独立后置审计(audit provenance ≠ 起草/实现 provenance;D-409 口径延伸)
> 审计范围:核心码(rurix-render: graph/graph.rs / geometry/visBuffer.rs / streaming/engine.rs / material/closure.rs / shadow/vsm.rs / gi/ / rt/ / temporal/)+ rurix-geom-build(dag.rs)+ CI 工作流(pr-smoke.yml 步骤 82~87)+ 预算(g5_budget.json)+ 证据 schema
> 审计方法:逐文件源码审查 + 契约/条款交叉核验 + cargo test 回归验证

---

## 1. 审计结论

G5 close-out 已签署(status=closed),门终审表 G-G5-1~G-G5-9 全过。本次独立后置审计
发现 **7 项 findings**(2 项经复核为误报/降级,5 项确认),其中:
- Critical:1 项(初报 Critical,复核降级为防御性 Medium,已修复)
- High:2 项(1 项已修复,1 项经复核为误报)
- Medium:3 项(全部已修复)
- Low:1 项(已修复)

全部确认 findings 已在本审计中修复并通过 cargo test 回归验证。

---

## 2. Findings 明细

### G5-AF-01 [Critical → Medium,已修复] DAG group_seq u32 溢出

| 字段 | 值 |
|---|---|
| 初报严重度 | Critical |
| 复核严重度 | Medium(防御性;实际场景不可能溢出) |
| 位置 | `src/rurix-geom-build/src/dag.rs:712-718,733-737` |
| 状态 | **已修复** |

**描述**:`group_seq` 为 `u32`,在组号全局化循环中逐层累加 `group_counts[li]` 和顶层
逐簇 +1。极端场景下(超 4G 组)溢出回绕,破坏 parent_error 单调性不变量。

**复核降级**:DAG 组数上界 = 簇数 ≤ mesh 三角形数 / 最小簇三角形数。对任何实际网格,
组数远不可能超 u32(需 >40 亿组)。因此实际风险为防御性而非 Critical。但 `+=` 静默回绕
不符合代码库「确定性失败优于静默腐化」纪律。

**修复**:将 `group_seq += group_counts[li]` 和 `group_seq += 1` 改为
`group_seq = group_seq.saturating_add(...)`,饱和不回绕,保单调性不破。

**验证**:`cargo test -p rurix-geom-build --lib` 22 passed / 0 failed。

---

### G5-AF-02 [High → 误报] DAG 顶层组初始化为 0

| 字段 | 值 |
|---|---|
| 初报严重度 | High |
| 复核结论 | **误报(False Positive)** |
| 位置 | `src/rurix-geom-build/src/dag.rs:697-701` |

**初报描述**:`DagNode { group: 0 }` 初始化为 0,导致顶层组分配不正确。

**复核结论**:`group: 0` 是**占位符**,在后续两处被覆写:
1. 层 0..top:组号全局化循环(line 716)`dag.nodes[base + ci].group = group_seq + g`
2. 层 top:顶层独立组循环(line 735)`dag.nodes[(top_base + ci) as usize].group = group_seq`

所有层的 `group` 字段均被正确赋值,初始化 `group: 0` 仅是 push 时的临时占位。
`cargo test` 中 `error_monotonic` / `levels_decrease_and_roots` / `uv_sphere_64_full_dag`
等单测全过,佐证组分配正确。

**处置**:无需修复,标记为误报。

---

### G5-AF-03 [High → 已修复] VisBuffer clip[3] 除零风险

| 字段 | 值 |
|---|---|
| 严重度 | High |
| 位置 | `src/rurix-render/src/geometry/visBuffer.rs:208-212` |
| 状态 | **已修复** |

**描述**:`inv_w = 1.0 / clip[3]` 前的守卫仅检查 `clip[3] <= 0.0`,未拦截极小正值
(如 `1e-40`)。`1.0 / 1e-40` 产生 `inf`,后续 `clip[0] * inf` 可能产生 `inf` 或 `NaN`,
传播至光栅化导致 VisBuffer 腐化。

**修复**:将守卫条件从 `clip[3] <= 0.0` 收紧为 `clip[3] <= 1e-20`,拦截极小正值,
保守丢弃该三角形(裁决 4 口径不变)。

**验证**:`cargo test -p rurix-render --lib` 239 passed / 0 failed。

---

### G5-AF-04 [Medium → 已修复] 流送引擎 next_seq 溢出

| 字段 | 值 |
|---|---|
| 严重度 | Medium |
| 位置 | `src/rurix-render/src/streaming/engine.rs:164-165` |
| 状态 | **已修复** |

**描述**:`next_seq` 为 `u64`,`self.next_seq += 1` 在极长运行(>580 亿年纪)后溢出,
破坏 FIFO 全序。实际不可能触发,但 `+=` 静默回绕不符合确定性纪律。

**修复**:改为 `self.next_seq = self.next_seq.saturating_add(1)`,饱和不回绕。

**验证**:`cargo test -p rurix-render --lib` 239 passed / 0 failed。

---

### G5-AF-05 [Medium → 已修复] 材质闭合 RGBE 对 INFINITY 编码

| 字段 | 值 |
|---|---|
| 严重度 | Medium |
| 位置 | `src/rurix-render/src/material/closure.rs:203-227` |
| 状态 | **已修复** |

**描述**:`pack_emissive_rgbe` 中 `v.to_bits()` 对 `f32::INFINITY` 得 `raw_exp=0xFF`,
经饱和路径隐式正确编码(E=255, R=G=B=255)。但未显式拦截 INFINITY,依赖隐式路径
不够确定性。

**修复**:在 `v <= 1e-32 || v.is_nan()` 守卫后追加 `v.is_infinite()` 显式短路,
直接返回 `0xFFFFFFFF`(E=255, R=G=B=255,极端 HDR 饱和编码),避免依赖隐式路径。

**验证**:`cargo test -p rurix-render --lib` 239 passed / 0 failed。

---

### G5-AF-06 [Medium → 已修复] Graph compile unwrap_or(u32::MAX) 静默溢出

| 字段 | 值 |
|---|---|
| 严重度 | Medium |
| 位置 | `src/rurix-render/src/graph/graph.rs:198,205,219` |
| 状态 | **已修复** |

**描述**:`add_resource` / `add_pass` / `add_pass_with` 中
`u32::try_from(self.resources.len()).unwrap_or(u32::MAX)` 在溢出时静默回退为 `u32::MAX`,
可能导致多个资源/pass 共享同一 ID。不符合「确定性失败优于静默腐化」纪律。

**修复**:将三处 `unwrap_or(u32::MAX)` 改为 `expect("resource/pass count overflow u32")`,
溢出时确定性 panic。

**验证**:`cargo test -p rurix-render --lib` 239 passed / 0 failed。

---

### G5-AF-07 [Low → 已修复] G5 预算 counter 阈值不一致

| 字段 | 值 |
|---|---|
| 严重度 | Low(预算一致性;影响 close-out strict 门判定) |
| 位置 | `milestones/g5/g5_budget.json` `g5.counter.render_exec_device_tests` |
| 状态 | **已修复** |

**描述**:`g5.counter.render_exec_device_tests` 的 description 声明「render_exec device
真跑见证基数 = 4」,但 `threshold` 设为 `1`。description 与 threshold 不一致,
close-out strict 门判定时可能产生歧义。

**修复**:将 `threshold` 从 `1` 改为 `4`,与 description 声明一致。

**验证**:JSON 语法校验通过;description 与 threshold 现已一致。

---

## 3. 修复清单

| ID | 严重度 | 文件 | 修复内容 | 验证 |
|---|---|---|---|---|
| G5-AF-01 | Medium(防御) | `dag.rs` | `group_seq` 用 `saturating_add` 防溢出 | cargo test 22/0 |
| G5-AF-03 | High | `visBuffer.rs` | 守卫收紧 `clip[3] <= 1e-20` 防除零 | cargo test 239/0 |
| G5-AF-04 | Medium | `engine.rs` | `next_seq` 用 `saturating_add` 防溢出 | cargo test 239/0 |
| G5-AF-05 | Medium | `closure.rs` | 显式拦截 INFINITY,确定性 RGBE 编码 | cargo test 239/0 |
| G5-AF-06 | Medium | `graph.rs` | `unwrap_or(MAX)` → `expect` 确定性 panic | cargo test 239/0 |
| G5-AF-07 | Low | `g5_budget.json` | `threshold` 1→4 与 description 一致 | JSON 校验通过 |

---

## 4. 未修复项(登记存续)

| ID | 严重度 | 存续理由 |
|---|---|---|
| G5-AF-02 | 误报 | `group: 0` 为占位符,后续被正确覆写 |

---

## 5. 审计签署

- 审计执行:独立后置审计(audit provenance ≠ 起草/实现 provenance)
- 审计基线:HEAD @ 2026-07-30(含 RD-038 W1/W2 device 侧变更)
- 回归验证:
  - `cargo test -p rurix-geom-build --lib` → 22 passed / 0 failed
  - `cargo test -p rurix-render --lib` → 239 passed / 0 failed
- 审计结论:G5 close-out 门终审结论维持;本次审计修复 6 项(High×1 + Medium×4 + Low×1),
  登记存续 1 项(误报×1),零 Critical 确认项(初报 Critical 经复核降级为防御性 Medium)。
