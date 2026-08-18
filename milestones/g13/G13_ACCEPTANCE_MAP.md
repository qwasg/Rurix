<!-- Assisted-by: Kimi-K3（G13.1 治理波起草） -->
# G13_ACCEPTANCE_MAP — P0 验收映射

> **性质**：G13.1 治理交付物（governance-only）；事实源为 [G13_CONTRACT.md](G13_CONTRACT.md) v1.0 front matter acceptance_gates（G-G13-1~9）与 §4.2 五行 P0 独立断言表、2026-08-18 G13 立项前调研报告（P0 建议清单 M-a~M-e 与 §6 判据草案——主会话留痕）、[G13_CANDIDATE_DECISIONS.md](G13_CANDIDATE_DECISIONS.md) v1.0（§3 新增候选 → M-a~M-e 映射，§1 G12 defer 22 行处置，§2 RD-041 FSR/DirectSR 分项与 RD040-nrd 决策窗承接）。
> **编号纪律**：本表 P0 行只冻结 symbolic CI gate key 与稳定脚本名，不 claim 数字 CI step。数字步骤必须等 G-G13-3 实现互锁（§6）通过后，再从 `registry/number_ledger.json` 当时实测的 `CI_step.next_free` 顺位分配——**P0 行 numeric_step 一律写 `post-interlock actual-next-free allocation`**（当前实测 `CI_step.next_free=233`，G12 已消费至 232，ledger v1.134）；禁止沿用任何草案建议值。**例外面**：G13.1 治理三门（§5）按 2026-08-18 任务面明令本波即落盘真脚本真步骤——步骤 233~235 按落盘前实测 actual next_free=233 顺位领取，ledger 校准同批。
> **M 行号纪律**：M-a~M-e 字母行号为治理期稳定身份；M### 数字在 G13.2+ 实现波 materialize 时按落盘前实测 M 命名空间实际顺位领取（沿 G12 M158~M166 先例），本表不预占 M 数字。
> **证据纪律**：表内脚本与 schema 是实现 PR 的强制目标路径；二者须与对应 RED→GREEN 实现同 PR 落地。路径尚未 materialize 只表示未开工，不能记 PASS；本波不预建空脚本、空 schema 壳或占位 workflow 步骤（只冻结路径——治理三门为例外，见上）。

---

## 1. P0 硬门（精确 5 行）

- P0 精确集合（5 行）：`{M-a, M-b, M-c, M-d, M-e}`。每一行的 symbolic key 同时是该能力唯一的 `assertion_id`，与 [G13_CONTRACT.md](G13_CONTRACT.md) §4.2 **逐字一致**（key 命名空间双方一致性机核面，禁止任何改写）；独立硬判据列同逐字。实现后的 evidence 顶层必须至少含：`schema_version`、`subject`、`milestone`、`wave`、`assertion_id`、`status`、`commands`、`environment`、`base_commit`、`run_url`、`timestamp`；其中 `assertion_id` 必须等于本表 key，`status` 必须为 `pass|fail`，不得以 `skip|estimated|advisory` 充绿（条件未触发只能登记 `not-triggered`、环境缺失只能登记 `dev_env_degrade`，见 §3）。
- 每个 symbolic key 最终一对一取得一个 numeric CI step。脚本可复用，但 workflow 必须按 `--gate <symbolic-key>` 独立调用、独立产 evidence、独立给结论；任一 P0 assertion 缺失或失败，只能使该 P0 为红，**不得用同脚本内另一 assertion 的绿色代替**。一次 smoke 可共享进程启动成本，但每行必须单独产出 `PASS|FAIL|SKIP|DEV_ENV_DEGRADE`；只有 `PASS` 满足 P0。
- **单一 key/脚本命名空间**：symbolic key 一律小写点分 `g13.p0.m_<a~e>.<slug>`，脚本一律 `ci/g13_<slug>_smoke.py`，evidence schema 一律 `milestones/g13/g13_m_<a~e>_<slug>_evidence_schema.json`（slug 与 key 末段同字面，只冻结路径不预建文件）。本表 §1 与 [G13_CONTRACT.md](G13_CONTRACT.md) §4.2 引用同一份 key/脚本，由 `ci/g13_acceptance_map_check.py` 双向比对强制一致（见 §4；G13 治理三件套无独立 CI_GATES——门冻结面 = 契约 §4.2 + 本表 §1/§2，沿 G12 体例精简）。
- device 判据统一要求真实路径、`RURIX_REQUIRE_REAL=1`、validation error 为 0；mock、stub、host substitution、仅检查非零输出、缺 provisioning 的 SKIP 均不满足能力门。host oracle、既有最小见证、人工截图均不能替代目标门。
- **统一判据形态**（M-a~M-e 共用纪律字面）：接入/落盘 + 冻结面 0-byte（UpscaleBackend trait 签名面与 temporal 底座历史接口面 / G11 GI 既有判据 / M96 golden 门序 D2-Q7）+ measured 面标定程序产阈禁手写（P-09）+ 不降级既有 71 门绿面。**G13 不设绝对 DLSS/超分画质通过线**——「已达 UE5 超分画质」判定归 G15 商用收口期（契约 §1/§5 字面）；**M-c 帧率面 zero_pass_line 不设通过线**——正式帧率对标锚定 G14（G10-N11/N16 承接锚字面 0-byte）。
- 负例 RED 臂列：「内联」= 契约 §4.2 判据字面中已含的 RED 臂（逐字摘录，与判据列同源）。

