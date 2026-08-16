<!-- Assisted-by: Kimi-K3（G11.1 治理波 RFC 起草） -->
<!-- Assisted-by: Kimi-K3（D-409 修法批，2026-08-16，Draft v0.2） -->
# RFC-0028 — G11 GI 与光照画质闭环语义（G11 伞形：R4 多反弹 GI 修复 / M99-clipmap 世界辐射缓存世界级承接 / R3 灯种子集表达 / spec/global_illumination.md 世界级登记翻转显式修订行 / C1 口径对齐 GI·天光语义面 / 修复闭环判据语义）

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0028（4 位制，编号永不复用，10 §9.5；编号按 2026-08-16 实测 `registry/number_ledger.json` namespaces.RFC `next_free=28` 领取，非推测号；`reserved_in_flight[G11]` 登记由 G11.1 治理波落） |
| 标题 | G11 GI 与光照画质闭环语义（G11 伞形单章） |
| 档位 | **Full RFC**（GI 运行时语义面——触 `spec/global_illumination.md` 冻结面 RXS-0360 世界级 not-triggered 登记翻转与 RXS-0357 门序面，G5~G9 冻结面改动必须 RFC 显式修订行；判档争议向上取严，10 §3 / AGENTS 硬规则 5） |
| 状态 | **Agent Approved（2026-08-16）**——D-409 对抗性评审已完成：独立评审会话 12 findings（3 high F1~F3 / 5 med F4~F8 / 4 low F9~F12）全部 disposition 落实（v0.2 修法批），§9.1 评审记录段已回填；同环境单一模型 provenance 偏差按 RFC-0015 §9.1/v1.29/v1.73（G9 三 RFC）/v1.90（RFC-0025）/v1.102（G10 双 RFC）先例如实登记于 §9.1，效力自限并留 G11.7b 终审复核锚。主会话核对契约/MAP/CI_GATES 三面一致后翻 Agent Approved。Approved 不构成实现许可，G11.2+ 由 G-G11-3 实现互锁硬门独立阻断 |
| 承接里程碑 | G11（G11.4 光照与 GI 修复波 M153/M154；G11.2 口径差对齐波 M144 消费面；验收门 G-G11-6 / G-G11-7） |
| 关联条款 | 拟落 spec **RXS-####~**（条款号一律 **post-interlock actual-next-free allocation**，不预写推测号；候选落点见 §5：`spec/global_illumination.md` 修订行 + 追加条款） |
| 依据决策 | D-406 v2.0 · D-409 · P-09 · P-13 · 10 §7/§9.5 · G11 立项十项裁决（[G11_CONTRACT](../milestones/g11/G11_CONTRACT.md) §7：裁决 3 修复闭环判据、裁决 4 M99-clipmap 承接确认、裁决 5 RFC 判档）· G11_CONTRACT §4.2（M144/M153/M154 硬判据字面）· [G11_PLAN](../milestones/g11/G11_PLAN.md) §2 G11.2/G11.4 · R-G11-1/R-G11-4 · [G11_ACCEPTANCE_MAP](../milestones/g11/G11_ACCEPTANCE_MAP.md) §1 · [`g10_gap_registry.json`](../milestones/g10/g10_gap_registry.json) R3/R4/C1 行（G10.8b 终审锁定，0-byte 消费）· [G10_DEFER_REEVALUATION](../milestones/g10/G10_DEFER_REEVALUATION.md) §1 M99-clipmap 行（rejudged-go 承接锚字面）· RXS-0357~0362（spec/global_illumination.md 冻结面）· RXS-0384~0391（spec/visual_comparison.md 度量口径冻结面）· [调研报告 2](../渲染器调研/调研报告2-GI与Lumen类全局光照.md)（GI-1.0/SHaRC 与 Lumen Radiance Cache 双级蓝本，2026-07-28 快照） |
| Provenance | `Assisted-by: Kimi-K3（G11.1 治理波 RFC 起草）` |
| Agent 批准 | 待 D-409 第 1 轮对抗性评审完成、findings 全部 disposition 后由主会话翻 Agent Approved（本文 Draft 不自行批准） |
| 对抗性评审 | **已完成**（D-409 第 1 轮，2026-08-16，独立评审会话零共享上下文；12 条 findings 全部采纳并修，disposition 逐条见 §9.1；provenance 偏差如实登记：评审者与起草者同模型、独立性 = 评审轮次隔离，非跨工具——跨工具评审者可得时建议补一轮；评审全文见 [rfc0028_adversarial_review.md](../milestones/g11/design/rfc0028_adversarial_review.md)） |

---

## 1. 摘要

本 RFC 冻结 G11「画质修复期」GI 与光照修复的语义面——五个子面一份冻结：

1. **R4 多反弹 GI 修复语义**（G11.4 M154 消费面之一）：GI 自「屏幕探针单反弹」升级为「多反弹」——屏幕探针近场 + 世界空间辐射缓存远场兜底的双级语义；反弹次数、能量守恒（只丢能量不漏光口径继承 RXS-0358）、远场能量回归的判定语义。
2. **M99-clipmap 世界辐射缓存世界级承接语义**：世界级（clipmap 级）辐射缓存 = **空间哈希世界缓存（双哈希 + 线性探测，GI-1.0/SHaRC 蓝本）+ 距离自适应辐射 LOD（clipmap 语义：按距离的辐射度细节层级）+ 屏幕缓存失效处回落世界缓存**；远场能量回归 measured 判定、回落路径计数、辐射 LOD 层级计数、与 M96 golden 匹配深度对拍（RXS-0357 L6 门序衔接）。
3. **R3 灯种子集表达语义**（G11.4 M153 消费面）：点/面光源与 glTF emissive 的表达语义——光源集自契约 sun + sky 常量天光扩为「契约 sun/sky + glTF 点光源/面光源/emissive 表面」闭集；cornell 契约 sun+sky 灯面 0-byte 不动（复测对照口径）。
4. **spec/global_illumination.md 显式修订行**：RXS-0360「世界级 clipmap 未 measured 举证 not-triggered 不充绿」登记翻转为「世界级承接落地（G11.4，M99-clipmap rejudged-go 承接锚兑现）」——修订行经本 RFC 授权，条款字面修订走 spec-first 程序（post-interlock actual-next-free allocation 领新 RXS 条款承载修订行，RXS-0360 既有字面 0-byte）。
5. **C1 口径对齐 GI/天光遮蔽语义面**（G11.2 M144 消费面）：UE SkyLight 指定 cubemap 全向 IBL vs Rurix 屏幕探针 + 世界缓存的口径差**不拟合、只对齐**——天光/太阳辐照链参数化对齐 + 残余口径差显式登记的语义；曝光/位深口径面（C2/C3）为 harness 派生链面对齐，不进本 RFC（Direct PR 面，G11 立项裁决 5）。

**修复闭环判据语义**（横切五面）：每修复项闭环 = 修复落盘 + 修复前后度量 delta 收敛 measured（复测 delta 相对 G10.8b 锁定基线 delta 收敛，收敛阈值由标定程序 measured 产出禁手写）+ 契约参数 digest 0-byte。**本 RFC 不冻结任何绝对画质通过线**——「已达 UE5 画质」判定归 G15 商用收口期。

