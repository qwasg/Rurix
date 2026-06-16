//! Rurix cublas 绑定包(M8.2,D-M8-2;spec/cublas.md RXS-0126 ~ RXS-0129)。
//!
//! GEMM/GEMV **三层绑定**(09;01 §6 克制声明——绑定既有高性能库而非重造):
//! - **第 1 层 raw FFI**([`sys`]):cublas v2 C API `extern "C"` 声明面 + `cublasHandle_t`
//!   不透明句柄,cublas runtime DLL 动态加载(对齐 rurix-rt nvcuda 先例;RXS-0126)。
//! - **第 2 层 safe wrapper**([`CublasHandle`] RAII + 设备指针 / 维度合法性校验 +
//!   列主序 / 转置约定 + `cublasStatus_t` 映射;对上**全 safe**,无 `unsafe` 出现在签名;
//!   RXS-0127)。
//! - **第 3 层 高层 API**([`gemm`]/[`gemv`]:复用 [`rurix_rt::Context::from_primary`]
//!   共享 primary context + [`rurix_rt::Context::from_device_ptr`] 借用外部设备指针缓冲,
//!   row-major(Rurix/PyTorch)↔ col-major(cublas)适配;RXS-0128)。
//! - **C ABI 导出层**([`ffi`]):`extern "C"` 薄包供 `ci/cublas_binding_smoke.py` 经
//!   ctypes 以 torch CUDA 张量设备指针零拷贝调用,返回 [`i32`] 错误码。
//!
//! 新段位错误码(07 §5,7xxx 链接 / 工具链续接,接 M8.1 互操作 RX7013~RX7015 之后,
//! 只追加、含义冻结):
//! - [`RX_CUBLAS_HANDLE_INIT_FAILED`](7016):cublas runtime DLL 不可用 / `cublasCreate` 失败。
//! - [`RX_CUBLAS_INVALID_DEVICE_PTR`](7017):设备指针非法(空指针 / 非设备地址)。
//! - [`RX_CUBLAS_DIMENSION_MISMATCH`](7018):维度不匹配(维度为 0 / 算子维度不相容)。
//! - [`RX_CUBLAS_RUNTIME_FAILED`](7019):cublas 执行 / context 同步运行时失败。

pub mod ffi;
pub mod sys;

use rurix_rt::Context;
use sys::{CUBLAS_OP_N, CUBLAS_OP_T, CUBLAS_STATUS_SUCCESS, CublasHandleRaw};

/// 成功(C ABI 返回码,07 §5)。
pub const RX_OK: i32 = 0;
/// RX7016:cublas runtime DLL 不可用 / `cublasCreate` 失败(RXS-0126/0127)。
pub const RX_CUBLAS_HANDLE_INIT_FAILED: i32 = 7016;
/// RX7017:cublas 设备指针非法(空指针 / 非设备地址;RXS-0127)。
pub const RX_CUBLAS_INVALID_DEVICE_PTR: i32 = 7017;
/// RX7018:cublas 维度不匹配(维度为 0 / 算子维度不相容;RXS-0127)。
pub const RX_CUBLAS_DIMENSION_MISMATCH: i32 = 7018;
/// RX7019:cublas 执行 / context 同步运行时失败(`cublasStatus_t != SUCCESS` 等;RXS-0128)。
pub const RX_CUBLAS_RUNTIME_FAILED: i32 = 7019;

/// cublas 句柄的 safe RAII 守卫(RXS-0127):创建于 current(primary)context,Drop 调
/// `cublasDestroy`。对上全 safe(构造 / 析构无 `unsafe` 出现在签名)。
pub struct CublasHandle {
    raw: CublasHandleRaw,
}

impl CublasHandle {
    /// 在 current context 创建 cublas 句柄(须先经 [`Context::from_primary`] 绑定与
    /// PyTorch 共享的 primary context)。cublas 不可用 / 创建失败 → `Err(RX7016)`。
    pub fn create() -> Result<CublasHandle, i32> {
        let lib = sys::cublas().ok_or(RX_CUBLAS_HANDLE_INIT_FAILED)?;
        let (status, raw) = lib.create();
        if status != CUBLAS_STATUS_SUCCESS || raw.is_null() {
            return Err(RX_CUBLAS_HANDLE_INIT_FAILED);
        }
        Ok(CublasHandle { raw })
    }

    /// 加载成功的 cublas runtime DLL 名(Attachment A 白名单审计留痕,RXS-0129)。
    pub fn loaded_dll() -> Option<&'static str> {
        sys::cublas().map(|c| c.loaded_dll())
    }
}

