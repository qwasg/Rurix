# RFC-0008 — edition 机制与 stabilization 流程

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0008（4 位制，编号永不复用，10 §9.5） |
| 标题 | 首个 edition 机制（语义版本边界声明）与 stabilization 流程锚点 |
| 档位 | **Full RFC**（10 §3：edition / stabilization 为显式 Full RFC 触发面；AGENTS 硬规则 5/7） |
| 状态 | Agent Approved（2026-06-30）。agent 完全自主批准（AGENTS v3.0 硬规则 1），批准后推进下游实现 PR |
| 承接里程碑 | G2.5（D-G2-5 / G-G2-5：语言 1.0 + 首个 edition）；激活 RD-008（stable API 快照冻结机制） |
| 关联条款 | 拟落 spec **RXS-0177~RXS-0180**（见 §5）：新建 `spec/edition.md`（edition 声明 / 解析校验 / 不匹配诊断 / stable 面关系） |
| 依据决策 | 11 §5（G2 = 语言 1.0 = spec 全量条款化 + conformance 覆盖 + 首个 edition）· 01 §6（使命与生态成功判据）· 10 §3（变更三档）/ §5（特性生命周期）/ §6（稳定面）/ §2.2（FCP-lite）· 13 D-308~D-311（manifest/lock 已锁决策）· RD-008（stable API 快照冻结机制激活，G2.5 候选触发点） |
| Provenance | `Assisted-by: cursor:claude-opus-4.8`。agent 自主决策，批准后推进下游实现 |
| Agent 批准 | Approved — 2026-06-30；批准范围含 §4 全部设计（edition 声明语义 / 解析校验 / 不匹配诊断 / stable 面边界 / stabilization 流程）+ §9 全部裁决 + RD-008 激活裁决；记录于 §9 裁决 + 本表 + 修订记录。本 RFC 不触 🔒 禁区（无 UB / 无内存模型映射 / 无 FFI ABI / 无安全包络），edition 为编译期/host 工具链面 |

---

## 1. 摘要

本 RFC 为 Rurix 引入**首个 edition 机制**——把"语义版本边界"作为 `rurix.toml` 清单的一等声明 `[package].edition`，并落 stabilization 流程锚点（feature gate → tracking → 两里程碑无重大修订 → stabilization report → FCP-lite，10 §5/§2.2/§6）。

首个 edition 命名 **`"2026"`**，定位为**机制锚点**：首期 edition **不引入任何破坏性 edition-gated 行为差异**（首期 edition-gated 差异集 = 空集），仅建立"语言面有 edition 边界、未来破坏性变更经 edition 隔离"的机制基座，为语言 1.0 的稳定化铺设可演进路径。

通路（编译期/host，无 device）：

```
rurix.toml  [package].edition = "2026"
  └→ rurix-pkg Manifest::parse → edition 字段(缺省 "2026")
       └→ edition 解析/校验(合法集 = {"2026"})
            ├→ 合法 → Manifest.edition 确定 → edition-gated 行为分发锚点(首期空集)
            └→ 未知 edition → RX7020 strict-only 拒(无 fallback,P-01)
```

## 2. 动机

- **已锁路线落地**：11 §5 把 G2 期"语言 1.0"定义为 **spec 全量条款化 + conformance 覆盖 + 首个 edition**。前两者经 G2.1~G2.4 + 本里程碑审计兑现；**首个 edition 机制是语言 1.0 的最后一块拼图**，无 edition 机制则语言无破坏性演进的版本隔离手段，未来任何破坏性变更都将无处安放（要么破坏既有代码、要么永久冻结语言面）。
- **stabilization 流程需要锚点**：语言 1.0 = 首个 stable 发布候选点。stable 面一旦定义就需要"哪些面进 stable、变更如何受控"的流程；edition 是其天然边界（stable 面以 edition 为版本锚）。
- **RD-008 候选触发点**：deferred RD-008（stable API 快照冻结机制激活）的 backfill 条件 = "首个 stable 发布时定义 stable 面并激活快照机制 + bless 守卫"。G2.5 语言 1.0 即该触发点（见 §9 Q-RD008）。

