//! G6.5 Taichi Vulkan AOT spike — TiRT(`taichi_c_api`)FFI 边界(RFC-0017 §4.E;U43)。
//!
//! **动态装载**(非链接期绑定,镜像 [`crate::sys`] nvcuda.dll / [`crate::vk`] vulkan-1.dll
//! 纪律):`taichi_c_api.dll` 绝对路径经环境变量 [`TIRT_DLL_ENV`] 给出,
//! `LoadLibraryW`/`dlopen` + `GetProcAddress`/`dlsym` 运行时装载;未设 / 文件不存在 /
//! 符号缺失 → 确定性 [`TirtError`] **fail-closed**(P-01,绝不静默回退、不搜默认路径)。
//! 零外部依赖(手写 extern 薄 FFI,对齐 crate 无依赖纪律)。
//!
//! **设备面**(RFC-0017 §4.E2 明示允许「并行设备上下文」):经 vk.rs
//! `create_tirt_vulkan_device`(pub(crate) 新增,既有入口 0-byte)自建一套
//! instance(api 1.1)+ device + compute/graphics queue,九字段
//! `TiVulkanRuntimeInteropInfo` 注入 `ti_import_vulkan_runtime`;与 render_exec 的
//! 每帧自建 device 无共享状态。
//!
//! **同步纪律**(§4.E2):`ti_launch_kernel` 后 `ti_flush` + `ti_wait` 全排空,再
//! `ti_export_vulkan_memory` 导出;同 device compute queue 上 `vkCmdCopyBuffer` →
//! host-visible buffer,submit 后 `vkQueueWaitIdle` 再 map 读回(单 queue 全序,
//! host 同步后无在途写)。释放顺序 = copy 完成(queue wait)→ TiRT free/destroy
//! 逆序(module → memory → runtime)→ Vk 设备上下文销毁,经 RAII guard 反向
//! 声明序单点兑现。
//!
//! # SAFETY(U43,G6.5 TiRT FFI 边界;沿 U26/U32 审计模式)
//! 对上全 safe(无 `unsafe` 签名)。全部 `unsafe` 内聚本模块,每块携 `// SAFETY:`:
//! TiRT 函数指针签名与 `taichi_core.h`/`taichi_vulkan.h`(taichi 1.7.4,
//! `TI_C_API_VERSION 1007000`)逐一对应(`TiBool/TiFlags`=u32,C 枚举=i32,
//! x64 `extern "system"`);`#[repr(C)]` 结构(TiMemoryAllocateInfo/TiNdShape/TiNdArray/
//! TiArgument/TiVulkanRuntimeInteropInfo/TiVulkanMemoryInteropInfo + readback 用
//! VkStruct)编译期 `const assert!` 布局锚定(24/68/152/160/72/40 等);句柄
//! (runtime/memory/module/kernel + instance/device/queue/buffer/cmdpool)经 RAII
//! guard 线性配对、逆序销毁,早退路径同走销毁序,无泄漏/双释放;loader 不
//! `FreeLibrary`/`dlclose`(进程常驻,镜像 U1 纪律)。gate feature `taichi-tirt`
//! (依赖 `vulkan`)默认关闭,default/vulkan 构建零改动零回归。

use core::ffi::{c_char, c_void};
use std::fmt;

use crate::vk::{self, FnGetInstanceProcAddr, PfnVoid, cast_fn, load_vulkan_loader};

/// TiRT 动态库路径环境变量名(绝对路径;未设/不存在/符号缺失 → fail-closed Err)。
pub const TIRT_DLL_ENV: &str = "RURIX_TAICHI_C_API_DLL";

// ── TiRT C API 类型面(taichi_core.h / taichi_vulkan.h,taichi 1.7.4 头核对) ──────
// 不透明句柄 = 指针;C 枚举 = i32;`TiBool`/`TiFlags` = u32(taichi_core.h L246/L266)。
type TiRuntime = *mut c_void;
type TiAotModule = *mut c_void;
type TiMemory = *mut c_void;
type TiKernel = *mut c_void;

const TI_ERROR_SUCCESS: i32 = 0;
/// `TI_ARGUMENT_TYPE_NDARRAY`(taichi_core.h L461)。
const TI_ARGUMENT_TYPE_NDARRAY: i32 = 2;
/// `TI_DATA_TYPE_F32`(taichi_core.h L426)。
const TI_DATA_TYPE_F32: i32 = 1;
/// `TI_MEMORY_USAGE_STORAGE_BIT`(taichi_core.h L477;kernel 参数内存必须置位)。
const TI_MEMORY_USAGE_STORAGE_BIT: u32 = 0x1;

/// `TiMemoryAllocateInfo`(taichi_core.h L490-502)。
#[repr(C)]
struct TiMemoryAllocateInfo {
    size: u64,
    host_write: u32,
    host_read: u32,
    export_sharing: u32,
    usage: u32,
}
const _: () = assert!(size_of::<TiMemoryAllocateInfo>() == 24);

/// `TiNdShape`(taichi_core.h L521-526;dim_count 之后的维度被忽略)。
#[repr(C)]
#[derive(Clone, Copy)]
struct TiNdShape {
    dim_count: u32,
    dims: [u32; 16],
}
const _: () = assert!(size_of::<TiNdShape>() == 68);

/// `TiNdArray`(taichi_core.h L531-541;scalar ndarray 的 elem_shape 必须为空)。
#[repr(C)]
#[derive(Clone, Copy)]
struct TiNdArray {
    memory: TiMemory,
    shape: TiNdShape,
    elem_shape: TiNdShape,
    elem_type: i32,
}
const _: () = assert!(size_of::<TiNdArray>() == 152);

/// `TiArgumentValue`(taichi_core.h L840-855):spike 唯一用型为 ndarray(最大成员,
/// 单成员布局与全集一致);i32/f32/scalar/tensor 成员省略。
#[repr(C)]
#[derive(Clone, Copy)]
union TiArgumentValue {
    ndarray: TiNdArray,
}

/// `TiArgument`(taichi_core.h L860-865)。
#[repr(C)]
struct TiArgument {
    type_: i32,
    value: TiArgumentValue,
}
const _: () = assert!(size_of::<TiArgument>() == 160);

/// `TiVulkanRuntimeInteropInfo`(taichi_vulkan.h L26-47 九字段;compute/graphics
/// queue 允许同 queue,头文件注释明示)。
#[repr(C)]
struct TiVulkanRuntimeInteropInfo {
    get_instance_proc_addr: FnGetInstanceProcAddr,
    api_version: u32,
    instance: *mut c_void,
    physical_device: *mut c_void,
    device: *mut c_void,
    compute_queue: *mut c_void,
    compute_queue_family_index: u32,
    graphics_queue: *mut c_void,
    graphics_queue_family_index: u32,
}
const _: () = assert!(size_of::<TiVulkanRuntimeInteropInfo>() == 72);

/// `TiVulkanMemoryInteropInfo`(taichi_vulkan.h L53-66;`ti_export_vulkan_memory` 出参)。
#[repr(C)]
struct TiVulkanMemoryInteropInfo {
    buffer: u64,
    size: u64,
    usage: u32,
    memory: u64,
    offset: u64,
}
const _: () = assert!(size_of::<TiVulkanMemoryInteropInfo>() == 40);