impl Drop for CublasHandle {
    fn drop(&mut self) {
        if let Some(lib) = sys::cublas()
            && !self.raw.is_null()
        {
            // SAFETY: self.raw 由 cublasCreate 成功产出且本类型独占,Drop 仅销毁一次。
            let _ = unsafe { lib.destroy(self.raw) };
        }
    }
}

/// 校验设备指针非空(RXS-0127:设备指针合法性前置)。任一为 0 →
/// [`RX_CUBLAS_INVALID_DEVICE_PTR`]。纯 CPU 校验,先于任何 cublas 调用。
fn validate_ptrs(ptrs: &[u64]) -> Result<(), i32> {
    if ptrs.contains(&0) {
        return Err(RX_CUBLAS_INVALID_DEVICE_PTR);
    }
    Ok(())
}

/// 校验维度全为正(RXS-0127:维度合法性)。任一为 0 → [`RX_CUBLAS_DIMENSION_MISMATCH`]。
fn validate_dims(dims: &[usize]) -> Result<(), i32> {
    if dims.contains(&0) {
        return Err(RX_CUBLAS_DIMENSION_MISMATCH);
    }
    Ok(())
}

/// 高层 GEMM:`C[M,N] = A[M,K] · B[K,N]`(**行主序** f32,零拷贝接入 PyTorch;RXS-0128)。
/// `c`/`a`/`b` 为 PyTorch CUDA 张量设备指针(同一 primary context),返回
/// 0 = 成功 / RX7016 / RX7017 / RX7018 / RX7019。
///
/// 行主序 ↔ cublas 列主序适配:行主序 `C(M×N)` 在内存中 ≡ 列主序 `C^T(N×M)`,故
/// `cublasSgemm(OP_N, OP_N, N, M, K, B, N, A, K, C, N)` 直接产行主序结果(参数交换,
/// 不做显式转置 kernel)。
pub fn gemm(c: u64, a: u64, b: u64, m: usize, n: usize, k: usize) -> i32 {
    if let Err(code) = validate_ptrs(&[c, a, b]) {
        return code;
    }
    if let Err(code) = validate_dims(&[m, n, k]) {
        return code;
    }
    match run_gemm(c, a, b, m, n, k) {
        Ok(()) => RX_OK,
        Err(code) => code,
    }
}

/// 高层 GEMV:`y[M] = A[M,N] · x[N]`(**行主序** f32,零拷贝接入 PyTorch;RXS-0128)。
/// `y`/`a`/`x` 为 PyTorch CUDA 张量设备指针,`m` = A 行数(= y 长度),`n` = A 列数
/// (= x 长度)。返回 0 = 成功 / RX7016 / RX7017 / RX7018 / RX7019。
///
/// 行主序 ↔ cublas 列主序适配:行主序 `A(M×N)` 在内存中 ≡ 列主序 `A_cm(N×M)`,故
/// `cublasSgemv(OP_T, N, M, A, N, x, 1, y, 1)` 经转置直接产行主序 `y = A·x`。
pub fn gemv(y: u64, a: u64, x: u64, m: usize, n: usize) -> i32 {
    if let Err(code) = validate_ptrs(&[y, a, x]) {
        return code;
    }
    if let Err(code) = validate_dims(&[m, n]) {
        return code;
    }
    match run_gemv(y, a, x, m, n) {
        Ok(()) => RX_OK,
        Err(code) => code,
    }
}

