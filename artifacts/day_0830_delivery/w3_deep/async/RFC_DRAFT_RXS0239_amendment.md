# RFC-DRAFT — RXS-0239 修订行草案:多队列 async compute **opt-in** 语义(W3 深水区起草,未登记)

| 字段 | 值 |
|---|---|
| RFC 编号 | **草案,未领号**(正式登记时按 `registry/number_ledger.json` `RFC.next_free` 实测顺位领取;或经裁决并入 RFC-0019 修订记录——RFC-0019 §4.8 语义面已 Approved,本草案的净增量只是 render_graph.md 面的条款修订行,倾向后者,留主 agent 裁决) |
| 标题 | RXS-0239 pass 边界 happens-before:多队列 async compute opt-in 加性臂 |
| 档位 | 修订行(🔒 禁区条款 RXS-0239 为 RFC-0013 全文批准对象;其「全序措辞封死」即修订门,任何多队列执行前置本修订——spec/render_graph.md:178-181/330-333) |
| 状态 | **Draft(无效力)**。不进 spec/、不进 rfcs/;落地前置 = M59 重判 **go**(D3-Q7 多队列 measured 收益证据,G9_P2_DECISIONS M59 行只追加重判程序)+ 对抗性评审(D-409) |
| 承接 | G31+ TODO #57/#59/#60/#62/#63;RFC-0019 §4.8(语义母版,Approved)§5 RP-MULTIQUEUE(materialize 目标 rhi.md + rendering_platform.md);本草案覆盖 render_graph.md RXS-0239 面 |
| 体例先例 | RXS-0346「🔒 唯一显式修订行表 + 既有条款字面 0-byte 声明」(render_graph.md §2A/v1.1);G4.3 PR-E「承诺字面不动 + 加性执行模型段」(render_graph.md:187-192) |
| Provenance | `Assisted-by: cursor:claude-fable-5`(侦察 + 起草;主 agent 正式登记前不签批准行) |

---

## 0. 要点(≤5 条)

1. **默认承诺面字面 0-byte 的 opt-in 加性臂**:RXS-0239「单 queue;声明序 = 提交序 = pass 粒度完成序」对
   **缺省执行形态字面不动**;仅当四条件同时成立(显式 `enable_async=true` + 设备 `timelineSemaphore` + 存在
   compute-only family + 单队列等价门新鲜绿),AsyncCompute 白名单 pass 与图形车道间的**完成序**放宽为
   fence 弧偏序;图形车道内部全序、pass 粒度可见性、pass 内不承诺三点全部维持字面。
2. **单/双队列 digest 等价门为硬前置**(RFC-0019 §4.8.3 语义):同图同输入,single-queue plan 与 multi-queue
   plan 输出资源 digest 逐字节相等;不等 = RED;回落臂 evidence 标 `single_queue_fallback`,不充多队列绿。
3. **跨队列同步唯一形态 = 一条 64-bit timeline semaphore + 成对 release/acquire**:wait 精确值;值域由
   `FencePair.value = v` 确定性映射为 `(2v-1, 2v)` 两点(计划面 FencePair 0-byte);同队列 signal 严格递增、
   timeline 依赖图无环;跨 family 资源过手 release/acquire 成对相等,半对/漏 wait/错值/值回退 = **提交前**
   validator 确定性 RED(RFC-0019 §4.8.2 五步序全文引用)。
4. **承诺面扩写范围最小化**(0-byte 清单):split barrier、pass 内重排、跨队列 concurrent write、多 timeline
   分轨、transfer 队列、EB 三轴结构(§4.8.4:ownership/timeline 是 companion metadata 不是第四轴)、
   RXS-0240 执行器逐字重放纪律、RXS-0241 cabi tag 域——全部不在本修订,措辞维持封死。
5. **程序前置**:本修订行落地以 M59 重判 go 为前置(D3-Q7 measured 收益 + 等价门恒绿);no-go 时本草案留档
   不落地,RXS-0239 字面维持,单 queue 全序为兜底(G9_P2_DECISIONS M59 行兜底字面)。

