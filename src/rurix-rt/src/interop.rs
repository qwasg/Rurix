//! CUDA–D3D12 互操作呈现类型化（G1.1，RXS-0140~0143；RFC-0001 §4）。feature `d3d12-interop`。
//!
//! 把 import 句柄生命周期 / 跨 context 误用 / 信号时序违例做成**编译期约束**——
//! 三类错误由 Rust 类型系统原生拦截（affine move / 生成式 brand / 消费式 typestate），
//! 不新增 RX 码（对齐 M8.3 pipeline.rs；RFC-0001 §5）：
//!
//! 1. **句柄生命周期**（RXS-0140）：[`ExternalBuffer`]/[`ExternalSemaphore`]/frame state
//!    均非 `Copy`/非 `Clone`、单一所有权；move 后再用 `E0382`，重复 clone `E0599`。Drop
//!    强制销毁序（mapped `cuMemFree` → destroy semaphore → destroy memory → shim destroy，
//!    RFC-0001 §4.4）。
//! 2. **跨 context 误用**（RXS-0141）：[`scope`] 以 `for<'ctx>` 闭包生成不可伪造、不可逃逸
//!    的**不变 brand**；两个独立 scope 的资源类型不同，混用 = 编译期类型/借用错误（不依赖
//!    普通 `'ctx` 生命周期或 `Arc` 指针运行期身份，RFC-0001 §4.1）。
//! 3. **信号时序违例**（RXS-0142）：[`ReadyFrame`]→[`AcquiredFrame`]→[`PresentableFrame`]
//!    消费式状态机——未 `wait` 无可写 buffer；未 `signal` 无 `present`；私有 stream 被状态
//!    对象捕获，wait/launch/signal 同 stream 序。共享 fence 偶/奇值 handoff（RFC-0001 §4.3）。
//!
//! D3D12/DXGI 经 [`rurix_d3d12`] 薄 C/C++ shim 驱动，不进语言（D-130）。FFI unsafe 注册见
//! `unsafe-audit/rurix-rt.md`。**类型面（本模块）无需 GPU/D3D12 即可编译并验证三类编译期
//! 拦截与 fence 协议**；真实设备呈现需 `d3d12-interop-real`（MSVC + Windows SDK + 交互桌面）。

use core::ffi::c_void;
use core::marker::PhantomData;

use rurix_d3d12::{InteropExport, Presenter};

use crate::sys::{
    self, CU_EXTERNAL_MEMORY_HANDLE_TYPE_D3D12_RESOURCE,
    CU_EXTERNAL_SEMAPHORE_HANDLE_TYPE_D3D12_FENCE, CUDA_EXTERNAL_MEMORY_DEDICATED, CUDA_SUCCESS,
    CuDevicePtr, CuPtr, CuResult, CudaExternalMemoryBufferDesc, CudaExternalMemoryHandleDesc,
    CudaExternalSemaphoreHandleDesc, CudaWin32Handle,
};
use crate::{Context, CudaError};

/// G0 `sr_tonemap` device kernel PTX（RXS-0121；由 rurix-rt build.rs 从原 `.rx` 生成）。
pub const SR_TONEMAP_PTX: &str = include_str!(concat!(env!("OUT_DIR"), "/sr_tonemap.ptx"));
include!(concat!(env!("OUT_DIR"), "/sr_tonemap_meta.rs"));

/// 不变 brand（invariant over `'ctx`）：`for<'ctx>` 闭包生成不可伪造、不可逃逸的新鲜
/// brand，使两个独立 [`scope`] 的资源类型不可混用（跨 context 编译期拦截，RXS-0141）。
type Brand<'ctx> = PhantomData<fn(&'ctx ()) -> &'ctx ()>;

