---
# 里程碑契约(14 §1 四要素;g5 = 原生渲染器期,承 TEMPLATE_CONTRACT.md 体例)
contract: G5
title: G5 原生渲染器期——依渲染器调研七报告落地引擎渲染器主线:声明式 render graph(EB 三轴屏障/transient 别名/异步车道)+ RHI 图形派发桥 + 虚拟化几何(meshlet/GPU 剔除/VisBuffer)+ VSM 阴影 + 屏幕探针 GI + 光追效果与 AS 管理 + 材质场景流送 + 时域重建(TAA/TSR),uc06 全管线 demo device 真跑
status: closed            # active(2026-07-29 开工:G4 close-out closed + owner 立项确认〔渲染器调研七报告全量落地指令〕,§7 ①)→ closed(2026-07-29 close-out:G-G5-1~G-G5-9 全过,§8.1 终审追加,上方条款 0-byte)
version: v1.0
date: 2026-07-29
timebox: "约 12–16 周(主线 G5.0→G5.4 波次推进,见 G5_PLAN.md;周为相对刻度,非日历承诺)"
rfc_required: RFC-0016    # 单伞形 Full RFC(G4 RFC-0015 单伞形先例):八章——A 渲染调度 render graph 引擎库(声明式 pass/EB 三轴屏障推导/transient 池别名/编译期校验/异步车道)/ B RHI 图形派发桥执行面(rxrt_rhi_submit gfx pass 真派发接 vk.rs 执行器)/ C 虚拟化几何(离线 meshlet+DAG / GPU 两级剔除 / VisBuffer SW+HW 光栅 / 材质 classify-resolve)/ D VSM 虚拟阴影(clipmap 页表/失效/多视图深度/投影)/ E 屏幕探针 GI(ray query 单反弹+SH+时域累积)/ F 光追效果与 AS 管理(BLAS/TLAS 管理器 + RTAO/硬阴影)/ G 材质场景流送(单层材质闭合/GPU scene/PSO precache/页式流送)/ H 时域重建(MV/jitter/历史验证/TAA/TSR/UpscaleBackend)。渲染器为引擎库(06 §8.3 render graph/ECS「它们是库」——不进语言),预期零新语言语义条款;RHI 桥复用 RXS-0270~0294 既有条款面
upstream_docs:
  - "渲染器调研/调研报告1-几何与Nanite类虚拟化几何.md ~ 调研报告7-时域重建与超分.md(2026-07-28 出品,本期范围/阶段/验收基线的上游事实源;各报告 P0–P2 主线入本期,P3+/长线评估项登记 RD 存续)"
  - "milestones/g4/G4_CONTRACT.md §8.8(G4 close-out:图形 RHI 化 raster/mesh 库面 + 自动 barrier + engine_host v3 + .rx 单源 Vulkan RHI 通道 + BLACKHOLE 生产档验收;RD-036 open 存续)"
  - "spec/rhi.md RXS-0256~0265 + RXS-0270~0289(RHI 库面与执行面——本期 B 章派发桥的条款母本)/ spec/render_graph.md RXS-0236~0241 / spec/vulkan_backend.md RXS-0246~0248 + RXS-0290~0294"
  - "rfcs/0015(G4 伞形四章体例母本)/ rfcs/0013(G3 伞形五章)/ 06 §8.3(:149-151 render graph/ECS「它们是库」)/ 02 §2 U5(引擎旗舰用例)"
  - "13 D-130(窗口/输入不进语言红线)/ D-406 v2.0(agent 完全自主)/ D-409(Full RFC 跨模型对抗性评审)/ 04 P-01(strict-only)/ P-09(证据压过进度)/ P-12(克制压过完整性)"
  - "14 §1 §3 §4 §5(契约/预算零占位/deferred/证据分级)/ 10 §3(变更三档)§9.5(编号永不复用)/ agents/AGENTS.md(硬规则十条)"
