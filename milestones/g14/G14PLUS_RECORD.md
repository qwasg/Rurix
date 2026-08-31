<!-- Assisted-by: Claude Fable 5（G14plus 超大纠正优化案——波0 治理立项批） -->
# G14PLUS_RECORD — G14plus 超大纠正优化案叙事总档

> **性质**：G14plus（G14.8~G14.12 延续波集）叙事与指针总档。**本档不承载任何判据**——机器可核事实源恒为 [G14_CONTRACT.md](G14_CONTRACT.md) §8 只追加验收记录、[G14_ACCEPTANCE_MAP.md](G14_ACCEPTANCE_MAP.md)、[RFC-0030](../../rfcs/0030-g14plus-pipeline-structural-optimization.md) 与 `evidence/` 真跑件；本档每处结论均附 evidence/文档指针。
> **载体裁决**：G14_CONTRACT §7 裁决 7 字面二选一（G14.x 延续波 / G16+ 里程碑）取 **G14.x 延续波**——用户 2026-08-22 指令「一次性完成 G14 硬收尾，门禁严格全绿」字面指向 G14 期内收口，G16+ 另立里程碑与「硬收尾」语义相悖。

## 1. 立项授权（双授权字面逐字登记）

- **2026-08-22 用户指令（本案立项授权面）**：「帮我一次性完成G14硬收尾，要求门禁严格全绿。先优化再测试以减少工期，最大化并行且允许委派fable5的子agent。需要真实读取渲染出图判断画面质量，性能优化的同时不能削减画质。本次任务可附加为G14plus作为文档记录，允许派遣大量gork4.6的子agent进行大量项目探索和深度论文技术调研，且允许计划前委派多个fable5的子agent进行多计划设计。本次进程允许视为超越G类里程碑的超大项目纠正优化案，不需要考虑工作量，务必完成任务使项目达到预期」。
- **2026-08-19 用户全期授权面（契约 §7 裁决 2 既登记）**：「彻底完成对标UE5渲染器的目标……同时优化渲染管线效率，使帧率对标UE5略高（不降级画质）……最终交付产物需要真实可商用，否则不要停止优化，并在此时允许在G15后无限制新建里程碑继续优化」。
- **目标判据（不改 G14 既有判据字面）**：M-d 门 18 格通过线 ×1.00 全部达标（严格全绿）+ 画质零降级守护（G13 锁定对拍 deficit 基线带内 + G14.3 车道锚带 ≤0.010779849285388998）+ 固定 seed 位级确定性协议成立（RD-045 修复）+ G14.5a soak / G14.5b closeout 全绿收口。

## 2. 波次结构与依赖（治理裁决快照）

| 波 | 名称 | 内容 | 验收面 | 依赖 |
|---|---|---|---|---|
| 波0 | 治理立项批 | 异己面 patch 存档清场 + P2 表后事件登记 + 本档首建 + RFC-0030 起草/D-409 评审/Approved + M-h 门 materialize（步骤 265）+ 契约 §8.8 | 治理文件齐备 + 守卫套件全 PASS | — |
| G14.8 | 测量与确定性基线波 | 锁频环境控制 + 环境画像登记 + flip-trace 诊断臂扩展（TSR 车道）+ RD-045 基线漂移率 N=20 + 根因定位取证 | §8.9 验收记录 + 诊断产物 | 波0 |
| G14.9 | host 面与 RT 白给波 | readback HOST_CACHED 选择器 + FIF=2 submit/collect 分离 + AS PREFER_FAST_TRACE(+compaction) + rurixc first-hit 内建与阴影臂切换 + 背光 keep 跳射线 + TSR kernel 变体（8×8） | 逐项 L0 位级探针（digest==旧锚）+ §8.10 | G14.8（确定性可信后验位级） |
| G14.10 | 帧循环重构波 | TSR 并 session（RD-045 候选根因消除）+ 生产帧关回读/锚点帧 + mv GPU 化（RFC-0030 §4.1）+ cornell 样本拆散重排 + vendor 原生 VkImage tag（可选） | L1 验证（SSIM+AI 读图）+ §8.11 | G14.9 + RFC-0030 Approved |
| G14.11 | 结构条件波（**仅当 G14.10 后 M-d 探针复跑仍有未达格**） | bistro 光栅 G-buffer 主可见性 + 保守光剔除 + 阴影结构阶梯（RFC-0030 §4.4 禁跳级） | 画质锚带复核硬前置 + §8.11b | G14.10 后决策点 |
| G14.12 | 复测收口波 | digest 锚 18 格重收割（三证）+ soak/closeout 特判达标分支修订 + M-c→M-d（18/18 判定）→M-a/M-b/M-e/wave 门复跑 → 收口基线 commit → soak full-run（≥1800s）→ M-h → closeout READY → §8.13 → flip → tag | M-h 门 + closeout 八 facts + §8.12/§8.13 | 全部优化波落定 |

