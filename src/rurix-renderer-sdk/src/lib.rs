//! 渲染器 SDK C ABI 实现层（G31+ 波 C Task C1，G31_PLUS_COMMERCIAL_RENDERER_TODO §5 #48）。
//!
//! `rxsdk_*` 会话面 = Rurix 渲染器首个 **stable 嵌入 ABI** 的实现层：u64 不透明
//! 句柄表（进程级 `Mutex<HashMap>`；句柄 `0` = 无效/失败）+ i32 状态码错误面
//! （RD-026 无 Result 面纪律），薄封装 G14.3 统一四 pass TSR 生产车道
//! （`g14_3_lane_body.rs` 经 `include!` 逐字共享——第三消费方，两 bin
//! （g14_3_pipeline_perf / g31_window_present）先例；共享体 0-byte，digest 锚定
//! 逻辑禁动）。.rx stable API 面（`apps/g31-renderer-sdk/src/sdk.rx`）以
//! `#[link(name = "rurix_renderer_sdk")] extern "C"` 逐字转发本面，export_c
//! codegen 产 `rurix_renderer.dll` + import lib + 生成头（单一事实源，RXS-0253）。
//!
//! ## 语义面（v1 = 1.0.0；版本政策见 apps/g31-renderer-sdk/API_VERSIONING.md）
//!
//! - **初始化（设备/能力协商）**：[`rxsdk_renderer_create`] 探测 Vulkan loader
//!   （`vk::vulkan_available`）；完整能力链（ray query 四扩展 + sync2 等）在
//!   [`rxsdk_renderer_load_scene`] 的 `UnifiedTsrLane::create` 期 fail-closed
//!   （render_exec P-01 纪律：缺一 → 确定性 Err → 状态码 5，不降级不静默）。
//! - **场景提交（资产/契约路径）**：`load_scene` 消费 bistro 生产契约（严格
//!   解析 + digest 门 == FROZEN，与 bench 腿同一 fail-closed 语义）+ gltf 场景
//!   装配 + SPV 四件套（g14_3_direct_gi / g14_mv / g14_8_tsr_{resample,resolve}，
//!   缺件在 `UnifiedLaneBits::load` 之前确定性拒——共享体的 CLI fail 路径
//!   （process::exit）不经 SDK 面可达，见 API_VERSIONING.md 已知限制登记）。
//! - **帧循环**：`render_frame` 逐帧推进与 bench 腿**逐字同式**的序列
//!   （jitter_base = seed % 65521、`halton(base + i + 1)` 双维、首帧 reset、
//!   TSR 历史链；`readback != 0` 帧回读 TSR 输出算 `frame_content_digest`）——
//!   canonical 160 帧 + warmup 10 序列末帧 digest 与 Stage A 锚
//!   `bistro-interior_t100_tsr_device` 位级对拍（机器证明 SDK 面 = 生产管线）。
//! - **参数更新**：`set_camera`（eye/forward/up/fov_y_rad/near/far 全量替换
//!   CameraSpec，逐帧 `build_vp` + `jittered_vp` 同一 192B 帧参数 uniform 通路）
//!   / `set_exposure_ev100`（128B TSR 参数通路）——与 G31 游戏循环同一 uniform
//!   通道，帧间生效、确定性。
//! - **present 句柄**：v1 = 离屏会话呈现完成计数（`rxsdk_renderer_present`
//!   计数 + 成功；真窗口 swapchain present 归后续 MINOR 加性，不冒充）。
//! - **关闭**：`destroy` 逆序回收（lane/DeviceFrameSession → leak 三件套），
//!   双 destroy / 无效句柄 → 状态码 2 不崩。
//! - **G35-8 粒子作者面（加性,2026-08-27;RFC-0049 §4.11 冻结四签名）**：
//!   `rxsdk_particles_emitter_create / rxsdk_particles_emitter_set_param /
//!   rxsdk_particles_emitter_destroy / rxsdk_particles_stats`——u64 句柄 +
//!   i32 状态码复用既有闭集语义;单线程 apartment 同上。**v1 粒子臂 = host
//!   臂**：emitter 资产（JSON 十字段闭集,`rurix_render::particles::
//!   emitter_asset` fail-closed 解析）→ host 金标准粒子系统
//!   （`particles::core::frame`,每 emitter 独立池,SDK_EMITTER_CAP=4096/
//!   dt=1/60/seed=SDK_PARTICLE_SEED 冻结）随 `rxsdk_renderer_render_frame`
//!   每成功帧 tick 一帧;`rxsdk_particles_stats` 写 alive_total(u64)。
//!   device 车道接线归 G35 收口批（登记见 apps/g31-renderer-sdk/
//!   API_VERSIONING.md §6,不冒充）。既有导出/句柄语义 0-byte：无 emitter
//!   时 render_frame 行为与前版逐字一致（tick 空转不触池）;用户面
//!   （sdk.rx 导出集/生成头/ABI 版本 1.0.0）本批 0-byte——`rurix_renderer_
//!   particles_*` 薄转发 + MINOR 1.1.0 + stable 快照重 bless 归收口批同批
//!   执行（五面 1.0.0/9 导出字面冻结于 g31.waveC 门族,见 §6 登记）。
//!
//! ## 错误口径（P-01 fail-closed，镜像 RXS-0193）
//!
//! 状态码：0 = 成功；2 = 句柄无效/状态错序/双 destroy；3 = 输入越界（长度/容量/
//! tier 闭集/契约 digest 不等/指针空）；4 = 资产面缺失（契约/gltf/SPV 读不到）；
//! 5 = device/能力链缺失（Vulkan loader / ray query 链 / 车道创建失败）；6 =
//! 渲染执行失败（含输出非有限）；7 = validation ERROR 计数非零。任何失败落
//! stderr 确定性诊断一行 `RXSDK: error op=<op> detail=<...>` 后返回状态码，
//! **不 panic 越过 C ABI**（帧/装载路经 `catch_unwind` 收口）。
//!
//! # SAFETY（U-59；沿 U25 审计模式，注册见 unsafe-audit/rurix-renderer-sdk.md）
//!
//! unsafe 全部集中于：① C ABI 导出属性面（`#[unsafe(no_mangle)] extern "C"`，
//! 符号 `rxsdk_` 前缀唯一）；② 调用方指针契约边界（裸指针 → 切片/CStr 视图，
//! 一律先判 null、长度/容量在解引用前确定性核）；③ `!Send` 会话跨线程存表豁免
//! （句柄表 Mutex 全程互斥，存表仅 move 语义）；④ 会话资源 `Box::leak` 'static
//! 化与 destroy 逆序回收（lane 先析构 → 引用面后回收，单点单回收）。

// 共享体含本 crate 未消费面（render/bench 腿、dlss/fsr 双臂、EXR/PNG 出图、
// slab/tex/skin/dyn/RESTIR/HZB 车道等）——与两 bin 消费方同一豁免纪律（dead_code
// 豁免如实登记）；共享体 lint 面归其 bin 消费方既有纪律，crate 级 clippy::all
// 豁免仅覆盖共享体代码位置，本 crate 自有 SDK 码经 `mod sdk` 内层 re-deny 恢复
// clippy 门禁（lint 按代码位置归属，豁免不外溢）。
#![allow(dead_code, unused_imports, unused_variables, clippy::all)]

// device 执行面全部内容(车道共享体 include! + rxsdk_* 面)随 feature
// `sdk-device` 编译(默认空骨架——常驻回归网 clippy/test 绿,与 rurix-render
// bins required-features 跳过同律;Cargo.toml [features] 段纪律注释)。
#[cfg(feature = "sdk-device")]
include!("../../rurix-render/src/bin/g14_3_lane/g14_3_lane_body.rs");

