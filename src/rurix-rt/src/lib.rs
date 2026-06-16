//! Rurix 运行时 `rurix-rt`(M4.3,契约 D-M4-4;08 §1/§2,D-230~D-234)。
//!
//! CUDA Driver API **薄层**(P-05:不做调度器/自动内存管理/隐式搬运)。
//! Context(affine 根)/ Stream / DeviceBuffer / PinnedBuffer / Module / Kernel 的
//! RAII 封装,经典内存路径(`cuMemAlloc`/`cuMemAllocHost`,显式 H2D/D2H,D-232),
//! 装载协商(PTX `.version` 比对降版,RXS-0076 / 08 §2.4),poisoned context
//! 状态机(`CUDA_ERROR_ASSERT`/`CONTEXT_IS_DESTROYED` → 确定性错误,RXS-0077 /
//! 08 §2.5)。`nvcuda.dll` 运行时动态加载(见 [`sys`]),不依赖 CUDA Toolkit。
//!
//! 资源归属经生命周期 brand 编码(D-107 的 M4.3 形态):`Stream<'ctx>` /
//! `DeviceBuffer<'ctx, T>` / `Module<'ctx>` 借用 `&'ctx Context`,跨 context 误用
//! 与逃逸为借用检查错误;完整 affine 销毁纪律(stream 先同步)随 M5 深化。

mod error;
pub mod sys;

use core::ffi::c_void;
use core::marker::PhantomData;
use std::cell::Cell;

pub use error::{CudaError, Result};

use sys::{CuDevice, CuDevicePtr, CuPtr};

/// PTX `.version` 协商降版阶梯(08 §2.4;高→低,驱动不支持时逐级回退)。
const PTX_VERSION_LADDER: [&str; 3] = ["8.0", "7.8", "7.0"];

/// GPU 上下文(affine 根,D-231):拥有 `CUcontext`,!Send + !Sync(current
/// context 线程绑定);携 poisoned 状态机(RXS-0077)。
pub struct Context {
    raw: CuPtr,
    /// 设备序号(primary context retain/release 配对需要;互操作零拷贝 M8.1)。
    device: CuDevice,
    /// `true` = primary context(`cuDevicePrimaryCtxRetain`,Drop 走 release);
    /// `false` = 独占 context(`cuCtxCreate`,Drop 走 destroy)。
    primary: bool,
    /// poisoned 触发点(`Some((op, code))` 后全部操作确定性失败,08 §2.5)。
    poison: Cell<Option<(&'static str, sys::CuResult)>>,
    /// `*mut` 字段已使 Context !Send/!Sync;显式标注语义(affine,线程绑定)。
    _not_sync: PhantomData<*const ()>,
}

impl Context {
    /// 在默认设备(序号 0)上创建 context(`cuInit` + `cuCtxCreate`,D-231)。
    pub fn new() -> Result<Context> {
        Self::on_device(0)
    }

    /// 在指定设备序号上创建 context。
    pub fn on_device(ordinal: i32) -> Result<Context> {
        let cuda = sys::cuda().ok_or(CudaError::DriverUnavailable)?;
        error::check("cuInit", cuda.init())?;
        let (r, dev) = cuda.device_get(ordinal);
        error::check("cuDeviceGet", r)?;
        let (r, raw) = cuda.ctx_create(dev);
        error::check("cuCtxCreate", r)?;
        Ok(Context {
            raw,
            device: dev,
            primary: false,
            poison: Cell::new(None),
            _not_sync: PhantomData,
        })
    }

