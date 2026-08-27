// Assisted-by: Kimi-K3（G31+ 波 A Task A1）
// ── G31 外部图像 present 通路（vk.rs `include!` 共享模块作用域;vk_m50_rt_body.rs 同型先例）──
//
// 泛化 present 底座:`run_graphics_present`(自渲染三角形 demo,0-byte 不动)之外新增
// "外部图像 present" 会话——调用方逐帧传入**已渲染完成的图像**(本波 = host 像素缓冲
// RGBA8/BGRA8 + extent/format;device-image 零拷贝面需与渲染产物同 device,归后续波
// 如实登记),由本通路负责 swapchain acquire → staging buffer → `vkCmdCopyBufferToImage`
// → `PRESENT_SRC_KHR` → `vkQueuePresentKHR`。extent/format 协商(`pick_surface_format`
// / `choose_present_extent` 既有纯函数复用)、acquire/present `OUT_OF_DATE`/`SUBOPTIMAL`
// → 重建 swapchain(`swapchain_present_action` 三分类复用)均按 RXS-0221 纪律处理。
//
// 与 demo 的差异(不共改一行):
// - 无 render pass/graphics pipeline/shader/vertex buffer——纯 transfer 拷贝上屏;
// - 窗口默认**可见**(WS_POPUP + ShowWindow SW_SHOW;`visible=false` 退隐藏供 headless
//   自检),逐帧 `pump_messages` 保持消息队列排空;
// - staging = host-visible+coherent buffer,逐帧 map→copy→unmap(全同步口径:
//   submit 后 `vkQueueWaitIdle`,与 render_exec `execute_with_frame_update` 每帧 fence
//   全同步同律;present 开销由调用方独立墙钟计量,禁混入渲染帧率口径)。
//
// fail-closed:无 loader/无 present-capable queue/format 非 8-bit RGBA|BGRA/像素长度
// 不符 → 确定性 `Err`(非 panic);`RURIX_VK_VALIDATION=1` 开 debug messenger,ERROR 级
// 校验消息翻 `Err`。非 Windows → `create` 确定性 `Err`(android present = G-MB1-7 尾门)。

#[cfg(windows)]
const G31_SW_SHOW: i32 = 5;

// ── G31+ 波 A Task A3 窗口事件面（游戏循环最小面：输入→相机 / WM_SIZE resize
// → swapchain 重建 + 渲染 extent 联动 / 最小化跳过 / ESC·关闭干净退出）──
/// `WS_OVERLAPPEDWINDOW`（可缩放带标题栏;A1 = WS_POPUP 固定尺寸,A3 游戏循环
/// 面升级为用户可拖拽缩放/可最小化的真窗口;客户区尺寸经 `AdjustWindowRect`
/// 折算,创建期客户区 == 请求 w×h 不变式保持）。
#[cfg(windows)]
const G31_WS_OVERLAPPEDWINDOW: u32 = 0x00CF_0000;
#[cfg(windows)]
const G31_WM_SIZE: u32 = 0x0005;
#[cfg(windows)]
const G31_WM_CLOSE: u32 = 0x0010;
#[cfg(windows)]
const G31_WM_DESTROY: u32 = 0x0002;
#[cfg(windows)]
const G31_WM_KEYDOWN: u32 = 0x0100;
#[cfg(windows)]
const G31_WM_KEYUP: u32 = 0x0101;
#[cfg(windows)]
const G31_WM_MOUSEMOVE: u32 = 0x0200;
#[cfg(windows)]
const G31_VK_ESCAPE: usize = 0x1B;
/// win32 `SIZE_RESTORED`/`SIZE_MINIMIZED`(C4 窗口风暴臂程序化 WM_SIZE 注入)。
#[cfg(windows)]
const G31_SIZE_RESTORED: usize = 0;
#[cfg(windows)]
const G31_SIZE_MINIMIZED: usize = 1;

#[cfg(windows)]
#[link(name = "user32")]
unsafe extern "system" {
    /// user32 `ShowWindow`(win32 模块未声明,本通路自补;同 #[link(user32)] 纪律)。
    fn ShowWindow(hwnd: win32::Hwnd, cmd_show: i32) -> i32;
    /// user32 `AdjustWindowRect`(按样式把客户区请求折算为窗口外框)。
    fn AdjustWindowRect(rect: *mut G31Rect, style: u32, has_menu: i32) -> i32;
    /// user32 `SendMessageW`(C4 窗口风暴臂:同步直投本进程 wnd_proc,
    /// 与 OS 最小化/恢复消息面逐字同通路,免桌面焦点/SetWindowPos 依赖)。
    fn SendMessageW(hwnd: win32::Hwnd, msg: u32, w: win32::Wparam, l: win32::Lparam) -> win32::Lresult;
    /// user32 `SetWindowPos`(C4 窗口风暴臂:真改 win32 窗口客户区——
    /// `caps.current_extent` 随之变化,extent 协商面 = 用户拖拽同通路)。
    fn SetWindowPos(
        hwnd: win32::Hwnd,
        insert_after: win32::Hwnd,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        flags: u32,
    ) -> i32;
}

/// `SetWindowPos` 标志（C4 窗口风暴臂:仅尺寸生效;NOMOVE 0x0002 |
/// NOZORDER 0x0004 | NOACTIVATE 0x0010——不改位置/层级/不激活,**不取**
/// NOSIZE 0x0001,否则 cx/cy 被忽略）。
#[cfg(windows)]
const G31_SWP_FLAGS: u32 = 0x0002 | 0x0004 | 0x0010;

// ── G31+ 波 C Task C4 故障注入面（env 门控默认关,零行为变更）──
/// `RURIX_G31_FAULT_DEVICE_LOST=<point>@<index>`(point ∈ acquire|submit|present,
/// index = 0-based present 调用序 = `frames_presented` 快照):命中时把该点 Vulkan
/// 调用的**返回值**覆写为 `VK_ERROR_DEVICE_LOST`(真实调用已先行完成,swapchain/
/// GPU 态不受污染;仅 device-lost 处置面 = poisoned 锁存 + 确定性 Err 被演习)。
/// 未设置/形态非法 → `None` 原样直通(默认关,逐字节零行为变更)。
#[cfg(windows)]
fn g31_fault_device_lost(point: &'static str, present_index: u64) -> Option<VkResult> {
    static SPEC: std::sync::OnceLock<Option<(u8, u64)>> = std::sync::OnceLock::new();
    let spec = SPEC.get_or_init(|| {
        let raw = std::env::var("RURIX_G31_FAULT_DEVICE_LOST").ok()?;
        let (p, n) = raw.split_once('@')?;
        let code = match p {
            "acquire" => 0u8,
            "submit" => 1,
            "present" => 2,
            _ => return None,
        };
        Some((code, n.parse::<u64>().ok()?))
    });
    let want = match point {
        "acquire" => 0u8,
        "submit" => 1,
        _ => 2,
    };
    match spec {
        Some((c, idx)) if *c == want && *idx == present_index => Some(VK_ERROR_DEVICE_LOST),
        _ => None,
    }
}

/// win32 `RECT`（仅 `AdjustWindowRect` 消费）。
#[cfg(windows)]
#[repr(C)]
struct G31Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

/// G31 窗口输入静态态（wnd_proc 写 / `poll_input` 读;全程原子,单窗口槽——
/// hwnd 匹配才记录,其余窗口消息直通 `DefWindowProcW`;键位面 = 4×u64 位集
/// 覆盖 VK 0..=255,mouse 累计量/mouse 末位/close/resize(W<<32|H)/minimized）。
/// 静态可变驻留的理由:wnd_proc 是 win32 回调签名（无 user_data 通道经
/// cb_wnd_extra=0 的既有窗口类面）,输入态只能走进程静态;安全性由全原子
/// 操作承载（同一 UI 线程泵消息,无数据竞争）。
#[cfg(windows)]
struct G31InputStatics {
    hwnd: std::sync::atomic::AtomicUsize,
    keys: [std::sync::atomic::AtomicU64; 4],
    mouse_dx: std::sync::atomic::AtomicI32,
    mouse_dy: std::sync::atomic::AtomicI32,
    mouse_last_x: std::sync::atomic::AtomicI32,
    mouse_last_y: std::sync::atomic::AtomicI32,
    mouse_last_valid: std::sync::atomic::AtomicBool,
    close: std::sync::atomic::AtomicBool,
    resize_wh: std::sync::atomic::AtomicU64,
    minimized: std::sync::atomic::AtomicBool,
}