fn run_gemm(c: u64, a: u64, b: u64, m: usize, n: usize, k: usize) -> Result<(), i32> {
    let ctx = Context::from_primary(0).map_err(|_| RX_CUBLAS_HANDLE_INIT_FAILED)?;
    // 借用外部设备指针缓冲(affine 借用,Drop 不释放,所有权留外部 deleter;RXS-0128)。
    // 借用仅用于文档化所有权语义 + 取设备地址(零拷贝),cublas 经设备地址直接读写。
    // SAFETY: a/b/c 为 PyTorch CUDA 张量设备指针(行主序 f32:a=m*k,b=k*n,c=m*n),
    // 同一 primary context 内有效、借用期内不释放(RXS-0128;rurix-rt U10)。
    let d_a = unsafe { ctx.from_device_ptr::<f32>(a, m * k) };
    // SAFETY: 同上(b 输入张量,k*n 个 f32)。
    let d_b = unsafe { ctx.from_device_ptr::<f32>(b, k * n) };
    // SAFETY: 同上(c 输出张量,m*n 个 f32)。
    let d_c = unsafe { ctx.from_device_ptr::<f32>(c, m * n) };

    let handle = CublasHandle::create()?;
    let lib = sys::cublas().ok_or(RX_CUBLAS_HANDLE_INIT_FAILED)?;
    let alpha = 2.0f32; // TEMP 篡改 cublas GEMM 绑定数值(α=2 ≠ 1)— 验证步骤35 红;下一提交复原
    let beta = 0.0f32;
    let (mi, ni, ki) = (m as i32, n as i32, k as i32);
    // cublasSgemm(OP_N, OP_N, /*m=*/N, /*n=*/M, /*k=*/K, alpha, /*A=*/B, /*lda=*/N,
    //             /*B=*/A, /*ldb=*/K, beta, /*C=*/C, /*ldc=*/N) → 行主序 C=A·B。
    // SAFETY: handle 有效(刚创建);设备地址 d_a/d_b/d_c 为 current context 内有效、
    // 容量与 m/n/k 相容的 f32 缓冲;alpha/beta 为有效主机标量(CUBLAS_POINTER_MODE_HOST)。
    let status = unsafe {
        lib.sgemm(
            handle.raw,
            CUBLAS_OP_N,
            CUBLAS_OP_N,
            ni,
            mi,
            ki,
            &raw const alpha,
            d_b.device_ptr(),
            ni,
            d_a.device_ptr(),
            ki,
            &raw const beta,
            d_c.device_ptr(),
            ni,
        )
    };
    if status != CUBLAS_STATUS_SUCCESS {
        return Err(RX_CUBLAS_RUNTIME_FAILED);
    }
    ctx.synchronize().map_err(|_| RX_CUBLAS_RUNTIME_FAILED)
}