**G14.8~G14.11 不设新 P0 门**——契约 §4.2 M-c 判据字面（「异步回读/提交面重叠 + 逐帧同步消除 + TSR device kernel 效率面」）本身即优化改动的法定验收对象，M-d 判据为全案终审裁判；各波退出事实由 §8.x 验收记录承载当波复跑 evidence。**唯一新门 = M-h**（附录 A：锚重收割合法性三证 + 18/18 达标 + RD-045 登记完备，步骤 265）。

## 3. 优化清单（十项技术改动 × 承接锚 × 所属波）

| # | 改动 | 承接锚 | 波 | 预期收益（取证来源） | 画质/确定性等级 |
|---|---|---|---|---|---|
| 1 | readback HOST_CACHED 分路选择器 | G14-N9（表后登记窗口提前） | G14.9 | bistro scene host 余 19/32/71ms → ~2/4/8ms（DLSS 同型先例 ~1.8GB/s） | L0 位级不变 |
| 2 | TSR kernel 变体 8×8（原 LocalSize 1,1,1） | G14-N17 重评（访存结构演进）+ RFC-0030 §4.5 | G14.9 | bistro TSR upscale ~120ms 锁死 → 正常 compute 量级 | L0 位级不变（逐像素独立） |
| 3 | AS PREFER_FAST_TRACE + compaction | RFC-0030 §4.8 登记面 | G14.9 | RT 遍历 1.1~1.4×（bistro 105 万三角） | L0 试探（漂移即弃） |
| 4 | rurixc first-hit 内建 + 阴影臂切换 | RFC-0030 §4.6 | G14.9 | 阴影段 1.3~2.0×（closest-hit 全遍历 → 早退） | L0 位级不变（存在性等价） |
| 5 | 背光 keep 预判跳射线 | RFC-0030 §4.4 L2 ① | G14.9 | 阴影射线 −20~50%（背光/切向面） | L0 严格位级不变 |
| 6 | FIF=2 submit/collect 分离 | G14-N10（窗口提前） | G14.9 | host 残余与 GPU 重叠 | L0（500 帧逐帧序列比对） |
| 7 | TSR 并 session + 生产帧关回读 | G14-N9/N12 联动 + RFC-0030 §4.5 L2/§4.3 L3 | G14.10 | 消 TSR 输入再上传 + 全帧回读归零；RD-045 候选根因消除 | L0 |
| 8 | mv GPU 化 | G14-N11（RFC-0030 §4.1 显式修订行） | G14.10 | mv 3.2~12.4ms → ~0.2ms | L1（ULP 微差，锚重收割） |
| 9 | cornell 16 样本拆散重排（3-pass 同序求和） | G14.4 取证 f 条 + RFC-0030 §4.4 | G14.10 | cornell RT 段 ~1.5×（延迟隐藏） | L0 位级设计（偏差即回退） |
| 10 | 光栅 G-buffer 主可见性 / 光剔除 / 样本阶梯 | G14-N13/N14（RFC-0030 §4.4 条件条款） | G14.11（条件） | bistro GPU 段至 ~3ms 预算 | L1~L2（锚带硬前置） |

## 4. 波次记录（逐波追加）

### 波0 治理立项批（2026-08-22）

- **异己面处置档案（G14-N6 处置形态变更登记）**：14 个已跟踪异己文件（`src/rurix-rt/src/render_exec.rs`〔F1 D3D12 import 脚手架，`--ignore-cr-at-eol` 实质 +378/−6〕+ 5 个 `external_import: None` 配套 + 5 个研究面 mod.rs 挂载 + `evidence/d3d12_interop_smoke.json` + `milestones/g12/g12_pt_sampler_selection.json`〔异己 timestamp〕+ `ci/check_schemas.py`〔纯 CRLF 漂移，-w diff=0〕）经 `git diff HEAD --output` 导出 patch **双份存档**后 `git checkout` 回退到 HEAD：
  - 存档①：`K:\rurix-ext\archive\alien_worktree_20260822\alien_tracked.patch`（1,514,122 bytes，sha256:5bd8ebfa6f580b90fab0370c0eb2f8522a612f284613b25f0f4d30282e7f509e）；
  - 存档②：`.tmp/g14plus_archive/alien_tracked.patch`（同内容副本，不入 commit）；
  - 6 个 untracked 研究面文件（ktx2_read/hzb/restir/sdf_trace/smrt/ssr，共 ~96KB）同步复制入两存档目录；原文件保留工作树（mod.rs 回退后不在编译树，无害零消费）；
  - 完整性核验：`git apply --check --reverse` exit=0（patch 可逆向应用 = 异己会话可随时 `git apply` 恢复）；
  - 恢复路径（归属会话自理）：`git apply K:\rurix-ext\archive\alien_worktree_20260822\alien_tracked.patch`；
  - 三张 G14 登记表（g14_budget / g14_fps_gap_registry / g14_ue_variance_samples）为 G14 车道门产刷新与只追加样本，**保留工作树态**（回退将删除合法只追加样本；G14.12 复测将全量重写）。
