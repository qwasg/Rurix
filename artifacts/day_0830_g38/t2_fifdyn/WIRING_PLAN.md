# WIRING_PLAN — g14_3 生产接线 FIF×动态(slot_as)精确 edit 计划(G38 T2 段 2;**设计文档,本窗零代码**)

> 日期:2026-08-30。前置已兑:RFC-0030 **v1.1 §4.3 L2a** 已正式登记(判档双 PASS = `g31_fif_dyn_probe` 三臂等价门 rebuild/refit);rt 加性入口 `submit_with_frame_update_slot_as` / `SlotAsGroup` / `g37_validate_slot_as_frame` 在树(`src/rurix-rt/src/render_exec_g37_fif_dyn.rs`,body-include);预算门条目 `g31.fif_dyn.slot_as_group_mem_bytes`(estimated 占位)+ 标定脚本 `ci/calibrate_fif_budget.py` 在树。
>
> **行号快照声明**:`g14_3_lane_body.rs` 与 `render_exec.rs` 正被并行窗(T3/lane 窗)编辑,本文行号 = 2026-08-30 晚本窗复核值,**实施时以「替换前文本」字面锚为准**,行号仅助定位。实施窗禁改:`render_exec*.rs`(rt 面已足,零 rt 改动即可接线)。

---

## 0. 目标形态总览

| 臂 | 现状(inflight) | 目标 | 批次建议 |
|---|---|---|---|
| `--bench tsr_device` 静态 | 1\|2\|3(A2 已接,FIF 真流水) | 0-byte 不动 | — |
| `--dyn-demo refit\|rebuild` | 恒 1(CLI fail-closed) | 1 = 顺序入口 0-byte;**2\|3 = slot_as FIF**(AS 表 ×S 副本组 + 平行提交方法) | 批次 A(先) |
| `--skin-demo` | 恒 1(CLI fail-closed) | 同上(`blas_refit` 目标随槽轮换;rt 通路已支持,device 判档随本臂首兑) | 批次 B(后,依赖 A 的循环骨架 + 与 T3 `bridge_ext` 加性面协调) |
| `g31_window_present`(HZB/B1 车道) | 顺序入口 | **不接线(设计结论,见 §4)** | — |

核心机制(与 probe 判档形一致):session AS 表建 S = inflight 份同构副本(组 `[0,S)`);逐帧 `slot = session.next_frame_slot()`(= k % S),`tlas_update`/`blas_refit` 目标与 scene pass 的 AS 绑定全部落 `base + slot`;host 实例写序钉在本槽 fence 之后(rt 入口内建);错槽/组外/跨槽绑定 = 提交前确定性 RED。

---

## 1. edit 计划 A — `src/rurix-render/src/bin/g14_3_lane/g14_3_lane_body.rs`

### A1. `UnifiedTsrLane` 加性字段(快照 L10069 struct,L10087 `pending`,L10090 `inflight`)

在 `pending: VecDeque<PendingTsrFrame>,` 字段块后追加(字段加性;`create` 的 `Ok(Self { … })` 字面初始化两行,行为 0 变):

```rust
    /// G38(RFC-0030 v1.1 §4.3 L2a):每槽 AS 副本组(opt-in;None = 既有面
    /// 0-byte)。经 create_with_slot_as 建,组 [0, inflight)。
    slot_as_group: Option<SlotAsGroup>,
    /// scene pass(0)创建期绑定组克隆(slot_as 逐帧 AS 换槽 override 的
    /// 单一事实源——禁在提交面手写绑定列表,防与 descs 双源漂移)。
    scene_bindings: Option<Bindings>,
    /// slot_as 动态臂在飞票据 FIFO(与静态 `pending` 分列——静态
    /// submit/collect_frame 字面 0-byte)。
    pending_dyn: VecDeque<PendingDynFrame>,
```

`create` 末尾 `Ok(Self { … })`(快照 L10297-10323)追加三行初始化:`slot_as_group: None, scene_bindings: None, pending_dyn: VecDeque::new(),`。新结构体(置于 `PendingTsrFrame`〔快照 L10123〕后):

