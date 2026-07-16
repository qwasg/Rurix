# DXIL A 路 PSV0 不一致 bug 源码定位 + PoC patch + 本地验证 — Round-8 取证报告

> 类型:**纯 spike 取证报告**(Windows-only,源码级)。不裁 A/B(硬规则 1,裁决权属 agent)、不动 D-205 pin、不入库任何 LLVM/patch 产物、不落 codegen、不创建 spec 条款、不造错误码、不入 golden、不登 spike_gating、不签/不翻 G-G2-2、不向 llvm-project 公开提交;D-131 维持 C。
> 承 round-1~7;round-1~7 既有 evidence/ 文件全部 byte-unchanged,本报告与 `dxil_path_spike_20260624_r8.json` 为**新增**。
> Provenance:`Assisted-by: kiro:claude-opus-4-8`(agent 自主记录机器可核对事实,非代决、非代签)。
> 纪律:measured-first / blocked-honest——所有数字来自命令真实输出(emit size / git diff / IDxcValidator ×25 + dxv.exe);定位不到精确行如实标 not-localized。

---

## 0. 核心命题与结论(TL;DR)

**命题**:round-7 把 Bug 2 established 为「llc 容器 PSV0(写 52)与自身 DXIL 模块(validator 推得期望 24)内部不一致」,但**未定位到 LLVM 源码具体缺陷行**(标 not-yet-localized)。本轮在隔离 fork worktree 定位写出侧缺陷点,打最小 PoC patch,增量重建 llc,用 round-7 同款 2026 签名 validator 复验,判 Bug 2 浅修/深坑。

**结论(measured)**:
- **root cause established 到函数/行**:`llvm/lib/Target/DirectX/DXContainerGlobals.cpp:388-389` 的 `addPipelineStateValidationInfo()` 调 `PSV.finalize(MMI.ShaderProfile)` / `PSV.write(OS)` **均不传 Version 实参** → 取默认 `std::numeric_limits<uint32_t>::max()` → `DXContainerPSVInfo.cpp:73-90` 的 `write()` switch 命中 `default` 分支 → `InfoSize = sizeof(dxbc::PSV::v3::RuntimeInfo) = 52`。即 LLVM 写出侧**无条件按最高 PSV 版本(v3=52B)写**,不看模块实际 validator 版本。
- **期望侧机理 established**:validator 从模块 `dx.valver` 推 PSV 版本→期望 size。`official_cs.ll` 无 `dx.valver`,经 `DXILMetadataAnalysis.cpp:43-50` 留 `ValidatorVersion` 空 → `DXILTranslateMetadata.cpp:291-293 emitValidatorVersionMD` 提前 return 不写 `dx.valver` → validator 据缺失推 valver 0.0 → 期望 PSV v0 = 24B。**写 52 / 期望 24 → 0x80aa0013**。
- **PoC patch = 浅修(SHALLOW)**:14 行、单函数、单文件,按 `MMI.ValidatorVersion` 派生 PSV 版本传 `finalize/write`,**不动共享 PSV 基础设施**(`DXContainerPSVInfo.cpp` / `DXContainer.h` 结构与版本-size 表均未改)。增量重建仅 3 步(分钟级)。
- **validator 复验:reject → accept**。同一 2026 签名 validator(dxcompiler.dll+dxil.dll 1.9.2602.24 + 独立 dxv.exe):
  - patch 前 `pre_official_cs.obj`(PSV0=52):IDxcValidator **0/25 accept**({0x80aa0013:25})+ dxv.exe 5/5 reject。
  - patch 后 `post_official_cs.obj`(PSV0=24):IDxcValidator **25/25 accept**({0x0:25})+ dxv.exe 5/5 `Validation succeeded.`。
- **判定**:Bug 2 = **浅修** → **A 路 validator 互操作 gap 可被已知小补丁闭合**(工具链层)。注:A 工具链 validator 可行性 **≠** Rurix MIR→DXIL 实现 **≠** 签名 **≠** device 真跑 golden;**G-G2-2 仍 open**。

