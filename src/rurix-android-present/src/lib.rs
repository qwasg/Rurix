//! rurix-android-present — 零-Java NativeActivity glue(mb1 W7,G-MB1-7 Phase B)。
//!
//! 最小 cdylib(`librurix_vk.so`),经 `ANativeActivity_onCreate` 手写回调(无
//! `android_native_app_glue.c`,无 C 编译)驱动:UI 主线程回调 O(1) 返回(防 ANR),Vulkan
//! present 只在自建渲染线程跑——复用 `rurix_rt::vk::run_graphics_present_android_safe`
//! (`VK_KHR_android_surface` 出图循环)+ 可选 compute saxpy(`rurix_rt::backend::run_job`)。
//! `VK_LAYER_KHRONOS_validation` on-device 校验(mode marker `red`/`green` 驱动 RED 反证 /
//! GREEN 零报错),结果落 `present_result.json` + logcat(tag `RurixVK` / `RurixVK-VVL`)。
//!
//! **unsafe 边界(U28,注册见 unsafe-audit/rurix-android-present.md)**:NativeActivity ABI +
//! ANativeWindow 生命周期 + 跨-FFI panic 边界(每 `extern "C"` 最外层 `catch_unwind`,绝不
//! unwind 过 C 边界)+ 渲染线程 FFI。每 `unsafe` 块携 `// SAFETY:`。**桌面(非 android)整
//! crate 为空**(target-cfg 依赖不激活,零回归)。
#![cfg(target_os = "android")]
#![allow(non_snake_case)]

use core::ffi::{c_char, c_int, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Condvar, Mutex};

use rurix_rt::vk::android_present::ANativeWindow;

// ── liblog / libnativewindow FFI(build.rs 链接 log/android/nativewindow) ─────────
const ANDROID_LOG_INFO: c_int = 4;
const ANDROID_LOG_ERROR: c_int = 6;

unsafe extern "C" {
    fn __android_log_write(prio: c_int, tag: *const c_char, text: *const c_char) -> c_int;
    fn ANativeWindow_acquire(window: *mut ANativeWindow);
    fn ANativeWindow_release(window: *mut ANativeWindow);
}

/// 一行 logcat(tag `RurixVK`)。内部吞错(诊断路径不 fail-hard)。
fn logcat(prio: c_int, msg: &str) {
    if let Ok(c) = std::ffi::CString::new(msg) {
        // SAFETY: tag 为 NUL 结尾字面量;c 为 NUL 结尾 C 串,liblog 只读不持有(调用期存活)。
        unsafe {
            __android_log_write(prio, c"RurixVK".as_ptr(), c.as_ptr());
        }
    }
}

// ── NativeActivity ABI（android/native_activity.h,#[repr(C)] 逐字节对齐） ─────────
#[repr(C)]
#[allow(dead_code)] // ABI 布局:仅读 callbacks/internal_data_path/instance,余字段占位定偏移。
pub struct ANativeActivity {
    callbacks: *mut ANativeActivityCallbacks,
    vm: *mut c_void,
    env: *mut c_void,
    clazz: *mut c_void,
    internal_data_path: *const c_char,
    external_data_path: *const c_char,
    sdk_version: i32,
    instance: *mut c_void,
    asset_manager: *mut c_void,
    obb_path: *const c_char,
}

type ActivityCb = Option<unsafe extern "C" fn(*mut ANativeActivity)>;
type WindowCb = Option<unsafe extern "C" fn(*mut ANativeActivity, *mut ANativeWindow)>;

#[repr(C)]
#[allow(dead_code)] // ABI 布局:仅写 on_native_window_{created,destroyed}/on_destroy,余占位。
pub struct ANativeActivityCallbacks {
    on_start: ActivityCb,
    on_resume: ActivityCb,
    on_save_instance_state:
        Option<unsafe extern "C" fn(*mut ANativeActivity, *mut usize) -> *mut c_void>,
    on_pause: ActivityCb,
    on_stop: ActivityCb,
    on_destroy: ActivityCb,
    on_window_focus_changed: Option<unsafe extern "C" fn(*mut ANativeActivity, c_int)>,
    on_native_window_created: WindowCb,
    on_native_window_resized: WindowCb,
    on_native_window_redraw_needed: WindowCb,
    on_native_window_destroyed: WindowCb,
    on_input_queue_created: Option<unsafe extern "C" fn(*mut ANativeActivity, *mut c_void)>,
    on_input_queue_destroyed: Option<unsafe extern "C" fn(*mut ANativeActivity, *mut c_void)>,
    on_content_rect_changed: Option<unsafe extern "C" fn(*mut ANativeActivity, *const c_void)>,
    on_configuration_changed: ActivityCb,
    on_low_memory: ActivityCb,
}

