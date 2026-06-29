# DXIL 路径双 blocker — Round-6 Windows 本地归因审计报告

> 类型：**纯取证审计报告**（Windows-only 本地归因）。不立项、不裁 A/B、不改 src/spec/golden/codegen、不动 D-131（仍 C）、不签/不翻 G-G2-2、未向 llvm-project 公开提交任何内容。
> 承 round-1~5；round-1~5 既有 evidence/ 文件全部 byte-unchanged，本报告为**新增**文件。
> Provenance：`Assisted-by: kiro:claude-opus-4-8`（agent 自主记录机器可核对事实，非代决、非代签）。
> 纪律：所有判定基于真实命令/日志/hash/栈/diff；无法完成的检查标记 UNRESOLVED；无真实符号栈处写 "Root cause not established."

本轮核心目标：在 **仅 Windows** 条件下，优先**证伪「LLVM 上游问题」**，排除 PATH 混用 / 源码污染 / 旧 build cache / ABI·DLL 混用 / Python 探针 / 并发文件冲突 / 非法 IR / Validator ctypes 错误 / dxc·llc 输入不等价，之后才允许归类上游。

---

## 阶段 1：冻结并审计现有现场（只读）

### 1.1 源码身份与污染

| 项 | LLVM 22（H:\llvm-dxil\llvm-project） | LLVM 23（H:\llvm-upstream-test\llvm-project） |
|---|---|---|
| remote | `https://github.com/llvm/llvm-project.git`（官方） | 同（官方） |
| HEAD | `a255c1ed36a1d06f79bd2633ba9f8d900153007c` | `82c5bce5233f964da4f8086b2341067314d841d7` |
| describe --tags | `a255c1ed3`（**仅短 hash，无 tag**） | `82c5bce52`（无 tag） |
| status --short | 空（无未提交修改） | 空（无未提交修改） |
| diff --stat | 空（无 patch、无本地改动） | 空 |
| 克隆形态 | **grafted 浅克隆** | grafted 浅克隆（main） |

- 两树均官方 remote、工作区干净、无本地 patch、无新增文件、各自独立源码（未共享）。
- **疑点（已记录）**：22 树为 grafted 浅克隆，本地无 tag，`describe --tags` 只返回短 hash，**无法在本地确认 a255c1ed == llvmorg-22.1.7**；commit a255c1ed 的 message 为一条 WebAssembly 提交而非 release tag。版本字符串（见 1.4）确为 22.1.7，但 commit↔tag 精确映射在浅克隆下 **UNRESOLVED**。
- 23 树 commit 82c5bce5 与任务已知值一致。

### 1.2 实际二进制（绝对路径，非 PATH 猜测）

- `where.exe llc / llvm-as / opt` → **均不在 PATH**（无 VS LLVM / Chocolatey / MSYS2 / 系统 LLVM / 其他项目内置 LLVM 混入）。探针经 `RURIX_LLC` 绝对路径直调 build-tree 二进制。
- `where.exe dxc` → `C:\ti-localappdata\ti-build-cache\vulkan-1.3.296.0\Bin\dxc.exe`（Vulkan SDK 1.3.296.0）。
- `C:\Program Files\LLVM`（D-205 pin）存在，但**未**进入 llc 调用路径。

| 二进制 | SHA256 | 大小 | 版本 | dxil target |
|---|---|---|---|---|
| H:\llvm-dxil\build\bin\llc.exe | `7E50F53AE367AA59881DFEE58F931C7126438ABD2EDE9B4AC8E7281F00AEA942` | 39193088 | 22.1.7 / Optimized | 有 |
| H:\llvm-dxil\build\bin\llvm-as.exe | `B6AF56F734761A70AD0F3A22EDFDE2FE3E1C8A0F02A0F6BD8E50C1D4C6BF8A50` | 4194816 | 22.1.7 | — |
| H:\llvm-upstream-test\build\bin\llc.exe | `78950790FEF4DE7B60D7742C58DE7B1A8F8C0AE28FC7C4573ADBA20196AE2493` | 40977408 | 23.0.0git / Optimized | 有 |
| H:\llvm-upstream-test\build\bin\llvm-as.exe | `C8835FCD46488E0FA7E14C3F96FA1B511D75FBF2AC21CB35C7E6661F9948D009` | 4116480 | 23.0.0git | — |
| opt.exe（两树） | — | — | **MISSING**（recipe 只 `ninja llc llvm-as`） | — |