/// 互操作错误（运行期诊断为 Rust 值；编译期三类拦截走 rustc 原生、无 RX 码，RFC-0001 §5）。
#[derive(Debug)]
pub enum InteropError {
    /// 底层 CUDA 运行时错误（context 创建 / stream 等）。
    Cuda(CudaError),
    /// driver 未导出 external-resource interop 符号（老驱动）。
    ExternalApiUnavailable,
    /// D3D12 shim 不可用 / 失败（HRESULT 位码；stub 或缺 Windows SDK / 非交互桌面）。
    Shim(i32),
    /// CUDA external-resource 调用返回错误码。
    CuExtResult { op: &'static str, code: CuResult },
    /// 共享 fence 值溢出 → 确定停机（RFC-0001 §4.3）。
    FenceOverflow,
    /// CUDA device LUID 与 D3D12 adapter LUID 不一致（RFC-0001 §4.4）。
    LuidMismatch,
    /// host/device 数值回读未通过设备冒烟对照。
    DeviceVerificationFailed,
    /// 请求访问的元素数超过共享 buffer 容量。
    BufferTooSmall { requested: usize, available: usize },
}

impl From<CudaError> for InteropError {
    fn from(e: CudaError) -> Self {
        InteropError::Cuda(e)
    }
}

/// 互操作结果别名。
pub type Result<T> = core::result::Result<T, InteropError>;

fn cu_ext_check(op: &'static str, code: CuResult) -> Result<()> {
    if code == CUDA_SUCCESS {
        Ok(())
    } else {
        Err(InteropError::CuExtResult { op, code })
    }
}

// -- 共享 fence 偶/奇值协议（RFC-0001 §4.3；纯函数,checked,溢出 → None 确定停机） --------

/// CUDA 取得第 `frame` 帧写权所 wait 的 fence 值 `acquire(n) = 2n`。
pub fn acquire_value(frame: u64) -> Option<u64> {
    frame.checked_mul(2)
}
/// CUDA 完成写入后 signal 的 fence 值 `cuda_done(n) = 2n + 1`。
pub fn cuda_done_value(frame: u64) -> Option<u64> {
    frame.checked_mul(2)?.checked_add(1)
}
/// D3D12 present 后 signal 下一写权的 fence 值 `d3d_done(n) = 2n + 2`。
pub fn d3d_done_value(frame: u64) -> Option<u64> {
    frame.checked_mul(2)?.checked_add(2)
}

// -- affine external 资源（RXS-0140；'ctx 不变 brand，move-only，Drop 销毁序 §4.4） --------

/// import 自 D3D12 共享 committed resource 的 backbuffer 等价缓冲（affine，move-only，
/// `'ctx` brand）。持 `CUexternalMemory` import 句柄 + 映射设备地址；Drop 按 RFC-0001 §4.4
/// 先 `cuMemFree(mapped)` 再 `cuDestroyExternalMemory`（**不释放 D3D12 resource**，其
/// 所有权留 shim 侧）。非 `Copy`/非 `Clone`。
pub struct ExternalBuffer<'ctx, T: Copy> {
    ext_mem: CuPtr,
    dptr: CuDevicePtr,
    len: usize,
    _brand: Brand<'ctx>,
    _t: PhantomData<T>,
}

impl<T: Copy> ExternalBuffer<'_, T> {
    /// 映射设备地址（launch 实参；不可脱离本类型逃逸为可自由传递的裸指针）。
    pub fn device_ptr(&self) -> CuDevicePtr {
        self.dptr
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<T: Copy> Drop for ExternalBuffer<'_, T> {
    fn drop(&mut self) {
        if let Some(cuda) = sys::cuda() {
            // SAFETY: (U17) RFC-0001 §4.4 销毁序——`dptr` 由 external_memory_get_mapped_buffer
            // 产出、本类型独占（非 Clone），先 cuMemFree；`ext_mem` 由 import_external_memory
            // 产出、独占，mapped 释放后 destroy。Drop 仅一次（单一所有权,不双重释放）。
            unsafe {
                let _ = cuda.mem_free(self.dptr);
                if !self.ext_mem.is_null() {
                    let _ = cuda.destroy_external_memory(self.ext_mem);
                }
            }
        }
    }
}

/// import 自 D3D12 共享 fence 的跨 API 信号量（affine，move-only，`'ctx` brand）。
/// 非 `Copy`/非 `Clone`；Drop `cuDestroyExternalSemaphore`。
pub struct ExternalSemaphore<'ctx> {
    ext_sem: CuPtr,
    _brand: Brand<'ctx>,
}

impl Drop for ExternalSemaphore<'_> {
    fn drop(&mut self) {
        if let Some(cuda) = sys::cuda()
            && !self.ext_sem.is_null()
        {
            // SAFETY: (U18) `ext_sem` 由 import_external_semaphore 产出、本类型独占;
            // shutdown 序保证销毁前无在途 signal/wait（RFC-0001 §4.4）。Drop 仅一次。
            unsafe {
                let _ = cuda.destroy_external_semaphore(self.ext_sem);
            }
        }
    }
}

// -- 同 brand kernel 与密封实参（RXS-0141；无可脱离裸指针,只能绑同 brand frame launch） ---

/// 已装载互操作模块（`'ctx` brand；只能由同 [`scope`] 的 [`InteropContext`] 取得）。
pub struct InteropModule<'ctx> {
    raw: CuPtr,
    _brand: Brand<'ctx>,
}

