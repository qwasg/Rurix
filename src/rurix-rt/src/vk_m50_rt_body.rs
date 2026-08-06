// G8.2 M50 `run_rt_pipeline_offscreen_impl`(由 `vk.rs` `include!`;U30 扩注)。
// 两三角形 BLAS × 两 hit group × SBT v2 record + callable + stack + optional library。
//
// 本文件由 `tmp_gen_m50_body.py` 自 `rt_body` 机械派生后手工接线;勿以
// `run_ray_tracing_offscreen` 充绿本门。

use crate::rt_incremental::{
    RtPipelineDesc, RtPipelineMode, RtPipelineRunResult, RtStackQuery, RT_STACK_FORMULA_VERSION,
    compute_rt_stack_size, plan_sbt_v2,
};

/// M50 增量 RT device 入口实现(RXS-0327)。
///
/// # SAFETY(U30 扩注)
/// AS/SBT/device-address 细审计邻域与 `run_ray_tracing_offscreen` 同界;本入口加性
/// 扩多 hit group / shader-record / stack dynamic state / pipeline library。
//@ spec: RXS-0327
pub(crate) fn run_rt_pipeline_offscreen_impl(
    desc: &RtPipelineDesc<'_>,
) -> Result<RtPipelineRunResult, String> {
    if desc.hit_groups.len() < 2 {
        return Err("run_rt_pipeline_offscreen: need ≥2 hit groups (M50)".into());
    }
    if desc.miss.is_empty() {
        return Err("run_rt_pipeline_offscreen: miss[] empty".into());
    }

    let gipa = load_vulkan_loader().ok_or("vulkan loader (vulkan-1.dll/libvulkan.so) 不可用")?;

    let tri_a: [f32; 9] = [-0.85, 0.85, 0.0, -0.85, -0.85, 0.0, -0.05, 0.0, 0.0];
    let tri_b: [f32; 9] = [0.05, 0.0, 0.0, 0.85, -0.85, 0.0, 0.85, 0.85, 0.0];
    let tris: [&[f32]; 2] = [&tri_a[..], &tri_b[..]];
    let instances = [
        RayQueryInstanceDesc {
            blas: 0,
            custom_index: 0,
            mask: 0xFF,
            sbt_record_offset: 0,
        },
        RayQueryInstanceDesc {
            blas: 1,
            custom_index: 1,
            mask: 0xFF,
            sbt_record_offset: 1,
        },
    ];

    let rg = desc.raygen;
    let ms = desc.miss[0];
    let ch_a = desc.hit_groups[0].closest_hit;
    let ch_b = desc.hit_groups[1].closest_hit;
    let callable = if desc.callables.is_empty() {
        // m50_raygen 含 ExecuteCallable(0);无 callable SPIR-V 时用空模块会失败——
        // harness 应传 m50_callable;缺省则 Err。
        return Err("run_rt_pipeline_offscreen: callables[] empty (m50_raygen needs ExecuteCallable)".into());
    } else {
        desc.callables[0]
    };

    let hit_recs: Vec<&[u8]> = desc.records.hit.to_vec();
    if hit_recs.len() < 2 {
        return Err("run_rt_pipeline_offscreen: need ≥2 hit records".into());
    }

    let mono = unsafe {
        run_rt_m50_two_hit(
            gipa,
            rg,
            ms,
            ch_a,
            ch_b,
            callable,
            &tris,
            &instances,
            hit_recs[0],
            hit_recs[1],
            desc.width,
            desc.height,
            desc.stack_override,
            /*library=*/ false,
        )?
    };

    if desc.mode == RtPipelineMode::LibraryLink {
        let lib = unsafe {
            run_rt_m50_two_hit(
                gipa,
                rg,
                ms,
                ch_a,
                ch_b,
                callable,
                &tris,
                &instances,
                hit_recs[0],
                hit_recs[1],
                desc.width,
                desc.height,
                desc.stack_override,
                /*library=*/ true,
            )?
        };
        if mono.pixels_rgba8 != lib.pixels_rgba8 {
            return Err("library_link ≠ monolithic pixels".into());
        }
        return Ok(RtPipelineRunResult {
            mode: "library_link",
            ..lib
        });
    }
    Ok(mono)
}

/// # SAFETY(U30)
/// 同 `run_ray_tracing_offscreen`/`run_rt_inner`。
#[allow(clippy::too_many_arguments)]
unsafe fn run_rt_m50_two_hit(
    gipa: FnGetInstanceProcAddr,
    raygen_spv: &[u32],
    miss_spv: &[u32],
    chit_a: &[u32],
    chit_b: &[u32],
    callable_spv: &[u32],
    tris: &[&[f32]],
    instances: &[RayQueryInstanceDesc],
    hit_rec_a: &[u8],
    hit_rec_b: &[u8],
    width: u32,
    height: u32,
    stack_override: Option<u32>,
    library: bool,
) -> Result<RtPipelineRunResult, String> {
    run_rt_m50_inner(
        gipa,
        raygen_spv,
        miss_spv,
        chit_a,
        chit_b,
        callable_spv,
        tris,
        instances,
        hit_rec_a,
        hit_rec_b,
        width,
        height,
        stack_override,
        library,
    )
}


