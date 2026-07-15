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

### G-MB1-1 治理闸口 — 签署（owner 白栀,2026-07-15）

owner（白栀）于本工作会话**明确指示「把多端红线解除并继续工作」**（10 §9.2 owner 主动决策,非 close-out 自动触发）。三项 owner 裁决前置满足:
- ① **D-008 红线 3 解除**：[13_DECISION_LOG.md](../../13_DECISION_LOG.md) §7 D-008 行 resolved + §8 errata v2.1。
- ② **SG-003 → triggered(RFC-0011)**：[registry/spike_gating.json](../../registry/spike_gating.json) current_verdict 翻转 + decisions 追加 + revision_log v1.5（append-only,trigger_condition 0-byte）。
- ③ **RFC-0011 Owner Approved**：[rfcs/0011-vulkan-spirv-backend.md](../../rfcs/0011-vulkan-spirv-backend.md) 状态 / Agent 批准 / §9 Q-Redline 定案 / 修订记录 Owner approval。
- **MB1 激活 + 索引登记**：spec/README §4+§5 v1.54 / rfcs/README 台账（RFC-0011 Owner Approved,next-free RFC-0012）。

**诚实留痕**：解除前提『NVIDIA 单栈纵深完成』先前(2026-07-14 SG-003)判定未达,本次为 **owner 主动裁决解除**（其 prerogative,10 §9.2),非 agent 宣布前提达成。agent 依 owner 明确授权代录机器事实,非自签。

### 进度记录（Phase 1 + Phase 2 core,真实红绿）

- **Phase 1 codegen（G-MB1-2）达成**：`rx build --target vulkan` 产 **spirv-val --target-env vulkan1.0 clean** SPIR-V——compute（walking skeleton → saxpy 规范 UC → 数学 intrinsic GLSL.std.450）+ graphics（vertex/fragment 复用 dxil_spirv）；RXS-0200~0205；`ci/vulkan_codegen_smoke.py` 步骤 54（6 语料 6/6）。零回归（dxil 404/default 318/vulkan 351 test pass）。
- **Phase 2 core 运行时（G-MB1-3 部分）达成**：手写 `vulkan-1` FFI（`src/rurix-rt/src/vk.rs`,feature vulkan 默认关闭,unsafe-audit U26）——**本机 NVIDIA RTX 4070 Ti(Vulkan 1.4.351) 真跑 saxpy = a*x + out 数值精确 max_err=0（n=1024,a=2）+ VK_LAYER_KHRONOS_validation 零报错**（反证:错入口名触发 VUID,证 layer 生效）；RXS-0207；`ci/vulkan_device_smoke.py` 步骤 55；`bin/vk_saxpy` demo。NVIDIA(CUDA) 零回归。
- **余待**：RXS-0206 Backend trait（CUDA 收敛）/ RXS-0208~0213 / lavapipe 第二 ICD（G-MB1-3 完整）/ Phase 3 graphics+present（G-MB1-4）/ Phase 4 Android 交叉构建（G-MB1-5）。
- **两道硬件尾门维持 open**：G-MB1-6（AMD 真卡）/ G-MB1-7（Android 真机 on-device）——缺硬件,NVIDIA(+lavapipe) 跑通不充作已验证,不伪造、不签。

### 进度记录(续作 W1~W4:artifact/marshalling + graphics + Android + 2nd-ICD,真实红绿 — 2026-07-15)

续作 4 工作流栈式落地(off `mb1(handoff)` e63773fa;分支 `mb1/governance-package`,**本地未 push / 未 merge**)。commit 栈:W1 `e3d5f822` → chore(fmt) `2d6064ef` → W2 `d13f6e76` → W3 `795156ed` → W4 `cd12abf2`。

