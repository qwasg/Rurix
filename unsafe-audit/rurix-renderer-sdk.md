# unsafe-audit: rurix-renderer-sdk(渲染器 SDK C ABI 实现层)

> 注册依据:AGENTS.md 硬规则 9 / 10 §7.6(无注册条目的 unsafe 是 CI 错误);
> 14 §2 常驻集 unsafe-audit 完整性。G31+ 波 C Task C1(G31_PLUS_COMMERCIAL_RENDERER_TODO
> §5 #48「渲染器 SDK 稳定 API 面」):`rxsdk_*` 会话面 = Rurix 渲染器首个 stable
> 嵌入 ABI 的实现层(u64 不透明句柄表 + i32 状态码错误面),薄封装 G14.3 统一四
> pass TSR 生产车道(`g14_3_lane_body.rs` 经 `include!` 逐字第三消费方共享,共享体
> 0-byte——其 Vulkan FFI 义务归 [`rurix-rt.md`](rurix-rt.md) U32/U30 既有注册,
> 本 crate 经 safe API 消费不重复其义务)。.rx stable API 面
> (`apps/g31-renderer-sdk/src/sdk.rx`)经 `#[link] extern` 绑定本 cdylib,export_c
> codegen 产生成头(单一事实源,RXS-0253)。沿 U25(rurix-rt-cabi)审计模式。

## 范围与豁免

- crate:`src/rurix-renderer-sdk`(`[lints.rust] unsafe_code = "allow"`;
  `undocumented_unsafe_blocks = "deny"` 维持——每个 unsafe 块 / `unsafe impl` 强制
  `// SAFETY:`)。
- 全仓其余 crate 维持 `unsafe_code = "deny"`(根 workspace 默认),不受影响。
- unsafe 全部集中于 **C ABI 导出属性面**、**调用方指针契约边界**(裸指针 →
  切片视图/出参写)、**!Send 会话跨线程存表豁免**、**会话资源 `Box::leak`
  'static 化与逆序回收** 四类;C ABI 入口保持 **safe `extern "C"` 签名**(裸
  指针解引用契约见下表,函数级文档注释载调用方前置条件)。
- 失败语义:状态码 + stderr 确定性诊断 `RXSDK: error op=<op> detail=<...>`,
  **不 panic 越过 C ABI**(装载/帧路经 `catch_unwind` 收口;共享体 CLI `fail`
  (process::exit)路径经 SDK 前置校验不可达——SPV/资产/契约缺件在
  `UnifiedLaneBits::load`/`assemble_scene` 之前确定性拒,登记于
  apps/g31-renderer-sdk/API_VERSIONING.md 已知限制)。

## 原语清单与验证义务(RustBelt 式)

| # | 原语 | 位置 | 验证义务(SAFETY 不变量) |
|---|---|---|---|
| U-59 | C ABI 导出属性 `#[unsafe(no_mangle)] pub extern "C" fn rxsdk_*`(cdylib 符号面) | `src/rurix-renderer-sdk/src/lib.rs` `mod sdk` 全部 10 个入口 | 符号以 `rxsdk_` 前缀唯一,与既有 C ABI 导出(`rxrt_*`/`rxp_*`/`rxio_*` RXS-0194/0197/0199、`rurix_uc01_*` RXS-0125、`rurix_engine_*` RXS-0149、UC-05 `uc05_*` 导出面)不冲突(no_mangle 符号唯一性);签名 = 标量 + 裸指针(export_c subset v1 全合规——u64 句柄/标量按值/`*const u8`/`*const f32`/`*mut f64`/`*mut u8`/`*mut u32`),句柄 `0` = 无效/失败;运行期失败 → stderr 确定性诊断 + i32 状态码(0/2/3/4/5/6/7 闭集),不 panic 越过 C ABI(`catch_unwind` 收口于 load_scene/render_frame 两 GPU 路) |
| U-59 | 调用方指针契约边界(`from_raw_parts` 路径/场景 ID 字节串 + 相机 f32 三元组;`copy_nonoverlapping` digest 写;`*mut` 出参写) | `lib.rs` `str_arg` / `f32x3_arg` / `render_frame_inner` | 指针一律先判 null;`(ptr,len)` 路径串 `len ∈ (0,4096]` 且指向 `len` 字节有效可读主机内存且调用期存活(生成头随声明交付该前置条件,RXS-0251 §4.A6 documented unsafe FFI 边界),非 UTF-8 确定性拒;相机三元组指向 3 个 f32 有效可读内存,非有限分量拒;`out_digest` 指向 `digest_cap ≥ 71` 字节有效可写内存(`n == 71 == bytes.len()` 闭集核验后 `copy_nonoverlapping`,源/目的无别名);`out_frame_ms`/`out_digest_len` 单槽出参 null 先于一切解引用核验;借用不越出本函数调用域 |
| U-59 | `!Send` 会话跨线程存表豁免(`unsafe impl Send for Session`) | `lib.rs` 句柄表包装 | Session 持裸指针(leak 回收面)与 Vulkan 车道对象(`UnifiedTsrLane`/`DeviceFrameSession`,进程级驱动对象——镜像 U25 ④/U13 论证);句柄表 `Mutex` 全程互斥(含 GPU 工作段——v1 会话间串行化,如实登记于 crate 文档),存表仅 move 语义、无跨线程共享 `&`(仅 `Send` 豁免,不豁免 `Sync`);Mutex poison 面经 `into_inner` 确定性恢复,不 panic |
| U-59 | 会话资源 `Box::leak` 'static 化与逆序回收(四件套:LaneAssets/UnifiedLaneBits/UnifiedDescs/blas 引用表) | `lib.rs` `load_scene_inner` / `impl Drop for Session` | 自引用结构(`UnifiedTsrLane<'a>` 借用 descs/assets/bits + blas 引用表)经 leak 打平;回收指针一律取自 `&'static mut` 的 `*mut` 面(`&T as *mut T` 非法,编译期钉死);回收序 = lane 先析构(DeviceFrameSession/Vulkan 资源不再引用 leak 面)后四件套 `Box::from_raw` 重建所有权释放,每指针恰一次(双 destroy 由句柄表移除语义拦截——句柄出表后 `Box<Session>` 恰一次 drop);lane 创建失败臂在同一函数内即回收,`Session::leak_*` 保持 null,Drop 不双释 |
