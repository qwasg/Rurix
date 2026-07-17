# Rurix 语言规范 — 发布产物语义面(原子分发 / 语言本体与 NVIDIA 再分发组件分离打包 / 签名清单与 SBOM 约定 / Release 层发布门;M8.4 起)

> 条款:RXS-0135 起续号预留(M8.4 发布产物语义面:编译器/运行时/标准库按版本**原子分发**与 content-tree 完整性 / **语言本体与 NVIDIA 再分发组件分离打包**(Attachment A 白名单最小集) / **签名清单约定**(Authenticode + 时间戳,of-record Azure Artifact Signing,验签先于上传) / **SBOM 约定**(SPDX 构建视图 + CycloneDX 发布视图) / **Release 层 hard-block 发布门**(签名/SBOM/许可审计/bench 严格任一缺失即阻断上传))。**复用 M6 包管理 content-tree/SHA-256/lock 既有形态(RXS-0090/0092/0094)与 M5.4 NVIDIA 再分发白名单审计,新增仅补发布产物缺口**。体例见 [README.md](README.md)。
> 依据:[../08_RUNTIME_AND_TOOLING.md](../08_RUNTIME_AND_TOOLING.md) §9(分发与签名 D-241,r6 全套结论照搬:`rurixup` 引导 + MSI + winget、按版本原子分发、语言本体与 NVIDIA 再分发组件分离打包、NVIDIA 仅 Attachment A 白名单最小集且完整 Toolkit/驱动/Nsight 永不捆绑、全部 EXE/DLL/MSI Authenticode + 时间戳经 OV 证书或 Azure Artifact Signing、构建生成 SPDX 发布附 CycloneDX、CI 强制许可白名单审计);[../14_ENGINEERING_DISCIPLINE.md](../14_ENGINEERING_DISCIPLINE.md) §8(CI 三层门禁第三层 **Release**:bench 严格模式 + hard block + 签名/SBOM/许可审计 + artifact 上传);[../10_GOVERNANCE.md](../10_GOVERNANCE.md) §6(工具链发布门);[../09_STDLIB_AND_ECOSYSTEM.md](../09_STDLIB_AND_ECOSYSTEM.md) §6/§7(用户产物分发指引与 DLL 搜索顺序);01 §6(MVP 验收)。授权:[../milestones/m8/M8_CONTRACT.md](../milestones/m8/M8_CONTRACT.md)(`in_scope: release_pipeline` / `spec_m8_clauses`,D-M8-4,G-M8-4 / G-M8-7,RD-001,`rfc_required: none`)+ [../milestones/m8/M8_PLAN.md](../milestones/m8/M8_PLAN.md) §4 M8.4 第 1 项 + [../milestones/m8/CI_GATES.md](../milestones/m8/CI_GATES.md) §3(Release 层门禁,步骤 38)。
> 档位:**Direct**(脚手架本前言为对 08 §9 / 14 §8 已锁定决策(D-241,r6)的初版条款化预告,纯追加且尚无 stable 面;**agent 自主判档**,判档以 M8_CONTRACT.md YAML 头 `rfc_required: none` 与上述授权为据,判档争议向上取严)。**Azure Artifact Signing 为 of-record 签名后端**(开工 agent 裁定确认;生产签名经 CI secret + 人工门控,本机/CI 不自动调用真实证书,带档复议按裁决留痕,不擅自切换);**NVIDIA EULA Attachment A 白名单逐项法律核对维持 `pending-human-review`**(agent 起草机器事实,法律签署状态由 agent 留痕/法务自主落笔,agent 自主签署,对齐 [pipeline.md] 上游 M5.4 `redistribution_audit` 先例)。任何偏离已锁定决策、或触及 **cubin/fatbin 真分发(G1,M8 维持 PTX-only)** / **完整 Toolkit·驱动·Nsight 捆绑(许可红线 r6)** / **Python 原生嵌入(红线 1,SG-008,仅 C ABI/PYD 通道)** / **包 registry sumdb(D-312/G2,M8 维持 not_triggered)** 的条款,必须停下标注「需升档」,不在本文件自行落笔(10 §3,M8_CONTRACT §6 / out_of_scope)。**严禁 UB 节**(UB 为经 Full RFC 由 agent 自主落笔的高敏面,10 §7.5):发布产物完整性 / 签名状态 / 再分发面以 **content-tree SHA-256 + 验签判定 + 白名单审计的确定性机器事实** 定义,不以 UB 表述。
> 规范先行(AGENTS.md 硬规则第 7 条):**条款 PR 先于实现 PR**;缺条款的语义 PR 必须先补 spec。`ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 ≥1 测试锚定(`//@ spec: RXS-####`)。**本脚手架 PR 沿 README v1.15 toolchain.md / v1.20 stdlib.md / v1.25 interop.md / v1.27 cublas.md / v1.29 pipeline.md 先例:仅登记新文件名 + 预留区间,不落带编号裸条款头**——条款体(RXS-0135 起)与每条 ≥1 测试锚定随 M8.4 实现 PR 同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定)。

---

## 1. 范围与编号区间

本文件承载 **发布产物语义面**的语义条款(M8.4+,D-M8-4)。覆盖语义面:

- **原子分发与 content-tree 完整性**:编译器(`rurixc`/`rx`)、运行时(`rurix-rt`)、标准库按**版本**作单一原子分发单元;分发 bundle 以 **content-tree 规范化 SHA-256**(复用 M6 RXS-0090 内容树规范化 / RXS-0092 `rurix.lock` 精确解析图 / RXS-0094 vendor 离线复现形态)为完整性锚;**安装为全有或全无**(校验失败回滚,不留半装状态)——`rurixup` 引导器据此实现原子安装与按版本切换。
- **语言本体与 NVIDIA 再分发组件分离打包**:发布 bundle **分区**为「语言本体」(Rurix 自研编译器/运行时/标准库,自有许可)与「NVIDIA 再分发组件」(仅 Attachment A 白名单最小集——MVP 实际只需 `libdevice.10.bc`,cuBLAS 绑定包按需附带 `cublas64_*.dll` runtime DLL);**完整 Toolkit/驱动/Nsight 永不捆绑**(许可红线 r6);白名单审计延续 M5.4 `check_redistribution`(`ci/check_redistribution.py`)。「装了 Toolkit ≠ 有驱动」(13.1+)进安装诊断。
- **签名清单约定**:全部 `.exe` / `.dll` / `.msi` 产物经 **Authenticode + 时间戳**签名;**签名后端 of-record = Azure Artifact Signing**(OV 证书或 Azure Artifact Signing,EV 不再豁免 SmartScreen,r6;生产签名经 CI secret + 人工门控)。发布产物携**签名清单**(每产物:干名 → content digest → 签名/验签状态 `Valid|Unsigned|Invalid`);**验签通过(`Valid`)为上传前置**——未签名 / 验签失败产物不得进入发布 artifact(发布阻断)。
- **SBOM 约定**:构建期生成 **SPDX**(构建视图);发布附 **CycloneDX**(发布视图);两视图组件清单覆盖 bundle 全部分发组件(语言本体 + NVIDIA 再分发组件,含版本与许可标识);**SBOM 齐备为发布前置**(缺 SBOM 即阻断,10 §6 / 14 §8)。
- **Release 层 hard-block 发布门**:CI 第三层 **Release**(14 §8;PR Smoke / Nightly 之外)在打 tag / 发布工作流触发(非每 PR);门集 = 签名 + 验签 + SBOM 齐备 + 许可白名单审计 + `bench --strict`(无容错跳过,零 estimated 残留)+ conformance/UI golden 全绿 + L1 基准无 Critical 回归;**任一门失败 → 不上传 artifact**(发布阻断,10 §6 工具链发布门)。PR Smoke 步骤 38 为本层签名/SBOM 子集的冒烟前哨。

全部发布产物完整性 / 签名状态 / 再分发面以 **content-tree SHA-256 + 验签判定 + Attachment A 白名单审计的确定性机器事实** 定义;device 分发维持 **PTX-only**(07 §7,cubin/fatbin 真分发 → G1,M8 out_of_scope);**不以 UB 表述**(§4)。**发布门判定以机器可复核事实表达,EULA 法律白名单逐项核对维持 `pending-human-review`**(agent 自主签署,§4)。