```rust
/// slot_as 动态臂在飞票据(FIFO 项;verify/readback 意图随票据延迟到 collect)。
struct PendingDynFrame {
    ticket: FrameTicket,
    frame_index: u32,
    readback_out: bool,
    /// 动态核验帧(scene color 回读在子集;collect 侧组装 DynVerifyFrame)。
    readback_scene: bool,
}
```

import 面:lane_body 的 `use rurix_rt::render_exec::{…}` 列表追加 `SlotAsGroup`(如未在)。

### A2. 平行创建入口 `create_with_slot_as`(置于 `create` 后,快照 L10323 后)

```rust
    /// G38(RFC-0030 v1.1 §4.3 L2a)每槽 AS 副本 opt-in 创建面:
    /// `accel_structs` 须为 inflight(≥2)份同构副本(调用方显式构造——
    /// AS 面内存 ×S 显式代价,预算门 g31.fif_dyn.slot_as_group_mem_bytes);
    /// 组 [0, inflight);scene pass(0)绑定组克隆存档供逐帧换槽 override。
    fn create_with_slot_as(
        descs: &'a UnifiedDescs<'a>,
        accel_structs: &[AccelStructDesc<'a>],
        inflight: usize,
    ) -> Result<Self, String> {
        if inflight < 2 || accel_structs.len() != inflight {
            return Err(format!(
                "slot_as 组:inflight ≥2 且 AS 表须 {inflight} 份同构副本(实得 {};L2a opt-in 显式条件)",
                accel_structs.len()
            ));
        }
        let mut lane = Self::create(descs, accel_structs, inflight)?;
        lane.slot_as_group = Some(SlotAsGroup { base: 0, len: inflight as u32 });
        lane.scene_bindings = Some(match descs {
            UnifiedDescs::Mega(d) | /* …七变体同型,复用 create 内 scene_name match 的展开式… */
            UnifiedDescs::MegaDyn(d) => match &d.1[0] {
                Pass::Compute(cp) => cp.bindings.clone(),
                _ => return Err("descs 首 pass 非 compute(scene pass 门面)".into()),
            },
            // …其余变体逐一同型(与 create 的 scene_name match 同构)…
        });
        Ok(lane)
    }
```

(实施注:七个 `UnifiedDescs` 变体的 match 臂与 `create` 内 `scene_name` match〔快照 L10176-10205〕逐臂同构,只是取 `cp.bindings.clone()`;`Bindings` 已 `#[derive(Clone)]`——rt 侧事实,实施时 `cargo check` 即证。)

### A3. `prepare_update_ext` 加性参数 `scene_as_override`(快照:fn 头 ~L10438,`FrameUpdate` 构造 L10541-10551,调用点 `frame_dyn` L10715-10728)

**为什么不新建平行构造器**:prov 由 `next_provenance_with_update(&update)` 从 update 派生(L10552),构造后再改 `binding_overrides` 必致 provenance 校验 RED,override 必须在构造器内;逐字复制构造器 = fif_dyn REPORT §7-3 登记过的双源漂移风险,故取**加性参数**(既有调用点补 `None` 一行,行为 0 变)。

- 签名(替换前,快照 L10452-10453):

```rust
        tlas_update: Option<(u32, Vec<RayQueryTransformedInstanceDesc>, TlasBuildAction)>,
    ) -> Result<(SubmissionProvenance, FrameUpdate), String> {
```

- 替换后:

```rust
        tlas_update: Option<(u32, Vec<RayQueryTransformedInstanceDesc>, TlasBuildAction)>,
        scene_as_override: Option<u32>,
    ) -> Result<(SubmissionProvenance, FrameUpdate), String> {
```

- 构造体(替换前,快照 L10541-10548):

```rust
        let update = FrameUpdate {
            tlas_update,
            buffer_uploads: uploads,
            binding_overrides: vec![
                (idx_resample, bindings_resample),
                (idx_resolve, bindings_resolve),
            ],
```

- 替换后:

```rust
        let mut binding_overrides = vec![
            (idx_resample, bindings_resample),
            (idx_resolve, bindings_resolve),
        ];
        if let Some(as_index) = scene_as_override {
            // L2a 每槽 AS 描述符集:scene pass(0)组内 AS 绑定逐帧轮换到本槽
            // 副本(绑定组 = 创建期克隆,仅 accel_structs 换槽——per-slot
            // override set 既有基建承载,零新描述符面)。
            let mut b = self
                .scene_bindings
                .clone()
                .ok_or("slot_as:scene 绑定组未建(须 create_with_slot_as)")?;
            b.accel_structs = vec![as_index];
            binding_overrides.push((0, b));
        }
        let update = FrameUpdate {
            tlas_update,
            buffer_uploads: uploads,
            binding_overrides,
```

