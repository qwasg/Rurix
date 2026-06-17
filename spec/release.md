# Rurix 语言规范 — 发布产物语义面(原子分发 / 语言本体与 NVIDIA 再分发组件分离打包 / 签名清单与 SBOM 约定 / Release 层发布门;M8.4 起)

> 条款:RXS-0135 起续号预留(M8.4 发布产物语义面:编译器/运行时/标准库按版本**原子分发**与 content-tree 完整性 / **语言本体与 NVIDIA 再分发组件分离打包**(Attachment A 白名单最小集) / **签名清单约定**(Authenticode + 时间戳,of-record Azure Artifact Signing,验签先于上传) / **SBOM 约定**(SPDX 构建视图 + CycloneDX 发布视图) / **Release 层 hard-block 发布门**(签名/SBOM/许可审计/bench 严格任一缺失即阻断上传))。**复用 M6 包管理 content-tree/SHA-256/lock 既有形态(RXS-0090/0092/0094)与 M5.4 NVIDIA 再分发白名单审计,新增仅补发布产物缺口**。体例见 [README.md](README.md)。
> 依据:[../08_RUNTIME_AND_TOOLING.md](../08_RUNTIME_AND_TOOLING.md) §9(分发与签名 D-241,r6 全套结论照搬:`rurixup` 引导 + MSI + winget、按版本原子分发、语言本体与 NVIDIA 再分发组件分离打包、NVIDIA 仅 Attachment A 白名单最小集且完整 Toolkit/驱动/Nsight 永不捆绑、全部 EXE/DLL/MSI Authenticode + 时间戳经 OV 证书或 Azure Artifact Signing、构建生成 SPDX 发布附 CycloneDX、CI 强制许可白名单审计);[../14_ENGINEERING_DISCIPLINE.md](../14_ENGINEERING_DISCIPLINE.md) §8(CI 三层门禁第三层 **Release**:bench 严格模式 + hard block + 签名/SBOM/许可审计 + artifact 上传);[../10_GOVERNANCE.md](../10_GOVERNANCE.md) §6(工具链发布门);[../09_STDLIB_AND_ECOSYSTEM.md](../09_STDLIB_AND_ECOSYSTEM.md) §6/§7(用户产物分发指引与 DLL 搜索顺序);01 §6(MVP 验收)。授权:[../milestones/m8/M8_CONTRACT.md](../milestones/m8/M8_CONTRACT.md)(`in_scope: release_pipeline` / `spec_m8_clauses`,D-M8-4,G-M8-4 / G-M8-7,RD-001,`rfc_required: none`)+ [../milestones/m8/M8_PLAN.md](../milestones/m8/M8_PLAN.md) §4 M8.4 第 1 项 + [../milestones/m8/CI_GATES.md](../milestones/m8/CI_GATES.md) §3(Release 层门禁,步骤 38)。
> 档位:**Direct**(脚手架本前言为对 08 §9 / 14 §8 已锁定决策(D-241,r6)的初版条款化预告,纯追加且尚无 stable 面;**AI 无权自判 Direct**,判档以 M8_CONTRACT.md YAML 头 `rfc_required: none` 与上述授权为据,判档争议向上取严)。**Azure Artifact Signing 为 of-record 签名后端**(开工 owner 裁定确认;生产签名经 CI secret + 人工门控,本机/CI 不自动调用真实证书,带档复议按裁决留痕,不擅自切换);**NVIDIA EULA Attachment A 白名单逐项法律核对维持 `pending-human-review`**(AI 仅起草机器事实,法律签署由所有者/法务人工落笔,AI 不代签,对齐 [pipeline.md] 上游 M5.4 `redistribution_audit` 先例)。任何偏离已锁定决策、或触及 **cubin/fatbin 真分发(G1,M8 维持 PTX-only)** / **完整 Toolkit·驱动·Nsight 捆绑(许可红线 r6)** / **Python 原生嵌入(红线 1,SG-008,仅 C ABI/PYD 通道)** / **包 registry sumdb(D-312/G2,M8 维持 not_triggered)** 的条款,必须停下标注「需人工升档」,不在本文件自行落笔(10 §3,M8_CONTRACT §6 / out_of_scope)。**严禁 UB 节**(UB 为人类经 Full RFC 落笔的禁区,10 §7.5):发布产物完整性 / 签名状态 / 再分发面以 **content-tree SHA-256 + 验签判定 + 白名单审计的确定性机器事实** 定义,不以 UB 表述。
> 规范先行(AGENTS.md 硬规则第 7 条):**条款 PR 先于实现 PR**;缺条款的语义 PR 必须先补 spec。`ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 ≥1 测试锚定(`//@ spec: RXS-####`)。**本脚手架 PR 沿 README v1.15 toolchain.md / v1.20 stdlib.md / v1.25 interop.md / v1.27 cublas.md / v1.29 pipeline.md 先例:仅登记新文件名 + 预留区间,不落带编号裸条款头**——条款体(RXS-0135 起)与每条 ≥1 测试锚定随 M8.4 实现 PR 同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定)。

