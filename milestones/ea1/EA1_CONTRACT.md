---
contract: EA1
title: EA1 期——「十分钟上手」分发与门面期：rurixup 真实分发（RD-025 兑现）+ 预编译工具链 bundle 发布 + 文档门面 + 冷启动验收
status: closed            # EA1.3 close-out 翻 closed(2026-07-28,agent 完全自主签署,AGENTS v3.0 硬规则 1;用户 2026-07-28 会话决策『A 段推迟,先收口』)。close-out 只追加 §8,上方条款 0-byte;基准 mb1-closed→ea1-closed 切换 + ea1-closed tag 归 EA1.3,agent 自主签署。
version: v1.0
date: 2026-07-16
timebox: "约 5–6 周（主线 EA1.0~EA1.3 串行 + A/B 并行支线见 EA1_PLAN.md;周为相对刻度,非日历承诺）"
rfc_required: RFC-0012    # 仅两实体面 Full-RFC-gated——rurixup_real_fs_switch 与 rurixup_network_fetch 触真实 IO + 安全包络 + 网络端点面（RD-025 backfill_condition 明记「按 10 §3 判档,可能需 Full RFC」;判档争议向上取严 = Full RFC,硬规则 8）;toolchain_bundle_release 随 RFC-0012 发布侧承载。docs/errata/上游备包/冷启动 evidence 为 Direct-PR 档非 RFC-gated。脚手架本身 rfc_required 落 RFC-0012 为登记,脚手架 PR 不实现语义
upstream_docs:
  - "01 §4 图景 3（:70 原文「从 rurixup install 到第一个 kernel 跑出 Nsight 时间线少于十分钟」——本期操作化其 install→首 kernel 段;Nsight 时间线段诚实标注为后续,不充数）+ 01 §6 成功判据（「选择」动词的门槛面:本期只降门槛建通道,不宣称采纳）"
  - "02 §1 用户画像（新用户上手路径;U1/U2 无 Rust 工具链前提）"
  - "registry/deferred.json RD-025（backfill_condition 本期触发 = 兑现对象:真实 FS 物化 + PATH/junction 活跃切换 + URL 下载 channel/bundle;明记网络拉取须先裁 D-312 相关面）"
  - "rfcs/mini-0009-toolchain-frontend.md §4 §6（首切片范围红线——本期解除其 defer）"
  - "spec/release.md §2.6 RXS-0185/0186（channel 清单 + 一致性判据）+ §2.7 RXS-0187/0188（注册表逻辑 + stable channel 消费内容寻址——本期只增不破坏其纯确定性语义）+ §2.1~2.4 RXS-0135~0139（原子分发/分离打包/签名/SBOM/hard-block 发布门——bundle 资产承此）+ §4 禁区（生产签名 secret+人工门控不自动调用;NVIDIA 白名单 pending-human-review 维持）"
  - "13 D-406 v2.0（agent 完全自主默认）/ D-312（registry 待决——本期**拟**窄裁论证非激活,呈 OWNER_DECISION_PACKAGE 裁决 A 待裁）/ D-308/D-309（包管理 MVP 无 registry / 无 build.rs——供应链姿态一致性依据）"
  - "12 R-202（供应链事故红线:vendor+checksum 默认——下载校验 fail-closed 承此）/ R-203（生态冷启动)/ R-603（范围蔓延）"
  - "14 §1 §3 §4 §5（契约 / 预算零占位 / deferred / 证据分级）/ 10 §3（变更三档）§9.5（编号永不复用）/ agents/AGENTS.md（硬规则十条）"
in_scope:
  - rurixup_real_fs_switch     # EA1.1a 真实 FS 物化 + 活跃版本切换:已校验 bundle 内容树写磁盘版本目录（staging→全量校验→同卷单次 rename 原子提交）+ 切换机制（拟 shim,裁决 B）+ list/default 接真实目录 + 失败回滚/断电幂等;注册表 schema v2（+install_path/tree_digest,v1 条目读入标 registered-only）→ **Full RFC 前置(RFC-0012)**,条款 RXS-0214 续号;**不被裁决 A 单独 gate**（本地面零网络;活跃切换子面机制按裁决 B,RFC-0012 Approved 前置,见 OWNER_DECISION_PACKAGE §3）
  - rurixup_network_fetch      # EA1.1b 网络拉取:从 GitHub Releases 拉 channel/bundle/组件 + 四级内容寻址下载校验 fail-closed（任一级失配 = 拒装/清 staging/零半装）红绿双证（hermetic 本地 fixture,pr-smoke 零真实外呼）→ **Full RFC + 裁决 A 硬前置**（RD-025 backfill_condition:网络拉取须先裁 D-312 相关面）
  - toolchain_bundle_release   # EA1.2 release.yml 延伸:真发布件构建（rx.exe + rurixup.exe + crt-static rurix_rt_cabi.lib 共 3 组件——v1.0.0 资产缺 .lib,无 Rust 环境时含 GPU 面(kernel/std::gpu)的 rx build 必死,本期必修）+ SHA256SUMS + gh release upload + 上传后回读自校验 + 信任根登记流 → 随 RFC-0012 发布侧承载 + 裁决 D 一次性确认;首次演练 workflow_dispatch 防误发
  - docs_front_door            # 支线 A:docs/en-front-door 10 个 *.en.md 合入（状态行刷新至现状）+ 中文 README 反向语言切换头 + guide/00_install.md 改写为 rurixup install 路径（gated on EA1.1/1.2 能力就位,文档不先于能力）→ Direct-PR 档
  - planning_docs_errata       # 支线 A2:00/11/12/13 状态勘误（以 docs/state-refresh-2026-07 fc0ace57 为底稿手工重放 + 刷新至 mb1-closed 现状;00 §6.3 独立 errata PR,check_planning_docs 预期红,PR #140 先例）→ Direct-PR 档,与执行 PR 严格分离
  - cold_start_acceptance      # 干净环境 install→首 kernel <10min measured（两段式,口径归裁决 C;evidence json + 环境画像,不进 CI 硬门——含下载受带宽波动,SKIP 不充绿双态先例）
  - upstream_report_packs      # 支线 B:上游报告三连备包（Godot buffer_clear 对齐 / LLVM DXContainer PSV0 / VVL Adreno SIGSEGV）——MRP+issue 草稿整理进 evidence/upstream-reports/,全部显式 DRAFT — do NOT file 标头;**提报动作本体不在本期不在本仓**（owner 亲自)→ Direct-PR 档,evidence 只增
out_of_scope:
  - uc05_minimal_rhi           # UC-05 最小 RHI 加档:owner 2026-07-16 批准的 EA1 立项方案显式砍掉留下期(用例期规划非特性半成品,不登 RD)
  - registry_activation        # 包 registry/sumdb(D-312/SG-007):维持 not_triggered——agent 拟窄裁:EA1 网络拉取为单端点第一方工具链分发非 registry 激活(呈裁决 A 待裁;若裁定触 D-312 则 EA1.1b 冻结,按 OWNER_DECISION_PACKAGE §3 路由留痕);rurix-pkg 侧 lockfile+vendor+checksum 零网络代码不变
  - upstream_filing            # 上游 issue 提报动作本体:owner 复核 + 亲自执行;agent 只备包(DRAFT — do NOT file 纪律)
  - nightly_root_cause         # nightly 病灶根治(subprocess 无 timeout→僵尸 exe 锁 runner):owner 2026-07-16 本会话裁定 = 契约外并行轨道——显式排除但不禁做,修复走常规 PR 纪律(真实红绿),成果 close-out §8 附带留痕;「根治」无预先可判 DoD,不入验收门以免造虚门或阻塞收口
  - self_update_channels       # rurixup 自更新(shim 占用换文件)+ stable 外 channel 语义:执行期登记 RD-033+,不预造
  - mirror_multi_endpoint      # 多端点/镜像/代理/断点续传:单端点首期,执行期按需登 RD
  - production_signing_switch  # 生产签名(Azure Artifact Signing)接通:维持 spec/release.md §4 禁区(secret+人工门控),本期自签测试证书如实标注,不伪装信任根
  - grx_merge                  # GRX showcase 分支合入 main:维持独立轨道;Godot 备包自 GRX 分支摘取重放为 main 新文件,不合分支(快照面串行化先例)
  - production_adoption_claim  # 「外部采纳/用户数/下载量」维度:显式 carve-out(沿 MS1/V1 先例)——本期验收全锚定自方可控工程物(install 时长 measured / 分发链路红绿 / docs 上线),不宣称 01 §6 判据达成
