---
contract: M8
title: 互操作、加固与 MVP 验收——UC-01/UC-02 互操作 / cublas 包 / 发布链路 / 双语发布门 / MVP 收口
status: closed            # active → closed(close-out 只追加,既有条款 0-byte 修改;M8 close-out 终审 §8 自主签署)
version: v1.0
date: 2026-06-16
timebox: "M+15 ~ M+18(约 12 周,两级结构见 M8_PLAN.md)"
rfc_required: none        # UC-01(PyTorch 算子替换:PYD/nanobind/DLPack/__cuda_array_interface__)/ UC-02(三 stream 重叠流水线)/ cublas 包绑定 / 发布链路(rurixup/MSI/winget + 签名/SBOM)/ 双语发布门 / 文档站 是对 01/02/08/09/11 已锁定决策的条款化与工程实现:纯追加。M8 新决策面(签名后端 / 分发格式 / D-312 是否触发 / stable 面冻结)经开工 agent 确认裁定(见 §7 裁决留痕);任何偏离已锁定决策的语义动作按 10 §3 升档,agent 自主判档,判档争议向上取严
upstream_docs:
  - "11 §3 (M8 定义,互操作、加固与 MVP 验收;MVP 验收门 = 01 §6 第一层全量)"
  - "01 §6 / 02 §(UC-01 PyTorch 算子替换 / UC-02 三 stream 重叠流水线 / UC-03 端到端)"
  - "09 §(PYD/nanobind/scikit-build-core + __cuda_array_interface__ v3 + DLPack 双协议;cublas 绑定包)"
  - "08 §9 (发布链路:rurixup 引导 + MSI + winget + 签名/SBOM/许可审计 + artifact 上传)"
  - "07 §7 (device codegen 分发:M8 维持 PTX-only;cubin/fatbin 真分发 → G1)"
  - "10 §6 (工具链发布门:conformance/UI/L1 基准 + SBOM/签名;诊断双语覆盖核对入 CI)"
  - "14 §1 §3 §4 §8 (契约/预算/deferred/Release 层门禁)"
in_scope:
  - uc01_pytorch_interop    # UC-01 PyTorch 瓶颈算子替换:rx build --emit=pyd 产 PYD(nanobind + scikit-build-core)+ __cuda_array_interface__ v3 / DLPack 双协议零拷贝接入(02 §U1 / 09;11 §3 M8)
  - cublas_pkg              # cublas 绑定包:GEMM/GEMV 三层绑定(raw FFI / safe wrapper / 高层 API),NVIDIA 组件按需附带 runtime DLL(09;08 §9)
  - uc02_stream_pipeline    # UC-02 三 stream 重叠流水线:affine Context/Stream/Event/Buffer + 跨线程所有权转移 + 流序分配类型化端到端(02 §U2;05/06/08)
  - release_chain           # 发布链路:rurixup 引导 + MSI + winget + 签名(Azure Artifact Signing)/SBOM(SPDX+CycloneDX)/许可白名单审计 + artifact 上传 + CI Release 层门禁(RD-001;08 §9 / 14 §8)
  - bilingual_diagnostics   # 诊断消息中英双语全量覆盖(message-key 本地化)+ 覆盖率核对入发布门(RD-006;10 §6)
  - doc_site                # 文档站(rx doc 生成);全量 conformance/UI/基准回归冻结 + stable API 快照冻结评估(11 §3 M8;MVP 收口)
  - spec_m8_clauses         # spec 互操作 / 发布产物语义面条款(新建 spec/interop.md 等,RXS-0122 续号,FLS 体例);**条款 PR 先于实现 PR**
out_of_scope:
  - cubin_fatbin_dist       # libdevice 真分发 / 生产分发 fatbin(按架构 cubin + 保守 PTX fallback)→ G1(07 §7;agent 裁定 M8 维持 PTX-only,不拉前,见 §7)
  - realtime_window_present # 软光栅 demo 升级实时窗口呈现 → G1-1(11 §4 CUDA–D3D12 interop);M8 沿用 M7 离线出图
  - registry_sumdb          # 包 registry(sparse index + sumdb 透明日志 + OIDC/Sigstore)→ D-312/G2(09 §7.3;agent 裁定 M8 复评维持 not_triggered,MVP=lockfile+vendor+checksum,见 §7 / SG-007)
  - multi_backend           # 多后端(AMD/Intel/Metal/Vulkan/SPIR-V)→ G2 + 解除红线 3(SG-003)
  - python_native_embed     # Python 原生嵌入永久裁剪(红线 1,仅 C ABI/PYD 通道,SG-008 维持 not_triggered)
  - advanced_gpu_intrinsics # Tensor Core/WGMMA/TMA / cluster / 动态并行 / cooperative groups 永久裁剪(11 §2 红线,SG-001/SG-002 维持 not_triggered)
  - const_generic_value_mono # const 泛型值运行期单态化(RD-007)随 device codegen 进一步扩展评估接通——非本契约验收门;M8 互操作/绑定若触发数组长度类 const 泛型则按需接通或继续留痕