// ── TiRT 函数指针类型(`TI_API_CALL` = Windows x64 `__stdcall` ≡ `extern "system"`) ──
type FnTiGetVersion = unsafe extern "system" fn() -> u32;
type FnTiGetLastError = unsafe extern "system" fn(*mut u64, *mut c_char) -> i32;
type FnTiImportVulkanRuntime =
    unsafe extern "system" fn(*const TiVulkanRuntimeInteropInfo) -> TiRuntime;
type FnTiDestroyRuntime = unsafe extern "system" fn(TiRuntime);
type FnTiAllocateMemory =
    unsafe extern "system" fn(TiRuntime, *const TiMemoryAllocateInfo) -> TiMemory;
type FnTiFreeMemory = unsafe extern "system" fn(TiRuntime, TiMemory);
type FnTiCreateAotModule = unsafe extern "system" fn(TiRuntime, *const c_void, u64) -> TiAotModule;
type FnTiDestroyAotModule = unsafe extern "system" fn(TiAotModule);
type FnTiGetAotModuleKernel = unsafe extern "system" fn(TiAotModule, *const c_char) -> TiKernel;
type FnTiLaunchKernel = unsafe extern "system" fn(TiRuntime, TiKernel, u32, *const TiArgument);
type FnTiFlush = unsafe extern "system" fn(TiRuntime);
type FnTiWait = unsafe extern "system" fn(TiRuntime);
type FnTiExportVulkanMemory =
    unsafe extern "system" fn(TiRuntime, TiMemory, *mut TiVulkanMemoryInteropInfo);

// ── OS 动态装载缝(镜像 sys.rs loader 纪律;Windows 走宽字符 LoadLibraryW,
//    环境变量路径可能含非 ASCII 目录名) ────────────────────────────────────────────
#[cfg(windows)]
mod loader {
    use core::ffi::{c_char, c_void};
    unsafe extern "system" {
        fn LoadLibraryW(name: *const u16) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const c_char) -> *mut c_void;
    }
    /// 装载指定绝对路径的 DLL(失败 → null)。
    pub(super) fn open(path: &str) -> *mut c_void {
        let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: wide 为 NUL 结尾 UTF-16;LoadLibraryW 为 Win32 稳定 ABI(kernel32)。
        unsafe { LoadLibraryW(wide.as_ptr()) }
    }
    /// # Safety
    /// `lib` 为 `open` 返回的有效模块句柄;`name` NUL 结尾。
    pub(super) unsafe fn sym(lib: *mut c_void, name: *const c_char) -> *mut c_void {
        // SAFETY: 调用方保证 `lib` 有效、`name` NUL 结尾;GetProcAddress 为 Win32 稳定 ABI。
        unsafe { GetProcAddress(lib, name) }
    }
}

#[cfg(not(windows))]
mod loader {
    use core::ffi::{c_char, c_void};
    use std::ffi::CString;
    unsafe extern "C" {
        fn dlopen(filename: *const c_char, flag: i32) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    }
    const RTLD_NOW: i32 = 2; // 立即绑定全部符号(POSIX 通用值,glibc/musl 一致)。
    /// 装载指定绝对路径的共享库(失败/路径含内嵌 NUL → null)。
    pub(super) fn open(path: &str) -> *mut c_void {
        let Ok(c) = CString::new(path) else {
            return std::ptr::null_mut();
        };
        // SAFETY: c 为 NUL 结尾;dlopen 为 POSIX 稳定 ABI(libc)。
        unsafe { dlopen(c.as_ptr(), RTLD_NOW) }
    }
    /// # Safety
    /// `lib` 为 `open` 返回的有效模块句柄;`name` NUL 结尾。
    pub(super) unsafe fn sym(lib: *mut c_void, name: *const c_char) -> *mut c_void {
        // SAFETY: 调用方保证 `lib` 有效、`name` NUL 结尾;dlsym 为 POSIX 稳定 ABI。
        unsafe { dlsym(lib, name) }
    }
}

/// 符号地址 → 类型化函数指针;null → None(镜像 sys::cast_fn)。
///
/// # Safety
/// `raw` 须为 taichi_c_api 库中同名导出符号、ABI 与 `T` 一致的函数地址(或 null)。
unsafe fn cast_sym<T: Copy>(raw: *mut c_void) -> Option<T> {
    if raw.is_null() {
        return None;
    }
    debug_assert_eq!(size_of::<T>(), size_of::<*mut c_void>());
    // SAFETY: raw 非 null(已查);T 为指针宽度函数指针(宽度断言校核);调用方保证
    // 符号名 ⇔ `T` 签名 ⇔ taichi_core.h/taichi_vulkan.h 声明逐一对应。
    Some(unsafe { std::mem::transmute_copy::<*mut c_void, T>(&raw) })
}

/// tirt 库层错误枚举(Display;无新 RX 码,RFC-0017 §4.E spike 口径)。
#[derive(Debug)]
pub enum TirtError {
    /// 环境变量未设 / 路径不存在 / 动态装载失败(fail-closed,不搜默认路径)。
    DllNotFound(String),
    /// DLL 在位但导出符号缺失(版本不符)。
    SymbolMissing(String),
    /// Vulkan loader / 物理设备 / queue 家族 / vk 侧对象创建不可用。
    DeviceUnavailable(String),
    /// TiRT 调用失败(附 `ti_get_last_error` 诊断文本)。
    TaichiError(String),
    /// `ti_export_vulkan_memory` 导出面失败(空句柄 / 尺寸不足 / usage 缺 TRANSFER_SRC)。
    BufferExport(String),
    /// readback 路径不一致(map 失败 / 读回长度异常)。
    ReadbackMismatch(String),
}

impl fmt::Display for TirtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TirtError::DllNotFound(m) => write!(f, "taichi_c_api 库不可用: {m}"),
            TirtError::SymbolMissing(m) => write!(f, "taichi_c_api 符号缺失: {m}"),
            TirtError::DeviceUnavailable(m) => write!(f, "vulkan 设备不可用: {m}"),
            TirtError::TaichiError(m) => write!(f, "taichi 运行时错误: {m}"),
            TirtError::BufferExport(m) => write!(f, "tirt buffer 导出失败: {m}"),
            TirtError::ReadbackMismatch(m) => write!(f, "tirt readback 不一致: {m}"),
        }
    }
}

impl std::error::Error for TirtError {}

/// 已装载的 TiRT C API 入口集(13 个符号,函数指针 Send + Sync;loader 进程常驻)。
#[derive(Clone, Copy, Debug)]
pub struct TirtApi {
    ti_get_version: FnTiGetVersion,
    ti_get_last_error: FnTiGetLastError,
    ti_import_vulkan_runtime: FnTiImportVulkanRuntime,
    ti_destroy_runtime: FnTiDestroyRuntime,
    ti_allocate_memory: FnTiAllocateMemory,
    ti_free_memory: FnTiFreeMemory,
    ti_create_aot_module: FnTiCreateAotModule,
    ti_destroy_aot_module: FnTiDestroyAotModule,
    ti_get_aot_module_kernel: FnTiGetAotModuleKernel,
    ti_launch_kernel: FnTiLaunchKernel,
    ti_flush: FnTiFlush,
    ti_wait: FnTiWait,
    ti_export_vulkan_memory: FnTiExportVulkanMemory,
}