```text
G10.8b 锁定清单（R3/R4/C1 measured delta + 承接锚，0-byte 消费）
   │  法定输入：R4 P0（GI 单反弹，HDR p90 delta=4.697253086805343）
   │            C1 P1→G11 P0（GI/天光口径差，HDR 中位 ≈21×）
   │            R3 P0（灯种子集，HDR 中位 delta=2.664779790997505）
   ▼
G11.2 口径对齐（C1：天光/太阳辐照链参数化对齐 + 残余口径差登记）
   │  硬前置——口径不对齐则修复闭环断言被口径噪声淹没
   ▼
G11.4 光照与 GI 修复：
   R3 灯种子集表达（点/面光源 + glTF emissive 闭集）
   R4 多反弹 GI ＝ 屏幕探针近场 + 世界辐射缓存远场兜底（双级）
     └─ M99-clipmap 世界级承接：空间哈希世界缓存 + 距离自适应辐射 LOD
        + 屏幕缓存失效回落 + 远场能量回归 measured（rejudged-go 锚兑现）
   ▼
G11.5 同契约复测：修复前后 delta 收敛 measured（收敛阈标定程序产）
```

本 RFC 是 **G11.1 governance-only** 交付物。即使随后 Agent Approved，也只表示语义评审通过，**不会解锁任何 `src/`、`spec/`、`conformance/` 实现**；G11.2 互锁（G-G11-3）是独立硬门。§4 全部 schema/参数/算法为**拟议语义（Draft）**，批准前不构成契约。

## 2. 动机、范围与治理门

### 2.1 为什么需要 Full RFC

G10.6 重评窗已实证：「屏幕探针远场缺失成为画质 measured 问题」（R4 P0：bistro HDR 亮度 p90 a=0.30276253819465637 vs UE 5.000015625，delta=4.697253086805343；C1 P1：GI/天光遮蔽口径差 = 室内亮度主差 ≈21×）并按只追加程序 rejudged-go，指定 G11 画质修复期承接世界辐射缓存世界 clipmap 级。该承接面翻转 `spec/global_illumination.md` RXS-0360 的冻结登记（「世界级 clipmap 未 measured 举证 not-triggered 不充绿；本条款只冻结屏幕级」）——G5~G9 冻结面改动必须 RFC 显式修订行（G11_CONTRACT guardrails 字面）；多反弹 GI 语义、世界级缓存的空间索引/辐射 LOD/回落语义、灯种子集表达语义均属运行时渲染语义面，MR（Mini-RFC）体例不承载新语义面 + 冻结面修订（RFC-0025 判档先例），判档向上取严为 **Full RFC**。

法定输入字面（G11.1 立项已定案）：

- G11 立项裁决 3：「修复闭环判据 = 修复前后度量 delta 收敛 measured（收敛阈值由 G11.2/G11.5 标定程序 measured 产出，禁手写）；G11 不设绝对画质通过线」——本 RFC §4.6 即该裁决的语义兑现面；
- G11 立项裁决 4：「M99-clipmap 承接确认：G10.6 rejudged-go 逐字承接；语义面经 Full RFC-0028 冻结（RXS-0360 世界级 not-triggered 登记翻转走显式修订行）」——本 RFC §4.2/§4.4 即该裁决的兑现面；
- G11_CONTRACT §4.2 三行 P0 硬判据（M144/M153/M154）是本 RFC 语义面的下游机器消费者，判据字面不在本 RFC 重定。

### 2.2 双门互锁：RFC 批准不等于实现开工

| 门 | 允许动作 | 禁止动作 |
|---|---|---|
| G11.1 governance-only（本波） | 起草/评审/批准 RFC；冻结语义面与 §5 spec 映射计划；编号 claim 登记 | 不改 `src/`、`spec/`、`conformance/`；不 materialize 数字 CI 步骤；不预建空 schema 壳/空脚本占位；不领取 RXS/RD/U/RX 共享在途号 |
| G11.2+ implementation gate | G-G11-3 机器事实（validator READY + 用户开工指令 + actual `next_free` 重校）齐备后，spec-first 落条款与 RED | 互锁任一红时不得以 RFC Approved 或立项裁决替代机器事实 |

### 2.3 in-scope（语义面 ↔ P0 key 映射）

| 面 | 本 RFC 冻结内容 | P0 key（G11.1 已冻结字面，0-byte 引用） | 最晚波次 |
|---|---|---|---|
| C1 口径对齐（GI/天光面） | 天光/太阳辐照链参数化对齐 + 残余口径差登记语义 | `g11.p0.m144.caliber_c1_indoor_luminance` | G11.2 |
| R3 灯种子集表达 | 光源集闭集（sun/sky + 点/面光源 + emissive）+ cornell 契约灯面 0-byte | `g11.p0.m153.fix_r3_light_subset` | G11.4 |
| R4 多反弹 GI | 双级语义（屏幕探针近场 + 世界缓存远场兜底）+ 能量守恒口径 | `g11.p0.m154.fix_r4_gi_multibounce_world_cache` | G11.4 |
| M99-clipmap 世界级承接 | 空间哈希世界缓存 + 距离自适应辐射 LOD + 回落路径 + 远场能量回归 measured 判定 | `g11.p0.m154.fix_r4_gi_multibounce_world_cache`（同 key 双语义面，不拆双 key） | G11.4 |
| 世界级登记翻转修订行 | RXS-0360 not-triggered 登记翻转的修订行语义 | （spec-first 面，M154 门机核消费） | G11.4 |
| 修复闭环判据 | delta 收敛 measured + 锁定基线锚消费 + 收敛阈标定程序产语义 | M144/M153/M154 + `g11.p0.m155.ab_retest_closure`（消费面） | G11.2~G11.5 |

key/脚本/evidence schema 三方逐字一致字面以 [G11_ACCEPTANCE_MAP](../milestones/g11/G11_ACCEPTANCE_MAP.md) §1 为唯一事实源；本 RFC 只消费不新造。C2 曝光链 / C3 位深 / R1 材质 / R2 法线 / R5 i64 / U1 壳体 / U2 DDS / U3 动画八面经 G11.1 判档评估不触 spec 语义冻结面 → **Direct PR 面**（G11 立项裁决 5；触则升级 Full RFC 修订行，升级触发条件见 §8），不在本 RFC in-scope。

## 3. 指导级解释（用户视角）

### 3.1 为什么单反弹不够

G10.5 的 A/B 对拍里，bistro 室内 Rurix 帧 HDR 亮度 p90 只有 0.303，UE 5.8.1 是 5.000——约 16.5 倍。原因之一是 Rurix 侧 GI 只做「屏幕探针单反弹」：探针只收集屏幕内可见面的直接光做一次反弹。**屏幕外的能量回不来**——被家具遮挡的墙面反光、画面外灯光经地板的二次反弹，单反弹全部丢失。UE 侧的 Lumen 用「屏幕探针拿近场 + 世界空间辐射缓存兜底远场」的双级结构把这部分能量找回来。G11 的修复就是把同一双级结构落到 Rurix：近处继续用屏幕探针（G9 M99 已验收的屏幕级 SPG + Radiance Cache 不动），屏幕探针查不到的地方**回落**到世界空间的辐射缓存——它按空间哈希建索引、按距离分辐射细节层级（近细远粗，即 clipmap 语义），不需要任何离线烘焙。

