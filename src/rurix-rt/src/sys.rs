//! CUDA Driver API 原始 FFI 与运行时动态加载(08 §1 / D-230;14 §2 PTX-only)。
//!
//! **动态加载**(非链接期绑定):`nvcuda.dll` 经 `LoadLibraryA`/`GetProcAddress`
//! 运行时装载——不依赖 CUDA Toolkit 的 `nvcuda.lib` 导入库(开发机无 Toolkit 时
//! 仍可真跑,沿用 M0 `bench/cuda_driver.py` ctypes 先例)。符号缺失/驱动不可用
//! → [`Cuda::load`] 返回 `None`(上层映射结构化错误,08 §2.5)。
//!
//! **unsafe 边界**(AGENTS 硬规则 9,注册见 `unsafe-audit/rurix-rt.md`):本模块是
//! Rurix 全仓首个 unsafe 边界。全部 `unsafe` 集中于此——每个 `Cuda` 方法以单个
//! `unsafe` 块前向到已加载的 Driver API 函数指针;调用约定/签名与 CUDA Driver
//! API ABI(D-113:`extern "system"` / `#[repr(C)]` / 原始指针,Windows x64 唯一
//! ABI)对齐,句柄/指针有效性由上层所有权类型([`crate::Context`] 等)维持。

use core::ffi::{c_char, c_void};
use std::sync::OnceLock;

/// `CUresult`(Driver API 返回码;0 = `CUDA_SUCCESS`)。
pub type CuResult = i32;
/// `CUdevice`(设备序号句柄,Copy)。
pub type CuDevice = i32;
/// 不透明 Driver API 句柄(`CUcontext`/`CUmodule`/`CUfunction`/`CUstream`)。
pub type CuPtr = *mut c_void;
/// `CUdeviceptr`(64 位设备地址,Windows x64)。
pub type CuDevicePtr = u64;

pub const CUDA_SUCCESS: CuResult = 0;
/// `CUDA_ERROR_DEINITIALIZED` / 上下文销毁后操作(709)。
pub const CUDA_ERROR_CONTEXT_IS_DESTROYED: CuResult = 709;
/// `CUDA_ERROR_ASSERT`(710):device 侧断言触发,context 不可恢复(08 §2.5)。
pub const CUDA_ERROR_ASSERT: CuResult = 710;
/// `CUDA_ERROR_UNSUPPORTED_PTX_VERSION`(222):PTX `.version` 超驱动 JIT 能力。
pub const CUDA_ERROR_UNSUPPORTED_PTX_VERSION: CuResult = 222;

/// JIT 装载选项键(`CUjit_option`,08 §2.4 日志缓冲常开)。
pub const CU_JIT_INFO_LOG_BUFFER: i32 = 3;
pub const CU_JIT_INFO_LOG_BUFFER_SIZE_BYTES: i32 = 4;
pub const CU_JIT_ERROR_LOG_BUFFER: i32 = 5;
pub const CU_JIT_ERROR_LOG_BUFFER_SIZE_BYTES: i32 = 6;

/// `CU_EVENT_DEFAULT`(0):默认 event 标志(计时 + 阻塞同步,M8.3 UC-02 跨 stream 同步)。
pub const CU_EVENT_DEFAULT: u32 = 0;
/// `cuStreamWaitEvent` 标志(0 = 默认;M8.3 流序依赖,RXS-0131)。
pub const CU_STREAM_WAIT_DEFAULT: u32 = 0;

/// `CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MAJOR`(75)/ `..._MINOR`(76):查 device sm 架构键
/// 供 fatbin 装载协商(G1.5,RXS-0151;`select_load_variant`)。
pub const CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MAJOR: i32 = 75;
pub const CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MINOR: i32 = 76;

// -- Windows 动态加载(kernel32;std 默认链接,无需 toolkit) -----------------

unsafe extern "system" {
    fn LoadLibraryA(name: *const c_char) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const c_char) -> *mut c_void;
}

// -- Driver API 函数指针类型(D-113:extern "system",x64 ABI) ----------------

type FnInit = unsafe extern "system" fn(u32) -> CuResult;
type FnDeviceGet = unsafe extern "system" fn(*mut CuDevice, i32) -> CuResult;
type FnDeviceGetCount = unsafe extern "system" fn(*mut i32) -> CuResult;
type FnCtxCreate = unsafe extern "system" fn(*mut CuPtr, u32, CuDevice) -> CuResult;
type FnCtxDestroy = unsafe extern "system" fn(CuPtr) -> CuResult;
type FnCtxSync = unsafe extern "system" fn() -> CuResult;
type FnCtxSetCurrent = unsafe extern "system" fn(CuPtr) -> CuResult;
type FnPrimaryCtxRetain = unsafe extern "system" fn(*mut CuPtr, CuDevice) -> CuResult;
type FnPrimaryCtxRelease = unsafe extern "system" fn(CuDevice) -> CuResult;
type FnStreamCreate = unsafe extern "system" fn(*mut CuPtr, u32) -> CuResult;
type FnStreamDestroy = unsafe extern "system" fn(CuPtr) -> CuResult;
type FnStreamSync = unsafe extern "system" fn(CuPtr) -> CuResult;
type FnMemAlloc = unsafe extern "system" fn(*mut CuDevicePtr, usize) -> CuResult;
type FnMemFree = unsafe extern "system" fn(CuDevicePtr) -> CuResult;
type FnMemAllocHost = unsafe extern "system" fn(*mut *mut c_void, usize) -> CuResult;
type FnMemFreeHost = unsafe extern "system" fn(*mut c_void) -> CuResult;
type FnMemcpyHtoD = unsafe extern "system" fn(CuDevicePtr, *const c_void, usize) -> CuResult;
type FnMemcpyDtoH = unsafe extern "system" fn(*mut c_void, CuDevicePtr, usize) -> CuResult;
type FnModuleLoadDataEx = unsafe extern "system" fn(
    *mut CuPtr,
    *const c_void,
    u32,
    *mut i32,
    *mut *mut c_void,
) -> CuResult;
type FnModuleUnload = unsafe extern "system" fn(CuPtr) -> CuResult;
type FnModuleGetFunction = unsafe extern "system" fn(*mut CuPtr, CuPtr, *const c_char) -> CuResult;
type FnLaunchKernel = unsafe extern "system" fn(
    CuPtr,
    u32,
    u32,
    u32,
    u32,
    u32,
    u32,
    u32,
    CuPtr,
    *mut *mut c_void,
    *mut *mut c_void,
) -> CuResult;
type FnGetErrorName = unsafe extern "system" fn(CuResult, *mut *const c_char) -> CuResult;
type FnGetErrorString = unsafe extern "system" fn(CuResult, *mut *const c_char) -> CuResult;
// -- M8.3 UC-02:event 跨 stream 同步 + 异步搬运(三 stream 重叠流水线) ----------
type FnEventCreate = unsafe extern "system" fn(*mut CuPtr, u32) -> CuResult;
type FnEventRecord = unsafe extern "system" fn(CuPtr, CuPtr) -> CuResult;
type FnEventDestroy = unsafe extern "system" fn(CuPtr) -> CuResult;
type FnEventSync = unsafe extern "system" fn(CuPtr) -> CuResult;
type FnStreamWaitEvent = unsafe extern "system" fn(CuPtr, CuPtr, u32) -> CuResult;
type FnMemcpyHtoDAsync =
    unsafe extern "system" fn(CuDevicePtr, *const c_void, usize, CuPtr) -> CuResult;
