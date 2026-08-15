<!-- Assisted-by: Kimi-K3（G10.1 治理波起草） -->
# G10_PLAN — UE5 画面对标基线期 主线分解

> **状态**：**计划起草稿（v1.0）——G10 未立项**。本文以 [G9_PLAN.md](../g9/G9_PLAN.md) v1.1 为波次结构/退出门/风险表范式起草；本文与 G10.1 治理波全部产物均不构成任何契约、验收承诺或编号 claim。G10 正式立项（用户指令 + 治理裁决 + §5 治理门）前，零 `src/`/`spec/`/`conformance/` 语义实现、零编号消费。
> **蓝本与上游**：[G9_PLAN](../g9/G9_PLAN.md) v1.1（范式模板）· [G9_CAPABILITY_MATRIX](../g9/G9_CAPABILITY_MATRIX.md) v1.0（能力矩阵范式）· [G9_P2_DECISIONS](../g9/G9_P2_DECISIONS.md) v1.0（十项 defer-to-G10+ 承接锚，法定输入）· [G9 CI_GATES](../g9/CI_GATES.md) v1.21（步骤编号已至 172；数字步骤纪律 = `post-interlock actual-next-free allocation`）· [G10_CAPABILITY_MATRIX.md](G10_CAPABILITY_MATRIX.md)（能力缺口矩阵，M128 起顺延 G9）· [G10_CANDIDATE_DECISIONS.md](G10_CANDIDATE_DECISIONS.md) v1.0（候选决策表起草稿：十锚初裁全 defer-to-G11+ + 新增候选五行）· [design/g10_ue5_harness_spike.md](design/g10_ue5_harness_spike.md) v1.0（G10.1 治理波 UE5 出图环境 spike 只读探测成果与裁决建议）· [渲染器调研](../../渲染器调研/) 七份报告（2026-07-28 快照）· [14_ENGINEERING_DISCIPLINE](../../14_ENGINEERING_DISCIPLINE.md) §5（证据分级：measured_local 优先，estimated 占位不得超 2 期——P-09）。
> **推进形态**：**严格波次**——G10.1 治理波 → G10.2 UE5 出图环境波 → G10.3 压测语料波 → G10.4 度量基建波 → G10.5 首轮 A/B 对比波 → G10.6 defer 重评窗波 → G10.7 P2 穷举 → G10.8a soak → G10.8b close-out。波次内可蜂群并行，波次间串行；spec-first + RED 先行；禁止 stub/mock/host substitution 抢跑。
> **本波边界（G10.1，governance-only）**：本波仅落 PLAN / 矩阵 / 契约 / 候选决策 / 验收映射 / RFC / CI_GATES 治理面，作为独立纯文档提交——零编号、零 registry、零 `spec/src/conformance` 改动、零 CI 步骤 materialize（验收门只写判据草案；数字 CI 步骤一律 `post-interlock actual-next-free allocation`，禁预占）。

---

## 1. 目标与口径

**G10 = UE5 画面对标基线期**。G10~G15 总体分期（立项裁决口径）：**G10 = 画面对标基线期**（UE5 出图环境 + 压测语料 + 度量基建 + 首轮 A/B 对比 + 差距清单）→ **G11 = 画质修复期** → **G12 = 路径追踪生产化期** → **G13 = 超分与 DLSS 期** → **G14 = 性能优化期** → **G15 = 商用收口期**。用户指令总目标（彻底对标 UE5 渲染器画质；支持 DLSS/超分采样/路径追踪前沿技术；严格画面审查〔完整渲染帧与 UE5 出图对比，修复细节〕；优化管线使帧率对标 UE5 略高不降级画质；最终交付真实可商用）由 G10~G15 六期合流兑现；**G10 只交基线与差距清单，不承诺修复**——画质修复归 G11、路径追踪生产化实施归 G12、DLSS/超分接入实施归 G13、性能优化实施归 G14、商用收口归 G15。

**「UE5 级」可核对基线 = UE 5.8**（沿用 G9 口径，[G9_PLAN](../g9/G9_PLAN.md) §0；本地源码 = `E:\Kimi_Agent_Taichi Engine 优化计划\references\UnrealEngine`，ue5-main @4517329fa，含 `Engine\Source\Runtime\Renderer` 完整源码树——Nanite/Lumen/MegaLights/VirtualShadowMaps/HairStrands/PostProcess/PathTracing 等模块路径经实测在树，见 [G10_CAPABILITY_MATRIX](G10_CAPABILITY_MATRIX.md) §0.5；无 Content、无编译产物、不可直接出图；本机无 UE5 编辑器二进制）。验收五层级沿用 G9/G8 口径：**核心等价、功能闭环、可降级、可生产化、Vulkan 主线**。

