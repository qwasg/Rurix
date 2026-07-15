# DXIL A 路 validator 互操作复验 — Round-7 取证报告

> 类型:**纯 spike 取证报告**(Windows-only)。不裁 A/B(硬规则 1,裁决权属 agent)、不落 codegen、不创建 spec 条款、不造错误码、不入 golden、不登 spike_gating、不签/不翻 G-G2-2;D-131 维持 C。
> 承 round-1~6;round-1~6 既有 evidence/ 文件全部 byte-unchanged,本报告与 `dxil_path_spike_20260624_r7.json` 为**新增**。
> Provenance:`Assisted-by: kiro:claude-opus-4-8`(agent 自主记录机器可核对事实,非代决、非代签)。
> 纪律:measured-first / blocked-honest——所有数字来自命令真实输出(IDxcValidator + dxv.exe 各 ×25),探不到如实 blocked。

---

## 0. 核心命题与结论(TL;DR)

**命题**:用与 LLVM 22/23 同年代(2026)的更新 DXC release(带独立 dxil.dll 签名 validator),重验 round-6 已落盘的合法 llc DXContainer,裁开 round-6 留下的 Bug 2 归因悬案(「LLVM 过度 emit 新版 PSV」vs「dxc 1.8 太旧不识新 PSV」)。

**结论(measured)**:
- 新 validator(dxc/dxil.dll/dxv.exe **1.9.2602.24**,d355aa836,2026 年代)对 round-6 合法 llc 容器(llc22 1804B / llc23 1936B)**仍 reject**,IDxcValidator 0/25 accept + dxv.exe 0/25 accept,具名拒因 `0x80aa0013`:`PSVRuntimeInfoSize 'PSV0' part(52) vs DXIL module(24)`——与 dxc 1.8.0.4739 **完全相同**。
- **决定性子轴**:新 dxc 1.9 自产 cs_6_0 容器的 PSV0 RuntimeInfoSize **也是 52**,且被新 validator **accept**;而 llc 容器 PSV0 **同为 52** 却被 **reject**。→ 同一 2026 validator 接受 52 字节 PSV、拒绝 llc 的 52 字节 PSV,**排除「dxc 太旧不识新 PSV」假说**。
- **归因(established)**:拒因是 llc 容器的 PSV0 part(52)与其**自身 DXIL 模块**推得的期望值(24)**内部不一致**,即 LLVM DirectX 后端 emit 的 PSV0 不合规——**上游 PSV 兼容性/一致性 bug**,非 validator 版本 gap。Bug 2 归因由 round-6 的 **C(UNRESOLVED)** 收紧为 **established 上游 emit 不合规**。
- **A 路 validator 互操作 gap:未闭合**——当前 D-205 pin(及 round-5 测的最新 LLVM main)产物即便经最新 2026 签名 validator 仍被拒;A 在工具链层的 validator 互操作仍不通。(注:A 工具链可行性 ≠ Rurix MIR→DXIL 实现 ≠ device 真跑 golden;G-G2-2 仍 open。)

裁决归属 agent;本报告只摆事实 + 复现清单。

---

## 1. 取更新 DXC(隔离,不入库)

从 microsoft/DirectXShaderCompiler 官方 release 取**显著新于 1.8.0.4739** 且**自带 dxil.dll(签名 validator)**的版本,下载到仓库外隔离目录 `H:\dxc-round7`(不入库)。

| 项 | 值 |
|---|---|
| release tag | **v1.9.2602.24**(published 2026-06-03;最新 stable) |
| asset | `dxc_2026_05_27.zip` |
| zip SHA256 | `CF658AACF070D3045E31B8F1F8A696C2945F37C1095019481EF7C513368DB3B4` |
| zip 大小 | 27108038 B |
| dxc 版本字符串 | `dxcompiler.dll: 1.9(5191-d355aa83)(1.9.2602.24) - 1.9.2602.24 (d355aa836)` |

解压 `bin/x64/` 二进制(SHA256):

