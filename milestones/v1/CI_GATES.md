# V1 CI 门禁增量

> 所属契约:[V1_CONTRACT.md](V1_CONTRACT.md)
> 版本:v1.0（2026-07-14）
> 基线:[../m0/CI_GATES.md](../m0/CI_GATES.md) ~ [../g2/CI_GATES.md](../g2/CI_GATES.md)（全部沿用:runner 约定、PR Smoke 1–49 步、Release 层门禁(14 §8,M8.4 RD-001 建成 + G1.5 fatbin + G2 DXIL 覆盖)、guardrail 全部激活项(含 stable 快照 bless check_stable_snapshot_bless,G2.5 RD-008 激活)、nightly 全量回归冻结）;本文只规定 V1 期的**增量**。
> 铁律不变:任何新增门禁必须在真实 PR 上以真实失败/通过路径验证过(反 YAML-only)。
> 开工脚手架口径:本文 V1 增量步骤(50)为 **V1.2 计划项**,开工**不**写入 workflow YAML 真实步骤(随 V1.2 实现 PR 落地回填,对齐 M8/G1/G2 计划 → 回填范式)。**V1 开工脚手架零 CI 代码改动**:预算 glob 已泛化为 `*_budget.json`,自动纳入 `v1_budget.json`;`check_closed_contracts` 的 `*_CONTRACT.md` glob 与无参默认基准 `g2-closed` 均已就位;**`v1_budget.json` entries/ratio_assertions/counter_assertions 全期留空(验收走既有全局 `m1.counter.spec_clause_test_anchoring` + `m8.counter.release_artifacts_signed` 断言与 evidence/ 归档,不预造 v1 counter,无 `ci/budget_eval.py` 新分支)**。

---

## 1. Runner

沿用 M0 §1(自托管 RTX 4070 Ti 开发机 `rurix-dev-4070ti`)~ G2 §1(原生 D3D12 + DXIL 工具链 + 全量 conformance + stable 快照)。V1 新增 runner 预置项:**无**(channel 清单为纯 host 确定性 JSON,CPU-only;发行走既有 release.yml 环境:MSVC + signtool 测试证书 + CUDA + 签名 DXC)。gh CLI(GitHub Release 创建)走本机人工链路,不进 workflow。

## 2. PR Smoke 追加步骤（计划项,编号接 G2 §2 的 45–49;落地随 V1.2 实现 PR 回填 workflow）

| # | 步骤 | 失败即红 |
|---|---|---|
| 50 | stable channel 清单冒烟(契约 G-V1-3 通道;V1.2 落地接入,**Mini-RFC MR-0008 前置后**):`ci/channel_manifest_smoke.py` —— green(rurixup release 产出 channel_manifest.json:channel=stable / rurix_version=workspace 版号 / bundle 清单 digest 引用 / 组件一致;同一输入两次生成逐字节一致 = 确定性)+ red→绿闭合(漂移注入 `--simulate-channel-drift` → 发布门红 exit 2 且 failed_gates 含 channel-manifest;未知 channel `--channel nightly` → 用法错误 exit 1;复原绿,反 YAML-only)。**纯 host/CPU-only,不 SKIP 充绿(除无 cargo 工具链)**。不写 evidence(无新 budget counter)。建设期未落地 → 步骤不存在(随实现回填) | 是 |

预算 evaluator 自动合并加载 [v1_budget.json](v1_budget.json)(命名空间冲突即红;**开工 entries/ratio_assertions/counter_assertions 均留空且全期预期不回填**——V1 验收走既有全局断言 + evidence/ 归档 + 契约 §8 留痕,不预造 v1 counter,无新 evaluator 分支)。**V1 close-out 必须跑 `--strict` 且全局零 estimated 残留**(延续零占位纪律,14 §3)。

## 3. Release 层门禁（14 §8;M8.4 建成 + G1.5 fatbin + G2 DXIL 覆盖;V1 延伸 channel 清单 + 版号跳变 + 触发器收窄）

- **channel 清单纳入 Release 层**(G-V1-3,随 V1.2 回填):`rurixup release` 追加产出 `channel_manifest.json`,其一致性为 Release 层 hard-block 门集第 8 子门 `channel-manifest`(既有 7 门相对顺序 0-byte;任一红 → `allow_upload=false`,退出码 2);release.yml 步骤与 upload-artifact path 同步追加。
- **版号跳变**(G-V1-4,随 V1.3 回填):workspace 1.0.0;`ci/release_pipeline_smoke.py` 版号注入点参数化 `workspace_version()`(bundle `rurix_version` 与 tag/workspace 三点一致,RXS-0135 同版号判据)。
- **tag 触发器收窄**(G-V1-4,随 V1.3 回填):`release.yml` `tags: "v*"` → `"v[0-9]+.[0-9]+.[0-9]+*"`——防 `v1-closed` milestone tag 误触发 release workflow(历史 m*/g*-closed tag 均无 v 前缀,v1-closed 首次撞上;**必须先于 v1-closed tag 生效**,契约 §7 ⑪)。
- **GitHub Release 创建不进 workflow**:`.github/` 现状不授任何 workflow 写权限(无 permissions/GITHUB_TOKEN 用例),维持;`gh release create v1.0.0 --verify-tag` 由本机人工链路在 release.yml 全绿后执行(人工执行即发布动作的人工门,契约 §7 ⑤)。
- **激活经真实红绿验证**(反 YAML-only):漂移注入 → Release 门红 → 复原转绿,run URL 归档(随 V1.2/V1.3 回填)。

## 4. Nightly 追加

