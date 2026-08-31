# REPORT — G38 T2:#90 FIF×动态正式化(段 1 实施 + 段 2 设计)

> 日期:2026-08-30。承接:G37 W3 fif_dyn 判档窗(artifacts/day_0830_delivery/w3_deep/fif_dyn/)+ GPU 判档双 PASS(rebuild/refit,gates 六项全 true)。本窗纪律:零 GPU 真跑(归主 agent 批次 1)、零 git commit、禁编辑 render_exec*.rs / lane_body / 窗口 bin / perf.rs、既有 schema/evidence 文件零字节改动(版本化新文件)。

---

## 1. 段 1 改动清单

### 1a. RFC-0030 v1.1 正式登记(体例 = RFC-0019 v1.1 加性修订先例)

| 文件 | 改动 |
|---|---|
| `rfcs/0030-g14plus-pipeline-structural-optimization.md` | ① §4.3 L2 行(L97)后插入 **L2a 条款行**——草案 §3 底稿**逐字**(半角标点原样,不随 RFC 本体全角转换),尾缀「(加性修订行;2026-08-30)」体例标注;② §9.2 修订表追加 **v1.1 行**(三列;变更列「**§4.3 加性**:…零既有语义改动。」)。头部状态字段/其余全文 0-byte |
| `artifacts/day_0830_delivery/w3_deep/fif_dyn/RFC_DRAFT_RFC0030_amendment.md` | 头部(标题后)追加状态更新行「已正式登记 → RFC-0030 v1.1(2026-08-30,G38 窗)…档案面 0-byte 保留」;草案正文 0-byte |
| `G31_PLUS_COMMERCIAL_RENDERER_TODO.md` | #90 行(L232)现状格追加登记注(原义字面保留):「**G38 登记**:判档 GPU 双 PASS(…)→ **L2a 已登记 RFC-0030 v1.1(G38)**;加性入口在树,预算门条目占位,g14_3 接线计划 = 本目录 WIRING_PLAN.md」(表格式所迫行内改,原文全角标点段未动) |
| `milestones/g14/G14PLUS_RECORD.md` | 新增 **§7 收口后 RFC-0030 修订事件登记(只追加)** 一条(叙事指针,判据事实源 = RFC v1.1 + evidence 件)+ 修订记录 v1.2 行 |

### 1b. 每槽 AS 内存预算门(零新数字步骤号——CI_step next_free=525 不消费,budget_eval 通用路自动消费)

| 文件 | 性质 | 内容 |
|---|---|---|
| `src/rurix-render/src/bin/g31_fif_dyn_probe.rs` | 本窗独占,改 | evidence **v2**(`rurix.g31.fif_dyn_probe.v2`):① `slot_as_mem` 区段——逐臂从 session telemetry 全量 allocation ledger 按 AS 表项 resource_id(= resources.len()+ai+1 登记式,本 probe 基=3)过滤求和,A×1/B×2/C×3,含 `per_slot_bytes`+`group_total_bytes`(纯函数 `slot_as_mem_from_ledger`,首帧采集);② `results.trimmed_mean` 镜像槽 = 最大组 group_total(bytes,budget_eval 通用路直读);③ gates 第 7 门 `slot_as_mem_registered`(逐臂表项数=副本数 ∧ 全>0〔0=ledger 映射漂移假账,fail-closed〕∧ 双跑账相等);④ selftest 第 5 项 `slot_as_mem`(合成 ledger 红绿)+ `#[cfg(test)]` 第 5 测;⑤ 文档注释 v2/收割前缀指引。既有六门/三臂/RED 双臂判据字面不动 |
| `milestones/g31/g31_fif_dyn_probe_v2_evidence_schema.json` | 新文件 | v2 全形状(七门闭集/slot_as_mem 逐臂 minItems=maxItems/results/6 arm 对象);**如实登记:v1 从未注册 schema 文件/路由**(artifacts sidecar 自 declare 形态),v2 为首个注册件——交接单「旧 v1 schema 0-byte / v2 先于 v1 路由序」两条空适用 |
| `ci/_patch_g31_fif_dyn_schemas.py` | 新文件 | check_schemas 三处纯追加(load/validator/route,锚 = g31_dynamic_scene 三块后;前缀 `g31_fif_dyn_probe_` 与既有 g31 全族互不包含);幂等 ×2 实跑 PASS(第二跑仅核验);io.open newline="" 字节面 + py_compile,照 sdk_dist_v2/texture_heap 先例 |
| `milestones/g31/g31_budget.json` | 纯追加条目 | `g31.fif_dyn.slot_as_group_mem_bytes`:direction=max,unit=bytes,**evidence=estimated + skip_reason**「待 G38 批次 1 GPU 收割回填(threshold = measured × 1.5 程序产;…)」,threshold/evidence_file/measured_value=null(estimated 自动 skip 不红;既有六条目字面 0-byte) |
| `ci/calibrate_fif_budget.py` | 新文件 | 标定回填:v2 evidence 资格核验(schema/verdict PASS/七门/镜像槽**复算互核**〔trimmed_mean == max 组 == Σper_slot,不信任单值〕)→ **外科手术式字节级回填**(仅条目内五字段,条目外前后缀逐字节核验 0 改动)→ 回填后自核;幂等(值一致零写盘);`--check` 只读互核(measured_value 位级等 + threshold==×1.5 程序产);`--selftest` 伪 evidence 双向(临时目录零真实文件) |