deferred_refs: [RD-001, RD-006, RD-007]   # RD-001(发布链路 Release 层,M8 承接)/ RD-006(诊断双语全量覆盖,M8 承接)/ RD-007(const 泛型值单态化,M7→M8 顺延评估,inherited);M8 开工无预造新 deferred,执行期按需登记 RD-###(14 §4)
deliverables:
  - id: D-M8-1
    name: UC-01 PyTorch 算子替换(rx build --emit=pyd 产 PYD + nanobind + __cuda_array_interface__/DLPack 双协议)+ spec 互操作条款先行(新建 spec/interop.md,RXS-0122 续号)(G-M8-1)
  - id: D-M8-2
    name: cublas 绑定包(GEMM/GEMV 三层绑定)+ UC-01/UC-02 L1/L2 性能判据 measured_local 回填(G-M8-2)
  - id: D-M8-3
    name: UC-02 三 stream 重叠流水线端到端(affine Context/Stream/Event/Buffer + 跨线程所有权转移 + 流序分配类型化)(G-M8-3)
  - id: D-M8-4
    name: 发布链路(rurixup + MSI + winget + Azure 签名/SBOM/许可审计 + artifact 上传)+ CI Release 层门禁(RD-001)(G-M8-4)
  - id: D-M8-5
    name: 诊断消息中英双语全量覆盖 + 覆盖率核对入发布门(RD-006)(G-M8-5)
  - id: D-M8-6
    name: 文档站(rx doc 生成)+ 全量 conformance/UI/基准回归冻结 + stable API 快照冻结评估(G-M8-6)
  - id: D-M8-7
    name: spec M8 条款(互操作 / 发布产物语义面,RXS-0122 续号,条款 PR 先于实现 PR)(G-M8-7)
acceptance_gates:
  - id: G-M8-1
    check: "UC-01 PyTorch 算子替换端到端:rx build --emit=pyd 产 PYD(nanobind + scikit-build-core),经 __cuda_array_interface__ v3 / DLPack 双协议零拷贝接入 PyTorch,算子替换端到端真跑(SAXPY/Reduction/GEMM 类),覆盖计数 m8.counter.uc01_pytorch_operators ≥ 预设算子集数(estimated 工程选择,增删经 Direct PR 留痕,对齐 G-M5-1 UC-01 判据);CI 批跑(互操作冒烟步骤),失败即红。激活经真实红绿验证(篡改算子数值结果 → 红 → 复原绿,run URL 归档,反 YAML-only)"
  - id: G-M8-2
    check: "cublas 绑定包 + UC-01/UC-02 性能判据:GEMM/GEMV 三层绑定覆盖计数 m8.counter.cublas_bindings ≥ 预设绑定数;L1/L2 性能判据(自研 / 绑定 kernel ≥ 手写 CUDA C++ 对照 90%,01 §6 UC-01 判据)由 m8.bench.* / m8.ratio.* measured_local 于各 m8.x 实测回填(BENCH_PROTOCOL §3 协议,direction/阈值裁定;零 estimated 占位,不跨里程碑欠债 14 §3),close-out 跑 budget_eval --strict 通过"
  - id: G-M8-3
    check: "UC-02 三 stream 重叠流水线端到端:affine Context/Stream/Event/Buffer + 跨线程所有权转移 + 流序分配类型化经全管线真跑(三 stream 重叠 + 资源生命周期 100% 编译期拦截类别覆盖),端到端证据计数 m8.counter.uc02_stream_pipeline ≥1;CI 批跑,失败即红"
  - id: G-M8-4
    check: "发布链路 + CI Release 层门禁(RD-001):rurixup 引导 + MSI + winget 打包,全部 EXE/DLL/MSI 经 Azure Artifact Signing(Authenticode + 时间戳)签名,SBOM(SPDX 构建生成 + CycloneDX 发布视图)+ 许可白名单审计(check_redistribution 延续,NVIDIA 组件仅 Attachment A 白名单最小集),artifact 上传;签名产物计数 m8.counter.release_artifacts_signed ≥1。Release 层门禁(14 §8):bench --strict + hard block + 签名/SBOM/许可审计 + artifact 上传,任一缺失即红(反 YAML-only,激活经真实红绿验证)"
  - id: G-M8-5
    check: "诊断消息中英双语全量覆盖(RD-006):message-key 中英双语全量回填,覆盖率核对入发布门(10 §6),双语覆盖完整计数 m8.counter.bilingual_diagnostic_coverage ≥1(zh/en key 集对齐,缺键即红);CI 批跑(双语覆盖核对步骤),失败即红"
  - id: G-M8-6
    check: "MVP 收口冻结:文档站 rx doc 生成端到端;全量 conformance/UI/基准回归冻结(conformance 全绿 + UI golden 全绿 + L1 基准无 Critical 回归,10 §6 发布门);stable API 快照冻结评估(M7 无 stable 面,MVP 收口激活评估——stable 面定义 + 快照机制激活与否经裁决留痕)。MVP 验收门(11 §3 / 01 §6 第一层全量):UC-01/UC-02/UC-03 三大旗舰用例端到端 + L1/L2 性能判据达标 + 预设资源生命周期错误类别 100% 编译期拦截 + 全部预算阈值 measured_local(零 estimated 占位)"
  - id: G-M8-7
    check: "traceability 延续:M8 新增 RXS 条款(新建 spec/interop.md 等,RXS-0122 续号:互操作 / 发布产物语义面)每条 ≥1 测试锚定;ci/trace_matrix.py 全局口径核对(m1.counter.spec_clause_test_anchoring 全局断言,无需另立 m8 计数器);条款 PR 先于实现 PR"
