# M8 CI 门禁增量

> 所属契约:[M8_CONTRACT.md](M8_CONTRACT.md)
> 版本:v1.0(2026-06-16)
> 基线:[../m0/CI_GATES.md](../m0/CI_GATES.md) + [../m1/CI_GATES.md](../m1/CI_GATES.md) + [../m2/CI_GATES.md](../m2/CI_GATES.md) + [../m3/CI_GATES.md](../m3/CI_GATES.md) + [../m4/CI_GATES.md](../m4/CI_GATES.md) + [../m5/CI_GATES.md](../m5/CI_GATES.md) + [../m6/CI_GATES.md](../m6/CI_GATES.md) + [../m7/CI_GATES.md](../m7/CI_GATES.md)(全部沿用:runner 约定、PR Smoke 1–33 步、guardrail 含 M1.1/M1.2/M1.4/M3.3/M4.2/M4.3/M5.4/M6/M7 激活项、nightly 含 Compute Sanitizer racecheck+memcheck + measured 基准 + rx test 子进程隔离 + 软光栅 device kernel);本文只规定 M8 期的**增量**。
> 铁律不变:任何新增门禁必须在真实 PR 上以真实失败/通过路径验证过(反 YAML-only,H06 D11.8-2)。

---

## 1. Runner

沿用 M0 §1(自托管 RTX 4070 Ti 开发机)+ M4 §1(device 路径:CUDA Toolkit 含 `ptxas` + Driver API)+ M5 §1(Compute Sanitizer + libdevice bc)+ M6 §1(离线重建 + LSP server)+ M7 §1(数学库/软光栅/UC-03 demo 路径)。M8 新增 runner 预置项(随实现落地时本表修订行留痕):

- **Python 互操作链**(G-M8-1):Python ≥3.10 + PyTorch(CUDA 12.x build)+ nanobind + scikit-build-core,`rx build --emit=pyd` 产 PYD 经 `__cuda_array_interface__` v3 / DLPack 双协议接入 PyTorch 端到端真跑。**M8.1 落地兑现**(见 §7 v1.1):互操作链固定进 [../../requirements.txt](../../requirements.txt)(`--extra-index-url cu128` + `torch==2.7.1+cu128` + `nanobind>=2.0` + `scikit-build-core>=0.10`,CI `deps` 步自动预置);runner 实测 Python 3.12 + torch 2.7.1+cu128(CUDA 12.8,`cuda.is_available()==True`,RTX 4070 Ti / Ada sm_89)+ nanobind 2.12 + scikit-build-core 0.11 + MSVC 2022(cl 14.44)+ CUDA Toolkit v13.3(`ptxas`)。
- **cublas 绑定**(G-M8-2):cublas runtime DLL(Attachment A 白名单最小集,按需附带)+ GEMM/GEMV 三层绑定冒烟;性能采样沿用 BENCH_PROTOCOL §2/§3。
- **发布链路**(G-M8-4):MSI 打包工具链 + winget manifest 校验 + **Azure Artifact Signing**(Authenticode + 时间戳)签名/验签 + SBOM(SPDX/CycloneDX)生成 + 许可白名单审计(check_redistribution 延续)。
- **文档站**(G-M8-6):`rx doc` 生成 + 全量回归冻结跑。

## 2. PR Smoke 追加步骤(编号接 M7 §2 的 29–33)

