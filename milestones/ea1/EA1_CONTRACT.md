---
contract: EA1
title: EA1 期——「十分钟上手」分发与门面期：rurixup 真实分发（RD-025 兑现）+ 预编译工具链 bundle 发布 + 文档门面 + 冷启动验收
status: active            # active → closed（close-out 只追加 §8,上方条款 0-byte;基准 mb1-closed→ea1-closed 切换 + ea1-closed tag 归 EA1.3,agent 自主签署）
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
