---
contract: MS1
title: MS1 期——使命判据落地第一期：single-source 宿主 GPU 编排（std::gpu）+ UC-07 以 Rurix 为主语言的生产级渲染器/仿真二合一应用（ruridrop）
status: closed            # active → closed（close-out 只追加 §8,上方条款 0-byte;基准 v1-closed→ms1-closed 切换 + ms1-closed tag 归 MS1.5,agent 自主签署）
version: v1.0
date: 2026-07-14
timebox: "中期（约 3–5 周,MS1.1~MS1.5 严格串行见 MS1_PLAN.md;周为相对刻度,非日历承诺）"
rfc_required: none        # 开工脚手架取 rfc_required: none（结构件,对齐 M4~V1 先例）:in_scope 两实体面各标 **Full RFC 前置 gating**——host_gpu_orchestration_stdlib 触运行时语义 + FFI ABI（硬规则 5 / 10 §3）= RFC-0009;uc07_sim_renderer_app 为使命级验收载体（对齐 UC-04=RFC-0006 先例,含主语言判据操作化）= RFC-0010。脚手架只登记不实现;agent 自主判档,判档争议向上取严（硬规则 8）
upstream_docs:
  - "01 §6 (使命成功层判据原文:「至少一个生产级渲染器/仿真系统选择 Rurix 作为主语言」;§4 五年图景第 1 条:kernel 与 host 调度代码在同一语言里、无不可诊断胶水——本期最严判据的叙述性依据)"
  - "11 §6 (五年愿景第一条同句;MS1 为该使命判据的第一期操作化落地,不宣称外部采纳维度达成)"
  - "02 §2 U3/U4 画像 + §4 UC-03(SPH+软光栅,M7 已达成)/UC-04(deferred 渲染器,G2.4 已达成)——UC-07 承此序列续号(编号 claim 见 §7 ④,02 为冻结规划档案不改写)"
  - "05 §1 (双层模型:host 层完整语言——本期把宿主 GPU 编排面兑现进 host .rx)"
  - "06 §2 §3 (kernel 抽象与内存路径;views/shared/atomics 安全基件)"
  - "08 §1 §2 (Driver API 薄层运行时;D-230~D-234 销毁纪律/poisoned 语义——rurix-rt-cabi 包装其 ownership 系)"
  - "spec/device.md RXS-0066~0082 (device 语义/launch 类型契约 RXS-0074/0075/ptxas 纪律 RXS-0073/libdevice RXS-0082) + spec/pipeline.md RXS-0130~0134 (affine 资源语义) + spec/interop.md RXS-0125 (手写 extern C ABI 口径) + spec/interop_d3d12.md RXS-0140~0143 (present typestate/fence 协议,D-130 shim 边界) + spec/imageio.md RXS-0114~0117 (PPM 确定性序列化) + spec/release.md RXS-0150~0152 (fatbin 装载协商)"
  - "13 D-406 v2.0 (agent 完全自主) / D-130 (present shim 边界) / D-131 (DXIL 混合裁决——本期不走 DXIL 图形路) / D-008 (多后端红线,维持不解除)"
  - "14 §1 §3 §4 §5 (契约 / 预算零占位 / deferred / 证据分级) / 10 §3 (变更三档) §7 (AI 贡献政策) / agents/AGENTS.md (硬规则十条)"
in_scope:
  - host_gpu_orchestration_stdlib  # MS1.2 single-source 宿主 GPU 编排 stdlib:host .rx 经 std::gpu(Context/Stream/Buffer/launch 首期收敛子集)编排 GPU,同源 kernel PTX 嵌入单 EXE + 装载协商复用(RXS-0150/0151/0076);经 src/rurix-rt-cabi(staticlib,rxrt_* C ABI)绑定 rurix-rt → **Full RFC 前置(RFC-0009)**;条款先行 spec/host_orchestration.md RXS-0189 续号;含前端机械面(extern "C" 符号保名 + #[link] 接线 + `mod name;` out-of-line 模块)
  - uc07_present_imageio_face      # MS1.2b present 宿主 typestate 面(.rx 侧 Present/Ready/Acquired/Presentable affine 消费式,镜像 RXS-0142;窗/泵/交换链维持 C++ shim,D-130 0-byte)+ 宿主图像落盘桥(rxio_write_ppm,RXS-0114~0117 语义)→ 随 RFC-0009 §4 承载,条款 RXS-0197~0199
  - uc07_sim_renderer_app          # MS1.3 UC-07 应用 ruridrop(apps/ruridrop):GPU SPH 溃坝(均匀网格 + bit-split 基数排序,atomics-free 确定性)+ 二合一渲染(球体图元 + 复用仿真网格 3D-DDA;离线路径追踪 + 实时光线投射),CUDA/PTX compute 路线;应用层全 .rx 零 .rs → **Full RFC 前置(RFC-0010,含主语言判据操作化)**;依赖 host_gpu_orchestration_stdlib 就位
  - offline_golden_gate            # MS1.3 离线确定性出图 golden 门:CI 步骤 53(零 .rs 审计 + 同机两跑逐字节一致 + CPU 参考容差 + blessed 哈希三层,device 真跑)
  - realtime_present_evidence      # MS1.4 实时 present 取证:本机交互桌面真跑 N 帧 → evidence/uc07_present_*.json(measured_local);**不进 CI 硬门**(镜像 ci/realtime_present_smoke.py 双态先例,SKIP 不充绿)+ 性能预算 ms1.bench.* 回填
