# EVAL — bridge_ext × FIF rt 平行入口评估(G40 T4;纯文档零代码)

> 2026-09-01。留窗谱系:G38 `t2_fifdyn/WIRING_PLAN.md` §5-4「skin×bridge_ext
> ——skin slot_as 首兑走普通 blas_refit 路;bridge_ext×FIF 须 rt 平行入口,
> 留 T3/后续窗」→ G39 `t2_skin/REPORT.md` §8 兑现声明(全批零 bridge_ext
> 触碰,维持留窗)→ G39 HANDOVER §D-7 后半句。判档留 owner,本文不预支实施。
> 素材 = R3 侦察集(recon/R3_T3T4.md);源锚逐处标注。

## 1. 现状两入口(rt 在树事实)

| 入口 | 签名 | 形态 | 消费者 |
|---|---|---|---|
| `execute_with_frame_update_bridge_ext` | `(&prov, &update, Option<&BlasRefitBridgeExt>)`(render_exec.rs L1594) | **顺序单槽**同步执行;bridge = 多 region 脏拷贝 + 桥接 GPU 计时;`None` 与既有入口逐字同路径 | frame_cut 臂(G38 T3;G40 T2 P2 沿用) |
| `submit_with_frame_update_slot_as` | `(&prov, &update, &SlotAsGroup)`(render_exec_g37_fif_dyn.rs L168) | **FIF 流水**提交半程(票据/collect);每槽 AS 副本,`tlas_update`/`blas_refit` 目标落 `base + slot` 表项;槽纪律三判据提交前 fail-closed | dyn/skin slot_as 臂(G38 批次 A / G39 批次 B) |

缺口 = 两形态的笛卡尔积:**FIF 提交半程 × bridge 多 region copy** 无入口
——skin×FIF 现走 `FrameUpdate.blas_refit` 单 region 全量桥(G39 B1 已位级
PASS),frame_cut 的 incr 桥收益(75MB→脏差集,copy GPU 4.348→0.022ms,
G38 T3 measured)在 FIF 车道不可达。

## 2. 平行入口形态案

**形态 = 加性第四入口**(两既有入口 0-byte,单源主体复用):

```text
pub fn submit_with_frame_update_slot_as_bridge_ext(
    &mut self,
    supplied: &SubmissionProvenance,
    update: &FrameUpdate,
    group: &SlotAsGroup,
    bridge_ext: Option<&BlasRefitBridgeExt>,
) -> Result<FrameTicket, String>
```

- 语义合成律:`bridge_ext = None` ⇔ 既有 `submit_with_frame_update_slot_as`
  逐字等价(G38 bridge 入口对顺序面的 0-byte 先例同构);`Some` 时
  `blas_refit` 的单 region copy 换多 region(相对 refit 窗偏移,
  `validate_bridge_ext` 校验面直接复用),**目标 AS 恒 = `group.base +
  next_frame_slot()` 表项**(slot_as 槽纪律三判据逐字保持)。
- 实施路线(供开窗窗直接消费):G39 T3 单源折叠先例——
  `submit_pipelined_frame` 已收 `Option<&SlotAsGroup>` 末参;桥面同律折入
  (再加 `Option<&BlasRefitBridgeExt>` 末参,None 路逐字等价),两薄壳
  转发,复制体零新增。桥接计时 query 追加区形态照录(逐 pass 时戳口径
  不动)。
- 每槽 AS 副本提交面语义:copy 的 dst = 本槽 BLAS 的 vbuf(refit 窗),
  src = `update.blas_refit.src` 资源——**src 驻留形态是本案唯一新增语义
  约束**(§3-2)。

## 3. 决策语义分析(WP §4 模板对照)

1. **与 §4 NO-GO 场景结构相反**:bridge 的 copy_regions 由 host 当帧纯函数
   产出(frame_cut = cut 差集〔相机纯函数〕;skin = 蒙皮源段〔帧号纯函数〕)
   ——决策输入零 GPU 回读,无「上帧回读驱动本帧决策」的 host 在环反馈闭环
   (WP §4 原文判据)。FIF 化不产生 S−1 帧决策延迟语义——региons 与
   uploads 同帧同源。**语义面 GO 无障碍。**
2. **src 写面竞争 = 真约束(与 §4 无关的资源面)**:FIF 下 host 逐帧改写
   src(如 frame_cut arena SSBO 差集上传)时,上一帧在飞的桥 copy 可能尚在
   读同一 src——两解:①src 走 per-slot staging(FrameUpdate.buffer_uploads
   既有 per-slot 机制,G39 skin palette 双表先例)或 src 资源 ×S 副本
   (frame_cut arena 75MB×S 显式代价);②GPU-写 src(skin 蒙皮输出)零
   host 写面——队内 barrier 全序已保正确(L2 字面「GPU 帧间守卫 barrier
   全序维持」,G39 skin×FIF 位级 PASS 的同一根据)。
3. 判据面免重锚推导:regions 任意合法区段集 ⇒ vbuf 终态字节同(bridge 入口
   doc 字面「provenance 面不感知 regions」)⇒ 既有 digest 门(双跑/incr==
   full/跨臂)推导链 = G40 T2 §3.1 同构,平行入口开窗时直接消费。