impl TirtApi {
    /// 装载 [`TIRT_DLL_ENV`] 指定的 `taichi_c_api` 动态库并解析全部所需符号。
    ///
    /// 未设环境变量 / 路径不存在 / 装载失败 / 任一符号缺失 → 确定性 `Err`
    /// (fail-closed P-01,绝不静默回退、不搜默认路径)。
    pub fn load() -> Result<TirtApi, TirtError> {
        let path = std::env::var(TIRT_DLL_ENV).map_err(|_| {
            TirtError::DllNotFound(format!("环境变量 {TIRT_DLL_ENV} 未设置(不搜默认路径)"))
        })?;
        if path.is_empty() {
            return Err(TirtError::DllNotFound(format!(
                "环境变量 {TIRT_DLL_ENV} 为空字符串"
            )));
        }
        if !std::path::Path::new(&path).is_file() {
            return Err(TirtError::DllNotFound(format!(
                "库文件不存在或不是常规文件: {path}"
            )));
        }
        let lib = loader::open(&path);
        if lib.is_null() {
            return Err(TirtError::DllNotFound(format!(
                "动态装载失败(LoadLibrary/dlopen 拒绝): {path}"
            )));
        }
        macro_rules! need {
            ($name:literal, $ty:ty) => {{
                // SAFETY: lib 为本函数刚装载成功的模块句柄;符号名为 NUL 结尾字面量;
                // cast_sym 内 null 校验 + 指针宽度断言,符号名 ⇔ 签名 ⇔ 头文件声明对应。
                unsafe { cast_sym::<$ty>(loader::sym(lib, $name.as_ptr())) }
                    .ok_or_else(|| TirtError::SymbolMissing($name.to_string_lossy().into_owned()))?
            }};
        }
        Ok(TirtApi {
            ti_get_version: need!(c"ti_get_version", FnTiGetVersion),
            ti_get_last_error: need!(c"ti_get_last_error", FnTiGetLastError),
            ti_import_vulkan_runtime: need!(c"ti_import_vulkan_runtime", FnTiImportVulkanRuntime),
            ti_destroy_runtime: need!(c"ti_destroy_runtime", FnTiDestroyRuntime),
            ti_allocate_memory: need!(c"ti_allocate_memory", FnTiAllocateMemory),
            ti_free_memory: need!(c"ti_free_memory", FnTiFreeMemory),
            ti_create_aot_module: need!(c"ti_create_aot_module", FnTiCreateAotModule),
            ti_destroy_aot_module: need!(c"ti_destroy_aot_module", FnTiDestroyAotModule),
            ti_get_aot_module_kernel: need!(c"ti_get_aot_module_kernel", FnTiGetAotModuleKernel),
            ti_launch_kernel: need!(c"ti_launch_kernel", FnTiLaunchKernel),
            ti_flush: need!(c"ti_flush", FnTiFlush),
            ti_wait: need!(c"ti_wait", FnTiWait),
            ti_export_vulkan_memory: need!(c"ti_export_vulkan_memory", FnTiExportVulkanMemory),
        })
    }

    /// TiRT C API 版本(`ti_get_version`,如 1007000 = 1.7.0;诊断留痕用)。
    #[must_use]
    pub fn version(&self) -> u32 {
        // SAFETY: ti_get_version 无入参纯查询(taichi_core.h L881);符号已经 load 校验。
        unsafe { (self.ti_get_version)() }
    }
}

/// `ti_get_last_error` 诊断文本(单次定长缓冲调用;message 截断可接受,错误码权威)。
fn last_error_text(api: &TirtApi) -> String {
    let mut buf = [0u8; 1024];
    let mut cap = buf.len() as u64;
    // SAFETY: buf 为 cap 字节有效可写缓冲;出参 cap 回写实际长度(taichi_core.h L905)。
    let code = unsafe { (api.ti_get_last_error)(&mut cap, buf.as_mut_ptr().cast::<c_char>()) };
    if code == TI_ERROR_SUCCESS {
        return "TiError=SUCCESS".to_owned();
    }
    let end = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
    let text = String::from_utf8_lossy(&buf[..end]).into_owned();
    format!("TiError={code} {text}")
}

/// 并行 Vk 设备上下文句柄包(值拷贝,`usize` 承载避免裸指针逃逸进公共签名;
/// 由 spike 链自建,供 [`TirtRuntime::import_vulkan`] 九字段注入)。
#[derive(Debug, Clone, Copy)]
pub struct VulkanHandles {
    /// instance 创建所用 api version(≥ `VK_MAKE_API_VERSION(0,1,1,0)`)。
    pub api_version: u32,
    /// `VkInstance` 句柄值。
    pub instance: usize,
    /// `VkPhysicalDevice` 句柄值。
    pub physical_device: usize,
    /// `VkDevice` 句柄值。
    pub device: usize,
    /// compute `VkQueue` 句柄值。
    pub compute_queue: usize,
    /// 含 `VK_QUEUE_COMPUTE_BIT` 的 queue family 下标。
    pub compute_queue_family_index: u32,
    /// graphics `VkQueue` 句柄值(允许与 compute 同 queue,头文件注释明示)。
    pub graphics_queue: usize,
    /// 含 `VK_QUEUE_GRAPHICS_BIT` 的 queue family 下标。
    pub graphics_queue_family_index: u32,
}

/// TiRT runtime RAII 句柄(Drop = `ti_destroy_runtime`;单一所有权,非 Clone)。
pub struct TirtRuntime {
    raw: TiRuntime,
    api: TirtApi,
}

impl TirtRuntime {
    /// 注入并行 Vk 设备上下文,创建 TiRT runtime(`ti_import_vulkan_runtime`)。
    ///
    /// `handles` 的句柄在返回的 runtime 存活期间必须保持有效(由调用方设备上下文
    /// RAII 晚于 runtime 销毁保证,反向声明序)。
    pub fn import_vulkan(api: &TirtApi, handles: &VulkanHandles) -> Result<TirtRuntime, TirtError> {
        let gipa = load_vulkan_loader().ok_or_else(|| {
            TirtError::DeviceUnavailable(
                "vulkan loader (vulkan-1.dll/libvulkan.so) 不可用,gipa 无法取得".to_owned(),
            )
        })?;
        let info = TiVulkanRuntimeInteropInfo {
            get_instance_proc_addr: gipa,
            api_version: handles.api_version,
            instance: handles.instance as *mut c_void,
            physical_device: handles.physical_device as *mut c_void,
            device: handles.device as *mut c_void,
            compute_queue: handles.compute_queue as *mut c_void,
            compute_queue_family_index: handles.compute_queue_family_index,
            graphics_queue: handles.graphics_queue as *mut c_void,
            graphics_queue_family_index: handles.graphics_queue_family_index,
        };
        // SAFETY: info 为栈上有效结构,与 taichi_vulkan.h 九字段逐字段对应(布局锚定);
        // gipa/instance/device/queue 为调用方持有的有效 Vulkan 句柄(并行设备上下文,
        // §4.E2);空返回经 ti_get_last_error 附诊断映射 Err。
        let raw = unsafe { (api.ti_import_vulkan_runtime)(&info) };
        if raw.is_null() {
            return Err(TirtError::TaichiError(format!(
                "ti_import_vulkan_runtime 返回空句柄: {}",
                last_error_text(api)
            )));
        }
        Ok(TirtRuntime { raw, api: *api })
    }