| # | 步骤 | 失败即红 |
|---|---|---|
| 34 | UC-01 PyTorch 互操作冒烟(契约 G-M8-1 通道;M8.1 落地接入):`rx build --emit=pyd` 产 PYD,经 `__cuda_array_interface__`/DLPack 双协议零拷贝接入 PyTorch,算子替换端到端真跑。**实测命令(M8.1 回填)**:`py -3 ci/uc01_interop_smoke.py`(产 PYD → PyTorch 张量经双协议零拷贝 → 算子替换数值结果对照 + 内建篡改算子结果红绿,写唯一 `evidence/uc01_interop_smoke.json` 的 `operators_passed`);计数核对 `py -3 ci/budget_eval.py`(`m8.counter.uc01_pytorch_operators ≥3`)。建设期互操作未落地 → 0 → normal SKIP 属预期 | 是 |
| 35 | cublas 绑定冒烟(契约 G-M8-2 通道;M8.2 落地接入):GEMM/GEMV 三层绑定端到端 + 性能采样。**实测命令(M8.2 回填)**:cublas 绑定冒烟脚本(三层绑定数值正确性 + runtime DLL 白名单审计,写 `evidence/cublas_*.json` 的 `bindings_passed`);计数核对 `py -3 ci/budget_eval.py`(`m8.counter.cublas_bindings ≥2`);L1/L2 性能 `m8.bench.*`/`m8.ratio.*` measured_local 经 `rx bench` 入口实测回填(RD-003 已收编)。建设期未落地 → 0 → normal SKIP 属预期 | 是 |
| 36 | UC-02 三 stream 重叠流水线冒烟(契约 G-M8-3 通道;M8.2/M8.3 落地接入):affine Context/Stream/Event/Buffer + 跨线程所有权转移 + 流序分配类型化端到端。**实测命令(回填)**:UC-02 流水线冒烟脚本(三 stream 重叠 + 资源生命周期错误类别编译期拦截覆盖,写 `evidence/uc02_*.json` 的 `stream_pipeline_ok`);计数核对 `m8.counter.uc02_stream_pipeline ≥1`。建设期未落地 → 0 → normal SKIP 属预期 | 是 |
| 37 | 诊断双语覆盖核对(契约 G-M8-5 通道,RD-006;M8.x 落地接入,CPU-only):message-key zh/en key 集对齐核对。**实测命令(回填)**:`py -3 ci/bilingual_coverage.py`(解析 `src/rurixc/src/messages/{en,zh}.messages`,断言 zh 与 en key 集合一致,缺键即红,写 `evidence/bilingual_*.json` 的 `coverage_complete`);计数核对 `m8.counter.bilingual_diagnostic_coverage ≥1`。门为 check_* 守卫风格,失败即红(反 YAML-only)。建设期双语未全量 → 0 → normal SKIP 属预期 | 是 |
| 38 | 发布链路签名/SBOM/许可审计冒烟(契约 G-M8-4 通道,RD-001;M8.3 落地接入,**Release 层**):产物签名 + 验签 + SBOM 齐备 + NVIDIA 再分发白名单审计 + artifact 上传。**实测命令(M8.3 回填)**:发布链路冒烟脚本(MSI/winget 打包 → Azure Artifact Signing 签名 → 验签通过 → SBOM SPDX/CycloneDX 生成 → check_redistribution 白名单审计 → 写 `evidence/release_*.json` 的 `signed_artifacts`);计数核对 `m8.counter.release_artifacts_signed ≥1`。详见 §3 Release 层门禁。建设期发布链路未建成 → 0 → normal SKIP 属预期 | 是 |
| 39 | 文档站 `rx doc` 生成冒烟(契约 G-M8-6 子项;M8.x 落地接入,功能冒烟非硬阈门):`rx doc` 产文档站。**实测命令(回填)**:`rx doc` 生成往返冒烟;门为 check_* 守卫风格(不写 budget counter) | 是 |

预算 evaluator(M0 步骤 6)自动合并加载 [m8_budget.json](m8_budget.json)(命名空间冲突即红;evaluator 已配 `m8.counter.uc01_pytorch_operators`/`m8.counter.uc02_stream_pipeline`/`m8.counter.cublas_bindings`/`m8.counter.release_artifacts_signed`/`m8.counter.bilingual_diagnostic_coverage` 五分支,目录/证据缺失 → 0 → normal SKIP,对齐 M4/M5/M6/M7 计数器先例)。**M8 期 PR Smoke 跑 normal 模式**:`m8.counter.*` 建设期未达标 SKIP 属预期;UC-01/UC-02 L1/L2 性能 `m8.bench.*`/`m8.ratio.*` 随各 m8.x 实测回填(**开工 entries 留空,不预欠 estimated 占位**)。**M8 close-out 必须跑 `--strict` 且全局零 estimated 残留**(MVP 验收门"零 estimated 占位",11 §3 / 01 §6;不跨里程碑欠债,14 §3)。