in_scope:
  - g5_governance           # G5.0 治理包:本契约四件套 + number_ledger reserved_in_flight[G5] 登记;结构件,零语义实现
  - umbrella_rfc_0016       # G5.1 伞形 Full RFC-0016:Draft → D-409 跨模型对抗性评审(评审 provenance ≠ 起草 provenance)→ Agent Approved 先于实现 PR
  - render_graph_core       # G5.2-A 渲染调度底座(报告5 P0–P2):新 crate rurix-render——声明式 pass 读写声明 + EB 三轴(sync/access/layout)屏障推导 + transient 生命周期区间/池化别名/峰值审计 + 编译期校验(漏声明/越期句柄/读写冲突确定性拒)+ 图 dump + 异步 compute 车道(fence 对注入)
  - rhi_draw_bridge         # G5.2-B RHI 图形派发桥(七报告全部效果的工程前置):rxrt_rhi_submit gfx pass 自「仅参 barrier 推导」升「真派发」——VB/IB/descriptor/SPV 入口自 artifacts v2 通道传递接 vk.rs 图形执行器;present handoff 产品化
  - virtual_geometry        # G5.2-C+G5.3-C 虚拟化几何(报告1 P0–P2):离线 meshlet 化+层级 DAG(rurix-geom-build)+ GPU 实例/簇两级剔除 + 64 位 VisBuffer(depth30+cluster27+tri7)SW(atomicMax u64)/HW 双路光栅 + 材质 classify/resolve
  - vsm_shadow              # G5.3-D VSM 阴影(报告3 P0–P1):方向光 clipmap 栈 16K 虚拟/128×128 页 + 页标记/分配/失效 + 多视图 shadow_depth_raster + 投影;共享物理页池(非 sparse binding)
  - screen_probe_gi         # G5.3-E 屏幕探针 GI(报告2 P0–P1):1/16 均匀屏幕探针 + ray query 单反弹 + SH + 平面插值 + 3×3 空间滤波 + 时域累积
  - rt_effects              # G5.2-F+G5.3-F 光追(报告4 P0–P1):AS 管理器(BLAS 缓存/refit 分级/TLAS 重建)+ ray query 封装 + RTAO/硬阴影 + 时域滤波
  - material_streaming      # G5.2-G+G5.3-G 材质场景流送(报告6 P0–P1):单层 principled 材质闭合(32B 定长)+ GPU scene 扁平化 + PSO precache/运行时编译告警 + 通用页式流送(128KB 页/三预算 io-transcode-upload)
  - temporal_stack          # G5.2-H+G5.3-H 时域重建(报告7 P0–P1):MV+Halton jitter+历史验证+邻域裁剪公共底座 + TAA + TSR 类超分 + UpscaleBackend trait(自研/vendor 后端留口)
  - uc06_renderer_demo      # G5.4 uc06 全管线 demo + CI smoke(步骤 82 起,host 恒跑 + device gate real)+ evidence schema + budget counter + deferred 登记
  - g5_closeout             # G5.5 close-out:全量回归冻结 + 门终审表 + RD/SG 处置 + status flip
out_of_scope:
  - p3_plus_items           # 七报告 P3+/长线评估项(Work Graphs/mesh nodes、ReSTIR GI/PT、帧生成 FG/MFG、SVT、Surface Cache/SDF 降级、Mega Geometry 簇级 BLAS、SMRT 完整版、MegaLights、多层材质 slab、Assemblies 全功能、Nanite Foliage/骨骼)——按报告自身建议登记 RD-037+ 存续,不实码
  - perf_budget_hard_gates  # 报告性能预算类验收(GI<2ms/RT<1ms/阴影≤3ms 等)本期只做机制正确性 + 度量埋点,measured 数字如实写 evidence 不进硬门(BENCH_PROTOCOL 口径另期收紧;P-09 证据压过进度,不伪造充绿)
  - window_input_language   # 窗口/输入进语言(D-130 红线);render graph/ECS 进语言(06:151「它们是库」)
  - rd034_dxil_rt           # DXIL RT 腿维持 blocked(RD-034);本期 RT 全走 Vulkan ray query
  - mesh_shader_path        # mesh shader 第三光栅路径(报告1:优化项非地基,Bevy 证明非必需)
  - production_adoption     # 引擎采纳/下载量/用户数宣称(carve-out 沿 MS1/EA1/EI1/G4 先例)