**成功判据（草案；G10.1 治理波硬化为契约验收门，本波不 materialize CI 步骤）**：

1. **UE5 出图环境可用**：spike 裁决路径落地，固定场景 UE 5.8 侧批量出参考帧，同参数双跑帧 digest 一致，provenance 登记闭集（矩阵 M128/M129）。
2. **压测语料就绪**：Bistro/Sponza/CornellBox 等场景清单落盘，逐资产许可登记零缺行（商用可再分发白名单，G10.1 裁决冻结），Rurix 加载门全绿（M131/M132；清单冻结 M133）。
3. **度量基建就绪**：HDR 帧捕获管线往返无损；FLIP/SSIM/PSNR 与参考实现逐图对拍一致（容差一律 measured 标定，禁手写阈值）；逐像素 diff 报告落盘（M134~M137；标定程序 M138）。
4. **首轮 A/B 对比完成**：压测场景全集同场景同相机同光照双端出图，度量报告 + 差距清单 measured 落盘零空行，每差距项带 UE5 Renderer 模块归属 + G11 承接锚（M139/M140）；双端帧率对标基线采样存档（M141——「帧率对标 UE5 略高不降级画质」是 G14 目标，G10 只建基线数据，不设帧率通过线）。
5. **defer 重评窗完成**：G9 十项 defer 逐行重判零空行（§2 G10.6）；G10 期新产生 P2/留档经 G10.7 穷举，defer 必有承接锚。
6. **measured 纪律**：`g10_budget.json` 非空 measured_local、零 estimated（P-09）；全部度量阈值实测标定；环境画像（驱动/锁频/WDDM-HAGS/TDR）随证据存档（14 §5）。
7. **零修复纪律**：G10 全域不提交任何画质修复 PR；差距清单只登记不修复，修复面由 G11 立项承接（§6 边界）。

**out-of-scope**：

| 项 | 依据 |
|---|---|
| 画质修复实施（含 GI/阴影/材质/时域任一差距项的修复 PR） | G11 承接；本计划 §1 口径 |
| 路径追踪生产化实施 | G12 承接；G10 仅产 M96→生产化差距档案（矩阵 M143） |
| DLSS/超分接入实施、NRD 接入 | G13 承接；G10 仅复核 UpscaleBackend 接入面档案（矩阵 M142）；RD040-nrd 重判条件字面见 [G9_P2_DECISIONS](../g9/G9_P2_DECISIONS.md) §1 |
| 性能优化实施 | G14 承接；G10 只建双端帧率基线（M141） |
| UE 源码/二进制 vendoring 进 rurix 仓库 | 许可边界；UE 仅作外部参照出图端，条款核验**待定（G10.1 治理波裁决）** |
| 任何编号（RXS/RD/U/RX/CI step/RFC）推测性消费 | 立项时按实测 `next_free` 领取；数字 CI 步骤一律 `post-interlock actual-next-free allocation`（[G9 CI_GATES](../g9/CI_GATES.md) §1.2 纪律继承） |

---

## 2. 波次分解

```text
G10.1 治理波（本波，governance-only：契约四件套 + 候选决策 + 验收映射 + RFC + measured baseline + validator）
  → G10.2 UE5 出图环境波（spike 裁决出图路径；目标 = 能批量出 UE5 5.8 参考帧）
  → G10.3 压测语料波（联网获取场景/材质资产 + 许可登记 + 场景清单 + 加载门）
  → G10.4 度量基建波（帧捕获 EXR/HDR 管线 + FLIP/SSIM/PSNR + 逐像素 diff + evidence schema + 阈值标定）
  → G10.5 首轮 A/B 对比波（同场景同相机同光照双端出图 + 度量报告 + 差距清单 measured 落盘 + 帧率基线）
  → G10.6 defer 重评窗波（G9 十项 defer 逐行重判，G10.5 measured 数据为法定证据输入）
  → G10.7 P2 穷举决策（G10 期新产生分项，defer 必有承接锚）
  → G10.8a soak → G10.8b close-out（差距清单终审锁定 → G11 法定输入）
```