out_of_scope:
  - multi_backend            # 多后端(D-008/SG-003 红线 3):维持不解除——本期 CUDA/PTX compute 路线是 NVIDIA 单栈纵深,不触碰
  - dxil_graphics_route      # UC-07 渲染不走 G2 DXIL 图形管线(RXS-0171 body 白名单为直线代码+LOD-0 采样,生产着色器不可达):显式声明以免与 UC-04 混淆;DXIL 图形 body 表达力扩展留后续里程碑按档处置
  - tensor_core_intrinsics   # SG-002:PT/SPH 不引 WGMMA/协作组,维持 not_triggered
  - interactive_windowed_app # 交互事件循环/键鼠输入/摄像机交互:realtime 入口为固定飞行相机帧循环;交互模式 defer(执行期登记 RD-027)
  - pt_completeness          # 路径追踪完备性(BVH 加速/重要性采样扩展/多材质系统/compute 路纹理对象):首期收敛子集外,defer(执行期登记 RD-028;纹理面触 06 §4.2 须 Full RFC 增补 RFC-0007)
  - registry                 # 包 registry(D-312/SG-007):维持休眠 not_triggered
  - grx_merge                # GRX showcase 分支合入 main:独立轨道,MS1 期间不合入(快照面串行化,§7 ⑦;例外合入则 rebase 后按合并面重 bless)
  - production_adoption_claim # 使命判据的「外部生产采纳」维度(社会判据):显式 carve-out——本期落「首个以 Rurix 为主语言的生产级渲染器/仿真系统」(第一方),不宣称 01 §6 使命成功层整体达成(对齐 G2 §8.8.5 / V1 ecosystem_criteria carve-out 口径)
  - rd_implementation        # RD-009(#[export(c)])实现:仅账面承接;RD-025(rurixup 真实 FS/网络):仅账面承接;RD-007(const 泛型运行期单态化):执行期评估接通、非开工验收门