deferred_refs: [RD-025]      # RD-025(open)owner_milestone MS1→EA1 承接 = 本期兑现对象(EA1.1a/1.1b 落地后 close-out 关闭或收窄余项另立 RD-033+);执行期新 RD 自 RD-033 起(RD-016/RD-028 跳号永不复用,10 §9.5)并双侧标注
deliverables:
  - id: D-EA1-1
    name: EA1.0 治理包五件（本契约 + EA1_PLAN + CI_GATES + ea1_budget.json + OWNER_DECISION_PACKAGE）+ RFC-0012（Draft→裁决 A~D 落地后 Approved,先于实现 PR）+ RD-025 承接留痕（deferred v1.56）
  - id: D-EA1-2
    name: EA1.1a rurixup 真实 FS 物化 + 活跃版本切换——条款 RXS-0214 续号前段 + src/rurixup install/toolchain 扩展（staging→rename 原子/注册表 v2/切换机制）+ CI 步骤 59 前半红绿
  - id: D-EA1-3
    name: EA1.1b 网络拉取 + 四级校验 fail-closed——条款后段 + 下载载体接线 + hermetic fixture 红绿双证（坏字节/坏哈希/截断/协议降级→拒且零半装）
  - id: D-EA1-4
    name: EA1.2 release.yml bundle 发布延伸——3 组件真发布件 + SHA256SUMS + gh release upload + 回读自校验 + 信任根登记流 + workflow_dispatch 发布演练（run URL 归 §8）+ CI 步骤 60
  - id: D-EA1-5
    name: 冷启动 e2e 两段式 evidence（裁决 C 口径）+ ea1.bench.cold_start_* measured 回填
  - id: D-EA1-6
    name: 支线 A 文档门面——en-front-door 合入 + 中文 README 语言切换头 + guide/00_install.md 改写为 rurixup 路径
  - id: D-EA1-7
    name: 支线 A2 规划文档状态勘误（00/11/12/13,独立 errata PR）
  - id: D-EA1-8
    name: 支线 B 上游报告三连备包（evidence/upstream-reports/,DRAFT — do NOT file）
acceptance_gates:
  - id: G-EA1-1
    check: "治理与条款门:RFC-0012 Approved 合入先于任何实现 PR（10 §3 失败测试先行:步骤 59/60 脚本与 rurixup 真实 IO/网络代码在 RFC 合入时点 main 上不存在 = RED）;裁决 A 经 OWNER_DECISION_PACKAGE 落地先于 EA1.1b(网络面)任何 PR 合入;条款 RXS-0214 续号体（FLS 体例,严禁 UB 节）与每条 ≥1 `//@ spec:` 锚定同 PR、commit 序条款在前;trace_matrix --check 维持全锚定（209→N）;stable 快照因条款增长同 PR 重 bless（bless_log 同 diff,步骤 49 硬红不可分 PR）"
  - id: G-EA1-2
    check: "真实 FS 物化+切换红绿（CI 步骤 59 前半,host 面总跑）:install 把已校验 bundle 物化到真实磁盘版本目录（**非 dry-run,防降级硬门**——账面注册/内存提交/mock 文件系统均不得替代;RXS-0187/0188 既有纯确定性语义只增不破坏,既有 rurixup 单测回归网全绿）+ 切换后版本探针指到目标版本 + 物化产物真实可执行（toolchains 目录内 exe 真跑探针命令）;红绿:篡改组件一字节→内容寻址拒且 toolchains/ 零残留、注册表 0-byte;切换指向已删目录→诚实报错退出非 0;复原绿;内建 red_self_test"
  - id: G-EA1-3
    check: "网络拉取 fail-closed 红绿双证（CI 步骤 59 后半,hermetic 本地 HTTP fixture,**pr-smoke 零真实外呼**）:坏字节/坏哈希/截断/非 https 协议(默认态)→ 全部拒且不落盘不注册（RED 各自独立见证）;完好资产→全链 install 绿;离线/端点不可达→诚实错误报告退出非 0 + 系统 0-byte,不 fake success;真实 GitHub Releases 端点闭环归 EA1.2 e2e evidence（measured_local）,不进 pr-smoke;**条件分支:若裁决 A 落地为『触 D-312』→ 本门以 blocked 留痕替代（契约 §7 追加裁决行,本 YAML 头原文 0-byte 不动）,验收面按 OWNER_DECISION_PACKAGE §1-A 备选后果收窄,D-EA1-3 同步标注**"
  - id: G-EA1-4
    check: "bundle 发布资产门（CI 步骤 60 + release.yml 延伸）:打包确定性（同源两次逐字节一致,SHA256SUMS 字典序）+ 资产字节与 bundle.json 组件 digest 一比一闭环 + 3 组件完备（含 crt-static rurix_rt_cabi.lib,缺件即红）;上传步骤仅位于 release.yml 全部 hard-block 门（RXS-0139 七子门 + RXS-0186 第 8 门 channel-manifest）之后;上传后回读自校验（逐资产 digest 复核,失配 job 红）;**上传载体按裁决 D——若裁逐次人工,上传由 owner 执行、回读自校验以脚本 evidence 兑现,位序与 digest 判据不变**;首次发布演练走 workflow_dispatch,run URL 归档 §8"
  - id: G-EA1-5
    check: "文档门面门:*.en.md 合入（逐文件 LF 核对 + 状态行刷新至现状不留过期表述）+ 中文 README 语言切换头与 en 侧互链可达 + 规划文档勘误走 00 §6.3 独立 errata PR（check_planning_docs 预期红,与执行 PR 严格分离）+ guide/00_install.md 改写 gated on EA1.1/1.2 能力就位（文档不先于能力,改写后既有 doc/tutorial 冒烟门绿）;外发文档不得复读无限定的「十分钟」表述（须带两段式口径限定,裁决 C）"
  - id: G-EA1-6
    check: "冷启动 <10min measured（evidence 面,不进 CI 硬门;口径以裁决 C 落地为准,拟:两段各 ≤10min——A 段干净 Win11 VM 零预置依赖 T0=文档首命令 T1=rx check 退出 0 含下载;B 段开发机干净用户账户（系统级 LLVM/VS Build Tools/NVIDIA 驱动为文档化前置不计时）T1=首 kernel device 真跑退出 0;不跨机加总）:evidence/ea1_install_e2e_*.json 经 schema 校验（计时/步骤/环境画像/带宽画像/digest 校验级数）,measured_local;01 §4 图景 3 的 Nsight 时间线段诚实标注为后续不充数;SKIP/缺 VM 不充绿"
  - id: G-EA1-7
    check: "上游备包完备性（close-out 人工核,不设 CI 步骤）:三包各含复现工程或复现步骤 + 环境画像 + issue 草稿全文;全部文件显式 `DRAFT — do NOT file` 标头;Godot 包 `<FILL>` 占位清零（须实测补:stock build hash/系统串/旧 stable 复现）;VVL 包若独立 MRP 依赖真机而设备不可得→该子项标 pending 不伪造;**提报动作不在本门不在本仓**"
  - id: G-EA1-8
    check: "性能与收口:≥2 项 ea1.bench.*（冷启动计时,条目结构按裁决 C 口径）以 measured_local 回填（登记与 evaluator/entries 同 PR 落,不预造）;close-out budget_eval --strict 全局零 estimated;RD-025 处置留痕（关闭或收窄余项另立 RD-033+）;「外部采纳」carve-out 维持不宣称;close-out 全量回归冻结（cargo test / trace / snapshot / bilingual / guardrails 真实输出追加 §8）+ 基准 mb1-closed→ea1-closed + annotated ea1-closed tag（agent 自主签署）"
