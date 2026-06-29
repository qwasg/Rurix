# B 路图形签名能力取证 — SPIR-V→DXIL 能否产带真实 SV 签名的图形 DXIL

> 类型:**纯 spike 取证报告**(Windows-only)。不裁 A/B/混合架构(硬规则 1,裁决权属 agent)、
> 不改 src/spec/codegen、不动 D-131(维持 A=C 决策载体不动)、不动 D-205 pin、不签/不翻 G-G2-2。
> 承 slice3(evidence/dxil_slice3_rxs0159_sig_disasm_round8.md)证伪 A 路图形签名 elemcount=0 假绿、
> round-2(evidence/dxil_path_spike_report_round2.md)B 端到端确定性。round-1~8 既有 evidence 全部 byte-unchanged,
> 本报告与 `dxil_b_graphics_sig_20260625.json` 为新增。
> Provenance:`Assisted-by: kiro:claude-opus-4-8`(agent 自主记录机器可核对事实,非代决、非代签)。
> 纪律:measured-first / blocked-honest——所有数字来自命令真实输出(IDxcValidator + dxv.exe 各 ×25);
> **validator accept 不等于签名达产物——必看签名 part dump**(吸取 slice3 假绿教训)。

---

## 0. 核心命题与结论(TL;DR)

**命题**:agent 正评估混合架构(compute=A / 图形=B)。A 路图形签名经 slice3 证伪不可达
(LLVM DirectX 后端 `addSignature()` 对图形无条件写空签名,ISG1/OSG1 **elemcount=0**,
validator accept 是空签名结构合法的假绿;填充需 Register/Mask = §9 Q-Builtin FFI ABI 禁区)。
本轮取证 B 路(MIR→SPIR-V→SPIRV-Cross→HLSL→dxc→DXIL)的图形签名能力,作为 agent 裁混合架构的输入。

**结论(measured)**:

- **B 图形签名实测可行**:B 全链产物 ISG1/OSG1 **全部 elemcount>0**,SV 系统值端到端存活——
  `SV_VertexID`(vs 入)/ `SV_Position`(vs 出)/ `SV_Target ×2 MRT`(ps 出)/ `SV_IsFrontFace`(ps 入),
  `system_value` 命名与 dxc 直产基线一致。与 A 路 **elemcount=0** 苹果对苹果:B 是 SV 真达产物,**非假绿**。
- **validator 真验证**:wrapper 正反向控制通过(自产 accept `0x0` / 翻尾 64 字节 reject `0x80aa0009`);
  B 产物 + 直产基线各 **IDxcValidator ×25 = 25/25 accept(`{0x0:25}`)+ dxv.exe ×25 = 25/25 `Validation succeeded.`**。
  B accept 非假绿:签名 part 非空且含真实 SV(对照 slice3 A 路 accept 但 elemcount=0)。
- **确定性**:B 全链同输入 ×25,容器 SHA256 各样例一致(deterministic)。
- **strict-only(P-01)保真子轴 = measured 但非完美**(详见 §5):用户自定义语义名经 SPIR-V 往返
  **静默降级**为通用 `TEXCOORD`、寄存器/顺序重排、未被 body 使用的 `SV_Position` **输入**元素被消除。
  即 B 图形签名可达,但转译链对用户语义名/寄存器布局/未用输入元素**非保真**(混合架构图形=B 需权衡此代价)。

裁决归属 agent;本报告只摆事实 + 复现清单。

---

## 1. 工具链(隔离不入库;version + SHA256 写进证据)

| 角色 | 工具 | 版本 | SHA256 |
|---|---|---|---|
| SPIR-V producer | dxc.exe -spirv(Vulkan SDK 1.3.296.0) | `1.8.0.4739 (d9a5e97d0)` | `8B1321A4...F65D037` |
| SPIR-V 合规 | spirv-val.exe(SPIRV-Tools) | `v2024.4` | `ABCB0DA8...4B4641` |
| SPIR-V→HLSL | spirv-cross.exe | `vulkan-sdk-1.3.290.0-44-g65d73934`(2024-10-04) | `7DE34891...1F8AD5` |
| HLSL→DXIL + validator | dxc.exe(round-7 套件) | `1.9.2602.24 (d355aa836)` | `1367FD29...476EA4` |
| 签名 validator DLL | dxcompiler.dll | `1.9.2602.24` | `9B5E10ED...F2AEC6` |
| 签名 validator | dxil.dll | `1.9.2602.24` | `CBCFE883...3A169DF` |
| CLI validator | dxv.exe | `1.9.2602.24` | `F26242EF...5979F61` |