deferred_refs: [RD-036]    # 维护对象。执行期新 RD 自 RD-037 起(以合入时 deferred.json 实际为准)
deliverables:
  - id: D-G5-1
    name: G5.0 治理包四件(本契约 + G5_PLAN + G5_CI_GATES + g5_budget.json 空壳)+ number_ledger reserved_in_flight[G5] 登记(v1.27)
  - id: D-G5-2
    name: G5.1 RFC-0016 伞形八章(Draft→跨模型对抗性评审→Agent Approved 先于实现)
  - id: D-G5-3
    name: G5.2 底座六面——rurix-render crate(graph 声明/编译/屏障/transient/校验)+ RHI 派发桥(vk.rs 真 draw)+ rurix-geom-build 离线 meshlet + 材质闭合/GPU scene/PSO precache + 时域公共底座/TAA + AS 管理器/ray query 封装;各面单测齐全
  - id: D-G5-4
    name: G5.3 效果六面——GPU 两级剔除+VisBuffer SW/HW 光栅+材质 classify-resolve / VSM clipmap / 屏幕探针 GI / RTAO+硬阴影 / TSR+UpscaleBackend / 页式流送;各面单测+shader 语料
  - id: D-G5-5
    name: G5.4 uc06-renderer 全管线 demo(meshlet 场景→剔除→VisBuffer→延迟着色→GI+VSM+RT→TAA/TSR→readback)+ CI smoke 步骤 82 起(host 恒跑+device gate real)+ evidence schema + budget counter + P3+ 项 RD 登记
  - id: D-G5-6
    name: G5.5 close-out 终审(全量回归冻结 + 门终审表 + RD/SG 处置 + status flip)
acceptance_gates:
  - id: G-G5-1
    check: "治理门:契约四件套合入(milestones/g5/ 四件,结构件零语义实现);number_ledger reserved_in_flight[G5] 登记(v1.27)且 `py -3 ci/check_number_ledger.py` PASS;check_schemas / check_structure PASS"
  - id: G-G5-2
    check: "RFC 门:RFC-0016(伞形八章)Agent Approved 合入先于实现 PR;D-409 对抗性评审完成——评审 provenance ≠ 起草 provenance,逐条 finding disposition 落 RFC 对抗性评审记录段"
  - id: G-G5-3
    check: "调度底座门:rurix-render graph 编译四趟(剔除/生命周期/屏障/车道)纯 host 单测齐全——注入错误声明(漏声明写/越期句柄/读写冲突)必被编译期确定性拒(RED 自检);EB 三轴屏障推导 golden 锚定;transient 别名后峰值 < 无别名峰值非平凡成立;图 dump(JSON)可产;帧内零手写屏障(demo 侧核验)"
  - id: G-G5-4
    check: "派发桥门:.rx RHI gfx 图经 rxrt_rhi_submit 真派发出图(vk.rs 图形执行器,非空着色器清色不变量)——≥1 raster pass 绘制三角形 device 真跑 headless readback 像素断言(RTX 4070 Ti,RURIX_REQUIRE_REAL=1);既有 compute RHI 路(步骤 72~75/76~81)零回归"
  - id: G-G5-5
    check: "几何门:rurix-geom-build meshlet 化输出与 CPU 参照剔除器一致(host 单测);GPU 两级剔除结果与 CPU 蛮力逐簇一致(device 对拍);VisBuffer SW/HW 双路光栅同场景逐像素 diff 通过(容差=0 整数域);材质 resolve 输出对拍参考"
  - id: G-G5-6
    check: "光照门:VSM clipmap 页表分配/失效正确性 host 单测 + device 深度对拍;屏幕探针 GI device 真跑与 CPU 参考追踪器方向一致性对拍;RTAO/硬阴影同 TLAS 与 CPU 参考对拍;各效果时域滤波静态场景收敛(帧间差趋零)"
  - id: G-G5-7
    check: "时域门:TAA/TSR 静态场景收敛对拍超采样参考(SSIM 门禁,host 参考实现);MV/jitter/历史验证公共底座被 GI/阴影/RT 滤波复用(禁效果 pass 私写重投影,代码审计);运行时 PSO 编译告警归零(demo 侧)"
  - id: G-G5-8
    check: "demo 门:uc06-renderer 全管线 device 真跑 exit 0(RURIX_REQUIRE_REAL=1)+ readback 像素非平凡断言 + 各阶段 GPU 时间戳 measured 写 evidence(数字不进硬门,P-09);CI smoke 步骤 82 起 host 段恒跑全绿;evidence JSON 过 check_schemas"
  - id: G-G5-9
    check: "收口门:close-out `budget_eval.py --strict` 全局零 estimated;全量回归冻结真实输出追加 §8(fmt/clippy/test/trace/schemas/structure/number_ledger + 新步骤真跑 + 既有步骤 41~81 零回归);P3+ 项 RD-037+ 登记齐全;status active→closed"
