# EA1 CI 门禁增量

> 所属契约:[EA1_CONTRACT.md](EA1_CONTRACT.md)
> 版本:v1.0(2026-07-16)
> 基线:[../m0/CI_GATES.md](../m0/CI_GATES.md) ~ [../mb1/CI_GATES.md](../mb1/CI_GATES.md)(全部沿用:runner 约定、PR Smoke 1–58 步、Release 层门禁(RXS-0139 八子门含 channel-manifest)、guardrail 全部激活项(含 stable 快照 bless)、nightly 全量回归冻结);本文只规定 EA1 期的**增量**。
> 铁律不变:任何新增门禁必须在真实 PR 上以真实失败/通过路径验证过(反 YAML-only)。
> 开工脚手架口径:本文 EA1 增量步骤(59/60)为 **EA1.1/EA1.2 计划项**,开工**不**写入 workflow YAML 真实步骤(随实现 PR 落地回填,对齐 M8~MB1 计划→回填范式)。**EA1 开工脚手架零 CI 代码改动**:预算 glob 已泛化为 `*_budget.json` 自动纳入 `ea1_budget.json`;`check_closed_contracts` glob 与无参默认基准 `mb1-closed` 均已就位;**counter/entries 不预造**(登记与 `ci/budget_eval.py` evaluator 分支同实现 PR 落,未知 id 强制 FAIL)。

---

## 1. Runner

沿用 M0 §1(自托管 RTX 4070 Ti 开发机)~ MB1 §1。EA1 新增 runner 预置项:**无**——步骤 59 为纯 host/hermetic 面(离线 `--from-dir` 源 + 本地环回 HTTP fixture,零真实外呼、零 GPU 依赖);步骤 60 为离线打包确定性面。**真实网络端点只出现在 release.yml**(上传后回读自校验 + workflow_dispatch 演练)与 e2e 冷启动取证(evidence 面,本机/VM 人工链路,不进 CI)。

## 2. PR Smoke 追加步骤(计划项,编号接 MB1 §2 的 58;落地随 EA1.1/EA1.2 实现 PR 回填 workflow)

| # | 步骤 | 失败即红 |
|---|---|---|
| 59 | rurixup 真实分发冒烟(契约 G-EA1-2/G-EA1-3 通道;EA1.1a 落前半、EA1.1b 落后半,**RFC-0012 前置后**):`ci/rurixup_dist_smoke.py` —— **前半(纯离线,`--from-dir` 本地源)**:install 真实物化到临时 `RURIX_HOME`(staging→全量校验→rename)→ toolchains 目录内 exe 真跑探针 → 切换后版本探针指到目标版本(机制按裁决 B);RED:篡改组件一字节 → 内容寻址拒且 toolchains/ 零残留、注册表 0-byte;切换指向已删目录 → 诚实报错退出非 0;复原 → 绿。**后半(hermetic 环回 HTTP,Python http.server 本地 fixture,`RURIXUP_TEST_ALLOW_LOOPBACK_HTTP=1`)**:完好资产全链 install 绿;RED 四路各自独立见证:坏字节 / 坏哈希(锚级失配)/ 截断 / 默认态(无测试 env)非 https 被拒;离线(fixture 关闭)→ 诚实错误退出非 0 + 系统 0-byte。**零真实外呼**;内建 red_self_test;EXE/物化产物落 %TEMP%,不留仓库;写 `evidence/rurixup_dist_smoke.json`(schema 校验) | 是 |
| 60 | 发布 bundle 打包冒烟(契约 G-EA1-4 通道;EA1.2 落地接入):`ci/release_bundle_smoke.py` —— 3 组件编排(rx.exe/rurixup.exe/rurix_rt_cabi.lib,缺件即红)+ SHA256SUMS 生成确定性(同源两次逐字节一致,字典序)+ 资产字节与 bundle.json 组件 digest 一比一闭环 + channels/stable.json 锚 schema 校验;**上传本体不在 pr-smoke,只在 release.yml**(全 hard-block 门后 + 回读自校验) | 是 |

