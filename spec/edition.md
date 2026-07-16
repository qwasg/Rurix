# Rurix 语言规范 — edition 机制语义面（语义版本边界声明；G2.5 起）

> 条款:**RXS-0177 ~ RXS-0180**(G2.5 edition 机制语义面:edition 声明语义 / edition 解析校验规则 / edition 不匹配诊断 / stable 面与 edition 关系)。体例见 [README.md](README.md)。
> 依据:**[RFC-0008](../rfcs/0008-edition-stabilization.md)**(edition 机制与 stabilization 流程,agent Approved 2026-06-30);11 §5(语言 1.0 = spec 全量条款化 + conformance 覆盖 + 首个 edition);10 §3(变更三档,edition 为 Full RFC 触发面)/ §5(特性生命周期)/ §6(稳定面)/ §2.2(FCP-lite);04 P-01(strict-only);13 D-308~D-311(rurix.toml manifest / rurix.lock 已锁格式);RD-008(stable API 快照冻结机制激活,G2.5 候选触发点)。授权:[../milestones/g2/G2_CONTRACT.md](../milestones/g2/G2_CONTRACT.md)(D-G2-5,G-G2-5)+ [../milestones/g2/G2_PLAN.md](../milestones/g2/G2_PLAN.md) §5 G2.5 子里程碑。
> 档位:**Full RFC**(RFC-0008;10 §3:edition / stabilization 为显式 Full RFC 触发面)。RFC-0008 已由 agent 于 2026-06-30 自主批准(完全自主,AGENTS v3.0 硬规则 1)并裁决 §9 全部项。**agent 自主判档**,判档以 RFC-0008 与 G2_CONTRACT 授权为据,判档争议向上取严。**严禁 UB 节**(10 §7.5):未知 edition / edition 不匹配以编译期 RX7020 工具链诊断(P-01 strict-only,无运行期 fallback)定义,**本文件不触 🔒 禁区**(无 UB / 无内存模型映射 / 无 FFI ABI 二进制布局 / 无安全包络边界——edition 是纯编译期/host 工具链声明语义)。
> 规范先行(AGENTS.md 硬规则第 7 条):**条款 PR 先于实现 PR**;缺条款的语义 PR 必须先补 spec。`ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 >=1 测试锚定(`//@ spec: RXS-####`)。**本轮已落带编号条款体 RXS-0177~0180 + edition 解析/校验实现([`src/rurix-pkg/src/manifest.rs`](../src/rurix-pkg/src/manifest.rs))+ 每条 ≥1 测试锚定**(FLS 体例,见 §2)。

---

## 1. 范围与编号区间

本文件承载 **edition 机制** 的语义条款(G2.5+,D-G2-5)。edition 把"语义版本边界"作为 `rurix.toml` 清单的一等声明 `[package].edition`,为语言演进提供版本隔离边界,并锚定 stable 面与 stabilization 流程。

覆盖语义面(RFC-0008 §4 / §9):

- **edition 声明语义**:`[package].edition`(可选字符串),缺省取首个 edition `"2026"`(向后兼容,既有无 edition 字段清单 0-byte 不破坏)。
- **edition 解析/校验规则**:合法 edition 集合(首期 = `{ "2026" }`)冻结于 RFC-0008,解析为确定性纯函数,校验在 `Manifest::parse` 内联完成。
- **edition 不匹配/未知诊断**:声明的 edition 不在合法集合 → 编译期 **RX7020** strict-only 拒(无 fallback,P-01);edition 值类型错误(非字符串)复用既有 **RX7005**(清单类型错误,不新增码)。
- **stable 面与 edition 关系**:edition 是 stable 面的版本锚边界;语言 1.0 stable 面以首个 edition `"2026"` 为基准快照;同一 edition 内 stable 面只增不破坏,破坏性变更经新 edition 隔离。

首个 edition `"2026"` 定位为**机制锚点**:首期 edition-gated 行为差异 = **空集**(`"2026"` 与无 edition 声明行为完全一致),不引入任何破坏性 edition-gated 差异(RFC-0008 §9 Q-Scope)。

明确不在本文件落的范围:

- **第二 edition / 跨 edition 破坏性差异**:本期不引入,留未来里程碑经 Full RFC。
- **stable 快照字节内容冻结为语言 ABI 保证**:stable 快照是确定性回归锚(镜像 golden bless),register/字节布局/工具版本不冻结为 stable(对齐 [dxil_backend.md](dxil_backend.md) RXS-0162 / [binding_layout.md](binding_layout.md) RXS-0165 先例)。stable 快照机制本体由 RD-008 激活落 [`ci/stable_snapshot.py`](../ci/stable_snapshot.py) + bless 守卫,本文件仅条款化 edition ↔ stable 面**关系**。
- **registry / 多后端 / Python 嵌入**:edition 与三者正交(D-312/SG-007 registry / D-008/SG-003 多后端 / SG-008 Python 嵌入均维持 not_triggered)。