guardrails:
  - "milestones/m0~m7 的 measured_local 既有预算条目 git diff 0-byte(新增条目允许)"
  - "milestones/m0~m7 的 M*_CONTRACT.md(均 closed)既有内容只追加不修改"
  - "registry/deferred.json 与 registry/spike_gating.json 只追加(既有条目修改触发审查);RD-001/RD-006 仅允许 open→inherited→closed 的状态留痕追加(agent M8 承接为生命周期既定动作);RD-007 仅允许 inherited→closed;SG 复评只追加 decisions(SG-007/D-312 M8 复评维持 not_triggered)"
  - "registry/error_codes.json 错误码语义可加不可改(M1.1 已激活);M8 新段位(互操作/cublas/发布链路/双语诊断)首批分配随 M8.1+ 诊断 PR 留痕,段位分配制递增、含义冻结"
  - "evidence/ 只增不删不改"
  - "00–14 共 15 份规划文档不被执行 PR 改写(勘误走 00 §6.3 追加式修订)"
  - "tests/ui/ 的 .stderr snapshot 变更必须经审批 bless(M1.4 已激活,check_ui_bless)"
  - "tests/mir/ 的 .mir golden 变更必须经审批 bless(M3.3 WP6 已激活,check_mir_bless)"
  - "tests/ptx/ 的 IR golden 变更必须经审批 bless(M4.2 已激活,check_ptx_bless);M8 维持 PTX-only 开发期产物,cubin/fatbin 真分发 → G1"
  - "spec/ 变更必须携带变更档位标记(M1.2 已激活);spec/interop.md 等新建 + RXS-0122 续号,条款 PR 先于实现 PR,每条 ≥1 测试锚定(G-M8-7)"
  - "src/rurix-rt 的 unsafe 边界维持 undocumented_unsafe_blocks=deny(M4.3 已激活);全仓其余 crate 维持 unsafe_code=deny;互操作(PYD/C ABI/DLPack 边界)/cublas 绑定 FFI 凡落 unsafe 须每 unsafe 块 // SAFETY: + 注册条目(AGENTS 硬规则 9),新 crate 默认 unsafe_code=deny"
  - "NVIDIA 再分发白名单审计维持(M5.4 check_redistribution 已激活);cublas 绑定包按需附带 runtime DLL 须经 Attachment A 白名单最小集审计,完整 Toolkit/驱动/Nsight 永不捆绑(许可红线 r6)"
  - "guardrail 核对基准切至 m7-closed(M7 close-out 已完成 m6-closed→m7-closed 切换,M8 开工无需再切;ci/check_guardrails.py 无参默认 = m7-closed;PR 路径仍以 GITHUB_BASE_REF 为准);若 M8 期需再切按 check_* 守卫风格 + 双基准核对"
  - "Compute Sanitizer racecheck+memcheck nightly 维持全绿(M5.4 已激活);UC-02 多 stream device 路径落地后纳入既有 nightly 全跑"
  - "stable API 快照冻结评估(M8 MVP 收口激活):stable 面定义 + 快照机制激活与否经裁决留痕(14 §2 常驻集);激活后 stable API 快照变更须经审批 bless"
  - "本契约 in_scope/acceptance_gates 等既有条款 0-byte 修改,close-out 只追加"
---

# M8 契约 — 互操作、加固与 MVP 验收(UC-01/UC-02 互操作 / cublas 包 / 发布链路 / 双语发布门 / MVP 收口)

> 所属:[../../11_ROADMAP.md](../../11_ROADMAP.md) §3 M8 / 契约机制见 [../../14_ENGINEERING_DISCIPLINE.md](../../14_ENGINEERING_DISCIPLINE.md) §1
> 规范先行延续(AGENTS.md 硬规则第 7 条):互操作 / 发布产物的语义面 PR 必须引用 RXS-#### 条款号(新建 `spec/interop.md`,RXS-0122 续号);缺条款先补 spec,**条款 PR 先于实现 PR**。
> 基准 ref:**切至 `m7-closed`**(M7 close-out 已完成 `m6-closed → m7-closed` 切换,M8 开工**无需再切基准**;`ci/check_guardrails.py` 无参默认 = `m7-closed`,PR 路径仍以 `GITHUB_BASE_REF` 为准)。

---

## 1. 目标

把 Rurix 从 M7 的"能跑出旗舰图形用例"(core 数学库 / image-io / G0 软光栅 / UC-03 demo)推进到 **互操作、加固与 MVP 验收**:兑现 **UC-01 PyTorch 瓶颈算子替换**(`rx build --emit=pyd` 产 PYD + nanobind + `__cuda_array_interface__` v3 / DLPack 双协议零拷贝接入,02 §U1 / 09);落下 **cublas 绑定包**(GEMM/GEMV 三层绑定);接通 **UC-02 三 stream 重叠流水线**(affine Context/Stream/Event/Buffer + 跨线程所有权转移 + 流序分配类型化,02 §U2);建成 **发布链路**(`rurixup` 引导 + MSI + winget + **Azure Artifact Signing** 签名 / SBOM / 许可审计 + artifact 上传 + **CI Release 层门禁**,RD-001 / 08 §9 / 14 §8);回填 **诊断消息中英双语全量覆盖**并纳入发布门(RD-006 / 10 §6);产 **文档站**(`rx doc`)并完成 **全量 conformance/UI/基准回归冻结 + stable API 快照冻结评估**。M8 结束 = **MVP 验收**(01 §6 第一层全量):UC-01/UC-02/UC-03 三大旗舰用例端到端、L1/L2 性能判据达标、预设资源生命周期错误类别 100% 编译期拦截、**全部预算阈值 `measured_local`(零 estimated 占位——上一项目最大教训的硬性反转)**。

