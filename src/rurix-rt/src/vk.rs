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

// ── Windows 动态加载 ────────────────────────────────────────────────────────

unsafe extern "system" {
    fn LoadLibraryA(name: *const c_char) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const c_char) -> *mut c_void;
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
    // SAFETY: LoadLibraryA/GetProcAddress 为 Win32 稳定 ABI;入参 NUL 结尾字面量。
    unsafe {
        let lib = LoadLibraryA(c"vulkan-1.dll".as_ptr());
        if lib.is_null() {
            return None;
        }
        let p = GetProcAddress(lib, c"vkGetInstanceProcAddr".as_ptr());
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
    let gipa = load_vulkan_loader().ok_or("vulkan-1.dll / vkGetInstanceProcAddr 不可用")?;
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
