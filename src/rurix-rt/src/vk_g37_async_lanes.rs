// Assisted-by: Cursor Agent(G37 W3 async — 异步 compute 三件套判档窗实施)
// ── G37 W3 async:异步 compute 双队列判档加性面(TODO #57/#59/#60/#62;设计事实源 =
//    artifacts/day_0830_delivery/w3_deep/async/{PLAN.md, PATCH_PROPOSAL_vk_timeline.md})──
//
// 职责闭集(**全部加性**;vk.rs 既有函数/结构/常量 0 改写;body-include 先例
// vk_g31_mesh_bench.rs 同型,经 vk.rs 尾部 include! 编入同一模块命名空间):
//
// 1. timeline semaphore FFI 面(1.2 core):`SemaphoreTypeCreateInfo`(32B)/
//    `TimelineSemaphoreSubmitInfo`(48B)/ `SemaphoreWaitInfo`(40B)+
//    `vkWaitSemaphores` / `vkGetSemaphoreCounterValue` fn 指针;sType 经 SDK 1.3.296
//    `vulkan_core.h` 逐值核对 + 锚单测(vk.rs 27820 段先例)。
// 2. [`probe_async_queue_caps`]:compute-only(COMPUTE 且非 GRAPHICS)family 探测 +
//    `timelineSemaphore` feature 单链查询(复用既有
//    `PhysicalDeviceTimelineSemaphoreFeatures`,探测结果反哺 device 创建判据)+
//    timestamp 有效面(`timestamp_valid_bits` / `timestampPeriod`@blob 720,
//    vk_g31_mesh_bench 同口径)。真值不进 golden(RXS-0351 L9 同律)。
// 3. [`run_async_lanes`]:判档 harness 专用双队列执行器 —— device 创建 `p_next` 挂
//    timeline feature(镜像 TIRT 并行上下文先例 `create_tirt_vulkan_device`,既有
//    入口 0-byte)+ **逐字消费** harness 段切分/值域合法化产物(禁二次推导,
//    RXS-0240 逐字重放同律;仅做提交前 fail-closed 结构核验)+ 每段一次
//    `vkQueueSubmit` 挂 `TimelineSemaphoreSubmitInfo` + 帧末 host `vkWaitSemaphores`
//    (替代 QueueWaitIdle)+ 每段首尾 `vkCmdWriteTimestamp`(重叠率 evidence;
//    query pool FFI 复用 vk_g31_mesh_bench 既有定义,零重定义)。
//    单队列臂(`dual_queue=false`)= 同一入口的显式 single-queue 形态(一条 queue、
//    零 timeline、`vkQueueWaitIdle`)——回落语义由调用方经 `enable_async=false`
//    **重编译**产段(RFC-0019 §4.8.3 显式 single-queue plan;非忽略 fence)。
// 4. 判档窗简化(诚实登记):双队列臂资源建 `VK_SHARING_MODE_CONCURRENT`(规避
//    queue family ownership transfer;semaphore signal/wait 自带全量 memory
//    dependency,buffer 无 layout 面)⇒ `report.sharing_mode` 如实登记,**不充**
//    EXCLUSIVE release/acquire 语义绿(go 后实施窗再落成对 release/acquire,
//    RFC_DRAFT_RXS0239_amendment 修订行 3)。
//
// SAFETY(U26/U27 同族扩注,compute FFI 边界内,0 新 U 号):对上全 safe(pub 入口
// 无 unsafe 签名);内部契约同 U26/U27 —— `vulkan-1.dll` 经 loader 动态装载(缺失 →
// `Err` 非 panic);每个 #[repr(C)] 结构与 spec 逐字节对齐(size 锚单测);句柄线性
// 配对 create/destroy(末尾逆序销毁);`RURIX_VK_VALIDATION=1` 时 ERROR 级校验翻
// `Err`(fail-closed,vk_g31_mesh_bench 同律)。

// ── 常量(G37 W3 async;VK_KHR_timeline_semaphore 扩展号 207 → 1.2 core 收编编号
//    不变,经 SDK 1.3.296 vulkan_core.h 逐值核对)──
const ST_SEMAPHORE_TYPE_CREATE_INFO: u32 = 1_000_207_002;
const ST_TIMELINE_SEMAPHORE_SUBMIT_INFO: u32 = 1_000_207_003;
const ST_SEMAPHORE_WAIT_INFO: u32 = 1_000_207_004;
/// `VkSemaphoreType`:TIMELINE = 1(BINARY = 0)。
const SEMAPHORE_TYPE_TIMELINE: u32 = 1;
/// `VK_PIPELINE_STAGE_ALL_COMMANDS_BIT`(timeline wait 的 dst stage:段内含
/// timestamp/reset 命令,须整段被 wait 门住,否则段首时间戳先于 wait 触发,
/// 重叠率 evidence 失真)。
const PIPELINE_STAGE_ALL_COMMANDS: u32 = 0x0001_0000;
/// `VK_SHARING_MODE_CONCURRENT`(判档窗双队列臂 buffer 简化形态;§4 诚实登记)。
const SHARING_MODE_CONCURRENT: u32 = 1;
/// `VK_QUERY_RESULT_WAIT_BIT`(帧末 host wait 后读回;等待位仅兜底,预期即返)。
const QUERY_RESULT_WAIT_BIT: u32 = 0x2;

// ── 结构(repr(C) + size 锚;布局:sType@0 + pad → pNext@8 先例)──

/// `VkSemaphoreTypeCreateInfo`(1.2 core;挂 `SemaphoreCreateInfo.p_next`)。32B align 8。
#[repr(C)]
struct SemaphoreTypeCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    semaphore_type: u32,
    initial_value: u64,
}

/// `VkTimelineSemaphoreSubmitInfo`(挂 `SubmitInfo.p_next`;wait/signal 值数组与
/// SubmitInfo 的 semaphore 数组一一对应)。48B align 8。
#[repr(C)]
struct TimelineSemaphoreSubmitInfo {
    s_type: u32,
    p_next: *const c_void,
    wait_semaphore_value_count: u32,
    p_wait_semaphore_values: *const u64,
    signal_semaphore_value_count: u32,
    p_signal_semaphore_values: *const u64,
}

/// `VkSemaphoreWaitInfo`(host 侧 `vkWaitSemaphores`;flags=0 = wait all)。40B align 8。
#[repr(C)]
struct SemaphoreWaitInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: u32,
    semaphore_count: u32,
    p_semaphores: *const VkSemaphore,
    p_values: *const u64,
}

// ── fn 指针(1.2 core,device 级取址;缺失即能力探测已挡)──

/// 同 `vkWaitSemaphores`(device, &SemaphoreWaitInfo, timeout_ns)→ VkResult。
type FnWaitSemaphores =
    unsafe extern "system" fn(VkDevice, *const SemaphoreWaitInfo, u64) -> VkResult;
/// 同 `vkGetSemaphoreCounterValue`(device, semaphore, &mut value)→ VkResult。
type FnGetSemaphoreCounterValue =
    unsafe extern "system" fn(VkDevice, VkSemaphore, *mut u64) -> VkResult;

// ── 能力探测(#62 硬前置)──

/// 异步车道能力探测结果(instance/physical device 级,不建 device;驱动真值
/// **非 stable**,不进 canonical/golden)。
#[derive(Debug, Clone, PartialEq)]
pub struct AsyncQueueCaps {
    /// 物理设备名(驱动写入)。
    pub device_name: String,
    /// `VkPhysicalDeviceProperties::apiVersion`(打包值;timeline 须 ≥1.2)。
    pub api_version: u32,
    /// `timelineSemaphore` feature(1.2 core;单链查询,扩展缺席恒 false)。
    pub timeline_semaphore: bool,
    /// graphics 族 index(首个含 GRAPHICS 位,现选族逻辑同律)。
    pub graphics_family: u32,
    /// graphics 族 `timestamp_valid_bits`。
    pub graphics_timestamp_bits: u32,
    /// 首个 compute-only family(COMPUTE 且非 GRAPHICS;真异步族)。None = 仅共享族。
    pub compute_only_family: Option<u32>,
    /// compute-only 族 `timestamp_valid_bits`(0 = 该队列禁时间戳,重叠 evidence SKIP)。
    pub compute_only_timestamp_bits: u32,
    /// 与 graphics 异族但含 GRAPHICS 位的第二族(共享族常假重叠,如实登记 kind,
    /// 不作双队列判据)。
    pub distinct_compute_family: Option<u32>,
    /// `VkPhysicalDeviceLimits::timestampPeriod`(ns/tick;blob@720 实测)。
    pub timestamp_period_ns: f32,
}