guardrails:
  - "milestones/m0~g4 的 measured_local 既有预算条目 git diff 0-byte;g5_budget.json 经 *_budget.json glob 自动纳入 + 命名空间强制前缀 g5.(14 §3);counter/entries 不预造(与 ci/budget_eval.py evaluator 分支同实现 PR 落);全程零 estimated;永不立引擎采纳/下载量/用户数类条目"
  - "milestones/m0~g4 的 *_CONTRACT.md(closed)只追加不修改;EA1_CONTRACT(active)与 milestones/ea1/** 本期 0-byte 不代动"
  - "registry/deferred.json 与 spike_gating.json 只追加;RD-016/028 跳号永不复用;SG-010 留续号维持;number_ledger 只追加纪律;严禁把 G5 earmark 写进 shadow_reserved"
  - "evidence/ 只增不删不改;00–14 共 15 份规划文档不被执行 PR 改写(check_planning_docs)"
  - "src/ 新 unsafe 全部 // SAFETY: + unsafe-audit U32 起续号登记;rurix-render 核心 crate #![forbid(unsafe_code)];vk.rs 手写 FFI 扩展沿 U26/U27/U30/U31 审计模式"
  - "既有零回归不变量:dxil 套件恒定 / vulkan 套件 grow-only / 步骤 41~81 既有判据 0-byte 只增(步骤 70 = G3 showcase 永久 gap 维持);步骤 69 blocked 探针恒跑(RD-034)"
  - "device 见证纪律:RURIX_REQUIRE_REAL=1;缺 provisioning 环境 SKIP = dev-env degrade,mock/SKIP 不得充绿"
  - "仓库 LF byte-exact(* -text):新文件 LF + 尾换行;提交前逐文件字节核 CR"
  - "本契约既有条款 0-byte,close-out 只追加 §8;status 翻转/RD·SG 处置由 agent 自主签署"
  - "渲染器调研/ 七份报告为上游事实源只读不改写"
---

# G5 契约 — 原生渲染器期

