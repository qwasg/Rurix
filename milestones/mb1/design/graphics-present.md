I have full context now. Here is the implementation design for Phase 3 Vulkan graphics + present (RXS-0210).

---

# Phase 3 — Vulkan graphics + present (RXS-0210) 实现设计

## 决策摘要
- **坑的根因确认**:graphics `.spv` 由 `build_and_emit_vulkan` (`src/rurixc/src/vulkan_codegen.rs:507-508`) 委派 `dxil_spirv::emit_spirv_body`,后者对每个带 `field_name` 的 I/O 元素 emit `OpDecorate … UserSemantic`(`dxil_spirv.rs:518-523` I/O、`609-614` 资源)并置 `used_user_semantic=true`,组装期据此 emit `OpExtension "SPV_GOOGLE_hlsl_functionality1"`(`dxil_spirv.rs:1356-1361`)。`spirv-val --target-env vulkan1.0` 接受它(故 RXS-0204 已绿),但 `vkCreateShaderModule` 在**未启用** device 扩展 `VK_GOOGLE_hlsl_functionality1` 时按 VUID-VkShaderModuleCreateInfo-pCode-08742 拒。
- **推荐方案 B(codegen 侧对 Vulkan target 不 emit UserSemantic/SPV_GOOGLE)**。
- **运行时推荐 offscreen-first**(render→image→`vkCmdCopyImageToBuffer`→host readback→像素断言),对齐 uc04 offscreen 先例与 RXS-0170 readback 布局纪律,headless 本机 NVIDIA 真跑;swapchain/present 作 open 尾门 defer(RD-019 先例)。

---

## 1. 方案对比:VK_GOOGLE 扩展(A) vs codegen 不 emit(B)

### 方案 A — vkCreateDevice 启用 `VK_GOOGLE_hlsl_functionality1`
改动点:在 `vk.rs` graphics 设备创建路径的 `DeviceCreateInfo`(镜像现 `vk.rs:678-689`)设 `enabled_extension_count=1` + `pp_enabled_extension_names=[c"VK_GOOGLE_hlsl_functionality1"]`,并先 `vkEnumerateDeviceExtensionProperties` 探测存在性(否则 `vkCreateDevice` 报 VK_ERROR_EXTENSION_NOT_PRESENT)。

代价:
- **可移植性倒退**:该扩展在 Android(`libvulkan.so` 各厂商 ICD)、lavapipe、部分 AMD 驱动上并非普遍暴露 → 与 mb1 "单一 `.spv` 同覆盖 AMD 桌面 + Android"(spec §1)的核心承诺冲突,Phase 4 会立刻踩雷。
- **携带无用负载**:UserSemantic 是 B 路 SPIRV-Cross→HLSL→dxc 保名 provenance(`dxil_spirv.rs:512` 注释明言"spirv-cross 不消费"),Vulkan 原生按 `Location`/`BuiltIn` 消费,永不需要它。
- 需新增 `vkEnumerateDeviceExtensionProperties` FFI + 运行期探测分支,复杂度高于 B。

### 方案 B — codegen 对 Vulkan target 不 emit UserSemantic/SPV_GOOGLE ✅ 推荐
Vulkan 通道 SPIR-V 即终产物,不进 HLSL 转译链,保名无消费者。去掉 UserSemantic 后 `.spv` 对**所有** Vulkan ICD(NVIDIA/AMD/Android/lavapipe)零扩展依赖直喂 `vkCreateShaderModule`。DXIL 路(`dxil-backend`)保持 provenance,字节不变、零回归。

