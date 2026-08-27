<!-- Assisted-by: TraeCode:Kimi-K3（G31+ 波 A 验收门 Task A6） -->
# G31 CI_GATES — 里程碑冒烟门登记（实时呈现期 · 波 A）

> 事实源 = [G31_CONTRACT.md](G31_CONTRACT.md)。本表只登记门 key / 脚本 / 步骤号口径，不复述判据。

## 1. 波 A 实现五门（A1~A5）

symbolic gate key 与脚本名 = 波 A 交付即冻结字面；**未占 CI 数字步骤**（pr-smoke.yml 无 g31 条目——波 A 门为本地/device 真跑门，非 pr-smoke 秒级核验面；落盘前实测 registry/number_ledger.json CI_step.next_free=525 维持零消费）。

| 步骤 | gate key | 脚本 |
|---|---|---|
| 未占号 | g31.waveA.present | ci/g31_window_present_smoke.py |
| 未占号 | g31.waveA.pipelining | ci/g31_frame_pipelining_smoke.py |
| 未占号 | g31.waveA.gameloop | ci/g31_game_loop_smoke.py |
| 未占号 | g31.waveA.dynscene | ci/g31_dynamic_scene_smoke.py |
| 未占号 | g31.waveA.framegen | ci/g31_framegen_present_smoke.py |

## 2. 波 A 验收门两门（A6，本波 materialize）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 未占号 | g31.waveA.anchor_check | ci/g31_wave_a_anchor_check.py |
| 未占号 | g31.waveA.soak | ci/g31_wave_a_soak.py |

## 2.1 波 B 实现门（Task B3，本批 materialize）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 未占号 | g31.waveB.slab | ci/g31_slab_wiring_smoke.py |

## 2.2 波 B 实现门（Task B1，本批 materialize）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 未占号 | g31.waveB.hzb | ci/g31_hzb_wiring_smoke.py |

## 2.3 波 B 实现门（Task B4，本批 materialize）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 未占号 | g31.waveB.texture | ci/g31_texture_sampling_smoke.py |

## 2.4 波 B 实现门（Task B2，本批 materialize）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 未占号 | g31.waveB.restir | ci/g31_restir_wiring_smoke.py |

## 2.5 波 B 实现门（Task B5，本批 materialize）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 未占号 | g31.waveB.skinning | ci/g31_skinning_wiring_smoke.py |

## 2.6 波 B 评估窗（Task B6/B7，登记面无硬门）

B6 GI 默认档 measured 权衡窗与 B7 OIT/半透明评估窗 = 评估登记面（决策只追加），**不设硬门不占号**：无 ci 脚本、无 evidence schema、check_schemas.py 零消费。结论落盘 = milestones/g31/ 下两份只追加 JSON（measured 数字全部来自真实命令输出，既有锚/bench 默认面 0-byte）。

| 窗 | 结论件 | 结论 |
|---|---|---|
| B6 GI 默认档 | g31_gi_default_tier_decision.json | maintain_default_off（off 1.79~1.93ms vs on 7.03ms 生产口径 ×3.64~3.93；画质 +10.05% luma 但对 UE Lumen 在案诚实红未闭） |
| B7 OIT/半透明 | g31_oit_evaluation_window.json | not_triggered（压测闭集机核全 OPAQUE；oit/ 维持 M120 测量 harness 态；strand 档锚未命中维持） |

## 2.7 波 B 验收门（Task B8，登记面无硬门不占号）

B8 = 验收登记面（同 B6/B7 律：无 ci 脚本、无 evidence schema、check_schemas 零消费），六面判据与实测 facts 落盘 = milestones/g32/G32_CONTRACT.md §8 close-out（G32 画面完整期契约批）+ milestones/g31/ 下两份只追加 JSON：

| 面 | 结论件 | 结论 |
|---|---|---|
| 组合矩阵 + demo 定版 | g31_waveb_combo_matrix.json | 可组合臂 5/5 真跑绿（双跑 digest 位级 + 帧率 measured）+ 互斥 12/12 fail-closed 拒跑 exit=1；demo = --textures on + orbit 200+10 帧双跑位级（real_render 5.113/5.431ms + present 1.004/1.013ms 双口径） |
| RD-045 复核 | g31_waveb_rd045_observation_results.json | 波 B 各臂 digest 锚 6/6 零漂移 + Stage A 18/18 ×2 跑；三件 0/3 维持 maintain-open 不冒充；registry/deferred.json history 2026-08-26 行只追加 |

## 2.8 波 C 调查门（Task C9，提前至波 B 同窗 materialize）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 未占号 | g31.waveC.ngx_decomp | ci/g31_ngx_decomposition_smoke.py |

## 2.12 波 C 实现门（Task C1，本批 materialize；节号避让同窗并发 Task C2/C3/C4 的 §2.9~§2.11）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 未占号 | g31.waveC.sdk | ci/g31_renderer_sdk_smoke.py |

Task C1 = G31_PLUS_COMMERCIAL_RENDERER_TODO §5 #48「渲染器 SDK 稳定 API 面」兑现面。架构 = 两层 DLL（export_c codegen 复用，RD-009 closed 机制第三消费方）：`apps/g31-renderer-sdk/src/sdk.rx`（9 个 #[export(c)] 薄转发，subset v1 全合规）经 `rurixc --emit=dll` 产 `rurix_renderer.dll` + import lib + 生成头（RXS-0253 单一事实源）→ `#[link]` 绑定实现层 `src/rurix-renderer-sdk` cdylib（`rxsdk_*` u64 句柄会话面，薄封装 G14.3 统一四 pass TSR 生产车道 `g14_3_lane_body.rs` include! 第三消费方共享，unsafe-audit **U-59** 登记——共享编号段消费：U next_free 59→60，ledger v1.189；RFC/RXS/RD/CI_step 段零消费）。语义化版本 = `apps/g31-renderer-sdk/API_VERSIONING.md`（v1 = 1.0.0；MAJOR 破坏走 RFC + 新旧并存、MINOR 同 MAJOR 只增、PATCH 语义不变修复）。

