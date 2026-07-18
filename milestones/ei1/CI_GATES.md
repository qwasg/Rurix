# EI1 CI 门禁增量(gated)

> 所属契约:[EI1_CONTRACT.md](EI1_CONTRACT.md)(**gated**:激活 gated on G3 close-out + owner 立项确认,契约 §0/G-EI1-0)
> 版本:v1.0(2026-07-18)
> 基线:[../m0/CI_GATES.md](../m0/CI_GATES.md) ~ [../ea1/CI_GATES.md](../ea1/CI_GATES.md) + G3 期届时增量(全部沿用);本文只规定 EI1 期的**增量**。
> 铁律不变:任何新增门禁必须在真实 PR 上以真实失败/通过路径验证过(反 YAML-only)。
> **gated 脚手架口径**:本文 EI1 增量步骤(71~75 earmark)为**激活后 EI1.2~EI1.5 计划项**,gated 期与开工均**不**写入 workflow YAML 真实步骤(随激活后实现 PR 落地回填,对齐 M8~EA1 计划→回填范式)。**EI1 gated 包零 CI 代码改动**:预算 glob 已泛化为 `*_budget.json` 自动纳入 `ei1_budget.json`(空态);counter/entries 不预造。

---

## 1. Runner

沿用 M0 §1(自托管 RTX 4070 Ti 开发机)~ EA1 §1 + G3 期届时增量。EI1 新增 runner 预置项:**无**——步骤 71/72/74 的 device 段用既有 CUDA 链(rxrt_* PTX,RURIX_REQUIRE_REAL);步骤 71/74 的 C 宿主编译用既有 MSVC(cl.exe/link.exe,步骤 43 engine_integration_smoke 同源);步骤 73/75 纯 host 面。零网络外呼。

## 2. PR Smoke 追加步骤(激活后计划项;**步骤 71~75 = EI1 earmark**(owner 2026-07-18 双轨分配,G3_CONTRACT §7 v1.1 固化:步骤 61~70 = G3,71~75 = EI1,gated 期零消费)。激活时经 G-EI1-0 以届时 workflow 末号与台账实际为准复核兑现;若 G3 步骤面溢出占用(不应发生),以届时现状续号并留痕。落地随激活后 EI1.2~EI1.5 实现 PR 回填 workflow)

| # | 步骤 | 失败即红 |
|---|---|---|
| 71 | export(c) 接通冒烟(契约 G-EI1-2 通道;EI1.2 落地接入,**RFC-0014 前置后**):`ci/export_c_smoke.py` —— .rx fixture → `--emit=dll` 产 DLL + import lib + 生成头 → cl.exe 编译链接 C 调用方 → device 真跑数值对照(RURIX_REQUIRE_REAL);头生成幂等(同源两次逐字节一致);RED 三路各自独立见证:非 C 兼容签名 → 编译期拒 / 篡改入库头一字节 → 再生成 byte-diff 红 / 导出名冲突 → 编译期拒;内建 red_self_test;产物落 %TEMP% 不留仓库;写 evidence json(schema 校验) | 是 |
| 72 | UC-05 RHI demo 冒烟(契约 G-EI1-3 通道前半;EI1.3 落地接入):`ci/uc05_rhi_smoke.py` —— apps/uc05-rhi in-EXE demo:graph ≥3 pass device 真跑数值对照 + 同机两跑逐字节确定;零 .rs 审计(包内 *.rs 计数 = 0);RED:桩化拦截/执行逻辑 → 数值对照红 | 是 |
| 73 | UC-05 不变量拦截门(契约 G-EI1-3 通道后半;EI1.3 落地接入):`ci/uc05_invariant_gate.py` —— conformance/uc05/reject 矩阵逐条编译期/构建期断言期望诊断(I1~I8),漏拦即红;内建 red_self_test | 是 |
| 74 | UC-05 引擎嵌入冒烟(契约 G-EI1-4 通道;EI1.4 落地接入):`ci/uc05_engine_embed_smoke.py` —— rurix_rhi.dll(export(c) 产)+ 生成头(再生成逐字节比对)→ engine_host v2(C++/D3D12,LUID 匹配)编译链接 → ≥1 graph compute pass device 真跑数值对照(步骤 43 结构先例;RURIX_REQUIRE_REAL);G1.3 既有资产 0-byte 核 | 是 |
| 75 | UC-05 对照报告一致性核(契约 G-EI1-5 通道;EI1.5 落地接入):`ci/uc05_report_check.py` —— uc05_invariant_matrix.json schema 校验 + 矩阵↔reject 语料↔uc05_comparison_report.md 三方一致性互查(check_* 守卫风格,不写 budget counter);documented_historical 分级字面核(Python 侧条目必须带引文 文件:行号) | 是 |