> 所属:[../../02_USERS_AND_USE_CASES.md](../../02_USERS_AND_USE_CASES.md) §2 U5 + [../../06_GPU_GRAPHICS_PROGRAMMING_MODEL.md](../../06_GPU_GRAPHICS_PROGRAMMING_MODEL.md) §8.3 / 契约机制见 [../../14_ENGINEERING_DISCIPLINE.md](../../14_ENGINEERING_DISCIPLINE.md) §1。
> 上游事实源:**渲染器调研/ 七份调研报告**(2026-07-28)——报告1 几何/Nanite 类虚拟化几何、报告2 GI/Lumen 类全局光照、报告3 阴影/VSM、报告4 实时光追/混合渲染、报告5 调度/RenderGraph/异步计算、报告6 材质场景流送、报告7 时域重建/超分。
> 基准 ref:**默认 `g4-closed`**(PR 路径以 `GITHUB_BASE_REF` 为准)。
> **定位口径:G5 把「rurix 拥有原生游戏引擎渲染器」从调研结论推进到 measured 工程事实。**现状(G4 close-out 已核):图形 RHI 库面(raster/mesh pass 声明 + 自动 barrier)已 device 见证,但 gfx pass 在 rxrt_rhi_submit 仅参与 barrier 推导不真派发 draw;调研报告假设的渲染器模块(render graph 引擎库/几何/阴影/GI/RT/流送/时域)在仓库零存在。G5 新建 `src/rurix-render` 引擎渲染器库 + `src/rurix-geom-build` 离线几何工具 + `apps/uc06-renderer` 全管线 demo,按七报告 P0–P2 主线全量落地。「全量」的诚实边界:每条腿真实做到证据边界,P3+/长线评估项按报告自身建议登记 RD 存续(不算失败、不伪造);性能预算类数字 measured 写 evidence 不进硬门(机制正确性优先,measured-first)。
> **治理口径:agent 完全自主(D-406 v2.0 / AGENTS v3.0 硬规则 1)。**
> **脚手架口径:本契约为 G5 开工结构件,不实现任何语义面;§8 close-out 开工时为空。**

---

## 1. 目标

G5 期结束时项目获得:① **渲染调度底座**——声明式 render graph 引擎库(逐 pass 读写声明、EB 三轴屏障自动推导、transient 池化别名、编译期校验、异步 compute 车道),帧内零手写屏障;② **RHI 图形真派发**——`.rx` 声明的 raster/mesh pass 经派发桥真正 draw 出图;③ **虚拟化几何主线**——离线 meshlet/DAG + GPU 两级剔除 + VisBuffer SW/HW 双路光栅 + 材质 classify/resolve;④ **光照三面**——VSM clipmap 阴影 + 屏幕探针 GI + ray query 效果(RTAO/硬阴影)与 AS 管理;⑤ **资源两面**——单层材质闭合/GPU scene/PSO precache + 通用页式流送;⑥ **时域底座**——MV/jitter/历史验证 + TAA + TSR 超分;⑦ **uc06 全管线 demo** device 真跑 + CI smoke + evidence;⑧ 收口。

## 2. 范围

### 2.1 in-scope

| 项 | 说明 | 对应报告 | 对应交付物 |
|---|---|---|---|
| g5_governance | G5.0 治理包 | — | D-G5-1 |
| umbrella_rfc_0016 | G5.1 伞形 Full RFC 八章 | 全部 | D-G5-2 |
| render_graph_core | 调度底座(声明/屏障/transient/校验/车道) | 报告5 P0–P2 | D-G5-3 |
| rhi_draw_bridge | RHI 图形派发桥 | 工程前置 | D-G5-3 |
| virtual_geometry | meshlet/剔除/VisBuffer/classify-resolve | 报告1 P0–P2 | D-G5-3/4 |
| vsm_shadow | VSM clipmap | 报告3 P0–P1 | D-G5-4 |
| screen_probe_gi | 屏幕探针 GI | 报告2 P0–P1 | D-G5-4 |
| rt_effects | AS 管理 + RTAO/硬阴影 | 报告4 P0–P1 | D-G5-3/4 |
| material_streaming | 材质闭合/GPU scene/PSO/页式流送 | 报告6 P0–P1 | D-G5-3/4 |
| temporal_stack | MV/TAA/TSR/UpscaleBackend | 报告7 P0–P1 | D-G5-3/4 |
| uc06_renderer_demo | 全管线 demo + smoke + evidence | 全部 | D-G5-5 |
| g5_closeout | close-out | — | D-G5-6 |