    /// 底层 `TiRuntime` 句柄值(诊断/证据留痕用)。
    #[must_use]
    pub fn raw_handle(&self) -> usize {
        self.raw as usize
    }
}

impl Drop for TirtRuntime {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            // SAFETY: raw 由 ti_import_vulkan_runtime 成功返回且本类型独占,Drop 仅一次;
            // module/memory guard 声明晚于本 runtime,反向 drop 先销毁(module → memory →
            // runtime 逆序,RFC-0017 §4.E2);TiRT 侧在途命令已经 ti_wait 排空。
            unsafe { (self.api.ti_destroy_runtime)(self.raw) };
        }
    }
}

/// spike 证据结果(device 名 + 读回统计;由 demo/测试断言判据)。
#[derive(Debug, Clone)]
pub struct ParticlesSpikeOutcome {
    /// 物理设备名(`vkGetPhysicalDeviceProperties.deviceName`)。
    pub device_name: String,
    /// 粒子数(= ndarray shape = 读回 f32 个数)。
    pub particle_count: u32,
    /// 读回非零元素个数。
    pub nonzero_count: u32,
    /// 读回前 ≤4 个元素值(判据抽查)。
    pub first_values: Vec<f32>,
    /// `ti_export_vulkan_memory` 报告的导出 buffer 字节数(≥ 请求值,含对齐余量)。
    pub exported_buffer_size: u64,
}

// ── 内部 RAII guard(声明序 = 销毁逆序的单点事实源) ─────────────────────────────

/// 并行 Vk 设备上下文 guard(Drop = vk.rs `destroy_tirt_vulkan_device`)。
struct VkDevGuard {
    gipa: FnGetInstanceProcAddr,
    ctx: vk::TirtVulkanDevice,
}

impl Drop for VkDevGuard {
    fn drop(&mut self) {
        // SAFETY: ctx 由 create_tirt_vulkan_device 成功返回且仅此一处销毁(配对一次,
        // 逆序 device → instance 由 vk.rs 函数兑现);TiRT runtime 已先销毁(声明序)。
        unsafe { vk::destroy_tirt_vulkan_device(self.gipa, &self.ctx) };
    }
}

/// TiRT 内存 guard(Drop = `ti_free_memory`;导出的 VkBuffer 随之一并释放)。
struct TirtMemoryGuard {
    api: TirtApi,
    runtime: TiRuntime,
    raw: TiMemory,
}

impl Drop for TirtMemoryGuard {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            // SAFETY: raw 由 ti_allocate_memory 成功返回;runtime 存活(本 guard 声明晚于
            // runtime,反向 drop 先于此);copy 已 vkQueueWaitIdle 完成,无在途读写。
            unsafe { (self.api.ti_free_memory)(self.runtime, self.raw) };
        }
    }
}

/// TiRT AOT module guard(Drop = `ti_destroy_aot_module`)。
struct TirtAotModuleGuard {
    api: TirtApi,
    raw: TiAotModule,
}

impl Drop for TirtAotModuleGuard {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            // SAFETY: raw 由 ti_create_aot_module 成功返回且本 guard 独占,Drop 仅一次。
            unsafe { (self.api.ti_destroy_aot_module)(self.raw) };
        }
    }
}

/// G6.5 particles spike 全链(库面,对上全 safe):
/// 装载 TiRT → 建并行 Vk 设备上下文 → import runtime →
/// `ti_allocate_memory`(export_sharing + STORAGE_BIT,count*4 字节)→
/// `ti_create_aot_module`(tcm 字节)→ `ti_get_aot_module_kernel("fill_particles")` →
/// ndarray launch → `ti_flush`+`ti_wait` → `ti_export_vulkan_memory` 取 VkBuffer →
/// 同 device `vkCmdCopyBuffer` → host-visible readback → 统计非零 + 前 ≤4 值。
pub fn run_particles_spike(
    tcm_bytes: &[u8],
    particle_count: u32,
) -> Result<ParticlesSpikeOutcome, TirtError> {
    if tcm_bytes.is_empty() {
        return Err(TirtError::TaichiError("tcm 字节为空".to_owned()));
    }
    if particle_count == 0 {
        return Err(TirtError::TaichiError("particle_count 为 0".to_owned()));
    }
    let api = TirtApi::load()?;
    let gipa = load_vulkan_loader().ok_or_else(|| {
        TirtError::DeviceUnavailable("vulkan loader (vulkan-1.dll/libvulkan.so) 不可用".to_owned())
    })?;

    // 并行设备上下文(RFC-0017 §4.E2):独立 instance+device,与 render_exec 无共享状态。
    // SAFETY: 成功返回的句柄包由 VkDevGuard 单点销毁(配对一次,逆序 device → instance)。
    let ctx = unsafe { vk::create_tirt_vulkan_device(gipa, c"rurix-tirt-spike") }
        .map_err(TirtError::DeviceUnavailable)?;
    let device_name = ctx.device_name.clone();
    let dev = VkDevGuard { gipa, ctx };

    let handles = VulkanHandles {
        api_version: dev.ctx.api_version,
        instance: dev.ctx.instance as usize,
        physical_device: dev.ctx.physical_device as usize,
        device: dev.ctx.device as usize,
        compute_queue: dev.ctx.compute_queue as usize,
        compute_queue_family_index: dev.ctx.compute_queue_family,
        graphics_queue: dev.ctx.graphics_queue as usize,
        graphics_queue_family_index: dev.ctx.graphics_queue_family,
    };
    let runtime = TirtRuntime::import_vulkan(&api, &handles)?;

    // 设备内存:export_sharing(导出前提)+ STORAGE_BIT(kernel ndarray 参数要求)。
    let bytes = u64::from(particle_count) * 4;
    let mai = TiMemoryAllocateInfo {
        size: bytes,
        host_write: 0,
        host_read: 0,
        export_sharing: 1,
        usage: TI_MEMORY_USAGE_STORAGE_BIT,
    };
    // SAFETY: mai 为栈上有效结构;runtime 有效;空返回经 last_error 附诊断映射 Err。
    let mem_raw = unsafe { (api.ti_allocate_memory)(runtime.raw, &mai) };
    if mem_raw.is_null() {
        return Err(TirtError::TaichiError(format!(
            "ti_allocate_memory({bytes}B, export_sharing) 失败: {}",
            last_error_text(&api)
        )));
    }
    let memory = TirtMemoryGuard {
        api,
        runtime: runtime.raw,
        raw: mem_raw,
    };

    // SAFETY: tcm_bytes 调用期存活;size = 实际字节长;空返回附诊断映射 Err。
    let mod_raw = unsafe {
        (api.ti_create_aot_module)(
            runtime.raw,
            tcm_bytes.as_ptr().cast::<c_void>(),
            tcm_bytes.len() as u64,
        )
    };
    if mod_raw.is_null() {
        return Err(TirtError::TaichiError(format!(
            "ti_create_aot_module 失败(.tcm {}B): {}",
            tcm_bytes.len(),
            last_error_text(&api)
        )));
    }
    let module = TirtAotModuleGuard { api, raw: mod_raw };

    // SAFETY: module 有效;kernel 名为 NUL 结尾字面量。
    let kernel = unsafe { (api.ti_get_aot_module_kernel)(module.raw, c"fill_particles".as_ptr()) };
    if kernel.is_null() {
        return Err(TirtError::TaichiError(format!(
            "ti_get_aot_module_kernel(\"fill_particles\") 未找到: {}",
            last_error_text(&api)
        )));
    }

    // ndarray 参数:shape=(N,)、elem_shape 空(scalar)、f32。
    let mut shape = TiNdShape {
        dim_count: 1,
        dims: [0; 16],
    };
    shape.dims[0] = particle_count;
    let ndarray = TiNdArray {
        memory: memory.raw,
        shape,
        elem_shape: TiNdShape {
            dim_count: 0,
            dims: [0; 16],
        },
        elem_type: TI_DATA_TYPE_F32,
    };
    let args = [TiArgument {
        type_: TI_ARGUMENT_TYPE_NDARRAY,
        value: TiArgumentValue { ndarray },
    }];
    // SAFETY: args 长度 1 与 kernel 形参表匹配(fill_particles(p: ndarray<f32,1D>));
    // kernel/runtime 有效;ndarray 引用的 memory.raw(export_sharing 分配)在 launch
    // 完成前由 memory guard 保活;launch 后排空经 ti_flush+ti_wait(§4.E2 同步纪律)。
    unsafe { (api.ti_launch_kernel)(runtime.raw, kernel, 1, args.as_ptr()) };
    // SAFETY: runtime 有效;flush 提交全部在途 device commands。
    unsafe { (api.ti_flush)(runtime.raw) };
    // SAFETY: runtime 有效;wait 阻塞至全部在途命令完成(之后再导出/copy)。
    unsafe { (api.ti_wait)(runtime.raw) };

    // 导出 VkBuffer(ti_wait 后,无在途写)。
    let mut interop = TiVulkanMemoryInteropInfo {
        buffer: 0,
        size: 0,
        usage: 0,
        memory: 0,
        offset: 0,
    };
    // SAFETY: runtime/memory 有效;interop 为栈上有效出参。
    unsafe { (api.ti_export_vulkan_memory)(runtime.raw, memory.raw, &mut interop) };
    if interop.buffer == 0 {
        return Err(TirtError::BufferExport(format!(
            "ti_export_vulkan_memory 返回空 VkBuffer: {}",
            last_error_text(&api)
        )));
    }
    if interop.size < bytes {
        return Err(TirtError::BufferExport(format!(
            "导出 buffer 尺寸 {}B 小于请求 {bytes}B",
            interop.size
        )));
    }
    if interop.usage & VK_BUFFER_USAGE_TRANSFER_SRC == 0 {
        return Err(TirtError::BufferExport(format!(
            "导出 buffer usage={:#010x} 不含 TRANSFER_SRC_BIT,vkCmdCopyBuffer 不可用",
            interop.usage
        )));
    }

    // 同 device compute queue 上 copy → host-visible readback(copy 完成后再释放,§4.E2)。
    let values = copy_exported_to_host(gipa, &dev.ctx, interop.buffer, bytes, particle_count)?;

    let nonzero_count = values.iter().filter(|v| **v != 0.0).count() as u32;
    let first_values: Vec<f32> = values.iter().take(4).copied().collect();
    Ok(ParticlesSpikeOutcome {
        device_name,
        particle_count,
        nonzero_count,
        first_values,
        exported_buffer_size: interop.size,
    })
    // 销毁序(guard 反向 drop):module → memory → runtime → dev(并行设备上下文),
    // 其中 module/memory/runtime = TiRT free/destroy 逆序(RFC-0017 §4.E2)。
}

