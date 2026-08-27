<!-- Assisted-by: TraeCode:Kimi-K3（G31+ 波 C Task C8 支持渠道与版本政策文档化） -->
# Rurix 渲染器支持政策（issue / 版本 / 安全响应 / 兼容承诺）

> 所属：G31+ 波 C Task C8（支持渠道与版本政策，G31_PLUS_COMMERCIAL_RENDERER_TODO §5 #55）。
> 读者：采纳/集成 Rurix 渲染器 SDK 的外部项目维护者与用户。
> 纪律：**引用不复制**——治理骨架 [10_GOVERNANCE.md](../../10_GOVERNANCE.md)、SDK 版本政策事实源
> [apps/g31-renderer-sdk/API_VERSIONING.md](../../apps/g31-renderer-sdk/API_VERSIONING.md)、语言面安全政策
> [SECURITY.md](../../SECURITY.md) 各为独立事实源，本文只引要点不镜像全文；政策条目中可机器核验的部分
> 全部由 `ci/g31_support_policy_smoke.py`（门 `g31.waveC.support`）核验，防文档腐化。
> 姊妹篇：[integration_guide.md](integration_guide.md) · [feature_matrix.md](feature_matrix.md) ·
> [performance_tuning.md](performance_tuning.md) · [compatibility_matrix.md](compatibility_matrix.md) ·
> [release_checklist.md](release_checklist.md)（发布操作单）。

---

## 1. 缺陷报告流程

### 1.1 报告要素（缺陷模板）

渲染器面缺陷报告请尽量携带以下要素——全部是仓内既有机器面的产物，不是新流程：

| 要素 | 产出方式（既有面） | 何时必附 |
|---|---|---|
| **capability report** | `vk_capability_report`（C3 统一运行时能力探测面，schema `rurix.g31.capability_report.v1`；探测内容见 [compatibility_matrix.md](compatibility_matrix.md) §1） | 兼容/降级类必附 |
| **digest** | canonical bench receipt 的 `last_frame_digest`：`--bench --scene bistro-interior --tier 100 --backend tsr_device --frames 160 --warmup 10`（与 Stage A 锚位级比对，口径见 [integration_guide.md](integration_guide.md) §6~§7） | 渲染正确性类必附 |
| **帧时** | 同一 receipt 的 `stats_post_warmup` 逐帧 `frame_ms`；双口径纪律（real_render 与 presented 独立登记，勿混——[performance_tuning.md](performance_tuning.md) §1） | 性能类必附 |
| **复现步骤** | 命令行逐字 + scene/tier/backend/flags + GPU 型号/驱动版本/OS；环境纪律 `RURIX_REQUIRE_REAL=1 RURIX_VK_VALIDATION=1`（validation 全程静默是绿件前提） | 全部四类必附 |
| **profiler 数据** | 自助测量清单（[performance_tuning.md](performance_tuning.md) §6）：内部 GPU 时间戳 receipt + DLSS 臂分解探针（`RURIX_VENDOR_TIMING=1` / `RURIX_G31_NGX_TS=1`，默认关、零行为变更） | 性能类建议附 |

> pass 级 profiler 对外暴露面（Nsight 标注 / RenderDoc 兼容）= **待建立**，见 §5——当前不可要求报告者提供该面数据。

### 1.2 分类（四面闭集）

| 类 | 判据锚 | 首附要素 |
|---|---|---|
| **渲染正确性** | digest 漂移 / 错图 / 确定性协议破坏（固定 seed digest 锚，integration_guide.md §7） | digest 双跑对拍 + 复现步骤 |
| **性能** | 帧时回归 / 帧时预算违约（在案基线 = performance_tuning.md §2） | 帧时 receipt + 同窗 A/B 对照 |
| **崩溃** | device lost / 驱动 TDR / 进程退出 / validation ERROR 非零（C4 健壮性面：`ci/g31_robustness_smoke.py` 故障注入三点在案） | validation 输出 + 复现步骤 |
| **兼容** | 能力探测误判 / 降级链裁决异常 / 厂商格问题（六链 fail-closed 闭集 = compatibility_matrix.md §2） | capability report |

### 1.3 响应口径（诚实面）

- **分级序**：安全 > 崩溃 > 渲染正确性 > 性能 > 兼容。安全类**不走公开 issue**，走 §3 专用私下渠道。
- **节奏如实标注**：本项目 = 单维护者 + AI 集群（[10_GOVERNANCE.md](../../10_GOVERNANCE.md) §2 角色帽面），
  **无商业 SLA，不捏造响应时限数字**。缺陷经机器可核验复现后按里程碑波次处置，登记面 =
  `registry/deferred.json`（open/maintain 纪律）与各期契约；处置结论只追加、不回写。
