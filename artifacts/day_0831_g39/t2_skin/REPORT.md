# T2 批次 B 施工报告 — FIF×蒙皮 skin 臂（TODO #90;G39 战役 T2 段）

- 日期:2026-08-31。实施依据:`artifacts/day_0830_g38/t2_fifdyn/WIRING_PLAN.md` §1-A6 + §2-B2/B3;交接单 `artifacts/day_0831_g39/recon/R3_T2T3.md`。
- 改动文件(共 2,均在授权面内;**未 commit**):
  1. `src/rurix-render/src/bin/g14_3_lane/g14_3_lane_body.rs`(+704/−120 行,histogram diff 存档 `diff_lane_body.patch`)
  2. `src/rurix-render/src/bin/g14_3_pipeline_perf.rs`(+27/−13 行,`diff_pipeline_perf.patch`)
- **零 include 消费方补参**:本批次未变动任何既有共享签名(`frame_skin` 签名逐字保持,新方法全部加性)——批次 A 的 `g35_particle_lane.rs` 类补参本批不需要。
- **cargo check:0 error,0 新增 warning**(全文 log = `cargo_check.log`;详见 §7)。

---

## 1. A6-1 — `frame_skin` rec 组装段提取 `skin_rec_from_output`

拆分策略(diff 最小化):`frame_skin` 的 doc+签名**原位改写**为 `prepare_update_skin` 的 doc+签名,其下的构造段/rec 组装段**物理原位 0-byte 不动**,仅在两段接缝处落刀;`frame_skin` 以薄封装形态(原 doc 逐字恢复 + 提取注记)重建于 `skin_rec_from_output` 之后。故 git diff 中构造/组装两段全为 context 行(逐字搬移可机械核验)。

- 替换前锚(rec 段头,原 L10973-10974):
  `// rec 组装（telemetry 五 pass 逐名提取;回读按子集构建序解析）。` + `let gpu = |name: &str| -> Result<f64, String> {`
- 替换后:该注释吸收进 `skin_rec_from_output` doc;`let gpu = …` 起至 `readback_convert_ms: t_convert.elapsed()…` 全部 0-byte。
- 替换前锚(rec 段尾,原 L11029/L11045-11047):`let rec = SkinFrameRec {` … `};` + `self.advance(vp_j);` + `Ok(rec)`
- 替换后:`Ok(SkinFrameRec {` … `frame_index: 0,` `})`(尾字段加性,见 §3);`execute_with_frame_update`/`advance`/`Ok(rec)` 移入薄封装 `frame_skin`(调用点 1 行替换形态:`prepare_update_skin(…, None)` → `execute` → `skin_rec_from_output(out, readback_out, verify, debug_tris, ow, oh, iw, ih)` → `advance`)。
- 签名 = 任务书指定形:`fn skin_rec_from_output(&self, out: DeviceFrameOutput, readback_out: bool, verify: bool, debug_tris: bool, ow: u32, oh: u32, iw: u32, ih: u32) -> Result<SkinFrameRec, String>`。禁复制第二源遵守:原内联组装即日废除,树内唯一事实源 = 本 helper(顺序/FIF 两面共用)。

## 2. A6-2 — update 构造段 `scene_as_override: Option<u32>` 分叉(skin scene pass = **pass 1**)

- 构造段(原 frame_skin L10874-10971)提取为 `fn prepare_update_skin(&self, …〔frame_skin 原 16 参〕, scene_as_override: Option<u32>) -> Result<(SubmissionProvenance, FrameUpdate), String>`(`&mut self`→`&self`;prov 必须在构造器内派生——与批次 A `prepare_update_ext` 同理由:构造后改绑定必致 provenance 校验 RED)。
- 替换前锚(原 L10960-10966):`binding_overrides: vec![\n (3, bindings_resample),\n (4, bindings_resolve),\n ],`
- 替换后:hoist 为 `let mut binding_overrides = vec![(3, …), (4, …)];` + `if let Some(as_index) = scene_as_override { let mut b = self.skin_scene_bindings.clone().ok_or(…)?; b.accel_structs = vec![as_index]; binding_overrides.push((1, b)); }`——**override 推 `(1, b)` 非 `(0, b)`**;既有 (3,resample)/(4,resolve) 两项字面不动;None 路产物与原内联构造逐字段同(0-byte)。
- **pass-1 bindings 取得方式(本批关键决策)**:批次 A 的 `scene_bindings` 按 scene pass=0 形建(`prepare_update_ext` 硬编码推 `(0, b)`),不可复用——采任务书授权的「存新字段」方案,`UnifiedTsrLane` 加性字段 `skin_scene_bindings: Option<Bindings>`,在 `create_with_slot_as` 内以 `matches!(descs, UnifiedDescs::MegaSkin(_))` 判定、自 `passes.get(1)` 克隆(非 compute fail-closed:`"MegaSkin descs 第 2 pass 非 compute（skin scene pass 门面）"`);非 MegaSkin 恒 None。既有 `scene_bindings` 计算行与赋值行逐字未动 ⇒ **dyn 臂字节语义 0 变**(创建期该字段对 MegaSkin 仍存 pass0〔g31_skin,无 AS 绑定〕克隆,无消费方,惰性数据无行为面)。
- 创建期锚:skin scene pass `g31_skin_scene` 创建期 `accel_structs: vec![0]`(现 L14246-14251 附近)**0-byte 未动**——组内绑定纪律由 rt 侧 `g37_validate_slot_as_frame` 三判据核(base+slot 恒等于逐帧 override 值)。

