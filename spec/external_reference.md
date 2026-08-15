# external_reference.md — 外部参照 harness 与压测语料治理语义面（G10 M128~M133）

> **地位**：G10 外部参照与压测语料治理轴事实源——UE 5.8 出图 harness 编排
> 边界（外部进程纪律 / 出图命令面闭集 / provenance 七元组 / Epic 人工接管
> 点）、压测资产许可白名单与按类登记（M131）、资产外部缓存与 git 零二进制
> 守卫（M132 前置 digest 核验）、场景清单冻结与只追加修订程序（M133）
> （RFC-0027，Agent Approved 2026-08-15，§4.1~§4.4 逐字承接；G10_CONTRACT
> §4.2 M128/M129/M131/M132 行 + G10_ACCEPTANCE_MAP §1/§2 + §2 M133 行〔判据
> 逐字〕）。本文件不承载 A/B 度量语义（帧捕获 HDR/FLIP/SSIM/PSNR/双端确定
> 性契约属 RFC-0026 面）；G8 资产管线 cook 语义（spec/asset_pipeline.md
> RXS-0328~0343）**字面 0-byte 不动**。
>
> **档位**：Full RFC / RFC-0027。
>
> **编号**：RXS-0380~0383（G10.3 spec-first，自合入时实测
> `registry/number_ledger.json` `RXS.next_free = 380` 顺位领取，0380~0383
> 连续不跳号；编号永不复用，10 §9.5）。
>
> **新建裁决留痕（G10.3 spec PR）**：RFC-0027 §5 目标文件裁决逐字承接——
> 新建本卷（外部参照编排与语料治理/许可/缓存独立语义轴，沿 G9.2
> virtual_geometry.md / G9.4 global_illumination.md / G9.5 双卷 / G9.6
> physics.md 新建先例）。候选既有卷 spec/asset_pipeline.md 头部明示
> 「资产管线条款（RFC-0020）」为 G8.3 资产闭环专属语义域，混入会污染 G8
> 追溯面，本体 0-byte（spec/README.md §4 登记 + 本头注留痕）。
>
> **编号纪律**：条款号按合入当时实测 `next_free` 顺位，**不保证连续、不预
> 留区间**（RFC-0027 §5 沿 RFC-0020 §5/F15 先例字面）；本卷不消费任何
> RX 错误码 / RD / U / SG（诊断类别见 RXS-0381~0383 Legality，实现期按真
> 实可达类别从 actual next-free 追加，en/zh message-key 同步）。

---

## 1. 范围与体例

- 体例 = FLS 风格（spec/README.md §2）；本文件**严禁 UB 节**——许可登记、
  缓存核验、清单冻结与 harness 编排的所有失败均为 typed 拒绝 / 确定性
  fail-closed（登记缺字段、白名单外许可、digest 不符、守卫闭集命中、清单
  原地改一律判 RED），不设未定义行为。
- 实现锚定（实现期命名）：许可登记与缓存/清单核验落 `ci/g10_*_smoke.py` 纯
  host 门脚本 + `milestones/g10/` 治理面 JSON 登记件；UE 出图 harness 编排
  落 G10.2 波 `ci/g10_ue5_*_smoke.py`；压测资产二进制本体恒在仓库外外部缓
  存（RXS-0382 L2），仓库内零二进制。
- 每条款 ≥1 `//@ spec: RXS-####` 测试锚定（traceability 矩阵全锚定，10 §4）。

## 2. 术语

- **external 类资产**：自仓库外来源获取的第三方资产（本卷首发 = Amazon
  Lumberyard Bistro，ORCA，CC-BY-4.0）；按五元组闭集登记（RXS-0381 L3）。
- **generated 类资产**：仓库内程序生成的资产（本卷首发 = Cornell Box，
  `ci/_gen_g10_cornell_box.py` 纯自写几何/反射率公式，不读取/转换任何外
  部数据文件）；按替代登记型六字段闭集登记（RXS-0381 L4）。
- **清单级 canonical digest**：多文件资产的登记 digest——逐文件
  `相对路径 + sha256` 按路径稳定排序的清单再 sha256（canonical 规则沿
  RFC-0020 §4.2 同构子集：LF 行分隔、相对路径正斜杠、零时间戳、零主机
  绝对路径），并附 `file_count` 与总 `byte_len`（RFC-0027 §4.2 F5 修法）。