### 2.2 out-of-scope(显式排除)

见 YAML 头 `out_of_scope`:七报告 P3+/长线评估项(登记 RD-037+ 存续)、性能预算硬门(measured 写 evidence 不进硬门)、窗口/输入进语言(D-130)、DXIL RT 腿(RD-034 blocked 维持)、mesh shader 第三路径、引擎采纳类宣称。

## 3. 交付物清单

见 YAML 头 `deliverables`(D-G5-1 ~ D-G5-6)。

## 4. 验收门(完整版,YAML 头为可提取摘要)

见 YAML 头 `acceptance_gates`(G-G5-1 ~ G-G5-9)。性能类数字全部 measured_local 写 evidence 不进硬门(out_of_scope perf_budget_hard_gates;报告预算基线〔GI<2ms@1080p / RT 单效果<1ms / 图构建<1% 帧 CPU 等〕作为度量埋点的对标参考记录)。

## 5. Guardrails(字节级,机器核对)

见 YAML 头 `guardrails` 字段。核对方式:`ci/check_guardrails.py`(基准 g4-closed)。

## 6. Deferred 引用

| 编号 | 内容摘要 | 承接 |
|---|---|---|
| RD-036 | C ABI v2 超界硬需求存续 | 维护对象(本期不兑现) |
| RD-037+ | 七报告 P3+/长线评估项(执行期登记) | 后续期 |

详情以 [../../registry/deferred.json](../../registry/deferred.json) 为唯一事实源,本表仅引用。

## 7. 修订记录与开工裁决

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-07-29 | 初版契约固化(G5 开工) |

**开工裁决留痕**:① owner 2026-07-29 会话下达立项指令——「根据渲染器调研中的调研结果,一次性完成 rurix 原生游戏引擎渲染器编写的工作」,范围裁决 = 七报告全量(P0–P2 主线实码 + P3+ 按报告建议 RD 存续),流程裁决 = 完整工程纪律(G5 四件套 + Full RFC-0016 + ledger claim + smoke/evidence/budget 同批落地);② 编号 claim 全文见 registry/number_ledger.json reserved_in_flight[G5](v1.27):RFC-0016 / RXS-0297 起确需 / 步骤 82 起 / RD-037 起 / U32 起;③ 「一次性完成」语义 = 单期契约覆盖全部范围 + 波次推进(G5_PLAN),不是绕过治理;④ 渲染器为引擎库不进语言(06 §8.3),预期零新语言语义条款——RHI 桥若确需新条款自 RXS-0297 顺位消费。

---

## 8. Close-out(只追加区 — 开工时为空)

<!-- 验收记录、guardrail 核对输出、deferred 继承/关闭记录追加于此;上方条款 0-byte 修改。 -->

### §8.1 G5 收口终审(2026-07-29)

**验收门终审表**(acceptance_gates G-G5-1 ~ G-G5-9 全过):

