//! rxrt C ABI 运行时边界(MS1.2,RFC-0009 §4.3;spec/host_orchestration.md
//! RXS-0193/0194)。
//!
//! host `.rx` codegen 发射的 `rxrt_*` 调用经本 staticlib 绑定 rurix-rt 运行时(先例
//! rurix-interop PYD 链路,RXS-0125):`#[unsafe(no_mangle)] extern "C"` 符号面 +
//! **u64 句柄表**(进程级 `Mutex<HashMap>`;句柄 `0` = 无效/失败)+ 状态断言,内部包
//! rurix-rt `pipeline.rs` ownership 系([`SharedContext`]/[`DeviceBox`]/[`SharedStream`])
//! 与 `fatbin.rs` 装载协商([`DeviceArtifactSet`],RXS-0150/0151)。
//!
//! - **符号集含义冻结、布局不冻结为语言 ABI**(RXS-0180 L3 口径):`rxrt_*` 面是工具链
//!   内部实现要求(RXS-0194),非用户 stable ABI;用户面是 std::gpu 类型/方法语义
//!   (RFC-0009 §4.1)。
//! - **运行期失败语义**(RXS-0193 / RFC-0009 §4.5):任何失败落 stderr 确定性诊断一行
//!   `RXRT: error op=<op> detail=<...>` 后返回失败值(句柄 `0` / 负 `i32` / null 指针)
//!   ——是否终止由调用方(编译器注入的检查)裁决;context 携 **poisoned 标志**,任何
//!   CUDA 失败后置位,后续该 ctx 系操作全部确定性失败,**不产生 UB、无静默降级**(P-01)。
//! - **销毁纪律**(D-231 镜像):[`rxrt_ctx_destroy`] 先 `cuCtxSynchronize` 再落表;
//!   [`rxrt_buf_free`] free 前对**所属 ctx** 做 sync(封口 affine 检查覆盖不了的 launch
//!   异步窗口,防 in-flight UAF)。
//! - **惰性装载缓存**(RFC-0009 §4.4):module 按 ctx 惰性装载并缓存(fatbin 协商:
//!   cubin 命中免 JIT,否则 PTX 版号梯子);CUfunction 按 entry 名经 `cuModuleGetFunction`
//!   每次查取(驱动端查表,JIT/装载才是成本重点)。
//! - **每操作重绑 current context**(`SharedContext::bind`):cabi 可从宿主任意线程进入,
//!   操作前 `cuCtxSetCurrent` 重绑(U13 论证);句柄表 `Mutex` 全程互斥。
//! - unsafe 全部集中于本 crate 的调用方指针契约边界(逐处 `// SAFETY:`),注册见
//!   `unsafe-audit/rurix-rt-cabi.md`(U25);全仓其余 crate `unsafe_code = deny` 维持。
//!
//! MS1.2b 延伸面(RFC-0009 §4.6/§4.7):`rxp_*` present 会话(见 [`present`] 模块,
//! feature `present`,default 含;RXS-0197/0198)+ `rxio_write_ppm` 宿主图像落盘桥
//! (见 [`imageio`] 模块,RXS-0199)。present backbuffer 以**借用**形态进设备缓冲
//! 句柄表([`BufKind::Borrowed`],owned = false):[`rxrt_buf_free`] 对其 no-op,
//! 设备内存释放责任留呈现会话(RXS-0198)。
//!
//! 嵌入产物描述表(`@__rx_gpu_artifacts`)二进制布局见 [`artifacts`] 模块文档。

use core::ffi::c_void;
use std::collections::HashMap;
use std::ffi::CStr;
use std::sync::{Mutex, MutexGuard, OnceLock};

use rurix_rt::fatbin::DeviceArtifactSet;
use rurix_rt::{DeviceBox, PinnedBox, SharedContext, SharedModule, SharedStream};

mod artifacts;
mod imageio;
#[cfg(feature = "present")]
mod present;

/// 运行期失败返回值(RXS-0193:诊断行 + 负值,由调用方裁决终止)。
const RXRT_FAIL: i32 = -1;

/// poisoned 确定性失败诊断 detail(RXS-0193,对齐 RXS-0077 poisoned 语义)。
const POISONED: &str = "poisoned context (a previous CUDA failure was recorded; \
     all further operations on this ctx fail deterministically, RXS-0193)";

/// 确定性诊断行(RFC-0009 §4.5):`RXRT: error op=<op> detail=<...>` 落 stderr。
fn diag(op: &str, detail: impl std::fmt::Display) {
    eprintln!("RXRT: error op={op} detail={detail}");
}

// -- !Send 句柄跨线程存表包装(U25) -------------------------------------------------
//
// 句柄表为进程级 `static Mutex`(要求条目 `Send`);`SharedStream`/`SharedModule` 持裸
// CUDA 句柄、`PinnedBox` 持裸主机指针,故 `!Send`。CUDA driver 对象为**进程级**(绑
// context),跨线程使用合法——前提是操作前重绑 current context(镜像 pipeline.rs
// U13/U14 论证);本 crate 每个 cabi 操作先经 `SharedContext::bind` 重绑,各内层类型
// 持 `Arc<SharedInner>` 保证 context 存活、Drop 自行重绑 current 后释放(单一所有权,
// Drop 仅一次)。`Mutex` 全程互斥,存表仅 move 语义、无跨线程共享 `&`(不实现 `Sync`
// 语义面;`Mutex` 内部可变性已覆盖)。

/// [`SharedStream`] 存表包装(`!Send` 豁免,见上方模块注释)。
struct SendStream(SharedStream);

// SAFETY: (U25):CUstream 为绑 context 的进程级驱动对象,跨线程使用合法(每操作先
// `SharedContext::bind` 重绑 current;镜像 U13/U14);内层持 `Arc<SharedInner>` 保证
// context 存活,Drop 自行重绑后 `cuStreamDestroy`,单一所有权 Drop 仅一次。
unsafe impl Send for SendStream {}

