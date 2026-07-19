# RFC-0014 — EI1 引擎集成期：`#[export(c)]` C ABI 导出 codegen + 内建头文件生成 + UC-05 最小 RHI/render graph

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0014（4 位制，编号永不复用，10 §9.5；G3_CONTRACT §7 v1.1 双轨分配：RFC-0013 = G3 单伞形，RFC-0014 = EI1 earmark） |
| 标题 | EI1 引擎集成期双面单 RFC：`#[export(c)]` 导出表 codegen + `--emit=dll` cdylib 通道 + 编译器内建头文件生成（Part A，语言/工具面）+ UC-05 最小 RHI + render graph 核心（Part B，旗舰应用面）+「同一组不变量：类型系统拦截 vs 计数器事后观测」对照报告 |
| 档位 | **Full RFC**（10 §3：**FFI ABI codegen 面**——`#[export(c)]` 从 `.rx` 源产 DLL 导出表 + 跨 ABI 运行期契约（AGENTS 硬规则 5）；RD-009 backfill_condition + spec/engine_integration.md 头注升档红线字面依据；判档争议向上取严 = Full，硬规则 8） |
| 状态 | **Agent Approved**（2026-07-19；§9.1 对抗性评审〔评审 provenance `claude-code:claude-opus-4-8` ≠ 起草 `claude-code:claude-fable-5`，跨模型三镜头 correctness/redline/implementability，D-409〕已完成，15 findings 逐条 disposition 落 §9.1，先于任何实现 PR，G-EI1-1） |
| 承接里程碑 | EI1（[milestones/ei1/EI1_CONTRACT.md](../milestones/ei1/EI1_CONTRACT.md)，验收门 **G-EI1-1 ~ G-EI1-6**；主线 EI1.1→EI1.5 串行，[EI1_PLAN.md](../milestones/ei1/EI1_PLAN.md)） |
| 关联条款 | 拟落 spec **RXS-0250 ~ RXS-0269**（earmark 区间，materialize 16 + 预留 4，见 §5）；**新建 spec/export_c.md**（Part A）+ **新建 spec/rhi.md**（Part B；与 G3.5 spec/render_graph.md 是不同面——后者是 G3 语言/图形面 render graph，本文 spec/rhi.md 是 UC-05 库面 RHI + compute-pass render graph，§7-2 论证） |
| 关联 deferred | **RD-009**（`#[export(c)]` C ABI 导出属性 + 编译器内建头文件生成 codegen；激活期兑现对象，EI1.2 落地后关闭或收窄余项另立 RD） |
| 依据决策 | D-113（FFI 战略 = `#[export(c)]` + 编译器内建头文件生成，cbindgen 角色内置化 P-11）· D-406 v2.0（agent 完全自主）· EI1_CONTRACT §7 ⑦（Q-A~Q-D 预记录）· RD-009 backfill_condition · 05 §11 · 02 §2 U5（:53-59 旗舰用例）· 06 §8.3（:149-151 affine 资源 + 生命周期 brand + C ABI 嵌入承诺）· RXS-0180 L3（符号面非 stable ABI）（13_DECISION_LOG.md 已锁决策，禁止重新发明） |
| Provenance | `Assisted-by: claude-code:claude-fable-5`（起草）。agent 自主决策；批准前置 = §9.1 对抗性评审完成 |
| Agent 批准 | **Agent Approved 2026-07-19**——§9.1 对抗性评审（评审 provenance `claude-code:claude-opus-4-8` ≠ 起草 `claude-code:claude-fable-5`，三镜头 correctness/redline/implementability，D-409）完成，10 major 正文实改 + 5 minor 措辞订正逐条 disposition（§9.1），先于任何实现 PR（G-EI1-1） |
| 对抗性评审 | **已完成 第 1 轮 2026-07-19**——见 §9.1；由与起草者 Provenance **不同**的模型执行三镜头（correctness/redline/implementability）批判性评审（评审 provenance `claude-code:claude-opus-4-8` ≠ 起草 `claude-code:claude-fable-5`，硬规则 2 可机验，`ci/check_contribution.py` 规则 4）；15 findings（10 major 正文实改 / 5 minor 措辞订正，0 blocker）逐条 disposition 后翻 Agent Approved |

---

## 1. 摘要

本 RFC 是 EI1 引擎集成期的**双面单 RFC**（镜像 RFC-0010 对 UC-07 的单 RFC 双角色承载）：一次对抗性评审、一次 Approved 合入即满足两面「RFC 前置」。

- **Part A（语言/工具面，→ RXS-0250~0255）**：兑现 RD-009——rurixc 把 `.rx` 源里 `#[export(c)]` 标注的 host `pub fn` 从 parser 桩（parsed-but-inert，`src/rurixc/src/parser.rs` 现仅解析不 codegen）转为**真实 C ABI 导出**：C 兼容签名子集 v1（标量 + 裸指针 + unit）合法性编译期校验 + 保名不 mangle + **link.exe `/EXPORT:` 参数发射**（从 typeck 导出集拼参，§9 Q-EmitMech）+ `--emit=dll` cdylib 通道（免 main host 编译，`/DLL` + import lib）+ **编译器内建头文件生成**（从同一 typeck C 映射确定性产 `.h`，单一事实源，cbindgen 内置化 D-113/P-11）。**rustc cdylib 端到端通路已验**（`build/spike-emit-dll/CONCLUSION.md`：`rustc --crate-type cdylib` → DLL + import lib + 未 mangle 导出表，link.exe 14.44 全在位）——**证 DLL/import-lib/导出表工具链在位、非 G2.2 DXIL 那种上游 blocked 面；但该 spike 走 rustc 内部 cdylib 导出机制，rurixc 自身「clang obj + driver 拼 `/EXPORT:`」集成路仍待兑现（impl 评审 EI1-IMPL-01，§6.3 首验闸口）**。
- **Part B（旗舰应用面，→ RXS-0256~0265）**：UC-05 最小 RHI（`Rhi`/`Queue`/`Res`/`Pass` 四件薄映射 std::gpu lang items RXS-0189/0190）+ render graph 核心（节点 = compute pass / 边 = 资源访问序推导 / 依赖环构建期拒 / 1-submit typestate 镜像 RXS-0197）——`apps/uc05-rhi` 全 `.rx`、零 `.rs`（主语言判据沿 MS1 最严口径），in-EXE demo device 真跑，再经 Part A 的 `#[export(c)]` 导出为 `rurix_rhi.dll` + 生成头，被自建 C++/D3D12 宿主（engine_host v2，升级 G1.3 `harness/engine_host.cpp` 母本）链接执行 graph compute pass。交付「同一组不变量：类型系统编译期拦截 vs 上一项目 Python 计数器事后观测」对照报告（I1~I10 矩阵，§4.B I1~I8 可拦截 / I9~I10 报告项）。

```
apps/uc05-rhi> rx build src/demo.rx     -o uc05_demo.exe        # in-EXE：graph ≥3 compute pass device 真跑
apps/uc05-rhi> rx build src/lib.rx --emit=dll -o rurix_rhi.dll  # export(c)：DLL + import lib + rurix_rhi.h（生成）
engine_host_v2.exe                                              # C++/D3D12 宿主链 DLL 执行 graph pass，数值对照
```

「引擎/外部采纳」维度显式 carve-out（EI1_CONTRACT out_of_scope `production_adoption_claim`）；达成表述 =「引擎集成工程闭环落地」，不宣称社会事实。

## 2. 动机

使命判据 U5（02 §2 :53-59）承诺「用 Rurix 实现一个最小 RHI + render graph 核心……『同一组不变量，类型系统拦截 vs 计数器事后观测』的对比报告」，采纳判据 = C ABI FFI 成熟 + 增量 check <5s；06 §8.3（:149-151）承诺「affine 资源 + 生命周期 brand + …C ABI（嵌入现存引擎）…UC-05 对照实验是此承诺的验收形式」，且明记「render graph/ECS 它们是库」——不进语言。UC-05 被 EA1 显式砍掉留下期（EA1_CONTRACT out_of_scope `uc05_minimal_rhi`）。

现状缺口硬核：**`.rx` 代码今天没有任意 host fn 的 DLL 导出出口**——rurixc 自身 link 路径只产 EXE（`src/rurixc/src/driver.rs:524-604` link 段恒 `/subsystem:console`，无 `/DLL`）；唯一既有 DLL 出口 `--emit=pyd`（RXS-0122）产的 `.pyd` 其 DLL 由外部 CMake/nanobind 打包、导出为 Python 模块 init 而**非任意 host fn**（driver.rs:353-393，非 rurixc link 路自产的通用 C ABI 导出面，impl 评审 EI1-IMPL-05）；手写 `extern "C"` 回退（RXS-0125 基座 + RXS-0149 引擎集成头↔ABI 守卫）**仅对 Rust crate 有效**；`#[export(c)]` 自 G1.3 起八连顺延（parser 桩 parsed-but-inert）。UC-05 是 `.rx` 单源库要导出为 DLL 被 C++ 工程链接，这是「需经 rurixc `#[export(c)]` 自动产 DLL 导出表 + 单一事实源内建头文件生成」的硬需求（RD-009 backfill_condition 原文）——手写路在本仓对 `.rx` 出口不成立。

**为何需要 Full RFC（而非 Direct/Mini）**：`#[export(c)]` 是 **FFI ABI codegen 面**——从 `.rx` 源产生 DLL 导出符号表 + 定义跨 ABI 运行期契约（unwind/panic 边界、well-defined 语义），触 AGENTS 硬规则 5（FFI ABI）；RD-009 backfill_condition 明记「按 10 §3 判档，FFI ABI codegen 触硬规则 5 则 Full RFC，向上取严」，spec/engine_integration.md 头注即升档字面依据。Part B 的 RHI/graph 本身零新语言语义（薄映射既有 std::gpu，06 §8.3「它们是库」），但其验收协议（不变量矩阵 100% 拦截判据 / 主语言判据 / 对照报告口径）与 Part A 同 RFC 承载（EI1_CONTRACT §7 ⑦ Q-B，镜像 RFC-0010 对 UC-07 角色）。