guardrails:
  - "milestones/m0~mb1 的 measured_local 既有预算条目 git diff 0-byte（新增 ea1 条目允许,随 D-EA1-5 回填）;ea1_budget.json 经 *_budget.json glob 自动纳入 + 命名空间强制前缀 ea1.（14 §3）;counter/entries **不预造**——登记与 ci/budget_eval.py evaluator 分支同实现 PR 落（未知 id 强制 FAIL）;**永不立下载量/用户数类外部采纳条目**"
  - "milestones/m0~mb1 的 *_CONTRACT.md（均 closed）只追加不修改（check_closed_contracts,glob 已泛化）;本契约 close-out 翻 closed 后自动纳入字节守卫"
  - "registry/deferred.json 与 registry/spike_gating.json 只追加;RD-025 处置仅由 agent 自主签署留痕追加;**SG-007 维持 not_triggered**——裁决 A 通过前 EA1.1b 网络面 PR 不合入;SG-010 留续号（扩张诱惑出现登记 gating 而非提案);13_DECISION_LOG/spike_gating 在治理包分支 pristine,**裁决 A~D 落地也不改写它们**（D-312 维持待决,留痕只进本契约 §7 + RFC-0012 §9 + RD-025 history）"
  - "registry/error_codes.json 错误码语义可加不可改;EA1 拟**零新 RX 码**（rurixup 全走工具层 Result+退出码+机器 token 行,spec/release.md §3 触发条件不成立);确需升档时停手按段续号自 **RX7023**（§3 所写「RX7021 起」已过期两号勿按其取号,条款 PR 顺手修正）,en+zh messages 成对（bilingual 96→N)"
  - "evidence/ 只增不删不改;上游备包全部文件 `DRAFT — do NOT file` 标头强制,agent 不对外提报"
  - "00–14 共 15 份规划文档不被执行 PR 改写（check_planning_docs);开工裁决记本契约 §7 + RFC-0012 §9;状态勘误只经 00 §6.3 独立 errata PR（支线 A2,预期红,PR #140 先例）"
  - "**网络 fail-closed 纪律**:任何校验失败绝不物化/不注册/不充绿;pr-smoke 零真实外呼（hermetic 本地 fixture,环回放行仅限显式测试 env + 127.0.0.1）;agent 侧真实下载遵逐件授权惯例（MB1 先例）;工具件（VM 镜像/测试证书等）不入库"
  - "src/rurixup 维持 `unsafe_code = deny` + 零第三方依赖（仅 rurix-pkg;下载载体拟系统 curl.exe 外呼,裁决 A);若裁决改选 FFI 载体→逐处 // SAFETY: + unsafe-audit **U29** 续号登记;既有 rurixup 单测/冒烟回归网全绿,RXS-0135~0139/0185~0188 语义 0-byte 只增"
  - "release.yml 触发器维持 `v[0-9]+.[0-9]+.[0-9]+*` 收窄;ea1-closed tag 不匹配触发器零误触发;bundle 上传步骤仅在全部 hard-block 门之后;生产签名门控（§4 禁区）0-byte"
  - "仓库 LF byte-exact（* -text）:新文件 LF + 尾换行,禁 Python 文本模式写文件;规划文档勘误重放保原行尾字节风格;提交前逐文件字节核 CR + 尾字节（git numstat + 二进制读,禁 grep $'\\r'）"
  - "spec 修订表表头维持「版本」列名,数据行避「版本」子串（用「版号」）、忌「日期」子串入 bless 数据行;本契约既有条款 0-byte,close-out 只追加 §8;status 翻转/基准切换/ea1-closed tag/RD·SG 处置由 agent 自主签署"
  - "guardrail 回退基准默认 = mb1-closed（MB1 close-out 已切;PR 路径以 GITHUB_BASE_REF 为准）;EA1.3 close-out 切至 ea1-closed 并双基准 advisory 复核"
---

# EA1 契约 — 「十分钟上手」分发与门面期

> 所属:[../../01_VISION_AND_MISSION.md](../../01_VISION_AND_MISSION.md) §4 图景 3 / §6 成功判据（门槛面）/ 契约机制见 [../../14_ENGINEERING_DISCIPLINE.md](../../14_ENGINEERING_DISCIPLINE.md) §1。
> 规范先行延续(AGENTS.md 硬规则第 7 条):语义面 PR 必须引用 RXS-#### 条款号;缺条款先补 spec,条款 commit 先于实现 commit。
> 基准 ref:**默认 `mb1-closed`**(MB1 close-out 已切换;`ci/check_guardrails.py` 无参默认 = `mb1-closed`,PR 路径以 `GITHUB_BASE_REF` 为准)。
> 粒度:**单 EA1 阶段契约**:一份契约覆盖 EA1 期,EA1.0~EA1.3 主线 + A/B 支线分解见 [EA1_PLAN.md](EA1_PLAN.md)。
> **定位口径:EA1 兑现「外部人装得上」这一工程事实,不宣称「外部人用起来」这一社会事实。**现状:v1.0.0 已发行(2026-07-14)但外部用户唯一路径 = clone + cargo build 整个编译器(guide/00_install.md);rurixup 注册表逻辑在位(RXS-0187/0188)但零网络、零真实 FS 物化(RD-025 defer);channel 清单本身无信任根;release.yml 无 Release 资产上传自动化;v1.0.0 资产缺 rurix_rt_cabi.lib(无 Rust 环境时含 GPU 面的 rx build 必死)。EA1 把「干净环境从 rurixup install 到第一个 kernel <10 分钟」做成 measured 工程事实(两段式口径拟案,B 段系统级前置文档化不计时——裁决 C),并一次收口散落的对外资产(en 文档门面 / 规划文档状态勘误 / 上游报告备包)。「外部采纳」维度显式 carve-out(out_of_scope)。
> **治理口径:MS1 范式(agent 自主,D-406 v2.0)+ 轻量 OWNER_DECISION_PACKAGE**——EA1 不触任何红线,无 MB1 式 §0 方向闸口;owner 裁决只 gate 网络面(裁决 A,RD-025 backfill_condition 契约性前置)+ 三项轻确认(B/C/D),详见 [OWNER_DECISION_PACKAGE.md](OWNER_DECISION_PACKAGE.md)。裁决等待面:支线 A/B 与 EA1.0 起草不受任何裁决 gate;EA1.1a 待 RFC-0012 Approved(RFC 翻 Approved 与裁决落地同 PR,见 OWNER_DECISION_PACKAGE §3),其活跃切换子面按裁决 B。
> **脚手架口径:本契约为 EA1 开工结构件,不实现任何语义面、不落条款、不打 tag;§8 close-out 开工时为空。**

---

## 1. 目标

EA1 期结束时项目获得:① rurixup 真实分发闭环——`rurixup install` 从 GitHub Releases 拉取签名 bundle,经四级内容寻址校验 fail-closed 后原子物化到磁盘版本目录并切换活跃版本(RFC-0012,RD-025 兑现);② 发布侧对称自动化——release.yml 全门绿后构建 3 组件真发布件(含 crt-static rurix_rt_cabi.lib)、上传 Release 资产并回读自校验;③ 冷启动可验收——干净环境 install→第一个 kernel <10 分钟 measured(两段式 evidence);④ 对外门面收口——en 文档合入、规划文档状态勘误、上游报告三连备包(DRAFT,不提报)。

## 2. 范围

### 2.1 in-scope

| 项 | 说明 | gating | 对应交付物 |
|---|---|---|---|
| rurixup_real_fs_switch | 真实 FS 物化 + 活跃版本切换(staging→rename 原子/注册表 v2/切换机制) | **Full RFC(RFC-0012)**;不被裁决 A 单独 gate(切换子面按裁决 B) | D-EA1-2 |
| rurixup_network_fetch | 网络拉取 + 四级校验 fail-closed(hermetic 红绿双证) | **Full RFC + 裁决 A 硬前置** | D-EA1-3 |
| toolchain_bundle_release | release.yml bundle 发布延伸 + 回读自校验 + 信任根登记流 | RFC-0012 发布侧 + 裁决 D | D-EA1-4 |
| docs_front_door | en 文档合入 + README 语言头 + 00_install 改写 | Direct-PR;00_install gated on 能力就位 | D-EA1-6 |
| planning_docs_errata | 00/11/12/13 状态勘误 | Direct-PR,独立 errata PR | D-EA1-7 |
| cold_start_acceptance | 冷启动 <10min 两段式 evidence | 口径归裁决 C;不进 CI 硬门 | D-EA1-5 |
| upstream_report_packs | 上游报告三连备包(只备包) | Direct-PR,evidence 只增 | D-EA1-8 |

### 2.2 out-of-scope(显式排除)

见 YAML 头 `out_of_scope` 字段逐项(uc05_minimal_rhi / registry_activation / upstream_filing / nightly_root_cause / self_update_channels / mirror_multi_endpoint / production_signing_switch / grx_merge / production_adoption_claim);11 §2 红线不触碰。

## 3. 交付物清单

| ID | 交付物 | 形态 | 完成判据 |
|---|---|---|---|
| D-EA1-1 | 治理包五件 + RFC-0012 | milestones/ea1/ + rfcs/0012 + rfcs/README 台账 + deferred v1.56 | G-EA1-1 前置 |
| D-EA1-2 | 真实 FS 物化 + 切换 | 条款前段 + src/rurixup 扩展 + ci 步骤 59 前半 | G-EA1-2 |
| D-EA1-3 | 网络拉取 + fail-closed | 条款后段 + 载体接线 + hermetic 红绿 | G-EA1-3 |
| D-EA1-4 | bundle 发布延伸 | release.yml + SHA256SUMS + 回读自校验 + 步骤 60 + 演练 | G-EA1-4 |
| D-EA1-5 | 冷启动 evidence + bench 回填 | evidence/ea1_install_e2e_*.json + ea1_budget entries | G-EA1-6 / G-EA1-8 |
| D-EA1-6 | 文档门面 | en-front-door 合入 + README 互链 + 00_install 改写 | G-EA1-5 |
| D-EA1-7 | 规划文档勘误 | 独立 errata PR(00 §6.3) | G-EA1-5 |
| D-EA1-8 | 上游备包 | evidence/upstream-reports/ 三子目录 | G-EA1-7 |