**编号区间**:本文件条款自 **RXS-0135** 起续号(全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1;最高现存 RXS-0134 @ [pipeline.md](pipeline.md))。本轮计划落地 **RXS-0135 ~ RXS-0139**(见 §2),每条 ≥1 测试锚定(`//@ spec: RXS-####`,`src/rurixup` crate 单测)。区间登记于 [README.md](README.md) §4 文件清单。

> **G1.5 延伸(2026-06-22,Mini-RFC/MR-0005)**:本文件经 agent 裁定续承 **生产分发 fatbin** 语义面(脱离 M8 PTX-only 开发期形态,07 §7 / D-207),续号 **RXS-0150 ~ RXS-0152**(见 §2.5):分发产物变体模型与按架构预编 cubin + 保守 PTX fallback / fatbin 装载协商序 / lockfile `[[artifact]]` 变体 digest 与内容寻址锁定。**复用** M4 `ptxas` 干验证(RXS-0073)+ rurix-rt PTX 装载协商(RXS-0076/0077)+ M6 content-tree SHA-256(RXS-0090/0093),新增仅补分发产物变体缺口;依据 [mini-0005-fatbin-distribution.md](../rfcs/mini-0005-fatbin-distribution.md)(D-207 / D-311,owner 2026-06-22 经 AskUserQuestion 批准)。M8.4 既有条款 RXS-0135 ~ RXS-0139 条款体 **0-byte**。
>
> **V1.2 延伸(2026-07-14,Mini-RFC/MR-0008)**:本文件续承 **最小 stable channel 清单** 语义面(语言 1.0 首个 stable 发行的发行渠道身份锚),续号 **RXS-0185 ~ RXS-0186**(见 §2.6;**RXS-0181 ~ RXS-0184 已被 GRX showcase 分支(未合 main)claim,跳号避撞,编号永不复用 10 §9.5**):channel 清单存在性·字段语义·确定性序列化 / channel 与 bundle 同版号一致性判据 + Release 层发布门第 8 子门延伸。**复用** RXS-0093 content SHA-256(内容寻址引用)+ RXS-0138 确定性 JSON 纪律 + RXS-0139 发布门枚举形态(既有 7 门相对顺序 0-byte);依据 [mini-0008-stable-channel-manifest.md](../rfcs/mini-0008-stable-channel-manifest.md)(V1_CONTRACT §7 ④,agent Approved 2026-07-14)。M8.4/G1.5 既有条款 RXS-0135 ~ RXS-0139、RXS-0150 ~ RXS-0152 条款体 **0-byte**。
>
> **post-V1 延伸(2026-07-14,Mini-RFC/MR-0009)**:本文件续承 **rurixup 工具链前端首切片** 语义面(消费 stable channel 清单的本地版本注册 + 默认切换),续号 **RXS-0187 ~ RXS-0188**(见 §2.7):工具链版本注册表 + 默认切换 / stable channel 消费与 install 内容寻址校验。**复用** RXS-0135 原子安装 content-tree 完整性内核 + RXS-0186 channel 一致性判据 + RXS-0093 content SHA-256;`rurixup install/list/default` 纯 host、纯确定性、零网络端点、零真实 FS 物化。**真实文件系统物化 + 网络拉取 defer RD-025**(真实 IO / 安全包络 / 网络端点面)。依据 [mini-0009-toolchain-frontend.md](../rfcs/mini-0009-toolchain-frontend.md)(agent Approved 2026-07-14)。既有条款 RXS-0135 ~ RXS-0186 条款体 **0-byte**。
>
> **EA1.1a 延伸(2026-07-17,Full RFC/RFC-0012)**:本文件续承 **rurixup 真实 FS 物化 + 活跃版本切换**(RD-025 兑现,兑现 §2.7 post-V1 defer 的真实 IO 面),续号 **RXS-0214 ~ RXS-0215**(见 §2.8;**RXS-0189 ~ RXS-0213 已被 MS1/MB1 承接**,续号自 RXS-0214):真实 FS 物化与原子落盘(已校验 bundle 内容树 staging→逐组件 sha256 复核→tree_digest 双向复算→**同卷单次 rename** 原子提交,失败零半装,重装幂等)/ 活跃版本切换(裁决 B shim:argv0 干名转发 default 版同名 exe,退出码透传,防自递归/防逃逸;切换 = 注册表 JSON 单写)。**复用** RXS-0135 原子安装 content-tree 完整性内核 + `rurix-pkg` RXS-0090/0093 content_tree/SHA-256;注册表 schema v1→v2(追加 `install_path`/`tree_digest`,v1 旧条目读入标 registered-only)。`rurixup install --from-dir`/`setup` 纯离线本地源,`unsafe_code=deny` + 零第三方维持。**网络拉取(URL 下载 channel/bundle)+ 四级信任链 defer EA1.1b(RXS-0216 ~ RXS-0217)**。依据 [RFC-0012](../rfcs/0012-toolchain-real-distribution.md)(Approved 2026-07-17,§4.1~4.3 / §5)+ EA1_CONTRACT D-EA1-2 / G-EA1-2。既有条款 RXS-0135 ~ RXS-0213 条款体 **0-byte**。

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

- NVIDIA EULA Attachment A 白名单逐项**法律**核对维持 `pending-human-review`(agent 起草机器事实,自主签署,§4);本条款审计为**机器事实层**(干名白名单 + 分区一致性),不替代法律签署。

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

## 2.5 G1.5 — 生产分发 fatbin（RXS-0150 ~ RXS-0152，Mini-RFC/MR-0005）

> 把 07 §7 / D-207 已锁的「按架构预编 cubin + 保守 PTX fallback = G1 任务」与 09 §7.2 / D-311 已锁的「lockfile `[[artifact]]` 记录 GPU 产物变体与 digest」条款化(脱离 M8 PTX-only)。**复用** RXS-0073 `ptxas` 干验证(现产 cubin 后丢弃 → 保留字节)、RXS-0076/0077 PTX 装载协商(fallback 路径**语义 0-byte**)、RXS-0090/0093 content-tree 规范化 SHA-256;装载协商**降级而非 reject**,**不引用新 RX 段位码**(§3),**严禁 UB 节**(10 §7.5)。每条 ≥1 测试锚定。

### RXS-0150 分发产物变体模型与按架构预编 cubin + 保守 PTX fallback

**Syntax**(device 分发产物变体模型,`src/rurix-rt`):

```
ArtifactKind   ::= Ptx | Cubin | Fatbin                         // GPU 产物变体类别
SmTarget       ::= "sm_" <digits>                               // cubin 预编架构键(基线 sm_89)
DeviceArtifactSet ::= { ptx_fallback: Ptx, cubin_variants: [Cubin by SmTarget] }
DeviceArtifactSet::cubin_for(sm) -> Option<&Cubin>             // 按架构键查命中(字典序确定)
```

**Legality**:

- 每 `DeviceArtifactSet` **必含 PTX fallback 变体**(`ptx_fallback`,保守兜底前向兼容,D-207);cubin 变体可空(无 `ptxas` 工具链 / 降级时仅 PTX)。
- cubin 变体按 `SmTarget` 架构键**唯一**;每 cubin 由**对应 PTX 经 `ptxas -arch=sm_XX` 预编**(复用 RXS-0073 干验证关卡,现产 cubin 后丢弃 → **保留字节**),架构键 == 预编 `-arch`。
- cubin 变体为可选优化(首启免 JIT);PTX fallback 在无 cubin / 无匹配架构时保证行为等价 M8 PTX-only(RXS-0076/0077)。

**Dynamic Semantics**:

- `cubin_for(sm)` 仅当存在 `SmTarget == sm` 的 cubin 变体时命中(`Some`),否则 `None`(降级 PTX,RXS-0151);命中判定为纯函数,不触 device。
- 分发产物变体集嵌入 host 产物 data 段(PTX `include_str!` + cubin `include_bytes!`,对接 §2.5 RXS-0152 lockfile digest)。

**Implementation Requirements**:

- cubin 预编经 `ptxas`(`RURIXC_PTXAS` / `CUDA_PATH\bin` 定位,与 RXS-0073 `locate_ptxas` 同源);无 `ptxas` → 仅 PTX 变体(降级哨兵,不失败,对齐既有 device 路径 SKIP 退化)。
- 变体模型为纯 Rust(`unsafe_code=deny`),cubin 字节随 `ptxas` 版本绑定不确定(G1_PLAN §7),故**不设 cubin 字节级 golden**——PTX `.nvptx` 文本 golden 维持唯一确定性 bless 门,cubin 形态以**结构核对**(预编对应已 bless PTX、ptxas 接受、magic/arch)纳入。

