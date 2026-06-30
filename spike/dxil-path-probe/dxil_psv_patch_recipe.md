# A 路 DXIL 后端 round-8 LLVM PSV patch — 可复现 recipe + dev 工具链解锁说明

> 跟踪条目:`registry/deferred.json` **RD-011**(受控、dev-only、临时工具链偏差)。
> 决策依据:D-131 = **A**(LLVM DirectX 后端直接 emit DXIL),RFC-0003 §9 Q-D131(C→A 回填)/ 13 §D-131(v1.3)。
> 取证来源:`evidence/dxil_path_spike_report_round8.md`(root cause §2 + patch diff §3 + 前后 validator 对照 §4)。
> Provenance:`Assisted-by: kiro:claude-opus-4-8`。

---

## 0. 这是什么 / 边界

A 路依赖的上游 LLVM 缺陷(round-8 established):`DXContainerGlobals.cpp:388-389` 的
`addPipelineStateValidationInfo()` 调 `PSV.finalize/write` **不传 Version** → 取默认
`uint32_t::max()` → 无条件按最高 PSV 版本(v3=52B)写;而缺 `dx.valver` 的模块令
validator 期望 v0(24B)→ `0x80aa0013 PSVRuntimeInfoSize` mismatch。14 行单函数 PoC patch
按模块 `MMI.ValidatorVersion` 派生 PSV 版本,令写出与 validator 期望对齐。

**严格边界(同 RD-011)**:
- patch 二进制 / 重建 llc **不入库**,仅隔离于仓库外(见 §4)。本仓库内只存本 recipe(diff 文本 + 步骤)。
- **不静默改** committed D-205 pin(`C:\Program Files\LLVM`)/ `toolchain.rs` / `src/`。dev 使用经**显式 env 覆盖**(§5)。
- 本偏差为**临时**:退役条件 = 上游 merge + release + D-205 pin bump(D-205 真 bump 属 owner 独立决策 + 独立 errata)。
- 本 recipe 不裁 A/B(已由 agent 裁 A)、不签 G-G2-2(device 真跑 + golden 仍 open)。

## 1. 隔离环境(仓库外,不入库)

| 项 | 路径 | 说明 |
|---|---|---|
| LLVM fork worktree | `H:\llvm-clean-82c5bce5-src` | 官方 remote 派生 worktree,HEAD=82c5bce5(LLVM 23.0.0git),patch 前 `git status` 干净 |
| LLVM build(assertions+PDB) | `H:\llvm-clean-82c5bce5-build` | RelWithDebInfo+Assertions,含 `bin/llc.exe` |
| patch 文件 | `H:\dxil-round8\dxil_psv_version_round8.patch` | diff 全文见 §3(SHA256 见 `evidence/dxil_path_spike_20260624_r8.json` round8_detail) |
| 2026 签名 validator | `H:\dxc-round7\extracted\bin\x64` | dxcompiler.dll + dxil.dll + dxv.exe 1.9.2602.24 |
| 合法输入 | `H:\llvm-audit-round6\official_cs.ll` | 289B,SHA256 `8FA89702...`(无 dx.valver) |
| round8 工作目录 | `H:\dxil-round8` | patch / 容器 / 探针 / 日志 |

> 路径为本机 spike 现场约定;他机复现按 `evidence/dxil_path_spike_report_round{6,8}.md` 的环境/重建清单备等价隔离目录即可,**不要**写入 `C:\Program Files\LLVM`(D-205 pin)。

## 2. 缺陷定位(established 到函数/行)

- 写出侧:`llvm/lib/Target/DirectX/DXContainerGlobals.cpp:388-389` 调 `PSV.finalize(MMI.ShaderProfile)` / `PSV.write(OS)` 不传 Version。
- 默认值:`llvm/include/llvm/MC/DXContainerPSVInfo.h:74-82` 默认 `Version = std::numeric_limits<uint32_t>::max()`。
- 选 size:`llvm/lib/MC/DXContainerPSVInfo.cpp:73-90` `write()` switch,max 命中 `default` → `InfoSize = sizeof(v3::RuntimeInfo) = 52`(v0=24/v1=36/v2=48/v3=52)。
- 期望侧:`official_cs.ll` 无 `dx.valver` → `DXILMetadataAnalysis.cpp:43-50` ValidatorVersion 空 → `DXILTranslateMetadata.cpp:291-293 emitValidatorVersionMD` 提前 return 不写 `dx.valver` → validator 推 valver 0.0 期望 PSV v0=24B。
- 结果:写 52 / 期望 24 → `0x80aa0013`。

