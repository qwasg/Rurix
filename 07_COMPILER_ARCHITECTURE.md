# 07 — 编译器架构

> 所属文档集：[00_MASTER_INDEX.md](00_MASTER_INDEX.md)
> 版本：v1.0（2026-06-11）
> 主要输入：r1（rustc 解剖）、r2（NVPTX 链路）、r3（MLIR 评估）、r9（诊断/LSP）、r6（Windows 工具链）
> 关联决策：D-201 ~ D-210（见 [13](13_DECISION_LOG.md)）
> 编译器代号：`rurixc`。实现语言：**Rust**（D-201：借用检查器/IR/LLVM 绑定生态最成熟；自举不是目标，登记为远期愿景非承诺）。

---

## 1. 总体管线

r1 的核心方法论照搬："先把用户语法变成更稳定、更可分析的内部表示，再把复杂静态语义下沉到 CFG 化的中层 IR 上做"。

```
源码 (.rx)
  │  lexer（保留 span/source map——一切诊断与工具的地基，先于一切落地）
  ▼
TokenStream
  │  手写递归下降 parser（错误恢复优先；事件流式，预留无损语法树通道）
  ▼
AST ──── 名称解析 / feature gate 检查 / early lints
  │  lowering（item/body 分离；desugar：for/while-let/? 等）
  ▼
HIR ──── 类型收集 → 类型推断 → trait 求解（单态化友好的简化版）→ 类型检查
  │      着色检查（host/device/kernel 边界）、地址空间检查在此层完成
  ▼
TBIR（typed-body IR，临时、仅 body）
  │      模式匹配穷尽性、autoderef/方法糖显式化、drop scope、unsafety 检查
  ▼
MIR（CFG 化、显式类型、泛型态）
  │      借用检查（NLL 数据流）+ views 不相交证明 + barrier 一致性检查
  │      move/init 数据流、const eval（MIR 解释器）、最小优化、单态化项收集
  ▼
┌─────────────┴──────────────┐
│ host 路径                   │ device 路径
│ MIR → LLVM IR → x86-64     │ MIR → LLVM IR（NVPTX 约束子集）
│ → COFF .obj → link.exe     │ → PTX 文本 → 嵌入 host 产物 data 段
│ → PE EXE/DLL/PYD + PDB     │ （运行时 cuModuleLoadDataEx 装载）
└────────────────────────────┘
```

### 1.1 IR 分层职责（D-202）

| 层 | 职责 | 不做什么 |
|---|---|---|
| AST | 贴近用户语法；parser 错误恢复；feature gate | 类型/数据流分析 |
| HIR | 主工作 IR：类型系统全部工作 + 着色/地址空间检查；item/body 分离给增量提供依赖边界 | 借用检查（树状结构不适合，r1 已证伪路径） |
| TBIR | "类型填充后、MIR 之前"的窄门：模式/方法糖/drop scope 显式化；**临时存在、构造 MIR 后即释放**（控峰值内存，r1） | 不长期存活、不表示 item |
| MIR | flow-sensitive 静态语义主战场：借用/move-init/const eval/单态化收集；**GPU 语义（执行层级、地址空间、barrier）在 MIR 显式建模** | 平台细节（后端 facade 之后） |

r1 的强警告全部采纳：TBIR 窄门不能省（有模式匹配/方法糖/析构语义时 HIR→MIR 直连会脏）；借用检查必须在 MIR/CFG（rustc 旧 HIR 实现已被删除的证伪路径）。

## 2. 查询化与增量编译（D-203）

照搬 r1 的"接口第一天、存储最后一天"原则：

- **第一天**：全部语义分析 API 写成 query 风格纯函数——`type_of(def_id)`、`mir_built(body_id)`、`mir_borrowck(body_id)`、`ptx_of(mono_item)` 等；provider 只经 query context 访问其他 query。
- **MVP**：进程内 memoization + 模块/函数级失效。**不做**跨会话红绿增量（fingerprint 成本与磁盘 dep-graph 全套是规模团队产物；且"增量有时比全量更慢"的 fingerprint 教训，r1）。
- **Phase 2（工具链里程碑）**：常驻编译器进程服务 LSP（同一 query 层，§9）；GPU 路径的粗粒度缓存（per-kernel PTX 缓存，键 = MIR 指纹 + 目标 + 编译选项——后端缓存收益先于前端持久化，r1 的工程判断）。
- **Phase 3（开源后）**：跨会话持久化按需评估。
- **并行前端：不做**（r1：2023 nightly 至 2026 仍未 stable，deadlock/race 长尾）；但 query API 从第一天遵守"无全局可变状态、纯函数"约束，保留并行可能性。

## 3. 类型检查与 trait 求解

