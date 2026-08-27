---
contract: G33
title: G33 商业化期（波 C 批次：渲染器 SDK 稳定 API 面/文档与示例/兼容矩阵与降级链/运行时健壮性/分发打包/许可终审/profiling 工具面/支持政策 + NGX 分解 + RD-027 守护 + P4 四行/HLOD L4/SVT/KTX2/RT pipeline 长线窗 + 六窗重判 + 十二阻塞探针 + 波 C 验收门 C18）
status: active
implementation_status: unlocked
active_scope: g33_wave_c_commercialization
version: v1.0
date: 2026-08-26
timebox: "波 C 批次 = g30-closed 后 G31+ 战役第三波（C1~C17 交付 + C18 最终验收）；#56 外部采纳使命判据维持未宣称（carve-out 字面 0-byte）；RD-015 重判窗（llvm#57928 closed 信号在案）待启动归后续期"
rfc_required: "波 C 批次一件 Full RFC——RFC-0048 RT pipeline + SBT 宿主车道（C15，Agent Approved 2026-08-25，D-409 第 1 轮 8 findings 全 disposition；number_ledger RFC on_tree_max 47→48、next_free 48→49）；其余面全部消费既有语义面与冻结面（export_c codegen RD-008/RD-009 机制、EA1 分发链 RXS-0214~0218、G26~G29 device kernel、M50 RT 底座、MR-0011 PTXAS_OPT 护栏既有 Approved 面）；零新 RXS 条款（spec/release.md RXS-0214 同条修订零新条款 ID）；共享编号段落盘前实测：RXS next_free=408 / RD next_free=46 / CI_step next_free=525 / U next_free=60（C1 消费 U-59）全维持"
upstream_docs:
  - "milestones/g30/g30_campaign_handover_registry.json（RFC-0047 §5.5：G31+ 唯一法定输入面）"
  - "G31_PLUS_COMMERCIAL_RENDERER_TODO.md §5 #48~#56（商业化工程面）+ §3 #20~#32（P2 长线窗）+ §6 波次线 3（G33+ = 商业化期定义）"
  - "registry/deferred.json（RD-027/RD-036/RD-039/RD-040/RD-026/RD-008/RD-045/RD-015/RD-033 承接面）"
in_scope:
  - g33_wave_c_renderer_sdk（C1，TODO #48）
  - g33_wave_c_renderer_docs（C2，#49）
  - g33_wave_c_capability_fallback（C3，#50）
  - g33_wave_c_robustness（C4，#51）
  - g33_wave_c_sdk_dist（C5，#52）
  - g33_wave_c_vendor_license（C6，#53）
  - g33_wave_c_profiling（C7，#54）
  - g33_wave_c_support_policy（C8，#55）
  - g33_wave_c_ngx_decomposition（C9，#14 承接锚兑现形态）
  - g33_wave_c_rd027_guard（C10，#16 落档绕行）
  - g33_wave_c_p4_streaming（C11，#20~#23）
  - g33_wave_c_hlod_l4（C12，#25）
  - g33_wave_c_svt（C13，#33~#35）
  - g33_wave_c_ktx2（C14，#37~#39）
  - g33_wave_c_rt_pipeline（C15，#31/#32）
  - g33_wave_c_rejudgment（C16，#24/#26~#29/#43 六窗）
  - g33_wave_c_blocked_probes（C17，阻塞项全量新鲜探针）
  - g33_wave_c_acceptance_gate（C18，本波验收）
out_of_scope:
  - external_adoption_claim（#56 外部采纳使命判据维持未宣称——05 年愿景 carve-out 字面 0-byte，本波零外部项目采纳证据不预支）
  - bistro_exterior_scene_arm（#11 维持 G10-N6 锚挂起）
  - hdr_display_chain_implementation（M118-hdr-cal maintain-SDR 字面维持）
  - rd027_fix_masquerade（RD-027 = 落档绕行非修复确证，维持 open；修复 = 上游 NVIDIA 本体，DRAFT 备包 do-NOT-file owner 复核门）
  - rewriting_g13_g32_frozen_registries_or_anchors
  - presented_fps_masquerading_as_real_render_fps（双口径分离）
  - gap_01_03_closure_masquerade（GAP-01~03 许可义务三件维持 open；附带义务未闭前不以对应形态发布——release_checklist 在案）