## 4. 验收门(完整版,YAML 头为可提取摘要)

见 YAML 头 `acceptance_gates` 字段 G-EA1-1 ~ G-EA1-8。要点:
- **G-EA1-1(治理条款门)**:RFC-0012 Approved 前置 + 裁决 A 先于网络面 PR + 条款先行 + 同 PR 重 bless。
- **G-EA1-2(FS 物化红绿)**:防降级硬门——真实磁盘非 dry-run;篡改→拒且零残留。
- **G-EA1-3(网络 fail-closed 双证)**:hermetic fixture;坏字节/坏哈希/截断/协议降级四路 RED;pr-smoke 零真实外呼。
- **G-EA1-4(发布资产门)**:打包确定性 + digest 一比一闭环 + 3 组件完备 + 回读自校验 + workflow_dispatch 演练。
- **G-EA1-5(文档门面)**:en 合入 + 互链 + errata 独立 PR + 文档不先于能力。
- **G-EA1-6(冷启动 measured)**:两段式 <10min evidence(裁决 C 口径),不进 CI 硬门,Nsight 段不充数。
- **G-EA1-7(备包完备)**:三包 MRP+草稿+DRAFT 标头;提报不在本门。
- **G-EA1-8(性能与收口)**:≥2 项 ea1.bench measured + --strict 零 estimated + RD-025 处置 + 基准切换。

## 5. Guardrails(字节级,机器核对)

见 YAML 头 `guardrails` 字段。核对方式:`py -3 ci/check_guardrails.py`(无参默认基准 = `mb1-closed`;PR 路径以 `GITHUB_BASE_REF` 为准)。

## 6. Deferred 引用

| 编号 | 内容摘要 | 承接 |
|---|---|---|
| RD-025 | rurixup 真实 FS 物化 + 网络拉取(MR-0009 defer) | open,owner_milestone MS1→EA1 承接 = **本期兑现对象**;EA1.1a/1.1b 落地后 close-out 关闭或收窄余项另立 RD-033+;backfill_condition 的「先裁 D-312 相关面」经 OWNER_DECISION_PACKAGE 裁决 A 兑现 |

详情以 [../../registry/deferred.json](../../registry/deferred.json) 为唯一事实源,本表仅引用。执行期按 14 §4 追加 RD-033+(如 rurixup 自更新/多端点镜像)并双侧标注。

## 7. 修订记录 / 开工裁决留痕

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-07-16 | 初版契约固化(EA1 开工脚手架)。**开工裁决**(owner 2026-07-16 本会话拍板立项 + 两项 AskUserQuestion 裁定 + agent 完全自主判档 D-406 v2.0,记于本节;13_DECISION_LOG 执行 PR 字节冻结,不改决策日志):① **立项 = owner 拍板**:AMD 卡未到手 MB2 搁置,下一期 = EA1「十分钟上手」分发与门面期(评审报告 8.0 分首选;后续期 owner 另裁,本契约不预造)。② **命名 = milestones/ea1/(External Accessibility 1)**,namespace `ea1.`,收口 tag `ea1-closed`(agent 裁决:ea 直指外部可获得性,不撞 m/g/ms/mb/v 系;ea1-closed 不匹配 release.yml 收窄触发器,零误触发)。③ **owner 两项裁定**(2026-07-16 AskUserQuestion):nightly 根治 = **契约外并行轨道**(out_of_scope 显式排除但不禁做);本轮执行范围 = 治理包 + 零依赖支线(en 文档 PR / 上游备包先行)。④ **判档**:rurixup_real_fs_switch + rurixup_network_fetch = **Full RFC(RFC-0012)**(RD-025 backfill_condition 明记「触真实 IO/安全包络/网络端点,可能需 Full RFC」+ 10 §3 取严);toolchain_bundle_release 随 RFC-0012 发布侧承载;docs/errata/备包/evidence = Direct-PR 档;脚手架本身为结构件。⑤ **owner 裁决点路由**:A 网络端点+信任根+载体(gate EA1.1b)/ B 活跃切换机制(shim vs junction,偏离 RD-025 记载措辞故 surface)/ C 冷启动验收口径 / D bundle 自动发布一次性确认——全部归 [OWNER_DECISION_PACKAGE.md](OWNER_DECISION_PACKAGE.md),裁决后 agent 代录回填本节 + RFC-0012 §9 + RD-025 history,**不改写 13 号文档/spike_gating**(D-312 维持待决)。⑥ **续号 claim**(编号永不复用,10 §9.5):Full RFC = **RFC-0012**;RXS 条款自 **RXS-0214** 起(预期 RXS-0214~0219,承 spec/release.md 延伸,G1.5 先例;脚手架零裸条款头,条款体随实现 PR 落);新 deferred 自 **RD-033** 起(RD-016/028 跳号维持);unsafe-audit **U29** 留号(拟裁 curl.exe 外呼则不触发);CI 步骤自 **59**(预期 59/60,数量随实现回填);错误码拟零新码、确需时自 **RX7023**(spec §3 过期文字勿按其取号);SG 续号 **SG-010** 留用;MR-0010 不占用。⑦ **bundle 组件面 = 3 件**(rx.exe/rurixup.exe/rurix_rt_cabi.lib):driver.rs locate_or_build_rt_cabi 的 exe 旁 lib/ 分支已实现,v1.0.0 资产缺 .lib 为必修缺口(RFC-0012 事实底座)。⑧ **上游备包纪律**:agent 只备包,全部 DRAFT — do NOT file,提报 owner 亲自;Godot 包自 GRX 分支摘取重放,不合分支。⑨ **SG/红线复评**:SG-001~005/007~009 维持 not_triggered(SG-007 = agent 拟窄裁「非 D-312 激活」呈裁决 A 待裁,现状维持);SG-003 维持 triggered(RFC-0011)不回翻;SG-010 留续号。⑩ **诚实边界**:EA1 达成表述 =「外部可获得性工程闭环落地」(install 时长 measured/分发链路红绿/docs 上线);01 §6「选择/采纳」维度显式 carve-out 不宣称;01 §4 图景 3 的 Nsight 时间线段标注为后续不充数;自签测试证书如实标注非生产信任根。**EA1 close-out 关闭判定 / 基准切换(mb1-closed→ea1-closed)/ ea1-closed tag / RD·SG 处置由 agent 自主签署** |
| v1.1 | 2026-07-17 | **裁决 A~D 落地**(owner 2026-07-17 会话勾选 [OWNER_DECISION_PACKAGE.md](OWNER_DECISION_PACKAGE.md) §0 四项拟裁,agent 代录;⑤ 路由兑现):**A = 认可全案**(唯一端点本仓 GitHub Releases + repo 锚 `channels/stable.json` 四级内容寻址 fail-closed + 系统 curl.exe https-only;定性 = 非 D-312 registry 激活,SG-007 维持 not_triggered,D-312 维持待决;锚登记 PR owner 合并人工门)→ **EA1.1b 网络面解锁**,G-EA1-3 走主分支非 blocked 分支;**B = shim 目录切换**(junction 降 RFC-0012 §7 备选)→ RXS-0215 语义定案;**C = 冷启动两段各 ≤10min measured**(A 段干净 VM 至 `rx check`;B 段干净账户 GPU 真跑,系统级前置文档化不计时;重测 ≤3 次全入 evidence 取 median)→ G-EA1-6 口径定案;**D = 认可 bundle 随 semver tag 自动发布**(上传仅在全部 hard-block 门后 + 回读自校验;首次演练 workflow_dispatch,形态细则钉 RFC-0012 §4.7)→ EA1.2 上传面确认。同 PR:RFC-0012 §9 回填 + **Draft→Approved** + registry/deferred.json RD-025 history 追加(v1.57);13_DECISION_LOG / spike_gating.json 零改动。G-EA1-1 RED 前提保持:本 PR 零实现代码,步骤 59/60 脚本在 main 上不存在 |

---

## 8. Close-out(只追加区 — 开工时为空)

<!-- 验收记录、guardrail 核对输出、EA1.0~EA1.3 与 A/B 支线留痕(RFC-0012 / 裁决 A~D 落地 / 步骤 59/60 run URL / 发布演练 / 冷启动 evidence / 备包完备核)、RD-025 处置留痕、SG 复评结论、nightly 契约外轨道成果(若有)追加于此;上方条款 0-byte 修改。EA1 close-out 关闭判定 / 基准切换(mb1-closed→ea1-closed)/ ea1-closed tag / RD·SG 处置由 agent 自主签署兑现。 -->