**精确改动(`src/rurixc/src/dxil_spirv.rs`)**:
1. `Builder` 结构(`:301-326`)加字段 `emit_provenance: bool`;`Builder::new()`(`:329`)默认 `true`(DXIL 保名不变)。
2. 两处 UserSemantic emit 加门:`:518` `if self.emit_provenance && !elem.field_name.is_empty()`、`:609` `if self.emit_provenance && !res.name.is_empty()`。`used_user_semantic` 因此在 Vulkan 路保持 `false`,`:1357` 的 `OpExtension` 自然不 emit。
3. 新增公开入口:
```rust
/// Vulkan 原生消费:去 UserSemantic/SPV_GOOGLE(保名仅 B 路 HLSL 转译需要)。RXS-0210。
pub fn emit_spirv_body_vulkan(stage: ShaderStage, body: &Body) -> Result<Vec<u32>, DxilError> {
    emit_spirv_inner_prov(stage, &body.io_sig, &body.resources, Some(body), /*provenance=*/false)
}
```
把 `emit_spirv_inner`(`:1291`)抽成带 `provenance: bool` 的 `emit_spirv_inner_prov`,在 `:1310` 处 `let mut b = Builder::new(); b.emit_provenance = provenance;`。现有 `emit_spirv`/`emit_spirv_body` 保持 `provenance=true`(委派新函数)。
4. `vulkan_codegen.rs:508` 改 `emit_spirv_body` → `emit_spirv_body_vulkan`(仅此一处路由改)。

**副作用面**:vertex 顶点属性保名(RXS-0159 IR1(a) location 覆盖旗标)是 DXIL host 侧 `dxil_codegen` 逻辑,不经 SPIR-V UserSemantic,故 Vulkan 侧去 UserSemantic 不影响顶点属性绑定;Vulkan 按 `Location` 号(`:501-507` 递增)绑定,已足够。

**spec 影响**:RXS-0204 IR3(`spec/vulkan_backend.md:173`)现声明"SPV_GOOGLE… Vulkan 驱动忽略但合规"——此断言在 device 面被 VUID 证伪。RXS-0210 条款体须记该 erratum(Vulkan 路去保名),并在 RXS-0204 修订记录追注"provenance 装饰移至 target-conditional,Vulkan 路不 emit"。此为 codegen 行为微调,`spirv-val` 仍 accept(去装饰只减不增),conformance 无红。

---

## 2. Vulkan graphics 运行时最小 FFI 增量(offscreen-first)

在 `src/rurix-rt/src/vk.rs` 内**新增 graphics 路径**,与 `run_compute` 并列,复用现有 loader/instance/device/queue/memtype 骨架(`:494-695` 可抽 `Instance`/`Device` 建立为共享 helper,或复制骨架保持线性)。queue family 选择改为 `QUEUE_GRAPHICS_BIT (0x1)`(现 `:66` 只有 `QUEUE_COMPUTE_BIT`)。

### 新增常量(sType/enum)
```
ST_IMAGE_CREATE_INFO=14, ST_IMAGE_VIEW_CREATE_INFO=15,
ST_RENDER_PASS_CREATE_INFO=38, ST_FRAMEBUFFER_CREATE_INFO=37,
ST_GRAPHICS_PIPELINE_CREATE_INFO=28,
ST_PIPELINE_VERTEX_INPUT_STATE_CI=19, ST_PIPELINE_INPUT_ASSEMBLY_STATE_CI=20,
ST_PIPELINE_VIEWPORT_STATE_CI=22, ST_PIPELINE_RASTERIZATION_STATE_CI=23,
ST_PIPELINE_MULTISAMPLE_STATE_CI=24, ST_PIPELINE_COLOR_BLEND_STATE_CI=26,
ST_RENDER_PASS_BEGIN_INFO=43
IMAGE_TYPE_2D=1, IMAGE_TILING_OPTIMAL=0,
IMAGE_USAGE_COLOR_ATTACHMENT=0x10, IMAGE_USAGE_TRANSFER_SRC=0x1,
FORMAT_R8G8B8A8_UNORM=37,
IMAGE_LAYOUT_UNDEFINED=0, _COLOR_ATTACHMENT_OPTIMAL=2, _TRANSFER_SRC_OPTIMAL=6,
IMAGE_ASPECT_COLOR=0x1,
ATTACHMENT_LOAD_OP_CLEAR=1, ATTACHMENT_STORE_OP_STORE=0,
PIPELINE_BIND_POINT_GRAPHICS=0,
PRIMITIVE_TOPOLOGY_TRIANGLE_LIST=3,
SUBPASS_CONTENTS_INLINE=0, SAMPLE_COUNT_1=0x1,
POLYGON_MODE_FILL=0, CULL_MODE_NONE=0, FRONT_FACE_CW=1,
SHADER_STAGE_VERTEX=0x1, SHADER_STAGE_FRAGMENT=0x10,
BUFFER_USAGE_TRANSFER_DST=0x2, BUFFER_USAGE_VERTEX=0x80,
MEM_DEVICE_LOCAL=0x1,
BUFFER_IMAGE_COPY / IMAGE_MEMORY_BARRIER 相关
```

