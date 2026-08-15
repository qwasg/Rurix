<!-- Assisted-by: Kimi-K3（G10.1 治理波 RFC 起草）；Assisted-by: Kimi-K3（D-409 修法批） -->
# RFC-0027 — G10 外部参照 harness 与压测资产许可边界

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0027（4 位制，编号永不复用，10 §9.5）。按 G10 立项时实测 `registry/number_ledger.json` namespaces.RFC `next_free=26`、并行在途 RFC-0026 之后顺位登记；**本 Draft 不改 ledger，主会话统一核对** |
| 标题 | G10 外部参照 harness 与压测资产许可边界（UE5 出图编排边界 / 压测资产许可白名单 / 资产外部缓存与场景清单冻结） |
| 档位 | **Full RFC**（10 §3：供应链与再分发许可边界、外部进程编排契约、资产外部缓存语义——沿 D-313 审计模式与 RFC-0020 §4.13 vendor 许可面的加性扩展；AGENTS 硬规则 5/8 向上取严） |
| 状态 | **Agent Approved（2026-08-15）**——D-409 对抗性评审已完成：独立隔离评审会话 18 findings（high F1~F3 / med F4~F11 / low F12~F18）全部 disposition 落实（v0.2 修法批），§9.1 评审记录段已回填；F1 同环境单一模型 provenance 偏差不静默处理、按 RFC-0015 §9.1/v1.29/v1.73/v1.90 先例如实登记于 §9.1，补救承诺与 G10.8b 终审复核锚同录。主会话核对契约三面一致后翻 Agent Approved。**批准不解锁任何实现**（G-G10-3 互锁为唯一实现入口） |
| 承接里程碑 | G10.1 起草/评审；实现互锁开放后由 G10.2 承接 M128/M129、G10.3 承接 M131/M132/M133（验收门 G-G10-4 / G-G10-5） |
| 关联条款 | 拟新建 `spec/external_reference.md`（理由见 §5）；**本 Draft 不 claim RXS/CI/RD/U/RX 等编号**，条款号与数字 CI 步骤一律 post-interlock actual-next-free allocation（沿 RFC-0020 §5/F15 先例：不保证连续、不预留区间） |
| 依据决策 | [G10_CONTRACT](../milestones/g10/G10_CONTRACT.md) v1.0（立项裁决 2/3/9 · §4.2 M128/M129/M131/M132/M133 · guardrails「UE 源码仅外部参照」「压测资产白名单」「二进制不入 git」）· [G10_PLAN](../milestones/g10/G10_PLAN.md) v1.0 §2 G10.2/G10.3、§4 R-G10-1/2/4/9/10/11 · [g10_ue5_harness_spike](../milestones/g10/design/g10_ue5_harness_spike.md) v1.0 · D-409 · 14 §5（环境画像证据纪律）· P-09（禁手写 digest/阈值） |
| Provenance | `Assisted-by: Kimi-K3（G10.1 治理波 RFC 起草）` |
| Agent 批准 | 留 Draft——§9.1 独立 provenance 对抗性评审完成、findings 全 disposition 后由主会话翻 Agent Approved 并记录 |
| 对抗性评审 | D-409 第 1 轮已执行（2026-08-15，18 findings：high F1~F3 / med F4~F11 / low F12~F18，逐条 disposition 见 §9.1，评审记录 `milestones/g10/design/rfc0027_adversarial_review.md`）；**provenance 偏差如实登记：评审会话模型同为 Kimi-K3（零共享上下文独立隔离会话），不满足 D-409「评审 provenance ≠ 起草 provenance」字面——偏差登记与补救承诺见 §9.1，处置经主会话裁决前维持 Draft** |

---

## 1. 摘要

本 RFC 冻结 G10 的三条外部边界，使「用本机 UE 5.8 出参考帧、用联网获取的压测资产做 A/B 对标」成为可机器核验、许可自洽、仓库零污染的闭环：

1. **UE 出图 harness 编排边界**——UE 5.8 恒为**外部进程**：零 vendoring、零源码/二进制片段复制进 rurix 仓库；出图命令面为开关白名单 + 参数 schema 闭集；每帧参考帧带 provenance 七元组闭集登记；Epic 账号登录为唯一人工接管点，未完成记 `DEV_ENV_DEGRADE` 不充绿。
2. **压测资产许可白名单**——精确 SPDX 名单 `{CC0-1.0, CC-BY-3.0, CC-BY-4.0}`；逐资产按类登记闭集（external 类五元组 = asset_id + SPDX id + 来源 URL + attribution + 资产清单级 digest；generated 类替代登记型 = generator_script_digest + 生成参数 digest + 本地产物 digest，§4.2）；白名单外许可注入即 fail-closed。**联网核验结论：CornellBox（程序生成）与 Bistro（CC-BY-4.0）通过；Crytek Sponza 为 Cryengine Limited License Agreement 自定义引擎许可、不在 CC 族，fail-closed 拒入首发清单**（§4.2 裁决）。
3. **资产外部缓存与清单冻结**——压测资产二进制零入 git；K: 盘外部缓存 + 仓库内元数据 JSON 登记 + digest 核验；场景清单版本化冻结、清单 digest 注册（M133）、后续变更只追加修订行。

```text
Rurix 仓库（git，零 UE 源性 / 零资产二进制）
├─ milestones/g10/  元数据登记：许可按类登记（五元组/生成六字段） · 场景清单+digest · provenance 七元组
├─ spec/external_reference.md（post-interlock 条款，spec-first）
└─ ci/g10_*.py ──编排──► ══ 外部进程边界（零 vendoring / 零片段复制 / 零凭据） ══
                            │
                            ▼
              UE 5.8 正式版（Launcher 安装；Epic 账号人工接管点一次）
                            │ MRQ 主臂 / HighResShot 快速臂 / Python 编排臂
                            ▼
              HDR 参考帧（K: 盘帧库；逐帧 provenance 登记；双跑 digest 门）
                            ▲
   压测资产外部缓存 K:\...\g10-corpus\（白名单 CC0/CC-BY 族；加载前 digest 核验）
```

## 2. 动机

### 2.1 现状缺口

- 仓库压测资产现状为零：仅 fixture 生成器 `ci/_gen_m81_gltf_fixtures.py`（G10_PLAN §2 G10.3 实测登记），无任何真实压测场景资产；许可登记面不存在。
- UE 出图环境 spike（`design/g10_ue5_harness_spike.md` v1.0）已完成只读探测并给出裁决建议（Launcher 首选 / 源码编译备选 / 公开参考图兜底），立项裁决 2 已采纳；但**编排边界**（什么算合法调用、什么算污染、凭据如何隔离、人工接管点如何登记）尚无冻结文本。
- G10_CONTRACT guardrails 已冻结三条纪律字面（UE 源码仅外部参照 / 压测资产白名单+SPDX+URL+attribution+digest 登记 / 二进制不入 git 走 K: 盘外部缓存+元数据登记），但**可机验的闭集 schema、RED 臂与许可事实表**未定义——M131 门「白名单外许可注入即 RED」当前没有可求值的白名单本体。
- 联网核查（2026-08-15，§4.2/§4.5）发现首发清单三资产中 **Sponza 的许可事实与「CC0/CC-BY 族」白名单冲突**（Crytek 自定义引擎许可），必须在清单冻结前裁决处置，否则 M131 门无法诚实求值。

### 2.2 为何需要 Full RFC（而非 Direct/Mini）

本 RFC 触及：①供应链与再分发许可边界（第三方资产白名单、SPDX/attribution/digest 登记面——D-313 审计模式的加性扩展，沿 RFC-0020 §4.13 范式）；②外部专有软件（UE）的编排契约与 EULA 事实登记面；③资产外部缓存语义（git 零二进制的内容寻址外部缓存，与 RFC-0020 DDC 不同域但同级纪律）。三者均为跨里程碑长期互操作面且误判代价外溢到 G15 商用收口（R-G10-4）。依 10 §3、P-11 与 AGENTS 硬规则 5/8，按 Full RFC 留档，并在实现前走 spec-first、RED-first 与独立对抗性评审。

### 2.3 in-scope

