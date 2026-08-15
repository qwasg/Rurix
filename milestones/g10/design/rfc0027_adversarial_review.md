<!-- Assisted-by: Kimi-K3（D-409 独立评审会话，与起草会话隔离） -->
# RFC-0027 对抗性评审记录（D-409，独立评审会话）

| 字段 | 值 |
|---|---|
| 评审对象 | `rfcs/0027-external-reference-harness-license.md` Draft v0.1（2026-08-15） |
| 评审者 provenance | `Assisted-by: Kimi-K3（D-409 独立评审会话，与起草会话隔离）` |
| 评审轮次 | 第 1 轮，2026-08-15 |
| 评审方法 | 逐节细读 RFC 全文 + G10_CONTRACT §4.2/§6/立项裁决 2/3/9 + spike v1.0 + TEMPLATE-RFC；联网核查：UE EULA 现行页面全文（unrealengine.com/en-US/eula/unreal）、CE-terms 全文（cryengine.com/ce-terms）、glTF-Sample-Assets Sponza README、McGuire 档案逐模型 License 行（casual-effects.com/data）、UE benchmark 条款检索 |

## 0. 先行确认（RFC 做对的部分，如实记录）

1. **E 表条款号经逐字核对全部准确**：E1（§2 授权）、E2（§4(a)(i) Non-Engine Products 渲染帧免版税，含 Starter Content 渲染描绘）、E3（§3(b) seat 义务与 $1M/非商业/教育例外）、E4（§5(a)(ii) 30 行 Engine Code 公共讨论规则）与 2026-08-15 现行 EULA 网页版 18 节结构逐字一致（注意：旧 CDN PDF 版为 16 节结构，条款号不同，RFC 引用的是现行网页版，正确）。E9「未检索到 benchmark/画面对比专门禁止条款」经独立检索未见反例，结论稳健。E7 FAQ 竞品口径成立。
2. **CE-terms 引用逐字准确**：1.2（CRYENGINE Assets = 随引擎分发的视听文件）、2.1.2（禁与其他游戏引擎代码混合）、2.1.3/2.1.4（Assets 仅随 CRYENGINE Game 使用/发布）、2.4（禁再分发）与 cryengine.com/ce-terms 现行文本一致。Sponza fail-closed 裁决的一手证据链权威、方向正确。
3. Sponza README Legal 行（© 2016 Crytek, Cryengine Limited License Agreement）核实一致；SA/NC/ND 族排除有理；Emerald Square 反例夹具登记是 RED 臂好实践；零 RXS/CI/RD 编号 claim、不改 ledger 的纪律合规。
4. SPDX 三件套 id 写法本身规范（`CC0-1.0`/`CC-BY-3.0`/`CC-BY-4.0` 均为 SPDX License List 合法 id；CC0 在 SPDX 仅有 1.0 一版，无需更高版本号）。

## 1. Findings

### F1（high）评审 provenance 与 D-409 字面冲突——主会话必须裁决
- **位置**：RFC 头表「对抗性评审」行、§9.1；TEMPLATE-RFC §9.1；本记录头部。
- **问题**：RFC 明文要求「评审 provenance **须 ≠** 起草 provenance `Kimi-K3`」，硬规则 2 可机验。本评审会话模型同为 Kimi-K3，按 D-409 字面（不同 AI 工具/模型）不满足相异要求。本记录只能诚实定性为「独立会话隔离的批判性评审」，不能冒充异模型/异工具评审。
- **建议修法**：主会话合并时二选一并留痕：①另派非 Kimi-K3 工具/模型补一轮评审，本记录作为首轮输入；②由用户/立项级裁决显式豁免 provenance 相异要求（写明豁免理由与日期）。禁止静默把本记录填进 §9.1 即宣称 D-409 满足。

