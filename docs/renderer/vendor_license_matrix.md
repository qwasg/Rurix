<!-- Assisted-by: TraeCode:Kimi-K3（G31+ 波 C Task C6 vendor 许可合规终审） -->
# vendor 许可合规矩阵（G31+ 波 C Task C6）

> **机器可读规范事实源** = [`milestones/g31/g31_vendor_license_matrix.json`](../../milestones/g31/g31_vendor_license_matrix.json)（本文 = 人类可读渲染面，字面以 JSON 为准；append-only 纪律同 JSON）。
> 兑现面 = G31_PLUS_COMMERCIAL_RENDERER_TODO §5 #53「vendor 许可合规终审」（商用分发口径 = 再分发许可合规）；门 = `ci/g31_vendor_license_smoke.py --gate g31.waveC.license`。
> 超分面（DLSS/Streamline/NGX + FSR + NRD）清结 = **引用不复制**：[`milestones/g13/design/vendor_upscale_license_clearance.md`](../../milestones/g13/design/vendor_upscale_license_clearance.md)（owner 2026-08-18 法律面接受 NVIDIA RTX SDKs LICENSE + FSR MIT 零障碍确认，M-a cleared 在案）。

## 1. 判定口径

| 判定 | 语义 |
|---|---|
| `cleared` | 商用再分发当前面无阻塞；义务已登记且当前分发面满足或不触发 |
| `conditional` | 当前分发面有未闭合义务/条件（条件 + 义务 + 缺口逐项登记） |
| `pending_owner` | 需 owner 法律面动作（agent 不冒充 owner 接受；G13 范式留痕） |
| `blocked` | 阻塞（处置登记） |

**owner 动作政策**：OSI 批准许可（MIT / Apache-2.0 / 双许可）沿 G13 §2 FSR MIT 先例 = 零障碍确认，无需 owner 逐项动作；非 OSI 自定义许可须 owner 留痕——DLSS/Streamline/NGX/NRD 面 owner 已在案（G13），NVIDIA CUDA EULA Attachment A 面 = 白名单机制清结（`ci/check_redistribution.py` + `src/rurixup/src/bundle.rs::audit_redistribution`，M5.4 起在案）。本批新核项全为 OSI 或既有机制面 → **零新 owner 动作（pending_owner = 0，如实登记）**。

## 2. 全 vendor 面盘点（16 项）

### 2.1 外部 SDK（运行时装载，零 git 零 bundle）

| # | 项 | 版本 / pin | 许可 | 判定 | 再分发要点 |
|---|---|---|---|---|---|
| 1 | Streamline SDK + NGX（DLSS SR） | 2.10.3；external/streamline-2.10.3（zip sha256 + 四 DLL digest 见 `milestones/g13/g13_vendor_sdk_registry.json`） | NVIDIA RTX SDKs LICENSE（自定义非 OSI） | **cleared**（G13 引用） | owner 接受在案；当前零再分发（二进制不入 git 不入 bundle）；商用捆绑时随附许可文本与 NVIDIA 归属 |
| 2 | FidelityFX SDK（FSR 3.1.5） | SDK 2.0.0；external/fidelityfx-sdk-2.0.0（digest 同登记） | MIT | **cleared**（G13 引用） | MIT 零障碍；捆绑 `amd_fidelityfx_*_dx12.dll` 时随附 license.md；当前零再分发 |
| 3 | NRD（RD040-nrd 面） | 未接入（0-byte 维持） | NVIDIA RTX SDKs LICENSE（同 DLSS 协议面） | **cleared**（G13 引用） | 与 DLSS 同批清结；未接入 → 零再分发面；接入触发按许可字面执行（RD-040 另判） |
| 4 | Taichi AOT 运行时（taichi_c_api.dll） | 用户自备；`RURIX_TAICHI_C_API_DLL` 运行时装载 | Apache-2.0（taichi-dev/taichi） | **cleared** | 当前面零再分发（spike 评估面，feature 默认 off）；未来捆绑运行时 → 随附 Apache-2.0 声明（GAP-01 同链） |

### 2.2 树内 vendored native 库