**stable 快照守卫扩展（RD-008 处置）**：`ci/stable_snapshot.py` 新增 `renderer_sdk_api` 段（sdk.rx 9 导出规范化签名 + abi_version=1.0.0 程序读）——RD-008 closed 机制的渲染器面延伸，**取舍登记**：语言面四段 0-byte（spec_clauses=389/error_codes=113/editions/subcommands 不变），渲染器面作为独立第五段纳入同一快照比对 + bless 纪律；非「激活 RD-008」（机制 G2.5 已激活），而是首个 stable C ABI 嵌入面按同机制守卫。bless_log.md 2026-08-25 行 + registry/deferred.json RD-008 history 同日行。

**RD-036 判档**：backfill 两判据逐项核验均不成立（① 无 upcall——全下调用 + 宿主缓冲出参轮询；② 非外部固定 ABI——greenfield 新面两侧协同指针化），超界四项（repr(C) struct 按值/回调指针/数组按值/跨堆所有权）逐项不触，签名面 subset v1 机器核验归门 fact `rd036_subset_v1_compliant`；disposition=maintain_open，deferred.json RD-036 history 2026-08-25 行。

## 2.11 波 C 健壮性门（Task C4，本批 materialize）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 未占号 | g31.waveC.robustness | ci/g31_robustness_smoke.py |

## 2.9 波 C 文档门（Task C2，本批 materialize）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 未占号 | g31.waveC.docs | ci/g31_renderer_docs_smoke.py |

## 2.10 波 C 设备兼容矩阵与能力降级链门（Task C3，本批 materialize）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 未占号 | g31.waveC.capability | ci/g31_capability_fallback_smoke.py |

## 2.13 波 C 支持渠道与版本政策门（Task C8，本批 materialize；节号避让同窗并发 Task 已占 §2.9~§2.12）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 未占号 | g31.waveC.support | ci/g31_support_policy_smoke.py |

Task C8 = G31_PLUS_COMMERCIAL_RENDERER_TODO §5 #55「支持渠道与版本政策」兑现面。交付 = `docs/renderer/support_policy.md`（缺陷报告流程五要素/四面分类/诚实响应口径 + 版本政策引用 `apps/g31-renderer-sdk/API_VERSIONING.md` + 里程碑期联动节奏 + LTS 修复线 + 安全响应镜像语言面 SECURITY.md 与渲染器三特有面 + stable ABI 守卫与 RFC 纪律 + 待建立项五件诚实登记）+ `docs/renderer/release_checklist.md`（发布机器门操作单八面：stable ABI 守卫/波 A·B·C 十七门/签名·SBOM·分发链/许可·再分发/兼容矩阵/soak·健壮性/文档与政策/环境三态，全部引用真实 ci 脚本）+ SECURITY.md 与 SECURITY.en.md 各一段 append-only 渲染器面增补指针段（既有结构 0-byte；并列新文件未采用——单一安全入口纪律）。本门 = host 恒跑面七 facts（文档节锚与在案字面防腐化/引用 ci 脚本 27 件与仓内面 19 件存在性机器核验/版本政策五面同一字面 lib.rs 程序读 1.0.0 ≡ snapshot abi_version·export_count=9 ≡ API_VERSIONING.md ≡ 政策文档/安全镜像四要素+特有面+双件增补段锚/待建立在飞登记/00~14 冻结文档 15 件零触碰），无 GPU/工具链腿不设 DEV_ENV_DEGRADE 降级；编号段零消费（CI_step/RFC/RXS/RD/U 全段维持）。

## 2.14 波 C vendor 许可合规终审门（Task C6，本批 materialize；节号避让同窗并发 Task 已占 §2.9~§2.13）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 未占号 | g31.waveC.license | ci/g31_vendor_license_smoke.py |

Task C6 = G31_PLUS_COMMERCIAL_RENDERER_TODO §5 #53「vendor 许可合规终审」兑现面（商用分发口径 = 再分发许可合规）。交付 = `milestones/g31/g31_vendor_license_matrix.json`（机器可读 16 项全 vendor 面矩阵：外部 SDK 4 + 树内 vendored 6 + NVIDIA CUDA EULA 面 2 + Rust crate 面 4；cleared 15 / conditional 1〔rust_rowan，GAP-01 义务未闭合〕/ pending_owner 0 / blocked 0）+ `docs/renderer/vendor_license_matrix.md`（人读渲染面）+ GAP-01~03 缺口登记（发布 bundle 未随附许可文本与第三方声明 / release.yml 许可单标与 workspace 双许可字面不一致 / SBOM 组件级粒度未展开内嵌第三方库——均归 C5 分发链面）。G13 超分面（DLSS/Streamline/NGX + FSR + NRD）**引用不复制**（milestones/g13/design/vendor_upscale_license_clearance.md owner 2026-08-18 接受在案）；本批新核项全 OSI/既有机制面 → 零新 owner 动作如实登记（agent 不冒充 owner 接受）。本门 = host 纯 host 门七判据（矩阵结构/覆盖闭集 16 项/许可文本在树/G13 引用/SBOM 对账〔release.yml 组件许可段 + rurixup sbom.rs licenseConcluded + basis SBOM.md + g13 SDK 登记 + 逐项 sbom_faces〕/义务与 GAP 登记/summary 计数重算诚实），无 GPU 腿不设降级；编号段零消费（CI_step/RFC/RXS/RD/U 全段维持，CI_step.next_free=525 落盘前实测）。

## 2.15 波 C 渲染器 SDK 分发打包门（Task C5，本批 materialize；节号避让同窗并发 Task 已占 §2.9~§2.14）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 未占号 | g31.waveC.dist | ci/g31_sdk_dist_smoke.py |