#[cfg(windows)]
static G31_INPUT: G31InputStatics = G31InputStatics {
    hwnd: std::sync::atomic::AtomicUsize::new(0),
    keys: [
        std::sync::atomic::AtomicU64::new(0),
        std::sync::atomic::AtomicU64::new(0),
        std::sync::atomic::AtomicU64::new(0),
        std::sync::atomic::AtomicU64::new(0),
    ],
    mouse_dx: std::sync::atomic::AtomicI32::new(0),
    mouse_dy: std::sync::atomic::AtomicI32::new(0),
    mouse_last_x: std::sync::atomic::AtomicI32::new(0),
    mouse_last_y: std::sync::atomic::AtomicI32::new(0),
    mouse_last_valid: std::sync::atomic::AtomicBool::new(false),
    close: std::sync::atomic::AtomicBool::new(false),
    resize_wh: std::sync::atomic::AtomicU64::new(0),
    minimized: std::sync::atomic::AtomicBool::new(false),
};

/// G31 窗口过程：输入记录（键位/mouse 增量）+ 事件标记（resize/minimize/
/// close）后一律委派 `DefWindowProcW`（WM_CLOSE 默认路 = DestroyWindow,
/// close 标记先行,调用方帧边界干净退出;hwnd 不匹配的消息不记录）。
/// # Safety
/// 由 win32 消息泵按 WNDPROC 契约调用;本函数只写进程静态原子与委派默认
/// 处理,不触碰窗口/设备句柄生命周期。
#[cfg(windows)]
unsafe extern "system" fn g31_wnd_proc(
    hwnd: win32::Hwnd,
    msg: u32,
    w: win32::Wparam,
    l: win32::Lparam,
) -> win32::Lresult {
    use std::sync::atomic::Ordering::Relaxed;
    if G31_INPUT.hwnd.load(Relaxed) == hwnd as usize {
        match msg {
            G31_WM_KEYDOWN => {
                let code = w & 0xFF;
                G31_INPUT.keys[code / 64].fetch_or(1u64 << (code % 64), Relaxed);
                if w == G31_VK_ESCAPE {
                    G31_INPUT.close.store(true, Relaxed);
                }
            }
            G31_WM_KEYUP => {
                let code = w & 0xFF;
                G31_INPUT.keys[code / 64].fetch_and(!(1u64 << (code % 64)), Relaxed);
            }
            G31_WM_MOUSEMOVE => {
                let x = (l & 0xFFFF) as i16 as i32;
                let y = ((l >> 16) & 0xFFFF) as i16 as i32;
                if G31_INPUT.mouse_last_valid.swap(true, Relaxed) {
                    let lx = G31_INPUT.mouse_last_x.swap(x, Relaxed);
                    let ly = G31_INPUT.mouse_last_y.swap(y, Relaxed);
                    G31_INPUT.mouse_dx.fetch_add(x - lx, Relaxed);
                    G31_INPUT.mouse_dy.fetch_add(y - ly, Relaxed);
                } else {
                    G31_INPUT.mouse_last_x.store(x, Relaxed);
                    G31_INPUT.mouse_last_y.store(y, Relaxed);
                }
            }
            G31_WM_SIZE => {
                let nw = (l & 0xFFFF) as u32;
                let nh = ((l >> 16) & 0xFFFF) as u32;
                if nw == 0 || nh == 0 {
                    G31_INPUT.minimized.store(true, Relaxed);
                } else {
                    G31_INPUT.minimized.store(false, Relaxed);
                    G31_INPUT
                        .resize_wh
                        .store((u64::from(nw) << 32) | u64::from(nh), Relaxed);
                }
            }
            G31_WM_CLOSE | G31_WM_DESTROY => {
                G31_INPUT.close.store(true, Relaxed);
            }
            _ => {}
        }
    }
    win32::DefWindowProcW(hwnd, msg, w, l)
}

/// `poll_input` 的一帧输入快照（键位 4×u64 位集 + mouse 增量（消费清零）+
/// close/resize/minimized 标记;纯数据,调用方（g31 bin）据此刻相机/曝光）。
#[derive(Debug, Clone, Copy, Default)]
pub struct ExternalInputFrame {
    /// VK 0..=255 键位位集（`key(vk)` 查询）。
    pub keys: [u64; 4],
    /// 自上次 poll 的 mouse 累计位移（像素;窗口内移动即累计,flycam 最小面）。
    pub mouse_dx: i32,
    /// 见 mouse_dx。
    pub mouse_dy: i32,
    /// ESC 按下 / WM_CLOSE / WM_DESTROY 之一触发（锁存,干净退出信号）。
    pub close_requested: bool,
    /// 最近一次 WM_SIZE 的非零客户区（消费清零;与当前 extent 不同才须 resize）。
    pub resize_pending: Option<(u32, u32)>,
    /// 最小化中（WM_SIZE 0×0;此间跳过渲染/present,消息泵保持）。
    pub minimized: bool,
}

impl ExternalInputFrame {
    /// VK 键当前按下态（vk ∈ 0..=255;越界恒 false）。
    pub fn key(&self, vk: usize) -> bool {
        vk < 256 && (self.keys[vk / 64] >> (vk % 64)) & 1 != 0
    }
}

/// G31 外部图像 present 会话(win32 窗口 + VkSurfaceKHR + VkSwapchainKHR 纯 transfer 上屏)。
///
/// # SAFETY(U27 扩注镜像,present FFI 边界)
/// 公共面对上全 safe,内部手写 Vulkan + win32 FFI:win32 窗口(RegisterClassW +
/// CreateWindowExW + ShowWindow 可选 + DestroyWindow/UnregisterClassW 逆序拆除)+
/// VkSurfaceKHR/VkSwapchainKHR/VkSemaphore×2/VkCommandPool/staging VkBuffer+VkDeviceMemory
/// 句柄线性配对 create/destroy(Drop 逆序;swapchain image 归 swapchain 所有,**不单独
/// destroy**);validation messenger user_data 指向 `Box<AtomicBool>`(堆地址稳定,会话
/// 移动不变;messenger 先销毁于 device/instance);单 graphics queue 全同步
/// (`vkQueueWaitIdle`)令两 binary semaphore 逐帧复用安全、staging 无数据竞争。
#[cfg(windows)]
pub struct ExternalImagePresent {
    // win32(Drop 末段拆;hwnd 非空即建过)。
    hwnd: win32::Hwnd,
    hinstance: win32::Hinstance,
    class_name: Vec<u16>,
    // vk 句柄(Drop 逆序销毁非 null 者;gdpa 仅 create 期用,不留存)。
    instance: VkInstance,
    surface: VkSurfaceKHR,
    device: VkDevice,
    queue: VkQueue,
    swapchain: VkSwapchainKHR,
    images: Vec<VkImage>,
    staging: VkBuffer,
    staging_mem: VkDeviceMemory,
    cmdpool: VkCommandPool,
    cmd: VkCommandBuffer,
    sem_acquire: VkSemaphore,
    sem_done: VkSemaphore,
    messenger: VkDebugUtilsMessengerEXT,
    // device 级符号(present/重建/销毁复用)。
    destroy_swapchain: FnDestroySwapchainKHR,
    get_swapchain_images: FnGetSwapchainImagesKHR,
    acquire_next: FnAcquireNextImageKHR,
    queue_present: FnQueuePresentKHR,
    queue_submit: FnQueueSubmit,
    queue_wait: FnQueueWaitIdle,
    create_swapchain: FnCreateSwapchainKHR,
    begin_cmd: FnBeginCommandBuffer,
    end_cmd: FnEndCommandBuffer,
    cmd_barrier: FnCmdPipelineBarrier,
    cmd_copy_buf_img: FnCmdCopyBufferToImage,
    map_mem: FnMapMemory,
    unmap_mem: FnUnmapMemory,
    destroy_buffer: FnDestroyBuffer,
    free_mem: FnFreeMemory,
    // A3 resize:staging 容量随 extent 重建的创建侧符号 + host 内存型（create 期
    // 一次协商留存;rebuild 采用同一内存型——物理设备/堆不变式由会话生命周期承载）。
    create_buffer: FnCreateBuffer,
    buf_mem_req: FnGetBufferMemoryRequirements,
    alloc_mem: FnAllocateMemory,
    bind_buf: FnBindBufferMemory,
    staging_mem_type: u32,
    destroy_sem: FnDestroySemaphore,
    destroy_cmdpool: FnDestroyCommandPool,
    destroy_device: Option<FnDestroyDevice>,
    destroy_surface: FnDestroySurfaceKHR,
    destroy_messenger: Option<FnDestroyDebugUtilsMessengerEXT>,
    destroy_instance: FnDestroyInstance,
    // 重建协商面(物理设备 + caps 查询 + 协商参数)。
    pd: VkPhysicalDevice,
    get_surf_caps: FnGetPhysicalDeviceSurfaceCapabilitiesKHR,
    min_image_count: u32,
    req_w: u32,
    req_h: u32,
    ext_w: u32,
    ext_h: u32,
    format: u32,
    color_space: u32,
    rebuild_pending: bool,
    // C4 device-lost 处置面:任一 acquire/submit/present 真返或注入
    // VK_ERROR_DEVICE_LOST → 锁存确定性错误串(RXS-0077 poisoned 同律;
    // 后续 present/resize 一律确定性 `Err`,禁 UB 级联;干净退出由调用方承载)。
    poisoned: Option<String>,
    validation_error: Box<std::sync::atomic::AtomicBool>,
    frames_presented: u64,
    swapchain_rebuilds: u64,
}