**为何需要 Full RFC（而非 Direct/Mini）**：edition / stabilization 是 10 §3 显式列举的 Full RFC 触发面（与 UB / 内存模型映射 / FFI ABI 并列）。edition 定义了语言演进的版本契约边界，触及语言稳定化治理本体，按 AGENTS 硬规则 5/7 由 agent 自主经 Full RFC 落笔作留档与可追溯；判档争议向上取严作为自我约束建议。本 RFC **不触 🔒 禁区**（无 UB 条款、无内存模型映射、无 FFI ABI 二进制布局、无安全包络边界——edition 是纯编译期/host 工具链声明语义）。

## 3. 指导级解释（用户视角）

用户在 `rurix.toml` 的 `[package]` 表声明所用 edition：

```toml
[package]
name = "my-app"
version = "0.1.0"
edition = "2026"
```

- **`edition` 字段**：可选字符串。缺省时取首个 edition `"2026"`（向后兼容：现存无 `edition` 字段的清单均隐式按 `"2026"` 解析，不破坏既有包）。
- **合法值**：首期仅 `"2026"`。声明未知 edition（如 `"2099"` / `"2015"` / `"latest"`）→ 编译期 **RX7020** 诊断 strict-only 拒绝，**无 fallback、无静默降级**（P-01）。
- **首个 edition 的行为**：`"2026"` 仅建立机制锚点，**与"无 edition 声明"行为完全一致**（首期 edition-gated 行为差异 = 空集）。用户现在声明 edition 不会改变任何编译/运行语义；其价值是为未来破坏性语言演进预留版本隔离边界。
- **跨 edition 迁移（未来）**：当出现第二个 edition（如 `"20XX"`）携带破坏性差异时，迁移路径 = 用户显式 bump `edition` 字段 + 按届时迁移指南调整源码；老 edition 包不受影响（edition 边界隔离）。本 RFC 不引入第二 edition，仅锚定机制。

## 4. 参考级设计

### 4.1 edition 声明语义（RXS-0177）

- `rurix.toml` `[package]` 表新增可选键 `edition`（字符串）。
- 解析进 `rurix-pkg` 的 `Manifest.edition: Edition`（新枚举/新类型）。
- 缺省语义：清单缺 `edition` 键 → `Edition::Edition2026`（首个 edition），保证既有清单 0-byte 兼容。
- `edition` 值非字符串类型 → 复用既有 `RX7005`（`toolchain.pkg_manifest_invalid`，清单类型错误，与 name/version 类型错误同类）。

### 4.2 edition 解析/校验规则（RXS-0178）

- 合法 edition 集合（首期）= `{ "2026" }`，集合冻结于本 RFC，新增 edition 经后续 Full RFC 扩展。
- 解析为确定性纯函数 `Edition::parse(&str) -> Result<Edition, EditionError>`，无环境依赖、无 I/O。
- 校验在 `Manifest::parse` 内联完成（与 name/version/build 校验同期），不引入新解析阶段。

### 4.3 edition 不匹配/未知诊断（RXS-0179）

- 声明的 edition 不在合法集合 → **RX7020**（`toolchain.edition_unknown`，7xxx 工具链段续号，真实可达）。
- strict-only：未知 edition 直接拒，**不回退到缺省 edition、不警告后继续**（P-01 strict-only，对齐 manifest 既有 RX7005 拒绝纪律）。
- 诊断 message 含被拒的 edition 值 + 合法集合提示（`{detail}` 占位）。

### 4.4 stable 面与 edition 的关系（RXS-0180）

- edition 是 stable 面的**版本锚边界**：语言 1.0 的 stable 面（见 §6 / RD-008）以首个 edition `"2026"` 为基准快照。
- 同一 edition 内 stable 面只增不破坏（加性演进）；破坏性变更须经新 edition 隔离。
- 本条款仅声明关系本体（edition ↔ stable 面），具体 stable 快照机制由 RD-008 激活落地（`ci/stable_snapshot.py` + bless 守卫），**不在 spec 条款冻结 stable 快照的字节内容**（快照是回归锚，非语言 ABI 保证）。

### 4.5 edition-gated 行为分发锚点（首期空集）

