# D6 GGX 高光 + 窗口车道合流 — 补充独立评审报告

评审基面：工作树未提交改动中属于本次范围的 4 个文件（`g18_smooth_nrm.rx` 新增未跟踪 + 3 个 .rs 修改），全部只读，未改任何文件。前次评审（D1/D2/D3/D5，结论"可安全合入"，`artifacts/night_0828/review_report.md`）之外仅审 D6 增量。

---

## 1. 位级确定性 — **PASS**（静态机制 + 机器证明双承载）

**GGX 关臂（params[48]=0）逐位 == 加 GGX 前的 kernel**：

- `tri_mr` 唯一读取被 `if params[48] > 0.5` 均匀分支包裹（`g18_smooth_nrm.rx` L187-191），关臂零触达 ⇒ 8B 哑表安全；
- 关臂时无条件新增的逐像素常量面（L192-198 `rough_c/alpha/alpha2/f0_*/cos_v`）为纯局部计算，无输出写；
- `spec_*` 累加器初值 `+0.0`（L206-208），仅门内累加（L301/L395）⇒ 关臂恒 `+0.0`；
- 输出尾加 `+ spec_r`（L451-453）：左结合 `((emission + al·inv_pi·dir) + al·amb) + spec`，被加数三项均非负（emission≥0、albedo≥0、dir 经 keep 门控 ≥+0、amb = params[45]·hemi·amb_i 且 hemi = 0.65+0.35·ny ∈ [0.3,1.0] 恒正）⇒ IEEE `x + (+0.0) = x` 在 x ≥ +0 域严格恒等（唯一反例 x=−0.0 在此域不可达）。既有乘加结合序未动。

**机器证据**（与 D2/G36 在案锚交叉互证，非 D6 自洽）：
- snrm 臂 GGX-off 双跑 == `6b46f70a`（= D2 证据 `d2_smooth_nrm/d2_evidence.json` L24-25 的 post_on 锚）+ 128f 收敛 == `778f1dfc`（D2 渲染锚）— `d6_ggx/verify_summary.json` L46/L61、`d6_verdict.json` L8-9；
- 默认臂双跑 == `f39e9808`（= G36 门在案锚，前次评审已交叉确认）— `verify_summary.json` L16/L31 vs `baseline_pre.json` L14/L29；
- 窗口车道 off == `b02b08b57`（= D2 窗口锚 `d2_window/d2w_summary.json` L25/L42）— `d6_ggx/d6w2_summary.json` L8/L78。

**窗口车道 off 臂逐位 == b02b08b57 锚**：nrm 描述组第 9 绑定（25/33）仅多绑一块 8B 零哑表（`g31_window_present.rs` L1557-1562/L1633-1638），kernel 关臂不读；`G31TsrLane.ggx=false` ⇒ `pack_frame_params_ggx(.., false)` 产物 params[48]=0（L2061-2072）。机器证据同上 d6w2。

**PARAMS_LEN 48→56 零漂移**：
- 读取面不变：全族 kernel 逐一 grep——`g14_3_direct_gi.rx` 最高读 params[41]（L68）、`g16_gi_multibounce.rx` 最高 [41]（L67）、`g18_light_transport_depth.rx` 最高 [42]（L311）；追加的 [48..56) 恒 0 无任何既有 kernel 读取；
- 长度自洽：buffer 创建（`params0_bytes = PARAMS_LEN*4`，`g14_3_lane_body.rs` L10686）与逐帧上传（`bytes_f32(&scene_params)` 56 f32，L7950）同常量派生；vendor 双臂（DLSS L9125-9139 / FSR L9893+）经 `pack_frame_params` 委托链同产 56 f32，与其 LaneAssets 同源；
- **无门钉 params 字节 digest**：`selftest_leg` 只哈希内置 tiny JSON + 契约 digest（L10331-10353）；`capture_params_digest`（L10113）名有误导实为图像内容 digest。receipt 中 "192B" 字样为未改动的 provenance 字符串（L12231 等）⇒ 回执文本零漂移；
- 机器证据：默认臂 == f39e9808（上）+ 窗口默认车道 == `5596a730`（`d6w_summary.json` L12）+ 04:28 Stage A 18/18 零漂移（NIGHT_LOG L106，时点在 D6 双车道改动之后）。

## 2. BRDF 正确性 — **PASS**（一条已登记 CONCERN）

逐项核（quad 臂 L301-334 / point 臂 L395-422 同式）：