### 3.2 什么算「修好了」

G11 不说「画质已达 UE5」——那是 G15 的事。G11 只说**修复闭环**：修复前 G10.8b 锁定清单里 bistro HDR p90 的双端 delta 是 4.697253086805343；修复后同一份契约参数（相机/光照/seed 一字不动）双端复跑，这个 delta 必须**收敛**——收敛多少算收敛，不由人写死，由标定程序对修复前后度量数据实测标定产出（和 G10.4 M138 标定度量阈值同一程序纪律）。灯种子集（R3）同理：bistro 包内实测 4+ 盏点光源和 emissive 表面被表达出来之后，HDR 亮度中位的 delta（2.664779790997505）必须收敛。而 C1 口径差（≈21×）在修复之前先**对齐口径**：双端天光/太阳辐照链参数化对齐，对不齐的残余部分显式登记为残余口径差——不拿没对齐的口径去度量修复，否则收敛断言会被口径噪声淹没。

### 3.3 世界级与屏幕级的界限

G9 已验收的 M99 是**屏幕级**（SPG 自适应细分 + Radiance Cache，g9.p1.m99 门绿）。本 RFC 承接的是**世界级 clipmap 级**：缓存索引从屏幕空间扩到世界空间（空间哈希），缓存内容按距离分辐射细节层级。两级的界限机核：远场探针集（屏幕缓存物理上查不到的场景区域）的能量回归必须 measured 非零且与参考对拍一致——屏幕级绿色不能冒充世界级验收（G11_CONTRACT §4.2 M154 行字面）。

## 4. 参考级设计（拟议语义，Draft）

### 4.0 跨面不变量

1. **measured-only**：全部容差与阈值（对拍容差、收敛阈值、远场能量回归判定阈）经标定程序实测标定入 `g11_budget.json`（measured_local，provenance 齐备，P-09）；本 RFC 冻结口径、语义与标定程序语义，**不冻结任何数值阈值**；手写阈值冒充标定即 RED。
2. **fail-closed / strict-only**：schema 外字段、未知枚举值、能量计数缺失、回落路径未登记、契约 digest 漂移、残余口径差未登记，均确定性拒绝，不得静默继续或静默降级。
3. **deterministic**：同契约参数双跑 digest 一致（M129/M130 既有字面继承）；世界缓存构建/查询不含未登记随机量——采样种子走契约 time.random_seed 面（u64 全域，R5 修复后消费面）。
4. **修复范围唯一法定来源**：只消费 `milestones/g10/g10_gap_registry.json` 11 行闭集 + 每项承接锚字面；不得无锚新立修复项；锁定清单 0-byte 不回写。
5. **0-byte 边界**：`spec/global_illumination.md` RXS-0357~0362 既有条款字面 0-byte（世界级登记翻转走新条款修订行，不改写 RXS-0360 原文）；`spec/visual_comparison.md` RXS-0384~0391 度量口径 0-byte；G9 M99 屏幕级 SPG + Radiance Cache 已验收面 0-byte；cornell 契约 sun+sky 灯面 0-byte；契约参数（相机/光照/seed/post）digest == G10.5 锁定值 0-byte；UE 源码零 vendoring、零片段复制。
6. **门序硬约束继承**：GI 门验收以 RXS-0357（M96 golden）门绿为前置（RXS-0357 L6 字面）；契约 digest 不等仍出 A/B 报告即 RED（M130/M139 门序字面，G11.5 复测继承）。

### 4.1 R4 多反弹 GI 修复语义（双级结构）

**裁决：GI = 屏幕探针近场 + 世界空间辐射缓存远场兜底的双级结构；反弹支持多反弹（≥2），远场能量经世界缓存回归。**

1. **双级语义**：近场 = 屏幕探针（G9 M99 已验收屏幕级 SPG + Radiance Cache 底座 0-byte 复用）；远场 = 世界空间辐射缓存（§4.2）。屏幕探针查询失效（无有效屏幕覆盖/反照率不足/超出屏幕域）时**必须回落**世界缓存，回落路径逐帧计数进 evidence；禁止静默返回零辐射（远场能量丢失即 R4 差距的成因面）。
2. **多反弹语义**：间接光计算支持 ≥2 次反弹；第二次及以上反弹的入射辐射度经世界缓存查询获得（屏幕探针只承担第一级近场面）。反弹次数、每级能量计数进 evidence；反弹截断处只丢能量、不漏光（RXS-0358「只丢能量不漏光」口径继承——漏光像素计数 = 0 断言面继承；**漏光适用面注（D-409 F9 修法）**：本语境漏光 = 双级合计后非物理正能量穿越遮挡的像素，判定 = 与 M96 golden 按匹配深度对拍超容差带的漏光模式像素，计数 = 0 断言面沿 RXS-0358 口径继承）。
3. **能量守恒口径**：双级合计的远场能量回归必须 measured 非零（对屏幕缓存物理不可达区域）；与 M96 golden 按匹配深度对拍（RXS-0357 L2/L6 门序面衔接——匹配深度表与容差带 0-byte 引用）。
4. **host 参考管线消费面（双侧最小兑现面裁决，D-409 F1 修法）**：A/B host 参考管线（g10_5_scene_render）消费同一双级语义，形态裁决 = **同构世界缓存的 host CPU 参考实现**（同一语义面双实现——解析式远场估计不构成「世界辐射缓存世界级」语义兑现，否决）；renderer 面 = 世界级缓存落地 + 远场能量回归判定锚（§4.2.4 spec 面），host 面 = G11.5 复测 R4 delta 收敛断言的载体。**不以 host 参考管线多反弹冒充 GPU 管线世界级验收，不以 GPU 管线世界级落地冒充 host 臂 delta 收敛**（立项裁决 8：复测臂 = host CPU 参考管线 + UE MRQ；GPU 管线双端面锚定 G14 不动）。工程量风险登记（R-G11-4 联动）：若 host 同构面实测工程量失控，按只追加程序登记 G12+ 承接并契约 §8 只追加修订 M154 判据——**禁以屏幕级绿色或解析式捷径冒充世界级**。

### 4.2 M99-clipmap 世界辐射缓存世界级承接语义

**裁决：世界级辐射缓存 = 空间哈希世界缓存（双哈希 + 线性探测）+ 距离自适应辐射 LOD（clipmap 语义）+ 屏幕缓存失效回落；无预处理、在线建格、动态内容直接适配（GI-1.0/SHaRC 蓝本，调研报告 2 §1/§3）。**