deferred_refs: [RD-027, RD-036, RD-039, RD-040, RD-026, RD-008, RD-045, RD-015, RD-033, RD-032]
deliverables:
  - id: D-G33-1
    name: 波 C 交付十七项（C1~C17 harness + smoke + evidence schema/登记件，均 gate PASS 或如实登记在案）
  - id: D-G33-2
    name: 波 C 验收门（C18：终验三面复跑 + 发布件核验 + 全量回归 + soak 汇总 + 零降级三面终判 + 战役总登记）
  - id: D-G33-3
    name: G33 四件套（PLAN/CONTRACT/CI_GATES/g33_budget.json，零 estimated 全 measured）
  - id: D-G33-4
    name: G31+ 战役总登记（milestones/g31_plus_campaign_record.md——三波 56 项 TODO 逐项终态映射）
acceptance_gates:
  - id: G-G33-1
    check: "终验三面复跑：C1 SDK 宿主真跑 digest==Stage A 锚（g31.waveC.sdk PASS）+ C5 离线可建链 digest==锚（g31.waveC.dist PASS，签名/SBOM/红臂四路 + EA1 回归绿）+ C2 文档门（g31.waveC.docs PASS）"
  - id: G-G33-2
    check: "发布件核验：g31.waveC.license PASS（16 项 cleared 15/conditional 1）+ GAP-01~03 处置状态核验维持 open + release_checklist『附带义务未闭前不以对应形态发布』口径在案"
  - id: G-G33-3
    check: "全量回归：守卫套件五条 exit 0（budget_eval --strict 零 estimated）+ 波 A 五门 + 波 B 五门 + 波 C 全门 --gate 新鲜复跑（长门如 soak 引用在案）——26 PASS + 2 诚实红终态如实登记（ngx_decomp ratio 轨迹面 / waveB texture 跨波工作树机核面）"
  - id: G-G33-4
    check: "三面锚：Stage A digest 18/18 canonical 重跑零漂移 + G16plus M-g 18/18 canonical 复跑 VERDICT=PASS + G17-MD-F1 焦点格新鲜多样本中位如实登记（诚实红维持/恶化均合法终态，禁冒充）"
  - id: G-G33-5
    check: "soak 汇总：波 A 10010 帧在案 + C4 故障臂在案 1010 帧引用 + 波 C SDK 面增量 soak ≥1000 帧零崩"
  - id: G-G33-6
    check: "G33 四件套落盘 + G31+ 战役总登记（56 项逐项终态映射：兑现门 evidence 指针 / 维持 open 锚 / 诚实红项）+ §8 close-out 实测 facts + 零降级三面终判结论"
guardrails:
  - "诚实登记不冒充：所有数字来自真实命令输出；达标/维持/诚实红均合法终态"
  - "append-only：evidence/ 只增不删不改；deferred history 只追加；既有锚/注册表 0-byte 不回写"
  - "双口径分离：presented 帧率（含 FG 生成帧）与 real_render 帧率独立登记，禁混入"
  - "三态纪律：dev-env 降级 = SKIP 如实登记，禁冒充 PASS；RURIX_REQUIRE_REAL=1 翻硬 FAIL"
  - "未 commit 纪律维持：本波验收不改 commit 状态；commit 带 Assisted-by: trailer 且不 push"
---

# G33 契约 — 商业化期（波 C 批次范围）

> 所属：[../11_ROADMAP.md](../11_ROADMAP.md) §3 / G31+ 期待办总表 [../G31_PLUS_COMMERCIAL_RENDERER_TODO.md](../G31_PLUS_COMMERCIAL_RENDERER_TODO.md) §6 波次线 3；契约机制见 14 §1。front matter 双状态机：`status` 与 `implementation_status` 严格分离。

---

## 1. 目标

波 C 批次结束时，项目获得：**外部游戏引擎/项目可安全采纳 Rurix 渲染器的工程面**——渲染器 SDK 稳定 C ABI（9 函数 + stable 快照守卫第五段 + 外部 C++ 宿主真跑对拍 Stage A 锚）、集成文档与最小示例（新用户 <1 天最小集成）、设备兼容矩阵与六链 fail-closed 降级链、运行时健壮性故障注入面、16 组件签名/SBOM 分发 bundle 离线可建链、16 项 vendor 许可矩阵、对外 profiling/调试工具面、支持渠道与版本政策；同窗兑现 NGX 分解 profiling（G17-MD-F1 承接锚）、RD-027 毒区测绘与 fail-closed 守护、cluster 流送 P4 四行、HLOD L4 Far Field、SVT/KTX2 各三行、RT pipeline + SBT 宿主车道（RFC-0048）、六窗重判批量执行与十二阻塞探针全量新鲜复核；且全部既有画质/性能/确定性锚零降级（机器核验在案，诚实红面如实登记不冒充）。