## 3. patch diff 全文(14 行净逻辑,单文件单函数)

改 `llvm/lib/Target/DirectX/DXContainerGlobals.cpp` 的 `addPipelineStateValidationInfo()`;
**不动**共享 PSV 基础设施(`DXContainerPSVInfo.cpp` / `DXContainer.h` 版本-size 表与结构均 byte-unchanged)。

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

> 注释里的 `SPIKE(RD-010)` 是 round-8 取证期标记;偏差跟踪正式条目为 **RD-011**(本 recipe 入库时 D-131 已裁 A)。

## 4. apply → 增量重建 → validator 复验(分钟级)

```bat
:: 1) 打 patch(在 fork worktree)
cd /d H:\llvm-clean-82c5bce5-src
git apply H:\dxil-round8\dxil_psv_version_round8.patch
:: 2) 增量重建 llc(VS2022 14.44 + Ninja;仅 3 步:编 DXContainerGlobals.obj → 链 lib → 链 llc)
call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
cd /d H:\llvm-clean-82c5bce5-build
ninja -j6 llc
:: 3) emit + 验证
bin\llc.exe H:\llvm-audit-round6\official_cs.ll -filetype=obj -o H:\dxil-round8\post_official_cs.obj
```

复验(IDxcValidator ×25 复用探针 + 独立 dxv.exe):
```bat
set RURIX_DXC_NEW_DIR=H:\dxc-round7\extracted\bin\x64
py -3 H:\dxil-round8\r8_probe.py H:\dxil-round8\post_official_cs.obj
H:\dxc-round7\extracted\bin\x64\dxv.exe H:\dxil-round8\post_official_cs.obj
```

预期(round-8 实测):

| 容器 | PSV0 InfoSize | IDxcValidator ×25 | dxv.exe |
|---|---|---|---|
| pre(未 patch) | 52 | **0/25** accept,`{0x80aa0013:25}` | 5/5 reject |
| post(patch 后,无 valver) | 24 | **25/25** accept,`{0x0:25}` | 5/5 `Validation succeeded.` |
| post(+ `dx.valver=!{1,8}`) | 52 | **25/25** accept,`{0x0:25}` | 5/5 `Validation succeeded.` |

双向对齐:写出版本随模块 `dx.valver` 跟随(缺→v0/24,1.8→v3/52),均被同一 2026 签名 validator 接受。

## 5. dev 工具链解锁(dev-only / 临时 / 不改 committed D-205 pin)

A 路下游开发(PR-C2 codegen)需要一个能产合规 DXIL 的 llc。在上游 merge 前,**经显式
环境变量覆盖**定位 §4 重建出的 patched llc 绝对路径,**不改** committed D-205 pin
(`C:\Program Files\LLVM`)、不改 `toolchain.rs`、不改 `src/`:

```bat
:: dev-only:指向仓库外 patched llc 绝对路径(临时,退役即删)
set RURIX_LLC=H:\llvm-clean-82c5bce5-build\bin\llc.exe
```

约定:
- 工具链解析 DXIL 后端的 llc 时,**优先**读 `RURIX_LLC`(若设置且文件存在),否则回落
  committed D-205 pin。该 env 覆盖仅在 dev 机生效,**不写入仓库配置 / CI / pin**。
- patched llc 不入库;CI 与默认构建仍走 committed D-205 pin(未受影响)。
- 该覆盖是 **RD-011 跟踪的临时偏差**:上游 merge + release + D-205 pin bump 后,删除
  `RURIX_LLC` 覆盖、直用 pin llc,并在 RD-011 history close 留痕。

> ⚠️ 上述 `RURIX_LLC` 解析接线属 PR-C2 codegen 范围(条款先于实现,硬规则 7);本 recipe
> 仅约定 dev 使用方式与边界,**不**在本勘误 PR 落 `src/` / toolchain.rs 实现。

