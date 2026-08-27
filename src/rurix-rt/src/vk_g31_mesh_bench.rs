// Assisted-by: Kimi-K3（G31+ 波 C Task C16 重判窗批量执行 M61 ③ measured 面）
// ── G31 mesh shader HW 路径 vs 现 VS 光栅路径 measured 对照底座（RFC-0034 重判表
//    三项闭集之③「mesh shader HW 管线性能差 measured 证据」唯一新面；TODO §3.1 #24）──
//
// 设计（诚实登记,不对称面不藏）:
// - **单会话三臂**（instance/device/queue 一次创建,同一 color image/render pass/fb）:
//   ① `vs_fetch` = 经典光栅路径（vertex stage + device-local vertex buffer 取数,
//   3N×16B 取数流量）;② `vs_procedural` = 同 vertex stage 但顶点由 gl_VertexIndex
//   整数哈希程序化生成（隔离取数成本,管线形态与①仅差 vertex input 状态）;
//   ③ `mesh_procedural` = mesh 阶段（无 vertex input/IA 状态,64 lane/wg 每 lane
//   1 三角形,workgroup 数 = N/64）。①vs③ = mesh 路径 vs 现取数光栅路径对照;
//   ②vs③ = 同程序化数据流下的纯管线形态对照（②为解释性臂,主对照 = ①vs③）。
// - **同一三角形集**:三臂顶点坐标全部出自同一确定性 u32 整数哈希（PCG RXS-M-XS;
//   host 侧 bin 逐字同源填 vertex buffer;浮点仅同序 IEEE 加减乘除,GLSL `precise`
//   禁 FMA 收缩）⇒ 渲染像素逐臂 digest 对拍（零差 = 同一几何真上屏的结构证据;
//   无深度/无混合 + fragment 恒色 ⇒ 重叠三角形写序不影响终图,digest 可比）。
// - **计时**:`vkCmdWriteTimestamp`（核心 1.0;TOP_OF_PIPE/BOTTOM_OF_PIPE 包 render
//   pass）逐帧双 tick × `timestampPeriod`（VkPhysicalDeviceProperties blob@720,
//   render_exec 同口径）折算 GPU ms;每帧 submit + vkQueueWaitIdle 全同步 + 壁钟
//   双口径;warmup 后取 median/mean/min/max 如实登记,**不设通过线**（G6 无硬门
//   纪律）。timestamp_valid_bits=0 / timestampPeriod≤0 → 确定性 Err（fail-closed）。
// - **fail-closed**:无 loader/无设备/`meshShader` feature 缺失 → 确定性 `Err`
//   （"mesh shader feature" 字样 → harness SKIP 三态,非 panic、不占 RX 码,
//   RXS-0210 L3 / RXS-0248 同律）;`RURIX_VK_VALIDATION=1` ERROR 级校验翻 `Err`。
// - **0-byte 纪律**:`run_mesh_offscreen` / `run_graphics_offscreen*` / render_exec
//   全族既有入口 0-byte 不动;本模块纯追加（vk_g31_present.rs 同型 include 先例）。
//
// SAFETY（U27 同族扩注,graphics FFI 边界内,0 新 U 号——mesh 管线在既有 graphics
// FFI 边界内,run_mesh_offscreen §4.E7 先例）:对上全 safe（无 `unsafe` 签名）。
// 内部 `g31_mesh_bench_inner` 契约同 U27:`vulkan-1.dll` 经 loader 动态装载（缺失 →
// `Err` 非 panic）;每个 #[repr(C)] VkStruct 与 spec 逐字节对齐;句柄（image·mem·view /
// renderPass / framebuffer / pipeline×3 / pipelineLayout / shaderModule×4 / vertex
// staging·device buffer / queryPool / readbackBuffer / commandPool）线性配对
// create/destroy（末尾逆序销毁）;单 graphics queue 同步提交 + `vkQueueWaitIdle`
// 后读 query/回读紧凑 RGBA8;messenger fail-closed 同 U27;feature `vulkan` 默认
// 关闭,CUDA 路零回归。

/// 单臂 measured 统计（GPU timestamp 主口径 + 壁钟副口径,如实登记不设通过线）。
pub struct MeshVsRasterArmReport {
    /// 臂名（vs_fetch / vs_procedural / mesh_procedural）。
    pub arm: &'static str,
    /// warmup 后逐帧 GPU ms 样本（render pass 区间,timestamp 主口径）。
    pub gpu_ms_samples: Vec<f64>,
    /// warmup 后逐帧壁钟 ms 样本（submit+waitIdle 区间,含 host 税,副口径）。
    pub wall_ms_samples: Vec<f64>,
    /// 末帧回读像素（RGBA8 紧凑;digest 对拍由调用方做）。
    pub pixels: Vec<u8>,
}

/// 三臂 measured 报告 + 设备面（全部实测自 VkPhysicalDeviceProperties/查询）。
pub struct MeshVsRasterBenchReport {
    pub width: u32,
    pub height: u32,
    /// 每臂绘制三角形数（三臂同一三角形集）。
    pub triangles: u32,
    pub frames: u32,
    pub warmup: u32,
    /// 驱动 `timestampPeriod`（ns/tick,实测）。
    pub timestamp_period_ns: f32,
    /// `vkGetPhysicalDeviceProperties` 实测（deviceName/driverVersion/vendorID/apiVersion）。
    pub device_name: String,
    pub driver_version: u32,
    pub vendor_id: u32,
    pub api_version: u32,
    pub arms: Vec<MeshVsRasterArmReport>,
}

#[allow(clippy::too_many_arguments)]
pub fn run_mesh_vs_raster_bench(
    mesh_spv: &[u32],
    vs_fetch_spv: &[u32],
    vs_proc_spv: &[u32],
    fs_spv: &[u32],
    width: u32,
    height: u32,
    grid_w: u32,
    grid_h: u32,
    cell_px: u32,
    tri_verts: &[[f32; 4]],
    tris_per_group: u32,
    frames: u32,
    warmup: u32,
) -> Result<MeshVsRasterBenchReport, String> {
    if tri_verts.is_empty() || !tri_verts.len().is_multiple_of(3) {
        return Err("tri_verts 须非空且为 3 的倍数（每三角形 3 顶点）".into());
    }
    let triangles = (tri_verts.len() / 3) as u32;
    if !triangles.is_multiple_of(tris_per_group) || tris_per_group == 0 {
        return Err(format!(
            "triangles({triangles}) 须为 tris_per_group({tris_per_group}) 非零整数倍"
        ));
    }
    if grid_w * cell_px != width || grid_h * cell_px != height {
        return Err("grid×cell 须恰等于 width×height（哈希坐标域不变式）".into());
    }
    let gipa = load_vulkan_loader().ok_or("vulkan loader (vulkan-1.dll/libvulkan.so) 不可用")?;
    // SAFETY: 见模块头 U27 同族扩注;句柄生命周期由内部函数线性管理,末尾逆序销毁。
    unsafe {
        g31_mesh_bench_inner(
            gipa,
            mesh_spv,
            vs_fetch_spv,
            vs_proc_spv,
            fs_spv,
            width,
            height,
            grid_w,
            grid_h,
            cell_px,
            tri_verts,
            tris_per_group,
            frames,
            warmup,
        )
    }
}