    /// 保留并绑定设备 primary context(`cuDevicePrimaryCtxRetain` +
    /// `cuCtxSetCurrent`;互操作零拷贝 M8.1 / RXS-0125)。
    ///
    /// 与 PyTorch / CuPy 等基于 CUDA runtime API 的框架**共享同一 primary
    /// context**——外部框架(如 PyTorch CUDA caching allocator)分配的设备指针在
    /// 同一 context 内直接有效,launch 复用 M5 自研 kernel 即可零拷贝读写,无需
    /// 跨 context 搬运。Drop 走 `cuDevicePrimaryCtxRelease`(不 destroy 共享 context)。
    pub fn from_primary(ordinal: i32) -> Result<Context> {
        let cuda = sys::cuda().ok_or(CudaError::DriverUnavailable)?;
        error::check("cuInit", cuda.init())?;
        let (r, dev) = cuda.device_get(ordinal);
        error::check("cuDeviceGet", r)?;
        let (r, raw) = cuda.primary_ctx_retain(dev);
        error::check("cuDevicePrimaryCtxRetain", r)?;
        // SAFETY: raw 由 primary_ctx_retain 成功返回,为有效 context 句柄。
        let r = unsafe { cuda.ctx_set_current(raw) };
        if r != sys::CUDA_SUCCESS {
            // SAFETY: dev 刚 retain 成功,release 与之配对(回滚 retain)。
            unsafe {
                let _ = cuda.primary_ctx_release(dev);
            }
            error::check("cuCtxSetCurrent", r)?;
        }
        Ok(Context {
            raw,
            device: dev,
            primary: true,
            poison: Cell::new(None),
            _not_sync: PhantomData,
        })
    }

    /// 设备数(`cuInit` 后查询;0 = 无可用 GPU)。
    pub fn device_count() -> Result<i32> {
        let cuda = sys::cuda().ok_or(CudaError::DriverUnavailable)?;
        error::check("cuInit", cuda.init())?;
        let (r, n) = cuda.device_count();
        error::check("cuDeviceGetCount", r)?;
        Ok(n)
    }

    /// poisoned 守卫(RXS-0077):已 poisoned → 确定性 `Poisoned` 错误。
    fn guard(&self) -> Result<&'static sys::Cuda> {
        if let Some((triggered_by, code)) = self.poison.get() {
            return Err(CudaError::Poisoned { triggered_by, code });
        }
        sys::cuda().ok_or(CudaError::DriverUnavailable)
    }

    /// 处理 `CUresult`:poisoning 码置 poisoned 状态后再映射(RXS-0077 / 08 §2.5)。
    fn finish(&self, op: &'static str, code: sys::CuResult) -> Result<()> {
        if code != sys::CUDA_SUCCESS && error::is_poisoning(code) {
            self.poison.set(Some((op, code)));
        }
        error::check(op, code)
    }