| M 行 | Symbolic CI gate key / 稳定脚本 | Evidence schema（目标路径，只冻结不预建） | 独立硬判据（契约 §4.2 逐字） | 负例 RED 臂 | device/host 性质 | 最晚波次 | numeric_step |
|---|---|---|---|---|---|---|---|
| **M-a** | `g13.p0.m_a.vendor_upscale_integration`<br>`py -3 ci/g13_vendor_upscale_integration_smoke.py --gate g13.p0.m_a.vendor_upscale_integration` | `milestones/g13/g13_m_a_vendor_upscale_integration_evidence_schema.json` | vendor 超分接入：许可前置 owner 法律面清结留痕（未清结即 blocked 不充绿）+ DLSS SR 经 Streamline SDK（2.10.3 + NGX 签名 DLL，Vulkan interop 臂）真跑出帧（RURIX_REQUIRE_REAL=1 + validation 零错误，RTX 4070 Ti）+ FSR 3.1.5 同接口档（同一 UpscaleBackend 冻结面，FSR4 ML 自动回退登记）+ 双端超分帧对拍 measured 登记（vs 自研 TSR host 金标准同输入帧集，SSIM/逐像素 diff 口径 RXS-0387/0388 继承）+ UpscaleBackend trait 签名面与 temporal 底座 0-byte 机核（目录级 diff）+ 树内零 UE/vendor 源码 vendoring；许可未清结开工即 RED；底座接线即 RED；mock/stub 充真跑即 RED；单 vendor 缺臂聚合 PASS 即 RED | 内联：许可未清结开工即 RED；底座接线即 RED；mock/stub 充真跑即 RED；单 vendor 缺臂聚合 PASS 即 RED | host+device | **G13.2** | post-interlock actual-next-free allocation |
| **M-b** | `g13.p0.m_b.tsr_device_kernel`<br>`py -3 ci/g13_tsr_device_kernel_smoke.py --gate g13.p0.m_b.tsr_device_kernel` | `milestones/g13/g13_m_b_tsr_device_kernel_evidence_schema.json` | 自研 TSR device 化：tsr.rs host 金标准 → .rx kernel device 面（复用 G12 PT megakernel 车道，rurixc --target vulkan 产 SPV + spirv-val 通过）+ device vs host 金标准同输入逐帧对拍（容差标定程序产禁手写）+ 50/67/100% 三档质量/帧时 measured 对照入 g13_budget（measured_local 零 estimated，50×3 trimmed mean 协议沿 M141/M165 字面）+ 固定 seed 位级确定性协议维持 + host 金标准面 0-byte；host/device 对拍超容差静默即 RED；estimated 冒充 measured 即 RED；确定性协议漂移即 RED | 内联：host/device 对拍超容差静默即 RED；estimated 冒充 measured 即 RED；确定性协议漂移即 RED | host+device | **G13.3** | post-interlock actual-next-free allocation |
| **M-c** | `g13.p0.m_c.ue_upscale_parity`<br>`py -3 ci/g13_ue_upscale_parity_smoke.py --gate g13.p0.m_c.ue_upscale_parity` | `milestones/g13/g13_m_c_ue_upscale_parity_evidence_schema.json` | UE5 超分双端对拍：复用 G12.4 MRQ harness 扩 DLSS 臂（UE 5.8.1 DLSS 插件面 vs Rurix M-a/M-b 超分面，同场景同档位双端出图；UE build digest == M128 登记 ue_build_id 机核继承）+ SSIM/FLIP/噪声谱差距登记表落盘（差距项显式登记即 RED 评审面，不静默混入）+ 帧率 measured 基线登记 **zero_pass_line 不设通过线**（G10-N11/N16 锚定 G14 字面）+ 单端缺帧聚合不得 PASS；以基线冒充帧率对标即 RED；差距项静默混入即 RED；契约 digest 不等仍出报告即 RED | 内联：以基线冒充帧率对标即 RED；差距项静默混入即 RED；契约 digest 不等仍出报告即 RED；单端缺帧聚合 PASS 即 RED | host+device | **G13.4** | post-interlock actual-next-free allocation |
| **M-d** | `g13.p0.m_d.ue_lumen_gi_parity`<br>`py -3 ci/g13_ue_lumen_gi_parity_smoke.py --gate g13.p0.m_d.ue_lumen_gi_parity` | `milestones/g13/g13_m_d_ue_lumen_gi_parity_evidence_schema.json` | UE Lumen GI 对照：Rurix M98/M99/M154 GPU GI 面（屏幕探针近场 + 世界辐射缓存远场 + 多反弹链，G9.4/G11.4 已验收面只消费不改写）vs UE Lumen 同场景双端出图 + GI 能量/间接光 measured 对拍（容差标定程序产）+ Lumen 差距登记表落盘（UE Lumen 模块归属，RXS-0391 归属枚举口径继承）+ G11 GI 面既有判据 0-byte；Lumen 差距项静默混入即 RED；GI 既有门降级即 RED；单端缺帧聚合 PASS 即 RED | 内联：Lumen 差距项静默混入即 RED；GI 既有门降级即 RED；单端缺帧聚合 PASS 即 RED | host+device | **G13.4** | post-interlock actual-next-free allocation |
| **M-e** | `g13.p0.m_e.regression_drift_guard`<br>`py -3 ci/g13_regression_drift_guard_smoke.py --gate g13.p0.m_e.regression_drift_guard` | `milestones/g13/g13_m_e_regression_drift_guard_evidence_schema.json` | 回归门 + 漂移监控：既有 71 门（G9 34 key + G10 14 key + G11 14 key + G12 9 key）最新 evidence 全绿只读汇总（聚合不遮蔽子断言 FAIL/SKIP/DEV_ENV_DEGRADE）+ G13 触改面既有门重跑回归零降级（M96 golden 门序面真跑抽检）+ M165 漂移监控登记（G13 复跑面同型 digest 漂移检出计数/零检出字面入 evidence，FAIL 件 0-byte 保留纪律继承）；既有门降级即 RED；聚合遮蔽即 RED；漂移检出未登记即 RED | 内联：既有门降级即 RED；聚合遮蔽即 RED；漂移检出未登记即 RED | host 纯 host（抽检子进程自持 device 面） | **G13.5a** | post-interlock actual-next-free allocation |