### 关键 #[repr(C)] 结构(逐字节对齐,镜像现有 `:81-341` 风格)
- `VkExtent2D{width,height:u32}`、`VkExtent3D`(已在 `:104`)、`VkOffset3D`、`VkRect2D{offset:VkOffset2D, extent:VkExtent2D}`、`VkViewport{x,y,w,h,minDepth,maxDepth:f32}`
- `ImageCreateInfo`(sType,pNext,flags,imageType,format,extent:VkExtent3D,mipLevels=1,arrayLayers=1,samples,tiling,usage,sharingMode,qfic,pQfi,initialLayout)
- `ImageViewCreateInfo`(…image,viewType=2D,format,components:VkComponentMapping〔全 IDENTITY=0〕,subresourceRange:VkImageSubresourceRange{aspectMask,baseMip=0,levelCount=1,baseLayer=0,layerCount=1})
- `AttachmentDescription`(flags,format,samples,loadOp,storeOp,stencilLoad=DONT_CARE,stencilStore=DONT_CARE,initialLayout=UNDEFINED,finalLayout=TRANSFER_SRC_OPTIMAL)
- `AttachmentReference{attachment=0, layout=COLOR_ATTACHMENT_OPTIMAL}`
- `SubpassDescription`(flags,bindPoint=GRAPHICS,inputCount=0,…,colorCount=1,pColorAttachments=&ref,…)
- `RenderPassCreateInfo`(…attachmentCount=1,pAttachments,subpassCount=1,pSubpasses,dependencyCount=0)
- `FramebufferCreateInfo`(…renderPass,attachmentCount=1,pAttachments=&imageView,width,height,layers=1)
- `PipelineShaderStageCreateInfo`(已在 `:237`,复用;两个实例 vertex/fragment)
- `PipelineVertexInputStateCreateInfo`(bindingCount,pVertexBindingDescriptions:`VkVertexInputBindingDescription{binding=0,stride,inputRate=VERTEX}`,attributeCount,pAttrs:`VkVertexInputAttributeDescription{location,binding=0,format,offset}`)
- `PipelineInputAssemblyStateCreateInfo`(…topology=TRIANGLE_LIST,primitiveRestart=0)
- `PipelineViewportStateCreateInfo`(…viewportCount=1,pViewports=&vp,scissorCount=1,pScissors=&rect)
- `PipelineRasterizationStateCreateInfo`(…depthClamp=0,rasterizerDiscard=0,polygonMode=FILL,cullMode=NONE,frontFace,depthBias=0,lineWidth=1.0)
- `PipelineMultisampleStateCreateInfo`(…rasterizationSamples=SAMPLE_COUNT_1,sampleShading=0,…)
- `PipelineColorBlendAttachmentState{blendEnable=0, …, colorWriteMask=0xF}`
- `PipelineColorBlendStateCreateInfo`(…logicOp=0,attachmentCount=1,pAttachments,blendConstants=[0;4])
- `GraphicsPipelineCreateInfo`(…stageCount=2,pStages,pVertexInputState,pInputAssemblyState,pViewportState,pRasterizationState,pMultisampleState,pDepthStencilState=null,pColorBlendState,pDynamicState=null,layout,renderPass,subpass=0,basePipeline=NULL,-1)
- `RenderPassBeginInfo`(…renderPass,framebuffer,renderArea:VkRect2D,clearValueCount=1,pClearValues:`VkClearValue`〔union,填 float32[4] clear color〕)
- `VkImageMemoryBarrier`(…srcAccess,dstAccess,oldLayout,newLayout,srcQFI=IGNORED,dstQFI=IGNORED,image,subresourceRange)
- `VkBufferImageCopy{bufferOffset=0,bufferRowLength=0,bufferImageHeight=0,imageSubresource:VkImageSubresourceLayers{aspect=COLOR,mip=0,baseLayer=0,layerCount=1},imageOffset:{0,0,0},imageExtent:{w,h,1}}`