fn run_gemv(y: u64, a: u64, x: u64, m: usize, n: usize) -> Result<(), i32> {
    let ctx = Context::from_primary(0).map_err(|_| RX_CUBLAS_HANDLE_INIT_FAILED)?;
    // SAFETY: a 为 PyTorch CUDA 张量设备指针(行主序 f32,m*n 元素),同一 primary
    // context 内有效、借用期内不释放(RXS-0128;rurix-rt U10)。
    let d_a = unsafe { ctx.from_device_ptr::<f32>(a, m * n) };
    // SAFETY: 同上(x 输入向量,n 个 f32)。
    let d_x = unsafe { ctx.from_device_ptr::<f32>(x, n) };
    // SAFETY: 同上(y 输出向量,m 个 f32)。
    let d_y = unsafe { ctx.from_device_ptr::<f32>(y, m) };

    let handle = CublasHandle::create()?;
    let lib = sys::cublas().ok_or(RX_CUBLAS_HANDLE_INIT_FAILED)?;
    let alpha = 1.0f32;
    let beta = 0.0f32;
    let (mi, ni) = (m as i32, n as i32);
    // cublasSgemv(OP_T, /*m=*/N, /*n=*/M, alpha, /*A=*/A, /*lda=*/N, /*x=*/x, 1,
    //             beta, /*y=*/y, 1) → 行主序 y = A·x。
    // SAFETY: handle 有效;设备地址 d_a/d_x/d_y 为 current context 内有效、容量与 m/n
    // 相容的 f32 缓冲;alpha/beta 为有效主机标量(CUBLAS_POINTER_MODE_HOST)。
    let status = unsafe {
        lib.sgemv(
            handle.raw,
            CUBLAS_OP_T,
            ni,
            mi,
            &raw const alpha,
            d_a.device_ptr(),
            ni,
            d_x.device_ptr(),
            1,
            &raw const beta,
            d_y.device_ptr(),
            1,
        )
    };
    if status != CUBLAS_STATUS_SUCCESS {
        return Err(RX_CUBLAS_RUNTIME_FAILED);
    }
    ctx.synchronize().map_err(|_| RX_CUBLAS_RUNTIME_FAILED)
}

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0126
    // cublas raw FFI 边界:runtime DLL 候选名为 Attachment A 白名单最小集
    // (cublas64_*.dll),`cublasHandle_t` 为不透明句柄(*mut c_void),v2 C API 符号
    // 经动态加载(对齐 rurix-rt nvcuda 先例,不链接期绑定)。
    #[test]
    fn raw_ffi_dll_candidates_attachment_a() {
        // 候选 DLL 名全部匹配 Attachment A 白名单形态(cublas64_<ver>.dll),
        // 不含完整 Toolkit / 驱动 / Nsight 组件名(许可红线 r6,RXS-0129)。
        assert!(!sys::CUBLAS_DLL_CANDIDATES.is_empty());
        for cand in sys::CUBLAS_DLL_CANDIDATES {
            let name = cand.to_str().unwrap();
            assert!(
                name.starts_with("cublas64_") && name.ends_with(".dll"),
                "cublas runtime DLL 候选 {name} 不在 Attachment A 白名单形态"
            );
        }
        // 不透明句柄为指针宽度(05 §FFI:不透明句柄边界)。
        assert_eq!(
            size_of::<CublasHandleRaw>(),
            size_of::<*mut core::ffi::c_void>()
        );
    }

    //@ spec: RXS-0127
    // safe wrapper:设备指针 / 维度合法性先于任何 cublas 调用(纯 CPU 前置),
    // 空指针 → RX7017,维度 0 → RX7018(对上全 safe,签名无 unsafe)。
    #[test]
    fn safe_wrapper_validates_before_cublas() {
        // 空设备指针(未初始化 / 抽取失败)→ RX7017。
        assert_eq!(gemm(0, 0, 0, 4, 4, 4), RX_CUBLAS_INVALID_DEVICE_PTR);
        assert_eq!(gemv(0, 0, 0, 4, 4), RX_CUBLAS_INVALID_DEVICE_PTR);
        // 维度为 0 → RX7018(指针非 0 占位,仅校验维度)。
        assert_eq!(gemm(16, 32, 48, 0, 4, 4), RX_CUBLAS_DIMENSION_MISMATCH);
        assert_eq!(gemm(16, 32, 48, 4, 0, 4), RX_CUBLAS_DIMENSION_MISMATCH);
        assert_eq!(gemm(16, 32, 48, 4, 4, 0), RX_CUBLAS_DIMENSION_MISMATCH);
        assert_eq!(gemv(16, 32, 48, 0, 4), RX_CUBLAS_DIMENSION_MISMATCH);
        assert_eq!(gemv(16, 32, 48, 4, 0), RX_CUBLAS_DIMENSION_MISMATCH);
    }

    //@ spec: RXS-0128
    // 高层 API ↔ C ABI 薄包返回码语义一致(段位 RX7016~RX7019 + 0/运行时),
    // 校验在 cublas 调用之前,host 上可确定性核对(对齐 safe API)。
    #[test]
    fn ffi_thin_wrapper_codes_consistent() {
        assert_eq!(
            ffi::rurix_cublas_gemm(0, 0, 0, 4, 4, 4),
            RX_CUBLAS_INVALID_DEVICE_PTR
        );
        assert_eq!(
            ffi::rurix_cublas_gemv(16, 32, 48, 4, 0),
            RX_CUBLAS_DIMENSION_MISMATCH
        );
        // 段位常量含义冻结(07 §5):cublas 诊断 7016~7019。
        assert_eq!(RX_CUBLAS_HANDLE_INIT_FAILED, 7016);
        assert_eq!(RX_CUBLAS_INVALID_DEVICE_PTR, 7017);
        assert_eq!(RX_CUBLAS_DIMENSION_MISMATCH, 7018);
        assert_eq!(RX_CUBLAS_RUNTIME_FAILED, 7019);
    }

    //@ spec: RXS-0129
    // cublas runtime DLL 按需附带与 Attachment A 白名单约定:loaded_dll 内省接口存在
    // (审计留痕);候选集仅 runtime DLL,完整 Toolkit / 驱动 / Nsight 永不捆绑(r6)。
    #[test]
    fn runtime_dll_attachment_a_whitelist() {
        // 内省接口可调用(无 GPU / cublas 环境返回 None,建设期正常;有则返回白名单名)。
        if let Some(dll) = CublasHandle::loaded_dll() {
            assert!(
                dll.starts_with("cublas64_") && dll.ends_with(".dll"),
                "加载的 runtime DLL {dll} 不在 Attachment A 白名单"
            );
        }
        // 候选集不含禁止组件(完整 Toolkit / 驱动 / Nsight / 静态库 / libdevice)。
        for cand in sys::CUBLAS_DLL_CANDIDATES {
            let n = cand.to_str().unwrap();
            assert!(!n.contains("nvcuda"), "不得捆绑驱动组件");
            assert!(!n.contains("nsight"), "不得捆绑 Nsight");
            assert!(!n.ends_with(".lib"), "不得捆绑静态导入库");
            assert!(!n.contains("libdevice"), "不得捆绑 libdevice");
        }
    }
}