裁决归属 agent;本报告只摆事实 + 复现清单。

---

## 1. 隔离环境(复用 round-6/7,不重克隆/不全量重编)

| 项 | 路径 | 状态 |
|---|---|---|
| LLVM fork worktree | `H:\llvm-clean-82c5bce5-src` | 官方 remote 派生 worktree,HEAD=82c5bce5,patch 前 `git status` 干净 |
| LLVM assertions+PDB build | `H:\llvm-clean-82c5bce5-build` | RelWithDebInfo+Assertions,llc.exe + llc.pdb 在位 |
| llc 版本 | — | `LLVM version 23.0.0git / Optimized build with assertions / dxil - DirectX Intermediate Language` |
| round-7 签名 validator | `H:\dxc-round7\extracted\bin\x64` | dxc/dxcompiler.dll/dxil.dll/dxv.exe 1.9.2602.24(SHA256 见证据 JSON round8_detail) |
| 合法输入 | `H:\llvm-audit-round6\official_cs.ll` | 289B,SHA256 `8FA89702...`(byte-unchanged) |
| round8 工作目录 | `H:\dxil-round8` | patch / 容器 / 探针 / 日志(仓库外,不入库) |

- 未动 `C:\Program Files\LLVM`(D-205 pin)、未动 `toolchain.rs`、未动 `src/`。
- patch 前先以未改 llc emit `pre_official_cs.obj` = 1936B SHA256 `76A3D75A...`,与 round-7 `llc23` 字节一致 → 复用现场可信。

## 2. 源码定位(双侧)

### 2.1 写出侧(established 函数/行)

`PSV0` part 由 `DXContainerGlobals::addPipelineStateValidationInfo()` 写出。关键三段:

**(a) 调用处不传 Version**(`llvm/lib/Target/DirectX/DXContainerGlobals.cpp:388-389`):
```cpp
  PSV.finalize(MMI.ShaderProfile);   // 默认 Version
  PSV.write(OS);                     // 默认 Version
  addSection(M, Globals, Data, "dx.psv0", "PSV0");
```

**(b) 默认 Version = 最高版本**(`llvm/include/llvm/MC/DXContainerPSVInfo.h:74-82`):
```cpp
// the default value specifies encoding the highest supported version.
void write(raw_ostream &OS,
           uint32_t Version = std::numeric_limits<uint32_t>::max()) const;
LLVM_ABI VersionTuple
finalize(Triple::EnvironmentType Stage,
         uint32_t Version = std::numeric_limits<uint32_t>::max());
```

**(c) write() 据 Version 选 InfoSize**(`llvm/lib/MC/DXContainerPSVInfo.cpp:73-90`):
```cpp
switch (Version) {
case 0: InfoSize = sizeof(dxbc::PSV::v0::RuntimeInfo); ... break;  // 24
case 1: InfoSize = sizeof(dxbc::PSV::v1::RuntimeInfo); ... break;  // 36
case 2: InfoSize = sizeof(dxbc::PSV::v2::RuntimeInfo); ... break;  // 48
case 3:
default: InfoSize = sizeof(dxbc::PSV::v3::RuntimeInfo);            // 52  ← max 命中此处
}
support::endian::write(OS, InfoSize, ...);   // PSV0 首 u32 写 52
```

各版本 `RuntimeInfo` 字节数(`llvm/include/llvm/BinaryFormat/DXContainer.h` v0..v3 累加):**v0=24 / v1=36 / v2=48 / v3=52**。→ 默认 max ⇒ 写 **52(v3)**。

### 2.2 期望侧(validator 从模块推 24)

validator 据模块 `dx.valver` 推 PSV 版本 → 期望 RuntimeInfoSize。`official_cs.ll` **无** `dx.valver`:
- `llvm/lib/Analysis/DXILMetadataAnalysis.cpp:43-50`:`getNamedMetadata("dx.valver")` 为空 → `MMDAI.ValidatorVersion` 保持空 `VersionTuple{}`。
- `llvm/lib/Target/DirectX/DXILTranslateMetadata.cpp:291-293 emitValidatorVersionMD`:`if (MMDI.ValidatorVersion.empty()) return;` → **不写 dx.valver** 进输出模块。
- validator 见模块无 `dx.valver` → 推 valver 0.0 → 期望 PSV **v0 = 24B**。

