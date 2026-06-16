# Rurix 语言规范 — 标准库语义(core 数学库类型面;M7.1 起)

> 条款:RXS-0104 ~ RXS-0113(M7.1 core 数学库 Vec/Mat/swizzle 类型面 RXS-0104~0109 + 几何原语 Point·Vector·Normal·AABB·Ray 与几何谓词 RXS-0110~0113)。体例见 [README.md](README.md)。
> 依据:01 §6(UC-03 旗舰用例:SPH 仿真 + 软光栅出图);08 §5(stdlib 充实——core 数学库 Vec/Mat/swizzle/几何原语,全 safe API);05 §1(device ⊂ host——同一类型面在 host 与 device 两个执行世界语义一致);11 §3 M7(标准库充实与 G0 图形演示)。授权:[../milestones/m7/M7_CONTRACT.md](../milestones/m7/M7_CONTRACT.md)(`in_scope: core_math_stdlib` / `spec_m7_clauses`,D-M7-1,G-M7-4 / G-M7-5,`rfc_required: none`)+ [../milestones/m7/M7_PLAN.md](../milestones/m7/M7_PLAN.md) §1 M7.1 第 1 项。
> 档位:**Direct**。本文是对 01/08/11 已锁定决策(UC-03 旗舰用例 / stdlib 充实 / G0 软光栅 demo)的初版条款化、纯追加且尚无 stable 面;**AI 无权自判 Direct**,判档以 M7_CONTRACT.md YAML 头 `rfc_required: none` 与上述授权为据,判档争议向上取严。任何偏离已锁定决策、或触及 **const 泛型值运行期单态化(RD-007)** / **软光栅 unsafe 逃生**语义的条款,必须停下标注「需人工升档」,不在本文件自行落笔(10 §3,M7_CONTRACT §6 / out_of_scope)。
> 规范先行(AGENTS.md 硬规则第 7 条):**条款 PR 先于实现 PR**;缺条款的语义 PR 必须先补 spec。`ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 ≥1 测试锚定(`//@ spec: RXS-####`),本轮带编号条款体(RXS-0104 ~ RXS-0109)连同每条 ≥1 conformance 锚定样例(`conformance/stdlib/**`)一并落地,该门维持全绿。
> 双路径实现说明(M7.1 NVPTX codegen 标量子集):host 路径以具体 `f32` 结构体类型面(`Vec2`/`Vec3`/`Vec4`、`Mat2`/`Mat3`/`Mat4`)+ inherent `device fn` 方法实现;device 路径在当前 NVPTX codegen **标量值类型子集**(聚合/结构体值类型 codegen 为后续扩展,现作用面外报 `RX6003`,见 §5)下,以**数值语义同义**的标量分量 `device fn` 原语实现并经 device codegen + `ptxas` 干验证。两路径数值语义同义(05 §1 device ⊂ host);元素类型 M7.1 规范性收窄为 `f32`(族记 `VecN<T>`/`MatRxC<T>` 的 `T`,其余元素类型加性后续)。

---

## 1. 范围与编号区间

本文件承载 **core 数学库类型面**的语义条款(M7.1,D-M7-1)。覆盖类型与语义面:

- **Vec**:`VecN<T>`,N ∈ {2, 3, 4}(分量向量);M7.1 落地具体类型 `Vec2` / `Vec3` / `Vec4`,元素类型 `T = f32`。
- **Mat**:`MatRxC<T>`,方阵 `Mat2` / `Mat3` / `Mat4`;M7.1 元素类型 `T = f32`。
- **swizzle**:分量重排与取子集(如 `.xy()` / `.zyx()` / `.xxxx()`,方法形)。
- **几何原语**:点 `Point3`、向量 `Vector3`、法线 `Normal3`(语义区分与互转)、轴对齐包围盒 `Aabb`、射线 `Ray`(构造与字段);M7.1 落地具体类型,元素类型 `f32`。

每类型面的语义维度:**构造**、**分量访问与 swizzle**、**逐元素算术**、**点积 / 叉积 / 范数**、**矩阵乘**、**几何谓词**(Point∈AABB 包含 / 点到 AABB 距离 / Ray–AABB 相交,RXS-0112~0113)。