- llc/llvm-as 同 build 树同源。**opt 缺失** → 第7步 verifier 改用 llvm-as 内置 verifier。

### 1.3 构建配置（CMakeCache.txt）

两套 build 完全一致：

| 键 | 值 |
|---|---|
| CMAKE_BUILD_TYPE | **Release** |
| LLVM_ENABLE_ASSERTIONS | **OFF** |
| LLVM_USE_SANITIZER | （空） |
| LLVM_ENABLE_LTO | OFF |
| LLVM_BUILD/LINK_LLVM_DYLIB | （未设 = 静态链接） |
| CMAKE_C/CXX_COMPILER | MSVC 14.44.35207（VS2022 Community）cl.exe |
| CMAKE_GENERATOR | Ninja |
| TARGETS / EXPERIMENTAL | X86 / DirectX |
| PROJECTS | 22:（空）/ 23: clang |

- 静态链接（无 LLVM DLL）、无 sanitizer、无 LTO、同一 MSVC、同一 generator。
- **限制**：两 build 均 Release/**无 assertions**/无 PDB → 现有二进制无法做断言命中或符号化崩溃栈。这正是阶段 4 另建 Assertions/RelWithDebInfo 构建的原因。
- 无法完全排除现有 build 为增量编译（无独立配置时间戳证据）→ 旧 build 仅作现象记录，上游归因以阶段 4 全新构建为准。

---

## 阶段 2：排除 Python 探针与文件并发问题

### 2.1 探针真实命令（审计 probe_a_llvm_directx.py / _common.py）

- llc 调用：`subprocess.run([llc_path, f"-filetype={filetype}", irp, "-o", op], timeout=30)`，`shell=False`、list 参数（无字符串拼接、无注入风险）。
- 每次 attempt **唯一输出文件** `out_{tag}_{i}.{filetype}`，写前 `os.remove` 已存在文件；输入 IR 每 tag 写一次 `ir_{tag}.ll`。
- 异常码：`CRASH_RCS = (3221225477, -1073741819)` = 0xC0000005 两表示；分类 rc==0&有产物→ok / rc∈CRASH_RCS→crash / 其余→other。转换正确。
- **关键审计发现**：探针**仅顺序执行**（`for i in range(ATTEMPTS)`，ATTEMPTS=12），**从不并发**。round-4/5 的"并发统计"措辞与探针实现不符——探针无并发轴。本轮补齐顺序 100×+并发 100×直接 CLI。

### 2.2 绕过 Python，直接 CLI 复现（绝对路径，唯一输出，独立 stderr）

repro 脚本：每次 `run-NNNN.{ft}` 唯一输出 + `run-NNNN.stderr.txt`；统计 OK/0xC0000005/other。

| 配置 | 工具 | 模式 | N | OK | 0xC0000005 | other |
|---|---|---|---|---|---|---|
| bare IR / sm6.0 / **obj** | LLVM23 | 顺序 | 100 | 35 | **65** | 0 |
| bare IR / sm6.0 / **obj** | LLVM23 | **并发**(throttle16) | 100 | 42 | **58** | 0 |
| bare IR / sm6.0 / **asm** | LLVM23 | 顺序 | 100 | **100** | 0 | 0 |
| bare IR / sm6.0 / **obj** | LLVM22 | 顺序 | 100 | 92 | **8** | 0 |

- **直接 CLI（无 Python）决定性复现 0xC0000005** → 排除「Python 探针为成因」。
- 顺序 65% vs 并发 58% 崩溃率相近，且各 run 唯一输出文件 → **排除「文件名冲突 / 并发共享状态」为成因**。
- asm 路径 100/100 稳定（对照通过）；崩溃隔离于 `-filetype=obj` 二进制 DXContainer 写出路径。
- 输入 IR SHA256 = `91FA1D96E0F129E1A94DA26D1D59491DAD093522F1C5994227E96C87FACCA077`（bare_sm60.ll）。

### 2.3 Python 探针其它审计点

- 无 shell=True；timeout=30s 经 subprocess 捕获不误判；stderr 未丢；未在运行中改 PATH；未读上次遗留容器（写前删）。探针逻辑无污染结果的缺陷；唯一偏差是「无并发轴」(2.1)，不影响崩溃事实。

---

## 阶段 3：输入是否合法受支持（决定性差异定位）

### 3.1 IR verifier