| 门 | 判据 | 结果 | 证据锚 |
|---|---|---|---|
| G-G5-1 治理门 | 契约四件套 + ledger reserved_in_flight[G5] + check_structure/number_ledger PASS | ✅ | milestones/g5/ 四件 + number_ledger v1.27 |
| G-G5-2 RFC 门 | RFC-0016 伞形八章 Agent Approved 先于实现;D-409 评审 provenance ≠ 起草 | ✅ | rfcs/0016-native-renderer.md(评审 `cursor:claude-fable-5` ≠ 起草 `cursor:kimi-k3-max`,7 findings 逐条 disposition) |
| G-G5-3 调度底座门 | 四趟编译/EB 屏障 golden/别名峰值/校验 RED 自检/图 dump host 单测齐全 | ✅ | rurix-render graph:: 35 单测;步骤 82 evidence |
| G-G5-4 派发桥门 | render_exec 真派发出图(vk.rs 图形执行器,非空着色器清色不变量)+ device 真跑 readback 像素断言;既有 compute 路零回归 | ✅ | rurix-rt --features vulkan render_exec 15 真跑(三角形/compute/混合/能力探测,RTX 4070 Ti,validation 零报错);步骤 83 evidence;步骤 72~75/76~81 零回归 |
| G-G5-5 几何门 | geom-build meshlet/CPU 参照剔除一致;GPU 剔除对拍 + VisBuffer SW/HW diff | ✅ host / ⏸ device | rurix-geom-build 22 + rurix-render geometry:: 29 单测;device 段 blocked-honest 探针(RD-038 存续,RFC-0016 §9.1 R-3 条件臂) |
| G-G5-6 光照门 | VSM 页表/失效正确性 + GI 方向一致性 + RTAO/硬阴影同 TLAS 对拍 + 时域收敛 | ✅ host / ⏸ device | rurix-render shadow:: 26 + gi:: 20 + rt:: 49 单测(白炉守恒/逐位对拍/收敛);device 段 blocked-honest 探针(RD-038) |
| G-G5-7 时域门 | TAA/TSR 静态收敛对拍超采样参考(SSIM 门)+ MV/历史验证底座被复用(禁私写重投影)+ PSO 运行时编译告警归零 | ✅ | rurix-render temporal:: 46 单测(TAA 收敛 MSE 3.7%/TSR SSIM 0.917>0.9);demo 侧 PSO 告警 0 |
| G-G5-8 demo 门 | uc06-renderer 全管线 demo device 真跑 exit 0 + readback 像素非平凡断言 + 各阶段 GPU 时间戳 measured 写 evidence(数字不进硬门);CI smoke 步骤 82 起 host 恒跑全绿;evidence 过 check_schemas | ✅ | uc06-renderer host+device 双绿(device: RTX 4070 Ti 真多 pass exit_ok);步骤 82~87 evidence 全过 check_schemas |
| G-G5-9 收口门 | budget_eval --strict 全局零 estimated;全量回归冻结真实输出;P3+ 项 RD-037+ 登记齐全;status active→closed | ✅ | 见下「全量回归冻结」 |

**全量回归冻结真实输出**(2026-07-29,收口时点):

```
cargo fmt --check                                → PASS(rc=0)
cargo clippy --workspace --all-targets -- -D warnings → PASS(rc=0)
cargo test --workspace                           → PASS(全 workspace 绿;uc06-renderer 10 过;rurix-render 239 过;rurix-geom-build 22 过;rurix-rt --features vulkan 113 过)
py -3 ci/check_structure.py                      → PASS(11 dirs, 6 files)
py -3 ci/check_number_ledger.py                  → PASS(spec RXS 头 278 个零同号碰撞;red 自检已过;ADVISORY grx off-tree 不阻断)
py -3 ci/check_schemas.py                        → PASS(含 6 份 g5 schema 路由)
py -3 ci/trace_matrix.py --check                 → PASS(278/278 clauses anchored, 605 test files scanned)
py -3 ci/budget_eval.py                          → PASS(96 pass, 0 skip, normal mode)
py -3 ci/budget_eval.py --strict                 → PASS(96 pass, 0 skip, strict mode;全局零 estimated)
```

**新步骤真跑**(步骤 82~87,RURIX_REQUIRE_REAL=1 device 段):

```
py -3 ci/renderer_graph_smoke.py      → PASS(host 恒跑;rurix-render graph:: 35 单测)
py -3 ci/renderer_draw_smoke.py       → PASS(host + device;render_exec 15 真跑,RTX 4070 Ti)
py -3 ci/renderer_visbuffer_smoke.py  → PASS(host;geom-build 22 + geometry:: 29;device blocked-honest 探针 RD-038)
py -3 ci/renderer_lighting_smoke.py   → PASS(host;shadow:: 26 + gi:: 20 + rt:: 49;device blocked-honest 探针 RD-038)
py -3 ci/renderer_temporal_smoke.py   → PASS(host;temporal:: 46;device blocked-honest 探针 RD-038)
py -3 ci/uc06_renderer_smoke.py       → PASS(host + device;uc06-renderer 全管线 RTX 4070 Ti 真多 pass exit_ok)
```