impl AsyncQueueCaps {
    /// 双队列臂硬前置(PLAN §2.1-1):timeline + compute-only 族 + api ≥1.2。
    #[must_use]
    pub fn dual_queue_eligible(&self) -> bool {
        self.timeline_semaphore
            && self.compute_only_family.is_some()
            && self.api_version >= API_VERSION_1_2
    }
}

/// 异步车道能力探测(自建临时 instance,探完即毁;无 loader/无设备 → 确定性 `Err`,
/// dev-env degrade 由调用方三态裁决)。
pub fn probe_async_queue_caps() -> Result<AsyncQueueCaps, String> {
    let gipa = load_vulkan_loader()
        .ok_or("vulkan loader (vulkan-1.dll/libvulkan.so) 不可用(dev-env degrade)")?;
    // SAFETY: gipa 经 U26 loader 成功装载;instance 单点 create/destroy 配对,
    // 调用均为 1.0/1.1 core 已知 ABI 符号(probe_device_capability 同律)。
    unsafe { probe_async_queue_caps_inner(gipa) }
}

unsafe fn probe_async_queue_caps_inner(
    gipa: FnGetInstanceProcAddr,
) -> Result<AsyncQueueCaps, String> {
    let vk_create_instance: FnCreateInstance =
        cast_fn(gipa(std::ptr::null_mut(), c"vkCreateInstance".as_ptr()))
            .ok_or("缺 vkCreateInstance")?;
    let app = ApplicationInfo {
        s_type: ST_APPLICATION_INFO,
        p_next: std::ptr::null(),
        p_application_name: c"rurix-g37-async-caps".as_ptr(),
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
        enabled_layer_count: 0,
        pp_enabled_layer_names: std::ptr::null(),
        enabled_extension_count: 0,
        pp_enabled_extension_names: std::ptr::null(),
    };
    let mut instance: VkInstance = std::ptr::null_mut();
    if vk_create_instance(&ici, std::ptr::null(), &mut instance) != VK_SUCCESS {
        return Err("vkCreateInstance 失败".into());
    }
    let out = (|| {
        let vk_enum_pd: FnEnumeratePhysicalDevices =
            cast_fn(gipa(instance, c"vkEnumeratePhysicalDevices".as_ptr()))
                .ok_or("缺 vkEnumeratePhysicalDevices")?;
        let mut count = 0u32;
        vk_enum_pd(instance, &mut count, std::ptr::null_mut());
        if count == 0 {
            return Err("无 Vulkan 物理设备".to_owned());
        }
        let mut pds = vec![std::ptr::null_mut::<c_void>(); count as usize];
        vk_enum_pd(instance, &mut count, pds.as_mut_ptr());
        g37_async_caps_on_pd(gipa, instance, pds[0])
    })();
    let destroy_instance: Option<FnDestroyInstance> =
        cast_fn(gipa(instance, c"vkDestroyInstance".as_ptr()));
    if let Some(di) = destroy_instance {
        di(instance, std::ptr::null());
    }
    out
}

/// 单物理设备能力采集本体(探测面复用:props blob 身份/timestampPeriod@720 =
/// vk_g31_mesh_bench 同口径;timeline feature 单链 = 27656 段同构)。
unsafe fn g37_async_caps_on_pd(
    gipa: FnGetInstanceProcAddr,
    instance: VkInstance,
    pd: VkPhysicalDevice,
) -> Result<AsyncQueueCaps, String> {
    let vk_get_qf: FnGetPhysicalDeviceQueueFamilyProperties = cast_fn(gipa(
        instance,
        c"vkGetPhysicalDeviceQueueFamilyProperties".as_ptr(),
    ))
    .ok_or("缺 vkGetPhysicalDeviceQueueFamilyProperties")?;
    let vk_get_props: FnGetPhysicalDeviceProperties =
        cast_fn(gipa(instance, c"vkGetPhysicalDeviceProperties".as_ptr()))
            .ok_or("缺 vkGetPhysicalDeviceProperties")?;
    let vk_get_features2: FnGetPhysicalDeviceFeatures2 =
        cast_fn(gipa(instance, c"vkGetPhysicalDeviceFeatures2".as_ptr()))
            .ok_or("缺 vkGetPhysicalDeviceFeatures2(Vulkan 1.1 core)")?;

    // 身份 + timestampPeriod(blob 2048B 严格超集;deviceName@20..276 / period@720)。
    let mut props_blob = std::mem::zeroed::<PhysicalDevicePropertiesBlob>();
    vk_get_props(pd, &mut props_blob);
    let props_bytes: &[u8] = std::slice::from_raw_parts(
        (&raw const props_blob).cast::<u8>(),
        size_of::<PhysicalDevicePropertiesBlob>(),
    );
    let api_version = u32::from_le_bytes([
        props_bytes[0],
        props_bytes[1],
        props_bytes[2],
        props_bytes[3],
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

    // queue family 全枚举:graphics 首个含位 / compute-only = COMPUTE 且非 GRAPHICS /
    // distinct-compute = 与 graphics 异族但含 GRAPHICS 位(如实登记,不作判据)。
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
    let graphics_family = qfs
        .iter()
        .position(|q| q.queue_flags & QUEUE_GRAPHICS_BIT != 0)
        .ok_or("无 graphics queue family")? as u32;
    let compute_only_family = qfs
        .iter()
        .position(|q| {
            q.queue_flags & QUEUE_COMPUTE_BIT != 0 && q.queue_flags & QUEUE_GRAPHICS_BIT == 0
        })
        .map(|i| i as u32);
    let distinct_compute_family = qfs
        .iter()
        .enumerate()
        .position(|(i, q)| {
            i as u32 != graphics_family
                && q.queue_flags & QUEUE_COMPUTE_BIT != 0
                && q.queue_flags & QUEUE_GRAPHICS_BIT != 0
        })
        .map(|i| i as u32);
    let graphics_timestamp_bits = qfs[graphics_family as usize].timestamp_valid_bits;
    let compute_only_timestamp_bits = compute_only_family
        .map(|i| qfs[i as usize].timestamp_valid_bits)
        .unwrap_or(0);

    // timeline feature 单链查询(既有 PhysicalDeviceTimelineSemaphoreFeatures 复用;
    // 探测结果反哺 device 创建判据 —— PLAN §1.4「探测归探测」的补链)。
    let mut feat_tl = PhysicalDeviceTimelineSemaphoreFeatures {
        s_type: ST_PHYSICAL_DEVICE_TIMELINE_SEMAPHORE_FEATURES,
        p_next: std::ptr::null_mut(),
        timeline_semaphore: 0,
    };
    let mut feats2 = PhysicalDeviceFeatures2 {
        s_type: ST_PHYSICAL_DEVICE_FEATURES_2,
        p_next: (&raw mut feat_tl).cast::<c_void>(),
        features: std::mem::zeroed(),
    };
    vk_get_features2(pd, &mut feats2);

    Ok(AsyncQueueCaps {
        device_name,
        api_version,
        timeline_semaphore: feat_tl.timeline_semaphore != 0,
        graphics_family,
        graphics_timestamp_bits,
        compute_only_family,
        compute_only_timestamp_bits,
        distinct_compute_family,
        timestamp_period_ns,
    })
}

// ── timeline semaphore 创建 ──

/// 创建 timeline semaphore(p_next 挂 `SemaphoreTypeCreateInfo`;单条,值域由
/// harness 合法化产物给定)。
unsafe fn create_timeline_semaphore(
    device: VkDevice,
    create_sem: FnCreateSemaphore,
    initial_value: u64,
) -> Result<VkSemaphore, String> {
    let tci = SemaphoreTypeCreateInfo {
        s_type: ST_SEMAPHORE_TYPE_CREATE_INFO,
        p_next: std::ptr::null(),
        semaphore_type: SEMAPHORE_TYPE_TIMELINE,
        initial_value,
    };
    let sci = SemaphoreCreateInfo {
        s_type: ST_SEMAPHORE_CREATE_INFO,
        p_next: (&raw const tci).cast::<c_void>(),
        flags: 0,
    };
    let mut sem: VkSemaphore = VK_NULL_HANDLE;
    if create_sem(device, &sci, std::ptr::null(), &mut sem) != VK_SUCCESS {
        return Err("vkCreateSemaphore(timeline) 失败".into());
    }
    Ok(sem)
}

// ── 判档执行计划(harness 段切分/值域合法化产物的逐字消费面)──

/// 逻辑车道(与 rurix-render `QueueClass` 字面同构;vk 面独立定义避免跨 crate 反向依赖)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncLaneQueueKind {
    /// 图形车道(graphics family 队列)。
    Graphics,
    /// 异步 compute 车道(compute-only family 队列)。
    Compute,
}

/// 单次 dispatch 描述(判档 workload:确定性整数 kernel,3 SSBO + 12B push constant)。
#[derive(Debug, Clone)]
pub struct AsyncLaneDispatchSpec {
    /// 输出 buffer 下标(binding 0)。
    pub out_buf: usize,
    /// 输入 A buffer 下标(binding 1)。
    pub in_a: usize,
    /// 输入 B buffer 下标(binding 2)。
    pub in_b: usize,
    /// kernel seed(push constant)。
    pub seed: u32,
    /// kernel 迭代数(push constant;--scale 参数化面)。
    pub iters: u32,
}

/// 判档 pass(镜像图线性序;`dispatches` 依序录制,间隔全局 compute→compute 屏障)。
#[derive(Debug, Clone)]
pub struct AsyncLanePassSpec {
    /// pass 名(evidence 用)。
    pub name: String,
    /// 所属车道。
    pub queue: AsyncLaneQueueKind,
    /// 依序 dispatch 列表。
    pub dispatches: Vec<AsyncLaneDispatchSpec>,
}

/// 提交段(harness `plan_submission_segments` + 值域合法化产物;执行器逐字消费,
/// 禁二次推导)。wait/signal 为**合法化后**的 timeline 值(每帧加 base 偏移提交)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsyncLaneSubmitSegment {
    /// 段车道(提交到哪条队列)。
    pub queue: AsyncLaneQueueKind,
    /// 段内 pass 下标(入 `AsyncLanesPlan.passes`,线性序)。
    pub pass_indices: Vec<usize>,
    /// 段首 wait 的 timeline 值(合法化值;None = 无跨队列等待)。
    pub wait_value: Option<u64>,
    /// 段末 signal 的 timeline 值(合法化值,段线性序严格递增;None = 不发信号)。
    pub signal_value: Option<u64>,
}