## 3. A6-3 — `submit_frame_skin_slot_as`(+ `PendingSkinFrame`/`pending_skin_len`/`collect_frame_skin`)

- 形同 `submit_frame_dyn_slot_as`(置于 `collect_frame_dyn` 后,impl 收口前;三件均 `#[allow(dead_code)]` 诚实标注 g14_3_pipeline_perf 独消费面——批次 A 同型注解)。
- 差异恰如计划:**无 tlas_update**;`blas_refit` 目标逐帧换槽——`let target = group.base + slot;` + `let blas = BlasRefitUpdate { as_index: target, ..blas };`(`BlasRefitUpdate: Copy`,functional-update 语法保证 blas_index/src/src_offset/byte_len/after_pass 五字段与顺序臂调用方字面同源直传);`prepare_update_skin(…, Some(target))` 同时落 scene pass 1 绑定 override——**blas_refit 目标与 AS 绑定同槽同帧**,rt 入口 `submit_with_frame_update_slot_as` 的 `g37_validate_slot_as_frame` 已核 `update.blas_refit.as_index == base+slot`(render_exec_g37_fif_dyn.rs L193,在树事实,本批零 rt 改动)。
- AS 副本组创建期每表项 `updatable_blas: &SKIN_UPDATABLE_BLAS`(=[1],角色 BLAS 打标 ×S 份,见 §4);palette/params uploads 走既有 `buffer_uploads` → rt per-slot staging,**零改动**。
- `PendingSkinFrame { ticket, frame_index, readback_out, verify, debug_tris }`(置于 `PendingDynFrame` 后);`collect_frame_skin(ow,oh,iw,ih)` = FIFO 出队 → `session.collect` → `skin_rec_from_output`(verify/debug_tris 随票据)→ `rec.frame_index = p.frame_index` 帧号回填。
- 承载字段:`SkinFrameRec` 尾部加性 `frame_index: u32`(顺序面恒 0 不被消费——顺序 flip-trace 行号取循环下标,0-byte;唯一构造点 = `skin_rec_from_output`,全库 grep 无其它构造点〔`g34_skin_section.rs` 的 `G34SkinFrameRec` 为异名异构不涉〕)。

## 4. A6-4 — skin bench 循环 FIF 分支