- **首发清单基数**：G10 首发场景清单 = CornellBox + Bistro 两行（立项裁
  决 3 + RFC-0027 §4.2 Sponza 裁决），ready 场景数下界 = 2（RXS-0383 L6）。

### RXS-0380 UE 外部进程编排边界与出图命令面闭集（XR-HARNESS）

**Legality**

- L1 UE 5.8 恒为**外部进程**。rurix 仓库（`src/`、`spec/`、`conformance/`、
  `.github/`、`ci/`、文档）内零 UE 二进制、零 UE 源码/着色器片段复制、零
  vendoring 目录；违反即 revert + 留痕（G10_CONTRACT guardrail 逐字承接，
  RFC-0027 §4.1.1）。UE 安装与源码参照面只读 0-byte；帧库与大体积产物落
  外部缓存盘（RXS-0382）。
- L2 **出图命令面闭集**：仅允许 spike 实证的三臂命令形态——臂 A（MRQ 批
  量臂：`-game -MoviePipelineConfig=<queue> -windowed -resx/-resy -log
  -notexturestreaming -Unattended`）、臂 B（快速截屏臂：`-game -benchmark
  -fps=<N> -seconds=<N> -ResX/-ResY -execcmds="…" -unattended -log
  -FixedSeed`）、臂 C（Python 编排臂：`-ExecutePythonScript=<script>.py` +
  MoviePipelineQueueSubsystem 回调退出）。命令行由结构化参数生成（禁
  shell 字符串拼接注入）；schema 外开关/参数注入即 fail-closed；臂 B 的
  `-execcmds` 内嵌控制台命令同闭集化——仅允许白名单控制台命令 + 参数模
  板（当前闭集 = `HighResShot <W>x<H>` 模板 + `r.ResetViewState`；扩展走
  只追加修订行），模板外自由文本注入即 fail-closed（RFC-0027 §4.1.3）。
- L3 **provenance 七元组闭集**（逐帧登记，缺行即 RED）：`scene_id` /
  `camera_params_digest` / `lighting_params_digest` / `ue_build_id` /
  `gpu_driver_version` / `clock_lock_state` / `capture_arm`。其中
  `ue_build_id` = Launcher 版本号 + CL 字符串文本（非安装目录哈希）；
  `capture_arm` = 出图臂 id（A/B/C）+ 命令面/queue 配置 digest。时间戳、
  主机绝对路径、用户名字段不得进入签名面（RFC-0027 §4.1.4）。
- L4 **Epic 账号人工接管点**：Launcher 首次 Epic 登录 = 唯一人工接管点
  （一次性用户交互）；凭据永不进命令行参数、环境变量、日志、CI、仓库。
  接管点未完成 = `DEV_ENV_DEGRADE`（不充 P0 绿）；重做触发闭集 = 新机器
  首跑 / UE 补丁更新后首跑 / session 失效，三者之外不得要求人工交互
  （RFC-0027 §4.1.5）。
- L5 **回退臂程序**：首选 Launcher 臂登录受阻且人工介入不可得 → 回退源
  码编译臂 → 契约 §8 只追加修订本波判据；**禁以截图/人工采集帧冒充
  harness 出帧**；公开参考图仅兜底对照材料，不进验收证据链（RFC-0027
  §4.1.6）。

**Implementation Requirements**

- IR1 出图 harness 须提供仓库 UE 源性零扫描：扫描闭集 = 已知 UE 文件签名
  /扩展名/路径模式 + 体积阈值启发式，辅以人工 review 留痕（不冒充绝对判
  据，RFC-0027 §5 XR-HARNESS 锚定 F18 修法）。
- IR2 本条款挂接 `g10.p0.m128.ue5_capture_environment` /
  `g10.p0.m129.ue5_reference_frames`（G10.2 波实现）；测试锚定 =
  conformance/external_reference/ 语料 + G10.2 门脚本。

### RXS-0381 许可白名单 SPDX 闭集与按类登记（XR-LICENSE）

**Legality**

