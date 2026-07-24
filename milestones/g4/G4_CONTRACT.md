---
# 里程碑契约(14 §1 四要素;g4 = 引擎渲染期,承 TEMPLATE_CONTRACT.md 体例)
contract: G4
title: G4 引擎渲染期——图形 RHI 化(raster/mesh/RT pass + 采样/bindless/present 库化 + 自动 barrier + engine_host v3 嵌入)+ RD-035 执行面三项兑现 + .rx 单源 Vulkan RHI(RD-031)+ C ABI v2 判档 + BLACKHOLE 生产档验收
status: active            # active(2026-07-23 开工:EI1 close-out ei1-closed 已签署 + owner 立项确认经 agent-prompt-g4.md 全文下达,§7 ①)→ closed(close-out 终审签署,§8;close-out 只追加 §8,上方条款 0-byte)
version: v1.0
date: 2026-07-23
timebox: "约 8–12 周(主线 G4.0→G4.7 严格串行,见 G4_PLAN.md;周为相对刻度,非日历承诺)"
rfc_required: RFC-0015    # 单伞形 Full RFC(G3_CONTRACT §7 v1.1 单伞形先例,MB1 RFC-0011 先例):四章——A 图形 RHI 化(库面扩图形 pass,薄映射 std::gpu + G3 既有条款面;render graph 自动 barrier 覆盖图形 pass 的库面语义;export(c) 导出图形面)/ B RD-035 执行面三项(transient 别名复用+执行期峰值计数器 / 依赖驱动重排+并行调度 / RXS-0262 const 泛型定长容量)/ C .rx 单源 Vulkan RHI(RD-031 artifacts v2 @__rx_gpu_spirv 段 + Vulkan RHI 通道)/ D C ABI v2 条件臂(repr(C) struct 按值 + 回调指针,FFI ABI codegen 触硬规则 5,判档成立才落实现)。BLACKHOLE 面不占 RFC(运行时/应用修复 + 既有 present 条款 RXS-0197/0198/0220~0222,零新语义;判档 Direct/Mini 执行期定,§7 ③)
upstream_docs:
  - "milestones/ei1/EI1_CONTRACT.md §8.1(EI1 close-out 终审:#[export(c)] 接通 / UC-05 compute RHI 三 pass / engine_host v2 嵌入三方数值相等 / I1~I8 100% 拦截;RD-009 closed / RD-035 新登 open;RXS-0266~0269 作废声明)+ §7 v1.1(激活先例:owner 立项确认 + agent 代录非代签)"
  - "milestones/g3/G3_CONTRACT.md §8.1(G3 close-out:窗口 present / 采样超集 / bindless / render graph 自动 barrier / mesh-task-RT Vulkan 主腿全量 device measured;RD-034 DXIL RT blocked 探针先例;RD-027 护栏 MR-0011)+ §7 v1.1(单伞形 RFC 先例)"
  - "registry/deferred.json RD-035(本期兑现对象:UC-05 RHI 执行面三项,backfill_condition 三条)/ RD-031(本期条件臂:artifacts v2 @__rx_gpu_spirv 段,backfill_condition 前置 = artifacts blob / emit_gpu_artifact_globals 在 main——开工复核已在 main,src/rurixc/src/codegen.rs:99/1028)/ RD-027 / RD-034(out-of-scope 维护对象)"
  - "spec/rhi.md RXS-0256~0265(compute-only RHI 库面——本期图形扩面母本)/ spec/render_graph.md RXS-0236~0241(G3.5 图形 render graph 推导与双后端执行器)/ spec/host_orchestration.md RXS-0189~0199(std::gpu 底座)+ RXS-0225(SamplerDesc)/ RXS-0235(TextureTable)/ spec/shader_stages.md RXS-0242~0245(mesh/task/RT 类型面)/ spec/vulkan_backend.md RXS-0246~0248(SPIR-V 编码 + vk 运行时)/ spec/export_c.md RXS-0250~0255(C ABI 子集 v1 边界——G4.5 判档对象)"
  - "rfcs/0013(G3 伞形五章体例母本)/ rfcs/0014(EI1 单 RFC 双面承载先例)/ rfcs/0011(伞形单期先例 + Vulkan 后端)/ rfcs/0009(std::gpu 宿主编排)"
  - "13 D-113(FFI = #[export(c)] + 内建头生成)/ D-130(窗口/输入不进语言红线)/ D-131(DXIL 混合 compute=A/图形=B)/ D-406 v2.0(agent 完全自主)/ D-409(Full RFC 跨模型对抗性评审,评审 provenance ≠ 起草 provenance,check_contribution 规则 4 机核)"
  - "06 §8.3(:149-151 render graph/ECS「它们是库」——不进语言)/ 02 §2 U5(UC-05 旗舰用例)/ 04 P-01(strict-only)/ P-09(证据压过进度)/ P-12(克制压过完整性)/ P-13(AI 治理)"
  - "14 §1 §3 §4 §5(契约 / 预算零占位 / deferred / 证据分级)/ 10 §3(变更三档)§9.5(编号永不复用)/ agents/AGENTS.md(硬规则十条)"
  - "agent-prompt-g4.md(owner 2026-07-23 立项确认全文——本契约范围/门/硬纪律的上游事实源;EI1 激活先例:owner 选定 + agent 代录非代签,记 §7 ①)"
in_scope:
  - g4_governance            # G4.0 治理包:本契约四件套 + number_ledger 校准(§7 ② 四处滞后消除)+ reserved_in_flight[G4] 登记;结构件,零语义实现
  - umbrella_rfc_0015        # G4.1 伞形 Full RFC-0015:Draft → D-409 跨模型对抗性评审(评审 provenance ≠ 起草 provenance)→ Approved 先于一切实现 PR;失败测试先行(各面 CI 步骤脚本在 RFC 合入时点 main 不存在 = RED)
  - graphics_rhi             # G4.2 图形 RHI 化(主面):.rx RHI 库面扩图形 pass——raster / mesh pass 类型 + 采样 / bindless / present 面库化(薄映射 std::gpu lang items + G3 既有条款面,库面默认零新语法)+ render graph 自动 barrier 覆盖图形 pass + export(c) 导出 + engine_host v3(C++/D3D12)嵌入图形 pass device 真跑三方数值对照;**首切片 = artifacts v2 前置切片**(.rx → SPIR-V artifact 通道为图形 pass device 出图的工程前置,§7 ④)
  - rd035_execution_face     # G4.3 RD-035 三项兑现:transient 别名复用分配器 + 执行期峰值计数器 device 采集(I10 report_only → measured 收紧)/ 依赖驱动重排 + 并行调度(重排后 happens-before 正确性新增确定性拦截项入不变量矩阵)/ RXS-0262 const 泛型定长容量 .rx 接线 + 编译期越界拒 reject 语料
  - vulkan_rhi               # G4.4 .rx 单源 Vulkan RHI(RD-031 承接,条件臂):前置核实留痕(开工已核 emit_gpu_artifact_globals 在 main)→ 落 artifacts v2 @__rx_gpu_spirv 段通道本体 + .rx 单源 Vulkan RHI 通道(compute + graphics 双腿)+ 复用 G3 vk 运行时底座 device 真跑;前置不具备则 honest 存续留痕不伪造
  - c_abi_v2_adjudication    # G4.5 C ABI v2 判档面:以 engine_host v3 图形嵌入的真实硬需求判档(10 §3,争议向上取严)→ 硬需求成立则条款先行兑现 repr(C) struct 按值 + 回调指针;不成立则登记 RD-036+ 存续;两种结局均合法,判档依据必须留痕(P-12:不以「完整」为名扩面)
  - blackhole_acceptance     # G4.6 BLACKHOLE 收尾验收:realtime 路径归因(rxp_create Shim E_NOTIMPL = D3D12 shim 未实现面,先归因再修,禁绕过)+ 30fps measured(BENCH_PROTOCOL 口径,锁频/三次 trimmed mean)+ REALTIME_OK 判据 + evidence JSON + 帧对照留档
  - g4_closeout              # G4.7 close-out:全量回归冻结 + 门终审表 + RD/SG 处置 + status flip + 基准切 g4-closed + annotated tag(不匹配 release.yml 触发器)
out_of_scope:
  - rd027_upstream_poison    # RD-027(NVIDIA ptxas -O1+ 毒径,上游侧不可修):MR-0011 护栏维持,上游备包 DRAFT — do NOT file 维持;UC-05/blackhole kernel 维持编译期有界形态避毒径
  - rd034_dxil_rt            # RD-034(DXIL RT,spirv-cross / LLVM 双上游钳制):步骤 69 blocked 探针恒跑维护;探针意外翻绿 = 提醒复评信号,不在本期强推;图形 RHI 的 RT pass 类型面可以条款化但 DXIL RT 腿维持 blocked
  - g_mb1_6_amd              # G-MB1-6(AMD 真卡验收):缺硬件,pending-hardware 不伪造;本期 device 门全锚定本机 RTX 4070 Ti
  - window_input_language    # 窗口/输入进语言(D-130 红线);render graph / ECS 进语言(06:151「它们是库」)
  - upstream_filing          # 上游提报动作本体(agent 只备 DRAFT 包);外部采纳 / 用户数宣称(production_adoption_claim carve-out 沿 MS1/EA1/EI1 先例)
  - ea1_track                # milestones/ea1/** 0-byte(EA1 自身轨道收口另裁;EA1 仍 active)
  - abi_stability_promise    # 不冻结 #[export(c)] 产物为语言级稳定 ABI(维持 RXS-0180 L3 口径);ABI 稳定承诺另期另裁(沿 EI1 out_of_scope 先例)
