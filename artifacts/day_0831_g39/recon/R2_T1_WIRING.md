# R2 — 窗口 bin 加性臂接线模式交接单(2026-08-31;行号 = 侦察时快照,实施以字面锚为准)

窗口 bin = `src/rurix-render/src/bin/g31_window_present.rs`(12,732 行);lane_body = `src/rurix-render/src/bin/g14_3_lane/g14_3_lane_body.rs`(17,821 行,L228 `include!` 同编译单元,T1 禁改)。

## 加性臂九步范式(臂⑧ gi2-ris/nee 先例,逐步行号)

1. flag 声明+解析:L7209-7211 声明 / L7594-7614 解析(闭集 off|on,`fail(...)` 越集即拒)。
2. fail-closed 校验:L8603-8631(依赖臂检查:须随 `--gi2`、`--smooth-normals on 且 --textures on`;子参数范围检查)。
3. scene SPV 链换载:L8618-8631,「默认字面才换」阶梯,每臂一级超集工件;常量族 L300-373(如 `G31_DEFAULT_SPV_REALISM_RIS` L368 = `.tmp/night_0830/spv/g31_realism_ris.spv`)。
4. host 侧表一次构建(era 外):L9335-9369(nee on = `g31_ris_lamps::build_lamp_table` 真表;off = 占位)。
5. descs 尾挂绑定 + 资源计数断言 + 屏障计划超集:L3571-3620(`d.resources.push(...)`;`G31_U_*` 下标常量 L533-537 模式;`G31_U_RESOURCE_COUNT_*` 断言;屏障计划 L644/L781 模式)。
6. AE 三件下标族顺延:`_RIS` 族 L1286-1292;guard 最先 match L10017-10035、L10357-10363。
7. 车道创建后一次挂载:`set_gi2_ris(...)` L10446-10450(fn L4357-4361,字段 L4172-4174)。
8. `prepare_update` 逐帧写 params 门槽:L4499-4507(`scene_params[69]=1.0/[70]=ris_m/[71]=1.0`;off 不写 = 参数面 0-byte)。
9. kernel gate 化段消费:`kernels/g31_realism.rx` L1210 起;贡献并入既有合成行 → 帧尾 BGRA8 digest(L11112)自动覆盖。

evidence 登记:`quality_arms` L11744(实参 L11781-11784)、PASS 行 L12704-12708。

## 关键先例

- **跨帧持久小状态(reservoir 最贴模板)= AE state buffer**:常量 `G31_AE_STATE_INIT = &[0u8;16]` L1155-1157;`g31_apply_autoexp` L1578-1664 创建(`data: Some(G31_AE_STATE_INIT)`,device 跨帧持久不清,era 重建重置)。
- **逐像素跨帧状态(parity 双缓冲)= TSR 五对**:lane_body L8827-8831 下标常量 / L9151-9161 分配(`data: None, device_local: true`);ping-pong = `let p = self.parity` L4531 逐帧 binding_overrides,翻转 L5006;历史门 `has_history = !reset && self.has_history_state` L4521 → tsr_params[8](reset 帧 kernel 忽略历史,无需清 buffer)。
- **时域随机**:`params[52]=fi`(L10744-10752 → L4508-4512)驱动 R2 序列 + 黄金比共轭相位(g31_realism.rx L1324-1347),闭式无状态。seed 来自契约 JSON(lane_body L13351-13357),jitter_base = seed % 65521(L9378)。
- **digest**:`g31_bgra_digest` L7035-7041(payload `"G31BGRA-1\0"`+w/h LE+BGRA8);逐帧 digest_seq(仅 --auto-move)L11070-11074;末帧 presented_digest L11108-11112。
- **VUID/帧时**:L10800-10806 `validation_error_count != 0` 即 fail;帧时 `real_render_frame_ms` L11624 与 `--profile-json` render_wall 段 L2018-2036。
- **RIS 选灯现状(增强点)**:kernel 内逐帧逐反弹点 `g31_realism.rx` L1212-1274 均匀选 1 灯 NEE;**臂⑧闭式蓄水池块 L1275-1480**(L1188 自述「闭式蓄水池,无跨帧状态」;WRS 更新 L1456-1462 与 g28 同源比较形)。lamp 直接光消费面在 g18_smooth_nrm/g31_realism 点光循环(`scene.points`,`apply_lamp_lights` 施加点窗口 bin L8850,`params[49]=lamp_contrib` L10338-10342)。
- **lamp 提取**:`extract_lamp_lights` lane_body L2274-2476(emissive 扫描→grid 26 邻域 union-find→峰值通量 top-K);env `RURIX_G31_LAMP_GRID_M`(缺省 0.6,parse 失败即 fail)lane_body L2283-2289;`--lamp-k` 窗口 bin L8003 `unwrap_or(12)`。

## kernel 工件链(新增加性 .rx/SPV 步骤)

无 build.rs;离线编译:`cargo build -p rurixc --features vulkan-backend --bin rurixc` → `rurixc src\rurix-render\kernels\g31_realism.rx --target vulkan -o .tmp\night_0831\spv\g31_realism_restir.spv`(内嵌 spirv-val);既有各级 SPV 为点时工件(源-工件 divergence 如实登记先例 = w2_wiring REPORT L130)。新增步骤:①g31_realism.rx 就地 gate 化追加下一链位(臂⑧先例)②编译新 SPV ③新 `G31_DEFAULT_SPV_*` 常量+换载梯级 ④新 `G31_U_*` 下标+`G31_U_RESOURCE_COUNT_*`+屏障计划两件+descs 尾挂(`g31_lane_descs_tex_nrm` L3583-3589 / `_tex_nrm_bloom` L3839-3845 双变体)⑤AE 三件下标族 +1(guard 最先 match)。`g28_restir.rx` 0-byte。

## 命名规约

- flag:主臂 `--<arm> off|on` 闭集 + 子参数 `--<arm>-<param>`;主臂进 `QUALITY_FULL_EXPANSION` dup 表(L7817-7842)与否决定可否与 `--quality full` 组合(**新臂 off 缺省不进 dup 表 ⇒ full19 字面不含之,锚零漂**;微调子参数不进 dup 表)。既有族:`--lamp-lights/--lamp-gain/--lamp-k/--lamp-contrib`、`--gi2*`。新臂定名 `--lamp-restir off|on` + `--lamp-restir-mcap`。
- env:`RURIX_G31_*` 域;语义律 = 缺席即默认字面(锚零漂),在位 parse 失败即 fail。