- 安全类确认时限 = **3 个工作日**（镜像 [SECURITY.md](../../SECURITY.md) 既有承诺，见 §3.3）。
- 报告质量口径：缺 §1.1 必附要素的报告可能被要求补件后重评——机器可核验复现是处置前提（10 §7 政策 4「数字必须来自命令输出」同律）。

## 2. 版本政策

### 2.1 SDK 语义化版本（事实源 = API_VERSIONING.md）

渲染器 SDK（`rurix_renderer.dll` C ABI 导出集）版本政策全文 =
[apps/g31-renderer-sdk/API_VERSIONING.md](../../apps/g31-renderer-sdk/API_VERSIONING.md)。要点引用（不复制）：

- **v1 = 1.0.0**（打包值 `0x00010000`；`rurix_renderer_abi_version()` 返回 `MAJOR<<16 | MINOR<<8 | PATCH`）。
- 单一事实源 = `ABI_VERSION_PACKED`（`src/rurix-renderer-sdk/src/lib.rs`）；stable 快照
  `tests/stable/stable_api.snapshot` 的 `renderer_sdk_api` 段程序读同一字面（`abi_version` = 1.0.0，
  导出集 9 函数规范化签名）。
- 三档：MAJOR = 破坏性变更（先 RFC + 新旧 MAJOR DLL 并存分发 + 旧 MAJOR 安全修复至次 MAJOR）；
  MINOR = 同 MAJOR 内只增不破坏；PATCH = 语义不变修复（导出集/生成头 0-byte）。
- 宿主兼容裁决：`(abi_version() >> 16)` 与自身构建期 MAJOR 不等 → **不得继续调用**。

### 2.2 release 节奏（里程碑期联动）

- **渲染器面 = 里程碑期驱动**：SDK 版本随 G31+ 商业化波次的机器绿门齐备而发布；发布前逐项核验清单 =
  [release_checklist.md](release_checklist.md)（全部条目为真实 ci 脚本，机器可核）。
- 语言面工具链的 6 周 train（[10_GOVERNANCE.md](../../10_GOVERNANCE.md) §6）是**语言面**政策，
  不冒充渲染器面节奏；渲染器面无独立周期承诺（在案事实）。
- stable 通道事实源 = `channels/stable.json`（现登记语言工具链 `v1.0.1-dist` 系列；渲染器 SDK bundle
  进通道 = 待建立，见 §5 C5）。

### 2.3 LTS / 修复线政策

| 版本线 | 修复支持 |
|---|---|
| 最新 `1.x`（当前 MAJOR 线） | 全部修复（含安全）——镜像 [SECURITY.md](../../SECURITY.md)「仅最新发布线接受安全修复」口径的渲染器面延伸 |
| 旧 MAJOR 线 | **仅安全修复，至次 MAJOR 发布为止**（API_VERSIONING.md §2 MAJOR 行字面） |
| 预 1.0 面 | 不存在——v1 即 1.0.0，首版即按 stable 纪律治理（API_VERSIONING.md §2 预 1.0 纪律：以 bistro 生产管线 canonical 序列末帧 digest 与 Stage A 锚位级对拍为语义锚） |

> 多版本线长期并行维护能力 = **未建立**（单维护者面，如实登记，见 §5）——LTS 承诺上限即上表两行。

## 3. 安全响应

### 3.1 报告渠道（镜像 SECURITY.md）

**请勿在公开 issue 中报告渲染器面安全漏洞。** 私下渠道（与语言面同一入口）：

- 邮件：**25890346@qq.com**（安全联系）
- GitHub 私有漏洞报告：仓库 **Security → Report a vulnerability**

渲染器面报告请尽量包含：受影响面（SDK ABI / 驱动交互 / shader 供应链 / vendor SDK / 分发签名链）
与版本/commit；复现步骤或 PoC；影响评估；兼容面问题附 capability report；驱动面问题附
`RURIX_VK_VALIDATION=1` 全程输出。

### 3.2 渲染器特有面（驱动交互 / shader 供应链 / vendor SDK）

| 面 | 既有机器面 | 尤其欢迎报告 |
|---|---|---|
| **驱动交互** | Vulkan 能力链协商 fail-closed（ray query 四扩展 + sync2，SDK `create`/`load_scene` 口径）；device lost / TDR / budget 违约面（C4 `ci/g31_robustness_smoke.py` 故障注入在案） | validation 绕过；能力协商误判导致越界执行；TDR/device-lost 恢复路径的状态机缺陷 |
| **shader 供应链** | canonical SPV 四件套（场景提交面）+ 生产分发 fatbin 守卫（`ci/fatbin_dist_smoke.py`，RXS-0150~0152） | SPV / 胖二进制篡改；确定性 digest 锚绕过；构建链注入 |
| **vendor SDK** | DLSS Streamline 2.10.3 四 DLL / FSR FidelityFX SDK 2.0.0 双 DLL 动态加载（NGX 动态加载 fail-closed）；BasisU（`src/rurix-basis-sys`）；Jolt 物理桥；再分发白名单守卫（`ci/check_redistribution.py`） | vendor DLL 装载链注入/劫持；再分发面越界（白名单外捆绑）；vendor 组件已知 CVE 影响评估 |