deferred_refs: [RD-007, RD-009, RD-025]   # RD-007(inherited)/ RD-009(open)owner_milestone V1→MS1 顺延(deferred.json v1.46「待后续阶段顺延」兑现);RD-025(open)owner_milestone post-V1→MS1(MS1 即首个 post-V1 里程碑,账面承接不实现)。执行期按 14 §4 追加 RD-026+ 并双侧标注
deliverables:
  - id: D-MS1-1
    name: MS1.1 双 Full RFC——rfcs/0009-host-gpu-orchestration.md(std::gpu 面/C ABI 绑定/launch marshalling/PTX 嵌入/失败模式,§9 裁决清单)+ rfcs/0010-uc07-sim-renderer.md(应用形态/主语言判据操作化/golden 三层协议/防降级硬门,§9 裁决清单),串行合入先于实现(G-MS1-1 前置)
  - id: D-MS1-2
    name: MS1.2 single-source 宿主 GPU 编排 stdlib——spec/host_orchestration.md RXS-0189~0196 条款体 + rurixc host lowering(typeck 已知签名分支/mir_build 字面符号降级/driver 链接段+PTX 嵌入)+ src/rurix-rt-cabi(staticlib,rxrt_* C ABI,U25 unsafe-audit)+ 前端机械(extern 保名/#[link]/mod name;)+ conformance/host_orch 语料 + CI 步骤 52 + stable 快照重 bless(G-MS1-1/G-MS1-2)
  - id: D-MS1-3
    name: MS1.2b present typestate + 宿主图像落盘桥——spec RXS-0197~0199 + rurix-rt interop OwnedPresentSession 重构(scope()/uc03 零漂移)+ rxp_*/rxio_* C ABI + .rx typestate 面 + stable 快照重 bless(G-MS1-1)
  - id: D-MS1-4
    name: MS1.3 UC-07 应用 ruridrop——apps/ruridrop 全 .rx(SPH 10 kernel + DDA 渲染核 + pt/rt/refcpu 三入口 + rurix.toml)+ tests/uc07 golden manifest/bless_log + CI 步骤 53 离线 golden 门(G-MS1-3/G-MS1-4)
  - id: D-MS1-5
    name: MS1.4 实时 present 取证 + 性能预算回填——evidence/uc07_present_*.json(measured_local)+ ms1.bench.uc07_{sph_step_ms,pt_frame_ms,realtime_frame_ms} triple_run 回填(G-MS1-5/G-MS1-6)
acceptance_gates:
  - id: G-MS1-1
    check: "条款先行:RFC-0009/RFC-0010 Approved 合入先于对应实现 PR(10 §3 失败测试先行:步骤 52/53 脚本与 std::gpu lowering/apps/ruridrop 在提案时点 main 上不存在 = RED);spec/host_orchestration.md RXS-0189 续号条款体(FLS 体例,严禁 UB 节)与每条 ≥1 `//@ spec:` 锚定同 PR 落地,commit 序条款在前;trace_matrix --check 维持全锚定(沿用全局 m1.counter.spec_clause_test_anchoring,不另立锚定 counter);**stable 快照因条款增长同 PR 重 bless**(tests/stable/bless_log.md 同 diff 追加,RXS-0180 L2 加性演进,步骤 49 硬红不可分 PR;MS1 预期两次:MS1.2 与 MS1.2b)"
  - id: G-MS1-2
    check: "single-source 端到端:最小 .rx 单源程序(宿主编排 + kernel 同编译单元)经 rx build 产单 EXE → device 真跑 launch(RTX 4070 Ti)→ 数值对照通过;**防降级硬门**:宿主编排必须来自 .rx 源经 rurixc host codegen 产出——手写 Rust 宿主 harness / host-only 模拟 / 桩化 launch / SKIP 充绿均不得替代;CI 步骤 52 ci/host_orch_smoke.py 内建红绿闭合(篡改嵌入 PTX → 装载协商拒 RXS-0192;桩化 device 写回 → 数值对照红;复原绿,反 YAML-only)+ run URL 归档 §8;evidence/host_orch_smoke.json 经 schema 校验"
  - id: G-MS1-3
    check: "主语言判据(RFC-0010 操作化定义):apps/ruridrop 包内**零 .rs 源**——SPH kernel、渲染 kernel、宿主帧循环/资源编排/出图落盘全部 .rx,单包经 rx build 产 EXE;白名单 = 链接语言基础设施(rurix-rt / rurix-rt-cabi / rurix-d3d12 shim,等价于任何语言的运行时);机器审计(源清单零 .rs + 产物链路来自 rx build)为 CI 步骤 53 前置检查并写 evidence;不以任何 Rust/C++ 应用层胶水替代(防降级硬门,措辞镜像 G-G2-4)"
  - id: G-MS1-4
    check: "离线确定性 golden 三层(CI 步骤 53,device 真跑 RTX 4070 Ti):①硬门-确定性:固定初值/dt/seed/SPP 冒烟档跑 N 帧,同机两次运行逐帧量化 PPM 字节 SHA-256 逐字节一致;②硬门-参考容差:GPU 帧 vs CPU 参考(同一 .rx 共享 device fn 经 refcpu 入口 host 重放)量化域 ≥99.5% 像素每通道差 ≤1 LSB 且最大 ≤2 LSB;③软门-blessed 哈希:逐帧 SHA-256 == tests/uc07/golden_manifest(bless 受控,tests/uc07/bless_log.md 留痕,驱动升级触发重 bless);内建数据流红绿:篡改 kernel 物理/着色常数经同一编译链重编 → digest 变红,复原绿(镜像步骤 48 先例,仅「跑完了」不接受)+ run URL 归档 §8"
  - id: G-MS1-5
    check: "实时 present 取证(evidence 面,不进 CI 硬门):本机交互桌面以 ruridrop realtime 入口经 RFC-0001 interop typestate(RXS-0140~0143 fence 协议)真跑 ≥300 帧 → evidence/uc07_present_*.json(measured_local:帧数/采样像素对照/环境画像)+ 契约 §8 留痕;CI 无交互桌面 → 不设步骤(镜像 realtime_present_smoke 双态先例,SKIP 不充绿纪律)"
  - id: G-MS1-6
    check: "性能与收口:≥2 项 ms1.bench.* 预算条目以 measured_local 回填(BENCH_PROTOCOL triple_run trimmed mean;登记与 evaluator/entries 同 PR 落,不预造);close-out budget_eval --strict 全局零 estimated;「外部生产采纳」维持 carve-out 不宣称达成;close-out 全量回归冻结(cargo test / trace / snapshot / bilingual / guardrails 真实输出追加 §8)"
guardrails:
  - "milestones/m0~v1 的 measured_local 既有预算条目 git diff 0-byte(新增 ms1 条目允许,随 MS1.4 回填);ms1_budget.json 经 *_budget.json glob 自动纳入 + 命名空间强制前缀 ms1.(14 §3);counter/entries **不预造**——登记与 ci/budget_eval.py eval_counter 新分支同实现 PR 落(未知 id 强制 FAIL,预造即红)"
  - "milestones/m0~v1 的 *_CONTRACT.md(均 closed)只追加不修改(check_closed_contracts,glob 已泛化);本契约 close-out 翻 closed 后自动纳入字节守卫"
  - "registry/deferred.json 与 registry/spike_gating.json 只追加;RD-007/RD-009/RD-025 处置仅由 agent 自主签署留痕追加;SG 复评(全维持 not_triggered;SG-010 留续号,若出现「窗口/UI 框架进语言」或「通用异步宿主运行时」扩张诱惑则登记 gating 而非提案)只追加 decisions"
  - "registry/error_codes.json 错误码语义可加不可改;MS1 预期新码按段续号(1xxx/2xxx/3xxx/6xxx/7xxx,以实现实际为准,见 §7 ⑥),每码 en+zh messages 成对(bilingual 门 88→N)"
  - "evidence/ 只增不删不改(M0.3 起);tests/uc07/ 出图 golden 纳入 bless 纪律(golden_manifest + bless_log.md,数据行忌「日期」子串)"
  - "00–14 共 15 份规划文档(含 13_DECISION_LOG.md)不被执行 PR 改写(check_planning_docs);开工裁决记本契约 §7;02 号文档不因 UC-07 改写(§7 ④);11 §6 落地标注(可选)走 00 §6.3 独立勘误 PR 且在 close-out 后"
  - "**stable API 快照变更必经 bless**(check_stable_snapshot_bless):MS1 预期两次触发(MS1.2 条款 RXS-0189~0196 → 184→192;MS1.2b RXS-0197~0199 → 192→195),各与条款/实现同 PR(步骤 49 硬红,不可分 PR)"
  - "tests/ui .stderr / tests/mir .mir / tests/ptx .nvptx / tests/dxil golden 变更必经审批 bless(既有机制);MS1 预期新增 UI golden(host_orch reject 语料)经同款 bless 纪律"
  - "全仓 crate 维持 unsafe_code=deny;**src/rurix-rt-cabi 为 FFI 边界例外**:新 unsafe 须逐处 // SAFETY: + unsafe-audit U25 续号登记(镜像 rurix-interop/rurix-d3d12 先例)"
  - "guardrail 回退基准默认 = v1-closed(V1 close-out 已切;PR 路径以 GITHUB_BASE_REF 为准);MS1.5 close-out 时切至 ms1-closed(agent 自主签署;ms1-closed 不匹配 release.yml 收窄触发器 v[0-9]+.[0-9]+.[0-9]+*,零误触发)"
  - "仓库 LF byte-exact(* -text):新文件 LF + 尾换行,禁 Python 文本模式写文件;registry/*.json 等既有 CRLF 例外文件追加行保持原行尾风格、既有行 0-byte"
  - "spec 修订表表头维持「版本」列名,数据行避「版本」子串(用「版号」);本契约 in_scope/acceptance_gates 等既有条款 0-byte 修改,close-out 只追加 §8;status active→closed 翻转 / 基准切换 / ms1-closed tag / RD·SG 处置由 agent 自主签署"
---

# MS1 契约 — 使命判据落地第一期：single-source 宿主 GPU 编排 + UC-07 主语言渲染器/仿真二合一应用

> 所属:[../../01_VISION_AND_MISSION.md](../../01_VISION_AND_MISSION.md) §6 使命成功层 / [../../11_ROADMAP.md](../../11_ROADMAP.md) §6 五年愿景第一条 / 契约机制见 [../../14_ENGINEERING_DISCIPLINE.md](../../14_ENGINEERING_DISCIPLINE.md) §1。
> 规范先行延续(AGENTS.md 硬规则第 7 条):语义面 PR 必须引用 RXS-#### 条款号;缺条款先补 spec,条款 commit 先于实现 commit。
> 基准 ref:**默认 `v1-closed`**(V1 close-out 已切换;`ci/check_guardrails.py` 无参默认 = `v1-closed`,PR 路径以 `GITHUB_BASE_REF` 为准)。
> 粒度:**单 MS1 阶段契约**:一份契约覆盖 MS1 期,MS1.1~MS1.5 子里程碑分解见 [MS1_PLAN.md](MS1_PLAN.md)(对齐 M*/G1/G2/V1「每里程碑一份契约 + 内部子里程碑」范式)。
> **定位口径:MS1 是使命判据(01 §6 第三层)的第一期操作化落地,不是其整体达成。**现状:kernel 已全 .rx,但宿主编排一律 Rust/C++(uc03/uc04/rurix-engine/GRX 均为宿主承载 + Rurix 当 compute pass = bolt-on);launch 仅类型面(RXS-0074/0075)无执行语义。MS1 按用户 2026-07-14 三裁定(§7 ③)把「以 Rurix 为主语言」按最严口径操作化:宿主编排也须 .rx——落 single-source 宿主 GPU 编排语言特性(RFC-0009)+ 首个全 .rx 生产级渲染器/仿真二合一应用(RFC-0010)。「外部生产采纳」维度显式 carve-out(out_of_scope)。
> **脚手架口径:本契约为 MS1 开工结构件,不实现任何语义面、不落条款、不打 tag;§8 close-out 开工时为空。**

---

## 1. 目标

MS1 期结束时项目获得:① host .rx 可经 `std::gpu` 首期收敛子集(Context/Stream/Buffer/launch/同步)编排 GPU,同源 kernel PTX 嵌入单 EXE,`rx build` 一步出可执行应用(RFC-0009);② 首个以 Rurix 为主语言(应用层零 .rs)的生产级渲染器/仿真二合一应用 **ruridrop**(GPU SPH 溃坝 + 路径追踪/光线投射,apps/ruridrop),离线确定性 golden 进 CI(步骤 53)、实时 D3D12 present 本机取证(RFC-0010);③ 主语言判据被操作化为机器可审计门(G-MS1-3)。CUDA/PTX compute 路线,不触多后端红线、不走 DXIL 图形路。

## 2. 范围

### 2.1 in-scope

| 项 | 说明 | gating | 对应交付物 |
|---|---|---|---|
| host_gpu_orchestration_stdlib | single-source 宿主 GPU 编排 stdlib(std::gpu 首期收敛子集 + rurix-rt-cabi C ABI + PTX 嵌入/装载协商 + 前端机械面) | **Full RFC 前置(RFC-0009)**;条款先行 RXS-0189 续号 | D-MS1-2 |
| uc07_present_imageio_face | present 宿主 typestate 面 + 宿主图像落盘桥(D-130 shim 边界 0-byte) | RFC-0009 §4 承载;条款 RXS-0197~0199 | D-MS1-3 |
| uc07_sim_renderer_app | UC-07 应用 ruridrop:GPU SPH + 二合一渲染,应用层全 .rx | **Full RFC 前置(RFC-0010)**;依赖 D-MS1-2/3 | D-MS1-4 |
| offline_golden_gate | 离线确定性出图 golden 门(三层,CI 步骤 53,device 真跑) | 随 RFC-0010 §5 | D-MS1-4 |
| realtime_present_evidence | 实时 present 取证(evidence 面)+ 性能预算回填 | 随 RFC-0010 §5;不进 CI 硬门 | D-MS1-5 |

### 2.2 out-of-scope(显式排除)

见 YAML 头 `out_of_scope` 字段逐项(multi_backend / dxil_graphics_route / tensor_core_intrinsics / interactive_windowed_app / pt_completeness / registry / grx_merge / production_adoption_claim / rd_implementation);11 §2 红线不触碰。

## 3. 交付物清单

| ID | 交付物 | 形态 | 完成判据 |
|---|---|---|---|
| D-MS1-1 | 双 Full RFC | rfcs/0009-host-gpu-orchestration.md + rfcs/0010-uc07-sim-renderer.md + rfcs/README §5 台账 | Approved 合入先于实现(G-MS1-1 前置) |
| D-MS1-2 | single-source 宿主编排 stdlib | spec/host_orchestration.md RXS-0189~0196 + rurixc lowering + src/rurix-rt-cabi + conformance/host_orch + ci/host_orch_smoke.py(步骤 52) | G-MS1-1 / G-MS1-2 |
| D-MS1-3 | present typestate + 落盘桥 | spec RXS-0197~0199 + OwnedPresentSession + rxp_*/rxio_* + .rx 面 | G-MS1-1 |
| D-MS1-4 | UC-07 应用 + 离线 golden 门 | apps/ruridrop(全 .rx)+ tests/uc07/ + ci/uc07_offline_golden_smoke.py(步骤 53) | G-MS1-3 / G-MS1-4 |
| D-MS1-5 | present 取证 + 性能回填 | evidence/uc07_present_*.json + ms1_budget.json entries(measured_local) | G-MS1-5 / G-MS1-6 |

## 4. 验收门(完整版,YAML 头为可提取摘要)

见 YAML 头 `acceptance_gates` 字段 G-MS1-1 ~ G-MS1-6。要点:
- **G-MS1-1(条款先行)**:双 RFC 前置;条款体+锚定+快照重 bless 同 PR(步骤 49 硬红,MS1 预期两次 bless:184→192→195)。
- **G-MS1-2(single-source 端到端)**:防降级硬门——宿主编排必须来自 .rx 源经 rurixc host codegen;篡改 PTX / 桩化双红绿。
- **G-MS1-3(主语言判据)**:apps/ruridrop 零 .rs 机器审计;白名单 = 语言基础设施链接。
- **G-MS1-4(golden 三层)**:确定性硬门 + CPU 参考容差硬门 + blessed 哈希软门;数据流红绿(篡改物理常数 → digest 变红)。
- **G-MS1-5(present 取证)**:evidence 面 measured_local,不进 CI 硬门,SKIP 不充绿。
- **G-MS1-6(性能与收口)**:≥2 项 ms1.bench.* measured_local;close-out --strict 零 estimated;外部采纳 carve-out 维持。

## 5. Guardrails(字节级,机器核对)

见 YAML 头 `guardrails` 字段。核对方式:`py -3 ci/check_guardrails.py`(无参默认基准 = `v1-closed`;PR 路径以 `GITHUB_BASE_REF` 为准)。

## 6. Deferred 引用

| 编号 | 内容摘要 | 承接 |
|---|---|---|
| RD-007 | const 泛型值运行期单态化(turbofish const 实参 → 实例值代入 + codegen) | inherited,owner_milestone V1→MS1 顺延;**MS1 有真实评估点**——ruridrop kernel 若用 turbofish const 实参则接通,否则继续留痕;非开工验收门,执行期处置 |
| RD-009 | `#[export(c)]` C ABI 导出属性 + 内建头文件生成 codegen | open,owner_milestone V1→MS1 顺延;MS1 的 C ABI 绑定方向为**导入**(host .rx 消费 rxrt_* 运行时符号,RXS-0125 手写 extern "C" 口径 0-byte),不触发导出 codegen 硬需求;账面承接不实现,非验收门 |
| RD-025 | rurixup 真实 FS 物化 + 网络拉取 | open,owner_milestone post-V1→MS1(MS1 即首个 post-V1 里程碑);MS1 无 rurixup 工作,账面承接不实现,非验收门,close-out carry-forward |

详情以 [../../registry/deferred.json](../../registry/deferred.json) 为唯一事实源,本表仅引用。执行期按 14 §4 追加 RD-026+(std::gpu 首期子集外宿主编排面)/ RD-027(交互实时模式)/ RD-028(路径追踪完备性)并双侧标注。

## 7. 修订记录 / 开工裁决留痕

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-07-14 | 初版契约固化(MS1 开工脚手架)。**开工裁决**(用户 2026-07-14 经 AskUserQuestion 三项裁决 + agent 完全自主判档 D-406 v2.0 / AGENTS v3.0 硬规则 1,记于本节;13_DECISION_LOG 执行 PR 字节冻结,不改决策日志):① **新里程碑 = milestones/ms1/(Mission Stage 1)**,namespace `ms1.`,收口 tag `ms1-closed`(agent 裁决;v 前缀绑定 SemVer 发行语义、m 系 MVP 已收官、g 系 = D-002 图形三阶段 G0/G1/G2 无 G3、a 前缀与语义公理 A-xx 撞号——ms 直指 01 §6 使命成功层,为后续使命系工作留序列;ms1-closed 不匹配 release.yml 收窄触发器,零误触发)。② **子里程碑 = MS1.1 双 Full RFC → MS1.2 single-source 宿主编排(含 MS1.2b present/落盘面)→ MS1.3 UC-07 应用 + 离线 golden 门 → MS1.4 present 取证 + 性能回填 → MS1.5 close-out**,严格串行(MS1_PLAN.md;MS1.1 内两 RFC 可并行起草、串行合入)。③ **用户三裁定**(2026-07-14 AskUserQuestion):主线 = 渲染器+仿真二合一;呈现 = 离线 golden 进 CI + 实时 D3D12 present;主语言判据 = **最严口径:宿主编排也须 .rx**(判据操作化细节归 RFC-0010 §9)。④ **UC-07 编号 claim**:承 02 §4 UC-01~06 序列续号,自本契约 claim,永不复用(10 §9.5);02 号文档为冻结规划档案(00 §6.4 只接受勘误),新增用例超出 00 §6.3 勘误射程,**不改写 02**——UC-07 上游依据直接锚 01 §6 / 11 §6 既有判据行。⑤ **判档**:host_gpu_orchestration_stdlib = **Full RFC(RFC-0009)**(触运行时语义 + FFI ABI,硬规则 5 / 10 §3);uc07_sim_renderer_app = **Full RFC(RFC-0010)**(使命级验收载体 + 主语言判据操作化,对齐 UC-04=RFC-0006 先例);脚手架本身 rfc_required: none(结构件,对齐 M4~V1 先例);应用走 CUDA/PTX compute 路线(完整语言表达力),显式不走 DXIL 图形路(RXS-0171 body 白名单阻断生产着色器,非本期修复面)。⑥ **续号 claim**(编号永不复用,10 §9.5):Full RFC = **RFC-0009/RFC-0010**;RXS 条款自 **RXS-0189** 起(0181~0184 GRX 分支占用维持跳号,0185~0188 已用),预期 MS1.2 = RXS-0189~0196、MS1.2b = RXS-0197~0199,新 spec 文件 spec/host_orchestration.md;新 deferred 自 **RD-026** 起;SG 续号 **SG-010** 留用;unsafe-audit 单元 **U25**(rurix-rt-cabi);新错误码按段续号(预期 RX1005 模块装配 / RX2010 元素推断 / RX3015 宿主 API 着色 / RX6024 marshalling 子集 / RX6025 嵌入失败 / RX7021 cabi 定位 / RX7022 #[link] 定位,以实现实际为准,en+zh 成对);CI 步骤 = **52/53**;应用 = **apps/ruridrop**(新顶层 apps/,Rurix 包不进 cargo workspace);D-xxx 段位裁决记双 RFC §9 + 本契约 §7,不动 13 号文档(勘误型回填若需走 00 §6.3 独立 PR)。⑦ **GRX 不合入 main**(MS1 全期):RXS-0181~0184/MR-0006/0007 撞号与 stable 快照面串行化;例外合入则 MS1.2/MS1.2b rebase 后按合并面重 bless。⑧ **deferred 承接**:RD-007(inherited)/RD-009(open)owner_milestone V1→MS1 顺延、RD-025(open)post-V1→MS1(deferred.json v1.48 留痕);RD-007 在 MS1 有真实评估点(ruridrop kernel 的 turbofish const 实参),执行期评估接通,非开工验收门。⑨ **红线/SG 复评**:D-008 多后端红线不解除(CUDA/PTX 路线 = NVIDIA 纵深);SG-001/002/003/004/005/007/008/009 维持 not_triggered;SG-010 留续号(「窗口/UI 框架进语言」或「通用异步宿主运行时」扩张诱惑出现时登记 gating 而非提案);D-312 registry 维持休眠;D-130 present shim 边界 0-byte(窗/泵/交换链不进语言,.rx 面经 rxp_* C ABI 消费)。⑩ **present/离线双态**:离线 golden 为 CI 硬门(runner 具真 GPU,步骤 46~48 先例);实时 present 为 evidence 面(CI 无交互桌面,SKIP 不充绿纪律,镜像 realtime_present_smoke 双态)。⑪ **诚实边界**:MS1 达成表述 =「首个以 Rurix 为主语言的生产级渲染器/仿真系统落地(第一方)」;01 §6 使命成功层的「外部选择/采纳」维度显式 carve-out,不宣称达成;close-out 后可选 11 §6 落地标注走 00 §6.3 独立勘误 PR(诚实措辞)。**MS1 close-out 关闭判定 / 基准切换(v1-closed→ms1-closed)/ ms1-closed tag / RD·SG 处置由 agent 自主签署** |

---

## 8. Close-out(只追加区 — 开工时为空)

<!-- 验收记录、guardrail 核对输出、MS1.1~MS1.4 端到端留痕(双 RFC / 步骤 52/53 run URL / golden bless / present evidence / 性能回填)、RD-007/RD-009/RD-025 处置留痕、SG 复评结论追加于此;上方条款 0-byte 修改。MS1 close-out 关闭判定 / 基准切换(v1-closed→ms1-closed)/ ms1-closed tag / RD·SG 处置由 agent 自主签署兑现。 -->

### 8.1 MS1.1 验收留痕(2026-07-14,D-MS1-1 / G-MS1-1 前置)

agent 完全自主签署(AGENTS v3.0 硬规则 1 / D-406 v2.0),记录机器事实:

- **双 Full RFC 合入先于实现**:RFC-0009(single-source 宿主 GPU 编排 std::gpu,Agent Approved 2026-07-14)经 PR [#130](https://github.com/qwasg/Rurix/pull/130) 合入(pr-smoke [run 29339267711](https://github.com/qwasg/Rurix/actions/runs/29339267711));RFC-0010(UC-07 ruridrop + 主语言判据操作化,Agent Approved 2026-07-14)经 PR [#131](https://github.com/qwasg/Rurix/pull/131) 合入(pr-smoke [run 29341689141](https://github.com/qwasg/Rurix/actions/runs/29341689141))。失败测试先行成立:两 RFC 合入时点,`ci/host_orch_smoke.py` / `ci/uc07_offline_golden_smoke.py` / `src/rurix-rt-cabi` / `apps/ruridrop` 在 main 上均不存在 = RED。
- 脚手架 PR [#129](https://github.com/qwasg/Rurix/pull/129)(pr-smoke [run 29338179327](https://github.com/qwasg/Rurix/actions/runs/29338179327))先行落契约四件套 + RD-007/009/025 顺延(deferred v1.48)。

### 8.2 MS1.2 / MS1.2b 验收留痕(2026-07-14/15,D-MS1-2 / D-MS1-3;G-MS1-1 / G-MS1-2)

- **G-MS1-1 条款先行**:spec/host_orchestration.md RXS-0189~0196(PR [#132](https://github.com/qwasg/Rurix/pull/132),commit 序条款在前)+ RXS-0197~0199(PR [#133](https://github.com/qwasg/Rurix/pull/133))落体,每条 ≥1 锚定,trace 184→192→**195/195** 全锚定;stable 快照两次同 PR 重 bless(184→192→195,tests/stable/bless_log.md 两行留痕,RXS-0180 L2 加性演进);7 新码 RX1005/RX2010/RX3015/RX6024/RX6025/RX7021/RX7022 en/zh 成对(bilingual 95/95);unsafe-audit **U25**(src/rurix-rt-cabi,FFI 边界例外,逐处 // SAFETY:)。
- **G-MS1-2 single-source 端到端**:conformance/host_orch/accept/saxpy_single_source(宿主编排 + kernel 同编译单元)经 `rx build` 产单 EXE(PTX + sm_89 cubin 真预编嵌入,DeviceArtifactSet 装载协商复用)→ RTX 4070 Ti 真跑数值自校验 exit 0;防降级硬门兑现:宿主编排全部来自 .rx 源经 rurixc host codegen(CallTarget::Rt 字面符号 → rxrt_* C ABI → rurix-rt);CI 步骤 52 内建双红绿(篡改嵌入 PTX → 装载协商拒 `RXRT: error` / 桩化 kernel 写回 → 数值红 / 复原绿)。PR #132 pr-smoke [run 29381369800](https://github.com/qwasg/Rurix/actions/runs/29381369800)、PR #133 [run 29384566787](https://github.com/qwasg/Rurix/actions/runs/29384566787) 全量 success(runner 真 GPU,RURIX_REQUIRE_REAL=1)。
- MS1.2b:present 宿主 typestate 面(OwnedPresentSession 下沉 + rxp_* 七符号 + .rx 消费式四态,错序 = 编译期 move 违例;D-130 shim 边界 0-byte,uc03 回归网零漂移)+ 宿主图像落盘桥(rxio_write_ppm,与 image-io 逐字节一致);步骤 52 扩面(八 reject / 五 accept / imageio device 真跑)。RD-026 执行期登记(deferred v1.49)。
- 附带运维留痕:runner 僵尸 exe(nightly 遗留 async_buffer_pipeline.exe 内核态锁)经用户授权击杀 + Move-Item 隔离区解锁,PR #132 首跑 checkout EPERM 红 → 隔离后 rerun 全绿(run 29381369800 为 rerun)。

### 8.3 MS1.3 验收留痕(2026-07-15,D-MS1-4;G-MS1-3 / G-MS1-4)

- **G-MS1-3 主语言判据(RFC-0010 §4.1 四条,机器审计)**:apps/ruridrop 文件集 = 13 个 .rx + rurix.toml,**零 .rs/.cpp/.c/.py**(步骤 53 前置审计严格白名单);GPU kernel(14 处 kernel fn)与宿主编排/帧循环/落盘同包,`rx build` 单 EXE;链接白名单 = 语言基础设施(rurix-rt / rurix-rt-cabi / rurix-d3d12 shim / image-io 经 RFC-0009 C ABI 面);防降级硬门无替代物。
- **G-MS1-4 离线 golden 三层(步骤 53,device 真跑)**:① 确定性硬门——冒烟档(160×120/8spp/2 帧/N=4096)同机两跑逐帧 SHA-256 逐字节一致;② 参考容差硬门——GPU 帧 vs refcpu(同一 .rx device fn host 单线程同构重放)**逐字节全等**(|Δ|≤1 占比 100%,max=0;门限 ≥99.5% / ≤2);③ blessed 哈希软门——tests/uc07/golden_manifest(05b59ff2… / e9c2c2c2…)+ bless_log 首次 bless 留痕;④ 数据流红绿——篡改 GRAVITY 10.0→2.5 同链重编 → 双帧 digest ≠ golden → 原树 0-byte 复原绿。PR [#134](https://github.com/qwasg/Rurix/pull/134) pr-smoke [run 29386668923](https://github.com/qwasg/Rurix/actions/runs/29386668923) 全量 success。
- 仿真确定性由构造保证(bit-split 基数排序 atomics-free + 固定序累加,SIM_HASH 三跑同值);RD-007 评估点兑现:kernel 全标量实参,未用 turbofish const,未触发接通(§6 处置)。

### 8.4 MS1.4 验收留痕(2026-07-15,D-MS1-5;G-MS1-5 / G-MS1-6)

- **G-MS1-5 实时 present 取证(evidence 面,measured_local)**:本机交互桌面真窗口 600 帧(present-real shim,d3d12-interop-real,~68fps),末帧普通 Buffer 采样对照 sample_ok=true(天空区 (159,166,175) 与梯度公式逐点吻合 + 水体区蓝主导核验)→ [evidence/uc07_present_20260715.json](../../evidence/uc07_present_20260715.json)(环境画像 driver 620.02 / CUDA 13.2)。不进 CI 硬门(双态先例,SKIP 不充绿)。
- **G-MS1-6 性能预算**:三项 ms1.bench.* 以 measured_local 回填(triple_run trimmed mean,锁频 -lgc 2610/-lmc 10501):uc07_sph_step_ms **11.46ms**(≤17.19,N=131072 sim-only 差分法)/ uc07_offline_frame_s **0.116s**(≤0.174,720p×8 帧全序列)/ uc07_realtime_frame_ms **14.83ms**(≤33.3 = 30fps 档)。budget_eval --strict **74 pass 零 estimated**。
- **诚实发现 RD-027(deferred v1.50)**:生产档 256spp/4 弹射触工具链毒径挂起(疑 rurixc PTX 发散重汇聚缺陷,二分实录);处置 = params.rx 生产档暂锁已验证切片 32spp/2 弹射(STUB(RD-027) 双侧标注,offline 端到端 8 帧 1.17s 复验绿)+ RFC-0010 修订表 Q-AppScope 切片留痕行 + bench 脚本切片容错;修复后回填 256/4 重测,不静默放宽。PR [#135](https://github.com/qwasg/Rurix/pull/135) pr-smoke [run 29391498364](https://github.com/qwasg/Rurix/actions/runs/29391498364) 全量 success。

### 8.5 MS1 整体 close-out 终审(2026-07-15,agent 完全自主签署)

**全量回归冻结(本机真实输出)**:`cargo test --workspace` **79 套件 0 failed**;`trace_matrix --check` **195/195** 全锚定;`stable_snapshot --check` **195/95/["2026"]/8**;`bilingual_coverage` **95/95**;`budget_eval --strict` **74 pass, 0 skip, strict mode**(零 estimated);`check_schemas` PASS;`check_guardrails`(基准 v1-closed)PASS;步骤 49 红绿闭合复绿。

**验收门终审表**:

| 门 | 判据 | 结论 |
|---|---|---|
| G-MS1-1 | 双 RFC 前置 + 条款先行 + 全锚定 + 两次同 PR 重 bless | ✅(§8.1/§8.2) |
| G-MS1-2 | single-source .rx → 单 EXE → device 真跑数值对照 + 防降级硬门 + 双红绿 | ✅(§8.2) |
| G-MS1-3 | 应用零 .rs 机器审计 + 同包 + 基础设施白名单 | ✅(§8.3) |
| G-MS1-4 | 离线 golden 三层 + 数据流红绿 | ✅(§8.3,②门以逐字节全等超额达成) |
| G-MS1-5 | present evidence(measured_local,不进 CI 硬门) | ✅(§8.4) |
| G-MS1-6 | ≥2 项 ms1.bench.* measured_local + --strict 零 estimated + carve-out 维持 | ✅(§8.4;三项回填) |

**deferred 处置**(registry/deferred.json v1.51,§6 对应):RD-007 维持 inherited(评估点兑现未触发,待后续阶段顺延)/ RD-009、RD-025 维持 open carry-forward / RD-026 维持 open(首期子集全程够用)/ RD-027 维持 open(切片处置闭环,工具链缺陷调查归后续)。**SG 复评**(spike_gating.json v1.5):SG-001~005/007~009 全维持 not_triggered;SG-010 全期未触发留续号;D-008 多后端红线 3 不解除(UC-07 CUDA/PTX 落地 = NVIDIA 纵深推进)。

**使命判据表述(诚实边界,§7 ⑪)**:MS1 达成 =「**首个以 Rurix 为主语言的生产级渲染器/仿真系统(第一方)落地**」——ruridrop:应用层零 .rs、宿主编排/kernel/帧循环/落盘全 .rx、单 EXE、确定性三层 golden、实时 ~68fps@1280×720/131k 粒子、性能 measured_local。01 §6 使命成功层的「外部选择/采纳」维度维持 carve-out,**不宣称使命判据整体达成**;后续外部项目采纳属社会判据,时间驱动。

**签署兑现**:本契约 `status: active → closed`;`ci/check_guardrails.py` 回退基准默认 `v1-closed → ms1-closed`;合入后推 annotated `ms1-closed` tag(不匹配 release.yml 收窄触发器,零误触发);双基准 advisory 复核(v1-closed PASS + ms1-closed PASS)输出随 close-out PR 验证记录。MS1 期正式关闭;post-MS1 里程碑由后续裁决另立,本契约不预造。(可选尾巴:11 §6 落地标注走 00 §6.3 独立勘误 PR,诚实措辞,与执行 PR 分离。)