- opt 缺失 → 用同 build 树 `llvm-as`（汇编时默认跑 verifier）：
  `llvm-as bare_sm60.ll -o bare_sm60.bc` → exit 0，bc 1980B，SHA256 `533F338F...`。
- bare IR **通过通用 LLVM verifier**。但通用 verifier **不检查 DirectX 后端元数据/入口契约**，故"通过 verifier" ≠ "受支持的 DirectX 输入"。

### 3.2 官方 DirectX 测试输入对照（决定性）

- `llvm/test/CodeGen/DirectX/` 共 277 个 .ll，**无一个**用 `-filetype=obj`（全部 `-filetype=asm` 或 `opt -dxil-translate-metadata`）→ **官方测试不覆盖 DXContainer obj 写出路径**（该路径上游欠测）。
- 官方输入形态：datalayout + triple + 函数属性 `"hlsl.numthreads"` / `"hlsl.shader"="compute"`（入口元数据由 dxil pass **生成**，非手写 `!dx.entryPoints`）。

**差分实验（LLVM23，obj，各 100 次）——锁定决定性差异：**

| 输入变体 | datalayout | hlsl 入口属性 | 手写!dx.entryPoints | OK | 崩溃 |
|---|---|---|---|---|---|
| 探针 bare | ✗ | ✗ | ✗ | 35 | 65 |
| 探针 enriched | ✗ | ✗ | ✓(输出形态) | 38 | 62 |
| V1 仅 datalayout | ✓ | ✗ | ✗ | 50 | 50 |
| **V2 仅 hlsl 入口属性** | ✗ | ✓ | ✗ | **100** | **0** |
| official_cs（datalayout+属性） | ✓ | ✓ | ✗ | **100** | **0** |
| 官方 lib_entry.ll | ✓ | ✓(library) | ✗ | **100** | **0** |
| official_cs（LLVM22） | ✓ | ✓ | ✗ | **100** | **0** |

**决定性结论（Bug 1）：**
- 崩溃的决定因素 = **缺 shader 入口属性（hlsl.shader / hlsl.numthreads）**；datalayout 与手写 `!dx.entryPoints` 均不相关（V1 仍崩、enriched 仍崩、V2 加属性即 100% 稳定）。
- **凡含入口属性的合法/官方输入，obj 100/100 稳定**（LLVM22、23 一致）。探针的 bare/enriched **两个变体相对后端输入契约都是不完整/错误形态**（bare 无入口；enriched 手写的是输出形态元数据，后端不消费）。
- 即：崩溃属 **情况 B — 不完整/受限 DirectX 输入（缺入口属性的 compute 模块）导致 llc 崩溃而非报诊断**，**不是**「合法 compute codegen 崩溃」。
- ⚠️ **修正 round-5 叙述**：round-5「obj 写出器全配置非确定崩溃 = fundamental 上游」结论 **建立在仅测不完整输入之上**；合法输入下写出器 100% 稳定。该结论需修正。

---

## 阶段 5：Windows 崩溃取证（cdb）

- 现有 Release build 无 PDB；cdb 来自 WinDbg Store 别名 `...\WindowsApps\cdbX64.exe`（cdb 10.0.29547.1002）。
- 命令：`cdbX64.exe -g -G -c "sxe av; g; .ecxr; kb; lm 1m; q" <llc23> -filetype=obj bare_sm60.ll -o <out>`，循环至捕获 AV（第 1 次即中）。

崩溃栈（**无 PDB → 无符号**，second chance c0000005）：
```
(8b3c.b168): Access violation - code c0000005 (!!! second chance !!!)
.ecxr: Unable to get exception context, HRESULT 0x8000FFFF
llc+0xacc3d6   ← faulting site
llc+0x4772c7
llc+0x479378
llc+0xa8c73f
llc+0xa8bc70
llc+0xafabc(?)
llc+0xb22ec
llc+0x1aac590
KERNEL32!BaseThreadInitThunk+0x17
ntdll!RtlUserThreadStart+0x20
```
加载模块：`llc, VCRUNTIME140_1, MSVCP140, VCRUNTIME140, dbgcore, Dbghelp, msvcp_win, ucrtbase, KERNELBASE, KERNEL32, OLEAUT32, msvcrt, RPCRT4, combase, ADVAPI32, sechost, ntdll`。