#[cfg(feature = "sdk-device")]
#[deny(clippy::all)]
mod sdk {
    //! `rxsdk_*` 导出面与会话实现（自有码 clippy 门禁恢复面）。

    use std::collections::{BTreeMap, HashMap};
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// ABI 版本（语义化版本打包：`major << 16 | minor << 8 | patch`；v1 = 1.0.0）。
    /// 与 apps/g31-renderer-sdk/src/sdk.rx `rurix_renderer_abi_version` 返回字面、
    /// API_VERSIONING.md、stable 快照 renderer_sdk 段四面同一字面（单一事实源 =
    /// 本常量；其余三面程序读/逐字转发）。
    #[allow(clippy::identity_op)] // 打包布局自文档化（minor/patch 位恒 0 亦显式占位；stable_snapshot ABI_VERSION_RE 按本式程序读）
    pub const ABI_VERSION_PACKED: u32 = (1 << 16) | (0 << 8) | 0;

    // ── 状态码闭集（文档级契约；见 crate 级文档「错误口径」段）──
    const ST_OK: i32 = 0;
    const ST_HANDLE: i32 = 2;
    const ST_INPUT: i32 = 3;
    const ST_ASSET: i32 = 4;
    const ST_DEVICE: i32 = 5;
    const ST_RENDER: i32 = 6;
    const ST_VALIDATION: i32 = 7;

    /// canonical SPV 四件套文件名（生产车道 kernel 编译产物；路径由宿主给目录）。
    const SPV_NAMES: [&str; 4] = [
        "g14_3_direct_gi.spv",
        "g14_mv.spv",
        "g14_8_tsr_resample.spv",
        "g14_8_tsr_resolve.spv",
    ];

    /// digest 串面：`sha256:` 前缀 7 + 64 hex = 71 字节（无 NUL，长度经出参回）。
    const DIGEST_BYTES: u32 = 71;

    /// 会话状态机（Created → SceneLoaded；错序调用 = ST_HANDLE 确定性拒）。
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum State {
        Created,
        SceneLoaded,
    }

    /// SDK 会话（u64 句柄表载荷）。车道/场景资源经 `Box::leak` 'static 化——
    /// `UnifiedTsrLane<'a>` 借用 descs/assets/bits 的自引用结构经 leak 打平，
    /// destroy/Drop 逆序单点回收（SAFETY 见 unsafe-audit U-59 ④）。
    struct Session {
        state: State,
        backend: u32,
        /// 场景装配产物（契约相机/曝光/几何/lights 的持有面；车道输入面）。
        scene: Option<crate::SceneData>,
        /// 当前相机/曝光参数（set_camera/set_exposure_ev100 逐帧生效面）。
        camera: crate::CameraSpec,
        ev100: f32,
        eps: f32,
        out_w: u32,
        out_h: u32,
        in_w: u32,
        in_h: u32,
        seed: u64,
        frame_index: u32,
        presented: u64,
        /// 生产车道（load_scene 成功后才存在；'static = leak 三件套借用面）。
        lane: Option<crate::UnifiedTsrLane<'static>>,
        /// leak 回收指针（仅 destroy/Drop 消费；非 null 当且仅当 lane 曾建成）。
        leak_assets: *mut crate::LaneAssets,
        leak_bits: *mut crate::UnifiedLaneBits,
        leak_descs: *mut crate::UnifiedDescs<'static>,
        leak_blas: *mut [&'static [f32]; 1],
    }

    // SAFETY: Session 持裸指针（leak 回收面）与 Vulkan 车道对象，原生 !Send；
    // 句柄表 `Mutex` 全程互斥保证同刻单线程访问，存表仅 move 语义、无跨线程
    // 共享 `&`（仅 Send 豁免，不豁免 Sync）；leak 指针指向进程生命期有效的堆
    // 分配，回收仅 destroy/Drop 单点一次（镜像 U25 ④ 论证）。
    unsafe impl Send for Session {}

    impl Drop for Session {
        fn drop(&mut self) {
            // 逆序回收：lane（DeviceFrameSession/Vulkan 资源，引用 leak 面）先
            // 析构，leak 三件套后回收；双 destroy 已由句柄表移除语义拦截（句柄
            // 出表后 Box 恰一次 drop）。
            if let Some(lane) = self.lane.take() {
                drop(lane);
            }
            if !self.leak_descs.is_null() {
                // SAFETY: 四指针均来自 load_scene 的 `Box::leak`（同 crate 单点
                // 分配的 `*mut` 面），非 null 即未被回收（lane 已先析构，不再
                // 引用其指向面）；`Box::from_raw` 重建所有权即 drop 释放，每
                // 指针恰一次。
                unsafe {
                    drop(Box::from_raw(self.leak_descs));
                    drop(Box::from_raw(self.leak_assets));
                    drop(Box::from_raw(self.leak_bits));
                    drop(Box::from_raw(self.leak_blas));
                }
                self.leak_descs = std::ptr::null_mut();
                self.leak_assets = std::ptr::null_mut();
                self.leak_bits = std::ptr::null_mut();
                self.leak_blas = std::ptr::null_mut();
            }
        }
    }