// ── 共享状态(UI 线程 ↔ 渲染线程;window 存地址〔usize〕使 State 保持 Send+Sync) ────
#[derive(Default)]
struct RenderState {
    window_addr: usize, // ANativeWindow* 地址;0 = 无
    have_window: bool,
    stop: bool,
    finished: bool,   // 渲染线程已完成(不再触 window)
    data_dir: String, // internalDataPath(结果/marker 目录基址)
}

type Shared = (Mutex<RenderState>, Condvar);

/// 从 `activity->instance`(onCreate 存 `Arc::into_raw`)借出一份 Arc 克隆,**不改净引用计数**
/// (from_raw 复原 → clone → into_raw 归还原 raw)。
///
/// # Safety
/// `activity` 有效或 null;其 `instance` 若非空须为本 crate onCreate 存入的 `Arc<Shared>` raw。
unsafe fn shared_ref(activity: *mut ANativeActivity) -> Option<Arc<Shared>> {
    if activity.is_null() {
        return None;
    }
    // SAFETY: activity 由框架担保有效(回调期存活);读 instance 字段。
    let inst = unsafe { (*activity).instance };
    if inst.is_null() {
        return None;
    }
    // SAFETY: instance 为 onCreate 的 Arc::into_raw(Shared);from_raw 复原后 clone 出一份返回,
    // 再 into_raw 归还原 raw —— 净引用计数不变(不释放存储的那一份)。
    unsafe {
        let arc = Arc::from_raw(inst as *const Shared);
        let cloned = Arc::clone(&arc);
        let _ = Arc::into_raw(arc);
        Some(cloned)
    }
}

fn finish(shared: &Shared) {
    let (lock, cv) = shared;
    if let Ok(mut st) = lock.lock() {
        st.finished = true;
        cv.notify_all();
    }
}

// ── NativeActivity 入口 + 回调(每个最外层 catch_unwind,绝不 unwind 过 C 边界) ─────

/// NativeActivity 入口(OS 于 UI 主线程调):注册窗口/销毁回调,spawn 渲染线程,**立即返回**。
///
/// # Safety
/// 由 Android 框架经 `android.app.lib_name` 装载后调用;`activity` 为有效 `ANativeActivity*`
/// (存活至 `onDestroy`);`_saved_state` 生命周期由框架管理(本壳不消费)。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ANativeActivity_onCreate(
    activity: *mut ANativeActivity,
    _saved_state: *mut c_void,
    _saved_state_size: usize,
) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if activity.is_null() {
            return;
        }
        // SAFETY: activity 由框架担保有效;读 internal_data_path、写 callbacks/instance。
        let act = unsafe { &mut *activity };
        let data_dir = if act.internal_data_path.is_null() {
            String::new()
        } else {
            // SAFETY: internal_data_path 为框架提供的 NUL 结尾 C 串(app 内部数据目录)。
            unsafe {
                std::ffi::CStr::from_ptr(act.internal_data_path)
                    .to_string_lossy()
                    .into_owned()
            }
        };
        let shared: Arc<Shared> = Arc::new((
            Mutex::new(RenderState {
                data_dir,
                ..Default::default()
            }),
            Condvar::new(),
        ));
        // 存一份 Arc 到 instance(onDestroy 回收);渲染线程 move 另一份。净引用计数 = 2。
        let stored = Arc::into_raw(Arc::clone(&shared));
        act.instance = stored as *mut c_void;
        if !act.callbacks.is_null() {
            // SAFETY: callbacks 为框架提供的有效 ANativeActivityCallbacks*(回调表);仅写函数指针。
            let cbs = unsafe { &mut *act.callbacks };
            cbs.on_native_window_created = Some(on_native_window_created);
            cbs.on_native_window_destroyed = Some(on_native_window_destroyed);
            cbs.on_destroy = Some(on_destroy);
        }
        std::thread::spawn(move || render_thread(shared));
        logcat(ANDROID_LOG_INFO, "onCreate: glue armed (rurix_vk)");
    }));
}