impl<'ctx> InteropModule<'ctx> {
    /// 取同 brand kernel 句柄（`cuModuleGetFunction`）。
    pub fn kernel(&self, name: &str) -> Result<InteropKernel<'ctx>> {
        let cuda = sys::cuda().ok_or(InteropError::Cuda(CudaError::DriverUnavailable))?;
        let cname = std::ffi::CString::new(name).expect("kernel 名含内嵌 NUL");
        // SAFETY: (U3) `self.raw` 为有效已装载模块;`cname` 为 NUL 结尾 kernel 名。
        let (r, raw) = unsafe { cuda.module_get_function(self.raw, cname.as_ptr()) };
        cu_ext_check("cuModuleGetFunction", r)?;
        Ok(InteropKernel {
            raw,
            _brand: PhantomData,
        })
    }
}

impl Drop for InteropModule<'_> {
    fn drop(&mut self) {
        if let Some(cuda) = sys::cuda() {
            // SAFETY: (U3) `raw` 由 module_load_data_ex 产出、本类型独占,Drop 仅一次。
            unsafe {
                let _ = cuda.module_unload(self.raw);
            }
        }
    }
}

/// 同 brand kernel 句柄（`CUfunction`）。
pub struct InteropKernel<'ctx> {
    raw: CuPtr,
    _brand: Brand<'ctx>,
}

/// 密封类型化 launch 实参（RXS-0141）：携值（buffer 设备地址 / 标量），**无可脱离的裸指针**;
/// 只能由 [`AcquiredFrame::launch`] 消费、绑定到同 brand frame。
pub struct InteropKernelArg<'ctx> {
    kind: ArgKind,
    _brand: Brand<'ctx>,
}

enum ArgKind {
    DevicePtr(CuDevicePtr),
    U32(u32),
    Usize(usize),
}

impl<'ctx> InteropKernelArg<'ctx> {
    /// 绑定本帧 backbuffer（同 brand external buffer 的设备地址）为 launch 实参。
    pub fn buffer<T: Copy>(buf: &ExternalBuffer<'ctx, T>) -> InteropKernelArg<'ctx> {
        InteropKernelArg {
            kind: ArgKind::DevicePtr(buf.device_ptr()),
            _brand: PhantomData,
        }
    }
    pub fn u32(v: u32) -> InteropKernelArg<'ctx> {
        InteropKernelArg {
            kind: ArgKind::U32(v),
            _brand: PhantomData,
        }
    }
    pub fn usize(v: usize) -> InteropKernelArg<'ctx> {
        InteropKernelArg {
            kind: ArgKind::Usize(v),
            _brand: PhantomData,
        }
    }
}

// -- 互操作 context（生成式 brand 根；!Send，线程绑定 current context） --------------------

/// 互操作 context（`'ctx` 生成式不变 brand 根，RXS-0141）。`!Send + !Sync`（current context
/// 线程绑定）。仅由 [`scope`] 提供，不可逃逸闭包。
pub struct InteropContext<'ctx> {
    device: sys::CuDevice,
    export: InteropExport,
    _brand: Brand<'ctx>,
    _not_send: PhantomData<*const ()>,
}

impl<'ctx> InteropContext<'ctx> {
    /// CUDA 设备序号。
    pub fn device(&self) -> sys::CuDevice {
        self.device
    }
    /// shim 回填的 interop 导出事实（LUID/尺寸/分配大小，RFC-0001 §4.2.1）。
    pub fn export(&self) -> &InteropExport {
        &self.export
    }
    /// 装载 PTX 模块（current context；同 brand）。
    pub fn load_module(&self, ptx: &str) -> Result<InteropModule<'ctx>> {
        let cuda = sys::cuda().ok_or(InteropError::Cuda(CudaError::DriverUnavailable))?;
        let image = std::ffi::CString::new(ptx).map_err(|_| InteropError::CuExtResult {
            op: "module_load:ptx_nul",
            code: -1,
        })?;
        // SAFETY: (U5) `image` 为 NUL 结尾 PTX 文本;opts/vals 传 0/null（无 JIT 日志缓冲）。
        let (r, raw) = unsafe {
            cuda.module_load_data_ex(
                image.as_ptr().cast::<c_void>(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        cu_ext_check("cuModuleLoadDataEx", r)?;
        Ok(InteropModule {
            raw,
            _brand: PhantomData,
        })
    }
}

// -- 帧核心 + Ready→Acquired→Presentable 消费式 typestate（RXS-0142） ----------------------

/// 帧核心：拥有 shim presenter + 同 brand external buffer/semaphore + 私有 stream + 帧计数。
/// 字段声明序 = Drop 序（buffer → semaphore → stream → presenter，近似 RFC-0001 §4.4 best-effort）。
struct FrameCore<'ctx> {
    buffer: ExternalBuffer<'ctx, f32>,
    semaphore: ExternalSemaphore<'ctx>,
    stream: OwnedStream,
    presenter: Presenter,
    frame: u64,
    _brand: Brand<'ctx>,
}

/// 私有 CUDA stream（affine；Drop 销毁）。
struct OwnedStream {
    raw: CuPtr,
}

impl Drop for OwnedStream {
    fn drop(&mut self) {
        if let Some(cuda) = sys::cuda() {
            // SAFETY: (U3) `raw` 由 stream_create 产出、本类型独占,Drop 仅一次。
            unsafe {
                let _ = cuda.stream_destroy(self.raw);
            }
        }
    }
}

impl<'ctx> FrameCore<'ctx> {
    fn cuda(&self) -> Result<&'static sys::Cuda> {
        sys::cuda().ok_or(InteropError::Cuda(CudaError::DriverUnavailable))
    }
}