## 2. 范围

### 2.1 in-scope（波 C 批次）

| 项 | 说明 | 对应交付物/门 |
|---|---|---|
| g33_wave_c_renderer_sdk | SDK 稳定 API 面（§5 #48） | D-G33-1 / g31.waveC.sdk |
| g33_wave_c_renderer_docs | 文档与示例（#49） | D-G33-1 / g31.waveC.docs |
| g33_wave_c_capability_fallback | 兼容矩阵与降级链（#50） | D-G33-1 / g31.waveC.capability |
| g33_wave_c_robustness | 运行时健壮性（#51） | D-G33-1 / g31.waveC.robustness |
| g33_wave_c_sdk_dist | SDK 分发打包（#52） | D-G33-1 / g31.waveC.dist |
| g33_wave_c_vendor_license | 许可合规终审（#53） | D-G33-1 / g31.waveC.license |
| g33_wave_c_profiling | profiling/调试工具面（#54） | D-G33-1 / g31.waveC.profiling |
| g33_wave_c_support_policy | 支持渠道与版本政策（#55） | D-G33-1 / g31.waveC.support |
| g33_wave_c_ngx_decomposition | NGX 分解（#14 承接锚） | D-G33-1 / g31.waveC.ngx_decomp |
| g33_wave_c_rd027_guard | RD-027 毒区守护（#16） | D-G33-1 / g31.waveC.rd027 |
| g33_wave_c_p4_streaming | cluster 流送 P4 四行（#20~23） | D-G33-1 / g31.waveC.p4stream |
| g33_wave_c_hlod_l4 | HLOD L4 Far Field（#25） | D-G33-1 / g31.waveC.hlodl4 |
| g33_wave_c_svt | SVT 三行（#33~35） | D-G33-1 / g31.waveC.svt |
| g33_wave_c_ktx2 | KTX2 三行（#37~39） | D-G33-1 / g31.waveC.ktx2 |
| g33_wave_c_rt_pipeline | RT pipeline + SBT（#31/#32） | D-G33-1 / g31.waveC.rtpipeline |
| g33_wave_c_rejudgment | 六窗重判（#24/#26~29/#43） | D-G33-1 / g31.waveC.rejudgment + g31.waveC.meshbench |
| g33_wave_c_blocked_probes | 十二阻塞探针 | D-G33-1 / g31.waveC.blockedprobes |
| g33_wave_c_acceptance_gate | 波 C 验收门 C18 | D-G33-2/D-G33-3/D-G33-4 / G-G33-1~6 |

### 2.2 out-of-scope（显式排除）

- **#56 外部采纳判据**——维持未宣称（11_ROADMAP §6 carve-out 字面 0-byte：「外部选择/采纳」维度未宣称达成；本波交付 = 可采纳工程面，非采纳事实）。
- RD-027 修复冒充（绕行登记非修复确证；毒区 fail-closed + O0 护栏为守护面，条目维持 open）。
- GAP-01~03 许可义务闭合冒充（三件维持 open；发布口径 = 附带义务未闭前不以对应形态发布，docs/renderer/release_checklist.md §3/§4 + v1.0 修订行在案）。
- 任何新优化/新特性工作、冻结面（G13~G32 注册表/锚/契约条款）改写。
- .rx→SPIR-V RT 阶段 codegen 缺口冒充闭合（RFC-0048 §6 PR-2/3/4 维持 open 登记；镜像语料臂不充 .rx codegen 绿）。

## 3. 交付物清单

| ID | 交付物 | 形态 | 完成判据 |
|---|---|---|---|
| D-G33-1 | C1~C17 十七项 | 各 ci 门脚本 + evidence schema/登记件（milestones/g31/ 族） | 各门 --gate PASS 或如实登记在案（C9/C16 含诚实红面终态合法） |
| D-G33-2 | C18 波 C 验收门 | 本契约 §8 close-out 实测 facts + g33_budget.json 七条 | G-G33-1~6 六面全落 |
| D-G33-3 | 四件套 | milestones/g33/{G33_PLAN.md,G33_CONTRACT.md,CI_GATES.md,g33_budget.json} | 在树 + budget_eval 全 PASS 零 estimated |
| D-G33-4 | 战役总登记 | milestones/g31_plus_campaign_record.md | 56 项逐项终态映射 + #56 字面 + G17-MD-F1 诚实红轨迹 |

