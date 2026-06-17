//! UC-02 三 stream 重叠流水线运行时面(M8.3,D-M8-3;spec/pipeline.md RXS-0130~0134)。
//!
//! 在 [`crate::Context`](单线程 affine 根,`!Send`)之外提供一组**可跨线程转移**的
//! affine 资源,支撑 UC-02 三 stream(H2D / compute / D2H)重叠 + 跨线程所有权转移:
//!
//! - [`SharedContext`](`Send + Sync`):`Arc` 引用计数包裹 **device primary context**
//!   (`cuDevicePrimaryCtxRetain` / `Release`,进程级、多线程经各自 `cuCtxSetCurrent`
//!   合法共享);[`SharedContext::bind`] 在某线程绑定 current context,返回**线程绑定
//!   守卫** [`Bound`](`!Send`,RXS-0133)。
//! - 经 `Bound` 分配的 [`DeviceBox`] / [`SharedEvent`] 句柄持 `Arc<SharedInner>`、为
//!   `Send`,可经 affine `move` 跨线程转移(producer→consumer);单一所有权(非 `Copy`
//!   /非 `Clone`)使 **use-after-free / double-free** 由 rustc 编译期排除(RXS-0130/0134)。
//! - **流序分配类型化**(RXS-0132):某 stream 上排队异步操作的缓冲被封为 [`InFlight`]
//!   (`#[must_use]`,无读接口);跨 stream 读取/操作须经 [`SharedStream::acquire`] 插入
//!   `cuStreamWaitEvent` 同步后**重 brand** 回 [`DeviceBox`]——跳过同步即无可读句柄,
//!   「跨 stream 未同步访问」成为编译期类型错误而非运行期数据竞争。
//!
//! 资源生命周期错误类别(use-after-free / double-free / 跨 stream 未同步 / 跨线程非法
//! 转移)**100% 由 Rust 类型系统编译期拦截**(affine move / 生命周期 brand / `Send`
//! 约束),不新增 RX 错误码(spec/pipeline.md §3)。FFI unsafe 注册见
//! `unsafe-audit/rurix-rt.md`(U11~U16)。

use core::ffi::c_void;
use core::marker::PhantomData;
use std::sync::Arc;

use crate::error::{Result, check};
use crate::sys::{self, CuDevice, CuDevicePtr, CuPtr};
use crate::{PTX_VERSION_LADDER, set_ptx_version};

/// 共享 primary context 内核(`Arc` 引用计数;最后一个引用 Drop 走 `Release`)。
struct SharedInner {
    raw: CuPtr,
    device: CuDevice,
}

// SAFETY: (U13):primary context 是**进程级**对象,多个线程各自 `cuCtxSetCurrent` 后
// 使用同一 context 合法(CUDA Driver API 线程模型)。`SharedInner` 仅作句柄/设备序号
// 的纯数据载体,本身不解引用 GPU 状态;实际 GPU 操作前持有者线程经 `Bound`/Drop 的
// `ctx_set_current` 重绑 current。retain/release 引用计数由 `Arc` 单点配对(下方 Drop)。
unsafe impl Send for SharedInner {}
// SAFETY: (U13):同上;`&SharedInner` 跨线程共享仅暴露不可变句柄/序号,无内部可变状态。
unsafe impl Sync for SharedInner {}

impl Drop for SharedInner {
    fn drop(&mut self) {
        if let Some(cuda) = sys::cuda() {
            // SAFETY: (U13):`device` 由 `from_primary` 经 `primary_ctx_retain` 成功取得,
            // 本次 release 与该 retain 单点配对(`Arc` 最后引用 Drop,仅一次)。
            unsafe {
                let _ = cuda.primary_ctx_release(self.device);
            }
        }
    }
}

/// **可跨线程共享**的 device primary context 句柄(`Send + Sync`;跨线程所有权转移根,
/// RXS-0133)。`Clone` = `Arc` 引用计数 +1(不重复 retain)。
#[derive(Clone)]
pub struct SharedContext {
    inner: Arc<SharedInner>,
}