/// 外部图像 present 会话的逐帧/累计计数面(evidence 登记用)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExternalPresentCounts {
    pub frames_presented: u64,
    pub swapchain_rebuilds: u64,
}

#[cfg(windows)]
impl ExternalImagePresent {
    /// 建会话:win32 窗口(`visible` 控 SW_SHOW;客户区 == w×h)+ instance/surface/device +
    /// swapchain + staging + 同步件。任一失败 = 确定性 `Err`,已建句柄逆序拆除不泄漏。
    pub fn create(width: u32, height: u32, title: &str, visible: bool) -> Result<Self, String> {
        if width == 0 || height == 0 {
            return Err("外部图像 present:extent 塌零".into());
        }
        let gipa = load_vulkan_loader().ok_or("vulkan loader (vulkan-1.dll) 不可用")?;
        // SAFETY: 见结构体 U27 扩注镜像;窗口/句柄生命周期由本会话线性管理,Drop 逆序拆除。
        unsafe { Self::create_inner(gipa, width, height, title, visible) }
    }

    /// SAFETY: 同 `create` 扩注;gipa 来自 `load_vulkan_loader`(loader 生命周期覆盖会话)。
    unsafe fn create_inner(
        gipa: FnGetInstanceProcAddr,
        width: u32,
        height: u32,
        title: &str,
        visible: bool,
    ) -> Result<Self, String> {
        // ── win32 窗口(class 名唯一化,镜像 run_graphics_present_inner)──
        let hinstance = win32::GetModuleHandleW(std::ptr::null());
        static G31_WND_SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let seq = G31_WND_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let class_name =
            win32::to_wide(&format!("RurixG31Present_{}_{}", std::process::id(), seq));
        let window_name = win32::to_wide(title);
        let wc = win32::WndClassW {
            style: 0,
            lpfn_wnd_proc: Some(g31_wnd_proc),
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
        // A3:WS_OVERLAPPEDWINDOW（用户可缩放/最小化;客户区 == 请求 w×h 由
        // AdjustWindowRect 折算外框保证,创建期不变式与 A1 同）。
        let mut rect = G31Rect {
            left: 0,
            top: 0,
            right: width as i32,
            bottom: height as i32,
        };
        // SAFETY: rect 为栈上合法 RECT;AdjustWindowRect 纯计算不写他处。
        AdjustWindowRect(&mut rect, G31_WS_OVERLAPPEDWINDOW, 0);
        let win_w = rect.right - rect.left;
        let win_h = rect.bottom - rect.top;
        let hwnd = win32::CreateWindowExW(
            0,
            class_name.as_ptr(),
            window_name.as_ptr(),
            G31_WS_OVERLAPPEDWINDOW,
            64,
            64,
            win_w,
            win_h,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null_mut(),
        );
        if hwnd.is_null() {
            win32::UnregisterClassW(class_name.as_ptr(), hinstance);
            return Err("win32 CreateWindowExW 失败".into());
        }
        // 输入静态态复位 + 注册本窗口（单窗口槽;此前会话残留清零,键位/
        // mouse/close/resize/minimized 全归零——同进程多会话形态自检面）。
        {
            use std::sync::atomic::Ordering::Relaxed;
            for k in &G31_INPUT.keys {
                k.store(0, Relaxed);
            }
            G31_INPUT.mouse_dx.store(0, Relaxed);
            G31_INPUT.mouse_dy.store(0, Relaxed);
            G31_INPUT.mouse_last_valid.store(false, Relaxed);
            G31_INPUT.close.store(false, Relaxed);
            G31_INPUT.resize_wh.store(0, Relaxed);
            G31_INPUT.minimized.store(false, Relaxed);
            G31_INPUT.hwnd.store(hwnd as usize, Relaxed);
        }
        if visible {
            // SAFETY: hwnd 为本会话刚建的合法窗口句柄。
            ShowWindow(hwnd, G31_SW_SHOW);
        }
        pump_messages(hwnd);

        // ── vk 会话(失败 → 拆窗口后返回 Err)──
        match Self::create_vk(gipa, hinstance, hwnd, width, height) {
            Ok(parts) => Ok(Self {
                hwnd,
                hinstance,
                class_name,
                instance: parts.instance,
                surface: parts.surface,
                device: parts.device,
                queue: parts.queue,
                swapchain: parts.swapchain,
                images: parts.images,
                staging: parts.staging,
                staging_mem: parts.staging_mem,
                cmdpool: parts.cmdpool,
                cmd: parts.cmd,
                sem_acquire: parts.sem_acquire,
                sem_done: parts.sem_done,
                messenger: parts.messenger,
                destroy_swapchain: parts.destroy_swapchain,
                get_swapchain_images: parts.get_swapchain_images,
                acquire_next: parts.acquire_next,
                queue_present: parts.queue_present,
                queue_submit: parts.queue_submit,
                queue_wait: parts.queue_wait,
                create_swapchain: parts.create_swapchain,
                begin_cmd: parts.begin_cmd,
                end_cmd: parts.end_cmd,
                cmd_barrier: parts.cmd_barrier,
                cmd_copy_buf_img: parts.cmd_copy_buf_img,
                map_mem: parts.map_mem,
                unmap_mem: parts.unmap_mem,
                destroy_buffer: parts.destroy_buffer,
                free_mem: parts.free_mem,
                create_buffer: parts.create_buffer,
                buf_mem_req: parts.buf_mem_req,
                alloc_mem: parts.alloc_mem,
                bind_buf: parts.bind_buf,
                staging_mem_type: parts.staging_mem_type,
                destroy_sem: parts.destroy_sem,
                destroy_cmdpool: parts.destroy_cmdpool,
                destroy_device: parts.destroy_device,
                destroy_surface: parts.destroy_surface,
                destroy_messenger: parts.destroy_messenger,
                destroy_instance: parts.destroy_instance,
                pd: parts.pd,
                get_surf_caps: parts.get_surf_caps,
                min_image_count: parts.min_image_count,
                req_w: width,
                req_h: height,
                ext_w: parts.ext_w,
                ext_h: parts.ext_h,
                format: parts.format,
                color_space: parts.color_space,
                rebuild_pending: false,
                poisoned: None,
                validation_error: parts.validation_error,
                frames_presented: 0,
                swapchain_rebuilds: 0,
            }),
            Err(e) => {
                win32::DestroyWindow(hwnd);
                win32::UnregisterClassW(class_name.as_ptr(), hinstance);
                Err(e)
            }
        }
    }

    /// 协商后的 swapchain extent(== 请求 extent,否则 create 已 fail-closed)。
    pub fn extent(&self) -> (u32, u32) {
        (self.ext_w, self.ext_h)
    }

    /// 像素通道序(`present_rgba8` 入参布局):所选 swapchain format 派生。
    pub fn channel_order(&self) -> &'static str {
        if self.format == FORMAT_B8G8R8A8_UNORM {
            "bgra8_unorm"
        } else if self.format == FORMAT_R8G8B8A8_UNORM {
            "rgba8_unorm"
        } else {
            "unreachable(create 已 fail-closed 非 8-bit RGBA/BGRA)"
        }
    }

    /// 累计计数面(frames_presented / swapchain_rebuilds;evidence 登记)。
    pub fn counts(&self) -> ExternalPresentCounts {
        ExternalPresentCounts {
            frames_presented: self.frames_presented,
            swapchain_rebuilds: self.swapchain_rebuilds,
        }
    }