- **AS 副本组构造**(替换前锚 = `const SKIN_UPDATABLE_BLAS: [u32; 1] = [1];` 后的单表项 `let accel_structs = [AccelStructDesc {…}];` + `let mut lane = match UnifiedTsrLane::create(&descs, &accel_structs, 1) {`,原 L16685-16693):替换为 dyn 臂同形 `slot_as_copies` 副本 vec(每表项 `updatable_blas: &SKIN_UPDATABLE_BLAS`)+ `if inflight > 1 { create_with_slot_as(…, inflight as usize) } else { create(…, 1) }`。`SKIN_UPDATABLE_BLAS` 常量行 0-byte。`blas` 模板构造(`BlasRefitUpdate{as_index:0, blas_index:1, src:U_TRIS+1, src_offset:skin_tri_base*36, byte_len:tri_count*36, after_pass:0}`,原 L16697-16704)0-byte——FIF 路在 submit 内换槽覆写 as_index。
- **核验组装提取**:原顺序循环内联核验块(`if verify { let scene_color = rec… … verify_recs.push(SkinVerifyFrame{…}); }`,约 267 行)逐字迁入循环前闭包 `push_verify = |verify_recs, rec: &SkinFrameRec, i: u32|`(**体内缩进原位保持**,diff 可逐行比对);帧号纯函数复算件:`j/vp_j`(halton∘jittered_vp)、`pal = skin_palette(i, origin)`、`prev = if i==0 {pal} else {skin_palette(i-1, origin)}`(= `prev_pal.unwrap_or(pal)` 同语义)、`prev_vp_h = if i==0 {vp_j} else {jittered_vp(halton(jitter_base+i,…))}`(= 原 `prev_vp_host.unwrap_or(vp_j)` 同语义;核验帧恒 i≥1,i=0 分支为防御性镜像)。顺序循环调用点 = `if verify { push_verify(&mut verify_recs, &rec, i); }`(注释追加同一事实源注记)。
- **FIF 循环分支**:顺序循环整段外包 `if lane.inflight > 1 { FIF } else { 既有顺序循环 }`——else 内顺序循环**缩进原位、字面 0-byte**(批次 A/静态 A2 同式;git diff 因块移动显示为删+增,`diff_lane_body.patch` 逐行核对为字节恒等,仅两处例外见 §6-c)。FIF 循环骨架逐字镜像 dyn slot_as FIF 分支(L16280-16451 形),差异四处 = submit 换 `submit_frame_skin_slot_as`(palette 双表/skin_params/debug_tris 随臂)、`pending_skin_len`/`collect_frame_skin`、collect 后 `if rec.scene_color.is_some() { push_verify(…, rec.frame_index) }`、测量样本追加 `skin_probe_ms`(map_or 0.0 同律)。`prev_pal` 循环态维持(palette 上传消费);排空段 while 收干 + `drain_wait_ms/drain_tail_ms` 并入末一测量样本——**`prod = frame − tail` 不变式与静态 A2/dyn FIF 登记口径逐字一致**。
- **receipt**:第一元组元素改条件式——`inflight=1` 走既有字面 `.to_owned()`(逐字保留);`inflight>1` = slot_as 形态如实登记(格式串含 ×S 副本组/updatable 打标/base+slot/逐帧 digest 序列判据/refit 非纯 L2a 降档预案)。timer/caliber 两串 0-byte。
- skin_verify 落盘与 all_pass 判定(含窗级真动门 `motion_max >= SKIN_MV_HOST_MOTION_MIN_PX`)位于分支之后的共享段,**0-byte 未动**,对 FIF 分支自然生效(verify 帧集合与顺序臂完全同:`i>=1 && i>=warmup && (i-warmup)%DYN_VERIFY_EVERY==0`,提交期判定随票据)。

## 5. B2 — CLI skin fail 块解除 + **锚过期修正登记**

- **锚过期登记(判读器口径修正)**:WIRING_PLAN §2-B2 所载「替换前」字面(旧 L639-641,`"--skin-demo 要求 --inflight 1（A2 同律约束：FIF 流水入口拒 blas_refit——BLAS 顶点缓冲为共享写面,在飞帧 ray query 读取中不可改写；蒙皮车道走顺序入口）"`)**已过期**——批次 A 落地时该块曾按计划改写为「批次 B 留窗」措辞。本批以**在树现行字面为准**(R3 交接单口径,实测 L648-650):
  `"--skin-demo 要求 --inflight 1（蒙皮 × slot_as 批次 B 留窗：RFC-0030 v1.1 §4.3 L2a 通路 rt 侧已支持〔blas_refit 槽纪律同律〕,g14_3 接线计划 = artifacts/day_0830_g38/t2_fifdyn/WIRING_PLAN.md §1-A6;接线前 fail-closed 维持,蒙皮车道走顺序入口）"`
- 替换后 = B1 体例登记注释(现 L650-657):措辞把 `tlas_update` 换 `blas_refit`,注「角色 BLAS 顶点副本 ×S,updatable 打标逐表项」与 base+slot 换槽;**fail-closed 语义保持**——非法组合(非 bench/非 tsr_device/非 bistro/与 dyn 互斥/gi/profile)全部既有拒绝面不动,合法组合语义门 = lane `create_with_slot_as` 显式条件 + rt 三判据。
- **闭集注释②同步改写(登记)**:L634-643 原「② inflight 恒 1——批次 B 留窗…接线前本拒绝面维持」已随兑现失效,改写为 dyn 臂③同体例「② inflight 1|2|3——1 = 顺序入口既有面 0-byte;2|3 = L2a 批次 B 每槽 AS 副本 opt-in FIF(…预算门 g31.fif_dyn.slot_as_group_mem_bytes;--warmup ≥ inflight−1 通则已覆盖填充段)」。

