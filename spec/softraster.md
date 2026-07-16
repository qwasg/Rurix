# Rurix 语言规范 — 软光栅 kernel 语义面(G0 compute 软光栅;M7.3 起)

> 条款:RXS-0118 ~ RXS-0121(M7.3 G0 compute 软光栅 kernel 语义面:图元分桶到 tile(binning)/ tile 光栅(覆盖判定·重心坐标·边函数)/ 深度(z-buffer 写入与深度测试)/ tonemap(HDR→LDR 像素量化))。体例见 [README.md](README.md)。
> 依据:01 §6(UC-03 旗舰用例:SPH 仿真 + 软光栅出图);06(GPU 图形编程模型——kernel 抽象 / tile 调度 / shared+barrier 安全并行基元);07 §7(device codegen 作用面,NVPTX 标量子集);05 §1(device ⊂ host——同一标量语义在 host 与 device 两个执行世界一致);11 §3 M7(标准库充实与 G0 图形演示)。授权:[../milestones/m7/M7_CONTRACT.md](../milestones/m7/M7_CONTRACT.md)(`in_scope: g0_soft_raster` / `spec_m7_clauses`,D-M7-3,G-M7-3 / G-M7-1 子集 / G-M7-5,`rfc_required: none`)+ [../milestones/m7/M7_PLAN.md](../milestones/m7/M7_PLAN.md) §3 M7.3 第 1·2 项。
> 档位:**Direct**。本文是对 01/06/11 已锁定决策(UC-03 旗舰用例 / G0 软光栅 demo / device ⊂ host)的初版条款化、纯追加且尚无 stable 面;**agent 自主判档**,判档以 M7_CONTRACT.md YAML 头 `rfc_required: none` 与上述授权为据,判档争议向上取严。任何偏离已锁定决策、或触及 **const 泛型值运行期单态化(RD-007)** / **device 原子 lowering 与 `atom.{order}.{scope}` PTX 映射(D-406 / RD-008 agent 自主落笔的高敏面)** / **软光栅 unsafe 逃生** / **编译器侧诊断扩面**的条款,必须停下标注「需升档」,不在本文件自行落笔(10 §3,M7_CONTRACT §6 / out_of_scope)。
> 规范先行(AGENTS.md 硬规则第 7 条):**条款 PR 先于实现 PR**;缺条款的语义 PR 必须先补 spec。`ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 ≥1 测试锚定(`//@ spec: RXS-####`),本轮带编号条款体(RXS-0118 ~ RXS-0121)连同每条 ≥1 锚定(`conformance/soft_raster/**` device codegen 样例 + `src/soft-raster` crate 确定性单测)一并落地,该门维持全绿。
> device 路径说明(NVPTX codegen 标量子集):软光栅 kernel 以 `kernel fn` / `device fn` 在当前 NVPTX codegen **标量值类型子集**(`f32` / `usize` 标量 + `View<global, f32>` / `ViewMut<global, f32>` 索引 + `shared let [f32; N]` + `ThreadCtx<DIM>` 线程索引 + `block.sync()` barrier;聚合 / 结构体值类型 codegen 为后续扩展,作用面外报 `RX6003`,见 §4)下表达,复用 [stdlib.md](stdlib.md)(M7.1)核心数学库的标量分量 `device fn` 原语(`rx_sqrt` / `rx_min` / `rx_max` / 边函数等),经 device codegen + `ptxas` 干验证(RXS-0073)。host 路径以语义同义的全 safe CPU 参考实现(`src/soft-raster`,复用 [imageio.md](imageio.md) 的 `ImageBuffer` / `Rgb` 像素口径与 `f32→u8` 确定量化)产**确定性帧像素**,两路径标量数值语义同义(05 §1 device ⊂ host)。
> 全 safe 代码目标(M7 CI_GATES §4 第 3 项,G-M7-3):软光栅 kernel(device `.rx`)与 host CPU 参考(`src/soft-raster`)均维持 `unsafe_code = "deny"`;凡落 unsafe 须按 AGENTS 硬规则 9 注册 unsafe-audit 条目 + 每 unsafe 块 `// SAFETY:` + safe 覆盖率报告留痕原因(反哺 views 扩展清单),本轮**零 unsafe**。
> 确定性(14 §3 风险对策:消除非确定性源):固定输入 → 逐字节确定帧缓冲。各 kernel 的归约序 / tile 调度序 / 分桶遍历序 / 深度合成序在实现内**固定一致**;每像素 / 每桶由**单一 owner 线程独写**,**不使用 device 原子**(规避 D-406 / RD-008 禁区),对齐 `reduce.rx` / `transpose.rx` 的 atomics-free 纪律。

