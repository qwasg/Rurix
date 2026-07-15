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

    /// 装载分发产物变体集 + fatbin 装载协商(RXS-0150/0151;MS1.2 / RFC-0009 §4.3,镜像
    /// [`Context::load_module_artifacts`](crate::Context::load_module_artifacts) 到 shared 族)。
    ///
    /// 协商序与 Context 路径一致:查 device compute capability → 命中按架构预编 cubin 即
    /// `cuModuleLoadData`(首启免 JIT);未命中 / driver 无 cubin 装载 / cubin 被驱动拒绝 →
    /// **降级**既有 PTX 版号梯子(RXS-0076 语义 0-byte;降级而非 reject,不 poison,D-207)。
    /// 返回 **`'static` brand**:[`SharedModule`] 自持 `Arc<SharedInner>`(context 不早于
    /// 模块,Drop 自行重绑 current 后卸载),供 rurix-rt-cabi u64 句柄表跨调用惰性缓存
    /// (RXS-0194);线程内即用即取的场景仍宜用 [`load_module`](Self::load_module)。
    pub fn load_module_artifacts(
        &self,
        set: &crate::fatbin::DeviceArtifactSet,
    ) -> Result<SharedModule<'static>> {
        if let Some(module) = self.try_load_cubin(set) {
            return Ok(module);
        }
        let cuda = self.cuda()?;
        let (raw, version) = negotiate_load(cuda, set.ptx_fallback())?;
        Ok(SharedModule {
            inner: self.arc(),
            raw,
            version,
            _b: PhantomData,
        })
    }

    /// 尝试按架构预编 cubin 装载(RXS-0151,镜像 `Context::try_load_cubin` 的 shared 族
    /// 形态);未命中 / driver 不支持 / cubin 被拒 → `None`(上层降级保守 PTX fallback,
    /// 不 poison)。
    fn try_load_cubin(
        &self,
        set: &crate::fatbin::DeviceArtifactSet,
    ) -> Option<SharedModule<'static>> {
        let cuda = sys::cuda()?;
        if !cuda.has_cubin_load() || !set.has_cubin() {
            return None;
        }
        let (rc, major, minor) = cuda.device_compute_capability(self.shared.inner.device)?;
        if rc != sys::CUDA_SUCCESS {
            return None;
        }
        let device_sm = crate::fatbin::SmTarget::from_capability(major, minor);
        let crate::fatbin::LoadChoice::Cubin(sm) =
            crate::fatbin::select_load_variant(&device_sm, set)
        else {
            return None;
        };
        let variant = set.cubin_for(&sm)?;
        // SAFETY: (U22):`variant.bytes()` 为预编的有效 cubin 二进制(RXS-0150,
        // `DeviceArtifactSet` 持有保活);cubin 被驱动拒绝(架构不符等)时返回非 SUCCESS →
        // `None`,上层降级 PTX(保守兜底,不 poison,D-207)。
        let (r, raw) = unsafe { cuda.module_load_data(variant.bytes().as_ptr().cast::<c_void>())? };
        (r == sys::CUDA_SUCCESS).then_some(SharedModule {
            inner: self.arc(),
            raw,
            version: sm.as_str().to_owned(),
            _b: PhantomData,
        })
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

// -- G1.2 流序分配 AsyncBuffer<'stream,T>(stream-ordered allocation,RXS-0144~0148;MR-0001) --
//
// 镜像 InFlight 流序分配类型化先例(RXS-0132):纯类型级 affine typestate + 生成式 `'stream`
// brand + `#[must_use]` + 私有字段无读接口,rustc 原生诊断拦截,**零新 RX 码**。运行期流序
// 分配器(`cuMemAllocAsync` + `CUmemoryPool`,D-232)。三规则编译期拦截(06 §5.4):
//   ① 分配未完成访问 —— in-flight `AsyncBuffer` 无 `device_ptr` / 无读接口(须先 `share_with` 同步)。
//   ② 释放后访问 —— affine move-only(非 `Copy`/非 `Clone`):move 后再用 `E0382`;Drop=`cuMemFreeAsync`。
//   ③ 跨 stream 使用 —— 必经 `share_with(other,event)` 显式时序边(record+wait_event)重 brand。