- 既有调用点 `frame_dyn`(替换前,快照 L10727-10728)`Some(tlas_update),\n        )?;` → 替换后 `Some(tlas_update),\n            None,\n        )?;`(顺序臂零 override,行为 0 变)。

### A4. 平行提交/收集方法(置于 `collect_frame`〔快照 L10986-10996〕后;静态三件 `submit_frame`/`pending_len`/`collect_frame` 字面 0-byte)

```rust
    /// G38 L2a:动态臂 slot_as FIF 提交半程——与 frame_dyn 同一构造事实源
    /// (prepare_update_ext + scene_as_override),tlas_update 目标 = 本槽副本
    /// (base + slot;rt 入口三判据 fail-closed 复核),票据入 pending_dyn。
    #[allow(clippy::too_many_arguments)]
    fn submit_frame_dyn_slot_as(
        &mut self,
        iw: u32, ih: u32, ow: u32, oh: u32,
        jitter: [f32; 2],
        vp_j: &Mat4,
        exposure: f32,
        reset: bool,
        scene_params: Vec<f32>,
        insts: Vec<RayQueryTransformedInstanceDesc>,
        action: TlasBuildAction,
        readback_out: bool,
        readback_scene: bool,
        frame_index: u32,
    ) -> Result<(), String> {
        let group = self
            .slot_as_group
            .ok_or("slot_as 组未建(须 create_with_slot_as;L2a opt-in)")?;
        let slot = self.session.next_frame_slot() as u32;
        let target = group.base + slot;
        let (prov, update) = self.prepare_update_ext(
            iw, ih, ow, oh, jitter, vp_j, exposure, reset,
            readback_out, readback_scene, scene_params,
            Some((target, insts, action)),
            Some(target),
        )?;
        let ticket = self
            .session
            .submit_with_frame_update_slot_as(&prov, &update, &group)?;
        self.pending_dyn.push_back(PendingDynFrame {
            ticket, frame_index, readback_out, readback_scene,
        });
        self.advance(vp_j);
        Ok(())
    }

    fn pending_dyn_len(&self) -> usize { self.pending_dyn.len() }

    /// 收集半程:FIFO 出队 → collect → 与 frame_dyn 同一 rec_from_output
    /// 事实源(readback_scene 随票据;帧号回填)。
    fn collect_frame_dyn(
        &mut self, ow: u32, oh: u32, iw: u32, ih: u32,
    ) -> Result<UnifiedFrameRec, String> {
        let p = self.pending_dyn.pop_front().ok_or_else(|| {
            "slot_as collect: 无在飞票据(提交/收集配平破缺,fail-closed)".to_owned()
        })?;
        let out = self.session.collect(p.ticket)?;
        let mut rec = self.rec_from_output(out, p.readback_out, p.readback_scene, ow, oh, iw, ih)?;
        rec.frame_index = p.frame_index;
        Ok(rec)
    }
```

### A5. dyn 臂 bench 循环 FIF 分支(快照:AS 建面 L15966-15974,create 调用 L15975,顺序循环 L15992-16050+,核验组装块在循环体内)

1. **AS 副本组构造**(替换前,快照 L15966-15978):

```rust
        let accel_structs = [AccelStructDesc {
            scene: RayQuerySceneDesc {
                blas_triangles: &blas_refs,
                instances: &assets_dyn.base.instances,
            },
            transforms: None,
            // G31+ 波 B Task B5 字段面:静态/厂商车道无顶点可更新 BLAS(0-byte)。
            updatable_blas: &[],
        }];
        let mut lane = match UnifiedTsrLane::create(&descs, &accel_structs, 1) {
            Ok(l) => l,
            Err(e) => dev_env_or_fail("device_lane", &e),
        };
```

替换后(inflight=1 分支与既有字面逐字同;S 份副本 = probe 同形):