### 新增 device 符号(经 `dp!` 宏 `vk.rs:734`)
`vkCreateImage / vkDestroyImage / vkGetImageMemoryRequirements / vkBindImageMemory / vkCreateImageView / vkDestroyImageView / vkCreateRenderPass / vkDestroyRenderPass / vkCreateFramebuffer / vkDestroyFramebuffer / vkCreateGraphicsPipelines / vkCmdBeginRenderPass / vkCmdEndRenderPass / vkCmdBindVertexBuffers / vkCmdSetViewport(可选,用静态则免) / vkCmdDraw / vkCmdPipelineBarrier / vkCmdCopyImageToBuffer`(pipeline layout/shader module/command/queue/buffer/memory 符号复用现有)。

### 命令序列(offscreen)
```
建立:instance→pd→graphics qfi→device→queue (镜像 :570-695)
颜色 image:vkCreateImage(R8G8B8A8_UNORM, usage=COLOR_ATTACHMENT|TRANSFER_SRC, tiling=OPTIMAL,
  initialLayout=UNDEFINED) → device-local mem alloc+bind → vkCreateImageView
render pass:1 color attachment(loadOp=CLEAR, storeOp=STORE, finalLayout=TRANSFER_SRC_OPTIMAL)
framebuffer(renderPass, imageView, W×H)
vertex buffer:host-visible, 上传 3 顶点(pos vec4 + color vec4) [复用现 buffer/memtype 逻辑]
readback buffer:host-visible, size = align_up(W*4,256)*H (对齐 RXS-0170 口径)
shader:vkCreateShaderModule(vs.spv) + vkCreateShaderModule(fs.spv),pName 均 "main"
pipeline layout(空 set/无 push const)→ vkCreateGraphicsPipelines
录制 cmd:
  vkCmdBeginRenderPass(CLEAR 到已知背景色) 
  vkCmdBindPipeline(GRAPHICS)
  vkCmdSetViewport/Scissor(或静态) 
  vkCmdBindVertexBuffers(0,1,&vbuf,&0)
  vkCmdDraw(3,1,0,0)
  vkCmdEndRenderPass         // storeOp→image 现 TRANSFER_SRC_OPTIMAL
  vkCmdPipelineBarrier(image→TRANSFER_SRC,若 renderpass finalLayout 已达则免)
  vkCmdCopyImageToBuffer(image, TRANSFER_SRC_OPTIMAL, readbackBuf, region)
提交:vkQueueSubmit + vkQueueWaitIdle (镜像 :1131-1145)
回读:map readback → 逐像素断言
逆序销毁全部句柄
```
公共入口签名:
```rust
pub fn run_graphics_offscreen(
    vs_spv: &[u32], fs_spv: &[u32],
    vertices: &[u8],            // 3 顶点交错 pos+color
    vertex_stride: u32,
    attrs: &[(u32/*location*/, u32/*format*/, u32/*offset*/)],
    width: u32, height: u32,
    clear: [f32;4],
) -> Result<Vec<u8>, String>   // 返回 tightly-packed RGBA8 (去 row-pitch padding)
```
row-pitch:`vkCmdCopyImageToBuffer` 用 `bufferRowLength=0`(紧凑)即可,但 readback buffer 须按对齐分配;回读时按实际 copy 的紧凑布局取 `W*4*H`。**unsafe-audit 追加 U27**(graphics FFI 边界,`unsafe-audit/rurix-rt.md`),SAFETY 契约在 `run_graphics_offscreen` 公共入口统一声明,同 U26 模式。