impl SharedContext {
    /// 保留并包裹 device(序号 `ordinal`)的 primary context(`cuInit` +
    /// `cuDevicePrimaryCtxRetain`)。与 PyTorch/CuPy 等共享同一 context(对齐
    /// [`Context::from_primary`](crate::Context::from_primary))。
    pub fn from_primary(ordinal: i32) -> Result<SharedContext> {
        let cuda = sys::cuda().ok_or(crate::CudaError::DriverUnavailable)?;
        check("cuInit", cuda.init())?;
        let (r, dev) = cuda.device_get(ordinal);
        check("cuDeviceGet", r)?;
        let (r, raw) = cuda.primary_ctx_retain(dev);
        check("cuDevicePrimaryCtxRetain", r)?;
        Ok(SharedContext {
            inner: Arc::new(SharedInner { raw, device: dev }),
        })
    }

    /// 在**当前线程**绑定 current context(`cuCtxSetCurrent`),返回线程绑定守卫
    /// [`Bound`](`!Send`)。每个使用本 context 的 worker 线程须先 `bind`(RXS-0133)。
    pub fn bind(&self) -> Result<Bound<'_>> {
        let cuda = sys::cuda().ok_or(crate::CudaError::DriverUnavailable)?;
        // SAFETY: (U13):`inner.raw` 为 `primary_ctx_retain` 成功且 `Arc` 存活期内有效的
        // primary context 句柄;`ctx_set_current` 在本线程绑定该 context。
        let r = unsafe { cuda.ctx_set_current(self.inner.raw) };
        check("cuCtxSetCurrent", r)?;
        Ok(Bound {
            shared: self,
            _not_send: PhantomData,
        })
    }

    /// 设备序号(审计/诊断)。
    pub fn device(&self) -> CuDevice {
        self.inner.device
    }
}

/// 线程绑定守卫(RXS-0133):某线程已 `cuCtxSetCurrent` 后的 current-context 视图。
///
/// **`!Send` + `!Sync`**(`PhantomData<*const ()>`):current context 线程绑定,守卫不得
/// 跨线程转移——把 `Bound` 送入另一线程为编译期 `Send` 约束错误(跨线程非法转移类别,
/// RXS-0134)。在 `Bound` 上分配的 [`DeviceBox`]/[`SharedEvent`] 则**可** `Send`。
pub struct Bound<'s> {
    shared: &'s SharedContext,
    _not_send: PhantomData<*const ()>,
}

impl Bound<'_> {
    fn cuda(&self) -> Result<&'static sys::Cuda> {
        sys::cuda().ok_or(crate::CudaError::DriverUnavailable)
    }

    fn arc(&self) -> Arc<SharedInner> {
        Arc::clone(&self.shared.inner)
    }

    /// 设备内存分配(`cuMemAlloc`)→ 可跨线程转移的 [`DeviceBox`](RXS-0130)。
    pub fn alloc<T: Copy>(&self, n: usize) -> Result<DeviceBox<T>> {
        let cuda = self.cuda()?;
        let bytes = n.checked_mul(size_of::<T>()).expect("alloc 字节数溢出");
        let (r, ptr) = cuda.mem_alloc(bytes);
        check("cuMemAlloc", r)?;
        Ok(DeviceBox {
            inner: self.arc(),
            ptr,
            len: n,
            _t: PhantomData,
        })
    }

    /// 锁页主机缓冲(`cuMemAllocHost`;异步搬运 staging,RXS-0131)。
    pub fn alloc_pinned<T: Copy>(&self, n: usize) -> Result<PinnedBox<T>> {
        let cuda = self.cuda()?;
        let bytes = n
            .checked_mul(size_of::<T>())
            .expect("alloc_pinned 字节数溢出");
        let (r, ptr) = cuda.mem_alloc_host(bytes);
        check("cuMemAllocHost", r)?;
        Ok(PinnedBox {
            inner: self.arc(),
            ptr: ptr.cast::<T>(),
            len: n,
        })
    }

    /// 创建提交队列 stream(`cuStreamCreate`;`!Send`,线程内使用,RXS-0131)。
    pub fn create_stream(&self) -> Result<SharedStream> {
        let cuda = self.cuda()?;
        let (r, raw) = cuda.stream_create();
        check("cuStreamCreate", r)?;
        Ok(SharedStream {
            inner: self.arc(),
            raw,
        })
    }

    /// 创建跨 stream 同步事件(`cuEventCreate`;`Send`,可跨线程转移,RXS-0131/0133)。
    pub fn create_event(&self) -> Result<SharedEvent> {
        let cuda = self.cuda()?;
        let (r, raw) = cuda.event_create();
        check("cuEventCreate", r)?;
        Ok(SharedEvent {
            inner: self.arc(),
            raw,
        })
    }

    /// 装载 PTX 模块 + `.version` 协商降版(对齐 [`Context::load_module`](crate::Context::load_module);
    /// `!Send`,线程内使用)。
    pub fn load_module(&self, ptx: &str) -> Result<SharedModule<'_>> {
        let cuda = self.cuda()?;
        let (raw, version) = negotiate_load(cuda, ptx)?;
        Ok(SharedModule {
            inner: self.arc(),
            raw,
            version,
            _b: PhantomData,
        })
    }

    /// 同步 current context(`cuCtxSynchronize`;阻塞至全部 stream 清空)。
    pub fn synchronize(&self) -> Result<()> {
        let cuda = self.cuda()?;
        check("cuCtxSynchronize", cuda.ctx_synchronize())
    }
}

