# G38 T3:frame_cut 增量 refit(桥接多 region + 计时分解 + 簇粒度降档)实施报告

日期:2026-08-30 · 仓:h:\rurix · agent:T3(窗口 frame_cut 臂)
目标:把逐帧 BLAS refit 的 27ms(s09 实测 exec_ms 均 27.06,fence_ms≈25.5~28.5 占绝对主体)降下来,
90fps 预算 11.11ms。手段 = ①桥接 copy 多 region 化(只搬 cut 差集脏槽,替代恒 75MB 全量桥)
②簇粒度降档臂 --min-level(UPDATE build 全 BLAS 扫地板下降)③copy/build GPU 计时分解(归因)。
**本 agent 无 GPU 锁——全部 device 验收留给主 agent 批次 1(§5 命令清单)。**

---

## 1. 实施摘要

| 件 | 状态 | 一句话 |
| --- | --- | --- |
| 桥接 copy 多 region 化 | 已实施 | `BlasRefitBridgeExt.copy_regions` 走**新执行入口**(等价形态,见 §3 设计裁决);arm 差集循环顺带收集脏区段(相邻槽合并),帧 0 全量单 region;`--refit-copy incr|full` 两态对照旋钮,incr 为默认(窗口臂经旧入口自动受益) |
| 计时分解 | 已实施 | query pool 尾部**追加区** 3 时戳(桥首/copy 后/build 后),逐 pass 时戳口径 0 改动;`DeviceFrameTelemetry.blas_bridge_{copy,build}_gpu_ms` 加性字段,query 读取失败 fail-soft `None`;落 probe 逐帧 evidence |
| 簇粒度降档臂 | 已实施 | `--min-level N`:竞技场只装 level≥N 簇(+链兜底根);cut 走「level<N → 首个 level≥N 祖先」**提升映射**(生产 `verify_cut_coverage` 提升后复核 fail-closed);visible_cluster_set.rs **0 字节改动** |
| 冻结面纪律 | 全守 | g31_window_present.rs / g14_3_lane_body.rs / render_exec_g37_fif_dyn.rs 0 字节;FIF 拒绝面不碰;既有公开签名(`run_frame_cut_arm`/`FrameCutArmOpt`/`execute_with_frame_update*`/`record_frame_body`/`BlasRefitUpdate`/`FrameUpdate`)全部不变 |
| check/test | 全绿 | cargo check 两 bin EXIT=0 且 warning 集与改动前基线逐条一致(rt 15 / window 4 / probe 0,零新增);cargo test host 面 rurix-rt 115 passed / rurix-render 583 passed |

## 2. 改动清单(文件 + 行号,均为加性)

### src/rurix-rt/src/render_exec.rs