## 3. Release 层门禁(14 §8,RD-001 — M8 新建)

M0~M7 仅建 **PR Smoke** + **Nightly** 两层(Release 层 RD-001 承接 M8)。M8 建成第三层 **Release**(14 §8:bench 严格模式 + hard block + 签名/SBOM/许可审计 + artifact 上传):

- **bench 严格模式**:`py -3 ci/budget_eval.py --strict`(无容错跳过;estimated 即 FAIL;全局零 estimated 残留)。
- **hard block**:任一门(签名 / SBOM / 许可审计 / bench strict / conformance 全绿 / UI golden 全绿 / L1 基准无 Critical 回归)失败 → **不上传 artifact**(发布阻断,10 §6 工具链发布门)。
- **签名**:全部 EXE/DLL/MSI 经 **Azure Artifact Signing**(Authenticode + 时间戳);验签通过方可上传。
- **SBOM**:构建生成 SPDX(发布附 CycloneDX 视图);CI 强制许可白名单审计(`check_redistribution` 延续:NVIDIA 组件仅 Attachment A 白名单最小集,完整 Toolkit/驱动/Nsight 永不捆绑,许可红线 r6)。
- **artifact 上传**:全门绿后上传发布产物 + SBOM + 签名清单。
- **触发**:Release 层在打 tag / 发布工作流触发(非每 PR);PR Smoke 步骤 38 为 Release 层签名/SBOM 子集的冒烟前哨。
- **激活经真实红绿验证**(反 YAML-only):构造未签名产物 / 缺 SBOM / 白名单外组件 → Release 门红 → 修复转绿,run URL 归档。

## 4. Nightly 追加

- 既有 nightly 全保留(M5.4 Compute Sanitizer racecheck+memcheck + M5.3/M5.4 measured 基准 + M6.3 rx test 子进程隔离 + M7.3 软光栅 device kernel + M7.5 软光栅 L3 趋势)。
- **UC-02 多 stream device 路径**(M8 落地):三 stream 重叠 + 跨线程所有权转移的 device 路径纳入既有 Compute Sanitizer racecheck+memcheck nightly 全跑。
- **UC-01/UC-02/cublas L1/L2 基准趋势**:经 `rx bench` 入口纳入 nightly 趋势归档(门禁判定在 close-out `--strict`,nightly 为趋势参考)。
- **全量回归冻结**(M8 收口,G-M8-6):全量 conformance/UI/基准回归纳入 nightly 冻结跑(MVP 验收前回归网常驻绿)。

## 5. Guardrail

沿用 M0 五项 + M1 三项 + M3 一项 + M4(PTX/IR golden bless + unsafe-audit)+ M5(NVIDIA 再分发白名单 / Compute Sanitizer)+ M6(rx fmt 幂等 / rx test 隔离 / 新 crate unsafe_code=deny)+ M7(软光栅 unsafe-audit / PTX golden / Sanitizer 延续)。M8 期动作:

1. **基准 ref 切至 `m7-closed`**:M7 close-out 已完成 `m6-closed → m7-closed` 切换(M7 CI_GATES §6 v1.1 / M7_CONTRACT §8.11),`ci/check_guardrails.py` 无参默认 = `m7-closed`,**M8 开工无需再切**;PR 路径仍以 `GITHUB_BASE_REF` 为准。若 M8 期需再切按 `check_*` 守卫风格 + 双基准核对,留痕本表修订行。
2. **新段位错误码首批分配**(互操作/cublas/发布链路/双语诊断):随 M8.1+ 诊断 PR 留痕,段位按 07 §5 语义分配,分配制递增、含义冻结(10 §6,`check_error_codes` 延续)。**开工脚手架不预造错误码**。
3. **互操作 / FFI unsafe-audit**(PYD/C ABI/DLPack 边界 + cublas 绑定 FFI):凡落 unsafe 须按 AGENTS 硬规则 9 注册条目,每 unsafe 块 `// SAFETY:`;互操作/cublas/发布链路新 crate 默认 `unsafe_code=deny`(FFI 边界 crate 经裁决最小开 unsafe + 注册留痕)。
4. **NVIDIA 再分发白名单审计延续**(M5.4 check_redistribution):cublas runtime DLL 按需附带须经 Attachment A 白名单最小集审计;完整 Toolkit/驱动/Nsight 永不捆绑(许可红线 r6)。
5. **stable API 快照冻结评估**(G-M8-6,M8 MVP 收口激活):M7 无 stable 面;M8 评估 stable 面定义 + 快照机制激活与否,裁决留痕;激活后 stable API 快照变更须经审批 bless。
6. **CI Release 层门禁建成**(RD-001,§3):Release 层 bench --strict + 签名/SBOM/许可审计 + artifact 上传 hard block。

14 §2 常驻集其余项的 M8 期评估结论:

| 项 | 结论 |
|---|---|
| MIR/PTX/IR 文本 golden | M3.3/M4.2 已激活;M8 维持 PTX-only(cubin/fatbin 真分发 → G1),codegen 形态变更纳入既有 PTX/IR golden 核对 |
| stable API 快照 | M7 无 stable 面;**M8 MVP 收口激活评估**(G-M8-6,stable 面定义 + 快照机制激活与否裁决留痕) |
| unsafe-audit 完整性 | M4.3 已激活(rurix-rt);M8 互操作/FFI 边界凡落 unsafe 按硬规则 9 注册;新 crate 维持 `unsafe_code=deny` |
| Compute Sanitizer | M5.4 已激活;M8 UC-02 多 stream device 路径落地后纳入既有 nightly 全跑 |
| NVIDIA 再分发白名单审计 | M5.4 已激活(`check_redistribution`);M8 cublas runtime DLL 按需附带须经 Attachment A 白名单审计;M8 维持 PTX-only,真分发(G1 cubin/fatbin)不在本里程碑 |
| registry sumdb(D-312) | **owner 裁定 M8 复评维持 not_triggered**(MVP = lockfile+vendor+checksum,真 registry 留 G2);SG-007 追加复评 decisions([../../registry/spike_gating.json](../../registry/spike_gating.json)) |

m0~m7 历史预算的回填/冻结走 `check_guardrails.py` 既有机制(measured_local 条目 0-byte;estimated 只允许回填为 measured_local),不属新增激活项。

## 6. 验证程序(对应契约 G-M8-1~G-M8-7 与步骤 34–39)