1. **空间索引形态**：世界空间哈希缓存——位置按距离自适应量化（近细远粗，辐射 LOD 层级即 clipmap 级语义：每一级对应一个距离带的辐射度细节层级；**量化函数族闭集（D-409 F11 修法）= {对数族, 幂律族}**，具体函数与参数实现波 measured 标定后冻结入条款，P-09；族外发明即口径漂移 RED），哈希冲突走线性探测；索引结构在线构建、零离线预处理（Surface Cache/Mesh Card 重资产路径继续后置，调研报告 2 §2.1 字面）。
2. **辐射 LOD（clipmap 级）**：按距离自适应的辐射度细节层级；层级数、每层覆盖距离带、每层命中率/耗时逐帧计数进 evidence；禁静默降层级（降级路径显式登记，RXS-0359 禁静默回退口径继承）。
3. **回落语义**：屏幕探针失效处回落世界缓存（§4.1.1）；回落查询命中率、回落辐射度能量计数进 evidence；世界缓存未命中再回落天光/常量环境项（末级兜底显式登记）。
4. **世界级验收判定（机核面，D-409 F3 修法——双锚同真）**：①远场探针集（屏幕缓存物理不可达的场景区域集，场景标定面登记）能量回归 measured **达标定阈**（阈值由标定程序 measured 产——「非零」字面不构成判定，任意噪声冒充能量回归即 RED）；②与 M96 golden 按匹配深度对拍一致（RXS-0357 L2 匹配深度表与容差带 0-byte 引用，L6 门序硬约束——M96 golden 未绿本面不得验收）。双锚同真方为世界级；**UE 对拍面归 G11.5 复测 delta 收敛（§4.6），不与 M96 golden 混用**。**屏幕级绿色（g9.p1.m99）不得冒充世界级验收**（G11_CONTRACT §4.2 M154 行字面）。
5. **承接锚字面（G10.6 rejudged-go，0-byte 转引）**：「重判条件已命中（G10.6：R4 P0 + C1 P1 measured 举证落地）→ G11 画质修复期承接世界辐射缓存世界 clipmap 级（只消费 G10.8b 锁定清单 R4/C1 行 + 本锚）；兜底 = 屏幕级 SPG + Radiance Cache（g9.p1.m99 门绿）维持」——本 RFC §4.2 即该承接锚的语义兑现面；兜底面 0-byte 不动。

### 4.3 R3 灯种子集表达语义

**裁决：光源集 = 契约 sun + sky 常量天光 + glTF 点光源/面光源/emissive 表面闭集；cornell 契约 sun+sky 灯面 0-byte 不动（复测对照口径）。**

1. **光源集闭集**：场景光源集 = { 契约 sun（方向光）, 契约 sky（常量天光）, glTF 点光源集（KHR_lights_punctual point）, glTF 面光源集（area/spot 若包内存在）, glTF emissive 表面集（材质 emissiveFactor/emissiveTexture 非零面）}——五元闭集，缺类显式登记（不得以缺类冒充空集）。
2. **bistro 消费面（单通道冻结，D-409 F4 修法）**：包内 pointLight1~N（glTF 节点实测 4+ 盏）与 emissive surfaces 全部表达进渲染；**光源参数唯一事实源 = 契约光照参数面（M130 schema corpus/lighting_*.json 面）**——包内 glTF 字段为**派生输入**（经 corpus 派生链转入契约光照 JSON，语料修订走 M133 只追加修订程序），Rurix harness 与 UE build_scenes 双端同消费契约 JSON；**禁止运行时双通道并存**（M130 契约 digest 一致性门序继承），glTF 字段直读绕过契约面即 RED；每盏光源的位姿/强度/色温 provenance 逐盏登记。
3. **cornell 契约灯面 0-byte**：cornell 语料灯面维持契约 sun+sky（G10.3 生成器注释登记口径）——复测对照一致性（G11.5 契约 digest 锁定值 0-byte）；bistro 灯面表达不得回流改写 cornell 灯面。
4. **emissive 语义**：emissive 表面作为光源参与直接光与 GI 双级能量贡献；emissive 强度/纹理消费口径与材质面（R1 修复面）解耦登记——材质未消费面如实登记，不以 emissive 表达冒充材质修复。

### 4.4 spec/global_illumination.md 显式修订行（世界级登记翻转）

**裁决：RXS-0360「世界级 clipmap 未 measured 举证 not-triggered 不充绿」登记翻转为「世界级承接落地（G11.4）」——修订行经本 RFC 授权，以新 RXS 条款承载（post-interlock actual-next-free allocation 领取），RXS-0360 既有字面 0-byte 不改写。**

1. **翻转依据（measured 举证已落地，G10.6 法定通道）**：`g10_gap_registry.json` R4 行（P0，bistro HDR p90 delta=4.697253086805343，evidence_digest sha256:d5f5d644…）+ C1 行（HDR 中位 ≈21×）双行 measured 举证命中「屏幕探针远场缺失成为画质 measured 问题」——重判条件已命中（G10.6 重评窗核验，deferred.json RD-040 history 2026-08-15 行）。
2. **修订行内容（spec-first 面）**：世界级辐射缓存的空间索引/辐射 LOD/回落/远场能量回归判定语义（§4.2 全量）落为新条款；RXS-0360 维持「屏幕级」冻结面 0-byte；新条款与 RXS-0357 门序（L6）、RXS-0358 能量守恒口径、RXS-0359 禁静默回退口径的衔接逐条登记。**边界声明（D-409 F7 修法）**：世界级辐射缓存 **≠** RXS-0359 L4 Far Field 档——L4 为追踪降级链远场档（M98-l4 维持 defer，承接锚字面 0-byte 不动），世界级缓存为辐射度复用缓存；两语义面不互冒充，世界级落地不构成 M98-l4 的静默兑现。
3. **门序**：新条款 PR 先于实现 PR（spec-first）；M154 门机核消费新条款字面（条款头在树 + 修订行字面）——无条款落地冒充世界级即 RED。

### 4.5 C1 口径对齐 GI/天光遮蔽语义面（不拟合、只对齐）

**裁决：C1 口径差对齐 = 双端天光/太阳辐照链参数化对齐 + 残余口径差显式登记；不拟合、不反向调参凑数。**

1. **天光口径面（参数集枚举冻结，D-409 F5 修法）**：UE SkyLight 指定 cubemap 全向 IBL vs Rurix 屏幕探针 + 世界缓存——天光辐照链参数化对齐，参数集枚举闭集 = { 天光模式（cubemap IBL / 探针采样）、强度（cd/m² 或 lux 同单位链）、色温/光谱面（常量或 cubemap 资产 digest）、采样档位（探针分辨率/光线数档位） }——双端逐项同参数登记，任一环节不对齐即该环节残余口径差；**cubemap 资产若使用须过 M131 许可白名单登记面（SPDX/来源 URL/attribution/资产 digest，未登记资产混入即 RED）**。对齐后残余口径差（结构面差异：全向 IBL vs 探针采样的覆盖差）显式登记为残余口径差项，进复测差距清单 caliber_diff 面（RXS-0391 schema 面 0-byte 消费）。
2. **太阳 lux→辐射度链**：双端太阳强度 lux→辐射度转换链参数化对齐（EV100 同字面前提，C2 派生尺度面解耦）；转换链每环节参数 provenance 登记。
3. **不拟合**：禁止以拟合/调参使 delta 人为缩小——对齐 = 参数化口径一致或显式登记残余，不以拟合冒充对齐（M144 判据字面）；残余口径差未登记即 RED。
4. **与修复面的序贯**：C1 对齐是 R3/R4 修复闭环断言的硬前置（G11.2 → G11.4 单点依赖，G11_PLAN §2 字面）——未对齐口径消费复测 delta 即 RED。

