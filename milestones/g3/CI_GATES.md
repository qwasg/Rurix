# G3 CI 门禁增量

> 所属契约:[G3_CONTRACT.md](G3_CONTRACT.md)
> 版本:v1.0(2026-07-18)
> 基线:[../m0/CI_GATES.md](../m0/CI_GATES.md) ~ [../ea1/CI_GATES.md](../ea1/CI_GATES.md)(全部沿用:runner 约定、PR Smoke 1–60 步、Release 层门禁、guardrail 全部激活项、nightly 全量回归冻结);本文只规定 G3 期的**增量**。
> 铁律不变:任何新增门禁必须在真实 PR 上以真实失败/通过路径验证过(反 YAML-only)。
> 开工脚手架口径:本文 G3 增量步骤(预期 61~67,DXIL 腿视 probe ±68/69,步骤 70 集成 showcase 视判档)为**各面计划项**,开工**不**写入 workflow YAML 真实步骤(随实现 PR 落地回填,对齐 M8~EA1 计划→回填范式)。**G3 开工脚手架零 CI 代码改动**:预算 glob 已泛化自动纳入 `g3_budget.json`;`check_closed_contracts` glob 与无参默认基准均已就位;**counter/entries 不预造**(登记与 `ci/budget_eval.py` evaluator 分支同实现 PR 落,未知 id 强制 FAIL)。

---

## 1. Runner

沿用 M0 §1(自托管 RTX 4070 Ti 开发机)~ EA1 §1。G3 新增 runner 预置项(工具件不入库,provisioning 注归 close-out §8):

- **spirv-cross / dxc / dxv / spirv-val**:B 链与 SPIR-V 校验既有预置沿用(G2/MB1 已就位);DXIL mesh probe 需 dxc 支持 `-T ms_6_5`(版本核对随 probe 落证据)。
- **Vulkan 驱动扩展探测**:`VK_EXT_mesh_shader` / `VK_KHR_ray_tracing_pipeline` / `VK_KHR_acceleration_structure` / descriptor indexing——运行期 feature chain 探测,缺失 = 确定性 SKIP/Err 非 fake pass(RXS-0212 三态先例);本机 RTX 4070 Ti 两扩展均在。
- **spike 专用**:无新预置(CUDA 工具链/compute-sanitizer/Nsight 已在位);spike 实验**不进 CI**(操作者工具,uc07_bench「不进 CI」先例),nightly 零新增毒径步骤。

## 2. PR Smoke 追加步骤(计划项,编号接 EA1 §2 的 60;落地随各面实现 PR 回填 workflow)

| # | 步骤 | 失败即红 |
|---|---|---|
| 61 | UC-04 窗口 present 冒烟(G-G3-2 通道;G3.2 落地,RFC-0013 前置后):`ci/uc04_present_smoke.py` —— host 段恒跑(present 装配核验单测 + typestate 编译面);device 段:窗口 present N 帧逐帧成功 + backbuffer readback 像素断言(与步骤 48 同判据)+ ResizeBuffers 重建后再 readback;RED:篡改 PRESENT 态迁移 barrier → debug layer 报错翻红;无显示环境 SKIP(dev-env degrade)+ `RURIX_REQUIRE_REAL=1` 翻硬红;**步骤 48 offscreen 硬门 0-byte 不动**;内建 red_self_test;写 evidence JSON(schema 校验) | 是 |
| 62 | 采样超集 codegen/host 冒烟(G-G3-3 通道;G3.3 落地,RFC-0013·采样超集章前置后):全模式语料 B 链(spirv-cross→dxc→**dxv 全过**+签名门)+ SPIR-V `spirv-val` 三态 gate;reject 通道 UI golden(RX3014/RX6023 扩类别);既有显式 LOD 0 路零回归 | 是 |
| 63 | 采样超集 device 冒烟(同面 device 腿):≥6 模式数值判据(隐式 LOD/lod/grad/fetch/sampler 状态对照/shadow/gather/UAV 回读)+ mip 金字塔逐层异色 + wrap-vs-clamp 像素对照 + 双后端(D3D12/Vulkan)同语义源数值一致性;篡改→像素变 RED;REQUIRE_REAL | 是 |
| 64 | bindless 冒烟(G-G3-4 通道;G3.4 落地,RFC-0013·bindless 章前置后):host(推导单测:Unbounded 合法化+独占 space 分配律+RTS0 roundtrip;nonuniform 缺失 reject UI golden)+ device(四纹理四象限动态索引采样==四色;篡改注册序→换位 RED;feature 缺失→确定性 Err) | 是 |
| 65 | render graph 冒烟(G-G3-5 通道;G3.5 落地,RFC-0013·render graph 章前置后):host 互证恒跑(uc04 三 pass 图自动推导 barrier 集 == RXS-0169 手动锚点集;环/冲突/未声明访问 reject)+ device(uc04 迁 Graph API 重跑步骤 48 同判据;漏声明 read → strict 拒 RED;Vulkan 同图同判据) | 是 |
| 66 | Vulkan mesh/task 冒烟(G-G3-6 通道;G3.6 落地,RFC-0013·mesh-task-RT 章前置后):spirv-val(vulkan1.2/spv1.4 三态)+ device mesh 程序化网格 offscreen 像素判据(covered 计数)+ 篡改 SetMeshOutputs 顶点数 RED;扩展缺失 SKIP 非 fake + REQUIRE_REAL 硬红 | 是 |
| 67 | Vulkan RT 冒烟(同面 RT 腿):device 单三角形 BLAS/TLAS raygen/miss/closesthit 命中-miss 双色断言 + 移动顶点→命中区域移动 RED(数据流红绿);VVL 崩溃与驱动崩溃以退出码区分(反 grep) | 是 |
| (68) | DXIL mesh 冒烟(**视 probe**:probe 绿则落——B 链 ms_6_5/as_6_5 dxv 全过 + DispatchMesh device 像素判据 + RX6008 改接见证;probe 红则本步骤不落,mesh DXIL 与 RT 同入 RD-034+ 尾门) | 视落地 |
| (69) | DXIL RT blocked 探针(预判 blocked 时落:上游能力探测脚本——spirv-cross RT 转译能力/A 路签名缺口逐项探测,能转译则翻活提示升级,否则输出 BLOCKED 见证防静默腐烂;对齐 RD-011/RD-015 跟踪纪律;**非 fake pass:BLOCKED 见证 = 预期绿,能力出现未跟进 = 红**) | 视落地 |
| (70) | 集成 showcase(视判档:五面同台 demo——bindless 材质+采样超集+graph+mesh/RT pass+窗口 present;落与不落随 G3.6 后判档,不预占) | 视落地 |