    /// 一帧:host 像素(`channel_order()` 布局,w×h×4 字节)→ staging → acquire →
    /// copy(buffer→image)→ PRESENT_SRC → present → queue idle(全同步)+ 消息泵。
    /// SAFETY: 见结构体扩注;调用方像素缓冲在本调用内只读,无跨帧别名。
    pub fn present_rgba8(&mut self, pixels: &[u8]) -> Result<(), String> {
        let need = (self.ext_w as usize) * (self.ext_h as usize) * 4;
        if pixels.len() != need {
            return Err(format!(
                "外部图像 present:像素长度 {} != {}({}x{}x4)",
                pixels.len(),
                need,
                self.ext_w,
                self.ext_h
            ));
        }
        // SAFETY: 见结构体 U27 扩注镜像(句柄配对/单 queue 全同步/信号量逐帧复用)。
        unsafe { self.present_inner(pixels) }
    }

    /// C4 device-lost 处置面:锁存 poisoned 确定性错误串并返回(RXS-0077 同律——
    /// device lost 后本会话一切后续操作确定性失败,禁 UB 级联;消息含注入臂
    /// 标记由 `g31_fault_device_lost` 覆写路径与真返路径共用,字面不区分——
    /// 处置确定性不区分来源)。
    fn poison(&mut self, op: &str, code: VkResult) -> String {
        let msg = format!(
            "{op} 返回 VK_ERROR_DEVICE_LOST({code}):device lost——present 会话 poisoned\
             (RXS-0077 同律;后续 acquire/submit/present/resize 确定性失败,禁 UB 级联;\
             干净退出/会话重建由调用方承载)"
        );
        self.poisoned = Some(msg.clone());
        msg
    }

    /// SAFETY: 同 `present_rgba8`;仅由其调用。
    #[allow(clippy::too_many_lines)]
    unsafe fn present_inner(&mut self, pixels: &[u8]) -> Result<(), String> {
        // C4 poisoned 锁存:device-lost 后一切后续 present 确定性失败(禁 UB 级联)。
        if let Some(p) = self.poisoned.as_ref() {
            return Err(p.clone());
        }
        pump_messages(self.hwnd);
        // ── 重建(前一帧 present/acquire 报失效;OUT_OF_DATE 重试本帧不推进;
        //    extent 塌零 = 最小化——本帧跳过 present 不报错,恢复后自然重建)──
        let mut image_index = 0u32;
        loop {
            if self.rebuild_pending {
                (self.queue_wait)(self.queue);
                if !self.rebuild_swapchain()? {
                    return Ok(());
                }
                self.rebuild_pending = false;
            }
            let mut acq = (self.acquire_next)(
                self.device,
                self.swapchain,
                u64::MAX,
                self.sem_acquire,
                VK_NULL_HANDLE,
                &mut image_index,
            );
            // C4 注入臂(默认关):覆写 acquire 返回值为 DEVICE_LOST 演习处置面。
            if let Some(f) = g31_fault_device_lost("acquire", self.frames_presented) {
                acq = f;
            }
            if acq == VK_ERROR_DEVICE_LOST {
                return Err(self.poison("vkAcquireNextImageKHR", acq));
            }
            match swapchain_present_action(acq) {
                SwapchainAction::Fatal => {
                    return Err(format!("vkAcquireNextImageKHR(外部图像)失败: {acq}"));
                }
                SwapchainAction::Rebuild if acq == ERROR_OUT_OF_DATE_KHR => {
                    self.rebuild_pending = true;
                    continue;
                }
                SwapchainAction::Rebuild => self.rebuild_pending = true,
                SwapchainAction::Present => {}
            }
            break;
        }

        // ── staging 上传(host coherent;逐帧 map→copy→unmap)──
        {
            let mut ptr: *mut c_void = std::ptr::null_mut();
            if (self.map_mem)(self.device, self.staging_mem, 0, WHOLE_SIZE, 0, &mut ptr)
                != VK_SUCCESS
            {
                return Err("staging vkMapMemory 失败".into());
            }
            std::ptr::copy_nonoverlapping(pixels.as_ptr(), ptr.cast::<u8>(), pixels.len());
            (self.unmap_mem)(self.device, self.staging_mem);
        }

        // ── 录制:UNDEFINED→TRANSFER_DST → copy → PRESENT_SRC ──
        let cbbi = CommandBufferBeginInfo {
            s_type: ST_COMMAND_BUFFER_BEGIN_INFO,
            p_next: std::ptr::null(),
            flags: CMD_BUFFER_USAGE_ONE_TIME_SUBMIT,
            p_inheritance_info: std::ptr::null(),
        };
        (self.begin_cmd)(self.cmd, &cbbi);
        let img = self.images[image_index as usize];
        let to_transfer = ImageMemoryBarrier {
            s_type: ST_IMAGE_MEMORY_BARRIER,
            p_next: std::ptr::null(),
            src_access_mask: 0,
            dst_access_mask: ACCESS_TRANSFER_WRITE,
            old_layout: IMAGE_LAYOUT_UNDEFINED,
            new_layout: IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
            src_queue_family_index: QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: QUEUE_FAMILY_IGNORED,
            image: img,
            subresource_range: VkImageSubresourceRange {
                aspect_mask: IMAGE_ASPECT_COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
        };
        (self.cmd_barrier)(
            self.cmd,
            PIPELINE_STAGE_TOP_OF_PIPE,
            PIPELINE_STAGE_TRANSFER,
            0,
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
            1,
            &to_transfer,
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
                width: self.ext_w,
                height: self.ext_h,
                depth: 1,
            },
        };
        (self.cmd_copy_buf_img)(
            self.cmd,
            self.staging,
            img,
            IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
            1,
            &region,
        );
        let to_present = ImageMemoryBarrier {
            s_type: ST_IMAGE_MEMORY_BARRIER,
            p_next: std::ptr::null(),
            src_access_mask: ACCESS_TRANSFER_WRITE,
            dst_access_mask: 0,
            old_layout: IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
            new_layout: IMAGE_LAYOUT_PRESENT_SRC_KHR,
            src_queue_family_index: QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: QUEUE_FAMILY_IGNORED,
            image: img,
            subresource_range: VkImageSubresourceRange {
                aspect_mask: IMAGE_ASPECT_COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
        };
        (self.cmd_barrier)(
            self.cmd,
            PIPELINE_STAGE_TRANSFER,
            PIPELINE_STAGE_BOTTOM_OF_PIPE,
            0,
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
            1,
            &to_present,
        );
        (self.end_cmd)(self.cmd);

        // ── submit(wait acquire @ TRANSFER,signal done)──
        let wait_stage: VkFlags = PIPELINE_STAGE_TRANSFER;
        let si = SubmitInfo {
            s_type: ST_SUBMIT_INFO,
            p_next: std::ptr::null(),
            wait_semaphore_count: 1,
            p_wait_semaphores: &self.sem_acquire,
            p_wait_dst_stage_mask: &wait_stage,
            command_buffer_count: 1,
            p_command_buffers: &self.cmd,
            signal_semaphore_count: 1,
            p_signal_semaphores: &self.sem_done,
        };
        let mut sr = (self.queue_submit)(self.queue, 1, &si, VK_NULL_HANDLE);
        // C4 注入臂(默认关):覆写 submit 返回值为 DEVICE_LOST 演习处置面。
        if let Some(f) = g31_fault_device_lost("submit", self.frames_presented) {
            sr = f;
        }
        if sr == VK_ERROR_DEVICE_LOST {
            return Err(self.poison("vkQueueSubmit", sr));
        }
        if sr != VK_SUCCESS {
            return Err(format!("vkQueueSubmit(外部图像 present)失败: {sr}"));
        }

        // ── present(wait done)──
        let mut per_result: VkResult = VK_SUCCESS;
        let pi = PresentInfoKHR {
            s_type: ST_PRESENT_INFO_KHR,
            p_next: std::ptr::null(),
            wait_semaphore_count: 1,
            p_wait_semaphores: &self.sem_done,
            swapchain_count: 1,
            p_swapchains: &self.swapchain,
            p_image_indices: &image_index,
            p_results: &mut per_result,
        };
        let mut pr = (self.queue_present)(self.queue, &pi);
        // C4 注入臂(默认关):覆写 present 返回值为 DEVICE_LOST 演习处置面。
        if let Some(f) = g31_fault_device_lost("present", self.frames_presented) {
            pr = f;
        }
        if pr == VK_ERROR_DEVICE_LOST {
            return Err(self.poison("vkQueuePresentKHR", pr));
        }
        match swapchain_present_action(pr) {
            SwapchainAction::Fatal => {
                return Err(format!("vkQueuePresentKHR(外部图像)失败: {pr}"));
            }
            SwapchainAction::Rebuild => self.rebuild_pending = true,
            SwapchainAction::Present => {}
        }
        if per_result == VK_ERROR_DEVICE_LOST {
            return Err(self.poison("vkQueuePresentKHR(per-swapchain)", per_result));
        }
        match swapchain_present_action(per_result) {
            SwapchainAction::Fatal => {
                return Err(format!("present per-swapchain 结果失败: {per_result}"));
            }
            SwapchainAction::Rebuild => self.rebuild_pending = true,
            SwapchainAction::Present => {}
        }
        (self.queue_wait)(self.queue); // 令 binary semaphore 逐帧复用安全(全同步口径)。
        self.frames_presented += 1;

        // fail-closed(L3):validation ERROR 级消息翻 Err(退出码判红)。
        if self.validation_error.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(
                "VK_LAYER_KHRONOS_validation 报 ERROR 级校验错误(见 stderr;fail-closed,L3)"
                    .into(),
            );
        }
        Ok(())
    }

