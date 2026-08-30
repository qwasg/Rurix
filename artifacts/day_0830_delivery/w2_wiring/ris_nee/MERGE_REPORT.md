# G37 W2 ris_nee 窗口 bin 合入记录(MERGE_REPORT)

依据 `artifacts/day_0830_delivery/w2_wiring/ris_nee/REPORT.md` §六 16 组「内容锚 +
插入文本」,把 GI2 反弹 RIS 选灯(--gi2-ris/--gi2-ris-m)与 44k 灯片 CDF 面光
NEE(--gi2-nee)两臂接进 `src/rurix-render/src/bin/g31_window_present.rs`。
REPORT 锚点基于 transparency 合入后字面制作,本次合入时 LUT/PSO/VisBuffer 三臂
已再合入过——**全部按内容锚定位**,16 组锚点字面全部命中(无一失锚),偏差见
§三。所有新增处注释「G37 W2 ris_nee」。

纪律执行:零 GPU、零 `--release`、零 target-night;`g14_3_lane_body.rs` /
`g14_3_pipeline_perf.rs` / `kernels/` / 既有 `.spv` / `milestones/` /
`registry/` / `ci/` 全程未触。仅改 `g31_window_present.rs` 一个文件。

## 一、16 组锚点落点(合入后行号)

| 锚 | 内容 | 落点(行) |
|---|---|---|
| ① | `include!("g37_w2/g31_ris_lamps.rs")` | 221–223(g31_lut_assets include 之后,g37_w2 组内相邻) |
| ② | `G31_DEFAULT_SPV_REALISM_RIS` 常量 + `G31_REAL_PARAMS_LEN` 文档注释 [69..72) 三槽登记 | 351–356 / 357–362 |
| ③ | `G31_U_LAMPTBL_TEXNRM=36`/`COUNT_TEXNRM_RIS=37`/`LAMPTBL_TEXNRM_BLOOM=44`/`COUNT_TEXNRM_BLOOM_RIS=45` | 523–529 |
| ④ | `G31_U_PLAN_SCENE_TEXNRM_RIS`(TRANSP 计划逐字 + lamp_tbl 尾项)/ `_BLOOM_RIS` 同形 | 632–654 / 769–791 |
| ⑤ | AE 下标族 `_RIS` 37..=39 / ×bloom 45..=47(guard 最先律注释在内) | 1237–1245 |
| ⑥ | `G31_U_PLAN_AE_{REDUCE,STATE}_TEXNRM_RIS` + `_BLOOM_RIS` 四计划 | 1395–1418 |
| ⑦ | `g31_lane_descs_tex_nrm`:doc +1 行 / 签名 +`lamp_tbl_bytes` / transp 块改判 + lamp_tbl 尾挂 36 / 屏障头分支 `_RIS` 最先 | 3123–3124 / 3131 / 3238–3256 / 3274–3279 |
| ⑧ | `g31_lane_descs_tex_nrm_bloom`:同形三处 + doc,尾挂 44,`_BLOOM_RIS` 计划 | 3381–3382 / 3390 / 3494–3512 / 3530–3535 |
| ⑨ | lane 状态机:字段 `gi2_ris/gi2_ris_m/gi2_nee` / 构造默认 false/6.0/false / `set_gi2_ris` 方法 / prepare_update 扩面门 + params[69..72) 写入 | 3827–3833 / 3902–3904 / 3986–3993 / 4097–4099 + 4131–4139 |
| ⑩ | CLI:声明 / 三 parse 臂 / 校验 + 链换载(默认字面才换,10 字面集含 TRANSP)/ realism_any 并入 | 6770–6776 / 7143–7165 / 8048–8078 / 8080–8090 |
| ⑪ | `QUALITY_FULL_EXPANSION` 20→22(`--gi2-ris`/`--gi2-nee` 进 dup 表,`--gi2-ris-m` 不进)+ 赋值区 `gi2_ris = true; gi2_nee = true;` | 7346–7371 / 7403–7406 |
| ⑫ | era 外字节面:trinm 回退条件并入 ris\|nee / `tri_transp_zero_bytes`(ris\|nee on 而 transp off = tri_count×0.0 零表)/ `lamp_tbl_bytes`(nee 真表 + 装配日志 eprintln / ris 80B 哑表 / off 空)含 points 非空前置 | 8747–8758 / 8759–8766 / 8767–8800 |
| ⑬ | 调用点:nm_ref 回退条件并入 / transp_ref 零表占位分支 + ris_ref / 两处 descs 传参 `ris_ref,` | 9294–9298 / 9299–9313 / 9329、9350 |
| ⑭ | AE 施加 match:`(true,true)/(true,false) if gi2_ris \|\| gi2_nee` 两分支插于 transparency 两分支**之前**(guard 最先) | 9436–9459 |
| ⑮ | set_autoexp 选择块:`_RIS`×bloom / `_RIS` 两分支为新首二分支,原 transparency 首分支降为第三,`let (pi, ti) =` 头移新首 | 9765–9776 |
| ⑯ | 挂载点:`if gi2_ris \|\| gi2_nee { l.set_gi2_ris(gi2_ris, gi2_ris_m_v as f32, gi2_nee); }` 于 set_transparency 之后 | 9855–9860 |

