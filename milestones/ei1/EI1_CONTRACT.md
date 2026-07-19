---
# 里程碑契约(14 §1 四要素;ei1 = 引擎集成期,承 TEMPLATE_CONTRACT.md 体例)
# 状态:gated——激活 gated on G3 close-out + owner 立项确认(G3_CONTRACT §7 ④ / G-G3-8,MB1 §0 先例);未获确认前零实现 PR、零共享编号消费。
contract: EI1
title: EI1 期——引擎集成期:UC-05 最小 RHI + render graph 核心(U5 旗舰用例,EA1 显式留下期项)+ RD-009 `#[export(c)]` C ABI 导出 codegen 与内建头文件生成(D-113)
status: active            # gated → active(2026-07-19 激活:G3 close-out g3-closed 签署 + owner 立项确认,G-EI1-0 过门,§7 v1.1)→ closed(close-out 只追加 §8,上方条款 0-byte)
version: v1.0
date: 2026-07-18
timebox: "激活后约 5–7 周(主线 EI1.1~EI1.5 串行,见 EI1_PLAN.md;周为相对刻度,非日历承诺;gated 期不计时)"
rfc_required: RFC-0014    # earmark(owner 2026-07-18 双轨分配,G3_CONTRACT §7 v1.1 编号更正确认:RFC-0014 = EI1 earmark;RFC-0013 = G3 单伞形)。`#[export(c)]` DLL 导出表 codegen + 内建头文件生成触 FFI ABI codegen 面(AGENTS 硬规则 5;RD-009 backfill_condition 明记「按 10 §3 判档,FFI ABI codegen 触硬规则 5 则 Full RFC,向上取严」;spec/engine_integration.md 头注升档红线字面依据);UC-05 RHI 库面随 RFC-0014 单 RFC 双面承载(§7 ⑦ 预记录 Q-B,镜像 RFC-0010 对 UC-07 角色)。**gated 期零消费:不落 RFC 文件、不动 rfcs/README 台账**;激活确认后起草 Draft→对抗性评审(D-409)→Approved 先于任何实现 PR
upstream_docs:
  - "milestones/g3/G3_CONTRACT.md §7 ①④(双轨立项 + EI1 顺手立契约脚手架裁决)+ §7 v1.1(编号更正:EI1 earmark = RFC-0014 / RXS-0250~0269 / CI 步骤 71~75,G3 溢出自 RXS-0270 顺续)+ G-G3-8(本契约的合入门)"
  - "02 §2 U5(:53-59 旗舰用例原文「用 Rurix 实现一个最小 RHI + render graph 核心…『同一组不变量,类型系统拦截 vs 计数器事后观测』的对比报告」+ 采纳判据 = C ABI FFI 成熟 + 增量 check <5s)+ §4 映射行(:89)"
  - "06 §8.3(:149-151「affine 资源 + 生命周期 brand + …C ABI(嵌入现存引擎)…UC-05 对照实验是此承诺的验收形式」;render graph/ECS「它们是库」——不进语言)"
  - "05 §11 + 13 D-113(FFI 战略 = `#[export(c)]` + 编译器内建头文件生成,cbindgen 角色内置化 P-11)"
  - "registry/deferred.json RD-009(激活后兑现对象:backfill_condition 触发面——UC-05 C ABI 导出即『需经 rurixc `#[export(c)]` 自动产 DLL 导出表 + 单一事实源内建头文件生成』的硬需求;.rx 代码今天无任何 DLL 出口,手写 extern \"C\" 回退仅对 Rust crate 有效。**gated 期不承接:owner_milestone 维持 MS1,激活小 PR 内翻转**)"
  - "spec/interop.md RXS-0125(手写 C ABI 基座,语义 0-byte 只增)+ spec/engine_integration.md RXS-0149(头↔ABI 一致性守卫 + 步骤 43,激活后共存升级对象)+ src/rurix-engine G1.3 资产(MR-0002 血统,0-byte 冻结;harness/engine_host.cpp 为引擎嵌入宿主升级母本)"
  - "spec/host_orchestration.md RXS-0189~0199(std::gpu 底座——RHI 类型面为其薄映射)+ rfcs/0009/0010(单源宿主编排 + 全 .rx 应用先例)"
  - "milestones/ea1/EA1_CONTRACT.md out_of_scope uc05_minimal_rhi(:27 显式留下期;UC-05 无 RD 条目,承接依据即该行)"
  - "07 §6(增量 check <5s 行业线)+ §9(tooling-server 热重查——「增量」的诚实兑现口径)"
  - "14 §1 §3 §4 §5(契约 / 预算零占位 / deferred / 证据分级)/ 10 §3(变更三档)§9.5(编号永不复用)/ agents/AGENTS.md(硬规则十条)"