任一行缺失、合并后不可区分、非 `PASS` 或无对应 evidence schema，均阻断 G13.5b（契约 §4.2 末段字面）。**G13 不设绝对 DLSS/超分画质通过线**——本表不设任何绝对画质 FLIP/SSIM 通过线；**M-c 不设帧率通过线**（zero_pass_line 登记，锚定 G14）。

---

## 2. 已 go P1 硬门（零行）

G13.1 无 go 的 P1 行——调研报告 P0 建议清单 5 行（M-a~M-e）全为 P0（契约 §4.2 末段字面）。后续波次若治理程序将新 P1 判为 go，须先按治理程序修订本表及覆盖集合（只追加进 §2）再开对应实现；不得把它静默并入现有 key。本节的机核面 = §2 零行声明与 `ci/g13_acceptance_map_check.py` 的 P1 空集断言（§5）。

---

## 3. 条件型 / not-triggered 登记面

### 3.1 异己并发工作树面（不混入零消费 G13 车道）

立项裁决 1（契约 §7 逐字登记）：G13 带未提交项立项——工作树异己会话 src/ 未提交面（2026-08-18 `git status` 实测：apps/uc06-renderer、apps/uc08-physics、src/rurix-asset/src/lib.rs、src/rurix-render/src/{bin/g10_m134_frame_capture.rs, bin/g9_m95_visbuffer_swhw.rs, geometry/mod.rs, gi/mod.rs, lib.rs, shadow/mod.rs}、src/rurix-rt/src/render_exec.rs 改写面 + evidence/d3d12_interop_smoke.json、milestones/g12/g12_pt_sampler_selection.json 异己改写面）保持不混入 G13 车道、严禁消费（G10.8b §8.10/G11/G12 先例同模）。纪律：G13 车道 commit 只含 G13 车道文件；异己面 evidence 不充 G13 任何门绿；若 G13 实现波与异己面触及同一文件，按只追加程序登记冲突面并请治理裁决（不得静默合并）。