预算 evaluator 自动合并加载 [g3_budget.json](g3_budget.json)(命名空间冲突即红;**开工全空,counter 登记与 evaluator 分支随各面实现 PR 同落**)。**G3 close-out 必须跑 `--strict` 且全局零 estimated 残留**(14 §3;EA1×G3 双活跃互斥保险 = 双方零 estimated 铁律)。

## 3. Release 层门禁

- 既有全部 hard-block 门 **0-byte 沿用**;触发器维持收窄;`g3-closed` tag 不匹配触发器零误触发。
- G3 无 Release 层新增(五面均为编译器/运行时/CI 面,不触发布链路;bundle 组件面不变)。

## 4. Nightly 追加

- 既有 nightly 全保留。**G3 无新增 nightly 项**:五面冒烟归 PR smoke 步骤 61~67±(秒~分级);**spike 毒径实验绝不进 nightly/CI**(挂起风险,操作者工具纪律)。
- (信息性)nightly 现红 = compute-sanitizer attach flake(async_buffer 步),与 G3 无涉且 budget 判据已满足;挂起 nightly run 即 `gh run cancel` 释放串行队列;根治不入 G3 验收面(EA1 契约外轨道口径沿用)。

## 5. Guardrail

沿用 M0~EA1 全部激活项。G3 期动作:

1. **基准 ref 默认 `mb1-closed`**(EA1 active 未切;PR 路径以 `GITHUB_BASE_REF` 为准;若 EA1 先收口默认自动为 `ea1-closed`,G3 全措辞兼容)。G3.7 close-out 切至 `g3-closed`(agent 自主签署)+ 双基准 advisory 复核。
2. **stable 快照 bless**:G3 预期多次加性触发(RXS-0220+ 条款增长 215→N);各与条款/实现同 PR 重 bless + bless_log 同 diff 追加(数据行忌「日期」子串);不可分 PR(步骤 49 硬红)。
3. **错误码**:codegen 新码自 **RX6027** 续号;**RX6008 = RD-012 预留码本期正式改接**(唯一预留已存在码;RX6009 burned 不用);工具类确需自 RX7023;en/zh 成对(bilingual 96→N)。
4. **unsafe 边界**:新 unsafe 全部 `// SAFETY:` + unsafe-audit **U30 起**续号(U29=EA1 预留显式跳让);vk.rs mesh/RT FFI 扩展沿 U26/U27 审计模式;单块单操作。
5. **既有零回归不变量**:dxil 套件(404+ 恒定)/ vulkan 套件 grow-only / 步骤 41/48/54~58 既有判据 0-byte 只增;B 链 dxv validator+签名门(RX6011/6012)不可裁剪;**SPIR-V 1.4 分叉不动 1.0 路径**(dxil 腿零回归门)。
6. **spike 纪律(新增激活项,随 G3.1 进核对面)**:探针标 `// SPIKE(RD-027)` 不入 src/ 不随产品编译;全 GPU 运行经 proc_guard(禁裸 subprocess);evidence 只增;上游备包 DRAFT — do NOT file 强制。
7. **规划文档冻结**:00–14 执行 PR 0-byte;开工裁决记 G3_CONTRACT §7;MS1 §7 旧文「g 系无 G3」由 §7 ① 命名裁决覆盖,11 号勘误(如需)走独立 errata PR。
8. **trace 矩阵扫描面**:新条款 RXS-0220+ 锚定随实现 PR 同落,`--check` 全程全锚定(215→N/N)。
9. **LF byte-exact**:新文件 LF+尾换行;禁 Python 文本模式写文件;提交前逐文件字节核 CR+尾字节。