- 类型推断：HIR body 内局部推断（Hindley-Milner 风格 + 子类型仅限生命周期），函数签名强制完整标注（无全程序推断——诊断质量与增量边界优先）。
- trait 求解：单态化导向的简化求解器（无 trait 对象/特化/HKT，[05](05_LANGUAGE_ARCHITECTURE.md) §2.2 已裁剪），避免 rustc trait solver 的长期成本中心（r1 人年估算表）。
- 着色与地址空间检查：作为类型检查的一部分在 HIR 层完成——函数着色（host/device/kernel）是符号属性，地址空间是类型参数，两者都不需要数据流。

## 4. 借用检查（D-204）

- **算法**：NLL 风格、MIR/CFG 层数据流——region 推断变量替换 → move/init 数据流 → MIR type check 收集 region constraints → region inference → 逐点 in-scope borrows → 报错 walk（r1 流程照搬）。
- **明确不做 Polonius**（r1 最强警告：2026 仍未 stable、alpha 仍慢、已知 soundness issue）。
- **GPU 扩展**：views 不相交证明实现为 MIR 借用检查的扩展 pass——view 分解算子（split/group/transpose）的形状代数在类型层已编码，MIR 层只需验证"每个 `&mut` 分片的索引集由分解路径唯一决定"；barrier 一致性做保守的 uniform control flow 检查（[06](06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) §2.2）。Descend 证明此检查不需要整数算术求解器（r5）。
- **先正确性、后诊断对齐**（r1 的 NLL migration 教训）：MVP 借用诊断允许保守粗糙，UI golden 测试锁住质量底线后逐步打磨。

## 5. 诊断架构（D-206）

r1/r9 的最小落地集，全部进 MVP：

1. **基础设施先于 lexer**：`Span`/`SourceMap`/`DiagCtxt` + `Diag` 结构（emit-or-cancel 强制，泄漏即 ICE）。
2. **结构**：error/warning + 多 span label + note + help + suggestion；suggestion 携带 `Applicability`（`MachineApplicable` / `MaybeIncorrect`）。
3. **错误码**：`RX####` 注册表（分配制，文档生成）；`rurixc --explain RX0301`。错误码段位预留：0xxx 词法/语法、1xxx 名称/模块、2xxx 类型、3xxx 着色/地址空间、4xxx 借用/生命周期、5xxx const eval、6xxx codegen/目标、7xxx 链接/工具链。
4. **JSON 输出**：`--error-format=json` 结构化诊断，是 LSP 与 `rx fix` 的唯一数据源。
5. **UI golden 测试**：compiletest 风格（`//~ ERROR RX0301` 注释 + `.stderr` snapshot + 路径/行号规范化）；MVP 覆盖**四条黄金路径**：解析错误、类型/着色错误、借用/views 错误、目标后端错误（r1 建议照搬）。
6. **渲染**：采用 annotate-snippets 类库（2026 年 rustc 已批准方向，r9），不自研渲染器。
7. **多语言**：message-key 骨架第一天定（编译期校验 key 有效性），首发中英双语。

## 6. 编译性能预算（P-07/P-09 的编译器侧落点）

- 内建 `-Z self-profile` 式 query 级计时 + 阶段计数器，从第一个 query 开始。
- 预算项（进 [14](14_ENGINEERING_DISCIPLINE.md) 预算 JSON，阈值待 M2 实测回填）：冷编译 hello-kernel 端到端、增量 check 延迟（目标 < 5s，r11 引用的行业线）、单 kernel PTX 重生成延迟、内存峰值。
- 编译时间代理指标用 instructions:u（最稳代理，r11），墙钟做趋势参考。

## 7. Device codegen：MIR → LLVM → NVPTX → PTX（D-205/D-207）

r2 的 MVP 收缩清单照搬，作为 codegen 的硬边界：

| 维度 | MVP 决策 | 来源 |
|---|---|---|
| LLVM 版本 | **pin 22.1.x**，季度评估升级；vendored 构建，承认"深度绑定 LLVM 版本与 ptxas 行为"的持续成本 | r2（Triton 同款现实） |
| kernel 标记 | `ptx_kernel` 调用约定；launch bounds → `nvvm.maxntid/reqntid` 属性 | r2 |
| 地址空间 | LLVM addrspace 0/1/3/4/5 显式建模，与 [05](05_LANGUAGE_ARCHITECTURE.md) §5 类型一一对应 | r2 |
| 索引 | `llvm.nvvm.read.ptx.sreg.*` intrinsics | r2 |
| 目标基线 | `compute_89`（PTX 7.8 起即可表达，工具链按 pin 的 LLVM 实际产出版本锁定）；产物 **PTX-only**（开发期）；生产分发"按架构预编 cubin + 保守 PTX fallback"是 G1 任务 | r2 |
| libdevice | **MVP 第一阶段不链接**（先 SAXPY 级），M5 起按需引入：保留外部符号 → 链接 bc → internalize → DCE → NVVMReflect（早期）→ 常规优化；FASTMATH 经编译器开关 + 库显式变体双通道 | r2/r12 |
| 调试信息 | 默认 **line-tables**（`-lineinfo` 等价，Nsight 源码关联即可用）；full debug 独立模式且不混入性能构建；防御性处理非 ASCII 路径（ptxas 崩溃先例） | r2/r11 |
| 验证关卡 | 生成的 PTX 过 `ptxas -arch=sm_89` 干验证（strict-only：ptxas 拒绝 = 编译错误带 RX6xxx 码）；IR golden 测试锁 codegen 形状 | P-01/r2 |
| 已知雷区 | NVPTX shfl 选择失败、sqrt 近似约束失效等 bug 类别 → 建立"NVPTX 雷区回归集"，遇雷登记并 pin 绕行 | r2 |