- **治理产物**：RFC-0030 v1.0 Agent Approved（[rfcs/0030](../../rfcs/0030-g14plus-pipeline-structural-optimization.md) + [D-409 评审](design/rfc0030_adversarial_review.md)）；P2 表后事件登记（G14plus 立项条，[G14_P2_DECISIONS](G14_P2_DECISIONS.md) 表后区）；MAP 附录 A M-h 行（步骤 265 实测领取）；`ci/g14_continuation_closeout_smoke.py` + schema materialize；契约 §8.8 立项记录；ledger v1.150 校准（RFC 30 消费 + CI_step 265 消费）。

### G14.8 测量与确定性基线波（2026-08-22）

- **锁频降级如实登记**：`nvidia-smi -lgc 2400,2400` 普通权限被拒（exit 4）；UAC 提权尝试被用户取消（"The operation was canceled by the user"）——**锁频不可用**。降级处置（方案登记面）：环境画像登记（`nvidia-smi --query-gpu` 时钟/温度读取面可用）+ 重型 bench 前冷却门控（热态等待）替代；M-b/M-d 复测时环境画像入 receipt。UE 臂基线重测取消（锁频不可用后无新环境态，M-d 复测时自然重测同口径 UE 臂——裁决登记）。
- **flip-trace 诊断臂扩展落地**（RD-045 backfill_condition 字面动作，RFC-0030 §4.2 L1）：`g14_3_pipeline_perf.rs` bench 腿 env `RURIX_G14_FLIP_TRACE=<dir>` 逐帧 digest 轨迹追加写 `frame_digests_<scene>_t<tier>_<backend>.jsonl`（digest 本就逐帧计算，trace 仅多一次文件追加，数据面位级零漂移）。
- **RD-045 基线漂移率 N=20**：HEAD 基线码面（47cd0750 副本 binary `.tmp/g14plus_rd045/g14_3_pipeline_perf_head.exe`）bistro-interior/t50/tsr_device 20 轮进程级独立 bench（~101s/轮，逐轮末帧 digest 对 stage_a 锚）——**drift=0/20**（统计诚实：p≈1.9% 历史检出率下 N=20 零检出置信 ~68%，快筛非闭环证据；full log `.tmp/g14plus_rd045/summary.txt`）。前两次脚本启动失败留痕（缺 RURIX_VK_VALIDATION env / --expect-digest 语义误用为契约 digest 断言——修正后重跑，失败轮 0 有效数据）。

### G14.9 host 面与 RT 白给波（2026-08-22）

三个并行实施域（波序与 L0 验证协议沿 RFC-0030 + 方案 C 分级验证）：

- **编译器域**（rurixc 13 文件 + conformance 4 文件）：`ray_query_initialize_first_hit` 内建全链——HIR `InitializeFirstHit` 变体 + MIR `Rvalue::RayQueryInitialize` 加 `first_hit: bool` 字段（默认 false 路径 W1/W2 五 kernel golden manifest 重编 **BYTE-IDENTICAL** 位级 0-byte 实证）+ vulkan_codegen 按字段发射 RayFlags `0x1|0x4=0x5` + `ray_query_check` 三态协议 by-construction 列入初始化集 + dxil/PTX/host 腿通配自动同态拒绝（零改动）+ conformance accept/reject 语料（reject 裁决：committed_t 语义警示不适用错误码机制，改用 S3 未守卫真实错误 RX3018）+ trace_matrix 再生成 388/388；`cargo test -p rurixc` 524+ 全绿含 5 个新单测。
- **执行器域**（render_exec.rs + vk.rs）：① FIF=2 submit/collect 分离——`execute_persistent_frame` 拆 `submit_persistent_frame`+`collect_persistent_frame`（代码逐字搬移顺序路位级等价，GPU 真跑两帧 readback 逐位相等实证）+ 公共 API `submit_with_frame_update`/`collect`（FrameTicket 线性令牌）+ per-slot cmd/query 区间/上传/回读 staging（懒建，顺序 session 零增量）；② readback HOST_CACHED 选择器（`create_device_buffer` 加 `prefer_cached` 参数，优选 HV|HC|CACHED 缺型回退）;③ AS flags（BLAS PREFER_FAST_TRACE + TLAS 叠加）。`cargo test -p rurix-rt` 209 过/2 败——两败为 HEAD 基线既有（m103_descriptor_buffer_ffi_layout_anchors 常量锚 + binding_supply_chain Cargo.toml vulkan feature 断言），非本批引入，如实登记待查。
- **kernel 域**：TSR 调度变体 `g14_8_tsr_resample/resolve.rx`（`#[numthreads(8,8,1)]` 2D + 越界门，数学全式与 g13_tsr_* 逐字同源；原 g13 kernel 0-byte 保留）+ bin 侧 dispatch 2D 化与 SPV 路径切换 + M-c 门脚本 SPV 路径同步；`g14_3_direct_gi.rx` 背光 keep 预判跳射线（quad/point 双臂，恒 0 keep 门乘掉 vis 位级不变）+ 阴影臂 first-hit 切换（spirv-dis 字面：主射线 `%uint_1` 维持 / 两处阴影 `%uint_5`）。
- **L0 位级探针与两起实测归因修订（诚实登记）**：
  1. **AS PREFER_FAST_TRACE 漂移即弃**：bistro t50 tsr 末帧 digest ≠ 锚（c099fc86… ≠ cd35a878…）——bisect（revert flags 单变量）确认 AS flags 为漂移源（105 万三角共面 tie-break 随 BVH 遍历序改变；cornell 36 三角无 tie 故 PASS）；按 RFC-0030 §4.8「漂移即弃」字面**放弃本项并 revert**（vk.rs 两处回退基线 flags，探针注释留痕）。
  2. **SSBO 本体切 cached 的 GPU snoop 惩罚修订**：初版把 Readback::Buffer 引用的 SSBO 本体切 HOST_CACHED——实测 bistro t50 scene GPU 8.58→30.5ms（≈3.5×劣化，GPU kernel 散写 snooped 内存 cache 一致性惩罚；HEAD kernel + 新 render_exec 隔离测试归因排除 kernel 改动嫌疑）；修订 = session SSBO 本体恒保持 WC（HEAD 行为 0-byte），cached 优选仅用于 staging 类用途；输出 SSBO 的 DEVICE_LOCAL 终态 + 锚点帧 staged 回读归 G14.10。
  - 修订后 L0 终态：**cornell t67 tsr 双跑 converged digest 三方一致 PASS**（== pre 锚 e9bc79a7…）+ **bistro t50 tsr 末帧 digest == stage_a 锚 PASS**。