14 §2 常驻集其余项的 G3 期评估结论:

| 项 | 结论 |
|---|---|
| stable API 快照 | 已激活;G3 预期 5~8 次加性重 bless(五面条款增长) |
| MIR/PTX/DXIL/UI golden | 已激活;RD-027 修复若触 device_codegen → tests/ptx IR golden 重 bless + bless_log(check_ptx_bless);五面 UI/DXIL golden 随各面增长 |
| unsafe-audit 完整性 | 已激活;U30+ 随 vk mesh/RT FFI 落 |
| Compute Sanitizer / NVIDIA 白名单 | 已激活维持;spike memcheck 为操作者取证非 CI 步骤;白名单审计 0-byte 沿用 |
| 多后端(D-008/SG-003) | SG-003 维持 triggered(RFC-0011)不回翻;G-MB1-6 AMD 尾门独立于 G3(硬件 gated,G3 device 门全锚定本机 NVIDIA) |
| registry sumdb(D-312/SG-007) | 维持 not_triggered(G3 零网络面) |
| MLIR(SG-001)/ Tensor Core(SG-002)/ autodiff·fusion(SG-004/005)/ Python 嵌入(SG-008)/ 自举(SG-009) | 维持 not_triggered;SG-010 软保留维持(present 面 D-130 红线使窗口/UI 方向不触) |
| 贡献校验门(ci/check_contribution.py) | 已激活延续;**G3 五 Full RFC 全部触发规则 4(对抗性评审 provenance),合入前本地验绿** |

## 6. 验证程序(对应契约 G-G3-1~G-G3-9 与计划步骤 61~67±)

1. G3.0:治理包合入序核验(G3 脚手架先合、EI1 随后);host 门全绿;零 CI 代码改动核验。
2. G3.1:spike 全程 proc_guard 见证(evidence JSONL 逐 run)+ 归因 evidence JSON 过 check_schemas + 报告合入 = 开闸;处置尾项按路径核验(修复:golden 重 bless + 256/4 升回 + ms1.bench 回填;备包:DRAFT 标头 + `<FILL>` 清零)。
3. G3.2~G3.6 各面:RFC Approved 合入先于实现 PR(失败测试先行:对应步骤脚本在 RFC 合入时点 main 不存在);条款先行 commit 序;本机 `py -3 ci/<步骤脚本>.py` 红绿双证 + runner run URL 归档;trace/快照/bilingual/guardrails 全绿。
4. G3.7:`budget_eval --strict` 输出原文(全局零 estimated)+ G-G3-1~9 留痕指针 + 九条 RD 处置 + SG 复评 + 双基准 advisory 复核输出 + 全冒烟 REQUIRE_REAL 真跑记录。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-07-18 | 初版(G3 契约配套;计划步骤 61~67 为五面计划项+68/69/70 条件项,落地时回填 workflow YAML 实测命令与 run URL;Release/Nightly 零增量(spike 不进 CI);guardrail 动作:基准兼容两序、快照 5~8 次加性重 bless、RX6027 续号+RX6008 改接、U30 起跳让 U29、SPIR-V 1.4 分叉零回归门、spike 纪律新增激活项、D-409 规则 4 全程;runner 预置增量 = Vulkan 扩展探测(缺失确定性 SKIP)与 dxc ms_6_5 版本核对)。**G3 开工脚手架零 CI 代码改动**:g3_budget.json 经 glob 自动纳入,counter/entries 不预造;开工不写入 workflow YAML 真实步骤 |
| v1.1 | 2026-07-18 | 编号更正对齐(契约 §7 v1.1):步骤 62/64/65/66 前置措辞自「RFC-0014/0015/0016/0017」改为「RFC-0013 对应面章节」——五面共用单伞形 Full RFC-0013(五面五章,MB1 RFC-0011 先例;RFC-0014 = EI1 earmark)。步骤号分配确认:G3 = 61~70、EI1 = 71~75(owner 双轨分配);本文步骤面 0-byte 无实质变更 |