unsafe fn run_rt_m50_inner(
    gipa: FnGetInstanceProcAddr,
    raygen_spv: &[u32],
    miss_spv: &[u32],
    chit_a: &[u32],
    chit_b: &[u32],
    callable_spv: &[u32],
    tris: &[&[f32]],
    instances: &[RayQueryInstanceDesc],
    hit_rec_a: &[u8],
    hit_rec_b: &[u8],
    width: u32,
    height: u32,
    stack_override: Option<u32>,
    library: bool,
) -> Result<RtPipelineRunResult, String> {
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

    // ── 扩展协商（vkEnumerateDeviceExtensionProperties → negotiate_device_extensions）──
    let mut ext_count = 0u32;
    enum_dev_ext(pd, std::ptr::null(), &mut ext_count, std::ptr::null_mut());
    let mut ext_props = vec![
        ExtensionProperties {
            extension_name: [0; 256],
            spec_version: 0,
        };
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
    if library {
        if !avail_refs.iter().any(|e| *e == "VK_KHR_pipeline_library") {
            bail!("缺扩展 VK_KHR_pipeline_library (M50 library_link)".into());
        }
    }

    // ── feature 探测（accel_struct + rt_pipeline + bda 链;缺失确定性 Err）──
    let mut bda_feat = PhysicalDeviceBufferDeviceAddressFeatures {
        s_type: ST_PHYSICAL_DEVICE_BUFFER_DEVICE_ADDRESS_FEATURES,
        p_next: std::ptr::null_mut(),
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
        bail!(format!(
            "device 缺 RT feature: {}（确定性 Err,RXS-0248/RXS-0210 L3,无静默降级）",
            missing.join(", ")
        ));
    }

    // ── RT 管线属性（SBT 对齐三参;§4.E8）──
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
        None => bail!("无 graphics queue family".into()),
    };

    // ── device：4 扩展 + feature 链全启用（accel_struct + rt_pipeline + bda）──
    as_feat.acceleration_structure = 1;
    rtp_feat.ray_tracing_pipeline = 1;
    bda_feat.buffer_device_address = 1;
    // 重挂 pNext 链（enable bit 写入后再取址）：驱动经 as_feat→rtp_feat→bda_feat 链在
    // vkCreateDevice 读取全部 enable bit（消除 unused_assignments 误报,链语义不变）。
    rtp_feat.p_next = &mut bda_feat as *mut _ as *mut c_void;
    as_feat.p_next = &mut rtp_feat as *mut _ as *mut c_void;
    let mut ext_ptrs: Vec<*const c_char> = RT_DEVICE_EXTENSIONS.iter().map(|e| e.as_ptr()).collect();
    let pipe_lib_name = c"VK_KHR_pipeline_library";
    if library {
        ext_ptrs.push(pipe_lib_name.as_ptr());
    }
    let dev_exts = ext_ptrs;
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
        p_next: &as_feat as *const _ as *const c_void,
        flags: 0,
        queue_create_info_count: 1,
        p_queue_create_infos: &dqci,
        enabled_layer_count: 0,
        pp_enabled_layer_names: std::ptr::null(),
        enabled_extension_count: dev_exts.len() as u32,
        pp_enabled_extension_names: dev_exts.as_ptr(),
        p_enabled_features: std::ptr::null(),
    };
    let mut device: VkDevice = std::ptr::null_mut();
    if vk_create_device(pd, &dci, std::ptr::null(), &mut device) != VK_SUCCESS {
        bail!("vkCreateDevice 失败（RT 扩展/feature 启用）".into());
    }

    let mut out = rt_body_m50(
        vk_get_device_proc,
        device,
        pd,
        vk_get_mem,
        qfi,
        raygen_spv,
        miss_spv,
        chit_a,
        chit_b,
        callable_spv,
        tris,
        instances,
        hit_rec_a,
        hit_rec_b,
        &rt_props,
        width,
        height,
        stack_override,
        library,
    );
    if validation && validation_error.load(std::sync::atomic::Ordering::Relaxed) {
        out = Err("VK_LAYER_KHRONOS_validation 报 ERROR 级校验错误（fail-closed,L3）".into());
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

unsafe fn rt_body_m50(
    gdpa: FnGetDeviceProcAddr,
    device: VkDevice,
    pd: VkPhysicalDevice,
    vk_get_mem: FnGetPhysicalDeviceMemoryProperties,
    qfi: u32,
    raygen_spv: &[u32],
    miss_spv: &[u32],
    chit_a: &[u32],
    chit_b: &[u32],
    callable_spv: &[u32],
    tris: &[&[f32]],
    instances: &[RayQueryInstanceDesc],
    hit_rec_a: &[u8],
    hit_rec_b: &[u8],
    rt_props: &PhysicalDeviceRayTracingPipelineProperties,
    width: u32,
    height: u32,
    stack_override: Option<u32>,
    library: bool,
) -> Result<RtPipelineRunResult, String> {
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
    let create_image: FnCreateImage = dp!(c"vkCreateImage", FnCreateImage);
    let destroy_image: FnDestroyImage = dp!(c"vkDestroyImage", FnDestroyImage);
    let img_mem_req: FnGetImageMemoryRequirements = dp!(
        c"vkGetImageMemoryRequirements",
        FnGetImageMemoryRequirements
    );
    let bind_image: FnBindImageMemory = dp!(c"vkBindImageMemory", FnBindImageMemory);
    let create_view: FnCreateImageView = dp!(c"vkCreateImageView", FnCreateImageView);
    let destroy_view: FnDestroyImageView = dp!(c"vkDestroyImageView", FnDestroyImageView);
    let cmd_barrier: FnCmdPipelineBarrier = dp!(c"vkCmdPipelineBarrier", FnCmdPipelineBarrier);
    let cmd_copy_img_buf: FnCmdCopyImageToBuffer =
        dp!(c"vkCmdCopyImageToBuffer", FnCmdCopyImageToBuffer);
    let cmd_clear: FnCmdClearColorImage = dp!(c"vkCmdClearColorImage", FnCmdClearColorImage);
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
    // RT/AS 专用符号（U30 面;AS 六函数经 VkAsFns 统一加载,单所有者消费）。
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

    let mut queue: VkQueue = std::ptr::null_mut();
    get_queue(device, qfi, 0, &mut queue);
    let mut memprops = std::mem::zeroed::<PhysicalDeviceMemoryProperties>();
    vk_get_mem(pd, &mut memprops);
    let readback_len = (width as usize) * (height as usize) * 4;

    // ── 通用 buffer helper（host-visible? + device_address?）──
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
        let want = if host_visible {
            MEM_HOST_VISIBLE | MEM_HOST_COHERENT
        } else {
            MEM_DEVICE_LOCAL
        };
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

    // 所有句柄 up-front 声明（末尾逆序统一销毁,含错误路;AS 十四句柄归单所有者）。
    let mut as_mgr: Option<VkAsManager> = None;
    let mut simage: VkImage = VK_NULL_HANDLE;
    let mut smem: VkDeviceMemory = VK_NULL_HANDLE;
    let mut sview: VkImageView = VK_NULL_HANDLE;
    let mut dsl_tlas: VkDescriptorSetLayout = VK_NULL_HANDLE;
    let mut dsl_img: VkDescriptorSetLayout = VK_NULL_HANDLE;
    let mut dpool: VkDescriptorPool = VK_NULL_HANDLE;
    let mut player: VkPipelineLayout = VK_NULL_HANDLE;
    let mut pipeline: VkPipeline = VK_NULL_HANDLE;
    let mut sbt_buf: VkBuffer = VK_NULL_HANDLE;
    let mut sbt_mem: VkDeviceMemory = VK_NULL_HANDLE;
    let mut rbuf: VkBuffer = VK_NULL_HANDLE;
    let mut rmem: VkDeviceMemory = VK_NULL_HANDLE;
    let mut cmdpool: VkCommandPool = VK_NULL_HANDLE;
    let mut lib_pipes: [VkPipeline; 2] = [VK_NULL_HANDLE, VK_NULL_HANDLE];
    let result: Result<RtPipelineRunResult, String> = 'body: {
        // ── BLAS/TLAS 全量建面（单所有者 VkAsManager,等序提取;G-G7-5 禁止第二所有者）──
        let scene = RayQuerySceneDesc {
            blas_triangles: tris,
            instances,
        };
        let mgr = match VkAsManager::create_scene(&as_fns, device, &memprops, &scene, None) {
            Ok(m) => m,
            Err(e) => {
                break 'body Err(e);
            }
        };
        let tlas = mgr.tlas();
        as_mgr = Some(mgr);

        // ── storage image（UAV;GENERAL;回读源）+ view ──
        let sici = ImageCreateInfo {
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

        // ── descriptor set-per-class：set0 TLAS(SRV) / set1 storage image(UAV)──
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
            Err(e) => {
                break 'body Err(e);
            }
        }
        match mk_dsl(&img_binding) {
            Ok(h) => dsl_img = h,
            Err(e) => {
                break 'body Err(e);
            }
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
            DescriptorPoolSize {
                descriptor_type: DESCRIPTOR_TYPE_ACCELERATION_STRUCTURE_KHR,
                descriptor_count: 1,
            },
            DescriptorPoolSize {
                descriptor_type: DESCRIPTOR_TYPE_STORAGE_IMAGE,
                descriptor_count: 1,
            },
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
        // 写 TLAS descriptor（AS write 经 pNext 链;p_image/buffer 忽略）。
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

        // ── RT 管线 M50：rg + miss + 2×chit + callable；groups=5 ──
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
        let rg_mod = match make_shader(raygen_spv) {
            Ok(m) => m,
            Err(e) => break 'body Err(format!("raygen: {e}")),
        };
        let ms_mod = match make_shader(miss_spv) {
            Ok(m) => m,
            Err(e) => {
                destroy_shader(device, rg_mod, std::ptr::null());
                break 'body Err(format!("miss: {e}"));
            }
        };
        let ch_a_mod = match make_shader(chit_a) {
            Ok(m) => m,
            Err(e) => {
                destroy_shader(device, ms_mod, std::ptr::null());
                destroy_shader(device, rg_mod, std::ptr::null());
                break 'body Err(format!("closesthit_a: {e}"));
            }
        };
        let ch_b_mod = match make_shader(chit_b) {
            Ok(m) => m,
            Err(e) => {
                destroy_shader(device, ch_a_mod, std::ptr::null());
                destroy_shader(device, ms_mod, std::ptr::null());
                destroy_shader(device, rg_mod, std::ptr::null());
                break 'body Err(format!("closesthit_b: {e}"));
            }
        };
        let call_mod = match make_shader(callable_spv) {
            Ok(m) => m,
            Err(e) => {
                destroy_shader(device, ch_b_mod, std::ptr::null());
                destroy_shader(device, ch_a_mod, std::ptr::null());
                destroy_shader(device, ms_mod, std::ptr::null());
                destroy_shader(device, rg_mod, std::ptr::null());
                break 'body Err(format!("callable: {e}"));
            }
        };
        let entry = c"main";
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
                module: ch_a_mod,
                p_name: entry.as_ptr(),
                p_specialization_info: std::ptr::null(),
            },
            PipelineShaderStageCreateInfo {
                s_type: ST_PIPELINE_SHADER_STAGE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                stage: SHADER_STAGE_CLOSEST_HIT_KHR,
                module: ch_b_mod,
                p_name: entry.as_ptr(),
                p_specialization_info: std::ptr::null(),
            },
            PipelineShaderStageCreateInfo {
                s_type: ST_PIPELINE_SHADER_STAGE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                stage: SHADER_STAGE_CALLABLE_KHR,
                module: call_mod,
                p_name: entry.as_ptr(),
                p_specialization_info: std::ptr::null(),
            },
        ];
        // groups: 0=rg, 1=miss, 2=hitA, 3=hitB, 4=callable
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
                closest_hit_shader: 3,
                any_hit_shader: SHADER_UNUSED_KHR,
                intersection_shader: SHADER_UNUSED_KHR,
                p_shader_group_capture_replay_handle: std::ptr::null(),
            },
            RayTracingShaderGroupCreateInfo {
                s_type: ST_RAY_TRACING_SHADER_GROUP_CREATE_INFO_KHR,
                p_next: std::ptr::null(),
                ty: RT_SHADER_GROUP_TYPE_GENERAL,
                general_shader: 4,
                closest_hit_shader: SHADER_UNUSED_KHR,
                any_hit_shader: SHADER_UNUSED_KHR,
                intersection_shader: SHADER_UNUSED_KHR,
                p_shader_group_capture_replay_handle: std::ptr::null(),
            },
        ];

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

        let mut pipeline_flags: VkFlags = 0;
        let mut p_lib_info: *const c_void = std::ptr::null();
        let mut p_iface: *const c_void = std::ptr::null();
        let mut lib_info_storage: PipelineLibraryCreateInfo = PipelineLibraryCreateInfo {
            s_type: ST_PIPELINE_LIBRARY_CREATE_INFO_KHR,
            p_next: std::ptr::null(),
            library_count: 0,
            p_libraries: std::ptr::null(),
        };

        if library {
            // 两库分链:lib0=rg/miss/callable, lib1=hitA/hitB;link 序继承组。
            let lib0_stages = [stages[0], stages[1], stages[4]];
            let lib0_groups = [
                groups[0],
                groups[1],
                RayTracingShaderGroupCreateInfo {
                    s_type: ST_RAY_TRACING_SHADER_GROUP_CREATE_INFO_KHR,
                    p_next: std::ptr::null(),
                    ty: RT_SHADER_GROUP_TYPE_GENERAL,
                    general_shader: 2, // callable = lib0 stage 2
                    closest_hit_shader: SHADER_UNUSED_KHR,
                    any_hit_shader: SHADER_UNUSED_KHR,
                    intersection_shader: SHADER_UNUSED_KHR,
                    p_shader_group_capture_replay_handle: std::ptr::null(),
                },
            ];
            let lib1_stages = [stages[2], stages[3]];
            let lib1_groups = [
                RayTracingShaderGroupCreateInfo {
                    s_type: ST_RAY_TRACING_SHADER_GROUP_CREATE_INFO_KHR,
                    p_next: std::ptr::null(),
                    ty: RT_SHADER_GROUP_TYPE_TRIANGLES_HIT_GROUP,
                    general_shader: SHADER_UNUSED_KHR,
                    closest_hit_shader: 0,
                    any_hit_shader: SHADER_UNUSED_KHR,
                    intersection_shader: SHADER_UNUSED_KHR,
                    p_shader_group_capture_replay_handle: std::ptr::null(),
                },
                RayTracingShaderGroupCreateInfo {
                    s_type: ST_RAY_TRACING_SHADER_GROUP_CREATE_INFO_KHR,
                    p_next: std::ptr::null(),
                    ty: RT_SHADER_GROUP_TYPE_TRIANGLES_HIT_GROUP,
                    general_shader: SHADER_UNUSED_KHR,
                    closest_hit_shader: 1,
                    any_hit_shader: SHADER_UNUSED_KHR,
                    intersection_shader: SHADER_UNUSED_KHR,
                    p_shader_group_capture_replay_handle: std::ptr::null(),
                },
            ];
            let mk_lib = |stage_count: u32,
                          p_stages: *const PipelineShaderStageCreateInfo,
                          group_count: u32,
                          p_groups: *const RayTracingShaderGroupCreateInfo,
                          out: &mut VkPipeline|
             -> VkResult {
                let ci = RayTracingPipelineCreateInfo {
                    s_type: ST_RAY_TRACING_PIPELINE_CREATE_INFO_KHR,
                    p_next: std::ptr::null(),
                    flags: PIPELINE_CREATE_LIBRARY_BIT_KHR,
                    stage_count,
                    p_stages,
                    group_count,
                    p_groups,
                    max_pipeline_ray_recursion_depth: 1,
                    p_library_info: std::ptr::null(),
                    p_library_interface: &iface as *const _ as *const c_void,
                    p_dynamic_state: std::ptr::null(),
                    layout: player,
                    base_pipeline_handle: VK_NULL_HANDLE,
                    base_pipeline_index: -1,
                };
                create_rt_pipe(
                    device,
                    VK_NULL_HANDLE,
                    VK_NULL_HANDLE,
                    1,
                    &ci,
                    std::ptr::null(),
                    out,
                )
            };
            let pr0 = mk_lib(
                3,
                lib0_stages.as_ptr(),
                3,
                lib0_groups.as_ptr(),
                &mut lib_pipes[0],
            );
            let pr1 = mk_lib(
                2,
                lib1_stages.as_ptr(),
                2,
                lib1_groups.as_ptr(),
                &mut lib_pipes[1],
            );
            if pr0 != VK_SUCCESS
                || pr1 != VK_SUCCESS
                || lib_pipes[0] == VK_NULL_HANDLE
                || lib_pipes[1] == VK_NULL_HANDLE
            {
                destroy_shader(device, call_mod, std::ptr::null());
                destroy_shader(device, ch_b_mod, std::ptr::null());
                destroy_shader(device, ch_a_mod, std::ptr::null());
                destroy_shader(device, ms_mod, std::ptr::null());
                destroy_shader(device, rg_mod, std::ptr::null());
                break 'body Err(format!(
                    "library create failed: {pr0}/{pr1} handles={:x}/{:x}",
                    lib_pipes[0], lib_pipes[1]
                ));
            }
            let lib_info = PipelineLibraryCreateInfo {
                s_type: ST_PIPELINE_LIBRARY_CREATE_INFO_KHR,
                p_next: std::ptr::null(),
                library_count: 2,
                p_libraries: lib_pipes.as_ptr(),
            };
            let rtpci = RayTracingPipelineCreateInfo {
                s_type: ST_RAY_TRACING_PIPELINE_CREATE_INFO_KHR,
                p_next: std::ptr::null(),
                flags: 0,
                stage_count: 0,
                p_stages: std::ptr::null(),
                group_count: 0,
                p_groups: std::ptr::null(),
                max_pipeline_ray_recursion_depth: 1,
                p_library_info: &lib_info as *const _ as *const c_void,
                p_library_interface: &iface as *const _ as *const c_void,
                p_dynamic_state: &dyn_ci as *const _ as *const c_void,
                layout: player,
                base_pipeline_handle: VK_NULL_HANDLE,
                base_pipeline_index: -1,
            };
            let pr = create_rt_pipe(
                device, VK_NULL_HANDLE, VK_NULL_HANDLE, 1, &rtpci, std::ptr::null(), &mut pipeline,
            );
            destroy_shader(device, call_mod, std::ptr::null());
            destroy_shader(device, ch_b_mod, std::ptr::null());
            destroy_shader(device, ch_a_mod, std::ptr::null());
            destroy_shader(device, ms_mod, std::ptr::null());
            destroy_shader(device, rg_mod, std::ptr::null());
            if pr != VK_SUCCESS || pipeline == VK_NULL_HANDLE {
                break 'body Err(format!(
                    "vkCreateRayTracingPipelinesKHR link 失败: {pr} pipe={pipeline:x}"
                ));
            }
            let _ = (pipeline_flags, p_lib_info, p_iface, lib_info_storage);
        } else {
            let _ = (pipeline_flags, p_lib_info, p_iface, lib_info_storage);
            let rtpci = RayTracingPipelineCreateInfo {
                s_type: ST_RAY_TRACING_PIPELINE_CREATE_INFO_KHR,
                p_next: std::ptr::null(),
                flags: 0,
                stage_count: 5,
                p_stages: stages.as_ptr(),
                group_count: 5,
                p_groups: groups.as_ptr(),
                max_pipeline_ray_recursion_depth: 1,
                p_library_info: std::ptr::null(),
                p_library_interface: std::ptr::null(),
                p_dynamic_state: &dyn_ci as *const _ as *const c_void,
                layout: player,
                base_pipeline_handle: VK_NULL_HANDLE,
                base_pipeline_index: -1,
            };
            let pr = create_rt_pipe(
                device, VK_NULL_HANDLE, VK_NULL_HANDLE, 1, &rtpci, std::ptr::null(), &mut pipeline,
            );
            destroy_shader(device, call_mod, std::ptr::null());
            destroy_shader(device, ch_b_mod, std::ptr::null());
            destroy_shader(device, ch_a_mod, std::ptr::null());
            destroy_shader(device, ms_mod, std::ptr::null());
            destroy_shader(device, rg_mod, std::ptr::null());
            if pr != VK_SUCCESS {
                break 'body Err(format!("vkCreateRayTracingPipelinesKHR 失败: {pr}"));
            }
        }

        // stack query (逐组) → 保守公式 → configured
        let get_stack: FnGetRayTracingShaderGroupStackSize = match cast_fn(gdpa(
            device,
            c"vkGetRayTracingShaderGroupStackSizeKHR".as_ptr(),
        )) {
            Some(f) => f,
            None => break 'body Err("缺 vkGetRayTracingShaderGroupStackSizeKHR".into()),
        };
        let set_stack: FnCmdSetRayTracingPipelineStackSize = match cast_fn(gdpa(
            device,
            c"vkCmdSetRayTracingPipelineStackSizeKHR".as_ptr(),
        )) {
            Some(f) => f,
            None => break 'body Err("缺 vkCmdSetRayTracingPipelineStackSizeKHR".into()),
        };
        // Linked pipeline group order for library path: lib0 groups then lib1 →
        // [rg, miss, callable, hitA, hitB] — differs from monolithic [rg,miss,hitA,hitB,callable].
        // For SBT we always use monolithic group index convention by re-querying handles
        // from the executable pipeline; group count = 5 either way.
        let group_count = 5u32;
        // 仅查询组内实际存在的 shader 类(VUID-groupShader-03609)。
        // Library 可执行管线:validation 层对 link 后 group 计数可能失步;
        // stack/handle 查询落在 LIBRARY 管线(组完整);executable 用于 TraceRays。
        let mut q = RtStackQuery::default();
        if library {
            // link 序:[lib0 rg,miss,call][lib1 hitA,hitB]
            q.raygen = get_stack(device, lib_pipes[0], 0, SHADER_GROUP_SHADER_GENERAL_KHR) as u32;
            q.miss_max = get_stack(device, lib_pipes[0], 1, SHADER_GROUP_SHADER_GENERAL_KHR) as u32;
            q.callable_max =
                get_stack(device, lib_pipes[0], 2, SHADER_GROUP_SHADER_GENERAL_KHR) as u32;
            q.chit_max = get_stack(device, lib_pipes[1], 0, SHADER_GROUP_SHADER_CLOSEST_HIT_KHR)
                .max(get_stack(
                    device,
                    lib_pipes[1],
                    1,
                    SHADER_GROUP_SHADER_CLOSEST_HIT_KHR,
                )) as u32;
        } else {
            q.raygen = get_stack(device, pipeline, 0, SHADER_GROUP_SHADER_GENERAL_KHR) as u32;
            q.miss_max = get_stack(device, pipeline, 1, SHADER_GROUP_SHADER_GENERAL_KHR) as u32;
            q.callable_max = get_stack(device, pipeline, 4, SHADER_GROUP_SHADER_GENERAL_KHR) as u32;
            q.chit_max = get_stack(device, pipeline, 2, SHADER_GROUP_SHADER_CLOSEST_HIT_KHR)
                .max(get_stack(
                    device,
                    pipeline,
                    3,
                    SHADER_GROUP_SHADER_CLOSEST_HIT_KHR,
                )) as u32;
        }
        let required = compute_rt_stack_size(&q);
        let configured = stack_override.unwrap_or(required);
        if configured < required {
            break 'body Err(format!(
                "stack undersize: configured {configured} < required {required}"
            ));
        }

        // ── SBT v2 四 region（hit×2 + callable×1 + records）──
        let handle_size = rt_props.shader_group_handle_size as u64;
        let hit_rec_bytes = hit_rec_a.len().max(hit_rec_b.len()) as u64;
        let sbt = match plan_sbt_v2(
            handle_size,
            rt_props.shader_group_handle_alignment as u64,
            rt_props.shader_group_base_alignment as u64,
            1,
            2,
            1,
            0,
            0,
            hit_rec_bytes,
            0,
        ) {
            Ok(s) => s,
            Err(e) => break 'body Err(e),
        };
        let mut handles = vec![0u8; (handle_size as usize) * (group_count as usize)];
        // Handle 必须从可执行管线取(library 管线需 pipelineLibraryGroupHandles feature)。
        if get_group_handles(
            device,
            pipeline,
            0,
            group_count,
            handles.len(),
            handles.as_mut_ptr() as *mut c_void,
        ) != VK_SUCCESS
        {
            break 'body Err("vkGetRayTracingShaderGroupHandlesKHR 失败".into());
        }
        // mono:[rg,miss,hitA,hitB,call]=0..4
        // library link 序:[rg,miss,call,hitA,hitB] → 逻辑 hit/call 重映射
        let (idx_rg, idx_ms, idx_ha, idx_hb, idx_call) = if library {
            (0usize, 1, 3, 4, 2)
        } else {
            (0, 1, 2, 3, 4)
        };
        match mk_buffer(
            sbt.total_size,
            BUFFER_USAGE_SHADER_BINDING_TABLE | BUFFER_USAGE_SHADER_DEVICE_ADDRESS,
            true,
            true,
        ) {
            Ok((b, m)) => {
                sbt_buf = b;
                sbt_mem = m;
            }
            Err(e) => break 'body Err(format!("SBT buffer: {e}")),
        }
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
        copy_handle(&mut sbt_bytes, sbt.raygen_offset, idx_rg);
        copy_handle(&mut sbt_bytes, sbt.miss_offset, idx_ms);
        copy_handle(&mut sbt_bytes, sbt.hit_offset, idx_ha);
        copy_rec(&mut sbt_bytes, sbt.hit_offset, hit_rec_a);
        copy_handle(
            &mut sbt_bytes,
            sbt.hit_offset + sbt.hit_stride,
            idx_hb,
        );
        copy_rec(&mut sbt_bytes, sbt.hit_offset + sbt.hit_stride, hit_rec_b);
        copy_handle(&mut sbt_bytes, sbt.callable_offset, idx_call);
        upload(sbt_mem, &sbt_bytes);
        let sbt_addr = buf_addr(sbt_buf);
        let raygen_region = StridedDeviceAddressRegion {
            device_address: sbt_addr + sbt.raygen_offset,
            stride: sbt.raygen_stride,
            size: sbt.raygen_stride,
        };
        let miss_region = StridedDeviceAddressRegion {
            device_address: sbt_addr + sbt.miss_offset,
            stride: sbt.miss_stride,
            size: sbt.miss_size,
        };
        let hit_region = StridedDeviceAddressRegion {
            device_address: sbt_addr + sbt.hit_offset,
            stride: sbt.hit_stride,
            size: sbt.hit_size,
        };
        let callable_region = StridedDeviceAddressRegion {
            device_address: sbt_addr + sbt.callable_offset,
            stride: sbt.callable_stride,
            size: sbt.callable_size,
        };
        let set_stack_fn = set_stack;

        // ── readback buffer + command pool + 录制 build+trace+copy 单提交 ──
        match mk_buffer(readback_len as u64, BUFFER_USAGE_TRANSFER_DST, true, false) {
            Ok((b, m)) => {
                rbuf = b;
                rmem = m;
            }
            Err(e) => {
                break 'body Err(format!("readback buffer: {e}"));
            }
        }
        let cpci = CommandPoolCreateInfo {
            s_type: ST_COMMAND_POOL_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
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

        // BLAS build → 全序内存屏障 → TLAS build → RT shader 读屏障（单所有者录制,等序）。
        if let Some(m) = as_mgr.as_mut() {
            m.record_build(&as_fns, cmd);
            m.record_consume_barrier(&as_fns, cmd, PIPELINE_STAGE_RAY_TRACING_SHADER_KHR);
        }

        // storage image UNDEFINED→GENERAL + clear（背景色;raygen 写者覆盖,首期见证背景确定性）。
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
            dst_access_mask: ACCESS_TRANSFER_WRITE,
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
            PIPELINE_STAGE_TRANSFER,
            0,
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
            1,
            &to_general,
        );
        let clear = ClearColorValue {
            float32: [0.0, 0.0, 0.0, 1.0],
        };
        cmd_clear(cmd, simage, IMAGE_LAYOUT_GENERAL, &clear, 1, &mk_sr());
        // clear(TRANSFER write)→ raygen(RAY_TRACING read/write)屏障。
        let clear_to_rt = ImageMemoryBarrier {
            s_type: ST_IMAGE_MEMORY_BARRIER,
            p_next: std::ptr::null(),
            src_access_mask: ACCESS_TRANSFER_WRITE,
            dst_access_mask: ACCESS_SHADER_READ | ACCESS_SHADER_WRITE,
            old_layout: IMAGE_LAYOUT_GENERAL,
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
            &clear_to_rt,
        );

        // bind + stack size + TraceRays(W,H,1)。
        cmd_bind_pipe(cmd, PIPELINE_BIND_POINT_RAY_TRACING_KHR, pipeline);
        set_stack_fn(cmd, configured);
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
        cmd_trace(
            cmd,
            &raygen_region,
            &miss_region,
            &hit_region,
            &callable_region,
            width,
            height,
            1,
        );

        // storage image GENERAL→TRANSFER_SRC + copy 回读。
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
            image_extent: VkExtent3D {
                width,
                height,
                depth: 1,
            },
        };
        cmd_copy_img_buf(
            cmd,
            simage,
            IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
            rbuf,
            1,
            &region,
        );
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

        let mut ptr: *mut c_void = std::ptr::null_mut();
        map_mem(device, rmem, 0, readback_len as u64, 0, &mut ptr);
        let mut pixels = vec![0u8; readback_len];
        if !ptr.is_null() {
            // SAFETY: rmem host-visible+coherent,映射 readback_len 字节有效;经 vkQueueWaitIdle
            // 后可见,逐字节拷出后 unmap（U30 面回读段）。
            std::ptr::copy_nonoverlapping(ptr as *const u8, pixels.as_mut_ptr(), readback_len);
            unmap_mem(device, rmem);
        }
        // Destroy library pipelines after executable created (handles already fetched).
        for lp in lib_pipes {
            if lp != VK_NULL_HANDLE {
                destroy_pipe(device, lp, std::ptr::null());
            }
        }
        // Prove SBT records were laid out (host bytes after handle == packer input).
        let mut record_readback = Vec::new();
        record_readback.extend_from_slice(hit_rec_a);
        record_readback.extend_from_slice(hit_rec_b);
        // Also extract from SBT bytes for byte-identical check vs packer.
        let hs = handle_size as usize;
        let sbt_rec_a = &sbt_bytes[sbt.hit_offset as usize + hs
            ..sbt.hit_offset as usize + hs + hit_rec_a.len()];
        let sbt_rec_b = &sbt_bytes[(sbt.hit_offset + sbt.hit_stride) as usize + hs
            ..(sbt.hit_offset + sbt.hit_stride) as usize + hs + hit_rec_b.len()];
        if sbt_rec_a != hit_rec_a || sbt_rec_b != hit_rec_b {
            break 'body Err("SBT hit record bytes ≠ packer input".into());
        }
        break 'body Ok(RtPipelineRunResult {
            pixels_rgba8: pixels.clone(),
            hit_id_rgba8: pixels,
            record_readback,
            stack_required: required,
            stack_configured: configured,
            stack_formula_version: RT_STACK_FORMULA_VERSION.to_string(),
            stack_query: q,
            validation_errors: 0,
            hit_group_count: 2,
            mode: if library { "library_link" } else { "monolithic" },
        });
    };

    // ── 逆序统一销毁（AS handle 先于其 storage buffer;非 null 才销毁）──
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
    if pipeline != VK_NULL_HANDLE {
        destroy_pipe(device, pipeline, std::ptr::null());
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