// ── vk readback 腿(本模块内聚的 Vulkan FFI 子集;U43 同一边界) ──────────────────

const VK_SUCCESS: i32 = 0;
const VK_BUFFER_USAGE_TRANSFER_SRC: u32 = 0x1;
const VK_BUFFER_USAGE_TRANSFER_DST: u32 = 0x2;
const VK_MEM_HOST_VISIBLE: u32 = 0x2;
const VK_MEM_HOST_COHERENT: u32 = 0x4;
const VK_ST_BUFFER_CREATE_INFO: u32 = 12;
const VK_ST_MEMORY_ALLOCATE_INFO: u32 = 5;
const VK_ST_COMMAND_POOL_CREATE_INFO: u32 = 39;
const VK_ST_COMMAND_BUFFER_ALLOCATE_INFO: u32 = 40;
const VK_ST_COMMAND_BUFFER_BEGIN_INFO: u32 = 42;
const VK_ST_SUBMIT_INFO: u32 = 4;
const VK_SHARING_MODE_EXCLUSIVE: u32 = 0;
const VK_CMD_BUFFER_LEVEL_PRIMARY: u32 = 0;
const VK_CMD_BUFFER_USAGE_ONE_TIME_SUBMIT: u32 = 0x1;

#[repr(C)]
struct VkBufferCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: u32,
    size: u64,
    usage: u32,
    sharing_mode: u32,
    queue_family_index_count: u32,
    p_queue_family_indices: *const u32,
}
const _: () = assert!(size_of::<VkBufferCreateInfo>() == 56);

#[repr(C)]
struct VkMemoryRequirements {
    size: u64,
    alignment: u64,
    memory_type_bits: u32,
}
const _: () = assert!(size_of::<VkMemoryRequirements>() == 24);

#[repr(C)]
struct VkMemoryAllocateInfo {
    s_type: u32,
    p_next: *const c_void,
    allocation_size: u64,
    memory_type_index: u32,
}
const _: () = assert!(size_of::<VkMemoryAllocateInfo>() == 32);

#[repr(C)]
#[derive(Clone, Copy)]
struct VkMemoryType {
    property_flags: u32,
    heap_index: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VkMemoryHeap {
    size: u64,
    flags: u32,
}

#[repr(C)]
struct VkPhysicalDeviceMemoryProperties {
    memory_type_count: u32,
    memory_types: [VkMemoryType; 32],
    memory_heap_count: u32,
    memory_heaps: [VkMemoryHeap; 16],
}
const _: () = assert!(size_of::<VkPhysicalDeviceMemoryProperties>() == 520);

#[repr(C)]
struct VkCommandPoolCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: u32,
    queue_family_index: u32,
}
const _: () = assert!(size_of::<VkCommandPoolCreateInfo>() == 24);

#[repr(C)]
struct VkCommandBufferAllocateInfo {
    s_type: u32,
    p_next: *const c_void,
    command_pool: u64,
    level: u32,
    command_buffer_count: u32,
}
const _: () = assert!(size_of::<VkCommandBufferAllocateInfo>() == 32);

#[repr(C)]
struct VkCommandBufferBeginInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: u32,
    p_inheritance_info: *const c_void,
}
const _: () = assert!(size_of::<VkCommandBufferBeginInfo>() == 32);

#[repr(C)]
struct VkBufferCopy {
    src_offset: u64,
    dst_offset: u64,
    size: u64,
}
const _: () = assert!(size_of::<VkBufferCopy>() == 24);

#[repr(C)]
struct VkSubmitInfo {
    s_type: u32,
    p_next: *const c_void,
    wait_semaphore_count: u32,
    p_wait_semaphores: *const c_void,
    p_wait_dst_stage_mask: *const c_void,
    command_buffer_count: u32,
    p_command_buffers: *const *mut c_void,
    signal_semaphore_count: u32,
    p_signal_semaphores: *const c_void,
}
const _: () = assert!(size_of::<VkSubmitInfo>() == 72);