## 6. B3 + 附带字面修正(均登记)

a. `--inflight` 帮助字面(L580;B1 未曾追加,本批兑现):追加 `〔动态/蒙皮臂 = L2a 每槽 AS 副本 opt-in〕`(嵌套括号取〔〕体例)。
b. **计划外最小字面修正**:L585 `--inflight` 三通则 fail 消息的消费面清单 `"…G38 L2a 动态臂〔--dyn-demo〕消费面…"` → `"…G38 L2a 动态/蒙皮臂〔--dyn-demo/--skin-demo〕消费面…"`——skin 兑现后原清单即失实(判读器口径同类修正,与 B2 同性质;fail 消息字面,零行为)。
c. **计划外必要删除**:lane_body skin bench 顺序循环态 `prev_vp_host`(声明 + `prev_vp_host = Some(vp_j);` 两行)——唯一消费方(核验块)迁入闭包后凭帧号复算,死状态置留即 `unused_variables`/`unused_assignments` 两条新告警(首轮 cargo check 实测),删除 = 零行为变化(写后无读)。原位留注释登记。`prev_pal` 循环态保留(palette 上传仍消费)。
d. `frame_skin` 重建 doc 尾部追加两行提取注记(原 doc 十行逐字保留;其「本车道恒 inflight=1,CLI fail-closed 保证」措辞与 frame_dyn 同型保守——两臂顺序入口在 CLI 解除后事实上仅 inflight=1 路径调用,批次 A 先例同样未改 frame_dyn 措辞,守对称字面纪律)。

## 7. cargo check 结果

```
$env:CARGO_TARGET_DIR="H:\rurix\target-night"
cargo check --release -p rurix-render --features vendor-upscale --bin g14_3_pipeline_perf
→ Finished `release` profile [optimized] target(s)（全文 = cargo_check.log）
```

- **error 0;rurix-render(bin g14_3_pipeline_perf)新增 warning 0**。
- 既有 warning:`rurix-rt` lib 17 条(vendor_upscale.rs/vk_m50_rt_body.rs/vk.rs/vk_g31_ser_body.rs——本批禁区未触碰,缓存重放,不计)。
- 首轮曾出 2 条新告警(§6-c 的 prev_vp_host 死状态),已按上述删除收敛,复检归零。

## 8. 纪律遵守声明

- **与 T3 `bridge_ext` 不混(§5-4)**:skin slot_as 首兑走**普通 `blas_refit` 路**——提交面唯一入口 = 既有 `submit_with_frame_update_slot_as(&prov, &update, &group)`(FrameUpdate.blas_refit 单 region 全量桥);全批 **零** `execute_with_frame_update_bridge_ext`/`BlasRefitBridgeExt` 触碰,**零 `src/rurix-rt/**` 改动**(git status 复核:rt 树无 M 项)。bridge_ext×FIF 须 rt 平行入口,维持留 T3/后续窗。
- 禁区 0 改动:`render_exec*.rs`/`g31_window_present.rs`/`kernels/**`/`src/rurix-render/src/gi/**`/`ci/**`/milestones——git diff 仅上列 2 文件(工作树中另有 g31_realism.rx/g35_render_*.rx/g35_particle_lane.rs/ci/milestones 的 M 项为**并行窗改动,非本批**,未触碰)。
- 禁 GPU 真跑:遵守(0 次 exe/--bench 执行);禁 git commit:遵守。
- 既有行字面:dyn 臂/静态臂/skin 顺序循环逐锚复核未被动(histogram diff 存档,块移动区逐行字节恒等;例外仅 §6-c 两行删除与核验块调用点替换,均登记)。

## 9. Refit 非纯降档预案(L2a 引用)

- 语义登记:skin BLAS refit 的**槽副本历史 = k−S**(slot s 的 BLAS 上次 UPDATE 在 k−S 帧;refit 顶点源恒为当帧蒙皮输出,几何内容与顺序臂同帧同值,但驱动 UPDATE build 结果可依赖前态 ⇒ 副本间收敛路径可分歧)。probe refit 对照臂已双 PASS 逐字节(判档前提);若 g14_3 生产臂实测非纯(x2/x3 digest 序列 ≠ x1 顺序基线,但各臂双跑位级绿 + skin_verify all_pass),**按 RFC-0030 v1.1 L99 字面「Refit 非纯实测时按槽稳定判据显式降档登记」处置,不充逐字节绿**;登记先例 = `ci/gpu_batch1.py` L231(eq_double 成立而 eq_across 破时不计 FAIL,note 登记)。receipt 的 inflight>1 lane 描述串已内嵌该预案字面。