---

## 1. 范围与编号区间

本文件承载 **发布产物语义面**的语义条款(M8.4+,D-M8-4)。覆盖语义面:

- **原子分发与 content-tree 完整性**:编译器(`rurixc`/`rx`)、运行时(`rurix-rt`)、标准库按**版本**作单一原子分发单元;分发 bundle 以 **content-tree 规范化 SHA-256**(复用 M6 RXS-0090 内容树规范化 / RXS-0092 `rurix.lock` 精确解析图 / RXS-0094 vendor 离线复现形态)为完整性锚;**安装为全有或全无**(校验失败回滚,不留半装状态)——`rurixup` 引导器据此实现原子安装与按版本切换。
- **语言本体与 NVIDIA 再分发组件分离打包**:发布 bundle **分区**为「语言本体」(Rurix 自研编译器/运行时/标准库,自有许可)与「NVIDIA 再分发组件」(仅 Attachment A 白名单最小集——MVP 实际只需 `libdevice.10.bc`,cuBLAS 绑定包按需附带 `cublas64_*.dll` runtime DLL);**完整 Toolkit/驱动/Nsight 永不捆绑**(许可红线 r6);白名单审计延续 M5.4 `check_redistribution`(`ci/check_redistribution.py`)。「装了 Toolkit ≠ 有驱动」(13.1+)进安装诊断。
- **签名清单约定**:全部 `.exe` / `.dll` / `.msi` 产物经 **Authenticode + 时间戳**签名;**签名后端 of-record = Azure Artifact Signing**(OV 证书或 Azure Artifact Signing,EV 不再豁免 SmartScreen,r6;生产签名经 CI secret + 人工门控)。发布产物携**签名清单**(每产物:干名 → content digest → 签名/验签状态 `Valid|Unsigned|Invalid`);**验签通过(`Valid`)为上传前置**——未签名 / 验签失败产物不得进入发布 artifact(发布阻断)。
- **SBOM 约定**:构建期生成 **SPDX**(构建视图);发布附 **CycloneDX**(发布视图);两视图组件清单覆盖 bundle 全部分发组件(语言本体 + NVIDIA 再分发组件,含版本与许可标识);**SBOM 齐备为发布前置**(缺 SBOM 即阻断,10 §6 / 14 §8)。
- **Release 层 hard-block 发布门**:CI 第三层 **Release**(14 §8;PR Smoke / Nightly 之外)在打 tag / 发布工作流触发(非每 PR);门集 = 签名 + 验签 + SBOM 齐备 + 许可白名单审计 + `bench --strict`(无容错跳过,零 estimated 残留)+ conformance/UI golden 全绿 + L1 基准无 Critical 回归;**任一门失败 → 不上传 artifact**(发布阻断,10 §6 工具链发布门)。PR Smoke 步骤 38 为本层签名/SBOM 子集的冒烟前哨。

全部发布产物完整性 / 签名状态 / 再分发面以 **content-tree SHA-256 + 验签判定 + Attachment A 白名单审计的确定性机器事实** 定义;device 分发维持 **PTX-only**(07 §7,cubin/fatbin 真分发 → G1,M8 out_of_scope);**不以 UB 表述**(§4)。**发布门判定以机器可复核事实表达,EULA 法律白名单逐项核对维持 `pending-human-review`**(AI 不代签,§4)。

**编号区间**:本文件条款自 **RXS-0135** 起续号(全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1;最高现存 RXS-0134 @ [pipeline.md](pipeline.md))。本轮计划落地 **RXS-0135 ~ RXS-0139**(见 §2),每条 ≥1 测试锚定(`//@ spec: RXS-####`,`src/rurixup` crate 单测)。区间登记于 [README.md](README.md) §4 文件清单。