## 2. 范围

### 2.1 in-scope

| 项 | 说明 | 对应交付物 |
|---|---|---|
| UC-01 PyTorch 互操作 | `rx build --emit=pyd` 产 PYD(nanobind + scikit-build-core)+ `__cuda_array_interface__` v3 / DLPack 双协议零拷贝(02 §U1 / 09;11 §3 M8) | D-M8-1 |
| cublas 绑定包 | GEMM/GEMV 三层绑定(raw FFI / safe wrapper / 高层 API);runtime DLL 按需附带(白名单审计) | D-M8-2 |
| UC-02 三 stream 流水线 | affine Context/Stream/Event/Buffer + 跨线程所有权转移 + 流序分配类型化端到端(02 §U2;05/06/08) | D-M8-3 |
| 发布链路 | `rurixup` + MSI + winget + **Azure 签名** / SBOM / 许可审计 + artifact 上传 + CI Release 层门禁(RD-001;08 §9 / 14 §8) | D-M8-4 |
| 双语诊断 | 诊断消息中英双语全量覆盖 + 覆盖率核对入发布门(RD-006;10 §6) | D-M8-5 |
| 文档站 + MVP 冻结 | `rx doc` 生成;全量 conformance/UI/基准回归冻结 + stable API 快照冻结评估 | D-M8-6 |
| spec M8 条款 | 互操作 / 发布产物语义面 spec 条款(新建 `spec/interop.md`,RXS-0122 续号,FLS 体例);**条款 PR 先于实现 PR** | D-M8-1 ~ D-M8-4 |

### 2.2 out-of-scope(显式排除)

- libdevice 真分发 / 生产分发 fatbin(按架构预编 cubin + 保守 PTX fallback)——→ G1(07 §7);**agent 裁定 M8 维持 PTX-only 开发期产物,不拉前**(见 §7 裁决留痕)。
- 软光栅 demo 升级为**实时窗口呈现**——→ G1-1(11 §4 CUDA–D3D12 interop);M8 沿用 M7 离线出图。
- 包 registry(sparse index + sumdb 式透明日志 + scopes/OIDC trusted publishing/Sigstore)——→ agent决策点 **D-312**(09 §7.3 阶段三 / G2 期 11 §5);**agent 裁定 M8 复评维持 `not_triggered`**(MVP = lockfile + vendor + checksum),见 §7 / [../../registry/spike_gating.json](../../registry/spike_gating.json) SG-007。
- 多后端(AMD/Intel/Metal/Vulkan/SPIR-V)——→ G2 + 解除红线 3(SG-003)。
- Python 原生嵌入永久裁剪(死亡路线红线 1;仅保留 C ABI / PYD 通道,SG-008 维持 not_triggered)。
- 11 §2 MVP 红线清单全部不触碰:Tensor Core/WGMMA/TMA intrinsics、cluster、动态并行、cooperative groups([../../registry/spike_gating.json](../../registry/spike_gating.json) SG-001 ~ SG-009 维持 not_triggered)。
- const 泛型值运行期单态化(RD-007)随 device codegen 进一步扩展评估接通——**非本契约验收门**;M8 互操作 / cublas 绑定若触发数组长度类 const 泛型则按需接通或继续留痕(执行期处置)。

## 3. 交付物清单

| ID | 交付物 | 形态 | 完成判据 |
|---|---|---|---|
| D-M8-1 | UC-01 PyTorch 互操作 | `rx build --emit=pyd` 产 PYD + nanobind + `__cuda_array_interface__`/DLPack 双协议 + spec 互操作条款(新建 spec/interop.md,RXS-0122 续号) | G-M8-1 + G-M8-7 |
| D-M8-2 | cublas 绑定包 | GEMM/GEMV 三层绑定 + UC-01/UC-02 L1/L2 性能 measured_local 回填 | G-M8-2 |
| D-M8-3 | UC-02 三 stream 流水线 | affine Context/Stream/Event/Buffer + 跨线程所有权转移 + 流序分配类型化端到端 | G-M8-3 |
| D-M8-4 | 发布链路 + Release 层门禁 | rurixup + MSI + winget + Azure 签名/SBOM/许可审计 + artifact 上传 + CI Release 层 | G-M8-4 |
| D-M8-5 | 双语诊断 | 诊断消息中英双语全量覆盖 + 覆盖率核对入发布门 | G-M8-5 |
| D-M8-6 | 文档站 + MVP 冻结 | rx doc 生成 + 全量回归冻结 + stable API 快照冻结评估 | G-M8-6 |
| D-M8-7 | spec M8 条款 | 互操作 / 发布产物语义面(RXS-0122 续号,条款 PR 先于实现 PR) | G-M8-7 |

## 4. 验收门(完整版,YAML 头为可提取摘要)