/// [`SharedModule`] 存表包装(`'static` brand 由 `Bound::load_module_artifacts` 产出,
/// 模块自持 `Arc<SharedInner>`;`!Send` 豁免同上)。
struct SendModule(SharedModule<'static>);

// SAFETY: (U25):CUmodule 为绑 context 的进程级驱动对象(论证同 `SendStream`);
// Drop 自行重绑后 `cuModuleUnload`,单一所有权 Drop 仅一次。
unsafe impl Send for SendModule {}

/// [`PinnedBox`] 存表包装(`!Send` 豁免同上;锁页内存为进程级主机内存,指针在
/// `cuMemFreeHost` 前稳定有效)。
struct SendPinned(PinnedBox<u8>);

// SAFETY: (U25):锁页主机内存(`cuMemAllocHost`)为进程级分配,任意线程读写合法;
// 内层持 `Arc<SharedInner>` 保证 context 存活,Drop 自行重绑后 `cuMemFreeHost`,
// 单一所有权 Drop 仅一次。
unsafe impl Send for SendPinned {}

// -- u64 句柄表(RFC-0009 §4.3;句柄 0 = 无效/失败) --------------------------------

/// context 条目:共享 primary context + 嵌入产物变体集 + 惰性 module 缓存 + poisoned。
struct CtxEntry {
    shared: SharedContext,
    artifacts: DeviceArtifactSet,
    /// 惰性装载缓存(首次 launch 时经 fatbin 协商装载,RFC-0009 §4.4)。
    module: Option<SendModule>,
    /// 任何 CUDA 失败后置位;置位后该 ctx 系操作全部确定性失败(RXS-0193)。
    poisoned: bool,
}

/// stream 条目(记所属 ctx 句柄,供 poisoned 检查与重绑)。
struct StreamEntry {
    ctx: u64,
    stream: SendStream,
}

/// 设备缓冲条目(`bytes` = 分配/注册字节数,upload/download 长度须精确匹配)。
struct BufEntry {
    ctx: u64,
    bytes: u64,
    kind: BufKind,
}

/// 设备缓冲所有权形态(RXS-0198:present backbuffer 以**借用**句柄进表)。
enum BufKind {
    /// [`rxrt_buf_alloc`] 拥有的设备分配([`DeviceBox`] Drop 释放)。
    Owned(DeviceBox<u8>),
    /// 借用注册的设备指针(owned = false):[`rxrt_buf_free`] 对其 **no-op**——不触
    /// CUDA、不落表、不释放,设备内存释放责任留注册方(`rxp_*` 呈现会话的共享
    /// backbuffer,生命期 = 会话,`rxp_destroy` 清表;RXS-0198)。`sess` = 注册方
    /// 会话句柄(诊断/清表锚)。
    #[cfg_attr(not(feature = "present"), allow(dead_code))]
    Borrowed { dptr: u64, sess: u64 },
}

impl BufEntry {
    /// launch 实参物化用设备指针(owned / borrowed 同一口径)。
    fn device_ptr(&self) -> u64 {
        match &self.kind {
            BufKind::Owned(buf) => buf.device_ptr(),
            BufKind::Borrowed { dptr, .. } => *dptr,
        }
    }
}

/// 锁页主机缓冲条目。
struct PinnedEntry {
    ctx: u64,
    buf: SendPinned,
}

/// G3.4 bindless(RXS-0235):std::gpu `TextureTable` 宿主注册面条目——**注册序即索引**
/// 稳定单调的纹理句柄段(元素为纹理资源句柄 u64);descriptor 写入 / feature chain 探
/// 测归 vk.rs 运行时(设备路),本表仅承载注册序与计数(host 侧,提交前注册)。
struct TableEntry {
    /// 所属 ctx 句柄(poisoned 检查 / 清表锚)。
    ctx: u64,
    /// 已注册纹理资源句柄(按注册序;下标 = `register` 返回的索引)。
    textures: Vec<u64>,
}

/// 进程级句柄表(单锁:无锁序问题;宿主 `.rx` 首期单线程,互斥仅为 Send/Sync 健全性)。
#[derive(Default)]
struct Tables {
    next: u64,
    ctxs: HashMap<u64, CtxEntry>,
    streams: HashMap<u64, StreamEntry>,
    bufs: HashMap<u64, BufEntry>,
    pinned: HashMap<u64, PinnedEntry>,
    /// G3.4 bindless(RXS-0235):`TextureTable` 注册面(只追加符号族 `rxrt_table_*`)。
    texture_tables: HashMap<u64, TableEntry>,
}

impl Tables {
    /// 派发新句柄(自 1 起单调递增;0 恒为无效句柄)。
    fn alloc_handle(&mut self) -> u64 {
        self.next += 1;
        self.next
    }
}

fn tables() -> &'static Mutex<Tables> {
    static TABLES: OnceLock<Mutex<Tables>> = OnceLock::new();
    TABLES.get_or_init(|| Mutex::new(Tables::default()))
}

/// 取句柄表锁(锁 poisoned 时取内层数据继续:cabi 自身不 panic,持锁线程 panic 属
/// 调用方异常路径,确定性失败优先于 panic 级联)。
fn lock() -> MutexGuard<'static, Tables> {
    match tables().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

// -- context ------------------------------------------------------------------------

/// C ABI:创建 GPU 上下文(RFC-0009 §4.3)。`artifacts` 指向 codegen 发射的嵌入产物
/// 描述表(布局见 [`artifacts`] 模块;PTX fallback 必存 + 可选 sm 键 cubin),解析后
/// 构造 [`DeviceArtifactSet`] 并保留 device 0 primary context。失败(畸形描述表 / 无
/// 驱动 / CUDA 错误)→ 确定性诊断 + 返回 `0`。
//@ spec: RXS-0194
#[allow(clippy::not_unsafe_ptr_arg_deref)] // C ABI 入口:指针契约由调用方 codegen 保证(U25)
#[unsafe(no_mangle)]
pub extern "C" fn rxrt_ctx_create(artifacts: *const u8) -> u64 {
    const OP: &str = "ctx_create";
    // SAFETY: (U25):`artifacts` 为 codegen 发射的 `@__rx_gpu_artifacts` 常量地址
    // (RFC-0009 §4.4),指向 ≥48 字节 v1 描述表,其 ptx/cubin 指针字段指向同产物常量段
    // (进程生命期有效);null 与字段级畸形在 `parse` 内解引用载荷前确定性拒绝。
    let parsed = match unsafe { artifacts::parse(artifacts) } {
        Ok(parsed) => parsed,
        Err(detail) => {
            diag(OP, detail);
            return 0;
        }
    };
    let shared = match SharedContext::from_primary(0) {
        Ok(shared) => shared,
        Err(e) => {
            diag(OP, e);
            return 0;
        }
    };
    let mut set = DeviceArtifactSet::new(parsed.ptx);
    if let Some((sm, bytes)) = parsed.cubin {
        set = set.with_cubin(sm, bytes);
    }
    let mut t = lock();
    let h = t.alloc_handle();
    t.ctxs.insert(
        h,
        CtxEntry {
            shared,
            artifacts: set,
            module: None,
            poisoned: false,
        },
    );
    h
}

/// C ABI:销毁上下文——**先 sync 再落表**(D-231 镜像);poisoned ctx 跳过 sync(必然
/// 确定性失败)直接落表。重复/未知句柄 = no-op + 诊断。存活的 stream/buffer 各持
/// `Arc<SharedInner>`,primary context 不早于其资源释放。
//@ spec: RXS-0194
#[unsafe(no_mangle)]
pub extern "C" fn rxrt_ctx_destroy(ctx: u64) {
    const OP: &str = "ctx_destroy";
    let mut t = lock();
    let Some(ce) = t.ctxs.get_mut(&ctx) else {
        diag(
            OP,
            format!("unknown or already destroyed ctx handle {ctx} (no-op)"),
        );
        return;
    };
    if !ce.poisoned
        && let Err(e) = ce.shared.bind().and_then(|b| b.synchronize())
    {
        // 仍继续销毁(镜像 Context::drop best-effort 销毁纪律)。
        diag(OP, e);
    }
    t.ctxs.remove(&ctx); // Drop 序:module 卸载 → SharedContext(Arc 引用计数)
}

/// C ABI:同步上下文(`cuCtxSynchronize`)。`0` 成功;失败 → 诊断 + 负值 + poison。
//@ spec: RXS-0194
#[unsafe(no_mangle)]
pub extern "C" fn rxrt_ctx_sync(ctx: u64) -> i32 {
    const OP: &str = "ctx_sync";
    let mut t = lock();
    let Some(ce) = t.ctxs.get_mut(&ctx) else {
        diag(OP, format!("unknown ctx handle {ctx}"));
        return RXRT_FAIL;
    };
    if ce.poisoned {
        diag(OP, POISONED);
        return RXRT_FAIL;
    }
    if let Err(e) = ce.shared.bind().and_then(|b| b.synchronize()) {
        ce.poisoned = true;
        diag(OP, e);
        return RXRT_FAIL;
    }
    0
}