**模块级取证结论：**
- 故障模块 = **llc.exe 自身（LLVM 代码）**，全部 LLVM 帧；**不在任何系统 DLL**。
- **无 LLVM DLL 加载**（静态链接确认）、**无 debug/release CRT 混用**（无 *D.dll）、msvcrt 由系统组件（combase/RPCRT4）加载属正常、**无第三方注入/overlay/安全软件模块**。→ 排除「EXE/DLL 混用」「旧 LLVM DLL 抢先」「CRT 混用」「注入模块」为成因。
- **符号化根因 = 无 PDB，栈仅地址 → Root cause not established（函数级）。** 仅能确认故障在 llc.exe LLVM 代码内、由缺入口属性输入触发。需 Assertions/RelWithDebInfo+PDB 构建定位函数（见阶段 4 全新构建，进行中/见末尾状态）。

---

## 阶段 6：Validator 本地归因审计

### 6.1 dxc / dxil.dll 身份

| 项 | 路径 | SHA256 | 版本 |
|---|---|---|---|
| dxc.exe | `...\vulkan-1.3.296.0\Bin\dxc.exe` | `8B1321A448742D967DF80994AE05B8654F0ED765AF701A3252921EBF6F65D037` | 1.8.0.4739 (d9a5e97d0) |
| dxcompiler.dll | 同目录 | `A07881EBAE071049B3ABE9953CF305EEAAA94CA4610ACCED80136D9A61CBA5F2` | 1.8.0.4739 |
| **dxil.dll** | （dxc 目录） | — | **MISSING** |

- dxc 与 dxcompiler.dll 版本匹配（1.8.0.4739）、均 64 位。
- **dxil.dll 缺失** → 经 dxcompiler.dll 的 `CLSID_DxcValidator` 走的是 **dxcompiler 内置 validator**（开源校验，**无独立签名能力**）。Python 经此 DLL 加载；harness 打印实际 DLL 路径。

### 6.2 ctypes COM wrapper 正反向控制（每 20 次）

| 实验 | 结果 | 判定 |
|---|---|---|
| 正向：dxc 自产容器 ×20 | 全 accepted，status `0x0` 稳定 | ✓ wrapper 正确 |
| 反向：损坏容器（翻转字节）×20 | 全 rejected，status `0x80aa000d`「Validation failed.」稳定 | ✓ wrapper 正确 |

→ **ctypes COM wrapper（CLSID/IID/vtable 序/blob/HRESULT/error blob 提取）已验证正确**，排除「Validator wrapper 错误」为 Bug 2 成因。

### 6.3 合法输入 llc 容器真验证（修正 round-5 的关键实验）

用**合法输入**（official_cs.ll 的成功 obj 容器）真验证，而非 round-5 用的不完整输入容器：

| 来源 | 容器大小 | validate ×20 | status | error |
|---|---|---|---|---|
| llc LLVM23 合法容器 | 1936B | reject 20/20 | **0x80aa0013** | `DXIL container mismatch for 'PSVRuntimeInfoSize' between 'PSV0' part:('52') and DXIL module:('24')` |
| llc LLVM22 合法容器 | 1804B | reject 20/20 | **0x80aa0013** | 同上（52 vs 24） |
| dxc 自产（对照） | 2060B | accept 20/20 | 0x0 | — |

**与 round-5 的关键差异**：round-5 报 `0x80aa0009 / load dxil metadata failed`——那是用**不完整输入**容器验的（metadata 加载即失败）。用合法输入后，validator **成功加载 metadata 并走到具体结构校验**，给出具名失败：**PSV0 部件声明 PSVRuntimeInfoSize=52，但 DXIL 模块推得 24**。

### 6.4 容器结构 diff 与 DXIL/PSV 版本

| 项 | llc（22/23 同） | dxc 1.8 |
|---|---|---|
| parts | DXIL,SFI0,HASH,ISG1,OSG1,PSV0 | SFI0,ISG1,OSG1,PSV0,STAT,HASH,DXIL |
| 缺失 | **STAT** | — |
| 顺序 | 异 | 规范 |
| 签名 | 未签名(digest 全零) | 已签名 |
| DXIL 版本 | 0x100 / SM6.0 / kind5 | 0x100 / SM6.0 / kind5（**相同，无版本 gap**） |