    /// 重建 swapchain(等 GPU idle 由调用方先行;old_swapchain 复用后销毁)。
    /// 返回 `Ok(false)` = extent 塌零(最小化)——**不重建不报错**,旧链保留,
    /// 恢复非零 extent 后下一帧自然重建(最小化健壮性面:acquire/present 在
    /// 0×0 surface 上 OUT_OF_DATE 是正轨,WM_SIZE 塌零帧须跳过而非判红)。
    /// A3 语义升级:extent **采纳** caps 协商值(可缩放窗口 WM_SIZE 驱动漂移是
    /// 正轨——A1 固定 WS_POPUP 的漂移 fail-closed 面随窗口样式升级退场);
    /// extent 变化时 staging 容量同步重建(host 像素缓冲与 swapchain extent
    /// 逐位匹配不变式保持,缩放 blit 面仍归后续波)。
    /// SAFETY: 同结构体扩注;queue 已 idle,旧链 image/staging 无在途引用。
    unsafe fn rebuild_swapchain(&mut self) -> Result<bool, String> {
        let mut caps = std::mem::zeroed::<SurfaceCapabilitiesKHR>();
        if (self.get_surf_caps)(self.pd, self.surface, &mut caps) != VK_SUCCESS {
            return Err("vkGetPhysicalDeviceSurfaceCapabilitiesKHR(rebuild)失败".into());
        }
        let (ew, eh) = choose_present_extent(
            (caps.current_extent.width, caps.current_extent.height),
            self.req_w,
            self.req_h,
            (caps.min_image_extent.width, caps.min_image_extent.height),
            (caps.max_image_extent.width, caps.max_image_extent.height),
        );
        if ew == 0 || eh == 0 {
            return Ok(false);
        }
        let extent_changed = (ew, eh) != (self.ext_w, self.ext_h);
        let sci = SwapchainCreateInfoKHR {
            s_type: ST_SWAPCHAIN_CREATE_INFO_KHR,
            p_next: std::ptr::null(),
            flags: 0,
            surface: self.surface,
            min_image_count: self.min_image_count,
            image_format: self.format,
            image_color_space: self.color_space,
            image_extent: VkExtent2D {
                width: ew,
                height: eh,
            },
            image_array_layers: 1,
            image_usage: IMAGE_USAGE_TRANSFER_DST,
            image_sharing_mode: SHARING_MODE_EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
            pre_transform: caps.current_transform,
            composite_alpha: pick_composite_alpha(caps.supported_composite_alpha),
            present_mode: PRESENT_MODE_FIFO_KHR,
            clipped: 1,
            old_swapchain: self.swapchain,
        };
        let mut sc: VkSwapchainKHR = VK_NULL_HANDLE;
        let r = (self.create_swapchain)(self.device, &sci, std::ptr::null(), &mut sc);
        if r != VK_SUCCESS {
            return Err(format!("vkCreateSwapchainKHR(rebuild)失败: {r}"));
        }
        let mut img_count = 0u32;
        (self.get_swapchain_images)(self.device, sc, &mut img_count, std::ptr::null_mut());
        if img_count == 0 {
            (self.destroy_swapchain)(self.device, sc, std::ptr::null());
            return Err("swapchain(rebuild)无 image".into());
        }
        let mut imgs: Vec<VkImage> = vec![VK_NULL_HANDLE; img_count as usize];
        (self.get_swapchain_images)(self.device, sc, &mut img_count, imgs.as_mut_ptr());
        (self.destroy_swapchain)(self.device, self.swapchain, std::ptr::null());
        self.swapchain = sc;
        self.images = imgs;
        self.ext_w = ew;
        self.ext_h = eh;
        if extent_changed {
            // staging 容量随 extent 重建(queue 已 idle,无在途引用;先拆后建,
            // 建失败 = 确定性 Err 且 staging 句柄恒 null——后续 present 的像素
            // 长度门/map 失败路 fail-closed,不悬垂旧容量)。
            if self.staging != VK_NULL_HANDLE {
                (self.destroy_buffer)(self.device, self.staging, std::ptr::null());
                self.staging = VK_NULL_HANDLE;
            }
            if self.staging_mem != VK_NULL_HANDLE {
                (self.free_mem)(self.device, self.staging_mem, std::ptr::null());
                self.staging_mem = VK_NULL_HANDLE;
            }
            let bci = BufferCreateInfo {
                s_type: ST_BUFFER_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                size: (ew as u64) * (eh as u64) * 4,
                usage: BUFFER_USAGE_TRANSFER_SRC,
                sharing_mode: SHARING_MODE_EXCLUSIVE,
                queue_family_index_count: 0,
                p_queue_family_indices: std::ptr::null(),
            };
            if (self.create_buffer)(self.device, &bci, std::ptr::null(), &mut self.staging)
                != VK_SUCCESS
            {
                return Err("staging vkCreateBuffer(resize)失败".into());
            }
            let mut req = std::mem::zeroed::<MemoryRequirements>();
            (self.buf_mem_req)(self.device, self.staging, &mut req);
            if req.memory_type_bits & (1u32 << self.staging_mem_type) == 0 {
                return Err("staging(resize)内存型不兼容(fail-closed)".into());
            }
            let mai = MemoryAllocateInfo {
                s_type: ST_MEMORY_ALLOCATE_INFO,
                p_next: std::ptr::null(),
                allocation_size: req.size,
                memory_type_index: self.staging_mem_type,
            };
            if (self.alloc_mem)(self.device, &mai, std::ptr::null(), &mut self.staging_mem)
                != VK_SUCCESS
            {
                return Err("staging vkAllocateMemory(resize)失败".into());
            }
            (self.bind_buf)(self.device, self.staging, self.staging_mem, 0);
        }
        self.swapchain_rebuilds += 1;
        Ok(true)
    }

    /// A3 resize:显式请求 extent 变更(WM_SIZE 驱动;与当前相同 = 无操作)。
    /// 返回是否真实重建。queue idle → 链重建(extent 采纳 caps 协商值——极小
    /// 窗口被 caps 钳制时以协商值为准,调用方渲染 extent 联动取 `extent()`;
    /// 请求非零而协商塌零 = 最小化竞态,确定性 Err 由调用方按 minimized 面
    /// 先行跳过)。
    /// SAFETY 面:内部全同步(queue_wait),无在途帧;调用方随后须以新 extent
    /// 重建渲染侧资源(渲染 extent 联动由调用方承载)。
    pub fn resize(&mut self, width: u32, height: u32) -> Result<bool, String> {
        if width == 0 || height == 0 {
            return Err("外部图像 present resize:extent 塌零".into());
        }
        // C4 poisoned 锁存:device-lost 后 resize 同样确定性失败(禁 UB 级联)。
        if let Some(p) = self.poisoned.as_ref() {
            return Err(p.clone());
        }
        self.rebuild_pending = false;
        if (width, height) == (self.ext_w, self.ext_h) {
            return Ok(false);
        }
        self.req_w = width;
        self.req_h = height;
        // SAFETY: 同结构体 U27 扩注镜像;本调用内 staging/swapchain 无在途引用。
        let rebuilt = unsafe {
            (self.queue_wait)(self.queue);
            self.rebuild_swapchain()?
        };
        if !rebuilt {
            return Err("外部图像 present resize:协商 extent 塌零(最小化竞态)".into());
        }
        Ok(true)
    }