// -- stream -------------------------------------------------------------------------

/// C ABI:在上下文上创建 stream(`cuStreamCreate`)。失败 → 诊断 + `0`。
//@ spec: RXS-0194
#[unsafe(no_mangle)]
pub extern "C" fn rxrt_stream_create(ctx: u64) -> u64 {
    const OP: &str = "stream_create";
    let mut guard = lock();
    let t = &mut *guard;
    let stream = {
        let Some(ce) = t.ctxs.get_mut(&ctx) else {
            diag(OP, format!("unknown ctx handle {ctx}"));
            return 0;
        };
        if ce.poisoned {
            diag(OP, POISONED);
            return 0;
        }
        match ce.shared.bind().and_then(|b| b.create_stream()) {
            Ok(stream) => stream,
            Err(e) => {
                ce.poisoned = true;
                diag(OP, e);
                return 0;
            }
        }
    };
    let h = t.alloc_handle();
    t.streams.insert(
        h,
        StreamEntry {
            ctx,
            stream: SendStream(stream),
        },
    );
    h
}

/// C ABI:销毁 stream(落表;`SharedStream` Drop 自行重绑 current 后 `cuStreamDestroy`)。
/// 重复/未知句柄 = no-op + 诊断。
//@ spec: RXS-0194
#[unsafe(no_mangle)]
pub extern "C" fn rxrt_stream_destroy(s: u64) {
    const OP: &str = "stream_destroy";
    let mut t = lock();
    if t.streams.remove(&s).is_none() {
        diag(
            OP,
            format!("unknown or already destroyed stream handle {s} (no-op)"),
        );
    }
}

/// C ABI:同步 stream(`cuStreamSynchronize`)。`0` 成功;失败 → 诊断 + 负值 + poison。
//@ spec: RXS-0194
#[unsafe(no_mangle)]
pub extern "C" fn rxrt_stream_sync(s: u64) -> i32 {
    const OP: &str = "stream_sync";
    let mut guard = lock();
    let t = &mut *guard;
    let Some(se) = t.streams.get(&s) else {
        diag(OP, format!("unknown stream handle {s}"));
        return RXRT_FAIL;
    };
    let Some(ce) = t.ctxs.get_mut(&se.ctx) else {
        diag(OP, format!("ctx of stream {s} already destroyed"));
        return RXRT_FAIL;
    };
    if ce.poisoned {
        diag(OP, POISONED);
        return RXRT_FAIL;
    }
    if let Err(e) = ce.shared.bind().and_then(|_b| se.stream.0.synchronize()) {
        ce.poisoned = true;
        diag(OP, e);
        return RXRT_FAIL;
    }
    0
}

// -- device buffer ------------------------------------------------------------------

/// C ABI:设备内存分配(`cuMemAlloc`,`bytes` 字节)。失败/零字节 → 诊断 + `0`。
//@ spec: RXS-0194
#[unsafe(no_mangle)]
pub extern "C" fn rxrt_buf_alloc(ctx: u64, bytes: u64) -> u64 {
    const OP: &str = "buf_alloc";
    if bytes == 0 {
        diag(OP, "zero-byte allocation");
        return 0;
    }
    let mut guard = lock();
    let t = &mut *guard;
    let buf = {
        let Some(ce) = t.ctxs.get_mut(&ctx) else {
            diag(OP, format!("unknown ctx handle {ctx}"));
            return 0;
        };
        if ce.poisoned {
            diag(OP, POISONED);
            return 0;
        }
        match ce.shared.bind().and_then(|b| b.alloc::<u8>(bytes as usize)) {
            Ok(buf) => buf,
            Err(e) => {
                ce.poisoned = true;
                diag(OP, e);
                return 0;
            }
        }
    };
    let h = t.alloc_handle();
    t.bufs.insert(
        h,
        BufEntry {
            ctx,
            bytes,
            kind: BufKind::Owned(buf),
        },
    );
    h
}

/// C ABI:释放设备缓冲——**free 前对所属 ctx 做 sync**(D-231 镜像,封口 launch 异步
/// 窗口,防 in-flight UAF);sync 失败 → poison + 诊断,仍落表释放(镜像 `Context::drop`
/// best-effort)。重复/未知句柄 = no-op + 诊断。
//@ spec: RXS-0194
#[unsafe(no_mangle)]
pub extern "C" fn rxrt_buf_free(b: u64) {
    const OP: &str = "buf_free";
    let mut guard = lock();
    let t = &mut *guard;
    let Some(be) = t.bufs.get(&b) else {
        diag(
            OP,
            format!("unknown or already freed buffer handle {b} (no-op)"),
        );
        return;
    };
    // 借用条目(owned = false,RXS-0198):free = **no-op**——不触 CUDA、不落表,
    // 设备内存释放责任留注册方(呈现会话共享 backbuffer 生命期 = 会话,rxp_destroy
    // 清表);条目留表供后续帧 launch 复用同句柄。
    if matches!(be.kind, BufKind::Borrowed { .. }) {
        return;
    }
    match t.ctxs.get_mut(&be.ctx) {
        Some(ce) if !ce.poisoned => {
            if let Err(e) = ce.shared.bind().and_then(|bound| bound.synchronize()) {
                ce.poisoned = true;
                diag(OP, e);
            }
        }
        // poisoned:sync 必然确定性失败,直接落表(Drop 自行重绑后 free)。
        Some(_) => {}
        None => diag(OP, format!("ctx of buffer {b} already destroyed")),
    }
    t.bufs.remove(&b); // DeviceBox Drop:重绑本 context 后 cuMemFree(U13/U3)
}

/// C ABI:H2D 上传(`cuMemcpyHtoD`;`bytes` 须与缓冲分配字节数精确一致,不匹配 =
/// 失败诊断,不触 CUDA)。`0` 成功;失败 → 诊断 + 负值。
//@ spec: RXS-0194
#[allow(clippy::not_unsafe_ptr_arg_deref)] // C ABI 入口:指针契约由调用方 codegen 保证(U25)
#[unsafe(no_mangle)]
pub extern "C" fn rxrt_buf_upload(b: u64, src: *const u8, bytes: u64) -> i32 {
    const OP: &str = "buf_upload";
    if src.is_null() {
        diag(OP, "null src pointer");
        return RXRT_FAIL;
    }
    let mut guard = lock();
    let t = &mut *guard;
    let Some(be) = t.bufs.get_mut(&b) else {
        diag(OP, format!("unknown buffer handle {b}"));
        return RXRT_FAIL;
    };
    // 借用 backbuffer(RXS-0198)无 upload 面:内容由 blit kernel 经 launch 写入。
    let buf = match &mut be.kind {
        BufKind::Owned(buf) => buf,
        BufKind::Borrowed { sess, .. } => {
            diag(
                OP,
                format!(
                    "buffer handle {b} is a borrowed present backbuffer of session {sess} \
                     (upload unsupported, RXS-0198)"
                ),
            );
            return RXRT_FAIL;
        }
    };
    let Some(ce) = t.ctxs.get_mut(&be.ctx) else {
        diag(OP, format!("ctx of buffer {b} already destroyed"));
        return RXRT_FAIL;
    };
    if ce.poisoned {
        diag(OP, POISONED);
        return RXRT_FAIL;
    }
    if bytes != be.bytes {
        diag(
            OP,
            format!("length mismatch: buffer is {} bytes, got {bytes}", be.bytes),
        );
        return RXRT_FAIL;
    }
    // SAFETY: (U25):`src` 非 null(上方已检),调用方(codegen 发射的 upload 调用)保证
    // 其指向 `bytes` 字节有效可读主机内存且调用期存活(RFC-0009 §4.3 指针契约);借用不
    // 越出本函数。
    let host = unsafe { core::slice::from_raw_parts(src, bytes as usize) };
    if let Err(e) = ce.shared.bind().and_then(|_b| buf.copy_from_host(host)) {
        ce.poisoned = true;
        diag(OP, e);
        return RXRT_FAIL;
    }
    0
}