| 行号 | 改动 |
| --- | --- |
| L464 | 新 pub struct `BlasRefitBridgeExt { copy_regions: Option<Vec<(u64,u64)>>, collect_gpu_timing: bool }`(含完整语义文档) |
| L992 | `DeviceFrameTelemetry` 加性 2 字段 `blas_bridge_copy_gpu_ms` / `blas_bridge_build_gpu_ms: Option<f64>`(唯一构造点 collect_persistent_frame 同批补填;fif_dyn 不构造该结构,复核过) |
| L1594 | 新 pub 入口 `execute_with_frame_update_bridge_ext(supplied, update, bridge_ext)` |
| L1605 | 私有 `execute_with_frame_update_inner`(= 原 `execute_with_frame_update_dual_tlas_ex` 主体整体迁入 + bridge_ext 参数;既有三入口改为转发,None 路径逐字等价) |
| L2740 | 新 `validate_bridge_ext`(fail-closed:regions 须与 blas_refit 同现;逐段 4 对齐正段/升序不重叠/落 `[0, byte_len)`;空列表合法 = 跳 copy) |
| L5811 | 私有 `BridgeRecordExt<'a> { regions, query_base }`(native 录制形;**独立新类型而非给 `BlasRefitRecord`/`AsFrameOps` 加字段——两者被 fif_dyn L507/L516 字面量构造,加字段即打崩 T2 冻结面**) |
| L5834 | `record_frame_body` 原签名保留,一行转发 `_ex(…, None)`(fif_dyn L544 / 创建期 L8563 / FIF 流水 L10391 三调用点 0 改写) |
| L5852 | `record_frame_body_ex`(原主体 + bridge 参数);桥接段(主 refit 臂)改造:regions `None`=原单 region 命令流逐字 / `Some(rs)` 非空=一次 `vkCmdCopyBuffer` 携 `VkBufferCopy[]`(src_offset+off → dst off,同布局) / `Some(空)`=跳过 pre-barrier+copy+post-barrier 三步(build 照录);时戳 3 点(reset+write,STAGE2_ALL_COMMANDS);**blas_refit_b(hzb_skin 表 1)臂不开放不改动** |
| L6739 | `NativePersistentFrame.bridge_query_base: u32`(创建期 = passes×2×slots,构造点 L9296 同式) |
| L8370 | query pool `query_count += BRIDGE_QUERY_COUNT`(追加区不开启时从不 reset/写/读,既有区间下标 0 漂移) |
| L9326 | `const BRIDGE_QUERY_COUNT: u32 = 3` |
| L9403(execute_persistent_frame) | 加 `bridge_ext` 参数;桥计时读取判据 = `collect_gpu_timing ∧ update.blas.is_some()`(写/读同源,`needs_rerecord` 由 blas Some 结构性保证 ⇒ 无 WAIT_BIT 悬垂);调用点 L1418(execute_with_provenance,None)/L1720(inner,实参) |
| L9419(submit_persistent_frame) | 加 `bridge_ext` 参数;重录路构造 `BridgeRecordExt` 传 `record_frame_body_ex` |
| L9741(collect_persistent_frame) | 加 `bridge_query: Option<u32>` 参数;L9808 起追加区单独 `vkGetQueryPoolResults`(64BIT|WAIT;失败 fail-soft None 不拒帧),×timestampPeriod 换算 ms;FIF collect 调用点 L1858 恒 None(FIF 拒 refit) |

### src/rurix-render/src/bin/g14_3_lane/g31_frame_cut_arm.rs(probe 与窗口 bin include! 共享单源)