1. **G-M8-1(UC-01 PyTorch 算子替换端到端)**:`rx build --emit=pyd` 产 PYD(nanobind + scikit-build-core),经 `__cuda_array_interface__` v3 / DLPack 双协议零拷贝接入 PyTorch,算子替换端到端真跑;覆盖计数 `m8.counter.uc01_pytorch_operators ≥` 预设算子集(estimated 工程选择,增删经 Direct PR 留痕)。**真实红绿验证**(篡改算子数值结果 → 红 → 复原绿,run URL 归档,反 YAML-only)。
2. **G-M8-2(cublas 绑定 + UC-01/UC-02 性能判据)**:GEMM/GEMV 三层绑定覆盖 `m8.counter.cublas_bindings ≥` 预设绑定数;L1/L2 性能(≥ 手写 CUDA C++ 对照 90%,01 §6 UC-01 判据)由 `m8.bench.*` / `m8.ratio.*` `measured_local` 于各 m8.x 实测回填(BENCH_PROTOCOL §3;`direction`/阈值裁定;**零 estimated 占位,不跨里程碑欠债** 14 §3),close-out 跑 `budget_eval --strict` 通过。
3. **G-M8-3(UC-02 三 stream 重叠流水线)**:affine Context/Stream/Event/Buffer + 跨线程所有权转移 + 流序分配类型化经全管线真跑(三 stream 重叠 + 资源生命周期错误类别编译期拦截);端到端证据计数 `m8.counter.uc02_stream_pipeline ≥1`。CI 批跑,失败即红。
4. **G-M8-4(发布链路 + CI Release 层门禁,RD-001)**:`rurixup` + MSI + winget 打包,全部 EXE/DLL/MSI 经 **Azure Artifact Signing**(Authenticode + 时间戳),SBOM(SPDX 构建 + CycloneDX 发布视图)+ 许可白名单审计 + artifact 上传;签名产物计数 `m8.counter.release_artifacts_signed ≥1`。**Release 层门禁**(14 §8):bench `--strict` + hard block + 签名/SBOM/许可审计 + artifact 上传,任一缺失即红(激活经真实红绿验证,反 YAML-only)。
5. **G-M8-5(诊断双语全量覆盖,RD-006)**:message-key 中英双语全量回填,覆盖率核对入发布门(10 §6),双语覆盖完整计数 `m8.counter.bilingual_diagnostic_coverage ≥1`(zh/en key 集对齐,缺键即红)。CI 批跑(双语覆盖核对步骤)。
6. **G-M8-6(MVP 收口冻结)**:文档站 `rx doc` 生成端到端;全量 conformance/UI/基准回归冻结(conformance 全绿 + UI golden 全绿 + L1 基准无 Critical 回归,10 §6);stable API 快照冻结评估(M7 无 stable 面,MVP 收口激活评估——stable 面定义 + 快照机制激活与否经裁决留痕)。**MVP 验收门**(11 §3 / 01 §6 第一层全量):UC-01/UC-02/UC-03 端到端 + L1/L2 性能达标 + 资源生命周期错误类别 100% 编译期拦截 + 全部预算阈值 `measured_local`(零 estimated 占位)。
7. **G-M8-7(traceability 延续)**:M8 新增 RXS 条款(新建 `spec/interop.md` 等,RXS-0122 续号)每条 ≥1 测试锚定;`ci/trace_matrix.py` 全局口径核对(`m1.counter.spec_clause_test_anchoring` 全局断言,无需另立 m8 计数器);条款 PR 先于实现 PR。

## 5. Guardrails(字节级,机器核对)

见 YAML 头 `guardrails` 字段。核对方式:`ci/check_guardrails.py [基准ref]`(**默认基准切至 `m7-closed`**,M7 close-out 已完成 `m6-closed → m7-closed` 切换,M8 开工无需再切;PR 路径仍以 `GITHUB_BASE_REF` 为准)。M8 期计划动作:**(1)新段位错误码首批分配**(互操作/cublas/发布链路/双语诊断,随 M8.1+ 诊断 PR 留痕,分配制递增、含义冻结);**(2)互操作/FFI unsafe-audit**(PYD/C ABI/DLPack 边界 + cublas FFI,凡落 unsafe 须 `// SAFETY:` + 注册条目);**(3)NVIDIA 再分发白名单审计**(cublas runtime DLL 按需附带须经 Attachment A 白名单,check_redistribution 延续);**(4)stable API 快照冻结评估**(M8 MVP 收口激活,激活与否裁决留痕);**(5)CI Release 层门禁建成**(14 §8:bench --strict + 签名/SBOM/许可审计 + artifact 上传,RD-001)。M0~M7 历史预算的回填/冻结与既有 bless/spec/error_codes guardrail 走既有机制,无需新代码。

## 6. Deferred 引用

| 编号 | 内容摘要 | 承接 |
|---|---|---|
| RD-001 | CI Release 层门禁(bench 严格模式 + 签名/SBOM/许可审计 + artifact 上传) | M8(M0 登记,open→inherited;发布链路 rurixup/MSI/winget 开工同步建成,14 §8 Release 层结构) |
| RD-006 | 诊断消息中英双语全量覆盖(message-key 本地化文件) | M8(M1 登记,open→inherited;MVP 收口前全量回填并纳入发布门,10 §6;覆盖率核对进 CI) |
| RD-007 | const 泛型值运行期单态化(turbofish const 实参 → 实例值代入 + codegen) | M8(M7 close-out agent M7→M8 顺延,inherited;M8 互操作/绑定作用面若触发数组长度类 const 泛型则按需接通或继续留痕,spec/consteval.md RXS-0064 语义不变,回填仅补实现侧。**非本契约验收门**,接通与否执行期处置留痕) |