in_scope:
  - export_c_codegen         # (激活后 EI1.2)`#[export(c)]` 导出表 codegen:属性合法性(仅 host pub fn;C 兼容签名子集 v1 = 标量+裸指针+unit)+ 保名不 mangle + dllexport 发射 + `--emit=dll` cdylib 通道(免 main,link.exe /DLL + import lib)→ Full RFC(RFC-0014 earmark)前置,条款自 RXS-0250 earmark 段续号
  - builtin_header_gen       # (激活后 EI1.2)编译器内建头文件生成(cbindgen 内置化,D-113/P-11):从导出集确定性产 .h(LF/无时间戳/两次逐字节一致),单一事实源 = typeck 的 C 映射;守卫 = CI 再生成逐字节比对(漂移即红)→ 同 RFC-0014;§7 ⑦ 预记录 Q-D:与导出表 codegen 两面全做,不分段
  - uc05_minimal_rhi         # (激活后 EI1.3/EI1.4)UC-05 最小 RHI:apps/uc05-rhi 全 .rx(Rhi/Queue/Res/Pass 四件薄映射 std::gpu lang items,主语言判据沿 MS1 最严口径)+ in-EXE demo + rurix_rhi.dll 经 export(c) 导出被 engine_host v2(C++/D3D12)链接执行 → RFC-0014 Part B 承载(预记录 Q-A/Q-B)
  - uc05_render_graph_probe  # (激活后 EI1.3/EI1.5)render graph 核心 + 「同一组不变量」对照报告:节点=compute pass/边=资源访问序推导/依赖环构建期确定性拒绝/1-submit typestate;I1~I10 不变量矩阵(evidence md+json)+ 100% 编译期/构建期拦截判据 → 入验收门(预记录 Q-C,owner 2026-07-18 勾选;06:151 字面依据);对照物 = 规划文档引文纸面对照(documented_historical——上一项目 Python 代码与 H01~H07 均不在仓库,已核实;严禁杜撰 Python 侧数字)
out_of_scope:
  - g3_industrial_rendering   # G3 工业渲染期轨道全部内容(RFC-0013 伞形/RXS-0220~0249/步骤 61~70):双轨互不侵入;本契约激活本身 gated on 其 close-out
  - render_graph_into_language # render graph/ECS 进语言:06:151「它们是库」——RHI/graph 全为库面 + std::gpu 薄映射,零新语言机制默认;语言硬缺口出现登记 RD 按 10 §3 判档,不静默扩语言
  - rhi_on_vulkan             # .rx 单源 Vulkan 通道:rxrt C ABI 为 CUDA-only,artifacts v2 Spirv 变体 = RD-031 open;首期 CUDA 底座 + rxp_*(D3D12 shim)present;Vulkan RHI 硬需求另议(注:G3 期将建 vk graphics descriptor 底座,激活时复评本条是否收窄)
  - export_c_extended_signatures # C 兼容签名子集 v1 之外:repr(C) struct 按值/回调函数指针/数组按值等——RFC-0014 §8 锁边界,超界登 RD
  - abi_stability_promise     # 不冻结 `#[export(c)]` 产物为语言级稳定 ABI(维持 RXS-0180 L3 口径);ABI 稳定承诺另期另裁
  - record_derive             # `Record` derive(05 §2.2 意向,rurixc 零实现、spec 零条款):显式不进本期;硬需求出现按 10 §3 判档另立
  - python_pyd_integration    # export(c) 与 pyd/Python 联动:UC-01 既有通道维持,本期不动
  - ea1_closeout_matters      # EA1 收口面不代管:冷启动 A 段/G-MB1-6 AMD 尾门/基准切换归各自轨道;EI1 对 milestones/ea1/** PR 0-byte
  - production_adoption_claim # 「引擎/外部采纳/用户数」维度:显式 carve-out(沿 EA1/MS1 先例)——验收全锚定自方可控工程物(自建 harness 嵌入真跑/不变量矩阵/measured bench),不宣称外部引擎接入