    /// 设备内存分配(`cuMemAlloc`,D-232):`n` 个 `T`(未初始化)。
    pub fn alloc<T: Copy>(&self, n: usize) -> Result<DeviceBuffer<'_, T>> {
        let cuda = self.guard()?;
        let bytes = n.checked_mul(size_of::<T>()).expect("alloc 字节数溢出");
        let (r, ptr) = cuda.mem_alloc(bytes);
        self.finish("cuMemAlloc", r)?;
        Ok(DeviceBuffer {
            ctx: self,
            ptr,
            len: n,
            owned: true,
            _t: PhantomData,
        })
    }

    /// 从外部设备指针构造**借用**缓冲(零拷贝互操作 M8.1 / RXS-0123 / RXS-0124)。
    ///
    /// 设备内存由外部框架(PyTorch / CuPy,经 `__cuda_array_interface__` v3 或
    /// DLPack capsule)拥有;本缓冲仅借用其设备地址用于 launch,**Drop 不
    /// `cuMemFree`**(所有权留在外部 deleter,affine 借用,不悬垂 / 不双重释放)。
    /// 借用 brand 绑 `'ctx`,不晚于 context;须与外部内存同一 device primary context
    /// ([`Context::from_primary`])以保证设备指针在本 context 内有效。
    ///
    /// # Safety
    /// 调用方必须保证 `ptr` 是在本 `Context` 设备上有效、可读写、至少容纳 `len`
    /// 个 `T` 的设备地址,且在本借用缓冲存活期间保持有效(外部 deleter 未释放)。
    pub unsafe fn from_device_ptr<T: Copy>(
        &self,
        ptr: CuDevicePtr,
        len: usize,
    ) -> DeviceBuffer<'_, T> {
        DeviceBuffer {
            ctx: self,
            ptr,
            len,
            owned: false,
            _t: PhantomData,
        }
    }

    /// 锁页主机内存分配(`cuMemAllocHost`,D-232;pinned staging)。
    pub fn alloc_pinned<T: Copy>(&self, n: usize) -> Result<PinnedBuffer<'_, T>> {
        let cuda = self.guard()?;
        let bytes = n
            .checked_mul(size_of::<T>())
            .expect("alloc_pinned 字节数溢出");
        let (r, ptr) = cuda.mem_alloc_host(bytes);
        self.finish("cuMemAllocHost", r)?;
        Ok(PinnedBuffer {
            ptr: ptr.cast::<T>(),
            len: n,
            _marker: PhantomData,
        })
    }

    /// 创建 stream(`cuStreamCreate`;affine 资源,brand 绑 `'ctx`)。
    pub fn create_stream(&self) -> Result<Stream<'_>> {
        let cuda = self.guard()?;
        let (r, raw) = cuda.stream_create();
        self.finish("cuStreamCreate", r)?;
        Ok(Stream { ctx: self, raw })
    }

    /// 装载 PTX 模块 + 协商(`cuModuleLoadDataEx` 驱动内 JIT,RXS-0076 / 08 §2.4)。
    ///
    /// 解析 PTX `.version`,自其起按 [`PTX_VERSION_LADDER`] 降版重试;
    /// `UNSUPPORTED_PTX_VERSION` 触发降级,阶梯耗尽 → `LoadNegotiation`(携指引)。
    pub fn load_module(&self, ptx: &str) -> Result<Module<'_>> {
        let cuda = self.guard()?;
        let start = parse_ptx_version(ptx);
        let mut ladder: Vec<String> = Vec::new();
        if let Some(v) = &start {
            ladder.push(v.clone());
        }
        for v in PTX_VERSION_LADDER {
            if !ladder.iter().any(|x| x == v) {
                ladder.push(v.to_owned());
            }
        }

        let mut last_jit_log = String::new();
        for version in &ladder {
            let text = set_ptx_version(ptx, version);
            let Ok(image) = std::ffi::CString::new(text) else {
                continue; // PTX 文本含内嵌 NUL(异常):跳过该候选
            };
            let mut info_buf = vec![0u8; 8192];
            let mut err_buf = vec![0u8; 8192];
            let mut opts: [i32; 4] = [
                sys::CU_JIT_INFO_LOG_BUFFER,
                sys::CU_JIT_INFO_LOG_BUFFER_SIZE_BYTES,
                sys::CU_JIT_ERROR_LOG_BUFFER,
                sys::CU_JIT_ERROR_LOG_BUFFER_SIZE_BYTES,
            ];
            let mut vals: [*mut c_void; 4] = [
                info_buf.as_mut_ptr().cast::<c_void>(),
                8192usize as *mut c_void,
                err_buf.as_mut_ptr().cast::<c_void>(),
                8192usize as *mut c_void,
            ];
            // SAFETY: image 为 NUL 结尾 PTX 文本(CString);opts/vals 为长度 4 的
            // 平行有效数组,缓冲区 info_buf/err_buf 在调用期存活(08 §2.4 日志常开)。
            let (r, raw) = unsafe {
                cuda.module_load_data_ex(
                    image.as_ptr().cast::<c_void>(),
                    4,
                    opts.as_mut_ptr(),
                    vals.as_mut_ptr(),
                )
            };
            if r == sys::CUDA_SUCCESS {
                return Ok(Module {
                    ctx: self,
                    raw,
                    version: version.clone(),
                });
            }
            last_jit_log = cstr_prefix(&err_buf);
            if error::is_poisoning(r) {
                self.poison.set(Some(("cuModuleLoadDataEx", r)));
                return Err(CudaError::Poisoned {
                    triggered_by: "cuModuleLoadDataEx",
                    code: r,
                });
            }
            if r != sys::CUDA_ERROR_UNSUPPORTED_PTX_VERSION {
                // 非版本协商类失败:直接报 Driver(保留原始码);r≠SUCCESS 故必 Err
                self.finish("cuModuleLoadDataEx", r)?;
            }
        }
        Err(CudaError::LoadNegotiation {
            tried: ladder,
            jit_log: last_jit_log,
        })
    }

    /// 同步 current context(`cuCtxSynchronize`;阻塞至全部排队操作完成)。
    pub fn synchronize(&self) -> Result<()> {
        let cuda = self.guard()?;
        self.finish("cuCtxSynchronize", cuda.ctx_synchronize())
    }

    /// 是否已 poisoned(RXS-0077;供程序判定是否需重建 context 子树)。
    pub fn is_poisoned(&self) -> bool {
        self.poison.get().is_some()
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // 销毁纪律(D-231):先同步再释放;Drop 中错误吞掉(无 panic)。
        if let Some(cuda) = sys::cuda() {
            let _ = cuda.ctx_synchronize();
            if self.primary {
                // SAFETY: self.device 由 from_primary 经 primary_ctx_retain 成功,
                // 与本次 release 配对(retain/release 引用计数),Drop 仅一次。
                unsafe {
                    let _ = cuda.primary_ctx_release(self.device);
                }
            } else {
                // SAFETY: self.raw 由 ctx_create 产出且本类型独占,Drop 仅一次。
                unsafe {
                    let _ = cuda.ctx_destroy(self.raw);
                }
            }
        }
    }
}