预算 evaluator 自动合并加载 [ea1_budget.json](ea1_budget.json)(命名空间冲突即红;**开工全空,counter 登记与 evaluator 分支随 EA1.1/EA1.2 实现 PR 同落,冷启动 entries 随 e2e 取证 measured_local 回填**)。**EA1 close-out 必须跑 `--strict` 且全局零 estimated 残留**(14 §3)。

## 3. Release 层门禁(EA1.2 延伸,随实现 PR 回填)

- 既有 8 子门(RXS-0139 + channel-manifest)**0-byte 沿用**;触发器维持 `v[0-9]+.[0-9]+.[0-9]+*` 收窄。
- EA1.2 追加 job 步骤(**全部 hard-block 门之后**):真发布件构建(`--release` + crt-static rurix_rt_cabi)→ 自签(如实 selftest)→ `rurixup release` 3 组件 → SHA256SUMS → `gh release upload` → **回读自校验**(逐资产 digest 复核,失配 job 红)→ 信任根登记流(channels/stable.json 新条目自动开 PR,owner 合并 = 人工门)。
- 首次演练 = `workflow_dispatch`(防误发);生产签名(Azure)门控维持 §4 禁区 0-byte。

## 4. Nightly 追加

- 既有 nightly 全保留。**EA1 无新增 nightly 项**:分发冒烟归 PR smoke 步骤 59/60(秒级);冷启动 e2e 为一次性 evidence 非趋势项。
- (契约外并行轨道,信息性)nightly 病灶根治若本期动手 = 统一子进程超时包装 + concurrency 隔离,走常规 PR 纪律,不入本表验收面;落地后按实际回填修订记录。

## 5. Guardrail

沿用 M0~MB1 全部激活项。EA1 期动作:

1. **基准 ref 默认 `mb1-closed`**:MB1 close-out 已切,EA1 开工无需再切;PR 路径以 `GITHUB_BASE_REF` 为准。EA1.3 close-out 时切至 `ea1-closed`(agent 自主签署;不匹配 release.yml 收窄触发器,零误触发)。
2. **stable 快照 bless(check_stable_snapshot_bless)**:EA1 预期触发——EA1.1a 条款 RXS-0214/0215(spec_clauses 209→211)、EA1.1b 条款 RXS-0216/0217(211→213)、EA1.2 若落 RXS-0218/0219(→215;以实现实际为准);各与条款/实现同 PR 重 bless + bless_log 同 diff 追加(数据行忌「日期」子串);不可分 PR(步骤 49 硬红)。
3. **错误码**:EA1 拟**零新 RX 码**(rurixup 工具层 Result+退出码+机器 token 行 `RURIXUP_INSTALL_ERROR: kind=...`;spec/release.md §3 触发条件不成立);确需升档停手按段续号自 **RX7023**(§3 过期文字「7021 起」勿按其取号,条款 PR 顺手修正),en/zh 成对(bilingual 96→N)。
4. **unsafe 边界**:src/rurixup 维持 `unsafe_code = deny` + 零第三方依赖(仅 rurix-pkg);下载载体拟 curl.exe 外呼零 unsafe;若裁决 A 改选 FFI 载体 → 逐处 `// SAFETY:` + unsafe-audit **U29** 续号登记。
5. **网络纪律(新增激活项,随 EA1.1b 实现 PR 进 guardrail 核对面)**:pr-smoke 零真实外呼;下载校验 fail-closed 绝不物化/不注册/不充绿;环回例外仅显式测试 env + 127.0.0.1。
6. **spec 修订行纪律**:spec/release.md 修订表表头「版本」,数据行避「版本」子串(用「版号」)。
7. **规划文档冻结**:00–14 执行 PR 0-byte;开工裁决记 EA1_CONTRACT §7 + RFC-0012 §9;状态勘误 = 支线 A2 独立 errata PR(00 §6.3,check_planning_docs 预期红,与执行 PR 严格分离);**裁决 A~D 落地不改写 13 号文档/spike_gating**(D-312 维持待决)。
8. **trace 矩阵扫描面**:src/rurixup 已在扫描列表(RXS-0135~0139/0185~0188 锚定既有);新条款 RXS-0214+ 锚定随实现 PR 同落。
9. **LF byte-exact**:新文件 LF+尾换行;禁 Python 文本模式写文件;规划文档勘误重放保原行尾字节风格;提交前逐文件字节核 CR+尾字节。