| 面 | 本 RFC 冻结面 | G10 行 |
|---|---|---|
| UE 编排边界 | 外部进程纪律、命令面闭集、provenance 七元组、Epic 人工接管点协议、回退臂程序 | M128/M129 |
| 许可白名单 | 精确 SPDX 名单、external/generated 按类登记闭集、fail-closed RED 臂、逐资产许可事实表、Sponza/BMW 裁决 | M131 |
| 外部缓存 | K: 盘缓存布局纪律、仓库内元数据 JSON 登记面、digest 核验、git 零二进制拦截面 | M131/M132 |
| 清单冻结 | 场景清单 schema、清单 digest 注册、只追加修订程序 | M133 |
| EULA 事实登记 | UE EULA / CE-terms 条款事实与风险登记（**不构法律意见**） | M128 证据面 |

### 2.4 out-of-scope 与事实互锁

- 不改 UE 引擎、不消费 UE 源码（E:\Kimi_Agent_Taichi Engine 优化计划\references\UnrealEngine 只读参照面 0-byte）；不 vendoring 任何 UE 二进制/源码/着色器片段。
- 不做画质修复（G11）、不设画质/帧率通过线（立项裁决 5）；不产出 A/B 度量语义（帧捕获 HDR 格式、FLIP/SSIM/PSNR 口径——属并行 RFC-0026）。
- 不提供法律意见；EULA/CE-terms 仅为事实与风险登记（§4.5）。
- **G10.1 仅 governance-only**：即便本 RFC 翻 Agent Approved，`src/`/`spec/`/`conformance/` 与数字 CI 步骤仍由 G-G10-3 互锁硬阻断；互锁红时禁止 materialize 本 RFC 的 spec 条款、实现与编号。

## 3. 指导级解释（使用者视角）

**出图编排**：实现波的建设者不需要「打开 UE 编辑器手动截图」。harness 以结构化参数拼出白名单内的命令形态，例如 MRQ 主臂（spike 问题 3 官方文档实证形态）：

```text
UnrealEditor-Cmd.exe <proj>.uproject <map> -game
  -MoviePipelineConfig="/Game/Cinematics/<Queue>"
  -windowed -resx=<W> -resy=<H> -log -notexturestreaming -Unattended
```

每产出一帧，evidence 里必须同时落一行 provenance：场景 id、相机参数 digest、光照参数 digest、UE build id、GPU 驱动版本、锁频状态、出图臂（capture_arm）——七元组缺任一行即 RED。同一参数集双跑，帧 digest 必须一致（M129 硬判据）。若 Launcher 要求 Epic 登录，agent 停下来把控制权交还用户（**人工接管点**），用户完成登录后 agent 继续；这一步未完成只记 `DEV_ENV_DEGRADE`，绝不充绿，凭据永不进命令行/日志/CI。

**许可登记**：每个进清单的资产在仓库内只有一份元数据 JSON 行（按类登记：external 五元组 / generated 替代登记型），二进制本体躺在 K: 盘外部缓存。external 类示意形态（文本语法由下游 spec 冻结，本例不构成 stable 承诺）：

```json
{
  "asset_id": "bistro-orca",
  "spdx_id": "CC-BY-4.0",
  "source_url": "https://developer.nvidia.com/orca/amazon-lumberyard-bistro",
  "attribution": "Amazon Lumberyard Bistro, Open Research Content Archive (ORCA), © 2017 Amazon Lumberyard, CC BY 4.0",
  "digest": "sha256:<清单级 canonical digest，获取时实测登记>",
  "cache_rel": "bistro-orca/<versioned_subdir>/",
  "byte_len": 0
}
```

generated 类（程序生成资产）示意形态——以 `generator_script_digest` + 生成参数 digest + 本地产物 digest 替代 `source_url`，`NONE` 为字面闭集值而非缺字段：

```json
{
  "asset_id": "cornell-box-generated",
  "spdx_id": "NONE",
  "source_url": "NONE",
  "generator_script_digest": "sha256:<生成器脚本实测登记>",
  "generator_params_digest": "sha256:<生成参数/seed canonical digest>",
  "digest": "sha256:<本地产物清单级 canonical digest，实测登记>",
  "cache_rel": "cornell-box-generated/<versioned_subdir>/",
  "byte_len": 0
}
```

加载门前对缓存逐文件复算 SHA-256 并复算清单级 canonical digest，与登记不符即 fail-closed；任何 external 类资产行的 SPDX id 不在 `{CC0-1.0, CC-BY-3.0, CC-BY-4.0}` 闭集内即 RED（generated 类恒为 `NONE`）；git 树内出现守卫闭集命中的资产二进制即 RED。

**清单冻结**：场景清单冻结后只追加——任何变更（加场景、改相机引用）都以新修订行 + 新清单 digest 落盘，原地改即 RED。

## 4. 参考级设计

### 4.1 🔒 UE 出图 harness 编排边界（M128/M129）

**外部进程纪律（冻结）**：

1. UE 5.8 恒为外部进程。rurix 仓库（`src/`、`spec/`、`conformance/`、`.github/`、`ci/`、文档）内零 UE 二进制、零 UE 源码/着色器片段复制、零 vendoring 目录；违反即 revert + 留痕（G10_CONTRACT guardrail 逐字承接）。
2. UE 安装与源码参照面：`E:\Kimi_Agent_Taichi Engine 优化计划\references\UnrealEngine` 只读外部参照 0-byte；Launcher 安装落盘位置由实现波按 spike 磁盘事实选择（E:/F:/K: 均可，~40–60 GB）；帧库与大体积产物落 K: 盘（H: 盘仅余 6.9 GB，R-G10-11）。
3. **出图命令面闭集**：仅允许 spike 实证的三臂命令形态——
   - **臂 A（主路，MRQ 批量臂）**：`-game -MoviePipelineConfig=<queue> -windowed -resx/-resy -log -notexturestreaming -Unattended`（官方 MRQ 命令行文档实证）；
   - **臂 B（快速截屏臂）**：`-game -benchmark -fps=<N> -seconds=<N> -ResX/-ResY -execcmds="…HighResShot <W>x<H>" -unattended -log -FixedSeed`（`LaunchEngineLoop.cpp` 源码实证开关集；`-execcmds` 触发时序与 `-renderoffscreen` 在 5.8 的可用性为 spike 标注的实现波待验证项，首日实测登记）；
   - **臂 C（Python 编排臂）**：`-ExecutePythonScript=<script>.py` + `MoviePipelineQueueSubsystem` 回调退出（实现波验证）。
   命令行由结构化参数生成（禁 shell 字符串拼接注入）；schema 外开关/参数注入即 fail-closed；**臂 B 的 `-execcmds` 内嵌控制台命令同闭集化——仅允许白名单控制台命令 + 参数模板（当前闭集 = `HighResShot <W>x<H>` 模板 + `r.ResetViewState`；扩展走只追加修订行），模板外自由文本注入即 fail-closed（F17 修法：注入面不得从开关层转移到 execcmds 内容层）**；选臂依据实测登记（spike 待验证清单），两臂诚实登记禁伪绿。
