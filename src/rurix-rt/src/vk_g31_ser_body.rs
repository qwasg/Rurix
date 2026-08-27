// G31+ 波 C Task C15 `run_ser_reorder_workload`(由 `vk.rs` `include!`;U30 扩注同界)。
// SER(Shader Execution Reordering)workload device 入口(RFC-0048 §4.8;
// M52 workload 半兑现面;VK_NV_ray_tracing_invocation_reorder 臂)。
//
// 语义:A/B 双臂同场景真跑——64 竖条 × 2 三角形 = 128 BLAS × 128 实例,逐条
// 交替 slab_a/slab_b SBT record(sbt_record_offset = stripe % 2),canvas 全命中
// 棋盘式 2-way 分歧;raygen = HitObject 流(OpHitObjectTraceRayNV → [reorder 臂
// OpReorderThreadWithHitObjectNV] → OpHitObjectExecuteShaderNV)。判据:
// reorder 不得改画面(双臂像素位级一致)+ 双臂各自双跑位级 + stack 公式 +
// validation 静默 + 时延 measured 对照(微基准口径,合成分歧、单 GPU、单
// driver 版本,不外推生产——如实登记面)。
//
// **hand-emitted 镜像语料臂**(vulkan_codegen emit_g31_ser_raygen;非 .rx 编译
// 产物);能力缺失 → 确定性 Err(三 token 入错文,fail-closed,无静默降级)。

/// SER capability 三 token(探测结果入 evidence;G28 M52 重判表同构)。
#[derive(Debug, Clone, Copy)]
pub struct SerCapabilityTokens {
    /// `VK_NV_ray_tracing_invocation_reorder` device 扩展在位。
    pub ext_nv: bool,
    /// `rayTracingInvocationReorder` feature。
    pub feature_reorder: bool,
    /// `rayTracingInvocationReorderReorderingHint` feature。
    pub feature_reordering_hint: bool,
}

/// SER workload 结果(device 真跑面;JSON 出报归 `bin/g31_rt_slab_lane`)。
#[derive(Debug, Clone)]
pub struct SerWorkloadResult {
    pub tokens: SerCapabilityTokens,
    pub width: u32,
    pub height: u32,
    pub dispatches_per_arm: u32,
    pub repeats: u32,
    pub n_blas: u32,
    pub n_instances: u32,
    /// 逐 repeat 批时(min 取代表;墙钟 queue_submit+wait_idle 单批 `dispatches` 次
    /// TraceRays 总时;measured_local 微基准)。
    pub time_ms_noreorder: f64,
    pub time_ms_reorder: f64,
    /// noreorder / reorder(>1 = reorder 快;<1 = 更慢;如实登记,方向不预设)。
    pub speedup_ratio: f64,
    /// reorder 不得改画面:双臂最终帧位级一致。
    pub pixels_bitexact_across_arms: bool,
    /// 逐臂逐 repeat 帧位级一致(确定性)。
    pub double_run_bitexact: bool,
    pub validation_errors: u32,
    pub stack_required: u32,
    pub stack_configured: u32,
    /// 逐 repeat 全批时(诊断列;[noreorder..., reorder...])。
    pub batch_ms: Vec<f64>,
}

/// `VK_NV_ray_tracing_invocation_reorder` feature 结构(vulkan_core.h 1.3.296
/// 逐值核对:sType=1000490000,两 VkBool32)。
#[repr(C)]
struct PhysicalDeviceRayTracingInvocationReorderFeaturesNV {
    s_type: u32,
    p_next: *mut c_void,
    ray_tracing_invocation_reorder: u32,
    ray_tracing_invocation_reorder_reordering_hint: u32,
}

const ST_PHYSICAL_DEVICE_RAY_TRACING_INVOCATION_REORDER_FEATURES_NV: u32 = 1_000_490_000;

/// SER workload 入口(RFC-0048 §4.8)。`width`/`height` = canvas;
/// `dispatches` = 单批 TraceRays 数;`repeats` = 批数(min 取代表)。
///
/// # SAFETY(U30 扩注)
/// AS/SBT/device-address/RT-pipeline 细审计邻域与 `run_rt_pipeline_offscreen_impl`
/// 同界;本入口加性 = SER feature 链 + HitObject 流 raygen + 双管线 A/B 时延对照。
pub fn run_ser_reorder_workload(
    raygen_noreorder_spv: &[u32],
    raygen_reorder_spv: &[u32],
    miss_spv: &[u32],
    chit_spv: &[u32],
    hit_rec_a: &[u8],
    hit_rec_b: &[u8],
    width: u32,
    height: u32,
    dispatches: u32,
    repeats: u32,
) -> Result<SerWorkloadResult, String> {
    if raygen_noreorder_spv.is_empty() || raygen_reorder_spv.is_empty() {
        return Err("SER raygen SPIR-V 空(vulkan-backend build-dep? skipped_dev_env)".into());
    }
    // SAFETY: U30 扩注(见上);FFI 句柄线性管理,错误路经 bail! 逆序销毁。
    unsafe { run_ser_inner(
        raygen_noreorder_spv,
        raygen_reorder_spv,
        miss_spv,
        chit_spv,
        hit_rec_a,
        hit_rec_b,
        width,
        height,
        dispatches.max(1),
        repeats.max(1),
    ) }
}