---

## 1. 范围与编号区间

本文件承载 **G0 compute 软光栅 kernel 语义面**的语义条款(M7.3,D-M7-3)。覆盖语义面:

- **图元分桶到 tile(binning)**:将屏幕空间图元(三角形,以三顶点 `f32` 屏幕坐标表达)按其包围盒与 tile 网格的覆盖关系分桶到各 tile 的图元列表;确定性遍历序、每桶 owner 线程独写、atomics-free。
- **tile 光栅(覆盖判定 / 重心坐标 / 边函数)**:在 tile 内逐像素以**边函数**符号判定三角形覆盖,并以**重心坐标**对顶点属性插值;2D tile 调度确定序。
- **深度(z-buffer 写入与深度测试)**:z-buffer 写入与深度测试(`less` 比较)语义;确定性深度合成序(每像素 owner 线程按固定图元序比较,无原子)。
- **tonemap(HDR→LDR 像素量化)**:HDR `f32` 颜色 → LDR `u8` 像素的确定量化;量化口径**对接 [imageio.md](imageio.md) RXS-0116**(钳制 `[0,1]`、NaN→0、`floor(clamp(c)*255+0.5)` 半值向上)与 M7.1 标量 `f32` 像素口径([stdlib.md](stdlib.md) §1)。

全部为**全 safe** 代码目标;两路径(device `kernel`/`device fn` 标量子集 / host CPU 参考)标量数值语义同义(05 §1 device ⊂ host;11 §3 M7)。图元 / tile / 缓冲维度以运行期 `usize` 索引与 `View<global>` 长度表达,**不使用 const 泛型值维度**(RD-007 不触碰,见 §4)。

**编号区间**:本文件条款自 **RXS-0118** 起续号(全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1;M7.2 止于 RXS-0117)。本轮落地 **RXS-0118 ~ RXS-0121**。区间登记于 [README.md](README.md) §4 文件清单。

## 2. 条款

> 每条按需分 Syntax / Legality / Dynamic Semantics / Implementation Requirements 节,**严禁 UB 节**(UB 为经 Full RFC 由 agent 自主落笔的高敏面,10 §7.5)。Legality 违例只**引用**错误码(§3 引用汇总),不在此定义其含义;运行期边界(越界 / 退化)以确定性返回值 / 哨兵定义,**不进入未定义行为**。

### RXS-0118 图元分桶到 tile(binning)

**Syntax**(binning kernel 形态,标量子集;`f32` 屏幕坐标 + `View<global>` 图元 / 桶缓冲):

```
BinningKernel ::= "kernel" "fn" Ident "(" ThreadParam "," ViewParams "," DimParams ")" Block
ThreadParam   ::= Ident ":" "ThreadCtx" "<" "1" ">"
ViewParams    ::= 图元属性 View<global, f32> + tile 桶计数 / 列表 ViewMut<global, f32>
DimParams     ::= tile 网格宽高 / 图元数 / 每桶容量 (usize)
```

**Legality**:

- binning kernel 着色为 `kernel fn`(`ptx_kernel` 入口,RXS-0070),图元属性 / tile 桶缓冲以 `View<global, f32>` / `ViewMut<global, f32>` 传入,网格与计数维度以 `usize` 传入;均为标量值类型子集构造。作用面外构造(聚合 / 结构体值类型参数等)→ `RX6003`(device codegen 暂不支持构造,见 §4)。
- 每个 tile 桶的计数 / 图元下标列表由**唯一 owner 线程**(`global_id` 映射到 tile)写入;binning **不使用 device 原子**(`atom.*`)、不跨线程写同一桶槽位。全 safe,不含 `unsafe`。

**Dynamic Semantics**:

- 屏幕被划分为 `tiles_x × tiles_y` 个固定大小 tile。图元 `k`(`k = 0 … prim_count-1`)的屏幕包围盒 `[bx0,bx1]×[by0,by1]` 由其三顶点坐标的逐分量 `min`/`max` 求得;图元 `k` **覆盖** tile `(tx, ty)` 当且仅当其包围盒与 tile 的像素矩形相交(逐轴区间交非空)。
- tile `(tx, ty)`(线性下标 `ty * tiles_x + tx`)的 owner 线程**按图元下标 `k` 升序**遍历全部图元,将覆盖本 tile 的图元下标依序追加入本桶列表,直至达到每桶容量 `cap`(溢出图元按确定性丢弃,不写越界存储);桶计数为追加的图元数。该遍历序固定,故分桶结果对固定输入**逐次一致**(确定性)。
- 越界访问(图元下标 / 桶槽位超出 `View` 长度或 `cap`)以确定性边界(跳过 / 截断)处理,不读写越界存储、不进入未定义行为。

**Implementation Requirements**:

- device 路径以 `kernel fn` + 标量分量包围盒 / 区间相交 `device fn` 原语实现(NVPTX codegen 标量子集,§4),每 tile owner 线程独写本桶(atomics-free,确定性遍历序)。host 路径以语义同义的全 safe CPU 参考(`src/soft-raster`)实现同一分桶序。两路径分桶结果对固定输入一致。

> 锚定测试:`conformance/soft_raster/device/sr_binning.rx`(binning kernel device codegen + ptxas 干验证);`src/soft-raster`(`#[cfg(test)]`:固定图元 / tile 网格分桶结果确定性)。

### RXS-0119 tile 光栅:覆盖判定 / 重心坐标 / 边函数

**Syntax**(tile 光栅 kernel 形态,2D 线程上下文 + 标量边函数 `device fn`):

```
RasterKernel ::= "kernel" "fn" Ident "(" Ident ":" "ThreadCtx" "<" "2" ">" "," ViewParams "," DimParams ")" Block
EdgeFn       ::= "device" "fn" Ident "(" "ax" ":" "f32" "," "ay" ":" "f32" "," "bx" ":" "f32" "," "by" ":" "f32" "," "px" ":" "f32" "," "py" ":" "f32" ")" "->" "f32"
```

**Legality**:

- 光栅 kernel 着色为 `kernel fn`,以 `ThreadCtx<2>` 映射 tile 内像素 `(x, y)`;覆盖判定 / 插值以标量 `f32` 边函数 / 重心权重 `device fn` 原语表达(标量子集)。作用面外构造 → `RX6003`(§4)。
- 覆盖结果以 `f32`(`1.0`/`0.0`)或分支表达(与 `bool` 数值同义,对齐 [stdlib.md](stdlib.md) RXS-0112 谓词口径);全 safe,不含 `unsafe`。

**Dynamic Semantics**:

- **边函数**:对有向边 `A→B` 与点 `P`,`edge(A, B, P) = (B.x − A.x)·(P.y − A.y) − (B.y − A.y)·(P.x − A.x)`(二维叉积,IEEE-754 `f32`)。
- **覆盖判定**:三角形 `(V0, V1, V2)` 覆盖像素中心 `P` 当且仅当三条边函数 `e0 = edge(V1, V2, P)`、`e1 = edge(V2, V0, P)`、`e2 = edge(V0, V1, P)` **同号**(取约定:对面积非负的逆时针三角形,`e0 ≥ 0 ∧ e1 ≥ 0 ∧ e2 ≥ 0` 为覆盖);边上像素(某 `ei == 0`)按 `≥ 0` 约定确定性归入覆盖,边界判定在两路径一致。
- **重心坐标**:设三角形二倍面积 `area2 = edge(V0, V1, V2)`。当 `area2 == 0.0`(退化三角形)时该三角形**不覆盖任何像素**(确定性,不产生除零、不进入未定义行为);否则重心权重 `w0 = e0 / area2`、`w1 = e1 / area2`、`w2 = e2 / area2`(`w0 + w1 + w2 = 1`),顶点属性 `attr` 的插值为 `w0·attr0 + w1·attr1 + w2·attr2`。
- **tile 调度序**:tile 内像素 `(x, y)` 由 `ThreadCtx<2>` 唯一映射、各像素独立求值,调度序不影响每像素确定结果。

**Implementation Requirements**:

- device 路径以 `kernel fn` + 标量 `edge` / 重心权重 `device fn` 原语实现(NVPTX codegen 标量子集,§4);host 路径以语义同义全 safe CPU 参考(`src/soft-raster`)实现同一边函数 / 覆盖约定 / 重心插值。边函数 / 比较 / 插值的运算序在两路径**固定一致**以保数值可复现(确定性归约,14 §3)。退化三角形按上述确定性边界处理。