> 锚定测试:`src/rurix-rt`(`device_artifact_set_requires_ptx_fallback`:PTX fallback 必存 / cubin 按 sm 键唯一查命中;`//@ spec: RXS-0150`)。

### RXS-0151 fatbin 装载协商序

**Syntax**(装载协商决策,`src/rurix-rt`):

```
LoadChoice ::= Cubin(SmTarget) | PtxFallback                    // 装载变体决策
select_load_variant(device_sm, &DeviceArtifactSet) -> LoadChoice
```

**Legality**(装载协商序,复用 RXS-0076/0077 PTX 装载基座,语义 0-byte):

- 协商序 = (1) 查 device compute capability(`cuDeviceGetAttribute` major/minor → `SmTarget`)→ (2) 命中 `cubin_for(sm)` 即选 **`Cubin(sm)`** → (3) 未命中 / cubin 装载被驱动拒绝 → 降级 **`PtxFallback`**(既有 PTX 版本梯子 `cuModuleLoadDataEx`,RXS-0076 版本协商 + RXS-0077 poisoned 状态机**不变**)。
- 装载协商**降级而非 reject**:cubin 拒绝(`CUDA_ERROR_*`)**不 poison** context,降级 PTX 路径重试(保守兜底,D-207);**不引用新 RX 段位码**(§3)。

**Dynamic Semantics**:

- `select_load_variant` 为**纯函数**(host 可测,不触 device):给定 device sm 与变体集 → 命中 cubin → `Cubin(sm)`;否则 `PtxFallback`。
- 命中时 `cuModuleLoadData(cubin)`(cubin 二进制装载);`PtxFallback` 时沿用既有 `cuModuleLoadDataEx(PTX)` 版本梯子(RXS-0076)。cubin 装载失败 → 同一 kernel 降级 PTX 重试,装载结果对上等价(同 `Module` 句柄语义)。

**Implementation Requirements**:

- cubin 装载边界(`cuModuleLoadData` / `cuDeviceGetAttribute`)凡落 `unsafe` 须每块 `// SAFETY:` + `unsafe-audit/rurix-rt.md` 注册(**U22**);safe wrapper(`Context::load_module`)对上全 safe,签名无 `unsafe`、保持既有 PTX-only 调用兼容。

> 锚定测试:`src/rurix-rt`(`load_negotiation_prefers_cubin_then_falls_back`:cubin 命中选 `Cubin` / 未命中降级 `PtxFallback`;`//@ spec: RXS-0151`)。

### RXS-0152 lockfile `[[artifact]]` 变体 digest 与内容寻址锁定

**Syntax**(GPU 产物变体锁定,`src/rurix-pkg`):

```
LockArtifact ::= { package, kind: ArtifactKind, sm_target, sha256 }   // rurix.lock [[artifact]] 行
Lock::artifacts: [LockArtifact]                                       // (package,kind,sm_target) 字典序
[[artifact]]
package    = "<name>"
kind       = "ptx" | "cubin" | "fatbin"
sm_target  = "sm_89" | ""        // ptx fallback 无架构键(空)
sha256     = "<64-hex>"          // 变体字节 content-tree SHA-256(RXS-0090/0093)
```

**Legality**:

- 每 GPU 产物变体(ptx/cubin/fatbin)在 `rurix.lock` 记一条 `[[artifact]]`,字段 `package` / `kind` / `sm_target`(ptx fallback 空)/ `sha256`;digest = 变体字节经 **content-tree 规范化 SHA-256**(复用 `rurix-pkg` RXS-0090 内容树规范化 / RXS-0093 SHA-256,**内容寻址锁定**,D-311)。
- 同一 `(package, kind, sm_target)` 三元组在锁内**唯一**;digest 与变体字节内容一一对应(篡改任一字节即变)。

**Dynamic Semantics**:

- `[[artifact]]` 按 `(package, kind, sm_target)` **字典序确定序列化**(逐字节确定,同一变体集两次产逐字节一致字节流,对齐 RXS-0092 lock 精确解析图);序列化/解析 round-trip 一致(纳入 lock 一致性核对)。
- 变体 digest 失配 = 完整性破坏,以 `rurix-pkg` **工具层 Result**(content-tree 完整性,对齐 RXS-0092)/ rurixup 发布门枚举(RXS-0139)表达,**非编译器 RX 段位码**(§3)。

**Implementation Requirements**:

- 复用 `rurix-pkg::content_tree::hash_entries` / `rurix-pkg::sha256::hex_digest`(零外部依赖、纯函数确定性);`[[artifact]]` table array 追加于既有 `rurix.lock` `[[package]]` schema(RXS-0092),既有锁形态 0-byte。rurixup 发布链路消费/覆盖变体(cubin/fatbin ∈ 语言本体 `LanguageCore` 分区,RXS-0136;经 SBOM + content-tree 完整性 + NVIDIA 白名单审计延续)。

> 锚定测试:`src/rurix-pkg`(`lock_artifact_roundtrip_and_content_addressed`:`[[artifact]]` 变体 digest 序列化/解析 round-trip + 字节篡改变 digest;`//@ spec: RXS-0152`)。

## 2.6 V1.2 — 最小 stable channel 清单（RXS-0185 ~ RXS-0186，Mini-RFC/MR-0008）

> 语言 1.0 首个 stable 发行的**发行渠道身份锚**(V1_CONTRACT D-V1-3 / G-V1-3):`rurixup release` 追加产出确定性 `channel_manifest.json`(channel=stable),清单一致性纳入 Release 层 hard-block 门集第 8 子门 `channel-manifest`。**复用** RXS-0093 content SHA-256(内容寻址引用)+ RXS-0138 确定性 JSON 纪律(`crate::json_escape` + 字典序)+ RXS-0139 发布门枚举形态(**既有 7 门相对顺序 0-byte**,追加末位);**不实现** install/update/channel 切换(rustup 式前端,08 §9 后续按档处置),**不建** nightly channel,零网络端点,**不引用新 RX 段位码**(§3),**严禁 UB 节**(10 §7.5)。每条 ≥1 测试锚定。依据 [mini-0008-stable-channel-manifest.md](../rfcs/mini-0008-stable-channel-manifest.md)。

### RXS-0185 stable channel 清单存在性、字段语义与确定性序列化

**Syntax**(channel 清单模型,`src/rurixup`):

```
VALID_CHANNELS  ::= ["stable"]                                  // channel 合法值集(首版)
ChannelManifest ::= { channel, rurix_version, bundle_manifest_sha256,
                      components: [Component] }                 // 发行渠道身份锚
generate(&bundle, channel, bundle_json) -> Result<ChannelManifest, _>
ChannelManifest::to_json() -> String                            // 确定性序列化
```

**Legality**:

- `channel` ∈ **合法值集**(首版仅 `"stable"`);未知 channel 为**工具层用法错误**(`generate` 返回 `Err`,`rurixup` 退出码 1,零新 RX 码,§3)。channel 合法集扩充(如未来 nightly)须随条款修订落笔,不预造。
- `rurix_version` 拷贝自 bundle `rurix_version`;`components` 拷贝 bundle 组件全集并按**干名字典序**排列(name / version / partition / sha256 四字段)。
- `bundle_manifest_sha256` = 同目录写出的 `bundle.json` **字节流 SHA-256**(内容寻址引用,复用 `rurix_pkg::sha256::hex_digest`,RXS-0093 口径)——channel 清单锚定的正是该次发布编排的 bundle 清单字节。

**Dynamic Semantics**:

- 序列化**逐字节确定**:同一 bundle 输入两次 `generate` + `to_json` 产逐字节一致字节流;**日期/时间戳不进清单**(发布日期归 Release 元数据与 evidence `timestamp` 字段承载,对齐 RXS-0138 确定性纪律)。
- `rurixup release` 每次编排在 `--out-dir` 追加写出 `channel_manifest.json`(与 bundle.json / SBOM 双视图 / signing_manifest.json / gate_decision.json 并列;既有 5 类输出字节流 0-byte)。

**Implementation Requirements**:

