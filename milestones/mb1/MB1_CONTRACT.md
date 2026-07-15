---
# 里程碑契约(14 §1 四要素;mb1 = 多后端新纪元首期,承 TEMPLATE_CONTRACT.md 体例)
# 状态:草案——gated on owner 裁决红线 3(D-008/SG-003)解除 + RFC-0011 批准;未获裁决前不合入 main、不激活。
contract: MB1
title: MB1 期——多后端新纪元第一期:单一 Vulkan/SPIR-V 跨端后端(AMD 桌面 + Android;compute + graphics)
status: active            # active → closed(close-out 只追加 §8,上方条款 0-byte);**激活 gated on 红线 3 解除 + RFC-0011 批准(§0/§7)**
version: v1.0
date: 2026-07-15
timebox: "多后端新纪元第一期(约 4–6 周,MB1.0~MB1.4 严格串行见 MB1_PLAN.md;周为相对刻度,非日历承诺)"
rfc_required: RFC-0011    # 全 in_scope 实现面 Full-RFC-gated(RFC-0011,Draft);**RFC-0011 批准 + 红线 3(D-008)解除 + SG-003 triggered 三者为 owner 裁决前置**,合入先于任何 mb1 实现 PR(10 §3 / 硬规则 7)
upstream_docs:
  - "11 §5 (多后端解禁评估——红线 3 的正式重审)"
  - "11 §2 (红线 3:多后端 MVP 不做,spike gating SG-003)"
  - "03 §4 (死亡路线红线:WGSL/wgpu/SYCL/HIP 跨平台优先教训)"
  - "13 §7 (D-008 多后端红线解除待决点)"
  - "rfcs/0011-vulkan-spirv-backend.md (设计面)"
  - "spec/vulkan_backend.md (条款先行,RXS-0200~0213)"
in_scope:
  - vulkan_spirv_codegen        # MB1.1 一条 MIR→SPIR-V codegen(rx build --target vulkan):compute(GLCompute/LocalSize/builtins/存储缓冲/结构化控制流)+ graphics(vertex/fragment 复用 dxil_spirv.rs 种子)+ 数学 intrinsic→GLSL.std.450;每 .spv 过 spirv-val;feature vulkan-backend(default off);RXS-0200~0205/0212/0213
  - vulkan_compute_runtime       # MB1.2 一个 Vulkan 运行时后端:rurix-rt Backend trait 抽象(CUDA 收敛为并列实现,NVIDIA 零回归)+ Vulkan compute(instance/device/pipeline/descriptor/dispatch)+ launch marshalling(保 MS1.2 ABI)+ artifact 泛化(Spirv+gfx);本机 NVIDIA + lavapipe 双 ICD 真跑;RXS-0206~0209
  - vulkan_graphics_present       # MB1.3 Vulkan graphics + present:render pass/graphics pipeline/swapchain/present;uc03/uc04 等价验收;本机 NVIDIA 真跑;RXS-0210
  - android_cross_build           # MB1.4 Android 交叉编译移植缝:dlopen libvulkan.so + aarch64-linux-android + NDK + ANativeWindow;交叉构建绿 + 平台无关单测绿;设备运行 pending-hardware;RXS-0211
out_of_scope:
  - amd_realcard_acceptance       # AMD 真卡最终验收红绿:缺硬件,open 尾门 G-MB1-6 + DoD(NVIDIA-Vulkan 跑通 ≠ AMD 已验证)
  - android_ondevice_smoke        # Android 真机 on-device smoke:缺设备,open 尾门 G-MB1-7 + DoD(交叉构建绿,设备运行 pending-hardware)
  - metal_ios_d3d12_new_route     # Metal / iOS / D3D12 新路:owner 范围锁定不触
  - mesh_task_rt_stages           # mesh/task/RT 着色阶段:honest-defer RD-029(compute+graphics 首期)
  - portable_abstraction_layer    # 通用可移植抽象层 / 地址空间推断 / 隐式多目标 fallback:红线 3 底层关切,永不做(RFC-0011 §7/§8)
