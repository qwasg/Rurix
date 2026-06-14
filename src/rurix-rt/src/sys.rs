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
type FnStreamCreate = unsafe extern "system" fn(*mut CuPtr, u32) -> CuResult;
type FnStreamDestroy = unsafe extern "system" fn(CuPtr) -> CuResult;
type FnStreamSync = unsafe extern "system" fn(CuPtr) -> CuResult;
type FnMemAlloc = unsafe extern "system" fn(*mut CuDevicePtr, usize) -> CuResult;
type FnMemFree = unsafe extern "system" fn(CuDevicePtr) -> CuResult;
type FnMemAllocHost = unsafe extern "system" fn(*mut *mut c_void, usize) -> CuResult;
type FnMemFreeHost = unsafe extern "system" fn(*mut c_void) -> CuResult;
type FnMemcpyHtoD = unsafe extern "system" fn(CuDevicePtr, *const c_void, usize) -> CuResult;
type FnMemcpyDtoH = unsafe extern "system" fn(*mut c_void, CuDevicePtr, usize) -> CuResult;
type FnModuleLoadDataEx =
    unsafe extern "system" fn(*mut CuPtr, *const c_void, u32, *mut i32, *mut *mut c_void) -> CuResult;
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

/// 已加载的 Driver API 入口集(进程内一次加载,函数指针 Send + Sync)。
pub struct Cuda {
    cu_init: FnInit,
    cu_device_get: FnDeviceGet,
    cu_device_get_count: FnDeviceGetCount,
    cu_ctx_create: FnCtxCreate,
    cu_ctx_destroy: FnCtxDestroy,
    cu_ctx_sync: FnCtxSync,
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
    pub unsafe fn memcpy_htod(&self, dst: CuDevicePtr, src: *const c_void, bytes: usize) -> CuResult {
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
    pub unsafe fn module_get_function(&self, module: CuPtr, name: *const c_char) -> (CuResult, CuPtr) {
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
                f, grid[0], grid[1], grid[2], block[0], block[1], block[2], shared_bytes, stream,
                params, std::ptr::null_mut(),
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
        Some(unsafe { core::ffi::CStr::from_ptr(p) }.to_string_lossy().into_owned())
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
        Some(unsafe { core::ffi::CStr::from_ptr(p) }.to_string_lossy().into_owned())
    }
}