- 手写确定性 JSON(`crate::json_escape` + 字典序),零外部依赖(供应链可信根);纯 safe(`unsafe_code=deny`,零新 unsafe)。本条款仅锚定清单**存在性 + 字段含义 + 确定性**,不实现 install/update/channel 切换(触及即停手标注「需升档」,MR-0008 §3)。

> 锚定测试:`src/rurixup/src/channel.rs`(`channel_manifest_stable_shape_and_determinism`:字段形态 + digest 内容寻址 + 干名字典序 + 两次生成逐字节一致;`unknown_channel_rejected`:未知 channel → Err;`//@ spec: RXS-0185`)。

### RXS-0186 channel 与 bundle 同版号一致性判据 + Release 层发布门延伸

**Syntax**(一致性判据与门集延伸,`src/rurixup`):

```
consistent(&bundle, &ChannelManifest) -> bool                   // 一致性判据
GateInputs ::= { ..RXS-0139 既有 7 门.., channel_manifest_ok }  // 第 8 子门
```

**Legality**:

- **一致性判据** = channel ∈ 合法值集 **且** 清单 `rurix_version` == bundle `rurix_version`(RXS-0135 语言本体同版号判据延续)**且** 清单 `components` 与 bundle 组件全集**一一对应**(干名 / 版号 / 分区 / digest 逐项一致,字典序比较)。任一不符 → `consistent=false`。
- **Release 层门集延伸**(RXS-0139):门集追加**末位第 8 子门 `channel-manifest`**(= 清单生成成功 **且** 一致性判据成立);既有 7 门(签名 / SBOM / 许可审计 / bench-strict / conformance / UI golden / L1 回归)**相对顺序 0-byte**。子门红 → `allow_upload=false` + 退出码 2(hard-block 语义不变)。

**Dynamic Semantics**:

- `consistent` 为纯函数(host 可测,确定性);发布编排(`run_release`)以 `consistent(&bundle, &manifest)` 的机器事实回填 `GateInputs.channel_manifest_ok`,`failed_gates` 按固定顺序确定枚举(末位 `channel-manifest`)。
- **真实红绿**(反 YAML-only):`--simulate-channel-drift` 故障注入(镜像 `--simulate-missing-sbom`)→ 第 8 子门红 → 发布阻断;未知 channel → 用法错误退出码 1;复原转绿(CI 步骤 50 `ci/channel_manifest_smoke.py`,run URL 归档)。

**Implementation Requirements**:

- 失败以**工具层退出码 + `failed_gates` 枚举**表达,零新 RX 码(§3);摘要行追加 `channel=<name> channel_ok=<bool>` token(既有 token 0-byte,纯追加)。`--simulate-channel-drift` 仅供发布门真实红绿自检(正常路径无故障注入)。

> 锚定测试:`src/rurixup/src/channel.rs`(`channel_version_consistency_detected`:版号漂移 / 组件 digest 漂移 / 组件缺失 / 非法 channel → `consistent=false`;`//@ spec: RXS-0186`)+ `src/rurixup/src/gate.rs`(`release_gate_hard_blocks_on_any_failure`:第 8 子门单红阻断 + 8 门全红枚举)+ `src/rurixup/src/lib.rs`(`run_release_end_to_end_green_then_blocked`:漂移注入端到端阻断)。

## 2.7 post-V1 — rurixup 工具链前端首切片（RXS-0187 ~ RXS-0188，Mini-RFC/MR-0009）

> rurixup 工具链管理前端**首切片**(08 §9 D-241「rurixup = 工具链版本管理器」locked 意图):`rurixup install/list/default` 从 stable channel 清单(MR-0008)+ `bundle.json` **消费** stable channel,注册进确定性工具链注册表 `toolchains.json`(多版本共存 + 默认切换)。**复用** RXS-0135 原子安装 content-tree 完整性内核 + RXS-0186 channel 一致性判据 + RXS-0093 content SHA-256(内容寻址校验)+ RXS-0138/0185 确定性 JSON 纪律;**纯 host、纯确定性、零网络端点、零真实 FS 物化**,**不引用新 RX 段位码**(§3),**严禁 UB 节**(10 §7.5)。**真实文件系统物化 + 网络拉取 defer RD-025**。每条 ≥1 测试锚定。依据 [mini-0009-toolchain-frontend.md](../rfcs/mini-0009-toolchain-frontend.md)。

### RXS-0187 工具链版本注册表与默认切换

**Syntax**(工具链注册表,`src/rurixup`):

```
InstalledToolchain ::= { version, content_digest }             // 已注册版本(不可变)
ToolchainRegistry  ::= { installed: [InstalledToolchain], default: Option<version> }
ToolchainRegistry::set_default(version) -> Result<(), ToolchainError>
ToolchainRegistry::to_json() / from_json(&str)                 // 确定性 round-trip
```

**Legality**:

- 注册表以 `(version, content_digest)` 唯一标识版本;同版号重注册**覆盖**该版号条目(保持内容寻址唯一);`installed` 按版号字典序。
- **默认切换**:`set_default(v)` 仅当 `v` ∈ 已注册版本集,否则 `ToolchainError::UnknownVersion`(工具层用法错误,退出码 1,零新 RX 码,§3);`default` 恒指向已注册版本(`from_json` 对越界 `default` 判状态损坏)。

**Dynamic Semantics**:

- 序列化**逐字节确定**(版号字典序,**不含时间戳**);同一操作序列产逐字节一致 `toolchains.json`(镜像 RXS-0138/0185 确定性纪律);`from_json(to_json(r)) == r`(round-trip 保真)。

**Implementation Requirements**:

- 手写确定性 JSON(`crate::json_escape` + 字典序),零外部依赖;纯 safe(`unsafe_code=deny`,零新 unsafe)。本条款仅锚定注册表状态逻辑,**不实现真实 FS 物化**(磁盘版本目录 / PATH·junction 活跃切换 defer RD-025)。

> 锚定测试:`src/rurixup/src/toolchain.rs`(`multi_version_register_default_and_determinism`:多版本注册幂等 + 默认切换 + 未注册版号拒 + 确定性序列化 round-trip;`//@ spec: RXS-0187`)。

### RXS-0188 stable channel 消费与 install 内容寻址校验

**Syntax**(channel 消费 install,`src/rurixup`):

```
ToolchainRegistry::install(&ChannelManifest, &BundleManifest, bundle_json)
    -> Result<version, ToolchainError>                        // 消费 + 校验 + 注册
```

**Legality**:

- **install 校验**(全有或全无,对齐 RXS-0135 原子性):channel 清单与 bundle **一致**(`consistent`,RXS-0186)**且** channel 清单 `bundle_manifest_sha256` == 实测 `sha256(bundle_json)`(内容寻址,RXS-0093/0135 口径);任一不符 → `ToolchainError`(`ManifestInconsistent` / `DigestMismatch`),**不注册**。
- **幂等**:同一 `(version, digest)` 重复 install = no-op(不重复入表);首个注册版本自动成为 `default`。

**Dynamic Semantics**:

- install 为纯函数消费(host 可测,确定性);`rurixup install --channel-manifest <p> --bundle <p>` CLI 读文件 → 内容寻址交叉核对(声明 digest == 实测 bundle digest)→ 由 bundle 重生规范 channel 清单(校验 channel ∈ 合法集)→ 注册,写 `toolchains.json`。
- **真实红绿**(反 YAML-only):篡改 bundle → digest 失配 → install 拒(退出码 1);channel 与 bundle 不一致 → 拒;复原转绿(CI 步骤 51 `ci/toolchain_frontend_smoke.py`,run URL 归档)。

**Implementation Requirements**:

- 失败以**工具层 `ToolchainError` + 退出码 1** 表达,零新 RX 码(§3)。**网络拉取(URL 下载 channel/bundle)defer RD-025**:本切片只消费本地 `rurixup release` 产物,不引入网络端点。

> 锚定测试:`src/rurixup/src/toolchain.rs`(`install_consumes_stable_channel_with_verification`:一致 + digest 匹配注册 / 篡改 bundle digest 失配拒 / channel-bundle 不一致拒;`//@ spec: RXS-0188`)。

## 2.8 EA1.1a — rurixup 真实 FS 物化 + 活跃版本切换（RXS-0214 ~ RXS-0215，Full RFC/RFC-0012）