> 锚定测试:`conformance/soft_raster/device/sr_raster_tile.rx`(覆盖 / 边函数 / 重心 device codegen + ptxas 干验证);`src/soft-raster`(`#[cfg(test)]`:边函数符号 / 覆盖判定 / 重心插值 / 退化三角形)。

### RXS-0120 深度:z-buffer 写入与深度测试

**Syntax**(深度 kernel 形态,1D 线程上下文 + z-buffer `ViewMut<global, f32>`):

```
DepthKernel ::= "kernel" "fn" Ident "(" Ident ":" "ThreadCtx" "<" "1" ">" "," ViewParams "," DimParams ")" Block
```

**Legality**:

- 深度 kernel 着色为 `kernel fn`,z-buffer / 颜色目标以 `ViewMut<global, f32>` 传入,候选深度 / 颜色以 `View<global, f32>` 传入。作用面外构造 → `RX6003`(§4)。
- 每像素的 z-buffer 槽位 / 颜色目标由**唯一 owner 线程**(`global_id` 映射到像素)读写;深度合成**不使用 device 原子**、不跨线程写同一像素槽位。全 safe,不含 `unsafe`。

**Dynamic Semantics**:

- z-buffer 初值为远平面哨兵 `zfar`(约定 `+∞` 或实现选定的最大深度)。**深度测试**采用 `less` 约定:候选片元深度 `z_cand` 通过测试当且仅当 `z_cand < z_buf`(严格小于,IEEE-754 `f32` 比较);通过则**写入** `z_buf = z_cand` 并更新该像素颜色,否则保持原值(深度遮挡)。
- **确定性深度合成序**:像素 `(x, y)` 的 owner 线程**按固定片元 / 图元序**(如分桶后桶内图元下标升序,RXS-0118)依次对该像素做深度测试与条件写入;同一像素的全部候选由单一 owner 线程串行合成,无跨线程竞争。相等深度(`z_cand == z_buf`)按 `less` 严格比较**不覆盖**(保留先到者),故对固定输入与固定序帧像素**逐次一致**(确定性)。
- 越界(像素下标超出 `View` 长度)以确定性边界(跳过)处理,不读写越界存储、不进入未定义行为。

**Implementation Requirements**:

- device 路径以 `kernel fn` + 标量深度比较 / 条件写 `device fn` 原语实现(NVPTX codegen 标量子集,§4),每像素 owner 线程串行合成(atomics-free,固定片元序)。host 路径以语义同义全 safe CPU 参考(`src/soft-raster`)实现同一 `less` 测试与固定合成序。两路径 z-buffer / 颜色结果对固定输入一致。

> 锚定测试:`conformance/soft_raster/device/sr_depth.rx`(z-buffer 写入 / 深度测试 device codegen + ptxas 干验证);`src/soft-raster`(`#[cfg(test)]`:less 深度测试 / 遮挡 / 相等不覆盖 / 固定合成序确定性)。

### RXS-0121 tonemap:HDR→LDR 像素量化

**Syntax**(tonemap kernel 形态,1D 线程上下文;HDR `f32` → LDR `f32`(量化值)):

```
TonemapKernel ::= "kernel" "fn" Ident "(" Ident ":" "ThreadCtx" "<" "1" ">" "," ViewParams "," DimParams ")" Block
QuantizeFn    ::= "device" "fn" Ident "(" "c" ":" "f32" ")" "->" "f32"   // 单分量确定量化
```

**Legality**:

- tonemap kernel 着色为 `kernel fn`,HDR 源 / LDR 目标以 `View<global, f32>` / `ViewMut<global, f32>` 传入(分量 `f32`,通道序 R, G, B 对接 [imageio.md](imageio.md) RXS-0114 像素口径)。作用面外构造 → `RX6003`(§4)。
- 量化为逐分量纯函数,每像素分量由唯一 owner 线程独写;全 safe,不含 `unsafe`。

**Dynamic Semantics**:

- **量化口径**(对接 [imageio.md](imageio.md) RXS-0116,逐分量):分量 `c`(`f32`)先钳制到 `[0.0, 1.0]`(`c < 0.0 → 0.0`,`c > 1.0 → 1.0`,`NaN → 0.0`),再以**确定取整** `q = floor(clamp(c) * 255.0 + 0.5)`(就近取整、半值向上)映射到 `[0, 255]` 的整数刻度值(以 `f32` 承载 `0.0 … 255.0`,与 host `u8` 量化数值同义)。
- HDR→LDR 为逐像素逐分量独立映射,**不引入有损量化以外的信息丢失**;tonemap 输出经 host 路径写入 [imageio.md](imageio.md) `Rgb` 像素 / PPM P6 字节(`f32→u8`,RXS-0116),整条帧字节流由 `(width, height, 像素分量值)` 唯一确定,固定输入两次运行**逐字节一致**。
- 越界(像素分量下标超出 `View` 长度)以确定性边界(跳过)处理,不读写越界存储、不进入未定义行为。

**Implementation Requirements**:

- device 路径以 `kernel fn` + 标量量化 `device fn` 原语(`clamp` + 就近取整,纯 `f32` 算术,**不依赖** libdevice device-only intrinsic)实现(NVPTX codegen 标量子集,§4);host 路径复用 [imageio.md](imageio.md) `f32→u8` 确定量化(RXS-0116)写 `Rgb` 像素 / PPM P6 字节。量化阈值 / 取整在两路径**固定一致**以保逐字节可复现(确定性,14 §3)。

> 锚定测试:`conformance/soft_raster/device/sr_tonemap.rx`(标量量化 device codegen + ptxas 干验证);`src/soft-raster`(`#[cfg(test)]`:量化边界 0/255/NaN/半值、帧像素 → PPM 字节确定性、固定输入两次落盘逐字节一致)。

## 3. 错误码引用汇总 / 库层错误值口径

> 本表仅**引用**既有错误码,含义以 [../registry/error_codes.json](../registry/error_codes.json) 为唯一事实源。软光栅 kernel 以 device codegen 标量子集 `kernel`/`device fn` 实现,作用面外构造天然落入既有 **6xxx codegen/目标段位诊断**(device codegen 不支持构造),**不新增错误码、不预造条目**(无 bespoke 诊断实现,M7 CI_GATES §4 第 2 项);故不改 [../registry/error_codes.json](../registry/error_codes.json) 与 `en.messages`。

| 错误码 | 含义 | 条款 |
|---|---|---|
| RX6003 | device codegen 暂不支持构造(聚合 / 结构体值类型作为 device 函数参数 / 返回 / 局部 / 字段投影等,在 NVPTX 标量子集作用面外) | RXS-0118 / RXS-0119 / RXS-0120 / RXS-0121 |

> **运行期边界以确定性返回值 / 哨兵表达**:软光栅 kernel 的越界(图元 / 桶 / 像素下标越界)、退化(退化三角形 / 零面积)、深度遮挡等边界以**确定性返回值 / 哨兵 / 跳过**定义(§2 各条 Dynamic Semantics),**不分配编译器 RX 段位、不设 UB 节**。host CPU 参考(`src/soft-raster`)的越界以 `ImageBuffer` 既有确定性 `None`/`false` 边界([imageio.md](imageio.md) RXS-0114)表达。若后续实测确需编译器侧 RX 段位诊断(error_codes.json 分配 + en.messages key,即触及**编译器诊断扩面**),按 §4 **停下标注「需升档」**,不在本文件 / 本轮自行落笔。

## 4. 升档 / 禁区留痕

