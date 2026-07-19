//! Vulkan compute 运行时后端(mb1,RXS-0206/0207;RFC-0011 §4.6)。
//!
//! **动态加载**(非链接期绑定,镜像 [`crate::sys`] 的 nvcuda.dll 纪律):`vulkan-1.dll`
//! (桌面)/ `libvulkan.so`(Android)经 `LoadLibraryA`/`dlopen` 运行时装载 + 手写薄 FFI
//! (RFC §9 Q-Binding 默认:零外部绑定,对齐 sys.rs 无依赖纪律)。SPIR-V(Phase 1
//! `--target vulkan` 产)→ `vkCreateShaderModule` → compute pipeline → descriptor set
//! (StorageBuffer)+ push constant → `vkCmdDispatch`。AMD 桌面与 Android 消费同一 `.spv`。
//!
//! **unsafe 边界**(AGENTS 硬规则 9,注册见 `unsafe-audit/rurix-rt.md` 追加 U26):Vulkan
//! FFI 集中于本模块;每 `unsafe` 块携 `// SAFETY:`。gate 于 feature `vulkan`。
//!
//! 首期 compute 面:host-visible+coherent StorageBuffer(免 flush/invalidate)+ 单 push
//! constant 块 + 单 queue 同步提交(`vkQueueWaitIdle`)。开发期开 `VK_LAYER_KHRONOS_validation`
//! (env `RURIX_VK_VALIDATION=1`)。

// 本模块为 Vulkan FFI unsafe 边界(U26):`unsafe fn` 内的 FFI 调用不逐一再包 `unsafe {}`
// (2024 edition unsafe_op_in_unsafe_fn),SAFETY 契约在 `run_compute` 公共入口统一声明
// (句柄线性生命周期 + 逐字节对齐的 #[repr(C)] 布局)。命名沿 Vulkan 大小写惯例。
#![allow(non_snake_case, non_upper_case_globals, unsafe_op_in_unsafe_fn)]

use core::ffi::{c_char, c_void};

use crate::sampler::SamplerDesc;

// ── 句柄类型 ────────────────────────────────────────────────────────────────
// dispatchable = 指针;non-dispatchable = u64(VK_DEFINE_NON_DISPATCHABLE_HANDLE)。
type VkInstance = *mut c_void;
type VkPhysicalDevice = *mut c_void;
type VkDevice = *mut c_void;
type VkQueue = *mut c_void;
type VkCommandBuffer = *mut c_void;
type VkBuffer = u64;
type VkDeviceMemory = u64;
type VkShaderModule = u64;
type VkDescriptorSetLayout = u64;
type VkPipelineLayout = u64;
type VkPipeline = u64;
type VkDescriptorPool = u64;
type VkDescriptorSet = u64;
type VkCommandPool = u64;
type VkResult = i32;
type VkFlags = u32;
type VkDeviceSize = u64;
// graphics 句柄(RXS-0210,offscreen 出图路径;non-dispatchable = u64)。
type VkImage = u64;
type VkImageView = u64;
type VkRenderPass = u64;
type VkFramebuffer = u64;

const VK_SUCCESS: VkResult = 0;
const VK_NULL_HANDLE: u64 = 0;

// sType。
const ST_APPLICATION_INFO: u32 = 0;
const ST_INSTANCE_CREATE_INFO: u32 = 1;
const ST_DEVICE_QUEUE_CREATE_INFO: u32 = 2;
const ST_DEVICE_CREATE_INFO: u32 = 3;
const ST_SUBMIT_INFO: u32 = 4;
const ST_MEMORY_ALLOCATE_INFO: u32 = 5;
const ST_BUFFER_CREATE_INFO: u32 = 12;
const ST_PIPELINE_SHADER_STAGE_CREATE_INFO: u32 = 18;
const ST_COMPUTE_PIPELINE_CREATE_INFO: u32 = 29;
const ST_PIPELINE_LAYOUT_CREATE_INFO: u32 = 30;
const ST_DESCRIPTOR_SET_LAYOUT_CREATE_INFO: u32 = 32;
const ST_DESCRIPTOR_POOL_CREATE_INFO: u32 = 33;
const ST_DESCRIPTOR_SET_ALLOCATE_INFO: u32 = 34;
const ST_WRITE_DESCRIPTOR_SET: u32 = 35;
const ST_SHADER_MODULE_CREATE_INFO: u32 = 16;
const ST_COMMAND_POOL_CREATE_INFO: u32 = 39;
const ST_COMMAND_BUFFER_ALLOCATE_INFO: u32 = 40;
const ST_COMMAND_BUFFER_BEGIN_INFO: u32 = 42;

const QUEUE_COMPUTE_BIT: u32 = 0x2;
const BUFFER_USAGE_STORAGE_BUFFER: u32 = 0x20;
const MEM_HOST_VISIBLE: u32 = 0x2;
const MEM_HOST_COHERENT: u32 = 0x4;
const DESCRIPTOR_TYPE_STORAGE_BUFFER: u32 = 7;
const SHADER_STAGE_COMPUTE: u32 = 0x20;
const PIPELINE_BIND_POINT_COMPUTE: u32 = 1;
const CMD_BUFFER_LEVEL_PRIMARY: u32 = 0;
const CMD_BUFFER_USAGE_ONE_TIME_SUBMIT: u32 = 0x1;
const SHARING_MODE_EXCLUSIVE: u32 = 0;
const API_VERSION_1_1: u32 = 1 << 22; // VK_MAKE_API_VERSION(0,1,1,0)
const WHOLE_SIZE: u64 = u64::MAX;

// ── graphics 常量(RXS-0210,offscreen 出图) ────────────────────────────────
// sType。
const ST_IMAGE_CREATE_INFO: u32 = 14;
const ST_IMAGE_VIEW_CREATE_INFO: u32 = 15;
const ST_GRAPHICS_PIPELINE_CREATE_INFO: u32 = 28;
const ST_FRAMEBUFFER_CREATE_INFO: u32 = 37;
const ST_RENDER_PASS_CREATE_INFO: u32 = 38;
const ST_PIPELINE_VERTEX_INPUT_STATE_CI: u32 = 19;
const ST_PIPELINE_INPUT_ASSEMBLY_STATE_CI: u32 = 20;
const ST_PIPELINE_VIEWPORT_STATE_CI: u32 = 22;
const ST_PIPELINE_RASTERIZATION_STATE_CI: u32 = 23;
const ST_PIPELINE_MULTISAMPLE_STATE_CI: u32 = 24;
const ST_PIPELINE_COLOR_BLEND_STATE_CI: u32 = 26;
const ST_RENDER_PASS_BEGIN_INFO: u32 = 43;
const ST_IMAGE_MEMORY_BARRIER: u32 = 45;

const QUEUE_GRAPHICS_BIT: u32 = 0x1;
const MEM_DEVICE_LOCAL: u32 = 0x1;

const IMAGE_TYPE_2D: u32 = 1;
const IMAGE_VIEW_TYPE_2D: u32 = 1;
const IMAGE_TILING_OPTIMAL: u32 = 0;
const IMAGE_USAGE_TRANSFER_SRC: u32 = 0x1;
const IMAGE_USAGE_COLOR_ATTACHMENT: u32 = 0x10;
const BUFFER_USAGE_TRANSFER_DST: u32 = 0x2;
const BUFFER_USAGE_VERTEX: u32 = 0x80;
const FORMAT_R8G8B8A8_UNORM: u32 = 37;
// 注:顶点属性格式(如 R32G32B32A32_SFLOAT=109)由调用方(demo)按 Vulkan 枚举给定,
// 经 `attrs` 传入 run_graphics_offscreen,不在本模块常量化(避未用常量)。

const IMAGE_LAYOUT_UNDEFINED: u32 = 0;
const IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL: u32 = 2;
const IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL: u32 = 6;
const IMAGE_ASPECT_COLOR: u32 = 0x1;

const ATTACHMENT_LOAD_OP_CLEAR: u32 = 1;
const ATTACHMENT_LOAD_OP_DONT_CARE: u32 = 2;
const ATTACHMENT_STORE_OP_STORE: u32 = 0;
const ATTACHMENT_STORE_OP_DONT_CARE: u32 = 1;

const PIPELINE_BIND_POINT_GRAPHICS: u32 = 0;
const PRIMITIVE_TOPOLOGY_TRIANGLE_LIST: u32 = 3;
const SUBPASS_CONTENTS_INLINE: u32 = 0;
const SAMPLE_COUNT_1: u32 = 0x1;
const POLYGON_MODE_FILL: u32 = 0;
const CULL_MODE_NONE: u32 = 0;
const FRONT_FACE_COUNTER_CLOCKWISE: u32 = 0;
const VERTEX_INPUT_RATE_VERTEX: u32 = 0;
const COMPONENT_SWIZZLE_IDENTITY: u32 = 0;
const COLOR_COMPONENT_RGBA: u32 = 0xF;

const SHADER_STAGE_VERTEX: u32 = 0x1;
const SHADER_STAGE_FRAGMENT: u32 = 0x10;

// 屏障:color attachment 写 → transfer 读(access + pipeline stage 掩码)。
const ACCESS_COLOR_ATTACHMENT_WRITE: u32 = 0x100;
const ACCESS_TRANSFER_READ: u32 = 0x800;
const PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT: u32 = 0x400;
const PIPELINE_STAGE_TRANSFER: u32 = 0x1000;
const QUEUE_FAMILY_IGNORED: u32 = u32::MAX;

// ── #[repr(C)] 结构(布局与 Vulkan spec 逐字节对齐) ─────────────────────────

#[repr(C)]
struct ApplicationInfo {
    s_type: u32,
    p_next: *const c_void,
    p_application_name: *const c_char,
    application_version: u32,
    p_engine_name: *const c_char,
    engine_version: u32,
    api_version: u32,
}

#[repr(C)]
struct InstanceCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    p_application_info: *const ApplicationInfo,
    enabled_layer_count: u32,
    pp_enabled_layer_names: *const *const c_char,
    enabled_extension_count: u32,
    pp_enabled_extension_names: *const *const c_char,
}

#[repr(C)]
struct VkExtent3D {
    width: u32,
    height: u32,
    depth: u32,
}

#[repr(C)]
struct QueueFamilyProperties {
    queue_flags: VkFlags,
    queue_count: u32,
    timestamp_valid_bits: u32,
    min_image_transfer_granularity: VkExtent3D,
}

#[repr(C)]
struct DeviceQueueCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    queue_family_index: u32,
    queue_count: u32,
    p_queue_priorities: *const f32,
}

#[repr(C)]
struct DeviceCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    queue_create_info_count: u32,
    p_queue_create_infos: *const DeviceQueueCreateInfo,
    enabled_layer_count: u32,
    pp_enabled_layer_names: *const *const c_char,
    enabled_extension_count: u32,
    pp_enabled_extension_names: *const *const c_char,
    p_enabled_features: *const c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct MemoryType {
    property_flags: VkFlags,
    heap_index: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct MemoryHeap {
    size: VkDeviceSize,
    flags: VkFlags,
}

#[repr(C)]
struct PhysicalDeviceMemoryProperties {
    memory_type_count: u32,
    memory_types: [MemoryType; 32],
    memory_heap_count: u32,
    memory_heaps: [MemoryHeap; 16],
}

#[repr(C)]
struct BufferCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    size: VkDeviceSize,
    usage: VkFlags,
    sharing_mode: u32,
    queue_family_index_count: u32,
    p_queue_family_indices: *const u32,
}

#[repr(C)]
struct MemoryRequirements {
    size: VkDeviceSize,
    alignment: VkDeviceSize,
    memory_type_bits: u32,
}

#[repr(C)]
struct MemoryAllocateInfo {
    s_type: u32,
    p_next: *const c_void,
    allocation_size: VkDeviceSize,
    memory_type_index: u32,
}

#[repr(C)]
struct ShaderModuleCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    code_size: usize,
    p_code: *const u32,
}

#[repr(C)]
struct DescriptorSetLayoutBinding {
    binding: u32,
    descriptor_type: u32,
    descriptor_count: u32,
    stage_flags: VkFlags,
    p_immutable_samplers: *const c_void,
}

#[repr(C)]
struct DescriptorSetLayoutCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    binding_count: u32,
    p_bindings: *const DescriptorSetLayoutBinding,
}

#[repr(C)]
struct PushConstantRange {
    stage_flags: VkFlags,
    offset: u32,
    size: u32,
}

#[repr(C)]
struct PipelineLayoutCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    set_layout_count: u32,
    p_set_layouts: *const VkDescriptorSetLayout,
    push_constant_range_count: u32,
    p_push_constant_ranges: *const PushConstantRange,
}

#[repr(C)]
struct PipelineShaderStageCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    stage: VkFlags,
    module: VkShaderModule,
    p_name: *const c_char,
    p_specialization_info: *const c_void,
}

#[repr(C)]
struct ComputePipelineCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    stage: PipelineShaderStageCreateInfo,
    layout: VkPipelineLayout,
    base_pipeline_handle: VkPipeline,
    base_pipeline_index: i32,
}

#[repr(C)]
struct DescriptorPoolSize {
    descriptor_type: u32,
    descriptor_count: u32,
}

#[repr(C)]
struct DescriptorPoolCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    max_sets: u32,
    pool_size_count: u32,
    p_pool_sizes: *const DescriptorPoolSize,
}

#[repr(C)]
struct DescriptorSetAllocateInfo {
    s_type: u32,
    p_next: *const c_void,
    descriptor_pool: VkDescriptorPool,
    descriptor_set_count: u32,
    p_set_layouts: *const VkDescriptorSetLayout,
}

#[repr(C)]
struct DescriptorBufferInfo {
    buffer: VkBuffer,
    offset: VkDeviceSize,
    range: VkDeviceSize,
}

#[repr(C)]
struct WriteDescriptorSet {
    s_type: u32,
    p_next: *const c_void,
    dst_set: VkDescriptorSet,
    dst_binding: u32,
    dst_array_element: u32,
    descriptor_count: u32,
    descriptor_type: u32,
    p_image_info: *const c_void,
    p_buffer_info: *const DescriptorBufferInfo,
    p_texel_buffer_view: *const c_void,
}

#[repr(C)]
struct CommandPoolCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    queue_family_index: u32,
}

#[repr(C)]
struct CommandBufferAllocateInfo {
    s_type: u32,
    p_next: *const c_void,
    command_pool: VkCommandPool,
    level: u32,
    command_buffer_count: u32,
}

#[repr(C)]
struct CommandBufferBeginInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    p_inheritance_info: *const c_void,
}

#[repr(C)]
struct SubmitInfo {
    s_type: u32,
    p_next: *const c_void,
    wait_semaphore_count: u32,
    p_wait_semaphores: *const u64,
    p_wait_dst_stage_mask: *const VkFlags,
    command_buffer_count: u32,
    p_command_buffers: *const VkCommandBuffer,
    signal_semaphore_count: u32,
    p_signal_semaphores: *const u64,
}

// ── graphics #[repr(C)] 结构(RXS-0210;逐字节对齐,镜像上文风格) ────────────

#[repr(C)]
struct VkExtent2D {
    width: u32,
    height: u32,
}

#[repr(C)]
struct VkOffset2D {
    x: i32,
    y: i32,
}

#[repr(C)]
struct VkOffset3D {
    x: i32,
    y: i32,
    z: i32,
}

#[repr(C)]
struct VkRect2D {
    offset: VkOffset2D,
    extent: VkExtent2D,
}

#[repr(C)]
struct VkViewport {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    min_depth: f32,
    max_depth: f32,
}

#[repr(C)]
struct VkComponentMapping {
    r: u32,
    g: u32,
    b: u32,
    a: u32,
}

#[repr(C)]
struct VkImageSubresourceRange {
    aspect_mask: VkFlags,
    base_mip_level: u32,
    level_count: u32,
    base_array_layer: u32,
    layer_count: u32,
}

#[repr(C)]
struct VkImageSubresourceLayers {
    aspect_mask: VkFlags,
    mip_level: u32,
    base_array_layer: u32,
    layer_count: u32,
}

#[repr(C)]
struct ImageCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    image_type: u32,
    format: u32,
    extent: VkExtent3D,
    mip_levels: u32,
    array_layers: u32,
    samples: VkFlags,
    tiling: u32,
    usage: VkFlags,
    sharing_mode: u32,
    queue_family_index_count: u32,
    p_queue_family_indices: *const u32,
    initial_layout: u32,
}

#[repr(C)]
struct ImageViewCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    image: VkImage,
    view_type: u32,
    format: u32,
    components: VkComponentMapping,
    subresource_range: VkImageSubresourceRange,
}

#[repr(C)]
struct AttachmentDescription {
    flags: VkFlags,
    format: u32,
    samples: VkFlags,
    load_op: u32,
    store_op: u32,
    stencil_load_op: u32,
    stencil_store_op: u32,
    initial_layout: u32,
    final_layout: u32,
}

#[repr(C)]
struct AttachmentReference {
    attachment: u32,
    layout: u32,
}

#[repr(C)]
struct SubpassDescription {
    flags: VkFlags,
    pipeline_bind_point: u32,
    input_attachment_count: u32,
    p_input_attachments: *const AttachmentReference,
    color_attachment_count: u32,
    p_color_attachments: *const AttachmentReference,
    p_resolve_attachments: *const AttachmentReference,
    p_depth_stencil_attachment: *const AttachmentReference,
    preserve_attachment_count: u32,
    p_preserve_attachments: *const u32,
}

#[repr(C)]
struct RenderPassCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    attachment_count: u32,
    p_attachments: *const AttachmentDescription,
    subpass_count: u32,
    p_subpasses: *const SubpassDescription,
    dependency_count: u32,
    p_dependencies: *const c_void,
}

#[repr(C)]
struct FramebufferCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    render_pass: VkRenderPass,
    attachment_count: u32,
    p_attachments: *const VkImageView,
    width: u32,
    height: u32,
    layers: u32,
}

#[repr(C)]
struct VkVertexInputBindingDescription {
    binding: u32,
    stride: u32,
    input_rate: u32,
}

#[repr(C)]
struct VkVertexInputAttributeDescription {
    location: u32,
    binding: u32,
    format: u32,
    offset: u32,
}

#[repr(C)]
struct PipelineVertexInputStateCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    vertex_binding_description_count: u32,
    p_vertex_binding_descriptions: *const VkVertexInputBindingDescription,
    vertex_attribute_description_count: u32,
    p_vertex_attribute_descriptions: *const VkVertexInputAttributeDescription,
}

#[repr(C)]
struct PipelineInputAssemblyStateCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    topology: u32,
    primitive_restart_enable: u32,
}

#[repr(C)]
struct PipelineViewportStateCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    viewport_count: u32,
    p_viewports: *const VkViewport,
    scissor_count: u32,
    p_scissors: *const VkRect2D,
}

#[repr(C)]
struct PipelineRasterizationStateCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    depth_clamp_enable: u32,
    rasterizer_discard_enable: u32,
    polygon_mode: u32,
    cull_mode: VkFlags,
    front_face: u32,
    depth_bias_enable: u32,
    depth_bias_constant_factor: f32,
    depth_bias_clamp: f32,
    depth_bias_slope_factor: f32,
    line_width: f32,
}

#[repr(C)]
struct PipelineMultisampleStateCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    rasterization_samples: VkFlags,
    sample_shading_enable: u32,
    min_sample_shading: f32,
    p_sample_mask: *const u32,
    alpha_to_coverage_enable: u32,
    alpha_to_one_enable: u32,
}

#[repr(C)]
struct PipelineColorBlendAttachmentState {
    blend_enable: u32,
    src_color_blend_factor: u32,
    dst_color_blend_factor: u32,
    color_blend_op: u32,
    src_alpha_blend_factor: u32,
    dst_alpha_blend_factor: u32,
    alpha_blend_op: u32,
    color_write_mask: VkFlags,
}

#[repr(C)]
struct PipelineColorBlendStateCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    logic_op_enable: u32,
    logic_op: u32,
    attachment_count: u32,
    p_attachments: *const PipelineColorBlendAttachmentState,
    blend_constants: [f32; 4],
}

#[repr(C)]
struct GraphicsPipelineCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    stage_count: u32,
    p_stages: *const PipelineShaderStageCreateInfo,
    p_vertex_input_state: *const PipelineVertexInputStateCreateInfo,
    p_input_assembly_state: *const PipelineInputAssemblyStateCreateInfo,
    p_tessellation_state: *const c_void,
    p_viewport_state: *const PipelineViewportStateCreateInfo,
    p_rasterization_state: *const PipelineRasterizationStateCreateInfo,
    p_multisample_state: *const PipelineMultisampleStateCreateInfo,
    p_depth_stencil_state: *const c_void,
    p_color_blend_state: *const PipelineColorBlendStateCreateInfo,
    p_dynamic_state: *const c_void,
    layout: VkPipelineLayout,
    render_pass: VkRenderPass,
    subpass: u32,
    base_pipeline_handle: VkPipeline,
    base_pipeline_index: i32,
}

/// `VkClearValue` union(填 color float32[4];union 尺寸 = 16 字节 = `[f32;4]`)。
#[repr(C)]
struct ClearValue {
    color: [f32; 4],
}

#[repr(C)]
struct RenderPassBeginInfo {
    s_type: u32,
    p_next: *const c_void,
    render_pass: VkRenderPass,
    framebuffer: VkFramebuffer,
    render_area: VkRect2D,
    clear_value_count: u32,
    p_clear_values: *const ClearValue,
}

#[repr(C)]
struct ImageMemoryBarrier {
    s_type: u32,
    p_next: *const c_void,
    src_access_mask: VkFlags,
    dst_access_mask: VkFlags,
    old_layout: u32,
    new_layout: u32,
    src_queue_family_index: u32,
    dst_queue_family_index: u32,
    image: VkImage,
    subresource_range: VkImageSubresourceRange,
}

#[repr(C)]
struct VkBufferImageCopy {
    buffer_offset: VkDeviceSize,
    buffer_row_length: u32,
    buffer_image_height: u32,
    image_subresource: VkImageSubresourceLayers,
    image_offset: VkOffset3D,
    image_extent: VkExtent3D,
}

// ── 函数指针类型 ────────────────────────────────────────────────────────────

type PfnVoid = unsafe extern "system" fn();
type FnGetInstanceProcAddr =
    unsafe extern "system" fn(VkInstance, *const c_char) -> Option<PfnVoid>;
type FnGetDeviceProcAddr = unsafe extern "system" fn(VkDevice, *const c_char) -> Option<PfnVoid>;
type FnCreateInstance = unsafe extern "system" fn(
    *const InstanceCreateInfo,
    *const c_void,
    *mut VkInstance,
) -> VkResult;
type FnDestroyInstance = unsafe extern "system" fn(VkInstance, *const c_void);
type FnEnumeratePhysicalDevices =
    unsafe extern "system" fn(VkInstance, *mut u32, *mut VkPhysicalDevice) -> VkResult;
type FnGetPhysicalDeviceQueueFamilyProperties =
    unsafe extern "system" fn(VkPhysicalDevice, *mut u32, *mut QueueFamilyProperties);
type FnGetPhysicalDeviceMemoryProperties =
    unsafe extern "system" fn(VkPhysicalDevice, *mut PhysicalDeviceMemoryProperties);
type FnCreateDevice = unsafe extern "system" fn(
    VkPhysicalDevice,
    *const DeviceCreateInfo,
    *const c_void,
    *mut VkDevice,
) -> VkResult;
type FnDestroyDevice = unsafe extern "system" fn(VkDevice, *const c_void);
type FnGetDeviceQueue = unsafe extern "system" fn(VkDevice, u32, u32, *mut VkQueue);
type FnCreateBuffer = unsafe extern "system" fn(
    VkDevice,
    *const BufferCreateInfo,
    *const c_void,
    *mut VkBuffer,
) -> VkResult;
type FnDestroyBuffer = unsafe extern "system" fn(VkDevice, VkBuffer, *const c_void);
type FnGetBufferMemoryRequirements =
    unsafe extern "system" fn(VkDevice, VkBuffer, *mut MemoryRequirements);
type FnAllocateMemory = unsafe extern "system" fn(
    VkDevice,
    *const MemoryAllocateInfo,
    *const c_void,
    *mut VkDeviceMemory,
) -> VkResult;
type FnFreeMemory = unsafe extern "system" fn(VkDevice, VkDeviceMemory, *const c_void);
type FnBindBufferMemory =
    unsafe extern "system" fn(VkDevice, VkBuffer, VkDeviceMemory, VkDeviceSize) -> VkResult;
type FnMapMemory = unsafe extern "system" fn(
    VkDevice,
    VkDeviceMemory,
    VkDeviceSize,
    VkDeviceSize,
    VkFlags,
    *mut *mut c_void,
) -> VkResult;
type FnUnmapMemory = unsafe extern "system" fn(VkDevice, VkDeviceMemory);
type FnCreateShaderModule = unsafe extern "system" fn(
    VkDevice,
    *const ShaderModuleCreateInfo,
    *const c_void,
    *mut VkShaderModule,
) -> VkResult;
type FnDestroyShaderModule = unsafe extern "system" fn(VkDevice, VkShaderModule, *const c_void);
type FnCreateDescriptorSetLayout = unsafe extern "system" fn(
    VkDevice,
    *const DescriptorSetLayoutCreateInfo,
    *const c_void,
    *mut VkDescriptorSetLayout,
) -> VkResult;
type FnDestroyDescriptorSetLayout =
    unsafe extern "system" fn(VkDevice, VkDescriptorSetLayout, *const c_void);
type FnCreatePipelineLayout = unsafe extern "system" fn(
    VkDevice,
    *const PipelineLayoutCreateInfo,
    *const c_void,
    *mut VkPipelineLayout,
) -> VkResult;
type FnDestroyPipelineLayout = unsafe extern "system" fn(VkDevice, VkPipelineLayout, *const c_void);
type FnCreateComputePipelines = unsafe extern "system" fn(
    VkDevice,
    u64,
    u32,
    *const ComputePipelineCreateInfo,
    *const c_void,
    *mut VkPipeline,
) -> VkResult;
type FnDestroyPipeline = unsafe extern "system" fn(VkDevice, VkPipeline, *const c_void);
type FnCreateDescriptorPool = unsafe extern "system" fn(
    VkDevice,
    *const DescriptorPoolCreateInfo,
    *const c_void,
    *mut VkDescriptorPool,
) -> VkResult;
type FnDestroyDescriptorPool = unsafe extern "system" fn(VkDevice, VkDescriptorPool, *const c_void);
type FnAllocateDescriptorSets = unsafe extern "system" fn(
    VkDevice,
    *const DescriptorSetAllocateInfo,
    *mut VkDescriptorSet,
) -> VkResult;
type FnUpdateDescriptorSets =
    unsafe extern "system" fn(VkDevice, u32, *const WriteDescriptorSet, u32, *const c_void);
type FnCreateCommandPool = unsafe extern "system" fn(
    VkDevice,
    *const CommandPoolCreateInfo,
    *const c_void,
    *mut VkCommandPool,
) -> VkResult;
type FnDestroyCommandPool = unsafe extern "system" fn(VkDevice, VkCommandPool, *const c_void);
type FnAllocateCommandBuffers = unsafe extern "system" fn(
    VkDevice,
    *const CommandBufferAllocateInfo,
    *mut VkCommandBuffer,
) -> VkResult;
type FnBeginCommandBuffer =
    unsafe extern "system" fn(VkCommandBuffer, *const CommandBufferBeginInfo) -> VkResult;
type FnEndCommandBuffer = unsafe extern "system" fn(VkCommandBuffer) -> VkResult;
type FnCmdBindPipeline = unsafe extern "system" fn(VkCommandBuffer, u32, VkPipeline);
type FnCmdBindDescriptorSets = unsafe extern "system" fn(
    VkCommandBuffer,
    u32,
    VkPipelineLayout,
    u32,
    u32,
    *const VkDescriptorSet,
    u32,
    *const u32,
);
type FnCmdPushConstants =
    unsafe extern "system" fn(VkCommandBuffer, VkPipelineLayout, VkFlags, u32, u32, *const c_void);
type FnCmdDispatch = unsafe extern "system" fn(VkCommandBuffer, u32, u32, u32);
type FnQueueSubmit = unsafe extern "system" fn(VkQueue, u32, *const SubmitInfo, u64) -> VkResult;
type FnQueueWaitIdle = unsafe extern "system" fn(VkQueue) -> VkResult;

// graphics 函数指针(RXS-0210)。
type FnCreateImage = unsafe extern "system" fn(
    VkDevice,
    *const ImageCreateInfo,
    *const c_void,
    *mut VkImage,
) -> VkResult;
type FnDestroyImage = unsafe extern "system" fn(VkDevice, VkImage, *const c_void);
type FnGetImageMemoryRequirements =
    unsafe extern "system" fn(VkDevice, VkImage, *mut MemoryRequirements);
type FnBindImageMemory =
    unsafe extern "system" fn(VkDevice, VkImage, VkDeviceMemory, VkDeviceSize) -> VkResult;
type FnCreateImageView = unsafe extern "system" fn(
    VkDevice,
    *const ImageViewCreateInfo,
    *const c_void,
    *mut VkImageView,
) -> VkResult;
type FnDestroyImageView = unsafe extern "system" fn(VkDevice, VkImageView, *const c_void);
type FnCreateRenderPass = unsafe extern "system" fn(
    VkDevice,
    *const RenderPassCreateInfo,
    *const c_void,
    *mut VkRenderPass,
) -> VkResult;
type FnDestroyRenderPass = unsafe extern "system" fn(VkDevice, VkRenderPass, *const c_void);
type FnCreateFramebuffer = unsafe extern "system" fn(
    VkDevice,
    *const FramebufferCreateInfo,
    *const c_void,
    *mut VkFramebuffer,
) -> VkResult;
type FnDestroyFramebuffer = unsafe extern "system" fn(VkDevice, VkFramebuffer, *const c_void);
type FnCreateGraphicsPipelines = unsafe extern "system" fn(
    VkDevice,
    u64,
    u32,
    *const GraphicsPipelineCreateInfo,
    *const c_void,
    *mut VkPipeline,
) -> VkResult;
type FnCmdBeginRenderPass =
    unsafe extern "system" fn(VkCommandBuffer, *const RenderPassBeginInfo, u32);
type FnCmdEndRenderPass = unsafe extern "system" fn(VkCommandBuffer);
type FnCmdBindVertexBuffers =
    unsafe extern "system" fn(VkCommandBuffer, u32, u32, *const VkBuffer, *const VkDeviceSize);
type FnCmdDraw = unsafe extern "system" fn(VkCommandBuffer, u32, u32, u32, u32);
type FnCmdPipelineBarrier = unsafe extern "system" fn(
    VkCommandBuffer,
    VkFlags,
    VkFlags,
    VkFlags,
    u32,
    *const c_void,
    u32,
    *const c_void,
    u32,
    *const ImageMemoryBarrier,
);
type FnCmdCopyImageToBuffer = unsafe extern "system" fn(
    VkCommandBuffer,
    VkImage,
    u32,
    VkBuffer,
    u32,
    *const VkBufferImageCopy,
);

// ── OS 动态加载缝(跨端;镜像 sys.rs 无外部依赖纪律) ───────────────────────────
// Windows:      vulkan-1.dll  / LoadLibraryA + GetProcAddress(Win32 kernel32)。
// Android+Linux: libvulkan.so / dlopen(RTLD_NOW) + dlsym(libc;Android 由 libc 直接
//                提供 dlopen/dlsym,NDK 默认链接;现代 glibc 亦并入 libc,无需 -ldl)。
#[cfg(windows)]
mod loader {
    use core::ffi::{CStr, c_char, c_void};
    unsafe extern "system" {
        fn LoadLibraryA(name: *const c_char) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const c_char) -> *mut c_void;
    }
    pub(super) const VULKAN_LIB: &CStr = c"vulkan-1.dll";
    /// # Safety
    /// `name` 为 NUL 结尾字面量。
    pub(super) unsafe fn open(name: *const c_char) -> *mut c_void {
        LoadLibraryA(name)
    }
    /// # Safety
    /// `lib` 为 `open` 返回的有效模块句柄或 null;`name` NUL 结尾。
    pub(super) unsafe fn sym(lib: *mut c_void, name: *const c_char) -> *mut c_void {
        GetProcAddress(lib, name)
    }
}

#[cfg(not(windows))]
mod loader {
    use core::ffi::{CStr, c_char, c_void};
    unsafe extern "C" {
        fn dlopen(filename: *const c_char, flag: i32) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    }
    const RTLD_NOW: i32 = 2; // 立即绑定全部符号(POSIX 通用值,Android/glibc/musl 一致)。
    pub(super) const VULKAN_LIB: &CStr = c"libvulkan.so";
    /// # Safety
    /// `name` 为 NUL 结尾字面量。
    pub(super) unsafe fn open(name: *const c_char) -> *mut c_void {
        dlopen(name, RTLD_NOW)
    }
    /// # Safety
    /// `handle` 为 `open` 返回的有效句柄或 null;`name` NUL 结尾。
    pub(super) unsafe fn sym(handle: *mut c_void, name: *const c_char) -> *mut c_void {
        dlsym(handle, name)
    }
}

/// null 校验后转函数指针(镜像 sys::cast_fn)。
///
/// # Safety
/// `raw` 须为 `T`(匹配 ABI 的函数指针类型)对应的有效符号地址或 null。
unsafe fn cast_fn<T: Copy>(raw: Option<PfnVoid>) -> Option<T> {
    let p = raw? as *const c_void;
    if p.is_null() {
        return None;
    }
    debug_assert_eq!(size_of::<T>(), size_of::<*const c_void>());
    // SAFETY: 调用方保证 raw 对应 T 的 ABI;size 断言防误配。
    Some(unsafe { std::mem::transmute_copy::<*const c_void, T>(&p) })
}

fn load_vulkan_loader() -> Option<FnGetInstanceProcAddr> {
    // SAFETY: open/sym 为各 OS 稳定 ABI 加载原语(Win32 LoadLibraryA / POSIX dlopen);
    // 入参 NUL 结尾字面量;返回地址经 null 校验后 transmute 为已知 ABI 的函数指针。
    // loader 不 close/FreeLibrary —— 进程生命周期常驻(镜像 sys.rs U1 nvcuda.dll 纪律)。
    unsafe {
        let lib = loader::open(loader::VULKAN_LIB.as_ptr());
        if lib.is_null() {
            return None;
        }
        let p = loader::sym(lib, c"vkGetInstanceProcAddr".as_ptr());
        if p.is_null() {
            return None;
        }
        Some(std::mem::transmute::<*mut c_void, FnGetInstanceProcAddr>(p))
    }
}

/// 解析 SPIR-V 首个 `OpEntryPoint` 的入口名(codegen 用 mangled 符号名;Vulkan pipeline
/// 的 `pName` 需与之一致)。header 5 字后扫指令流,opcode 15 = OpEntryPoint,operand
/// [exec_model, entry_id, name(NUL 终止)..]。
pub fn entry_point_name(spv: &[u32]) -> Option<String> {
    if spv.len() < 5 {
        return None;
    }
    let mut i = 5;
    while i < spv.len() {
        let wc = (spv[i] >> 16) as usize;
        let op = (spv[i] & 0xffff) as u16;
        if wc == 0 {
            break;
        }
        if op == 15 && i + 3 <= spv.len() {
            let end = (i + wc).min(spv.len());
            let mut bytes = Vec::new();
            'outer: for w in &spv[i + 3..end] {
                for k in 0..4 {
                    let b = ((w >> (8 * k)) & 0xff) as u8;
                    if b == 0 {
                        break 'outer;
                    }
                    bytes.push(b);
                }
            }
            return String::from_utf8(bytes).ok();
        }
        i += wc;
    }
    None
}

/// 从 SPIR-V 字流跑一个 compute shader,同步执行后回读所有 buffer。
///
/// - `spv`:Phase 1 `--target vulkan` 产 SPIR-V 字流。
/// - `entry`:`OpEntryPoint` 名(codegen 用 mangled 符号名)。
/// - `buffers[i]`:StorageBuffer 绑定 (set 0, binding i) 的 host 数据(in/out,原位回写)。
/// - `push_constants`:push constant 块字节(shader 布局:标量顺排,4 字节对齐)。
/// - `groups`:`vkCmdDispatch` 工作组数([x,y,z])。
///
/// host-visible+coherent 内存(免 flush)+ 单 queue 同步(`vkQueueWaitIdle`)。
pub fn run_compute(
    spv: &[u32],
    entry: &str,
    buffers: &mut [Vec<u8>],
    push_constants: &[u8],
    groups: [u32; 3],
) -> Result<(), String> {
    let gipa = load_vulkan_loader().ok_or("vulkan loader (vulkan-1.dll/libvulkan.so) 不可用")?;
    // SAFETY: 全程手写 Vulkan FFI;句柄生命周期由本函数线性管理,末尾逆序销毁。
    // 每个 create/destroy 配对;结构布局与 Vulkan spec 逐字节对齐(见上 #[repr(C)])。
    unsafe { run_compute_inner(gipa, spv, entry, buffers, push_constants, groups) }
}

/// StorageBuffer 句柄 + 其 host-visible 内存(线性生命周期,末尾逆序销毁)。
struct Buf {
    buffer: VkBuffer,
    mem: VkDeviceMemory,
}

unsafe fn run_compute_inner(
    gipa: FnGetInstanceProcAddr,
    spv: &[u32],
    entry: &str,
    buffers: &mut [Vec<u8>],
    push_constants: &[u8],
    groups: [u32; 3],
) -> Result<(), String> {
    // 全局级符号(instance=null)。
    let vk_create_instance: FnCreateInstance =
        cast_fn(gipa(std::ptr::null_mut(), c"vkCreateInstance".as_ptr()))
            .ok_or("缺 vkCreateInstance")?;

    let validation = std::env::var("RURIX_VK_VALIDATION").as_deref() == Ok("1");
    let layer_name = c"VK_LAYER_KHRONOS_validation";
    let layers: [*const c_char; 1] = [layer_name.as_ptr()];
    let app = ApplicationInfo {
        s_type: ST_APPLICATION_INFO,
        p_next: std::ptr::null(),
        p_application_name: c"rurix-mb1".as_ptr(),
        application_version: 0,
        p_engine_name: c"rurix".as_ptr(),
        engine_version: 0,
        api_version: API_VERSION_1_1,
    };
    let ici = InstanceCreateInfo {
        s_type: ST_INSTANCE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        p_application_info: &app,
        enabled_layer_count: if validation { 1 } else { 0 },
        pp_enabled_layer_names: if validation {
            layers.as_ptr()
        } else {
            std::ptr::null()
        },
        enabled_extension_count: 0,
        pp_enabled_extension_names: std::ptr::null(),
    };
    let mut instance: VkInstance = std::ptr::null_mut();
    let r = vk_create_instance(&ici, std::ptr::null(), &mut instance);
    if r != VK_SUCCESS {
        return Err(format!("vkCreateInstance 失败: {r}"));
    }

    // instance 级符号。
    let vk_destroy_instance: FnDestroyInstance =
        cast_fn(gipa(instance, c"vkDestroyInstance".as_ptr())).ok_or("缺 vkDestroyInstance")?;
    let vk_enum_pd: FnEnumeratePhysicalDevices =
        cast_fn(gipa(instance, c"vkEnumeratePhysicalDevices".as_ptr()))
            .ok_or("缺 vkEnumeratePhysicalDevices")?;
    let vk_get_qf: FnGetPhysicalDeviceQueueFamilyProperties = cast_fn(gipa(
        instance,
        c"vkGetPhysicalDeviceQueueFamilyProperties".as_ptr(),
    ))
    .ok_or("缺 vkGetPhysicalDeviceQueueFamilyProperties")?;
    let vk_get_mem: FnGetPhysicalDeviceMemoryProperties = cast_fn(gipa(
        instance,
        c"vkGetPhysicalDeviceMemoryProperties".as_ptr(),
    ))
    .ok_or("缺 vkGetPhysicalDeviceMemoryProperties")?;
    let vk_create_device: FnCreateDevice =
        cast_fn(gipa(instance, c"vkCreateDevice".as_ptr())).ok_or("缺 vkCreateDevice")?;
    let vk_get_device_proc: FnGetDeviceProcAddr =
        cast_fn(gipa(instance, c"vkGetDeviceProcAddr".as_ptr())).ok_or("缺 vkGetDeviceProcAddr")?;

    // 枚举物理设备,取首个。
    let mut count = 0u32;
    vk_enum_pd(instance, &mut count, std::ptr::null_mut());
    if count == 0 {
        vk_destroy_instance(instance, std::ptr::null());
        return Err("无 Vulkan 物理设备".into());
    }
    let mut pds = vec![std::ptr::null_mut::<c_void>(); count as usize];
    vk_enum_pd(instance, &mut count, pds.as_mut_ptr());
    let pd = pds[0];

    // 找 compute queue family。
    let mut qf_count = 0u32;
    vk_get_qf(pd, &mut qf_count, std::ptr::null_mut());
    let mut qfs: Vec<QueueFamilyProperties> = (0..qf_count)
        .map(|_| QueueFamilyProperties {
            queue_flags: 0,
            queue_count: 0,
            timestamp_valid_bits: 0,
            min_image_transfer_granularity: VkExtent3D {
                width: 0,
                height: 0,
                depth: 0,
            },
        })
        .collect();
    vk_get_qf(pd, &mut qf_count, qfs.as_mut_ptr());
    let qfi = qfs
        .iter()
        .position(|q| q.queue_flags & QUEUE_COMPUTE_BIT != 0)
        .ok_or("无 compute queue family")? as u32;

    // 创建 device + queue。
    let prio = [1.0f32];
    let dqci = DeviceQueueCreateInfo {
        s_type: ST_DEVICE_QUEUE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        queue_family_index: qfi,
        queue_count: 1,
        p_queue_priorities: prio.as_ptr(),
    };
    let dci = DeviceCreateInfo {
        s_type: ST_DEVICE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        queue_create_info_count: 1,
        p_queue_create_infos: &dqci,
        enabled_layer_count: 0,
        pp_enabled_layer_names: std::ptr::null(),
        enabled_extension_count: 0,
        pp_enabled_extension_names: std::ptr::null(),
        p_enabled_features: std::ptr::null(),
    };
    let mut device: VkDevice = std::ptr::null_mut();
    let r = vk_create_device(pd, &dci, std::ptr::null(), &mut device);
    if r != VK_SUCCESS {
        vk_destroy_instance(instance, std::ptr::null());
        return Err(format!("vkCreateDevice 失败: {r}"));
    }

    // device 级符号加载 + 主流程。
    let out = run_on_device(
        vk_get_device_proc,
        device,
        pd,
        vk_get_mem,
        qfi,
        spv,
        entry,
        buffers,
        push_constants,
        groups,
    );

    // 销毁 device / instance(device 级 destroy 经 device-proc 已在 run_on_device 内做完 body)。
    let vk_destroy_device: Option<FnDestroyDevice> =
        cast_fn(vk_get_device_proc(device, c"vkDestroyDevice".as_ptr()));
    if let Some(dd) = vk_destroy_device {
        dd(device, std::ptr::null());
    }
    vk_destroy_instance(instance, std::ptr::null());
    out
}

#[allow(clippy::too_many_arguments)]
unsafe fn run_on_device(
    gdpa: FnGetDeviceProcAddr,
    device: VkDevice,
    _pd: VkPhysicalDevice,
    vk_get_mem: FnGetPhysicalDeviceMemoryProperties,
    qfi: u32,
    spv: &[u32],
    entry: &str,
    buffers: &mut [Vec<u8>],
    push_constants: &[u8],
    groups: [u32; 3],
) -> Result<(), String> {
    macro_rules! dp {
        ($name:literal, $ty:ty) => {
            cast_fn::<$ty>(gdpa(device, $name.as_ptr())).ok_or("缺 device 符号")?
        };
    }
    let get_queue: FnGetDeviceQueue = dp!(c"vkGetDeviceQueue", FnGetDeviceQueue);
    let create_buffer: FnCreateBuffer = dp!(c"vkCreateBuffer", FnCreateBuffer);
    let destroy_buffer: FnDestroyBuffer = dp!(c"vkDestroyBuffer", FnDestroyBuffer);
    let buf_mem_req: FnGetBufferMemoryRequirements = dp!(
        c"vkGetBufferMemoryRequirements",
        FnGetBufferMemoryRequirements
    );
    let alloc_mem: FnAllocateMemory = dp!(c"vkAllocateMemory", FnAllocateMemory);
    let free_mem: FnFreeMemory = dp!(c"vkFreeMemory", FnFreeMemory);
    let bind_buf: FnBindBufferMemory = dp!(c"vkBindBufferMemory", FnBindBufferMemory);
    let map_mem: FnMapMemory = dp!(c"vkMapMemory", FnMapMemory);
    let unmap_mem: FnUnmapMemory = dp!(c"vkUnmapMemory", FnUnmapMemory);
    let create_shader: FnCreateShaderModule = dp!(c"vkCreateShaderModule", FnCreateShaderModule);
    let destroy_shader: FnDestroyShaderModule =
        dp!(c"vkDestroyShaderModule", FnDestroyShaderModule);
    let create_dsl: FnCreateDescriptorSetLayout =
        dp!(c"vkCreateDescriptorSetLayout", FnCreateDescriptorSetLayout);
    let destroy_dsl: FnDestroyDescriptorSetLayout = dp!(
        c"vkDestroyDescriptorSetLayout",
        FnDestroyDescriptorSetLayout
    );
    let create_pl: FnCreatePipelineLayout = dp!(c"vkCreatePipelineLayout", FnCreatePipelineLayout);
    let destroy_pl: FnDestroyPipelineLayout =
        dp!(c"vkDestroyPipelineLayout", FnDestroyPipelineLayout);
    let create_cp: FnCreateComputePipelines =
        dp!(c"vkCreateComputePipelines", FnCreateComputePipelines);
    let destroy_pipe: FnDestroyPipeline = dp!(c"vkDestroyPipeline", FnDestroyPipeline);
    let create_dp: FnCreateDescriptorPool = dp!(c"vkCreateDescriptorPool", FnCreateDescriptorPool);
    let destroy_dp: FnDestroyDescriptorPool =
        dp!(c"vkDestroyDescriptorPool", FnDestroyDescriptorPool);
    let alloc_ds: FnAllocateDescriptorSets =
        dp!(c"vkAllocateDescriptorSets", FnAllocateDescriptorSets);
    let update_ds: FnUpdateDescriptorSets = dp!(c"vkUpdateDescriptorSets", FnUpdateDescriptorSets);
    let create_cmdpool: FnCreateCommandPool = dp!(c"vkCreateCommandPool", FnCreateCommandPool);
    let destroy_cmdpool: FnDestroyCommandPool = dp!(c"vkDestroyCommandPool", FnDestroyCommandPool);
    let alloc_cmd: FnAllocateCommandBuffers =
        dp!(c"vkAllocateCommandBuffers", FnAllocateCommandBuffers);
    let begin_cmd: FnBeginCommandBuffer = dp!(c"vkBeginCommandBuffer", FnBeginCommandBuffer);
    let end_cmd: FnEndCommandBuffer = dp!(c"vkEndCommandBuffer", FnEndCommandBuffer);
    let cmd_bind_pipe: FnCmdBindPipeline = dp!(c"vkCmdBindPipeline", FnCmdBindPipeline);
    let cmd_bind_ds: FnCmdBindDescriptorSets =
        dp!(c"vkCmdBindDescriptorSets", FnCmdBindDescriptorSets);
    let cmd_push: FnCmdPushConstants = dp!(c"vkCmdPushConstants", FnCmdPushConstants);
    let cmd_dispatch: FnCmdDispatch = dp!(c"vkCmdDispatch", FnCmdDispatch);
    let queue_submit: FnQueueSubmit = dp!(c"vkQueueSubmit", FnQueueSubmit);
    let queue_wait: FnQueueWaitIdle = dp!(c"vkQueueWaitIdle", FnQueueWaitIdle);

    let mut queue: VkQueue = std::ptr::null_mut();
    get_queue(device, qfi, 0, &mut queue);

    // 内存类型属性(选 host-visible + coherent)。
    let mut memprops = std::mem::zeroed::<PhysicalDeviceMemoryProperties>();
    vk_get_mem(_pd, &mut memprops);
    let pick_memtype = |type_bits: u32| -> Option<u32> {
        (0..memprops.memory_type_count).find(|&i| {
            let mt = memprops.memory_types[i as usize];
            type_bits & (1 << i) != 0
                && mt.property_flags & (MEM_HOST_VISIBLE | MEM_HOST_COHERENT)
                    == (MEM_HOST_VISIBLE | MEM_HOST_COHERENT)
        })
    };

    // 每 buffer:create + alloc host-visible mem + bind + 上传。
    let mut bufs: Vec<Buf> = Vec::new();
    let mut cleanup_err: Option<String> = None;
    'setup: {
        for host in buffers.iter() {
            let size = host.len().max(4) as u64;
            let bci = BufferCreateInfo {
                s_type: ST_BUFFER_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                size,
                usage: BUFFER_USAGE_STORAGE_BUFFER,
                sharing_mode: SHARING_MODE_EXCLUSIVE,
                queue_family_index_count: 0,
                p_queue_family_indices: std::ptr::null(),
            };
            let mut buffer: VkBuffer = VK_NULL_HANDLE;
            if create_buffer(device, &bci, std::ptr::null(), &mut buffer) != VK_SUCCESS {
                cleanup_err = Some("vkCreateBuffer 失败".into());
                break 'setup;
            }
            let mut req = std::mem::zeroed::<MemoryRequirements>();
            buf_mem_req(device, buffer, &mut req);
            let Some(mt) = pick_memtype(req.memory_type_bits) else {
                cleanup_err = Some("无 host-visible+coherent 内存类型".into());
                break 'setup;
            };
            let mai = MemoryAllocateInfo {
                s_type: ST_MEMORY_ALLOCATE_INFO,
                p_next: std::ptr::null(),
                allocation_size: req.size,
                memory_type_index: mt,
            };
            let mut mem: VkDeviceMemory = VK_NULL_HANDLE;
            if alloc_mem(device, &mai, std::ptr::null(), &mut mem) != VK_SUCCESS {
                cleanup_err = Some("vkAllocateMemory 失败".into());
                break 'setup;
            }
            bind_buf(device, buffer, mem, 0);
            // 上传 host 数据。
            let mut ptr: *mut c_void = std::ptr::null_mut();
            if map_mem(device, mem, 0, WHOLE_SIZE, 0, &mut ptr) != VK_SUCCESS {
                cleanup_err = Some("vkMapMemory 失败".into());
                break 'setup;
            }
            std::ptr::copy_nonoverlapping(host.as_ptr(), ptr.cast::<u8>(), host.len());
            unmap_mem(device, mem);
            let _ = size;
            bufs.push(Buf { buffer, mem });
        }
    }

    let result = if let Some(e) = cleanup_err {
        Err(e)
    } else {
        dispatch_and_readback(
            device,
            queue,
            spv,
            entry,
            &bufs,
            push_constants,
            groups,
            qfi,
            buffers,
            &create_shader,
            &destroy_shader,
            &create_dsl,
            &destroy_dsl,
            &create_pl,
            &destroy_pl,
            &create_cp,
            &destroy_pipe,
            &create_dp,
            &destroy_dp,
            &alloc_ds,
            &update_ds,
            &create_cmdpool,
            &destroy_cmdpool,
            &alloc_cmd,
            &begin_cmd,
            &end_cmd,
            &cmd_bind_pipe,
            &cmd_bind_ds,
            &cmd_push,
            &cmd_dispatch,
            &queue_submit,
            &queue_wait,
            &map_mem,
            &unmap_mem,
        )
    };

    // buffer/mem 清理。
    for b in &bufs {
        destroy_buffer(device, b.buffer, std::ptr::null());
        free_mem(device, b.mem, std::ptr::null());
    }
    result
}

#[allow(clippy::too_many_arguments)]
unsafe fn dispatch_and_readback(
    device: VkDevice,
    queue: VkQueue,
    spv: &[u32],
    entry: &str,
    bufs: &[Buf],
    push_constants: &[u8],
    groups: [u32; 3],
    qfi: u32,
    out_buffers: &mut [Vec<u8>],
    create_shader: &FnCreateShaderModule,
    destroy_shader: &FnDestroyShaderModule,
    create_dsl: &FnCreateDescriptorSetLayout,
    destroy_dsl: &FnDestroyDescriptorSetLayout,
    create_pl: &FnCreatePipelineLayout,
    destroy_pl: &FnDestroyPipelineLayout,
    create_cp: &FnCreateComputePipelines,
    destroy_pipe: &FnDestroyPipeline,
    create_dp: &FnCreateDescriptorPool,
    destroy_dp: &FnDestroyDescriptorPool,
    alloc_ds: &FnAllocateDescriptorSets,
    update_ds: &FnUpdateDescriptorSets,
    create_cmdpool: &FnCreateCommandPool,
    destroy_cmdpool: &FnDestroyCommandPool,
    alloc_cmd: &FnAllocateCommandBuffers,
    begin_cmd: &FnBeginCommandBuffer,
    end_cmd: &FnEndCommandBuffer,
    cmd_bind_pipe: &FnCmdBindPipeline,
    cmd_bind_ds: &FnCmdBindDescriptorSets,
    cmd_push: &FnCmdPushConstants,
    cmd_dispatch: &FnCmdDispatch,
    queue_submit: &FnQueueSubmit,
    queue_wait: &FnQueueWaitIdle,
    map_mem: &FnMapMemory,
    unmap_mem: &FnUnmapMemory,
) -> Result<(), String> {
    let n = bufs.len();
    // shader module。
    let smci = ShaderModuleCreateInfo {
        s_type: ST_SHADER_MODULE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        code_size: spv.len() * 4,
        p_code: spv.as_ptr(),
    };
    let mut shader: VkShaderModule = VK_NULL_HANDLE;
    if create_shader(device, &smci, std::ptr::null(), &mut shader) != VK_SUCCESS {
        return Err("vkCreateShaderModule 失败".into());
    }

    // descriptor set layout(每 buffer 一 StorageBuffer binding)。
    let bindings: Vec<DescriptorSetLayoutBinding> = (0..n)
        .map(|i| DescriptorSetLayoutBinding {
            binding: i as u32,
            descriptor_type: DESCRIPTOR_TYPE_STORAGE_BUFFER,
            descriptor_count: 1,
            stage_flags: SHADER_STAGE_COMPUTE,
            p_immutable_samplers: std::ptr::null(),
        })
        .collect();
    let dslci = DescriptorSetLayoutCreateInfo {
        s_type: ST_DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        binding_count: n as u32,
        p_bindings: bindings.as_ptr(),
    };
    let mut dsl: VkDescriptorSetLayout = VK_NULL_HANDLE;
    if create_dsl(device, &dslci, std::ptr::null(), &mut dsl) != VK_SUCCESS {
        destroy_shader(device, shader, std::ptr::null());
        return Err("vkCreateDescriptorSetLayout 失败".into());
    }

    // pipeline layout(+ push constant range)。
    let pcr = PushConstantRange {
        stage_flags: SHADER_STAGE_COMPUTE,
        offset: 0,
        size: push_constants.len().max(4) as u32,
    };
    let has_pc = !push_constants.is_empty();
    let plci = PipelineLayoutCreateInfo {
        s_type: ST_PIPELINE_LAYOUT_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        set_layout_count: 1,
        p_set_layouts: &dsl,
        push_constant_range_count: if has_pc { 1 } else { 0 },
        p_push_constant_ranges: if has_pc { &pcr } else { std::ptr::null() },
    };
    let mut pl: VkPipelineLayout = VK_NULL_HANDLE;
    if create_pl(device, &plci, std::ptr::null(), &mut pl) != VK_SUCCESS {
        destroy_dsl(device, dsl, std::ptr::null());
        destroy_shader(device, shader, std::ptr::null());
        return Err("vkCreatePipelineLayout 失败".into());
    }

    // compute pipeline。
    let entry_c = std::ffi::CString::new(entry).map_err(|_| "entry 名含 NUL")?;
    let cpci = ComputePipelineCreateInfo {
        s_type: ST_COMPUTE_PIPELINE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        stage: PipelineShaderStageCreateInfo {
            s_type: ST_PIPELINE_SHADER_STAGE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            stage: SHADER_STAGE_COMPUTE,
            module: shader,
            p_name: entry_c.as_ptr(),
            p_specialization_info: std::ptr::null(),
        },
        layout: pl,
        base_pipeline_handle: VK_NULL_HANDLE,
        base_pipeline_index: -1,
    };
    let mut pipe: VkPipeline = VK_NULL_HANDLE;
    let r = create_cp(
        device,
        VK_NULL_HANDLE,
        1,
        &cpci,
        std::ptr::null(),
        &mut pipe,
    );
    if r != VK_SUCCESS {
        destroy_pl(device, pl, std::ptr::null());
        destroy_dsl(device, dsl, std::ptr::null());
        destroy_shader(device, shader, std::ptr::null());
        return Err(format!("vkCreateComputePipelines 失败: {r}"));
    }

    // descriptor pool + set + update。
    let pool_size = DescriptorPoolSize {
        descriptor_type: DESCRIPTOR_TYPE_STORAGE_BUFFER,
        descriptor_count: n as u32,
    };
    let dpci = DescriptorPoolCreateInfo {
        s_type: ST_DESCRIPTOR_POOL_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        max_sets: 1,
        pool_size_count: 1,
        p_pool_sizes: &pool_size,
    };
    let mut pool: VkDescriptorPool = VK_NULL_HANDLE;
    create_dp(device, &dpci, std::ptr::null(), &mut pool);
    let dsai = DescriptorSetAllocateInfo {
        s_type: ST_DESCRIPTOR_SET_ALLOCATE_INFO,
        p_next: std::ptr::null(),
        descriptor_pool: pool,
        descriptor_set_count: 1,
        p_set_layouts: &dsl,
    };
    let mut dset: VkDescriptorSet = VK_NULL_HANDLE;
    alloc_ds(device, &dsai, &mut dset);
    let binfos: Vec<DescriptorBufferInfo> = bufs
        .iter()
        .map(|b| DescriptorBufferInfo {
            buffer: b.buffer,
            offset: 0,
            range: WHOLE_SIZE,
        })
        .collect();
    let writes: Vec<WriteDescriptorSet> = (0..n)
        .map(|i| WriteDescriptorSet {
            s_type: ST_WRITE_DESCRIPTOR_SET,
            p_next: std::ptr::null(),
            dst_set: dset,
            dst_binding: i as u32,
            dst_array_element: 0,
            descriptor_count: 1,
            descriptor_type: DESCRIPTOR_TYPE_STORAGE_BUFFER,
            p_image_info: std::ptr::null(),
            p_buffer_info: &binfos[i],
            p_texel_buffer_view: std::ptr::null(),
        })
        .collect();
    update_ds(device, n as u32, writes.as_ptr(), 0, std::ptr::null());

    // command pool + buffer + 录制。
    let cpci2 = CommandPoolCreateInfo {
        s_type: ST_COMMAND_POOL_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        queue_family_index: qfi,
    };
    let mut cmdpool: VkCommandPool = VK_NULL_HANDLE;
    create_cmdpool(device, &cpci2, std::ptr::null(), &mut cmdpool);
    let cbai = CommandBufferAllocateInfo {
        s_type: ST_COMMAND_BUFFER_ALLOCATE_INFO,
        p_next: std::ptr::null(),
        command_pool: cmdpool,
        level: CMD_BUFFER_LEVEL_PRIMARY,
        command_buffer_count: 1,
    };
    let mut cmd: VkCommandBuffer = std::ptr::null_mut();
    alloc_cmd(device, &cbai, &mut cmd);
    let cbbi = CommandBufferBeginInfo {
        s_type: ST_COMMAND_BUFFER_BEGIN_INFO,
        p_next: std::ptr::null(),
        flags: CMD_BUFFER_USAGE_ONE_TIME_SUBMIT,
        p_inheritance_info: std::ptr::null(),
    };
    begin_cmd(cmd, &cbbi);
    cmd_bind_pipe(cmd, PIPELINE_BIND_POINT_COMPUTE, pipe);
    cmd_bind_ds(
        cmd,
        PIPELINE_BIND_POINT_COMPUTE,
        pl,
        0,
        1,
        &dset,
        0,
        std::ptr::null(),
    );
    if has_pc {
        cmd_push(
            cmd,
            pl,
            SHADER_STAGE_COMPUTE,
            0,
            push_constants.len() as u32,
            push_constants.as_ptr().cast::<c_void>(),
        );
    }
    cmd_dispatch(cmd, groups[0], groups[1], groups[2]);
    end_cmd(cmd);

    // 提交 + 等待。
    let si = SubmitInfo {
        s_type: ST_SUBMIT_INFO,
        p_next: std::ptr::null(),
        wait_semaphore_count: 0,
        p_wait_semaphores: std::ptr::null(),
        p_wait_dst_stage_mask: std::ptr::null(),
        command_buffer_count: 1,
        p_command_buffers: &cmd,
        signal_semaphore_count: 0,
        p_signal_semaphores: std::ptr::null(),
    };
    let r = queue_submit(queue, 1, &si, VK_NULL_HANDLE);
    if r == VK_SUCCESS {
        queue_wait(queue);
        // 回读所有 buffer。
        for (i, b) in bufs.iter().enumerate() {
            let mut ptr: *mut c_void = std::ptr::null_mut();
            if map_mem(device, b.mem, 0, WHOLE_SIZE, 0, &mut ptr) == VK_SUCCESS {
                std::ptr::copy_nonoverlapping(
                    ptr.cast::<u8>(),
                    out_buffers[i].as_mut_ptr(),
                    out_buffers[i].len(),
                );
                unmap_mem(device, b.mem);
            }
        }
    }

    // 清理(逆序)。
    destroy_cmdpool(device, cmdpool, std::ptr::null());
    destroy_dp(device, pool, std::ptr::null());
    destroy_pipe(device, pipe, std::ptr::null());
    destroy_pl(device, pl, std::ptr::null());
    destroy_dsl(device, dsl, std::ptr::null());
    destroy_shader(device, shader, std::ptr::null());
    if r != VK_SUCCESS {
        return Err(format!("vkQueueSubmit 失败: {r}"));
    }
    Ok(())
}

// ───────────────────────── graphics offscreen(RXS-0210) ─────────────────────

/// 选内存类型:`type_bits` 允许集合内首个属性含全部 `required` 标志者(host 可测纯函数)。
fn pick_mem_type(
    memprops: &PhysicalDeviceMemoryProperties,
    type_bits: u32,
    required: VkFlags,
) -> Option<u32> {
    (0..memprops.memory_type_count).find(|&i| {
        let mt = memprops.memory_types[i as usize];
        type_bits & (1 << i) != 0 && mt.property_flags & required == required
    })
}

// ── VK_EXT_debug_utils messenger(RXS-0210 fail-closed 判据 / L3;仅 validation 开启)──
// 手写 FFI:开发期(`RURIX_VK_VALIDATION=1`)装 debug messenger,把 `VK_LAYER_KHRONOS_validation`
// 的 ERROR 级消息经回调记入 `AtomicBool` 标志 → `run_graphics_offscreen` 末尾据此翻 `Err`
// (fail-closed,非 panic)。这使 provenance 变体(带 SPV_GOOGLE)喂 `vkCreateShaderModule`
// 触 VUID-...-08742 时**以退出码判红**(NVIDIA 驱动本身仍返 VK_SUCCESS,仅 layer 报 → 无
// messenger 则 demo 假绿;messenger 是 red_self_test 退出码判据的根)。
type VkDebugUtilsMessengerEXT = u64;
const ST_DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT: u32 = 1_000_128_004;
const DEBUG_UTILS_SEVERITY_ERROR: u32 = 0x0000_1000;
const DEBUG_UTILS_TYPE_GENERAL: u32 = 0x1;
const DEBUG_UTILS_TYPE_VALIDATION: u32 = 0x2;
const DEBUG_UTILS_TYPE_PERFORMANCE: u32 = 0x4;

type PfnDebugUtilsMessengerCallback = unsafe extern "system" fn(
    u32,
    u32,
    *const DebugUtilsMessengerCallbackDataEXT,
    *mut c_void,
) -> u32;
type FnCreateDebugUtilsMessengerEXT = unsafe extern "system" fn(
    VkInstance,
    *const DebugUtilsMessengerCreateInfoEXT,
    *const c_void,
    *mut VkDebugUtilsMessengerEXT,
) -> VkResult;
type FnDestroyDebugUtilsMessengerEXT =
    unsafe extern "system" fn(VkInstance, VkDebugUtilsMessengerEXT, *const c_void);

#[repr(C)]
struct DebugUtilsMessengerCreateInfoEXT {
    s_type: u32,
    p_next: *const c_void,
    flags: u32,
    message_severity: u32,
    message_type: u32,
    pfn_user_callback: PfnDebugUtilsMessengerCallback,
    p_user_data: *mut c_void,
}

/// `VkDebugUtilsMessengerCallbackDataEXT`(逐字节对齐;本回调仅读 `p_message`,但全字段列出
/// 以定位其偏移)。
#[repr(C)]
struct DebugUtilsMessengerCallbackDataEXT {
    s_type: u32,
    p_next: *const c_void,
    flags: u32,
    p_message_id_name: *const c_char,
    message_id_number: i32,
    p_message: *const c_char,
    queue_label_count: u32,
    p_queue_labels: *const c_void,
    cmd_buf_label_count: u32,
    p_cmd_buf_labels: *const c_void,
    object_count: u32,
    p_objects: *const c_void,
}

/// ERROR 级校验消息 → 置 `p_user_data`(指向调用方栈上 `AtomicBool`)真 + 打印到 stderr。
/// 返回 `VK_FALSE`(0):不中断被回调的 Vulkan 命令(仅记录,fail-closed 在入口统一判)。
unsafe extern "system" fn debug_messenger_cb(
    severity: u32,
    _types: u32,
    data: *const DebugUtilsMessengerCallbackDataEXT,
    user_data: *mut c_void,
) -> u32 {
    if severity & DEBUG_UTILS_SEVERITY_ERROR != 0 {
        if !user_data.is_null() {
            // SAFETY: user_data 是 run_graphics_inner 栈上 AtomicBool 的指针;messenger 生命周期
            // 严格短于该 AtomicBool(messenger 在函数末尾、instance destroy 前销毁)。原子写经
            // 共享引用合法(内部可变),无 &mut 别名。
            let flag = &*(user_data as *const std::sync::atomic::AtomicBool);
            flag.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        if !data.is_null() {
            // SAFETY: 回调契约保证 data 在回调期间有效;p_message 为有效 NUL 结尾 C 串。
            let d = &*data;
            if !d.p_message.is_null() {
                let msg = std::ffi::CStr::from_ptr(d.p_message).to_string_lossy();
                eprintln!("[vk-validation] {msg}");
            }
        }
    }
    0
}

/// offscreen 渲染一帧三角形并回读像素(RXS-0210;headless,无 swapchain/窗口)。
///
/// render pass(单 color attachment,CLEAR→STORE,finalLayout=TRANSFER_SRC_OPTIMAL)+
/// graphics pipeline(vertex+fragment 双 stage,pName 恒 `"main"`——`OpEntryPoint` 名恒
/// `"main"`,不走 compute mangled 路径)+ framebuffer + 顶点缓冲 + `vkCmdDraw` +
/// `vkCmdCopyImageToBuffer` 回读 → 紧凑 RGBA8(`width*height*4`,行 pitch 紧凑)。
///
/// - `vs_spv`/`fs_spv`:`--target vulkan` 产的 vertex/fragment SPIR-V(RXS-0210 去 provenance)。
/// - `vertices`:交错顶点字节(每顶点 `vertex_stride` 字节;pos+color 等)。
/// - `attrs`:`(location, format, offset)` 顶点属性描述(单 binding 0)。
/// - `clear`:清屏色 RGBA(f32);未被三角形覆盖处即此色。
///
/// 缺 Vulkan 驱动 / 无 graphics queue / pipeline 创建失败 → 确定性 `Err`(非 panic,
/// fail-closed,P-01,无静默 fallback)。swapchain/窗口 present 为 open 尾门(RD-032)。
///
/// # SAFETY(U27,graphics FFI 边界)
/// 本公共入口对上全 safe(无 `unsafe` 签名)。内部 `run_graphics_inner`/`graphics_body`
/// 全程手写 Vulkan FFI:`vulkan-1.dll` 经 loader 动态装载(缺失 → `Err` 非 panic);每个
/// `#[repr(C)]` VkStruct 与 Vulkan spec 逐字节对齐(由 `VK_LAYER_KHRONOS_validation` 真跑
/// 零报错实证);句柄(image/imageView/renderPass/framebuffer/buffer/memory/shaderModule/
/// pipeline/commandPool)在 `graphics_body` 内**线性配对 create/destroy**(逆序销毁,无
/// 泄漏、无双重释放);顶点/回读缓冲 host-visible+coherent(免 flush);单 queue 同步提交 +
/// `vkQueueWaitIdle` 后回读(无数据竞争)。gate feature `vulkan` 默认关闭,CUDA 路零回归。
#[allow(clippy::too_many_arguments)]
pub fn run_graphics_offscreen(
    vs_spv: &[u32],
    fs_spv: &[u32],
    vertices: &[u8],
    vertex_stride: u32,
    attrs: &[(u32, u32, u32)],
    width: u32,
    height: u32,
    clear: [f32; 4],
) -> Result<Vec<u8>, String> {
    let gipa = load_vulkan_loader().ok_or("vulkan loader (vulkan-1.dll/libvulkan.so) 不可用")?;
    // SAFETY: 见 U27 契约(上）;句柄生命周期由内部函数线性管理,末尾逆序销毁。
    unsafe {
        run_graphics_inner(
            gipa,
            vs_spv,
            fs_spv,
            vertices,
            vertex_stride,
            attrs,
            width,
            height,
            clear,
        )
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn run_graphics_inner(
    gipa: FnGetInstanceProcAddr,
    vs_spv: &[u32],
    fs_spv: &[u32],
    vertices: &[u8],
    vertex_stride: u32,
    attrs: &[(u32, u32, u32)],
    width: u32,
    height: u32,
    clear: [f32; 4],
) -> Result<Vec<u8>, String> {
    let vk_create_instance: FnCreateInstance =
        cast_fn(gipa(std::ptr::null_mut(), c"vkCreateInstance".as_ptr()))
            .ok_or("缺 vkCreateInstance")?;

    let validation = std::env::var("RURIX_VK_VALIDATION").as_deref() == Ok("1");
    let layer_name = c"VK_LAYER_KHRONOS_validation";
    let layers: [*const c_char; 1] = [layer_name.as_ptr()];
    // validation 开:装 VK_EXT_debug_utils(layer 提供)→ 后续经 messenger 记 ERROR 级消息。
    let debug_ext = c"VK_EXT_debug_utils";
    let exts: [*const c_char; 1] = [debug_ext.as_ptr()];
    let app = ApplicationInfo {
        s_type: ST_APPLICATION_INFO,
        p_next: std::ptr::null(),
        p_application_name: c"rurix-mb1".as_ptr(),
        application_version: 0,
        p_engine_name: c"rurix".as_ptr(),
        engine_version: 0,
        api_version: API_VERSION_1_1,
    };
    let ici = InstanceCreateInfo {
        s_type: ST_INSTANCE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        p_application_info: &app,
        enabled_layer_count: if validation { 1 } else { 0 },
        pp_enabled_layer_names: if validation {
            layers.as_ptr()
        } else {
            std::ptr::null()
        },
        enabled_extension_count: if validation { 1 } else { 0 },
        pp_enabled_extension_names: if validation {
            exts.as_ptr()
        } else {
            std::ptr::null()
        },
    };
    let mut instance: VkInstance = std::ptr::null_mut();
    let r = vk_create_instance(&ici, std::ptr::null(), &mut instance);
    if r != VK_SUCCESS {
        return Err(format!("vkCreateInstance 失败: {r}"));
    }

    let vk_destroy_instance: FnDestroyInstance =
        cast_fn(gipa(instance, c"vkDestroyInstance".as_ptr())).ok_or("缺 vkDestroyInstance")?;
    let vk_enum_pd: FnEnumeratePhysicalDevices =
        cast_fn(gipa(instance, c"vkEnumeratePhysicalDevices".as_ptr()))
            .ok_or("缺 vkEnumeratePhysicalDevices")?;
    let vk_get_qf: FnGetPhysicalDeviceQueueFamilyProperties = cast_fn(gipa(
        instance,
        c"vkGetPhysicalDeviceQueueFamilyProperties".as_ptr(),
    ))
    .ok_or("缺 vkGetPhysicalDeviceQueueFamilyProperties")?;
    let vk_get_mem: FnGetPhysicalDeviceMemoryProperties = cast_fn(gipa(
        instance,
        c"vkGetPhysicalDeviceMemoryProperties".as_ptr(),
    ))
    .ok_or("缺 vkGetPhysicalDeviceMemoryProperties")?;
    let vk_create_device: FnCreateDevice =
        cast_fn(gipa(instance, c"vkCreateDevice".as_ptr())).ok_or("缺 vkCreateDevice")?;
    let vk_get_device_proc: FnGetDeviceProcAddr =
        cast_fn(gipa(instance, c"vkGetDeviceProcAddr".as_ptr())).ok_or("缺 vkGetDeviceProcAddr")?;

    // fail-closed 判据(L3,RXS-0210):validation 开时装 debug messenger,ERROR 级校验消息
    // 经回调置此标志 → 末尾翻 `Err`(使 provenance 变体的 VUID-...-08742 以退出码判红;
    // NVIDIA 驱动本身对带 SPV_GOOGLE 的 SPIR-V 仍返 VK_SUCCESS,无 messenger 则 demo 假绿)。
    // **置于全部 instance-符号 `?` 查找之后、首个 Vulkan API 调用(vk_enum_pd)之前**:上述
    // 查找皆纯 `vkGetInstanceProcAddr` 取址、不发 Vulkan 调用(messenger 无需在其间存在);
    // 建于此既完整保住对 `vkCreateShaderModule` 等真实调用的错误捕获(red_self_test),又确保
    // messenger 创建后的**每个** early-return 都经 destroy_msgr!() 正确拆除(闭合泄漏窗口——
    // 创建与首个销毁点之间无 `?` 早退)。
    let validation_error = std::sync::atomic::AtomicBool::new(false);
    let mut messenger: VkDebugUtilsMessengerEXT = VK_NULL_HANDLE;
    let destroy_messenger: Option<FnDestroyDebugUtilsMessengerEXT> = if validation {
        cast_fn(gipa(instance, c"vkDestroyDebugUtilsMessengerEXT".as_ptr()))
    } else {
        None
    };
    if validation
        && let Some(create_messenger) = cast_fn::<FnCreateDebugUtilsMessengerEXT>(gipa(
            instance,
            c"vkCreateDebugUtilsMessengerEXT".as_ptr(),
        ))
    {
        let dumci = DebugUtilsMessengerCreateInfoEXT {
            s_type: ST_DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
            p_next: std::ptr::null(),
            flags: 0,
            // ERROR 级订阅:good 路无 ERROR → 回调不触 → stderr 静默 + 标志假(绿);
            // provenance 路 VUID-08742 为 ERROR → 触发 → 末尾 Err(红)。
            message_severity: DEBUG_UTILS_SEVERITY_ERROR,
            message_type: DEBUG_UTILS_TYPE_GENERAL
                | DEBUG_UTILS_TYPE_VALIDATION
                | DEBUG_UTILS_TYPE_PERFORMANCE,
            pfn_user_callback: debug_messenger_cb,
            p_user_data: &validation_error as *const std::sync::atomic::AtomicBool as *mut c_void,
        };
        let _ = create_messenger(instance, &dumci, std::ptr::null(), &mut messenger);
    }
    // messenger 逆序销毁 helper(须先于 vkDestroyInstance,否则 instance 尚有子对象 → VUID)。
    macro_rules! destroy_msgr {
        () => {
            if let Some(dm) = destroy_messenger {
                if messenger != VK_NULL_HANDLE {
                    dm(instance, messenger, std::ptr::null());
                }
            }
        };
    }

    let mut count = 0u32;
    vk_enum_pd(instance, &mut count, std::ptr::null_mut());
    if count == 0 {
        destroy_msgr!();
        vk_destroy_instance(instance, std::ptr::null());
        return Err("无 Vulkan 物理设备".into());
    }
    let mut pds = vec![std::ptr::null_mut::<c_void>(); count as usize];
    vk_enum_pd(instance, &mut count, pds.as_mut_ptr());
    let pd = pds[0];

    // 找 graphics queue family(区别于 compute:QUEUE_GRAPHICS_BIT)。
    let mut qf_count = 0u32;
    vk_get_qf(pd, &mut qf_count, std::ptr::null_mut());
    let mut qfs: Vec<QueueFamilyProperties> = (0..qf_count)
        .map(|_| QueueFamilyProperties {
            queue_flags: 0,
            queue_count: 0,
            timestamp_valid_bits: 0,
            min_image_transfer_granularity: VkExtent3D {
                width: 0,
                height: 0,
                depth: 0,
            },
        })
        .collect();
    vk_get_qf(pd, &mut qf_count, qfs.as_mut_ptr());
    let qfi = match qfs
        .iter()
        .position(|q| q.queue_flags & QUEUE_GRAPHICS_BIT != 0)
    {
        Some(i) => i as u32,
        None => {
            destroy_msgr!();
            vk_destroy_instance(instance, std::ptr::null());
            return Err("无 graphics queue family".into());
        }
    };

    let prio = [1.0f32];
    let dqci = DeviceQueueCreateInfo {
        s_type: ST_DEVICE_QUEUE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        queue_family_index: qfi,
        queue_count: 1,
        p_queue_priorities: prio.as_ptr(),
    };
    let dci = DeviceCreateInfo {
        s_type: ST_DEVICE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        queue_create_info_count: 1,
        p_queue_create_infos: &dqci,
        enabled_layer_count: 0,
        pp_enabled_layer_names: std::ptr::null(),
        enabled_extension_count: 0,
        pp_enabled_extension_names: std::ptr::null(),
        p_enabled_features: std::ptr::null(),
    };
    let mut device: VkDevice = std::ptr::null_mut();
    let r = vk_create_device(pd, &dci, std::ptr::null(), &mut device);
    if r != VK_SUCCESS {
        destroy_msgr!();
        vk_destroy_instance(instance, std::ptr::null());
        return Err(format!("vkCreateDevice 失败: {r}"));
    }

    let mut out = graphics_body(
        vk_get_device_proc,
        device,
        pd,
        vk_get_mem,
        qfi,
        vs_spv,
        fs_spv,
        vertices,
        vertex_stride,
        attrs,
        width,
        height,
        clear,
    );

    // fail-closed(L3):validation 开 + 出现 ERROR 级校验消息 → 覆盖为 Err(退出码判红)。
    // good 路无 ERROR → 标志假 → 保持 graphics_body 的 Ok(退出码判绿)。
    if validation && validation_error.load(std::sync::atomic::Ordering::Relaxed) {
        out =
            Err("VK_LAYER_KHRONOS_validation 报 ERROR 级校验错误(见 stderr;fail-closed,L3)".into());
    }

    let vk_destroy_device: Option<FnDestroyDevice> =
        cast_fn(vk_get_device_proc(device, c"vkDestroyDevice".as_ptr()));
    if let Some(dd) = vk_destroy_device {
        dd(device, std::ptr::null());
    }
    destroy_msgr!();
    vk_destroy_instance(instance, std::ptr::null());
    out
}

#[allow(clippy::too_many_arguments)]
unsafe fn graphics_body(
    gdpa: FnGetDeviceProcAddr,
    device: VkDevice,
    pd: VkPhysicalDevice,
    vk_get_mem: FnGetPhysicalDeviceMemoryProperties,
    qfi: u32,
    vs_spv: &[u32],
    fs_spv: &[u32],
    vertices: &[u8],
    vertex_stride: u32,
    attrs: &[(u32, u32, u32)],
    width: u32,
    height: u32,
    clear: [f32; 4],
) -> Result<Vec<u8>, String> {
    macro_rules! dp {
        ($name:literal, $ty:ty) => {
            cast_fn::<$ty>(gdpa(device, $name.as_ptr())).ok_or("缺 device 符号")?
        };
    }
    let get_queue: FnGetDeviceQueue = dp!(c"vkGetDeviceQueue", FnGetDeviceQueue);
    let create_buffer: FnCreateBuffer = dp!(c"vkCreateBuffer", FnCreateBuffer);
    let destroy_buffer: FnDestroyBuffer = dp!(c"vkDestroyBuffer", FnDestroyBuffer);
    let buf_mem_req: FnGetBufferMemoryRequirements = dp!(
        c"vkGetBufferMemoryRequirements",
        FnGetBufferMemoryRequirements
    );
    let alloc_mem: FnAllocateMemory = dp!(c"vkAllocateMemory", FnAllocateMemory);
    let free_mem: FnFreeMemory = dp!(c"vkFreeMemory", FnFreeMemory);
    let bind_buf: FnBindBufferMemory = dp!(c"vkBindBufferMemory", FnBindBufferMemory);
    let map_mem: FnMapMemory = dp!(c"vkMapMemory", FnMapMemory);
    let unmap_mem: FnUnmapMemory = dp!(c"vkUnmapMemory", FnUnmapMemory);
    let create_shader: FnCreateShaderModule = dp!(c"vkCreateShaderModule", FnCreateShaderModule);
    let destroy_shader: FnDestroyShaderModule =
        dp!(c"vkDestroyShaderModule", FnDestroyShaderModule);
    let create_pl: FnCreatePipelineLayout = dp!(c"vkCreatePipelineLayout", FnCreatePipelineLayout);
    let destroy_pl: FnDestroyPipelineLayout =
        dp!(c"vkDestroyPipelineLayout", FnDestroyPipelineLayout);
    let destroy_pipe: FnDestroyPipeline = dp!(c"vkDestroyPipeline", FnDestroyPipeline);
    let create_cmdpool: FnCreateCommandPool = dp!(c"vkCreateCommandPool", FnCreateCommandPool);
    let destroy_cmdpool: FnDestroyCommandPool = dp!(c"vkDestroyCommandPool", FnDestroyCommandPool);
    let alloc_cmd: FnAllocateCommandBuffers =
        dp!(c"vkAllocateCommandBuffers", FnAllocateCommandBuffers);
    let begin_cmd: FnBeginCommandBuffer = dp!(c"vkBeginCommandBuffer", FnBeginCommandBuffer);
    let end_cmd: FnEndCommandBuffer = dp!(c"vkEndCommandBuffer", FnEndCommandBuffer);
    let cmd_bind_pipe: FnCmdBindPipeline = dp!(c"vkCmdBindPipeline", FnCmdBindPipeline);
    let queue_submit: FnQueueSubmit = dp!(c"vkQueueSubmit", FnQueueSubmit);
    let queue_wait: FnQueueWaitIdle = dp!(c"vkQueueWaitIdle", FnQueueWaitIdle);
    // graphics 专属符号。
    let create_image: FnCreateImage = dp!(c"vkCreateImage", FnCreateImage);
    let destroy_image: FnDestroyImage = dp!(c"vkDestroyImage", FnDestroyImage);
    let img_mem_req: FnGetImageMemoryRequirements = dp!(
        c"vkGetImageMemoryRequirements",
        FnGetImageMemoryRequirements
    );
    let bind_image: FnBindImageMemory = dp!(c"vkBindImageMemory", FnBindImageMemory);
    let create_view: FnCreateImageView = dp!(c"vkCreateImageView", FnCreateImageView);
    let destroy_view: FnDestroyImageView = dp!(c"vkDestroyImageView", FnDestroyImageView);
    let create_rp: FnCreateRenderPass = dp!(c"vkCreateRenderPass", FnCreateRenderPass);
    let destroy_rp: FnDestroyRenderPass = dp!(c"vkDestroyRenderPass", FnDestroyRenderPass);
    let create_fb: FnCreateFramebuffer = dp!(c"vkCreateFramebuffer", FnCreateFramebuffer);
    let destroy_fb: FnDestroyFramebuffer = dp!(c"vkDestroyFramebuffer", FnDestroyFramebuffer);
    let create_gp: FnCreateGraphicsPipelines =
        dp!(c"vkCreateGraphicsPipelines", FnCreateGraphicsPipelines);
    let cmd_begin_rp: FnCmdBeginRenderPass = dp!(c"vkCmdBeginRenderPass", FnCmdBeginRenderPass);
    let cmd_end_rp: FnCmdEndRenderPass = dp!(c"vkCmdEndRenderPass", FnCmdEndRenderPass);
    let cmd_bind_vbuf: FnCmdBindVertexBuffers =
        dp!(c"vkCmdBindVertexBuffers", FnCmdBindVertexBuffers);
    let cmd_draw: FnCmdDraw = dp!(c"vkCmdDraw", FnCmdDraw);
    let cmd_barrier: FnCmdPipelineBarrier = dp!(c"vkCmdPipelineBarrier", FnCmdPipelineBarrier);
    let cmd_copy_img_buf: FnCmdCopyImageToBuffer =
        dp!(c"vkCmdCopyImageToBuffer", FnCmdCopyImageToBuffer);

    let mut queue: VkQueue = std::ptr::null_mut();
    get_queue(device, qfi, 0, &mut queue);

    let mut memprops = std::mem::zeroed::<PhysicalDeviceMemoryProperties>();
    vk_get_mem(pd, &mut memprops);

    let readback_len = (width as usize) * (height as usize) * 4;

    // host-visible buffer 建立 helper(顶点/回读共用)。
    let make_host_buffer = |usage: u32, size: u64| -> Result<(VkBuffer, VkDeviceMemory), String> {
        let bci = BufferCreateInfo {
            s_type: ST_BUFFER_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            size: size.max(4),
            usage,
            sharing_mode: SHARING_MODE_EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
        };
        let mut buffer: VkBuffer = VK_NULL_HANDLE;
        if create_buffer(device, &bci, std::ptr::null(), &mut buffer) != VK_SUCCESS {
            return Err("vkCreateBuffer 失败".into());
        }
        let mut req = std::mem::zeroed::<MemoryRequirements>();
        buf_mem_req(device, buffer, &mut req);
        let Some(mt) = pick_mem_type(
            &memprops,
            req.memory_type_bits,
            MEM_HOST_VISIBLE | MEM_HOST_COHERENT,
        ) else {
            destroy_buffer(device, buffer, std::ptr::null());
            return Err("无 host-visible+coherent 内存类型".into());
        };
        let mai = MemoryAllocateInfo {
            s_type: ST_MEMORY_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            allocation_size: req.size,
            memory_type_index: mt,
        };
        let mut mem: VkDeviceMemory = VK_NULL_HANDLE;
        if alloc_mem(device, &mai, std::ptr::null(), &mut mem) != VK_SUCCESS {
            destroy_buffer(device, buffer, std::ptr::null());
            return Err("vkAllocateMemory 失败".into());
        }
        bind_buf(device, buffer, mem, 0);
        Ok((buffer, mem))
    };

    // 句柄(全 null 初始,末尾逆序销毁非 null 者)。
    let mut color_image: VkImage = VK_NULL_HANDLE;
    let mut color_mem: VkDeviceMemory = VK_NULL_HANDLE;
    let mut color_view: VkImageView = VK_NULL_HANDLE;
    let mut render_pass: VkRenderPass = VK_NULL_HANDLE;
    let mut framebuffer: VkFramebuffer = VK_NULL_HANDLE;
    let mut vbuf: VkBuffer = VK_NULL_HANDLE;
    let mut vbuf_mem: VkDeviceMemory = VK_NULL_HANDLE;
    let mut rbuf: VkBuffer = VK_NULL_HANDLE;
    let mut rbuf_mem: VkDeviceMemory = VK_NULL_HANDLE;
    let mut vs_mod: VkShaderModule = VK_NULL_HANDLE;
    let mut fs_mod: VkShaderModule = VK_NULL_HANDLE;
    let mut pipe_layout: VkPipelineLayout = VK_NULL_HANDLE;
    let mut pipeline: VkPipeline = VK_NULL_HANDLE;
    let mut cmdpool: VkCommandPool = VK_NULL_HANDLE;

    // 标签块产出 result(每 break 携 Err,正常尾出 Ok);句柄在块外销毁(逆序)。
    let result: Result<Vec<u8>, String> = 'run: {
        // ── color image(device-local,COLOR_ATTACHMENT|TRANSFER_SRC)──
        let img_ci = ImageCreateInfo {
            s_type: ST_IMAGE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            image_type: IMAGE_TYPE_2D,
            format: FORMAT_R8G8B8A8_UNORM,
            extent: VkExtent3D {
                width,
                height,
                depth: 1,
            },
            mip_levels: 1,
            array_layers: 1,
            samples: SAMPLE_COUNT_1,
            tiling: IMAGE_TILING_OPTIMAL,
            usage: IMAGE_USAGE_COLOR_ATTACHMENT | IMAGE_USAGE_TRANSFER_SRC,
            sharing_mode: SHARING_MODE_EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
            initial_layout: IMAGE_LAYOUT_UNDEFINED,
        };
        if create_image(device, &img_ci, std::ptr::null(), &mut color_image) != VK_SUCCESS {
            break 'run Err("vkCreateImage 失败".into());
        }
        let mut ireq = std::mem::zeroed::<MemoryRequirements>();
        img_mem_req(device, color_image, &mut ireq);
        let Some(imt) = pick_mem_type(&memprops, ireq.memory_type_bits, MEM_DEVICE_LOCAL) else {
            break 'run Err("无 device-local 内存类型".into());
        };
        let mai = MemoryAllocateInfo {
            s_type: ST_MEMORY_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            allocation_size: ireq.size,
            memory_type_index: imt,
        };
        if alloc_mem(device, &mai, std::ptr::null(), &mut color_mem) != VK_SUCCESS {
            break 'run Err("color image vkAllocateMemory 失败".into());
        }
        bind_image(device, color_image, color_mem, 0);

        // ── image view ──
        let view_ci = ImageViewCreateInfo {
            s_type: ST_IMAGE_VIEW_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            image: color_image,
            view_type: IMAGE_VIEW_TYPE_2D,
            format: FORMAT_R8G8B8A8_UNORM,
            components: VkComponentMapping {
                r: COMPONENT_SWIZZLE_IDENTITY,
                g: COMPONENT_SWIZZLE_IDENTITY,
                b: COMPONENT_SWIZZLE_IDENTITY,
                a: COMPONENT_SWIZZLE_IDENTITY,
            },
            subresource_range: VkImageSubresourceRange {
                aspect_mask: IMAGE_ASPECT_COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
        };
        if create_view(device, &view_ci, std::ptr::null(), &mut color_view) != VK_SUCCESS {
            break 'run Err("vkCreateImageView 失败".into());
        }

        // ── render pass(单 color attachment,CLEAR→STORE,final=TRANSFER_SRC)──
        let att = AttachmentDescription {
            flags: 0,
            format: FORMAT_R8G8B8A8_UNORM,
            samples: SAMPLE_COUNT_1,
            load_op: ATTACHMENT_LOAD_OP_CLEAR,
            store_op: ATTACHMENT_STORE_OP_STORE,
            stencil_load_op: ATTACHMENT_LOAD_OP_DONT_CARE,
            stencil_store_op: ATTACHMENT_STORE_OP_DONT_CARE,
            initial_layout: IMAGE_LAYOUT_UNDEFINED,
            final_layout: IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
        };
        let att_ref = AttachmentReference {
            attachment: 0,
            layout: IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
        };
        let subpass = SubpassDescription {
            flags: 0,
            pipeline_bind_point: PIPELINE_BIND_POINT_GRAPHICS,
            input_attachment_count: 0,
            p_input_attachments: std::ptr::null(),
            color_attachment_count: 1,
            p_color_attachments: &att_ref,
            p_resolve_attachments: std::ptr::null(),
            p_depth_stencil_attachment: std::ptr::null(),
            preserve_attachment_count: 0,
            p_preserve_attachments: std::ptr::null(),
        };
        let rp_ci = RenderPassCreateInfo {
            s_type: ST_RENDER_PASS_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            attachment_count: 1,
            p_attachments: &att,
            subpass_count: 1,
            p_subpasses: &subpass,
            dependency_count: 0,
            p_dependencies: std::ptr::null(),
        };
        if create_rp(device, &rp_ci, std::ptr::null(), &mut render_pass) != VK_SUCCESS {
            break 'run Err("vkCreateRenderPass 失败".into());
        }

        // ── framebuffer ──
        let fb_ci = FramebufferCreateInfo {
            s_type: ST_FRAMEBUFFER_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            render_pass,
            attachment_count: 1,
            p_attachments: &color_view,
            width,
            height,
            layers: 1,
        };
        if create_fb(device, &fb_ci, std::ptr::null(), &mut framebuffer) != VK_SUCCESS {
            break 'run Err("vkCreateFramebuffer 失败".into());
        }

        // ── vertex buffer + 上传 ──
        match make_host_buffer(BUFFER_USAGE_VERTEX, vertices.len().max(4) as u64) {
            Ok((b, m)) => {
                vbuf = b;
                vbuf_mem = m;
            }
            Err(e) => {
                break 'run Err(e);
            }
        }
        {
            let mut ptr: *mut c_void = std::ptr::null_mut();
            if map_mem(device, vbuf_mem, 0, WHOLE_SIZE, 0, &mut ptr) != VK_SUCCESS {
                break 'run Err("顶点缓冲 vkMapMemory 失败".into());
            }
            std::ptr::copy_nonoverlapping(vertices.as_ptr(), ptr.cast::<u8>(), vertices.len());
            unmap_mem(device, vbuf_mem);
        }

        // ── readback buffer(host-visible,TRANSFER_DST,W*H*4)──
        match make_host_buffer(BUFFER_USAGE_TRANSFER_DST, readback_len as u64) {
            Ok((b, m)) => {
                rbuf = b;
                rbuf_mem = m;
            }
            Err(e) => {
                break 'run Err(e);
            }
        }

        // ── shader modules(pName 恒 "main")──
        let make_shader = |spv: &[u32]| -> Result<VkShaderModule, String> {
            let smci = ShaderModuleCreateInfo {
                s_type: ST_SHADER_MODULE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                code_size: spv.len() * 4,
                p_code: spv.as_ptr(),
            };
            let mut m: VkShaderModule = VK_NULL_HANDLE;
            if create_shader(device, &smci, std::ptr::null(), &mut m) != VK_SUCCESS {
                return Err("vkCreateShaderModule 失败(VUID-...-08742?)".into());
            }
            Ok(m)
        };
        match make_shader(vs_spv) {
            Ok(m) => vs_mod = m,
            Err(e) => {
                break 'run Err(format!("vertex {e}"));
            }
        }
        match make_shader(fs_spv) {
            Ok(m) => fs_mod = m,
            Err(e) => {
                break 'run Err(format!("fragment {e}"));
            }
        }

        // ── pipeline layout(空 set / 无 push const)──
        let plci = PipelineLayoutCreateInfo {
            s_type: ST_PIPELINE_LAYOUT_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            set_layout_count: 0,
            p_set_layouts: std::ptr::null(),
            push_constant_range_count: 0,
            p_push_constant_ranges: std::ptr::null(),
        };
        if create_pl(device, &plci, std::ptr::null(), &mut pipe_layout) != VK_SUCCESS {
            break 'run Err("vkCreatePipelineLayout 失败".into());
        }

        // ── graphics pipeline ──
        let stages = [
            PipelineShaderStageCreateInfo {
                s_type: ST_PIPELINE_SHADER_STAGE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                stage: SHADER_STAGE_VERTEX,
                module: vs_mod,
                p_name: c"main".as_ptr(),
                p_specialization_info: std::ptr::null(),
            },
            PipelineShaderStageCreateInfo {
                s_type: ST_PIPELINE_SHADER_STAGE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                stage: SHADER_STAGE_FRAGMENT,
                module: fs_mod,
                p_name: c"main".as_ptr(),
                p_specialization_info: std::ptr::null(),
            },
        ];
        let vbind = VkVertexInputBindingDescription {
            binding: 0,
            stride: vertex_stride,
            input_rate: VERTEX_INPUT_RATE_VERTEX,
        };
        let vattrs: Vec<VkVertexInputAttributeDescription> = attrs
            .iter()
            .map(
                |&(location, format, offset)| VkVertexInputAttributeDescription {
                    location,
                    binding: 0,
                    format,
                    offset,
                },
            )
            .collect();
        let vin = PipelineVertexInputStateCreateInfo {
            s_type: ST_PIPELINE_VERTEX_INPUT_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            vertex_binding_description_count: 1,
            p_vertex_binding_descriptions: &vbind,
            vertex_attribute_description_count: vattrs.len() as u32,
            p_vertex_attribute_descriptions: vattrs.as_ptr(),
        };
        let ia = PipelineInputAssemblyStateCreateInfo {
            s_type: ST_PIPELINE_INPUT_ASSEMBLY_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            topology: PRIMITIVE_TOPOLOGY_TRIANGLE_LIST,
            primitive_restart_enable: 0,
        };
        let viewport = VkViewport {
            x: 0.0,
            y: 0.0,
            width: width as f32,
            height: height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };
        let scissor = VkRect2D {
            offset: VkOffset2D { x: 0, y: 0 },
            extent: VkExtent2D { width, height },
        };
        let vp = PipelineViewportStateCreateInfo {
            s_type: ST_PIPELINE_VIEWPORT_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            viewport_count: 1,
            p_viewports: &viewport,
            scissor_count: 1,
            p_scissors: &scissor,
        };
        let rs = PipelineRasterizationStateCreateInfo {
            s_type: ST_PIPELINE_RASTERIZATION_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            depth_clamp_enable: 0,
            rasterizer_discard_enable: 0,
            polygon_mode: POLYGON_MODE_FILL,
            cull_mode: CULL_MODE_NONE,
            front_face: FRONT_FACE_COUNTER_CLOCKWISE,
            depth_bias_enable: 0,
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            line_width: 1.0,
        };
        let ms = PipelineMultisampleStateCreateInfo {
            s_type: ST_PIPELINE_MULTISAMPLE_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            rasterization_samples: SAMPLE_COUNT_1,
            sample_shading_enable: 0,
            min_sample_shading: 0.0,
            p_sample_mask: std::ptr::null(),
            alpha_to_coverage_enable: 0,
            alpha_to_one_enable: 0,
        };
        let blend_att = PipelineColorBlendAttachmentState {
            blend_enable: 0,
            src_color_blend_factor: 0,
            dst_color_blend_factor: 0,
            color_blend_op: 0,
            src_alpha_blend_factor: 0,
            dst_alpha_blend_factor: 0,
            alpha_blend_op: 0,
            color_write_mask: COLOR_COMPONENT_RGBA,
        };
        let cb = PipelineColorBlendStateCreateInfo {
            s_type: ST_PIPELINE_COLOR_BLEND_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            logic_op_enable: 0,
            logic_op: 0,
            attachment_count: 1,
            p_attachments: &blend_att,
            blend_constants: [0.0; 4],
        };
        let gpci = GraphicsPipelineCreateInfo {
            s_type: ST_GRAPHICS_PIPELINE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            stage_count: 2,
            p_stages: stages.as_ptr(),
            p_vertex_input_state: &vin,
            p_input_assembly_state: &ia,
            p_tessellation_state: std::ptr::null(),
            p_viewport_state: &vp,
            p_rasterization_state: &rs,
            p_multisample_state: &ms,
            p_depth_stencil_state: std::ptr::null(),
            p_color_blend_state: &cb,
            p_dynamic_state: std::ptr::null(),
            layout: pipe_layout,
            render_pass,
            subpass: 0,
            base_pipeline_handle: VK_NULL_HANDLE,
            base_pipeline_index: -1,
        };
        let r = create_gp(
            device,
            VK_NULL_HANDLE,
            1,
            &gpci,
            std::ptr::null(),
            &mut pipeline,
        );
        if r != VK_SUCCESS {
            break 'run Err(format!("vkCreateGraphicsPipelines 失败: {r}"));
        }

        // ── command pool + buffer + 录制 ──
        let cpci = CommandPoolCreateInfo {
            s_type: ST_COMMAND_POOL_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            queue_family_index: qfi,
        };
        if create_cmdpool(device, &cpci, std::ptr::null(), &mut cmdpool) != VK_SUCCESS {
            break 'run Err("vkCreateCommandPool 失败".into());
        }
        let cbai = CommandBufferAllocateInfo {
            s_type: ST_COMMAND_BUFFER_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            command_pool: cmdpool,
            level: CMD_BUFFER_LEVEL_PRIMARY,
            command_buffer_count: 1,
        };
        let mut cmd: VkCommandBuffer = std::ptr::null_mut();
        alloc_cmd(device, &cbai, &mut cmd);
        let cbbi = CommandBufferBeginInfo {
            s_type: ST_COMMAND_BUFFER_BEGIN_INFO,
            p_next: std::ptr::null(),
            flags: CMD_BUFFER_USAGE_ONE_TIME_SUBMIT,
            p_inheritance_info: std::ptr::null(),
        };
        begin_cmd(cmd, &cbbi);

        let clear_val = ClearValue { color: clear };
        let rpbi = RenderPassBeginInfo {
            s_type: ST_RENDER_PASS_BEGIN_INFO,
            p_next: std::ptr::null(),
            render_pass,
            framebuffer,
            render_area: VkRect2D {
                offset: VkOffset2D { x: 0, y: 0 },
                extent: VkExtent2D { width, height },
            },
            clear_value_count: 1,
            p_clear_values: &clear_val,
        };
        cmd_begin_rp(cmd, &rpbi, SUBPASS_CONTENTS_INLINE);
        cmd_bind_pipe(cmd, PIPELINE_BIND_POINT_GRAPHICS, pipeline);
        let vbuf_offset: VkDeviceSize = 0;
        cmd_bind_vbuf(cmd, 0, 1, &vbuf, &vbuf_offset);
        let vertex_count = if vertex_stride > 0 {
            (vertices.len() / vertex_stride as usize) as u32
        } else {
            0
        };
        cmd_draw(cmd, vertex_count, 1, 0, 0);
        cmd_end_rp(cmd);

        // renderpass 结束后 image 已 TRANSFER_SRC_OPTIMAL;补 color-write→transfer-read
        // 内存可见性屏障(oldLayout==newLayout,仅内存/执行依赖)后 copy 到 readback。
        let barrier = ImageMemoryBarrier {
            s_type: ST_IMAGE_MEMORY_BARRIER,
            p_next: std::ptr::null(),
            src_access_mask: ACCESS_COLOR_ATTACHMENT_WRITE,
            dst_access_mask: ACCESS_TRANSFER_READ,
            old_layout: IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
            new_layout: IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
            src_queue_family_index: QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: QUEUE_FAMILY_IGNORED,
            image: color_image,
            subresource_range: VkImageSubresourceRange {
                aspect_mask: IMAGE_ASPECT_COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
        };
        cmd_barrier(
            cmd,
            PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT,
            PIPELINE_STAGE_TRANSFER,
            0,
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
            1,
            &barrier,
        );
        let region = VkBufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,   // 紧凑(= imageExtent.width)
            buffer_image_height: 0, // 紧凑(= imageExtent.height)
            image_subresource: VkImageSubresourceLayers {
                aspect_mask: IMAGE_ASPECT_COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_offset: VkOffset3D { x: 0, y: 0, z: 0 },
            image_extent: VkExtent3D {
                width,
                height,
                depth: 1,
            },
        };
        cmd_copy_img_buf(
            cmd,
            color_image,
            IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
            rbuf,
            1,
            &region,
        );
        end_cmd(cmd);

        // 提交 + 等待。
        let si = SubmitInfo {
            s_type: ST_SUBMIT_INFO,
            p_next: std::ptr::null(),
            wait_semaphore_count: 0,
            p_wait_semaphores: std::ptr::null(),
            p_wait_dst_stage_mask: std::ptr::null(),
            command_buffer_count: 1,
            p_command_buffers: &cmd,
            signal_semaphore_count: 0,
            p_signal_semaphores: std::ptr::null(),
        };
        let r = queue_submit(queue, 1, &si, VK_NULL_HANDLE);
        if r != VK_SUCCESS {
            break 'run Err(format!("vkQueueSubmit 失败: {r}"));
        }
        queue_wait(queue);

        // 回读紧凑 RGBA8。
        let mut ptr: *mut c_void = std::ptr::null_mut();
        if map_mem(device, rbuf_mem, 0, WHOLE_SIZE, 0, &mut ptr) != VK_SUCCESS {
            break 'run Err("回读 vkMapMemory 失败".into());
        }
        let mut pixels = vec![0u8; readback_len];
        std::ptr::copy_nonoverlapping(ptr.cast::<u8>(), pixels.as_mut_ptr(), readback_len);
        unmap_mem(device, rbuf_mem);
        Ok(pixels)
    };

    // ── 逆序销毁(非 null 者)──
    if cmdpool != VK_NULL_HANDLE {
        destroy_cmdpool(device, cmdpool, std::ptr::null());
    }
    if pipeline != VK_NULL_HANDLE {
        destroy_pipe(device, pipeline, std::ptr::null());
    }
    if pipe_layout != VK_NULL_HANDLE {
        destroy_pl(device, pipe_layout, std::ptr::null());
    }
    if fs_mod != VK_NULL_HANDLE {
        destroy_shader(device, fs_mod, std::ptr::null());
    }
    if vs_mod != VK_NULL_HANDLE {
        destroy_shader(device, vs_mod, std::ptr::null());
    }
    if rbuf != VK_NULL_HANDLE {
        destroy_buffer(device, rbuf, std::ptr::null());
    }
    if rbuf_mem != VK_NULL_HANDLE {
        free_mem(device, rbuf_mem, std::ptr::null());
    }
    if vbuf != VK_NULL_HANDLE {
        destroy_buffer(device, vbuf, std::ptr::null());
    }
    if vbuf_mem != VK_NULL_HANDLE {
        free_mem(device, vbuf_mem, std::ptr::null());
    }
    if framebuffer != VK_NULL_HANDLE {
        destroy_fb(device, framebuffer, std::ptr::null());
    }
    if render_pass != VK_NULL_HANDLE {
        destroy_rp(device, render_pass, std::ptr::null());
    }
    if color_view != VK_NULL_HANDLE {
        destroy_view(device, color_view, std::ptr::null());
    }
    if color_image != VK_NULL_HANDLE {
        destroy_image(device, color_image, std::ptr::null());
    }
    if color_mem != VK_NULL_HANDLE {
        free_mem(device, color_mem, std::ptr::null());
    }

    result
}

// ─────────────── graphics offscreen v2:descriptor 建面(RXS-0230,G3.3 PR-S0) ────────────
// RFC-0013 §4.B7:v1 `run_graphics_offscreen` 零 descriptor 面 → v2 加性 API 追加
// `resources: &[GraphicsResource]`(纹理逐层 mip / SamplerDesc / storage image),内部建
// Vk-native set-per-class descriptor 底座(采样 / bindless / graph / present 四面共用;
// set = 类别轴 0=CBV/1=SRV/2=UAV/3=Sampler、binding = 类内声明序,与 rurixc
// `binding_layout::infer_spirv_bindings_vk_native` 同一分配律)。v1 签名与行为 0-byte 保留
// (MB1 语料零回归,RXS-0230 L1);v2 独立组装(`graphics_body_v2`),不触 v1 路径字节。

// v2 句柄(non-dispatchable = u64)。
type VkSampler = u64;

// v2 sType / descriptor / usage / layout / access / stage 常量(RXS-0230)。
const ST_SAMPLER_CREATE_INFO: u32 = 31;
const DESCRIPTOR_TYPE_SAMPLER: u32 = 0;
const DESCRIPTOR_TYPE_SAMPLED_IMAGE: u32 = 2;
const DESCRIPTOR_TYPE_STORAGE_IMAGE: u32 = 3;
const IMAGE_USAGE_TRANSFER_DST: u32 = 0x2;
const IMAGE_USAGE_SAMPLED: u32 = 0x4;
const IMAGE_USAGE_STORAGE: u32 = 0x8;
const BUFFER_USAGE_TRANSFER_SRC: u32 = 0x1;
const IMAGE_LAYOUT_GENERAL: u32 = 1;
const IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL: u32 = 5;
const IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL: u32 = 7;
const ACCESS_TRANSFER_WRITE: u32 = 0x1000;
const ACCESS_SHADER_READ: u32 = 0x20;
const ACCESS_SHADER_WRITE: u32 = 0x40;
const PIPELINE_STAGE_TOP_OF_PIPE: u32 = 0x1;
const PIPELINE_STAGE_VERTEX_SHADER: u32 = 0x8;
const PIPELINE_STAGE_FRAGMENT_SHADER: u32 = 0x80;
const FORMAT_R32G32B32A32_SFLOAT: u32 = 109;
const BORDER_COLOR_FLOAT_OPAQUE_BLACK: u32 = 2;
/// `VK_LOD_CLAMP_NONE`(maxLod 语义上限;宿主 `f32::MAX` 语义等价,构造时钳此值)。
const LOD_CLAMP_NONE: f32 = 1000.0;
/// `VkPhysicalDeviceFeatures` 定长 VkBool32 字段数(Vulkan 1.0 固定 55,字段序 spec 固定)。
const PHYSICAL_DEVICE_FEATURE_COUNT: usize = 55;
/// `samplerAnisotropy` 字段序(0 基第 20 字段;sampler `max_anisotropy > 1` 需之)。
const FEATURE_SAMPLER_ANISOTROPY: usize = 19;
/// `fragmentStoresAndAtomics` 字段序(storage image 片元 store 需之;机会性启用)。
const FEATURE_FRAGMENT_STORES_AND_ATOMICS: usize = 26;

/// `VkPhysicalDeviceFeatures`(Vulkan 1.0;55 个 VkBool32 顺排——全字段同型,以定长数组
/// 建模逐字节对齐,按 spec 字段序索引〔`FEATURE_*` 常量〕)。
#[repr(C)]
struct PhysicalDeviceFeatures {
    features: [VkBool32; PHYSICAL_DEVICE_FEATURE_COUNT],
}

#[repr(C)]
struct SamplerCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    mag_filter: u32,
    min_filter: u32,
    mipmap_mode: u32,
    address_mode_u: u32,
    address_mode_v: u32,
    address_mode_w: u32,
    mip_lod_bias: f32,
    anisotropy_enable: VkBool32,
    max_anisotropy: f32,
    compare_enable: VkBool32,
    compare_op: u32,
    min_lod: f32,
    max_lod: f32,
    border_color: u32,
    unnormalized_coordinates: VkBool32,
}

#[repr(C)]
struct DescriptorImageInfo {
    sampler: VkSampler,
    image_view: VkImageView,
    image_layout: u32,
}

// v2 新增函数指针(既有 descriptor 集〔create_dsl/pool/alloc/update/bind〕复用 compute
// 先例的类型;此处仅 sampler / features / copy-buffer-to-image 三面为新)。
type FnGetPhysicalDeviceFeatures =
    unsafe extern "system" fn(VkPhysicalDevice, *mut PhysicalDeviceFeatures);
type FnCreateSampler = unsafe extern "system" fn(
    VkDevice,
    *const SamplerCreateInfo,
    *const c_void,
    *mut VkSampler,
) -> VkResult;
type FnDestroySampler = unsafe extern "system" fn(VkDevice, VkSampler, *const c_void);
type FnCmdCopyBufferToImage = unsafe extern "system" fn(
    VkCommandBuffer,
    VkBuffer,
    VkImage,
    u32,
    u32,
    *const VkBufferImageCopy,
);

/// v2 纹理逐层数据(RXS-0230;level 0 起,每层尺寸减半钳 1〔`mip_level_extents`〕)。
#[derive(Debug, Clone, PartialEq)]
pub enum TextureData {
    /// RGBA8 UNORM(4 字节/纹素;层字节数 = w×h×4)。
    Rgba8(Vec<Vec<u8>>),
    /// RGBA32 SFLOAT(4×f32/纹素;层 f32 数 = w×h×4)。
    RgbaF32(Vec<Vec<f32>>),
}

impl TextureData {
    /// mip 层数(= 数据层数)。
    fn level_count(&self) -> u32 {
        match self {
            TextureData::Rgba8(v) => v.len() as u32,
            TextureData::RgbaF32(v) => v.len() as u32,
        }
    }

    /// 纹素字节数(staging 布局用)。
    fn bytes_per_texel(&self) -> u64 {
        match self {
            TextureData::Rgba8(_) => 4,
            TextureData::RgbaF32(_) => 16,
        }
    }

    /// VkFormat(R8G8B8A8_UNORM / R32G32B32A32_SFLOAT)。
    fn vk_format(&self) -> u32 {
        match self {
            TextureData::Rgba8(_) => FORMAT_R8G8B8A8_UNORM,
            TextureData::RgbaF32(_) => FORMAT_R32G32B32A32_SFLOAT,
        }
    }
}

/// storage image 显式格式(RXS-0230;UAV 轴,承 §4.B5「`OpTypeImage` 带显式 format」纪律)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageFormat {
    /// R8G8B8A8_UNORM。
    Rgba8Unorm,
    /// R32G32B32A32_SFLOAT(`TextureRw2D<f32>` → Rgba32f)。
    Rgba32Float,
}

impl StorageFormat {
    fn vk_format(self) -> u32 {
        match self {
            StorageFormat::Rgba8Unorm => FORMAT_R8G8B8A8_UNORM,
            StorageFormat::Rgba32Float => FORMAT_R32G32B32A32_SFLOAT,
        }
    }
}

/// `run_graphics_offscreen_v2` 的 per-dispatch 有界资源(RXS-0230 / RFC-0013 §4.B7)。
///
/// **声明序须与着色器签名资源声明序一致**——binding 号 = 类内声明序,与 rurixc
/// `binding_layout::infer_spirv_bindings_vk_native`(单一 binding-号事实源)同序;三条
/// 资源下发路分工见 §4.0-6(本枚举 = 有界 per-dispatch 路,无界表 = §4.C4 `rxrt_table_*`)。
#[derive(Debug, Clone, PartialEq)]
pub enum GraphicsResource {
    /// SRV 轴 sampled 纹理(逐层 mip)→ set1,`VK_DESCRIPTOR_TYPE_SAMPLED_IMAGE`,
    /// `SHADER_READ_ONLY_OPTIMAL`。
    Texture2D {
        /// level 0 宽(≥1)。
        width: u32,
        /// level 0 高(≥1)。
        height: u32,
        /// 逐层数据(层数 = mip 数,1..=完整链)。
        data: TextureData,
    },
    /// Sampler 轴采样器(immutable sampler 进 set3 layout,RXS-0230 L3)。
    Sampler(SamplerDesc),
    /// UAV 轴 storage image → set2,`VK_DESCRIPTOR_TYPE_STORAGE_IMAGE`,`GENERAL` layout。
    StorageImage {
        /// 宽(≥1)。
        width: u32,
        /// 高(≥1)。
        height: u32,
        /// 显式格式。
        format: StorageFormat,
    },
}

// ── v2 纯 host 计划函数(无 FFI,单测锚定 RXS-0230) ─────────────────────────────

/// RXS-0230 IR2:Vk-native set-per-class 归类计划(host 纯函数)。返回逐资源
/// `(set, binding)`:set = 类别轴(1=SRV / 2=UAV / 3=Sampler;set0 = CBV 首期恒空占位),
/// binding = 类内声明序——与 rurixc `infer_spirv_bindings_vk_native` 同一分配律
/// (单一 binding-号事实源同序;两 crate 分立,镜像同一律)。
fn plan_descriptor_sets(resources: &[GraphicsResource]) -> Vec<(u32, u32)> {
    let (mut srv, mut uav, mut smp) = (0u32, 0u32, 0u32);
    resources
        .iter()
        .map(|r| match r {
            GraphicsResource::Texture2D { .. } => {
                let b = srv;
                srv += 1;
                (1, b)
            }
            GraphicsResource::StorageImage { .. } => {
                let b = uav;
                uav += 1;
                (2, b)
            }
            GraphicsResource::Sampler(_) => {
                let b = smp;
                smp += 1;
                (3, b)
            }
        })
        .collect()
}

/// set layout 数 = 最大使用 set + 1(中间未用 set〔含 set0 CBV〕以空 layout 占位,
/// 保 set 号 = 类别轴不平移;资源空 → 0,v2 入口直接委派 v1)。
fn planned_set_count(plans: &[(u32, u32)]) -> u32 {
    plans.iter().map(|&(s, _)| s + 1).max().unwrap_or(0)
}

/// descriptor pool sizes(host 纯函数):`(VkDescriptorType, count)`,确定性序
/// SAMPLED_IMAGE → STORAGE_IMAGE → SAMPLER,零计数不入。immutable sampler 仍占
/// SAMPLER pool 容量(分配含 immutable 绑定的 set 消耗之)。
fn plan_pool_sizes(resources: &[GraphicsResource]) -> Vec<(u32, u32)> {
    let mut tex = 0u32;
    let mut sto = 0u32;
    let mut smp = 0u32;
    for r in resources {
        match r {
            GraphicsResource::Texture2D { .. } => tex += 1,
            GraphicsResource::StorageImage { .. } => sto += 1,
            GraphicsResource::Sampler(_) => smp += 1,
        }
    }
    let mut out = Vec::new();
    if tex > 0 {
        out.push((DESCRIPTOR_TYPE_SAMPLED_IMAGE, tex));
    }
    if sto > 0 {
        out.push((DESCRIPTOR_TYPE_STORAGE_IMAGE, sto));
    }
    if smp > 0 {
        out.push((DESCRIPTOR_TYPE_SAMPLER, smp));
    }
    out
}

/// mip 尺寸序列(host 纯函数):`levels` 层,每层减半、钳 1(level 0 = `(width, height)`)。
fn mip_level_extents(width: u32, height: u32, levels: u32) -> Vec<(u32, u32)> {
    (0..levels)
        .map(|i| ((width >> i).max(1), (height >> i).max(1)))
        .collect()
}

/// 完整 mip 链层数:`floor(log2(max(w,h))) + 1`(w/h ≥ 1)。
fn full_mip_chain_len(width: u32, height: u32) -> u32 {
    32 - width.max(height).max(1).leading_zeros()
}

/// mip 链 staging 紧凑布局(host 纯函数):逐层偏移 + 总字节(层字节 = w×h×texel;偏移
/// 恒为纹素尺寸整倍 → 满足 `vkCmdCopyBufferToImage` bufferOffset 对齐 VUID)。
fn mip_staging_layout(extents: &[(u32, u32)], bytes_per_texel: u64) -> (u64, Vec<u64>) {
    let mut offsets = Vec::with_capacity(extents.len());
    let mut total = 0u64;
    for &(w, h) in extents {
        offsets.push(total);
        total += u64::from(w) * u64::from(h) * bytes_per_texel;
    }
    (total, offsets)
}

/// v2 资源合法性预校验(host 纯函数,任何句柄创建**前** fail-closed,P-01):纹理尺寸 /
/// mip 层数(1..=完整链)/ 逐层数据长(RGBA8 字节数与 RGBA-f32 f32 数同为 w×h×4)、
/// `SamplerDesc::is_valid`(RXS-0225 合法域)、storage 尺寸。Err = 确定性拒,非 panic。
fn validate_resources(resources: &[GraphicsResource]) -> Result<(), String> {
    for (i, r) in resources.iter().enumerate() {
        match r {
            GraphicsResource::Texture2D {
                width,
                height,
                data,
            } => {
                if *width == 0 || *height == 0 {
                    return Err(format!("resources[{i}]: 纹理尺寸须 ≥1({width}×{height})"));
                }
                let levels = data.level_count();
                if levels == 0 {
                    return Err(format!("resources[{i}]: 纹理须至少 1 个 mip 层"));
                }
                let full = full_mip_chain_len(*width, *height);
                if levels > full {
                    return Err(format!(
                        "resources[{i}]: mip 层数 {levels} 超完整链 {full}({width}×{height})"
                    ));
                }
                for (lvl, (w, h)) in mip_level_extents(*width, *height, levels)
                    .into_iter()
                    .enumerate()
                {
                    let want = (w as usize) * (h as usize) * 4;
                    let got = match data {
                        TextureData::Rgba8(v) => v[lvl].len(),
                        TextureData::RgbaF32(v) => v[lvl].len(),
                    };
                    if got != want {
                        return Err(format!(
                            "resources[{i}] level {lvl}: 数据长 {got} != {want}({w}×{h}×4)"
                        ));
                    }
                }
            }
            GraphicsResource::Sampler(desc) => {
                if !desc.is_valid() {
                    return Err(format!(
                        "resources[{i}]: SamplerDesc 非法(lod_bias 域 [-16,16) / max_anisotropy ≥ 1)"
                    ));
                }
            }
            GraphicsResource::StorageImage { width, height, .. } => {
                if *width == 0 || *height == 0 {
                    return Err(format!(
                        "resources[{i}]: storage image 尺寸须 ≥1({width}×{height})"
                    ));
                }
            }
        }
    }
    Ok(())
}

/// `SamplerDesc`(经 RXS-0225 [`SamplerDesc::vk_fields`] plain 降级)→ `VkSamplerCreateInfo`
/// 构造(RXS-0230 L3;host 纯构造,可单测)。UVW 三向同值(首期);maxLod 钳
/// `VK_LOD_CLAMP_NONE`(1000.0,宿主 `f32::MAX` 语义等价);border 预置 FLOAT_OPAQUE_BLACK;
/// 归一化坐标恒开。
fn sampler_create_info(desc: &SamplerDesc) -> SamplerCreateInfo {
    let f = desc.vk_fields();
    SamplerCreateInfo {
        s_type: ST_SAMPLER_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        mag_filter: f.mag_min_filter,
        min_filter: f.mag_min_filter,
        mipmap_mode: f.mipmap_mode,
        address_mode_u: f.address_mode,
        address_mode_v: f.address_mode,
        address_mode_w: f.address_mode,
        mip_lod_bias: f.mip_lod_bias,
        anisotropy_enable: f.anisotropy_enable,
        max_anisotropy: f.max_anisotropy,
        compare_enable: f.compare_enable,
        compare_op: f.compare_op,
        min_lod: f.min_lod,
        max_lod: if f.max_lod > LOD_CLAMP_NONE {
            LOD_CLAMP_NONE
        } else {
            f.max_lod
        },
        border_color: BORDER_COLOR_FLOAT_OPAQUE_BLACK,
        unnormalized_coordinates: 0,
    }
}

/// v2 纹理句柄组(线性生命周期,末尾统一逆序销毁;`VK_NULL_HANDLE` = 未建)。
struct V2Tex {
    image: VkImage,
    mem: VkDeviceMemory,
    view: VkImageView,
    staging: VkBuffer,
    staging_mem: VkDeviceMemory,
    levels: u32,
}

/// v2 storage image 句柄组。
struct V2Storage {
    image: VkImage,
    mem: VkDeviceMemory,
    view: VkImageView,
}

/// offscreen 渲染一帧并回读(**v2 加性 API**,RXS-0230 / RFC-0013 §4.B7):v1 全参语义
/// 不变,追加 `resources`——SRV 纹理(逐层 mip 经 staging 上传 + `TRANSFER_DST→
/// SHADER_READ_ONLY` 迁移)/ immutable sampler(进 set3 layout,RXS-0230 L3)/ storage
/// image(`GENERAL`)。descriptor 建面 set-per-class(set0 CBV 空占位 / set1 SRV /
/// set2 UAV / set3 Sampler,binding = 类内声明序,与 rurixc
/// `infer_spirv_bindings_vk_native` 同一分配律);pipeline layout 由 v1 的空 layout 扩为
/// 含 set layouts(**v1 路径不动,v2 独立组装**)。`resources` 为空 ≡ v1(直接委派)。
///
/// sampler `max_anisotropy > 1` → device 探测 `samplerAnisotropy`,缺失 → 确定性 `Err`
/// (RXS-0193 口径,不占 RX 码);storage image 在位时机会性启用
/// `fragmentStoresAndAtomics`(片元 store 需之;缺失不 Err——绑定本身不需该 feature)。
/// 缺 Vulkan / 资源非法 → 确定性 `Err`(P-01 fail-closed)。device 数值见证(≥6 模式,
/// 步骤 63)归主循环;本入口交付编译绿 + host 单测。
///
/// # SAFETY(U27 扩注,graphics FFI 边界,0 新 U 号)
/// 对上全 safe(无 `unsafe` 签名)。内部 `run_graphics_inner_v2`/`graphics_body_v2` 契约
/// 同 v1 U27:句柄(sampler×N / texture image·mem·view·staging×N / storage
/// image·mem·view×N / setLayout×N / descriptorPool + v1 全集)线性配对 create/destroy、
/// 末尾逆序销毁(pool 先于 setLayout,sampler 在 pool/layout 之后);资源合法性在任何
/// 句柄创建**前**纯 host 预校验;单 graphics queue 同步提交 + `vkQueueWaitIdle` 后回读。
//@ spec: RXS-0230
#[allow(clippy::too_many_arguments)]
pub fn run_graphics_offscreen_v2(
    vs_spv: &[u32],
    fs_spv: &[u32],
    vertices: &[u8],
    vertex_stride: u32,
    attrs: &[(u32, u32, u32)],
    width: u32,
    height: u32,
    clear: [f32; 4],
    resources: &[GraphicsResource],
) -> Result<Vec<u8>, String> {
    run_graphics_offscreen_v2_dispatch(
        vs_spv,
        fs_spv,
        vertices,
        vertex_stride,
        attrs,
        width,
        height,
        clear,
        resources,
        /*readback_storage=*/ false,
    )
    .map(|(color, _storage)| color)
}

/// [`run_graphics_offscreen_v2`] 加 **storage image 回读出口**(RXS-0230 / RFC-0013
/// §4.B5/B8 模式⑤ TextureRw2D 唯一写者 store→回读)。返回 `(color, storage)`:`color` 同
/// v2 颜色附件回读;`storage` = **首个** `GraphicsResource::StorageImage` 经 pass 边界
/// barrier(shader-write→transfer-read,GENERAL 保持)+ `vkCmdCopyImageToBuffer` 回读的
/// 逐纹素字节(`Rgba8Unorm`=4B/纹素、`Rgba32Float`=16B/纹素;小端),无 storage image 时
/// `None`。**唯一写者纪律**(§4.B5)保证回读值 well-defined。device 数值判据(identity 坐标
/// 见证 + 篡改红绿阈值)归 `bin/sampling_modes` + 步骤 63 owner 本机。
///
/// # SAFETY(U27 扩注,graphics FFI 边界,0 新 U 号)
/// 契约同 [`run_graphics_offscreen_v2`];额外 storage 回读缓冲(host-visible TRANSFER_DST)
/// 与其余句柄同线性配对 create/destroy、末尾逆序销毁,`vkCmdCopyImageToBuffer` 在同一命令
/// 缓冲的 pass 结束 barrier 之后录制,单 queue 同步提交 + `vkQueueWaitIdle` 后映射回读。
//@ spec: RXS-0230
#[allow(clippy::too_many_arguments)]
pub fn run_graphics_offscreen_v2_readback(
    vs_spv: &[u32],
    fs_spv: &[u32],
    vertices: &[u8],
    vertex_stride: u32,
    attrs: &[(u32, u32, u32)],
    width: u32,
    height: u32,
    clear: [f32; 4],
    resources: &[GraphicsResource],
) -> Result<(Vec<u8>, Option<Vec<u8>>), String> {
    run_graphics_offscreen_v2_dispatch(
        vs_spv,
        fs_spv,
        vertices,
        vertex_stride,
        attrs,
        width,
        height,
        clear,
        resources,
        /*readback_storage=*/ true,
    )
}

/// v2 公共派发(RXS-0230):校验 + 空资源委派 v1 + loader + `unsafe` 内部。`readback_storage`
/// 决定是否额外回读首个 storage image(§4.B8 模式⑤)。
#[allow(clippy::too_many_arguments)]
fn run_graphics_offscreen_v2_dispatch(
    vs_spv: &[u32],
    fs_spv: &[u32],
    vertices: &[u8],
    vertex_stride: u32,
    attrs: &[(u32, u32, u32)],
    width: u32,
    height: u32,
    clear: [f32; 4],
    resources: &[GraphicsResource],
    readback_storage: bool,
) -> Result<(Vec<u8>, Option<Vec<u8>>), String> {
    validate_resources(resources)?;
    if resources.is_empty() {
        // 空资源 ≡ v1 行为(加性中性;RXS-0230 L1)。storage 回读恒 None(无 storage image)。
        return run_graphics_offscreen(
            vs_spv,
            fs_spv,
            vertices,
            vertex_stride,
            attrs,
            width,
            height,
            clear,
        )
        .map(|color| (color, None));
    }
    let gipa = load_vulkan_loader().ok_or("vulkan loader (vulkan-1.dll/libvulkan.so) 不可用")?;
    // SAFETY: 见 U27 v2 扩注(上);句柄生命周期由内部函数线性管理,末尾逆序销毁。
    unsafe {
        run_graphics_inner_v2(
            gipa,
            vs_spv,
            fs_spv,
            vertices,
            vertex_stride,
            attrs,
            width,
            height,
            clear,
            resources,
            readback_storage,
        )
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn run_graphics_inner_v2(
    gipa: FnGetInstanceProcAddr,
    vs_spv: &[u32],
    fs_spv: &[u32],
    vertices: &[u8],
    vertex_stride: u32,
    attrs: &[(u32, u32, u32)],
    width: u32,
    height: u32,
    clear: [f32; 4],
    resources: &[GraphicsResource],
    readback_storage: bool,
) -> Result<(Vec<u8>, Option<Vec<u8>>), String> {
    let vk_create_instance: FnCreateInstance =
        cast_fn(gipa(std::ptr::null_mut(), c"vkCreateInstance".as_ptr()))
            .ok_or("缺 vkCreateInstance")?;

    let validation = std::env::var("RURIX_VK_VALIDATION").as_deref() == Ok("1");
    let layer_name = c"VK_LAYER_KHRONOS_validation";
    let layers: [*const c_char; 1] = [layer_name.as_ptr()];
    let debug_ext = c"VK_EXT_debug_utils";
    let exts: [*const c_char; 1] = [debug_ext.as_ptr()];
    let app = ApplicationInfo {
        s_type: ST_APPLICATION_INFO,
        p_next: std::ptr::null(),
        p_application_name: c"rurix-mb1".as_ptr(),
        application_version: 0,
        p_engine_name: c"rurix".as_ptr(),
        engine_version: 0,
        api_version: API_VERSION_1_1,
    };
    let ici = InstanceCreateInfo {
        s_type: ST_INSTANCE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        p_application_info: &app,
        enabled_layer_count: if validation { 1 } else { 0 },
        pp_enabled_layer_names: if validation {
            layers.as_ptr()
        } else {
            std::ptr::null()
        },
        enabled_extension_count: if validation { 1 } else { 0 },
        pp_enabled_extension_names: if validation {
            exts.as_ptr()
        } else {
            std::ptr::null()
        },
    };
    let mut instance: VkInstance = std::ptr::null_mut();
    let r = vk_create_instance(&ici, std::ptr::null(), &mut instance);
    if r != VK_SUCCESS {
        return Err(format!("vkCreateInstance 失败: {r}"));
    }

    // instance 级符号(v1 全集 + `vkGetPhysicalDeviceFeatures`;messenger 创建前全部
    // `?` 完毕——镜像 v1 U27「创建点与首个销毁点之间无 `?` 早退」契约)。
    let vk_destroy_instance: FnDestroyInstance =
        cast_fn(gipa(instance, c"vkDestroyInstance".as_ptr())).ok_or("缺 vkDestroyInstance")?;
    let vk_enum_pd: FnEnumeratePhysicalDevices =
        cast_fn(gipa(instance, c"vkEnumeratePhysicalDevices".as_ptr()))
            .ok_or("缺 vkEnumeratePhysicalDevices")?;
    let vk_get_qf: FnGetPhysicalDeviceQueueFamilyProperties = cast_fn(gipa(
        instance,
        c"vkGetPhysicalDeviceQueueFamilyProperties".as_ptr(),
    ))
    .ok_or("缺 vkGetPhysicalDeviceQueueFamilyProperties")?;
    let vk_get_mem: FnGetPhysicalDeviceMemoryProperties = cast_fn(gipa(
        instance,
        c"vkGetPhysicalDeviceMemoryProperties".as_ptr(),
    ))
    .ok_or("缺 vkGetPhysicalDeviceMemoryProperties")?;
    let vk_create_device: FnCreateDevice =
        cast_fn(gipa(instance, c"vkCreateDevice".as_ptr())).ok_or("缺 vkCreateDevice")?;
    let vk_get_device_proc: FnGetDeviceProcAddr =
        cast_fn(gipa(instance, c"vkGetDeviceProcAddr".as_ptr())).ok_or("缺 vkGetDeviceProcAddr")?;
    let vk_get_features: FnGetPhysicalDeviceFeatures =
        cast_fn(gipa(instance, c"vkGetPhysicalDeviceFeatures".as_ptr()))
            .ok_or("缺 vkGetPhysicalDeviceFeatures")?;

    // fail-closed messenger(L3,RXS-0210 契约复用;ERROR 级校验消息 → 末尾翻 `Err`)。
    let validation_error = std::sync::atomic::AtomicBool::new(false);
    let mut messenger: VkDebugUtilsMessengerEXT = VK_NULL_HANDLE;
    let destroy_messenger: Option<FnDestroyDebugUtilsMessengerEXT> = if validation {
        cast_fn(gipa(instance, c"vkDestroyDebugUtilsMessengerEXT".as_ptr()))
    } else {
        None
    };
    if validation
        && let Some(create_messenger) = cast_fn::<FnCreateDebugUtilsMessengerEXT>(gipa(
            instance,
            c"vkCreateDebugUtilsMessengerEXT".as_ptr(),
        ))
    {
        let dumci = DebugUtilsMessengerCreateInfoEXT {
            s_type: ST_DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
            p_next: std::ptr::null(),
            flags: 0,
            message_severity: DEBUG_UTILS_SEVERITY_ERROR,
            message_type: DEBUG_UTILS_TYPE_GENERAL
                | DEBUG_UTILS_TYPE_VALIDATION
                | DEBUG_UTILS_TYPE_PERFORMANCE,
            pfn_user_callback: debug_messenger_cb,
            p_user_data: &validation_error as *const std::sync::atomic::AtomicBool as *mut c_void,
        };
        let _ = create_messenger(instance, &dumci, std::ptr::null(), &mut messenger);
    }
    macro_rules! destroy_msgr {
        () => {
            if let Some(dm) = destroy_messenger {
                if messenger != VK_NULL_HANDLE {
                    dm(instance, messenger, std::ptr::null());
                }
            }
        };
    }

    let mut count = 0u32;
    vk_enum_pd(instance, &mut count, std::ptr::null_mut());
    if count == 0 {
        destroy_msgr!();
        vk_destroy_instance(instance, std::ptr::null());
        return Err("无 Vulkan 物理设备".into());
    }
    let mut pds = vec![std::ptr::null_mut::<c_void>(); count as usize];
    vk_enum_pd(instance, &mut count, pds.as_mut_ptr());
    let pd = pds[0];

    let mut qf_count = 0u32;
    vk_get_qf(pd, &mut qf_count, std::ptr::null_mut());
    let mut qfs: Vec<QueueFamilyProperties> = (0..qf_count)
        .map(|_| QueueFamilyProperties {
            queue_flags: 0,
            queue_count: 0,
            timestamp_valid_bits: 0,
            min_image_transfer_granularity: VkExtent3D {
                width: 0,
                height: 0,
                depth: 0,
            },
        })
        .collect();
    vk_get_qf(pd, &mut qf_count, qfs.as_mut_ptr());
    let qfi = match qfs
        .iter()
        .position(|q| q.queue_flags & QUEUE_GRAPHICS_BIT != 0)
    {
        Some(i) => i as u32,
        None => {
            destroy_msgr!();
            vk_destroy_instance(instance, std::ptr::null());
            return Err("无 graphics queue family".into());
        }
    };

    // ── device feature 协商(RXS-0225 aniso 探测 + storage 片元写机会性启用)──
    let needs_aniso = resources
        .iter()
        .any(|r| matches!(r, GraphicsResource::Sampler(d) if d.max_anisotropy > 1));
    let has_storage = resources
        .iter()
        .any(|r| matches!(r, GraphicsResource::StorageImage { .. }));
    let mut supported = PhysicalDeviceFeatures {
        features: [0; PHYSICAL_DEVICE_FEATURE_COUNT],
    };
    vk_get_features(pd, &mut supported);
    if needs_aniso && supported.features[FEATURE_SAMPLER_ANISOTROPY] == 0 {
        destroy_msgr!();
        vk_destroy_instance(instance, std::ptr::null());
        return Err(
            "sampler max_anisotropy>1 但设备缺 samplerAnisotropy feature(确定性 Err,RXS-0193 口径)"
                .into(),
        );
    }
    let mut enabled = PhysicalDeviceFeatures {
        features: [0; PHYSICAL_DEVICE_FEATURE_COUNT],
    };
    if needs_aniso {
        enabled.features[FEATURE_SAMPLER_ANISOTROPY] = 1;
    }
    if has_storage && supported.features[FEATURE_FRAGMENT_STORES_AND_ATOMICS] != 0 {
        enabled.features[FEATURE_FRAGMENT_STORES_AND_ATOMICS] = 1;
    }

    let prio = [1.0f32];
    let dqci = DeviceQueueCreateInfo {
        s_type: ST_DEVICE_QUEUE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        queue_family_index: qfi,
        queue_count: 1,
        p_queue_priorities: prio.as_ptr(),
    };
    let dci = DeviceCreateInfo {
        s_type: ST_DEVICE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        queue_create_info_count: 1,
        p_queue_create_infos: &dqci,
        enabled_layer_count: 0,
        pp_enabled_layer_names: std::ptr::null(),
        enabled_extension_count: 0,
        pp_enabled_extension_names: std::ptr::null(),
        p_enabled_features: (&enabled as *const PhysicalDeviceFeatures).cast::<c_void>(),
    };
    let mut device: VkDevice = std::ptr::null_mut();
    let r = vk_create_device(pd, &dci, std::ptr::null(), &mut device);
    if r != VK_SUCCESS {
        destroy_msgr!();
        vk_destroy_instance(instance, std::ptr::null());
        return Err(format!("vkCreateDevice 失败: {r}"));
    }

    let mut out = graphics_body_v2(
        vk_get_device_proc,
        device,
        pd,
        vk_get_mem,
        qfi,
        vs_spv,
        fs_spv,
        vertices,
        vertex_stride,
        attrs,
        width,
        height,
        clear,
        resources,
        readback_storage,
    );

    if validation && validation_error.load(std::sync::atomic::Ordering::Relaxed) {
        out =
            Err("VK_LAYER_KHRONOS_validation 报 ERROR 级校验错误(见 stderr;fail-closed,L3)".into());
    }

    let vk_destroy_device: Option<FnDestroyDevice> =
        cast_fn(vk_get_device_proc(device, c"vkDestroyDevice".as_ptr()));
    if let Some(dd) = vk_destroy_device {
        dd(device, std::ptr::null());
    }
    destroy_msgr!();
    vk_destroy_instance(instance, std::ptr::null());
    out
}

#[allow(clippy::too_many_arguments)]
unsafe fn graphics_body_v2(
    gdpa: FnGetDeviceProcAddr,
    device: VkDevice,
    pd: VkPhysicalDevice,
    vk_get_mem: FnGetPhysicalDeviceMemoryProperties,
    qfi: u32,
    vs_spv: &[u32],
    fs_spv: &[u32],
    vertices: &[u8],
    vertex_stride: u32,
    attrs: &[(u32, u32, u32)],
    width: u32,
    height: u32,
    clear: [f32; 4],
    resources: &[GraphicsResource],
    readback_storage: bool,
) -> Result<(Vec<u8>, Option<Vec<u8>>), String> {
    macro_rules! dp {
        ($name:literal, $ty:ty) => {
            cast_fn::<$ty>(gdpa(device, $name.as_ptr())).ok_or("缺 device 符号")?
        };
    }
    // v1 符号全集(镜像 graphics_body)。
    let get_queue: FnGetDeviceQueue = dp!(c"vkGetDeviceQueue", FnGetDeviceQueue);
    let create_buffer: FnCreateBuffer = dp!(c"vkCreateBuffer", FnCreateBuffer);
    let destroy_buffer: FnDestroyBuffer = dp!(c"vkDestroyBuffer", FnDestroyBuffer);
    let buf_mem_req: FnGetBufferMemoryRequirements = dp!(
        c"vkGetBufferMemoryRequirements",
        FnGetBufferMemoryRequirements
    );
    let alloc_mem: FnAllocateMemory = dp!(c"vkAllocateMemory", FnAllocateMemory);
    let free_mem: FnFreeMemory = dp!(c"vkFreeMemory", FnFreeMemory);
    let bind_buf: FnBindBufferMemory = dp!(c"vkBindBufferMemory", FnBindBufferMemory);
    let map_mem: FnMapMemory = dp!(c"vkMapMemory", FnMapMemory);
    let unmap_mem: FnUnmapMemory = dp!(c"vkUnmapMemory", FnUnmapMemory);
    let create_shader: FnCreateShaderModule = dp!(c"vkCreateShaderModule", FnCreateShaderModule);
    let destroy_shader: FnDestroyShaderModule =
        dp!(c"vkDestroyShaderModule", FnDestroyShaderModule);
    let create_pl: FnCreatePipelineLayout = dp!(c"vkCreatePipelineLayout", FnCreatePipelineLayout);
    let destroy_pl: FnDestroyPipelineLayout =
        dp!(c"vkDestroyPipelineLayout", FnDestroyPipelineLayout);
    let destroy_pipe: FnDestroyPipeline = dp!(c"vkDestroyPipeline", FnDestroyPipeline);
    let create_cmdpool: FnCreateCommandPool = dp!(c"vkCreateCommandPool", FnCreateCommandPool);
    let destroy_cmdpool: FnDestroyCommandPool = dp!(c"vkDestroyCommandPool", FnDestroyCommandPool);
    let alloc_cmd: FnAllocateCommandBuffers =
        dp!(c"vkAllocateCommandBuffers", FnAllocateCommandBuffers);
    let begin_cmd: FnBeginCommandBuffer = dp!(c"vkBeginCommandBuffer", FnBeginCommandBuffer);
    let end_cmd: FnEndCommandBuffer = dp!(c"vkEndCommandBuffer", FnEndCommandBuffer);
    let cmd_bind_pipe: FnCmdBindPipeline = dp!(c"vkCmdBindPipeline", FnCmdBindPipeline);
    let queue_submit: FnQueueSubmit = dp!(c"vkQueueSubmit", FnQueueSubmit);
    let queue_wait: FnQueueWaitIdle = dp!(c"vkQueueWaitIdle", FnQueueWaitIdle);
    let create_image: FnCreateImage = dp!(c"vkCreateImage", FnCreateImage);
    let destroy_image: FnDestroyImage = dp!(c"vkDestroyImage", FnDestroyImage);
    let img_mem_req: FnGetImageMemoryRequirements = dp!(
        c"vkGetImageMemoryRequirements",
        FnGetImageMemoryRequirements
    );
    let bind_image: FnBindImageMemory = dp!(c"vkBindImageMemory", FnBindImageMemory);
    let create_view: FnCreateImageView = dp!(c"vkCreateImageView", FnCreateImageView);
    let destroy_view: FnDestroyImageView = dp!(c"vkDestroyImageView", FnDestroyImageView);
    let create_rp: FnCreateRenderPass = dp!(c"vkCreateRenderPass", FnCreateRenderPass);
    let destroy_rp: FnDestroyRenderPass = dp!(c"vkDestroyRenderPass", FnDestroyRenderPass);
    let create_fb: FnCreateFramebuffer = dp!(c"vkCreateFramebuffer", FnCreateFramebuffer);
    let destroy_fb: FnDestroyFramebuffer = dp!(c"vkDestroyFramebuffer", FnDestroyFramebuffer);
    let create_gp: FnCreateGraphicsPipelines =
        dp!(c"vkCreateGraphicsPipelines", FnCreateGraphicsPipelines);
    let cmd_begin_rp: FnCmdBeginRenderPass = dp!(c"vkCmdBeginRenderPass", FnCmdBeginRenderPass);
    let cmd_end_rp: FnCmdEndRenderPass = dp!(c"vkCmdEndRenderPass", FnCmdEndRenderPass);
    let cmd_bind_vbuf: FnCmdBindVertexBuffers =
        dp!(c"vkCmdBindVertexBuffers", FnCmdBindVertexBuffers);
    let cmd_draw: FnCmdDraw = dp!(c"vkCmdDraw", FnCmdDraw);
    let cmd_barrier: FnCmdPipelineBarrier = dp!(c"vkCmdPipelineBarrier", FnCmdPipelineBarrier);
    let cmd_copy_img_buf: FnCmdCopyImageToBuffer =
        dp!(c"vkCmdCopyImageToBuffer", FnCmdCopyImageToBuffer);
    // descriptor 建面符号(compute `run_on_device` 先例镜像,graphics 路首次消费)。
    let create_dsl: FnCreateDescriptorSetLayout =
        dp!(c"vkCreateDescriptorSetLayout", FnCreateDescriptorSetLayout);
    let destroy_dsl: FnDestroyDescriptorSetLayout = dp!(
        c"vkDestroyDescriptorSetLayout",
        FnDestroyDescriptorSetLayout
    );
    let create_dpool: FnCreateDescriptorPool =
        dp!(c"vkCreateDescriptorPool", FnCreateDescriptorPool);
    let destroy_dpool: FnDestroyDescriptorPool =
        dp!(c"vkDestroyDescriptorPool", FnDestroyDescriptorPool);
    let alloc_ds: FnAllocateDescriptorSets =
        dp!(c"vkAllocateDescriptorSets", FnAllocateDescriptorSets);
    let update_ds: FnUpdateDescriptorSets = dp!(c"vkUpdateDescriptorSets", FnUpdateDescriptorSets);
    let cmd_bind_ds: FnCmdBindDescriptorSets =
        dp!(c"vkCmdBindDescriptorSets", FnCmdBindDescriptorSets);
    // v2 新增符号(sampler / staging→image copy)。
    let create_sampler: FnCreateSampler = dp!(c"vkCreateSampler", FnCreateSampler);
    let destroy_sampler: FnDestroySampler = dp!(c"vkDestroySampler", FnDestroySampler);
    let cmd_copy_buf_img: FnCmdCopyBufferToImage =
        dp!(c"vkCmdCopyBufferToImage", FnCmdCopyBufferToImage);

    let mut queue: VkQueue = std::ptr::null_mut();
    get_queue(device, qfi, 0, &mut queue);

    let mut memprops = std::mem::zeroed::<PhysicalDeviceMemoryProperties>();
    vk_get_mem(pd, &mut memprops);

    let readback_len = (width as usize) * (height as usize) * 4;

    // 计划(纯函数;资源已在公共入口预校验,resources 非空)。
    let plans = plan_descriptor_sets(resources);
    let set_count = planned_set_count(&plans);
    let pool_sizes_plan = plan_pool_sizes(resources);

    // host-visible buffer 建立 helper(顶点/回读/staging 共用;镜像 v1)。
    let make_host_buffer = |usage: u32, size: u64| -> Result<(VkBuffer, VkDeviceMemory), String> {
        let bci = BufferCreateInfo {
            s_type: ST_BUFFER_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            size: size.max(4),
            usage,
            sharing_mode: SHARING_MODE_EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
        };
        let mut buffer: VkBuffer = VK_NULL_HANDLE;
        if create_buffer(device, &bci, std::ptr::null(), &mut buffer) != VK_SUCCESS {
            return Err("vkCreateBuffer 失败".into());
        }
        let mut req = std::mem::zeroed::<MemoryRequirements>();
        buf_mem_req(device, buffer, &mut req);
        let Some(mt) = pick_mem_type(
            &memprops,
            req.memory_type_bits,
            MEM_HOST_VISIBLE | MEM_HOST_COHERENT,
        ) else {
            destroy_buffer(device, buffer, std::ptr::null());
            return Err("无 host-visible+coherent 内存类型".into());
        };
        let mai = MemoryAllocateInfo {
            s_type: ST_MEMORY_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            allocation_size: req.size,
            memory_type_index: mt,
        };
        let mut mem: VkDeviceMemory = VK_NULL_HANDLE;
        if alloc_mem(device, &mai, std::ptr::null(), &mut mem) != VK_SUCCESS {
            destroy_buffer(device, buffer, std::ptr::null());
            return Err("vkAllocateMemory 失败".into());
        }
        bind_buf(device, buffer, mem, 0);
        Ok((buffer, mem))
    };

    // device-local image 建立 helper(v2 纹理 / storage 共用)。
    let make_image = |w: u32,
                      h: u32,
                      mips: u32,
                      format: u32,
                      usage: u32|
     -> Result<(VkImage, VkDeviceMemory), String> {
        let ici = ImageCreateInfo {
            s_type: ST_IMAGE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            image_type: IMAGE_TYPE_2D,
            format,
            extent: VkExtent3D {
                width: w,
                height: h,
                depth: 1,
            },
            mip_levels: mips,
            array_layers: 1,
            samples: SAMPLE_COUNT_1,
            tiling: IMAGE_TILING_OPTIMAL,
            usage,
            sharing_mode: SHARING_MODE_EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
            initial_layout: IMAGE_LAYOUT_UNDEFINED,
        };
        let mut image: VkImage = VK_NULL_HANDLE;
        if create_image(device, &ici, std::ptr::null(), &mut image) != VK_SUCCESS {
            return Err("v2 vkCreateImage 失败".into());
        }
        let mut req = std::mem::zeroed::<MemoryRequirements>();
        img_mem_req(device, image, &mut req);
        let Some(mt) = pick_mem_type(&memprops, req.memory_type_bits, MEM_DEVICE_LOCAL) else {
            destroy_image(device, image, std::ptr::null());
            return Err("无 device-local 内存类型".into());
        };
        let mai = MemoryAllocateInfo {
            s_type: ST_MEMORY_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            allocation_size: req.size,
            memory_type_index: mt,
        };
        let mut mem: VkDeviceMemory = VK_NULL_HANDLE;
        if alloc_mem(device, &mai, std::ptr::null(), &mut mem) != VK_SUCCESS {
            destroy_image(device, image, std::ptr::null());
            return Err("v2 image vkAllocateMemory 失败".into());
        }
        bind_image(device, image, mem, 0);
        Ok((image, mem))
    };

    // image view 建立 helper(levels 层全视图)。
    let make_view = |image: VkImage, format: u32, levels: u32| -> Result<VkImageView, String> {
        let vci = ImageViewCreateInfo {
            s_type: ST_IMAGE_VIEW_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            image,
            view_type: IMAGE_VIEW_TYPE_2D,
            format,
            components: VkComponentMapping {
                r: COMPONENT_SWIZZLE_IDENTITY,
                g: COMPONENT_SWIZZLE_IDENTITY,
                b: COMPONENT_SWIZZLE_IDENTITY,
                a: COMPONENT_SWIZZLE_IDENTITY,
            },
            subresource_range: VkImageSubresourceRange {
                aspect_mask: IMAGE_ASPECT_COLOR,
                base_mip_level: 0,
                level_count: levels,
                base_array_layer: 0,
                layer_count: 1,
            },
        };
        let mut view: VkImageView = VK_NULL_HANDLE;
        if create_view(device, &vci, std::ptr::null(), &mut view) != VK_SUCCESS {
            return Err("v2 vkCreateImageView 失败".into());
        }
        Ok(view)
    };

    // 句柄(全 null 初始,末尾逆序销毁非 null 者)。
    let mut samplers: Vec<VkSampler> = Vec::new();
    let mut texs: Vec<V2Tex> = Vec::new();
    let mut storages: Vec<V2Storage> = Vec::new();
    let mut set_layouts: Vec<VkDescriptorSetLayout> = Vec::new();
    let mut desc_pool: VkDescriptorPool = VK_NULL_HANDLE;
    let mut color_image: VkImage = VK_NULL_HANDLE;
    let mut color_mem: VkDeviceMemory = VK_NULL_HANDLE;
    let mut color_view: VkImageView = VK_NULL_HANDLE;
    let mut render_pass: VkRenderPass = VK_NULL_HANDLE;
    let mut framebuffer: VkFramebuffer = VK_NULL_HANDLE;
    let mut vbuf: VkBuffer = VK_NULL_HANDLE;
    let mut vbuf_mem: VkDeviceMemory = VK_NULL_HANDLE;
    let mut rbuf: VkBuffer = VK_NULL_HANDLE;
    let mut rbuf_mem: VkDeviceMemory = VK_NULL_HANDLE;
    // storage image 回读缓冲(§4.B8 模式⑤;仅 readback_storage && 存在 storage image 时建)。
    let mut srbuf: VkBuffer = VK_NULL_HANDLE;
    let mut srbuf_mem: VkDeviceMemory = VK_NULL_HANDLE;
    let mut vs_mod: VkShaderModule = VK_NULL_HANDLE;
    let mut fs_mod: VkShaderModule = VK_NULL_HANDLE;
    let mut pipe_layout: VkPipelineLayout = VK_NULL_HANDLE;
    let mut pipeline: VkPipeline = VK_NULL_HANDLE;
    let mut cmdpool: VkCommandPool = VK_NULL_HANDLE;

    // 首个 storage image 的回读几何(w, h, 字节/纹素);无 storage / 不回读 → None。
    let storage_readback: Option<(u32, u32, u64)> = if readback_storage {
        resources.iter().find_map(|r| match r {
            GraphicsResource::StorageImage {
                width: sw,
                height: sh,
                format,
            } => {
                let bpt: u64 = match format {
                    StorageFormat::Rgba8Unorm => 4,
                    StorageFormat::Rgba32Float => 16,
                };
                Some((*sw, *sh, bpt))
            }
            _ => None,
        })
    } else {
        None
    };
    let storage_readback_len: usize = storage_readback
        .map(|(w, h, bpt)| (w as usize) * (h as usize) * (bpt as usize))
        .unwrap_or(0);

    let result: Result<(Vec<u8>, Option<Vec<u8>>), String> = 'run: {
        // ── ① samplers(声明序 = Sampler 轴 binding 序;immutable 进 set3 layout)──
        for r in resources {
            if let GraphicsResource::Sampler(desc) = r {
                let sci = sampler_create_info(desc);
                let mut s: VkSampler = VK_NULL_HANDLE;
                if create_sampler(device, &sci, std::ptr::null(), &mut s) != VK_SUCCESS {
                    break 'run Err("vkCreateSampler 失败".into());
                }
                samplers.push(s);
            }
        }

        // ── ② SRV 纹理:device-local image + 全层 view + staging(逐层紧凑)上传源 ──
        for r in resources {
            let GraphicsResource::Texture2D {
                width: tw,
                height: th,
                data,
            } = r
            else {
                continue;
            };
            let levels = data.level_count();
            let extents = mip_level_extents(*tw, *th, levels);
            let (staging_total, staging_offsets) =
                mip_staging_layout(&extents, data.bytes_per_texel());
            let (image, mem) = match make_image(
                *tw,
                *th,
                levels,
                data.vk_format(),
                IMAGE_USAGE_SAMPLED | IMAGE_USAGE_TRANSFER_DST,
            ) {
                Ok(v) => v,
                Err(e) => break 'run Err(e),
            };
            // 先入列(建到一半失败时块外统一销毁已建部分)。
            texs.push(V2Tex {
                image,
                mem,
                view: VK_NULL_HANDLE,
                staging: VK_NULL_HANDLE,
                staging_mem: VK_NULL_HANDLE,
                levels,
            });
            let ti = texs.len() - 1;
            match make_view(image, data.vk_format(), levels) {
                Ok(v) => texs[ti].view = v,
                Err(e) => break 'run Err(e),
            }
            match make_host_buffer(BUFFER_USAGE_TRANSFER_SRC, staging_total) {
                Ok((b, m)) => {
                    texs[ti].staging = b;
                    texs[ti].staging_mem = m;
                }
                Err(e) => break 'run Err(e),
            }
            // 逐层数据写入 staging(host-visible+coherent,免 flush)。
            let mut ptr: *mut c_void = std::ptr::null_mut();
            if map_mem(device, texs[ti].staging_mem, 0, WHOLE_SIZE, 0, &mut ptr) != VK_SUCCESS {
                break 'run Err("纹理 staging vkMapMemory 失败".into());
            }
            for (lvl, off) in staging_offsets.iter().enumerate() {
                let dst = ptr.cast::<u8>().add(*off as usize);
                match data {
                    TextureData::Rgba8(v) => {
                        std::ptr::copy_nonoverlapping(v[lvl].as_ptr(), dst, v[lvl].len());
                    }
                    TextureData::RgbaF32(v) => {
                        std::ptr::copy_nonoverlapping(
                            v[lvl].as_ptr().cast::<u8>(),
                            dst,
                            v[lvl].len() * 4,
                        );
                    }
                }
            }
            unmap_mem(device, texs[ti].staging_mem);
        }

        // ── ③ storage images(UAV 轴;GENERAL layout 迁移在命令录制段)──
        for r in resources {
            let GraphicsResource::StorageImage {
                width: sw,
                height: sh,
                format,
            } = r
            else {
                continue;
            };
            let (image, mem) = match make_image(
                *sw,
                *sh,
                1,
                format.vk_format(),
                IMAGE_USAGE_STORAGE | IMAGE_USAGE_TRANSFER_SRC,
            ) {
                Ok(v) => v,
                Err(e) => break 'run Err(e),
            };
            storages.push(V2Storage {
                image,
                mem,
                view: VK_NULL_HANDLE,
            });
            let si = storages.len() - 1;
            match make_view(image, format.vk_format(), 1) {
                Ok(v) => storages[si].view = v,
                Err(e) => break 'run Err(e),
            }
        }

        // ── ④ set layouts(set-per-class;set0 CBV 空占位;immutable sampler 进 set3)──
        for set in 0..set_count {
            let mut bindings: Vec<DescriptorSetLayoutBinding> = Vec::new();
            for (r, &(s, b)) in resources.iter().zip(&plans) {
                if s != set {
                    continue;
                }
                let (dtype, stage_flags, p_imm) = match r {
                    GraphicsResource::Texture2D { .. } => (
                        DESCRIPTOR_TYPE_SAMPLED_IMAGE,
                        SHADER_STAGE_VERTEX | SHADER_STAGE_FRAGMENT,
                        std::ptr::null(),
                    ),
                    GraphicsResource::StorageImage { .. } => (
                        DESCRIPTOR_TYPE_STORAGE_IMAGE,
                        SHADER_STAGE_FRAGMENT,
                        std::ptr::null(),
                    ),
                    // binding = Sampler 类内声明序 = samplers 向量下标(同为声明序)。
                    GraphicsResource::Sampler(_) => (
                        DESCRIPTOR_TYPE_SAMPLER,
                        SHADER_STAGE_VERTEX | SHADER_STAGE_FRAGMENT,
                        (&samplers[b as usize]) as *const VkSampler as *const c_void,
                    ),
                };
                bindings.push(DescriptorSetLayoutBinding {
                    binding: b,
                    descriptor_type: dtype,
                    descriptor_count: 1,
                    stage_flags,
                    p_immutable_samplers: p_imm,
                });
            }
            let dslci = DescriptorSetLayoutCreateInfo {
                s_type: ST_DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                binding_count: bindings.len() as u32,
                p_bindings: if bindings.is_empty() {
                    std::ptr::null()
                } else {
                    bindings.as_ptr()
                },
            };
            let mut dsl: VkDescriptorSetLayout = VK_NULL_HANDLE;
            if create_dsl(device, &dslci, std::ptr::null(), &mut dsl) != VK_SUCCESS {
                break 'run Err("v2 vkCreateDescriptorSetLayout 失败".into());
            }
            set_layouts.push(dsl);
        }

        // ── ⑤ pipeline layout(v1 空 layout → 含 set layouts;v2 独立组装)──
        let plci = PipelineLayoutCreateInfo {
            s_type: ST_PIPELINE_LAYOUT_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            set_layout_count: set_count,
            p_set_layouts: set_layouts.as_ptr(),
            push_constant_range_count: 0,
            p_push_constant_ranges: std::ptr::null(),
        };
        if create_pl(device, &plci, std::ptr::null(), &mut pipe_layout) != VK_SUCCESS {
            break 'run Err("v2 vkCreatePipelineLayout 失败".into());
        }

        // ── ⑥ descriptor pool + sets + update ──
        let pool_sizes: Vec<DescriptorPoolSize> = pool_sizes_plan
            .iter()
            .map(|&(t, c)| DescriptorPoolSize {
                descriptor_type: t,
                descriptor_count: c,
            })
            .collect();
        let dpci = DescriptorPoolCreateInfo {
            s_type: ST_DESCRIPTOR_POOL_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            max_sets: set_count,
            pool_size_count: pool_sizes.len() as u32,
            p_pool_sizes: pool_sizes.as_ptr(),
        };
        if create_dpool(device, &dpci, std::ptr::null(), &mut desc_pool) != VK_SUCCESS {
            break 'run Err("v2 vkCreateDescriptorPool 失败".into());
        }
        let dsai = DescriptorSetAllocateInfo {
            s_type: ST_DESCRIPTOR_SET_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            descriptor_pool: desc_pool,
            descriptor_set_count: set_count,
            p_set_layouts: set_layouts.as_ptr(),
        };
        let mut dsets: Vec<VkDescriptorSet> = vec![VK_NULL_HANDLE; set_count as usize];
        if alloc_ds(device, &dsai, dsets.as_mut_ptr()) != VK_SUCCESS {
            break 'run Err("v2 vkAllocateDescriptorSets 失败".into());
        }
        // image infos 先建全(向量此后不增删,写引用其元素地址稳定)。
        let mut image_infos: Vec<DescriptorImageInfo> = Vec::new();
        {
            let mut tex_i = 0usize;
            let mut sto_i = 0usize;
            for r in resources {
                match r {
                    GraphicsResource::Texture2D { .. } => {
                        image_infos.push(DescriptorImageInfo {
                            sampler: VK_NULL_HANDLE,
                            image_view: texs[tex_i].view,
                            image_layout: IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
                        });
                        tex_i += 1;
                    }
                    GraphicsResource::StorageImage { .. } => {
                        image_infos.push(DescriptorImageInfo {
                            sampler: VK_NULL_HANDLE,
                            image_view: storages[sto_i].view,
                            image_layout: IMAGE_LAYOUT_GENERAL,
                        });
                        sto_i += 1;
                    }
                    GraphicsResource::Sampler(_) => {} // immutable → 无 write。
                }
            }
        }
        let mut writes: Vec<WriteDescriptorSet> = Vec::new();
        let mut info_i = 0usize;
        for (r, &(s, b)) in resources.iter().zip(&plans) {
            let dtype = match r {
                GraphicsResource::Texture2D { .. } => DESCRIPTOR_TYPE_SAMPLED_IMAGE,
                GraphicsResource::StorageImage { .. } => DESCRIPTOR_TYPE_STORAGE_IMAGE,
                GraphicsResource::Sampler(_) => continue,
            };
            writes.push(WriteDescriptorSet {
                s_type: ST_WRITE_DESCRIPTOR_SET,
                p_next: std::ptr::null(),
                dst_set: dsets[s as usize],
                dst_binding: b,
                dst_array_element: 0,
                descriptor_count: 1,
                descriptor_type: dtype,
                p_image_info: (&image_infos[info_i]) as *const DescriptorImageInfo as *const c_void,
                p_buffer_info: std::ptr::null(),
                p_texel_buffer_view: std::ptr::null(),
            });
            info_i += 1;
        }
        if !writes.is_empty() {
            update_ds(
                device,
                writes.len() as u32,
                writes.as_ptr(),
                0,
                std::ptr::null(),
            );
        }

        // ── ⑦ v1 镜像:color image / view / render pass / framebuffer ──
        let img_ci = ImageCreateInfo {
            s_type: ST_IMAGE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            image_type: IMAGE_TYPE_2D,
            format: FORMAT_R8G8B8A8_UNORM,
            extent: VkExtent3D {
                width,
                height,
                depth: 1,
            },
            mip_levels: 1,
            array_layers: 1,
            samples: SAMPLE_COUNT_1,
            tiling: IMAGE_TILING_OPTIMAL,
            usage: IMAGE_USAGE_COLOR_ATTACHMENT | IMAGE_USAGE_TRANSFER_SRC,
            sharing_mode: SHARING_MODE_EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
            initial_layout: IMAGE_LAYOUT_UNDEFINED,
        };
        if create_image(device, &img_ci, std::ptr::null(), &mut color_image) != VK_SUCCESS {
            break 'run Err("vkCreateImage 失败".into());
        }
        let mut ireq = std::mem::zeroed::<MemoryRequirements>();
        img_mem_req(device, color_image, &mut ireq);
        let Some(imt) = pick_mem_type(&memprops, ireq.memory_type_bits, MEM_DEVICE_LOCAL) else {
            break 'run Err("无 device-local 内存类型".into());
        };
        let mai = MemoryAllocateInfo {
            s_type: ST_MEMORY_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            allocation_size: ireq.size,
            memory_type_index: imt,
        };
        if alloc_mem(device, &mai, std::ptr::null(), &mut color_mem) != VK_SUCCESS {
            break 'run Err("color image vkAllocateMemory 失败".into());
        }
        bind_image(device, color_image, color_mem, 0);
        match make_view(color_image, FORMAT_R8G8B8A8_UNORM, 1) {
            Ok(v) => color_view = v,
            Err(e) => break 'run Err(e),
        }

        let att = AttachmentDescription {
            flags: 0,
            format: FORMAT_R8G8B8A8_UNORM,
            samples: SAMPLE_COUNT_1,
            load_op: ATTACHMENT_LOAD_OP_CLEAR,
            store_op: ATTACHMENT_STORE_OP_STORE,
            stencil_load_op: ATTACHMENT_LOAD_OP_DONT_CARE,
            stencil_store_op: ATTACHMENT_STORE_OP_DONT_CARE,
            initial_layout: IMAGE_LAYOUT_UNDEFINED,
            final_layout: IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
        };
        let att_ref = AttachmentReference {
            attachment: 0,
            layout: IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
        };
        let subpass = SubpassDescription {
            flags: 0,
            pipeline_bind_point: PIPELINE_BIND_POINT_GRAPHICS,
            input_attachment_count: 0,
            p_input_attachments: std::ptr::null(),
            color_attachment_count: 1,
            p_color_attachments: &att_ref,
            p_resolve_attachments: std::ptr::null(),
            p_depth_stencil_attachment: std::ptr::null(),
            preserve_attachment_count: 0,
            p_preserve_attachments: std::ptr::null(),
        };
        let rp_ci = RenderPassCreateInfo {
            s_type: ST_RENDER_PASS_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            attachment_count: 1,
            p_attachments: &att,
            subpass_count: 1,
            p_subpasses: &subpass,
            dependency_count: 0,
            p_dependencies: std::ptr::null(),
        };
        if create_rp(device, &rp_ci, std::ptr::null(), &mut render_pass) != VK_SUCCESS {
            break 'run Err("vkCreateRenderPass 失败".into());
        }
        let fb_ci = FramebufferCreateInfo {
            s_type: ST_FRAMEBUFFER_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            render_pass,
            attachment_count: 1,
            p_attachments: &color_view,
            width,
            height,
            layers: 1,
        };
        if create_fb(device, &fb_ci, std::ptr::null(), &mut framebuffer) != VK_SUCCESS {
            break 'run Err("vkCreateFramebuffer 失败".into());
        }

        // ── 顶点缓冲 + 回读缓冲 ──
        match make_host_buffer(BUFFER_USAGE_VERTEX, vertices.len().max(4) as u64) {
            Ok((b, m)) => {
                vbuf = b;
                vbuf_mem = m;
            }
            Err(e) => {
                break 'run Err(e);
            }
        }
        {
            let mut ptr: *mut c_void = std::ptr::null_mut();
            if map_mem(device, vbuf_mem, 0, WHOLE_SIZE, 0, &mut ptr) != VK_SUCCESS {
                break 'run Err("顶点缓冲 vkMapMemory 失败".into());
            }
            std::ptr::copy_nonoverlapping(vertices.as_ptr(), ptr.cast::<u8>(), vertices.len());
            unmap_mem(device, vbuf_mem);
        }
        match make_host_buffer(BUFFER_USAGE_TRANSFER_DST, readback_len as u64) {
            Ok((b, m)) => {
                rbuf = b;
                rbuf_mem = m;
            }
            Err(e) => {
                break 'run Err(e);
            }
        }
        // storage image 回读缓冲(host-visible TRANSFER_DST;§4.B8 模式⑤;仅在需回读时建)。
        if storage_readback_len > 0 {
            match make_host_buffer(BUFFER_USAGE_TRANSFER_DST, storage_readback_len as u64) {
                Ok((b, m)) => {
                    srbuf = b;
                    srbuf_mem = m;
                }
                Err(e) => {
                    break 'run Err(e);
                }
            }
        }

        // ── shader modules(pName 恒 "main")──
        let make_shader = |spv: &[u32]| -> Result<VkShaderModule, String> {
            let smci = ShaderModuleCreateInfo {
                s_type: ST_SHADER_MODULE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                code_size: spv.len() * 4,
                p_code: spv.as_ptr(),
            };
            let mut m: VkShaderModule = VK_NULL_HANDLE;
            if create_shader(device, &smci, std::ptr::null(), &mut m) != VK_SUCCESS {
                return Err("vkCreateShaderModule 失败(VUID-...-08742?)".into());
            }
            Ok(m)
        };
        match make_shader(vs_spv) {
            Ok(m) => vs_mod = m,
            Err(e) => {
                break 'run Err(format!("vertex {e}"));
            }
        }
        match make_shader(fs_spv) {
            Ok(m) => fs_mod = m,
            Err(e) => {
                break 'run Err(format!("fragment {e}"));
            }
        }

        // ── graphics pipeline(layout = v2 含 set layouts)──
        let stages = [
            PipelineShaderStageCreateInfo {
                s_type: ST_PIPELINE_SHADER_STAGE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                stage: SHADER_STAGE_VERTEX,
                module: vs_mod,
                p_name: c"main".as_ptr(),
                p_specialization_info: std::ptr::null(),
            },
            PipelineShaderStageCreateInfo {
                s_type: ST_PIPELINE_SHADER_STAGE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                stage: SHADER_STAGE_FRAGMENT,
                module: fs_mod,
                p_name: c"main".as_ptr(),
                p_specialization_info: std::ptr::null(),
            },
        ];
        let vbind = VkVertexInputBindingDescription {
            binding: 0,
            stride: vertex_stride,
            input_rate: VERTEX_INPUT_RATE_VERTEX,
        };
        let vattrs: Vec<VkVertexInputAttributeDescription> = attrs
            .iter()
            .map(
                |&(location, format, offset)| VkVertexInputAttributeDescription {
                    location,
                    binding: 0,
                    format,
                    offset,
                },
            )
            .collect();
        let vin = PipelineVertexInputStateCreateInfo {
            s_type: ST_PIPELINE_VERTEX_INPUT_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            vertex_binding_description_count: 1,
            p_vertex_binding_descriptions: &vbind,
            vertex_attribute_description_count: vattrs.len() as u32,
            p_vertex_attribute_descriptions: vattrs.as_ptr(),
        };
        let ia = PipelineInputAssemblyStateCreateInfo {
            s_type: ST_PIPELINE_INPUT_ASSEMBLY_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            topology: PRIMITIVE_TOPOLOGY_TRIANGLE_LIST,
            primitive_restart_enable: 0,
        };
        let viewport = VkViewport {
            x: 0.0,
            y: 0.0,
            width: width as f32,
            height: height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };
        let scissor = VkRect2D {
            offset: VkOffset2D { x: 0, y: 0 },
            extent: VkExtent2D { width, height },
        };
        let vp = PipelineViewportStateCreateInfo {
            s_type: ST_PIPELINE_VIEWPORT_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            viewport_count: 1,
            p_viewports: &viewport,
            scissor_count: 1,
            p_scissors: &scissor,
        };
        let rs = PipelineRasterizationStateCreateInfo {
            s_type: ST_PIPELINE_RASTERIZATION_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            depth_clamp_enable: 0,
            rasterizer_discard_enable: 0,
            polygon_mode: POLYGON_MODE_FILL,
            cull_mode: CULL_MODE_NONE,
            front_face: FRONT_FACE_COUNTER_CLOCKWISE,
            depth_bias_enable: 0,
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            line_width: 1.0,
        };
        let ms = PipelineMultisampleStateCreateInfo {
            s_type: ST_PIPELINE_MULTISAMPLE_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            rasterization_samples: SAMPLE_COUNT_1,
            sample_shading_enable: 0,
            min_sample_shading: 0.0,
            p_sample_mask: std::ptr::null(),
            alpha_to_coverage_enable: 0,
            alpha_to_one_enable: 0,
        };
        let blend_att = PipelineColorBlendAttachmentState {
            blend_enable: 0,
            src_color_blend_factor: 0,
            dst_color_blend_factor: 0,
            color_blend_op: 0,
            src_alpha_blend_factor: 0,
            dst_alpha_blend_factor: 0,
            alpha_blend_op: 0,
            color_write_mask: COLOR_COMPONENT_RGBA,
        };
        let cb = PipelineColorBlendStateCreateInfo {
            s_type: ST_PIPELINE_COLOR_BLEND_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            logic_op_enable: 0,
            logic_op: 0,
            attachment_count: 1,
            p_attachments: &blend_att,
            blend_constants: [0.0; 4],
        };
        let gpci = GraphicsPipelineCreateInfo {
            s_type: ST_GRAPHICS_PIPELINE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            stage_count: 2,
            p_stages: stages.as_ptr(),
            p_vertex_input_state: &vin,
            p_input_assembly_state: &ia,
            p_tessellation_state: std::ptr::null(),
            p_viewport_state: &vp,
            p_rasterization_state: &rs,
            p_multisample_state: &ms,
            p_depth_stencil_state: std::ptr::null(),
            p_color_blend_state: &cb,
            p_dynamic_state: std::ptr::null(),
            layout: pipe_layout,
            render_pass,
            subpass: 0,
            base_pipeline_handle: VK_NULL_HANDLE,
            base_pipeline_index: -1,
        };
        let r = create_gp(
            device,
            VK_NULL_HANDLE,
            1,
            &gpci,
            std::ptr::null(),
            &mut pipeline,
        );
        if r != VK_SUCCESS {
            break 'run Err(format!("vkCreateGraphicsPipelines 失败: {r}"));
        }

        // ── command pool + buffer + 录制 ──
        let cpci = CommandPoolCreateInfo {
            s_type: ST_COMMAND_POOL_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            queue_family_index: qfi,
        };
        if create_cmdpool(device, &cpci, std::ptr::null(), &mut cmdpool) != VK_SUCCESS {
            break 'run Err("vkCreateCommandPool 失败".into());
        }
        let cbai = CommandBufferAllocateInfo {
            s_type: ST_COMMAND_BUFFER_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            command_pool: cmdpool,
            level: CMD_BUFFER_LEVEL_PRIMARY,
            command_buffer_count: 1,
        };
        let mut cmd: VkCommandBuffer = std::ptr::null_mut();
        alloc_cmd(device, &cbai, &mut cmd);
        let cbbi = CommandBufferBeginInfo {
            s_type: ST_COMMAND_BUFFER_BEGIN_INFO,
            p_next: std::ptr::null(),
            flags: CMD_BUFFER_USAGE_ONE_TIME_SUBMIT,
            p_inheritance_info: std::ptr::null(),
        };
        begin_cmd(cmd, &cbbi);

        // ── 资源上传与 layout 迁移(render pass 前;RXS-0230 IR1)──
        // 纹理:UNDEFINED → TRANSFER_DST(全 mip)。
        for t in &texs {
            let to_dst = ImageMemoryBarrier {
                s_type: ST_IMAGE_MEMORY_BARRIER,
                p_next: std::ptr::null(),
                src_access_mask: 0,
                dst_access_mask: ACCESS_TRANSFER_WRITE,
                old_layout: IMAGE_LAYOUT_UNDEFINED,
                new_layout: IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                src_queue_family_index: QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: QUEUE_FAMILY_IGNORED,
                image: t.image,
                subresource_range: VkImageSubresourceRange {
                    aspect_mask: IMAGE_ASPECT_COLOR,
                    base_mip_level: 0,
                    level_count: t.levels,
                    base_array_layer: 0,
                    layer_count: 1,
                },
            };
            cmd_barrier(
                cmd,
                PIPELINE_STAGE_TOP_OF_PIPE,
                PIPELINE_STAGE_TRANSFER,
                0,
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                1,
                &to_dst,
            );
        }
        // 逐层 staging → image copy(偏移 = 纯函数 mip_staging_layout,与上传写入同源)。
        {
            let mut tex_i = 0usize;
            for r in resources {
                let GraphicsResource::Texture2D {
                    width: tw,
                    height: th,
                    data,
                } = r
                else {
                    continue;
                };
                let t = &texs[tex_i];
                tex_i += 1;
                let extents = mip_level_extents(*tw, *th, t.levels);
                let (_, offsets) = mip_staging_layout(&extents, data.bytes_per_texel());
                for (lvl, (&(w, h), &off)) in extents.iter().zip(offsets.iter()).enumerate() {
                    let region = VkBufferImageCopy {
                        buffer_offset: off,
                        buffer_row_length: 0,   // 紧凑(= 层宽)
                        buffer_image_height: 0, // 紧凑(= 层高)
                        image_subresource: VkImageSubresourceLayers {
                            aspect_mask: IMAGE_ASPECT_COLOR,
                            mip_level: lvl as u32,
                            base_array_layer: 0,
                            layer_count: 1,
                        },
                        image_offset: VkOffset3D { x: 0, y: 0, z: 0 },
                        image_extent: VkExtent3D {
                            width: w,
                            height: h,
                            depth: 1,
                        },
                    };
                    cmd_copy_buf_img(
                        cmd,
                        t.staging,
                        t.image,
                        IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                        1,
                        &region,
                    );
                }
            }
        }
        // 纹理:TRANSFER_DST → SHADER_READ_ONLY(全 mip;供 vertex/fragment 采样)。
        for t in &texs {
            let to_read = ImageMemoryBarrier {
                s_type: ST_IMAGE_MEMORY_BARRIER,
                p_next: std::ptr::null(),
                src_access_mask: ACCESS_TRANSFER_WRITE,
                dst_access_mask: ACCESS_SHADER_READ,
                old_layout: IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                new_layout: IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
                src_queue_family_index: QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: QUEUE_FAMILY_IGNORED,
                image: t.image,
                subresource_range: VkImageSubresourceRange {
                    aspect_mask: IMAGE_ASPECT_COLOR,
                    base_mip_level: 0,
                    level_count: t.levels,
                    base_array_layer: 0,
                    layer_count: 1,
                },
            };
            cmd_barrier(
                cmd,
                PIPELINE_STAGE_TRANSFER,
                PIPELINE_STAGE_VERTEX_SHADER | PIPELINE_STAGE_FRAGMENT_SHADER,
                0,
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                1,
                &to_read,
            );
        }
        // storage image:UNDEFINED → GENERAL(片元 load/store)。
        for s in &storages {
            let to_general = ImageMemoryBarrier {
                s_type: ST_IMAGE_MEMORY_BARRIER,
                p_next: std::ptr::null(),
                src_access_mask: 0,
                dst_access_mask: ACCESS_SHADER_READ | ACCESS_SHADER_WRITE,
                old_layout: IMAGE_LAYOUT_UNDEFINED,
                new_layout: IMAGE_LAYOUT_GENERAL,
                src_queue_family_index: QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: QUEUE_FAMILY_IGNORED,
                image: s.image,
                subresource_range: VkImageSubresourceRange {
                    aspect_mask: IMAGE_ASPECT_COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
            };
            cmd_barrier(
                cmd,
                PIPELINE_STAGE_TOP_OF_PIPE,
                PIPELINE_STAGE_FRAGMENT_SHADER,
                0,
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                1,
                &to_general,
            );
        }

        // ── render pass:clear → bind(pipeline + 全部 descriptor set)→ draw ──
        let clear_val = ClearValue { color: clear };
        let rpbi = RenderPassBeginInfo {
            s_type: ST_RENDER_PASS_BEGIN_INFO,
            p_next: std::ptr::null(),
            render_pass,
            framebuffer,
            render_area: VkRect2D {
                offset: VkOffset2D { x: 0, y: 0 },
                extent: VkExtent2D { width, height },
            },
            clear_value_count: 1,
            p_clear_values: &clear_val,
        };
        cmd_begin_rp(cmd, &rpbi, SUBPASS_CONTENTS_INLINE);
        cmd_bind_pipe(cmd, PIPELINE_BIND_POINT_GRAPHICS, pipeline);
        cmd_bind_ds(
            cmd,
            PIPELINE_BIND_POINT_GRAPHICS,
            pipe_layout,
            0,
            set_count,
            dsets.as_ptr(),
            0,
            std::ptr::null(),
        );
        let vbuf_offset: VkDeviceSize = 0;
        cmd_bind_vbuf(cmd, 0, 1, &vbuf, &vbuf_offset);
        let vertex_count = if vertex_stride > 0 {
            (vertices.len() / vertex_stride as usize) as u32
        } else {
            0
        };
        cmd_draw(cmd, vertex_count, 1, 0, 0);
        cmd_end_rp(cmd);

        // color-write → transfer-read 屏障 + 回读 copy(v1 镜像)。
        let barrier = ImageMemoryBarrier {
            s_type: ST_IMAGE_MEMORY_BARRIER,
            p_next: std::ptr::null(),
            src_access_mask: ACCESS_COLOR_ATTACHMENT_WRITE,
            dst_access_mask: ACCESS_TRANSFER_READ,
            old_layout: IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
            new_layout: IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
            src_queue_family_index: QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: QUEUE_FAMILY_IGNORED,
            image: color_image,
            subresource_range: VkImageSubresourceRange {
                aspect_mask: IMAGE_ASPECT_COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
        };
        cmd_barrier(
            cmd,
            PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT,
            PIPELINE_STAGE_TRANSFER,
            0,
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
            1,
            &barrier,
        );
        let region = VkBufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: VkImageSubresourceLayers {
                aspect_mask: IMAGE_ASPECT_COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_offset: VkOffset3D { x: 0, y: 0, z: 0 },
            image_extent: VkExtent3D {
                width,
                height,
                depth: 1,
            },
        };
        cmd_copy_img_buf(
            cmd,
            color_image,
            IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
            rbuf,
            1,
            &region,
        );

        // storage image 回读(§4.B8 模式⑤;仅 readback_storage && 存在 storage image):片元
        // store 写 → transfer 读内存/执行可见性屏障(pass 边界 barrier,RXS-0169 手动锚点,
        // GENERAL 保持)+ `vkCmdCopyImageToBuffer` 到 srbuf。唯一写者纪律(§4.B5)下无跨
        // invocation 冲突写 → 回读 well-defined。
        if storage_readback_len > 0
            && let Some((sw, sh, _bpt)) = storage_readback
            && let Some(s0) = storages.first()
        {
            let s_barrier = ImageMemoryBarrier {
                s_type: ST_IMAGE_MEMORY_BARRIER,
                p_next: std::ptr::null(),
                src_access_mask: ACCESS_SHADER_WRITE,
                dst_access_mask: ACCESS_TRANSFER_READ,
                old_layout: IMAGE_LAYOUT_GENERAL,
                new_layout: IMAGE_LAYOUT_GENERAL,
                src_queue_family_index: QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: QUEUE_FAMILY_IGNORED,
                image: s0.image,
                subresource_range: VkImageSubresourceRange {
                    aspect_mask: IMAGE_ASPECT_COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
            };
            cmd_barrier(
                cmd,
                PIPELINE_STAGE_FRAGMENT_SHADER,
                PIPELINE_STAGE_TRANSFER,
                0,
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                1,
                &s_barrier,
            );
            let s_region = VkBufferImageCopy {
                buffer_offset: 0,
                buffer_row_length: 0,
                buffer_image_height: 0,
                image_subresource: VkImageSubresourceLayers {
                    aspect_mask: IMAGE_ASPECT_COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                image_offset: VkOffset3D { x: 0, y: 0, z: 0 },
                image_extent: VkExtent3D {
                    width: sw,
                    height: sh,
                    depth: 1,
                },
            };
            cmd_copy_img_buf(cmd, s0.image, IMAGE_LAYOUT_GENERAL, srbuf, 1, &s_region);
        }
        end_cmd(cmd);

        // 提交 + 等待 + 回读。
        let si = SubmitInfo {
            s_type: ST_SUBMIT_INFO,
            p_next: std::ptr::null(),
            wait_semaphore_count: 0,
            p_wait_semaphores: std::ptr::null(),
            p_wait_dst_stage_mask: std::ptr::null(),
            command_buffer_count: 1,
            p_command_buffers: &cmd,
            signal_semaphore_count: 0,
            p_signal_semaphores: std::ptr::null(),
        };
        let r = queue_submit(queue, 1, &si, VK_NULL_HANDLE);
        if r != VK_SUCCESS {
            break 'run Err(format!("vkQueueSubmit 失败: {r}"));
        }
        queue_wait(queue);

        let mut ptr: *mut c_void = std::ptr::null_mut();
        if map_mem(device, rbuf_mem, 0, WHOLE_SIZE, 0, &mut ptr) != VK_SUCCESS {
            break 'run Err("回读 vkMapMemory 失败".into());
        }
        let mut pixels = vec![0u8; readback_len];
        std::ptr::copy_nonoverlapping(ptr.cast::<u8>(), pixels.as_mut_ptr(), readback_len);
        unmap_mem(device, rbuf_mem);

        // storage image 回读映射(§4.B8 模式⑤;srbuf 仅在需回读且存在 storage image 时非空)。
        let storage_pixels = if storage_readback_len > 0 && srbuf_mem != VK_NULL_HANDLE {
            let mut sptr: *mut c_void = std::ptr::null_mut();
            if map_mem(device, srbuf_mem, 0, WHOLE_SIZE, 0, &mut sptr) != VK_SUCCESS {
                break 'run Err("storage 回读 vkMapMemory 失败".into());
            }
            let mut sp = vec![0u8; storage_readback_len];
            std::ptr::copy_nonoverlapping(sptr.cast::<u8>(), sp.as_mut_ptr(), storage_readback_len);
            unmap_mem(device, srbuf_mem);
            Some(sp)
        } else {
            None
        };
        Ok((pixels, storage_pixels))
    };

    // ── 逆序销毁(非 null 者;pool 先于 setLayout,sampler 在 pool/layout 之后)──
    if cmdpool != VK_NULL_HANDLE {
        destroy_cmdpool(device, cmdpool, std::ptr::null());
    }
    if pipeline != VK_NULL_HANDLE {
        destroy_pipe(device, pipeline, std::ptr::null());
    }
    if pipe_layout != VK_NULL_HANDLE {
        destroy_pl(device, pipe_layout, std::ptr::null());
    }
    if desc_pool != VK_NULL_HANDLE {
        destroy_dpool(device, desc_pool, std::ptr::null());
    }
    for &dsl in &set_layouts {
        if dsl != VK_NULL_HANDLE {
            destroy_dsl(device, dsl, std::ptr::null());
        }
    }
    if fs_mod != VK_NULL_HANDLE {
        destroy_shader(device, fs_mod, std::ptr::null());
    }
    if vs_mod != VK_NULL_HANDLE {
        destroy_shader(device, vs_mod, std::ptr::null());
    }
    if rbuf != VK_NULL_HANDLE {
        destroy_buffer(device, rbuf, std::ptr::null());
    }
    if rbuf_mem != VK_NULL_HANDLE {
        free_mem(device, rbuf_mem, std::ptr::null());
    }
    if srbuf != VK_NULL_HANDLE {
        destroy_buffer(device, srbuf, std::ptr::null());
    }
    if srbuf_mem != VK_NULL_HANDLE {
        free_mem(device, srbuf_mem, std::ptr::null());
    }
    if vbuf != VK_NULL_HANDLE {
        destroy_buffer(device, vbuf, std::ptr::null());
    }
    if vbuf_mem != VK_NULL_HANDLE {
        free_mem(device, vbuf_mem, std::ptr::null());
    }
    for t in &texs {
        if t.view != VK_NULL_HANDLE {
            destroy_view(device, t.view, std::ptr::null());
        }
        if t.image != VK_NULL_HANDLE {
            destroy_image(device, t.image, std::ptr::null());
        }
        if t.mem != VK_NULL_HANDLE {
            free_mem(device, t.mem, std::ptr::null());
        }
        if t.staging != VK_NULL_HANDLE {
            destroy_buffer(device, t.staging, std::ptr::null());
        }
        if t.staging_mem != VK_NULL_HANDLE {
            free_mem(device, t.staging_mem, std::ptr::null());
        }
    }
    for s in &storages {
        if s.view != VK_NULL_HANDLE {
            destroy_view(device, s.view, std::ptr::null());
        }
        if s.image != VK_NULL_HANDLE {
            destroy_image(device, s.image, std::ptr::null());
        }
        if s.mem != VK_NULL_HANDLE {
            free_mem(device, s.mem, std::ptr::null());
        }
    }
    for &s in &samplers {
        if s != VK_NULL_HANDLE {
            destroy_sampler(device, s, std::ptr::null());
        }
    }
    if framebuffer != VK_NULL_HANDLE {
        destroy_fb(device, framebuffer, std::ptr::null());
    }
    if render_pass != VK_NULL_HANDLE {
        destroy_rp(device, render_pass, std::ptr::null());
    }
    if color_view != VK_NULL_HANDLE {
        destroy_view(device, color_view, std::ptr::null());
    }
    if color_image != VK_NULL_HANDLE {
        destroy_image(device, color_image, std::ptr::null());
    }
    if color_mem != VK_NULL_HANDLE {
        free_mem(device, color_mem, std::ptr::null());
    }

    result
}

// ───────── G3.4 bindless 无界表运行时(RXS-0235;RFC-0013 §4.C4,验收门 G-G3-4) ─────────
// 无界纹理表(`[Texture2D<F>]`,rurixc 侧 set4 独占,RXS-0233)的 Vulkan descriptor
// indexing 运行时:feature chain 四 bit 探测(任一缺失 → 确定性 Err,P-01)+
// `UPDATE_AFTER_BIND`/`PARTIALLY_BOUND` binding flags + 注册序写入(索引 = 注册序)+
// **pipeline layout push-constant range + `vkCmdPushConstants` 下发 table_len**——codegen
// 强制 clamp `UMin(idx, table_len - 1)` 的上界源(RXS-0208 尾槽/RXS-0234);缺此注册则
// shader 读到未初始化 push-constant,clamp 到垃圾上界(前轮负重点 a,故为硬接线)。
// 既有 v1/v2/present 路径 0-byte 不动,独立组装(v2 先例,§6.4/E-2)。

/// `VK_MAKE_API_VERSION(0, 1, 2, 0)`(bindless 路 descriptor indexing = **Vulkan 1.2
/// core**,承 RXS-0212 spirv-val vulkan1.2 校验环境;仅 bindless 入口用,v1/v2 维持 1.1)。
const API_VERSION_1_2: u32 = (1 << 22) | (2 << 12);

// descriptor indexing sType(VK_EXT_descriptor_indexing 扩展号 162 → 基 1000161000;
// Vulkan 1.2 core 收编,结构体编号不变)。
const ST_DESCRIPTOR_SET_LAYOUT_BINDING_FLAGS_CREATE_INFO: u32 = 1_000_161_000;
const ST_PHYSICAL_DEVICE_DESCRIPTOR_INDEXING_FEATURES: u32 = 1_000_161_001;
/// `VkPhysicalDeviceFeatures2`(VK_KHR_get_physical_device_properties2 扩展号 60;1.1 core)。
const ST_PHYSICAL_DEVICE_FEATURES_2: u32 = 1_000_059_000;

// `VkDescriptorBindingFlagBits`(RXS-0235:无界表 binding 双 flag)。
const DESCRIPTOR_BINDING_UPDATE_AFTER_BIND: u32 = 0x1;
const DESCRIPTOR_BINDING_PARTIALLY_BOUND: u32 = 0x4;
/// `VK_DESCRIPTOR_SET_LAYOUT_CREATE_UPDATE_AFTER_BIND_POOL_BIT`。
const DESCRIPTOR_SET_LAYOUT_CREATE_UPDATE_AFTER_BIND_POOL: u32 = 0x2;
/// `VK_DESCRIPTOR_POOL_CREATE_UPDATE_AFTER_BIND_BIT`。
const DESCRIPTOR_POOL_CREATE_UPDATE_AFTER_BIND: u32 = 0x2;

/// `VkPhysicalDeviceDescriptorIndexingFeatures` 的 VkBool32 字段数(spec 字段序顺排 20;
/// 定长数组建模逐字节对齐,按 `FEAT_*` 常量索引——镜像 [`PhysicalDeviceFeatures`] 纪律)。
const DESCRIPTOR_INDEXING_FEATURE_COUNT: usize = 20;
/// `shaderSampledImageArrayNonUniformIndexing` 字段序(0 基第 5 字段)。
const FEAT_SAMPLED_IMAGE_NON_UNIFORM: usize = 4;
/// `descriptorBindingSampledImageUpdateAfterBind`(0 基第 12 字段)。
const FEAT_SAMPLED_IMAGE_UPDATE_AFTER_BIND: usize = 11;
/// `descriptorBindingPartiallyBound`(0 基第 18 字段)。
const FEAT_PARTIALLY_BOUND: usize = 17;
/// `runtimeDescriptorArray`(0 基第 20 字段)。
const FEAT_RUNTIME_DESCRIPTOR_ARRAY: usize = 19;

/// 无界表独占 descriptor set(首表 = **set4**;镜像 rurixc `binding_layout.rs`
/// `VK_BINDLESS_SET_BASE` 分配律——类别轴 set0~3 之后首个空闲 set,RXS-0233;首期单表)。
/// 有界 set0~3 完全不触 feature chain(零回归最强形,RXS-0235)。
const BINDLESS_SET: u32 = 4;
/// push-constant 表长块字节数(`struct { u32 table_len; }` 成员 0 Offset 0,RXS-0208/0234)。
const TABLE_LEN_PC_BYTES: u32 = 4;

#[repr(C)]
struct PhysicalDeviceDescriptorIndexingFeatures {
    s_type: u32,
    p_next: *mut c_void,
    bits: [VkBool32; DESCRIPTOR_INDEXING_FEATURE_COUNT],
}

#[repr(C)]
struct PhysicalDeviceFeatures2 {
    s_type: u32,
    p_next: *mut c_void,
    features: PhysicalDeviceFeatures,
}

#[repr(C)]
struct DescriptorSetLayoutBindingFlagsCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    binding_count: u32,
    p_binding_flags: *const VkFlags,
}

/// `VkPhysicalDeviceProperties` 承载 blob:仅消费首字段 `apiVersion`(u32);驱动写入
/// 完整结构(x64 约 824 字节),blob 预留 2048 字节严格超集 + `align(8)`(结构含
/// VkDeviceSize limits)防越界/错位写。
#[repr(C, align(8))]
struct PhysicalDevicePropertiesBlob {
    api_version: u32,
    _rest: [u8; 2044],
}

type FnGetPhysicalDeviceFeatures2 =
    unsafe extern "system" fn(VkPhysicalDevice, *mut PhysicalDeviceFeatures2);
type FnGetPhysicalDeviceProperties =
    unsafe extern "system" fn(VkPhysicalDevice, *mut PhysicalDevicePropertiesBlob);
// `FnCmdPushConstants` 复用 compute 路既有定义(RXS-0206;bindless 表长下发同一签名)。

/// descriptor indexing 四 bit 探测结果(RXS-0235;host 纯数据——设备路自
/// `VkPhysicalDeviceDescriptorIndexingFeatures` 摘取;本机 RTX 4070 Ti 四 bit 全在,
/// missing 路由 mock 单测覆盖〔`bindless_feature_chain_*`〕,不伪造设备)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DescriptorIndexingBits {
    /// `shaderSampledImageArrayNonUniformIndexing`。
    pub sampled_image_non_uniform: bool,
    /// `descriptorBindingSampledImageUpdateAfterBind`。
    pub sampled_image_update_after_bind: bool,
    /// `descriptorBindingPartiallyBound`。
    pub partially_bound: bool,
    /// `runtimeDescriptorArray`。
    pub runtime_descriptor_array: bool,
}

impl DescriptorIndexingBits {
    /// 四 bit 全在(mock 单测基线)。
    pub fn all_present() -> Self {
        DescriptorIndexingBits {
            sampled_image_non_uniform: true,
            sampled_image_update_after_bind: true,
            partially_bound: true,
            runtime_descriptor_array: true,
        }
    }
}

/// 四 bit feature chain 判定(RXS-0235;host 纯函数):任一缺失 → 确定性 `Err` **全列
/// 缺失位名**(RXS-0193 封口不占 RX 码,无静默降级 P-01)。
//@ spec: RXS-0235
pub fn check_descriptor_indexing_bits(bits: &DescriptorIndexingBits) -> Result<(), String> {
    let mut missing: Vec<&str> = Vec::new();
    if !bits.sampled_image_non_uniform {
        missing.push("shaderSampledImageArrayNonUniformIndexing");
    }
    if !bits.sampled_image_update_after_bind {
        missing.push("descriptorBindingSampledImageUpdateAfterBind");
    }
    if !bits.partially_bound {
        missing.push("descriptorBindingPartiallyBound");
    }
    if !bits.runtime_descriptor_array {
        missing.push("runtimeDescriptorArray");
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "device 缺 descriptor indexing feature: {}(bindless 无界表不可用;确定性 Err,\
             RXS-0235/RXS-0193 口径,无静默降级)",
            missing.join(", ")
        ))
    }
}

/// bindless 入口纯 host 预校验(任何句柄创建前 fail-closed,P-01):表非空、表元素全为
/// SRV 纹理(首期 RXS-0233:无界 Sampler/CBV/UAV 维持 Unmappable)、有界资源首期仅
/// Texture2D/Sampler(storage 面归 v2)。
fn validate_bindless_inputs(
    resources: &[GraphicsResource],
    table: &[GraphicsResource],
) -> Result<(), String> {
    if table.is_empty() {
        return Err(
            "bindless 表须至少注册 1 纹理(空表无动态索引面;有界路走 run_graphics_offscreen_v2)"
                .into(),
        );
    }
    if table
        .iter()
        .any(|r| !matches!(r, GraphicsResource::Texture2D { .. }))
    {
        return Err(
            "bindless 表元素首期仅 SRV 纹理 Texture2D(无界 Sampler/CBV/UAV 维持 Unmappable,\
             RXS-0233)"
                .into(),
        );
    }
    if resources
        .iter()
        .any(|r| matches!(r, GraphicsResource::StorageImage { .. }))
    {
        return Err(
            "bindless 入口有界资源首期仅 Texture2D/Sampler(storage image 面走 \
             run_graphics_offscreen_v2)"
                .into(),
        );
    }
    Ok(())
}

/// offscreen 渲染一帧并回读——**bindless 无界表路**(RXS-0235;RFC-0013 §4.C4)。
/// `resources` = 有界 per-dispatch 资源(set0~3 类别轴,分配律同 v2);`table` = 无界表
/// 注册纹理(**切片序 = 注册序 = shader 动态索引值**,稳定单调;独占 set4 binding 0)。
///
/// 运行时纪律(RXS-0235):
/// - descriptor pool / set4 layout 带 `UPDATE_AFTER_BIND` + `PARTIALLY_BOUND` flags;
/// - `VkPhysicalDeviceFeatures2` + pNext `VkPhysicalDeviceDescriptorIndexingFeatures`
///   四 bit 探测,任一缺失(或 device < Vulkan 1.2)→ 确定性 `Err`(无静默降级,P-01);
/// - pipeline layout 声明 push-constant range(offset 0,4 字节,VS|FS)并在录制期
///   `vkCmdPushConstants` 下发 `table_len = table.len()`(codegen clamp 上界源,RXS-0234);
/// - 注册写入(set4 descriptor writes)仅发生在提交前,in-flight 期间不更新(§8)。
///
/// device 数值判据(四象限动态索引四色 / 篡改注册序换位 RED / feature 缺失 Err,步骤 64)
/// 归 `bin/bindless_modes` + owner 本机主循环;本入口交付编译绿 + host 单测 + 确定性 Err 路。
///
/// # SAFETY(U27 G3.4 bindless 扩注,graphics FFI 边界,0 新 U 号)
/// 对上全 safe(无 `unsafe` 签名)。内部契约同 v2 U27 扩注:句柄(table 纹理
/// image·mem·view·staging×N + v2 有界全集 + set4 layout)线性配对 create/destroy、末尾
/// 逆序销毁;资源合法性在任何句柄创建**前**纯 host 预校验;单 graphics queue 同步提交 +
/// `vkQueueWaitIdle` 后回读。
//@ spec: RXS-0235
#[allow(clippy::too_many_arguments)]
pub fn run_graphics_offscreen_bindless(
    vs_spv: &[u32],
    fs_spv: &[u32],
    vertices: &[u8],
    vertex_stride: u32,
    attrs: &[(u32, u32, u32)],
    width: u32,
    height: u32,
    clear: [f32; 4],
    resources: &[GraphicsResource],
    table: &[GraphicsResource],
) -> Result<Vec<u8>, String> {
    validate_resources(resources)?;
    validate_resources(table)?;
    validate_bindless_inputs(resources, table)?;
    let gipa = load_vulkan_loader().ok_or("vulkan loader (vulkan-1.dll/libvulkan.so) 不可用")?;
    // SAFETY: 见 U27 G3.4 bindless 扩注(上);句柄生命周期由内部函数线性管理,末尾逆序销毁。
    unsafe {
        run_graphics_inner_bindless(
            gipa,
            vs_spv,
            fs_spv,
            vertices,
            vertex_stride,
            attrs,
            width,
            height,
            clear,
            resources,
            table,
        )
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn run_graphics_inner_bindless(
    gipa: FnGetInstanceProcAddr,
    vs_spv: &[u32],
    fs_spv: &[u32],
    vertices: &[u8],
    vertex_stride: u32,
    attrs: &[(u32, u32, u32)],
    width: u32,
    height: u32,
    clear: [f32; 4],
    resources: &[GraphicsResource],
    table: &[GraphicsResource],
) -> Result<Vec<u8>, String> {
    let vk_create_instance: FnCreateInstance =
        cast_fn(gipa(std::ptr::null_mut(), c"vkCreateInstance".as_ptr()))
            .ok_or("缺 vkCreateInstance")?;

    let validation = std::env::var("RURIX_VK_VALIDATION").as_deref() == Ok("1");
    let layer_name = c"VK_LAYER_KHRONOS_validation";
    let layers: [*const c_char; 1] = [layer_name.as_ptr()];
    let debug_ext = c"VK_EXT_debug_utils";
    let exts: [*const c_char; 1] = [debug_ext.as_ptr()];
    let app = ApplicationInfo {
        s_type: ST_APPLICATION_INFO,
        p_next: std::ptr::null(),
        p_application_name: c"rurix-g3-bindless".as_ptr(),
        application_version: 0,
        p_engine_name: c"rurix".as_ptr(),
        engine_version: 0,
        // descriptor indexing = Vulkan 1.2 core(RXS-0212 spirv-val vulkan1.2 同环境)。
        api_version: API_VERSION_1_2,
    };
    let ici = InstanceCreateInfo {
        s_type: ST_INSTANCE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        p_application_info: &app,
        enabled_layer_count: if validation { 1 } else { 0 },
        pp_enabled_layer_names: if validation {
            layers.as_ptr()
        } else {
            std::ptr::null()
        },
        enabled_extension_count: if validation { 1 } else { 0 },
        pp_enabled_extension_names: if validation {
            exts.as_ptr()
        } else {
            std::ptr::null()
        },
    };
    let mut instance: VkInstance = std::ptr::null_mut();
    let r = vk_create_instance(&ici, std::ptr::null(), &mut instance);
    if r != VK_SUCCESS {
        return Err(format!("vkCreateInstance 失败: {r}"));
    }

    // instance 级符号(v2 全集 + features2/properties;messenger 创建前全部 `?` 完毕——
    // 镜像 v1/v2 U27「创建点与首个销毁点之间无 `?` 早退」契约)。
    let vk_destroy_instance: FnDestroyInstance =
        cast_fn(gipa(instance, c"vkDestroyInstance".as_ptr())).ok_or("缺 vkDestroyInstance")?;
    let vk_enum_pd: FnEnumeratePhysicalDevices =
        cast_fn(gipa(instance, c"vkEnumeratePhysicalDevices".as_ptr()))
            .ok_or("缺 vkEnumeratePhysicalDevices")?;
    let vk_get_qf: FnGetPhysicalDeviceQueueFamilyProperties = cast_fn(gipa(
        instance,
        c"vkGetPhysicalDeviceQueueFamilyProperties".as_ptr(),
    ))
    .ok_or("缺 vkGetPhysicalDeviceQueueFamilyProperties")?;
    let vk_get_mem: FnGetPhysicalDeviceMemoryProperties = cast_fn(gipa(
        instance,
        c"vkGetPhysicalDeviceMemoryProperties".as_ptr(),
    ))
    .ok_or("缺 vkGetPhysicalDeviceMemoryProperties")?;
    let vk_create_device: FnCreateDevice =
        cast_fn(gipa(instance, c"vkCreateDevice".as_ptr())).ok_or("缺 vkCreateDevice")?;
    let vk_get_device_proc: FnGetDeviceProcAddr =
        cast_fn(gipa(instance, c"vkGetDeviceProcAddr".as_ptr())).ok_or("缺 vkGetDeviceProcAddr")?;
    let vk_get_features: FnGetPhysicalDeviceFeatures =
        cast_fn(gipa(instance, c"vkGetPhysicalDeviceFeatures".as_ptr()))
            .ok_or("缺 vkGetPhysicalDeviceFeatures")?;
    // Vulkan 1.1 core(instance 已 1.2):feature chain 探测 + apiVersion 门。
    let vk_get_features2: FnGetPhysicalDeviceFeatures2 =
        cast_fn(gipa(instance, c"vkGetPhysicalDeviceFeatures2".as_ptr()))
            .ok_or("缺 vkGetPhysicalDeviceFeatures2(需 Vulkan 1.1+ loader)")?;
    let vk_get_props: FnGetPhysicalDeviceProperties =
        cast_fn(gipa(instance, c"vkGetPhysicalDeviceProperties".as_ptr()))
            .ok_or("缺 vkGetPhysicalDeviceProperties")?;

    // fail-closed messenger(L3,RXS-0210 契约复用;ERROR 级校验消息 → 末尾翻 `Err`)。
    let validation_error = std::sync::atomic::AtomicBool::new(false);
    let mut messenger: VkDebugUtilsMessengerEXT = VK_NULL_HANDLE;
    let destroy_messenger: Option<FnDestroyDebugUtilsMessengerEXT> = if validation {
        cast_fn(gipa(instance, c"vkDestroyDebugUtilsMessengerEXT".as_ptr()))
    } else {
        None
    };
    if validation
        && let Some(create_messenger) = cast_fn::<FnCreateDebugUtilsMessengerEXT>(gipa(
            instance,
            c"vkCreateDebugUtilsMessengerEXT".as_ptr(),
        ))
    {
        let dumci = DebugUtilsMessengerCreateInfoEXT {
            s_type: ST_DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
            p_next: std::ptr::null(),
            flags: 0,
            message_severity: DEBUG_UTILS_SEVERITY_ERROR,
            message_type: DEBUG_UTILS_TYPE_GENERAL
                | DEBUG_UTILS_TYPE_VALIDATION
                | DEBUG_UTILS_TYPE_PERFORMANCE,
            pfn_user_callback: debug_messenger_cb,
            p_user_data: &validation_error as *const std::sync::atomic::AtomicBool as *mut c_void,
        };
        let _ = create_messenger(instance, &dumci, std::ptr::null(), &mut messenger);
    }
    macro_rules! destroy_msgr {
        () => {
            if let Some(dm) = destroy_messenger {
                if messenger != VK_NULL_HANDLE {
                    dm(instance, messenger, std::ptr::null());
                }
            }
        };
    }

    let mut count = 0u32;
    vk_enum_pd(instance, &mut count, std::ptr::null_mut());
    if count == 0 {
        destroy_msgr!();
        vk_destroy_instance(instance, std::ptr::null());
        return Err("无 Vulkan 物理设备".into());
    }
    let mut pds = vec![std::ptr::null_mut::<c_void>(); count as usize];
    vk_enum_pd(instance, &mut count, pds.as_mut_ptr());
    let pd = pds[0];

    let mut qf_count = 0u32;
    vk_get_qf(pd, &mut qf_count, std::ptr::null_mut());
    let mut qfs: Vec<QueueFamilyProperties> = (0..qf_count)
        .map(|_| QueueFamilyProperties {
            queue_flags: 0,
            queue_count: 0,
            timestamp_valid_bits: 0,
            min_image_transfer_granularity: VkExtent3D {
                width: 0,
                height: 0,
                depth: 0,
            },
        })
        .collect();
    vk_get_qf(pd, &mut qf_count, qfs.as_mut_ptr());
    let qfi = match qfs
        .iter()
        .position(|q| q.queue_flags & QUEUE_GRAPHICS_BIT != 0)
    {
        Some(i) => i as u32,
        None => {
            destroy_msgr!();
            vk_destroy_instance(instance, std::ptr::null());
            return Err("无 graphics queue family".into());
        }
    };

    // ── device apiVersion 门(descriptor indexing = 1.2 core;< 1.2 → 确定性 Err)──
    let mut props = std::mem::zeroed::<PhysicalDevicePropertiesBlob>();
    vk_get_props(pd, &mut props);
    if props.api_version < API_VERSION_1_2 {
        destroy_msgr!();
        vk_destroy_instance(instance, std::ptr::null());
        return Err(format!(
            "device Vulkan apiVersion {:#x} < 1.2(descriptor indexing core 面;确定性 Err,\
             RXS-0235/RXS-0193 口径)",
            props.api_version
        ));
    }

    // ── feature chain 四 bit 探测(VkPhysicalDeviceFeatures2 + pNext indexing 链)──
    let mut probe_indexing = PhysicalDeviceDescriptorIndexingFeatures {
        s_type: ST_PHYSICAL_DEVICE_DESCRIPTOR_INDEXING_FEATURES,
        p_next: std::ptr::null_mut(),
        bits: [0; DESCRIPTOR_INDEXING_FEATURE_COUNT],
    };
    let mut probe = PhysicalDeviceFeatures2 {
        s_type: ST_PHYSICAL_DEVICE_FEATURES_2,
        p_next: (&mut probe_indexing as *mut PhysicalDeviceDescriptorIndexingFeatures)
            .cast::<c_void>(),
        features: PhysicalDeviceFeatures {
            features: [0; PHYSICAL_DEVICE_FEATURE_COUNT],
        },
    };
    vk_get_features2(pd, &mut probe);
    let bits = DescriptorIndexingBits {
        sampled_image_non_uniform: probe_indexing.bits[FEAT_SAMPLED_IMAGE_NON_UNIFORM] != 0,
        sampled_image_update_after_bind: probe_indexing.bits[FEAT_SAMPLED_IMAGE_UPDATE_AFTER_BIND]
            != 0,
        partially_bound: probe_indexing.bits[FEAT_PARTIALLY_BOUND] != 0,
        runtime_descriptor_array: probe_indexing.bits[FEAT_RUNTIME_DESCRIPTOR_ARRAY] != 0,
    };
    if let Err(e) = check_descriptor_indexing_bits(&bits) {
        destroy_msgr!();
        vk_destroy_instance(instance, std::ptr::null());
        return Err(e);
    }

    // ── 1.0 feature 协商(镜像 v2:aniso 探测;bindless 入口无 storage 面)──
    let needs_aniso = resources
        .iter()
        .any(|r| matches!(r, GraphicsResource::Sampler(d) if d.max_anisotropy > 1));
    let mut supported = PhysicalDeviceFeatures {
        features: [0; PHYSICAL_DEVICE_FEATURE_COUNT],
    };
    vk_get_features(pd, &mut supported);
    if needs_aniso && supported.features[FEATURE_SAMPLER_ANISOTROPY] == 0 {
        destroy_msgr!();
        vk_destroy_instance(instance, std::ptr::null());
        return Err(
            "sampler max_anisotropy>1 但设备缺 samplerAnisotropy feature(确定性 Err,RXS-0193 口径)"
                .into(),
        );
    }
    let mut enabled = PhysicalDeviceFeatures {
        features: [0; PHYSICAL_DEVICE_FEATURE_COUNT],
    };
    if needs_aniso {
        enabled.features[FEATURE_SAMPLER_ANISOTROPY] = 1;
    }
    // device 启用链:恰启用探测过的四 bit(不启用未消费面;pNext 直挂 indexing 链,
    // pEnabledFeatures 仍 1.0 结构——链中无 Features2 时两者并用合法)。
    let mut enabled_indexing = PhysicalDeviceDescriptorIndexingFeatures {
        s_type: ST_PHYSICAL_DEVICE_DESCRIPTOR_INDEXING_FEATURES,
        p_next: std::ptr::null_mut(),
        bits: [0; DESCRIPTOR_INDEXING_FEATURE_COUNT],
    };
    enabled_indexing.bits[FEAT_SAMPLED_IMAGE_NON_UNIFORM] = 1;
    enabled_indexing.bits[FEAT_SAMPLED_IMAGE_UPDATE_AFTER_BIND] = 1;
    enabled_indexing.bits[FEAT_PARTIALLY_BOUND] = 1;
    enabled_indexing.bits[FEAT_RUNTIME_DESCRIPTOR_ARRAY] = 1;

    let prio = [1.0f32];
    let dqci = DeviceQueueCreateInfo {
        s_type: ST_DEVICE_QUEUE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        queue_family_index: qfi,
        queue_count: 1,
        p_queue_priorities: prio.as_ptr(),
    };
    let dci = DeviceCreateInfo {
        s_type: ST_DEVICE_CREATE_INFO,
        p_next: (&enabled_indexing as *const PhysicalDeviceDescriptorIndexingFeatures)
            .cast::<c_void>(),
        flags: 0,
        queue_create_info_count: 1,
        p_queue_create_infos: &dqci,
        enabled_layer_count: 0,
        pp_enabled_layer_names: std::ptr::null(),
        enabled_extension_count: 0,
        pp_enabled_extension_names: std::ptr::null(),
        p_enabled_features: (&enabled as *const PhysicalDeviceFeatures).cast::<c_void>(),
    };
    let mut device: VkDevice = std::ptr::null_mut();
    let r = vk_create_device(pd, &dci, std::ptr::null(), &mut device);
    if r != VK_SUCCESS {
        destroy_msgr!();
        vk_destroy_instance(instance, std::ptr::null());
        return Err(format!("vkCreateDevice 失败: {r}"));
    }

    let mut out = graphics_body_bindless(
        vk_get_device_proc,
        device,
        pd,
        vk_get_mem,
        qfi,
        vs_spv,
        fs_spv,
        vertices,
        vertex_stride,
        attrs,
        width,
        height,
        clear,
        resources,
        table,
    );

    if validation && validation_error.load(std::sync::atomic::Ordering::Relaxed) {
        out =
            Err("VK_LAYER_KHRONOS_validation 报 ERROR 级校验错误(见 stderr;fail-closed,L3)".into());
    }

    let vk_destroy_device: Option<FnDestroyDevice> =
        cast_fn(vk_get_device_proc(device, c"vkDestroyDevice".as_ptr()));
    if let Some(dd) = vk_destroy_device {
        dd(device, std::ptr::null());
    }
    destroy_msgr!();
    vk_destroy_instance(instance, std::ptr::null());
    out
}

#[allow(clippy::too_many_arguments)]
unsafe fn graphics_body_bindless(
    gdpa: FnGetDeviceProcAddr,
    device: VkDevice,
    pd: VkPhysicalDevice,
    vk_get_mem: FnGetPhysicalDeviceMemoryProperties,
    qfi: u32,
    vs_spv: &[u32],
    fs_spv: &[u32],
    vertices: &[u8],
    vertex_stride: u32,
    attrs: &[(u32, u32, u32)],
    width: u32,
    height: u32,
    clear: [f32; 4],
    resources: &[GraphicsResource],
    table: &[GraphicsResource],
) -> Result<Vec<u8>, String> {
    macro_rules! dp {
        ($name:literal, $ty:ty) => {
            cast_fn::<$ty>(gdpa(device, $name.as_ptr())).ok_or("缺 device 符号")?
        };
    }
    // v2 符号全集(镜像 graphics_body_v2)+ bindless 新增 vkCmdPushConstants。
    let get_queue: FnGetDeviceQueue = dp!(c"vkGetDeviceQueue", FnGetDeviceQueue);
    let create_buffer: FnCreateBuffer = dp!(c"vkCreateBuffer", FnCreateBuffer);
    let destroy_buffer: FnDestroyBuffer = dp!(c"vkDestroyBuffer", FnDestroyBuffer);
    let buf_mem_req: FnGetBufferMemoryRequirements = dp!(
        c"vkGetBufferMemoryRequirements",
        FnGetBufferMemoryRequirements
    );
    let alloc_mem: FnAllocateMemory = dp!(c"vkAllocateMemory", FnAllocateMemory);
    let free_mem: FnFreeMemory = dp!(c"vkFreeMemory", FnFreeMemory);
    let bind_buf: FnBindBufferMemory = dp!(c"vkBindBufferMemory", FnBindBufferMemory);
    let map_mem: FnMapMemory = dp!(c"vkMapMemory", FnMapMemory);
    let unmap_mem: FnUnmapMemory = dp!(c"vkUnmapMemory", FnUnmapMemory);
    let create_shader: FnCreateShaderModule = dp!(c"vkCreateShaderModule", FnCreateShaderModule);
    let destroy_shader: FnDestroyShaderModule =
        dp!(c"vkDestroyShaderModule", FnDestroyShaderModule);
    let create_pl: FnCreatePipelineLayout = dp!(c"vkCreatePipelineLayout", FnCreatePipelineLayout);
    let destroy_pl: FnDestroyPipelineLayout =
        dp!(c"vkDestroyPipelineLayout", FnDestroyPipelineLayout);
    let destroy_pipe: FnDestroyPipeline = dp!(c"vkDestroyPipeline", FnDestroyPipeline);
    let create_cmdpool: FnCreateCommandPool = dp!(c"vkCreateCommandPool", FnCreateCommandPool);
    let destroy_cmdpool: FnDestroyCommandPool = dp!(c"vkDestroyCommandPool", FnDestroyCommandPool);
    let alloc_cmd: FnAllocateCommandBuffers =
        dp!(c"vkAllocateCommandBuffers", FnAllocateCommandBuffers);
    let begin_cmd: FnBeginCommandBuffer = dp!(c"vkBeginCommandBuffer", FnBeginCommandBuffer);
    let end_cmd: FnEndCommandBuffer = dp!(c"vkEndCommandBuffer", FnEndCommandBuffer);
    let cmd_bind_pipe: FnCmdBindPipeline = dp!(c"vkCmdBindPipeline", FnCmdBindPipeline);
    let queue_submit: FnQueueSubmit = dp!(c"vkQueueSubmit", FnQueueSubmit);
    let queue_wait: FnQueueWaitIdle = dp!(c"vkQueueWaitIdle", FnQueueWaitIdle);
    let create_image: FnCreateImage = dp!(c"vkCreateImage", FnCreateImage);
    let destroy_image: FnDestroyImage = dp!(c"vkDestroyImage", FnDestroyImage);
    let img_mem_req: FnGetImageMemoryRequirements = dp!(
        c"vkGetImageMemoryRequirements",
        FnGetImageMemoryRequirements
    );
    let bind_image: FnBindImageMemory = dp!(c"vkBindImageMemory", FnBindImageMemory);
    let create_view: FnCreateImageView = dp!(c"vkCreateImageView", FnCreateImageView);
    let destroy_view: FnDestroyImageView = dp!(c"vkDestroyImageView", FnDestroyImageView);
    let create_rp: FnCreateRenderPass = dp!(c"vkCreateRenderPass", FnCreateRenderPass);
    let destroy_rp: FnDestroyRenderPass = dp!(c"vkDestroyRenderPass", FnDestroyRenderPass);
    let create_fb: FnCreateFramebuffer = dp!(c"vkCreateFramebuffer", FnCreateFramebuffer);
    let destroy_fb: FnDestroyFramebuffer = dp!(c"vkDestroyFramebuffer", FnDestroyFramebuffer);
    let create_gp: FnCreateGraphicsPipelines =
        dp!(c"vkCreateGraphicsPipelines", FnCreateGraphicsPipelines);
    let cmd_begin_rp: FnCmdBeginRenderPass = dp!(c"vkCmdBeginRenderPass", FnCmdBeginRenderPass);
    let cmd_end_rp: FnCmdEndRenderPass = dp!(c"vkCmdEndRenderPass", FnCmdEndRenderPass);
    let cmd_bind_vbuf: FnCmdBindVertexBuffers =
        dp!(c"vkCmdBindVertexBuffers", FnCmdBindVertexBuffers);
    let cmd_draw: FnCmdDraw = dp!(c"vkCmdDraw", FnCmdDraw);
    let cmd_barrier: FnCmdPipelineBarrier = dp!(c"vkCmdPipelineBarrier", FnCmdPipelineBarrier);
    let cmd_copy_img_buf: FnCmdCopyImageToBuffer =
        dp!(c"vkCmdCopyImageToBuffer", FnCmdCopyImageToBuffer);
    let create_dsl: FnCreateDescriptorSetLayout =
        dp!(c"vkCreateDescriptorSetLayout", FnCreateDescriptorSetLayout);
    let destroy_dsl: FnDestroyDescriptorSetLayout = dp!(
        c"vkDestroyDescriptorSetLayout",
        FnDestroyDescriptorSetLayout
    );
    let create_dpool: FnCreateDescriptorPool =
        dp!(c"vkCreateDescriptorPool", FnCreateDescriptorPool);
    let destroy_dpool: FnDestroyDescriptorPool =
        dp!(c"vkDestroyDescriptorPool", FnDestroyDescriptorPool);
    let alloc_ds: FnAllocateDescriptorSets =
        dp!(c"vkAllocateDescriptorSets", FnAllocateDescriptorSets);
    let update_ds: FnUpdateDescriptorSets = dp!(c"vkUpdateDescriptorSets", FnUpdateDescriptorSets);
    let cmd_bind_ds: FnCmdBindDescriptorSets =
        dp!(c"vkCmdBindDescriptorSets", FnCmdBindDescriptorSets);
    let create_sampler: FnCreateSampler = dp!(c"vkCreateSampler", FnCreateSampler);
    let destroy_sampler: FnDestroySampler = dp!(c"vkDestroySampler", FnDestroySampler);
    let cmd_copy_buf_img: FnCmdCopyBufferToImage =
        dp!(c"vkCmdCopyBufferToImage", FnCmdCopyBufferToImage);
    // bindless 新增:push-constant 表长下发(负重点 a;RXS-0208/0234)。
    let cmd_push_constants: FnCmdPushConstants = dp!(c"vkCmdPushConstants", FnCmdPushConstants);

    let mut queue: VkQueue = std::ptr::null_mut();
    get_queue(device, qfi, 0, &mut queue);

    let mut memprops = std::mem::zeroed::<PhysicalDeviceMemoryProperties>();
    vk_get_mem(pd, &mut memprops);

    let readback_len = (width as usize) * (height as usize) * 4;
    let table_len = table.len() as u32;

    // 有界计划(纯函数;set0~3 类别轴,分配律同 v2/rurixc 单一镜像律);无界表恒 set4。
    let plans = plan_descriptor_sets(resources);
    let set_count = BINDLESS_SET + 1; // set0~3 有界(空 set 占位)+ set4 无界表。
    let mut pool_sizes_plan = plan_pool_sizes(resources);
    // 无界表占 SAMPLED_IMAGE 容量(注册计数;并入既有类目防重复条目)。
    if let Some(e) = pool_sizes_plan
        .iter_mut()
        .find(|(t, _)| *t == DESCRIPTOR_TYPE_SAMPLED_IMAGE)
    {
        e.1 += table_len;
    } else {
        pool_sizes_plan.insert(0, (DESCRIPTOR_TYPE_SAMPLED_IMAGE, table_len));
    }

    // host-visible buffer 建立 helper(镜像 v2)。
    let make_host_buffer = |usage: u32, size: u64| -> Result<(VkBuffer, VkDeviceMemory), String> {
        let bci = BufferCreateInfo {
            s_type: ST_BUFFER_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            size: size.max(4),
            usage,
            sharing_mode: SHARING_MODE_EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
        };
        let mut buffer: VkBuffer = VK_NULL_HANDLE;
        if create_buffer(device, &bci, std::ptr::null(), &mut buffer) != VK_SUCCESS {
            return Err("vkCreateBuffer 失败".into());
        }
        let mut req = std::mem::zeroed::<MemoryRequirements>();
        buf_mem_req(device, buffer, &mut req);
        let Some(mt) = pick_mem_type(
            &memprops,
            req.memory_type_bits,
            MEM_HOST_VISIBLE | MEM_HOST_COHERENT,
        ) else {
            destroy_buffer(device, buffer, std::ptr::null());
            return Err("无 host-visible+coherent 内存类型".into());
        };
        let mai = MemoryAllocateInfo {
            s_type: ST_MEMORY_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            allocation_size: req.size,
            memory_type_index: mt,
        };
        let mut mem: VkDeviceMemory = VK_NULL_HANDLE;
        if alloc_mem(device, &mai, std::ptr::null(), &mut mem) != VK_SUCCESS {
            destroy_buffer(device, buffer, std::ptr::null());
            return Err("vkAllocateMemory 失败".into());
        }
        bind_buf(device, buffer, mem, 0);
        Ok((buffer, mem))
    };

    // device-local image 建立 helper(镜像 v2)。
    let make_image = |w: u32,
                      h: u32,
                      mips: u32,
                      format: u32,
                      usage: u32|
     -> Result<(VkImage, VkDeviceMemory), String> {
        let ici = ImageCreateInfo {
            s_type: ST_IMAGE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            image_type: IMAGE_TYPE_2D,
            format,
            extent: VkExtent3D {
                width: w,
                height: h,
                depth: 1,
            },
            mip_levels: mips,
            array_layers: 1,
            samples: SAMPLE_COUNT_1,
            tiling: IMAGE_TILING_OPTIMAL,
            usage,
            sharing_mode: SHARING_MODE_EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
            initial_layout: IMAGE_LAYOUT_UNDEFINED,
        };
        let mut image: VkImage = VK_NULL_HANDLE;
        if create_image(device, &ici, std::ptr::null(), &mut image) != VK_SUCCESS {
            return Err("bindless vkCreateImage 失败".into());
        }
        let mut req = std::mem::zeroed::<MemoryRequirements>();
        img_mem_req(device, image, &mut req);
        let Some(mt) = pick_mem_type(&memprops, req.memory_type_bits, MEM_DEVICE_LOCAL) else {
            destroy_image(device, image, std::ptr::null());
            return Err("无 device-local 内存类型".into());
        };
        let mai = MemoryAllocateInfo {
            s_type: ST_MEMORY_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            allocation_size: req.size,
            memory_type_index: mt,
        };
        let mut mem: VkDeviceMemory = VK_NULL_HANDLE;
        if alloc_mem(device, &mai, std::ptr::null(), &mut mem) != VK_SUCCESS {
            destroy_image(device, image, std::ptr::null());
            return Err("bindless image vkAllocateMemory 失败".into());
        }
        bind_image(device, image, mem, 0);
        Ok((image, mem))
    };

    // image view 建立 helper(镜像 v2)。
    let make_view = |image: VkImage, format: u32, levels: u32| -> Result<VkImageView, String> {
        let vci = ImageViewCreateInfo {
            s_type: ST_IMAGE_VIEW_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            image,
            view_type: IMAGE_VIEW_TYPE_2D,
            format,
            components: VkComponentMapping {
                r: COMPONENT_SWIZZLE_IDENTITY,
                g: COMPONENT_SWIZZLE_IDENTITY,
                b: COMPONENT_SWIZZLE_IDENTITY,
                a: COMPONENT_SWIZZLE_IDENTITY,
            },
            subresource_range: VkImageSubresourceRange {
                aspect_mask: IMAGE_ASPECT_COLOR,
                base_mip_level: 0,
                level_count: levels,
                base_array_layer: 0,
                layer_count: 1,
            },
        };
        let mut view: VkImageView = VK_NULL_HANDLE;
        if create_view(device, &vci, std::ptr::null(), &mut view) != VK_SUCCESS {
            return Err("bindless vkCreateImageView 失败".into());
        }
        Ok(view)
    };

    // SRV 纹理建面 + staging 上传写入 helper(有界 texs / 无界 table_texs 共用同一纪律)。
    // 闭包捕获 device 符号;失败即 Err(块外统一销毁已建部分)。
    let build_tex =
        |dst: &mut Vec<V2Tex>, tw: u32, th: u32, data: &TextureData| -> Result<(), String> {
            let levels = data.level_count();
            let extents = mip_level_extents(tw, th, levels);
            let (staging_total, staging_offsets) =
                mip_staging_layout(&extents, data.bytes_per_texel());
            let (image, mem) = make_image(
                tw,
                th,
                levels,
                data.vk_format(),
                IMAGE_USAGE_SAMPLED | IMAGE_USAGE_TRANSFER_DST,
            )?;
            dst.push(V2Tex {
                image,
                mem,
                view: VK_NULL_HANDLE,
                staging: VK_NULL_HANDLE,
                staging_mem: VK_NULL_HANDLE,
                levels,
            });
            let ti = dst.len() - 1;
            dst[ti].view = make_view(image, data.vk_format(), levels)?;
            let (b, m) = make_host_buffer(BUFFER_USAGE_TRANSFER_SRC, staging_total)?;
            dst[ti].staging = b;
            dst[ti].staging_mem = m;
            let mut ptr: *mut c_void = std::ptr::null_mut();
            if map_mem(device, dst[ti].staging_mem, 0, WHOLE_SIZE, 0, &mut ptr) != VK_SUCCESS {
                return Err("bindless 纹理 staging vkMapMemory 失败".into());
            }
            for (lvl, off) in staging_offsets.iter().enumerate() {
                let p = ptr.cast::<u8>().add(*off as usize);
                match data {
                    TextureData::Rgba8(v) => {
                        std::ptr::copy_nonoverlapping(v[lvl].as_ptr(), p, v[lvl].len());
                    }
                    TextureData::RgbaF32(v) => {
                        std::ptr::copy_nonoverlapping(
                            v[lvl].as_ptr().cast::<u8>(),
                            p,
                            v[lvl].len() * 4,
                        );
                    }
                }
            }
            unmap_mem(device, dst[ti].staging_mem);
            Ok(())
        };

    // 句柄(全 null 初始,末尾逆序销毁非 null 者)。
    let mut samplers: Vec<VkSampler> = Vec::new();
    let mut texs: Vec<V2Tex> = Vec::new(); // 有界 SRV(set1)
    let mut table_texs: Vec<V2Tex> = Vec::new(); // 无界表(set4;下标 = 注册序 = 索引)
    let mut set_layouts: Vec<VkDescriptorSetLayout> = Vec::new();
    let mut desc_pool: VkDescriptorPool = VK_NULL_HANDLE;
    let mut color_image: VkImage = VK_NULL_HANDLE;
    let mut color_mem: VkDeviceMemory = VK_NULL_HANDLE;
    let mut color_view: VkImageView = VK_NULL_HANDLE;
    let mut render_pass: VkRenderPass = VK_NULL_HANDLE;
    let mut framebuffer: VkFramebuffer = VK_NULL_HANDLE;
    let mut vbuf: VkBuffer = VK_NULL_HANDLE;
    let mut vbuf_mem: VkDeviceMemory = VK_NULL_HANDLE;
    let mut rbuf: VkBuffer = VK_NULL_HANDLE;
    let mut rbuf_mem: VkDeviceMemory = VK_NULL_HANDLE;
    let mut vs_mod: VkShaderModule = VK_NULL_HANDLE;
    let mut fs_mod: VkShaderModule = VK_NULL_HANDLE;
    let mut pipe_layout: VkPipelineLayout = VK_NULL_HANDLE;
    let mut pipeline: VkPipeline = VK_NULL_HANDLE;
    let mut cmdpool: VkCommandPool = VK_NULL_HANDLE;

    let result: Result<Vec<u8>, String> = 'run: {
        // ── ① samplers(声明序 = Sampler 轴 binding 序;immutable 进 set3 layout)──
        for r in resources {
            if let GraphicsResource::Sampler(desc) = r {
                let sci = sampler_create_info(desc);
                let mut s: VkSampler = VK_NULL_HANDLE;
                if create_sampler(device, &sci, std::ptr::null(), &mut s) != VK_SUCCESS {
                    break 'run Err("vkCreateSampler 失败".into());
                }
                samplers.push(s);
            }
        }

        // ── ② 有界 SRV 纹理 + ③ 无界表纹理(注册序 = table 切片序 = 索引)──
        for r in resources {
            if let GraphicsResource::Texture2D {
                width: tw,
                height: th,
                data,
            } = r
                && let Err(e) = build_tex(&mut texs, *tw, *th, data)
            {
                break 'run Err(e);
            }
        }
        for r in table {
            let GraphicsResource::Texture2D {
                width: tw,
                height: th,
                data,
            } = r
            else {
                // validate_bindless_inputs 已拒;防御性兜底。
                break 'run Err("bindless 表元素须为 Texture2D".into());
            };
            if let Err(e) = build_tex(&mut table_texs, *tw, *th, data) {
                break 'run Err(e);
            }
        }

        // ── ④ set layouts:有界 set0~3(v2 同律,空 set 占位)+ set4 无界表 ──
        for set in 0..BINDLESS_SET {
            let mut bindings: Vec<DescriptorSetLayoutBinding> = Vec::new();
            for (r, &(s, b)) in resources.iter().zip(&plans) {
                if s != set {
                    continue;
                }
                let (dtype, stage_flags, p_imm) = match r {
                    GraphicsResource::Texture2D { .. } => (
                        DESCRIPTOR_TYPE_SAMPLED_IMAGE,
                        SHADER_STAGE_VERTEX | SHADER_STAGE_FRAGMENT,
                        std::ptr::null(),
                    ),
                    // bindless 入口无 storage 面(validate_bindless_inputs 已拒)。
                    GraphicsResource::StorageImage { .. } => {
                        break 'run Err("bindless 入口不承载 storage image".into());
                    }
                    GraphicsResource::Sampler(_) => (
                        DESCRIPTOR_TYPE_SAMPLER,
                        SHADER_STAGE_VERTEX | SHADER_STAGE_FRAGMENT,
                        (&samplers[b as usize]) as *const VkSampler as *const c_void,
                    ),
                };
                bindings.push(DescriptorSetLayoutBinding {
                    binding: b,
                    descriptor_type: dtype,
                    descriptor_count: 1,
                    stage_flags,
                    p_immutable_samplers: p_imm,
                });
            }
            let dslci = DescriptorSetLayoutCreateInfo {
                s_type: ST_DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                binding_count: bindings.len() as u32,
                p_bindings: if bindings.is_empty() {
                    std::ptr::null()
                } else {
                    bindings.as_ptr()
                },
            };
            let mut dsl: VkDescriptorSetLayout = VK_NULL_HANDLE;
            if create_dsl(device, &dslci, std::ptr::null(), &mut dsl) != VK_SUCCESS {
                break 'run Err("bindless 有界 vkCreateDescriptorSetLayout 失败".into());
            }
            set_layouts.push(dsl);
        }
        // set4 无界表 layout:binding 0 × table_len,UPDATE_AFTER_BIND + PARTIALLY_BOUND
        // binding flags + UPDATE_AFTER_BIND_POOL layout flag(RXS-0235)。
        {
            let binding_flags: VkFlags =
                DESCRIPTOR_BINDING_UPDATE_AFTER_BIND | DESCRIPTOR_BINDING_PARTIALLY_BOUND;
            let bfci = DescriptorSetLayoutBindingFlagsCreateInfo {
                s_type: ST_DESCRIPTOR_SET_LAYOUT_BINDING_FLAGS_CREATE_INFO,
                p_next: std::ptr::null(),
                binding_count: 1,
                p_binding_flags: &binding_flags,
            };
            let table_binding = DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: DESCRIPTOR_TYPE_SAMPLED_IMAGE,
                descriptor_count: table_len,
                stage_flags: SHADER_STAGE_VERTEX | SHADER_STAGE_FRAGMENT,
                p_immutable_samplers: std::ptr::null(),
            };
            let dslci = DescriptorSetLayoutCreateInfo {
                s_type: ST_DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
                p_next: (&bfci as *const DescriptorSetLayoutBindingFlagsCreateInfo)
                    .cast::<c_void>(),
                flags: DESCRIPTOR_SET_LAYOUT_CREATE_UPDATE_AFTER_BIND_POOL,
                binding_count: 1,
                p_bindings: &table_binding,
            };
            let mut dsl: VkDescriptorSetLayout = VK_NULL_HANDLE;
            if create_dsl(device, &dslci, std::ptr::null(), &mut dsl) != VK_SUCCESS {
                break 'run Err("bindless set4 vkCreateDescriptorSetLayout 失败".into());
            }
            set_layouts.push(dsl);
        }

        // ── ⑤ pipeline layout:set0~4 + push-constant range(table_len 下发面,负重点 a)──
        let pc_range = PushConstantRange {
            stage_flags: SHADER_STAGE_VERTEX | SHADER_STAGE_FRAGMENT,
            offset: 0,
            size: TABLE_LEN_PC_BYTES,
        };
        let plci = PipelineLayoutCreateInfo {
            s_type: ST_PIPELINE_LAYOUT_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            set_layout_count: set_count,
            p_set_layouts: set_layouts.as_ptr(),
            push_constant_range_count: 1,
            p_push_constant_ranges: &pc_range,
        };
        if create_pl(device, &plci, std::ptr::null(), &mut pipe_layout) != VK_SUCCESS {
            break 'run Err("bindless vkCreatePipelineLayout 失败".into());
        }

        // ── ⑥ descriptor pool(UPDATE_AFTER_BIND flag)+ sets + update ──
        let pool_sizes: Vec<DescriptorPoolSize> = pool_sizes_plan
            .iter()
            .map(|&(t, c)| DescriptorPoolSize {
                descriptor_type: t,
                descriptor_count: c,
            })
            .collect();
        let dpci = DescriptorPoolCreateInfo {
            s_type: ST_DESCRIPTOR_POOL_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: DESCRIPTOR_POOL_CREATE_UPDATE_AFTER_BIND,
            max_sets: set_count,
            pool_size_count: pool_sizes.len() as u32,
            p_pool_sizes: pool_sizes.as_ptr(),
        };
        if create_dpool(device, &dpci, std::ptr::null(), &mut desc_pool) != VK_SUCCESS {
            break 'run Err("bindless vkCreateDescriptorPool 失败".into());
        }
        let dsai = DescriptorSetAllocateInfo {
            s_type: ST_DESCRIPTOR_SET_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            descriptor_pool: desc_pool,
            descriptor_set_count: set_count,
            p_set_layouts: set_layouts.as_ptr(),
        };
        let mut dsets: Vec<VkDescriptorSet> = vec![VK_NULL_HANDLE; set_count as usize];
        if alloc_ds(device, &dsai, dsets.as_mut_ptr()) != VK_SUCCESS {
            break 'run Err("bindless vkAllocateDescriptorSets 失败".into());
        }
        // image infos 先建全(向量此后不增删,写引用其元素地址稳定):有界 SRV + 无界表。
        let mut image_infos: Vec<DescriptorImageInfo> = Vec::new();
        for t in &texs {
            image_infos.push(DescriptorImageInfo {
                sampler: VK_NULL_HANDLE,
                image_view: t.view,
                image_layout: IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
            });
        }
        let table_info_base = image_infos.len();
        for t in &table_texs {
            image_infos.push(DescriptorImageInfo {
                sampler: VK_NULL_HANDLE,
                image_view: t.view,
                image_layout: IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
            });
        }
        let mut writes: Vec<WriteDescriptorSet> = Vec::new();
        let mut info_i = 0usize;
        for (r, &(s, b)) in resources.iter().zip(&plans) {
            if !matches!(r, GraphicsResource::Texture2D { .. }) {
                continue; // sampler = immutable 无 write;storage 已拒。
            }
            writes.push(WriteDescriptorSet {
                s_type: ST_WRITE_DESCRIPTOR_SET,
                p_next: std::ptr::null(),
                dst_set: dsets[s as usize],
                dst_binding: b,
                dst_array_element: 0,
                descriptor_count: 1,
                descriptor_type: DESCRIPTOR_TYPE_SAMPLED_IMAGE,
                p_image_info: (&image_infos[info_i]) as *const DescriptorImageInfo as *const c_void,
                p_buffer_info: std::ptr::null(),
                p_texel_buffer_view: std::ptr::null(),
            });
            info_i += 1;
        }
        // 无界表写入:set4 binding0,dstArrayElement = 注册序 = 索引(RXS-0235;
        // 写入仅发生在提交前,in-flight 期间不更新)。
        for (i, _) in table_texs.iter().enumerate() {
            writes.push(WriteDescriptorSet {
                s_type: ST_WRITE_DESCRIPTOR_SET,
                p_next: std::ptr::null(),
                dst_set: dsets[BINDLESS_SET as usize],
                dst_binding: 0,
                dst_array_element: i as u32,
                descriptor_count: 1,
                descriptor_type: DESCRIPTOR_TYPE_SAMPLED_IMAGE,
                p_image_info: (&image_infos[table_info_base + i]) as *const DescriptorImageInfo
                    as *const c_void,
                p_buffer_info: std::ptr::null(),
                p_texel_buffer_view: std::ptr::null(),
            });
        }
        if !writes.is_empty() {
            update_ds(
                device,
                writes.len() as u32,
                writes.as_ptr(),
                0,
                std::ptr::null(),
            );
        }

        // ── ⑦ v2 镜像:color image / view / render pass / framebuffer ──
        let img_ci = ImageCreateInfo {
            s_type: ST_IMAGE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            image_type: IMAGE_TYPE_2D,
            format: FORMAT_R8G8B8A8_UNORM,
            extent: VkExtent3D {
                width,
                height,
                depth: 1,
            },
            mip_levels: 1,
            array_layers: 1,
            samples: SAMPLE_COUNT_1,
            tiling: IMAGE_TILING_OPTIMAL,
            usage: IMAGE_USAGE_COLOR_ATTACHMENT | IMAGE_USAGE_TRANSFER_SRC,
            sharing_mode: SHARING_MODE_EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
            initial_layout: IMAGE_LAYOUT_UNDEFINED,
        };
        if create_image(device, &img_ci, std::ptr::null(), &mut color_image) != VK_SUCCESS {
            break 'run Err("vkCreateImage 失败".into());
        }
        let mut ireq = std::mem::zeroed::<MemoryRequirements>();
        img_mem_req(device, color_image, &mut ireq);
        let Some(imt) = pick_mem_type(&memprops, ireq.memory_type_bits, MEM_DEVICE_LOCAL) else {
            break 'run Err("无 device-local 内存类型".into());
        };
        let mai = MemoryAllocateInfo {
            s_type: ST_MEMORY_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            allocation_size: ireq.size,
            memory_type_index: imt,
        };
        if alloc_mem(device, &mai, std::ptr::null(), &mut color_mem) != VK_SUCCESS {
            break 'run Err("color image vkAllocateMemory 失败".into());
        }
        bind_image(device, color_image, color_mem, 0);
        match make_view(color_image, FORMAT_R8G8B8A8_UNORM, 1) {
            Ok(v) => color_view = v,
            Err(e) => break 'run Err(e),
        }

        let att = AttachmentDescription {
            flags: 0,
            format: FORMAT_R8G8B8A8_UNORM,
            samples: SAMPLE_COUNT_1,
            load_op: ATTACHMENT_LOAD_OP_CLEAR,
            store_op: ATTACHMENT_STORE_OP_STORE,
            stencil_load_op: ATTACHMENT_LOAD_OP_DONT_CARE,
            stencil_store_op: ATTACHMENT_STORE_OP_DONT_CARE,
            initial_layout: IMAGE_LAYOUT_UNDEFINED,
            final_layout: IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
        };
        let att_ref = AttachmentReference {
            attachment: 0,
            layout: IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
        };
        let subpass = SubpassDescription {
            flags: 0,
            pipeline_bind_point: PIPELINE_BIND_POINT_GRAPHICS,
            input_attachment_count: 0,
            p_input_attachments: std::ptr::null(),
            color_attachment_count: 1,
            p_color_attachments: &att_ref,
            p_resolve_attachments: std::ptr::null(),
            p_depth_stencil_attachment: std::ptr::null(),
            preserve_attachment_count: 0,
            p_preserve_attachments: std::ptr::null(),
        };
        let rp_ci = RenderPassCreateInfo {
            s_type: ST_RENDER_PASS_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            attachment_count: 1,
            p_attachments: &att,
            subpass_count: 1,
            p_subpasses: &subpass,
            dependency_count: 0,
            p_dependencies: std::ptr::null(),
        };
        if create_rp(device, &rp_ci, std::ptr::null(), &mut render_pass) != VK_SUCCESS {
            break 'run Err("vkCreateRenderPass 失败".into());
        }
        let fb_ci = FramebufferCreateInfo {
            s_type: ST_FRAMEBUFFER_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            render_pass,
            attachment_count: 1,
            p_attachments: &color_view,
            width,
            height,
            layers: 1,
        };
        if create_fb(device, &fb_ci, std::ptr::null(), &mut framebuffer) != VK_SUCCESS {
            break 'run Err("vkCreateFramebuffer 失败".into());
        }

        // ── 顶点缓冲 + 回读缓冲 ──
        match make_host_buffer(BUFFER_USAGE_VERTEX, vertices.len().max(4) as u64) {
            Ok((b, m)) => {
                vbuf = b;
                vbuf_mem = m;
            }
            Err(e) => {
                break 'run Err(e);
            }
        }
        {
            let mut ptr: *mut c_void = std::ptr::null_mut();
            if map_mem(device, vbuf_mem, 0, WHOLE_SIZE, 0, &mut ptr) != VK_SUCCESS {
                break 'run Err("顶点缓冲 vkMapMemory 失败".into());
            }
            std::ptr::copy_nonoverlapping(vertices.as_ptr(), ptr.cast::<u8>(), vertices.len());
            unmap_mem(device, vbuf_mem);
        }
        match make_host_buffer(BUFFER_USAGE_TRANSFER_DST, readback_len as u64) {
            Ok((b, m)) => {
                rbuf = b;
                rbuf_mem = m;
            }
            Err(e) => {
                break 'run Err(e);
            }
        }

        // ── shader modules(pName 恒 "main")──
        let make_shader = |spv: &[u32]| -> Result<VkShaderModule, String> {
            let smci = ShaderModuleCreateInfo {
                s_type: ST_SHADER_MODULE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                code_size: spv.len() * 4,
                p_code: spv.as_ptr(),
            };
            let mut m: VkShaderModule = VK_NULL_HANDLE;
            if create_shader(device, &smci, std::ptr::null(), &mut m) != VK_SUCCESS {
                return Err("vkCreateShaderModule 失败(VUID-...-08742?)".into());
            }
            Ok(m)
        };
        match make_shader(vs_spv) {
            Ok(m) => vs_mod = m,
            Err(e) => {
                break 'run Err(format!("vertex {e}"));
            }
        }
        match make_shader(fs_spv) {
            Ok(m) => fs_mod = m,
            Err(e) => {
                break 'run Err(format!("fragment {e}"));
            }
        }

        // ── graphics pipeline(layout = set0~4 + push-constant range)──
        let stages = [
            PipelineShaderStageCreateInfo {
                s_type: ST_PIPELINE_SHADER_STAGE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                stage: SHADER_STAGE_VERTEX,
                module: vs_mod,
                p_name: c"main".as_ptr(),
                p_specialization_info: std::ptr::null(),
            },
            PipelineShaderStageCreateInfo {
                s_type: ST_PIPELINE_SHADER_STAGE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                stage: SHADER_STAGE_FRAGMENT,
                module: fs_mod,
                p_name: c"main".as_ptr(),
                p_specialization_info: std::ptr::null(),
            },
        ];
        let vbind = VkVertexInputBindingDescription {
            binding: 0,
            stride: vertex_stride,
            input_rate: VERTEX_INPUT_RATE_VERTEX,
        };
        let vattrs: Vec<VkVertexInputAttributeDescription> = attrs
            .iter()
            .map(
                |&(location, format, offset)| VkVertexInputAttributeDescription {
                    location,
                    binding: 0,
                    format,
                    offset,
                },
            )
            .collect();
        let vin = PipelineVertexInputStateCreateInfo {
            s_type: ST_PIPELINE_VERTEX_INPUT_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            vertex_binding_description_count: 1,
            p_vertex_binding_descriptions: &vbind,
            vertex_attribute_description_count: vattrs.len() as u32,
            p_vertex_attribute_descriptions: vattrs.as_ptr(),
        };
        let ia = PipelineInputAssemblyStateCreateInfo {
            s_type: ST_PIPELINE_INPUT_ASSEMBLY_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            topology: PRIMITIVE_TOPOLOGY_TRIANGLE_LIST,
            primitive_restart_enable: 0,
        };
        let viewport = VkViewport {
            x: 0.0,
            y: 0.0,
            width: width as f32,
            height: height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };
        let scissor = VkRect2D {
            offset: VkOffset2D { x: 0, y: 0 },
            extent: VkExtent2D { width, height },
        };
        let vp = PipelineViewportStateCreateInfo {
            s_type: ST_PIPELINE_VIEWPORT_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            viewport_count: 1,
            p_viewports: &viewport,
            scissor_count: 1,
            p_scissors: &scissor,
        };
        let rs = PipelineRasterizationStateCreateInfo {
            s_type: ST_PIPELINE_RASTERIZATION_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            depth_clamp_enable: 0,
            rasterizer_discard_enable: 0,
            polygon_mode: POLYGON_MODE_FILL,
            cull_mode: CULL_MODE_NONE,
            front_face: FRONT_FACE_COUNTER_CLOCKWISE,
            depth_bias_enable: 0,
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            line_width: 1.0,
        };
        let ms = PipelineMultisampleStateCreateInfo {
            s_type: ST_PIPELINE_MULTISAMPLE_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            rasterization_samples: SAMPLE_COUNT_1,
            sample_shading_enable: 0,
            min_sample_shading: 0.0,
            p_sample_mask: std::ptr::null(),
            alpha_to_coverage_enable: 0,
            alpha_to_one_enable: 0,
        };
        let blend_att = PipelineColorBlendAttachmentState {
            blend_enable: 0,
            src_color_blend_factor: 0,
            dst_color_blend_factor: 0,
            color_blend_op: 0,
            src_alpha_blend_factor: 0,
            dst_alpha_blend_factor: 0,
            alpha_blend_op: 0,
            color_write_mask: COLOR_COMPONENT_RGBA,
        };
        let cb = PipelineColorBlendStateCreateInfo {
            s_type: ST_PIPELINE_COLOR_BLEND_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            logic_op_enable: 0,
            logic_op: 0,
            attachment_count: 1,
            p_attachments: &blend_att,
            blend_constants: [0.0; 4],
        };
        let gpci = GraphicsPipelineCreateInfo {
            s_type: ST_GRAPHICS_PIPELINE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            stage_count: 2,
            p_stages: stages.as_ptr(),
            p_vertex_input_state: &vin,
            p_input_assembly_state: &ia,
            p_tessellation_state: std::ptr::null(),
            p_viewport_state: &vp,
            p_rasterization_state: &rs,
            p_multisample_state: &ms,
            p_depth_stencil_state: std::ptr::null(),
            p_color_blend_state: &cb,
            p_dynamic_state: std::ptr::null(),
            layout: pipe_layout,
            render_pass,
            subpass: 0,
            base_pipeline_handle: VK_NULL_HANDLE,
            base_pipeline_index: -1,
        };
        let r = create_gp(
            device,
            VK_NULL_HANDLE,
            1,
            &gpci,
            std::ptr::null(),
            &mut pipeline,
        );
        if r != VK_SUCCESS {
            break 'run Err(format!("vkCreateGraphicsPipelines 失败: {r}"));
        }

        // ── command pool + buffer + 录制 ──
        let cpci = CommandPoolCreateInfo {
            s_type: ST_COMMAND_POOL_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            queue_family_index: qfi,
        };
        if create_cmdpool(device, &cpci, std::ptr::null(), &mut cmdpool) != VK_SUCCESS {
            break 'run Err("vkCreateCommandPool 失败".into());
        }
        let cbai = CommandBufferAllocateInfo {
            s_type: ST_COMMAND_BUFFER_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            command_pool: cmdpool,
            level: CMD_BUFFER_LEVEL_PRIMARY,
            command_buffer_count: 1,
        };
        let mut cmd: VkCommandBuffer = std::ptr::null_mut();
        alloc_cmd(device, &cbai, &mut cmd);
        let cbbi = CommandBufferBeginInfo {
            s_type: ST_COMMAND_BUFFER_BEGIN_INFO,
            p_next: std::ptr::null(),
            flags: CMD_BUFFER_USAGE_ONE_TIME_SUBMIT,
            p_inheritance_info: std::ptr::null(),
        };
        begin_cmd(cmd, &cbbi);

        // ── 纹理上传与 layout 迁移(有界 + 无界表同一纪律;render pass 前)──
        for t in texs.iter().chain(table_texs.iter()) {
            let to_dst = ImageMemoryBarrier {
                s_type: ST_IMAGE_MEMORY_BARRIER,
                p_next: std::ptr::null(),
                src_access_mask: 0,
                dst_access_mask: ACCESS_TRANSFER_WRITE,
                old_layout: IMAGE_LAYOUT_UNDEFINED,
                new_layout: IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                src_queue_family_index: QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: QUEUE_FAMILY_IGNORED,
                image: t.image,
                subresource_range: VkImageSubresourceRange {
                    aspect_mask: IMAGE_ASPECT_COLOR,
                    base_mip_level: 0,
                    level_count: t.levels,
                    base_array_layer: 0,
                    layer_count: 1,
                },
            };
            cmd_barrier(
                cmd,
                PIPELINE_STAGE_TOP_OF_PIPE,
                PIPELINE_STAGE_TRANSFER,
                0,
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                1,
                &to_dst,
            );
        }
        // 逐层 staging → image copy(偏移与上传写入同源 mip_staging_layout)。
        {
            let tex_sources = resources
                .iter()
                .filter(|r| matches!(r, GraphicsResource::Texture2D { .. }))
                .chain(table.iter());
            for (r, t) in tex_sources.zip(texs.iter().chain(table_texs.iter())) {
                let GraphicsResource::Texture2D {
                    width: tw,
                    height: th,
                    data,
                } = r
                else {
                    continue;
                };
                let extents = mip_level_extents(*tw, *th, t.levels);
                let (_, offsets) = mip_staging_layout(&extents, data.bytes_per_texel());
                for (lvl, (&(w, h), &off)) in extents.iter().zip(offsets.iter()).enumerate() {
                    let region = VkBufferImageCopy {
                        buffer_offset: off,
                        buffer_row_length: 0,
                        buffer_image_height: 0,
                        image_subresource: VkImageSubresourceLayers {
                            aspect_mask: IMAGE_ASPECT_COLOR,
                            mip_level: lvl as u32,
                            base_array_layer: 0,
                            layer_count: 1,
                        },
                        image_offset: VkOffset3D { x: 0, y: 0, z: 0 },
                        image_extent: VkExtent3D {
                            width: w,
                            height: h,
                            depth: 1,
                        },
                    };
                    cmd_copy_buf_img(
                        cmd,
                        t.staging,
                        t.image,
                        IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                        1,
                        &region,
                    );
                }
            }
        }
        for t in texs.iter().chain(table_texs.iter()) {
            let to_read = ImageMemoryBarrier {
                s_type: ST_IMAGE_MEMORY_BARRIER,
                p_next: std::ptr::null(),
                src_access_mask: ACCESS_TRANSFER_WRITE,
                dst_access_mask: ACCESS_SHADER_READ,
                old_layout: IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                new_layout: IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
                src_queue_family_index: QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: QUEUE_FAMILY_IGNORED,
                image: t.image,
                subresource_range: VkImageSubresourceRange {
                    aspect_mask: IMAGE_ASPECT_COLOR,
                    base_mip_level: 0,
                    level_count: t.levels,
                    base_array_layer: 0,
                    layer_count: 1,
                },
            };
            cmd_barrier(
                cmd,
                PIPELINE_STAGE_TRANSFER,
                PIPELINE_STAGE_VERTEX_SHADER | PIPELINE_STAGE_FRAGMENT_SHADER,
                0,
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                1,
                &to_read,
            );
        }

        // ── render pass:clear → bind(pipeline + set0~4)→ push-constant → draw ──
        let clear_val = ClearValue { color: clear };
        let rpbi = RenderPassBeginInfo {
            s_type: ST_RENDER_PASS_BEGIN_INFO,
            p_next: std::ptr::null(),
            render_pass,
            framebuffer,
            render_area: VkRect2D {
                offset: VkOffset2D { x: 0, y: 0 },
                extent: VkExtent2D { width, height },
            },
            clear_value_count: 1,
            p_clear_values: &clear_val,
        };
        cmd_begin_rp(cmd, &rpbi, SUBPASS_CONTENTS_INLINE);
        cmd_bind_pipe(cmd, PIPELINE_BIND_POINT_GRAPHICS, pipeline);
        cmd_bind_ds(
            cmd,
            PIPELINE_BIND_POINT_GRAPHICS,
            pipe_layout,
            0,
            set_count,
            dsets.as_ptr(),
            0,
            std::ptr::null(),
        );
        // table_len 下发(负重点 a;RXS-0208 尾槽/RXS-0234):stageFlags 与 range 逐位一致,
        // 注册计数即 clamp 上界源——缺此调用则 shader 读未初始化 push-constant。
        cmd_push_constants(
            cmd,
            pipe_layout,
            SHADER_STAGE_VERTEX | SHADER_STAGE_FRAGMENT,
            0,
            TABLE_LEN_PC_BYTES,
            (&table_len as *const u32).cast::<c_void>(),
        );
        let vbuf_offset: VkDeviceSize = 0;
        cmd_bind_vbuf(cmd, 0, 1, &vbuf, &vbuf_offset);
        let vertex_count = if vertex_stride > 0 {
            (vertices.len() / vertex_stride as usize) as u32
        } else {
            0
        };
        cmd_draw(cmd, vertex_count, 1, 0, 0);
        cmd_end_rp(cmd);

        // color-write → transfer-read 屏障 + 回读 copy(v1/v2 镜像)。
        let barrier = ImageMemoryBarrier {
            s_type: ST_IMAGE_MEMORY_BARRIER,
            p_next: std::ptr::null(),
            src_access_mask: ACCESS_COLOR_ATTACHMENT_WRITE,
            dst_access_mask: ACCESS_TRANSFER_READ,
            old_layout: IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
            new_layout: IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
            src_queue_family_index: QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: QUEUE_FAMILY_IGNORED,
            image: color_image,
            subresource_range: VkImageSubresourceRange {
                aspect_mask: IMAGE_ASPECT_COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
        };
        cmd_barrier(
            cmd,
            PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT,
            PIPELINE_STAGE_TRANSFER,
            0,
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
            1,
            &barrier,
        );
        let region = VkBufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: VkImageSubresourceLayers {
                aspect_mask: IMAGE_ASPECT_COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_offset: VkOffset3D { x: 0, y: 0, z: 0 },
            image_extent: VkExtent3D {
                width,
                height,
                depth: 1,
            },
        };
        cmd_copy_img_buf(
            cmd,
            color_image,
            IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
            rbuf,
            1,
            &region,
        );
        end_cmd(cmd);

        // 提交 + 等待 + 回读。
        let si = SubmitInfo {
            s_type: ST_SUBMIT_INFO,
            p_next: std::ptr::null(),
            wait_semaphore_count: 0,
            p_wait_semaphores: std::ptr::null(),
            p_wait_dst_stage_mask: std::ptr::null(),
            command_buffer_count: 1,
            p_command_buffers: &cmd,
            signal_semaphore_count: 0,
            p_signal_semaphores: std::ptr::null(),
        };
        let r = queue_submit(queue, 1, &si, VK_NULL_HANDLE);
        if r != VK_SUCCESS {
            break 'run Err(format!("vkQueueSubmit 失败: {r}"));
        }
        queue_wait(queue);

        let mut ptr: *mut c_void = std::ptr::null_mut();
        if map_mem(device, rbuf_mem, 0, WHOLE_SIZE, 0, &mut ptr) != VK_SUCCESS {
            break 'run Err("回读 vkMapMemory 失败".into());
        }
        let mut pixels = vec![0u8; readback_len];
        std::ptr::copy_nonoverlapping(ptr.cast::<u8>(), pixels.as_mut_ptr(), readback_len);
        unmap_mem(device, rbuf_mem);
        Ok(pixels)
    };

    // ── 逆序销毁(非 null 者;pool 先于 setLayout,sampler 在 pool/layout 之后)──
    if cmdpool != VK_NULL_HANDLE {
        destroy_cmdpool(device, cmdpool, std::ptr::null());
    }
    if pipeline != VK_NULL_HANDLE {
        destroy_pipe(device, pipeline, std::ptr::null());
    }
    if pipe_layout != VK_NULL_HANDLE {
        destroy_pl(device, pipe_layout, std::ptr::null());
    }
    if desc_pool != VK_NULL_HANDLE {
        destroy_dpool(device, desc_pool, std::ptr::null());
    }
    for &dsl in &set_layouts {
        if dsl != VK_NULL_HANDLE {
            destroy_dsl(device, dsl, std::ptr::null());
        }
    }
    if fs_mod != VK_NULL_HANDLE {
        destroy_shader(device, fs_mod, std::ptr::null());
    }
    if vs_mod != VK_NULL_HANDLE {
        destroy_shader(device, vs_mod, std::ptr::null());
    }
    if rbuf != VK_NULL_HANDLE {
        destroy_buffer(device, rbuf, std::ptr::null());
    }
    if rbuf_mem != VK_NULL_HANDLE {
        free_mem(device, rbuf_mem, std::ptr::null());
    }
    if vbuf != VK_NULL_HANDLE {
        destroy_buffer(device, vbuf, std::ptr::null());
    }
    if vbuf_mem != VK_NULL_HANDLE {
        free_mem(device, vbuf_mem, std::ptr::null());
    }
    for t in texs.iter().chain(table_texs.iter()) {
        if t.view != VK_NULL_HANDLE {
            destroy_view(device, t.view, std::ptr::null());
        }
        if t.image != VK_NULL_HANDLE {
            destroy_image(device, t.image, std::ptr::null());
        }
        if t.mem != VK_NULL_HANDLE {
            free_mem(device, t.mem, std::ptr::null());
        }
        if t.staging != VK_NULL_HANDLE {
            destroy_buffer(device, t.staging, std::ptr::null());
        }
        if t.staging_mem != VK_NULL_HANDLE {
            free_mem(device, t.staging_mem, std::ptr::null());
        }
    }
    for &s in &samplers {
        if s != VK_NULL_HANDLE {
            destroy_sampler(device, s, std::ptr::null());
        }
    }
    if framebuffer != VK_NULL_HANDLE {
        destroy_fb(device, framebuffer, std::ptr::null());
    }
    if render_pass != VK_NULL_HANDLE {
        destroy_rp(device, render_pass, std::ptr::null());
    }
    if color_view != VK_NULL_HANDLE {
        destroy_view(device, color_view, std::ptr::null());
    }
    if color_image != VK_NULL_HANDLE {
        destroy_image(device, color_image, std::ptr::null());
    }
    if color_mem != VK_NULL_HANDLE {
        free_mem(device, color_mem, std::ptr::null());
    }

    result
}

// ───────────────────── win32 swapchain present(RXS-0210 L4,W6) ──────────────
// present 完成 RXS-0210 的 L4 present-defer(RD-032 的 code-deferral 部分):真 win32
// surface + swapchain 出图 + `vkQueuePresentKHR`,并**经 swapchain-image readback 数值校验**
// 反证 design graphics-present.md §2「present 无 headless 数值校验」的 defer 理由。仅
// `#[cfg(windows)]`(win32 surface Windows-only);Android surface present = 尾门 G-MB1-7,
// AMD 真卡 present 像素校验 = 尾门 G-MB1-6(均维持 open,本片不触)。复用 graphics offscreen
// 的 render pass / pipeline / readback 骨架 + `VK_EXT_debug_utils` messenger fail-closed。

// present 句柄(non-dispatchable = u64)。
type VkSurfaceKHR = u64;
type VkSwapchainKHR = u64;
type VkSemaphore = u64;
type VkBool32 = u32;

// present sType / enum。
const ST_SWAPCHAIN_CREATE_INFO_KHR: u32 = 1_000_001_000;
const ST_PRESENT_INFO_KHR: u32 = 1_000_001_001;
#[cfg(windows)] // win32 surface 专属(present_vk);android/其他平台不引入。
const ST_WIN32_SURFACE_CREATE_INFO_KHR: u32 = 1_000_009_000;
const ST_SEMAPHORE_CREATE_INFO: u32 = 9;
const IMAGE_LAYOUT_PRESENT_SRC_KHR: u32 = 1_000_001_002;
const PRESENT_MODE_FIFO_KHR: u32 = 2; // 唯一 spec 保证可用的 present mode。
const COLOR_SPACE_SRGB_NONLINEAR_KHR: u32 = 0;
const FORMAT_B8G8R8A8_UNORM: u32 = 44;
// composite alpha 位（`pick_composite_alpha` 派生用;win32 常 OPAQUE、Android surface 常
// 仅 INHERIT）。pre_transform 直接取 `caps.current_transform`（无需 IDENTITY 常量兜底——
// Vulkan 保证 currentTransform 恒为受支持变换,可直接用作 preTransform）。
const COMPOSITE_ALPHA_OPAQUE_BIT_KHR: u32 = 0x1;
const COMPOSITE_ALPHA_PRE_MULTIPLIED_BIT_KHR: u32 = 0x2;
const COMPOSITE_ALPHA_POST_MULTIPLIED_BIT_KHR: u32 = 0x4;
const COMPOSITE_ALPHA_INHERIT_BIT_KHR: u32 = 0x8;
const SUBOPTIMAL_KHR: VkResult = 1_000_001_003;
// RXS-0221(RFC-0013 §4.A2 Vulkan 收尾):swapchain 失效 = 正常路径非错误。
const ERROR_OUT_OF_DATE_KHR: VkResult = -1_000_001_004;
const SUBPASS_EXTERNAL: u32 = u32::MAX;
const PIPELINE_STAGE_BOTTOM_OF_PIPE: u32 = 0x2000;

#[cfg(windows)] // win32 surface 专属(present_vk);android surface 用 android_present 模块。
#[repr(C)]
struct Win32SurfaceCreateInfoKHR {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    hinstance: *mut c_void,
    hwnd: *mut c_void,
}

#[repr(C)]
struct SurfaceFormatKHR {
    format: u32,
    color_space: u32,
}

#[repr(C)]
struct SurfaceCapabilitiesKHR {
    min_image_count: u32,
    max_image_count: u32,
    current_extent: VkExtent2D,
    min_image_extent: VkExtent2D,
    max_image_extent: VkExtent2D,
    max_image_array_layers: u32,
    supported_transforms: VkFlags,
    current_transform: VkFlags,
    supported_composite_alpha: VkFlags,
    supported_usage_flags: VkFlags,
}

#[repr(C)]
struct SwapchainCreateInfoKHR {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    surface: VkSurfaceKHR,
    min_image_count: u32,
    image_format: u32,
    image_color_space: u32,
    image_extent: VkExtent2D,
    image_array_layers: u32,
    image_usage: VkFlags,
    image_sharing_mode: u32,
    queue_family_index_count: u32,
    p_queue_family_indices: *const u32,
    pre_transform: VkFlags,
    composite_alpha: VkFlags,
    present_mode: u32,
    clipped: VkBool32,
    old_swapchain: VkSwapchainKHR,
}

#[repr(C)]
struct PresentInfoKHR {
    s_type: u32,
    p_next: *const c_void,
    wait_semaphore_count: u32,
    p_wait_semaphores: *const VkSemaphore,
    swapchain_count: u32,
    p_swapchains: *const VkSwapchainKHR,
    p_image_indices: *const u32,
    p_results: *mut VkResult,
}

#[repr(C)]
struct SemaphoreCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
}

#[repr(C)]
struct SubpassDependency {
    src_subpass: u32,
    dst_subpass: u32,
    src_stage_mask: VkFlags,
    dst_stage_mask: VkFlags,
    src_access_mask: VkFlags,
    dst_access_mask: VkFlags,
    dependency_flags: VkFlags,
}

// present 函数指针(surface/swapchain/semaphore;经 instance/device proc 解析)。
#[cfg(windows)] // win32 surface 专属(present_vk);android surface FFI 见 android_present。
type FnCreateWin32SurfaceKHR = unsafe extern "system" fn(
    VkInstance,
    *const Win32SurfaceCreateInfoKHR,
    *const c_void,
    *mut VkSurfaceKHR,
) -> VkResult;
type FnGetPhysicalDeviceSurfaceSupportKHR =
    unsafe extern "system" fn(VkPhysicalDevice, u32, VkSurfaceKHR, *mut VkBool32) -> VkResult;
type FnGetPhysicalDeviceSurfaceCapabilitiesKHR = unsafe extern "system" fn(
    VkPhysicalDevice,
    VkSurfaceKHR,
    *mut SurfaceCapabilitiesKHR,
) -> VkResult;
type FnGetPhysicalDeviceSurfaceFormatsKHR = unsafe extern "system" fn(
    VkPhysicalDevice,
    VkSurfaceKHR,
    *mut u32,
    *mut SurfaceFormatKHR,
) -> VkResult;
type FnGetPhysicalDeviceSurfacePresentModesKHR =
    unsafe extern "system" fn(VkPhysicalDevice, VkSurfaceKHR, *mut u32, *mut u32) -> VkResult;
type FnDestroySurfaceKHR = unsafe extern "system" fn(VkInstance, VkSurfaceKHR, *const c_void);
type FnCreateSwapchainKHR = unsafe extern "system" fn(
    VkDevice,
    *const SwapchainCreateInfoKHR,
    *const c_void,
    *mut VkSwapchainKHR,
) -> VkResult;
type FnDestroySwapchainKHR = unsafe extern "system" fn(VkDevice, VkSwapchainKHR, *const c_void);
type FnGetSwapchainImagesKHR =
    unsafe extern "system" fn(VkDevice, VkSwapchainKHR, *mut u32, *mut VkImage) -> VkResult;
type FnAcquireNextImageKHR = unsafe extern "system" fn(
    VkDevice,
    VkSwapchainKHR,
    u64,
    VkSemaphore,
    u64,
    *mut u32,
) -> VkResult;
type FnQueuePresentKHR = unsafe extern "system" fn(VkQueue, *const PresentInfoKHR) -> VkResult;
type FnCreateSemaphore = unsafe extern "system" fn(
    VkDevice,
    *const SemaphoreCreateInfo,
    *const c_void,
    *mut VkSemaphore,
) -> VkResult;
type FnDestroySemaphore = unsafe extern "system" fn(VkDevice, VkSemaphore, *const c_void);

// ── present 纯 host helper(无设备,单测锚定 RXS-0210) ────────────────────────

/// swapchain extent 协商:`current_extent.width == u32::MAX` 表示 surface 允许自选 → 把
/// 请求尺寸 clamp 进 `[min, max]`;否则**必须**用 `current_extent`(Windows 上 surface 固定
/// 为窗口客户区,`imageExtent != currentExtent` 触 VUID)。返回 `(w, h)`。
fn choose_present_extent(
    current: (u32, u32),
    req_w: u32,
    req_h: u32,
    min: (u32, u32),
    max: (u32, u32),
) -> (u32, u32) {
    if current.0 != u32::MAX {
        return current;
    }
    (req_w.clamp(min.0, max.0), req_h.clamp(min.1, max.1))
}

/// surface format 选择:优先 `B8G8R8A8_UNORM` / `R8G8B8A8_UNORM` + `SRGB_NONLINEAR` color
/// space;否则退回首个可用(Vulkan 保证 `count ≥ 1`)。返回 `(format, color_space)`。
/// 注:readback 逐字节按所选 8-bit-per-channel 布局取(RGBA vs BGRA 仅影响通道序,像素断言
/// 「背景黑 / 中心非背景 / covered」对通道序不敏感)。
fn pick_surface_format(formats: &[(u32, u32)]) -> (u32, u32) {
    for &(fmt, cs) in formats {
        if (fmt == FORMAT_B8G8R8A8_UNORM || fmt == FORMAT_R8G8B8A8_UNORM)
            && cs == COLOR_SPACE_SRGB_NONLINEAR_KHR
        {
            return (fmt, cs);
        }
    }
    formats
        .first()
        .copied()
        .unwrap_or((FORMAT_B8G8R8A8_UNORM, COLOR_SPACE_SRGB_NONLINEAR_KHR))
}

/// swapchain 最小图像数:`min + 1`(免 acquire 阻塞),`max_count > 0` 时 clamp 进 max。
fn choose_min_image_count(min_count: u32, max_count: u32) -> u32 {
    let desired = min_count + 1;
    if max_count > 0 && desired > max_count {
        max_count
    } else {
        desired
    }
}

/// RXS-0221:`vkAcquireNextImageKHR`/`vkQueuePresentKHR` 返回码 → swapchain 重建协商动作
/// (RFC-0013 §4.A2:swapchain 失效是**正常路径**非错误)。纯 host 确定性三分类,可单测:
/// - `VK_SUCCESS` → `Present`(正常呈现);
/// - `VK_ERROR_OUT_OF_DATE_KHR` / `SUBOPTIMAL_KHR` → `Rebuild`(等 GPU idle → 重建 swapchain/
///   imageView/framebuffer,重查 surface caps extent;`SUBOPTIMAL` 首期与 `OUT_OF_DATE` 同走
///   保守重建,窄化收益登 RD-034+);
/// - 其余非预期码 → `Fatal`(终止,不重建)。
///
/// 调用侧语境(acquire vs present)另行区分:acquire 返回 `OUT_OF_DATE` 时该帧未取到 image →
/// 重建后重试不推进帧;acquire 返回 `SUBOPTIMAL` 时 image 已取到 → 本帧照常渲染、present 后再
/// 重建(避免信号量泄漏)。本纯函数只做码 → 动作映射,不含 FFI。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapchainAction {
    /// 正常呈现,不重建。
    Present,
    /// swapchain 失效 → 重建(idle → 释放 → 重查 caps extent 重建 → 重建后首帧再校验)。
    Rebuild,
    /// 非预期返回码 → 终止。
    Fatal,
}

/// RXS-0221 重建协商纯函数(host 可单测;见 [`SwapchainAction`])。
pub fn swapchain_present_action(result: VkResult) -> SwapchainAction {
    match result {
        VK_SUCCESS => SwapchainAction::Present,
        SUBOPTIMAL_KHR | ERROR_OUT_OF_DATE_KHR => SwapchainAction::Rebuild,
        _ => SwapchainAction::Fatal,
    }
}

/// 从 surface 支持的 composite alpha 位集择一(host 可测纯函数)。优先级
/// OPAQUE → INHERIT → PRE_MULTIPLIED → POST_MULTIPLIED,均不支持则退回最低置位。
/// win32 surface 常报 OPAQUE(数值零回归,swapchain 与旧硬编码等价);Android surface 常
/// **不支持 OPAQUE**、只报 INHERIT(0x8),硬编码 OPAQUE 会触 VUID → 必须查询派生。
fn pick_composite_alpha(supported: u32) -> u32 {
    for bit in [
        COMPOSITE_ALPHA_OPAQUE_BIT_KHR,
        COMPOSITE_ALPHA_INHERIT_BIT_KHR,
        COMPOSITE_ALPHA_PRE_MULTIPLIED_BIT_KHR,
        COMPOSITE_ALPHA_POST_MULTIPLIED_BIT_KHR,
    ] {
        if supported & bit != 0 {
            return bit;
        }
    }
    // 回退:最低置位(Vulkan 保证 supportedCompositeAlpha 至少 1 位置位;0 时仍给 OPAQUE 兜底)。
    if supported == 0 {
        COMPOSITE_ALPHA_OPAQUE_BIT_KHR
    } else {
        supported & supported.wrapping_neg()
    }
}

// ── win32 窗口 FFI(仅 #[cfg(windows)];user32/kernel32 由 std 常态链接) ──────
#[cfg(windows)]
mod win32 {
    use core::ffi::c_void;

    pub type Hwnd = *mut c_void;
    pub type Hinstance = *mut c_void;
    pub type Wparam = usize;
    pub type Lparam = isize;
    pub type Lresult = isize;
    pub type WndProc = unsafe extern "system" fn(Hwnd, u32, Wparam, Lparam) -> Lresult;

    pub const WS_POPUP: u32 = 0x8000_0000;
    pub const PM_REMOVE: u32 = 0x0001;

    #[repr(C)]
    pub struct WndClassW {
        pub style: u32,
        pub lpfn_wnd_proc: Option<WndProc>,
        pub cb_cls_extra: i32,
        pub cb_wnd_extra: i32,
        pub h_instance: Hinstance,
        pub h_icon: *mut c_void,
        pub h_cursor: *mut c_void,
        pub hbr_background: *mut c_void,
        pub lpsz_menu_name: *const u16,
        pub lpsz_class_name: *const u16,
    }

    #[repr(C)]
    pub struct Msg {
        pub hwnd: Hwnd,
        pub message: u32,
        pub w_param: Wparam,
        pub l_param: Lparam,
        pub time: u32,
        pub pt_x: i32,
        pub pt_y: i32,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        pub fn GetModuleHandleW(module_name: *const u16) -> Hinstance;
    }

    // 窗口 / 消息 API 在 user32.dll(std 不常态链接,须显式 #[link])。
    #[link(name = "user32")]
    unsafe extern "system" {
        pub fn RegisterClassW(wc: *const WndClassW) -> u16;
        pub fn UnregisterClassW(class_name: *const u16, instance: Hinstance) -> i32;
        #[allow(clippy::too_many_arguments)]
        pub fn CreateWindowExW(
            ex_style: u32,
            class_name: *const u16,
            window_name: *const u16,
            style: u32,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            parent: Hwnd,
            menu: *mut c_void,
            instance: Hinstance,
            param: *mut c_void,
        ) -> Hwnd;
        pub fn DestroyWindow(hwnd: Hwnd) -> i32;
        pub fn DefWindowProcW(hwnd: Hwnd, msg: u32, w: Wparam, l: Lparam) -> Lresult;
        pub fn PeekMessageW(
            msg: *mut Msg,
            hwnd: Hwnd,
            filter_min: u32,
            filter_max: u32,
            remove: u32,
        ) -> i32;
        pub fn TranslateMessage(msg: *const Msg) -> i32;
        pub fn DispatchMessageW(msg: *const Msg) -> Lresult;
    }

    /// 隐藏窗口的窗口过程:一律委派 `DefWindowProcW`(不出图、不交互)。
    /// # Safety
    /// 由 win32 消息泵按 WNDPROC 契约调用;`DefWindowProcW` 对任意消息安全。
    pub unsafe extern "system" fn wnd_proc(hwnd: Hwnd, msg: u32, w: Wparam, l: Lparam) -> Lresult {
        DefWindowProcW(hwnd, msg, w, l)
    }

    /// UTF-16 NUL 结尾宽串(win32 W-API 入参)。
    pub fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(core::iter::once(0)).collect()
    }
}

/// win32 swapchain present:创建隐藏 win32 窗口 + `VkSurfaceKHR`(`VK_KHR_win32_surface`)+
/// `VkSwapchainKHR`(`VK_KHR_swapchain`),渲染 `frames` 帧居中三角形到 swapchain image →
/// **`vkCmdCopyImageToBuffer` 回读像素**(反证 present 可数值校验)→ 转 `PRESENT_SRC_KHR` →
/// `vkQueuePresentKHR`。返回**最后一帧**的紧凑 RGBA8 回读(所选 swapchain format 的 8-bit
/// 通道布局;像素断言对通道序不敏感)。
///
/// 每帧 `vkAcquireNextImageKHR`(imageAvailable 信号)→ 录制(render→barrier→copy→转
/// PRESENT_SRC)→ `vkQueueSubmit`(wait imageAvailable @ COLOR_ATTACHMENT_OUTPUT,signal
/// renderFinished)→ `vkQueuePresentKHR`(wait renderFinished)→ `vkQueueWaitIdle`(令
/// 两 binary semaphore 逐帧复用安全)。present 返回值须逐帧 `VK_SUCCESS`/`SUBOPTIMAL_KHR`。
///
/// 缺 Vulkan 驱动 / 无 present-capable graphics queue / surface 建失败 → 确定性 `Err`
/// (非 panic,fail-closed,P-01);`RURIX_VK_VALIDATION=1` 开 `VK_EXT_debug_utils` messenger,
/// ERROR 级校验消息翻 `Err`(退出码判红)。**Android surface present = 尾门 G-MB1-7,AMD 真卡
/// present 像素校验 = 尾门 G-MB1-6**(均 RD-032 open,本函数不触)。
///
/// # SAFETY(U27 扩注,present FFI 边界)
/// 本公共入口对上全 safe。内部全程手写 Vulkan + win32 FFI:win32 窗口(`RegisterClassW` +
/// `CreateWindowExW` WS_POPUP 隐藏 + `DestroyWindow`/`UnregisterClassW` 逆序拆除)+
/// `VkSurfaceKHR`/`VkSwapchainKHR`/`VkSemaphore`×2 句柄线性配对 create/destroy(逆序销毁;
/// swapchain image 归 swapchain 所有,**只销毁 imageView/framebuffer/swapchain,不 destroy
/// swapchain image**);每个 present `#[repr(C)]` VkStruct 与 Vulkan spec 逐字节对齐(由
/// `VK_LAYER_KHRONOS_validation` 真跑零报错实证);单 graphics queue 同步(`vkQueueWaitIdle`)
/// 后回读(无数据竞争)。gate feature `vulkan` 默认关闭,CUDA 路零回归。
#[cfg(windows)]
#[allow(clippy::too_many_arguments)]
pub fn run_graphics_present(
    vs_spv: &[u32],
    fs_spv: &[u32],
    vertices: &[u8],
    vertex_stride: u32,
    attrs: &[(u32, u32, u32)],
    width: u32,
    height: u32,
    clear: [f32; 4],
    frames: u32,
) -> Result<Vec<u8>, String> {
    let gipa = load_vulkan_loader().ok_or("vulkan loader (vulkan-1.dll) 不可用")?;
    // SAFETY: 见 U27 present 扩注(上);窗口/句柄生命周期由内部线性管理,末尾逆序拆除。
    unsafe {
        run_graphics_present_inner(
            gipa,
            vs_spv,
            fs_spv,
            vertices,
            vertex_stride,
            attrs,
            width,
            height,
            clear,
            frames.max(1),
        )
    }
}

/// 非 Windows:win32 surface 不可用。Android present 走 `android_present` 模块的
/// `vkCreateAndroidSurfaceKHR`,on-device 出图循环 = 尾门 G-MB1-7(无 android runner)。
#[cfg(not(windows))]
#[allow(clippy::too_many_arguments)]
pub fn run_graphics_present(
    _vs_spv: &[u32],
    _fs_spv: &[u32],
    _vertices: &[u8],
    _vertex_stride: u32,
    _attrs: &[(u32, u32, u32)],
    _width: u32,
    _height: u32,
    _clear: [f32; 4],
    _frames: u32,
) -> Result<Vec<u8>, String> {
    Err("win32 present: windows-only (android present = G-MB1-7 尾门)".into())
}

#[cfg(windows)]
#[allow(clippy::too_many_arguments)]
unsafe fn run_graphics_present_inner(
    gipa: FnGetInstanceProcAddr,
    vs_spv: &[u32],
    fs_spv: &[u32],
    vertices: &[u8],
    vertex_stride: u32,
    attrs: &[(u32, u32, u32)],
    width: u32,
    height: u32,
    clear: [f32; 4],
    frames: u32,
) -> Result<Vec<u8>, String> {
    // ── 隐藏 win32 窗口(WS_POPUP,不 ShowWindow;客户区 == 请求尺寸)──
    let hinstance = win32::GetModuleHandleW(std::ptr::null());
    // class 名唯一化(pid + 单调计数)避免残留 class 冲突(ERROR_CLASS_ALREADY_EXISTS)。
    static PRESENT_WND_SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let seq = PRESENT_WND_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let class_name = win32::to_wide(&format!("RurixVkPresent_{}_{}", std::process::id(), seq));
    let window_name = win32::to_wide("rurix-vk-present");
    let wc = win32::WndClassW {
        style: 0,
        lpfn_wnd_proc: Some(win32::wnd_proc),
        cb_cls_extra: 0,
        cb_wnd_extra: 0,
        h_instance: hinstance,
        h_icon: std::ptr::null_mut(),
        h_cursor: std::ptr::null_mut(),
        hbr_background: std::ptr::null_mut(),
        lpsz_menu_name: std::ptr::null(),
        lpsz_class_name: class_name.as_ptr(),
    };
    if win32::RegisterClassW(&wc) == 0 {
        return Err("win32 RegisterClassW 失败".into());
    }
    let hwnd = win32::CreateWindowExW(
        0,
        class_name.as_ptr(),
        window_name.as_ptr(),
        win32::WS_POPUP, // 隐藏(无 WS_VISIBLE);headless present。
        0,
        0,
        width as i32,
        height as i32,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        hinstance,
        std::ptr::null_mut(),
    );
    if hwnd.is_null() {
        win32::UnregisterClassW(class_name.as_ptr(), hinstance);
        return Err("win32 CreateWindowExW 失败".into());
    }
    // 泵一次消息(处理 WM_CREATE 等,避免窗口挂起态)。
    pump_messages(hwnd);

    // ── vk present(窗口拆除保证在其后,无论 Ok/Err)──
    let result = present_vk(
        gipa,
        hinstance,
        hwnd,
        vs_spv,
        fs_spv,
        vertices,
        vertex_stride,
        attrs,
        width,
        height,
        clear,
        frames,
    );

    win32::DestroyWindow(hwnd);
    win32::UnregisterClassW(class_name.as_ptr(), hinstance);
    result
}

/// 非阻塞消息泵(PM_REMOVE 排空隐藏窗口消息队列)。
#[cfg(windows)]
unsafe fn pump_messages(hwnd: win32::Hwnd) {
    let mut msg = std::mem::zeroed::<win32::Msg>();
    while win32::PeekMessageW(&mut msg, hwnd, 0, 0, win32::PM_REMOVE) != 0 {
        win32::TranslateMessage(&msg);
        win32::DispatchMessageW(&msg);
    }
}

#[cfg(windows)]
#[allow(clippy::too_many_arguments)]
unsafe fn present_vk(
    gipa: FnGetInstanceProcAddr,
    hinstance: win32::Hinstance,
    hwnd: win32::Hwnd,
    vs_spv: &[u32],
    fs_spv: &[u32],
    vertices: &[u8],
    vertex_stride: u32,
    attrs: &[(u32, u32, u32)],
    width: u32,
    height: u32,
    clear: [f32; 4],
    frames: u32,
) -> Result<Vec<u8>, String> {
    let vk_create_instance: FnCreateInstance =
        cast_fn(gipa(std::ptr::null_mut(), c"vkCreateInstance".as_ptr()))
            .ok_or("缺 vkCreateInstance")?;

    let validation = std::env::var("RURIX_VK_VALIDATION").as_deref() == Ok("1");
    let layer_name = c"VK_LAYER_KHRONOS_validation";
    let layers: [*const c_char; 1] = [layer_name.as_ptr()];
    // instance 扩展:present 恒需 surface + win32_surface;validation 追加 debug_utils。
    let mut exts: Vec<*const c_char> =
        vec![c"VK_KHR_surface".as_ptr(), c"VK_KHR_win32_surface".as_ptr()];
    if validation {
        exts.push(c"VK_EXT_debug_utils".as_ptr());
    }
    let app = ApplicationInfo {
        s_type: ST_APPLICATION_INFO,
        p_next: std::ptr::null(),
        p_application_name: c"rurix-mb1".as_ptr(),
        application_version: 0,
        p_engine_name: c"rurix".as_ptr(),
        engine_version: 0,
        api_version: API_VERSION_1_1,
    };
    let ici = InstanceCreateInfo {
        s_type: ST_INSTANCE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        p_application_info: &app,
        enabled_layer_count: if validation { 1 } else { 0 },
        pp_enabled_layer_names: if validation {
            layers.as_ptr()
        } else {
            std::ptr::null()
        },
        enabled_extension_count: exts.len() as u32,
        pp_enabled_extension_names: exts.as_ptr(),
    };
    let mut instance: VkInstance = std::ptr::null_mut();
    let r = vk_create_instance(&ici, std::ptr::null(), &mut instance);
    if r != VK_SUCCESS {
        return Err(format!("vkCreateInstance(present) 失败: {r}"));
    }

    // instance 级符号。
    let vk_destroy_instance: FnDestroyInstance =
        cast_fn(gipa(instance, c"vkDestroyInstance".as_ptr())).ok_or("缺 vkDestroyInstance")?;
    let vk_enum_pd: FnEnumeratePhysicalDevices =
        cast_fn(gipa(instance, c"vkEnumeratePhysicalDevices".as_ptr()))
            .ok_or("缺 vkEnumeratePhysicalDevices")?;
    let vk_get_qf: FnGetPhysicalDeviceQueueFamilyProperties = cast_fn(gipa(
        instance,
        c"vkGetPhysicalDeviceQueueFamilyProperties".as_ptr(),
    ))
    .ok_or("缺 vkGetPhysicalDeviceQueueFamilyProperties")?;
    let vk_get_mem: FnGetPhysicalDeviceMemoryProperties = cast_fn(gipa(
        instance,
        c"vkGetPhysicalDeviceMemoryProperties".as_ptr(),
    ))
    .ok_or("缺 vkGetPhysicalDeviceMemoryProperties")?;
    let vk_create_device: FnCreateDevice =
        cast_fn(gipa(instance, c"vkCreateDevice".as_ptr())).ok_or("缺 vkCreateDevice")?;
    let vk_get_device_proc: FnGetDeviceProcAddr =
        cast_fn(gipa(instance, c"vkGetDeviceProcAddr".as_ptr())).ok_or("缺 vkGetDeviceProcAddr")?;
    // surface 级 instance 符号。
    let create_win32_surface: FnCreateWin32SurfaceKHR =
        cast_fn(gipa(instance, c"vkCreateWin32SurfaceKHR".as_ptr()))
            .ok_or("缺 vkCreateWin32SurfaceKHR(未启用 VK_KHR_win32_surface?)")?;
    let destroy_surface: FnDestroySurfaceKHR =
        cast_fn(gipa(instance, c"vkDestroySurfaceKHR".as_ptr())).ok_or("缺 vkDestroySurfaceKHR")?;
    let get_surf_support: FnGetPhysicalDeviceSurfaceSupportKHR = cast_fn(gipa(
        instance,
        c"vkGetPhysicalDeviceSurfaceSupportKHR".as_ptr(),
    ))
    .ok_or("缺 vkGetPhysicalDeviceSurfaceSupportKHR")?;
    let get_surf_caps: FnGetPhysicalDeviceSurfaceCapabilitiesKHR = cast_fn(gipa(
        instance,
        c"vkGetPhysicalDeviceSurfaceCapabilitiesKHR".as_ptr(),
    ))
    .ok_or("缺 vkGetPhysicalDeviceSurfaceCapabilitiesKHR")?;
    let get_surf_formats: FnGetPhysicalDeviceSurfaceFormatsKHR = cast_fn(gipa(
        instance,
        c"vkGetPhysicalDeviceSurfaceFormatsKHR".as_ptr(),
    ))
    .ok_or("缺 vkGetPhysicalDeviceSurfaceFormatsKHR")?;
    let get_surf_present_modes: FnGetPhysicalDeviceSurfacePresentModesKHR = cast_fn(gipa(
        instance,
        c"vkGetPhysicalDeviceSurfacePresentModesKHR".as_ptr(),
    ))
    .ok_or("缺 vkGetPhysicalDeviceSurfacePresentModesKHR")?;

    // fail-closed messenger(承 offscreen 同模;建于全部 instance-符号 `?` 之后、首个 Vulkan
    // 调用之前 → 创建点与首销毁点间无 `?` 早退,每 early-return 经 destroy_msgr!() 拆除)。
    let validation_error = std::sync::atomic::AtomicBool::new(false);
    let mut messenger: VkDebugUtilsMessengerEXT = VK_NULL_HANDLE;
    let destroy_messenger: Option<FnDestroyDebugUtilsMessengerEXT> = if validation {
        cast_fn(gipa(instance, c"vkDestroyDebugUtilsMessengerEXT".as_ptr()))
    } else {
        None
    };
    if validation
        && let Some(create_messenger) = cast_fn::<FnCreateDebugUtilsMessengerEXT>(gipa(
            instance,
            c"vkCreateDebugUtilsMessengerEXT".as_ptr(),
        ))
    {
        let dumci = DebugUtilsMessengerCreateInfoEXT {
            s_type: ST_DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
            p_next: std::ptr::null(),
            flags: 0,
            message_severity: DEBUG_UTILS_SEVERITY_ERROR,
            message_type: DEBUG_UTILS_TYPE_GENERAL
                | DEBUG_UTILS_TYPE_VALIDATION
                | DEBUG_UTILS_TYPE_PERFORMANCE,
            pfn_user_callback: debug_messenger_cb,
            p_user_data: &validation_error as *const std::sync::atomic::AtomicBool as *mut c_void,
        };
        let _ = create_messenger(instance, &dumci, std::ptr::null(), &mut messenger);
    }
    macro_rules! destroy_msgr {
        () => {
            if let Some(dm) = destroy_messenger {
                if messenger != VK_NULL_HANDLE {
                    dm(instance, messenger, std::ptr::null());
                }
            }
        };
    }

    // ── surface(vkCreateWin32SurfaceKHR)──
    let w32ci = Win32SurfaceCreateInfoKHR {
        s_type: ST_WIN32_SURFACE_CREATE_INFO_KHR,
        p_next: std::ptr::null(),
        flags: 0,
        hinstance,
        hwnd,
    };
    let mut surface: VkSurfaceKHR = VK_NULL_HANDLE;
    if create_win32_surface(instance, &w32ci, std::ptr::null(), &mut surface) != VK_SUCCESS {
        destroy_msgr!();
        vk_destroy_instance(instance, std::ptr::null());
        return Err("vkCreateWin32SurfaceKHR 失败".into());
    }
    macro_rules! teardown_surface_instance {
        () => {{
            destroy_surface(instance, surface, std::ptr::null());
            destroy_msgr!();
            vk_destroy_instance(instance, std::ptr::null());
        }};
    }

    // 物理设备。
    let mut count = 0u32;
    vk_enum_pd(instance, &mut count, std::ptr::null_mut());
    if count == 0 {
        teardown_surface_instance!();
        return Err("无 Vulkan 物理设备".into());
    }
    let mut pds = vec![std::ptr::null_mut::<c_void>(); count as usize];
    vk_enum_pd(instance, &mut count, pds.as_mut_ptr());
    let pd = pds[0];

    // graphics + present 兼备的 queue family。
    let mut qf_count = 0u32;
    vk_get_qf(pd, &mut qf_count, std::ptr::null_mut());
    let mut qfs: Vec<QueueFamilyProperties> = (0..qf_count)
        .map(|_| QueueFamilyProperties {
            queue_flags: 0,
            queue_count: 0,
            timestamp_valid_bits: 0,
            min_image_transfer_granularity: VkExtent3D {
                width: 0,
                height: 0,
                depth: 0,
            },
        })
        .collect();
    vk_get_qf(pd, &mut qf_count, qfs.as_mut_ptr());
    let mut qfi_opt: Option<u32> = None;
    for (i, q) in qfs.iter().enumerate() {
        if q.queue_flags & QUEUE_GRAPHICS_BIT == 0 {
            continue;
        }
        let mut supported: VkBool32 = 0;
        get_surf_support(pd, i as u32, surface, &mut supported);
        if supported != 0 {
            qfi_opt = Some(i as u32);
            break;
        }
    }
    let qfi = match qfi_opt {
        Some(i) => i,
        None => {
            teardown_surface_instance!();
            return Err("无 present-capable graphics queue family".into());
        }
    };

    // device(+ VK_KHR_swapchain)。
    let prio = [1.0f32];
    let dqci = DeviceQueueCreateInfo {
        s_type: ST_DEVICE_QUEUE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        queue_family_index: qfi,
        queue_count: 1,
        p_queue_priorities: prio.as_ptr(),
    };
    let dev_exts: [*const c_char; 1] = [c"VK_KHR_swapchain".as_ptr()];
    let dci = DeviceCreateInfo {
        s_type: ST_DEVICE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        queue_create_info_count: 1,
        p_queue_create_infos: &dqci,
        enabled_layer_count: 0,
        pp_enabled_layer_names: std::ptr::null(),
        enabled_extension_count: 1,
        pp_enabled_extension_names: dev_exts.as_ptr(),
        p_enabled_features: std::ptr::null(),
    };
    let mut device: VkDevice = std::ptr::null_mut();
    let r = vk_create_device(pd, &dci, std::ptr::null(), &mut device);
    if r != VK_SUCCESS {
        teardown_surface_instance!();
        return Err(format!("vkCreateDevice(present) 失败: {r}"));
    }

    let mut out = present_body(
        vk_get_device_proc,
        device,
        pd,
        vk_get_mem,
        qfi,
        surface,
        &get_surf_caps,
        &get_surf_formats,
        &get_surf_present_modes,
        vs_spv,
        fs_spv,
        c"main", // win32 present 无 red_selftest → 恒真入口名(桌面 red 在 offscreen VUID-08742 路)。
        vertices,
        vertex_stride,
        attrs,
        width,
        height,
        clear,
        frames,
    )
    // win32 wrapper 丢弃真实 extent(win32 客户区固定 == 请求 w/h,签名不变);
    // 真实 extent 仅 android 全屏 present 需要(`run_graphics_present_android` 保留)。
    .map(|(pixels, _ext_w, _ext_h)| pixels);

    // fail-closed(L3):validation 开 + ERROR 级校验消息 → 覆盖为 Err(退出码判红)。
    if validation && validation_error.load(std::sync::atomic::Ordering::Relaxed) {
        out =
            Err("VK_LAYER_KHRONOS_validation 报 ERROR 级校验错误(见 stderr;fail-closed,L3)".into());
    }

    let vk_destroy_device: Option<FnDestroyDevice> =
        cast_fn(vk_get_device_proc(device, c"vkDestroyDevice".as_ptr()));
    if let Some(dd) = vk_destroy_device {
        dd(device, std::ptr::null());
    }
    teardown_surface_instance!();
    out
}

/// swapchain + 渲染循环 + 逐帧 present + readback(device 级;句柄逆序销毁)。平台无关
/// (仅依赖 `VkSurfaceKHR` + device/surface 符号,无 win32/android 特化)——win32
/// (`present_vk`)与 android(`run_graphics_present_android`)present 共用本体;故 gate 于
/// `any(windows, android)`(两处唯一调用方所在平台;避免其他平台编入未用 `unsafe fn`)。
/// 返回 `(最后一帧紧凑 RGBA8, ext_w, ext_h)`——**全屏 present 的真实 extent 由 surface
/// `currentExtent` 决定(非入参 w/h)**,调用方须据真实 extent 索引 corner/center 像素。
#[cfg(any(windows, target_os = "android"))]
#[allow(clippy::too_many_arguments)]
unsafe fn present_body(
    gdpa: FnGetDeviceProcAddr,
    device: VkDevice,
    pd: VkPhysicalDevice,
    vk_get_mem: FnGetPhysicalDeviceMemoryProperties,
    qfi: u32,
    surface: VkSurfaceKHR,
    get_surf_caps: &FnGetPhysicalDeviceSurfaceCapabilitiesKHR,
    get_surf_formats: &FnGetPhysicalDeviceSurfaceFormatsKHR,
    get_surf_present_modes: &FnGetPhysicalDeviceSurfacePresentModesKHR,
    vs_spv: &[u32],
    fs_spv: &[u32],
    // vertex stage pipeline `pName`：绿路 = 真入口名 `c"main"`;android red_selftest = 模块内不
    // 存在的假名(SPIR-V 保持原样合法,仅入口名不匹配 → 干净触 pName VUID,详见调用方)。
    vs_entry: &std::ffi::CStr,
    vertices: &[u8],
    vertex_stride: u32,
    attrs: &[(u32, u32, u32)],
    width: u32,
    height: u32,
    clear: [f32; 4],
    frames: u32,
) -> Result<(Vec<u8>, u32, u32), String> {
    macro_rules! dp {
        ($name:literal, $ty:ty) => {
            cast_fn::<$ty>(gdpa(device, $name.as_ptr())).ok_or("缺 device 符号")?
        };
    }
    let get_queue: FnGetDeviceQueue = dp!(c"vkGetDeviceQueue", FnGetDeviceQueue);
    let create_buffer: FnCreateBuffer = dp!(c"vkCreateBuffer", FnCreateBuffer);
    let destroy_buffer: FnDestroyBuffer = dp!(c"vkDestroyBuffer", FnDestroyBuffer);
    let buf_mem_req: FnGetBufferMemoryRequirements = dp!(
        c"vkGetBufferMemoryRequirements",
        FnGetBufferMemoryRequirements
    );
    let alloc_mem: FnAllocateMemory = dp!(c"vkAllocateMemory", FnAllocateMemory);
    let free_mem: FnFreeMemory = dp!(c"vkFreeMemory", FnFreeMemory);
    let bind_buf: FnBindBufferMemory = dp!(c"vkBindBufferMemory", FnBindBufferMemory);
    let map_mem: FnMapMemory = dp!(c"vkMapMemory", FnMapMemory);
    let unmap_mem: FnUnmapMemory = dp!(c"vkUnmapMemory", FnUnmapMemory);
    let create_shader: FnCreateShaderModule = dp!(c"vkCreateShaderModule", FnCreateShaderModule);
    let destroy_shader: FnDestroyShaderModule =
        dp!(c"vkDestroyShaderModule", FnDestroyShaderModule);
    let create_pl: FnCreatePipelineLayout = dp!(c"vkCreatePipelineLayout", FnCreatePipelineLayout);
    let destroy_pl: FnDestroyPipelineLayout =
        dp!(c"vkDestroyPipelineLayout", FnDestroyPipelineLayout);
    let destroy_pipe: FnDestroyPipeline = dp!(c"vkDestroyPipeline", FnDestroyPipeline);
    let create_cmdpool: FnCreateCommandPool = dp!(c"vkCreateCommandPool", FnCreateCommandPool);
    let destroy_cmdpool: FnDestroyCommandPool = dp!(c"vkDestroyCommandPool", FnDestroyCommandPool);
    let alloc_cmd: FnAllocateCommandBuffers =
        dp!(c"vkAllocateCommandBuffers", FnAllocateCommandBuffers);
    let begin_cmd: FnBeginCommandBuffer = dp!(c"vkBeginCommandBuffer", FnBeginCommandBuffer);
    let end_cmd: FnEndCommandBuffer = dp!(c"vkEndCommandBuffer", FnEndCommandBuffer);
    let cmd_bind_pipe: FnCmdBindPipeline = dp!(c"vkCmdBindPipeline", FnCmdBindPipeline);
    let queue_submit: FnQueueSubmit = dp!(c"vkQueueSubmit", FnQueueSubmit);
    let queue_wait: FnQueueWaitIdle = dp!(c"vkQueueWaitIdle", FnQueueWaitIdle);
    let destroy_image: FnDestroyImage = dp!(c"vkDestroyImage", FnDestroyImage);
    let create_view: FnCreateImageView = dp!(c"vkCreateImageView", FnCreateImageView);
    let destroy_view: FnDestroyImageView = dp!(c"vkDestroyImageView", FnDestroyImageView);
    let create_rp: FnCreateRenderPass = dp!(c"vkCreateRenderPass", FnCreateRenderPass);
    let destroy_rp: FnDestroyRenderPass = dp!(c"vkDestroyRenderPass", FnDestroyRenderPass);
    let create_fb: FnCreateFramebuffer = dp!(c"vkCreateFramebuffer", FnCreateFramebuffer);
    let destroy_fb: FnDestroyFramebuffer = dp!(c"vkDestroyFramebuffer", FnDestroyFramebuffer);
    let create_gp: FnCreateGraphicsPipelines =
        dp!(c"vkCreateGraphicsPipelines", FnCreateGraphicsPipelines);
    let cmd_begin_rp: FnCmdBeginRenderPass = dp!(c"vkCmdBeginRenderPass", FnCmdBeginRenderPass);
    let cmd_end_rp: FnCmdEndRenderPass = dp!(c"vkCmdEndRenderPass", FnCmdEndRenderPass);
    let cmd_bind_vbuf: FnCmdBindVertexBuffers =
        dp!(c"vkCmdBindVertexBuffers", FnCmdBindVertexBuffers);
    let cmd_draw: FnCmdDraw = dp!(c"vkCmdDraw", FnCmdDraw);
    let cmd_barrier: FnCmdPipelineBarrier = dp!(c"vkCmdPipelineBarrier", FnCmdPipelineBarrier);
    let cmd_copy_img_buf: FnCmdCopyImageToBuffer =
        dp!(c"vkCmdCopyImageToBuffer", FnCmdCopyImageToBuffer);
    // swapchain / semaphore 专属符号。
    let create_swapchain: FnCreateSwapchainKHR = dp!(c"vkCreateSwapchainKHR", FnCreateSwapchainKHR);
    let destroy_swapchain: FnDestroySwapchainKHR =
        dp!(c"vkDestroySwapchainKHR", FnDestroySwapchainKHR);
    let get_swapchain_images: FnGetSwapchainImagesKHR =
        dp!(c"vkGetSwapchainImagesKHR", FnGetSwapchainImagesKHR);
    let acquire_next: FnAcquireNextImageKHR = dp!(c"vkAcquireNextImageKHR", FnAcquireNextImageKHR);
    let queue_present: FnQueuePresentKHR = dp!(c"vkQueuePresentKHR", FnQueuePresentKHR);
    let create_sem: FnCreateSemaphore = dp!(c"vkCreateSemaphore", FnCreateSemaphore);
    let destroy_sem: FnDestroySemaphore = dp!(c"vkDestroySemaphore", FnDestroySemaphore);

    let mut queue: VkQueue = std::ptr::null_mut();
    get_queue(device, qfi, 0, &mut queue);

    let mut memprops = std::mem::zeroed::<PhysicalDeviceMemoryProperties>();
    vk_get_mem(pd, &mut memprops);

    // ── surface caps / format / present mode 协商 ──
    let mut caps = std::mem::zeroed::<SurfaceCapabilitiesKHR>();
    if get_surf_caps(pd, surface, &mut caps) != VK_SUCCESS {
        return Err("vkGetPhysicalDeviceSurfaceCapabilitiesKHR 失败".into());
    }
    let mut fmt_count = 0u32;
    get_surf_formats(pd, surface, &mut fmt_count, std::ptr::null_mut());
    if fmt_count == 0 {
        return Err("surface 无可用 format".into());
    }
    let mut raw_formats: Vec<SurfaceFormatKHR> = (0..fmt_count)
        .map(|_| SurfaceFormatKHR {
            format: 0,
            color_space: 0,
        })
        .collect();
    get_surf_formats(pd, surface, &mut fmt_count, raw_formats.as_mut_ptr());
    let fmt_pairs: Vec<(u32, u32)> = raw_formats
        .iter()
        .map(|f| (f.format, f.color_space))
        .collect();
    let (chosen_format, chosen_cs) = pick_surface_format(&fmt_pairs);

    // present mode:FIFO spec 保证可用;仍探测确认(honesty:实测在位)。
    let mut pm_count = 0u32;
    get_surf_present_modes(pd, surface, &mut pm_count, std::ptr::null_mut());
    let mut present_modes: Vec<u32> = vec![0u32; pm_count as usize];
    if pm_count > 0 {
        get_surf_present_modes(pd, surface, &mut pm_count, present_modes.as_mut_ptr());
    }
    if !present_modes.contains(&PRESENT_MODE_FIFO_KHR) {
        return Err("surface 不含 FIFO present mode(spec 违例)".into());
    }

    // RXS-0221:extent / readback_len 由 swapchain 构建(初次 + OUT_OF_DATE 重建)重查 surface
    // caps 得出并随重建更新(见 'run 内 build_swapchain);此处仅取 min_image_count
    // (缓冲数恒定、不随 extent 变)。
    let min_image_count = choose_min_image_count(caps.min_image_count, caps.max_image_count);

    // 句柄(全 null 初始;末尾逆序销毁非 null 者)。
    let mut swapchain: VkSwapchainKHR = VK_NULL_HANDLE;
    let mut image_views: Vec<VkImageView> = Vec::new();
    let mut framebuffers: Vec<VkFramebuffer> = Vec::new();
    let mut render_pass: VkRenderPass = VK_NULL_HANDLE;
    let mut vbuf: VkBuffer = VK_NULL_HANDLE;
    let mut vbuf_mem: VkDeviceMemory = VK_NULL_HANDLE;
    let mut rbuf: VkBuffer = VK_NULL_HANDLE;
    let mut rbuf_mem: VkDeviceMemory = VK_NULL_HANDLE;
    let mut vs_mod: VkShaderModule = VK_NULL_HANDLE;
    let mut fs_mod: VkShaderModule = VK_NULL_HANDLE;
    let mut pipe_layout: VkPipelineLayout = VK_NULL_HANDLE;
    let mut pipeline: VkPipeline = VK_NULL_HANDLE;
    let mut cmdpool: VkCommandPool = VK_NULL_HANDLE;
    let mut sem_image_available: VkSemaphore = VK_NULL_HANDLE;
    let mut sem_render_finished: VkSemaphore = VK_NULL_HANDLE;

    let make_host_buffer = |usage: u32, size: u64| -> Result<(VkBuffer, VkDeviceMemory), String> {
        let bci = BufferCreateInfo {
            s_type: ST_BUFFER_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            size: size.max(4),
            usage,
            sharing_mode: SHARING_MODE_EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
        };
        let mut buffer: VkBuffer = VK_NULL_HANDLE;
        if create_buffer(device, &bci, std::ptr::null(), &mut buffer) != VK_SUCCESS {
            return Err("vkCreateBuffer 失败".into());
        }
        let mut req = std::mem::zeroed::<MemoryRequirements>();
        buf_mem_req(device, buffer, &mut req);
        let Some(mt) = pick_mem_type(
            &memprops,
            req.memory_type_bits,
            MEM_HOST_VISIBLE | MEM_HOST_COHERENT,
        ) else {
            destroy_buffer(device, buffer, std::ptr::null());
            return Err("无 host-visible+coherent 内存类型".into());
        };
        let mai = MemoryAllocateInfo {
            s_type: ST_MEMORY_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            allocation_size: req.size,
            memory_type_index: mt,
        };
        let mut mem: VkDeviceMemory = VK_NULL_HANDLE;
        if alloc_mem(device, &mai, std::ptr::null(), &mut mem) != VK_SUCCESS {
            destroy_buffer(device, buffer, std::ptr::null());
            return Err("vkAllocateMemory 失败".into());
        }
        bind_buf(device, buffer, mem, 0);
        Ok((buffer, mem))
    };

    let result: Result<(Vec<u8>, u32, u32), String> = 'run: {
        // ── render pass(单 color attachment,CLEAR→STORE,final=TRANSFER_SRC;+ 外部子通道
        //    依赖同步 acquire 的 layout 转换)。format 恒定 → 一次创建,swapchain 重建复用 ──
        let att = AttachmentDescription {
            flags: 0,
            format: chosen_format,
            samples: SAMPLE_COUNT_1,
            load_op: ATTACHMENT_LOAD_OP_CLEAR,
            store_op: ATTACHMENT_STORE_OP_STORE,
            stencil_load_op: ATTACHMENT_LOAD_OP_DONT_CARE,
            stencil_store_op: ATTACHMENT_STORE_OP_DONT_CARE,
            initial_layout: IMAGE_LAYOUT_UNDEFINED,
            final_layout: IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
        };
        let att_ref = AttachmentReference {
            attachment: 0,
            layout: IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
        };
        let subpass = SubpassDescription {
            flags: 0,
            pipeline_bind_point: PIPELINE_BIND_POINT_GRAPHICS,
            input_attachment_count: 0,
            p_input_attachments: std::ptr::null(),
            color_attachment_count: 1,
            p_color_attachments: &att_ref,
            p_resolve_attachments: std::ptr::null(),
            p_depth_stencil_attachment: std::ptr::null(),
            preserve_attachment_count: 0,
            p_preserve_attachments: std::ptr::null(),
        };
        let dep = SubpassDependency {
            src_subpass: SUBPASS_EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT,
            dst_stage_mask: PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT,
            src_access_mask: 0,
            dst_access_mask: ACCESS_COLOR_ATTACHMENT_WRITE,
            dependency_flags: 0,
        };
        let rp_ci = RenderPassCreateInfo {
            s_type: ST_RENDER_PASS_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            attachment_count: 1,
            p_attachments: &att,
            subpass_count: 1,
            p_subpasses: &subpass,
            dependency_count: 1,
            p_dependencies: &dep as *const SubpassDependency as *const c_void,
        };
        if create_rp(device, &rp_ci, std::ptr::null(), &mut render_pass) != VK_SUCCESS {
            break 'run Err("vkCreateRenderPass 失败".into());
        }

        // ── swapchain + imageView + framebuffer 构建闭包(RXS-0221:初次 + OUT_OF_DATE 重建
        //    共用;重查 surface caps extent;`old_swapchain` 传入以复用旧链资源)。返回
        //    (swapchain, images, image_views, framebuffers, ext_w, ext_h);出错前清理自身部分产物 ──
        type SwapResources = (
            VkSwapchainKHR,
            Vec<VkImage>,
            Vec<VkImageView>,
            Vec<VkFramebuffer>,
            u32,
            u32,
        );
        let build_swapchain = |old_swapchain: VkSwapchainKHR| -> Result<SwapResources, String> {
            // 重查 surface caps → 重算 extent / transform / composite alpha(重建时随窗口新尺寸)。
            let mut caps2 = std::mem::zeroed::<SurfaceCapabilitiesKHR>();
            if get_surf_caps(pd, surface, &mut caps2) != VK_SUCCESS {
                return Err("vkGetPhysicalDeviceSurfaceCapabilitiesKHR(rebuild)失败".into());
            }
            let (ew, eh) = choose_present_extent(
                (caps2.current_extent.width, caps2.current_extent.height),
                width,
                height,
                (caps2.min_image_extent.width, caps2.min_image_extent.height),
                (caps2.max_image_extent.width, caps2.max_image_extent.height),
            );
            let sci = SwapchainCreateInfoKHR {
                s_type: ST_SWAPCHAIN_CREATE_INFO_KHR,
                p_next: std::ptr::null(),
                flags: 0,
                surface,
                min_image_count,
                image_format: chosen_format,
                image_color_space: chosen_cs,
                image_extent: VkExtent2D {
                    width: ew,
                    height: eh,
                },
                image_array_layers: 1,
                image_usage: IMAGE_USAGE_COLOR_ATTACHMENT | IMAGE_USAGE_TRANSFER_SRC,
                image_sharing_mode: SHARING_MODE_EXCLUSIVE,
                queue_family_index_count: 0,
                p_queue_family_indices: std::ptr::null(),
                // pre_transform / composite_alpha 由 caps 派生(**不硬编码**):win32 得
                // IDENTITY + OPAQUE(与旧值等价,数值零回归);Android 得设备旋转变换 +
                // INHERIT(硬编码 OPAQUE 会触 VUID-VkSwapchainCreateInfoKHR-compositeAlpha)。
                pre_transform: caps2.current_transform,
                composite_alpha: pick_composite_alpha(caps2.supported_composite_alpha),
                present_mode: PRESENT_MODE_FIFO_KHR,
                clipped: 1,
                old_swapchain, // 重建时复用旧链(初次 = VK_NULL_HANDLE)。
            };
            let mut sc: VkSwapchainKHR = VK_NULL_HANDLE;
            let r = create_swapchain(device, &sci, std::ptr::null(), &mut sc);
            if r != VK_SUCCESS {
                return Err(format!("vkCreateSwapchainKHR 失败: {r}"));
            }
            // swapchain images(所有权归 swapchain,不单独 destroy)。
            let mut img_count = 0u32;
            get_swapchain_images(device, sc, &mut img_count, std::ptr::null_mut());
            if img_count == 0 {
                destroy_swapchain(device, sc, std::ptr::null());
                return Err("swapchain 无 image".into());
            }
            let mut imgs: Vec<VkImage> = vec![VK_NULL_HANDLE; img_count as usize];
            get_swapchain_images(device, sc, &mut img_count, imgs.as_mut_ptr());

            // per-image view + framebuffer(出错前销毁已建 view/fb + swapchain,不泄漏)。
            let mut views: Vec<VkImageView> = Vec::new();
            let mut fbs: Vec<VkFramebuffer> = Vec::new();
            for &img in &imgs {
                let view_ci = ImageViewCreateInfo {
                    s_type: ST_IMAGE_VIEW_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: 0,
                    image: img,
                    view_type: IMAGE_VIEW_TYPE_2D,
                    format: chosen_format,
                    components: VkComponentMapping {
                        r: COMPONENT_SWIZZLE_IDENTITY,
                        g: COMPONENT_SWIZZLE_IDENTITY,
                        b: COMPONENT_SWIZZLE_IDENTITY,
                        a: COMPONENT_SWIZZLE_IDENTITY,
                    },
                    subresource_range: VkImageSubresourceRange {
                        aspect_mask: IMAGE_ASPECT_COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    },
                };
                let mut view: VkImageView = VK_NULL_HANDLE;
                if create_view(device, &view_ci, std::ptr::null(), &mut view) != VK_SUCCESS {
                    for &f in &fbs {
                        destroy_fb(device, f, std::ptr::null());
                    }
                    for &v in &views {
                        destroy_view(device, v, std::ptr::null());
                    }
                    destroy_swapchain(device, sc, std::ptr::null());
                    return Err("vkCreateImageView(swapchain)失败".into());
                }
                views.push(view);
                let fb_ci = FramebufferCreateInfo {
                    s_type: ST_FRAMEBUFFER_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: 0,
                    render_pass,
                    attachment_count: 1,
                    p_attachments: &view,
                    width: ew,
                    height: eh,
                    layers: 1,
                };
                let mut fb: VkFramebuffer = VK_NULL_HANDLE;
                if create_fb(device, &fb_ci, std::ptr::null(), &mut fb) != VK_SUCCESS {
                    for &f in &fbs {
                        destroy_fb(device, f, std::ptr::null());
                    }
                    for &v in &views {
                        destroy_view(device, v, std::ptr::null());
                    }
                    destroy_swapchain(device, sc, std::ptr::null());
                    return Err("vkCreateFramebuffer(swapchain)失败".into());
                }
                fbs.push(fb);
            }
            Ok((sc, imgs, views, fbs, ew, eh))
        };

        // 初次构建(old_swapchain = VK_NULL_HANDLE);extent / readback_len 以实建为准。
        let mut images: Vec<VkImage>;
        let mut ext_w: u32;
        let mut ext_h: u32;
        let mut readback_len: usize;
        match build_swapchain(VK_NULL_HANDLE) {
            Ok((sc, imgs, views, fbs, ew, eh)) => {
                swapchain = sc;
                images = imgs;
                image_views = views;
                framebuffers = fbs;
                ext_w = ew;
                ext_h = eh;
                readback_len = (ew as usize) * (eh as usize) * 4;
            }
            Err(e) => break 'run Err(e),
        }

        // ── vertex buffer + 上传 ──
        match make_host_buffer(BUFFER_USAGE_VERTEX, vertices.len().max(4) as u64) {
            Ok((b, m)) => {
                vbuf = b;
                vbuf_mem = m;
            }
            Err(e) => break 'run Err(e),
        }
        {
            let mut ptr: *mut c_void = std::ptr::null_mut();
            if map_mem(device, vbuf_mem, 0, WHOLE_SIZE, 0, &mut ptr) != VK_SUCCESS {
                break 'run Err("顶点缓冲 vkMapMemory 失败".into());
            }
            std::ptr::copy_nonoverlapping(vertices.as_ptr(), ptr.cast::<u8>(), vertices.len());
            unmap_mem(device, vbuf_mem);
        }

        // ── readback buffer ──
        match make_host_buffer(BUFFER_USAGE_TRANSFER_DST, readback_len as u64) {
            Ok((b, m)) => {
                rbuf = b;
                rbuf_mem = m;
            }
            Err(e) => break 'run Err(e),
        }

        // ── shader modules ──
        let make_shader = |spv: &[u32]| -> Result<VkShaderModule, String> {
            let smci = ShaderModuleCreateInfo {
                s_type: ST_SHADER_MODULE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                code_size: spv.len() * 4,
                p_code: spv.as_ptr(),
            };
            let mut m: VkShaderModule = VK_NULL_HANDLE;
            if create_shader(device, &smci, std::ptr::null(), &mut m) != VK_SUCCESS {
                return Err("vkCreateShaderModule 失败".into());
            }
            Ok(m)
        };
        match make_shader(vs_spv) {
            Ok(m) => vs_mod = m,
            Err(e) => break 'run Err(format!("vertex {e}")),
        }
        match make_shader(fs_spv) {
            Ok(m) => fs_mod = m,
            Err(e) => break 'run Err(format!("fragment {e}")),
        }

        // ── pipeline layout + graphics pipeline ──
        let plci = PipelineLayoutCreateInfo {
            s_type: ST_PIPELINE_LAYOUT_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            set_layout_count: 0,
            p_set_layouts: std::ptr::null(),
            push_constant_range_count: 0,
            p_push_constant_ranges: std::ptr::null(),
        };
        if create_pl(device, &plci, std::ptr::null(), &mut pipe_layout) != VK_SUCCESS {
            break 'run Err("vkCreatePipelineLayout 失败".into());
        }
        let stages = [
            PipelineShaderStageCreateInfo {
                s_type: ST_PIPELINE_SHADER_STAGE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                stage: SHADER_STAGE_VERTEX,
                module: vs_mod,
                // 绿路 = `c"main"`;android red_selftest 传入假名 → 干净触 pName-00707 VUID。
                p_name: vs_entry.as_ptr(),
                p_specialization_info: std::ptr::null(),
            },
            PipelineShaderStageCreateInfo {
                s_type: ST_PIPELINE_SHADER_STAGE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                stage: SHADER_STAGE_FRAGMENT,
                module: fs_mod,
                p_name: c"main".as_ptr(),
                p_specialization_info: std::ptr::null(),
            },
        ];
        let vbind = VkVertexInputBindingDescription {
            binding: 0,
            stride: vertex_stride,
            input_rate: VERTEX_INPUT_RATE_VERTEX,
        };
        let vattrs: Vec<VkVertexInputAttributeDescription> = attrs
            .iter()
            .map(
                |&(location, format, offset)| VkVertexInputAttributeDescription {
                    location,
                    binding: 0,
                    format,
                    offset,
                },
            )
            .collect();
        let vin = PipelineVertexInputStateCreateInfo {
            s_type: ST_PIPELINE_VERTEX_INPUT_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            vertex_binding_description_count: 1,
            p_vertex_binding_descriptions: &vbind,
            vertex_attribute_description_count: vattrs.len() as u32,
            p_vertex_attribute_descriptions: vattrs.as_ptr(),
        };
        let ia = PipelineInputAssemblyStateCreateInfo {
            s_type: ST_PIPELINE_INPUT_ASSEMBLY_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            topology: PRIMITIVE_TOPOLOGY_TRIANGLE_LIST,
            primitive_restart_enable: 0,
        };
        let viewport = VkViewport {
            x: 0.0,
            y: 0.0,
            width: ext_w as f32,
            height: ext_h as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };
        let scissor = VkRect2D {
            offset: VkOffset2D { x: 0, y: 0 },
            extent: VkExtent2D {
                width: ext_w,
                height: ext_h,
            },
        };
        let vp = PipelineViewportStateCreateInfo {
            s_type: ST_PIPELINE_VIEWPORT_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            viewport_count: 1,
            p_viewports: &viewport,
            scissor_count: 1,
            p_scissors: &scissor,
        };
        let rs = PipelineRasterizationStateCreateInfo {
            s_type: ST_PIPELINE_RASTERIZATION_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            depth_clamp_enable: 0,
            rasterizer_discard_enable: 0,
            polygon_mode: POLYGON_MODE_FILL,
            cull_mode: CULL_MODE_NONE,
            front_face: FRONT_FACE_COUNTER_CLOCKWISE,
            depth_bias_enable: 0,
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            line_width: 1.0,
        };
        let ms = PipelineMultisampleStateCreateInfo {
            s_type: ST_PIPELINE_MULTISAMPLE_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            rasterization_samples: SAMPLE_COUNT_1,
            sample_shading_enable: 0,
            min_sample_shading: 0.0,
            p_sample_mask: std::ptr::null(),
            alpha_to_coverage_enable: 0,
            alpha_to_one_enable: 0,
        };
        let blend_att = PipelineColorBlendAttachmentState {
            blend_enable: 0,
            src_color_blend_factor: 0,
            dst_color_blend_factor: 0,
            color_blend_op: 0,
            src_alpha_blend_factor: 0,
            dst_alpha_blend_factor: 0,
            alpha_blend_op: 0,
            color_write_mask: COLOR_COMPONENT_RGBA,
        };
        let cb = PipelineColorBlendStateCreateInfo {
            s_type: ST_PIPELINE_COLOR_BLEND_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            logic_op_enable: 0,
            logic_op: 0,
            attachment_count: 1,
            p_attachments: &blend_att,
            blend_constants: [0.0; 4],
        };
        let gpci = GraphicsPipelineCreateInfo {
            s_type: ST_GRAPHICS_PIPELINE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            stage_count: 2,
            p_stages: stages.as_ptr(),
            p_vertex_input_state: &vin,
            p_input_assembly_state: &ia,
            p_tessellation_state: std::ptr::null(),
            p_viewport_state: &vp,
            p_rasterization_state: &rs,
            p_multisample_state: &ms,
            p_depth_stencil_state: std::ptr::null(),
            p_color_blend_state: &cb,
            p_dynamic_state: std::ptr::null(),
            layout: pipe_layout,
            render_pass,
            subpass: 0,
            base_pipeline_handle: VK_NULL_HANDLE,
            base_pipeline_index: -1,
        };
        if create_gp(
            device,
            VK_NULL_HANDLE,
            1,
            &gpci,
            std::ptr::null(),
            &mut pipeline,
        ) != VK_SUCCESS
        {
            break 'run Err("vkCreateGraphicsPipelines 失败".into());
        }

        // ── semaphores(imageAvailable / renderFinished;逐帧复用,WaitIdle 保证安全)──
        let sem_ci = SemaphoreCreateInfo {
            s_type: ST_SEMAPHORE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
        };
        if create_sem(device, &sem_ci, std::ptr::null(), &mut sem_image_available) != VK_SUCCESS
            || create_sem(device, &sem_ci, std::ptr::null(), &mut sem_render_finished) != VK_SUCCESS
        {
            break 'run Err("vkCreateSemaphore 失败".into());
        }

        // ── command pool(RESET_COMMAND_BUFFER,逐帧重录)──
        let cpci = CommandPoolCreateInfo {
            s_type: ST_COMMAND_POOL_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0x2, // VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT
            queue_family_index: qfi,
        };
        if create_cmdpool(device, &cpci, std::ptr::null(), &mut cmdpool) != VK_SUCCESS {
            break 'run Err("vkCreateCommandPool 失败".into());
        }
        let cbai = CommandBufferAllocateInfo {
            s_type: ST_COMMAND_BUFFER_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            command_pool: cmdpool,
            level: CMD_BUFFER_LEVEL_PRIMARY,
            command_buffer_count: 1,
        };
        let mut cmd: VkCommandBuffer = std::ptr::null_mut();
        alloc_cmd(device, &cbai, &mut cmd);

        let vertex_count = if vertex_stride > 0 {
            (vertices.len() / vertex_stride as usize) as u32
        } else {
            0
        };

        // ── 渲染 / present 循环(RXS-0221:acquire/present 返回 OUT_OF_DATE/SUBOPTIMAL →
        //    vkDeviceWaitIdle 等价 queue idle → 重建 swapchain/imageView/framebuffer(重查
        //    surface caps extent)→ 重建后首帧再校验;失效是正常路径非错误)──
        let mut last_present: VkResult = VK_SUCCESS;
        let mut rebuild_pending = false;
        let mut frame = 0u32;
        while frame < frames {
            // 重建(前一帧 present/acquire 报失效;或 acquire OUT_OF_DATE 重试本帧)。
            if rebuild_pending {
                queue_wait(queue); // 等 GPU idle(单 queue,等价 vkDeviceWaitIdle)。
                let old_sc = swapchain;
                match build_swapchain(old_sc) {
                    Ok((sc, imgs, views, fbs, ew, eh)) => {
                        // 新链已建(old_swapchain 复用)→ 销毁旧 framebuffer/imageView/swapchain。
                        for &f in &framebuffers {
                            destroy_fb(device, f, std::ptr::null());
                        }
                        for &v in &image_views {
                            destroy_view(device, v, std::ptr::null());
                        }
                        destroy_swapchain(device, old_sc, std::ptr::null());
                        swapchain = sc;
                        images = imgs;
                        image_views = views;
                        framebuffers = fbs;
                        // extent 变(客户区 resize)→ 重查后重建 readback buffer(尺寸相关)。
                        if (ew, eh) != (ext_w, ext_h) {
                            ext_w = ew;
                            ext_h = eh;
                            let new_len = (ew as usize) * (eh as usize) * 4;
                            if rbuf != VK_NULL_HANDLE {
                                destroy_buffer(device, rbuf, std::ptr::null());
                            }
                            if rbuf_mem != VK_NULL_HANDLE {
                                free_mem(device, rbuf_mem, std::ptr::null());
                            }
                            match make_host_buffer(BUFFER_USAGE_TRANSFER_DST, new_len as u64) {
                                Ok((b, m)) => {
                                    rbuf = b;
                                    rbuf_mem = m;
                                }
                                Err(e) => break 'run Err(e),
                            }
                            readback_len = new_len;
                        }
                    }
                    Err(e) => break 'run Err(e),
                }
                rebuild_pending = false;
            }

            let mut image_index = 0u32;
            let acq = acquire_next(
                device,
                swapchain,
                u64::MAX,
                sem_image_available,
                VK_NULL_HANDLE,
                &mut image_index,
            );
            match swapchain_present_action(acq) {
                SwapchainAction::Fatal => {
                    break 'run Err(format!("vkAcquireNextImageKHR 失败: {acq}"));
                }
                SwapchainAction::Rebuild if acq == ERROR_OUT_OF_DATE_KHR => {
                    // 未取到 image(信号量未触发)→ 重建后重试本帧,不推进 frame。
                    rebuild_pending = true;
                    continue;
                }
                SwapchainAction::Rebuild => {
                    // SUBOPTIMAL:image 已取到(信号量已触发)→ 本帧照常渲染,present 后再重建。
                    rebuild_pending = true;
                }
                SwapchainAction::Present => {}
            }

            // 录制命令。
            let cbbi = CommandBufferBeginInfo {
                s_type: ST_COMMAND_BUFFER_BEGIN_INFO,
                p_next: std::ptr::null(),
                flags: CMD_BUFFER_USAGE_ONE_TIME_SUBMIT,
                p_inheritance_info: std::ptr::null(),
            };
            begin_cmd(cmd, &cbbi);
            let clear_val = ClearValue { color: clear };
            let rpbi = RenderPassBeginInfo {
                s_type: ST_RENDER_PASS_BEGIN_INFO,
                p_next: std::ptr::null(),
                render_pass,
                framebuffer: framebuffers[image_index as usize],
                render_area: VkRect2D {
                    offset: VkOffset2D { x: 0, y: 0 },
                    extent: VkExtent2D {
                        width: ext_w,
                        height: ext_h,
                    },
                },
                clear_value_count: 1,
                p_clear_values: &clear_val,
            };
            cmd_begin_rp(cmd, &rpbi, SUBPASS_CONTENTS_INLINE);
            cmd_bind_pipe(cmd, PIPELINE_BIND_POINT_GRAPHICS, pipeline);
            let vbuf_offset: VkDeviceSize = 0;
            cmd_bind_vbuf(cmd, 0, 1, &vbuf, &vbuf_offset);
            cmd_draw(cmd, vertex_count, 1, 0, 0);
            cmd_end_rp(cmd);
            // renderpass final=TRANSFER_SRC;补 color-write→transfer-read 可见性屏障后回读。
            let barrier_read = ImageMemoryBarrier {
                s_type: ST_IMAGE_MEMORY_BARRIER,
                p_next: std::ptr::null(),
                src_access_mask: ACCESS_COLOR_ATTACHMENT_WRITE,
                dst_access_mask: ACCESS_TRANSFER_READ,
                old_layout: IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
                new_layout: IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
                src_queue_family_index: QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: QUEUE_FAMILY_IGNORED,
                image: images[image_index as usize],
                subresource_range: VkImageSubresourceRange {
                    aspect_mask: IMAGE_ASPECT_COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
            };
            cmd_barrier(
                cmd,
                PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT,
                PIPELINE_STAGE_TRANSFER,
                0,
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                1,
                &barrier_read,
            );
            let region = VkBufferImageCopy {
                buffer_offset: 0,
                buffer_row_length: 0,
                buffer_image_height: 0,
                image_subresource: VkImageSubresourceLayers {
                    aspect_mask: IMAGE_ASPECT_COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                image_offset: VkOffset3D { x: 0, y: 0, z: 0 },
                image_extent: VkExtent3D {
                    width: ext_w,
                    height: ext_h,
                    depth: 1,
                },
            };
            cmd_copy_img_buf(
                cmd,
                images[image_index as usize],
                IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
                rbuf,
                1,
                &region,
            );
            // copy 后转 PRESENT_SRC_KHR(transfer-read → present)。
            let barrier_present = ImageMemoryBarrier {
                s_type: ST_IMAGE_MEMORY_BARRIER,
                p_next: std::ptr::null(),
                src_access_mask: ACCESS_TRANSFER_READ,
                dst_access_mask: 0,
                old_layout: IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
                new_layout: IMAGE_LAYOUT_PRESENT_SRC_KHR,
                src_queue_family_index: QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: QUEUE_FAMILY_IGNORED,
                image: images[image_index as usize],
                subresource_range: VkImageSubresourceRange {
                    aspect_mask: IMAGE_ASPECT_COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
            };
            cmd_barrier(
                cmd,
                PIPELINE_STAGE_TRANSFER,
                PIPELINE_STAGE_BOTTOM_OF_PIPE,
                0,
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                1,
                &barrier_present,
            );
            end_cmd(cmd);

            // 提交(wait imageAvailable @ COLOR_ATTACHMENT_OUTPUT,signal renderFinished)。
            let wait_stage: VkFlags = PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT;
            let si = SubmitInfo {
                s_type: ST_SUBMIT_INFO,
                p_next: std::ptr::null(),
                wait_semaphore_count: 1,
                p_wait_semaphores: &sem_image_available,
                p_wait_dst_stage_mask: &wait_stage,
                command_buffer_count: 1,
                p_command_buffers: &cmd,
                signal_semaphore_count: 1,
                p_signal_semaphores: &sem_render_finished,
            };
            let sr = queue_submit(queue, 1, &si, VK_NULL_HANDLE);
            if sr != VK_SUCCESS {
                break 'run Err(format!("vkQueueSubmit(present)失败: {sr}"));
            }

            // present(wait renderFinished)。
            let mut present_result: VkResult = VK_SUCCESS;
            let pi = PresentInfoKHR {
                s_type: ST_PRESENT_INFO_KHR,
                p_next: std::ptr::null(),
                wait_semaphore_count: 1,
                p_wait_semaphores: &sem_render_finished,
                swapchain_count: 1,
                p_swapchains: &swapchain,
                p_image_indices: &image_index,
                p_results: &mut present_result,
            };
            last_present = queue_present(queue, &pi);
            // RXS-0221:OUT_OF_DATE/SUBOPTIMAL → 下帧前重建(非错误);其余非预期码翻红。
            match swapchain_present_action(last_present) {
                SwapchainAction::Fatal => {
                    break 'run Err(format!("vkQueuePresentKHR 失败: {last_present}"));
                }
                SwapchainAction::Rebuild => rebuild_pending = true,
                SwapchainAction::Present => {}
            }
            match swapchain_present_action(present_result) {
                SwapchainAction::Fatal => {
                    break 'run Err(format!("present per-swapchain 结果失败: {present_result}"));
                }
                SwapchainAction::Rebuild => rebuild_pending = true,
                SwapchainAction::Present => {}
            }
            queue_wait(queue); // 令 binary semaphore 逐帧复用安全。
            frame += 1;
        }
        let _ = last_present;

        // ── 回读最后一帧紧凑 RGBA8 ──
        let mut ptr: *mut c_void = std::ptr::null_mut();
        if map_mem(device, rbuf_mem, 0, WHOLE_SIZE, 0, &mut ptr) != VK_SUCCESS {
            break 'run Err("回读 vkMapMemory 失败".into());
        }
        let mut pixels = vec![0u8; readback_len];
        std::ptr::copy_nonoverlapping(ptr.cast::<u8>(), pixels.as_mut_ptr(), readback_len);
        unmap_mem(device, rbuf_mem);
        // 返回真实 present extent(全屏 android 由 currentExtent 决定,≠ 入参 w/h)。
        Ok((pixels, ext_w, ext_h))
    };

    // ── 逆序销毁(非 null 者;swapchain image 归 swapchain 所有,不单独 destroy)──
    queue_wait(queue);
    if sem_render_finished != VK_NULL_HANDLE {
        destroy_sem(device, sem_render_finished, std::ptr::null());
    }
    if sem_image_available != VK_NULL_HANDLE {
        destroy_sem(device, sem_image_available, std::ptr::null());
    }
    if cmdpool != VK_NULL_HANDLE {
        destroy_cmdpool(device, cmdpool, std::ptr::null());
    }
    if pipeline != VK_NULL_HANDLE {
        destroy_pipe(device, pipeline, std::ptr::null());
    }
    if pipe_layout != VK_NULL_HANDLE {
        destroy_pl(device, pipe_layout, std::ptr::null());
    }
    if fs_mod != VK_NULL_HANDLE {
        destroy_shader(device, fs_mod, std::ptr::null());
    }
    if vs_mod != VK_NULL_HANDLE {
        destroy_shader(device, vs_mod, std::ptr::null());
    }
    if rbuf != VK_NULL_HANDLE {
        destroy_buffer(device, rbuf, std::ptr::null());
    }
    if rbuf_mem != VK_NULL_HANDLE {
        free_mem(device, rbuf_mem, std::ptr::null());
    }
    if vbuf != VK_NULL_HANDLE {
        destroy_buffer(device, vbuf, std::ptr::null());
    }
    if vbuf_mem != VK_NULL_HANDLE {
        free_mem(device, vbuf_mem, std::ptr::null());
    }
    for &fb in &framebuffers {
        if fb != VK_NULL_HANDLE {
            destroy_fb(device, fb, std::ptr::null());
        }
    }
    if render_pass != VK_NULL_HANDLE {
        destroy_rp(device, render_pass, std::ptr::null());
    }
    for &view in &image_views {
        if view != VK_NULL_HANDLE {
            destroy_view(device, view, std::ptr::null());
        }
    }
    // swapchain image 由 swapchain 拥有 —— 不 destroy_image;仅销毁 swapchain 本体。
    let _ = destroy_image; // (对齐 offscreen 符号集;present 不单独销毁 swapchain image)
    if swapchain != VK_NULL_HANDLE {
        destroy_swapchain(device, swapchain, std::ptr::null());
    }

    result
}

// ── demo 着色器 SPIR-V(build.rs 经 vulkan_codegen 产;android glue + desktop 共享) ──
/// mb1 Android present demo 三着色器 SPIR-V 字节:`(tri_vs, tri_fs, saxpy)`(小端字流,
/// `len % 4 == 0`;消费侧转 u32)。`build.rs` 经 `vulkan_codegen`(纯 Rust MIR→SPIR-V)对
/// `conformance/vulkan/accept/vk_tri_{vs,fs}.rx` 与 `kernels/saxpy.rx` 产,复现命令等价
/// `rurixc --target vulkan <src>.rx`。全绿构建下三者非空;codegen 降级(极少)→ 空切片,
/// 消费侧据空 SKIP(对齐既有 saxpy.spv 降级纪律)。零外部资源——`include_bytes!` 自 `OUT_DIR`。
//@ spec: RXS-0210
pub fn demo_shaders_spv() -> (&'static [u8], &'static [u8], &'static [u8]) {
    const TRI_VS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tri_vs.spv"));
    const TRI_FS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tri_fs.spv"));
    const SAXPY: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/saxpy.spv"));
    (TRI_VS, TRI_FS, SAXPY)
}

/// G3.3 PR-S3 采样超集 device 数值判据模式着色器 SPIR-V(RFC-0013 §4.B8;RXS-0223~0230)。
/// `build.rs` 经 `dxil_spirv::emit_spirv_body_vulkan`(Vulkan 原生路,Vk-native set-per-class
/// 绑定装饰,与 [`run_graphics_offscreen_v2`] 的 `plan_descriptor_sets` 分配律对齐)对
/// `conformance/dxil/graphics/accept/sampling_*.rx` 产,`include_bytes!` 自 `OUT_DIR`。全绿
/// 构建下各非空;codegen 降级 → 空切片,harness(`bin/sampling_modes`)据空 SKIP(非 fake)。
pub struct SamplingShadersSpv {
    /// 全屏三角形 vertex(pos + uv perspective varying):sample_lod/gather/cmp/多分量/
    /// wrap-vs-clamp 模式共用(模式 ①③④⑥⑦)。
    pub fullscreen_vs: &'static [u8],
    /// 整型取址 vertex(pos + px flat + val flat):load/storage 模式共用(模式 ②⑤)。
    pub fetch_vs: &'static [u8],
    /// sample_lod fragment(模式 ① mip 选层 / ⑥ wrap-vs-clamp / ⑦ 多分量;LOD=1.0 显式)。
    pub sample_lod_fs: &'static [u8],
    /// load 越界钳制 fragment(模式 ②;OpImageFetch + RXS-0228 钳制序列)。
    pub load_fs: &'static [u8],
    /// gather 角点 2×2 单分量 fragment(模式 ③;OpImageGather 分量字面量 0)。
    pub gather_fs: &'static [u8],
    /// sample_cmp shadow fragment(模式 ④;OpImageSampleDrefExplicitLod 恒 LOD 0)。
    pub cmp_fs: &'static [u8],
    /// TextureRw2D 唯一写者 store→load fragment(模式 ⑤;OpImageWrite/OpImageRead)。
    pub storage_fs: &'static [u8],
}

/// [`SamplingShadersSpv`] 全体(build.rs 嵌入的 device 模式 SPIR-V)。
pub fn sampling_shaders_spv() -> SamplingShadersSpv {
    SamplingShadersSpv {
        fullscreen_vs: include_bytes!(concat!(env!("OUT_DIR"), "/sampling_fullscreen_vs.spv")),
        fetch_vs: include_bytes!(concat!(env!("OUT_DIR"), "/sampling_fetch_vs.spv")),
        sample_lod_fs: include_bytes!(concat!(env!("OUT_DIR"), "/sampling_sample_lod_fs.spv")),
        load_fs: include_bytes!(concat!(env!("OUT_DIR"), "/sampling_load_fs.spv")),
        gather_fs: include_bytes!(concat!(env!("OUT_DIR"), "/sampling_gather_fs.spv")),
        cmp_fs: include_bytes!(concat!(env!("OUT_DIR"), "/sampling_cmp_fs.spv")),
        storage_fs: include_bytes!(concat!(env!("OUT_DIR"), "/sampling_storage_fs.spv")),
    }
}

/// G3.4 bindless device 判据模式着色器 SPIR-V(RFC-0013 §4.C4;RXS-0231~0235)。
/// `build.rs` 经 `dxil_spirv::emit_spirv_body_vulkan`(Vulkan 原生路;无界表 set4 binding0 +
/// sampler set3 binding0 装饰与 [`run_graphics_offscreen_bindless`] 分配律同源镜像)对
/// `conformance/dxil/graphics/accept/bindless_*.rx` 产,`include_bytes!` 自 `OUT_DIR`。
/// codegen 降级 → 空切片,harness(`bin/bindless_modes`)据空 SKIP(非 fake)。
pub struct BindlessShadersSpv {
    /// 四象限 vertex(pos + uv perspective + idx flat u32 passthrough)。
    pub quadrant_vs: &'static [u8],
    /// 无界表动态非均匀索引采样 fragment(`table[nonuniform(idx)].sample(samp, uv)`;
    /// clamp `UMin(idx, table_len-1)`,table_len 经 push-constant 尾槽,RXS-0234)。
    pub sample_fs: &'static [u8],
}

/// [`BindlessShadersSpv`] 全体(build.rs 嵌入的 bindless device 模式 SPIR-V)。
//@ spec: RXS-0234, RXS-0235
pub fn bindless_shaders_spv() -> BindlessShadersSpv {
    BindlessShadersSpv {
        quadrant_vs: include_bytes!(concat!(env!("OUT_DIR"), "/bindless_quadrant_vs.spv")),
        sample_fs: include_bytes!(concat!(env!("OUT_DIR"), "/bindless_sample_fs.spv")),
    }
}

// ── Android on-device present（VK_KHR_android_surface;尾门 G-MB1-7 兑现） ────────
// libandroid liblog:`__android_log_write`——on-device validation 消息(VUID)直落 logcat
// (tag `RurixVK-VVL`),证 layer 真加载(桌面 messenger 走 stderr,android 无用故改 logcat)。
// 符号在最终 cdylib 链接期由 glue crate 的 `-llog` 解析(rlib 不解析,不影响桌面构建)。
#[cfg(target_os = "android")]
unsafe extern "C" {
    fn __android_log_write(
        prio: core::ffi::c_int,
        tag: *const c_char,
        text: *const c_char,
    ) -> core::ffi::c_int;
}
#[cfg(target_os = "android")]
const ANDROID_LOG_ERROR: core::ffi::c_int = 6;

/// Android debug messenger 回调:ERROR 级校验消息 → `validation_errors`(栈上 `AtomicU32`)
/// `+= 1` + 消息落 logcat(tag `RurixVK-VVL`,ERROR)。返回 `VK_FALSE`(不中断被回调命令,
/// 仅记录;fail-closed 在 `run_graphics_present_android` 末尾据计数统一判)。桌面 `debug_messenger_cb`
/// 保持不变(走 stderr + `AtomicBool`);本 android 变体额外走 logcat 且用 `AtomicU32` 计数。
#[cfg(target_os = "android")]
unsafe extern "system" fn debug_messenger_cb_android(
    severity: u32,
    _types: u32,
    data: *const DebugUtilsMessengerCallbackDataEXT,
    user_data: *mut c_void,
) -> u32 {
    if severity & DEBUG_UTILS_SEVERITY_ERROR != 0 {
        if !user_data.is_null() {
            // SAFETY: user_data 指向 run_graphics_present_android 栈上 AtomicU32;messenger 生命周期
            // 严格短于该 AtomicU32(末尾 instance destroy 前销毁)。原子加经共享引用合法,无 &mut 别名。
            let ctr = &*(user_data as *const std::sync::atomic::AtomicU32);
            ctr.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        if !data.is_null() {
            // SAFETY: 回调契约保证 data 在回调期间有效;p_message 为有效 NUL 结尾 C 串。
            let d = &*data;
            if !d.p_message.is_null() {
                // SAFETY: p_message 为有效 NUL 结尾 C 串;tag 为 NUL 结尾字面量;直接转发 liblog。
                __android_log_write(ANDROID_LOG_ERROR, c"RurixVK-VVL".as_ptr(), d.p_message);
            }
        }
    }
    0
}

/// Android surface present（`VK_KHR_android_surface`;on-device 尾门 G-MB1-7 兑现的出图循环）。
/// 镜像 win32 `present_vk`——instance 扩展换 `VK_KHR_surface`+`VK_KHR_android_surface`,surface
/// 由 `ANativeWindow*` 经 `vkCreateAndroidSurfaceKHR`(复用既有 `create_android_surface`)建,其余
/// 物理设备/queue-family(graphics+present)/`vkCreateDevice([VK_KHR_swapchain])`/平台无关
/// `present_body`(swapchain acquire→render→copy readback→PRESENT_SRC→present)全复用。返回
/// `(最后一帧紧凑 RGBA8, ext_w, ext_h)`——**全屏 extent 由 surface `currentExtent` 决定(非入参
/// w/h)**,调用方据真实 extent 索引 corner/center。
///
/// - `validation=true`:装 `VK_LAYER_KHRONOS_validation` + `VK_EXT_debug_utils` messenger,每条
///   ERROR 级校验消息经 android callback 落 logcat(`RurixVK-VVL`)并计数,末尾 **fail-closed**
///   翻 `Err`(反假绿:「零报错」仅在 layer 真加载〔RED 见 VUID〕前提下采信)。
/// - `red_selftest=true`:vertex stage `pName` 用**模块内不存在的假入口名**(SPIR-V 保持原样
///   合法),使 graphics pipeline 建立干净触 `VUID-VkPipelineShaderStageCreateInfo-pName-00707`
///   证 layer 真加载 → 期望 messenger 计数 >0 + logcat 见 VUID + 本函数 fail-closed 返回 `Err`
///   (pipeline create 若直接返回 `VK_ERROR`〔入口名不存在〕亦判红,且 messenger 为 instance 级、
///   pipeline create 时已活跃,VUID 已先落 logcat)。旧机制喂损坏 `.spv`(pCode-08742)已弃——
///   某些 layer 解析非法 SPIR-V 自身 SIGSEGV(Adreno/MTE 实测,VUID 未吐出即崩);合法模块 + 假名
///   不向驱动/layer 喂非法 SPIR-V,不存在该崩溃/UB 风险。
///
/// 缺 Vulkan 驱动 / 无 present-capable graphics queue / surface 建失败 → 确定性 `Err`(非 panic,
/// P-01)。gate feature `vulkan` 默认关闭。
///
/// # Safety
/// `gipa` 为 `load_vulkan_loader()` 解析所得有效 `vkGetInstanceProcAddr`;`window` 为 Android app
/// 存活期内有效 `ANativeWindow*`(调用方〔渲染线程〕持 `ANativeWindow_acquire` 保活,present 返回
/// 前不 `release`)。内部句柄(instance/surface/device/messenger/swapchain/imageView×N/framebuffer
/// ×N/semaphore×2/pipeline/…)线性配对 create/destroy、逆序销毁(swapchain image 归 swapchain,
/// 不单独 destroy);每个 `#[repr(C)]` VkStruct 逐字节对齐(由 `VK_LAYER_KHRONOS_validation`
/// on-device 真跑实证);messenger `p_user_data` 指向本函数栈上 `AtomicU32`(生命周期严格长于
/// messenger)。`red_selftest` 反证路**始终**向 `vkCreateShaderModule` 喂原样合法 SPIR-V(仅
/// pipeline `pName` 用假入口名),故不存在「向驱动/layer 喂非法 SPIR-V」的解析路径——与 `validation`
/// 开关无关地内存安全(消解旧「损坏 `.spv` 依赖 validation 兜底否则驱动 UB」的 review advisory)。
//@ spec: RXS-0210
//@ spec: RXS-0211
#[cfg(target_os = "android")]
#[allow(clippy::too_many_arguments)]
pub unsafe fn run_graphics_present_android(
    gipa: FnGetInstanceProcAddr,
    window: *mut android_present::ANativeWindow,
    vs_spv: &[u32],
    fs_spv: &[u32],
    vertices: &[u8],
    vertex_stride: u32,
    attrs: &[(u32, u32, u32)],
    width: u32,
    height: u32,
    clear: [f32; 4],
    frames: u32,
    validation: bool,
    red_selftest: bool,
) -> Result<(Vec<u8>, u32, u32), String> {
    let vk_create_instance: FnCreateInstance =
        cast_fn(gipa(std::ptr::null_mut(), c"vkCreateInstance".as_ptr()))
            .ok_or("缺 vkCreateInstance")?;

    let layer_name = c"VK_LAYER_KHRONOS_validation";
    let layers: [*const c_char; 1] = [layer_name.as_ptr()];
    // instance 扩展:present 恒需 surface + android_surface;validation 追加 debug_utils。
    let mut exts: Vec<*const c_char> = vec![
        c"VK_KHR_surface".as_ptr(),
        c"VK_KHR_android_surface".as_ptr(),
    ];
    if validation {
        exts.push(c"VK_EXT_debug_utils".as_ptr());
    }
    let app = ApplicationInfo {
        s_type: ST_APPLICATION_INFO,
        p_next: std::ptr::null(),
        p_application_name: c"rurix-mb1".as_ptr(),
        application_version: 0,
        p_engine_name: c"rurix".as_ptr(),
        engine_version: 0,
        api_version: API_VERSION_1_1,
    };
    let ici = InstanceCreateInfo {
        s_type: ST_INSTANCE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        p_application_info: &app,
        enabled_layer_count: if validation { 1 } else { 0 },
        pp_enabled_layer_names: if validation {
            layers.as_ptr()
        } else {
            std::ptr::null()
        },
        enabled_extension_count: exts.len() as u32,
        pp_enabled_extension_names: exts.as_ptr(),
    };
    let mut instance: VkInstance = std::ptr::null_mut();
    let r = vk_create_instance(&ici, std::ptr::null(), &mut instance);
    if r != VK_SUCCESS {
        return Err(format!("vkCreateInstance(android present) 失败: {r}"));
    }

    // instance 级符号。
    let vk_destroy_instance: FnDestroyInstance =
        cast_fn(gipa(instance, c"vkDestroyInstance".as_ptr())).ok_or("缺 vkDestroyInstance")?;
    let vk_enum_pd: FnEnumeratePhysicalDevices =
        cast_fn(gipa(instance, c"vkEnumeratePhysicalDevices".as_ptr()))
            .ok_or("缺 vkEnumeratePhysicalDevices")?;
    let vk_get_qf: FnGetPhysicalDeviceQueueFamilyProperties = cast_fn(gipa(
        instance,
        c"vkGetPhysicalDeviceQueueFamilyProperties".as_ptr(),
    ))
    .ok_or("缺 vkGetPhysicalDeviceQueueFamilyProperties")?;
    let vk_get_mem: FnGetPhysicalDeviceMemoryProperties = cast_fn(gipa(
        instance,
        c"vkGetPhysicalDeviceMemoryProperties".as_ptr(),
    ))
    .ok_or("缺 vkGetPhysicalDeviceMemoryProperties")?;
    let vk_create_device: FnCreateDevice =
        cast_fn(gipa(instance, c"vkCreateDevice".as_ptr())).ok_or("缺 vkCreateDevice")?;
    let vk_get_device_proc: FnGetDeviceProcAddr =
        cast_fn(gipa(instance, c"vkGetDeviceProcAddr".as_ptr())).ok_or("缺 vkGetDeviceProcAddr")?;
    // android surface 级 instance 符号。
    let create_android_surface_fn: android_present::FnCreateAndroidSurfaceKHR =
        cast_fn(gipa(instance, c"vkCreateAndroidSurfaceKHR".as_ptr()))
            .ok_or("缺 vkCreateAndroidSurfaceKHR(未启用 VK_KHR_android_surface?)")?;
    let destroy_surface: FnDestroySurfaceKHR =
        cast_fn(gipa(instance, c"vkDestroySurfaceKHR".as_ptr())).ok_or("缺 vkDestroySurfaceKHR")?;
    let get_surf_support: FnGetPhysicalDeviceSurfaceSupportKHR = cast_fn(gipa(
        instance,
        c"vkGetPhysicalDeviceSurfaceSupportKHR".as_ptr(),
    ))
    .ok_or("缺 vkGetPhysicalDeviceSurfaceSupportKHR")?;
    let get_surf_caps: FnGetPhysicalDeviceSurfaceCapabilitiesKHR = cast_fn(gipa(
        instance,
        c"vkGetPhysicalDeviceSurfaceCapabilitiesKHR".as_ptr(),
    ))
    .ok_or("缺 vkGetPhysicalDeviceSurfaceCapabilitiesKHR")?;
    let get_surf_formats: FnGetPhysicalDeviceSurfaceFormatsKHR = cast_fn(gipa(
        instance,
        c"vkGetPhysicalDeviceSurfaceFormatsKHR".as_ptr(),
    ))
    .ok_or("缺 vkGetPhysicalDeviceSurfaceFormatsKHR")?;
    let get_surf_present_modes: FnGetPhysicalDeviceSurfacePresentModesKHR = cast_fn(gipa(
        instance,
        c"vkGetPhysicalDeviceSurfacePresentModesKHR".as_ptr(),
    ))
    .ok_or("缺 vkGetPhysicalDeviceSurfacePresentModesKHR")?;

    // fail-closed messenger(android:每条 ERROR 记 logcat RurixVK-VVL + 计数;末尾据计数翻 Err)。
    // 建于全部 instance-符号 `?` 之后、首个 Vulkan 调用之前 → 创建点与首销毁点间无 `?` 早退。
    let validation_errors = std::sync::atomic::AtomicU32::new(0);
    let mut messenger: VkDebugUtilsMessengerEXT = VK_NULL_HANDLE;
    let destroy_messenger: Option<FnDestroyDebugUtilsMessengerEXT> = if validation {
        cast_fn(gipa(instance, c"vkDestroyDebugUtilsMessengerEXT".as_ptr()))
    } else {
        None
    };
    if validation
        && let Some(create_messenger) = cast_fn::<FnCreateDebugUtilsMessengerEXT>(gipa(
            instance,
            c"vkCreateDebugUtilsMessengerEXT".as_ptr(),
        ))
    {
        let dumci = DebugUtilsMessengerCreateInfoEXT {
            s_type: ST_DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
            p_next: std::ptr::null(),
            flags: 0,
            message_severity: DEBUG_UTILS_SEVERITY_ERROR,
            message_type: DEBUG_UTILS_TYPE_GENERAL
                | DEBUG_UTILS_TYPE_VALIDATION
                | DEBUG_UTILS_TYPE_PERFORMANCE,
            pfn_user_callback: debug_messenger_cb_android,
            p_user_data: &validation_errors as *const std::sync::atomic::AtomicU32 as *mut c_void,
        };
        let _ = create_messenger(instance, &dumci, std::ptr::null(), &mut messenger);
    }
    macro_rules! destroy_msgr {
        () => {
            if let Some(dm) = destroy_messenger {
                if messenger != VK_NULL_HANDLE {
                    dm(instance, messenger, std::ptr::null());
                }
            }
        };
    }

    // ── android surface（vkCreateAndroidSurfaceKHR，复用 create_android_surface）──
    let surface: VkSurfaceKHR = match android_present::create_android_surface(
        instance,
        window,
        create_android_surface_fn,
    ) {
        Ok(s) => s,
        Err(e) => {
            destroy_msgr!();
            vk_destroy_instance(instance, std::ptr::null());
            return Err(e);
        }
    };
    macro_rules! teardown_surface_instance {
        () => {{
            destroy_surface(instance, surface, std::ptr::null());
            destroy_msgr!();
            vk_destroy_instance(instance, std::ptr::null());
        }};
    }

    // 物理设备。
    let mut count = 0u32;
    vk_enum_pd(instance, &mut count, std::ptr::null_mut());
    if count == 0 {
        teardown_surface_instance!();
        return Err("无 Vulkan 物理设备".into());
    }
    let mut pds = vec![std::ptr::null_mut::<c_void>(); count as usize];
    vk_enum_pd(instance, &mut count, pds.as_mut_ptr());
    let pd = pds[0];

    // graphics + present 兼备的 queue family。
    let mut qf_count = 0u32;
    vk_get_qf(pd, &mut qf_count, std::ptr::null_mut());
    let mut qfs: Vec<QueueFamilyProperties> = (0..qf_count)
        .map(|_| QueueFamilyProperties {
            queue_flags: 0,
            queue_count: 0,
            timestamp_valid_bits: 0,
            min_image_transfer_granularity: VkExtent3D {
                width: 0,
                height: 0,
                depth: 0,
            },
        })
        .collect();
    vk_get_qf(pd, &mut qf_count, qfs.as_mut_ptr());
    let mut qfi_opt: Option<u32> = None;
    for (i, q) in qfs.iter().enumerate() {
        if q.queue_flags & QUEUE_GRAPHICS_BIT == 0 {
            continue;
        }
        let mut supported: VkBool32 = 0;
        get_surf_support(pd, i as u32, surface, &mut supported);
        if supported != 0 {
            qfi_opt = Some(i as u32);
            break;
        }
    }
    let qfi = match qfi_opt {
        Some(i) => i,
        None => {
            teardown_surface_instance!();
            return Err("无 present-capable graphics queue family".into());
        }
    };

    // device(+ VK_KHR_swapchain)。
    let prio = [1.0f32];
    let dqci = DeviceQueueCreateInfo {
        s_type: ST_DEVICE_QUEUE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        queue_family_index: qfi,
        queue_count: 1,
        p_queue_priorities: prio.as_ptr(),
    };
    let dev_exts: [*const c_char; 1] = [c"VK_KHR_swapchain".as_ptr()];
    let dci = DeviceCreateInfo {
        s_type: ST_DEVICE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        queue_create_info_count: 1,
        p_queue_create_infos: &dqci,
        enabled_layer_count: 0,
        pp_enabled_layer_names: std::ptr::null(),
        enabled_extension_count: 1,
        pp_enabled_extension_names: dev_exts.as_ptr(),
        p_enabled_features: std::ptr::null(),
    };
    let mut device: VkDevice = std::ptr::null_mut();
    let r = vk_create_device(pd, &dci, std::ptr::null(), &mut device);
    if r != VK_SUCCESS {
        teardown_surface_instance!();
        return Err(format!("vkCreateDevice(android present) 失败: {r}"));
    }

    // red_selftest:vertex stage `pName` 用**模块内不存在的假入口名**(SPIR-V 保持原样合法),使
    // graphics pipeline 建立干净触 `VUID-VkPipelineShaderStageCreateInfo-pName-00707`(本仓库桌面
    // compute 冒烟已实证:错入口名触 pName VUID,layer 干净报错不崩)证 layer 真加载。green = 真
    // 入口名 `c"main"`。旧机制(损坏 vertex `.spv` 字节喂 vkCreateShaderModule 触 pCode-08742)已弃:
    // 某些 layer 解析非法 SPIR-V 时自身 SIGSEGV(HONOR Adreno/Android16+MTE 实测,layer 错误格式化
    // 路径内存 bug 被 MTE 抓死,VUID 未吐出即崩)→ RED 取证失败;合法模块 + 假名不向驱动/layer 喂
    // 非法 SPIR-V,天然消除该崩溃/UB 风险,不依赖 validation 兜底。
    let vs_entry: &std::ffi::CStr = if red_selftest {
        c"rurix_red_bogus_entry"
    } else {
        c"main"
    };

    let mut out = present_body(
        vk_get_device_proc,
        device,
        pd,
        vk_get_mem,
        qfi,
        surface,
        &get_surf_caps,
        &get_surf_formats,
        &get_surf_present_modes,
        vs_spv,
        fs_spv,
        vs_entry,
        vertices,
        vertex_stride,
        attrs,
        width,
        height,
        clear,
        frames,
    );

    // fail-closed:validation 开 + 出现 ERROR 级校验消息 → 覆盖为 Err(反假绿判据的根)。
    let verr = validation_errors.load(std::sync::atomic::Ordering::Relaxed);
    if validation && verr > 0 {
        out = Err(format!(
            "VK_LAYER_KHRONOS_validation 报 {verr} 条 ERROR 级校验错误(见 logcat RurixVK-VVL;fail-closed)"
        ));
    }

    let vk_destroy_device: Option<FnDestroyDevice> =
        cast_fn(vk_get_device_proc(device, c"vkDestroyDevice".as_ptr()));
    if let Some(dd) = vk_destroy_device {
        dd(device, std::ptr::null());
    }
    teardown_surface_instance!();
    out
}

/// `run_graphics_present_android` 的 loader-管理入口:装载 loader 后转内层(镜像 win32
/// `run_graphics_present` 但**保留 `unsafe`**——win32 版自建窗口无指针入参故 safe,本版必须收
/// 外部 `ANativeWindow*`,其有效性是调用方义务,故按正确性判为 `unsafe fn`(clippy
/// `not_unsafe_ptr_arg_deref` 亦印证:裸 window 前向至 unsafe 内层不得由 safe fn 承载)。名后缀
/// `_safe` 指「免调用方自持 gipa」的高层封装,非「内存 safe」。
///
/// # Safety
/// `window` 为 Android app 存活期内有效 `ANativeWindow*`(调用方〔渲染线程〕持
/// `ANativeWindow_acquire` 保活,本调用返回前不 `release`);其余同
/// `run_graphics_present_android` 的 `# Safety`(loader/句柄生命周期由内层线性管理)。
//@ spec: RXS-0210
//@ spec: RXS-0211
#[cfg(target_os = "android")]
#[allow(clippy::too_many_arguments)]
pub unsafe fn run_graphics_present_android_safe(
    window: *mut android_present::ANativeWindow,
    vs_spv: &[u32],
    fs_spv: &[u32],
    vertices: &[u8],
    vertex_stride: u32,
    attrs: &[(u32, u32, u32)],
    width: u32,
    height: u32,
    clear: [f32; 4],
    frames: u32,
    validation: bool,
    red_selftest: bool,
) -> Result<(Vec<u8>, u32, u32), String> {
    let gipa = load_vulkan_loader().ok_or("vulkan loader (libvulkan.so) 不可用")?;
    // SAFETY(unsafe fn 内,模块 allow(unsafe_op_in_unsafe_fn)):gipa 由 loader 解析;window 有效性
    // 由调用方(渲染线程持 ANativeWindow_acquire)担保并经本 fn `# Safety` 上传;句柄逆序销毁在内层。
    run_graphics_present_android(
        gipa,
        window,
        vs_spv,
        fs_spv,
        vertices,
        vertex_stride,
        attrs,
        width,
        height,
        clear,
        frames.max(1),
        validation,
        red_selftest,
    )
}

// ── Android surface 创建 FFI 缝(VK_KHR_android_surface) ──────────────────────
// `create_android_surface` 由上方 on-device present 编排(`run_graphics_present_android`)复用;
// compute 语义与本模块无关(compute 不需 surface,`enabled_extension_count=0`)。
#[cfg(target_os = "android")]
pub mod android_present {
    use core::ffi::c_void;

    /// 由 Android app(NativeActivity / GameActivity)经 JNI/native glue 提供的不透明窗口。
    #[repr(C)]
    pub struct ANativeWindow {
        _private: [u8; 0],
    }

    type VkInstance = *mut c_void;
    type VkSurfaceKHR = u64;
    const ST_ANDROID_SURFACE_CREATE_INFO_KHR: u32 = 1_000_008_000;

    #[repr(C)]
    pub struct AndroidSurfaceCreateInfoKHR {
        s_type: u32,
        p_next: *const c_void,
        flags: u32,
        window: *mut ANativeWindow,
    }

    // pub:`run_graphics_present_android`(vk.rs 主作用域)须 `cast_fn` 解析该 FFI 类型再传入
    // `create_android_surface`;`AndroidSurfaceCreateInfoKHR` 仍私有(模块内封装)。
    pub type FnCreateAndroidSurfaceKHR = unsafe extern "system" fn(
        VkInstance,
        *const AndroidSurfaceCreateInfoKHR,
        *const c_void,
        *mut VkSurfaceKHR,
    ) -> i32;

    /// 从 ANativeWindow* 建 VkSurfaceKHR。要求 instance 已启用扩展
    /// `VK_KHR_surface` + `VK_KHR_android_surface`(present 路径 vkCreateInstance 时启用;
    /// compute 路径不启用,故 run_compute 的 InstanceCreateInfo 保持 enabled_extension_count=0)。
    ///
    /// # Safety
    /// `instance` 为有效 VkInstance;`window` 为 Android app 存活期内的有效 ANativeWindow*;
    /// `create_fn` 为 vkGetInstanceProcAddr(instance,"vkCreateAndroidSurfaceKHR") 解析所得。
    pub unsafe fn create_android_surface(
        instance: VkInstance,
        window: *mut ANativeWindow,
        create_fn: FnCreateAndroidSurfaceKHR,
    ) -> Result<VkSurfaceKHR, String> {
        let ci = AndroidSurfaceCreateInfoKHR {
            s_type: ST_ANDROID_SURFACE_CREATE_INFO_KHR,
            p_next: core::ptr::null(),
            flags: 0,
            window,
        };
        let mut surface: VkSurfaceKHR = 0;
        // SAFETY: ci 布局与 VkAndroidSurfaceCreateInfoKHR 逐字节对齐;window 由调用方担保有效。
        let r = create_fn(instance, &ci, core::ptr::null(), &mut surface);
        if r != 0 {
            return Err(format!("vkCreateAndroidSurfaceKHR 失败: {r}"));
        }
        Ok(surface)
    }
}

// ── G3.5 render graph Vulkan 执行器（RXS-0240）──────────────────────────────────────
//
// `run_graph` 多 pass command buffer 录制（逐 pass render pass begin/end + 边界
// `vkCmdPipelineBarrier`）**逐字重放** `graph.rs`（RXS-0238）纯 host 推导产物——layout/stage/access
// 全取自 `graph.rs` 同源表，执行器**禁二次推导或语义重映射**（P-11 单一事实源）。现
// `run_graphics_offscreen` / `run_graphics_offscreen_v2` / `run_graphics_present` 手写定点 barrier
// 路径 0-byte 保留（RXS-0240b）；device 多 pass 出图段真跑归主循环（活驱动，D3D12 shim pass/barrier
// 数组下发入口大改留后续 = 诚实边界）。本节落地**逐字重放映射本体**（host 可单测，无 device 调用）。

/// 把 `graph.rs` 纯 host 推导的一条 image [`crate::graph::PlannedBarrier`] 逐字重放为
/// `vkCmdPipelineBarrier` 的 `VkImageMemoryBarrier` 字段与 stage 掩码（RXS-0240）：返回
/// `(old_layout, new_layout, src_access, dst_access, src_stage, dst_stage)`——全取自 `graph.rs`
/// 同源表（layout/stage/access 单一事实源），执行器只补 image 句柄与 aspect（按资源类别）。
/// 纯 host（无 device 调用），host 单测恒跑。
#[must_use]
pub fn graph_image_barrier_fields(
    b: &crate::graph::PlannedBarrier,
) -> (u32, u32, VkFlags, VkFlags, VkFlags, VkFlags) {
    (
        b.vk_old_layout,
        b.vk_new_layout,
        b.vk_src_access,
        b.vk_dst_access,
        b.vk_src_stage,
        b.vk_dst_stage,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RXS-0240:render graph Vulkan 执行器逐字重放映射(host 纯函数,无设备)——`graph.rs` 推导的
    /// image barrier 逐字段映射为 vk barrier 字段(layout/stage/access 全取自 graph.rs 同源表,禁
    /// 二次推导);deferred 图的 RT→PSR 转换映射为 COLOR_ATTACHMENT_OPTIMAL→SHADER_READ_ONLY_OPTIMAL +
    /// COLOR_ATTACHMENT_OUTPUT→FRAGMENT_SHADER stage + COLOR_ATTACHMENT_WRITE→SHADER_READ access。
    //@ spec: RXS-0240
    #[test]
    fn graph_barrier_replay_maps_verbatim() {
        use crate::graph::{Graph, PassSpec};
        let mut g = Graph::new();
        let a = g.color_target("gbuf");
        let lit = g.color_target("lit");
        g.add_pass(PassSpec::new("geo").writes_rt(a)).unwrap();
        g.add_pass(PassSpec::new("light").reads(a).writes_rt(lit))
            .unwrap();
        let plan = g.execute().unwrap();
        // gbuf 的 RT→PSR image transition。
        let rt_psr = plan
            .iter()
            .find(|b| b.resource_name == "gbuf")
            .expect("gbuf barrier");
        let (old_layout, new_layout, src_access, dst_access, src_stage, dst_stage) =
            graph_image_barrier_fields(rt_psr);
        // 逐字重放:layout/stage/access 与 vk.rs 常量逐值一致(graph.rs 同源表单一事实源)。
        assert_eq!(old_layout, IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL);
        assert_eq!(new_layout, IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL);
        assert_eq!(src_access, ACCESS_COLOR_ATTACHMENT_WRITE);
        assert_eq!(dst_access, ACCESS_SHADER_READ);
        assert_eq!(src_stage, PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT);
        assert_eq!(dst_stage, PIPELINE_STAGE_FRAGMENT_SHADER);
    }

    /// RXS-0210:graphics offscreen 路的内存类型选择(host 纯函数,无设备)——device-local
    /// (color image)与 host-visible+coherent(顶点/回读缓冲)分道选取,`type_bits` 掩码守约。
    //@ spec: RXS-0210
    #[test]
    fn graphics_pick_mem_type_selects_by_property_flags() {
        let mut memprops: PhysicalDeviceMemoryProperties = PhysicalDeviceMemoryProperties {
            memory_type_count: 3,
            memory_types: [MemoryType {
                property_flags: 0,
                heap_index: 0,
            }; 32],
            memory_heap_count: 1,
            memory_heaps: [MemoryHeap { size: 0, flags: 0 }; 16],
        };
        memprops.memory_types[0].property_flags = MEM_DEVICE_LOCAL; // 0x1
        memprops.memory_types[1].property_flags = MEM_HOST_VISIBLE | MEM_HOST_COHERENT; // 0x6
        memprops.memory_types[2].property_flags =
            MEM_DEVICE_LOCAL | MEM_HOST_VISIBLE | MEM_HOST_COHERENT; // 0x7

        // 全类型允许:device-local 取首个含 DEVICE_LOCAL(idx 0);host-visible 取 idx 1。
        assert_eq!(pick_mem_type(&memprops, 0b111, MEM_DEVICE_LOCAL), Some(0));
        assert_eq!(
            pick_mem_type(&memprops, 0b111, MEM_HOST_VISIBLE | MEM_HOST_COHERENT),
            Some(1)
        );
        // type_bits 仅允许 idx 2:两种需求都落到 idx 2。
        assert_eq!(pick_mem_type(&memprops, 0b100, MEM_DEVICE_LOCAL), Some(2));
        assert_eq!(
            pick_mem_type(&memprops, 0b100, MEM_HOST_VISIBLE | MEM_HOST_COHERENT),
            Some(2)
        );
        // 无满足项 → None(fail-closed,上层报 Err 非 panic)。
        assert_eq!(pick_mem_type(&memprops, 0b010, MEM_DEVICE_LOCAL), None);
    }

    /// RXS-0210(present L4 落地,W6):win32 swapchain 协商纯 host helper——extent 协商
    /// (`current==u32::MAX` 自选则 clamp / 否则用 currentExtent)、surface format 优选
    /// (B8G8R8A8/R8G8B8A8 + SRGB_NONLINEAR)、min image count(min+1 clamp max)。无设备。
    //@ spec: RXS-0210
    #[test]
    fn present_swapchain_negotiation_helpers() {
        // extent:current 固定(Windows 常态)→ 必用 currentExtent(忽略 req)。
        assert_eq!(
            choose_present_extent((64, 64), 128, 128, (1, 1), (4096, 4096)),
            (64, 64)
        );
        // extent:current==u32::MAX(surface 允许自选)→ clamp(req) 进 [min,max]。
        assert_eq!(
            choose_present_extent((u32::MAX, u32::MAX), 128, 128, (1, 1), (96, 96)),
            (96, 96) // req 128 clamp 到 max 96
        );
        assert_eq!(
            choose_present_extent((u32::MAX, u32::MAX), 10, 10, (32, 32), (4096, 4096)),
            (32, 32) // req 10 clamp 到 min 32
        );

        // format:优选 B8G8R8A8_UNORM + SRGB_NONLINEAR(即便非首个)。
        assert_eq!(
            pick_surface_format(&[
                (37, 1),
                (FORMAT_B8G8R8A8_UNORM, COLOR_SPACE_SRGB_NONLINEAR_KHR)
            ]),
            (FORMAT_B8G8R8A8_UNORM, COLOR_SPACE_SRGB_NONLINEAR_KHR)
        );
        // format:R8G8B8A8_UNORM + SRGB_NONLINEAR 亦优选。
        assert_eq!(
            pick_surface_format(&[(FORMAT_R8G8B8A8_UNORM, COLOR_SPACE_SRGB_NONLINEAR_KHR)]),
            (FORMAT_R8G8B8A8_UNORM, COLOR_SPACE_SRGB_NONLINEAR_KHR)
        );
        // format:无优选项 → 退回首个可用(Vulkan 保证 count≥1)。
        assert_eq!(pick_surface_format(&[(99, 7), (100, 8)]), (99, 7));

        // min image count:min+1;max>0 时 clamp 进 max。
        assert_eq!(choose_min_image_count(1, 0), 2); // max=0 无上限
        assert_eq!(choose_min_image_count(2, 8), 3);
        assert_eq!(choose_min_image_count(3, 3), 3); // min+1=4 clamp 到 max 3
    }

    /// RXS-0221(RFC-0013 §4.A2 Vulkan 收尾):swapchain 重建协商纯 host helper——
    /// `vkAcquireNextImageKHR`/`vkQueuePresentKHR` 返回码 → 三分类动作
    /// (`VK_SUCCESS`→Present / `OUT_OF_DATE`·`SUBOPTIMAL`→Rebuild / 其余→Fatal)。无设备。
    //@ spec: RXS-0221
    #[test]
    fn swapchain_present_action_classification() {
        // 成功 → 正常呈现。
        assert_eq!(
            swapchain_present_action(VK_SUCCESS),
            SwapchainAction::Present
        );
        // 失效是正常路径 → 重建(OUT_OF_DATE 与 SUBOPTIMAL 首期同走保守重建)。
        assert_eq!(
            swapchain_present_action(ERROR_OUT_OF_DATE_KHR),
            SwapchainAction::Rebuild
        );
        assert_eq!(
            swapchain_present_action(SUBOPTIMAL_KHR),
            SwapchainAction::Rebuild
        );
        // 其余非预期码(如 VK_ERROR_DEVICE_LOST = -4)→ 终止。
        assert_eq!(swapchain_present_action(-4), SwapchainAction::Fatal);
        assert_eq!(swapchain_present_action(-1), SwapchainAction::Fatal);
    }

    /// RXS-0210(W7 android present 派生):composite alpha 择位纯函数——win32 得 OPAQUE
    /// (数值零回归)、android 得 INHERIT;优先级 OPAQUE→INHERIT→PRE→POST→最低置位。无设备。
    //@ spec: RXS-0210
    #[test]
    fn pick_composite_alpha_derivation() {
        // win32 常态:支持 OPAQUE(0x1)→ OPAQUE(与旧硬编码 COMPOSITE_ALPHA_OPAQUE 等价)。
        assert_eq!(pick_composite_alpha(0x1), COMPOSITE_ALPHA_OPAQUE_BIT_KHR);
        // 全支持位集:仍 OPAQUE 优先(win32 常报 0x9 = OPAQUE|INHERIT,取 OPAQUE 数值零回归)。
        assert_eq!(pick_composite_alpha(0xF), COMPOSITE_ALPHA_OPAQUE_BIT_KHR);
        assert_eq!(pick_composite_alpha(0x9), COMPOSITE_ALPHA_OPAQUE_BIT_KHR);
        // Android 常态:不支持 OPAQUE、仅 INHERIT(0x8)→ INHERIT(硬编码 OPAQUE 会触 VUID)。
        assert_eq!(pick_composite_alpha(0x8), COMPOSITE_ALPHA_INHERIT_BIT_KHR);
        // OPAQUE/INHERIT 皆缺:PRE(0x2)优先于 POST(0x4)。
        assert_eq!(
            pick_composite_alpha(0x2 | 0x4),
            COMPOSITE_ALPHA_PRE_MULTIPLIED_BIT_KHR
        );
        assert_eq!(
            pick_composite_alpha(0x4),
            COMPOSITE_ALPHA_POST_MULTIPLIED_BIT_KHR
        );
        // 四优先级外:退回最低置位(spec 保证 supportedCompositeAlpha ≥1 位置位)。
        assert_eq!(pick_composite_alpha(0x10), 0x10);
        assert_eq!(pick_composite_alpha(0x30), 0x10); // 最低置位 = 0x10
        // 全零(不应发生)→ OPAQUE 兜底(非 panic)。
        assert_eq!(pick_composite_alpha(0), COMPOSITE_ALPHA_OPAQUE_BIT_KHR);
    }

    //@ spec: RXS-0207
    #[test]
    fn entry_point_name_parses() {
        // 手工最小 SPIR-V:header(magic/ver/gen/bound/schema)+ OpEntryPoint GLCompute "k"。
        // 仅测 OpEntryPoint 名解析(纯 host,无 Vulkan 设备);pipeline pName 须与之一致。
        let mut spv = vec![0x0723_0203u32, 0x0001_0000, 0, 5, 0];
        spv.push((4u32 << 16) | 15); // OpEntryPoint,wc=4
        spv.push(5); // GLCompute
        spv.push(1); // entry id
        spv.push(u32::from_le_bytes([b'k', 0, 0, 0]));
        assert_eq!(entry_point_name(&spv).as_deref(), Some("k"));
        assert_eq!(entry_point_name(&[]), None);
    }

    //@ spec: RXS-0208
    #[test]
    fn marshalling_ordinal_matches_codegen_binding() {
        // build.rs 经 vulkan_codegen(纯 Rust MIR→SPIR-V)对 kernels/saxpy.rx 产**真** .spv
        // (复现:`rurixc --target vulkan src/rurix-rt/kernels/saxpy.rx`)。本测试解析其
        // 实际 `OpDecorate Binding` / `OpMemberDecorate Offset` 装饰值,核对 codegen 侧描述符
        // 布局(RXS-0203)与运行时 `run_compute` 的 descriptor-binding 构造序位是**单一事实
        // 源**——两侧同源于形参出现序,非各自约定的两份可漂移拷贝。若 codegen 曾产非连续 /
        // 乱序 binding,`run_compute` 的 `binding: i`(vk.rs 描述符布局)将误绑,本测试即红。
        const SAXPY_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/saxpy.spv"));

        if SAXPY_SPV.is_empty() {
            // 构建期 vulkan_codegen 未产(全静态检查失败/降级)→ dev-env degrade SKIP
            // (对齐 PTX 降级纪律,非 fake-green;纯 Rust codegen 常态下不触发)。
            eprintln!("[marshalling] SKIP: build.rs 未产 saxpy.spv (dev-env degrade)");
            return;
        }

        // SPIR-V 字节 → u32 字流(小端;RXS-0203 words_to_bytes 逆)。
        assert_eq!(SAXPY_SPV.len() % 4, 0, "SPIR-V 字节须 4 字节对齐");
        let words: Vec<u32> = SAXPY_SPV
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        assert!(words.len() > 5, "SPIR-V 应含 header + 指令");
        assert_eq!(words[0], 0x0723_0203, "SPIR-V magic");

        // SPIR-V 枚举常量(与 vulkan_codegen 同源;此处**只解析真产物**,不复刻绑定规则)。
        const OP_DECORATE: u16 = 71;
        const OP_MEMBER_DECORATE: u16 = 72;
        const DEC_BLOCK: u32 = 2; // push-constant 块(区别于 SSBO 的 BufferBlock=3)
        const DEC_BINDING: u32 = 33;
        const DEC_OFFSET: u32 = 35;

        let mut bindings: Vec<u32> = Vec::new(); // buffer var 的 Binding 装饰值
        let mut block_structs: Vec<u32> = Vec::new(); // push-constant Block 结构 id
        let mut member_offsets: Vec<(u32, u32, u32)> = Vec::new(); // (struct, member, offset)

        // 指令流迭代(word = (wordCount<<16)|opcode;跳 5-word header)。
        let mut i = 5usize;
        while i < words.len() {
            let wc = (words[i] >> 16) as usize;
            let op = (words[i] & 0xffff) as u16;
            if wc == 0 {
                break;
            }
            let end = (i + wc).min(words.len());
            let ops = &words[i + 1..end];
            match op {
                OP_DECORATE if ops.len() >= 3 && ops[1] == DEC_BINDING => bindings.push(ops[2]),
                OP_DECORATE if ops.len() >= 2 && ops[1] == DEC_BLOCK => block_structs.push(ops[0]),
                OP_MEMBER_DECORATE if ops.len() >= 4 && ops[2] == DEC_OFFSET => {
                    member_offsets.push((ops[0], ops[1], ops[3]));
                }
                _ => {}
            }
            i += wc;
        }

        // ── 断言 1:buffer binding 序位 = [0,1,..,N-1](连续,从 0)。 ──
        bindings.sort_unstable();
        let n_buffers = bindings.len();
        assert!(
            n_buffers >= 2,
            "saxpy 应有多 StorageBuffer(out/x/y),实测 {n_buffers}"
        );
        // 运行时 descriptor-binding 构造(vk.rs run_compute:每 buffer i → (set 0, binding i))
        // 重建其序位;codegen 真产物 binding 序须与之逐一相等(单一事实源)。
        let runtime_bindings: Vec<u32> = (0..n_buffers as u32).collect();
        assert_eq!(
            bindings, runtime_bindings,
            "codegen (set,binding) 序须 = 运行时 descriptor-binding 构造序 [0..N)"
        );

        // ── 断言 2:push-constant 成员 Offset 序位 = [0,4,8,..](标量顺排 4 字节)。 ──
        assert_eq!(block_structs.len(), 1, "saxpy 单 push-constant 块");
        let pc = block_structs[0];
        let mut offsets: Vec<(u32, u32)> = member_offsets
            .iter()
            .filter(|(s, _, _)| *s == pc)
            .map(|(_, m, off)| (*m, *off))
            .collect();
        offsets.sort_unstable();
        let n_scalars = offsets.len();
        assert!(n_scalars >= 1, "saxpy 应有标量形参(a/n)");
        // 运行时 push_constants 布局 = 单块,标量按序 4 字节顺排(vk.rs vkCmdPushConstants
        // offset 0);codegen 真产物 member offset 序须与之相等(单一事实源)。
        let runtime_offsets: Vec<(u32, u32)> = (0..n_scalars as u32).map(|m| (m, m * 4)).collect();
        assert_eq!(
            offsets, runtime_offsets,
            "codegen push-constant offset 序须 = 运行时顺排 4 字节布局 [0,4,8,..]"
        );
    }

    //@ spec: RXS-0211
    #[test]
    fn loader_seam_selects_platform_lib() {
        // OS 加载缝库名 per-OS 唯一(cfg 选择正确);不触设备,纯 host。
        let expected = if cfg!(windows) {
            "vulkan-1.dll"
        } else {
            "libvulkan.so"
        };
        assert_eq!(loader::VULKAN_LIB.to_str().unwrap(), expected);
        // 平台无关的 entry-name 编排(桌面/Android 共用同一 .spv 消费路径)在两 OS 一致。
        let spv = [0x0723_0203u32, 0x0001_0000, 0, 5, 0];
        assert_eq!(entry_point_name(&spv), None); // 无 OpEntryPoint → None,确定性。
    }

    /// 某 crate 名是否作为**依赖声明键**出现于 manifest 文本。捕获两形态:
    /// ① **内联行** `name = ..` / `name.workspace` / `name = {..}`(行首去空白后 `name` 紧跟
    ///    ` `/`=`/`.`/tab);
    /// ② **TOML 表头** `[<..>dependencies.name]`(`[dependencies.name]` / `[build-dependencies.name]`
    ///    / `[dev-dependencies.name]` / `[target.'cfg(..)'.dependencies.name]`——末段 dotted 路径
    ///    == `name` 且其前一段以 `dependencies` 结尾)。
    /// 注释行(`#` 起)不匹配;子串不误判(`ashley` ≠ `ash`;`[dependencies.ashley]` ≠ `ash`),
    /// 故 doc-comment 内提及 `ash`/`spirv` 不会误判。(FIX 1:补 ② 表头形态,堵外部绑定 crate
    /// 经 `[dependencies.NAME]` 逃逸 tripwire 的缺口。)
    fn declares_dep(manifest: &str, name: &str) -> bool {
        manifest.lines().any(|line| {
            let t = line.trim_start();
            // ① 内联依赖行。
            if t.strip_prefix(name).is_some_and(|rest| {
                matches!(
                    rest.chars().next(),
                    Some(' ') | Some('=') | Some('.') | Some('\t')
                )
            }) {
                return true;
            }
            // ② 依赖表头 `[<..>dependencies.name]`。
            if let Some(inner) = t.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                let suffix = format!(".{name}");
                if let Some(prefix) = inner.strip_suffix(&suffix) {
                    // 末段前须为依赖表(dependencies / build-dependencies / dev-dependencies);
                    // 排除 `[features]` / `[package.metadata.name]` 等非依赖表的同名末段。
                    return prefix
                        .rsplit('.')
                        .next()
                        .is_some_and(|seg| seg.ends_with("dependencies"));
                }
            }
            false
        })
    }

    /// RXS-0213:Vulkan 绑定供应链纪律——运行时 = 手写薄 `vulkan-1`/`libvulkan` FFI loader
    /// (本模块 U26/U27),codegen = 纯 Rust SPIR-V,**两侧零外部 Vulkan/SPIR-V 绑定 crate**。
    /// 解析**真** `rurix-rt`(CARGO_MANIFEST_DIR)+ `rurixc` 的 Cargo.toml 依赖清单断言
    /// 不含 `ash`/`vulkano`/`erupt`/`gpu-alloc`(Vulkan 绑定)与外部 SPIR-V crate,且 `vulkan`
    /// feature 为空依赖集(`vulkan = []`)——非内联复刻,直接校验真 manifest(若有人加
    /// `ash = ".."` 到 [dependencies],本测试即红)。
    //@ spec: RXS-0213
    #[test]
    fn binding_supply_chain_no_external_vulkan_crate() {
        // ── tripwire 自检:declares_dep 须同时捕获内联形态与 TOML 表头形态。 ──
        // ① 内联形态。
        assert!(declares_dep("ash = \"0.37\"", "ash"), "内联 `name = ..`");
        assert!(
            declares_dep("  vulkano.workspace = true", "vulkano"),
            "内联 `name.workspace`"
        );
        assert!(
            declares_dep("erupt = { version = \"0.23\" }", "erupt"),
            "内联 `name = {{..}}`"
        );
        // ② TOML 表头形态(FIX 1:此前被漏,外部绑定 crate 可经此逃逸 tripwire)。
        assert!(
            declares_dep("[dependencies.ash]", "ash"),
            "表头 [dependencies.NAME]"
        );
        assert!(
            declares_dep("[build-dependencies.vulkano]", "vulkano"),
            "表头 [build-dependencies.NAME]"
        );
        assert!(
            declares_dep("[dev-dependencies.erupt]", "erupt"),
            "表头 [dev-dependencies.NAME]"
        );
        assert!(
            declares_dep("[target.'cfg(unix)'.dependencies.gpu-alloc]", "gpu-alloc"),
            "表头 [target.'…'.dependencies.NAME]"
        );
        // 负例:注释 / 非依赖表 / 子串不误判。
        assert!(
            !declares_dep("# ash 是成熟 Vulkan 绑定,但本项目手写 loader 不采", "ash"),
            "注释行不匹配"
        );
        assert!(!declares_dep("[features]", "ash"), "无关表头不匹配");
        assert!(
            !declares_dep("ashley = \"1.0\"", "ash"),
            "crate 名子串不误判(ashley≠ash)"
        );
        assert!(
            !declares_dep("[dependencies.ashley]", "ash"),
            "表头 crate 名子串不误判"
        );

        let rt_manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let rt = std::fs::read_to_string(&rt_manifest_path)
            .expect("读 rurix-rt Cargo.toml(CARGO_MANIFEST_DIR)");

        // 外部 Vulkan 绑定 crate:手写薄 loader 纪律禁引入(RFC-0011 §4.12 / §9 Q-Binding 默认)。
        for dep in ["ash", "vulkano", "erupt", "gpu-alloc"] {
            assert!(
                !declares_dep(&rt, dep),
                "RXS-0213:rurix-rt 不得声明外部 Vulkan 绑定依赖 `{dep}`(手写薄 vk.rs FFI loader,零外部绑定)"
            );
        }
        // `vulkan` feature 空依赖集:开 feature 不引入任何 dep(loader 手写、无 crate 增量)。
        assert!(
            rt.contains("vulkan = []"),
            "RXS-0213:`vulkan` feature 须为空依赖集(`vulkan = []`,不引入外部绑定 dep)"
        );

        // codegen 侧 SPIR-V 自包含:rurixc(../rurixc/Cargo.toml)无外部 SPIR-V crate。
        let rurixc_manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("rurixc")
            .join("Cargo.toml");
        let rurixc = std::fs::read_to_string(&rurixc_manifest_path)
            .expect("读 rurixc Cargo.toml(../rurixc)");
        for dep in [
            "rspirv",
            "spirv-tools",
            "spirv_headers",
            "spirv-builder",
            "spirv-cross",
        ] {
            assert!(
                !declares_dep(&rurixc, dep),
                "RXS-0213:rurixc codegen 须自包含,不得声明外部 SPIR-V 绑定依赖 `{dep}`(vulkan_codegen.rs 纯 Rust emitter)"
            );
        }
    }

    /// RXS-0230(v2 descriptor 建面,G3.3 PR-S0):Vk-native set-per-class 归类纯函数——
    /// set = 类别轴(1=SRV/2=UAV/3=Sampler,set0=CBV 空占位)、binding = 类内声明序(与
    /// rurixc `infer_spirv_bindings_vk_native` 同一分配律)+ set layout 数 + pool sizes。无设备。
    //@ spec: RXS-0230
    #[test]
    fn v2_plan_descriptor_sets_set_per_class() {
        let tex = |w: u32, h: u32| GraphicsResource::Texture2D {
            width: w,
            height: h,
            data: TextureData::Rgba8(vec![vec![0u8; (w * h * 4) as usize]]),
        };
        let resources = [
            tex(4, 4),
            GraphicsResource::Sampler(SamplerDesc::default()),
            tex(2, 2),
            GraphicsResource::StorageImage {
                width: 8,
                height: 8,
                format: StorageFormat::Rgba32Float,
            },
        ];
        let plans = plan_descriptor_sets(&resources);
        // SRV→set1 类内序 0,1;Sampler→set3 序 0;UAV→set2 序 0(声明序穿插不乱类内序)。
        assert_eq!(plans, vec![(1, 0), (3, 0), (1, 1), (2, 0)]);
        // set 数 = 最大使用 set + 1(set0 CBV 空占位,类别轴不平移)。
        assert_eq!(planned_set_count(&plans), 4);
        // 仅 SRV → set0(空)+ set1 两层;资源空 → 0(入口直接委派 v1)。
        assert_eq!(planned_set_count(&plan_descriptor_sets(&[tex(4, 4)])), 2);
        assert_eq!(planned_set_count(&plan_descriptor_sets(&[])), 0);
        // pool sizes:确定性序 SAMPLED_IMAGE → STORAGE_IMAGE → SAMPLER,零计数不入。
        assert_eq!(
            plan_pool_sizes(&resources),
            vec![
                (DESCRIPTOR_TYPE_SAMPLED_IMAGE, 2),
                (DESCRIPTOR_TYPE_STORAGE_IMAGE, 1),
                (DESCRIPTOR_TYPE_SAMPLER, 1)
            ]
        );
        assert_eq!(
            plan_pool_sizes(&[GraphicsResource::Sampler(SamplerDesc::default())]),
            vec![(DESCRIPTOR_TYPE_SAMPLER, 1)]
        );
    }

    /// RXS-0230:mip 尺寸序列(逐层减半钳 1)+ 完整链层数 + staging 紧凑布局偏移
    /// (RGBA8 = 4 B/texel,RGBA-f32 = 16 B/texel;偏移恒纹素尺寸整倍)。无设备。
    //@ spec: RXS-0230
    #[test]
    fn v2_mip_extents_and_staging_layout() {
        assert_eq!(
            mip_level_extents(8, 4, 4),
            vec![(8, 4), (4, 2), (2, 1), (1, 1)]
        );
        assert_eq!(full_mip_chain_len(8, 4), 4);
        assert_eq!(full_mip_chain_len(1, 1), 1);
        assert_eq!(full_mip_chain_len(4, 4), 3);
        // RGBA8:4×4=64 B → 2×2=16 B → 1×1=4 B,偏移 [0,64,80],总 84。
        let (total, offsets) = mip_staging_layout(&[(4, 4), (2, 2), (1, 1)], 4);
        assert_eq!(offsets, vec![0, 64, 80]);
        assert_eq!(total, 84);
        // RGBA-f32 纹素 16 B:2×2=64 B → 1×1=16 B。
        let (total_f, offsets_f) = mip_staging_layout(&[(2, 2), (1, 1)], 16);
        assert_eq!(offsets_f, vec![0, 64]);
        assert_eq!(total_f, 80);
    }

    /// RXS-0230:资源合法性预校验(fail-closed,任何句柄创建前)——mip 层数超链 / 层数据
    /// 长错 / 零层 / sampler 非法域(RXS-0225)/ storage 零尺寸各拒;合法组合过。无设备。
    //@ spec: RXS-0230
    #[test]
    fn v2_validate_resources_fail_closed() {
        // 合法:4×4 两层 RGBA8 + 默认 sampler + storage。
        let good = [
            GraphicsResource::Texture2D {
                width: 4,
                height: 4,
                data: TextureData::Rgba8(vec![vec![0; 64], vec![0; 16]]),
            },
            GraphicsResource::Sampler(SamplerDesc::default()),
            GraphicsResource::StorageImage {
                width: 8,
                height: 8,
                format: StorageFormat::Rgba8Unorm,
            },
        ];
        assert!(validate_resources(&good).is_ok());
        // f32 层长按 f32 数(2×2×4 = 16 个 f32)。
        let f32_ok = [GraphicsResource::Texture2D {
            width: 2,
            height: 2,
            data: TextureData::RgbaF32(vec![vec![0.0; 16]]),
        }];
        assert!(validate_resources(&f32_ok).is_ok());
        // mip 层数超完整链(4×4 完整链 = 3)。
        let too_deep = [GraphicsResource::Texture2D {
            width: 4,
            height: 4,
            data: TextureData::Rgba8(vec![vec![0; 64], vec![0; 16], vec![0; 4], vec![0; 4]]),
        }];
        assert!(validate_resources(&too_deep).is_err());
        // 层数据长错(level 1 应 2×2×4 = 16)。
        let bad_len = [GraphicsResource::Texture2D {
            width: 4,
            height: 4,
            data: TextureData::Rgba8(vec![vec![0; 64], vec![0; 15]]),
        }];
        assert!(validate_resources(&bad_len).is_err());
        // 零 mip 层拒。
        let no_level = [GraphicsResource::Texture2D {
            width: 4,
            height: 4,
            data: TextureData::Rgba8(vec![]),
        }];
        assert!(validate_resources(&no_level).is_err());
        // sampler lod_bias 越域拒(RXS-0225 合法域 [-16,16))。
        let bad_sampler = [GraphicsResource::Sampler(SamplerDesc {
            lod_bias: 32.0,
            ..SamplerDesc::default()
        })];
        assert!(validate_resources(&bad_sampler).is_err());
        // storage 零尺寸拒。
        let bad_storage = [GraphicsResource::StorageImage {
            width: 0,
            height: 8,
            format: StorageFormat::Rgba32Float,
        }];
        assert!(validate_resources(&bad_storage).is_err());
    }

    /// RXS-0230 L3 + RXS-0225:`SamplerDesc`(经 `vk_fields` plain 降级)→
    /// `VkSamplerCreateInfo` 全字段构造断言(immutable sampler 用;UVW 同值 / maxLod 钳
    /// `VK_LOD_CLAMP_NONE` / border 预置 / 归一化坐标恒开)。无设备。
    //@ spec: RXS-0230
    #[test]
    fn v2_sampler_create_info_field_mapping() {
        use crate::sampler::{Address, Compare, Filter};
        // 默认 linear + clamp:LINEAR(1)/CLAMP_TO_EDGE(2),无 aniso 无比较。
        let sci = sampler_create_info(&SamplerDesc::default());
        assert_eq!(sci.s_type, ST_SAMPLER_CREATE_INFO);
        assert_eq!(sci.mag_filter, 1);
        assert_eq!(sci.min_filter, 1);
        assert_eq!(sci.mipmap_mode, 1);
        assert_eq!(sci.address_mode_u, 2);
        assert_eq!(sci.address_mode_v, sci.address_mode_u, "UVW 三向同值");
        assert_eq!(sci.address_mode_w, sci.address_mode_u, "UVW 三向同值");
        assert_eq!(sci.anisotropy_enable, 0);
        assert_eq!(sci.compare_enable, 0);
        assert_eq!(
            sci.max_lod, LOD_CLAMP_NONE,
            "宿主 f32::MAX 钳 VK_LOD_CLAMP_NONE"
        );
        assert_eq!(sci.border_color, BORDER_COLOR_FLOAT_OPAQUE_BLACK);
        assert_eq!(sci.unnormalized_coordinates, 0);

        // 全非默认:nearest + wrap + aniso 8 + LessEqual 比较 + 有限 LOD 域。
        let sci2 = sampler_create_info(&SamplerDesc {
            filter: Filter::Nearest,
            address: Address::Wrap,
            max_anisotropy: 8,
            lod_bias: 1.5,
            min_lod: 0.5,
            max_lod: 4.0,
            compare: Some(Compare::LessEqual),
        });
        assert_eq!(sci2.mag_filter, 0, "nearest → VK_FILTER_NEAREST");
        assert_eq!(sci2.mipmap_mode, 0);
        assert_eq!(sci2.address_mode_u, 0, "wrap → REPEAT");
        assert_eq!(sci2.mip_lod_bias, 1.5);
        assert_eq!(sci2.anisotropy_enable, 1);
        assert_eq!(sci2.max_anisotropy, 8.0);
        assert_eq!(sci2.compare_enable, 1);
        assert_eq!(sci2.compare_op, 3, "LessEqual → LESS_OR_EQUAL");
        assert_eq!(sci2.min_lod, 0.5);
        assert_eq!(sci2.max_lod, 4.0, "未超钳值不动");
    }

    /// RXS-0235(G3.4 bindless):feature chain 四 bit 全在 → Ok(本机 RTX 4070 Ti 实况的
    /// host 镜像基线)。无设备(纯函数)。
    //@ spec: RXS-0235
    #[test]
    fn bindless_feature_chain_all_present_ok() {
        assert_eq!(
            check_descriptor_indexing_bits(&DescriptorIndexingBits::all_present()),
            Ok(())
        );
    }

    /// RXS-0235(G3.4 bindless):feature chain **missing 路 mock**——四 bit 逐一缺失 →
    /// 确定性 `Err` 且消息点名缺失位;多缺失全列。本机四 bit 全在,missing 路无真设备
    /// 可跑,mock 即其唯一诚实覆盖(RXS-0193 封口,无静默降级 P-01)。
    //@ spec: RXS-0235
    #[test]
    fn bindless_feature_chain_missing_bits_err() {
        type ClearBit = fn(&mut DescriptorIndexingBits);
        let cases: [(&str, ClearBit); 4] = [
            ("shaderSampledImageArrayNonUniformIndexing", |b| {
                b.sampled_image_non_uniform = false
            }),
            ("descriptorBindingSampledImageUpdateAfterBind", |b| {
                b.sampled_image_update_after_bind = false
            }),
            ("descriptorBindingPartiallyBound", |b| {
                b.partially_bound = false
            }),
            ("runtimeDescriptorArray", |b| {
                b.runtime_descriptor_array = false
            }),
        ];
        for (name, clear) in cases {
            let mut bits = DescriptorIndexingBits::all_present();
            clear(&mut bits);
            let e = check_descriptor_indexing_bits(&bits)
                .expect_err("任一 bit 缺失应确定性 Err(无静默降级)");
            assert!(e.contains(name), "Err 应点名缺失位 {name}: {e}");
        }
        // 全缺失:四位全列(确定性拒,列全便于 owner 定位驱动面)。
        let none = DescriptorIndexingBits {
            sampled_image_non_uniform: false,
            sampled_image_update_after_bind: false,
            partially_bound: false,
            runtime_descriptor_array: false,
        };
        let e = check_descriptor_indexing_bits(&none).expect_err("全缺失应 Err");
        for name in [
            "shaderSampledImageArrayNonUniformIndexing",
            "descriptorBindingSampledImageUpdateAfterBind",
            "descriptorBindingPartiallyBound",
            "runtimeDescriptorArray",
        ] {
            assert!(e.contains(name), "全缺失 Err 应全列: {e}");
        }
    }

    /// RXS-0233/0235(G3.4 bindless):入口纯 host 预校验——空表拒 / 非纹理表元素拒
    /// (无界 Sampler 维持 Unmappable 口径)/ 有界 storage 拒(归 v2);合法组合 Ok。
    /// `VkPhysicalDeviceDescriptorIndexingFeatures` 布局锚:20 个 VkBool32 + sType/pNext 头。
    //@ spec: RXS-0233
    //@ spec: RXS-0235
    #[test]
    fn bindless_input_validation_and_ffi_layout() {
        let tex = GraphicsResource::Texture2D {
            width: 2,
            height: 2,
            data: TextureData::Rgba8(vec![vec![0u8; 16]]),
        };
        let samp = GraphicsResource::Sampler(SamplerDesc::default());
        // 合法:有界 [Sampler] + 表 [tex; 4]。
        let table = vec![tex.clone(), tex.clone(), tex.clone(), tex.clone()];
        assert!(validate_bindless_inputs(std::slice::from_ref(&samp), &table).is_ok());
        // 空表拒。
        assert!(validate_bindless_inputs(&[], &[]).is_err());
        // 表元素非纹理拒(无界 Sampler/CBV/UAV 维持 Unmappable,RXS-0233)。
        assert!(validate_bindless_inputs(&[], std::slice::from_ref(&samp)).is_err());
        // 有界 storage 拒(bindless 入口首期面窄;storage 归 v2)。
        let sto = GraphicsResource::StorageImage {
            width: 4,
            height: 4,
            format: StorageFormat::Rgba8Unorm,
        };
        assert!(validate_bindless_inputs(std::slice::from_ref(&sto), &table).is_err());
        // FFI 布局锚:sType(u32)+ pad + pNext(ptr)+ 20×VkBool32(x64:8+8+80=96)。
        assert_eq!(
            std::mem::size_of::<PhysicalDeviceDescriptorIndexingFeatures>(),
            std::mem::size_of::<*const c_void>() * 2 + DESCRIPTOR_INDEXING_FEATURE_COUNT * 4,
            "VkPhysicalDeviceDescriptorIndexingFeatures 布局(头 16 + 20×4)"
        );
        // properties blob ≥ 真实 VkPhysicalDeviceProperties(x64 约 824 B)严格超集 + 8 对齐。
        assert!(std::mem::size_of::<PhysicalDevicePropertiesBlob>() >= 1024);
        assert_eq!(std::mem::align_of::<PhysicalDevicePropertiesBlob>(), 8);
    }
}