### swapchain present(defer,尾门)
需 `VK_KHR_surface`+平台 surface(`VK_KHR_win32_surface`/`VK_KHR_android_surface`)+`VK_KHR_swapchain` device 扩展 + 窗口句柄 + `vkAcquireNextImageKHR`/`vkQueuePresentKHR` + semaphore/fence。**不进 Phase 3 必要面**:窗口/present 无法 headless 数值校验、引入平台 surface 分叉,归 RXS-0210 honest-defer(记 RD-019 或续号 RD-030,DoD 写明"真窗口 present 目视 + 平台 surface"),与 uc04 offscreen-first 先例(`readback.rs:4-9`)一致。

---

## 3. 本机 NVIDIA 验证方案(offscreen 三角形 → readback → 像素断言)

**conformance 新增**(`conformance/vulkan/accept/`,全在现有 lowering 子集内,复用 RXS-0204 已绿的 passthrough 面):
- `vk_tri_vs.rx`:
```rust
//@ spec: RXS-0210
struct TriIn  { pos: vec4<f32>, color: vec4<f32> }
struct TriVary { #[builtin(position)] clip: vec4<f32>, #[interpolate(perspective)] color: vec4<f32> }
vertex fn tri_vs(inp: TriIn) -> TriVary { TriVary { clip: inp.pos, color: inp.color } }
```
- `vk_tri_fs.rx`:
```rust
//@ spec: RXS-0210
struct TriVary { #[builtin(position)] clip: vec4<f32>, #[interpolate(perspective)] color: vec4<f32> }
struct FsOut  { color: vec4<f32> }        // location 0 → 颜色附件
fragment fn tri_fs(inp: TriVary) -> FsOut { FsOut { color: inp.color } }
```
顶点几何放 host 顶点缓冲(避免 vertex_index 算术超子集),passthrough vertex/fragment 保持在 body-lowering 白名单内。

**新 demo bin** `src/rurix-rt/src/bin/vk_triangle.rs`(镜像 `vk_saxpy.rs`,`required-features=["vulkan"]`):读 `vs.spv fs.spv` 两参 → 定义 3 顶点(全屏三角形 clip 空间 `(-1,-1),(3,-1),(-1,3)`,顶点色红/绿/蓝)→ `run_graphics_offscreen(…, W=64,H=64, clear=[0,0,0,1])` → 断言:
- 背景角像素(如 (0,63) 三角外)== clear 黑;
- 中心像素 (32,32) 非零且落在三色重心插值区间(容差断言,或断"非背景色");
- 至少若干像素被覆盖(rasterization 生效)。
输出 `VK_TRIANGLE: ok W=64 H=64 covered=<n> center=(r,g,b)`,数值校验通过 exit 0,失配 exit 1。

**验证命令(本机 NVIDIA)**:
```bash
cargo build -p rurixc --features vulkan-backend
cargo build -p rurix-rt --features vulkan --bin vk_triangle
target/debug/rurixc --target vulkan conformance/vulkan/accept/vk_tri_vs.rx -o /tmp/vs.spv
target/debug/rurixc --target vulkan conformance/vulkan/accept/vk_tri_fs.rx -o /tmp/fs.spv
spirv-val --target-env vulkan1.0 /tmp/vs.spv     # 应 accept 且不再含 SPV_GOOGLE
spirv-dis /tmp/vs.spv | grep -c UserSemantic      # 期望 0(方案 B 生效反证)
RURIX_VK_VALIDATION=1 target/debug/vk_triangle /tmp/vs.spv /tmp/fs.spv
# 期望:VK_TRIANGLE: ok …,stderr 无 "Validation Error"/"VUID-"
```
**反证(证 layer 生效 + 证坑已修)**:临时把 codegen 改回 `emit_spirv_body`(带保名)跑 `vk_triangle` → `vkCreateShaderModule` 报 VUID-…-08742(证方案 B 前坑真实);恢复后绿。