关键语义复核:
- **易漏点已接**:ris|nee on 而 transparency off 时,`transp_ref` 走
  `tri_transp_zero_bytes`(tri_count×0.0 零表)占位保持 kernel 签名序;descs
  函数内 lamp_tbl Some 而 tri_transp None 直接 assert fail-closed(两函数同律)。
- 换载「默认字面才换」:ris|nee on 且 `spv_texture` ∈ 10 个默认字面
  (NRM/GI2/EM/F0/AO/SOFT/REFL/GITEX/NRM/TRANSP)才换 `_ris`,显式 --spv-texture 尊重。
- AE 双接线:锚⑭ `g31_apply_autoexp` 调用点 + 锚⑮ `set_autoexp` 选择块两处
  _RIS guard 均最先,W1 升级后的 assert 连号(37==len/38/39、45/46/47)为保护网。
- params 面:off = 三槽不写 0-byte;[70] 仅 ris on 写(默认 6.0,kernel 钳 [1,16])。

## 二、cargo check(dev 默认 target,`--features vendor-upscale --bin g31_window_present`)

| 轮次 | 覆盖 | 结果 |
|---|---|---|
| 第 1 轮 | 锚①~⑥(include/常量/下标/屏障计划) | rc=0 |
| 第 2 轮 | 锚⑦~⑬(descs 两函数/状态机/CLI/full/装配/调用点——签名与调用点须同轮) | rc=0 |
| 最终轮 | 全部 16 组(+锚⑭~⑯) | **rc=0 全绿** |

三轮 warning 恒 4 条且全为既有(8877/8880 unused doc comment、8897/8898
unused_assignments,HZB/SVT 面;rurix-rt lib 15 条既有)——本次合入零新增
warning,linter 零错误。

## 三、偏差登记(内容锚全部命中;以下为形态级偏差)

1. **锚①后邻漂移**:REPORT 锚 `include!("g37_w2/g31_lut_assets.rs");` 之后现已有
   `g31_pso_warmup.rs` 与 `g31_visbuffer_arm.rs` 两个 include(PSO/VisBuffer 合入
   所致)。仍按 REPORT 字面插在 lut_assets 之后,保持 g37_w2 include 组相邻。
2. **锚④成员数**:REPORT 记 TRANSP scene 计划「全 17 项」,现文件实为 19 项
   (含 `U_SCENE_COLOR`/`U_SCENE_DEPTH`)。按「逐字拷贝 + 尾项」律展开为 19 项
   + lamp_tbl 尾项 = 20 项;屏障计划为集合语义,尾项位序无行为差。×bloom 同形。
3. **rustfmt 折行**:锚②(params 文档注释)、锚⑫(trinm 回退 let 条件超宽折
   为两行 let-if 形态)与 REPORT 模板存在纯排版差,语义逐字。
4. **注释性添加**(REPORT 模板未列,按「修改处注释 G37 W2 ris_nee」纪律补):
   锚⑥计划族 doc 一行、锚⑦c/⑧c 屏障头分支一行、锚⑪ dup 表两行、锚⑫ trinm
   块尾注一行、锚⑬a nm_ref 一行、锚⑮ guard 序两行。
5. **锚⑨d 注释位置**:现文件 `|| self.transparency` 前已有 transparency 自己的
   注释行,ris_nee 注释 + 两行门插于其后(REPORT 模板将注释并在替换体内,语义同)。

其余锚点(③⑤⑩⑬b/c⑭⑯)与 REPORT 插入文本逐字一致。

## 四、GPU 验收步骤(主 agent 执行;REPORT §七逐字,RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1,VUID=0 门;run_arm.py 环形态)