Task C5 = G31_PLUS_COMMERCIAL_RENDERER_TODO §5 #52「渲染器 SDK 分发打包」兑现面（交付判据 = SDK bundle 一键安装 + 示例工程离线可建）。链路 = EA1 分发链复用（RXS-0214~0218 机制面，零新 RXS 消费）：16 组件预编译 bundle（SDK 两层 DLL `rurix_renderer.dll`/`rurix_renderer_sdk.dll` + 编译器生成头 + import lib ×2 + canonical SPV 四件套 + bistro 生产契约 + 示例工程源 `renderer_sdk_host.cpp` + 文档五件）经 `rurixup release --channel stable`（VALID_CHANNELS 0-byte，最小侵入面）编排签名/SBOM → `--from-dir` 四级校验真实物化 → default 切换 + `list --verify` + 幂等 → hermetic 环回 HTTP 网络 install（零真实外呼）→ 干净目录仅 bundle+MSVC 公开工具链离线构建示例（毒化代理 env 见证零网络依赖）→ 真跑 canonical 160+10 末帧 digest 对拍 Stage A 锚。**component_rel_path SDK 面纯追加**（`src/rurixup/src/install.rs`：`*.h→include/`、`*.spv→spv/`、`*.json→manifests/`、`*.md→docs/`、`*.cpp→examples/`；既有 `*.exe`/`*.lib`/nvidia 路径 0-byte；spec/release.md RXS-0214 同条修订——条款文本修订零新条款 ID，stable 快照只记条款 ID 不受影响）。红臂四路（签名错 release 阻断 failed_gates=[signing] / 哈希错 kind=integrity 零半装 / 截断 kind=network / 清单篡改 kind=integrity）+ 端点不可达 + 复原绿闭合；EA1 回归（ci/rurixup_dist_smoke.py 复跑 exit 0）为门 fact `ea1_regression_green`。签名/SBOM 扩展 = 两 DLL selftest 验签项（生产后端 azure 归 release.yml 既有面）+ SBOM 双视图覆盖 16 组件 + vendor 运行件技术对账三件（NGX/Streamline·FSR dynamic-load-not-bundled / basis_universal static-in-dll；与 C6 许可矩阵协同，本面仅 SBOM 技术对账不做许可裁决；C6 GAP-01~03 分发链面缺口维持 open 登记不冒充闭合）。编号段零消费（CI_step/RFC/RXS/RD/U 全段维持）。

## 2.16 波 C 性能剖析与调试工具面门（Task C7，本批 materialize；节号避让同窗并发 Task 已占 §2.9~§2.15）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 未占号 | g31.waveC.profiling | ci/g31_profiling_smoke.py |

Task C7 = G31_PLUS_COMMERCIAL_RENDERER_TODO §5 #54「性能剖析与调试工具面」兑现面（验收锚 = 外部用户可自助定位帧内热点）。三面：① `--profile-json <path>` 统一 profiler 输出面（g31_window_present/g14_3_pipeline_perf 双 bin 同 schema `rurix.g31.profile_output.v1`，规范面 `milestones/g31/g31_profile_output_schema.json` 非 evidence 路由 check_schemas 零消费——逐 pass GPU/CPU/帧段 mean/p50/p99 + 分解恒等式字段 + debug label 态 + profiler 开销如实登记；默认关,on/off 双臂 digest 位级一致为渲染语义零变更机器证明;g14_3 首接 = tsr_device 静态臂 inflight=1,vendor 双臂/FIF/dyn/skin CLI fail-closed 拒跑归后续）;② Nsight 标注（`src/rurix-rt/src/render_exec.rs` 建 instance 枚举 VK_EXT_debug_utils 在位即启用〔validation 关也在位〕,record_frame_body 逐 pass `vkCmdBegin/EndDebugUtilsLabelEXT` 包裹 timestamp 区间 + pass 本体,label 名 == telemetry pass 名 == profile gpu_passes 名三面同名词;absent 双 None 零开销跳过 fail-silent）;③ RenderDoc 捕获兼容（renderdoccmd 在机 → 真捕获腿自动切换 real_capture 口径;不在机 → validation 静默 + 捕获不兼容 API 模式 blocklist 0 命中静态核验 + DEV_ENV_DEGRADE 如实登记不冒充真捕获——本机 RenderDoc/Nsight Graphics 双 absent 在案）。文档 = `docs/renderer/profiling_debugging.md`（外部用户自助定位面）+ performance_tuning.md 姊妹篇指针行。七 facts 闭集（profile schema 合规/分解 measured/恒等式 0.10+2.00ms/标注段存在/on-off 零漂移/捕获兼容/工具探测）。编号段零消费（CI_step/RFC/RXS/RD/U/SG/MR/D/RX_error 全段维持）。

## 2.17 波 C RD-027 PT 毒径回归守护门（Task C10，本批 materialize；节号避让同窗并发 Task 已占 §2.9~§2.16）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 未占号 | g31.waveC.rd027 | ci/g31_rd027_poison_guard.py |