**CI 新增 `ci/vulkan_graphics_smoke.py`**(镜像 `vulkan_device_smoke.py` 结构):build `rurixc --features vulkan-backend` + `rurix-rt --features vulkan --bin vk_triangle` → 两 `.spv` codegen → 跑 demo → `VK_TRIANGLE: ok` + exit 0 + validation 静默。fail-closed:无 Vulkan 设备 → SKIP(`RURIX_REQUIRE_REAL=1` 缺设备翻硬红,GPU runner)。接 `pr-smoke.yml` **步骤 56**(在 `:430` 后追加,契约 G-MB1-4,`RURIX_REQUIRE_REAL=1`)。

---

## 4. RXS-0210 条款体要点 + conformance/anchor

**`### RXS-0210 Vulkan graphics 运行时 + offscreen present`**(落 `spec/vulkan_backend.md:228` 之后,§2 内,修订记录加 v1.6 行)。按 FLS 分节,**严禁 UB 节**:

- **Syntax**:无语言文法面(运行时/FFI 面 + codegen provenance 微调)。
- **Legality**:
  - L1(offscreen 必要面):render pass(单 color attachment,CLEAR/STORE)+ graphics pipeline(vertex+fragment 双 stage,pName="main")+ framebuffer + vertex buffer 绑定 + `vkCmdDraw` + `vkCmdCopyImageToBuffer` 回读;像素数值对照为 device 必要证据。
  - L2(provenance 去除,承 RXS-0204 erratum):Vulkan 原生 SPIR-V **不 emit** `UserSemantic`/`SPV_GOOGLE_hlsl_functionality1`(去后 `vkCreateShaderModule` 免 device 扩展依赖,跨 ICD 可移植);DXIL 路保名字节不变(target-conditional,零回归)。
  - L3(fail-closed):缺 Vulkan 驱动 / 无 graphics queue / pipeline 创建失败 / image 格式不支持 → 确定性 `Err`(非 panic,P-01,无静默 fallback)。
  - L4(present defer):swapchain/窗口 present(平台 surface)→ honest-defer **RD-030**(不进必要条款,尾门 DoD:真窗口目视 + win32/android surface)。
- **Dynamic Semantics**:`run_graphics_offscreen(vs,fs,vertices,attrs,W,H,clear)` 确定性渲染到 device-local color image → 转 TRANSFER_SRC → copy 到 host-visible buffer → 回读紧凑 RGBA8;单 queue 同步(`vkQueueWaitIdle`)后像素确定。`VK_LAYER_KHRONOS_validation` 零报错。
- **Implementation Requirements**:
  - IR1(手写 FFI,U27):graphics VkStruct `#[repr(C)]` 逐字节对齐 + 句柄线性配对 create/destroy;gate feature `vulkan` 默认关闭,**CUDA 路零回归**。
  - IR2(codegen provenance gate):`emit_spirv_body_vulkan`(provenance=false)路由;dxil-backend 单独启用 test 字节不变(反证:diff `.spv` 仅少 UserSemantic/OpExtension)。
  - IR3(真跑校验):本机 NVIDIA offscreen 全屏三角形 → readback → 像素断言(覆盖/背景/插值)+ validation 零报错;经 `bin/vk_triangle` + `ci/vulkan_graphics_smoke.py`(GPU runner)。AMD 真卡 + present 窗口为 open 尾门(G-MB1-6 / RD-030)。
  - IR4(锚定):≥1 `//@ spec: RXS-0210`。
