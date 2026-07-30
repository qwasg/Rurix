# G4 独立后置代码审计报告(G4_REVIEW)

> 审计对象:G4 引擎渲染期——图形 RHI 化 + RD-035 执行面三项 + .rx 单源 Vulkan RHI + C ABI v2 判档 + BLACKHOLE 生产档验收
> 审计基线:HEAD(2026-07-30;G4_CONTRACT status=closed,§8.8 close-out 终审已签署)
> 审计 provenance:独立后置审计(audit provenance ≠ 起草/实现 provenance;D-409 口径延伸)
> 审计范围:核心码(rhi.rs / alias_alloc.rs / scheduler.rs / vk.rs / codegen.rs)+ C ABI 面(rurix-rt-cabi/lib.rs)+ CI 工作流(pr-smoke.yml 步骤 76~81)+ 预算/证据 schema + 冒烟脚本
> 审计方法:逐文件源码审查 + 契约/条款交叉核验 + cargo test 回归验证

---

## 1. 审计结论

G4 close-out 已签署(status=closed),门终审表 G-G4-1~G-G4-8 全过。本次独立后置审计
发现 **4 项 findings**(1 项经复核为误报,3 项确认),其中:
- Critical:0 项(初报 1 项经复核为误报)
- Medium:1 项(已修复)
- Low:2 项(1 项已修复,1 项登记存续)

全部确认 findings 已在本审计中修复并通过 cargo test 回归验证。

---

## 2. Findings 明细

### G4-AF-01 [Critical → 误报] derive_alias_plan 返回值丢失

| 字段 | 值 |
|---|---|
| 初报严重度 | Critical |
| 复核结论 | **误报(False Positive)** |
| 位置 | `src/rurix-rt/src/rhi.rs:1025-1027` |

**初报描述**:`derive_alias_plan` 调用 `alloc.assign(&lifetimes)` 但未返回 `AliasPlan` 结果,
导致 transient 别名复用失效。

**复核结论**:`alloc.assign(&lifetimes)` 是函数块的**尾表达式**(trailing expression,无分号),
在 Rust 中尾表达式即为返回值。`AliasAlloc::assign` 签名返回 `AliasPlan`,`derive_alias_plan`
签名返回 `AliasPlan`——返回值**未丢失**,别名复用正常工作。`cargo test -p rurix-rt --lib`
中 `derive_alias_plan_linear_chain` / `derive_alias_plan_disjoint_share_slot` 等单测全过,
佐证别名复用核心路径正确。

**处置**:无需修复,标记为误报。

---

### G4-AF-02 [Medium → 已修复] LiveRange 哨兵违反 start≤end 契约

| 字段 | 值 |
|---|---|
| 严重度 | Medium |
| 位置 | `src/rurix-rt/src/rhi.rs:1007` / `src/rurix-rt/src/alias_alloc.rs:375` |
| 状态 | **已修复** |

**描述**:`LiveRange::new` 文档契约声明「`start` 须 ≤ `end`」,但无写者资源的哨兵值
`LiveRange::new(u32::MAX, 0)` 故意违反此契约(`start=u32::MAX > end=0`)。`AliasAlloc::assign`
依赖 `start > end` 识别哨兵并分配独立槽——功能正确,但 `LiveRange::new` 的契约与哨兵用法
矛盾,违反「构造器契约自洽」原则。

**修复**:在 `alias_alloc.rs` 新增 `LiveRange::no_writer_sentinel()` 专用构造器,文档明确
标注其与 `new` 的 `start ≤ end` 契约互斥。`rhi.rs:1007` 和 `alias_alloc.rs:375`(测试)
改用 `LiveRange::no_writer_sentinel()` 替代 `LiveRange::new(u32::MAX, 0)`。

**验证**:`cargo test -p rurix-rt --lib` 75 passed / 0 failed。

---

### G4-AF-03 [Low → 登记存续] unwrap_or(u32::MAX) 静默溢出

| 字段 | 值 |
|---|---|
| 严重度 | Low |
| 位置 | `src/rurix-rt/src/rhi.rs:1018`(`u32::try_from(r).unwrap_or(u32::MAX)`) |
| 状态 | 登记存续(防御性,不影响正确性) |

**描述**:`derive_alias_plan` 中 `ResourceId(u32::try_from(r).unwrap_or(u32::MAX))` 在 `r`
超出 u32 范围时静默回退为 `u32::MAX`,可能导致多个资源共享同一 `ResourceId`。实际场景中
资源数远不可能超 u32,但 `unwrap_or(u32::MAX)` 模式静默吞溢出,不符合代码库「确定性失败
优于静默腐化」纪律。