**通道工具链在位、rurixc 集成待兑现（spike 实证，§1）**：`--emit=dll` 曾标为 EI1.2 最高风险面（EI1_PLAN R1「常规但未在本仓验证」）；RFC 起草前排雷 spike（`build/spike-emit-dll/CONCLUSION.md`，2026-07-19，MSVC 14.44）经 **rustc cdylib 通路**确认 DLL + import lib + 未 mangle 导出表三件产出链路完整、link.exe 常规能力即可——**证工具链在位（rustc 通路），但 rurixc 自身「clang obj + driver 拼 `/EXPORT:`」集成路未在该 spike 覆盖，属 EI1.2 首验闸口（§6.3，impl 评审 EI1-IMPL-01）**，非上游 blocked 面。

## 3. 指导级解释（用户视角）

### 3.1 Part A — `#[export(c)]` 与 `--emit=dll`

`.rx` 侧标注 host 函数导出，`rx build --emit=dll` 产 DLL + import lib + **编译器生成的头文件**（用户不手写头）：

```rx
// rurix_math.rx —— C 兼容签名子集 v1（标量 + 裸指针 + unit）
#[export(c)]
pub fn rurix_add(a: i32, b: i32) -> i32 { a + b }

#[export(c, name = "rurix_store_out")]      // name= 覆写导出符号名
pub fn store(out: *mut i32, v: i32) { unsafe { *out = v; } }
```

```
rx build rurix_math.rx --emit=dll -o rurix_math.dll
# 产出：rurix_math.dll + rurix_math.dll.lib（import lib）+ rurix_math.h（生成头，LF/无时间戳/幂等）
```

生成的 `rurix_math.h` 每个声明 ↔ 恰一个 DLL 导出符号（保名不 mangle，`dumpbin /EXPORTS` 可见）；C 调用方 `#include "rurix_math.h"` + 链 `rurix_math.dll.lib` 即用。违例编译期拒：`#[export(c)]` 挂 device/kernel fn 或非 pub fn → 拒；签名含子集 v1 外类型（`repr(C)` struct 按值 / 回调指针 / 数组按值）→ 拒（§8 登 RD）；`--emit=dll` 无任何导出 → 拒。**头生成幂等**：同源两次生成逐字节一致（时间戳/路径不入头）。

### 3.2 Part B — UC-05 最小 RHI + render graph

打开 `apps/uc05-rhi/src/` 看到的全是 `.rx`：RHI 四件（`Rhi`/`Queue`/`Res`/`Pass`）薄映射 std::gpu，render graph 声明式建图，节点是 compute pass、边由资源读写声明推导：

```rx
let rhi = Rhi::create(&ctx);                 // brand 化根句柄（affine，沿 RXS-0189；opaque brand 类型 C）
let a = rhi.resource(n);                      // Res<C>：affine 资源句柄（per-instance 新鲜 brand C）
let b = rhi.resource(n);
let c = rhi.resource(n);
let mut g = rhi.graph();                       // Graph<C>
g.pass(scale_k).reads(&a).writes(&b);          // reads/writes 借 &Res（图内以资源 id/索引记账，非 move）→ 边推导
g.pass(add_k).reads(&b).reads(&a).writes(&c);  // &a 二次借用合法（借用非 use-after-move，故示例可编译，§4.B1）；b 写后读 → 自动建序
let done = g.submit();                          // 消费 g（Graph move-out，1-submit typestate，镜像 RXS-0197）
rhi.readback(c, &mut out);                      // 消费 c（Res move-out 点：I1 use-after-free / I2 double-free 的拦截锚，§4.B4）
```

用户不写任何 barrier/同步；漏声明访问、依赖环、跨 brand 误用、重复 submit 在 **`submit()` 装配期或编译期确定性 strict 拒**——不存在跑出错误图像或数据竞争的静默出口。kernel 保持编译期有界简单核（saxpy/scale/reduce 级，避开 RD-027 深弹射毒径，G3.1 归因结论届时并读）。同一套 `.rx` 库经 `#[export(c)]` 导出为 `rurix_rhi.dll` 被 C++/D3D12 宿主链接执行，无一行应用层 Rust/C++。

## 4. 参考级设计

### 4.0 双面一致性约定（单一事实源钉点）

1. **导出单一事实源 = typeck 的 C 映射**：`#[export(c)]` 导出集经 typeck 确定后，**同一份 C 映射既产 link.exe `/EXPORT:` 参数、又产内建头文件声明**（§4.A3/§4.A4，§9 Q-EmitMech）——导出符号与头声明恒逐一对应，无第二事实源（否决 obj dllexport 标注，§7-1）。**单一事实源正确性不止名字层**：`/EXPORT:` 只承载符号名，`.h` 承载完整 C 签名（类型/宽度/符号性/调用约定）——后者才是真正的 ABI 契约，其正确性由步骤 71 **类型层 ABI 往返 conformance**（per-type 哨兵值穿生成头往返，§4.A5）端到端机验，非仅 `dumpbin` 名字集 bijection（redline 评审 F6）。
2. **两条 C ABI 出口分工**：手写路 RXS-0125（`src/rurix-interop`）+ RXS-0149 守卫（`src/rurix-engine`，步骤 43）**冻结覆盖 Rust crate 出口**（语义 0-byte 只增）；生成路 `#[export(c)]` codegen + 内建头生成**覆盖 `.rx` 出口**——两制共存，判据升级与条款同 PR（§4.A5，EI1_CONTRACT guardrail）。
3. **RHI/graph 无新语言机制、零新 RX 码**：Part B 全为库面 + std::gpu 薄映射（06 §8.3「它们是库」）；affine/brand/typestate 复用既有裁决零新借用码（RXS-0189/0197），graph 构建期错误走**库层状态值零新 RX 码**（spec/imageio.md 先例，EI1_CONTRACT guardrail）。**细分（impl 评审 EI1-IMPL-03）**：I3 依赖环 / I5 写写冲突为**纯库层定长数组状态值**静态判（真零新码，无编译器介入）；**I4 未声明访问**的声明-反射相等核验须**编译器在 typeck/构建期喂 kernel 反射集**（`.rx` 无运行期反射，RD-026；镜像 graph.rs::with_reflection），故计入**语言/编译器面**（仍零新 RX 码，非纯库层零新码，见 §4.B2）——「零新 RX 码」不等于「Part B 零编译器承担」。
4. **合并序敏感号软化**：新 RX 码 / RD-035+ / trace 条数正文一律相对措辞或引 §5 预测表，以各 PR 合入时 registry/ledger/trace 再生实号为准（RXS 严格用 0250~0269 earmark 段）。

---

### 4.A Part A — `#[export(c)]` C ABI 导出 codegen + 内建头文件生成（G-EI1-2；RXS-0250~0255）

> 定位：兑现 RD-009。语言面新增 = `#[export(c)]` 属性由 parsed-but-inert 转正 + `--emit=dll` 目标；codegen 面新增 = 导出发射 + 头文件生成器。跨 ABI 运行期契约 **strict-only、well-defined、严禁 UB 节**（§4.A6）。

#### A1. `#[export(c)]` 属性合法性（→ RXS-0250）

- **仅 host `pub fn` 合法**：`#[export(c)]` 挂载对象须 `FnColor::Host`（沿 coloring）+ `pub` 可见性。挂 device/kernel fn、非 pub fn、非 fn item → **编译期 strict-only 拒**（属性误用，§5 预测新码，或复用 coloring RX3015 口径——实现期判真实可达类别，不预造）。parser 桩（`src/rurixc/src/parser.rs` 现解析 `#[export(c)]` 但零 codegen）转正为规范化校验入 resolve/typeck。
- **`name = "…"` 覆写键**：可选，覆写导出符号名（默认 = fn 名）；键/值非法 → strict 拒。

#### A2. C 兼容签名子集 v1 与类型映射（→ RXS-0251；§9 Q-SigSubset）

首期 C 兼容签名子集 **v1**（导出函数参数/返回类型全集）：

| Rurix 类型 | C 映射 | 备注 |
|---|---|---|
| 标量 `{i8/i16/i32/i64, u8/u16/u32/u64, f32/f64, bool}` | `int8_t…/uint…/float/double/bool` | 定宽整型 + IEEE 浮点 |
| 裸指针 `*mut T` / `*const T`（T ∈ 标量） | `T*` / `const T*` | **documented unsafe FFI boundary**（§4.A6，对齐 RXS-0125 口径）：codegen 侧不引入隐式解引用；体内解引用属用户 `unsafe`，指针有效性/对齐/别名为调用方前置条件（调用方自持缓冲、DLL 不分配-并-返回，§4.A4/§8 CRT-boundary 不变量） |
| unit `()`（仅返回位） | `void` | — |

子集 v1 **外**类型（`repr(C)` struct 按值 / 回调函数指针 / 数组按值 / 切片 / affine 句柄）出现在导出签名 → **编译期 strict-only 拒**（签名不兼容，§5 预测新码）；边界锁 §8，超界登 RD-035+。此判据是 `#[export(c)]` 的类型面守门，防未定义 ABI 布局静默逃逸。

#### A3. 导出符号发射：link.exe `/EXPORT:` 参数（→ RXS-0252；§9 Q-EmitMech，裁决 2）

- **保名不 mangle**：导出符号名 = fn 名（或 `name=` 覆写值），不施 Rust name mangling（`dumpbin /EXPORTS` 见裸名，spike 实证 `rurix_add`/`rurix_store` ordinal 1/2 未 mangle）。
- **发射机制 = driver 从 typeck 导出集拼 `/EXPORT:name` 传 link.exe**（**非 obj 内 `dllexport` 源标注**）：与内建头生成**同源单一事实源**（typeck C 映射既产 `/EXPORT` 参数又产 `.h`，§4.0-1）——契合 D-113/P-11 cbindgen 内置化。`/EXPORT:` 为 link.exe 常规链接器能力；**但 rurixc 自身「clang obj（无 dllexport 标注）+ driver 拼 `/EXPORT:`」集成路未在 spike 覆盖——spike 走 rustc 内部 cdylib 导出机制（§1），非 rurixc link 路，故该机制系 EI1.2 首验闸口（§6.3，impl 评审 EI1-IMPL-01），不以 rustc/cargo cdylib 充作 rurixc link 路已验先例**；否决 obj dllexport 见 §7-1（两事实源漂移风险）。
- driver link 段（`src/rurixc/src/driver.rs:524-604` 现产 EXE）扩展：`--emit=dll` 时改 `/DLL` + 拼 `/EXPORT:` 序列 + 生成 import lib（link.exe 副产 `<name>.dll.lib` + `.exp`）。

#### A4. `--emit=dll` cdylib 通道（→ RXS-0252）