全部为**全 safe API**,且 **host 与 device 双路径数值语义同义**——同一原语在两个执行世界数值语义一致(05 §1 device ⊂ host;08 §5;11 §3 M7)。维度以具体类型名编码(`Vec2`/`Vec3`/`Vec4`、`Mat2`/`Mat3`/`Mat4`),**不使用 const 泛型维度**(RD-007 不触碰,见 §5);元素类型 M7.1 规范性收窄为 `f32`(`VecN<T>`/`MatRxC<T>` 的 `T`),其余元素类型为加性后续(不改既有条款语义)。

**编号区间**:本文件条款自 **RXS-0104** 起续号(全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1)。本轮落地 **RXS-0104 ~ RXS-0113**(Vec/Mat/swizzle 类型面 RXS-0104~0109 + 几何原语 / 谓词 RXS-0110~0113)。区间登记于 [README.md](README.md) §4 文件清单。

## 2. 条款

> 每条按需分 Syntax / Legality / Dynamic Semantics / Implementation Requirements 节,**严禁 UB 节**(UB 为人类经 Full RFC 落笔的禁区,10 §7.5)。Legality 违例只**引用**错误码(§4 引用汇总),不在此定义其含义。

### RXS-0104 Vec 类型与构造

**Syntax**(类型与构造形态;族记 `VecN<T>`,M7.1 `T = f32`):

```
VecType ::= "Vec2" | "Vec3" | "Vec4"
VecCtor ::= VecType "::" "new" "(" Expr {"," Expr} ")"   // 实参元数 = N,元素类型 f32
          | VecType "::" "splat" "(" Expr ")"            // 单标量铺满 N 个分量
          | VecType "::" "zero" "(" ")"                  // 全零向量
VecLit  ::= VecType "{" Ident ":" Expr {"," Ident ":" Expr} "}"   // 结构体字面构造
```

**Legality**:

- `VecN` 为具体维度类型(N∈{2,3,4}),分量字段依序为 `x`,`y`(,`z`(,`w`)),类型 `f32`;派生 `Copy`/`Clone`(按值传递不发生 move)。
- 字面构造须恰好给齐 N 个声明分量;缺字段 / 未知字段 → `RX2002`;分量表达式类型非 `f32` → `RX2001`。
- `new` 的实参元数须等于 N;不符 → `RX2003`;实参类型非 `f32` → `RX2001`。
- 构造为全 safe,不含 `unsafe`。

**Dynamic Semantics**:

- `VecN::new(c0, …, c_{N-1})` 产分量依序为 `c0 … c_{N-1}`;`VecN::splat(s)` 产每个分量为 `s`;`VecN::zero()` 产每个分量为 IEEE-754 `+0.0`。
- 分量按字段声明序 `x, y, z, w` 紧致存放;两路径(host 结构体 / device 标量分量)构造产同一数值分量集。

**Implementation Requirements**:

- host 路径以结构体 + inherent `device fn` 构造方法实现;device 路径以语义同义的标量分量构造实现(NVPTX codegen 标量子集,§5)。构造方法须为 `device fn` 以满足 device ⊂ host 可达(05 §1)。

> 锚定测试:`conformance/stdlib/host/vec_ops.rx`(构造 / splat / zero 真跑);`conformance/stdlib/device/vec_scalar.rx`(device 标量构造 codegen)。

### RXS-0105 Vec 分量访问与 swizzle

**Syntax**:

```
CompAccess ::= Expr "." ("x" | "y" | "z" | "w")          // 命名分量读
Swizzle    ::= Expr "." SwizzleSel "(" ")"               // swizzle 取子集 / 重排(方法形)
SwizzleSel ::= CompChar {CompChar}                        // 2~4 个分量字符
CompChar   ::= "x" | "y" | "z" | "w"
```

**Legality**:

- 命名分量 `.x/.y/.z/.w` 仅对拥有该分量的 `VecN` 合法(`Vec2` 仅 `x,y`;`Vec3` 仅 `x,y,z`;`Vec4` 全部);访问不存在的分量字段 → `RX2002`。
- swizzle 选择子的每个分量字符须在源类型分量集内,结果元数 = 选择子长度(2→`Vec2`,3→`Vec3`,4→`Vec4`);分量可重复(如 `.xxxx()`)。
- 选择子含源类型不存在的分量、或结果元数不在 {2,3,4} 的 swizzle 为**非法 swizzle**:该 swizzle 方法在源类型上不存在 → `RX2004`。
- swizzle 为只读取值(读侧),全 safe,不产生别名或可变借用。

**Dynamic Semantics**:

- `.c` 取对应分量值;swizzle `.s0 s1 …()` 按选择子依序取源分量构造结果向量,分量值为源对应分量的副本(值语义,源不变)。两路径对同一选择子产同一结果分量序。

**Implementation Requirements**:

- host 路径以 inherent `device fn` swizzle 方法实现(每个支持的选择子一个方法);device 路径以标量分量重排同义实现。

> 锚定测试:`conformance/stdlib/host/vec_ops.rx`(命名访问 + swizzle);`conformance/stdlib/reject/illegal_swizzle/basic.rx`(非法 swizzle → `RX2004`)。

### RXS-0106 Vec 逐元素算术

**Syntax**(方法形,全 safe):

```
VecArith ::= Expr "." ("add" | "sub" | "mul" | "div") "(" Expr ")"   // 同型逐元素
           | Expr "." "scale" "(" Expr ")"                            // 标量缩放(f32)
           | Expr "." "neg" "(" ")"                                   // 逐元素取负
```

**Legality**:

- `add`/`sub`/`mul`/`div` 的接收者与实参须为**同一 `VecN` 类型**(同元数同元素类型);元数 / 类型不相容 → `RX2001`;实参数目不符 → `RX2003`。
- `scale` 实参为 `f32` 标量;`neg` 无实参。
- 逐元素算术全 safe,不含 `unsafe`;结果类型与接收者同型。

**Dynamic Semantics**:

- `a.add(b)` 产分量 `a_i + b_i`;`sub`→`a_i - b_i`;`mul`→`a_i * b_i`;`div`→`a_i / b_i`(逐元素 IEEE-754 f32 运算,inf/NaN 按 IEEE-754 传播);`scale(s)`→`a_i * s`;`neg()`→`-a_i`。两路径数值同义。

**Implementation Requirements**:

- host 路径以 inherent `device fn` 方法实现;device 路径以标量分量逐元素同义实现。除法保持 IEEE-754 语义,不引入额外检查(本条无 UB 节)。

> 锚定测试:`conformance/stdlib/host/vec_ops.rx`(加减乘除 / scale / neg 真跑);`conformance/stdlib/reject/dim_mismatch/basic.rx`(异元数 → `RX2001`)。

### RXS-0107 Vec 点积 / 叉积 / 范数

**Syntax**:

```
VecDot   ::= Expr "." "dot" "(" Expr ")"                  // 点积 → f32
VecCross ::= Expr "." "cross" "(" Expr ")"                // 叉积,仅 Vec3 → Vec3
VecNorm  ::= Expr "." ("length" | "length_sq") "(" ")"    // 范数 / 平方范数 → f32
           | Expr "." "normalize" "(" ")"                 // 归一化 → 同型 VecN
```

**Legality**:

- `dot` 接收者与实参须同一 `VecN` 类型;不相容 → `RX2001`。
- `cross` 仅对 `Vec3` 定义;对 `Vec2`/`Vec4` 调用 → 该方法不存在 `RX2004`;实参须为 `Vec3`,否则 `RX2001`。
- `length`/`length_sq` 返回 `f32`;`normalize` 返回与接收者同型 `VecN`。均为全 safe。

**Dynamic Semantics**:

- `a.dot(b)` = `Σ_i a_i * b_i`(IEEE-754 f32);`length_sq()` = `self.dot(self)`;`length()` = `sqrt(length_sq())`,`sqrt` 为收敛的全 safe 软件平方根(见 Implementation Requirements)。
- `Vec3` `a.cross(b)` 产 `(a_y·b_z − a_z·b_y, a_z·b_x − a_x·b_z, a_x·b_y − a_y·b_x)`。
- `normalize()` 的**边界**:当 `length() == 0.0`(零向量,或下溢至 0)时,`normalize()` 返回**零向量**(各分量 `+0.0`),不产生除零、不进入未定义行为;否则返回 `self.scale(1.0 / length())`。该边界在两路径一致(以确定性返回值定义边界,**不设 UB 节**)。

**Implementation Requirements**:

- host 与 device 路径的 `sqrt` 均以同一全 safe 软件实现(如 Newton–Raphson 迭代,纯 `f32` 算术,**不依赖** libdevice `__nv_sqrtf` 等 device-only intrinsic),保证 `length`/`normalize` 两路径数值同义且在 host 可真跑。
- 全 safe,无 `unsafe`;零向量归一化按上述确定性边界处理。

