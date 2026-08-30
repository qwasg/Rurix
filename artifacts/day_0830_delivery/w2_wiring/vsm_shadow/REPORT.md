# G37 W2 判档:VSM 页失效 + 缓存阴影生产接线(TODO #104 / #106)

日期 2026-08-29(UTC+8)。判档子任务:host `shadow/vsm.rs`(#104)与
`shadow/page_cache.rs` M19 16 帧金标准(#106)在生产 ray traced 阴影形态下的
"接线"语义是否成立。纪律:能接就接(最小可判档面),接不成 = 如实 no-go/留窗;
禁跑 GPU / 禁 release / 禁碰 target-night;冻结清单(两模块本体、窗口车道三文件、
既有 kernels、milestones/、registry/、ci/)0-byte。

## 0. 结论

**方案 A:no-go/留窗 + device 判档件。**

生产窗口车道的阴影是 RayQuery 逐像素内联遮挡射线——没有 shadow map 生成
pass,页管线的全部价值前提(光栅阴影深度生成成本的跨帧摊销)在该车道不存在;
强行接入 = 造一个无人采样的页管线(dispatch 而无消费),与 #104 原病"编而不
dispatch"同构,判档纪律拒。留窗锚沿用既有登记(见 §5)。同时交付 G31 侧独立
device 闭环判档件 `g31_vsm_device_probe`(三腿,含金标准 `dirty_depth` 轴的
**首个** device 消费腿),`--selftest` 纯 CPU 腿全绿(§4)。

## 1. 侦察事实(全部在案可复核)

### 1.1 页管线 host 面 = 冻结金标准(#104/#106 的"齐"侧)

- `src/rurix-render/src/shadow/vsm.rs`:mark(A2.1 拆分为纯函数
  `page_mark_bits`(device 核 host 镜像)+ `apply_mark_bitmap`(host 消费))、
  alloc(近级优先紧凑请求 + 帧龄 LRU 驱逐 + 本帧标记保护)、失效三源
  (`invalidate_aabb` / `invalidate_light_direction` / begin_frame 环形更新带)、
  多视图 CPU 深度光栅(仅"脏且驻留")、投影采样(缺页保守 lit)。模块头自述:
  "device 接线属 W3,本模块为对拍金标准"。
- `src/rurix-render/src/shadow/page_cache.rs`:M19 16 帧确定性脚本——跨帧
  cache hit(F1/F6 零 alloc 零 raster)、五失效原因闭集、clipmap scroll(F4)、
  local spot(F12 起)、non-virtual caster(F8–11)、multi-view batch(≥5 视图)、
  驱逐轴(F13/F14)。产出四轴 golden:`page_table` / `depth_pool` / `sample` /
  `dirty_depth` 逐帧 digest + canonical 事件序列 sha256。
  `MarkFrameSnapshot` 的字段与 kernel `vsm_page_mark_project` 输入布局**逐字段
  对齐**,`FrameDeviceSnapshot` 是三 digest 的原像(单测钉死)——金标准本来就是
  为 device 对拍腿设计的。

### 1.2 `vsm_page_mark_project` 下落(#104"编而不 dispatch"考据)

- kernel 本体:`apps/uc06-renderer/kernels/vsm_page_mark_project.rx`(冻结),
  经 uc06 `build.rs` KERNELS 表编入(feature vulkan)。
- **"编进 SPV 无人 dispatch"已于 A2.1(2026-08-07)在 M19 门层面关闭**:
  `apps/uc06-renderer/src/device_m19.rs` 逐帧(16/16)dispatch 该核 + readback
  位图与 host 镜像逐位/逐槽对拍,含 `--m19-red-skip-mark`(不 dispatch 冒充)与
  `--m19-red-host-mark`(host 预知 page id 冒充,F13+ 必分叉)两条 RED 轴,及
  `mark_depth_is_causal` 结构证据(F12→F13 唯一输入差 = 深度,位图必变)。
  在案:`.a21_evidence/A21_CLOSURE_LOG.txt`(M19 gate 18/18 PASS,mark 段
  dispatch 16/16、失配 0/0/0)、`evidence/g8_m19_vsm_page_cache_20260807T140650Z.json`。
  TODO #104 行的"**曾**「编进 SPV 无人 dispatch」"措辞与此一致(历史态)。
- 本会话复核 fixture 无漂移:`--selftest` 测得
  `host_events_sha256 = 4402d721a1496be4cd8d9822f0ed28e93b89d2cd42c8f6511a373b3009974b3c`
  == A2.1 重生成的 golden(A21 闭环日志 §3 字面)。

### 1.3 消费空缺(#104/#106 的"未进窗口车道"侧)

- 全仓 `shadow::vsm` / `shadow::page_cache` 消费点:uc06(device_m19 /
  device_kernels / main / scene / shading)、uc08-physics、
  `src/bin/g8_m19_probe.rs`(host probe)、`src/bin/g9_g93_geometry_probe.rs`、
  shadow 模块内部。**生产窗口车道三文件(g14_3_lane_body.rs /
  g31_window_present.rs / g14_3_pipeline_perf.rs)零引用。**
- 生产车道阴影语义:`kernels/g14_3_direct_gi.rx` 直接光 = RayQuery
  (`ray_query_initialize_first_hit` 遮挡射线,L103/L221/L279);day_0829
  realism 战役 soft-shadows 臂 = "点灯阴影射线 → 逐灯半径圆盘采样 N 条"
  (SMRT 简化形,TODO #27;HANDOVER §D.3)。g14 的
  `g14_3_shadow_scatter.rx` 与页管线无关(#104 行自登记)。
  **全车道无光栅 shadow map 生成/采样面。**
- 留窗登记**已在案**:`milestones/g31/g31_rejudgment_windows.json` SMRT 行
  (todo_ref #27)半②"shadow page 采样车道出现" = **miss**,verdict partial →
  maintain-defer;followup 字面:"半②窗 = VSM 页管线生产采样车道接线
  (TODO #104 VSM 页失效/clipmap 生产接线 = 同族前置面,后续期立项程序)"。
- **真空缺(本次侦察新钉)**:金标准四轴中 `FrameDigest.dirty_depth`
  (逐帧"失效→重光栅"脏页深度拼接 digest,注释明言"device multi-view gather
  对拍序")**至今无任何 device 腿消费**——uc06 mv 段只取单个 multi-view batch
  与 bin 内自算的 host_gather 对拍,不触逐帧 golden。#104 的核心语义
  ("动态物体/灯变只重绘脏页")恰恰落在这条轴上。

## 2. 判档分析

### 2.1 方案 B 检验(窗口车道找可消费点)→ 不成立

逐点核验过的候选:

1. **点光/lamp-lights 阴影**:soft-shadows 臂 = 逐像素阴影射线的圆盘采样,
   `lamp radius` 是射线语义参数(tmax/圆盘半径),不是可跨帧驻留的阴影资源;
   每帧每像素重新发射,无"生成成本"可被页缓存摊销。
2. **跨帧复用**:车道现有的跨帧设施是 TSR 时域积累(样本域,收敛半影即靠它);
   页缓存是资源域(深度页字节跨帧驻留),两者不同构,页缓存无处挂。
3. **强接的形态**:在 lane 里 dispatch mark/alloc/raster 而 shading kernel
   (RayQuery)不读页 = 无人采样的空转管线。这与 #104 历史病灶("编而不
   dispatch")只是换了一层的同构物,判档纪律(不冒充)直接否决。

### 2.2 方案 A 论证

- **语义锚**:VSM 页失效/页缓存的价值 = "只重光栅脏页,静态页跨帧复用"——
  前提是存在光栅 shadow map 深度生成 pass。生产车道阴影 = ray traced,
  该 pass 不存在,#104/#106 的"生产接线"在当前形态**语义不成立**。
- **no-go ≠ 废弃**:host 金标准活在树内回归网(`cargo test -p rurix-render
  shadow::` 34 用例)+ uc06 M19 门(CI smoke)。留窗锚已注册(SMRT 半②),
  待光栅阴影档(#105 PCSS)或 SMRT 完整版(#27)立项时,页管线即其采样车道
  的现成底座。
- **判档面补强(本次实现)**:缺一个 G31 战役侧、不依赖 uc06 门 harness 的
  可复跑 device 判档件,且 `dirty_depth` 轴(#104"页失效"的直接可观测量)
  无 device 消费。二者由 `g31_vsm_device_probe` 一并补上(§3)。

## 3. 实现说明(方案 A 的代码面)

### 3.1 新增文件

| 文件 | 内容 |
|---|---|
| `src/rurix-render/src/bin/g31_vsm_device_probe.rs` | 判档 probe(约 900 行,`forbid(unsafe_code)`) |
| `src/rurix-render/Cargo.toml` | 追加 `[[bin]] g31_vsm_device_probe`(`required-features = ["vulkan"]`,仓库既有 device bin 同例;本会话 delta 仅此 13 行,同文件另一处 `g31_visbuffer_wiring` 块系兄弟 W2 子任务) |
| `artifacts/day_0830_delivery/w2_wiring/vsm_shadow/spv/*.spv` | 四个冻结 kernel 的 rurixc 产物(CPU 面编译,§4.3) |
| `artifacts/day_0830_delivery/w2_wiring/vsm_shadow/selftest.json` | selftest 输出(§4.2) |

**冻结面 0-byte(本会话)**:`shadow/vsm.rs`、`shadow/page_cache.rs`、
`g31_window_present.rs`、`g14_3_lane_body.rs`、`g14_3_pipeline_perf.rs`、
既有 `kernels/*.rx`/`.spv`、`milestones/`、`registry/`、`ci/` 均未触碰
(git 状态中这些路径的 M 记录系 day_0828/0829 他会话既有未提交工作,
HANDOVER 在案"未 git commit")。

### 3.2 probe 三腿(全部消费冻结 `run_m19_fixture()` 金标准,судья纯 host)

- **腿⓪ mark**:逐帧(16)dispatch `vsm_page_mark_project`(输入 =
  `MarkFrameSnapshot` 逐字段;与 uc06 device_m19 同布局),readback 4096 字
  位图 → 逐位(word)+ 逐槽(`marked_slots_from_bitmap` 反解)对拍 + 越界写
  检测(`levels*512` 后必须全零)+ 位图去重 ≥2 + F12→F13 深度因果结构证据。
- **腿① invalidate→raster(新增判据面)**:逐帧把该帧"脏且驻留"页批次
  (`batches[f].pages`,= 五类失效源的直接产物)交 `vsm_depth_raster_mv`
  在 device 重建:三角形 = 方向光世界 tris 经该帧灯基(快照 right/up/fwd)
  变换 ++ local 灯空间 tris,页 meta 按 `LOCAL_LEVEL_TAG` 分派 tri 段——
  与逐帧金标准对拍:期望纹素 = 快照物理池按批次序切片(selftest 证明该切片
  的 sha256 == golden `dirty_depth`,即判读数据路径与 golden 同源);硬判据 =
  逐纹素 `max_abs ≤ 1e-6`(G7.5 冻结口径),sha256 严格臂如实登记。空批帧
  (跨帧 cache hit)核验 golden == sha256(空串)。16 帧中 9 帧非空:F0 首帧
  全建、F3 CasterMoved、F5 LightChanged、F8–11 NonVirtualCaster、F12
  local+强制脏、F13 驱逐链新页;F4 ClipmapScroll 只打非驻留槽 → 无重光栅批,
  与金标准一致(其事件在事件序列轴覆盖)。
- **腿② alloc→sample**:逐帧 dispatch `vsm_sample`(+F12 起
  `vsm_sample_local`)——device 真读页表(驻留/脏/物理页 = alloc/失效决策的
  落地态)+ 物理池,产 0/1 采样值:逐值位级对拍 + `sample` digest 对拍 +
  非退化(遮蔽臂非空)。

三腿合取 `pass`;validation 计数/messenger 位如实登记(uc06 A2.1 实数化同源)。
三态纪律:无 loader/设备 → `skipped_dev_env` 退 0(`RURIX_REQUIRE_REAL=1`
翻硬红退 1);判据不符退 1。

### 3.3 `--selftest` 纯 CPU 腿(судья自证,防"判据空转")

1. 金标准脚本判据位(冻结 fixture 八谓词 + 驱逐非零 + 16 帧齐)。
2. 逐帧四轴原像重建:`page_table`/`depth_pool`/`sample`/`dirty_depth` 均从
   快照独立重算并与 golden 相等(= device судья的期望值构造路径与 golden
   同源,无 host 代填空间)。
3. 绿臂:судья吃 host 镜像(位图/池切片/采样值)必须 16/16 全绿(三腿)。
4. 证伪臂(臂间独立):位图翻一位 → 腿⓪ судья必红;纹素扰动 +2e-6(> 容差)
   → 腿① судья必红(容差臂与 digest 臂双红);采样值 0/1 翻转 → 腿② судья
   必红。
5. 结构位:host 位图去重 ≥2、F12→F13 深度因果、采样 0/1 两臂皆非空。

### 3.4 与 uc06 M19 device 腿的关系(不重复造门)

probe **不替代** uc06 M19 门(其 RED 轴/golden 文件/ci smoke 原样有效),
差异面:① 落 rurix-render 侧,SPV 运行时 `--spv-dir` 装载,不依赖 uc06 app
harness,G31 战役可独立复跑;② `dirty_depth` 逐帧 device 消费是 uc06 腿没有
的判据面(uc06 mv 段 = 单 batch × bin 内 host_gather);③ судья被 selftest
证伪臂钉死(uc06 的 RED 轴在 GPU 侧,本件在 CPU 侧,零 GPU 即可验判据活性)。

## 4. measured 结果(本会话,全 CPU)

### 4.1 编译面

- `cargo check -p rurix-render`(默认特性,dev):**EXIT=0**(库面零回归;
  probe 因 `required-features` 不参与默认面,与既有 device bin 同律)。
- `cargo check -p rurix-render --features vulkan --bin g31_vsm_device_probe`:
  **通过**(Finished dev,本 bin 零警告)。
- `cargo clippy -p rurix-render --features vulkan --bin g31_vsm_device_probe --no-deps`:
  **本 bin 零条目**(输出 100 条 lib 既有警告与本件无关)。如实登记:带依赖的
  全量 clippy 在 rurix-rt 既有 8 处 `undocumented_unsafe_blocks`(vk.rs /
  vk_m50_rt_body.rs)上翻红,系预存问题非本会话引入,未触碰。

### 4.2 `--selftest`(纯 CPU,EXIT=0)

```json
{"subject":"g31_vsm_device_probe_selftest","frames":16,"host_events_sha256":"4402d721a1496be4cd8d9822f0ed28e93b89d2cd42c8f6511a373b3009974b3c","evict_count":4,"preimage_frames_ok":16,"green_mark":16,"green_raster":16,"green_sample":16,"raster_frames_nonempty":9,"host_distinct_bitmaps":2,"red_mark_flips":true,"red_raster_flips":true,"red_sample_flips":true,"fails":[],"selftest_pass":true}
```

`host_events_sha256` 与 A2.1 golden 逐字相等 ⇒ 冻结金标准无漂移。

### 4.3 kernel 编译(CPU 面允许;主agent可直接消费)

`cargo build -p rurixc --features vulkan-backend --bin rurixc`(dev)后,
四个冻结 kernel 经 `rurixc --target vulkan` 产 SPV,rurixc 内建校验 +
独立 `spirv-val` 双过:

| SPV(`artifacts/day_0830_delivery/w2_wiring/vsm_shadow/spv/`) | 字节 | sha256(前 16) |
|---|---|---|
| `vsm_page_mark_project.spv` | 16536 | `681cfe0d82541e70` |
| `vsm_depth_raster_mv.spv` | 11780 | `5622e0552e2c7be7` |
| `vsm_sample.spv` | 15112 | `c2a23190e71fadc6` |
| `vsm_sample_local.spv` | 8832 | `8ea49bca3eb8b9f7` |

## 5. 留窗登记字面(建议;milestones/ 冻结,登记程序归主线)

> TODO #104/#106(VSM 页失效 / 缓存阴影生产接线):判 **no-go/maintain-defer**。
> 生产窗口车道阴影 = RayQuery 逐像素遮挡射线(`g14_3_direct_gi.rx`;
> soft-shadows 臂 = 逐灯圆盘采样),无 shadow map 生成成本可摊销,页管线在该
> 车道无消费面;强接 = 无人采样的空转 dispatch,判档纪律拒。重启锚(与
> `milestones/g31/g31_rejudgment_windows.json` SMRT 行半②同锚):**光栅阴影档 /
> VSM page 采样车道出现**(#105 PCSS 或 #27 SMRT 完整版立项即触发)。届时消费
> 面基建已备:host 金标准(`shadow/vsm.rs` + `page_cache.rs`,冻结)+ uc06 M19
> device 门(A2.1)+ G31 侧独立判档件 `g31_vsm_device_probe`(mark /
> invalidate-raster / alloc-sample 三腿,golden `dirty_depth` 逐帧 device 消费)。

## 6. 主agent GPU 步骤(全部工件已就位;建议 GPU 锁内,双环境变量)

```powershell
# ① device 腿(probe exe 与 SPV 均已编好;dev target,不碰 target-night)
$env:RURIX_REQUIRE_REAL="1"; $env:RURIX_VK_VALIDATION="1"
target\debug\g31_vsm_device_probe.exe `
  --spv-dir artifacts\day_0830_delivery\w2_wiring\vsm_shadow\spv `
  --out artifacts\day_0830_delivery\w2_wiring\vsm_shadow\device_probe.json
# 判读:退 0 且 JSON pass=true;硬判据 = mark_all_match / raster_all_match /
# sample_all_match 三真;raster_measured_max_abs 如实登记(硬线 = G7.5 冻结
# 容差 1e-6;uc06 同类臂在 4070 实测 ~1e-7);
# raster_digest_frames_matched(sha 严格臂)与 validation_errors=0 +
# validation_messenger=true 如实登记。
# 预期 dispatch 数:mark 16 + raster 9 + sample 20 = 45。

# ②(仅当工件缺失需重建时)
# cargo build -p rurix-render --features vulkan --bin g31_vsm_device_probe
# cargo build -p rurixc --features vulkan-backend --bin rurixc
# target\debug\rurixc.exe apps\uc06-renderer\kernels\<k>.rx --target vulkan -o <out>\<k>.spv
# spirv-val <out>\<k>.spv
```
