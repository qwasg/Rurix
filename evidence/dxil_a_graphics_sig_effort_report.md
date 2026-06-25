# A 路 DirectX 图形签名工作量评估 spike — 给 A-graphics 成本一个实测锚（2026-06-25）

> 类型：**纯评估 spike**（Windows-only）。不裁 A/B/混合架构（硬规则 1，裁决权属 owner）、
> 不改 Rurix src/spec/codegen、不动 D-131（维持 A）/D-205 pin/toolchain.rs、未向 llvm-project 公开提交。
> 跟踪：`registry/deferred.json` **RD-010**（A/B 路径裁决取证）/ **RD-011**（dev-only patched llc）。
> 关联：RFC-0003 §4.6/§9 Q-Builtin（禁区边界）/ `evidence/dxil_slice3_rxs0159_sig_disasm_round8.md`（空签名根因）/
> `evidence/dxil_b_graphics_sig_report.md`（B 对照）。
> Provenance：`Assisted-by: kiro:claude-opus-4-8`。证据 JSON：`evidence/dxil_a_graphics_sig_effort_20260625.json`（schema PASS）。
> 纪律：measured-first / blocked-honest——源码行/上游 URL/PoC 结果来自真实勘察 + GitHub API + 隔离 LLVM 重建；工作量明确标 estimated。

---

## 0. 结论（TL;DR）

让 A 路（LLVM DirectX 后端直接 emit DXIL）产**带真实 SV 签名的图形 DXIL**，当前缺口横跨
**clang 前端 + LLVM 后端 + PSV** 三侧，且 **validator 按模块重算签名交叉校验**（本轮 PoC 实测）。

- **最小 SV-only**：estimated **~300-600 LOC / 3-4 文件**，跨前后端协同，**非孤立单点**。
- **完整签名**（user varying + 插值 + packing + 多 RT）：estimated 增量 **~800-1500+ LOC**。
- **packing 布局 = conformance**（复刻 dxc/D3D12 既定算法），**非自由 ABI 设计**——事实陈述供 owner 判是否仍需 Full RFC（不替 owner 落笔禁区，硬规则 5）。
- **carry-patch 不可行**（像 RD-011 那样）：最小 SV-only 是上游未实现的整块新功能（三处 FIXME），非浅修单点；上游 **#90504 / #57928 无在途 PR 可 cherry-pick**。
- **PoC 硬锚**：~12 行 patch 即让 ISG1 出现真实 SV_Position（elemcount 0→1），但 validator **拒绝**（`0x80aa0013`，"Program Input Signature does not match expected for module"）——证明工作量不在 part emit（trivial），而在让模块一致编码签名。

**取证落点**：与 slice3/B 苹果对苹果——A 路图形签名启动成本**高、跨前后端、周期长**，上游签名出口子任务**停滞无在途 PR**；证据偏向「图形成本远高于 compute」。裁决留 owner。

---

## 1. 环境（隔离不入库）

| 项 | 值（真实输出） |
|---|---|
| LLVM 源码 worktree | `H:\llvm-clean-82c5bce5-src`（只读勘察 + PoC） |
| HEAD commit | `626063a6…`（[DirectX] Select PSV version from validator version，= RD-011 PSV patch） |
| base（origin/main） | `82c5bce5…`（grafted 浅克隆，单 commit graft） |
| LLVM 版本 | `LLVM 23.0.0git, Optimized build with assertions` |
| baseline llc SHA256 | `CB56E0D9…`（空签名，elemcount=0） |
| PoC llc SHA256 | `6E7584E5…`（硬编码 1 SV_Position，PoC 后已还原） |
| 签名 validator | `H:\dxc-round7\extracted\bin\x64` dxcompiler.dll/dxil.dll/dxv.exe `1.9.2602.24` |

> **git 历史 blocked**：grafted 浅克隆无法逐 commit 分析 DirectX 后端历史活跃度；改用 GitHub API 评估上游活跃度（§3）。

## 2. 源码勘察（写出侧全路径——缺口落在哪几个函数/行）

