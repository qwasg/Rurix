# Rurix 语言规范 — 标准库语义(core 数学库类型面;M7.1 起)

> 条款:RXS-0104 起续号预留(M7.1 起;**本轮仅登记新文件名 + 预留区间,不落带编号的裸条款头**)。体例见 [README.md](README.md)。
> 依据:01 §6(UC-03 旗舰用例:SPH 仿真 + 软光栅出图);08 §5(stdlib 充实——core 数学库 Vec/Mat/swizzle/几何原语,全 safe API);05 §1(device ⊂ host——同一类型面在 host 与 device 两个执行世界语义一致);11 §3 M7(标准库充实与 G0 图形演示)。授权:[../milestones/m7/M7_CONTRACT.md](../milestones/m7/M7_CONTRACT.md)(`in_scope: core_math_stdlib` / `spec_m7_clauses`,D-M7-1,G-M7-4 / G-M7-5,`rfc_required: none`)+ [../milestones/m7/M7_PLAN.md](../milestones/m7/M7_PLAN.md) §1 M7.1 第 1 项。
> 档位:**Direct**。本文是对 01/08/11 已锁定决策(UC-03 旗舰用例 / stdlib 充实 / G0 软光栅 demo)的初版条款化、纯追加且尚无 stable 面;**AI 无权自判 Direct**,判档以 M7_CONTRACT.md YAML 头 `rfc_required: none` 与上述授权为据,判档争议向上取严。任何偏离已锁定决策、或触及 **const 泛型值运行期单态化(RD-007)** / **软光栅 unsafe 逃生**语义的条款,必须停下标注「需人工升档」,不在本文件自行落笔(10 §3,M7_CONTRACT §6 / out_of_scope)。
> 规范先行(AGENTS.md 硬规则第 7 条):**条款 PR 先于实现 PR**;缺条款的语义 PR 必须先补 spec。`ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 ≥1 测试锚定(`//@ spec: RXS-####`),故本轮**沿 README v1.15 toolchain.md 先例**:只登记文件名 + RXS-0104 预留区间并书写文件级前言/范围/依据/修订记录,**不落带编号的裸条款头**;带编号的条款体连同 ≥1 conformance 锚定样例一并放到下一轮实现 PR,使该门维持全绿。

---

## 1. 范围与编号区间

本文件承载 **core 数学库类型面**的语义条款(M7.1,D-M7-1)。覆盖类型与语义面:

- **Vec**:`VecN<T>`,N ∈ {2, 3, 4}(分量向量)。
- **Mat**:`MatRxC<T>`,方阵(`Mat2`/`Mat3`/`Mat4`)与常用矩形矩阵。
- **swizzle**:分量重排与取子集(如 `.xy` / `.zyx` / `.xxxx`)。
- **几何原语**:点 `Point`、向量 `Vector`、法线 `Normal`、轴对齐包围盒 `AABB`、射线 `Ray` 等。

每类型面的语义维度:**构造**、**分量访问与 swizzle**、**逐元素算术**、**点积 / 叉积 / 范数**、**矩阵乘**、**几何谓词**(相交 / 包含 / 距离)。

全部为**全 safe API**,且 **host 与 device 双路径同义**——同一类型面在两个执行世界语义一致(05 §1 device ⊂ host;08 §5;11 §3 M7)。

**编号区间**:本文件条款自 **RXS-0104** 起续号(全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1)。区间登记于 [README.md](README.md) §4 文件清单(`RXS-0104 起续号预留`)。**本轮无带编号条款落地**;条款号随下一轮实现 PR 的条款体 + 锚定样例正式落定。

## 2. 计划条款骨架(预留 — 非裸条款头)

> 下表是 M7.1 实现 PR 的条款规划草图,**仅作排程参考,不是带编号的裸条款头**(为维持 `trace_matrix --check` 全锚定,本轮不落 `### RXS-####` 标题)。条款号自 RXS-0104 起按落地顺序分配;每条按需分 Syntax / Legality / Dynamic Semantics / Implementation Requirements 节,**严禁 UB 节**(UB 为人类经 Full RFC 落笔的禁区,10 §7.5)。条款体与 ≥1 conformance 锚定样例(`//@ spec: RXS-####`)随实现 PR 同落。