### 1c. 本窗未触面(纪律确认)

`render_exec.rs` / `render_exec_g37_fif_dyn.rs` / `g14_3_lane_body.rs` / `g14_3_pipeline_perf.rs` / `g31_window_present.rs` / spec/ / registry/number_ledger.json / 既有 evidence 与 schema 文件:**零字节改动**。旧 v1 判档 evidence 双件(artifacts/…/evidence_fif_dyn_{rebuild,refit}.json)0-byte 档案面。

## 2. 验证结果

| 项 | 结果 |
|---|---|
| `py -3 ci/_patch_g31_fif_dyn_schemas.py` ×2 | **PASS ×2**(首跑三处插入,次跑幂等仅核验;py_compile 绿) |
| `py -3 ci/check_schemas.py` | **PASS**(v2 schema/路由注册后全量绿) |
| `py -3 ci/budget_eval.py` | **PASS(329 pass, 1 skip)**——唯一 skip = 本条目 estimated 占位,skip_reason 如实输出,零红 |
| `py -3 ci/calibrate_fif_budget.py --selftest` | **PASS 4/4**(绿臂回填+幂等+check / RED verdict 拒 / 镜像互核拒 / 篡改 threshold check 红) |
| `cargo check -p rurix-render --features vulkan --bin g31_fif_dyn_probe`(CARGO_TARGET_DIR=target-night) | 见 §2a |
| `g31_fif_dyn_probe --selftest`(纯 host) | 见 §2a |

### 2a. cargo 面并行窗阻断登记(诚实分界)

首跑(~19:2x)`rurix-rt` lib **5 个编译错误全在 render_exec.rs**(collect/submit_persistent_frame 签名迁移半程 + DeviceFrameTelemetry 新字段 blas_bridge_* 缺初始化 + query_slots 未定义)——`git diff` 实证 render_exec.rs +144/−4 未提交 = **T3 agent 在途改动的基线过渡态,非本窗引入**(本窗对该文件零字节)。复跑(~19:4x)降至 1 错(E0425 query_slots)——T3 正在愈合。probe 自身面(本窗改动)在两轮报错中**零条目**。终态见 §2b(本报告随末次复跑更新);若交付时仍红,复核命令与判定基准已列,基线愈合后 `cargo check` + `--selftest`(应 5/5)即证——probe 单测 `slot_as_mem_ledger_sum` 为纯函数测,不依赖 GPU。

### 2b. cargo 终态(末次复跑)

(见本文件尾部「验证终态补记」——写 REPORT 时 T3 在途,尾部补记为末次实测。)

## 3. 段 2 产物指针

`artifacts/day_0830_g38/t2_fifdyn/WIRING_PLAN.md`——g14_3 生产接线精确 edit 计划(文件+行号快照+替换前后文本),要点:

- **lane_body**:`UnifiedTsrLane` 加性字段(slot_as_group/scene_bindings/pending_dyn)+ 平行创建 `create_with_slot_as`(调用方显式 ×S 副本组,opt-in)+ `prepare_update_ext` 加性参数 `scene_as_override`(prov 派生约束所迫,拒复制构造器防双源)+ 平行方法 `submit_frame_dyn_slot_as`/`collect_frame_dyn`(静态 submit/collect_frame 字面 0-byte,独立 FIFO)+ dyn 臂 FIF 循环分支(镜像静态 A2 分支,verify 凭帧号纯函数在 collect 侧复算);skin 批次 B(scene pass=1 override / `BlasRefitUpdate.as_index` 逐帧 base+slot / rec 组装 helper 提取防双源 / 与 T3 bridge_ext 协调登记)。
- **pipeline_perf**:dyn/skin 两处 `--inflight 1` 强制 fail 块 → L2a 登记注释(替换前后文本逐字在案;通则三门〔bench/tsr_device/warmup≥N−1〕已覆盖,fail-closed 语义不失)。
- **窗口 bin**:**不接线 = 设计结论**——HZB 车道逐帧 host 决策在环(上帧可见性判定驱动下帧 TLAS 掩码),FIF 引入 S−1 帧决策延迟改变剔除语义本体,非副本可消解;既有 L5725-27 登记字面维持。
- **验收环**:三跑 digest 等价(inflight 1/2/3,`RURIX_G14_FLIP_TRACE` 逐帧序列逐字节 + receipt 末帧)+ 双跑位级 + validation=0 + dyn/skin 位置核验 all_pass;refit 非纯按 L2a 降档登记;零新数字步骤号,门化建议 = g31_dynamic_scene_smoke 加性对照腿(留实施批)。

## 4. 主 agent 批次 1 收割清单(GPU,本窗零跑;**终版,含段 2 dyn 生产验收环**)

### 4a. probe 收割 + 预算门回填(段 1 面)

```powershell
$env:CARGO_TARGET_DIR = "H:\rurix\target-night"
# ① host selftest(应 5/5)
cargo run -p rurix-render --features vulkan --bin g31_fif_dyn_probe -- --selftest
# ② v2 收割(文件名前缀 g31_fif_dyn_probe_ = check_schemas 路由;ts 自取)
cargo run -p rurix-render --features vulkan --bin g31_fif_dyn_probe -- --frames 48 --rays 96x72 --out evidence/g31_fif_dyn_probe_rebuild_<ts>.json
cargo run -p rurix-render --features vulkan --bin g31_fif_dyn_probe -- --frames 48 --rays 96x72 --action refit --out evidence/g31_fif_dyn_probe_refit_<ts>.json
# ③ 标定回填 + 门验
py -3 ci/calibrate_fif_budget.py          # 缺省取 evidence/ 最新件;或 --evidence <path>
py -3 ci/calibrate_fif_budget.py --check
py -3 ci/check_schemas.py; py -3 ci/budget_eval.py
```

### 4b. g14_3 dyn 臂 slot_as 生产验收环(段 2 面;逐帧 digest 序列 ≡ 顺序基线)

```powershell
$env:CARGO_TARGET_DIR = "H:\rurix\target-night"; $env:RURIX_VK_VALIDATION = "1"
# 三臂 × 双跑(x ∈ 1|2|3,r ∈ a|b;rebuild 硬门):
#   逐帧 digest 轨迹经 RURIX_G14_FLIP_TRACE 落 jsonl(A2 既有基建,FIF 收集侧按
#   票据帧号写行,FIFO 保序);--warmup 10 ≥ inflight−1 通则;120 帧含核验帧
#   (DYN_VERIFY_EVERY 采样,dyn_verify.json all_pass 内建 fail-closed)。
foreach ($x in 1,2,3) { foreach ($r in "a","b") {
  $env:RURIX_G14_FLIP_TRACE = ".tmp/g38_dyn_fif/rebuild_x${x}_${r}"
  cargo run -p rurix-render --features vendor-upscale --bin g14_3_pipeline_perf -- `
    --bench --backend tsr_device --scene bistro-interior --tier 100 `
    --frames 120 --warmup 10 --dyn-demo rebuild --inflight $x
} }
Remove-Item Env:RURIX_G14_FLIP_TRACE
# 判据(全部满足才判 PASS,任一破缺如实登记 RED):
#  ① 等价门:x1_a ≡ x2_a ≡ x3_a 逐帧 digest 逐字节(jsonl 全文 fc /b);
#  ② 双跑位级:x?_a ≡ x?_b 逐臂;
#  ③ validation=0 + dyn_verify all_pass(bench 内建 fail-closed,RC=0 即含);
#  ④ receipt last_frame_digest 三臂同(①的末帧投影,附核)。
$base = Get-ChildItem .tmp/g38_dyn_fif/rebuild_x1_a/*.jsonl | Select-Object -First 1
foreach ($d in "x1_b","x2_a","x2_b","x3_a","x3_b") {
  $f = Get-ChildItem ".tmp/g38_dyn_fif/rebuild_$d/*.jsonl" | Select-Object -First 1
  fc.exe /b $base.FullName $f.FullName | Select-Object -First 3
}
# refit 对照臂(同环换 --dyn-demo refit;非纯实测时按 L2a「按槽稳定」降档登记,
# 不充逐字节绿——Rebuild 硬门不受影响):
#   同上 foreach,rebuild → refit,输出目录 refit_x${x}_${r}。
```