- L1 **白名单（精确 SPDX 名单，冻结）**：`{ CC0-1.0, CC-BY-3.0, CC-BY-4.0 }`。
  仅上述三个 SPDX id 合法；NC/ND/SA 后缀变体、自定义/专有许可、无许可法
  律文本的声明性文字一律不在族内——凡「无 creativecommons.org legalcode
  或等效正式文本锚点的许可标注」一律 fail-closed 入待定池，无例外（档
  案维护者声明、原作者声明、第三方转引同此标准）；白名单外许可注入即
  RED（M131 判据逐字）。`spdx_id` 允许 SPDX 受限表达式子集：白名单 id 的
  `AND` 组合 + `LicenseRef-<name>`（LicenseRef 行必须附
  `license_snapshot`）；表达式中出现白名单外 id 即 RED（RFC-0027 §4.2）。
- L2 **登记闭集分两类（逐资产，按类缺字段即 RED）**：
  - **external 类五元组**：`asset_id` + `spdx_id` + `source_url` +
    `attribution` + `digest`（SHA-256 清单级 canonical digest，获取时实
    测登记，禁手写——P-09；只下载包内部分文件充数即 RED）。
  - **generated 类替代登记型六字段**：`asset_id` + `spdx_id=NONE` +
    `source_url=NONE` + `generator_script_digest` + `generator_params_digest`
    + `digest`（本地产物清单级 canonical digest，规则同 external 类）。
    `NONE` 为字面闭集值而非缺字段。
  - **两类不得互相冒充**：generated 类谎报外部来源、external 类借
    generated 类规避 source_url/digest，均即 RED。
- L3 **两类通用登记字段**：`license_snapshot`（许可文本快照文件名，快照
  入 git 治理面——纯文本非二进制资产；内容为官方 legalcode 文本 + 一手
  页面 License 行摘录；generated 类登记生成器内嵌许可头或 `NONE`）、
  `checked_at`（许可核查日期）、`upstream_ref`（上游版本/Updated 日期/
  commit；无版本面者登记 `NONE`）、`cache_rel`（仓库外缓存相对路径，禁主
  机绝对路径入签名面）、`file_count`、`byte_len`。source_url 失效程序：
  复核发现 URL 失效即登记漂移事件，以 `license_snapshot` + archive.org
  镜像兜底复核；许可事实未变者仅追加 `checked_at` 修订行，许可事实变动
  者按 fail-closed 重判。
- L4 **attribution 结构化子字段闭集**（替代自由文本、使机器可核）：
  `creator` / `title` / `source_uri` / `license_uri` / `copyright_notice` /
  `modified_flag`（布尔；产出物含修改时须 `true` 并附修改说明行）；闭集
  取 CC-BY-4.0 §3(a) 与 CC-BY-3.0 §4(a) 法定要素并集（TASL +
  copyright_notice + disclaimer 行 + modified_flag），按资产 `spdx_id` 对
  应版本求值，子字段缺失即 RED；CC0 无法定 attribution 义务仍登记出处
  行（`creator`/`source_uri`）。
- L5 **未登记资产混入清单即 RED；登记 digest 与缓存实算不符即 RED**
  （M131 判据逐项承接，RFC-0027 §4.2 fail-closed RED 臂）。
- L6 **逐资产许可事实表（联网核查日 2026-08-15）为登记事实源**：Cornell
  Box（程序生成，generated 类，NONE，PASS）/ Bistro（ORCA，CC-BY-4.0，
  PASS——ORCA 页 License 行「Creative Commons CC-BY 4.0」+ © 2017 Amazon
  Lumberyard 引用 BibTeX + 包内 LICENSE.txt 全 CC-BY-4.0 legalcode）/
  Crytek Sponza（Cryengine Limited License Agreement 自定义引擎许可，
  白名单外，FAIL——移入特许待定池，以任何形式进清单/缓存签名面即 RED）/
  NVIDIA Emerald Square（CC-BY-NC-SA-3.0，白名单外反例登记，作为白名单
  外注入 RED 臂测试夹具候选）；San Miguel 2.0 / Breakfast Room 为白名单
  内追加候选（待获取 + digest 实测登记后经只追加程序入清单）；BMW 在待
  定池解封前不获取不进入（RFC-0027 §4.2 事实表与裁决逐字承接）。