---

## 10. GPU 验收命令清单(主 agent 锁内跑;子 agent 禁 GPU 已遵守)

```powershell
$env:CARGO_TARGET_DIR = "H:\rurix\target-night"
$env:RURIX_REQUIRE_REAL = "1"; $env:RURIX_VK_VALIDATION = "1"
# 臂 x ∈ {1,2,3}；每臂双跑(r ∈ {a,b})；--warmup 10 ≥ inflight−1 通则满足
$env:RURIX_G14_FLIP_TRACE = "artifacts\day_0831_g39\t2_skin\gpu\ft_x<x>_r<r>"
cargo run --release -p rurix-render --features vendor-upscale --bin g14_3_pipeline_perf -- `
  --bench --backend tsr_device --scene bistro-interior --tier 100 `
  --frames 120 --warmup 10 --skin-demo --inflight <x>
# 建议加 --out-root artifacts\day_0831_g39\t2_skin\gpu\out_x<x>_r<r> 隔离逐跑产物
```

### 落盘路径(本批实施后)

| 产物 | 路径 | 说明 |
|---|---|---|
| 逐帧 digest 轨迹 | `<RURIX_G14_FLIP_TRACE>/frame_digests_bistro-interior_t100_tsr_device.jsonl` | FIF 路行号 = `rec.frame_index`(票据回填,FIFO 保序 ⇒ 与顺序臂行序同构可逐字节 diff) |
| skin 核验 | `<out_root>/bistro-interior/tier100/tsr_device/skin_verify.json` | schema `rurix.g31.skin_verify.v1`;默认 out_root = `K:/rurix-ext/g14-frames/rurix_prod`(建议 --out-root 显式隔离) |
| bench receipt | `<out_root>/bistro-interior/tier100/tsr_device/bench_receipt.json` | `inflight` 字段 + lane 描述串(x>1 = slot_as 形态登记,含 L2a 降档预案字面) |
| skin GPU 段 | stderr `…: SKIN_GPU_MS mean=… min=… max=…` 行 | FIF 下样本为 collect(k−S+1) 帧值(map_or 口径;warmup≥S−1 ⇒ 测量窗无空样本) |

### 判读要点

1. **等价环(硬门)**:`fc.exe /b` 三臂 flip-trace jsonl 两两逐字节(x1≡x2≡x3;逐帧序列门强于末帧门)+ 各臂双跑位级(同臂 a/b 逐字节)。若 x2/x3 ≠ x1 而双跑稳:进 refit 非纯降档流程(§9——「按槽稳定」显式登记,不计 FAIL 不硬凑)。
2. **skin_verify all_pass(硬门)**:进程内 fail-closed(非 all_pass 即 exit≠0);**窗级真动门** = `motion_gate.host_motion_max_px ≥ 1.0`(标定口径 = 100 帧窗;`--frames 120 --warmup 10` ⇒ 核验窗 (i≥max(1,10), 每 DYN_VERIFY_EVERY 帧) 满足)。FIF 下核验帧集合与顺序臂完全同(提交期判定),palette/相机凭帧号复算——`palette` 字段可跨臂逐字节 diff 佐证复算同源。
3. **VUID=0(硬门)**:`RURIX_VK_VALIDATION=1` 下 bench 逐帧 `validation_error_count != 0` 即 fail(FIF 提交/排空两路均已接同门)。
4. **refit 槽副本历史 k−S 语义注意点**:x=2|3 时每槽 BLAS 的 UPDATE 前态为 k−S 帧几何——顶点位移幅度 ×S 于顺序臂;这不改 refit 合法域(拓扑/顶点数不变),但为非纯分歧的敏感放大器,判读时先看首个分歧帧号是否为槽序周期(k mod S 簇状)以归因。
5. **AS 副本内存(evidence notes)**:bistro 生产规模角色+全场景 BLAS/TLAS/instance/scratch ×S(数百 MB 级)如实入 receipt/evidence notes;预算门条目 `g31.fif_dyn.slot_as_group_mem_bytes` 锚 probe 场景,**生产规模登记面另行 evidence,不混口径**。
6. 配套(R3 GPU 验收段既定):`g31_fif_dyn_probe` 双臂 + `ci/calibrate_fif_budget.py --check`;skin ×1 臂 digest 应与批前顺序基线逐字节等(顺序路 0-byte 的实证锚)。