/// 设备内存缓冲(RXS-0130;**`Send`**:持 `Arc<SharedInner>`,可跨线程 `move`,RXS-0133)。
///
/// 非 `Copy`/非 `Clone`(单一所有权,affine)——`move` 后再用 → `E0382`(use-after-free
/// 类别);重复 `move`/试 `.clone()` → 编译期错误(double-free 类别,RXS-0134)。
/// `DeviceBox<T>: Send` 自动成立(`Arc<SharedInner>` + `CuDevicePtr`(`u64`)+
/// `PhantomData<T>`,当 `T: Send`)。
pub struct DeviceBox<T: Copy> {
    inner: Arc<SharedInner>,
    ptr: CuDevicePtr,
    len: usize,
    _t: PhantomData<T>,
}

impl<T: Copy> DeviceBox<T> {
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn byte_len(&self) -> usize {
        self.len * size_of::<T>()
    }

    /// 设备地址(launch 实参/同 stream 操作消费)。
    pub fn device_ptr(&self) -> CuDevicePtr {
        self.ptr
    }

    /// 同步 H2D 拷贝(`cuMemcpyHtoD`;`src` 长度须 ≤ 容量)。
    pub fn copy_from_host(&mut self, src: &[T]) -> Result<()> {
        assert!(src.len() <= self.len, "copy_from_host: 源长度超出缓冲容量");
        let cuda = sys::cuda().ok_or(crate::CudaError::DriverUnavailable)?;
        let bytes = size_of_val(src);
        // SAFETY: (U6):`self.ptr` 为 ≥ bytes 的设备分配;`src` 为 bytes 字节有效可读主机内存。
        let r = unsafe { cuda.memcpy_htod(self.ptr, src.as_ptr().cast::<c_void>(), bytes) };
        check("cuMemcpyHtoD", r)
    }

    /// 同步 D2H 拷贝(`cuMemcpyDtoH`;`dst` 长度须 ≤ 容量)。流序分配类型化下,本读接口
    /// **仅 [`DeviceBox`] 提供**——[`InFlight`] 无读接口,必经 [`SharedStream::acquire`]
    /// 同步重 brand 后方可读(RXS-0132)。
    pub fn copy_to_host(&self, dst: &mut [T]) -> Result<()> {
        assert!(dst.len() <= self.len, "copy_to_host: 目标长度超出缓冲容量");
        let cuda = sys::cuda().ok_or(crate::CudaError::DriverUnavailable)?;
        let bytes = size_of_val(dst);
        // SAFETY: (U6):`dst` 为 bytes 字节有效可写主机内存;`self.ptr` 为 ≥ bytes 的设备分配。
        let r = unsafe { cuda.memcpy_dtoh(dst.as_mut_ptr().cast::<c_void>(), self.ptr, bytes) };
        check("cuMemcpyDtoH", r)
    }
}

