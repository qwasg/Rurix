# G37 W3 — FIF×动态共存判档(G36 五留窗之一,TODO #90)侦察·RFC 草案·最小判档件·验证

> 日期:2026-08-30。模式对齐异步三件套判档(同目录 ../async/:RFC 草案 +
> 独立判档 probe + measured 两态,no-go 留档合法)。GPU 真跑归主 agent(§6)。
> 纪律遵守:零 GPU、零 `cargo build --release`、零 target-night;生产车道文件
> (g31_window_present / g14_3_pipeline_perf / g14_3_lane_body)、render_exec
> 既有行为(一处例外见 §7-1,机械愈合非语义改)、rfcs//spec/ 本体、milestones/、
> registry/、ci/ 全部 0-byte。
>
> 交付件:`src/rurix-rt/src/render_exec_g37_fif_dyn.rs`(加性 body-include,
> 752 行)+ `render_exec.rs` 尾部 4 行 include 块 + `src/rurix-render/src/bin/
> g31_fif_dyn_probe.rs`(独立 harness,1230 行)+ `Cargo.toml` 加性 `[[bin]]`
> 一段 + 同目录 [RFC_DRAFT_RFC0030_amendment.md](RFC_DRAFT_RFC0030_amendment.md)。
> cargo check(dev)全绿、`--selftest` 4/4 PASS 退 0、rt/bin 单测 3+4 全过(§5)。

---

## 1. 侦察记录(任务①–⑤;render_exec.rs 行号在本会话内因并行窗漂移,以引用字面为准,括注 = 末次复核值)

### ① FIF(frames-in-flight)实现在哪

| 面 | 位置 | 语义 |
|---|---|---|
| 公共提交半程 | render_exec.rs `DeviceFrameSession::submit_with_frame_update`(L1687)→ `FrameTicket` | 校验序与顺序入口同源(`frame_update_state` → expected provenance → `validate_submission_provenance`),submit 后**不等完成 fence** |
| 收集半程 | `DeviceFrameSession::collect`(FrameTicket → DeviceFrameOutput) | 等票据帧 fence(有界)→ slot 区间 timestamp → per-slot staging 回读 → telemetry,释放 slot |
| 内部提交体 | `submit_pipelined_frame`(头注「G14plus RFC-0030 §4.3 L2」) | slot 占用检查 → slot fence 复用等待 + reset → per-slot 面懒建 → 上传写本 slot staging → per-slot cmd 全量重录(**首条全局守卫 barrier** → staged copies → 冲刷 → `record_frame_body` 同一录制事实源 → 帧尾 staged buffer readback)→ submit |
| 确定性论证 | 同函数头注 | 「GPU 帧间全序:cmd 首条全局守卫 barrier(ALL_COMMANDS/MEMORY_WRITE → ALL_COMMANDS/MEMORY_READ\|WRITE)使帧 N+1 全部 GPU 访问序于帧 N 之后……**流水收益 = CPU 侧 submit 与 fence 等待解耦,非 GPU 帧重叠**」 |
| CLI 消费面 | g14_3_pipeline_perf.rs L351(解析)/ L562(`--inflight 只接受 1\|2\|3`)/ L567(仅 `--bench --backend tsr_device` 已接线)/ L572(`--warmup ≥ N−1`) | lane_body `submit_frame`(L10714,票据入 FIFO)/ `collect_frame`(L10754,FIFO 出队)/ `pending_len`;循环 = submit k → 满深度 collect 最早 |

### ② FIF 拒动态的 fail-closed 位置与理由

- **render_exec 入口拒**(`submit_with_frame_update` 体内,消息字面):
  - L1709:「FIF 流水不支持 tlas_update(**TLAS instance buffer 为共享 host 写面,在飞帧读取中不可改写**;需 TLAS 更新请走顺序 execute_with_frame_update)」;
  - L1718:「FIF 流水不支持 blas_refit(**BLAS 顶点缓冲为共享写面,在飞帧 ray query 读取中不可改写**;需 BLAS 更新请走顺序 execute_with_frame_update)」;
  - `submit_pipelined_frame` 内另有防御性复核(L10016)。