deferred_refs: [RD-029]
deliverables:
  - id: D-MB1-1
    name: 治理包——RFC-0011(Full RFC) + spec/vulkan_backend.md 脚手架 + 红线 3(D-008)解除 errata 草案 + SG-003→triggered 草案 + mb1 四件套(owner 裁决前置,不自签/不自翻)
  - id: D-MB1-2
    name: MB1.1 MIR→SPIR-V codegen——rx build --target vulkan(compute+graphics)+ 数学 intrinsic→GLSL.std.450 + spirv-val golden + ci/vulkan_codegen_smoke.py;RXS-0200~0205/0212/0213
  - id: D-MB1-3
    name: MB1.2 Vulkan compute 运行时——Backend trait(NVIDIA 零回归)+ Vulkan compute + marshalling + artifact 泛化;本机 NVIDIA + lavapipe 双 ICD 真跑;RXS-0206~0209
  - id: D-MB1-4
    name: MB1.3 Vulkan graphics + present——render pass/pipeline/swapchain/present;本机 NVIDIA 出图+present 真跑;RXS-0210
  - id: D-MB1-5
    name: MB1.4 Android 交叉编译——dlopen/libvulkan/NDK/ANativeWindow;android-arm64 交叉构建绿 + 平台无关单测绿;设备 pending-hardware;RXS-0211
acceptance_gates:
  - id: G-MB1-1
    check: "治理闸口(owner 裁决):① D-008 红线 3 解除(独立 errata PR,13_DECISION_LOG 追加式勘误 00 §6.3)② SG-003 current_verdict→triggered(RFC-0011)(独立 registry PR,append decisions[])③ RFC-0011 status Draft→Approved。三者均 owner 主动决策(10 §9.2);SG-003 现存记录判定前提『NVIDIA 单栈纵深完成』未达(2026-07-14),agent 不自行宣布达成、不自签。三者合入先于任何 mb1 实现 PR。证据 = OWNER_DECISION_PACKAGE.md 三改动经 owner 裁决落地。"
  - id: G-MB1-2
    check: "MB1.1 SPIR-V codegen 真实红绿:rx build --target vulkan 对 compute + vertex + fragment 语料产 spirv-val-clean .spv(退出码 0);篡改 .spv 字节 → spirv-val 拒(红),复原绿;conformance SPIR-V golden bless(compute+vs+fs);ci/vulkan_codegen_smoke.py 内建 red_self_test + 缺工具 SKIP(非 fake pass);host 四门(guardrails/budget/contribution/LF)+ trace N/N 全绿;数学 intrinsic→GLSL.std.450 映射单测绿。"
  - id: G-MB1-3
    check: "MB1.2 Vulkan compute 运行时真实红绿:本机 NVIDIA(RTX 4070 Ti)-Vulkan compute 端到端真跑(saxpy/reduce 等价 UC)数值对照绿(measured);**第二 ICD**(lavapipe/SwiftShader)CI 红绿(跨厂商回归;若 ICD 不可得则标 dev-env degrade + DoD,不 fake);全程 VK_LAYER_KHRONOS_validation 零报错;Backend trait 引入后 CUDA(PTX/cubin/DXIL)既有路回归网零漂移(NVIDIA 零回归硬约束);host 门 + trace 全绿。run URL 归档 §8。"
  - id: G-MB1-4
    check: "MB1.3 Vulkan graphics + present 真实红绿:本机 NVIDIA 出图 + present 真跑,像素/截图对照归档(measured);validation layer 零报错;uc03/uc04 等价验收;host 门 + trace 全绿。run URL 归档 §8。"
  - id: G-MB1-5
    check: "MB1.4 Android 交叉构建绿:aarch64-linux-android + NDK 交叉编译 rurix-rt(Vulkan 后端)+ Android surface present 代码就位构建绿;平台无关单元/逻辑测试绿;NDK 缺失 → SKIP 标 dev-env degrade(非 fake)。**设备 on-device 运行不在本门**(→ G-MB1-7 open 尾门)。"
  - id: G-MB1-6
    check: "【OPEN 尾门 — 缺 AMD 硬件,不签】AMD 真卡 Vulkan compute + graphics 验收红绿。DoD:一台 AMD 桌面 GPU(gfxNNNN,如 RX 7000/gfx1100)上 rx build --target vulkan 产物经 AMD Vulkan 驱动装载真跑 compute(数值对照)+ graphics(出图/present 像素对照),VK_LAYER_KHRONOS_validation 零报错,run URL + 环境画像(deviceName/driverInfo/gfx 架构键)归档。**NVIDIA-Vulkan 跑通不充作 AMD 已验证**;本门在获得 AMD 硬件前维持 open,不伪造、不签。"
  - id: G-MB1-7
    check: "【OPEN 尾门 — 缺 Android 设备,不签】Android 真机 on-device smoke。DoD:一台 arm64 Android 设备(含 libvulkan.so)上装载 android-arm64 交叉产物,ANativeWindow present 真跑 N 帧 + compute 数值对照,VK_LAYER_KHRONOS_validation 零报错,logcat + run 证据归档。**交叉构建绿(G-MB1-5)不充作设备已验证**;本门在获得 Android 设备前维持 open(pending-hardware),不伪造、不签。"