- **W1 — RXS-0208 marshalling + RXS-0209 artifact 泛化(`e3d5f822`)**:`fatbin.rs` `ArtifactKind+=Spirv` / `SmTarget→ArchKey{Sm/Gfx/SpirvPortable}` prefix-dispatch / `DeviceArtifactSet.spirv_fallback`(NV cubin/ptx 逐字节不变);`lock.rs` format-generic doc-comment + roundtrip;marshalling anchor 解析 build.rs 经 vulkan_codegen 产的**真** saxpy `.spv` 的 Binding/Offset 装饰(单一事实源,非内联复刻)。**RD-029**(mesh/task/RT,§6 首个实现 PR mandated)+ **RD-030**(rxrt_launch ABI honest-defer)+ **RD-031**(描述表 v2 blob honest-defer)。
- **W2 — RXS-0210 graphics + offscreen present(`d13f6e76`)**:Scheme B codegen `emit_spirv_body_vulkan`(provenance=false)去 SPV_GOOGLE/UserSemantic 修 VUID-08742,**DXIL 路 A/B sha256 逐字节相等 + `dxil-backend --lib` 恒 404**(byte-identity 双证);`vk.rs` `run_graphics_offscreen`(render pass + graphics pipeline vtx+frag + framebuffer + `vkCmdDraw(3)` + `vkCmdCopyImageToBuffer`,**U27**)+ `VK_EXT_debug_utils` messenger(仅 `RURIX_VK_VALIDATION=1`,fail-closed L3,创建后无 `?` 早退→无泄漏);`bin/vk_triangle` + `conformance/vulkan/accept/vk_tri_{vs,fs}.rx`;`ci/vulkan_graphics_smoke.py` 步骤 56(`RURIX_REQUIRE_REAL=1`)含 red_self_test 反证。**RD-032**(present/swapchain honest-defer)。**G-MB1-4 达成**(offscreen device 真绿)。
- **W3 — RXS-0211 Android 交叉构建(`795156ed`)**:`vk.rs` loader cfg 分叉(`#[cfg(windows)]` vulkan-1.dll/LoadLibraryA · `#[cfg(not(windows))]` libvulkan.so/dlopen RTLD_NOW,**Windows 逐字节零漂移**,compute+graphics 两冒烟复验)+ `#[cfg(target_os="android")] android_present`(vkCreateAndroidSurfaceKHR FFI);**U26 扩注**(无新 U 号);`.cargo/config.toml` + `ci/vulkan_android_build_smoke.py` 步骤 57(两阶段:host 平台无关单测 21 恒跑 + NDK 交叉构建 SKIP@缺-NDK;**无 RURIX_REQUIRE_REAL**)。**G-MB1-5 达成**(交叉构建绿@NDK / 本机 SKIP + 平台无关单测绿)。
- **W4 — lavapipe 第二 ICD 接线(`cd12abf2`,无新条款)**:`ci/vulkan_device_smoke.py`(步骤 55)加性第二 ICD 阶段——primary NVIDIA(系统 ICD,env 副本剥离 ambient `VK_DRIVER_FILES`/`VK_ICD_FILENAMES` 保 honest,反证=ambient bogus 路径下 primary 仍绿)PASS 后经 `RURIX_VK_LAVAPIPE_ICD` 发现软件 ICD → 仅 subprocess env 副本注入跑同一 `.spv`,断言 `out[*]`+`max_err` 跨厂商一致 + validation 静默;缺 ICD → SKIP(dev-env degrade,**绝不下载**)。