    /// C4 窗口风暴臂:程序化 WM_SIZE 注入(`minimized=true` ⇒ wParam
    /// SIZE_MINIMIZED + 0×0;否则 SIZE_RESTORED + w×h)——同步直投本进程
    /// wnd_proc,与 OS 最小化/恢复触发的消息面**逐字同通路**(G31_INPUT 原子
    /// 面 → `poll_input` minimized/resize_pending),但免桌面焦点/真实窗口
    /// 管理器依赖(非交互 runner 可重放)。仅 `--window-storm`/`--storm-soak`
    /// 臂消费,常态零调用。
    /// SAFETY: hwnd 为本会话存活窗口;SendMessageW 同步调用本进程 wnd_proc,
    /// 仅写 G31_INPUT 原子(下一帧 poll_input 消费),不触碰窗口/设备句柄生命周期。
    pub fn storm_wm_size(&self, width: u32, height: u32, minimized: bool) {
        let wp = if minimized {
            G31_SIZE_MINIMIZED
        } else {
            G31_SIZE_RESTORED
        };
        let lp = ((height as usize) << 16) | (width as usize & 0xFFFF);
        // SAFETY: 见方法级注;返回值(DefWindowProcW 结果)无语义消费面。
        unsafe {
            SendMessageW(self.hwnd, G31_WM_SIZE, wp, lp as win32::Lparam);
        }
    }

    /// C4 窗口风暴臂:程序化**真**窗口尺寸变更（SetWindowPos;客户区 == w×h
    /// 由 AdjustWindowRect 折算外框）——win32 窗口本体变化 ⇒ WM_SIZE 同步
    /// 入 G31_INPUT（与用户拖拽同通路）⇒ 下一帧 `poll_input` resize_pending
    /// ⇒ surface `caps.current_extent` 随之变化 ⇒ extent 协商/staging/era
    /// 重建全真面。仅 `--window-storm`/`--storm-soak` 臂消费,常态零调用。
    /// SAFETY: hwnd 为本会话存活窗口;SetWindowPos 同步派发 WM_SIZE 经本进程
    /// wnd_proc 仅写 G31_INPUT 原子,不触碰设备/交换链句柄生命周期。
    pub fn storm_set_window_size(&self, width: u32, height: u32) {
        let mut rect = G31Rect {
            left: 0,
            top: 0,
            right: width as i32,
            bottom: height as i32,
        };
        // SAFETY: rect 为栈上合法 RECT;AdjustWindowRect 纯计算不写他处。
        unsafe {
            AdjustWindowRect(&mut rect, G31_WS_OVERLAPPEDWINDOW, 0);
            SetWindowPos(
                self.hwnd,
                std::ptr::null_mut(),
                0,
                0,
                rect.right - rect.left,
                rect.bottom - rect.top,
                G31_SWP_FLAGS,
            );
        }
    }

    /// A3 游戏循环输入面:泵消息队列(非阻塞)+ 一帧输入快照(mouse 累计量/
    /// resize 槽消费清零;close 锁存读)。无窗口会话形态下 wnd_proc 不注册,
    /// 快照恒默认(全零/无事件)。
    pub fn poll_input(&mut self) -> ExternalInputFrame {
        // SAFETY: hwnd 为本会话存活窗口;非阻塞泵,无重入(render/present 段
        // 之外调用由调用方纪律承载)。
        unsafe {
            pump_messages(self.hwnd);
        }
        use std::sync::atomic::Ordering::Relaxed;
        let keys = [
            G31_INPUT.keys[0].load(Relaxed),
            G31_INPUT.keys[1].load(Relaxed),
            G31_INPUT.keys[2].load(Relaxed),
            G31_INPUT.keys[3].load(Relaxed),
        ];
        let mouse_dx = G31_INPUT.mouse_dx.swap(0, Relaxed);
        let mouse_dy = G31_INPUT.mouse_dy.swap(0, Relaxed);
        let close_requested = G31_INPUT.close.load(Relaxed);
        let packed = G31_INPUT.resize_wh.swap(0, Relaxed);
        let resize_pending = if packed == 0 {
            None
        } else {
            Some(((packed >> 32) as u32, packed as u32))
        };
        let minimized = G31_INPUT.minimized.load(Relaxed);
        ExternalInputFrame {
            keys,
            mouse_dx,
            mouse_dy,
            close_requested,
            resize_pending,
            minimized,
        }
    }

    /// vk 会话件(create_inner 中段产物;任一失败逆序拆除已建句柄)。
    /// SAFETY: 同 `create` 扩注。
    #[allow(clippy::too_many_lines)]
    unsafe fn create_vk(
        gipa: FnGetInstanceProcAddr,
        hinstance: win32::Hinstance,
        hwnd: win32::Hwnd,
        width: u32,
        height: u32,
    ) -> Result<G31VkParts, String> {
        let vk_create_instance: FnCreateInstance =
            cast_fn(gipa(std::ptr::null_mut(), c"vkCreateInstance".as_ptr()))
                .ok_or("缺 vkCreateInstance")?;

        let validation = std::env::var("RURIX_VK_VALIDATION").as_deref() == Ok("1");
        let layer_name = c"VK_LAYER_KHRONOS_validation";
        let layers: [*const c_char; 1] = [layer_name.as_ptr()];
        let mut exts: Vec<*const c_char> =
            vec![c"VK_KHR_surface".as_ptr(), c"VK_KHR_win32_surface".as_ptr()];
        if validation {
            exts.push(c"VK_EXT_debug_utils".as_ptr());
        }
        let app = ApplicationInfo {
            s_type: ST_APPLICATION_INFO,
            p_next: std::ptr::null(),
            p_application_name: c"rurix-g31".as_ptr(),
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
        if vk_create_instance(&ici, std::ptr::null(), &mut instance) != VK_SUCCESS {
            return Err("vkCreateInstance(g31 present) 失败".into());
        }

        let destroy_instance: FnDestroyInstance =
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
        let gdpa: FnGetDeviceProcAddr = cast_fn(gipa(instance, c"vkGetDeviceProcAddr".as_ptr()))
            .ok_or("缺 vkGetDeviceProcAddr")?;
        let create_win32_surface: FnCreateWin32SurfaceKHR =
            cast_fn(gipa(instance, c"vkCreateWin32SurfaceKHR".as_ptr()))
                .ok_or("缺 vkCreateWin32SurfaceKHR(未启用 VK_KHR_win32_surface?)")?;
        let destroy_surface: FnDestroySurfaceKHR =
            cast_fn(gipa(instance, c"vkDestroySurfaceKHR".as_ptr()))
                .ok_or("缺 vkDestroySurfaceKHR")?;
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

        // fail-closed messenger(镜像 present_vk;user_data = Box<AtomicBool> 堆地址稳定)。
        let validation_error = Box::new(std::sync::atomic::AtomicBool::new(false));
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
                p_user_data: &*validation_error as *const std::sync::atomic::AtomicBool
                    as *mut c_void,
            };
            let _ = create_messenger(instance, &dumci, std::ptr::null(), &mut messenger);
        }
        macro_rules! teardown_msgr_instance {
            () => {{
                if let Some(dm) = destroy_messenger {
                    if messenger != VK_NULL_HANDLE {
                        dm(instance, messenger, std::ptr::null());
                    }
                }
                destroy_instance(instance, std::ptr::null());
            }};
        }

        // ── surface ──
        let w32ci = Win32SurfaceCreateInfoKHR {
            s_type: ST_WIN32_SURFACE_CREATE_INFO_KHR,
            p_next: std::ptr::null(),
            flags: 0,
            hinstance,
            hwnd,
        };
        let mut surface: VkSurfaceKHR = VK_NULL_HANDLE;
        if create_win32_surface(instance, &w32ci, std::ptr::null(), &mut surface) != VK_SUCCESS {
            teardown_msgr_instance!();
            return Err("vkCreateWin32SurfaceKHR(g31) 失败".into());
        }
        macro_rules! teardown_surface {
            () => {{
                destroy_surface(instance, surface, std::ptr::null());
                teardown_msgr_instance!();
            }};
        }

