# G6.5 Taichi Vulkan AOT spike — 技术侦查记录（Task 1，2026-07-31）

> 本文件为 spike 前置侦查留痕（RFC-0017 §4.E / spec `run_g6_5_taichi_vulkan_aot_spike` Task 1）。
> 以 `.md` 落 evidence/：`ci/check_schemas.py` 只 glob `evidence/*.json` 且未知前缀 .json 会被
> 强制路由 m0 GPU schema 必 FAIL；`.md` 不进该脚本视野（实测核实，2026-07-31）。

## 1. UC-09 编号（Task 1.1）

- `apps/` 现存：`blackhole/` `ruridrop/` `uc05-rhi/` `uc06-renderer/` `uc08-physics/`——**UC-09 自由**（UC-07 = ruridrop 占用，UC-08 = G6.3 物理合流）。
- workspace members 经根 `Cargo.toml` `members` 注册（demo 落 `apps/uc09-taichi-spike` 时需加入）。

## 2. taichi C API 面（Task 1.2；实测 `py -3` = taichi 1.7.4 源码构建，`TI_C_API_VERSION 1007000`）

- 头文件（仓外，仅侦查引用）：`H:\Kimi_Agent_Taichi Engine 优化计划\taichi\c_api\include\taichi\{taichi.h,taichi_core.h,taichi_vulkan.h}`；运行库 `…\taichi\build\Release\taichi_c_api.dll` 在位。
- **无 `ti_load`/`ti_unload`**：宿主 `LoadLibrary` + `GetProcAddress` 直链符号（与 rurix-rt nvcuda/vulkan loader 同款纪律）。
- runtime：`ti_create_runtime(TiArch,uint32_t)`（taichi_core.h L927-932）；**interop 注入** `ti_import_vulkan_runtime(const TiVulkanRuntimeInteropInfo*)`（taichi_vulkan.h L107-108），结构九字段：get_instance_proc_addr / api_version / instance / physical_device / device / compute_queue(+family) / graphics_queue(+family)（L26-47）。
- AOT：`ti_load_aot_module`（目录）/ `ti_create_aot_module`（.tcm 字节）/ `ti_get_aot_module_kernel` / `ti_launch_kernel(runtime,kernel,argc,TiArgument*)`；ndarray 传参 = `TiArgument{type=TI_ARGUMENT_TYPE_NDARRAY, value.ndarray}`。
- 显存：`ti_allocate_memory`（`TiMemoryAllocateInfo{size,host_write,host_read,export_sharing,usage}`，kernel 参数须 `TI_MEMORY_USAGE_STORAGE_BIT`）；**`ti_export_vulkan_memory` 存在**（taichi_vulkan.h L127-130，`TiVulkanMemoryInteropInfo{buffer,size,usage,memory,offset}`）——spike 成功判据导出面**无缺口**。
- 同步：仅 `ti_flush`/`ti_wait` 全排空；原生 queue 可经 `ti_export_vulkan_runtime` 取出后自行插 fence（§4.E2 fence 排序由此兑现）。

## 3. AOT 编译链真跑（Task 1.2）

- `ti.init(arch=ti.vulkan)` + `ti.aot.Module(ti.vulkan)` + `add_kernel(..., template_args={...})` + `archive("particles.tcm")` **真跑成功**（exit 0，零报错）；产物 3873 B `.tcm` + 1908 B `.spv` + `metadata.json`（`required_caps: spirv_version=66304` 即 SPIR-V 1.3 → Vulkan ≥1.1 满足）。
- 1.7.4 要点：ndarray 参数必须 `template_args` 传真实 `ti.ndarray` 实例匹配；`.tcm` 由 `Module.archive()` 产出（zip 形态，**非逐位可复现**——sha256 核验对象为入仓产物本体，非再生成物）。

## 4. 渲染侧接缝（Task 1.3；仓内实测）

- `rurix-rt` feature 面：`default = []`，`vulkan = []`（手写 vulkan-1.dll 薄 loader，零外部依赖纪律）；`unsafe_code = "allow"` + `undocumented_unsafe_blocks = "deny"`（每 unsafe 块携 `// SAFETY:`）。
- `render_exec::execute_frame`（render_exec.rs L595）**每次调用自建 device**，`ResourceDesc::Buffer(BufferDesc)` 仅 host 数据上载，**无外部 VkBuffer 绑定位**——TiRT buffer 要经 graph external import 消费，须同设备组合：新增 `tirt` 模块内聚「建 device → `ti_import_vulkan_runtime` → launch → `ti_export_vulkan_memory` → copy → readback」，device 创建面自 vk.rs/render_exec 内部最小提取（既有行为 0-byte）。
- `rurix-render` graph 为纯 host 规划面（`#![forbid(unsafe_code)]`，lib.rs L19）：`RenderGraph::import`（graph.rs L192）标记外部资源不入 transient；`CmdRecorder::copy` 录 `RecordedCommand` 入 `CommandLog`（compile.rs L301）——uc09 照 uc06/uc08 先例手工映射：graph import+copy 计划 host 侧真编译，device 腿由 tirt 模块在同一 VkDevice 上执行该 copy 消费 TiRT buffer。
- uc08 device 腿（apps/uc08-physics/src/device.rs）= demo 结构模板（probe_device_caps → Err fail-closed / RURIX_REQUIRE_REAL 裁决在调用方）。

## 5. 预判档

**成功臂可行，无机制缺口**：AOT 链通、interop 注入/内存导出 API 齐备、Vulkan 1.1+ queue 面满足、graph import 机制可复用不新建通路。残余风险 = 实现摩擦（R-G6-3 设备共享参数细节），由 Task 3/4 真跑定终判；失败则 RD-042 诚实登记。侦查临时产物在 `target/spike_recon/`（不入仓）。