deferred_refs: [RD-009]      # **引用不承接**(gated):RD-009 open,owner_milestone 维持 MS1;激活小 PR 内翻转 MS1→EI1 + history 承接留痕(共享面 gated 期零消费)。激活后执行期新 RD 不预留号,按 main 合并序取(双轨纪律;RD-016/028 跳号永不复用,10 §9.5)
deliverables:
  - id: D-EI1-1
    name: gated 契约四件套(本契约 + EI1_PLAN + CI_GATES + ei1_budget.json;= G3 侧交付物 D-G3-2 的兑现体,G-G3-8 门)——零实现、零共享编号消费、对 milestones/ei1/ 之外全 0-byte
  - id: D-EI1-2
    name: (激活后)激活小 PR:§0/status 翻转 active + owner 立项确认留痕(§7 追加行)+ RD-009 承接(deferred vNext)+ number_ledger reserved_in_flight[EI1] 登记/校准 + earmark 复核(以届时台账实际为准)
  - id: D-EI1-3
    name: (激活后 EI1.1/EI1.2)RFC-0014(Draft→对抗性评审→Approved,先于实现)+ `#[export(c)]` 导出表 codegen + `--emit=dll` + 内建头文件生成 + 条款 RXS-0250 段前段 + CI 步骤 71 红绿
  - id: D-EI1-4
    name: (激活后 EI1.3)UC-05 最小 RHI + render graph 核心(apps/uc05-rhi 全 .rx,零 .rs)+ in-EXE demo device 真跑 + 不变量 reject 语料矩阵 + CI 步骤 72/73
  - id: D-EI1-5
    name: (激活后 EI1.4)引擎嵌入——rurix_rhi.dll(export(c) 产)+ 生成头被 engine_host v2(C++/D3D12,LUID 匹配)链接执行 ≥1 graph compute pass 数值对照 + CI 步骤 74
  - id: D-EI1-6
    name: (激活后 EI1.5)对照报告(evidence/uc05_invariant_matrix.json + uc05_comparison_report.md + schema)+ ei1.bench.uc05_check_{cold,warm}_ms measured 回填 + CI 步骤 75 + close-out
acceptance_gates:
  - id: G-EI1-0
    check: "激活门(gated 期唯一活门):G3 close-out(g3-closed 签署)+ owner 立项确认 → 激活小 PR(D-EI1-2:status gated→active + §7 确认留痕 + RD-009 承接 + ledger 登记 + earmark 复核——RFC-0014/RXS-0250~0269/步骤 71~75 以届时 number_ledger 实际为准,若期间被占用(不应发生,G3_CONTRACT §7 v1.1 已固化 earmark)则以台账现状续号并留痕)。**未过本门前:零实现 PR、零 RFC 文件、零条款头、零 workflow 步骤、registry/rfcs/spec 全 0-byte**"
  - id: G-EI1-1
    check: "(激活后)治理与条款门:RFC-0014 Approved 合入先于任何实现 PR(10 §3 失败测试先行:步骤 71~75 脚本与 export(c) codegen/`--emit=dll`/RHI 代码在 RFC 合入时点 main 上不存在 = RED);条款 RXS-0250 段续号体(FLS 体例,严禁 UB 节)与每条 ≥1 `//@ spec:` 锚定同 PR、commit 序条款在前;trace_matrix --check 维持全锚定;stable 快照因条款增长同 PR 重 bless(bless_log 同 diff,步骤 49 硬红不可分 PR);check_number_ledger PASS"
  - id: G-EI1-2
    check: "(激活后)export(c) 红绿门(CI 步骤 71):.rx fixture → `--emit=dll` 产 DLL + import lib + 生成头;C 调用方(cl.exe)编译链接**真跑数值对照**(device 段,RURIX_REQUIRE_REAL 纪律,防降级硬门——mock/dry-run 不得替代);头生成幂等(同源两次逐字节一致);RED 三路各自独立见证:① 非 C 兼容签名 → 编译期拒(诊断码断言)② 篡改入库头一字节 → 再生成 byte-diff 红 ③ 导出名冲突 → 编译期拒;内建 red_self_test;run URL + evidence 归档 §8"
  - id: G-EI1-3
    check: "(激活后)RHI + render graph 语义门(CI 步骤 72/73):apps/uc05-rhi 零 .rs 审计过(主语言判据,MS1 最严口径先例;硬缺口登记 RD 判档,**不静默降级 .rs**);graph ≥3 pass device 真跑数值对照 + 同机两跑逐字节确定;I1~I8 不变量 100% 编译期/构建期拦截——conformance/uc05 reject 语料矩阵逐条断言期望诊断,漏拦即红;run URL 归档 §8"
  - id: G-EI1-4
    check: "(激活后)引擎嵌入门(CI 步骤 74):rurix_rhi.dll(export(c) 产,非手写 extern \"C\")+ 编译器生成头(CI 再生成逐字节比对,非手写)被 engine_host v2(自建 C++/D3D12 宿主,LUID 匹配,G1.3 母本升级)链接执行 ≥1 个 graph compute pass 数值对照真跑(RURIX_REQUIRE_REAL 硬红纪律);src/rurix-engine G1.3 既有资产(EXPORTED_C_ABI 三符号/手写头/RXS-0149 守卫)0-byte;run URL 归档 §8"
  - id: G-EI1-5
    check: "(激活后)对照报告与采纳判据门(CI 步骤 75 + evidence 面):evidence/uc05_invariant_matrix.json 经 schema 校验 + 矩阵↔reject 语料↔uc05_comparison_report.md 一致性机核(防 YAML-only);Rurix 侧证据全 measured/ci_checked、Python 对照侧全 documented_historical 带规划文档引文行号(**零杜撰数字**,硬规则 3;上一项目代码不在仓库为已核事实,报告显式声明纸面对照口径);采纳判据 ei1.bench.uc05_check_cold_ms / uc05_check_warm_ms(冷全检 + tooling-server 热重查双口径,07 §6 阈 5000ms)以 measured_local 回填——evidence 面不进 CI 硬门(计时波动,EA1 冷启动先例),SKIP 不充绿"
  - id: G-EI1-6
    check: "(激活后)性能与收口:close-out budget_eval --strict 全局零 estimated;RD-009 处置留痕(EI1.2 落地后关闭,或收窄余项另立 RD——号按 main 合并序取);执行期新 RD/SG 处置;close-out 全量回归冻结(cargo test / trace / snapshot / bilingual / guardrails 真实输出追加 §8)+ 基准切换按 main 合并序串行化(以收口时 main 现状基准为底,双基准 advisory 复核)+ annotated ei1-closed tag(不匹配 release.yml 收窄触发器;agent 自主签署)"
