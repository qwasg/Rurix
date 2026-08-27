# 设备兼容矩阵与能力降级链（G31+ 波 C Task C3）

> **机器可读规范事实源** = [`milestones/g31/g31_compatibility_matrix.json`](../../milestones/g31/g31_compatibility_matrix.json)（本文 = 人类可读渲染面，数字/字面以 JSON 为准；append-only 纪律同 JSON）。
> 兑现面 = G31_PLUS_COMMERCIAL_RENDERER_TODO §5 #50；规范降级裁决 = `src/rurix-render/src/capability_matrix.rs`（`resolve_chains`，fail-closed 单测 mock 覆盖每链缺失面）；统一探测面 = `src/rurix-rt/src/bin/vk_capability_report.rs`（schema `rurix.g31.capability_report.v1`）。

## 1. 运行时能力探测统一面

单一 capability report 聚合既有探测逻辑（不重复造）：

| 探测面 | 载体 | 覆盖 |
|---|---|---|
| 设备能力 | `rurix_rt::vk::probe_device_capability`（vk.rs G31 C3 聚合段，instance 级不建 device 句柄，U56 体例） | vendor/device id、deviceType、api/driver version、扩展全量枚举、feature 链九节点（rayQuery / rayTracingPipeline / accelerationStructure / taskShader / meshShader / descriptorBuffer / timelineSemaphore / synchronization2 / bufferDeviceAddress）+ shaderInt64、descriptor 面上限（maxPerStageDescriptor{SampledImages,StorageBuffers}）、显存 heap 求和 + `VK_EXT_memory_budget` heapBudget |
| DLSS 可用性 | `DlssVkSession::create` 320×180→640×360 真建 | Streamline 2.10.3 四 DLL 在树 + 装载 + NGX init + feature 创建全链（NGX 动态加载 fail-closed） |
| FSR 可用性 | `FsrDx12Session::create` 同口径真建 | FidelityFX SDK 2.0.0 双 DLL + D3D12 设备 + FSR 3.1.5 context（厂商中立臂） |
| TSR | 自研恒可用面（不发起额外探测） | kernels/g13_tsr_{resample,resolve}.rx 经 `vk::run_compute`；需求 = Vulkan compute，设备面非空即 available |

既有分散探测（`probe_cluster_acceleration_structure` CLAS 面 / `probe_execution_set_capability` DGC·ExecutionSet 面 / KernelWave W1~W3 snapshot / 各 `run_*` 内嵌 feature 链协商）维持各自 harness 专用，不并入本面。

## 2. 降级映射闭集（六链，每链 fail-closed）

能力缺时**确定性降级 + 登记**（reason 携带缺失件字面，裁决集 digest 可重现进 evidence）；禁崩溃、禁静默错图——梯底恒可选中（by construction）。

| 链 | 梯 | 能力需求（诚实来源） | 降级语义 |
|---|---|---|---|
| `upscale` | `dlss_sr` → `fsr_3_1_5` → `tsr_device` | DLSS/FSR = vendor session 真建事实；TSR 自研恒可用 | 逐级降档，梯底 TSR 恒可选中 |
| `hzb` | `on` → `off` | `rayQuery` + `accelerationStructure`（BLAS 分解 + TLAS 相机射线车道） | off = 高成本如实（无遮挡剔除，全量场景渲染） |
| `restir` | `restir_high` → `megalights_low` | 显存 ≥ **1073741824**（1 GiB，声明阈值 declared） | 低档 = MegaLights 均匀选灯（RIS M=1 恒等语义） |
| `gi` | `on` → `off` | `rayQuery` + `accelerationStructure`（GI kernel AccelStruct 形参） | off = 直接光现状语义 |
| `framegen` | `x3`/`x2` → `off` | 显存 ≥ **536870912**（512 MiB，声明阈值 declared；四缓冲 ≈100MB@1080p + 余量） | off = presented=real 双口径登记面维持 |
| `texture_sampling` | `textures` → `constant_material` | RT 双 feature + SSBO ≥ **12**（基座 7 + B4 五件侧表事实） | 常量材质 = textures off 车道现状语义 |

声明阈值 = 链定义自带策略常量（**如实标注 declared，非 measured**）；能力布尔与上限全部为探测真值。

## 3. 兼容矩阵格

| 格 | vendor | 状态 | 内容 |
|---|---|---|---|
| `nvidia-ada-rtx4070ti` | 0x10DE | **measured**（measured_local，2026-08-25 真跑） | 十 feature 全真；limits 1048576/1048576；heap 12576620544 / budget 11771314176；DLSS（NGX 310.5.2）/FSR（3.1.5）/TSR 全 available；全量最大请求六链全绿零降级 |
| `amd-desktop` | 0x1002 | **dev_env_degrade** | AMD 真卡缺硬件（**锚 G-MB1-6**，MB1 唯一存续尾门 open）；获得硬件后按同一探测面 + 六链裁决补测，禁 mock 冒充 measured |
| `intel-desktop` | 0x8086 | **dev_env_degrade** | 同 AMD 格纪律（同锚 G-MB1-6）；≥2 厂商真卡全链绿 = 硬件补齐后收尾判据 |

## 4. 真机超分臂切换实测（measured_local，RTX 4070 Ti）

门 `g31.waveC.capability`（`ci/g31_capability_fallback_smoke.py`）真跑三后端 `g14_3_pipeline_perf --bench --scene bistro-interior --tier 100`：每后端双跑 16+4 帧，同后端双跑 `last_frame_digest` 位级一致（可重现机器证明）+ `frame_ms_production_mean` measured 登记；实测数字见 `evidence/g31_capability_fallback_<ts>.json`（append-only，数字全来自真实命令输出）。

## 5. fail-closed 单测面

`cargo test -p rurix-render --lib capability_matrix`（12 项）：每链构造能力缺失 mock 事实 → 断言降级路径触发 + reason 携带缺失件 + 选中档恒 ∈ 梯闭集（组合遍历机核）+ 裁决集 digest 双跑可重现/事实扰动必变 + 注册表三向锚定（chain id/梯档字面/阈值数字/G-MB1-6 锚）。