guardrails:
  - "条款先行(硬规则 7):RFC-0011 Approved + 红线 3 解除 errata + SG-003 triggered 合入先于任何 mb1 实现 PR;spec/vulkan_backend.md RXS-0200 续号条款体(FLS 体例,严禁 UB 节)与每条 ≥1 //@ spec 锚定同 PR 落地,commit 序条款在前;trace_matrix --check 维持全锚定。"
  - "NVIDIA 零回归(硬约束):Backend trait 抽象引入后,CUDA(PTX/cubin)+ DXIL 既有路功能字节/行为零漂移;既有 rurix-rt / rurixc 回归网全绿方可合。vulkan-backend feature default off,未启用时编译器/运行时零 bifurcate。"
  - "字节洁净:新文件(RFC-0011/vulkan_backend.md/milestones/mb1/*/src 新增 crate)LF 字节洁净;既有 CRLF 例外文件(spec/README.md/rfcs/README.md/13_DECISION_LOG.md/spike_gating.json)编辑保 CRLF、既有字节 0-byte;每文件提交前逐个核 CR 与尾字节(git numstat + Python 字节读,不用 grep $'\\r')。"
  - "两道硬件尾门(G-MB1-6 AMD 真卡 / G-MB1-7 Android 真机)标 open + 写 DoD,缺硬件不伪造 device 绿、不签;NVIDIA(+lavapipe)跑通不充作 AMD/Android 已验证(反 Godot 退出码/grep 教训)。"
  - "fail-closed:缺工具(glslang/spirv-val/NDK/lavapipe)→ SKIP 标 dev-env degrade 或 pending-hardware,绝不 fake success;RURIX_REQUIRE_REAL=1 在 GPU runner 翻硬红。"
  - "13_DECISION_LOG.md + registry/spike_gating.json + error_codes.json + deferred.json 只追加/字节冻结(check_guardrails);D-008 解除只经独立 errata PR(00 §6.3),SG-003 只 append decisions[]、trigger_condition 0-byte;新码只追加 en/zh 成对(bilingual_coverage)。"
---

# MB1 契约 — 多后端新纪元第一期:单一 Vulkan/SPIR-V 跨端后端

> 所属:[../../11_ROADMAP.md](../../11_ROADMAP.md) §5(多后端解禁评估——红线 3 正式重审)/ 契约机制见 [../../14_ENGINEERING_DISCIPLINE.md](../../14_ENGINEERING_DISCIPLINE.md) §1
> **档位定位:MB1 是多后端新纪元(red line 3 方向)的第一期,正面触死亡路线红线 3——激活 gated on owner 裁决(§0)。**

---

## 0. 治理闸口(读在最前 — 本里程碑区别于既往里程碑之处)

MB1 方向 = **Vulkan/SPIR-V 跨端后端**(AMD 桌面 + Android),**正面触死亡路线红线 3**(多后端 AMD/Intel/Metal/Vulkan/SPIR-V;D-008/SG-003)。这不是常规里程碑:

- **红线 3 解除是 owner 主动决策**(10 §9.2,一次一条)。项目自己的记录(SG-003,最近 2026-07-14)判定其前提『NVIDIA 单栈纵深完成』**未达**;`03 §4`/`11 §2` 将 WGSL/wgpu/SYCL/HIP 的「跨平台优先牺牲性能/能力/provenance」列为死亡路线。
- 与仓库默认治理(D-406 v2.0 agent 完全自主)不同,**本任务由 owner 明确保留红线 3 解除、SG-003 触发、RFC-0011 批准、milestone 激活为 owner 裁决闸口**;agent 起草并把待裁摊清,**默认不自签、不自翻**。
- 三项 owner 裁决前置(G-MB1-1)+ 精确改动草案见 [OWNER_DECISION_PACKAGE.md](OWNER_DECISION_PACKAGE.md)。**未获裁决前 mb1 实现 PR 不合入 main。**