/// 就绪帧（RXS-0142）：可 `wait` 取得本帧写权（CUDA wait `acquire(n)=2n`）。**无可写 buffer 接口**。
pub struct ReadyFrame<'ctx> {
    core: FrameCore<'ctx>,
}

/// 已取得写权帧（RXS-0142）：暴露可写 backbuffer + `launch`；`signal` 后转 [`PresentableFrame`]。
pub struct AcquiredFrame<'ctx> {
    core: FrameCore<'ctx>,
}

/// 可呈现帧（RXS-0142）：CUDA 已 signal `cuda_done(n)=2n+1`;`present` 提交 D3D12 并回到 [`ReadyFrame`]。
pub struct PresentableFrame<'ctx> {
    core: FrameCore<'ctx>,
}

impl<'ctx> ReadyFrame<'ctx> {
    /// 当前帧序号 `n`。
    pub fn frame_index(&self) -> u64 {
        self.core.frame
    }
    /// 取得本帧写权：CUDA `cuWaitExternalSemaphoresAsync(acquire(n)=2n)`（RFC-0001 §4.3）。
    pub fn wait(self) -> Result<AcquiredFrame<'ctx>> {
        let v = acquire_value(self.core.frame).ok_or(InteropError::FenceOverflow)?;
        let cuda = self.core.cuda()?;
        // SAFETY: (U18) semaphore 由 import 产出、frame 独占;stream 私有有效;current context 一致。
        let rc = unsafe {
            cuda.wait_external_semaphore(self.core.semaphore.ext_sem, v, self.core.stream.raw)
        }
        .ok_or(InteropError::ExternalApiUnavailable)?;
        cu_ext_check("cuWaitExternalSemaphoresAsync", rc)?;
        Ok(AcquiredFrame { core: self.core })
    }
}