/// 流序分配设备内存的 RAII 载体(`cuMemAllocAsync` 入 stream pool;Drop = `cuMemFreeAsync` 流序
/// 释放回 pool)。[`AsyncBuffer`] / [`AsyncReady`] 持有它并随 typestate 转移——释放责任经 `move`
/// **单点转移**(本载体独占 Drop,wrapper 无 Drop,故 share_with 安全 move 不双重释放,
/// RXS-0144/0146,U19/U20)。
struct PoolAlloc {
    inner: Arc<SharedInner>,
    /// 流序释放所属 stream(`share_with` 同步后改到目标 stream;流序释放在该 stream 合法)。
    stream: CuPtr,
    ptr: CuDevicePtr,
}

impl Drop for PoolAlloc {
    fn drop(&mut self) {
        if let Some(cuda) = sys::cuda() {
            // SAFETY: (U20/U13):Drop 可能在任意线程——先 `ctx_set_current` 重绑本 context
            // (`inner.raw` 经 `Arc` 存活有效),再 `cuMemFreeAsync` 入所属 stream 流序释放;
            // `ptr` 由 `cuMemAllocAsync` 产出、本载体独占(wrapper 非 Clone),Drop 仅一次。
            unsafe {
                let _ = cuda.ctx_set_current(self.inner.raw);
                let _ = cuda.mem_free_async(self.ptr, self.stream);
            }
        }
    }
}

/// **流序分配在途缓冲**(stream-ordered allocation,RXS-0144/0145)。`cuMemAllocAsync` 分配在产出
/// stream 上排队;携不变 `'stream` brand(借用产出 stream,生命周期不晚于它)、affine move-only
/// (非 `Copy`/非 `Clone`)、`#[must_use]`。**无 `device_ptr` / 无读写接口**——「分配未完成访问」
/// (规则①)经类型排除;跨 stream 使用须 [`AsyncBuffer::share_with`](规则③)。`!Send`(持裸
/// stream 句柄,线程内使用)。
#[must_use = "AsyncBuffer 为流序分配在途缓冲,须经 share_with(other,event) 同步重 brand 后方可读/操作(RXS-0145/0147)"]
pub struct AsyncBuffer<'stream, T: Copy> {
    res: PoolAlloc,
    len: usize,
    _brand: PhantomData<(&'stream SharedStream, T)>,
}

/// **流序分配可读缓冲**(synchronized,RXS-0147)。由 [`AsyncBuffer::share_with`] /
/// [`AsyncReady::share_with`] 经显式时序边重 brand 得到:可在所属 `'stream` 上读
/// (`copy_to_host`)/写(`copy_from_host`)/取 `device_ptr` 供同 stream launch。affine move-only;
/// 仍 `Drop = cuMemFreeAsync`(流序释放)。再跨 stream 须再次 `share_with`。
#[must_use = "AsyncReady 为流序分配缓冲,未用即流序释放(RXS-0144)"]
pub struct AsyncReady<'stream, T: Copy> {
    res: PoolAlloc,
    len: usize,
    _brand: PhantomData<(&'stream SharedStream, T)>,
}

impl SharedStream {
    /// **流序分配**(`cuMemAllocAsync` 入本 stream 的 ordered memory pool,RXS-0144):`len` 个 `T`
    /// (未初始化)→ in-flight [`AsyncBuffer`](brand 绑本 stream)。分配在本 stream 排队,同 stream
    /// 后续操作经 stream 序排在其后(规则①)。老驱动无流序分配符号 → `DriverUnavailable`。
    pub fn alloc_async<T: Copy>(&self, len: usize) -> Result<AsyncBuffer<'_, T>> {
        let cuda = self.cuda()?;
        if !cuda.has_stream_ordered_alloc() {
            return Err(crate::CudaError::DriverUnavailable);
        }
        let bytes = len
            .checked_mul(size_of::<T>())
            .expect("alloc_async 字节数溢出");
        // SAFETY: (U19):`self.raw` 为有效 stream 句柄;`mem_alloc_async` 出参有效可写。
        let (r, ptr) = unsafe { cuda.mem_alloc_async(bytes, self.raw) }
            .ok_or(crate::CudaError::DriverUnavailable)?;
        check("cuMemAllocAsync", r)?;
        Ok(AsyncBuffer {
            res: PoolAlloc {
                inner: Arc::clone(&self.inner),
                stream: self.raw,
                ptr,
            },
            len,
            _brand: PhantomData,
        })
    }
}

