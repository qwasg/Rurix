//! cublas v2 C API 原始 FFI 与 runtime DLL 动态加载(09 cublas 绑定包;M8.2 / RXS-0126)。
//!
//! **动态加载**(非链接期绑定,对齐 [`rurix_rt`] `nvcuda.dll` 先例):cublas runtime
//! DLL(`cublas64_*.dll`,Attachment A 白名单最小集)经 `LoadLibraryA`/`GetProcAddress`
//! 运行时装载——不依赖 CUDA Toolkit 的 `cublas.lib` 导入库(开发机无 Toolkit 时仍可
//! 编译,host-only CI 不致链接死)。符号缺失 / cublas 不可用 → [`Cublas::load`] 返回
//! `None`(上层映射 [`crate::RX_CUBLAS_HANDLE_INIT_FAILED`])。
//!
//! **unsafe 边界**(AGENTS 硬规则 9,注册见 `unsafe-audit/rurix-cublas.md`):全部
//! `unsafe` 集中于此——每个 [`Cublas`] 方法以单个 `unsafe` 块前向到已加载的 cublas v2
//! C API 函数指针;调用约定 / 签名与 cublas ABI(`extern "C"`,Windows x64 唯一 ABI)
//! 对齐,句柄 / 设备指针有效性由上层(safe wrapper [`crate::CublasHandle`] RAII + 调用方
//! 设备指针契约)维持。device 指针(A/B/C/x/y)以 `u64` 设备地址按值传参(x64 GP 寄存器
//! 与 `const float*` 同宽,ABI 兼容,FFI 层不解引用);alpha/beta 为**主机**标量指针
//! (`CUBLAS_POINTER_MODE_HOST` 默认)。

use core::ffi::{c_char, c_int, c_void};
use std::sync::OnceLock;

/// `cublasStatus_t`(0 = `CUBLAS_STATUS_SUCCESS`)。
pub type CublasStatus = c_int;
/// 不透明 `cublasHandle_t` 句柄(05 §FFI:复杂类型不透明句柄)。
pub type CublasHandleRaw = *mut c_void;

pub const CUBLAS_STATUS_SUCCESS: CublasStatus = 0;

/// `cublasOperation_t`:不转置(列主序原样)。
pub const CUBLAS_OP_N: c_int = 0;
/// `cublasOperation_t`:转置(行主序 ↔ 列主序适配,RXS-0128)。
pub const CUBLAS_OP_T: c_int = 1;

/// 候选 cublas runtime DLL 名(Attachment A 白名单最小集;CUDA 13.x = `cublas64_13.dll`,
/// 12.x = `cublas64_12.dll`,按序尝试)。完整 Toolkit / 驱动 / Nsight 永不捆绑(许可红线
/// r6,RXS-0129);本绑定仅动态加载已安装的 runtime DLL,物理附带 / 再分发承接 M8.4。
pub const CUBLAS_DLL_CANDIDATES: &[&core::ffi::CStr] =
    &[c"cublas64_13.dll", c"cublas64_12.dll", c"cublas64_11.dll"];

// -- Windows 动态加载(kernel32;std 默认链接,无需 toolkit) -----------------

unsafe extern "system" {
    fn LoadLibraryA(name: *const c_char) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const c_char) -> *mut c_void;
}

// -- cublas v2 C API 函数指针类型(extern "C",Windows x64 唯一 ABI) -------------
//
// device 指针(a/b/c/x/y)以 u64 设备地址按值传参(ABI 等价 const float* / float*);
// alpha/beta 为主机标量指针。

type FnCreate = unsafe extern "C" fn(*mut CublasHandleRaw) -> CublasStatus;
type FnDestroy = unsafe extern "C" fn(CublasHandleRaw) -> CublasStatus;
#[allow(clippy::type_complexity)]
type FnSgemm = unsafe extern "C" fn(
    CublasHandleRaw,
    c_int,      // transa
    c_int,      // transb
    c_int,      // m
    c_int,      // n
    c_int,      // k
    *const f32, // alpha(host)
    u64,        // A(device)
    c_int,      // lda
    u64,        // B(device)
    c_int,      // ldb
    *const f32, // beta(host)
    u64,        // C(device)
    c_int,      // ldc
) -> CublasStatus;
#[allow(clippy::type_complexity)]
type FnSgemv = unsafe extern "C" fn(
    CublasHandleRaw,
    c_int,      // trans
    c_int,      // m
    c_int,      // n
    *const f32, // alpha(host)
    u64,        // A(device)
    c_int,      // lda
    u64,        // x(device)
    c_int,      // incx
    *const f32, // beta(host)
    u64,        // y(device)
    c_int,      // incy
) -> CublasStatus;

/// 已加载的 cublas v2 C API 入口集(进程内一次加载,函数指针 Send + Sync)。
pub struct Cublas {
    cublas_create: FnCreate,
    cublas_destroy: FnDestroy,
    cublas_sgemm: FnSgemm,
    cublas_sgemv: FnSgemv,
    /// 实际加载成功的 runtime DLL 名(Attachment A 白名单审计留痕,RXS-0129)。
    loaded_dll: &'static str,
}

static CUBLAS: OnceLock<Option<Cublas>> = OnceLock::new();

/// 进程内单次加载 cublas runtime DLL 的 v2 C API 入口(失败 → `None`,上层映射
/// [`crate::RX_CUBLAS_HANDLE_INIT_FAILED`])。
pub fn cublas() -> Option<&'static Cublas> {
    CUBLAS.get_or_init(Cublas::load).as_ref()
}