4. **provenance 七元组闭集（逐帧登记，缺行即 RED）**：`scene_id` / `camera_params_digest` / `lighting_params_digest` / `ue_build_id` / `gpu_driver_version` / `clock_lock_state` / `capture_arm`。其中 `ue_build_id` = Launcher 版本号 + CL 字符串文本（非安装目录哈希——数十 GB 安装面不可取 digest，F12 修法）；`capture_arm` = 出图臂 id（A/B/C）+ 命令面/queue 配置 digest，使同 scene 不同臂出帧可从 provenance 区分，§4.1.3 的选臂诚实登记落到签名面（F12 修法）。时间戳、主机绝对路径、用户名字段不得进入签名面；相机/光照参数本体按 M130 双端确定性契约（RFC-0026 面）同 schema 双端各一份，此处只登记其 digest。
5. **Epic 账号人工接管点协议**：Launcher 首次 Epic 登录 = 唯一人工接管点（一次性用户交互）。agent 遇登录门即暂停并交还控制权；凭据（口令/token/session）永不进命令行参数、环境变量、日志、CI、仓库（R-G10-2）。接管点未完成 = `DEV_ENV_DEGRADE`（证据留痕，不充 P0 绿，G10_CONTRACT §4.1 逐字承接）；缺硬件/工具链同口径。**CI 可执行性分层（F7 修法）**：①Launcher 安装的 UnrealEditor-Cmd 出图运行时是否需登录态，列入 G10.2 首日实测登记事实项（通行事实为不需要，本 RFC 不预断）——该事实决定接管点是「安装时一次」还是「每次运行前」；②接管点重做触发条件闭集 = 新机器首跑 / UE 补丁更新后首跑 / session 失效，三者之外不得要求人工交互；③UE 出图门（M128/M129）仅由已完成接管点的预登录本机承载，无登录态环境一律 `DEV_ENV_DEGRADE` 显式登记、不充 G-G10-4 绿，非 UE 面门与 UE 面门分层求值——无登录态 CI runner 不构死锁也不构伪绿。
6. **回退臂程序**：首选② Launcher 臂登录受阻且人工介入不可得 → 回退①源码编译臂（K: 盘承载；qwasg GitHub 凭据已核查在 EpicGames 组织且 token 含 repo scope，spike 问题 5/风险 2）→ 契约 §8 只追加修订本波判据；ue5-main 快照 vs 5.8-release 口径差登记（Launcher 版即官方 5.8 release，口径优先，立项裁决 2）。**禁以截图/人工采集帧冒充 harness 出帧**（G-G10-4 RED 臂）；③公开参考图仅兜底对照材料，不进验收证据链（F16 编号修正：②即 Launcher 首选本身）。

### 4.2 🔒 压测资产许可白名单（M131）

**白名单（精确 SPDX 名单，立项裁决 3「CC0/CC-BY 族」的精确化，冻结）**：

```text
{ CC0-1.0, CC-BY-3.0, CC-BY-4.0 }
```

- 仅上述三个 SPDX id 合法；**NC/ND/SA 后缀变体（CC-BY-NC-\*、CC-BY-ND-\*、CC-BY-SA-\*、CC-BY-NC-SA-\*、CC-BY-NC-ND-\*）、自定义/专有许可、无许可文本的声明性文字一律不在族内**。CC-BY 其他版本（如 2.0）如需进入，走 §4.4 同构的白名单只追加修订程序（本 RFC 不预放行）。
- **统一纪律字面（F3 修法）**：凡「无许可法律文本的声明性文字」（无 creativecommons.org legalcode 或等效正式文本锚点的许可标注），一律 fail-closed 入待定池，无例外——档案维护者声明、原作者声明、第三方转引同此标准；任何资产不得凭声明性文字直接判 PASS。
- **登记闭集分两类（逐资产，按类缺字段即 RED——F2 修法）**：
  - **external 类（外部获取资产）五元组**：`asset_id` + `spdx_id` + `source_url` + `attribution` + `digest`（SHA-256，获取时实测登记，禁手写——P-09）。**多文件资产（如 Bistro 的 FBX + Falcor scene + 纹理集）的 `digest` 为清单级 canonical digest**：逐文件 `相对路径 + sha256` 按路径稳定排序的清单再 sha256（canonical 规则沿 RFC-0020 §4.2 同构子集），并附 `file_count` 与总 `byte_len`；只下载包内部分文件充数即 RED（F5 修法，堵伪绿口）。
  - **generated 类（程序生成资产）替代登记型六字段**：`asset_id` + `spdx_id=NONE` + `source_url=NONE` + `generator_script_digest`（生成器脚本 sha256）+ `generator_params_digest`（生成参数/seed canonical digest）+ `digest`（本地产物清单级 canonical digest，规则同 external 类）。`NONE` 为字面闭集值而非缺字段。本分类是立项裁决 3「Cornell Box 程序生成、零许可风险」与 guardrail「SPDX + 来源 URL + attribution + digest 登记」对程序生成资产的可机验精确化；契约 M131 判据字面「来源 URL」与 guardrail 字面的同类精确化由主会话经契约 §8 只追加区登记（本 RFC 不改契约字面）。
  - **M131 门按类求值**：external 类五元组缺字段即 RED；generated 类六字段缺字段即 RED；两类不得互相冒充（generated 类谎报外部来源、external 类借 generated 类规避 source_url/digest，均即 RED）。
  - **两类通用登记字段（F6 修法）**：`license_snapshot`（许可文本快照文件名，快照入 git 治理面——纯文本非二进制资产，不违反零二进制纪律；内容为官方 legalcode 文本 + 一手页面 License 行摘录；generated 类登记生成器内嵌许可头或 `NONE`）、`checked_at`（许可核查日期）、`upstream_ref`（上游版本/Updated 日期/commit——McGuire 档案 Updated 行、ORCA 版本面；无版本面者登记 `NONE`）。**source_url 失效程序**：复核发现 URL 失效即登记漂移事件，以 `license_snapshot` + archive.org 镜像兜底复核；许可事实未变者仅追加 `checked_at` 修订行，许可事实变动者按 fail-closed 重判。
  - **attribution 结构化子字段闭集（F4 修法，替代自由文本、使机器可核）**：`creator` / `title` / `source_uri` / `license_uri` / `copyright_notice` / `modified_flag`（布尔；产出物含修改时须 `true` 并附修改说明行）。版本差异登记：CC-BY-4.0 §3(a) 法定要素 = 创作者标示 + 版权声明 + 许可声明 + 免责声明 + 来源 URI + **修改标示**；CC-BY-3.0 §4(a) 要素 = 保留版权声明 + 合理方式署名 + 标题 + URI——闭集取两版并集（TASL + copyright_notice + disclaimer 行 + modified_flag），按资产 `spdx_id` 对应版本求值，子字段缺失即 RED。CC0 无法定 attribution 义务仍登记出处行（`creator`/`source_uri`）。
  - **复合许可表达（F4 修法）**：`spdx_id` 允许 SPDX 表达式受限子集——白名单 id 的 `AND` 组合 + `LicenseRef-<name>`（LicenseRef 行必须附 `license_snapshot`）；如 Breakfast Room 主体 CC-BY-3.0 + public domain 大理石纹理登记为 `CC-BY-3.0 AND LicenseRef-PublicDomain-MarbleTexture`。表达式中出现白名单外 id（NC/ND/SA 族、自定义许可）即 RED。
- **fail-closed RED 臂（M131 判据逐项承接）**：未登记资产混入清单即 RED；白名单外许可注入即 RED；按类登记字段缺失即 RED；登记 digest 与缓存实算不符即 RED。
- 许可事实核查一律给出来源 URL 与核查日期；二次来源（第三方 notices 转引）须标注「二次核验」，与一手页面分列。

**逐资产许可事实表（联网核查日 2026-08-15）**：