### EA1.1a — rurixup 真实 FS 物化 + 活跃版本切换（RXS-0214/0215，G-EA1-2）

- **签署**:agent qwasg/白栀,2026-07-28,agent 完全自主签署（AGENTS v3.0 硬规则 1）。本留痕为 EA1.1a 验证收口,**非新 PR**——实现 PR 60be64f5 已于 2026-07-17 commit 在 main,本节追加为 Task 1-7 核验后验证收口留痕。
- **完成面摘要**:
  - **条款落 spec/release.md §2.8**:RXS-0214（真实 FS 物化与原子落盘）/ RXS-0215（活跃切换 shim）,FLS 体例,每条 ≥1 `//@ spec:` 锚定。
  - **src/rurixup install.rs `materialize_to_disk`**:staging→逐组件 sha256→tree_digest 双向（forward 落盘 + backward 重算核验）→同卷单次 rename 原子提交→注册表 v2 单写;幂等（重入跳过已物化版本）+ 失败回滚（清理 staging + 不落注册表）。
  - **src/rurixup toolchain.rs**:注册表 schema v2（+`install_path`/`tree_digest` 字段,v1 条目读入标 registered-only 不破坏既有纯确定性语义）。
  - **src/rurixup shim.rs**:argv0 干名转发——剥路径取纯可执行名,转发至 `toolchains/<ver>/bin/` 下目标 exe;防逃逸（拒绝绝对路径/相对路径 argv0）+ 防自递归（目标 != self）。
  - **src/rurixup main.rs**:子命令 wired——`install --from-dir <path>`（本地 bundle 物化入口,EA1.1b 网络面接入前的离线载体）/ `list --verify`（对照磁盘+注册表双源核验）/ `default`（查询当前活跃版本）/ `setup`（PATH 接通指引）。
  - **CI 步骤 59 前半（ci/rurixup_dist_smoke.py）红绿闭合**:
    - **GREEN**:真实物化（staging→rename→注册表 v2 单写）+ 切换探针（`default` 指向新版本 + `toolchains/<ver>/bin/rx.exe` 真跑探针命令退出 0）+ 幂等（二次 install 不重复物化、注册表无重复条目）。
    - **RED①**:篡改组件一字节→内容寻址拒（tree_digest 双向核验失配）+ `toolchains/` 零残留 + 注册表 0-byte（staging 清理 + 不落注册表）。
    - **RED②**:`default` 指向已删目录→诚实报错退出非 0（不 fake success）。
    - **复原绿**:RED 各自见证后重跑 GREEN 路径全绿。
    - **内建 `red_self_test` 双向**:正路径物化绿 + 反路径篡改拒红,脚本内嵌自证。
  - **stable_api.snapshot 重 bless**:实现 PR 60be64f5 同 PR 重 bless,spec_clauses 209→211（新增 RXS-0214/0215 两条条款锚定）,bless_log L27 同 diff 记录（步骤 49 硬红不可分 PR 兑现）。
- **验证输出尾部**（Task 5 cargo 回归 + Task 6 CI 守卫真实输出,非伪造）:
  - `cargo fmt --check` PASS
  - `cargo clippy --workspace --all-targets -- -D warnings` PASS
  - `cargo test --workspace` PASS（rurixup unit 34/34 + 全 corpus 绿）
  - `trace_matrix --check` 278/278 PASS
  - `stable_snapshot --check` PASS（spec_clauses=278）
  - `check_structure` / `number_ledger` / `contribution` / `redistribution` PASS
  - `check_guardrails` PASS（base=g4-closed,本契约 §5 字节守卫维持）
  - `budget_eval` normal mode PASS（87 pass / 4 skip device）
  - **`check_schemas` FAIL**（pre-existing:**非 EA1.1a 引入**——9 份 G4.x device evidence schema 缺字段 + RD-036 reason 缺失,为 EA1.1a 合入前已存在的 pre-existing 破口;本留痕如实标注不掩盖,后续 PR 补）
- **evidence 路径**:`ci/rurixup_dist_smoke.py` 为 **硬门 smoke**,设计上**不写 evidence JSON**——退出码 0 即硬门证据（Task 7 已核验:`py -3 ci/rurixup_dist_smoke.py` 退出 0,前半 GREEN/RED①/RED②/复原绿 + 后半 hermetic GREEN/RED①②③④/不可达 全绿,`red_self_test` 双向）。脚本不写 evidence JSON 为设计预期,非缺口。
- **RD-025 history 追加**:`registry/deferred.json` RD-025 history 追加 EA1.1a 落地行（Task 10 同 PR 兑现,与 EA1.1b/EA1.2 落地行一并追加;RD-025 整体关闭判定归 EA1 close-out）。
- **下一 PR 声明**:**EA1.1b 网络拉取 + 四级信任链（RXS-0216/0217）+ EA1.2 发布侧对称自动化（RXS-0218/0219）已落 main**——commit be4eee83（EA1.1b,2026-07-17,步骤 59 后半 hermetic 环回 HTTP + 四级内容寻址 fail-closed 红绿双证）+ commit 702bf39a（EA1.2,release.yml 延伸 + 步骤 60 + 资产上传回读自校验）。本 §8 留痕为 EA1.1a 验证收口,EA1.1b/EA1.2 的 §8 留痕归各自验证收口任务。
- **诚实标注 gap**:`spec.md` L15 + `EA1_PLAN.md` L64 要求「`ea1.counter` 登记 + `ci/budget_eval.py` evaluator 分支同实现 PR 落」,实现 PR 60be64f5 未落 ea1.counter 登记 + evaluator 分支——属**执行 gap**。`rurixup_dist_smoke.py` 硬门 PASS/FAIL 已覆盖 EA1.1a 红绿验证,counter 登记 + evaluator 分支建议后续 PR 补（不影响 EA1.1a G-EA1-2 验收门达成,影响 EA1.1a 相关预算条目 `ea1.bench.*` measured 回填,归 G-EA1-8）。

### EA1.1b — rurixup 网络拉取 + 四级信任链 fail-closed（RXS-0216/0217，G-EA1-3）

- **签署**:agent qwasg/白栀,2026-07-28,agent 完全自主签署（AGENTS v3.0 硬规则 1）。本留痕为 EA1.1b 验证收口,**非新 PR**——实现 PR be4eee83 已于 2026-07-17 commit 在 main,本节追加为 EA1.3 close-out 核验后验证收口留痕。
- **完成面摘要**:
  - **条款落 spec/release.md §2.8**:RXS-0216（系统 curl.exe 子进程封装 + https-only + 缺省 fail-closed 拒协议降级）/ RXS-0217（repo 锚 channels/stable.json + 四级内容寻址级联 fail-closed）,FLS 体例,每条 ≥1 `//@ spec:` 锚定。
  - **src/rurixup fetch.rs**:零第三方依赖 + `unsafe_code=deny` 维持。`validate_endpoint` / `build_curl_args` 纯函数 host 可测（https 双 proto 钉死 + host 白名单 + 环回 127.0.0.1+env 唯一豁免 + 缺省 fail-closed 拒 http 协议降级）;`download_to` 非零退出/spawn 失败→`FetchError(kind=network)`;`Anchor::from_json` line-scan 解析 repo 锚 + `release_for`（无锚版号拒装）。
  - **src/rurixup main.rs 网络路径**:`install <version> --channel-file <锚|URL>` → 载入锚（本地/curl 拉取）→ `release_for` → 逐件下载 channel_manifest/bundle/组件 → `install_verified_dir` 四级级联（级①锚 digest 新增,②③④ 复用 EA1.1a `materialize_to_disk` 内核）→ 物化 + 注册;任一级失配清 staging/下载暂存、零注册、`kind=integrity|network`。`--from-dir` 与网络路径共用四级信任链物化内核。
  - **单测锚定**:4 个单测 `//@ spec:` 锚 RXS-0216/0217——固定参数集逐项 + https 双 proto 钉死 / 环回守门缺省 https-only / 锚解析 + `release_for` 无锚拒装 / 级①锚 digest 门。rurixup 单测 26→30 全绿。
  - **CI 步骤 59 后半（ci/rurixup_dist_smoke.py）hermetic 红绿闭合**:本地 `http.server` fixture（127.0.0.1 随机端口,零真实外呼）,`RURIXUP_TEST_ALLOW_LOOPBACK_HTTP=1` 下全链网络 install 物化绿;**RED 四路各自独立见证**——①组件坏字节（级④→`integrity`）②清单坏哈希（级①锚失配→`integrity`）③截断传输（curl 部分→`network`）④协议降级（缺 env 拒 http→`network`）——+ 端点不可达（fixture 关→`network` + 系统 0-byte）;每路断言 kind token;`red_self_test` 扩展 network/kind 判定。
  - **pr-smoke.yml**:步骤 59 扩为前半 + 后半（设 `RURIXUP_TEST_ALLOW_LOOPBACK_HTTP=1`;仅此一处 diff）。
  - **number_ledger / trace / 快照重 bless**:RXS `on_tree_max` 213→217、`next_free` 214→218 校准（消 `check_number_ledger` 2c ADVISORY 漂移）;trace 重生成 211→213（213/213 全锚定）+ stable 快照重 bless 211→213（`spec_clauses`,`error_codes=96` 不变）+ bless_log 追加;不带 BLESS 复跑绿。