- 实现侧预留 edition 分发 hook（如 `Edition::feature_gated(...)` 查询点），首期所有 edition-gated 查询返回"无差异"（空集）。
- 这是机制锚点：保证未来加入第二 edition 时有明确接入点，而非散落的 ad-hoc 版本判断。首期不实现任何具体差异。

## 5. 下游 spec 条款映射（spec diff，10 §3 要件）

新建 `spec/edition.md`，自 **RXS-0177** 起续号（当前最高现存条款号 = RXS-0176）。FLS 体例（Syntax / Legality / Dynamic Semantics / Implementation Requirements，**严禁 UB 节**）。条款 PR 先于实现 PR（硬规则 7）；trace_matrix 维持全锚定（176 → 180）。

| 条款（拟） | 标题 | 测试锚定计划（每条 ≥1，`//@ spec`） |
|---|---|---|
| RXS-0177 | edition 声明语义（`[package].edition`，缺省首个 edition `2026`） | `src/rurix-pkg` 单测：缺省 edition / 显式 `2026` 解析 + conformance/edition/accept fixture |
| RXS-0178 | edition 解析/校验规则（合法集 `{2026}`，确定性纯函数） | `src/rurix-pkg` 单测：`Edition::parse` 合法/非法 + conformance/edition |
| RXS-0179 | edition 不匹配/未知诊断（未知 edition → RX7020 strict-only） | `src/rurix-pkg` 单测：未知 edition → RX7020 + conformance/edition/reject fixture |
| RXS-0180 | stable 面与 edition 的关系（edition 作 stable 面版本锚边界） | `src/rurix-pkg` 单测：edition 作 stable 快照基准 + ci/stable_snapshot 自检 |

- **错误码策略**：edition 未知诊断从 **RX7020** 起（7xxx 工具链/诊断段续号，接 RX7019）按真实可达类别分配；不预留、不预造。`registry/error_codes.json` 只追加 + en/zh message-key `toolchain.edition_unknown`。edition 值类型错误复用既有 RX7005（不新增）。

## 6. feature gate / tracking / 实现序（10 §3 要件）

- **feature gate**：edition 解析/校验是 `rurix-pkg` 清单核心面，**无独立 cargo feature gate**（edition 是清单一等字段，缺省兼容，非可选编译面）。stabilization 流程的 stable 快照机制经 `ci/stable_snapshot.py` + `check_guardrails` 守卫 gate。
- **tracking**：
  - edition 机制 tracking = 本 RFC + RXS-0177~0180 + RD-008 激活留痕；
  - stabilization：首个 edition `"2026"` + 语言 1.0 stable 面经本里程碑定型 → 后续两里程碑无重大修订 → stabilization report → FCP-lite（10 §2.2 advisory 公开等待窗）。第二 edition 出现时另立 tracking。
- **实现序**（条款先于实现，硬规则 7）：本 RFC §4 → spec/edition.md RXS-0177~0180 条款体 → rurix-pkg `Edition` 解析/校验 + RX7020 + 双语 message → conformance/edition 语料 → RD-008 stable 快照机制 + bless 守卫 + 首份 bless → CI 步骤 49 `ci/edition_smoke.py` 真实红绿。
- **真实红绿**（反 YAML-only）：
  - host green：合法 `edition = "2026"` 声明经 `Manifest::parse` 接受 + 缺省兼容 + 全量 conformance 关键面 0 诊断 + stable 快照匹配；
  - host red：未知 edition（如 `"2099"`）→ RX7020 strict-only 拒；篡改 `tests/stable/stable_api.snapshot` → `check_guardrails`/`ci/stable_snapshot.py` 翻红；
  - 复原绿：复原合法 edition / 复原快照 → 绿（红绿闭合）。
  - run URL：edition 为编译期/host 面无 device，优先经 self-hosted/GitHub Actions 取真实 run URL；若 runner 不可达则贴本机 `ci/edition_smoke.py` 真实红绿输出 + 诚实标注，不伪造。

## 7. 备选方案