## 1. 目标

MB1 期结束时项目获得:① 一条 `rx build --target vulkan` MIR→SPIR-V codegen,compute + graphics(vertex/fragment) 皆产 spirv-val-clean 的 `.spv`;② 一个 rurix-rt Vulkan 运行时后端(经新 `Backend` trait,CUDA 收敛为并列实现、NVIDIA 零回归),compute 与 graphics+present 在本机 NVIDIA(+lavapipe 第二 ICD)真实红绿、validation layer 零报错;③ 同一份 `.spv` 经 AMD 桌面驱动 / Android `libvulkan.so` 装载的移植缝就位,android-arm64 交叉构建绿——**用一条 codegen + 一个后端同覆盖 AMD 桌面与 Android**。两道硬件尾门(AMD 真卡验收 / Android 真机 smoke)明确 open,缺硬件不伪造。**不触通用可移植抽象层(红线 3 底层关切),不回归 NVIDIA 既有路。**

## 2. 范围

### 2.1 in-scope

| 项 | 说明 | 对应交付物 |
|---|---|---|
| vulkan_spirv_codegen | MB1.1 一条 MIR→SPIR-V codegen(`--target vulkan`):compute(GLCompute/LocalSize/builtins/存储缓冲/结构化控制流子集)+ graphics(vertex/fragment 复用 `dxil_spirv.rs`)+ 数学 intrinsic→GLSL.std.450;每 `.spv` 过 spirv-val;feature `vulkan-backend`(default off) | D-MB1-2 |
| vulkan_compute_runtime | MB1.2 一个 Vulkan 运行时后端:`Backend` trait 抽象(CUDA 收敛,NVIDIA 零回归)+ Vulkan compute + launch marshalling(保 MS1.2 ABI)+ artifact 泛化(Spirv+gfx);本机 NVIDIA + lavapipe 双 ICD 真跑 | D-MB1-3 |
| vulkan_graphics_present | MB1.3 Vulkan graphics + present:render pass/pipeline/swapchain/present;uc03/uc04 等价验收;本机 NVIDIA 真跑 | D-MB1-4 |
| android_cross_build | MB1.4 Android 交叉编译移植缝:dlopen `libvulkan.so` + `aarch64-linux-android` + NDK + `ANativeWindow`;交叉构建绿 + 平台无关单测绿;设备 pending-hardware | D-MB1-5 |

### 2.2 out-of-scope(显式排除)

- **AMD 真卡最终验收红绿** → open 尾门 G-MB1-6 + DoD(缺硬件;NVIDIA-Vulkan 跑通 ≠ AMD 已验证)。
- **Android 真机 on-device smoke** → open 尾门 G-MB1-7 + DoD(缺设备;交叉构建绿 ≠ 设备已验证,pending-hardware)。
- **Metal / iOS / D3D12 新路** → owner 范围锁定不触。
- **mesh / task / RT 着色阶段** → honest-defer **RD-029**(compute+graphics 首期;随需按 10 §3 判档)。
- **通用可移植抽象层 / 地址空间推断 / 隐式多目标 fallback** → 红线 3 底层关切,永不做(RFC-0011 §7/§8;本后端 explicit、单目标 per-build、无地址空间推断)。

## 3. 交付物清单

