# mb1 Vulkan/SPIR-V 跨端后端 — 续作 HANDOFF（派发给新 agent）

> 本文件自包含。新 agent 从冷启动读本文 + `milestones/mb1/design/*.md`（4 份 Opus 实现设计,file:line 级）即可接手。**先读本文 §3 纪律再动手。**

---

## 0. 一句话现状

`Rurix 源 → rx build --target vulkan → SPIR-V → 手写 Vulkan 运行时 → 真 NVIDIA GPU → saxpy 数值精确`,**端到端已跑通**。红线 3 已 owner 解除、RFC-0011 Owner Approved。分支 `mb1/governance-package`（worktree `H:\rurix_mb1`,off origin/main），**14 commits,未 push/未 merge**。剩 4 个工作流待落。

## 1. 已完成（勿重做;先 verify 再动）

| 面 | 条款 | 载体 | 验证命令(本机 NVIDIA RTX 4070 Ti + Vulkan SDK 1.3.296.0) |
|---|---|---|---|
| 治理:红线 3 解除 | D-008/SG-003/RFC-0011 | 13_DECISION_LOG §7+§8 v2.1 · spike_gating SG-003 triggered · rfcs/0011 Owner Approved | `grep triggered registry/spike_gating.json` |
| codegen compute | RXS-0200~0203,0205 | `src/rurixc/src/vulkan_codegen.rs` | `py -3 ci/vulkan_codegen_smoke.py`（6/6 spirv-val vulkan1.0） |
| codegen graphics | RXS-0204 | 复用 `dxil_spirv`（cfg 扩 any(dxil,vulkan)） | 见 conformance/vulkan/accept/vk_{vertex,fragment}.rx |
| 运行时 compute | RXS-0207 | `src/rurix-rt/src/vk.rs`（手写 vulkan-1 FFI,U26） | `py -3 ci/vulkan_device_smoke.py`（saxpy max_err=0 + validation 零报错） |
| Backend trait | RXS-0206 | `src/rurix-rt/src/backend.rs`（零 unsafe） | saxpy 经 `run_job(Vulkan)` 真跑;`cargo test -p rurix-rt --features vulkan --lib` |

**全绿基线**:`trace 192/192` · guardrails/schemas/budget/structure PASS · clippy/fmt clean · **NVIDIA(CUDA)零回归**（dxil-backend 404 / default 318 / vulkan 351 test pass;所有 Vulkan 码 gate 于默认关闭 feature）。

## 2. 剩余 4 工作流（各独立可验、栈式 PR、逐片真实红绿;详见 design/*.md）

### W1 — RXS-0209 artifact 泛化 + RXS-0208 marshalling clause（`design/artifact-gen-lavapipe.md`;加性,低风险,优先）
- **RXS-0209**：`fatbin.rs` `ArtifactKind` 加 `Spirv`;`SmTarget(String)` → `ArchKey{Sm(String),Gfx(String),SpirvPortable}`（prefix-dispatch,现 `strip_prefix("sm_")` 拒 `gfx1100` 是必改点）。ripple 12 点（fatbin.rs 主体 + lib.rs:300-302 + bin/fatbin_saxpy.rs:19,82 + sys.rs 注释）设计已列全。`DeviceArtifactSet` 加性 `spirv_fallback: Option<Vec<u8>>`（不动 NV `ptx_fallback`）。`rurix-pkg/src/lock.rs` 已 format-generic（`kind`/`sm_target` 皆 String）→ **零码改**,仅 doc-comment + roundtrip 测试加 `kind="spirv"/sm_target="gfx1100"`。anchor RXS-0209 = fatbin.rs 测试。
- **RXS-0208**：**诚实两分**——(A) 正文 = vk.rs 已实现的 ordinal→(set,binding)/push-constant marshalling **语义** + 与 codegen RXS-0203 描述符布局单一事实源一致（可 host 测）;(B) 「保 MS1.2 rxrt_launch ABI」= **honest-defer RD-030**（origin/main **无 rxrt_launch**,无回归对象,不假装保兼容）。anchor = vk.rs 新单测 `marshalling_ordinal_matches_codegen_binding`。
- 新 RD：**RD-030**（rxrt_launch ABI 回归,backfill=MS1.2 合入）+ **RD-031**（描述表 v2 blob,backfill=MS1.2 artifacts blob）。均 open。
- 零新 RX 码。trace 192→194。

