//! Compute 后端抽象(mb1,RXS-0206;RFC-0011 §4.5)。把「跑一个 compute:artifact→module→
//! buffers→launch→readback」收敛为单一 `ComputeBackend` trait,**CUDA 收敛为一实现、Vulkan
//! 为并列实现**。纯 host 薄层,**零 unsafe**(组合各后端 safe public API + safe 字节转换)——
//! 不入 unsafe-audit、不新增 U 号。后端选择显式(`RURIX_BACKEND`),**无隐式 fallback**(P-01):
//! 选定后端不可用 → 确定性 `Err`,绝不自动改跑另一后端。NVIDIA(CUDA)零回归:CUDA 实现只调
//! 既有 `Context`/`Module`/`Stream`/`DeviceBuffer` pub 方法,不触 sys.rs/pipeline.rs/vk.rs。

use core::ffi::c_void;

#[cfg(feature = "vulkan")]
use crate::vk;

/// 一次 compute dispatch 的后端无关描述(in/out 原位回写)。
pub struct ComputeJob<'a> {
    /// 编译产物字节:CUDA = PTX 文本(UTF-8);Vulkan = SPIR-V 字流小端字节(len%4==0)。
    pub artifact: &'a [u8],
    /// 入口符号名(kernel 名 / `OpEntryPoint` 名;codegen mangled 符号)。
    pub entry: &'a str,
    /// StorageBuffer / kernel 指针实参对应 host 数据(in/out,原位回写);
    /// 顺序 = (set 0, binding i) / kernel 指针形参序。
    pub buffers: &'a mut [Vec<u8>],
    /// 标量实参块:Vulkan = push constant 块字节;CUDA = 尾随标量(marshalling 归 RXS-0208)。
    pub scalars: &'a [u8],
    /// 工作组数([x,y,z];= `vkCmdDispatch` / `cuLaunchKernel` grid)。
    pub groups: [u32; 3],
    /// 每工作组线程数([x,y,z]);Vulkan 侧 LocalSize 编码在 SPIR-V 内,CUDA 为 block。
    pub block: [u32; 3],
}

/// 统一后端错误(host 侧收敛;P-01 fail-closed,无静默 fallback)。
#[derive(Debug)]
pub enum BackendError {
    /// 请求后端未编译进本二进制(feature 缺失)。
    NotCompiled(&'static str),
    /// `RURIX_BACKEND` 未知取值。
    Unknown(String),
    /// 后端执行失败(CUDA driver / Vulkan runtime 诊断)。
    Run(String),
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendError::NotCompiled(b) => write!(f, "backend `{b}` 未编译(feature 缺失)"),
            BackendError::Unknown(v) => write!(f, "未知 backend `{v}`(合法:cuda/vulkan)"),
            BackendError::Run(e) => write!(f, "backend 执行失败: {e}"),
        }
    }
}

impl std::error::Error for BackendError {}

/// Compute 后端(RXS-0206):`open` 建会话 → `dispatch` 跑一个 compute。
pub trait ComputeBackend {
    /// 后端持久句柄(**关联类型**):CUDA = 拥有 `CUcontext` 的 [`crate::Context`];
    /// Vulkan = ZST(instance/device 每次 dispatch 由 `vk::run_compute` 一次性自建)。
    type Session;

    /// 后端稳定标识(`"cuda"` / `"vulkan"`;诊断/选择器)。
    fn name(&self) -> &'static str;

    /// 建立会话(打开 device/context)。fail-closed:不可用 → `Err`。
    fn open(&self) -> Result<Self::Session, BackendError>;

    /// 一次 compute:artifact→module→buffers 上传→launch→同步→回读(`job.buffers` 原位回写)。
    fn dispatch(
        &self,
        session: &Self::Session,
        job: &mut ComputeJob<'_>,
    ) -> Result<(), BackendError>;
}

// ── CUDA 后端(组合既有 pub API,零改 Context/Stream) ────────────────────────

/// CUDA 后端(RXS-0206;组合既有 Context/Module/Stream/DeviceBuffer public API,零改其类型)。
pub struct CudaBackend;

/// CUDA 会话 = 拥有 `CUcontext` 的 affine 根(可跨多次 dispatch 复用)。
pub struct CudaSession {
    ctx: crate::Context,
}

fn cuda_err(e: crate::CudaError) -> BackendError {
    BackendError::Run(format!("{e:?}"))
}

impl ComputeBackend for CudaBackend {
    type Session = CudaSession;