| # | 项 | 版本 / pin | 许可 | 判定 | 再分发要点 |
|---|---|---|---|---|---|
| 5 | JoltC（rurix-physics-sys 5.3 基线） | main @ `2982004387a9e36ca89525a87d983709d3666da7` | MIT OR Apache-2.0 | **cleared** | 许可文本在树（`vendor/JoltC/LICENSE-MIT` + `LICENSE-APACHE`）；义务 = 声明保留；当前 bundle 无物理组件 → 未触发 |
| 6 | JoltPhysics 5.3.0（生产默认后端） | commit `0373ec0dd762e4bc2f6acdb08371ee84fa23c6db` | MIT | **cleared** | `vendor/JoltC/JoltPhysics/LICENSE` 在树；义务同上 |
| 7 | JoltC（rurix-physics-sys56 5.6 评估线） | 同一 commit `2982004`（JPC56_/JPH56 符号隔离） | MIT OR Apache-2.0 | **cleared** | 许可文本在树；评估臂 feature `jolt56` 默认 off 不进分发 |
| 8 | JoltPhysics 5.6.0（评估线） | tag v5.6.0 = `e77f175595e64cb44218cc9d9d56fc365ad0e36a` | MIT | **cleared** | `JoltPhysics/LICENSE` 在树；评估臂默认 off |
| 9 | basis_universal（纹理 codec） | tag 1.16.4 @ `900e40fb5d2502927360fe2f31762bdbb624455f` | Apache-2.0 | **cleared** | `vendor/basis_universal/LICENSE` + `LICENSES/` + crate `NOTICE` 在树；义务 = 许可文本随附 + 声明保留（§4.1/§4.4）+ 专利授权（§3）；无 patch 原版快照（§4.2 修改声明不触发）；当前 bundle 无 rxcook → 未触发 |
| 10 | rurix_basis_shim（旧过渡 shim） | 已停用不编译 | MIT OR Apache-2.0（自有） | **cleared**（信息项） | 自有代码；不编译/不链接/不分发 |

### 2.3 NVIDIA CUDA EULA 面（Attachment A 白名单机制）

| # | 项 | 形态 | 许可 | 判定 | 再分发要点 |
|---|---|---|---|---|---|
| 11 | libdevice.10.bc | 运行期 CUDA_PATH 定位，**不入产物** | NVIDIA CUDA EULA Attachment A | **cleared** | Attachment A 明确可再分发；当前零再分发 = 嵌入 PTX 无 `__nv_*` + 源无 .bc 打包（`ci/check_redistribution.py` 机核在案）；若捆绑限白名单最小集（`audit_redistribution` 门禁，红线 r6） |
| 12 | cuBLAS runtime（cublas64_*/cublasLt64_*） | 系统 DLL 动态加载 | NVIDIA CUDA EULA Attachment A | **cleared** | 同上；仓库零 DLL/导入库 + 候选名白名单机核在案 |

### 2.4 Rust crate 面（Cargo.lock pin）

| # | 项 | 版本 | 许可 | 判定 | 再分发要点 |
|---|---|---|---|---|---|
| 13 | rowan（rurixc 语法树） | 0.15.1 | MIT OR Apache-2.0 | **conditional** | **编译进 rx.exe 已随 v1.0.1-dist.1/.2 分发**；声明保留义务未闭合（发布资产零许可文本/零第三方声明）→ **GAP-01**，闭合后转 cleared |
| 14 | rapier3d（+ 传递闭包 nalgebra/parry3d/simba/glam 等） | =0.33.0 | Apache-2.0（闭包 MIT/Apache 族） | **cleared** | feature `rapier` 默认 off 不在分发面；启用分发时随附声明（GAP-01 同链） |
| 15 | cc（build-dep ×3 crate） | 1.x | MIT OR Apache-2.0 | **cleared** | 构建期工具，不进产物，零再分发面 |
| 16 | cmake（build-dep ×2 crate） | 0.1.x | MIT OR Apache-2.0 | **cleared** | 同上 |

**计数**：cleared 15 / conditional 1 / pending_owner 0 / blocked 0（total 16）。

## 3. SBOM 对账

| 对账腿 | 结论 |
|---|---|
| 发布 SBOM 生成机制（`src/rurixup/src/sbom.rs`，SPDX 构建视图 + CycloneDX 发布视图） | bundle 每组件逐一带 `licenseConcluded`（SPDX）/ `licenses[].license.id`（CycloneDX）；`components_covered` 机器判据在案（RXS-0138，缺组件即不齐备阻断发布） |
| 现发行面（v1.0.1-dist.2 = rx.exe / rurixup.exe / rurix_rt_cabi.lib） | 3 组件全 language-core、许可登记在 release.yml `--component` 三段（见 GAP-02 单标口径）；SBOM×2 随资产上传 → 每个 SBOM 条目有许可登记 ✓ |
| vendor ↔ SBOM 映射 | basis_universal → `src/rurix-basis-sys/SBOM.md`（SPDX Apache-2.0 + 双 digest）；streamline/fidelityfx → `milestones/g13/g13_vendor_sdk_registry.json`（许可 + 逐 DLL digest）；Jolt 四面 → VENDOR.md/VENDOR56.md pin+许可登记；NRD/Taichi/构建期 crate = 未接入/用户自备/不进产物如实登记 not_applicable |
| 缺口 | GAP-03：SBOM 粒度 = 组件级，未展开二进制内嵌第三方库（rx.exe 内 rowan）→ C5 SBOM 扩展面 |

## 4. 义务落实核验（现发行面；C5 在飞以 v1.0.1-dist.2 + release.yml 为准）