| 计划条款主题 | 语义面要点 | host+device 双路径 |
|---|---|---|
| Vec 类型与构造 | `VecN<T>`(N∈{2,3,4})类型形态、字面构造 / splat / 零值、分量布局 | 同义 |
| Vec 分量访问与 swizzle | 命名分量(`.x/.y/.z/.w`)读写、swizzle 重排 / 取子集的合法分量集与结果元数 | 同义 |
| Vec 逐元素算术 | 加减乘除 / 标量缩放 / 逐元素一元运算的类型与语义 | 同义 |
| Vec 点积 / 叉积 / 范数 | 点积、叉积(N=3)、长度 / 平方长度 / 归一化的求值语义与边界(零向量归一化) | 同义 |
| Mat 类型与构造 | `MatRxC<T>` 方阵 / 矩形、单位阵 / 零阵 / 按行或列构造、元素与行列布局约定 | 同义 |
| Mat 逐元素算术与矩阵乘 | 矩阵加减 / 标量缩放;矩阵乘(维度相容规则)、矩阵–向量乘 | 同义 |
| 几何原语类型面 | 点 `Point` / 向量 `Vector` / 法线 `Normal` 的语义区分与互转、`AABB` / `Ray` 的构造与字段 | 同义 |
| 几何谓词 | 相交(Ray–AABB 等)/ 包含(Point∈AABB)/ 距离的求值语义与边界 | 同义 |

> 上述主题数与切分为 estimated 工程选择;实际条款拆分(合并 / 细分)随实现 PR 按 FLS 体例裁定并在本文件 §「修订记录」留痕。`m7.counter.math_primitives`(G-M7-4)核心原语覆盖计数为实现侧门,非本登记轮范畴。

## 3. 错误码先行引用(占位说明 — 本轮不分配、不预造)

stdlib 数学库段位诊断将随下一轮**条款体**在各条 Legality 节以**占位 `RX####` 名**先行引用,典型诊断类别:

- **维度不匹配**:向量 / 矩阵元数或矩阵乘维度相容性违例。
- **swizzle 非法分量**:超出类型分量集、或写侧重复分量等非法 swizzle。
- (其余按条款体落地时按需补充,如范数 / 几何谓词的退化输入诊断。)

纪律:**本轮绝不改 [../registry/error_codes.json](../registry/error_codes.json) 的任何既有含义字段、也不预造错误码条目、不落带编号的错误码引用汇总表**。正式段位分配与 `message_key` 随后续实现 / 诊断 PR 在 `registry/error_codes.json` **只追加留痕**(段位语义见该文件 `segments`,分配制递增、含义冻结,10 §6;沿 toolchain.md「spec 先行引用 → 实现 WP 正式分配」先例)。错误码引用汇总表随条款体一并落地。

## 4. 升档 / 禁区留痕

- **const 泛型值运行期单态化(RD-007)**:几何原语 / 数组长度类 const 泛型(如固定维度 `Mat`、N∈{2,3,4} 的 `VecN<T>`)可能触发运行期单态化。RD-007 **非 M7 验收门**(M7_CONTRACT out_of_scope / §6,inherited;owner M6→M7 顺延);本文件**不实现 RD-007**,亦不在条款中改变 [consteval.md](consteval.md) RXS-0064 语义。若某条款确需 const 泛型值运行期单态化语义,**停下标注「需人工升档」**,按 14 §4 处置,不在本文件自行落笔。
- **软光栅 unsafe 逃生**:全 safe 代码目标下的 unsafe 落点语义属 G0 软光栅 kernel 作用面(D-M7-3,后续里程碑 spec 段),不在本文件 core 数学库类型面登记;触及即停下标注「需人工升档」。
- **既有禁区**:不碰 device 原子 lowering 与 `atom.{order}.{scope}` PTX 映射(D-406 / RD-008 人工落笔禁区);本文件全 safe、host+device 双路径同义,不引入任何 device-only unsafe 语义。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-15 | 新建 spec/stdlib.md(M7.1 core 数学库类型面起始文件):登记编号区间 RXS-0104 起续号预留 + 文件级前言 / 范围(Vec `VecN<T>` N∈{2,3,4} / Mat `MatRxC<T>` / swizzle / 几何原语 Point·Vector·Normal·AABB·Ray 的构造·分量访问与 swizzle·逐元素算术·点积/叉积/范数·矩阵乘·几何谓词,全 safe、host+device 双路径同义)/ 依据与授权(01 §6 UC-03 + 08 §5 stdlib 充实 + 05 §1 device⊂host + 11 §3 M7;M7_CONTRACT D-M7-1 / G-M7-4 / G-M7-5 `rfc_required: none` + M7_PLAN M7.1)/ 计划条款骨架(预留,非裸条款头)/ 错误码先行引用占位说明 / 升档·禁区留痕。**沿 README v1.15 toolchain.md 先例:本轮不落带编号裸条款头**——条款体与 ≥1 测试锚定随下一轮实现 PR 同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定),无体例变更 | Direct |