- **性能过渡态登记**（G14.10 前不作对标输入）：bistro t50 tsr prod 156.06ms（vs 立项基线 139.67——TSR OUT_COLOR 24.9MB 回读走 WC 的过渡态；TSR kernel 8×8 已实证生效〔cached 对照轮 upscale 120.29→38.24ms 旁证〕，回读/上传税待 G14.10 并 session + 关生产回读一次性消灭）；scene GPU 12.49ms（vs 8.58 基线 +3.9ms，first-hit/跳射线在 bistro 点光臂的微扰待 G14.10 后复核归因）。

### G14.10 帧循环重构波（2026-08-22，四子批：主体/10b external memory/10c TSR 形态+拆散/10d 驻留〔在途〕）

- **主体批（b75c773d）**：tsr 臂统一四 pass 单 session（scene→mv→resample→resolve，GPU 链内零 host 往返；测量循环零回读+末帧锚点回读；render 腿逐帧回读出 EXR 维持）+ mv GPU 化（`kernels/g14_mv.rx` host 机械转写 + bin 侧 SPV NoContraction 注入；digest 变化归因完整 = Vulkan FDiv 2.5 ULP 规范容差在 miss 像素病态反投影放大，depth+1ULP 敏感度实验实证，L1 级预期内）+ vendor 驻留输出加性 API（退档方案 B——DLSS/render_exec 各持独立 VkDevice 如实登记；upscale_resident/readback_output_into；pack kernel 不可行登记 = .rx 无 f16/storage image/位 reinterpret）。**bistro t50 tsr prod 156→29.0ms（5.4×）**；L1 画质验证 = vs G13.4 锚 deficit 0.0053891 带内 + vs pre SSIM 0.9999995 + AI 读图 C1~C6 全过（READLOG 条目 2）。dlss 臂驻留输出接线：**bistro t67 dlss prod 99.73→64.43ms**，末帧 digest == stage_a 锚逐字（resident 路径位级同实证）。
- **10b external memory 批（agent 交付）**：render_exec exportable 纹理导出面（`new_with_exportable_textures`/`export_texture_win32_handle`/LUID 对拍；OPAQUE_WIN32=0x2 踩坑登记）+ DLSS 侧 Win32 导入与 `upscale_resident_external`（SL 代理 device 扩展注入实测通）+ 跨 device release/acquire（QUEUE_FAMILY_EXTERNAL，GENERAL 恒定 layout 协定）；**SL 格式探明 = RGBA32F color + R32F depth 第一臂即过（全 f32 零转换路线成立，host pack f16 全链可消）**；跨 device 闭环单测位级一致 + 214 测试全过。FSR 方向性不可行登记（OPAQUE_WIN32 非 D3D12 可导入句柄；正确路线 = 反向 D3D12 建 shared resource→VK 导入 = 导出面镜像工程，二期登记，FSR 臂维持 host 链）。
- **10c TSR 形态 + cornell 拆散批（e2d84cd3）**：TSR LocalSize 8×8→**32×4**（六候选形态扫描全 digest 位级不变；warp=完整 32 宽输出行整 warp 合并；bistro tsr_gpu 27.3→12.7-16.4ms、cornell t67 tsr 2.19→1.30ms）+ **根因定位 = create_device_buffer 恒 HOST_VISIBLE（PCIe ~25GB/s，TSR 每帧 135MB 净流量物理下限 7-8ms）→ G14.10d DEVICE_LOCAL 驻留立项**；语言面登记 = shared let 语法在（RXS-0024/0079）而 vulkan_codegen 无 Workgroup 降级（RFC 面次优先）。cornell 拆散六 pass 车道（primary/scatter〔16 层每 invocation 1 条 first-hit ray〕/reduce〔固定层序求和位级同序〕）：**digest == 统一车道锚 86dda848 逐字一致**（位级同序设计完全兑现）；**cornell t100 tsr prod 27.73→7.944ms（3.5×）**（scene_gpu 5.72+mv 0.13+tsr 1.42——scene 残余 = scatter/reduce 逐 invocation 读 tris/quads 走 PCIe，10d 驻留消面）。
- **10d SSBO DEVICE_LOCAL 驻留批（df6c985d 前半）**：三路内存机核（DeviceLocal/HostWc/HostCachedPreferred）+ 创建期 one-shot staging 上传 + readback staged copy；**bistro t50 tsr prod 21.47→1.227ms（581fps，UE 线 3.274ms 首格达标 ratio 2.67）** + cornell 锚 digest 逐字命中 + A/B bisect 位级零漂移实证。
- **10e dlss 驻留统一车道批（df6c985d）**：手编 pack SPV（SSBO→storage image）+ exportable 三纹理直写 + DLSS OPAQUE_WIN32 导入驻留 evaluate；bistro t67 dlss prod 15.62→2.56ms。**读图验证抓获输出全毁（本批 PASS 判据只有 digest 双跑一致——错位是确定性的故门全绿）→ 10f 修正**。
- **10f dlss buffer 共享重构 + 曝光显示域修正批（2da40f4c）**：AI 读图抓获两缺陷——①**OPAQUE_WIN32 OPTIMAL image 跨 device 布局解释不一致**（render_exec 侧 dump 正常、DLSS 侧 device 回读同一 memory 为确定性块状乱序，双侧 dump 对拍实锤；NVIDIA 上跨 VkDevice 的 OPTIMAL tiling swizzle 不保证一致）→ 弃 image 共享改 **exportable buffer**（线性布局无歧义；pack SPV 改 PackHalf2x16 RTE 写 u32 对 = RGBA16F 8B/px 紧凑，host `f32_to_f16` 同语义；depth/mv SSBO 紧凑布局直接导出；DLSS 侧 `import_win32_buffer_input` + `upscale_resident_buffers` = acquire×3 + `vkCmdCopyBufferToImage`×3 进 session 自建 input image——输入位面回归 host 路径 f16/D32/RG32F 语义）；②**vendor 臂输出 scene 域 vs tsr 臂显示域语义分裂**（TSR resample `o=v·exposure` 契约字面 vs vendor 直通；bistro ev100=−4 → vendor 输出暗 2^4=16×，G14.3 以来存量缺陷；G13.4 M-a 门锚定 scene 域语义 + 度量侧 ×2^(−ev100) 尺度链故从未显形）→ pack SPV `rgb×exposure` 转显示域（push constant 第 3 字段；**host `pack_vendor_inputs` 共享面零触碰保 M-a 锚**）。验证：cornell（ev100=0，×1.0 IEEE 位恒等）converged digest 修正前后逐字同 = 位保持判据；bistro 亮度对齐 tsr（lum mean 0.00961 vs 0.00984，HDR max 13.3 恢复）；双场景读图 PASS（cornell 结构锐利、bistro 吊灯/吧台/桌椅全可见）。性能：bistro t67 dlss 2.88ms（ratio 1.13）/cornell t67 dlss 0.90ms（ratio 2.84）双 PASS（较 image 版 +0.3ms = DLSS 侧三条 copy 代价，可接受）。**教训登记：「digest 双跑一致」不能替代内容正确性验证——确定性的坏内容照样全绿,AI 读图为改图波硬门（方案 C 协议追认为强制）**。fsr 臂并行进行中（fsr 执行域 D3D12 SHARED staging buffer 方案；bin 侧 create 暂 stub + `RURIX_G14_FSR_HOST=1` host 链逃生门）。
- **10 波中间图景（60 帧快扫,2da40f4c 时点）**：tsr 6 格全 PASS（ratio 1.72~9.13）+ dlss 6 格全 PASS（1.06~3.66，10f 后 bistro t67 实测 1.13）+ fsr 6 格 MISS（host 链 0.06~0.82，G14.11 主战场）。