| ID | 交付物 | 形态 | 完成判据 |
|---|---|---|---|
| D-MB1-1 | 治理包 | RFC-0011(Full RFC Draft)+ spec/vulkan_backend.md 脚手架 + D-008 解除 errata 草案 + SG-003→triggered 草案 + mb1 四件套 + OWNER_DECISION_PACKAGE.md | owner 裁决前置摊清;agent 不自签/不自翻(G-MB1-1) |
| D-MB1-2 | MB1.1 SPIR-V codegen | `vulkan_codegen.rs`(泛化 `dxil_spirv.rs`)+ `--target vulkan` + 数学 intrinsic 映射 + conformance golden + `ci/vulkan_codegen_smoke.py` | spirv-val-clean + golden bless + host 门绿(G-MB1-2) |
| D-MB1-3 | MB1.2 Vulkan compute 运行时 | rurix-rt `Backend` trait + Vulkan compute 后端 + marshalling + artifact 泛化 + `ci/vulkan_device_smoke.py` | NV + lavapipe 双 ICD 真绿 + validation 零报错 + NVIDIA 零回归(G-MB1-3) |
| D-MB1-4 | MB1.3 graphics + present | render pass/pipeline/swapchain/present + `ci/vulkan_present_smoke.py` | NV 出图/present 真绿 + 像素对照(G-MB1-4) |
| D-MB1-5 | MB1.4 Android 交叉编译 | OS 移植缝(dlopen/调用约定/target)+ NDK 交叉 + ANativeWindow present | android-arm64 交叉构建绿 + 平台无关单测绿(G-MB1-5) |

## 4. 验收门(完整版,YAML 头为可提取摘要)

见 YAML 头 `acceptance_gates`。要点:

- **G-MB1-1 治理闸口**:三项 owner 裁决(D-008 解除 / SG-003 triggered / RFC-0011 Approved)。**owner 主动决策,agent 不自签**;前提诚实——SG-003 记录判定『NVIDIA 纵深完成』未达。
- **G-MB1-2 ~ G-MB1-4**:本机 NVIDIA(+lavapipe)**真实红绿**(measured,反 YAML-only)——codegen 层 spirv-val + golden;运行时层数值/像素对照 + validation 零报错;NVIDIA 零回归。证据等级 measured_local,run URL 归 §8。
- **G-MB1-5**:android-arm64 交叉**构建**绿 + 平台无关单测绿(设备运行不在本门)。
- **G-MB1-6 / G-MB1-7 = 两道 OPEN 硬件尾门**:AMD 真卡 / Android 真机。缺硬件维持 open,**不伪造 device 绿、不签**;DoD 写清(见 YAML)。NVIDIA(+lavapipe)跑通不充作 AMD/Android 已验证(反 Godot「退出码非 grep stdout」教训)。

## 5. Guardrails(字节级,机器核对)

见 YAML 头 `guardrails` 字段。核对方式:`ci/check_guardrails.py <上一里程碑 close tag>`。要点:条款先行 / NVIDIA 零回归 / 字节洁净(LF 新文件 + CRLF 既有例外保形)/ 两道硬件尾门不伪造 / fail-closed / 治理文件只追加。

## 6. Deferred 引用

| 编号 | 内容摘要 | 承接 |
|---|---|---|
| RD-029 | mesh/task/RT 着色阶段 Vulkan/SPIR-V 降级(compute+graphics 首期外) | MB1 |

详情以 [../../registry/deferred.json](../../registry/deferred.json) 为唯一事实源,本表仅引用。**RD-029 登记随 mb1 首个实现 PR 落地(执行期登记,跳 RD-027/028 = MS1 规划占用)。**

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-07-15 | 初版契约(mb1 多后端新纪元第一期草案):四要素固化——in_scope(Vulkan/SPIR-V codegen + compute 运行时 + graphics/present + Android 交叉)/ out_of_scope(AMD 真卡·Android 真机尾门 + Metal/iOS/D3D12 + mesh/task/RT RD-029 + 通用抽象层)/ deliverables D-MB1-1~5 / acceptance_gates G-MB1-1~7(含两道 open 硬件尾门 G-MB1-6/7)/ guardrails。**§0 治理闸口:激活 gated on owner 裁决红线 3(D-008/SG-003)解除 + RFC-0011 批准;agent 不自签/不自翻(OWNER_DECISION_PACKAGE.md)。** 承 RFC-0011(Draft)/ spec/vulkan_backend.md 脚手架 / 11 §5 多后端解禁评估。 |

---

## 8. Close-out(只追加区 — 开工时为空)

<!-- 验收记录、run URL、guardrail 核对输出、device 真实红绿证据、deferred 继承/关闭记录追加于此;上方条款 0-byte 修改。两道硬件尾门(G-MB1-6/7)在获得 AMD 真卡 / Android 设备前维持 open,不在此伪造 device 绿。 -->