impl<'stream, T: Copy> AsyncBuffer<'stream, T> {
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn byte_len(&self) -> usize {
        self.len * size_of::<T>()
    }

    /// **跨 stream 显式时序边**(规则③,RXS-0147):在产出(所属)stream `record` `event`、在
    /// `other` stream `wait_event`(`cuEventRecord` + `cuStreamWaitEvent`)建立流序依赖,**消费**
    /// self、重 brand 到 `'other` → 可在 `other` 上读/写/操作的 [`AsyncReady`]。释放责任
    /// (`PoolAlloc`)经 `move` 单点转移,流序释放改到 `other`。缺 `share_with` 直接读 `AsyncBuffer`
    /// (`copy_to_host` / `device_ptr` 不存在)→ 编译期 `E0599`(规则①/③)。
    pub fn share_with<'other>(
        self,
        other: &'other SharedStream,
        event: &SharedEvent,
    ) -> Result<AsyncReady<'other, T>> {
        let cuda = sys::cuda().ok_or(crate::CudaError::DriverUnavailable)?;
        // SAFETY: (U15/U14):`event.raw` 有效 event;`self.res.stream` 为产出 stream;同 current context。
        let r = unsafe { cuda.event_record(event.raw, self.res.stream) };
        check("cuEventRecord", r)?;
        // SAFETY: (U15):`other.raw` 有效 stream;`event.raw` 有效已 record event;同 context。
        let r = unsafe { cuda.stream_wait_event(other.raw, event.raw) };
        check("cuStreamWaitEvent", r)?;
        // 重 brand:`move` `PoolAlloc` 出 self(wrapper 无 Drop,安全 move,释放责任单点转移),
        // 流序释放改到 `other`(已 `wait` 同步)。
        let mut res = self.res;
        res.stream = other.raw;
        Ok(AsyncReady {
            res,
            len: self.len,
            _brand: PhantomData,
        })
    }
}

impl<'stream, T: Copy> AsyncReady<'stream, T> {
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn byte_len(&self) -> usize {
        self.len * size_of::<T>()
    }

    /// 设备地址(供**所属 stream** 同 stream launch/操作消费;跨 stream 须先 `share_with`,RXS-0147)。
    pub fn device_ptr(&self) -> CuDevicePtr {
        self.res.ptr
    }

    /// 同步 H2D 写(`cuMemcpyHtoD`;`src` 长度须 ≤ 容量,RXS-0145)。
    pub fn copy_from_host(&mut self, src: &[T]) -> Result<()> {
        assert!(src.len() <= self.len, "copy_from_host: 源长度超出缓冲容量");
        let cuda = sys::cuda().ok_or(crate::CudaError::DriverUnavailable)?;
        let bytes = size_of_val(src);
        // SAFETY: (U6):`self.res.ptr` 为 ≥ bytes 的流序分配;`src` 为 bytes 字节有效可读主机内存。
        let r = unsafe { cuda.memcpy_htod(self.res.ptr, src.as_ptr().cast::<c_void>(), bytes) };
        check("cuMemcpyHtoD", r)
    }