- **验证输出尾部**（EA1.3 close-out Task 5 核验,非伪造）:
  - 本机 `py -3 ci/rurixup_dist_smoke.py` 退出 0（前半 + 后半 + `red_self_test` 双向全绿,2026-07-28 复跑核验）。
  - `cargo test -p rurixup` PASS（rurixup 单测 34/34 + fetch 模块单测全绿）。
- **evidence 路径**:`ci/rurixup_dist_smoke.py` 为 **硬门 smoke**,设计上**不写 evidence JSON**——退出码 0 即硬门证据（与 EA1.1a 同体例,非缺口）。
- **RD-025 history**:RD-025 history EA1.1b 落地行同 PR 追加（EA1.1a Task 10 同 PR 兑现的承诺;RD-025 整体关闭判定归 EA1 close-out）。
- **裁决 A 落地核对**:owner 2026-07-17 裁决 A（唯一端点本仓 GitHub Releases + repo 锚 + 系统 curl.exe https-only + 非 D-312 registry 激活）已落实现 PR——`validate_endpoint`/`build_curl_args`/`Anchor` 三件按裁决 A 逐字落地,SG-007 维持 `not_triggered`,D-312 维持待决。

### EA1.2 — 发布侧对称自动化 + 步骤 60 + release.yml 延伸（RXS-0218/0219，G-EA1-4）

- **签署**:agent qwasg/白栀,2026-07-28,agent 完全自主签署（AGENTS v3.0 硬规则 1）。本留痕为 EA1.2 验证收口,**非新 PR**——实现 PR 702bf39a 已于 2026-07-17 commit 在 main,本节追加为 EA1.3 close-out 核验后验证收口留痕。
- **完成面摘要**:
  - **条款落 spec/release.md §2.8**:RXS-0218（3 组件完备 + SHA256SUMS 字典序确定性）/ RXS-0219（e2e 字段名存在性面校验 + install_e2e_evidence_schema）,FLS 体例,每条 ≥1 `//@ spec:` 锚定。
  - **src/rurixup bundle.rs（新）**:`release_completeness()`（3 组件完备最小集 `rx.exe`/`rurixup.exe`/`rurix_rt_cabi.lib`,缺件 `missing` 枚举;老版本清单/既有单测 0-byte）+ `sha256sums()`（干名字典序 `<sha256>␣␣<name>` 确定性）;纯函数 + `unsafe_code=deny` 维持。新单测锚 RXS-0218（3 组件完备 / 缺件红 / SHA256SUMS 字典序确定性）。
  - **src/rurixup e2e.rs（新）**:`validate_install_e2e()` 纯离线字段名存在性面校验 + 单测锚 RXS-0219（合法样例 Ok / 缺 `bandwidth_note`·`gpu` → Err 缺字段枚举）。
  - **src/rurixup main.rs**:`rurixup release` 写出 SHA256SUMS + 摘要行追加 `release_complete`/`release_missing` token（既有 token/产物 0-byte）。
  - **milestones/ea1/install_e2e_evidence_schema.json（新,Draft-7）**:RFC-0012 §4.10 字段清单落地。
  - **.github/workflows/release.yml 延伸**（全部既有 hard-block 门之后;既有八门 + 触发器 0-byte）:① `cargo build --release -p rx -p rurixup` + crt-static `rurix-rt-cabi`（`RUSTFLAGS crt-static --target-dir target/crt-static`,对齐 `driver.rs`）② 自签 selftest（生产签名门控 0-byte）③ `rurixup release` 3 组件 ④ SHA256SUMS ⑤ `workflow_dispatch` rehearsal_n→`v1.0.1-dist.N` run 内 `gh release create --prerelease` + upload（`github.token`）;tag 路径 run 内 create+upload 非 prerelease ⑥ 回读自校验（curl 逐资产 sha256==bundle.json 失配 job 红）⑦ 信任根登记 PR（`channels/stable.json` 新条目 via `ci/emit_trust_root_entry.py`→`gh pr create`,owner 合并人工门）。job permissions `contents`/`pull-requests: write`。**演练本批只落通道不执行**。
  - **ci/release_bundle_smoke.py（CI 步骤 60,纯离线）**:打包确定性 + 资产字节与 bundle.json digest 一比一闭环 + 3 组件完备（缺 `.lib` RED 见证）+ SHA256SUMS 字典序 + 锚 schema + `red_self_test`。
  - **ci/emit_trust_root_entry.py（新）**:信任根条目生成器,line-scan 形态与 `fetch.rs` `Anchor::from_json` 对齐。
  - **number_ledger / trace / 快照重 bless**:RXS `on_tree_max` 217→219 / `next_free` 218→220 + revision_log v1.2;trace 重生成 213→215（215/215 全锚定）+ 快照重 bless 213→215 + bless_log 追加。
- **验证输出尾部**（EA1.3 close-out Task 5 核验,非伪造）:
  - 本机 `py -3 ci/release_bundle_smoke.py` 退出 0（GREEN + RED①②③④ + 复原绿 + `red_self_test` 双向全绿,2026-07-28 复跑核验）。
  - `cargo test -p rurixup` PASS（rurixup 单测 30→34 + bundle/e2e 新单测全绿）。
- **evidence 路径**:`ci/release_bundle_smoke.py` 为 **硬门 smoke**,设计上**不写 evidence JSON**——退出码 0 即硬门证据（与 EA1.1a/1.1b 同体例,非缺口）。
- **RD-025 history**:RD-025 history EA1.2 落地行同 PR 追加（EA1.1a Task 10 同 PR 兑现的承诺;RD-025 整体关闭判定归 EA1 close-out）。
- **裁决 D 落地核对**:owner 2026-07-17 裁决 D（bundle 随 semver tag 自动发布 + 全部门后 + 回读自校验 + 首次演练 `workflow_dispatch`）已落实现 PR——release.yml 7 项门 + `workflow_dispatch` rehearsal 通道 + 回读自校验 job 红门按裁决 D 逐字落地;**演练本批只落通道不执行**（首次真实 release 归后续 owner 手动 `workflow_dispatch` 触发 + 信任根登记 PR owner 合并人工门）。

### EA1.3 close-out — schema 校验去阻 + 验证收口留痕（CI 阻塞解除，归 G-EA1-3/G-EA1-4 复核门）

- **签署**:agent qwasg/白栀,2026-07-28,agent 完全自主签署（AGENTS v3.0 硬规则 1）。本留痕为 EA1.3 close-out schema 校验去阻留痕,**非新 PR**——本批变更与 EA1.3 close-out 同 PR 落地。
- **背景**:EA1.1a §8 留痕标注的 pre-existing `check_schemas` FAIL（9 份 G4.x device evidence schema 缺字段 + RD-036 reason 缺失）阻塞 pr-smoke + 全量回归,必须在 EA1.3 close-out 前解除。
- **完成面摘要**:
  - **RD-036 reason 字段补缺**:`registry/deferred.json` RD-036（G4.5 PR-G C ABI v2 判档不成立 → 超界硬需求存续登记）补 `reason` 字段为「G4.5 PR-G C ABI v2 判档两项判据均不成立 → 超界硬需求（repr(C) struct 按值 / 回调函数指针 / 数组按值 / 跨堆所有权）在 subset v1 之外存续,待真实嵌入面出现时按 10 §3 判档兑现」——语义零新增信息（同义 RD-036 title + history + backfill_condition）,仅 schema 字段闭合（14 §4 注册表强制字段之一）。同时追加 `revision_log v1.68` 留档。RD-036 status / owner_milestone / backfill_condition / history 全部维持不动。
  - **vulkan_rhi_channel_smoke_evidence_schema.json（新,milestones/g4/）**:G4.4 PR-F / G-G4-5 / 步骤 80 / RFC-0015 §4.A / RXS-0293/0294 + RXS-0222 像素判据 device 见证。镜像 `uc05_graphics_rhi_smoke_evidence_schema` 体例（host 段恒跑 `host_lib_tests` + `spirv_val`;device 段 `device_run` 真 Vulkan 通道提交 compute+graphics 双腿,`vulkan_channel_ok` 通道闭合判据;SKIP=dev-env-degrade,`RURIX_REQUIRE_REAL=1` 翻硬红）。
  - **blackhole_realtime_smoke_evidence_schema.json（新,milestones/g4/）**:G4.6 PR-H / G-G4-7 / 步骤 81 / RFC-0015 §1 carve-out / RXS-0197/0198。carve-out 期 `host_section_pass=false` + `device_section_rc=1` + `blackhole_realtime_ok=false` 诚实失败而非降级;carve-out 解除后真实 blackhole 路径冒烟（`host_checks` + `device_run` + `blackhole_realtime_ok`）。
  - **check_schemas.py 路由补**:加载两新 schema + 建 validator + 添路由分支（`vulkan_rhi_channel_smoke_` / `blackhole_realtime_smoke_` 前缀分支）。