type VkResult = i32;
type FnGetDeviceProcAddr = unsafe extern "system" fn(*mut c_void, *const c_char) -> Option<PfnVoid>;
type FnGetPhysicalDeviceMemoryProperties =
    unsafe extern "system" fn(*mut c_void, *mut VkPhysicalDeviceMemoryProperties);
type FnCreateBuffer = unsafe extern "system" fn(
    *mut c_void,
    *const VkBufferCreateInfo,
    *const c_void,
    *mut u64,
) -> VkResult;
type FnDestroyBuffer = unsafe extern "system" fn(*mut c_void, u64, *const c_void);
type FnGetBufferMemoryRequirements =
    unsafe extern "system" fn(*mut c_void, u64, *mut VkMemoryRequirements);
type FnAllocateMemory = unsafe extern "system" fn(
    *mut c_void,
    *const VkMemoryAllocateInfo,
    *const c_void,
    *mut u64,
) -> VkResult;
type FnFreeMemory = unsafe extern "system" fn(*mut c_void, u64, *const c_void);
type FnBindBufferMemory = unsafe extern "system" fn(*mut c_void, u64, u64, u64) -> VkResult;
type FnMapMemory =
    unsafe extern "system" fn(*mut c_void, u64, u64, u64, u32, *mut *mut c_void) -> VkResult;
type FnUnmapMemory = unsafe extern "system" fn(*mut c_void, u64);
type FnCreateCommandPool = unsafe extern "system" fn(
    *mut c_void,
    *const VkCommandPoolCreateInfo,
    *const c_void,
    *mut u64,
) -> VkResult;
type FnDestroyCommandPool = unsafe extern "system" fn(*mut c_void, u64, *const c_void);
type FnAllocateCommandBuffers = unsafe extern "system" fn(
    *mut c_void,
    *const VkCommandBufferAllocateInfo,
    *mut *mut c_void,
) -> VkResult;
type FnBeginCommandBuffer =
    unsafe extern "system" fn(*mut c_void, *const VkCommandBufferBeginInfo) -> VkResult;
type FnCmdCopyBuffer = unsafe extern "system" fn(*mut c_void, u64, u64, u32, *const VkBufferCopy);
type FnEndCommandBuffer = unsafe extern "system" fn(*mut c_void) -> VkResult;
type FnQueueSubmit =
    unsafe extern "system" fn(*mut c_void, u32, *const VkSubmitInfo, u64) -> VkResult;
type FnQueueWaitIdle = unsafe extern "system" fn(*mut c_void) -> VkResult;

/// readback 侧 vk 对象清理表(Drop 逆序:cmdpool → buffer → memory;0 = 未创建跳过)。
struct VkCopyCleanup {
    device: *mut c_void,
    destroy_buffer: FnDestroyBuffer,
    free_memory: FnFreeMemory,
    destroy_cmdpool: FnDestroyCommandPool,
    buffer: u64,
    memory: u64,
    cmdpool: u64,
}

impl Drop for VkCopyCleanup {
    fn drop(&mut self) {
        // SAFETY: 三句柄各自由本结构对应 create 成功产出或仍为 0(跳过);destroy 函数
        // 指针与同 device 配对;单点 Drop 仅一次,无泄漏/双释放。
        unsafe {
            if self.cmdpool != 0 {
                (self.destroy_cmdpool)(self.device, self.cmdpool, std::ptr::null());
            }
            if self.buffer != 0 {
                (self.destroy_buffer)(self.device, self.buffer, std::ptr::null());
            }
            if self.memory != 0 {
                (self.free_memory)(self.device, self.memory, std::ptr::null());
            }
        }
    }
}