/// 设备内存缓冲(`cuMemAlloc`;brand 绑 `'ctx`,D-232)。
pub struct DeviceBuffer<'ctx, T: Copy> {
    ctx: &'ctx Context,
    ptr: CuDevicePtr,
    len: usize,
    /// `true` = 本类型拥有(`cuMemAlloc`,Drop free);`false` = 借用外部设备内存
    /// (互操作零拷贝,Drop 不 free;所有权在外部 deleter,M8.1)。
    owned: bool,
    _t: PhantomData<T>,
}

impl<T: Copy> DeviceBuffer<'_, T> {
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn byte_len(&self) -> usize {
        self.len * size_of::<T>()
    }

    /// 设备地址(launch 实参构造消费)。
    pub fn device_ptr(&self) -> CuDevicePtr {
        self.ptr
    }

    /// H2D 拷贝(`cuMemcpyHtoD`,D-232;`src` 长度须 ≤ 容量)。
    pub fn copy_from_host(&mut self, src: &[T]) -> Result<()> {
        assert!(src.len() <= self.len, "copy_from_host: 源长度超出缓冲容量");
        let cuda = self.ctx.guard()?;
        let bytes = size_of_val(src);
        // SAFETY: self.ptr 为 ≥ bytes 的设备分配;src 为 bytes 字节有效可读主机内存。
        let r = unsafe { cuda.memcpy_htod(self.ptr, src.as_ptr().cast::<c_void>(), bytes) };
        self.ctx.finish("cuMemcpyHtoD", r)
    }

    /// D2H 拷贝(`cuMemcpyDtoH`,D-232;`dst` 长度须 ≤ 容量)。
    pub fn copy_to_host(&self, dst: &mut [T]) -> Result<()> {
        assert!(dst.len() <= self.len, "copy_to_host: 目标长度超出缓冲容量");
        let cuda = self.ctx.guard()?;
        let bytes = size_of_val(dst);
        // SAFETY: dst 为 bytes 字节有效可写主机内存;self.ptr 为 ≥ bytes 的设备分配。
        let r = unsafe { cuda.memcpy_dtoh(dst.as_mut_ptr().cast::<c_void>(), self.ptr, bytes) };
        self.ctx.finish("cuMemcpyDtoH", r)
    }
}