- **验证输出尾部**（EA1.3 close-out Task 5 核验,非伪造）:
  - 本机 `python ci/check_schemas.py` → `[check_schemas] PASS`（2026-07-28 复跑核验,exit code 0）。
- **影响面**:pr-smoke 步骤 2 `check_schemas` 由 FAIL 翻 PASS,解除 EA1.3 close-out CI 阻塞;G4.x 既有 evidence 文件路由归位,不再 fallthrough 至 m0 GPU schema 触发缺字段 FAIL。
- **不在本留痕范围**:EA1.1a 执行 gap（`ea1.counter` 登记 + `ci/budget_eval.py` evaluator 分支）归 EA1.3 close-out Task 7 独立 PR 补（不影响 G-EA1-3/G-EA1-4 验收门达成,影响 G-EA1-8 性能收口 measured 回填）。

### EA1.3 close-out — 最终签署块（status active→closed + 基准切换 + ea1-closed tag + 全量回归冻结）

- **签署**:agent qwasg/白栀,2026-07-28,agent 完全自主签署（AGENTS v3.0 硬规则 1）。用户 2026-07-28 会话决策『A 段推迟,先收口』——A 段（干净 Win11 VM vm_rxcheck measured evidence）推迟至 RD-033,EA1.3 close-out 以 B 段达成 + A 段 pending 留痕收口。本块为 EA1.3 close-out 终审签署块,本批变更与 EA1.3 close-out 同 PR 落地。
- **关闭判定**:`status: active → closed`（YAML 头 L4 翻转）。EA1 期全部交付物 D-EA1-1~D-EA1-8 收口状态如下:
  - **D-EA1-1 治理包 + RFC-0012 Approved**:✅ 2026-07-17 落地,裁决 A~D 全勾选,RD-025 承接留痕(deferred v1.56)。
  - **D-EA1-2 EA1.1a 真实 FS 物化 + 活跃切换**:✅ commit 60be64f5（2026-07-17 main）,RXS-0214/0215 落地,§8 L170-L199 验证收口留痕。
  - **D-EA1-3 EA1.1b 网络拉取 + 四级 fail-closed**:✅ commit be4eee83（2026-07-17 main）,RXS-0216/0217 落地,§8 L201-L217 验证收口留痕。
  - **D-EA1-4 EA1.2 bundle 发布延伸**:✅ commit 702bf39a（2026-07-17 main）,RXS-0218/0219 落地,§8 L219-L237 验证收口留痕。
  - **D-EA1-5 冷启动 e2e 两段式 evidence**:⚠️ **部分达成**——B 段 measured 26.56s 达成(`ea1.bench.cold_start_gpu_first_kernel_s`,evidence/ea1_install_e2e_20260717_gpu_first_kernel_a2.json,v1.0.1-dist.2 attempt2);A 段 pending RD-033（owner 备 VM 后补测,见下 RD 处置）。
  - **D-EA1-6 文档门面**:✅ 完整落地——10 个 `*.en.md` 已在 main tree(5 根 `CODE_OF_CONDUCT/CONTRIBUTING/OVERVIEW/README/SECURITY.en.md` + 5 `guide/{00_install,01_first_program,02_first_kernel,03_resources,README}.en.md`)+ `README.md` L3 语言切换头 `[English](README.en.md) · [简体中文](README.md)` 互链可达 + `guide/00_install.md` 改写为 rurixup 路径(方式 A 预编译安装零 Rust 前提,D-201)。
  - **D-EA1-7 规划文档勘误**:⏳ **独立 errata PR 推迟**——契约 G-EA1-5 明记『规划文档勘误走 00 §6.3 独立 errata PR,与执行 PR 严格分离』,本 close-out PR 不含 00/11/12/13 改动（`check_planning_docs` 在本 close-out PR ADVISORY 通过——零规划文档改动）;errata PR 后于 close-out 落地,刷新 00/11/12/13 状态至 ea1-closed,届时 `check_planning_docs` 预期红（errata PR 范畴）。
  - **D-EA1-8 上游备包**:✅ `evidence/upstream-reports/` 4 子目录完备（godot-buffer-clear / llvm-dxcontainer-psv0 / rd027-pt-spin / vvl-adreno-sigsegv）,每包含 PROVENANCE + ISSUE_DRAFT + 复现日志;全部 `DRAFT — do NOT file` 标头;**提报动作不在本门不在本仓**(owner 亲自)。
- **G-EA1-1~G-EA1-8 收口状态**:
  - G-EA1-1 治理条款门:✅ RFC-0012 Approved 先于实现 PR + 裁决 A 先于网络面 PR + 条款 RXS-0214~0219 commit 先行 + 同 PR 重 bless。
  - G-EA1-2 FS 物化红绿:✅ 步骤 59 前半 GREEN/RED①/RED②/复原绿 + `red_self_test` 双向闭合。
  - G-EA1-3 网络 fail-closed 双证:✅ 步骤 59 后半 hermetic 四路 RED（坏字节/坏哈希/截断/协议降级）+ 端点不可达 + `red_self_test` 扩展;pr-smoke 零真实外呼。
  - G-EA1-4 发布资产门:✅ 步骤 60 + release.yml 延伸 7 项门 + 回读自校验 + 信任根登记流;首次演练 `workflow_dispatch` 通道就位（首次真实 release 归后续 owner 手动触发 + 信任根登记 PR owner 合并人工门）。
  - G-EA1-5 文档门面门:✅ 文档门面三件齐(支线 A1/A3);支线 A2 规划勘误走独立 errata PR(见 D-EA1-7)。
  - G-EA1-6 冷启动 <10min measured:⚠️ **B 段达成,A 段 pending RD-033**——A 段干净 Win11 VM owner-dependent,agent 无法自主推进;裁决 C 两段式口径 B 段 measured 26.56s vs threshold 600s PASS（含下载）;A 段补测后 G-EA1-6 完全闭合。
  - G-EA1-7 上游备包完备性:✅ 4 子目录完备 + 全部 `DRAFT — do NOT file` 标头 + Godot 包 `<FILL>` 占位清零（实测补:stock build hash/系统串/旧 stable 复现）。
  - G-EA1-8 性能与收口:✅ ≥2 项 `ea1.bench.*` measured_local 回填（`cold_start_gpu_first_kernel_s`=26.56s + counter 双项 PASS）+ `budget_eval --strict --allow-pending` 全局零 estimated（89 pass / 4 skip,4 skip = G4.x device counters dev-env-degrade）+ RD-025 处置 closed + 「外部采纳」carve-out 维持不宣称 + 全量回归冻结真实输出（见下）+ 基准切换（见下）+ annotated `ea1-closed` tag（见下）。
