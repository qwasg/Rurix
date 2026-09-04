# G40 T3 — skin/dyn 生产规模 AS 副本内存 evidence 补登(施工 + 验收报告)

> 2026-09-01。留窗谱系 = G39 HANDOVER §D-7 前半句(「T2 生产 bistro 规模 AS
> 副本内存 = receipt/evidence notes 登记面」)。编辑面(独占)=
> `g14_3_lane_body.rs`;禁改 render_exec*(git 面复核零触碰)。验收 =
> `gpu_b2_t3.py` B2(B2_SUMMARY.json verdict=PASS)。

## 一、交付形态

- **账来源**(probe 机制直迁,fif_dyn_probe `slot_as_mem_from_ledger`
  同一过滤式):session telemetry 全量 allocation ledger 按 AS 表项
  `resource_id == resources_len + ai + 1` 过滤求和——逐表项含 instance
  buffer/BLAS 顶点/BLAS storage/TLAS storage/scratch 全部 vkAllocateMemory
  真账;首次 collect 采集一次(ledger 项 session 生命周期稳定)。
- **lane_body 加性面**:lane 字段 `slot_as_mem_bytes/slot_as_resources_len`
  + `capture_slot_as_mem`(collect_frame_dyn/collect_frame_skin 双半程
  出队后、rec 消费前采集)+ `slot_as_mem_receipt_json`(receipt 段构造)
  + bench receipt 格式串一个 `{}` 注入位。
- **off 面 0-byte**:非 slot_as 臂(inflight=1/静态/顺序)注入位 = 空串 ⇒
  receipt 字节逐位不变(B2 负控机核:inflight=1 receipt 无 `slot_as_mem`
  键);lane 字段 None 零成本。
- **口径分界(不混)**:预算门条目 `g31.fif_dyn.slot_as_group_mem_bytes`
  锚 **probe 场景**(44,544B measured / 66,816 threshold)0 动;本登记面 =
  生产 bistro 规模 receipt notes,不进预算判读(receipt note 字面自述)。
  budget_eval 维持 330 pass 0 skip(收役复核)。

## 二、B2 验收(全绿;b2_log.jsonl + B2_SUMMARY.json)

| 判 | 结果 |
|---|---|
| ① 字段在档(dyn rebuild/skin × inflight 1\|2\|3 各一跑) | per_slot_bytes 长度==inflight、全 >0、group_total==Σ;skin_verify 全 all_pass;VUID=0 |
| ② flip-trace 对 G39 在案件 | dyn_rebuild x1\|x2\|x3 对 `t3_fold/baseline/` + skin x1\|x2\|x3 对 `t2_skin/gpu/ft_x*_a` **六臂位级全等**(T3 改动对渲染语义零漂的机核) |
| ③ B1f 附录 | T2+T3 树新建窗口 bin:fc/base 双跑 == `5540ecae` 锚 MATCH(B1 ⑤ 二进制时序缺口闭合) |

**measured(生产 bistro 规模,tier100 120f 口径)**:

| 臂 | per_slot | group_total |
|---|---|---|
| dyn ×2 | 120,307,200B(114.7MiB) | 240,614,400B(229.5MiB) |
| dyn ×3 | 120,307,200B | 360,921,600B(344.2MiB) |
| skin ×2 | 120,310,656B | 240,621,312B |
| skin ×3 | 120,310,656B | 360,931,968B(344.2MiB) |

「数百 MB 级」预告(G39 t2_skin REPORT §D-5 字面)兑现;skin 略大于 dyn
(+3,456B/槽 = 角色 updatable BLAS 面差)。probe 场景(44,544B)与生产规模
差 ~2,700×,分口径登记的必要性自证。

## 三、登记

1. 采账点 = collect 半程(而非 create):ledger 经 frame output telemetry
   暴露,首帧 collect 即组恒值;排空段读面幂等。
2. `capture_slot_as_mem`/getter 挂 `#[allow(dead_code)]`(pipeline_perf 独
   消费面,include 方诚实标注律)。
3. receipt schema `rurix.g14.pipeline_perf_bench_receipt.v1` 字面维持——
   `slot_as_mem` 为加性键(receipt 非 check_schemas 路由面;off 面字节不变
   使既有消费者〔Stage A 探针读 last_frame_digest〕零影响)。