### 3.2 触发评估 / 决策窗登记面（G12 defer 行的 G13 窗）

- **G10-N5 DLSS/Streamline 方向（G13 兑现窗）**：承接锚「DLSS/超分立项且许可与 ABI 评估齐备」——G13 立项本身即「立项」条件命中；许可与 ABI 评估齐备面 = M-a 开工硬门（契约 §7 裁决 5：owner 法律面清结留痕），未清结 M-a 保持 blocked。
- **RD-041 FSR/DirectSR 分项（G13 兑现窗）**：随 G10-N5 同族——M-a 波 FSR 3.1.5 经 UpscaleBackend trait 接入即本分项接入评估兑现面（接口已冻结不改底座字面）；FG/MFG 独立层另判字面不动。
- **RD040-nrd 接入裁决（G13 决策窗）**：G12.3 评估已完结不接线（评估报告 milestones/g12/design/nrd_vendor_denoise_evaluation.md v1.0 在树）；G13 接入裁决三条件 = 接入真实需求 + owner 法律面许可清结 + measured 对拍面齐备，未齐备维持不接线——不以 NRD 评估报告冒充接入。
- **M52 SER（G13 登记）**：G13.4 若上 Lumen 化 workload 自然 materialize 高分歧 RT workload 面，按只追加程序重评登记（双条件 = 真实集成需求 + capability rt.ser 设备面实测）；未命中维持 defer（G12.1/G12.6 双窗核验 = 需求未至 + 设备面未实测在案）。
- **M100-high（锚定 G14 登记）**：G12.6 触发重判 = 证据未齐备（低档 MegaLights GPU 管线对照面未产）maintain-defer 锚定 G14 字面不动；异己 restir 面零消费维持。
- **G10-N17 M137 scalars.flip（G13.4 触发评估登记）**：M-c 对拍若消费 diff 报告 FLIP 标量面，按 RXS-0388 L3 演进位程序翻转实值并回归 M137 门；未消费维持 null 演进位（G12.4 触发评估已兑现=未触发在案）。

登记 `SKIP=not-triggered` 只表示决策已记录，不是成功，不充任何门绿。

### 3.3 帧率面 / 材质链边界登记面

- **帧率面（G10-N11/N16/G11-N3 锚定 G14）**：M-c 只建帧率 measured 基线登记 zero_pass_line；正式帧率对标（三轮进程级独立运行 + MRQ 开销剥离口径）锚定 G14 字面 0-byte；以基线冒充帧率对标即 RED（M-c 判据内联）。
- **材质链边界（G11-N8/G11-N9/G12-N10/G12-N12 锚定 G15）**：G13 超分/Lumen 对照若实测透射/焦散/镜面 IBL 类能量为画质量级主差，进 G13 新差距登记表显式归属登记但不承接修复——锚定 G15 画质量级收口面维持；`g12_ue_pt_gap_registry.json` 10 行终态只消费不回写（G13 新产差距另立新表）。

