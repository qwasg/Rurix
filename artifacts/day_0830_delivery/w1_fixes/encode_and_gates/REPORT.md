# G37 W1 子任务报告：SPV divergence #1 收编 + encode parity 转正门 + 纹理判读器 heap 同步

> 日期 2026-08-29。依据 `artifacts/day_0828/e_final/DEFAULT_FLIP_PLAN.md` §1.2 与
> `artifacts/day_0828/e_final/HANDOVER.md` §B.4/§B.5。纪律执行：零 GPU 运行（全部 smoke 只跑
> `--selftest` 纯 CPU 腿）、零 cargo 构建、src/*.rs 与 kernels/*.rx 与 milestones/registry 既有文件
> 0-byte（milestones/g31/ 仅新增 3 个 schema 文件）；ci/ 修改走专项授权。

## A. encode 共享路径切 v2 字节（divergence #1 收编）

### A.1 侦察与勘误

共享路径常量（源码只读侦察）：

- `src/rurix-render/src/bin/g34_full_lane.rs` L107：`G34_DEFAULT_SPV_ENCODE = ".tmp/g14_gates/m_c/g31_display_encode.spv"`（→ L1317 `spv_encode` 默认，default/hzb/skin 三车道 encode pass 绑定）。
- `src/rurix-render/src/bin/g35_particle_lane.rs` L128：`G35L_DEFAULT_SPV_ENCODE` 同字面（→ L2429 encode 绑定）。
- `src/rurix-render/src/bin/g31_window_present.rs` L248：`G31_DEFAULT_SPV_ENCODE` **已指 v2**（A2b 在案，非本次动作）。

**尺寸勘误（如实登记）**：任务书/DEFAULT_FLIP_PLAN §1.2 记共享旧字节 = 95,088B，实测共享路径当前
= **95,660B**。95,088B 是 **pre-A2 旧件**（备份件 `artifacts/day_0828/a2_autoexp/g31_display_encode_pre_a2.spv.bak`，
sha256 `ba638a31…` 实证在案）；A2 相（autoexp aeg 增益槽）曾重编共享件 → 95,660B。A2b 自己的验收 JSON
（`a2b_aces_fix/ACCEPTANCE_SUMMARY.json` spv_governance.audit）已如实记录：`"shared_spv": "….spv（sha256:43b0c255…,A2 期重编件,0-byte 不动）"`。
即：**待替换旧字节 = 43b0c255（A2 重编、含 aeg、仍带 ACES 转置 bug）**，flip plan 表格的 95,088B 为过时行文。
v2 与旧件同为 95,660B（修复仅改样条 b1/b2 系数算式，指令量不变）。

### A.2 执行记录（前后 sha256）

| 步骤 | 文件 | sha256 | 大小 |
|---|---|---|---|
| 前置核验 v2 | `.tmp/night_0828/spv/g31_display_encode_v2.spv` | `e7291c7936a08f185614060c48077104e2929280cba9613653c29fd7eca94b2d` | 95,660B ✓ |
| 替换前共享路径 | `.tmp/g14_gates/m_c/g31_display_encode.spv` | `43b0c2557ca27ba222f8608e403f5998628b5ad23a0692582b57ea0f35220109` | 95,660B |
| 备份 | 同目录 `g31_display_encode.spv.pre_g37.bak` | `43b0c255…`（== 替换前，逐字节核验） | 95,660B |
| 替换后共享路径 | `.tmp/g14_gates/m_c/g31_display_encode.spv` | `e7291c79…`（== v2，逐字节核验） | 95,660B |
| spirv-val | 替换后共享路径 | rc=0（vulkan-1.3.296.0 工具链） | — |

回滚 = `Copy-Item .tmp/g14_gates/m_c/g31_display_encode.spv.pre_g37.bak .tmp/g14_gates/m_c/g31_display_encode.spv -Force`（v2 隔离件不删）。
`.tmp/` 不入 git，本步无 git 可见变更。

### A.3 受影响消费者清单

**运行时消费共享字节（presented 面必漂——计划内重锚，W4 统一重收割，本任务只登记不收割）**：

1. `g34_full_lane`（default / `--hzb`（g34_2_hzb.rs）/ skin 三车道 encode pass；`ci/g34_unified_lane_smoke.py`〔g34.wave1.unified〕、`ci/g34_hzb_unified_smoke.py`〔g34.wave2.hzb〕、`ci/g34_skin_unified_smoke.py`〔g34.wave2.skin〕消费）。
2. `g35_particle_lane`（`ci/g36_geo_composition_smoke.py`〔g36.wave1.geo_composition〕经 m_c SPV 前置面消费；`ci/g35_render_wiring_smoke.py`/`ci/g35_sort_oit_smoke.py` **不消费共享件**——两门 rurixc 现编 encode 自源码并显式 `--spv-encode` WORK 件，重编字节已含 A2b 修复，与本切换同向）。

**重大侦察发现（RD-045 消费面已消亡）**：`target/release/g31_window_present.exe`（RD-045 P02 腿宿主）
mtime = 2026-08-28 21:43，**已非"旧二进制"**——Phase F 的 CARGO_TARGET_DIR 会话丢失事故（HANDOVER §27）
把新构建静默落进了 `target/`。二进制字符串扫描证明：其内嵌 encode 路径 = `.tmp/night_0828/spv/g31_display_encode_v2.spv`
（offset 1,818,759 命中），旧共享路径字面 **不存在**（target-night 同）。即：

- 「RD-045 旧二进制运行时读共享旧字节」这一消费面**在本次替换前已不存在**；
- RD-045 锚 `060e69a8` **本就已漂**（flip plan §1.1 预言的"旧二进制被重建 → 060e69a8 必漂"已于 Phase F 兑现），与本次替换无关但同须 W4 重收割 + 改写 `ci/g31_blocked_probes_smoke.py` L63 字面（ci/ 改动虽有授权，但锚**重收割需 GPU 跑**，归 W4 统一批）。

**存在性前置消费（非字节断言，零影响）**：`ci/g31_blocked_probes_smoke.py`、`ci/g31_game_loop_smoke.py`
（`ensure_encode_spv` 缺件才现编——文件在位不触发）、`ci/g31_texture_sampling_smoke.py`、`ci/g34_unified_lane_smoke.py`、
`ci/g36_geo_composition_smoke.py` 等 LANE_SPVS 检查。

## B. encode_parity_probe 转正 CI 门

### B.1 交付件

- **门脚本** `ci/g31_encode_parity_smoke.py`（GATE_KEY `g31.g37w1.encode_parity`；风格同 blocked_probes/texture_sampling：三态语义 + gpu_device_lock + evidence PASS-only 落盘 + FAIL 诊断件留 .tmp + `--selftest` 纯 CPU）。
- **evidence schema** `milestones/g31/g31_encode_parity_evidence_schema.json`（新增文件；schema id `rurix.g31.encode_parity_smoke_evidence.v1`；阈值 const 钉死）。
- **注册 patch** `ci/_patch_g31_encode_parity_schemas.py`（check_schemas.py 三处纯追加：load/validator/`g31_encode_parity_` 前缀路由；幂等 + py_compile；**已运行**，`py -3 ci/check_schemas.py` 全量 PASS = 既有路由零破坏）。

### B.2 探针依赖分析（任务①）

输入 = **需 GPU 现跑产出**的同帧切层对：`RURIX_G31_DUMP_F32=1` 落末帧 TSR 输出 f32（固定落点
`.tmp/g31_gates/hzb/last_f32.bin`，源码字面；fs::write 不建父目录且错误被吞 ⇒ 门脚本预建目录 + 清陈旧件 + 跑后验在位）
+ `--dump-present-raw` 同帧 presented 字节（w,h u32 LE 头）。host 侧纯 CPU：aces13 f64 金标准逐字向量化，
数学单源 import 自 `artifacts/day_0828/recon/bluefan_encode_sim.py`（A2b 交叉验证件；**刻意不在 ci/ 复制第二份
常量表**，防双源漂移）。**提交前置登记：`artifacts/day_0828/recon/` 现为 untracked，须随战役工件入 git，否则门在
clean checkout 上 selftest 红**（诚实红，不静默）。

### B.3 GPU 腿设计（写好未执行,纪律面）

臂 = **显式 `--quality off`**（parity 口径前提 dither off/autoexp off/aeg=1.0；显式旗标 = 对 W1 后默认翻转免疫）
静态契约相机 `--frames 8 --warmup 2 --hidden`，env `RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1`（A3 律）。
硬门阈值（A2b 实测锚 exact 99.9891%/p100=1/>1LSB=0，留裕不放水）：`pixels_gt_1lsb == 0 ∧ p100 ≤ 1 ∧ exact ≥ 99.9%`
∧ 0.18 灰设计点双口径 == 99³。

### B.4 selftest 结果

`py -3 ci/g31_encode_parity_smoke.py --selftest` → **PASS（rc=0，20 臂）**。防复发核心臂：
`kernel_sim(transposed=True)`（A2b 根因形态逐字复刻）在 0.18 灰设计点产 **47³ ≠ 99³ 必被判红**；
fan 地标向量化预测 == A2b 在案 [144,122,77]；parity 度量器红绿臂（位级同帧/1 LSB 带内绿/2 LSB 必红）；
raw 双通道序解析；schema 阈值 const 与脚本常量互核。

### B.5 注册方式说明（编号纪律）

G31+ 门族先例（`milestones/g31/CI_GATES.md` 全表"未占号"）：**不占 pr-smoke.yml 数字步骤号**。
台账核实 `registry/number_ledger.json` CI_step `next_free=525` / `on_tree_max=524`（pr-smoke 零 g31 条目）——
**本任务零编号消费，维持 next_free=525**。已落地注册 = schema 文件 + check_schemas 三处纯追加（已跑已验）。
**待主 agent 的登记项**（milestones 既有文件禁改，本任务不动）：`milestones/g31/CI_GATES.md` 追加一行
`| 未占号 | g31.g37w1.encode_parity | ci/g31_encode_parity_smoke.py |`（如主线认为该门应升级为占号硬门，
则按 number_ledger CI_step 纪律顺位领 525 并 materialize 进 pr-smoke.yml + 校准行——本任务不擅自消费）。

## C. 纹理判读器同步 heap 形态（divergence #2 收编）

### C.1 修改摘要（`ci/g31_texture_sampling_smoke.py` 全量重写,8 facts 闭集不变）

| 面 | 旧（B4 网格图集形态） | 新（heap 形态,day_0828 Phase B 后事实） |
|---|---|---|
| 映射档 | N_MAPPED=12（top-12） | **N_MAPPED=70**（全覆盖;lane_body `G31_TEX_N_MAPPED_HEAP` 同源） |
| 探针律法 | 步幅 3 `(slot,u,v)`,288 探针 const | **步幅 4 `(slot,u,v,lod)`**：`probe_lods = dedup[0,mips/2,mips−1]`（镜像 `g31_tex_probes_mip` 含 Vec::dedup 连续去重语义）,计数律法独立重算 `Σ槽 24×级数`（bistro = **5040**）与 harness `probe_count` 互核 |
| 槽判据 | width/height ≤2048 pow2 + origin 瓦位律法（slot×2048 网格） | **src_w/h pow2 ≤2048 + width==min(src,1024)（cap 律法）+ mip_count==log2(max)+1（truncated=false 档）+ mip_digests 长度/形态 + origin 废除**；manifest 互核 70/70 |
| 图集判据 | atlas 8192×6144/tile 2048 const | **texel heap 恒等式**：form==texel_heap ∧ cap==1024 ∧ mip_slots==13 ∧ header_entries==slots×13(=910) ∧ heap_bytes==heap_texels×4 |
| harness SPV | rurixc 现编件直接喂 harness | **v2 隔离件** `.tmp/night_0828/spv/g31_texture_{gi,probe}_v2.spv`（战役锚承载字节,B 相验收同款）；源码有效性另走现编 `_srccheck_*.spv` + spirv-val（两面分离,现编不覆盖锚承载件）；v2 缺件 = DEV_ENV_DEGRADE |
| G11.3 复跑 | 12 纹理复解码 | **70 纹理复解码** |
| 作废锚 | （无字面,判据历史即双跑） | 占位常量 `TEX_ARM_PRESENTED_ANCHOR = "PENDING_W4_REHARVEST"` + 注释登记 6fab598c 作废谱系；进 gate evidence `demo.presented_anchor`,**schema const 钉死防误消费** |
| evidence 路由 | `g31_texture_sampling_{harness_,gate_}` 前缀 → 旧 schema | **加性双形态**：新前缀 `g31_texture_sampling_heap_{,gate_}` → 新 schema 双件;旧 schema/旧 evidence 0-byte 不动;off 腿沿用 `g31_game_loop_tex_` 前缀（顶层形态无漂移,dolly_off 实证对 game_loop schema 仅 textures 块差） |

**6fab598c 字面清理核实**：全仓检索（含 ignored）证实该锚字面只存于历史工件/文档
（CAMPAIGN_LOG、b_textures/ACCEPTANCE_SUMMARY、e_final 三文档、night_0828 两 evidence），
**不在任何活门/代码中**——历史记录不改写；"字面清理"以判读器占位常量 + schema const 落地（上表）。

**新增 schema 双件**（milestones/g31/ 新文件）：`g31_texture_sampling_heap_evidence_schema.json`
（harness 侧 schema/gate 字面沿用 `rurix.g31.texture_sampling_evidence.v1`/`g31.waveB.texture`——src 禁改面的
harness 常量如此；heap 形态经文件名前缀路由承载；quality_arms 按当前源 12 键含 gi2 三键；对 B 相真实件
tex_1.json〔补 gi2 键〕验证 textures 块零错）+ `g31_texture_sampling_heap_gate_evidence_schema.json`
（gate id `rurix.g31.texture_sampling_heap_gate_evidence.v1`,新增 heap 块与 probe_count 律法位）。
**注册 patch** `ci/_patch_g31_texture_heap_schemas.py`（**已运行**;路由序律机核：heap 前缀含旧前缀 ⇒
heap_gate → heap → 旧 gate → 旧 plain 严格递增,check_schemas 全量 PASS）。

### C.2 selftest 结果

`py -3 ci/g31_texture_sampling_smoke.py --selftest` → **PASS（rc=0，77 臂）**：heap 律法红绿臂
（cap/mip 链/digest 长度/truncated/旧形态槽必红）、heap 恒等式红绿臂、探针步幅 4 律法（lods 五档 + bistro
5040 计数 + 步幅 3 旧计数 1680 必红 + lod 注入/跨级 UV 同源/无重复）、既有 SSBO/sampler/序列/frame_ms 臂全保、
双 schema 互核 + 作废锚占位 const 钉死。

## D. 需主 agent GPU 复跑清单（准确命令；全部在 gpu_device_lock 内由门脚本自持）

> 环境律：`RURIX_REQUIRE_REAL=1` 必配 `RURIX_VK_VALIDATION=1`（门脚本内已自置）。
> 前置：`artifacts/day_0828/recon/` 入 git（B 门单源依赖）。

**W1 即时可跑（验证本任务交付）**：

1. `py -3 ci/g31_encode_parity_smoke.py --gate g31.g37w1.encode_parity`
   （encode parity 新门首跑收编验证；替换后共享路径与 v2 同字节，window_present 走 v2 常量——预期 PASS,
   exact ≈99.99%/p100≤1/gt1=0）
2. `py -3 ci/g31_texture_sampling_smoke.py --gate g31.waveB.texture`
   （heap 判读器首跑；门内自带 cargo build + rurixc 现编 + spirv-val + off×2/on×2 + Stage A 160f bench；
   预期全 8 facts PASS——Stage A 锚 c1d28ad7 在 encode 上游不受 A 影响）

**A 替换的消费面复跑（presented 基线漂移 = 计划内,验证「新字节下门语义仍绿」;W4 统一重收割锚字面）**：

3. `py -3 ci/g34_unified_lane_smoke.py --gate g34.wave1.unified`
4. `py -3 ci/g34_hzb_unified_smoke.py --gate g34.wave2.hzb`
5. `py -3 ci/g34_skin_unified_smoke.py --gate g34.wave2.skin`
6. `py -3 ci/g36_geo_composition_smoke.py --gate g36.wave1.geo_composition`
7. （自编面同向验证,可选）`py -3 ci/g35_render_wiring_smoke.py --gate g35.wave3.render` 与
   `py -3 ci/g35_sort_oit_smoke.py --gate g35.wave4.sort_oit`

**W4 重收割批（本任务登记不执行）**：

8. RD-045 锚重收割：`target/release/g31_window_present.exe --frames 64 --warmup 10 --hidden --auto-move orbit`
   双跑位级一致后改写 `ci/g31_blocked_probes_smoke.py` L63 `RD045_ANCHOR_DIGEST` 字面（现值 060e69a8 已因
   Phase F 二进制覆盖失效——见 §A.3）；然后 `py -3 ci/g31_blocked_probes_smoke.py --gate g31.waveC.blockedprobes`。
9. heap tex 臂 presented 锚重收割 → 回填 `TEX_ARM_PRESENTED_ANCHOR` 占位（判读器 + heap gate schema
   `demo.presented_anchor` const 同步改）。
10. g34/g35 presented 锚整批重收割（HANDOVER §D.18 二进制绑定锚律：重建/换字节后先复验再消费）。

## E. 变更清单（git 面）

- 修改：`ci/g31_texture_sampling_smoke.py`（heap 判读器全量重写）、`ci/check_schemas.py`（两 patch 六处纯追加）。
- 新增：`ci/g31_encode_parity_smoke.py`、`ci/_patch_g31_encode_parity_schemas.py`、`ci/_patch_g31_texture_heap_schemas.py`、
  `milestones/g31/g31_encode_parity_evidence_schema.json`、`milestones/g31/g31_texture_sampling_heap_evidence_schema.json`、
  `milestones/g31/g31_texture_sampling_heap_gate_evidence_schema.json`、本报告。
- 文件系统（.tmp 不入 git）：共享 encode SPV 切 v2 + `.pre_g37.bak` 备份（§A.2）。
- 验证：两 selftest PASS + `py -3 ci/check_schemas.py` 全量 PASS + 新增/修改 py 零 lint。
- 未动（纪律面）：src/ 与 kernels/ 全部、milestones/registry 既有文件（含 CI_GATES.md——追加行文本见 §B.5）、
  pr-smoke.yml、number_ledger（零编号消费）。
