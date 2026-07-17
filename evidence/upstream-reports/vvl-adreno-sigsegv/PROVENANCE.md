> **Status: DRAFT — do NOT file.** Owner review gate; agent does not file externally.

# PROVENANCE — VVL Adreno SIGSEGV 上游报告包

## 来源(素材 → 草稿字段映射)

- **崩溃逐字栈 + 诊断**:`evidence/mb1-android-ondevice/round1_halt_excerpt.md`(MB1 G-MB1-7 round-1 HALT 摘录,2026-07-16 17:00 场)。草稿的 backtrace 代码块、故障地址/信号解读、「栈顶 6 帧全在 layer / VUID 未吐即崩 / 非受控 abort」诊断要点全部逐字/逐义取自该文件。原始全量 buffer(`logcat_red_full.txt`,1,299,363 B)与 round-1 transcript 留 scratch 未入库(该摘录 §开头如是声明)。
- **环境画像 + layer 版本/provenance**:`evidence/mb1-android-ondevice/android_present_smoke_report.md` §1/§1.1/§1.3(设备 HONOR BKQ-AN10 / SM8850 Adreno / Android 16 SDK 36;`libVkLayer_khronos_validation.so` = vulkan-sdk android-binaries **1.4.350.1**,sha256 `34a741d5…`,26,345,704 B;NativeActivity APK 壳 + in-APK layer 加载机制)与 §2(round-1 机制描述)。
- **对照实验(同机干净变红)**:`evidence/mb1-android-ondevice/logcat_red.txt`(round-2,pName-00707 两行 VVL 输出逐字引入草稿 Control experiments 节)+ 主报告 §3。
- **项目侧定性**:`milestones/mb1/MB1_CONTRACT.md` §8 进度记录——「此崩溃本身是 layer 上游鲁棒性 bug 被 MTE 硬抓的独立证据,非本项目缺陷(逐字栈见 round1_halt_excerpt.md)」。
- **崩溃二进制 BuildId**(`13204c6e71811fabb9fd173b89b19c786d8337b4`)直接取自 tombstone 帧 #00;layer sha256 行记录于 round-2 provenance 表(同 session、同一 1.4.350.1 layer 二进制)——owner 提报前宜核对 BuildId ↔ sha256 对应关系。

## 定性(一句话)

VVL 1.4.350.1 在 Adreno/Android 16(硬件指针标记开启)上处理故意字节损坏的 SPIR-V 时,自身在解析/错误格式化路径踩已释放或错标指针,被硬件抓死为 SIGSEGV(SEGV_ACCERR),预期的 `VUID-VkShaderModuleCreateInfo-pCode-08742` 从未吐出——layer 上游鲁棒性 bug,非本项目缺陷。

## 日期

- 崩溃取证:2026-07-16(17:00 场,round-1 HALT)。
- 本备包整理:2026-07-16(EA1 支线 B,分支 `evidence/upstream-report-packs`)。

## 诚实边界

1. **无独立 MRP(提报前必补)**:当前复现依赖本项目 APK 壳(`com.rurix.vk` NativeActivity + `librurix_vk.so`),不依赖 Rurix 的纯 C/NDK 最小复现工程**尚未制作**——需真机(设备在 owner 手上),属 EA1 支线 B 后续工作。草稿 MRP 节已如实标 `<PENDING: standalone MRP not yet extracted>`;EA1 契约 G-EA1-7 明文允许「VVL 包若独立 MRP 依赖真机而设备不可得→该子项标 pending 不伪造」。
2. **上游 VVL 版本漂移风险**:取证钉在 1.4.350.1(2026-07-16);上游 invalid-SPIR-V 路径随时可能改动。提报前必须对最新 SDK / VVL main 重测;若不再复现则不提报。
3. **崩溃栈未符号化**:六个 layer 帧仅有 PC + BuildId;「use-after-free / MTE tag 错配」是从信号签名(`SEGV_ACCERR` + `0xb400…` tagged-pointer 地址 + `tagged_addr_ctrl PR_TAGGED_ADDR_ENABLE`)推断的项目侧诊断,非符号级证明。崩溃瞬间的精确 Vulkan 入口(vkCreateShaderModule vs vkCreateGraphicsPipelines)同样未定,草稿以 `<FILL>` 留白。
4. **损坏字节配方未存档**:具体损坏了哪些偏移/字节未记录于入库素材;损坏 `.spv` 样本是否留存亦未记载——须随独立 MRP 一并钉死。
5. **桌面对照**(取证素材层面缺失,但 EA1 支线 B 已实测补齐):原始归档素材中没有桌面 VVL 对照;**2026-07-17 EA1 支线 B 已在本机 NVIDIA + VVL 1.3.296 实测同类损坏字节 → VVL 崩溃(0xC0000005,3/3),VVL-off 干净**(详见下「EA1 支线 B 回填」节)。结论:桌面(无 MTE)也崩,崩溃非 Adreno/MTE 独有——此实测**超越**本条原「均为未知」的诚实快照(快照为取证时点,回填为 EA1 期新增)。版本差(桌面 1.3.296 vs 设备 1.4.350.1)与配方同类非逐字节等限定见回填节。
6. **待办已挂**:MB1 期内已通过 spawn_task 挂过「owner 复核提报此 VVL 上游 bug」的待办(见 memory `mb1-vulkan-backend`);本备包即该待办的证据整理落地,不构成提报动作本身。