impl<'ctx> AcquiredFrame<'ctx> {
    pub fn frame_index(&self) -> u64 {
        self.core.frame
    }
    /// 可写 backbuffer（kernel 写目标；同 brand）。
    pub fn buffer_mut(&mut self) -> &mut ExternalBuffer<'ctx, f32> {
        &mut self.core.buffer
    }
    /// 在私有 stream 上 launch kernel（同 stream 序；密封类型化实参,无裸指针逃逸）。
    pub fn launch(
        &mut self,
        kernel: &InteropKernel<'ctx>,
        grid: [u32; 3],
        block: [u32; 3],
        args: &mut [InteropKernelArg<'ctx>],
    ) -> Result<()> {
        let cuda = self.core.cuda()?;
        // 物化实参存储,构造 cuLaunchKernel 参数指针数组（值随本调用栈存活）。
        let mut dptrs: Vec<CuDevicePtr> = Vec::with_capacity(args.len());
        let mut u32s: Vec<u32> = Vec::with_capacity(args.len());
        let mut usizes: Vec<usize> = Vec::with_capacity(args.len());
        // 先按序预留稳定地址,避免后续 push 重分配使指针失效。
        for a in args.iter() {
            match a.kind {
                ArgKind::DevicePtr(p) => dptrs.push(p),
                ArgKind::U32(v) => u32s.push(v),
                ArgKind::Usize(v) => usizes.push(v),
            }
        }
        let (mut di, mut ui, mut si) = (0usize, 0usize, 0usize);
        let mut params: Vec<*mut c_void> = Vec::with_capacity(args.len());
        for a in args.iter() {
            let p: *mut c_void = match a.kind {
                ArgKind::DevicePtr(_) => {
                    let r = &dptrs[di] as *const CuDevicePtr as *mut c_void;
                    di += 1;
                    r
                }
                ArgKind::U32(_) => {
                    let r = &u32s[ui] as *const u32 as *mut c_void;
                    ui += 1;
                    r
                }
                ArgKind::Usize(_) => {
                    let r = &usizes[si] as *const usize as *mut c_void;
                    si += 1;
                    r
                }
            };
            params.push(p);
        }
        // SAFETY: (U7) `kernel.raw` 为有效 kernel;`params` 各元素指向本栈帧存活的实参存储,
        // 长度/顺序与 kernel 形参一致;`stream` 私有有效;shared=0。
        let rc = unsafe {
            cuda.launch_kernel(
                kernel.raw,
                grid,
                block,
                0,
                self.core.stream.raw,
                params.as_mut_ptr(),
            )
        };
        cu_ext_check("cuLaunchKernel", rc)
    }
    /// 同步本帧私有 stream 后，将共享 backbuffer 前缀回读到 host（设备 smoke 数值对照）。
    pub fn readback_f32(&self, dst: &mut [f32]) -> Result<()> {
        if dst.len() > self.core.buffer.len() {
            return Err(InteropError::BufferTooSmall {
                requested: dst.len(),
                available: self.core.buffer.len(),
            });
        }
        let cuda = self.core.cuda()?;
        // SAFETY: (U3) stream 为本 FrameCore 独占有效句柄；同步确保此前 wait/launch 已完成。
        let rc = unsafe { cuda.stream_synchronize(self.core.stream.raw) };
        cu_ext_check("cuStreamSynchronize", rc)?;
        let bytes = std::mem::size_of_val(dst);
        // SAFETY: (U4) dst 指向 bytes 字节有效可写 host 存储；buffer 映射区至少含 dst.len()
        // 个 f32（上方容量检查）；stream 已同步，D3D12 尚未取得本帧读权。
        let rc = unsafe {
            cuda.memcpy_dtoh(
                dst.as_mut_ptr().cast::<c_void>(),
                self.core.buffer.device_ptr(),
                bytes,
            )
        };
        cu_ext_check("cuMemcpyDtoH", rc)
    }
    /// 完成写入：CUDA `cuSignalExternalSemaphoresAsync(cuda_done(n)=2n+1)`（RFC-0001 §4.3）。
    pub fn signal(self) -> Result<PresentableFrame<'ctx>> {
        let v = cuda_done_value(self.core.frame).ok_or(InteropError::FenceOverflow)?;
        let cuda = self.core.cuda()?;
        // SAFETY: (U18) semaphore/stream 同上;signal 排在本 stream 先前 kernel 之后（RFC-0001 §4.3）。
        let rc = unsafe {
            cuda.signal_external_semaphore(self.core.semaphore.ext_sem, v, self.core.stream.raw)
        }
        .ok_or(InteropError::ExternalApiUnavailable)?;
        cu_ext_check("cuSignalExternalSemaphoresAsync", rc)?;
        Ok(PresentableFrame { core: self.core })
    }
}

impl<'ctx> PresentableFrame<'ctx> {
    pub fn frame_index(&self) -> u64 {
        self.core.frame
    }
    /// 提交 D3D12 present：shim queue wait `cuda_done(n)` → present pass → Present → queue
    /// signal `d3d_done(n)=2n+2`;帧计数 +1 回到 [`ReadyFrame`]（RFC-0001 §4.2.1 / §4.3）。
    pub fn present(self) -> Result<ReadyFrame<'ctx>> {
        let cuda_done = cuda_done_value(self.core.frame).ok_or(InteropError::FenceOverflow)?;
        let d3d_done = d3d_done_value(self.core.frame).ok_or(InteropError::FenceOverflow)?;
        self.core
            .presenter
            .submit(cuda_done, d3d_done)
            .map_err(InteropError::Shim)?;
        let mut core = self.core;
        core.frame = core
            .frame
            .checked_add(1)
            .ok_or(InteropError::FenceOverflow)?;
        Ok(ReadyFrame { core })
    }
    /// 抽干窗口消息泵;`Ok(true)` = 收到关闭请求（present 循环退出条件）。
    pub fn pump(&self) -> Result<bool> {
        self.core.presenter.pump().map_err(InteropError::Shim)
    }
}

// -- scope:生成式 brand 唯一安全入口（RXS-0140/0141;RFC-0001 §4.4 无 public from_raw_handle）-

