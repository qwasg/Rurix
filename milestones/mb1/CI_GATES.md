# MB1 CI Gates — Vulkan/SPIR-V 跨端后端(多后端新纪元第一期)

> 所属:[MB1_CONTRACT.md](MB1_CONTRACT.md) 验收门 / [../../14_ENGINEERING_DISCIPLINE.md](../../14_ENGINEERING_DISCIPLINE.md) §2。CI 步骤全在 `.github/workflows/pr-smoke.yml`(self-hosted Windows GPU runner,串行 gpu-runner)。**全部 gated on MB1.0 治理闸口(RFC-0011 批准 + 红线 3 解除);实现 PR 合入先于闸口无效。**

---

## §1 既有门复用(零改动)

mb1 复用既有 host 四门 + trace,零改动即纳入:
- `ci/check_guardrails.py`(字节级 + 修订表档位标记 + 治理文件只追加/字节冻结)
- `ci/budget_eval.py`(预算 glob `*_budget.json` 已泛化——`mb1_budget.json` 自动纳入,零 CI 码改动)
- `ci/check_contribution.py`(provenance / 条款号 / 验证)
- `ci/trace_matrix.py --check`(每 `### RXS-####` ≥1 `//@ spec` 锚定,全锚定 N/N)
- LF/`.gitattributes` 字节洁净(新文件 LF;既有 CRLF 例外保形)

## §2 新增 mb1 冒烟步骤(随各 Phase 实现 PR 落地)

步骤号待分配——**避 MS1.2 步骤 52 / MS1.3 步骤 53 占用(feat/ms1.2b 在途),mb1 自步骤 54 起续号**;各步骤仿既有 `ci/dxil_codegen_smoke.py` / `ci/host_orch_smoke.py` 体例(locate 工具 env-first、缺工具 SKIP exit-0 非 fake、内建 `red_self_test`、`_report` 退出码纪律、退出码判定非 grep stdout)。

| 步骤(拟) | 脚本 | 面 | 门 | REQUIRE_REAL |
|---|---|---|---|---|
| §2.54 | `ci/vulkan_codegen_smoke.py` | MB1.1 codegen:`--target vulkan` 产 `.spv` → `spirv-val` clean;篡改红/复原绿;确定性 ×N;数学 intrinsic 映射;缺 glslang/spirv-val → SKIP dev-env degrade | G-MB1-2 | 否(host/CPU,工具在位即真跑) |
| §2.55 | `ci/vulkan_device_smoke.py` | MB1.2 compute 运行时:本机 NVIDIA-Vulkan compute 端到端数值对照 + **lavapipe 第二 ICD** 跨厂商回归 + `VK_LAYER_KHRONOS_validation` 零报错 + CUDA 零回归探针;缺 Vulkan 运行时 → SKIP | G-MB1-3 | 是(GPU runner 硬红) |
| §2.56 | `ci/vulkan_graphics_smoke.py` | MB1 Phase 3 graphics(offscreen-first):本机 NVIDIA offscreen 三角形出图 → `vkCmdCopyImageToBuffer` readback 像素对照(背景角==clear / 中心覆盖非背景 / covered>0)+ `VK_LAYER_KHRONOS_validation` 零报错(messenger fail-closed)+ codegen 去 `SPV_GOOGLE`/`UserSemantic` 反证(spirv-dis grep 0 + spirv-val accept)+ 内建 `red_self_test`(provenance-带保名 `.spv` 喂同管线 → VUID-...-08742 → 退出码判红);窗口/swapchain present → RD-032 尾门;缺 Vulkan 设备 → SKIP dev-env degrade | G-MB1-4 | 是(GPU runner 硬红) |
| §2.57 | `ci/vulkan_android_build_smoke.py` | MB1.4:aarch64-linux-android + NDK 交叉**构建**绿 + 平台无关单测;NDK 缺失 → SKIP dev-env degrade;**设备运行不在本门** | G-MB1-5 | 否(交叉构建;设备 → G-MB1-7 open) |

## §3 两道硬件尾门(不设 CI 步骤,open)

- **G-MB1-6 AMD 真卡验收** / **G-MB1-7 Android 真机 on-device smoke**:缺硬件,**不设 CI 硬门**(镜像 `realtime_present_smoke` 双态先例,SKIP 不充绿);DoD 见契约 §4。获得硬件后按 DoD 补 evidence + run URL,不伪造 device 绿、不签。

## §4 真实红绿纪律(反 YAML-only)

- 每个 mb1 冒烟内建 `red_self_test`(合成红/绿输入,helper 分不出即整体 FAIL);篡改 `.spv`/buffer 字节 → 拒(红),复原 → 绿。
- 退出码判定(非 grep stdout);`VK_LAYER_KHRONOS_validation` 全程开、零报错方绿(Godot 教训)。
- 缺工具/硬件 → SKIP 标 dev-env degrade 或 pending-hardware,`RURIX_REQUIRE_REAL=1` 在 GPU runner 翻硬红;**绝不 fake success**。
- **NVIDIA 零回归**:mb1 各 PR 跑既有 CUDA(PTX/cubin)+ DXIL 回归网,零漂移方可合。