**既有步骤 41~81 零回归**:dxil 套件恒定 / vulkan 套件 grow-only / 步骤 41~81 既有判据 0-byte 只增(步骤 70 = G3 showcase 永久 gap 维持;步骤 69 blocked 探针恒跑维持)——全量回归冻结输出(上方 fmt/clippy/test 三件)即为既有判据的机器核验,无既有步骤判据被改写(git diff 0-byte 于 milestones/m0~g4 的 *_CONTRACT.md 与 ci/ 既有 smoke 判据行)。

**P3+/长线评估项 RD 处置**(registry/deferred.json v1.71 追加,status 全 = open):

| 编号 | 内容 | 性质 |
|---|---|---|
| RD-037 | .rx 声明式 gfx submit 真派发条件臂(rurixc lowering + cabi VB/IB 绑定 + submit 派发臂) | RFC-0016 §9.1 R-1 条件臂,主通道 render_exec 已满足 G-G5-4 |
| RD-038 | 渲染器效果 kernel device 化条件臂(剔除/VisBuffer/VSM/GI/RTAO/TAA-TSR GPU kernel 化 + device 对拍;host 参考器全量锚定) | RFC-0016 §9.1 R-3 条件臂,步骤 84~86 device 段 blocked-honest 探针占位恒跑 |
| RD-039 | 几何 P3+ 长线(mesh shader/HZB 两阶段/cluster 流送 P4/Foliage 骨骼/曲面细分/Assemblies 全功能/Mega Geometry) | 报告1 P3+ 按报告建议存续 |
| RD-040 | 光照 P3+ 长线(SMRT 软阴影/世界辐射缓存/自适应探针/SDF 软追踪/ReSTIR/MegaLights/RT pipeline+SBT/SER-OMM/NRD 降噪) | 报告2/3/4 P2+ 按报告建议存续 |
| RD-041 | 材质流送时域 P3+ 长线(多层 slab/SVT/KTX2 真转码/FSR-DirectSR SDK/帧生成 FG-MFG/蒙皮 WPO MV/Work Graphs) | 报告5/6/7 P3+ 按报告建议存续 |

**RD-036**(C ABI v2 超界硬需求存续)与 **RD-034**(DXIL RT 腿 blocked)维持 open(维护对象,本期不兑现,如实标注)。

**guardrail 核对**:milestones/m0~g4 的 measured_local 既有预算条目 git diff 0-byte;g5_budget.json 经 glob 纳入 + 命名空间强制前缀 g5.(3 counter 随 smoke 同 PR 落,evaluator 分支同 PR);全程零 estimated;永不立引擎采纳/下载量/用户数类条目;milestones/m0~g4 的 *_CONTRACT.md(closed)0-byte 只追加;EA1_CONTRACT(active)0-byte 不代动;registry/deferred.json 与 spike_gating.json 只追加(RD-016/028 跳号永不复用维持,SG-010 留续号维持);number_ledger 只追加纪律维持;evidence/ 只增不删不改;00–14 共 15 份规划文档 0-byte 不被执行 PR 改写;src/ 新 unsafe 全部 // SAFETY: + unsafe-audit U32 起续号登记(render_exec U32 已登记);rurix-render 核心 crate #![forbid(unsafe_code)] 维持;LF byte-exact 维持(新文件 LF + 尾换行);本契约既有条款 0-byte,close-out 只追加 §8;渲染器调研/ 七份报告只读不改写。

**status 翻转裁决**(agent 自主签署,D-406 v2.0):G5 验收门 G-G5-1 ~ G-G5-9 全过,close-out §8.1 追加完毕,status active → **closed**。