```rust
        // G38 L2a:inflight>1 ⇒ AS 表 = inflight 份同构副本组(每表项独立
        // instance buffer/BLAS/TLAS/scratch——内存 ×S 显式代价,evidence 登记);
        // inflight=1 ⇒ 单表项顺序面 0-byte。
        let slot_as_copies = if inflight > 1 { inflight as usize } else { 1 };
        let accel_structs: Vec<AccelStructDesc<'_>> = (0..slot_as_copies)
            .map(|_| AccelStructDesc {
                scene: RayQuerySceneDesc {
                    blas_triangles: &blas_refs,
                    instances: &assets_dyn.base.instances,
                },
                transforms: None,
                updatable_blas: &[],
            })
            .collect();
        let mut lane = match if inflight > 1 {
            UnifiedTsrLane::create_with_slot_as(&descs, &accel_structs, inflight as usize)
        } else {
            UnifiedTsrLane::create(&descs, &accel_structs, 1)
        } {
            Ok(l) => l,
            Err(e) => dev_env_or_fail("device_lane", &e),
        };
```

2. **核验组装提取**(防 FIF 分支复制既有核验块 ~60 行成双源):把顺序循环内 `DynVerifyFrame` 组装块(以 `verify_recs.push(DynVerifyFrame {` 收尾的整段,输入 = `rec.scene_color`/`rec.mv_out` + 帧号)提取为循环前闭包 `let mut push_verify = |rec: &UnifiedFrameRec, i: u32| -> Result<(), String> { …原块逐字搬移,i 处帧号参数化… }`;顺序循环调用点 1 行替换。**轨迹/相机可由帧号重建**(`dyn_trajectory(i, origin)` 帧号纯函数 + `vp_j = jittered_vp(&vp, halton(jitter_base+i+1,…), …)` 帧号纯函数)——FIF collect 侧凭 `rec.frame_index` 复算,无需随票据携带。

3. **FIF 循环分支**(顺序循环整段外包 `if lane.inflight > 1 { …新 FIF 循环… } else { 既有循环 0-byte }`,骨架逐字镜像静态 FIF 分支〔快照 L16955-17093〕,差异仅四处):
   - `lane.submit_frame(…)` → `lane.submit_frame_dyn_slot_as(…, scene_params, dyn_frame_instances(xf), action, readback_out, verify, i)`(`xf`/`scene_params`/`verify` 计算 = 顺序循环逐字同,均帧号纯函数);
   - `lane.pending_len()` → `lane.pending_dyn_len()`;`lane.collect_frame(out_w, out_h)` → `lane.collect_frame_dyn(out_w, out_h, in_w, in_h)`;
   - collect 到 rec 后:`if rec.scene_color.is_some() { push_verify(&rec, rec.frame_index)?; }`(核验帧判定随票据,组装凭帧号复算);
   - 排空段同静态(drain 墙钟并入末样本,`prod=frame−tail` 不变式维持;receipt 口径与 A2 静态 FIF 登记一致)。

### A6. skin 臂(批次 B;形与 A4/A5 同构,差异清单)

- `frame_skin`(快照 L10746-10939)是**内联构造 + 内联 rec 组装**(不走 prepare_update_ext)。拆分:
  1. rec 组装段(快照 L10864-10937,自 `let gpu = |name…` 至 `SkinFrameRec { … }` 构造)提取为 `fn skin_rec_from_output(&self, out: DeviceFrameOutput, readback_out: bool, verify: bool, debug_tris: bool, ow,oh,iw,ih) -> Result<SkinFrameRec,String>`(原地逐字搬移,`frame_skin` 调用点 1 行替换——行为 0 变,禁复制成第二源);
  2. update 构造段加 `scene_as_override: Option<u32>` 分叉(skin 的 scene pass = **pass 1** `g31_skin_scene`〔创建期 `accel_structs: vec![0]`,快照 L14051-14057〕——override 推 `(1, b)` 而非 `(0, b)`;binding_overrides 既有 (3,resample)/(4,resolve) 不动);
  3. `submit_frame_skin_slot_as`:`blas_refit: Some(BlasRefitUpdate { as_index: target, … })`——**`as_index` 逐帧 = base+slot**(render_exec.rs L430-431 字段文档「与 tlas_update 同域」;skin 臂无 tlas_update,仅 blas_refit 换槽);副本组创建期每表项 `updatable_blas: &[1]`(角色 BLAS 打标 ×S 份);palette/params 双表 uploads 走既有 per-slot staging(FIF 兼容面,零改动);
  4. `PendingSkinFrame { ticket, frame_index, readback_out, verify, debug_tris }` + `collect_frame_skin`(调 1 的 helper);核验(`SkinVerifyFrame` 组装)同 A5-2 提取闭包、collect 侧凭帧号复算 palette(骨骼动画 = 帧号纯函数)。