- **几何项符号**：wo = −主射线 dir；`hv = wi − d` = wi+wo ✓（L302-304）；`cos_v = −(n·d).max(0)`，双面翻转后 n 朝相机 ⇒ ∈[0,1] ✓（L198）；`cos_wh = −(d·h)` = wo·h ✓（L325）；
- **D** = Trowbridge-Reitz：`dd = cos_h²·(α²−1)+1`，`d = α²·inv_pi/(dd²).max(1e-6)`（L315-316）——α=rough²、rough 钳 [0.05,1]（L192）⇒ dd ≥ α² ≥ 6.25e-6 > 0，分母恒正无奇异 ✓（L313-314 注释中间不等式 "dd ≥ 1−α² ≥ 0.0025" 书写有误，但结论"分母恒正"正确，且有 .max(tiny) 兜底——cosmetic）；
- **G** = Smith Schlick-GGX G1 乘积，k = α/2 ∈ [0.00125, 0.5] ⇒ 分母 ≥ k > 0；cos_s=0/cos_v=0 ⇒ G1=0 ⇒ spec=0 ✓（L319-322）；
- **F** = Schlick：`f0 + (1−f0)·(1−cos_wh)⁵`，五次幂 = om²·om²·om 左结合 ✓（L325-328, 331-333）；
- **分母** `4·cos_s·cos_v + 1e-3 ≥ 1e-3 > 0` ✓（L329）；估计量代数自洽（g 含 cos_s，与 fr·cos_s 形式一致）；
- **hl=0 角隅**（wi==d ⇒ cos_s≤0 ⇒ keep=0）：gate_hl 门恒 0 有限值，贡献 ±0 无 NaN ✓（L299-300 注释与代码互核）；
- **哑表全 0 假设性误读**（不变式破坏的纵深防御）：metal=0/rough=0 → 钳 0.05 → 全有限，零 NaN ✓；
- **算子域外值**：metallicFactor 越 [0,1] 的违规范资产可产负 F0/负 spec（非 NaN，下游钳吸收）——与 D2 畸形 NORMAL 同类，opt-in 臂低概率，登记。

**CONCERN（已登记，d6_ggx_report.md known-gap #2）**：F0 = mix(0.04, albedo, metal) 中的 albedo 是 mats 面**已乘 (1−metallic) 的漫反射口径**（装配 L1752-1765）⇒ 金属 F0 被低估 (1−m) 倍；metal=1 极端资产 F0→0，仅剩 Schlick 掠射沿（F=(1−cos)⁵），无 NaN/黑洞但高光近失。bistro 全 70 材质 metal=0.4 下视觉验收成立（地板釉面/柜台高光，20.2% 像素结构化变化）。纯画质语义限制，opt-in 臂，不威胁默认面。

## 3. 治理合规 — **PASS**

- **冻结面 0-byte**：`g14_3_direct_gi.rx` / `g18_light_transport_depth.rx` / `g16_gi_multibounce.rx` / Stage A 锚面 — `git status` 全绿未触 ✓；
- **他人未提交面**：`g34_full_lane.rs` / `g35_particle_lane.rs` / `check_schemas.py` / `milestones/g36/` — 未触 ✓（`milestones/g35/G35_CONTRACT.md` 的修改为并行 G35/G36 会话遗产，NIGHT_LOG L6 与前次评审 §2 已登记，非本改动集；合入时按文件名择取分拆的既有纪律沿用）；
- 同树其余改动（`g31_display_encode.rx`/`aces13.rs`/`g31_bloom_*.rx`）= 前次已 PASS 的 D1/D3 面；
- **SPV 新鲜度**：kernel 源 03:28:10 → SPV 03:28:18 编译 → host 双 bin 03:31-03:54，均先于各自验收跑（03:40 bench / 04:05 窗口）⇒ 在档证据与当前工作树同源 ✓（手工 rurixc 编译流程为前次评审已登记的流程性留窗）。

## 4. 风险登记（按严重度）

1. **CONCERN｜F0 用漫反射 albedo**（见 §2）：metal→1 资产高光近失；已如实登记，bistro 资产面不触达；真解 = mats 面增存未调制 baseColor 或 tri_mr 扩 5 f32/tri——归后续内容管线窗。
2. **CONCERN（文档债）｜"192B"/"48 f32" 陈旧注释**：`g14_3_lane_body.rs` L6343、`g14_3_pipeline_perf.rs` L16 及多处 provenance 字符串（L6896/12231/14557 等）仍写 192B/48 f32（实际 224B/56）。字符串未改 ⇒ 回执零漂移（若被锚钉反而不能轻动）；建议登记后续窗统一修订，不在本次合入混入。
3. **登记｜`--ggx on` + 显式 `--spv-scene` 覆盖面无 fail-closed**：若用户给 8 路旧 SPV，9 绑定 vs 8 签名 mismatch 未校验——前次评审 CONCERN #2 同型延伸，低危，CLI 已引导正确用法。
4. **登记｜quad 面光 GGX 路径未实证**：bistro quads=0，cornell Split 与 smooth-normals fail-closed 互斥；代码与 point 臂同式对称（d6 报告 known-gap #3）。
5. **登记｜`d6_ggx_report.md` 留窗 #1 陈旧**："窗口车道无 --ggx 旗标" 已被 04:05 d6w2 接线超越（报告写于 03:40 中途态）；NIGHT_LOG 终态为准。
6. **登记｜SPV 手工编译流程**：.rx 改后未重编译则车道静默用旧 SPV——D1/D2/D3 同有的流程性面，非 D6 新增。

## 总判：**可安全合入** ✅

四项评审要点全 PASS：关臂/默认臂/窗口 off 臂位级恒等有**静态机制证明 + 与 D2/G36 在案锚交叉互证的机器证据**（f39e9808 / 6b46f70a / 778f1dfc / 5596a730 / b02b08b57 / 12d5dc91 全零漂移，on 臂双跑位级 46e0af63 / 52020f9c / 组合 48353e86，validation 全静默，风暴+soak 收口）；BRDF 数值面无奇异/无 NaN 通道；PARAMS_LEN 扩面对既有面真零影响；冻结面与他人文件零触碰。唯一实质性 CONCERN（F0 口径）为 opt-in on 臂的画质语义限制且已如实登记，不构成合入阻断。

Now write the full review report as the final response.