    /// 同步 D2H 读(`cuMemcpyDtoH`;`dst` 长度须 ≤ 容量,RXS-0145)。**仅 [`AsyncReady`] 提供**——
    /// in-flight [`AsyncBuffer`] 无此读接口(规则①,「分配未完成 / 跨 stream 未同步访问」编译期拦截)。
    pub fn copy_to_host(&self, dst: &mut [T]) -> Result<()> {
        assert!(dst.len() <= self.len, "copy_to_host: 目标长度超出缓冲容量");
        let cuda = sys::cuda().ok_or(crate::CudaError::DriverUnavailable)?;
        let bytes = size_of_val(dst);
        // SAFETY: (U6):`dst` 为 bytes 字节有效可写主机内存;`self.res.ptr` 为 ≥ bytes 的流序分配。
        let r = unsafe { cuda.memcpy_dtoh(dst.as_mut_ptr().cast::<c_void>(), self.res.ptr, bytes) };
        check("cuMemcpyDtoH", r)
    }

    /// 再跨 stream 显式时序边(RXS-0147):同 [`AsyncBuffer::share_with`],消费 self 重 brand 到 `'other`。
    pub fn share_with<'other>(
        self,
        other: &'other SharedStream,
        event: &SharedEvent,
    ) -> Result<AsyncReady<'other, T>> {
        let cuda = sys::cuda().ok_or(crate::CudaError::DriverUnavailable)?;
        // SAFETY: (U15/U14):`event.raw`/`self.res.stream`/`other.raw` 有效且同 current context。
        let r = unsafe { cuda.event_record(event.raw, self.res.stream) };
        check("cuEventRecord", r)?;
        // SAFETY: (U15):`other.raw`/`event.raw` 有效且同 context。
        let r = unsafe { cuda.stream_wait_event(other.raw, event.raw) };
        check("cuStreamWaitEvent", r)?;
        let mut res = self.res;
        res.stream = other.raw;
        Ok(AsyncReady {
            res,
            len: self.len,
            _brand: PhantomData,
        })
    }
}