- **与 T3 协调**:T3 正在 render_exec 落 `BlasRefitBridgeExt`(多 region 脏拷贝 + 桥接计时,加性;`execute_with_frame_update_bridge_ext`)。skin slot_as 首兑走**普通 `blas_refit` 路**(slot_as 入口既有形);若 T3 的 bridge_ext 也要 × FIF,须 rt 侧另开 `submit_with_frame_update_slot_as_bridge_ext`(留 T3/后续窗,本计划不占)。
- **Refit 语义登记**:skin BLAS refit 槽副本历史 = k−S(probe refit 对照臂已双 PASS 逐字节;若 g14_3 实测非纯,按 L2a 字面降档「按槽稳定」显式登记,不充逐字节绿)。

---

## 2. edit 计划 B — `src/rurix-render/src/bin/g14_3_pipeline_perf.rs`(CLI fail-closed 措辞随 L2a)

### B1. dyn 臂解除(替换前,本窗复核 L618-620):

```rust
                if inflight != 1 {
                    fail("--dyn-demo 要求 --inflight 1（A2 约束：FIF 流水入口拒 tlas_update——共享 instance buffer host 写面在飞帧不可改写；动态场景走顺序入口，per-slot 实例缓冲归后续波）");
                }
```

替换后(**fail-closed 语义保持**:非法组合仍拒;合法组合的语义门 = lane 侧 create_with_slot_as/rt 三判据):

```rust
                // G38(RFC-0030 v1.1 §4.3 L2a):--inflight 2|3 = 每槽 AS 副本
                // opt-in 路径(AS 表 ×inflight 同构副本组 + 平行入口
                // submit_with_frame_update_slot_as;每表项独立 instance buffer/
                // BLAS/TLAS/scratch——内存 ×S 显式代价,预算门条目
                // g31.fif_dyn.slot_as_group_mem_bytes)。inflight=1 顺序入口
                // 字面 0-byte;--warmup ≥ inflight−1 通则(上方)已覆盖填充段。
```

(即删除该 fail 块换为登记注释——`--inflight` 的 bench/tsr_device/warmup 三通则〔本窗复核 L578-592〕对 dyn 臂自然生效,无需新拒绝行。)

### B2. skin 臂解除(批次 B;替换前,本窗复核 L639-641):

```rust
                if inflight != 1 {
                    fail("--skin-demo 要求 --inflight 1（A2 同律约束：FIF 流水入口拒 blas_refit——BLAS 顶点缓冲为共享写面,在飞帧 ray query 读取中不可改写；蒙皮车道走顺序入口）");
                }
```

替换后:同 B1 体例注释(措辞把 `tlas_update` 换 `blas_refit`,并注「角色 BLAS 顶点副本 ×S(updatable 打标逐表项)」)。**批次 A 落地时本块字面不动**(skin 未接线前维持拒绝)。

### B3. 帮助/描述行(可选,同批):`--inflight` 帮助字面(L578-580「1 = 顺序全同步既有面 0-byte;2/3 = FIF 真流水深度」)追加「(动态/蒙皮臂 = L2a 每槽 AS 副本 opt-in)」。

---

## 3. 数字步骤 / 验收环怎么跑

**不领新数字步骤号**(registry CI_step next_free=525 不消费):

1. **等价环(digest ≡ 顺序基线;逐格三跑)**——同参数三跑对照,receipt `last_frame_digest` 逐字节等 + `RURIX_G14_FLIP_TRACE` 逐帧 digest 序列逐字节等(A2 既有逐帧轨迹基建,快照 L17020 flip_trace 写行;**逐帧序列门强于末帧门**):