**编号区间**:本文件条款为 **RXS-0177 ~ RXS-0180**(全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1;当前最高现存 RXS-0176 @ [dxil_backend.md](dxil_backend.md))。本轮已落带编号条款体(下文 §2,FLS 体例),每条 ≥1 `//@ spec` 测试锚定(`src/rurix-pkg` edition 解析/校验单测 + `conformance/edition/` accept|reject fixtures)。区间登记于 [README.md](README.md) §4 文件清单。

## 2. 条款

> 本节落带编号条款体。各条按需分 **Syntax / Legality / Dynamic Semantics / Implementation Requirements** 节,**严禁 UB 节**(10 §7.5:未知 edition / edition 不匹配以编译期 RX7020 工具链诊断定义,P-01 strict-only,无运行期 fallback)。edition 解析/校验落 [`src/rurix-pkg/src/manifest.rs`](../src/rurix-pkg/src/manifest.rs)(纯 host/safe,零新 unsafe)。**本文件不触 🔒 禁区**:无 UB / 内存模型映射 / FFI ABI 二进制布局 / 安全包络边界——edition 是编译期/host 工具链声明语义。

### RXS-0177 edition 声明语义

`rurix.toml` `[package]` 表新增可选键 `edition`(字符串),声明包所用的语义版本边界。缺省(清单缺 `edition` 键)取首个 edition `"2026"`,保证既有无 edition 字段清单向后兼容(0-byte 不破坏)。

#### Syntax

```
package-table   = "[package]" , { package-key } ;
package-key     = ... | edition-key ;
edition-key     = "edition" , "=" , string-literal ;   (* 可选 *)
```

edition 声明为 `rurix.toml` 清单文法面的可选 `[package]` 键,值为字符串字面量。

#### Legality

- L1(可选声明):`[package].edition` 可缺省;缺省 = 首个 edition `"2026"`(`Edition::Edition2026`)。
- L2(值类型):`edition` 值须为字符串;非字符串(整数 / 布尔 / 表 / 数组)→ 复用 **RX7005**(`toolchain.pkg_manifest_invalid`,清单类型错误,与 name/version 类型错误同类,不新增码)。
- L3(合法值):字符串值的合法集合校验归 RXS-0178(未知值 → RX7020)。本条只承诺**声明形态 + 缺省语义 + 值类型**。

#### Dynamic Semantics

edition 声明为编译期清单解析面,本条无运行期语言语义。给定清单文本,`Manifest.edition` 对相同输入确定(缺 edition 键恒解析为 `Edition2026`)。

#### Implementation Requirements

- IR1(缺省兼容):[`Manifest::parse`](../src/rurix-pkg/src/manifest.rs) 在 `[package]` 缺 `edition` 键时置 `Manifest.edition = Edition::Edition2026`;既有无 edition 字段清单解析结果 0-byte 兼容(仅新增 `edition` 字段,既有字段不变)。
- IR2(值类型):`edition` 值非字符串 → `PkgError::ManifestInvalid`(RX7005);纯 host/safe,零新 unsafe。
- IR3(测试锚定):≥1 `//@ spec: RXS-0177`——`src/rurix-pkg` 单测(缺省 edition = `Edition2026` + 显式 `edition = "2026"` 解析 + 非字符串值 → RX7005)+ `conformance/edition/accept` fixture。

### RXS-0178 edition 解析/校验规则

edition 字符串值经确定性纯函数解析为内部 `Edition` 表示。首期合法 edition 集合 = `{ "2026" }`,集合冻结于 RFC-0008,新增 edition 经后续 Full RFC 扩展。

#### Syntax

edition 解析为清单语义面,非新增独立文法(承 RXS-0177 字符串值)。

#### Legality

- L1(合法集合):首期合法 edition 字符串集合 = `{ "2026" }`。`"2026"` → `Edition::Edition2026`。
- L2(确定性):`Edition::parse(&str) -> Result<Edition, EditionError>` 为确定性纯函数,无环境依赖、无 I/O;相同输入恒得相同结果。
- L3(集合外):合法集合外的字符串值 → strict-only 失败(诊断归 RXS-0179 RX7020)。

#### Dynamic Semantics

edition 解析为编译期确定性变换,本条无运行期语言语义。给定 edition 字符串,解析结果对相同输入确定。