→ 写出侧 52(v3)/ 期望侧 24(v0)**内部不一致** = round-7 具名拒因 `0x80aa0013 PSVRuntimeInfoSize 'PSV0' part:('52') vs DXIL module:('24')` 的精确根因。

### 2.3 定位法与诚实边界

- Bug 2 **非崩溃**(不同于 Bug 1),无 cdb 异常栈;定位经**源码读 + emit 前后 PSV0 size 52→24 实测 + valver 双向对照**三证齐全(assertions+PDB build 在位但对非崩溃路径不产栈)。
- **未做**:未反汇编 `dxil.dll` validator 内部「期望 size 计算」源码逐字节(valver→PSV 版本映射边界 1.1/1.6/1.8 交叉读 DXC 规范 + round-7 线索推得,未逐字节确认 DXC 源);未向 llvm-project 公开提交。这些不影响写出侧函数/行的 established 定位。

## 3. PoC patch(最小、隔离,不入库)

仅改 `DXContainerGlobals.cpp:388-389` 一函数,按模块 validator 版本派生 PSV 版本传入 `finalize/write`,与 validator 推导对齐。patch 存隔离 `H:\dxil-round8\dxil_psv_version_round8.patch`(SHA256 `8C170AA763EE9DC6...`,2620B,16 +行);**不入 Rurix 仓库、不入 llvm-project**。`git diff` 全文:

```diff
@@ -385,8 +385,21 @@ void DXContainerGlobals::addPipelineStateValidationInfo(
       MMI.ShaderProfile != Triple::RootSignature)
     PSV.EntryName = MMI.EntryPropertyVec[0].Entry->getName();

-  PSV.finalize(MMI.ShaderProfile);
-  PSV.write(OS);
+  // SPIKE(RD-010): derive PSV version from the module validator version so the
+  // emitted PSVRuntimeInfoSize matches what the validator computes from the
+  // module (mirrors DXC). Without this, PSV is always written at the max
+  // version (v3=52B) while a module lacking dx.valver makes the validator
+  // expect v0 (24B) -> 0x80aa0013 PSVRuntimeInfoSize mismatch.
+  uint32_t PSVVersion = 3;
+  VersionTuple ValVer = MMI.ValidatorVersion;
+  if (ValVer.empty() || ValVer < VersionTuple(1, 1))
+    PSVVersion = 0;
+  else if (ValVer < VersionTuple(1, 6))
+    PSVVersion = 1;
+  else if (ValVer < VersionTuple(1, 8))
+    PSVVersion = 2;
+  PSV.finalize(MMI.ShaderProfile, PSVVersion);
+  PSV.write(OS, PSVVersion);
   addSection(M, Globals, Data, "dx.psv0", "PSV0");
 }
```

- 改动文件:`llvm/lib/Target/DirectX/DXContainerGlobals.cpp`(单文件);改动函数:`addPipelineStateValidationInfo`(单函数)。
- **不动**共享 PSV 基础设施:`DXContainerPSVInfo.cpp`(write/finalize 实现)、`DXContainer.h`(v0..v3 RuntimeInfo 结构 + 版本-size 表)均 byte-unchanged,只改调用方传入的 `Version` 实参。

## 4. 增量重建 + 验证闭环

**重建**:`vcvars64`(VS2022 14.44)→ `ninja -j6 llc`,增量仅 3 步(`[1/3]` 编译 `DXContainerGlobals.cpp.obj` → `[2/3]` 链 `LLVMDirectXCodeGen.lib` → `[3/3]` 链 `llc.exe`),分钟级。post-patch `llc.exe` SHA256 `9E57C24C65DB0EBC...`。