| 行号 | 改动 |
| --- | --- |
| L81 | 新 `FrameCutArmExtOpt { copy_full: bool, min_level: u32 }` + `default_ext()`(incr + 0;**新类型承载——`FrameCutArmOpt` 字段集冻结**,窗口 bin L8112 字面量构造) |
| L151 | `FrameCutFrameStat` 加性 5 字段:`cut_tris_promoted`/`copy_regions`/`copy_bytes`/`bridge_copy_gpu_ms`/`bridge_build_gpu_ms`(窗口 bin 不逐字段消费 stat,复核过) |
| L171 | `FC_NO_SLOT = u32::MAX` 无槽哨兵 |
| L185 | `frame_cut_arena_layout_ext(blocks, pt_len, min_level, min_parents_all)`(占槽判据 = level≥N ∨ 链根;哨兵槽不入 owner 二分表;旧 `frame_cut_arena_layout` 转发 `(…, 0, &[])` 逐字等价) |
| L232 | `frame_cut_min_parents`(最小 id 父映射,生产 `apply_page_fallback::min_parents` 同律;帧无关逐块预计算) |
| L250 | `frame_cut_merge_region`(升序追加流相邻合并;selftest 直测) |
| L267 | `frame_cut_promote_min_level`(提升映射:上行至首个 level≥N 祖先/根兜底 → 替换祖先 children 可达域内成员标记撤出〔含 replacement 间后代消除〕→ 升序去重;覆盖性由调用方生产 verify 复核,不自证) |
| L458 | `frame_cut_select_ext`(生产链不动:`select_lod_cut_grouped` 原 DAG 原样 → verify 原 cut → 提升 → **verify 提升后**;返回〔提升后 set, 提升后簇数, 提升前 tris, 提升后 tris〕;旧 `frame_cut_select` 转发 (0, &[])) |
| L862 | `frame_cut_run_session` 加 `ext`/`min_parents_all` 参数(私有,probe/窗口不直调);差集循环顺带 `frame_cut_merge_region` 收集脏区段(canonical 槽升序前置);帧 0 恒 None(全量单 region);full 臂恒 None;差集簇无槽 = fail-closed assert;帧 0 折叠槽计数只数占槽簇 |
| L1141 | 执行换 `execute_with_frame_update_bridge_ext`(refit 帧携 `BlasRefitBridgeExt{copy_regions, collect_gpu_timing: true}`,非 refit 帧 None;全路径引用 `rurix_rt::render_exec::…`——body 的 use 列表冻结不扩) |
| L1251 | `run_frame_cut_arm_ext`(min_level 域校验〔超包内最大层 fail〕+ min_parents 逐块预计算 + layout_ext;双跑两遍同 ext;单调门**仍用提升前 cut_tris**——LOD 判据面,提升是表示层映射;旧 `run_frame_cut_arm` 转发 default_ext ⇒ 窗口臂自动 incr 化) |
| L1401 | `frame_cut_finish_ext`(schema 保持 v1 + 加性字段:顶层 `refit_copy_mode`/`min_level`,逐帧 5 新字段〔Option→null fail-soft 如实〕;汇总行加 bridge_gpu copy/build 均值;旧 finish 转发 default_ext) |
| selftest | 新 ⑥ 段:min_parents 锚/5 组提升映射含 verify/降档布局(槽基+哨兵+owner 二分+全量流+施加折叠)/脏区段合并(合成块构造 0 改动——并行 T1 #96 UV 表最终挂 `ClusterPack` 级,`ClusterPackBlock` 字面构造面维持) |
| `frame_cut_full_stream`/`frame_cut_apply_cut` | 哨兵槽跳过(min_level=0 时无哨兵,行为逐字不变) |

### src/rurix-render/src/bin/g31_frame_cut_probe.rs

| 行号 | 改动 |
| --- | --- |
| L25-31 | 用法文档补两旗标 |
| L108-113 | `--refit-copy incr|full`(默认 incr)/`--min-level N`(默认 0)解析 |
| L140-144 | copy 模式闭集校验 fail-closed |
| L246-259 | 构造 `FrameCutArmExtOpt` + 旗标 stderr 登记;调 `run_frame_cut_arm_ext` + `frame_cut_finish_ext` |

## 3. 多 region 设计(关键裁决)

**为什么不给 `FrameUpdate`/`BlasRefitUpdate` 加可选字段**:两结构被 g31_window_present.rs(L4772/L5955,禁改)与 fif_dyn(L234/L507,T2 冻结)以**完整字段字面量**构造(无 `..Default::default()`),加任何字段 = E0063 打崩两个禁改文件。任务书「或等价形态」条款 ⇒ 选**新执行入口**形态:

- 公开面:`BlasRefitBridgeExt`(新类型)+ `DeviceFrameSession::execute_with_frame_update_bridge_ext(prov, update, Option<&ext>)`(新方法)。既有全部入口转发 inner(None),命令流/provenance/遥测逐字节不变。
- 贯穿链(全部私有面,fif_dyn 均不触及):inner → `execute_persistent_frame`/`submit_persistent_frame` 加参 → `record_frame_body_ex`(旧签名转发包装,fif_dyn 调旧的)→ 桥接段。桥读取判据与录制判据同一来源(`ext.collect_gpu_timing ∧ blas.is_some()`),`PersistentFrameTicket`(fif_dyn L640 字面量构造)不加字段。
- region 语义:`(off,len)` 相对 refit 窗,`src[src_offset+off …] → vbuf[off …]`(src 与 vbuf 同布局前提 = 本臂竞技场事实);升序不重叠 + 4 对齐 + 界内,`validate_bridge_ext` fail-closed。屏障计划不变(pre/post barrier 仍覆 `[0, byte_len)` 全窗,保守正确)。`vkCmdCopyBuffer` 原生收 region 数组(`dev.cmd_copy_buf` 裸 fn 指针带 count+ptr),vk.rs 0 改动。
- **空 region 列表 = 合法**:本帧 cut 无变化(cut_every>1 相邻 refit 帧可现),跳过 copy 三步、UPDATE build 照录——vbuf 已与 arena SSBO 位级同步,与 full 全量覆写同字节 ⇒ digest 等价;且保证 refit 帧 AS build 序列两态一致(digest 等价的结构性前提)。
- 合并策略:arm 差集循环按 canonical(块,簇)升序走,slot_base 升序分配 ⇒ 脏槽偏移单调;`frame_cut_merge_region` 相邻(`last.off+last.len == off`)即并段。s09 口径 changed_slots 111~227 ⇒ 合并后段数 ≤ 该量级,单次 `vkCmdCopyBuffer` 携数组完全常规。
- incr/full 位级等价论证:host 上传两态逐字节同(增量上传既有);vbuf 帧 k 终态 = arena SSBO 帧 k 内容(incr:归纳——帧 0 全量,其后每帧脏集 ⊇ 变化字节;full:平凡)⇒ UPDATE build 输入序列位级同 ⇒ 同设备同驱动 digest 序列位级同(双跑判据同一依赖,GPU 批次 B3 机核)。

## 4. 降档口径设计(--min-level N)

**visible_cluster_set.rs 0 字节改动**(生产金标准 `select_lod_cut_grouped`/`verify_cut_coverage` 签名与行为 0 动)。实现 = 任务书预案 B「cut 后叶→level N 祖先提升映射」:

1. 生产 select 在**原 DAG 原样**跑 + verify 原 cut(既有链逐字)。
2. `frame_cut_promote_min_level`:cut 内 level<N 成员沿 `min_parents`(最小 id 父,`apply_page_fallback` 生产同律)上行至**首个** level≥N 祖先;链上全 <N 时以链根兜底(根如实保留,可能 level<N)。替换祖先 children 可达域内的全部 cut 成员标记撤出(叶域 ⊆ 祖先域 ⇒ 无洞;含 replacement 间后代消除 ⇒ 无重叠)。升序去重 = 确定性 canonical。
3. **提升后再过一遍生产 `verify_cut_coverage`(原 DAG 视图)fail-closed**——提升输出是原 DAG 的合法 cut,生产校验器就是正确校验器,无需自写截断版校验的独立正确性论证(比「截断 DAG 校验」更强的口径)。
4. 竞技场:`frame_cut_arena_layout_ext` 只给「level≥N ∨ 链根」分配槽(哨兵 `FC_NO_SLOT` 不入 owner 二分表);叶层 ~1.0M tri 不再占槽 ⇒ arena_tris 近减半 ⇒ UPDATE build 全 BLAS 扫地板下降(实际值 GPU 批次 B5 登记)。
5. 判据口径:双跑位级不变;**单调门仍以提升前 cut_tris 判**(LOD 判据面为相机纯函数保持单调;提升后集合随组边界可小幅非单调,登记 `cut_tris_promoted` 为竞技场施加口径 measured);命中∈已施加 cut 判据自动用提升后 applied(竞技场事实)。
6. 边界诚实登记:①提升目标是「首个 ≥N」——DAG 跳级时可能落在 >N 层(更粗,保守正确);②多父簇沿最小 id 父链上行,可能越过另一条链上恰 level==N 的祖先(生产 fallback 同一先例语义);③块内最大层 <N 的链以根兜底(根 level<N 如实保留占槽);`--min-level` 超全包最大层 = fail-closed 拒(误配置)。

## 5. GPU 验收命令清单(留主 agent 批次 1;本 agent 无 GPU 锁未跑)

先决:构建(所有命令 PowerShell,仓根 H:\rurix 执行;RXCP 资产 `.tmp/g36_gates/wave1_geo_composition/bistro.rxcp` 在位):

```powershell
$env:CARGO_TARGET_DIR='H:\rurix\target-night'
cargo build -p rurix-render --features vendor-upscale --bin g31_frame_cut_probe --bin g31_window_present --release
$FCP = 'H:\rurix\target-night\release\g31_frame_cut_probe.exe'
$EV  = 'H:\rurix\artifacts\day_0830_g38\t3_framecut\ev'; mkdir $EV -Force | Out-Null
```

s09 基线命令形状(w4_verify.py L258-261 原样,仅 evidence 路径/新旗标不同):
`g31_frame_cut_probe --cluster-pack .tmp/g36_gates/wave1_geo_composition/bistro.rxcp --error-px 2.0 --frames 16 --step-m 0.15 --res 96x54 --evidence <out>.json`

**B0 selftest(纯 host,零 device;可锁外先跑)**:

```powershell
& $FCP --selftest   # 期望:selftest OK(…/min-level 提升+降档布局/脏区段合并…)+ PASS
```

**B1 incr 16 帧(新默认)**:

```powershell
& $FCP --cluster-pack .tmp/g36_gates/wave1_geo_composition/bistro.rxcp --error-px 2.0 --frames 16 --step-m 0.15 --res 96x54 --refit-copy incr --evidence $EV\t3_incr.json
```

期望:PASS(内建双跑位级);stderr 出现 `G38 T3 旗标 refit_copy=incr min_level=0` 与 `bridge_gpu(copy均=…ms build均=…ms copy_mode=incr)`。

**B2 full 16 帧(对照臂 = 既有恒全量语义)**:

```powershell
& $FCP --cluster-pack .tmp/g36_gates/wave1_geo_composition/bistro.rxcp --error-px 2.0 --frames 16 --step-m 0.15 --res 96x54 --refit-copy full --evidence $EV\t3_full.json
```

**B3 incr vs full 16 帧 digest 逐字节等价(硬门)**:

```powershell
python -c "import json;a=json.load(open(r'$EV\t3_incr.json'.replace('$EV',r'H:\rurix\artifacts\day_0830_g38\t3_framecut\ev')));b=json.load(open(r'H:\rurix\artifacts\day_0830_g38\t3_framecut\ev\t3_full.json'));da=[f['digest'] for f in a['frames_data']];db=[f['digest'] for f in b['frames_data']];assert da==db and len(da)==16, (da,db);print('B3 PASS: incr==full 16f digest 位级')"
```

**B4 双跑位级(跨进程;probe 内建双跑之上再加一重)**:B1 重跑一遍到 `t3_incr_r2.json`,digest 序列与 `t3_incr.json` 逐字节比对(B3 同式脚本)。

**B5 min-level 1 档(digest 自洽 + 帧时)**:

```powershell
& $FCP --cluster-pack .tmp/g36_gates/wave1_geo_composition/bistro.rxcp --error-px 2.0 --frames 16 --step-m 0.15 --res 96x54 --refit-copy incr --min-level 1 --evidence $EV\t3_ml1.json
```

期望:PASS(digest 自洽 = 内建双跑位级;**不与 min_level 0 比 digest——几何表示不同,digest 必异,这是口径不是缺陷**);stderr `min-level 降档臂 N=1 arena_tris=…`(对照 2,082,603 应显著缩减);登记 exec_ms/bridge_build_gpu_ms 相对 B1 的下降。可选 `--min-level 2` 再降一档。

**B6 evidence 新字段核对**:

```powershell
python -c "import json;e=json.load(open(r'H:\rurix\artifacts\day_0830_g38\t3_framecut\ev\t3_incr.json'));f1=e['frames_data'][1];assert e['refit_copy_mode']=='incr' and e['min_level']==0;assert f1['copy_regions']>=1 and 0<f1['copy_bytes']<75_139_596;assert isinstance(f1['bridge_copy_gpu_ms'],(int,float)) and isinstance(f1['bridge_build_gpu_ms'],(int,float)),'桥计时 null=fail-soft,需查 query 面';assert all('cut_tris_promoted' in fr for fr in e['frames_data']);print('B6 PASS: 新字段在位', {k:f1[k] for k in ('copy_regions','copy_bytes','bridge_copy_gpu_ms','bridge_build_gpu_ms')})"
```

同时核帧 0:`copy_regions==1 && copy_bytes==75139596`(全量单 region);full 臂逐 refit 帧同此。

**B7 窗口臂加性回归(增量默认化后;s09 窗口命令原样)**:

```powershell
$WIN = 'H:\rurix\target-night\release\g31_window_present.exe'
& $WIN --quality off --headless-smoke --auto-move dolly --tier 100 --cluster-lod on --cluster-error-px 2.0 --cluster-pack .tmp/g36_gates/wave1_geo_composition/bistro.rxcp --cluster-per-frame-cut on --frame-cut-out $EV\t3_window_fc.json
& $WIN --quality off --headless-smoke --auto-move dolly --tier 100 --cluster-lod on --cluster-error-px 2.0 --cluster-pack .tmp/g36_gates/wave1_geo_composition/bistro.rxcp
```

期望:两跑窗口 digest 相等(s09 加性回归同判据,历史锚 sha256:5540ecae…——frame_cut 臂在循环后不进 presented digest)+ 臂内建双跑 OK + sidecar 带新字段(copy_mode=incr 默认)。

**性能判读口径**(measured 登记不设通过线,但本窗目标):B1 vs B2 的 `bridge_copy_gpu_ms` 差 = 多 region 收益(75MB→~0.5MB/帧);B5 vs B1 的 `bridge_build_gpu_ms` 差 = 降档收益(2.08M→~1.1M tri UPDATE);exec_ms(refit均) 相对 27.06 的总降幅。若 copy 段占比小、build 段主导,则 --min-level 为主杠杆,如实登记归因。

## 6. check / test 结果(host 面,已跑)

- 基线(改动前):`cargo check -p rurix-render --features vendor-upscale --bin g31_frame_cut_probe --bin g31_window_present --release` EXIT=0;warning 集 = rurix-rt lib 15 / g31_window_present 4 / probe 0。
- 改动后同命令:EXIT=0;**warning 集逐条一致,零新增**(rt 15 条内容同、window 4 条同、probe 0)。g31_window_present 未被本 agent 编辑但 include arm 单源,已确认未打崩。
- `cargo test -p rurix-rt --release --lib`:**115 passed, 0 failed, 1 ignored**。
- `cargo test -p rurix-render --release --lib`:**583 passed, 0 failed**。
- probe `--selftest`(.exe 运行,纯 host 腿)按「不跑 .exe」纪律未执行,列 B0 由主 agent 锁外先跑。

## 7. 风险与诚实登记

1. **regions 不进 provenance**:copy 子集为执行细节(合法区段集下 vbuf 终态字节唯一),AS 内容代记账仍由 `blas_refit` bump;`next_provenance_with_update` 无需感知。若后续要把「copy 形状」纳入 provenance 口径,归 RFC 面。
2. 桥计时口径:copy 段含桥内 pre/post 屏障,build 段含 consume barrier;时戳 STAGE2_ALL_COMMANDS(逐 pass 既有同式)。追加区只在顺序路 + blas_refit + collect_gpu_timing 帧 reset/写/读;FIF 恒不触(拒 refit 面未动,L1782/L1791 区域 0 改动)。
3. sidecar schema 保持 `rurix.g31.frame_cut_probe.v1` + 加性字段(w4_verify.py 判据 = 进程 rc + 臂 OK,无 schema 断言;w4_resume.py 仅 docstring 提及)。窗口臂 sidecar 自动带新字段。
4. 窗口臂 copy 行为变化(full→incr 默认)是**任务书明示允许**的默认化;digest 等价由 B3/B7 机核。回退旋钮 = probe `--refit-copy full`;窗口臂如需回退,把 `FrameCutArmExtOpt::default_ext().copy_full` 翻 true 即全线回旧(单点)。
5. 并行合流:实施中 T1(#96 RXCP v2)先给 `ClusterPackBlock` 加 `cluster_vertex_uv` 后改挂 `ClusterPack.blocks_vertex_uv`(pack 级,注释明示为保 frame_cut selftest 夹具构造面);本臂 selftest 一度补字段后随之撤回,终态合成块构造 0 改动。最终 check 在两波合流后全绿(EXIT=0,warning 集与基线逐条一致)。若 T1 后续再动共享结构,收尾需重 check。
6. min-level 语义边界见 §4.6(提升目标可能 >N/根兜底 <N/多父最小 id 链)——全部 fail-closed 由生产 verify 兜底,不静默。