### G14.11 结构条件波（2026-08-22；触发条件命中 = G14.10 后 fsr 6 格未达）

- **fsr 臂 D3D12 反向共享驻留（9e144bbd）**：texture 直共享首选案经内容对拍 + 读图实锤弃案（NVIDIA 上 D3D12_RESOURCE handle 导入 OPTIMAL VkImage 的跨 API tiling 解释不一致，D3D12 侧读为确定性条纹乱序——与 dlss 臂 OPAQUE_WIN32 弃案**同族**，两次独立踩中同一硬件面事实）→ 落 **D3D12 SHARED staging buffer** 形态：D3D12 建 committed BUFFER（三段 256B 行距：color f16 RGBA / depth f32 / mv f32 RG，64KB 对齐）→ `CreateSharedHandle` → Vulkan `D3D12_RESOURCE_BIT` + dedicated 导入 bind 为 SSBO → pack SPV v2 按行距直写（color rgb ×exposure 后 PackHalf2x16；depth/mv Bitcast 位拷贝）→ 帧末 EXTERNAL release → D3D12 侧逐帧 3× `CopyTextureRegion` 搬入三输入纹理（formats 与 host 链逐字同）→ ffx dispatch。CPU 序跨界同步（dlss 车道同律），LUID + staging 布局双对拍 fail-closed。**6 格全绿 ratio 1.65~3.55（bistro t67 host 链 31.71ms → 1.46ms，21.8×）**；digest 双跑位级一致；cornell exposure=1.0 digest 位保持；bistro 亮度对齐 tsr（0.00966 vs 0.00992）；双侧内容对拍真差异 0（352,947 值）；读图非乱序；tsr/dlss digest 逐字零回归。
- **`f16_to_f32` subnormal 解码缺陷修复（同批）**：fsr 双侧对拍暴露 173 处 a=2b 表观差 → 归因 = 解码函数 `e` 初值 −1 使 f32 指数字段恒 113−k 少 1（应 113−k），**全体 f16 次正规数（<2⁻¹⁴≈6.1e-5，即深阴影像素）解码为正确值的一半**；该函数被 host 链与 vendor 输出转换共用、涉 digest 锚（G14.12 统一重收割吸收）。加**全 65536 位型枚举 vs 位精确公式**回归锚测试永久钉死。治理登记：G13.4 M-a 门为容差带口径（无输出 digest 锚），既存 evidence 不因本修复失效，深阴影修正为严格正确性改善。
- **DLSS 侧批量屏障（同批）**：三条 buffer→image copy 原各夹两道全局 `vkCmdPipelineBarrier`，把本可并发的三条 copy 串成「copy→流水 drain→copy→drain→copy→drain」；合并为 [3 barrier]→[3 copy]→[3 barrier]（acquire 同理）——**数据面零变化 digest 不变**，bistro t100 evaluate CPU 0.25→0.055ms、upscale 3.02→2.977ms。附 reactive 恒零内容驻留跳过 + `RURIX_VENDOR_TIMING` 驻留路分解遥测 + `RURIX_G14_DLSS_SKIP_COPY` 诊断门。
- **测量口径勘误**：中间快扫脚本内联的 UE 线有误，真值 = 最新 M-d evidence 逐格 `ue_median_ms`（cornell t50/67/100 = 2.193/2.141/2.054；bistro = 3.274/3.431/4.322），已按门事实源校准；本波前的"MISS/PASS"判读以校准后为准。
- **末格攻坚（bistro t100 dlss）与根因勘误(37ac2688 / d6eab741)**：批量屏障后 prod 4.358 vs UE 4.322（ratio 0.992），时间解剖 = Vulkan 侧 GPU 1.08（scene 0.93 + mv 0.03 + pack 0.12）+ 墙钟 1.381；DLSS 侧 2.977（submit_wait 2.70 = **三条跨设备 copy 0.6 + DLSS 网络 2.1**，经 `RURIX_G14_DLSS_SKIP_COPY` 差分实测分离）。攻坚中**推翻了本记录 §4 G14.10f 与 G14.11 两处的错误结论**——见下条勘误。三项优化收益:①消 copy(pack 直写三 exportable storage image)upscale 2.977→2.19~2.52,但 SSBO→image 引入每帧布局转换税,prod 仅到 4.24;②**跨界 image layout 跨帧常驻 GENERAL**(建面期 one-shot `UNDEFINED→GENERAL`,帧内初值 = 帧末收敛态,免每帧 3 次全表面压缩元数据重初始化)4.24→4.075;③**同形帧跳过命令体重录**(`readback_subset` 逐帧同形〔驻留车道恒 `Some([])`〕且无 TLAS/binding/push override 时命令体逐字节不变,原样重放)`cpu_record` 229µs→5.9µs、prod 4.075→**3.545**——②③是①的必要配套。终态 prod 3.846(7 样本均值;中位 3.911/最好 3.545/最差 4.032)、ratio 1.124(最差样本仍 1.072);7 跑 digest 位级一致;lum mean 0.009795 + 读图无乱序;四格无回归。治理裁决：**"按 tier 切 SL DLSS mode" 路线否决**——G13.4 契约 `ue_dlss_quality_map` 的 `tier_note` 字面限定该映射为 **UE 臂**插件质量枚举面，并规定 **Rurix 臂档位语义 = tier% 内部渲染分辨率**，改 mode 反偏离契约（另:NGX 日志实证 t100 in=out 时内部已走 `NGXDLAA::DLSS_GetOptimalSettings`,即本就在跑 DLAA 路径）。