- **CLI 联动拒**(g14_3_pipeline_perf.rs):L601 「--dyn-demo 要求 --inflight 1(A2 约束……per-slot 实例缓冲归后续波)」;L622 「--skin-demo 要求 --inflight 1(A2 同律……蒙皮车道走顺序入口)」。
- **机制理由**(代码序核实):顺序路的 TLAS 更新 = submit 期 host `write_transforms` memcpy 进 instance buffer(vk.rs L13242 面,逐 64B 槽与 host 影子 diff)→ 录 BUILD/UPDATE。FIF 下 submit 帧 N+1 时帧 N 的 GPU 仍可能在执行「读 instance buffer 的 TLAS build」与「读 AS 的 ray query」——守卫 barrier 只全序化 **GPU↔GPU**,管不了 **host 写 ↔ 在飞 GPU 读**;单份 AS 对象上 TLAS/BLAS 原地 build 也使「本帧写 AS」与「上帧读 AS」落同一对象。故 fail-closed。

### ③ RFC-0030 §4.3 L2 条款原文

`rfcs/0030-g14plus-pipeline-structural-optimization.md`(状态 Agent Approved v1.0)§4.3
「readback 内存型与 FIF 流水结构面(联动登记)」L2 行(L97):

> **L2(FIF=2)**:加性 API `submit_persistent_frame`(至 vkQueueSubmit 止,含 slot-reuse bounded
> wait)+ `collect_persistent_frame`(当帧 wait + query + readback 后移);**per-slot cmd/params/
> descriptor/query/readback 双缓冲**;既有 `execute_persistent_frame` = submit+collect 顺序调用等价
> 形态 0-byte 保留(既有消费方零漂移);**数据依赖正确性 = 逐帧 digest 序列与 FIF=1 全等(500 帧
> 压测机核)**。

关键事实:①per-slot 枚举**不含 AS/实例缓冲**——这就是 #90 的语义缺口;②冻结确定性协议 =
「逐帧 digest 序列与 FIF=1 全等」;③§5 映射表明言 §4.3 「无 spec 落点(运行时实现面……经契约
§8.x + G14PLUS_RECORD 承载)」⇒ 修订落点 = RFC 条款行本体 + §9.2 版本行,**不触 spec/ 条款号**;
④**「共享 host 写面」措辞不在 RFC-0030 §4.3 原文里**——它冻结在 render_exec.rs 的入口拒绝面
(②的消息字面 + `FrameUpdate::blas_refit` 字段文档 L479-481 等处),故修订须双落点:§4.3 L2a
条款行 + 代码拒绝面的加性平行入口(既有拒绝措辞对未 opt-in 调用字面不动)——本窗实现即后者。

### ④ 动态臂(--dyn-demo)的逐帧 host 写面

lane_body `frame_dyn`(L10462+,「顺序入口专用——FIF 流水面公共入口已拒 tlas_update,本车道恒
inflight=1,CLI fail-closed 保证」):每帧 host 写 =

1. **场景参数 uniform 60 f32**(`buffer_uploads`,FIF 兼容面——流水路走 per-slot staging);
2. **TLAS 实例变换**:`tlas_update: (0, insts, Refit|Rebuild)` → 顺序路 submit 期
   `VkAsManager::write_transforms`(vk.rs L13242:实例数必须恒定;逐 64B 槽与 host 影子 diff 仅变
   化槽上传)→ `record_tlas_update`(vk.rs L13331,BUILD/UPDATE + consume barrier,录 pass 链前)。
   MegaDyn 形状 = 2 BLAS(静态场景 + 动态发光立方体),实例集恒定、只动 transform;
3. 蒙皮臂(`frame_skin`,L10503+)另有 `blas_refit`:蒙皮输出 SSBO → pass0 后桥接 copy → BLAS 顶点
   缓冲 → UPDATE build(vk.rs `record_blas_refit` L13366,「顶点数/拓扑不变 = 合法域」)——**不逐帧
   tlas_update**(BLAS 原地 refit,TLAS 不动)。精化(侦察 explore 复核):skin 臂的顶点写为
   **GPU 内链路零 host 写**(`vkCmdCopyBuffer` 桥 + UPDATE build),其 host 写面只在骨骼 palette
   双表 + skin 参数(走 `buffer_uploads`,FIF 兼容的 per-slot staging 面);dyn 臂则**不存在**
   blas_refit(`updatable_blas: &[]`,纯刚体变换)。⇒ blas_refit 被 FIF 拒的写面本体 = BLAS 顶点
   缓冲与 AS 对象的**在飞共享**(GPU 写 vs 在飞 ray query 读同一对象),每槽副本同样消解之。
   dyn 轨迹锚:`dyn_trajectory`(lane_body L13146,帧号纯函数)→ `dyn_transform_3x4`(L13158)→
   `dyn_frame_instances`(L13169,2 槽全量实例表)→ 臂入口 L15701+。