guardrails:
  - "**gated 期总红线**:本包仅 milestones/ei1/ 四件;registry/* / rfcs/* / spec/* / ci/* / .github/* / 00–14 / 13_DECISION_LOG / spike_gating 全 0-byte;零共享编号消费(RFC/RXS/RD/CI 步骤/U/RX 码全不取;earmark 记载在 G3_CONTRACT §7 v1.1 + number_ledger,非本包消费);激活前本契约唯一合法后续改动 = G-EI1-0 激活小 PR 与 §7 追加行"
  - "milestones/m0~ea1 的 measured_local 既有预算条目 git diff 0-byte;ei1_budget.json 经 *_budget.json glob 自动纳入 + 命名空间强制前缀 ei1.(14 §3);counter/entries **不预造**(gated 期恒空;激活后登记与 ci/budget_eval.py evaluator 分支同实现 PR 落);**永不立引擎采纳/下载量/用户数类条目**"
  - "milestones/m0~mb1 的 *_CONTRACT.md(均 closed)只追加不修改(check_closed_contracts,glob 已泛化);EA1_CONTRACT(active)与 milestones/g3/**(G3 轨)对 EI1 侧 PR 0-byte——双轨互不侵入红线;本契约翻 closed 后自动纳入字节守卫"
  - "**编号双轨纪律**:RFC-0013 / RXS-0220~0249 / CI 步骤 61~70 = G3;RFC-0014 / RXS-0250~0269 / 步骤 71~75 = EI1 earmark(G3_CONTRACT §7 v1.1 固化,G3 溢出自 RXS-0270 顺续)——gated 期 EI1 零消费,激活时经 G-EI1-0 复核兑现;RD-/U-/RX- 码按 main 合并序取号不预留;**严禁把双轨 earmark 写进 number_ledger `shadow_reserved`**(该字段专记 off-tree 永久 burned 号,写入将致 check_number_ledger 查 2a/2b 误红)"
  - "(激活后)registry/error_codes.json 可加不可改;新 RX 码(拟 ≤4:属性误用/签名不兼容/空导出集/DLL 链接失败)按合并时点 main 段位续号,en+zh messages 成对;graph 构建期错误走库层状态值零新码(spec/imageio.md 先例)"
  - "(激活后)evidence/ 只增不删不改;对照报告 Python 侧引文须带 文件:行号,documented_historical 字面如实分级"
  - "(激活后)src/rurix-interop RXS-0125 手写 extern \"C\" 语义 0-byte 只增;RXS-0149 守卫(CI 步骤 43 host 段)在 export(c) 落地前维持全绿,落地时共存判据升级与条款同 PR(手写路冻结覆盖 Rust crate 出口,生成路覆盖 .rx 出口);src/rurix-engine G1.3 资产 0-byte(harness 升级 = 新文件/新入口,不改既有三符号面)"
  - "仓库 LF byte-exact(* -text):新文件 LF + 尾换行,禁 Python 文本模式写文件;提交前逐文件字节核 CR + 尾字节(git numstat + 二进制读,禁 grep $'\\r')"
  - "spec 修订表表头维持「版本」列名,数据行避「版本」子串(用「版号」)、忌「日期」子串入 bless 数据行;本契约既有条款 0-byte,close-out 只追加 §8;status 翻转(gated→active 经 G-EI1-0)/ 基准切换 / ei1-closed tag / RD·SG 处置由 agent 自主签署"
  - "guardrail 回退基准以合入时 main 现状为准(现 mb1-closed;PR 路径以 GITHUB_BASE_REF 为准);EI1 close-out 基准切换按 main 合并序串行化(不预 claim 下一默认)"
  - "(激活后)UC-05 kernel 保持编译期有界简单核(saxpy/scale/reduce 级),避开深弹射循环形态(RD-027 毒径警示,G3.1 归因结论届时并读);RURIX_REQUIRE_REAL 纪律贯穿 device 段(mock/SKIP 不充绿)"