### 3.4 对拍契约面（G13.4 独立冻结口径）

G13.4 对拍契约参数（场景/相机/光照/档位/种子）独立冻结——digest 机核，不动 G10.5/G11.5b/G12.4 锁定值（closed 复测对照面 0-byte）；UE 臂 = UE 5.8.1（DLSS 插件面 / Lumen 面 MRQ 臂，窗口模式主路，G10-N8/N9 口径继承），UE build digest == M128 登记 ue_build_id 机核；曝光/位深口径沿 G11.2/G12.4 对齐口径（RXS-0385 strip-and-log / EV100 派生链），残余口径差显式登记（未对齐口径消费对拍 delta 即 RED）。

---

## 4. 双向一致与互斥面（key 命名空间机器可核声明）

1. **双方逐字一致**：本表 §1 五行与 [G13_CONTRACT.md](G13_CONTRACT.md) §4.2 五行对同一 P0 M 行给出的 symbolic gate key 与稳定脚本名**必须逐字相等**，由 `ci/g13_acceptance_map_check.py` 双向比对机器强制；任一处漂移即 FAIL。本声明为机器可核面：比对以文件字面为准，不以叙述替代。（G13 无独立 CI_GATES——G12 三向比对体例精简为契约 ↔ MAP 双向，契约 §4.3 治理三门表为第三冻结面。）
2. **唯一命名空间**：`g13.p0.m_<a~e>.<slug>` + `ci/g13_<slug>_smoke.py` + `milestones/g13/g13_m_<a~e>_<slug>_evidence_schema.json` 为唯一合法形态；G13 命名空间（`g13.*`）与 G9 已消费 34 key（`g9.*`）、G10 已消费 14 key（`g10.*`）、G11 已消费 14 key（`g11.*`）、G12 已消费 9 key（`g12.*`）互不包含；全部 key 全局唯一，匹配 `g13\.p0\.m_[a-e]\.[a-z0-9_]+`；没有两个 M 行共享 key。
3. **互斥**：M 行与 key 一对一；`no-go`/`defer` 项（如 G13-N6 异己面）不产生 key、不入本表，不得冒充 PASS；G12 defer 维持行（M61/SAFE-GPU/M127 等 22 行）不入本表；P0 集合变更属于契约变更，不得以勘误处理。
4. **防混淆登记**：`g13.p0.m_c.ue_upscale_parity` 等对拍门消费的 FLIP = 图像感知度量（RXS-0389），与 RD-044 族 FLIP（流体）同名不同物，互不构成触发（G10/G11/G12 口径维持）。
5. **基线锚溯源**：M-b 三档对照的统计协议（50×3 trimmed mean）转引自 M141/M165 冻结口径（BENCH_PROTOCOL §3）；M-c/M-d 对拍的度量口径（SSIM RXS-0387 / FLIP RXS-0389 / 逐像素 diff RXS-0388 / 差距清单 schema RXS-0391）为 G10/G11/G12 冻结面只消费不回写；`g12_ue_pt_gap_registry.json` 10 行只消费不回写（G13 新差距另立 `milestones/g13/` 新表）。

---

## 5. G13.1 治理覆盖与空行门

G13.1 治理三门（本波 materialize，步骤 233~235 按落盘前实测 actual next_free=233 顺位领取）：

```text
g13.wave.1.acceptance_map         步骤 233
  py -3 ci/g13_acceptance_map_check.py --gate g13.wave.1.acceptance_map

g13.wave.1.candidate_decisions    步骤 234
  py -3 ci/g13_candidate_decisions_check.py --gate g13.wave.1.candidate_decisions

g13.gov.implementation_interlock  步骤 235
  py -3 ci/g13_interlock_check.py --gate g13.gov.implementation_interlock
```