/// 同 device 上把 TiRT 导出 VkBuffer copy 到 host-visible buffer 并读回 f32 列。
/// 调用前置:TiRT 侧已 `ti_flush`+`ti_wait` 全排空(§4.E2 同步纪律)。
fn copy_exported_to_host(
    gipa: FnGetInstanceProcAddr,
    dev: &vk::TirtVulkanDevice,
    src_buffer: u64,
    bytes: u64,
    particle_count: u32,
) -> Result<Vec<f32>, TirtError> {
    // 符号解析(instance 级 gdpa/memprops + device 级 readback 子集)。
    // SAFETY: instance/device 为有效句柄;符号名 NUL 结尾字面量;cast_fn 逐符号
    // null 校验 + 指针宽度断言(镜像 vk::cast_fn);缺失经 `?` 映射确定性 Err。
    let (gdpa, vk_get_mem) = unsafe {
        (
            cast_fn::<FnGetDeviceProcAddr>(gipa(dev.instance, c"vkGetDeviceProcAddr".as_ptr())),
            cast_fn::<FnGetPhysicalDeviceMemoryProperties>(gipa(
                dev.instance,
                c"vkGetPhysicalDeviceMemoryProperties".as_ptr(),
            )),
        )
    };
    let gdpa = gdpa.ok_or_else(|| TirtError::DeviceUnavailable("缺 vkGetDeviceProcAddr".into()))?;
    let vk_get_mem = vk_get_mem.ok_or_else(|| {
        TirtError::DeviceUnavailable("缺 vkGetPhysicalDeviceMemoryProperties".into())
    })?;
    // SAFETY: device 有效;gdpa 为有效 vkGetDeviceProcAddr;逐符号 null 校验,缺失即 Err。
    let (
        create_buffer,
        destroy_buffer,
        buf_mem_req,
        alloc_mem,
        free_mem,
        bind_buf,
        map_mem,
        unmap_mem,
        create_cmdpool,
        destroy_cmdpool,
        alloc_cmd,
        begin_cmd,
        cmd_copy_buf,
        end_cmd,
        queue_submit,
        queue_wait,
    ) = unsafe {
        (
            cast_fn::<FnCreateBuffer>(gdpa(dev.device, c"vkCreateBuffer".as_ptr())),
            cast_fn::<FnDestroyBuffer>(gdpa(dev.device, c"vkDestroyBuffer".as_ptr())),
            cast_fn::<FnGetBufferMemoryRequirements>(gdpa(
                dev.device,
                c"vkGetBufferMemoryRequirements".as_ptr(),
            )),
            cast_fn::<FnAllocateMemory>(gdpa(dev.device, c"vkAllocateMemory".as_ptr())),
            cast_fn::<FnFreeMemory>(gdpa(dev.device, c"vkFreeMemory".as_ptr())),
            cast_fn::<FnBindBufferMemory>(gdpa(dev.device, c"vkBindBufferMemory".as_ptr())),
            cast_fn::<FnMapMemory>(gdpa(dev.device, c"vkMapMemory".as_ptr())),
            cast_fn::<FnUnmapMemory>(gdpa(dev.device, c"vkUnmapMemory".as_ptr())),
            cast_fn::<FnCreateCommandPool>(gdpa(dev.device, c"vkCreateCommandPool".as_ptr())),
            cast_fn::<FnDestroyCommandPool>(gdpa(dev.device, c"vkDestroyCommandPool".as_ptr())),
            cast_fn::<FnAllocateCommandBuffers>(gdpa(
                dev.device,
                c"vkAllocateCommandBuffers".as_ptr(),
            )),
            cast_fn::<FnBeginCommandBuffer>(gdpa(dev.device, c"vkBeginCommandBuffer".as_ptr())),
            cast_fn::<FnCmdCopyBuffer>(gdpa(dev.device, c"vkCmdCopyBuffer".as_ptr())),
            cast_fn::<FnEndCommandBuffer>(gdpa(dev.device, c"vkEndCommandBuffer".as_ptr())),
            cast_fn::<FnQueueSubmit>(gdpa(dev.device, c"vkQueueSubmit".as_ptr())),
            cast_fn::<FnQueueWaitIdle>(gdpa(dev.device, c"vkQueueWaitIdle".as_ptr())),
        )
    };
    macro_rules! need {
        ($sym:expr, $name:literal) => {
            $sym.ok_or_else(|| TirtError::DeviceUnavailable(concat!("缺 ", $name).to_owned()))?
        };
    }
    let create_buffer = need!(create_buffer, "vkCreateBuffer");
    let destroy_buffer = need!(destroy_buffer, "vkDestroyBuffer");
    let buf_mem_req = need!(buf_mem_req, "vkGetBufferMemoryRequirements");
    let alloc_mem = need!(alloc_mem, "vkAllocateMemory");
    let free_mem = need!(free_mem, "vkFreeMemory");
    let bind_buf = need!(bind_buf, "vkBindBufferMemory");
    let map_mem = need!(map_mem, "vkMapMemory");
    let unmap_mem = need!(unmap_mem, "vkUnmapMemory");
    let create_cmdpool = need!(create_cmdpool, "vkCreateCommandPool");
    let destroy_cmdpool = need!(destroy_cmdpool, "vkDestroyCommandPool");
    let alloc_cmd = need!(alloc_cmd, "vkAllocateCommandBuffers");
    let begin_cmd = need!(begin_cmd, "vkBeginCommandBuffer");
    let cmd_copy_buf = need!(cmd_copy_buf, "vkCmdCopyBuffer");
    let end_cmd = need!(end_cmd, "vkEndCommandBuffer");
    let queue_submit = need!(queue_submit, "vkQueueSubmit");
    let queue_wait = need!(queue_wait, "vkQueueWaitIdle");

    let mut cleanup = VkCopyCleanup {
        device: dev.device,
        destroy_buffer,
        free_memory: free_mem,
        destroy_cmdpool,
        buffer: 0,
        memory: 0,
        cmdpool: 0,
    };

    let mut memprops = VkPhysicalDeviceMemoryProperties {
        memory_type_count: 0,
        memory_types: [VkMemoryType {
            property_flags: 0,
            heap_index: 0,
        }; 32],
        memory_heap_count: 0,
        memory_heaps: [VkMemoryHeap { size: 0, flags: 0 }; 16],
    };
    // SAFETY: memprops 为栈上有效结构(布局锚定);pd 有效。
    unsafe { vk_get_mem(dev.physical_device, &mut memprops) };

    // host-visible readback 目的 buffer。
    let bci = VkBufferCreateInfo {
        s_type: VK_ST_BUFFER_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        size: bytes,
        usage: VK_BUFFER_USAGE_TRANSFER_DST,
        sharing_mode: VK_SHARING_MODE_EXCLUSIVE,
        queue_family_index_count: 0,
        p_queue_family_indices: std::ptr::null(),
    };
    let mut dst_buf: u64 = 0;
    // SAFETY: device 有效;bci 为逐字节对齐结构;dst_buf 出参有效可写。
    let r = unsafe { create_buffer(dev.device, &bci, std::ptr::null(), &mut dst_buf) };
    if r != VK_SUCCESS {
        return Err(TirtError::DeviceUnavailable(format!(
            "vkCreateBuffer(readback {bytes}B) 失败: {r}"
        )));
    }
    cleanup.buffer = dst_buf;

    let mut req = VkMemoryRequirements {
        size: 0,
        alignment: 0,
        memory_type_bits: 0,
    };
    // SAFETY: req 出参有效可写;dst_buf 刚创建。
    unsafe { buf_mem_req(dev.device, dst_buf, &mut req) };
    let need_flags = VK_MEM_HOST_VISIBLE | VK_MEM_HOST_COHERENT;
    let mut type_index = u32::MAX;
    for (i, mt) in memprops
        .memory_types
        .iter()
        .enumerate()
        .take(memprops.memory_type_count as usize)
    {
        if req.memory_type_bits & (1u32 << i) != 0 && mt.property_flags & need_flags == need_flags {
            type_index = i as u32;
            break;
        }
    }
    if type_index == u32::MAX {
        return Err(TirtError::DeviceUnavailable(
            "无 host-visible+coherent 内存类型(readback 不可能)".to_owned(),
        ));
    }
    let mai = VkMemoryAllocateInfo {
        s_type: VK_ST_MEMORY_ALLOCATE_INFO,
        p_next: std::ptr::null(),
        allocation_size: req.size,
        memory_type_index: type_index,
    };
    let mut dst_mem: u64 = 0;
    // SAFETY: device 有效;mai 为逐字节对齐结构(type_index 已按 memoryTypeBits 过滤);
    // dst_mem 出参有效可写。
    let r = unsafe { alloc_mem(dev.device, &mai, std::ptr::null(), &mut dst_mem) };
    if r != VK_SUCCESS {
        return Err(TirtError::DeviceUnavailable(format!(
            "vkAllocateMemory(readback {}B) 失败: {r}",
            req.size
        )));
    }
    cleanup.memory = dst_mem;
    // SAFETY: dst_buf/dst_mem 同 device 刚配对创建,offset 0 在分配内。
    let r = unsafe { bind_buf(dev.device, dst_buf, dst_mem, 0) };
    if r != VK_SUCCESS {
        return Err(TirtError::DeviceUnavailable(format!(
            "vkBindBufferMemory 失败: {r}"
        )));
    }

    // 命令池 + 单次命令缓冲(compute 家族)。
    let cpci = VkCommandPoolCreateInfo {
        s_type: VK_ST_COMMAND_POOL_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: 0,
        queue_family_index: dev.compute_queue_family,
    };
    let mut cmdpool: u64 = 0;
    // SAFETY: device 有效;cpci 为逐字节对齐结构;cmdpool 出参有效可写。
    let r = unsafe { create_cmdpool(dev.device, &cpci, std::ptr::null(), &mut cmdpool) };
    if r != VK_SUCCESS {
        return Err(TirtError::DeviceUnavailable(format!(
            "vkCreateCommandPool 失败: {r}"
        )));
    }
    cleanup.cmdpool = cmdpool;
    let cbai = VkCommandBufferAllocateInfo {
        s_type: VK_ST_COMMAND_BUFFER_ALLOCATE_INFO,
        p_next: std::ptr::null(),
        command_pool: cmdpool,
        level: VK_CMD_BUFFER_LEVEL_PRIMARY,
        command_buffer_count: 1,
    };
    let mut cmd: *mut c_void = std::ptr::null_mut();
    // SAFETY: cmdpool 刚创建;cmd 出参有效可写;pool 与 cmd 同生共死(cleanup 销毁池)。
    let r = unsafe { alloc_cmd(dev.device, &cbai, &mut cmd) };
    if r != VK_SUCCESS {
        return Err(TirtError::DeviceUnavailable(format!(
            "vkAllocateCommandBuffers 失败: {r}"
        )));
    }

    let cbi = VkCommandBufferBeginInfo {
        s_type: VK_ST_COMMAND_BUFFER_BEGIN_INFO,
        p_next: std::ptr::null(),
        flags: VK_CMD_BUFFER_USAGE_ONE_TIME_SUBMIT,
        p_inheritance_info: std::ptr::null(),
    };
    // SAFETY: cmd 为有效已分配命令缓冲(primary)。
    let r = unsafe { begin_cmd(cmd, &cbi) };
    if r != VK_SUCCESS {
        return Err(TirtError::DeviceUnavailable(format!(
            "vkBeginCommandBuffer 失败: {r}"
        )));
    }
    let region = VkBufferCopy {
        src_offset: 0,
        dst_offset: 0,
        size: bytes,
    };
    // SAFETY: src_buffer 为 TiRT 导出 VkBuffer(ti_export_vulkan_memory 产出,同 device,
    // ti_wait 后无在途写,usage 含 TRANSFER_SRC 已核);dst_buf 容量 ≥ bytes(TRANSFER_DST);
    // region 栈上有效;cmd 录制中。
    unsafe { cmd_copy_buf(cmd, src_buffer, dst_buf, 1, &region) };
    // SAFETY: cmd 录制中;结束录制。
    let r = unsafe { end_cmd(cmd) };
    if r != VK_SUCCESS {
        return Err(TirtError::DeviceUnavailable(format!(
            "vkEndCommandBuffer 失败: {r}"
        )));
    }

    let si = VkSubmitInfo {
        s_type: VK_ST_SUBMIT_INFO,
        p_next: std::ptr::null(),
        wait_semaphore_count: 0,
        p_wait_semaphores: std::ptr::null(),
        p_wait_dst_stage_mask: std::ptr::null(),
        command_buffer_count: 1,
        p_command_buffers: &cmd,
        signal_semaphore_count: 0,
        p_signal_semaphores: std::ptr::null(),
    };
    // SAFETY: si 引用 cmd 栈上有效;fence=null,完成等待走 vkQueueWaitIdle(单 queue
    // 同步纪律,同 U26);compute_queue 有效。
    let r = unsafe { queue_submit(dev.compute_queue, 1, &si, 0) };
    if r != VK_SUCCESS {
        return Err(TirtError::DeviceUnavailable(format!(
            "vkQueueSubmit(copy) 失败: {r}"
        )));
    }
    // SAFETY: queue 有效;阻塞至 copy 完成(同步纪律:copy 完成后再 map/释放,§4.E2)。
    let r = unsafe { queue_wait(dev.compute_queue) };
    if r != VK_SUCCESS {
        return Err(TirtError::DeviceUnavailable(format!(
            "vkQueueWaitIdle 失败: {r}"
        )));
    }

    // map 读回(host-visible+coherent 免 flush)。
    let mut mapped: *mut c_void = std::ptr::null_mut();
    // SAFETY: dst_mem host-visible;范围 [0,bytes) 在分配内(req.size ≥ bytes);
    // mapped 出参有效可写。
    let r = unsafe { map_mem(dev.device, dst_mem, 0, bytes, 0, &mut mapped) };
    if r != VK_SUCCESS || mapped.is_null() {
        return Err(TirtError::ReadbackMismatch(format!(
            "vkMapMemory 失败: r={r} ptr_null={}",
            mapped.is_null()
        )));
    }
    let values: Vec<f32> = {
        // SAFETY: mapped 指向 ≥ bytes 字节 host-visible 内存(分配对齐 ≥ 4,f32 对齐满足);
        // particle_count = bytes/4;unmap 前 to_vec 拷出,无悬垂引用。
        unsafe { std::slice::from_raw_parts(mapped.cast::<f32>(), particle_count as usize) }
            .to_vec()
    };
    // SAFETY: 与 vkMapMemory 配对;此后不再触碰 mapped。
    unsafe { unmap_mem(dev.device, dst_mem) };
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// env 操作串行锁(cargo test 并行下,RURIX_TAICHI_C_API_DLL 读写必须互斥)。
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// 还原环境变量到测试前状态。
    fn restore_env(saved: Option<String>) {
        // SAFETY: 调用方持有 ENV_LOCK,env 读写串行化(edition 2024 env 变更为 unsafe)。
        unsafe {
            match saved {
                Some(v) => std::env::set_var(TIRT_DLL_ENV, v),
                None => std::env::remove_var(TIRT_DLL_ENV),
            }
        }
    }

    //@ spec: RFC-0017 §4.E(P-01 fail-closed:未设环境变量 → 确定性 DllNotFound)
    #[test]
    fn load_without_env_fails_closed() {
        let _g = ENV_LOCK.lock().expect("env 锁中毒");
        let saved = std::env::var(TIRT_DLL_ENV).ok();
        // SAFETY: ENV_LOCK 串行化,无并发 env 读写。
        unsafe { std::env::remove_var(TIRT_DLL_ENV) };
        let r = TirtApi::load();
        restore_env(saved);
        assert!(
            matches!(r, Err(TirtError::DllNotFound(_))),
            "未设环境变量须确定性 DllNotFound: {r:?}"
        );
    }

    //@ spec: RFC-0017 §4.E(P-01 fail-closed:bogus 路径 → 确定性 DllNotFound)
    #[test]
    fn load_bogus_path_fails_closed() {
        let _g = ENV_LOCK.lock().expect("env 锁中毒");
        let saved = std::env::var(TIRT_DLL_ENV).ok();
        // SAFETY: ENV_LOCK 串行化,无并发 env 读写。
        unsafe { std::env::set_var(TIRT_DLL_ENV, r"H:\rurix\nonexistent\taichi_c_api.dll") };
        let r = TirtApi::load();
        restore_env(saved);
        assert!(
            matches!(r, Err(TirtError::DllNotFound(_))),
            "bogus 路径须确定性 DllNotFound: {r:?}"
        );
    }

    //@ spec: U43 布局锚定(编译期 const assert 的运行期镜像,host 无 GPU/DLL 恒绿)
    #[test]
    fn ffi_layout_anchors() {
        assert_eq!(size_of::<TiMemoryAllocateInfo>(), 24);
        assert_eq!(size_of::<TiNdShape>(), 68);
        assert_eq!(size_of::<TiNdArray>(), 152);
        assert_eq!(size_of::<TiArgument>(), 160);
        assert_eq!(size_of::<TiVulkanRuntimeInteropInfo>(), 72);
        assert_eq!(size_of::<TiVulkanMemoryInteropInfo>(), 40);
        assert_eq!(size_of::<VkBufferCreateInfo>(), 56);
        assert_eq!(size_of::<VkMemoryAllocateInfo>(), 32);
        assert_eq!(size_of::<VkCommandPoolCreateInfo>(), 24);
        assert_eq!(size_of::<VkCommandBufferAllocateInfo>(), 32);
        assert_eq!(size_of::<VkCommandBufferBeginInfo>(), 32);
        assert_eq!(size_of::<VkSubmitInfo>(), 72);
        assert_eq!(size_of::<VkBufferCopy>(), 24);
        assert_eq!(size_of::<VkPhysicalDeviceMemoryProperties>(), 520);
    }
}