### W2 — Phase 3 graphics + present（`design/graphics-present.md`;RXS-0210）
- **关键坑（已实证）**：`dxil_spirv` graphics `.spv` 声明 `SPV_GOOGLE_hlsl_functionality1`,`vkCreateShaderModule` 未启用 device ext 时按 VUID-08742 拒（spirv-val 却 accept,故 RXS-0204 codegen 绿但运行时会炸）。**推荐方案 B**：codegen 对 Vulkan target 不 emit UserSemantic/SPV_GOOGLE（`dxil_spirv.rs` `Builder` 加 `emit_provenance: bool`,新 `emit_spirv_body_vulkan(provenance=false)`;`vulkan_codegen.rs:508` 路由改）——去后跨所有 ICD 零扩展依赖,DXIL 路保名字节不变。须在 RXS-0204 修订记录注 provenance erratum。
- **运行时 offscreen-first**（headless 真跑校验,免 swapchain/窗口）：vk.rs 新增 graphics 路径（render pass 单 color attachment CLEAR/STORE + graphics pipeline vertex+fragment + framebuffer + vertex buffer + `vkCmdDraw(3)` + `vkCmdCopyImageToBuffer` 回读像素）。设计已列全部 sType/#[repr(C)] 结构/device 符号/命令序列。graphics `OpEntryPoint` 名恒 `"main"`（pName 硬编 `c"main"`）。unsafe-audit **U27**。
- 验证：全屏三角形 → 64×64 offscreen → 像素断言（覆盖/背景/插值）+ validation 零报错。新 `bin/vk_triangle` + `conformance/vulkan/accept/vk_tri_{vs,fs}.rx` + `ci/vulkan_graphics_smoke.py`（步骤 56,`RURIX_REQUIRE_REAL=1`）。**反证**：临时用带保名的 `emit_spirv_body` 跑 → VUID-08742 红（证坑真实）。
- swapchain 真窗口 present = honest-defer **RD-030**（与 W1 的 RD-030 择一,注意别撞号——W1 用 RD-030/031,则 present 用 **RD-032**）。trace +1。

### W3 — Phase 4 Android 交叉编译（`design/android-cross.md`;RXS-0211）
- **本机无 NDK / 无 aarch64-linux-android target → 交叉构建门本机必 SKIP（dev-env degrade,非 fake）**。达标 = 「交叉 build 绿」（有 NDK 的 runner）+ 平台无关单测绿;设备 on-device = **G-MB1-7 open**。
- **唯一链接期缝**：vk.rs:475-478 `LoadLibraryA`/`GetProcAddress`（其余 ~35 Vulkan 命令全经 `vkGetInstanceProcAddr` 动态解析,零链接期 Vulkan 符号)。抽 `#[cfg(windows)] mod loader`（LoadLibraryA/vulkan-1.dll）/ `#[cfg(not(windows))] mod loader`（dlopen RTLD_NOW/libvulkan.so）——Windows 路径逐字节等价（零漂移）。`extern "system"` 在 aarch64-android == AAPCS64 == `extern "C"`,~35 Fn* 零改。
- `#[cfg(target_os="android")] pub mod android_present`（VkCreateAndroidSurfaceKHR FFI 就位,android 编译绿;compute 路径不启 surface ext)。新 `.cargo/config.toml`（aarch64-linux-android linker=NDK clang;桌面 target 不触）。新 `ci/vulkan_android_build_smoke.py`（步骤 57,**不加 RURIX_REQUIRE_REAL**——NVIDIA runner 无 NDK 应干净 SKIP;专用 android runner 用 `RURIX_REQUIRE_ANDROID=1`)。anchor = vk.rs `loader_seam_selects_platform_lib` 单测。unsafe-audit U26 扩注（不新增 U 号,同 feature 边界）。trace +1。

### W4 — lavapipe 第二 ICD（`design/artifact-gen-lavapipe.md` §3;G-MB1-3 完整,非新条款）
- **本机无软件 ICD**。机制确定：`VK_DRIVER_FILES=<lvp_icd.json>` 覆盖系统 ICD,强制 lavapipe 跑同一 `.spv` 拿跨厂商数值回归。`ci/vulkan_device_smoke.py` 补 lavapipe 分支（ICD 存在→跑第二遍数值对照;不存在→`SKIP: second ICD unavailable`,**非红**）。
- 二进制获取：**lavapipe 首选**（Mesa Windows 打包 `pal1000/mesa-dist-win` GitHub releases 含 `vulkan_lvp.dll`,纯 CPU;或 MSYS2 `mingw-w64-x86_64-mesa`）;SwiftShader 需源码自建（备选）。**获取留 follow-up**,不下载。**无新 RXS/RD**（第二 ICD 是 G-gate/CI 取证义务）。软件 ICD 跑通 ≠ AMD/Android 已验证（G-MB1-6/7 不受影响）。

## 3. 硬性纪律（违反=返工;新 agent 必读）