impl<T: Copy> Drop for DeviceBuffer<'_, T> {
    fn drop(&mut self) {
        // 借用缓冲(from_device_ptr)不释放:设备内存所有权在外部框架 deleter,
        // 释放它会双重释放(M8.1 零拷贝互操作所有权纪律)。
        if !self.owned {
            return;
        }
        if let Some(cuda) = sys::cuda() {
            // SAFETY: self.ptr 由 mem_alloc 产出且本类型独占(owned),Drop 仅一次。
            unsafe {
                let _ = cuda.mem_free(self.ptr);
            }
        }
    }
}

/// 锁页主机缓冲(`cuMemAllocHost`;pinned staging,D-232)。
pub struct PinnedBuffer<'ctx, T: Copy> {
    ptr: *mut T,
    len: usize,
    /// brand 绑 `'ctx`(不晚于 context;不持有 context 引用,host 内存 Drop 经全局入口)。
    _marker: PhantomData<(&'ctx Context, T)>,
}

impl<T: Copy> PinnedBuffer<'_, T> {
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_slice(&self) -> &[T] {
        // SAFETY: ptr 为 cuMemAllocHost 返回的 len*size_of::<T>() 字节锁页内存,
        // 对齐满足(主机分配),生命期受 &self 约束。
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        // SAFETY: 同上;&mut self 保证独占访问。
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl<T: Copy> Drop for PinnedBuffer<'_, T> {
    fn drop(&mut self) {
        if let Some(cuda) = sys::cuda() {
            // SAFETY: self.ptr 由 mem_alloc_host 产出且本类型独占,Drop 仅一次。
            unsafe {
                let _ = cuda.mem_free_host(self.ptr.cast::<c_void>());
            }
        }
    }
}

/// 提交队列(`cuStream`;FIFO,brand 绑 `'ctx`,D-231)。
pub struct Stream<'ctx> {
    ctx: &'ctx Context,
    raw: CuPtr,
}

impl Stream<'_> {
    /// 在本 stream 上发起 kernel launch(`cuLaunchKernel`,D-232)。
    ///
    /// `params` 为按 kernel 形参顺序的指针数组,各元素指向对应实参存储(设备
    /// 指针 / 标量),长度须与 kernel 形参一致(由调用方维持,启动类型契约在
    /// 编译期由 rurixc launch_check 裁决,RXS-0074)。
    pub fn launch(
        &self,
        kernel: &Kernel<'_>,
        grid: [u32; 3],
        block: [u32; 3],
        params: &mut [*mut c_void],
    ) -> Result<()> {
        let cuda = self.ctx.guard()?;
        // SAFETY: kernel.raw 为有效 kernel 句柄;params 各元素指向调用方维持的有效
        // 实参存储且长度与 kernel 形参匹配;self.raw 为有效 stream;shared=0。
        let r = unsafe {
            cuda.launch_kernel(kernel.raw, grid, block, 0, self.raw, params.as_mut_ptr())
        };
        self.ctx.finish("cuLaunchKernel", r)
    }

    /// 同步本 stream(`cuStreamSynchronize`;阻塞至队列清空)。
    pub fn synchronize(&self) -> Result<()> {
        let cuda = self.ctx.guard()?;
        // SAFETY: self.raw 为有效 stream 句柄。
        let r = unsafe { cuda.stream_synchronize(self.raw) };
        self.ctx.finish("cuStreamSynchronize", r)
    }
}

impl Drop for Stream<'_> {
    fn drop(&mut self) {
        if let Some(cuda) = sys::cuda() {
            // SAFETY: self.raw 由 stream_create 产出且本类型独占,Drop 仅一次。
            unsafe {
                let _ = cuda.stream_destroy(self.raw);
            }
        }
    }
}

/// 已装载 PTX 模块(`cuModule`;brand 绑 `'ctx`,D-231)。
pub struct Module<'ctx> {
    ctx: &'ctx Context,
    raw: CuPtr,
    version: String,
}