type FnMemcpyDtoHAsync =
    unsafe extern "system" fn(*mut c_void, CuDevicePtr, usize, CuPtr) -> CuResult;
// -- G1.2 流序分配:stream-ordered allocator(`cuMemAllocAsync` + `CUmemoryPool`,D-232;RXS-0144) --
// 非 `_v2`(CUDA 11.2+ 引入);作为 **Option 字段非致命解析**(老驱动缺失 → 上层报运行期
// 不可用,核心 CUDA 不受影响,对齐 G1.1 external-resource 先例)。分配/释放都携 stream 句柄
// (流序),分配自该 stream 当前 memory pool(默认 = 设备默认池)。
type FnMemAllocAsync = unsafe extern "system" fn(*mut CuDevicePtr, usize, CuPtr) -> CuResult;
type FnMemFreeAsync = unsafe extern "system" fn(CuDevicePtr, CuPtr) -> CuResult;
// -- G1.5 生产分发 fatbin:按架构预编 cubin 装载 + compute capability 查询(RXS-0150/0151,D-207) --
// `cuModuleLoadData`(cubin 二进制装载,首启免 JIT)+ `cuDeviceGetAttribute`(sm 查询)。作为
// **Option 字段非致命解析**(缺失 → 装载协商降级保守 PTX fallback,核心 PTX 路径不受影响,U22)。
type FnModuleLoadData = unsafe extern "system" fn(*mut CuPtr, *const c_void) -> CuResult;
type FnDeviceGetAttribute = unsafe extern "system" fn(*mut i32, i32, CuDevice) -> CuResult;
// -- G1.1 CUDA–D3D12 互操作:external memory/semaphore import(RXS-0140/0143;RFC-0001 §4.2.3) --
// `CUexternalMemory`/`CUexternalSemaphore` 为不透明句柄(= CuPtr)。下列符号无 `_v2`
// 后缀(RFC-0001 §4.2.3);作为 **Option 字段非致命解析**(缺失不禁用核心 CUDA)。
type FnDeviceGetLuid = unsafe extern "system" fn(*mut c_char, *mut u32, CuDevice) -> CuResult;
type FnImportExternalMemory =
    unsafe extern "system" fn(*mut CuPtr, *const CudaExternalMemoryHandleDesc) -> CuResult;
type FnExternalMemoryGetMappedBuffer = unsafe extern "system" fn(
    *mut CuDevicePtr,
    CuPtr,
    *const CudaExternalMemoryBufferDesc,
) -> CuResult;
type FnDestroyExternalMemory = unsafe extern "system" fn(CuPtr) -> CuResult;
type FnImportExternalSemaphore =
    unsafe extern "system" fn(*mut CuPtr, *const CudaExternalSemaphoreHandleDesc) -> CuResult;
type FnSignalExternalSemaphoresAsync = unsafe extern "system" fn(
    *const CuPtr,
    *const CudaExternalSemaphoreParams,
    u32,
    CuPtr,
) -> CuResult;
type FnWaitExternalSemaphoresAsync = unsafe extern "system" fn(
    *const CuPtr,
    *const CudaExternalSemaphoreParams,
    u32,
    CuPtr,
) -> CuResult;
type FnDestroyExternalSemaphore = unsafe extern "system" fn(CuPtr) -> CuResult;

/// `CUexternalMemoryHandleType`:D3D12 resource = 5(RFC-0001 §4.2.2:采纳 RESOURCE,否决 HEAP=4)。
pub const CU_EXTERNAL_MEMORY_HANDLE_TYPE_D3D12_RESOURCE: u32 = 5;
/// `CUDA_EXTERNAL_MEMORY_DEDICATED`:committed resource 整块专用分配,必须置(RFC-0001 §4.2.2)。
pub const CUDA_EXTERNAL_MEMORY_DEDICATED: u32 = 0x1;
/// `CUexternalSemaphoreHandleType`:D3D12 fence = 4。
pub const CU_EXTERNAL_SEMAPHORE_HANDLE_TYPE_D3D12_FENCE: u32 = 4;

/// `CUDA_EXTERNAL_MEMORY_HANDLE_DESC` 的 win32 句柄成员(union 最大成员,16 字节)。
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CudaWin32Handle {
    /// NT HANDLE(`CreateSharedHandle` 产出;CUDA import 不接管所有权,RFC-0001 §4.2.2)。
    pub handle: *mut c_void,
    /// 命名共享(NULL = 用 handle)。
    pub name: *const c_void,
}

/// `CUDA_EXTERNAL_MEMORY_HANDLE_DESC`(头文件 v1 布局,Windows x64 = 104 字节)。
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CudaExternalMemoryHandleDesc {
    /// `CUexternalMemoryHandleType`(D3D12_RESOURCE=5)。
    pub type_: u32,
    /// union 成员(仅用 win32;repr(C) 自动在 type_ 后补 4 字节对齐到 8)。
    pub win32: CudaWin32Handle,
    /// 分配字节数(`GetResourceAllocationInfo.SizeInBytes`)。
    pub size: u64,
    /// `CUDA_EXTERNAL_MEMORY_DEDICATED`。
    pub flags: u32,
    pub reserved: [u32; 16],
}
const _: () = assert!(size_of::<CudaExternalMemoryHandleDesc>() == 104);

