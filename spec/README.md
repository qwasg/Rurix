# spec/ — Rurix 语言规范

> 地位:进入实现期后的**唯一语义事实源**(10 §4,D-403);规范领导实现——先写条款再写实现,缺条款的语义 PR 必须先补 spec。
> 本 README 规定条款体例与变更纪律,自身变更同样适用 §3 档位标记。

---

## 1. 条款编号

- 编号 `RXS-####`(四位,分配制递增),**永不复用**(10 §9.5);弃用条款标注 deprecated 并保留原文,不删除。
- 编号在全 spec 范围内唯一,不按文件分段——文件只是组织载体,条款号是稳定锚点。
- 已分配区间登记于 §4 文件清单。

## 2. 条款格式(FLS 风格,10 §4)

每条款一个三级标题:`### RXS-#### 标题`,按需分节:

| 节 | 内容 |
|---|---|
| Syntax | 文法产生式(EBNF 式记法)或词法形式定义 |
| Legality | 静态合法性规则;违例引用错误码 `RX####`([../registry/error_codes.json](../registry/error_codes.json)) |
| Dynamic Semantics | 运行期语义(词法/纯语法条款不适用) |
| UB | 未定义行为边界(**仅人类经 Full RFC 落笔**,10 §7.5 禁区) |
| Implementation Requirements | 对实现的强制要求(错误恢复/诊断质量/平台行为) |

**测试锚定**(10 §4):每条款 ≥1 测试,测试侧以 `//@ spec: RXS-####` 注释(conformance 样例)或单测注释引用条款号;traceability 矩阵由工具生成(M1.4 起)。

## 3. 变更纪律与档位标记

- spec 文件变更必须走对应档位(10 §3):初版条款化已选定决策(D-1xx 等)为 **Direct**;规范内 bug fix 为 **Mini-RFC**;新语法/语义变更为 **Full RFC**(RFC 合入后才可改 spec)。判档争议向上取严;AI 无权自行判档为 Direct。
- **机器核对**(M1 CI_GATES §4 第 7 项):每个 spec 文件末尾维护修订记录表,含"档位"列;任何对 spec 文件的变更必须**新增修订行**(既有行 0-byte),否则 guardrail FAIL。
- 错误码语义可加不可改(10 §6);条款引用的错误码一经发布含义冻结。

## 4. 文件清单与编号区间

| 文件 | 内容 | 已分配条款 | 起始里程碑 |
|---|---|---|---|
| [lexical.md](lexical.md) | 词法结构 | RXS-0001 ~ RXS-0010 | M1.2 |
| [syntax.md](syntax.md) | 语法结构 | RXS-0011 ~ RXS-0031 | M1.3 |
| [names.md](names.md) | 名称与模块语义 | RXS-0032 ~ RXS-0038 | M2.1 |
| [types.md](types.md) | 类型与检查语义 | RXS-0039 ~ RXS-0047 | M2.2 |
| [borrow.md](borrow.md) | 所有权与借用语义(desugar/穷尽性/drop scope 首批;move/init/Drop 执行语义;借用/生命周期主体) | RXS-0048 ~ RXS-0061 | M3.1 |
| [consteval.md](consteval.md) | const 求值语义(const fn 子集 / const item 求值 / const 泛型 / 求值失败) | RXS-0062 ~ RXS-0065 | M3.4 |
| [device.md](device.md) | device 语义(函数着色与跨着色调用 / 地址空间类型与一致性 / barrier uniform 可达性保守骨架 / 着色与地址空间诊断要求 / NVPTX codegen 目标与调用约定 / 地址空间 codegen 建模 / 线程索引与 launch bounds / ptxas 干验证关卡 / launch 类型契约与诊断要求 / PTX 装载协商 / poisoned context 状态机 / views 算子集语义与子 view 不相交证明 / shared+barrier 一致性数据流 / scoped atomics 类型契约与 PTX 映射 / device 数学函数 intrinsic 集与求值语义 / libdevice bitcode 链接流程与 codegen 诊断) | RXS-0066 ~ RXS-0082 | M4.1 |
| toolchain.md（M6.1 续号预留，条款体随实现 PR 落地） | 工具链语义(rx CLI 子命令语义面与退出码约定 / 包管理 rurix.toml·rurix.lock 格式与依赖三来源解析 / LSP 能力面契约) | RXS-0083 ~ （续号预留，待分配） | M6.1 |