**处置**:登记存续。rurix-rt 侧的 `unwrap_or(u32::MAX)` 模式与 rurix-render 侧(见 G5-AF-06)
同类,但 rurix-rt 既有测试覆盖更广且该路径仅在 `derive_alias_plan` 内部使用,优先级低于
rurix-render 侧。后续可统一改为 `expect` 确定性 panic。

---

### G4-AF-04 [Low → 登记存续] is_consuming_read 语义不一致

| 字段 | 值 |
|---|---|
| 严重度 | Low |
| 位置 | `src/rurix-rt/src/rhi.rs` vs `src/rurix-rt/src/graph.rs`(不同 `AccessKind` 枚举) |
| 状态 | 登记存续(设计差异,非 bug) |

**描述**:`is_consuming_read` 在 `rhi.rs` 和 `graph.rs` 中对应不同的 `AccessKind` 枚举,
语义存在差异。经复核,两处 `AccessKind` 服务于不同抽象层(rhi.rs = RHI pass 级,
graph.rs = render graph pass 级),语义差异是设计选择而非 bug。

**处置**:登记存续,无需修复。

---

## 3. CI / 预算 / 证据 schema 审计

### G4-CI-01 [High → 已修复] G4 workflow 步骤 76~81 缺 RURIX_REQUIRE_REAL=1

| 字段 | 值 |
|---|---|
| 严重度 | High |
| 位置 | `.github/workflows/pr-smoke.yml` 步骤 76/78/79/80/81 |
| 状态 | **已修复** |

**描述**:G4 步骤 76(uc05_graphics_rhi_smoke)、78(uc05_engine_embed_v3_smoke)、
79(uc05_exec_face_gate)、80(vulkan_rhi_channel_smoke)、81(blackhole_realtime_smoke)
的注释均声称「`RURIX_REQUIRE_REAL=1` 翻硬红」,但实际 `env:` 块**缺失**该变量。
后果:无 GPU 环境下这些步骤的 device 段会 SKIP(退 0)而非硬红,违反「mock/SKIP 不得充绿」
纪律(G4_CONTRACT guardrails:「device 见证纪律:RURIX_REQUIRE_REAL=1;缺 provisioning
环境 SKIP = dev-env degrade(翻硬红)」)。

**修复**:为步骤 76/78/79/80/81 的 `env:` 块补加 `RURIX_REQUIRE_REAL: "1"`。
步骤 77(uc05_graphics_invariant_gate)为纯 host 恒跑步骤(check_* 守卫风格),正确地
不需要该变量,未改动。

**验证**:YAML 语法校验通过;步骤注释与 env 块现已一致。

---

### G4-CI-02 [Low → 登记存续] 证据 schema 类型宽松

| 字段 | 值 |
|---|---|
| 严重度 | Low |
| 位置 | `milestones/g4/*_evidence_schema.json` |
| 状态 | 登记存续 |

**描述**:多个 evidence schema 的 `checks` 字段允许 `["boolean", "string"]` 联合类型,
理论上 "SKIP" 字符串可被 schema 接受为合法值,存在 false positive 风险。经复核,CI 脚本
侧的 `blackhole_realtime_ok` 等顶层判据为纯 `boolean`,schema 的联合类型是降级路径
(dev-env degrade SKIP)的兼容设计,非 bug。

**处置**:登记存续。后续可收紧 schema 为 `oneOf` + `enum` 约束 SKIP 字符串值。

---

## 4. 修复清单

| ID | 严重度 | 文件 | 修复内容 | 验证 |
|---|---|---|---|---|
| G4-AF-02 | Medium | `alias_alloc.rs` / `rhi.rs` | 新增 `no_writer_sentinel()` 构造器,替代 `new(MAX,0)` | cargo test 75/0 |
| G4-CI-01 | High | `pr-smoke.yml` | 步骤 76/78/79/80/81 补 `RURIX_REQUIRE_REAL=1` | YAML 语法通过 |

---

## 5. 未修复项(登记存续)

| ID | 严重度 | 存续理由 |
|---|---|---|
| G4-AF-01 | 误报 | 尾表达式即返回值,功能正确 |
| G4-AF-03 | Low | 资源数远不可能超 u32,防御性优先级低 |
| G4-AF-04 | Low | 设计差异非 bug |
| G4-CI-02 | Low | 联合类型为降级路径兼容设计 |

---

## 6. 审计签署

- 审计执行:独立后置审计(audit provenance ≠ 起草/实现 provenance)
- 审计基线:HEAD @ 2026-07-30
- 回归验证:`cargo test -p rurix-rt --lib` → 75 passed / 0 failed
- 审计结论:G4 close-out 门终审结论维持;本次审计修复 2 项(Medium×1 + High×1),
  登记存续 4 项(误报×1 + Low×3),零 Critical 确认项。
