# G39 T3 — slot_as 单源折叠施工登记(2026-08-31)

**任务**:fif_dyn REPORT §7-3(`artifacts/day_0830_delivery/w3_deep/fif_dyn/REPORT.md` L211)/ WIRING_PLAN §5-1 登记治理项兑现——复制适配体 `g37_submit_pipelined_frame_slot_as` 折叠回单源:`submit_pipelined_frame` 加 `slot_as: Option<&SlotAsGroup>` 末参,None 路 0 语义等价,Some 路吸收三处插入 + 一处换向。

**改动文件(仅二,与授权面一致)**:
- `src/rurix-rt/src/render_exec.rs`(numstat +125/−14,13068 → 13179 行)
- `src/rurix-rt/src/render_exec_g37_fif_dyn.rs`(numstat +11/−389,772 → 394 行)

不 commit(纪律);禁跑 GPU(纪律,零 GPU 触碰)。工作树同时携带并行窗/已收口窗的未提交改动(T2 lane_body/pipeline_perf、frame_cut、window_present、kernels、ci、milestones、.gitignore 等)——非本窗所触,本窗 delta 以上述两文件 numstat 为准(`git diff` 存档 = 本目录 `fold_diff.patch`)。

## 1. 折叠前机械 diff 复核(在树字节为准)

实施前提取两函数体逐段 diff(`pre_base_fn.txt` = render_exec.rs L10267-10563 共 297 行;`pre_copy_fn.txt` = g37 文件 L291-649 共 359 行;diff = `pre_fold_diff.txt`)。确认与 R3 差异清单逐项吻合,语义差异恰四处:

| # | 差异 | 基函数(折叠前) | 复制体 |
|---|---|---|---|
| ① | 防御性复核 | 一律拒 `prepared.tlas`(L10276-10278) | 核 tlas/blas 目标 `== group.base+slot`,拒 `tlas_b`/`blas_b`(原 L301-320) |
| ② | host TLAS 写 | 无 | fence 等待+`ensure_pipelined_slot` 后 `mgr.write_transforms(...)`(原 L357-368) |
| ③ | as_ops | `record_frame_body(..., None, None)` 恒 None | 组装 `AsFrameOps{..}`(含 `BlasRefitRecord`)传 `record_frame_body(..., None, as_ops)`(原 L488-526+L566-567) |
| ④ | 报错前缀 | `"FIF"` ×9 处 | `"slot-AS FIF"` ×9 处(vkMapMemory/校验漏网/slot 面缺失/Reset/Begin/上判已拒/类型漂移/End/vkQueueSubmit op) |

其余差异全为注释字面(§2-5 处置)。另二处同形注意项:`let slot = native.next_slot;` 位序(基 = 拒面后,复制体 = 拒面前)与复制体错误构造短形(`.into()` 与 `format!` 混用)——见 §3 等价论证。

## 2. 吸收落点(折叠后 render_exec.rs)

折叠后基函数 = L10291-10674(签名→闭括 384 行;doc L10248-10290 共 43 行)。对折叠前基函数的全部 delta(机械 diff 存档 `post_vs_prebase.txt`,-U0 共 12 个 hunk):

1. **签名**:末参 `slot_as: Option<&SlotAsGroup>`(`SlotAsGroup` 定义在 include 文件,同模块可见,无需 use)。
2. **前缀承载(决策)**:函数头部 `let err_pfx = if slot_as.is_some() { "slot-AS FIF" } else { "FIF" };`——9 处共同体报错字面改经 `{err_pfx}` 插值,**message 内容 0 改**(两路求值产物与折叠前各自字面逐字节同,见 §3/§4);`vkQueueSubmit` op 串经 `&format!("vkQueueSubmit({err_pfx} pipelined frame)")` 同律。
3. **换向①落点**:原「一律拒 tlas」单判改为 `if let Some(group) = slot_as { 槽向防御复核(tlas/blas == group.base + native.next_slot;tlas_b/blas_b 拒) } else if prepared.tlas.is_some() { 原拒面字面 }`——None 路判序、判据、报错字面均原样;Some 路三判与复制体逐字面同(`expect_as` 取 `native.next_slot` 现值 = 复制体 `let slot` 先行取值,bump 在 slot_busy 判后,逐位同)。
4. **插入②落点**:`ensure_pipelined_slot(...)` 之后、G31 override 块之前,`if slot_as.is_some() && let Some((as_index, instances, _)) = &prepared.tlas { ...write_transforms... }`(块体与复制体逐字面同;双保险——None 路 `prepared.tlas` 已在换向①拒,守卫使 0 语义论证不依赖该前提)。
5. **插入③落点**:staged 冲刷块之后、`effective_rb` 之前,`let as_ops = if slot_as.is_none() { None } else { match (&prepared.tlas, &prepared.blas) {...} };`(match 体与复制体逐字面同,仅整体 +4 缩进);`record_frame_body` 末实参 `None` → `as_ops`(None 路求值恒 `None`,实参效果逐字维持;cleanup 位 `None` 字面不动)。
6. **调用点**(仅 1 处,`submit_with_frame_update` L1832):机械补 `None,` 末参 + 1 行注释。
7. **共同体 0 改写**:上述之外无任何 hunk——slot 占用/fence 等待+reset/懒建/G31 override/staging 上传/cmd 录制/守卫 barrier/staged copies/冲刷/effective_rb/exportable/帧尾回读/end/submit/票据全部原字面(含全部原注释)。