## 4. 验收门（完整版，YAML 头为可提取摘要）

G-G33-1~G-G33-6 逐字见 front matter `acceptance_gates`。性能/帧率面证据等级 = measured_local（RTX 4070 Ti + Vulkan，本机真跑，gpu_device_lock 串行，RURIX_VK_VALIDATION=1）；采样协议 = canonical 160 帧 warmup 10（焦点格 5 样本取中位）/ 门各自口径。

## 5. Guardrails（字节级，机器核对）

见 YAML 头 `guardrails` 字段。核对方式：`ci/check_guardrails.py`（agent 完全自主模式 ADVISORY 不阻断）+ `ci/check_schemas.py` / `ci/budget_eval.py` 硬门。

## 6. Deferred 引用

| 编号 | 内容摘要 | 承接 |
|---|---|---|
| RD-027 | PT 毒径挂起 | C10 落档绕行：毒区全测绘 + O0 护栏 + fail-closed 毒区拒绝；**维持 open**（绕行非修复；backfill_condition 0-byte） |
| RD-036 | C ABI v2 超界需求 | C1 判档 maintain_open：backfill 两判据均不成立，超界四项不触；subset v1 机器核验归门 fact |
| RD-039 | 虚拟化几何长线 | C11 P4 四行清零（差距闭集）；C16 骨骼分项 triggered 开实施窗判档，余项维持；**维持 open** |
| RD-040 | 光照五分项 | C15 RT pipeline 兑现（RT-PIPELINE-SBT 分项锚消费）；C16 SMRT/世界缓存/NRD 维持 defer；**维持 open** |
| RD-026 | std::gpu 首期外编排面 | C16 not-triggered（A3 = Rust host 非 .rx 单源）；maintain-open |
| RD-008 | stable 快照机制 | C1 渲染器面第五段纳入同一快照比对 + bless 纪律（机制延伸非激活） |
| RD-045 | 间歇 digest 漂移三件 | 本波观察面只追加（各臂 digest 零漂移）；三件 0/3 维持不冒充 |
| RD-015 | DXIL B 路供应链 | C17 锚信号登记：llvm#57928 closed-as-completed 2026-08-13 → 重判程序启动信号，条目维持 open 不冒充 close |
| RD-033 | EA1 冷启动 A 段 VM | C17 探针：Win11 x64 VMware VM 候选在盘（owner 窗核验前非锚兑现）；维持 open |
| RD-032 | 平台 surface 余量 | G-MB1-6 AMD 真卡验收维持 open（C3 兼容矩阵 AMD/Intel 格 DEV_ENV_DEGRADE 如实） |

详情以 [../registry/deferred.json](../registry/deferred.json) 为唯一事实源，本表仅引用。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-26 | 初版契约固化（波 C 批次 = 商业化期首波；G31_PLUS §6 波次线 3 承接；C1~C17 交付 + C18 验收 §8 close-out） |

---

## 8. Close-out 区（只追加 — 波 C 验收记录）

### §8.1 波 C 验收门（C18）验收记录（2026-08-26，验证+治理 agent 真跑产出）——六面：五面全绿 + 焦点格诚实红面维持如实登记

**① 终验三面复跑（G-G33-1 全绿）**：
- **C1 SDK 宿主**：`py -3 ci/g31_renderer_sdk_smoke.py --gate g31.waveC.sdk` → GATE PASS——外部 C++ 宿主真跑 canonical 160+10 末帧 digest sha256:c1d28ad73783cc3c… == Stage A 锚 bistro-interior_t100_tsr_device 位级 MATCH；帧时 mean=2.1572ms p50=2.0561ms n=160；stable 快照 renderer_sdk_api 段 export_count=9 abi_version=1.0.0 --check exit=0；export_c 回归绿；evidence evidence/g31_renderer_sdk_20260826T205708Z.json。
- **C5 离线可建链**：`py -3 ci/g31_sdk_dist_smoke.py --gate g31.waveC.dist` → GATE PASS——干净目录仅 bundle+MSVC 编示例 0.5s（毒化代理 env 零网络）+ 真跑 canonical 160+10 digest == 同一 Stage A 锚位级 MATCH + 帧时 mean=2.0648ms + 签名/SBOM/红臂四路 + EA1 回归（ci/rurixup_dist_smoke.py exit 0）绿；evidence evidence/g31_sdk_dist_20260826T205842Z.json。
- **C2 文档门**：`py -3 ci/g31_renderer_docs_smoke.py --gate g31.waveC.docs` → GATE PASS——minimal_host 示例 cl 编译链接真跑标记逐字 `RURIX_MINIMAL_HOST_OK passes=2 frames=4 pixel=0x00000000`；evidence evidence/g31_renderer_docs_20260826T205521Z.json。