## 2. 条款

> 每条按需分 Syntax / Legality / Dynamic Semantics / Implementation Requirements 节,**严禁 UB 节**(10 §7.5)。发布产物完整性 / 签名状态 / 再分发面以 **content-tree SHA-256 + 验签判定 + Attachment A 白名单审计的确定性机器事实** 定义,违例由 `rurixup` **工具层 Result / 退出码 / 失败子门枚举**表达,**不引用新 RX 段位码**(§3)。条款体复用 M6 包管理既有内容树 / SHA-256(RXS-0090/0092/0094)与 M5.4 NVIDIA 再分发白名单审计,新增仅补发布产物缺口。

### RXS-0135 原子分发与 content-tree 完整性

**Syntax**(发布产物模型,`src/rurixup`):

```
Component ::= { name, version, license, partition, sha256 }   // 单个分发组件
BundleManifest ::= { rurix_version, components: [Component] }  // 同一版号下的分发单元
InstallTarget::atomic_install(&bundle, staged, expected_digest)
    -> Result<InstallReceipt, InstallError>                   // 全有或全无
```

**Legality**:

- 语言本体组件(`Partition::LanguageCore`,编译器 `rurixc`/`rx` + 运行时 `rurix-rt` + 标准库)**同一版号**作单一原子分发单元:任一语言本体组件版号 ≠ bundle `rurix_version` 即 `InstallError::VersionSkew`(NVIDIA 再分发组件各携上游版号,豁免本判据,RXS-0136)。
- 分发 bundle 完整性锚 = **content-tree 规范化 SHA-256**(复用 `rurix-pkg` RXS-0090 内容树规范化 / RXS-0092 `rurix.lock` 精确解析图 / RXS-0093 SHA-256:相对路径 `/` 归一 + 字典序排序 + 长度前缀消歧,排除时间戳/权限元数据)。

**Dynamic Semantics**:

- **原子安装为全有或全无**:`atomic_install` 仅当 staged 内容树实测摘要 == 已发布(已签名)摘要 `expected_digest` 且语言本体同一版号时,**一次性提交**全部组件并产 `InstallReceipt`;摘要不符(篡改任一字节即变)→ `InstallError::IntegrityMismatch` 且**安装目标保持安装前状态**(不留半装),`rurixup` 引导器据此实现失败回滚与按版本切换。
- content-tree 摘要不依赖 staged 切片顺序(规范化排序),同一内容树在不同机器 / 时刻摘要一致(逐字节复现根,对齐 M6.3)。

**Implementation Requirements**:

- 复用 `rurix-pkg::content_tree::hash_entries` / `rurix-pkg::sha256`(零外部依赖、纯函数确定性);`rurixup` 默认 `unsafe_code=deny`(纯 Rust,无 FFI)。

> 锚定测试:`src/rurixup/src/install.rs`(`atomic_install_verifies_content_tree`:摘要匹配原子提交 / 篡改拒装回滚;`content_digest_is_order_independent`)+ `src/rurixup/src/bundle.rs`(`language_core_version_skew_detected`)。

### RXS-0136 语言本体与 NVIDIA 再分发组件分离打包

**Syntax**(分区与白名单审计,`src/rurixup`):

```
Partition ::= LanguageCore | NvidiaRedist                     // bundle 分区
BundleManifest::partition(p) -> [&Component]                  // 按分区筛选(字典序)
audit_redistribution(&bundle) -> RedistributionAudit { pass, violations }
is_attachment_a_whitelisted(name) -> bool                     // libdevice.<d>.bc | cublas(Lt)?64_<d>.dll
```

**Legality**:

- 发布 bundle **分区**为「语言本体」(`LanguageCore`,Rurix 自研、自有许可)与「NVIDIA 再分发组件」(`NvidiaRedist`)。NVIDIA 分区**仅容 Attachment A 白名单最小集**:`libdevice.<digits>.bc`(MVP 实际只需 `libdevice.10.bc`)与 `cublas64_<digits>.dll` / `cublasLt64_<digits>.dll`(cuBLAS 绑定包按需附带,对齐 M5.4 `check_redistribution` 断言 3c 白名单正则)。
- NVIDIA 分区中任一**非 Attachment A 白名单**组件(完整 Toolkit `nvcc`/`ptxas`、驱动、Nsight 等)即审计违例(`pass=false`,`violations` 枚举)——**完整 Toolkit/驱动/Nsight 永不捆绑**(许可红线 r6)。