```powershell
# 0) 合入后构建窗口 bin(主agent构建纪律/target 目录);装配日志应见
#    「G37 W2 臂⑧ 灯片表 44024 片(…总功率 …,748412 f32 = 2993648 B)」
# 1) all-off 锚零漂移(off 面不载新 SPV):== 55e4a92d…
g31_window_present.exe --frames 8 --warmup 2 --hidden --evidence ev_alloff.json
# 2) 两臂单开最小组合,各双跑位级(digest run1 == run2 + VUID=0)
g31_window_present.exe --frames 96 --warmup 2 --hidden --smooth-normals on --textures on --gi2 on --gi2-ris on --evidence ev_ris_1.json
g31_window_present.exe --frames 96 --warmup 2 --hidden --smooth-normals on --textures on --gi2 on --gi2-nee on --evidence ev_nee_1.json
# 3) 双臂组合 + full 预设(锚⑪已采纳:--quality full 已含两臂)双跑位级 + 帧时
#    记账(基线十七臂,预算 11.11ms;成本预估:ris M=6 ≈ +6×~80ALU+96 表读/px·
#    反弹,nee 臂 +1 条阴影射线/px·反弹——A1 斜率口径预估 +0.2~0.5ms,实测为准)
g31_window_present.exe --frames 96 --warmup 2 --hidden --quality full --evidence ev_full19_1.json
# 4) 无 AE A/B(红修 #1 定形;EXPLICIT_NOAE 十七臂显式 + 臂旗标;掩码 = GI2
#    间接光面;判读 tools/ab_metrics.py 跳头版;dump 形状 run_arm.py 定形)
#    判据建议:nee 臂 = 反弹通道方差显降(conv 协议 std_p95,C 相口径)+
#    |Δmean| 有界(能量口径换代:代表灯反弹让位真面光,幅度登记);
#    ris 臂 = 同均值方差降(选灯升级不改积分域)。
# 5) 正交抽查:ris on 而 transparency off(零表占位链位)/nee on 而 gi2-tex off
g31_window_present.exe --frames 96 --warmup 2 --hidden --smooth-normals on --textures on --gi2 on --gi2-ris on --gi2-nee on --gi2-ris-m 8 --evidence ev_orth.json
```

### tsrq clamp K 阶梯(REPORT §八逐字;EVAL_DENOISE 第 0 级,零代码 GPU 实验;K 旋钮 = `--tsrq-clamp`〔tsr_params[20],须随 --tsr-quality on——full 已含;子参数不进 dup 表可与 full 直接组合〕)

```powershell
# 全程 GPU 锁内;RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1
# ── 窗口腿(presented 口径;K=0 基线 = --quality full 裸跑,k_on 门位级恒等)──
target-night\release\g31_window_present.exe --frames 96 --warmup 2 --hidden --quality full --evidence ev_k0.json
target-night\release\g31_window_present.exe --frames 96 --warmup 2 --hidden --quality full --tsrq-clamp 3   --evidence ev_k3.json
target-night\release\g31_window_present.exe --frames 96 --warmup 2 --hidden --quality full --tsrq-clamp 2   --evidence ev_k2.json
target-night\release\g31_window_present.exe --frames 96 --warmup 2 --hidden --quality full --tsrq-clamp 1.5 --evidence ev_k15.json
# ── bench conv 腿(EVAL_DENOISE §8 首选判据口径;K=0 基线 = D 相 arm4 在案
#    d_metrics.json 可直接对照,无须重跑)──
target-night\release\g14_3_pipeline_perf.exe --render --scene bistro-interior --tier 100 --backend tsr_device --frames 128 --smooth-normals on --ggx on --lamp-lights on --lamp-gain 4 --gi2 on --gi2-clamp 0.01 --tsr-quality on --tsrq-clamp 3   --out-root artifacts\day_0830_delivery\w2_wiring\ris_nee\kladder\k3
target-night\release\g14_3_pipeline_perf.exe --render --scene bistro-interior --tier 100 --backend tsr_device --frames 128 --smooth-normals on --ggx on --lamp-lights on --lamp-gain 4 --gi2 on --gi2-clamp 0.01 --tsr-quality on --tsrq-clamp 2   --out-root artifacts\day_0830_delivery\w2_wiring\ris_nee\kladder\k2
target-night\release\g14_3_pipeline_perf.exe --render --scene bistro-interior --tier 100 --backend tsr_device --frames 128 --smooth-normals on --ggx on --lamp-lights on --lamp-gain 4 --gi2 on --gi2-clamp 0.01 --tsr-quality on --tsrq-clamp 1.5 --out-root artifacts\day_0830_delivery\w2_wiring\ris_nee\kladder\k15
# 判读:artifacts/day_0828/d_tsr/d_ladder.py 的 ARMS 表指到 kladder/k{X}/
#   bistro-interior/tier100/tsr_device(四 ROI conv std_p95 vs arm4 K=0 在案
#   基线,frame_01*.exr stride2×16 末段口径)。
# 双判据(EVAL_DENOISE §8):①微光点再降(四 ROI std_p95)②远小灯保真
#   (dolly f0240 远灯 ROI 亮度不降超阈——K 过小误杀合法孤立小灯,
#   g31_tsr_resolve_q.rx:28 原注;D 相 dolly 协议同位)。
# 产出 tsrq_clamp_ladder.json:存在正区间 ⇒ 定档入 full 复评;不存在 ⇒
#   登记「K 档关死,降噪投资转第 1 级(tsrq v4 方差引导)」。
```

注:--quality full 现已含 --gi2-ris/--gi2-nee(锚⑪),full 语义变更重锚归 W4
(与 transparency 重锚同窗);K 阶梯步 0 的 full 基线即十九臂形态。

## 五、编辑权

`g31_window_present.rs` 独占编辑权**已释放**(本合入会话终结;文件终态 =
16 组全部合入 + cargo check 全绿)。