⇒ FIF 化须分离的动态写面恰两处:**TLAS instance buffer(host memcpy)** 与 **BLAS 顶点缓冲 + AS
本体(原地 build 写)**——即 TODO #90 字面「每槽实例缓冲/BLAS 顶点副本」。

### ⑤ frame_slots≥2 时的每槽/共享资源清点

| 已是每槽副本(`PipelinedSlot` + session 建面) | 共享单份 |
|---|---|
| cmd buffer(per-slot 重录)/ fence(`fences[slot]`)/ 上传 staging(`ensure_pipelined_slot`,grow-only)/ 回读 staging(per-slot)/ timestamp query 区间(`[slot*passes*2, …)`)/ **descriptor override set**(G31 A2 `ensure_pipelined_override_set`,池尺寸 = 声明组合 × frame_slots,**池已含 acceleration structure 描述符型**——L9868+ 重写时 `as_handles` 全表传入) | session 全部 buffer/texture 资源本体 / pipeline / 声明 descriptor set / sampler / **AS 表全部表项**(每表项 = `VkAsManager` 单所有者:instance buffer + BLAS 顶点缓冲 + BLAS + TLAS + scratch——`VkBlasEntry` vk.rs L12486+) |

⇒ **「每槽 AS 描述符集」的基建已在树**(A2 override set 池含 AS 型 + `Bindings.accel_structs` 即
session AS 表下标,`binding_overrides` 可逐帧换绑);缺的只是「允许动态更新入流水的提交入口 + 槽
纪律校验」。每槽实例缓冲/BLAS 顶点副本无须新建面——**session AS 表多份同构表项天然各持独立副本**
(单所有者纪律逐表项维持)。

## 2. 每槽副本设计(实现 = render_exec_g37_fif_dyn.rs,body-include 加性面)

```
调用方(probe):session AS 表 = S 份同构表项(组 [0,S), S = frame_slots)
逐帧 k(slot = next_frame_slot() = k % S):
  update = { tlas_update: (slot, insts(k), Rebuild|Refit),
             binding_overrides: [(rq_pass, accel_structs=[slot])] }   ← 每槽 AS 描述符集
  submit_with_frame_update_slot_as(prov, update, group)              ← 新平行入口
    ① 同源校验序(frame_update_state → provenance → validate)
    ② g37_validate_slot_as_frame(纯 host,fail-closed):
       组长==frame_slots ∧ 组界内 ∧ tlas/blas 目标==base+slot(错槽/组外拒)
       ∧ 各 pass 组内 AS 绑定==base+slot(跨槽绑定拒)
    ③ g37_submit_pipelined_frame_slot_as(submit_pipelined_frame 复制适配体,三处插入):
       slot fence 等待+reset → 【插②:host write_transforms 落本槽副本——
       本槽上一票据已完成 ⇒ 该 instance buffer 无在途 device 读】→ per-slot
       override set 重写(as_handles 全表)→ staging → per-slot cmd 重录:
       守卫 barrier → staged copies → 【插③:as_ops 经同一录制事实源
       record_frame_body——TLAS build 录 pass 链前 / BLAS refit 桥录 after_pass
       后,与顺序路逐字同形】→ 帧尾 staged readback → submit(不等 fence)
  collect(ticket)  ← 既有半程原样
```

- **写面按槽分离的语义论证**:host 写(instance buffer memcpy)只落本槽副本,时序钉死在本槽
  fence 等待之后(与既有 per-slot staging/override-set 复用纪律同一根据);GPU 写(TLAS/BLAS
  build)落本槽副本对象、序于守卫 barrier 之后 ⇒ 「共享 host 写面在飞帧读取中不可改写」的前提
  被结构性移除,而非绕过。
- **确定性**:Rebuild 下逐帧 AS 内容 = 纯函数(本帧实例数据);组内表项同构创建 ⇒ Refit =
  f(创建期拓扑, 本帧数据) 同构。⇒ 固定轨迹逐帧 digest 与单槽顺序**逐字节相等**(判据,非假设
  ——由 probe 三臂物理核验)。守卫 barrier/录制事实源/回读 staging 全走既有面,digest 等价的其余
  机制根据与 §4.3 L2「与 FIF=1 全等」论证共享。
- **成本**:AS 面内存 ×S(instance buffer/BLAS/TLAS/scratch/顶点缓冲副本)——opt-in 显式代价,
  probe 场景微小;生产规模预算门留接线窗(RFC 草案 §5 备选行登记)。