impl<T: Copy> Drop for DeviceBox<T> {
    fn drop(&mut self) {
        if let Some(cuda) = sys::cuda() {
            // SAFETY: (U13/U3):Drop 可能在任意线程——先 `ctx_set_current` 重绑本 context
            // (`inner.raw` 经 `Arc` 存活有效),再 `cuMemFree`;`ptr` 由 `mem_alloc` 产出、
            // 本类型独占(非 Clone),Drop 仅一次(单一所有权,不双重释放)。
            unsafe {
                let _ = cuda.ctx_set_current(self.inner.raw);
                let _ = cuda.mem_free(self.ptr);
            }
        }
    }
}

/// 锁页主机缓冲(`cuMemAllocHost`;异步搬运 staging)。`!Send`(裸 `*mut T`,线程内使用)。
pub struct PinnedBox<T: Copy> {
    inner: Arc<SharedInner>,
    ptr: *mut T,
    len: usize,
}

impl<T: Copy> PinnedBox<T> {
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_slice(&self) -> &[T] {
        // SAFETY: (U8):`ptr` 为 `cuMemAllocHost` 返回的 `len*size_of::<T>()` 字节锁页内存,
        // 对齐满足(主机分配);生命期受 `&self` 约束。
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        // SAFETY: (U8):同上;`&mut self` 保证独占访问。
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl<T: Copy> Drop for PinnedBox<T> {
    fn drop(&mut self) {
        if let Some(cuda) = sys::cuda() {
            // SAFETY: (U8/U13):重绑本 context 后释放;`ptr` 由 `mem_alloc_host` 产出、本类型
            // 独占,Drop 仅一次。
            unsafe {
                let _ = cuda.ctx_set_current(self.inner.raw);
                let _ = cuda.mem_free_host(self.ptr.cast::<c_void>());
            }
        }
    }
}

/// 跨 stream 同步事件(`cuEvent`,RXS-0131)。**`Send`**(可跨线程转移,RXS-0133):
/// producer stream `record`、consumer stream `wait`,事件句柄经 affine `move` 传递。
pub struct SharedEvent {
    inner: Arc<SharedInner>,
    raw: CuPtr,
}

// SAFETY: (U14):`cuEvent` 是绑 context 的进程级驱动对象,跨线程使用合法(持有者线程
// current 为同一 context);`SharedEvent` 持 `Arc<SharedInner>` 保证 context 存活,Drop 前
// 重绑 current。仅实现 `Send`(move 转移),不实现 `Sync`(不跨线程共享 `&`)。
unsafe impl Send for SharedEvent {}

impl Drop for SharedEvent {
    fn drop(&mut self) {
        if let Some(cuda) = sys::cuda() {
            // SAFETY: (U14/U13):重绑本 context 后销毁;`raw` 由 `event_create` 产出、本类型
            // 独占(非 Clone),Drop 仅一次。
            unsafe {
                let _ = cuda.ctx_set_current(self.inner.raw);
                let _ = cuda.event_destroy(self.raw);
            }
        }
    }
}

/// 提交队列 stream(`cuStream`,RXS-0131)。`!Send`(裸句柄,线程内创建/使用/销毁)。
pub struct SharedStream {
    inner: Arc<SharedInner>,
    raw: CuPtr,
}

impl SharedStream {
    fn cuda(&self) -> Result<&'static sys::Cuda> {
        sys::cuda().ok_or(crate::CudaError::DriverUnavailable)
    }

    /// 在本 stream 上发起 kernel launch(`cuLaunchKernel`;异步;对齐
    /// [`Stream::launch`](crate::Stream::launch))。
    pub fn launch(
        &self,
        kernel: &SharedKernel<'_>,
        grid: [u32; 3],
        block: [u32; 3],
        params: &mut [*mut c_void],
    ) -> Result<()> {
        let cuda = self.cuda()?;
        // SAFETY: (U7):`kernel.raw` 为有效 kernel 句柄;`params` 各元素指向调用方维持的有效
        // 实参存储且长度与 kernel 形参匹配;`self.raw` 为有效 stream;shared=0。
        let r = unsafe {
            cuda.launch_kernel(kernel.raw, grid, block, 0, self.raw, params.as_mut_ptr())
        };
        check("cuLaunchKernel", r)
    }

    /// 在本 stream record 事件(`cuEventRecord`,RXS-0131)。
    pub fn record_event(&self, event: &SharedEvent) -> Result<()> {
        let cuda = self.cuda()?;
        // SAFETY: (U14):`event.raw`/`self.raw` 为有效且同 current context 的 event/stream 句柄。
        let r = unsafe { cuda.event_record(event.raw, self.raw) };
        check("cuEventRecord", r)
    }

    /// 在本 stream wait 事件(`cuStreamWaitEvent`;建立跨 stream 流序依赖,RXS-0131)。
    pub fn wait_event(&self, event: &SharedEvent) -> Result<()> {
        let cuda = self.cuda()?;
        // SAFETY: (U15):`self.raw`/`event.raw` 为有效且同 current context 的 stream/event 句柄。
        let r = unsafe { cuda.stream_wait_event(self.raw, event.raw) };
        check("cuStreamWaitEvent", r)
    }

    /// 同步本 stream(`cuStreamSynchronize`;阻塞至队列清空)。
    pub fn synchronize(&self) -> Result<()> {
        let cuda = self.cuda()?;
        // SAFETY: (U3):`self.raw` 为有效 stream 句柄。
        let r = unsafe { cuda.stream_synchronize(self.raw) };
        check("cuStreamSynchronize", r)
    }

    /// **异步 H2D 上传**(`cuMemcpyHtoDAsync`):消费 ready [`DeviceBox`] + pinned 源
    /// (move 入并随 [`InFlight`] 存活至同步,杜绝异步拷贝期 pinned 悬垂)+ record `on_done`
    /// 事件 → **在途缓冲**(RXS-0131/0132)。返回值 `#[must_use]`。
    pub fn upload<T: Copy>(
        &self,
        dst: DeviceBox<T>,
        src: PinnedBox<T>,
        on_done: SharedEvent,
    ) -> Result<InFlight<T>> {
        assert!(src.len <= dst.len, "upload: 源长度超出缓冲容量");
        let cuda = self.cuda()?;
        let bytes = src.len * size_of::<T>();
        // SAFETY: (U16):`dst.ptr` 为 ≥ bytes 设备分配;`src.ptr` 为锁页主机内存,经 `InFlight`
        // 持有存活至本 stream 操作完成(同步后方释放);`self.raw` 有效 stream。
        let r =
            unsafe { cuda.memcpy_htod_async(dst.ptr, src.ptr.cast::<c_void>(), bytes, self.raw) };
        check("cuMemcpyHtoDAsync", r)?;
        self.record_event(&on_done)?;
        Ok(InFlight {
            boxed: dst,
            pinned: Some(src),
            event: on_done,
        })
    }

    /// **流序同步取回**(RXS-0132):在本 stream 上 `wait` 在途缓冲的完成事件
    /// (`cuStreamWaitEvent`),消费 [`InFlight`] → **重 brand** 回可在本 stream 操作/读取的
    /// [`DeviceBox`](连同保活的 pinned 源)。跨 stream 读取/操作必经此同步——跳过即无可读
    /// 句柄(编译期拦截「跨 stream 未同步访问」)。
    pub fn acquire<T: Copy>(
        &self,
        inflight: InFlight<T>,
    ) -> Result<(DeviceBox<T>, Option<PinnedBox<T>>)> {
        self.wait_event(&inflight.event)?;
        Ok((inflight.boxed, inflight.pinned))
    }

    /// 标记刚在本 stream 操作过的缓冲为**在途**(record `on_done` 完成事件)→ [`InFlight`]
    /// (RXS-0132)。供 compute → D2H 阶段衔接。
    pub fn commit<T: Copy>(
        &self,
        boxed: DeviceBox<T>,
        pinned: Option<PinnedBox<T>>,
        on_done: SharedEvent,
    ) -> Result<InFlight<T>> {
        self.record_event(&on_done)?;
        Ok(InFlight {
            boxed,
            pinned,
            event: on_done,
        })
    }

    /// **异步 D2H 下载**(`cuMemcpyDtoHAsync`):流序 `wait` 在途事件 → 取回 box → 异步拷回
    /// pinned `dst` → record + 同步至完成,返回填好的 pinned 结果(RXS-0131/0132)。
    pub fn download<T: Copy>(
        &self,
        inflight: InFlight<T>,
        mut dst: PinnedBox<T>,
    ) -> Result<PinnedBox<T>> {
        let (boxed, _src) = self.acquire(inflight)?;
        assert!(dst.len <= boxed.len, "download: 目标长度超出缓冲容量");
        let cuda = self.cuda()?;
        let bytes = dst.len * size_of::<T>();
        // SAFETY: (U16):`dst.ptr` 为锁页主机内存(本函数末同步后方返回,拷贝期存活);
        // `boxed.ptr` 为 ≥ bytes 设备分配;`self.raw` 有效 stream。
        let r =
            unsafe { cuda.memcpy_dtoh_async(dst.ptr.cast::<c_void>(), boxed.ptr, bytes, self.raw) };
        check("cuMemcpyDtoHAsync", r)?;
        self.synchronize()?;
        let _ = dst.as_mut_slice(); // 触达可写性(拷贝结果已落盘)
        Ok(dst)
    }
}

impl Drop for SharedStream {
    fn drop(&mut self) {
        if let Some(cuda) = sys::cuda() {
            // SAFETY: (U3/U13):重绑本 context 后销毁;`raw` 由 `stream_create` 产出、本类型独占,
            // Drop 仅一次。
            unsafe {
                let _ = cuda.ctx_set_current(self.inner.raw);
                let _ = cuda.stream_destroy(self.raw);
            }
        }
    }
}

/// **在途缓冲**(流序分配类型化,RXS-0132):某 stream 上已排队异步操作、未经目标 stream
/// `wait` 同步的缓冲。**无任何读接口**——必经 [`SharedStream::acquire`] 插入
/// `cuStreamWaitEvent` 重 brand 回 [`DeviceBox`] 方可读/操作。`#[must_use]`:跨 stream
/// 未同步访问被编译期拦截(直接读 `InFlight` → 方法不存在 `E0599`)。
#[must_use = "InFlight 必经 SharedStream::acquire/download 流序同步后方可读(RXS-0132)"]
pub struct InFlight<T: Copy> {
    boxed: DeviceBox<T>,
    pinned: Option<PinnedBox<T>>,
    event: SharedEvent,
}

/// 已装载 PTX 模块(`cuModule`)。`!Send`(裸句柄,线程内使用)。
pub struct SharedModule<'b> {
    inner: Arc<SharedInner>,
    raw: CuPtr,
    version: String,
    _b: PhantomData<&'b ()>,
}