- 既有 nightly 全保留(Compute Sanitizer racecheck+memcheck + measured 基准 + 全量回归冻结)。
- **V1 无新增 nightly 项**:channel 清单冒烟为纯 host 秒级,归 PR smoke 步骤 50;发行为一次性动作非趋势项。

## 5. Guardrail

沿用 M0~G2 全部激活项。V1 期动作:

1. **基准 ref 默认 `g2-closed`**:G2 close-out 已切,**V1 开工无需再切**;PR 路径以 `GITHUB_BASE_REF` 为准。V1.4 close-out 时切至 `v1-closed`(agent 自主签署;glob 已泛化,仅切 `resolve_base` 默认值 + 打 tag,代码改动最小)。
2. **stable 快照 bless(check_stable_snapshot_bless,已激活)**:V1 期唯一预期触发 = V1.2 条款增长(RXS-0185 续号)→ `spec_clauses` 漂移 → 同 PR 重 bless + `tests/stable/bless_log.md` 同 diff 追加审批行;与条款/实现不可分 PR(步骤 49 硬红)。
3. **错误码零新增**:channel 清单失败以工具层退出码 + `failed_gates` 枚举表达(spec/release.md §3 口径);RX7021 续号仅备用,V1 预期不动 `registry/error_codes.json`。
4. **零新 unsafe**:channel 模块为纯 host 确定性 JSON,全仓 `unsafe_code=deny` 维持;无 U 续号预期。
5. **spec 修订行纪律**:spec/release.md 修订表**表头**维持「版本」列名;**数据行**避「版本」子串(用「版号」),否则被 `spec_revision_rows` 按表头跳过致追加核对失真;`tests/stable/bless_log.md` 数据行忌「日期」子串(同理)。
6. **规划文档冻结**:00–14(含 13)执行 PR 0-byte;开工裁决记 V1_CONTRACT §7;11_ROADMAP 发行标注走 00 §6.3 独立勘误 PR(V1.3 发布后)。
7. **LF byte-exact**:新文件 LF+尾换行;registry/*.json 等既有 CRLF 例外文件追加行保持原行尾风格、既有行 0-byte。

14 §2 常驻集其余项的 V1 期评估结论:

| 项 | 结论 |
|---|---|
| stable API 快照 | **已激活**(G2.5/RD-008 closed);V1.2 条款增长触发一次重 bless(加性演进,RXS-0180 L2);1.0 发布后同 edition 内 stable 面只增不破坏 |
| MIR/PTX/DXIL/UI golden | 已激活;V1 预期零 golden 变更 |
| unsafe-audit 完整性 | 已激活;V1 零新 unsafe 预期 |
| Compute Sanitizer / NVIDIA 白名单 | 已激活维持;发行 run 内 `check_redistribution` 照跑 |
| 多后端(D-008,SG-003) | **维持 not_triggered**(1.0 发布 ≠ NVIDIA 纵深完成,红线 3 不解除) |
| registry sumdb(D-312,SG-007) | **维持 not_triggered**(社区规模未达触发阈) |
| MLIR(SG-001)/ Tensor Core(SG-002)/ Python 嵌入(SG-008) | 维持 not_triggered |
| 贡献校验门(ci/check_contribution.py) | 已激活延续(provenance / 条款号 / 验证标记三类缺项即红) |

## 6. 验证程序（对应契约 G-V1-1~G-V1-5 与计划步骤 50）

1. V1.1:report 数字逐项与 `py -3 ci/stable_snapshot.py --check` 本机真实输出比对一致;evidence/v1.1-stabilization/ 归档命令原文;FCP-lite issue URL 真实可访问。
2. 步骤 50 落地后(**MR-0008 前置**),构造 channel 清单漂移(`--simulate-channel-drift`)→ 发布门红 exit 2(failed_gates 含 channel-manifest);未知 channel → exit 1;复原 → 绿,run URL 归档(反 YAML-only)。**纯 host/CPU-only 可完整演示**。
3. V1.2 快照重 bless:改 spec 后 `stable_snapshot --check` 红 → `RURIX_BLESS=1` 重生成 + bless_log 同 diff 追加 → `--check` 复绿 → `ci/edition_smoke.py`(步骤 49 内含篡改红绿闭合)复绿。
4. V1.3 发行:tag v1.0.0 → release.yml 全量 success(run URL 归档);run 日志核验 bundle `rurix_version=1.0.0`;`gh release create --verify-tag` 后 Release URL 回填;版号三点一致(tag/workspace/bundle)。
5. close-out 附 `budget_eval --strict` 输出原文(全局零 estimated)+ G-V1-1~5 留痕指针 + RD-007/RD-009 处置 + SG 复评结论 + 双基准(g2-closed / v1-closed)advisory 复核输出。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-07-14 | 初版(V1 契约配套;计划步骤 50 为 V1.2 计划项,落地时回填 workflow YAML 实测命令与 run URL;Release 层延伸 = channel 清单第 8 子门 + 版号跳变参数化 + tag 触发器收窄(均随 V1.2/V1.3 回填);guardrail 动作:基准 g2-closed 无需再切、stable 快照 V1.2 预期一次重 bless、错误码/unsafe 零新增、spec 修订行「版号」纪律、close-out 切 v1-closed;SG-001/002/003/007/008 开工复评维持 not_triggered)。**V1 开工脚手架零 CI 代码改动**:`v1_budget.json` 经 `*_budget.json` glob 自动纳入,entries/ratio_assertions/counter_assertions 全期留空(验收走既有全局断言 + evidence/ 归档,不预造 v1 counter,无 `ci/budget_eval.py` 新分支);`*_CONTRACT.md` glob 与无参基准 g2-closed 均已就位。开工不写入 workflow YAML 真实步骤(随 V1.2 实现 PR 回填) |