Task C10 = G31_PLUS_COMMERCIAL_RENDERER_TODO §5 尾项 #16「RD-027 PT 毒径挂起修复」兑现面（生产档路径追踪商用前置 = 毒径定位）。**定位（复确认）**：E8 全网格实测毒区图（spp∈{8,32,64,128,256}×bounces∈{1,2,3,4} 20 格全实测非外推，spike/rd027-pt-poison/run_e8_zone.py，proc_guard 60s 判定线 + 挂起后金丝雀 14/14 过）——b1 全 spp 绿 / b2 spp≤64 绿、≥128 毒 / b3·b4 全 spp 毒（绿 7 毒 13）；distinct PTX digests=1 单 artifact 复确认（PTX sha256 与 G3.1 逐字节同一）；判别腿 (8spp/b3) @O1 挂 60s / @O0 完成 0.68s + SASS `rx_pt_render` 节 `@!P0 CALL.REL.NOINC` 无记账 latch 出口 O0=0/O1=4/O3=4（与 G3.1 归因逐点一致）——**根因层维持定罪 NVIDIA 优化后段（层③④，M1′ 机理），本仓不可修**；算法层（源循环全封顶仍挂/俄轮终止无缺陷）与 rurixc/LLVM 层（O0 同 PTX 正确终止 + refcpu 逐位）排除维持。**处置 = 落档绕行**（修复=上游 NVIDIA 本体，DRAFT 备包维持 do-NOT-file owner 复核门）：MR-0011 `RURIXC_PTXAS_OPT=0` 护栏常驻（(8,3)/(8,4)/(256,2) 护栏腿终止 + digest 基线）+ 毒区参数面 **fail-closed 拒绝**（静态判 params.rx 生产档切片，poison/unmapped 一律诚实红；未测绘组合按毒处理）。本门 = 静态 fail-closed + 边界绿腿（(32,2)/(8,2) 默认档终止 + digest 命中基线）+ 护栏毒腿（(8,3)/(256,2)@O0 终止 + digest 命中）+ 毒确认腿（(8,3) 默认档有界超时必须 hang_timeout，证毒区仍毒；完成 = 漂移诚实红促重测/backfill 评估）+ 挂起后金丝雀 + 三态 DEV_ENV_DEGRADE。毒区图 = `milestones/g31/g31_rd027_poison_zone_map.json`（机器可读，digest 基线入图；driver/ptxas/GPU 变更漂移 = 门红按生成器全网格重测，禁手改）。RD-027 **维持 open**（绕行登记非修复确证；backfill_condition 原文 0-byte 不预支），registry/deferred.json history 2026-08-26 行只追加。编号段零消费（CI_step/RFC/RXS/RD/U/SG/MR 全段维持；判档 = 本批零 codegen/工具行为变更——纯取证 + CI 守护 + registry 留痕，MR-0011 护栏为既有 Approved 面复用，Direct 档不涉新 RFC 面）。

## 2.18 波 C cluster 流送 P4 四行门（Task C11，本批 materialize；节号避让同窗并发 Task 已占 §2.9~§2.17）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 未占号 | g31.waveC.p4stream | ci/g31_p4_streaming_smoke.py |

Task C11 = G31_PLUS_COMMERCIAL_RENDERER_TODO §3 #20~#23 四行兑现面（RD-039 cluster 流送 P4 分项；milestones/g20/g20_cluster_streaming_p4_gap.json 四行差距闭集 + milestones/g27/g27_cluster_p4_rejudgment.json「P4-2 依赖解除（HZB device 化已兑现）」登记消费面）。四行逐行独立判档：**P4-1** = RXPD major=2 加性磁盘面（`src/rurix-geom-pages/src/disk_v2.rs` 新文件：payload = RXPZ-LZ1(RXPL v2 映像)、v2 域分离 schema_digest、加性映射行 {(RXPD,2)→(RXPL,2)}、四类损坏 fail-closed 沿 v1 口径；v1 七文件 0-byte）+ bistro 派生真实页集（严格 glTF 导入 → build_asset_dag 簇 DAG → pack_cluster_dag_v2；**v2 段感知装箱修复同 PR**——v1 估算不计 v2 段字节 + v1_section_bytes:u16 双界，近边界页实际超 128KB 的 bistro 实证 bug，geom_build.rs 0-byte、修复全落 geom_build_v2.rs 加性面内）+ host 驻留池（PagePool LRU/容量预算/root 钉住 0-byte 复用，逐出真实发生）。**P4-2** = `src/rurix-asset/kernels/g31_cluster_stream.rx`（rurixc 产 SPV + harness bin 侧 NoContraction 注入，SPV/kernel 源 0-byte；geometry/cull.rs 金标准字面 0-byte 消费）剔除 pass 产 cluster 缺页请求 → device 请求缓冲读回 → host 驻留调度消费（`streaming/cluster.rs` 加性模块：PriorityIoPool 异步读 + ClusterPageResource 异步缓存模式〔M37 ready_raw 同律 fail-closed〕+ StreamingEngine 三预算 tick 0-byte 复用）→ 页表/页池镜像上传 → 次帧 device 消费校验（页槽首字 checksum 演化）闭环真跑。**P4-3** = 驻留约束一致性 cut（streaming/cluster.rs `lod_cut_with_residency` host 金标准：缺页 → 最近驻留祖先-or-自身 → 祖先-后代合并——渲染集恒为 DAG 合法 cut，禁空洞/禁重复覆盖；root 钉住保证终止）+ device 读回归一（`normalize_render_decisions` 单源双臂共用）逐帧对拍 + `verify_cut_cover` 逐帧覆盖不变量 + 全驻留参考零回退双跑位级。**P4-4** = PriorityIoPool 固定工作线程 + 优先级堆（屏幕投影直径量化重要度——近处/大屏占比优先；priority 降序 + seq FIFO tie-break 调度序与墙钟无关）真实磁盘读 + 优先级倒置探针（开工闸前 [低×3,高×1] 单 worker 出队序 measured 高优先级先驻留）。**整合** = 强制小驻留池（root/流动分项定纲：root 钉住占容量 + hold 段选中流动页上界 reference 实测 +1 在途余量，容量 < 全集）+ 穿越式轨迹（相机环道平移切线朝前 = 工作集稀疏化结构来源）+ 常驻工作集逐帧零成本触新（LRU 保活——缺则池抖动不收敛，bistro 实证）+ hold 段有界真等 + tick 排空；零回退帧 digest 与全驻留参考逐帧位级一致（回退帧允许 LOD 差——容差结构依据 = 一致性 cut 语义），末 2 帧收敛位级，缺页率/回退率/IO 量 measured。harness = `src/rurix-asset/src/bin/g31_cluster_stream.rs`（rurix-asset 既有 vulkan 面 + [[bin]] 加性；真跑件 `rurix.g31.cluster_stream_evidence.v1` 留 .tmp 工作区无注册 schema，数字经门裁决件蒸馏）。本门 = device 真跑门六 facts（①页集/驻留池 ②请求-驻留闭环 ③回退对拍 ④IO 优先级 ⑤整合真跑 ⑥冻结旧面 0-byte 机核——geom-pages v1 七文件/streaming v1 四文件/geometry 三文件/geom_build.rs 工作树 0-byte）+ 三态 DEV_ENV_DEGRADE（无 Vulkan/SPV 编译失败/bistro 缺失；RURIX_REQUIRE_REAL=1 翻硬 FAIL）。编号段零消费（CI_step/RFC/RXS/RD/U/SG/MR/D/RX_error 全段维持；CI_step.next_free=525 落盘前实测维持）。RD-039 **维持 open**（四行进展 = history 只追加登记，P4 长线其余分项（超显存 P4 运行时等）不在本门字面）。