---

# EI1 契约 — 引擎集成期(gated)

> 所属:[../../02_USERS_AND_USE_CASES.md](../../02_USERS_AND_USE_CASES.md) §2 U5 / §4 UC-05 + [../../06_GPU_GRAPHICS_PROGRAMMING_MODEL.md](../../06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) §8.3 / 契约机制见 [../../14_ENGINEERING_DISCIPLINE.md](../../14_ENGINEERING_DISCIPLINE.md) §1 / 本契约合入门 = G3 侧 [../g3/G3_CONTRACT.md](../g3/G3_CONTRACT.md) G-G3-8。
> 规范先行延续(AGENTS.md 硬规则第 7 条):语义面 PR 必须引用 RXS-#### 条款号;缺条款先补 spec,条款 commit 先于实现 commit(激活后适用;gated 期零语义面 PR)。
> 粒度:**单 EI1 阶段契约**:一份契约覆盖 EI1 期,激活后 EI1.1~EI1.5 主线分解见 [EI1_PLAN.md](EI1_PLAN.md)。
> **定位口径:EI1 激活后兑现「Rurix 能被嵌进 C++ 工程承担 RHI 角色」这一工程事实,不宣称「引擎采纳」这一社会事实。**现状:.rx 代码没有任何 DLL 出口——rurixc host 只产 EXE,手写 `extern "C"` 回退(RXS-0125/0149)仅对 Rust crate 有效;`#[export(c)]` 自 G1.3 起八连顺延(parser 桩 parsed-but-inert);UC-05 被 EA1 显式砍掉留下期(EA1_CONTRACT :27)。EI1 把「.rx 单源写最小 RHI + render graph,经 `#[export(c)]` 导出为 DLL + 编译器生成头,被自建 C++/D3D12 宿主链接真跑,并交『同一组不变量:类型系统拦截 vs 计数器事后观测』对照报告」做成 measured 工程事实。「外部采纳」维度显式 carve-out(out_of_scope)。
> **脚手架口径:本契约为 gated 结构件(G3 交付物 D-G3-2 的兑现体),不实现任何语义面、不落条款、不落 RFC 文件、不打 tag、零共享编号消费;§8 close-out 开工时为空。**

---

## 0. 治理闸口(读在最前 — 本契约区别于既往 active 开工契约之处)

EI1 为 **gated 契约**(owner 2026-07-18 双轨终裁,MB1 §0 先例):