**单点依赖声明**：G10.2 是全部后续波的硬前置——UE5 出图环境不可用则 G10.3~G10.5 的「双端」无从谈起；出图路径最终裁决归 G10.1 治理门（§5 表项 2，G10.1 spike 报告 [design/g10_ue5_harness_spike.md](design/g10_ue5_harness_spike.md) 为裁决输入）；G10.2 按裁决路径执行建设，spike 标注的全部「实现波待验证」项首日实测登记。G10.3 与 G10.2 可部分并行（语料获取不依赖 UE 出图），但 A/B 面必须待 G10.2 退出门绿后开放。

### G10.1 — 治理波（governance-only，与 G9.1 同构）

交付：

1. **用户立项指令留痕 + agent 治理裁决**；§5 治理裁决表项全部落定；G10 文档集（本计划 + 矩阵）不可变 ref 登记。
2. **`G10_CONTRACT.md`**：契约四要素 + front matter 状态机（`implementation_status: blocked` 起始）+ §8 只追加条款结构。
3. **`G10_CANDIDATE_DECISIONS.md`**：G9 十项 defer 承接登记（逐行引 [G9_P2_DECISIONS](../g9/G9_P2_DECISIONS.md) §3 承接锚字面）+ G10 新增候选项（矩阵 §1~§5 全部行）；维护 **G9 defer → G10 M##/重评窗 → 波次 → 退出门 → 最终状态** 总表，缺行不得开工 G10.2。
4. **`G10_ACCEPTANCE_MAP.md`**：全部 P0（及 go 的 P1）的 `M## → CI step → evidence schema → 判据`；缺行阻断 G10.2。
5. **伞形 Full RFC**（实际编号按立项时 `registry/number_ledger.json` 实测 `next_free` 领取，禁止推测号），建议两份：
   - 画面对标与度量语义 RFC：帧捕获 HDR 格式面 / FLIP/SSIM/PSNR 口径冻结 / 逐像素 diff 报告 schema / 差距清单 schema / 双端确定性契约（相机/光照/时间参数）；
   - 外部参照 harness 与许可边界 RFC：UE 出图编排边界（外部进程、零 vendoring）/ 压测资产许可登记面（白名单/SPDX/attribution/资产 digest）。
6. **RTX 4070 Ti measured baseline → `g10_budget.json` 非空**（P-09：无 measured baseline 不得设性能硬门；度量标定阈值全部实测回填禁手写）。
7. **`CI_GATES`** + G10 专属 validator（acceptance map 三向比对、implementation interlock、budget baseline 检查、决策表承接锚机核——脚本模式照 G9 五件套；数字 CI 步骤一律 `post-interlock actual-next-free allocation`，互锁后实测领取）。

退出门（判据草案，防假绿）：§5 治理裁决表项全落定并留痕；四件套齐备且互锁 validator 诚实输出 BLOCKED→READY；候选决策表与验收映射无缺行；零数字 CI 步骤 materialize；registry/spec/src/conformance 0-byte。

### G10.2 — UE5 出图环境波

| 面 | 内容 | 矩阵 |
|---|---|---|
| 路径执行 | 按 G10.1 治理门裁决路径建设出图环境（spike 裁决建议 = ②Launcher 安装 UE 5.8 正式版首选、①源码编译降为增强备选、③公开参考图仅兜底不进验收链——最终选择以治理裁决为准，[design/g10_ue5_harness_spike.md](design/g10_ue5_harness_spike.md) 问题 5）；spike「实现波待验证清单」（依赖下载实积/时长、出图命令形态可用性等）首日实测登记，两臂诚实登记 | M128 |
| 出图 harness | 固定场景 + 固定相机 + 固定光照 → UE 5.8 侧批量出 HDR 参考帧（spike 候选臂：MRQ 批量臂推荐主路 / HighResShot 快速臂 / Python 编排臂，选臂依据实测登记）；环境画像（UE build digest/驱动/锁频）随证据存档 | M128/M129 |
| 确定性契约 | 相机/光照/时间参数同 schema 双端各一份 + digest 比对（Rurix 侧消费面骨架） | M130 |

退出门（判据草案）：`g10.p0.m128` / `g10.p0.m129` 门绿——参考帧批出成功 + 同参数双跑帧 digest 一致 + provenance（场景/相机/光照/build）登记闭集零缺行；M130 骨架期 digest 比对面就位（双端核验归 G10.5）。**若裁决路径实测不可行：回退 spike 备选臂并契约 §8 只追加修订本波判据，禁以截图/人工采集帧冒充 harness 出帧。**