> 锚定测试:`conformance/stdlib/host/vec_ops.rx`(dot / cross / length / normalize,含零向量边界);`conformance/stdlib/device/vec_scalar.rx`(device 标量 dot / cross / length codegen)。

### RXS-0108 Mat 类型与构造

**Syntax**:

```
MatType ::= "Mat2" | "Mat3" | "Mat4"
MatCtor ::= MatType "::" "identity" "(" ")"
          | MatType "::" "zero" "(" ")"
          | MatType "::" "from_rows" "(" Expr {"," Expr} ")"   // N 个 VecN 行
```

**Legality**:

- `MatN` 为 N×N 方阵(N∈{2,3,4}),元素 `f32`,派生 `Copy`/`Clone`。
- `from_rows` 实参为 N 个 `VecN`(与矩阵阶相同的行向量);实参数目 ≠ N → `RX2003`;行向量类型不符 → `RX2001`。
- 元素以**行主序**(row-major)逻辑布局:第 i 行第 j 列记 `m[i][j]`;`from_rows(r0, …, r_{N-1})` 以 `r_k` 为第 k 行。
- 全 safe。

**Dynamic Semantics**:

- `identity()` 产单位阵(对角 `1.0`,其余 `+0.0`);`zero()` 产全零阵;`from_rows(r0,…)` 产以给定向量为各行的矩阵。两路径数值同义。

**Implementation Requirements**:

- host 路径以结构体 + inherent `device fn` 构造方法实现;device 路径以标量元素构造同义实现(标量子集,§5)。行列布局约定(row-major)在两路径一致。

> 锚定测试:`conformance/stdlib/host/mat_ops.rx`(identity / zero / from_rows);`conformance/stdlib/device/mat_scalar.rx`(device 标量构造 codegen)。

### RXS-0109 Mat 逐元素算术与矩阵乘

**Syntax**:

```
MatArith ::= Expr "." ("add" | "sub") "(" Expr ")"     // 同阶逐元素
           | Expr "." "scale" "(" Expr ")"             // 标量缩放(f32)
MatMul   ::= Expr "." "mul" "(" Expr ")"               // 矩阵乘 MatN × MatN → MatN
           | Expr "." "mul_vec" "(" Expr ")"           // 矩阵-向量 MatN × VecN → VecN
```

**Legality**:

- `add`/`sub` 接收者与实参须为**同阶 `MatN`**;阶不相容 → `RX2001`;实参数目不符 → `RX2003`。
- `mul` 接收者与实参须同阶 `MatN`,结果 `MatN`;`mul_vec` 实参须为与矩阵阶相同的 `VecN`,结果 `VecN`;维度不相容 → `RX2001`(或对阶不匹配的类型调用不存在的方法 → `RX2004`)。
- 全 safe。

**Dynamic Semantics**:

- `add`/`sub`/`scale` 为逐元素 IEEE-754 f32 运算。
- `A.mul(B)` 产 `C`,`C[i][j] = Σ_k A[i][k] * B[k][j]`(标准矩阵乘,row-major,RXS-0108 布局)。
- `A.mul_vec(v)` 产 `w`,`w[i] = Σ_k A[i][k] * v[k]`(矩阵-列向量乘)。两路径数值同义。

**Implementation Requirements**:

- host 路径以 inherent `device fn` 方法实现;device 路径以标量元素累加同义实现。矩阵乘累加序在两路径固定一致以保数值可复现(确定性归约,14 §3 风险对策)。

> 锚定测试:`conformance/stdlib/host/mat_ops.rx`(add/sub/scale/mul/mul_vec 真跑);`conformance/stdlib/device/mat_scalar.rx`(device 标量矩阵乘 / 矩阵-向量乘 codegen)。

### RXS-0110 几何向量类语义区分与互转(Point / Vector / Normal)

**Syntax**(三个互异几何类型与互转,方法形,全 safe;M7.1 `T = f32`):

```
GeomVecType ::= "Point3" | "Vector3" | "Normal3"
GeomCtor    ::= GeomVecType "::" "new" "(" Expr "," Expr "," Expr ")"   // 三 f32 分量 x,y,z
GeomConv    ::= Expr "." "sub" "(" Expr ")"        // Point3 − Point3 → Vector3(两点位移)
              | Expr "." "offset" "(" Expr ")"     // Point3 + Vector3 → Point3(平移)
              | Expr "." "as_vector" "(" ")"       // Point3 → Vector3(位置向量)
              | Expr "." "to_normal" "(" ")"       // Vector3 → Normal3(归一化)
              | Expr "." "to_vector" "(" ")"       // Normal3 → Vector3
```