/// UI 线程:acquire window(保活至 destroyed)→ stash 地址 → notify → 立即返回。
unsafe extern "C" fn on_native_window_created(
    activity: *mut ANativeActivity,
    window: *mut ANativeWindow,
) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: activity 由框架担保有效;shared_ref 借出 Arc 克隆不改净计数。
        let Some(shared) = (unsafe { shared_ref(activity) }) else {
            return;
        };
        if !window.is_null() {
            // SAFETY: window 由框架提供,acquire 增引用计数使其在 onNativeWindowDestroyed 前保活。
            unsafe { ANativeWindow_acquire(window) };
        }
        let (lock, cv) = &*shared;
        if let Ok(mut st) = lock.lock() {
            st.window_addr = window as usize;
            st.have_window = true;
            cv.notify_all();
        }
        logcat(ANDROID_LOG_INFO, "onNativeWindowCreated");
    }));
}

/// UI 线程:置 stop → 有界等待(≤2s)渲染线程停用 window → release(window return 后失效)。
unsafe extern "C" fn on_native_window_destroyed(
    activity: *mut ANativeActivity,
    window: *mut ANativeWindow,
) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: 见 shared_ref 契约。
        let Some(shared) = (unsafe { shared_ref(activity) }) else {
            return;
        };
        let (lock, cv) = &*shared;
        if let Ok(mut st) = lock.lock() {
            st.stop = true;
            cv.notify_all();
            // 有界等待渲染线程 finished(停用 window),≤2s 后无论如何 release(window 即将失效)。
            let (mut st, _timeout) = cv
                .wait_timeout_while(st, std::time::Duration::from_secs(2), |s| !s.finished)
                .unwrap_or_else(|e| {
                    let (g, t) = e.into_inner();
                    (g, t)
                });
            st.have_window = false;
            st.window_addr = 0;
        }
        if !window.is_null() {
            // SAFETY: 与 on_native_window_created 的 acquire 配对;渲染线程已 quiesce(finished 或
            // 2s 超时),不再触 window。release 后 window 失效,此后不再使用。
            unsafe { ANativeWindow_release(window) };
        }
        logcat(ANDROID_LOG_INFO, "onNativeWindowDestroyed");
    }));
}

/// UI 线程:通知停止 + 回收 onCreate 存入的 Arc 引用(仅一次)。
unsafe extern "C" fn on_destroy(activity: *mut ANativeActivity) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if activity.is_null() {
            return;
        }
        // SAFETY: 见 shared_ref 契约;通知渲染线程停止。
        if let Some(shared) = unsafe { shared_ref(activity) } {
            let (lock, cv) = &*shared;
            if let Ok(mut st) = lock.lock() {
                st.stop = true;
                cv.notify_all();
            }
        }
        // SAFETY: 读 instance;若非空为 onCreate 的 Arc::into_raw,from_raw 复原并 drop 回收该份
        // 引用(仅一次;渲染线程若已退出则此为最后一份 → Shared 释放)。置空避重复回收。
        unsafe {
            let inst = (*activity).instance;
            if !inst.is_null() {
                drop(Arc::from_raw(inst as *const Shared));
                (*activity).instance = std::ptr::null_mut();
            }
        }
        logcat(ANDROID_LOG_INFO, "onDestroy");
    }));
}

// ── 渲染线程(present + 可选 compute;Vulkan 只在此,回调线程绝不调 present) ─────────
fn render_thread(shared: Arc<Shared>) {
    let res = catch_unwind(AssertUnwindSafe(|| render_thread_inner(&shared)));
    if res.is_err() {
        logcat(
            ANDROID_LOG_ERROR,
            "render thread panicked (吞掉,不 unwind 过边界)",
        );
    }
    // 无论正常 / panic 均置 finished,使 onNativeWindowDestroyed 不空等满 2s。
    finish(&shared);
}

fn render_thread_inner(shared: &Shared) {
    let (lock, cv) = shared;
    let (window_addr, data_dir) = {
        let guard = match lock.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        let guard = cv
            .wait_while(guard, |s| !s.have_window && !s.stop)
            .unwrap_or_else(|e| e.into_inner());
        if !guard.have_window {
            return; // stop 先到,无 window。
        }
        (guard.window_addr, guard.data_dir.clone())
    };
    let window = window_addr as *mut ANativeWindow;
    let json = run_once(window, &data_dir);
    write_result(&data_dir, &json);
    logcat(ANDROID_LOG_INFO, &format!("RESULT {json}"));
}