| 文件 | 大小 | SHA256 |
|---|---|---|
| dxc.exe | 1127736 | `1367FD29D0EBBA5BF10D1041A9DEFF85396D30090B3651D872EC65D11A476EA4` |
| dxcompiler.dll | 17987384 | `9B5E10ED756C461B4EC2C83A99F1D6ACE20E97826E9C0B0E966B7B1CD6F2AEC6` |
| **dxil.dll** | 1503072 | `CBCFE883A09FD0CA1F98ABDF3A9553B560895E3283A136DA82A8381253A169DF` |
| dxv.exe | 295224 | `F26242EFB0197FFEFA51EB5CB14603207D1D81FA547394020A5767BF65979F61` |

- **dxil.dll 在位**(round-6 在位的 Vulkan SDK dxc 1.8.0.4739 **dxil.dll MISSING**,只能用 dxcompiler 内置开源 validator)。本轮经 dxcompiler.dll + 同目录 dxil.dll → **签名 validator**;另有独立 **dxv.exe** 签名 validator 作 CLI 交叉验证。
- 版本跨度:dxc 1.8.0.4739(约 2023)→ 1.9.2602.24(2026-05),与 LLVM 22/23(2025/2026)同年代,满足「同年代更新 validator」要求。

## 2. 合法 llc 容器(复用 round-6,byte-unchanged)

复用 round-6 已落盘的合法 emit 容器(均带 hlsl 入口属性,round-6 阶段 3 证 obj 100/100 稳定,非不完整输入):

| 容器 | 来源 | 大小 | SHA256 |
|---|---|---|---|
| llc23 | `H:\llvm-audit-round6\official_cs.obj`(= off_cs/run-*.obj) | 1936 | `76A3D75ABD2368C05C37848B78566F19A8C457D35052D754D34907A917D7AAD0` |
| llc22 | `H:\llvm-audit-round6\off_cs22\run-0001.obj` | 1804 | `6EA297A8B72DBB5495283E088E29FC03A1F81C5A1BC94A1221223FB6BAC975DC` |
| 输入 | `H:\llvm-audit-round6\official_cs.ll` | 289 | `8FA89702D57840BE106C1BBAA56C5E6C9FDDE71208D3ABF2648AA9EB4E695A1A` |

- 未重跑 llc(blocked-honest 角度:本轮焦点是 validator 互操作,复用 round-6 合法产物即可,且避免 destabilize);llc 容器只读引用。

## 3. wrapper 正反向控制(换新 DLL 后必重做)

复用 `spike/dxil-path-probe/dxil_validator.py` 的 IDxcValidator ctypes harness,但加载**新 dxcompiler.dll**(同目录 dxil.dll → 签名 validator);并用独立 dxv.exe 交叉验。换 DLL 后先正反向控制,确认 wrapper 仍正确再信任结果。

| 实验 | IDxcValidator(新 dxcompiler.dll) | dxv.exe CLI | 判定 |
|---|---|---|---|
| 正向:新 dxc 自产 cs_6_0 容器 ×25 | 25/25 accept,status `0x0` | 25/25 accept(`Validation succeeded.`) | ✓ wrapper 正确 |
| 反向:翻字节损坏容器 ×25 | 0/25 accept,status `0x80aa000d`「Validation failed.」 | 25/25 reject(`error code 0x80070459`) | ✓ wrapper 正确 |

→ **wrapper_validated = True**(新 DLL 下正向全 accept、反向全 reject)。排除「wrapper 在新 DLL 下失灵」。

## 4. 真验证:合法 llc 容器 × 新 validator(各 ×25)

| 容器 | IDxcValidator | status 直方图 | dxv.exe CLI | 拒因原文 |
|---|---|---|---|---|
| llc23 1936B | 0/25 accept | `{0x80aa0013: 25}` | 0/25 accept,25/25 reject | `DXIL container mismatch for 'PSVRuntimeInfoSize' between 'PSV0' part:('52') and DXIL module:('24')` |
| llc22 1804B | 0/25 accept | `{0x80aa0013: 25}` | 0/25 accept,25/25 reject | 同上(52 vs 24) |

**对照(§3 正向)**:新 dxc 自产容器 25/25 accept(0x0)。