### G10.3 — 压测语料波

| 面 | 内容 | 矩阵 |
|---|---|---|
| 资产获取 | 联网获取 Bistro/Sponza/CornellBox 等 glTF/FBX 压测场景与材质资产（现状：仓库仅 fixture 生成器 `ci/_gen_m81_gltf_fixtures.py`，零真实压测资产）；逐资产许可核验——商用可再分发白名单（CC0/CC-BY 族，名单 G10.1 裁决冻结）+ SPDX id + 来源 URL + attribution + 资产 digest 登记 | M131 |
| 加载门 | 场景清单每场景 Rurix 加载成功 + 三角形/材质/纹理计数非空 + 加载事件序列 golden | M132 |
| 清单冻结 | 场景清单版本化冻结（清单 digest 注册；后续变更只追加修订行） | M133 |

退出门（判据草案）：`g10.p0.m131` / `g10.p0.m132` 门绿——许可登记零缺行、白名单外许可注入即 RED；清单全场景加载绿、静默丢场景即 RED；M133 清单 digest 在树。

### G10.4 — 度量基建波

| 面 | 内容 | 矩阵 |
|---|---|---|
| 帧捕获 | Rurix 帧捕获 HDR 管线（EXR 或等价格式，格式裁决 G10.1）落盘 + 捕获→回读逐像素往返无损 + 分辨率/色彩空间元数据齐备 | M134 |
| 图像度量 | FLIP 自实现与参考实现逐图对拍一致（容差 measured 标定；参考实现版本 pin）；SSIM/PSNR 口径冻结进 spec + 参考对拍；恒等图对极值断言（FLIP=0 / SSIM=1 / PSNR=inf） | M135/M136 |
| diff 报告 | 逐像素 diff 热区图 + 逐区域统计落盘（阈值 measured 标定）+ evidence schema 闭集 | M137 |
| 阈值标定 | 度量阈值标定程序可复跑 + 标定值入 `g10_budget.json`（measured_local，禁手写） | M138 |

退出门（判据草案）：`g10.p0.m134`~`g10.p0.m137` 四门绿——位深截断/sRGB 混标注入即 RED；参考输出扰动注入即 RED；diff 图与标量报告不一致注入即 RED；M138 标定值 provenance 齐备（P-09）。

### G10.5 — 首轮 A/B 对比波

| 面 | 内容 | 矩阵 |
|---|---|---|
| A/B 出图 | 压测场景全集同场景同相机同光照 Rurix vs UE5 双端出图（M130 双端 digest 核验前置，不等不得出报告） | M139 |
| 度量与差距清单 | 逐场景逐指标度量报告 + 逐像素 diff + 差距清单落盘；每差距项带 UE5 Renderer 模块归属（模块路径枚举闭集，矩阵 §4）+ measured delta + 建议 P 级 + G11 承接锚 | M139/M140 |
| 帧率基线 | 双端同场景帧率采样（协议沿 14 §5：L0 环境验证 → warmup/稳态 → trimmed mean → IQR）+ 环境画像随证据存档 + 双端交替采样顺序登记 | M141 |

退出门（判据草案）：`g10.p0.m139` / `g10.p0.m140` / `g10.p0.m141` 门绿——差距清单场景全集零空行；单端缺帧聚合不得 PASS（不遮蔽）；非 measured 叙述充差距即 RED；未锁频/环境画像缺字段即 RED。**G10 不设画质通过阈值与帧率通过线——差距全量登记即绿，修复归 G11。**

### G10.6 — defer 重评窗波

G9 十项 defer-to-G10+（[G9_P2_DECISIONS](../g9/G9_P2_DECISIONS.md) §1/§3）逐行重判：M61 mesh shader · M52 SER · M99-clipmap 世界辐射缓存 · M100-high ReSTIR 高档 · SAFE-GPU · M127 神经变形 · M98-l4 Far Field · M114-strand 毛发精确 OIT · M118-hdr-cal HDR 标定 · M125-adopt3 Jolt5.6 三件。**G10.1 候选决策表已初裁十锚全 defer-to-G11+**（其中 M99-clipmap/M100-high/M98-l4 三锚登记「G10 触发评估」——G10 A/B 对比与大世界压测语料是其 measured 举证的法定产出通道，[G10_CANDIDATE_DECISIONS](G10_CANDIDATE_DECISIONS.md) §1/§4）；本波 = **窗口核验**：以 G10.5 差距清单 measured 数据为法定证据输入，逐行核验重判条件是否命中——命中者按只追加程序重判 go 并指定承接波次（实现类分项由 G11+ 承接，G10 零实现面），未命中者维持 defer 且承接锚字面 0-byte 维持（沿用 G9.7 原文，修订只追加留痕）；deferred history 只追加，禁静默改判（G8.7/G9.7 纪律继承）。