**emit + 验证**(IDxcValidator ×25 复用 round-7 harness + 独立 dxv.exe CLI):

| 容器 | 输入 | PSV0 InfoSize | 大小/SHA256 | IDxcValidator ×25 | dxv.exe | 拒因 |
|---|---|---|---|---|---|---|
| `pre_official_cs.obj`(未 patch) | official_cs.ll | **52** | 1936B / `76A3D75A...` | **0/25** accept,`{0x80aa0013:25}` | 5/5 reject | PSV0 52 vs module 24 |
| `post_official_cs.obj`(patch 后) | official_cs.ll(无 valver) | **24** | 1892B / `019E3A51...` | **25/25** accept,`{0x0:25}` | 5/5 `Validation succeeded.` | — |
| `post_valver18.obj`(patch 后) | + `!dx.valver=!{1,8}` | **52** | 1960B / `62C49FCD...` | **25/25** accept,`{0x0:25}` | 5/5 `Validation succeeded.` | — |

**双向对齐证**:patch 后写出版本随模块 `dx.valver` 双向跟随——缺 valver → v0(24)accept;valver 1.8 → v3(52,含 NumThreads)accept。两子例均自洽且被同一 2026 签名 validator 接受。`post_valver18` 直接证明 validator 期望 size 确随模块 valver 变化(无 valver 期望 24、valver 1.8 期望 52),坐实期望侧机理。

**reject → accept**:`pre`(52)0/25 → `post`(24)25/25,status `0x80aa0013` → `0x0`;IDxcValidator 与 dxv.exe 双签名 validator 一致。

## 5. 判深浅(产证据,不下 A/B 裁决)

| 维度 | 量化 | 落点 |
|---|---|---|
| patch 行数 | 16 +行(净逻辑约 9 行) | 小 |
| 触及范围 | 单文件 / 单函数 / 不动共享 PSV 基础设施 | 浅 |
| validator 转化 | reject 0/25(0x80aa0013)→ accept 25/25(0x0),双 validator 一致 | 闭合 |

**判定 = 浅修(SHALLOW)**:Bug 2 是 LLVM 写出侧「PSV 版本未按模块 valver 派生、硬选 max」的小缺陷,minimal patch(改调用方传参,不碰版本-size 表/结构)即令 validator 由 reject 转 accept。→ **A 路 validator 互操作 gap 可被已知小补丁闭合(工具链层)**;上游 PR 具备 root cause(函数/行)+ fix(diff)+ 前后 validator 对照。

**诚实边界**:缺 valver 时写 v0(24)会丢 compute 的 NumThreads,属「最小公约数对齐」(令 part 与缺 valver 模块自洽);更完整修法 = 确保模块带 `dx.valver` 使写 v3 全量 RuntimeInfo(见 `post_valver18` 子例,亦 accept)。A 工具链 validator 可行性 **≠** Rurix MIR→DXIL 实现 **≠** 签名 **≠** device 真跑 golden;**G-G2-2 仍 open**,D-131 维持 C。

## 6. 复现清单

1. 复用隔离 fork worktree `H:\llvm-clean-82c5bce5-src`(82c5bce5,干净)+ build `H:\llvm-clean-82c5bce5-build`(RelWithDebInfo+Assertions+PDB);若不在按 round-6 阶段 4 recipe 重建(vcvars64 VS2022 14.44 + CMake + Ninja,`-DLLVM_EXPERIMENTAL_TARGETS_TO_BUILD=DirectX`,`ninja -j6 llc llvm-as`,-j 勿过高)。
2. 打 PoC patch(§3 diff):改 `DXContainerGlobals.cpp:388-389` 按 `MMI.ValidatorVersion` 派生 PSV 版本传 `finalize/write`;增量 `ninja -j6 llc`(3 步,分钟级)。
3. emit:`llc official_cs.ll -filetype=obj -o post_official_cs.obj`(1892B,PSV0 InfoSize 24);加 `!dx.valver=!{i32 1,i32 8}` 的 `official_cs_valver18.ll` → `post_valver18.obj`(1960B,PSV0 52)。对照 PRE = 未 patch llc emit(1936B,PSV0 52)。
4. 验:`RURIX_DXC_NEW_DIR` 指 `H:\dxc-round7\extracted\bin\x64`(dxcompiler.dll+dxil.dll+dxv.exe 1.9.2602.24),`py -3 H:\dxil-round8\r8_probe.py <obj>`(IDxcValidator ×25,复用 `spike/dxil-path-probe/dxil_validator.py` + `dxil_container.py`)+ `dxv.exe <obj>`(独立 CLI)。
5. 期望:PRE 0/25 accept(0x80aa0013)/ POST 25/25 accept(0x0);dxv.exe PRE reject / POST `Validation succeeded.`。