impl<'ctx> Module<'ctx> {
    /// 取强类型 kernel 句柄(`cuModuleGetFunction`,06 §5.2)。
    pub fn function(&self, name: &str) -> Result<Kernel<'_>> {
        let cuda = self.ctx.guard()?;
        let cname = std::ffi::CString::new(name).expect("kernel 名含内嵌 NUL");
        // SAFETY: self.raw 为有效模块;cname 为 NUL 结尾 kernel 名。
        let (r, raw) = unsafe { cuda.module_get_function(self.raw, cname.as_ptr()) };
        self.ctx.finish("cuModuleGetFunction", r)?;
        Ok(Kernel {
            raw,
            _m: PhantomData,
        })
    }

    /// 协商后实际装载的 PTX `.version`(RXS-0076)。
    pub fn negotiated_version(&self) -> &str {
        &self.version
    }
}

impl Drop for Module<'_> {
    fn drop(&mut self) {
        if let Some(cuda) = sys::cuda() {
            // SAFETY: self.raw 由 module_load_data_ex 产出且本类型独占,Drop 仅一次。
            unsafe {
                let _ = cuda.module_unload(self.raw);
            }
        }
    }
}

/// 强类型 kernel 句柄(`CUfunction`;brand 绑模块 `'m`,Copy 语义经引用获取)。
pub struct Kernel<'m> {
    raw: CuPtr,
    _m: PhantomData<&'m ()>,
}

// -- PTX `.version` 解析 / 改写(RXS-0076;零依赖,无 regex) -------------------

/// 解析 PTX 首个 `.version X.Y`(协商起点;08 §2.4)。
pub fn parse_ptx_version(ptx: &str) -> Option<String> {
    let idx = ptx.find(".version")?;
    let rest = ptx[idx + ".version".len()..].trim_start();
    let ver: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    if ver.is_empty() { None } else { Some(ver) }
}

/// 改写首个 `.version` 为给定版本(降版协商;不动其余文本)。
fn set_ptx_version(ptx: &str, version: &str) -> String {
    let Some(idx) = ptx.find(".version") else {
        return ptx.to_owned();
    };
    let after = idx + ".version".len();
    let ws_end = after + ptx[after..].len() - ptx[after..].trim_start().len();
    let tail = &ptx[ws_end..];
    let num_len = tail
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .map(char::len_utf8)
        .sum::<usize>();
    format!("{} {}{}", &ptx[..after], version, &tail[num_len..])
}

/// C 字符串缓冲前缀(JIT 日志;截至首个 NUL)。
fn cstr_prefix(buf: &[u8]) -> String {
    let end = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0076
    #[test]
    fn parse_and_rewrite_ptx_version() {
        let ptx = ".version 8.0\n.target sm_89\n.address_size 64\n";
        assert_eq!(parse_ptx_version(ptx).as_deref(), Some("8.0"));
        let rewritten = set_ptx_version(ptx, "7.8");
        assert!(rewritten.contains(".version 7.8"));
        assert!(rewritten.contains(".target sm_89"));
        assert_eq!(parse_ptx_version(&rewritten).as_deref(), Some("7.8"));
    }

    //@ spec: RXS-0076
    #[test]
    fn missing_version_parses_none() {
        assert_eq!(parse_ptx_version(".target sm_89\n"), None);
    }

    //@ spec: RXS-0077
    #[test]
    fn poisoning_codes_classified() {
        // CUDA_ERROR_ASSERT / CONTEXT_IS_DESTROYED 触发 poisoned;成功码不触发(08 §2.5)
        assert!(error::is_poisoning(sys::CUDA_ERROR_ASSERT));
        assert!(error::is_poisoning(sys::CUDA_ERROR_CONTEXT_IS_DESTROYED));
        assert!(!error::is_poisoning(sys::CUDA_SUCCESS));
        assert!(!error::is_poisoning(
            sys::CUDA_ERROR_UNSUPPORTED_PTX_VERSION
        ));
    }
}