**真实红绿证据(本机 NVIDIA RTX 4070 Ti + Vulkan SDK 1.3.296.0,退出码判定):**
- CI 门:步骤 54 codegen 8/8 spirv-val vulkan1.0 · 步骤 55 device saxpy `max_err=0.00e0` + 2nd-ICD SKIP · 步骤 56 graphics offscreen 出图像素校验 + validation 静默 + red_self_test VUID-08742 反证 · 步骤 57 android host 单测 21 PASS + cross-build SKIP。
- 零回归:`rurixc dxil-backend 404`(byte-identity 不变量,W2 后恒定)/ `rurixc default 318` / `rurixc vulkan-backend 353` / `rurix-rt default 17` / `rurix-rt vulkan 21` / `rurix-pkg 35`(均 grow-only);`cargo fmt --all --check` clean;`cargo clippy -D warnings` clean。
- trace `192→196`(RXS-0208/0209/0210/0211 各 ≥1 `//@ spec` 锚定于 `.rs`/`.rx`;`--check` 196/196)。host 门 schemas/structure/budget(69 pass,0 skip)PASS;guardrails per-slice `HEAD` base 空 findings。
- 编号:RXS-0208~0211 / RD-029~032 / U27 + U26 扩 / CI 步骤 56·57 + 55 augment / **零新 RX 码 / 零新 RFC**。跳 RD-026/027/028(MS1 活分支占用,避撞)。
- 每片 5 镜头(W4 为 3 镜头)对抗 review:W1 0 阻断;W2 1 advisory 已修(messenger 泄漏窗 + U27 措辞);W3 1 advisory 已修(§2.57 平台无关单测接线);W4 2 advisory 已修(primary ICD 污染 + max_err 比较)。零 blocking finding。

**冻结行 reconcile(append-only,不改上方 0-byte 条款):**
- **D-MB1-4**:§3 冻结 deliverable 行的 `ci/vulkan_present_smoke.py` 为规划期占位名;**操作性 CI 门实际 = `ci/vulkan_graphics_smoke.py`**(步骤 56,offscreen graphics 真绿),present/swapchain 归 honest-defer **RD-032**(CI_GATES §2.56 改名已落 W2)。offscreen 出图像素对照即 D-MB1-4「像素对照」证据面;真窗口 present 属 RD-032 / 尾门。
- **G-MB1-6 DoD 补注**:非-Windows loader 现锁 `libvulkan.so`(Android 正确);Linux 桌面运行期 SONAME 常为 `libvulkan.so.1`(`libvulkan.so` 为 -dev 符号链接)→ AMD/Linux runtime bring-up(G-MB1-6)时须加 `libvulkan.so.1` 优先的 fallback 探测顺序。

**尾门状态:**
- **G-MB1-3**:NVIDIA 系统-ICD compute 真绿;第二 ICD 跨厂商 leg 已接线 + SKIP-honest,**维持 PARTIAL**——真 lavapipe 绿待 runner 装软件 ICD 二进制(follow-up,本轮不下载)。
- **G-MB1-6(AMD 真卡)/ G-MB1-7(Android 真机 on-device)维持 open**:缺硬件,不伪造 device 绿、不签;NVIDIA(+lavapipe)跑通不充作 AMD/Android 已验证。
- **未 push / 未 merge**:全部本地 commit(18 ahead of origin/main);合入公开 main 及与并行未合的 MS1.4 分支的 `deferred.json`/`traceability_matrix.json`/`pr-smoke.yml` post-merge reconcile(RD/CI 步骤已分区预留,机械)归 owner 明确授权。

### 进度记录(续作 W5 + W6:toolchain/供应链条款 + win32 present,真实红绿 — 2026-07-15)

W5 `bcde12fc`(RXS-0212 SPIR-V/glslang 工具链定位 + fail-closed gate 三态 · RXS-0213 Vulkan 绑定供应链纪律〔手写薄 loader + 零外部 Vulkan/SPIR-V 绑定 crate〕——formalize 既落实现,anchor 真跑 spirv-val 三态 + 解析真 Cargo.toml 断言零外部 crate;完成 D-MB1-2 声明的 0200~0205/0212/0213 条款集)+ W6 `35c94f90`(**win32 swapchain present 落地** — `vk::run_graphics_present`〔feature vulkan + cfg(windows)〕隐藏 win32 窗口 + VkSurfaceKHR〔VK_KHR_win32_surface〕+ VkSwapchainKHR〔VK_KHR_swapchain〕渲染 N 帧居中三角形,swapchain-image `vkCmdCopyImageToBuffer` 回读像素断言〔covered=968 == offscreen,**反证原 defer 的『present 无 headless 数值校验』理由**〕+ `vkQueuePresentKHR` 逐帧 VK_SUCCESS + validation 零报错;CI 步骤 58 `ci/vulkan_present_smoke.py`;RXS-0210 L4 present code-deferral **discharge**,RD-032 追加 history〔status 维持 open:AMD/Android 硬件 present = G-MB1-6/7〕)。trace 196→198;`dxil-backend 404 恒` / `rurix-rt default 17` / `vulkan 23` / `rurix-pkg 35`;offscreen+compute 两冒烟零漂移;每片对抗 review 全 pass(零 blocking)。