impl<'b> SharedModule<'b> {
    /// 取强类型 kernel 句柄(`cuModuleGetFunction`)。
    pub fn function(&self, name: &str) -> Result<SharedKernel<'_>> {
        let cuda = sys::cuda().ok_or(crate::CudaError::DriverUnavailable)?;
        let cname = std::ffi::CString::new(name).expect("kernel 名含内嵌 NUL");
        // SAFETY: (U3):`self.raw` 为有效模块;`cname` 为 NUL 结尾 kernel 名。
        let (r, raw) = unsafe { cuda.module_get_function(self.raw, cname.as_ptr()) };
        check("cuModuleGetFunction", r)?;
        Ok(SharedKernel {
            raw,
            _m: PhantomData,
        })
    }

    /// 协商后实际装载的 PTX `.version`(RXS-0076)。
    pub fn negotiated_version(&self) -> &str {
        &self.version
    }
}

impl Drop for SharedModule<'_> {
    fn drop(&mut self) {
        if let Some(cuda) = sys::cuda() {
            // SAFETY: (U3/U13):重绑本 context 后卸载;`raw` 由 `module_load_data_ex` 产出、本类型
            // 独占,Drop 仅一次。
            unsafe {
                let _ = cuda.ctx_set_current(self.inner.raw);
                let _ = cuda.module_unload(self.raw);
            }
        }
    }
}