| 资产 | 地位 | 许可事实（一手来源） | SPDX 判定 | attribution 要求 | 白名单判定 |
|---|---|---|---|---|---|
| Cornell Box（程序生成） | **首发** | 仓库内程序生成，零第三方资产摄入，零外部许可面；**程序生成输入面声明（F14 修法）：生成器为纯自写几何/反射率公式、不读取/转换任何外部数据文件，`NONE` 判定以此声明为成立前提**；几何/反射率数值参考 Cornell PCG「Public Use Data」页（https://www.graphics.cornell.edu/online/box/data.html ，页面无显式许可文本——按统一纪律仅作数值来源参考登记，不作资产摄入）；**替代源改写：若改用外部数据源（如 McGuire 档案 CornellBox.zip，CC-BY-3.0 © 2009 Morgan McGuire，https://casual-effects.com/data/ ），须按该数据 SPDX 转 external 类重新登记并走只追加程序** | NONE（程序生成，generated 类登记） | 无法定要求；登记出处参考行 | **PASS**（零许可风险，立项裁决 3 口径） |
| Amazon Lumberyard Bistro（ORCA） | **首发** | ORCA 页面 License 行：「Creative Commons CC-BY 4.0」（https://developer.nvidia.com/orca/amazon-lumberyard-bistro ；ORCA 总页 https://developer.nvidia.com/orca ）；页面附「How to cite」BibTeX（© 2017 Amazon Lumberyard）；三角形计数登记：Interior 1,046,609 / Interior+wine 1,293,691 / Exterior 2,832,120；格式 FBX + Falcor scene | **CC-BY-4.0** | 必须：按 ORCA 引用文本 + © Amazon Lumberyard + CC BY 4.0 链接 | **PASS** |
| Crytek Sponza（Khronos glTF-Sample-Assets） | 首发清单**核验不通过** | README Legal 行：「© 2016, Crytek. Cryengine Limited License Agreement」（https://github.com/KhronosGroup/glTF-Sample-Assets/blob/main/Models/Sponza/README.md ；条款 https://www.cryengine.com/ce-terms ）：CE-terms 1.2 定义 CRYENGINE Assets 为随引擎分发的视听文件，2.1.3/2.1.4 仅授权随 CRYENGINE 开发的 Game 使用/发布，2.1.2 禁与其他游戏引擎代码混合，2.4 禁再分发；2010 年 Crytek 下载页声明性文字「donated to the public … for use with various commercial 3D applications and renderers」（经 SponzaPBR copyright.txt 转引，同 README）**无 SPDX、非正式许可文本** | NONE（自定义专有许可，LicenseRef-Cryengine-LLA） | 不适用（不入清单） | **FAIL**（白名单外，fail-closed） |
| San Miguel 2.0 | 追加候选 | CC BY 3.0 © Guillermo M. Leal Llaguno，McGuire 档案逐模型 License 行（https://casual-effects.com/data/ ；二次核验：msu-graphics-group/scenes README 转换自该档案同许可行 https://github.com/msu-graphics-group/scenes ） | **CC-BY-3.0** | 必须：© Guillermo M. Leal Llaguno + 档案出处 + CC BY 3.0 链接 | 候选 **PASS**（待 G10.3 获取 + digest 实测登记后经只追加程序入清单） |
| Breakfast Room | 追加候选 | CC BY 3.0 © Wig42（ blendswap 原作；McGuire 档案 License 行，https://casual-effects.com/data/ ；大理石纹理为 public domain，同页登记） | **CC-BY-3.0** | 必须：© Wig42 + 档案出处 + CC BY 3.0 链接 | 候选 **PASS**（同上行程序） |
| BMW | 待定池（F3 重判，2026-08-15 复核） | License 行「CC0/Public Domain」为**无许可文本链接的纯声明性文字**（© Mike Pan & Morgan McGuire，https://casual-effects.com/data/ ；复核实证：同页 Clouds 的 CC0、Breakfast Room 的 CC BY 3.0、Bedroom 的 CC BY-SA 4.0、CornellBox 的 CC BY 3.0 各行 License 均带 creativecommons.org legalcode 链接，唯独 BMW 行无） | 暂定 CC0-1.0（未证） | 不适用（待定） | **待定**：与 Sponza 2010 声明同形态（无许可法律文本的声明性文字），按统一纪律 fail-closed 移出候选 PASS；须取得 Mike Pan 原始 Blender demo 发布面（blender.org demo files）CC0 法律文本或权利人一手确认后，方准按只追加程序入候选 |
| NVIDIA Emerald Square（ORCA） | **反例登记** | CC BY-NC-SA 3.0（https://developer.nvidia.com/orca/nvidia-emerald-square ；二次核验：ORCA 资产第三方 notices 转引） | CC-BY-NC-SA-3.0（白名单外） | — | **FAIL**（NC+SA 族明确排除；作为白名单外注入 RED 臂的测试夹具候选登记） |

**Sponza 裁决（冻结）**：Crytek Sponza 不满足 CC0/CC-BY 白名单，**首发清单冻结为 CornellBox + Bistro 两行**；Sponza 移入「特许待定池」——仅当白名单经只追加修订程序显式扩展（属立项级治理裁决，需独立程序，本 RFC 不构成该扩展）方可入清单；在此之前 Sponza 以任何形式进清单/缓存签名面即 RED。G10.3 如需第三场景补充，从上表白名单内追加候选经 §4.4 只追加程序进入。本裁决与立项裁决 3「许可逐资产核验后冻结（M131 门）」及 R-G10-4「未核验资产一律不得进清单」字面一致（§9 Q1 登记主会话可裁项）。**变体覆盖程序（F15 修法）**：任何 Sponza 变体入清单前，须枚举全部已知变体（Khronos glTF 版——纹理为 Alexandre Pestana SponzaPBR 第三方重打包层、McGuire 档案 OBJ 变体、marketplace 分发渠道等）并逐一一手核验（含纹理层与渠道条款层），「全部变体同源同族」结论须经核查登记方可引用。

### 4.3 🔒 资产外部缓存形式（M131/M132）