**Dynamic Semantics**:

- `audit_redistribution` 仅对 NVIDIA 分区组件逐项核白名单(语言本体分区不参与);违例项按干名字典序确定枚举,供发布门(RXS-0139)与 `ci/check_redistribution.py` 延续审计消费。

**Implementation Requirements**:

- NVIDIA EULA Attachment A 白名单逐项**法律**核对维持 `pending-human-review`(AI 仅起草机器事实,不代签,§4);本条款审计为**机器事实层**(干名白名单 + 分区一致性),不替代法律签署。

> 锚定测试:`src/rurixup/src/bundle.rs`(`bundle_separates_core_from_nvidia_redist`:分区筛选 + 白名单识别 + 全白名单审计通过;`non_whitelisted_nvidia_component_fails_audit`:完整 Toolkit/Nsight 混入即违例枚举)。

### RXS-0137 签名清单约定与验签发布前置

**Syntax**(签名清单,`src/rurixup`):

```
SignStatus ::= Valid | Unsigned | Invalid                     // Get-AuthenticodeSignature 判定
SignBackend ::= AzureArtifactSigning | SelfSignedTest         // of-record vs 本地冒烟
SignedArtifact ::= { name, digest, status, timestamped, backend }
SigningManifest::upload_permitted() -> bool                   // 验签发布前置
SigningManifest::verified_artifacts() -> [name]               // 验签通过去重集
```

**Legality**:

- 全部 `.exe` / `.dll` / `.msi` 产物经 **Authenticode + 时间戳**签名;**签名后端 of-record = Azure Artifact Signing**(生产签名经 CI secret + 人工门控,本机/CI 不自动调用真实证书,§4)。
- 单产物**验签通过**判据 = `status == Valid` **且** `timestamped`(缺 RFC 3161 时间戳不计通过)。
- **验签发布前置**:签名清单非空 **且**全部产物验签通过 → 允许上传(`upload_permitted`);任一**未签名** / **验签失败**(`Invalid`)/ **缺时间戳** → 不得进入发布 artifact(发布阻断,RXS-0139)。

**Dynamic Semantics**:

- `verified_artifacts` = 验签通过产物干名字典序去重集——**机器事实**:验签通过 + 时间戳齐备;= 计入 `m8.counter.release_artifacts_signed` 的 `signed_artifacts`(`ci/budget_eval.py`)。
- 本地/CI 冒烟以 `SelfSignedTest` 临时自签测试证书产**真实 Authenticode 红绿**(`Set-AuthenticodeSignature` 签 → `Get-AuthenticodeSignature` 验);`AzureArtifactSigning` 生产路径以 secret 门控分支占位,不自动调用(§4)。

**Implementation Requirements**:

- 验签状态由外部(`Get-AuthenticodeSignature`)回填(`SignStatus::parse` 映射 `Valid`/`NotSigned`/`HashMismatch` 等);签名后端不擅自切换(带档复议留痕,§4)。

> 锚定测试:`src/rurixup/src/signing.rs`(`signing_manifest_shape_and_verify_gate`:全 Valid+时间戳放行 / 未签名·失败·缺时间戳阻断 + verified 集;`sign_status_parse_roundtrip`)。

### RXS-0138 SBOM 约定(SPDX 构建视图 + CycloneDX 发布视图)

**Syntax**(SBOM 双视图,`src/rurixup`):

```
SbomViews ::= { spdx: String, cyclonedx: String }
generate(&bundle) -> SbomViews                                // SPDX 2.3 + CycloneDX 1.5
components_covered(&bundle, &views) -> bool                   // 组件齐备判据
```

**Legality**:

- 构建期生成 **SPDX**(构建视图,SPDX-2.3 JSON:`packages[]` 含 `name`/`versionInfo`/`licenseConcluded`/SHA256 checksum + `partition` 注记);发布附 **CycloneDX**(发布视图,CycloneDX-1.5 JSON:`components[]` 含 `name`/`version`/`licenses`/SHA-256 hash + `rurix:partition` property)。
- **组件齐备判据**:bundle 每个组件的干名与版本均出现于**两**视图——任一视图缺任一组件即不齐备(`components_covered=false`);空 bundle 视为不齐备。**SBOM 齐备为发布前置**(缺 SBOM / 不齐备即阻断,10 §6 / 14 §8,RXS-0139)。