| ID | 文件:行 | 缺口 |
|---|---|---|
| S1 | `lib/Target/DirectX/DXContainerGlobals.cpp:226-236` `addSignature` | 图形签名 emit 出口：无条件构造**空** InputSig/OutputSig 直接 write 8 字节空 ISG1/OSG1，从不 `addParam`、不读任何模块元数据。`// FIXME: support graphics shader.`（#90504） |
| S2 | `lib/Target/DirectX/DXILTranslateMetadata.cpp:541-543` | `dx.entryPoints` 的 `Signatures` 操作数**硬编码 `nullptr`**，签名元数据从不构造。`// FIXME: Add support to construct Signatures`（#57928） |
| S3 | `clang/lib/CodeGen/CGHLSLRuntime.cpp:1036-1090` `emitDXILUserSemanticLoad/Store` | 前端 `dx.load.input`/`dx.store.output` intrinsic **已生成**，但 sigpoint/row/col/index **全硬编码占位**（`getInt32(4),getInt32(0),getInt32(0),getInt8(0)`）。`// DXIL packing rules etc shall be handled here. // FIXME: generate proper sigpoint, index, col, row values.` |
| S4 | `DXContainerGlobals.cpp:268-375` `addResourcesForPSV` | PSV 路径只填 `Resources`，从不填 `PSVRuntimeInfo.InputElements/OutputElements`（签名元素 PSV 侧同样空） |
| S5 | `include/llvm/MC/DXContainerPSVInfo.h:90-115` `mcdxbc::Signature::addParam` | 填一个签名元素**强制要求** Stream/Name/Index/SystemValue/CompType/**Register/Mask/ExclusiveMask**/MinPrecision；Register/Mask = 寄存器/分量 packing 布局 |

**全后端签名/语义消费点 = 0**：`Select-String Semantic|SignatureElement|D3DSystemValue|SigComponentType` 扫全 DirectX 后端，
除 S1 两个空构造外**零命中**——后端无任何从语义/签名信息 lower 进 ISG1/OSG1 的代码。

**缺的几块**：(a) 后端签名 part 生成（#90504）；(b) 后端签名元数据构造（#57928）；
(c) 前端 SV 索引/packing（S3 占位待实现）；(d) PSV 签名字段填充 + 与 ISG1/OSG1 一致；(e) ViewIdState 依赖分析（完整签名）。

## 3. 上游现状（GitHub API，2026-06-25 查）