1. 步骤 34(UC-01 互操作)落地后,构造**篡改算子数值结果**的 PR → 互操作冒烟红;复原转绿,run URL 归档(反 YAML-only)。
2. 步骤 35(cublas 绑定)落地后,构造绑定数值错误 → 红;复原 → 绿,run URL 归档。
3. 步骤 36(UC-02 流水线)落地后,构造资源生命周期违例(应编译期拦截却放行)→ 红;复原 → 绿,run URL 归档。
4. 步骤 37(双语覆盖)落地后,构造缺键(en 有 zh 无)→ 红;复原 → 绿,run URL 归档。
5. 步骤 38 / §3 Release 层落地后,构造未签名产物 / 缺 SBOM / 白名单外组件 → Release 门红;修复转绿,run URL 归档。
6. close-out 附 `budget_eval --strict` 输出原文(MVP 验收门"全部预算阈值 measured_local 零 estimated 残留")+ UC-01/UC-02/UC-03 端到端证据 + 发布链路签名/SBOM 证据 + 双语覆盖证据 + stable API 快照冻结评估结论 + RD-001/RD-006/RD-007 处置留痕。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-16 | 初版(M8 契约配套;步骤 34–39 为 M8.1~M8.x 计划项,落地时回填实测命令;**新建 Release 层门禁** §3,RD-001 承接;guardrail 动作:基准 ref 切至 m7-closed 无需再切、新段位错误码首批分配随 M8.1+ 诊断 PR、互操作/FFI unsafe-audit、NVIDIA 白名单审计延续、stable API 快照冻结评估、Release 层建成均为计划项;SG-007/D-312 M8 复评维持 not_triggered)。配套 `ci/budget_eval.py` 新增 `m8.counter.uc01_pytorch_operators`/`uc02_stream_pipeline`/`cublas_bindings`/`release_artifacts_signed`/`bilingual_diagnostic_coverage` 五 evaluator 分支(目录/证据缺失 → 0 → normal SKIP,对齐 M4/M5/M6/M7 计数器先例);`m8_budget.json` entries 留空(不预欠 estimated bench 占位)+ 五计数器。`py -3 ci/budget_eval.py`(normal)= PASS(m8.* 计数器 SKIP 属预期) |
| v1.1 | 2026-06-16 | **M8.1 落地回填**:步骤 34(UC-01 PyTorch 互操作冒烟)接入 `ci/uc01_interop_smoke.py`(`rx build --emit=pyd` 产 PYD → PyTorch CUDA 张量经 `__cuda_array_interface__` v3 / DLPack 双协议零拷贝 → SAXPY/Reduction/GEMM 算子替换数值对照 + 内建篡改算子结果红绿,写唯一 `evidence/uc01_interop_smoke.json` 的 `operators_passed`;`m8.counter.uc01_pytorch_operators ≥3`)。§1 互操作链 runner 预置兑现(互操作链固定进 `requirements.txt`,CI `deps` 步自动安装;实测 torch 2.7.1+cu128 / nanobind 2.12 / scikit-build-core 0.11 / MSVC 2022 / CUDA Toolkit v13.3)。新增 `milestones/m8/uc01_interop_evidence_schema.json`(evidence schema)接 `ci/check_schemas.py` `uc01_` 路由;新段位错误码首批分配 RX7013~RX7015(互操作诊断:协议不支持 / 设备指针非法 / 形状不匹配)+ en.messages message-key;新 crate `src/rurix-interop`(FFI 边界经裁决最小开 unsafe + `unsafe-audit/rurix-interop.md` 注册,safe wrapper 对上全 safe)。条款 PR(#47 spec/interop.md RXS-0122 脚手架)先于本实现 PR |
| v1.2 | 2026-06-16 | **M8.2 落地回填**:步骤 35(cublas GEMM/GEMV 三层绑定冒烟)接入 `ci/cublas_binding_smoke.py`(`cargo build -p rurix-cublas` 产 cdylib → PyTorch CUDA 张量经 ctypes `rurix_cublas_gemm`/`rurix_cublas_gemv` 设备指针零拷贝调用 cublas → 与 `torch.matmul`/`torch.mv` 数值对照 + 内建篡改绑定结果红绿,写唯一 `evidence/cublas_binding_smoke.json` 的 `bindings_passed`;`m8.counter.cublas_bindings ≥2`)。新增 `milestones/m8/cublas_binding_evidence_schema.json`(evidence schema)接 `ci/check_schemas.py` `cublas_` 路由;新段位错误码首批分配 RX7016~RX7019(cublas 诊断:句柄初始化失败 / 设备指针非法 / 维度不匹配 / 执行运行时失败)+ en.messages message-key;新 crate `src/rurix-cublas`(cublas v2 C API FFI 边界经裁决最小开 unsafe + `unsafe-audit/rurix-cublas.md` 注册,cublas runtime DLL 动态加载对齐 rurix-rt nvcuda 先例,safe wrapper 对上全 safe);`ci/check_redistribution.py` 延伸断言 3(cublas runtime DLL 不入产物 + 动态加载候选限 Attachment A 白名单 `cublas64_*.dll`,r6,RXS-0129);`ci/trace_matrix.py` 锚定源加入 `src/rurix-cublas`。UC-01/UC-02 L1/L2 性能 measured_local 回填 `m8.bench.*`/`m8.ratio.*`(自研 SAXPY/Reduce/GEMM + cublas GEMM/GEMV vs 手写 CUDA C++ ≥90%,三次进程级独立运行经 `rx bench` 协议化采样,BENCH_PROTOCOL §3;`budget_eval --strict` 对 M8.2 新增条目 PASS)。条款 PR(#49 spec/cublas.md RXS-0126 脚手架)先于本实现 PR |
| v1.3 | 2026-06-17 | **M8.4 Release 分阶段 strict 接线修正**:Release workflow 保留 bench 严格模式与 hard-block,但 M8.4 发布链路先于 M8.5 双语覆盖落地,因此 `ci/budget_eval.py` 新增显式 `--allow-pending <counter>` 仅供 strict 模式下保留未到期计数器 SKIP。`release.yml` 签名前跑 `py -3 ci/budget_eval.py --strict --allow-pending m8.counter.release_artifacts_signed --allow-pending m8.counter.bilingual_diagnostic_coverage`(release 证据尚未生成、双语属 M8.5);`ci/release_pipeline_smoke.py` 真实签名/SBOM/许可审计写 `evidence/release_pipeline_smoke.json`;签名后跑 `py -3 ci/budget_eval.py --strict --allow-pending m8.counter.bilingual_diagnostic_coverage`,此时 `m8.counter.release_artifacts_signed ≥1` 必须 PASS,否则不上传 artifact。这样 M8.4 绿跑证明发布签名产物,同时不把 M8.5 双语未来门提前误阻断 |
| v1.4 | 2026-06-17 | **M8.4 Release 真实红绿与 tag 发布归档**:红跑 `workflow_dispatch` 于临时分支 `codex/m8.4-release-redgate` 注入 `--simulate-missing-sbom`,run [27675184250](https://github.com/qwasg/Rurix/actions/runs/27675184250) 在 `Release pipeline sign/SBOM/audit gate` 失败,日志含 `RURIXUP_RELEASE: allow_upload=false signed_artifacts=1 sbom_present=false audit_pass=true failed_gates=[sbom]`,后续 `budget eval release counter` 与 upload 均 skipped(发布门 hard-block 兑现)。绿跑推 tag `v0.1.0-m8.4` 指向 `4d813ce89c04eb47d46721a903668aaf117b9c1e`,run [27676332410](https://github.com/qwasg/Rurix/actions/runs/27676332410) success:`ci/release_pipeline_smoke.py` 真实 Authenticode 绿(`signed_artifacts=['rurixup.exe','rx.exe']`),`budget eval release counter (strict)` PASS(`m8.counter.release_artifacts_signed`=2,64 pass / 1 skip),artifact `rurix-release-v0.1.0-m8.4` 上传(ID 7690095777,digest `sha256:7af908392420f7a26c35d16a1efe11b5798a1dd6c94b6508611ab138d4aaac34`)。证据归档 [../../evidence/release_pipeline_smoke.json](../../evidence/release_pipeline_smoke.json),其 `redgreen.run_url` 同时记录 red/green 两个 run URL |
| v1.5 | 2026-06-17 | **M8.5 双语诊断覆盖落地与真实红绿归档**:步骤 37 接入 `ci/bilingual_coverage.py`(`src/rurixc/src/messages/{en,zh}.messages` key 集完全对齐,缺键/多键即红),新增 `src/rurixc/src/messages/zh.messages`、Rust `zh_aligns_with_en` 单测、`milestones/m8/bilingual_diagnostic_coverage_evidence_schema.json` 与 `evidence/bilingual_diagnostic_coverage.json`;`m8.counter.bilingual_diagnostic_coverage ≥1` 后 `py -3 ci/budget_eval.py --strict` 本地 PASS(65 pass / 0 skip)。真实红绿:baseline 绿 run [27680365968](https://github.com/qwasg/Rurix/actions/runs/27680365968);红跑 commit `ba0befc` 注释 `zh.messages` 的 `cublas.runtime_failed`,run [27680580391](https://github.com/qwasg/Rurix/actions/runs/27680580391) 在 `bilingual diagnostic coverage gate` 失败,日志含 `zh 缺键(1): ['cublas.runtime_failed']`;恢复 commit `3df7a7b` 后 run [27680780231](https://github.com/qwasg/Rurix/actions/runs/27680780231) 全绿。证据归档 [../../evidence/bilingual_diagnostic_coverage.json](../../evidence/bilingual_diagnostic_coverage.json),其 `redgreen.run_url` 记录 baseline/red/restored 三个 run URL |
| v1.6 | 2026-06-17 | **M8.6 文档站步骤 39 落地**:步骤 39(文档站 `rx doc` 生成冒烟)接入 `ci/doc_site_smoke.py`(`cargo build -p rx` → `rx doc --root . --out <A/B>` 两次生成 4 关键页 index/spec/errors/traceability **逐字节 SHA-256 一致**(确定性可复现);断言关键页齐备 + spec.html 含独立扫描 `spec/*.md` 的全部 139 条 RXS 条款锚点 `id="RXS-####"` + errors.html 含 `registry/error_codes.json` 全部 68 个错误码索引项 `id="RX####"`;内置「缺关键页 / 缺条款锚点」red 自检,反 YAML-only)。判档(契约 §7 owner 裁定):`rx doc` 系既有 spec/conformance/API 的工程化呈现,纯工程不造裸条款,归口既有 CLI 分发条款 RXS-0083(`//@ spec` 锚定),trace 维持 139/139,无 spec PR。新增 `src/rx/src/doc.rs`(复用 `rurixc::tooling::json_util` 无 serde 读取器,不引新外部依赖)+ `milestones/m8/doc_site_smoke_evidence_schema.json` 接 `ci/check_schemas.py` `doc_` 前缀路由 + `evidence/doc_site_smoke.json`。门为 check_* 守卫风格,**不写 budget counter**(功能冒烟非硬阈门);`py -3 ci/budget_eval.py --strict` 维持 65 pass / 0 skip 不受影响。真实红绿(反 YAML-only):baseline 绿 run [27684598897](https://github.com/qwasg/Rurix/actions/runs/27684598897);红跑 commit `294912e` 抹 `render_errors` 的 `<tr id="RX####">` 锚点,run [27684925366](https://github.com/qwasg/Rurix/actions/runs/27684925366) 在 `doc-site rx doc generation smoke` 步骤失败(日志含 `errors.html 缺错误码索引项 68 条`);复原 commit `345ef74` 后 run [27685136456](https://github.com/qwasg/Rurix/actions/runs/27685136456) 全绿(其中一次 clippy 命中 os error 1455 页面文件不足 transient,rerun 即绿)。证据归档 [../../evidence/doc_site_smoke.json](../../evidence/doc_site_smoke.json),`redgreen.run_url` 记录 baseline/red/restored 三 run URL |
| v1.7 | 2026-06-17 | **M8 close-out 人工签署落档**:owner 裁决 M8 正式关闭(`active→closed`)并批准 MVP 验收判定;guardrail 回退基准默认值 `m7-closed→m8-closed`(`ci/check_guardrails.py` docstring / `resolve_base()` 注释与返回值)。本地签署前复核:`budget_eval --strict` PASS(65 pass / 0 skip)、`trace_matrix --check` PASS(139/139)、`check_schemas` PASS、`check_guardrails.py m7-closed` PASS;`m8-closed` tag 锚定本 close-out 签署提交后 `check_guardrails.py m8-closed` PASS(0 changed paths)。RD-001/RD-006 formal close(`inherited→closed`,registry/deferred.json v1.11);RD-007 维持 inherited;RD-008 维持 open。 |