/// 单次:读 mode → (可选 compute saxpy)→ present N 帧 → 结构性像素校验 → result json 串。
fn run_once(window: *mut ANativeWindow, base: &str) -> String {
    let mode = read_mode(base);
    let red = mode == "red";
    let (vs_b, fs_b, saxpy_b) = rurix_rt::vk::demo_shaders_spv();

    // compute 腿(可选,四要素 coherence):saxpy 断言 max_err;NaN = 未跑 / 降级。
    let max_err = run_compute_leg(saxpy_b);

    // present 腿:居中三角形(NDC),全屏 extent 由 surface currentExtent 决定(入参 64×64 忽略)。
    let vs = bytes_to_words(vs_b);
    let fs = bytes_to_words(fs_b);
    let (vertices, attrs) = triangle_geometry();
    let clear = [0.0f32, 0.0, 0.0, 1.0]; // 背景黑(A=1)
    // SAFETY: window 为 on_native_window_created 经 ANativeWindow_acquire 保活的有效句柄;渲染
    // 线程在 finished(present 返回)前独占使用,on_native_window_destroyed 有界等待 finished 后才
    // release —— present 调用全程 window 有效(U28 window 生命周期契约)。
    let res = unsafe {
        rurix_rt::vk::run_graphics_present_android_safe(
            window, &vs, &fs, &vertices, 32, &attrs, 64, 64, clear, 3, true, red,
        )
    };

    match res {
        Ok((pixels, ew, eh)) => {
            let (covered, corner_bg, center_covered) = analyze(&pixels, ew, eh);
            json_record(
                &mode,
                true,
                3,
                ew,
                eh,
                covered,
                corner_bg,
                center_covered,
                0,
                max_err,
            )
        }
        Err(e) => {
            logcat(ANDROID_LOG_ERROR, &format!("present err: {e}"));
            // fail-closed:validation 触发的 Err 记 ≥1(精确逐条 VUID 在 logcat RurixVK-VVL)。
            let verr = if e.contains("VK_LAYER_KHRONOS_validation") {
                1
            } else {
                0
            };
            json_record(&mode, false, 0, 0, 0, 0, false, false, verr, max_err)
        }
    }
}

/// 可选 compute 腿:saxpy = a*x + out,回读 max_err(镜像 vk_saxpy)。降级 / 失败 → NaN。
fn run_compute_leg(saxpy_b: &[u8]) -> f32 {
    if saxpy_b.is_empty() {
        return f32::NAN;
    }
    let words = bytes_to_words(saxpy_b);
    let Some(entry) = rurix_rt::vk::entry_point_name(&words) else {
        return f32::NAN;
    };
    let n: u32 = 1024;
    let a: f32 = 2.0;
    let x: Vec<f32> = (0..n).map(|i| i as f32).collect();
    let out0: Vec<f32> = (0..n).map(|i| i as f32 * 0.5).collect();
    // buffer binding0=out(in/out),binding1=x;push constant a(f32 @0)+n(u32 @4)。
    let mut buffers = vec![f32s_to_bytes(&out0), f32s_to_bytes(&x)];
    let mut pc = Vec::new();
    pc.extend_from_slice(&a.to_le_bytes());
    pc.extend_from_slice(&n.to_le_bytes());
    use rurix_rt::backend::{BackendKind, ComputeJob, run_job};
    let mut job = ComputeJob {
        artifact: saxpy_b,
        entry: &entry,
        buffers: &mut buffers,
        scalars: &pc,
        groups: [n, 1, 1],
        block: [1, 1, 1],
    };
    if run_job(BackendKind::Vulkan, &mut job).is_err() {
        return f32::NAN;
    }
    let out = bytes_to_f32(&buffers[0]);
    let mut max_err = 0.0f32;
    for i in 0..n as usize {
        max_err = max_err.max((out[i] - (a * x[i] + out0[i])).abs());
    }
    max_err
}