退出门（判据草案）：十行重判核验表零空行；每行「重判条件核验结果 + 裁决 + 承接锚」全列非空；机核进 G10 validator（同构 `ci/g9_p2_decisions_check.py` 模式）。

### G10.7 — P2 穷举决策（软门，必须穷尽）

对 G10 期内新产生的 P2/留档/未触发分项逐条 go/no-go/defer-to-G11+（不得遗漏；defer 必有承接锚，机核同构 `ci/g9_p2_decisions_check.py`）。候选行集 = G10.1 决策表校准后冻结 + G10.2~G10.6 期内新增分项。软门失败/未触发 → 诚实登记不阻塞 G10.8a；close-out 审计要求本表无空行。

### G10.8a — Stabilization / soak（close 前必经）

- 全 P0 硬门回归 + 已 go 的 P1 回归；G5~G9 既有判据 0-byte。
- 代表性场景集全量双端出图 + 度量回归 soak：出图 harness/捕获/度量/差距清单全链路连续复跑；soak 量级口径**沿 G9.8a 继承或 measured 证明更短足够**（具体阈值 G10.1 裁决 measured 标定，本计划不写死）。
- `budget_eval --strict` 非空、零 estimated/skip。
- 条件实现刚绿**不得**当日进 8b；**G9.8b 同日放行先例是否继承属治理裁决（§5 表项 8）**。

### G10.8b — Close-out

- 验收映射终审；RD 分项最终状态与 G10.1/G10.6/G10.7 决策表逐字一致。
- **差距清单终审锁定 → G11 法定输入**（G11 修复范围只能消费本清单 + 其承接锚，不得在 G11 内无锚新立修复项）。
- 契约 §8 只追加 + status flip。

### 2.9 P0 → 硬门覆盖（判据草案；G10.1 固化为 ACCEPTANCE_MAP 时方可 materialize CI 步骤）

任一 P0 无独立硬门 → **禁止** G10.8b status flip。数字 CI 步骤一律 `post-interlock actual-next-free allocation`（[G9 CI_GATES](../g9/CI_GATES.md) 已至步骤 172，v1.21；G10 编号自互锁后实测 `next_free` 顺位领取，禁预占）。

---

## 3. P0 建议清单（12 行，M128 起顺延 G9 矩阵 M90~M127；G10.1 决策表重裁后硬化）