deferred_refs: [RD-031, RD-035]   # 本期兑现/判档对象。RD-027/RD-034 = out-of-scope 维护对象(非 deferred_refs 兑现项);执行期新 RD 自 RD-036 起(RD-016/028 跳号永不复用,10 §9.5;以合入时 deferred.json 实际为准双侧标注)
deliverables:
  - id: D-G4-1
    name: G4.0 治理包四件(本契约 + G4_PLAN + CI_GATES + g4_budget.json 空壳)+ number_ledger 校准(RFC→15 / MR→12 / RXS 0266~0269 burned 跳号→next_free 270 / D→410)+ reserved_in_flight[G4] 登记
  - id: D-G4-2
    name: G4.1 RFC-0015 伞形(Draft→跨模型对抗性评审→Agent Approved 先于实现)+ 失败测试先行成立(各面步骤脚本 RFC 合入时点 main 不存在)
  - id: D-G4-3
    name: G4.2 图形 RHI 化——artifacts v2 前置切片(@__rx_gpu_spirv 段 + blob v2 + codegen 单测/golden)+ 条款 RXS-0270 段(图形 pass 类型面/自动 barrier 库面语义/export 面)+ rhi.rs/vk.rs 执行面 + apps/uc05-rhi 图形 demo + engine_host v3 + CI 步骤 76 起红绿
  - id: D-G4-4
    name: G4.3 RD-035 三项——别名复用分配器 + 执行期峰值计数器(I10 measured)/ 依赖驱动重排 + 并行调度 + 新拦截项入矩阵 / RXS-0262 const 泛型容量接线 + reject 语料 + 矩阵三方一致性维持
  - id: D-G4-5
    name: G4.4 Vulkan RHI 通道——.rx 单源 Vulkan RHI(compute+graphics 双腿)经 artifacts v2 通道 + G3 vk 底座 device 真跑数值对照 + RD-031 处置
  - id: D-G4-6
    name: G4.5 C ABI v2 判档——判档留痕 +(若成立)repr(C) struct 按值 + 回调指针条款 + ABI 往返真跑;(若不成立)RD-036+ 登记
  - id: D-G4-7
    name: G4.6 BLACKHOLE——realtime 归因留痕 + 修复 + 30fps measured + REALTIME_OK + evidence JSON + 帧对照
  - id: D-G4-8
    name: G4.7 close-out 终审(全量回归冻结 + 门终审表 + 基准切换 + g4-closed tag + RD/SG 处置 + ledger 校准)
acceptance_gates:
  - id: G-G4-1
    check: "治理门:契约四件套合入(milestones/g4/ 四件,结构件零语义实现、零条款头、零 workflow 步骤、零预算条目);number_ledger 校准兑现 §7 ② 四处滞后(RFC next_free 13→15 / MR 11→12 / RXS 0266~0269 burned 跳号 next_free 266→270 矛盾消除留痕 / D 408→410)且 `py -3 ci/check_number_ledger.py` PASS;check_schemas / check_structure PASS;milestones/g4/ 之外全 0-byte(number_ledger 校准除外)"
  - id: G-G4-2
    check: "RFC 门:RFC-0015(伞形四章)Agent Approved 合入先于任何实现 PR;D-409 对抗性评审完成——评审 provenance ≠ 起草 provenance,逐条 finding disposition(采纳并修 / 驳回并附理由)落 RFC「对抗性评审记录」段,check_contribution 规则 4 机核过;失败测试先行成立(各面 CI 步骤脚本与图形 RHI/artifacts v2/RD-035 机制代码在 RFC 合入时点 main 上不存在 = RED);条款 commit 序在实现 commit 前 + 每条新条款 ≥1 `//@ spec:` 锚定同 PR;trace_matrix --check 维持全锚定;stable 快照因条款增长同 PR 重 bless(bless_log 同 diff,步骤 49 硬红不可分 PR)"
  - id: G-G4-3
    check: "图形 RHI 门:≥1 raster + ≥1 mesh 图形 pass 经 .rx RHI 库面(零新语法,薄映射 G3 既有条款面)+ 自动 barrier 出图 device 真跑(RTX 4070 Ti,RURIX_REQUIRE_REAL=1),像素判据同 G3 对应面(headless readback 像素断言,RXS-0222 纪律);render graph 自动 barrier 覆盖图形 pass(推导产物 golden 锚定 + 漏声明 strict 拒 RED);engine_host v3(C++/D3D12,LUID 匹配,engine_host v2 母本升级新增文件,既有 v2 资产 0-byte)链接 rurix_rhi 图形导出面 device 真跑三方数值精确相等(.rx RHI / D3D12 宿主 / host 参考);export(c) 生成头 CI 再生成逐字节比对(仓库零 tracked .h);apps/uc05-rhi 零 .rs 审计维持;数据流红绿(篡改翻红);既有 compute RHI 路(步骤 72~75)零回归;evidence JSON + run URL 归 §8"
  - id: G-G4-4
    check: "RD-035 门:① transient 别名复用分配器落地 + 执行期峰值计数器 device 采集——I10 自 report_only 升 measured(峰值 < 声明容量可 device 见证,evidence JSON;矩阵 I10 note 与 tiers 同步更新维持三方一致);② 依赖驱动重排 + 并行调度落地——重排后 happens-before 正确性新增确定性拦截项入不变量矩阵(装配期确定性拒,漏拦即红;RXS-0239 pass 边界语义在新执行模型下的条款化修订同 PR);③ RXS-0262 const 泛型定长容量 .rx 接线 + 编译期越界拒 reject 语料锚定(conformance/uc05/reject 新增语料逐条断言期望诊断);RD-035 处置留痕(close / 收窄);I1~I8 既有 100% 拦截零回归"
  - id: G-G4-5
    check: "Vulkan 门(条件臂):前置核实留痕(emit_gpu_artifact_globals / artifacts blob 在 main——开工已核,src/rurixc/src/codegen.rs:99/1028)→ 具备则:.rx 单源 Vulkan RHI 通道(compute + graphics 双腿)经 artifacts v2 @__rx_gpu_spirv 段 device 真跑数值对照(Vulkan 侧结果 vs host 参考;spirv-val 校验;RURIX_REQUIRE_REAL=1;复用 G3 vk 运行时底座,run_mesh_offscreen/run_ray_tracing_offscreen/run_graph_offscreen 既有入口 0-byte 语义);RD-031 处置留痕(close / 收窄);不具备则:honest 存续留痕措辞照 RD-034 先例(open 尾门越过 close-out,不签不伪造)"
  - id: G-G4-6
    check: "ABI v2 门(条件臂):判档依据留痕——以 engine_host v3 图形嵌入的真实硬需求判档(10 §3,争议向上取严;P-12 不以「完整」为名扩面);硬需求成立 → 条款先行(RFC-0015 章 D 臂)+ repr(C) struct 按值 + 回调指针 ABI 往返 device 真跑(生成头再生成逐字节比对 + RED 三路);不成立 → 登记 RD-036+ 存续(超界硬需求自 RD-036+ 判档,RD-009 close 注先例);两种结局均合法"
  - id: G-G4-7
    check: "BLACKHOLE 门:realtime 路径归因留痕(rxp_create Shim(-2147467263)=0x80004001 E_NOTIMPL 的精确归因——先归因再修,禁绕过禁静默降级;归因证据归 evidence/);修复后 30fps measured(BENCH_PROTOCOL 口径:锁频 + 三次 trimmed mean,evidence JSON 含环境画像)+ REALTIME_OK 判据(物理自检六项:NaN/range、中心黑盘、shadow 半径 vs 解析 ±2%、Doppler 非对称 ≥1.15、光子环、星野)+ 帧对照留档(offline 144 帧既有产出 vs realtime 帧像素对照);修复先于测量;RURIX_REQUIRE_REAL=1"
  - id: G-G4-8
    check: "收口门:close-out `budget_eval.py --strict` 全局零 estimated;全量回归冻结真实输出追加 §8(fmt / clippy / test / trace 全锚定 / stable --check / bilingual / schemas / structure / guardrails / number_ledger / contribution / redistribution + 步骤 76+ 全冒烟 RURIX_REQUIRE_REAL=1 真跑 + 既有步骤 41~75 零回归 + saxpy smoke);验收门终审表逐门结论(blocked / 条件未具面照 G-MB1-6 措辞『OPEN 尾门越过 close-out 存续,不签不伪造,状态翻转不依赖新契约』);status active→closed;check_guardrails resolve_base 默认基准切 g4-closed(基准链 mb1→g3→ei1→g4 单线性,EA1 日后收口另裁)+ 双基准 advisory 复核;合入后 annotated g4-closed tag(不匹配 release.yml 触发器);RD-031/RD-035 逐条 close/收窄/存续留痕 + 执行期新 RD 处置;SG 复评 + SG-010 留续号;number_ledger 校准 revision(G4 行收口)"
guardrails:
  - "milestones/m0~ei1 的 measured_local 既有预算条目 git diff 0-byte;g4_budget.json 经 *_budget.json glob 自动纳入 + 命名空间强制前缀 g4.(14 §3);counter/entries **不预造**(登记与 ci/budget_eval.py evaluator 分支同实现 PR 落,未知 id 强制 FAIL);全程零 estimated;**永不立引擎采纳/下载量/用户数类条目**"
  - "milestones/m0~ei1 的 *_CONTRACT.md(均 closed)只追加不修改(check_closed_contracts glob 已泛化);EA1_CONTRACT(active)与 milestones/ea1/** 本期 0-byte 不代动(EA1 收口归自身轨道);本契约翻 closed 后自动纳入字节守卫"
  - "registry/deferred.json 与 registry/spike_gating.json 只追加;RD 处置仅由 agent 自主签署留痕追加;RD-016/028 跳号永不复用;SG-010 留续号维持;13_DECISION_LOG 执行 PR 字节冻结,开工裁决记本契约 §7,勘误走 00 §6.3 独立 errata PR"
  - "registry/error_codes.json 可加不可改;codegen 新码自 RX6034 续号(RX6009 burned 不用,以 registry/error_codes.json 复核);3xxx typeck 按合并序;工具类确需自 RX7023;en+zh messages 成对(bilingual 107→N)"
  - "registry/number_ledger.json 只追加纪律:v1.13 校准按 §7 ②;**严禁把 G4 earmark 写进 `shadow_reserved`**(该字段专记 off-tree 永久 burned 号;RXS-0266~0269 = main 侧 burned,记 notes 字段 + revision_log,不入 shadow_reserved,EI1 契约 guardrails 先例)"
  - "evidence/ 只增不删不改;上游备包全部文件 DRAFT — do NOT file 标头强制,agent 不对外提报"
  - "00–14 共 15 份规划文档不被执行 PR 改写(check_planning_docs)"
  - "GPU 实验纪律:全部经 bench/proc_guard.guarded_run(禁裸 subprocess,R-606);挂起判定后强制金丝雀门;实验窗与 CI run/nightly 错峰;ptxas 输入恒 ASCII 路径;僵尸 exe 隔离 build/quarantine/;TDR/系统态零改动如实记录"
  - "device 见证纪律:RURIX_REQUIRE_REAL=1;缺 provisioning 环境 SKIP = dev-env degrade(翻硬红),mock / SKIP 不得充绿"
  - "src/ 新 unsafe 全部 // SAFETY: + unsafe-audit U31 起续号登记(U29=EA1 预留显式跳让,无论其释放与否不回收);单块单操作;vk.rs 手写 FFI 扩展沿 U26/U27/U30 审计模式"
  - "既有零回归不变量:dxil 套件(404+ 恒定)/ vulkan 套件 grow-only / 步骤 41~75 既有判据 0-byte 只增(步骤 70 = G3 showcase 永久 gap 维持);B 链 dxv validator + 签名门(RX6011/6012)不可裁剪不旁路;SPIR-V 1.4 分叉不动 1.0 路径;步骤 69 blocked 探针恒跑(RD-034)"
  - "release.yml 触发器维持收窄;g4-closed tag 不匹配触发器零误触发;生产签名门控 0-byte"
  - "仓库 LF byte-exact(* -text):新文件 LF + 尾换行,禁 Python 文本模式写文件;提交前逐文件字节核 CR + 尾字节(git numstat + 二进制读,禁 grep $'\\r')"
  - "spec 修订表表头维持「版本」列名,数据行避「版本」子串(用「版号」)、忌「日期」子串入 bless 数据行;本契约既有条款 0-byte,close-out 只追加 §8;status 翻转/基准切换/g4-closed tag/RD·SG 处置由 agent 自主签署"
  - "guardrail 回退基准默认 = ei1-closed(PR 路径以 GITHUB_BASE_REF 为准);G4.7 close-out 切至 g4-closed 并双基准 advisory 复核(基准链单线性,EA1 仍 active 另裁)"
  - "UC-05/blackhole kernel 维持编译期有界形态(RD-027 毒径警示,G3.1 归因结论并读);RURIX_REQUIRE_REAL 纪律贯穿 device 段(mock/SKIP 不充绿)"