**Dynamic Semantics**:

- 两视图组件按干名字典序确定排序,生成**逐字节确定**(同一 bundle 两次产逐字节一致字节流);覆盖语言本体 + NVIDIA 再分发组件全集(含版本与许可标识)。

**Implementation Requirements**:

- 零外部依赖:手写确定性 JSON 序列化(`crate::json_escape` + 字典序);不引入 SBOM 第三方生成器(供应链可信根,全仓零依赖纪律)。

> 锚定测试:`src/rurixup/src/sbom.rs`(`sbom_spdx_cyclonedx_generation`:双视图格式标识 + 组件齐备 + 确定性重生一致;`sbom_coverage_detects_missing_component`:漏组件 / 空 bundle 不齐备)。

### RXS-0139 Release 层 hard-block 发布门

**Syntax**(发布门决策,`src/rurixup`):

```
GateInputs ::= { signing_all_valid, sbom_present, redistribution_audit_pass,
                 bench_strict_pass, conformance_green, ui_golden_green,
                 l1_no_critical_regression }                  // 各子门机器事实
release_decision(&inputs) -> ReleaseDecision { allow_upload, failed_gates }
```

**Legality**(14 §8 第三层 Release;打 tag / 发布工作流触发,非每 PR):

- 门集 = 签名验签(RXS-0137 `upload_permitted`)+ SBOM 齐备(RXS-0138 `components_covered`)+ NVIDIA 再分发白名单审计(RXS-0136 `audit.pass`)+ `bench --strict`(无容错跳过,零 estimated 残留)+ conformance 全绿 + UI golden 全绿 + L1 基准无 Critical 回归。
- **hard block**:**任一子门失败 → `allow_upload=false`(不上传 artifact)**;`failed_gates` 按固定顺序(签名 / SBOM / 许可审计 / bench-strict / conformance / UI golden / L1 回归)确定枚举失败门。

**Dynamic Semantics**:

- `release_decision` 为纯函数:全门绿 → 放行上传发布产物 + SBOM + 签名清单;任一门红 → 发布阻断(10 §6 工具链发布门)。PR Smoke 步骤 38 为本层签名/SBOM 子集的冒烟前哨(`ci/release_pipeline_smoke.py`);Release workflow 在 tag 触发跑全门。
- **真实红绿**(反 YAML-only):构造未签名产物 / 缺 SBOM / 白名单外组件 → 对应子门红 → 发布门阻断;修复转绿,run URL 归档(§4 / CI_GATES §6 第 5 项)。

**Implementation Requirements**:

- 发布门失败以**工具层退出码 + `failed_gates` 枚举**表达(`rurixup` 退出码 2 = 发布阻断),**不引用 RX 段位码**(§3);四项 CI 子门事实(bench-strict / conformance / UI golden / L1 回归)由 Release workflow 实测回填 `CiFacts`。

> 锚定测试:`src/rurixup/src/gate.rs`(`release_gate_hard_blocks_on_any_failure`:全绿放行 / 未签名·缺 SBOM·白名单外各阻断 + 多门枚举)+ `src/rurixup/src/lib.rs`(`run_release_end_to_end_green_then_blocked`:端到端编排)。

## 3. 错误码引用汇总

> **本里程碑不新增 RX 错误码**(零追加)。`rurixup` 为独立发布工具(非编译器前端),其发布门失败诊断(未签名/验签失败 / SBOM 缺失或不全 / 再分发白名单违例 / bundle content-tree 完整性不符)以**工具层错误值 + 退出码 + 失败子门枚举**表达——`InstallError`(`IntegrityMismatch` / `VersionSkew`,RXS-0135)、`SigningManifest::upload_permitted=false`(RXS-0137)、`RedistributionAudit::violations`(RXS-0136)、`ReleaseDecision::failed_gates` + 退出码 2(RXS-0139),**而非编译器侧 `RX####` 段位码**;`registry/error_codes.json` 与 `src/rurixc/src/messages/{en,zh}.messages` **本里程碑不动**(对齐 M8.3 pipeline.md §3「rustc 原生诊断而非 RX 段位码」零追加先例)。
>
> 若实现期发现某发布门失败**确需编译器侧 RX 诊断 / 运行期段位码**(如发布产物经 `rurixc` 工具链链接阶段诊断),则**停手标注「需人工升档」**(§4),按段位 7(链接/工具链,07 §5)`RX70xx` 续接分配(分配制递增、含义冻结、只追加,10 §6),不在本文件自行预造。
>
> NVIDIA EULA Attachment A 白名单逐项法律核对的人工签署状态以证据字段 `eula_whitelist_verdict`(`pending-human-review` / `signed-compliant` / `signed-noncompliant`,沿 M5.4 `redistribution_audit_evidence_schema.json` 先例)表达,**非 RX 段位码**;AI 不代签(§4)。