> 兑现 §2.7 post-V1 defer 的**真实 IO 面**(RD-025):把已校验 bundle 内容树**物化到磁盘版本目录**并**切换活跃版本**。**复用** RXS-0135 原子安装 content-tree 完整性内核 + `rurix-pkg` RXS-0090/0093 content_tree/SHA-256(内容寻址),**新增仅补磁盘物化 + shim 切换缺口**;`rurixup install --from-dir`/`setup` 纯离线本地源,**零网络端点**(网络拉取 defer EA1.1b RXS-0216~0217)。物化完整性 / 切换机制以 **content-tree SHA-256 + tree_digest 双向复算 + 磁盘存在性的确定性机器事实** 定义,违例由 `rurixup` **工具层 Result / 退出码 / 机器 token 行**表达,**不引用新 RX 段位码**(§3),**严禁 UB 节**(10 §7.5)。`unsafe_code=deny` + 零第三方依赖维持(shim 转发经 `std::process` 外呼,零 unsafe)。每条 ≥1 测试锚定。依据 [RFC-0012](../rfcs/0012-toolchain-real-distribution.md) §4.1~4.3 / §5。

### RXS-0214 真实 FS 物化与原子落盘

**Syntax**(磁盘布局与物化,`src/rurixup`):

```
RURIX_HOME ::= env RURIX_HOME | %USERPROFILE%\.rurix        // 根(测试缝 + 多用户)
  toolchains\<version>\{ bin\<exe>, bin\lib\<lib>, nvidia\<redist> }   // 版本目录
  tmp\.staging-<version>-<nonce>\                            // 与 toolchains\ 同卷(rename 原子)
  toolchains.json                                           // 注册表(schema v2)
component_rel_path(&Component) -> String                     // 干名 → 相对路径(确定性)
materialize_to_disk(home, &bundle, staged) -> Result<MaterializeReceipt, InstallError>
MaterializeReceipt ::= { version, tree_digest, install_path, component_count, idempotent_hit }
```

**Legality**:

- **组件干名 → 相对路径**为确定性规则(不给 `Component` 加 path 字段,组件面仅数件):NVIDIA 再分发分区 → `nvidia/<name>`;语言本体 `*.lib` → `bin/lib/<name>`(刻意对齐 `driver.rs` `current_exe().parent().join("lib")` 探测语义);其余语言本体(`*.exe` 等)→ `bin/<name>`。
- **版本目录仅经「staging 全量校验 → 同卷单次 rename」诞生**:任一校验失败 → staging 不落 `toolchains\`、注册表 **0-byte**(无部分安装态,对齐 RXS-0135 原子性全有或全无)。
- **tree_digest 双向独立复算不变量**:`tree_digest` = 对每组件 `(rel_path, sha256)` 的规范化内容树哈希(复用 `rurix-pkg::content_tree::hash_entries`,RXS-0090/0093);**从 bundle.json 可预算**(`tree_digest_from_bundle`)、**从磁盘经 `collect_dir` 重哈希可复算**(`tree_digest_from_dir`),二者对同一内容树**必相等**。
- **逐组件 sha256 复核**:staging 每组件回读磁盘字节,其 SHA-256 必 == bundle 声明 `Component::sha256`,否则 `InstallError::ComponentDigestMismatch`(拒装、清 staging)。
- **语言本体同一版号**(RXS-0135 判据延续)先于任何落盘校验:任一语言本体组件版号 ≠ bundle `rurix_version` → `InstallError::VersionSkew`(目标不诞生)。

**Dynamic Semantics**:

- 物化序:(1) staging 目录写全部组件(`toolchains\` 与注册表未触碰)→(2) 逐组件回读 sha256 复核 == bundle 声明 → (3) `tree_digest` 磁盘侧复算 == bundle 侧预算 →(4) **提交 = staging → `toolchains\<version>` 同卷单次目录 rename**(提交点唯一,无逐文件半拷贝态)→(5) 注册表 v2 单写(先写 `.tmp` 再 rename)。任一步失败 → **清 staging、不写注册表**(回滚,`InstallError`)。
- **重装幂等**:目标已存在且 `tree_digest` 匹配 → 命中(`idempotent_hit=true`),不重物化;同源两次 install 后 `toolchains.json` 逐字节一致。
- **断电语义 = 「版本目录只经 rename 诞生」不变量**:staging 残留下次运行按 `.staging-` 前缀例清孤儿;rename 后注册前断电 → install 幂等重跑 `collect_dir` 重校验,匹配即补注册(修复而非报错)。
- **注册表 schema v2**:`InstalledToolchain` 增 `install_path` + `tree_digest`(`toolchains.json` schema_version 1→2);v1 旧条目(无路径账面项)读入标 **registered-only**(`install_path == None`),`list` 如实区分,不静默升格;`list --verify` 经 `tree_digest_from_dir` 重哈希标注 corrupted 条目(失败模式:已装目录事后损坏)。

**Implementation Requirements**:

- 复用 `rurix-pkg::content_tree`(`hash_entries` / `collect_dir`)+ `rurix-pkg::sha256`(零外部依赖、纯函数确定性);`rurixup` 默认 `unsafe_code=deny`(纯 Rust,无 FFI)。物化失败以**工具层 `InstallError` + 退出码 1 + 机器 token `RURIXUP_INSTALL_ERROR: kind=<integrity|io|usage>`** 表达,零新 RX 码(§3)。`RURIXUP_INSTALL:` 摘要行纯追加 `components=.. digest_levels_verified=4 installed=<path>`(既有 `version/channel/default/registered` 字段 0-byte,RXS-0187 语义只增)。

> 锚定测试:`src/rurixup/src/install.rs`(`materialize_green_bidirectional_tree_digest_and_bytes`:磁盘树在 + 逐字节 == 源 + tree_digest 双向复算相等;`materialize_is_idempotent`:重装幂等;`materialize_rolls_back_on_component_tamper_zero_residue`:篡改组件 → 逐组件 sha256 拒 + 零残留;`materialize_rejects_version_skew_before_disk`;`component_rel_path_deterministic_rule`)+ `src/rurixup/src/toolchain.rs`(`registry_v2_materialized_roundtrip_and_v1_compat`:v2 install_path/tree_digest round-trip + v1 registered-only 兼容读入;`//@ spec: RXS-0214`)。

### RXS-0215 活跃版本切换（shim,裁决 B）

**Syntax**(shim 代理,`src/rurixup`):

```
<RURIX_HOME>\bin\<name>.exe   ::= rurixup.exe 的拷贝(shim,一次入 PATH)
exe_stem(current_exe) -> String                              // 干名(小写)
forward_if_shim(args)                                        // 干名 ≠ "rurixup" → 代理并透传退出码
resolve_target(home, stem, default, current_exe) -> Result<PathBuf, ShimError>
ShimError ::= NoHome | Registry | NoDefault | Escape | SelfRecursion | TargetMissing | Spawn
setup [--add-path]                                           // 缺省只打印;--add-path 显式改用户 PATH
```

**Legality**:

- `<RURIX_HOME>\bin\<name>.exe` = `rurixup.exe` 的一份拷贝,**一次入 PATH**;`rurixup` 起始按 `current_exe()` 干名判定:干名 == `"rurixup"` → 正常子命令分发;干名 ≠ `"rurixup"` → **代理模式**。
- 代理转发目标 = `toolchains\<default>\bin\<干名>.exe`(其中 `<default>` = `toolchains.json` 的 default 版本);**参数透传、退出码逐位透传、stdio 继承**。
- **防逃逸 / 防自递归**:转发目标**必须**位于 `<RURIX_HOME>\toolchains\` 下(否则 `ShimError::Escape`);目标经规范化**不得**等于 shim 自身路径(否则 `ShimError::SelfRecursion`)。
- **切换 = 注册表 JSON 单写**(`set_default`,原子、免特殊权限、已开 shell 即时生效);切换指向缺失版本目录(已物化条目 `install_path` 目录不存在)→ **诚实报错退出非 0**(`rurixup default` 拒且不写注册表);代理时 default 缺失 / 目标 exe 不存在 → `ShimError` 退出非 0。
- **PATH 接入**:`rurixup setup` **缺省只打印**接入指令(免副作用);`rurixup setup --add-path` 显式 opt-in 才改用户 PATH。

**Dynamic Semantics**:

- `resolve_target` 为**纯路径推导 + 防逃逸/防自递归判定**(host 可测,不 spawn);`forward_if_shim` 在代理成功/失败时 `std::process::exit`(透传子进程退出码 / 错误退出 1),干名 == `"rurixup"` 或无法确定 `current_exe` → 返回交由正常分发。
- 切换后已开 shell 即时生效(shim 每调用读 `toolchains.json` default);代理为每调用一跳进程(毫秒级)。
- `setup --add-path` 经 PowerShell `[Environment]::SetEnvironmentVariable('Path', <new>, 'User')` 幂等追加(已含则 no-op;免 setx 1024 截断),`std::process` 外呼零 unsafe。

**Implementation Requirements**:

- shim / 切换全 safe(`unsafe_code=deny`,仅 `std::process` / `std::fs`,零 unsafe、零第三方);代理/切换失败以**工具层 `ShimError` / `String` + 退出码非 0** 表达,零新 RX 码(§3)。junction 为裁决 B 备选(RFC-0012 §7),本条款落 shim 形态。

> 锚定测试:`src/rurixup/src/shim.rs`(`exe_stem_and_home_derivation`:干名判定 + home 派生;`resolve_target_stays_under_toolchains_and_blocks_escape`:目标恒在 toolchains\ 下;`resolve_target_detects_self_recursion`:目标 == 自身判自递归;`//@ spec: RXS-0215`)。