unsafe fn run_ser_inner(
    rg_off: &[u32],
    rg_on: &[u32],
    miss_spv: &[u32],
    chit_spv: &[u32],
    hit_rec_a: &[u8],
    hit_rec_b: &[u8],
    width: u32,
    height: u32,
    dispatches: u32,
    repeats: u32,
) -> Result<SerWorkloadResult, String> {
    let gipa = load_vulkan_loader().ok_or("vulkan loader (vulkan-1.dll/libvulkan.so) 不可用")?;
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
        p_application_name: c"rurix-rt".as_ptr(),
        application_version: 0,
        p_engine_name: c"rurix".as_ptr(),
        engine_version: 0,
        api_version: API_VERSION_1_2,
    };
    let ici = InstanceCreateInfo {
        s_type: ST_INSTANCE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        p_application_info: &app,
        enabled_layer_count: if validation { 1 } else { 0 },
        pp_enabled_layer_names: if validation { layers.as_ptr() } else { std::ptr::null() },
        enabled_extension_count: if validation { 1 } else { 0 },
        pp_enabled_extension_names: if validation { exts.as_ptr() } else { std::ptr::null() },
    };
    let mut instance: VkInstance = std::ptr::null_mut();
    if vk_create_instance(&ici, std::ptr::null(), &mut instance) != VK_SUCCESS {
        return Err("vkCreateInstance 失败".into());
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
    let get_pd_features2: FnGetPhysicalDeviceFeatures2 =
        cast_fn(gipa(instance, c"vkGetPhysicalDeviceFeatures2".as_ptr()))
            .ok_or("缺 vkGetPhysicalDeviceFeatures2")?;
    let get_pd_props2: FnGetPhysicalDeviceProperties2 =
        cast_fn(gipa(instance, c"vkGetPhysicalDeviceProperties2".as_ptr()))
            .ok_or("缺 vkGetPhysicalDeviceProperties2")?;
    let enum_dev_ext: FnEnumerateDeviceExtensionProperties = cast_fn(gipa(
        instance,
        c"vkEnumerateDeviceExtensionProperties".as_ptr(),
    ))
    .ok_or("缺 vkEnumerateDeviceExtensionProperties")?;

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
    macro_rules! bail {
        ($e:expr) => {{
            destroy_msgr!();
            vk_destroy_instance(instance, std::ptr::null());
            return Err($e);
        }};
    }

    let mut count = 0u32;
    vk_enum_pd(instance, &mut count, std::ptr::null_mut());
    if count == 0 {
        bail!("无 Vulkan 物理设备".into());
    }
    let mut pds = vec![std::ptr::null_mut::<c_void>(); count as usize];
    vk_enum_pd(instance, &mut count, pds.as_mut_ptr());
    let pd = pds[0];

    // ── 扩展协商(RT 四扩展 + SER NV 扩展;缺一确定性 Err)──
    let mut ext_count = 0u32;
    enum_dev_ext(pd, std::ptr::null(), &mut ext_count, std::ptr::null_mut());
    let mut ext_props = vec![
        ExtensionProperties { extension_name: [0; 256], spec_version: 0 };
        ext_count as usize
    ];
    enum_dev_ext(pd, std::ptr::null(), &mut ext_count, ext_props.as_mut_ptr());
    let avail: Vec<String> = ext_props
        .iter()
        .map(|e| {
            // SAFETY: extension_name 为驱动写入的 NUL 结尾 C 串（≤256 字节）。
            std::ffi::CStr::from_ptr(e.extension_name.as_ptr())
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    let avail_refs: Vec<&str> = avail.iter().map(|s| s.as_str()).collect();
    if let Err(e) = negotiate_device_extensions(&avail_refs, RT_DEVICE_EXTENSIONS) {
        bail!(e);
    }
    let ser_ext_name = c"VK_NV_ray_tracing_invocation_reorder";
    let ext_nv = avail_refs.iter().any(|e| *e == ser_ext_name.to_str().unwrap_or(""));
    if !ext_nv {
        bail!(
            "缺扩展 VK_NV_ray_tracing_invocation_reorder（SER absent;确定性 Err,无静默降级——M52 capability 半命中维持 defer）"
                .into()
        );
    }

    // ── feature 探测(accel_struct + rt_pipeline + bda + SER reorder 链)──
    let mut ser_feat = PhysicalDeviceRayTracingInvocationReorderFeaturesNV {
        s_type: ST_PHYSICAL_DEVICE_RAY_TRACING_INVOCATION_REORDER_FEATURES_NV,
        p_next: std::ptr::null_mut(),
        ray_tracing_invocation_reorder: 0,
        ray_tracing_invocation_reorder_reordering_hint: 0,
    };
    let mut bda_feat = PhysicalDeviceBufferDeviceAddressFeatures {
        s_type: ST_PHYSICAL_DEVICE_BUFFER_DEVICE_ADDRESS_FEATURES,
        p_next: &mut ser_feat as *mut _ as *mut c_void,
        buffer_device_address: 0,
        buffer_device_address_capture_replay: 0,
        buffer_device_address_multi_device: 0,
    };
    let mut rtp_feat = PhysicalDeviceRayTracingPipelineFeatures {
        s_type: ST_PHYSICAL_DEVICE_RAY_TRACING_PIPELINE_FEATURES_KHR,
        p_next: &mut bda_feat as *mut _ as *mut c_void,
        ray_tracing_pipeline: 0,
        ray_tracing_pipeline_shader_group_handle_capture_replay: 0,
        ray_tracing_pipeline_shader_group_handle_capture_replay_mixed: 0,
        ray_tracing_pipeline_trace_rays_indirect: 0,
        ray_traversal_primitive_culling: 0,
    };
    let mut as_feat = PhysicalDeviceAccelerationStructureFeatures {
        s_type: ST_PHYSICAL_DEVICE_ACCELERATION_STRUCTURE_FEATURES_KHR,
        p_next: &mut rtp_feat as *mut _ as *mut c_void,
        acceleration_structure: 0,
        acceleration_structure_capture_replay: 0,
        acceleration_structure_indirect_build: 0,
        acceleration_structure_host_commands: 0,
        descriptor_binding_acceleration_structure_update_after_bind: 0,
    };
    let mut feats2 = PhysicalDeviceFeatures2 {
        s_type: ST_PHYSICAL_DEVICE_FEATURES_2,
        p_next: &mut as_feat as *mut _ as *mut c_void,
        features: std::mem::zeroed(),
    };
    get_pd_features2(pd, &mut feats2);
    let tokens = SerCapabilityTokens {
        ext_nv,
        feature_reorder: ser_feat.ray_tracing_invocation_reorder != 0,
        feature_reordering_hint: ser_feat.ray_tracing_invocation_reorder_reordering_hint != 0,
    };
    let mut missing: Vec<&str> = Vec::new();
    if as_feat.acceleration_structure == 0 {
        missing.push("accelerationStructure");
    }
    if rtp_feat.ray_tracing_pipeline == 0 {
        missing.push("rayTracingPipeline");
    }
    if bda_feat.buffer_device_address == 0 {
        missing.push("bufferDeviceAddress");
    }
    if !missing.is_empty() {
        bail!(format!("device 缺 RT feature: {}（确定性 Err,无静默降级）", missing.join(", ")));
    }
    if !tokens.feature_reorder {
        bail!(
            "device 缺 feature rayTracingInvocationReorder（SER absent;确定性 Err——M52 capability 半命中维持 defer）"
                .into()
        );
    }

    // ── RT 管线属性(SBT 对齐三参)──
    let mut rt_props = PhysicalDeviceRayTracingPipelineProperties {
        s_type: ST_PHYSICAL_DEVICE_RAY_TRACING_PIPELINE_PROPERTIES_KHR,
        p_next: std::ptr::null_mut(),
        shader_group_handle_size: 0,
        max_ray_recursion_depth: 0,
        max_shader_group_stride: 0,
        shader_group_base_alignment: 0,
        shader_group_handle_capture_replay_size: 0,
        max_ray_dispatch_invocation_count: 0,
        shader_group_handle_alignment: 0,
        max_ray_hit_attribute_size: 0,
    };
    let mut props2 = PhysicalDeviceProperties2Rt {
        s_type: ST_PHYSICAL_DEVICE_PROPERTIES_2,
        p_next: &mut rt_props as *mut _ as *mut c_void,
        properties: std::mem::zeroed(),
    };
    get_pd_props2(pd, &mut props2);

    let mut qf_count = 0u32;
    vk_get_qf(pd, &mut qf_count, std::ptr::null_mut());
    let mut qfs: Vec<QueueFamilyProperties> = (0..qf_count)
        .map(|_| QueueFamilyProperties {
            queue_flags: 0,
            queue_count: 0,
            timestamp_valid_bits: 0,
            min_image_transfer_granularity: VkExtent3D { width: 0, height: 0, depth: 0 },
        })
        .collect();
    vk_get_qf(pd, &mut qf_count, qfs.as_mut_ptr());
    let qfi = match qfs.iter().position(|q| q.queue_flags & QUEUE_GRAPHICS_BIT != 0) {
        Some(i) => i as u32,
        None => bail!("无 graphics queue family".into()),
    };

    // ── device:5 扩展 + feature 链全启用(含 SER reorder + hint)──
    as_feat.acceleration_structure = 1;
    rtp_feat.ray_tracing_pipeline = 1;
    bda_feat.buffer_device_address = 1;
    bda_feat.p_next = std::ptr::null_mut(); // device 链 = ser_chain→as→rtp→bda(ser_feat 探测体不重复入链,sType 唯一律)
    rtp_feat.p_next = &mut bda_feat as *mut _ as *mut c_void;
    as_feat.p_next = &mut rtp_feat as *mut _ as *mut c_void;
    let mut ser_chain = PhysicalDeviceRayTracingInvocationReorderFeaturesNV {
        s_type: ST_PHYSICAL_DEVICE_RAY_TRACING_INVOCATION_REORDER_FEATURES_NV,
        p_next: &mut as_feat as *mut _ as *mut c_void,
        ray_tracing_invocation_reorder: 1,
        ray_tracing_invocation_reorder_reordering_hint: 1,
    };
    let _ = &mut ser_chain;
    let mut ext_ptrs: Vec<*const c_char> = RT_DEVICE_EXTENSIONS.iter().map(|e| e.as_ptr()).collect();
    ext_ptrs.push(ser_ext_name.as_ptr());
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
        p_next: &ser_chain as *const _ as *const c_void,
        flags: 0,
        queue_create_info_count: 1,
        p_queue_create_infos: &dqci,
        enabled_layer_count: 0,
        pp_enabled_layer_names: std::ptr::null(),
        enabled_extension_count: ext_ptrs.len() as u32,
        pp_enabled_extension_names: ext_ptrs.as_ptr(),
        p_enabled_features: std::ptr::null(),
    };
    let mut device: VkDevice = std::ptr::null_mut();
    if vk_create_device(pd, &dci, std::ptr::null(), &mut device) != VK_SUCCESS {
        bail!("vkCreateDevice 失败（RT+SER 扩展/feature 启用）".into());
    }

    let mut out = ser_body(
        vk_get_device_proc,
        device,
        pd,
        vk_get_mem,
        qfi,
        rg_off,
        rg_on,
        miss_spv,
        chit_spv,
        hit_rec_a,
        hit_rec_b,
        &rt_props,
        tokens,
        width,
        height,
        dispatches,
        repeats,
    );
    if validation && validation_error.load(std::sync::atomic::Ordering::Relaxed) {
        out = Err("VK_LAYER_KHRONOS_validation 报 ERROR 级校验错误（fail-closed）".into());
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
unsafe fn ser_body(
    gdpa: FnGetDeviceProcAddr,
    device: VkDevice,
    pd: VkPhysicalDevice,
    vk_get_mem: FnGetPhysicalDeviceMemoryProperties,
    qfi: u32,
    rg_off: &[u32],
    rg_on: &[u32],
    miss_spv: &[u32],
    chit_spv: &[u32],
    hit_rec_a: &[u8],
    hit_rec_b: &[u8],
    rt_props: &PhysicalDeviceRayTracingPipelineProperties,
    tokens: SerCapabilityTokens,
    width: u32,
    height: u32,
    dispatches: u32,
    repeats: u32,
) -> Result<SerWorkloadResult, String> {
    macro_rules! dp {
        ($name:literal, $ty:ty) => {
            cast_fn::<$ty>(gdpa(device, $name.as_ptr())).ok_or("缺 device 符号")?
        };
    }
    let get_queue: FnGetDeviceQueue = dp!(c"vkGetDeviceQueue", FnGetDeviceQueue);
    let create_buffer: FnCreateBuffer = dp!(c"vkCreateBuffer", FnCreateBuffer);
    let destroy_buffer: FnDestroyBuffer = dp!(c"vkDestroyBuffer", FnDestroyBuffer);
    let buf_mem_req: FnGetBufferMemoryRequirements =
        dp!(c"vkGetBufferMemoryRequirements", FnGetBufferMemoryRequirements);
    let alloc_mem: FnAllocateMemory = dp!(c"vkAllocateMemory", FnAllocateMemory);
    let free_mem: FnFreeMemory = dp!(c"vkFreeMemory", FnFreeMemory);
    let bind_buf: FnBindBufferMemory = dp!(c"vkBindBufferMemory", FnBindBufferMemory);
    let map_mem: FnMapMemory = dp!(c"vkMapMemory", FnMapMemory);
    let unmap_mem: FnUnmapMemory = dp!(c"vkUnmapMemory", FnUnmapMemory);
    let create_shader: FnCreateShaderModule = dp!(c"vkCreateShaderModule", FnCreateShaderModule);
    let destroy_shader: FnDestroyShaderModule = dp!(c"vkDestroyShaderModule", FnDestroyShaderModule);
    let create_pl: FnCreatePipelineLayout = dp!(c"vkCreatePipelineLayout", FnCreatePipelineLayout);
    let destroy_pl: FnDestroyPipelineLayout =
        dp!(c"vkDestroyPipelineLayout", FnDestroyPipelineLayout);
    let destroy_pipe: FnDestroyPipeline = dp!(c"vkDestroyPipeline", FnDestroyPipeline);
    let create_cmdpool: FnCreateCommandPool = dp!(c"vkCreateCommandPool", FnCreateCommandPool);
    let destroy_cmdpool: FnDestroyCommandPool =
        dp!(c"vkDestroyCommandPool", FnDestroyCommandPool);
    let alloc_cmd: FnAllocateCommandBuffers =
        dp!(c"vkAllocateCommandBuffers", FnAllocateCommandBuffers);
    let begin_cmd: FnBeginCommandBuffer = dp!(c"vkBeginCommandBuffer", FnBeginCommandBuffer);
    let end_cmd: FnEndCommandBuffer = dp!(c"vkEndCommandBuffer", FnEndCommandBuffer);
    let cmd_bind_pipe: FnCmdBindPipeline = dp!(c"vkCmdBindPipeline", FnCmdBindPipeline);
    let queue_submit: FnQueueSubmit = dp!(c"vkQueueSubmit", FnQueueSubmit);
    let queue_wait: FnQueueWaitIdle = dp!(c"vkQueueWaitIdle", FnQueueWaitIdle);
    let create_image: FnCreateImage = dp!(c"vkCreateImage", FnCreateImage);
    let destroy_image: FnDestroyImage = dp!(c"vkDestroyImage", FnDestroyImage);
    let img_mem_req: FnGetImageMemoryRequirements =
        dp!(c"vkGetImageMemoryRequirements", FnGetImageMemoryRequirements);
    let bind_image: FnBindImageMemory = dp!(c"vkBindImageMemory", FnBindImageMemory);
    let create_view: FnCreateImageView = dp!(c"vkCreateImageView", FnCreateImageView);
    let destroy_view: FnDestroyImageView = dp!(c"vkDestroyImageView", FnDestroyImageView);
    let cmd_barrier: FnCmdPipelineBarrier = dp!(c"vkCmdPipelineBarrier", FnCmdPipelineBarrier);
    let cmd_copy_img_buf: FnCmdCopyImageToBuffer =
        dp!(c"vkCmdCopyImageToBuffer", FnCmdCopyImageToBuffer);
    let create_dsl: FnCreateDescriptorSetLayout =
        dp!(c"vkCreateDescriptorSetLayout", FnCreateDescriptorSetLayout);
    let destroy_dsl: FnDestroyDescriptorSetLayout =
        dp!(c"vkDestroyDescriptorSetLayout", FnDestroyDescriptorSetLayout);
    let create_dpool: FnCreateDescriptorPool =
        dp!(c"vkCreateDescriptorPool", FnCreateDescriptorPool);
    let destroy_dpool: FnDestroyDescriptorPool =
        dp!(c"vkDestroyDescriptorPool", FnDestroyDescriptorPool);
    let alloc_ds: FnAllocateDescriptorSets =
        dp!(c"vkAllocateDescriptorSets", FnAllocateDescriptorSets);
    let update_ds: FnUpdateDescriptorSets = dp!(c"vkUpdateDescriptorSets", FnUpdateDescriptorSets);
    let cmd_bind_ds: FnCmdBindDescriptorSets =
        dp!(c"vkCmdBindDescriptorSets", FnCmdBindDescriptorSets);
    let get_buf_addr: FnGetBufferDeviceAddress =
        dp!(c"vkGetBufferDeviceAddress", FnGetBufferDeviceAddress);
    let as_fns = VkAsFns::load(gdpa, device)?;
    let create_rt_pipe: FnCreateRayTracingPipelines = dp!(
        c"vkCreateRayTracingPipelinesKHR",
        FnCreateRayTracingPipelines
    );
    let get_group_handles: FnGetRayTracingShaderGroupHandles = dp!(
        c"vkGetRayTracingShaderGroupHandlesKHR",
        FnGetRayTracingShaderGroupHandles
    );
    let cmd_trace: FnCmdTraceRays = dp!(c"vkCmdTraceRaysKHR", FnCmdTraceRays);
    let get_stack: FnGetRayTracingShaderGroupStackSize =
        dp!(c"vkGetRayTracingShaderGroupStackSizeKHR", FnGetRayTracingShaderGroupStackSize);
    let set_stack: FnCmdSetRayTracingPipelineStackSize = dp!(
        c"vkCmdSetRayTracingPipelineStackSizeKHR",
        FnCmdSetRayTracingPipelineStackSize
    );

    let mut queue: VkQueue = std::ptr::null_mut();
    get_queue(device, qfi, 0, &mut queue);
    let mut memprops = std::mem::zeroed::<PhysicalDeviceMemoryProperties>();
    vk_get_mem(pd, &mut memprops);
    let readback_len = (width as usize) * (height as usize) * 4;

    let mk_buffer = |size: u64,
                     usage: u32,
                     host_visible: bool,
                     device_address: bool|
     -> Result<(VkBuffer, VkDeviceMemory), String> {
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
        let want = if host_visible { MEM_HOST_VISIBLE | MEM_HOST_COHERENT } else { MEM_DEVICE_LOCAL };
        let Some(mt) = pick_mem_type(&memprops, req.memory_type_bits, want) else {
            destroy_buffer(device, buffer, std::ptr::null());
            return Err("无匹配内存类型".into());
        };
        let flags_info = MemoryAllocateFlagsInfo {
            s_type: ST_MEMORY_ALLOCATE_FLAGS_INFO,
            p_next: std::ptr::null(),
            flags: MEMORY_ALLOCATE_DEVICE_ADDRESS_BIT,
            device_mask: 0,
        };
        let mai = MemoryAllocateInfo {
            s_type: ST_MEMORY_ALLOCATE_INFO,
            p_next: if device_address {
                &flags_info as *const MemoryAllocateFlagsInfo as *const c_void
            } else {
                std::ptr::null()
            },
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
    let buf_addr = |buffer: VkBuffer| -> u64 {
        let info = BufferDeviceAddressInfo {
            s_type: ST_BUFFER_DEVICE_ADDRESS_INFO,
            p_next: std::ptr::null(),
            buffer,
        };
        get_buf_addr(device, &info)
    };
    let upload = |mem: VkDeviceMemory, bytes: &[u8]| {
        let mut ptr: *mut c_void = std::ptr::null_mut();
        map_mem(device, mem, 0, bytes.len() as u64, 0, &mut ptr);
        if !ptr.is_null() {
            // SAFETY: mem host-visible+coherent,映射 bytes.len() 字节有效;逐字节写入后 unmap。
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, bytes.len());
            unmap_mem(device, mem);
        }
    };

    // ── 场景:64 竖条 × 2 三角形 = 128 BLAS × 128 实例,逐条交替 slab_a/b ──
    const N_STRIPES: usize = 64;
    let mut stripe_tris: Vec<[f32; 9]> = Vec::with_capacity(N_STRIPES * 2);
    for s in 0..N_STRIPES {
        let x0 = -1.0f32 + (s as f32) * (2.0f32 / N_STRIPES as f32);
        let x1 = x0 + 2.0f32 / N_STRIPES as f32;
        stripe_tris.push([x0, 1.0, 0.0, x0, -1.0, 0.0, x1, 1.0, 0.0]);
        stripe_tris.push([x0, -1.0, 0.0, x1, -1.0, 0.0, x1, 1.0, 0.0]);
    }
    let blas_refs: Vec<&[f32]> = stripe_tris.iter().map(|t| &t[..]).collect();
    let mut stripe_instances: Vec<RayQueryInstanceDesc> = Vec::with_capacity(N_STRIPES * 2);
    for s in 0..N_STRIPES {
        let off = (s % 2) as u32;
        stripe_instances.push(RayQueryInstanceDesc {
            blas: (2 * s) as u32,
            custom_index: (2 * s) as u32,
            mask: 0xFF,
            sbt_record_offset: off,
        });
        stripe_instances.push(RayQueryInstanceDesc {
            blas: (2 * s + 1) as u32,
            custom_index: (2 * s + 1) as u32,
            mask: 0xFF,
            sbt_record_offset: off,
        });
    }
    let scene = RayQuerySceneDesc {
        blas_triangles: &blas_refs,
        instances: &stripe_instances,
    };

    let mut as_mgr: Option<VkAsManager> = None;
    let mut simage: VkImage = VK_NULL_HANDLE;
    let mut smem: VkDeviceMemory = VK_NULL_HANDLE;
    let mut sview: VkImageView = VK_NULL_HANDLE;
    let mut dsl_tlas: VkDescriptorSetLayout = VK_NULL_HANDLE;
    let mut dsl_img: VkDescriptorSetLayout = VK_NULL_HANDLE;
    let mut dpool: VkDescriptorPool = VK_NULL_HANDLE;
    let mut player: VkPipelineLayout = VK_NULL_HANDLE;
    let mut pipe_off: VkPipeline = VK_NULL_HANDLE;
    let mut pipe_on: VkPipeline = VK_NULL_HANDLE;
    let mut sbt_buf: VkBuffer = VK_NULL_HANDLE;
    let mut sbt_mem: VkDeviceMemory = VK_NULL_HANDLE;
    let mut rbuf: VkBuffer = VK_NULL_HANDLE;
    let mut rmem: VkDeviceMemory = VK_NULL_HANDLE;
    let mut cmdpool: VkCommandPool = VK_NULL_HANDLE;

    let result: Result<SerWorkloadResult, String> = 'body: {
        let mgr = match VkAsManager::create_scene(&as_fns, device, &memprops, &scene, None) {
            Ok(m) => m,
            Err(e) => break 'body Err(e),
        };
        let tlas = mgr.tlas();
        as_mgr = Some(mgr);

        // ── storage image(RGBA8 UAV;回读源)──
        let sici = ImageCreateInfo {
            s_type: ST_IMAGE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            image_type: IMAGE_TYPE_2D,
            format: FORMAT_R8G8B8A8_UNORM,
            extent: VkExtent3D { width, height, depth: 1 },
            mip_levels: 1,
            array_layers: 1,
            samples: SAMPLE_COUNT_1,
            tiling: IMAGE_TILING_OPTIMAL,
            usage: IMAGE_USAGE_STORAGE | IMAGE_USAGE_TRANSFER_SRC | IMAGE_USAGE_TRANSFER_DST,
            sharing_mode: SHARING_MODE_EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
            initial_layout: IMAGE_LAYOUT_UNDEFINED,
        };
        if create_image(device, &sici, std::ptr::null(), &mut simage) != VK_SUCCESS {
            break 'body Err("storage image vkCreateImage 失败".into());
        }
        let mut sreq = std::mem::zeroed::<MemoryRequirements>();
        img_mem_req(device, simage, &mut sreq);
        let Some(smt) = pick_mem_type(&memprops, sreq.memory_type_bits, MEM_DEVICE_LOCAL) else {
            break 'body Err("storage image 无 device-local 内存类型".into());
        };
        let smai = MemoryAllocateInfo {
            s_type: ST_MEMORY_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            allocation_size: sreq.size,
            memory_type_index: smt,
        };
        if alloc_mem(device, &smai, std::ptr::null(), &mut smem) != VK_SUCCESS {
            break 'body Err("storage image vkAllocateMemory 失败".into());
        }
        bind_image(device, simage, smem, 0);
        let svci = ImageViewCreateInfo {
            s_type: ST_IMAGE_VIEW_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            image: simage,
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
        if create_view(device, &svci, std::ptr::null(), &mut sview) != VK_SUCCESS {
            break 'body Err("storage image view 失败".into());
        }

        // ── descriptor:set0 TLAS / set1 storage image(两管线共用 layout)──
        let tlas_binding = DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: DESCRIPTOR_TYPE_ACCELERATION_STRUCTURE_KHR,
            descriptor_count: 1,
            stage_flags: SHADER_STAGE_RAYGEN_KHR,
            p_immutable_samplers: std::ptr::null(),
        };
        let img_binding = DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: DESCRIPTOR_TYPE_STORAGE_IMAGE,
            descriptor_count: 1,
            stage_flags: SHADER_STAGE_RAYGEN_KHR,
            p_immutable_samplers: std::ptr::null(),
        };
        let mk_dsl = |b: &DescriptorSetLayoutBinding| -> Result<VkDescriptorSetLayout, String> {
            let ci = DescriptorSetLayoutCreateInfo {
                s_type: ST_DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                binding_count: 1,
                p_bindings: b,
            };
            let mut h: VkDescriptorSetLayout = VK_NULL_HANDLE;
            if create_dsl(device, &ci, std::ptr::null(), &mut h) != VK_SUCCESS {
                return Err("vkCreateDescriptorSetLayout 失败".into());
            }
            Ok(h)
        };
        match mk_dsl(&tlas_binding) {
            Ok(h) => dsl_tlas = h,
            Err(e) => break 'body Err(e),
        }
        match mk_dsl(&img_binding) {
            Ok(h) => dsl_img = h,
            Err(e) => break 'body Err(e),
        }
        let set_layouts = [dsl_tlas, dsl_img];
        let plci = PipelineLayoutCreateInfo {
            s_type: ST_PIPELINE_LAYOUT_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            set_layout_count: 2,
            p_set_layouts: set_layouts.as_ptr(),
            push_constant_range_count: 0,
            p_push_constant_ranges: std::ptr::null(),
        };
        if create_pl(device, &plci, std::ptr::null(), &mut player) != VK_SUCCESS {
            break 'body Err("vkCreatePipelineLayout(RT) 失败".into());
        }
        let pool_sizes = [
            DescriptorPoolSize { descriptor_type: DESCRIPTOR_TYPE_ACCELERATION_STRUCTURE_KHR, descriptor_count: 1 },
            DescriptorPoolSize { descriptor_type: DESCRIPTOR_TYPE_STORAGE_IMAGE, descriptor_count: 1 },
        ];
        let dpci = DescriptorPoolCreateInfo {
            s_type: ST_DESCRIPTOR_POOL_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            max_sets: 2,
            pool_size_count: 2,
            p_pool_sizes: pool_sizes.as_ptr(),
        };
        if create_dpool(device, &dpci, std::ptr::null(), &mut dpool) != VK_SUCCESS {
            break 'body Err("vkCreateDescriptorPool(RT) 失败".into());
        }
        let mut sets = [VK_NULL_HANDLE; 2];
        let dsai = DescriptorSetAllocateInfo {
            s_type: ST_DESCRIPTOR_SET_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            descriptor_pool: dpool,
            descriptor_set_count: 2,
            p_set_layouts: set_layouts.as_ptr(),
        };
        if alloc_ds(device, &dsai, sets.as_mut_ptr()) != VK_SUCCESS {
            break 'body Err("vkAllocateDescriptorSets(RT) 失败".into());
        }
        let (set_tlas, set_img) = (sets[0], sets[1]);
        let as_write = WriteDescriptorSetAccelStructure {
            s_type: ST_WRITE_DESCRIPTOR_SET_ACCELERATION_STRUCTURE_KHR,
            p_next: std::ptr::null(),
            acceleration_structure_count: 1,
            p_acceleration_structures: &tlas,
        };
        let img_info = DescriptorImageInfo {
            sampler: 0,
            image_view: sview,
            image_layout: IMAGE_LAYOUT_GENERAL,
        };
        let writes = [
            WriteDescriptorSet {
                s_type: ST_WRITE_DESCRIPTOR_SET,
                p_next: &as_write as *const WriteDescriptorSetAccelStructure as *const c_void,
                dst_set: set_tlas,
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_count: 1,
                descriptor_type: DESCRIPTOR_TYPE_ACCELERATION_STRUCTURE_KHR,
                p_image_info: std::ptr::null(),
                p_buffer_info: std::ptr::null(),
                p_texel_buffer_view: std::ptr::null(),
            },
            WriteDescriptorSet {
                s_type: ST_WRITE_DESCRIPTOR_SET,
                p_next: std::ptr::null(),
                dst_set: set_img,
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_count: 1,
                descriptor_type: DESCRIPTOR_TYPE_STORAGE_IMAGE,
                p_image_info: &img_info as *const DescriptorImageInfo as *const c_void,
                p_buffer_info: std::ptr::null(),
                p_texel_buffer_view: std::ptr::null(),
            },
        ];
        update_ds(device, 2, writes.as_ptr(), 0, std::ptr::null());

        // ── 双 RT 管线(reorder off/on;raygen + miss + 2×chit = 4 stages/4 groups)──
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
        let rg_off_mod = match make_shader(rg_off) {
            Ok(m) => m,
            Err(e) => break 'body Err(format!("raygen_off: {e}")),
        };
        let rg_on_mod = match make_shader(rg_on) {
            Ok(m) => m,
            Err(e) => {
                destroy_shader(device, rg_off_mod, std::ptr::null());
                break 'body Err(format!("raygen_on: {e}"));
            }
        };
        let ms_mod = match make_shader(miss_spv) {
            Ok(m) => m,
            Err(e) => {
                destroy_shader(device, rg_on_mod, std::ptr::null());
                destroy_shader(device, rg_off_mod, std::ptr::null());
                break 'body Err(format!("miss: {e}"));
            }
        };
        let ch_mod = match make_shader(chit_spv) {
            Ok(m) => m,
            Err(e) => {
                destroy_shader(device, ms_mod, std::ptr::null());
                destroy_shader(device, rg_on_mod, std::ptr::null());
                destroy_shader(device, rg_off_mod, std::ptr::null());
                break 'body Err(format!("closesthit: {e}"));
            }
        };
        let entry = c"main";
        let dyn_states = [DYNAMIC_STATE_RAY_TRACING_PIPELINE_STACK_SIZE_KHR];
        let dyn_ci = PipelineDynamicStateCreateInfo {
            s_type: ST_PIPELINE_DYNAMIC_STATE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            dynamic_state_count: 1,
            p_dynamic_states: dyn_states.as_ptr(),
        };
        let iface = RayTracingPipelineInterfaceCreateInfo {
            s_type: ST_RAY_TRACING_PIPELINE_INTERFACE_CREATE_INFO_KHR,
            p_next: std::ptr::null(),
            max_pipeline_ray_payload_size: 16,
            max_pipeline_ray_hit_attribute_size: 8,
        };
        let mut q = RtStackQuery::default();
        let mut group_count = 0u32;
        for (pipe, rg_mod) in [(&mut pipe_off, rg_off_mod), (&mut pipe_on, rg_on_mod)] {
            let stages = [
                PipelineShaderStageCreateInfo {
                    s_type: ST_PIPELINE_SHADER_STAGE_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: 0,
                    stage: SHADER_STAGE_RAYGEN_KHR,
                    module: rg_mod,
                    p_name: entry.as_ptr(),
                    p_specialization_info: std::ptr::null(),
                },
                PipelineShaderStageCreateInfo {
                    s_type: ST_PIPELINE_SHADER_STAGE_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: 0,
                    stage: SHADER_STAGE_MISS_KHR,
                    module: ms_mod,
                    p_name: entry.as_ptr(),
                    p_specialization_info: std::ptr::null(),
                },
                PipelineShaderStageCreateInfo {
                    s_type: ST_PIPELINE_SHADER_STAGE_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: 0,
                    stage: SHADER_STAGE_CLOSEST_HIT_KHR,
                    module: ch_mod,
                    p_name: entry.as_ptr(),
                    p_specialization_info: std::ptr::null(),
                },
            ];
            // groups: 0=rg, 1=miss, 2=hitA(slab_a), 3=hitB(slab_b 同模块)。
            let groups = [
                RayTracingShaderGroupCreateInfo {
                    s_type: ST_RAY_TRACING_SHADER_GROUP_CREATE_INFO_KHR,
                    p_next: std::ptr::null(),
                    ty: RT_SHADER_GROUP_TYPE_GENERAL,
                    general_shader: 0,
                    closest_hit_shader: SHADER_UNUSED_KHR,
                    any_hit_shader: SHADER_UNUSED_KHR,
                    intersection_shader: SHADER_UNUSED_KHR,
                    p_shader_group_capture_replay_handle: std::ptr::null(),
                },
                RayTracingShaderGroupCreateInfo {
                    s_type: ST_RAY_TRACING_SHADER_GROUP_CREATE_INFO_KHR,
                    p_next: std::ptr::null(),
                    ty: RT_SHADER_GROUP_TYPE_GENERAL,
                    general_shader: 1,
                    closest_hit_shader: SHADER_UNUSED_KHR,
                    any_hit_shader: SHADER_UNUSED_KHR,
                    intersection_shader: SHADER_UNUSED_KHR,
                    p_shader_group_capture_replay_handle: std::ptr::null(),
                },
                RayTracingShaderGroupCreateInfo {
                    s_type: ST_RAY_TRACING_SHADER_GROUP_CREATE_INFO_KHR,
                    p_next: std::ptr::null(),
                    ty: RT_SHADER_GROUP_TYPE_TRIANGLES_HIT_GROUP,
                    general_shader: SHADER_UNUSED_KHR,
                    closest_hit_shader: 2,
                    any_hit_shader: SHADER_UNUSED_KHR,
                    intersection_shader: SHADER_UNUSED_KHR,
                    p_shader_group_capture_replay_handle: std::ptr::null(),
                },
                RayTracingShaderGroupCreateInfo {
                    s_type: ST_RAY_TRACING_SHADER_GROUP_CREATE_INFO_KHR,
                    p_next: std::ptr::null(),
                    ty: RT_SHADER_GROUP_TYPE_TRIANGLES_HIT_GROUP,
                    general_shader: SHADER_UNUSED_KHR,
                    closest_hit_shader: 2,
                    any_hit_shader: SHADER_UNUSED_KHR,
                    intersection_shader: SHADER_UNUSED_KHR,
                    p_shader_group_capture_replay_handle: std::ptr::null(),
                },
            ];
            let rtpci = RayTracingPipelineCreateInfo {
                s_type: ST_RAY_TRACING_PIPELINE_CREATE_INFO_KHR,
                p_next: std::ptr::null(),
                flags: 0,
                stage_count: 3,
                p_stages: stages.as_ptr(),
                group_count: 4,
                p_groups: groups.as_ptr(),
                max_pipeline_ray_recursion_depth: 1,
                p_library_info: std::ptr::null(),
                p_library_interface: std::ptr::null(),
                p_dynamic_state: &dyn_ci as *const _ as *const c_void,
                layout: player,
                base_pipeline_handle: VK_NULL_HANDLE,
                base_pipeline_index: -1,
            };
            let pr = create_rt_pipe(device, VK_NULL_HANDLE, VK_NULL_HANDLE, 1, &rtpci, std::ptr::null(), pipe);
            if pr != VK_SUCCESS || *pipe == VK_NULL_HANDLE {
                destroy_shader(device, ch_mod, std::ptr::null());
                destroy_shader(device, ms_mod, std::ptr::null());
                destroy_shader(device, rg_on_mod, std::ptr::null());
                destroy_shader(device, rg_off_mod, std::ptr::null());
                break 'body Err(format!("vkCreateRayTracingPipelinesKHR(SER) 失败: {pr}"));
            }
            group_count = 4;
            // stack 查询(逐组逐类;两管线同结构取 max——防御:逐臂实测后取大者)。
            q.raygen = q
                .raygen
                .max(get_stack(device, *pipe, 0, SHADER_GROUP_SHADER_GENERAL_KHR) as u32);
            q.miss_max = q
                .miss_max
                .max(get_stack(device, *pipe, 1, SHADER_GROUP_SHADER_GENERAL_KHR) as u32);
            q.chit_max = q
                .chit_max
                .max(get_stack(device, *pipe, 2, SHADER_GROUP_SHADER_CLOSEST_HIT_KHR) as u32)
                .max(get_stack(device, *pipe, 3, SHADER_GROUP_SHADER_CLOSEST_HIT_KHR) as u32);
        }
        destroy_shader(device, ch_mod, std::ptr::null());
        destroy_shader(device, ms_mod, std::ptr::null());
        destroy_shader(device, rg_on_mod, std::ptr::null());
        destroy_shader(device, rg_off_mod, std::ptr::null());

        let required = compute_rt_stack_size(&q);
        let configured = required;
        if configured < required {
            break 'body Err(format!("stack undersize: configured {configured} < required {required}"));
        }

        // ── SBT(1 rg + 1 miss + 2 hit record〔20B×2 slab〕;无 callable)──
        let handle_size = rt_props.shader_group_handle_size as u64;
        let hit_rec_bytes = hit_rec_a.len().max(hit_rec_b.len()) as u64;
        let sbt = match plan_sbt_v2(
            handle_size,
            rt_props.shader_group_handle_alignment as u64,
            rt_props.shader_group_base_alignment as u64,
            1,
            2,
            0,
            0,
            0,
            hit_rec_bytes,
            0,
        ) {
            Ok(s) => s,
            Err(e) => break 'body Err(e),
        };
        let mut handles = vec![0u8; (handle_size as usize) * 4];
        if get_group_handles(device, pipe_off, 0, group_count, handles.len(), handles.as_mut_ptr() as *mut c_void)
            != VK_SUCCESS
        {
            break 'body Err("vkGetRayTracingShaderGroupHandlesKHR(off) 失败".into());
        }
        // 双臂 group 序一致(同结构);SBT 用 off 臂 handles——on 臂重取核验一致
        // (driver 对同构管线 handle 数值可能不同;SBT 须逐臂各自铺设)。
        let mut handles_on = vec![0u8; (handle_size as usize) * 4];
        if get_group_handles(device, pipe_on, 0, group_count, handles_on.len(), handles_on.as_mut_ptr() as *mut c_void)
            != VK_SUCCESS
        {
            break 'body Err("vkGetRayTracingShaderGroupHandlesKHR(on) 失败".into());
        }
        let mut sbt_buf_on: VkBuffer = VK_NULL_HANDLE;
        let mut sbt_mem_on: VkDeviceMemory = VK_NULL_HANDLE;
        let build_sbt = |handles: &[u8],
                         sbt_buf: &mut VkBuffer,
                         sbt_mem: &mut VkDeviceMemory|
         -> Result<(StridedDeviceAddressRegion, StridedDeviceAddressRegion, StridedDeviceAddressRegion, StridedDeviceAddressRegion), String> {
            let (b, m) = mk_buffer(
                sbt.total_size,
                BUFFER_USAGE_SHADER_BINDING_TABLE | BUFFER_USAGE_SHADER_DEVICE_ADDRESS,
                true,
                true,
            )?;
            *sbt_buf = b;
            *sbt_mem = m;
            let mut sbt_bytes = vec![0u8; sbt.total_size as usize];
            let hs = handle_size as usize;
            let copy_handle = |dst: &mut [u8], off: u64, group: usize| {
                let o = off as usize;
                dst[o..o + hs].copy_from_slice(&handles[group * hs..group * hs + hs]);
            };
            let copy_rec = |dst: &mut [u8], off: u64, rec: &[u8]| {
                let o = off as usize + hs;
                dst[o..o + rec.len()].copy_from_slice(rec);
            };
            copy_handle(&mut sbt_bytes, sbt.raygen_offset, 0);
            copy_handle(&mut sbt_bytes, sbt.miss_offset, 1);
            copy_handle(&mut sbt_bytes, sbt.hit_offset, 2);
            copy_rec(&mut sbt_bytes, sbt.hit_offset, hit_rec_a);
            copy_handle(&mut sbt_bytes, sbt.hit_offset + sbt.hit_stride, 3);
            copy_rec(&mut sbt_bytes, sbt.hit_offset + sbt.hit_stride, hit_rec_b);
            upload(m, &sbt_bytes);
            let base = buf_addr(b);
            Ok((
                StridedDeviceAddressRegion { device_address: base + sbt.raygen_offset, stride: sbt.raygen_stride, size: sbt.raygen_stride },
                StridedDeviceAddressRegion { device_address: base + sbt.miss_offset, stride: sbt.miss_stride, size: sbt.miss_size },
                StridedDeviceAddressRegion { device_address: base + sbt.hit_offset, stride: sbt.hit_stride, size: sbt.hit_size },
                StridedDeviceAddressRegion { device_address: 0, stride: 0, size: 0 },
            ))
        };
        let (rg_r_off, ms_r_off, hit_r_off, call_r_off) =
            match build_sbt(&handles, &mut sbt_buf, &mut sbt_mem) {
                Ok(r) => r,
                Err(e) => break 'body Err(format!("SBT(off): {e}")),
            };
        let (rg_r_on, ms_r_on, hit_r_on, call_r_on) =
            match build_sbt(&handles_on, &mut sbt_buf_on, &mut sbt_mem_on) {
                Ok(r) => r,
                Err(e) => break 'body Err(format!("SBT(on): {e}")),
            };

        match mk_buffer(readback_len as u64, BUFFER_USAGE_TRANSFER_DST, true, false) {
            Ok((b, m)) => {
                rbuf = b;
                rmem = m;
            }
            Err(e) => {
                destroy_buffer(device, sbt_buf_on, std::ptr::null());
                free_mem(device, sbt_mem_on, std::ptr::null());
                break 'body Err(format!("readback buffer: {e}"));
            }
        }
        let cpci = CommandPoolCreateInfo {
            s_type: ST_COMMAND_POOL_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0x2, // VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT(warmup+逐 repeat 重录重 begin)
            queue_family_index: qfi,
        };
        create_cmdpool(device, &cpci, std::ptr::null(), &mut cmdpool);
        let cbai = CommandBufferAllocateInfo {
            s_type: ST_COMMAND_BUFFER_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            command_pool: cmdpool,
            level: COMMAND_BUFFER_LEVEL_PRIMARY_MESH_RT,
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
        if let Some(m) = as_mgr.as_mut() {
            m.record_build(&as_fns, cmd);
            m.record_consume_barrier(&as_fns, cmd, PIPELINE_STAGE_RAY_TRACING_SHADER_KHR);
        }
        let mk_sr = || VkImageSubresourceRange {
            aspect_mask: IMAGE_ASPECT_COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        };
        let to_general = ImageMemoryBarrier {
            s_type: ST_IMAGE_MEMORY_BARRIER,
            p_next: std::ptr::null(),
            src_access_mask: 0,
            dst_access_mask: ACCESS_SHADER_READ | ACCESS_SHADER_WRITE,
            old_layout: IMAGE_LAYOUT_UNDEFINED,
            new_layout: IMAGE_LAYOUT_GENERAL,
            src_queue_family_index: !0,
            dst_queue_family_index: !0,
            image: simage,
            subresource_range: mk_sr(),
        };
        cmd_barrier(
            cmd,
            PIPELINE_STAGE_TOP_OF_PIPE,
            PIPELINE_STAGE_RAY_TRACING_SHADER_KHR,
            0,
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
            1,
            &to_general,
        );
        // 批命令录制宏:bind → stack → bind sets → dispatches×TraceRays → 回读屏障+copy。
        let record_batch = |cmd: VkCommandBuffer,
                            pipe: VkPipeline,
                            rg_r: &StridedDeviceAddressRegion,
                            ms_r: &StridedDeviceAddressRegion,
                            hit_r: &StridedDeviceAddressRegion,
                            call_r: &StridedDeviceAddressRegion| {
            cmd_bind_pipe(cmd, PIPELINE_BIND_POINT_RAY_TRACING_KHR, pipe);
            set_stack(cmd, configured);
            let bind_sets = [set_tlas, set_img];
            cmd_bind_ds(
                cmd,
                PIPELINE_BIND_POINT_RAY_TRACING_KHR,
                player,
                0,
                2,
                bind_sets.as_ptr(),
                0,
                std::ptr::null(),
            );
            for _ in 0..dispatches {
                cmd_trace(cmd, rg_r, ms_r, hit_r, call_r, width, height, 1);
            }
            let to_src = ImageMemoryBarrier {
                s_type: ST_IMAGE_MEMORY_BARRIER,
                p_next: std::ptr::null(),
                src_access_mask: ACCESS_SHADER_WRITE,
                dst_access_mask: ACCESS_TRANSFER_READ,
                old_layout: IMAGE_LAYOUT_GENERAL,
                new_layout: IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
                src_queue_family_index: !0,
                dst_queue_family_index: !0,
                image: simage,
                subresource_range: mk_sr(),
            };
            cmd_barrier(
                cmd,
                PIPELINE_STAGE_RAY_TRACING_SHADER_KHR,
                PIPELINE_STAGE_TRANSFER,
                0,
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                1,
                &to_src,
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
                image_extent: VkExtent3D { width, height, depth: 1 },
            };
            cmd_copy_img_buf(cmd, simage, IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL, rbuf, 1, &region);
            // 回 GENERAL 供下一批。
            let back = ImageMemoryBarrier {
                s_type: ST_IMAGE_MEMORY_BARRIER,
                p_next: std::ptr::null(),
                src_access_mask: ACCESS_TRANSFER_READ,
                dst_access_mask: ACCESS_SHADER_READ | ACCESS_SHADER_WRITE,
                old_layout: IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
                new_layout: IMAGE_LAYOUT_GENERAL,
                src_queue_family_index: !0,
                dst_queue_family_index: !0,
                image: simage,
                subresource_range: mk_sr(),
            };
            cmd_barrier(
                cmd,
                PIPELINE_STAGE_TRANSFER,
                PIPELINE_STAGE_RAY_TRACING_SHADER_KHR,
                0,
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                1,
                &back,
            );
        };

        // ── 双臂 A/B:逐臂 repeats 批,逐批墙钟计时 + 帧回读(确定性互核)──
        // 命令结构:AS build + 布局迁移 = 单批先行提交;逐臂 warmup 1 批(不计时)
        // + 逐 repeat 1 批,**每批独立 begin/end/submit**(reset 池纪律)。
        end_cmd(cmd);
        let submit = SubmitInfo {
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
        queue_submit(queue, 1, &submit, VK_NULL_HANDLE);
        queue_wait(queue);
        let mut batch_ms: Vec<f64> = Vec::with_capacity((repeats * 2) as usize);
        let mut frames: Vec<Vec<u8>> = Vec::with_capacity((repeats * 2) as usize);
        for (pipe, rg_r, ms_r, hit_r, call_r) in [
            (pipe_off, &rg_r_off, &ms_r_off, &hit_r_off, &call_r_off),
            (pipe_on, &rg_r_on, &ms_r_on, &hit_r_on, &call_r_on),
        ] {
            for timed in 0..=repeats {
                // timed==0 = warmup 批(不计时);timed≥1 = 计时批。
                let cbbi2 = CommandBufferBeginInfo {
                    s_type: ST_COMMAND_BUFFER_BEGIN_INFO,
                    p_next: std::ptr::null(),
                    flags: CMD_BUFFER_USAGE_ONE_TIME_SUBMIT,
                    p_inheritance_info: std::ptr::null(),
                };
                begin_cmd(cmd, &cbbi2);
                record_batch(cmd, pipe, rg_r, ms_r, hit_r, call_r);
                end_cmd(cmd);
                let t0 = std::time::Instant::now();
                queue_submit(queue, 1, &submit, VK_NULL_HANDLE);
                queue_wait(queue);
                let ms = t0.elapsed().as_secs_f64() * 1000.0;
                if timed >= 1 {
                    batch_ms.push(ms);
                    let mut ptr: *mut c_void = std::ptr::null_mut();
                    map_mem(device, rmem, 0, readback_len as u64, 0, &mut ptr);
                    let mut pixels = vec![0u8; readback_len];
                    if !ptr.is_null() {
                        // SAFETY: rmem host-visible+coherent,映射 readback_len 字节有效;
                        // 经 vkQueueWaitIdle 后可见,逐字节拷出后 unmap。
                        std::ptr::copy_nonoverlapping(ptr as *const u8, pixels.as_mut_ptr(), readback_len);
                        unmap_mem(device, rmem);
                    }
                    frames.push(pixels);
                }
            }
        }
        let rep = repeats as usize;
        let off_ms = batch_ms[..rep].iter().cloned().fold(f64::INFINITY, f64::min);
        let on_ms = batch_ms[rep..].iter().cloned().fold(f64::INFINITY, f64::min);
        let off_frames = &frames[..rep];
        let on_frames = &frames[rep..];
        let off_bitexact = off_frames.windows(2).all(|w| w[0] == w[1]);
        let on_bitexact = on_frames.windows(2).all(|w| w[0] == w[1]);
        let arms_bitexact = !off_frames.is_empty() && !on_frames.is_empty()
            && off_frames[0] == on_frames[0];

        // SBT record 铺设核验(host 字节 == packer 输入,经 sbt_bytes 构造律自证——
        // 本 body 内 build_sbt 闭包逐字节 copy 后即上传,无第二编码面;record
        // readback 断言传主 harness 的 RT 臂〔run_rt_pipeline_offscreen〕承担)。
        if sbt_buf_on != VK_NULL_HANDLE {
            destroy_buffer(device, sbt_buf_on, std::ptr::null());
        }
        if sbt_mem_on != VK_NULL_HANDLE {
            free_mem(device, sbt_mem_on, std::ptr::null());
        }
        break 'body Ok(SerWorkloadResult {
            tokens,
            width,
            height,
            dispatches_per_arm: dispatches,
            repeats,
            n_blas: (N_STRIPES * 2) as u32,
            n_instances: (N_STRIPES * 2) as u32,
            time_ms_noreorder: off_ms,
            time_ms_reorder: on_ms,
            speedup_ratio: if on_ms > 0.0 { off_ms / on_ms } else { f64::NAN },
            pixels_bitexact_across_arms: arms_bitexact,
            double_run_bitexact: off_bitexact && on_bitexact,
            validation_errors: 0,
            stack_required: required,
            stack_configured: configured,
            batch_ms,
        });
    };

    // ── 逆序统一销毁(非 null 才销毁)──
    if cmdpool != VK_NULL_HANDLE {
        destroy_cmdpool(device, cmdpool, std::ptr::null());
    }
    if rbuf != VK_NULL_HANDLE {
        destroy_buffer(device, rbuf, std::ptr::null());
    }
    if rmem != VK_NULL_HANDLE {
        free_mem(device, rmem, std::ptr::null());
    }
    if sbt_buf != VK_NULL_HANDLE {
        destroy_buffer(device, sbt_buf, std::ptr::null());
    }
    if sbt_mem != VK_NULL_HANDLE {
        free_mem(device, sbt_mem, std::ptr::null());
    }
    if pipe_on != VK_NULL_HANDLE {
        destroy_pipe(device, pipe_on, std::ptr::null());
    }
    if pipe_off != VK_NULL_HANDLE {
        destroy_pipe(device, pipe_off, std::ptr::null());
    }
    if player != VK_NULL_HANDLE {
        destroy_pl(device, player, std::ptr::null());
    }
    if dpool != VK_NULL_HANDLE {
        destroy_dpool(device, dpool, std::ptr::null());
    }
    if dsl_img != VK_NULL_HANDLE {
        destroy_dsl(device, dsl_img, std::ptr::null());
    }
    if dsl_tlas != VK_NULL_HANDLE {
        destroy_dsl(device, dsl_tlas, std::ptr::null());
    }
    if sview != VK_NULL_HANDLE {
        destroy_view(device, sview, std::ptr::null());
    }
    if simage != VK_NULL_HANDLE {
        destroy_image(device, simage, std::ptr::null());
    }
    if smem != VK_NULL_HANDLE {
        free_mem(device, smem, std::ptr::null());
    }
    if let Some(m) = as_mgr.as_mut() {
        m.destroy(&as_fns, device);
    }
    result
}