## 4. 升档 / 禁区留痕

- **Azure Artifact Signing 为 of-record 签名后端(owner 裁定)**:全部 EXE/DLL/MSI 经 Authenticode + 时间戳,签名后端 of-record = Azure Artifact Signing(OV 证书或 Azure Artifact Signing,EV 不再豁免 SmartScreen,r6)。**生产签名经 CI secret + 人工门控,本机/CI 不自动调用真实证书**;本地/CI 冒烟以临时自签测试证书产真实 Authenticode 红绿(机器事实层验签判定),Azure 生产路径以 secret 门控分支占位不调用。带档复议按裁决留痕,不擅自切换签名后端。
- **NVIDIA EULA Attachment A 白名单法律签署维持 `pending-human-review`(AI 不代签)**:NVIDIA 再分发组件仅 Attachment A 白名单最小集(MVP 实际只需 `libdevice.10.bc`,cuBLAS 绑定包按需附带 `cublas64_*.dll`),完整 Toolkit/驱动/Nsight **永不捆绑**(许可红线 r6);白名单逐项法律核对/再分发判定由所有者/法务人工签署,**AI 仅起草机器事实**(`check_redistribution` 白名单审计 + content-tree 清单),不代签(对齐 M5.4 `redistribution_audit` 先例)。
- **cubin/fatbin 真分发(G1,PTX-only)**:M8 维持 **PTX-only** 开发期产物(07 §7);发布 bundle 复用 `rurix-rt` PTX 装载路径,不改 device codegen 分发形态;cubin/fatbin 真分发 → G1(M8 out_of_scope)。触及即停下标注「需人工升档」。
- **完整 Toolkit·驱动·Nsight 捆绑(许可红线 r6)**:发布 bundle 永不捆绑完整 CUDA Toolkit / 驱动 / Nsight;违例为许可红线。触及即停下标注「需人工升档」。
- **Python 原生嵌入(永久红线 1,SG-008)**:发布链路仅分发 C ABI / PYD 产物(对接 interop.md RXS-0122~0125),不分发 Python 解释器宿主 / 原生嵌入(死亡路线,SG-008 维持 not_triggered)。触及即停下标注「需人工升档」。
- **包 registry sumdb(D-312/G2)**:MVP 发布 = lockfile + vendor + checksum + content-tree SHA-256;真 registry / sumdb 留 G2(D-312,SG-007),**M8 维持 not_triggered**(owner 裁定);触及即停下标注「需人工升档」。
- **UB 节禁区**:发布产物完整性 / 签名状态 / 再分发面以 **content-tree SHA-256 + 验签判定 + 白名单审计的确定性机器事实** 定义,**严禁 UB 节**(UB 为人类经 Full RFC 落笔的禁区,10 §7.5)。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-17 | 新建 spec/release.md(M8.4 发布产物语义面起始文件):登记编号区间 RXS-0135 起续号预留 + 文件级前言 / 范围(原子分发与 content-tree 完整性 / 语言本体与 NVIDIA 再分发组件分离打包 / 签名清单约定与验签发布前置 / SBOM SPDX+CycloneDX 约定 / Release 层 hard-block 发布门;**复用 M6 包管理 content-tree/lock RXS-0090/0092/0094 与 M5.4 NVIDIA 再分发白名单审计,新增仅补发布产物缺口**;PTX-only、完整 Toolkit/驱动/Nsight 永不捆绑 r6、永不 Python 原生嵌入、registry sumdb 维持 not_triggered、发布门以机器事实定义不设 UB)/ 依据与授权(08 §9 D-241 r6 + 14 §8 Release 层 + 10 §6 工具链发布门 + 09 §6/§7 + 01 §6;M8_CONTRACT D-M8-4 / G-M8-4 / G-M8-7 / RD-001 `rfc_required: none` + M8_PLAN §4 + CI_GATES §3 步骤 38)/ 计划条款骨架(§2 预留,非裸条款头:RXS-0135 原子分发与 content-tree 完整性 / RXS-0136 语言本体与 NVIDIA 再分发组件分离打包 / RXS-0137 签名清单约定与验签发布前置 / RXS-0138 SBOM SPDX+CycloneDX 约定 / RXS-0139 Release 层 hard-block 发布门)/ 错误码说明(§3:rurixup 发布门诊断按需段位 7 RX70xx 续接,**脚手架不预造**,最终随实现 PR 按 07 §5 裁定;EULA 白名单 `eula_whitelist_verdict` 人工签署字段非 RX 码)/ 升档·禁区留痕(§4:Azure Artifact Signing of-record owner 裁定 + 生产签名 secret/人工门控、EULA Attachment A 白名单 pending-human-review AI 不代签、cubin/fatbin G1·PTX-only、完整 Toolkit/驱动/Nsight 捆绑 r6 红线、Python 原生嵌入红线 1/SG-008、registry sumdb D-312/G2 not_triggered、UB 节禁区)。**沿 README v1.15 toolchain.md / v1.20 stdlib.md / v1.25 interop.md / v1.27 cublas.md / v1.29 pipeline.md 先例:本轮不落带编号裸条款头**——条款体与 ≥1 测试锚定随 M8.4 实现 PR 同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定),无体例变更 | Direct |
| v1.1 | 2026-06-17 | 落地带编号条款体 RXS-0135 ~ RXS-0139(M8.4 实现 PR,条款体随实现 + 测试锚定同落,§2 计划骨架升格为条款体):RXS-0135 原子分发与 content-tree 完整性(语言本体同一版号单一原子分发单元 + content-tree 规范化 SHA-256 完整性锚 复用 rurix-pkg RXS-0090/0092/0093;原子安装全有或全无、校验失败回滚不留半装)/ RXS-0136 语言本体与 NVIDIA 再分发组件分离打包(LanguageCore ⟂ NvidiaRedist 分区;NVIDIA 仅 Attachment A 白名单最小集 libdevice.<d>.bc + cublas(Lt)?64_<d>.dll,完整 Toolkit/驱动/Nsight 永不捆绑 r6;延续 M5.4 check_redistribution 口径)/ RXS-0137 签名清单约定与验签发布前置(Authenticode + 时间戳;of-record Azure Artifact Signing 生产 secret/人工门控、本地冒烟 SelfSignedTest 自签真实红绿;验签通过=Valid+时间戳为上传前置,未签名/失败/缺时间戳阻断;verified 去重集计入 m8.counter.release_artifacts_signed)/ RXS-0138 SBOM 约定(SPDX-2.3 构建视图 + CycloneDX-1.5 发布视图,组件齐备判据=干名与版次均落两视图,手写确定性 JSON 零依赖)/ RXS-0139 Release 层 hard-block 发布门(签名/SBOM/许可审计/bench-strict/conformance/UI golden/L1 回归任一红 → allow_upload=false 不上传 artifact + failed_gates 确定枚举,退出码 2)。每条 ≥1 锚定(`src/rurixup` 单测:install / bundle / signing / sbom / gate + lib 端到端;trace_matrix 维持全锚定 134→139)。**本里程碑不新增 RX 码**(§3:rurixup 工具层 Result/退出码/失败子门枚举,registry/error_codes.json 与 en.messages 零追加,对齐 M8.3 pipeline.md 先例)。实现裁决:新 crate `src/rurixup` 默认 `unsafe_code=deny`(纯 Rust 无 FFI),复用 rurix-pkg content_tree/sha256;ci/trace_matrix.py 锚定源加入 src/rurixup;PTX-only、不触 cubin/fatbin G1 / 完整 Toolkit r6 / 红线 1 SG-008 / registry sumdb D-312。Azure 为 of-record 签名后端(owner 裁定)、EULA Attachment A 白名单维持 pending-human-review(AI 不代签),无体例变更 | Direct |