// ── query pool / timestamp FFI（vk.rs 首面;核心 1.0,零扩展依赖）──
type VkQueryPool = *mut c_void;
const ST_QUERY_POOL_CREATE_INFO: u32 = 11;
const QUERY_TYPE_TIMESTAMP: u32 = 2;
const QUERY_RESULT_64_BIT: u32 = 0x1;
#[repr(C)]
struct QueryPoolCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    query_type: u32,
    query_count: u32,
    pipeline_statistics: u32,
}
type FnCreateQueryPool = unsafe extern "system" fn(
    VkDevice,
    *const QueryPoolCreateInfo,
    *const c_void,
    *mut VkQueryPool,
) -> VkResult;
type FnDestroyQueryPool = unsafe extern "system" fn(VkDevice, VkQueryPool, *const c_void);
type FnCmdResetQueryPool = unsafe extern "system" fn(VkCommandBuffer, VkQueryPool, u32, u32);
type FnCmdWriteTimestamp = unsafe extern "system" fn(VkCommandBuffer, u32, VkQueryPool, u32);
type FnGetQueryPoolResults = unsafe extern "system" fn(
    VkDevice,
    VkQueryPool,
    u32,
    u32,
    usize,
    *mut c_void,
    u64,
    u32,
) -> VkResult;

