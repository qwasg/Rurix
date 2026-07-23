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