/// 判档执行计划(全部 host 产物;`run_async_lanes` 消费)。
#[derive(Debug, Clone)]
pub struct AsyncLanesPlan<'a> {
    /// compute kernel SPIR-V(entry 恒 "main";3 SSBO + 12B push constant 布局)。
    pub spv: &'a [u32],
    /// buffer 总数(全部 `elem_count` 个 u32,host-visible+coherent,零初始化)。
    pub buffer_count: usize,
    /// 每 buffer 元素数(u32)。
    pub elem_count: u32,
    /// pass 表(线性序)。
    pub passes: &'a [AsyncLanePassSpec],
    /// 提交段(合法化产物)。
    pub segments: &'a [AsyncLaneSubmitSegment],
    /// 每帧 timeline 值域跨度(= 合法化信号值最大值,含 frame-end;帧 f 的点 v
    /// 以 `f*span+v` 提交,跨帧严格递增)。单队列臂 = 0。
    pub timeline_span: u64,
    /// true = 双队列臂(graphics + compute-only 两条队列 + timeline);
    /// false = 显式单队列形态(一条 graphics 队列、零 timeline、QueueWaitIdle)。
    pub dual_queue: bool,
    /// 计时帧数(measured;总帧 = 1 digest 帧 + warmup + frames)。
    pub frames: u32,
    /// warmup 帧数(丢弃)。
    pub warmup: u32,
}

/// 单帧 measured 样本(GPU timestamp 主口径 + 壁钟副口径;evidence-only 不进硬门)。
#[derive(Debug, Clone, Copy)]
pub struct AsyncLaneFrameSample {
    /// GPU 帧时长(全段 min(begin)→max(end),ns;timestamps 无效时 0)。
    pub frame_ns: u64,
    /// host 壁钟(提交→帧末等待返回,ns)。
    pub wall_ns: u64,
    /// graphics 段忙时合计(ns)。
    pub graphics_busy_ns: u64,
    /// 异步段忙时合计(ns)。
    pub async_busy_ns: u64,
    /// 异步段与 graphics 段时间窗交叠合计(ns)。
    pub overlap_ns: u64,
}

/// 判档执行报告(measured 真值 + 回读字节;digest 由调用方统一计算)。
#[derive(Debug)]
pub struct AsyncLanesReport {
    /// 设备名(驱动真值)。
    pub device_name: String,
    /// 设备 apiVersion(打包值)。
    pub api_version: u32,
    /// "dual" / "single"。
    pub queue_mode: &'static str,
    /// graphics family index。
    pub graphics_family: u32,
    /// compute-only family index(单队列臂 None)。
    pub compute_family: Option<u32>,
    /// "concurrent"(双队列臂)/ "exclusive"(单队列臂)——诚实登记,见模块头 §4。
    pub sharing_mode: &'static str,
    /// `timestampPeriod`(ns/tick)。
    pub timestamp_period_ns: f32,
    /// 时间戳面是否有效(所用族 `timestamp_valid_bits` 全非零且 period>0;
    /// false = 重叠 evidence SKIP,不充 measured)。
    pub timestamps_valid: bool,
    /// warmup 后逐帧样本。
    pub samples: Vec<AsyncLaneFrameSample>,
    /// 首帧(digest 帧)后全 buffer 回读(竞态金丝雀:与末帧比对)。
    pub readback_first: Vec<Vec<u8>>,
    /// 末帧后全 buffer 回读(等价门 digest 源)。
    pub readback_final: Vec<Vec<u8>>,
    /// 帧循环结束后 `vkGetSemaphoreCounterValue` 实测终值(单队列臂 None;
    /// evidence:应 = 总帧数 × span)。
    pub final_timeline_value: Option<u64>,
}

/// 判档 harness 双队列执行入口(G37 W3 async;职责见模块头)。
///
/// 提交前 fail-closed 结构核验(不重推导计划):下标越界 / 车道-模式矛盾 /
/// signal 值非严格递增(timeline 值回退在提交前拒,RFC_DRAFT 修订行 3 判据)。
pub fn run_async_lanes(plan: &AsyncLanesPlan<'_>) -> Result<AsyncLanesReport, String> {
    // ── 提交前结构核验(值域/下标;逐字消费不改写)──
    if plan.segments.is_empty() || plan.passes.is_empty() {
        return Err("空计划(segments/passes 不可为空)".into());
    }
    if plan.elem_count == 0 || plan.buffer_count == 0 {
        return Err("空 workload(elem_count/buffer_count 不可为 0)".into());
    }
    let mut last_signal = 0u64;
    for seg in plan.segments {
        for &pi in &seg.pass_indices {
            let p = plan
                .passes
                .get(pi)
                .ok_or_else(|| format!("段引用越界 pass {pi}"))?;
            if p.queue != seg.queue {
                return Err(format!("pass {pi} 车道与段车道不一致"));
            }
            for d in &p.dispatches {
                if d.out_buf >= plan.buffer_count
                    || d.in_a >= plan.buffer_count
                    || d.in_b >= plan.buffer_count
                {
                    return Err(format!("pass {pi} dispatch buffer 下标越界"));
                }
            }
        }
        if let Some(s) = seg.signal_value {
            if s == 0 || s <= last_signal {
                return Err(format!(
                    "signal 值非严格递增(提交前值回退拒):{s} ≤ {last_signal}"
                ));
            }
            last_signal = s;
        }
        if !plan.dual_queue {
            if seg.queue != AsyncLaneQueueKind::Graphics {
                return Err("单队列臂计划含异步段(须 enable_async=false 重编译)".into());
            }
            if seg.wait_value.is_some() || seg.signal_value.is_some() {
                return Err("单队列臂计划含 timeline 点(须重编译产零 fence 段)".into());
            }
        }
    }
    if plan.dual_queue && last_signal != plan.timeline_span {
        return Err(format!(
            "timeline_span({})与末 signal 值({last_signal})不一致",
            plan.timeline_span
        ));
    }
    let gipa = load_vulkan_loader()
        .ok_or("vulkan loader (vulkan-1.dll/libvulkan.so) 不可用(dev-env degrade)")?;
    // SAFETY: 见模块头 U26/U27 同族扩注;句柄生命周期由内部函数线性管理,末尾逆序销毁。
    unsafe { run_async_lanes_inner(gipa, plan) }
}