## 6. 上游并行 + 退役

- 同步向 llvm-project 提交上游 PR(root cause §2 + fix §3 diff + 前后 validator 对照 §4 作为 PR 描述)。上游进展随 **RD-011** history 留痕。
- 退役条件(全满足):① 上游 merge 该 PSV 版本派生修复;② 进入 LLVM release;③ agent 裁 D-205 pin bump 到含修复版本(独立决策 + 独立 errata)。
- 退役动作:删除 dev 机 `RURIX_LLC` 覆盖与隔离 patch/llc;RD-011 status open→closed 并附退役证据。

## 7. 复现性自检清单

- [ ] patch apply 干净(`git apply` 无 reject)
- [ ] 增量 `ninja -j6 llc` 3 步成功,产 patched `bin/llc.exe`
- [ ] post(无 valver)PSV0 InfoSize = 24,IDxcValidator 25/25 accept(`{0x0:25}`)+ dxv.exe `Validation succeeded.`
- [ ] pre(未 patch)对照 PSV0 = 52,0/25 accept(`{0x80aa0013:25}`)
- [ ] 仓库内仅本 recipe(diff 文本),无 patch 二进制 / llc 入库;`C:\Program Files\LLVM` 未改

## 8. 上游 PR 提交 + round-8「14 行单点 PoC」精度更正(2026-06-24 追加,append-only)

> 本节为末尾追加更正段;§0~§7 既有内容 0-byte 不动。跟踪见 `registry/deferred.json` **RD-011** 同日 history。
> 上游 PR:**https://github.com/llvm/llvm-project/pull/205546**(OPEN,base main,+93/-4,4 文件)。

### 8.1 上游 PR 实况

| 项 | 值 |
|---|---|
| URL | https://github.com/llvm/llvm-project/pull/205546 |
| 状态 | OPEN(base main) |
| 规模 | +93 / -4,4 文件 |
| 本地验证 | DirectX lit 测试 465 全过;新测试 pre-fail / post-pass;2026 签名 validator post **25/25 accept** |

### 8.2 精度更正:最终 patch 非孤立 14 行单点

§3 与 round-8 报告把 patch 描述为「14 行单函数单点 PoC」。**实测推翻该精度**——最终上游 patch 形态:

- `getPSVVersion()` 辅助函数(按模块 `ValidatorVersion` 派生 PSV 版本,非内联进 `addPipelineStateValidationInfo()`);
- **2 个既有上游测试更正**:`test/CodeGen/DirectX` 的 `RuntimeInfoCS.ll` 与 `PipelineStateValidation.ll`,`valver 1.7 → 1.8`;
- 新增测试文件 `PSVVersionFromValidatorVersion.ll`。

**涟漪根因**:既有上游测试编码了「永远写 PSV v3、忽略 `dx.valver`」的 buggy 行为;修复 PSV 版本派生后,这些既有测试的期望值必然连带更新(否则既有测试反而失败)。故修复非孤立单点。

**判定收紧**:由 round-8 的「孤立单点 SHALLOW 浅修」修正为「**局部修复但含语义涟漪**」——触及既有测试编码的 buggy 行为、需社区评审认可,非孤立单点。

### 8.3 漂移声明与退役口径修正

- **本地 PoC ≠ 最终 merged 形态**:§3 本地 diff(`DXContainerGlobals.cpp` 内联 `valver → PSVVersion` 派生)与上游最终形态可能不同——reviewer 可能偏好显式断言 v2 而非 bump 既有测试 `valver`、或抽出独立 helper。
- **退役以上游 merged 形态为准**:RD-011 退役对齐 = **上游 merged 形态 + D-205 pin bump 到含修复版本**(非本地 PoC 形态);§6 退役条件不变,本节仅澄清「对齐目标 = 上游 merged 形态」。
- **不回改 round-8 evidence**:`evidence/dxil_path_spike_report_round8.md` 等为当时诚实快照,不可篡改门保护,**不回改**;本更正仅走本 recipe §8 追加 + RD-011 history 追加。