/// C ABI:D2H 下载(`cuMemcpyDtoH`;长度纪律同 [`rxrt_buf_upload`])。
//@ spec: RXS-0194
#[allow(clippy::not_unsafe_ptr_arg_deref)] // C ABI 入口:指针契约由调用方 codegen 保证(U25)
#[unsafe(no_mangle)]
pub extern "C" fn rxrt_buf_download(b: u64, dst: *mut u8, bytes: u64) -> i32 {
    const OP: &str = "buf_download";
    if dst.is_null() {
        diag(OP, "null dst pointer");
        return RXRT_FAIL;
    }
    let mut guard = lock();
    let t = &mut *guard;
    let Some(be) = t.bufs.get(&b) else {
        diag(OP, format!("unknown buffer handle {b}"));
        return RXRT_FAIL;
    };
    // 借用 backbuffer(RXS-0198)无 download 面(呈现内容回读不在 v1 契约)。
    let buf = match &be.kind {
        BufKind::Owned(buf) => buf,
        BufKind::Borrowed { sess, .. } => {
            diag(
                OP,
                format!(
                    "buffer handle {b} is a borrowed present backbuffer of session {sess} \
                     (download unsupported, RXS-0198)"
                ),
            );
            return RXRT_FAIL;
        }
    };
    let Some(ce) = t.ctxs.get_mut(&be.ctx) else {
        diag(OP, format!("ctx of buffer {b} already destroyed"));
        return RXRT_FAIL;
    };
    if ce.poisoned {
        diag(OP, POISONED);
        return RXRT_FAIL;
    }
    if bytes != be.bytes {
        diag(
            OP,
            format!("length mismatch: buffer is {} bytes, got {bytes}", be.bytes),
        );
        return RXRT_FAIL;
    }
    // SAFETY: (U25):`dst` 非 null(上方已检),调用方保证其指向 `bytes` 字节有效可写
    // 主机内存、调用期存活且无别名并发访问(RFC-0009 §4.3 指针契约);借用不越出本函数。
    let host = unsafe { core::slice::from_raw_parts_mut(dst, bytes as usize) };
    if let Err(e) = ce.shared.bind().and_then(|_b| buf.copy_to_host(host)) {
        ce.poisoned = true;
        diag(OP, e);
        return RXRT_FAIL;
    }
    0
}

// -- pinned host buffer ---------------------------------------------------------------

/// C ABI:锁页主机内存分配(`cuMemAllocHost`,真 pinned;RXS-0131 staging 语义)。
/// 失败/零字节 → 诊断 + `0`。
//@ spec: RXS-0194
#[unsafe(no_mangle)]
pub extern "C" fn rxrt_pinned_alloc(ctx: u64, bytes: u64) -> u64 {
    const OP: &str = "pinned_alloc";
    if bytes == 0 {
        diag(OP, "zero-byte allocation");
        return 0;
    }
    let mut guard = lock();
    let t = &mut *guard;
    let buf = {
        let Some(ce) = t.ctxs.get_mut(&ctx) else {
            diag(OP, format!("unknown ctx handle {ctx}"));
            return 0;
        };
        if ce.poisoned {
            diag(OP, POISONED);
            return 0;
        }
        match ce
            .shared
            .bind()
            .and_then(|b| b.alloc_pinned::<u8>(bytes as usize))
        {
            Ok(buf) => buf,
            Err(e) => {
                ce.poisoned = true;
                diag(OP, e);
                return 0;
            }
        }
    };
    let h = t.alloc_handle();
    t.pinned.insert(
        h,
        PinnedEntry {
            ctx,
            buf: SendPinned(buf),
        },
    );
    h
}

/// C ABI:取锁页缓冲主机指针(host 侧 get/set 消费;指针至 [`rxrt_pinned_free`] 前
/// 稳定有效)。未知句柄 / poisoned ctx → 诊断 + null。
//@ spec: RXS-0194
#[unsafe(no_mangle)]
pub extern "C" fn rxrt_pinned_ptr(p: u64) -> *mut u8 {
    const OP: &str = "pinned_ptr";
    let mut guard = lock();
    let t = &mut *guard;
    let Some(pe) = t.pinned.get_mut(&p) else {
        diag(OP, format!("unknown pinned handle {p}"));
        return core::ptr::null_mut();
    };
    let Some(ce) = t.ctxs.get(&pe.ctx) else {
        diag(OP, format!("ctx of pinned buffer {p} already destroyed"));
        return core::ptr::null_mut();
    };
    if ce.poisoned {
        diag(OP, POISONED);
        return core::ptr::null_mut();
    }
    pe.buf.0.as_mut_slice().as_mut_ptr()
}

/// C ABI:释放锁页缓冲(落表;`PinnedBox` Drop 自行重绑 current 后 `cuMemFreeHost`)。
/// `rxrt_*` v1 面无异步搬运,无 in-flight pinned 窗口,故不前置 sync(对照
/// [`rxrt_buf_free`])。重复/未知句柄 = no-op + 诊断。
//@ spec: RXS-0194
#[unsafe(no_mangle)]
pub extern "C" fn rxrt_pinned_free(p: u64) {
    const OP: &str = "pinned_free";
    let mut t = lock();
    if t.pinned.remove(&p).is_none() {
        diag(
            OP,
            format!("unknown or already freed pinned handle {p} (no-op)"),
        );
    }
}

/// C ABI:查设备缓冲分配字节数(编译器注入的 upload/download 长度与 `buf.len()`
/// 消费;RXS-0194 符号面**只追加**口径)。未知句柄 / ctx 已销毁 / poisoned →
/// 诊断 + `0`(长度 0 使后续长度匹配检查确定性失败,RXS-0193)。
//@ spec: RXS-0194
#[unsafe(no_mangle)]
pub extern "C" fn rxrt_buf_len(b: u64) -> u64 {
    const OP: &str = "buf_len";
    let mut guard = lock();
    let t = &mut *guard;
    let Some(be) = t.bufs.get(&b) else {
        diag(OP, format!("unknown buffer handle {b}"));
        return 0;
    };
    let Some(ce) = t.ctxs.get(&be.ctx) else {
        diag(OP, format!("ctx of buffer {b} already destroyed"));
        return 0;
    };
    if ce.poisoned {
        diag(OP, POISONED);
        return 0;
    }
    be.bytes
}

/// C ABI:查锁页缓冲分配字节数(编译器注入的 get/set 越界检查与 `pinned.len()`
/// 消费;纪律同 [`rxrt_buf_len`])。
//@ spec: RXS-0194
#[unsafe(no_mangle)]
pub extern "C" fn rxrt_pinned_len(p: u64) -> u64 {
    const OP: &str = "pinned_len";
    let mut guard = lock();
    let t = &mut *guard;
    let Some(pe) = t.pinned.get(&p) else {
        diag(OP, format!("unknown pinned handle {p}"));
        return 0;
    };
    let Some(ce) = t.ctxs.get(&pe.ctx) else {
        diag(OP, format!("ctx of pinned buffer {p} already destroyed"));
        return 0;
    };
    if ce.poisoned {
        diag(OP, POISONED);
        return 0;
    }
    pe.buf.0.len() as u64
}