```powershell
$env:CARGO_TARGET_DIR = "H:\rurix\target-night"; $env:RURIX_VK_VALIDATION = "1"
# 臂 x ∈ {1,2,3};rebuild 硬门 + refit 对照臂;--warmup ≥ inflight−1
cargo run -p rurix-render --features vulkan --bin g14_3_pipeline_perf -- --bench --backend tsr_device `
  --scene bistro-interior --tier 100 --frames 120 --warmup 10 --dyn-demo rebuild --inflight <x> `
  # + RURIX_G14_FLIP_TRACE=<dir_x> 环境变量(逐帧 digest 轨迹)
```

   判据:①`fc.exe /b`(或逐行 diff)`<dir_1>/frame_digests_*.jsonl` ≡ `<dir_2>` ≡ `<dir_3>`(逐帧逐字节);②各臂双跑位级(同臂重放 jsonl 逐字节);③validation ERROR=0(bench 内建 fail-closed);④dyn 位置核验 all_pass(verify 帧机制 FIF 下维持);⑤`--dyn-demo refit` 同环,非纯即按 L2a 降档登记。skin 批次同构(判据 + skin_verify all_pass + 窗级真动门)。
2. **内存预算门收割**(同批,GPU 真跑归主 agent):`g31_fif_dyn_probe --frames 48 --rays 96x72 --out evidence/g31_fif_dyn_probe_rebuild_<ts>.json`(+refit 件)→ `py -3 ci/calibrate_fif_budget.py` 回填 → `py -3 ci/budget_eval.py` 绿(通用路直读)。生产 bistro 规模的 AS 副本内存(×S 数百 MB 级)如实入 receipt/evidence notes——预算条目锚 probe 场景,生产规模登记面另行 evidence(不混口径)。
3. **门/schema 面建议**(实施批裁决,不在本窗):`ci/g31_dynamic_scene_smoke.py` 加性 `--inflight` 对照腿(evidence schema 版本化经 `_patch` 纯追加,先例 = 本窗 `_patch_g31_fif_dyn_schemas.py`);或先 evidence-only 登记 + TODO #90 行补注,门化留稳定后窗。

---

## 4. 窗口 bin(`g31_window_present.rs`)——不接线(设计结论,非缺口)

既有登记字面(本窗复核 L5725-5727):「**B1 车道状态机(顺序入口——逐帧 host 决策在环,FIF 流水面天然不适用,A2 约束〔FIF 拒 tlas_update〕同律登记;两阶段调度 + 闭环重渲全记录)**」。

机制根据:HZB 车道的逐帧 TLAS 掩码更新(`masks`/`uploaded_masks`,L5735-5738)由**上帧回读的可见性判定**驱动——host 在环反馈闭环。FIF 化会把决策延迟 S−1 帧(读到的是 k−S 帧判定),改变剔除语义本体,不是「每槽副本」能消解的写面竞争问题。故:窗口 bin **不接 slot_as**,维持顺序入口与既有登记字面 0-byte。本结论随 L2a 落 RFC 不改(L2a 是 opt-in 臂,不承诺全消费面)。

---

## 5. 风险与留窗登记

| # | 项 | 处置 |
|---|---|---|
| 1 | `g37_submit_pipelined_frame_slot_as` 为复制适配体(fif_dyn REPORT §7-3) | 单源折叠(既有 `submit_pipelined_frame` 加 `Option<&SlotAsGroup>` 参数)留 render_exec 空窗期;本计划零 rt 改动,不阻塞 |
| 2 | lane_body/render_exec 并行窗行号漂移 | 全部 edit 以「替换前文本」字面锚;实施前逐锚 grep 复核 |
| 3 | dyn FIF 下 verify 帧 scene 回读税进流水 | verify 帧稀疏(`DYN_VERIFY_EVERY`),回读经 per-slot staging(既有面);tail 计量口径与静态 FIF drain 登记同律 |
| 4 | skin×`bridge_ext`(T3 在途) | skin slot_as 首兑走普通 blas_refit 路;bridge_ext×FIF 须 rt 平行入口,留 T3/后续窗 |
| 5 | Refit 非纯风险(驱动面) | probe refit 臂已双 PASS(逐字节);g14_3 复证非纯时按 L2a「按槽稳定」降档显式登记 |
| 6 | AS 副本组内存 ×S(bistro 级数百 MB) | opt-in 显式代价;预算门条目已占位,probe 收割标定;生产规模登记面随批次 evidence |