| P0 | 名称 | 独立硬判据（草案） | 负例 RED 臂要求 | device/host 性质 | 最晚波次 |
|---|---|---|---|---|---|
| M128 | UE5 出图环境可用门 | spike 裁决路径落地 + 固定场景 UE 5.8 侧出帧成功 + 环境画像（UE build digest/驱动/锁频）随证据存档 | 出帧进程非零退出冒充成功即 RED；环境画像缺字段即 RED；预置假帧冒充真出帧即 RED | host 编排 + device（UE 侧 GPU 渲染） | G10.2 |
| M129 | UE 参考帧批量出图与 provenance 库门 | 场景清单逐场景参考帧落盘 + 同参数双跑帧 digest 一致 + provenance（场景/相机/光照/build）登记闭集 | 双跑 digest 不等即 RED；provenance 缺行即 RED；帧文件篡改检测 RED | host+device | G10.2 |
| M130 | 双端场景确定性门 | 相机/光照/时间参数同 schema 双端各一份 + digest 比对相等 + 双端解析读入一致 | 单端参数漂移注入即 RED；schema 外字段注入即 RED | host 纯 host | G10.2 骨架 → G10.5 双端核验 |
| M131 | 压测资产许可登记门 | 逐资产 license 白名单闭集（CC0/CC-BY 族，名单 G10.1 裁决冻结）+ SPDX id + 来源 URL + attribution 文本 + 资产 digest | 未登记资产混入清单即 RED；白名单外许可注入即 RED；URL/digest 缺字段即 RED | host 纯 host | G10.3 |
| M132 | 压测语料加载门 | 场景清单逐场景 Rurix 加载成功 + 三角形/材质/纹理计数非空 + 加载事件序列 golden | 计数为零冒充成功即 RED；场景静默丢弃即 RED | host+device | G10.3 |
| M134 | 帧捕获管线门 | Rurix 帧捕获 HDR（EXR 或等价，格式 G10.1 裁决）落盘 + 捕获→回读逐像素往返无损 + 分辨率/色彩空间元数据齐备 | 位深截断（8bit clamp）注入即 RED；sRGB/线性混标注入即 RED；元数据缺字段即 RED | host+device | G10.4 |
| M135 | FLIP 度量门 | 自实现与参考实现在同一测试图对上逐图输出一致（容差 measured 标定）+ 恒等图对 FLIP=0 极值断言 + 参考实现版本 pin | 参考输出扰动注入即 RED；恒等图对非零即 RED；口径参数漂移注入即 RED | host 纯 host | G10.4 |
| M136 | SSIM/PSNR 度量门 | 口径冻结进 spec + 与参考实现逐图对拍一致（容差 measured 标定）+ 恒等图对 SSIM=1/PSNR=inf 极值断言 | 参考输出扰动注入即 RED；恒等图对非极值即 RED；口径漂移注入即 RED | host 纯 host | G10.4 |
| M137 | 逐像素 diff 报告门 | diff 热区图 + 逐区域统计（超阈像素计数/分布，阈值 measured 标定）落盘 + evidence schema 闭集 | diff 图与标量报告不一致注入即 RED；空场景行即 RED | host 纯 host | G10.4 |
| M139 | A/B 对比门 | 场景全集同场景同相机同光照双端出图 + 度量报告 + 差距清单落盘（evidence schema）+ 单端缺帧不充绿 | 差距清单缺场景行即 RED；单端帧缺失聚合 PASS 即 RED；M130 digest 不等仍出报告即 RED | host+device | G10.5 |
| M140 | 差距清单登记门 | 每差距项带 UE5 模块归属（Renderer 源码模块路径枚举闭集）+ measured delta + 建议 P 级 + G11 承接锚 | 缺归属/缺承接锚行即 RED；非 measured 叙述充差距即 RED | host 纯 host | G10.5 |
| M141 | 性能对标基线门 | 双端同场景帧率采样（14 §5 协议：L0 环境验证 → warmup/稳态 → trimmed mean → IQR）+ 环境画像（驱动/锁频/WDDM-HAGS/TDR）随证据存档 + 双端交替采样顺序登记 | 未锁频/环境画像缺字段即 RED；采样轮数不足冒充即 RED | host+device | G10.5 |

---

## 4. 风险与止损

