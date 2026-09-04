# G40 T2 — device cut P2 生产 dispatch(#77;施工 + 验收报告)

> 2026-09-01。法定输入 = G39 `t5_devicecut/DESIGN.md` §2.7 P2 行 + §4-2;
> 开窗条件(P1 C1-C5 全绿)= G39 B5 在案。编辑面(独占)=
> `g31_frame_cut_arm.rs` + `g31_frame_cut_probe.rs` 两文件;kernel
> (`g31_cluster_cull.rx`)冻结 / rt / lane_body / 窗口 bin / schema 0-byte;
> 缺省 `--cut-source host` 字面 0-byte。验收 = `gpu_b1_t2.py` B1 六判
> (B1_SUMMARY.json verdict=PASS)+ B2 附录(窗口臂新树锚复验)。

## 一、交付形态(P1 对拍臂 → P2 决策码为源)

| 面 | P1(G39 B5) | P2(本役) |
|---|---|---|
| 决策权 | host `select_lod_cut_grouped`;device 平行复算判定码逐帧对拍 | **device 决策码为源**:host 由 `d==4` 构造 cut 布尔集 |
| dispatch 形态 | `vk::run_compute` 逐帧独立 device(82.7ms/帧上界,含设备创建) | **表驻留常驻 cull 会话**(`DeviceFrameSession::new` compute-only;簇表/lod 表/input_ids device_local 驻留 staging 上传一次,每帧仅 params 256B 上传) |
| 回读 | 判据面(决策码对拍后弃) | 生产路回读(决策码 n×4B ≈ 493KB/帧 → cut 集构造) |
| 校验 | 期望码逐项全等 + decisions∈{2,4} | `verify_cut_coverage` **host 影子核直跑回读集**(fail-closed 语义逐字保持,DESIGN §2.7 P2 行字面)+ decisions∈{2,4} 闭集(`frame_cut_sets_from_decisions` 内,附首破簇归因) |
| min-level 提升 | host | **照旧 host**(`frame_cut_select_from_decisions` = select_ext 后链字面同形,仅决策源换 device) |
| 施加链 | 0 改 | **0 改**(差集/上传/refit 逐字不动) |
| red-arm | lod 表篡改 ⇒ 期望码 mismatch 必红 | lod 表篡改 ⇒ 决策翻转 ⇒ **影子核覆盖性必破必红**(施加链真实消费 device 决策的构造性证明;受害裁决仍凭帧 0 host 参考码——诊断臂,生产决策源不回移) |

P3(直写竞技场)不预支;`frame_cut_device_cut_compare`(P1 本体)退役留档
(G39 B5 等价门 evidence 锚在其口径;10 buffer 布局被 cull 会话逐字继承)。

## 二、cut_ms 分项计时(DESIGN §4-2 登记义务兑现)

- `FrameCutSelectTiming { select_ms, verify_ms, promote_ms }` 加性尾参进
  `frame_cut_select_ext`(3 调用点机械补;既有语句字面 0 改,计时环绕追加)
  与 `frame_cut_select_from_decisions`;stat/evidence 逐帧恒出三分项 +
  `device_cut_dispatch_gpu_ms`(cull 会话 telemetry pass 0)。
- P1 字段 `device_cut_probe_ms` 随 run_compute 路退役不再发射(G39 B5 在案
  evidence 不回写,谱系各自完整);`device_cut_decisions_sha256` 保留
  (跨跑/跨窗审计面)。

## 三、B1 验收(六判全绿;b1_log.jsonl + B1_SUMMARY.json)

| 判 | 结果 |
|---|---|
| ① dev==host 位级 | 16f digest 逐帧全等 + 臂内建双跑 + 跨进程双跑(p2_dev == p2_dev_r2)全 OK |
| ② ×ml1 | PASS(提升前口径 × ml1 组合,digest 自洽双跑) |
| ③ incr==full + 跨锚 | dev_full == dev_incr 位级;**跨 G38 t3_incr + G39 t5_dev 双参考锚 MATCH**(P2 免重锚推导链 DESIGN §3.1 机核落地) |
| ④ red-arm | rc≠0 + stderr「red-arm 模式」+「覆盖性」(P2 报文形态变化如实登记:期望码 mismatch → 影子核覆盖性破) |
| ⑤ 0-byte 回归 | 缺省 host 臂 digest == device == G39 t5_host;窗口臂 5540ecae 锚 —— B1 首验用 W0 二进制(时序缺口如实登记),**B2 附录以 T2+T3 树新建二进制复验 MATCH,缺口闭合** |
| ⑥ 帧时 measured(登记不判红) | 见下表 |

| 口径(f1-15 均值,96x54/16f) | 值 |
|---|---|
| device select_ms(params 上传+dispatch+决策回读+集构造墙钟) | 4.992ms |
| device **dispatch GPU** | **0.113ms**(P1 run_compute 82.7ms 上界 → 三个量级降;分项 = 表驻留会话化收益本体) |
| device verify_ms(影子核)/ promote_ms | 3.183 / 0.0(ml0) |
| device cut_ms 合计 vs host cut_ms | 8.179 vs 7.874ms(ml0 下 select 墙钟被 fence/回读税抵消——诚实登记:ml0 无净收益,DESIGN §4-1「下沉不解预算」维持) |
| **ml1×device 组合墙钟(cut+delta+exec)** | **16.095ms/帧 —— 落 DESIGN §4-2 预期带 ~15-19ms**(对照 G38 t3_ml1 host 形态 23.7-27.3ms;ml1 device cut_ms 7.994 vs G38 host 14.0-15.2ms 近减半)。登记不判红,不冒充进预算(build 地板 8-11ms 仍在,90fps 叙事须 P3/更深组合) |
| ml0 墙钟 | device 29.088 / host 28.520ms(UPDATE build ~21ms 地板主导不变) |

## 四、偏离与登记

1. **`--cut-source device` 语义升级**(P1 对拍臂 → P2 生产源):同旗标语义
   换代,probe 文档头同步改写;P1 形态不可再跑(如需复现,G39 树 + B5
   evidence 在案)。
2. red-arm 报文形态变化(§三-④)。
3. counters 不逐帧清零:决策码逐输入项无条件写(kernel 头注「固定槽位,
   顺序无关对拍面」),原子计数仅门 vis/occ 列表追加写(本臂零消费),
   溢出丢弃语义(cap=n)不回染 decisions——审计根据登记于会话资源注释。
4. selftest ⑧ 段新增(决策码逆展平 + ml0/ml1 消费链与 host select 后链
   同判);既有 ①-⑦ 字面 0 改。
5. 工程量:arm +~420/−60、probe 文档 ±14(实测,DESIGN「P2 形态预登记」无
   行数当量承诺——P1 300±80 先例同量级)。