**Legality**:

- `Point3` / `Vector3` / `Normal3` 为**三个互异**的具体几何类型(分量字段依序 `x`,`y`,`z`,类型 `f32`;派生 `Copy`/`Clone`)。**语义区分**:三者类型不可互换——在期望某一类型处误用另一类型 → `RX2001`。
- 字面构造须恰好给齐 `x`,`y`,`z` 三分量;缺字段 / 未知字段 → `RX2002`;分量表达式类型非 `f32` → `RX2001`;`new` 实参元数 ≠ 3 → `RX2003`。
- 互转方法接收者 / 实参类型须相符:`Point3.sub` 实参须 `Point3`、`Point3.offset` 实参须 `Vector3`,不符 → `RX2001`;实参数目不符 → `RX2003`。
- 构造与互转全 safe,不含 `unsafe`。

**Dynamic Semantics**:

- `Point3::new(x,y,z)`(`Vector3`/`Normal3` 同)产分量依序为 `x,y,z`。
- `p.sub(q)` 产 `Vector3(p.x−q.x, p.y−q.y, p.z−q.z)`;`p.offset(v)` 产 `Point3(p.x+v.x, p.y+v.y, p.z+v.z)`;`p.as_vector()` 产 `Vector3(p.x, p.y, p.z)`;`n.to_vector()` 产 `Vector3(n.x, n.y, n.z)`。
- `v.to_normal()` 的**边界**:设 `L = sqrt(v.x² + v.y² + v.z²)`(`sqrt` 为 RXS-0107 同一全 safe 软件平方根)。当 `L == 0.0` 时产**零** `Normal3`(各分量 `+0.0`),不产生除零、不进入未定义行为;否则产 `Normal3(v.x/L, v.y/L, v.z/L)`。该边界以确定性返回值定义(**不设 UB 节**),两路径一致。

**Implementation Requirements**:

- host 路径以具体 f32 结构体 + inherent `device fn` 方法实现;device 路径以语义同义的标量分量 `device fn` 原语实现(归一化分量等,NVPTX codegen 标量子集,§5),复用 RXS-0107 的 `rx_sqrt`(Newton,纯 f32)。全 safe;零向量归一化按上述确定性边界处理。

> 锚定测试:`conformance/stdlib/host/geom_ops.rx`(Point/Vector/Normal 构造与互转真跑);`conformance/stdlib/device/geom_scalar.rx`(device 标量归一化分量 codegen);`conformance/stdlib/reject/geom_type_confusion/basic.rx`(类型互斥误用 → `RX2001`)。

### RXS-0111 AABB / Ray 类型与构造

**Syntax**:

```
AabbCtor ::= "Aabb" "::" "new" "(" Expr "," Expr ")"   // (min: Point3, max: Point3)
RayCtor  ::= "Ray"  "::" "new" "(" Expr "," Expr ")"   // (origin: Point3, dir: Vector3)
```

**Legality**:

- `Aabb` 表示轴对齐包围盒,逻辑字段为下角 `min` / 上角 `max`(各 `f32` 三分量);`Ray` 表示射线,逻辑字段为起点 `origin`(`Point3`)+ 方向 `dir`(`Vector3`)。均派生 `Copy`/`Clone`。
- `Aabb::new` 实参须为 `(Point3, Point3)`;`Ray::new` 实参须为 `(Point3, Vector3)`;实参类型不符 → `RX2001`;实参数目 ≠ 2 → `RX2003`。
- 全 safe。

**Dynamic Semantics**:

- `Aabb::new(min, max)` 产以 `min` 为下角、`max` 为上角的盒(`min_axis ≤ max_axis` 为良构前提,本条不强制校验排序,排序异常交由几何谓词按 IEEE-754 求值);`Ray::new(o, d)` 产以 `o` 为起点、`d` 为方向的射线。字段读取产对应分量。两路径数值同义。

**Implementation Requirements**:

- host 路径以具体 f32 结构体(分量扁平存放,经构造投影自 `Point3`/`Vector3` 实参)+ inherent `device fn` 构造实现;device 路径以标量分量构造同义实现(标量子集,§5)。字段布局约定在两路径一致。