#[allow(clippy::too_many_arguments)]
unsafe fn g31_mesh_bench_inner(
    gipa: FnGetInstanceProcAddr,
    mesh_spv: &[u32],
    vs_fetch_spv: &[u32],
    vs_proc_spv: &[u32],
    fs_spv: &[u32],
    width: u32,
    height: u32,
    grid_w: u32,
    grid_h: u32,
    cell_px: u32,
    tri_verts: &[[f32; 4]],
    tris_per_group: u32,
    frames: u32,
    warmup: u32,
) -> Result<MeshVsRasterBenchReport, String> {
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
        p_application_name: c"rurix-g31-mesh-bench".as_ptr(),
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
    // SAFETY: ici 栈上有效,实例句柄回写;U27 同型调用。
    if vk_create_instance(&ici, std::ptr::null(), &mut instance) != VK_SUCCESS {
        return Err("vkCreateInstance 失败".into());
    }
    let vk_destroy_instance: FnDestroyInstance =
        // SAFETY: 核心符号缺失即 Err（fail-closed）。
        cast_fn(gipa(instance, c"vkDestroyInstance".as_ptr())).ok_or("缺 vkDestroyInstance")?;
    let vk_enum_pd: FnEnumeratePhysicalDevices =
        // SAFETY: 同上。
        cast_fn(gipa(instance, c"vkEnumeratePhysicalDevices".as_ptr()))
            .ok_or("缺 vkEnumeratePhysicalDevices")?;
    let vk_get_qf: FnGetPhysicalDeviceQueueFamilyProperties =
        // SAFETY: 同上。
        cast_fn(gipa(instance, c"vkGetPhysicalDeviceQueueFamilyProperties".as_ptr()))
            .ok_or("缺 vkGetPhysicalDeviceQueueFamilyProperties")?;
    let vk_get_mem: FnGetPhysicalDeviceMemoryProperties =
        // SAFETY: 同上。
        cast_fn(gipa(instance, c"vkGetPhysicalDeviceMemoryProperties".as_ptr()))
            .ok_or("缺 vkGetPhysicalDeviceMemoryProperties")?;
    let vk_get_props: FnGetPhysicalDeviceProperties =
        // SAFETY: 同上。
        cast_fn(gipa(instance, c"vkGetPhysicalDeviceProperties".as_ptr()))
            .ok_or("缺 vkGetPhysicalDeviceProperties")?;
    let vk_create_device: FnCreateDevice =
        // SAFETY: 同上。
        cast_fn(gipa(instance, c"vkCreateDevice".as_ptr())).ok_or("缺 vkCreateDevice")?;
    let vk_get_device_proc: FnGetDeviceProcAddr =
        // SAFETY: 同上。
        cast_fn(gipa(instance, c"vkGetDeviceProcAddr".as_ptr())).ok_or("缺 vkGetDeviceProcAddr")?;
    let get_pd_features2: FnGetPhysicalDeviceFeatures2 =
        // SAFETY: 同上。
        cast_fn(gipa(instance, c"vkGetPhysicalDeviceFeatures2".as_ptr()))
            .ok_or("缺 vkGetPhysicalDeviceFeatures2（Vulkan 1.1 core）")?;

    let validation_error = std::sync::atomic::AtomicBool::new(false);
    let mut messenger: VkDebugUtilsMessengerEXT = VK_NULL_HANDLE;
    let destroy_messenger: Option<FnDestroyDebugUtilsMessengerEXT> = if validation {
        // SAFETY: 扩展符号可选;缺失则静默不开 messenger（与 U27 同律——校验层缺失不致命）。
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
        // SAFETY: dumci 栈上有效;失败容忍（messenger 留 VK_NULL_HANDLE,后续不销毁）。
        let _ = create_messenger(instance, &dumci, std::ptr::null(), &mut messenger);
    }
    macro_rules! destroy_msgr {
        () => {
            if let Some(dm) = destroy_messenger {
                if messenger != VK_NULL_HANDLE {
                    // SAFETY: messenger 由本实例创建且仅销毁一次。
                    dm(instance, messenger, std::ptr::null());
                }
            }
        };
    }

    let mut count = 0u32;
    // SAFETY: count 查询先行。
    vk_enum_pd(instance, &mut count, std::ptr::null_mut());
    if count == 0 {
        destroy_msgr!();
        vk_destroy_instance(instance, std::ptr::null());
        return Err("无 Vulkan 物理设备".into());
    }
    let mut pds = vec![std::ptr::null_mut::<c_void>(); count as usize];
    // SAFETY: pds 容量 ≥ count（同次枚举无并发变更）。
    vk_enum_pd(instance, &mut count, pds.as_mut_ptr());
    let pd = pds[0];

    // ── mesh feature 探测（fail-closed;RXS-0248/RXS-0210 L3 口径）──
    let mut mesh_feat = PhysicalDeviceMeshShaderFeatures {
        s_type: ST_PHYSICAL_DEVICE_MESH_SHADER_FEATURES_EXT,
        p_next: std::ptr::null_mut(),
        task_shader: 0,
        mesh_shader: 0,
        multiview_mesh_shader: 0,
        primitive_fragment_shading_rate_mesh_shader: 0,
        mesh_shader_queries: 0,
    };
    let mut feats2 = PhysicalDeviceFeatures2 {
        s_type: ST_PHYSICAL_DEVICE_FEATURES_2,
        p_next: &mut mesh_feat as *mut PhysicalDeviceMeshShaderFeatures as *mut c_void,
        features: std::mem::zeroed(),
    };
    // SAFETY: pNext 链 mesh_feat 栈上有效。
    get_pd_features2(pd, &mut feats2);
    if mesh_feat.mesh_shader == 0 {
        destroy_msgr!();
        vk_destroy_instance(instance, std::ptr::null());
        return Err(
            "device 缺 mesh shader feature（meshShader=0）;确定性 Err,无静默降级".to_string()
        );
    }

    // ── 设备 identity + timestampPeriod（blob@720,render_exec 同口径;SDK 1.3.296 布局）──
    let mut props_blob = std::mem::zeroed::<PhysicalDevicePropertiesBlob>();
    // SAFETY: blob 2048B ≥ VkPhysicalDeviceProperties（布局锚单测钉死）。
    vk_get_props(pd, &mut props_blob);
    // SAFETY: blob 尺寸恰 2048B（api_version u32 + _rest 2044）,按字节读字段偏移。
    let props_bytes: &[u8] = std::slice::from_raw_parts(
        &props_blob as *const PhysicalDevicePropertiesBlob as *const u8,
        2048,
    );
    let api_version = u32::from_le_bytes([
        props_bytes[0],
        props_bytes[1],
        props_bytes[2],
        props_bytes[3],
    ]);
    let driver_version = u32::from_le_bytes([
        props_bytes[4],
        props_bytes[5],
        props_bytes[6],
        props_bytes[7],
    ]);
    let vendor_id = u32::from_le_bytes([
        props_bytes[8],
        props_bytes[9],
        props_bytes[10],
        props_bytes[11],
    ]);
    let name_end = props_bytes[20..276]
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(256);
    let device_name = String::from_utf8_lossy(&props_bytes[20..20 + name_end]).into_owned();
    let timestamp_period_ns = f32::from_le_bytes([
        props_bytes[720],
        props_bytes[721],
        props_bytes[722],
        props_bytes[723],
    ]);
    if timestamp_period_ns <= 0.0 {
        destroy_msgr!();
        vk_destroy_instance(instance, std::ptr::null());
        return Err("timestampPeriod ≤ 0（设备不支持 timestamp 查询;fail-closed）".into());
    }

    let mut qf_count = 0u32;
    // SAFETY: count 查询先行。
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
    // SAFETY: qfs 容量 ≥ qf_count。
    vk_get_qf(pd, &mut qf_count, qfs.as_mut_ptr());
    let qfi = match qfs
        .iter()
        .position(|q| q.queue_flags & QUEUE_GRAPHICS_BIT != 0 && q.timestamp_valid_bits != 0)
    {
        Some(i) => i as u32,
        None => {
            destroy_msgr!();
            vk_destroy_instance(instance, std::ptr::null());
            return Err("无 graphics queue family（或 timestamp_valid_bits=0）".into());
        }
    };

    mesh_feat.mesh_shader = 1;
    mesh_feat.task_shader = 0;
    mesh_feat.multiview_mesh_shader = 0;
    mesh_feat.primitive_fragment_shading_rate_mesh_shader = 0;
    mesh_feat.mesh_shader_queries = 0;
    let dev_exts: Vec<*const c_char> = MESH_DEVICE_EXTENSIONS.iter().map(|e| e.as_ptr()).collect();
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
        p_next: &mesh_feat as *const PhysicalDeviceMeshShaderFeatures as *const c_void,
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
    // SAFETY: dci 栈上有效;mesh 扩展/feature 已探测在位。
    if vk_create_device(pd, &dci, std::ptr::null(), &mut device) != VK_SUCCESS {
        destroy_msgr!();
        vk_destroy_instance(instance, std::ptr::null());
        return Err("vkCreateDevice 失败（mesh 扩展/feature 启用）".into());
    }

    let mut out = g31_mesh_bench_body(
        vk_get_device_proc,
        device,
        pd,
        vk_get_mem,
        qfi,
        mesh_spv,
        vs_fetch_spv,
        vs_proc_spv,
        fs_spv,
        width,
        height,
        grid_w,
        grid_h,
        cell_px,
        tri_verts,
        tris_per_group,
        frames,
        warmup,
        timestamp_period_ns,
    );
    if validation && validation_error.load(std::sync::atomic::Ordering::Relaxed) {
        out = Err("VK_LAYER_KHRONOS_validation 报 ERROR 级校验错误（fail-closed,L3）".into());
    }
    if let Ok(rep) = out.as_mut() {
        rep.device_name = device_name;
        rep.driver_version = driver_version;
        rep.vendor_id = vendor_id;
        rep.api_version = api_version;
        rep.timestamp_period_ns = timestamp_period_ns;
    }
    let vk_destroy_device: Option<FnDestroyDevice> =
        // SAFETY: device 符号查询;缺失则跳过销毁（进程尾,OS 回收;与 U27 同律容忍）。
        cast_fn(vk_get_device_proc(device, c"vkDestroyDevice".as_ptr()));
    if let Some(dd) = vk_destroy_device {
        // SAFETY: device 由本函数创建,body 内全部子句柄已逆序销毁。
        dd(device, std::ptr::null());
    }
    destroy_msgr!();
    vk_destroy_instance(instance, std::ptr::null());
    out
}