### F2（high）五元组闭集对程序生成资产不自洽——首发清单第一行即卡死
- **位置**：§4.2 五元组闭集、§3 JSON 示例、§4.2 事实表 CornellBox 行；G10_CONTRACT §4.2 M131 判据。
- **问题**：五元组 = asset_id + spdx_id + source_url + attribution + digest，「缺字段即 RED」。CornellBox 是首发清单两行之一且为程序生成资产：没有外部 source_url、没有获取面 digest 对象、SPDX 判定 NONE。按闭集字面，CornellBox 永远缺 source_url/digest 字段即 RED——M131 门对首发资产无法求值 PASS。合同 M131 判据字面同样写「来源 URL」。
- **建议修法**：为程序生成资产定义替代登记型：`spdx_id=NONE` + `source_url=NONE` + 以 `generator_script_digest`（生成器脚本 sha256）+ 生成参数/seed digest 替代资产 digest；或在 RFC 中把「资产」分为 external/generated 两类登记 schema，RED 判据按类求值。

### F3（high）BMW 候选判 PASS 与 RFC 自身纪律构成双重标准
- **位置**：§4.2 事实表 BMW 行、§4.2 白名单纪律、§7 备选方案 Sponza 行。
- **问题**：RFC 纪律明文「无许可文本的声明性文字一律不在族内」，否决 Sponza 的核心理由正是「2010 声明性文字、非正式许可文本」。经核查 McGuire 档案：BMW 行 License 标注为纯文字「CC0/Public Domain」**且无许可文本链接**（同页对照：Clouds 的 CC0、Breakfast Room 的 CC BY 3.0、Bedroom 的 CC BY-SA 4.0 均带 creativecommons.org legalcode 链接，唯独 BMW 无）。BMW 与 Sponza 2010 声明同为「无许可法律文本的声明性文字」，却一个候选 PASS、一个 fail-closed，标准适用不一致。
- **建议修法**：二选一：①BMW 降为「待一手核验」——核 Mike Pan 原始 Blender demo 发布面（blender.org demo files）是否附 CC0 法律文本，核验通过才准入候选；②在 §4.2 显式论证「档案维护者对自己修复版所作的许可授予声明」与「第三方转引原作者声明」的效力分层，并把该分层写成白名单纪律条文，消除双重标准外观。

### F4（med）attribution 自由文本不可机核；CC-BY 4.0/3.0 attribution 法定要素差异未登记
- **位置**：§4.2 五元组闭集 attribution 条、§3 JSON 示例。
- **问题**：①「attribution 文本须含创作者名 + 来源 + 许可链接」是自由文本要求，M131 门「五元组缺字段即 RED」机器无法求值「文本中是否含创作者名」——判据不可机核。②RFC 未登记 4.0 vs 3.0 的 attribution 差异：CC-BY-4.0 §3(a) 法定要素为创作者标示 + 版权声明 + 许可声明 + 免责声明 + 来源 URI + **修改标示（indicate if modified）**；3.0 §4(a) 要素为保留版权声明 + 合理方式署名 + 标题 + URI。RFC 三要素口径漏 title、copyright notice、disclaimer、修改标示。③复合许可表达力缺失：Breakfast Room 为 CC BY 3.0 主体 + public domain 大理石纹理（McGuire 档案同页登记），单 spdx_id 字段无法表达，实现者只能谎报或漏报。
- **建议修法**：attribution 改结构化子字段闭集（creator/title/source_uri/license_uri/copyright_notice/modified_flag），RED 判据落子字段缺失；spdx_id 允许 SPDX 表达式受限子集（AND 组合 + LicenseRef）或增加 notices 自由登记字段。

### F5（med）digest 对多文件资产语义未定义，且留伪绿口
- **位置**：§4.2 五元组 digest 条、§4.3.4 digest 核验。
- **问题**：Bistro = FBX + Falcor scene + 纹理集多文件（ORCA 分发形态），单 `digest` 字段未定义是对什么算——原始下载包？解压后目录？逐文件？「资产」边界未冻结，M132 加载门前的「复算 SHA-256 与登记比对」无法一致实现。伪绿口：只下载包内某小文件并对其算 digest 即可填满字段。
- **建议修法**：二选一冻结：①资产以原始下载包为单位登记（digest = 下载包 sha256 + byte_len），缓存保包不解压登记面；②digest 改为文件清单 canonical digest（逐文件 相对路径+sha256 的排序清单再 sha256，沿 RFC-0020 canonical 规则）。