- **免 main host 编译**：rurixc 现 host 路径要 `main`；`--emit=dll` 走 cdylib 语义（无 `main`，`/DLL`）——本仓**既有免-main 机制**是 `src/rurixc/src/driver.rs:206-216` 的 `device_emit` 分支（含 `pyd`/`nvptx-ir`/`ptx`），`dll` 入该 match 即得（比 `src/rurix-engine` rustc cdylib 先例更贴切，impl 评审 EI1-IMPL-05）。`--emit` 目标枚举（`driver.rs:44` 现 `check`/`mir`/`llvm-ir`/`nvptx-ir`/`ptx`/**`pyd`**，合法集见 driver.rs:434）加 `dll`；未知 `--emit` 维持拒（driver.rs:424/434 现纪律）。
- **CRT + 跨堆所有权红线（真实安全论证，非「与 EXE 一致」的范畴外类比）**：`--emit=dll` 用 libcmt（静态）；C 调用方链 import lib。EXE 无跨模块 CRT 边界（无人链接它），DLL 被异宿主加载才有——故不以「与现 EXE link 段一致」作理由。**真论证**：subset v1 **无任何堆/资源所有权跨 ABI 边界**（调用方自持全部缓冲，DLL 不分配-并-返回，见 §4.A2/§4.A6），故 `rurix_rhi.dll`（静态 libcmt）与宿主 engine_host v2（可能 `/MD` 动态 CRT）是否同源 CRT 对内存正确性**无影响**——跨堆 `malloc`/`free` 配对陷阱（`spec/interop.md`「Windows DLL 陷阱纪律」）在 v1 by-construction 不触发。该不变量作为红线登 §8（扩签名让 DLL 返回分配物前必先解，redline 评审 F2）。
- **空导出集诊断**：`--emit=dll` 但零 `#[export(c)]` 导出 → 编译期拒（空导出集，§5 预测新码）；link.exe `/DLL` 失败（退出码非零）→ 诊断（DLL 链接失败——**优先复用 RX7001**「外部工具链失败，含 link.exe 退出非零」/ RX7022 同族，§5.1 判为**条件**新码：仅当确需 export-table 上下文专诊断才另立，correctness 评审 F2）。

#### A5. 内建头文件生成 + RXS-0149 守卫升级（→ RXS-0253 生成器 / RXS-0254 守卫共存）

- **确定性生成器**（cbindgen 内置化，D-113/P-11）：从 typeck 导出集（§4.0-1 单一事实源）确定性产 `<out>.h`——**LF 行尾、无时间戳、无绝对路径、两次逐字节一致**（幂等判据，§9 Q-HeaderIdem）；每声明 ↔ 恰一 DLL 导出符号（无悬空声明 / 无未声明导出，承 RXS-0149 逐一对应口径）。
- **类型层 ABI 往返 conformance 硬门（redline 评审 F6，名字层 bijection 不足）**：`dumpbin /EXPORTS` 名字集 ↔ 头声明集逐一对应只证名字层；生成头的**类型映射**（`int32_t rurix_add(int32_t,int32_t)` 的宽度/符号性/位型）才是真正的 ABI 契约——错宽映射（如 `i64`→`int32_t`）在名字层 bijection + `cl.exe include` 编译下全绿却静默逃逸 = 边界 UB。故 subset v1 **每种类型各出一条 fixture，用在错误映射下会损坏的哨兵值穿生成头往返**：i64 传 >2³² 验宽度、i8/i32 负值验符号扩展、f32 vs f64 验位型、`*mut`/`*const` 读写——生成头声明宽度/符号性错误即**数值红**。该 per-type ABI 断言列**步骤 71 硬门（非仅 evidence）**，写入 RXS-0252/0253 测试锚定计划。
- **RXS-0149 守卫共存升级**（→ RXS-0254）：既有 RXS-0149（`src/rurix-engine`，`EXPORTED_C_ABI` 手写三符号 ↔ 随附手写 `rurix_engine.h`，步骤 43 host 段守卫）**手写路冻结覆盖 Rust crate 出口，语义 0-byte 只增**；`#[export(c)]` **生成路的守卫 = CI 再生成逐字节比对覆盖 `.rx` 出口**（入库生成头被篡改一字节 → 再生成 byte-diff 红，步骤 71 RED 路之一）。两守卫共存，判据升级与 RXS-0254 条款同 PR。

#### A6. 跨 ABI 运行期契约（→ RXS-0255；strict-only，严禁 UB 节）

一切不支持面编译期 strict-only 诊断；一切运行期语义 **well-defined**，全条无 UB 措辞：

- **无 panic 面 by-construction（跨 ABI panic 契约的诚实兑现，redline 评审 F1）**：subset v1 导出体仅 C 兼容算术（标量+裸指针+unit），**结构上不含任何可 panic 面**——无整数溢出检查触发点、无数组越界、无显式 `panic!`（`rurix_add` 仅 `a+b` 之类）。故 RXS-0255 **不是运行期「确定性终止契约」，而是编译期结构性保证**：`#[export(c)]` 导出函数体**禁含可 panic 面**（整数溢出检查 / 数组越界 / 显式 `panic!` 等），违者**编译期 strict-only 拒**（reject 语料 = 构造一个含 panic 面的导出体 → 编译期拒，步骤 71 RED）。「不 unwind 穿 C 帧」（C 侧 UB）由此 by-construction 保证，**无需（也无法构造）运行期「panic-across-ABI 确定性终止单测」**——删去该 YAML-only 空条款（§6.1 反 YAML-only）。后期扩签名若引入 panic 面（RD-035+），届时另钉唯一终止机制（推荐 per-export `catch_unwind→abort` 边界 shim，避免全程 `-C panic=abort` 污染 EXE 路）+ 补 RED 语料。10 §7.5 无 UB 口径。
- **裸指针责任边界 = documented unsafe FFI boundary（对齐 RXS-0125 口径，redline 评审 F5）**：`*mut T`/`*const T` 的有效性/对齐/别名为调用方（C 侧）前置条件，导出契约不承诺（等价任意 C 库裸指针面）——**「不解引用语义承诺」确切含义 = codegen 侧不引入隐式解引用；体内解引用属用户 `unsafe`（§3.1 `store` 示例），其内存正确性为调用方责任**。对账既有教条：本 subset 首次容许 caller-responsibility 裸指针面（通用 C 互操作必需，RXS-0125 opaque-u64 句柄不敷 host `int*` 场景），Rurix 侧 codegen 不引入额外 UB，故与「全条无 UB 表述」不冲突（框为 documented unsafe 边界，非 UB）；Rurix 侧不引入「有界非确定」内存序断言。
- **无返回值魔数**：unit 返回映射 `void`；错误信号由用户显式 `i32` 返回码承载（应用契约面，非语言）。

---

### 4.B Part B — UC-05 最小 RHI + render graph 核心 + 不变量对照（G-EI1-3/G-EI1-5；RXS-0256~0265）

> 定位：06 §8.3「它们是库」的库面兑现——RHI/graph 零新语言机制，全为 std::gpu 薄映射 + 库层状态值。`apps/uc05-rhi` 全 `.rx` 零 `.rs`（主语言判据沿 MS1 最严口径，ci/uc07_offline_golden_smoke.py:95-113 零 .rs 审计先例）。graph.rs（G3.5 `src/rurix-rt/src/graph.rs`，Rust `#![forbid(unsafe_code)]` 图形面）**仅设计参照非复用**（§7-2）。

#### B1. RHI 四件类型面与 brand（→ RXS-0256）

- `Rhi`/`Queue`/`Res`/`Pass` 为编译器 lang items **薄映射 std::gpu**（`Rhi::create(&Context)` 沿 RXS-0189 `Context` 底座；方法集经 typeck 已知签名分支 RXS-0190）；全部**非 Copy affine**——move/borrow 违例复用 RXS-0054/RXS-0057~0061 既有裁决（**零新借用码**）。
- **opaque 类型 brand（`Res<C>` / `Graph<C>` / `Buffer<C,T>`，沿 RXS-0189 opaque brand 类型，非「生命周期 brand `Res<'rhi>`」）**：`Rhi::create` **每实例合成新鲜 opaque brand 类型 `C`**（per-instance 新鲜 brand，编译期可拦截）；跨 `Rhi` 实例（brand A 的 `Res` 入 brand B 的 `Pass`）→ **编译期 context-brand 不匹配 RX3006**（复用 RXS-0189/RXS-0074 既有 brand 裁决，**非 RX2001**；I7）。**显式排除** RXS-0189 line 61「单 brand + cabi 运行期 context-id 校验」降级路径——该路系运行期拦截，取之则 I7 无法满足 I1~I8「100% 编译期/构建期拦截」判据（G-EI1-3），须改列 I9 类观测项；UC-05 取 per-instance 新鲜 brand 类型保 I7 ∈ 编译期集（correctness 评审 F1）。
- **方法所有权模式（impl 评审 EI1-IMPL-02，保 §3.2 示例可编译）**：`rhi.resource(n) -> Res<C>` 产 owned affine 句柄；`Graph::pass(k).reads(&res).writes(&res)` 的 `reads`/`writes` **取 `&Res` 借用**（非 move——否则 §3.2 `.reads(&a)` 二次借用即 use-after-move、示例编译不过），graph 在**无堆定长数组内以资源 id/索引**（非借用）记账多 pass 资源（RD-026 无堆约束，避自指借用结构）；`Graph::submit(self) -> Submitted` **move-out Graph**（I6）；`Res` 的 move-out 点 = `rhi.readback(res, &mut out)` / 显式释放（I1 use-after-free / I2 double-free 的实际消费锚——无此消费点则 I1/I2 by-construction 不可达而非「被拦截」，故点名钉死）。
- **着色合法性**：RHI 构造/方法出现在 `kernel`/`device fn` 体内 → **RX3015**（RXS-0189/0197 同点位，I8）。

#### B2. pass 声明与资源访问集（→ RXS-0257）

- `g.pass(kernel).reads(&res).writes(&res)` builder 方法链（逐方法即逐 typeck 已知签名分支，RXS-0190 先例；诊断 span 精确到单条访问声明）。访问种类首期封闭枚举：`read` / `write`。
- **未声明访问拦截**（I4，**编译器/语言面强制，非纯库层零新码；impl 评审 EI1-IMPL-03**）：判「pass 实际触碰未声明的 `Res`」须把 kernel 实际访问集与声明集比对——kernel 签名是**编译期知识**，`.rx` 无运行期反射（RD-026 无字符串/无集合/无反射），故 **reflected 集由编译器在 typeck/构建期喂入**（镜像 G3.5 `src/rurix-rt/src/graph.rs::with_reflection` line 307 由外部/编译器提供 kernel 签名反射集），再与声明集精确相等核验（漏声明 / 声明未用即失配，镜像 RX6030 口径）→ **构建期确定性 Err**。**承载论证更新**：I4 的声明-反射相等核验由编译器在 typeck/构建期喂 kernel 实际访问集承担，故 I4 拦截**计入语言/编译器面**（仍零新 RX 码，但非「纯库层定长数组状态值」的零新码面）；对照下 **I3（依赖环）/ I5（写写冲突）纯库层定长数组状态值即可静态判 = 真零新码**（不需编译器喂反射，见 B3）。

#### B3. graph 构建与依赖推导 + 依赖环拒绝（→ RXS-0258）

- **边推导**：写后读（RAW）/ 写后写（WAW）按声明序建 pass 序（声明全序无重排，RFC-0010 确定性口径）。
- **依赖环拒绝**（I3）：use-before-write 可达形态的环 → **构建期（`submit()`/装配期）确定性 strict 拒**（库层状态值零新码，镜像 G3.5 RX6029 口径）。
- **写写冲突**（I5）：同资源同序位多写者 / 写序违例 → 构建期确定性 Err。

#### B4. 资源生命周期 affine 拦截 + submit typestate（→ RXS-0259 生命周期 / RXS-0260 submit）

- **资源生命周期 affine 拦截**（I1/I2，→ RXS-0259）：`Res` affine——move 后再用（use-after-free 面，I1）/ 重复 move-out（double-free 面，I2）→ **编译期 move 违例 RX4001**（复用 RXS-0054，零新码）。
- **1-submit typestate**（I6，→ RXS-0260，**镜像 RXS-0197 present 消费式**）：`Graph` 消费式 `submit(self) -> Submitted`——**二次 submit = 编译期 move 违例 RX4001**（经引用消费 → RX4003）；跳态 / 非本态方法走既有方法查找 RX2004。**零新借用码、零新 RX 码**（RXS-0197 同模）。

#### B5. 执行语义 + transient 资源（→ RXS-0261 执行 / RXS-0262 transient）

- **执行语义**（→ RXS-0261）：顺序调度 + 显式 sync；运行期失败走 RXS-0193 确定性诊断封口（终止，不占 RX 码）。device 真跑数值对照 + 同机两跑逐字节确定。
- **transient 资源图内生命周期**（→ RXS-0262）：graph 内生资源容量编译期有界（const 泛型定长，RD-026 无堆集合对策，镜像 ruridrop 静态容量）；执行期实际峰值 ≤ 声明容量（I10 报告项，B6）。

#### B6. I1~I10 不变量矩阵（→ RXS-0263；裁决 1 划界）

**划界（消除 EI1_CONTRACT §1「I1~I10」vs 门「I1~I8」内部不一致）**：**I1~I8 = 可编译期/构建期 100% 拦截项**（逐条 reject 语料断言，入验收门 **G-EI1-3** / 步骤 73，漏拦即红）；**I9~I10 = 仅报告/观测对照项**（对标上一项目 Python 计数器事后观测，**不可静态拦截**，入对照报告 **G-EI1-5** / 步骤 75，`documented_historical` 口径）。矩阵逐条给：拦截机制 / 条款号或诊断码 / reject 语料路径样式 / 期望诊断 / 证据级 /（I9~I10）Python 引文占位。

| # | 不变量 | 拦截机制 | 条款 / 诊断码 | reject 语料路径样式 | 期望诊断 | 证据级 |
|---|---|---|---|---|---|---|
| **I1** | 资源生命周期 use-after-free（`Res` move 后再用） | affine 所有权（RXS-0189/0054） | RXS-0259 / **RX4001**（复用，零新码） | `conformance/uc05/reject/res_use_after_move.rx` | 编译期 move 违例 RX4001 | ci_checked（步骤 73） |
| **I2** | 资源重复销毁 / double-free（`Res` 重复 move-out） | affine（重复 move） | RXS-0259 / **RX4001**（复用） | `.../res_double_move.rx` | 编译期 move 违例 RX4001 | ci_checked |
| **I3** | pass 依赖环（use-before-write 可达形态） | graph 构建期确定性拒（库层状态值） | RXS-0258 / 库层状态 Err（镜像 RX6029 口径） | `.../graph_cycle.rx` | 构建期确定性 Err（装配拒） | ci_checked |
| **I4** | 未声明访问（读/写未在 pass 声明集内的 `Res`） | 声明-反射精确相等（库层状态值） | RXS-0257 / 库层状态 Err（镜像 RX6030 口径） | `.../pass_undeclared_read.rx` | 构建期确定性 Err | ci_checked |
| **I5** | 写写冲突（同资源同序位多写者 / 写序违例） | graph 构建期拒 | RXS-0258 / 库层状态 Err | `.../graph_write_write.rx` | 构建期确定性 Err | ci_checked |
| **I6** | 1-submit typestate 重复 submit（`Graph→Submitted` 后二次 submit） | 消费式 typestate（镜像 RXS-0197） | RXS-0260 / **RX4001**（复用 RXS-0054，零新码） | `.../graph_double_submit.rx` | 编译期 move 违例 RX4001 | ci_checked |
| **I7** | 跨 context/brand 资源误用（brand A `Res` 入 brand B `Pass`） | per-instance 新鲜 opaque brand 类型（镜像 RXS-0189 brand） | RXS-0256 / **RX3006**（复用 brand 裁决 RXS-0074/0189，非 RX2001） | `.../cross_brand_res.rx` | 编译期 context-brand 不匹配 RX3006 | ci_checked |
| **I8** | RHI 类型面着色合法性（RHI 构造/方法于 `kernel`/`device fn` 体内） | 着色合法性（RXS-0189/0197 同点位） | RXS-0256 / **RX3015**（复用） | `.../rhi_in_kernel.rx` | 编译期 RX3015 | ci_checked |
| **I9** | compute pass 数值结果正确性（GPU 输出 vs host 参考） | 运行期 device 数值对照（本质动态，**不可静态全证**） | RXS-0263 报告项 / 无诊断码 | evidence（步骤 72 数值对照） | GPU vs host checksum 一致 | measured_local；Python 侧 = **无数字的定性历史陈述**（上一项目代码/交接档不在仓库，EI1_PLAN R3；非可复跑、零杜撰数字，report.md 顶部醒目标注） |
| **I10** | transient 资源执行期实际峰值 / 生命周期（并发存活 vs 声明容量） | 运行期观测（**不可静态全证实际峰值**） | RXS-0263 报告项 / 无诊断码 | evidence（执行期计数） | 实际峰值 ≤ 声明容量 | measured_local；Python 侧 = 无数字的定性历史陈述（同 I9，非可复跑、零杜撰数字） |

> **对照口径（documented_historical，硬规则 3；redline 评审 F3 钉死）**：上一项目代码与 H01~H07 交接档**不在仓库**（已核实事实，EI1_PLAN R3）——故 `文件:行号` 引文会指向仓外不存在文件、其数字永不可由命令输出复核（正面顶撞硬规则 3「所有数字必须来自命令输出」），**取消对仓外源的伪引文格式**（防「看似可机验」的杜撰窗口）。I9~I10 的 Python 侧「计数器事后观测」= **无数字的定性历史陈述**（纸面对照）——`evidence/uc05_comparison_report.md` **顶部醒目标注**「historical counters unavailable in-repo, non-reproducible, no fabricated figures」，报告显式声明不可复跑 A/B、**零杜撰 Python 数字**；**schema 层（`check_schemas` 硬拦）禁止 I9/I10 出现无 in-repo 出处的数值字段**（RXS-0263/0264 测试锚定计划落）。Rurix 侧证据全 measured/ci_checked。对照核心论点：I1~I8 这组不变量上一项目靠运行期 Python 计数器事后捕获（部分漏到生产），Rurix 由类型系统/构建期 **100% 结构拦截**（编译期即不可构造违例）；I9~I10 本质动态（数值/执行期峰值），两侧同为观测面，Rurix 侧以 device measured 兑现。

#### B7. 对照报告证据形态 + 采纳判据（→ RXS-0264 报告 / RXS-0265 采纳判据）

- **对照报告证据形态**（→ RXS-0264，镜像 RXS-0134/0148 体例）：`evidence/uc05_invariant_matrix.json`（逐不变量：机制/条款号/reject 语料路径/期望诊断/CI 结果/证据级；**I9/I10 Python 侧为无数字定性历史陈述，schema 禁止无 in-repo 出处的数值字段**，redline 评审 F3）+ `milestones/ei1/uc05_invariant_matrix_schema.json`（入 `check_schemas` 硬拦）+ `evidence/uc05_comparison_report.md`（叙事面；**顶部醒目标注**「historical counters unavailable in-repo, non-reproducible, no fabricated figures」，纸面对照口径显式声明）。三方一致性机核（矩阵 ↔ reject 语料 ↔ report.md，步骤 75，防 YAML-only）。
- **采纳判据操作化**（→ RXS-0265，§9 Q-CheckBudget）：C ABI 成熟 = `#[export(c)]` 端到端（DLL + 生成头 + C 宿主真跑，G-EI1-4）；增量 check <5s = **双口径**——`ei1.bench.uc05_check_cold_ms`（`apps/uc05-rhi` 全包 `--emit=check` 冷全检，含磁盘 `mod` 解析，BENCH_PROTOCOL 三次 trimmed mean）+ `ei1.bench.uc05_check_warm_ms`（**进程/缓存预热后的全包 `--emit=check` 重跑**——**诚实标注全量重析、非 LSP 增量**：现 tooling session（`src/rurixc/src/tooling/session.rs::analyze`）只对单个内存文件 lex+parse+check_crate、无 `mod` 解析/磁盘加载，无法「增量」检全包 `apps/uc05-rhi`，故 warm 口径**不用 didChange→publishDiagnostics 增量路**、去「增量/incremental」措辞，impl 评审 EI1-IMPL-04；若坚持 LSP 增量则须把 tooling server 扩为整 crate 分析＝net-new 工作量，本期不取），阈 5000ms measured_local 回填。evidence 面不进 CI 硬门（计时波动，EA1 冷启动先例），SKIP 不充绿。

## 5. 下游 spec 条款映射（spec diff，10 §3 要件）

自 **RXS-0250** 起续号（main 现最高 RXS-0249 @ G3 五面；`registry/number_ledger.json` `next_free` = 250；earmark 区间 0250~0269，G3_CONTRACT §7 v1.1 固化）。**条款先行**（硬规则 7）：每 PR 条款 commit 先于实现 commit；每条 ≥1 `//@ spec:` 锚定；trace_matrix 全程全锚定；stable 快照加性重 bless 同 PR + bless_log 同 diff（步骤 49 硬红不可分 PR）；新文件修订表沿「表头『版本』列名、数据行用『版号』」纪律。**RXS 严格用 0250~0269 earmark 段；RXS-0266~0269 预留不落裸条款头**（close-out 作废声明留痕，burned 机制）。

| 条款（拟） | Part | 标题 | 落点 spec 文件 | 测试锚定计划（每条 ≥1） |
|---|---|---|---|---|
| RXS-0250 | A | `#[export(c)]` 属性语法与合法性（仅 host `pub fn`；device/kernel/非 pub 拒） | spec/export_c.md（新建） | conformance/export_c accept/reject（坏色/非 pub）+ UI golden |
| RXS-0251 | A | C 兼容签名子集 v1 与类型映射（标量+裸指针+unit；`name=` 覆写） | spec/export_c.md | reject（struct 按值/回调指针/数组按值）+ accept 子集 v1 |
| RXS-0252 | A | 导出符号表与 cdylib 产物（保名不 mangle + link.exe `/EXPORT:` 发射 + import lib + `--emit=dll`/`/DLL`） | spec/export_c.md | `.rx` fixture → DLL + import lib 产出单测 + dumpbin 未 mangle 断言 + **per-type ABI 往返 conformance 哨兵值断言（步骤 71 硬门，redline F6）** |
| RXS-0253 | A | 内建头文件生成（确定性 LF/无时间戳/两次逐字节一致；单一事实源 = typeck C 映射） | spec/export_c.md | 生成头幂等单测（两次 byte-eq）+ 步骤 71 C 宿主 include 编译 + **类型层 ABI 往返（i64 宽度 / i8·i32 符号扩展 / f32-f64 位型 / `*mut`·`*const` 读写哨兵，redline F6）** |
| RXS-0254 | A | 头↔ABI 守卫共存判据（手写路 RXS-0149 冻结覆盖 Rust crate；生成路 CI 再生成逐字节比对覆盖 `.rx`） | spec/export_c.md | 篡改入库生成头一字节 → 再生成 byte-diff 红（步骤 71 RED） |
| RXS-0255 | A | 跨 ABI 契约（**subset v1 无 panic 面 by-construction：导出体禁含可 panic 面的编译期结构性保证** + 裸指针 documented unsafe 边界 + 严禁 UB） | spec/export_c.md | **reject 语料：含可 panic 面的导出体 → 编译期拒**（步骤 71 RED）+ 无 UB 措辞核（**删去无法构造的运行期 panic-across-ABI 终止单测**，redline F1） |
| RXS-0256 | B | RHI 类型面与 brand（`Rhi`/`Queue`/`Res`/`Pass` 薄映射 std::gpu lang items；per-instance opaque brand；I7/I8） | spec/rhi.md（新建） | accept rhi_min + reject（cross_brand **RX3006** / rhi_in_kernel RX3015） |
| RXS-0257 | B | pass 声明与资源访问集（read/write；未声明访问 I4） | spec/rhi.md | accept pass_declared + reject pass_undeclared_read（库层状态 Err） |
| RXS-0258 | B | graph 构建与依赖推导（写后读/写后写建序；依赖环 I3 / 写写冲突 I5 构建期拒，库层状态值零新码） | spec/rhi.md | reject（graph_cycle / graph_write_write）+ accept graph_three_pass |
| RXS-0259 | B | 资源生命周期 affine 拦截判据（I1/I2） | spec/rhi.md | reject（res_use_after_move / res_double_move，RX4001 复用） |
| RXS-0260 | B | submit typestate（`Graph→Submitted` 消费式，1-submit，镜像 RXS-0197；I6） | spec/rhi.md | reject graph_double_submit（RX4001 复用）+ accept single_submit |
| RXS-0261 | B | 执行语义（顺序调度 + 显式 sync + RXS-0193 诊断封口） | spec/rhi.md | 步骤 72 device 数值对照 + 同机两跑逐字节确定 |
| RXS-0262 | B | transient 资源图内生命周期（const 泛型定长，RD-026 对策；I10 峰值观测源） | spec/rhi.md | 容量越界编译期拒单测 + 执行期峰值 evidence |
| RXS-0263 | B | I1~I10 不变量矩阵与 100% 拦截判据（裁决 1 划界；镜像 RXS-0134/0148 体例） | spec/rhi.md | 步骤 73 reject 矩阵逐条断言 + 步骤 75 矩阵↔语料一致性 + **schema 禁 I9/I10 无 in-repo 出处数值字段（redline F3）** |
| RXS-0264 | B | 对照报告证据形态（md + json schema + documented_historical 口径；report.md 顶部标注无仓内历史计数） | spec/rhi.md | 步骤 75 三方一致性互查 + schema 校验（无 in-repo 出处数值字段 `check_schemas` 硬拦） |
| RXS-0265 | B | 采纳判据操作化（C ABI 成熟 = export(c) 端到端 + check <5s 双口径：冷全检 + **预热后全包重析，非 LSP 增量**） | spec/rhi.md | `ei1.bench.uc05_check_{cold,warm}_ms` measured 回填（阈 5000ms，warm = 全量重析口径） |
| RXS-0266~0269 | — | **预留不落裸条款头** | — | close-out 作废声明留痕（burned 机制，MR-0006/0007 先例） |

### 5.1 新错误码策略（预测；合并时以 registry 实号为准，不预留不预造）

**前提**：6xxx 段自 **RX6031** 续（main 现最高 **RX6030**：G3.2 RX6027/6028 + G3.5 RX6029/6030 已落 main active〔RXS-0237〕，非「预测消费」，correctness 评审 F3；EI1 自 RX6031 续）；7xxx 工具段自 **RX7023** 续（`number_ledger` `next_free` = 7023）。**本表为预测**，materialize 时以合并时 registry 实号为准（先合入面的码落地会使后续预测号右移）；en/zh message-key 成对（bilingual 门）；registry/error_codes.json 只追加。**RXS 用 earmark 0250~0269；RX 码不预留具体号。**

| 章节 | 类别（归属场景） | 段位 | 需新码 | 状态 |
|---|---|---|---|---|
| §4.A1 | `#[export(c)]` 属性误用（挂 device/kernel/非 pub fn） | RX6031+ 段（或复用 coloring RX3015 口径） | ×1（条件——实现期判真实可达类别） | 条件 |
| §4.A2 | C 兼容签名子集 v1 违例（struct 按值/回调指针/数组按值） | RX6031+ 段 | ×1 | 确定 |
| §4.A4 | 空导出集（`--emit=dll` 零 `#[export(c)]` 导出） | RX6031+ 段 | ×1 | 确定 |
| §4.A4 | DLL 链接失败（link.exe `/DLL` 退出码非零） | **优先复用 RX7001/RX7022**（外部工具链 / `#[link]` 失败含 link.exe 退出非零，同族已覆盖）；仅需 export-table 专诊断才另立 RX7023+ 工具段 | ×1 | **条件（复用评估）** |

- **合计**：**需新码 ×N（≤4）**——**确定 ×2（签名不兼容 / 空导出集）+ 条件 ×2**（属性误用〔实现期判是否独立可达或复用 RX3015〕/ DLL 链接失败〔复用评估 RX7001/RX7022 是否覆盖 `--emit=dll` 的 `/DLL` 失败，correctness 评审 F2〕）；上限 4。**不定具体号**（不预留不预造，07 §5）。
- **Part B graph/RHI 构建期错误零新码**：依赖环（I3）/ 写写冲突（I5）走**纯库层状态值**，未声明访问（I4）由编译器喂反射集核验（§4.B2，仍零新 RX 码）（spec/imageio.md 先例，EI1_CONTRACT guardrail）；affine/typestate 违例复用 **RX4001/RX4003**、**brand 误用复用 RX3006（RXS-0074/0189，I7，非 RX2001）**、类型/元数/方法查找复用 **RX2001/RX2003/RX2004**、着色违例复用 **RX3015**（语义可加不可改）。
- **运行期/环境失败**（panic-across-ABI 终止、device 分配失败、DLL 装载失败）一律**不占 RX 码**（06 §8.2 / RXS-0193 口径）。

## 6. feature gate / tracking / 实现序（10 §3 要件）

### 6.1 前置与失败测试先行

- 本 RFC **Approved 合入先于任何实现 PR**（G-EI1-1，10 §3 硬性）；**失败测试先行**（反 YAML-only）：RFC 合入时点，`ci/export_c_smoke.py`（步骤 71）、`ci/uc05_rhi_smoke.py`（步骤 72）、`ci/uc05_invariant_gate.py`（步骤 73）、`ci/uc05_engine_embed_smoke.py`（步骤 74）、`ci/uc05_report_check.py`（步骤 75）、`apps/uc05-rhi/**`、spec/export_c.md、spec/rhi.md、RXS-0250~0265 条款体、`#[export(c)]` codegen / `--emit=dll` 在 main **均不存在 = RED**（脚本名为拟名，随实现 PR 定案）。

### 6.2 feature gate 总裁决

零新 cargo feature、零语言 gate：`#[export(c)]`/`--emit=dll` 为 host codegen 加性通道（既有 LLVM 文本 IR + link.exe 路，CUDA 路零回归）；RHI/graph 为 always-on 库面 + std::gpu 薄映射，无独立 gate。默认构建（全 feature off）零 GPU/SDK 依赖绿。

### 6.3 栈式 PR 计划（EI1.2→EI1.5 串行；条款 commit 先行 + 实现同 PR，EA1 #158/#159 结构先例）

- **EI1.2 export(c)（步骤 71，关键路径，本期最重单段）**：`PR-A`（条款+前端+后端+CI 不可分）—— spec/export_c.md RXS-0250~0255 + spec/README §4/§5 → attr 校验（parser 桩转正）+ 签名子集 typeck + 导出标记 → driver `--emit=dll` + `/EXPORT:` 发射 + import lib → 内建头生成器 → 步骤 71 `ci/export_c_smoke.py`（`.rx`→dll+头→cl.exe C 调用方 device 真跑 + 头幂等 + RED 三路：坏签名拒 / 篡改头 byte-diff 红 / 名冲突拒 + 内建 red_self_test）+ evidence json + `ei1.counter` 登记与 evaluator 分支同 PR。首 commit **最小 dll spike = 走 rurixc 产物（clang obj + driver 拼 `/EXPORT:`，非再跑 rustc）取证**——既有 `build/spike-emit-dll/` rustc cdylib spike 仅证 DLL/import-lib/导出表工具链在位，rurixc 自身「clang+link.exe `/EXPORT:`」集成路系本 commit 首验闸口（Approved 后真实可行性关卡，impl 评审 EI1-IMPL-01）。
- **EI1.3 RHI + graph（步骤 72/73）**：`PR-B1`（spec/rhi.md RXS-0256~0263 + 前端类型面 + `apps/uc05-rhi` 全 `.rx` + in-EXE demo device 真跑）+ `PR-B2`（conformance/uc05/reject 矩阵 I1~I8 逐条 + 步骤 73 不变量拦截门 + 步骤 72 demo 冒烟）。
- **EI1.4 引擎嵌入（步骤 74）**：`PR-C`—— `apps/uc05-rhi` 经 `#[export(c)]` 导出 `rurix_rhi.dll` + 生成头 → engine_host v2（升级 G1.3 `harness/engine_host.cpp`，C++/D3D12 + LUID 匹配，既有三符号/手写头/RXS-0149 守卫 0-byte）链接执行 ≥1 graph compute pass 数值对照 + 步骤 74。
- **EI1.5 对照报告 + close-out（步骤 75）**：`PR-D`—— spec/rhi.md 收口 RXS-0264/0265 → evidence 矩阵 json + schema + report.md + 步骤 75 三方一致性 → `ei1.bench.uc05_check_{cold,warm}_ms` 回填 → close-out 终审（RD-009 处置）。

### 6.4 每 PR 不变量核验（全期硬约束）

RXS-0125 手写 extern "C" / RXS-0149 守卫（步骤 43）/ `src/rurix-engine` 三符号面 / 手写头 **git diff 0-byte 只增**；LF byte-exact（新文件 LF+尾换行，禁 Python 文本模式写，逐文件核 CR+尾字节）；counter/entries 不预造（登记与 evaluator 分支同实现 PR）；device measured + run URL 归 EI1_CONTRACT §8；RURIX_REQUIRE_REAL 纪律贯穿步骤 71/72/74 device 段（mock/SKIP 不充绿）；trace 全程全锚定。

## 7. 备选方案

1. **obj 内 `dllexport` 源标注发射导出**（替代 link.exe `/EXPORT:` 参数）——**否决**：obj dllexport 标注与内建头生成会成**两个独立导出真相源**（codegen 侧标注 + 头生成侧枚举），漂移风险（一处改另一处忘）；`/EXPORT:` 从 typeck 导出集拼参数 = 与头生成**同源单一事实源**（§4.0-1，D-113/P-11 契合），`/EXPORT:` 为 link.exe 常规链接器能力（rurixc「clang obj + driver 拼 `/EXPORT:`」集成路系 EI1.2 首验，§6.3，impl 评审 EI1-IMPL-01）。
2. **render graph 复用 G3.5 `graph.rs`**（`src/rurix-rt/src/graph.rs`）——**否决**：`graph.rs` 是 Rust `#![forbid(unsafe_code)]` 的**图形面** render graph（G3 语言/运行时面），**不能进零 `.rs` 的 `apps/uc05-rhi`**（主语言判据硬约束，MS1 最严口径）；UC-05 render graph 是 **RHI 层新建 `.rx` 库**（compute-pass 面），`graph.rs` 仅**设计参照**（状态推导/依赖建序思路）非代码复用。两面概念重叠但定位不同（EI1_PLAN R6：UC-05 维持「库面 + 对照报告」不与 G3 语言/运行时面重复造轮）。
3. **继续手写 `extern "C"` 回退（不做 `#[export(c)]` codegen）**——**否决**：手写路（RXS-0125/0149）**仅对 Rust crate 有效**，`.rx` 单源无 DLL 出口；UC-05 是 `.rx` 库要导出，硬需求 `#[export(c)]`（RD-009 backfill_condition）。
4. **RHI 先行手写 ABI + `#[export(c)]` 后替换**——**否决**：`.rx` 代码今天没有任何 DLL 出口（driver.rs:524-604 EXE-only），此路在本仓不成立（EI1_PLAN §0 关键依赖洞察）；故 **export(c) 先行**（同时是最高风险 FFI ABI 面，硬规则 5，先做先暴露）。
5. **C 兼容签名子集 v1 直接含 `repr(C)` struct 按值 / 回调指针**——**否决（首期）**：struct 按值 ABI 布局（对齐/填充/寄存器分类）与回调指针（跨 ABI 调用约定）面显著扩风险，超首期 correctness-only 目标；边界锁 §8，超界登 RD-035+（加性方向，后期升级）。

## 8. 不做（范围红线）

| 不做项 | 理由（摘） | 登记去向 |
|---|---|---|
| `export_c_extended_signatures`（`repr(C)` struct 按值 / 回调函数指针 / 数组按值 / 切片） | 首期 C 兼容签名子集 v1 = 标量+裸指针+unit（§4.A2）；扩签名面 ABI 布局风险大 | **RD-035+**（合入时 deferred.json 续号，不预留） |
| `cross_boundary_heap_ownership`（**no cross-boundary heap/resource ownership transfer in v1**，CRT-boundary 陷阱不变量） | subset v1 调用方自持全部缓冲、DLL 不分配-并-返回（§4.A4/§4.A6）——静态 libcmt 与异宿主 CRT 不同源在 v1 无害，跨堆 `malloc`/`free` 配对陷阱 by-construction 不触发（`spec/interop.md`「Windows DLL 陷阱纪律」先例，redline 评审 F2） | **RD-035+ 扩签名前置守门**（让 DLL 返回分配物前必先解此不变量，合入时 deferred.json 续号） |
| `rhi_on_vulkan`（`.rx` 单源 Vulkan RHI 通道） | 首期 CUDA std::gpu 底座（rxrt_* PTX）+ engine_host v2（D3D12 嵌入）；rxrt C ABI CUDA-only（§9 Q-A） | **RD-031 open**（引用；硬需求另议，激活时复评 G3 vk descriptor 底座影响，EI1_CONTRACT out_of_scope） |
| `record_derive`（`Record` derive，05 §2.2 意向） | rurixc 零实现、spec 零条款；显式不进本期 | **RD-035+**（硬需求出现按 10 §3 判档另立） |
| `abi_stability_promise`（冻结 `#[export(c)]` 产物为语言级稳定 ABI） | 维持 RXS-0180 L3 口径（§10）；ABI 稳定承诺另期另裁 | 不立，随 RD-008 届时定义 |
| `python_pyd_integration`（export(c) 与 pyd/Python 联动） | UC-01 既有通道维持，本期不动 | 维持零登记 |
| 外部/引擎采纳宣称 | §1 诚实边界，carve-out 维持（EI1_CONTRACT `production_adoption_claim`） | 不立，carve-out |

## 9. 未决问题 / 关键裁决

编号规则：`Q-<名>`。Q-A~Q-D = EI1_CONTRACT §7 ⑦ 预记录（owner 2026-07-18 裁 + v1.1 2026-07-19 激活复核生效）；Q-Matrix / Q-EmitMech = owner 2026-07-19 确认（本 RFC 起草裁决 1/2）；余为 agent 拟裁（D-406 v2.0，批准即定案）。

| # | 裁决点 | 裁决 |
|---|---|---|
| Q-A | 执行底座 | **已裁 2026-07-18（v1.1 2026-07-19 复核生效）**：CUDA std::gpu 链（rxrt_* PTX）+ engine_host v2（C++/D3D12，G1.3 母本）嵌入侧，Vulkan 不进本期——复评注：G3 期 vk graphics descriptor/mesh/RT 底座为**图形着色面**，UC-05 RHI 首期为 **compute pass graph**，CUDA 底座判定不变，`rhi_on_vulkan` out_of_scope 维持（§8） |
| Q-B | UC-05 RHI 库面承载 | **已裁 2026-07-18/19**：随 RFC-0014 单 RFC 双面承载（镜像 RFC-0010 对 UC-07 角色）——Part A 语言/工具面 + Part B 旗舰应用面共一次对抗性评审、一次 Approved |
| Q-C | 对照报告入验收门 | **已裁 2026-07-18/19（owner 勾选）**：入 G-EI1-5（步骤 75）；documented_historical 纸面对照，零杜撰（§4.B6） |
| Q-D | export(c) 两面全做 | **已裁 2026-07-18/19（owner 勾选）**：导出表 codegen + 内建头生成两面全做，不分段（D-113 完整兑现；RD-009 激活期可关） |
| Q-Matrix | I1~I10 不变量矩阵划界（裁决 1） | **已裁 2026-07-19（owner 确认）**：**I1~I8 = 可编译期/构建期 100% 拦截项**（逐条 reject 语料断言，入 G-EI1-3 / 步骤 73）；**I9~I10 = 仅报告/观测对照项**（对标上一项目 Python 计数器事后观测，不可静态拦截，入 G-EI1-5 / 步骤 75，documented_historical）——**消除 EI1_CONTRACT §1「I1~I10」vs 门「I1~I8」内部不一致**（§4.B6） |
| Q-EmitMech | export(c) 导出表发射机制（裁决 2） | **已裁 2026-07-19（owner 确认）**：**link.exe `/EXPORT:` 参数**（driver 从 typeck 导出集拼参，非 obj dllexport 源标注）——与内建头生成**同源单一事实源**（typeck C 映射既产 `/EXPORT` 又产 `.h`，契合 D-113/P-11）；`/EXPORT:` 为 link.exe 常规能力，rurixc「clang obj + driver 拼 `/EXPORT:`」集成路系 EI1.2 首验（§4.A3，§6.3，impl 评审 EI1-IMPL-01；§7-1 否决 obj dllexport） |
| Q-SigSubset | C 兼容签名子集 v1 边界 | **agent 拟裁**：标量 + 裸指针（`*mut/*const T`，T ∈ 标量）+ unit（`void` 返回位）；`repr(C)` struct 按值 / 回调指针 / 数组按值 → 编译期 strict 拒 + §8 登 RD-035+（§4.A2） |
| Q-HeaderIdem | 内建头生成幂等口径 | **agent 拟裁**：LF 行尾 / 无时间戳 / 无绝对路径 / 两次逐字节一致；单一事实源 = typeck C 映射（§4.A5，守卫 = CI 再生成 byte-diff） |
| Q-CheckBudget | check <5s 双口径 | **agent 拟裁**：冷全检（`--emit=check` 全包）+ **预热后全包 `--emit=check` 重析**（诚实标注全量重析、**非 LSP 增量**——现 tooling session 单文件无 `mod` 解析，无法增量检全包，impl 评审 EI1-IMPL-04）双口径 measured，阈 5000ms；evidence 面不进 CI 硬门（计时波动，EA1 先例），SKIP 不充绿（§4.B7，RXS-0265 锁） |
| Q-GraphReuse | render graph 是否复用 G3.5 graph.rs | **agent 拟裁**：不复用——`graph.rs` 是 Rust `#![forbid(unsafe_code)]` 图形面，不能进零 `.rs` `apps/uc05-rhi`；UC-05 render graph 是 RHI 层新建 `.rx` 库，graph.rs 仅设计参照（§7-2） |

## 9.1 对抗性评审记录（对抗性评审要求，10 §3 / §7 · [`../13_DECISION_LOG.md`](../13_DECISION_LOG.md) D-409）

**已完成 第 1 轮 2026-07-19**——由与起草者 Provenance **不同**的模型执行三镜头（correctness / redline / implementability）批判性（对抗性）评审，**评审 provenance `claude-code:claude-opus-4-8` ≠ 起草 provenance `claude-code:claude-fable-5`**（硬规则 2 可机验，`ci/check_contribution.py` 规则 4 advisory 校验，跨模型镜头，D-409）。15 findings（10 major / 5 minor / 0 blocker）逐条 disposition：10 major **正文实改**、5 minor **措辞订正**，无驳回、无空过。状态 Draft → Agent Approved（先于任何实现 PR，G-EI1-1）。§9.2 攻击面为评审输入（A-1 由 redline F6 消化、A-3 由 redline F1 消化）。

| 字段 | 值 |
|---|---|
| 评审者 provenance | `Assisted-by: claude-code:claude-opus-4-8`（≠ 起草 `claude-code:claude-fable-5`，跨模型三镜头 correctness/redline/implementability，D-409/硬规则 2） |
| 评审轮次 | 第 1 轮，2026-07-19 |
| 结论 | **0 blocker / 10 major / 5 minor**；全部采纳——10 major 正文实改（I7 brand·panic 面·CRT 边界·单一事实源类型层·spike 降级·builder 所有权·I4 承载·warm 口径·DLL 链接失败码·矩阵纪律），5 minor 措辞订正（RX6030 前提·RXS-0249 落点·P-10 对账·裸指针边界·emit 枚举 pyd）；无驳回 |

**Findings 与 disposition**（每条一行；disposition：**采纳并修** §X ／ **驳回** + 理由；镜头前缀 C=correctness / R=redline / I=implementability 消歧同号）：

| # | Finding（评审者提出） | 严重度 | Disposition |
|---|---|---|---|
| C-F1 | I7 跨 brand 误用错配 RX2001——RXS-0189 既有裁决为 RX3006、且「单 brand 方案」是运行期 context-id 校验，与 I7∈100% 编译期拦截集互斥，动摇对照报告核心论点 | major | **采纳并改 §4.B1 / §4.B6 I7 行 / §5 RXS-0256**：I7 复用 **RX3006**（非 RX2001）；「生命周期 brand `Res<'rhi>`」更正为 opaque 类型 brand `Res<C>`/`Buffer<C,T>`；UC-05 采 **per-instance 新鲜 brand 类型**（编译期可拦截，满足 I7∈I1~I8/G-EI1-3），显式排除 RXS-0189 line 61 单 brand 运行期降级路 |
| C-F2 | 「DLL 链接失败」判为「确定」新码，与既有 RX7001/RX7022 覆盖面冲突且违复用纪律 | major | **采纳并改 §5.1 / §4.A4**：DLL 链接失败下调为「条件（复用评估）」——论证 RX7001/RX7022 是否覆盖 `/DLL` 失败；合计改「确定 ×2 + 条件 ×2」 |
| C-F3 | §5.1「main 现最高 RX6026 / 预测消费 RX6027~6030」过期——G3 已 close，RX6027/6028/6029/6030 均 active | minor | **采纳并改 §5.1 前提行**：改「main 现最高 RX6030（G3.2/G3.5 已落 active，RXS-0237）」，删「预测消费」措辞 |
| C-F4 | §11 把 RXS-0249 挂到 spec/release.md，实际位于 spec/dxil_backend.md，release.md 尾条款为 RXS-0219 | minor | **采纳并改 §11**：改「spec/dxil_backend.md（RXS-0249 = G3.6 DXIL 腿，RXS 区间尾）」+ release.md 标真实尾 RXS-0219 |
| R-F1 | 跨 ABI panic 契约（本 RFC 存在理由）机制未钉（abort/panic=abort/catch_unwind 三选留空）+ 自陈无 RED 语料可测 = YAML-only 风险 | major | **采纳并改 §4.A6 / RXS-0255 / §5 / §9.2 A-3**：subset v1 诚实声明「无 panic 面 by-construction」；RXS-0255 改**编译期结构性保证**（导出体禁含可 panic 面 + reject 语料）；删无法构造的运行期终止单测 |
| R-F2 | 静态 libcmt 理由「与现 EXE 一致」是范畴错误；跨堆所有权红线未在 §8 锁定，Windows DLL CRT 陷阱纪律零引用 | major | **采纳并改 §4.A4 / §8**：§4.A4 补真实安全论证（subset v1 无跨 ABI 堆/资源所有权转移）；§8 登记红线「no cross-boundary heap/resource ownership transfer in v1」作 RD-035+ 前置守门，引 spec/interop.md「Windows DLL 陷阱纪律」先例 |
| R-F3 | I9/I10 的 `文件:行号` 引文指向仓外 H01~H07，不可机验，结构性顶撞硬规则 3「数字须来自命令输出」且「严禁杜撰」无机制兜底 | major | **采纳并改 §4.B6 I9/I10 + 对照口径注 + §4.B7 / RXS-0263/0264**：取消仓外源伪引文格式，降为无数字定性历史陈述；report.md 顶部醒目标注「historical counters unavailable in-repo…」；schema 层禁 I9/I10 无 in-repo 出处数值字段（`check_schemas` 硬拦） |
| R-F6 | 「单一事实源」bijection 仅名字层——生成头的类型映射（真 ABI 契约）无端到端机验，错宽映射静默逃逸 = 边界 UB | major | **采纳并改 §4.A5 / §4.0-1 / RXS-0252/0253**：守卫升级为**类型层 ABI 往返 conformance**（subset v1 每类型哨兵值穿生成头往返，宽度/符号性错即数值红），列步骤 71 硬门，写入测试锚定计划 |
| R-F4 | §10 未对账 10 §6 P-10「C ABI 导出约定」为 1.0 stable 面，把导出约定一并推给 RD-008 称非稳定 = 相对 P-10 欠承诺 | minor | **采纳并改 §10**：对账 P-10，区分「单 DLL 符号字节布局不冻结（RXS-0180 L3）」vs「导出约定（命名不 mangle + subset v1 类型映射 + 调用约定）= P-10 stable 面候选，经 RD-008 定型」 |
| R-F5 | 裸指针面未像 RXS-0125 那样显式框为 documented unsafe boundary；「不解引用语义承诺」歧义、与 §3.1 `store` 示例矛盾 | minor | **采纳并改 §4.A2 / §4.A6**：框为 documented unsafe FFI boundary（对齐 RXS-0125）；澄清「不解引用」= codegen 不引入隐式解引用、体内解引用属用户 unsafe 前置条件；对账既有教条 |
| I-EI1-IMPL-01 | 「spike 已验 link.exe 支持 /EXPORT:」系 overclaim——spike 走 rustc cdylib，rurixc clang+link.exe /EXPORT: 路零实测 | major | **采纳并改 §1 / §4.A3 / §9 Q-EmitMech / §6.3**：降级为「rustc cdylib 通路已验；rurixc /EXPORT: 集成待兑现」；§6.3 首 commit dll spike = 走 rurixc 产物 + driver 拼 /EXPORT:（非再跑 rustc）作 EI1.2 首验 |
| I-EI1-IMPL-02 | reads()/writes() 若 move Res 则 §3.2 示例二次 `.reads(a)` use-after-move 编译不过；builder 所有权模式未规定、I1/I2 消费点未定 | major | **采纳并改 §4.B1 / §3.2**：reads()/writes() 取 &Res 借用；submit move-out Graph；readback/释放为 Res move-out 点（I1/I2 锚）；graph 无堆定长数组以资源 id/索引持资源；§3.2 示例改 `.reads(&a)` 确保可编译 |
| I-EI1-IMPL-03 | I4 判未声明访问须编译期反射 + set-difference（graph.rs::with_reflection 实证），「库层状态值零新码」不成立 | major | **采纳并改 §4.B2 / §4.0-3 / §4.B6 I4**：I4 reflected 集由编译器 typeck/构建期喂入（镜像 with_reflection line 307），拦截**计入语言/编译器面**（仍零新 RX 码，非纯库层）；I3/I5 纯库层定长数组状态值 = 真零新码 |
| I-EI1-IMPL-04 | warm check「增量」名不副实——现 tooling session 单文件、无 mod 解析/磁盘加载、Full-sync 全量重析，无法增量检全包 | major | **采纳并改 §4.B7 / §9 Q-CheckBudget / RXS-0265**：warm 口径去「增量/incremental」，改「进程/缓存预热后的全包 --emit=check 重跑（诚实标注全量重析）」；LSP 增量须扩整 crate 分析＝net-new，本期不取 |
| I-EI1-IMPL-05 | emit 枚举漏 pyd（driver.rs:434 含 pyd）；「没有任何 DLL 出口」不精确（pyd 产 .pyd DLL）；免-main 先例引错 | minor | **采纳并改 §2 / §4.A4**：emit 枚举补 pyd；「rurixc 自身 link 路径只产 EXE；pyd 的 DLL 由外部 CMake/nanobind 打包、导出非任意 host fn」；免-main 引 driver.rs:206-216 device_emit 分支为先例 |

## 9.2 已知风险与评审攻击面（起草侧自暴，供 §9.1 评审镜头用）

> 供 §9.1 对抗性评审输入（correctness / redline / implementability 三镜头参考，D-409）。**评审已消化（2026-07-19）**：本节所列攻击面已由第 1 轮三镜头评审逐条覆盖，disposition 见 §9.1；下列条目补「评审已消化」指针，正文钉死位置随附。

**Part A（export(c)）**
- **A-1 `/EXPORT:` 参数 vs 生成头单一事实源真独立性**：两者都从「typeck 导出集」派生，但拼参与头文本是两处独立实现——若一处漏字段另一处不知，「同源」沦为形式。评审镜头 = 是否有机核（dll `dumpbin /EXPORTS` 符号集 ↔ 生成头声明集逐一对应断言，步骤 71）。**〔评审已消化：redline F6——名字层 bijection 不足，守卫升级为类型层 ABI 往返 conformance 硬门（哨兵值穿生成头往返），§4.0-1 / §4.A5 / RXS-0252/0253。〕**
- **A-2 签名子集 v1 越界的类型面覆盖完整性**：`repr(C)` struct 按值 / 回调指针 / 数组按值 / 泛型 / affine 句柄逃逸——reject 语料是否覆盖全部越界形态，漏一类即 ABI 布局静默逃逸。
- **A-3 panic-across-ABI「确定性终止」的落地机制**：abort vs 进程退出 vs catch_unwind→i32 的选择在 `.rx` 无 panic 面时是否可测；RED 语料如何构造（`.rx` 无显式 panic 语法则该条款为前置锁定，评审应核是否 by-construction）。**〔评审已消化：redline F1——subset v1 导出体无 panic 面 by-construction，RXS-0255 改编译期结构性保证（禁含可 panic 面 + reject 语料），删无法构造的运行期终止单测，§4.A6。〕**
- **A-4 空导出集 / DLL 链接失败诊断的段位归属**：属性误用/签名不兼容（编译期前端）vs 空导出集（driver 装配期）vs link 失败（工具段）——RX 码段位（6xxx vs 7xxx）划分是否与既有 07 §5 段位语义一致。

**Part B（RHI/graph）**
- **B-1 I1~I8「100% 拦截」的 by-construction 论证寿命**：affine/typestate/brand 拦截依赖既有借用检查 + 库层状态值；graph 依赖环/写写冲突「构建期确定性拒」是否对所有可构造违例形态闭合（镜像 G3.5 RXS-0237 环检测可达性质询）。
- **B-2 I9~I10 报告项的对照说服力**：上一项目代码不在仓库，Python 侧全 documented_historical 纸面引文——「计数器事后观测 vs 类型系统拦截」对照是否沦为单边叙事；评审应质询报告口径是否诚实标注不可复跑 A/B（零杜撰）。
- **B-3 库层状态值零新码 vs 诊断可定位性**：依赖环/未声明访问走库层状态 Err（非 RX 码），诊断 span 能否定位到违例 pass（对比 G3.5 RX6029/6030 编译期码）；库层状态口径是否弱化 strict-only 承诺。
- **B-4 RHI 薄映射 std::gpu 的语言面零扩张真实性**：`Rhi`/`Queue`/`Res`/`Pass` 是否真为库面 lang item 薄映射（06 §8.3「它们是库」），有无隐性新语言机制（新借用码/新 typestate 语义）潜入。
- **B-5 主语言判据零 `.rs` 与 export(c) 产物边界**：`apps/uc05-rhi` 全 `.rx`，但 engine_host v2 是 C++——零 `.rs` 审计边界（应用包内 vs 嵌入宿主）是否清晰，避免「宿主 C++ 混入应用主语言判据」误判。

## 10. 稳定化与 provenance

- **稳定化**（10 §5）：**对账 10 §6 P-10「C ABI 导出约定」stable 面**（RFC-0014 系首次物化该约定，redline 评审 F4）——须区分两层：（i）**单个 DLL 的符号字节布局 / 生成头 ABI / 工具版本 = 非稳定**，维持 **RXS-0180 L3** 口径（符号面是工具链内部实现要求，非用户 stable ABI，镜像 RXS-0149/0162/0165 先例）；（ii）**导出约定本身（命名不 mangle 规则 + subset v1 类型映射 + 调用约定）= P-10 所列「C ABI 导出约定」1.0 stable 面的候选**（用户会依赖），其稳定化经 **RD-008** 定型——**非把整个 C ABI 导出面一并判为非稳定**（否则相对 P-10 欠承诺/冲突）。§8 `abi_stability_promise` carve-out 仅冻结 (i) 层字节布局不作语言 ABI 承诺，不否认 (ii) 层约定属 P-10 stable 面候选。Part B RHI/graph 条款随 stable 快照加性重 bless（RXS-0180 L2 只增不破坏）。FCP-lite（advisory 公开等待窗）下公开，agent 自主裁决合入。
- **Provenance**：`Assisted-by: claude-code:claude-fable-5`（起草）。agent 自主决策；批准前置 = §9.1 对抗性评审完成（评审 provenance ≠ 起草，D-409/硬规则 2），批准后推进 §6.3 下游实现 PR。

## 11. 规范与实现依据

- **仓内**：02_USERS_AND_USE_CASES.md §2 U5（:53-59）；06_GPU_GRAPHICS_PROGRAMMING_MODEL.md §8.3（:149-151）；05 §11 + 13_DECISION_LOG.md D-113；milestones/ei1/{EI1_CONTRACT.md（§7 ⑦ Q-A~Q-D 预记录）,EI1_PLAN.md,CI_GATES.md（步骤 71~75）}；registry/{deferred.json（RD-009）,error_codes.json,number_ledger.json（RXS next_free=250）,spike_gating.json}；spec/interop.md（RXS-0125 手写 C ABI 基座）、spec/engine_integration.md（RXS-0149 头↔ABI 守卫 + 步骤 43 + `EXPORTED_C_ABI`）、spec/host_orchestration.md（RXS-0189/0190 std::gpu lang items + RXS-0197 present typestate + RXS-0193 诊断封口）、spec/edition.md（RXS-0180 L3）、spec/dxil_backend.md（RXS-0249 = G3.6 DXIL 腿，RXS 区间尾，correctness 评审 F4）、spec/release.md（RXS-0219 = EA1.2 尾条款）；rfcs/0009（single-source 宿主编排）、rfcs/0010（UC-07 单 RFC 双角色先例 + 主语言判据操作化 + documented_historical 口径）、rfcs/0013（G3 伞形 + §9.1 对抗性评审格式）；src/rurixc/src/{parser.rs（`#[export(c)]` parsed-but-inert）,driver.rs（:44 emit 枚举 / :524-604 EXE-only link 段）}；src/rurix-engine/{src/lib.rs（`EXPORTED_C_ABI`）,harness/engine_host.cpp（v1 saxpy + LUID，engine_host v2 母本）,include/rurix_engine.h（手写头）}；src/rurix-rt/src/graph.rs（G3.5 图形面 render graph，设计参照非复用）；ci/uc07_offline_golden_smoke.py（:95-113 零 .rs 审计先例）；build/spike-emit-dll/CONCLUSION.md（`--emit=dll` 通道可行性 spike，工件驻 build/ 不入库）。
- **外部**：MSVC link.exe `/DLL` / `/EXPORT:` / import lib 机制；DLL 导出表 / `dumpbin /EXPORTS`；C ABI 调用约定与裸指针语义；cbindgen（头生成角色，D-113 内置化对标）。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v1.0 | 2026-07-19 | AI 起草初版（EI1.1）：双面单 RFC（Part A `#[export(c)]` 导出 codegen + `--emit=dll` + 内建头生成 → RXS-0250~0255；Part B UC-05 最小 RHI + render graph + I1~I10 不变量对照 → RXS-0256~0265；预留 RXS-0266~0269）。落两起草裁决——裁决 1（Q-Matrix：I1~I8 可拦截入 G-EI1-3 / I9~I10 报告项入 G-EI1-5，消除契约 §1 vs 门内部不一致）、裁决 2（Q-EmitMech：link.exe `/EXPORT:` 参数机制，与内建头生成同源单一事实源）；Q-A~Q-D 预记录回填（已裁 2026-07-18/19）；§5.1 新码策略（需新码 ≤4，RX6031+/RX7023+ 段，不预留具体号）；§7 备选（obj dllexport / graph.rs 复用否决）；§8 超界登 RD-035+。`--emit=dll` 通道可行性 spike 实证注入（build/spike-emit-dll/CONCLUSION.md）。状态 **Draft**：Agent Approved 待 §9.1 对抗性评审（评审 provenance ≠ 起草）后翻，先于任何实现 PR（G-EI1-1） | Full RFC（Draft） |
| v1.1 | 2026-07-19 | **对抗性评审 disposition 落实（第 1 轮，评审 provenance `claude-code:claude-opus-4-8` ≠ 起草 `claude-code:claude-fable-5`，三镜头 correctness/redline/implementability，D-409/硬规则 2）**：15 findings（10 major 正文实改 + 5 minor 措辞订正，0 blocker）逐条落 §9.1。**FFI 红线钉死**——① panic 面：subset v1「无 panic 面 by-construction」，RXS-0255 改编译期结构性保证 + reject 语料（§4.A6）；② CRT 边界：静态 libcmt 真实安全论证（无跨 ABI 堆/资源所有权转移）+ §8 登「no cross-boundary heap/resource ownership transfer」红线（§4.A4/§8）；③ 单一事实源类型层：守卫升类型层 ABI 往返 conformance 硬门（哨兵值穿生成头，§4.0-1/§4.A5/RXS-0252/0253）；④ I7 brand：RX2001→RX3006 + opaque per-instance brand `Res<C>`，排除 line 61 运行期降级路（§4.B1/§4.B6）。另落：spike overclaim 降级 + rurixc 首验闸口（§1/§4.A3/§6.3）、builder 所有权模式（§3.2/§4.B1）、I4 承载计入编译器面（§4.B2/§4.0-3）、warm check 去「增量」改预热全量重析（§4.B7/RXS-0265）、DLL 链接失败 确定→条件（§5.1）；五 minor 订正（RX6030 前提 / RXS-0249 落点 / P-10 对账 / 裸指针 documented unsafe 边界 / emit 枚举补 pyd）。**状态 Draft → Agent Approved（2026-07-19，先于任何实现 PR，G-EI1-1）** | Full RFC（Agent Approved） |
