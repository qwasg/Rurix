# 夜间画质优化改动 — 只读独立评审报告

评审基面：工作树未提交改动（5 改 + 4 新，见 `git status`），全部只读，未改任何文件。

---

## 1. 位级确定性（默认 off 臂是否真位级恒等）

### 1.1 `g31_display_encode.rx` dither 关臂 — **PASS**

diff 确认改动纯加性（旧行 `(fr.powf(0.41666666)*255.0+0.5).floor() as u32` → 新行加 `+ dn` 与 clamp）：

```451:467:src/rurix-render/kernels/g31_display_encode.rx
        let mut dn = 0.0;
        if params[3] > 0.5 {
            // ... IGN ...
            dn = r1 - r2;
        }
        let qr8 = ((fr.powf(0.41666666) * 255.0 + 0.5 + dn).floor()).max(0.0).min(255.0) as u32;
```

- **`dn=0` 时 `x+0.0` 恒等**：左结合为 `((fr.powf*255.0)+0.5)+0.0`。`fr` 经色域钳 ∈ [0,1]（L435–443 `.max(0.0).min(1.0)`）⇒ `powf ≥ +0.0` ⇒ `x ≥ 0.5 > 0`。IEEE 下正非零加 `+0.0` 严格恒等（唯一的 `-0.0` 反例在此域不可达）。✓
- **量化后 clamp 恒等**：`fr∈[0,1]` ⇒ `v*255+0.5 ∈ [0.5, 255.5]` ⇒ `floor ∈ {0..255}` ⇒ `.max(0.0).min(255.0)` 逐位恒等。✓（on 臂 floor ∈ [−1,256] 由 clamp 正确收口 ✓）
- **on 臂确定性**：IGN 纯像素坐标函数，零帧序零状态 ⇒ 双跑位级 ✓。
- 机器证据：NIGHT_LOG 01:27 行「off 臂 presented digest == 改前旧二进制（5596a730）零回归 + on≠off」。

### 1.2 `aces13.rs` 原函数 0-byte — **PASS**

diff 确认 `aces13_device_encode_params` 逐字未动，新增 `_ex` 仅覆写 `v[3]`：

```492:501:src/rurix-render/src/display/aces13.rs
pub fn aces13_device_encode_params_ex(...) -> Vec<f32> {
    let mut v = aces13_device_encode_params(width, height, bgra);
    v[3] = if dither_enable { 1.0 } else { 0.0 };
    v
}
```

`dither_enable=false` ⇒ `[3]=0.0` = 旧 reserved 值 ⇒ 参数 SSBO 与历史逐位同。✓

### 1.3 `g18_smooth_nrm.rx` vs 母版（params[43]=0 / params[44]=0）— **PASS**（含 ±0 角隅静态证明）

- **gate_sn 门**：params[43]=0 ⇒ `((0−0.5)·1e30).min(1).max(0) = +0.0`（L145）。选择式 `hgx = 0.0·hgx_s + 1.0·hgx_f`：`1.0·hgx_f` 恒等；`0.0·hgx_s` 在哑表全 0 下 = `+0.0`（有限，无 NaN——见 §3.4）。唯一理论位差：`hgx_f = −0.0`（零面积退化三角的带号零叉积）时，`+0.0 + (−0.0) = +0.0 ≠` 母版 `−0.0`。逐链追踪：该 ±0 差经 `flip=1.0`（±0 经 min/max 门同归 0）→ `cos_s` 求和（非零项精确吸收 ±0；全零项时 `.max(0.0)` 同归数值 0 ⇒ `gate_cs=0` ⇒ 贡献恒 0）⇒ **输出位级不变**。kernel 头 L16–18 的「±0 号位角隅经下游 max(0) 门吸收」声明成立。
- **环境光关臂**（L334–336）：`amb_i=0` ⇒ `amb_* = 0·hemi·0 = +0.0` ⇒ `al·(+0.0) = +0.0`（albedo 非负）；部分和 `emission + al·inv_pi·dir` 非负（直接光累加 ±0 代数已核：`acc + (−0.0)` 不翻转 `+0.0`）⇒ `x + 0.0` 恒等。即使病态 `−0.0` 角隅也被外层 `+ sky_amb·0.55` 冲刷归一。✓
- **生产面事实上不触达 fork 关臂**：MegaSmoothNrm 车道恒 `params[43]=1.0`（`pack_frame_params_nrm` L6349–6350），默认臂跑的是**未改的母版 SPV**——锚由母版 0-byte 承载，fork 关臂位级性仅为纵深防御。
- 机器证据：`d2_smooth_nrm/verify_summary.json` — post_off×2 digest == 改前基线 `sha256:f39e9808…`（**与 G36 门在案锚同一值**，交叉互证）+ on 双跑位级 + on≠off，`all_pass: true`。