## 7. 约束遵守声明

- **硬规则 1**:未裁 A/B、未代签 G-G2-2;结论只到「Bug 2 root cause established 到 LLVM 函数/行 + 浅修 + A validator 互操作 gap 可被已知小补丁闭合」,未替 agent 选路径,**未**把 Rurix 切到 fork LLVM(D-205 决策属 agent + 独立勘误,不在本 spike)。D-131 维持 C。
- **D-205 pin 不动**:未动 `C:\Program Files\LLVM`、未动 `toolchain.rs`、未动 `src/`;fork/patch/重建只进仓库外 `H:\llvm-clean-82c5bce5-*` / `H:\dxil-round8`。
- **硬规则 3/4**:measured-first / blocked-honest;数字全部来自命令真实输出(emit size / `git diff` / IDxcValidator ×25 + dxv.exe);未做的(反汇编 dxil.dll validator 内部计算、公开提交)如实标注。
- **evidence/ 不可篡改门**:round-1~7 既有 evidence/ 文件全部 byte-unchanged;仅新增 `dxil_path_spike_20260624_r8.json` 与本报告。
- **LF 字节精确**:本报告与证据 JSON 二进制写 + 显式 LF(落盘自核 CR=0、尾字节 0x0a)。
- **禁区(硬规则 5)**:未碰 DXIL UB 边界 / 纹理内存模型 / FFI ABI——PSV 是容器元数据结构(RuntimeInfoSize 版本字段),非禁区。
- **隔离纪律**:LLVM patch / `.patch` 文件 / 重建 llc **未入库**,只进 `H:\dxil-round8`(diff 全文抄进本报告 §3,digest 写证据 JSON `round8_detail`)。
- 未落 codegen / 未建 spec 条款 / 未造错误码 / 未入 golden / 未登 spike_gating;trace 维持现状、零新 RXS;未签 / 未翻 G-G2-2;未向 llvm-project 公开提交 issue/PR/discussion/评论。

## 8. 同口径对照(round-7 → round-8)

| 维度 | round-7 | round-8 |
|---|---|---|
| Bug 2 归因 | established 上游 emit PSV 内部不一致(差分证据) | **+ 源码级 root cause 到函数/行**(DXContainerGlobals.cpp:388-389 默认 max PSV 版本) |
| LLVM 源码缺陷点 | **not-yet-localized** | **established**(写出侧函数/行 + 期望侧机理链) |
| PoC patch | 无(round-6/7 未打 patch) | **14 行单函数 PoC + 增量重建 llc** |
| validator 结果 | reject 0/25(0x80aa0013) | patch 后 **accept 25/25(0x0)**,IDxcValidator + dxv.exe 一致 |
| 深浅判定 | 未判 | **浅修**(A validator 互操作 gap 可被已知小补丁闭合) |

## 9. 结尾声明

未向 llvm-project 公开提交任何内容;未修改 Rurix src/spec/golden/codegen;未动 D-205 pin;D-131 维持 C;G-G2-2 未签署、仍 open;仅完成 Windows 验证,未声称跨平台;round-1~7 既有 evidence/ 未改动,本报告与 `dxil_path_spike_20260624_r8.json` 为新增;自编 LLVM/patch/重建 llc 隔离于仓库外,未入库。