/// `CUDA_EXTERNAL_MEMORY_BUFFER_DESC`(Windows x64 = 88 字节)。
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CudaExternalMemoryBufferDesc {
    pub offset: u64,
    pub size: u64,
    pub flags: u32,
    pub reserved: [u32; 16],
}
const _: () = assert!(size_of::<CudaExternalMemoryBufferDesc>() == 88);

/// `CUDA_EXTERNAL_SEMAPHORE_HANDLE_DESC`(Windows x64 = 96 字节)。
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CudaExternalSemaphoreHandleDesc {
    /// `CUexternalSemaphoreHandleType`(D3D12_FENCE=4)。
    pub type_: u32,
    pub win32: CudaWin32Handle,
    pub flags: u32,
    pub reserved: [u32; 16],
}
const _: () = assert!(size_of::<CudaExternalSemaphoreHandleDesc>() == 96);

/// `CUDA_EXTERNAL_SEMAPHORE_SIGNAL_PARAMS` / `..._WAIT_PARAMS`(同布局,Windows x64 = 144 字节)。
/// 仅 `fence_value` 用于 D3D12 fence handoff(RFC-0001 §4.3:2n / 2n+1 / 2n+2)。
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CudaExternalSemaphoreParams {
    /// `params.fence.value`(D3D12 fence 目标值)。
    pub fence_value: u64,
    /// `params.nvSciSync`(union,未用)。
    pub nv_sci_sync: u64,
    /// `params.keyedMutex.key`(未用)。
    pub keyed_mutex_key: u64,
    /// `params.reserved[12]`。
    pub params_reserved: [u32; 12],
    pub flags: u32,
    pub reserved: [u32; 16],
}
const _: () = assert!(size_of::<CudaExternalSemaphoreParams>() == 144);

/// 已加载的 Driver API 入口集(进程内一次加载,函数指针 Send + Sync)。
pub struct Cuda {
    cu_init: FnInit,
    cu_device_get: FnDeviceGet,
    cu_device_get_count: FnDeviceGetCount,
    cu_ctx_create: FnCtxCreate,
    cu_ctx_destroy: FnCtxDestroy,
    cu_ctx_sync: FnCtxSync,
    cu_ctx_set_current: FnCtxSetCurrent,
    cu_primary_ctx_retain: FnPrimaryCtxRetain,
    cu_primary_ctx_release: FnPrimaryCtxRelease,
    cu_stream_create: FnStreamCreate,
    cu_stream_destroy: FnStreamDestroy,
    cu_stream_sync: FnStreamSync,
    cu_mem_alloc: FnMemAlloc,
    cu_mem_free: FnMemFree,
    cu_mem_alloc_host: FnMemAllocHost,
    cu_mem_free_host: FnMemFreeHost,
    cu_memcpy_htod: FnMemcpyHtoD,
    cu_memcpy_dtoh: FnMemcpyDtoH,
    cu_module_load_data_ex: FnModuleLoadDataEx,
    cu_module_unload: FnModuleUnload,
    cu_module_get_function: FnModuleGetFunction,
    cu_launch_kernel: FnLaunchKernel,
    cu_get_error_name: FnGetErrorName,
    cu_get_error_string: FnGetErrorString,
    cu_event_create: FnEventCreate,
    cu_event_record: FnEventRecord,
    cu_event_destroy: FnEventDestroy,
    cu_event_sync: FnEventSync,
    cu_stream_wait_event: FnStreamWaitEvent,
    cu_memcpy_htod_async: FnMemcpyHtoDAsync,
    cu_memcpy_dtoh_async: FnMemcpyDtoHAsync,
    // G1.2 流序分配(Option:缺失不禁用核心 CUDA,D-232;RXS-0144)。
    cu_mem_alloc_async: Option<FnMemAllocAsync>,
    cu_mem_free_async: Option<FnMemFreeAsync>,
    // G1.1 external resource interop(Option:缺失不禁用核心 CUDA,RFC-0001 §4.2.3)。
    cu_device_get_luid: Option<FnDeviceGetLuid>,
    cu_import_external_memory: Option<FnImportExternalMemory>,
    cu_external_memory_get_mapped_buffer: Option<FnExternalMemoryGetMappedBuffer>,
    cu_destroy_external_memory: Option<FnDestroyExternalMemory>,
    cu_import_external_semaphore: Option<FnImportExternalSemaphore>,
    cu_signal_external_semaphores_async: Option<FnSignalExternalSemaphoresAsync>,
    cu_wait_external_semaphores_async: Option<FnWaitExternalSemaphoresAsync>,
    cu_destroy_external_semaphore: Option<FnDestroyExternalSemaphore>,
    // G1.5 生产分发 fatbin:cubin 装载 + sm 查询(Option:缺失 → 装载协商降级 PTX fallback,D-207;U22)。
    cu_module_load_data: Option<FnModuleLoadData>,
    cu_device_get_attribute: Option<FnDeviceGetAttribute>,
}

static CUDA: OnceLock<Option<Cuda>> = OnceLock::new();

/// 进程内单次加载 `nvcuda.dll` 的 Driver API 入口(失败 → `None`,上层映射
/// `DriverUnavailable`,08 §2.5)。
pub fn cuda() -> Option<&'static Cuda> {
    CUDA.get_or_init(Cuda::load).as_ref()
}

/// 从 `*mut c_void`(GetProcAddress 返回)转成类型化函数指针;null → None。
///
/// # Safety
/// `raw` 必须是 `nvcuda.dll` 中名为该符号、且其 ABI 与 `T`(`unsafe extern
/// "system" fn`)一致的导出函数地址(CUDA Driver API 稳定 ABI,D-113)。
unsafe fn cast_fn<T: Copy>(raw: *mut c_void) -> Option<T> {
    if raw.is_null() {
        return None;
    }
    debug_assert_eq!(size_of::<T>(), size_of::<*mut c_void>());
    // SAFETY: raw 非 null(已查);T 为指针宽度函数指针(debug_assert 校核);
    // 调用方保证 raw 为匹配 ABI 的导出符号地址(见 fn 文档)。
    Some(unsafe { std::mem::transmute_copy::<*mut c_void, T>(&raw) })
}