// -- G3.4 bindless:std::gpu `TextureTable` 宿主注册面(RXS-0235;`rxrt_table_*` 只追加) --
//
// RXS-0194「符号面只追加」纪律:`rxrt_launch` 及既有 `rxrt_*`/`rxp_*`/`rxio_*` 符号面字节
// 不变;u64 句柄表 / handle-0 = 失败 / poisoned 传播跨后端不变式维持。注册序即索引稳定
// 单调;descriptor pool/set-layout(UPDATE_AFTER_BIND + PARTIALLY_BOUND)+ feature chain
// 四 bit 探测归 vk.rs 运行时(设备路,缺失确定性 Err);本 cabi 面仅承载注册序与计数。
// unsafe 新增集中 vk.rs(折叠 U27 扩注);本 cabi 面纯 safe(HashMap/Vec 累积)。

/// C ABI:创建 `TextureTable`(RXS-0235)。未知 ctx / poisoned → 诊断 + handle-0(失败)。
//@ spec: RXS-0235
#[unsafe(no_mangle)]
pub extern "C" fn rxrt_table_create(ctx: u64) -> u64 {
    const OP: &str = "table_create";
    let mut guard = lock();
    let t = &mut *guard;
    let Some(ce) = t.ctxs.get(&ctx) else {
        diag(OP, format!("unknown ctx handle {ctx}"));
        return 0;
    };
    if ce.poisoned {
        diag(OP, POISONED);
        return 0;
    }
    let h = t.alloc_handle();
    t.texture_tables.insert(
        h,
        TableEntry {
            ctx,
            textures: Vec::new(),
        },
    );
    h
}

/// C ABI:向 `TextureTable` 注册纹理句柄(RXS-0235)——返回**注册序即索引**(0,1,2,…,
/// 稳定单调)。未知 table / ctx 已销毁 / poisoned → 诊断 + `u32::MAX`(失败哨兵,使
/// 后续动态索引确定性越出已注册段;非静默)。注册写入仅发生在提交前(§8,in-flight
/// 期间不更新)。
//@ spec: RXS-0235
#[unsafe(no_mangle)]
pub extern "C" fn rxrt_table_register(table: u64, tex: u64) -> u32 {
    const OP: &str = "table_register";
    let mut guard = lock();
    let t = &mut *guard;
    let Some(te) = t.texture_tables.get(&table) else {
        diag(OP, format!("unknown texture table handle {table}"));
        return u32::MAX;
    };
    let ctx = te.ctx;
    let Some(ce) = t.ctxs.get(&ctx) else {
        diag(
            OP,
            format!("ctx of texture table {table} already destroyed"),
        );
        return u32::MAX;
    };
    if ce.poisoned {
        diag(OP, POISONED);
        return u32::MAX;
    }
    let te = t
        .texture_tables
        .get_mut(&table)
        .expect("table 存在(上文已取)");
    let index = te.textures.len() as u32;
    te.textures.push(tex);
    index
}

/// C ABI:查 `TextureTable` 已注册计数(RXS-0235;= 动态索引 clamp 表长源,codegen 经
/// push-constant 尾槽下发,RXS-0208/0234)。未知 table / ctx 已销毁 / poisoned → 诊断 + `0`。
//@ spec: RXS-0235
#[unsafe(no_mangle)]
pub extern "C" fn rxrt_table_len(table: u64) -> u32 {
    const OP: &str = "table_len";
    let mut guard = lock();
    let t = &mut *guard;
    let Some(te) = t.texture_tables.get(&table) else {
        diag(OP, format!("unknown texture table handle {table}"));
        return 0;
    };
    let Some(ce) = t.ctxs.get(&te.ctx) else {
        diag(
            OP,
            format!("ctx of texture table {table} already destroyed"),
        );
        return 0;
    };
    if ce.poisoned {
        diag(OP, POISONED);
        return 0;
    }
    te.textures.len() as u32
}

/// C ABI:销毁 `TextureTable`(RXS-0235;affine 消费式,清表)。未知 / 已销毁 → no-op 诊断。
//@ spec: RXS-0235
#[unsafe(no_mangle)]
pub extern "C" fn rxrt_table_destroy(table: u64) {
    const OP: &str = "table_destroy";
    let mut guard = lock();
    let t = &mut *guard;
    if t.texture_tables.remove(&table).is_none() {
        diag(
            OP,
            format!("unknown or already destroyed texture table handle {table} (no-op)"),
        );
    }
}

/// C ABI:运行期失败终止(RXS-0193):编译器对每个 `rxrt_*` 失败返回值(负 `i32` /
/// 句柄 `0` / 越界)注入检查分支,命中即调本符号终止进程。确定性诊断行已由失败点
/// 的 [`diag`] 落 stderr,此处直接 abort(无静默降级、无 UB 出口,P-01)。
//@ spec: RXS-0193
#[unsafe(no_mangle)]
pub extern "C" fn rxrt_trap() -> ! {
    std::process::abort()
}

// -- launch ---------------------------------------------------------------------------