- 确定性:llc22/llc23 各 25/25 一致 reject,status 单一 `0x80aa0013`,拒因原文逐次一致(量化确定性,非单发假象)。
- 与 round-6(dxc 1.8 内置 validator)**完全同码同因**(0x80aa0013 / PSV0 52 vs module 24)——升级到带签名能力的 2026 validator 未改变拒绝结果。

## 5. DXIL/PSV 版本子轴(裁开 Bug 2)

| 容器 | DXIL part 版本 | program_version | PSV0 RuntimeInfoSize | part 集 / 签名 | validator |
|---|---|---|---|---|---|
| 新 dxc 1.9 自产 | 0x100 / SM6.0 / kind5 | raw 327776 | **52** | SFI0,ISG1,OSG1,PSV0,STAT,HASH,DXIL / signed | **accept** |
| llc23 | 0x100 / SM6.0 / kind5 | raw 327776 | **52** | DXIL,SFI0,HASH,ISG1,OSG1,PSV0(缺 STAT)/ unsigned | **reject** |
| llc22 | 0x100 / SM6.0 / kind5 | raw 327776 | **52** | 同 llc23(缺 STAT)/ unsigned | **reject** |

**决定性裁开**:
- 三者 DXIL 版本(0x100)/ SM(6.0)/ program_version(327776)**完全相同**——非 DXIL 版本 gap(承 round-5)。
- 新 dxc 自产容器 PSV0 RuntimeInfoSize = **52**,被新 validator **accept**;llc 容器 PSV0 同为 **52** 却被 **reject**,拒因「part 52 vs module 24」。
- ⇒ 2026 validator **认识并接受 52 字节 PSV**(它自己就产 52)。llc 被拒**不是**因为 validator 太旧不识 52 字节新 PSV(那样新 dxc 自产的 52 也该被拒)。
- ⇒ 拒因是 llc 容器**内部不一致**:PSV0 part 实际写出 52 字节,但其 DXIL 模块所编码的元数据使 validator 推得期望 RuntimeInfoSize=24,二者不符。**dxc 保持一致(模块与 part 均对应 52),llc 不一致**。
- ⇒ **Bug 2 归因 established = LLVM DirectX 后端 emit 的 PSV0 与自身 DXIL 模块不一致(上游 PSV 兼容性 bug)**,非 dxc 版本年代问题。

> 诚实边界:本轮**未**反汇编 validator 内部「期望 size 计算」源码、**未**定位 LLVM 源码中导致模块推得 24 的具体 emit 缺陷行(需上游源码级取证)。归因的**差分证据**(同一 validator:dxc 52 accept / llc 52 reject)已 established「非版本 gap、是 llc emit 不一致」这一结论;LLVM 源码级精确缺陷点标 **not-yet-localized(函数/行级)**。

## 6. 判定逻辑落点(产证据,不下 A/B 裁决)

任务给定三分支,本轮落在**第二分支**:

- ~~新 validator accept → Bug 2 = dxc 1.8 太旧;gap 闭合~~ — **不成立**(新 validator 仍 reject)。
- ✅ **新 validator 仍 reject 且为实质具名拒因(非版本 gap)→ 坐实 LLVM emit 的 PSV 不合规 → 上游 PSV 兼容性 bug established**。本轮即此分支:拒因 `0x80aa0013` 是实质 PSV 结构不一致(非签名缺失、非版本 gap),差分证据排除「validator 太旧」。
- ~~取不到带 dxil.dll 的新 DXC / 加载失败 → blocked~~ — 不适用(新 DXC + dxil.dll + dxv.exe 全部到位并真跑)。

**对 agent 的取证落点**:A 路在工具链层的 validator 互操作 gap **未闭合**——当前 LLVM pin(及 round-5 最新 main)产物经最新 2026 签名 validator 仍被拒,根因为上游 DirectX 后端 PSV emit 不合规(established)。A 打通需上游修复 PSV emit 一致性(或在 Rurix 侧 MIR→DXIL 实现时绕开/修正 PSV0 写出)。这是工具链可行性事实,**不构成 A/B 裁决**,亦 **不**等于 Rurix MIR→DXIL 实现或 device 真跑 golden。

## 7. 同口径对照表(round-6 dxc 1.8 vs round-7 dxc 1.9)