unsafe fn run_async_lanes_inner(
    gipa: FnGetInstanceProcAddr,
    plan: &AsyncLanesPlan<'_>,
) -> Result<AsyncLanesReport, String> {
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
        p_application_name: c"rurix-g37-async-lanes".as_ptr(),
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
    let vk_get_mem: FnGetPhysicalDeviceMemoryProperties = cast_fn(gipa(
        instance,
        c"vkGetPhysicalDeviceMemoryProperties".as_ptr(),
    ))
    .ok_or("缺 vkGetPhysicalDeviceMemoryProperties")?;
    let vk_create_device: FnCreateDevice =
        cast_fn(gipa(instance, c"vkCreateDevice".as_ptr())).ok_or("缺 vkCreateDevice")?;
    let vk_get_device_proc: FnGetDeviceProcAddr =
        cast_fn(gipa(instance, c"vkGetDeviceProcAddr".as_ptr())).ok_or("缺 vkGetDeviceProcAddr")?;

    // 校验层 messenger(ERROR 级翻 Err;vk_g31_mesh_bench 同律 fail-closed)。
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
                    // SAFETY: messenger 由本实例创建且仅销毁一次。
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

    // 能力面(同 instance 复采;探测结果反哺 device 创建)。
    let caps = match g37_async_caps_on_pd(gipa, instance, pd) {
        Ok(c) => c,
        Err(e) => {
            destroy_msgr!();
            vk_destroy_instance(instance, std::ptr::null());
            return Err(e);
        }
    };
    if plan.dual_queue && !caps.dual_queue_eligible() {
        destroy_msgr!();
        vk_destroy_instance(instance, std::ptr::null());
        return Err(format!(
            "双队列硬前置不满足(timeline={} compute_only={:?} api=0x{:x});\
             调用方须 enable_async=false 重编译走显式单队列回落",
            caps.timeline_semaphore, caps.compute_only_family, caps.api_version
        ));
    }

    // ── device 创建(harness 专用路径;镜像 TIRT 并行上下文先例,既有入口 0-byte):
    //    双队列臂 = graphics + compute-only 两条 DeviceQueueCreateInfo + p_next 挂
    //    timeline feature;单队列臂 = 一条 graphics 队列 + p_next null(既有形态)──
    let gfx_family = caps.graphics_family;
    let comp_family = caps.compute_only_family;
    let prio = [1.0f32];
    let dqci_gfx = DeviceQueueCreateInfo {
        s_type: ST_DEVICE_QUEUE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        queue_family_index: gfx_family,
        queue_count: 1,
        p_queue_priorities: prio.as_ptr(),
    };
    let dqci_comp = DeviceQueueCreateInfo {
        queue_family_index: comp_family.unwrap_or(0),
        ..dqci_gfx
    };
    let dqcis = [dqci_gfx, dqci_comp];
    let mut feat_tl_enable = PhysicalDeviceTimelineSemaphoreFeatures {
        s_type: ST_PHYSICAL_DEVICE_TIMELINE_SEMAPHORE_FEATURES,
        p_next: std::ptr::null_mut(),
        timeline_semaphore: 1,
    };
    let dci = DeviceCreateInfo {
        s_type: ST_DEVICE_CREATE_INFO,
        p_next: if plan.dual_queue {
            (&raw mut feat_tl_enable).cast::<c_void>()
        } else {
            std::ptr::null()
        },
        flags: 0,
        queue_create_info_count: if plan.dual_queue { 2 } else { 1 },
        p_queue_create_infos: dqcis.as_ptr(),
        enabled_layer_count: 0,
        pp_enabled_layer_names: std::ptr::null(),
        enabled_extension_count: 0,
        pp_enabled_extension_names: std::ptr::null(),
        p_enabled_features: std::ptr::null(),
    };
    let mut device: VkDevice = std::ptr::null_mut();
    if vk_create_device(pd, &dci, std::ptr::null(), &mut device) != VK_SUCCESS {
        destroy_msgr!();
        vk_destroy_instance(instance, std::ptr::null());
        return Err("vkCreateDevice(async lanes) 失败".into());
    }

    let mut out = async_lanes_body(vk_get_device_proc, device, pd, vk_get_mem, &caps, plan);
    if validation && validation_error.load(std::sync::atomic::Ordering::Relaxed) {
        out = Err("VK_LAYER_KHRONOS_validation 报 ERROR 级校验错误(fail-closed)".into());
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

/// 每队列 timestamp 区间(query 序对:段 k → (2k, 2k+1))。
struct G37QueueLane {
    queue: VkQueue,
    cmd_pool: VkCommandPool,
    query_pool: VkQueryPool,
    /// 本队列段数(query_pool 容量 = 2×seg_count)。
    seg_count: u32,
}

#[allow(clippy::too_many_arguments)]
unsafe fn async_lanes_body(
    gdpa: FnGetDeviceProcAddr,
    device: VkDevice,
    pd: VkPhysicalDevice,
    vk_get_mem: FnGetPhysicalDeviceMemoryProperties,
    caps: &AsyncQueueCaps,
    plan: &AsyncLanesPlan<'_>,
) -> Result<AsyncLanesReport, String> {
    macro_rules! dp {
        ($name:literal, $ty:ty) => {
            // SAFETY: 核心 device 符号缺失即 Err(fail-closed)。
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
    let cmd_barrier: FnCmdPipelineBarrier = dp!(c"vkCmdPipelineBarrier", FnCmdPipelineBarrier);
    let queue_submit: FnQueueSubmit = dp!(c"vkQueueSubmit", FnQueueSubmit);
    let queue_wait: FnQueueWaitIdle = dp!(c"vkQueueWaitIdle", FnQueueWaitIdle);
    let create_qp: FnCreateQueryPool = dp!(c"vkCreateQueryPool", FnCreateQueryPool);
    let destroy_qp: FnDestroyQueryPool = dp!(c"vkDestroyQueryPool", FnDestroyQueryPool);
    let cmd_reset_qp: FnCmdResetQueryPool = dp!(c"vkCmdResetQueryPool", FnCmdResetQueryPool);
    let cmd_write_ts: FnCmdWriteTimestamp = dp!(c"vkCmdWriteTimestamp", FnCmdWriteTimestamp);
    let get_qp_results: FnGetQueryPoolResults =
        dp!(c"vkGetQueryPoolResults", FnGetQueryPoolResults);
    let create_sem: FnCreateSemaphore = dp!(c"vkCreateSemaphore", FnCreateSemaphore);
    let destroy_sem: FnDestroySemaphore = dp!(c"vkDestroySemaphore", FnDestroySemaphore);
    // timeline host wait / counter(1.2 core;仅双队列臂取址,单臂不依赖)。
    let (wait_semaphores, get_sem_counter): (
        Option<FnWaitSemaphores>,
        Option<FnGetSemaphoreCounterValue>,
    ) = if plan.dual_queue {
        (
            Some(dp!(c"vkWaitSemaphores", FnWaitSemaphores)),
            Some(dp!(
                c"vkGetSemaphoreCounterValue",
                FnGetSemaphoreCounterValue
            )),
        )
    } else {
        (None, None)
    };

    let gfx_family = caps.graphics_family;
    let comp_family_idx = caps.compute_only_family.unwrap_or(0);
    let mut gfx_queue: VkQueue = std::ptr::null_mut();
    get_queue(device, gfx_family, 0, &mut gfx_queue);
    let mut comp_queue: VkQueue = std::ptr::null_mut();
    if plan.dual_queue {
        get_queue(device, comp_family_idx, 0, &mut comp_queue);
    }
    let mut memprops = std::mem::zeroed::<PhysicalDeviceMemoryProperties>();
    vk_get_mem(pd, &mut memprops);

    // 时间戳有效面(所用族 bits 全非零 + period 有效;无效 = SKIP 不充 measured)。
    let uses_compute_lane = plan
        .segments
        .iter()
        .any(|s| s.queue == AsyncLaneQueueKind::Compute);
    let timestamps_valid = caps.timestamp_period_ns > 0.0
        && caps.timestamp_period_ns.is_finite()
        && caps.graphics_timestamp_bits != 0
        && (!uses_compute_lane || caps.compute_only_timestamp_bits != 0);

    let buf_bytes = (plan.elem_count as u64) * 4;
    let concurrent_families = [gfx_family, comp_family_idx];
    let total_dispatches: usize = plan.passes.iter().map(|p| p.dispatches.len()).sum();

    // ── 句柄表(失败统一 'body 跳出,末尾逆序销毁)──
    let mut bufs: Vec<(VkBuffer, VkDeviceMemory)> = Vec::new();
    let mut shader: VkShaderModule = VK_NULL_HANDLE;
    let mut dsl: VkDescriptorSetLayout = VK_NULL_HANDLE;
    let mut pl: VkPipelineLayout = VK_NULL_HANDLE;
    let mut pipe: VkPipeline = VK_NULL_HANDLE;
    let mut dpool: VkDescriptorPool = VK_NULL_HANDLE;
    let mut timeline: VkSemaphore = VK_NULL_HANDLE;
    let mut lanes: Vec<G37QueueLane> = Vec::new();

    let result: Result<AsyncLanesReport, String> = 'body: {
        // buffer 建面 + 零初始化(双队列臂 CONCURRENT sharing,诚实登记)。
        for _ in 0..plan.buffer_count {
            let bci = BufferCreateInfo {
                s_type: ST_BUFFER_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                size: buf_bytes.max(4),
                usage: BUFFER_USAGE_STORAGE_BUFFER,
                sharing_mode: if plan.dual_queue {
                    SHARING_MODE_CONCURRENT
                } else {
                    SHARING_MODE_EXCLUSIVE
                },
                queue_family_index_count: if plan.dual_queue { 2 } else { 0 },
                p_queue_family_indices: if plan.dual_queue {
                    concurrent_families.as_ptr()
                } else {
                    std::ptr::null()
                },
            };
            let mut buffer: VkBuffer = VK_NULL_HANDLE;
            if create_buffer(device, &bci, std::ptr::null(), &mut buffer) != VK_SUCCESS {
                break 'body Err("vkCreateBuffer 失败".into());
            }
            let mut req = std::mem::zeroed::<MemoryRequirements>();
            buf_mem_req(device, buffer, &mut req);
            let Some(mt) = pick_mem_type(
                &memprops,
                req.memory_type_bits,
                MEM_HOST_VISIBLE | MEM_HOST_COHERENT,
            ) else {
                destroy_buffer(device, buffer, std::ptr::null());
                break 'body Err("无 host-visible+coherent 内存类型".into());
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
                break 'body Err("vkAllocateMemory 失败".into());
            }
            bind_buf(device, buffer, mem, 0);
            let mut ptr: *mut c_void = std::ptr::null_mut();
            if map_mem(device, mem, 0, WHOLE_SIZE, 0, &mut ptr) != VK_SUCCESS || ptr.is_null() {
                destroy_buffer(device, buffer, std::ptr::null());
                free_mem(device, mem, std::ptr::null());
                break 'body Err("vkMapMemory(零初始化) 失败".into());
            }
            // SAFETY: 映射 ≥ buf_bytes 字节;确定性零初始化(两臂同起点)。
            std::ptr::write_bytes(ptr.cast::<u8>(), 0, buf_bytes as usize);
            unmap_mem(device, mem);
            bufs.push((buffer, mem));
        }

        // shader module(entry 恒 "main")。
        let smci = ShaderModuleCreateInfo {
            s_type: ST_SHADER_MODULE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            code_size: plan.spv.len() * 4,
            p_code: plan.spv.as_ptr(),
        };
        if create_shader(device, &smci, std::ptr::null(), &mut shader) != VK_SUCCESS {
            break 'body Err("vkCreateShaderModule 失败".into());
        }

        // descriptor set layout(3 SSBO)+ pipeline layout(12B push constant)。
        let bindings: [DescriptorSetLayoutBinding; 3] = std::array::from_fn(|i| {
            DescriptorSetLayoutBinding {
                binding: i as u32,
                descriptor_type: DESCRIPTOR_TYPE_STORAGE_BUFFER,
                descriptor_count: 1,
                stage_flags: SHADER_STAGE_COMPUTE,
                p_immutable_samplers: std::ptr::null(),
            }
        });
        let dslci = DescriptorSetLayoutCreateInfo {
            s_type: ST_DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            binding_count: 3,
            p_bindings: bindings.as_ptr(),
        };
        if create_dsl(device, &dslci, std::ptr::null(), &mut dsl) != VK_SUCCESS {
            break 'body Err("vkCreateDescriptorSetLayout 失败".into());
        }
        let pcr = PushConstantRange {
            stage_flags: SHADER_STAGE_COMPUTE,
            offset: 0,
            size: 12,
        };
        let plci = PipelineLayoutCreateInfo {
            s_type: ST_PIPELINE_LAYOUT_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            set_layout_count: 1,
            p_set_layouts: &dsl,
            push_constant_range_count: 1,
            p_push_constant_ranges: &pcr,
        };
        if create_pl(device, &plci, std::ptr::null(), &mut pl) != VK_SUCCESS {
            break 'body Err("vkCreatePipelineLayout 失败".into());
        }
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
                p_name: c"main".as_ptr(),
                p_specialization_info: std::ptr::null(),
            },
            layout: pl,
            base_pipeline_handle: VK_NULL_HANDLE,
            base_pipeline_index: -1,
        };
        if create_cp(device, VK_NULL_HANDLE, 1, &cpci, std::ptr::null(), &mut pipe) != VK_SUCCESS {
            break 'body Err("vkCreateComputePipelines 失败".into());
        }

        // descriptor pool + 每 dispatch 一个 set(预烘焙,录制期零更新)。
        let pool_size = DescriptorPoolSize {
            descriptor_type: DESCRIPTOR_TYPE_STORAGE_BUFFER,
            descriptor_count: (3 * total_dispatches.max(1)) as u32,
        };
        let dpci = DescriptorPoolCreateInfo {
            s_type: ST_DESCRIPTOR_POOL_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            max_sets: total_dispatches.max(1) as u32,
            pool_size_count: 1,
            p_pool_sizes: &pool_size,
        };
        if create_dp(device, &dpci, std::ptr::null(), &mut dpool) != VK_SUCCESS {
            break 'body Err("vkCreateDescriptorPool 失败".into());
        }
        let set_layouts = vec![dsl; total_dispatches];
        let dsai = DescriptorSetAllocateInfo {
            s_type: ST_DESCRIPTOR_SET_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            descriptor_pool: dpool,
            descriptor_set_count: total_dispatches as u32,
            p_set_layouts: set_layouts.as_ptr(),
        };
        let mut sets: Vec<VkDescriptorSet> = vec![VK_NULL_HANDLE; total_dispatches];
        if alloc_ds(device, &dsai, sets.as_mut_ptr()) != VK_SUCCESS {
            break 'body Err("vkAllocateDescriptorSets 失败".into());
        }
        // 写 descriptor(infos 预分配定容,指针稳定)。
        let mut buf_infos: Vec<DescriptorBufferInfo> = Vec::with_capacity(3 * total_dispatches);
        let mut writes: Vec<WriteDescriptorSet> = Vec::with_capacity(3 * total_dispatches);
        let mut set_cursor = 0usize;
        for p in plan.passes {
            for d in &p.dispatches {
                let set = sets[set_cursor];
                set_cursor += 1;
                for (binding, bi) in [(0u32, d.out_buf), (1, d.in_a), (2, d.in_b)] {
                    buf_infos.push(DescriptorBufferInfo {
                        buffer: bufs[bi].0,
                        offset: 0,
                        range: WHOLE_SIZE,
                    });
                    writes.push(WriteDescriptorSet {
                        s_type: ST_WRITE_DESCRIPTOR_SET,
                        p_next: std::ptr::null(),
                        dst_set: set,
                        dst_binding: binding,
                        dst_array_element: 0,
                        descriptor_count: 1,
                        descriptor_type: DESCRIPTOR_TYPE_STORAGE_BUFFER,
                        p_image_info: std::ptr::null(),
                        p_buffer_info: &buf_infos[buf_infos.len() - 1],
                        p_texel_buffer_view: std::ptr::null(),
                    });
                }
            }
        }
        update_ds(device, writes.len() as u32, writes.as_ptr(), 0, std::ptr::null());

        // pass → dispatch 起始 set 序(线性序前缀和)。
        let mut pass_set_base: Vec<usize> = Vec::with_capacity(plan.passes.len());
        let mut acc = 0usize;
        for p in plan.passes {
            pass_set_base.push(acc);
            acc += p.dispatches.len();
        }

        // ── 每队列 lane:command pool + query pool(2×段数)──
        let gfx_seg_count = plan
            .segments
            .iter()
            .filter(|s| s.queue == AsyncLaneQueueKind::Graphics)
            .count() as u32;
        let comp_seg_count = plan.segments.len() as u32 - gfx_seg_count;
        let mut make_lane = |family: u32, queue: VkQueue, seg_count: u32| -> Result<(), String> {
            let cpci2 = CommandPoolCreateInfo {
                s_type: ST_COMMAND_POOL_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                queue_family_index: family,
            };
            let mut pool: VkCommandPool = VK_NULL_HANDLE;
            if create_cmdpool(device, &cpci2, std::ptr::null(), &mut pool) != VK_SUCCESS {
                return Err("vkCreateCommandPool 失败".into());
            }
            let mut qp: VkQueryPool = std::ptr::null_mut();
            if timestamps_valid && seg_count > 0 {
                let qpci = QueryPoolCreateInfo {
                    s_type: ST_QUERY_POOL_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: 0,
                    query_type: QUERY_TYPE_TIMESTAMP,
                    query_count: 2 * seg_count,
                    pipeline_statistics: 0,
                };
                if create_qp(device, &qpci, std::ptr::null(), &mut qp) != VK_SUCCESS {
                    destroy_cmdpool(device, pool, std::ptr::null());
                    return Err("vkCreateQueryPool 失败".into());
                }
            }
            lanes.push(G37QueueLane {
                queue,
                cmd_pool: pool,
                query_pool: qp,
                seg_count,
            });
            Ok(())
        };
        if let Err(e) = make_lane(gfx_family, gfx_queue, gfx_seg_count) {
            break 'body Err(e);
        }
        if plan.dual_queue
            && let Err(e) = make_lane(comp_family_idx, comp_queue, comp_seg_count)
        {
            break 'body Err(e);
        }
        let lane_of = |q: AsyncLaneQueueKind| -> usize {
            match q {
                AsyncLaneQueueKind::Graphics => 0,
                AsyncLaneQueueKind::Compute => 1,
            }
        };

        // ── 逐段录制(一次录制,逐帧重复提交;段 k(队列内序)→ query (2k, 2k+1))──
        let cbbi = CommandBufferBeginInfo {
            s_type: ST_COMMAND_BUFFER_BEGIN_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            p_inheritance_info: std::ptr::null(),
        };
        let barrier = MemoryBarrier {
            s_type: ST_MEMORY_BARRIER,
            p_next: std::ptr::null(),
            src_access_mask: ACCESS_SHADER_WRITE,
            dst_access_mask: ACCESS_SHADER_READ | ACCESS_SHADER_WRITE,
        };
        let groups_x = plan.elem_count.div_ceil(256);
        let mut seg_cmds: Vec<VkCommandBuffer> = Vec::with_capacity(plan.segments.len());
        let mut lane_seg_seen = [0u32; 2];
        for seg in plan.segments {
            let li = lane_of(seg.queue);
            let lane = &lanes[li];
            let k = lane_seg_seen[li];
            lane_seg_seen[li] += 1;
            let cbai = CommandBufferAllocateInfo {
                s_type: ST_COMMAND_BUFFER_ALLOCATE_INFO,
                p_next: std::ptr::null(),
                command_pool: lane.cmd_pool,
                level: CMD_BUFFER_LEVEL_PRIMARY,
                command_buffer_count: 1,
            };
            let mut cmd: VkCommandBuffer = std::ptr::null_mut();
            if alloc_cmd(device, &cbai, &mut cmd) != VK_SUCCESS || cmd.is_null() {
                break 'body Err("vkAllocateCommandBuffers 失败".into());
            }
            begin_cmd(cmd, &cbbi);
            if timestamps_valid {
                if k == 0 {
                    // 本队列首段负责整 pool reset(同队列提交序先于后段写入)。
                    cmd_reset_qp(cmd, lane.query_pool, 0, 2 * lane.seg_count);
                }
                cmd_write_ts(cmd, PIPELINE_STAGE_TOP_OF_PIPE, lane.query_pool, 2 * k);
            }
            cmd_bind_pipe(cmd, PIPELINE_BIND_POINT_COMPUTE, pipe);
            for &pi in &seg.pass_indices {
                let p = &plan.passes[pi];
                for (di, d) in p.dispatches.iter().enumerate() {
                    // 全局 memory barrier(compute→compute):序化同队列前序写
                    //(含跨段,提交序作用域);跨队列可见性由 timeline signal/wait
                    // 全量 memory dependency 承担(CONCURRENT sharing,无 layout 面)。
                    cmd_barrier(
                        cmd,
                        PIPELINE_STAGE_COMPUTE_SHADER,
                        PIPELINE_STAGE_COMPUTE_SHADER,
                        0,
                        1,
                        (&raw const barrier).cast::<c_void>(),
                        0,
                        std::ptr::null(),
                        0,
                        std::ptr::null(),
                    );
                    let set = sets[pass_set_base[pi] + di];
                    cmd_bind_ds(cmd, PIPELINE_BIND_POINT_COMPUTE, pl, 0, 1, &set, 0, std::ptr::null());
                    let pc: [u32; 3] = [plan.elem_count, d.seed, d.iters];
                    cmd_push(
                        cmd,
                        pl,
                        SHADER_STAGE_COMPUTE,
                        0,
                        12,
                        pc.as_ptr().cast::<c_void>(),
                    );
                    cmd_dispatch(cmd, groups_x, 1, 1);
                }
            }
            if timestamps_valid {
                cmd_write_ts(cmd, PIPELINE_STAGE_BOTTOM_OF_PIPE, lane.query_pool, 2 * k + 1);
            }
            if end_cmd(cmd) != VK_SUCCESS {
                break 'body Err("vkEndCommandBuffer 失败".into());
            }
            seg_cmds.push(cmd);
        }

        // timeline semaphore(双队列臂;初值 0,值域 = 帧 f × span + 合法化点)。
        if plan.dual_queue {
            match create_timeline_semaphore(device, create_sem, 0) {
                Ok(s) => timeline = s,
                Err(e) => break 'body Err(e),
            }
        }

        // ── 帧循环:帧 0 = digest 帧(竞态金丝雀),再 warmup + frames ──
        let total_frames = 1 + plan.warmup + plan.frames;
        let mut samples: Vec<AsyncLaneFrameSample> = Vec::with_capacity(plan.frames as usize);
        let mut readback_first: Vec<Vec<u8>> = Vec::new();
        let mut ts_gfx: Vec<u64> = vec![0; 2 * gfx_seg_count as usize];
        let mut ts_comp: Vec<u64> = vec![0; 2 * comp_seg_count as usize];
        let read_all = |map_mem: &FnMapMemory,
                        unmap_mem: &FnUnmapMemory,
                        bufs: &[(VkBuffer, VkDeviceMemory)]|
         -> Result<Vec<Vec<u8>>, String> {
            let mut out = Vec::with_capacity(bufs.len());
            for &(_, mem) in bufs {
                let mut ptr: *mut c_void = std::ptr::null_mut();
                // SAFETY: mem host-visible+coherent;帧末 host 等待后写入已可见。
                if map_mem(device, mem, 0, WHOLE_SIZE, 0, &mut ptr) != VK_SUCCESS || ptr.is_null()
                {
                    return Err("vkMapMemory(readback) 失败".into());
                }
                let mut bytes = vec![0u8; buf_bytes as usize];
                // SAFETY: 映射区 ≥ buf_bytes;逐字节拷出后 unmap。
                std::ptr::copy_nonoverlapping(
                    ptr.cast::<u8>(),
                    bytes.as_mut_ptr(),
                    buf_bytes as usize,
                );
                unmap_mem(device, mem);
                out.push(bytes);
            }
            Ok(out)
        };
        for fi in 0..total_frames {
            let base = u64::from(fi) * plan.timeline_span;
            let t0 = std::time::Instant::now();
            for (si, seg) in plan.segments.iter().enumerate() {
                let lane = &lanes[lane_of(seg.queue)];
                let wait_vals: [u64; 1] = [base + seg.wait_value.unwrap_or(0)];
                let sig_vals: [u64; 1] = [base + seg.signal_value.unwrap_or(0)];
                let wait_sems: [VkSemaphore; 1] = [timeline];
                let sig_sems: [VkSemaphore; 1] = [timeline];
                let wait_stage: [VkFlags; 1] = [PIPELINE_STAGE_ALL_COMMANDS];
                let n_wait: u32 = u32::from(seg.wait_value.is_some());
                let n_sig: u32 = u32::from(seg.signal_value.is_some());
                let tsi = TimelineSemaphoreSubmitInfo {
                    s_type: ST_TIMELINE_SEMAPHORE_SUBMIT_INFO,
                    p_next: std::ptr::null(),
                    wait_semaphore_value_count: n_wait,
                    p_wait_semaphore_values: wait_vals.as_ptr(),
                    signal_semaphore_value_count: n_sig,
                    p_signal_semaphore_values: sig_vals.as_ptr(),
                };
                let si_info = SubmitInfo {
                    s_type: ST_SUBMIT_INFO,
                    p_next: if plan.dual_queue && (n_wait + n_sig) > 0 {
                        (&raw const tsi).cast::<c_void>()
                    } else {
                        std::ptr::null()
                    },
                    wait_semaphore_count: n_wait,
                    p_wait_semaphores: if n_wait > 0 {
                        wait_sems.as_ptr()
                    } else {
                        std::ptr::null()
                    },
                    p_wait_dst_stage_mask: if n_wait > 0 {
                        wait_stage.as_ptr()
                    } else {
                        std::ptr::null()
                    },
                    command_buffer_count: 1,
                    p_command_buffers: &seg_cmds[si],
                    signal_semaphore_count: n_sig,
                    p_signal_semaphores: if n_sig > 0 {
                        sig_sems.as_ptr()
                    } else {
                        std::ptr::null()
                    },
                };
                let sr = queue_submit(lane.queue, 1, &si_info, VK_NULL_HANDLE);
                if sr != VK_SUCCESS {
                    break 'body Err(format!("vkQueueSubmit(段 {si}) rc={sr}"));
                }
            }
            // 帧末等待:双队列 = host vkWaitSemaphores 终值(替代 QueueWaitIdle);
            // 单队列 = QueueWaitIdle(既有形态)。
            if plan.dual_queue {
                let end_val = [base + plan.timeline_span];
                let sems = [timeline];
                let swi = SemaphoreWaitInfo {
                    s_type: ST_SEMAPHORE_WAIT_INFO,
                    p_next: std::ptr::null(),
                    flags: 0,
                    semaphore_count: 1,
                    p_semaphores: sems.as_ptr(),
                    p_values: end_val.as_ptr(),
                };
                let ws = wait_semaphores.expect("双队列臂已取址");
                let wr = ws(device, &swi, 10_000_000_000);
                if wr != VK_SUCCESS {
                    break 'body Err(format!(
                        "vkWaitSemaphores(帧 {fi} 终值 {}) rc={wr}(疑似死锁/值域错误)",
                        end_val[0]
                    ));
                }
            } else {
                let wr = queue_wait(gfx_queue);
                if wr != VK_SUCCESS {
                    break 'body Err(format!("vkQueueWaitIdle(帧 {fi}) rc={wr}"));
                }
            }
            let wall_ns = t0.elapsed().as_nanos() as u64;

            // 时间戳读回 + 区间统计(evidence;fi 计入判据窗才留样:
            // 帧 0 = digest 帧,1..=warmup = 预热,其后为 measured)。
            if fi > plan.warmup {
                let mut sample = AsyncLaneFrameSample {
                    frame_ns: 0,
                    wall_ns,
                    graphics_busy_ns: 0,
                    async_busy_ns: 0,
                    overlap_ns: 0,
                };
                if timestamps_valid {
                    let read_lane = |lane: &G37QueueLane,
                                     out: &mut [u64]|
     -> Result<(), String> {
                        if lane.seg_count == 0 {
                            return Ok(());
                        }
                        let qr = get_qp_results(
                            device,
                            lane.query_pool,
                            0,
                            2 * lane.seg_count,
                            out.len() * 8,
                            out.as_mut_ptr().cast::<c_void>(),
                            8,
                            QUERY_RESULT_64_BIT | QUERY_RESULT_WAIT_BIT,
                        );
                        if qr != VK_SUCCESS {
                            return Err(format!("vkGetQueryPoolResults rc={qr}"));
                        }
                        Ok(())
                    };
                    if let Err(e) = read_lane(&lanes[0], &mut ts_gfx) {
                        break 'body Err(e);
                    }
                    if plan.dual_queue
                        && let Err(e) = read_lane(&lanes[1], &mut ts_comp)
                    {
                        break 'body Err(e);
                    }
                    let period = f64::from(caps.timestamp_period_ns);
                    let to_ns = |ticks: u64| -> u64 { (ticks as f64 * period) as u64 };
                    let ivals = |ts: &[u64]| -> Vec<(u64, u64)> {
                        ts.chunks_exact(2).map(|c| (c[0], c[1])).collect()
                    };
                    let gfx_iv = ivals(&ts_gfx);
                    let comp_iv = ivals(&ts_comp);
                    let mut min_b = u64::MAX;
                    let mut max_e = 0u64;
                    for &(b, e) in gfx_iv.iter().chain(comp_iv.iter()) {
                        min_b = min_b.min(b);
                        max_e = max_e.max(e);
                    }
                    sample.frame_ns = to_ns(max_e.saturating_sub(min_b));
                    sample.graphics_busy_ns =
                        to_ns(gfx_iv.iter().map(|&(b, e)| e.saturating_sub(b)).sum());
                    sample.async_busy_ns =
                        to_ns(comp_iv.iter().map(|&(b, e)| e.saturating_sub(b)).sum());
                    let mut overlap_ticks = 0u64;
                    for &(gb, ge) in &gfx_iv {
                        for &(cb, ce) in &comp_iv {
                            let lo = gb.max(cb);
                            let hi = ge.min(ce);
                            overlap_ticks += hi.saturating_sub(lo);
                        }
                    }
                    sample.overlap_ns = to_ns(overlap_ticks);
                }
                samples.push(sample);
            }
            if fi == 0 {
                match read_all(&map_mem, &unmap_mem, &bufs) {
                    Ok(r) => readback_first = r,
                    Err(e) => break 'body Err(e),
                }
            }
        }

        // 末帧回读 + timeline 终值 evidence。
        let readback_final = match read_all(&map_mem, &unmap_mem, &bufs) {
            Ok(r) => r,
            Err(e) => break 'body Err(e),
        };
        let final_timeline_value = if plan.dual_queue {
            let mut v = 0u64;
            let gc = get_sem_counter.expect("双队列臂已取址");
            if gc(device, timeline, &mut v) != VK_SUCCESS {
                break 'body Err("vkGetSemaphoreCounterValue 失败".into());
            }
            Some(v)
        } else {
            None
        };

        break 'body Ok(AsyncLanesReport {
            device_name: caps.device_name.clone(),
            api_version: caps.api_version,
            queue_mode: if plan.dual_queue { "dual" } else { "single" },
            graphics_family: gfx_family,
            compute_family: if plan.dual_queue {
                caps.compute_only_family
            } else {
                None
            },
            sharing_mode: if plan.dual_queue {
                "concurrent"
            } else {
                "exclusive"
            },
            timestamp_period_ns: caps.timestamp_period_ns,
            timestamps_valid,
            samples,
            readback_first,
            readback_final,
            final_timeline_value,
        });
    };

    // ── 逆序销毁(句柄线性配对;cmd buffer 随 pool 释放,set 随 dpool 释放)──
    if timeline != VK_NULL_HANDLE {
        // SAFETY: 配对销毁(队列此刻空闲:成功路径帧末已等待;失败路径兜底等待)。
        queue_wait(gfx_queue);
        if plan.dual_queue && !comp_queue.is_null() {
            queue_wait(comp_queue);
        }
        destroy_sem(device, timeline, std::ptr::null());
    } else if result.is_err() {
        // 失败路径兜底:确保无 in-flight 工作再销毁资源。
        queue_wait(gfx_queue);
        if plan.dual_queue && !comp_queue.is_null() {
            queue_wait(comp_queue);
        }
    }
    for lane in lanes.iter().rev() {
        if !lane.query_pool.is_null() {
            // SAFETY: 配对销毁。
            destroy_qp(device, lane.query_pool, std::ptr::null());
        }
        if lane.cmd_pool != VK_NULL_HANDLE {
            // SAFETY: 配对销毁(其下 cmd 随之释放)。
            destroy_cmdpool(device, lane.cmd_pool, std::ptr::null());
        }
    }
    if dpool != VK_NULL_HANDLE {
        // SAFETY: 配对销毁(set 随 pool 释放)。
        destroy_dp(device, dpool, std::ptr::null());
    }
    if pipe != VK_NULL_HANDLE {
        // SAFETY: 配对销毁。
        destroy_pipe(device, pipe, std::ptr::null());
    }
    if pl != VK_NULL_HANDLE {
        // SAFETY: 配对销毁。
        destroy_pl(device, pl, std::ptr::null());
    }
    if dsl != VK_NULL_HANDLE {
        // SAFETY: 配对销毁。
        destroy_dsl(device, dsl, std::ptr::null());
    }
    if shader != VK_NULL_HANDLE {
        // SAFETY: 配对销毁。
        destroy_shader(device, shader, std::ptr::null());
    }
    for &(buffer, mem) in bufs.iter().rev() {
        // SAFETY: 配对销毁。
        destroy_buffer(device, buffer, std::ptr::null());
        free_mem(device, mem, std::ptr::null());
    }
    result
}