### 4.6 修复闭环判据语义（delta 收敛 measured）

**裁决：修复闭环 = 修复落盘 + 修复前后度量 delta 收敛 measured（复测 delta 相对 G10.8b 锁定基线 delta 收敛）+ 契约参数 digest 0-byte + 不降级既有 48 门绿面；收敛阈值由标定程序 measured 产出，禁手写。**

1. **锁定基线锚**：每修复项的基线 delta 转引自 `g10_gap_registry.json` 对应行 `measured_delta[].delta`（0-byte 消费）；G11.1 已转录为 `g11_budget.json` 的 `g11.closure_baseline.*` 十一条基线锚（direction=max：同 row 重登记 delta 不得大于本锚——防修复反向恶化冒充）。
2. **收敛判定（D-409 F2 修法——分两款 + 方向性）**：**quality_gap 行（R/U 族）**：收敛语义 = 复测 delta（G11.5 同契约双端复跑实测）**向 0 收敛**（双端趋于一致），非绝对值单调缩小——|复测 delta| < |基线 delta| 且收敛幅度 ≥ 收敛幅度阈值；方向性注入（修复反向过冲冒充收敛 / 绝对值缩小但双端仍实质不一致冒充闭环）即 RED。**caliber_diff 行（C 族）**：闭环语义 = 口径对齐完成（参数化一致或显式互证登记）+ 残余口径差显式登记 + 复测 delta 与登记残余一致——口径差行不是「被修没」，不以 quality_gap 款收敛字面冒充口径对齐闭环；未对齐口径消费复测 delta 即 RED。收敛幅度阈值由标定程序对修复前后度量数据实测标定产出（p100×k 同 M138 程序纪律），标定值入 `g11_budget.json`（measured_local，provenance 齐备）；收敛阈值缺失（标定未产）时闭环断言不成立——不得以「delta 有变小」叙述冒充收敛判定。
3. **契约 digest 0-byte（锁定值溯源，D-409 F8 修法）**：修复不得改契约参数（相机/光照/seed/post）——G11.5 复测契约参数 digest == G10.5 锁定值（cornell `sha256:80305791a68ccc66c5b046efaf193244796b52570494cf00aa1c86efa55be118` / bistro `sha256:ad45951ba641106b24e7d91d49ebf5992fb6a42cb70a3082520e8de19a6cf514`，联合 `sha256:64fd54df6e9be522d6dbb3bec8fac1eb30a0a421c7a5a8185a3452c381178aa4`）；**锁定值的机核验事实源 = `evidence/g10_m130_dual_determinism_contract_20260815T233315Z.json`**（M130 `--phase g10.5` 门实测登记，param_digest 三方一致）——复测门以该 evidence 登记值为机核基准，本 RFC 字面为转引便利不构成唯一事实源；不等仍出报告即 RED（门序硬约束继承）。
4. **不设绝对通过线**：本 RFC 不冻结任何「画质已达 UE5」绝对判定；闭环判据只断言 delta 收敛 measured，不断言绝对画质达标（G15 商用收口期面）。

## 5. 下游 spec 条款映射（spec diff 计划，G11.2 互锁后 materialize）

条款号一律 **post-interlock actual-next-free allocation**（当前实测 `RXS.next_free=392` 仅为快照，以落盘时实测为准），不预写推测号。

| 面 | 目标 spec | 形态 |
|---|---|---|
| 世界级辐射缓存（空间索引/辐射 LOD/回落/远场能量回归判定） | `spec/global_illumination.md` | 新条款承载（含 RXS-0360 世界级登记翻转修订行字面；RXS-0360 既有字面 0-byte） |
| R4 多反弹 GI 双级语义（近场屏幕探针 + 远场世界缓存 + 能量守恒口径衔接） | `spec/global_illumination.md` | 新条款（与 RXS-0358/0359/0360 衔接登记） |
| R3 灯种子集表达（光源集五元闭集 + emissive 语义 + cornell 契约灯面 0-byte 声明） | `spec/global_illumination.md`（治理期裁决统一落点，D-409 F6 修法——灯光作为 GI/直接光能量源语义面，与 RXS-0361 多灯面同卷衔接；候选既有卷本体 0-byte 声明维持） | 新条款 |
| C1 口径对齐（天光/太阳辐照链参数化对齐 + 残余口径差登记语义） | `spec/visual_comparison.md` | 追加新条款（RXS-0384~0391 既有字面 0-byte） |
| 修复闭环判据（锁定基线锚消费 + 收敛判定 + 标定程序语义） | `spec/visual_comparison.md` | 追加新条款 |

## 6. feature gate / tracking / 实现序（G11.2 互锁后生效）

### 6.1 Gate 命名空间（G11.1 已冻结字面，0-byte 引用）

`g11.p0.m144.caliber_c1_indoor_luminance`（G11.2）· `g11.p0.m153.fix_r3_light_subset`（G11.4）· `g11.p0.m154.fix_r4_gi_multibounce_world_cache`（G11.4）· `g11.p0.m155.ab_retest_closure`（G11.5，消费面）——key/脚本/schema 三方逐字一致以 [G11_ACCEPTANCE_MAP](../milestones/g11/G11_ACCEPTANCE_MAP.md) §1 为唯一事实源。

### 6.2 真实 RED/GREEN

- M144：未对齐口径消费复测 delta 注入即 RED；拟合冒充对齐注入即 RED；残余口径差未登记注入即 RED。
- M153：点光源未表达冒充修复注入即 RED；cornell 契约灯面漂移注入即 RED。
- M154：世界级未落地冒充承接注入即 RED（远场探针集能量回归为零冒充世界级）；屏幕级绿色冒充世界级注入即 RED（g9.p1.m99 evidence 冒充 M154 evidence）；漏光像素注入即 RED（能量守恒口径）；契约 digest 漂移注入即 RED。
- 每门 RED 臂须先在 main 上为 RED（spec-first + RED 先行），再随实现转 GREEN；反 YAML-only。

### 6.3 栈式实现序

```text
G11.2 C1 口径对齐（M144 + HDR-FLIP 标定 M157 同波）
  → G11.4 spec-first：世界级缓存/多反弹/灯种子集条款（本 RFC §5 映射）
  → R3 灯种子集实现（M153）→ R4 多反弹 + 世界级缓存实现（M154）
  → G11.5 同契约复测（M155 消费：R3/R4 delta 收敛机核）
```

## 7. 备选方案