> **⚠ 勘误(2026-08-23,凌驾本记录 §4 G14.10f「①跨 device 布局解释不一致」与 G14.11「texture 直共享…同族」两处结论)**：跨 device 共享 OPTIMAL tiling image 的块状乱序,**真因不是两个 VkDevice 对同一显存的布局解释不一致(该"硬件事实"判定错误)**,而是 `vendor_upscale.rs::import_win32_input` 的 `image_type: 2`(= `VK_IMAGE_TYPE_3D`,注释却写 "2D")与导出侧 `render_exec` 的 `IMAGE_TYPE_2D`(=1)不匹配——同一块显存被两侧按 2D/3D 两种布局解释。memreq 实证:1920×1080 RGBA16F OPTIMAL,**2D 需 17694720B、3D 需 16588800B**。改为 1 后跨 device image 共享**直接成立**,DLSS 侧三条 buffer→image copy(41.5MB/帧、0.6ms)整体消失,读图结构完整。同源笔误另存于 `mk_image`(session 自有 image,同 device 写读故内容自洽无可见损坏,但 3D image 上建 2D view 触 VUID-…-06728 十条 validation);裁决=修,实测 digest 逐字不变(纯收益)。
> **教训(与 G14.10f「digest 双跑一致 ≠ 内容正确」并列登记)**:**把可复现的自研缺陷误判为"平台/硬件固有限制"并据此绕道,代价是两条独立路线(dlss OPAQUE_WIN32 / fsr D3D12_RESOURCE)各绕一次远路 + 每帧 0.6ms 的常驻税**。判定"硬件不支持"前必须逐字段对拍两侧 create info(本例中导出/导入侧 `imageType` 字面不同,且错误侧注释与取值自相矛盾——代码审查即可发现)。
> **附带实测(供后人省一轮)**:硬件 `linearTilingFeatures` 全支持 ≠ NGX 接受——NGX 在 `vkCreateImage(LINEAR, fmt=97)` 返回 `VK_ERROR_FORMAT_NOT_SUPPORTED` 并崩于 `slEvaluateFeature`(内部另建副本不接受 linear 源),LINEAR 别名路线已弃并回退。`RURIX_G14_DLSS_VK_VALIDATION=1` 是陷阱:开启后 NGX 首帧崩在 `vkCreateCuModuleNVX`(validation 层 × NVX CUDA 模块 pNext 链兼容问题,与共享面无关),仅可用于建面期查错。