### F6（med）登记面缺许可文本快照 / 核查日期 / 上游版本字段；URL 失效无程序
- **位置**：§4.2 五元组闭集、§4.2 核查纪律段、§4.3.3 元数据 JSON。
- **问题**：①五元组只登记 source_url，无许可文本快照要求——G15 商用审计时死链 = 证据链断裂；新机重建缓存同样依赖 URL 可达。②§4.2 纪律要求「核查一律给出来源 URL 与核查日期」，但五元组闭集无 checked_at 字段，纪律与 schema 不一致。③无 upstream 版本/commit 字段——同 URL 上游静默更新后 digest 漂移无法归因（McGuire 档案有 Updated 日期行，Bistro ORCA 有版本面）。
- **建议修法**：登记面增加 `license_snapshot`（许可文本快照入 git 治理面——文本非二进制资产，不违反零二进制纪律）、`checked_at`、`upstream_ref` 三字段；补 source_url 失效复核程序（archive.org 镜像或快照兜底）。

### F7（med）Epic 人工接管点在 CI/无人值守场景的可执行性分层缺失
- **位置**：§4.1.5 人工接管点协议、§6.2 实现序；R-G10-2。
- **问题**：①协议钉死「Launcher 首次登录 = 唯一人工接管点（一次性）」，但未登记关键事实「Launcher 安装的 UnrealEditor-Cmd 运行时是否需要登录态」（通行事实为不需要，RFC 未作事实登记也未列入 spike 待验证清单）——此事实决定接管点究竟是「安装时一次」还是「每次 CI 运行前」。②session 过期、UE 补丁更新需再登录、多机迁移后新机器重做接管点，均无程序；「唯一/一次性」字面与这些现实触发冲突。③与 R-G10-2「凭据永不进 CI」叠加：无预登录态的 CI runner 跑 M128/M129 永远 DEV_ENV_DEGRADE，G-G10-4 永不绿——哪类机器承载 UE 出图门（预登录专用本机？）、DEV_ENV_DEGRADE 在波次退出门的处置，文本未规定，存在死锁。
- **建议修法**：§4.1.5 补：运行时登录态需求列入 G10.2 首日实测登记；接管点重做触发条件闭集（新机器/补丁更新/session 失效）；UE 出图门执行面分层声明（承载机器资格 + 非 UE 面门与 UE 面门的 CI 分层）。

### F8（med）M129 判据引用未存在物——「场景清单」时序悬空
- **位置**：G10_CONTRACT §4.2 M129 行（最晚 G10.2）、M133（P1，最晚 G10.3）；RFC §6.2 实现序 1/2。
- **问题**：M129 判据「场景清单逐场景参考帧落盘」，但场景清单冻结（M133）与首发资产获取（G10.3）都在 M129 的最晚波次之后——G10.2 验收 M129 时不存在已冻结清单，「逐场景」引用哪份清单全文未定义。M128 的「固定场景」同样未定义来源。伪绿口：用任意临时小场景集充「场景清单」即可混过 M129。
- **建议修法**：RFC 明确 G10.2 期 M129 的场景集定义（如「首发清单草案两行 CornellBox+Bistro 的最小场景面」或固定场景 id 闭集），并规定 G10.3 清单冻结后对 M129 证据的回归复核义务；或主会话把 M129 最晚波次修订到 G10.3（触契约修订程序）。

### F9（med）空清单 / 全 not-ready 清单的 vacuous PASS 通道
- **位置**：§4.3.4 not-ready 口径、§4.4 清单 schema；G-G10-5「清单全场景加载绿」；M132「逐场景加载成功」。
- **问题**：「逐场景」对零 ready 行的清单是 vacuous truth；Bistro 下载失败（诚实 not-ready）+ CornellBox 异常时，清单可零 ready 行仍满足字面。无 ready 行数下界判据。
- **建议修法**：M132/M133 增加下界判据：ready 场景数 ≥ 首发清单基数（2），缺额行必须 DEV_ENV_DEGRADE 显式登记且波次退出门不充绿。