**Implementation Requirements**

- IR1 许可事实核查一律给出来源 URL 与核查日期；二次来源须标注「二次核
  验」，与一手页面分列。
- IR2 本条款挂接 `g10.p0.m131.asset_license_registry`（G10.3 波实现）；
  测试锚定 = conformance/external_reference/ 语料 + M131 门脚本 RED 臂
  （白名单外注入 / 未登记混入 / 按类缺字段 / 两类互冒充 / digest 篡改）。

### RXS-0382 外部缓存布局、git 零二进制与 digest 核验（XR-CACHE）

**Legality**

- L1 **git 零二进制**：压测资产二进制一律不入 git 树与 git 历史。拦截面
  = `.gitignore` 规则 + CI 守卫；守卫为启发式闭集拦截，判据字面 = 「守卫
  闭集内命中即 RED」（不冒充数学意义绝对零）：①扩展名闭集 = `.glb`
  `.gltf`（二进制裁荷）`.bin` `.fbx` `.obj` `.exr` `.hdr` `.ktx2` `.zip`
  `.png` `.tga` `.dds` `.tif` `.tiff`（闭集全量列出，扩展走只追加修订
  行）；②体积阈值双判（量值按首发资产实测 measured 标定，禁手写——
  P-09，阈值与标定来源登记进 M131 注册表 `git_binary_guard` 面）；③
  magic-bytes 内容嗅探（上述格式已知文件签名，防改扩展名绕过）。作用域
  = 全仓工作树 + commit 范围检查面；既有合法夹具/资产经白名单路径闭集豁免并
  留痕（`conformance/asset/gltf/**`、`tests/geom_pages/golden/**`、
  `apps/uc09-taichi-spike/assets/particles.tcm`〔既有 zip 容器 .tcm 资产，魔
  数嗅探命中豁免留痕〕）。绕过
  面（阈值以下小文件、分片、未嗅探格式）由 digest 核验（L4）与人工
  review 兜底，如实登记不夸大（RFC-0027 §4.3.1 F11 修法）。
- L2 **缓存根解析序闭集**：环境变量 `RURIX_G10_CACHE_ROOT` 优先 → 机器局
  部配置文件 `<repo>/g10_cache_root.local.json`（JSON 对象
  `{"cache_root": "<本机绝对路径>"}`，gitignored，**不入签名面**）兜底 →
  缺省 `K:\rurix_g10_cache`（机器局部约定值，盘符漂移/多机迁移仅改机器
  局部配置，仓库 `cache_rel` 签名面不变）。根不可达即 fail-closed，禁静
  默回退其他盘符。**重建程序**：新机/缓存损毁后按元数据 JSON 逐资产重下
  载/重生成 → 逐文件实算并复算清单级 digest 比对（不符即 fail-closed 并
  触发 RXS-0381 L3 URL 失效程序）→ 全量通过方恢复 ready（RFC-0027 §4.3.2
  F10 修法）。
- L3 **仓库内元数据 JSON 登记面**：每资产一行 = 按类登记闭集（RXS-0381
  L2/L3）+ `cache_rel` + `byte_len`；登记文件 =
  `milestones/g10/g10_asset_license_registry.json`（冻结路径）。外部格式
  转换产物（如 FBX→glTF 派生物）以 `derived` 块登记：转换器标识与版本
  pin + 转换器本体 sha256 + 输入 digest + 派生物清单级 canonical digest；
  派生物 digest 与缓存实算不符即 RED。
- L4 **digest 核验**：加载门（M132）前对缓存逐文件实算 SHA-256 并复算清
  单级 canonical digest，与登记比对不符即 fail-closed；获取失败/缓存缺
  失的场景行诚实登记 `not-ready` 不充绿（R-G10-9），禁以 fixture 生成器
  产物冒充真实压测资产。

**Implementation Requirements**

- IR1 缓存布局：`<cache_root>/<asset_id>/<versioned_subdir>/`；`cache_rel`
  登记到 versioned_subdir 一层，正斜杠相对路径。
- IR2 本条款挂接 `g10.p0.m132.corpus_loading`（G10.3 波实现）；测试锚定
  = conformance/external_reference/ 语料 + M132 门脚本 RED 臂（digest 篡
  改 / 守卫闭集命中 / 绝对路径入签名面 / 缓存根不可达）。