## 2.19 波 C 阻塞项新鲜探针门（Task C17，本批 materialize；节号避让同窗并发 Task 已占 §2.9~§2.18）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 未占号 | g31.waveC.blockedprobes | ci/g31_blocked_probes_smoke.py |

Task C17 = G31+ 待办总表 TODO 阻塞项全量新鲜探针 + 如实登记兑现面。探针登记件 = `milestones/g31/g31_blocked_probes_2026.json`（落 milestones/ 非 evidence/ 路由面，check_schemas 零消费——12 探针逐项：方法/真实命令输出/结果/verdict ∈ {open-maintained, blocked-dev-env} 闭集/状态维持字面/锚不变确认；零冒充：无一项被标 closed/resolved）。本门 = 登记件机器核验（12 探针齐备 + verdict 闭集 + 零冒充机核 + anchor_unchanged 逐项 true + summary 计数重算）+ 活体复核 host 恒跑十腿（三工具 PATH/BistroExterior 三检索根、vulkaninfo HDR×3+WG+DGC×3 token 与设备枚举、Hyper-V/VMware 面、bistro gltf 材质计数、物理观察轨 22 pattern〔g30 常量表逐字沿用禁缩面〕、SAFE-GPU 期面+docs/ 三面、legacy 清册零 close、本地工具链版本、RD-034 blocked 探针真跑、RD-045 三件面）——阻塞→解锁翻转 = 锚命中重判信号（合法门绿，F10 门态映射分支捕获非透传；门 FAIL 只留程序未诚实执行）+ device 腿三态（RD-045 orbit 64+10 digest 抽查对波 B 锚；GPU/产物缺 DEV_ENV_DEGRADE 退 0 不冒充 PASS，RURIX_REQUIRE_REAL=1 翻硬 FAIL，digest 漂移 = 诚实红）。新鲜发现如实登记：RD-015 reeval_anchor「LLVM 上游任一 issue 关闭」字面命中（llvm#57928 closed-as-completed 2026-08-13 → 重判程序启动信号，条目维持 open 不冒充 close）+ Win11 x64 VMware VM 候选在盘（RD-033 owner 窗核验前非锚兑现）。deferred.json 13 条 history 只追加汇总行指向探针件（tail_six/legacy 面引用不复制）。编号段零消费（CI_step/RFC/RXS/RD/U/SG/MR/D/RX_error 全段维持）。

## 2.20 波 C 重判窗批量执行门（Task C16，本批 materialize；节号避让同窗并发 Task 已占 §2.9~§2.19）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 未占号 | g31.waveC.rejudgment | ci/g31_rejudgment_smoke.py |
| 未占号 | g31.waveC.meshbench | ci/g31_mesh_vs_raster_bench.py（M61 ③ measured 载体门——六窗重判的新鲜真跑输入面） |

Task C16 = G31_PLUS_COMMERCIAL_RENDERER_TODO §3 #24/#26/#27/#28/#29 + §4 #43 六窗重判批量执行（评审 agent 面 = 锚字面核验 + 只追加登记，代码改动最小）。判档登记表 = `milestones/g31/g31_rejudgment_windows.json`（落 milestones/ 非 evidence/ 路由面，六窗逐行：anchor_literal/anchor_source/method/items/verdict ∈ {triggered, not-triggered, partial} 闭集/verdict_detail/evidence/followup）。六窗结论：**M61 mesh shader** = RFC-0034 三项闭集 3/3 齐备（①HZB device 化 G27 在案+B1 门维持绿 ②C11 P4 四行清零 ③本任务新鲜真跑——新 measured 对照底座 `src/rurix-rt/src/vk_g31_mesh_bench.rs` 单会话三臂〔vs_fetch 取数/vs_procedural/mesh_procedural〕同一确定性三角形集 digest 位级全等 + GPU timestamp 逐帧 measured：N=262144 档 0.2344 vs 0.2342ms、N=1048576 档 0.9065 vs 0.9057ms，median RTX 4070 Ti measured_local）⇒ 按重判表只追加程序执行改判评估，结论 = **maintain-no-go**（性能差 measured = 零 + 多厂商收敛单卡不可证 + 真实消费方零；VS 光栅唯一 fallback 维持）；**RD-039 backfill 逐项** = partial 1/5（骨骼 = triggered〔动态资产面字面命中，B5 蒙皮骨骼进生产 measured 件在案〕开实施窗判档登记——实施归后续期；Foliage/曲面细分/Assemblies/Mega Geometry 维持）；**SMRT** = partial 1/2（多灯动态资产面命中、shadow page 采样车道未出现 ⇒ maintain-defer）；**世界辐射缓存演进** = partial 1/2（大世界流送面命中、GI 联动窗未成立〔B6 maintain_off 在案〕⇒ maintain-defer）；**NRD** = not-triggered（自研降噪在案绿、画质差距 measured 检出零命中）；**RD-026** = not-triggered（A3 = Rust host 驱动非 .rx 单源，子集外七面硬需求零出现 ⇒ maintain-open）。登记面 = deferred.json RD-039/RD-040/RD-026 三行 history 只追加（status/backfill_condition 字面 0-byte）+ RFC-0034 重判记录 G31+ C16 行只追加（G18.5/G20.3/G27.2 三行 0-byte）+ 锚文件四件 tracked git 干净机核（TODO 在飞未跟踪件零触碰）。本门 = host 纯文件机核面八 facts（登记表结构/verdict 闭集/证据指针在盘/M61 ③ measured 门件 schema+数字健全/deferred 三行+0-byte 抽查/RFC-0034 行/锚文件 0-byte/窗内一致性），无 GPU 腿不设降级；measured 载体门 g31.waveC.meshbench = device 真跑门六 facts + 三态 DEV_ENV_DEGRADE（glslang/Vulkan/mesh feature 缺；RURIX_REQUIRE_REAL=1 翻硬 FAIL）。编号段零消费（CI_step/RFC/RXS/RD/U/SG/MR/D/RX_error 全段维持；unsafe 面 = vk.rs 块级豁免内 SAFETY 注释齐 0 新 U 号〔mesh 管线在既有 graphics FFI 边界内，run_mesh_offscreen 先例〕）。