### F10（med）K: 盘符在契约/治理面的漂移风险；缓存根解析与重建程序缺失
- **位置**：§1 ASCII 图、§4.3.2；G10_CONTRACT 立项裁决 9 / guardrails「外部缓存 K: 盘」。
- **问题**：RFC §4.3.2 已正确把缓存根降为机器局部配置、签名面只登记 cache_rel（做对了），但：①契约层字面钉死「K: 盘」，盘符漂移/多机迁移后 guardrail 字面变伪事实；②缓存根写在哪份机器局部配置、CI 如何发现、根不可达时 fail 模式，均未规定；③新机/缓存损毁后的重建程序（重下载 + digest 核验 + 与 F6 的 URL 失效耦合）没有文本。
- **建议修法**：§4.3 补「缓存根解析机制（配置文件/环境变量闭集）+ 根不可达 fail-closed + 重建程序」小节；主会话评估把契约 guardrail「K: 盘」软化为「外部缓存盘（G10.1 实测 K:）」。

### F11（med）git 零二进制守卫：绝对判据 vs 启发式实现的落差
- **位置**：§4.3.1。
- **问题**：判据字面「任一资产二进制入树即 RED」是绝对的；实现「扩展名闭集 + 体积阈值双判」是启发式——改扩展名、小于阈值、分片即可绕过。且：「大图纹理」未给扩展名闭集（.png/.tga/.dds/.tif 是否在内）；体积阈值量值与标定方式未定；守卫作用域（全仓 vs 限定目录——会误伤他域合法 zip 夹具）与检查面（工作树 / commit 范围 / 全历史）未定。
- **建议修法**：补 magic-bytes 内容嗅探要求 + 扩展名闭集全量列出 + 阈值 measured 标定口径 + 作用域/检查面定义；或把判据措辞与启发式实现对齐（承认「守卫按闭集拦截」而非「任一」绝对零）。

### F12（low）provenance 六元组缺出图臂维度；ue_build_digest 措辞误导
- **位置**：§4.1.3 选臂登记、§4.1.4 六元组闭集。
- **问题**：§4.1.3 要求「两臂诚实登记禁伪绿」，但六元组 {scene_id, camera_params_digest, lighting_params_digest, ue_build_digest, gpu_driver_version, clock_lock_state} 无 render_arm / 命令面 digest / MRQ queue 配置 digest——同 scene 不同臂出帧从 provenance 无法区分，选臂诚实登记落空。另 `ue_build_digest` 暗示哈希，Launcher 版 build 标识实为版本号+CL 字符串，不可能对数十 GB 安装目录取 hash。
- **建议修法**：六元组扩为七元组（+capture_arm：臂 id + 命令面/queue 配置 digest）；ue_build_digest 改名 ue_build_id（版本+CL 文本）。

### F13（low）E 表遗漏：输出物权利归属与商标事实行
- **位置**：§4.5 E1~E10。
- **问题**：条款号准确性见 §0.1（记功），但评审要求④点名的「截图/输出物权利归属」只有 E2 的免版税定性面，未登记 EULA §7「Who Owns What」（Licensed Technology 归 Epic、Product 归用户、Epic 商标不授权）事实行；商标/logo 义务只在 R-L3 风险里一带而过、无事实锚。可补 §12 Records and Audits 的适用面（非 royalty 用途不适用）一句。
- **建议修法**：补 E11（§7 所有权与商标不授权）、可选 E12（§12 审计适用面）。

### F14（low）CornellBox 数据源的双标嫌疑与 NONE 判定歧义
- **位置**：§4.2 事实表 CornellBox 行。
- **问题**：一面判 NONE（程序生成零摄入），一面登记「几何/反射率数据源参考 Cornell PCG『Public Use Data』页（页面无显式许可文本）」并附「CC-BY-3.0 替代源 McGuire 档案 CornellBox.zip」。Cornell PCG 页「无显式许可文本」按 RFC 自己的纪律本应 fail-closed；且若程序生成器实际读取/转换 Cornell 数据或 McGuire OBJ，SPDX 判定即非 NONE。判定行与替代源行并存，登记语义含糊。
- **建议修法**：显式声明程序生成输入面（纯自写几何/反射率公式 = NONE 成立）；「替代源」改写为「若改用外部数据源则按该数据 SPDX 重新登记走只追加程序」。