impl Cuda {
    /// 加载 `nvcuda.dll` 并解析全部所需 Driver API 符号(任一缺失 → None)。
    fn load() -> Option<Cuda> {
        // SAFETY: `LoadLibraryA`/`GetProcAddress` 为 Win32 稳定 ABI(kernel32);
        // 入参为 NUL 结尾 C 字符串字面量(`c"..."`);返回的模块/符号地址仅经
        // `cast_fn` 在 null 校验后转为匹配 ABI 的函数指针(D-113)。每个符号名与
        // 其上方类型别名签名按 CUDA Driver API(`_v2` ABI 版本)一一对应。
        unsafe {
            let lib = LoadLibraryA(c"nvcuda.dll".as_ptr());
            if lib.is_null() {
                return None;
            }
            let sym = |name: &core::ffi::CStr| GetProcAddress(lib, name.as_ptr());
            Some(Cuda {
                cu_init: cast_fn(sym(c"cuInit"))?,
                cu_device_get: cast_fn(sym(c"cuDeviceGet"))?,
                cu_device_get_count: cast_fn(sym(c"cuDeviceGetCount"))?,
                cu_ctx_create: cast_fn(sym(c"cuCtxCreate_v2"))?,
                cu_ctx_destroy: cast_fn(sym(c"cuCtxDestroy_v2"))?,
                cu_ctx_sync: cast_fn(sym(c"cuCtxSynchronize"))?,
                cu_ctx_set_current: cast_fn(sym(c"cuCtxSetCurrent"))?,
                cu_primary_ctx_retain: cast_fn(sym(c"cuDevicePrimaryCtxRetain"))?,
                cu_primary_ctx_release: cast_fn(sym(c"cuDevicePrimaryCtxRelease_v2"))?,
                cu_stream_create: cast_fn(sym(c"cuStreamCreate"))?,
                cu_stream_destroy: cast_fn(sym(c"cuStreamDestroy_v2"))?,
                cu_stream_sync: cast_fn(sym(c"cuStreamSynchronize"))?,
                cu_mem_alloc: cast_fn(sym(c"cuMemAlloc_v2"))?,
                cu_mem_free: cast_fn(sym(c"cuMemFree_v2"))?,
                cu_mem_alloc_host: cast_fn(sym(c"cuMemAllocHost_v2"))?,
                cu_mem_free_host: cast_fn(sym(c"cuMemFreeHost"))?,
                cu_memcpy_htod: cast_fn(sym(c"cuMemcpyHtoD_v2"))?,
                cu_memcpy_dtoh: cast_fn(sym(c"cuMemcpyDtoH_v2"))?,
                cu_module_load_data_ex: cast_fn(sym(c"cuModuleLoadDataEx"))?,
                cu_module_unload: cast_fn(sym(c"cuModuleUnload"))?,
                cu_module_get_function: cast_fn(sym(c"cuModuleGetFunction"))?,
                cu_launch_kernel: cast_fn(sym(c"cuLaunchKernel"))?,
                cu_get_error_name: cast_fn(sym(c"cuGetErrorName"))?,
                cu_get_error_string: cast_fn(sym(c"cuGetErrorString"))?,
                cu_event_create: cast_fn(sym(c"cuEventCreate"))?,
                cu_event_record: cast_fn(sym(c"cuEventRecord"))?,
                cu_event_destroy: cast_fn(sym(c"cuEventDestroy_v2"))?,
                cu_event_sync: cast_fn(sym(c"cuEventSynchronize"))?,
                cu_stream_wait_event: cast_fn(sym(c"cuStreamWaitEvent"))?,
                cu_memcpy_htod_async: cast_fn(sym(c"cuMemcpyHtoDAsync_v2"))?,
                cu_memcpy_dtoh_async: cast_fn(sym(c"cuMemcpyDtoHAsync_v2"))?,
                // G1.2 流序分配:非致命解析(无 `?`;老驱动缺失 → None → 上层报运行期不可用,
                // 核心 CUDA 不受影响,D-232 / RXS-0144)。
                cu_mem_alloc_async: cast_fn(sym(c"cuMemAllocAsync")),
                cu_mem_free_async: cast_fn(sym(c"cuMemFreeAsync")),
                // G1.1 external resource interop:非致命解析(无 `?`;缺失 → None →
                // 上层 interop 报运行期不可用,核心 CUDA 不受影响,RFC-0001 §4.2.3)。
                cu_device_get_luid: cast_fn(sym(c"cuDeviceGetLuid")),
                cu_import_external_memory: cast_fn(sym(c"cuImportExternalMemory")),
                cu_external_memory_get_mapped_buffer: cast_fn(sym(
                    c"cuExternalMemoryGetMappedBuffer",
                )),
                cu_destroy_external_memory: cast_fn(sym(c"cuDestroyExternalMemory")),
                cu_import_external_semaphore: cast_fn(sym(c"cuImportExternalSemaphore")),
                cu_signal_external_semaphores_async: cast_fn(sym(
                    c"cuSignalExternalSemaphoresAsync",
                )),
                cu_wait_external_semaphores_async: cast_fn(sym(c"cuWaitExternalSemaphoresAsync")),
                cu_destroy_external_semaphore: cast_fn(sym(c"cuDestroyExternalSemaphore")),
                // G1.5 fatbin:非致命解析(无 `?`;缺失 → 装载协商降级保守 PTX fallback,
                // 核心 PTX 装载 cuModuleLoadDataEx 不受影响,D-207 / RXS-0151 / U22)。
                cu_module_load_data: cast_fn(sym(c"cuModuleLoadData")),
                cu_device_get_attribute: cast_fn(sym(c"cuDeviceGetAttribute")),
            })
        }
    }

    pub fn init(&self) -> CuResult {
        // SAFETY: cuInit ABI = fn(u32)->CUresult;flags=0 合法(08 §2.1)。
        unsafe { (self.cu_init)(0) }
    }

    pub fn device_count(&self) -> (CuResult, i32) {
        let mut n = 0;
        // SAFETY: 出参 `n` 为有效可写 i32;ABI 匹配。
        let r = unsafe { (self.cu_device_get_count)(&mut n) };
        (r, n)
    }

