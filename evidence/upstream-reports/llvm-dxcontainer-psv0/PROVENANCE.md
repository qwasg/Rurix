> **Status: DRAFT — do NOT file.** Owner review gate; agent does not file externally.

# PROVENANCE — LLVM DXContainer PSV0 上游报告包

## 本包形态与口径调和(须 owner 知悉)

- **单目录 · 双草稿**:本目录 `evidence/upstream-reports/llvm-dxcontainer-psv0/` 是「三连备包」
  的第三包(契约 `EA1_CONTRACT.md` YAML `upstream_report_packs` + G-EA1-7 记「LLVM DXContainer
  PSV0」**单包**);目录内含**两份草稿**(`ISSUE_DRAFT_psv0_writer.md` + `ISSUE_DRAFT_empty_signature.md`),
  调和 `EA1_PLAN.md` §7 B2(行 114)记「PSV0 写出器 + 空签名**两包各一**」。即:对外**一个包**(一个
  目录),内部**两个议题**(PSV0 版本派生 / 图形空签名),分别对应两个不同的上游条目。此口径分歧为契约
  与计划文档间的既有措辞不一致,**surface 给 owner**,本包不自行改判文档。
- **两份草稿均非「待 owner 提报的新草稿」**——两个 bug 都已在上游有既存条目(见下),故两份文档的
  `DRAFT — do NOT file` 标头一律解释为「**不发起任何新提报**」,而非「待 owner 首次提报」。这与 Godot
  / VVL 包的「待 owner 亲自首报」语义**不同**,是本包的关键区别,单列于此。

## 上游既存条目(关键 — 与「do NOT file」纪律的张力,surface 给 owner)