        // 物理设备 + present-capable graphics queue family。
        let mut count = 0u32;
        vk_enum_pd(instance, &mut count, std::ptr::null_mut());
        if count == 0 {
            teardown_surface!();
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
        let Some(qfi) = qfi_opt else {
            teardown_surface!();
            return Err("无 present-capable graphics queue family".into());
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
        if vk_create_device(pd, &dci, std::ptr::null(), &mut device) != VK_SUCCESS {
            teardown_surface!();
            return Err("vkCreateDevice(g31 present) 失败".into());
        }

        macro_rules! dp {
            ($name:literal, $ty:ty) => {
                cast_fn::<$ty>(gdpa(device, $name.as_ptr())).ok_or("缺 device 符号")?
            };
        }
        let result: Result<G31VkParts, String> = 'run: {
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
            let create_cmdpool: FnCreateCommandPool =
                dp!(c"vkCreateCommandPool", FnCreateCommandPool);
            let destroy_cmdpool: FnDestroyCommandPool =
                dp!(c"vkDestroyCommandPool", FnDestroyCommandPool);
            let alloc_cmd: FnAllocateCommandBuffers =
                dp!(c"vkAllocateCommandBuffers", FnAllocateCommandBuffers);
            let begin_cmd: FnBeginCommandBuffer =
                dp!(c"vkBeginCommandBuffer", FnBeginCommandBuffer);
            let end_cmd: FnEndCommandBuffer = dp!(c"vkEndCommandBuffer", FnEndCommandBuffer);
            let cmd_barrier: FnCmdPipelineBarrier =
                dp!(c"vkCmdPipelineBarrier", FnCmdPipelineBarrier);
            let cmd_copy_buf_img: FnCmdCopyBufferToImage =
                dp!(c"vkCmdCopyBufferToImage", FnCmdCopyBufferToImage);
            let queue_submit: FnQueueSubmit = dp!(c"vkQueueSubmit", FnQueueSubmit);
            let queue_wait: FnQueueWaitIdle = dp!(c"vkQueueWaitIdle", FnQueueWaitIdle);
            let create_swapchain: FnCreateSwapchainKHR =
                dp!(c"vkCreateSwapchainKHR", FnCreateSwapchainKHR);
            let destroy_swapchain: FnDestroySwapchainKHR =
                dp!(c"vkDestroySwapchainKHR", FnDestroySwapchainKHR);
            let get_swapchain_images: FnGetSwapchainImagesKHR =
                dp!(c"vkGetSwapchainImagesKHR", FnGetSwapchainImagesKHR);
            let acquire_next: FnAcquireNextImageKHR =
                dp!(c"vkAcquireNextImageKHR", FnAcquireNextImageKHR);
            let queue_present: FnQueuePresentKHR =
                dp!(c"vkQueuePresentKHR", FnQueuePresentKHR);
            let create_sem: FnCreateSemaphore = dp!(c"vkCreateSemaphore", FnCreateSemaphore);
            let destroy_sem: FnDestroySemaphore = dp!(c"vkDestroySemaphore", FnDestroySemaphore);
            let destroy_device: Option<FnDestroyDevice> =
                cast_fn(gdpa(device, c"vkDestroyDevice".as_ptr()));

            let mut queue: VkQueue = std::ptr::null_mut();
            get_queue(device, qfi, 0, &mut queue);
            let mut memprops = std::mem::zeroed::<PhysicalDeviceMemoryProperties>();
            vk_get_mem(pd, &mut memprops);

            // ── surface caps / format / present mode 协商 ──
            let mut caps = std::mem::zeroed::<SurfaceCapabilitiesKHR>();
            if get_surf_caps(pd, surface, &mut caps) != VK_SUCCESS {
                break 'run Err("vkGetPhysicalDeviceSurfaceCapabilitiesKHR 失败".into());
            }
            let mut fmt_count = 0u32;
            get_surf_formats(pd, surface, &mut fmt_count, std::ptr::null_mut());
            if fmt_count == 0 {
                break 'run Err("surface 无可用 format".into());
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
            // 8-bit RGBA/BGRA 闭集外 format 不猜通道序——fail-closed 确定性 Err。
            if chosen_format != FORMAT_B8G8R8A8_UNORM && chosen_format != FORMAT_R8G8B8A8_UNORM {
                break 'run Err(format!(
                    "surface format {chosen_format} 非 8-bit RGBA/BGRA(fail-closed 不猜通道序)"
                ));
            }
            let mut pm_count = 0u32;
            get_surf_present_modes(pd, surface, &mut pm_count, std::ptr::null_mut());
            let mut present_modes: Vec<u32> = vec![0u32; pm_count as usize];
            if pm_count > 0 {
                get_surf_present_modes(pd, surface, &mut pm_count, present_modes.as_mut_ptr());
            }
            if !present_modes.contains(&PRESENT_MODE_FIFO_KHR) {
                break 'run Err("surface 不含 FIFO present mode(spec 违例)".into());
            }
            let (ew, eh) = choose_present_extent(
                (caps.current_extent.width, caps.current_extent.height),
                width,
                height,
                (caps.min_image_extent.width, caps.min_image_extent.height),
                (caps.max_image_extent.width, caps.max_image_extent.height),
            );
            if (ew, eh) != (width, height) {
                break 'run Err(format!(
                    "surface extent {ew}x{eh} ≠ 请求 {}x{}(fail-closed;缩放 blit 面归后续波)",
                    width, height
                ));
            }
            let min_image_count = choose_min_image_count(caps.min_image_count, caps.max_image_count);

            // 句柄(全 null 初始;Err 路局部逆序拆除)。
            let mut swapchain: VkSwapchainKHR = VK_NULL_HANDLE;
            let mut staging: VkBuffer = VK_NULL_HANDLE;
            let mut staging_mem: VkDeviceMemory = VK_NULL_HANDLE;
            let mut cmdpool: VkCommandPool = VK_NULL_HANDLE;
            let mut sem_acquire: VkSemaphore = VK_NULL_HANDLE;
            let mut sem_done: VkSemaphore = VK_NULL_HANDLE;
            macro_rules! teardown_run {
                () => {{
                    if sem_done != VK_NULL_HANDLE {
                        destroy_sem(device, sem_done, std::ptr::null());
                    }
                    if sem_acquire != VK_NULL_HANDLE {
                        destroy_sem(device, sem_acquire, std::ptr::null());
                    }
                    if cmdpool != VK_NULL_HANDLE {
                        destroy_cmdpool(device, cmdpool, std::ptr::null());
                    }
                    if staging != VK_NULL_HANDLE {
                        destroy_buffer(device, staging, std::ptr::null());
                    }
                    if staging_mem != VK_NULL_HANDLE {
                        free_mem(device, staging_mem, std::ptr::null());
                    }
                    if swapchain != VK_NULL_HANDLE {
                        destroy_swapchain(device, swapchain, std::ptr::null());
                    }
                }};
            }

            // ── swapchain(TRANSFER_DST;image 归 swapchain 所有)──
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
                image_usage: IMAGE_USAGE_TRANSFER_DST,
                image_sharing_mode: SHARING_MODE_EXCLUSIVE,
                queue_family_index_count: 0,
                p_queue_family_indices: std::ptr::null(),
                pre_transform: caps.current_transform,
                composite_alpha: pick_composite_alpha(caps.supported_composite_alpha),
                present_mode: PRESENT_MODE_FIFO_KHR,
                clipped: 1,
                old_swapchain: VK_NULL_HANDLE,
            };
            if create_swapchain(device, &sci, std::ptr::null(), &mut swapchain) != VK_SUCCESS {
                teardown_run!();
                break 'run Err("vkCreateSwapchainKHR(g31) 失败".into());
            }
            let mut img_count = 0u32;
            get_swapchain_images(device, swapchain, &mut img_count, std::ptr::null_mut());
            if img_count == 0 {
                teardown_run!();
                break 'run Err("swapchain 无 image".into());
            }
            let mut images: Vec<VkImage> = vec![VK_NULL_HANDLE; img_count as usize];
            get_swapchain_images(device, swapchain, &mut img_count, images.as_mut_ptr());

            // ── staging(host-visible+coherent,容量 == w*h*4)──
            let bci = BufferCreateInfo {
                s_type: ST_BUFFER_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                size: (ew as u64) * (eh as u64) * 4,
                usage: BUFFER_USAGE_TRANSFER_SRC,
                sharing_mode: SHARING_MODE_EXCLUSIVE,
                queue_family_index_count: 0,
                p_queue_family_indices: std::ptr::null(),
            };
            if create_buffer(device, &bci, std::ptr::null(), &mut staging) != VK_SUCCESS {
                teardown_run!();
                break 'run Err("staging vkCreateBuffer 失败".into());
            }
            let mut req = std::mem::zeroed::<MemoryRequirements>();
            buf_mem_req(device, staging, &mut req);
            let Some(mt) = pick_mem_type(
                &memprops,
                req.memory_type_bits,
                MEM_HOST_VISIBLE | MEM_HOST_COHERENT,
            ) else {
                teardown_run!();
                break 'run Err("无 host-visible+coherent 内存类型".into());
            };
            let mai = MemoryAllocateInfo {
                s_type: ST_MEMORY_ALLOCATE_INFO,
                p_next: std::ptr::null(),
                allocation_size: req.size,
                memory_type_index: mt,
            };
            if alloc_mem(device, &mai, std::ptr::null(), &mut staging_mem) != VK_SUCCESS {
                teardown_run!();
                break 'run Err("staging vkAllocateMemory 失败".into());
            }
            bind_buf(device, staging, staging_mem, 0);

            // ── semaphores + cmdpool + cmd ──
            let sem_ci = SemaphoreCreateInfo {
                s_type: ST_SEMAPHORE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
            };
            if create_sem(device, &sem_ci, std::ptr::null(), &mut sem_acquire) != VK_SUCCESS
                || create_sem(device, &sem_ci, std::ptr::null(), &mut sem_done) != VK_SUCCESS
            {
                teardown_run!();
                break 'run Err("vkCreateSemaphore(g31) 失败".into());
            }
            let cpci = CommandPoolCreateInfo {
                s_type: ST_COMMAND_POOL_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0x2, // VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT
                queue_family_index: qfi,
            };
            if create_cmdpool(device, &cpci, std::ptr::null(), &mut cmdpool) != VK_SUCCESS {
                teardown_run!();
                break 'run Err("vkCreateCommandPool(g31) 失败".into());
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

            Ok(G31VkParts {
                instance,
                surface,
                device,
                queue,
                swapchain,
                images,
                staging,
                staging_mem,
                cmdpool,
                cmd,
                sem_acquire,
                sem_done,
                messenger,
                destroy_swapchain,
                get_swapchain_images,
                acquire_next,
                queue_present,
                queue_submit,
                queue_wait,
                create_swapchain,
                begin_cmd,
                end_cmd,
                cmd_barrier,
                cmd_copy_buf_img,
                map_mem,
                unmap_mem,
                destroy_buffer,
                free_mem,
                create_buffer,
                buf_mem_req,
                alloc_mem,
                bind_buf,
                staging_mem_type: mt,
                destroy_sem,
                destroy_cmdpool,
                destroy_device,
                destroy_surface,
                destroy_messenger,
                destroy_instance,
                pd,
                get_surf_caps,
                min_image_count,
                ext_w: ew,
                ext_h: eh,
                format: chosen_format,
                color_space: chosen_cs,
                validation_error,
            })
        };

        if result.is_err() {
            // device 级创建失败:拆 device → surface → messenger/instance(window 由调用方拆)。
            if let Some(dd) = cast_fn::<FnDestroyDevice>(gdpa(device, c"vkDestroyDevice".as_ptr()))
            {
                dd(device, std::ptr::null());
            }
            teardown_surface!();
        }
        result
    }
}