    pub fn device_get(&self, ordinal: i32) -> (CuResult, CuDevice) {
        let mut dev = 0;
        // SAFETY: 出参 `dev` 有效可写;ABI 匹配。
        let r = unsafe { (self.cu_device_get)(&mut dev, ordinal) };
        (r, dev)
    }

    pub fn ctx_create(&self, dev: CuDevice) -> (CuResult, CuPtr) {
        let mut ctx: CuPtr = std::ptr::null_mut();
        // SAFETY: 出参 `ctx` 有效可写;flags=0 合法;`dev` 来自 device_get。
        let r = unsafe { (self.cu_ctx_create)(&mut ctx, 0, dev) };
        (r, ctx)
    }

    /// # Safety
    /// `ctx` 必须是 `ctx_create` 返回且尚未销毁的 current context 句柄。
    pub unsafe fn ctx_destroy(&self, ctx: CuPtr) -> CuResult {
        // SAFETY: 调用方保证 `ctx` 为有效未销毁句柄(见 fn 文档)。
        unsafe { (self.cu_ctx_destroy)(ctx) }
    }

    pub fn ctx_synchronize(&self) -> CuResult {
        // SAFETY: 作用于 current context(由 ctx_create 设置);无指针入参。
        unsafe { (self.cu_ctx_sync)() }
    }

    /// 保留设备 primary context(`cuDevicePrimaryCtxRetain`;互操作零拷贝:与
    /// PyTorch/CuPy runtime API 共享同一 context,设备指针同 context 直接可用,
    /// M8.1 UC-01 / RXS-0125)。
    pub fn primary_ctx_retain(&self, dev: CuDevice) -> (CuResult, CuPtr) {
        let mut ctx: CuPtr = std::ptr::null_mut();
        // SAFETY: 出参 `ctx` 有效可写;`dev` 来自 device_get。
        let r = unsafe { (self.cu_primary_ctx_retain)(&mut ctx, dev) };
        (r, ctx)
    }

    /// # Safety
    /// `dev` 必须是此前 `primary_ctx_retain` 成功的设备序号(retain/release 配对)。
    pub unsafe fn primary_ctx_release(&self, dev: CuDevice) -> CuResult {
        // SAFETY: 调用方保证 retain/release 配对(见 fn 文档)。
        unsafe { (self.cu_primary_ctx_release)(dev) }
    }

    /// # Safety
    /// `ctx` 必须是有效未销毁的 context 句柄(或 null = 解绑 current)。
    pub unsafe fn ctx_set_current(&self, ctx: CuPtr) -> CuResult {
        // SAFETY: 调用方保证 `ctx` 有效(见 fn 文档)。
        unsafe { (self.cu_ctx_set_current)(ctx) }
    }

    pub fn stream_create(&self) -> (CuResult, CuPtr) {
        let mut s: CuPtr = std::ptr::null_mut();
        // SAFETY: 出参 `s` 有效可写;flags=0(default stream 行为)。
        let r = unsafe { (self.cu_stream_create)(&mut s, 0) };
        (r, s)
    }

    /// # Safety
    /// `stream` 必须是 `stream_create` 返回且尚未销毁的句柄。
    pub unsafe fn stream_destroy(&self, stream: CuPtr) -> CuResult {
        // SAFETY: 调用方保证 `stream` 有效未销毁(见 fn 文档)。
        unsafe { (self.cu_stream_destroy)(stream) }
    }

    /// # Safety
    /// `stream` 必须是有效句柄(或 null = default stream)。
    pub unsafe fn stream_synchronize(&self, stream: CuPtr) -> CuResult {
        // SAFETY: 调用方保证 `stream` 有效(见 fn 文档)。
        unsafe { (self.cu_stream_sync)(stream) }
    }

    pub fn mem_alloc(&self, bytes: usize) -> (CuResult, CuDevicePtr) {
        let mut ptr: CuDevicePtr = 0;
        // SAFETY: 出参 `ptr` 有效可写;bytes 为请求字节数(0 由驱动裁决)。
        let r = unsafe { (self.cu_mem_alloc)(&mut ptr, bytes) };
        (r, ptr)
    }

    /// # Safety
    /// `ptr` 必须是 `mem_alloc` 返回且未释放的设备地址。
    pub unsafe fn mem_free(&self, ptr: CuDevicePtr) -> CuResult {
        // SAFETY: 调用方保证 `ptr` 为有效未释放设备地址(见 fn 文档)。
        unsafe { (self.cu_mem_free)(ptr) }
    }

    pub fn mem_alloc_host(&self, bytes: usize) -> (CuResult, *mut c_void) {
        let mut ptr: *mut c_void = std::ptr::null_mut();
        // SAFETY: 出参 `ptr` 有效可写;返回锁页主机内存指针。
        let r = unsafe { (self.cu_mem_alloc_host)(&mut ptr, bytes) };
        (r, ptr)
    }

    /// # Safety
    /// `ptr` 必须是 `mem_alloc_host` 返回且未释放的锁页主机指针。
    pub unsafe fn mem_free_host(&self, ptr: *mut c_void) -> CuResult {
        // SAFETY: 调用方保证 `ptr` 为有效未释放锁页主机指针(见 fn 文档)。
        unsafe { (self.cu_mem_free_host)(ptr) }
    }

    /// # Safety
    /// `dst` 为有效设备地址且 `[dst, dst+bytes)` 在分配范围内;`src` 指向至少
    /// `bytes` 字节的可读主机内存。
    pub unsafe fn memcpy_htod(
        &self,
        dst: CuDevicePtr,
        src: *const c_void,
        bytes: usize,
    ) -> CuResult {
        // SAFETY: 调用方保证 dst/src 范围有效(见 fn 文档)。
        unsafe { (self.cu_memcpy_htod)(dst, src, bytes) }
    }

    /// # Safety
    /// `dst` 指向至少 `bytes` 字节的可写主机内存;`src` 为有效设备地址且范围内。
    pub unsafe fn memcpy_dtoh(&self, dst: *mut c_void, src: CuDevicePtr, bytes: usize) -> CuResult {
        // SAFETY: 调用方保证 dst/src 范围有效(见 fn 文档)。
        unsafe { (self.cu_memcpy_dtoh)(dst, src, bytes) }
    }