### G-MB1-2 MB1.1 SPIR-V codegen — 签署(agent 完全自主,evidence-based;2026-07-15)

DoD 逐项达成(措辞见 YAML `acceptance_gates` G-MB1-2):`rx build --target vulkan` 对 compute + vertex + fragment 语料产 **spirv-val-clean** `.spv`(退出码 0)——RXS-0200~0205 + toolchain 定位/gate RXS-0212;篡改 `.spv` → spirv-val 拒(红)/复原绿;数学 intrinsic→GLSL.std.450 映射单测绿;`ci/vulkan_codegen_smoke.py` **步骤 54**(8 语料 8/8 spirv-val vulkan1.0)内建 red_self_test〔F64-subset→RX6026 / 篡改字节→spirv-val 拒〕+ 缺工具 SKIP(dev-env degrade,非 fake pass);host 门 + trace(198/198)全绿;**NVIDIA(dxil/default)零回归**(`rurixc --features dxil-backend --lib` 404 / `--lib` 318 恒定)。证据:commits 2388434d/fad6c880/76a44fc6/7a00f3b4/db2c899f + W5 `bcde12fc`;`ci/vulkan_codegen_smoke.py`。**签署:agent 完全自主(D-406 v2.0;红线 3 已 owner discharge〔G-MB1-1〕,技术门 evidence-based 非治理闸口)。**

### G-MB1-3 MB1.2 Vulkan compute 运行时 — 签署(agent 完全自主;2026-07-15)

DoD 逐项达成:本机 **NVIDIA RTX 4070 Ti-Vulkan compute 端到端真跑 saxpy=a*x+out 数值精确 max_err=0**(measured;n=1024,a=2)+ `VK_LAYER_KHRONOS_validation` **零报错**(反证:错入口名 → VUID-VkPipelineShaderStageCreateInfo-pName-00707,证 layer 生效);**第二 ICD**:W4 跨厂商数值回归 leg 已接线(`RURIX_VK_LAVAPIPE_ICD`→ 仅 subprocess env 注入 `VK_DRIVER_FILES` 跑同一 `.spv` 断言 out[*]+max_err 一致),本机无软件 ICD → **`SKIP: second ICD unavailable (dev-env degrade)`**——**符合 DoD「若 ICD 不可得则标 dev-env degrade + DoD,不 fake」**(primary env 剥 ambient `VK_DRIVER_FILES` 保 honest,反证 bogus 路径下 primary 仍绿)。**〔coherence:先前 W1~W4 §8 记录保守标 PARTIAL;本正式签署据上述 DoD 明文 dev-env-degrade 条款确认达标——第二 ICD 缺件属 dev-env 降级,非达标缺口;lavapipe 真绿为可选证据强化。〕**Backend trait(RXS-0206)引入后 **CUDA(PTX/cubin/DXIL)零漂移**(NVIDIA 零回归硬约束:dxil 404 / rurix-rt default 恒);host 门 + trace 全绿。RXS-0206~0209。`ci/vulkan_device_smoke.py` **步骤 55**。证据:commits b0567a51/139c08c7/46055517/e3d5f822/cd12abf2。**可选强化(available follow-up)**:runner 装 lavapipe 二进制 → 第二 ICD SKIP→真跨厂商 green(见「里程碑状态」pending 授权)。**签署:agent 完全自主。**

### G-MB1-4 MB1.3 Vulkan graphics + present — 签署(agent 完全自主;2026-07-15)

