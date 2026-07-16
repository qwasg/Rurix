# unsafe-audit: rurix-android-present(NativeActivity glue FFI 边界)

> 注册依据:AGENTS.md 硬规则 9 / 10 §7.6(无注册条目的 unsafe 是 CI 错误);
> 14 §2 常驻集 unsafe-audit 完整性。mb1 W7 激活(G-MB1-7 Phase B:Android on-device
> present 尾门兑现,零-Java NativeActivity cdylib 壳)。决策依据:RFC-0011 §4.6(Vulkan/SPIR-V
> 跨端后端,Android libvulkan.so 消费同一 `.spv`)、D-113(FFI 战略:`extern "system"`/`extern "C"`
> + `#[repr(C)]` + 原始指针)。mb1 契约 `rfc_required: none`(红线 3 已 owner 解除 + RFC-0011
> 批准),会话授权直接实现 + 块级豁免(不另走 RFC)。spec:无新条款(承 RXS-0210/0211,复用
> 既有 Vulkan 出图 + android surface 缝;glue 不入 trace `gather_repo`)。

## 范围与豁免

- crate:`src/rurix-android-present`(`[lints.rust] unsafe_code = "allow"`;`undocumented_unsafe_blocks
  = "deny"` 维持——每个 unsafe 块强制 `// SAFETY:` 注释)。**桌面(非 android)整 crate
  `#![cfg(target_os = "android")]` 为空**(target-cfg 依赖不激活,`unsafe_code` 豁免不触桌面)。
- 全仓其余新 crate 维持 `unsafe_code = "deny"`(根 workspace 默认),不受影响。
- **本 crate 零 Vulkan unsafe**:所有 Vulkan FFI 在 `rurix-rt`(vk.rs U26/U27,含 android present
  `run_graphics_present_android`,见 [`rurix-rt.md`](rurix-rt.md));本 crate 仅 NativeActivity ABI +
  ANativeWindow 生命周期 + 跨-FFI panic 边界 + 渲染线程 FFI —— **独立** unsafe 边界(NativeActivity
  ABI 契约、window acquire/release 配对、`extern "C"` 回调不得 unwind 过 C 边界)故诚实登记新号 U28
  (不作 U27 扩注)。
- **不设 `panic = "abort"`**:保每个 `extern "C"` 边界最外层 `catch_unwind` 能吞 panic 落结果行
  (abort 杀进程丢结果;unwind + catch 保证 FAIL 结果协议总能落盘)。

## 原语清单与验证义务(RustBelt 式)

| # | 原语 | 位置 | 验证义务(SAFETY 不变量) |
|---|---|---|---|
| U28 | NativeActivity ABI + ANativeWindow 生命周期 + 跨-FFI panic 边界 + 渲染线程 FFI(`ANativeActivity_onCreate` 导出 + `on_native_window_{created,destroyed}`/`on_destroy` 回调 + `ANativeWindow_acquire`/`ANativeWindow_release` + `__android_log_write` + 渲染线程经 `Arc<(Mutex<RenderState>, Condvar)>` 共享 + present 前向 rurix-rt) | `src/rurix-android-present/src/lib.rs`(cdylib `librurix_vk.so`,仅 android target) | ① **ABI 逐字节对齐**:`ANativeActivity` / `ANativeActivityCallbacks` `#[repr(C)]` 与 `android/native_activity.h` 字段序/尺寸一致(仅读 `callbacks`/`internal_data_path`/`instance`,余字段占位定偏移;由框架真跑调回调实证);`ANativeWindow` 复用 rurix-rt `android_present::ANativeWindow`(同一 opaque 类型,免跨 crate 转换)。② **回调不 unwind 过 C 边界**:每个 `extern "C"`(入口 + 3 回调)最外层 `std::panic::catch_unwind(AssertUnwindSafe(..))`,捕获即 `__android_log_write(ERROR)` 吞掉,绝不 unwind 过 NativeActivity C 边界(**不设 `panic=abort`**,保结果落盘)。③ **ANativeWindow 生命周期**:`on_native_window_created` 内 `ANativeWindow_acquire`(增引用计数保活)⇔ `on_native_window_destroyed` 内 `ANativeWindow_release` **线性配对**;destroyed 先置 `stop` + **有界等待**(`Condvar::wait_timeout_while` ≤2s)渲染线程 `finished`(停用 window)**再** release,故 present 期 window 恒有效(window `release` 后失效,此后不再触)。④ **Arc 引用计数单点**:`onCreate` `Arc::into_raw` 存 `activity->instance`(净计数 +1)+ 渲染线程 `move` 另一份;回调经 `shared_ref`(`from_raw`→`clone`→`into_raw` 归还)借出**不改净计数**;`onDestroy` `Arc::from_raw` 回收存储那份**仅一次**(置空 `instance` 防重复),Shared 于最后一份 Drop 时释放。⑤ **共享态 Send+Sync**:window 存 `usize` 地址(非裸指针)使 `RenderState` 保持 `Send+Sync`,`Arc<Shared>` 跨线程合法;present 只在自建渲染线程(UI 回调 O(1) 返回防 ANR,**回调线程绝不调 present**)。⑥ **前向 rurix-rt safe API**:present = `vk::run_graphics_present_android_safe`(内层 U27 android 扩注管句柄逆序销毁 + messenger fail-closed)、compute = `backend::run_job`(U26 委托);`window` 裸指针仅前向不在本层解引用。⑦ **liblog / libnativewindow FFI**:`__android_log_write`/`ANativeWindow_*` 入参为 NUL 结尾 C 串 / 有效 window 句柄,由系统库(build.rs `-llog -landroid -lnativewindow`)提供,只读不持有 |

## 销毁纪律

- **window**:acquire(created)⇔ release(destroyed after quiesce)线性配对,仅一次;release 前有界
  等待渲染线程 `finished` 保证无并发使用(≤2s 超时兜底,防 present 挂起无限阻塞 UI)。
- **Arc<Shared>**:`onCreate` 建两份引用(instance 存储 + 渲染线程),`onDestroy` 回收存储份仅一次,
  渲染线程份于线程退出 Drop;Shared(Mutex/Condvar)于最后一份释放,无泄漏、无双重释放。
- **无 Vulkan 资源所有权**:instance/device/surface/swapchain 全在 `run_graphics_present_android`
  内线性 create/destroy(rurix-rt U27),本 crate 不持有、不 Drop。

## 测试

- `cargo build -p rurix-android-present --target aarch64-linux-android --release`:交叉链接绿 + 产物
  `librurix_vk.so`(导出 `ANativeActivity_onCreate`;NEEDED liblog/libandroid;**不链接 libvulkan**——
  运行时 dlopen)。桌面 `cargo build --workspace` 为空 lib(零回归)。
- on-device RED/GREEN 真跑(present N 帧结构性校验 + `VK_LAYER_KHRONOS_validation` 零报错 / RED 反证
  VUID)= 硬件尾门 **G-MB1-7**,证据归档待 owner 裁签(无 android runner,不设 CI 硬门;**不伪造、不
  自签**)。scratch 打包/设备序列见 `…/scratchpad/mb1-apk/`(工具件不入库)。