1. **git 零二进制（F11 修法：判据字面与启发式实现对齐）**：压测资产二进制一律不入 git 树与 git 历史。拦截面 = `.gitignore` 规则 + CI 守卫；守卫为启发式闭集拦截，判据字面冻结为「**守卫闭集内命中即 RED**」（不冒充数学意义绝对零）：①扩展名闭集 = `.glb` `.gltf`（二进制裁荷）`.bin` `.fbx` `.obj` `.exr` `.hdr` `.ktx2` `.zip` + 大图纹理扩展名 `.png` `.tga` `.dds` `.tif` `.tiff`（闭集全量列出，post-interlock 扩展走只追加修订行）；②体积阈值双判（量值 G10.3 按首发资产实测 measured 标定，禁手写——P-09）；③magic-bytes 内容嗅探（上述格式已知文件签名，防改扩展名绕过）。作用域 = 全仓工作树 + commit 范围检查面，全历史扫描为 G10.3 一次性基线核查；他域合法 zip/夹具经白名单路径闭集豁免并留痕。绕过面（阈值以下小文件、分片、未嗅探格式）由 digest 核验（本条 4）与人工 review 兜底，如实登记不夸大。
2. **K: 盘外部缓存**：二进制本体落 K: 盘外部缓存（立项裁决 9 + spike 磁盘事实：K: 余量 ≈3.6 TB，H: 仅 6.9 GB）。缓存根为机器局部配置；**仓库内只登记相对路径** `cache_rel`（禁主机绝对路径入签名面，沿 RFC-0020 `logical_uri` 纪律）；建议缓存根默认 `K:\rurix-ext\g10-corpus\`（非冻结字面值，机器局部）。**缓存根解析与重建程序（F10 修法）**：解析序闭集 = 环境变量 `RURIX_G10_CACHE_ROOT` 优先 → 机器局部配置文件兜底（文件名由下游 spec 冻结，不入签名面）；根不可达即 fail-closed，禁静默回退其他盘符。盘符漂移/多机迁移仅改机器局部配置，仓库 `cache_rel` 签名面不变；契约 guardrail「K: 盘」字面的软化（→「外部缓存盘（G10.1 实测 K:）」）由主会话经契约 §8 只追加区评估登记（本 RFC 不改契约字面）。**重建程序**：新机/缓存损毁后按元数据 JSON 逐资产重下载 → 逐文件实算并复算清单级 digest 比对（不符即 fail-closed 并触发 §4.2 的 URL 失效程序）→ 全量通过方恢复 ready。
3. **仓库内元数据 JSON 登记面**：每资产一行 = 按类登记闭集（§4.2 external 五元组 / generated 六字段 + 通用字段）+ `cache_rel` + `byte_len`；登记文件落 `milestones/g10/` 治理面（具体文件名与 schema 由下游 spec 条款冻结）。
4. **digest 核验**：加载门（M132）前对缓存逐文件实算 SHA-256 并复算清单级 canonical digest，与登记比对不符即 fail-closed；获取失败/缓存缺失的场景行诚实登记 `not-ready` 不充绿（R-G10-9），禁以 fixture 生成器产物冒充真实压测资产（G10_PLAN R-G10-9 止损逐字承接）。

### 4.4 🔒 场景清单冻结与只追加修订程序（M133）

1. **清单 schema（冻结字段集）**：每场景行 = `scene_id` + `asset_id`（须已按类登记，§4.2）+ 相机参数引用 + 光照参数引用 + `status`（`ready`/`not-ready`）；行集合语义无序，按 `scene_id` 稳定排序。
2. **清单 digest 注册**：冻结清单的 canonical digest 注册在树（M133 判据「清单 digest 在树」）；digest 算法 SHA-256，canonical 规则沿 RFC-0020 §4.2 同构子集（稳定排序、零时间戳、零主机路径）。
3. **只追加修订程序**：冻结后任何变更（追加场景、状态翻转、参数引用修订）以**新修订行 + 新清单 digest** 落盘并留痕；原地改既有行即 RED；白名单扩展同构走只追加修订行。
4. 白名单外资产行、按类登记缺字段资产行不得进入清单（M131 门序硬约束：先许可登记，后清单行）。
5. **ready 场景数下界（F9 修法，堵 vacuous PASS）**：G10.3 门验收时 `ready` 场景数 ≥ 首发清单基数（2）；缺额行必须 `not-ready` + `DEV_ENV_DEGRADE` 显式登记且不充绿。空清单或全 `not-ready` 清单的「逐场景」判定为 vacuous truth，一律不构 PASS（M132/G-G10-5 同口径）。
6. **M129 场景集时序定义（F8 修法）**：G10.2 验收 M129 时 M133 冻结清单尚不存在，M129 判据「场景清单」指 **G10.2 期暂定清单形态**——首发清单草案两行（CornellBox + Bistro）的最小场景面，scene_id 闭集于 G10.2 首日登记留痕；M128 判据「固定场景」同指该闭集。G10.3 M133 清单冻结后须对 M129 证据做回归复核（冻结清单 ⊇ 暂定场景面；缺行即登记漂移并按只追加程序处理）。禁以任意临时场景集冒充「场景清单」混过 M129。契约 M129 最晚波次（G10.2）维持不动；若主会话裁决后移波次，属契约修订程序，本 RFC 不预断。

### 4.5 UE EULA / CE-terms 事实与风险登记（只登记事实与风险，不构法律意见）

**事实（核查日 2026-08-15，全部附来源）**：

| # | 事实 | 来源 |
|---|---|---|
| E1 | UE EULA §2：授予私下使用/复制/展示/执行/修改 Licensed Technology 的许可（含开发 Products），前提是不违反协议与适用法律 | https://www.unrealengine.com/en-US/eula/unreal |
| E2 | EULA §4(a)(i)：渲染视频文件/图像属 **Non-Engine Products**，分发免版税（即使含 Starter Content 的渲染描绘）——G10 参考帧的定性面：渲染帧不是 Licensed Technology 本体 | 同上 |
| E3 | EULA §3(b)：非 Royalty 用途存在 seat subscription 义务及其例外（个人爱好/独立开发者年收入 <\$1M/非商业/教育机构） | 同上 |
| E4 | EULA §5(a)(ii)：允许在公共论坛贴 ≤30 行 Engine Code 片段用于讨论——**Rurix 不消费该宽限**，维持零片段复制更严纪律（§4.1） | 同上 |
| E5 | EULA「Non-Compatible Licenses」限制：不得将 Licensed Technology 与 GPL 等会改变其许可条款的代码/内容组合——Rurix 侧 UE 恒为外部进程、无代码组合，与该限制保持距离 | 同上（条款文本另见第三方随产品附 EULA 全文转引，如 https://arenabreakout.com/attribution.html ，二次核验） |
| E6 | EULA 含生成式 AI 训练输入禁止条款（Licensed Technology 不得作为 Generative AI Program 训练输入）——G10 参考帧与双端数据**不得用作生成式 AI 训练数据**的纪律登记 | 同上（条款原文转引核验：https://conductatlas.com/platform/unreal-engine/unreal-engine-eula/ai-training-input-prohibition/ ，二次核验） |
| E7 | Epic 官方 FAQ：「There is no blanket prohibition in the Unreal Engine EULA against using Unreal Engine in connection with other game engines. …provided you aren't copying code, you may use, learn from, and freely discuss Unreal…」——「用 UE 出参考帧做竞品渲染器对比」未见一揽子禁止；**以不复制代码为界** | https://enterprise.unrealengine.com/en-US/faq （同内容见 https://www.unrealengine.com/faq ） |
| E8 | EULA 设官方 change log（https://www.unrealengine.com/eula-change-log/unreal ）——条款可变，本登记以 2026-08-15 页面文本为准 | 同 E1 |
| E9 | 未检索到 EULA 中针对 benchmark/画面对比测试的专门禁止条款（以 2026-08-15 页面文本 + FAQ 为据） | 同 E1/E7 |
| E10 | CE-terms（Crytek）：2.1.2 禁与其他游戏引擎代码混合、2.1.3/2.1.4 Assets 仅随 CRYENGINE Game 使用/发布、2.4 禁再分发——Sponza 裁决依据（§4.2） | https://www.cryengine.com/ce-terms |
| E11 | EULA §7「Who Owns What」：所有权分层——Licensed Technology 归 Epic 所有（用户获得的是许可）；协议明示不就 Epic 商标（Unreal Engine、MetaHuman 名称与 logo 等 Epic Trademarks）授予任何许可，商标使用利益归 Epic——G10 参考帧对外材料的商标风险事实锚（联动 R-L3） | 同 E1（二次核验：Steam 商店 EULA 镜像同文 https://store.steampowered.com/eula/2373700_eula_1 ） |
| E12 | EULA §12 Records and Audits：簿记与审计义务随 Seat subscription 与 Royalty Addendum 的合规核验而设（审计经合理提前通知、由独立第三方执行、每 12 个月不超过一次等限制）——G10 非 royalty 研究用途的适用面不设判，与 R-L2 seat 义务定性一并届时复核 | 同 E11 |

**风险登记（不构法律意见；届时复核程序）**：

- **R-L1 EULA 可变性**：EULA 条款可经 change log 修订。程序：G10.2/G10.3 每波首日复核 change log 与 E1/E7 页面；命中变更即登记并触发 §4.4 同构的本 RFC 只追加修订。
- **R-L2 seat 义务定性**：G10 期本机研究性使用与 E3 例外（非商业/个人）的适配性、以及 G15 商用收口期 seat 义务，**不在本 RFC 裁决**——登记为 G15 立项复核项（§9 Q3）。
- **R-L3 对比材料对外发布**：G10 参考帧与 A/B 报告为内部证据；若 G11+ 计划对外发布含 UE 渲染帧的对比材料，需届时复核 EULA/商标/表述风险（本 RFC 不预放行）。
- **R-L4 账号条款分层**：Epic 账号 TOS、Fab/市场条款与 UE EULA 分属不同文本；人工接管点的账号交互以用户本人完成为准，agent 不代接受任何协议文本。
- **R-L5 Sponza 特许路径**：若立项级裁决拟为 Sponza 开口（如取得 Crytek 书面确认），须先完成许可文本一手核验再按只追加程序扩展白名单；仅凭 2010 年声明性文字不放行。

## 5. 下游 spec 条款映射（先符号、后实号）

**目标文件裁决**：**新建 `spec/external_reference.md`**，不追加 `spec/asset_pipeline.md`。理由：①`spec/asset_pipeline.md` 头部明示「资产管线条款（RFC-0020）」，是 G8.3 资产闭环（import/cook/DDC/页 ABI）的专属语义域；②本 RFC 的域是外部参照编排与语料治理/许可/缓存，与资产 cook 语义无实现耦合；③混入会污染 G8 追溯面。`spec/README.md` §4 文件清单的加性登记随首个条款 PR 同 PR 完成（post-interlock；G10.1 期 `spec/` 0-byte 纪律不动）。

本 RFC 不占用任何条款号。实现互锁开放、ledger 校准后，spec PR **按合入当时实测 `next_free` 顺位**将以下符号映射为真实 RXS，并同时落 traceability；实现 PR 不得先行（硬规则 7）。顺位分配**不保证连续、不预留区间**（沿 RFC-0020 §5/F15 先例）。

| 符号条款 | 拟定标题 | 最小测试锚定计划（每条 ≥1） |
|---|---|---|
| `XR-HARNESS` | UE 外部进程编排边界与出图命令面闭集 | 命令面 schema accept + schema 外开关注入 RED + execcmds 模板外注入 RED + 嵌凭据注入 RED + 仓库 UE 源性零扫描（扫描闭集 = 已知 UE 文件签名/扩展名/路径模式 + 体积阈值启发式，辅以人工 review 留痕——F18 修法，不冒充绝对判据）；provenance 七元组缺行 RED；`g10.p0.m128.ue5_capture_environment` / `g10.p0.m129.ue5_reference_frames` 挂接 |
| `XR-LICENSE` | 许可白名单 SPDX 闭集与按类登记 | 白名单外许可注入 RED（含 CC-BY-NC-SA-3.0 反例夹具）+ 未登记资产混入 RED + external 五元组 / generated 六字段缺字段 RED + 两类互冒充 RED；`g10.p0.m131.asset_license_registry` 挂接 |
| `XR-CACHE` | 外部缓存布局、git 零二进制与 digest 核验 | 清单级 digest 不符 fail-closed + 守卫闭集命中（扩展名/阈值/magic-bytes）RED + 绝对路径入签名面 RED + 缓存根不可达 fail-closed；`g10.p0.m132.corpus_loading` 挂接 |
| `XR-MANIFEST` | 场景清单冻结、清单 digest 注册与只追加修订 | 原地改 RED + 清单 digest 漂移检测 + 白名单只追加修订行留痕 + ready 下界（≥2）与 vacuous PASS 拦截；M133（P1）挂接 |

**错误码策略**：本 RFC 只冻结诊断类别（命令面越界、provenance 缺行、白名单外许可、按类登记字段缺失、digest 不符、git 二进制守卫闭集命中、清单原地改、ready 下界不满足）。不预造 RX 号；实现期按真实可达类别从 actual next-free 追加，en/zh message-key 同步。数字 CI step 一律 `post-interlock actual-next-free allocation`。

**gate key 事实源**：验收一律引用 G10_CONTRACT §4.2 与 CI_GATES 的唯一命名空间 `g10.p{0,1}.m1##.<slug>` + `ci/g10_<slug>_smoke.py`；本 RFC 不自行裁定 key 或脚本名。