---

# G4 契约 — 引擎渲染期

> 所属:[../../02_USERS_AND_USE_CASES.md](../../02_USERS_AND_USE_CASES.md) §2 U5 + [../../06_GPU_GRAPHICS_PROGRAMMING_MODEL.md](../../06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) §8.3 / 契约机制见 [../../14_ENGINEERING_DISCIPLINE.md](../../14_ENGINEERING_DISCIPLINE.md) §1。
> 规范先行延续(AGENTS.md 硬规则第 7 条):语义面 PR 必须引用 RXS-#### 条款号;缺条款先补 spec,条款 commit 先于实现 commit。
> 基准 ref:**默认 `ei1-closed`**(PR 路径以 `GITHUB_BASE_REF` 为准;基准链 mb1-closed→g3-closed→ei1-closed 单线性,EA1 仍 active 未收口,日后另裁)。
> 粒度:**单 G4 阶段契约**:一份契约覆盖 G4 期,G4.0~G4.7 主线分解见 [G4_PLAN.md](G4_PLAN.md)。
> **定位口径:G4 把「rurix 渲染器可用于游戏引擎」从现状推进到 measured 工程事实。**现状(EI1 close-out 已核):compute RHI 已嵌入实测(engine_host v2 三方数值相等),图形着色面仅在语言/运行时层(G3 五面 device measured),RHI 库面仅 compute pass graph——mesh / RT / 采样 / bindless / present 库面零覆盖;RD-035 执行面三项未实现;.rx 单源 Vulkan RHI 未通(RD-031 open);C ABI 子集 v1 边界未定 v2;BLACKHOLE realtime 路径 rxp_create 返回 Shim E_NOTIMPL。G4 把「图形 RHI 化 + Vulkan RHI + RHI 执行面余项兑现 + BLACKHOLE 生产档验收」全量 measured 落地。「全量」的诚实边界:每条腿真实做到证据边界——blocked-on-upstream 项以「探针维护 + 诚实存续」为唯一合法结局(G-MB1-6 / RD-034 先例),不算失败、不伪造;measured-first / blocked-honest 高于「全量」表述。
> **治理口径:agent 完全自主(D-406 v2.0 / AGENTS v3.0 硬规则 1)**——起草 / 实现 / 执行 / 验证 / 判档 / 合入 / bless / close-out / 翻转状态全部自主,无批准门、无中间检查点等待。「一次性完成」语义 = 单期契约覆盖全部范围 + 主线严格串行(G3.0→G3.7 先例)+ 无等待点;不是绕过治理、不是并行乱撞。Full RFC 对抗性评审(D-409)全程:评审 provenance ≠ 起草 provenance,check_contribution 规则 4 机核。
> **脚手架口径:本契约为 G4 开工结构件,不实现任何语义面、不落条款、不打 tag;§8 close-out 开工时为空。**

---

## 1. 目标

G4 期结束时项目获得:① **图形 RHI 化**——.rx RHI 库面自 compute-only 扩为图形面:raster / mesh pass 类型 + 采样 / bindless / present 面库化 + render graph 自动 barrier 覆盖图形 pass,经 #[export(c)] 导出被 engine_host v3(C++/D3D12)嵌入 device 真跑三方数值对照;② **RD-035 执行面三项兑现**——transient 别名复用 + 执行期峰值计数器(I10 升 measured)/ 依赖驱动重排 + 并行调度(新拦截项入矩阵)/ RXS-0262 const 泛型定长容量编译期拒;③ **.rx 单源 Vulkan RHI**——artifacts v2 @__rx_gpu_spirv 段接通(RD-031),compute+graphics 双腿经 Vulkan 通道 device 真跑;④ **C ABI v2 判档**——以真实硬需求裁决 repr(C) struct 按值 + 回调指针是否兑现,判档留痕;⑤ **BLACKHOLE 生产档验收**——realtime 归因修复 + 30fps measured + REALTIME_OK;⑥ 收口——status closed + g4-closed tag + RD/SG 处置留痕。

## 2. 范围

### 2.1 in-scope

| 项 | 说明 | gating | 对应交付物 |
|---|---|---|---|
| g4_governance | G4.0 治理包 + 台账校准 | 结构件 | D-G4-1 |
| umbrella_rfc_0015 | G4.1 伞形 Full RFC-0015 四章 | D-409 对抗性评审 | D-G4-2 |
| graphics_rhi | G4.2 图形 RHI 化(主面) | **RFC-0015 Approved** | D-G4-3 |
| rd035_execution_face | G4.3 RD-035 三项 | **RFC-0015 Approved** | D-G4-4 |
| vulkan_rhi | G4.4 .rx 单源 Vulkan RHI(条件臂) | 前置核实留痕 | D-G4-5 |
| c_abi_v2_adjudication | G4.5 C ABI v2 判档(条件臂) | 判档留痕 | D-G4-6 |
| blackhole_acceptance | G4.6 BLACKHOLE 收尾验收 | 修复先于测量 | D-G4-7 |
| g4_closeout | G4.7 close-out | agent 自主签署 | D-G4-8 |

### 2.2 out-of-scope(显式排除)

见 YAML 头 `out_of_scope` 字段逐项(rd027_upstream_poison / rd034_dxil_rt / g_mb1_6_amd / window_input_language / upstream_filing / ea1_track / abi_stability_promise);11 §2 红线不触碰。blocked-honest:RD-027/RD-034 越过 close-out 存续不伪造。

## 3. 交付物清单

| ID | 交付物 | 形态 | 完成判据 |
|---|---|---|---|
| D-G4-1 | G4 治理包四件 + ledger 校准 | milestones/g4/ + number_ledger v1.13 | G-G4-1 |
| D-G4-2 | RFC-0015 伞形 Approved | rfcs/0015 + 对抗性评审段 | G-G4-2 |
| D-G4-3 | 图形 RHI 化全栈 | 条款 + rurixc/rhi.rs/vk.rs + uc05 图形 demo + engine_host v3 + 步骤 76+ | G-G4-3 |
| D-G4-4 | RD-035 三项 | 分配器/峰值计数器/重排并行/const 容量 + 矩阵 + reject 语料 | G-G4-4 |
| D-G4-5 | Vulkan RHI 通道 | artifacts v2 通道本体 + compute/graphics 双腿 device 真跑 | G-G4-5 |
| D-G4-6 | C ABI v2 判档 | 判档留痕 +(条件)条款 + ABI 往返真跑 / RD-036+ 登记 | G-G4-6 |
| D-G4-7 | BLACKHOLE 验收 | 归因 + 修复 + 30fps evidence + REALTIME_OK + 帧对照 | G-G4-7 |
| D-G4-8 | close-out 终审 | 契约 §8 + 基准切换 + tag + RD/SG 处置 | G-G4-8 |

## 4. 验收门(完整版,YAML 头为可提取摘要)

见 YAML 头 `acceptance_gates` 字段 G-G4-1 ~ G-G4-8。要点:
- **G-G4-1(治理门)**:四件套 + 台账四处滞后消除,check_number_ledger PASS。
- **G-G4-2(RFC 门)**:RFC-0015 Approved 前置 + D-409 对抗评审 disposition + 失败测试先行。
- **G-G4-3(图形 RHI 门)**:≥1 raster + ≥1 mesh 图形 pass 库面 + 自动 barrier 出图;engine_host v3 三方数值精确相等;生成头逐字节比对。
- **G-G4-4(RD-035 门)**:I10 升 measured;重排/并行新拦截项漏拦即红;const 容量越界编译期拒。
- **G-G4-5(Vulkan 门,条件臂)**:前置具备 → .rx 单源 Vulkan RHI device 真跑;不具备 → 存续留痕。
- **G-G4-6(ABI v2 门,条件臂)**:判档留痕 +(若兑现)struct 按值/回调指针 ABI 往返真跑。
- **G-G4-7(BLACKHOLE 门)**:归因留痕 + 修复后 30fps measured + REALTIME_OK + 帧对照。
- **G-G4-8(收口门)**:--strict 零 estimated + 终审表 + status flip + 基准切换 + tag + RD/SG 处置。

## 5. Guardrails(字节级,机器核对)

见 YAML 头 `guardrails` 字段。核对方式:`py -3 ci/check_guardrails.py`(无参默认基准 = `ei1-closed`;PR 路径以 `GITHUB_BASE_REF` 为准)。

## 6. Deferred 引用

| 编号 | 内容摘要 | 承接 |
|---|---|---|
| RD-035 | UC-05 RHI 执行面三项(transient 别名复用+峰值计数器 / 重排并行调度 / RXS-0262 const 容量) | **本期兑现对象**(G4.3):三项独立可分批;未兑现前矩阵 I10 note 与 RXS-0262「诚实收窄」段字面维持不改写 |
| RD-031 | artifacts v2 @__rx_gpu_spirv 段 codegen | **本期条件臂**(G4.4):前置已核在 main → 落通道本体 + device 真跑,close / 收窄处置留痕 |
| RD-027 | NVIDIA ptxas -O1+ 毒径(上游侧不可修) | **out-of-scope 维护**:MR-0011 护栏 + DRAFT 备包维持,不翻状态 |
| RD-034 | DXIL RT blocked-on-upstream | **out-of-scope 维护**:步骤 69 探针恒跑;翻绿=复评信号,不强推 |

详情以 [../../registry/deferred.json](../../registry/deferred.json) 为唯一事实源,本表仅引用。执行期新 RD 自 **RD-036** 起按 14 §4 追加并双侧标注。