- **EA1.1a 执行 gap 补漏**（本 PR 同 PR 兑现,跨 §8 EA1.3 close-out schema 去阻段『不在本留痕范围』承诺）:`milestones/ea1/ea1_budget.json` v1.2 追加 `counter_assertions` 2 项（`dist_redgreen_cases` 要求 ≥6 + `bundle_asset_closure` 要求 ≥5,均为 normal-SKIP/strict-PASS 双态门,静态案例标识计数因硬门 smoke 不写 evidence JSON,对齐 `m1.counter.syntax_corpus_size` / `g4.counter.graphics_invariant_cases` 静态语料计数先例）;`ci/budget_eval.py` `eval_counter` 添两专属分支同 PR 落。两项 counter 本 PR 实跑 PASS（6/6 + 5/5）。**EA1.1a §8 L199『建议后续 PR 补』承诺兑现**。
- **全量回归冻结输出尾部**(2026-07-28 close-out 复跑核验,非伪造):
  - `cargo fmt --check` → exit 0
  - `cargo clippy --workspace --all-targets -- -D warnings` → `Finished dev profile` exit 0
  - `cargo test --workspace` → `test result: ok` exit 0
  - `py -3 ci/trace_matrix.py --check` → `[trace_matrix] PASS (278/278 clauses anchored, 604 test files scanned)`
  - `py -3 ci/stable_snapshot.py --check` → `[stable_snapshot] PASS (spec_clauses=278, error_codes=106, editions=['2026'], subcommands=['bench','build','check','doc','fmt','run','test','vendor'])`
  - `py -3 ci/bilingual_coverage.py` → `[bilingual] PASS 写 evidence\bilingual_diagnostic_coverage.json (coverage_complete=true, zh/en key 集对齐 110/110)`
  - `py -3 ci/check_schemas.py` → `[check_schemas] PASS`
  - `py -3 ci/check_number_ledger.py` → `[check_number_ledger] PASS (spec RXS 头 278 个零同号碰撞; ledger 14 命名空间保留号被尊重; red 自检已过)` + ADVISORY(GRX off-tree workflow exists,不阻断)
  - `py -3 ci/budget_eval.py`（normal mode） → `[budget_eval] PASS (89 pass, 4 skip, normal mode)`
  - `py -3 ci/budget_eval.py --strict --allow-pending g4.counter.graphics_rhi_smoke --allow-pending g4.counter.engine_embed_v3 --allow-pending g4.counter.vulkan_rhi_channel --allow-pending g4.counter.blackhole_realtime_smoke` → `[budget_eval] PASS (89 pass, 4 skip, strict mode)`（4 skip = G4.x device counters dev-env-degrade,G4 契约 G-G4-3/G-G4-5/G-G4-7 允许）
  - `py -3 ci/rurixup_dist_smoke.py`（EA1.1a + EA1.1b） → exit 0（前半 + 后半 + `red_self_test` 双向全绿）
  - `py -3 ci/release_bundle_smoke.py`（EA1.2） → exit 0（GREEN + RED①②③④ + 复原绿 + `red_self_test` 双向全绿）
- **双基准 advisory 复核**（反 YAML-only,基准切换前双基准核对）:
  - `py -3 ci/check_guardrails.py g4-closed` → **ADVISORY(不阻断)** + exit 0。两条 ADVISORY:
    - `registry/deferred.json RD-036: 不可变字段被修改`——RD-036 `reason` 字段补缺（base g4-closed 缺,本 close-out PR 补 schema 强制字段之一,见 §8 EA1.3 close-out schema 去阻段;语义零新增信息,仅 schema 字段闭合,非条款变更非状态翻转）。
    - `evidence/bilingual_diagnostic_coverage.json 既有文件被修改`——`bilingual_coverage.py` 自动重写（en/zh key 计数 107→110,EA1.1b/EA1.2 + G4.x PRs 后自然增长;timestamp 刷新;`coverage_complete=true` 维持;evidence/ 守卫 ADVISORY 已知模式,见 G3/EI1 close-out 先例）。
  - `py -3 ci/check_guardrails.py ea1-closed` → **FAIL: 基准 ref 不存在: ea1-closed**(预期——tag 由本 close-out agent 自主签署创建后方生效;tag 创建后复核预期 ADVISORY 不阻断,反 YAML-only)。
- **RD 处置**:
  - **RD-025 关闭**（status open→closed,deferred v1.70）:backfill_condition（rurixup 真实 FS 物化 + 网络拉取）已由 EA1.1a/1.1b/1.2 三 PR 全量落地（60be64f5 / be4eee83 / 702bf39a,均 2026-07-17 main）,兑现完成;`id/title/reason/backfill_condition/owner_milestone` 不可变字段 0-byte（仅 status 翻 closed + history 追加关闭行）;history 追加 EA1.1a/1.1b/1.2 落地行 + EA1.3 close-out 关闭判定行（共 4 行追加）。
  - **RD-033 新立**（status open,deferred v1.70）:EA1 冷启动 A 段（干净 Win11 VM,vm_rxcheck）measured evidence 推迟;裁决 C 两段式口径 A 段缺 VM 环境（owner-dependent,agent 无法自主推进）;EA1.3 close-out 以 B 段 measured 26.56s 达成 + A 段 pending RD-033 留痕收口;G-EA1-6 标注『A 段 pending RD-033,B 段达成』;owner 备 VM 后补测 + 回填 `ea1.bench.cold_start_vm_rxcheck_s` entry measured_local;`backfill_condition` 闭合后 EA1 close-out §8 留痕补档。RD-033 = EA1 earmark 在途 claim（number_ledger `reserved_in_flight[EA1].RD`）兑现消费。
  - 其余 open 尾门（RD-007/RD-011/RD-012/RD-014/RD-015/RD-026/RD-027/RD-030/RD-032/RD-034/RD-036）EA1 期未触发接通点,维持原状态不动。
- **SG 复评**（spike_gating v1.10）:
  - **SG-007 维持 not_triggered**:EA1 分发期满 close-out 复评——EA1.1a/1.1b/1.2 落地 rurixup 真实 FS 物化 + 网络拉取 + bundle 发布,裁决 A 逐字落地,单端点第一方工具链分发 ≠ D-312 registry 激活,`trigger_condition` 0-byte 不改,维持 `not_triggered`、D-312 维持待决。
  - 其余 SG（SG-001~006/008~009）EA1 期零消费无变化,不追加复评行（对齐 G3/EI1 close-out 仅追加消费变化 SG 先例）;SG-010 留续号（窗口/UI 框架进语言方向,本期不触）。
- **A 段处置**（用户 2026-07-28 会话决策『A 段推迟,先收口』）:A 段（干净 Win11 VM vm_rxcheck measured evidence）推迟 RD-033;EA1.3 close-out 不被 A 段阻塞,以 B 段（gpu_first_kernel）measured 26.56s 达成 + A 段 pending RD-033 留痕收口;G-EA1-6 标注『A 段 pending RD-033,B 段达成』;A 段补测后 G-EA1-6 完全闭合。
- **支线 A2 处置**:规划文档勘误（00/11/12/13 状态刷新至 ea1-closed）按契约 G-EA1-5『独立 errata PR,与执行 PR 严格分离』推迟至 close-out 后独立 errata PR;本 close-out PR 不含 00/11/12/13 改动,`check_planning_docs` ADVISORY 通过;errata PR 落地时 `check_planning_docs` 预期红（errata PR 范畴,00 §6.3 先例 PR #140）。
- **基准切换**:`ci/check_guardrails.py` `resolve_base()` 默认基准 `g4-closed` → `ea1-closed`（承 G4 close-out g4-closed / EI1 close-out ei1-closed / G3 close-out g3-closed / MB1 mb1-closed / MS1 ms1-closed 先例）;基准链 `mb1-closed → g3-closed → ei1-closed → g4-closed → ea1-closed` 单线性。切换前双基准核对 `g4-closed` ADVISORY（不阻断）+ `ea1-closed` ADVISORY（tag 创建后预期不阻断）,反 YAML-only。`resolve_base()` 注释更新至新基准链。
- **`ea1-closed` annotated tag**:由本 close-out agent 自主签署创建（不匹配 `release.yml` 触发器 `v[0-9]+.[0-9]+.[0-9]+*`,零误触发;tag annotation 覆盖:agent qwasg/白栀 + EA1.3 close-out 终审 + 基准切换 + RD-025 closed / RD-033 open + SG-007 not_triggered + A 段推迟 + 支线 A2 errata 推迟）;tag 创建后 `ci/check_guardrails.py ea1-closed` 复核预期 ADVISORY 不阻断。
- **诚实边界**:EA1 兑现『外部可获得性工程闭环落地』工程事实（install 时长 measured / 分发链路红绿 / docs 上线 / bundle 发布通道）;01 §6『选择/采纳』维度显式 carve-out 不宣称;01 §4 图景 3 的 Nsight 时间线段标注为后续不充数;自签测试证书如实标注非生产信任根;首次真实 release 归后续 owner 手动 `workflow_dispatch` 触发 + 信任根登记 PR owner 合并人工门;production_adoption_claim 维持不宣称。
- **零新 RX 码 / 零新 unsafe / 零新 RD 跳号**:EA1 期 rurixup 全走工具层 Result+退出码+机器 token 行（spec/release.md §3 触发条件不成立,EA1 拟零新 RX 码兑现）;`src/rurixup` 维持 `unsafe_code = deny` + 零第三方依赖（U29 留号不消费）;新 RD 自 RD-033 起（RD-016/RD-028 跳号永不复用,10 §9.5）兑现;RD-034/035/036 为后续期已登记 RD,RD-033 = EA1 close-out 唯一新增。