PTX 版本与驱动 JIT 的匹配（Numba 的 `CUDA_ERROR_UNSUPPORTED_PTX_VERSION` 事故类别）是**运行时启动协商**的职责：装载前比对 PTX `.version` 与驱动能力，不匹配给结构化诊断 + 指引（[08](08_RUNTIME_AND_TOOLING.md) §2.4）。

### 7.1 MLIR：明确不入 MVP（D-208）

r3 结论照搬：MVP 走 MIR→LLVM 直通。MLIR 以"kernel island"形态后置，触发条件三选一（写入 spike gating）：

1. 自研 MIR 上的 tile/fuse/promotion/vector lowering 重复造轮子且维护爆炸；
2. 需要 Tensor Core/WGMMA/TMA 级中层抽象且不愿全 inline intrinsic；
3. 多后端（DXIL 之外）正式立项。

引入形态限定为局部翻译层（自研 MIR → 局部 dialect → nvgpu/nvvm sink），**永不**整体替换 MIR（Mojo/IREE/Triton 的共同实践：官方方言是地基不是整栋楼，r3）。

## 8. Host codegen 与链接（D-209）

- MIR → LLVM → x86-64 COFF `.obj`，Microsoft x64 ABI（r6：Windows x64 唯一主 ABI）。
- 链接：默认 `link.exe`（增量/PDB/VS 生态原生道路）；`lld-link` 作为 opt-in 快路径（与 link.exe 的 parity 未核实——r6 的保守结论）。CI/发行关闭增量链接。
- 调试信息：CodeView + PDB；标准库类型的 Natvis 文件随工具链分发（r6 三层调试体验）。
- 产物：EXE / DLL（+ 自动生成 .lib/.h）/ PYD；PTX 嵌入 data 段（§7）。
- 单态化与 CGU：MVP 单 CGU per package 起步（链接简单优先），编译时间证据驱动再拆分。

## 9. LSP 与工具模式（D-210）

- **单一前端**：LSP 语义全部来自 `rurixc` 的 query 层（r9：4–6 人禁双引擎）。`rurixc --tooling-server` 常驻进程模式。
- parser 预留事件流 → 无损语法树（rowan 式）通道；MVP 的 LSP 容忍"保存时全量 body 重查询"，增量细化随 Phase 2。
- LSP MVP 范围：publishDiagnostics（直接消费 §5 JSON）、completion、definition/references、高亮、rename。中期：hover/signatureHelp/codeAction（消费 MachineApplicable）/inlay hints。远期 GPU 特有：launch 维度提示、地址空间着色、寄存器/occupancy 内联提示（Nsight 数据回灌，r9）。
- 编辑器优先级：VS Code（自维护扩展）→ Visual Studio（LSP + VSIX，VS 2017 15.8+ 原生支持，不做 Roslyn 式原生服务）。

## 10. 构建系统集成

- `rx build/run/test/check` 是唯一入口（manifest 驱动，[09](09_STDLIB_AND_ECOSYSTEM.md) §7）；不生成 MSBuild/CMake 工程。
- 对外被集成：`rx build --emit=cdylib` 产物 + 生成头文件即集成边界；提供 `rurix.props/targets`（MSBuild Build Customization 模式，r6）供 VS 工程消费 DLL 产物，CUDA Toolkit 探测复用运行时探测器（NVML/`CUDA_PATH` 枚举，禁硬编码版本文件名——r6 的 `CUDA 13.2.props` 教训）。

## 11. 测试与验证体系（编译器自身）

- **UI golden**（§5）+ **IR golden**（MIR/LLVM IR/PTX 三层 snapshot，锁 codegen 形状）+ **conformance**（spec 条款锚定，[10](10_GOVERNANCE.md) §4）+ **执行测试**（编译产物在真 GPU 上跑，子进程隔离——GPU 崩溃不连坐 harness，上一项目验证模式 H03 §6）。
- 差分测试：里程碑级 grammar-based fuzz（r7 建议的频率：周级/里程碑级，非日常门禁）。
- 已知失败基线对照纪律照搬（H03 §6）。

## 12. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-11 | 初版 |