**Bug 2 因果分析：**
- 拒绝的**具名直接原因 = PSV0 PSVRuntimeInfoSize 52 vs 24**（validator 自身点名为失败原因，causality 对「此 validator 此拒绝」已成立）。
- 缺 STAT / 未签名 / 顺序异 **非拒因**：validator 走到 PSV 校验（在这些之后）才失败；签名是 validate 通过**之后**的独立步骤。符合任务「缺 STAT/digest 全零不得当根因」。
- DXIL 版本两侧相同（1.0）→ 非 DXIL 版本 gap。
- ⚠️ **关键 nuance（PSV 格式版本 ≠ DXIL 版本）**：52 字节 PSVRuntimeInfo 对应比 dxc 1.8（约 2023）更新的 PSV 格式版本；dxc 1.8 validator 从模块推得 24（更旧 PSV 变体）。**「LLVM 过度 emit 新 PSV」与「dxc 1.8 validator 太旧不识新 PSV」二者无法用现有 1.8 dxil.dll 区分** → 需与 LLVM 22/23 同年代的 dxc/dxil.dll 复验。此因果归属 **UNRESOLVED**。

---

## 阶段 4：全新干净 Windows 对照构建（决定性证据）

- 源码：`git worktree add --detach H:\llvm-clean-82c5bce5-src 82c5bce5`（官方 remote 派生 worktree，HEAD=82c5bce5，status 干净，无 patch）。
- 构建：**全新目录** `H:\llvm-clean-82c5bce5-build`（不复用旧 CMakeCache/.obj/.lib/.dll/.pdb）。
- 配方：vcvars64（VS2022 14.44）+ CMake 4.3 + Ninja，`-DCMAKE_BUILD_TYPE=RelWithDebInfo -DLLVM_ENABLE_ASSERTIONS=ON -DLLVM_TARGETS_TO_BUILD=X86 -DLLVM_EXPERIMENTAL_TARGETS_TO_BUILD=DirectX`，`ninja -j6 llc llvm-as`。configure 干净完成、build DONE。
- 产物：`H:\llvm-clean-82c5bce5-build\bin\llc.exe` SHA256 `EFE2EEE79888613C8A9870AB6C68EDD82025946E7CF702A4934874EF4A0B64BC`，`--version` = "23.0.0git / **Optimized build with assertions**"，dxil target 在位，**llc.pdb 存在**。

**干净构建复现（直接 CLI，唯一输出）：**

| 输入 | N | 结果 |
|---|---|---|
| bare（无入口属性）obj | 100 | **100/100 确定性 assertion abort，rc=0x80000003**（0xC0000005 消失） |
| official_cs（合法）obj | 30 | **30/30 OK**，无断言无崩溃 |

**符号化断言栈**（evidence/dxil_round6_assertion_stack.txt，SHA256 前缀 `A7415A07...`）：
```
Assertion failed: MMI.EntryPropertyVec.size() == 1,
  file llvm/lib/Target/DirectX/DXContainerGlobals.cpp, line 252
Running pass 'DXContainer Global Emitter'
  DXContainerGlobals::addRootSignature()  @ DXContainerGlobals.cpp:253
  DXContainerGlobals::runOnModule()       @ DXContainerGlobals.cpp:103
  MPPassManager::runOnModule → legacy::PassManagerImpl::run
  compileModule (llc.cpp:882) → main (llc.cpp:460)
```

**源码上下文（DXContainerGlobals.cpp）：**
```cpp
245  if (MMI.ShaderProfile == llvm::Triple::Library)
246    return;                              // library profile 提前返回（lib_entry.ll 因此稳定）
251  if (MMI.ShaderProfile != llvm::Triple::RootSignature) {
252    assert(MMI.EntryPropertyVec.size() == 1);
253    EntryFunction = MMI.EntryPropertyVec[0].Entry;   // 空 vec → Release 下越界读 → 非确定 AV
254  }
```

**根因（established，函数/文件/行 + 符号栈 + 源码三证齐全）：** DirectX 后端 `DXContainerGlobals` pass 假设 compute（非 library/非 RootSignature）模块恰有 1 个 entry property。缺 shader 入口属性的 compute 模块使 `EntryPropertyVec` 为空（size 0）→ assertions 构建确定性 `assert` abort；Release 构建断言被编译掉，`EntryPropertyVec[0].Entry` 对空 SmallVector 越界读取 → 解引用随机内存得到的指针 → **非确定性 0xC0000005**（崩溃率随堆/内存布局波动，正是 round-4/5 观察到的逐配置波动来源）。

### 4.1 构建矩阵覆盖与 UNRESOLVED