| 维度 | round-6(dxc 1.8.0.4739,~2023) | round-7(dxc 1.9.2602.24,2026) |
|---|---|---|
| dxil.dll 签名 validator | MISSING(用 dxcompiler 内置) | **在位**(dxil.dll + 独立 dxv.exe) |
| wrapper 正反向控制 | ✓(20×) | ✓(25×,新 DLL 重做) |
| llc23 合法容器验证 | reject 20/20,`0x80aa0013` | reject 25/25,`0x80aa0013`(同) |
| llc22 合法容器验证 | reject 20/20,`0x80aa0013` | reject 25/25,`0x80aa0013`(同) |
| 拒因原文 | PSV0 52 vs module 24 | PSV0 52 vs module 24(同) |
| dxc 自产对照 | accept | accept(新 dxc 自产 PSV0=52 仍 accept) |
| DXIL 版本 gap | 无(0x100 两侧同) | 无(0x100 三侧同) |
| **Bug 2 归因** | **C — UNRESOLVED**(无法区分「LLVM 过度 emit」vs「dxc 太旧」) | **established — LLVM emit PSV 内部不一致**(新 validator 接受 52B PSV 却拒 llc → 排除「dxc 太旧」) |

## 8. 复现清单

1. 取 microsoft/DirectXShaderCompiler release **v1.9.2602.24** 的 `dxc_2026_05_27.zip`(zip SHA256 `CF658AAC...`),解压到仓库外 `H:\dxc-round7\extracted`;`bin\x64\` 须含 dxc.exe / dxcompiler.dll / **dxil.dll** / dxv.exe(SHA256 见 §1)。
2. 设环境变量:`RURIX_DXC_NEW_DIR=H:\dxc-round7\extracted\bin\x64`、`RURIX_R6_DIR=H:\llvm-audit-round6`(round-6 合法容器:`official_cs.obj` 1936B / `off_cs22\run-0001.obj` 1804B,SHA256 见 §2);可选 `RURIX_ROUND7_N=25`。
3. 跑探针:`py -3 spike\dxil-path-probe\probe_round7_validator.py`(人读 JSON)或 `RURIX_EMIT_R7=1 py -3 ...`(写 `evidence\dxil_path_spike_20260624_r7.json`)。
4. 流程:正反向控制(新 dxc 自产 accept / 翻字节 reject)验 wrapper → llc22/llc23 各 IDxcValidator(新 dxcompiler.dll + dxil.dll)+ dxv.exe CLI 各 ×25 真验证 → PSV0 RuntimeInfoSize 子轴对照。
5. 交叉验:`dxv.exe <container>` 直跑;`dxv.exe H:\llvm-audit-round6\official_cs.obj` → reject(52 vs 24),`dxv.exe H:\llvm-audit-round6\ctrl_dxc.cso`(旧 dxc 1.8 自产)→ `Validation succeeded.`。

## 9. 约束遵守声明

- **硬规则 1**:未裁 A/B、未代签 G-G2-2;结论只到「validator gap 未闭合 + Bug 2 归因 established」。D-131 维持 C。
- **硬规则 3/4**:measured-first / blocked-honest;数字全部来自命令真实输出(IDxcValidator + dxv.exe 各 ×25);LLVM 源码级精确缺陷点如实标 not-yet-localized。
- **evidence/ 不可篡改门**:round-1~6 既有 evidence/ 文件全部 byte-unchanged;仅新增 `dxil_path_spike_20260624_r7.json` 与本报告。
- **隔离纪律**:新 DXC / dxil.dll 隔离于仓库外 `H:\dxc-round7`,不入库(version/SHA256 写进证据);未动 `C:\Program Files\LLVM`(D-205 pin)、未动 toolchain.rs、未动 src/。
- **禁区(硬规则 5)**:未碰 DXIL UB 边界 / 纹理内存模型 / FFI ABI。
- 未落 codegen / 未建 spec 条款 / 未造错误码 / 未入 golden / 未登 spike_gating;trace 维持现状、零新 RXS;未签 / 未翻 G-G2-2;D-131 维持 C。
- 未向 llvm-project 公开提交 issue/PR/discussion/评论;未修改 Rurix src/spec/golden/codegen。
