<!-- Assisted-by: TraeCode:Kimi-K3（G31+ 波 C Task C8 支持渠道与版本政策文档化） -->
# Rurix 渲染器发布核对清单（机器门操作单）

> 所属：G31+ 波 C Task C8（G31_PLUS_COMMERCIAL_RENDERER_TODO §5 #55 交付件，与
> [support_policy.md](support_policy.md) 姊妹篇）。
> 用途：渲染器 SDK 每次发布前**逐项核验**的操作单——全部条目引用仓内**真实存在**的 ci 脚本与在案
> 注册表（脚本存在性由 `ci/g31_support_policy_smoke.py` 门 `g31.waveC.support` 机器核验，防文档腐化）。
> 纪律：本清单是操作单、不是判据源——各项判据以各脚本/契约自身为准；任一项 RED 或
> DEV_ENV_DEGRADE（SKIP）**不得发布**（三态纪律：SKIP 不冒充 PASS，`RURIX_REQUIRE_REAL=1` 下降级翻硬 FAIL）。

---

## 1. stable ABI 守卫

- [ ] `py -3 ci/stable_snapshot.py` —— `renderer_sdk_api` 段零漂移（`sdk.rx` 9 导出规范化签名 +
  `abi_version` = 1.0.0，快照件 `tests/stable/stable_api.snapshot`）。
- [ ] 若有漂移：先判档（MAJOR = Full RFC；MINOR = `RURIX_BLESS=1` 重 bless +
  `tests/stable/bless_log.md` 追加；PATCH = 快照 0-byte），未留痕不得发布。
- [ ] `git status --porcelain` 面快照与 bless_log 改动成对出现（漂移无 bless 留痕 = 静默破坏，拒发）。

## 2. 渲染器面 gate 套件（波 A/B/C 全绿）

| 波 | gate key | 脚本 |
|---|---|---|
| A | `g31.waveA.present` | `ci/g31_window_present_smoke.py` |
| A | `g31.waveA.pipelining` | `ci/g31_frame_pipelining_smoke.py` |
| A | `g31.waveA.gameloop` | `ci/g31_game_loop_smoke.py` |
| A | `g31.waveA.dynscene` | `ci/g31_dynamic_scene_smoke.py` |
| A | `g31.waveA.framegen` | `ci/g31_framegen_present_smoke.py` |
| A | `g31.waveA.anchor_check` | `ci/g31_wave_a_anchor_check.py` |
| B | `g31.waveB.hzb` | `ci/g31_hzb_wiring_smoke.py` |
| B | `g31.waveB.restir` | `ci/g31_restir_wiring_smoke.py` |
| B | `g31.waveB.slab` | `ci/g31_slab_wiring_smoke.py` |
| B | `g31.waveB.texture` | `ci/g31_texture_sampling_smoke.py` |
| B | `g31.waveB.skinning` | `ci/g31_skinning_wiring_smoke.py` |
| C | `g31.waveC.sdk` | `ci/g31_renderer_sdk_smoke.py` |
| C | `g31.waveC.docs` | `ci/g31_renderer_docs_smoke.py` |
| C | `g31.waveC.capability` | `ci/g31_capability_fallback_smoke.py` |
| C | `g31.waveC.robustness` | `ci/g31_robustness_smoke.py` |
| C | `g31.waveC.ngx_decomp` | `ci/g31_ngx_decomposition_smoke.py` |
| C | `g31.waveC.support` | `ci/g31_support_policy_smoke.py`（本清单与支持政策门） |

> 各门 evidence 落 `evidence/` 对应前缀件（PASS-only；无件 = 门未过），schema 路由由
> `ci/check_schemas.py` 机核。门登记表 = `milestones/g31/CI_GATES.md`。

> **G37 商业化收官追加（2026-08-30）**：① 套件追加 `g31.g37w1.encode_parity`
> （`ci/g31_encode_parity_smoke.py`，ACES encode 共享面收编 v2 的防复发硬门）与
> `g31.waveC.license`（`ci/g31_vendor_license_smoke.py`，vendor 许可矩阵 + GAP closure 腿）。
> ② 窗口默认档已翻 `--quality full`（十九臂，W4）：诊断类门调用面已补 `--quality off`
> （对账表 `artifacts/day_0830_delivery/w4_flip/QUALITY_OFF_SWEEP.md` A 类 18 点）；默认臂门
> （A1 present / A3 gameloop / A6 soak / RD-045 P02）语义随翻转升级，按 W4 复跑清单以新默认核验。
> ③ presented 锚 = 二进制绑定锚——发布重建后整批重收割（锚登记 = `w4_flip/W4_ANCHORS.json`），
> 跨重建可沿用面仅 all-off 与 bench 锚。

## 3. 签名 / SBOM / 分发链

- [ ] `py -3 ci/release_bundle_smoke.py` —— 发布 bundle 打包冒烟（EA1.2 / RFC-0012，RXS-0218；
  签名 + SBOM + 信任根面）。