#[allow(clippy::too_many_arguments)]
unsafe fn g31_mesh_bench_body(
    gdpa: FnGetDeviceProcAddr,
    device: VkDevice,
    pd: VkPhysicalDevice,
    vk_get_mem: FnGetPhysicalDeviceMemoryProperties,
    qfi: u32,
    mesh_spv: &[u32],
    vs_fetch_spv: &[u32],
    vs_proc_spv: &[u32],
    fs_spv: &[u32],
    width: u32,
    height: u32,
    grid_w: u32,
    grid_h: u32,
    cell_px: u32,
    tri_verts: &[[f32; 4]],
    tris_per_group: u32,
    frames: u32,
    warmup: u32,
    timestamp_period_ns: f32,
) -> Result<MeshVsRasterBenchReport, String> {
    macro_rules! dp {
        ($name:literal, $ty:ty) => {
            // SAFETY: 核心 device 符号缺失即 Err（fail-closed）。
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
    let create_rp: FnCreateRenderPass = dp!(c"vkCreateRenderPass", FnCreateRenderPass);
    let destroy_rp: FnDestroyRenderPass = dp!(c"vkDestroyRenderPass", FnDestroyRenderPass);
    let create_fb: FnCreateFramebuffer = dp!(c"vkCreateFramebuffer", FnCreateFramebuffer);
    let destroy_fb: FnDestroyFramebuffer = dp!(c"vkDestroyFramebuffer", FnDestroyFramebuffer);
    let create_gp: FnCreateGraphicsPipelines =
        dp!(c"vkCreateGraphicsPipelines", FnCreateGraphicsPipelines);
    let cmd_begin_rp: FnCmdBeginRenderPass = dp!(c"vkCmdBeginRenderPass", FnCmdBeginRenderPass);
    let cmd_end_rp: FnCmdEndRenderPass = dp!(c"vkCmdEndRenderPass", FnCmdEndRenderPass);
    let cmd_barrier: FnCmdPipelineBarrier = dp!(c"vkCmdPipelineBarrier", FnCmdPipelineBarrier);
    let cmd_copy_img_buf: FnCmdCopyImageToBuffer =
        dp!(c"vkCmdCopyImageToBuffer", FnCmdCopyImageToBuffer);
    let cmd_draw_mesh: FnCmdDrawMeshTasksEXT = dp!(c"vkCmdDrawMeshTasksEXT", FnCmdDrawMeshTasksEXT);
    let cmd_draw: FnCmdDraw = dp!(c"vkCmdDraw", FnCmdDraw);
    let cmd_bind_vb: FnCmdBindVertexBuffers =
        dp!(c"vkCmdBindVertexBuffers", FnCmdBindVertexBuffers);
    let cmd_push: FnCmdPushConstants = dp!(c"vkCmdPushConstants", FnCmdPushConstants);
    let cmd_copy_buf: FnCmdCopyBuffer = dp!(c"vkCmdCopyBuffer", FnCmdCopyBuffer);
    let create_qp: FnCreateQueryPool = dp!(c"vkCreateQueryPool", FnCreateQueryPool);
    let destroy_qp: FnDestroyQueryPool = dp!(c"vkDestroyQueryPool", FnDestroyQueryPool);
    let cmd_reset_qp: FnCmdResetQueryPool = dp!(c"vkCmdResetQueryPool", FnCmdResetQueryPool);
    let cmd_write_ts: FnCmdWriteTimestamp = dp!(c"vkCmdWriteTimestamp", FnCmdWriteTimestamp);
    let get_qp_results: FnGetQueryPoolResults =
        dp!(c"vkGetQueryPoolResults", FnGetQueryPoolResults);

    let mut queue: VkQueue = std::ptr::null_mut();
    // SAFETY: qfi 已探测 graphics+timestamp 有效族。
    get_queue(device, qfi, 0, &mut queue);
    let mut memprops = std::mem::zeroed::<PhysicalDeviceMemoryProperties>();
    // SAFETY: memprops 栈上有效。
    vk_get_mem(pd, &mut memprops);
    let readback_len = (width as usize) * (height as usize) * 4;
    let triangles = (tri_verts.len() / 3) as u32;

    // ── 资源宏:失败统一跳 'cleanup（逆序销毁表）──
    macro_rules! alloc_buf {
        ($size:expr, $usage:expr, $mem_flags:expr) => {{
            let bci = BufferCreateInfo {
                s_type: ST_BUFFER_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                size: ($size).max(4),
                usage: $usage,
                sharing_mode: SHARING_MODE_EXCLUSIVE,
                queue_family_index_count: 0,
                p_queue_family_indices: std::ptr::null(),
            };
            let mut buf: VkBuffer = VK_NULL_HANDLE;
            // SAFETY: bci 栈上有效。
            if create_buffer(device, &bci, std::ptr::null(), &mut buf) != VK_SUCCESS {
                Err("vkCreateBuffer 失败".to_string())
            } else {
                let mut req = std::mem::zeroed::<MemoryRequirements>();
                // SAFETY: buf 有效。
                buf_mem_req(device, buf, &mut req);
                match pick_mem_type(&memprops, req.memory_type_bits, $mem_flags) {
                    None => {
                        // SAFETY: buf 已建未绑。
                        destroy_buffer(device, buf, std::ptr::null());
                        Err("无匹配内存类型".to_string())
                    }
                    Some(mt) => {
                        let mai = MemoryAllocateInfo {
                            s_type: ST_MEMORY_ALLOCATE_INFO,
                            p_next: std::ptr::null(),
                            allocation_size: req.size,
                            memory_type_index: mt,
                        };
                        let mut mem: VkDeviceMemory = VK_NULL_HANDLE;
                        // SAFETY: mai 栈上有效。
                        if alloc_mem(device, &mai, std::ptr::null(), &mut mem) != VK_SUCCESS {
                            destroy_buffer(device, buf, std::ptr::null());
                            Err("vkAllocateMemory 失败".to_string())
                        } else {
                            // SAFETY: buf/mem 均有效,offset 0。
                            bind_buf(device, buf, mem, 0);
                            Ok((buf, mem))
                        }
                    }
                }
            }
        }};
    }

    // color image（三臂共享同一 render target;COLOR_ATTACHMENT | TRANSFER_SRC）。
    let ici2 = ImageCreateInfo {
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
    let mut color_image: VkImage = VK_NULL_HANDLE;
    // SAFETY: ici2 栈上有效。
    if create_image(device, &ici2, std::ptr::null(), &mut color_image) != VK_SUCCESS {
        return Err("vkCreateImage 失败".into());
    }
    let mut ireq = std::mem::zeroed::<MemoryRequirements>();
    // SAFETY: color_image 有效。
    img_mem_req(device, color_image, &mut ireq);
    let Some(imt) = pick_mem_type(&memprops, ireq.memory_type_bits, MEM_DEVICE_LOCAL) else {
        // SAFETY: color_image 已建未绑。
        destroy_image(device, color_image, std::ptr::null());
        return Err("无 device-local 内存类型".into());
    };
    let mai = MemoryAllocateInfo {
        s_type: ST_MEMORY_ALLOCATE_INFO,
        p_next: std::ptr::null(),
        allocation_size: ireq.size,
        memory_type_index: imt,
    };
    let mut color_mem: VkDeviceMemory = VK_NULL_HANDLE;
    // SAFETY: mai 栈上有效。
    if alloc_mem(device, &mai, std::ptr::null(), &mut color_mem) != VK_SUCCESS {
        destroy_image(device, color_image, std::ptr::null());
        return Err("color image vkAllocateMemory 失败".into());
    }
    // SAFETY: color_image/color_mem 有效,offset 0。
    bind_image(device, color_image, color_mem, 0);
    let vci = ImageViewCreateInfo {
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
    let mut color_view: VkImageView = VK_NULL_HANDLE;
    let mut rp: VkRenderPass = VK_NULL_HANDLE;
    let mut fb: VkFramebuffer = VK_NULL_HANDLE;
    let mut layout: VkPipelineLayout = VK_NULL_HANDLE;
    let mut pipe_vs_fetch: VkPipeline = VK_NULL_HANDLE;
    let mut pipe_vs_proc: VkPipeline = VK_NULL_HANDLE;
    let mut pipe_mesh: VkPipeline = VK_NULL_HANDLE;
    let mut vbuf: VkBuffer = VK_NULL_HANDLE;
    let mut vmem: VkDeviceMemory = VK_NULL_HANDLE;
    let mut qpool: VkQueryPool = std::ptr::null_mut();
    let mut rbuf: VkBuffer = VK_NULL_HANDLE;
    let mut rmem: VkDeviceMemory = VK_NULL_HANDLE;
    let mut cmdpool: VkCommandPool = VK_NULL_HANDLE;

    let result: Result<MeshVsRasterBenchReport, String> = 'body: {
        // SAFETY: vci 引用有效 image。
        if create_view(device, &vci, std::ptr::null(), &mut color_view) != VK_SUCCESS {
            break 'body Err("vkCreateImageView 失败".into());
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
        // SAFETY: rp_ci 引用栈上 att/subpass 有效。
        if create_rp(device, &rp_ci, std::ptr::null(), &mut rp) != VK_SUCCESS {
            break 'body Err("vkCreateRenderPass 失败".into());
        }
        let fb_ci = FramebufferCreateInfo {
            s_type: ST_FRAMEBUFFER_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            render_pass: rp,
            attachment_count: 1,
            p_attachments: &color_view,
            width,
            height,
            layers: 1,
        };
        // SAFETY: fb_ci 引用有效 rp/view。
        if create_fb(device, &fb_ci, std::ptr::null(), &mut fb) != VK_SUCCESS {
            break 'body Err("vkCreateFramebuffer 失败".into());
        }

        // shader modules ×4。
        let make_shader = |spv: &[u32]| -> Result<VkShaderModule, String> {
            let smci = ShaderModuleCreateInfo {
                s_type: ST_SHADER_MODULE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                code_size: spv.len() * 4,
                p_code: spv.as_ptr(),
            };
            let mut m: VkShaderModule = VK_NULL_HANDLE;
            // SAFETY: spv 调用方缓冲有效至本调用返回。
            if create_shader(device, &smci, std::ptr::null(), &mut m) != VK_SUCCESS {
                return Err("vkCreateShaderModule 失败".into());
            }
            Ok(m)
        };
        let mut shader_mods: Vec<VkShaderModule> = Vec::new();
        let mut shader_err: Option<String> = None;
        for spv in [mesh_spv, vs_fetch_spv, vs_proc_spv, fs_spv] {
            match make_shader(spv) {
                Ok(m) => shader_mods.push(m),
                Err(e) => {
                    shader_err = Some(e);
                    break;
                }
            }
        }
        if let Some(e) = shader_err {
            for m in shader_mods {
                // SAFETY: 配对销毁。
                destroy_shader(device, m, std::ptr::null());
            }
            break 'body Err(e);
        }
        let (mesh_mod, vsf_mod, vsp_mod, fs_mod) = (
            shader_mods[0],
            shader_mods[1],
            shader_mods[2],
            shader_mods[3],
        );
        let entry = c"main";

        // 共享 pipeline layout（push constants [gw,gh,cell,nt] 16B,VERTEX|MESH 双阶段位）。
        let pcr = PushConstantRange {
            stage_flags: SHADER_STAGE_VERTEX | SHADER_STAGE_MESH_EXT,
            offset: 0,
            size: 16,
        };
        let plci = PipelineLayoutCreateInfo {
            s_type: ST_PIPELINE_LAYOUT_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            set_layout_count: 0,
            p_set_layouts: std::ptr::null(),
            push_constant_range_count: 1,
            p_push_constant_ranges: &pcr,
        };
        // SAFETY: pcr 栈上有效。
        if create_pl(device, &plci, std::ptr::null(), &mut layout) != VK_SUCCESS {
            destroy_shader(device, fs_mod, std::ptr::null());
            destroy_shader(device, vsp_mod, std::ptr::null());
            destroy_shader(device, vsf_mod, std::ptr::null());
            destroy_shader(device, mesh_mod, std::ptr::null());
            break 'body Err("vkCreatePipelineLayout 失败".into());
        }

        let vp = VkViewport {
            x: 0.0,
            y: 0.0,
            width: width as f32,
            height: height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };
        let sc = VkRect2D {
            offset: VkOffset2D { x: 0, y: 0 },
            extent: VkExtent2D { width, height },
        };
        let vpstate = PipelineViewportStateCreateInfo {
            s_type: ST_PIPELINE_VIEWPORT_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            viewport_count: 1,
            p_viewports: &vp,
            scissor_count: 1,
            p_scissors: &sc,
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
        let cba = PipelineColorBlendAttachmentState {
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
            p_attachments: &cba,
            blend_constants: [0.0; 4],
        };
        let vbind = [VkVertexInputBindingDescription {
            binding: 0,
            stride: 16,
            input_rate: VERTEX_INPUT_RATE_VERTEX,
        }];
        let vattr = [VkVertexInputAttributeDescription {
            location: 0,
            binding: 0,
            format: FORMAT_R32G32B32A32_SFLOAT,
            offset: 0,
        }];
        let vin_fetch = PipelineVertexInputStateCreateInfo {
            s_type: ST_PIPELINE_VERTEX_INPUT_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            vertex_binding_description_count: 1,
            p_vertex_binding_descriptions: vbind.as_ptr(),
            vertex_attribute_description_count: 1,
            p_vertex_attribute_descriptions: vattr.as_ptr(),
        };
        let vin_empty = PipelineVertexInputStateCreateInfo {
            s_type: ST_PIPELINE_VERTEX_INPUT_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            vertex_binding_description_count: 0,
            p_vertex_binding_descriptions: std::ptr::null(),
            vertex_attribute_description_count: 0,
            p_vertex_attribute_descriptions: std::ptr::null(),
        };
        let ia = PipelineInputAssemblyStateCreateInfo {
            s_type: ST_PIPELINE_INPUT_ASSEMBLY_STATE_CI,
            p_next: std::ptr::null(),
            flags: 0,
            topology: PRIMITIVE_TOPOLOGY_TRIANGLE_LIST,
            primitive_restart_enable: 0,
        };
        let mk_stages =
            |a_mod: VkShaderModule, a_stage: u32| -> [PipelineShaderStageCreateInfo; 2] {
                [
                    PipelineShaderStageCreateInfo {
                        s_type: ST_PIPELINE_SHADER_STAGE_CREATE_INFO,
                        p_next: std::ptr::null(),
                        flags: 0,
                        stage: a_stage,
                        module: a_mod,
                        p_name: entry.as_ptr(),
                        p_specialization_info: std::ptr::null(),
                    },
                    PipelineShaderStageCreateInfo {
                        s_type: ST_PIPELINE_SHADER_STAGE_CREATE_INFO,
                        p_next: std::ptr::null(),
                        flags: 0,
                        stage: SHADER_STAGE_FRAGMENT,
                        module: fs_mod,
                        p_name: entry.as_ptr(),
                        p_specialization_info: std::ptr::null(),
                    },
                ]
            };
        let pipe_create = |stages: &[PipelineShaderStageCreateInfo],
                           vin: *const PipelineVertexInputStateCreateInfo,
                           iap: *const PipelineInputAssemblyStateCreateInfo,
                           out: &mut VkPipeline|
         -> Result<(), String> {
            let gpci = GraphicsPipelineCreateInfo {
                s_type: ST_GRAPHICS_PIPELINE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                stage_count: stages.len() as u32,
                p_stages: stages.as_ptr(),
                p_vertex_input_state: vin,
                p_input_assembly_state: iap,
                p_tessellation_state: std::ptr::null(),
                p_viewport_state: &vpstate,
                p_rasterization_state: &rs,
                p_multisample_state: &ms,
                p_depth_stencil_state: std::ptr::null(),
                p_color_blend_state: &cb,
                p_dynamic_state: std::ptr::null(),
                layout,
                render_pass: rp,
                subpass: 0,
                base_pipeline_handle: VK_NULL_HANDLE,
                base_pipeline_index: -1,
            };
            // SAFETY: gpci 全部引用栈上有效;stages 调用方数组有效。
            if create_gp(device, VK_NULL_HANDLE, 1, &gpci, std::ptr::null(), out) != VK_SUCCESS {
                return Err("vkCreateGraphicsPipelines 失败".into());
            }
            Ok(())
        };
        let stages_vs_fetch = mk_stages(vsf_mod, SHADER_STAGE_VERTEX);
        let stages_vs_proc = mk_stages(vsp_mod, SHADER_STAGE_VERTEX);
        let stages_mesh = mk_stages(mesh_mod, SHADER_STAGE_MESH_EXT);
        let pipes_ok = pipe_create(&stages_vs_fetch, &vin_fetch, &ia, &mut pipe_vs_fetch)
            .and_then(|_| pipe_create(&stages_vs_proc, &vin_empty, &ia, &mut pipe_vs_proc))
            .and_then(|_| {
                // mesh 管线:无 vertex input / IA 状态（§4.E7 先例）。
                pipe_create(
                    &stages_mesh,
                    std::ptr::null(),
                    std::ptr::null(),
                    &mut pipe_mesh,
                )
            });
        // shader modules 可在 pipeline 建立后销毁。
        destroy_shader(device, fs_mod, std::ptr::null());
        destroy_shader(device, vsp_mod, std::ptr::null());
        destroy_shader(device, vsf_mod, std::ptr::null());
        destroy_shader(device, mesh_mod, std::ptr::null());
        if let Err(e) = pipes_ok {
            break 'body Err(e);
        }

        // vertex buffer（device-local,生产口径）+ staging 上传。
        let vb_len = (tri_verts.len() * 16) as u64;
        match alloc_buf!(
            vb_len,
            BUFFER_USAGE_VERTEX | BUFFER_USAGE_TRANSFER_DST,
            MEM_DEVICE_LOCAL
        ) {
            Ok((b, m)) => {
                vbuf = b;
                vmem = m;
            }
            Err(e) => break 'body Err(e),
        }
        let staging = alloc_buf!(
            vb_len,
            BUFFER_USAGE_TRANSFER_SRC,
            MEM_HOST_VISIBLE | MEM_HOST_COHERENT
        );
        let (sbuf, smem) = match staging {
            Ok(t) => t,
            Err(e) => break 'body Err(e),
        };
        let mut sptr: *mut c_void = std::ptr::null_mut();
        // SAFETY: smem host-visible,vb_len 字节有效。
        map_mem(device, smem, 0, vb_len.max(4), 0, &mut sptr);
        if sptr.is_null() {
            destroy_buffer(device, sbuf, std::ptr::null());
            free_mem(device, smem, std::ptr::null());
            break 'body Err("staging vkMapMemory 失败".into());
        }
        // SAFETY: sptr 有效 vb_len 字节;tri_verts 源切片同长。
        std::ptr::copy_nonoverlapping(
            tri_verts.as_ptr() as *const u8,
            sptr as *mut u8,
            vb_len as usize,
        );
        unmap_mem(device, smem);

        // query pool（2 slot;逐帧 reset 复用）。
        let qpci = QueryPoolCreateInfo {
            s_type: ST_QUERY_POOL_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            query_type: QUERY_TYPE_TIMESTAMP,
            query_count: 2,
            pipeline_statistics: 0,
        };
        // SAFETY: qpci 栈上有效。
        if create_qp(device, &qpci, std::ptr::null(), &mut qpool) != VK_SUCCESS {
            destroy_buffer(device, sbuf, std::ptr::null());
            free_mem(device, smem, std::ptr::null());
            break 'body Err("vkCreateQueryPool 失败".into());
        }

        // readback buffer（host-visible+coherent）。
        match alloc_buf!(
            readback_len as u64,
            BUFFER_USAGE_TRANSFER_DST,
            MEM_HOST_VISIBLE | MEM_HOST_COHERENT
        ) {
            Ok((b, m)) => {
                rbuf = b;
                rmem = m;
            }
            Err(e) => {
                destroy_buffer(device, sbuf, std::ptr::null());
                free_mem(device, smem, std::ptr::null());
                break 'body Err(e);
            }
        }

        // command pool + 4 cmd（upload 1 + 臂 3;readback 复录于臂 cmd 尾部变体——
        // 计时 cmd 与回读 cmd 分离:每臂 1 条计时 cmd（循环提交）+ 共享 1 条回读 cmd）。
        let cpci = CommandPoolCreateInfo {
            s_type: ST_COMMAND_POOL_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            queue_family_index: qfi,
        };
        // SAFETY: cpci 栈上有效。
        create_cmdpool(device, &cpci, std::ptr::null(), &mut cmdpool);
        let cbai = CommandBufferAllocateInfo {
            s_type: ST_COMMAND_BUFFER_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            command_pool: cmdpool,
            level: COMMAND_BUFFER_LEVEL_PRIMARY_MESH_RT,
            command_buffer_count: 5,
        };
        let mut cmds: [VkCommandBuffer; 5] = [std::ptr::null_mut(); 5];
        // SAFETY: cmds 容量 5 = command_buffer_count。
        if alloc_cmd(device, &cbai, cmds.as_mut_ptr()) != VK_SUCCESS || cmds[4].is_null() {
            break 'body Err("vkAllocateCommandBuffers 失败".into());
        }
        let (cmd_upload, cmd_rb) = (cmds[0], cmds[4]);
        // 计时/回读 cmd 逐帧重复提交 ⇒ 不带 ONE_TIME_SUBMIT(多提交合法;
        // UNASSIGNED-DrawState-CommandBufferSingleSubmitViolation 规避)。
        let cbbi = CommandBufferBeginInfo {
            s_type: ST_COMMAND_BUFFER_BEGIN_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            p_inheritance_info: std::ptr::null(),
        };

        // ① upload cmd:staging → device-local VB。
        // SAFETY: cmd_upload 已分配。
        begin_cmd(cmd_upload, &cbbi);
        let cp_region = VkBufferCopy {
            src_offset: 0,
            dst_offset: 0,
            size: vb_len,
        };
        // SAFETY: sbuf/vbuf 有效,size ≤ 双缓冲容量。
        cmd_copy_buf(cmd_upload, sbuf, vbuf, 1, &cp_region);
        end_cmd(cmd_upload);
        let submit = SubmitInfo {
            s_type: ST_SUBMIT_INFO,
            p_next: std::ptr::null(),
            wait_semaphore_count: 0,
            p_wait_semaphores: std::ptr::null(),
            p_wait_dst_stage_mask: std::ptr::null(),
            command_buffer_count: 1,
            p_command_buffers: &cmd_upload,
            signal_semaphore_count: 0,
            p_signal_semaphores: std::ptr::null(),
        };
        // SAFETY: submit 引用有效 cmd。
        queue_submit(queue, 1, &submit, VK_NULL_HANDLE);
        queue_wait(queue);
        // staging 一次性消费完毕即毁。
        destroy_buffer(device, sbuf, std::ptr::null());
        free_mem(device, smem, std::ptr::null());

        // ② 回读 cmd（三臂共享;render pass finalLayout 已为 TRANSFER_SRC）。
        // SAFETY: cmd_rb 已分配。
        begin_cmd(cmd_rb, &cbbi);
        let bar = ImageMemoryBarrier {
            s_type: ST_IMAGE_MEMORY_BARRIER,
            p_next: std::ptr::null(),
            src_access_mask: ACCESS_COLOR_ATTACHMENT_WRITE,
            dst_access_mask: ACCESS_TRANSFER_READ,
            old_layout: IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
            new_layout: IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
            src_queue_family_index: !0,
            dst_queue_family_index: !0,
            image: color_image,
            subresource_range: VkImageSubresourceRange {
                aspect_mask: IMAGE_ASPECT_COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
        };
        // SAFETY: bar 栈上有效。
        cmd_barrier(
            cmd_rb,
            PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT,
            PIPELINE_STAGE_TRANSFER,
            0,
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
            1,
            &bar,
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
        // SAFETY: color_image TRANSFER_SRC 布局;rbuf 容量 ≥ readback_len。
        cmd_copy_img_buf(
            cmd_rb,
            color_image,
            IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
            rbuf,
            1,
            &region,
        );
        end_cmd(cmd_rb);

        // ③ 三臂计时 cmd（每臂一条,循环重录外循环复用——同 cmd 重复提交,timestamp
        //    slot 每帧 reset 覆写,读回在 waitIdle 后）。
        let pc_data: [u32; 4] = [grid_w, grid_h, cell_px, triangles];
        let clearv = ClearValue {
            color: [0.0, 0.0, 0.0, 1.0],
        };
        let rpbi = RenderPassBeginInfo {
            s_type: ST_RENDER_PASS_BEGIN_INFO,
            p_next: std::ptr::null(),
            render_pass: rp,
            framebuffer: fb,
            render_area: VkRect2D {
                offset: VkOffset2D { x: 0, y: 0 },
                extent: VkExtent2D { width, height },
            },
            clear_value_count: 1,
            p_clear_values: &clearv,
        };
        let arm_defs: [(&str, VkPipeline, u32, bool); 3] = [
            ("vs_fetch", pipe_vs_fetch, SHADER_STAGE_VERTEX, true),
            ("vs_procedural", pipe_vs_proc, SHADER_STAGE_VERTEX, false),
            ("mesh_procedural", pipe_mesh, SHADER_STAGE_MESH_EXT, false),
        ];
        for (ai, (_name, pipe, stage_bit, use_vb)) in arm_defs.iter().enumerate() {
            let cmd = cmds[ai + 1];
            // SAFETY: cmd 已分配;全部引用对象存活至销毁段。
            begin_cmd(cmd, &cbbi);
            cmd_reset_qp(cmd, qpool, 0, 2);
            cmd_write_ts(cmd, PIPELINE_STAGE_TOP_OF_PIPE, qpool, 0);
            cmd_begin_rp(cmd, &rpbi, SUBPASS_CONTENTS_INLINE);
            cmd_bind_pipe(cmd, PIPELINE_BIND_POINT_GRAPHICS, *pipe);
            // VUID-vkCmdPushConstants-offset-01796:stageFlags 须覆盖 layout 重叠 range
            // 全阶段位(VERTEX|MESH 共享 range ⇒ 三臂同按 union 位推送)。
            cmd_push(
                cmd,
                layout,
                SHADER_STAGE_VERTEX | SHADER_STAGE_MESH_EXT,
                0,
                16,
                pc_data.as_ptr() as *const c_void,
            );
            if *use_vb {
                let offsets: [u64; 1] = [0];
                let vbs: [VkBuffer; 1] = [vbuf];
                cmd_bind_vb(cmd, 0, 1, vbs.as_ptr(), offsets.as_ptr());
                cmd_draw(cmd, triangles * 3, 1, 0, 0);
            } else if *stage_bit == SHADER_STAGE_VERTEX {
                cmd_draw(cmd, triangles * 3, 1, 0, 0);
            } else {
                cmd_draw_mesh(cmd, triangles / tris_per_group, 1, 1);
            }
            cmd_end_rp(cmd);
            cmd_write_ts(cmd, PIPELINE_STAGE_BOTTOM_OF_PIPE, qpool, 1);
            end_cmd(cmd);
        }

        // ④ 逐臂计时循环 + 末帧回读。
        let mut arms: Vec<MeshVsRasterArmReport> = Vec::new();
        let mut ts_buf: [u64; 2] = [0, 0];
        for (ai, (name, _pipe, _stage, _vb)) in arm_defs.iter().enumerate() {
            let cmd = cmds[ai + 1];
            let submit_arm = SubmitInfo {
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
            let mut gpu_ms_samples: Vec<f64> = Vec::new();
            let mut wall_ms_samples: Vec<f64> = Vec::new();
            for fi in 0..(warmup + frames) {
                let t0 = std::time::Instant::now();
                // SAFETY: submit_arm 引用有效 cmd;qpool 有效。
                queue_submit(queue, 1, &submit_arm, VK_NULL_HANDLE);
                queue_wait(queue);
                let wall_ms = t0.elapsed().as_secs_f64() * 1.0e3;
                // SAFETY: waitIdle 后 query 结果可用;buf 16B 有效。
                let qr = get_qp_results(
                    device,
                    qpool,
                    0,
                    2,
                    16,
                    ts_buf.as_mut_ptr() as *mut c_void,
                    8,
                    QUERY_RESULT_64_BIT,
                );
                if qr != VK_SUCCESS {
                    break 'body Err(format!(
                        "vkGetQueryPoolResults rc={qr}（timestamp 读回失败）"
                    ));
                }
                if fi >= warmup {
                    let delta = ts_buf[1].wrapping_sub(ts_buf[0]);
                    gpu_ms_samples.push(delta as f64 * timestamp_period_ns as f64 * 1.0e-6);
                    wall_ms_samples.push(wall_ms);
                }
            }
            // 末帧已留 TRANSFER_SRC → 回读。
            let submit_rb = SubmitInfo {
                s_type: ST_SUBMIT_INFO,
                p_next: std::ptr::null(),
                wait_semaphore_count: 0,
                p_wait_semaphores: std::ptr::null(),
                p_wait_dst_stage_mask: std::ptr::null(),
                command_buffer_count: 1,
                p_command_buffers: &cmd_rb,
                signal_semaphore_count: 0,
                p_signal_semaphores: std::ptr::null(),
            };
            // SAFETY: cmd_rb 已录制有效。
            queue_submit(queue, 1, &submit_rb, VK_NULL_HANDLE);
            queue_wait(queue);
            let mut ptr: *mut c_void = std::ptr::null_mut();
            // SAFETY: rmem host-visible+coherent,readback_len 字节有效;waitIdle 后可见。
            map_mem(device, rmem, 0, readback_len as u64, 0, &mut ptr);
            let mut pixels = vec![0u8; readback_len];
            if !ptr.is_null() {
                // SAFETY: 逐字节拷出后 unmap（U27 纪律）。
                std::ptr::copy_nonoverlapping(ptr as *const u8, pixels.as_mut_ptr(), readback_len);
                unmap_mem(device, rmem);
            }
            arms.push(MeshVsRasterArmReport {
                arm: name,
                gpu_ms_samples,
                wall_ms_samples,
                pixels,
            });
        }

        break 'body Ok(MeshVsRasterBenchReport {
            width,
            height,
            triangles,
            frames,
            warmup,
            timestamp_period_ns,
            device_name: String::new(),
            driver_version: 0,
            vendor_id: 0,
            api_version: 0,
            arms,
        });
    };
    // ── 逆序销毁（句柄线性配对）──
    if cmdpool != VK_NULL_HANDLE {
        // SAFETY: cmdpool 有效即毁（其下 cmd 随之释放）。
        destroy_cmdpool(device, cmdpool, std::ptr::null());
    }
    if rbuf != VK_NULL_HANDLE {
        // SAFETY: 配对销毁。
        destroy_buffer(device, rbuf, std::ptr::null());
        free_mem(device, rmem, std::ptr::null());
    }
    if !qpool.is_null() {
        // SAFETY: 配对销毁。
        destroy_qp(device, qpool, std::ptr::null());
    }
    if vbuf != VK_NULL_HANDLE {
        // SAFETY: 配对销毁。
        destroy_buffer(device, vbuf, std::ptr::null());
        free_mem(device, vmem, std::ptr::null());
    }
    for p in [pipe_vs_fetch, pipe_vs_proc, pipe_mesh] {
        if p != VK_NULL_HANDLE {
            // SAFETY: 配对销毁。
            destroy_pipe(device, p, std::ptr::null());
        }
    }
    if layout != VK_NULL_HANDLE {
        // SAFETY: 配对销毁。
        destroy_pl(device, layout, std::ptr::null());
    }
    if fb != VK_NULL_HANDLE {
        // SAFETY: 配对销毁。
        destroy_fb(device, fb, std::ptr::null());
    }
    if rp != VK_NULL_HANDLE {
        // SAFETY: 配对销毁。
        destroy_rp(device, rp, std::ptr::null());
    }
    if color_view != VK_NULL_HANDLE {
        // SAFETY: 配对销毁。
        destroy_view(device, color_view, std::ptr::null());
    }
    // SAFETY: color image/mem 配对销毁。
    destroy_image(device, color_image, std::ptr::null());
    free_mem(device, color_mem, std::ptr::null());
    result
}