- **edition 声明放在 toolchain 配置（非 manifest）**：否决——edition 是包级语义版本契约，与 `name`/`version` 同属包身份，放 `[package]` 最自然（对齐 Rust `[package].edition`）。
- **首期即引入破坏性 edition-gated 差异**：否决——语言 1.0 应稳定、首个 edition 应是兼容基准；首期引入破坏性差异会增加风险且无明确需求。首期仅锚定机制，差异留未来 edition。
- **edition 不匹配走警告 + 回退缺省（而非 strict 拒）**：否决——违反 P-01 strict-only（无静默降级），未知 edition 必须显式拒绝。
- **edition 落 spec 时并入 toolchain.md（不新建文件）**：否决——edition 是独立语义面（语言版本边界 + stabilization），独立成文 `spec/edition.md` 对齐 binding_layout.md / d3d12_runtime.md / shader_stages.md 独立成文先例，便于后续 edition 演进集中维护。
- **本里程碑不激活 RD-008（维持 not_frozen）**：否决——见 §9 Q-RD008，语言 1.0 = 首个 stable 发布触发点，激活为最完整兑现 acceptance_gate。

## 8. 不做（范围红线）+ 新 deferred

- **不解多后端红线**（D-008 / SG-003 维持 not_triggered）：edition 与后端无关。
- **不触 Python 原生嵌入**（红线 1 / SG-008 维持 not_triggered）。
- **不触发 registry**（D-312 / SG-007 维持 not_triggered）：edition 是包级声明，不引入 registry 透明日志。
- **不引入第二 edition、不引入任何破坏性 edition-gated 差异**：首期 `"2026"` 仅机制锚点。
- **不冻结 stable 快照的字节内容为语言 ABI 保证**：stable 快照是确定性回归锚（镜像 golden bless），register/字节布局/工具版本不冻结为 stable（对齐 RXS-0162 / RFC-0005 §4.5 🔒 先例）。
- **新 deferred**：本 RFC **不新造 deferred**。RD-008 由本 RFC 激活（open→closed，见 §9 Q-RD008 + registry）。第二 edition / 跨 edition 破坏性差异 / 完整 stabilization report 自动化为未来里程碑工作，届时随需登记，不预造。

## 9. agent 裁决清单（Agent Approved 2026-06-30）

> agent（完全自主，AGENTS v3.0 硬规则 1）于 2026-06-30 工作会话裁决下表全部项并批准 RFC-0008 全文；agent 自主记录裁决输入与机器事实。Provenance：`Assisted-by: cursor:claude-opus-4.8`。

| 项 | 抉择 | 裁决 |
|---|---|---|
| Q-Name | 首个 edition 命名：`2026` / `1.0` / `v1` | **`"2026"`**（年份制，对齐 Rust edition 心智模型 + 当前刻度；语义版本边界以年份锚定，未来 edition 续年份） |
| Q-Scope | 首期 edition 行为：仅机制锚点（空差异集）/ 引入首批 edition-gated 差异 | **仅机制锚点**（首期 edition-gated 行为差异 = 空集，`"2026"` 与无 edition 声明等价）；破坏性差异留未来 edition |
| Q-Decl | 声明形态：`[package].edition` / toolchain 配置 / CLI flag | **`[package].edition`** in rurix.toml（复用 `Manifest::parse`，对齐 Rust） |
| Q-Default | 缺省语义：缺 edition 键 → 报错 / 取首个 edition | **取首个 edition `"2026"`**（向后兼容，既有无 edition 清单 0-byte 不破坏） |
| Q-Mismatch | 未知 edition：strict 拒 / 警告回退 | **strict-only 拒**（RX7020，无 fallback，P-01） |
| Q-ErrCode | 诊断码：复用 RX7005 / 新码 | **新码 RX7020**（`toolchain.edition_unknown`，7xxx 续号，区别于 RX7005 类型/结构错）；edition 值类型错误复用 RX7005 |
| Q-File | spec 落点：新建 edition.md / 并入 toolchain.md | **新建 `spec/edition.md`**（独立成文，对齐 binding_layout.md 先例，RXS-0177~0180） |
| Q-Range | 条款数 | **4 条**（RXS-0177 声明 / RXS-0178 解析校验 / RXS-0179 诊断 / RXS-0180 stable 面关系） |
| Q-RD008 | RD-008 stable API 快照冻结机制：本里程碑激活 / 维持 not_frozen | **激活**——G2.5 语言 1.0 = 首个 stable 发布触发点（RD-008 backfill 条件兑现）；定义 stable 面（RXS 条款 ID 全集 + 冻结错误码 ID/含义 + edition 合法值集 + rx CLI 子命令面）+ 落快照比对（`ci/stable_snapshot.py` + `tests/stable/stable_api.snapshot`）+ bless 守卫（`check_guardrails` 新分支，镜像既有 golden bless）+ agent 自主 bless 首份快照；RD-008 status open→closed |
| Q-Stabilize | stabilization 流程 | 对齐 10 §5/§6 + 10 §2.2 FCP-lite：feature gate → tracking → 两里程碑无重大修订 → stabilization report → FCP-lite；首个 edition + 语言 1.0 stable 面本里程碑定型，后续里程碑续 |