/// C ABI:kernel launch(RFC-0009 §4.4 marshalling)。`entry` = NUL 终止 kernel 符号名
/// (codegen 以 device MIR 同源 mangle 名发射);`slots`/`kinds` 为长度 `n_args` 平行
/// 数组——`kinds[i] == 0`:`slots[i]` 是 buffer 句柄,换设备指针;`kinds[i] == 1`:
/// `slots[i]` 是标量按位值(x64 little-endian 下 kernelParams 指向 slot 本身,
/// `cuLaunchKernel` 按形参尺寸读低 4/8 字节)。module 经 fatbin 协商在 ctx 上惰性装载
/// 缓存(cubin 命中免 JIT,否则 PTX 版号梯子)。`0` 成功;失败 → 诊断 + 负值(CUDA
/// 失败并 poison ctx)。
//@ spec: RXS-0194
#[allow(clippy::too_many_arguments)] // C ABI 签名由 RFC-0009 §4.3 冻结含义
#[allow(clippy::not_unsafe_ptr_arg_deref)] // C ABI 入口:指针契约由调用方 codegen 保证(U25)
#[unsafe(no_mangle)]
pub extern "C" fn rxrt_launch(
    s: u64,
    entry: *const u8,
    gx: u32,
    gy: u32,
    gz: u32,
    bx: u32,
    by: u32,
    bz: u32,
    slots: *const u64,
    kinds: *const u8,
    n_args: u64,
) -> i32 {
    const OP: &str = "launch";
    if entry.is_null() {
        diag(OP, "null entry name");
        return RXRT_FAIL;
    }
    // SAFETY: (U25):`entry` 非 null(上方已检),调用方(codegen)保证其为 NUL 终止
    // 字符串常量(device MIR 同源 mangle 名,RFC-0009 §4.4),进程生命期有效。
    let name = unsafe { CStr::from_ptr(entry.cast()) };
    let Ok(name) = name.to_str() else {
        diag(OP, "entry name is not valid UTF-8");
        return RXRT_FAIL;
    };
    let n = n_args as usize;
    let (mut storage, kinds_v): (Vec<u64>, Vec<u8>) = if n == 0 {
        (Vec::new(), Vec::new())
    } else {
        if slots.is_null() || kinds.is_null() {
            diag(OP, "null slots/kinds with n_args > 0");
            return RXRT_FAIL;
        }
        // SAFETY: (U25):`slots`/`kinds` 非 null(上方已检),调用方保证为长度 `n_args`
        // 的平行数组(RFC-0009 §4.4 marshalling 布局);读入即拷贝为 owned Vec,借用不
        // 越出本函数。
        unsafe {
            (
                core::slice::from_raw_parts(slots, n).to_vec(),
                core::slice::from_raw_parts(kinds, n).to_vec(),
            )
        }
    };

    let mut guard = lock();
    let t = &mut *guard;
    let Some(se) = t.streams.get(&s) else {
        diag(OP, format!("unknown stream handle {s}"));
        return RXRT_FAIL;
    };
    let Some(ce) = t.ctxs.get_mut(&se.ctx) else {
        diag(OP, format!("ctx of stream {s} already destroyed"));
        return RXRT_FAIL;
    };
    if ce.poisoned {
        diag(OP, POISONED);
        return RXRT_FAIL;
    }

    // slot 物化:buffer 位换设备指针(校验句柄与所属 ctx),标量位按位保留。
    for (i, kind) in kinds_v.iter().enumerate() {
        match *kind {
            0 => {
                let Some(arg) = t.bufs.get(&storage[i]) else {
                    diag(OP, format!("arg {i}: unknown buffer handle {}", storage[i]));
                    return RXRT_FAIL;
                };
                if arg.ctx != se.ctx {
                    diag(
                        OP,
                        format!(
                            "arg {i}: buffer handle {} belongs to another ctx",
                            storage[i]
                        ),
                    );
                    return RXRT_FAIL;
                }
                storage[i] = arg.device_ptr();
            }
            1 => {}
            k => {
                diag(OP, format!("arg {i}: unknown arg kind {k} (expected 0|1)"));
                return RXRT_FAIL;
            }
        }
    }

    // module 惰性装载缓存(fatbin 协商:cubin 命中免 JIT,否则 PTX 版号梯子,RXS-0150/0151)。
    if ce.module.is_none() {
        match ce
            .shared
            .bind()
            .and_then(|b| b.load_module_artifacts(&ce.artifacts))
        {
            Ok(module) => ce.module = Some(SendModule(module)),
            Err(e) => {
                ce.poisoned = true;
                diag(OP, e);
                return RXRT_FAIL;
            }
        }
    }
    let Some(module) = ce.module.as_ref() else {
        diag(OP, "module cache unavailable"); // 防御:上方已装载,不可达
        return RXRT_FAIL;
    };

    // 物化纪律镜像 interop.rs `AcquiredFrame::launch`(U7 调用方义务):`storage` 先固定
    // (之后不再增删,地址稳定),`params` 各元素指向对应 slot,二者存活至 `cuLaunchKernel`
    // 返回(同步提交,参数由驱动在调用内拷贝)。
    let launched = ce.shared.bind().and_then(|_b| {
        let kernel = module.0.function(name)?;
        let mut params: Vec<*mut c_void> = storage
            .iter()
            .map(|slot| core::ptr::from_ref(slot).cast_mut().cast::<c_void>())
            .collect();
        se.stream
            .0
            .launch(&kernel, [gx, gy, gz], [bx, by, bz], &mut params)
    });
    if let Err(e) = launched {
        ce.poisoned = true;
        diag(OP, e);
        return RXRT_FAIL;
    }
    0
}

// -- tests ----------------------------------------------------------------------------