## 2.19 波 C RT pipeline + SBT 宿主车道门（Task C15，本批 materialize；节号避让同窗并发 Task 已占 §2.9~§2.18）

| 步骤 | gate key | 脚本 |
|---|---|---|
| 未占号 | g31.waveC.rtpipeline | ci/g31_rt_pipeline_smoke.py |

Task C15 = G31_PLUS_COMMERCIAL_RENDERER_TODO §3.2 #31/#32 兑现面（M52 承接锚 = g30_campaign_handover_registry.json G28 行；RD-040 RT-PIPELINE-SBT 分项 reeval_anchor 消费面）。语义面 = **Full RFC-0048**（rfcs/0048_rt_pipeline_sbt_host_lane.md，Agent Approved 2026-08-25，D-409 第 1 轮 8 findings 全 disposition；number_ledger v1.190：RFC on_tree_max 47→48、next_free 48→49；共享 RXS/RD/U/CI_step/SG/MR/D/RX_error 段零消费）。兑现形态：①语言面锚 kernel `src/rurix-render/kernels/g31_rt_slab_hit.rx`（raygen ×1 + miss ×1 + triangles 双 hit group slab 双材质；--emit=check 0 诊断 + --emit=rt-manifest 结构绿——required_capabilities 恰 [rt.pipeline, rt.sbt_user_data]，RXS-0311 隐式推导 manifest 构建器漏推导修复同批：src/rurixc/src/rt_pipeline.rs 加性 + 两单测）+ 对拍臂 kernel `kernels/g31_rt_slab_rayquery.rx`（真 .rx 经 rurixc --target vulkan 产 SPV + spirv-val）；②device 真跑 = `src/rurix-rt/src/bin/g31_rt_slab_lane.rs` 三臂——RT 臂（M50 底座 `run_rt_pipeline_offscreen` 0-byte 复用；hand-emitted 镜像语料 emit_g31_rt_slab_miss/closesthit〔与 kernel 公式面逐字同源静态机核，**非 .rx 编译产物，不充 .rx codegen 绿**〕，2 hit groups × 20B slab records 真跑，双跑位级 + record readback + validation 静默 + golden 三采样点 vs host f64 参照）+ RayQuery 对拍臂（同场景同材质同相机同公式，对拍结构容差 = RFC-0048 §4.7：bitexact ∨ (mismatch_ratio ≤ 0.001 ∧ max_lsb ≤ 1)；实测 mismatch 0/4096 位级一致 = 更强终态）+ SER workload 臂（`src/rurix-rt/src/vk_g31_ser_body.rs` NV 变体双臂 reorder off/on：画面双臂位级 + 双跑位级 + 时延 measured 对照〔1.3239ms vs 2.5554ms，ratio=0.518079，measured_local 微基准 caveats 在案〕+ evidence/g31_ser_gain_estimate_<ts>.json 落盘）。**诚实边界**：.rx→SPIR-V RT 阶段 codegen 缺位实测（--target vulkan 退出码 2「no compute kernel fn found」），经 RFC-0048 §6 PR-2（mir_build intrinsic）/PR-3（vulkan_codegen RT 腿）/PR-4（车道转正替代镜像语料）维持 open 如实登记不冒充。M52 维持 defer 字面 0-byte 不回写（SER 语言面 go 须独立 Full RFC 评估）；RD-040 维持 open，history 2026-08-25 行只追加。本门 = 八 facts 闭集 + 三态 DEV_ENV_DEGRADE（无 Vulkan/工具链降级；RURIX_REQUIRE_REAL=1 翻硬 FAIL）+ selftest 判读器红绿 30+ 臂。编号段：CI_step 零消费（next_free=525 落盘前实测维持）。

## 3. evidence schema 登记（milestones/g31/）