## 6. feature gate、tracking 与实现序

### 6.1 治理门与实现门

1. G10.1：本 RFC Draft → D-409 对抗性评审（第 1 轮已执行，provenance 偏差如实登记 §9.1 F1：本环境单一模型 Kimi-K3，评审由零共享上下文独立隔离会话执行，不冒充异工具评审）→ findings 全 disposition（§9.1，本批 v0.2 已落）→ 主会话裁决 F1 偏差处置（异工具补轮 / 立项级豁免 / 偏差留存 G10.8b 终审复核）后翻 Agent Approved。此阶段只改治理文档/RFC，不落 spec/实现/数字 workflow。
2. G-G10-3 实现互锁：validator READY + 用户 G10.2 开工指令 + actual `next_free` 重校三者齐备方解锁；任一红即停止。
3. 互锁绿后：spec 条款 PR（`spec/external_reference.md` + `spec/README.md` §4 加行）→ RED 语料 → 实现 PR。

### 6.2 实现序（G10.2/G10.3）

1. G10.2：Launcher 安装 UE 5.8（人工接管点一次）→ spike 待验证清单 + **出图运行时登录态需求**（§4.1.5 F7 修法事实项）首日实测登记 → 暂定清单最小场景面 scene_id 闭集登记（§4.4 F8 修法）→ MRQ 主臂 harness + provenance 七元组登记 + 双跑 digest 门（M128/M129）。
2. G10.3（可与 G10.2 部分并行）：白名单内资产获取（Bistro + 追加候选按只追加程序；BMW 在待定池解封前不获取）→ 按类登记 + 清单级 digest 实测登记 → 缓存布局 + digest 核验 → 加载门（M132，ready 下界 ≥2）→ 清单冻结 + digest 注册（M133）→ M129 证据回归复核（§4.4 F8 修法）。
3. 真实红绿：白名单外注入/git 二进制注入/digest 篡改/清单原地改各构造缺陷 → 红 → 复原 → 绿，run URL 归档。

## 7. 备选方案

| 备选 | 裁决 | 理由 |
|---|---|---|
| UE 二进制/源码 vendoring 进仓库或子模块 | 否决 | G10_CONTRACT out-of-scope 字面（`ue_source_or_binary_vendoring_into_rurix_repo`）+ EULA 分发边界（E5/E4）；体积与许可双重不可行 |
| 压测资产入 git（含 git-lfs） | 否决 | 立项裁决 9 已定外部缓存 K: 盘 + 元数据登记；H: 盘 6.9 GB（R-G10-11）与 G15 商用再分发面双重排除 |
| 白名单放宽到 CC-BY-SA / CC-BY-2.0 全 CC 族 | 否决 | SA 传染性与商用交付物兼容性需个案裁决；精确名单 `{CC0-1.0, CC-BY-3.0, CC-BY-4.0}` 已覆盖全部已核验候选；扩展走只追加程序，不预放行 |
| Sponza 凭 2010「donated to the public」声明性文字入白名单 | 否决 | 非正式许可文本、无 SPDX、与 CE-terms 一手条款冲突；白名单纯度优先（fail-closed），特许走立项级程序（§9 Q1） |
| BMW 凭 McGuire 档案无链接声明行「CC0/Public Domain」直接判候选 PASS | 否决（F3 重判） | 与 Sponza 2010 声明同形态（无许可法律文本的声明性文字），双重标准不成立；统一纪律一律 fail-closed 入待定池，取得 CC0 法律文本/一手确认后方准按只追加程序进入 |
| 截图/人工采集帧代替 harness 出帧 | 否决 | G-G10-4 独立 RED 臂；不可机核（spike 问题 5 ③号路径判词） |
| 凭据写入 CI secret 自动登录 Epic | 否决 | R-G10-2：人工接管点即纪律；凭据永不在自动化面出现 |

## 8. 不做（范围红线）

- 不修改/编译/调试 UE 引擎本体；不消费 UE 源码参照面（只读 0-byte）。
- 不做画质修复、不设画质/帧率通过线（立项裁决 5；修复归 G11）。
- 不定义 A/B 度量语义（HDR 帧格式/FLIP/SSIM/PSNR/双端确定性契约——属并行 RFC-0026）。
- 不提供法律意见；不为任何白名单外许可做放行解释（含 Sponza）。
- 不把压测资产或 UE 渲染帧再分发进任何交付物/仓库（资产本体永在 K: 盘缓存）。
- 不把参考帧/双端数据用作生成式 AI 训练数据（E6）。
- 不触碰 G5~G9 冻结面与 00–14（触即显式 RFC 修订行）；不消费任何编号（RXS/RD/U/RX/CI step）。

## 9. 未决问题 / 关键裁决（Draft）

| 问题 | Draft 倾向 | 批准前需核 |
|---|---|---|
| Q1 Sponza 特许扩展是否立项（立项级裁决） | 默认 no-go 维持白名单纯净；首发清单 = CornellBox + Bistro，第三场景走白名单内追加候选 | 主会话确认是否启动独立特许程序（一手许可文本核验为先决） |
| Q2 追加候选入清单节奏 | San Miguel 2.0 / Breakfast Room 按 §4.4 只追加程序逐行进入，获取失败诚实 `not-ready`（R-G10-9）；BMW 已入待定池（F3 重判），解封前不进入 | G10.3 实测下载体积/镜像可用性（K: 盘登记） |
| Q3 UE seat subscription 义务定性 | G10 期登记事实不设判；G15 商用收口立项复核（R-L2，E12 联动） | 届时 EULA 文本 + 营收事实 |
| Q4 `-renderoffscreen` / `-execcmds` 时序 / 系统 .NET 兼容性 | spike 待验证项，G10.2 首日实测登记后选臂 | 实测记录归档；两臂诚实登记 |
| Q5 白名单是否含 CC-BY-2.0 | 暂不含；出现具体候选资产再走只追加扩展 | 有真实候选时复核 |
| Q6 F1 provenance 偏差处置（D-409 字面） | 补救承诺：主会话尝试 GitHub Copilot PR 评审作异工具补轮（§9.1 第 1 轮记录为其输入）；补轮失败则偏差留存，随 G10.8b 终审复核一并裁决 | 主会话裁决（补轮结果 / 立项级豁免 / 偏差留存终审），裁决前维持 Draft |

以上仅为 Draft 倾向；§9.1 完成且 findings 已 disposition 后才能随 Agent Approved 冻结。

## 9.1 对抗性评审记录（10 §3/§7，D-409）