#### Implementation Requirements

- IR1(纯函数解析):[`Edition::parse`](../src/rurix-pkg/src/manifest.rs)`(s: &str)` 对 `"2026"` 返回 `Ok(Edition::Edition2026)`,对集合外字符串返回 `Err(EditionError::Unknown)`;纯 host/safe,零新 unsafe、无 I/O。
- IR2(集合冻结):合法集合 `{ "2026" }` 与 RFC-0008 §4.2 一字对齐;扩展合法集合须经后续 Full RFC(本里程碑不引入第二 edition)。
- IR3(测试锚定):≥1 `//@ spec: RXS-0178`——`src/rurix-pkg` 单测(`Edition::parse("2026")` Ok + `Edition::parse(集合外)` Err + 确定性两次解析一致)+ `conformance/edition` fixture。

### RXS-0179 edition 不匹配/未知诊断

声明的 edition 不在合法集合(RXS-0178 L1)→ 编译期 **RX7020** strict-only 诊断拒绝,**无 fallback、无静默降级、不回退到缺省 edition**(P-01 strict-only)。

#### Syntax

edition 诊断为清单语义校验面,非文法面。

#### Legality

- L1(未知 edition):`[package].edition` 字符串值不在合法集合 `{ "2026" }`(如 `"2099"` / `"2015"` / `"latest"` / 空串)→ **RX7020**(`toolchain.edition_unknown`,7xxx 工具链段续号,接 RX7019,真实可达)。
- L2(strict-only):未知 edition 直接拒,**不回退缺省、不警告后继续**(P-01 strict-only,对齐 manifest 既有 RX7005 拒绝纪律);无运行期 fallback。
- L3(诊断内容):RX7020 诊断 message 含被拒的 edition 值 + 合法集合提示(`{detail}` 占位)。

#### Dynamic Semantics

edition 诊断为编译期确定性拦截,本条无运行期语言语义。给定未知 edition,RX7020 拒绝结论对相同输入确定。

#### Implementation Requirements

- IR1(strict 拒):[`Manifest::parse`](../src/rurix-pkg/src/manifest.rs) 对集合外 edition 经 `Edition::parse` 的 `Err(EditionError::Unknown)` 映射为 `PkgError::EditionUnknown`(code = **RX7020**),拒绝清单解析;纯 host/safe,无 fallback。
- IR2(诊断码):RX7020 = `toolchain.edition_unknown`,en/zh message-key 齐全(`ci/bilingual_coverage.py` 对齐);registry/error_codes.json append-only。
- IR3(测试锚定):≥1 `//@ spec: RXS-0179`——`src/rurix-pkg` 单测(未知 edition `"2099"` → `PkgError::EditionUnknown`.code() == "RX7020" + 不回退缺省的断言)+ `conformance/edition/reject` fixture(真实红绿)。

### RXS-0180 stable 面与 edition 关系

edition 是 stable 面的**版本锚边界**:语言 1.0 stable 面以首个 edition `"2026"` 为基准快照;同一 edition 内 stable 面只增不破坏(加性演进),破坏性变更须经新 edition 隔离。

#### Syntax

stable 面与 edition 关系为治理/稳定化语义面,非文法面。

#### Legality

- L1(版本锚):语言 1.0 的 stable 面(RXS 条款 ID 全集 + 冻结错误码 ID/含义 + edition 合法值集 + rx CLI 子命令面)以首个 edition `"2026"` 为基准定义。
- L2(加性演进):同一 edition 内 stable 面只增(新增条款/错误码/子命令)不破坏(既有不删除、含义不改,10 §6 / §9.5);破坏性变更须经新 edition 隔离(本期无第二 edition)。
- L3(快照非 ABI):stable 面**语义**经本条 + RD-008 激活定型;stable 快照([`tests/stable/stable_api.snapshot`](../tests/stable/stable_api.snapshot))**字节内容**为确定性回归锚(镜像 golden bless),**不冻结为语言 ABI 保证**(register/字节布局/工具版本不进 stable,对齐 RXS-0162 / RXS-0165 先例)。

#### Dynamic Semantics

stable 面与 edition 关系为编译期/治理面,本条无运行期语言语义。给定 stable 面定义,快照内容对相同输入确定(`ci/stable_snapshot.py` 重算确定性)。

#### Implementation Requirements