- **口径注**:①的逐帧序列门**强于**末帧 receipt 门(中途漂移末帧可能回合);flip-trace 开启使每帧 `readback_out=true`,三臂回读税同形同价,digest 语义不变。inflight>1 时 receipt `render_lane` 描述串为 slot_as 形态如实登记(inflight=1 描述字面 0-byte)。
- **skin 臂本批不跑**:蒙皮×slot_as = 批次 B 留窗(WIRING_PLAN §1-A6;CLI 拒绝面维持,拒因指向 L2a + 计划文档)。

## 5. 风险与留窗

| # | 项 | 状态 |
|---|---|---|
| 1 | rt 复制适配体单源折叠(fif_dyn REPORT §7-3) | 留窗登记维持(render_exec 空窗期做;本窗零 rt 改动) |
| 2 | render_exec.rs 基线过渡态(T3 在途) | §2a 登记;probe 验证随基线愈合即证 |
| 3 | v1 schema「0-byte 维持」交接单条款 | 空适用如实登记(v1 从未有 schema 文件;v2 为首注册,schema description 内亦登记) |
| 4 | 预算条目锚 probe 微场景,生产 bistro 规模 ×S 数百 MB | 预算门语义 = probe 判档场景账;生产规模随接线批 evidence 另登记,不混口径(WIRING_PLAN §3-2) |
| 5 | trimmed_mean 镜像槽为整数 bytes | budget_eval 通用路 float() 兼容;schema type=integer 钉死;标定复算互核防单值伪造 |

---

## 验证终态补记(末次实测,2026-08-30 晚)

T3 基线愈合后(第三轮复跑)cargo 面全绿:

| 项 | 结果 |
|---|---|
| `cargo check -p rurix-render --features vulkan --bin g31_fif_dyn_probe`(CARGO_TARGET_DIR=H:\rurix\target-night) | **绿(exit 0)**;12 条 warning 全在存量/并行窗文件(vk_m50_rt_body ×4 / vk_g31_ser_body ×1 / vk.rs ×7),probe 本文件**零告警** |
| `cargo check -p rurix-render`(default) | **绿(exit 0)**——bin 经 required-features 门控不入 default,零回归旁证 |
| `cargo test -p rurix-render --features vulkan --bin g31_fif_dyn_probe` | **5/5 过**(既有 4 测 + 新 `slot_as_mem_ledger_sum`) |
| `g31_fif_dyn_probe --selftest`(dev 构建,纯 host,零 GPU) | **PASS 5/5,exit 0**(第 5 项 = slot_as_mem AS 账过滤求和红绿) |
| RFC-0030 落笔复核 | L2a 条款行在 L2/L3 之间(现 L99),底稿逐字 + 「(加性修订行;2026-08-30)」;§9.2 v1.1 行在表尾 |

§2a 登记的过渡态阻断已消除;本报告全部验证项终态绿。

---

## 6. 段 2 实施(2026-08-30 晚,第二批——lane_body/pipeline_perf 编辑权移交后按 WIRING_PLAN 落地)

### 6a. 改动清单