/// 结构性像素校验 @ 真实 extent:covered(非背景像素数)、corner(左下角应 bg 且 A=255)、
/// center(应非背景)。背景判定对通道序不敏感(黑 = RGB 全零)。
fn analyze(pixels: &[u8], ew: u32, eh: u32) -> (usize, bool, bool) {
    let need = (ew as usize) * (eh as usize) * 4;
    if ew == 0 || eh == 0 || pixels.len() < need {
        return (0, false, false);
    }
    let px = |x: u32, y: u32| -> (u8, u8, u8, u8) {
        let o = ((y * ew + x) * 4) as usize;
        (pixels[o], pixels[o + 1], pixels[o + 2], pixels[o + 3])
    };
    let is_bg = |p: (u8, u8, u8, u8)| p.0 == 0 && p.1 == 0 && p.2 == 0;
    let mut covered = 0usize;
    for y in 0..eh {
        for x in 0..ew {
            if !is_bg(px(x, y)) {
                covered += 1;
            }
        }
    }
    let corner = px(0, eh - 1);
    let center = px(ew / 2, eh / 2);
    let corner_bg = is_bg(corner) && corner.3 == 255;
    let center_covered = !is_bg(center);
    (covered, corner_bg, center_covered)
}

#[allow(clippy::too_many_arguments)]
fn json_record(
    mode: &str,
    present_ok: bool,
    frames: u32,
    ew: u32,
    eh: u32,
    covered: usize,
    corner_bg: bool,
    center_covered: bool,
    verr: u32,
    max_err: f32,
) -> String {
    let max_err_s = if max_err.is_nan() {
        "null".to_string()
    } else {
        format!("{max_err}")
    };
    format!(
        "{{\"mode\":\"{mode}\",\"present_ok\":{present_ok},\"frames\":{frames},\"ext_w\":{ew},\"ext_h\":{eh},\"covered\":{covered},\"corner_bg\":{corner_bg},\"center_covered\":{center_covered},\"validation_errors\":{verr},\"max_err\":{max_err_s}}}"
    )
}

/// 读 mode marker(`red`/`green`);internalDataPath 语义跨版本差异 → 两候选路径皆试。默认 green。
fn read_mode(base: &str) -> String {
    for cand in [
        format!("{base}/files/rurix_mode"),
        format!("{base}/rurix_mode"),
    ] {
        if let Ok(s) = std::fs::read_to_string(&cand) {
            match s.trim() {
                "red" => return "red".to_string(),
                "green" => return "green".to_string(),
                _ => {}
            }
        }
    }
    "green".to_string()
}

/// 写 result json(主 `base/present_result.json`;若存在 `base/files` 亦写一份,兼容
/// `run-as … cat files/present_result.json` 惯例)。best-effort,吞错。
fn write_result(base: &str, json: &str) {
    let _ = std::fs::write(format!("{base}/present_result.json"), json);
    let files_dir = format!("{base}/files");
    if std::path::Path::new(&files_dir).is_dir() {
        let _ = std::fs::write(format!("{files_dir}/present_result.json"), json);
    }
}

/// 居中三角形几何(镜像 vk_present):每顶点 pos(vec4)@0 + color(vec4)@16,stride 32。
fn triangle_geometry() -> (Vec<u8>, [(u32, u32, u32); 2]) {
    const FMT_R32G32B32A32_SFLOAT: u32 = 109;
    fn push_vec4(buf: &mut Vec<u8>, v: [f32; 4]) {
        for f in v {
            buf.extend_from_slice(&f.to_le_bytes());
        }
    }
    let mut vertices: Vec<u8> = Vec::with_capacity(3 * 32);
    push_vec4(&mut vertices, [0.0, 0.7, 0.0, 1.0]); // v0 pos(上)
    push_vec4(&mut vertices, [1.0, 0.0, 0.0, 1.0]); // v0 color R
    push_vec4(&mut vertices, [-0.7, -0.7, 0.0, 1.0]); // v1 pos(左下)
    push_vec4(&mut vertices, [0.0, 1.0, 0.0, 1.0]); // v1 color G
    push_vec4(&mut vertices, [0.7, -0.7, 0.0, 1.0]); // v2 pos(右下)
    push_vec4(&mut vertices, [0.0, 0.0, 1.0, 1.0]); // v2 color B
    (
        vertices,
        [
            (0, FMT_R32G32B32A32_SFLOAT, 0),  // location 0 = pos @0
            (1, FMT_R32G32B32A32_SFLOAT, 16), // location 1 = color @16
        ],
    )
}

fn bytes_to_words(b: &[u8]) -> Vec<u32> {
    b.chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn f32s_to_bytes(v: &[f32]) -> Vec<u8> {
    let mut b = Vec::with_capacity(v.len() * 4);
    for f in v {
        b.extend_from_slice(&f.to_le_bytes());
    }
    b
}

fn bytes_to_f32(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}