### RXS-0383 场景清单冻结、清单 digest 注册与只追加修订（XR-MANIFEST）

**Legality**

- L1 **清单 schema（冻结字段集）**：每场景行 = `scene_id` + `asset_id`
  （须已按 RXS-0381 按类登记）+ 相机参数引用 + 光照参数引用 + `status`
  （`ready`/`not-ready`）；行集合语义无序，按 `scene_id` 稳定排序。清单
  文件 = `milestones/g10/g10_corpus_scene_manifest.json`（冻结路径）。
- L2 **清单 digest 注册**：冻结清单的 canonical digest 注册在树（M133 判
  据「清单 digest 在树」）；digest 算法 SHA-256，canonical 规则沿
  RFC-0020 §4.2 同构子集（稳定排序、零时间戳、零主机路径）。
- L3 **只追加修订程序**：冻结后任何变更（追加场景、状态翻转、参数引用
  修订）以**新修订行 + 新清单 digest** 落盘并留痕；原地改既有行即 RED；
  白名单扩展同构走只追加修订行。未注册 digest 冒充冻结即 RED。
- L4 白名单外资产行、按类登记缺字段资产行不得进入清单（M131 门序硬约
  束：先许可登记，后清单行）；清单行集与许可/加载登记不对账即 RED
  （M133 判据逐字承接）。
- L5 **ready 场景数下界**：G10.3 门验收时 `ready` 场景数 ≥ 首发清单基数
  （2）；缺额行必须 `not-ready` + `DEV_ENV_DEGRADE` 显式登记且不充绿。空
  清单或全 `not-ready` 清单的「逐场景」判定为 vacuous truth，一律不构
  PASS（M132/G-G10-5 同口径）。
- L6 **M129 场景集时序**：G10.2 验收 M129 时本清单尚不存在，M129 判据
  「场景清单」指 G10.2 期暂定清单形态（首发清单草案两行最小场景面）；
  本清单冻结后须对 M129 证据做回归复核（冻结清单 ⊇ 暂定场景面；缺行即
  登记漂移并按只追加程序处理）。禁以任意临时场景集冒充「场景清单」混过
  M129（RFC-0027 §4.4 F8 修法）。

**Implementation Requirements**

- IR1 清单修订行字段：`revision`（单调递增整数）+ `manifest_digest`（该
  修订全量行集 canonical digest）+ `changed_at` + `change_note`；首修订
  = 初冻结。
- IR2 本条款挂接 `g10.p1.m133.corpus_list_freeze`（G10.3 波实现）；测试
  锚定 = conformance/external_reference/ 语料 + M133 门脚本 RED 臂（原地
  改 / 未注册 digest 冒充 / 行集不对账 / ready 下界不满足 vacuous 拦截）。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-08-15 | G10.3 spec-first 初版：RXS-0380（XR-HARNESS UE 外部进程编排边界与出图命令面闭集）/ RXS-0381（XR-LICENSE 许可白名单 SPDX 闭集与按类登记）/ RXS-0382（XR-CACHE 外部缓存布局、git 零二进制与 digest 核验）/ RXS-0383（XR-MANIFEST 场景清单冻结、清单 digest 注册与只追加修订）四条款体落地，RFC-0027（Agent Approved 2026-08-15）§4.1~§4.4/§5 逐字承接；条款号自 ledger 实测 `RXS.next_free=380` 顺位领取（0380~0383 连续不跳号，0295/0296 burned 与 shadow_reserved 181~184 维持）；零新 RX 码/RD/U/SG；conformance 最小锚定语料十四件（conformance/external_reference/ accept 四件 + reject 十件，inert + `//@ spec` 锚定 + 预期 RED 注释 + 转正路径旁注，G9.2~G9.6 spec 波先例）同 PR 落；symbolic key `g10.p0.m131/m132.*` 与 `g10.p1.m133.*`（G10.1 冻结字面）0-byte 不动；trace_matrix 重生成 CRLF 字节纪律维持。既有 spec 条款字面 0-byte（只追加新卷/登记行），不触红线/禁区 | **Full RFC**（RFC-0027） |