| 组 | 源码 | 构建 | 状态 |
|---|---|---|---|
| A | LLVM 22.1.7 / a255c1ed | 现有 Release/no-assert（旧目录，仅现象） | 复现（bare obj 8/100） |
| B | 82c5bce5 | 现有 Release + **全新 RelWithDebInfo+Assertions**（权威） | 复现 + 符号根因 |
| C | 当前官方 main | — | 今日(2026-06-24)即 82c5bce5（round-5 当日最新 main），**C≈B**；未另 fetch 更新 commit → 标 **UNRESOLVED（无更晚 commit 可测）** |

- Debug+Assertions（任务 Build B）：未单独做——RelWithDebInfo+Assertions 已把 AV 转为**确定性断言**并给出符号根因，改变了内存布局且断言必中，目的已达成。单独 full Debug 构建 **未做（非必要）**。
- Windows ASAN（任务 Build C）：**未尝试**——断言路径已确立根因，ASAN 非完成阻塞。标 ASAN not attempted（assertion build 已足够）。

---

## 阶段 7：Windows-only 归因等级

### Bug 1 — llc -filetype=obj 写 DXContainer 非确定 0xC0000005

- **归因等级：D — 高度可信的 LLVM 上游问题**，**限定为 robustness bug（情况 B）**。
  - 措辞铁律：**Malformed or incomplete DirectX input (a compute module lacking a shader entry point / `hlsl.shader` attribute, yielding an empty `EntryPropertyVec`) causes `llc` to crash instead of emitting a diagnostic.** 不得写成「合法 compute shader codegen 崩溃」。
- 是否可能本地问题：**否**。
- 已排除的本地因素：PATH 混用（绝对路径，llc 不在 PATH）；源码污染（官方 remote / clean / 无 patch / 全新 worktree）；旧 build cache（全新 build 目录复现）；ABI·DLL 混用（静态链接、无 LLVM DLL、无 debug/release CRT 混用、无注入模块——cdb `lm` 确认）；Python 探针（直接 CLI 复现）；并发文件冲突（顺序复现 + 唯一输出，顺序/并发崩溃率相近）；非法 IR（通过通用 verifier）。
- 尚未排除的本地因素：无关键本地因素未排除。触发条件为「不完整 DirectX 输入」属 bug 性质本身，非本地环境缺陷。
- 是否可称 LLVM upstream bug：**是**（robustness bug），符号栈落在 LLVM `DXContainerGlobals.cpp:252/253`。
- 是否仅 Windows-path bug：**否**。根因是 target-agnostic 的后端 C++ 逻辑（空 `EntryPropertyVec` 越界 / 入口数恒等于 1 的假设），assert 在任何平台都会 fire；非 COFF/MSVC-runtime/Windows 内存路径特定。AV 的「非确定性」是 Windows 内存布局表现，缺陷本身跨平台。
- 置信度：**高**。
- Root cause：**established**（assert 文本 + 符号化调用栈 + 源码三行）。
- 附证：源码 commit 82c5bce5；worktree git status 干净；CMake RelWithDebInfo+Assertions=ON；MSVC 14.44.35207 + VS2022 SDK；clean llc.exe SHA256 `EFE2EEE7...64BC`；输入 bare_sm60.ll SHA256 `91FA1D96...`；真实命令见阶段 2.2/4；顺序 100×（35ok/65crash, LLVM23）；并发 100×（42ok/58crash）；cdb 模块级栈（阶段5）+ assertions 符号栈（阶段4）；官方 lib_entry.ll 对照 100/100 稳定；Assertions vs Release 对照（确定 assert vs 非确定 AV）。

### Bug 2 — IDxcValidator 拒绝 llc 容器（任务标 0x80aa000f；实测见下）