`ci/g13_acceptance_map_check.py` 的 PASS 判据（coverage + no-empty 两组断言分别独立报告）：

1. P0 行集合与 §1 的 5 项**集合全等**，无遗漏、无额外 P0、无重复；§2 P1 集合为空集（G13.1 零 go P1 字面）。
2. 全部 symbolic key 全局唯一，均匹配 `g13\.p0\.m_[a-e]\.[a-z0-9_]+`；每行只有一个 canonical `assertion_id`，没有两个 M 行共享 key；key 的 m 段字母与行号一致。
3. 每一行均有脚本命令（`--gate` 参数 == canonical key）、evidence schema、可机器求值的 PASS 判据、负例 RED 臂、device/host 性质、最晚波次。
4. **双向一致**：本表 §1 与 `G13_CONTRACT.md` §4.2 对同一 P0 M 行给出的 key 与脚本必须逐字相等；任一处漂移即 FAIL。

no-empty 组的 PASS 判据：

- 逐单元格拒绝空串、空白、`TBD`、`TODO`、`待定`、`待补`、`—`；表中全部行的必填列均非空。
- 所有 schema 路径必须唯一落到对应 M 行，且文件名含同一 `m_<a~e>` 与同一 slug；所有波次属于 `G13.2|G13.3|G13.4|G13.5a` 的非空集合。
- G13.1 只核映射完整性，不把尚未 materialize 的脚本/schema 误判为实现绿色；对应能力实现 PR 合入前或同 PR，`ci/check_schemas.py` 必须能校验该 schema 与实际 evidence。

`ci/g13_candidate_decisions_check.py` 的 PASS 判据：G13_CANDIDATE_DECISIONS 候选行闭集全等（§1 G12 defer 22 行 + §2 open RD 7 行 + §3 G13 新增候选行）+ 裁决枚举合法（go/no-go/defer-to-G14+/strategic_override）+ 零空行（全列非空）+ 承接锚纪律（§1 行承接锚 = G12.6 字面 0-byte 转引，含「→」分节与 G13+ 承接源字面；§3 行含「重判条件 + 兜底」字面）+ defer-to-G14+ 裁决行 G14+ 重评窗字面（裁决/最终状态列承载，转引列不回写）+ go 行验收映射锚义务（登记留痕位置含 G13_ACCEPTANCE_MAP）+ §2 RD 行条目级 status==open + 与本表 5 key 互斥（候选行 ID 不得命中已 go 门裸 token）。

`ci/g13_interlock_check.py` 的 PASS 判据：逐项读取事实源输出 §6 各条件真值；G13.1 期间必须诚实输出 `BLOCKED`（validator 能识别阻断，不算互锁 PASS 实现面）；仅全部条件为真时才输出 `READY`（`--require-ready` exit 0）；`--gate` 模式产 evidence（VERDICT 字面入档，BLOCKED 不充绿）。

三个 validator 均把每组断言逐条打印，不以一个总 `all_pass=true` 掩盖具体缺行；`--selftest` 用受控负样本证明它们能红。

---

## 6. G13.2 硬互锁

`G13.GOV.G13_2.ENTRY_INTERLOCK` 是 G13.2 的前置 required check（`ci/g13_interlock_check.py --require-ready`，步骤 235 同脚本）。以下条件必须**同时**为真（契约 front matter `implementation_unlock.required_all` 与 G-G13-3 字面展开）：