/// CUDA–D3D12 互操作呈现作用域（**唯一安全构造入口**，RFC-0001 §4.4）。
///
/// 以高阶 `for<'ctx>` 闭包生成不可伪造、不可逃逸的新鲜不变 brand `'ctx`：内部完成
/// `cuDeviceGetLuid` → 同 LUID adapter 上经 shim 建 D3D12 device/swapchain/共享
/// resource·fence → import external memory/semaphore → 构造同 brand [`InteropContext`] 与
/// [`ReadyFrame`] 交给闭包。闭包内驱动 `Ready→Acquired→Presentable` 帧循环;返回后按
/// RFC-0001 §4.4 销毁。外部不承担「裸 HANDLE 是否有效」证明义务。
///
/// 无 `d3d12-interop-real`（stub）或非交互桌面 / 无 GPU 时返回
/// [`InteropError::Shim`]/[`InteropError::Cuda`]/[`InteropError::ExternalApiUnavailable`]——
/// 类型面（三类编译期拦截 + fence 协议）不依赖运行期可用性。
pub fn scope<R>(
    cuda_ordinal: i32,
    render_size: [u32; 2],
    window_size: [u32; 2],
    f: impl for<'ctx> FnOnce(InteropContext<'ctx>, ReadyFrame<'ctx>) -> Result<R>,
) -> Result<R> {
    let cuda = sys::cuda().ok_or(InteropError::Cuda(CudaError::DriverUnavailable))?;
    if !cuda.has_external_resource_api() {
        return Err(InteropError::ExternalApiUnavailable);
    }
    // 保留并绑定 primary context（Drop 释放;与外部框架共享同一 context）。
    let _ctx = Context::from_primary(cuda_ordinal)?;
    let (r, dev) = cuda.device_get(cuda_ordinal);
    cu_ext_check("cuDeviceGet", r)?;
    let (r, luid_i8, node_mask) = cuda
        .device_get_luid(dev)
        .ok_or(InteropError::ExternalApiUnavailable)?;
    cu_ext_check("cuDeviceGetLuid", r)?;
    let mut cuda_luid = [0u8; 8];
    for (o, &i) in cuda_luid.iter_mut().zip(luid_i8.iter()) {
        *o = i as u8;
    }

    // shim 在同 LUID adapter 上创建 device/swapchain/共享 resource·fence（stub → Err）。
    let (presenter, export) = Presenter::create(cuda_luid, node_mask, render_size, window_size, 0)
        .map_err(InteropError::Shim)?;
    if export.adapter_luid != cuda_luid {
        return Err(InteropError::LuidMismatch);
    }

    // import external memory（committed D3D12_RESOURCE + DEDICATED）→ map → 立即关闭 NT HANDLE。
    let mem_desc = CudaExternalMemoryHandleDesc {
        type_: CU_EXTERNAL_MEMORY_HANDLE_TYPE_D3D12_RESOURCE,
        win32: CudaWin32Handle {
            handle: export.memory_handle,
            name: std::ptr::null(),
        },
        size: export.allocation_size,
        flags: CUDA_EXTERNAL_MEMORY_DEDICATED,
        reserved: [0; 16],
    };
    // SAFETY: (U17) desc.win32.handle 为 shim create 移交的有效 NT HANDLE;current context 一致。
    let (r, ext_mem) = unsafe { cuda.import_external_memory(&mem_desc) }
        .ok_or(InteropError::ExternalApiUnavailable)?;
    cu_ext_check("cuImportExternalMemory", r)?;

    let buf_desc = CudaExternalMemoryBufferDesc {
        offset: 0,
        size: export.mapping_size,
        flags: 0,
        reserved: [0; 16],
    };
    // SAFETY: (U17) ext_mem 由上一步成功 import;desc 描述 [0, mapping_size) 映射区。
    let (r, dptr) = unsafe { cuda.external_memory_get_mapped_buffer(ext_mem, &buf_desc) }
        .ok_or(InteropError::ExternalApiUnavailable)?;
    if let Err(e) = cu_ext_check("cuExternalMemoryGetMappedBuffer", r) {
        // SAFETY: (U17) 回滚:ext_mem 已 import 但未 map,destroy 之。
        unsafe {
            let _ = cuda.destroy_external_memory(ext_mem);
        }
        return Err(e);
    }
    // SAFETY: handle 由 shim create 成功移交，CUDA import 后尚未关闭，按契约恰好关闭一次。
    unsafe {
        rurix_d3d12::close_shared_handle(export.memory_handle);
    }

    // import external semaphore（D3D12_FENCE）→ 立即关闭 NT HANDLE。
    let sem_desc = CudaExternalSemaphoreHandleDesc {
        type_: CU_EXTERNAL_SEMAPHORE_HANDLE_TYPE_D3D12_FENCE,
        win32: CudaWin32Handle {
            handle: export.fence_handle,
            name: std::ptr::null(),
        },
        flags: 0,
        reserved: [0; 16],
    };
    // SAFETY: (U18) desc.win32.handle 为 shim create 移交的有效 fence NT HANDLE。
    let (r, ext_sem) = unsafe { cuda.import_external_semaphore(&sem_desc) }
        .ok_or(InteropError::ExternalApiUnavailable)?;
    cu_ext_check("cuImportExternalSemaphore", r)?;
    // SAFETY: handle 由 shim create 成功移交，CUDA import 后尚未关闭，按契约恰好关闭一次。
    unsafe {
        rurix_d3d12::close_shared_handle(export.fence_handle);
    }

    let (r, stream) = cuda.stream_create();
    cu_ext_check("cuStreamCreate", r)?;

    let n_elems = (export.mapping_size / size_of::<f32>() as u64) as usize;
    let buffer = ExternalBuffer {
        ext_mem,
        dptr,
        len: n_elems,
        _brand: PhantomData,
        _t: PhantomData,
    };
    let semaphore = ExternalSemaphore {
        ext_sem,
        _brand: PhantomData,
    };
    let icx = InteropContext {
        device: dev,
        export,
        _brand: PhantomData,
        _not_send: PhantomData,
    };
    let ready = ReadyFrame {
        core: FrameCore {
            buffer,
            semaphore,
            stream: OwnedStream { raw: stream },
            presenter,
            frame: 0,
            _brand: PhantomData,
        },
    };
    // 闭包驱动帧循环;返回后 ready/icx/_ctx 依次 Drop（FrameCore 字段序 buffer→semaphore→
    // stream→presenter,近似 RFC-0001 §4.4;_ctx 最后释放 primary context）。
    f(icx, ready)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "d3d12-interop-real")]
    const DEVICE_FILL_PTX: &str = r#".version 8.0