    /// # Safety
    /// `image` 指向 NUL 结尾的 PTX 文本;`opts`/`vals` 为长度 `n` 的有效平行数组。
    pub unsafe fn module_load_data_ex(
        &self,
        image: *const c_void,
        n: u32,
        opts: *mut i32,
        vals: *mut *mut c_void,
    ) -> (CuResult, CuPtr) {
        let mut m: CuPtr = std::ptr::null_mut();
        // SAFETY: 出参 `m` 有效可写;调用方保证 image/opts/vals 有效(见 fn 文档)。
        let r = unsafe { (self.cu_module_load_data_ex)(&mut m, image, n, opts, vals) };
        (r, m)
    }

    /// # Safety
    /// `module` 必须是 `module_load_data_ex` 返回且未卸载的句柄。
    pub unsafe fn module_unload(&self, module: CuPtr) -> CuResult {
        // SAFETY: 调用方保证 `module` 有效未卸载(见 fn 文档)。
        unsafe { (self.cu_module_unload)(module) }
    }

    /// # Safety
    /// `module` 为有效已装载模块;`name` 为 NUL 结尾 kernel 名。
    pub unsafe fn module_get_function(
        &self,
        module: CuPtr,
        name: *const c_char,
    ) -> (CuResult, CuPtr) {
        let mut f: CuPtr = std::ptr::null_mut();
        // SAFETY: 出参 `f` 有效可写;调用方保证 module/name 有效(见 fn 文档)。
        let r = unsafe { (self.cu_module_get_function)(&mut f, module, name) };
        (r, f)
    }

    /// # Safety
    /// `f` 为有效 kernel 句柄;`params` 为按 kernel 形参顺序的指针数组(各元素指向
    /// 对应实参存储),长度与 kernel 形参一致;`stream` 有效(或 null)。
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn launch_kernel(
        &self,
        f: CuPtr,
        grid: [u32; 3],
        block: [u32; 3],
        shared_bytes: u32,
        stream: CuPtr,
        params: *mut *mut c_void,
    ) -> CuResult {
        // SAFETY: 调用方保证 f/params/stream 有效且 params 与 kernel 形参匹配(见 fn 文档)。
        unsafe {
            (self.cu_launch_kernel)(
                f,
                grid[0],
                grid[1],
                grid[2],
                block[0],
                block[1],
                block[2],
                shared_bytes,
                stream,
                params,
                std::ptr::null_mut(),
            )
        }
    }

    /// `CUresult` → 错误名(`cuGetErrorName`;失败返回 None)。
    pub fn error_name(&self, code: CuResult) -> Option<String> {
        let mut p: *const c_char = std::ptr::null();
        // SAFETY: 出参 `p` 有效可写;cuGetErrorName 写入静态 C 字符串地址。
        let r = unsafe { (self.cu_get_error_name)(code, &mut p) };
        if r != CUDA_SUCCESS || p.is_null() {
            return None;
        }
        // SAFETY: cuGetErrorName 成功时 `p` 指向 NUL 结尾静态字符串(进程生命期)。
        Some(
            unsafe { core::ffi::CStr::from_ptr(p) }
                .to_string_lossy()
                .into_owned(),
        )
    }

    /// `CUresult` → 错误描述(`cuGetErrorString`;失败返回 None)。
    pub fn error_string(&self, code: CuResult) -> Option<String> {
        let mut p: *const c_char = std::ptr::null();
        // SAFETY: 出参 `p` 有效可写;cuGetErrorString 写入静态 C 字符串地址。
        let r = unsafe { (self.cu_get_error_string)(code, &mut p) };
        if r != CUDA_SUCCESS || p.is_null() {
            return None;
        }
        // SAFETY: cuGetErrorString 成功时 `p` 指向 NUL 结尾静态字符串(进程生命期)。
        Some(
            unsafe { core::ffi::CStr::from_ptr(p) }
                .to_string_lossy()
                .into_owned(),
        )
    }

    // -- M8.3 UC-02:event 跨 stream 同步 + 异步搬运(三 stream 重叠,RXS-0131) ----

    /// 创建 event(`cuEventCreate`;default 标志,M8.3 跨 stream 流序依赖)。
    pub fn event_create(&self) -> (CuResult, CuPtr) {
        let mut e: CuPtr = std::ptr::null_mut();
        // SAFETY: 出参 `e` 有效可写;flags=CU_EVENT_DEFAULT 合法。
        let r = unsafe { (self.cu_event_create)(&mut e, CU_EVENT_DEFAULT) };
        (r, e)
    }

    /// # Safety
    /// `event` 为有效未销毁 event 句柄;`stream` 为有效句柄(或 null = default);两者同 current context。
    pub unsafe fn event_record(&self, event: CuPtr, stream: CuPtr) -> CuResult {
        // SAFETY: 调用方保证 event/stream 有效且同 context(见 fn 文档)。
        unsafe { (self.cu_event_record)(event, stream) }
    }

    /// # Safety
    /// `event` 必须是 `event_create` 返回且尚未销毁的句柄。
    pub unsafe fn event_destroy(&self, event: CuPtr) -> CuResult {
        // SAFETY: 调用方保证 `event` 有效未销毁(见 fn 文档)。
        unsafe { (self.cu_event_destroy)(event) }
    }

    /// # Safety
    /// `event` 为有效已 record 的 event 句柄。
    pub unsafe fn event_synchronize(&self, event: CuPtr) -> CuResult {
        // SAFETY: 调用方保证 `event` 有效(见 fn 文档)。
        unsafe { (self.cu_event_sync)(event) }
    }

    /// # Safety
    /// `stream` 有效(或 null);`event` 为有效已(或将)record 的 event,两者同 current context。
    pub unsafe fn stream_wait_event(&self, stream: CuPtr, event: CuPtr) -> CuResult {
        // SAFETY: 调用方保证 stream/event 有效且同 context(见 fn 文档)。
        unsafe { (self.cu_stream_wait_event)(stream, event, CU_STREAM_WAIT_DEFAULT) }
    }

    /// # Safety
    /// `dst` 为有效设备地址且 `[dst, dst+bytes)` 在分配内;`src` 指向 ≥ `bytes` 字节、在 stream
    /// 操作完成前保持有效的(宜为锁页)主机内存;`stream` 有效(或 null)。
    pub unsafe fn memcpy_htod_async(
        &self,
        dst: CuDevicePtr,
        src: *const c_void,
        bytes: usize,
        stream: CuPtr,
    ) -> CuResult {
        // SAFETY: 调用方保证 dst/src/stream 有效且 src 在异步拷贝完成前存活(见 fn 文档)。
        unsafe { (self.cu_memcpy_htod_async)(dst, src, bytes, stream) }
    }