| issue/PR | 状态 | 标题 | 关键事实 |
|---|---|---|---|
| [#90504](https://github.com/llvm/llvm-project/issues/90504) | **open** | Generate ISG1, OSG1 parts from signature module metadata | 后端签名 part 生成总 issue（S1 FIXME 指向）；created 2024-04-29，**updated 2026-06-19**；**无在途实现 PR** |
| [#57928](https://github.com/llvm/llvm-project/issues/57928) | **open** | Generate module metadata for input output signature | 前端/元数据侧签名构造（S2 FIXME 指向）；updated 2026-06-19；**无在途实现 PR** |
| [#143523](https://github.com/llvm/llvm-project/issues/143523) | **open** | Implement SV_Position semantic when targeting DXIL on pixel shaders | **恰是最小 SV-only(pixel SV_Position)场景**；created 2025-06-10，updated 2026-06-19，**unassigned**；前端 intrinsic 地基在建但 packing 未完成 |
| [#67346](https://github.com/llvm/llvm-project/pull/67346) | **merged** 2023-10-05 | [DX] Add support for program signatures | +859/-65，13 文件：建了 `mcdxbc::Signature`/`addParam`/PSV 签名元素**基础设施**，但**未接图形 emit 路径**，闲置至今 |

**direction = warm-but-incomplete**：2026 年前端 HLSL 语义工作活跃（CGHLSLRuntime `dx.load.input` 地基 + #143523 SV_Position 场景 + #90504/#57928 同日 2026-06-19 触达），
但**签名→DXContainer ISG1/OSG1 生成当前无在途 PR**；基础设施（#67346）2023 已 merged 闲置未接图形。
**结论：方向活跃，但签名出口子任务停滞、无可 carry 的在途工作**——这对工作量极不利（须自行实现，非等上游/cherry-pick）。

## 4. 禁区 vs conformance 厘清（关键）

**裁断 = packing 布局属 conformance（dxc/D3D12 既定硬性规则），非 Rurix 自由 ABI 决策。**

证据链：
1. **clang 源码注释明示**：`CGHLSLRuntime.cpp` `// DXIL packing rules etc shall be handled here.` +
   `// DXC completely ignores the semantic/index pair. Locations are assigned from the first semantic to the last.`
   ——即**复刻 dxc 既定 location 分配算法**，非自由设计。
2. **二进制格式规范固定**：`DxilProgramSignatureElement` 由 DXContainer 容器规范固定（[LLVM DXContainer 文档](https://www.llvm.org/docs/DirectX/DXContainer.html) PSV0 Signature Elements）。
3. **MS 官方 HLSL packing rules**：[learn.microsoft.com](https://learn.microsoft.com/en-us/windows/win32/direct3dhlsl/dx-graphics-hlsl-packing-rules)
   规范 VS output / GS in-out / PS in-out 的 packing；dxc 侧由 `DxilSignatureAllocator` 实现该算法。
4. **validator 强制一致**：本轮 PoC 实测 validator 按模块重算期望签名并交叉校验（§6）。
5. **B 路证据印证**：`dxil_b_graphics_sig_20260625.json` 中 dxc **直产** register/mask 值（SV_Position reg0 mask0xF、POSITION reg1 mask0x7…）
   即 dxc 算法分配——**无设计自由度**。

**分层结论**：SV 系统值语义名→D3DSystemValue 映射（SV_Position→Position）= RXS-0159 类型面已裁；
register/mask **packing = conformance 复刻项**（既非类型面亦非自由 ABI 设计）。

> **事实陈述供 owner 裁**（硬规则 5）：若 owner 认 packing 属 conformance 复刻（非自由 ABI），
> 则 slice3/RFC §4.6(c) 标的「签名二进制布局 = FFI ABI 禁区、需 Full RFC」**可能降级为 conformance 说明**——
> 但此判断由 owner 落笔，本 spike 只陈述「packing 无设计自由度、是 dxc/规范既定算法」这一事实，**不替 owner 改禁区条款**。

## 5. 工作量估算（分档，level=estimated）

### T0 最小 SV-only（vertex SV_Position 输出 + fragment SV_Target 输出 / SV_Position 输入，无 user-varying packing）
- **改的组件**：(a) 后端 addSignature 从模块 I/O 派生 SV 元素；(b) 签名元数据构造 `dx.entryPoints`（或后端直读 `dx.load.input/store.output` intrinsic）；(c) 前端 sigpoint/row/col 正确化（SV 专属）；(d) PSV InputElements/OutputElements 填充 + 与 ISG1/OSG1 一致。
- **估算（estimated）**：**~300-600 LOC，跨 clang 前端 + LLVM 后端 3-4 文件**（CGHLSLRuntime + DXILTranslateMetadata + DXContainerGlobals + PSV）。**非孤立单点**，比 RD-011 的 14 行 PSV 单函数 patch 大 1-2 个数量级。
- **风险（高）**：experimental 后端 + 三处 FIXME 跨前后端协同 + validator 按模块重算签名交叉校验（PoC 实测孤立元素被拒）+ obj 写出器历史不稳（round-4/5 崩溃，round-8 well-formed 输入稳定）。SV-only 可避开 user-varying packing，但 SV 仍需正确 register/sigpoint/PSV 一致。

### T1 完整签名（user varying + 插值 + register-component packing + 多 RT）
- **增量组件**：user 语义名 varying + 插值限定符（linear/nointerpolation…）+ packing 算法（复刻 dxc `DxilSignatureAllocator`）+ 多 RT（SV_Target0..N）+ 跨阶段 varying 兼容（vs out↔ps in）+ ViewIdState 依赖分析。
- **估算（estimated）**：增量 **~800-1500+ LOC**；complete graphics signature 是 dxc 中数千行模块的复刻面。
- **风险（很高）**：packing 虽属 conformance（无设计自由度）但工程量大、validator 一致性严苛；插值 + ViewId + 跨阶段链接逐项对齐 dxc 行为。

## 6. carry-patch 可行性

**partial-blocked：像 RD-011 那样 carry 小 patch 解锁 A-graphics 开发不可行。**

| 维度 | RD-011（PSV patch） | A-graphics 最小 SV-only carry |
|---|---|---|
| 性质 | 浅修上游既有 bug（单点） | 上游未实现的整块新功能（三处 FIXME） |
| 规模 | 14 行单函数 | estimated ~300-600 LOC / 3-4 文件 |
| 来源 | 自写浅修 | **上游 #90504/#57928 无在途 PR 可 cherry-pick**，须自行替上游实现 |
| 可维护性/rebase | 低成本 | 远高（fork 内维护整块新功能，跨前后端） |

→ RD-011 可行因其是浅修单点既有缺陷；A-graphics 签名 carry 等于在 fork 里替上游实现 #90504+#57928+前端 packing，量级与维护成本远高于 RD-011。

## 7. PoC 锚定结果（有界，~12 行 patch + 分钟级重建）

**PoC patch**（隔离不入库，`H:\dxil-a-graphics-sig\addsignature_poc.patch`，SHA256 `046E729E…`，+10/-2 单函数）：
在 `addSignature` 为 pixel shader **硬编码**一个 SV_Position 输入元素（非真实现：无元数据驱动/无 PSV 填充/无 packing）。

| 样例 | llc | ISG1 elemcount | IDxcValidator | dxv.exe |
|---|---|---|---|---|
| ps_svpos **pre** | baseline | **0** | accept `0x0` | `Validation succeeded.` |
| ps_svpos **post** | PoC patched | **1**（SV_Position / reg0 / mask 0xF / float32） | **reject `0x80aa0013`** | `error: Container part 'Program Input Signature' does not match expected for module. Validation failed.` |

**硬锚结论**：
1. 签名 **PART 承载真实 SV 元素 = trivial**——`addParam` + ~12 行 + 分钟级增量重建即让 ISG1 elemcount 0→1 含真实 SV_Position。
2. 但 validator（2026 签名 validator）**拒绝孤立签名元素**（`0x80aa0013`，"Program Input Signature does not match expected for module"）——validator **按模块重算期望签名并交叉校验**。
3. 故最小 SV-only 工作量**不在 part emit（trivial）**，而在**让模块一致编码签名**（前端 intrinsic packing #57928 + 后端从元数据派生 #90504 + PSV 元素填充 + validator 三侧一致），即 §5 T0 的 (a)-(d)。
4. PoC **既非假绿**（未录空 golden）**亦非伪 SV**（elemcount 真 0→1）；拒绝是正确的交叉校验信号，非工具坏。

PoC 后已 `git checkout` 还原源码 + 重建还原 baseline llc（elemcount 复 0，SHA256 `CB56E0D9…`），**RD-011 dev 工具链未污染**。

## 8. 复现清单

```powershell
# 0. 隔离环境(仓库外,不入库):H:\llvm-clean-82c5bce5-src(82c5bce5) + -build(RelWithDebInfo+Assertions)
#    签名 validator H:\dxc-round7\extracted\bin\x64 (1.9.2602.24)

# 1. 源码勘察(只读)
Select-String -Path H:\llvm-clean-82c5bce5-src\llvm\lib\Target\DirectX\DXContainerGlobals.cpp -Pattern "addSignature|FIXME"
Select-String -Path H:\llvm-clean-82c5bce5-src\llvm\lib\Target\DirectX\DXILTranslateMetadata.cpp -Pattern "Signatures = nullptr|construct Signatures"
Select-String -Path H:\llvm-clean-82c5bce5-src\clang\lib\CodeGen\CGHLSLRuntime.cpp -Pattern "packing rules|sigpoint"

# 2. 上游状态(GitHub API)
Invoke-RestMethod "https://api.github.com/repos/llvm/llvm-project/issues/90504" -Headers @{"User-Agent"="x"}  # state=open
Invoke-RestMethod "https://api.github.com/repos/llvm/llvm-project/issues/57928" -Headers @{"User-Agent"="x"}  # state=open

# 3. PoC:patch addSignature 硬编码 1 SV_Position(diff 见 H:\dxil-a-graphics-sig\addsignature_poc.patch)→ 增量重建
#    cd /d H:\llvm-clean-82c5bce5-build && ninja -j6 llc
# 4. emit + dump + validate
$llc="H:\llvm-clean-82c5bce5-build\bin\llc.exe"
& $llc H:\dxil-a-graphics-sig\ps_svpos.ll -filetype=obj -o post.obj
py -3 -c "import sys;sys.path.insert(0,r'spike/dxil-path-probe');import dxil_container as c;print(c.parse_signature_part(open('post.obj','rb').read(),'ISG1'))"  # elemcount 1, SV_Position
H:\dxc-round7\extracted\bin\x64\dxv.exe post.obj   # reject: Program Input Signature does not match expected for module
# 5. 还原:git checkout DXContainerGlobals.cpp; touch; ninja llc(elemcount 复 0)
# 6. schema 校验
py -3 ci\check_schemas.py    # PASS
```

## 9. 约束遵守声明

- **硬规则 1**：未裁 A/B/混合架构、未代签；结论只到「A-graphics 工作量 + 禁区归属 + carry-patch 可行性」。D-131 维持 A。
- **硬规则 3/4**：源码行/上游 URL/PoC status/SHA256 全来自命令真实输出 + GitHub API + 隔离 LLVM 重建；工作量明确标 **estimated**；git 浅克隆致历史活跃度分析 **blocked**（如实记，改用 GitHub API）。
- **硬规则 5**：勘察/PoC 仅读写隔离 LLVM 源码；**未在 Rurix 仓库落任何禁区(签名 ABI 布局)条款**——禁区归属(packing=conformance)只作「是否需 Full RFC」的事实陈述，不替 owner 落笔禁区内容。
- 不改 Rurix src/spec/codegen；不动 D-131/D-205 pin/toolchain.rs；未向 llvm-project 公开提交。
- **evidence/ 不可篡改门**：既有 evidence 全部 byte-unchanged；仅新增 `dxil_a_graphics_sig_effort_20260625.json` + 本报告 + g2 schema + check_schemas 接线。
- **隔离纪律**：LLVM PoC patch / 重建 llc / obj 隔离于 `H:\dxil-a-graphics-sig` + `H:\llvm-clean-82c5bce5-*` 不入库，digest 见证据 JSON `llvm_env`/`poc`。
- **LF 字节精确**：证据 JSON / schema 经显式 LF 写出（CR=0，尾 0x0a，自核通过）。
- Provenance：`Assisted-by: kiro:claude-opus-4-8`；影响范围 = 新增 evidence/schema + check_schemas 接线 + RD-010 history/revision_log 追加；验证见 §2/§3/§6/§7 + `ci/check_schemas.py` PASS。