## 3. 错误码引用汇总

> **本里程碑不新增 RX 错误码**(零追加)。`rurixup` 为独立发布工具(非编译器前端),其发布门失败诊断(未签名/验签失败 / SBOM 缺失或不全 / 再分发白名单违例 / bundle content-tree 完整性不符)以**工具层错误值 + 退出码 + 失败子门枚举**表达——`InstallError`(`IntegrityMismatch` / `VersionSkew`,RXS-0135)、`SigningManifest::upload_permitted=false`(RXS-0137)、`RedistributionAudit::violations`(RXS-0136)、`ReleaseDecision::failed_gates` + 退出码 2(RXS-0139),**而非编译器侧 `RX####` 段位码**;`registry/error_codes.json` 与 `src/rurixc/src/messages/{en,zh}.messages` **本里程碑不动**(对齐 M8.3 pipeline.md §3「rustc 原生诊断而非 RX 段位码」零追加先例)。
>
> 若实现期发现某发布门失败**确需编译器侧 RX 诊断 / 运行期段位码**(如发布产物经 `rurixc` 工具链链接阶段诊断),则**停手标注「需升档」**(§4),按段位 7(链接/工具链,07 §5)`RX70xx` 续接分配(分配制递增、含义冻结、只追加,10 §6),不在本文件自行预造。
>
> NVIDIA EULA Attachment A 白名单逐项法律核对的自主签署状态以证据字段 `eula_whitelist_verdict`(`pending-human-review` / `signed-compliant` / `signed-noncompliant`,沿 M5.4 `redistribution_audit_evidence_schema.json` 先例)表达,**非 RX 段位码**;agent 自主签署(§4)。
>
> **G1.5(§2.5,RXS-0150 ~ RXS-0152)同样零新增 RX 码**:fatbin 装载协商**降级而非 reject**(cubin 未命中 / 拒绝 → 静默降级既有 PTX 路径,沿用 RXS-0076 装载诊断 + RXS-0077 poisoned 状态机);lockfile `[[artifact]]` digest 失配以 `rurix-pkg` 工具层 Result(content-tree 完整性,RXS-0092)/ rurixup 发布门枚举(RXS-0139)表达。`registry/error_codes.json` 与 `en.messages` G1.5 零追加(对齐 G1.1~G1.3 零新码先例)。若实现期确需编译期 / 运行期诊断段位码,则停手标注「需升档」,按段位 7(`RX70xx` 从 **RX7020** 起)续接,不预造。
>
> **V1.2(§2.6,RXS-0185 ~ RXS-0186)同样零新增 RX 码**:未知 channel = 工具层用法错误(`generate` Err → `rurixup` 退出码 1,镜像既有未知参数路径);channel 清单漂移 / 缺失 / 版号不符 = 第 8 子门 `channel-manifest` 红 → `failed_gates` 枚举 + 退出码 2(RXS-0139 hard-block 语义延伸)。`registry/error_codes.json` 与双语 messages V1.2 零追加(bilingual 88/88 不变)。确需段位码则停手标注「需升档」,按段位 7(`RX70xx` 从 **RX7021** 起,RX7020 已用于 edition)续接,不预造。
>
> **post-V1(§2.7,RXS-0187 ~ RXS-0188)同样零新增 RX 码**:工具链前端校验失败以工具层 `ToolchainError`(`ManifestInconsistent` / `DigestMismatch` / `UnknownVersion`)+ `rurixup` 退出码 1 表达(镜像 install.rs `InstallError` 先例);`registry/error_codes.json` 与双语 messages post-V1 零追加(bilingual 88/88 不变)。真实 FS 物化 / 网络拉取(RD-025)若成硬需求且确需诊断段位码,则停手标注「需升档」,按段位 7(`RX70xx` 从 **RX7023** 起——RX7021/7022 已被 MS1 消费)续接,不预造。
>
> **EA1.1a(§2.8,RXS-0214 ~ RXS-0215)同样零新增 RX 码**:真实 FS 物化失败以工具层 `InstallError`(`IntegrityMismatch` / `ComponentDigestMismatch` / `UnknownComponent` / `VersionSkew` / `Io`)+ `rurixup` 退出码 1 + 机器 token `RURIXUP_INSTALL_ERROR: kind=<integrity|io|usage>` 表达;活跃切换 / shim 代理失败以 `ShimError`(`NoDefault` / `Escape` / `SelfRecursion` / `TargetMissing` / …)+ 退出码非 0 表达。`registry/error_codes.json` 与双语 messages EA1.1a 零追加(bilingual 96/96 不变)。网络拉取(URL 下载,EA1.1b RXS-0216~0217)若确需诊断段位码,则停手标注「需升档」,按段位 7(`RX70xx` 从 **RX7023** 起)续接,不预造。

## 4. 升档 / 禁区留痕