    /// # Safety
    /// `dst` 指向 ≥ `bytes` 字节、在 stream 操作完成前保持有效的(宜为锁页)主机内存;`src` 为有效
    /// 设备地址且范围内;`stream` 有效(或 null)。
    pub unsafe fn memcpy_dtoh_async(
        &self,
        dst: *mut c_void,
        src: CuDevicePtr,
        bytes: usize,
        stream: CuPtr,
    ) -> CuResult {
        // SAFETY: 调用方保证 dst/src/stream 有效且 dst 在异步拷贝完成前存活(见 fn 文档)。
        unsafe { (self.cu_memcpy_dtoh_async)(dst, src, bytes, stream) }
    }

    // -- G1.2 流序分配:stream-ordered allocator(`cuMemAllocAsync` + `CUmemoryPool`,RXS-0144) ----
    // 符号缺失时返回 None(老驱动无流序分配);上层 AsyncBuffer 映射运行期不可用。

    /// driver 是否导出流序分配符号(`cuMemAllocAsync`/`cuMemFreeAsync`;CUDA 11.2+,U19)。
    pub fn has_stream_ordered_alloc(&self) -> bool {
        self.cu_mem_alloc_async.is_some() && self.cu_mem_free_async.is_some()
    }

    /// 流序分配 `bytes` 字节到 `stream` 的 ordered memory pool(`cuMemAllocAsync`;默认池;
    /// 分配在 `stream` 上排队,同 stream 后续操作经 stream 序排在其后,RXS-0144/0145)。
    /// # Safety
    /// `stream` 必须是有效 stream 句柄(或 null = default);current context 一致。
    pub unsafe fn mem_alloc_async(
        &self,
        bytes: usize,
        stream: CuPtr,
    ) -> Option<(CuResult, CuDevicePtr)> {
        let f = self.cu_mem_alloc_async?;
        let mut ptr: CuDevicePtr = 0;
        // SAFETY: (U19):出参 `ptr` 有效可写;调用方保证 `stream` 有效且 current context 一致(见 fn 文档)。
        let r = unsafe { f(&mut ptr, bytes, stream) };
        Some((r, ptr))
    }

    /// 流序释放 `ptr`(`cuMemFreeAsync`;入 `stream` 序释放回 pool,RXS-0144)。
    /// # Safety
    /// `ptr` 必须是 `mem_alloc_async` 返回且未释放的设备地址;`stream` 有效(或 null);current context 一致。
    pub unsafe fn mem_free_async(&self, ptr: CuDevicePtr, stream: CuPtr) -> Option<CuResult> {
        let f = self.cu_mem_free_async?;
        // SAFETY: (U19):调用方保证 `ptr` 有效未释放、`stream` 有效且 current context 一致(见 fn 文档)。
        Some(unsafe { f(ptr, stream) })
    }

    // -- G1.1 CUDA–D3D12 互操作:external memory/semaphore(RXS-0140/0142/0143;RFC-0001 §4.2/§4.3) --
    // 全部入口在符号缺失时返回 None(driver 不支持 external resource interop);上层
    // interop 映射运行期诊断。signal/wait 单信号量(numExtSems=1)。

    /// driver 是否导出全部 external-resource interop 符号(G1.1 可用性前置判定)。
    pub fn has_external_resource_api(&self) -> bool {
        self.cu_device_get_luid.is_some()
            && self.cu_import_external_memory.is_some()
            && self.cu_external_memory_get_mapped_buffer.is_some()
            && self.cu_destroy_external_memory.is_some()
            && self.cu_import_external_semaphore.is_some()
            && self.cu_signal_external_semaphores_async.is_some()
            && self.cu_wait_external_semaphores_async.is_some()
            && self.cu_destroy_external_semaphore.is_some()
    }

    /// 设备 LUID + node mask(`cuDeviceGetLuid`;与 D3D12 adapter LUID 逐字节配对,RFC-0001 §4.4)。
    pub fn device_get_luid(&self, dev: CuDevice) -> Option<(CuResult, [c_char; 8], u32)> {
        let f = self.cu_device_get_luid?;
        let mut luid = [0 as c_char; 8];
        let mut node_mask: u32 = 0;
        // SAFETY: 出参 `luid`(8 字节)/`node_mask` 有效可写;`dev` 来自 device_get;ABI 匹配。
        let r = unsafe { f(luid.as_mut_ptr(), &mut node_mask, dev) };
        Some((r, luid, node_mask))
    }

    /// # Safety
    /// `desc` 指向有效 `CudaExternalMemoryHandleDesc`,其 `win32.handle` 为有效 NT HANDLE;current context 一致。
    pub unsafe fn import_external_memory(
        &self,
        desc: *const CudaExternalMemoryHandleDesc,
    ) -> Option<(CuResult, CuPtr)> {
        let f = self.cu_import_external_memory?;
        let mut ext: CuPtr = std::ptr::null_mut();
        // SAFETY: 出参 `ext` 有效可写;调用方保证 `desc` 有效(见 fn 文档)。
        let r = unsafe { f(&mut ext, desc) };
        Some((r, ext))
    }

    /// # Safety
    /// `ext_mem` 为 `import_external_memory` 成功返回且未销毁的句柄;`desc` 指向有效 buffer desc。
    pub unsafe fn external_memory_get_mapped_buffer(
        &self,
        ext_mem: CuPtr,
        desc: *const CudaExternalMemoryBufferDesc,
    ) -> Option<(CuResult, CuDevicePtr)> {
        let f = self.cu_external_memory_get_mapped_buffer?;
        let mut dptr: CuDevicePtr = 0;
        // SAFETY: 出参 `dptr` 有效可写;调用方保证 `ext_mem`/`desc` 有效(见 fn 文档)。
        let r = unsafe { f(&mut dptr, ext_mem, desc) };
        Some((r, dptr))
    }

    /// # Safety
    /// `ext_mem` 为 `import_external_memory` 返回且 mapped buffer 已 `cuMemFree` 后的句柄(RFC-0001 §4.4 销毁序)。
    pub unsafe fn destroy_external_memory(&self, ext_mem: CuPtr) -> Option<CuResult> {
        let f = self.cu_destroy_external_memory?;
        // SAFETY: 调用方保证 `ext_mem` 有效未销毁且 mapped buffer 已先释放(见 fn 文档)。
        Some(unsafe { f(ext_mem) })
    }