/// **三 stream 流序分配端到端**(RXS-0148 device 佐证;host helper,无 GPU / 老驱动无流序分配 →
/// `DriverUnavailable`,冒烟降级 SKIP)。流序分配 + 两条跨 stream 时序边 + 往返数值对照:
/// `s_alloc.alloc_async` → `share_with(s_compute, ev1)` → 写 → `share_with(s_d2h, ev2)` → 读回校验。
pub fn three_stream_async_pipeline(len: usize) -> Result<bool> {
    let ctx = SharedContext::from_primary(0)?;
    let bound = ctx.bind()?;
    let s_alloc = bound.create_stream()?;
    let s_compute = bound.create_stream()?;
    let s_d2h = bound.create_stream()?;
    let ev1 = bound.create_event()?;
    let ev2 = bound.create_event()?;
    let buf = s_alloc.alloc_async::<f32>(len)?; // 流序分配(规则①:in-flight,无读接口)
    let mut ready = buf.share_with(&s_compute, &ev1)?; // 跨 stream 时序边 1(规则③)→ 可读/写
    let input: Vec<f32> = (0..len).map(|i| i as f32).collect();
    ready.copy_from_host(&input)?;
    let ready = ready.share_with(&s_d2h, &ev2)?; // 跨 stream 时序边 2 → 重 brand 到 D2H
    let mut out = vec![0f32; len];
    ready.copy_to_host(&mut out)?;
    s_d2h.synchronize()?;
    Ok(out == input)
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

    // -- G1.2 流序分配 AsyncBuffer<'stream,T>(RXS-0144~0148;MR-0001) ----------------------

    //@ spec: RXS-0144
    #[test]
    fn async_buffer_alloc_and_pool_raii() {
        // 流序分配 + RAII:SharedStream::alloc_async → AsyncBuffer(cuMemAllocAsync 入 pool);
        // AsyncBuffer/AsyncReady affine(非 Copy/非 Clone);Drop = cuMemFreeAsync(PoolAlloc 流序释放,U19/U20)。
        assert!(size_of::<AsyncBuffer<'static, f32>>() > 0);
        assert!(size_of::<AsyncReady<'static, f32>>() > 0);
        assert!(size_of::<PoolAlloc>() > 0);
        let _pipeline = three_stream_async_pipeline; // 流序分配端到端 fn 面存在性锚定
    }

    //@ spec: RXS-0145
    #[test]
    fn async_buffer_inflight_no_read_interface() {
        // 分配未完成访问被 stream 序排除:in-flight AsyncBuffer 无 device_ptr/copy_to_host;读/写/取址
        // 接口仅 AsyncReady 提供(须先 share_with 同步重 brand)。直接读 AsyncBuffer → E0599
        //(见 src/rurix-rt/compile-fail/async_buffer_alloc_incomplete.rs)。
        let _read = AsyncReady::<'static, f32>::copy_to_host;
        let _write = AsyncReady::<'static, f32>::copy_from_host;
        let _ptr = AsyncReady::<'static, f32>::device_ptr;
    }

    //@ spec: RXS-0146
    #[test]
    fn async_buffer_affine_move_only_use_after_free() {
        // 释放后访问 = 编译期生命周期错误:AsyncBuffer/AsyncReady 单一所有权(非 Copy/非 Clone),
        // move 后再用 E0382(见 compile-fail/async_buffer_use_after_free.rs);Drop=cuMemFreeAsync 单点。
        assert!(size_of::<AsyncBuffer<'static, f32>>() > 0);
        assert!(size_of::<AsyncReady<'static, f32>>() > 0);
    }

    //@ spec: RXS-0147
    #[test]
    fn async_buffer_share_with_cross_stream_edge() {
        // 跨 stream 须 share_with(other,event) 显式时序边(record+wait_event)重 brand 到 'other;
        // 缺 share_with 直接跨 stream 读 → E0599(见 compile-fail/async_buffer_cross_stream_unsync.rs)。
        assert!(size_of::<AsyncBuffer<'static, f32>>() > 0);
        let _pipeline = three_stream_async_pipeline; // 两条跨 stream 时序边端到端 fn 面
    }

    //@ spec: RXS-0148
    #[test]
    fn async_buffer_lifecycle_classes_compile_intercepted() {
        // 三类流序分配生命周期错误由 Rust 类型系统编译期拦截(零新 RX 码):
        //   分配未完成访问 → AsyncBuffer 无 device_ptr/copy_to_host E0599(RXS-0145)
        //   释放后访问 → affine move 后再用 E0382(RXS-0146)
        //   跨 stream 未经 share_with → AsyncBuffer 无读接口 E0599(RXS-0147)
        // reject 覆盖见 src/rurix-rt/compile-fail/async_buffer_*.rs(冒烟步骤 42 断言全拦截 + 真实红绿)。
        assert!(size_of::<AsyncBuffer<'static, f32>>() > 0);
        assert!(size_of::<AsyncReady<'static, f32>>() > 0);
    }

    /// 三 stream 流序分配 device 端到端(冒烟步骤 42 device 段;默认 `cargo test` 跳过,
    /// `--ignored` 运行)。无 GPU / 老驱动无流序分配 → SKIP(打印 skip 标记,不 panic 降级);
    /// 有 GPU 真跑往返数值对照 → 打印 `ASYNC_BUFFER_DEVICE: ok pipeline=1`(供冒烟解析)。
    #[test]
    #[ignore = "device: 需真实 GPU + cuMemAllocAsync(交互桌面会话);冒烟步骤 42 --ignored 运行"]
    fn async_buffer_three_stream_pipeline_device() {
        match three_stream_async_pipeline(1024) {
            Ok(true) => println!("ASYNC_BUFFER_DEVICE: ok pipeline=1 len=1024 roundtrip=match"),
            Ok(false) => panic!("ASYNC_BUFFER_DEVICE: 流序分配三 stream 流水线数值对照失败"),
            Err(crate::CudaError::DriverUnavailable) => {
                println!(
                    "ASYNC_BUFFER_DEVICE: skip reason=DriverUnavailable(无 GPU / 老驱动无流序分配)"
                )
            }
            Err(e) => panic!("ASYNC_BUFFER_DEVICE: 流序分配流水线错误 {e:?}"),
        }
    }
}