**② 发布件核验（G-G33-2 全绿 + GAP 诚实登记）**：`py -3 ci/g31_vendor_license_smoke.py --gate g31.waveC.license` → PASS（16 项盘点 cleared 15/conditional 1/pending_owner 0/blocked 0；SBOM 对账 + 许可文本在树 + G13 引用；evidence evidence/g31_vendor_license_20260826T205856Z.json）。**GAP-01~03 处置状态核验 = 三件维持 open**（milestones/g31/g31_vendor_license_matrix.json gaps[] 在案：发布 bundle 未随附许可文本与第三方声明 / release.yml 许可单标与 workspace 双许可字面不一致 / SBOM 组件级粒度未展开内嵌第三方库）；发布件口径 = 「附带义务未闭前不以对应形态发布」在案（docs/renderer/release_checklist.md §3 渲染器 SDK bundle 待建立行 / §4 全 vendor 矩阵待建立行 / v1.0 修订行「落地前不以对应形态发布」字面核验）。

**③ 全量回归（G-G33-3/G-G33-4）**：
- **守卫套件五条**（仓库根目录，exit 全 0）：check_structure PASS (11 dirs, 6 files)；check_schemas PASS（含本批 `g33_baseline_` 快检件跳过路由一处纯追加重放核验）；check_number_ledger PASS（ADVISORY 不阻断：off_tree_workflows[grx] 两注预存）；trace_matrix --check PASS (389/389 clauses anchored, 883 test files scanned)；budget_eval normal PASS + **--strict PASS (321 pass, 0 skip, strict mode)——零 estimated**。
- **波 A 五门 --gate 新鲜复跑 5/5 PASS**：present（real_render=58.907ms present=1.181ms digest sha256:94a2cfc6…）/ pipelining（evidence/g31_frame_pipelining_20260826T220043Z.json）/ gameloop（orbit 双跑 digest_seq 位级 + dolly/曝光区分）/ dynscene（evidence/g31_dynamic_scene_20260826T221531Z.json）/ framegen（x2 双跑位级 + G26 对拍接线态复跑 pass + presented/real 双口径）。
- **波 B 五门 --gate 新鲜复跑 4/5 PASS + 1 诚实红**：hzb（evidence/g31_hzb_wiring_20260826T222243Z.json）/ restir（20260826T223017Z）/ slab（g31_slab_wiring_gate_20260826T223040Z）/ skinning（20260826T225801Z）全 PASS；**texture 复跑 FAIL 诚实红**——8 facts 中 6 PASS（asset 映射 12/12、probe 对拍位级、sampler 界、bistro demo、off 回归锚 == Stage A、on/off 帧时 measured），2 FAIL = `g11_3_anchor_rerun_green` 与 `texture_kernels_spv_valid` 内的整 crate/工作树 0-byte 机核（`git diff --quiet HEAD -- spec rurix-asset …`），**根因 = 波 C 自身未 commit 加性交付物落同一代理面**（src/rurix-asset：bcdec.rs/ktx2.rs/geom_build_v2.rs/Cargo.toml M + kernels/ + 两 bin ?? = C11/C13/C14 交付面；spec/release.md M = C5 RXS-0214 同条修订）——B4 真冻结面逐件 0-byte 本批实测维持（母版 kernel g14_3_direct_gi.rx / material/ / graph/types.rs / G11.3 manifest 全 0-byte）；渲染实质零回归，门件 evidence/g31_texture_sampling_gate_20260826T223804Z.json 如实留存不删。
- **波 C 全门 --gate 新鲜复跑 17/18 PASS + 1 诚实红**：sdk/docs/capability/robustness/dist/license/profiling/support/rd027/p4stream/hlodl4/svt/ktx2/rtpipeline/meshbench/rejudgment/blockedprobes 全 PASS（evidence 20260826T205708Z/205521Z/210628Z/212406Z/205842Z/205856Z/212626Z/210000Z/211300Z/213056Z/213134Z/213146Z/214234Z/214507Z/214521Z/211131Z/211436Z）；**ngx_decomp 复跑 FAIL 诚实红**——7 facts 中 6 PASS（digest 零漂移 sha256:55ea0c2b…/四段分解 measured/TS vs X2 互核/墙钟一致/UE 差主源定位/重判结论登记），1 FAIL = `canonical_ratio_not_worsened`（fresh ratio=0.957606 < 在案 0.960479）——性能轨迹面诚实红维持，诊断件 .tmp/g31_gates/ngx_decomp/gate_fail_20260826T210732Z.json 如实留存。
- **三面锚**：Stage A digest 锚 **18/18 零漂移**（canonical 160 帧逐格重跑全 MATCH，evidence/g31_wave_a_anchor_check_20260826T230806Z.json——锚检门本体因焦点格轨迹面判 FAIL，digest 零漂移事实三面独立成立）；G16plus M-g **18/18 VERDICT=PASS**（evidence/g16_m_g_absolute_quality_closure_20260826T231716Z.json，UE 臂在案锚只读）；**焦点格 5 样本中位诚实红维持**：本日新鲜样本 3.512540 / 3.539031 / 3.586359 / 3.587437（ngx 门跑）/ 3.596624（锚检门跑）ms → **中位 3.586359ms → 中位 ratio 0.957894 < 在案 0.960479**（ue_median 3.43535ms 在案锚只读）；五跑 digest 全 == 在案锚（确定性面零漂移，恶化面 = 帧时机态抖动非渲染产出漂移）；轨迹 = 0.856→0.960→0.966（波 A）→0.956162（波 B）→**0.957894（波 C，较波 B 微升仍低于在案）**；帧时预算杠 ×2.0=7.1534ms 远未触及（g33.baseline.focus_cell.frame_ms_production_mean 条目 PASS）。
- **SER 顺带新鲜值**：rtpipeline 门复跑 SER workload t_off=1.3208ms t_on=2.5425ms ratio=0.519489（在案 0.518079 同窗微基准口径一致）。