    fn name(&self) -> &'static str {
        "cuda"
    }

    fn open(&self) -> Result<CudaSession, BackendError> {
        Ok(CudaSession {
            ctx: crate::Context::new().map_err(cuda_err)?,
        })
    }

    fn dispatch(&self, s: &CudaSession, job: &mut ComputeJob<'_>) -> Result<(), BackendError> {
        let ptx = core::str::from_utf8(job.artifact)
            .map_err(|_| BackendError::Run("CUDA artifact 非 UTF-8 PTX".into()))?;
        let module = s.ctx.load_module(ptx).map_err(cuda_err)?;
        let kernel = module.function(job.entry).map_err(cuda_err)?;
        // buffers → 设备内存 + H2D(binding 序)。
        let mut bufs = Vec::with_capacity(job.buffers.len());
        for host in job.buffers.iter() {
            let mut d = s.ctx.alloc::<u8>(host.len().max(1)).map_err(cuda_err)?;
            d.copy_from_host(host).map_err(cuda_err)?;
            bufs.push(d);
        }
        // 实参 marshalling:buffer 设备指针(binding 序)装配为 kernelParams。标量块 ABI
        // 装配细节 = RXS-0208(本条只承诺 buffer orchestration + geometry)。
        let mut ptr_store: Vec<crate::sys::CuDevicePtr> =
            bufs.iter().map(|b| b.device_ptr()).collect();
        let mut params: Vec<*mut c_void> = ptr_store
            .iter_mut()
            .map(|p| (p as *mut crate::sys::CuDevicePtr).cast::<c_void>())
            .collect();
        let stream = s.ctx.create_stream().map_err(cuda_err)?;
        stream
            .launch(&kernel, job.groups, job.block, &mut params)
            .map_err(cuda_err)?;
        stream.synchronize().map_err(cuda_err)?;
        // 回读(D2H)。
        for (host, d) in job.buffers.iter_mut().zip(bufs.iter()) {
            d.copy_to_host(host).map_err(cuda_err)?;
        }
        Ok(())
    }
}

// ── Vulkan 后端(1:1 委托 vk::run_compute,零改 vk.rs) ───────────────────────

/// Vulkan 后端(RXS-0206/0207;委托 `vk::run_compute`)。
#[cfg(feature = "vulkan")]
pub struct VulkanBackend;

/// Vulkan 会话 = ZST:instance/device/queue 由 `vk::run_compute` 每次一次性自建。
#[cfg(feature = "vulkan")]
pub struct VulkanSession;

/// SPIR-V 小端字节 → 字流(fail-closed:非 4 的倍数 → Err)。
#[cfg(feature = "vulkan")]
fn bytes_to_spirv(b: &[u8]) -> Result<Vec<u32>, BackendError> {
    if !b.len().is_multiple_of(4) {
        return Err(BackendError::Run("SPIR-V 字节数非 4 的倍数".into()));
    }
    Ok(b.chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect())
}

#[cfg(feature = "vulkan")]
impl ComputeBackend for VulkanBackend {
    type Session = VulkanSession;

    fn name(&self) -> &'static str {
        "vulkan"
    }

    fn open(&self) -> Result<VulkanSession, BackendError> {
        Ok(VulkanSession)
    }

    fn dispatch(&self, _s: &VulkanSession, job: &mut ComputeJob<'_>) -> Result<(), BackendError> {
        let spv = bytes_to_spirv(job.artifact)?;
        vk::run_compute(&spv, job.entry, job.buffers, job.scalars, job.groups)
            .map_err(BackendError::Run)
    }
}

// ── 后端选择器(P-01 无隐式 fallback) ───────────────────────────────────────

/// 运行期后端种类(选择器出参)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Cuda,
    Vulkan,
}

/// 解析显式后端取值(纯函数,可 host 单测,不读 env)。未编译的后端 → 确定性 `Err`。
pub fn parse_backend(s: &str) -> Result<BackendKind, BackendError> {
    match s {
        "cuda" => Ok(BackendKind::Cuda),
        "vulkan" => {
            #[cfg(feature = "vulkan")]
            {
                Ok(BackendKind::Vulkan)
            }
            #[cfg(not(feature = "vulkan"))]
            {
                Err(BackendError::NotCompiled("vulkan"))
            }
        }
        other => Err(BackendError::Unknown(other.to_string())),
    }
}

/// 显式选定后端:`RURIX_BACKEND`(cuda|vulkan)。未设 = 默认 CUDA(核心后端,NVIDIA 零回归)。
/// **默认选择 ≠ 运行期 fallback**:选定 CUDA 后驱动不可用 → 确定性 `Err`,绝不自动改跑 Vulkan(P-01)。
pub fn select_backend() -> Result<BackendKind, BackendError> {
    match std::env::var("RURIX_BACKEND") {
        Ok(v) => parse_backend(&v),
        Err(_) => Ok(BackendKind::Cuda),
    }
}

/// 选定后端 → open + dispatch 一次 compute(枚举分派,绕开关联类型 object-safety)。
pub fn run_job(kind: BackendKind, job: &mut ComputeJob<'_>) -> Result<(), BackendError> {
    match kind {
        BackendKind::Cuda => {
            let be = CudaBackend;
            let s = be.open()?;
            be.dispatch(&s, job)
        }
        #[cfg(feature = "vulkan")]
        BackendKind::Vulkan => {
            let be = VulkanBackend;
            let s = be.open()?;
            be.dispatch(&s, job)
        }
        #[cfg(not(feature = "vulkan"))]
        BackendKind::Vulkan => Err(BackendError::NotCompiled("vulkan")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0206
    #[test]
    fn parse_backend_explicit_no_fallback() {
        assert_eq!(parse_backend("cuda").unwrap(), BackendKind::Cuda);
        // 未知取值 → 确定性 Err(非静默默认)。
        assert!(matches!(
            parse_backend("metal"),
            Err(BackendError::Unknown(_))
        ));
        assert!(matches!(parse_backend(""), Err(BackendError::Unknown(_))));
        // vulkan:feature 编译时 Ok,否则 NotCompiled——均非静默 fallback。
        match parse_backend("vulkan") {
            Ok(BackendKind::Vulkan) => {}
            Err(BackendError::NotCompiled("vulkan")) => {}
            other => panic!("vulkan 解析非预期: {other:?}"),
        }
    }
}