- **激活前置 = G3 close-out(g3-closed 签署)+ owner 立项确认**(G3_CONTRACT §7 ④「EI1 顺手立契约脚手架…激活 gated on G3 close-out + 立项确认」;本会话 owner 终裁确认随 G3 包)。**未获确认前:零实现 PR、零 RFC 文件、零条款头、零 workflow 步骤、共享编号零消费。**
- **编号 earmark 已固化但未消费**:RFC-0014 / RXS-0250~0269 / CI 步骤 71~75 = EI1 earmark(G3_CONTRACT §7 v1.1 编号更正 + number_ledger 记载;G3 溢出自 RXS-0270 顺续)——激活时经 G-EI1-0 以届时台账实际为准复核兑现,本包不 claim 不登记。
- **激活动作 = 激活小 PR**(D-EI1-2):status gated→active + §7 确认留痕 + RD-009 承接(MS1→EI1)+ number_ledger reserved_in_flight[EI1] 登记 + earmark 复核;此后 EI1.1~EI1.5 按 [EI1_PLAN.md](EI1_PLAN.md) 推进。
- 与仓库默认治理(D-406 v2.0 agent 完全自主)的关系:gating 来自 owner 双轨裁决(资源串行化:单 runner/单 owner 评审带宽,G3 五面已是重载),非红线类闸口;激活确认为 owner 动作,激活后回归 agent 完全自主范式。

## 1. 目标(激活后)

EI1 期结束时项目获得:① `#[export(c)]` 接通(RD-009 兑现)——rurixc 从 .rx 源自动产 DLL 导出表(`--emit=dll` cdylib 通道)+ 编译器内建单一事实源头文件生成(RFC-0014,D-113 完整兑现);② UC-05 最小 RHI + render graph 核心——全 .rx 库(Rhi/Queue/Res/Pass 薄映射 std::gpu)+ 编译期有界 graph + 1-submit typestate,device 真跑;③ 引擎嵌入可验收——`rurix_rhi.dll` + 生成头被自建 C++/D3D12 宿主(engine_host v2)链接执行 graph compute pass;④ 「同一组不变量」对照报告——I1~I10 矩阵(类型系统编译期拦截 vs 上一项目 Python 计数器事后观测,documented_historical 纸面对照)+ 采纳判据(C ABI 成熟 + 增量 check <5s)measured。

## 2. 范围

### 2.1 in-scope(全部激活后执行)

| 项 | 说明 | gating | 对应交付物 |
|---|---|---|---|
| export_c_codegen | 导出表 codegen + `--emit=dll` 通道(属性合法性/签名子集 v1/保名/dllexport) | G-EI1-0 → Full RFC(RFC-0014 earmark) | D-EI1-3 |
| builtin_header_gen | 内建头文件生成(确定性,单一事实源;预记录 Q-D 全做) | 同 RFC-0014 | D-EI1-3 |
| uc05_minimal_rhi | 全 .rx 最小 RHI + in-EXE demo + DLL 嵌入 engine_host v2 | RFC-0014 Part B(预记录 Q-A/Q-B) | D-EI1-4 / D-EI1-5 |
| uc05_render_graph_probe | render graph 核心 + 不变量对照报告(预记录 Q-C 入门) | RFC-0014 Part B | D-EI1-4 / D-EI1-6 |

### 2.2 out-of-scope(显式排除)

见 YAML 头 `out_of_scope` 字段逐项(g3_industrial_rendering / render_graph_into_language / rhi_on_vulkan / export_c_extended_signatures / abi_stability_promise / record_derive / python_pyd_integration / ea1_closeout_matters / production_adoption_claim);11 §2 红线不触碰。

## 3. 交付物清单

| ID | 交付物 | 形态 | 完成判据 |
|---|---|---|---|
| D-EI1-1 | gated 契约四件套(本 PR) | milestones/ei1/ 四件,零共享编号消费 | G-G3-8(G3 侧门) |
| D-EI1-2 | 激活小 PR | status 翻转 + 确认留痕 + RD-009 承接 + ledger 登记 | G-EI1-0 |
| D-EI1-3 | RFC-0014 + export(c) 接通 | rfcs/0014 + 条款前段 + rurixc 扩展 + 步骤 71 | G-EI1-1 / G-EI1-2 |
| D-EI1-4 | UC-05 RHI + graph 核心 | apps/uc05-rhi 全 .rx + reject 矩阵 + 步骤 72/73 | G-EI1-3 |
| D-EI1-5 | 引擎嵌入 | rurix_rhi.dll + 生成头 + engine_host v2 + 步骤 74 | G-EI1-4 |
| D-EI1-6 | 对照报告 + bench + close-out | evidence md+json+schema + ei1.bench + 步骤 75 | G-EI1-5 / G-EI1-6 |

## 4. 验收门(完整版,YAML 头为可提取摘要)