- **device 原子 lowering 与 `atom.{order}.{scope}` PTX 映射(D-406 / RD-008 agent 自主落笔的高敏面)**:本文件软光栅 kernel(binning / 深度合成)以**每桶 / 每像素 owner 线程独写 + 确定性遍历序**实现 atomics-free 合成(对齐 `reduce.rx` / `transpose.rx` 的 shared+barrier 安全并行基元),**不引入任何 device 原子**(`atom.*`)。device 原子 lowering 与 `atom.{order}.{scope}` PTX 映射为 **D-406 / RD-008 agent 自主落笔的高敏面**;若后续 binning / 深度合成确需 scoped atomics 真原子路径,**停下标注「需升档」**,不在本文件自行落笔。
- **聚合值类型 device codegen(后续扩展,非禁区)**:当前 NVPTX codegen 为**标量值类型子集**,结构体 / 聚合值类型(顶点 / 三角形 / 像素结构体作为 device 函数参数 / 返回 / 局部 / 字段投影)在作用面外报 `RX6003`(RXS-0070 / RXS-0073)。故本轮 device 路径以**语义同义的标量分量 `device fn` 原语**实现并经 device codegen + `ptxas` 干验证;聚合值类型 device codegen 为后续工程扩展(届时 host 结构体 / M7.1 数学库 `Vec`/`Mat` API 可直接在 device 复用),其接通**不改本文件既有条款语义**(纯实现侧回填)。
- **const 泛型值运行期单态化(RD-007)**:本文件以运行期 `usize` 索引与 `View<global>` 长度表达图元 / tile / 缓冲维度,**不使用 const 泛型值维度**,故不触发 RD-007。RD-007 **非 M7 验收门**(M7_CONTRACT out_of_scope / §6,inherited);本文件**不实现 RD-007**,亦不改 [consteval.md](consteval.md) RXS-0064 语义。若后续 tile 尺寸 / 数组长度类条款确需 const 泛型值运行期单态化语义,**停下标注「需升档」**。
- **软光栅 unsafe 逃生**:全 safe 代码目标下软光栅 kernel(device `.rx`)与 host CPU 参考(`src/soft-raster`)均维持 `unsafe_code = "deny"`,本轮**零 unsafe**。若实现期被迫落 unsafe(如裸指针访存),须先建 unsafe-audit 注册条目 + 每 unsafe 块 `// SAFETY:` + safe 覆盖率报告留痕原因(反哺 views 扩展清单,M7 CI_GATES §4 第 3 项),并就近**停下标注「需升档」**评估,不擅自扩 unsafe 面。
- **编译器诊断扩面**:软光栅运行期边界以确定性返回值 / 哨兵表达(§3),不触发编译器诊断。若确需编译器侧 RX 段位诊断(error_codes.json 新条目 + en.messages key)→ **停下标注「需升档」**,不擅自落笔(`registry/error_codes.json` 既有条目含义冻结,只追加且若触及即停手升档)。
- **realtime 窗口呈现 / demo 出图**:本文件只承载 G0 软光栅 **kernel 语义面**;软光栅 demo 单 EXE 出图(UC-03,G-M7-1)由 M7.4 承接、实时窗口呈现为 G1-1(M7_CONTRACT out_of_scope),均不在本文件作用面。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-16 | 新建 spec/softraster.md(M7.3 G0 compute 软光栅 kernel 语义面起始文件):落地带编号条款体 RXS-0118 ~ RXS-0121(RXS-0118 图元分桶到 tile binning·确定性遍历序·每桶 owner 线程独写·atomics-free / RXS-0119 tile 光栅覆盖判定·重心坐标·边函数 / RXS-0120 深度 z-buffer 写入与深度测试·less 约定·确定性深度合成序 / RXS-0121 tonemap HDR→LDR 像素量化·对接 imageio RXS-0116 f32→u8 确定量化与 M7.1 标量 f32 像素口径),每条 ≥1 锚定(`conformance/soft_raster/**` device codegen 样例 + `src/soft-raster` crate 确定性单测,trace_matrix 维持全锚定)。实现裁决:device 路径 NVPTX codegen 标量子集(`kernel`/`device fn` + `View<global, f32>` + `shared` + `ThreadCtx` + `block.sync()`,复用 M7.1 标量分量原语),host 路径全 safe CPU 参考(`src/soft-raster`,复用 imageio `ImageBuffer`/`Rgb`/PPM 确定编码);全 safe 代码目标(`unsafe_code=deny`),本轮零 unsafe。确定性:固定输入两次运行帧像素逐字节一致(固定归约 / tile 调度 / 分桶 / 深度合成序,每像素/桶 owner 线程独写,atomics-free)。维度以运行期 usize/View 长度编码,不用 const 泛型(RD-007 不触碰);不引入 device 原子(D-406/RD-008 禁区)。错误码:Legality 仅引用既有 6xxx codegen 段位诊断 RX6003(§3 引用汇总),不新增 / 不预造错误码、不改 error_codes.json 与 en.messages;运行期边界以确定性返回值/哨兵表达,若确需编译器侧 RX 诊断则停手升档。§1 编号区间登记 RXS-0118 ~ RXS-0121;README §4 文件清单 + §5 修订行同 PR 登记。授权:01 §6 UC-03 + 06 GPU 模型 + 07 §7 device codegen + 05 §1 device⊂host + 11 §3 M7,M7_CONTRACT D-M7-3 / G-M7-3 / G-M7-1 子集 / G-M7-5 `rfc_required: none` | Direct |