1. **Lumen 式 Surface Cache/Mesh Card 重资产路径**：否决（本期）——每网格 6–8 方向卡片 + 图集 + 逐帧重捕获对场景系统依赖极深，rurix 场景系统尚年轻（调研报告 2 §2.1 字面）；GI-1.0/SHaRC 证明空间哈希世界缓存零预处理即可达可比质量。留 G12+ 重评（RD-040 族面）。
2. **DDGI 探针体（八面体探针图集）**：备选未采纳——DDGI 为 MVP 备选形态（调研报告 2 §1.1 #12 高推荐度字面，D-409 F10 修法如实表述）；未采纳理由：bistro 室内复杂遮挡下体积探针均匀采样的泄漏/滞后控制需额外工程面，而空间哈希世界缓存贴附主可见面采样、更贴近本差距清单的远场缺失面（R4 行成因）。DDGI Resampling 留 P4 预研轨（调研报告 2 §1.1 字面）。
3. **ReSTIR GI/GRIS 储层重采样**：否决（本期）——ReSTIR 家族是质量天花板而非地基，假设已有稳定逐像素追踪闭环（调研报告 2 §1.2 字面）；M100-high（ReSTIR 高档 reservoir）维持 defer（G10.6 重判字面，G11.6 触发评估登记）。
4. **纯屏幕探针加样/加反弹不建世界缓存**：否决——屏幕探针物理上无法回归屏幕外能量（R4 差距成因面）；加样只降噪声不补远场缺失，delta 不收敛。
5. **以拟合/调参对齐 C1 口径差**：否决——拟合冒充对齐（M144 RED 臂）；口径对齐 = 参数化一致或显式登记残余。
6. **世界级用体素光照 clipmap（Lumen Global SDF + voxel lighting clipmap 4 级）**：备选未采纳——依赖 Mesh SDF/Global SDF 重资产前置（同备选 1 后置）；空间哈希同构可达距离自适应辐射 LOD（clipmap 级语义兑现）且零预处理；clipmap 语义 = 距离自适应细节层级，不以体素结构为唯一载体（调研报告 2 §1.2/§3.2）。

## 8. 不做（范围红线）

1. 不冻结任何绝对画质通过线（「已达 UE5 画质」判定归 G15）。
2. 不做 Surface Cache/Mesh Card、Mesh SDF/Global SDF、DDGI 探针体、ReSTIR GI/GRIS、NRC 神经辐射缓存（各面归属见 §7）。
3. 不改写 RXS-0357~0362 既有条款字面（世界级翻转走新条款修订行）；不改写 RXS-0384~0391 度量口径；不改 G10 帧库/语料契约参数/cornell 契约灯面。
4. 不做 GPU 管线双端 A/B 面（锚定 G14，G10-N16 承接锚字面）；不以 host 参考管线冒充 GPU 管线验收。
5. 不做 C2 曝光链/C3 位深/R1 材质/R2 法线/R5 i64/U1 壳体/U2 DDS/U3 动画的 spec 语义面（Direct PR 面；**升级触发条件**：实现波若触及 `spec/display_pipeline.md` MaterialClosure 32B / `spec/imageio.md` / `spec/visual_comparison.md` 既有冻结字面，即升级 Full RFC 显式修订行——判档争议向上取严，G11 立项裁决 5/6）。
6. 不做帧率面（G14）；不做 DLSS/超分（G13）；不做路径追踪生产化（G12）。
7. UE 零 vendoring、零片段复制；UE 模块路径仅作差距归属参照（G10 口径继承）。

## 9. 未决问题 / 关键裁决

| # | 问题 | 状态 |
|---|---|---|
| Q1 | 空间哈希世界缓存的具体哈希参数（格尺寸/探测步长/层级数） | 实现波按场景实测标定（measured 后冻结，P-09）；本 RFC 只冻结形态语义 |
| Q2 | 远场探针集的场景标定面（哪些区域算「屏幕物理不可达」） | 实现波按 G10 语料双场景登记（标定程序产，进 evidence）；本 RFC 冻结判定语义不冻结区域集 |
| Q3 | bistro emissive 表面的强度口径（glTF emissiveFactor 与光源强度的换算链） | 实现波按包内字段实测登记；换算链 provenance 进 evidence |
| Q4 | C1 残余口径差的登记粒度（逐环节 vs 汇总） | 倾向逐环节（天光/太阳/曝光三环节分列），实现波按 M144 判据硬化 |
| Q5 | 世界级缓存与 M97 Surface Cache（G9 已验收）的边界 | 本 RFC 不消费 M97 面（Surface Cache 路径后置）；边界若重叠由实现波 spec-first 修订行登记 |
| Q6 | host 参考管线多反弹的实现形态（CPU 世界缓存同构 vs 解析式远场估计） | **已裁决（D-409 F1）**：同构世界缓存的 host CPU 参考实现（同一语义面双实现，解析式否决——不构成世界级语义兑现）；工程量失控兜底 = 按只追加程序登记 G12+ 承接并契约 §8 只追加修订 M154 判据，禁以屏幕级绿色或解析式捷径冒充世界级（§4.1.4） |

## 9.1 对抗性评审记录（对抗性评审要求，10 §3 / §7 · [`../13_DECISION_LOG.md`](../13_DECISION_LOG.md) D-409）

**评审记录**：D-409 第 1 轮对抗性评审已完成，评审全文与独立事实核对记录见 [rfc0028_adversarial_review.md](../milestones/g11/design/rfc0028_adversarial_review.md)。

| 字段 | 值 |
|---|---|
| 评审者 provenance | `Assisted-by: Kimi-K3（D-409 独立评审会话，与起草会话隔离）` |
| 评审轮次 | 第 1 轮，2026-08-16 |
| 评审会话形态 | 独立评审会话、零共享上下文——评审者不复用起草会话任何结论；G10.8b 锁定清单 R3/R4/C1 行、G10.6 承接锚字面、spec/global_illumination.md RXS-0357~0362 冻结面、G11_CONTRACT 判据与裁决、调研报告 2 蓝本面、G10.5 契约 digest 溯源 evidence、编号台账均由评审会话独立复核（十项独立事实核对记录在案） |
| provenance 偏差登记（逐字登记评审记录对应行） | 评审者与起草者**同模型**（Kimi-K3），独立性 = 评审轮次隔离 + 不复用起草结论，不满足 D-409 首选「跨工具/跨模型」字面。按 RFC-0015 §9.1 / number_ledger v1.29/v1.73/v1.90/v1.102 已登记先例如实偏差登记并效力自限：本评审不替代未来跨工具评审；跨工具评审者可得时建议补一轮；留 G11.7b 终审复核锚 |

**Findings 与 disposition**（12 条全部**采纳并修**，Draft v0.2 同批落实）：

