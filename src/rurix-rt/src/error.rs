//! 运行时结构化错误(08 §2.5,D-230;全部 `CUresult` → `enum CudaError`)。
//!
//! 非穷尽枚举 + 原始码保留(异步错误现实:检测点 ≠ 起因点,r4);装载协商失败
//! (RXS-0076)与 poisoned context(RXS-0077)为专门变体,携可执行指引。
//! 装载协商/poisoned 为**运行时**结构化错误(`Result`),不占编译期 RX#### 段位
//! (registry = 编译诊断,07 §5);driver 原始 `CUresult` 经 `code` 字段保留。

use crate::sys;

/// 运行时错误(非穷尽:Driver API 错误面随版本演进)。
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CudaError {
    /// `nvcuda.dll` 不可用或符号缺失(无驱动 / 驱动过旧;08 §2.5)。
    DriverUnavailable,
    /// Driver API 调用返回非成功 `CUresult`(保留函数名 + 原始码 + 驱动错误名)。
    Driver {
        /// 失败的 Driver API 函数名(检测点)。
        op: &'static str,
        /// 原始 `CUresult`(D-230:原始码保留)。
        code: sys::CuResult,
        /// `cuGetErrorName` 文本(不可得时为原始码字符串)。
        name: String,
    },
    /// 装载协商失败(RXS-0076):PTX `.version` 超出驱动 JIT 能力,降版阶梯耗尽。
    /// 携可执行指引(升级驱动 / 重编降低 PTX floor,08 §2.4)。
    LoadNegotiation {
        /// 尝试过的 `.version` 阶梯。
        tried: Vec<String>,
        /// 末次 JIT error log(驱动诊断)。
        jit_log: String,
    },
    /// context 已 poisoned(RXS-0077):`CUDA_ERROR_ASSERT` /
    /// `CONTEXT_IS_DESTROYED` 后,后续操作返回确定性错误而非 UB 级联(08 §2.5)。
    Poisoned {
        /// 触发 poisoned 的 Driver API 函数名。
        triggered_by: &'static str,
        /// 触发时的原始 `CUresult`。
        code: sys::CuResult,
    },
}

impl std::fmt::Display for CudaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CudaError::DriverUnavailable => write!(
                f,
                "CUDA driver (nvcuda.dll) unavailable: no NVIDIA driver installed, or driver too old"
            ),
            CudaError::Driver { op, code, name } => {
                write!(f, "{op} failed: {name} (CUresult {code})")
            }
            CudaError::LoadNegotiation { tried, jit_log } => write!(
                f,
                "PTX load negotiation failed (tried .version {}): the installed driver does not \
                 support this PTX ISA. Upgrade the NVIDIA driver, or recompile with a lower \
                 --ptx-floor. JIT log: {jit_log}",
                tried.join(", ")
            ),
            CudaError::Poisoned { triggered_by, code } => write!(
                f,
                "context is poisoned (triggered by {triggered_by}, CUresult {code}); the context \
                 must be rebuilt — all further operations on it fail deterministically"
            ),
        }
    }
}

impl std::error::Error for CudaError {}

pub type Result<T> = std::result::Result<T, CudaError>;

/// `CUresult` → `Result`(成功 → Ok,否则 `Driver` 变体,保留原始码 + 错误名)。
pub(crate) fn check(op: &'static str, code: sys::CuResult) -> Result<()> {
    if code == sys::CUDA_SUCCESS {
        return Ok(());
    }
    let cuda = sys::cuda();
    let name = cuda
        .and_then(|c| c.error_name(code))
        .unwrap_or_else(|| code.to_string());
    Err(CudaError::Driver { op, code, name })
}

/// `CUresult` 是否触发 context poisoned(08 §2.5,RXS-0077)。
pub(crate) fn is_poisoning(code: sys::CuResult) -> bool {
    code == sys::CUDA_ERROR_ASSERT || code == sys::CUDA_ERROR_CONTEXT_IS_DESTROYED
}