各行为激活后拟分配,门内容以契约 §4 与实现 PR 实测为准;修订走本文件 §7,步骤号一旦占用不复用;若最终步骤数少于 5,多余号作废声明留痕不回收(burned 机制,MR-0006/0007 先例)。

预算 evaluator 自动合并加载 [ei1_budget.json](ei1_budget.json)(命名空间冲突即红;**gated 期恒空,counter 登记与 evaluator 分支随激活后实现 PR 同落,ei1.bench.* 随取证 measured_local 回填**)。**EI1 close-out 必须跑 `--strict` 且全局零 estimated 残留**(14 §3)。

## 3. Release 层门禁

既有门禁(RXS-0139 八子门 + channel-manifest + EA1.2 上传/回读自校验延伸)**0-byte 沿用**;EI1 零 Release 层增量(rurix_rhi.dll 为 UC-05 验收工程物,非发布资产——进发布面需另期另裁,防范围蔓延)。`ei1-closed` tag 不匹配触发器 `v[0-9]+.[0-9]+.[0-9]+*`,零误触发。

## 4. Nightly 追加

既有 nightly 全保留。**EI1 无新增 nightly 项**:export(c)/RHI/嵌入冒烟归 PR smoke 步骤 71~75(秒~分级);增量 check bench 为一次性 evidence 非趋势项。

## 5. Guardrail

沿用 M0~EA1 全部激活项 + G3 期届时增量。EI1 期动作:

1. **gated 期总红线**:本包仅 milestones/ei1/ 四件,registry/rfcs/spec/ci/.github 全 0-byte,共享编号零消费;激活前唯一合法后续改动 = G-EI1-0 激活小 PR 与契约 §7 追加行。
2. **基准 ref**:以合入时 main 现状为准(现 `mb1-closed`;PR 路径以 `GITHUB_BASE_REF` 为准);gated 期与激活时均不切基准;EI1.5 close-out 时按 main 合并序串行化切换(agent 自主签署)。
3. **编号双轨纪律**:RFC-0013 / RXS-0220~0249 / 步骤 61~70 = G3;RFC-0014 / RXS-0250~0269 / 步骤 71~75 = EI1 earmark(gated 期零消费,激活时复核兑现);RD-/U-/RX- 按 main 合并序取号不预留;earmark 不入 number_ledger `shadow_reserved`(off-tree burned 专用)。
4. **(激活后)stable 快照 bless**:EI1 预期 2~3 次加性重 bless(RXS-0250 earmark 段条款增长,RXS-0180 L2);各与条款/实现同 PR 重 bless + bless_log 同 diff 追加(数据行忌「日期」子串);不可分 PR(步骤 49 硬红)。
5. **(激活后)错误码**:拟新 RX 码 ≤4(属性误用/签名不兼容/空导出集/DLL 链接失败),按合并时点 main 段位续号(以 number_ledger 现状为准),en/zh 成对;graph 构建期错误走库层状态值零新码(spec/imageio.md 先例)。
6. **(激活后)C ABI 双制共存**:src/rurix-interop RXS-0125 手写 `extern "C"` 语义 0-byte 只增;RXS-0149 守卫(步骤 43)export(c) 落地前维持全绿,落地时共存判据升级与条款同 PR;src/rurix-engine G1.3 三符号面/手写头 0-byte。
7. **(激活后)主语言判据**:apps/uc05-rhi 零 .rs(步骤 72 审计);语言硬缺口登记 RD 按 10 §3 判档,不静默降级 .rs。
8. **spec 修订行纪律**:(激活后)spec/export_c.md / spec/rhi.md 修订表表头「版本」,数据行避「版本」子串(用「版号」)。
9. **规划文档冻结**:00–14 执行 PR 0-byte;裁决留痕只进 EI1_CONTRACT §7 + RFC-0014 §9;13_DECISION_LOG / spike_gating 全期 pristine。
10. **LF byte-exact**:新文件 LF+尾换行;禁 Python 文本模式写文件;提交前逐文件字节核 CR+尾字节。
11. **(激活后)RURIX_REQUIRE_REAL 纪律**:步骤 71/72/74 device 段真跑,mock/dry-run/SKIP 不充绿(防降级硬门,EA1 G-EA1-2 先例)。