### G14.11 终态：18/18 全绿（2026-08-23 快扫，UE 线 = M-d 逐格 `ue_median_ms`）

| 场景 | tier | tsr_device | dlss_sr | fsr_3_1_5 |
|---|---|---|---|---|
| cornell-box | 50 / 67 / 100 | 8.030 / 6.706 / 4.097 | 3.192 / 2.342 / 1.895 | 3.501 / 3.058 / 2.376 |
| bistro-interior | 50 / 67 / 100 | 2.364 / 2.080 / 1.856 | 1.918 / 1.528 / **1.177** | 2.940 / 2.522 / 1.753 |

（表值 = ratio = UE ÷ Rurix prod；60 帧快扫顺序跑含热漂，最紧格 bistro t100 dlss 1.177。正式判定以 G14.12 的 M-d 门 160 帧×3 轮跨轮中位数为准。）

## 5. 复测轨迹（M-d 逐版 ratio 收敛表——逐波追加）

| 版本 | 时点 | 口径 | 达标 | cornell ratio 区间 | bistro ratio 区间 | 备注 |
|---|---|---|---|---|---|---|
| v1 | 2026-08-20 012652Z | 全量 | 0/18 | 0.0606~0.1504 | 0.0116~0.0221 | 首跑诚实红 |
| v2 | 2026-08-20 053525Z | 生产 | 0/18 | 0.0791~0.4363 | 0.0116~0.0456 | 生产口径双列（M-f） |
| v3 | 2026-08-20 122608Z | 生产 | 0/18 | — | vendor 重格 +19~+97% | vendor 并行化（M-g） |
| v5 | 2026-08-21 003053Z | 生产 | 0/18 | — | — | RD-045 首检出（bistro t50 tsr run1） |
| v6 | 2026-08-21 132325Z | 生产 | 0/18 | 0.0741~0.4566 | 0.0186~0.0595 | RD-045 复发（bistro t67 tsr run3）；G14plus 立项基线 |
| v_g14plus_md | 2026-08-22 183532Z | 生产 | **18/18** | 1.7796~8.4972 | 1.2096~2.8844 | G14.12 首判定（160 帧×3 轮；`parity.met_count=18`）；最紧格 bistro t100 dlss 1.2096；`evidence/g14_m_d_dual_end_fps_parity_20260822T183532Z.json` |
| v_g14plus_soak | 2026-08-23 051754Z | 生产 | **18/18** | 2.0701~8.2261 | 1.0831~2.7920 | soak 回归复跑同口径确认；最紧格 bistro t100 dlss 1.0831 仍 ≥1.00；`evidence/g14_m_d_dual_end_fps_parity_20260823T051754Z.json` |