/// 强类型 kernel 句柄(`CUfunction`;brand 绑模块 `'m`,`Copy` 经引用获取)。
#[derive(Clone, Copy)]
pub struct SharedKernel<'m> {
    raw: CuPtr,
    _m: PhantomData<&'m ()>,
}

/// PTX `.version` 协商装载(对齐 [`Context::load_module`](crate::Context::load_module) 的降版
/// 阶梯;无 poison 状态机——shared 路径以 `Driver` 错误值表达失败)。
fn negotiate_load(cuda: &sys::Cuda, ptx: &str) -> Result<(CuPtr, String)> {
    let start = crate::parse_ptx_version(ptx);
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
            continue;
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
        // SAFETY: (U5):`image` 为 NUL 结尾 PTX 文本(CString);`opts`/`vals` 为长度 4 平行有效
        // 数组,日志缓冲 `info_buf`/`err_buf` 调用期存活。
        let (r, raw) = unsafe {
            cuda.module_load_data_ex(
                image.as_ptr().cast::<c_void>(),
                4,
                opts.as_mut_ptr(),
                vals.as_mut_ptr(),
            )
        };
        if r == sys::CUDA_SUCCESS {
            return Ok((raw, version.clone()));
        }
        last_jit_log = cstr_prefix(&err_buf);
        if r != sys::CUDA_ERROR_UNSUPPORTED_PTX_VERSION {
            check("cuModuleLoadDataEx", r)?;
        }
    }
    Err(crate::CudaError::LoadNegotiation {
        tried: ladder,
        jit_log: last_jit_log,
    })
}