// ── G37 W3 async 锚单测(sType/size 经 SDK 1.3.296 vulkan_core.h 逐值核对)──
#[cfg(test)]
mod g37_async_lanes_tests {
    use super::*;

    /// FFI 布局锚:三结构 size(sType@0+pad → pNext@8 先例)+ sType/枚举值 +
    /// stage/sharing 常量(PATCH_PROPOSAL §B 锚单测字面)。
    #[test]
    fn g37_timeline_ffi_layout_anchors() {
        assert_eq!(size_of::<SemaphoreTypeCreateInfo>(), 32);
        assert_eq!(align_of::<SemaphoreTypeCreateInfo>(), 8);
        assert_eq!(size_of::<TimelineSemaphoreSubmitInfo>(), 48);
        assert_eq!(align_of::<TimelineSemaphoreSubmitInfo>(), 8);
        assert_eq!(size_of::<SemaphoreWaitInfo>(), 40);
        assert_eq!(align_of::<SemaphoreWaitInfo>(), 8);
        // sType(扩展号 207 → 1.2 core 收编编号不变)。
        assert_eq!(ST_SEMAPHORE_TYPE_CREATE_INFO, 1_000_207_002);
        assert_eq!(ST_TIMELINE_SEMAPHORE_SUBMIT_INFO, 1_000_207_003);
        assert_eq!(ST_SEMAPHORE_WAIT_INFO, 1_000_207_004);
        assert_eq!(SEMAPHORE_TYPE_TIMELINE, 1);
        assert_eq!(PIPELINE_STAGE_ALL_COMMANDS, 0x0001_0000);
        assert_eq!(SHARING_MODE_CONCURRENT, 1);
        assert_eq!(QUERY_RESULT_WAIT_BIT, 0x2);
    }