- [ ] `py -3 ci/rurixup_dist_smoke.py` —— rurixup 真实分发冒烟（EA1.1a/EA1.1b，RXS-0214~0217；
  hermetic 环回 + 截断拒收面）。
- [ ] `py -3 ci/emit_trust_root_entry.py` —— 信任根登记条目生成（RFC-0012 §4.7，RXS-0218）。
- [ ] `channels/stable.json` 通道登记核对（现登记语言工具链 `v1.0.1-dist` 系列）。
- [ ] **渲染器 SDK bundle 进分发链 = 待建立（C5 在飞，support_policy.md §5）**——落地前渲染器 SDK
  不以分发链形态发布，只以仓库构建形态交付（[integration_guide.md](integration_guide.md) §3）。

## 4. 许可 / 再分发面

- [ ] `py -3 ci/check_redistribution.py` —— NVIDIA 再分发面守卫（Attachment A 白名单 + 禁捆绑面）。
- [ ] `py -3 ci/fatbin_dist_smoke.py` —— 生产分发 fatbin 冒烟（G-G1-5 / RXS-0150~0152；含再分发
  分区与 lockfile coverage 面）。
- [ ] 超分 vendor 许可面在案核对：`milestones/g13/design/vendor_upscale_license_clearance.md`。
- [ ] **全 vendor 面商用再分发许可矩阵 = 待建立（C6 在飞，support_policy.md §5）**——商用形态
  发布前必须落地（DLSS/Streamline、FSR、Jolt、BasisU 等全矩阵 + SBOM 对账）。

## 5. 兼容矩阵

- [ ] `milestones/g31/g31_compatibility_matrix.json` 核对：`nvidia-ada-rtx4070ti` 格 `measured` 全绿；
  `amd-desktop` / `intel-desktop` 两格 `dev_env_degrade` 锚 G-MB1-6 如实登记。
- [ ] `py -3 ci/g31_capability_fallback_smoke.py --gate g31.waveC.capability` 新鲜真跑（探测面 +
  六链裁决 + 三后端切换；降级链 fail-closed 单测 `cargo test -p rurix-render --lib capability_matrix`）。
- [ ] 宣称多厂商支持前：AMD/Intel 格必须按同一探测面补测翻 `measured`（禁 mock 冒充）。

## 6. soak / 健壮性

- [ ] `py -3 ci/g31_wave_a_soak.py --gate g31.waveA.soak` —— 波 A soak 门（迭代零失败口径以其自身判据为准）。
- [ ] `py -3 ci/g31_robustness_smoke.py --gate g31.waveC.robustness` —— C4 运行时健壮性 + 故障注入
  （device-lost 三点 / TDR / budget 探针臂 + 基线 + 窗口风暴 + soak 故障臂）。

## 7. 文档与政策面

- [ ] `py -3 ci/g31_renderer_docs_smoke.py --gate g31.waveC.docs` —— 文档三件套节锚 + 在案数字
  防腐化 + 最小宿主示例真跑。
- [ ] `py -3 ci/g31_support_policy_smoke.py --gate g31.waveC.support` —— 本清单与支持政策门
  （引用脚本存在性 + 版本政策与 stable 快照一致 + 安全响应镜像 + 待建立项登记 + 冻结文档零触碰）。
- [ ] `py -3 ci/check_guardrails.py` 与 `py -3 ci/check_schemas.py` —— guardrail 字节级核对 +
  registry/evidence schema 路由全绿。
- [ ] [SECURITY.md](../../SECURITY.md) 渲染器面增补段在场（入口指针 → support_policy.md §3）。

## 8. 环境纪律（三态）

- [ ] `RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1` 全程；GPU 独占串行 =
  `ci/gpu_device_lock.py`（测量臂互斥）。
- [ ] 测量/发布件一律 **release 形态**构建（[integration_guide.md](integration_guide.md) §3；
  debug 仅开发）。
- [ ] 任一 gate 报 DEV_ENV_DEGRADE/SKIP → 该面**未验收**，不得计入发布就绪（三态纪律，禁冒充）。

---

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-25 | 初版（G31+ 波 C Task C8）：八面操作单（stable ABI 守卫 / 波 A·B·C 十七门 / 签名·SBOM·分发链 / 许可·再分发 / 兼容矩阵 / soak·健壮性 / 文档与政策 / 环境三态），全部条目引用真实 ci 脚本与在案注册表；C5/C6 在飞项明确标注「落地前不以对应形态发布」 |
| v1.1 | 2026-08-30 | G37 商业化收官同步（W5）：§2 追加 G37 注（encode_parity 与 license 两门进套件 / 默认档翻转后诊断门 off 字面对账与默认臂门新默认复跑口径 / presented 二进制绑定锚整批重收割纪律，W4_ANCHORS 登记）；历史表与 C5/C6 在飞标注字面不回写 |