14 §2 常驻集其余项的 EI1 期评估结论(激活时复评):

| 项 | 结论 |
|---|---|
| stable API 快照 | 已激活;EI1 激活后预期 2~3 次加性重 bless(RXS-0250+ earmark 段条款增长,RXS-0180 L2) |
| MIR/PTX/DXIL/UI golden | 已激活;EI1 预期 UI golden 增长(export(c) reject 语料)+ MIR/PTX 零破坏性变更(导出面为加性通道) |
| unsafe-audit 完整性 | 已激活;EI1 拟零新 unsafe(RHI 全 .rx + codegen 走既有 LLVM 文本 IR 通道);触发则按合并时点台账取号登记,不预占 |
| Compute Sanitizer / NVIDIA 白名单 | 已激活维持;UC-05 kernel 简单核,白名单审计 0-byte 沿用 |
| 多后端(D-008/SG-003) | SG-003 维持 triggered(RFC-0011)不回翻;rhi_on_vulkan 显式 out_of_scope(RD-031 open;激活时复评 G3 vk descriptor 底座影响);G-MB1-6 AMD 尾门独立于 EI1 |
| registry sumdb(D-312/SG-007)| 维持 not_triggered(EI1 零网络面) |
| MLIR(SG-001)/ Tensor Core(SG-002)/ autodiff·fusion(SG-004/005)/ Python 嵌入(SG-008)/ 自举(SG-009) | 维持 not_triggered;SG-010 软保留维持 |
| 贡献校验门(ci/check_contribution.py) | 已激活延续 |

## 6. 验证程序(对应契约 G-EI1-0~G-EI1-6 与计划步骤 71~75)

1. gated 期:本包合入 = G3 侧 G-G3-8 门核验对象(四件套就位 + §0 gated 措辞 + 零实现零共享编号消费);host 门全绿(guardrails / schemas / budget 空态 / structure / number_ledger / trace 不变)。
2. G-EI1-0 激活:G3 close-out + owner 立项确认 → 激活小 PR 核验(status 翻转 + RD-009 承接 + ledger 登记 + earmark 复核留痕)。
3. (激活后)EI1.1:RFC-0014 Approved 合入序核验(先于任何实现 commit;失败测试先行——步骤 71~75 脚本与 export(c) codegen/`--emit=dll`/RHI 代码在 RFC 合入时点 main 上不存在)。
4. (激活后)EI1.2 步骤 71:本机 `py -3 ci/export_c_smoke.py`(dll+头+C 宿主真跑+三路 RED);runner PR run URL 归档 §8;trace / stable_snapshot(重 bless 后)/ bilingual / guardrails 全绿。
5. (激活后)EI1.3 步骤 72/73 与 EI1.4 步骤 74:demo 确定性两跑 + reject 矩阵逐条断言 + engine_host v2 编译链接 device 真跑,本机与 runner 双真跑,run URL 归档 §8。
6. (激活后)EI1.5 步骤 75 + evidence:schema + 三方一致性互查输出;ei1.bench.* 回填后 `py -3 ci/budget_eval.py`;close-out `--strict` 零 estimated + G-EI1-0~6 留痕指针 + RD-009 处置 + SG 复评 + 双基准 advisory 复核输出。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-07-18 | 初版(EI1 gated 契约配套;步骤 71~75 = EI1 earmark 计划项(G3_CONTRACT §7 v1.1 固化,61~70 = G3),激活后随 EI1.2~EI1.5 实现 PR 回填 workflow YAML 实测命令与 run URL;Release/nightly 零增量;guardrail 动作:gated 期总红线(四件 only + 零共享消费)、基准以合入时 main 现状为准、激活后快照重 bless/新 RX 码 ≤4/C ABI 双制共存/主语言判据零 .rs/RURIX_REQUIRE_REAL 纪律)。**EI1 gated 包零 CI 代码改动**:ei1_budget.json 经 *_budget.json glob 自动纳入(空态),counter/entries 不预造;不写入 workflow YAML 真实步骤 |