> 锚定测试:`conformance/stdlib/host/geom_ops.rx`(Aabb / Ray 构造与字段);`conformance/stdlib/device/geom_scalar.rx`(device 标量分量构造 codegen)。

### RXS-0112 几何谓词:Point∈AABB 包含与点到 AABB 距离

**Syntax**:

```
AabbContains ::= Expr "." "contains" "(" Expr ")"   // Aabb.contains(Point3) → bool
AabbDistance ::= Expr "." "distance" "(" Expr ")"   // Aabb.distance(Point3) → f32
```

**Legality**:

- `contains` 接收者为 `Aabb`、实参为 `Point3`,返回 `bool`;`distance` 接收者为 `Aabb`、实参为 `Point3`,返回 `f32`。实参类型非 `Point3` → `RX2001`;实参数目不符 → `RX2003`。
- 全 safe。

**Dynamic Semantics**:

- `a.contains(p)` 为真当且仅当对每轴 `a.min_axis ≤ p.axis ≤ a.max_axis`(逐轴 IEEE-754 f32 比较取合取);否则为假。
- `a.distance(p)`:逐轴偏移 `δ_axis = max(a.min_axis − p.axis, 0.0) + max(p.axis − a.max_axis, 0.0)`(点在该轴区间内则 `δ_axis = 0`);`distance = sqrt(δ_x² + δ_y² + δ_z²)`(RXS-0107 同一全 safe 软件 `sqrt`)。点在盒内 → `distance == +0.0`。两路径数值同义。

**Implementation Requirements**:

- host 路径以 `Aabb` inherent `device fn` 方法实现;device 路径以语义同义标量分量原语实现——布尔谓词以 `f32` `1.0`/`0.0` 表达(与 `bool` `true`/`false` 数值同义),距离以 `f32` 标量,复用 RXS-0107 的 `rx_sqrt`。全 safe;距离边界以确定性返回值定义(**不设 UB 节**)。

> 锚定测试:`conformance/stdlib/host/geom_ops.rx`(contains / distance 真跑,含内部点 distance 0 边界);`conformance/stdlib/device/geom_scalar.rx`(device 标量 point_in_aabb / point_aabb_dist codegen)。

### RXS-0113 几何谓词:Ray–AABB 相交

**Syntax**:

```
AabbIntersectRay ::= Expr "." "intersects" "(" Expr ")"   // Aabb.intersects(Ray) → bool
```

**Legality**:

- `intersects` 接收者为 `Aabb`、实参为 `Ray`,返回 `bool`。实参类型非 `Ray` → `RX2001`;实参数目不符 → `RX2003`。全 safe。

**Dynamic Semantics**:

- 以 **slab 法**求值:对每轴,设 `inv = 1.0 / dir_axis`,`t1 = (min_axis − origin_axis) · inv`,`t2 = (max_axis − origin_axis) · inv`,`tlo_axis = min(t1, t2)`,`thi_axis = max(t1, t2)`;令 `t_enter = max(tlo_x, tlo_y, tlo_z)`,`t_exit = min(thi_x, thi_y, thi_z)`。相交为真当且仅当 `t_exit ≥ t_enter` 且 `t_exit ≥ 0.0`(射线非负参数区间命中盒)。
- **轴平行退化**(`dir_axis == 0.0`):`inv` 为 IEEE-754 `±∞`,`t1`/`t2` 按 IEEE 算术传播为 `±∞`,`min`/`max` 与上述合取在 IEEE 比较下给出**确定性布尔**(原点在该轴 slab 内则该轴不约束、在外则判否)。全过程以确定性返回值定义边界,**不设 UB 节**。
- 两路径数值同义。

**Implementation Requirements**:

- host 路径以 `Aabb` inherent `device fn` 方法实现;device 路径以语义同义标量分量原语实现(布尔以 `f32` `1.0`/`0.0` 表达)。`min`/`max`/比较合取序在两路径**固定一致**以保数值可复现(确定性归约,14 §3)。全 safe。

> 锚定测试:`conformance/stdlib/host/geom_ops.rx`(intersects 真跑,命中 / 不命中两例);`conformance/stdlib/device/geom_scalar.rx`(device 标量 ray_aabb_hit codegen)。

## 3. 几何原语条款落地说明