见 YAML 头 `acceptance_gates` 字段 G-EI1-0 ~ G-EI1-6。要点:
- **G-EI1-0(激活门,gated 期唯一活门)**:G3 close-out + owner 立项确认 → 激活小 PR;未过前零实现零消费。
- **G-EI1-1(治理条款门)**:RFC-0014 Approved 前置 + 失败测试先行 + 条款先行 + 同 PR 重 bless。
- **G-EI1-2(export 红绿门)**:.rx→DLL+生成头→C 宿主真跑;头幂等;三路 RED。
- **G-EI1-3(语义门)**:零 .rs 审计 + graph device 真跑确定性 + I1~I8 不变量 100% 拦截,漏拦即红。
- **G-EI1-4(嵌入门)**:export(c) 产物(非手写)被 C++/D3D12 宿主链接真跑;G1.3 资产 0-byte。
- **G-EI1-5(报告与判据门)**:矩阵↔语料↔报告一致性机核;documented_historical 零杜撰;<5s 双口径 measured。
- **G-EI1-6(性能与收口)**:--strict 零 estimated + RD-009 处置 + 基准切换按 main 合并序串行化 + ei1-closed tag。

## 5. Guardrails(字节级,机器核对)

见 YAML 头 `guardrails` 字段(首条为 gated 期总红线)。核对方式:`py -3 ci/check_guardrails.py`(基准以合入时 main 现状为准,现 mb1-closed;PR 路径以 `GITHUB_BASE_REF` 为准)。

## 6. Deferred 引用

| 编号 | 内容摘要 | 承接 |
|---|---|---|
| RD-009 | `#[export(c)]` C ABI 导出属性 + 编译器内建头文件生成(D-113)codegen 实现 | **引用不承接**(gated):open,owner_milestone 维持 MS1;激活小 PR 内翻转 MS1→EI1 = 激活后兑现对象(backfill_condition 触发:UC-05 C ABI 导出面硬需求;判档 Full RFC = RFC-0014 earmark;预记录 Q-D 两面全做);EI1.2 落地后 close-out 关闭或收窄余项另立 RD |

详情以 [../../registry/deferred.json](../../registry/deferred.json) 为唯一事实源,本表仅引用;**本包对该文件 0-byte**。