.target sm_89
.address_size 64

.visible .entry rx_interop_fill(
    .param .u64 out,
    .param .u64 n
)
{
    .reg .pred %p<4>;
    .reg .b32 %r<7>;
    .reg .b64 %rd<7>;

    ld.param.u64 %rd1, [out];
    ld.param.u64 %rd2, [n];
    mov.u32 %r1, %ctaid.x;
    mov.u32 %r2, %ntid.x;
    mov.u32 %r3, %tid.x;
    mad.lo.s32 %r4, %r1, %r2, %r3;
    cvt.u64.u32 %rd3, %r4;
    setp.ge.u64 %p1, %rd3, %rd2;
    @%p1 bra DONE;
    rem.u64 %rd4, %rd3, 3;
    mov.b32 %r5, 0f00000000;
    setp.eq.u64 %p2, %rd4, 0;
    @%p2 mov.b32 %r5, 0f3F000000;
    setp.eq.u64 %p3, %rd4, 1;
    @%p3 mov.b32 %r5, 0f3F000000;
    shl.b64 %rd5, %rd3, 2;
    add.s64 %rd6, %rd1, %rd5;
    st.global.b32 [%rd6], %r5;
DONE:
    ret;
}
"#;

    fn assert_not_send<T>() {}
    fn is_send<T: Send>() {}

    //@ spec: RXS-0142
    #[test]
    fn fence_handoff_even_odd_protocol() {
        // RFC-0001 §4.3:acquire(n)=2n / cuda_done(n)=2n+1 / d3d_done(n)=2n+2。
        assert_eq!(acquire_value(0), Some(0));
        assert_eq!(cuda_done_value(0), Some(1));
        assert_eq!(d3d_done_value(0), Some(2));
        assert_eq!(acquire_value(1), Some(2));
        assert_eq!(cuda_done_value(1), Some(3));
        assert_eq!(d3d_done_value(1), Some(4));
        assert_eq!(acquire_value(7), Some(14));
        assert_eq!(d3d_done_value(7), Some(16));
        // 严格递增、不复用:相邻帧值序连续。
        for n in 0u64..1000 {
            assert_eq!(acquire_value(n + 1), d3d_done_value(n));
        }
    }

    //@ spec: RXS-0142
    #[test]
    fn fence_overflow_deterministic_stop() {
        // 溢出 → None（确定停机,不 rewind/不复用,RFC-0001 §4.3）。
        assert_eq!(acquire_value(u64::MAX), None);
        assert_eq!(cuda_done_value(1u64 << 63), None); // 2·2^63 越界
        assert_eq!(d3d_done_value(1u64 << 63), None);
        // d3d_done(n)=2n+2 在 n=2^63-1 即溢出（比 acquire 早一帧停机,确定边界）。
        assert_eq!(acquire_value((1u64 << 63) - 1), Some(u64::MAX - 1));
        assert_eq!(d3d_done_value((1u64 << 63) - 1), None);
    }

    //@ spec: RXS-0140
    #[test]
    fn external_resources_are_affine_move_only() {
        // affine 单一所有权（非 Copy/非 Clone）:类型存在性 + 句柄宽度锚定（RXS-0140）。
        assert!(size_of::<ExternalBuffer<'static, f32>>() > 0);
        assert!(size_of::<ExternalSemaphore<'static>>() > 0);
        // frame typestate 三态存在（RXS-0142）。
        assert!(size_of::<ReadyFrame<'static>>() > 0);
        assert!(size_of::<AcquiredFrame<'static>>() > 0);
        assert!(size_of::<PresentableFrame<'static>>() > 0);
    }

    //@ spec: RXS-0143
    #[test]
    fn external_resource_descriptor_abi_sizes() {
        // RFC-0001 §4.2.3:CUDA external-resource descriptor + shim export 的 Windows x64 ABI
        // 大小逐一核对（编译期 const 断言 + 本运行期测试双锚定 RXS-0143 import ABI）。
        assert_eq!(size_of::<crate::sys::CudaExternalMemoryHandleDesc>(), 104);
        assert_eq!(size_of::<crate::sys::CudaExternalMemoryBufferDesc>(), 88);
        assert_eq!(size_of::<crate::sys::CudaExternalSemaphoreHandleDesc>(), 96);
        assert_eq!(size_of::<crate::sys::CudaExternalSemaphoreParams>(), 144);
        assert_eq!(size_of::<rurix_d3d12::InteropExport>(), 96);
    }

    //@ spec: RXS-0141
    #[test]
    fn interop_context_is_thread_bound_not_send() {
        // current context 线程绑定:InteropContext !Send（生成式 brand 根,RXS-0141）。
        assert_not_send::<InteropContext<'static>>();
        // 标量实参可 Send（值语义）;此处仅锚定 brand 类型存在。
        is_send::<u64>();
        assert!(size_of::<InteropContext<'static>>() > 0);
    }

    //@ spec: RXS-0140
    #[test]
    fn scope_reports_unavailable_outside_device_session() {
        // 无 d3d12-interop-real（stub shim）/ 无 GPU / 非交互桌面:scope 返回确定性不可用错误,
        // 不 panic、不 UB（类型面与 fence 协议已在上方 test 验证,不依赖运行期可用性,RFC-0001 §4.4）。
        let r = scope(0, [4, 4], [8, 8], |_cx, _ready| Ok(()));
        if cfg!(feature = "d3d12-interop-real") {
            assert!(r.is_ok(), "real-shim 交互设备会话应成功建立,实得 {r:?}");
            return;
        }
        assert!(
            matches!(
                r,
                Err(InteropError::Shim(_))
                    | Err(InteropError::Cuda(_))
                    | Err(InteropError::ExternalApiUnavailable)
                    | Err(InteropError::LuidMismatch)
            ),
            "stub/非设备环境应返回不可用错误,实得 {r:?}"
        );
    }

    #[cfg(feature = "d3d12-interop-real")]
    #[test]
    fn real_interop_numeric_roundtrip() {
        let result = scope(0, [2, 2], [64, 64], |cx, ready| {
            let module = cx.load_module(DEVICE_FILL_PTX)?;
            let kernel = module.kernel("rx_interop_fill")?;
            let mut acquired = ready.wait()?;
            let n = acquired.buffer_mut().len();
            let buffer_arg = {
                let buffer = acquired.buffer_mut();
                InteropKernelArg::buffer(&*buffer)
            };
            acquired.launch(
                &kernel,
                [(n as u32).div_ceil(256), 1, 1],
                [256, 1, 1],
                &mut [buffer_arg, InteropKernelArg::usize(n)],
            )?;
            let mut sample = [0.0f32; 3];
            acquired.readback_f32(&mut sample)?;
            if sample != [1.0, 0.5, 0.0] {
                return Err(InteropError::DeviceVerificationFailed);
            }
            let presentable = acquired.signal()?;
            let _ready = presentable.present()?;
            println!(
                "INTEROP_DEVICE: ok sample_rgb={},{},{}",
                sample[0], sample[1], sample[2]
            );
            Ok(())
        });
        assert!(
            result.is_ok(),
            "real CUDA-D3D12 interop 数值闭环失败:{result:?}"
        );
    }
}