## 5. 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-11 | 占位 README 升级为正式体例(编号规则/条款格式/档位标记约定);依据 10 §3 §4 既定治理决策,无新语义 | Direct |
| v1.1 | 2026-06-11 | §4 文件清单追加 syntax.md(RXS-0011 ~ RXS-0031,M1.3;D-M1-3 条款化登记,无体例变更) | Direct |
| v1.2 | 2026-06-11 | §4 文件清单追加 names.md(RXS-0032 ~ RXS-0038,M2.1 names 条款化登记,无体例变更) | Direct |
| v1.3 | 2026-06-12 | §4 文件清单追加 types.md(RXS-0039 ~ RXS-0047,M2.2 types 条款化登记,无体例变更) | Direct |
| v1.4 | 2026-06-12 | §4 文件清单追加 borrow.md(RXS-0048 ~ RXS-0052,M3.1 desugar/穷尽性/drop scope 首批条款化登记,无体例变更) | Direct |
| v1.5 | 2026-06-13 | §4 borrow.md 行区间更新至 RXS-0056(M3.2 move/init/Drop 执行语义条款追加登记,无体例变更) | Direct |
| v1.6 | 2026-06-13 | §4 borrow.md 行区间更新至 RXS-0061(M3.3 借用/生命周期主体条款追加登记,无体例变更) | Direct |
| v1.7 | 2026-06-13 | §4 文件清单追加 consteval.md(RXS-0062 ~ RXS-0065,M3.4 const eval 首批条款化登记,无体例变更) | Direct |
| v1.8 | 2026-06-13 | §4 文件清单追加 device.md(RXS-0066 ~ RXS-0069,M4.1 device 着色/地址空间首批条款化登记;codegen/launch 条款随 M4.2/M4.3 续写本文件,无体例变更) | Direct |
| v1.9 | 2026-06-13 | §4 device.md 行区间更新至 RXS-0073(M4.2 NVPTX codegen 目标与调用约定 / 地址空间 codegen 建模 / 线程索引与 launch bounds / ptxas 干验证关卡条款追加登记;launch 类型契约条款随 M4.3 续写,无体例变更) | Direct |
| v1.10 | 2026-06-13 | §4 device.md 行区间更新至 RXS-0075(M4.3 launch 类型契约与诊断要求条款追加登记;运行时对象/装载协商/poisoned 状态机随 rurix-rt 实现 PR,无体例变更) | Direct |
| v1.11 | 2026-06-13 | §4 device.md 行区间更新至 RXS-0077(M4.3 运行时 PTX 装载协商 / poisoned context 状态机条款追加登记;rurix-rt 运行时落地,无体例变更) | Direct |
| v1.12 | 2026-06-14 | §4 device.md 行区间更新至 RXS-0078(M5.1 views 算子集语义与子 view 不相交证明条款追加登记;MIR 借用检查 device 扩展,条款 PR 先于实现 PR,无体例变更) | Direct |
| v1.13 | 2026-06-14 | §4 device.md 行区间更新至 RXS-0080(M5.2 shared+barrier 一致性数据流 / scoped atomics 类型契约与 PTX 映射条款追加登记;MIR 借用检查 device 扩展数据流 + scoped atomics 映射 D-406 人工落笔,条款 PR 先于实现 PR,无体例变更) | Direct |
| v1.14 | 2026-06-14 | §4 device.md 行区间更新至 RXS-0082(M5.3 device 数学函数 intrinsic 集与求值语义 / libdevice bitcode 链接流程与 codegen 诊断条款追加登记;libdevice 按需引入 06 §7 + 编译流程 07 §7 D-205/D-207 条款化,条款 PR 先于实现 PR,gpu 基元 kernel codegen 接通随实现 WP,无体例变更) | Direct |
| v1.15 | 2026-06-15 | §4 文件清单追加 toolchain.md（RXS-0083 起续号预留，起始里程碑 M6.1：rx CLI 子命令语义面 / 包管理 rurix.toml·rurix.lock 格式与三来源解析 / LSP 能力面契约）。M6 开工脚手架仅登记新文件名 + 预留区间，**不落裸条款头**——条款体与测试锚定随 M6.1+ 实现 PR 同落（条款 PR 先于实现 PR，trace_matrix 维持全锚定），无体例变更 | Direct |