## 1. 修订对象与现行字面

- 对象:`spec/render_graph.md` `### RXS-0239 🔒 pass 边界 happens-before 语义本体`(158-192 行)。
- 现行承诺(Dynamic Semantics,163-170):「单 queue;声明序 = 提交序 = pass 粒度完成序。对任意 i < j,
  pass i 的全部 device 内存效应在 pass j 的任何访问之前发生且可见——每个 pass 边界是全序同步点。」
- 现行封死(严禁 UB 节,178-181):「多 queue / async compute / split barrier 不在承诺面(§8),其不存在性
  即由本条全序措辞封死——条款不为未来扩张预留弱化措辞。」⇒ 本草案即该「联动修订」本体。

## 2. 🔒 唯一显式修订行表(RXS-0346 体例)

| # | 修订行 | 性质 |
|---|---|---|
| 1 | RXS-0239 追加「多队列 opt-in 臂」子节(体例同 G4.3 PR-E 追加段):**缺省执行形态承诺字面 0-byte**;opt-in 臂四条件 = ①执行器显式 `enable_async=true` ②设备 `timelineSemaphore=true`(探测面 = `DeviceCapabilityReport.timeline_semaphore` 谱系)③存在 compute-only(COMPUTE 且非 GRAPHICS)queue family ④单队列等价门(修订行 4)新鲜绿。四条件任一不成立 → 义务回落缺省形态(显式 single-queue plan 重编译,非忽略 fence) | 加性子节 |
| 2 | opt-in 臂下的完成序语义:图形车道维持声明序全序;AsyncCompute 白名单 pass 的完成序仅由其 fence 弧(`signal_after` / `wait_before`)裁定——异步 pass 的全部 device 内存效应在 `wait_before` pass 任何访问之前发生且可见;`signal_after` pass 的全部效应在异步 pass 任何访问之前发生且可见;**弧外相对完成序不承诺**。可见性粒度维持 pass 粒度;pass 内语义维持不触碰(RXS-0079/0080/0068 独占管辖字面) | 加性语义 |
| 3 | 跨队列同步形态:一条 64-bit timeline semaphore;`FencePair.value = v` → timeline 点 `(2v-1, 2v)` 确定性映射(生产侧 signal `2v-1`,异步段 wait `2v-1` / signal `2v`,消费侧 wait `2v`);同队列 signal 严格递增、依赖图无环;跨 family 资源过手 = exclusive-sharing + release/acquire 成对(RFC-0019 §4.8.2 五步序);成对律破缺(半对/漏 wait/错值/值回退/双 owner)= 提交前 validator 确定性 RED,禁运行期静默。首期判档 harness 的 concurrent-sharing 简化臂须 evidence 显式登记,不得充 exclusive 语义绿 | 加性判据 |
| 4 | 单/双队列 digest 等价门:同图同输入,single-queue plan 与 multi-queue plan 的全部输出资源 readback digest **逐字节相等**(RFC-0019 §4.8.3/§6.2 M59 GREEN 字面);multi-queue 双跑位级一致;不等 = RED 且整窗禁判收益;`single_queue_fallback` evidence 标注律(不充多队列绿,但为 portability correctness 硬门) | 加性判据 |
| 5 | 0-byte 清单(不动声明):EB 三轴结构与推导规则 / RXS-0236 访问声明集 / RXS-0237 装配核验 / RXS-0238 状态机 / RXS-0240 双后端映射与逐字重放 / RXS-0241 cabi tag 域 0..=6 字面 / `FencePair` 布局与 `plan_lanes` 弧算法 / uc04 D6 手动复核门——全部字面 0-byte;split barrier / pass 内重排 / 跨队列 concurrent write / 多 timeline 分轨 / transfer 队列(#61/#64/#91)不在本修订,封死措辞对其维持 | 0-byte 声明 |

## 3. 条款措辞草案(落 spec 时的替换/追加文本底稿)

追加子节(置于 RXS-0239 现「G4.3 PR-E 追加段」之后,同体例):

> **G3x 追加「多队列 opt-in 臂」段(本修订行表,既有承诺字面不动)**:本条「单 queue;声明序 = 提交序 =
> pass 粒度完成序」承诺对**缺省执行形态字面不动**。opt-in 臂(条件:显式 `enable_async` + 设备 timeline +
> compute-only family + 单队列等价门新鲜绿)下,完成序 = 图形车道声明序全序 × AsyncCompute 白名单 pass 经
> fence 弧(`signal_after`/`wait_before`,timeline 点 `2v-1`/`2v`)裁定的偏序;弧外相对完成序不承诺,可见性
> 仍仅 pass 粒度。跨队列内存效应传递唯一经 timeline signal/wait + 成对 release/acquire(RFC-0019 §4.8.2);
> 成对律破缺 = 提交前确定性 RED。等价门(single-queue plan 同 digest)为该臂存在性的硬前置;条件不满足时
> 义务回落缺省形态,回落 evidence 显式标注。本臂不弱化缺省形态的任何字面;split barrier / pass 内重排 /
> concurrent write 的不存在性维持封死。

## 4. RED / GREEN(落地时 materialize)

| 面 | RED(先可复现) | GREEN |
|---|---|---|
| 成对律 | 漏 wait / 错值 / 半对 release / timeline 值回退注入 → 提交前 validator 拒 | 三 pass 异步图 device 双队列跑通 + validation 零报错 |
| 等价门 | 注入错依赖(去掉一条 fence 弧)→ digest 漂移可检出 | single vs dual digest 逐字节相等 + dual 双跑位级一致 |
| 回落 | 强制 `compute_only_family=None` 注入 → 仍走 dual = RED | 回落臂 digest 与缺省臂相等 + `single_queue_fallback` 标注在案 |
| 收益(evidence-only) | — | overlap_ratio / frame_ms 中位改善写 receipt,不进硬门 |

## 5. 备选方案(简)

| 方案 | 裁决建议 | 理由 |
|---|---|---|
| 不修 RXS-0239,多队列语义只落 rhi.md/rendering_platform.md(RP-MULTIQUEUE) | 否 | render_graph 面的封死措辞仍在,G5 图(uc06/harness 消费面)执行双队列即违 RXS-0239;两面须同批修订 |
| binary semaphore 网替代 timeline | 否 | RFC-0019 §4.8.2 已裁 timeline;binary 不支持 wait-before-signal、值域不可回收(TODO #59 价值面字面) |
| 双 timeline(graphics/compute 各一)替代 2v-1/2v 值域编码 | 缓 | #64 多 timeline 分轨为 P2 后续;首期单条最小面,判档不受影响 |
| 忽略 fences 直接单队列跑(不重编译)充回落 | 否 | 趟 3 屏障 stage 随车道推导,忽略 fence 的产物不是 §4.8.3 要求的显式 single-queue plan |

## 6. 登记程序(主 agent 正式路径)

1. M59 重判裁决:harness measured 证据齐(PLAN §2.5 两态)→ go 才继续,no-go 本草案留档;
2. 对抗性评审(D-409,跨模型)→ 修订行表定稿;
3. 条款 commit 先于实现 commit:`spec/render_graph.md` 落修订行表(RXS-0346 体例,既有条款 0-byte 声明)+
   RFC-0019 §5 RP-MULTIQUEUE 行 materialize(rhi.md/rendering_platform.md,实号自 ledger `next_free` 领取)+
   conformance 最小 RED 锚;
4. 实现 PR:vk.rs 加性面(PATCH_PROPOSAL 清单)+ 执行器 + validator;gate 归属沿 RFC-0019 §6.1
   (`queue_mode=multi` 分支),不新造 gate 命名空间。