    static SESSIONS: std::sync::LazyLock<Mutex<HashMap<u64, Box<Session>>>> =
        std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));
    static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

    fn diag(op: &str, detail: &str) {
        eprintln!("RXSDK: error op={op} detail={detail}");
    }

    fn sessions_lock() -> std::sync::MutexGuard<'static, HashMap<u64, Box<Session>>> {
        // poison 面：panic 只可能发生于持锁内的车道路（catch_unwind 已收口）；
        // into_inner 恢复确定性访问，不 panic 越过 C ABI。
        SESSIONS.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// 调用方 `(ptr, len)` → UTF-8 `&str` 视图（路径/场景 ID；null/越界/非
    /// UTF-8 在解引用前确定性拒）。
    fn str_arg<'a>(ptr: *const u8, len: u32, what: &str) -> Result<&'a str, i32> {
        if ptr.is_null() || len == 0 || len > 4096 {
            return Err(ST_INPUT);
        }
        // SAFETY: 调用方契约 = `ptr` 指向 `len` 字节有效可读主机内存且调用期
        // 存活（生成头随声明交付该前置条件，RXS-0251 §4.A6 documented unsafe
        // FFI 边界）；null/len=0/len>4096 已在上方拒绝；借用不越出本函数调用域。
        let bytes = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
        std::str::from_utf8(bytes).map_err(|_| {
            diag("str_arg", &format!("{what} 非 UTF-8"));
            ST_INPUT
        })
    }

    /// 调用方 `*const f32` → 3 元素视图（相机向量；null/非有限在解引用前/后拒）。
    fn f32x3_arg<'a>(ptr: *const f32, what: &str) -> Result<&'a [f32], i32> {
        if ptr.is_null() {
            return Err(ST_INPUT);
        }
        // SAFETY: 调用方契约 = `ptr` 指向 3 个 f32 有效可读主机内存且调用期存活
        // （生成头随声明交付）；null 已在上方拒绝；借用不越出本函数调用域。
        let v = unsafe { std::slice::from_raw_parts(ptr, 3) };
        if !v.iter().all(|x| x.is_finite()) {
            diag("f32x3_arg", &format!("{what} 非有限"));
            return Err(ST_INPUT);
        }
        Ok(v)
    }

    // ─────────────────────────── 版本/能力面 ───────────────────────────

    /// ABI 版本（打包 u32；宿主据此做 MAJOR 兼容裁决——政策见 API_VERSIONING.md）。
    #[unsafe(no_mangle)]
    pub extern "C" fn rxsdk_abi_version() -> u32 {
        ABI_VERSION_PACKED
    }

    /// 能力探测（不建会话的轻量面）：bit0 = Vulkan loader 可用；完整能力链
    /// （ray query 四扩展等）协商归 create/load_scene fail-closed（v1 登记口径）。
    #[unsafe(no_mangle)]
    pub extern "C" fn rxsdk_caps_probe() -> u64 {
        u64::from(crate::vk::vulkan_available())
    }

    // ─────────────────────────── 生命周期 ───────────────────────────

    /// 创建会话（backend 闭集：0 = auto〔v1 唯一后端 = Vulkan〕）。句柄 0 =
    /// 失败（device 面缺失；能力链完整协商归 load_scene 车道创建期）。
    #[unsafe(no_mangle)]
    pub extern "C" fn rxsdk_renderer_create(backend: u32) -> u64 {
        if backend != 0 {
            diag("create", &format!("backend {backend} 越闭集 {{0=auto}}"));
            return 0;
        }
        if !crate::vk::vulkan_available() {
            diag("create", "vulkan loader 不可用");
            return 0;
        }
        let h = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
        if h == 0 {
            diag("create", "句柄空间耗尽");
            return 0;
        }
        let s = Session {
            state: State::Created,
            backend,
            scene: None,
            camera: crate::CameraSpec {
                eye: [0.0; 3],
                forward: [0.0, 0.0, 1.0],
                up0: [0.0, 1.0, 0.0],
                fov_y_rad: 1.0,
                near: 0.1,
                far: 100.0,
            },
            ev100: 0.0,
            eps: 0.0,
            out_w: 0,
            out_h: 0,
            in_w: 0,
            in_h: 0,
            seed: 0,
            frame_index: 0,
            presented: 0,
            lane: None,
            leak_assets: std::ptr::null_mut(),
            leak_bits: std::ptr::null_mut(),
            leak_descs: std::ptr::null_mut(),
            leak_blas: std::ptr::null_mut(),
        };
        sessions_lock().insert(h, Box::new(s));
        h
    }

    /// 关闭会话（逆序回收；无效/双 destroy = ST_HANDLE 确定性拒不崩）。
    /// G35-8 加性：会话名下 emitter 句柄随会话回收（所有权 = 会话产/会话收，
    /// 此后 emitter 句柄悬空 → ST_HANDLE fail-closed）；既有返回码语义 0-byte。
    #[unsafe(no_mangle)]
    pub extern "C" fn rxsdk_renderer_destroy(r: u64) -> i32 {
        match sessions_lock().remove(&r) {
            Some(_) => {
                emitters_lock().retain(|_, e| e.sys != r);
                ST_OK
            }
            None => {
                diag("destroy", &format!("句柄 {r} 无效或已销毁"));
                ST_HANDLE
            }
        }
    }

    // ─────────────────────────── 场景提交 ───────────────────────────

    /// 装载场景（契约严格解析 + digest 门 == FROZEN + gltf 装配 + SPV 四件套 +
    /// 车道创建 fail-closed）。成功 → 状态机进 SceneLoaded，帧序自 0 起。
    #[allow(clippy::too_many_arguments)]
    #[unsafe(no_mangle)]
    pub extern "C" fn rxsdk_renderer_load_scene(
        r: u64,
        contract: *const u8,
        clen: u32,
        gltf: *const u8,
        glen: u32,
        scene: *const u8,
        slen: u32,
        tier: u32,
        spv_dir: *const u8,
        sdlen: u32,
    ) -> i32 {
        let contract_path = match str_arg(contract, clen, "contract") {
            Ok(s) => s,
            Err(c) => return c,
        };
        let gltf_path = match str_arg(gltf, glen, "gltf") {
            Ok(s) => s,
            Err(c) => return c,
        };
        let scene_id = match str_arg(scene, slen, "scene") {
            Ok(s) => s,
            Err(c) => return c,
        };
        let spv_root = match str_arg(spv_dir, sdlen, "spv_dir") {
            Ok(s) => s,
            Err(c) => return c,
        };
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            load_scene_inner(r, contract_path, gltf_path, scene_id, tier, spv_root)
        })) {
            Ok(rc) => rc,
            Err(_) => {
                diag("load_scene", "车道/装配面 panic（catch_unwind 收口）");
                ST_RENDER
            }
        }
    }

    fn load_scene_inner(
        r: u64,
        contract_path: &str,
        gltf_path: &str,
        scene_id: &str,
        tier: u32,
        spv_root: &str,
    ) -> i32 {
        let mut guard = sessions_lock();
        let Some(s) = guard.get_mut(&r) else {
            diag("load_scene", &format!("句柄 {r} 无效"));
            return ST_HANDLE;
        };
        if s.state != State::Created {
            diag(
                "load_scene",
                "状态错序（会话已装载；重建须 destroy 后另建）",
            );
            return ST_HANDLE;
        }

        // ① 契约严格解析 + digest 门 == FROZEN（与 bench 腿同一 fail-closed 语义）。
        let text = match std::fs::read_to_string(contract_path) {
            Ok(t) => t,
            Err(e) => {
                diag("load_scene", &format!("契约读取 {contract_path}: {e}"));
                return ST_ASSET;
            }
        };
        let contract = match crate::parse_contract(&text) {
            Ok(c) => c,
            Err(e) => {
                diag("load_scene", &format!("契约解析: {e}"));
                return ST_INPUT;
            }
        };
        if contract.digest != crate::FROZEN_CONTRACT_DIGEST {
            diag(
                "load_scene",
                &format!(
                    "契约 digest {} ≠ 冻结锚 {}（生产契约面 fail-closed）",
                    contract.digest,
                    crate::FROZEN_CONTRACT_DIGEST
                ),
            );
            return ST_INPUT;
        }
        let srow = match crate::contract_scene_row(&contract.raw, scene_id) {
            Ok(r) => r,
            Err(e) => {
                diag("load_scene", &e);
                return ST_INPUT;
            }
        };
        let (out_w, out_h) = match (|| {
            let res = srow.get("camera")?.get("resolution")?;
            Some((
                res.get("w")?.as_u64()? as u32,
                res.get("h")?.as_u64()? as u32,
            ))
        })() {
            Some(v) => v,
            None => {
                diag("load_scene", "契约场景行 camera.resolution 缺 w/h");
                return ST_INPUT;
            }
        };
        let tiers_ok = contract
            .raw
            .get("tier_sequence")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_u64())
                    .any(|t| t == u64::from(tier))
            })
            .unwrap_or(false);
        if !tiers_ok {
            diag(
                "load_scene",
                &format!("tier {tier} 越契约 tier_sequence 闭集"),
            );
            return ST_INPUT;
        }
        let in_w = ((out_w as u64 * u64::from(tier)) / 100) as u32;
        let in_h = ((out_h as u64 * u64::from(tier)) / 100) as u32;
        if in_w == 0 || in_h == 0 {
            diag("load_scene", "内部分辨率塌零");
            return ST_INPUT;
        }
        let Some(seed) = contract.raw.get("seed").and_then(|v| v.as_u64()) else {
            diag("load_scene", "契约缺 seed");
            return ST_INPUT;
        };

        // ② 场景装配（资产缺失 = ST_ASSET，dev-env 三态面）。
        let scene = match crate::assemble_scene(&contract.raw, scene_id, Path::new(gltf_path)) {
            Ok(s) => s,
            Err(e) => {
                diag("load_scene", &format!("scene_assets: {e}"));
                return ST_ASSET;
            }
        };

        // ③ SPV 四件套预核验（在 UnifiedLaneBits::load 之前确定性拒——共享体
        // CLI fail 路径不经 SDK 面可达）。
        let spv_paths: Vec<PathBuf> = SPV_NAMES
            .iter()
            .map(|n| Path::new(spv_root).join(n))
            .collect();
        for p in &spv_paths {
            if !p.is_file() {
                diag("load_scene", &format!("SPV 缺失 {}", p.display()));
                return ST_ASSET;
            }
        }

        // ④ 车道资源打包 → leak 'static 化（自引用结构打平；destroy 逆序回收）。
        // 回收指针须从 `&'static mut` 直接取（`&T as *mut T` 非法）——leak 返回
        // 可写引用，先取裸指针再 reborrow 共享引用面供车道消费。
        let assets_mut: &'static mut crate::LaneAssets =
            Box::leak(Box::new(crate::lane_assets(&scene, in_w, in_h)));
        let assets_ptr: *mut crate::LaneAssets = assets_mut;
        let assets: &'static crate::LaneAssets = assets_mut;
        let bits_mut: &'static mut crate::UnifiedLaneBits =
            Box::leak(Box::new(crate::UnifiedLaneBits::load(
                spv_paths[0].to_str().unwrap_or(""),
                spv_paths[1].to_str().unwrap_or(""),
                spv_paths[2].to_str().unwrap_or(""),
                spv_paths[3].to_str().unwrap_or(""),
                in_w,
                in_h,
                out_w,
                out_h,
                false,
            )));
        let bits_ptr: *mut crate::UnifiedLaneBits = bits_mut;
        let bits: &'static crate::UnifiedLaneBits = bits_mut;
        let descs_mut: &'static mut crate::UnifiedDescs<'static> =
            Box::leak(Box::new(crate::UnifiedDescs::Mega(
                crate::unified_lane_descs(assets, bits, in_w, in_h, out_w, out_h),
            )));
        let descs_ptr: *mut crate::UnifiedDescs<'static> = descs_mut;
        let descs: &'static crate::UnifiedDescs<'static> = descs_mut;
        // blas 引用表亦为车道借用面（AccelStructDesc<'static> 内容引用）——同为
        // leak 第四件，随三件套同一回收序。
        let blas_mut: &'static mut [&'static [f32]; 1] = Box::leak(Box::new([&assets.tris[..]]));
        let blas_ptr: *mut [&'static [f32]; 1] = blas_mut;
        let blas_refs: &'static [&'static [f32]; 1] = blas_mut;
        let accel_structs = [crate::AccelStructDesc {
            scene: crate::RayQuerySceneDesc {
                blas_triangles: blas_refs,
                instances: &assets.instances,
            },
            transforms: None,
            updatable_blas: &[],
        }];
        let lane = match crate::UnifiedTsrLane::create(descs, &accel_structs, 1) {
            Ok(l) => l,
            Err(e) => {
                diag("load_scene", &format!("device_lane: {e}"));
                // SAFETY: lane 未建成，leak 四件套即在此回收（与 Drop 同一回收
                // 序；指针来自上方 `Box::leak` 的 `*mut` 单点分配面且此后再不
                // 被引用）。
                unsafe {
                    drop(Box::from_raw(descs_ptr));
                    drop(Box::from_raw(assets_ptr));
                    drop(Box::from_raw(bits_ptr));
                    drop(Box::from_raw(blas_ptr));
                }
                return ST_DEVICE;
            }
        };

        s.eps = crate::scene_eps(&scene.positions);
        // CameraSpec 无 Clone/Copy derive——逐字段复制（数组成员 Copy），避免
        // 部分移动 scene（scene 整体随后入 s.scene 持有面）。
        s.camera = crate::CameraSpec {
            eye: scene.camera.eye,
            forward: scene.camera.forward,
            up0: scene.camera.up0,
            fov_y_rad: scene.camera.fov_y_rad,
            near: scene.camera.near,
            far: scene.camera.far,
        };
        s.ev100 = scene.ev100;
        s.out_w = out_w;
        s.out_h = out_h;
        s.in_w = in_w;
        s.in_h = in_h;
        s.seed = seed;
        s.frame_index = 0;
        s.lane = Some(lane);
        s.leak_assets = assets_ptr;
        s.leak_bits = bits_ptr;
        s.leak_descs = descs_ptr;
        s.leak_blas = blas_ptr;
        s.scene = Some(scene);
        s.state = State::SceneLoaded;
        ST_OK
    }

    // ─────────────────────────── 参数更新 ───────────────────────────

    /// 相机全量替换（eye/forward/up 三元组 + fov_y_rad + near/far；与契约相机
    /// 同一 CameraSpec 结构，逐帧 build_vp/jittered_vp 同一 uniform 通路生效）。
    #[allow(clippy::too_many_arguments)]
    #[unsafe(no_mangle)]
    pub extern "C" fn rxsdk_renderer_set_camera(
        r: u64,
        eye: *const f32,
        fwd: *const f32,
        up: *const f32,
        fov_y_rad: f32,
        near: f32,
        far: f32,
    ) -> i32 {
        let eye = match f32x3_arg(eye, "eye") {
            Ok(v) => [v[0], v[1], v[2]],
            Err(c) => return c,
        };
        let fwd = match f32x3_arg(fwd, "forward") {
            Ok(v) => [v[0], v[1], v[2]],
            Err(c) => return c,
        };
        let up = match f32x3_arg(up, "up") {
            Ok(v) => [v[0], v[1], v[2]],
            Err(c) => return c,
        };
        if !(fov_y_rad.is_finite() && fov_y_rad > 0.0 && fov_y_rad < std::f32::consts::PI) {
            diag("set_camera", "fov_y_rad 越域 (0, π)");
            return ST_INPUT;
        }
        if !(near.is_finite() && far.is_finite() && near > 0.0 && near < far) {
            diag("set_camera", "near/far 越域（0 < near < far）");
            return ST_INPUT;
        }
        let mut guard = sessions_lock();
        let Some(s) = guard.get_mut(&r) else {
            diag("set_camera", &format!("句柄 {r} 无效"));
            return ST_HANDLE;
        };
        if s.state != State::SceneLoaded {
            diag("set_camera", "状态错序（须先 load_scene）");
            return ST_HANDLE;
        }
        s.camera = crate::CameraSpec {
            eye,
            forward: fwd,
            up0: up,
            fov_y_rad,
            near,
            far,
        };
        ST_OK
    }

    /// 曝光更新（ev100 域；exposure = 2^(−ev100) 经 128B TSR 参数逐帧生效）。
    #[unsafe(no_mangle)]
    pub extern "C" fn rxsdk_renderer_set_exposure_ev100(r: u64, ev100: f32) -> i32 {
        if !ev100.is_finite() {
            diag("set_exposure_ev100", "ev100 非有限");
            return ST_INPUT;
        }
        let mut guard = sessions_lock();
        let Some(s) = guard.get_mut(&r) else {
            diag("set_exposure_ev100", &format!("句柄 {r} 无效"));
            return ST_HANDLE;
        };
        if s.state != State::SceneLoaded {
            diag("set_exposure_ev100", "状态错序（须先 load_scene）");
            return ST_HANDLE;
        }
        s.ev100 = ev100;
        ST_OK
    }

    // ─────────────────────────── 帧循环 ───────────────────────────

    /// 渲染一帧（与 bench 腿逐字同式的确定性序列：halton jitter / 首帧 reset /
    /// TSR 历史链）。`readback != 0` → 本帧回读 TSR 输出算 digest 写入宿主缓冲
    /// （`out_digest` 容量须 ≥ 71；实写长度经 `out_digest_len` 回，无 NUL）。
    /// `out_frame_ms` = 本帧 host 墙钟毫秒（回读帧含回读税，诚实口径）。
    #[unsafe(no_mangle)]
    pub extern "C" fn rxsdk_renderer_render_frame(
        r: u64,
        readback: u32,
        out_frame_ms: *mut f64,
        out_digest: *mut u8,
        digest_cap: u32,
        out_digest_len: *mut u32,
    ) -> i32 {
        if out_frame_ms.is_null() || out_digest_len.is_null() {
            return ST_INPUT;
        }
        if readback != 0 && (out_digest.is_null() || digest_cap < DIGEST_BYTES) {
            diag("render_frame", "readback 帧 digest 缓冲空/容量 < 71");
            return ST_INPUT;
        }
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            render_frame_inner(
                r,
                readback != 0,
                out_frame_ms,
                out_digest,
                digest_cap,
                out_digest_len,
            )
        })) {
            Ok(rc) => rc,
            Err(_) => {
                diag("render_frame", "车道执行面 panic（catch_unwind 收口）");
                ST_RENDER
            }
        }
    }

    fn render_frame_inner(
        r: u64,
        readback: bool,
        out_frame_ms: *mut f64,
        out_digest: *mut u8,
        digest_cap: u32,
        out_digest_len: *mut u32,
    ) -> i32 {
        let mut guard = sessions_lock();
        let Some(s) = guard.get_mut(&r) else {
            diag("render_frame", &format!("句柄 {r} 无效"));
            return ST_HANDLE;
        };
        if s.state != State::SceneLoaded {
            diag("render_frame", "状态错序（须先 load_scene）");
            return ST_HANDLE;
        }
        let Some(lane) = s.lane.as_mut() else {
            diag("render_frame", "车道缺失（内部破缺）");
            return ST_HANDLE;
        };
        let i = s.frame_index;
        let jitter_base = (s.seed % crate::JITTER_WINDOW_MOD) as u32;
        let j = [
            crate::halton(jitter_base + i + 1, 2) - 0.5,
            crate::halton(jitter_base + i + 1, 3) - 0.5,
        ];
        let vp = crate::build_vp(&s.camera, s.in_w, s.in_h);
        let Some(inv_vp) = vp.inverse() else {
            diag("render_frame", "view-proj 不可逆（相机参数破缺）");
            return ST_INPUT;
        };
        let vp_j = crate::jittered_vp(&vp, j, s.in_w, s.in_h);
        let exposure = 2.0f32.powf(-s.ev100);
        let (quads, points) = match s.scene.as_ref() {
            Some(sc) => (sc.quads.len(), sc.points.len()),
            None => {
                diag("render_frame", "场景缺失（内部破缺）");
                return ST_HANDLE;
            }
        };
        let t0 = std::time::Instant::now();
        let rec = match lane.frame(
            s.in_w,
            s.in_h,
            s.out_w,
            s.out_h,
            j,
            s.eps,
            quads,
            points,
            &inv_vp,
            &vp,
            &vp_j,
            exposure,
            i == 0,
            readback,
        ) {
            Ok(r) => r,
            Err(e) => {
                diag("render_frame", &format!("帧 {i} 统一车道: {e}"));
                return ST_RENDER;
            }
        };
        let frame_ms = t0.elapsed().as_secs_f64() * 1000.0;
        if rec.validation_error_count != 0 {
            diag(
                "render_frame",
                &format!(
                    "帧 {i} validation ERROR 计数 {} ≠ 0",
                    rec.validation_error_count
                ),
            );
            return ST_VALIDATION;
        }
        let mut digest_written = 0u32;
        if readback {
            let Some(out_data) = rec.out_color.as_ref() else {
                diag("render_frame", "readback 帧缺 out_color（内部破缺）");
                return ST_RENDER;
            };
            if !out_data.iter().all(|v| v.is_finite()) {
                diag("render_frame", &format!("帧 {i} 输出非有限"));
                return ST_RENDER;
            }
            let digest = crate::frame_content_digest(s.out_w, s.out_h, 3, out_data);
            let bytes = digest.as_bytes();
            debug_assert_eq!(bytes.len(), DIGEST_BYTES as usize);
            let n = bytes.len().min(digest_cap as usize);
            // SAFETY: 调用方契约 = `out_digest` 指向 `digest_cap` 字节有效可写
            // 主机内存且调用期存活（生成头随声明交付）；n ≤ digest_cap 且
            // digest_cap ≥ 71 = bytes.len() 已由入口闭集核验，`n == bytes.len()`；
            // 源/目的不重叠（调用方缓冲与 DLL 内部串无别名）。
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_digest, n);
            }
            digest_written = n as u32;
        }
        s.frame_index = i + 1;
        // G35-8 加性：host 粒子臂随每成功帧 tick 一帧（会话无 emitter 时空转
        // 零池操作——既有行为 0-byte；渲染输出/digest 不受影响,v1 粒子臂不接
        // 渲染车道,device 接线归收口批）。SESSIONS→EMITTERS 锁序（本函数持
        // sessions guard 中嵌套取 emitters,全 crate 唯一嵌套序,禁反向）。
        particles_tick_session(r);
        // SAFETY: 两出参 null 已于入口核验；指向调用方有效可写单槽（生成头随
        // 声明交付的前置条件），写入不越出本次调用。
        unsafe {
            *out_frame_ms = frame_ms;
            *out_digest_len = digest_written;
        }
        ST_OK
    }

    /// present 句柄（v1 = 离屏会话呈现完成计数 + 成功；真窗口 swapchain
    /// present 归后续 MINOR 加性——语义登记不冒充，见 API_VERSIONING.md）。
    #[unsafe(no_mangle)]
    pub extern "C" fn rxsdk_renderer_present(r: u64) -> i32 {
        let mut guard = sessions_lock();
        let Some(s) = guard.get_mut(&r) else {
            diag("present", &format!("句柄 {r} 无效"));
            return ST_HANDLE;
        };
        if s.state != State::SceneLoaded {
            diag("present", "状态错序（须先 load_scene）");
            return ST_HANDLE;
        }
        s.presented = s.presented.saturating_add(1);
        ST_OK
    }

    // ─────────────────── G35-8 粒子作者面（RFC-0049 §4.11 冻结四签名） ───────────────────
    //
    // 加性纪律：本段全部为新增符号/新增表,既有导出签名/语义/状态码 0-byte
    // （render_frame 的 tick 挂点与 destroy 的 emitter 回收对无 emitter 会话
    // 均为空转）。v1 粒子臂 = host 臂（particles::core host 金标准,每 emitter
    // 独立池——host 金标准 frame() 单 desc 语义 1:1 映射,零语义发明）;device
    // 车道接线归 G35 收口批。用户面（sdk.rx）薄转发 + MINOR bump 同归收口批
    // （API_VERSIONING.md §6 登记）。

    /// v1 粒子臂逐 emitter 池容量（SEG=256 整倍数;RFC-0049 §4.2 分段布局）。
    const SDK_EMITTER_CAP: usize = 4096;
    /// v1 粒子臂随机带 seed（冻结常量——单源纪律:seed 归系统面不入资产字段
    /// 闭集,RFC-0049 §3;SDK v1 系统面 = 本常量,确定与会话装载序无关）。
    const SDK_PARTICLE_SEED: u64 = 35;
    /// v1 粒子臂固定步长（冻结确定性脚本;g35 probe 族同律）。
    const SDK_PARTICLE_DT: f32 = 1.0 / 60.0;
    /// 每会话 emitter 上限（句柄表卫生界;越界 = ST_INPUT fail-closed）。
    const SDK_EMITTERS_PER_SESSION_MAX: usize = 64;
    /// 资产 JSON 字节长上限（str_arg 同界）。
    const SDK_ASSET_JSON_MAX: usize = 4096;
    /// set_param key 闭集（RFC-0049 §4.11;闭集外 = ST_INPUT typed 拒）。
    const SDK_PARAM_KEYS: [&str; 5] = ["life_base", "gravity_y", "pos_x", "pos_y", "pos_z"];

    /// emitter 句柄表载荷（sys 归属 + 参数面运行时 + host 金标准粒子池）。
    struct EmitterEntry {
        /// 归属会话（会话 destroy 时随之回收——悬空 fail-closed）。
        sys: u64,
        /// 资产参数面 + 曲线帧钟（热参数语义:set_param 下一帧生效）。
        runtime: rurix_render::particles::emitter_asset::EmitterRuntime,
        /// SoA 粒子池 ping-pong 双组（读 A 写 B 帧末交换,G35-P v1 帧序）。
        pool_a: rurix_render::particles::core::ParticlePools,
        pool_b: rurix_render::particles::core::ParticlePools,
        /// persistent ID 单调水位（pid 硬域 [0, 2^24),RFC-0049 §4.4 F6）。
        pid_base: u32,
        /// 帧末存活数（stats 消费面;= 池 n）。
        alive: u64,
        /// 发射钳制 rejected 累计（accepted = min(requested, cap − n_curr) +
        /// pid 域余量;确定性钳制如实登记,RFC-0049 §4.4 F7）。
        rejected: u64,
    }

    /// emitter 句柄表（BTreeMap = 句柄升序确定迭代;锁序 = SESSIONS→EMITTERS
    /// 单向,禁反向嵌套）。
    static EMITTERS: std::sync::LazyLock<Mutex<BTreeMap<u64, EmitterEntry>>> =
        std::sync::LazyLock::new(|| Mutex::new(BTreeMap::new()));
    /// 随机带单源（host 一次生成全 emitter 只读共享;RFC-0045 §1.2 同律）。
    static PARTICLE_RAND: std::sync::LazyLock<Vec<f32>> =
        std::sync::LazyLock::new(|| rurix_render::particles::rand_table(SDK_PARTICLE_SEED));

    fn emitters_lock() -> std::sync::MutexGuard<'static, BTreeMap<u64, EmitterEntry>> {
        // poison 面同 sessions_lock:恢复确定性访问,不 panic 越过 C ABI。
        EMITTERS.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// host 粒子臂逐帧推进（render_frame 成功路径挂点;会话名下 emitter 按
    /// 句柄升序确定迭代,各自独立池跑 core::frame 一帧）。
    fn particles_tick_session(sys: u64) {
        let mut guard = emitters_lock();
        for e in guard.values_mut().filter(|e| e.sys == sys) {
            let requested = e.runtime.next_emit_count() as usize;
            let cap = e.pool_a.capacity();
            // 钳制语义（RFC-0049 §4.4 F7）:accepted = min(requested, cap −
            // n_curr);pid 硬域 [0, 2^24) 余量并钳（域尽 fail-closed 停发,
            // 禁静默回绕,F6）;rejected 确定性计数如实登记。
            let pid_room = ((1usize << 24) - 1).saturating_sub(e.pid_base as usize);
            let accepted = requested.min(cap - e.pool_a.n).min(pid_room);
            e.rejected += (requested - accepted) as u64;
            let desc = e.runtime.asset().to_desc();
            let st = rurix_render::particles::core::frame(
                &mut e.pool_a,
                &mut e.pool_b,
                &desc,
                &PARTICLE_RAND,
                SDK_PARTICLE_DT,
                e.pid_base,
                accepted,
            );
            e.pid_base += accepted as u32;
            e.alive = st.n_next as u64;
            std::mem::swap(&mut e.pool_a, &mut e.pool_b);
        }
    }

    /// 创建 emitter（RFC-0049 §4.11 冻结签名字面）。`desc_json` = UTF-8 JSON
    /// emitter 资产（v1 十字段闭集,fail-closed 解析违例 → ST_INPUT + 确定性
    /// 诊断行携 typed kind）;成功写 `*out` = 非零句柄。
    ///
    /// - `sys`:既有渲染会话句柄（无效 → ST_HANDLE;emitter 生命期 ⊆ 会话）。
    /// - `desc_json/len`:调用方 `(ptr, len)` 字节串（非空、len ∈ (0, 4096]、
    ///   指向 len 字节有效可读主机内存且调用期存活——documented unsafe FFI
    ///   边界,str_arg 同契约）。
    /// - `out`:仅成功时写入（失败不触,调用方以状态码裁决）。
    #[unsafe(no_mangle)]
    pub extern "C" fn rxsdk_particles_emitter_create(
        sys: u64,
        desc_json: *const u8,
        len: usize,
        out: *mut u64,
    ) -> i32 {
        if out.is_null() {
            diag("particles_emitter_create", "out 出参空");
            return ST_INPUT;
        }
        if desc_json.is_null() || len == 0 || len > SDK_ASSET_JSON_MAX {
            diag(
                "particles_emitter_create",
                &format!("desc_json 空/len {len} 越 (0, {SDK_ASSET_JSON_MAX}]"),
            );
            return ST_INPUT;
        }
        // SAFETY: 调用方契约 = `desc_json` 指向 `len` 字节有效可读主机内存且
        // 调用期存活（RXS-0251 §4.A6 documented unsafe FFI 边界,str_arg 同
        // 契约）;null/len 界已在上方拒;借用不越出本函数调用域。
        let bytes = unsafe { std::slice::from_raw_parts(desc_json, len) };
        let Ok(text) = std::str::from_utf8(bytes) else {
            diag("particles_emitter_create", "desc_json 非 UTF-8");
            return ST_INPUT;
        };
        // SESSIONS→EMITTERS 锁序（全 crate 唯一嵌套方向）。
        let sessions = sessions_lock();
        if !sessions.contains_key(&sys) {
            diag("particles_emitter_create", &format!("会话句柄 {sys} 无效"));
            return ST_HANDLE;
        }
        let mut emitters = emitters_lock();
        let per_session = emitters.values().filter(|e| e.sys == sys).count();
        if per_session >= SDK_EMITTERS_PER_SESSION_MAX {
            diag(
                "particles_emitter_create",
                &format!("会话 emitter 数越上限 {SDK_EMITTERS_PER_SESSION_MAX}"),
            );
            return ST_INPUT;
        }
        let asset = match rurix_render::particles::emitter_asset::EmitterAsset::parse(text) {
            Ok(a) => a,
            Err(e) => {
                diag(
                    "particles_emitter_create",
                    &format!("资产违例 kind={} {e}", e.kind_name()),
                );
                return ST_INPUT;
            }
        };
        let h = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
        if h == 0 {
            diag("particles_emitter_create", "句柄空间耗尽");
            return ST_INPUT;
        }
        emitters.insert(
            h,
            EmitterEntry {
                sys,
                runtime: rurix_render::particles::emitter_asset::EmitterRuntime::new(asset),
                pool_a: rurix_render::particles::core::ParticlePools::with_capacity(
                    SDK_EMITTER_CAP,
                ),
                pool_b: rurix_render::particles::core::ParticlePools::with_capacity(
                    SDK_EMITTER_CAP,
                ),
                pid_base: 0,
                alive: 0,
                rejected: 0,
            },
        );
        drop(emitters);
        drop(sessions);
        // SAFETY: 调用方契约 = `out` 指向有效可写 u64 单槽且调用期存活（生成
        // 头届时随声明交付同前置条件）;null 已于入口拒;写入不越出本次调用。
        unsafe {
            *out = h;
        }
        ST_OK
    }

    /// 更新 emitter 标量参数（RFC-0049 §4.11 冻结签名字面）。key 闭集 =
    /// [`SDK_PARAM_KEYS`]（life_base/gravity_y/pos_x/pos_y/pos_z;闭集外 =
    /// ST_INPUT typed 拒）;value 须有限（life_base 另须 > 0,资产域同律）;
    /// 热参数语义 = 纯参数面变化,粒子池/pid 连续,下一帧（下一次 render_frame
    /// tick）生效;悬空句柄 → ST_HANDLE fail-closed。
    #[unsafe(no_mangle)]
    pub extern "C" fn rxsdk_particles_emitter_set_param(
        h: u64,
        key: *const u8,
        klen: usize,
        value: f32,
    ) -> i32 {
        if key.is_null() || klen == 0 || klen > 64 {
            diag("particles_emitter_set_param", &format!("key 空/klen {klen} 越 (0, 64]"));
            return ST_INPUT;
        }
        // SAFETY: 调用方契约 = `key` 指向 `klen` 字节有效可读主机内存且调用期
        // 存活（str_arg 同契约）;null/界已拒;借用不越出本函数调用域。
        let kbytes = unsafe { std::slice::from_raw_parts(key, klen) };
        let Ok(kstr) = std::str::from_utf8(kbytes) else {
            diag("particles_emitter_set_param", "key 非 UTF-8");
            return ST_INPUT;
        };
        if !SDK_PARAM_KEYS.contains(&kstr) {
            diag(
                "particles_emitter_set_param",
                &format!("key {kstr:?} 越闭集 {SDK_PARAM_KEYS:?}"),
            );
            return ST_INPUT;
        }
        if !value.is_finite() {
            diag("particles_emitter_set_param", "value 非有限");
            return ST_INPUT;
        }
        if kstr == "life_base" && value <= 0.0 {
            diag("particles_emitter_set_param", "life_base 须 > 0（资产域同律）");
            return ST_INPUT;
        }
        let mut emitters = emitters_lock();
        let Some(e) = emitters.get_mut(&h) else {
            diag("particles_emitter_set_param", &format!("emitter 句柄 {h} 无效或已销毁"));
            return ST_HANDLE;
        };
        let a = e.runtime.asset_mut();
        match kstr {
            "life_base" => a.life_base = value,
            "gravity_y" => a.gravity_y = value,
            "pos_x" => a.pos[0] = value,
            "pos_y" => a.pos[1] = value,
            _ => a.pos[2] = value, // 闭集已核,余项唯 pos_z
        }
        ST_OK
    }

    /// 销毁 emitter（RFC-0049 §4.11 冻结签名字面）。句柄出表即回收其粒子臂
    /// （alive 归零不再计入 stats）;无效/双 destroy → ST_HANDLE 确定性拒。
    #[unsafe(no_mangle)]
    pub extern "C" fn rxsdk_particles_emitter_destroy(h: u64) -> i32 {
        match emitters_lock().remove(&h) {
            Some(_) => ST_OK,
            None => {
                diag("particles_emitter_destroy", &format!("emitter 句柄 {h} 无效或已销毁"));
                ST_HANDLE
            }
        }
    }

    /// 粒子系统统计（RFC-0049 §4.11 冻结签名字面）。`*out` = 会话名下全部
    /// emitter 帧末存活总数 alive_total(u64;host 金标准粒子系统随
    /// render_frame tick 推进——v1 SDK 粒子臂 = host 臂,device 车道接线归
    /// 收口批);会话句柄无效 → ST_HANDLE;out 空 → ST_INPUT。
    #[unsafe(no_mangle)]
    pub extern "C" fn rxsdk_particles_stats(sys: u64, out: *mut u64) -> i32 {
        if out.is_null() {
            diag("particles_stats", "out 出参空");
            return ST_INPUT;
        }
        // SESSIONS→EMITTERS 锁序。
        let sessions = sessions_lock();
        if !sessions.contains_key(&sys) {
            diag("particles_stats", &format!("会话句柄 {sys} 无效"));
            return ST_HANDLE;
        }
        let total: u64 = emitters_lock()
            .values()
            .filter(|e| e.sys == sys)
            .map(|e| e.alive)
            .sum();
        drop(sessions);
        // SAFETY: 调用方契约 = `out` 指向有效可写 u64 单槽且调用期存活;null
        // 已于入口拒;写入不越出本次调用。
        unsafe {
            *out = total;
        }
        ST_OK
    }

    // ─────────────────────────── 库单测（纯 host 面） ───────────────────────────

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn abi_version_packed_layout() {
            assert_eq!(ABI_VERSION_PACKED, 0x0001_0000);
            assert_eq!(rxsdk_abi_version() >> 16, 1);
            assert_eq!((rxsdk_abi_version() >> 8) & 0xFF, 0);
            assert_eq!(rxsdk_abi_version() & 0xFF, 0);
        }

        #[test]
        fn handle_zero_invalid() {
            assert_eq!(rxsdk_renderer_destroy(0), ST_HANDLE);
            assert_eq!(rxsdk_renderer_present(0), ST_HANDLE);
            assert_eq!(
                rxsdk_renderer_render_frame(
                    0,
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut()
                ),
                ST_INPUT // null 出参先于句柄核验（入口闭集序）
            );
        }

        #[test]
        fn str_arg_rejects() {
            assert_eq!(str_arg(std::ptr::null(), 3, "x").unwrap_err(), ST_INPUT);
            assert_eq!(str_arg(b"a".as_ptr(), 0, "x").unwrap_err(), ST_INPUT);
            assert_eq!(str_arg(b"ab".as_ptr(), 2, "x").unwrap(), "ab");
            let bad = [0xFFu8, 0xFE];
            assert_eq!(str_arg(bad.as_ptr(), 2, "x").unwrap_err(), ST_INPUT);
        }

        #[test]
        fn f32x3_arg_rejects() {
            assert_eq!(f32x3_arg(std::ptr::null(), "x").unwrap_err(), ST_INPUT);
            let nan3 = [0.0f32, f32::NAN, 1.0];
            assert_eq!(f32x3_arg(nan3.as_ptr(), "x").unwrap_err(), ST_INPUT);
            let ok3 = [1.0f32, 2.0, 3.0];
            assert_eq!(f32x3_arg(ok3.as_ptr(), "x").unwrap(), &ok3);
        }

        // ── G35-8 粒子作者面（rxsdk_particles_* 四冻结签名） ──

        /// 合法资产夹具（emitter_asset 十字段闭集;const 24.7 → 24/帧精确账目）。
        const PARTICLES_GOOD_JSON: &str = concat!(
            r#"{"name":"sdk_fixture","pos":[0.0,1.0,-0.5],"spread":[0.4,0.2,0.4],"#,
            r#""vel_base":[0.0,3.0,0.0],"vel_spread":[1.0,0.5,1.0],"life_base":1.2,"#,
            r#""gravity_y":-9.8,"emit_curve":{"kind":"const","value":24.7},"#,
            r#""render":"billboard","blend":"alpha"}"#
        );

        fn create_emitter(sys: u64, json: &str) -> (i32, u64) {
            let mut h = 0u64;
            let rc = rxsdk_particles_emitter_create(sys, json.as_ptr(), json.len(), &mut h);
            (rc, h)
        }

        /// 句柄/指针 fail-closed 闭集（零 device 依赖恒可跑）:悬空句柄 =
        /// ST_HANDLE;空指针/越界输入 = ST_INPUT——先于任何池操作确定性拒。
        #[test]
        fn particles_handle_and_input_fail_closed() {
            let good = PARTICLES_GOOD_JSON;
            // out 空指针。
            assert_eq!(
                rxsdk_particles_emitter_create(u64::MAX, good.as_ptr(), good.len(), std::ptr::null_mut()),
                ST_INPUT
            );
            // desc_json 空/零长/越长界。
            let mut h = 0u64;
            assert_eq!(
                rxsdk_particles_emitter_create(u64::MAX, std::ptr::null(), 3, &mut h),
                ST_INPUT
            );
            assert_eq!(
                rxsdk_particles_emitter_create(u64::MAX, good.as_ptr(), 0, &mut h),
                ST_INPUT
            );
            // 会话句柄悬空（u64::MAX 恒未发行;NEXT_HANDLE 自 1 递增）。
            assert_eq!(create_emitter(u64::MAX, good).0, ST_HANDLE);
            // emitter 句柄悬空:set_param/destroy fail-closed。
            let key = "gravity_y";
            assert_eq!(
                rxsdk_particles_emitter_set_param(u64::MAX, key.as_ptr(), key.len(), -1.0),
                ST_HANDLE
            );
            assert_eq!(rxsdk_particles_emitter_destroy(u64::MAX), ST_HANDLE);
            // set_param 输入面:key 空指针/零长。
            assert_eq!(
                rxsdk_particles_emitter_set_param(u64::MAX, std::ptr::null(), 3, 1.0),
                ST_INPUT
            );
            assert_eq!(
                rxsdk_particles_emitter_set_param(u64::MAX, key.as_ptr(), 0, 1.0),
                ST_INPUT
            );
            // stats:out 空 = ST_INPUT 先于句柄核;悬空会话 = ST_HANDLE。
            assert_eq!(rxsdk_particles_stats(u64::MAX, std::ptr::null_mut()), ST_INPUT);
            let mut alive = 0u64;
            assert_eq!(rxsdk_particles_stats(u64::MAX, &mut alive), ST_HANDLE);
        }

        /// 资产 fail-closed + 生命周期 + host 臂 tick 精确账目 + 跨会话确定性
        /// （会话面需 Vulkan loader——仅 loader 探测零 GPU 工作;缺席环境如实
        /// 跳过,三态纪律测试粒度镜像）。
        #[test]
        fn particles_lifecycle_tick_and_stats() {
            if !crate::vk::vulkan_available() {
                eprintln!("skip: vulkan loader 缺席（host-only 环境,如实跳过不冒充）");
                return;
            }
            let sys = rxsdk_renderer_create(0);
            assert_ne!(sys, 0, "会话创建(loader 面)必须成功");
            // 资产违例 fail-closed(ST_INPUT):缺字段/闭集外字段/语法错。
            let missing = PARTICLES_GOOD_JSON.replace(r#""life_base":1.2,"#, "");
            assert_eq!(create_emitter(sys, &missing).0, ST_INPUT);
            let unknown = PARTICLES_GOOD_JSON.replace(
                r#""gravity_y":-9.8"#,
                r#""gravity_y":-9.8,"drag":0.1"#,
            );
            assert_eq!(create_emitter(sys, &unknown).0, ST_INPUT);
            assert_eq!(create_emitter(sys, "{not json").0, ST_INPUT);
            // 合法资产 → 非零句柄;初始 stats = 0(未 tick)。
            let (rc, h1) = create_emitter(sys, PARTICLES_GOOD_JSON);
            assert_eq!(rc, ST_OK);
            assert_ne!(h1, 0);
            let mut alive = u64::MAX;
            assert_eq!(rxsdk_particles_stats(sys, &mut alive), ST_OK);
            assert_eq!(alive, 0, "未 tick 前 alive 必须 0");
            // host 臂 tick 精确账目:const 24.7 → 24/帧;life ∈ [0.6,1.2)s vs
            // 3/60 s 窗零死亡 ⇒ alive = 24×3(白盒直调 tick——render_frame 的
            // device 全链见证归收口批,如实分层)。
            for _ in 0..3 {
                particles_tick_session(sys);
            }
            assert_eq!(rxsdk_particles_stats(sys, &mut alive), ST_OK);
            assert_eq!(alive, 72, "3 tick × 24/帧 = 72 精确账目");
            // set_param:闭集内 OK(热参数,池不重置);闭集外/非有限/域违约拒。
            let k_g = "gravity_y";
            assert_eq!(
                rxsdk_particles_emitter_set_param(h1, k_g.as_ptr(), k_g.len(), -3.0),
                ST_OK
            );
            let k_bad = "spread_x";
            assert_eq!(
                rxsdk_particles_emitter_set_param(h1, k_bad.as_ptr(), k_bad.len(), 1.0),
                ST_INPUT
            );
            assert_eq!(
                rxsdk_particles_emitter_set_param(h1, k_g.as_ptr(), k_g.len(), f32::NAN),
                ST_INPUT
            );
            let k_life = "life_base";
            assert_eq!(
                rxsdk_particles_emitter_set_param(h1, k_life.as_ptr(), k_life.len(), 0.0),
                ST_INPUT
            );
            particles_tick_session(sys);
            assert_eq!(rxsdk_particles_stats(sys, &mut alive), ST_OK);
            assert_eq!(alive, 96, "set_param 后池连续不重置(4 tick × 24 = 96)");
            // 跨会话确定性:同资产同 seed 同 tick 数 → alive 全等(随机带单源
            // SDK_PARTICLE_SEED 冻结,与会话装载序无关)。
            let sys2 = rxsdk_renderer_create(0);
            assert_ne!(sys2, 0);
            let (rc2, h2) = create_emitter(sys2, PARTICLES_GOOD_JSON);
            assert_eq!(rc2, ST_OK);
            for _ in 0..4 {
                particles_tick_session(sys2);
            }
            let mut alive2 = 0u64;
            assert_eq!(rxsdk_particles_stats(sys2, &mut alive2), ST_OK);
            assert_eq!(alive2, alive, "跨会话同资产同 tick 数必须同 alive(确定性)");
            // destroy:出表回收;双 destroy = ST_HANDLE。
            assert_eq!(rxsdk_particles_emitter_destroy(h1), ST_OK);
            assert_eq!(rxsdk_particles_emitter_destroy(h1), ST_HANDLE);
            assert_eq!(rxsdk_particles_stats(sys, &mut alive), ST_OK);
            assert_eq!(alive, 0, "emitter 出表后 alive 归零");
            // 会话回收连带:sys2 destroy 后 h2 悬空 fail-closed。
            assert_eq!(rxsdk_renderer_destroy(sys2), ST_OK);
            assert_eq!(
                rxsdk_particles_emitter_set_param(h2, k_g.as_ptr(), k_g.len(), 1.0),
                ST_HANDLE,
                "会话回收后 emitter 句柄必须悬空"
            );
            assert_eq!(rxsdk_particles_stats(sys2, &mut alive2), ST_HANDLE);
            assert_eq!(rxsdk_renderer_destroy(sys), ST_OK);
        }
    }
}