| 义务 | 核验 | 结论 |
|---|---|---|
| 树内 vendored 库许可文本保留（MIT/Apache 要求） | JoltC ×2 线 LICENSE-MIT/APACHE、JoltPhysics ×2 线 LICENSE、basis_universal LICENSE + LICENSES/ + NOTICE 全部在树非空（门腿 `license_texts_on_tree` 机核） | ✓ 闭合 |
| 分发件随附许可文本/第三方声明 | release.yml 资产 = 3 二进制 + bundle/channel/SBOM×2/signing/gate/SHA256SUMS，**零 LICENSE/NOTICE 件**；rx.exe 内嵌 rowan（第三方 MIT/Apache）+ Rurix 本体双许可文本均未随附 | ✗ **GAP-01**（open，归 C5/后续分发链） |
| 组件许可登记与 Cargo 元数据一致 | release.yml 单标 `Apache-2.0` vs workspace `MIT OR Apache-2.0` | ⚠ **GAP-02**（consistency_note；保守单标不减损权利） |
| owner 接受记录面对新 cleared 项 | 本批新核项全 OSI（Jolt/BasisU/Taichi/rowan/rapier/cc/cmake）→ 沿 G13 FSR MIT 先例零障碍确认，**零新 owner 动作**；非 OSI 面 = G13 在案 + Attachment A 机制在案 | ✓ pending_owner = 0 如实登记 |

## 5. 缺口登记（append-only）

| 缺口 | 内容 | 处置归属 | 状态 |
|---|---|---|---|
| GAP-01 | 发布 bundle 未随附许可文本与第三方声明（本体双许可 + rowan；未来 Jolt/BasisU/FSR/NGX/Taichi 捆绑面同链） | G31+ 波 C Task C5（#52 分发打包/SBOM 扩展）或后续分发链波次 | open |
| GAP-02 | release.yml 许可单标 Apache-2.0 vs workspace 双许可字面不一致 | 同 GAP-01 链 | open |
| GAP-03 | SBOM 组件级粒度未展开内嵌第三方库 | C5 SBOM 扩展面 | open |

## 6. GAP closure 登记（G37 商业化收官 W5，2026-08-29；append-only 追加节）

> 登记纪律：§5 既有行与 JSON `gaps[].status` 字面**不改写**（evidence schema 把 gap status 钉为
> const `open`、summary 钉为 cleared 15/conditional 1——闭合态以 JSON `gaps[].closure` 追加段
> 与本节承载）。机器核验 = `ci/g31_vendor_license_smoke.py --gate g31.waveC.license` closure 腿
> （closed_date/actions/evidence 逐路径在树 + GAP-01 随附面接线 + GAP-02 字面互核 + GAP-03
> 与 Cargo.lock rowan 版本互核）。

| 缺口 | closure 日期 | 闭合动作 | 产出/证据 | 残余 |
|---|---|---|---|---|
| GAP-01 | 2026-08-29 | 第三方声明与许可文本集合落盘（rx.exe 内嵌 rowan 0.15.18 + 传递闭包 countme/hashbrown/memoffset/rustc-hash/text-size，上游源包 LICENSE 逐字随附；rurixup.exe / rurix_rt_cabi.lib 零第三方如实登记；SDK/未来捆绑面义务同链登记）+ release.yml 追加 4 组件进 digest 闭环与资产清单（LICENSE-MIT / LICENSE-APACHE / THIRD_PARTY_NOTICES.md / third_party_embedded.cdx.json） | `dist/licenses/THIRD_PARTY_NOTICES.md`、`.github/workflows/release.yml`、`artifacts/day_0830_delivery/w5_commercial/license_gaps/REPORT.md` | 历史 v1.0.1-dist.1/.2 已发布资产不可追溯补件；对下一次 release run 起效 |
| GAP-02 | 2026-08-29 | release.yml 三个二进制 `--component` 许可段 `Apache-2.0` → `MIT OR Apache-2.0`（与 `Cargo.toml` workspace 字面逐字一致；SBOM licenseConcluded 同源同字面） | `.github/workflows/release.yml`、`Cargo.toml` | 无（各冒烟夹具单标为夹具自述，不在本 GAP 义务面） |
| GAP-03 | 2026-08-29 | 内嵌第三方库级 CycloneDX 补充视图落盘并进发布资产（rx.exe → rowan 闭包 6 crate；两二进制零第三方如实登记；SDK 面 basis_universal static-in-dll 同批展开）；组件级生成机制 0-byte 不动 | `dist/sbom/third_party_embedded.cdx.json`、`Cargo.lock` | sbom.rs 生成器自动展开（Cargo.lock 驱动）归后续 src 授权批次 |

- `rust_rowan`（§2.4 #13）条件「GAP-01 闭合后转 cleared」的义务面已兑现（JSON `closure_note` 留痕）；
  `redistribution_status` 字面维持 conditional 不改写，正式改判 cleared 归下一次矩阵版本化修订
  （需 evidence schema summary const 同批修订）。
- 「附带义务未闭前不以对应形态发布」口径（G33 登记语义）：GAP-01~03 随附/字面/展开三义务已在
  分发编排内闭合，SDK bundle 分发链的许可前置由本节 + closure 腿承载。

---

*本文件随矩阵 JSON 只追加；判定变更须新行留痕，禁原地改判。*