| ID | 风险 | 预警 | 止损 |
|---|---|---|---|
| R-G10-1 | UE5 出图环境建设规模与时长不可控（spike 实测：依赖清单全平台 Blob 上界 ≈177.9 GB、Windows 子集未实测；源码编译 150 GB+ 仅 K: 盘可行；Launcher 安装 ~40 GB 级下载——[design/g10_ue5_harness_spike.md](design/g10_ue5_harness_spike.md) 问题 1/2/5） | 实现波首日实测下载/安装/编译墙钟超预算 | 路径最终裁决归 G10.1 治理门（spike 建议 = Launcher 首选/源码编译备选）；待验证清单首日实测登记；超时回退 spike 备选臂并契约 §8 只追加；两臂诚实登记禁伪绿 |
| R-G10-2 | Epic 账号交互风险（GitHub 账号 qwasg 在 EpicGames 组织；Launcher 路径须 Epic 账号人工登录一次、登录状态未知——spike 问题 5/风险 2；账号状态/组织成员资格/EULA 条款变化） | 出图环境建设被授权问题阻断 | 立项时账号状态核验留痕；交互操作（登录/协议确认）设人工接管点；登录受阻回退源码编译臂（qwasg 凭据已核查可用，spike 风险 2）；禁在 CI 内嵌任何凭据 |
| R-G10-3 | 图像度量参考实现口径风险（FLIP/SSIM/PSNR 实现口径多样，版本漂移） | 同一图对不同实现输出不一致 | 参考实现选型与版本 pin 进 RFC 冻结；口径进 spec；逐图对拍门（M135/M136）+ 容差 measured 标定；口径漂移注入 RED 臂 |
| R-G10-4 | 压测资产许可风险（商用可再分发性不成立则 G15 商用收口受阻） | 资产许可未核验即进清单 | 白名单（CC0/CC-BY 族）G10.1 裁决冻结；逐资产 SPDX/URL/attribution/digest 登记（M131）；白名单外注入即 RED；未核验资产一律不得进清单 |
| R-G10-5 | GPU 锁频与 WDDM 计时风险（双端同机同 GPU 交替出图互相干扰；计时不准则帧率基线失真） | 双端采样方差异常/环境画像缺字段 | 锁频协议为采样硬前置；环境画像（驱动/锁频/WDDM-HAGS/TDR）随证据存档（14 §5）；双端交替采样顺序登记（M141）；未锁频即 RED |
| R-G10-6 | 双端口径不对齐（相机/光照/色调映射链任一不对齐则 A/B 无意义；ACES/AgX 已知差异） | 差距清单被「口径差」噪声淹没 | M130 确定性门为 A/B 硬前置（digest 不等不得出报告）；显示管线显式固定单插件；已知口径差登记为「口径差项」与「画质差距项」分列 |
| R-G10-7 | UE 出帧时域非确定性（TSR/时域累积初帧未收敛致帧 digest 不稳） | M129 双跑 digest 不等 | 固定 seed + warmup 帧数 + 收敛后捕获协议进 harness；双跑 digest 一致门（M129）为硬判据；不稳则场景行登记 not-ready 不充绿 |
| R-G10-8 | 范围蔓延：G10 滑向修复（差距一出就想修，修复是 G11） | G10.5 后出现修复 PR | 零修复纪律进契约（§1/§6）；G10.5 后任何画质修复 PR 判 out-of-scope；差距清单只登记不修复 |
| R-G10-9 | 联网资产获取不可用（源站/镜像失效、下载中断） | 场景获取失败 | 多源镜像登记；本地产物 digest 缓存登记；获取失败的场景行诚实登记 not-ready，禁以 fixture 生成器产物冒充真实压测资产 |
| R-G10-10 | UE 源码许可边界误用（UE 代码/着色器片段混入 rurix 仓库） | src/spec 出现 UE 源性片段 | UE 仅作外部参照出图端；零 vendoring、零片段复制；许可条款核验与监控形式**待定（G10.1 治理波裁决）**；违反即 revert + 留痕 |
| R-G10-11 | 工作区盘 H: 容量枯竭（spike 实测 H: 仅剩 6.9 GB——spike 风险 1；G10 参考帧/证据/资产登记持续增长） | 证据/帧库写入失败或撑爆工作区 | 帧库与大体积产物落外置盘（K:）并在 provenance 登记路径；资产入库形式裁决（§5 表项 9）纳入容量面；接近阈值时归档压缩历史 evidence（只增不删不改纪律下走归档留痕程序） |

---

## 5. 治理裁决表项（供契约 §7 登记；本计划不定案，裁决权归用户/立项治理）

| # | 事项 | 候选口径 | 不定案原因 |
|---|---|---|---|
| 1 | G10 立项时机与工作树处置 | 先处置 G9 收尾未提交项（若有）再立项，或带未提交项立项 | 处置方式影响 G10 文档集不可变 ref 的基线内容 |
| 2 | UE5 出图路径裁决 | ①本机源码编译（VS2022 17.14 齐备；150 GB+ 仅 K: 盘可行）②Launcher 安装 UE 5.8 正式版（spike 裁决建议首选——出处最干净的官方签名基线、最短路径、MRQ/HighResShot/glTF 导入开箱可用；唯一人工门槛 = Epic 登录一次）③公开参考图（spike 判不可机核，仅兜底不进验收链）——详见 [design/g10_ue5_harness_spike.md](design/g10_ue5_harness_spike.md) 问题 5 | spike 仅出建议不构成裁决；登录人工介入可行性、ue5-main 快照 vs 5.8.0-release 口径选择须治理定夺 |
| 3 | 压测场景首发清单范围 | Bistro/Sponza/CornellBox 等起步 + 追加候选项；首发场景数与名单 | 资产许可逐资产未核验（R-G10-4）；清单规模影响 G10.3/G10.5 工期 |
| 4 | 图像度量指标集与口径冻结 | FLIP + SSIM + PSNR 三指标；参考实现选型与版本 pin；HDR/LDR 域口径 | 参考实现口径差异未对拍（R-G10-3）；选型须 RFC 冻结 |
| 5 | 「G10 不设画质通过阈值」口径确认 | 差距全量 measured 登记即绿，修复归 G11；不设 FLIP/SSIM/帧率通过线 | 涉及 G10/G11 边界严肃性，须立项显式裁决 |
| 6 | G9 十项 defer 重评窗程序 | G10.6 逐行重判（G10.5 measured 数据为法定证据输入）；deferred history 只追加 | 裁决权与程序须立项留痕；SAFE-GPU 归属联动表项 7 |
| 7 | SAFE-GPU 归属 | G10 承接「Safe GPU Operator Platform 独立期」or 继续 defer G11+ | G9.7 承接锚 = 「G10+ 独立期立项」（[G9_P2_DECISIONS](../g9/G9_P2_DECISIONS.md) §1 SAFE-GPU 行）；与画面对标主线无依赖 |
| 8 | G9.8b 同日放行先例继承 | 继承：8a full-run 先行完成后允许同日进 8b（G8.8b/G9.8b 先例链）or 不继承 | 涉及 soak 严肃性口径，须立项显式裁决（沿 G9_PLAN §5 表项 6 模式） |
| 9 | 资产入库形式与编号纪律确认 | 压测资产二进制入库形式（git-lfs/外部缓存）；数字 CI 步骤 `post-interlock actual-next-free allocation` 确认 | 仓库体积与可再分发形式影响 G15 商用面；编号纪律须立项重申 |

