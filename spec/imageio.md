# Rurix 语言规范 — image-io 接口语义(确定性图像序列输出;M7.2 起)

> 条款:RXS-0114 ~ RXS-0117(M7.2 image-io 接口语义面:图像缓冲与像素类型面 / 无损格式优先与格式选择 / 确定性字节布局与 header 规范化 / 图像序列落盘接口)。体例见 [README.md](README.md)。
> 依据:01 §6(UC-03 旗舰用例:SPH 仿真 + 软光栅出图,需确定性出图落盘);08 §5(stdlib 充实);09 §5 / §7(生态包形态——`image-io` PNG/PPM 等无损格式读写,G0 出图依赖,经 M6 包管理 `rurix.toml` 集成);11 §3 M7(标准库充实与 G0 图形演示)。授权:[../milestones/m7/M7_CONTRACT.md](../milestones/m7/M7_CONTRACT.md)(`in_scope: image_io_pkg` / `spec_m7_clauses`,D-M7-2,G-M7-1 子集 / G-M7-5,`rfc_required: none`)+ [../milestones/m7/M7_PLAN.md](../milestones/m7/M7_PLAN.md) §2 M7.2 第 1 项。
> 档位:**Direct**。本文是对 01/08/09/11 已锁定决策(UC-03 旗舰用例 / stdlib 充实 / image-io 生态包形态 / G0 软光栅 demo 出图）的初版条款化、纯追加且尚无 stable 面;**agent 自主判档**,判档以 M7_CONTRACT.md YAML 头 `rfc_required: none` 与上述授权为据,判档争议向上取严。任何偏离已锁定决策、或触及 **const 泛型值运行期单态化(RD-007)** / **软光栅 unsafe 逃生** / **device codegen 牵入** / **编译器侧诊断扩面**的条款,必须停下标注「需升档」,不在本文件自行落笔(10 §3,M7_CONTRACT §6 / out_of_scope)。
> 规范先行(AGENTS.md 硬规则第 7 条):**条款 PR 先于实现 PR**;缺条款的语义 PR 必须先补 spec。`ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 ≥1 测试锚定(`//@ spec: RXS-####`),本轮带编号条款体(RXS-0114 ~ RXS-0117)连同每条 ≥1 锚定(`src/image-io` crate 内确定性单测)一并落地,该门维持全绿。
> 单路径说明(**host-only**):image-io 为 **host 路径**确定性图像编码与落盘子系统(09 §5 出图依赖),**不引入 device 执行路径**——区别于 [stdlib.md](stdlib.md) core 数学库的 host+device 双路径同义。本文件不牵入 NVPTX device codegen / device-only intrinsic / 软光栅 kernel 语义(后者属 D-M7-3 软光栅作用面,后续里程碑 spec 段)。

---

## 1. 范围与编号区间

本文件承载 **image-io 接口语义面**的语义条款(M7.2,D-M7-2)。覆盖语义面:

- **图像缓冲与像素类型面**:像素 `Rgb` / `Rgba`(分量 `f32`,通道序 R,G,B(,A)),`ImageBuffer`(宽 × 高 + 行主序紧致像素存放);像素分量表示口径**复用 M7.1 数学库标量 `f32` 像素口径**([stdlib.md](stdlib.md) §1,`VecN<T>` 的 `T = f32`)。
- **无损格式优先与格式选择**:无损格式优先序 **PPM(P6,二进制)优先 / PNG 次**;M7.2 落地 PPM P6 编码,PNG 为加性后续(不改既有条款语义)。
- **确定性字节布局与 header 规范化**:固定输入 → 逐字节确定字节流;PPM P6 header 规范化、像素行 / 列序、通道序、`f32 → u8` 确定量化与字节序。
- **图像序列落盘接口**:确定性图像序列 sink(逐帧编码 → 落盘);逐帧 content SHA-256 可核对,同输入两次落盘逐字节一致。

全部为**全 safe** API(`unsafe_code = "deny"`,M7 CI_GATES §4.3);host 路径确定性纯函数编码(同一输入在不同机器 / 时刻产同一字节流,为 M7.4 UC-03 demo 出图落盘与 G-M7-1 逐帧 content SHA-256 复现铺底)。

**编号区间**:本文件条款自 **RXS-0114** 起续号(全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1;M7.1 止于 RXS-0113)。本轮落地 **RXS-0114 ~ RXS-0117**。区间登记于 [README.md](README.md) §4 文件清单。

## 2. 条款

> 每条按需分 Syntax / Legality / Dynamic Semantics / Implementation Requirements 节,**严禁 UB 节**(UB 为经 Full RFC 由 agent 自主落笔的高敏面,10 §7.5)。Legality 违例只**引用**错误码(§3 引用汇总),不在此定义其含义;运行期失败(格式不支持 / 写入失败)以**库层 `Result` 错误值**表达,不分配编译器 RX 段位(§3)。

### RXS-0114 图像缓冲与像素类型面

**Syntax**(像素类型与缓冲构造形态;分量 `f32`):

```
PixelType  ::= "Rgb" | "Rgba"
PixelCtor  ::= "Rgb"  "::" "new" "(" Expr "," Expr "," Expr ")"          // (r, g, b),f32
             | "Rgba" "::" "new" "(" Expr "," Expr "," Expr "," Expr ")"  // (r, g, b, a),f32
BufferCtor ::= "ImageBuffer" "::" "new" "(" Expr "," Expr "," Expr ")"     // (width, height, fill: Pixel)
BufferSet  ::= Expr "." "set" "(" Expr "," Expr "," Expr ")"               // (x, y, pixel)
BufferGet  ::= Expr "." "get" "(" Expr "," Expr ")"                        // (x, y) -> Pixel
```

**Legality**:

- 像素 `Rgb` / `Rgba` 分量字段依序为 `r`,`g`,`b`(,`a`),类型 `f32`(通道序 R,G,B(,A));派生 `Copy` / `Clone`。`Rgb` / `Rgba` 为**两个互异**像素类型,在期望某一像素类型处误用另一类型 → `RX2001`。
- `Rgb::new` / `Rgba::new` 实参元数须分别为 3 / 4,元素类型 `f32`;实参元数不符 → `RX2003`;实参类型非 `f32` → `RX2001`。
- `ImageBuffer` 以无符号整数 `width` × `height` 与统一像素元素类型参数化;行主序紧致存放 `width * height` 个像素。`set` / `get` 的坐标 `(x, y)` 满足 `0 ≤ x < width`、`0 ≤ y < height`(越界访问的运行期行为见 Dynamic Semantics,以确定性返回值定义,**不设 UB 节**)。
- 构造与访问全 safe,不含 `unsafe`。

**Dynamic Semantics**:

- `Rgb::new(r,g,b)`(`Rgba` 同,含 `a`)产分量依序为给定值;分量为 IEEE-754 `f32`。
- `ImageBuffer::new(w,h,fill)` 产 `w × h` 缓冲,每像素初始化为 `fill`。像素以**行主序**逻辑布局:像素 `(x, y)`(列 `x`、行 `y`)的线性下标为 `y * width + x`。
- `set(x, y, p)` 将 `(x, y)` 处像素置为 `p`;`get(x, y)` 取 `(x, y)` 处像素副本(值语义)。坐标越界时以确定性失败值(库层 `Result` / 哨兵)返回,不读写越界存储、不进入未定义行为。

**Implementation Requirements**:

- host 路径以具体 `f32` 像素结构体 + `ImageBuffer` 结构体(行主序 `Vec`-式紧致存放)实现,纯 safe;像素分量表示与 M7.1 数学库标量 `f32` 口径一致([stdlib.md](stdlib.md) §1),便于软光栅 / demo 像素与几何数学共用元素类型。本文件**不引入 device 路径**(host-only,前言)。

> 锚定测试:`src/image-io/src/lib.rs`(`#[cfg(test)]`:像素 / 缓冲构造、行主序 `get`/`set`、通道序)。

### RXS-0115 无损格式优先与格式选择

**Syntax**(格式枚举与编码入口,方法 / 函数形):

```
ImageFormat ::= "Ppm"            // PPM P6(二进制 RGB),M7.2 落地
              | "Png"            // PNG(无损),加性后续
Encode      ::= "encode" "(" Expr "," Expr ")"   // (&ImageBuffer, ImageFormat) -> Result<Vec<u8>, ImageError>
```

**Legality**:

- 编码入口接收图像缓冲与目标格式 `ImageFormat`,返回**库层 `Result`**:成功 → 确定字节流(`Vec<u8>`);格式不被当前实现支持 → 库层错误值 `ImageError`(**不分配编译器 RX 段位**,§3)。
- 全 safe。

**Dynamic Semantics**:

- **无损格式优先序**:image-io 以**无损格式优先**——`Ppm`(P6,二进制 RGB)为 M7.2 落地的首选编码格式;`Png`(无损)为次选,属加性后续(实现侧后续回填,**不改本条优先序语义**)。
- `encode(buf, Ppm)` 产 PPM P6 确定字节流(布局见 RXS-0116)。`encode(buf, fmt)` 在 `fmt` 尚未被当前实现支持时(如本轮 `Png`)返回 `Err(ImageError::UnsupportedFormat)`(库层错误值),不产生部分 / 非确定字节流。
- 无损编码**不引入有损量化以外的信息丢失**:除像素分量 `f32 → u8` 的确定量化(RXS-0116)外,像素网格与通道完整保留。

**Implementation Requirements**:

- host 路径以纯函数编码实现;格式选择以库层 `enum` + `Result` 表达,格式不支持 / 写入失败均为**库层错误值**(§3),不触发编译器诊断。PNG 编码器接通为后续工程扩展,其加入**不改本文件既有条款语义**(纯实现侧回填)。

> 锚定测试:`src/image-io/src/lib.rs`(`#[cfg(test)]`:`encode(buf, Ppm)` 成功产字节流、`encode(buf, Png)` 返回 `UnsupportedFormat` 库层错误)。

### RXS-0116 确定性字节布局与 header 规范化

**Syntax**:无新增产生式(PPM P6 字节流为 RXS-0115 `encode(buf, Ppm)` 的确定输出)。

**Legality**:

- PPM P6 编码要求像素元素类型为 `Rgb` / `Rgba`(`Rgba` 编码时丢弃 alpha 通道,仅写 RGB——PPM P6 无 alpha 通道);非 RGB(A) 像素类型 → `RX2001`。全 safe。

**Dynamic Semantics**(PPM P6 确定字节布局,固定输入 → 逐字节确定字节流):

- **header 规范化**:字节流前缀为 ASCII header,规范化为 `"P6\n{width} {height}\n255\n"`——魔数 `P6`、单个 `\n`(0x0A)分隔、`width` 与 `height` 为十进制 ASCII 以**单个空格**(0x20)分隔、最大色值固定 `255`、随后单个 `\n`。header 不含注释行、不含额外空白。
- **像素数据序**:header 之后为原始像素字节,按**行主序**自上而下(行 `y` 从 `0` 到 `height-1`)、自左而右(列 `x` 从 `0` 到 `width-1`)排列;每像素按**通道序 R, G, B** 依次写 3 个字节(`Rgba` 丢弃 alpha)。
- **`f32 → u8` 确定量化**:每分量 `c`(`f32`)先钳制到 `[0.0, 1.0]`(`c < 0.0 → 0.0`,`c > 1.0 → 1.0`,NaN → `0.0`),再以**确定取整** `u8_value = floor(clamp(c) * 255.0 + 0.5)`(就近取整、半值向上)映射到 `[0, 255]` 的 `u8`。
- **字节序**:像素分量为单字节 `u8`,无多字节字节序自由度;header 为 ASCII。整条字节流由 `(width, height, 像素分量值)` 唯一确定——同一输入在不同机器 / 时刻产**逐字节一致**字节流。

**Implementation Requirements**:

- host 路径以纯函数实现,量化 / 行列序 / 通道序 / header 文本在实现内**固定一致**以保逐字节可复现(确定性,14 §3 风险对策:消除非确定性源)。全 safe;越界 / 格式不支持以库层错误值返回(§3),不设 UB 节。

> 锚定测试:`src/image-io/src/lib.rs`(`#[cfg(test)]`:PPM P6 header golden 字节、通道序 / 行主序像素字节、`f32→u8` 量化边界、同输入两次编码 `Vec<u8>` 逐字节相等)。

### RXS-0117 图像序列落盘接口

**Syntax**:

```
SeqNew    ::= "ImageSequence" "::" "new" "(" Expr ")"        // (dir): 落盘目录
SeqPush   ::= Expr "." "push_frame" "(" Expr "," Expr ")"    // (&ImageBuffer, ImageFormat) -> Result<FrameRecord, ImageError>
FrameName ::= Expr "." "frame_path" "(" Expr ")"             // (index) -> 规范化帧文件名
```

**Legality**:

- `ImageSequence` 以落盘目录参数化,逐帧累加。`push_frame(buf, fmt)` 编码一帧(RXS-0115 / RXS-0116)并落盘,返回**库层 `Result`**:成功 → `FrameRecord`(含帧序号、规范化文件名、帧字节长度);编码失败(格式不支持)或写入失败 → 库层错误值 `ImageError`(`UnsupportedFormat` / `WriteFailed`,**不分配编译器 RX 段位**,§3)。
- 全 safe。

**Dynamic Semantics**:

- **帧序号与文件名规范化**:序列内帧自 `0` 起单调递增编号;第 `i` 帧规范化文件名为零填充定宽十进制序号 + 格式扩展名(PPM → `.ppm`),记 `frame_path(i)`(如 `frame_00000.ppm`)。同一序号产同一文件名(确定性命名)。
- **逐帧 content SHA-256 可核对**:每帧落盘字节即 RXS-0116 的确定 PPM P6 字节流;帧内容由 `(width, height, 像素值, 格式)` 唯一确定,故其 content SHA-256 在固定输入下可核对、且**两次运行逐字节一致**(G-M7-1 / M7.4 demo 出图复现地基)。
- **确定性序列**:给定同一帧输入序列与目录,两次落盘产同名文件且逐字节一致;序列不引入时间戳 / 随机量 / 平台相关字节。

**Implementation Requirements**:

- host 路径以纯函数编码(RXS-0116)+ 标准库文件写入实现,全 safe;写入失败(目录不存在 / IO 错误)以库层 `WriteFailed` 错误值返回,不 panic、不进入未定义行为。content SHA-256 核对由测试 / 冒烟侧(`ci/image_io_smoke.py` 与 crate 单测)对落盘字节计算背书;crate 本身只须保证落盘字节确定。帧命名 / 编码序在实现内固定一致以保逐字节可复现。

> 锚定测试:`src/image-io/src/lib.rs`(`#[cfg(test)]`:`frame_path` 规范化命名、`push_frame` 落盘字节与 `encode` 一致、序列同输入两次落盘逐字节相等)。

## 3. 错误码引用汇总 / 库层错误值口径

> 本表仅**引用**既有错误码(均为 2xxx 类型段位,07 §5),含义以 [../registry/error_codes.json](../registry/error_codes.json) 为唯一事实源。image-io 接口以具体 host 结构体 + inherent 方法实现,**类型误用**(像素 / 缓冲类型不匹配、实参元数 / 类型不符)天然落入既有**类型类诊断**,**不新增错误码、不预造条目**(无 bespoke 诊断实现,M7 CI_GATES §4.2);故不改 [../registry/error_codes.json](../registry/error_codes.json) 与 `en.messages`。

| 错误码 | 含义 | 条款 |
|---|---|---|
| RX2001 | 类型不匹配(`Rgb` / `Rgba` 像素类型互斥误用;像素 / 分量元素类型非 f32;非 RGB(A) 像素编码 PPM) | RXS-0114 / RXS-0116 |
| RX2003 | 实参数目不符(`Rgb::new` / `Rgba::new` 实参元数 ≠ 3 / 4) | RXS-0114 |

> **运行期失败以库层 `Result` 错误值表达**(M7_PLAN §2 第 4 项口径):image-io 的**格式不支持**(`ImageError::UnsupportedFormat`)与**写入失败**(`ImageError::WriteFailed`)为**库层错误值**(`Result::Err`),由调用方以值处理,**不分配编译器 RX 段位、不动 `registry/error_codes.json` 与 `en.messages`**。若后续实测确需编译器侧 RX 段位诊断(error_codes.json 分配 + en.messages key,即触及**编译器诊断扩面**),按 §4 **停下标注「需升档」**,不在本文件 / 本轮自行落笔。

## 4. 升档 / 禁区留痕

- **host-only,不牵 device codegen**:image-io 为 host 路径确定性编码子系统(09 §5 出图依赖),**不引入 device 执行路径 / NVPTX codegen / device-only intrinsic**。聚合 device codegen(RXS-0070 / RXS-0073 标量子集作用面外)、软光栅 device kernel(D-M7-3)均不在本文件作用面;触及即停下标注「需升档」。
- **编译器诊断扩面**:image-io 运行期诊断(格式不支持 / 写入失败)以库层 `Result` 错误值表达(§3)。若确需编译器侧 RX 段位诊断(error_codes.json 新条目 + en.messages key),即触及编译器诊断扩面 → **停下标注「需升档」**,不擅自落笔(M7 CI_GATES §4.2;`registry/error_codes.json` 既有条目含义冻结,只追加且若触及即停手升档)。
- **const 泛型值运行期单态化(RD-007)**:本文件以具体像素类型名(`Rgb` / `Rgba`)与运行期 `width × height` 编码缓冲,**不使用 const 泛型值维度**,故不触发 RD-007。RD-007 **非 M7 验收门**(M7_CONTRACT out_of_scope,inherited);本文件不实现 RD-007,亦不改 [consteval.md](consteval.md) 语义。若后续确需 const 泛型值运行期单态化语义,**停下标注「需升档」**。
- **软光栅 unsafe 逃生**:全 safe 代码目标下的 unsafe 落点语义属 G0 软光栅 kernel 作用面(D-M7-3,后续里程碑 spec 段),不在本文件 image-io 接口面登记;触及即停下标注「需升档」。
- **既有禁区**:不碰 device 原子 lowering / `atom.{order}.{scope}` PTX 映射(D-406 / RD-008 agent 自主落笔的高敏面);本文件全 safe、host-only,不引入任何 device-only / unsafe 语义。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-16 | 新建 spec/imageio.md(M7.2 image-io 接口语义面起始文件):落地带编号条款体 RXS-0114 ~ RXS-0117(图像缓冲与像素类型面 `Rgb`/`Rgba`/`ImageBuffer`,复用 M7.1 标量 f32 像素口径 / 无损格式优先与格式选择 PPM P6 优先·PNG 次 / 确定性字节布局与 PPM P6 header 规范化·行主序·通道序·f32→u8 确定量化 / 图像序列落盘接口逐帧 content SHA-256 可核对),每条 ≥1 锚定(`src/image-io` crate 内确定性单测,trace_matrix 维持全锚定)。实现裁决:**host-only 单路径**(不引入 device codegen,区别于 stdlib 双路径),纯函数确定性编码 + 标准库落盘,全 safe(`unsafe_code=deny`);维度以具体像素类型名 + 运行期宽高编码,不用 const 泛型(RD-007 不触碰)。错误码:Legality 仅引用既有 2xxx 类型类诊断 RX2001/RX2003(§3 引用汇总),不新增 / 不预造错误码、不改 error_codes.json 与 en.messages;运行期失败(格式不支持 / 写入失败)以库层 Result 错误值表达,若确需编译器侧 RX 诊断则停手升档。§1 编号区间登记 RXS-0114 ~ RXS-0117;README §4 文件清单 + §5 修订行同 PR 登记。授权:01 §6 UC-03 + 08 §5 stdlib 充实 + 09 §5/§7 image-io 包形态 + 11 §3 M7,M7_CONTRACT D-M7-2 / G-M7-1 子集 / G-M7-5 `rfc_required: none` | Direct |