> 几何原语(`Point3` / `Vector3` / `Normal3` / `Aabb` / `Ray`)+ 几何谓词(Point∈AABB 包含 / 点到 AABB 距离 / Ray–AABB 相交)已于 §2 落地为带编号条款 **RXS-0110 ~ RXS-0113**(每条 ≥1 conformance 锚定,`trace_matrix --check` 维持全锚定)。实现裁决沿用 §2 既定口径:host 路径具体 f32 结构体(`Point3`/`Vector3`/`Normal3`/`Aabb`/`Ray`)+ inherent `device fn` 方法;device 路径以语义同义的标量分量 `device fn` 原语(谓词布尔以 `f32` `1.0`/`0.0` 表达)经 device codegen + `ptxas` 干验证(聚合值类型 device codegen 为后续扩展,§5)。`m7.counter.math_primitives`(G-M7-4)核心原语覆盖计数并入几何谓词原语。维度以具体类型名编码、元素类型 `f32`,不使用 const 泛型(RD-007 不触碰,§5)。

## 4. 错误码引用汇总

> 本表仅**引用**既有错误码(均为 2xxx 类型段位,07 §5),含义以 [../registry/error_codes.json](../registry/error_codes.json) 为唯一事实源。本轮 core 数学库类型面以具体结构体 + inherent 方法实现,误用天然落入既有**类型类诊断**,**不新增错误码、不预造条目**(无 bespoke 诊断实现,M7 CI_GATES §4.2);故不改 `registry/error_codes.json` 与 `en.messages`。

| 错误码 | 含义 | 条款 |
|---|---|---|
| RX2001 | 类型不匹配(向量 / 矩阵元数·阶不相容;分量 / 行向量 / 实参元素类型非 f32;几何 `Point3`/`Vector3`/`Normal3`/`Aabb`/`Ray` 类型互斥误用;谓词 / 构造实参类型不符) | RXS-0104 / RXS-0106 / RXS-0107 / RXS-0108 / RXS-0109 / RXS-0110 / RXS-0111 / RXS-0112 / RXS-0113 |
| RX2002 | 未知或缺失字段(构造缺 / 多分量字段;访问不存在的命名分量;几何原语构造缺 / 多分量字段) | RXS-0104 / RXS-0105 / RXS-0108 / RXS-0110 |
| RX2003 | 实参数目不符(`new` / `from_rows` 实参元数 ≠ N;几何构造 / 谓词实参元数不符) | RXS-0104 / RXS-0106 / RXS-0108 / RXS-0109 / RXS-0110 / RXS-0111 / RXS-0112 / RXS-0113 |
| RX2004 | 无此方法或关联项(非法 swizzle 选择子;对不支持阶 / 类型调用 `cross` / `mul` 等) | RXS-0105 / RXS-0107 / RXS-0109 |

## 5. 升档 / 禁区留痕