| # | Finding（评审者提出） | 严重度 | Disposition |
|---|---|---|---|
| F1 | host 参考管线消费面与复测臂口径存在实现体量矛盾且形态悬空（§4.1.4 host 同构工程量 vs 参考管线定位；§9 Q6 悬空而 M154 判据字面依赖） | high | **采纳并修**：§4.1.4 改「双侧最小兑现面裁决」——形态裁决 = 同构世界缓存的 host CPU 参考实现（同一语义面双实现，解析式远场估计否决——不构成世界级语义兑现）；renderer 面 = 世界级落地 + 远场能量回归判定锚，host 面 = G11.5 R4 delta 收敛载体；工程量失控兜底 = 按只追加程序登记 G12+ 承接并契约 §8 只追加修订 M154 判据，禁以屏幕级绿色或解析式捷径冒充世界级；§9 Q6 关闭（已裁决） |
| F2 | 收敛判定方向性缺陷 + C 族行闭环语义错位（\|delta\| 单调缩小 ≠ delta→0；caliber_diff 行与 quality_gap 行闭环语义不同款） | high | **采纳并修**：§4.6.2 分两款——quality_gap 行收敛 = delta 向 0 收敛且幅度 ≥ 标定阈（方向性注入即 RED：反向过冲/绝对值缩小但双端仍实质不一致冒充闭环）；caliber_diff 行闭环 = 口径对齐完成 + 残余显式登记 + 复测 delta 与登记残余一致（不以 quality_gap 款字面冒充口径对齐闭环） |
| F3 | 世界级验收判定锚不可机核（「参考」未定义；「非零」阈值过弱，任意噪声即非零伪绿通道） | high | **采纳并修**：§4.2.4 硬化为双锚同真——①远场探针集能量回归 measured 达标定阈（标定程序产，「非零」字面不构成判定，任意噪声冒充能量回归即 RED）；②与 M96 golden 按匹配深度对拍（RXS-0357 L2/L6 门序面，容差带 0-byte 引用）；UE 对拍面归 G11.5 复测 delta 收敛，不与 M96 golden 混用 |
| F4 | R3 光源参数双通道未冻结（「契约光照参数面或 glTF 字段」——双端可各走一路破坏 M130 digest 一致性） | med | **采纳并修**：§4.3.2 冻结单通道——光源参数唯一事实源 = 契约光照参数面（corpus/lighting_*.json）；glTF 字段为派生输入（经 M133 只追加修订程序转入契约 JSON）；禁止运行时双通道并存，glTF 字段直读绕过契约面即 RED |
| F5 | C1 天光对齐「同参数」不可机核 + cubemap 资产面未登记 | med | **采纳并修**：§4.5.1 参数集枚举闭集冻结（天光模式/强度同单位链/色温或 cubemap 资产 digest/采样档位）；cubemap 资产若使用须过 M131 许可白名单登记面（未登记资产混入即 RED） |
| F6 | R3 目标 spec 卷摇摆（「或灯光语义面归属卷（实现波裁决）」——spec-first 落点应治理期裁决） | med | **采纳并修**：§5 治理期裁决统一落点 `spec/global_illumination.md` 新条款（灯光作为 GI/直接光能量源语义面，与 RXS-0361 同卷衔接）；候选既有卷本体 0-byte 声明维持 |
| F7 | 世界级缓存与 RXS-0359 L4 Far Field 档边界未声明（M98-l4 维持 defer，语义混同构成静默兑现） | med | **采纳并修**：§4.4.2 增边界声明——世界级辐射缓存 ≠ RXS-0359 L4 Far Field 档（L4 = 追踪降级链远场档，世界级缓存 = 辐射度复用缓存，两语义面不互冒充）；M98-l4 承接锚字面 0-byte 不动 |
| F8 | 契约 digest 锁定值缺机核验溯源（转录字面错误则 G11.5 复测门序错位） | med | **采纳并修**：§4.6.3 锁定值标注溯源 evidence `g10_m130_dual_determinism_contract_20260815T233315Z.json`（M130 g10.5 门实测登记）——复测门以 evidence 登记值为机核基准，RFC 字面为转引便利不构成唯一事实源 |
| F9 | 漏光像素计数 = 0 断言的适用面未说明（自 RXS-0358 Surface Cache 语境继承） | low | **采纳并修**：§4.1.2 补适用面注——本语境漏光 = 双级合计后非物理正能量穿越遮挡的像素（判定 = 与 M96 golden 匹配深度对拍超容差带的漏光模式像素），计数 = 0 断言面沿 RXS-0358 口径继承 |
| F10 | §7 备选 2 对 DDGI 表述与调研报告矛盾（调研报告 2 §1.1 #12 载 DDGI「MVP 备选形态」高推荐度） | low | **采纳并修**：§7 备选 2 改如实表述（DDGI 为 MVP 备选形态高推荐度字面）；未采纳理由修正为「bistro 室内复杂遮挡下体积探针均匀采样的泄漏/滞后控制需额外工程面，空间哈希贴附主可见面采样更贴近本清单远场缺失面」 |
| F11 | 距离自适应量化函数族未冻结（「近细远粗」无函数族语义，实现者可任意发明） | low | **采纳并修**：§4.2.1 冻结量化函数族闭集 {对数族, 幂律族}（具体函数与参数实现波 measured 标定后冻结入条款，P-09；族外发明即口径漂移 RED） |
| F12 | 评审 provenance 与起草同模型，偏差须随 findings 一并回填（RFC-0026 F17 先例） | low | **采纳**：本段「provenance 偏差登记」行逐字回填留痕；本评审不写成「跨工具」；跨工具评审者可得时建议补一轮；留 G11.7b 终审复核锚 |

**总评回填**：评审总评 = **approve-with-changes**（修订后可批准，非现状可批准）。本批修订（Draft v0.2，2026-08-16）将 F1~F12 全部采纳并修：三条 high 的判据可满足性/机核性空隙已在正文冻结（§4.1.4 双侧最小兑现面裁决 / §4.6.2 收敛两款 + 方向性 / §4.2.4 世界级双锚判定），五条 med 同批落实，四条 low 同批 disposition，§9 Q6 关闭、§5/§7 同步。**翻 Agent Approved 由主会话核对本批修订与契约三面（G11_CONTRACT / G11_ACCEPTANCE_MAP / CI_GATES）一致性后执行**（核对面：`ci/check_g11_acceptance_map.py` 三向 PASS + 互锁 validator 事实门④绿）。

## 10. 稳定化与 provenance

- **特性生命周期**（10 §5）：RFC Agent Approved 只是语义评审完成；随后仍需 G-G11-3 互锁 → spec-first/RED → gated implementation → tracking evidence → 至少两个里程碑无重大语义修订 → stabilization report → FCP-lite。
- **稳定面候选**：世界辐射缓存的空间索引/辐射 LOD/回落语义、远场能量回归判定语义、灯种子集五元闭集、修复闭环判据语义（锁定基线锚 + 收敛判定 + 标定程序）；是否 stable 由未来 stabilization report 裁决。
- **明确非 stable**：全部容差/阈值数值（标定程序 measured 产，`g11_budget.json` measured 行）、空间哈希具体参数、远场探针集区域集、emissive 换算链参数、层级数。
- **Provenance**：`Assisted-by: Kimi-K3（G11.1 治理波 RFC 起草）`；Draft v0.2 修法批 `Assisted-by: Kimi-K3（D-409 修法批）`。D-409 第 1 轮评审记录已回填 §9.1；翻 Agent Approved 由主会话核对后执行。

## 11. 规范与实现依据