## 3. None 路 0 语义等价论证(逐字面)

- **判序不变**:None 路执行序 = 拒 `prepared.tlas`(原字面 message)→ `let slot` → slot_busy 判 → bump → fence 等待/reset → 懒建 → 〔插入② 守卫不触:`slot_as.is_some()` 恒假〕→ G31 override → staging → cmd 录制 → 〔插入③:`slot_as.is_none()` ⇒ `as_ops = None`,无副作用〕→ `record_frame_body(..., None, None 值)` → 帧尾 → submit → 票据。与折叠前逐语句同序同效。
- **报错字面逐字节**:`err_pfx = "FIF"` 时 9 处产物 = `"FIF slot {slot}: 上传 staging vkMapMemory 失败: {map}"`、`"FIF: 上传目标资源 {res} 非 buffer(校验漏网)"`、`"FIF slot {slot}: slot 面缺失(建面序漂移)"`、`"FIF: vkResetCommandBuffer 失败"`、`"FIF: vkBeginCommandBuffer 失败"`、`"FIF: 上传目标资源 {res} 非 buffer(上判已拒)"`、`"FIF: readback 资源 {res} 非 buffer(类型漂移)"`、`"FIF: vkEndCommandBuffer 失败"`、`"vkQueueSubmit(FIF pipelined frame)"`——与折叠前原字面逐字节相等(其中 4 处原为 `&str.into()`,现为 `format!`;String 值逐字节同,仅构造形式差,登记)。防御拒面 `"FIF 流水不支持 tlas_update(公共入口已拒;防御性复核)"` 原字面原位。
- **新增求值仅二**:`err_pfx` 绑定(纯 host,无副作用)与 `slot_as.is_none()/is_some()` 判(纯读)。无新分配、无序变化、无 unsafe 面变动。

## 4. Some 路吸收完整性(对复制体)

机械 diff 存档 `post_vs_precopy.txt`:折叠后函数对原复制体的全部差异 = (a)分支包裹三处;(b)`expect_as` 以 `native.next_slot` 替 `slot`(同位同值,`let slot` 后移至分支后——纯读重排,无观测差);(c)9 处前缀字面 → `{err_pfx}`(Some 路求值 = `"slot-AS FIF"`,产物与复制体字面逐字节同);(d)注释回归基函数原字面(复制体注释中有历史价值的两点——守卫 barrier 对 AS build 的全序论证、staged 冲刷先于 AS build 的 refit src 可见性论证——已迁入基函数 doc「G39 T3 单源折叠」节第 3 条);(e)插入③整体 +4 缩进。无语义 delta。Some 路专属报错字面(tlas/blas 错槽、tlas_b/blas_b 拒、无 AS 面 ×2、AS 包空)逐字保留(tlas_b/blas_b 拒行因缩进换行重排,字面同)。

## 5. render_exec_g37_fif_dyn.rs 处置