| 议题 | 上游条目 | 类型 | 记录来源 |
|---|---|---|---|
| PSV0 版本派生(`0x80aa0013`) | **PR [#205546](https://github.com/llvm/llvm-project/pull/205546)**(OPEN,base main,+93/-4,4 文件) | agent 于 G2.2 期(2026-06-24)**已提报**的上游 PR | `registry/deferred.json` RD-011 history(2026-06-24) |
| 图形着色器空签名(`ISG1/OSG1 elemcount=0`) | **issue [#90504](https://github.com/llvm/llvm-project/issues/90504)** | 上游**既存** issue(后端源码 `// FIXME` 直接引用) | `evidence/dxil_slice3_rxs0159_sig_disasm_round8.md` §5 + RD-011 history(2026-06-25) |

> **治理张力(呈 owner,agent 不自决)**:EA1 契约把 `upstream_filing` 列为 out_of_scope、把三包定位为
> 「DRAFT — do NOT file / agent 只备包 / owner 亲自提报」;但 PSV0 这一 bug **已由 agent 于 G2.2 期提报为
> 上游 PR #205546**(D-406 v2.0 完全自主化授权下发生)。为一个已提报的 bug 备「do NOT file 草稿」语义上
> 需要澄清——本包据此重构为「**已提报记录 / 补充材料**」而非「待提报草稿」,并把两处上游条目的存在明确
> surface。owner 可裁:本包是否算满足 G-EA1-7「issue 草稿全文」(草稿在,但形态是记录),以及 agent 已
> 提报 PR #205546 与「AI 不对外提报」纪律的关系如何在治理层澄清。

## 来源(素材 → 草稿字段映射,全部 tracked 在 main)

`ISSUE_DRAFT_psv0_writer.md`:
- **定罪(非 validator 版本 gap)**:`evidence/dxil_path_spike_report_round7.md` §4/§5(同一 2026 签名
  validator 接受 DXC 的 52B PSV、拒 llc 的 52B PSV → 排除「dxc 太旧」,坐实 llc 容器内部不一致)。
- **root cause 到函数/行 + PoC diff + 前后 validator 对照**:`evidence/dxil_path_spike_report_round8.md`
  §2(写出侧 `DXContainerGlobals.cpp:388-389` / 期望侧 `DXILMetadataAnalysis.cpp` + `DXILTranslateMetadata.cpp`
  链)、§3(14 行 PoC diff 全文)、§4(reject 0/25 → accept 25/25 表)。
- **可复现 recipe + 上游 PR 实况 + 精度更正**:`spike/dxil-path-probe/dxil_psv_patch_recipe.md`
  §2/§3/§4 + §8(PR #205546 实况;「14 行单点」修正为「局部修复含语义涟漪」:getPSVVersion() helper +
  两既有测试 valver 1.7→1.8 + 新测试)。草稿「Proposed fix」的最终形态说明即取自此。
- **机器证据**:`evidence/dxil_path_spike_20260624_r7.json` / `_r8.json`(schema 校验;SHA256 记录)。

`ISSUE_DRAFT_empty_signature.md`:
- **签名 part 实证 + root cause + #90504 引用**:`evidence/dxil_slice3_rxs0159_sig_disasm_round8.md`
  §3/§4(vs_io/ps_io 经 patched llc 产 DXContainer,IDxcValidator 25/25 accept 但 ISG1/OSG1 size=8
  elemcount=0)、§5(`addSignature()` 无条件空签名 + `// FIXME support graphics shader` + `Signature::addParam`
  强制 Register/Mask/ExclusiveMask 无调用点)、§6(硬规则 5 边界:让 SV 真达 = FFI ABI 禁区)。
- **邻接崩溃观察(round4/5 Bug1)**:`evidence/dxil_path_spike_report_round4.md` /
  `evidence/dxil_path_spike_report_round5.md`(obj DXContainer 写出器非确定性崩溃 `0xC0000005`,
  asm 文本路 96/96 稳)——草稿仅作同区域鲁棒性信号引用,未在本包重新定位。

## 本地复验(零下载,measured 2026-07-17)

- 见 `repro_log_20260717.md`:on-disk `llc.exe`(SHA256 `D11CD2A1…`,LLVM 23.0.0git patched)+
  `H:\dxc-round7\extracted\bin\x64\dxv.exe`(1.9.2602.24)+ `H:\llvm-audit-round6\official_cs.ll`。
- **reject(bug)**:byte 保真的 pre-patch 容器 `H:\llvm-audit-round6\official_cs.obj`(1936B/`76A3D75A`,
  PSV0=52)→ dxv `mismatch 'PSV0' part:('52') vs DXIL module:('24')` → `Validation failed.`(exit 1)。
- **accept(fix)**:on-disk patched llc 重 emit → `official_cs.obj`(1892B/`019E3A51`,PSV0=24)→ dxv
  `Validation succeeded.`(exit 0)。
- **行为漂移诚实记录**:on-disk llc 是**已修**二进制(产 accept/24),故 reject 从 byte 保真的 pre-patch
  容器复验(非重跑 unpatched llc,该未修二进制不在盘上)。详见 repro_log §Behavior-drift note。

## 素材时效与漂移

- round-7/8 报告 §9 记「未向 llvm-project 公开提交」是 spike 时点(2026-06-24 提交 PR 之前)的诚实快照;
  提报发生在其后(RD-011 history)。**引用状态一律以 tracked `registry/deferred.json` RD-011 为准**,勿
  被过期 memory 或 spike 报告快照误导。
- 上游 PR #205546 / issue #90504 的 OPEN 状态为记录时点(截 2026-06-24/25)信息;上游随时可能 merge/关闭,
  owner 复核时应核实现状(**本包不联网核**,截止本会话未核 PR/issue 现状)。

## 提报纪律

- agent 只备包(本目录即备包产物);两处上游条目已存在,**do NOT file = 不发起任何新提报**(issue/PR/
  discussion/评论);对 PR #205546 / issue #90504 的任何后续互动由 owner 复核后亲自执行,AI 不对外提交。
- EA1 契约将 `upstream_filing` 列为 out_of_scope:本包在 EA1 期内仅作证据归档(D-EA1-8 / G-EA1-7)。