| schema | 产证脚本 |
|---|---|
| g31_window_present_evidence_schema.json | ci/g31_window_present_smoke.py（A1 既有） |
| g31_frame_pipelining_evidence_schema.json | ci/g31_frame_pipelining_smoke.py（A2 既有） |
| g31_game_loop_evidence_schema.json | ci/g31_game_loop_smoke.py（A3 既有） |
| g31_dynamic_scene_evidence_schema.json | ci/g31_dynamic_scene_smoke.py（A4 既有） |
| g31_framegen_present_evidence_schema.json | ci/g31_framegen_present_smoke.py（A5 既有） |
| g31_wave_a_anchor_check_evidence_schema.json | ci/g31_wave_a_anchor_check.py（A6 本波） |
| g31_wave_a_soak_evidence_schema.json | ci/g31_wave_a_soak.py（A6 本波） |
| g31_slab_wiring_evidence_schema.json | ci/g31_slab_wiring_smoke.py（波 B Task B3 本批；harness 真跑件） |
| g31_slab_wiring_gate_evidence_schema.json | ci/g31_slab_wiring_smoke.py（波 B Task B3 本批；门裁决件） |
| g31_hzb_wiring_evidence_schema.json | ci/g31_hzb_wiring_smoke.py（波 B Task B1 本批；门裁决件——PASS-only 闭集 schema，harness 真跑件留 .tmp 工作区不进 evidence/） |
| g31_texture_sampling_evidence_schema.json | ci/g31_texture_sampling_smoke.py（波 B Task B4 本批；harness 真跑件 --textures on 腿） |
| g31_texture_sampling_gate_evidence_schema.json | ci/g31_texture_sampling_smoke.py（波 B Task B4 本批；门裁决件） |
| g31_restir_wiring_evidence_schema.json | ci/g31_restir_wiring_smoke.py（波 B Task B2 本批；门裁决件——PASS 态 facts 闭集 schema） |
| g31_skinning_wiring_evidence_schema.json | ci/g31_skinning_wiring_smoke.py（波 B Task B5 本批；门裁决件） |
| g31_ngx_decomposition_evidence_schema.json | ci/g31_ngx_decomposition_smoke.py（波 C Task C9 本批；分解证据件——G30 承接锚 G17-MD-F1 行 NGX 分解 profiling 兑现载体） |
| g31_robustness_evidence_schema.json | ci/g31_robustness_smoke.py（波 C Task C4 本批；运行时健壮性 + 故障注入门裁决件——device-lost 三点/TDR/budget 探针臂 + 基线 + 窗口风暴 + soak 故障臂） |
| g31_renderer_docs_evidence_schema.json | ci/g31_renderer_docs_smoke.py（波 C Task C2 本批；文档与示例门裁决件——G31_PLUS §5 #49 兑现载体；walkthrough 记录件 g31_renderer_docs_walkthrough.json 落 milestones/ 非 evidence/ 路由面，check_schemas 零消费） |
| g31_capability_fallback_evidence_schema.json | ci/g31_capability_fallback_smoke.py（波 C Task C3 本批；兼容矩阵/降级链门裁决件——G31_PLUS §5 #50 兑现载体；能力报告真跑件留 .tmp 工作区不进 evidence/ 路由面） |
| g31_renderer_sdk_evidence_schema.json | ci/g31_renderer_sdk_smoke.py（波 C Task C1 本批；渲染器 SDK 稳定 API 面门裁决件——G31_PLUS §5 #48 兑现载体；宿主真跑工作件留 .tmp 工作区不进 evidence/ 路由面） |
| g31_support_policy_evidence_schema.json | ci/g31_support_policy_smoke.py（波 C Task C8 本批；支持渠道与版本政策门裁决件——G31_PLUS §5 #55 兑现载体；host 恒跑面无路由外工作件） |
| g31_vendor_license_evidence_schema.json | ci/g31_vendor_license_smoke.py（波 C Task C6 本批；vendor 许可合规终审门裁决件——G31_PLUS §5 #53 兑现载体；PASS-only 闭集 schema，FAIL 诊断件留 .tmp 工作区不进 evidence/ 路由面） |
| g31_sdk_dist_evidence_schema.json | ci/g31_sdk_dist_smoke.py（波 C Task C5 本批；渲染器 SDK 分发打包门裁决件——G31_PLUS §5 #52 兑现载体；宿主真跑/构建工作件留 .tmp 工作区不进 evidence/ 路由面） |
| g31_profiling_evidence_schema.json | ci/g31_profiling_smoke.py（波 C Task C7 本批；性能剖析与调试工具面门裁决件——G31_PLUS §5 #54 兑现载体；profile 真跑工作件留 .tmp 工作区不进 evidence/ 路由面，PASS-only 闭集 schema） |
| g31_rd027_poison_guard_evidence_schema.json | ci/g31_rd027_poison_guard.py（波 C Task C10 本批；RD-027 PT 毒径回归守护门裁决件——静态 fail-closed/边界绿腿/护栏毒腿/毒确认腿；毒区图 g31_rd027_poison_zone_map.json 落 milestones/ 非 evidence/ 路由面，check_schemas 零消费） |
| g31_p4_streaming_evidence_schema.json | ci/g31_p4_streaming_smoke.py（波 C Task C11 本批；cluster 流送 P4 四行门裁决件——RD-039 分项/TODO §3 #20~#23 兑现载体；PASS-only 闭集 schema，harness 真跑件留 .tmp 工作区不进 evidence/ 路由面） |
| g31_rt_pipeline_evidence_schema.json | ci/g31_rt_pipeline_smoke.py（波 C Task C15 本批；RT pipeline + SBT 宿主车道门裁决件——TODO §3.2 #31/#32 + M52/RD-040 承接锚 + RFC-0048 兑现载体；PASS-only 闭集 schema，harness 真跑件留 .tmp 工作区不进 evidence/ 路由面） |
| g31_ser_gain_estimate_evidence_schema.json | ci/g31_rt_pipeline_smoke.py（波 C Task C15 本批；SER 收益 measured 预估窗件——RD-040 RT-PIPELINE-SBT 分项锚 `evidence/*ser_gain_estimate*.json` 消费面；harness bin --out 直出经门 schema 自校验后归档 evidence/） |
| g31_blocked_probes_evidence_schema.json | ci/g31_blocked_probes_smoke.py（波 C Task C17 本批；阻塞项新鲜探针门裁决件——PASS-only 闭集 schema，FAIL 诊断件留 .tmp 工作区；探针登记件 g31_blocked_probes_2026.json 落 milestones/ 非 evidence/ 路由面，check_schemas 零消费） |
| g31_mesh_vs_raster_bench_evidence_schema.json | ci/g31_mesh_vs_raster_bench.py（波 C Task C16 本批；M61 ③ mesh HW vs VS 光栅 measured 对照门裁决件——RFC-0034 三项闭集之③唯一新面；PASS-only 闭集 schema，FAIL 诊断件留 .tmp 工作区不进 evidence/ 路由面） |
| g31_rejudgment_windows_evidence_schema.json | ci/g31_rejudgment_smoke.py（波 C Task C16 本批；重判窗批量执行门裁决件——六窗 verdict 与证据指针完整性机核；PASS-only 闭集 schema；判档登记表 g31_rejudgment_windows.json 落 milestones/ 非 evidence/ 路由面，check_schemas 零消费） |