**④ soak 汇总（G-G33-5）**：波 A soak **10010 帧**在案（evidence/g31_wave_a_soak_20260825T223024Z.json，零崩 + validation 静默 + leak 账本零 + digest_seq 抽查位级）+ C4 故障臂在案 **1010 帧** + 本日故障臂新鲜复跑 **1000 帧**（resize_ops=35 min_cycles=5 wall=84.6s 零崩零泄漏，evidence/g31_robustness_20260826T212406Z.json）+ **波 C SDK 面增量 soak 1010+10 帧零崩 exit=0**（外部 C++ 宿主 renderer_sdk_host.exe，RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1，token RXSDK_HOST_LOAD_OK/DIGEST/PARAMS_OK/PRESENT_OK/HOST_OK 全要素，非法面确定性拒 rc==3 见证；1010 帧末帧 digest 与 160 帧锚不同 = TSR jitter 序列帧位差异非漂移，确定性抽查纪律同波 A 在案口径）。

**⑤ 编号纪律**：CI 数字步骤零消费（落盘前实测 registry/number_ledger.json CI_step.next_free=525 维持；波 C 十八门 + C18 验收面均未占号，pr-smoke.yml 无 g31/g33 条目）；共享段实测：RFC on_tree_max=48/next_free=49（C15 RFC-0048 一件消费在案）/ RXS next_free=408 / RD next_free=46 / U next_free=60（C1 U-59 消费在案）/ SG/MR/D/RX_error 维持；evidence 前缀 `g33_baseline_` 经 check_schemas 一处纯追加（g31_baseline_/g32_baseline_ 同律跳过路由）PASS 重放核验。

**⑥ 波 C 验收总结论**：C1~C17 交付面全绿/如实在案 + C18 验收六面中五面全绿（终验三面/发布件核验/守卫套件/soak 汇总/三面锚之 digest·画质两面），**G17-MD-F1 焦点格 fresh 诚实红面 = 维持（轨迹 0.957894，较波 B 0.956162 微升、仍低于在案 0.960479）如实登记不冒充**——按「诚实登记不冒充；达标/维持/诚实红均合法终态」纪律，波 C 验收门以 **五面绿 + 一面诚实红（性能轨迹面）** 终态登记，无任何面冒充。**零降级终判三面结论**：画质 18/18 VERDICT=PASS + digest 18/18 零漂移 + 性能 17/18 诚实红不恶化（轨迹面维持 honest-red 终态，预算杠 ×2.0 远未触及）——**三面终判成立（零冒充）**。

**⑦ 签署**：`Assisted-by: TraeCode:Kimi-K3（G31+ 波 C 验收门 Task C18）`。
