# G4 CI 门禁增量

> 所属契约:[G4_CONTRACT.md](G4_CONTRACT.md)(status active,2026-07-23 开工)
> 版本:v1.0(2026-07-23)
> 基线:[../m0/CI_GATES.md](../m0/CI_GATES.md) ~ [../ei1/CI_GATES.md](../ei1/CI_GATES.md)(全部沿用);本文只规定 G4 期的**增量**。
> 铁律不变:任何新增门禁必须在真实 PR 上以真实失败/通过路径验证过(反 YAML-only)。
> **开工脚手架口径**:本文 G4 增量步骤(76+ 预期)为**对应子期实现 PR 的计划项**,开工**不**写入 workflow YAML 真实步骤(随实现 PR 落地回填,对齐 M8~EI1 计划→回填范式)。**G4.0 包零 CI 代码改动**:预算 glob 已泛化为 `*_budget.json` 自动纳入 `g4_budget.json`(空态);counter/entries 不预造(登记与 evaluator 分支随实现 PR 同落,未知 id 强制 FAIL)。

---

## 1. Runner

沿用 M0 §1(自托管 RTX 4070 Ti 开发机)~ EI1 §1。G4 新增 runner 预置项:**无**——步骤 76/78 的 device 段用既有 CUDA 链(rxrt_* PTX,RURIX_REQUIRE_REAL)+ MSVC(cl.exe,步骤 43/74 同源);步骤 76/80 的 Vulkan 段用既有 Vulkan SDK 1.3.296.0 + 活驱动(步骤 61~67 同源);步骤 77/79 含纯 host 面;步骤 81 device 段用既有 D3D12 系统 SDK + CUDA 链(步骤 61/74 同源)。零网络外呼。

## 2. PR Smoke 追加步骤(计划项;**步骤自 76 起 = G4 claim**(number_ledger v1.13;步骤 70 = G3 showcase 永久留空 gap 维持不回填,步骤 71~75 = EI1 已兑现)。落地随 G4.2~G4.6 实现 PR 回填 workflow;各行为拟分配,门内容以契约 §4 与实现 PR 实测为准)

| # | 步骤 | 失败即红 |
|---|---|---|
| 76 | 图形 RHI 冒烟(契约 G-G4-3 通道前半;G4.2 落地接入,**RFC-0015 前置后**):`ci/uc05_graphics_rhi_smoke.py`(拟)—— ≥1 raster + ≥1 mesh 图形 pass 经 .rx RHI 库面 + 自动 barrier 出图 device 真跑像素判据(headless readback,RXS-0222 纪律;RURIX_REQUIRE_REAL);零 .rs 审计;RED:桩化 barrier 推导/漏声明 → 像素变或 strict 拒 | 是 |
| 77 | 图形 RHI 不变量门(契约 G-G4-3 通道后半;G4.2 落地接入):图形 pass 声明↔反射双向相等 / 图结构违例 reject 语料逐条断言(纯 host 恒跑,内建 red_self_test,步骤 73 同构) | 是 |
| 78 | 引擎嵌入 v3 冒烟(契约 G-G4-3 通道;G4.2 落地接入):rurix_rhi 图形导出面(export(c) 产)+ 生成头(CI 再生成逐字节比对)→ engine_host v3(C++/D3D12,LUID 匹配)编译链接 → 图形 pass device 真跑三方数值精确相等(步骤 74 结构先例;RURIX_REQUIRE_REAL);engine_host v2 既有资产 0-byte 核 | 是 |
| 79 | RD-035 执行面门(契约 G-G4-4 通道;G4.3 落地接入):别名复用分配器 + 执行期峰值计数器(I10 measured,峰值 < 声明容量 device 见证)/ 重排+并行调度新拦截项漏拦即红 / const 泛型容量越界编译期拒 reject 语料;矩阵↔语料↔报告三方一致性维持 | 是 |
| 80 | Vulkan RHI 通道冒烟(契约 G-G4-5 通道;G4.4 落地接入):.rx 单源 Vulkan RHI(compute+graphics 双腿)经 artifacts v2 通道 device 真跑数值对照 + spirv-val(RURIX_REQUIRE_REAL;复用 G3 vk 底座) | 是 |
| 81 | BLACKHOLE realtime 验收冒烟(契约 G-G4-7 通道;G4.6 落地接入):REALTIME_OK 六项物理自检 + 帧对照(offline 既有帧 vs realtime 帧);30fps 数值为 evidence 面不进硬门(计时波动,EA1 冷启动先例),SKIP 不充绿 | 是 |

修订走本文件 §7;步骤号一旦占用不复用;若最终步骤数少于拟分配,多余号作废声明留痕不回收(burned 机制,MR-0006/0007 先例);若超出,自 82 顺续 + number_ledger 校准。

预算 evaluator 自动合并加载 [g4_budget.json](g4_budget.json)(命名空间冲突即红;**开工恒空,counter 登记与 evaluator 分支随实现 PR 同落,g4.bench.* 随取证 measured_local 回填**)。**G4 close-out 必须跑 `--strict` 且全局零 estimated 残留**(14 §3)。

## 3. Release 层门禁

既有门禁 0-byte 沿用;G4 零 Release 层增量(engine_host v3 / blackhole 为验收工程物,非发布资产——进发布面需另期另裁,防范围蔓延)。`g4-closed` tag 不匹配触发器 `v[0-9]+.[0-9]+.[0-9]+*`,零误触发。

## 4. Nightly 追加

既有 nightly 全保留。**G4 无新增 nightly 项**:图形 RHI/嵌入/Vulkan 冒烟归 PR smoke 步骤 76~80(秒~分级);30fps bench 为一次性 evidence 非趋势项;步骤 69 blocked 探针(RD-034)维持恒跑不改。

## 5. Guardrail

沿用 M0~EI1 全部激活项。G4 期动作:

- 基准:开工默认 `ei1-closed`(check_guardrails resolve_base 现状即此,本包 0-byte);G4.7 close-out 切 `g4-closed` + 双基准 advisory 复核(基准链 mb1→g3→ei1→g4 单线性;EA1 仍 active 另裁)。
- milestones/g4/ 四件纳入既有 glob;本契约翻 closed 后自动纳入 check_closed_contracts 字节守卫。
- 步骤 41~75 既有判据 0-byte 只增;步骤 70 永久 gap 维持;步骤 69 blocked 探针恒跑。
- dxil 套件(404+ 恒定)/ vulkan 套件 grow-only;B 链 dxv validator + 签名门不可裁剪不旁路。
- 新 unsafe U31 起续号;新 RX 码 en/zh 成对;spec 修订表表头「版本」列名纪律。
- evidence/ 只增不删不改;GPU 实验全经 proc_guard;RURIX_REQUIRE_REAL=1 贯穿 device 段。

## 6. 验证程序

本文件增量步骤的验证 = 对应实现 PR 的 CI run(host 段恒跑 + device 段 RURIX_REQUIRE_REAL=1 真跑,run URL 归契约 §8);本开工包自身的验证 = check_number_ledger / check_schemas / check_structure PASS(契约 G-G4-1)。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-07-23 | 初版(G4.0 治理包同 PR);步骤 76~81 为拟分配,落地随实现 PR 回填 |