## 7. 修订记录 / 开工裁决留痕

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-07-18 | 初版契约固化(EI1 gated 契约脚手架 = G3 交付物 D-G3-2 兑现体)。**开工裁决留痕**(owner 白栀 2026-07-18 两会话裁决合流,agent 代录;13_DECISION_LOG 执行 PR 字节冻结不改,G2/EA1 先例):① **立项 = owner 拍板双轨**:G3 工业渲染期 ∥ EI1 引擎集成期(G3_CONTRACT §7 ①);合并纪律 = 脚手架 G3 先合、EI1 随后、共享面后合者 rebase。② **EI1 定位终裁 = gated 壳**:EI1 轨会话先按「立即激活」口径备包并经 owner 四项 AskUserQuestion 裁定;G3 轨会话同日落地「EI1 顺手立契约脚手架,激活 gated on G3 close-out + 立项确认」(G3_CONTRACT §7 ④);两裁决冲突经 EI1 轨会话如实呈报,owner 终裁**随 G3 包**——EI1 = gated 壳,零实现零共享编号消费,本会话四项裁定降为预记录(⑦)。③ **编号 earmark**(G3_CONTRACT §7 v1.1 编号更正,与本轨协调后落):G3 = RFC-0013 单伞形 / RXS-0220~0249 / 步骤 61~70;**EI1 = RFC-0014 / RXS-0250~0269 / 步骤 71~75(earmark,gated 期零消费,激活时经 G-EI1-0 复核兑现)**;G3 溢出自 RXS-0270 顺续;RD-/U-/RX- 按 main 合并序取号。④ **判档预记录**:`#[export(c)]` 导出表 codegen + 内建头文件生成 = FFI ABI codegen 触硬规则 5 → Full RFC(RD-009 backfill_condition + spec/engine_integration.md 头注字面依据);激活后 RFC 起草→对抗性评审(D-409)→Approved 先于实现。⑤ **命名 = milestones/ei1/(Engine Integration 1)**,namespace `ei1.`,收口 tag `ei1-closed`(不撞既有系,不匹配 release.yml 收窄触发器)。⑥ **RD-009 gated 处理**:本包引用不承接(owner_milestone 维持 MS1,registry 0-byte);激活小 PR 内承接翻转 + history 留痕。⑦ **预记录(owner 2026-07-18 本轨会话四项 AskUserQuestion + 两项拟裁,激活时立项确认的输入,届时复核后生效)**:Q-A 执行底座 = CUDA std::gpu 链(rxrt_* PTX)+ engine_host v2(C++/D3D12,G1.3 母本)嵌入侧,Vulkan 不进本期(激活时复评 G3 期 vk descriptor 底座是否改变此判)/ Q-B UC-05 RHI 库面随 RFC-0014 单 RFC 双面承载(镜像 RFC-0010)/ **Q-C 对照报告入验收门**(owner 勾选;G-EI1-5,documented_historical 纸面对照,零杜撰)/ **Q-D export(c) 导出表 + 内建头生成两面全做**(owner 勾选;D-113 完整兑现,RD-009 激活期可关)。⑧ **诚实边界**:达成表述 =「引擎集成工程闭环落地」;「引擎/外部采纳」carve-out 不宣称;对照报告 Python 侧 = documented_historical 纸面引文,不假装可复跑 A/B(上一项目代码与 H01~H07 不在仓库为已核事实);gated 期不计时不宣称进度。**G-EI1-0 激活确认为 owner 动作;激活后 close-out 判定 / 基准切换 / ei1-closed tag / RD·SG 处置回归 agent 自主签署** |
| v1.1 | 2026-07-19 | **激活(G-EI1-0 过门,gated→active;激活小 PR = D-EI1-2 兑现)**。① **双条件齐**:G3 close-out 已签署(G3_CONTRACT §8.1 终审 + `g3-closed` annotated tag @ main `1cf81350`,基准已切 g3-closed,PR #179)+ **owner 立项确认**(2026-07-19 本会话 AskUserQuestion,owner 选定「激活 EI1 引擎集成期」,agent 代录非代签)。② **earmark 复核(以台账实际为准,全部无撞号)**:RFC-0014 空闲 ✓(rfcs/ 最高 0013,G3 单伞形已固化);RXS-0250~0269 空闲 ✓(G3 恰用满 0220~0249,ledger next_free=250);CI 步骤 71~75 空闲 ✓(G3 落 61~69;步骤 70 = G3 earmark 内 showcase 判档未落,维持 G3 段留空,EI1 不占用);RD/U/RX 按 main 合并序现状续(RD next 35 / U next 31 / RX6031+·RX7023+)。③ **§7 ⑦ 预记录四项复核生效**:Q-A 执行底座维持 CUDA std::gpu 链 + engine_host v2 嵌入侧——复评注:G3 期落地的 vk graphics descriptor/mesh/RT 底座为**图形着色面**,UC-05 RHI 首期为 **compute pass graph**,CUDA 底座判定不变,`rhi_on_vulkan` out_of_scope 维持(硬需求出现另议);Q-B UC-05 随 RFC-0014 单 RFC 双面承载生效;Q-C 对照报告入验收门生效(G-EI1-5);Q-D export(c) 两面全做生效(RD-009 激活期可关)。④ **RD-009 承接**:deferred.json owner_milestone MS1→EI1 + history 承接行(v1.65,同 PR);gated 期零消费纪律核验通过(rfcs/README/registry/spec 在激活前对 EI1 全 0-byte)。⑤ **执行编排(承 G3 已验证范式)**:EI1.1 RFC-0014(Draft→跨模型对抗性评审→Approved 先于实现)→ EI1.2 export(c)+头生成(步骤 71)→ EI1.3 UC-05 RHI+graph(步骤 72/73)→ EI1.4 引擎嵌入(步骤 74)→ EI1.5 对照报告+close-out(步骤 75);agent worktree 起草编译面 + 主循环 device 真跑迭代 + PR 合一等一;fmt 第一道/feature 矩阵双验/逐路径 add(G3 期 CI 坑清单纪律)。**激活后 agent 完全自主(D-406 v2.0)** |

---

## 8. Close-out(只追加区 — 开工时为空)

<!-- G-EI1-0 激活留痕(立项确认 + 激活小 PR 指针)、激活后验收记录、guardrail 核对输出、EI1.1~EI1.5 留痕(RFC-0014 Approved / 步骤 71~75 run URL / export(c) 红绿 / RHI demo / engine_host v2 嵌入真跑 / 对照报告 / bench 回填)、RD-009 处置留痕、SG 复评结论追加于此;上方条款 0-byte 修改。EI1 close-out 关闭判定 / 基准切换(按 main 合并序串行化)/ ei1-closed tag / RD·SG 处置由 agent 自主签署兑现。 -->