## 4. 工程量与风险

| 项 | 量/裁决 |
|---|---|
| rt 面 | 折叠式 ~+80-150 行(`submit_pipelined_frame` 加桥末参 + 两薄壳;G39 T3 折叠先例当量以下) |
| 消费面 | lane_body skin/dyn submit 半程加桥参转发 ~+40 行;或 frame_cut 臂若入 FIF 车道(现无此形态,窗口 bin 不 FIF——WP §4 表字面) |
| 验收 | skin ×2\|3 bridge(full-region)digest == 普通 blas_refit 路位级(结构性必然的机核)+ incr 区段臂 vbuf 终态等价 + 槽纪律 red-arm 维持 + fif probe 回归 |
| 风险 1:src 写面竞争 | §3-2 两解;frame_cut 若入 FIF 须 arena ×S(75MB×S)显式代价预算登记 |
| 风险 2:桥计时 query 追加区 × per-slot cmd | query pool 每槽独立(session 既有形态),追加区随槽复制——实施时核 telemetry 槽归属 |
| 风险 3:收益面窄(诚实) | **skin 的脏区 ≈ 全量**(蒙皮逐帧全角色重写,incr≈full 无差集收益);frame_cut 是真稀疏消费者但其车道(probe/窗口臂)为顺序单槽非 FIF——当前无「稀疏 dirty × FIF」的真实消费者 |

## 5. GO/NO-GO 判档建议(判档留 owner)

**建议:机制 GO、开窗 DEFER(条件开窗)。**

- 机制面三根据全绿:①决策语义与 WP §4 NO-GO 结构相反(零回读反馈环)
  ②形态 = 既有两入口的加性合成,折叠式实施有 G39 T3 先例,当量 ≤ 0.5 窗
  ③判据推导链与 G40 T2 §3.1 同构,零重锚。
- **但当前收益面为空集**(风险 3):skin incr≈full,frame_cut 非 FIF 车道。
  建议开窗条件 = 首个「稀疏 dirty × FIF」真实消费者成立(候选:dyn 大场景
  局部破坏/程序化位移的 slot_as 车道,或 frame_cut 语义进 FIF 化的渲染
  车道——后者另涉 #77 P3 与窗口车道 FIF 化两独立前提)。在此之前实施 =
  无消费者的 rt 加性面,违「勿预支」纪律。
- 若 owner 判 GO 即刻实施:范围圈定 = rt 折叠 + skin bridge(full-region)
  等价门(位级)先行,incr 区段臂随真实稀疏消费者补验;编辑权 =
  render_exec* 独占窗(G39 T3 同律)。

*(本文档为 G40 T4 交付;零代码/零 schema/零 GPU/零 commit。)*

## 6. 判档登记(2026-09-04 owner 已回填;本段以上字面 0-byte)

> 体例照 G39 `t1_restir/LAMP_K_PROPOSAL.md` 判档登记段先例(事实链 §1-4 →
> 形态案 §2 → 建议 §5 → 本段判档)。**禁预填**:未判档前本段保持空判,
> 不得由 agent 代填 GO/DEFER/NO-GO,亦不得据 §5 建议推断为已判档。
> G40 `HANDOVER.md` §E-1 owner 治理窗挂本项(与「本役工作树入库」并列两件)。

**判档结论:**机制面 **GO**;开窗面 **条件 DEFER**。

- 机制面裁决:GO —— §5 三根据全绿(决策语义与 WP §4 NO-GO 结构相反、零回读
  反馈环;形态 = 既有两入口加性合成,G39 T3 折叠先例当量 ≤ 0.5 窗;判据推导链
  与 G40 T2 §3.1 同构,零重锚)。
- 开窗面裁决:条件 DEFER。开窗条件字面 = **「首个『稀疏 dirty × FIF』真实
  消费者成立」**;候选 = dyn 大场景局部破坏 / 程序化位移的 slot_as 车道,或
  frame_cut 语义进 FIF 化的渲染车道(后者另涉 #77 P3 与窗口车道 FIF 化两独立
  前提)。条件未成立前不实施 —— 无消费者的 rt 加性面违「勿预支」纪律。
- 若判即刻开窗:承接窗口 = 不适用(本次判条件 DEFER;条件成立时承接窗口由该役
  任务书指定);编辑权独占面 = `render_exec*`
  (G39 T3 同律);验收判据 = §4 表「验收」行(skin ×2\|3 bridge full-region
  digest 位级 + incr 区段臂 vbuf 终态等价 + 槽纪律 red-arm 维持 + fif probe 回归)。
- 判档人 / 日期:owner(白栀)/ 2026-09-04。

**未判档期间的合法动作**(纪律面,非建议):维持留窗谱系原样承接
——G38 `t2_fifdyn/WIRING_PLAN.md` §5-4 → G39 `t2_skin/REPORT.md` §8 →
G39 HANDOVER §D-7 后半句 → 本文档。不实施、不预支、不静默关窗。

**本判档闭合 G40 `HANDOVER.md` §D-9 与 §E-1 ②;本次仅回填本段判档字段,§1-§5 字面 0-byte。**