- **不动面**:既有 `submit_with_frame_update` 拒绝字面/顺序入口/守卫 barrier/双 TLAS·双 BLAS 面
  (本入口恒 None + 防御性复核)/`VkAsManager` 零改动。

## 3. 改动清单

| # | 文件 | 性质 |
|---|---|---|
| 1 | `src/rurix-rt/src/render_exec_g37_fif_dyn.rs` | **新文件**(752 行,render_exec 首个 body-include;`SlotAsGroup` + `g37_validate_slot_as_frame`〔pub,probe selftest 直调同一事实源〕+ `next_frame_slot`/`frame_slot_count` 只读簿记 + `submit_with_frame_update_slot_as` 公共入口 + `g37_submit_pipelined_frame_slot_as` 内部提交体 + `#[cfg(test)]` 3 单测) |
| 2 | `src/rurix-rt/src/render_exec.rs` | **尾部追加 4 行**(注释 + include;vk.rs×vk_g37_async_lanes 先例同律)+ **1 行机械愈合**(§7-1:L1695 `frame_update_state` 调用随并行窗签名迁移补第三参 `None`,行为 0 变) |
| 3 | `src/rurix-render/src/bin/g31_fif_dyn_probe.rs` | **新 harness**(1230 行;三臂 + device RED 双臂 + `--selftest` 4 项 + `#[cfg(test)]` 4 测 + evidence sidecar `rurix.g31.fif_dyn_probe.v1`;`#![forbid(unsafe_code)]`;两 kernel 为 bin-local 手编 SPIR-V,逐字改置自 g31_frame_cut_arm `frame_cut_*_spv`〔m94 形制〕——冻结 kernels/*.rx 与 SPV 全 0-byte,无新 rurixc 编译面) |
| 4 | `src/rurix-render/Cargo.toml` | 加性 `[[bin]] g31_fif_dyn_probe` + `required-features = ["vulkan"]`(default 构建零回归) |
| 5 | 本目录 REPORT.md + RFC_DRAFT_RFC0030_amendment.md | 交付档 |

## 4. probe 三臂与判据(g31_fif_dyn_probe)

- **场景**:2 BLAS(地面 quad 2 tri + 单位立方体 12 tri)× 2 实例(地面 identity 静止 + 立方体
  沿 +x 匀速 0.1/帧,y=0.75 悬空防共面 tie);光线 = 针孔网格(默认 64×48,`--rays`),创建期一次
  上传——**逐帧唯一变量 = TLAS 实例变换**(轨迹 = 帧号纯函数,f32 闭式)。pass 链 = fd_clear
  (哨兵 0xFFFFFFFF canary)→ fd_rq(ray query 命中流:[committed, t_bits, instance_id, prim]/线)。
- **臂 A**:单槽顺序 `execute_with_frame_update` + `tlas_update(0, insts(k), action)`(现行为;
  dyn/HZB/skin 车道同形,session frame_slots=2 顺序语义)。
- **臂 B/C**:inflight=2/3——AS 表 2/3 份同构副本,`submit_with_frame_update_slot_as` FIFO 真流水
  (submit k → 满深度 collect 最早;槽轮转 `slot == k%S` 逐帧断言);逐帧 `tlas_update` 落本槽副本
  + `binding_overrides` 把 fd_rq 的 AS 绑定换到本槽副本。B 首跑前置 **device RED 双臂**:错槽
  tlas / 跨槽绑定注入 → 必拒(拒因字面断言;校验全在提交前,session 零污染)。
- **判据(fail-closed,任一破缺 exit 1)**:① B/C 与 A 逐帧 digest 序列**逐字节相等**(sha256/帧,
  首异帧号落 evidence)② 三臂各自双跑位级(重建会话重放)③ validation ERROR = 0(telemetry
  实数)④ 动态见证:逐帧地面/立方体命中皆 >0 + digest 序列非常量 + 哨兵零残留 + 命中实例 ∈{0,1}
  ⑤ RED 双臂必拒。**帧时 measured 登记不设通过线**:逐臂 wall_ms/ms_per_frame + gpu 逐 pass/
  cpu_record/cpu_submit/cpu_fence 中位(FIF 收益口径 = CPU record/submit/fence 解耦;微场景 GPU 段
  近零,收益读数以 cpu_fence_ms 中位与 wall_ms 对照为准,evidence `measured_note` 已注)。
- **`--action refit` 对照臂**:UPDATE 语义(A 槽刷新历史 = k−1,B/C 槽副本历史 = k−S;拓扑创建期
  同构 ⇒ 预期仍逐字节等,实测非纯时按 RFC 草案修订行 2 降档「按槽稳定」登记,不充逐字节绿)。
- **`--selftest` 纯 host 4 项**:槽纪律校验器红绿臂(**rt 事实源 `g37_validate_slot_as_frame`
  直调**,非镜像)/ 槽环写面隔离模型(FIFO 交错下帧 k 写槽 k%S 前同槽前帧 k−S 必已 collect)/
  轨迹确定性(双跑位级 + 相邻帧可辨 + 静态实例恒等)/ kernel 结构(magic/RayQueryKHR capability
  面/入口名/几何流长度)。`#[cfg(test)]` 同判据 4 测双承载;rt 侧另有 3 单测(绿臂/RED 七形/
  槽环模型)。
- **三态纪律**:无 loader → `skipped_dev_env` 退 0;会话创建 Err 匹配 dev-env 形(loader/物理设备/
  扩展/feature)→ 同 skip;`RURIX_REQUIRE_REAL=1` 翻硬红(g35/async 同律)。

## 5. 验证记录(2026-08-30,dev profile,纯 host,零 GPU)

| 项 | 结果 |
|---|---|
| `cargo check -p rurix-rt --features vulkan` | 绿(exit 0;本窗文件零警告——12 条 warning 全在 vk_m50_rt_body/vk_g31_ser_body/vk.rs 存量+并行窗面,未触) |
| `cargo check -p rurix-rt`(default) | 绿(exit 0;render_exec 经 feature vulkan 门控,default 连编译面都不触) |
| `cargo check -p rurix-render --features vulkan --bin g31_fif_dyn_probe` | 绿(exit 0;bin 零警告) |
| `cargo check -p rurix-render`(default) | 绿(exit 0;bin 经 required-features 门控不入 default) |
| `cargo check -p rurix-render --features vendor-upscale --bin g14_3_pipeline_perf --bin g31_window_present` | 绿(exit 0;生产两 bin 未受扰旁证——含 §7-1 愈合行) |
| `cargo test -p rurix-rt --features vulkan --lib g37_fif_dyn` | **3/3 过**(绿臂/RED 七形/槽环隔离;209 测滤除——按窗口许可只跑新增模块,登记) |
| `cargo test -p rurix-render --features vulkan --bin g31_fif_dyn_probe` | **4/4 过** |
| `g31_fif_dyn_probe --selftest`(dev 构建,纯 host) | **4/4 PASS,exit 0** |
| clippy(补充) | 本窗两文件在 `cargo clippy --features vulkan` 下**零告警**;crate 级 exit 101 全来自存量(vk.rs:22057/22123-22126、vk_m50_rt_body.rs:65/85 缺 SAFETY——async 窗 REPORT 已登记同一清单),非本窗引入,未触(W3 纪律不顺手改) |

## 6. 主 agent GPU 判档命令(本机 GPU,PowerShell;dev profile,禁 release)

```powershell
# ① host selftest(无 GPU;应 4/4 PASS 退 0)
cargo run -p rurix-render --features vulkan --bin g31_fif_dyn_probe -- --selftest

# ② 校验层冒烟(小规模;ERROR 级即翻红——telemetry validation_error_count 判据内建)
$env:RURIX_VK_VALIDATION = "1"
cargo run -p rurix-render --features vulkan --bin g31_fif_dyn_probe -- --frames 12 --rays 32x24
Remove-Item Env:RURIX_VK_VALIDATION

# ③ 判档主跑(Rebuild 硬门;三臂各双跑 + device RED 双臂内建)
cargo run -p rurix-render --features vulkan --bin g31_fif_dyn_probe -- --frames 48 --rays 96x72 `
  --out artifacts/day_0830_delivery/w3_deep/fif_dyn/evidence_fif_dyn_rebuild.json

# ④ Refit 对照臂(UPDATE 语义;若 RED 且仅 digest 门破——按 RFC 草案修订行 2
#    降档「按槽稳定」登记,Rebuild 硬门不受影响)
cargo run -p rurix-render --features vulkan --bin g31_fif_dyn_probe -- --frames 48 --rays 96x72 `
  --action refit --out artifacts/day_0830_delivery/w3_deep/fif_dyn/evidence_fif_dyn_refit.json
```

- 判读:`verdict` = PASS / RED(exit 1);`gates.*` 五门布尔;`arms.*` 帧时 measured(FIF 收益
  登记:比较 `a_seq` vs `b_fif2`/`c_fif3` 的 `ms_per_frame` 与 `cpu_fence_ms_median`——微场景下
  GPU 段近零,预期收益形态 = FIF 臂 fence 等待与 record 重叠,`wall_ms` 略降或持平;不设通过线)。
- RED 处置:digest 首异帧号在 `failures[]`;先复跑排噪(双跑门会先破),复现则本窗 no-go 留档
  (RFC 草案维持 Draft,#90 登记「判档 RED + 根因待查」——异步窗 no-go 同律)。

## 7. 诚实分界与风险登记

| # | 项 | 状态 |
|---|---|---|
| 1 | **基线愈合 1 行**:并行会话(「G37 W3 hzb_skin」窗)本会话内给 `frame_update_state` 加第三参 `blas_b` 并迁移了三处调用,漏了 `submit_with_frame_update`(L1695)——基线一度编不过(与我方改动无关,已单变量核实)。本窗补 `None` 第三参(机械适配,`blas_b=None` 按其自身文档 = 既有面 0-byte),并附注释。**该行属既有函数体,如实登记为纪律例外**;若 hzb_skin 窗对该行另有安排,以其为准回改零成本 | 已愈合,行为 0 变 |
| 2 | render_exec.rs 行号在本会话内漂移两轮(hzb_skin 窗并行合入)——本报告行号为末次复核值,消费时以字面锚为准 | 登记 |
| 3 | `g37_submit_pipelined_frame_slot_as` 为 `submit_pipelined_frame` 的**复制适配体**(三处插入外逐字同形)——为守「既有行 0 改写」以复制换安全,存在双源漂移风险;**正式化(RFC 落地)时应折叠回单源**(既有函数加 `Option<&SlotAsGroup>` 参数,顺序路 0-byte 等价重构) | RFC 草案 §6-4 登记 |
| 4 | 臂 A 的 session `frame_slots=2`(API 下限,顺序语义)——「单槽」指提交纪律非槽数,与 dyn/HZB/skin 车道同形,如实登记 | 口径注记 |
| 5 | Refit 臂判据可降档(「按槽稳定」)——驱动 refit 若非纯函数,逐字节门法理上不可强求;Rebuild 硬门不受影响 | RFC 草案修订行 2 字面 |
| 6 | 内存 ×S 成本(AS 面副本)未设预算门——probe 场景微小;生产接线窗(`--dyn-demo`×FIF 解锁/蒙皮车道/预算门)不在本窗 | RFC 草案 §6-4 留窗 |
| 7 | `blas_refit` 通路在新入口已支持(槽纪律同律)但 probe 三臂未消费(判档形状 = tlas 动态;蒙皮×每槽顶点副本的 device 判档留生产接线窗) | 如实分界 |
| 8 | GPU 真跑零次(纪律);判档判决(PASS/no-go)待主 agent §6 | 留主 agent |
| 9 | rurix-rt 整 lib 测试面(209 测)未全跑——只跑新增 g37_fif_dyn 模块;触改面(render_exec 加性 + 1 行愈合)有生产两 bin 编译旁证 + probe/rt 7 测覆盖 | 登记 |
| 10 | **跨任务观察**(顺手登记,非本窗职责):frame_cut_as 兄弟窗的 `g31_frame_cut_arm.rs` 帧循环 `FrameUpdate.readback_subset: None` 但随后消费 `out.readbacks[0]`——按 render_exec 字面 `None` = 本帧不 readback(输出空),其 GPU 首跑可能索引 panic;建议主 agent 验收该窗时改 `Some(vec![0])` | 提示 |

## 8. TODO #90 登记建议(主 agent GPU 判档后)

PASS ⇒ 「FIF×动态共存**判档在案**:每槽 AS 副本(session AS 表同构表项 ×frame_slots,每槽实例
缓冲/BLAS 顶点副本天然独立)+ 每槽 AS 描述符集(G31 A2 override set 既有基建 × binding_overrides
轮换)+ 加性平行入口 `submit_with_frame_update_slot_as`(槽纪律 fail-closed 三判据);三臂等价门
逐字节绿(evidence 路径);RFC-0030 §4.3 L2 修订行草案在档待登记。**留窗** = 生产接线(dyn/skin
CLI 解锁 + 蒙皮每槽顶点副本 device 判档 + 内存预算门)+ 复制适配体折叠回单源」。RED/no-go ⇒
measured 证据留档,草案维持 Draft,不充绿。
