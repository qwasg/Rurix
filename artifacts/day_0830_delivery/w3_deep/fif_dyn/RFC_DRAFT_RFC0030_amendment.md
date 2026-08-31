# RFC-DRAFT — RFC-0030 §4.3 L2 修订行草案:FIF 流水 × 动态 AS 更新的**每槽 AS 副本 opt-in** 语义(G37 W3 深水区起草,未登记;TODO #90)

> **状态更新(2026-08-30,G38 窗)**:已正式登记 → RFC-0030 v1.1(§4.3 L2a 条款行 = 本草案 §3 底稿逐字 + §9.2 v1.1 版本行;判档双 PASS 前置已兑,同目录 evidence_fif_dyn_rebuild.json / evidence_fif_dyn_refit.json)。本草案正文自此为档案面,0-byte 保留。

| 字段 | 值 |
|---|---|
| 修订对象 | `rfcs/0030-g14plus-pipeline-structural-optimization.md` **§4.3 L2(FIF=2)** 条款行(RFC-0030 状态 = Agent Approved,v1.0;修订按 §9.2 修订记录追加版本行,正式登记时领 v1.x) |
| 档位 | **修订行**(RFC-0030 §4.3 本身「无 spec 落点(运行时实现面…经契约 §8.x + G14PLUS_RECORD 承载)」——§5 映射表字面;故本修订**不触 spec/ 条款号**,落点 = RFC 条款行 + §9.2 版本行 + G14PLUS_RECORD/G31 TODO #90 行登记) |
| 状态 | **Draft(无效力)**。不进 rfcs/ 本体;落地前置 = `g31_fif_dyn_probe` GPU 判档 **PASS**(三臂 digest 等价门,§4)+ 主 agent 对抗性评审(D-409 同族程序) |
| 承接 | G36 五留窗「FIF×动态共存判档」/ G31+ TODO **#90**(字面:「FIF 拒 tlas_update/blas_refit,动态/蒙皮被迫顺序提交;真修复 = 每槽实例缓冲/BLAS 顶点副本 + 每槽 AS 描述符集;触 RFC-0030 §4.3 L2 共享 host 写面语义(冻结确定性协议)须 RFC 修订行」);关联 #89(FIF 波)/ #77(逐帧 AS 更新,frame_cut_as 兄弟窗) |
| 体例先例 | RXS-0346「🔒 唯一显式修订行表 + 既有条款字面 0-byte 声明」;G37 W3 async 窗 `RFC_DRAFT_RXS0239_amendment.md`(opt-in 加性臂 + 等价硬门 + 0-byte 清单五件套) |
| 实现底稿 | `src/rurix-rt/src/render_exec_g37_fif_dyn.rs`(body-include 加性面)+ `src/rurix-render/src/bin/g31_fif_dyn_probe.rs`(三臂判档 harness);实施记录 = 同目录 [REPORT.md](REPORT.md) |
| Provenance | `Assisted-by: cursor:claude-fable-5`(侦察 + 起草;主 agent 正式登记前不签批准行) |

---

## 0. 要点(≤5 条)

1. **既有拒绝面字面 0-byte 的 opt-in 加性臂**:§4.3 L2 现行实现(`submit_with_frame_update`)对
   `tlas_update`/`blas_refit` 的 fail-closed 拒绝(共享 host 写面在飞帧读取中不可改写)**字面不动**;
   仅当调用方显式声明**每槽 AS 副本组**(session AS 表内 `frame_slots` 份同构表项,组 `[base, base+len)`,
   `len == frame_slots`)并走新平行入口(`submit_with_frame_update_slot_as`)时,动态 AS 更新入流水。
2. **写面按槽分离三判据(提交前确定性 RED,不静默降级)**:①本帧 `tlas_update`/`blas_refit` 目标
   **必须 == 组 base + 本帧 slot**(错槽/组外一律拒);②本帧各 pass 绑定中凡落组内的 AS 引用必须 ==
   本槽表项(跨槽绑定拒;经既有 per-slot descriptor override set〔G31 A2〕逐帧轮换兑现「每槽 AS 描述符
   集」);③host 写(`write_transforms` 实例缓冲 memcpy)时序钉死在本槽 fence 等待**之后**(本槽上一
   票据已完成 ⇒ 该副本无在途 device 读——与 §4.3 L2 既有 per-slot staging 复用纪律同一根据)。
3. **确定性协议保持(冻结字面的扩展而非弱化)**:§4.3 L2「逐帧 digest 序列与 FIF=1 全等」对本臂维持——
   每槽副本下逐帧 AS 内容 = 纯函数(本帧实例/顶点数据)(`Rebuild` 直给;`Refit` = f(创建期拓扑,
   本帧数据),组内表项同构创建 ⇒ 拓扑同)⇒ 固定轨迹逐帧 digest 序列与单槽顺序提交**逐字节相等**;
   判档硬门 = probe 三臂(顺序 / FIF=2 / FIF=3)digest 逐字节相等 ∧ 各臂双跑位级 ∧ validation=0。
   `Refit` 臂若实测暴露驱动 refit 非纯函数,回退判据 = 「按槽稳定」(同臂双跑位级 + 语义命中等价),
   须 evidence 显式标注、不充逐字节绿。
4. **GPU 帧间守卫 barrier 全序字面不动**:本臂不改 per-slot cmd 首条全局守卫 barrier(§4.3 L2 确定性
   论证本体)——AS build 命令经**同一录制事实源**(`record_frame_body` as_ops:TLAS build 录 pass 链前、
   BLAS refit 桥录 after_pass 后,与顺序入口逐字同形)落在守卫 barrier 之后;FIF 收益维持「CPU
   record/submit/fence 解耦」口径,GPU 帧间重叠仍不在承诺面。
5. **0-byte 清单(不动声明)**:既有 `submit_with_frame_update`/`collect`/顺序 `execute*` 全部入口与
   拒绝措辞 / `FrameUpdate` 布局 / per-slot cmd·staging·query·override-set 建面 / 双 TLAS(`tlas_update_b`)
   与双 BLAS(`blas_b`,G37 W3 hzb_skin 并行窗)面(本臂恒不开放,顺序入口专属)/ vk.rs `VkAsManager`
   单所有者纪律(组内每表项各自单所有者,禁第二 BVH)——全部字面 0-byte。内存代价(AS 面 ×frame_slots:
   instance buffer/BLAS/TLAS/scratch/顶点缓冲副本)为本臂显式 opt-in 成本,evidence 登记。

## 1. 修订对象与现行字面

- 对象:`rfcs/0030-g14plus-pipeline-structural-optimization.md` §4.3「readback 内存型与 FIF 流水结构面」
  **L2(FIF=2)** 行(2026-08-30 工作树 L97)。
- 现行字面:「加性 API `submit_persistent_frame`(至 vkQueueSubmit 止,含 slot-reuse bounded wait)+
  `collect_persistent_frame`(当帧 wait + query + readback 后移);**per-slot cmd/params/descriptor/query/
  readback 双缓冲**;既有 `execute_persistent_frame` = submit+collect 顺序调用等价形态 0-byte 保留(既有
  消费方零漂移);**数据依赖正确性 = 逐帧 digest 序列与 FIF=1 全等(500 帧压测机核)**。」
- 实现面现状(侦察在案,REPORT §1):L2 的 per-slot 枚举**不含 AS/实例缓冲**——公共入口
  `submit_with_frame_update` 遂对 `tlas_update`(render_exec.rs「TLAS instance buffer 为共享 host 写面,
  在飞帧读取中不可改写」)与 `blas_refit`(「BLAS 顶点缓冲为共享写面,在飞帧 ray query 读取中不可改写」)
  fail-closed;消费面 CLI 强制 `--dyn-demo`/`--skin-demo` 随 `--inflight 1`。**本修订行即把 per-slot
  枚举扩展到 AS 面(opt-in),拒绝面对未 opt-in 调用维持字面。**

## 2. 🔒 唯一显式修订行表

| # | 修订行 | 性质 |
|---|---|---|
| 1 | §4.3 L2 追加「**L2a(FIF×动态,每槽 AS 副本 opt-in)**」子行:缺省流水形态承诺与拒绝面字面 0-byte;opt-in 臂三条件 = ①session AS 表含显式声明的每槽副本组(`len == frame_slots`,组内表项同构创建)②动态更新与组内绑定逐帧落 `base + slot`(提交前确定性 RED 三判据:错槽更新/组外更新/跨槽绑定)③host 实例写钉死在本槽 fence 等待之后。条件不满足 → 确定性 `Err`(义务回落顺序入口,非静默降级) | 加性子行 |
| 2 | L2a 确定性判据:固定轨迹逐帧 digest 序列与**单槽顺序提交逐字节相等**(L2「与 FIF=1 全等」冻结协议对动态臂的延伸;`Rebuild` 硬门);`Refit` 臂 = f(创建期拓扑, 本帧数据)预期同等,实测非纯时回退「按槽稳定」判据并 evidence 显式标注(不充逐字节绿) | 加性判据 |
| 3 | L2a 机制钉死:每槽副本 = session AS 表同构表项(每表项独立 instance buffer/BLAS 顶点缓冲/BLAS/TLAS/scratch,`VkAsManager` 单所有者纪律逐表项维持);每槽 AS 描述符集 = 既有 per-slot descriptor override set(G31 A2)× `binding_overrides` 逐帧轮换,**零新描述符基建**;AS build 录制 = 与顺序入口同一 `record_frame_body` 事实源,落守卫 barrier 后 | 机制行 |
| 4 | 0-byte 清单:守卫 barrier 全序论证 / 既有全部入口与拒绝措辞 / `FrameUpdate` 布局 / 双 TLAS·双 BLAS 面(本臂不开放)/ readback per-slot staging 纪律 / §4.3 L1·L3 全文——字面 0-byte;GPU 帧间重叠、蒙皮生产车道接线(`--skin-demo` × FIF)、内存预算门(副本 ×S 成本上线)不在本修订,留后续窗 | 0-byte 声明 |

## 3. 条款措辞草案(落 RFC 时的追加文本底稿)

置于 §4.3 L2 行之后,同体例:

> **L2a(FIF×动态,每槽 AS 副本 opt-in;G37 W3 #90 修订行)**:L2 的 per-slot 双缓冲枚举扩展到 AS 面
> ——调用方在 session AS 表显式声明 `frame_slots` 份同构副本组,经加性平行入口
> (`submit_with_frame_update_slot_as`)逐帧把 `tlas_update`/`blas_refit` 与组内 AS 绑定轮换到
> `base + slot` 表项(每表项独立 instance buffer/BLAS 顶点缓冲/BLAS/TLAS/scratch;每槽 AS 描述符集经
> per-slot override set 既有基建)。写面按槽分离:host 实例写序于本槽 fence 等待之后,错槽更新/组外
> 更新/跨槽绑定 = 提交前确定性 RED。缺省流水形态的 `tlas_update`/`blas_refit` 拒绝面与守卫 barrier
> 帧间全序**字面不动**;L2 确定性协议对本臂维持——固定轨迹逐帧 digest 序列与单槽顺序提交逐字节相等
> (`Rebuild`;判档机核 = `g31_fif_dyn_probe` 三臂等价门),`Refit` 非纯实测时按槽稳定判据显式降档
> 登记。副本内存成本(AS 面 ×frame_slots)为 opt-in 显式代价,evidence 登记;GPU 帧间重叠仍不在承诺面。

## 4. RED / GREEN(判档 harness 已落,GPU 真跑归主 agent)

| 面 | RED(先可复现) | GREEN |
|---|---|---|
| 槽纪律 | 错槽 `tlas_update` / 组外更新 / 跨槽绑定注入 → 提交前确定性拒(probe device 腿 RED 双臂 + rt 单测 `g37_slot_as_red_arms` + `--selftest`) | 本槽更新 + 本槽绑定 + 组长 == frame_slots 全过 |
| 等价门 | B/C 任一帧 digest ≠ A(首异帧号落 evidence)→ exit 1 整窗不判收益 | A(顺序)/ B(FIF=2)/ C(FIF=3)逐帧 digest 逐字节相等 ∧ 三臂各双跑位级 ∧ validation=0 ∧ 动态见证(逐帧双实例命中 >0 + 序列非常量 + 哨兵零残留) |
| 收益(evidence-only) | — | wall_ms / cpu_fence_ms 中位 A vs B/C 对照写 evidence,不进硬门 |

## 5. 备选方案(简)

| 方案 | 裁决建议 | 理由 |
|---|---|---|
| 单份 AS + 帧内双份 instance buffer ping-pong(不复制 BLAS/TLAS) | 否 | TLAS build 本身原地写 TLAS——在飞帧 ray query 读同一 TLAS 对象,写面未分离(守卫 barrier 只保 GPU 序,host 写 instance buffer 的竞争仍在;且 blas_refit 原地 UPDATE 同题) |
| 逐帧 QueueWaitIdle 后再写(伪 FIF) | 否 | 等价于顺序提交,FIF 收益归零——#90 字面「被迫顺序提交」即现状 |
| 把守卫 barrier 降级为逐资源精确屏障以换 GPU 帧间重叠 | 缓 | 触 §4.3 L2 确定性论证本体(全序化是 digest 全等的机制根据),独立修订行另议;本臂不依赖 |
| 每槽副本组由执行器隐式创建(调用方无感) | 缓 | 内存 ×S 成本须显式 opt-in(AS 面在 bistro 级场景数百 MB);隐式化留生产接线窗裁决 |

## 6. 登记程序(主 agent 正式路径)

1. GPU 判档:REPORT §6 命令跑 `g31_fif_dyn_probe`(rebuild 硬门 + refit 对照臂)→ PASS 才继续,
   RED/no-go 本草案留档;
2. 对抗性评审(D-409 同族)→ 修订行表定稿;
3. RFC commit:`rfcs/0030-…md` §4.3 追加 L2a 行(§3 底稿)+ §9.2 修订记录版本行;G14PLUS_RECORD /
   G31 TODO #90 行同批登记(实现已在树:render_exec_g37_fif_dyn.rs + probe);
4. 生产接线(后续窗,非本修订):`--dyn-demo` × `--inflight 2|3` 消费面解锁(CLI fail-closed 措辞随
   L2a 改写)、蒙皮车道(`blas_refit` × 每槽顶点副本)接线、内存预算门。