DoD 逐项达成:本机 NVIDIA **出图 + present 真跑,像素对照归档(measured)**——offscreen 三角形(W2,`run_graphics_offscreen`,步骤 56,covered=968 背景/中心/插值断言)+ **win32 swapchain present(W6,`run_graphics_present`,步骤 58,frames=3 covered=968,`vkQueuePresentKHR` 逐帧 VK_SUCCESS,swapchain-image 回读像素校验)**;两路 `VK_LAYER_KHRONOS_validation` **零报错**;codegen 方案 B 修 VUID-08742(Vulkan 路去 SPV_GOOGLE,DXIL A/B 字节相等 + dxil 404);red_self_test 反证(provenance `.spv` → VUID-08742 退出码判红,offscreen + present 双路);host 门 + trace 全绿。RXS-0210。证据:commit d13f6e76(offscreen)+ 35c94f90(present)。**D-MB1-4 present 面:NVIDIA/Windows win32 present 已实证达成**;AMD 真卡 present 像素校验 = G-MB1-6、Android surface present = G-MB1-7(硬件尾门)。**签署:agent 完全自主。**

### G-MB1-5 MB1.4 Android 交叉构建 — 签署(agent 完全自主;2026-07-15)

DoD 逐项达成:`aarch64-linux-android` 交叉编译 rurix-rt(Vulkan 后端)+ Android surface present 代码就位——**加载缝 cfg 分叉**(`#[cfg(not(windows))]` = `libvulkan.so`/dlopen)消除唯一链接期 OS 符号缝〔Windows 路逐字节零漂移双冒烟复验〕+ `#[cfg(target_os="android")] android_present`〔`vkCreateAndroidSurfaceKHR` FFI stub〕android 编译面就位;**平台无关单元测试绿**(`loader_seam_selects_platform_lib` + `entry_point_name`,`ci/vulkan_android_build_smoke.py` 步骤 57 阶段① host 21 恒跑);本机无 NDK/target → 交叉构建阶段② **SKIP 标 dev-env degrade(非 fake)**——**符合 DoD「NDK 缺失 → SKIP 标 dev-env degrade(非 fake)」**;`RURIX_REQUIRE_ANDROID=1` 翻硬红。**设备 on-device 运行不在本门(→ G-MB1-7 open 尾门)**。RXS-0211。证据:commit 795156ed。**可选强化(available follow-up)**:装 NDK + `rustup target add aarch64-linux-android` → 交叉构建 SKIP→真链接 green(`.cargo/config.toml` 已 wired;见 pending 授权)。**签署:agent 完全自主。**

### 里程碑状态(软件面完成,尾门 + 合入 pending;2026-07-15)