    /// 提交前 fail-closed 核验:值回退 / 车道矛盾 / 下标越界确定性拒
    /// (RFC_DRAFT 修订行 3「提交前 validator RED」的 vk 侧最小面)。
    #[test]
    fn g37_run_plan_precheck_rejects() {
        let spv: Vec<u32> = vec![0x0723_0203];
        let pass = AsyncLanePassSpec {
            name: "p".into(),
            queue: AsyncLaneQueueKind::Graphics,
            dispatches: vec![AsyncLaneDispatchSpec {
                out_buf: 0,
                in_a: 0,
                in_b: 0,
                seed: 1,
                iters: 1,
            }],
        };
        // 值回退(signal 非严格递增)。
        let segs_rollback = vec![
            AsyncLaneSubmitSegment {
                queue: AsyncLaneQueueKind::Graphics,
                pass_indices: vec![0],
                wait_value: None,
                signal_value: Some(2),
            },
            AsyncLaneSubmitSegment {
                queue: AsyncLaneQueueKind::Graphics,
                pass_indices: vec![0],
                wait_value: None,
                signal_value: Some(1),
            },
        ];
        let plan = AsyncLanesPlan {
            spv: &spv,
            buffer_count: 1,
            elem_count: 4,
            passes: std::slice::from_ref(&pass),
            segments: &segs_rollback,
            timeline_span: 2,
            dual_queue: true,
            frames: 1,
            warmup: 0,
        };
        let e = run_async_lanes(&plan).expect_err("值回退应拒");
        assert!(e.contains("值回退"), "错误应指明值回退:{e}");
        // 单队列臂含 timeline 点 = 拒(回落必须重编译,非忽略 fence)。
        let segs_single_with_point = vec![AsyncLaneSubmitSegment {
            queue: AsyncLaneQueueKind::Graphics,
            pass_indices: vec![0],
            wait_value: Some(1),
            signal_value: None,
        }];
        let plan2 = AsyncLanesPlan {
            segments: &segs_single_with_point,
            dual_queue: false,
            timeline_span: 0,
            ..plan.clone()
        };
        let e2 = run_async_lanes(&plan2).expect_err("单臂含点应拒");
        assert!(e2.contains("重编译"), "错误应指明重编译义务:{e2}");
        // buffer 下标越界。
        let pass_oob = AsyncLanePassSpec {
            name: "p".into(),
            queue: AsyncLaneQueueKind::Graphics,
            dispatches: vec![AsyncLaneDispatchSpec {
                out_buf: 9,
                in_a: 0,
                in_b: 0,
                seed: 1,
                iters: 1,
            }],
        };
        let segs_plain = vec![AsyncLaneSubmitSegment {
            queue: AsyncLaneQueueKind::Graphics,
            pass_indices: vec![0],
            wait_value: None,
            signal_value: None,
        }];
        let plan3 = AsyncLanesPlan {
            passes: std::slice::from_ref(&pass_oob),
            segments: &segs_plain,
            dual_queue: false,
            timeline_span: 0,
            ..plan.clone()
        };
        let e3 = run_async_lanes(&plan3).expect_err("越界应拒");
        assert!(e3.contains("越界"), "错误应指明越界:{e3}");
    }
}