    /// # Safety
    /// `desc` 指向有效 `CudaExternalSemaphoreHandleDesc`,其 `win32.handle` 为有效 NT HANDLE。
    pub unsafe fn import_external_semaphore(
        &self,
        desc: *const CudaExternalSemaphoreHandleDesc,
    ) -> Option<(CuResult, CuPtr)> {
        let f = self.cu_import_external_semaphore?;
        let mut ext: CuPtr = std::ptr::null_mut();
        // SAFETY: 出参 `ext` 有效可写;调用方保证 `desc` 有效(见 fn 文档)。
        let r = unsafe { f(&mut ext, desc) };
        Some((r, ext))
    }

    /// 在 `stream` 上 signal external semaphore 到 `value`(`cuSignalExternalSemaphoresAsync`,numExtSems=1)。
    /// # Safety
    /// `ext_sem` 为有效 external semaphore;`stream` 有效(或 null);current context 一致。
    pub unsafe fn signal_external_semaphore(
        &self,
        ext_sem: CuPtr,
        value: u64,
        stream: CuPtr,
    ) -> Option<CuResult> {
        let f = self.cu_signal_external_semaphores_async?;
        let params = CudaExternalSemaphoreParams {
            fence_value: value,
            nv_sci_sync: 0,
            keyed_mutex_key: 0,
            params_reserved: [0; 12],
            flags: 0,
            reserved: [0; 16],
        };
        let sems = [ext_sem];
        // SAFETY: `sems`/`params` 为长度 1 的有效栈数组(numExtSems=1);调用方保证 ext_sem/stream
        // 有效且 current context 一致(见 fn 文档)。
        Some(unsafe { f(sems.as_ptr(), &params, 1, stream) })
    }

    /// 在 `stream` 上 wait external semaphore 至 `value`(`cuWaitExternalSemaphoresAsync`,numExtSems=1)。
    /// # Safety
    /// `ext_sem` 为有效 external semaphore;`stream` 有效(或 null);current context 一致。
    pub unsafe fn wait_external_semaphore(
        &self,
        ext_sem: CuPtr,
        value: u64,
        stream: CuPtr,
    ) -> Option<CuResult> {
        let f = self.cu_wait_external_semaphores_async?;
        let params = CudaExternalSemaphoreParams {
            fence_value: value,
            nv_sci_sync: 0,
            keyed_mutex_key: 0,
            params_reserved: [0; 12],
            flags: 0,
            reserved: [0; 16],
        };
        let sems = [ext_sem];
        // SAFETY: `sems`/`params` 为长度 1 的有效栈数组(numExtSems=1);调用方保证 ext_sem/stream
        // 有效且 current context 一致(见 fn 文档)。
        Some(unsafe { f(sems.as_ptr(), &params, 1, stream) })
    }

    /// # Safety
    /// `ext_sem` 为 `import_external_semaphore` 返回且无在途 signal/wait 的句柄(RFC-0001 §4.4)。
    pub unsafe fn destroy_external_semaphore(&self, ext_sem: CuPtr) -> Option<CuResult> {
        let f = self.cu_destroy_external_semaphore?;
        // SAFETY: 调用方保证 `ext_sem` 有效未销毁且无在途操作(见 fn 文档)。
        Some(unsafe { f(ext_sem) })
    }

    // -- G1.5 生产分发 fatbin:按架构预编 cubin 装载 + compute capability 查询(RXS-0150/0151;U22) --
    // 符号缺失时返回 None / has_cubin_load=false → 上层装载协商降级保守 PTX fallback(D-207)。

    /// driver 是否导出 cubin 装载 + sm 查询符号(`cuModuleLoadData`/`cuDeviceGetAttribute`;U22)。
    /// 否 → 装载协商降级保守 PTX fallback(D-207,RXS-0151;核心 PTX 路径不受影响)。
    pub fn has_cubin_load(&self) -> bool {
        self.cu_module_load_data.is_some() && self.cu_device_get_attribute.is_some()
    }

    /// 查询 device compute capability `(major, minor)`(`cuDeviceGetAttribute`;构造 sm 架构键供
    /// 装载协商 `select_load_variant`,RXS-0151)。符号缺失 → `None`(降级 PTX fallback)。
    pub fn device_compute_capability(&self, dev: CuDevice) -> Option<(CuResult, u32, u32)> {
        let f = self.cu_device_get_attribute?;
        let mut major: i32 = 0;
        let mut minor: i32 = 0;
        // SAFETY: (U22):出参 `major` 有效可写;attrib 为合法 `CUdevice_attribute` 常量;
        // `dev` 来自 device_get;ABI 匹配(`fn(*mut i32, i32, CUdevice) -> CUresult`)。
        let r1 = unsafe {
            f(
                &mut major,
                CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MAJOR,
                dev,
            )
        };
        if r1 != CUDA_SUCCESS {
            return Some((r1, 0, 0));
        }
        // SAFETY: (U22):同上,查 minor。
        let r2 = unsafe {
            f(
                &mut minor,
                CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MINOR,
                dev,
            )
        };
        Some((r2, major.max(0) as u32, minor.max(0) as u32))
    }

    /// 装载按架构预编 cubin 二进制(`cuModuleLoadData`;首启免 JIT,RXS-0151)。符号缺失 →
    /// `None`(装载协商降级保守 PTX fallback,D-207)。
    /// # Safety
    /// (U22):`image` 指向有效的 cubin 二进制(`ptxas -arch=sm_xx` 预编产物,宜与 device 架构
    /// 匹配);cubin 被驱动拒绝(架构不符等)时由调用方降级 PTX 重试(保守兜底,**不 poison**)。
    pub unsafe fn module_load_data(&self, image: *const c_void) -> Option<(CuResult, CuPtr)> {
        let f = self.cu_module_load_data?;
        let mut m: CuPtr = std::ptr::null_mut();
        // SAFETY: (U22):出参 `m` 有效可写;调用方保证 `image` 指向有效 cubin 二进制(见 fn 文档)。
        let r = unsafe { f(&mut m, image) };
        Some((r, m))
    }
}