---

## 2. 治理合规 — **PASS**（一条工作树卫生 CONCERN）

- **冻结面 0-byte**：`g14_3_direct_gi.rx` / `g16_gi_multibounce.rx` / `g18_light_transport_depth.rx` / g13_4 对拍锚面 — `git status` 全绿未触。✓
- **他人未提交面**：`g34_full_lane.rs` / `g35_particle_lane.rs` / `check_schemas.py` / `milestones/g36/` — 未触。✓
- **CONCERN（非本改动集所为，但同树在飞）**：`milestones/g35/G35_CONTRACT.md`（+30 行 §8.1 收口批）、`00_MASTER_INDEX.md`（+2）、`11_ROADMAP.md`（+2）为**并行会话 G35 收口工作流**的未提交改动，不在本次 7 项清单内。HEAD commit（bece24e7）明确记载这些面"按文件名显式择取先例留工作树"。**合入时必须按文件名择取分拆，不得混入画质改动集**（仓库既有先例同律）。
- 新 SPV 默认路径指向 `.tmp/night_0828/spv/`（git-ignore 构建产物；d2_evidence 登记 `rurixc --target vulkan + spirv-val accepted`）。鲜检出 + opt-in 臂 = 文件缺失 fail-closed（可接受），但建议把编译配方登记进既有 m_c SPV 再生流程同档（流程性留窗，非阻断）。

---

## 3. 正确性 bug 排查 — **PASS**（无 OOB / 无 NaN 通道 / 无下标撞面）

- **3.1 bloom 半分辨率奇偶**：`g31_bloom_bright.rx` L34–45 `out_w=(in_w+1)/2` + 源坐标四项 clamp（奇数宽末列复制采样）⇒ 无越界；`in_w=0` 时 `px<0` 门卫不入循环。host 侧 `ew.div_ceil(2)` 同式（g31_window_present.rs L1349–1350 缓冲区 `ceil·ceil·12` 互核）。✓
- **3.2 composite 双线性边界 clamp**：`g31_bloom_composite.rx` L43–54 双重 clamp 正确处理 `bw=1`（`bx0→bw−2=−1→回钳 0`，`bx1→0`）⇒ 无 OOB。✓ 边角注记：`px=0` 时 `fx=−0.25`，钳位后 0.75 权重偏向内部邻 texel（非纯 edge-replicate）—— cosmetic 级，确定性不受影响。blur 9-tap clamp 取样 ✓，权和 = 0.999998（f32 字面，确定性微损非 bug）。
- **3.3 资源下标**：g14_3 车道 Mega 占 0..=21（`U_RESOURCE_COUNT=22`），`U_TRINRM=22` 空位；Split 22..=24 / G34Full 22..=26 形态互斥同律。窗口车道基面 24（encode 22/23），bloom 24..=31、trinrm 单臂 24 / 组合 32 —— 与 textures(24..=28)/svt 经 CLI fail-closed 互斥（diff L4754–4761 等）⇒ 零撞面。✓ 绑定序与 kernel 签名逐字同序（tris/mats/quads/points/params/trinrm/out×2）。✓
- **3.4 哑表全 0 ⇒ gate_sl 真零 NaN**：`snx=0 ⇒ sl=0 ⇒ gate_sl=0 ⇒ sl_safe=1 ⇒ hg_s=+0.0` 有限（kernel L138–144）。✓ 参数面：`PARAMS_LEN=48`，`v[43]`/`v[44..=47]` 界内；全 kernels/ 目录 grep 确认除 fork 外**无一同族 kernel 读 params[43..47]**（g14_3_direct_gi/g16/g18 母版均不读）⇒ 即使显式 SPV 覆盖路径，`params[43]=1.0` 也不会毒化其他 kernel。✓
- **3.5 pass 图一致性**：bloom 变体 pop encode → push bright(4)/blurH(5)/blurV(6)/composite(7)/encode(8)，与 `prepare_update` 的 parity override 下标 (4,7,8) 互核一致；屏障计划含 U_OUT_COLOR 双 parity 保守超集；窗口车道顺序全同步（frame_slots=2 无真 FIF）⇒ bloom 单缓冲中间件无跨帧竞争。✓