---

## 6. 与既有面的边界（0-byte 纪律）

| 面 | 约束 |
|---|---|
| G9 车道 | G9 四件套/决策表/evidence schema/budget 0-byte；G9 closed 判据不回写；G9_P2_DECISIONS 原文不改写（重判走 deferred history 只追加 + G10.6 重判表） |
| G5~G8 冻结面 | G5~G8 closed 契约与判据 0-byte；触冻结面必须显式 RFC 修订行（G9 纪律继承） |
| 00–14 | 只勘误，独立提交 |
| spec/conformance | G10.1 期 0-byte；spec-first + RED 先行自 G10.2 实现门开放后起，spec 条款 PR 先于实现 PR |
| 注册表 | G10.1 期 0-byte；登记/翻转/history 追加归立项后；deferred history 只追加禁静默改判 |
| 编号 | G10.1 期零消费；数字 CI 步骤一律 `post-interlock actual-next-free allocation`（[G9 CI_GATES](../g9/CI_GATES.md) 已至步骤 172，v1.21）；一切编号以领取时实测 `next_free` 为准，禁止沿用任何草案建议值 |
| UE 源码 | `E:\Kimi_Agent_Taichi Engine 优化计划\references\UnrealEngine` 只读外部参照；不进 rurix 仓库、不 vendoring、不复制片段进 `src/spec`；许可条款核验**待定（G10.1 治理波裁决）** |
| 压测资产 | 资产本体不进 `src/spec`；许可登记进 G10 文档面；二进制入库形式**待定（G10.1 治理波裁决，§5 表项 9）** |
| 修复面 | G10 全域零画质修复 PR；差距清单只登记不修复；G11 立项只消费 G10.8b 锁定清单 + 承接锚 |
| 调研引用 | [渲染器调研](../../渲染器调研/) 七份为 2026-07-28 快照；G10.1 复核关键引用时效（沿 G9 R-G9-3 模式） |
| unsafe | U 段按立项 next_free；`rurix-render` forbid(unsafe) 维持 |

---

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-15 | 初版起草（G10.1 治理波）：以 G9_PLAN v1.1 为范式落七节结构——§1 目标与口径（G10 = UE5 画面对标基线期；基线 = UE 5.8 沿用 G9 口径；G10 只交基线与差距清单不承诺修复）；§2 九波结构（G10.1 治理 → G10.2 UE5 出图环境 → G10.3 压测语料 → G10.4 度量基建 → G10.5 首轮 A/B → G10.6 defer 重评窗 → G10.7 P2 穷举 → G10.8a soak → G10.8b close-out），每波退出门判据草案；§3 P0 建议清单 12 行（M128 起顺延，含独立硬判据/负例 RED 臂/device-host 性质）；§4 风险表 R-G10-1~10（UE5 编译依赖/Epic 账号/度量口径/资产许可/锁频 WDDM 计时等）；§5 治理裁决表 9 项（供契约 §7 登记）；§6 0-byte 边界。全文零写死数字阈值（阈值一律 measured 标定）；数字 CI 步骤一律 `post-interlock actual-next-free allocation`。零编号、零 registry、零 spec/src/conformance 改动。 |