/// create_vk 中段产物(句柄 + 符号 + 协商态;成功路全量迁入会话结构)。
#[cfg(windows)]
struct G31VkParts {
    instance: VkInstance,
    surface: VkSurfaceKHR,
    device: VkDevice,
    queue: VkQueue,
    swapchain: VkSwapchainKHR,
    images: Vec<VkImage>,
    staging: VkBuffer,
    staging_mem: VkDeviceMemory,
    cmdpool: VkCommandPool,
    cmd: VkCommandBuffer,
    sem_acquire: VkSemaphore,
    sem_done: VkSemaphore,
    messenger: VkDebugUtilsMessengerEXT,
    destroy_swapchain: FnDestroySwapchainKHR,
    get_swapchain_images: FnGetSwapchainImagesKHR,
    acquire_next: FnAcquireNextImageKHR,
    queue_present: FnQueuePresentKHR,
    queue_submit: FnQueueSubmit,
    queue_wait: FnQueueWaitIdle,
    create_swapchain: FnCreateSwapchainKHR,
    begin_cmd: FnBeginCommandBuffer,
    end_cmd: FnEndCommandBuffer,
    cmd_barrier: FnCmdPipelineBarrier,
    cmd_copy_buf_img: FnCmdCopyBufferToImage,
    map_mem: FnMapMemory,
    unmap_mem: FnUnmapMemory,
    destroy_buffer: FnDestroyBuffer,
    free_mem: FnFreeMemory,
    // A3 resize:staging 容量随 extent 重建的创建侧符号 + host 内存型（create 期
    // 一次协商留存;rebuild 采用同一内存型——物理设备/堆不变式由会话生命周期承载）。
    create_buffer: FnCreateBuffer,
    buf_mem_req: FnGetBufferMemoryRequirements,
    alloc_mem: FnAllocateMemory,
    bind_buf: FnBindBufferMemory,
    staging_mem_type: u32,
    destroy_sem: FnDestroySemaphore,
    destroy_cmdpool: FnDestroyCommandPool,
    destroy_device: Option<FnDestroyDevice>,
    destroy_surface: FnDestroySurfaceKHR,
    destroy_messenger: Option<FnDestroyDebugUtilsMessengerEXT>,
    destroy_instance: FnDestroyInstance,
    pd: VkPhysicalDevice,
    get_surf_caps: FnGetPhysicalDeviceSurfaceCapabilitiesKHR,
    min_image_count: u32,
    ext_w: u32,
    ext_h: u32,
    format: u32,
    color_space: u32,
    validation_error: Box<std::sync::atomic::AtomicBool>,
}

#[cfg(windows)]
impl Drop for ExternalImagePresent {
    /// 逆序拆除(queue idle → semaphores/cmdpool/staging/swapchain → device → surface →
    /// messenger → instance → window/class;swapchain image 归 swapchain 所有不单独销毁)。
    fn drop(&mut self) {
        // SAFETY: 见结构体 U27 扩注镜像;各句柄非 null 才销毁,顺序与创建严格逆。
        unsafe {
            (self.queue_wait)(self.queue);
            if self.sem_done != VK_NULL_HANDLE {
                (self.destroy_sem)(self.device, self.sem_done, std::ptr::null());
            }
            if self.sem_acquire != VK_NULL_HANDLE {
                (self.destroy_sem)(self.device, self.sem_acquire, std::ptr::null());
            }
            if self.cmdpool != VK_NULL_HANDLE {
                (self.destroy_cmdpool)(self.device, self.cmdpool, std::ptr::null());
            }
            if self.staging != VK_NULL_HANDLE {
                (self.destroy_buffer)(self.device, self.staging, std::ptr::null());
            }
            if self.staging_mem != VK_NULL_HANDLE {
                (self.free_mem)(self.device, self.staging_mem, std::ptr::null());
            }
            if self.swapchain != VK_NULL_HANDLE {
                (self.destroy_swapchain)(self.device, self.swapchain, std::ptr::null());
            }
            if let Some(dd) = self.destroy_device {
                dd(self.device, std::ptr::null());
            }
            (self.destroy_surface)(self.instance, self.surface, std::ptr::null());
            if let Some(dm) = self.destroy_messenger {
                if self.messenger != VK_NULL_HANDLE {
                    dm(self.instance, self.messenger, std::ptr::null());
                }
            }
            (self.destroy_instance)(self.instance, std::ptr::null());
            win32::DestroyWindow(self.hwnd);
            win32::UnregisterClassW(self.class_name.as_ptr(), self.hinstance);
            // 输入静态态注销(单窗口槽;随后创建的新会话自复位重注册)。
            G31_INPUT
                .hwnd
                .store(0, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

/// 非 Windows:win32 surface 不可用(android present = G-MB1-7 尾门)——确定性 `Err`。
#[cfg(not(windows))]
pub struct ExternalImagePresent;

#[cfg(not(windows))]
impl ExternalImagePresent {
    pub fn create(_width: u32, _height: u32, _title: &str, _visible: bool) -> Result<Self, String> {
        Err("win32 present: windows-only (android present = G-MB1-7 尾门)".into())
    }

    pub fn extent(&self) -> (u32, u32) {
        (0, 0)
    }

    pub fn channel_order(&self) -> &'static str {
        "unreachable(windows-only)"
    }

    pub fn counts(&self) -> ExternalPresentCounts {
        ExternalPresentCounts {
            frames_presented: 0,
            swapchain_rebuilds: 0,
        }
    }

    pub fn present_rgba8(&mut self, _pixels: &[u8]) -> Result<(), String> {
        Err("win32 present: windows-only (android present = G-MB1-7 尾门)".into())
    }

    pub fn resize(&mut self, _width: u32, _height: u32) -> Result<bool, String> {
        Err("win32 present: windows-only (android present = G-MB1-7 尾门)".into())
    }

    pub fn storm_wm_size(&self, _width: u32, _height: u32, _minimized: bool) {}

    pub fn storm_set_window_size(&self, _width: u32, _height: u32) {}

    pub fn poll_input(&mut self) -> ExternalInputFrame {
        ExternalInputFrame::default()
    }
}