- **锚定测试**(> 引用行):`conformance/vulkan/accept/vk_tri_vs.rx` + `vk_tri_fs.rx`(codegen 面,`spirv-val vulkan1.0` accept 且无 SPV_GOOGLE)+ `src/rurixc/src/dxil_spirv.rs` 单测(`emit_spirv_body_vulkan` 不含 UserSemantic decoration / 无 OpExtension)+ `src/rurix-rt/src/bin/vk_triangle.rs`(本机 NVIDIA offscreen 真跑,像素断言)。

trace 全锚定:`191 → 192`(RXS-0210 ≥1 `//@ spec`;codegen 单测 + conformance + demo 均带锚点)。

---

## 5. 精确改动清单(最小改动集)

**codegen(方案 B)**
- `src/rurixc/src/dxil_spirv.rs`:`Builder` 加 `emit_provenance: bool`(`:301-326`),`new()` 默认 `true`(`:329-345`);UserSemantic 门 `:518` + `:609`;`emit_spirv_inner`→`emit_spirv_inner_prov(…, provenance)`(`:1291`),Builder 处 `b.emit_provenance=provenance`(`:1310`);新增 `pub fn emit_spirv_body_vulkan`;`OpExtension` 分支(`:1356-1361`)不改(靠 `used_user_semantic` 自然为 false)。加 2 个单测(vulkan 变体无 UserSemantic / 无 OpExtension)。
- `src/rurixc/src/vulkan_codegen.rs:508`:`emit_spirv_body` → `emit_spirv_body_vulkan`(唯一路由改)。

**运行时(vk.rs graphics 增量)**
- `src/rurix-rt/src/vk.rs`:新增 graphics 常量/#[repr(C)] 结构/函数指针类型/device 符号(见 §2);新增 `pub fn run_graphics_offscreen(...)` + 内部 `unsafe fn run_graphics_inner`(镜像 `run_compute`/`run_compute_inner`/`run_on_device` 三段式,graphics queue family)。`unsafe-audit/rurix-rt.md` 追加 **U27**。

**demo + conformance**
- 新 `src/rurix-rt/src/bin/vk_triangle.rs`;`src/rurix-rt/Cargo.toml` 加 `[[bin]] name="vk_triangle" required-features=["vulkan"]`(镜像 `Cargo.toml` vk_saxpy 块)。
- 新 `conformance/vulkan/accept/vk_tri_vs.rx` + `vk_tri_fs.rx`(LF 新文件)。

**CI + spec**
- 新 `ci/vulkan_graphics_smoke.py`(镜像 `ci/vulkan_device_smoke.py`)。
- `.github/workflows/pr-smoke.yml:430` 后追加**步骤 56**(G-MB1-4,`RURIX_REQUIRE_REAL=1`)。
- `spec/vulkan_backend.md`:§2 落 `### RXS-0210` 条款体;RXS-0204 IR3(`:173`)注 provenance erratum;修订记录加 v1.6 行;honest-defer 记 **RD-030**(present 窗口尾门)。
- milestones/mb1:`MB1_CONTRACT.md` 登记 G-MB1-4(offscreen graphics device 门)+ 尾门 G-MB1-6 增 present/AMD 项(若尾门清单在此)。

**纪律核对**:LF 新文件 / feature `vulkan` 默认关(NVIDIA CUDA 路 `cargo build/test -p rurix-rt` 零改动零回归)/ `spirv-val` 真实红绿(去保名后仍 accept,反证带保名触 VUID)/ 条款先行(RXS-0210 与实现同 PR)/ 运行期失败不占 RX 码(工具层确定性 Err)/ 编号 RXS-0210 与 spec §1 区间映射一致 / U27 unsafe 边界注册。

**关键校验点**:graphics `OpEntryPoint` 名恒为 `"main"`(`dxil_spirv.rs:1373`),故 graphics pipeline 两 stage 的 `pName` 均用 `"main"`(不走 compute 的 `entry_point_name` mangled 路径);`run_graphics_offscreen` 可硬编 `c"main"` 或复用 `entry_point_name` 解析首个 OpEntryPoint(返回 "main")。