> **第 1 轮评审已完成**（2026-08-15）。评审记录全文：[milestones/g10/design/rfc0027_adversarial_review.md](../milestones/g10/design/rfc0027_adversarial_review.md)。18 条 findings 逐条 disposition 如下，无空过。

| 字段 | 值 |
|---|---|
| 评审者 provenance | `Assisted-by: Kimi-K3（D-409 独立评审会话，与起草会话隔离）` |
| 评审轮次 | 第 1 轮，2026-08-15 |
| 评审方法 | 逐节细读 RFC 全文 + G10_CONTRACT §4.2/§6/立项裁决 2/3/9 + spike v1.0 + TEMPLATE-RFC；联网核查 UE EULA 现行页、CE-terms、glTF-Sample-Assets Sponza README、McGuire 档案逐模型 License 行、UE benchmark 条款检索 |

**F1 provenance 偏差如实登记（不静默处理）**：本环境为单一模型（Kimi-K3）；本轮评审由**零共享上下文的独立隔离会话**执行，但评审 provenance 字符串与起草 provenance 同为 Kimi-K3，**不满足 D-409「评审 provenance 须 ≠ 起草 provenance」的异工具/异模型字面，构成字面偏差**。本记录诚实定性为「独立会话隔离的批判性评审」，不冒充异模型/异工具评审。**补救承诺**：主会话尝试以 GitHub Copilot PR 评审作异工具补轮（本轮记录作为其首轮输入）；补轮失败则偏差留存，随 G10.8b 终审复核一并裁决（§9 Q6）。偏差处置未经主会话裁决前，本 RFC 维持 Draft。

**Findings 与 disposition**：

| # | Finding | 严重度 | Disposition |
|---|---|---|---|
| F1 | 评审 provenance 与 D-409 字面冲突 | high | **如实登记 + 补救承诺**（见上）；头表「对抗性评审」行、§6.1、§10 同步登记；最终处置（异工具补轮 / 立项级豁免 / 偏差留存 G10.8b 终审）由主会话裁决，本 RFC 不自封满足 D-409 |
| F2 | 五元组闭集对程序生成资产不自洽（CornellBox 缺 source_url 即 RED 卡死 M131） | high | **采纳**：§4.2 登记闭集分列 external 五元组 / generated 替代登记型六字段（`generator_script_digest` + `generator_params_digest` + 本地产物清单级 digest 替代 `source_url`，`NONE` 为字面闭集值）；M131 门按类求值、两类互冒充即 RED；契约 M131 判据/guardrail 字面「来源 URL」的同构精确化由主会话经契约 §8 只追加区登记（本 RFC 不改契约字面）；§3 补 generated 类 JSON 示例 |
| F3 | BMW 候选判 PASS 与 Sponza fail-closed 构成双重标准 | high | **采纳**：§4.2 新增统一纪律字面——凡「无许可法律文本的声明性文字」一律 fail-closed 入待定池、无例外；BMW 移出候选 PASS 入待定池（2026-08-15 复核 https://casual-effects.com/data/ ：BMW 行 License 为无链接纯文字「CC0/Public Domain」，同页 Clouds/Breakfast Room/Bedroom/CornellBox 各行均带 creativecommons.org legalcode 链接——评审断言属实），须取得 CC0 法律文本或一手确认后方准按只追加程序进入；§7 增备选否决行；§9 Q2 同步 |
| F4 | attribution 自由文本不可机核；CC-BY 4.0/3.0 要素差异未登记；复合许可表达缺失 | med | **采纳**：§4.2 attribution 改结构化子字段闭集（`creator`/`title`/`source_uri`/`license_uri`/`copyright_notice`/`modified_flag`，取 4.0 §3(a) 与 3.0 §4(a) 并集），子字段缺失即 RED；`spdx_id` 允许 SPDX 受限表达式（白名单 id 的 `AND` 组合 + `LicenseRef-<name>`，须附快照）表达复合许可 |
| F5 | digest 对多文件资产语义未定义、留伪绿口 | med | **采纳**：digest 冻结为清单级 canonical digest（逐文件 相对路径+sha256 稳定排序清单再 sha256，沿 RFC-0020 §4.2 同构子集）+ `file_count` + 总 `byte_len`；只取包内部分文件充数即 RED；§3/§4.3.4 同步 |
| F6 | 缺许可文本快照/checked_at/上游版本字段；URL 失效无程序 | med | **采纳**：§4.2 增 `license_snapshot`（快照入 git 治理面，纯文本不违反零二进制纪律）/`checked_at`/`upstream_ref` 三通用字段 + source_url 失效复核程序（快照 + archive.org 兜底，许可事实变动 fail-closed 重判） |
| F7 | Epic 人工接管点 CI/无人值守可执行性分层缺失 | med | **采纳**：§4.1.5 补——出图运行时登录态需求列 G10.2 首日实测登记；接管点重做触发闭集（新机器首跑/补丁更新后首跑/session 失效）；UE 面门与非 UE 面门 CI 分层，无登录态环境 `DEV_ENV_DEGRADE` 显式登记不充 G-G10-4 绿、不构死锁 |
| F8 | M129「场景清单」引用未冻结物，时序悬空 | med | **采纳**：§4.4 定义 G10.2 期 M129/M128 场景集 = 首发清单草案两行（CornellBox+Bistro）最小场景面暂定形态（scene_id 闭集首日登记），M133 冻结后回归复核；契约 M129 波次不动（后移属契约修订程序，不预断） |
| F9 | 空清单/全 not-ready 清单 vacuous PASS 通道 | med | **采纳**：§4.4 增 ready 场景数下界（≥ 首发清单基数 2），缺额行 `not-ready` + `DEV_ENV_DEGRADE` 显式登记不充绿；§5 XR-MANIFEST 锚定同步 |
| F10 | K: 盘符漂移风险；缓存根解析与重建程序缺失 | med | **采纳**：§4.3 补缓存根解析序闭集（环境变量 `RURIX_G10_CACHE_ROOT` 优先 → 机器局部配置兜底）+ 根不可达 fail-closed + 重建程序（重下载 + 清单级 digest 核验 + URL 失效程序联动）；契约 guardrail「K: 盘」软化由主会话经 §8 只追加区评估 |
| F11 | git 零二进制守卫绝对判据 vs 启发式实现落差 | med | **采纳**（取「判据字面与实现对齐」路径）：§4.3.1 判据改「守卫闭集内命中即 RED」——扩展名闭集全量列出 + 体积阈值 measured 标定 + magic-bytes 嗅探 + 作用域/检查面定义 + 豁免白名单路径留痕；绕过面由 digest 核验与人工 review 兜底，如实登记不冒充绝对零 |
| F12 | provenance 六元组缺出图臂维度；`ue_build_digest` 措辞误导 | low | **采纳**：六元组扩为七元组（+`capture_arm` = 臂 id + 命令面/queue 配置 digest）；`ue_build_digest` 改名 `ue_build_id`（版本号 + CL 字符串文本，非目录哈希）；§1/§3/§4.1/§5/§6.2/§10 同步 |
| F13 | E 表遗漏 §7 所有权/商标与 §12 审计适用面事实行 | low | **采纳**：补 E11（§7 Who Owns What：Licensed Technology 归 Epic、Epic Trademarks 不授权——R-L3 事实锚）、E12（§12 Records and Audits 随 Seat/Royalty 合规核验而设，G10 适用面不设判，联动 R-L2）；2026-08-15 经 EULA 页面 TOC 与 Steam 镜像同文二次核验 |
| F14 | CornellBox 数据源双标嫌疑与 NONE 判定歧义 | low | **采纳**：§4.2 事实表显式声明程序生成输入面（纯自写几何/反射率公式、不读取/转换外部数据文件为 NONE 成立前提）；「替代源」改写为「改用外部数据源则转 external 类按 SPDX 重新登记走只追加程序」 |
| F15 | Sponza 证据链变体覆盖与纹理层未展开 | low | **采纳**：Sponza 裁决段补变体覆盖程序——入清单前枚举全部已知变体（glTF 版 SponzaPBR 纹理层/McGuire OBJ 变体/marketplace 渠道）逐一一手核验 |
| F16 | 回退臂程序编号断裂（②缺失） | low | **采纳**：§4.1.6 改「首选② Launcher 臂受阻 → 回退①源码编译臂……；③公开参考图仅兜底」完整编号 |
| F17 | 臂 B `-execcmds` 自由文本与命令面闭集矛盾 | low | **采纳**：§4.1.3 补 execcmds 控制台命令白名单/参数模板闭集（`HighResShot <W>x<H>` 模板 + `r.ResetViewState`），模板外注入 fail-closed |
| F18 | 「仓库 UE 源性零扫描」机核方法未定义 | low | **采纳**：§5 XR-HARNESS 锚定定义扫描闭集（已知 UE 文件签名/扩展名/路径模式 + 体积阈值启发式）+ 人工 review 留痕，不冒充绝对判据 |