- **聚合值类型 device codegen(后续扩展,非禁区)**:当前 NVPTX codegen 为**标量值类型子集**,结构体 / 聚合值类型(`Vec`/`Mat` 结构体作为 device 函数参数 / 返回 / 局部 / 字段投影 / 构造)在作用面外报 `RX6003`(RXS-0070 / RXS-0073)。故本轮 device 路径以**语义同义的标量分量 `device fn` 原语**实现并经 device codegen + `ptxas` 干验证;聚合值类型 device codegen 为后续工程扩展(届时 host 结构体 API 可直接在 device 复用),其接通**不改本文件既有条款语义**(纯实现侧回填)。
- **const 泛型值运行期单态化(RD-007)**:本文件以具体维度类型名(`Vec2`/`Vec3`/`Vec4`、`Mat2`/`Mat3`/`Mat4`)编码维度,**不使用 const 泛型维度**,故不触发 RD-007。RD-007 **非 M7 验收门**(M7_CONTRACT out_of_scope / §6,inherited;owner M6→M7 顺延);本文件**不实现 RD-007**,亦不改 [consteval.md](consteval.md) RXS-0064 语义。若后续几何原语 / 数组长度类条款确需 const 泛型值运行期单态化语义,**停下标注「需人工升档」**,按 14 §4 处置,不在本文件自行落笔。
- **软光栅 unsafe 逃生**:全 safe 代码目标下的 unsafe 落点语义属 G0 软光栅 kernel 作用面(D-M7-3,后续里程碑 spec 段),不在本文件 core 数学库类型面登记;触及即停下标注「需人工升档」。
- **既有禁区**:不碰 device 原子 lowering 与 `atom.{order}.{scope}` PTX 映射(D-406 / RD-008 人工落笔禁区);本文件全 safe、host+device 双路径数值同义,不引入任何 device-only unsafe 语义。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-15 | 新建 spec/stdlib.md(M7.1 core 数学库类型面起始文件):登记编号区间 RXS-0104 起续号预留 + 文件级前言 / 范围(Vec `VecN<T>` N∈{2,3,4} / Mat `MatRxC<T>` / swizzle / 几何原语 Point·Vector·Normal·AABB·Ray 的构造·分量访问与 swizzle·逐元素算术·点积/叉积/范数·矩阵乘·几何谓词,全 safe、host+device 双路径同义)/ 依据与授权(01 §6 UC-03 + 08 §5 stdlib 充实 + 05 §1 device⊂host + 11 §3 M7;M7_CONTRACT D-M7-1 / G-M7-4 / G-M7-5 `rfc_required: none` + M7_PLAN M7.1)/ 计划条款骨架(预留,非裸条款头)/ 错误码先行引用占位说明 / 升档·禁区留痕。**沿 README v1.15 toolchain.md 先例:本轮不落带编号裸条款头**——条款体与 ≥1 测试锚定随下一轮实现 PR 同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定),无体例变更 | Direct |
| v1.1 | 2026-06-15 | 落地带编号条款体 RXS-0104 ~ RXS-0109(M7.1 core 数学库 Vec/Mat/swizzle 类型面首批:Vec 类型与构造 / 分量访问与 swizzle / 逐元素算术 / 点积·叉积·范数 / Mat 类型与构造 / Mat 逐元素算术与矩阵乘),每条 ≥1 conformance 锚定样例(`conformance/stdlib/**`:host 结构体 API 真跑 + device 标量分量原语 codegen,trace_matrix 维持全锚定)。实现裁决:host 路径具体 f32 结构体 + inherent `device fn` 方法(swizzle 方法形),device 路径在 NVPTX codegen 标量子集下以语义同义标量分量 `device fn` 原语实现(聚合值类型 device codegen 为后续扩展,§5);元素类型规范性收窄 f32、维度以具体类型名编码不用 const 泛型(RD-007 不触碰)。错误码:Legality 仅引用既有 2xxx 类型类诊断 RX2001/RX2002/RX2003/RX2004(§4 引用汇总),不新增 / 不预造错误码、不改 error_codes.json 与 en.messages。§1 编号区间更新为 RXS-0104 ~ RXS-0109;§3 几何原语保留为预留(不落裸条款头)。授权:01 §6 UC-03 + 08 §5 stdlib 充实 + 05 §1 device⊂host + 11 §3 M7,M7_CONTRACT D-M7-1 / G-M7-4 / G-M7-5 `rfc_required: none` | Direct |
| v1.2 | 2026-06-16 | 落地带编号条款体 RXS-0110 ~ RXS-0113(M7.1 几何原语 / 谓词:几何向量类语义区分与互转 Point3·Vector3·Normal3 / AABB·Ray 类型与构造 / 几何谓词 Point∈AABB 包含与点到 AABB 距离 / Ray–AABB 相交 slab 法),每条 ≥1 conformance 锚定(`conformance/stdlib/host/geom_ops.rx` 结构体 API 真跑 + `conformance/stdlib/device/geom_scalar.rx` 标量分量谓词原语 codegen + `conformance/stdlib/reject/geom_type_confusion/` 类型互斥 → RX2001,trace_matrix 维持全锚定)。实现裁决沿用 v1.1 口径:host 路径具体 f32 结构体 + inherent `device fn` 方法,device 路径标量分量 `device fn` 原语(布尔谓词以 f32 1.0/0.0 表达,与 bool 数值同义;距离 / 归一化复用 RXS-0107 rx_sqrt),零编译器改动;维度以具体类型名编码、元素类型 f32,不用 const 泛型(RD-007 不触碰),device 走标量子集(不触聚合 device codegen)。错误码:Legality 仅引用既有 2xxx 类型类诊断 RX2001/RX2002/RX2003(§4 引用汇总),不新增 / 不预造错误码、不改 error_codes.json 与 en.messages。§1 编号区间更新为 RXS-0104 ~ RXS-0113;§3 预留骨架升格为「几何原语条款落地说明」(条款体见 §2)。授权:01 §6 UC-03 + 08 §5 stdlib 充实 + 05 §1 device⊂host + 11 §3 M7,M7_CONTRACT D-M7-1 / G-M7-4 / G-M7-5 `rfc_required: none` | Direct |