详情以 [../../registry/deferred.json](../../registry/deferred.json) 为唯一事实源,本表仅引用。RD-002/RD-003/RD-004/RD-005 已 closed。M8 开工无预造新 deferred;执行期做不完的事按 14 §4 追加 `RD-###` 并双侧标注。

## 7. 修订记录 / 开工裁决留痕

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-16 | 初版契约固化(M8 开工脚手架;基准 ref 切至 m7-closed 无需再切;deferred RD-001/RD-006 open→inherited 承接、RD-007 agent M7→M8 顺延维持 inherited;新建 spec/interop.md RXS-0122 续号预留,条款体随 M8.1+ 与测试同 PR;新段位错误码首批分配随 M8.1+ 诊断 PR)。**开工 agent 裁决**(M8 触发新决策面,经 AskQuestion 确认):① 分发格式 = 维持 PTX-only,cubin/fatbin 真分发留 G1(不拉前);② 签名后端 = **Azure Artifact Signing**(of-record,m8.x 发布子里程碑可带档复议);③ D-312/SG-007 包 registry = **评估后维持 not_triggered**(MVP=lockfile+vendor+checksum,真 registry 留 G2);④ base 分支 = #40~#45 合入 main 后基于 main 新建 feat/m8.0-scaffolding。判档:脚手架取 `rfc_required: none`(对齐 M4~M7 先例,高层决策已锁 00–14);各新决策面在对应 m8.x 子里程碑带档位标记落笔,**agent 自主判档,判档争议向上取严** |
| v1.1 | 2026-06-17 | **M8.6 close-out 裁决留痕**(只追加,既有 v1.0 行 0-byte):**stable API 快照冻结评估结论**(G-M8-6 / §5 guardrail #4,经 AskQuestion agent 裁定)= **评估后维持 not_frozen,快照机制不激活**。理据:M8 MVP 为第一层全量首次验收(01 §6),公开面(rx CLI 命令面 + 公开 crate API)仍处 pre-stable/收敛期,过早激活快照 + bless 守卫将锁死尚在演进的接口;冻结机制(stable 面定义 + 快照比对 + bless 审批)激活留首个 stable 发布(post-MVP / G1 期)。registry 留痕:新增 deferred **RD-008**(stable API 快照冻结机制激活,status open,agent_milestone G1);不新立 SG 条目(stable 面冻结非 14 §7 spike-gating 扩张方向,属机制激活时点裁决)。对齐 §7 v1.0 / SG-007 保守裁定先例(维持 not_triggered,真机制留后续决策点)。**AI 不自判,agent 裁定留痕**。文档站 `rx doc`(D-M8-6 子项)判档:系既有 spec/conformance/API 的工程化呈现,纯工程不造裸条款,归口既有 CLI 分发条款 RXS-0083,trace 维持 139/139,无 spec PR(详见 CI_GATES.md §7 v1.6) |
| v1.2 | 2026-06-17 | **M8 close-out 自主签署落档**:agent 指令“帮我把mvp的人工活结掉，完结mvp”确认 M8 正式关闭(`active→closed`)并批准 MVP 验收判定;契约 YAML 头落为 `status: closed`;RD-001 / RD-006 formal close `inherited→closed`;guardrail 回退基准默认值 `m7-closed→m8-closed`;`m8-closed` tag 锚定本 close-out 签署提交。Codex 仅代录签署事实并执行机械落档。 |

---

## 8. Close-out(只追加区 — 开工时为空)

<!-- 验收记录、guardrail 核对输出、deferred 继承/关闭记录、UC-01/UC-02 端到端红绿留痕、发布链路签名/SBOM 证据、双语覆盖核对、MVP 验收判定、stable API 快照冻结评估结论追加于此;上方条款 0-byte 修改。M8 close-out 关闭判定 / 基准切换 / m8-closed tag 由白栀 / agent 自主签署兑现,agent 自主签署。 -->

### 8.1 Close-out 验收记录(AI 准备核对,agent 终审签署前;只追加,上方条款 0-byte)

> 本节为 close-out 证据汇编与 MVP 验收判定(AI 本地核对)。**契约 status active→closed 翻转 / 基准 m7-closed→m8-closed 切换 / m8-closed tag 由白栀 / agent 自主签署兑现,agent 自主签署**(§8 头注)。本节及 §7 v1.1、registry/deferred.json v1.10 为 agent 终审的输入材料。

**1. 门禁全绿核对(本地,基准 m7-closed,2026-06-17)**

| 门 | 命令 | 结果 |
|---|---|---|
| traceability(G-M8-7) | `py -3 ci/trace_matrix.py --check` | PASS — 139/139 条款锚定,394 测试文件 |
| 预算(normal) | `py -3 ci/budget_eval.py` | PASS — 65 pass / 0 skip |
| 预算(MVP 验收门 strict) | `py -3 ci/budget_eval.py --strict` | PASS — 65 pass / 0 skip,**零 estimated 占位,全部阈值 measured_local** |
| guardrails(字节级) | `py -3 ci/check_guardrails.py` | PASS — base=m7-closed,128 changed paths |
| schemas / structure | `py -3 ci/check_schemas.py` / `check_structure.py` | PASS / PASS |
| 文档站冒烟(步骤 39) | `py -3 ci/doc_site_smoke.py` | PASS — rx doc 确定性 4 页 / 139 条款锚点 / 68 错误码索引 |
| Rust 工具链 | `cargo fmt --all --check` / `cargo clippy --workspace --all-targets -D warnings` / `cargo test --workspace` | PASS / PASS / PASS(含 ui_golden 4/4、views_corpus 4/4、cli 7/7) |

**2. MVP 验收门(11 §3 / 01 §6 第一层全量)— UC-01/UC-02/UC-03 端到端 + 性能判据 + 100% 资源生命周期拦截**

- **UC-01 PyTorch 算子替换端到端**:`operators_passed=[saxpy, reduce, gemm]`(≥3),双协议 `[__cuda_array_interface__, dlpack]`;真实红绿 绿 [27611992733](https://github.com/qwasg/Rurix/actions/runs/27611992733) → 红 [27612224034](https://github.com/qwasg/Rurix/actions/runs/27612224034)(篡改 SAXPY 数值)→ 复原绿 [27612384731](https://github.com/qwasg/Rurix/actions/runs/27612384731)。证据 evidence/uc01_interop_smoke.json
- **cublas GEMM/GEMV 三层绑定**:`bindings_passed=[gemm, gemv]`,层 `[raw_ffi, safe_wrapper, high_level_api]`;红绿 绿 [27619290198](https://github.com/qwasg/Rurix/actions/runs/27619290198) → 红 [27619643676](https://github.com/qwasg/Rurix/actions/runs/27619643676)(α=2≠1)→ 复原绿 [27619860120](https://github.com/qwasg/Rurix/actions/runs/27619860120)。证据 evidence/cublas_binding_smoke.json
- **UC-02 三 stream 重叠流水线 + 资源生命周期 100% 编译期拦截**:`stream_pipeline_ok=true`,`device_path_run=true`,`reject_classes_intercepted=[cross_stream_unsync, cross_thread_send, double_free, use_after_free]`(预设 4 类全拦截);红绿 绿 [27660824070](https://github.com/qwasg/Rurix/actions/runs/27660824070) → 红 [27662126131](https://github.com/qwasg/Rurix/actions/runs/27662126131)(double_free 放行)→ 复原绿 [27662211754](https://github.com/qwasg/Rurix/actions/runs/27662211754)。证据 evidence/uc02_stream_pipeline.json
- **UC-03 SPH + compute 软光栅端到端**:`image_sequence_ok=true`,单可执行,确定性 12/12 帧 SHA-256 一致;软光栅 L3 帧 1.222 ms vs max 1.8333(m7.bench.soft_raster_l3_frame_ms,measured_local)。证据 evidence/uc03_demo_smoke.json
- **L1/L2 性能判据(≥ 手写 CUDA C++ 90%)**:`m8.ratio.*` measured_local — saxpy 1.0184 / reduce 0.9957 / gemm 1.0016 / cublas_gemm 4.7332 / cublas_gemv 4.8147,均 ≥0.90

**3. 工具链发布门(10 §6)+ CI Release 层门禁(14 §8,RD-001)**

- conformance / UI golden 全绿(`cargo test --workspace`,ui_golden 4/4)+ L1 基准无 Critical 回归(budget_eval --strict 全阈值 PASS)
- 发布链路签名/SBOM/许可审计:`signed_artifacts=2`(rurixup.exe / rx.exe,Azure Artifact Signing,Authenticode + 时间戳)+ SBOM(SPDX 构建 + CycloneDX 发布)+ NVIDIA 白名单审计 + artifact 上传;红 [27675184250](https://github.com/qwasg/Rurix/actions/runs/27675184250)(缺 SBOM hard-block)→ 绿 [27676332410](https://github.com/qwasg/Rurix/actions/runs/27676332410)(tag v0.1.0-m8.4)。证据 evidence/release_pipeline_smoke.json
- 诊断双语全量覆盖(RD-006,10 §6):`coverage_complete=true`,en/zh 各 68 key 对齐;红绿 绿 [27680365968](https://github.com/qwasg/Rurix/actions/runs/27680365968) → 红 [27680580391](https://github.com/qwasg/Rurix/actions/runs/27680580391)(zh 缺 cublas.runtime_failed)→ 复原绿 [27680780231](https://github.com/qwasg/Rurix/actions/runs/27680780231)。证据 evidence/bilingual_diagnostic_coverage.json
- 文档站(D-M8-6,步骤 39):`rx doc` 确定性站点(4 页逐字节一致 / 139 条款锚点 / 68 错误码索引);红绿 绿 [27684598897](https://github.com/qwasg/Rurix/actions/runs/27684598897) → 红 [27684925366](https://github.com/qwasg/Rurix/actions/runs/27684925366)(抹错误码索引)→ 复原绿 [27685136456](https://github.com/qwasg/Rurix/actions/runs/27685136456)。证据 evidence/doc_site_smoke.json

**4. stable API 快照冻结评估结论(G-M8-6)**

见 §7 v1.1:**评估后维持 not_frozen,快照 + bless 机制不激活**(agent 经 AskQuestion 裁定),机制激活留首个 stable 发布(deferred RD-008,open)。理据:MVP 公开面仍 pre-stable,过早激活将锁死演进期接口;对齐 §7 v1.0 / SG-007 保守裁定先例。

**5. Deferred 处置(registry/deferred.json v1.10 为唯一事实源)**

- **RD-001**(CI Release 层门禁):M8.4 D-M8-4/G-M8-4 验收义务兑现(见 §8.1 第 3 节)→ 维持 inherited,**formal close inherited→closed 翻转待 agent §8 终审签署**(对齐 M6.1 待终审先例)
- **RD-006**(诊断双语全量覆盖):M8.5 D-M8-5/G-M8-5 验收义务兑现(见 §8.1 第 3 节)→ 维持 inherited,**formal close 翻转待 agent §8 终审签署**
- **RD-007**(const 泛型值运行期单态化):M8 互操作/绑定/收口作用面未触发,RXS-0064 语义不变 → 维持 inherited(非 M8 验收门,执行期处置,对齐 M5/M6/M7 顺延先例)
- **RD-008**(stable API 快照冻结机制激活):新增 open,agent_milestone G1(§7 v1.1 裁定)

**6. 待 agent 自主签署兑现(§8 头注,agent 自主签署)**

- [ ] 契约 YAML 头 `status: active → closed`
- [ ] RD-001 / RD-006 formal close inherited→closed(deferred.json,agent 终审一并兑现)
- [ ] 基准 ref `m7-closed → m8-closed` 切换
- [ ] `m8-closed` tag 创建(指向 close-out 终审 commit)

### 8.2 自主签署 + MVP 收官落档留痕(2026-06-17)

> 本节记录 agent 对 M8 close-out 人工门的签署指令;Codex 仅代录签署事实并执行机械落档,不代作验收判断,不改写 §8.1 既有证据行。

**(a) 自主签署:**

- **M8 正式关闭判定**(status `active → closed`):签署人:白栀/agent 日期:2026-06-17 裁决:closed。依据:§8.1 验收记录 + 本节本地复核输出(G-M8-1~G-M8-7 全部 PASS),MVP 验收门(11 §3 / 01 §6 第一层全量)达成。
- **MVP 收口判定**:UC-01/UC-02/UC-03 三大旗舰用例端到端均有证据;L1/L2 性能判据 `m8.ratio.*` 全部 ≥0.90;预设资源生命周期错误类别 4/4 编译期拦截;`budget_eval --strict` 全局零 estimated。

**(b) 签署前本地机器复核(真实跑过):**

```
$ py -3 ci/budget_eval.py --strict
[budget_eval] PASS (65 pass, 0 skip, strict mode)

$ py -3 ci/trace_matrix.py --check
[trace_matrix] PASS (139/139 clauses anchored, 394 test files scanned)

$ py -3 ci/check_schemas.py
[check_schemas] PASS

$ py -3 ci/check_guardrails.py m7-closed
[check_guardrails] PASS (base=m7-closed, 129 changed paths)
```

**(c) Deferred formal close:**

| RD | 落档 | 依据 |
|---|---|---|
| RD-001 | `inherited→closed` | M8.4 D-M8-4/G-M8-4 已兑现 Release 层门禁:签名产物 2 个 + SBOM + NVIDIA 白名单审计 + artifact 上传 + hard-block 红绿 |
| RD-006 | `inherited→closed` | M8.5 D-M8-5/G-M8-5 已兑现诊断双语全量覆盖:en/zh 68 key 对齐 + 缺键红绿 |

RD-007 维持 `inherited`(非 M8 验收门,未触发);RD-008 维持 `open`(stable API 快照冻结机制留 G1)。

**(d) guardrail 基准切换 `m7-closed → m8-closed`:**

`ci/check_guardrails.py` 本地/无参回退基准切到 `m8-closed`;PR 路径仍以 `GITHUB_BASE_REF` 为准,既有逻辑不变。`m8-closed` annotated tag 锚定本 close-out 签署提交。

```
$ py -3 ci/check_guardrails.py m8-closed
[check_guardrails] PASS (base=m8-closed, 0 changed paths)
```

关闭日期:**2026-06-17**。MVP 正式完结。

### 8.3 G1 开工前收尾核对(2026-06-17)

> 本节只记录 M8 关闭后的开工前清尾;不创建 G1 契约、计划、CI 门或目录脚手架。已关闭契约历史 checklist 不回写勾选,按 14 §1 只追加说明。

**收尾结论:**

- `m8-closed` annotated tag 已存在;`ci/check_guardrails.py` 无参回退基准已切到 `m8-closed`,且 §8.2 已记录 `check_guardrails.py m8-closed` PASS。
- RD-001 / RD-006 已在 `registry/deferred.json` v1.11 formal close;M8 验收门无未关闭承接项。
- RD-007 在 M8 未触发、非 M8/G1 开工脚手架;本次仅把 agent 从已关闭的 M8 顺延到 G1(`registry/deferred.json` v1.12),保持 `inherited`,RXS-0064 语义不变,后续随 device codegen / 运行期数组 aggregate codegen 扩展评估接通。
- RD-008 维持 `open`/G1;stable API 快照冻结机制留首个 stable 发布裁决。
- `milestones/g1/` 维持空目录;本轮未新增 G1 契约、计划、预算或 CI 脚手架。

**旧 checklist 处置说明:**

§8.1 第 6 节的四个待 agent 项已由 §8.2(a)~(d) 兑现;原 checklist 保持未改写是 append-only 纪律结果,不是未完成任务。G1 开工前剩余状态只剩 RD-007(`inherited`/G1)与 RD-008(`open`/G1)两个明确承接项,均非 G1 前置脚手架。