ci/check_schemas.py 路由 = 三处纯追加（load / validator / 前缀路由 `g31_wave_a_anchor_check_` + `g31_wave_a_soak_`，与既有 g31_* 全族及 gpu fallthrough 互不包含，既有路由 0-byte）。波 B Task B3 同律两组三处纯追加（前缀路由 `g31_slab_wiring_gate_` 置于 `g31_slab_wiring_` 之前——前缀包含长前缀先匹配；与本表全族前缀互不包含）。波 B Task B1 同律三处纯追加（前缀路由 `g31_hzb_wiring_`，与本表全族前缀及 gpu fallthrough 互不包含）。波 B Task B4 同律两组三处纯追加（前缀路由 `g31_texture_sampling_gate_` 置于 `g31_texture_sampling_` 之前——前缀包含长前缀先匹配；与本表全族前缀互不包含）。波 B Task B2 同律三处纯追加（前缀路由 `g31_restir_wiring_`，与本表全族前缀互不包含）。波 B Task B5 同律三处纯追加（前缀路由 `g31_skinning_wiring_`，与本表全族前缀互不包含）。波 B Task B8 同律一处纯追加（`g32_baseline_` 快检件跳过路由——同 `g31_baseline_` 律，budget_eval eval_entry 通用路消费，无映射前缀跳过）。波 C Task C9 同律三处纯追加（前缀路由 `g31_ngx_decomposition_`，与本表全族前缀及 gpu fallthrough 互不包含）。波 C Task C2 同律三处纯追加（前缀路由 `g31_renderer_docs_`，与本表全族前缀及 gpu fallthrough 互不包含）。波 C Task C3 同律三处纯追加（前缀路由 `g31_capability_fallback_`，与本表全族前缀及 gpu fallthrough 互不包含）。波 C Task C4 同律三处纯追加（前缀路由 `g31_robustness_`，与本表全族前缀及 gpu fallthrough 互不包含）。波 C Task C1 同律三处纯追加（前缀路由 `g31_renderer_sdk_`，与本表全族前缀及 gpu fallthrough 互不包含——`g31_renderer_sdk_` vs `g31_renderer_docs_` 同享 `g31_renderer_` 公共前缀但互为完整前缀串互不包含，startswith 全串匹配语义下两路由各自独立）。波 C Task C8 同律三处纯追加（前缀路由 `g31_support_policy_`，与本表全族前缀及 gpu fallthrough 互不包含——同 s 头族 `g31_slab_wiring_`/`g31_skinning_wiring_` 第三字符分岔 sl/sk/su）。波 C Task C6 同律三处纯追加（前缀路由 `g31_vendor_license_`，与本表全族前缀及 gpu fallthrough 互不包含——v 头唯一无同族分岔）。波 C Task C5 同律三处纯追加（前缀路由 `g31_sdk_dist_`，与本表全族前缀及 gpu fallthrough 互不包含——`g31_sdk_dist_` vs `g31_skinning_wiring_`/`g31_slab_wiring_` 第三字符分岔 d/l/n，`g31_renderer_sdk_` 前缀含 `renderer_` 段与本前缀全串互不包含）。波 C Task C7 同律三处纯追加（前缀路由 `g31_profiling_`，与本表全族前缀及 gpu fallthrough 互不包含——p 头唯一无同族分岔）。波 C Task C10 同律三处纯追加（前缀路由 `g31_rd027_poison_guard_`，与本表全族前缀及 gpu fallthrough 互不包含——r 头族 `g31_renderer_docs_`/`g31_renderer_sdk_`/`g31_restir_wiring_` 第六字符分岔 d/e/e，`g31_rd027_` 数字段唯一无同族包含）。波 C Task C11 同律三处纯追加（前缀路由 `g31_p4_streaming_`，与本表全族前缀及 gpu fallthrough 互不包含——`g31_p` 头族 `g31_profiling_` 第四字符分岔 4/r，`p4_` 数字段唯一无同族包含）。波 C Task C15 同律两组三处纯追加（前缀路由 `g31_ser_gain_estimate_` 置于 `g31_rt_pipeline_` 之前——序仅为可读性，两前缀全串互不包含：`g31_rt_pipeline_` r 头族第六字符 t 唯一，`g31_ser_gain_estimate_` s 头族第四字符 e 唯一，与 `g31_slab_wiring_`/`g31_skinning_wiring_`/`g31_support_policy_`/`g31_sdk_dist_` 全串互不包含）。波 C Task C17 同律三处纯追加（前缀路由 `g31_blocked_probes_` 置于 `else: gpu_validator` 之前——b 头族 `g31_baseline_` skip 路由第六字符分岔 a/l 互不包含；并发覆写修复经 .tmp fix 脚本幂等重放在案）。波 C Task C16 同律两组三处纯追加（前缀路由 `g31_mesh_vs_raster_` + `g31_rejudgment_windows_` 置于 `else: gpu_validator` 之前——m 头族 `g31_m98_l4_` 第六字符分岔 9/e〔mesh_ 全串唯一〕、r 头族 rejudgment_ 段唯一，与本表全族前缀及 gpu fallthrough 互不包含；落地经 .tmp io.open 幂等补丁 + check_schemas 立即验证 PASS 在案）。

## 4. 编号纪律

- CI 数字步骤：本波零消费（next_free=525 实测维持；后续波若进 pr-smoke.yml 按 actual next_free 顺位领取，禁预占）。
- RFC/RXS/RD/U/SG/MR/D/RX_error 共享段：本波零消费（波 A = 既有语义面实现与验收，零新条款零新 RFC——详见 G31_CONTRACT front matter rfc_required）。