- B 链最终 HLSL→DXIL 与直产基线均用 **同一 round-7 dxc 1.9.2602.24**(签名 validator 年代编译器),
  apple-to-apple:仅 B 在前端多一段 SPIR-V 往返(dxc -spirv → spirv-cross),直产=dxc 原生上界。
- glslang 15.0.0 在位(SPIR-V producer 备选,本轮用 dxc -spirv 作 producer)。
- 全部隔离于 Vulkan SDK / `H:\dxc-round7`,不入库;中间 SPIR-V/HLSL/DXIL 临时目录产物不入库。

## 2. 图形语料(带真实 I/O 签名,承 RXS-0154/0159 SV 映射)

| 语料 | stage | 入口签名意图 | 出口签名意图 | 备注 |
|---|---|---|---|---|
| `vs_sig`(新增,// SPIKE) | vertex | SV_VertexID + POSITION/NORMAL/TEXCOORD0 | SV_Position + NORMAL/TEXCOORD0/COLOR | 富 SV 签名 |
| `ps_sig`(新增,// SPIKE) | pixel | SV_Position + NORMAL/TEXCOORD0 + SV_IsFrontFace | SV_Target0 + SV_Target1(MRT) | 多渲染目标 |
| `vs_passthrough`(复用 round-2) | vertex | POSITION + TEXCOORD0 | SV_Position + TEXCOORD0 | round-2 corpus |
| `ps_texture`(复用 round-2) | pixel | SV_Position + TEXCOORD0 | SV_Target | Texture2D+Sampler |

---

## 3. 签名 part dump(本轮决定性)——ISG1/OSG1 elemcount + SV 语义(对照 A elemcount=0)

解 DXContainer 的 ISG1(输入签名)/ OSG1(输出签名)part:头 `u32 ParamCount` + 各
`DxilProgramSignatureElement`(32B:Stream/SemanticName_off/SemanticIndex/SystemValue/CompType/Register/Mask...)。

| 语料 | part | direct elemcount | B elemcount | direct SV(system_value) | B SV(system_value) | SV 端到端存活 |
|---|---|---|---|---|---|---|
| vs_sig | ISG1 | 4 | 4 | **SV_VertexID** | **SV_VertexID** | ✓ |
| vs_sig | OSG1 | 4 | 4 | **SV_Position** | **SV_Position** | ✓ |
| ps_sig | ISG1 | 4 | 3 | SV_Position, **SV_IsFrontFace** | **SV_IsFrontFace** | SV_IsFrontFace ✓ / SV_Position 输入丢 |
| ps_sig | OSG1 | 2 | 2 | **SV_Target ×2** | **SV_Target ×2** | ✓(MRT) |
| vs_passthrough | ISG1 | 2 | 2 | (无 SV) | (无 SV) | ✓ |
| vs_passthrough | OSG1 | 2 | 2 | **SV_Position** | **SV_Position** | ✓ |
| ps_texture | ISG1 | 2 | 1 | SV_Position | (无 SV) | SV_Position 输入丢 |
| ps_texture | OSG1 | 1 | 1 | **SV_Target** | **SV_Target** | ✓ |

**苹果对苹果对照(B vs round-8 A)**:

```
A 路(slice3/round-8,LLVM DirectX 后端):
  vs_io  ISG1 elemcount 0   OSG1 elemcount 0   (无任何 SV 元素,空签名假绿)
  ps_io  ISG1 elemcount 0   OSG1 elemcount 0
B 路(本轮,SPIR-V→dxc):
  vs_sig ISG1 elemcount 4   OSG1 elemcount 4   (SV_VertexID / SV_Position 真达)
  ps_sig ISG1 elemcount 3   OSG1 elemcount 2   (SV_IsFrontFace / SV_Target ×2 真达)
```

→ **B 产物签名 part 含真实 SV 系统值(elemcount>0)**;A 产物 elemcount=0 全空。
输出侧 `SV_Position`(vertex)/ `SV_Target`(pixel,含 MRT)在 B 全链**无条件存活**。

## 4. validator 真验证(各 ×25,B + 直产基线)

| 实验 | IDxcValidator(round-7 dxcompiler.dll+dxil.dll) | dxv.exe CLI |
|---|---|---|
| wrapper 正向(dxc 自产 accept) | accept,status `0x0` | — |
| wrapper 反向(翻尾 64 字节) | reject,status `0x80aa0009` | — |
| vs_sig B / direct | 25/25 accept `{0x0:25}` | 25/25 succeeded |
| ps_sig B / direct | 25/25 accept `{0x0:25}` | 25/25 succeeded |
| vs_passthrough B / direct | 25/25 accept `{0x0:25}` | 25/25 succeeded |
| ps_texture B / direct | 25/25 accept `{0x0:25}` | 25/25 succeeded |

- `wrapper_validated=True`(正向全 accept、反向全 reject)→ harness 在 round-7 DLL 下正确。
- **B accept 非假绿**:与 §3 签名 dump 联读——B 容器 accept 时签名 part 非空且含真实 SV
  (对照 slice3 A 路 accept 但 elemcount=0 的假绿陷阱)。

---

## 5. strict-only(P-01)保真子轴 —— 入口意图 vs 出口签名(measured 非完美)

对照转译链入口意图签名(dxc 直产基线,= 作者 HLSL 意图)vs 出口 DXIL 签名(B 链),
判 SPIRV-Cross→dxc 是否静默丢/改/降级签名元素。**实测三类非保真**:

1. **用户自定义语义名静默降级**:`POSITION` / `NORMAL` / `TEXCOORD0` / `COLOR` 经 SPIR-V 往返
   全部被 spirv-cross 重写为通用 `TEXCOORD0` / `TEXCOORD1` / `TEXCOORD2`(SPIR-V 只携 location 编号,
   不携 HLSL 语义串)。例 vs_sig OSG1 意图 `[SV_Position0, NORMAL0, TEXCOORD0, COLOR0]` →
   出口 `[TEXCOORD0, TEXCOORD1, TEXCOORD2, SV_Position0]`。`SV_*` 系统值语义名保留(SV_Target0/1 亦保留)。
2. **寄存器/顺序重排**:SV 元素被推到签名末位、register 重编号(如 vs_sig 入 SV_VertexID register 0→3,
   出 SV_Position register 0→3)。系统值类型不变,但 register/顺序布局与意图不一致。
3. **未用 SV_Position 输入元素被消除**:ps_sig / ps_texture 的 `SV_Position` **输入**(body 内未读)经
   SPIR-V 优化往返被消除——ISG1 elemcount direct 4/2 → B 3/1。**诚实边界**:被消除的是 body 未使用的
   输入元素(可视作合法死输入消除);**输出** `SV_Position` 与所有被使用的 SV 输入/输出均存活。

**保真小结**:B 图形签名**系统值层**(SV_* 是否存活 + system_value 类型)保真良好——所有被使用的
SV 端到端存活;但**用户语义名层 + 寄存器布局层 + 未用输入元素**非保真。strict-only(P-01)语义级
运行期行为等价(无静默降级/回退)须 device 真跑 golden,超出本取证 spike 范围。

## 6. 确定性

B 全链(dxc -spirv → spirv-val → spirv-cross → dxc → DXIL)同输入 ×25,最终 DXIL 容器 SHA256:

| 语料 | unique SHA256 数 | consistent |
|---|---|---|
| vs_sig | 1 | ✓ deterministic |
| ps_sig | 1 | ✓ deterministic |
| vs_passthrough | 1 | ✓ deterministic |
| ps_texture | 1 | ✓ deterministic |

## 7. 复现清单

```powershell
# 1. B 链工具(Vulkan SDK 1.3.296.0 Bin):dxc / spirv-cross / spirv-val 置 PATH
# 2. 签名 validator(round-7 套件,隔离不入库):
$env:RURIX_DXC_NEW_DIR="H:\dxc-round7\extracted\bin\x64"   # dxc/dxcompiler.dll/dxil.dll/dxv.exe 1.9.2602.24
$env:RURIX_SIG_N="25"
# 3. 跑探针(人读 JSON):
py -3 spike\dxil-path-probe\probe_b_graphics_sig.py
# 4. 写证据 JSON:
$env:RURIX_EMIT_SIG="1"; $env:RURIX_SIG_DATE="20260625"; py -3 spike\dxil-path-probe\probe_b_graphics_sig.py
# 5. schema 校验:
py -3 ci\check_schemas.py    # PASS
```

流程:每语料跑 ① dxc 直产基线 ② B 全链 → ③ 解 ISG1/OSG1 签名 part(elemcount+SV 语义)
→ ④ IDxcValidator + dxv.exe 各 ×25 → ⑤ 入口意图 vs 出口保真对照 → ⑥ B 全链 ×25 容器 SHA256 确定性。

---

## 8. 判定逻辑落点(产证据,不下裁决)

任务给定三分支,本轮落在**第一分支**:

- ✅ **B accept + ISG1/OSG1 elemcount>0 且 SV 语义正确存活 + 确定性 + (系统值层)无降级**
  → **B 图形签名路径实测可行**(SV 真达产物)→ agent 裁混合架构(图形=B)有硬证据。
- 同时如实记**保真代价**(§5):用户语义名/寄存器布局/未用输入元素经 SPIR-V 往返非保真——
  混合架构图形=B 需权衡此代价(若依赖 reflection 按原始语义名绑定资源,需在 B 侧补语义名保持映射)。
- 工具链全部到位并真跑,无 blocked。

**对 agent 的取证落点**:与 slice3 苹果对苹果——A 路图形签名当前不可达(elemcount=0,填充耦合 FFI ABI 禁区 +
上游 #90504),B 路图形签名**实测可达**(SV 真达产物 + validator 真 accept + 确定),代价是转译链的
语义名/布局保真损耗。这是 B 路图形签名能力的工具链事实,**不构成 A/B/混合架构裁决**(裁决权属 agent),
亦**不**等于 Rurix MIR→DXIL/SPIR-V 实现或 device 真跑 golden。

## 9. 约束遵守声明

- **硬规则 1**:未裁 A/B/混合架构、未代签 G-G2-2;结论只到「B 图形签名可行 + 保真实况」。D-131 维持现状不动。
- **硬规则 3/4**:measured-first / blocked-honest;签名 elemcount/SV 名/SHA256/status 全来自命令真实输出
  (IDxcValidator + dxv.exe 各 ×25);**validator accept 必联读签名 part dump**,不以 accept 充当签名达产物。
- **禁区(硬规则 5)**:未碰 DXIL UB 边界 / 内存模型 / FFI ABI 二进制布局;只读解析签名 part,不定义/不修改布局。
- 不改 src/spec/codegen;不动 D-131 / D-205 pin / toolchain.rs;不落 codegen / 不创建 spec 条款 /
  不造错误码 / 不入 golden / 不登 spike_gating;零新 RXS;不签 / 不翻 G-G2-2。
- **evidence/ 不可篡改门**:round-1~8 及 slice3 既有 evidence 全部 byte-unchanged;仅新增
  `dxil_b_graphics_sig_20260625.json` + 本报告。
- **隔离纪律**:dxc / SPIRV-Cross / glslang / dxil.dll 及中间 SPIR-V/HLSL/DXIL 全部隔离不入库
  (version / SHA256 写进证据 JSON 与本报告 §1);新增 schema + 探针 + corpus 隔离于 spike/ 与 milestones/g2。
- **LF 字节精确**:证据 JSON / schema 经二进制 + 显式 LF 写出(`\r\n`→`\n`,尾 `0x0a`),禁 Python 文本模式。
- Provenance:`Assisted-by: kiro:claude-opus-4-8`;影响范围 = 新增 evidence/schema/探针/corpus + deferred.json
  RD-010 history/revision_log 追加 + check_schemas 接线;验证方式见 §3/§4/§6/§7 + ci/check_schemas.py PASS。