| 文件 | 改动(全部按 WIRING_PLAN 内容锚落点——T4 改动后行号已漂,未触 T4 新面〔RXCS/RXCP/RXHL v2/gather_tri_uv_attrs/LAMP_GRID_M〕) |
|---|---|
| `g14_3_lane_body.rs` | **A1** `use` 列表加 `SlotAsGroup`;`UnifiedTsrLane` 尾部加性三字段(`slot_as_group`/`scene_bindings`/`pending_dyn`,create 初始化 None/None/空——既有全车道行为逐位同);新结构 `PendingDynFrame`(独立 FIFO 项,`#[allow(dead_code)]` 诚实标注惯例)。**A2** 平行创建 `create_with_slot_as`(断言 inflight≥2 ∧ AS 表 = inflight 份副本;scene pass 绑定组自 descs 克隆存档——七变体 match)。**A3** `prepare_update_ext` 加性末参 `scene_as_override: Option<u32>`(Some 时构造器内追加 `(0, 绑定组换槽)` override——prov 由 update 派生所迫;None = 既有产物逐字段同),既有两调用点(`prepare_update` 委托/`frame_dyn`)补 `None`。**A4** 平行方法 `submit_frame_dyn_slot_as`(slot = next_frame_slot,tlas_update 目标与 scene override 同落 base+slot → `submit_with_frame_update_slot_as` → pending_dyn 入队 → advance)/`pending_dyn_len`/`collect_frame_dyn`(readback_scene 随票据,帧号回填)——静态 `submit_frame`/`pending_len`/`collect_frame` 三件字面 0-byte。**A5** dyn 臂:AS 表副本组构造(inflight>1 ⇒ ×inflight 同构,`create_with_slot_as` 分叉;=1 分支与既有逐字同)+ 核验帧组装提取为 `push_verify` 闭包(原内联块逐字迁入,轨迹/相机凭帧号纯函数复算——顺序/FIF 同一事实源)+ FIF 循环分支(骨架逐字镜像静态 A2 分支:FIFO submit/collect + 排空段墙钟并入末样本;核验帧在 collect 侧按 `rec.scene_color.is_some()` 组装)+ receipt `render_lane` 描述串 inflight=1 字面 0-byte / >1 slot_as 形态如实登记 |
| `g14_3_pipeline_perf.rs` | **B1** dyn 闭集注释③改写(inflight 1\|2\|3 = L2a 语义)+ `--dyn-demo 要求 --inflight 1` fail 块**解除**(换 L2a 登记注释;bench/tsr_device/warmup≥N−1 三通则维持 fail-closed);`--inflight` 接线面拒因追加「+ G38 L2a 动态臂」。**B2** skin:拒绝面**维持**,闭集注释②与拒因改写为批次 B 留窗指向(L2a + WIRING_PLAN §1-A6) |
| `g35_particle_lane.rs` | 机械适配 1 处:include 共享体的 `prepare_update_ext` 直调补末参 `None`(产物逐字段同,0-byte 语义;附注释)——WIRING_PLAN 未列的第三消费方,编译面发现如实登记 |
| `render_exec.rs` / `render_exec_g37_fif_dyn.rs` | **零编辑**(rt 面段 1 已足;T3 bridge_ext 面未触) |
| `g31_window_present.rs` | **零编辑**(不接线 = 设计结论维持,WIRING_PLAN §4) |

**skin 批次 B 如实登记不做**(本窗主体 = dyn 臂;rt 通路已支持,g14_3 侧计划在案 WIRING_PLAN §1-A6,CLI 拒绝面维持)。lane 侧无纯 host 可测口(新方法全依赖 DeviceFrameSession),不强求补单测——槽纪律纯 host 面已由 rt 3 单测 + probe selftest 5/5 承载,如实登记。

### 6b. 验证结果(全部 CARGO_TARGET_DIR=H:\rurix\target-night,dev,零 GPU)

| 项 | 结果 |
|---|---|
| `cargo check -p rurix-render --features vendor-upscale --bins` | **RC=0**;rurix-render 侧告警全落未触文件(g9_m99 ×1 / g12_pt_production ×1 / g31_window_present ×4〔L9441-9462 自有面〕/ g34_full_lane ×8〔全在 g34_skin_section.rs〕),**本窗三文件零告警、输出全文零本窗标识符**——零新增 |
| `cargo check -p rurix-rt` | **RC=0**(rt 未触;lib 15 warning = T3/存量面) |
| `cargo check -p rurix-asset -p rurix-geom-build` | **RC=0**(asset 2 warning = 存量 g10_5_scene_render) |
| `cargo check -p rurix-render --features vulkan --bin g31_fif_dyn_probe` | **RC=0**(段 1 面零回归) |
| dead_code 纪律 | 新方法/结构按共享体惯例 `#[allow(dead_code)] // …独消费面(诚实标注)`(7 个 include 方中仅 g14_3_pipeline_perf 消费);g34_full_lane 复编 fresh 指纹实证告警集不含本窗项 |

### 6c. 验收环

见 §4b 终版(inflight 1/2/3 × 双跑 × rebuild/refit,`RURIX_G14_FLIP_TRACE` 逐帧 digest 序列逐字节 + validation=0 + dyn_verify all_pass 内建;GPU 归主 agent 批次 1)。