14 §2 常驻集其余项的 EA1 期评估结论:

| 项 | 结论 |
|---|---|
| stable API 快照 | 已激活;EA1 预期 2~3 次加性重 bless(RXS-0214+ 条款增长,RXS-0180 L2) |
| MIR/PTX/DXIL/UI golden | 已激活;EA1 零变更预期(纯工具/发布面,不触编译器语义) |
| unsafe-audit 完整性 | 已激活;U29 留号(拟不触发) |
| Compute Sanitizer / NVIDIA 白名单 | 已激活维持;bundle NvidiaRedist 分区本期不带 libdevice(RFC-0012 §9 拟裁),白名单审计 0-byte 沿用 |
| 多后端(D-008/SG-003) | SG-003 维持 triggered(RFC-0011) 不回翻;G-MB1-6 AMD 尾门独立于 EA1(硬件 gated) |
| registry sumdb(D-312/SG-007) | **维持 not_triggered**——agent 拟窄裁「单端点第一方工具链分发 ≠ 包生态 registry 激活」呈裁决 A 待裁,留痕契约 §7/RFC §9/RD-025 history,不改写 registry 文件本条目 |
| MLIR(SG-001)/ Tensor Core(SG-002)/ autodiff·fusion(SG-004/005)/ Python 嵌入(SG-008)/ 自举(SG-009) | 维持 not_triggered;SG-010 留续号 |
| 贡献校验门(ci/check_contribution.py) | 已激活延续 |

## 6. 验证程序(对应契约 G-EA1-1~G-EA1-8 与计划步骤 59/60)

1. EA1.0:治理包 + RFC-0012 合入序核验(RFC Approved 先于任何实现 commit;失败测试先行——步骤 59/60 脚本与 rurixup 真实 IO/网络代码在 RFC 合入时点 main 上不存在);裁决 A 留痕先于 EA1.1b PR。
2. EA1.1a 步骤 59 前半落地后:本机 `py -3 ci/rurixup_dist_smoke.py`(物化+切换+篡改/错向双红绿);runner PR run URL 归档;trace / stable_snapshot(重 bless 后)/ bilingual / guardrails(基准 mb1-closed)全绿。
3. EA1.1b 步骤 59 后半:hermetic fixture 四路 RED + 全链绿本机与 runner 双真跑;零真实外呼核验(fixture 进程为唯一网络面)。
4. EA1.2 步骤 60 + release.yml:workflow_dispatch 演练 run URL + 回读自校验输出 + 信任根 PR 链接归档 §8。
5. 冷启动 e2e:两段式 evidence JSON(schema 校验 + measured_local)+ ea1.bench.* entries 回填后 `py -3 ci/budget_eval.py`。
6. close-out:`budget_eval --strict` 输出原文(零 estimated)+ G-EA1-1~8 留痕指针 + RD-025 处置 + SG 复评 + 双基准(mb1-closed / ea1-closed)advisory 复核输出。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-07-16 | 初版(EA1 契约配套;计划步骤 59/60 为 EA1.1/EA1.2 计划项,落地时回填 workflow YAML 实测命令与 run URL;Release 层延伸为 EA1.2 计划项(上传自动化+回读自校验+信任根登记流,全 hard-block 门后);nightly 零增量(病灶根治 = 契约外轨道信息性提示);guardrail 动作:基准 mb1-closed 无需再切、快照 2~3 次加性重 bless、拟零新 RX 码(升档停手 RX7023 起)、rurixup unsafe deny + 零依赖维持 + U29 留号、网络 fail-closed 纪律随 EA1.1b 进核对面、close-out 切 ea1-closed;SG-007 维持 not_triggered(拟窄裁呈裁决 A)+ SG-010 留续号)。**EA1 开工脚手架零 CI 代码改动**:ea1_budget.json 经 *_budget.json glob 自动纳入,counter/entries 不预造;开工不写入 workflow YAML 真实步骤 |