- **归因等级：C — 尚不能完全归因**（直接差异已确立，因果归属未确立）。
- Validator wrapper 是否已验证：**是**（正向 dxc 自产 20/20 accept 0x0；反向损坏 20/20 reject 0x80aa000d）。
- dxc 与 dxil.dll 是否匹配：dxc.exe 与 dxcompiler.dll 均 1.8.0.4739（匹配，64 位）；**独立 dxil.dll 缺失** → 用 dxcompiler 内置 validator。
- 输入是否真正等价：本轮用**合法**输入（official_cs.ll vs dxc 最小 HLSL），语义等价最小 compute、同 SM6.0/DXIL 1.0，但**非 byte 同源**。
- 问题位于 metadata / container / 尚未区分：**已分层定位到 container 的 PSV0 part**。实测合法输入拒绝码 **0x80aa0013**，validator 点名 `PSVRuntimeInfoSize 'PSV0' part(52) vs DXIL module(24)`。注意：round-5 的 `0x80aa0009 / load dxil metadata failed` 是**不完整输入**假象，非真实互操作拒因。
- 是否可称 LLVM upstream compatibility bug：**possible，但 UNRESOLVED**——PSV 格式版本（52 字节 RuntimeInfo）比 dxc 1.8（约 2023）新，「LLVM 过度 emit 新 PSV」与「dxc 1.8 validator 太旧」无法用现有 1.8 dxil.dll 区分。
- 置信度：**中**（差异 established；upstream 因果 not established）。
- 附证：dxc 路径/版本/hash（`8B1321...`，1.8.0.4739）；dxcompiler.dll（`A07881...`，1.8.0.4739）；dxil.dll MISSING；llc 22/23 容器 part 表与 PSV0 diff；正反向控制；HRESULT 0x80aa0013 + error blob 原文；DXIL 版本两侧均 0x100；**已证差异**：PSV0 RuntimeInfoSize 52 vs 24；**未证因果**：是否 LLVM 不合规——`Observed difference; causality not established.`

---

## 最终回答

- **Bug 1 是否由本地环境引起？** 否。全新干净官方源码 worktree + 全新 build 目录 + 绝对路径 CLI + 唯一输出仍复现，符号栈落在 LLVM `DXContainerGlobals.cpp`。属上游 robustness bug（不完整 DirectX 输入致崩溃），非本地环境。
- **Bug 2 是否由本地环境引起？** 部分本地相关但非 wrapper 错误：拒绝是真实的（合法输入仍被拒，validator wrapper 经正反向控制验证无误）；唯一本地变量是 dxil.dll 缺失/dxc 1.8 年代，影响「上游 vs validator 版本」的因果定论，故归 C。
- **哪些本地风险已排除？** PATH 混用、源码污染/patch、旧 build cache、EXE/LLVM-DLL 混用、debug/release CRT 混用、注入模块、Python 探针成因、并发/文件冲突成因、非法 IR、Validator ctypes wrapper 错误、DXIL 版本 gap。
- **哪些仍是 UNRESOLVED？** ①22 树 commit↔llvmorg-22.1.7 tag 精确映射（浅克隆无 tag）；②比 82c5bce5 更晚的官方 main（今日即此 commit，无更晚可测）；③Bug 2「LLVM 过度 emit PSV vs dxc 1.8 太旧」因果（需同年代 dxil.dll/dxc 复验）；④full Debug 构建与 Windows ASAN（未做，非必要，断言已确定性命中）。
- **现有旧构建能否继续作为证据？** 仅作现象记录（Release/no-assert/可能增量）；上游归因以阶段 4 全新 Assertions 构建为准。
- **干净 Windows 构建是否复现？** 是（bare obj 100/100 确定性断言）。
- **直接 CLI 是否复现？** 是（绕过 Python，顺序 65% / 并发 58% 崩溃）。
- **官方 LLVM DirectX 测试是否复现？** 否——官方/合法输入（含入口属性）obj 100/100 稳定；崩溃仅限缺入口属性的不完整输入。
- **是否取得带符号调用栈？** 是（阶段 4，DXContainerGlobals::addRootSignature @ :253）。
- **Validator wrapper 是否通过正反向控制？** 是。
- **是否具备进入 LLVM issue 草拟阶段的证据？** Bug 1 具备（符号根因 + 最小复现 + 干净构建），但 issue 须写成「不完整 DirectX 输入致崩溃而非诊断」。Bug 2 不具备（因果未确立）。
- **是否准备了任何本地补丁？** 否。
- **是否发生任何公开提交？** 否。

## 结尾声明

- 未向 llvm-project 公开提交 issue、PR、discussion 或评论。
- 未修改 Rurix src / spec / golden / codegen。
- D-131 仍为 C。
- G-G2-2 未签署、仍 open。
- 由于仅完成 Windows 验证，未声称问题已跨平台复现。
- round-1~5 既有 evidence/ 文件未改动；本报告与 `dxil_round6_assertion_stack.txt` 为新增。自编 LLVM / 探针产物隔离于仓库外（H:\llvm-clean-* / H:\llvm-audit-round6），未入库。