- IR1(快照机制):stable 面快照比对 + bless 守卫由 RD-008 激活落 [`ci/stable_snapshot.py`](../ci/stable_snapshot.py)(确定性生成/比对)+ [`tests/stable/stable_api.snapshot`](../tests/stable/stable_api.snapshot) + [`tests/stable/bless_log.md`](../tests/stable/bless_log.md) + `RURIX_BLESS=1` 路径 + `ci/check_guardrails.py` `check_stable_snapshot_bless` 守卫分支(镜像既有 UI/MIR/PTX/DXIL golden bless)。
- IR2(edition 锚):快照以首个 edition `"2026"` 为版本锚;edition 合法值集进快照内容(edition 演进经快照 diff + bless 受控)。
- IR3(非冻结):本条**不**把 register/字节布局/工具版本冻结为 stable;快照仅锚定稳定语言面的**存在性 + 含义**(条款 ID / 错误码 ID-含义 / edition 值 / 子命令名),非二进制保证。
- IR4(测试锚定):≥1 `//@ spec: RXS-0180`——`src/rurix-pkg` 单测(edition 合法值集作 stable 快照基准的存在性断言)+ `ci/stable_snapshot.py` 自检(快照确定性 + 篡改红绿)。

## 3. 裁决摘要与实现门控

- **Feature gate**:edition 解析/校验是 `rurix-pkg` 清单核心面,**无独立 cargo feature gate**(edition 是清单一等字段,缺省兼容,非可选编译面);stable 快照机制经 `ci/stable_snapshot.py` + `check_guardrails` 守卫 gate(RFC-0008 §6)。
- **错误码策略**:edition 未知诊断 = **RX7020**(`toolchain.edition_unknown`,7xxx 工具链段续号,接 RX7019,真实可达);edition 值类型错误复用既有 **RX7005**(不新增)。registry/error_codes.json append-only + en/zh message-key。
- **首个 edition 行为**:`"2026"` 仅机制锚点,edition-gated 行为差异 = 空集(RFC-0008 §9 Q-Scope);edition-gated 分发 hook 预留,首期所有查询返回"无差异"。
- **RD-008 激活**:本里程碑激活 stable API 快照冻结机制(RFC-0008 §9 Q-RD008):定义 stable 面 + 落快照比对 + bless 守卫 + agent 自主 bless 首份快照;RD-008 status open→closed(registry/deferred.json append-only)。
- **不触红线/禁区**:D-008/SG-003 多后端 / SG-008 Python 嵌入 / D-312/SG-007 registry 维持 not_triggered;无 UB / 内存模型映射 / FFI ABI / 安全包络边界。

---

## 4. 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-30 | 新建 edition.md（承 [RFC-0008](../rfcs/0008-edition-stabilization.md)，agent Approved 2026-06-30，G2.5 D-G2-5/G-G2-5）：登记文件名 + edition 机制语义面说明 + 带编号条款体 `### RXS-0177 ~ ### RXS-0180`（FLS 体例，按需分 Syntax / Legality / Dynamic Semantics / Implementation Requirements 节，**严禁 UB 节**）——RXS-0177 edition 声明语义（`[package].edition` 可选键，缺省首个 edition `"2026"`，向后兼容；值类型错误复用 RX7005）/ RXS-0178 edition 解析/校验规则（合法集 `{ "2026" }` 冻结于 RFC-0008，`Edition::parse` 确定性纯函数）/ RXS-0179 edition 不匹配/未知诊断（未知 edition → **RX7020** `toolchain.edition_unknown` strict-only 拒，无 fallback，P-01）/ RXS-0180 stable 面与 edition 关系（edition 作 stable 面版本锚边界，语言 1.0 stable 面以首个 edition `"2026"` 为基准，加性演进，快照字节内容非语言 ABI 保证）。配套 edition 解析/校验实现落 [`src/rurix-pkg/src/manifest.rs`](../src/rurix-pkg/src/manifest.rs)（纯 host/safe，零新 unsafe）+ 每条 ≥1 `//@ spec: RXS-####` 测试锚定（`src/rurix-pkg` 单测 + `conformance/edition/` accept|reject fixtures，**trace_matrix 176→180 全锚定**）。错误码 RX7020 落 `registry/error_codes.json`（7xxx 段续号，接 RX7019，append-only）+ en/zh message-key（bilingual 对齐）。首个 edition `"2026"` 仅机制锚点（edition-gated 行为差异 = 空集，RFC-0008 §9 Q-Scope）；RD-008 stable API 快照冻结机制经本里程碑激活（RFC-0008 §9 Q-RD008，stable 面定义 + 快照比对 + bless 守卫 + 首份 bless，RD-008 open→closed）。不触红线（D-008/SG-003 / SG-008 / D-312/SG-007 维持 not_triggered）、不触 🔒 禁区（无 UB / 内存模型映射 / FFI ABI / 安全包络）。 | **Full RFC**（RFC-0008） |