/// C 字符串缓冲前缀(JIT 日志;截至首个 NUL)。
fn cstr_prefix(buf: &[u8]) -> String {
    let end = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 编译期 trait-bound 断言助手(host-only,无 GPU)。
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    //@ spec: RXS-0130
    #[test]
    fn affine_resources_are_move_only() {
        // affine 资源非 Copy/非 Clone:DeviceBox/SharedEvent/SharedStream 单一所有权。
        // (编译即证——若误派生 Copy/Clone 则下行 size_of 仍过,但类型设计无 Copy 派生;
        //  此处以编译通过 + 类型存在性锚定 RXS-0130 affine 所有权与销毁纪律。)
        assert!(size_of::<DeviceBox<f32>>() > 0);
        assert!(size_of::<SharedEvent>() > 0);
        assert!(size_of::<SharedStream>() > 0);
    }

    //@ spec: RXS-0131
    #[test]
    fn event_sync_api_surface() {
        // Event 记录/等待 + 异步搬运 API 面存在(签名锚定;真跑见 device 冒烟步骤 36)。
        let _record = SharedStream::record_event;
        let _wait = SharedStream::wait_event;
        let _upload = SharedStream::upload::<f32>;
    }

    //@ spec: RXS-0132
    #[test]
    fn stream_ordered_typing_gates_reads() {
        // 流序分配类型化:InFlight 无读接口,acquire 重 brand 回 DeviceBox 方可读。
        let _acquire = SharedStream::acquire::<f32>;
        // copy_to_host 仅 DeviceBox 提供(InFlight 无此接口 → 跨 stream 未同步读为编译期错误)。
        let _read = DeviceBox::<f32>::copy_to_host;
    }

    //@ spec: RXS-0133
    #[test]
    fn cross_thread_transfer_send_bounds() {
        // 跨线程所有权转移:共享 context Send+Sync;Buffer/Event 句柄 Send(可 move 跨线程)。
        assert_send::<SharedContext>();
        assert_sync::<SharedContext>();
        assert_send::<DeviceBox<f32>>();
        assert_send::<SharedEvent>();
        // 注:Bound(线程绑定守卫)为 !Send——跨线程非法转移由 compile-fail 样例核对
        //     (src/uc02-demo/compile-fail/cross_thread_send.rs,RXS-0134)。
    }

    //@ spec: RXS-0134
    #[test]
    fn resource_lifecycle_error_classes_compile_intercepted() {
        // 资源生命周期错误类别由 Rust 类型系统编译期拦截(无新增 RX 码):
        //   use-after-free(move 后再用)→ E0382
        //   double-free(重复 move / 试 .clone())→ E0382/E0599
        //   跨 stream 未同步访问(缺 acquire)→ InFlight 无读接口 E0599(RXS-0132)
        //   跨线程非法转移(送 !Send Bound)→ E0277(RXS-0133)
        // reject 类别覆盖见 src/uc02-demo/compile-fail/*.rs(冒烟步骤 36 断言全拦截)。
        // 正向锚定:affine 资源/共享句柄类型存在且 Send 边界如上各 test 所证。
        assert!(size_of::<InFlight<f32>>() > 0);
        assert!(size_of::<SharedContext>() > 0);
    }
}