### 3.3 处理时间线与披露

镜像 [SECURITY.md](../../SECURITY.md) 既有承诺（渲染器面同律）：

- **确认**：3 个工作日内确认收到。
- **评估与修复**：依严重度排期；高危优先。
- **披露**：修复发布后协调公开（coordinated disclosure）；报告者可署名致谢（可选）。

## 4. 兼容承诺

### 4.1 stable ABI 守卫（stable 快照 renderer_sdk_api 段）

- 守卫面 = `ci/stable_snapshot.py` 的 `renderer_sdk_api` 段：`apps/g31-renderer-sdk/src/sdk.rx` 的
  9 个 `#[export(c)]` 导出规范化签名（名字 + 参数类型序 + 返回类型）+ `abi_version` 程序读
  `ABI_VERSION_PACKED`——快照件 = `tests/stable/stable_api.snapshot`。
- **任何导出集/版本变更都表现为快照漂移**，必须经 `RURIX_BLESS=1` 重 bless +
  `tests/stable/bless_log.md` 追加留痕（RD-008 closed 机制的渲染器面延伸）——破坏性变更因此
  **不可能静默发生**。
- v1 兼容承诺面（API_VERSIONING.md §3）：状态码闭集 0/2/3/4/5/6/7；资源句柄对宿主恒不透明（u64）；
  跨堆所有权不越界（宿主缓冲一律调用方分配，RXS-0255 口径）。

### 4.2 破坏性变更走 RFC 纪律

- 变更三档门 = [10_GOVERNANCE.md](../../10_GOVERNANCE.md) §3（Direct PR / Mini-RFC / Full RFC）；
  FFI ABI 面 = 高敏面（`agents/AGENTS.md` 硬规则 5）——**MAJOR 演进一律 Full RFC 留档**，
  且新旧 MAJOR DLL 并存分发（API_VERSIONING.md §2）。
- MINOR 加性 = stable 快照 bless 程序留痕（`RURIX_BLESS=1` + bless_log 追加）；PATCH = 快照 0-byte。
- 语义锚不移动：bistro 生产管线 canonical 序列末帧 digest 与 Stage A 锚 `bistro-interior_t100_tsr_device`
  位级对拍（SDK 面 ≡ 生产管线，API_VERSIONING.md §2）——任何版本演进不得使该锚静默漂移。

## 5. 待建立项（诚实登记，不冒充）

| 项 | 现状（在案事实） | 锚 |
|---|---|---|
| **C5 渲染器 SDK 分发打包** | **在飞未落地**：SDK bundle 进 rurixup/MSI/winget 链 + 签名/SBOM 扩展未完成；现分发件 = 语言工具链面（`channels/stable.json` 的 `v1.0.1-dist` 系列，EA1 链） | G31_PLUS §5 #52 |
| **C6 vendor 许可合规终审** | **在飞未落地**：全 vendor 面商用再分发许可矩阵未完成；超分面已有 `milestones/g13/design/vendor_upscale_license_clearance.md` | G31_PLUS §5 #53 |
| **C7 性能剖析与调试工具面** | **待建立**：pass 级 profiler 对外暴露 / Nsight 标注 / RenderDoc 帧捕获兼容未落地；现有 = 内部 GPU 时间戳 + [performance_tuning.md](performance_tuning.md) §6 自助测量清单 | G31_PLUS §5 #54 |
| **AMD/Intel 真卡兼容格** | `g31_compatibility_matrix.json` 两格 `dev_env_degrade` 如实登记（缺硬件）；≥2 厂商真卡全链绿未达成 | 锚 G-MB1-6（[compatibility_matrix.md](compatibility_matrix.md) §3） |
| **商业支持 SLA** | 未建立（单维护者 + AI 集群面）；响应口径见 §1.3 | [10_GOVERNANCE.md](../../10_GOVERNANCE.md) §2 |

---

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-25 | 初版（G31+ 波 C Task C8）：缺陷报告流程（五要素/四面分类/诚实响应口径）+ 版本政策（引用 API_VERSIONING.md + 里程碑期联动 + LTS 修复线）+ 安全响应（镜像 SECURITY.md + 驱动交互/shader 供应链/vendor SDK 三特有面）+ 兼容承诺（stable 快照 renderer_sdk_api 段 + RFC 纪律）+ 待建立项五件诚实登记 |
