# B 路 strict-only 达标取证 — SPIR-V→DXIL 能否零静默降级(不动 P-01)

> 类型:**纯 spike 取证报告**(Windows-only)。**不裁 A/B/混合架构**(硬规则 1,裁决权属 agent)、
> **不裁 P-01 规范性那条线**(转译保真非完美是否构成 P-01 违背/边界,留 agent,P-13)、
> 不改 src/spec/codegen、不动 D-131(混合尚待 agent 批 #100)/D-205 pin/toolchain.rs、不向 llvm-project 提交。
> 承 `evidence/dxil_b_graphics_sig_report.md` §5 的**默认 SPIRV-Cross 参数**三类保真损耗基线。
> agent 已定调:**P-01(strict-only,准永久公理)不开例外** → RFC-0004 §4.4 由"边界/例外声明"转为
> **达标判据**:B 要被接受,须证语言层零静默降级。本报告取证 B 链能否配置成语言层零静默降级,
> 从而**不靠任何 P-01 例外**就达标。
> Provenance:`Assisted-by: kiro:claude-opus-4-8`(agent 自主记录机器可核对事实,非代决、非代签)。
> 纪律:measured-first / blocked-honest——所有数字来自命令真实输出(IDxcValidator + dxv.exe 各 ×25);
> **validator accept 不等于名保真——必看签名 part dump**(吸取 slice3 假绿教训)。
> 既有 evidence(round-1~8 / slice3 / dxil_b_graphics_sig / dxil_a_graphics_sig_effort)全部 byte-unchanged;
> 本报告与 `dxil_b_strict_only_20260625.json` 为新增。

---

## 0. 核心命题与结论(TL;DR)

**命题**:agent 定调 P-01 不开例外。`evidence/dxil_b_graphics_sig_report.md` §5 用**默认 SPIRV-Cross 参数**测出
B 链三类 P-01 保真损耗:① 用户语义名 → 通用 `TEXCOORD`;② 寄存器/顺序重排;③ 未用 `SV_Position` 输入被消除。
本轮取证:用 **SPIRV-Cross 语义名保持配置**(非默认)重跑 B 全链,证三类损耗能否经配置消除 / 落语言契约线下 /
经显式报错落入 strict-only,从而 B **不靠 P-01 例外**达标 RFC-0004 §4.4。

**结论(measured)**:

- **损耗①(顶点阶段输入语义名)经配置可消除**:`POSITION`/`NORMAL` 等 IA 顶点缓冲按名绑定的输入语义名,
  经 `dxc -spirv -fspv-reflect`(携 `UserSemantic` 原 HLSL 语义串)+ `spirv-cross --set-hlsl-named-vertex-input-semantic`
  (经新增 `spirv_reflect.py` 解 SPIR-V 二进制**自动导出**,非硬编码)端到端**存活**。签名 part dump 实证:
  `vs_sig` ISG1 默认 `[TEXCOORD0,TEXCOORD1,TEXCOORD2,SV_VertexID0]` → 保名 `[POSITION0,NORMAL0,TEXCOORD0,SV_VertexID0]`。
- **损耗①(varying 语义名,vs-out→ps-in)spirv-cross 硬绑 `TEXCOORD#`、无输出语义保名 flag**;`SV_*` 系统值名恒保留。
  **拟判**:varying 名属语言契约线**下**(RXS-0155 阶段间接口=类型级字段匹配,非 HLSL 语义串)——**规范性裁断留 agent**。
- **损耗②(寄存器/顺序重排)、损耗③(未用 SV_Position 输入消除)保名后仍在**(与名保真正交)。
  **拟判**:②在契约线下(RXS-0154/0155 不约束寄存器/二进制布局,属 §4.6(a) 签名 ABI 禁区);③属标准死 I/O 消除
  (body 未读元素,对用户意图不可见)——**规范性裁断留 agent**。
- **strict-only 失败模式(设计级)**:对真留不住的契约线元素,可在 MIR→SPIR-V 降级侧显式检测 → 6xxx 结构化编译错误
  (设计面论证,不落 codegen 实现,详 §5)。
- **确定性 + validator**:保名配置 4 语料 IDxcValidator + dxv.exe 各 **×25 = 25/25 accept**(`{0x0:25}`)+
  保名 B 全链同输入 **×25 容器 SHA256 各样例一致**(deterministic)。

**判定落点**:①顶点输入名经配置可消除(measured);①varying / ② / ③拟判落契约线下或标准 DCE(**规范性裁断留 agent**)。
若 agent 拟判成立 → **B 可零静默降级达标 P-01,无需例外**(RFC-0004 §4.4 写成"经名保真达 strict-only",agent 落笔)。
本报告只摆事实 + 复现清单,**不裁 P-01 规范线、不裁 A/B**(P-13)。

---

## 1. 工具链(隔离不入库;version + SHA256 写进证据)

与 `dxil_b_graphics_sig_report.md` §1 同一套(apple-to-apple),SHA256 前 16 hex(命令 `Get-FileHash -Algorithm SHA256` 真实输出):

| 角色 | 工具 | 版本 | SHA256(前 16) |
|---|---|---|---|
| SPIR-V producer(保名携 `-fspv-reflect`) | dxc.exe -spirv(Vulkan SDK 1.3.296.0) | `1.8.0.4739 (d9a5e97d0)` | `8B1321A448742D96` |
| SPIR-V 合规 | spirv-val.exe(SPIRV-Tools) | `v2024.4` | `ABCB0DA88AB02991` |
| SPIR-V→HLSL(名保持 flag) | spirv-cross.exe | `vulkan-sdk-1.3.290.0-44-g65d73934`(2024-10-04) | `7DE3489184B050BB` |
| HLSL→DXIL + validator | dxc.exe(round-7 套件) | `1.9.2602.24 (d355aa836)` | `1367FD29D0EBBA5B` |
| 签名 validator DLL | dxcompiler.dll | `1.9.2602.24` | `9B5E10ED756C461B` |
| 签名 validator | dxil.dll | `1.9.2602.24` | `CBCFE883A09FD0CA` |
| CLI validator | dxv.exe | `1.9.2602.24` | `F26242EFB0197FFE` |
| SPIR-V producer 备选 | glslangValidator.exe | `15.0.0` | `6206C307B1E14213` |

- 全部隔离于 Vulkan SDK / `H:\dxc-round7`,不入库;中间 SPIR-V/HLSL/DXIL 临时目录产物不入库。
- B 链最终 HLSL→DXIL 与直产基线均用同一 round-7 dxc `1.9.2602.24`(签名 validator 年代编译器),apple-to-apple。

## 2. 名保真重测机制(非默认配置;经 SPIR-V 反射自动导出,非硬编码)

默认 B 链(`dxil_b_graphics_sig_report.md` §5 基线)用 `dxc -spirv`(无 reflect)+ `spirv-cross --hlsl`(默认参数):
SPIR-V 只携 `Location` 编号,不携 HLSL 语义串 → spirv-cross HLSL 后端按约定写 `TEXCOORD#`(损耗①)。

本轮**名保持配置**两段叠加(`spike/dxil-path-probe/probe_b_strict_only.py`,标 `// SPIKE(RD-014)`):

1. **producer 侧**:`dxc -spirv **-fspv-reflect**` → 在 SPIR-V 写 `OpDecorateString <var> UserSemantic "<原 HLSL 语义>"`
   + 保留 `OpName`。实证(`spirv-dis` 真实输出节选,`vs_sig`):
   ```
   OpDecorateString %in_var_POSITION  UserSemantic "POSITION"
   OpDecorateString %in_var_NORMAL    UserSemantic "NORMAL"
   OpDecorateString %in_var_TEXCOORD0 UserSemantic "TEXCOORD0"
   OpDecorateString %gl_Position      UserSemantic "SV_Position"
   ```
2. **转译侧**:`spirv-cross --hlsl --set-hlsl-named-vertex-input-semantic <opname> <semantic>`,参数由新增
   `spirv_reflect.py`(SPIR-V 二进制最小反射:`OpEntryPoint` 执行模型 + `OpName` + `OpDecorate BuiltIn` +
   `OpDecorateString UserSemantic` + `OpVariable` 存储类)**自动导出**——仅顶点阶段、非内建、`OpName`/`UserSemantic`
   均非空的 Input 变量(非硬编码 per-corpus)。实证(`vs_sig` 导出 flag):
   ```
   --set-hlsl-named-vertex-input-semantic in.var.POSITION  POSITION
   --set-hlsl-named-vertex-input-semantic in.var.NORMAL    NORMAL
   --set-hlsl-named-vertex-input-semantic in.var.TEXCOORD0 TEXCOORD0
   ```

**机制边界(measured)**:spirv-cross HLSL 后端
- **有** `--set-hlsl-named-vertex-input-semantic`(顶点**输入**语义保名,本轮用)与 `--set-hlsl-vertex-input-semantic`(按 location);
- **无**顶点/片元**输出 varying** 语义保名 flag——`--rename-interface-variable out` 仅改 HLSL 变量名,
  语义仍 `TEXCOORD#`(实测:`out 0 myNormal` → `float3 myNormal : TEXCOORD0;`)。
- `UserSemantic` 装饰 spirv-cross **不自动消费**为 HLSL 输出语义(实测:`-fspv-reflect` SPIR-V 直转 HLSL,varying 仍 `TEXCOORD#`)。

## 3. 签名 part dump 对照(本轮决定性)——默认 TEXCOORD vs 保名(direct = 作者意图上界)

每语料三链:`direct`(HLSL→dxc 直产=作者意图上界)/ `b_default`(默认参数 B 链=损耗基线)/ `b_keep`(保名配置 B 链)。
解 DXContainer ISG1/OSG1 part:各元素 `(语义名+index, system_value, register)`(`dxil_container.parse_signature_part` 真实输出)。

### vs_sig(vertex,富 SV 签名)

| part | direct(意图) | b_default(降级基线) | b_keep(保名) |
|---|---|---|---|
| ISG1 | `SV_VertexID0(reg0)·POSITION0·NORMAL0·TEXCOORD0` | `TEXCOORD0·TEXCOORD1·TEXCOORD2·SV_VertexID0(reg3)` | **`POSITION0·NORMAL0·TEXCOORD0·SV_VertexID0(reg3)`** |
| OSG1 | `SV_Position0(reg0)·NORMAL0·TEXCOORD0·COLOR0` | `TEXCOORD0·TEXCOORD1·TEXCOORD2·SV_Position0(reg3)` | `TEXCOORD0·TEXCOORD1·TEXCOORD2·SV_Position0(reg3)` |

→ ISG1 顶点输入名 `POSITION/NORMAL/TEXCOORD0` 经保名**端到端存活**(默认全降级 `TEXCOORD#`);
OSG1 varying 名保名后仍 `TEXCOORD#`(无输出语义 flag);`SV_VertexID`/`SV_Position` 名恒保留。

### ps_sig(pixel,SV 入 + MRT 出)

| part | direct(意图) | b_default | b_keep |
|---|---|---|---|
| ISG1(elemcount 4→3) | `SV_Position0·NORMAL0·TEXCOORD0·SV_IsFrontFace0` | `TEXCOORD0·TEXCOORD1·SV_IsFrontFace0` | `TEXCOORD0·TEXCOORD1·SV_IsFrontFace0` |
| OSG1 | `SV_Target0·SV_Target1` | `SV_Target0·SV_Target1` | `SV_Target0·SV_Target1` |

→ ps 输入为 varying(无顶点输入保名 flag 适用,导出 flag 为空)→ 名仍 `TEXCOORD#`;`SV_IsFrontFace`/`SV_Target ×2` 名恒保留;
ISG1 `SV_Position` **输入**(body 未读)被消除(elemcount direct 4→B 3,损耗③,保名后仍消除)。

### vs_passthrough / ps_texture(round-2 复用)

| 语料·part | direct | b_default | b_keep |
|---|---|---|---|
| vs_passthrough ISG1 | `POSITION0·TEXCOORD0` | `TEXCOORD0·TEXCOORD1` | **`POSITION0·TEXCOORD0`** |
| vs_passthrough OSG1 | `SV_Position0(reg0)·TEXCOORD0` | `TEXCOORD0·SV_Position0(reg1)` | `TEXCOORD0·SV_Position0(reg1)` |
| ps_texture ISG1(elemcount 2→1) | `SV_Position0·TEXCOORD0` | `TEXCOORD0` | `TEXCOORD0` |
| ps_texture OSG1 | `SV_Target0` | `SV_Target0` | `SV_Target0` |

→ vs_passthrough 顶点输入 `POSITION` 经保名存活;ps_texture `SV_Position` 输入(未读)消除(损耗③);`SV_Target` 名恒保留。

**对照小结(measured)**:保名配置消除了**顶点阶段输入**的损耗①(`POSITION/NORMAL` 真达);
varying 名(损耗①余项)、寄存器重排(损耗②)、未用输入消除(损耗③)在保名后**仍在**——它们与"用户语义名保真"正交。

## 4. 三类损耗逐项事实 + 拟分类(摆事实 + 拟判;规范性裁断留 agent，P-13)

> **硬规则 1 / P-13**:②③的契约线归属、以及"是否构成 P-01 规范性违背/边界"**只摆事实 + 拟判**,
> 规范性裁断**留 agent**。下表"拟分类"列为 AI 拟判(供 agent 参),"裁断"列标明归属。

| 损耗 | 事实(measured) | 拟分类(AI 拟判，非裁决) | 裁断归属 |
|---|---|---|---|
| **①a 顶点输入语义名** | `POSITION/NORMAL` 默认降级 `TEXCOORD#`，保名配置后端到端存活(§3 vs_sig/vs_passthrough ISG1) | **经配置可消除（保真）**——IA 顶点缓冲按名绑定的外部可见契约，已 measured 消除 | 事实已闭合（measured 可消除） |
| **①b varying 语义名** | vs-out→ps-in 的 `NORMAL/TEXCOORD0/COLOR` 恒 `TEXCOORD#`，spirv-cross 无输出语义保名 flag | **拟判契约线下**：RXS-0155 阶段间接口契约 = 类型级字段/类型/插值匹配，**非 HLSL 语义串**；两阶段同经 B 链产 → 双侧 location 对齐 `TEXCOORD#` → 链接正确；用户从未把 varying 名声明为外部绑定契约 | **agent 裁断**（是否落契约线下） |
| **② 寄存器/顺序重排** | SV 元素被推到签名末位、register 重编号（vs_sig ISG1 `SV_VertexID` reg 0→3、OSG1 `SV_Position` 0→3），保名后仍重排 | **拟判契约线下**：RXS-0154/0155 不约束寄存器/二进制布局；属 RFC-0004 §4.6(a) 签名 ABI 二进制布局**禁区**（dxc/D3D12 conformance 既定算法，用户不控制，Rurix 不定义） | **agent 裁断**（契约线下 + 禁区归属） |
| **③ 未用 SV_Position 输入消除** | ps_sig ISG1 elemcount direct 4→B 3、ps_texture 2→1，body 未读的 `SV_Position` **输入**元素被 SPIR-V 优化往返消除；保名后仍消除（与名保真正交） | **拟判标准死 I/O 消除**：被消除的是 body 未使用的输入元素，对用户意图不可见；输出 `SV_Position` 与所有被使用 SV 输入/输出恒存活 | **agent 裁断**（是否属可接受 DCE） |

**关键观察(供 agent 判 strict-only 达标)**:消除后能否零静默降级，取决于是否存在**同时满足**三条件的损耗——
*(i) 对用户可见/契约线上* + *(ii) 不可保真* + *(iii) 只能静默丢*。本轮 measured:
- 唯一对用户外部可见的命名契约（①a 顶点输入名）**可保真**（measured 消除）；
- 余下损耗（①b varying / ② 布局 / ③ 未用输入）**拟判落契约线下或标准 DCE**（待 agent 裁断）。
- 即:**若 agent 拟判成立，不存在"可见 + 不可保真 + 静默"的损耗** → B 可零静默降级达标 P-01，无需例外。

## 5. strict-only 失败模式(设计级论证，不落 codegen 实现)

> 本节是**设计面论证 + 可行性**，**不落 codegen 实现**（硬规则 5/7：本轮取证，不改 src/spec/codegen）。
> 论证对象:对**真留不住**的元素，能否在 MIR→SPIR-V 降级侧**显式检测 → 6xxx 结构化编译错误**（而非静默丢）。

**前提澄清**:本 spike 用 `dxc -spirv` 作 SPIR-V producer（作者 HLSL → SPIR-V），故名保真受 dxc 默认行为 +
spirv-cross flag 双重约束。**真 Rurix 路径**（RFC-0004 §4.2(a)）的 producer 是 **Rurix 自有 MIR→SPIR-V 降级**——
Rurix 控制 lowering 的全部决定，约束面与本 spike 不同。下列论证基于真 Rurix 路径。

**设计论证(三档处置，覆盖全部损耗)**:

1. **可保真元素 → by-construction 保名（无需报错）**。Rurix MIR→SPIR-V 拥有用户 I/O 的全部语义信息
   （RXS-0154 `#[builtin]`/`#[interpolate]` + 字段名），可对所有用户命名 I/O **by-construction** emit
   `UserSemantic` 装饰 + `OpName`，并在 SPIR-V→HLSL 段驱动 `--set-hlsl-named-vertex-input-semantic` 等保名。
   本轮 measured 证此机制对顶点输入有效（①a 已消除）。

2. **契约线下元素 → 不构成静默降级（无需报错）**。①b varying 名 / ② 寄存器布局属语言契约线下
   （§4 拟判，待 agent 裁）——RXS-0155 接口契约是类型级、§4.6(a) 布局是 ABI 禁区。语言层未承诺这些表层名/布局，
   故其工具层重写**不是语言层 lowering 失败**，不触发 P-01。③ 未用输入 DCE 同理（用户意图不可见）。

3. **真留不住的契约线元素 → 显式检测 → 6xxx 结构化错（可行性）**。**若** agent 裁某保真损耗落在契约线**上**
   且回译路径**可证无法保真**，则 strict-only 要求显式拦截而非静默丢。可行实现锚点（设计面，不落 codegen）:
   - **译后签名校验器**:B 链产 DXIL 后，解 ISG1/OSG1（本 spike `parse_signature_part` 已 measured 可解），
     与 MIR I/O 意图（语义名集合 / 系统值 / 被使用元素）做结构对照；分歧（如某契约线命名元素丢失/改名）→
     发 **6xxx codegen 错误**（承 RFC-0003 §5 RX6007~6009 段，只追加；真实可达类别随实现 PR 分配）。
   - **降级侧前置检测**:MIR→SPIR-V 时若某用户命名 I/O 在目标回译路径无保名通道（如 varying 若被 agent 裁为契约线上），
     降级即拒 → 6xxx，而非产出后再发现。
   - 二者皆为**显式、可诊断**（P-01 要求），对齐 RFC-0004 §3"任一分支降级失败 = 结构化编译错误，无静默回退"。

**可行性结论(measured 支撑)**:签名 part 可程序化解析（本 spike ×N 稳定解出 elemcount + 名 + 系统值 + register）→
译后校验器有可靠输入；6xxx 段已存在（RFC-0003 §5）→ 错误码载体就位。故"显式检测 → 6xxx"在工程上可行，
**不依赖任何 P-01 例外**。具体落地（校验器位置、6xxx 类别、检测粒度）随 agent 批准 RFC-0004 后的实现 PR（硬规则 7）。

## 6. 确定性 + validator(保名配置，各 ×25；validator accept ≠ 名保真，已联读 §3 part dump）

保名 B 链产物 + 直产基线各 IDxcValidator（round-7 dxcompiler.dll + dxil.dll）+ dxv.exe 各 ×25（真实直方图输出）:

| 语料 | b_keep IDxcValidator ×25 | b_keep dxv.exe ×25 | direct IDxcValidator ×25 | b_keep 确定性 ×25 |
|---|---|---|---|---|
| vs_sig | 25/25 accept `{0x0:25}` | 25/25 succeeded | 25/25 accept `{0x0:25}` | ✓ 1 unique SHA256 |
| ps_sig | 25/25 accept `{0x0:25}` | 25/25 succeeded | 25/25 accept `{0x0:25}` | ✓ 1 unique SHA256 |
| vs_passthrough | 25/25 accept `{0x0:25}` | 25/25 succeeded | 25/25 accept `{0x0:25}` | ✓ 1 unique SHA256 |
| ps_texture | 25/25 accept `{0x0:25}` | 25/25 succeeded | 25/25 accept `{0x0:25}` | ✓ 1 unique SHA256 |

- **确定性**:保名 B 全链（`dxc -spirv -fspv-reflect` → spirv-val → spirv-cross + 保名 flag → dxc）同输入 ×25，
  最终 DXIL 容器 SHA256 各语料均 **1 个 unique 值**（deterministic）。保名 flag 引入不破坏确定性。
- **validator accept ≠ 名保真**:本节 accept 必与 §3 签名 part dump 联读——`b_keep` accept 时 ISG1 顶点输入名
  确为 `POSITION/NORMAL`（非空、非降级），区别于"accept 但名已降级"的假绿。

## 7. 复现清单

```powershell
# 1. B 链工具(Vulkan SDK 1.3.296.0 Bin):dxc / spirv-cross / spirv-val / spirv-dis 置 PATH
# 2. 签名 validator(round-7 套件,隔离不入库):
$env:RURIX_DXC_NEW_DIR="H:\dxc-round7\extracted\bin\x64"   # dxc/dxcompiler.dll/dxil.dll/dxv.exe 1.9.2602.24
$env:RURIX_SIG_N="25"
# 3. 跑 strict-only 名保真探针(人读 JSON):
py -3 spike\dxil-path-probe\probe_b_strict_only.py
# 4. 写证据 JSON:
$env:RURIX_EMIT_SIG="1"; $env:RURIX_SIG_DATE="20260625"; py -3 spike\dxil-path-probe\probe_b_strict_only.py
# 5. schema 校验:
py -3 ci\check_schemas.py    # PASS
```

流程:每语料跑 ① dxc 直产基线 ② 默认 B 链(TEXCOORD 降级基线)③ 保名 B 链
（`-fspv-reflect` + `spirv_reflect.py` 自动导出 `--set-hlsl-named-vertex-input-semantic`）→ ④ 解 ISG1/OSG1 签名 part
（elemcount + 名 + 系统值 + register）→ ⑤ b_default/b_keep vs direct 保真对照 → ⑥ IDxcValidator + dxv.exe 各 ×25
→ ⑦ 保名 B 全链 ×25 容器 SHA256 确定性。手动名保真单点复核(spirv-dis + 全链到 DXIL)见 §2/§3 真实输出。

## 8. 判定逻辑落点(产证据，不下裁决)

任务给定三分支,本轮证据落点:

- ✅ **①顶点输入名经配置可消除(measured)** + **①varying/②/③拟判落契约线下或标准 DCE(规范性裁断留 agent)**
  → **若 agent 拟判成立,B 可零静默降级达标 P-01,无需例外** → 解锁 B 图形（RFC-0004 §4.4 由"边界/例外声明"
  写成"经名保真达 strict-only",**agent 落笔批准**）。
- 反向条件(本轮 measured **未**出现):若有损耗"既不可消除、又在契约线上、又只能静默"→ B as-is 不达未改的 P-01
  → 须 escalate。本轮唯一外部可见命名契约（顶点输入名）已 measured 可消除,余项拟判契约线下,故**未触发**该反向分支。
- 工具链/选项**无受阻**:dxc/spirv-cross/spirv-val/dxv 全到位真跑,名保真经签名 part dump 实证,无 blocked、无杜撰。

**对 agent 的取证落点**:B 链经名保持配置（非默认参数），用户外部可见的命名契约（顶点输入语义名）端到端保真
（measured，签名 part dump 实证），validator ×25 真 accept + ×25 确定。余下三类损耗（varying 名/寄存器布局/未用输入）
拟判落语言契约线下或标准 DCE。**这是 B 路名保真能力的工具链事实**,**不构成 P-01 规范性裁断**（留 agent，P-13）、
**不构成 A/B/混合架构裁决**（留 agent，硬规则 1），亦**不**等于 Rurix MIR→SPIR-V 实现或 device 真跑 golden。

## 9. 约束遵守声明

- **硬规则 1 / P-13**:未裁 A/B/混合架构、未裁 P-01 规范线、未代签 G-G2-2;②③契约线归属 + 是否构成 P-01 违背/边界
  **只摆事实 + 拟判,规范性裁断留 agent**。D-131 维持现状不动(混合尚待 agent 批 #100)。
- **硬规则 3/4**:measured-first / blocked-honest;保名/elemcount/SV 名/register/SHA256/status 全来自命令真实输出
  （IDxcValidator + dxv.exe 各 ×25）;**validator accept 必联读签名 part dump**（§3/§6），不以 accept 充名保真。
- **禁区(硬规则 5)**:未在仓库落签名 ABI 二进制布局内容;只读解析签名 part,不定义/不修改布局;§5 为设计面论证,不落 codegen。
- 不改 src/spec/codegen;不动 D-131 / D-205 pin / toolchain.rs;不落 codegen / 不创建 spec 条款 / 不造错误码 /
  不入 golden / 不登 spike_gating;零新 RXS;不签 / 不翻 G-G2-2;未向 llvm-project 提交。
- **evidence/ 不可篡改门**:round-1~8 / slice3 / dxil_b_graphics_sig / dxil_a_graphics_sig_effort 既有 evidence 全部
  byte-unchanged;仅新增 `dxil_b_strict_only_20260625.json` + 本报告。
- **隔离纪律**:dxc / SPIRV-Cross / glslang / dxil.dll 及中间 SPIR-V/HLSL/DXIL 全部隔离不入库
  （version / SHA256 写进证据 JSON 与本报告 §1）;新增 schema + 探针(`probe_b_strict_only.py` + `spirv_reflect.py`)
  隔离于 spike/ 与 milestones/g2,标 `// SPIKE(RD-014)`。
- **LF 字节精确**:证据 JSON / schema / 本报告经二进制 + 显式 LF 写出（`\r\n`→`\n`,尾 `0x0a`）。
- Provenance:`Assisted-by: kiro:claude-opus-4-8`;影响范围 = 新增 evidence(JSON+报告)/schema/探针 + deferred.json
  RD-014 history + revision_log v1.31 追加 + check_schemas 接线;验证方式见 §3/§4/§6/§7 + `ci/check_schemas.py` PASS。