---

## 4. 风险登记（隐患，按严重度）

1. **CONCERN｜`--dither on` 配默认 SPV = 静默无效**：默认 `--spv-encode` = `.tmp/g14_gates/m_c/g31_display_encode.spv`（2026-08-27 18:07，**dither 前构建**，sha256 C4A7ADF3… ≠ 夜间件 BA638A31…）。旧 kernel 不读 params[3] ⇒ on 臂静默无效果。夜间验收实际走 `--spv-encode` 显式覆盖（`artifacts/night_0828/d2_window/d2w_verify.py:29,90`）。off 臂两臂均位级不受影响。**建议**：合入时同步重建默认路径 SPV，或 `--dither on` 加 SPV 新鲜度 fail-closed。
2. **CONCERN｜`--smooth-normals on` + 显式 `--spv-scene` 无 fail-closed**：两 bin 均"尊重不覆盖"用户 SPV，但车道仍绑 8 路 storage；若用户给 7 路母版/direct_gi SPV，绑定数 mismatch 行为未经校验（注释自承"须为 8 路绑定面 kernel"）。与其他臂的 fail-closed 纪律不一致，建议补校验或显式拒跑。
3. **登记｜on 臂 NaN 暴露面**（opt-in，低概率）：畸形资产 NORMAL 含 NaN、或 `xform_dir` 溢出 inf 经 host `norm3` 除法（`inf/inf=NaN`）⇒ trinrm NaN ⇒ on 臂 NaN 像素；母版面对资产法线天然免疫。装配面无有限性校验。
4. **登记｜非均匀缩放法线方向错误**（已如实登记于代码注释 + d2_evidence known_gaps）：`xform_dir` 未接逆置变换，bistro（旋转+平移）正确，非均匀缩放场景静默着色偏差。
5. **登记｜IGN f32 精度**：≥4K 宽度下 `t1` 整数域占位压缩小数精度（≈2^-17@1080p → 2^-15@8K）——图案质量微降，确定性不受影响。
6. **登记｜pre-existing 同构面**（非本次引入，仅备忘）：NaN 输入经 encode `as u32` 未定义（上游 TSR 门拦假设）；0 字节 SSBO（bistro quads=0 既有先例，后端实测容忍）。

---

## 总判：**可安全合入** ✅（附两项处置建议）

默认臂/既有门零漂移有**机器证明**（D2：post_off == 改前基线 == G36 在案锚 `f39e9808…`；D1/窗口：off == `5596a730`），冻结面与他人文件零触碰，kernel 边界/下标/参数面/屏障计划全部互核通过，无 OOB、无 NaN 通道、无崩溃面。两项 CONCERN（默认 encode SPV 陈旧致 `--dither on` 静默无效；显式 `--spv-scene` 覆盖面缺 fail-closed）均为 **on 臂操作性陷阱**，不威胁默认臂位级与治理面——建议合入前处置 #1（重建默认 SPV 或加守卫）、#2 可登记留窗。提交时务必将并行会话的 G35 收口三件（`G35_CONTRACT.md`/`00_MASTER_INDEX.md`/`11_ROADMAP.md`）按文件名择取分拆。

Now write the full review report as the final response.