- **Azure Artifact Signing 为 of-record 签名后端(agent 裁定)**:全部 EXE/DLL/MSI 经 Authenticode + 时间戳,签名后端 of-record = Azure Artifact Signing(OV 证书或 Azure Artifact Signing,EV 不再豁免 SmartScreen,r6)。**生产签名经 CI secret + 人工门控,本机/CI 不自动调用真实证书**;本地/CI 冒烟以临时自签测试证书产真实 Authenticode 红绿(机器事实层验签判定),Azure 生产路径以 secret 门控分支占位不调用。带档复议按裁决留痕,不擅自切换签名后端。
- **NVIDIA EULA Attachment A 白名单法律签署维持 `pending-human-review`(agent 自主签署)**:NVIDIA 再分发组件仅 Attachment A 白名单最小集(MVP 实际只需 `libdevice.10.bc`,cuBLAS 绑定包按需附带 `cublas64_*.dll`),完整 Toolkit/驱动/Nsight **永不捆绑**(许可红线 r6);白名单逐项法律核对/再分发判定由agent 自主留痕,**agent 起草机器事实**(`check_redistribution` 白名单审计 + content-tree 清单),自主签署(对齐 M5.4 `redistribution_audit` 先例)。
- **cubin/fatbin 真分发(G1,PTX-only)**:M8 维持 **PTX-only** 开发期产物(07 §7);发布 bundle 复用 `rurix-rt` PTX 装载路径,不改 device codegen 分发形态;cubin/fatbin 真分发 → G1(M8 out_of_scope)。触及即停下标注「需升档」。
- **cubin/fatbin 真分发兑现(G1.5,MR-0005,2026-06-22)**:上条 M8 留的「需升档」前向引用agent 自主裁决 **Mini-RFC/MR-0005** 兑现(脱离 PTX-only),条款体 RXS-0150 ~ RXS-0152 落 §2.5——按架构预编 cubin(`ptxas`,sm_89 基线)+ 保守 PTX fallback 装载协商(`select_load_variant`,降级而非 reject,RXS-0076/0077 语义 0-byte)+ lockfile `[[artifact]]` 变体 digest 内容寻址锁定(RXS-0090/0093 复用)。**保守 PTX fallback 兜底**:无 cubin 预编工具链 / 无匹配架构 / cubin 拒绝 → 自动降级既有 PTX JIT 路径,行为等价 M8 PTX-only(前向兼容,D-207)。cubin/fatbin = Rurix 自编**语言本体**(由 Rurix 自研 PTX 经 ptxas 编译,自有许可,**非 NvidiaRedist**),经 SBOM + content-tree 完整性 + NVIDIA 白名单审计延续(`check_redistribution` 扩到 cubin/fatbin:无 `__nv_*` 残留 + 不打包 libdevice .bc/Toolkit/驱动/Nsight,r6)。**真 NVIDIA fatbinary 容器格式 / sm_89 外多架构矩阵**若成硬需求 → 按 14 §4 defer **RD-010**(不预造);不立装载首启延迟性能门(agent 裁,仅功能冒烟 + nightly 趋势)。
- **完整 Toolkit·驱动·Nsight 捆绑(许可红线 r6)**:发布 bundle 永不捆绑完整 CUDA Toolkit / 驱动 / Nsight;违例为许可红线。触及即停下标注「需升档」。
- **Python 原生嵌入(永久红线 1,SG-008)**:发布链路仅分发 C ABI / PYD 产物(对接 interop.md RXS-0122~0125),不分发 Python 解释器宿主 / 原生嵌入(死亡路线,SG-008 维持 not_triggered)。触及即停下标注「需升档」。
- **包 registry sumdb(D-312/G2)**:MVP 发布 = lockfile + vendor + checksum + content-tree SHA-256;真 registry / sumdb 留 G2(D-312,SG-007),**M8 维持 not_triggered**(agent 裁定);触及即停下标注「需升档」。
- **UB 节禁区**:发布产物完整性 / 签名状态 / 再分发面以 **content-tree SHA-256 + 验签判定 + 白名单审计的确定性机器事实** 定义,**严禁 UB 节**(UB 为经 Full RFC 由 agent 自主落笔的高敏面,10 §7.5)。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-17 | 新建 spec/release.md(M8.4 发布产物语义面起始文件):登记编号区间 RXS-0135 起续号预留 + 文件级前言 / 范围(原子分发与 content-tree 完整性 / 语言本体与 NVIDIA 再分发组件分离打包 / 签名清单约定与验签发布前置 / SBOM SPDX+CycloneDX 约定 / Release 层 hard-block 发布门;**复用 M6 包管理 content-tree/lock RXS-0090/0092/0094 与 M5.4 NVIDIA 再分发白名单审计,新增仅补发布产物缺口**;PTX-only、完整 Toolkit/驱动/Nsight 永不捆绑 r6、永不 Python 原生嵌入、registry sumdb 维持 not_triggered、发布门以机器事实定义不设 UB)/ 依据与授权(08 §9 D-241 r6 + 14 §8 Release 层 + 10 §6 工具链发布门 + 09 §6/§7 + 01 §6;M8_CONTRACT D-M8-4 / G-M8-4 / G-M8-7 / RD-001 `rfc_required: none` + M8_PLAN §4 + CI_GATES §3 步骤 38)/ 计划条款骨架(§2 预留,非裸条款头:RXS-0135 原子分发与 content-tree 完整性 / RXS-0136 语言本体与 NVIDIA 再分发组件分离打包 / RXS-0137 签名清单约定与验签发布前置 / RXS-0138 SBOM SPDX+CycloneDX 约定 / RXS-0139 Release 层 hard-block 发布门)/ 错误码说明(§3:rurixup 发布门诊断按需段位 7 RX70xx 续接,**脚手架不预造**,最终随实现 PR 按 07 §5 裁定;EULA 白名单 `eula_whitelist_verdict` 自主签署字段非 RX 码)/ 升档·禁区留痕(§4:Azure Artifact Signing of-record agent 裁定 + 生产签名 secret 门控、EULA Attachment A 白名单 pending-human-review agent 自主签署、cubin/fatbin G1·PTX-only、完整 Toolkit/驱动/Nsight 捆绑 r6 红线、Python 原生嵌入红线 1/SG-008、registry sumdb D-312/G2 not_triggered、UB 节禁区)。**沿 README v1.15 toolchain.md / v1.20 stdlib.md / v1.25 interop.md / v1.27 cublas.md / v1.29 pipeline.md 先例:本轮不落带编号裸条款头**——条款体与 ≥1 测试锚定随 M8.4 实现 PR 同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定),无体例变更 | Direct |
| v1.1 | 2026-06-17 | 落地带编号条款体 RXS-0135 ~ RXS-0139(M8.4 实现 PR,条款体随实现 + 测试锚定同落,§2 计划骨架升格为条款体):RXS-0135 原子分发与 content-tree 完整性(语言本体同一版号单一原子分发单元 + content-tree 规范化 SHA-256 完整性锚 复用 rurix-pkg RXS-0090/0092/0093;原子安装全有或全无、校验失败回滚不留半装)/ RXS-0136 语言本体与 NVIDIA 再分发组件分离打包(LanguageCore ⟂ NvidiaRedist 分区;NVIDIA 仅 Attachment A 白名单最小集 libdevice.<d>.bc + cublas(Lt)?64_<d>.dll,完整 Toolkit/驱动/Nsight 永不捆绑 r6;延续 M5.4 check_redistribution 口径)/ RXS-0137 签名清单约定与验签发布前置(Authenticode + 时间戳;of-record Azure Artifact Signing 生产 secret/人工门控、本地冒烟 SelfSignedTest 自签真实红绿;验签通过=Valid+时间戳为上传前置,未签名/失败/缺时间戳阻断;verified 去重集计入 m8.counter.release_artifacts_signed)/ RXS-0138 SBOM 约定(SPDX-2.3 构建视图 + CycloneDX-1.5 发布视图,组件齐备判据=干名与版次均落两视图,手写确定性 JSON 零依赖)/ RXS-0139 Release 层 hard-block 发布门(签名/SBOM/许可审计/bench-strict/conformance/UI golden/L1 回归任一红 → allow_upload=false 不上传 artifact + failed_gates 确定枚举,退出码 2)。每条 ≥1 锚定(`src/rurixup` 单测:install / bundle / signing / sbom / gate + lib 端到端;trace_matrix 维持全锚定 134→139)。**本里程碑不新增 RX 码**(§3:rurixup 工具层 Result/退出码/失败子门枚举,registry/error_codes.json 与 en.messages 零追加,对齐 M8.3 pipeline.md 先例)。实现裁决:新 crate `src/rurixup` 默认 `unsafe_code=deny`(纯 Rust 无 FFI),复用 rurix-pkg content_tree/sha256;ci/trace_matrix.py 锚定源加入 src/rurixup;PTX-only、不触 cubin/fatbin G1 / 完整 Toolkit r6 / 红线 1 SG-008 / registry sumdb D-312。Azure 为 of-record 签名后端(agent 裁定)、EULA Attachment A 白名单维持 pending-human-review(agent 自主签署),无体例变更 | Direct |
| v1.2 | 2026-06-22 | G1.5 生产分发 fatbin 语义面延伸(Mini-RFC/MR-0005,owner 2026-06-22 经 AskUserQuestion 裁决档位 + 落点 release.md + `[[artifact]]` 落 rurix.lock + 不立性能门)：§1 续号区间补 RXS-0150 ~ RXS-0152 + §2.5 落条款体——RXS-0150 分发产物变体模型与按架构预编 cubin + 保守 PTX fallback(每 `DeviceArtifactSet` 必含 PTX fallback、cubin 由对应 PTX 经 `ptxas -arch` 预编保留字节、复用 RXS-0073 干验证;cubin 不设字节 golden 改结构核对)/ RXS-0151 fatbin 装载协商序(`select_load_variant` 纯函数:cubin 命中即 `cuModuleLoadData`、未命中 / 拒绝降级既有 PTX 版号梯子 `cuModuleLoadDataEx` 即 RXS-0076/0077 语义 0-byte、降级而非 reject 不 poison、装载边界 unsafe-audit U22)/ RXS-0152 lockfile `[[artifact]]` 变体 digest 与内容寻址锁定(复用 rurix-pkg content-tree SHA-256 RXS-0090/0093、`(package,kind,sm_target)` 字典序确定序列化、`[[artifact]]` 追加既有 `[[package]]` schema 0-byte)。§4 追加 cubin/fatbin 真分发兑现 bullet(不动既有 PTX-only bullet);§3 追加 G1.5 零新 RX 码说明(降级而非 reject、digest 失配走工具层 Result、确需则 RX7020 续接停手不预造)。**M8.4 既有条款 RXS-0135 ~ RXS-0139 条款体 0-byte**;cubin/fatbin = Rurix 自编语言本体(非 NvidiaRedist)经 `check_redistribution` 白名单审计延续(无 `__nv_*` 残留、不打包 libdevice .bc/Toolkit/驱动/Nsight r6);不立装载首启延迟性能门(仅功能冒烟 + nightly 趋势)、真 fatbinary / 多架构矩阵 defer RD-010 不预造。依据 [mini-0005-fatbin-distribution.md](../rfcs/mini-0005-fatbin-distribution.md)(D-207 / D-311)、G1_CONTRACT D-G1-5 / G-G1-5 / G-G1-6 + G1_PLAN §5。agent 自主判档,判档争议向上取严,无体例变更 | Mini-RFC(MR-0005) |
| v1.3 | 2026-07-14 | V1.2 最小 stable channel 清单语义面延伸(Mini-RFC/MR-0008,agent Approved 2026-07-14;用户同日裁决范围 = 最小清单,V1_CONTRACT §7 ④):§1 续号区间补 RXS-0185 ~ RXS-0186(**RXS-0181~0184 已被 GRX showcase 分支 claim,跳号避撞,编号永不复用 10 §9.5**)+ §2.6 落条款体——RXS-0185 stable channel 清单存在性·字段语义·确定性序列化(channel ∈ {stable} 首版合法集 / rurix 版号拷贝 bundle / `bundle_manifest_sha256` = bundle.json 字节流 SHA-256 内容寻址引用复用 RXS-0093 / 组件干名字典序 / 同一输入两次产逐字节一致、无时间戳;为 rustup 式前端预留锚点,不实现 install/update/channel 切换,nightly 不建)/ RXS-0186 channel 与 bundle 同版号一致性判据 + Release 层发布门延伸(判据 = channel 合法 + 清单版号 == bundle 版号 RXS-0135 判据延续 + 组件全集一一对应;门集追加末位第 8 子门 channel-manifest,清单缺失/漂移/版号不符 → 发布阻断退出码 2;RXS-0139 既有 7 门相对顺序 0-byte;`--simulate-channel-drift` 故障注入真实红绿,CI 步骤 50 `ci/channel_manifest_smoke.py`)。§3 追加 V1.2 零新 RX 码说明(未知 channel 走工具层用法错误退出码 1,确需则 RX7021 续接停手不预造)。**M8.4/G1.5 既有条款 RXS-0135 ~ RXS-0139、RXS-0150 ~ RXS-0152 条款体 0-byte**;stable 快照(RD-008)因条款计数 180→182 同 PR 重 bless + bless_log 追加(RXS-0180 L2 加性演进)。依据 [mini-0008-stable-channel-manifest.md](../rfcs/mini-0008-stable-channel-manifest.md) + V1_CONTRACT D-V1-3 / G-V1-3 + V1_PLAN §2。agent 自主判档,判档争议向上取严,无体例变更 | Mini-RFC(MR-0008) |
| v1.4 | 2026-07-14 | post-V1 rurixup 工具链前端首切片语义面延伸(Mini-RFC/MR-0009,agent Approved 2026-07-14;兑现 MR-0008 §1 预留的 rustup 式前端锚点,08 §9 D-241 locked 意图):§1 续号区间补 RXS-0187 ~ RXS-0188 + §2.7 落条款体——RXS-0187 工具链版号注册表与默认切换(ToolchainRegistry:多版号注册幂等 + default 指针 + set_default 未注册版号拒 + 确定性序列化 round-trip、无时间戳)/ RXS-0188 stable channel 消费与 install 内容寻址校验(channel 一致性 RXS-0186 + bundle_manifest_sha256 == 实测 sha256(bundle_json)RXS-0093/0135,全有或全无不注册;幂等;首装成 default)。`rurixup install/list/default` 纯 host、纯确定性、零网络端点、零真实 FS 物化;复用 install.rs RXS-0135 原子安装内核 + channel.rs RXS-0186 判据。**真实 FS 物化 + 网络拉取 defer RD-025**(真实 IO/安全包络/网络端点面)。§3 追加 post-V1 零新 RX 码说明(ToolchainError 工具层 + 退出码 1)。**M8.4/G1.5/V1.2 既有条款 RXS-0135 ~ RXS-0186 条款体 0-byte**;stable 快照因条款计数 182→184 同 PR 重 bless + bless_log 追加(RXS-0180 L2 加性演进,同 edition 2026 内只增不破坏);锚定 src/rurixup/src/toolchain.rs 单测 + ci/toolchain_frontend_smoke.py(步骤 51)。依据 [mini-0009-toolchain-frontend.md](../rfcs/mini-0009-toolchain-frontend.md);新增 deferred RD-025(真实 FS 物化 + 网络拉取)。agent 自主判档,判档争议向上取严,无体例变更 | Mini-RFC(MR-0009) |
| v1.5 | 2026-07-17 | EA1.1a rurixup 真实 FS 物化 + 活跃切换语义面延伸(Full RFC/RFC-0012,Approved 2026-07-17;RD-025 兑现,兑现 §2.7 post-V1 defer 的真实 IO 面):§1 续号区间补 RXS-0214 ~ RXS-0215(**RXS-0189~0213 已被 MS1/MB1 承接,续号自 RXS-0214**)+ §2.8 落带编号条款体——RXS-0214 真实 FS 物化与原子落盘(RURIX_HOME 磁盘布局 + 组件干名→相对路径确定性规则 *.exe→bin/·*.lib→bin/lib/·NvidiaRedist→nvidia/;版号目录仅经「staging 全量校验→同卷单次 rename」诞生;逐组件 sha256 复核 == bundle 声明;tree_digest 双向独立复算不变量 从 bundle 预算 == 从磁盘 collect_dir 重哈希;失败零半装、重装幂等、断电孤儿清理;注册表 schema v1→v2 追加 install_path/tree_digest,v1 旧条目读入标 registered-only;复用 rurix-pkg content_tree/sha256 RXS-0090/0093)/ RXS-0215 活跃切换(裁决 B shim:`<home>\bin\<name>.exe` = rurixup 拷贝一次入 PATH,current_exe 干名≠rurixup → 代理转发 toolchains\<default>\bin\<干名>.exe,参数/退出码/stdio 透传;防逃逸 目标必在 toolchains\ 下 + 防自递归 目标≠自身;切换 = 注册表 JSON 单写,指向缺失目录诚实报错非 0;setup 缺省只打印 PATH 指令、--add-path 显式经 PowerShell SetEnvironmentVariable 改用户 PATH)。条款体 FLS 体例(Syntax/Legality/Dynamic Semantics/Implementation Requirements,**严禁 UB 节**)+ 每条 ≥1 `//@ spec` 单测锚定(`src/rurixup/src/{install,toolchain,shim}.rs`)同 PR 落(条款 commit 先于实现 commit,trace_matrix 209→211 全锚定);**零新 RX 码**(InstallError/ShimError 工具层 + 退出码 + 机器 token `RURIXUP_INSTALL_ERROR: kind=..`,§3 追加 EA1.1a 零新码说明;§3 过期取号文字「RX7021 起」→「RX7023 起」顺手修正,RX7021/7022 已被 MS1 消费)+ 零新 unsafe(`unsafe_code=deny` 维持,shim 转发经 std::process 外呼零 unsafe)+ 零第三方依赖(仅 rurix-pkg);stable 快照因条款计数 209→211 同 PR 重 bless + `tests/stable/bless_log.md` 追加(RXS-0180 L2 加性演进);CI 步骤 59 前半 `ci/rurixup_dist_smoke.py`(纯离线 --from-dir,物化+切换探针+幂等 green / 篡改组件红 / default 错向红 / 复原绿 + red_self_test)。**网络拉取 + 四级信任链 defer EA1.1b(RXS-0216~0217)**。**M8.4/G1.5/V1.2/post-V1 既有条款 RXS-0135 ~ RXS-0213 条款体 0-byte**。依据 [RFC-0012](../rfcs/0012-toolchain-real-distribution.md)(Approved 2026-07-17,§4.1~4.3 / §5)+ EA1_CONTRACT D-EA1-2 / G-EA1-2。不触红线/禁区。 | **Full RFC**(RFC-0012) |