## EA1 支线 B 回填 — 桌面对照实测(measured 2026-07-17)

**核心新发现:崩溃不是 Adreno/MTE 独有——桌面 VVL 无硬件指针标记也崩。** 本机 NVIDIA RTX 4070 Ti
(driver 620.2.0.0)+ VVL **1.3.296**(注册于 `HKLM\...\Khronos\Vulkan\ExplicitLayers` →
`C:\ti-localappdata\ti-build-cache\vulkan-1.3.296.0\Bin`)+ x86_64(**无 MTE**)下,将仓内既有确定性
字节损坏配方 `ci/vulkan_codegen_smoke.py:121-125`(`spv[20] ^= 0xFF`)施于 `rurixc --target vulkan`
产的合法顶点 `.spv`(`conformance/vulkan/accept/vk_tri_vs.rx` → 376B,byte20 `0x11→0xEE`),经
`rurix-rt` 的 `vk_triangle`(argv 收 vs/fs spv;`RURIX_VK_VALIDATION=1` +
`VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation`)喂 `vkCreateShaderModule`/pipeline:

- **VVL 开**:进程崩溃 `0xC0000005`(STATUS_ACCESS_VIOLATION;shell exit 139),**确定性 3/3**,
  stdout 空;与 Android 不同——桌面 VVL **先吐** `VUID-VkShaderModuleCreateInfo-pCode-08737`(spirv-val
  错误)到 stderr,**然后**才 access-violate。
- **VVL 关(对照的对照)**:同一损坏模块**干净跑**(exit 0,`VK_TRIANGLE: ok`)——NVIDIA 驱动容忍该
  模块,故崩溃归因于 **VVL 自身处理**,非驱动/harness 用坏句柄。
- **诚实边界**:桌面 VVL **1.3.296** vs 设备 **1.4.350.1**(版本差);桌面 VUID `pCode-08737` vs Android
  设计目标 `pCode-08742`;Android round-1 **精确**损坏偏移/值**未存档**(见下),故此为「同类配方」非逐字节
  重放。`spirv-val` 拒该损坏模块(word-5 OpAtomicSMax 解码越界)。逐字命令 + 输出留 scratch
  `…/scratchpad/vvl-control/`(good/tampered/no-vvl 三腿 + 3×重复,工具件不入库)。
- **意义**:回答了草稿此前的悬问「桌面(无 MTE)是否也崩」= **是**。VVL 非法-SPIR-V 处理路径在桌面亦
  crash-fragile;MTE 只是在 Android 上抓得更早更硬(`SEGV_ACCERR`,VUID 未吐即崩)。这使 bug 表观范围
  超出「Adreno / Android 16」——**标题/范围提报时或需上修,已在草稿 Control experiments 末尾 flag 给 owner**。

## `<FILL>` / `<PENDING>` 残留状态(ISSUE_DRAFT.md,EA1 支线 B 回填后)

| # | 位置 | 内容 | 状态 |
|---|---|---|---|
| 1 | Environment | Adreno 驱动 / Vulkan ICD 版本 | **pending**:tracked evidence 无版本串(仅 SM8850/Adreno系 + libvulkan.so 在位),须真机 vulkaninfo(设备在 owner 手上) |
| 2 | Environment | layer 设置确认(默认无覆盖) | FILL 保留:owner 核对取证配置 |
| 3 | Describe the issue | 精确字节损坏配方(偏移/值) | **部分回填**:引仓内确定性配方 `ci/vulkan_codegen_smoke.py:121-125`(`spv[20]^=0xFF`);Android round-1 **精确**偏移未存档,须随独立 MRP 钉死 |
| 4 | Describe the issue | 崩溃瞬间精确 Vulkan 入口点 | **pending**:符号器在位(NDK 27.3 llvm-symbolizer),但**无 VVL 1.4.350.1 符号包**(禁下载)→ 六帧未符号化;桌面对照旁证入口在 shader-module 创建路径(pCode-08737 VUID) |
| 5 | Control experiments | 桌面对照结果 | **已回填 measured**(见上;桌面 1.3.296 崩溃 3/3,VVL-off 干净) |
| 6 | Minimal reproduction | 独立 MRP | **pending**:2026-07-17 `adb devices` 无设备连接;纯 C/NDK 最小工程 + 真机复测须设备(G-EA1-7 明文允许标 pending) |

另:草稿末尾整节 "Pre-filing checklist" 为 owner 自查用,粘贴到 GitHub 前须整节删除;头部 `Status: DRAFT — do NOT file.` 行由 owner 在亲自提报时移除。

## 提报纪律

- **agent 只备包**(本目录即备包产物);**上游提报由 owner 亲自复核并执行,AI 不对外提交**。
- EA1 契约将 `upstream_filing` 列为 out_of_scope:本包在 EA1 期内仅作证据归档(D-EA1-8 / G-EA1-7),不构成提报动作。
- 提报时点须完整走一遍草稿 §Pre-filing checklist(独立 MRP、最新版复测、符号化、查重、桌面对照、`<FILL>` 清零)。