**总评处置**：评审总评「Draft 质量高于平均，但不满足翻 Agent Approved 条件——3 条 high 必须 disposition 后方可推进」。本批 v0.2 已落：F1 如实登记待主会话裁决（Q6），F2/F3 采纳并修 §4.2；med F4~F11 全部采纳并修对应节；low F12~F18 全部采纳（无驳回）。评审记功部分（E 表 E1~E4/E9 与 CE-terms 条款号逐字准确、Sponza 裁决证据链权威、SPDX id 规范、编号纪律合规）经复核无异议，相关事实行维持不变。findings 已全 disposition；F1 偏差处置经主会话裁决后，本 RFC 方可随主会话翻 Agent Approved。

## 10. 稳定化与 provenance

- 本 RFC 批准只冻结治理/设计边界，不代表任何能力实现绿色；实现仍由 G-G10-3 互锁决定。
- 互锁解除后：spec-first（`spec/external_reference.md`）→ RED 语料 → gated implementation → acceptance evidence（G-G10-4/G-G10-5）→ 两个里程碑无重大修订 → stabilization report → FCP-lite。
- 白名单、按类登记闭集（external 五元组 / generated 六字段）、provenance 七元组、清单只追加程序一经 Agent Approved 即语义冻结；变更须本 RFC 只追加修订行 + 新 digest 留痕。
- 起草 provenance：`Assisted-by: Kimi-K3（G10.1 治理波 RFC 起草）`。
- 评审 provenance：`Assisted-by: Kimi-K3（D-409 独立评审会话，与起草会话隔离）`——零共享上下文独立隔离会话但模型同为 Kimi-K3，provenance 相异要求构成字面偏差，如实登记 §9.1 F1，不冒充异工具评审。
- 修法批 provenance：`Assisted-by: Kimi-K3（D-409 修法批）`（按评审 18 条 findings 修订为 v0.2）。

## 11. 规范与实现依据

**仓内治理**：[10_GOVERNANCE](../10_GOVERNANCE.md) §3/§7/§9.5（D-409）· [13_DECISION_LOG](../13_DECISION_LOG.md) D-313 · [14_ENGINEERING_DISCIPLINE](../14_ENGINEERING_DISCIPLINE.md) §5 · [G10_CONTRACT](../milestones/g10/G10_CONTRACT.md) v1.0（立项裁决 2/3/9 · §4.2 · guardrails）· [G10_PLAN](../milestones/g10/G10_PLAN.md) v1.0 §2/§4 · [g10_ue5_harness_spike](../milestones/g10/design/g10_ue5_harness_spike.md) v1.0 · [RFC-0020](0020-asset-pipeline.md) §4.13/§5（许可审计与符号条款范式）。

**外部事实源（核查日 2026-08-15）**：

- UE EULA：https://www.unrealengine.com/en-US/eula/unreal ；change log：https://www.unrealengine.com/eula-change-log/unreal ；FAQ（竞品/学习讨论口径）：https://enterprise.unrealengine.com/en-US/faq （同 https://www.unrealengine.com/faq ）；AI 训练条款转引核验：https://conductatlas.com/platform/unreal-engine/unreal-engine-eula/ai-training-input-prohibition/ ；EULA 全文第三方转引核验：https://arenabreakout.com/attribution.html 。
- Bistro（ORCA）：https://developer.nvidia.com/orca/amazon-lumberyard-bistro （CC-BY 4.0 + 引用 BibTeX）；ORCA 总页：https://developer.nvidia.com/orca 。
- Sponza：https://github.com/KhronosGroup/glTF-Sample-Assets/blob/main/Models/Sponza/README.md （Legal = Cryengine Limited License Agreement）；CE-terms：https://www.cryengine.com/ce-terms 。
- Cornell Box：程序生成（本仓）；参考数据源 https://www.graphics.cornell.edu/online/box/data.html ；CC-BY-3.0 替代源 https://casual-effects.com/data/ 。
- 追加候选：McGuire Computer Graphics Archive 逐模型 License 行 https://casual-effects.com/data/ （San Miguel 2.0 CC BY 3.0 / Breakfast Room CC BY 3.0 / BMW 行声明性文字无链接——F3 重判待定；Emerald Square CC BY-NC-SA 3.0 反例 https://developer.nvidia.com/orca/nvidia-emerald-square ）；San Miguel 许可二次核验 https://github.com/msu-graphics-group/scenes ；EULA §7/§12 二次核验（Steam 镜像同文）https://store.steampowered.com/eula/2373700_eula_1 。
- 许可文本：CC0 1.0 https://creativecommons.org/publicdomain/zero/1.0/ ；CC BY 3.0 https://creativecommons.org/licenses/by/3.0/ ；CC BY 4.0 https://creativecommons.org/licenses/by/4.0/ ；SPDX License List https://spdx.org/licenses/ 。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v0.1 | 2026-08-15 | AI 起草初版（G10.1 治理波）：冻结 UE 出图编排边界（外部进程/命令面闭集/provenance 六元组/Epic 人工接管点/dev_env_degrade 口径）、压测资产许可白名单（精确 SPDX `{CC0-1.0, CC-BY-3.0, CC-BY-4.0}` + 五元组闭集 + 逐资产事实表 + Sponza fail-closed 裁决）、资产外部缓存（K: 盘 + 元数据登记 + git 零二进制）、场景清单冻结与只追加修订程序；UE EULA/CE-terms 事实与风险登记（不构法律意见）；§9.1 空段待 D-409 评审。零 RXS/CI/RD/U/RX 编号 claim；不改 `registry/number_ledger.json`（主会话统一核对）。起草 provenance `Kimi-K3` | Full RFC（Draft） |
| Draft v0.2 | 2026-08-15 | D-409 修法批（评审 18 findings 全 disposition，§9.1 回填）：**high**——F1 provenance 偏差如实登记 + 补救承诺（主会话尝试 GitHub Copilot PR 评审异工具补轮，失败则偏差留存 G10.8b 终审，Q6）；F2 登记闭集分列 external 五元组 / generated 六字段（generator_script_digest + 生成参数 digest + 本地产物清单级 digest），M131 门按类求值；F3 统一纪律字面（无许可文本的声明性文字一律 fail-closed），BMW 移出候选 PASS 入待定池（2026-08-15 复核 casual-effects.com/data 实证）。**med**——F4 attribution 结构化子字段闭集（CC-BY 3.0/4.0 要素并集）+ SPDX 受限表达式复合许可；F5 清单级 canonical digest；F6 license_snapshot/checked_at/upstream_ref + URL 失效程序；F7 接管点 CI 分层（运行时登录态首日实测登记 + 重做触发闭集 + UE 面门 dev_env_degrade 路径）；F8 M129 场景集时序修正（G10.2 暂定清单形态 + M133 冻结后回归复核）；F9 ready 下界 ≥2 堵 vacuous PASS；F10 缓存根解析序（RURIX_G10_CACHE_ROOT）+ 重建程序；F11 git 守卫判据与启发式实现对齐（扩展名闭集 + magic-bytes + measured 阈值 + 作用域）。**low**——F12 七元组（+capture_arm，ue_build_digest→ue_build_id）；F13 补 E11/E12；F14 CornellBox 程序生成输入面声明；F15 Sponza 变体枚举程序；F16 回退臂编号修正；F17 execcmds 模板闭集；F18 UE 源性零扫描闭集定义。E 表既有事实行（E1~E10）未改失真。状态维持 Draft（主会话核后翻 Approved）。修法批 provenance `Kimi-K3（D-409 修法批）` | Full RFC（Draft） |