/// GPU 探测(镜像 rurix-rt `tests/gpu_roundtrip.rs` 降级 SKIP 纪律;无驱动 / 无设备 /
/// 初始化异常 → 不可用,SKIP 不误判失败)。
#[cfg(test)]
pub(crate) fn gpu_available() -> bool {
    match rurix_rt::Context::device_count() {
        Ok(n) => n > 0,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// G3.4 bindless(RXS-0235):`rxrt_table_*` 符号面失败路不变式(handle-0 / u32::MAX /
    /// no-op)+ 注册序即索引语义——不需 CUDA(纯句柄表逻辑,未知 ctx/table 走确定性失败
    /// 哨兵)。register 顺序单调经 `TableEntry` 语义直接见证(注册序 = Vec 下标)。
    //@ spec: RXS-0235
    #[test]
    fn table_symbols_failure_path_and_register_order() {
        // 未知 ctx → table_create 返回 handle-0(失败)。
        let bogus_ctx = 0xDEAD_0001u64;
        assert_eq!(
            rxrt_table_create(bogus_ctx),
            0,
            "未知 ctx 应 handle-0 失败(RXS-0235)"
        );
        // 未知 table → register 返回 u32::MAX 失败哨兵(使动态索引确定性越出已注册段)。
        let bogus_table = 0xDEAD_0002u64;
        assert_eq!(
            rxrt_table_register(bogus_table, 42),
            u32::MAX,
            "未知 table register 应 u32::MAX 哨兵"
        );
        // 未知 table → len 返回 0(clamp 表长源确定性 0)。
        assert_eq!(rxrt_table_len(bogus_table), 0, "未知 table len 应 0");
        // 未知 table → destroy no-op(不 panic)。
        rxrt_table_destroy(bogus_table);

        // 注册序即索引稳定单调(TableEntry 语义直接见证;register 内 `textures.len()` 即索引)。
        let mut te = TableEntry {
            ctx: 1,
            textures: Vec::new(),
        };
        for (expect_idx, tex) in [(0u32, 100u64), (1, 200), (2, 300)] {
            let idx = te.textures.len() as u32;
            te.textures.push(tex);
            assert_eq!(idx, expect_idx, "注册序即索引(稳定单调,RXS-0235)");
        }
        assert_eq!(te.textures, vec![100, 200, 300], "注册序保序");
    }

    /// 手写 SAXPY PTX(镜像 rurix-rt `tests/gpu_roundtrip.rs`:`y[i] = a*x[i] + y[i]`;
    /// `.version 8.0` 为协商起点,驱动不支持时自动降版,RXS-0076)。
    const SAXPY_PTX: &str = r#".version 8.0
.target sm_89
.address_size 64

.visible .entry saxpy(
    .param .u64 p_x,
    .param .u64 p_y,
    .param .f32 p_a,
    .param .u32 p_n
)
{
    .reg .pred  %p1;
    .reg .b32   %r<6>;
    .reg .f32   %f<5>;
    .reg .b64   %rd<8>;

    ld.param.u64    %rd1, [p_x];
    ld.param.u64    %rd2, [p_y];
    ld.param.f32    %f1,  [p_a];
    ld.param.u32    %r1,  [p_n];

    mov.u32         %r2, %ctaid.x;
    mov.u32         %r3, %ntid.x;
    mov.u32         %r4, %tid.x;
    mad.lo.s32      %r5, %r2, %r3, %r4;

    setp.ge.u32     %p1, %r5, %r1;
    @%p1 bra        DONE;

    cvta.to.global.u64  %rd3, %rd1;
    cvta.to.global.u64  %rd4, %rd2;
    mul.wide.u32        %rd5, %r5, 4;
    add.s64             %rd6, %rd3, %rd5;
    add.s64             %rd7, %rd4, %rd5;

    ld.global.f32   %f2, [%rd6];
    ld.global.f32   %f3, [%rd7];
    mul.rn.f32      %f4, %f1, %f2;
    add.rn.f32      %f4, %f4, %f3;
    st.global.f32   [%rd7], %f4;

DONE:
    ret;
}
"#;

    const NO_CUBIN_KEY: &[u8; 8] = b"\0\0\0\0\0\0\0\0";

    //@ spec: RXS-0194
    #[test]
    fn artifacts_blob_parse_roundtrip() {
        let ptx = b".version 8.0\n.target sm_89\n";
        let cubin = [0xDEu8, 0xAD, 0xBE, 0xEF];
        let blob = artifacts::make_artifacts_blob(ptx, &cubin, b"sm_89\0\0\0");
        assert_eq!(blob.len(), artifacts::DESC_LEN);
        // SAFETY: `blob` 为 make_artifacts_blob 构造的 48 字节 v1 描述表;其指针字段借用
        // 本栈 `ptx`/`cubin`,解析期间存活。
        let parsed = unsafe { artifacts::parse(blob.as_ptr()) }.expect("解析 v1 描述表");
        assert_eq!(parsed.ptx.as_bytes(), ptx);
        let (sm, bytes) = parsed.cubin.expect("cubin 变体在位");
        assert_eq!(sm.as_str(), "sm_89");
        assert_eq!(bytes, cubin);

        // cubin_len = 0 → 仅 PTX fallback(sm_key 忽略)。
        let blob = artifacts::make_artifacts_blob(ptx, &[], NO_CUBIN_KEY);
        // SAFETY: 同上(无 cubin,指针字段仅借用 `ptx`)。
        let parsed = unsafe { artifacts::parse(blob.as_ptr()) }.expect("解析仅 PTX 描述表");
        assert!(parsed.cubin.is_none());
    }

    //@ spec: RXS-0193, RXS-0194
    #[test]
    fn artifacts_blob_rejects_malformed() {
        // null 描述表。
        // SAFETY: null 由 parse 首行确定性拒绝,不解引用。
        assert!(unsafe { artifacts::parse(core::ptr::null()) }.is_err());

        let ptx = b".version 8.0\n";
        // 版本不符(version != 1)。
        let mut blob = artifacts::make_artifacts_blob(ptx, &[], NO_CUBIN_KEY);
        blob[0] = 2;
        // SAFETY: blob 为 48 字节栈上描述表;版本检查在解引用载荷指针前拒绝。
        assert!(unsafe { artifacts::parse(blob.as_ptr()) }.is_err());

        // 缺 PTX(ptx_ptr/ptx_len = 0)。
        let mut blob = artifacts::make_artifacts_blob(ptx, &[], NO_CUBIN_KEY);
        blob[8..24].fill(0);
        // SAFETY: 同上;缺 PTX 在解引用载荷指针前拒绝。
        assert!(unsafe { artifacts::parse(blob.as_ptr()) }.is_err());

        // 坏 sm 键(有 cubin 而键不合 `sm_<digits>` 形态)。
        let cubin = [0x01u8, 0x02];
        let blob = artifacts::make_artifacts_blob(ptx, &cubin, b"compute_");
        // SAFETY: blob/ptx/cubin 均本栈存活;坏 sm 键在解引用 cubin 指针前拒绝。
        assert!(unsafe { artifacts::parse(blob.as_ptr()) }.is_err());

        // 非 UTF-8 PTX。
        let bad_ptx = [0xFFu8, 0xFE, 0x00, 0x01];
        let blob = artifacts::make_artifacts_blob(&bad_ptx, &[], NO_CUBIN_KEY);
        // SAFETY: blob/bad_ptx 本栈存活;UTF-8 校验确定性拒绝。
        assert!(unsafe { artifacts::parse(blob.as_ptr()) }.is_err());
    }

    //@ spec: RXS-0193
    #[test]
    fn unknown_handles_fail_deterministically() {
        // 句柄 0 恒无效;未知句柄一律确定性失败(诊断 + 失败值),不 panic 不 UB。
        assert!(rxrt_ctx_sync(0) < 0);
        assert!(rxrt_ctx_sync(u64::MAX) < 0);
        assert!(rxrt_stream_sync(0) < 0);
        assert_eq!(rxrt_stream_create(0), 0);
        assert_eq!(rxrt_buf_alloc(0, 16), 0);
        assert_eq!(rxrt_buf_alloc(u64::MAX, 0), 0); // 零字节先于句柄检查拒绝
        assert_eq!(rxrt_pinned_alloc(0, 16), 0);
        assert!(rxrt_pinned_ptr(0).is_null());
        let mut b = [0u8; 4];
        assert!(rxrt_buf_upload(0, b.as_ptr(), 4) < 0);
        assert!(rxrt_buf_download(0, b.as_mut_ptr(), 4) < 0);
        assert!(
            rxrt_launch(
                0,
                c"k".as_ptr().cast(),
                1,
                1,
                1,
                1,
                1,
                1,
                core::ptr::null(),
                core::ptr::null(),
                0,
            ) < 0
        );
        // 未知句柄销毁/释放 = no-op + 诊断。
        rxrt_ctx_destroy(0);
        rxrt_stream_destroy(0);
        rxrt_buf_free(0);
        rxrt_pinned_free(0);
    }

    //@ spec: RXS-0198
    #[test]
    fn borrowed_buffer_free_is_noop_and_rejects_copies() {
        // host-only:直接进表一个 Borrowed 条目(镜像 rxp_backbuffer 注册形态,owned =
        // false)——rxrt_buf_free 对其 no-op:条目留表、不触 CUDA、不释放设备内存
        // (释放责任留呈现会话,RXS-0198)。
        let h = {
            let mut t = lock();
            let h = t.alloc_handle();
            t.bufs.insert(
                h,
                BufEntry {
                    ctx: 0,
                    bytes: 48,
                    kind: BufKind::Borrowed {
                        dptr: 0xD3D1_2BB0,
                        sess: 0,
                    },
                },
            );
            h
        };
        rxrt_buf_free(h);
        {
            let t = lock();
            let be = t
                .bufs
                .get(&h)
                .expect("borrowed 条目在 free 后仍在表(no-op)");
            assert_eq!(be.bytes, 48, "字节数不受 free 影响");
            assert_eq!(be.device_ptr(), 0xD3D1_2BB0, "设备指针不受 free 影响");
        }
        // 借用条目无 upload/download 面(确定性拒绝,不触 CUDA;内容由 launch 写入)。
        let mut host = [0u8; 48];
        assert!(rxrt_buf_upload(h, host.as_ptr(), 48) < 0);
        assert!(rxrt_buf_download(h, host.as_mut_ptr(), 48) < 0);
        // 清表走注册方路径(镜像 rxp_destroy 直接移除,不走 free)。
        lock().bufs.remove(&h);
        assert_eq!(rxrt_buf_len(h), 0, "清表后句柄失效");
    }

    //@ spec: RXS-0194
    #[test]
    fn ctx_create_rejects_bad_blob_without_touching_gpu() {
        // 畸形描述表在触 CUDA 前确定性拒绝 → 0(host-only,无 GPU 也过)。
        assert_eq!(rxrt_ctx_create(core::ptr::null()), 0);
        let ptx = b".version 8.0\n";
        let mut blob = artifacts::make_artifacts_blob(ptx, &[], NO_CUBIN_KEY);
        blob[0] = 9; // version != 1
        assert_eq!(rxrt_ctx_create(blob.as_ptr()), 0);
    }

    //@ spec: RXS-0193, RXS-0194
    #[test]
    fn len_getters_report_bytes_and_fail_deterministically() {
        // 未知句柄 → 诊断 + 0(RXS-0193 确定性失败;长度 0 使长度匹配检查必拒)。
        assert_eq!(rxrt_buf_len(0), 0);
        assert_eq!(rxrt_buf_len(u64::MAX), 0);
        assert_eq!(rxrt_pinned_len(0), 0);
        if !gpu_available() {
            eprintln!(
                "[rurix-rt-cabi] SKIP len_getters_report_bytes_and_fail_deterministically 真分配段: 无可用 GPU/驱动(降级 SKIP)"
            );
            return;
        }
        let blob = artifacts::make_artifacts_blob(SAXPY_PTX.as_bytes(), &[], NO_CUBIN_KEY);
        let ctx = rxrt_ctx_create(blob.as_ptr());
        assert_ne!(ctx, 0);
        let b = rxrt_buf_alloc(ctx, 64);
        let p = rxrt_pinned_alloc(ctx, 128);
        assert_eq!(rxrt_buf_len(b), 64, "buf_len = 分配字节数");
        assert_eq!(rxrt_pinned_len(p), 128, "pinned_len = 分配字节数");
        rxrt_buf_free(b);
        rxrt_pinned_free(p);
        assert_eq!(rxrt_buf_len(b), 0, "落表后长度查询确定性失败");
        assert_eq!(rxrt_pinned_len(p), 0);
        rxrt_ctx_destroy(ctx);
    }

    //@ spec: RXS-0194
    #[test]
    fn saxpy_roundtrip_via_cabi() {
        if !gpu_available() {
            eprintln!("[rurix-rt-cabi] SKIP saxpy_roundtrip_via_cabi: 无可用 GPU/驱动(降级 SKIP)");
            return;
        }
        let blob = artifacts::make_artifacts_blob(SAXPY_PTX.as_bytes(), &[], NO_CUBIN_KEY);
        let ctx = rxrt_ctx_create(blob.as_ptr());
        assert_ne!(ctx, 0, "ctx_create");
        let stream = rxrt_stream_create(ctx);
        assert_ne!(stream, 0, "stream_create");

        let n: usize = 1024;
        let bytes = (n * size_of::<f32>()) as u64;
        let x = rxrt_buf_alloc(ctx, bytes);
        let y = rxrt_buf_alloc(ctx, bytes);
        assert_ne!(x, 0, "buf_alloc x");
        assert_ne!(y, 0, "buf_alloc y");
        let host = rxrt_pinned_alloc(ctx, bytes);
        assert_ne!(host, 0, "pinned_alloc");
        let p = rxrt_pinned_ptr(host);
        assert!(!p.is_null(), "pinned_ptr");

        // 经 pinned 指针填 x = i*0.5(镜像 .rx 侧 pinned.set 消费形态)。
        {
            // SAFETY: `p` 为刚分配 bytes 字节锁页主机内存(f32 对齐由 cuMemAllocHost 保证),
            // 本测试线程独占,借用在本块内结束。
            let hs = unsafe { core::slice::from_raw_parts_mut(p.cast::<f32>(), n) };
            for (i, v) in hs.iter_mut().enumerate() {
                *v = i as f32 * 0.5;
            }
        }
        assert_eq!(rxrt_buf_upload(x, p, bytes), 0, "upload x");
        {
            // SAFETY: 同上(重取借用填 y 初值)。
            let hs = unsafe { core::slice::from_raw_parts_mut(p.cast::<f32>(), n) };
            for v in hs.iter_mut() {
                *v = 1.0;
            }
        }
        assert_eq!(rxrt_buf_upload(y, p, bytes), 0, "upload y");

        // 长度不匹配 = 失败诊断(不触 CUDA、不 poison)。
        assert!(rxrt_buf_upload(x, p, bytes - 4) < 0);

        // launch:kinds 0 = buffer 句柄换设备指针;1 = 标量按位(f32 bits / u32)。
        let a: f32 = 2.0;
        let slots: [u64; 4] = [x, y, u64::from(a.to_bits()), n as u64];
        let kinds: [u8; 4] = [0, 0, 1, 1];
        let rc = rxrt_launch(
            stream,
            c"saxpy".as_ptr().cast(),
            (n as u32).div_ceil(256),
            1,
            1,
            256,
            1,
            1,
            slots.as_ptr(),
            kinds.as_ptr(),
            4,
        );
        assert_eq!(rc, 0, "launch saxpy");
        assert_eq!(rxrt_stream_sync(stream), 0, "stream_sync");
        assert_eq!(rxrt_buf_download(y, p, bytes), 0, "download y");
        {
            // SAFETY: 同上(download 返回后重取只读借用核对)。
            let hs = unsafe { core::slice::from_raw_parts(p.cast::<f32>(), n) };
            for (i, v) in hs.iter().enumerate() {
                let expect = 2.0f32 * (i as f32 * 0.5) + 1.0;
                assert_eq!(*v, expect, "y[{i}] 逐元素精确核对");
            }
        }
        assert_eq!(rxrt_ctx_sync(ctx), 0, "ctx_sync");

        // drop 序 = buffer → pinned → stream → ctx(RXS-0193 声明逆序)。
        rxrt_buf_free(x);
        rxrt_buf_free(y);
        rxrt_pinned_free(host);
        rxrt_stream_destroy(stream);
        rxrt_ctx_destroy(ctx);
        // 落表后句柄失效(确定性失败)。
        assert!(rxrt_ctx_sync(ctx) < 0);
    }

    //@ spec: RXS-0193
    #[test]
    fn poisoned_propagation_after_failed_launch() {
        if !gpu_available() {
            eprintln!(
                "[rurix-rt-cabi] SKIP poisoned_propagation_after_failed_launch: 无可用 GPU/驱动(降级 SKIP)"
            );
            return;
        }
        let blob = artifacts::make_artifacts_blob(SAXPY_PTX.as_bytes(), &[], NO_CUBIN_KEY);
        let ctx = rxrt_ctx_create(blob.as_ptr());
        assert_ne!(ctx, 0);
        let stream = rxrt_stream_create(ctx);
        assert_ne!(stream, 0);

        // 未知 entry 名 → cuModuleGetFunction 失败 → 诊断 + 负值 + ctx poisoned。
        let rc = rxrt_launch(
            stream,
            c"no_such_kernel".as_ptr().cast(),
            1,
            1,
            1,
            1,
            1,
            1,
            core::ptr::null(),
            core::ptr::null(),
            0,
        );
        assert!(rc < 0, "未知 entry 的 launch 须失败");

        // poisoned 传播:后续该 ctx 系操作全部确定性失败(RXS-0193)。
        assert!(rxrt_ctx_sync(ctx) < 0);
        assert!(rxrt_stream_sync(stream) < 0);
        assert_eq!(rxrt_buf_alloc(ctx, 16), 0);
        assert_eq!(rxrt_stream_create(ctx), 0);
        assert_eq!(rxrt_pinned_alloc(ctx, 16), 0);

        // 清理类操作仍可落表(不泄漏;poisoned ctx 跳过 sync 直接销毁)。
        rxrt_stream_destroy(stream);
        rxrt_ctx_destroy(ctx);
        assert!(rxrt_ctx_sync(ctx) < 0, "落表后句柄失效");
    }
}