1. G12 已 closed（`milestones/g12/G12_CONTRACT.md` front matter `status: closed`，2026-08-17，flip commit `8c5dc5ee` + tag `g12-closed`），且 G13.0 文档集不可变 ref `8c5dc5ee` 已登记。
2. `G13_CANDIDATE_DECISIONS.md` 分项映射无空行；`registry/deferred.json` history 只追加、无静默改判（vs G13.0 base 条目四字段 0-byte）；本表 §1/§2 无缺行（D-G13-3）。
3. §5 的 `g13.wave.1.acceptance_map`（coverage + no-empty 两组）与 `g13.wave.1.candidate_decisions` 独立 PASS。
4. **M-a 许可前置**：DLSS（Streamline SDK 2.10.3 + NGX 签名 DLL）与 FSR 3.1.5 redistribution/集成许可的 owner 法律面清结留痕在树——未清结 M-a 保持 blocked 且 G13.2 不得开工（不得以 FSR MIT 面宽松冒充 DLSS NGX 面清结）。
5. G13 的 P0 实现门 numeric CI step claim 发生在上述互锁通过之后，并以 claim 当下 ledger 实测 `CI_step.next_free` 为起点；每个 symbolic key 一对一分配，未复用、未覆盖、未沿用任何草案建议值（治理三门步骤 233~235 为 G13.1 实测领取的合法面）。
6. 用户 G13.2 开工指令已留痕（2026-08-15 指令全期授权面——「支持 dlss、超分采样」字面，契约 §7 逐字登记）。

任一条件为假时，互锁必须返回非零；此时禁止合入 G13.2 的 `spec/`、`conformance/`、`src/` 或 workflow 实现改动。不能用 owner override 绕过任何条件，也不能用本表存在本身当作 G13.2 开工许可。`ci/g13_interlock_check.py --require-ready` 输出 READY 是机器事实，不以叙述替代（契约 G-G13-3 字面）。

---

## 7. Close-out 审计

- G13.5a 必须重跑全部 5 个 P0 与所有 go 的 P1 的**各自 assertion**；evidence 的 `base_commit` 必须落在同一候选 close-out 基线，零 skip/estimated；soak 量级沿 G12.7a 继承（≥1800s）或 measured 证明更短足够；`budget_eval --strict` 非空全 PASS；既有 71 门（G9 34 + G10 14 + G11 14 + G12 9）零降级（M-e 门承载）。
- G13.5b 只有在 5 个 P0 key 全 PASS、所有 go 的 P1 key 全 PASS、验收映射/候选决策/RD 最终状态逐字一致、**Lumen/超分差距清单终审锁定**（残余差距/未闭环行如实登记不冒充全闭环）时才可 status flip；任一 P0 无独立硬门则禁止 flip。
- 同日放行先例继承（立项裁决 7）：7a full-run 先行完成后允许同日进 close-out；条件实现刚绿不得跳过 soak 直接 close。
- **G13 零绝对通过线口径维持到 close-out**：不设绝对 DLSS/超分画质通过线（归 G15）、不设帧率通过线（锚定 G14）；任何「已达 UE5 DLSS/超分画质」叙述在 G13 期内一律不成立（契约 §5 字面）。
- 后续若治理流程将新的 P1 判为 go，须先按治理流程修订本表及覆盖集合，再开对应实现；不得把它静默并入现有 key。

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-18 | G13.1 初版：冻结 5 个 P0 的 symbolic gate、目标脚本/schema、独立判据（契约 §4.2 逐字）与最晚波次——vendor 超分接入 1 行（M-a，G13.2，许可前置硬门）+ 自研 TSR device 化 1 行（M-b，G13.3）+ UE 对拍 2 行（M-c 超分对拍 zero_pass_line / M-d Lumen GI 对照，G13.4）+ 回归门+漂移监控 1 行（M-e，G13.5a）；§2 零 go P1 声明；§3 条件型/not-triggered 登记面（异己并发工作树面 / 六行触发评估与决策窗 / 帧率面与材质链边界 / 对拍契约面）；§4 key 命名空间双方逐字一致机器可核声明（G13 无独立 CI_GATES，契约 ↔ MAP 双向）+ 基线锚溯源；单一命名空间 `g13.p0.m_<a~e>.<slug>` + `ci/g13_<slug>_smoke.py` + `g13_m_<a~e>_<slug>_evidence_schema.json` 由 `ci/g13_acceptance_map_check.py` 双向比对强制；§5 治理三门（步骤 233~235 实测领取）、§6 G13.2 硬互锁六条件（含 M-a 许可前置）、§7 Close-out 审计。P0 行数字 CI 步骤全部 `post-interlock actual-next-free allocation`（当前实测 CI_step next_free=233），零 P0 workflow/script/schema 预放。 |
