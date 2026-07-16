# MB1 计划 — 串行子里程碑(多后端新纪元第一期)

> 所属:[MB1_CONTRACT.md](MB1_CONTRACT.md) / [../../11_ROADMAP.md](../../11_ROADMAP.md) §5。子里程碑严格串行,各自独立可验、栈式 PR、逐阶段真实红绿。**全部 gated on MB1.0 治理闸口(owner 裁决红线 3 解除 + RFC-0011 批准)。**

---

## MB1.0 · 治理包(owner 裁决闸口 — 不碰后端码)

- **产出**:RFC-0011(Full RFC,Draft)+ spec/vulkan_backend.md 脚手架(RXS-0200~0213 预留,无裸条款头)+ mb1 四件套 + D-008 解除 errata 草案 + SG-003→triggered 草案 + spec/README·rfcs/README 索引草案,全数汇于 [OWNER_DECISION_PACKAGE.md](OWNER_DECISION_PACKAGE.md)。
- **闸口(G-MB1-1)**:owner 裁决 ① D-008 红线 3 解除(独立 errata PR)② SG-003 → triggered(RFC-0011)③ RFC-0011 Approved。**agent 不自签、不自翻**;前提诚实(SG-003 记录判定『NVIDIA 纵深完成』未达)。
- **出口**:三项裁决落地后,MB1.1 解锁。

## MB1.1 · SPIR-V codegen(纯 emit + 验证,不需 GPU)

- **条款**:spec/vulkan_backend.md RXS-0200~0205 + RXS-0212(toolchain)+ RXS-0213(供应链)条款体 + 每条 ≥1 `//@ spec` 锚定(条款 commit 先于实现 commit)。
- **实现**:`driver.rs::compile` 加 `--target vulkan` arm(仿 dxil,cfg-gate feature `vulkan-backend`)+ CLI 合法目标扩容(`src/rx/main.rs`)+ 新模块 `src/rurixc/src/vulkan_codegen.rs`——**抽取泛化** `dxil_spirv.rs` 编码器骨架;compute 新增(GLCompute/LocalSize/compute builtins/存储缓冲/OpAccessChain/结构化控制流/OpControlBarrier)+ graphics 复用(vertex/fragment)+ `__nv_*`→GLSL.std.450 映射表。
- **验证**:每 `.spv` 过 `spirv-val`;conformance SPIR-V golden(compute + vs + fs bless);`ci/vulkan_codegen_smoke.py`(仿 `dxil_codegen_smoke.py`:locate glslang/spirv-val,缺工具 SKIP 非 fake,red_self_test,确定性 ×N,篡改红/复原绿);CI 步骤(§CI_GATES);错误码 6xxx en/zh 成对;stable 快照重 bless;trace 再生。
- **DoD(G-MB1-2)**:`--target vulkan` 产 spirv-val-clean `.spv`,golden bless,host 四门 + trace N/N 全绿。

## MB1.2 · Vulkan compute 运行时(本机 NVIDIA + lavapipe 真跑)

- **条款**:RXS-0206~0209 条款体 + 锚定。
- **实现**:rurix-rt 引入 `Backend`/`GpuDevice` trait(提升 `impl Cuda` 方法集,句柄改关联类型,backend 选择器替 static CUDA)——**CUDA 收敛为首实现,NVIDIA 零回归(硬约束,回归网守）**;新 Vulkan 后端(手写薄 `vulkan-1` loader 默认,§9 Q-Binding):instance/device/queue、shader module、compute pipeline+layout、descriptor set/pool、command buffer、dispatch、buffer upload/download;launch marshalling(descriptor-binding,保 MS1.2 `rxrt_launch` ABI 字节不变,ordinal→(set,binding)+push constant);artifact 泛化(`ArtifactKind::Spirv` + `ArchKey` gfx + 描述表 v2 + rurix.lock)。
- **验证**:本机 NVIDIA(RTX 4070 Ti)-Vulkan compute 端到端真跑(saxpy/reduce 等价 UC)数值对照;**第二 ICD**(lavapipe/SwiftShader)CI 红绿(跨厂商回归;ICD 不可得则标 dev-env degrade + DoD,vendor 或 follow-up);全程 `VK_LAYER_KHRONOS_validation` 零报错;CUDA 既有路回归网零漂移。
- **DoD(G-MB1-3)**:NV + lavapipe 双 ICD compute 真绿,validation 零报错,NVIDIA 零回归,host 门 + trace 全绿,run URL 归档。

## MB1.3 · Vulkan graphics + present(本机 NVIDIA 真跑)

- **条款**:RXS-0210 条款体 + 锚定。
- **实现**:render pass、graphics pipeline(vertex+fragment SPIR-V)、swapchain、`vkQueuePresentKHR`;复用 uc03/uc04 present 场景做等价验收。
- **验证**:本机 NVIDIA 出图 + present 真跑,截图/像素校验归档;validation 零报错。
- **DoD(G-MB1-4)**:NV 上出图 + present 真绿,validation 零报错,host 门 + trace 全绿,run URL 归档。

## MB1.4 · Android 交叉编译(真机前全量,构建绿即达标)

- **条款**:RXS-0211 条款体 + 锚定。
- **实现**:OS 移植缝——抽 dll 加载(`LoadLibraryA`↔`dlopen`)、调用约定、`x86_64-pc-windows-msvc`↔`aarch64-linux-android`、`libvulkan.so` 定位(`Backend` trait loader 即此缝);接 NDK 交叉编译;Android surface present(`ANativeWindow`/`VK_KHR_android_surface`)就位。NDK 缺失 → SKIP 标 dev-env degrade。
- **验证**:平台无关单元/逻辑测试绿;android-arm64 交叉**构建**绿;**设备 smoke 标 `pending-hardware`**。
- **DoD(G-MB1-5)**:android-arm64 交叉构建绿、平台无关单测绿;on-device 运行留 open 尾门(G-MB1-7)。

## 两道硬件尾门(贯穿,MB1 期不关闭)

- **G-MB1-6 AMD 真卡验收**:缺硬件 open,DoD 见契约。
- **G-MB1-7 Android 真机 on-device smoke**:缺设备 open(pending-hardware),DoD 见契约。
- **纪律**:NVIDIA(+lavapipe)跑通不充作 AMD/Android 已验证;缺硬件不伪造 device 绿、不签(反 Godot 退出码/grep 教训)。