/// 从 `*mut c_void`(GetProcAddress 返回)转成类型化函数指针;null → None。
///
/// # Safety
/// `raw` 必须是 cublas runtime DLL 中名为该符号、且其 ABI 与 `T`(`unsafe extern "C"
/// fn`)一致的导出函数地址(cublas v2 C API 稳定 ABI)。
unsafe fn cast_fn<T: Copy>(raw: *mut c_void) -> Option<T> {
    if raw.is_null() {
        return None;
    }
    debug_assert_eq!(size_of::<T>(), size_of::<*mut c_void>());
    // SAFETY: raw 非 null(已查);T 为指针宽度函数指针(debug_assert 校核);
    // 调用方保证 raw 为匹配 ABI 的导出符号地址(见 fn 文档)。
    Some(unsafe { std::mem::transmute_copy::<*mut c_void, T>(&raw) })
}

impl Cublas {
    /// 加载 cublas runtime DLL(Attachment A 候选名按序尝试)并解析全部所需 v2 C API
    /// 符号(任一缺失 → None)。
    fn load() -> Option<Cublas> {
        // SAFETY: `LoadLibraryA`/`GetProcAddress` 为 Win32 稳定 ABI(kernel32);入参为
        // NUL 结尾 C 字符串字面量(`c"..."`);返回的模块 / 符号地址仅经 `cast_fn` 在
        // null 校验后转为匹配 ABI 的函数指针。每个符号名与其上方类型别名签名按 cublas
        // v2 C API ABI 一一对应。仅尝试 Attachment A 白名单最小集 DLL 名(RXS-0129)。
        unsafe {
            for cand in CUBLAS_DLL_CANDIDATES {
                let lib = LoadLibraryA(cand.as_ptr());
                if lib.is_null() {
                    continue;
                }
                let sym = |name: &core::ffi::CStr| GetProcAddress(lib, name.as_ptr());
                let create = cast_fn(sym(c"cublasCreate_v2"));
                let destroy = cast_fn(sym(c"cublasDestroy_v2"));
                let sgemm = cast_fn(sym(c"cublasSgemm_v2"));
                let sgemv = cast_fn(sym(c"cublasSgemv_v2"));
                if let (Some(create), Some(destroy), Some(sgemm), Some(sgemv)) =
                    (create, destroy, sgemm, sgemv)
                {
                    return Some(Cublas {
                        cublas_create: create,
                        cublas_destroy: destroy,
                        cublas_sgemm: sgemm,
                        cublas_sgemv: sgemv,
                        loaded_dll: cand.to_str().unwrap_or("cublas64_?.dll"),
                    });
                }
            }
            None
        }
    }

    /// 加载成功的 runtime DLL 名(Attachment A 白名单审计留痕,RXS-0129)。
    pub fn loaded_dll(&self) -> &'static str {
        self.loaded_dll
    }

    /// `cublasCreate_v2`:在 current context 创建句柄(出参 `handle`)。
    pub fn create(&self) -> (CublasStatus, CublasHandleRaw) {
        let mut h: CublasHandleRaw = std::ptr::null_mut();
        // SAFETY: 出参 `h` 有效可写;cublasCreate_v2 ABI = fn(cublasHandle_t*)->status;
        // 句柄绑定到 current(primary)context(由 rurix-rt Context::from_primary 设置)。
        let r = unsafe { (self.cublas_create)(&mut h) };
        (r, h)
    }

    /// # Safety
    /// `handle` 必须是 `create` 返回且尚未销毁的句柄。
    pub unsafe fn destroy(&self, handle: CublasHandleRaw) -> CublasStatus {
        // SAFETY: 调用方保证 `handle` 为有效未销毁句柄(见 fn 文档;CublasHandle RAII 维持)。
        unsafe { (self.cublas_destroy)(handle) }
    }

    /// `cublasSgemm_v2`(列主序);行主序适配由上层 [`crate::gemm`] 经参数交换完成。
    ///
    /// # Safety
    /// `handle` 有效;`a`/`b`/`c` 为 current context 内有效设备地址,容量满足
    /// (lda/ldb/ldc 与 m/n/k 相容);`alpha`/`beta` 指向有效主机 `f32`。
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn sgemm(
        &self,
        handle: CublasHandleRaw,
        transa: c_int,
        transb: c_int,
        m: c_int,
        n: c_int,
        k: c_int,
        alpha: *const f32,
        a: u64,
        lda: c_int,
        b: u64,
        ldb: c_int,
        beta: *const f32,
        c: u64,
        ldc: c_int,
    ) -> CublasStatus {
        // SAFETY: 调用方保证 handle/设备地址/主机标量指针有效且维度相容(见 fn 文档)。
        unsafe {
            (self.cublas_sgemm)(
                handle, transa, transb, m, n, k, alpha, a, lda, b, ldb, beta, c, ldc,
            )
        }
    }

    /// `cublasSgemv_v2`(列主序);行主序适配由上层 [`crate::gemv`] 经 `CUBLAS_OP_T` 完成。
    ///
    /// # Safety
    /// `handle` 有效;`a`/`x`/`y` 为 current context 内有效设备地址,容量满足;
    /// `alpha`/`beta` 指向有效主机 `f32`。
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn sgemv(
        &self,
        handle: CublasHandleRaw,
        trans: c_int,
        m: c_int,
        n: c_int,
        alpha: *const f32,
        a: u64,
        lda: c_int,
        x: u64,
        incx: c_int,
        beta: *const f32,
        y: u64,
        incy: c_int,
    ) -> CublasStatus {
        // SAFETY: 调用方保证 handle/设备地址/主机标量指针有效且维度相容(见 fn 文档)。
        unsafe { (self.cublas_sgemv)(handle, trans, m, n, alpha, a, lda, x, incx, beta, y, incy) }
    }
}