## 10. 稳定化与 provenance

- **稳定化**（10 §5/§6）：首个 edition `"2026"` + 语言 1.0 stable 面经本 RFC + RXS-0177~0180 定型；stable 面（RXS 条款 ID 集 + 冻结错误码 + edition 值集 + rx CLI 子命令面）经 RD-008 激活落 stable 快照（`tests/stable/stable_api.snapshot`）+ bless 守卫。stable 面**语义**定型；快照**字节内容**为确定性回归锚、非语言 ABI 保证（不冻结为 stable）。stabilization report + FCP-lite advisory 公开等待窗按 10 §2.2，后续两里程碑无重大修订后推进。
- **Provenance**：本 RFC 实质内容 `Assisted-by: cursor:claude-opus-4.8`；agent 自主决策、批准、推进下游实现 PR；所有机器事实来自真实命令输出（硬规则 3），无 device（edition 编译期/host 面），CI run URL 不可达即诚实标注，不伪造。

## 11. 规范与实现依据

- **决策/规范**：11 §5（G2 = 语言 1.0 = spec 全量条款化 + conformance + 首个 edition）· 01 §6（使命/生态成功判据）· 10 §3（变更三档，Full RFC）/ §5（特性生命周期 D-404）/ §6（稳定面）/ §2.2（FCP-lite）· 04 P-01（strict-only）/ P-11（单一事实源）/ P-13（防 AI 幻觉治理）· AGENTS 硬规则 5/7。
- **依据决策**：13 D-308~D-311（rurix.toml manifest / rurix.lock 已锁格式）· RD-008（stable API 快照冻结机制激活，G2.5 候选触发点）。
- **关联 RFC**：无前置 RFC 依赖（edition 是新增独立面）；与 RFC-0002~0007（着色/DXIL/绑定/UC-04/采样）正交。
- **实现锚点**：`src/rurix-pkg/src/manifest.rs`（`Edition` 解析/校验 + RX7020）· `spec/edition.md`（RXS-0177~0180）· `conformance/edition/`（accept/reject fixtures）· `ci/stable_snapshot.py`（stable 快照比对 + bless）· `tests/stable/stable_api.snapshot`（首份快照）· `ci/edition_smoke.py`（CI 步骤 49 真实红绿）。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-30 | 初版 Full RFC：首个 edition 机制（`[package].edition = "2026"`，仅机制锚点空差异集，缺省兼容）+ edition 解析/校验/未知诊断 RX7020 strict-only + stable 面与 edition 关系 + stabilization 流程锚点（10 §5/§6/§2.2 FCP-lite）+ §5 RXS-0177~0180 条款投影 + §9 agent 自主裁决 Q-Name/Q-Scope/Q-Decl/Q-Default/Q-Mismatch/Q-ErrCode/Q-File/Q-Range/Q-RD008/Q-Stabilize + RD-008 激活裁决（open→closed，定义 stable 面 + 快照比对 + bless 守卫 + 首份 bless）。不触红线（D-008/SG-003 多后端 / SG-008 Python 嵌入 / D-312/SG-007 registry 维持 not_triggered）、不触 🔒 禁区（无 UB/内存模型/FFI ABI/安全包络）。Agent Approved 2026-06-30（完全自主，硬规则 1）；`Assisted-by: cursor:claude-opus-4.8`。 | Full RFC（Agent Approved） |