### F15（low）Sponza 证据链的变体覆盖与纹理层未展开
- **位置**：§4.2 事实表 Sponza 行、Sponza 裁决段。
- **问题**：裁决方向正确、一手证据权威（见 §0.2）。但：①glTF 版 Sponza 纹理为 Alexandre Pestana SponzaPBR 第三方重打包层（README Sources/Licensing notes 明示），证据链未展开纹理层；②McGuire 档案亦存 Sponza OBJ 变体（同 Crytek 源），RFC 未登记「全部变体同源同族」核查结论；③README 记 2023-03-08 Crytek marketplace 确认可得，marketplace 条款层未分析。均不翻转 fail-closed。
- **建议修法**：特许待定池程序（§9 Q1 / R-L5）补一句「任何 Sponza 变体入清单前须枚举全部已知变体并逐一一手核验（含纹理层）」。

### F16（low）回退臂程序编号断裂
- **位置**：§4.1.6。
- **问题**：行文「回退①源码编译臂……③公开参考图仅兜底」——②缺失（②即 Launcher 首选本身），阅读跳跃。
- **建议修法**：改为「首选②受阻 → 回退①……；③仅兜底对照」的完整编号。

### F17（low）臂 B `-execcmds` 自由文本与命令面闭集矛盾
- **位置**：§4.1.3 臂 B。
- **问题**：命令面闭集要求「schema 外开关/参数注入即 fail-closed」，但 `-execcmds="…"` 内嵌控制台命令是自由文本，内容未模板化则闭集形同虚设（注入面从开关层移到 execcmds 内容层）。
- **建议修法**：execcmds 允许的控制台命令白名单/参数模板闭集（如仅 `r.ResetViewState` + `HighResShot <W>x<H>` 模板）。

### F18（low）XR-HARNESS「仓库 UE 源性零扫描」机核方法未定义
- **位置**：§5 XR-HARNESS 测试锚定计划。
- **问题**：「仓库 UE 源性零扫描」扫描什么指纹（UE 源码片段库？文件头？路径名？）无可行定义，判据有不可求值风险。
- **建议修法**：定义扫描闭集（已知 UE 文件签名/扩展名/路径模式 + 体积阈值）+ 人工 review 留痕，或把该锚定降为纪律声明 + 抽查程序。

## 2. 总评

**总体判断：Draft 质量高于平均，但不满足翻 Agent Approved 条件——3 条 high 必须 disposition 后方可推进。**

RFC 在许可事实核查面上表现扎实：E 表与 CE-terms 条款号经独立逐字核对全部准确（这在许可登记类文档中并不常见），Sponza fail-closed 裁决证据链权威、方向正确，编号纪律与互锁纪律合规。但存在三类系统性短板：

1. **schema 自洽性**（F2/F5/F6/F4）：五元组闭集对首发清单第一行（CornellBox）即无法求值，digest/attribution/URL 三个字段的机器可核性都未闭环——M131 门按当前文本不可诚实求值，这正是 §2.1 自己指出的缺口（「M131 门当前没有可求值的白名单本体」）在 RFC 内部仍未完全闭合。
2. **标准适用一致性**（F3/F14）：BMW 与 Sponza 同形态的「声明性文字」得到相反处置，Cornell PCG 无许可文本数据页被引用——fail-closed 纪律必须无例外适用，否则白名单的严肃性在 G15 商用审计面前不成立。
3. **时序与执行面**（F7/F8/F9）：M129 引用未冻结的清单、CI 无登录态下 UE 门死锁、空清单 vacuous PASS——三处都是不真下载/真出图也可能混绿的通道或永不绿的死锁，与 RFC 自身的反伪绿立场直接相关。

**Disposition 建议**：F1~F3（high）翻 Approved 前必须显式 disposition（F1 由主会话/用户裁决，F2/F3 采纳并修 §4.2）；F4~F11（med）建议采纳并修对应节；F12~F18（low）可采纳修订或留痕驳回并附理由。本评审不改 RFC 本体；逐条 disposition 由主会话合并时回填 §9.1。