- 仓库内：[G11_CONTRACT](../milestones/g11/G11_CONTRACT.md) §4.2/§7（P0 硬判据字面、立项十项裁决）· [G11_PLAN](../milestones/g11/G11_PLAN.md) §2 G11.2/G11.4、§4 R-G11-1/4 · [G11_ACCEPTANCE_MAP](../milestones/g11/G11_ACCEPTANCE_MAP.md) §1（key 字面）· [`g10_gap_registry.json`](../milestones/g10/g10_gap_registry.json) R3/R4/C1 行（measured delta + 承接锚，0-byte 消费）· [G10_DEFER_REEVALUATION](../milestones/g10/G10_DEFER_REEVALUATION.md) §1 M99-clipmap 行（rejudged-go 承接锚字面）· [G10_P2_DECISIONS](../milestones/g10/G10_P2_DECISIONS.md) §1 M99-clipmap/M100-high 行 · [spec/global_illumination.md](../spec/global_illumination.md)（RXS-0357~0362 冻结面：M96 golden 门序 / Surface Cache 能量守恒 / 四级追踪降级链 / M99 屏幕级 / M100 低档 / M101 IF 档位）· [spec/visual_comparison.md](../spec/visual_comparison.md)（RXS-0384~0391 度量口径冻结面）· [RFC-0022](0022-virtual-geometry-gi-semantics.md) §4.6~§4.10（GI 语义面原始冻结）· [RFC-0026](0026-visual-comparison-metrics.md)（度量口径与差距清单 schema）· [`registry/deferred.json`](../registry/deferred.json) RD-040（M99-clipmap 承接锚 history）。
- 调研依据：[调研报告 2](../渲染器调研/调研报告2-GI与Lumen类全局光照.md)（2026-07-28 快照）：GI-1.0 两级辐射缓存（Boissé 等，AMD/GPUOpen 2023/2024——空间哈希无预处理世界缓存 + 距离自适应辐射 LOD，¼ spp 2–3ms 级）为首选蓝本；Lumen Radiance Cache 双级（Wright 2021/2022——屏幕探针近场 + 世界缓存远场兜底 + 探针空间滤波 + 重要性采样 + 时域累积）为架构导师；SHaRC（RTXGI 2.0）为世界缓存工程参照；UE 5.6/5.7 Lumen 工程演进为优化参照。
- 口径标注：本 RFC 冻结形态与判定语义；全部数值参数（哈希参数/层级数/阈值）以实现波 measured 标定登记为准，本 RFC 不预写未实测事实；调研报告为 2026-07-28 快照，G11.1 已复核关键引用时效（GI-1.0/SHaRC/Lumen 蓝本面沿用 G10.1 复核口径）。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v0.1 | 2026-08-16 | AI 起草初版（G11.1 治理波）：五面语义冻结——§4.1 R4 多反弹 GI 双级语义（屏幕探针近场 + 世界缓存远场兜底 + 能量守恒口径继承 + host 参考管线消费面不冒充 GPU 管线）/ §4.2 M99-clipmap 世界级承接（空间哈希世界缓存 + 距离自适应辐射 LOD〔clipmap 级语义〕+ 屏幕缓存失效回落 + 远场能量回归 measured 判定 + 承接锚字面 0-byte 转引）/ §4.3 R3 灯种子集表达（光源集五元闭集 + bistro 4+ 盏消费 + cornell 契约灯面 0-byte + emissive 语义）/ §4.4 RXS-0360 世界级 not-triggered 登记翻转显式修订行（新条款承载，既有字面 0-byte）/ §4.5 C1 口径对齐（不拟合、只对齐 + 残余口径差登记）+ §4.6 修复闭环判据语义（锁定基线锚 + 收敛判定 + 标定程序产 + 契约 digest 锁定值字面 + 不设绝对通过线）；§5 目标 spec 裁决 = global_illumination.md 新条款 + visual_comparison.md 追加，条款号一律 post-interlock actual-next-free allocation；§7 备选六行（Surface Cache/Mesh Card 重资产否决留 G12+ / DDGI 未采纳 / ReSTIR 否决本期 / 纯屏幕探针加样否决 / 拟合对齐否决 / 体素 clipmap 未采纳）；§8 不做七行（含 Direct PR 面升级触发条件）；§9.1 空段待 D-409 回填；零 `src/`、`spec/`、`conformance/`、workflows 改动；零绝对画质通过线 | Full RFC（Draft） |
| Draft v0.2 | 2026-08-16 | D-409 第 1 轮对抗性评审修法批（12 findings 全部采纳并修）：F1 §4.1.4 双侧最小兑现面裁决（host 面 = 同构世界缓存 host CPU 参考实现，解析式否决；工程量失控兜底 = G12+ 承接 + §8 只追加修订 M154 判据）+ §9 Q6 关闭；F2 §4.6.2 收敛判定分两款（quality_gap 行 delta→0 收敛 + 方向性注入 RED / caliber_diff 行 = 口径对齐 + 残余登记 + 复测 delta 与残余一致）；F3 §4.2.4 世界级验收双锚同真（远场能量回归达标定阈〔非零字面不构成判定〕+ M96 golden 匹配深度对拍，UE 对拍面归 G11.5 不混用）；F4 §4.3.2 光源参数单通道冻结（契约光照参数面唯一事实源，glTF 字段 = 派生输入经 M133 只追加程序，直读绕过即 RED）；F5 §4.5.1 天光参数集枚举闭集 + cubemap 资产 M131 白名单联动；F6 §5 R3 目标卷治理期统一裁决 global_illumination.md；F7 §4.4.2 世界级缓存 ≠ RXS-0359 L4 Far Field 档边界声明（M98-l4 承接锚 0-byte）；F8 §4.6.3 契约 digest 锁定值溯源 evidence（g10_m130_dual_determinism_contract_20260815T233315Z.json 为机核基准）；F9 §4.1.2 漏光适用面注（双级合计非物理正能量穿越遮挡，M96 golden 对拍判定）；F10 §7 备选 2 DDGI 如实表述（MVP 备选形态高推荐度字面 + 未采纳理由修正）；F11 §4.2.1 量化函数族闭集 {对数族, 幂律族}；F12 同模型评审偏差如实登记 + G11.7b 终审复核锚；§9.1 回填评审 provenance 与 12 条 disposition 表与总评；翻 Agent Approved 由主会话核对三面一致后执行。`Assisted-by: Kimi-K3（D-409 修法批）` | Full RFC（Draft → 核对后 Agent Approved） |
| Agent Approved | 2026-08-16 | 主会话核对翻案：D-409 第 1 轮评审 12 findings 全部 disposition 落实（v0.2 修法批）；契约/MAP/CI_GATES 三面一致核对（`ci/check_g11_acceptance_map.py` 三向 PASS + 互锁 validator 事实门④绿）后翻 **Agent Approved**。Approved 只表示语义/治理评审完成，不解锁任何实现：G11.2+ 仍由 G11_CONTRACT G-G11-3 与 `ci/check_g11_implementation_interlock.py` 硬门约束。`Assisted-by: Kimi-K3（G11.1 治理波）` | Full RFC（Agent Approved） |