（档内既有版本号惯例为 v1/v2/v3/v5/v6；`v_g14plus_md` / `v_g14plus_soak` 标识 G14plus 正式 18/18，不与 §4 六十帧快扫表混口径。）

## 6. 终审（G14.12 收口——达标定盘 + 遗留面）

closeout `VERDICT = READY`（`evidence/g14_wave5b_closeout_20260823T062927Z.json` 八 facts 全 PASS；`last_green_utc=20260823`）。

### 6.1 达标定盘

- **18/18**：`parity.met_count=18` / `unmet_count=0` / 通过线 ×1.00。首判定最紧格 1.2096；soak 回归最紧格 1.0831。空表终态：`g14_fps_gap_registry.json` `items=[]` 且双场景 `no_gap_explicit=true`。
- **锚重收割**：`reharvest.harvested_utc=20260822T183502Z` / `base_commit=1a9c561a4e6b41484f50c3a0f9c090933829d0ce` / `double_harvest_bitexact=true`。两次正式 M-d `stage_a_digest_drift_guard=true`。
- **画质**：最新 soak M-c `SSIM=0.99461088 deficit=0.00538912 ≤ 0.0107798`（`g14_m_c_rurix_pipeline_perf_20260823T044803Z.json`）。G13 双门 `g13_m_c_ue_upscale_parity_20260823T033850Z` / `g13_m_d_ue_lumen_gi_parity_20260823T043416Z` PASS。
- **soak**：58 迭代 / 1835.7s / sleep=0 / failures=0 / 9280 帧；5 P0 `base_commit` 同值 `afe090d5`；`budget_eval --strict` 244 pass。
- **M-h**：`g14_m_h_continuation_closeout_20260823T062856Z.json` 6/6 PASS。

### 6.2 遗留面：RD-045 观察窗

RD-045 维持 `status=open`，不因 18/18、锚重收割或 soak 零漂移关闭（`registry/deferred.json`；RFC-0030 §1 第 2 条 / MAP 附录 A M-h）。G14.10 已结构性消除候选根因面，根因未逐字定位；长窗生产化闭环归 G15+/G16+。本波 soak 58 轮 / 9280 帧零检出只证明该观察窗，不把条目判 closed。

### 6.3 遗留面：G15 承接锚

绝对画质通过线不在 G14 设立（契约 `out_of_scope.absolute_image_quality_pass_line` / MAP §7）。G13 超分登记表 8 行与 Lumen 登记表 2 行只消费不回写，逐项重评锚定 G15：

- Lumen `gap_id=2f6331a41404dfcd` cornell：`gi_energy_rel` delta=0.535625027781919，`indirect_ssim` b=0.033384483786469556，`indirect_flip` delta=0.6127988976249465。
- Lumen `gap_id=b7527c980cdd1d46` bistro：`gi_energy_rel` delta=2.964585170338064，`indirect_ssim` b=0.006566911636724374，`indirect_flip` delta=0.9671355491209283。

G15 法定输入 = 上述 8+2 行 + 本表 §5 18/18 定盘 + RD-045 仍 open 的观察窗，不得另起无锚差距面。

## 7. 收口后 RFC-0030 修订事件登记（只追加）

- **2026-08-30（G38 窗,G31+ TODO #90「FIF×动态共存」正式化）**：RFC-0030 §4.3 追加 **L2a 行**（FIF×动态,每槽 AS 副本 opt-in）→ **v1.1**。判档前置已兑 = `g31_fif_dyn_probe` 三臂等价门 GPU 双 PASS（Rebuild/Refit,evidence = `artifacts/day_0830_delivery/w3_deep/fif_dyn/evidence_fif_dyn_rebuild.json` / `evidence_fif_dyn_refit.json`,gates 六项全 true）；实现 = 加性 body-include `src/rurix-rt/src/render_exec_g37_fif_dyn.rs`（平行入口 `submit_with_frame_update_slot_as`）,§4.3 L2 既有字面与 `submit_with_frame_update` 拒绝面 0-byte。本档叙事指针登记,判据事实源 = RFC-0030 v1.1 + evidence 件 + G31+ TODO #90 行。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-22 | 首建（波0 治理立项批）：立项授权双字面 + 波次结构 + 优化清单十项 + 波0 处置档案 + 复测轨迹基线表 |
| v1.1 | 2026-08-23 | G14.12 收口终审：§5 追加 v_g14plus_md / v_g14plus_soak 18/18 行；§6 达标定盘 + RD-045 观察窗 + G15 承接锚 |
| v1.2 | 2026-08-30 | §7 收口后修订事件登记：RFC-0030 v1.1（§4.3 L2a 每槽 AS 副本 opt-in 加性行,G31+ TODO #90 判档双 PASS 前置） |