- **软件面(pre-hardware)完成**:D-MB1-1~5 交付物 + 技术门 G-MB1-2~5 全签署(evidence-based)。全 in-scope 条款体 RXS-0200~0213(14 条)落地,trace **198/198**;codegen + compute + graphics + **win32 present** + Android 交叉缝 + 2nd-ICD wiring 全就位,本机 NVIDIA RTX 4070 Ti 真实红绿。
- **可选强化(pending owner 授权下载软件件,非硬件)**:① lavapipe 软件 ICD(≈30–60MB)→ G-MB1-3 第二 ICD SKIP→真跨厂商 green;② Android NDK(≈600MB–1GB)+ `rustup target add` → G-MB1-5 交叉构建 SKIP→真链接 green。二者当前经 DoD 的 dev-env-degrade SKIP 条款已达标签署;下载后为**证据强化**,非达标前置。**下载属外部动作 → 待 owner 明确授权(安全纪律:下载须逐件授权)。**
- **两道硬件尾门维持 open,不签、不伪造**:**G-MB1-6 AMD 真卡**(gfxNNNN compute+graphics+present 像素/数值对照 + validation 零)/ **G-MB1-7 Android 真机 on-device**(arm64 libvulkan.so ANativeWindow present + compute,logcat)。NVIDIA(+未来 lavapipe)跑通不充作 AMD/Android 已验证。G-MB1-6 DoD 补注:非-Windows loader `libvulkan.so`(Android 正确);Linux 桌面运行期 SONAME 常 `libvulkan.so.1` → AMD/Linux bring-up 须加 fallback 探测。
- **里程碑整体 close-out(status active→closed + `mb1-closed` 基准 tag)= NOT done**:里程碑未上 main(现 21 commits 本地未 push/未 merge,已从 base #131 分叉于当前 origin/main〔含 MS1 close-out #136〕→ 合入为三路 merge);close-out 语义前置 = **owner 明确授权 push/merge**(硬规则:outward-facing)+ 与 MS1 分支 `deferred.json`/`traceability_matrix.json`/`pr-smoke.yml` post-merge reconcile(RD/CI 步骤已分区预留 = 机械)。技术门签署(本节)属 agent 自主;里程碑 status flip 归 owner merge 后。

### 进度记录(可选强化落地:lavapipe 第二 ICD 真跨厂商绿 + NDK 交叉构建真链接绿 — owner 授权下载,2026-07-15)

owner 明确授权下载两件软件件(非硬件),把 G-MB1-3 / G-MB1-5 从 DoD-达标的 dev-env-degrade SKIP **升级为 measured 真实红绿**。工具件驻 dev 机 scratch/系统(**不入库**);证据如下。

- **G-MB1-3 第二 ICD → measured 真跨厂商绿**:Mesa **lavapipe**(`vulkan_lvp.dll`,Vulkan 1.4.348,纯 CPU 光栅;Mesa 26.1.3,经 owner 授权自 `github.com/pal1000/mesa-dist-win` release `mesa3d-26.1.3-release-msvc.7z`〔65.8MB〕取,官方 `7zr` 解包)。`RURIX_VK_LAVAPIPE_ICD=<lvp_icd.x86_64.json>` → `ci/vulkan_device_smoke.py`(步骤 55)第二 ICD leg 真跑:primary NVIDIA RTX 4070 Ti + 2nd-ICD lavapipe 对同一 `.spv` saxpy **跨厂商数值逐位相等**(out[0]=0 / out[1]=2.5 / out[1023]=2557.5 / **max_err=0.00e0**,NVIDIA GPU 与 lavapipe CPU 一致)+ validation 静默 + exit 0 → 证「单一 `.spv` 经非-NVIDIA 驱动可消费且数值回归」。**G-MB1-3 第二 ICD 从 SKIP 升级为 measured 真绿。**
- **G-MB1-5 交叉构建 → measured 真链接绿**:Android **NDK r27d**(27.3.13750724,经 owner 授权自 Google 官方 `sdkmanager`〔dl.google.com〕装,JDK 22)+ `rustup target add aarch64-linux-android`。W8(commit `2353abd9`)修 `sys.rs` CUDA loader 第二链接期缝后,`cargo build -p rurix-rt --features vulkan --target aarch64-linux-android` **lib + 全 9 bin 真链接绿**(AARCH64 ELF,undef 仅 `dlopen@LIBC` 零 `LoadLibraryA`)+ `RURIX_REQUIRE_ANDROID=1 ci/vulkan_android_build_smoke.py`(步骤 57)phase-1 host 单测 23 + phase-2 **真交叉构建 PASS**(非 SKIP)+ exit 0。**G-MB1-5 交叉构建从 SKIP 升级为 measured 真绿。**
- **provisioning 注**:lavapipe DLL/ICD 驻 dev 机 scratch、NDK 驻 `C:/Android/Sdk/ndk/27.3.13750724`——**均不入库**(体积 + 非源);CI runner 需同等 provisioning 方可复现真绿(否则回落 DoD 的 dev-env-degrade SKIP,仍达标非 fake)。W8 `sys.rs` cfg-split 已入库(交叉构建能力随源走,不依赖机器)。
- **两道硬件尾门不受影响,维持 open**:**lavapipe(软件 CPU)跨厂商跑通 ≠ AMD 真卡(G-MB1-6)已验证**;**aarch64 交叉构建绿 ≠ Android 真机 on-device(G-MB1-7)已验证**。软件 ICD + 交叉构建是 pre-hardware 证据强化,非硬件替代;G-MB1-6/7 缺硬件维持 open、不签、不伪造。