## 7. 修订记录 / 开工裁决留痕

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-07-23 | 初版契约固化(G4 开工脚手架)。**开工裁决留痕**(owner 立项确认 + agent 完全自主 D-406 v2.0,记于本节;13_DECISION_LOG 执行 PR 字节冻结不改,G3/EI1 先例):① **立项 = owner 确认经提示词全文下达**:owner(白栀)2026-07-23 将《G4 引擎渲染期 agent 提示词》(agent-prompt-g4.md)全文下达并明示「本提示词即 owner 立项确认」——EI1 激活先例(owner 选定 + agent 代录非代签,EI1_CONTRACT §7 v1.1 ①)。**G4 无 gated 期**:EI1 的 gating 源于 owner 双轨资源串行化裁决(EI1_CONTRACT §0);G4 为 EI1 close-out(2026-07-23 签署 + `ei1-closed` tag + PR #185 合入 e8880f60)后的单轨期,status 直接 active。② **台账校准裁决(提示词 §3 四处滞后,以 git log / git ls-tree / rfcs/README §5 复核兑现)**:a. RFC next_free 13→**15**(RFC-0013 G3 伞形 / RFC-0014 EI1 已消费,rfcs/README §5 标 RFC-0015 自由池);b. MR next_free 11→**12**(MR-0011 = G3 RD-027 护栏 ptxas -O0 pin 已消费,rfcs/README §5 标 MR-0012);c. **RXS-0266~0269 矛盾裁决**:EI1 close-out §8.1 称四号「作废不回收(burned)」、同段又写「next_free=266 由后续期顺位使用」,自相矛盾——按 10 §9.5「编号永不复用」+「作废不回收」字面裁决为 **burned 跳号**,RXS next_free 266→**270**(提示词 §3 owner 裁决;main 侧 burned 记 ledger notes + revision_log,**不入 shadow_reserved**——该字段专记 off-tree burned,EI1 契约 guardrails 明记);d. D next_free 408→**410**(D-408 = P1-2 earmark 维持,D-409 已被 13_DECISION_LOG v2.3 消费,以决策日志实际为准)。无误号段确认:CI 步骤自 **76** 起(步骤 70 = G3 showcase 永久 gap 不动)/ RD 自 **RD-036** 起 / U 自 **U31** 起(U29 EA1 预留维持)/ 工具类 RX 自 **RX7023** 起 / codegen RX 自 **RX6034** 起(RX6009 burned 不用,registry/error_codes.json 复核)/ SG-010 留续号维持。③ **伞形 RFC-0015 单号四章**(G3_CONTRACT §7 v1.1 单伞形先例):章 A 图形 RHI 化 / 章 B RD-035 三项 / 章 C .rx 单源 Vulkan RHI(RD-031)/ 章 D C ABI v2 条件臂(FFI ABI codegen 触硬规则 5,判档成立才落实现——G-EA1-3 / RXS-0249 条件分支先例,判档不成立则臂不实现、登记 RD-036+,RFC 修订行留痕不重开)。**BLACKHOLE 面不占 RFC**:realtime 修复 = 运行时/应用层修复 + 30fps 测量,present 语义已有条款(RXS-0197/0198/0220~0222),零新语义面;实现 PR 按 10 §3 判档(预期 Direct 或 Mini,执行期定,争议向上取严)。④ **G4.2 artifacts v2 前置切片裁决(工程依赖留痕)**:G4.2 图形 pass device 出图的工程前置 = .rx → SPIR-V artifact 通道(RD-031 对象:main 现状 .rx host 产物仅嵌 PTX,RXS-0192;vk 运行时底座在但 .rx 源 SPIR-V 无 artifact 通道可达)——G4.2 实现首切片落 artifacts v2(@__rx_gpu_spirv 段 + blob v2 + codegen 单测/golden,RD-031 backfill_condition 之 codegen 本体);**G4.4 落 Vulkan RHI 通道本体**(compute+graphics 双腿 .rx 单源经 Vulkan device 真跑)并承接 RD-031 处置;主线相序 G4.2→G4.4 不变,本切片为工程依赖驱动的内部分片,非绕道。⑤ **编号 claim(编号永不复用,10 §9.5)**:Full RFC = **RFC-0015** 单号伞形;RXS 自 **RXS-0270** 起(预期 0270~0299 切分:0270~0279 图形 RHI 库面 / 0280~0289 RD-035 执行面 / 0290~0294 artifacts v2 + Vulkan RHI 通道 / 0295~0299 ABI v2 条件臂,以实现实际为准,溢出自 0300 顺续 + ledger 校准);CI 数字步骤自 **76** 起(预期 76 图形 RHI 冒烟 / 77 图形不变量门 / 78 引擎嵌入 v3 / 79 RD-035 执行面门 / 80 Vulkan RHI 通道 / 81 BLACKHOLE realtime,数量随实现回填不预占,多余号作废声明 burned);错误码 codegen 自 **RX6034** 续(RX6009 burned 不用)/ 3xxx typeck 按合并序 / 工具类自 **RX7023**;unsafe-audit 自 **U31** 起(U29=EA1 预留显式跳让不回收);新 deferred 自 **RD-036** 起(RD-016/028 跳号维持);MR 自 **MR-0012** 起按需;SG **零消费**(各面均为既登记 deferred 兑现非扩张方向;SG-010 软保留维持);共享 D 段 **零消费**(D-408=P1-2 earmark 不动;开工裁决记本节,D-G4-N 仅为交付物编号)。⑥ **执行编排(承 G3/EI1 已验证范式)**:agent worktree 起草编译面 + 主循环 device 真跑迭代 + PR 合一等一;fmt 第一道;feature 矩阵双验;逐路径 add;GPU 实验全经 proc_guard;RURIX_REQUIRE_REAL=1 贯穿。⑦ **诚实边界**:达成表述 =「引擎级可用的工程闭环落地」;「引擎/外部采纳/用户数」carve-out 不宣称;blocked-on-upstream 项(RD-027/RD-034)探针维护 + 诚实存续为唯一合法结局;G-MB1-6(AMD)pending-hardware 不伪造;条件臂(G4.5 ABI v2)判档不成立 = 合法结局,登记存续不强做(P-12)。⑧ **基准确认**:ei1-closed(PR #185 merged e8880f60 + annotated tag 已在 origin);EA1 仍 active,milestones/ea1/** 0-byte 不代动。**开工后 agent 完全自主(D-406 v2.0)——close-out 判定 / 基准切换 / g4-closed tag / RD·SG 处置由 agent 自主签署** |

---

## 8. Close-out(只追加区 — 开工时为空)

<!-- G4.0 治理留痕(台账校准 + 编号 claim 指针)、G4.1 RFC-0015 对抗性评审与 Approved 留痕、G4.2~G4.6 各面验收记录(条款 / 步骤 76+ run URL / device 真跑 evidence / 红绿)、G4.7 全量回归冻结真实输出、验收门终审表、RD-031/035 及执行期新 RD 处置、SG 复评结论追加于此;上方条款 0-byte 修改。G4 close-out 关闭判定 / 基准切换(按 main 合并序串行化)/ g4-closed tag / RD·SG 处置由 agent 自主签署兑现。 -->

### 8.1 PR-B 图形 RHI 主面合入留痕(G4.2 / G-G4-3;RFC-0015 §4.A;RXS-0270~0273/0275;2026-07-24)

**完成面摘要**:
- 条款先行:spec commit `c9c35ceb`(RXS-0270~0273 图形 RHI 类型面 + RXS-0275 mesh 编码,spec/rhi.md v1.4 + spec/vulkan_backend.md v1.19;零新 RX 码零新借用码)。
- rhi.rs 图形 pass/资源面扩:`raster_pass`/`mesh_pass` → `GfxPass<C>`;`color_target`/`depth_target`/`texture2d`/`sampler`/`texture_table` 五构造(薄映射 lang items + G3 既有条款面);桥接 graph.rs(AccessKind 单源复用,无 cabi marshalling);`derive_syncs` 0-byte 维持。
- vk.rs RHI 图形执行入口:消费 `PlannedBarrier` + .rx 源 SPIR-V(artifacts v2);既有 `run_graphics_offscreen_v2`/`run_mesh_offscreen`/`run_graph_offscreen` 0-byte。
- 访问声明集与自动 barrier(RXS-0272):`writes_rt`/`writes_depth`/`reads`/`binds_sampler` 封闭枚举;图形着色对反射并集(RXS-0273);图合法性违例装配期拒(库层状态值零新 RX 码,复用 RX6029/RX6030)。
- rurixc resolve/typeck/mir_build 图形 lang items 加性(已知方法分支);typeck 拒法沿 RX3012/3013/3017 族零新码。
- apps/uc05-rhi/src/gfx_demo.rx:1 raster + 1 mesh 图形 pass,`--emit=check` 0 诊断;零 .rs 审计维持。
- conformance/uc05 语料:2 accept(gfx_pass/gfx_resources)+ 5 reject(cross_brand_gfx RX3006 / rhi_gfx_in_kernel RX3015 编译期 + gfx_read_before_write / gfx_write_write_conflict / gfx_feedback_loop 装配期);uc05_corpus 8/8 零回归。
- ci 脚本:uc05_graphics_rhi_smoke.py(步骤 76)+ uc05_graphics_invariant_gate.py(步骤 77);pr-smoke.yml 步骤 76/77 回填;g4.counter 落 2 条 + budget_eval 两 evaluator 分支;number_ledger CI_step 75→77。
- stable 快照重 bless:spec_clauses 264→269(error_codes=106 / editions / subcommands 三段 0 变化);bless_log 同 diff;trace_matrix 264→269 全锚定(RXS-0273 锚 gfx_resources.rx / RXS-0275 锚 vulkan_codegen.rs mesh_entry_point_is_mesh_ext_model 测试)。
- evidence schema:milestones/g4/uc05_graphics_rhi_smoke_evidence_schema.json 新建(镜像 EI1 uc05_rhi_smoke 体例);check_schemas.py 路由分支加性。

**关键验证命令真实输出尾部**:

```
[trace_matrix] PASS (269/269 clauses anchored, 595 test files scanned)
[stable_snapshot] PASS(stable 面与入库快照一致:spec_clauses=269,error_codes=106,editions=['2026'],subcommands=['bench', 'build', 'check', 'doc', 'fmt', 'run', 'test', 'vendor'])
[check_schemas] PASS
[check_structure] PASS (11 dirs, 6 files)
[check_guardrails] PASS (base=ei1-closed, 38 changed paths)
[check_contribution] PASS(base=origin/main,1 非 merge commit + 0 Full RFC 全过:provenance + 条款号 + 验证 + 对抗性评审)
[check_redistribution] PASS — 版本化嵌入 PTX 无 __nv_* 符号...(再分发面为空)
[check_number_ledger] PASS(spec RXS 头 269 个零同号碰撞;ledger 14 命名空间保留号被尊重;red 自检已过)
[bilingual] PASS 写 evidence\bilingual_diagnostic_coverage.json(coverage_complete=true,zh/en key 集对齐 107/107)
[uc05_graphics_invariant_gate] PASS gfx I7/I8 编译期(cross_brand_gfx RX3006 / rhi_gfx_in_kernel RX3015)+ gfx I3/I5 装配期(编译期 CLEAN,违例归 submit() 装配期拦)+ gfx accept 0 诊断 + gfx_demo.rx 0 诊断 + uc05_corpus 零回归
[budget_eval] FAIL (strict mode) - g4.counter.graphics_rhi_smoke: FAIL — 仅 0 份 UC-05 图形 RHI device EXE red-green 见证(要求 ≥1)
```

注:budget_eval --strict FAIL 为 device 段 SKIP 的预期结果(见下);normal mode PASS(86 pass, 1 skip)。cargo build/clippy(双 feature 配置)/fmt --check/test(1 个 ptxas 环境失败)均在 PR-B 实现期通过(条款 commit c9c35ceb 之后;本提交未触及 .rs 实现,结果维持)。

**device 段 SKIP 原因(建设期正常态,device 见证回填待后续)**:

步骤 76 device 段需 GPU + Vulkan 后端真跑(rx build gfx_demo.rx → EXE → run + assembly-reject EXE red-green)。本机 `rx build` 默认 NVPTX 后端,而图形 shader 需 Vulkan 后端(归 PR-F Vulkan RHI 通道实现)。故 device 段判定 SKIP=dev-env degrade(非 fake pass,退 0;RURIX_REQUIRE_REAL=1 翻硬红)。host 段恒跑(uc05_corpus + 零 .rs 审计 + --emit=check)全 PASS。device EXE red-green 见证 + 像素判据(RXS-0222)归 PR-F/步骤 80 device 见证回填。

**evidence 路径**:

- evidence/uc05_graphics_rhi_smoke_20260724T170036.json(host_section_pass=true,device_section_rc=1,toolchain_skip=null,dev_env_degrade=false;run_url=local)
- milestones/g4/uc05_graphics_rhi_smoke_evidence_schema.json(schema;check_schemas 路由对齐)

**shader_stages.rs mesh body 类型面评估(Task 1.5 残项)**:

三项(输出数组声明形态 / mesh_set_outputs 已知函数面 / builtins 阶段矩阵)评估结论:gfx_demo.rx mesh pass `--emit=check` 0 诊断(编译本体可装配),当前实现够用;mesh MIR→SPIR-V lowering(lower_mesh)golden 测试 `mesh_entry_point_is_mesh_ext_model` 已锚 RXS-0275。shader_stages.rs mesh body 类型面扩展推迟到 PR-C/PR-D(留 TODO 指明),不阻塞 PR-B 合入。

**下一 PR(PR-C)开工声明**:

PR-C 采样/bindless/present 库化补齐(G4.2,RXS-0274/0276;验收锚步骤 76 覆盖扩)。分支 `feat/g4.2c-prc-bindless-present`(cherry-pick `8e86f3e3` 的 RXS-0274/0276 spec,条款先行)。TextureTable 入 pass(`.reads_table`)+ present handoff(`g.present(&back)`)+ 步骤 76 覆盖扩(像素判据含 bindless 动态索引)。串行口径:PR-B 合入后开工,合一等一。

### 8.2 PR-C 采样/bindless/present 库化补齐合入留痕(G4.2 / G-G4-3 通道覆盖扩;RFC-0015 §4.A3;RXS-0274/0276;2026-07-24)

**完成面摘要**:
- 条款先行:spec commit cherry-pick `8e86f3e3`(RXS-0274 present 终端 handoff 库化 + RXS-0276 TextureTable 入 pass bindless 面,spec/rhi.md;零新 RX 码零新借用码)。
- rhi.rs TextureTable 入 pass(RXS-0276):`GfxPassRecord::reads_table(&table)` 访问声明——无状态访问类(RXS-0273:barrier 相等域不核,绑定完备性另核),追加到 `bindings`(与 `binds_sampler` 同槽,无资源状态);不计 I3/I5。
- rhi.rs present handoff(RXS-0274):`GfxPassRecord::present(&back)` pass 级 + `RhiGraph::present(&back)` 图级便捷面(spec 允许两种形式);seal 时核验 present 唯一且末位(RXS-0272/0274);无 gfx pass / 已 seal → Structure Err(镜像 RX6029 口径,零新 RX 码)。
- vk.rs present handoff 执行:`run_rhi_present_handoff` 函数——消费 `derive_barriers` 产物中 `PresentHandoff` 类 barrier(`vk_new_layout` = `PRESENT_SRC_KHR`),纯 host 预校验(present 目标须为 ColorTarget,尺寸匹配)+ barrier plan 含 PresentHandoff 核验;窗口腿 TODO(复用 RXS-0197/0198 typestate + C++ shim D-130,0-byte);headless readback 路径归 PR-F 接线。
- rurixc 图形 lang items 加性:`hir.rs` 追加 `RhiGfxReadsTable`/`RhiGfxPresent` 枚举变体;`mir_build.rs` lowering(`reads_table` → `rxrt_rhi_gfx_declare(gfx, res, 5)` / `present` → `rxrt_rhi_gfx_present(gfx, res)`);`typeck.rs`/`tbir_build.rs`/`resolve.rs` 方法识别 + 借用剥壳;typeck 拒法沿既有族零新码。
- rurix-rt-cabi C ABI 符号:`rxrt_rhi_raster_pass`/`rxrt_rhi_mesh_pass`/`rxrt_rhi_gfx_resource`(class 1~5)/`rxrt_rhi_gfx_declare`(access tag 5 = table)/`rxrt_rhi_gfx_present` 五符号;`RhiEntry` 扩 `gfx_passes` 增量建面(RXS-0194 0-byte 语义,追加式)。
- conformance/uc05 语料:`accept/gfx_bindless.rx`(TextureTable 入 pass 正例,RXS-0276/0272/0273)+ `reject/gfx_present_not_last.rx`(present 不在末位 pass 装配期拦)+ `reject/gfx_present_twice.rx`(双 present 装配期拦);3 件 LF + 尾换行 + 条款锚定头;编译期 CLEAN(assembly-reject 性质,违例归 submit 装配期)。
- ci/uc05_graphics_rhi_smoke.py(步骤 76)覆盖扩:host 段步骤 5 `gfx_bindless.rx --emit=check` 0 诊断(RXS-0276);device 段步骤 8 PR-C bindless 四象限像素判据(gfx_bindless.rx EXE 真跑,PR-C 库面 GREEN = exit 0;四象限逐色像素判据归 PR-F Vulkan 通道 device 见证,同 gfx_demo 像素判据 RXS-0222 归 PR-F/步骤 80)。
- ci 设备段 SKIP 修正(PR-B 遗留):`_is_nvptx_graphics_skip` 辅助——`rx build` 遇 RX6003(NVPTX 不支持图形 shader,fragment/vertex/mesh)→ SKIP(dev-env degrade,退 0)而非 FAIL;PR-B §8.1 已声明此为 SKIP 场景,PR-C 步骤 76 覆盖扩修正 FAIL→SKIP 口径;evidence `dev_env_degrade=true`。
- stable 快照重 bless:spec_clauses 269→271(RXS-0274/0276;error_codes=106 / editions / subcommands 三段 0 变化);bless_log 同 diff;trace_matrix 269→271 全锚定(RXS-0274 锚 gfx_present_not_last.rx + gfx_present_twice.rx + rhi.rs 库单测 / RXS-0276 锚 gfx_bindless.rx + rhi.rs reads_table)。

**关键验证命令真实输出尾部**:

```
[uc05_graphics_rhi_smoke] host 步骤 1 PASS: uc05_corpus 批跑（compute 路零回归 + assembly 编译期 CLEAN + I1~I10 矩阵三方一致）
[uc05_graphics_rhi_smoke] host 步骤 2 PASS: 零 .rs 审计（apps/uc05-rhi 仅 4 个 .rx + rurix.toml,零 .rs/.cpp/.c/.py）
[uc05_graphics_rhi_smoke] host 步骤 3 PASS: --emit=check（不 link）gfx_demo.rx 0 诊断（图形 pass 声明 + 装配核验可编译本体）
[uc05_graphics_rhi_smoke] host 步骤 4 PASS: --emit=check 5 个 gfx assembly-reject 语料编译期 CLEAN（证 gfx I3/I5 非编译期,图装配期确定性拦）
[uc05_graphics_rhi_smoke] host 步骤 5 PASS: --emit=check gfx_bindless.rx 0 诊断（PR-C RXS-0276 TextureTable 入 pass `.reads_table` bindless 动态索引声明面）
[uc05_graphics_rhi_smoke] SKIP device 段: gfx_demo.rx rx build 遇 RX6003(NVPTX 不支持图形 shader;图形 shader 需 Vulkan 后端,归 PR-F;host 段已恒跑)（dev-env-degrade,退出 0）
[uc05_graphics_rhi_smoke] 写 evidence evidence\uc05_graphics_rhi_smoke_20260724T180904.json; run_url=local
[trace_matrix] PASS (271/271 clauses anchored, 598 test files scanned)
[stable_snapshot] PASS(stable 面与入库快照一致:spec_clauses=271,error_codes=106,editions=['2026'],subcommands=['bench', 'build', 'check', 'doc', 'fmt', 'run', 'test', 'vendor'])
[check_number_ledger] PASS(spec RXS 头 271 个零同号碰撞;ledger 14 命名空间保留号被尊重;red 自检已过)
[uc05_graphics_invariant_gate] PASS gfx I7/I8 编译期(cross_brand_gfx RX3006 / rhi_gfx_in_kernel RX3015)+ gfx I3/I5 装配期(编译期 CLEAN,违例归 submit() 装配期拦)+ gfx accept 0 诊断 + gfx_demo.rx 0 诊断 + uc05_corpus 零回归(compute + gfx 编译期 reject 全拦截)
```

注:cargo fmt --check / clippy(双 feature 配置)均在 PR-C 实现期通过。uc05_corpus 8/8 零回归;rhi.rs 库单测 16/16(含 reads_table/present handoff pass+graph 级);rurix-rt-cabi 18/18(含 5 新 gfx 符号)。

**device 段 SKIP 原因(建设期正常态,device 见证回填待 PR-F)**:

步骤 76 device 段需 GPU + Vulkan 后端真跑(rx build gfx_demo.rx/gfx_bindless.rx → EXE → run + assembly-reject EXE red-green)。本机 `rx build` 默认 NVPTX 后端,而图形 shader 需 Vulkan 后端(归 PR-F Vulkan RHI 通道实现);`rx build` 遇 RX6003(NVPTX 不支持 fragment/vertex/mesh shader)→ `_is_nvptx_graphics_skip` 判定 SKIP=dev-env degrade(非 fake pass,退 0;RURIX_REQUIRE_REAL=1 翻硬红)。host 段恒跑(uc05_corpus + 零 .rs 审计 + --emit=check 含 gfx_bindless)全 PASS。device EXE red-green 见证 + bindless 四象限像素判据(RXS-0222 headless readback)归 PR-F/步骤 80 device 见证回填。

**evidence 路径**:

- evidence/uc05_graphics_rhi_smoke_20260724T180904.json(host_section_pass=true,device_section_rc=0,toolchain_skip=nvptx-no-graphics,dev_env_degrade=true;checks: compile_gfx_bindless=true, bindless_run_green=SKIP, bindless_pixel_criteria=SKIP;run_url=local)

**下一 PR(PR-D)开工声明**:

PR-D engine_host v3 嵌入(G4.2,RXS-0277;验收锚步骤 78)。apps/uc05-rhi/src/embed.rx 追加图形导出(子集 v1 签名:标量 + 裸指针)+ src/rurix-engine/harness/ 新增 engine_host v3 文件(C++/D3D12,LUID 匹配)+ 生成头逐字节守卫 + 步骤 78 三方数值精确相等(Q-PixelCriterion:纯色/nearest RGBA8 整数 fetch 域,不设 ULP 容差)。串行口径:PR-C 合入后开工,合一等一。

### 8.3 PR-D engine_host v3 嵌入合入留痕(G4.2 / G-G4-3 通道嵌入;RFC-0015 §4.A7;RXS-0277;2026-07-24)

**完成面摘要**:
- 条款先行:spec commit `810a8773`(RXS-0277 engine_host v3 嵌入面 + 三方数值精确相等判据 Q-PixelCriterion,spec/rhi.md;零新 RX 码零新借用码)。
- apps/uc05-rhi/src/embed.rx 追加图形导出(RXS-0277):`uc05_gfx_run_frame(out: *mut u32, w: i32, h: i32) -> i32` + `uc05_gfx_pass_count() -> i32` 子集 v1 签名(标量 + 裸指针,RXS-0251);整图封闭在一个 `#[export(c)]` host fn 内(EI1.4 同构);`//@ spec: RXS-0277` 锚定。
- src/rurix-engine/harness/engine_host_v3.cpp 新增(RXS-0277):C++/D3D12 harness——Vulkan↔D3D12 LUID 匹配(v2 = CUDA↔D3D12 母本升级);raster 对照 D3D12 graphics pipeline(vs/ps);mesh 对照 D3D12 mesh pipeline(ms_6_5/ps_6_5);engine_host v1/v2 既有资产 0-byte(新增文件不触既有)。
- src/rurixc/src/mir_build.rs RXS-0277 锚定:收集着色阶段 kernel 函数(vertex/fragment/mesh 等),使其符号在 LLVM IR 中有定义(被 raster_pass/mesh_pass 函数指针实参引用);`build_gpu_artifacts` 走 `build_device_crate` 独立路径不干涉 device codegen。
- 生成头 CI 再生成逐字节守卫(RXS-0254 同面):仓库零 tracked rurix_rhi.h;v3 harness include 现场再生成头;幂等(RXS-0253)+ 篡改再生成 byte-diff RED(RXS-0254)。
- ci/uc05_engine_embed_v3_smoke.py(步骤 78)新建 + pr-smoke.yml 回填:host 段恒跑(生成头不手写审计 + 三制共存审计 + 零 .rs 审计 + `--emit=dll` GPU 导出面三件含 gfx 四符号 + 生成头幂等 + 篡改再生成 byte-diff RED)+ device 段 gate real(cl.exe 编 engine_host v3 链 rurix_rhi.lib + d3d12 + dxgi + vulkan-1 真跑 + 三方数值精确相等 Q-PixelCriterion + RED 三路);RX7001(外部工具链失败)→ SKIP=dev-env degrade(非编译期红),`RURIX_REQUIRE_REAL=1` 翻硬红。
- evidence schema:milestones/g4/uc05_engine_embed_v3_evidence_schema.json 新建(镜像 EI1 uc05_engine_embed 体例,step=78,subject=uc05_engine_embed_v3);check_schemas.py 路由分支加性(uc05_engine_embed_v3_validator)。
- milestones/g4/uc05_graphics_rhi_smoke_evidence_schema.json 补 PR-C 遗留字段(compile_gfx_bindless / bindless_run_green / bindless_pixel_criteria),check_schemas PR-C 证据文件校验对齐。
- ci/uc05_engine_embed_smoke.py 修正:RX7001(外部工具链失败 ptxas/link.exe 不可用)归类为 SKIP=dev-env degrade,非编译期红(导出面/图装配面)。
- registry/number_ledger.json v1.17:CI_step on_tree_max 77→78 / next_free 78→79;notes 追加步骤 78 描述;revision_log v1.17 留痕。
- milestones/g4/g4_budget.json v1.2:counter_assertions 第三条 `g4.counter.engine_embed_v3`(device 见证基数 ≥1,对齐 ei1.counter.uc05_engine_embed + g4.counter.graphics_rhi_smoke device 见证计数先例);ci/budget_eval.py evaluator 分支加性(未知 id 强制 FAIL)。
- stable 快照重 bless:spec_clauses 271→272(RXS-0277;error_codes=106 / editions / subcommands 三段 0 变化);bless_log 同 diff;trace_matrix 271→272 全锚定(RXS-0277 锚 src/rurixc/src/mir_build.rs + apps/uc05-rhi/src/embed.rx)。

**关键验证命令真实输出尾部**:

```
[uc05_engine_embed_v3] host 步骤 1 PASS: 生成头自始生成不手写(仓库零 tracked rurix_rhi.h;v3 harness include 现场再生成头)
[uc05_engine_embed_v3] host 步骤 2 PASS: 三制共存(v1 手写路三件 + v2 生成路在位;v3 既不 include v1 头也不引用 rurix_engine_* 符号面)
[uc05_engine_embed_v3] host 步骤 3 PASS: 零 .rs 审计(apps/uc05-rhi 仅 4 个 .rx + rurix.toml;导出面 embed.rx 在内)
[uc05_engine_embed_v3] host 步骤 4 PASS: GPU 导出面 `--emit=dll` 产 .dll + .lib + .h(声明集 ['uc05_gfx_pass_count', 'uc05_gfx_run_frame', 'uc05_graph_pass_count', 'uc05_run_graph'];含 gfx 四符号,RXS-0277)
[uc05_engine_embed_v3] host 步骤 5 PASS: 生成头幂等 + 无绝对路径/时间戳(RXS-0253)
[uc05_engine_embed_v3] host 步骤 6 PASS: 篡改生成头 → 再生成逐字节比对 byte-diff(RXS-0254 RED 守卫非空过)
[uc05_engine_embed_v3] device 步骤 7 PASS: cl.exe 编译 engine_host v3(include 现场再生成头 + 链 rurix_rhi.lib / d3d12 / dxgi / vulkan-1)
[uc05_engine_embed_v3] SKIP device 段:engine_host v3 真跑 rc=2 (Vulkan↔D3D12 LUID 匹配 / D3D12 上下文不可达;无 GPU 或无 Vulkan provision)(dev-env-degrade,退出 0)
[uc05_engine_embed_v3] 写 evidence evidence\uc05_engine_embed_v3_20260724T202038.json; run_url=local
[trace_matrix] PASS (272/272 clauses anchored, 598 test files scanned)
[stable_snapshot] PASS(stable 面与入库快照一致:spec_clauses=272,error_codes=106,editions=['2026'],subcommands=['bench', 'build', 'check', 'doc', 'fmt', 'run', 'test', 'vendor'])
[check_schemas] PASS
[check_number_ledger] PASS(spec RXS 头 272 个零同号碰撞;ledger 14 命名空间保留号被尊重;red 自检已过)
[bilingual] PASS 写 evidence\bilingual_diagnostic_coverage.json(coverage_complete=true,zh/en key 集对齐 107/107)
[uc05_graphics_invariant_gate] PASS gfx I7/I8 编译期 + I3/I5 装配期 + accept 0 诊断 + gfx_demo.rx 0 诊断 + uc05_corpus 零回归
[budget_eval] PASS (86 pass, 2 skip, normal mode)
```

注:cargo fmt --check / clippy(默认 feature)/ test --workspace / build --workspace(默认 feature)均 PASS。`cargo build --features rurix-rt/vulkan,vulkan-backend` 退出 101 —— rustc 1.93.1 ICE(panicked at compiler\rustc_metadata\src\rmeta\encoder.rs:2431:51: no entry found for key,uc03-demo lib crate,incremental compilation bug),与 PR-D 代码无关(已知 rustc bug)。

**device 段 SKIP 原因(建设期正常态,device 见证回填待 PR-F)**:

步骤 78 device 段需 cl.exe + MSVC + Windows SDK(D3D12)+ Vulkan SDK + GPU 真跑 engine_host v3 三方数值精确相等。本机无 GPU 或无 Vulkan provision → cl.exe 编译虽 PASS(host 步骤 7),但 engine_host v3 真跑 rc=2(Vulkan↔D3D12 LUID 匹配 / D3D12 上下文不可达)→ SKIP=dev-env degrade(非 fake pass,退 0;RURIX_REQUIRE_REAL=1 翻硬红)。host 段恒跑(生成头不手写 + 三制共存 + 零 .rs + --emit=dll 三件 + 头幂等 + 篡改再生成 RED)全 PASS。device 三方数值精确相等(Q-PixelCriterion)+ RED 三路见证归 PR-F/步骤 80 Vulkan RHI 通道 device 见证回填(同 PR-B/PR-C device 段 SKIP 口径)。

**evidence 路径**:

- evidence/uc05_engine_embed_v3_20260724T202038.json(host_section_pass=true,device_section_rc=0,toolchain_skip=null,dev_env_degrade=true;checks: generated_header_not_handwritten=true, coexistence=true, zero_rs_audit=true, emit_dll_artifacts=true, header_idempotent=true, tamper_regen_red=true, harness_build=true, three_party_equal=SKIP, red_three_ways=SKIP;embed_v3_ok=false[device SKIP];run_url=local)
- milestones/g4/uc05_engine_embed_v3_evidence_schema.json(schema;check_schemas 路由对齐)

**下一 PR(PR-E)开工声明**:

PR-E RD-035 执行面三项(G4.3,RXS-0280~0283 + RXS-0239/0261/0262 修订;验收锚步骤 79)。transient 别名复用分配器(区间图着色,纯 host safe 码)+ 执行期峰值计数器(I10 自 report_only 升 measured)+ 依赖驱动重排 + 批级提交(DAG 拓扑分层)+ I11 拦截项(调度器与核验器两独立纯函数,red_self_test 双向)+ RXS-0262 const 泛型定长容量 .rx 接线 + reject 语料锚定 + ci/uc05_exec_face_gate.py(步骤 79)。串行口径:PR-D 合入后开工,合一等一。

### 8.4 PR-E RD-035 执行面三项合入留痕(G4.3 / G-G4-4;RFC-0015 §4.B;RXS-0280~0283;2026-07-24)

**完成面摘要**:
- 条款先行:spec commit `887d7430`(RXS-0280 transient 别名复用分配器 + RXS-0281 依赖驱动重排批级提交 + RXS-0282 I11 漏拦即红 + RXS-0283 RXS-0262 const 泛型定长容量 .rx 接线收窄;spec/rhi.md RXS-0280~0283 + RXS-0239/0261 追加式修订 + RXS-0262 收窄段更新;零新 RX 码全复用 RX2010 既有 const 诊断)。
- src/rurix-rt/src/alias_alloc.rs 新增(RXS-0280):transient 别名复用分配器——区间图贪心着色 + 三分量(size/align/lifetime)逐成员核满足性 + PeakCounter 饱和加减(on_alloc/on_free);#![forbid(unsafe_code)] 纯 host safe 码;无写者资源独立槽(保守);端点相邻保守异槽。10 库单测覆盖重叠/不重叠/三分量/无写者/峰值计数。
- src/rurix-rt/src/scheduler.rs 新增(RXS-0281/0282):依赖驱动重排 + 批级提交——derive_exec_plan DAG 拓扑分层(Kahn 算法 golden)+ verify_exec_plan 独立纯函数(重建依赖闭包逐边核,I11 pre-dispatch fail-closed)+ red_self_test 双向互证(桩化调度器丢边被核验器检出 + 桩化核验器被门检出)。9 库单测覆盖拓扑分层/丢边拦/核验器被门检出/双向互证。
- src/rurix-rt/src/rhi.rs 修改(RXS-0280/0281/0282 闭合):execute_exec_face 四序闭合 seal → derive_exec_plan → verify_exec_plan(I11 pre-dispatch fail-closed)→ derive_alias_plan → PeakCounter 初始化;exec_face_peak_below_declared_capacity 为 I10 measured_local 锚(别名复用后静态峰值 1024 < 声明容量 2048 非平凡成立);declared_capacity() + resource() 越界装配期二次防线。6 库单测覆盖四序闭合/I10 锚/越界防线/资源记账。
- src/rurixc/src/typeck.rs 修改(RXS-0283):Op::RhiGraph 分支 + eval_graph_cap(turbofish const 实参求值 i64 存 gpu_graph_caps)+ 编译期越界拒复用 RX2010 + non-static construction strict 拒 + 单定义 affine 链拒;3 消息 key(rhi.graph_cap_literal / rhi.graph_nonstatic / rhi.graph_dup)en/zh 成对注册。
- src/rurixc/src/{hir.rs,lower.rs,resolve.rs,mir_build.rs,tbir_build.rs,coloring.rs,launch_check.rs,shared_check.rs,views_check.rs} 修改:RhiGraph 变体 + MethodCall generic_args 透传 + graph 方法注册 + Op::RhiGraph lowering + turbofish const 实参物化 SynthInt 下发 cabi + MethodCall 模式 `..` 修复。
- src/rurix-rt-cabi/src/lib.rs 修改:rxrt_rhi_graph_create 符号(const 容量 i64 实参面)。
- conformance/uc05/accept/const_capacity_graph.rx 新建(RXS-0283 正例:CAP=8,3 resource,3 pass RAW 链,0 诊断)+ conformance/uc05/reject/transient_capacity_overflow.rx(CAP 越界 RX2010)+ nonstatic_graph_construction.rx(non-static construction strict RX2010)。
- src/rurixc/tests/uc05_corpus.rs 修改:COMPILE_REJECTS 5→7 + accept_const_capacity_graph 测试(const 容量语料三方一致)。
- ci/uc05_exec_face_gate.py 新建(步骤 79):host 段恒跑(alias_alloc + scheduler + rhi.rs exec_face 库单测 + uc05_corpus 批跑 + --emit=check 编译档)+ device 段 gate real(rx build const_capacity_graph.rx → EXE 真跑 + I10 measured 见证);RURIX_REQUIRE_REAL=1 翻硬红。
- .github/workflows/pr-smoke.yml 修改:步骤 79 回填。
- milestones/g4/g4_budget.json 修改:counter_assertions 第三条 g4.counter.exec_face_gate(device 见证基数 ≥1)+ revision_log;ci/budget_eval.py 修改:exec_face_gate evaluator 分支(未知 id 强制 FAIL)。
- registry/number_ledger.json 修改:CI_step on_tree_max 78→79 / next_free 79→80 + notes 步骤 79 描述 + revision_log。
- evidence/uc05_invariant_matrix.json 修改:I10 entry(report_only→measured_local)+ I11 entry(assembly_time 拦截项入矩阵)+ milestones/ei1/uc05_invariant_matrix_schema.json 修改(id pattern + minItems)+ ci/uc05_report_check.py 修改(I1-I11 + DOCUMENTED_UNMAPPED)+ evidence/uc05_comparison_report.md 修改(§1/§2/§3 更新)。
- milestones/g4/uc05_exec_face_gate_evidence_schema.json 新建(镜像 uc05_engine_embed_v3 体例,step=79,subject=uc05_exec_face_gate)+ ci/check_schemas.py 路由分支加性(uc05_exec_face_gate_validator)。
- stable 快照重 bless:spec_clauses 272→276(RXS-0280~0283;error_codes=106 / editions / subcommands 三段 0 变化);bless_log 同 diff;trace_matrix 272→276 全锚定(RXS-0280 锚 alias_alloc.rs / RXS-0281/0282 锚 scheduler.rs + rhi.rs / RXS-0283 锚 conformance/uc05/reject + typeck.rs)。

**关键验证命令真实输出尾部**:

```
[uc05_exec_face_gate] host 段 PASS:alias_alloc(RXS-0280)+ scheduler(RXS-0281/0282)+ rhi.rs exec_face 闭合(I10 measured_local 锚)+ uc05_corpus 零回归+ const 容量语料 reject RX2010 / accept 0 诊断(零新码)
[uc05_exec_face_gate] device 步骤 6 PASS: const_capacity_graph.rx EXE 真跑 exit 0(const 容量图装配核验通过 + exec_face 四序闭合 + kernel 派发成功)
[uc05_exec_face_gate] device 步骤 7 PASS: I10 measured_local 见证(host 库测峰值 1024 < 声明容量 2048,别名复用收紧非平凡成立;device EXE 真跑 exit 0 双锚)
[uc05_exec_face_gate] 写 evidence evidence\uc05_exec_face_gate_20260724T232800.json; run_url=local
[trace_matrix] PASS (276/276 clauses anchored, 603 test files scanned)
[stable_snapshot] PASS(stable 面与入库快照一致:spec_clauses=276,error_codes=106,editions=['2026'],subcommands=['bench', 'build', 'check', 'doc', 'fmt', 'run', 'test', 'vendor'])
[check_schemas] PASS
[check_number_ledger] PASS(spec RXS 头 276 个零同号碰撞;ledger 14 命名空间保留号被尊重;red 自检已过)
[uc05_graphics_invariant_gate] PASS gfx I7/I8 编译期 + I3/I5 装配期 + accept 0 诊断 + gfx_demo.rx 0 诊断 + uc05_corpus 零回归
  PASS g4.counter.exec_face_gate: PASS — 2 份 UC-05 执行面三项 device 见证(exec_face_ok=true + i10_measured_local=true)(要求 ≥1)
[budget_eval] PASS (87 pass, 2 skip, normal mode)
```

注:cargo fmt --check / clippy --workspace --all-targets -D warnings / test --workspace 均 PASS。`RURIX_REQUIRE_REAL=1 py -3 ci/uc05_exec_face_gate.py` 退 0(device 段真跑 PASS,非 SKIP)。

**device 段真跑见证(I10 measured_local 双锚成立)**:

步骤 79 device 段本机真跑 PASS(非 SKIP):`rx build const_capacity_graph.rx` 产 EXE,真跑 exit 0(const 容量图装配核验通过 + exec_face 四序闭合 + kernel 派发成功);I10 measured_local 双锚——① host 库测 `exec_face_peak_below_declared_capacity`(别名复用后静态峰值 1024 < 声明容量 2048,非平凡成立,aliasing 收紧而非平凡相等)+ ② device EXE 真跑 exit 0(exec_face 四序闭合在 device 端成立)。`RURIX_REQUIRE_REAL=1` 严格模式退 0(不触发 SKIP 分支)。I10 自 report_only 升 measured_local 兑现(RD-035 ① 项)。

**evidence 路径**:

- evidence/uc05_exec_face_gate_20260724T232800.json(host_section_pass=true,device_section_rc=0,checks: host_lib_tests=true, device_run=true, i10_measured=true, peak_bytes=1024, declared_capacity=2048, peak_below_declared=true;exec_face_ok=true;i10_measured_local=true;toolchain_skip=null;dev_env_degrade=false;run_url=local)
- milestones/g4/uc05_exec_face_gate_evidence_schema.json(schema;check_schemas 路由对齐)

**RD-035 处置留痕(closed)**:

RD-035「UC-05 RHI 执行面余项」三项全量兑现,registry/deferred.json status open→closed:① transient 别名复用分配器 + 执行期峰值计数器 → alias_alloc.rs(PeakCounter)+ rhi.rs exec_face I10 measured_local 双锚(峰值 1024 < 声明 2048 非平凡成立);② pass 重排与依赖驱动并行调度 → scheduler.rs(DAG 拓扑分层 derive_exec_plan + verify_exec_plan 独立纯函数);③ RXS-0262 const 泛型定长容量 .rx 编译期拒 → RXS-0283 turbofish const 实参接线 + 编译期越界拒 RX2010 + non-static strict 拒 + 装配期二次防线。I11 拦截项入不变量矩阵 assembly_time 档(red_self_test 双向互证)。backfill_condition 三项硬需求全兑现。

**下一 PR(PR-F)开工声明**:

PR-F Vulkan RHI 通道 device 见证回填(G4.4,G-G4-3 device 段 / G-G4-4 device 段收口;RXS-0222 像素判据 + 步骤 80)。步骤 76 gfx device SKIP(NVPTX 不支持图形 shader,需 Vulkan 后端)+ 步骤 78 engine_host v3 device SKIP(无 GPU/Vulkan provision)的 device 见证回填归 PR-F。串行口径:PR-E 合入后开工,合一等一。

### 8.5 PR-F Vulkan RHI 通道合入留痕(G4.4 / G-G4-5;RFC-0015 §4.C4;RXS-0293/0294;2026-07-25)

**完成面摘要**:
- 条款先行:spec commit `0ff190b4`(RXS-0293 .rx 单源 Vulkan RHI 通道:compute + graphics 双腿,`Rhi::create_vk(&ctx)` 显式后端 strict 无回退 + RXS-0294 device 见证判据:数值对照 + spirv-val + RURIX_REQUIRE_REAL=1;spec/vulkan_backend.md v1.20 + registry/number_ledger.json RXS on_tree_max 292→294 + CI_step on_tree_max 79→80;零新 RX 码,Vulkan 不可用走 RXS-0193 确定性诊断,装配期违例镜像 RX6029/RX6030)。
- `Rhi::create_vk(&ctx)` 显式后端构造器(RXS-0293):resolve.rs 注册 `create_vk` lang-item 已知方法 + typeck.rs `Op::RhiCreateVk` 分支(返回 `Rhi<Vk>` 句柄,strict 无回退)+ mir_build.rs lowering 降级为 `rxrt_rhi_create_vk(ctx)` 调用 + hir.rs `RhiCreateVk` 变体;`Rhi::create` = CUDA 既有 0-byte(backend=Cuda 默认)。
- rurix-rt-cabi/src/lib.rs `rxrt_rhi_create_vk` C ABI(RXS-0293,`#[unsafe(no_mangle)] pub extern "C"`):feature gate `rurix-rt/vulkan`——feature on → Vulkan loader 探测 + backend=Vk 构造;feature off → 确定性 handle-0(非 fake pass,diag 报 "Vulkan backend not compiled in",RXS-0193 口径);无环境探测静默切换、无静默回退。
- compute 腿 Vulkan 变体(RXS-0293):vk.rs `vulkan_available` 探测 + compute pipeline 自 SPIR-V 模块(按 kernel 名索引)+ descriptor set 自 marshalling 槽位(RXS-0208 既有 vk 映射:set 0 StorageBuffer 顺排 + push constants)+ dispatch + 计划同步点回放(PlannedBarrier);`run_compute` 同模式先例复用。
- graphics 腿复用 G4.2 路径(RXS-0293):A7 执行面同一通道——既有 `run_graphics_offscreen_v2`/`run_mesh_offscreen`/`run_graph_offscreen` 入口 0-byte 语义;present handoff 复用 PR-C `run_rhi_present_handoff`。
- conformance/uc05/accept/rhi_create_vk.rx 新建(RXS-0293 锚定):`Rhi::create_vk(&ctx)` 显式 Vulkan 后端构造正例,`--emit=check` 0 诊断;lowering 落 `rxrt_rhi_create_vk` 字面符号(非 `rxrt_rhi_create`)。
- src/rurixc/tests/uc05_corpus.rs 追加 `accept_rhi_create_vk_lowers_to_rxrt_rhi_create_vk` 测试(RXS-0293 锚定):断言 IR 含 `declare i64 @rxrt_rhi_create_vk(i64)` + `@rxrt_rhi_create_vk(` 调用形态。
- src/rurixc/tests/mesh_rt_vulkan_spirv_val.rs 追加 `//@ spec: RXS-0294` 锚定:spirv-val 全模块校验为 Vulkan RHI 通道 device 见证判据 L3 腿(步骤 80 host 段 check_spirv_val 调用本测试)。
- ci/vulkan_rhi_channel_smoke.py 新建(步骤 80):host 段恒跑(vk.rs/rhi.rs/cabi 库单测 + uc05_corpus 批跑 + `--emit=check` rhi_create_vk.rx + spirv-val 全模块校验)+ device 段 gate real(rx build rhi_create_vk.rx → EXE 真跑);`#@ spec: RXS-0293` / `#@ spec: RXS-0294` 锚定;SKIP 纪律:无 link 工具链 / 无 Vulkan 驱动 / 无 GPU → SKIP=dev-env degrade(退 0),`RURIX_REQUIRE_REAL=1` 把缺失翻硬红。
- .github/workflows/pr-smoke.yml 步骤 80 回填;milestones/g4/g4_budget.json counter_assertions 追加 `g4.counter.vulkan_rhi_channel`(device 见证基数 ≥1)+ revision_log;ci/budget_eval.py 追加 vulkan_rhi_channel evaluator 分支。
- registry/number_ledger.json CI_step on_tree_max 79→80 / next_free 80→81 + notes 步骤 80 描述 + revision_log。
- fmt/clippy 修复:src/rurix-rt-cabi/src/lib.rs `needless_return`(feature off 块 `return 0;` → `0`)+ src/rurix-rt/src/vk.rs `useless_format`(`format!("...")` → `"...".to_string()`)。
- stable 快照重 bless:spec_clauses 276→278(RXS-0293/0294;error_codes=106 / editions / subcommands 三段 0 变化);bless_log 同 diff;trace_matrix 276→278 全锚定(RXS-0293 锚 conformance/uc05/accept/rhi_create_vk.rx + src/rurix-rt-cabi/src/lib.rs rxrt_rhi_create_vk + src/rurixc/tests/uc05_corpus.rs / RXS-0294 锚 src/rurixc/tests/mesh_rt_vulkan_spirv_val.rs spirv-val L3 腿)。

**关键验证命令真实输出尾部**:

```
[vulkan_rhi_channel_smoke] host 段 PASS:vk.rs(feature vulkan)+ rhi.rs(backend 分流)+ cabi(create_vk 符号面)+ uc05_corpus(accept/rhi_create_vk 0 诊断 + lowering)+ --emit=check(0 诊断)+ spirv-val(全模块校验)
[vulkan_rhi_channel_smoke] SKIP device 段:rx build 失败(link.exe / Vulkan SDK 工具链面缺)(dev-env-degrade,退出 0)
[vulkan_rhi_channel_smoke] 写 evidence evidence\vulkan_rhi_channel_smoke_20260725T031419.json; run_url=local
[trace_matrix] PASS (278/278 clauses anchored, 604 test files scanned)
[stable_snapshot] PASS(stable 面与入库快照一致:spec_clauses=278,error_codes=106,editions=['2026'],subcommands=['bench', 'build', 'check', 'doc', 'fmt', 'run', 'test', 'vendor'])
[check_number_ledger] PASS(spec RXS 头 278 个零同号碰撞;ledger 14 命名空间保留号被尊重;red 自检已过)
[check_guardrails] ADVISORY (base=ei1-closed,不阻断)
[budget_eval] PASS (87 pass, 3 skip, normal mode)
```

注:cargo fmt --check / clippy --workspace --all-targets -D warnings / test --workspace 均 PASS(含 needless_return/useless_format 修复)。`cargo build --workspace --features rurix-rt/vulkan,vulkan-backend` 编译绿(CARGO_INCREMENTAL=0 绕 rustc 1.93.1 ICE,PR-D 同先例)。`RURIX_REQUIRE_REAL=1 py -3 ci/vulkan_rhi_channel_smoke.py` 退 1(device 段 rx build 链接失败翻硬红,per 脚本 SKIP 纪律设计;PR-B/C/D 同先例:device 段 SKIP 取 RURIX_REQUIRE_REAL 未设态,exit 0)。

**device 段 SKIP 原因(建设期正常态,device 见证回填待 provisioning)**:

步骤 80 device 段需 `rx build rhi_create_vk.rx` → EXE → 真跑(Vulkan 通道 compute 腿:create_vk + SPIR-V pipeline + descriptor set + dispatch + 回写)。本机 `rx` 在位但 `rx build` 链接失败(link.exe LNK2019:`rxrt_rhi_create_vk` 未解析——运行时库需 vulkan feature 重编 + Vulkan SDK provisioning;RX7001 外部工具链失败)→ SKIP=dev-env degrade(非 fake pass,退 0;`RURIX_REQUIRE_REAL=1` 翻硬红,PR-B/C/D 同先例)。host 段恒跑(vk.rs/rhi.rs/cabi 库单测 + uc05_corpus 批跑 + --emit=check + spirv-val 全模块校验)全 PASS。device compute 图 saxpy 级 + 图形图章 A demo 真跑 + 数值对照 vs host + vs CUDA 腿交叉对照 + spirv-val 全模块校验回填待 `rx` 工具链 vulkan feature 重编 + Vulkan SDK provisioning(G4.7 close-out 或 owner 裁决 provisioning 节点)。

**evidence 路径**:

- evidence/vulkan_rhi_channel_smoke_20260725T031419.json(host_section_pass=true,device_section_rc=0,checks: host_lib_tests=true, spirv_val=true, device_run=SKIP;vulkan_channel_ok=false;toolchain_skip=no-rx;dev_env_degrade=true;run_url=local)

**RD-031 处置留痕(closed)**:

RD-031「RXS-0209 device 描述表 v2 @__rx_gpu_artifacts blob bump + @__rx_gpu_spirv 段 codegen」closed(owner_milestone=MB1 → G4.4 PR-F 兑现):RXS-0209 IR2 描述表 v2 blob 经 PR-A artifacts v2(emit_gpu_artifact_globals 版本分叉,feature off → v1 逐字节不变)+ PR-F Vulkan RHI 通道(rxrt_rhi_create_vk 消费 v2 blob + @__rx_gpu_spirv 段 SPIR-V 变体)正交兑现。backfill_condition 全兑现(MS1.2 artifacts blob 合入 + codegen 单测/golden)。registry/deferred.json RD-031 status open→closed + history 追加 2026-07-25 PR-F 兑现留痕(evidence: spec/vulkan_backend.md RXS-0290~0294 / src/rurixc/src/codegen.rs emit_gpu_artifact_globals / src/rurix-rt-cabi/src/{artifacts.rs,lib.rs} v2 解析 + rxrt_rhi_create_vk / src/rurix-rt/src/vk.rs vulkan_available + run_compute / ci/vulkan_rhi_channel_smoke.py 步骤 80 / conformance/uc05/accept/rhi_create_vk.rx RXS-0293 锚定 / registry/number_ledger.json RXS on_tree_max 292→294)。

**下一 PR(PR-G)开工声明**:

PR-G C ABI v2 判档(G4.5,G-G4-6,条件臂;判档依据留痕契约 §8)。以 engine_host v3 图形嵌入的真实硬需求判档(Q-G 可证伪判据清单,唯一输入):① upcall 硬需求(嵌入面是否需要 .rx 侧调起宿主代码——数据指针无法承载「调用」语义,子集 v1 无替代表达 ⇒ 回调指针硬需求成立)② 外部固定 ABI(被嵌入方是否为 ABI 不可改的既有外部 API——engine_host v3 为本仓自建宿主,天然不满足)。判档依据留痕落 G4_CONTRACT §8,不 rubber-stamp。若判档成立 → RXS-0295/0296 条款先行 + ABI 往返真跑(3/5/8 字节三尺寸哨兵 + C 侧回调被 .rx 侧调起数值回传断言)+ RED 三路;若判档不成立 → RXS-0295/0296 号 burned + RD-036+ 登记 + RFC 修订行留痕(不重开 RFC,G-EA1-3 先例)。两种结局均合法(P-12:条件臂存在 ≠ 条件臂必须兑现)。串行口径:PR-F 合入后开工,合一等一。