- **删除**:复制体整函数 + 其 doc(原 L272-649,378 行)。doc 中三处插入语义、Safety 契约已迁入基函数 doc;「复制换安全」的历史根据在本 REPORT §1/原 fif_dyn REPORT §7-3 存证。
- **公共入口 `submit_with_frame_update_slot_as`**:内部调用改 `submit_pipelined_frame(..., Some(group))`(2 行);**对外签名/校验序 0 改**;`SlotAsGroup`/`g37_validate_slot_as_frame` 0-byte(git diff 无 hunk 佐证)。
- **注释随实况改写 4 处**:文件头「纪律」段(复制适配体陈述 → 折叠现状陈述+登记指针)、入口 doc 的 `[g37_submit_pipelined_frame_slot_as]` 链接(→ `submit_pipelined_frame` slot_as 分支)、校验序注释④、SAFETY 注释。
- 单测模块 `g37_fif_dyn_tests`(3 测)0-byte。文件尾接缝空行修复 1 处(删除段吞并空行致 `}` 与分隔注释黏连,恢复规范单空行)。

## 6. 耦合冻结注记更新(render_exec.rs 原 L5805-5809)

原注记「两结构(BlasRefitRecord/AsFrameOps)被 render_exec_g37_fif_dyn.rs 以字面量构造(T2 冻结面),加字段即打崩其编译」改写为现状陈述:G38 当窗理由保留 + **G39 T3 折叠后该跨文件构造面已消失**(grep 佐证:`BlasRefitRecord {`/`AsFrameOps {` 字面量构造现仅存 render_exec.rs 内两处——顺序路 as_ops 归并〔L9561/9593/9602〕与本折叠 slot_as 分支;g37 文件 0 处),加性纪律以新类型承载维持。**登记**:该 T2 冻结面自此收缩为 render_exec.rs 文件内耦合,后续给两结构加字段仅需同步本文件构造点。

## 7. 验证结果

| 项 | 命令 | 结果 |
|---|---|---|
| 消费方 1 | `CARGO_TARGET_DIR=H:\rurix\target-night cargo check --release -p rurix-render --features vendor-upscale --bin g14_3_pipeline_perf` | **rc=0**;rurix-rt lib 17 warning 全为未触碰文件既有项(vendor_upscale.rs/vk_m50_rt_body.rs/vk_g31_ser_body.rs/vk.rs),`render_exec*` 0 warning ⇒ **0 新增** |
| 消费方 2 | 同上 `--features vulkan --bin g31_fif_dyn_probe` | **rc=0**;12 warning 同上既有项,0 新增 |
| rt 单测 | `cargo test --release -p rurix-rt --lib --features vulkan -- g37_` | **5 passed / 0 failed**(`g37_fif_dyn_tests` 3 测〔green/red/槽轮转〕+ 相邻 `g37_async_lanes_tests` 2 测),207 filtered |
| rt 全 lib 测 | 未跑(如实登记) | 默认特性下 `render_exec` 模块整体 `#[cfg(feature = "vulkan")]` 门控不编译(首跑 0 命中即此因);带 vulkan 的全量 212 测未筛跑——禁 GPU 纪律下不排除个别测触驱动,循 fif_dyn 窗先例(其 §7-9 同项登记)只跑目标模块 |
| git 自查 | `git diff --stat/--numstat` | 本窗改动仅两 rt 文件;共同体块字面未重写(`post_vs_prebase.txt` 12 hunk 全为计划项);公共入口签名未变(g37 文件 diff 无签名 hunk) |

## 8. 偏离与登记

1. **错误构造形式**:共同体 4 处 `&str.into()` 改 `format!("{err_pfx}: ...")`——产物 String 逐字节同,构造形式差属前缀承载方案的必然代价(任务建议路线)。
2. **纯读重排(Some 路对复制体)**:`let slot = native.next_slot;` 从防御复核前移至分支后(None 路原序不变);`expect_as` 以 `native.next_slot` 直取(同点同值)。无观测差。
3. **单测目标目录**:g37 过滤测跑在默认 `target\`(env 未跨 shell 持久),非 target-night;结果有效,登记。
4. **本目录证据件**:`pre_base_fn.txt`/`pre_copy_fn.txt`/`pre_fold_diff.txt`(折叠前)、`post_fold_fn.txt`/`post_vs_prebase.txt`/`post_vs_precopy.txt`(折叠后双向对拍)、`fold_diff.patch`(两文件全量 diff)。`baseline/` 归主 agent B2 零语义门(静态 FIF/dyn/skin ×1|2|3 + fif probe 位级复证),本窗不跑。