1. **NVIDIA(CUDA) 零回归**：所有 Vulkan 码 gate 于 feature（rurixc `vulkan-backend` / rurix-rt `vulkan`,均默认关）。每片跑 `cargo build/test -p rurix-rt`（无 feature）+ `cargo test -p rurixc --features dxil-backend --lib`（404）确认零漂移。
2. **LF/CRLF 逐文件保形**：仓库混合。**CRLF 例外文件**（编辑须保 CRLF,用 Python 二进制 I/O 或谨慎 Edit）：`spec/README.md`、`rfcs/README.md`、`13_DECISION_LOG.md`、`registry/*.json`、`conformance/traceability_matrix.json/.md`、`spec/dxil_backend.md`、`ci/dxil_codegen_smoke.py`。**LF 新文件**：`src/**/*.rs`、`spec/vulkan_backend.md`、`rfcs/0011*.md`、`conformance/vulkan/*.rx`、`milestones/mb1/*`、新 `ci/*.py`。提交前逐文件 `py -3 -c "print(open(f,'rb').read().count(b'\\r'))"` 核 CR。**绝不用 Python 文本模式写仓库文件**（写 CRLF）。
3. **真实红绿,退出码判定**（反 Godot：崩溃判退出码非 grep stdout）：codegen 过 `spirv-val --target-env vulkan1.0`;运行时开 `RURIX_VK_VALIDATION=1` 验 `VK_LAYER_KHRONOS_validation` 零报错;缺工具/设备 → SKIP 标 dev-env degrade（非 fake pass）,`RURIX_REQUIRE_REAL=1` 在 GPU runner 翻硬红。
4. **条款先行**（硬规则 7）：每 `### RXS-####` 条款体 + 每条 ≥1 `//@ spec: RXS-####` 锚定（`src/**/*.rs` 或 `conformance/**/*.rx` 或 `tests/ui/**/*.rx`——**.py 不被 trace 扫描**)同 PR;`py -3 ci/trace_matrix.py`（regen,Windows text-mode 写 CRLF json 保形）+ `--check` 全锚定 N/N。
5. **spec 修订表**：spec 文件每次改追加 `| 版本 | 日期 | 变更 | 档位 |` 新行（既有行 0-byte;`check_guardrails` 凭字面 `版本` 跳表头,忌 `版号/版次`）。
6. **不自翻治理**：红线 3 已解除,RFC-0011 已 Owner Approved——**勿重做**。新 RD 走 `registry/deferred.json` 追加（append-only,尾换行）。**未获 owner 明确 "push/merge" 前不 push 到公开 main**（现全部本地）。
7. **guardrails 陷阱**：`ci/check_guardrails.py` 取**位置参数** base（`py -3 ci/check_guardrails.py origin/main`,非 `--base`);现为 ADVISORY 不阻断,但仍逐项核。环境用 `py -3`。

## 4. 编号 next-free（避撞 origin/main + MS1.2b/MS1 在途 claim）
- **RXS**：0206/0207 已落;0208~0213 预留区间已在 spec §1 登记,落体即用（跳 0189~0199=MS1.2/MS1.2b）。
- **RD**：下一可用 **RD-030**（W1 用 030/031;W2 present 用 032——**别撞**）。跳 RD-027/028（MS1 规划）、029（mesh/task/RT,mb1）。
- **U**：下一 **U27**（W2 graphics FFI;U23 空号、U25=MS1.2b、U26=vk compute 已用）。
- **RX 错误码**：W1~W4 **均不新增 RX 码**（运行期失败=工具层确定性 Err,不占码;6xxx 不预造）。
- **CI 步骤**：54=vulkan_codegen、55=vulkan_device 已用;W2=**56**、W3=**57**（避 52/53=MS1.2/MS1.3 在途）。
- **RFC**：next-free RFC-0012。

## 5. 建议实施序 + 每片验证
`W1（加性,最低风险）→ W2（present,最大工程量,须先处 SPV_GOOGLE 坑）→ W3（Android,本机 SKIP 达标）→ W4（lavapipe,接线+SKIP,获取 follow-up）`。每片:改码 → `cargo clippy --features <feat> -- 无警告` + `cargo fmt` → 真实红绿（spirv-val / GPU run / validation）→ `trace --check N/N` → host 四门 → 逐文件核字节 → 单片 commit（LF 新文件/CRLF 保形;provenance `Assisted-by: claude-code:claude-opus-4-8` + `Co-Authored-By: Claude Opus 4.8`;治理签署 commit 例外=仅 Assisted-by 无 Co-Author）。

## 6. 两道硬件尾门（维持 open,勿伪造）
- **G-MB1-6 AMD 真卡**：AMD 桌面 GPU（gfxNNNN）真跑 compute+graphics 数值/像素对照 + validation 零报错。缺硬件 open。
- **G-MB1-7 Android 真机**：arm64 设备 libvulkan.so 真跑 compute + ANativeWindow present。缺设备 open（pending-hardware）。
- **NVIDIA(+lavapipe) 跑通 ≠ AMD/Android 已验证**——DoD 写清,不伪造 device 绿、不签。

## 7. 指针
- 完整设计：`milestones/mb1/design/{backend-trait,graphics-present,android-cross,artifact-gen-lavapipe}.md`（file:line 级,含精确改动清单/代码骨架/验证命令）。
- RFC/条款：`rfcs/0011-vulkan-spirv-backend.md`、`spec/vulkan_backend.md`。
- 治理裁决草案（已应用）：`milestones/mb1/OWNER_DECISION_PACKAGE.md`。
- 契约/进度：`milestones/mb1/{MB1_CONTRACT,MB1_PLAN,CI_GATES}.md`。
