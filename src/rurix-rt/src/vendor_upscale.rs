//! G13.2 M-a(M167) vendor 超分 FFI 边界（U58；G13_CONTRACT §4.2 M-a 行 / G-G13-4；
//! RFC-0016 §4.H3 UpscaleBackend 冻结面 vendor 实现位的 device 执行底座）。
//!
//! 双臂（同一 safe 公共面，供 rurix-render bin 侧 UpscaleBackend 冻结 trait 实现消费）：
//! - **DLSS SR**：Streamline SDK 2.10.3（`sl.interposer.dll` 运行时装载 + NGX 签名
//!   `nvngx_dlss.dll`），**Vulkan interop 臂**（契约字面：主腿 Vulkan，不引 DXIL
//!   依赖）——`slInit`(eUseManualHooking) → SL 代理 `vkCreateInstance`/`vkCreateDevice`
//!   （SL 自担其必需扩展/特性/队列）→ 逐帧 `slGetNewFrameToken` + `slSetConstants` +
//!   `slDLSSSetOptions` + `slEvaluateFeature(kFeatureDLSS)` 真跑出帧。
//! - **FSR 3.1.5**：FidelityFX SDK 2.0.0 预编译签名 DLL（`amd_fidelityfx_loader_dx12.dll`
//!   + `amd_fidelityfx_upscaler_dx12.dll`）**D3D12 臂**——FSR SDK 2.x 已移除 Vulkan
//!   后端（v2.0.0/v2.3.0 实测 `Kits/FidelityFX/backend` 仅存 dx12 面、signedbin 仅存
//!   `*_dx12.dll`），FSR 3.1.5 唯一交付通道 = DX12；FSR4 ML 需 RDNA4，本机 RTX 4070 Ti
//!   不可用 → 自动回退 FSR 3.1.5 分析版如实登记（`FsrDx12Session::report`）。
//!
//! **动态装载**（镜像 sys.rs nvcuda.dll / tirt.rs taichi_c_api.dll 纪律）：SDK 目录经
//! 环境变量 [`STREAMLINE_SDK_DIR_ENV`] / [`FSR_SDK_DIR_ENV`] 或默认 `external/` 相对
//! 路径给出，`LoadLibraryExW(LOAD_WITH_ALTERED_SEARCH_PATH)` 运行时装载；未设 / 文件
//! 不存在 / 符号缺失 → 确定性 [`VendorError`] **fail-closed**（P-01，不搜默认路径）。
//! 零外部依赖（手写 repr(C) FFI 薄层，对齐 crate 无依赖纪律；vendor SDK 头文件仅作
//! 声明核对参照，零复制进 src/——G13 立项裁决 10 / RFC-0027 字面）。
//!
//! **同步纪律**：单 queue 同步提交 + `vkQueueWaitIdle`（VK 臂）/ fence 有界等待
//! （D3D12 臂）后回读；host-visible+coherent / upload+readback heap 免 flush 口径。
//! **释放顺序**：VK 臂 = `vkDeviceWaitIdle` → `slFreeResources` → `slShutdown`（SL
//! 文档明示 slShutdown 必须先于 vk 对象销毁）→ vk 对象逆序销毁；D3D12 臂 = fence
//! 排空 → `ffxDestroyContext` → COM 对象逆序 Release。
//!
//! **G14plus vendor 域加性面**（RFC-0030；既有 `upscale`/`upscale_into`/
//! `probe_validation_frame` 行为 0-byte，FSR 臂零触碰）：DLSS 臂驻留输出——
//! [`DlssVkSession::upscale_resident`]（evaluate 后输出驻留 `color_out`，跳过
//! 逐帧回读与 host f16→f32 转换）+ [`DlssVkSession::readback_output_into`]
//! （按需回读，转换面与 `upscale` 同一事实源）+
//! [`DlssVkSession::output_image_raw`]（句柄簿记导出）。device 拓扑事实：DLSS
//! session 经 SL 代理**自建** instance/device，与 render_exec session 的 device
//! 相互独立——输入驻留（外部 VkImage 直 tag）需 VK_KHR_external_memory 跨
//! device 导出/导入或同 device 化改造，超出本波加性边界，pack/upload 面维持
//! （拓扑发现与裁决面见 RFC-0030 G14plus 实施记录）。
//!
//! # SAFETY（U58，G13.2 vendor SDK FFI 边界；沿 U26/U32/U43 审计模式）
//! 对上全 safe（无 `unsafe` 签名）。全部 `unsafe` 内聚本模块，每块携 `// SAFETY:`：
//! - SL 函数指针签名与 `sl_core_api.h`/`sl_dlss.h`/`sl_consts.h`（Streamline 2.10.3，
//!   `kSDKVersion = 2<<48|10<<32|3<<16|0xfedc`）逐一对应；SL_STRUCT 结构
//!   （Preferences/Constants/Resource/ResourceTag/ViewportHandle/DLSSOptions/
//!   DLSSOptimalSettings/AdapterInfo/FeatureVersion）`#[repr(C)]` + GUID+version 头
//!   （BaseStructure = next ptr + StructType 16B + size_t version，32B）编译期
//!   `const assert!` 布局锚定（144/456/112/64/40/88/64/56/56）；
//! - FFX 函数指针签名与 `ffx_api.h`/`ffx_upscale.h`（SDK 2.0.0）逐一对应；
//!   ffxApiHeader/ffxCreateContextDescUpscale/ffxCreateBackendDX12Desc/
//!   ffxDispatchDescUpscale/FfxApiResource 编译期布局锚定（40/24/432/48 等）；
//! - D3D12/DXGI COM vtable 槽位与 WinSDK 10.0.26100.0 `d3d12.h`/`d3d12sdklayers.h`/
//!   `dxgi.h` 逐槽核对（ID3D12Device CreateCommandQueue@8/CreateCommandAllocator@9/
//!   CreateCommandList@12/CreateCommittedResource@27/CreateFence@36、
//!   ID3D12GraphicsCommandList Close@9/Reset@10/CopyTextureRegion@16/ResourceBarrier@26、
//!   ID3D12CommandQueue ExecuteCommandLists@10/Signal@14、ID3D12Fence
//!   GetCompletedValue@8/SetEventOnCompletion@9、ID3D12Resource Map@8/Unmap@9、
//!   ID3D12Debug EnableDebugLayer@3、ID3D12InfoQueue GetMessage@5/
//!   GetNumStoredMessages@8、IDXGIFactory1 EnumAdapters1@12、IDXGIAdapter1 GetDesc1@10）；
//! - VkStruct（ApplicationInfo/InstanceCreateInfo/DeviceQueueCreateInfo/DeviceCreateInfo/
//!   ImageCreateInfo/ImageViewCreateInfo/BufferCreateInfo/MemoryAllocateInfo/
//!   CommandPoolCreateInfo/CommandBufferAllocateInfo/CommandBufferBeginInfo/SubmitInfo/
//!   ImageMemoryBarrier/BufferImageCopy/DebugUtilsMessengerCreateInfoEXT）与 Vulkan SDK
//!   1.3.296 `vulkan_core.h` 逐字节对齐（sType 值/字段序/尺寸），由本模块布局锚单测 +
//!   `VK_LAYER_KHRONOS_validation`（`RURIX_VK_VALIDATION=1`）探针帧零实错双证
//!   （探针跳过 slEvaluateFeature：NGX evaluate 层在下触发驱动内崩溃，vendor
//!   已知不兼容——排除面见 `probe_validation_frame` 文档与契约 §8.3）；
//! - 句柄（vk instance/device/image/view/buffer/memory/cmdpool + SL FrameToken + D3D12
//!   COM 对象 + ffx context）线性配对、逆序销毁，早退路径同走销毁序，无泄漏/双释放；
//!   loader 不 `FreeLibrary`（进程常驻，镜像 U1 纪律）；validation messenger 的
//!   `p_user_data` 指向 session 内 `Box<ValidationCounts>` 稳定地址，messenger 先于
//!   instance 销毁（生命周期严格短于该堆变量）。

#![allow(unsafe_op_in_unsafe_fn)]

use core::ffi::{c_char, c_void};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Streamline SDK 目录环境变量（绝对/相对路径;未设 → 默认 `external/streamline-2.10.3`）。
pub const STREAMLINE_SDK_DIR_ENV: &str = "RURIX_STREAMLINE_SDK_DIR";
/// FidelityFX SDK 目录环境变量（未设 → 默认 `external/fidelityfx-sdk-2.0.0`）。
pub const FSR_SDK_DIR_ENV: &str = "RURIX_FSR_SDK_DIR";

// ── OS 动态装载缝（镜像 tirt.rs;LoadLibraryExW ALTERED_SEARCH_PATH 使 DLL 自
//    身目录进入其依赖搜索序——sl.interposer 对 sl.common 等侧载依赖必需） ──────────
#[cfg(windows)]
mod loader {
    use core::ffi::{c_char, c_void};
    use std::path::Path;
    unsafe extern "system" {
        fn LoadLibraryExW(name: *const u16, file: *mut c_void, flags: u32) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const c_char) -> *mut c_void;
    }
    const LOAD_WITH_ALTERED_SEARCH_PATH: u32 = 0x8;
    /// 装载指定路径的 DLL(失败 → null)。
    pub(super) fn open(path: &Path) -> *mut c_void {
        let wide: Vec<u16> = path
            .as_os_str()
            .to_string_lossy()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: wide 为 NUL 结尾 UTF-16;LoadLibraryExW 为 Win32 稳定 ABI(kernel32);
        // ALTERED_SEARCH_PATH 使目标 DLL 目录进入其侧载依赖搜索序。
        unsafe { LoadLibraryExW(wide.as_ptr(), std::ptr::null_mut(), LOAD_WITH_ALTERED_SEARCH_PATH) }
    }
    /// # Safety
    /// `lib` 为 `open` 返回的有效模块句柄;`name` NUL 结尾。
    pub(super) unsafe fn sym(lib: *mut c_void, name: *const c_char) -> *mut c_void {
        // SAFETY: 调用方保证 `lib` 有效、`name` NUL 结尾;GetProcAddress 为 Win32 稳定 ABI。
        unsafe { GetProcAddress(lib, name) }
    }
}

#[cfg(not(windows))]
mod loader {
    use core::ffi::{c_char, c_void};
    use std::ffi::CString;
    use std::path::Path;
    unsafe extern "C" {
        fn dlopen(filename: *const c_char, flag: i32) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    }
    const RTLD_NOW: i32 = 2;
    pub(super) fn open(path: &Path) -> *mut c_void {
        let Ok(c) = CString::new(path.as_os_str().to_string_lossy().as_bytes()) else {
            return std::ptr::null_mut();
        };
        // SAFETY: c 为 NUL 结尾;dlopen 为 POSIX 稳定 ABI(libc)。
        unsafe { dlopen(c.as_ptr(), RTLD_NOW) }
    }
    /// # Safety
    /// `lib` 为 `open` 返回的有效模块句柄;`name` NUL 结尾。
    pub(super) unsafe fn sym(lib: *mut c_void, name: *const c_char) -> *mut c_void {
        // SAFETY: 调用方保证 `lib` 有效、`name` NUL 结尾;dlsym 为 POSIX 稳定 ABI。
        unsafe { dlsym(lib, name) }
    }
}

/// 符号地址 → 类型化函数指针;null → None(镜像 sys::cast_fn/tirt cast_sym)。
///
/// # Safety
/// `raw` 须为目标库中同名导出符号、ABI 与 `T` 一致的函数地址(或 null)。
unsafe fn cast_sym<T: Copy>(raw: *mut c_void) -> Option<T> {
    if raw.is_null() {
        return None;
    }
    debug_assert_eq!(size_of::<T>(), size_of::<*mut c_void>());
    // SAFETY: raw 非 null(已查);T 为指针宽度函数指针(宽度断言校核);调用方保证
    // 符号名 ⇔ `T` 签名 ⇔ SDK 头文件声明逐一对应。
    Some(unsafe { std::mem::transmute_copy::<*mut c_void, T>(&raw) })
}

/// vendor 边界错误枚举(Display;无新 RX 码)。
#[derive(Debug)]
pub enum VendorError {
    /// SDK 目录/DLL 缺失或动态装载失败(fail-closed)。
    DllNotFound(String),
    /// DLL 在位但导出符号缺失(版本不符)。
    SymbolMissing(String),
    /// GPU/设备/队列/驱动面不可用(含 DLSS 不支持/FSR 上下文创建失败)。
    DeviceUnavailable(String),
    /// vendor SDK 调用返回非 Ok(附 SDK 侧诊断)。
    VendorCall(String),
    /// Vulkan/D3D12 调用失败。
    ApiError(String),
    /// validation/debug 层报错计数非零。
    Validation(String),
}

impl fmt::Display for VendorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VendorError::DllNotFound(m) => write!(f, "vendor SDK DLL 不可用: {m}"),
            VendorError::SymbolMissing(m) => write!(f, "vendor SDK 符号缺失: {m}"),
            VendorError::DeviceUnavailable(m) => write!(f, "vendor 设备面不可用: {m}"),
            VendorError::VendorCall(m) => write!(f, "vendor SDK 调用失败: {m}"),
            VendorError::ApiError(m) => write!(f, "图形 API 调用失败: {m}"),
            VendorError::Validation(m) => write!(f, "validation 报错: {m}"),
        }
    }
}

impl std::error::Error for VendorError {}

/// DLL provenance 单项（名称 + SHA-256 + 字节数;registry 对拍面）。
#[derive(Debug, Clone)]
pub struct DllProvenance {
    pub name: String,
    pub sha256: String,
    pub bytes: u64,
}

/// 单帧 vendor 超分输入（UpscaleInputs 冻结语义的 device 投影;color/depth/mv/
/// reactive 为输入(内部)分辨率 f32 行主序;color 3 通道、mv 2 通道、depth/reactive 1 通道;
/// mv = uv 位移(prev_uv − cur_uv),jitter 为输入分辨率像素单位——RFC-0016 §4.0-3 口径）。
#[derive(Debug, Clone, Copy)]
pub struct VendorFrameInput<'a> {
    pub color: &'a [f32],
    pub depth: &'a [f32],
    pub mv: &'a [f32],
    pub reactive: Option<&'a [f32]>,
    pub exposure: f32,
    pub jitter: [f32; 2],
    pub frame_index: u32,
    pub reset: bool,
}

/// G14.10b(RFC-0030 §4.3)外部导入 image 描述(render_exec exportable 纹理的
/// `ExportedTextureWin32` 簿记逐字段对应;bin 侧拆字段传递,两模块类型不互引)。
/// `handle` 归导出 session 所有——本模块**不关闭**;导入完成后本 session 持有
/// 自己的 VkImage/VkDeviceMemory 引用(Drop 释放),导出侧内容跨界有效性契约:
/// 导出帧 fence 完成(collect/execute 返回)后才可 evaluate。
#[derive(Debug, Clone, Copy)]
pub struct ExternalImageImportDesc {
    /// NT handle 地址值(OPAQUE_WIN32;导出侧 `ExportedTextureWin32::handle`)。
    pub handle: usize,
    pub width: u32,
    pub height: u32,
    /// VkFormat 数值(与导出 image 一致;SL tag `native_format` 同源)。
    pub vk_format: u32,
    /// 导出 image 的 usage 位(导入 image 参数须一致)。
    pub usage_flags: u32,
    /// 导出 allocation 字节数。
    pub allocation_size: u64,
    /// 导出 allocation memory type index(同 LUID 物理设备类型序一致)。
    pub memory_type_index: u32,
}

/// G14.10f buffer 共享导入簿记(render_exec `ExportedBufferWin32` 对侧;
/// 跨 device OPTIMAL image 布局解释不一致的正解——buffer 线性无歧义,DLSS 侧
/// 每帧 `vkCmdCopyBufferToImage` 进 session 自建输入 image)。handle 所有权
/// 契约同 [`ExternalImageImportDesc`](导出 session 持有,导入方不得关闭)。
#[derive(Debug, Clone, Copy)]
pub struct ExternalBufferImportDesc {
    /// NT handle 地址值(OPAQUE_WIN32)。
    pub handle: usize,
    /// buffer 声明字节数(导入侧 VkBuffer size 同值;内容布局 = 紧凑纹素:
    /// color RGBA16F 8B/px、depth f32 4B/px、mv RG32F 8B/px)。
    pub size: u64,
    /// 导出 allocation 字节数。
    pub allocation_size: u64,
    /// 导出 allocation memory type index(同 LUID 物理设备类型序一致)。
    pub memory_type_index: u32,
}

/// G14.11(RFC-0030 §4.3)FSR D3D12 反向共享输入簿记(**buffer 形态**——
/// texture 直共享弃案:D3D12_RESOURCE handle 导入 OPTIMAL VkImage 在 NVIDIA
/// 驱动上跨 API tiling 解释不一致,D3D12 侧读为确定性条纹乱序,读图抓获,
/// 证据 evidence/g14_11_fsr_dump_{pack,import}.png;与 dlss 臂 OPAQUE_WIN32
/// 跨 device 弃案同族)。现方案:D3D12 侧 `D3D12_HEAP_FLAG_SHARED` committed
/// **BUFFER**(线性字节,布局无歧义)经 `CreateSharedHandle` 导出 NT handle,
/// Vulkan 侧(render_exec)以 D3D12_RESOURCE handle 导入 bind 为 SSBO,pack
/// kernel 按 host 链 upload 布局直写(color f16 RGBA / depth f32 / mv f32
/// RG,行距 256 对齐);D3D12 侧逐帧 `CopyTextureRegion` 搬入三输入纹理
/// (GPU 内拷,formats 与 host 链逐字同)。handle 归产出它的
/// [`FsrDx12Session`] 所有(Drop 单点 CloseHandle)——导入方**不得关闭**。
#[derive(Debug, Clone, Copy)]
pub struct FsrSharedInputHandles {
    /// staging buffer NT handle 地址值(D3D12 SHARED committed BUFFER)。
    pub staging: usize,
    /// buffer 字节数(64KB 对齐;Vulkan 侧资源声明须同值)。
    pub staging_size: u64,
    /// color 段行距(字节;= align256(8·iw),f16 RGBA 8B/px,段偏移 0)。
    pub color_row: u64,
    /// depth 段行距(字节;= align256(4·iw),f32 4B/px)。
    pub depth_row: u64,
    /// mv 段行距(字节;= align256(8·iw),f32 RG 8B/px)。
    pub mv_row: u64,
    /// depth 段字节偏移(= color_row·ih)。
    pub off_depth: u64,
    /// mv 段字节偏移(= off_depth + depth_row·ih)。
    pub off_mv: u64,
    /// D3D12 adapter LUID(DXGI_ADAPTER_DESC1.AdapterLuid 8B;与 Vulkan
    /// `VkPhysicalDeviceIDProperties::deviceLUID` 对拍,不等 fail-closed)。
    pub adapter_luid: [u8; 8],
}

/// G14.10b 外部输入槽(DLSS 三输入;reactive 仍走 session 自建 image 的 host
/// 上传路径——可选输入,驻留收益边际)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalInputSlot {
    /// scaling input color(导出侧建议 RGBA16F;RGBA32F 容忍度见冒烟探明)。
    Color,
    /// depth(D32 或 R32F——R32F 可被 render_exec compute 直写)。
    Depth,
    /// motion vectors(RG32F,与 DLSS 现输入格式同一)。
    Mv,
}

/// G14.10b 外部输入 evaluate 帧参数(输入内容驻留外部导入 image,host 仅传
/// 相机/jitter 簿记与可选 reactive;字段语义与 [`VendorFrameInput`] 同名同义)。
#[derive(Debug, Clone, Copy)]
pub struct VendorExternalFrameParams<'a> {
    /// 可选 reactive mask(长度 = in_w×in_h;None = 全零上传)。
    pub reactive: Option<&'a [f32]>,
    pub exposure: f32,
    pub jitter: [f32; 2],
    pub frame_index: u32,
    pub reset: bool,
}

/// vendor session 报告（evidence provenance 面）。
#[derive(Debug, Clone)]
pub struct VendorSessionReport {
    pub backend: String,
    pub gpu_name: String,
    pub validation_errors: u64,
    pub dlls: Vec<DllProvenance>,
    /// DLSS: NGX 版本字面;FSR: provider version 字面。
    pub engine_version: String,
    /// FSR 臂:FSR4 ML 可用性如实登记(DLSS 臂恒 None)。
    pub fsr4_ml_available: Option<bool>,
    pub fsr4_note: Option<String>,
    /// FSR 臂:ffxQueryDescGetVersions 实测可用版本清单。
    pub available_versions: Vec<String>,
    pub log_tail: Vec<String>,
}

/// G14plus vendor 域(RFC-0030)DLSS 输出 image 原生句柄簿记(vendor 侧
/// **独立** VkDevice 域——DLSS session 经 SL 代理自建 instance/device,句柄
/// 跨 VkDevice 不可直接消费;见 [`DlssVkSession::output_image_raw`])。
#[derive(Debug, Clone, Copy)]
pub struct DlssOutputImageRaw {
    /// VkImage(non-dispatchable u64)。
    pub image: u64,
    /// VkDeviceMemory。
    pub memory: u64,
    /// VkImageView。
    pub view: u64,
    /// VkFormat 数值(R16G16B16A16_SFLOAT = 97)。
    pub vk_format: u32,
    /// 当前 VkImageLayout(0=UNDEFINED 无内容;1=GENERAL 已有评估内容)。
    pub layout: i32,
    pub width: u32,
    pub height: u32,
}

fn sha256_file(path: &Path) -> Result<(String, u64), VendorError> {
    let bytes = std::fs::read(path)
        .map_err(|e| VendorError::DllNotFound(format!("{}: {e}", path.display())))?;
    let n = bytes.len() as u64;
    Ok((rurix_pkg::sha256::hex_digest(&bytes), n))
}

fn default_sdk_dir(env: &str, rel: &str) -> Result<PathBuf, VendorError> {
    if let Ok(v) = std::env::var(env)
        && !v.is_empty()
    {
        let p = PathBuf::from(v);
        if p.is_dir() {
            return Ok(p);
        }
        return Err(VendorError::DllNotFound(format!(
            "环境变量 {env} 指向的目录不存在: {}",
            p.display()
        )));
    }
    // 默认:工作区根 external/(CARGO_MANIFEST_DIR = src/rurix-rt)。
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| VendorError::DllNotFound("工作区根解析失败".into()))?;
    let p = root.join(rel);
    if p.is_dir() {
        Ok(p)
    } else {
        Err(VendorError::DllNotFound(format!(
            "默认 SDK 目录不存在: {}(可用环境变量 {env} 覆盖)",
            p.display()
        )))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 第 1 部:Streamline / DLSS Vulkan interop 臂
// ═══════════════════════════════════════════════════════════════════════════

// ── sl 基础类型(sl_struct.h/sl_core_types.h,Streamline 2.10.3 头核对) ──────────
type SlResult = i32; // enum class Result(int);eOk=0
const SL_OK: i32 = 0;

#[repr(C)]
#[derive(Clone, Copy)]
struct SlStructType {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}
const _: () = assert!(size_of::<SlStructType>() == 16);

const fn sl_guid(d1: u32, d2: u16, d3: u16, d4: [u8; 8]) -> SlStructType {
    SlStructType { data1: d1, data2: d2, data3: d3, data4: d4 }
}

#[repr(C)]
struct SlBaseStructure {
    next: *mut SlBaseStructure,
    struct_type: SlStructType,
    struct_version: usize,
}
const _: () = assert!(size_of::<SlBaseStructure>() == 32);

fn sl_base(guid: SlStructType, version: usize) -> SlBaseStructure {
    SlBaseStructure { next: std::ptr::null_mut(), struct_type: guid, struct_version: version }
}

/// sl::Preferences(sl_core_types.h L530,kStructVersion1;C++ bool=u8 精确展开)。
// Preferences 精确布局(手工展开,bool=u8):
// base(32) + showConsole u8@32 + pad3 + logLevel u32@36 + pathsToPlugins ptr@40
// + numPathsToPlugins u32@48 + pad4 + pathToLogsAndData ptr@56 + allocateCallback@64
// + releaseCallback@72 + logMessageCallback@80 + flags u64@88 + featuresToLoad ptr@96
// + numFeaturesToLoad u32@104 + applicationId u32@108 + engine u32@112 + pad4
// + engineVersion ptr@120 + projectId ptr@128 + renderAPI u32@136 + pad4 → 144。
#[repr(C)]
struct SlPreferencesLayout {
    base: SlBaseStructure,
    show_console: u8,
    _pad0: [u8; 3],
    log_level: u32,
    paths_to_plugins: *const *const u16,
    num_paths_to_plugins: u32,
    _pad1: u32,
    path_to_logs_and_data: *const u16,
    allocate_callback: *mut c_void,
    release_callback: *mut c_void,
    log_message_callback: *mut c_void,
    flags: u64,
    features_to_load: *const u32,
    num_features_to_load: u32,
    application_id: u32,
    engine: u32,
    _pad2: u32,
    engine_version: *const c_char,
    project_id: *const c_char,
    render_api: u32,
    _pad3: u32,
}
const _: () = assert!(size_of::<SlPreferencesLayout>() == 144);

const SL_GUID_PREFERENCES: SlStructType =
    sl_guid(0x1ca10965, 0xbf8e, 0x432b, [0x8d, 0xa1, 0x67, 0x16, 0xd8, 0x79, 0xfb, 0x14]);
const SL_GUID_CONSTANTS: SlStructType =
    sl_guid(0xdcd35ad7, 0x4e4a, 0x4bad, [0xa9, 0x0c, 0xe0, 0xc4, 0x9e, 0xb2, 0x3a, 0xfe]);
const SL_GUID_RESOURCE: SlStructType =
    sl_guid(0x3a9d70cf, 0x2418, 0x4b72, [0x83, 0x91, 0x13, 0xf8, 0x72, 0x1c, 0x72, 0x61]);
const SL_GUID_RESOURCE_TAG: SlStructType =
    sl_guid(0x4c6a5aad, 0xb445, 0x496c, [0x87, 0xff, 0x1a, 0xf3, 0x84, 0x5b, 0xe6, 0x53]);
const SL_GUID_DLSS_OPTIONS: SlStructType =
    sl_guid(0x6ac826e4, 0x4c61, 0x4101, [0xa9, 0x2d, 0x63, 0x8d, 0x42, 0x10, 0x57, 0xb8]);
const SL_GUID_DLSS_OPTIMAL: SlStructType =
    sl_guid(0xef1d0957, 0xfd58, 0x4df7, [0xb5, 0x04, 0x8b, 0x69, 0xd8, 0xaa, 0x6b, 0x76]);
const SL_GUID_VIEWPORT: SlStructType =
    sl_guid(0x171b6435, 0x9b3c, 0x4fc8, [0x99, 0x94, 0xfb, 0xe5, 0x25, 0x69, 0xaa, 0xa4]);
const SL_GUID_FEATURE_VERSION: SlStructType =
    sl_guid(0x6d5b51f0, 0x076b, 0x486d, [0x99, 0x95, 0x5a, 0x56, 0x10, 0x43, 0xf5, 0xc1]);

const SL_FEATURE_DLSS: u32 = 0;
const SL_KSDK_VERSION: u64 = (2u64 << 48) | (10u64 << 32) | (3u64 << 16) | 0xfedc;
const SL_PREF_MANUAL_HOOKING: u64 = 1 << 2;
const SL_LOG_VERBOSE: u32 = 2;
const SL_ENGINE_CUSTOM: u32 = 0;
const SL_RENDER_API_VULKAN: u32 = 2;
const SL_BUFFER_DEPTH: u32 = 0;
const SL_BUFFER_MV: u32 = 1;
const SL_BUFFER_SCALING_INPUT_COLOR: u32 = 3;
const SL_BUFFER_SCALING_OUTPUT_COLOR: u32 = 4;
const SL_BUFFER_REACTIVE_MASK: u32 = 36;
const SL_RESOURCE_TEX2D: i8 = 0;
const SL_LIFECYCLE_ONLY_VALID_NOW: i32 = 0;
const SL_DLSS_MODE_MAX_PERFORMANCE: u32 = 1; // 2.0x per-dimension
const SL_INVALID_FLOAT: f32 = f32::MAX;

/// sl::Constants(sl_consts.h L176,kStructVersion2;Boolean=i8)。
#[repr(C)]
struct SlConstants {
    base: SlBaseStructure,
    camera_view_to_clip: [[f32; 4]; 4],
    clip_to_camera_view: [[f32; 4]; 4],
    clip_to_lens_clip: [[f32; 4]; 4],
    clip_to_prev_clip: [[f32; 4]; 4],
    prev_clip_to_clip: [[f32; 4]; 4],
    jitter_offset: [f32; 2],
    mvec_scale: [f32; 2],
    camera_pinhole_offset: [f32; 2],
    camera_pos: [f32; 3],
    camera_up: [f32; 3],
    camera_right: [f32; 3],
    camera_fwd: [f32; 3],
    camera_near: f32,
    camera_far: f32,
    camera_fov: f32,
    camera_aspect_ratio: f32,
    motion_vectors_invalid_value: f32,
    depth_inverted: i8,
    camera_motion_included: i8,
    motion_vectors_3d: i8,
    reset: i8,
    orthographic_projection: i8,
    motion_vectors_dilated: i8,
    motion_vectors_jittered: i8,
    _pad0: u8,
    min_relative_linear_depth_object_separation: f32,
}
const _: () = assert!(size_of::<SlConstants>() == 456);

/// sl::Resource(sl_core_types.h L310,kStructVersion1;ResourceType=i8)。
#[repr(C)]
struct SlResource {
    base: SlBaseStructure,
    res_type: i8,
    _pad0: [u8; 7],
    native: *mut c_void,
    memory: *mut c_void,
    view: *mut c_void,
    state: u32,
    width: u32,
    height: u32,
    native_format: u32,
    mip_levels: u32,
    array_layers: u32,
    gpu_virtual_address: u64,
    flags: u32,
    usage: u32,
    reserved: u32,
}
const _: () = assert!(size_of::<SlResource>() == 112);

/// sl::Extent(sl_consts.h;4×u32)。
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct SlExtent {
    top: u32,
    left: u32,
    width: u32,
    height: u32,
}

/// sl::ResourceTag(sl_core_types.h L382,kStructVersion1;ResourceLifecycle=i32)。
#[repr(C)]
struct SlResourceTag {
    base: SlBaseStructure,
    resource: *const SlResource,
    tag_type: u32,
    lifecycle: i32,
    extent: SlExtent,
}
const _: () = assert!(size_of::<SlResourceTag>() == 64);

/// sl::ViewportHandle(sl_core_types.h L584,kStructVersion1)。
#[repr(C)]
struct SlViewportHandle {
    base: SlBaseStructure,
    value: u32,
}
const _: () = assert!(size_of::<SlViewportHandle>() == 40);

/// sl::DLSSOptions(sl_dlss.h L71,kStructVersion3;Boolean=i8)。
#[repr(C)]
struct SlDlssOptions {
    base: SlBaseStructure,
    mode: u32,
    output_width: u32,
    output_height: u32,
    sharpness: f32,
    pre_exposure: f32,
    exposure_scale: f32,
    color_buffers_hdr: i8,
    indicator_invert_axis_x: i8,
    indicator_invert_axis_y: i8,
    _pad0: u8,
    dlaa_preset: u32,
    quality_preset: u32,
    balanced_preset: u32,
    performance_preset: u32,
    ultra_performance_preset: u32,
    ultra_quality_preset: u32,
    use_auto_exposure: i8,
    alpha_upscaling_enabled: i8,
    _pad1: [u8; 2],
}
const _: () = assert!(size_of::<SlDlssOptions>() == 88);

/// sl::DLSSOptimalSettings(sl_dlss.h L111,kStructVersion1)。
#[repr(C)]
struct SlDlssOptimalSettings {
    base: SlBaseStructure,
    optimal_render_width: u32,
    optimal_render_height: u32,
    optimal_sharpness: f32,
    render_width_min: u32,
    render_height_min: u32,
    render_width_max: u32,
    render_height_max: u32,
}
const _: () = assert!(size_of::<SlDlssOptimalSettings>() == 64);

/// sl::FeatureVersion(sl_core_types.h L667,kStructVersion1)。
#[repr(C)]
struct SlFeatureVersion {
    base: SlBaseStructure,
    version_sl: [u32; 3],
    version_ngx: [u32; 3],
}
const _: () = assert!(size_of::<SlFeatureVersion>() == 56);

// ── SL 函数指针(sl_core_api.h/sl_dlss.h) ─────────────────────────────────────
type FnSlInit = unsafe extern "system" fn(*const SlPreferencesLayout, u64) -> SlResult;
type FnSlShutdown = unsafe extern "system" fn() -> SlResult;
type FnSlIsFeatureLoaded = unsafe extern "system" fn(u32, *mut u8) -> SlResult;
type FnSlGetFeatureFunction = unsafe extern "system" fn(u32, *const c_char, *mut *mut c_void) -> SlResult;
type FnSlGetNewFrameToken = unsafe extern "system" fn(*mut *mut c_void, *const u32) -> SlResult;
type FnSlSetConstants =
    unsafe extern "system" fn(*const SlConstants, *const c_void, *const SlViewportHandle) -> SlResult;
type FnSlEvaluateFeature = unsafe extern "system" fn(
    u32,
    *const c_void,
    *const *const SlBaseStructure,
    u32,
    *mut c_void,
) -> SlResult;
type FnSlFreeResources = unsafe extern "system" fn(u32, *const SlViewportHandle) -> SlResult;
type FnSlGetFeatureVersion = unsafe extern "system" fn(u32, *mut SlFeatureVersion) -> SlResult;
type FnSlDlssSetOptions = unsafe extern "system" fn(*const SlViewportHandle, *const SlDlssOptions) -> SlResult;
type FnSlDlssGetOptimalSettings =
    unsafe extern "system" fn(*const SlDlssOptions, *mut SlDlssOptimalSettings) -> SlResult;

// ── Vulkan 最小面(vulkan_core.h 1.3.296 逐字节核对) ───────────────────────────
type VkInstance = *mut c_void;
type VkPhysicalDevice = *mut c_void;
type VkDevice = *mut c_void;
type VkQueue = *mut c_void;
type VkImage = u64;
type VkImageView = u64;
type VkBuffer = u64;
type VkDeviceMemory = u64;
type VkCommandPool = u64;
type VkCommandBuffer = *mut c_void;
type VkDebugUtilsMessenger = u64;
type VkResult = i32;

const VK_SUCCESS: VkResult = 0;
const VK_STRUCTURE_TYPE_APPLICATION_INFO: u32 = 0;
const VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO: u32 = 1;
const VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO: u32 = 2;
const VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO: u32 = 3;
const VK_STRUCTURE_TYPE_SUBMIT_INFO: u32 = 4;
const VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO: u32 = 5;
const VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO: u32 = 12;
const VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO: u32 = 14;
const VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO: u32 = 15;
const VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO: u32 = 39;
const VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO: u32 = 40;
const VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO: u32 = 42;
const VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER: u32 = 45;
const VK_STRUCTURE_TYPE_DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT: u32 = 1000128004;
const VK_FORMAT_R8_UNORM: u32 = 9;
const VK_FORMAT_R16G16B16A16_SFLOAT: u32 = 97;
const VK_FORMAT_R32G32_SFLOAT: u32 = 103;
const VK_FORMAT_D32_SFLOAT: u32 = 126;
/// `VK_IMAGE_TYPE_2D`(= 1;3D = 2)。跨 device 共享 image 的 `imageType` 两侧
/// 必须同值——否则同一块显存被按不同 tiling 布局解释(G14.12 实锤,见
/// `import_win32_input`)。
const VK_IMAGE_TYPE_2D: i32 = 1;
const VK_IMAGE_LAYOUT_UNDEFINED: i32 = 0;
const VK_IMAGE_LAYOUT_GENERAL: i32 = 1;
const VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL: i32 = 5;
const VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL: i32 = 6;
const VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL: i32 = 7;
const VK_ACCESS_SHADER_READ: u32 = 0x20;
const VK_ACCESS_SHADER_WRITE: u32 = 0x40;
const VK_ACCESS_TRANSFER_READ: u32 = 0x800;
const VK_ACCESS_TRANSFER_WRITE: u32 = 0x1000;
const VK_PIPELINE_STAGE_TOP_OF_PIPE: u32 = 0x1;
const VK_PIPELINE_STAGE_COMPUTE_SHADER: u32 = 0x800;
const VK_PIPELINE_STAGE_TRANSFER: u32 = 0x1000;
const VK_IMAGE_ASPECT_COLOR: u32 = 0x1;
const VK_IMAGE_ASPECT_DEPTH: u32 = 0x2;
const VK_IMAGE_USAGE_TRANSFER_SRC: u32 = 0x1;
const VK_IMAGE_USAGE_TRANSFER_DST: u32 = 0x2;
const VK_IMAGE_USAGE_SAMPLED: u32 = 0x4;
const VK_IMAGE_USAGE_STORAGE: u32 = 0x8;
const VK_BUFFER_USAGE_TRANSFER_SRC: u32 = 0x1;
const VK_BUFFER_USAGE_TRANSFER_DST: u32 = 0x2;
const VK_MEMORY_PROPERTY_DEVICE_LOCAL: u32 = 0x1;
const VK_MEMORY_PROPERTY_HOST_VISIBLE: u32 = 0x2;
const VK_MEMORY_PROPERTY_HOST_COHERENT: u32 = 0x4;
const VK_MEMORY_PROPERTY_HOST_CACHED: u32 = 0x8;
const VK_QUEUE_GRAPHICS: u32 = 0x1;
const VK_QUEUE_COMPUTE: u32 = 0x2;
const VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER: u32 = 0x2;
const VK_DEBUG_SEVERITY_ERROR: u32 = 0x1000;

// ── G14.10b external memory 导入面(SDK 1.3.296 `vulkan_core.h` 核对;与
// render_exec.rs 导出面同族**独立**定义——两模块 FFI 自足纪律)──
/// `VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_PROPERTIES_2`(1.1 core)。
const VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_PROPERTIES_2: u32 = 1_000_059_001;
/// `VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_ID_PROPERTIES`(1.1 core;deviceLUID)。
const VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_ID_PROPERTIES: u32 = 1_000_071_004;
/// `VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_BUFFER_CREATE_INFO`(G14.10f buffer 共享)。
const VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_BUFFER_CREATE_INFO: u32 = 1_000_072_000;
/// `VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_IMAGE_CREATE_INFO`。
const VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_IMAGE_CREATE_INFO: u32 = 1_000_072_001;
/// `VK_STRUCTURE_TYPE_IMPORT_MEMORY_WIN32_HANDLE_INFO_KHR`。
const VK_STRUCTURE_TYPE_IMPORT_MEMORY_WIN32_HANDLE_INFO_KHR: u32 = 1_000_073_000;
/// `VK_STRUCTURE_TYPE_MEMORY_DEDICATED_ALLOCATE_INFO`(1.1 core;OPAQUE_WIN32
/// image 导入实务强制 dedicated,必挂)。
const VK_STRUCTURE_TYPE_MEMORY_DEDICATED_ALLOCATE_INFO: u32 = 1_000_127_001;
/// `VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_WIN32_BIT`(= **0x2**;0x1 是
/// OPAQUE_FD——render_exec 侧初版误用被 validation VUID 三连抓,同错勿犯)。
const VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_WIN32: u32 = 0x2;
/// `VK_QUEUE_FAMILY_EXTERNAL`(= ~1u32;acquire barrier 的 src 家族哨兵)。
const VK_QUEUE_FAMILY_EXTERNAL: u32 = !1u32;

/// `VkExternalMemoryImageCreateInfo`(sType@0/pNext@8/handleTypes@16,size 24)。
#[repr(C)]
struct VkExternalMemoryImageCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    handle_types: u32,
}

/// `VkExternalMemoryBufferCreateInfo`(布局同 image 版;G14.10f buffer 共享)。
#[repr(C)]
struct VkExternalMemoryBufferCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    handle_types: u32,
}

/// `VkImportMemoryWin32HandleInfoKHR`(sType@0/pNext@8/handleType@16/handle@24/
/// name@32,size 40)。
#[repr(C)]
struct VkImportMemoryWin32HandleInfoKHR {
    s_type: u32,
    p_next: *const c_void,
    handle_type: u32,
    handle: *mut c_void,
    name: *const u16,
}

/// `VkMemoryDedicatedAllocateInfo`(sType@0/pNext@8/image@16/buffer@24,size 32)。
#[repr(C)]
struct VkMemoryDedicatedAllocateInfo {
    s_type: u32,
    p_next: *const c_void,
    image: VkImage,
    buffer: VkBuffer,
}

/// `VkPhysicalDeviceIDProperties`(sType@0/pNext@8/deviceUUID[16]@16/
/// driverUUID[16]@32/deviceLUID[8]@48/nodeMask@56/luidValid@60,size 64)。
#[repr(C)]
struct VkPhysicalDeviceIDProperties {
    s_type: u32,
    p_next: *mut c_void,
    device_uuid: [u8; 16],
    driver_uuid: [u8; 16],
    device_luid: [u8; 8],
    device_node_mask: u32,
    device_luid_valid: u32,
}

/// `VkPhysicalDeviceProperties2` 链头(properties 以 2048B blob 超集承载,
/// 本文件 get_phys_props 同律)。
#[repr(C)]
struct VkPhysicalDeviceProperties2Blob {
    s_type: u32,
    p_next: *mut c_void,
    properties: [u64; 256],
}

type FnVkGetPhysicalDeviceProperties2 =
    unsafe extern "system" fn(VkPhysicalDevice, *mut VkPhysicalDeviceProperties2Blob);
type FnVkEnumerateDeviceExtensionProperties = unsafe extern "system" fn(
    VkPhysicalDevice,
    *const c_char,
    *mut u32,
    *mut c_void,
) -> i32;

#[repr(C)]
struct VkApplicationInfo {
    s_type: u32,
    p_next: *const c_void,
    p_application_name: *const c_char,
    application_version: u32,
    p_engine_name: *const c_char,
    engine_version: u32,
    api_version: u32,
}
const _: () = assert!(size_of::<VkApplicationInfo>() == 48);

#[repr(C)]
struct VkInstanceCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: u32,
    p_application_info: *const VkApplicationInfo,
    enabled_layer_count: u32,
    pp_enabled_layer_names: *const *const c_char,
    enabled_extension_count: u32,
    pp_enabled_extension_names: *const *const c_char,
}
const _: () = assert!(size_of::<VkInstanceCreateInfo>() == 64);

#[repr(C)]
struct VkDeviceQueueCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: u32,
    queue_family_index: u32,
    queue_count: u32,
    p_queue_priorities: *const f32,
}
const _: () = assert!(size_of::<VkDeviceQueueCreateInfo>() == 40);

#[repr(C)]
struct VkDeviceCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: u32,
    queue_create_info_count: u32,
    p_queue_create_infos: *const VkDeviceQueueCreateInfo,
    enabled_layer_count: u32,
    pp_enabled_layer_names: *const *const c_char,
    enabled_extension_count: u32,
    pp_enabled_extension_names: *const *const c_char,
    p_enabled_features: *const c_void,
}
const _: () = assert!(size_of::<VkDeviceCreateInfo>() == 72);

#[repr(C)]
struct VkExtent3D {
    width: u32,
    height: u32,
    depth: u32,
}

#[repr(C)]
struct VkImageCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: u32,
    image_type: i32,
    format: u32,
    extent: VkExtent3D,
    mip_levels: u32,
    array_layers: u32,
    samples: i32,
    tiling: i32,
    usage: u32,
    sharing_mode: i32,
    queue_family_index_count: u32,
    p_queue_family_indices: *const u32,
    initial_layout: i32,
}
const _: () = assert!(size_of::<VkImageCreateInfo>() == 88);

#[repr(C)]
struct VkImageSubresourceRange {
    aspect_mask: u32,
    base_mip_level: u32,
    level_count: u32,
    base_array_layer: u32,
    layer_count: u32,
}

#[repr(C)]
struct VkImageViewCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: u32,
    image: VkImage,
    view_type: i32,
    format: u32,
    components: [i32; 4],
    subresource_range: VkImageSubresourceRange,
}
const _: () = assert!(size_of::<VkImageViewCreateInfo>() == 80);

#[repr(C)]
struct VkBufferCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: u32,
    size: u64,
    usage: u32,
    sharing_mode: i32,
    queue_family_index_count: u32,
    p_queue_family_indices: *const u32,
}
const _: () = assert!(size_of::<VkBufferCreateInfo>() == 56);

#[repr(C)]
#[derive(Default)]
struct VkMemoryRequirements {
    size: u64,
    alignment: u64,
    memory_type_bits: u32,
}
const _: () = assert!(size_of::<VkMemoryRequirements>() == 24);

#[repr(C)]
struct VkMemoryAllocateInfo {
    s_type: u32,
    p_next: *const c_void,
    allocation_size: u64,
    memory_type_index: u32,
}
const _: () = assert!(size_of::<VkMemoryAllocateInfo>() == 32);

#[repr(C)]
struct VkCommandPoolCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: u32,
    queue_family_index: u32,
}
const _: () = assert!(size_of::<VkCommandPoolCreateInfo>() == 24);

#[repr(C)]
struct VkCommandBufferAllocateInfo {
    s_type: u32,
    p_next: *const c_void,
    command_pool: VkCommandPool,
    level: i32,
    command_buffer_count: u32,
}
const _: () = assert!(size_of::<VkCommandBufferAllocateInfo>() == 32);

#[repr(C)]
struct VkCommandBufferBeginInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: u32,
    p_inheritance_info: *const c_void,
}
const _: () = assert!(size_of::<VkCommandBufferBeginInfo>() == 32);

#[repr(C)]
struct VkSubmitInfo {
    s_type: u32,
    p_next: *const c_void,
    wait_semaphore_count: u32,
    p_wait_semaphores: *const u64,
    p_wait_dst_stage_mask: *const u32,
    command_buffer_count: u32,
    p_command_buffers: *const VkCommandBuffer,
    signal_semaphore_count: u32,
    p_signal_semaphores: *const u64,
}
const _: () = assert!(size_of::<VkSubmitInfo>() == 72);

#[repr(C)]
struct VkImageMemoryBarrier {
    s_type: u32,
    p_next: *const c_void,
    src_access_mask: u32,
    dst_access_mask: u32,
    old_layout: i32,
    new_layout: i32,
    src_queue_family_index: u32,
    dst_queue_family_index: u32,
    image: VkImage,
    subresource_range: VkImageSubresourceRange,
}
const _: () = assert!(size_of::<VkImageMemoryBarrier>() == 72);

/// `VkBufferMemoryBarrier`(sType=44;G14.10f 导入 buffer 的 EXTERNAL acquire)。
#[repr(C)]
struct VkBufferMemoryBarrier {
    s_type: u32,
    p_next: *const c_void,
    src_access_mask: u32,
    dst_access_mask: u32,
    src_queue_family_index: u32,
    dst_queue_family_index: u32,
    buffer: VkBuffer,
    offset: u64,
    size: u64,
}
const _: () = assert!(size_of::<VkBufferMemoryBarrier>() == 56);

#[repr(C)]
struct VkImageSubresourceLayers {
    aspect_mask: u32,
    mip_level: u32,
    base_array_layer: u32,
    layer_count: u32,
}

#[repr(C)]
struct VkBufferImageCopy {
    buffer_offset: u64,
    buffer_row_length: u32,
    buffer_image_height: u32,
    image_subresource: VkImageSubresourceLayers,
    image_offset: [i32; 3],
    image_extent: VkExtent3D,
}
const _: () = assert!(size_of::<VkBufferImageCopy>() == 56);

#[repr(C)]
struct VkDebugUtilsMessengerCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: u32,
    message_severity: u32,
    message_type: u32,
    pfn_user_callback: *const c_void,
    p_user_data: *mut c_void,
}
const _: () = assert!(size_of::<VkDebugUtilsMessengerCreateInfo>() == 48);

// Vk 函数指针(自 sl.interposer.dll 导出/代理链解析)。
type FnVkGetInstanceProcAddr =
    unsafe extern "system" fn(VkInstance, *const c_char) -> *mut c_void;
type FnVkCreateInstance = unsafe extern "system" fn(*const VkInstanceCreateInfo, *const c_void, *mut VkInstance) -> VkResult;
type FnVkDestroyInstance = unsafe extern "system" fn(VkInstance, *const c_void);
type FnVkEnumeratePhysicalDevices = unsafe extern "system" fn(VkInstance, *mut u32, *mut VkPhysicalDevice) -> VkResult;
type FnVkGetPhysicalDeviceProperties = unsafe extern "system" fn(VkPhysicalDevice, *mut c_void);
type FnVkGetPhysicalDeviceQueueFamilyProperties = unsafe extern "system" fn(VkPhysicalDevice, *mut u32, *mut c_void);
type FnVkGetPhysicalDeviceMemoryProperties = unsafe extern "system" fn(VkPhysicalDevice, *mut c_void);
type FnVkCreateDevice = unsafe extern "system" fn(VkPhysicalDevice, *const VkDeviceCreateInfo, *const c_void, *mut VkDevice) -> VkResult;
type FnVkGetDeviceProcAddr = unsafe extern "system" fn(VkDevice, *const c_char) -> *mut c_void;
type FnVkDestroyDevice = unsafe extern "system" fn(VkDevice, *const c_void);
type FnVkGetDeviceQueue = unsafe extern "system" fn(VkDevice, u32, u32, *mut VkQueue);
type FnVkCreateImage = unsafe extern "system" fn(VkDevice, *const VkImageCreateInfo, *const c_void, *mut VkImage) -> VkResult;
type FnVkDestroyImage = unsafe extern "system" fn(VkDevice, VkImage, *const c_void);
type FnVkGetImageMemoryRequirements = unsafe extern "system" fn(VkDevice, VkImage, *mut VkMemoryRequirements);
type FnVkAllocateMemory = unsafe extern "system" fn(VkDevice, *const VkMemoryAllocateInfo, *const c_void, *mut VkDeviceMemory) -> VkResult;
type FnVkFreeMemory = unsafe extern "system" fn(VkDevice, VkDeviceMemory, *const c_void);
type FnVkBindImageMemory = unsafe extern "system" fn(VkDevice, VkImage, VkDeviceMemory, u64) -> VkResult;
type FnVkCreateImageView = unsafe extern "system" fn(VkDevice, *const VkImageViewCreateInfo, *const c_void, *mut VkImageView) -> VkResult;
type FnVkDestroyImageView = unsafe extern "system" fn(VkDevice, VkImageView, *const c_void);
type FnVkCreateBuffer = unsafe extern "system" fn(VkDevice, *const VkBufferCreateInfo, *const c_void, *mut VkBuffer) -> VkResult;
type FnVkDestroyBuffer = unsafe extern "system" fn(VkDevice, VkBuffer, *const c_void);
type FnVkGetBufferMemoryRequirements = unsafe extern "system" fn(VkDevice, VkBuffer, *mut VkMemoryRequirements);
type FnVkBindBufferMemory = unsafe extern "system" fn(VkDevice, VkBuffer, VkDeviceMemory, u64) -> VkResult;
type FnVkMapMemory = unsafe extern "system" fn(VkDevice, VkDeviceMemory, u64, u64, u32, *mut *mut c_void) -> VkResult;
type FnVkUnmapMemory = unsafe extern "system" fn(VkDevice, VkDeviceMemory);
type FnVkCreateCommandPool = unsafe extern "system" fn(VkDevice, *const VkCommandPoolCreateInfo, *const c_void, *mut VkCommandPool) -> VkResult;
type FnVkDestroyCommandPool = unsafe extern "system" fn(VkDevice, VkCommandPool, *const c_void);
type FnVkAllocateCommandBuffers = unsafe extern "system" fn(VkDevice, *const VkCommandBufferAllocateInfo, *mut VkCommandBuffer) -> VkResult;
type FnVkBeginCommandBuffer = unsafe extern "system" fn(VkCommandBuffer, *const VkCommandBufferBeginInfo) -> VkResult;
type FnVkEndCommandBuffer = unsafe extern "system" fn(VkCommandBuffer) -> VkResult;
type FnVkResetCommandBuffer = unsafe extern "system" fn(VkCommandBuffer, u32) -> VkResult;
type FnVkCmdPipelineBarrier = unsafe extern "system" fn(VkCommandBuffer, u32, u32, u32, u32, *const c_void, u32, *const c_void, u32, *const VkImageMemoryBarrier);
type FnVkCmdCopyBufferToImage = unsafe extern "system" fn(VkCommandBuffer, VkBuffer, VkImage, i32, u32, *const VkBufferImageCopy);
type FnVkCmdCopyImageToBuffer = unsafe extern "system" fn(VkCommandBuffer, VkImage, i32, VkBuffer, u32, *const VkBufferImageCopy);
type FnVkQueueSubmit = unsafe extern "system" fn(VkQueue, u32, *const VkSubmitInfo, u64) -> VkResult;
type FnVkQueueWaitIdle = unsafe extern "system" fn(VkQueue) -> VkResult;
type FnVkDeviceWaitIdle = unsafe extern "system" fn(VkDevice) -> VkResult;
type FnVkCreateDebugUtilsMessenger = unsafe extern "system" fn(VkInstance, *const VkDebugUtilsMessengerCreateInfo, *const c_void, *mut VkDebugUtilsMessenger) -> VkResult;
type FnVkDestroyDebugUtilsMessenger = unsafe extern "system" fn(VkInstance, VkDebugUtilsMessenger, *const c_void);

/// SL 日志回调(prefs.logMessageCallback;原样写 stderr 诊断面——不记 secrets)。
unsafe extern "system" fn sl_log_cb(_log_type: u32, msg: *const c_char) {
    if msg.is_null() {
        return;
    }
    // SAFETY: msg 为 SL 提供的 NUL 结尾字符串(回调期有效,只读)。
    unsafe {
        let s = std::ffi::CStr::from_ptr(msg).to_string_lossy();
        eprintln!("[sl] {s}");
    }
}

/// validation 计数对(ERROR 实错 + NGX 内部伪报豁免;豁免白名单见回调)。
/// p_user_data = Box<ValidationCounts> 稳定地址(U27 同模)。
struct ValidationCounts {
    /// 我方 Vulkan 调用的 ERROR 级实错计数(门判据 = 0)。
    errors: AtomicU64,
    /// NGX/SL 内部私有扩展伪报豁免计数(evidence 全透明登记,不计实错)。
    excluded_ngx_internal: AtomicU64,
    /// 豁免 VUID 名去重登记(白名单外新 VUID 出现 → errors 计数,门红)。
    excluded_names: std::sync::Mutex<Vec<String>>,
}

/// NGX 内部伪报豁免白名单(逐字 VUID;仅这两条经实测确认为 NGX 私有扩展/
/// CUDA interop 内部调用面——本模块从不调 vkCreateCuModuleNVX、从不建
/// VK_IMAGE_TYPE_3D 图像,两条 VUID 命中即结构性归属 NGX 内部):
/// - VkCuModuleCreateInfoNVX-pNext-pNext:NGX 经 VK_NVX_binary_import 私有扩展
///   建 CUDA 模块,公共 validation 层(1.3.296)不识 NVX 私有 struct 链误报;
/// - VkImageViewCreateInfo-image-06728:NGX 为 CUDA 互操作建 3D 图像 + 2D 视图,
///   本模块全 2D 图像不变式保证该 VUID 不可能由我方调用触发。
const NGX_INTERNAL_VUID_WHITELIST: [&str; 2] = [
    "VUID-VkCuModuleCreateInfoNVX-pNext-pNext",
    "VUID-VkImageViewCreateInfo-image-06728",
];

/// validation ERROR 计数回调(p_user_data = Box<ValidationCounts> 稳定地址;U27 同模)。
unsafe extern "system" fn vk_debug_cb(
    _severity: u32,
    _msg_type: u32,
    data: *const c_void,
    user: *mut c_void,
) -> u32 {
    // SAFETY: user 指向 session 持有的 Box<ValidationCounts>(messenger 生命周期严格
    // 短于它);data 为驱动提供的 VkDebugUtilsMessengerCallbackDataEXT(回调期有效):
    // pMessageIdName @ offset 24(ptr,可为 null)——逐字段偏移与 vulkan_core.h 核对;
    // 地址先转 *const *const c_char 再解引(直接 deref *const u8 只得单字节,曾致
    // 把指针低字节当地址的 0xc0000005——已修,禁回归)。
    unsafe {
        if !user.is_null() {
            let counts = &*(user as *const ValidationCounts);
            if _severity & VK_DEBUG_SEVERITY_ERROR != 0 {
                let mut id_name = "";
                if !data.is_null() {
                    let name_ptr = *((data as *const u8).add(24) as *const *const c_char);
                    if !name_ptr.is_null() {
                        let mut len = 0usize;
                        while *name_ptr.add(len) != 0 {
                            len += 1;
                        }
                        id_name = core::str::from_utf8(core::slice::from_raw_parts(
                            name_ptr as *const u8,
                            len,
                        ))
                        .unwrap_or("");
                    }
                }
                if NGX_INTERNAL_VUID_WHITELIST.contains(&id_name) {
                    counts.excluded_ngx_internal.fetch_add(1, Ordering::Relaxed);
                    if let Ok(mut names) = counts.excluded_names.lock()
                        && !names.iter().any(|n| n == id_name)
                        && names.len() < 4
                    {
                        names.push(id_name.to_string());
                    }
                } else {
                    counts.errors.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }
    0 // VK_FALSE——不 abort。
}

/// SL/VK 已解析符号集。
#[derive(Clone, Copy)]
struct SlVkFns {
    sl_init: FnSlInit,
    sl_shutdown: FnSlShutdown,
    sl_is_feature_loaded: FnSlIsFeatureLoaded,
    sl_get_feature_function: FnSlGetFeatureFunction,
    sl_get_new_frame_token: FnSlGetNewFrameToken,
    sl_set_constants: FnSlSetConstants,
    sl_evaluate_feature: FnSlEvaluateFeature,
    sl_free_resources: FnSlFreeResources,
    sl_get_feature_version: FnSlGetFeatureVersion,
    vk_gipa: FnVkGetInstanceProcAddr,
    vk_gdpa: FnVkGetDeviceProcAddr,
}

struct DlssVkFns2 {
    dlss_set_options: FnSlDlssSetOptions,
    dlss_get_optimal: FnSlDlssGetOptimalSettings,
}

struct VkDevFns {
    destroy_device: FnVkDestroyDevice,
    get_device_queue: FnVkGetDeviceQueue,
    create_image: FnVkCreateImage,
    destroy_image: FnVkDestroyImage,
    get_image_memory_requirements: FnVkGetImageMemoryRequirements,
    allocate_memory: FnVkAllocateMemory,
    free_memory: FnVkFreeMemory,
    bind_image_memory: FnVkBindImageMemory,
    create_image_view: FnVkCreateImageView,
    destroy_image_view: FnVkDestroyImageView,
    create_buffer: FnVkCreateBuffer,
    destroy_buffer: FnVkDestroyBuffer,
    get_buffer_memory_requirements: FnVkGetBufferMemoryRequirements,
    bind_buffer_memory: FnVkBindBufferMemory,
    map_memory: FnVkMapMemory,
    unmap_memory: FnVkUnmapMemory,
    create_command_pool: FnVkCreateCommandPool,
    destroy_command_pool: FnVkDestroyCommandPool,
    allocate_command_buffers: FnVkAllocateCommandBuffers,
    begin_command_buffer: FnVkBeginCommandBuffer,
    end_command_buffer: FnVkEndCommandBuffer,
    reset_command_buffer: FnVkResetCommandBuffer,
    cmd_pipeline_barrier: FnVkCmdPipelineBarrier,
    cmd_copy_buffer_to_image: FnVkCmdCopyBufferToImage,
    cmd_copy_image_to_buffer: FnVkCmdCopyImageToBuffer,
    queue_submit: FnVkQueueSubmit,
    queue_wait_idle: FnVkQueueWaitIdle,
    device_wait_idle: FnVkDeviceWaitIdle,
}

struct VkImageRes {
    image: VkImage,
    memory: VkDeviceMemory,
    view: VkImageView,
    layout: i32,
    format: u32,
    w: u32,
    h: u32,
    aspect: u32,
}

/// DLSS(Vulkan interop)session——safe 公共面。
pub struct DlssVkSession {
    fns: SlVkFns,
    dlss: DlssVkFns2,
    dev: VkDevFns,
    instance: VkInstance,
    device: VkDevice,
    queue: VkQueue,
    cmd_pool: VkCommandPool,
    cmd: VkCommandBuffer,
    viewport: SlViewportHandle,
    in_w: u32,
    in_h: u32,
    out_w: u32,
    out_h: u32,
    color_in: VkImageRes,
    depth_in: VkImageRes,
    mv_in: VkImageRes,
    reactive_in: VkImageRes,
    color_out: VkImageRes,
    staging: VkBuffer,
    staging_mem: VkDeviceMemory,
    staging_size: u64,
    readback: VkBuffer,
    readback_mem: VkDeviceMemory,
    readback_size: u64,
    messenger: VkDebugUtilsMessenger,
    validation_counter: *mut ValidationCounts,
    gpu_name: String,
    ngx_version: String,
    dlls: Vec<DllProvenance>,
    log_tail: Vec<String>,
    shutdown_done: bool,
    /// G14.11:reactive 恒零内容已上传一次(驻留 evaluate 路径专用)。
    /// `reactive=None` 语义 = 全零 mask,内容逐帧不变——首帧上传后 image 内容
    /// 与 layout(SHADER_READ_ONLY_OPTIMAL)均恒定,后续帧跳过 staging
    /// map/fill/unmap 与 buffer→image copy(t100 面每帧省 2MB memset + 2MB
    /// 拷贝);`reactive=Some(..)` 帧照常上传并复位本标志(内容已被覆写)。
    /// host 路径 `frame_impl_ext` 不消费本标志(M-a 锚共享面 0-byte)。
    reactive_zero_resident: bool,
    /// G14.10b:device 创建时 external memory 两扩展是否已启用(设备扩展在位
    /// 才启;不在位 → 导入入口 fail-closed,既有路径行为不变)。
    external_memory_enabled: bool,
    /// G14.10b:物理设备 LUID(创建期实采;`None` = 驱动报无效/符号缺)。
    device_luid: Option<[u8; 8]>,
    /// G14.10b:外部导入输入([color, depth, mv] = (资源, 导出侧 usage 位);
    /// `None` = 未导入。image/memory/view 归本 session,Drop 逆序释放——导入
    /// memory 的 vkFreeMemory 仅解引用,不影响导出侧分配。
    ext_inputs: [Option<(VkImageRes, u32)>; 3],
    /// G14.10f:外部导入输入 **buffer**([color, depth, mv] = (buffer, memory,
    /// 字节数);`None` = 未导入)。跨 device OPTIMAL image 布局解释不一致的
    /// 正解——buffer 线性布局无歧义,消费经每帧 `vkCmdCopyBufferToImage` 进
    /// session 自建输入 image(与 host staging 路同一批 image/同一 tag 面)。
    /// buffer/memory 归本 session,Drop 逆序释放(free 仅解引用)。
    ext_input_bufs: [Option<(VkBuffer, VkDeviceMemory, u64)>; 3],
    /// G14.10b:cmd/queue 所属家族(acquire barrier 的 dst 家族;创建期定格)。
    queue_family: u32,
}

// session 句柄集非 Send(单线程门 harness 语义;不显式 impl Send/Sync)。

fn sl_result_name(r: SlResult) -> String {
    let names = [
        "eOk", "eErrorIO", "eErrorDriverOutOfDate", "eErrorOSOutOfDate", "eErrorOSDisabledHWS",
        "eErrorDeviceNotCreated", "eErrorNoSupportedAdapterFound", "eErrorAdapterNotSupported",
        "eErrorNoPlugins", "eErrorVulkanAPI", "eErrorDXGIAPI", "eErrorD3DAPI", "eErrorNRDAPI",
        "eErrorNVAPI", "eErrorReflexAPI", "eErrorNGXFailed", "eErrorJSONParsing",
        "eErrorMissingProxy", "eErrorMissingResourceState", "eErrorInvalidIntegration",
        "eErrorMissingInputParameter", "eErrorNotInitialized", "eErrorComputeFailed",
        "eErrorInitNotCalled", "eErrorExceptionHandler", "eErrorInvalidParameter",
        "eErrorMissingConstants", "eErrorDuplicatedConstants", "eErrorMissingOrInvalidAPI",
        "eErrorCommonConstantsMissing", "eErrorUnsupportedInterface", "eErrorFeatureMissing",
        "eErrorFeatureNotSupported", "eErrorFeatureMissingHooks", "eErrorFeatureFailedToLoad",
        "eErrorFeatureWrongPriority", "eErrorFeatureMissingDependency",
        "eErrorFeatureManagerInvalidState", "eErrorInvalidState", "eWarnOutOfVRAM",
    ];
    if (r as usize) < names.len() {
        format!("{}({r})", names[r as usize])
    } else {
        format!("unknown({r})")
    }
}

fn f32_to_f16(v: f32) -> u16 {
    let bits = v.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exp32 = ((bits >> 23) & 0xff) as i32;
    let mant = bits & 0x007f_ffff;
    if exp32 == 0xff {
        // inf/nan → f16 inf/nan 保号
        return if mant == 0 { sign | 0x7c00 } else { sign | 0x7e00 };
    }
    let exp = exp32 - 127 + 15;
    if exp >= 31 {
        return sign | 0x7c00; // 上溢 → inf
    }
    if exp <= 0 {
        if exp < -10 {
            return sign; // 下溢 → 0
        }
        let m = mant | 0x0080_0000;
        let shift = (14 - exp) as u32;
        let mut m16 = m >> shift;
        let rem = m & ((1u32 << shift) - 1);
        let halfway = 1u32 << (shift - 1);
        if rem > halfway || (rem == halfway && (m16 & 1) == 1) {
            m16 += 1;
        }
        return sign | m16 as u16;
    }
    let mut m16 = mant >> 13;
    let rem = mant & 0x1fff;
    if rem > 0x1000 || (rem == 0x1000 && (m16 & 1) == 1) {
        m16 += 1;
    }
    if m16 == 0x400 {
        m16 = 0;
        return sign | (((exp + 1) as u16) << 10) | m16 as u16;
    }
    sign | ((exp as u16) << 10) | m16 as u16
}

fn f16_to_f32(h: u16) -> f32 {
    let sign = ((h as u32) & 0x8000) << 16;
    let exp = ((h >> 10) & 0x1f) as u32;
    let mant = (h & 0x3ff) as u32;
    let bits = if exp == 0 {
        if mant == 0 {
            sign
        } else {
            // subnormal f16 → normalized f32。推导:mant 左移 k 次至 hidden
            // 位(0x400),值 = 1.frac × 2^(−14−k) → f32 指数字段 = 113 − k。
            // G14.11 修正(fsr 对拍臂检出):e 初值曾为 −1 使字段恒 112 − k,
            // 全体 subnormal 解码为正确值一半——e 初值归 0(涉 vendor 臂
            // digest 锚,修复后 G14.12 统一重收割)。
            let mut e = 0i32;
            let mut m = mant;
            while (m & 0x400) == 0 {
                m <<= 1;
                e -= 1;
            }
            m &= 0x3ff;
            sign | (((127 - 15 + 1 + e) as u32) << 23) | (m << 13)
        }
    } else if exp == 31 {
        sign | 0x7f80_0000 | (mant << 13)
    } else {
        sign | ((exp + 112) << 23) | (mant << 13)
    };
    f32::from_bits(bits)
}

/// G14.7 延续波：逐元素独立转换的像素带并行带数（`RURIX_VENDOR_PAR=0` → 单带
/// 串行对照臂；显式 N → N 带；缺省 = min(可用并行度, 8)）。< 128Kpx 小格维持
/// 单带——线程 spawn 开销（约 0.05ms/线程）在小格上无净收益。带数仅改 host
/// 写入的线程归属：每元素经同一 `f32_to_f16`/`f16_to_f32` 同式转换、元素间
/// 零依赖，输出最终字节面与单带串行逐位一致（M-d 末帧 digest 冻结锚守护
/// 机核面，漂移即 RED）。
fn par_band_count(px: usize) -> usize {
    par_band_count_with(px, std::env::var("RURIX_VENDOR_PAR").ok().as_deref())
}

/// 带数决策纯函数（env 面剥离，host 可测）：< PAR_MIN_PX 小格恒单带；
/// `Some("0")` → 单带串行对照臂；显式 N → N 带；None → min(可用并行度, 8)。
fn par_band_count_with(px: usize, par_env: Option<&str>) -> usize {
    const PAR_MIN_PX: usize = 1 << 17; // 131072px
    if px < PAR_MIN_PX {
        return 1;
    }
    let n = match par_env {
        Some("0") => 1,
        Some(v) => v.parse::<usize>().ok().map_or(8, |m| m.max(1)),
        None => std::thread::available_parallelism()
            .map_or(1, |n| n.get())
            .min(8),
    };
    // 每带至少 64Kpx（spawn 开销摊薄）；带数不超像素容量。
    n.min(px.div_ceil(1 << 16)).max(1)
}

/// G14.7：四区打包串行主体（G14.3/G14.6 原循环逐字保留，参数化为带内切片；
/// DLSS 臂 mapped staging 直写与 FSR 臂常驻 pack vec 共用同一事实源）。
#[allow(clippy::too_many_arguments)]
fn pack_vendor_inputs_serial(
    color: &[f32],
    depth: &[f32],
    mv: &[f32],
    reactive: Option<&[f32]>,
    color_out: &mut [u8],
    depth_out: &mut [u8],
    mv_out: &mut [u8],
    reac_out: &mut [u8],
) {
    for (o, rgb) in color_out.chunks_exact_mut(8).zip(color.chunks_exact(3)) {
        o[0..2].copy_from_slice(&f32_to_f16(rgb[0]).to_le_bytes());
        o[2..4].copy_from_slice(&f32_to_f16(rgb[1]).to_le_bytes());
        o[4..6].copy_from_slice(&f32_to_f16(rgb[2]).to_le_bytes());
        o[6..8].copy_from_slice(&f32_to_f16(1.0).to_le_bytes());
    }
    for (o, &d) in depth_out.chunks_exact_mut(4).zip(depth.iter()) {
        o.copy_from_slice(&d.to_le_bytes());
    }
    for (o, &m) in mv_out.chunks_exact_mut(4).zip(mv.iter()) {
        o.copy_from_slice(&m.to_le_bytes());
    }
    match reactive {
        Some(r) => {
            for (o, &v) in reac_out.iter_mut().zip(r.iter()) {
                *o = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
        None => reac_out.fill(0),
    }
}

/// G14.7：四区打包像素带并行（输入切片长度 = 全帧像素口径；输出切片长度 =
/// px*8 / px*4 / px*8 / px 精确贴合）。带切分仅改线程归属，带内逐值同式
/// 同序——上传/驻留字节面与串行逐位一致。
#[allow(clippy::too_many_arguments)]
fn pack_vendor_inputs(
    px: usize,
    color: &[f32],
    depth: &[f32],
    mv: &[f32],
    reactive: Option<&[f32]>,
    color_out: &mut [u8],
    depth_out: &mut [u8],
    mv_out: &mut [u8],
    reac_out: &mut [u8],
) {
    let bands = par_band_count(px);
    pack_vendor_inputs_bands(px, color, depth, mv, reactive, color_out, depth_out, mv_out, reac_out, bands);
}

/// 显式带数变体（测试面直接驱动，绕 env）。
#[allow(clippy::too_many_arguments)]
fn pack_vendor_inputs_bands(
    px: usize,
    color: &[f32],
    depth: &[f32],
    mv: &[f32],
    reactive: Option<&[f32]>,
    color_out: &mut [u8],
    depth_out: &mut [u8],
    mv_out: &mut [u8],
    reac_out: &mut [u8],
    bands: usize,
) {
    if bands <= 1 {
        pack_vendor_inputs_serial(color, depth, mv, reactive, color_out, depth_out, mv_out, reac_out);
        return;
    }
    let band_px = px.div_ceil(bands);
    std::thread::scope(|s| {
        let mut co_it = color_out.chunks_mut(band_px * 8);
        let mut do_it = depth_out.chunks_mut(band_px * 4);
        let mut mo_it = mv_out.chunks_mut(band_px * 8);
        let mut ro_it = reac_out.chunks_mut(band_px);
        let mut ci_it = color.chunks(band_px * 3);
        let mut di_it = depth.chunks(band_px);
        let mut mi_it = mv.chunks(band_px * 2);
        let mut b_lo = 0usize; // 当前带像素起点（reactive 输入带定位用）
        loop {
            let (Some(cb), Some(db), Some(mb), Some(rb)) =
                (co_it.next(), do_it.next(), mo_it.next(), ro_it.next())
            else {
                break;
            };
            let (Some(ci), Some(di), Some(mi)) = (ci_it.next(), di_it.next(), mi_it.next())
            else {
                break;
            };
            let npx = cb.len() / 8;
            let ri = reactive.map(|r| &r[b_lo..b_lo + npx]);
            b_lo += npx;
            s.spawn(move || {
                pack_vendor_inputs_serial(ci, di, mi, ri, cb, db, mb, rb);
            });
        }
    });
}

/// G14.7：f16→f32 输出转换串行主体（逐值同式同序，RGB 三通道、alpha 跳过）。
fn convert_out_serial(data: &[u16], out: &mut [f32]) {
    for (o, px4) in out.chunks_exact_mut(3).zip(data.chunks_exact(4)) {
        o[0] = f16_to_f32(px4[0]);
        o[1] = f16_to_f32(px4[1]);
        o[2] = f16_to_f32(px4[2]);
    }
}

/// G14.7：连续 RGBA f16 回读 → f32 转换像素带并行（DLSS 臂；data 长度 =
/// px*4，out 长度 = px*3）。
fn convert_out_par(data: &[u16], out: &mut [f32]) {
    let px = out.len() / 3;
    let bands = par_band_count(px);
    convert_out_par_bands(data, out, bands);
}

/// 显式带数变体（测试面直接驱动，绕 env）。
fn convert_out_par_bands(data: &[u16], out: &mut [f32], bands: usize) {
    let px = out.len() / 3;
    if bands <= 1 {
        convert_out_serial(data, out);
        return;
    }
    let band_px = px.div_ceil(bands);
    std::thread::scope(|s| {
        let mut o_it = out.chunks_mut(band_px * 3);
        let mut d_it = data.chunks(band_px * 4);
        loop {
            let (Some(ob), Some(db)) = (o_it.next(), d_it.next()) else {
                break;
            };
            s.spawn(move || convert_out_serial(db, ob));
        }
    });
}

/// G14.7：行距对齐 RGBA f16 回读 → f32 转换行带并行（FSR 臂；data 覆盖
/// (oh−1)·rp2 + ow·4 个 u16 的触及区间，行内只消费前 ow·4）。
fn convert_out_pitched_par(data: &[u16], rp2: usize, ow: usize, oh: usize, out: &mut [f32]) {
    let bands = par_band_count(ow * oh).min(oh);
    convert_out_pitched_par_bands(data, rp2, ow, oh, out, bands);
}

/// 显式带数变体（测试面直接驱动，绕 env）。
fn convert_out_pitched_par_bands(data: &[u16], rp2: usize, ow: usize, oh: usize, out: &mut [f32], bands: usize) {
    let bands = bands.min(oh);
    if bands <= 1 {
        for (y, out_row) in out.chunks_exact_mut(ow * 3).enumerate() {
            let row = &data[y * rp2..y * rp2 + ow * 4];
            convert_out_serial(row, out_row);
        }
        return;
    }
    let band_rows = oh.div_ceil(bands);
    let mut o_it = out.chunks_mut(band_rows * ow * 3);
    let mut r0 = 0usize;
    std::thread::scope(|s| {
        while let Some(ob) = o_it.next() {
            let rows = ob.len() / (ow * 3);
            let db = &data[r0 * rp2..];
            s.spawn(move || {
                for (y, out_row) in ob.chunks_exact_mut(ow * 3).enumerate() {
                    let row = &db[y * rp2..y * rp2 + ow * 4];
                    convert_out_serial(row, out_row);
                }
            });
            r0 += rows;
        }
    });
}

impl DlssVkSession {
    /// 创建 DLSS Vulkan interop session(slInit → SL 代理建 instance/device → DLSS
    /// 插件装载 → 资源面)。`validation` = RURIX_VK_VALIDATION 语义(KHRONOS 层 +
    /// debug messenger,ERROR 级计数)。fail-closed。
    pub fn create(
        sdk_dir: &Path,
        in_size: (u32, u32),
        out_size: (u32, u32),
        validation: bool,
    ) -> Result<Self, VendorError> {
        let bin_dir = sdk_dir.join("bin").join("x64");
        let interposer = bin_dir.join("sl.interposer.dll");
        if !interposer.is_file() {
            return Err(VendorError::DllNotFound(format!(
                "sl.interposer.dll 不在位: {}",
                interposer.display()
            )));
        }
        for req in ["sl.common.dll", "sl.dlss.dll", "nvngx_dlss.dll"] {
            let p = bin_dir.join(req);
            if !p.is_file() {
                return Err(VendorError::DllNotFound(format!("{req} 不在位: {}", p.display())));
            }
        }
        let dlls = ["sl.interposer.dll", "sl.common.dll", "sl.dlss.dll", "nvngx_dlss.dll"]
            .iter()
            .map(|n| {
                let (sha, bytes) = sha256_file(&bin_dir.join(n))?;
                Ok(DllProvenance { name: n.to_string(), sha256: sha, bytes })
            })
            .collect::<Result<Vec<_>, VendorError>>()?;

        // SAFETY(装载):interposer 路径实测在树;LoadLibraryExW ALTERED_SEARCH_PATH;
        // 进程常驻不 FreeLibrary(U1 纪律)。
        let lib = loader::open(&interposer);
        if lib.is_null() {
            return Err(VendorError::DllNotFound(format!(
                "sl.interposer.dll 装载失败: {}",
                interposer.display()
            )));
        }
        macro_rules! sym {
            ($name:literal, $ty:ty) => {
                // SAFETY: lib 有效;$name NUL 结尾字面量;cast_sym null 校验。
                match unsafe { cast_sym::<$ty>(loader::sym(lib, concat!($name, "\0").as_ptr() as *const c_char)) } {
                    Some(f) => f,
                    None => return Err(VendorError::SymbolMissing($name.into())),
                }
            };
        }
        let fns = SlVkFns {
            sl_init: sym!("slInit", FnSlInit),
            sl_shutdown: sym!("slShutdown", FnSlShutdown),
            sl_is_feature_loaded: sym!("slIsFeatureLoaded", FnSlIsFeatureLoaded),
            sl_get_feature_function: sym!("slGetFeatureFunction", FnSlGetFeatureFunction),
            sl_get_new_frame_token: sym!("slGetNewFrameToken", FnSlGetNewFrameToken),
            sl_set_constants: sym!("slSetConstants", FnSlSetConstants),
            sl_evaluate_feature: sym!("slEvaluateFeature", FnSlEvaluateFeature),
            sl_free_resources: sym!("slFreeResources", FnSlFreeResources),
            sl_get_feature_version: sym!("slGetFeatureVersion", FnSlGetFeatureVersion),
            vk_gipa: sym!("vkGetInstanceProcAddr", FnVkGetInstanceProcAddr),
            // SAFETY: lib 有效;NUL 结尾字面量;cast_sym null 校验——
            // vkGetDeviceProcAddr 需实例级解析——自 interposer 直接导出取址
            // (导出表实测含 vkGetDeviceProcAddr)。
            vk_gdpa: unsafe {
                cast_sym::<FnVkGetDeviceProcAddr>(loader::sym(lib, c"vkGetDeviceProcAddr".as_ptr()))
                    .ok_or_else(|| VendorError::SymbolMissing("vkGetDeviceProcAddr".into()))?
            },
        };

        // ── slInit(eUseManualHooking,featuresToLoad=[DLSS]) ──
        let plugin_dir_wide: Vec<u16> = bin_dir
            .as_os_str()
            .to_string_lossy()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let plugin_paths: [*const u16; 1] = [plugin_dir_wide.as_ptr()];
        let features: [u32; 1] = [SL_FEATURE_DLSS];
        let engine_ver = c"1.0.0";
        // NGX 硬校验:projectId 必须为合法 GUID 字面(纯 hex + 连字符——初版嵌入
        // 非 hex 字符被 NGX 拒,致 NGX 初始化失败、DLSS context 缺失;SL 日志实测)。
        let project_id = c"a7f31c25-8c4e-4b2d-9e16-10a1c0de5001";
        let prefs = SlPreferencesLayout {
            base: sl_base(SL_GUID_PREFERENCES, 1),
            show_console: 0,
            _pad0: [0; 3],
            log_level: SL_LOG_VERBOSE,
            paths_to_plugins: plugin_paths.as_ptr(),
            num_paths_to_plugins: 1,
            _pad1: 0,
            path_to_logs_and_data: std::ptr::null(),
            allocate_callback: std::ptr::null_mut(),
            release_callback: std::ptr::null_mut(),
            log_message_callback: sl_log_cb as *mut c_void,
            flags: SL_PREF_MANUAL_HOOKING,
            features_to_load: features.as_ptr(),
            num_features_to_load: 1,
            application_id: 0,
            engine: SL_ENGINE_CUSTOM,
            _pad2: 0,
            engine_version: engine_ver.as_ptr(),
            project_id: project_id.as_ptr(),
            render_api: SL_RENDER_API_VULKAN,
            _pad3: 0,
        };
        // SAFETY: prefs 为本函数栈上完整初始化值,GUID/版本头匹配 slInit 期望;
        // plugin_dir_wide/features/engine_ver/project_id 调用期存活。
        let r = unsafe { (fns.sl_init)(&prefs, SL_KSDK_VERSION) };
        if r != SL_OK {
            return Err(VendorError::VendorCall(format!("slInit → {}", sl_result_name(r))));
        }

        // ── instance(SL 代理 vkCreateInstance;validation 可选) ──
        // SAFETY: vk_gipa 为 SL 代理入口;全局级符号经 NULL instance 解析(Vulkan 装载语义)。
        let create_instance: FnVkCreateInstance = unsafe {
            cast_sym((fns.vk_gipa)(std::ptr::null_mut(), c"vkCreateInstance".as_ptr()))
                .ok_or_else(|| VendorError::SymbolMissing("vkCreateInstance(proxy)".into()))?
        };

        let mut layers: Vec<*const c_char> = Vec::new();
        let mut exts: Vec<*const c_char> = Vec::new();
        let khronos = c"VK_LAYER_KHRONOS_validation";
        let debug_utils = c"VK_EXT_debug_utils";
        if validation {
            layers.push(khronos.as_ptr());
            exts.push(debug_utils.as_ptr());
        }
        let app_name = c"rurix-g13-m167";
        let app = VkApplicationInfo {
            s_type: VK_STRUCTURE_TYPE_APPLICATION_INFO,
            p_next: std::ptr::null(),
            p_application_name: app_name.as_ptr(),
            application_version: 1,
            p_engine_name: app_name.as_ptr(),
            engine_version: 1,
            api_version: (1 << 22) | (3 << 12), // VK_API_VERSION_1_3
        };
        let ici = VkInstanceCreateInfo {
            s_type: VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            p_application_info: &app,
            enabled_layer_count: layers.len() as u32,
            pp_enabled_layer_names: if layers.is_empty() { std::ptr::null() } else { layers.as_ptr() },
            enabled_extension_count: exts.len() as u32,
            pp_enabled_extension_names: if exts.is_empty() { std::ptr::null() } else { exts.as_ptr() },
        };
        let mut instance: VkInstance = std::ptr::null_mut();
        // SAFETY: app/ici 栈上存活;SL 代理在创建时附加其必需实例扩展。
        let r = unsafe { create_instance(&ici, std::ptr::null(), &mut instance) };
        if r != VK_SUCCESS || instance.is_null() {
            return Err(VendorError::ApiError(format!("vkCreateInstance(proxy) → {r}")));
        }

        // 实例级符号:spec 要求非 NULL instance(NULL 仅解析全局级;SL 代理同律——
        // 初版 NULL 解析 vkGetPhysicalDeviceProperties 返 NULL,实测击穿)。
        // SAFETY: instance 有效;符号名 NUL 结尾字面量;cast_sym null 校验(下同五条同律)。
        let enumerate_physical: FnVkEnumeratePhysicalDevices = unsafe {
            cast_sym((fns.vk_gipa)(instance, c"vkEnumeratePhysicalDevices".as_ptr()))
                .ok_or_else(|| VendorError::SymbolMissing("vkEnumeratePhysicalDevices".into()))?
        };
        // SAFETY: instance 有效;符号名 NUL 结尾;cast_sym null 校验。
        let get_phys_props: FnVkGetPhysicalDeviceProperties = unsafe {
            cast_sym((fns.vk_gipa)(instance, c"vkGetPhysicalDeviceProperties".as_ptr()))
                .ok_or_else(|| VendorError::SymbolMissing("vkGetPhysicalDeviceProperties".into()))?
        };
        // SAFETY: instance 有效;符号名 NUL 结尾;cast_sym null 校验。
        let get_queue_props: FnVkGetPhysicalDeviceQueueFamilyProperties = unsafe {
            cast_sym((fns.vk_gipa)(instance, c"vkGetPhysicalDeviceQueueFamilyProperties".as_ptr()))
                .ok_or_else(|| VendorError::SymbolMissing("vkGetPhysicalDeviceQueueFamilyProperties".into()))?
        };
        // SAFETY: instance 有效;符号名 NUL 结尾;cast_sym null 校验。
        let get_mem_props: FnVkGetPhysicalDeviceMemoryProperties = unsafe {
            cast_sym((fns.vk_gipa)(instance, c"vkGetPhysicalDeviceMemoryProperties".as_ptr()))
                .ok_or_else(|| VendorError::SymbolMissing("vkGetPhysicalDeviceMemoryProperties".into()))?
        };
        // SAFETY: instance 有效;符号名 NUL 结尾;cast_sym null 校验。
        let create_device: FnVkCreateDevice = unsafe {
            cast_sym((fns.vk_gipa)(instance, c"vkCreateDevice".as_ptr()))
                .ok_or_else(|| VendorError::SymbolMissing("vkCreateDevice(proxy)".into()))?
        };
        // create 期 fail-closed 符号预检(Drop 期经 vk_gipa 再取销毁用实例函数;
        // 此处只验符号在位,绑定故意不消费)。
        // SAFETY: instance 有效;符号名 NUL 结尾;cast_sym null 校验。
        let _destroy_instance: FnVkDestroyInstance = unsafe {
            cast_sym((fns.vk_gipa)(instance, c"vkDestroyInstance".as_ptr()))
                .ok_or_else(|| VendorError::SymbolMissing("vkDestroyInstance".into()))?
        };

        // validation messenger(instance 创建后立刻挂;ERROR 级计数)。
        let mut messenger: VkDebugUtilsMessenger = 0;
        let mut validation_counter: *mut ValidationCounts = std::ptr::null_mut();
        if validation {
            // SAFETY: instance 有效;符号名 NUL 结尾;cast_sym null 校验(缺符号 = 无
            // 校验层环境,None 面下行容忍)。
            let create_msgr: Option<FnVkCreateDebugUtilsMessenger> = unsafe {
                cast_sym((fns.vk_gipa)(instance, c"vkCreateDebugUtilsMessengerEXT".as_ptr()))
            };
            if let Some(create_msgr) = create_msgr {
                let counter = Box::new(ValidationCounts {
                    errors: AtomicU64::new(0),
                    excluded_ngx_internal: AtomicU64::new(0),
                    excluded_names: std::sync::Mutex::new(Vec::new()),
                });
                validation_counter = Box::into_raw(counter);
                let ci = VkDebugUtilsMessengerCreateInfo {
                    s_type: VK_STRUCTURE_TYPE_DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
                    p_next: std::ptr::null(),
                    flags: 0,
                    message_severity: VK_DEBUG_SEVERITY_ERROR | 0x100,
                    message_type: 1 | 2 | 4,
                    pfn_user_callback: vk_debug_cb as *const c_void,
                    p_user_data: validation_counter as *mut c_void,
                };
                // SAFETY: ci 栈上存活;p_user_data 指向 Box 化计数器(session 持有,
                // messenger 先于 instance 销毁,生命周期严格短于该堆变量)。
                let r = unsafe { create_msgr(instance, &ci, std::ptr::null(), &mut messenger) };
                if r != VK_SUCCESS {
                    messenger = 0;
                }
            }
        }

        // ── 物理设备选取(NVIDIA 独显优先) ──
        let mut count = 0u32;
        // SAFETY: instance 有效。
        unsafe { enumerate_physical(instance, &mut count, std::ptr::null_mut()) };
        if count == 0 {
            return Err(VendorError::DeviceUnavailable("零 Vulkan 物理设备".into()));
        }
        let mut phys = vec![std::ptr::null_mut::<c_void>(); count as usize];
        // SAFETY: phys 容量 = count。
        unsafe { enumerate_physical(instance, &mut count, phys.as_mut_ptr()) };
        let mut chosen: Option<VkPhysicalDevice> = None;
        let mut gpu_name = String::new();
        for &p in &phys {
            // VkPhysicalDeviceProperties:vendorID@8,deviceType@16,deviceName[256]@20
            // (render_exec 同源偏移,SDK 头核对);2048B align(8) blob 超集承载防越界写。
            let mut blob = [0u64; 256];
            // SAFETY: blob 2048B ≥ 真实结构(约 824B),align 8 满足。
            unsafe { get_phys_props(p, blob.as_mut_ptr() as *mut c_void) };
            let bytes = blob.as_ptr() as *const u8;
            // SAFETY: 只读 blob 前 276 字节(结构已知前缀)。
            let (vendor, dev_type, name) = unsafe {
                let vendor = (bytes.add(8) as *const u32).read_unaligned();
                let dev_type = (bytes.add(16) as *const i32).read_unaligned();
                let name_ptr = bytes.add(20);
                let len = (0..256).position(|i| *name_ptr.add(i) == 0).unwrap_or(256);
                let name = String::from_utf8_lossy(core::slice::from_raw_parts(name_ptr, len)).into_owned();
                (vendor, dev_type, name)
            };
            let is_nv = vendor == 0x10de;
            let is_discrete = dev_type == 2;
            // 首选兜底(首个设备),NVIDIA 独显覆盖(NV+discrete 双位才替换——
            // 合并条件与原嵌套 if 语义逐点等价:chosen 为空必取,否则仅 NV 独显替换)。
            if chosen.is_none() || (is_nv && is_discrete) {
                chosen = Some(p);
                gpu_name = name;
            }
        }
        let physical_device = chosen.ok_or_else(|| VendorError::DeviceUnavailable("无可用物理设备".into()))?;

        // ── queue 家族(graphics|compute 优先,compute-only 兜底) ──
        // 我方上传深度经 vkCmdCopyBufferToImage(VK_IMAGE_ASPECT_DEPTH_BIT)——
        // VUID-vkCmdCopyBufferToImage-commandBuffer-07739 要求 cmd pool 所属
        // 家族带 GRAPHICS;DLSS evaluate 同走该 cmd buffer,compute 位亦需。
        // NVIDIA 家族 0 = GRAPHICS|COMPUTE|TRANSFER 天然满足;compute-only
        // 家族(家族 2,COMPUTE|TRANSFER|SPARSE)命中 07739 实错,仅作理论兜底。
        let mut qcount = 0u32;
        // SAFETY: physical_device 有效。
        unsafe { get_queue_props(physical_device, &mut qcount, std::ptr::null_mut()) };
        // VkQueueFamilyProperties = 24B {flags,count,validBits,granularity×3}
        let mut qprops = vec![0u8; qcount as usize * 24];
        // SAFETY: qprops 容量 = qcount×24。
        unsafe { get_queue_props(physical_device, &mut qcount, qprops.as_mut_ptr() as *mut c_void) };
        let mut family: Option<u32> = None;
        let mut family_fallback: Option<u32> = None;
        for i in 0..qcount {
            let off = i as usize * 24;
            // SAFETY: off+4 ≤ qprops 长度。
            let flags = unsafe { (qprops.as_ptr().add(off) as *const u32).read_unaligned() };
            if flags & (VK_QUEUE_COMPUTE | VK_QUEUE_GRAPHICS) == (VK_QUEUE_COMPUTE | VK_QUEUE_GRAPHICS) {
                family = Some(i);
                break;
            }
            if flags & VK_QUEUE_COMPUTE != 0 && family_fallback.is_none() {
                family_fallback = Some(i);
            }
        }
        let queue_family = family
            .or(family_fallback)
            .ok_or_else(|| VendorError::DeviceUnavailable("无 compute 队列家族".into()))?;

        // ── G14.10b:external memory 扩展探测(在位才启用——不在位维持旧空
        // 扩展表,导入入口 fail-closed;LUID 一并实采供同 adapter 对拍)──
        // SL manual hooking 的 vkCreateDevice 代理把应用扩展表与 SL 自需扩展
        // 合并转发(SL 文档:应用可在 DeviceCreateInfo 里追加自己的扩展)。
        let mut external_memory_enabled = false;
        {
            // SAFETY: instance 有效;符号名 NUL 结尾;cast_sym null 校验。
            let enum_dev_ext: Option<FnVkEnumerateDeviceExtensionProperties> = unsafe {
                cast_sym((fns.vk_gipa)(instance, c"vkEnumerateDeviceExtensionProperties".as_ptr()))
            };
            if let Some(enum_ext) = enum_dev_ext {
                let mut n = 0u32;
                // SAFETY: physical_device 有效;计数出参栈上有效写。
                unsafe { enum_ext(physical_device, std::ptr::null(), &mut n, std::ptr::null_mut()) };
                // VkExtensionProperties = 260B {char[256], u32}。
                let mut blob = vec![0u8; n as usize * 260];
                // SAFETY: blob 容量 = n×260。
                unsafe {
                    enum_ext(
                        physical_device,
                        std::ptr::null(),
                        &mut n,
                        blob.as_mut_ptr() as *mut c_void,
                    )
                };
                let target = c"VK_KHR_external_memory_win32".to_bytes();
                external_memory_enabled = (0..n as usize).any(|i| {
                    let name = &blob[i * 260..i * 260 + 256];
                    let len = name.iter().position(|&b| b == 0).unwrap_or(256);
                    &name[..len] == target
                });
            }
        }
        let device_luid: Option<[u8; 8]> = {
            // SAFETY: instance 有效;符号名 NUL 结尾;cast_sym null 校验(1.1 core,
            // 缺符号 = 异常 loader,None 面下行容忍)。
            let get_props2: Option<FnVkGetPhysicalDeviceProperties2> = unsafe {
                cast_sym((fns.vk_gipa)(instance, c"vkGetPhysicalDeviceProperties2".as_ptr()))
            };
            get_props2.and_then(|get2| {
                let mut id_props = VkPhysicalDeviceIDProperties {
                    s_type: VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_ID_PROPERTIES,
                    p_next: std::ptr::null_mut(),
                    device_uuid: [0; 16],
                    driver_uuid: [0; 16],
                    device_luid: [0; 8],
                    device_node_mask: 0,
                    device_luid_valid: 0,
                };
                let mut props2 = VkPhysicalDeviceProperties2Blob {
                    s_type: VK_STRUCTURE_TYPE_PHYSICAL_DEVICE_PROPERTIES_2,
                    p_next: (&mut id_props as *mut VkPhysicalDeviceIDProperties).cast(),
                    properties: [0; 256],
                };
                // SAFETY: physical_device 有效;props2/id_props 栈上存活,blob 2048B
                // ≥ 真实 properties(约 824B)。
                unsafe { get2(physical_device, &mut props2) };
                (id_props.device_luid_valid != 0).then_some(id_props.device_luid)
            })
        };

        // ── device(SL 代理 vkCreateDevice;SL 自担其扩展/特性/队列;G14.10b
        // 应用侧追加 external memory 两扩展——SL 代理合并转发) ──
        let priority = 1.0f32;
        let qci = VkDeviceQueueCreateInfo {
            s_type: VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            queue_family_index: queue_family,
            queue_count: 1,
            p_queue_priorities: &priority,
        };
        let ext_mem_exts: [*const c_char; 2] = [
            c"VK_KHR_external_memory".as_ptr(),
            c"VK_KHR_external_memory_win32".as_ptr(),
        ];
        let dci = VkDeviceCreateInfo {
            s_type: VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            queue_create_info_count: 1,
            p_queue_create_infos: &qci,
            enabled_layer_count: 0,
            pp_enabled_layer_names: std::ptr::null(),
            enabled_extension_count: if external_memory_enabled { 2 } else { 0 },
            pp_enabled_extension_names: if external_memory_enabled {
                ext_mem_exts.as_ptr()
            } else {
                std::ptr::null()
            },
            p_enabled_features: std::ptr::null(),
        };
        let mut device: VkDevice = std::ptr::null_mut();
        // SAFETY: qci/dci/ext_mem_exts 栈上存活;SL 代理附加 DLSS 必需设备扩展与特性。
        let r = unsafe { create_device(physical_device, &dci, std::ptr::null(), &mut device) };
        if r != VK_SUCCESS || device.is_null() {
            return Err(VendorError::ApiError(format!("vkCreateDevice(proxy) → {r}")));
        }

        macro_rules! dsym {
            ($name:literal, $ty:ty) => {
                // SAFETY: device 有效;代理 gdpa 对非拦截函数转发 vulkan-1.dll。
                match unsafe { cast_sym::<$ty>((fns.vk_gdpa)(device, concat!($name, "\0").as_ptr() as *const c_char)) } {
                    Some(f) => f,
                    None => return Err(VendorError::SymbolMissing($name.into())),
                }
            };
        }
        let dev = VkDevFns {
            destroy_device: dsym!("vkDestroyDevice", FnVkDestroyDevice),
            get_device_queue: dsym!("vkGetDeviceQueue", FnVkGetDeviceQueue),
            create_image: dsym!("vkCreateImage", FnVkCreateImage),
            destroy_image: dsym!("vkDestroyImage", FnVkDestroyImage),
            get_image_memory_requirements: dsym!("vkGetImageMemoryRequirements", FnVkGetImageMemoryRequirements),
            allocate_memory: dsym!("vkAllocateMemory", FnVkAllocateMemory),
            free_memory: dsym!("vkFreeMemory", FnVkFreeMemory),
            bind_image_memory: dsym!("vkBindImageMemory", FnVkBindImageMemory),
            create_image_view: dsym!("vkCreateImageView", FnVkCreateImageView),
            destroy_image_view: dsym!("vkDestroyImageView", FnVkDestroyImageView),
            create_buffer: dsym!("vkCreateBuffer", FnVkCreateBuffer),
            destroy_buffer: dsym!("vkDestroyBuffer", FnVkDestroyBuffer),
            get_buffer_memory_requirements: dsym!("vkGetBufferMemoryRequirements", FnVkGetBufferMemoryRequirements),
            bind_buffer_memory: dsym!("vkBindBufferMemory", FnVkBindBufferMemory),
            map_memory: dsym!("vkMapMemory", FnVkMapMemory),
            unmap_memory: dsym!("vkUnmapMemory", FnVkUnmapMemory),
            create_command_pool: dsym!("vkCreateCommandPool", FnVkCreateCommandPool),
            destroy_command_pool: dsym!("vkDestroyCommandPool", FnVkDestroyCommandPool),
            allocate_command_buffers: dsym!("vkAllocateCommandBuffers", FnVkAllocateCommandBuffers),
            begin_command_buffer: dsym!("vkBeginCommandBuffer", FnVkBeginCommandBuffer),
            end_command_buffer: dsym!("vkEndCommandBuffer", FnVkEndCommandBuffer),
            reset_command_buffer: dsym!("vkResetCommandBuffer", FnVkResetCommandBuffer),
            cmd_pipeline_barrier: dsym!("vkCmdPipelineBarrier", FnVkCmdPipelineBarrier),
            cmd_copy_buffer_to_image: dsym!("vkCmdCopyBufferToImage", FnVkCmdCopyBufferToImage),
            cmd_copy_image_to_buffer: dsym!("vkCmdCopyImageToBuffer", FnVkCmdCopyImageToBuffer),
            queue_submit: dsym!("vkQueueSubmit", FnVkQueueSubmit),
            queue_wait_idle: dsym!("vkQueueWaitIdle", FnVkQueueWaitIdle),
            device_wait_idle: dsym!("vkDeviceWaitIdle", FnVkDeviceWaitIdle),
        };
        let mut queue: VkQueue = std::ptr::null_mut();
        // SAFETY: device/queue_family 有效。
        unsafe { (dev.get_device_queue)(device, queue_family, 0, &mut queue) };

        // ── DLSS 插件装载确认 + 功能函数 ──
        let mut loaded: u8 = 0;
        // SAFETY: loaded 栈上有效写。
        let r = unsafe { (fns.sl_is_feature_loaded)(SL_FEATURE_DLSS, &mut loaded) };
        if r != SL_OK || loaded == 0 {
            return Err(VendorError::DeviceUnavailable(format!(
                "DLSS 插件未装载(slIsFeatureLoaded → {},loaded={loaded})",
                sl_result_name(r)
            )));
        }
        // SAFETY: fns 已解析;p1/p2 出参栈上有效写;cast_sym null 校验;符号名
        // NUL 结尾字面量。
        let dlss = unsafe {
            let mut p1: *mut c_void = std::ptr::null_mut();
            let r1 = (fns.sl_get_feature_function)(SL_FEATURE_DLSS, c"slDLSSSetOptions".as_ptr(), &mut p1);
            let mut p2: *mut c_void = std::ptr::null_mut();
            let r2 = (fns.sl_get_feature_function)(SL_FEATURE_DLSS, c"slDLSSGetOptimalSettings".as_ptr(), &mut p2);
            if r1 != SL_OK || r2 != SL_OK || p1.is_null() || p2.is_null() {
                return Err(VendorError::SymbolMissing(format!(
                    "slGetFeatureFunction(DLSS) → {}/{r2}",
                    sl_result_name(r1)
                )));
            }
            DlssVkFns2 {
                dlss_set_options: cast_sym::<FnSlDlssSetOptions>(p1)
                    .ok_or_else(|| VendorError::SymbolMissing("slDLSSSetOptions fn".into()))?,
                dlss_get_optimal: cast_sym::<FnSlDlssGetOptimalSettings>(p2)
                    .ok_or_else(|| VendorError::SymbolMissing("slDLSSGetOptimalSettings fn".into()))?,
            }
        };
        let mut fv = SlFeatureVersion {
            base: sl_base(SL_GUID_FEATURE_VERSION, 1),
            version_sl: [0; 3],
            version_ngx: [0; 3],
        };
        // SAFETY: fv 栈上有效写。
        let _ = unsafe { (fns.sl_get_feature_version)(SL_FEATURE_DLSS, &mut fv) };
        let ngx_version = format!(
            "SL {}.{}.{} / NGX {}.{}.{}",
            fv.version_sl[0], fv.version_sl[1], fv.version_sl[2],
            fv.version_ngx[0], fv.version_ngx[1], fv.version_ngx[2]
        );

        // ── DLSS optimal settings 核验(输入分辨率须在 [min,max]) ──
        let opts = SlDlssOptions {
            base: sl_base(SL_GUID_DLSS_OPTIONS, 3),
            mode: SL_DLSS_MODE_MAX_PERFORMANCE,
            output_width: out_size.0,
            output_height: out_size.1,
            sharpness: 0.0,
            pre_exposure: 1.0,
            exposure_scale: 1.0,
            color_buffers_hdr: 1,
            indicator_invert_axis_x: 0,
            indicator_invert_axis_y: 0,
            _pad0: 0,
            dlaa_preset: 0,
            quality_preset: 0,
            balanced_preset: 0,
            performance_preset: 0,
            ultra_performance_preset: 0,
            ultra_quality_preset: 0,
            use_auto_exposure: 0,
            alpha_upscaling_enabled: 0,
            _pad1: [0; 2],
        };
        let mut optimal = SlDlssOptimalSettings {
            base: sl_base(SL_GUID_DLSS_OPTIMAL, 1),
            optimal_render_width: 0,
            optimal_render_height: 0,
            optimal_sharpness: 0.0,
            render_width_min: 0,
            render_height_min: 0,
            render_width_max: 0,
            render_height_max: 0,
        };
        // SAFETY: opts/optimal 栈上有效。
        let r = unsafe { (dlss.dlss_get_optimal)(&opts, &mut optimal) };
        if r != SL_OK {
            return Err(VendorError::VendorCall(format!(
                "slDLSSGetOptimalSettings → {}",
                sl_result_name(r)
            )));
        }
        let (iw, ih) = in_size;
        if iw < optimal.render_width_min
            || ih < optimal.render_height_min
            || iw > optimal.render_width_max
            || ih > optimal.render_height_max
        {
            return Err(VendorError::DeviceUnavailable(format!(
                "输入 {iw}x{ih} 越出 DLSS 窗口 [{},{}]x[{},{}]",
                optimal.render_width_min,
                optimal.render_width_max,
                optimal.render_height_min,
                optimal.render_height_max
            )));
        }

        // ── 内存类型索引 ──
        // VkPhysicalDeviceMemoryProperties 520B(32 types × 8B + 16 heaps × 16B + 2×u32)。
        let mut mem_blob = [0u64; 66]; // 528B align(8) 超集
        // SAFETY: blob ≥ 520B 真实结构。
        unsafe { get_mem_props(physical_device, mem_blob.as_mut_ptr() as *mut c_void) };
        let mem_bytes = mem_blob.as_ptr() as *const u8;
        // SAFETY: 只读 520B 前缀;memoryTypeCount@0,types@4..260,heapCount@260,heaps@264。
        let (type_count, heap_count) = unsafe {
            (
                (mem_bytes as *const u32).read_unaligned() as usize,
                (mem_bytes.add(260) as *const u32).read_unaligned() as usize,
            )
        };
        let mut heaps = Vec::with_capacity(heap_count.min(16));
        for i in 0..heap_count.min(16) {
            // SAFETY: 264+i*16 在 520B 内。
            let size = unsafe { (mem_bytes.add(264 + i * 16) as *const u64).read_unaligned() };
            heaps.push(size);
        }
        let pick_type = |required: u32, prefer_device: bool| -> Option<u32> {
            let mut best: Option<(u32, u64)> = None;
            for i in 0..type_count.min(32) {
                // SAFETY: 4+i*8 在 260B 内。
                let (flags, heap_idx) = unsafe {
                    (
                        (mem_bytes.add(4 + i * 8) as *const u32).read_unaligned(),
                        (mem_bytes.add(8 + i * 8) as *const u32).read_unaligned() as usize,
                    )
                };
                if flags & required == required {
                    if !prefer_device {
                        return Some(i as u32);
                    }
                    let heap_size = heaps.get(heap_idx).copied().unwrap_or(0);
                    if best.is_none_or(|(_, s)| heap_size > s) {
                        best = Some((i as u32, heap_size));
                    }
                }
            }
            best.map(|(i, _)| i)
        };
        let device_local_type = pick_type(VK_MEMORY_PROPERTY_DEVICE_LOCAL, true)
            .ok_or_else(|| VendorError::DeviceUnavailable("无 device-local 内存类型".into()))?;
        let host_type = pick_type(VK_MEMORY_PROPERTY_HOST_VISIBLE | VK_MEMORY_PROPERTY_HOST_COHERENT, false)
            .ok_or_else(|| VendorError::DeviceUnavailable("无 host-visible+coherent 内存类型".into()))?;
        // G14.3 性能波:readback 专用 host-cached 型(原 readback 落首个
        // host-visible+coherent 型——NVIDIA 该型为 uncached/WC,逐元素 host 读
        // = PCIe 往返延迟,实测 readback 分项 ~325ms@1080p 输出/41ms@512²;
        // HOST_CACHED 块读/逐元素读均为缓存命中口径,内容面与 WC 逐位一致——
        // coherent 免 invalidate 语义不变,digest 不变机核;缺该型回退既有
        // host_type,行为零漂移)。
        let host_cached_type = pick_type(
            VK_MEMORY_PROPERTY_HOST_VISIBLE
                | VK_MEMORY_PROPERTY_HOST_COHERENT
                | VK_MEMORY_PROPERTY_HOST_CACHED,
            false,
        )
        .unwrap_or(host_type);

        // ── 资源面 ──
        let (ow, oh) = out_size;
        let dev_ref = &dev;
        let mk_image = |format: u32,
                        w: u32,
                        h: u32,
                        usage: u32,
                        aspect: u32|
         -> Result<VkImageRes, VendorError> {
            let ici = VkImageCreateInfo {
                s_type: VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                // G14.12 勘误:`VK_IMAGE_TYPE_2D` = **1**(2 是 3D)。本处原为
                // `2` 且注释写 "2D"——与 `import_win32_input` 同源笔误(该处
                // 曾致跨 device 共享块状乱序、被误判为硬件布局不一致)。本处
                // 为 session 自有输入/输出 image(同 device 写读,内容自洽故
                // 无可见损坏),但 3D image 上建 2D view 触
                // VUID-VkImageViewCreateInfo-image-06728(validation 10 条)。
                image_type: VK_IMAGE_TYPE_2D,
                format,
                extent: VkExtent3D { width: w, height: h, depth: 1 },
                mip_levels: 1,
                array_layers: 1,
                samples: 1,
                tiling: 0, // OPTIMAL
                usage,
                sharing_mode: 0,
                queue_family_index_count: 0,
                p_queue_family_indices: std::ptr::null(),
                initial_layout: VK_IMAGE_LAYOUT_UNDEFINED,
            };
            let mut image: VkImage = 0;
            // SAFETY: ici 栈上存活。
            let r = unsafe { (dev_ref.create_image)(device, &ici, std::ptr::null(), &mut image) };
            if r != VK_SUCCESS || image == 0 {
                return Err(VendorError::ApiError(format!("vkCreateImage(fmt={format}) → {r}")));
            }
            let mut req = VkMemoryRequirements::default();
            // SAFETY: image 有效。
            unsafe { (dev_ref.get_image_memory_requirements)(device, image, &mut req) };
            let mai = VkMemoryAllocateInfo {
                s_type: VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
                p_next: std::ptr::null(),
                allocation_size: req.size,
                memory_type_index: device_local_type,
            };
            let mut memory: VkDeviceMemory = 0;
            // SAFETY: mai 栈上存活。
            let r = unsafe { (dev_ref.allocate_memory)(device, &mai, std::ptr::null(), &mut memory) };
            if r != VK_SUCCESS {
                return Err(VendorError::ApiError(format!("vkAllocateMemory(image) → {r}")));
            }
            // SAFETY: image/memory 配对有效。
            let r = unsafe { (dev_ref.bind_image_memory)(device, image, memory, 0) };
            if r != VK_SUCCESS {
                return Err(VendorError::ApiError(format!("vkBindImageMemory → {r}")));
            }
            let vci = VkImageViewCreateInfo {
                s_type: VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                image,
                view_type: 1, // 2D
                format,
                components: [0; 4],
                subresource_range: VkImageSubresourceRange {
                    aspect_mask: aspect,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
            };
            let mut view: VkImageView = 0;
            // SAFETY: vci 栈上存活。
            let r = unsafe { (dev_ref.create_image_view)(device, &vci, std::ptr::null(), &mut view) };
            if r != VK_SUCCESS || view == 0 {
                return Err(VendorError::ApiError(format!("vkCreateImageView(fmt={format}) → {r}")));
            }
            Ok(VkImageRes { image, memory, view, layout: VK_IMAGE_LAYOUT_UNDEFINED, format, w, h, aspect })
        };
        let color_in = mk_image(
            VK_FORMAT_R16G16B16A16_SFLOAT,
            iw,
            ih,
            VK_IMAGE_USAGE_SAMPLED | VK_IMAGE_USAGE_TRANSFER_DST,
            VK_IMAGE_ASPECT_COLOR,
        )?;
        let depth_in = mk_image(
            VK_FORMAT_D32_SFLOAT,
            iw,
            ih,
            VK_IMAGE_USAGE_SAMPLED | VK_IMAGE_USAGE_TRANSFER_DST,
            VK_IMAGE_ASPECT_DEPTH,
        )?;
        let mv_in = mk_image(
            VK_FORMAT_R32G32_SFLOAT,
            iw,
            ih,
            VK_IMAGE_USAGE_SAMPLED | VK_IMAGE_USAGE_TRANSFER_DST,
            VK_IMAGE_ASPECT_COLOR,
        )?;
        let reactive_in = mk_image(
            VK_FORMAT_R8_UNORM,
            iw,
            ih,
            VK_IMAGE_USAGE_SAMPLED | VK_IMAGE_USAGE_TRANSFER_DST,
            VK_IMAGE_ASPECT_COLOR,
        )?;
        let color_out = mk_image(
            VK_FORMAT_R16G16B16A16_SFLOAT,
            ow,
            oh,
            VK_IMAGE_USAGE_STORAGE | VK_IMAGE_USAGE_TRANSFER_SRC,
            VK_IMAGE_ASPECT_COLOR,
        )?;

        // staging(上传)+ readback(回读)host 缓冲。
        let in_bytes = [
            (iw * ih * 4 * 2) as u64, // color RGBA f16
            (iw * ih * 4) as u64,     // depth f32
            (iw * ih * 2 * 4) as u64, // mv RG f32
            (iw * ih) as u64,         // reactive R8
        ];
        let staging_size = in_bytes.iter().sum::<u64>();
        let readback_size = (ow * oh * 4 * 2) as u64;
        let mk_buffer = |size: u64, usage: u32, mem_type: u32| -> Result<(VkBuffer, VkDeviceMemory), VendorError> {
            let bci = VkBufferCreateInfo {
                s_type: VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: 0,
                size,
                usage,
                sharing_mode: 0,
                queue_family_index_count: 0,
                p_queue_family_indices: std::ptr::null(),
            };
            let mut buffer: VkBuffer = 0;
            // SAFETY: bci 栈上存活。
            let r = unsafe { (dev.create_buffer)(device, &bci, std::ptr::null(), &mut buffer) };
            if r != VK_SUCCESS || buffer == 0 {
                return Err(VendorError::ApiError(format!("vkCreateBuffer → {r}")));
            }
            let mut req = VkMemoryRequirements::default();
            // SAFETY: buffer 有效。
            unsafe { (dev.get_buffer_memory_requirements)(device, buffer, &mut req) };
            let mai = VkMemoryAllocateInfo {
                s_type: VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
                p_next: std::ptr::null(),
                allocation_size: req.size,
                memory_type_index: mem_type,
            };
            let mut memory: VkDeviceMemory = 0;
            // SAFETY: mai 栈上存活。
            let r = unsafe { (dev.allocate_memory)(device, &mai, std::ptr::null(), &mut memory) };
            if r != VK_SUCCESS {
                return Err(VendorError::ApiError(format!("vkAllocateMemory(buffer) → {r}")));
            }
            // SAFETY: buffer/memory 配对有效。
            let r = unsafe { (dev.bind_buffer_memory)(device, buffer, memory, 0) };
            if r != VK_SUCCESS {
                return Err(VendorError::ApiError(format!("vkBindBufferMemory → {r}")));
            }
            Ok((buffer, memory))
        };
        let (staging, staging_mem) = mk_buffer(staging_size, VK_BUFFER_USAGE_TRANSFER_SRC, host_type)?;
        let (readback, readback_mem) =
            mk_buffer(readback_size, VK_BUFFER_USAGE_TRANSFER_DST, host_cached_type)?;

        let cpci = VkCommandPoolCreateInfo {
            s_type: VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER,
            queue_family_index: queue_family,
        };
        let mut cmd_pool: VkCommandPool = 0;
        // SAFETY: cpci 栈上存活。
        let r = unsafe { (dev.create_command_pool)(device, &cpci, std::ptr::null(), &mut cmd_pool) };
        if r != VK_SUCCESS || cmd_pool == 0 {
            return Err(VendorError::ApiError(format!("vkCreateCommandPool → {r}")));
        }
        let cbai = VkCommandBufferAllocateInfo {
            s_type: VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            command_pool: cmd_pool,
            level: 0, // PRIMARY
            command_buffer_count: 1,
        };
        let mut cmd: VkCommandBuffer = std::ptr::null_mut();
        // SAFETY: cbai 栈上存活。
        let r = unsafe { (dev.allocate_command_buffers)(device, &cbai, &mut cmd) };
        if r != VK_SUCCESS || cmd.is_null() {
            return Err(VendorError::ApiError(format!("vkAllocateCommandBuffers → {r}")));
        }

        Ok(DlssVkSession {
            fns,
            dlss,
            dev,
            instance,
            device,
            queue,
            cmd_pool,
            cmd,
            viewport: SlViewportHandle { base: sl_base(SL_GUID_VIEWPORT, 1), value: 0 },
            in_w: iw,
            in_h: ih,
            out_w: ow,
            out_h: oh,
            color_in,
            depth_in,
            mv_in,
            reactive_in,
            color_out,
            staging,
            staging_mem,
            staging_size,
            readback,
            readback_mem,
            readback_size,
            messenger,
            validation_counter,
            gpu_name,
            ngx_version,
            dlls,
            log_tail: Vec::new(),
            shutdown_done: false,
            reactive_zero_resident: false,
            external_memory_enabled,
            device_luid,
            ext_inputs: [None, None, None],
            ext_input_bufs: [None, None, None],
            queue_family,
        })
    }

    #[allow(clippy::too_many_arguments)] // Vulkan barrier 八元参数面与 VkImageMemoryBarrier 字段一一对应,封装为元组反损可读性
    fn vk_barrier(&self, res: &VkImageRes, old: i32, new: i32, src_access: u32, dst_access: u32, src_stage: u32, dst_stage: u32) {
        let barrier = VkImageMemoryBarrier {
            s_type: VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
            p_next: std::ptr::null(),
            src_access_mask: src_access,
            dst_access_mask: dst_access,
            old_layout: old,
            new_layout: new,
            src_queue_family_index: !0,
            dst_queue_family_index: !0,
            image: res.image,
            subresource_range: VkImageSubresourceRange {
                aspect_mask: res.aspect,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
        };
        // SAFETY: cmd 录制中;barrier 栈上存活至调用返回;image 有效。
        unsafe {
            (self.dev.cmd_pipeline_barrier)(
                self.cmd,
                src_stage,
                dst_stage,
                0,
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                1,
                &barrier,
            )
        };
    }

    fn vk_upload_image(&mut self, slot: VkInputSlot, staging_offset: u64) {
        self.vk_upload_image_src(slot, self.staging, staging_offset);
    }

    /// [`Self::vk_upload_image`] 的 copy 源参数化本体(G14.10f:导入 buffer
    /// 直接作 copy 源——同一 barrier/region/layout 状态机,仅源 buffer 异)。
    fn vk_upload_image_src(&mut self, slot: VkInputSlot, src: VkBuffer, src_offset: u64) {
        // src_offset 段数据契约:staging 路 = 调用前已写 map;导入 buffer 路 =
        // 导出侧该帧 fence 已完成 + EXTERNAL release 已录(acquire 由调用方先录)。
        let (image, aspect, w, h, old) = match slot {
            VkInputSlot::Color => (self.color_in.image, self.color_in.aspect, self.color_in.w, self.color_in.h, self.color_in.layout),
            VkInputSlot::Depth => (self.depth_in.image, self.depth_in.aspect, self.depth_in.w, self.depth_in.h, self.depth_in.layout),
            VkInputSlot::Mv => (self.mv_in.image, self.mv_in.aspect, self.mv_in.w, self.mv_in.h, self.mv_in.layout),
            VkInputSlot::Reactive => (self.reactive_in.image, self.reactive_in.aspect, self.reactive_in.w, self.reactive_in.h, self.reactive_in.layout),
        };
        let first = old == VK_IMAGE_LAYOUT_UNDEFINED;
        let src_access = if first { 0 } else { VK_ACCESS_SHADER_READ };
        let src_stage = if first { VK_PIPELINE_STAGE_TOP_OF_PIPE } else { VK_PIPELINE_STAGE_COMPUTE_SHADER };
        let res = VkImageRes { image, memory: 0, view: 0, layout: old, format: 0, w, h, aspect };
        self.vk_barrier(&res, old, VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL, src_access, VK_ACCESS_TRANSFER_WRITE, src_stage, VK_PIPELINE_STAGE_TRANSFER);
        let region = VkBufferImageCopy {
            buffer_offset: src_offset,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: VkImageSubresourceLayers {
                aspect_mask: aspect,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_offset: [0, 0, 0],
            image_extent: VkExtent3D { width: w, height: h, depth: 1 },
        };
        // SAFETY: cmd 录制中;src 数据契约见方法注释;region 与图像尺寸一致。
        unsafe {
            (self.dev.cmd_copy_buffer_to_image)(
                self.cmd,
                src,
                image,
                VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                1,
                &region,
            )
        };
        let res2 = VkImageRes { layout: VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL, ..res };
        self.vk_barrier(&res2, VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL, VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL, VK_ACCESS_TRANSFER_WRITE, VK_ACCESS_SHADER_READ, VK_PIPELINE_STAGE_TRANSFER, VK_PIPELINE_STAGE_COMPUTE_SHADER);
        let new_layout = VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL;
        match slot {
            VkInputSlot::Color => self.color_in.layout = new_layout,
            VkInputSlot::Depth => self.depth_in.layout = new_layout,
            VkInputSlot::Mv => self.mv_in.layout = new_layout,
            VkInputSlot::Reactive => self.reactive_in.layout = new_layout,
        }
    }

    /// 执行一帧 DLSS 超分;返回 `out_size` 的 3 通道 f32 显示域图像(行主序 RGB)。
    pub fn upscale(&mut self, input: &VendorFrameInput) -> Result<Vec<f32>, VendorError> {
        self.frame_impl(input, false)
    }

    /// validation 探针帧:跑我方 Vulkan 全表面(staging 打包/四槽上传含 DEPTH
    /// aspect 拷贝/barrier/提交/回读/reset)+ SL 簿记调用(frame token/constants/
    /// options),**跳过 slEvaluateFeature**——NGX evaluate 在 KHRONOS_validation
    /// 层在下崩溃于 NVIDIA 驱动内部(nvoglv64.dll,0xc0000005;SL 异常处理器捕获
    /// 报 eErrorExceptionHandler;vendor 已知 SL+validation 不兼容类,Streamline
    /// issue #84 ack/bug),校验层无法覆盖其内部 CUDA interop 段。探针帧输出内容
    /// 未定(DLSS 未写 color_out),仅供校验层覆盖,禁作画质消费。
    pub fn probe_validation_frame(&mut self, input: &VendorFrameInput) -> Result<Vec<f32>, VendorError> {
        self.frame_impl(input, true)
    }

    fn frame_impl(&mut self, input: &VendorFrameInput, skip_evaluate: bool) -> Result<Vec<f32>, VendorError> {
        let mut out = vec![0f32; (self.out_w * self.out_h * 3) as usize];
        self.frame_impl_into(input, skip_evaluate, &mut out)?;
        Ok(out)
    }

    /// G14.6 Stage A：驻留输出变体（upscale 字节面与 `frame_impl` 逐位一致——
    /// 同一 frame_impl_into 主体，调用方驻留 Vec 消逐帧 ~out_px·12B 分配+清零；
    /// dst 长度不符时由本层 resize（首帧一次），其后逐帧零分配）。
    pub fn upscale_into(&mut self, input: &VendorFrameInput, dst: &mut Vec<f32>) -> Result<(), VendorError> {
        let need = (self.out_w * self.out_h * 3) as usize;
        if dst.len() != need {
            dst.resize(need, 0.0);
        }
        self.frame_impl_into(input, false, dst)
    }

    fn frame_impl_into(&mut self, input: &VendorFrameInput, skip_evaluate: bool, out: &mut [f32]) -> Result<(), VendorError> {
        self.frame_impl_ext(input, skip_evaluate, Some(out))
    }

    /// G14plus vendor 域(RFC-0030)帧主体参数化:`out=Some` = 既有回读路径
    /// (`frame_impl_into` 全量委托本函数——调用序/上传字节面/输出字节面与
    /// G14.7 面逐位一致);`out=None` = **驻留输出路径**(跳过输出回读录制段与
    /// host f16→f32 转换——输出驻留 session 自建 `color_out` image(GENERAL
    /// layout),按需经 [`Self::readback_output_into`] 回读)。pack/upload/
    /// evaluate/submit_wait 两路径同一代码面。
    fn frame_impl_ext(&mut self, input: &VendorFrameInput, skip_evaluate: bool, mut out: Option<&mut [f32]>) -> Result<(), VendorError> {
        let (iw, ih) = (self.in_w, self.in_h);
        let px = (iw * ih) as usize;
        if input.color.len() != px * 3 || input.depth.len() != px || input.mv.len() != px * 2 {
            return Err(VendorError::ApiError("输入切片长度与 session 分辨率不符".into()));
        }
        if let Some(r) = input.reactive
            && r.len() != px
        {
            return Err(VendorError::ApiError("reactive 切片长度不符".into()));
        }
        if let Some(o) = out.as_deref()
            && o.len() != (self.out_w * self.out_h * 3) as usize
        {
            return Err(VendorError::ApiError("输出切片长度与 session 输出分辨率不符".into()));
        }
        // G14.3 性能波:内部分解遥测(env `RURIX_VENDOR_TIMING=1` 门控,默认关,
        // 零行为变更;轴 = pack 打包 / sl_book 簿记 / upload 上传录制 / evaluate
        // vendor 调用 / submit_wait GPU 执行+同步 / readback 回读转换)。
        let vtm_on = std::env::var("RURIX_VENDOR_TIMING").ok().as_deref() == Some("1");
        let vtm_t0 = std::time::Instant::now();
        // ── staging 打包(color f16 + depth f32 + mv f32 + reactive R8) ──
        // G14.3 性能波:session 常驻 pack_buf 复用(消逐帧 ~px·21B 新分配+清零)
        // + chunks_exact 定长块写(消逐元素边界检查);上传字节面与逐帧新分配
        // 逐位一致(reactive None 臂 fill(0) ≡ 新零 vec 字面)。
        // G14.6 Stage A:pack 直写 mapped staging(消 pack_buf→staging ~px·21B
        // 二次 memcpy;staging 最终字节面与 G14.3 面逐位一致——同序同式写入,
        // 仅落点由中转 vec 改为映射指针)。
        let color_bytes = px * 4 * 2;
        let depth_bytes = px * 4;
        let mv_bytes = px * 2 * 4;
        let reactive_bytes = px;
        let pack_total = color_bytes + depth_bytes + mv_bytes + reactive_bytes;
        if (pack_total as u64) > self.staging_size {
            return Err(VendorError::ApiError("pack 总长超 staging 容量".into()));
        }
        let d_off = color_bytes;
        let m_off = d_off + depth_bytes;
        let r_off = m_off + mv_bytes;
        // SAFETY: staging host-visible+coherent;map 尺寸 = staging_size ≥ pack_total;
        // 切片不越 map 区间;G14.7 像素带并行——四区经 split_at_mut 切分为互不重叠
        // 区段,pack_vendor_inputs 内 chunks_mut 再切带,scoped threads 汇前 join
        // (unmap 在 join 后),无别名/无悬垂访问;带内转换与串行同式同序,最终字节
        // 面与 G14.6 面逐位一致。
        unsafe {
            let mut ptr: *mut c_void = std::ptr::null_mut();
            let r = (self.dev.map_memory)(self.device, self.staging_mem, 0, self.staging_size, 0, &mut ptr);
            if r != VK_SUCCESS || ptr.is_null() {
                return Err(VendorError::ApiError(format!("vkMapMemory(staging) → {r}")));
            }
            let packed = std::slice::from_raw_parts_mut(ptr as *mut u8, pack_total);
            let (color_r, rest) = packed.split_at_mut(color_bytes);
            let (depth_r, rest) = rest.split_at_mut(depth_bytes);
            let (mv_r, reac_r) = rest.split_at_mut(mv_bytes);
            pack_vendor_inputs(
                px,
                input.color,
                input.depth,
                input.mv,
                input.reactive,
                color_r,
                depth_r,
                mv_r,
                reac_r,
            );
            (self.dev.unmap_memory)(self.device, self.staging_mem);
        }
        let vtm_pack = vtm_t0.elapsed();

        // ── 每帧 SL 调用序:frame token → constants → options → 录制 evaluate ──
        let mut token: *mut c_void = std::ptr::null_mut();
        // SAFETY: token 出参栈上有效。
        let r = unsafe { (self.fns.sl_get_new_frame_token)(&mut token, &input.frame_index) };
        if r != SL_OK || token.is_null() {
            return Err(VendorError::VendorCall(format!("slGetNewFrameToken → {}", sl_result_name(r))));
        }
        let consts = build_sl_constants(input, iw, ih);
        // SAFETY: consts 栈上存活;token/viewport 有效。
        let r = unsafe { (self.fns.sl_set_constants)(&consts, token, &self.viewport) };
        if r != SL_OK {
            return Err(VendorError::VendorCall(format!("slSetConstants → {}", sl_result_name(r))));
        }
        let opts = SlDlssOptions {
            base: sl_base(SL_GUID_DLSS_OPTIONS, 3),
            mode: SL_DLSS_MODE_MAX_PERFORMANCE,
            output_width: self.out_w,
            output_height: self.out_h,
            sharpness: 0.0,
            pre_exposure: input.exposure,
            exposure_scale: 1.0,
            color_buffers_hdr: 1,
            indicator_invert_axis_x: 0,
            indicator_invert_axis_y: 0,
            _pad0: 0,
            dlaa_preset: 0,
            quality_preset: 0,
            balanced_preset: 0,
            performance_preset: 0,
            ultra_performance_preset: 0,
            ultra_quality_preset: 0,
            use_auto_exposure: 0,
            alpha_upscaling_enabled: 0,
            _pad1: [0; 2],
        };
        // SAFETY: opts/viewport 栈上存活。
        let r = unsafe { (self.dlss.dlss_set_options)(&self.viewport, &opts) };
        if r != SL_OK {
            return Err(VendorError::VendorCall(format!("slDLSSSetOptions → {}", sl_result_name(r))));
        }
        let vtm_book = vtm_t0.elapsed();

        let begin = VkCommandBufferBeginInfo {
            s_type: VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
            p_next: std::ptr::null(),
            flags: 0x1, // ONE_TIME_SUBMIT
            p_inheritance_info: std::ptr::null(),
        };
        // SAFETY: cmd 已分配且不在未决状态(上一帧 queueWaitIdle 排空)。
        let r = unsafe { (self.dev.begin_command_buffer)(self.cmd, &begin) };
        if r != VK_SUCCESS {
            return Err(VendorError::ApiError(format!("vkBeginCommandBuffer → {r}")));
        }

        // 上传四输入(color/depth/mv/reactive);packed 各段已写入 staging map。
        self.vk_upload_image(VkInputSlot::Color, 0);
        self.vk_upload_image(VkInputSlot::Depth, d_off as u64);
        self.vk_upload_image(VkInputSlot::Mv, m_off as u64);
        self.vk_upload_image(VkInputSlot::Reactive, r_off as u64);
        // color_out 置 GENERAL(DLSS UAV 写)。
        if self.color_out.layout == VK_IMAGE_LAYOUT_UNDEFINED {
            let out_res = self.color_out.clone_shallow();
            self.vk_barrier(&out_res, VK_IMAGE_LAYOUT_UNDEFINED, VK_IMAGE_LAYOUT_GENERAL, 0, VK_ACCESS_SHADER_WRITE, VK_PIPELINE_STAGE_TOP_OF_PIPE, VK_PIPELINE_STAGE_COMPUTE_SHADER);
            self.color_out.layout = VK_IMAGE_LAYOUT_GENERAL;
        }
        let vtm_upload = vtm_t0.elapsed();

        // ── ResourceTag 集(evaluate 直接消费;Vulkan 需完整资源描述) ──
        let mk_sl_res = |res: &VkImageRes| -> SlResource {
            SlResource {
                base: sl_base(SL_GUID_RESOURCE, 1),
                res_type: SL_RESOURCE_TEX2D,
                _pad0: [0; 7],
                native: res.image as usize as *mut c_void,
                memory: res.memory as usize as *mut c_void,
                view: res.view as usize as *mut c_void,
                state: res.layout as u32,
                width: res.w,
                height: res.h,
                native_format: res.format,
                mip_levels: 1,
                array_layers: 1,
                gpu_virtual_address: 0,
                flags: 0,
                usage: if res.image == self.color_out.image {
                    VK_IMAGE_USAGE_STORAGE | VK_IMAGE_USAGE_TRANSFER_SRC
                } else {
                    VK_IMAGE_USAGE_SAMPLED | VK_IMAGE_USAGE_TRANSFER_DST
                },
                reserved: 0,
            }
        };
        let sl_color_in = mk_sl_res(&self.color_in);
        let sl_depth = mk_sl_res(&self.depth_in);
        let sl_mv = mk_sl_res(&self.mv_in);
        let sl_reactive = mk_sl_res(&self.reactive_in);
        let sl_color_out = mk_sl_res(&self.color_out);
        let extent_in = SlExtent { top: 0, left: 0, width: iw, height: ih };
        let extent_out = SlExtent { top: 0, left: 0, width: self.out_w, height: self.out_h };
        let tags = [
            SlResourceTag { base: sl_base(SL_GUID_RESOURCE_TAG, 1), resource: &sl_color_in, tag_type: SL_BUFFER_SCALING_INPUT_COLOR, lifecycle: SL_LIFECYCLE_ONLY_VALID_NOW, extent: extent_in },
            SlResourceTag { base: sl_base(SL_GUID_RESOURCE_TAG, 1), resource: &sl_color_out, tag_type: SL_BUFFER_SCALING_OUTPUT_COLOR, lifecycle: SL_LIFECYCLE_ONLY_VALID_NOW, extent: extent_out },
            SlResourceTag { base: sl_base(SL_GUID_RESOURCE_TAG, 1), resource: &sl_depth, tag_type: SL_BUFFER_DEPTH, lifecycle: SL_LIFECYCLE_ONLY_VALID_NOW, extent: extent_in },
            SlResourceTag { base: sl_base(SL_GUID_RESOURCE_TAG, 1), resource: &sl_mv, tag_type: SL_BUFFER_MV, lifecycle: SL_LIFECYCLE_ONLY_VALID_NOW, extent: extent_in },
            SlResourceTag { base: sl_base(SL_GUID_RESOURCE_TAG, 1), resource: &sl_reactive, tag_type: SL_BUFFER_REACTIVE_MASK, lifecycle: SL_LIFECYCLE_ONLY_VALID_NOW, extent: extent_in },
        ];
        // viewport handle 必须链入 evaluate inputs(SL 实测报错字面:「Missing
        // viewport handle, did you forget to chain it up in the slEvaluateFeature
        // inputs?」——ViewportHandle 是 BaseStructure 多态链成员,与 ResourceTag 同槽)。
        let tag_ptrs: [*const SlBaseStructure; 6] = [
            &self.viewport.base as *const _,
            &tags[0].base as *const _,
            &tags[1].base as *const _,
            &tags[2].base as *const _,
            &tags[3].base as *const _,
            &tags[4].base as *const _,
        ];
        // 探针帧(skip_evaluate)跳过本调用:NGX evaluate 在 KHRONOS_validation
        // 层在下触发驱动内崩溃(vendor 已知不兼容),校验覆盖止步于此——探针帧
        // 已完成 token/constants/options 簿记与资源 tag 组装面,type/布局检查仍
        // 经编译期锚定覆盖。
        if !skip_evaluate {
            // SAFETY: tags/sl_* 资源描述栈上存活至 evaluate 返回(eOnlyValidNow
            // 语义 = evaluate 期间有效);cmd 录制中;token 为本帧有效 token。
            let r = unsafe {
                (self.fns.sl_evaluate_feature)(
                    SL_FEATURE_DLSS,
                    token,
                    tag_ptrs.as_ptr(),
                    6,
                    self.cmd,
                )
            };
            if r != SL_OK {
                return Err(VendorError::VendorCall(format!("slEvaluateFeature(DLSS) → {}", sl_result_name(r))));
            }
        }
        let vtm_eval = vtm_t0.elapsed();

        // 输出回读:GENERAL → TRANSFER_SRC → copy → 回 GENERAL(驻留输出路径
        // (out=None)跳过——color_out 留 GENERAL,内容驻留待按需回读)。
        if out.is_some() {
            self.record_output_readback();
        }
        // HOST_READ 可见性屏障由 queueWaitIdle 全序保证(U26 同律)。
        // SAFETY: cmd 录制完成。
        let r = unsafe { (self.dev.end_command_buffer)(self.cmd) };
        if r != VK_SUCCESS {
            return Err(VendorError::ApiError(format!("vkEndCommandBuffer → {r}")));
        }
        let submit = VkSubmitInfo {
            s_type: VK_STRUCTURE_TYPE_SUBMIT_INFO,
            p_next: std::ptr::null(),
            wait_semaphore_count: 0,
            p_wait_semaphores: std::ptr::null(),
            p_wait_dst_stage_mask: std::ptr::null(),
            command_buffer_count: 1,
            p_command_buffers: &self.cmd,
            signal_semaphore_count: 0,
            p_signal_semaphores: std::ptr::null(),
        };
        // SAFETY: submit 栈上存活;cmd 为本帧录制完成态。
        let r = unsafe { (self.dev.queue_submit)(self.queue, 1, &submit, 0) };
        if r != VK_SUCCESS {
            return Err(VendorError::ApiError(format!("vkQueueSubmit → {r}")));
        }
        // SAFETY: queue 有效;waitIdle 后无在途写。
        let r = unsafe { (self.dev.queue_wait_idle)(self.queue) };
        if r != VK_SUCCESS {
            return Err(VendorError::ApiError(format!("vkQueueWaitIdle → {r}")));
        }
        let vtm_wait = vtm_t0.elapsed();

        // G14.6 Stage A:输出直写调用方驻留切片(消逐帧 ~out_px·12B 新分配+清零;
        // 转换逐值同式同序,字节面与 G14.3 面逐位一致)。驻留输出路径(out=None)
        // 跳过——无回读内容可转换。
        if let Some(o) = out.take() {
            self.map_convert_readback(o)?;
        }
        // 录制槽复用:reset cmd(下一帧重录)。
        // SAFETY: cmd 已完成提交且 queue 排空。
        let _ = unsafe { (self.dev.reset_command_buffer)(self.cmd, 0) };
        if vtm_on {
            let total = vtm_t0.elapsed().as_secs_f64() * 1e3;
            let ms = |d: std::time::Duration| d.as_secs_f64() * 1e3;
            eprintln!(
                "[vendor-timing dlss] frame={} pack={:.3} sl_book={:.3} upload={:.3} evaluate={:.3} submit_wait={:.3} readback={:.3} total={:.3}ms",
                input.frame_index,
                ms(vtm_pack),
                ms(vtm_book - vtm_pack),
                ms(vtm_upload - vtm_book),
                ms(vtm_eval - vtm_upload),
                ms(vtm_wait - vtm_eval),
                ms(vtm_t0.elapsed() - vtm_wait),
                total,
            );
        }
        Ok(())
    }

    /// 录制输出回读段(GENERAL → TRANSFER_SRC → copy → 回 GENERAL;调用方保证
    /// cmd 在录制态;G14.7 面原回读录制语句逐字提取——`frame_impl_ext(out=Some)`
    /// 与 [`Self::readback_output_into`] 共用同一事实源)。
    fn record_output_readback(&mut self) {
        let out_res = self.color_out.clone_shallow();
        self.vk_barrier(&out_res, VK_IMAGE_LAYOUT_GENERAL, VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL, VK_ACCESS_SHADER_WRITE, VK_ACCESS_TRANSFER_READ, VK_PIPELINE_STAGE_COMPUTE_SHADER, VK_PIPELINE_STAGE_TRANSFER);
        let region = VkBufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: VkImageSubresourceLayers {
                aspect_mask: VK_IMAGE_ASPECT_COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_offset: [0, 0, 0],
            image_extent: VkExtent3D { width: self.out_w, height: self.out_h, depth: 1 },
        };
        // SAFETY: cmd 录制中;readback 容量 ≥ out 像素字节。
        unsafe {
            (self.dev.cmd_copy_image_to_buffer)(
                self.cmd,
                self.color_out.image,
                VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
                self.readback,
                1,
                &region,
            )
        };
        self.vk_barrier(&out_res, VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL, VK_IMAGE_LAYOUT_GENERAL, VK_ACCESS_TRANSFER_READ, VK_ACCESS_SHADER_WRITE, VK_PIPELINE_STAGE_TRANSFER, VK_PIPELINE_STAGE_COMPUTE_SHADER);
    }

    /// map readback 缓冲 → f16→f32 像素带并行转换直写 `out`(G14.7 面原转换
    /// 语句逐字提取;调用方保证 queue 已排空、readback 含本帧回读内容、
    /// `out.len()` 已验 = out_px·3)。
    fn map_convert_readback(&mut self, out: &mut [f32]) -> Result<(), VendorError> {
        // SAFETY: readback host-visible+coherent;queueWaitIdle 后无在途写;map 区间 = 帧字节。
        unsafe {
            let mut ptr: *mut c_void = std::ptr::null_mut();
            let r = (self.dev.map_memory)(self.device, self.readback_mem, 0, self.readback_size, 0, &mut ptr);
            if r != VK_SUCCESS || ptr.is_null() {
                return Err(VendorError::ApiError(format!("vkMapMemory(readback) → {r}")));
            }
            let data = std::slice::from_raw_parts(
                ptr as *const u16,
                (self.out_w * self.out_h) as usize * 4,
            );
            // G14.7:裸指针步行改 from_raw_parts 视图(U8 镜像纪律,0 新 U 号——U58
            // 扩注)+ 像素带并行转换;带内逐值同式同序,输出字节面与 G14.6 面逐位
            // 一致。切片长度 = out_px·4 = readback 分配字节/2,不越 map 区间。
            convert_out_par(data, out);
            (self.dev.unmap_memory)(self.device, self.readback_mem);
        }
        Ok(())
    }

    /// G14plus vendor 域(RFC-0030)**驻留输出**变体:跑满 pack→upload→
    /// evaluate→submit_wait,**跳过输出回读与 host f16→f32 转换**(cornell t67
    /// readback 分项 ~0.84ms、bistro ~5.8ms 消除面)。输出驻留 session 自建
    /// `color_out` image(R16G16B16A16_SFLOAT,GENERAL layout,句柄面见
    /// [`Self::output_image_raw`]);调用方决定回读时机——按需(如 N 帧一测/
    /// 末帧)调 [`Self::readback_output_into`]。既有 [`Self::upscale`]/
    /// [`Self::upscale_into`]/[`Self::probe_validation_frame`] 行为 0-byte
    /// (共用 `frame_impl_ext` 同一代码面,`out=Some` 臂调用序逐字保持)。
    pub fn upscale_resident(&mut self, input: &VendorFrameInput) -> Result<(), VendorError> {
        self.frame_impl_ext(input, false, None)
    }

    /// G14plus vendor 域(RFC-0030)**按需回读**:把驻留 `color_out` 内容回读
    /// 转换为 3 通道 f32(行主序 RGB;f16→f32 转换与 [`Self::upscale`] 同式同
    /// 序——同一 `map_convert_readback` 事实源,内容逐位一致)。单独录制
    /// copy 命令 + 同步提交 + `vkQueueWaitIdle`(同步纪律与帧路径同律)。
    /// fail-closed:`out` 长度不符或输出 image 尚无已评估内容(未跑过任何
    /// evaluate 帧,layout 仍 UNDEFINED)→ 确定性 Err。
    pub fn readback_output_into(&mut self, out: &mut [f32]) -> Result<(), VendorError> {
        if out.len() != (self.out_w * self.out_h * 3) as usize {
            return Err(VendorError::ApiError("输出切片长度与 session 输出分辨率不符".into()));
        }
        if self.color_out.layout != VK_IMAGE_LAYOUT_GENERAL {
            return Err(VendorError::ApiError(
                "输出 image 无已评估内容(先跑 upscale_resident/upscale)".into(),
            ));
        }
        let begin = VkCommandBufferBeginInfo {
            s_type: VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
            p_next: std::ptr::null(),
            flags: 0x1, // ONE_TIME_SUBMIT
            p_inheritance_info: std::ptr::null(),
        };
        // SAFETY: cmd 已分配且不在未决状态(上次提交已 queueWaitIdle 排空后 reset)。
        let r = unsafe { (self.dev.begin_command_buffer)(self.cmd, &begin) };
        if r != VK_SUCCESS {
            return Err(VendorError::ApiError(format!("vkBeginCommandBuffer → {r}")));
        }
        self.record_output_readback();
        // SAFETY: cmd 录制完成。
        let r = unsafe { (self.dev.end_command_buffer)(self.cmd) };
        if r != VK_SUCCESS {
            return Err(VendorError::ApiError(format!("vkEndCommandBuffer → {r}")));
        }
        let submit = VkSubmitInfo {
            s_type: VK_STRUCTURE_TYPE_SUBMIT_INFO,
            p_next: std::ptr::null(),
            wait_semaphore_count: 0,
            p_wait_semaphores: std::ptr::null(),
            p_wait_dst_stage_mask: std::ptr::null(),
            command_buffer_count: 1,
            p_command_buffers: &self.cmd,
            signal_semaphore_count: 0,
            p_signal_semaphores: std::ptr::null(),
        };
        // SAFETY: submit 栈上存活;cmd 为录制完成态。
        let r = unsafe { (self.dev.queue_submit)(self.queue, 1, &submit, 0) };
        if r != VK_SUCCESS {
            return Err(VendorError::ApiError(format!("vkQueueSubmit → {r}")));
        }
        // SAFETY: queue 有效;waitIdle 后无在途写。
        let r = unsafe { (self.dev.queue_wait_idle)(self.queue) };
        if r != VK_SUCCESS {
            return Err(VendorError::ApiError(format!("vkQueueWaitIdle → {r}")));
        }
        self.map_convert_readback(out)?;
        // 录制槽复用:reset cmd(与帧路径同律)。
        // SAFETY: cmd 已完成提交且 queue 排空。
        let _ = unsafe { (self.dev.reset_command_buffer)(self.cmd, 0) };
        Ok(())
    }

    /// G14.10e 诊断臂:从 **DLSS 侧 device** 回读已导入的 color 输入 image
    /// (RGBA32F 紧凑 f32×4/px 直写 `out`)——跨 device OPTIMAL tiling 解释
    /// 一致性二分面(render_exec 侧 dump 对拍:一致 → 问题在 evaluate 参数;
    /// 乱 → 跨 device tiling 布局不一致实锤)。复用 session readback buffer
    /// (容量校验 fail-closed);诊断专用,不入常规帧路径。
    pub fn debug_readback_input_color(&mut self, out: &mut Vec<f32>) -> Result<(), VendorError> {
        let Some((c, _)) = &self.ext_inputs[0] else {
            return Err(VendorError::ApiError("color 槽未导入".into()));
        };
        let img = c.clone_shallow();
        let px_bytes = u64::from(img.w) * u64::from(img.h) * 16;
        if px_bytes > self.readback_size {
            return Err(VendorError::ApiError(format!(
                "诊断回读 {px_bytes}B 超 readback 容量 {}B",
                self.readback_size
            )));
        }
        let begin = VkCommandBufferBeginInfo {
            s_type: VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
            p_next: std::ptr::null(),
            flags: 0x1,
            p_inheritance_info: std::ptr::null(),
        };
        // SAFETY: cmd 空闲(上次提交已排空后 reset)。
        let r = unsafe { (self.dev.begin_command_buffer)(self.cmd, &begin) };
        if r != VK_SUCCESS {
            return Err(VendorError::ApiError(format!("vkBeginCommandBuffer → {r}")));
        }
        self.vk_barrier(&img, VK_IMAGE_LAYOUT_GENERAL, VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL, VK_ACCESS_SHADER_READ, VK_ACCESS_TRANSFER_READ, VK_PIPELINE_STAGE_COMPUTE_SHADER, VK_PIPELINE_STAGE_TRANSFER);
        let region = VkBufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: VkImageSubresourceLayers {
                aspect_mask: VK_IMAGE_ASPECT_COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_offset: [0, 0, 0],
            image_extent: VkExtent3D { width: img.w, height: img.h, depth: 1 },
        };
        // SAFETY: cmd 录制中;容量已验。
        unsafe {
            (self.dev.cmd_copy_image_to_buffer)(
                self.cmd,
                img.image,
                VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
                self.readback,
                1,
                &region,
            )
        };
        self.vk_barrier(&img, VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL, VK_IMAGE_LAYOUT_GENERAL, VK_ACCESS_TRANSFER_READ, VK_ACCESS_SHADER_READ, VK_PIPELINE_STAGE_TRANSFER, VK_PIPELINE_STAGE_COMPUTE_SHADER);
        // SAFETY: cmd 录制完成。
        let r = unsafe { (self.dev.end_command_buffer)(self.cmd) };
        if r != VK_SUCCESS {
            return Err(VendorError::ApiError(format!("vkEndCommandBuffer → {r}")));
        }
        let submit = VkSubmitInfo {
            s_type: VK_STRUCTURE_TYPE_SUBMIT_INFO,
            p_next: std::ptr::null(),
            wait_semaphore_count: 0,
            p_wait_semaphores: std::ptr::null(),
            p_wait_dst_stage_mask: std::ptr::null(),
            command_buffer_count: 1,
            p_command_buffers: &self.cmd,
            signal_semaphore_count: 0,
            p_signal_semaphores: std::ptr::null(),
        };
        // SAFETY: submit 栈上存活。
        let r = unsafe { (self.dev.queue_submit)(self.queue, 1, &submit, 0) };
        if r != VK_SUCCESS {
            return Err(VendorError::ApiError(format!("vkQueueSubmit → {r}")));
        }
        // SAFETY: queue 有效。
        let r = unsafe { (self.dev.queue_wait_idle)(self.queue) };
        if r != VK_SUCCESS {
            return Err(VendorError::ApiError(format!("vkQueueWaitIdle → {r}")));
        }
        out.clear();
        out.reserve((img.w * img.h * 4) as usize);
        // SAFETY: readback host-visible+coherent;queue 已排空;区间=px_bytes。
        unsafe {
            let mut ptr: *mut c_void = std::ptr::null_mut();
            let r = (self.dev.map_memory)(self.device, self.readback_mem, 0, px_bytes, 0, &mut ptr);
            if r != VK_SUCCESS || ptr.is_null() {
                return Err(VendorError::ApiError(format!("vkMapMemory(诊断回读) → {r}")));
            }
            let data = std::slice::from_raw_parts(ptr as *const f32, (img.w * img.h * 4) as usize);
            out.extend_from_slice(data);
            (self.dev.unmap_memory)(self.device, self.readback_mem);
        }
        // SAFETY: cmd 已排空。
        let _ = unsafe { (self.dev.reset_command_buffer)(self.cmd, 0) };
        Ok(())
    }

    /// G14plus vendor 域(RFC-0030)输出 image 原生句柄簿记导出(只读;句柄归
    /// session 所有,调用方**不得销毁/越 session 生命周期持有**)。注意:DLSS
    /// session 持**独立** VkInstance/VkDevice(SL 代理创建)——本句柄不可直接
    /// 作其它 VkDevice(如 render_exec session)的 image 消费;跨 device 共享需
    /// VK_KHR_external_memory 导出/导入改造(RFC-0030 裁决面,本 accessor 仅
    /// 暴露簿记事实供 bin 侧遥测/评估)。
    pub fn output_image_raw(&self) -> DlssOutputImageRaw {
        DlssOutputImageRaw {
            image: self.color_out.image,
            memory: self.color_out.memory,
            view: self.color_out.view,
            vk_format: self.color_out.format,
            layout: self.color_out.layout,
            width: self.color_out.w,
            height: self.color_out.h,
        }
    }

    /// validation ERROR 级累计计数(我方调用实错,白名单豁免不计;messenger 在位时,否则 0)。
    pub fn validation_errors(&self) -> u64 {
        if self.validation_counter.is_null() {
            return 0;
        }
        // SAFETY: session 存活期 counter 有效(messenger 先毁)。
        unsafe { (*self.validation_counter).errors.load(Ordering::Relaxed) }
    }

    /// NGX 内部伪报豁免计数与豁免 VUID 名(evidence 全透明登记面)。
    pub fn validation_excluded(&self) -> (u64, Vec<String>) {
        if self.validation_counter.is_null() {
            return (0, Vec::new());
        }
        // SAFETY: session 存活期 counter 有效(messenger 先毁)。
        unsafe {
            let c = &*self.validation_counter;
            let n = c.excluded_ngx_internal.load(Ordering::Relaxed);
            let names = c.excluded_names.lock().map(|v| v.clone()).unwrap_or_default();
            (n, names)
        }
    }

    /// session 报告(evidence provenance 面)。
    pub fn report(&self) -> VendorSessionReport {
        VendorSessionReport {
            backend: "dlss_sr_streamline_2.10.3_vulkan_interop".into(),
            gpu_name: self.gpu_name.clone(),
            validation_errors: self.validation_errors(),
            dlls: self.dlls.clone(),
            engine_version: self.ngx_version.clone(),
            fsr4_ml_available: None,
            fsr4_note: None,
            available_versions: Vec::new(),
            log_tail: self.log_tail.clone(),
        }
    }

    // ── G14.10b external memory 导入面(RFC-0030 §4.3;输入驻留)──

    /// 物理设备 LUID(创建期实采;`None` = 驱动报无效)。消费侧与 render_exec
    /// session 的 [`physical_device_luid`](crate::render_exec::DeviceFrameSession::physical_device_luid)
    /// 对拍——**同 adapter 才可共享 external memory**,不同即 fail-closed。
    pub fn physical_device_luid(&self) -> Option<[u8; 8]> {
        self.device_luid
    }

    /// device 创建时 external memory 两扩展是否已启用(设备扩展在位才启;
    /// false 时 [`Self::import_win32_input`] 确定性 `Err`)。
    pub fn external_memory_enabled(&self) -> bool {
        self.external_memory_enabled
    }

    /// G14.10b:导入 render_exec exportable 纹理为 DLSS 外部输入(OPAQUE_WIN32
    /// NT handle → 参数一致的 external image + `VkImportMemoryWin32HandleInfoKHR`
    /// dedicated 导入 + bind + view)。fail-closed:扩展未启用 / 尺寸与 session
    /// 输入分辨率不符 / 槽已导入 → 确定性 `Err`。导入后经
    /// [`Self::upscale_resident_external`] evaluate;调用纪律(内容有效性):
    /// 每帧须先等 render_exec 侧该帧 fence 完成(其 cmd 末已录 EXTERNAL
    /// release),本方法产物在 evaluate cmd 首段录对应 acquire。
    pub fn import_win32_input(
        &mut self,
        slot: ExternalInputSlot,
        desc: &ExternalImageImportDesc,
    ) -> Result<(), VendorError> {
        if !self.external_memory_enabled {
            return Err(VendorError::DeviceUnavailable(
                "VK_KHR_external_memory_win32 未启用(设备扩展不在位),导入面不可用".into(),
            ));
        }
        if desc.width != self.in_w || desc.height != self.in_h {
            return Err(VendorError::ApiError(format!(
                "导入 image 尺寸 {}x{} 与 session 输入 {}x{} 不符",
                desc.width, desc.height, self.in_w, self.in_h
            )));
        }
        if desc.handle == 0 {
            return Err(VendorError::ApiError("导入 handle 为空".into()));
        }
        let idx = match slot {
            ExternalInputSlot::Color => 0,
            ExternalInputSlot::Depth => 1,
            ExternalInputSlot::Mv => 2,
        };
        if self.ext_inputs[idx].is_some() {
            return Err(VendorError::ApiError(format!(
                "槽 {slot:?} 已导入(单次导入纪律,重导请重建 session)"
            )));
        }
        let aspect = if desc.vk_format == VK_FORMAT_D32_SFLOAT {
            VK_IMAGE_ASPECT_DEPTH
        } else {
            VK_IMAGE_ASPECT_COLOR
        };
        // image:参数与导出侧一致(format/extent/OPTIMAL/usage)+ external chain。
        let ext_info = VkExternalMemoryImageCreateInfo {
            s_type: VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_IMAGE_CREATE_INFO,
            p_next: std::ptr::null(),
            handle_types: VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_WIN32,
        };
        let ici = VkImageCreateInfo {
            s_type: VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO,
            p_next: (&ext_info as *const VkExternalMemoryImageCreateInfo).cast(),
            flags: 0,
            // `VK_IMAGE_TYPE_2D` = **1**(3D = 2)。G14.12 实锤:本处原为 `2` 且
            // 注为 "2D"——即导入侧把导出侧的 2D image 当 3D 重建,同一块显存被
            // 两侧按不同 tiling 布局解释,读出确定性块状乱序。这正是 G14.10f
            // 把 image 共享判为「OPAQUE_WIN32 跨 device 布局解释不一致」并退回
            // buffer+copy 的**真实根因**(驱动无过)。本机 memreq 对拍实证:
            // 1920×1080 RGBA16F OPTIMAL,imageType=2D 需 17694720 字节、
            // 3D 需 16588800 字节——尺寸不同 ⇒ 布局不同。render_exec 导出侧恒
            // `IMAGE_TYPE_2D`(=1),此处必须同值。
            image_type: VK_IMAGE_TYPE_2D,
            format: desc.vk_format,
            extent: VkExtent3D { width: desc.width, height: desc.height, depth: 1 },
            mip_levels: 1,
            array_layers: 1,
            samples: 1,
            tiling: 0, // OPTIMAL
            usage: desc.usage_flags,
            sharing_mode: 0,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
            initial_layout: VK_IMAGE_LAYOUT_UNDEFINED,
        };
        let mut image: VkImage = 0;
        // SAFETY: ici/ext_info 栈上存活;device 有效。
        let r = unsafe { (self.dev.create_image)(self.device, &ici, std::ptr::null(), &mut image) };
        if r != VK_SUCCESS || image == 0 {
            return Err(VendorError::ApiError(format!(
                "vkCreateImage(import fmt={}) → {r}",
                desc.vk_format
            )));
        }
        // 导入分配:import(handle)→ dedicated(image);allocationSize/
        // memoryTypeIndex 采导出侧簿记(LUID 对拍先行,类型序一致)。
        let dedicated = VkMemoryDedicatedAllocateInfo {
            s_type: VK_STRUCTURE_TYPE_MEMORY_DEDICATED_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            image,
            buffer: 0,
        };
        let import_info = VkImportMemoryWin32HandleInfoKHR {
            s_type: VK_STRUCTURE_TYPE_IMPORT_MEMORY_WIN32_HANDLE_INFO_KHR,
            p_next: (&dedicated as *const VkMemoryDedicatedAllocateInfo).cast(),
            handle_type: VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_WIN32,
            handle: desc.handle as *mut c_void,
            name: std::ptr::null(),
        };
        let mai = VkMemoryAllocateInfo {
            s_type: VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
            p_next: (&import_info as *const VkImportMemoryWin32HandleInfoKHR).cast(),
            allocation_size: desc.allocation_size,
            memory_type_index: desc.memory_type_index,
        };
        let mut memory: VkDeviceMemory = 0;
        // SAFETY: mai/import_info/dedicated 栈上存活;handle 归导出 session 所有
        // (导入不夺所有权,vkAllocateMemory import 语义引用计数)。
        let r = unsafe { (self.dev.allocate_memory)(self.device, &mai, std::ptr::null(), &mut memory) };
        if r != VK_SUCCESS || memory == 0 {
            // SAFETY: image 本函数创建,未 bind。
            unsafe { (self.dev.destroy_image)(self.device, image, std::ptr::null()) };
            return Err(VendorError::ApiError(format!(
                "vkAllocateMemory(import win32) → {r}"
            )));
        }
        // SAFETY: image/memory 配对有效(dedicated 导入,offset 0)。
        let r = unsafe { (self.dev.bind_image_memory)(self.device, image, memory, 0) };
        if r != VK_SUCCESS {
            // SAFETY: 本函数创建,逆序释放。
            unsafe {
                (self.dev.destroy_image)(self.device, image, std::ptr::null());
                (self.dev.free_memory)(self.device, memory, std::ptr::null());
            }
            return Err(VendorError::ApiError(format!("vkBindImageMemory(import) → {r}")));
        }
        let vci = VkImageViewCreateInfo {
            s_type: VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: 0,
            image,
            view_type: 1, // 2D
            format: desc.vk_format,
            components: [0; 4],
            subresource_range: VkImageSubresourceRange {
                aspect_mask: aspect,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
        };
        let mut view: VkImageView = 0;
        // SAFETY: vci 栈上存活。
        let r = unsafe { (self.dev.create_image_view)(self.device, &vci, std::ptr::null(), &mut view) };
        if r != VK_SUCCESS || view == 0 {
            // SAFETY: 本函数创建,逆序释放。
            unsafe {
                (self.dev.destroy_image)(self.device, image, std::ptr::null());
                (self.dev.free_memory)(self.device, memory, std::ptr::null());
            }
            return Err(VendorError::ApiError(format!("vkCreateImageView(import) → {r}")));
        }
        // 跨界 layout 协定:恒 GENERAL(render_exec 帧末 release 已转换;acquire
        // 于 evaluate cmd 首段配对)。
        self.ext_inputs[idx] = Some((
            VkImageRes {
                image,
                memory,
                view,
                layout: VK_IMAGE_LAYOUT_GENERAL,
                format: desc.vk_format,
                w: desc.width,
                h: desc.height,
                aspect,
            },
            desc.usage_flags,
        ));
        Ok(())
    }

    /// G14.10b:外部输入(三槽均已 [`Self::import_win32_input`])的驻留
    /// evaluate——**零 host 输入中转**:color/depth/mv 内容驻留导入 image,
    /// cmd 首段录三条 acquire barrier(`VK_QUEUE_FAMILY_EXTERNAL`→本家族,
    /// GENERAL→GENERAL,与 render_exec 帧末 release 配对——跨 device 内容
    /// 有效性经 queue 提交边界 + release/acquire 对保证;CPU 侧调用纪律:
    /// render_exec 该帧 fence 完成后才调本方法)。reactive 仍走 session 自建
    /// image 的 staging 上传(`Some` = pack 上传,`None` = 零填充,与既有
    /// pack 语义一致)。输出驻留 `color_out`(GENERAL),按需
    /// [`Self::readback_output_into`]。既有 upscale/upscale_into/
    /// upscale_resident 行为 0-byte(本方法独立 cmd 录制面)。
    pub fn upscale_resident_external(
        &mut self,
        p: &VendorExternalFrameParams<'_>,
    ) -> Result<(), VendorError> {
        let px = (self.in_w * self.in_h) as usize;
        // 分解遥测(`RURIX_VENDOR_TIMING=1` 门控,默认关零行为变更;轴 =
        // staging〔reactive〕/ sl_book / record〔acquire+reactive 上传〕/
        // evaluate〔slEvaluateFeature CPU 录制〕/ submit_wait〔GPU 执行+同步〕
        // ——与 buffer 路 `dlss-buf` 行同轴,便于逐项对拍 copy 消除的收益)。
        let vtm_on = std::env::var("RURIX_VENDOR_TIMING").ok().as_deref() == Some("1");
        let vtm_t0 = std::time::Instant::now();
        let mut vtm_staging = std::time::Duration::ZERO;
        let mut vtm_book = std::time::Duration::ZERO;
        let mut vtm_record = std::time::Duration::ZERO;
        let mut vtm_eval = std::time::Duration::ZERO;
        let [Some((c, cu)), Some((d, du)), Some((m, mu))] = &self.ext_inputs else {
            return Err(VendorError::ApiError(
                "外部输入未齐(color/depth/mv 三槽均须先 import_win32_input)".into(),
            ));
        };
        let (ext_color, color_usage) = (c.clone_shallow(), *cu);
        let (ext_depth, depth_usage) = (d.clone_shallow(), *du);
        let (ext_mv, mv_usage) = (m.clone_shallow(), *mu);
        if let Some(r) = p.reactive
            && r.len() != px
        {
            return Err(VendorError::ApiError("reactive 切片长度不符".into()));
        }
        // ── reactive staging(区段 [0, px);仅 Some→R8 pack)──
        // G15plus-II 候选 b(reactive 按需化):生产车道恒 `reactive=None`——
        // SL 注册面 reactive 非 required tag(kBufferTypeDepth/MotionVectors/
        // ScalingInputColor/ScalingOutputColor 四项为 required;SL verbose 日志
        // 逐字在案),NGX 缺省 = 零 mask 语义 ⇒ None 帧不再上传零 mask、不再
        // 附带 reactive tag(位级同一以 L0 digest 探针钉死:bistro t50/t100 +
        // cornell t67 末帧 digest == G14.12 冻结锚);Some 帧维持原 R8 pack +
        // 上传 + tag 全链(该形态语义 0-byte)。原 G14.12「恒零 mask 驻留跳过」
        // 面由「无 tag 即零 mask」结构性吸收。
        if let Some(rv) = p.reactive {
        // SAFETY: staging host-visible+coherent;px ≤ staging_size(建面 21B/px);
        // map/写/unmap 单线程序列化;rv.len()==px 上方校验。
        unsafe {
            let mut ptr: *mut c_void = std::ptr::null_mut();
            let r = (self.dev.map_memory)(self.device, self.staging_mem, 0, self.staging_size, 0, &mut ptr);
            if r != VK_SUCCESS || ptr.is_null() {
                return Err(VendorError::ApiError(format!("vkMapMemory(staging) → {r}")));
            }
            let reac = std::slice::from_raw_parts_mut(ptr as *mut u8, px);
            for (o, &v) in reac.iter_mut().zip(rv.iter()) {
                *o = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
            (self.dev.unmap_memory)(self.device, self.staging_mem);
        }
        }
        if vtm_on {
            vtm_staging = vtm_t0.elapsed();
        }

        // ── SL 簿记:frame token → constants → options(与 frame_impl_ext 同序;
        // constants 经临时 VendorFrameInput 借 build_sl_constants 单一事实源,
        // 内容切片不消费)──
        let mut token: *mut c_void = std::ptr::null_mut();
        // SAFETY: token 出参栈上有效。
        let r = unsafe { (self.fns.sl_get_new_frame_token)(&mut token, &p.frame_index) };
        if r != SL_OK || token.is_null() {
            return Err(VendorError::VendorCall(format!(
                "slGetNewFrameToken → {}",
                sl_result_name(r)
            )));
        }
        let tmp_input = VendorFrameInput {
            color: &[],
            depth: &[],
            mv: &[],
            reactive: None,
            exposure: p.exposure,
            jitter: p.jitter,
            frame_index: p.frame_index,
            reset: p.reset,
        };
        let consts = build_sl_constants(&tmp_input, self.in_w, self.in_h);
        // SAFETY: consts 栈上存活;token/viewport 有效。
        let r = unsafe { (self.fns.sl_set_constants)(&consts, token, &self.viewport) };
        if r != SL_OK {
            return Err(VendorError::VendorCall(format!(
                "slSetConstants → {}",
                sl_result_name(r)
            )));
        }
        let opts = SlDlssOptions {
            base: sl_base(SL_GUID_DLSS_OPTIONS, 3),
            mode: SL_DLSS_MODE_MAX_PERFORMANCE,
            output_width: self.out_w,
            output_height: self.out_h,
            sharpness: 0.0,
            pre_exposure: p.exposure,
            exposure_scale: 1.0,
            color_buffers_hdr: 1,
            indicator_invert_axis_x: 0,
            indicator_invert_axis_y: 0,
            _pad0: 0,
            dlaa_preset: 0,
            quality_preset: 0,
            balanced_preset: 0,
            performance_preset: 0,
            ultra_performance_preset: 0,
            ultra_quality_preset: 0,
            use_auto_exposure: 0,
            alpha_upscaling_enabled: 0,
            _pad1: [0; 2],
        };
        // SAFETY: opts/viewport 栈上存活。
        let r = unsafe { (self.dlss.dlss_set_options)(&self.viewport, &opts) };
        if r != SL_OK {
            return Err(VendorError::VendorCall(format!(
                "slDLSSSetOptions → {}",
                sl_result_name(r)
            )));
        }

        if vtm_on {
            vtm_book = vtm_t0.elapsed();
        }
        let begin = VkCommandBufferBeginInfo {
            s_type: VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
            p_next: std::ptr::null(),
            flags: 0x1, // ONE_TIME_SUBMIT
            p_inheritance_info: std::ptr::null(),
        };
        // SAFETY: cmd 不在未决状态(上次提交已 queueWaitIdle 排空后 reset)。
        let r = unsafe { (self.dev.begin_command_buffer)(self.cmd, &begin) };
        if r != VK_SUCCESS {
            return Err(VendorError::ApiError(format!("vkBeginCommandBuffer → {r}")));
        }
        // ── acquire ×3(EXTERNAL→本家族,GENERAL→GENERAL 零转换;与 render_exec
        // 帧末 release 逐帧配对——本方法每帧调用,render_exec 每帧 release)。
        // G14.12:三条并入单次 barrier 调用。──
        self.vk_images_acquire_external_batched(&[&ext_color, &ext_depth, &ext_mv]);
        // reactive 上传(staging 区段 0;自建 image,既有布局状态机)。G15plus-II
        // 候选 b:仅 `Some(..)` 帧上传并附带 tag;None 帧不上传不 tag(缺省 =
        // 零 mask 语义,见 staging 段登记)。
        if p.reactive.is_some() {
            self.vk_upload_image(VkInputSlot::Reactive, 0);
        }
        // color_out 置 GENERAL(DLSS UAV 写;与 frame_impl_ext 同律)。
        if self.color_out.layout == VK_IMAGE_LAYOUT_UNDEFINED {
            let out_res = self.color_out.clone_shallow();
            self.vk_barrier(&out_res, VK_IMAGE_LAYOUT_UNDEFINED, VK_IMAGE_LAYOUT_GENERAL, 0, VK_ACCESS_SHADER_WRITE, VK_PIPELINE_STAGE_TOP_OF_PIPE, VK_PIPELINE_STAGE_COMPUTE_SHADER);
            self.color_out.layout = VK_IMAGE_LAYOUT_GENERAL;
        }

        // ── ResourceTag 集(外部输入 state=GENERAL,usage=导出侧位;reactive/
        // color_out 沿既有簿记)──
        let mk_ext_res = |res: &VkImageRes, usage: u32| -> SlResource {
            SlResource {
                base: sl_base(SL_GUID_RESOURCE, 1),
                res_type: SL_RESOURCE_TEX2D,
                _pad0: [0; 7],
                native: res.image as usize as *mut c_void,
                memory: res.memory as usize as *mut c_void,
                view: res.view as usize as *mut c_void,
                state: res.layout as u32,
                width: res.w,
                height: res.h,
                native_format: res.format,
                mip_levels: 1,
                array_layers: 1,
                gpu_virtual_address: 0,
                flags: 0,
                usage,
                reserved: 0,
            }
        };
        let sl_color_in = mk_ext_res(&ext_color, color_usage);
        let sl_depth = mk_ext_res(&ext_depth, depth_usage);
        let sl_mv = mk_ext_res(&ext_mv, mv_usage);
        // G15plus-II 候选 b:reactive SlResource/tag 仅 Some 帧构建(缺省 =
        // 零 mask;None 帧 4 tag 集 = required 四面齐备)。
        let sl_reactive = p.reactive.map(|_| {
            mk_ext_res(
                &self.reactive_in.clone_shallow(),
                VK_IMAGE_USAGE_SAMPLED | VK_IMAGE_USAGE_TRANSFER_DST,
            )
        });
        let sl_color_out = mk_ext_res(
            &self.color_out.clone_shallow(),
            VK_IMAGE_USAGE_STORAGE | VK_IMAGE_USAGE_TRANSFER_SRC,
        );
        let extent_in = SlExtent { top: 0, left: 0, width: self.in_w, height: self.in_h };
        let extent_out = SlExtent { top: 0, left: 0, width: self.out_w, height: self.out_h };
        let mut tags: Vec<SlResourceTag> = Vec::with_capacity(5);
        tags.push(SlResourceTag { base: sl_base(SL_GUID_RESOURCE_TAG, 1), resource: &sl_color_in, tag_type: SL_BUFFER_SCALING_INPUT_COLOR, lifecycle: SL_LIFECYCLE_ONLY_VALID_NOW, extent: extent_in });
        tags.push(SlResourceTag { base: sl_base(SL_GUID_RESOURCE_TAG, 1), resource: &sl_color_out, tag_type: SL_BUFFER_SCALING_OUTPUT_COLOR, lifecycle: SL_LIFECYCLE_ONLY_VALID_NOW, extent: extent_out });
        tags.push(SlResourceTag { base: sl_base(SL_GUID_RESOURCE_TAG, 1), resource: &sl_depth, tag_type: SL_BUFFER_DEPTH, lifecycle: SL_LIFECYCLE_ONLY_VALID_NOW, extent: extent_in });
        tags.push(SlResourceTag { base: sl_base(SL_GUID_RESOURCE_TAG, 1), resource: &sl_mv, tag_type: SL_BUFFER_MV, lifecycle: SL_LIFECYCLE_ONLY_VALID_NOW, extent: extent_in });
        if let Some(reac) = &sl_reactive {
            tags.push(SlResourceTag { base: sl_base(SL_GUID_RESOURCE_TAG, 1), resource: reac, tag_type: SL_BUFFER_REACTIVE_MASK, lifecycle: SL_LIFECYCLE_ONLY_VALID_NOW, extent: extent_in });
        }
        let mut tag_ptrs: Vec<*const SlBaseStructure> = Vec::with_capacity(6);
        tag_ptrs.push(&self.viewport.base as *const _);
        for t in &tags {
            tag_ptrs.push(&t.base as *const _);
        }
        if vtm_on {
            vtm_record = vtm_t0.elapsed();
        }
        // SAFETY: tags/tag_ptrs/sl_* 栈上存活至 evaluate 返回(eOnlyValidNow
        // 语义);cmd 录制中;token 本帧有效;tag 计数与 tags 长度一致。
        let r = unsafe {
            (self.fns.sl_evaluate_feature)(SL_FEATURE_DLSS, token, tag_ptrs.as_ptr(), tag_ptrs.len() as u32, self.cmd)
        };
        if vtm_on {
            vtm_eval = vtm_t0.elapsed();
        }
        if r != SL_OK {
            // cmd 处于录制态,收敛后返错(end+reset,不提交)。
            // SAFETY: cmd 录制中 → end;错误路径不提交。
            unsafe {
                let _ = (self.dev.end_command_buffer)(self.cmd);
                let _ = (self.dev.reset_command_buffer)(self.cmd, 0);
            }
            return Err(VendorError::VendorCall(format!(
                "slEvaluateFeature(DLSS external) → {}",
                sl_result_name(r)
            )));
        }
        // SAFETY: cmd 录制完成。
        let r = unsafe { (self.dev.end_command_buffer)(self.cmd) };
        if r != VK_SUCCESS {
            return Err(VendorError::ApiError(format!("vkEndCommandBuffer → {r}")));
        }
        let submit = VkSubmitInfo {
            s_type: VK_STRUCTURE_TYPE_SUBMIT_INFO,
            p_next: std::ptr::null(),
            wait_semaphore_count: 0,
            p_wait_semaphores: std::ptr::null(),
            p_wait_dst_stage_mask: std::ptr::null(),
            command_buffer_count: 1,
            p_command_buffers: &self.cmd,
            signal_semaphore_count: 0,
            p_signal_semaphores: std::ptr::null(),
        };
        // SAFETY: submit 栈上存活;cmd 录制完成态。
        let r = unsafe { (self.dev.queue_submit)(self.queue, 1, &submit, 0) };
        if r != VK_SUCCESS {
            return Err(VendorError::ApiError(format!("vkQueueSubmit → {r}")));
        }
        // SAFETY: queue 有效;waitIdle 后无在途写(第一版 CPU 顺序化同步纪律)。
        let r = unsafe { (self.dev.queue_wait_idle)(self.queue) };
        if r != VK_SUCCESS {
            return Err(VendorError::ApiError(format!("vkQueueWaitIdle → {r}")));
        }
        // SAFETY: cmd 已提交且 queue 排空。
        let _ = unsafe { (self.dev.reset_command_buffer)(self.cmd, 0) };
        if vtm_on {
            let ms = |d: std::time::Duration| d.as_secs_f64() * 1e3;
            eprintln!(
                "[vendor-timing dlss-ext] frame={} staging={:.3} sl_book={:.3} record={:.3} evaluate={:.3} submit_wait={:.3} total={:.3}ms",
                p.frame_index,
                ms(vtm_staging),
                ms(vtm_book - vtm_staging),
                ms(vtm_record - vtm_book),
                ms(vtm_eval - vtm_record),
                ms(vtm_t0.elapsed() - vtm_eval),
                ms(vtm_t0.elapsed()),
            );
        }
        Ok(())
    }

    /// G14.10f:导入 exportable **buffer**(OPAQUE_WIN32;render_exec
    /// [`ExportedBufferWin32`] 对侧)。usage 恒 TRANSFER_SRC(每帧 copy 源;
    /// buffer usage 为 per-device 对象属性,可异于导出侧 STORAGE)。
    /// memoryTypeIndex 采导出侧簿记(LUID 对拍先行,类型序一致);dedicated
    /// (buffer) 绑定。单次导入纪律同 image 版。
    pub fn import_win32_buffer_input(
        &mut self,
        slot: ExternalInputSlot,
        desc: &ExternalBufferImportDesc,
    ) -> Result<(), VendorError> {
        if !self.external_memory_enabled {
            return Err(VendorError::DeviceUnavailable(
                "VK_KHR_external_memory_win32 未启用(设备扩展不在位),导入面不可用".into(),
            ));
        }
        if desc.handle == 0 {
            return Err(VendorError::ApiError("导入 handle 为空".into()));
        }
        let idx = match slot {
            ExternalInputSlot::Color => 0,
            ExternalInputSlot::Depth => 1,
            ExternalInputSlot::Mv => 2,
        };
        if self.ext_input_bufs[idx].is_some() {
            return Err(VendorError::ApiError(format!(
                "buffer 槽 {slot:?} 已导入(单次导入纪律,重导请重建 session)"
            )));
        }
        let ext_info = VkExternalMemoryBufferCreateInfo {
            s_type: VK_STRUCTURE_TYPE_EXTERNAL_MEMORY_BUFFER_CREATE_INFO,
            p_next: std::ptr::null(),
            handle_types: VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_WIN32,
        };
        let bci = VkBufferCreateInfo {
            s_type: VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO,
            p_next: (&ext_info as *const VkExternalMemoryBufferCreateInfo).cast(),
            flags: 0,
            size: desc.size,
            usage: VK_BUFFER_USAGE_TRANSFER_SRC,
            sharing_mode: 0,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
        };
        let mut buffer: VkBuffer = 0;
        // SAFETY: bci/ext_info 栈上存活;device 有效。
        let r = unsafe { (self.dev.create_buffer)(self.device, &bci, std::ptr::null(), &mut buffer) };
        if r != VK_SUCCESS || buffer == 0 {
            return Err(VendorError::ApiError(format!(
                "vkCreateBuffer(import,size={}) → {r}",
                desc.size
            )));
        }
        let dedicated = VkMemoryDedicatedAllocateInfo {
            s_type: VK_STRUCTURE_TYPE_MEMORY_DEDICATED_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            image: 0,
            buffer,
        };
        let import_info = VkImportMemoryWin32HandleInfoKHR {
            s_type: VK_STRUCTURE_TYPE_IMPORT_MEMORY_WIN32_HANDLE_INFO_KHR,
            p_next: (&dedicated as *const VkMemoryDedicatedAllocateInfo).cast(),
            handle_type: VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_WIN32,
            handle: desc.handle as *mut c_void,
            name: std::ptr::null(),
        };
        let mai = VkMemoryAllocateInfo {
            s_type: VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
            p_next: (&import_info as *const VkImportMemoryWin32HandleInfoKHR).cast(),
            allocation_size: desc.allocation_size,
            memory_type_index: desc.memory_type_index,
        };
        let mut memory: VkDeviceMemory = 0;
        // SAFETY: mai/import_info/dedicated 栈上存活;import 引用计数语义。
        let r = unsafe { (self.dev.allocate_memory)(self.device, &mai, std::ptr::null(), &mut memory) };
        if r != VK_SUCCESS || memory == 0 {
            // SAFETY: buffer 本函数创建,未 bind。
            unsafe { (self.dev.destroy_buffer)(self.device, buffer, std::ptr::null()) };
            return Err(VendorError::ApiError(format!(
                "vkAllocateMemory(import buffer win32) → {r}"
            )));
        }
        // SAFETY: buffer/memory 配对有效(dedicated 导入,offset 0)。
        let r = unsafe { (self.dev.bind_buffer_memory)(self.device, buffer, memory, 0) };
        if r != VK_SUCCESS {
            // SAFETY: 本函数创建,逆序释放。
            unsafe {
                (self.dev.destroy_buffer)(self.device, buffer, std::ptr::null());
                (self.dev.free_memory)(self.device, memory, std::ptr::null());
            }
            return Err(VendorError::ApiError(format!(
                "vkBindBufferMemory(import) → {r}"
            )));
        }
        self.ext_input_bufs[idx] = Some((buffer, memory, desc.size));
        Ok(())
    }

    /// G14.10f:**buffer 共享**驻留 evaluate——外部输入三槽为导入 buffer
    /// (线性布局跨 device 无歧义;OPTIMAL image 共享的布局解释不一致缺陷的
    /// 正解)。cmd 首段:三条 EXTERNAL acquire buffer barrier(与 render_exec
    /// 帧末 buffer release 配对)→ 三条 `vkCmdCopyBufferToImage` 进 session
    /// **自建输入 image**(与 host staging 路同一批 color_in/depth_in/mv_in,
    /// 同一 SL tag 面/布局状态机——buffer 内容契约:color RGBA16F 8B/px 紧凑、
    /// depth f32 4B/px(D32 image)、mv RG32F 8B/px)→ reactive staging 上传
    /// → evaluate。输入位面与 host 路径逐位同构(f16 pack 语义一致时)。
    /// 输出驻留 `color_out`,按需 [`Self::readback_output_into`]。
    pub fn upscale_resident_buffers(
        &mut self,
        p: &VendorExternalFrameParams<'_>,
    ) -> Result<(), VendorError> {
        let px = (self.in_w * self.in_h) as usize;
        let [Some((cb, _, cbs)), Some((db, _, dbs)), Some((mb, _, mbs))] = self.ext_input_bufs
        else {
            return Err(VendorError::ApiError(
                "外部输入 buffer 未齐(color/depth/mv 三槽均须先 import_win32_buffer_input)".into(),
            ));
        };
        if let Some(r) = p.reactive
            && r.len() != px
        {
            return Err(VendorError::ApiError("reactive 切片长度不符".into()));
        }
        // ── reactive staging(区段 [0, px);Some→R8 pack,None→零填充)──
        // G14.11:None 且零内容已驻留 → 整段跳过(内容恒定,见
        // `reactive_zero_resident` 字段契约);首帧/Some 帧照常。
        let skip_reactive = p.reactive.is_none() && self.reactive_zero_resident;
        // 分解遥测(`RURIX_VENDOR_TIMING=1` 门控,默认关零行为变更;轴 =
        // staging / sl_book / record〔acquire+三 copy+reactive〕/ evaluate /
        // submit_wait)。
        let vtm_on = std::env::var("RURIX_VENDOR_TIMING").ok().as_deref() == Some("1");
        let vtm_t0 = std::time::Instant::now();
        if !skip_reactive {
            // SAFETY: staging host-visible+coherent;px ≤ staging_size;单线程序列化。
            unsafe {
                let mut ptr: *mut c_void = std::ptr::null_mut();
                let r = (self.dev.map_memory)(self.device, self.staging_mem, 0, self.staging_size, 0, &mut ptr);
                if r != VK_SUCCESS || ptr.is_null() {
                    return Err(VendorError::ApiError(format!("vkMapMemory(staging) → {r}")));
                }
                let reac = std::slice::from_raw_parts_mut(ptr as *mut u8, px);
                match p.reactive {
                    Some(rv) => {
                        for (o, &v) in reac.iter_mut().zip(rv.iter()) {
                            *o = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                        }
                    }
                    None => reac.fill(0),
                }
                (self.dev.unmap_memory)(self.device, self.staging_mem);
            }
        }

        let vtm_staging = vtm_t0.elapsed();

        // ── SL 簿记(frame_impl_ext 同序;constants 单一事实源)──
        let mut token: *mut c_void = std::ptr::null_mut();
        // SAFETY: token 出参栈上有效。
        let r = unsafe { (self.fns.sl_get_new_frame_token)(&mut token, &p.frame_index) };
        if r != SL_OK || token.is_null() {
            return Err(VendorError::VendorCall(format!(
                "slGetNewFrameToken → {}",
                sl_result_name(r)
            )));
        }
        let tmp_input = VendorFrameInput {
            color: &[],
            depth: &[],
            mv: &[],
            reactive: None,
            exposure: p.exposure,
            jitter: p.jitter,
            frame_index: p.frame_index,
            reset: p.reset,
        };
        let consts = build_sl_constants(&tmp_input, self.in_w, self.in_h);
        // SAFETY: consts 栈上存活;token/viewport 有效。
        let r = unsafe { (self.fns.sl_set_constants)(&consts, token, &self.viewport) };
        if r != SL_OK {
            return Err(VendorError::VendorCall(format!(
                "slSetConstants → {}",
                sl_result_name(r)
            )));
        }
        let opts = SlDlssOptions {
            base: sl_base(SL_GUID_DLSS_OPTIONS, 3),
            mode: SL_DLSS_MODE_MAX_PERFORMANCE,
            output_width: self.out_w,
            output_height: self.out_h,
            sharpness: 0.0,
            pre_exposure: p.exposure,
            exposure_scale: 1.0,
            color_buffers_hdr: 1,
            indicator_invert_axis_x: 0,
            indicator_invert_axis_y: 0,
            _pad0: 0,
            dlaa_preset: 0,
            quality_preset: 0,
            balanced_preset: 0,
            performance_preset: 0,
            ultra_performance_preset: 0,
            ultra_quality_preset: 0,
            use_auto_exposure: 0,
            alpha_upscaling_enabled: 0,
            _pad1: [0; 2],
        };
        // SAFETY: opts/viewport 栈上存活。
        let r = unsafe { (self.dlss.dlss_set_options)(&self.viewport, &opts) };
        if r != SL_OK {
            return Err(VendorError::VendorCall(format!(
                "slDLSSSetOptions → {}",
                sl_result_name(r)
            )));
        }

        let vtm_book = vtm_t0.elapsed();

        let begin = VkCommandBufferBeginInfo {
            s_type: VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
            p_next: std::ptr::null(),
            flags: 0x1, // ONE_TIME_SUBMIT
            p_inheritance_info: std::ptr::null(),
        };
        // SAFETY: cmd 不在未决状态(上次提交已 queueWaitIdle 排空后 reset)。
        let r = unsafe { (self.dev.begin_command_buffer)(self.cmd, &begin) };
        if r != VK_SUCCESS {
            return Err(VendorError::ApiError(format!("vkBeginCommandBuffer → {r}")));
        }
        // ── acquire buffer ×3(EXTERNAL→本家族;与 render_exec 帧末 buffer
        // release 逐帧配对)──
        self.vk_buffers_acquire_external_batched([(cb, cbs), (db, dbs), (mb, mbs)]);
        // ── copy ×3:导入 buffer → session 自建输入 image(布局状态机/后置
        // SHADER_READ_ONLY 转换均由 vk_upload_image_src 承担)──
        // 诊断实验门:RURIX_G14_DLSS_SKIP_COPY=1 跳过三条 buffer→image copy
        // (输出内容无效,仅用于分离 copy 与 DLSS 网络的 GPU 时间占比)。
        if std::env::var("RURIX_G14_DLSS_SKIP_COPY").ok().as_deref() != Some("1") {
            self.vk_upload_images_batched([
                (VkInputSlot::Color, cb, 0),
                (VkInputSlot::Depth, db, 0),
                (VkInputSlot::Mv, mb, 0),
            ]);
        }
        // reactive 上传(staging 区段 0;自建 image,既有布局状态机)——零内容
        // 已驻留时跳过(layout 保持 SHADER_READ_ONLY_OPTIMAL,tag 面不变)。
        if !skip_reactive {
            self.vk_upload_image(VkInputSlot::Reactive, 0);
            self.reactive_zero_resident = p.reactive.is_none();
        }
        // color_out 置 GENERAL(DLSS UAV 写;frame_impl_ext 同律)。
        if self.color_out.layout == VK_IMAGE_LAYOUT_UNDEFINED {
            let out_res = self.color_out.clone_shallow();
            self.vk_barrier(&out_res, VK_IMAGE_LAYOUT_UNDEFINED, VK_IMAGE_LAYOUT_GENERAL, 0, VK_ACCESS_SHADER_WRITE, VK_PIPELINE_STAGE_TOP_OF_PIPE, VK_PIPELINE_STAGE_COMPUTE_SHADER);
            self.color_out.layout = VK_IMAGE_LAYOUT_GENERAL;
        }

        // ── ResourceTag 集(session 自建输入 image——frame_impl_ext 同一批
        // image/同一 usage 规则,tag 面逐字同构)──
        let mk_sl_res = |res: &VkImageRes, usage: u32| -> SlResource {
            SlResource {
                base: sl_base(SL_GUID_RESOURCE, 1),
                res_type: SL_RESOURCE_TEX2D,
                _pad0: [0; 7],
                native: res.image as usize as *mut c_void,
                memory: res.memory as usize as *mut c_void,
                view: res.view as usize as *mut c_void,
                state: res.layout as u32,
                width: res.w,
                height: res.h,
                native_format: res.format,
                mip_levels: 1,
                array_layers: 1,
                gpu_virtual_address: 0,
                flags: 0,
                usage,
                reserved: 0,
            }
        };
        let in_usage = VK_IMAGE_USAGE_SAMPLED | VK_IMAGE_USAGE_TRANSFER_DST;
        let sl_color_in = mk_sl_res(&self.color_in.clone_shallow(), in_usage);
        let sl_depth = mk_sl_res(&self.depth_in.clone_shallow(), in_usage);
        let sl_mv = mk_sl_res(&self.mv_in.clone_shallow(), in_usage);
        let sl_reactive = mk_sl_res(&self.reactive_in.clone_shallow(), in_usage);
        let sl_color_out = mk_sl_res(
            &self.color_out.clone_shallow(),
            VK_IMAGE_USAGE_STORAGE | VK_IMAGE_USAGE_TRANSFER_SRC,
        );
        let extent_in = SlExtent { top: 0, left: 0, width: self.in_w, height: self.in_h };
        let extent_out = SlExtent { top: 0, left: 0, width: self.out_w, height: self.out_h };
        let tags = [
            SlResourceTag { base: sl_base(SL_GUID_RESOURCE_TAG, 1), resource: &sl_color_in, tag_type: SL_BUFFER_SCALING_INPUT_COLOR, lifecycle: SL_LIFECYCLE_ONLY_VALID_NOW, extent: extent_in },
            SlResourceTag { base: sl_base(SL_GUID_RESOURCE_TAG, 1), resource: &sl_color_out, tag_type: SL_BUFFER_SCALING_OUTPUT_COLOR, lifecycle: SL_LIFECYCLE_ONLY_VALID_NOW, extent: extent_out },
            SlResourceTag { base: sl_base(SL_GUID_RESOURCE_TAG, 1), resource: &sl_depth, tag_type: SL_BUFFER_DEPTH, lifecycle: SL_LIFECYCLE_ONLY_VALID_NOW, extent: extent_in },
            SlResourceTag { base: sl_base(SL_GUID_RESOURCE_TAG, 1), resource: &sl_mv, tag_type: SL_BUFFER_MV, lifecycle: SL_LIFECYCLE_ONLY_VALID_NOW, extent: extent_in },
            SlResourceTag { base: sl_base(SL_GUID_RESOURCE_TAG, 1), resource: &sl_reactive, tag_type: SL_BUFFER_REACTIVE_MASK, lifecycle: SL_LIFECYCLE_ONLY_VALID_NOW, extent: extent_in },
        ];
        let tag_ptrs: [*const SlBaseStructure; 6] = [
            &self.viewport.base as *const _,
            &tags[0].base as *const _,
            &tags[1].base as *const _,
            &tags[2].base as *const _,
            &tags[3].base as *const _,
            &tags[4].base as *const _,
        ];
        let vtm_record = vtm_t0.elapsed();
        // SAFETY: tags/sl_* 栈上存活至 evaluate 返回;cmd 录制中;token 本帧有效。
        let r = unsafe {
            (self.fns.sl_evaluate_feature)(SL_FEATURE_DLSS, token, tag_ptrs.as_ptr(), 6, self.cmd)
        };
        if r != SL_OK {
            // SAFETY: cmd 录制中 → end;错误路径不提交。
            unsafe {
                let _ = (self.dev.end_command_buffer)(self.cmd);
                let _ = (self.dev.reset_command_buffer)(self.cmd, 0);
            }
            return Err(VendorError::VendorCall(format!(
                "slEvaluateFeature(DLSS buffers) → {}",
                sl_result_name(r)
            )));
        }
        let vtm_eval = vtm_t0.elapsed();
        // SAFETY: cmd 录制完成。
        let r = unsafe { (self.dev.end_command_buffer)(self.cmd) };
        if r != VK_SUCCESS {
            return Err(VendorError::ApiError(format!("vkEndCommandBuffer → {r}")));
        }
        let submit = VkSubmitInfo {
            s_type: VK_STRUCTURE_TYPE_SUBMIT_INFO,
            p_next: std::ptr::null(),
            wait_semaphore_count: 0,
            p_wait_semaphores: std::ptr::null(),
            p_wait_dst_stage_mask: std::ptr::null(),
            command_buffer_count: 1,
            p_command_buffers: &self.cmd,
            signal_semaphore_count: 0,
            p_signal_semaphores: std::ptr::null(),
        };
        // SAFETY: submit 栈上存活;cmd 录制完成态。
        let r = unsafe { (self.dev.queue_submit)(self.queue, 1, &submit, 0) };
        if r != VK_SUCCESS {
            return Err(VendorError::ApiError(format!("vkQueueSubmit → {r}")));
        }
        // SAFETY: queue 有效;waitIdle 后无在途写(CPU 顺序化同步纪律)。
        let r = unsafe { (self.dev.queue_wait_idle)(self.queue) };
        if r != VK_SUCCESS {
            return Err(VendorError::ApiError(format!("vkQueueWaitIdle → {r}")));
        }
        // SAFETY: cmd 已提交且 queue 排空。
        let _ = unsafe { (self.dev.reset_command_buffer)(self.cmd, 0) };
        if vtm_on {
            let ms = |d: std::time::Duration| d.as_secs_f64() * 1e3;
            eprintln!(
                "[vendor-timing dlss-buf] frame={} staging={:.3} sl_book={:.3} record={:.3} evaluate={:.3} submit_wait={:.3} total={:.3}ms",
                p.frame_index,
                ms(vtm_staging),
                ms(vtm_book - vtm_staging),
                ms(vtm_record - vtm_book),
                ms(vtm_eval - vtm_record),
                ms(vtm_t0.elapsed() - vtm_eval),
                ms(vtm_t0.elapsed()),
            );
        }
        Ok(())
    }

    /// G14.11:三输入 buffer→image 的**批量屏障**上传(逐 image 独立
    /// `vk_upload_image_src` 会在三条 copy 之间夹 4 道全局
    /// `vkCmdPipelineBarrier`,把本可并发的三条 copy 串成「copy→流水 drain→
    /// copy→drain→copy→drain」;本方法合并为 [3 barrier] → [3 copy] →
    /// [3 barrier],命令内容/copy region/最终 layout 与逐 image 路逐字同,
    /// 仅去掉中间 drain——**数据面零变化(digest 不变)**,GPU 段实测 t100 面
    /// 显著收窄)。调用方保证 cmd 录制中、三 src buffer 已 acquire。
    fn vk_upload_images_batched(&mut self, srcs: [(VkInputSlot, VkBuffer, u64); 3]) {
        let pick = |s: &Self, slot: VkInputSlot| -> (VkImage, u32, u32, u32, i32) {
            match slot {
                VkInputSlot::Color => (s.color_in.image, s.color_in.aspect, s.color_in.w, s.color_in.h, s.color_in.layout),
                VkInputSlot::Depth => (s.depth_in.image, s.depth_in.aspect, s.depth_in.w, s.depth_in.h, s.depth_in.layout),
                VkInputSlot::Mv => (s.mv_in.image, s.mv_in.aspect, s.mv_in.w, s.mv_in.h, s.mv_in.layout),
                VkInputSlot::Reactive => (s.reactive_in.image, s.reactive_in.aspect, s.reactive_in.w, s.reactive_in.h, s.reactive_in.layout),
            }
        };
        let mk_barrier = |image: VkImage, aspect: u32, old: i32, new: i32,
                          src_access: u32, dst_access: u32| VkImageMemoryBarrier {
            s_type: VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
            p_next: std::ptr::null(),
            src_access_mask: src_access,
            dst_access_mask: dst_access,
            old_layout: old,
            new_layout: new,
            src_queue_family_index: !0,
            dst_queue_family_index: !0,
            image,
            subresource_range: VkImageSubresourceRange {
                aspect_mask: aspect,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
        };
        let mut infos = Vec::with_capacity(3);
        for (slot, buf, off) in srcs {
            let (image, aspect, w, h, old) = pick(self, slot);
            infos.push((slot, buf, off, image, aspect, w, h, old));
        }
        // 首帧(UNDEFINED)与稳态(SHADER_READ_ONLY)混合时取保守 src scope:
        // 任一 UNDEFINED → TOP_OF_PIPE/0,否则 COMPUTE_SHADER/SHADER_READ。
        let any_first = infos.iter().any(|i| i.7 == VK_IMAGE_LAYOUT_UNDEFINED);
        let (src_stage, src_access) = if any_first {
            (VK_PIPELINE_STAGE_TOP_OF_PIPE, 0)
        } else {
            (VK_PIPELINE_STAGE_COMPUTE_SHADER, VK_ACCESS_SHADER_READ)
        };
        let pre: Vec<VkImageMemoryBarrier> = infos
            .iter()
            .map(|i| mk_barrier(i.3, i.4, i.7, VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                                src_access, VK_ACCESS_TRANSFER_WRITE))
            .collect();
        // SAFETY: cmd 录制中;barrier 数组栈上存活至调用返回;image 均有效。
        unsafe {
            (self.dev.cmd_pipeline_barrier)(
                self.cmd, src_stage, VK_PIPELINE_STAGE_TRANSFER, 0,
                0, std::ptr::null(), 0, std::ptr::null(),
                pre.len() as u32, pre.as_ptr(),
            )
        };
        for i in &infos {
            let region = VkBufferImageCopy {
                buffer_offset: i.2,
                buffer_row_length: 0,
                buffer_image_height: 0,
                image_subresource: VkImageSubresourceLayers {
                    aspect_mask: i.4,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                image_offset: [0, 0, 0],
                image_extent: VkExtent3D { width: i.5, height: i.6, depth: 1 },
            };
            // SAFETY: cmd 录制中;src buffer 已 acquire;region 与 image 尺寸一致。
            unsafe {
                (self.dev.cmd_copy_buffer_to_image)(
                    self.cmd, i.1, i.3, VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL, 1, &region,
                )
            };
        }
        let post: Vec<VkImageMemoryBarrier> = infos
            .iter()
            .map(|i| mk_barrier(i.3, i.4, VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                                VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
                                VK_ACCESS_TRANSFER_WRITE, VK_ACCESS_SHADER_READ))
            .collect();
        // SAFETY: 同上。
        unsafe {
            (self.dev.cmd_pipeline_barrier)(
                self.cmd, VK_PIPELINE_STAGE_TRANSFER, VK_PIPELINE_STAGE_COMPUTE_SHADER, 0,
                0, std::ptr::null(), 0, std::ptr::null(),
                post.len() as u32, post.as_ptr(),
            )
        };
        for i in &infos {
            match i.0 {
                VkInputSlot::Color => self.color_in.layout = VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
                VkInputSlot::Depth => self.depth_in.layout = VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
                VkInputSlot::Mv => self.mv_in.layout = VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
                VkInputSlot::Reactive => self.reactive_in.layout = VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
            }
        }
    }

    /// G14.10f:导入 buffer 的 EXTERNAL acquire(与 render_exec 帧末 buffer
    /// release 配对;dst = TRANSFER|TRANSFER_READ——本 cmd 内消费为 copy 源)。
    /// G14.11:三导入 buffer 的 EXTERNAL acquire 批量版(单次
    /// `vkCmdPipelineBarrier` 三 buffer barrier;与逐条版语义逐字同,去两道
    /// 冗余流水 drain)。
    fn vk_buffers_acquire_external_batched(&self, bufs: [(VkBuffer, u64); 3]) {
        let bs: Vec<VkBufferMemoryBarrier> = bufs
            .iter()
            .map(|&(buffer, size)| VkBufferMemoryBarrier {
                s_type: 44, // VK_STRUCTURE_TYPE_BUFFER_MEMORY_BARRIER
                p_next: std::ptr::null(),
                src_access_mask: 0,
                dst_access_mask: VK_ACCESS_TRANSFER_READ,
                src_queue_family_index: VK_QUEUE_FAMILY_EXTERNAL,
                dst_queue_family_index: self.queue_family,
                buffer,
                offset: 0,
                size,
            })
            .collect();
        // SAFETY: cmd 录制中;barrier 数组栈上存活至调用返回;buffer 均有效。
        unsafe {
            (self.dev.cmd_pipeline_barrier)(
                self.cmd,
                VK_PIPELINE_STAGE_TOP_OF_PIPE,
                VK_PIPELINE_STAGE_TRANSFER,
                0,
                0,
                std::ptr::null(),
                bs.len() as u32,
                bs.as_ptr().cast::<c_void>(),
                0,
                std::ptr::null(),
            )
        };
    }

    #[allow(dead_code)]
    fn vk_buffer_acquire_external(&self, buffer: VkBuffer, size: u64) {
        let barrier = VkBufferMemoryBarrier {
            s_type: 44, // VK_STRUCTURE_TYPE_BUFFER_MEMORY_BARRIER
            p_next: std::ptr::null(),
            src_access_mask: 0,
            dst_access_mask: VK_ACCESS_TRANSFER_READ,
            src_queue_family_index: VK_QUEUE_FAMILY_EXTERNAL,
            dst_queue_family_index: self.queue_family,
            buffer,
            offset: 0,
            size,
        };
        // SAFETY: cmd 录制中;barrier 栈上存活至调用返回;buffer 有效。
        unsafe {
            (self.dev.cmd_pipeline_barrier)(
                self.cmd,
                VK_PIPELINE_STAGE_TOP_OF_PIPE,
                VK_PIPELINE_STAGE_TRANSFER,
                0,
                0,
                std::ptr::null(),
                1,
                &barrier as *const VkBufferMemoryBarrier as *const c_void,
                0,
                std::ptr::null(),
            )
        };
    }

    /// G14.12:外部导入 image 的 EXTERNAL acquire **批量**版——单次
    /// `vkCmdPipelineBarrier` 承载 n 条 image barrier(逐条独立提交会串成 n 段
    /// pipeline flush;buffer 路的批量屏障同族收益已实测 evaluate 0.25→0.055ms)。
    fn vk_images_acquire_external_batched(&self, imgs: &[&VkImageRes]) {
        if imgs.is_empty() {
            return;
        }
        let bs: Vec<VkImageMemoryBarrier> = imgs
            .iter()
            .map(|res| VkImageMemoryBarrier {
                s_type: VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
                p_next: std::ptr::null(),
                src_access_mask: 0,
                dst_access_mask: VK_ACCESS_SHADER_READ,
                old_layout: VK_IMAGE_LAYOUT_GENERAL,
                new_layout: VK_IMAGE_LAYOUT_GENERAL,
                src_queue_family_index: VK_QUEUE_FAMILY_EXTERNAL,
                dst_queue_family_index: self.queue_family,
                image: res.image,
                subresource_range: VkImageSubresourceRange {
                    aspect_mask: res.aspect,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
            })
            .collect();
        // SAFETY: cmd 录制中;barrier 数组栈上存活至调用返回;image 均有效。
        unsafe {
            (self.dev.cmd_pipeline_barrier)(
                self.cmd,
                VK_PIPELINE_STAGE_TOP_OF_PIPE,
                VK_PIPELINE_STAGE_COMPUTE_SHADER,
                0,
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                bs.len() as u32,
                bs.as_ptr(),
            )
        };
    }

    /// acquire barrier(`VK_QUEUE_FAMILY_EXTERNAL`→本家族,GENERAL→GENERAL
    /// 零转换;dst = COMPUTE|SHADER_READ——NGX 内部 compute 采样)。
    fn vk_barrier_acquire_external(&self, res: &VkImageRes) {
        let barrier = VkImageMemoryBarrier {
            s_type: VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
            p_next: std::ptr::null(),
            src_access_mask: 0,
            dst_access_mask: VK_ACCESS_SHADER_READ,
            old_layout: VK_IMAGE_LAYOUT_GENERAL,
            new_layout: VK_IMAGE_LAYOUT_GENERAL,
            src_queue_family_index: VK_QUEUE_FAMILY_EXTERNAL,
            dst_queue_family_index: self.queue_family,
            image: res.image,
            subresource_range: VkImageSubresourceRange {
                aspect_mask: res.aspect,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
        };
        // SAFETY: cmd 录制中;barrier 栈上存活至调用返回;image 有效。
        unsafe {
            (self.dev.cmd_pipeline_barrier)(
                self.cmd,
                VK_PIPELINE_STAGE_TOP_OF_PIPE,
                VK_PIPELINE_STAGE_COMPUTE_SHADER,
                0,
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                1,
                &barrier,
            )
        };
    }
}

trait CloneShallow {
    fn clone_shallow(&self) -> Self;
}
impl CloneShallow for VkImageRes {
    fn clone_shallow(&self) -> Self {
        VkImageRes {
            image: self.image,
            memory: self.memory,
            view: self.view,
            layout: self.layout,
            format: self.format,
            w: self.w,
            h: self.h,
            aspect: self.aspect,
        }
    }
}

#[derive(Clone, Copy)]
enum VkInputSlot {
    Color,
    Depth,
    Mv,
    Reactive,
}

#[derive(Clone, Copy)]
enum D3dInputSlot {
    Color,
    Depth,
    Mv,
    Reactive,
}

impl Drop for DlssVkSession {
    fn drop(&mut self) {
        // 销毁序(SL 文档:slShutdown 必须先于 vk 对象销毁):queueWaitIdle →
        // slFreeResources → slShutdown → vk 逆序 → messenger → instance。
        if self.device.is_null() {
            return;
        }
        // SAFETY: device 有效;忽略错误(Drop 不 panic)。
        unsafe { (self.dev.device_wait_idle)(self.device) };
        if !self.shutdown_done {
            // SAFETY: viewport 有效。
            unsafe { (self.fns.sl_free_resources)(SL_FEATURE_DLSS, &self.viewport) };
            // SAFETY: SL 已初始化。
            unsafe { (self.fns.sl_shutdown)() };
            self.shutdown_done = true;
        }
        // SAFETY: 以下句柄均本 session 创建且存活;逆序销毁,无泄漏/双释放。
        unsafe {
            (self.dev.destroy_command_pool)(self.device, self.cmd_pool, std::ptr::null());
            // G14.10b 外部导入输入(view/image/导入 memory 归本 session;free
            // 导入 memory 仅解引用外部分配,导出侧不受影响)。
            for (res, _) in self.ext_inputs.iter().flatten() {
                (self.dev.destroy_image_view)(self.device, res.view, std::ptr::null());
                (self.dev.destroy_image)(self.device, res.image, std::ptr::null());
                (self.dev.free_memory)(self.device, res.memory, std::ptr::null());
            }
            // G14.10f 外部导入输入 buffer(同律:free 导入 memory 仅解引用)。
            for (buf, mem, _) in self.ext_input_bufs.iter().flatten() {
                (self.dev.destroy_buffer)(self.device, *buf, std::ptr::null());
                (self.dev.free_memory)(self.device, *mem, std::ptr::null());
            }
            for res in [&self.color_in, &self.depth_in, &self.mv_in, &self.reactive_in, &self.color_out] {
                (self.dev.destroy_image_view)(self.device, res.view, std::ptr::null());
                (self.dev.destroy_image)(self.device, res.image, std::ptr::null());
                (self.dev.free_memory)(self.device, res.memory, std::ptr::null());
            }
            (self.dev.destroy_buffer)(self.device, self.staging, std::ptr::null());
            (self.dev.free_memory)(self.device, self.staging_mem, std::ptr::null());
            (self.dev.destroy_buffer)(self.device, self.readback, std::ptr::null());
            (self.dev.free_memory)(self.device, self.readback_mem, std::ptr::null());
            (self.dev.destroy_device)(self.device, std::ptr::null());
            if self.messenger != 0 {
                let destroy_msgr: Option<FnVkDestroyDebugUtilsMessenger> =
                    cast_sym((self.fns.vk_gipa)(self.instance, c"vkDestroyDebugUtilsMessengerEXT".as_ptr()));
                if let Some(f) = destroy_msgr {
                    f(self.instance, self.messenger, std::ptr::null());
                }
            }
            let destroy_instance: Option<FnVkDestroyInstance> =
                cast_sym((self.fns.vk_gipa)(self.instance, c"vkDestroyInstance".as_ptr()));
            if let Some(f) = destroy_instance {
                f(self.instance, std::ptr::null());
            }
            self.device = std::ptr::null_mut();
            if !self.validation_counter.is_null() {
                drop(Box::from_raw(self.validation_counter));
                self.validation_counter = std::ptr::null_mut();
            }
        }
    }
}

/// sl::Constants 构造(静态固定相机 + 每帧 jitter/reset/exposure 语义;fixture 口径)。
fn build_sl_constants(input: &VendorFrameInput, iw: u32, ih: u32) -> SlConstants {
    let aspect = iw as f32 / ih as f32;
    let fov_y = 60.0f32.to_radians();
    let near = 0.1f32;
    let far = 100.0f32;
    let f = 1.0 / (fov_y / 2.0).tan();
    // ZO 透视(深度 [0,1],0=near;depthInverted=eFalse)。
    let view_to_clip = [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, far / (far - near), 1.0],
        [0.0, 0.0, -near * far / (far - near), 0.0],
    ];
    let inv_det = |m: &[[f32; 4]; 4]| -> [[f32; 4]; 4] {
        // 本矩阵的闭式逆(ZO 透视单参结构;fixture 固定参,无通用逆需求)。
        let _ = m;
        let finv = 1.0 / f;
        [
            [aspect * finv, 0.0, 0.0, 0.0],
            [0.0, finv, 0.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
            [0.0, 0.0, (near - far) / (near * far), 1.0 / (near * far) * far - (far / (far - near)) * (near - far) / (near * far) * 0.0 + 0.0],
        ]
    };
    let clip_to_view = inv_det(&view_to_clip);
    let identity = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
    SlConstants {
        base: sl_base(SL_GUID_CONSTANTS, 2),
        camera_view_to_clip: view_to_clip,
        clip_to_camera_view: clip_to_view,
        clip_to_lens_clip: identity,
        clip_to_prev_clip: identity, // 静态相机:clip→prevClip = I
        prev_clip_to_clip: identity,
        jitter_offset: input.jitter,
        mvec_scale: [1.0, 1.0], // mv 已为 uv 规范化([-1,1] 域),SL 归一化语义直通
        camera_pinhole_offset: [0.0, 0.0],
        camera_pos: [0.0, 0.0, 0.0],
        camera_up: [0.0, 1.0, 0.0],
        camera_right: [1.0, 0.0, 0.0],
        camera_fwd: [0.0, 0.0, 1.0],
        camera_near: near,
        camera_far: far,
        camera_fov: fov_y,
        camera_aspect_ratio: aspect,
        motion_vectors_invalid_value: SL_INVALID_FLOAT,
        depth_inverted: 0,
        camera_motion_included: 1,
        motion_vectors_3d: 0,
        reset: if input.reset { 1 } else { 0 },
        orthographic_projection: 0,
        motion_vectors_dilated: 0,
        motion_vectors_jittered: 0,
        _pad0: 0,
        min_relative_linear_depth_object_separation: 40.0,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 第 2 部:FSR 3.1.5 D3D12 臂(FidelityFX SDK 2.0.0 预编译签名 DLL)
// ═══════════════════════════════════════════════════════════════════════════

type FfxReturnCode = u32;
const FFX_OK: FfxReturnCode = 0;
const FFX_CREATE_DESC_TYPE_UPSCALE: u64 = 0x00010000;
const FFX_CREATE_DESC_TYPE_BACKEND_DX12: u64 = 0x2;
const FFX_DISPATCH_DESC_TYPE_UPSCALE: u64 = 0x00010001;
const FFX_QUERY_DESC_TYPE_GET_VERSIONS: u64 = 4;
const FFX_DESC_TYPE_OVERRIDE_VERSION: u64 = 5;
const FFX_QUERY_DESC_TYPE_GET_PROVIDER_VERSION: u64 = 6;
const FFX_UPSCALE_ENABLE_HIGH_DYNAMIC_RANGE: u32 = 1 << 0;
const FFX_SURFACE_FORMAT_R32G32B32A32_FLOAT: u32 = 2;
const FFX_SURFACE_FORMAT_R16G16B16A16_FLOAT: u32 = 3;
const FFX_SURFACE_FORMAT_R32G32_FLOAT: u32 = 5;
const FFX_SURFACE_FORMAT_R8_UNORM: u32 = 25;
const FFX_SURFACE_FORMAT_R32_FLOAT: u32 = 28;
const FFX_RESOURCE_TYPE_TEXTURE2D: u32 = 2;
const FFX_RESOURCE_USAGE_READ_ONLY: u32 = 0;
const FFX_RESOURCE_USAGE_UAV: u32 = 1 << 1;
/// `FFX_API_RESOURCE_STATE_COMMON`(G14.11 驻留共享输入声明态:资源经
/// SIMULTANEOUS_ACCESS decay 恒于 COMMON,ffx 内部录 COMMON→读态→COMMON
/// 显式转换,与 D3D12 实际状态逐帧恒真)。
const FFX_RESOURCE_STATE_COMMON: u32 = 1 << 0;
const FFX_RESOURCE_STATE_UNORDERED_ACCESS: u32 = 1 << 1;
const FFX_RESOURCE_STATE_COMPUTE_READ: u32 = 1 << 2;

#[repr(C)]
struct FfxApiHeader {
    desc_type: u64,
    p_next: *mut FfxApiHeader,
}
const _: () = assert!(size_of::<FfxApiHeader>() == 16);

#[repr(C)]
struct FfxDimensions2D {
    width: u32,
    height: u32,
}

#[repr(C)]
struct FfxFloatCoords2D {
    x: f32,
    y: f32,
}

/// ffxApiMessage = void(uint32_t type, const wchar_t* message)。
type FfxMessageCallback = unsafe extern "system" fn(u32, *const u16);

#[repr(C)]
struct FfxCreateContextDescUpscale {
    header: FfxApiHeader,
    flags: u32,
    max_render_size: FfxDimensions2D,
    max_upscale_size: FfxDimensions2D,
    fp_message: Option<FfxMessageCallback>,
}
const _: () = assert!(size_of::<FfxCreateContextDescUpscale>() == 48);

#[repr(C)]
struct FfxCreateBackendDx12Desc {
    header: FfxApiHeader,
    device: *mut c_void,
}
const _: () = assert!(size_of::<FfxCreateBackendDx12Desc>() == 24);

#[repr(C)]
#[derive(Clone, Copy)]
struct FfxResourceDescription {
    res_type: u32,
    format: u32,
    width: u32,
    height: u32,
    depth: u32,
    mip_count: u32,
    flags: u32,
    usage: u32,
}
const _: () = assert!(size_of::<FfxResourceDescription>() == 32);

#[repr(C)]
#[derive(Clone, Copy)]
struct FfxResource {
    resource: *mut c_void,
    description: FfxResourceDescription,
    state: u32,
}
const _: () = assert!(size_of::<FfxResource>() == 48);

#[repr(C)]
struct FfxDispatchDescUpscale {
    header: FfxApiHeader,
    command_list: *mut c_void,
    color: FfxResource,
    depth: FfxResource,
    motion_vectors: FfxResource,
    exposure: FfxResource,
    reactive: FfxResource,
    transparency_and_composition: FfxResource,
    output: FfxResource,
    jitter_offset: FfxFloatCoords2D,
    motion_vector_scale: FfxFloatCoords2D,
    render_size: FfxDimensions2D,
    upscale_size: FfxDimensions2D,
    enable_sharpening: u8,
    _pad0: [u8; 3],
    sharpness: f32,
    frame_time_delta: f32,
    pre_exposure: f32,
    reset: u8,
    _pad1: [u8; 3],
    camera_near: f32,
    camera_far: f32,
    camera_fov_angle_vertical: f32,
    view_space_to_meters_factor: f32,
    flags: u32,
}
const _: () = assert!(size_of::<FfxDispatchDescUpscale>() == 432);

#[repr(C)]
struct FfxQueryDescGetVersions {
    header: FfxApiHeader,
    create_desc_type: u64,
    device: *mut c_void,
    output_count: *mut u64,
    version_ids: *mut u64,
    version_names: *mut *const c_char,
}
const _: () = assert!(size_of::<FfxQueryDescGetVersions>() == 56);

#[repr(C)]
struct FfxOverrideVersion {
    header: FfxApiHeader,
    version_id: u64,
}
const _: () = assert!(size_of::<FfxOverrideVersion>() == 24);

#[repr(C)]
struct FfxQueryGetProviderVersion {
    header: FfxApiHeader,
    version_id: u64,
    version_name: *const c_char,
}
const _: () = assert!(size_of::<FfxQueryGetProviderVersion>() == 32);

type FnFfxCreateContext = unsafe extern "system" fn(*mut *mut c_void, *mut FfxApiHeader, *const c_void) -> FfxReturnCode;
type FnFfxDestroyContext = unsafe extern "system" fn(*mut *mut c_void, *const c_void) -> FfxReturnCode;
type FnFfxQuery = unsafe extern "system" fn(*mut *mut c_void, *mut FfxApiHeader) -> FfxReturnCode;
type FnFfxDispatch = unsafe extern "system" fn(*mut *mut c_void, *const FfxApiHeader) -> FfxReturnCode;

// ── D3D12/DXGI COM 最小面(WinSDK 10.0.26100 头核对 vtable 槽位) ──────────────
type Hresult = i32;
const S_OK: Hresult = 0;
const DXGI_ERROR_NOT_FOUND: Hresult = 0x887a_0002u32 as i32;

#[repr(C)]
#[derive(Clone, Copy)]
struct ComGuid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}
const fn com_guid(d1: u32, d2: u16, d3: u16, d4: [u8; 8]) -> ComGuid {
    ComGuid { data1: d1, data2: d2, data3: d3, data4: d4 }
}
const IID_ID3D12_DEVICE: ComGuid = com_guid(0x189819f1, 0x1db6, 0x4b57, [0xbe, 0x54, 0x18, 0x21, 0x33, 0x9b, 0x85, 0xf7]);
const IID_ID3D12_COMMAND_QUEUE: ComGuid = com_guid(0x0ec870a6, 0x5d7e, 0x4c22, [0x8c, 0xfc, 0x5b, 0xaa, 0xe0, 0x76, 0x16, 0xed]);
const IID_ID3D12_COMMAND_ALLOCATOR: ComGuid = com_guid(0x6102dee4, 0xaf59, 0x4b09, [0xb9, 0x99, 0xb4, 0x4d, 0x73, 0xf0, 0x9b, 0x24]);
const IID_ID3D12_GRAPHICS_COMMAND_LIST: ComGuid = com_guid(0x5b160d0f, 0xac1b, 0x4185, [0x8b, 0xa8, 0xb3, 0xae, 0x42, 0xa5, 0xa4, 0x55]);
const IID_ID3D12_FENCE: ComGuid = com_guid(0x0a753dcf, 0xc4d8, 0x4b91, [0xad, 0xf6, 0xbe, 0x5a, 0x60, 0xd9, 0x5a, 0x76]);
const IID_ID3D12_RESOURCE: ComGuid = com_guid(0x696442be, 0xa72e, 0x4059, [0xbc, 0x79, 0x5b, 0x5c, 0x98, 0x04, 0x0f, 0xad]);
const IID_ID3D12_DEBUG: ComGuid = com_guid(0x344488b7, 0x6846, 0x474b, [0xb9, 0x89, 0xf0, 0x27, 0x44, 0x82, 0x45, 0xe0]);
const IID_ID3D12_INFO_QUEUE: ComGuid = com_guid(0x0742a90b, 0xc387, 0x483f, [0xb9, 0x46, 0x30, 0xa7, 0xe4, 0xe6, 0x14, 0x58]);
const IID_IDXGI_FACTORY1: ComGuid = com_guid(0x770aae78, 0xf26f, 0x4dba, [0xa8, 0x29, 0x25, 0x3c, 0x83, 0xd1, 0xb3, 0x87]);

const D3D12_COMMAND_LIST_TYPE_DIRECT: i32 = 0;
const D3D12_HEAP_TYPE_DEFAULT: i32 = 1;
const D3D12_HEAP_TYPE_UPLOAD: i32 = 2;
const D3D12_HEAP_TYPE_READBACK: i32 = 3;
/// `D3D12_HEAP_FLAG_SHARED`(G14.11 反向共享:committed 资源可
/// `CreateSharedHandle` 导出 NT handle → Vulkan `D3D12_RESOURCE_BIT` 导入)。
const D3D12_HEAP_FLAG_SHARED: i32 = 0x1;
const D3D12_RESOURCE_DIMENSION_BUFFER: i32 = 1;
const D3D12_RESOURCE_DIMENSION_TEXTURE2D: i32 = 3;
const D3D12_TEXTURE_LAYOUT_ROW_MAJOR: i32 = 1;
const D3D12_TEXTURE_LAYOUT_UNKNOWN: i32 = 0;
const D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS: i32 = 0x4;
/// `D3D12_RESOURCE_FLAG_ALLOW_SIMULTANEOUS_ACCESS`(共享纹理跨 API 交替访问
/// 机核:ExecuteCommandLists 边界状态自动 decay 回 COMMON——Vulkan 写窗恒见
/// COMMON,D3D12 读窗经 implicit promotion/显式 COMMON before 转换,双侧状态
/// 机免协商;d3d12.h 头值 0x20)。
const D3D12_RESOURCE_FLAG_ALLOW_SIMULTANEOUS_ACCESS: i32 = 0x20;
const D3D12_RESOURCE_STATE_COMMON: i32 = 0;
const D3D12_RESOURCE_STATE_COPY_DEST: i32 = 0x400;
const D3D12_RESOURCE_STATE_COPY_SOURCE: i32 = 0x800;
const D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE: i32 = 0x40;
const D3D12_RESOURCE_STATE_UNORDERED_ACCESS: i32 = 0x8;
const D3D12_RESOURCE_BARRIER_TYPE_TRANSITION: i32 = 0;
const D3D12_RESOURCE_BARRIER_FLAG_NONE: i32 = 0;
/// G14.11 驻留共享输入格式(与 Vulkan 侧 pack 直写 Rgba32f/R32f/Rg32f 位面
/// 逐字对齐:RGBA32F=2 / RG32F=16 / R32F=41,dxgiformat.h 头值)。
const DXGI_FORMAT_R32G32B32A32_FLOAT: u32 = 2;
const DXGI_FORMAT_R16G16B16A16_FLOAT: u32 = 10;
const DXGI_FORMAT_R32G32_FLOAT: u32 = 16;
const DXGI_FORMAT_R32_FLOAT: u32 = 41;
const DXGI_FORMAT_R8_UNORM: u32 = 61;

#[repr(C)]
struct D3d12CommandQueueDesc {
    queue_type: i32,
    priority: i32,
    flags: i32,
    node_mask: u32,
}
const _: () = assert!(size_of::<D3d12CommandQueueDesc>() == 16);

#[repr(C)]
struct D3d12HeapProperties {
    heap_type: i32,
    cpu_page_property: i32,
    memory_pool_preference: i32,
    creation_node_mask: u32,
    visible_node_mask: u32,
}
const _: () = assert!(size_of::<D3d12HeapProperties>() == 20);

#[repr(C)]
struct D3d12ResourceDesc {
    dimension: i32,
    alignment: u64,
    width: u64,
    height: u32,
    depth_or_array_size: u16,
    mip_levels: u16,
    format: u32,
    sample_count: u32,
    sample_quality: u32,
    layout: i32,
    flags: i32,
}
const _: () = assert!(size_of::<D3d12ResourceDesc>() == 56);

#[repr(C)]
struct D3d12ResourceBarrier {
    barrier_type: i32,
    flags: i32,
    // Transition 变体(唯一消费变体)。
    p_resource: *mut c_void,
    subresource: u32,
    state_before: i32,
    state_after: i32,
    _pad: u32,
}
const _: () = assert!(size_of::<D3d12ResourceBarrier>() == 32);

#[repr(C)]
struct D3d12SubresourceFootprint {
    format: u32,
    width: u32,
    height: u32,
    depth: u32,
    row_pitch: u32,
}

#[repr(C)]
struct D3d12PlacedSubresourceFootprint {
    offset: u64,
    footprint: D3d12SubresourceFootprint,
}

#[repr(C)]
struct D3d12TextureCopyLocation {
    p_resource: *mut c_void,
    copy_type: i32,
    // 变体:SubresourceIndex(0)/PlacedFootprint(1)(d3d12.h D3D12_TEXTURE_COPY_TYPE)。
    placed: D3d12PlacedSubresourceFootprint,
}
const _: () = assert!(size_of::<D3d12TextureCopyLocation>() == 48);

/// COM vtable 槽位函数取址(单一 SAFETY 点)。
///
/// # Safety
/// `obj` 须为有效 COM 接口指针;`slot` 为该接口 vtable 内目标方法的声明序下标
/// (与 WinSDK 头逐槽核对);`T` 为匹配该方法 ABI 的函数指针类型。
unsafe fn com_fn<T: Copy>(obj: *mut c_void, slot: usize) -> T {
    // SAFETY: 调用方保证 obj 为有效 COM 对象且 slot/T 与目标方法 ABI 一致。
    unsafe {
        let vt = *(obj as *const *const *mut c_void);
        let raw = *vt.add(slot);
        debug_assert!(!raw.is_null());
        std::mem::transmute_copy::<*mut c_void, T>(&raw)
    }
}

fn com_release(obj: *mut c_void) {
    if obj.is_null() {
        return;
    }
    // SAFETY: obj 为本模块创建的有效 COM 对象;Release@2(IUnknown 恒定槽位)。
    unsafe {
        let f: unsafe extern "system" fn(*mut c_void) -> u32 = com_fn(obj, 2);
        f(obj);
    }
}

type FnD3d12CreateDevice = unsafe extern "system" fn(*mut c_void, u32, *const ComGuid, *mut *mut c_void) -> Hresult;
type FnD3d12GetDebugInterface = unsafe extern "system" fn(*const ComGuid, *mut *mut c_void) -> Hresult;
type FnCreateDxgiFactory1 = unsafe extern "system" fn(*const ComGuid, *mut *mut c_void) -> Hresult;

#[derive(Clone, Copy)]
struct D3dFns {
    create_device: FnD3d12CreateDevice,
    get_debug_interface: FnD3d12GetDebugInterface,
    create_factory: FnCreateDxgiFactory1,
}

struct D3dTexture {
    resource: *mut c_void,
    format: u32,
    w: u32,
    h: u32,
    state: i32,
}

/// FSR 3.1.5(DX12)session——safe 公共面。
pub struct FsrDx12Session {
    ffx_destroy: FnFfxDestroyContext,
    ffx_dispatch: FnFfxDispatch,
    device: *mut c_void,
    queue: *mut c_void,
    allocator: *mut c_void,
    cmd_list: *mut c_void,
    fence: *mut c_void,
    fence_value: u64,
    info_queue: *mut c_void,
    context: *mut c_void,
    in_w: u32,
    in_h: u32,
    out_w: u32,
    out_h: u32,
    color_in: D3dTexture,
    depth_in: D3dTexture,
    mv_in: D3dTexture,
    reactive_in: D3dTexture,
    color_out: D3dTexture,
    upload: *mut c_void,
    readback: *mut c_void,
    gpu_name: String,
    provider_version: String,
    available_versions: Vec<String>,
    fsr4_ml_available: bool,
    dlls: Vec<DllProvenance>,
    ffx_errors: u64,
    log_tail: Vec<String>,
    /// G14.3 性能波:四输入打包缓冲 session 常驻(原逐帧 4× `vec![0u8; …]`
    /// 新分配+清零 ≈ 19MB@1080p 输出档位——实测 pack 分项主构成;逐帧全量
    /// 重写,上传字节面逐位一致——digest 不变机核)。
    pack_color: Vec<u8>,
    pack_depth: Vec<u8>,
    pack_mv: Vec<u8>,
    pack_reac: Vec<u8>,
    /// G14.11:D3D12 adapter LUID(DXGI desc @296 实采;驻留面 LUID 对拍)。
    adapter_luid: [u8; 8],
    /// G14.11:驻留共享 staging buffer(资源指针 + NT handle;null/0 = 非驻留
    /// session。Drop 单点 Release + CloseHandle——导入方不得关闭)。
    staging_buf: *mut c_void,
    staging_handle: usize,
    /// staging 段布局(与 [`FsrSharedInputHandles`] 同源:color_row/depth_row/
    /// mv_row/off_depth/off_mv/size;dispatch_resident 的 CopyTextureRegion
    /// footprint 事实源)。
    staging_layout: [u64; 6],
}

static FFX_ERROR_COUNT: AtomicU64 = AtomicU64::new(0);

unsafe extern "system" fn ffx_message_cb(msg_type: u32, _message: *const u16) {
    // SAFETY: 进程级静态计数器;FFX 回调 ABI 对齐(u32 + wchar_t*)。
    if msg_type == 0 {
        // FFX_API_MESSAGE_TYPE_ERROR
        FFX_ERROR_COUNT.fetch_add(1, Ordering::Relaxed);
    }
}

impl FsrDx12Session {
    /// 创建 FSR 3.1.5 DX12 session(d3d12/dxgi 运行时装载 → NVIDIA 适配器 device →
    /// ffx backend+upscale context,FSR 3.1.5 版本显式 pin)。fail-closed。
    pub fn create(
        sdk_dir: &Path,
        in_size: (u32, u32),
        out_size: (u32, u32),
        validation: bool,
    ) -> Result<Self, VendorError> {
        Self::create_impl(sdk_dir, in_size, out_size, validation, false)
    }

    /// G14.11 驻留变体(buffer 共享形态,texture 直共享弃案见
    /// [`FsrSharedInputHandles`]):`D3D12_HEAP_FLAG_SHARED` committed BUFFER
    /// 建面并 `CreateSharedHandle` 导出(Vulkan D3D12_RESOURCE handle 导入
    /// bind 为 SSBO,pack kernel 按 host 链 upload 布局直写);三输入纹理与
    /// 现路径**逐字同**(f16 color/f32 depth/f32 RG mv,非共享);reactive 仍
    /// session 本地(创建期一次零上传,语义 = 现路径逐帧零上传);upload 堆缩
    /// 为 reactive 单段,host pack 缓冲三路归零(驻留面零 host 输入中转)。
    /// 输入内容契约:调用方(Vulkan 侧)写完并 fence 完成后才可
    /// [`Self::dispatch_resident`](内部先 CopyTextureRegion 搬入纹理)。
    pub fn create_resident(
        sdk_dir: &Path,
        in_size: (u32, u32),
        out_size: (u32, u32),
        validation: bool,
    ) -> Result<(Self, FsrSharedInputHandles), VendorError> {
        let mut s = Self::create_impl(sdk_dir, in_size, out_size, validation, true)?;
        // reactive 一次性零上传(现路径 reactive=None 臂 = 逐帧零上传;驻留面
        // 内容恒零,创建期一次落定,之后状态粘 NON_PIXEL_SHADER_RESOURCE)。
        let zeros = std::mem::take(&mut s.pack_reac);
        let up = s.d3d_upload_tex(D3dInputSlot::Reactive, &zeros, 1, 0);
        s.pack_reac = zeros;
        up?;
        s.d3d_submit_wait()?;
        let [color_row, depth_row, mv_row, off_depth, off_mv, size] = s.staging_layout;
        let handles = FsrSharedInputHandles {
            staging: s.staging_handle,
            staging_size: size,
            color_row,
            depth_row,
            mv_row,
            off_depth,
            off_mv,
            adapter_luid: s.adapter_luid,
        };
        Ok((s, handles))
    }

    fn create_impl(
        sdk_dir: &Path,
        in_size: (u32, u32),
        out_size: (u32, u32),
        validation: bool,
        resident: bool,
    ) -> Result<Self, VendorError> {
        let loader_dll = sdk_dir.join("signedbin").join("amd_fidelityfx_loader_dx12.dll");
        let upscaler_dll = sdk_dir.join("signedbin").join("amd_fidelityfx_upscaler_dx12.dll");
        for p in [&loader_dll, &upscaler_dll] {
            if !p.is_file() {
                return Err(VendorError::DllNotFound(format!("{} 不在位", p.display())));
            }
        }
        let dlls = [&loader_dll, &upscaler_dll]
            .iter()
            .map(|p| {
                let (sha, bytes) = sha256_file(p)?;
                Ok(DllProvenance {
                    name: p.file_name().unwrap_or_default().to_string_lossy().into_owned(),
                    sha256: sha,
                    bytes,
                })
            })
            .collect::<Result<Vec<_>, VendorError>>()?;
        // SAFETY(装载):DLL 路径实测在树;进程常驻不 FreeLibrary(U1 纪律)。
        let lib = loader::open(&loader_dll);
        if lib.is_null() {
            return Err(VendorError::DllNotFound(format!("loader 装载失败: {}", loader_dll.display())));
        }
        // 组件显式装载:loader 不自动从其自身目录装载 component(实测不装载 →
        // ffxQuery 返 FFX_API_RETURN_NO_PROVIDER=4);component DllMain 注册 provider
        // 到已驻 loader,必须显式 LoadLibrary(ALTERED_SEARCH_PATH 覆盖其侧载依赖)。
        // SAFETY(装载):upscaler_dll 路径实测在树;进程常驻不 FreeLibrary(U1 纪律)。
        let comp = loader::open(&upscaler_dll);
        if comp.is_null() {
            return Err(VendorError::DllNotFound(format!(
                "upscaler 组件装载失败: {}",
                upscaler_dll.display()
            )));
        }
        macro_rules! fsym {
            ($name:literal, $ty:ty) => {
                // SAFETY: lib 有效;$name NUL 结尾字面量;cast_sym null 校验。
                match unsafe { cast_sym::<$ty>(loader::sym(lib, concat!($name, "\0").as_ptr() as *const c_char)) } {
                    Some(f) => f,
                    None => return Err(VendorError::SymbolMissing($name.into())),
                }
            };
        }
        let ffx_create: FnFfxCreateContext = fsym!("ffxCreateContext", FnFfxCreateContext);
        let ffx_destroy: FnFfxDestroyContext = fsym!("ffxDestroyContext", FnFfxDestroyContext);
        let ffx_query: FnFfxQuery = fsym!("ffxQuery", FnFfxQuery);
        let ffx_dispatch: FnFfxDispatch = fsym!("ffxDispatch", FnFfxDispatch);

        // ── d3d12/dxgi 装载(系统目录稳定 ABI) ──
        let d3d12 = loader::open(Path::new("d3d12.dll"));
        let dxgi = loader::open(Path::new("dxgi.dll"));
        if d3d12.is_null() || dxgi.is_null() {
            return Err(VendorError::DllNotFound("d3d12.dll/dxgi.dll 装载失败".into()));
        }
        // SAFETY: d3d12/dxgi 模块句柄有效;符号名 NUL 结尾字面量;cast_sym null 校验。
        let d3d = unsafe {
            D3dFns {
                create_device: cast_sym(loader::sym(d3d12, c"D3D12CreateDevice".as_ptr()))
                    .ok_or_else(|| VendorError::SymbolMissing("D3D12CreateDevice".into()))?,
                get_debug_interface: cast_sym(loader::sym(d3d12, c"D3D12GetDebugInterface".as_ptr()))
                    .ok_or_else(|| VendorError::SymbolMissing("D3D12GetDebugInterface".into()))?,
                create_factory: cast_sym(loader::sym(dxgi, c"CreateDXGIFactory1".as_ptr()))
                    .ok_or_else(|| VendorError::SymbolMissing("CreateDXGIFactory1".into()))?,
            }
        };

        // ── debug layer(validation 语义;device 创建前启用) ──
        if validation {
            let mut debug: *mut c_void = std::ptr::null_mut();
            // SAFETY: IID_ID3D12_DEBUG 与头核对;debug 出参有效。
            let hr = unsafe { (d3d.get_debug_interface)(&IID_ID3D12_DEBUG, &mut debug) };
            eprintln!("[fsr-dbg] get_debug_interface → 0x{hr:08x} ptr={debug:p}");
            if hr == S_OK && !debug.is_null() {
                // SAFETY: ID3D12Debug::EnableDebugLayer @3。
                unsafe {
                    let f: unsafe extern "system" fn(*mut c_void) = com_fn(debug, 3);
                    f(debug);
                }
                com_release(debug);
            }
        }

        // ── 适配器(NVIDIA 优先) → device ──
        let mut factory: *mut c_void = std::ptr::null_mut();
        // SAFETY: IID_IDXGI_FACTORY1 与头核对。
        let hr = unsafe { (d3d.create_factory)(&IID_IDXGI_FACTORY1, &mut factory) };
        if hr != S_OK || factory.is_null() {
            return Err(VendorError::ApiError(format!("CreateDXGIFactory1 → 0x{hr:08x}")));
        }
        let mut adapter: *mut c_void = std::ptr::null_mut();
        let mut gpu_name = String::new();
        let mut vendor_id: u32 = 0;
        let mut adapter_luid = [0u8; 8];
        for i in 0..16u32 {
            let mut ad: *mut c_void = std::ptr::null_mut();
            // SAFETY: IDXGIFactory1::EnumAdapters1 @12。
            let hr = unsafe {
                let f: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> Hresult = com_fn(factory, 12);
                f(factory, i, &mut ad)
            };
            if hr == DXGI_ERROR_NOT_FOUND || ad.is_null() {
                break;
            }
            // DXGI_ADAPTER_DESC1:Description[128]u16@0,VendorId@256,...,
            // AdapterLuid@296(8B;G14.11 LUID 对拍源——免 GetAdapterLuid 的
            // 成员函数结构体返回 ABI 坑)。
            let mut desc = [0u8; 312];
            // SAFETY: IDXGIAdapter1::GetDesc1 @10;desc 312B = 结构实际尺寸。
            let hr = unsafe {
                let f: unsafe extern "system" fn(*mut c_void, *mut c_void) -> Hresult = com_fn(ad, 10);
                f(ad, desc.as_mut_ptr() as *mut c_void)
            };
            if hr == S_OK {
                // SAFETY: 只读 desc 已知前缀。
                let (vid, name, luid) = unsafe {
                    let vid = (desc.as_ptr().add(256) as *const u32).read_unaligned();
                    let wptr = desc.as_ptr() as *const u16;
                    let len = (0..128).position(|k| *wptr.add(k) == 0).unwrap_or(128);
                    let name = String::from_utf16_lossy(core::slice::from_raw_parts(wptr, len));
                    let mut luid = [0u8; 8];
                    luid.copy_from_slice(&desc[296..304]);
                    (vid, name, luid)
                };
                if vid == 0x10de {
                    adapter = ad;
                    gpu_name = name;
                    vendor_id = vid;
                    adapter_luid = luid;
                    break;
                }
                if adapter.is_null() {
                    adapter = ad;
                    gpu_name = name;
                    vendor_id = vid;
                    adapter_luid = luid;
                    continue;
                }
            }
            com_release(ad);
        }
        if adapter.is_null() {
            com_release(factory);
            return Err(VendorError::DeviceUnavailable("零 DXGI 适配器".into()));
        }
        let fsr4_ml_available = vendor_id == 0x1002; // FSR4 ML 需 AMD RDNA4
        let mut device: *mut c_void = std::ptr::null_mut();
        const D3D_FEATURE_LEVEL_12_0: u32 = 0xc000;
        // SAFETY: adapter 有效;IID_ID3D12_DEVICE 与头核对。
        let hr = unsafe { (d3d.create_device)(adapter, D3D_FEATURE_LEVEL_12_0, &IID_ID3D12_DEVICE, &mut device) };
        com_release(adapter);
        com_release(factory);
        if hr != S_OK || device.is_null() {
            return Err(VendorError::ApiError(format!("D3D12CreateDevice → 0x{hr:08x}")));
        }

        // info queue(validation 错误计数面)。
        let mut info_queue: *mut c_void = std::ptr::null_mut();
        if validation {
            // SAFETY: IUnknown::QueryInterface @0;IID_ID3D12_INFO_QUEUE 与头核对。
            unsafe {
                let f: unsafe extern "system" fn(*mut c_void, *const ComGuid, *mut *mut c_void) -> Hresult = com_fn(device, 0);
                let _ = f(device, &IID_ID3D12_INFO_QUEUE, &mut info_queue);
            }
        }

        // ── queue/allocator/list/fence ──
        let qdesc = D3d12CommandQueueDesc { queue_type: D3D12_COMMAND_LIST_TYPE_DIRECT, priority: 0, flags: 0, node_mask: 0 };
        let mut queue: *mut c_void = std::ptr::null_mut();
        // SAFETY: ID3D12Device::CreateCommandQueue @8;qdesc 栈上存活。
        let hr = unsafe {
            let f: unsafe extern "system" fn(*mut c_void, *const D3d12CommandQueueDesc, *const ComGuid, *mut *mut c_void) -> Hresult = com_fn(device, 8);
            f(device, &qdesc, &IID_ID3D12_COMMAND_QUEUE, &mut queue)
        };
        if hr != S_OK || queue.is_null() {
            return Err(VendorError::ApiError(format!("CreateCommandQueue → 0x{hr:08x}")));
        }
        let mut allocator: *mut c_void = std::ptr::null_mut();
        // SAFETY: CreateCommandAllocator @9。
        let hr = unsafe {
            let f: unsafe extern "system" fn(*mut c_void, i32, *const ComGuid, *mut *mut c_void) -> Hresult = com_fn(device, 9);
            f(device, D3D12_COMMAND_LIST_TYPE_DIRECT, &IID_ID3D12_COMMAND_ALLOCATOR, &mut allocator)
        };
        if hr != S_OK || allocator.is_null() {
            return Err(VendorError::ApiError(format!("CreateCommandAllocator → 0x{hr:08x}")));
        }
        let mut cmd_list: *mut c_void = std::ptr::null_mut();
        // SAFETY: CreateCommandList @12(nodeMask=0,DIRECT,allocator,无初始 PSO)。
        let hr = unsafe {
            let f: unsafe extern "system" fn(*mut c_void, u32, i32, *mut c_void, *mut c_void, *const ComGuid, *mut *mut c_void) -> Hresult = com_fn(device, 12);
            f(device, 0, D3D12_COMMAND_LIST_TYPE_DIRECT, allocator, std::ptr::null_mut(), &IID_ID3D12_GRAPHICS_COMMAND_LIST, &mut cmd_list)
        };
        if hr != S_OK || cmd_list.is_null() {
            return Err(VendorError::ApiError(format!("CreateCommandList → 0x{hr:08x}")));
        }
        let mut fence: *mut c_void = std::ptr::null_mut();
        // SAFETY: CreateFence @36(初值 0,NONE)。
        let hr = unsafe {
            let f: unsafe extern "system" fn(*mut c_void, u64, i32, *const ComGuid, *mut *mut c_void) -> Hresult = com_fn(device, 36);
            f(device, 0, 0, &IID_ID3D12_FENCE, &mut fence)
        };
        if hr != S_OK || fence.is_null() {
            return Err(VendorError::ApiError(format!("CreateFence → 0x{hr:08x}")));
        }

        let (iw, ih) = in_size;
        let (ow, oh) = out_size;
        let mk_tex = |format: u32, w: u32, h: u32, flags: i32, heap_flags: i32| -> Result<D3dTexture, VendorError> {
            let heap = D3d12HeapProperties {
                heap_type: D3D12_HEAP_TYPE_DEFAULT,
                cpu_page_property: 0,
                memory_pool_preference: 0,
                creation_node_mask: 1,
                visible_node_mask: 1,
            };
            let desc = D3d12ResourceDesc {
                dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
                alignment: 0,
                width: w as u64,
                height: h,
                depth_or_array_size: 1,
                mip_levels: 1,
                format,
                sample_count: 1,
                sample_quality: 0,
                layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
                flags,
            };
            let mut res: *mut c_void = std::ptr::null_mut();
            // SAFETY: CreateCommittedResource @27;heap/desc 栈上存活。
            let hr = unsafe {
                let f: unsafe extern "system" fn(*mut c_void, *const D3d12HeapProperties, i32, *const D3d12ResourceDesc, i32, *const c_void, *const ComGuid, *mut *mut c_void) -> Hresult = com_fn(device, 27);
                f(device, &heap, heap_flags, &desc, D3D12_RESOURCE_STATE_COMMON, std::ptr::null(), &IID_ID3D12_RESOURCE, &mut res)
            };
            if hr != S_OK || res.is_null() {
                return Err(VendorError::ApiError(format!("CreateCommittedResource(fmt={format},flags=0x{flags:x},heap=0x{heap_flags:x}) → 0x{hr:08x}")));
            }
            Ok(D3dTexture { resource: res, format, w, h, state: D3D12_RESOURCE_STATE_COMMON })
        };
        // 三输入纹理:驻留/现路径**逐字同形态**(f16 color/f32 depth/f32 RG
        // mv,非共享——G14.11 buffer 共享形态下纹理内容由 CopyTextureRegion
        // 从 shared staging buffer 搬入,texture 直共享弃案见
        // FsrSharedInputHandles)。
        let (color_in, depth_in, mv_in) = (
            mk_tex(DXGI_FORMAT_R16G16B16A16_FLOAT, iw, ih, 0, 0)?,
            mk_tex(DXGI_FORMAT_R32_FLOAT, iw, ih, 0, 0)?,
            mk_tex(DXGI_FORMAT_R32G32_FLOAT, iw, ih, 0, 0)?,
        );
        let reactive_in = mk_tex(DXGI_FORMAT_R8_UNORM, iw, ih, 0, 0)?;
        let color_out = mk_tex(DXGI_FORMAT_R16G16B16A16_FLOAT, ow, oh, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS, 0)?;
        // G14.11 驻留:shared staging BUFFER(DEFAULT heap + SHARED;布局 =
        // host 链 upload 三段同律:color f16 @align256(8·iw) / depth f32
        // @align256(4·iw) / mv f32 RG @align256(8·iw);总长 64KB 对齐)+
        // CreateSharedHandle @31 导出 NT handle(GENERIC_ALL,匿名;Drop 单点
        // CloseHandle)。
        let row256 = |bytes_per_px: u64, w: u32| -> u64 { (bytes_per_px * w as u64 + 255) & !255 };
        let mut staging_buf: *mut c_void = std::ptr::null_mut();
        let mut staging_handle = 0usize;
        let mut staging_layout = [0u64; 6];
        if resident {
            let color_row = row256(8, iw);
            let depth_row = row256(4, iw);
            let mv_row = row256(8, iw);
            let off_depth = color_row * ih as u64;
            let off_mv = off_depth + depth_row * ih as u64;
            let size = (off_mv + mv_row * ih as u64 + 0xFFFF) & !0xFFFF;
            staging_layout = [color_row, depth_row, mv_row, off_depth, off_mv, size];
            let heap = D3d12HeapProperties {
                heap_type: D3D12_HEAP_TYPE_DEFAULT,
                cpu_page_property: 0,
                memory_pool_preference: 0,
                creation_node_mask: 1,
                visible_node_mask: 1,
            };
            let desc = D3d12ResourceDesc {
                dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
                alignment: 0,
                width: size,
                height: 1,
                depth_or_array_size: 1,
                mip_levels: 1,
                format: 0, // UNKNOWN
                sample_count: 1,
                sample_quality: 0,
                layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
                flags: 0,
            };
            // SAFETY: CreateCommittedResource @27;SHARED heap 初态 COMMON
            // (buffer 状态隐式提升,copy source 免显式 barrier)。
            let hr = unsafe {
                let f: unsafe extern "system" fn(*mut c_void, *const D3d12HeapProperties, i32, *const D3d12ResourceDesc, i32, *const c_void, *const ComGuid, *mut *mut c_void) -> Hresult = com_fn(device, 27);
                f(device, &heap, D3D12_HEAP_FLAG_SHARED, &desc, D3D12_RESOURCE_STATE_COMMON, std::ptr::null(), &IID_ID3D12_RESOURCE, &mut staging_buf)
            };
            if hr != S_OK || staging_buf.is_null() {
                return Err(VendorError::ApiError(format!(
                    "CreateCommittedResource(shared staging buffer,{size}B) → 0x{hr:08x}"
                )));
            }
            const GENERIC_ALL: u32 = 0x1000_0000;
            let mut handle: *mut c_void = std::ptr::null_mut();
            // SAFETY: ID3D12Device::CreateSharedHandle @31(WinSDK 头核对);
            // resource 为 SHARED heap committed 资源;出参栈上有效。
            let hr = unsafe {
                let f: unsafe extern "system" fn(*mut c_void, *mut c_void, *const c_void, u32, *const u16, *mut *mut c_void) -> Hresult = com_fn(device, 31);
                f(device, staging_buf, std::ptr::null(), GENERIC_ALL, std::ptr::null(), &mut handle)
            };
            if hr != S_OK || handle.is_null() {
                return Err(VendorError::ApiError(format!(
                    "CreateSharedHandle(staging buffer) → 0x{hr:08x}"
                )));
            }
            staging_handle = handle as usize;
        }

        // upload/readback 缓冲(ROW_MAJOR;行距 256 对齐由 footprint 规划;
        // 驻留面 upload 缩为 reactive 单段——三输入零 host 上传)。
        let row = |bytes_per_px: u64, w: u32| -> u64 { (bytes_per_px * w as u64 + 255) & !255 };
        let upload_size = if resident {
            row(1, iw) * ih as u64
        } else {
            row(8, iw) * ih as u64
                + row(4, iw) * ih as u64
                + row(8, iw) * ih as u64
                + row(1, iw) * ih as u64
        };
        let readback_size = row(8, ow) * oh as u64;
        let mk_heap_buffer = |size: u64, heap_type: i32| -> Result<*mut c_void, VendorError> {
            let heap = D3d12HeapProperties {
                heap_type,
                cpu_page_property: 0,
                memory_pool_preference: 0,
                creation_node_mask: 1,
                visible_node_mask: 1,
            };
            let desc = D3d12ResourceDesc {
                dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
                alignment: 0,
                width: size,
                height: 1,
                depth_or_array_size: 1,
                mip_levels: 1,
                format: 0, // UNKNOWN
                sample_count: 1,
                sample_quality: 0,
                layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
                flags: 0,
            };
            let mut res: *mut c_void = std::ptr::null_mut();
            let init = if heap_type == D3D12_HEAP_TYPE_UPLOAD { 0x1 | 0x2 | 0x40 | 0x80 | 0x200 } else { D3D12_RESOURCE_STATE_COPY_DEST };
            // SAFETY: CreateCommittedResource @27;buffer 初态 UPLOAD 必须 GENERIC_READ(=COMMON 兼容位);
            // READBACK 必须 COPY_DEST。
            let hr = unsafe {
                let f: unsafe extern "system" fn(*mut c_void, *const D3d12HeapProperties, i32, *const D3d12ResourceDesc, i32, *const c_void, *const ComGuid, *mut *mut c_void) -> Hresult = com_fn(device, 27);
                f(device, &heap, 0, &desc, init, std::ptr::null(), &IID_ID3D12_RESOURCE, &mut res)
            };
            if hr != S_OK || res.is_null() {
                return Err(VendorError::ApiError(format!("CreateCommittedResource(buffer) → 0x{hr:08x}")));
            }
            Ok(res)
        };
        let upload = mk_heap_buffer(upload_size, D3D12_HEAP_TYPE_UPLOAD)?;
        let readback = mk_heap_buffer(readback_size, D3D12_HEAP_TYPE_READBACK)?;

        // ── ffx 版本查询 + FSR 3.1.5 pin ──
        eprintln!("[fsr-dbg] pre-query device={:p}", device);
        let mut version_count: u64 = 0;
        let mut qv = FfxQueryDescGetVersions {
            header: FfxApiHeader { desc_type: FFX_QUERY_DESC_TYPE_GET_VERSIONS, p_next: std::ptr::null_mut() },
            create_desc_type: FFX_CREATE_DESC_TYPE_UPSCALE,
            device,
            output_count: &mut version_count,
            version_ids: std::ptr::null_mut(),
            version_names: std::ptr::null_mut(),
        };
        // SAFETY: qv 栈上存活;device 有效。
        let rc = unsafe { ffx_query(std::ptr::null_mut(), &mut qv.header) };
        eprintln!("[fsr-dbg] query count rc={rc} count={version_count}");
        if rc != FFX_OK {
            return Err(VendorError::VendorCall(format!("ffxQuery(GetVersions,count) → {rc}")));
        }
        let mut version_ids = vec![0u64; version_count as usize];
        let mut version_names_ptrs = vec![std::ptr::null::<c_char>(); version_count as usize];
        qv.version_ids = version_ids.as_mut_ptr();
        qv.version_names = version_names_ptrs.as_mut_ptr();
        // SAFETY: 数组容量 = version_count。
        let rc = unsafe { ffx_query(std::ptr::null_mut(), &mut qv.header) };
        eprintln!("[fsr-dbg] query list rc={rc}");
        if rc != FFX_OK {
            return Err(VendorError::VendorCall(format!("ffxQuery(GetVersions,list) → {rc}")));
        }
        let mut available_versions = Vec::new();
        let mut fsr31_id: Option<u64> = None;
        for (k, &name_ptr) in version_names_ptrs.iter().enumerate() {
            if name_ptr.is_null() {
                continue;
            }
            // SAFETY: ffx 返回 NUL 结尾静态字符串(查询期有效;立即拷贝)。
            let name = unsafe {
                let mut len = 0usize;
                while *name_ptr.add(len) != 0 {
                    len += 1;
                }
                String::from_utf8_lossy(core::slice::from_raw_parts(name_ptr as *const u8, len)).into_owned()
            };
            if name.contains("3.1") {
                fsr31_id = Some(version_ids[k]);
            }
            available_versions.push(name);
        }

        // ── ffx upscale context(backend DX12 链 + 版本 override pin 3.1.5) ──
        let mut context: *mut c_void = std::ptr::null_mut();
        let mut override_v = FfxOverrideVersion {
            header: FfxApiHeader { desc_type: FFX_DESC_TYPE_OVERRIDE_VERSION, p_next: std::ptr::null_mut() },
            version_id: fsr31_id.unwrap_or(0),
        };
        // 链:upscale → backend → override(版本显式 pin 3.1.5;无 3.1 id 时 backend 链尾)。
        let backend_pnext = if fsr31_id.is_some() {
            &mut override_v.header as *mut FfxApiHeader
        } else {
            std::ptr::null_mut()
        };
        let mut backend = FfxCreateBackendDx12Desc {
            header: FfxApiHeader { desc_type: FFX_CREATE_DESC_TYPE_BACKEND_DX12, p_next: backend_pnext },
            device,
        };
        let mut create = FfxCreateContextDescUpscale {
            header: FfxApiHeader { desc_type: FFX_CREATE_DESC_TYPE_UPSCALE, p_next: &mut backend.header },
            flags: FFX_UPSCALE_ENABLE_HIGH_DYNAMIC_RANGE,
            max_render_size: FfxDimensions2D { width: iw, height: ih },
            max_upscale_size: FfxDimensions2D { width: ow, height: oh },
            fp_message: Some(ffx_message_cb),
        };
        // 存活纪律:create/backend/override 栈上存活至 ffxCreateContext 返回;ffx
        // 文档明示 desc 指针须存活至 ffxDestroyContext——本链字段全为值语义(无悬垂
        // 引用),满足「指针内容在调用期有效」的实质约束。
        eprintln!("[fsr-dbg] pre-create fsr31_id={fsr31_id:?} versions={available_versions:?}");
        // SAFETY: create/backend/override 栈上存活至本调用返回;链字段全值语义无悬垂。
        let rc = unsafe { ffx_create(&mut context, &mut create.header, std::ptr::null()) };
        eprintln!("[fsr-dbg] create rc={rc} context={context:p}");
        if rc != FFX_OK || context.is_null() {
            return Err(VendorError::VendorCall(format!("ffxCreateContext(upscale) → {rc}")));
        }
        let mut provider = FfxQueryGetProviderVersion {
            header: FfxApiHeader { desc_type: FFX_QUERY_DESC_TYPE_GET_PROVIDER_VERSION, p_next: std::ptr::null_mut() },
            version_id: 0,
            version_name: std::ptr::null(),
        };
        // SAFETY: context 有效(&mut context 传 ffxContext* 语义);provider 栈上存活。
        let rc = unsafe { ffx_query(&mut context, &mut provider.header) };
        let provider_version = if rc == FFX_OK && !provider.version_name.is_null() {
            // SAFETY: ffx 返回静态 NUL 结尾字符串;立即拷贝。
            unsafe {
                let mut len = 0usize;
                while *provider.version_name.add(len) != 0 {
                    len += 1;
                }
                String::from_utf8_lossy(core::slice::from_raw_parts(provider.version_name as *const u8, len)).into_owned()
            }
        } else {
            String::new()
        };

        Ok(FsrDx12Session {
            ffx_destroy,
            ffx_dispatch,
            device,
            queue,
            allocator,
            cmd_list,
            fence,
            fence_value: 0,
            info_queue,
            context,
            in_w: iw,
            in_h: ih,
            out_w: ow,
            out_h: oh,
            color_in,
            depth_in,
            mv_in,
            reactive_in,
            color_out,
            upload,
            readback,
            // 驻留面 host pack 三路归零(零 host 输入中转;reactive 段保留——
            // create_resident 创建期一次零上传消费)。
            pack_color: if resident { Vec::new() } else { vec![0u8; (iw * ih) as usize * 8] },
            pack_depth: if resident { Vec::new() } else { vec![0u8; (iw * ih) as usize * 4] },
            pack_mv: if resident { Vec::new() } else { vec![0u8; (iw * ih) as usize * 8] },
            pack_reac: vec![0u8; (iw * ih) as usize],
            gpu_name,
            provider_version,
            available_versions,
            fsr4_ml_available,
            dlls,
            ffx_errors: 0,
            log_tail: Vec::new(),
            adapter_luid,
            staging_buf,
            staging_handle,
            staging_layout,
        })
    }

    fn d3d_barrier(&self, tex: &D3dTexture, before: i32, after: i32) {
        let b = D3d12ResourceBarrier {
            barrier_type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
            flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
            p_resource: tex.resource,
            subresource: 0xffff_ffff, // ALL_SUBRESOURCES
            state_before: before,
            state_after: after,
            _pad: 0,
        };
        // SAFETY: cmd_list 录制中;ID3D12GraphicsCommandList::ResourceBarrier @26。
        unsafe {
            let f: unsafe extern "system" fn(*mut c_void, u32, *const D3d12ResourceBarrier) = com_fn(self.cmd_list, 26);
            f(self.cmd_list, 1, &b);
        }
    }

    fn d3d_upload_tex(&mut self, slot: D3dInputSlot, data: &[u8], bytes_per_px: u64, upload_offset: u64) -> Result<(), VendorError> {
        let tex = match slot {
            D3dInputSlot::Color => self.color_in.clone_shallow(),
            D3dInputSlot::Depth => self.depth_in.clone_shallow(),
            D3dInputSlot::Mv => self.mv_in.clone_shallow(),
            D3dInputSlot::Reactive => self.reactive_in.clone_shallow(),
        };
        let row_pitch = (bytes_per_px * tex.w as u64 + 255) & !255;
        // SAFETY: upload heap 常驻 mapped 区间写入(upload heap Map 免 Unmap 纪律);
        // 行主序按 row_pitch 打包。
        unsafe {
            let mut ptr: *mut c_void = std::ptr::null_mut();
            let f: unsafe extern "system" fn(*mut c_void, u32, *const c_void, *mut *mut c_void) -> Hresult = com_fn(self.upload, 8);
            let hr = f(self.upload, 0, std::ptr::null(), &mut ptr);
            if hr != S_OK || ptr.is_null() {
                return Err(VendorError::ApiError(format!("upload heap Map → 0x{hr:08x}")));
            }
            let dst = (ptr as *mut u8).add(upload_offset as usize);
            for y in 0..tex.h as usize {
                let src_row = data.as_ptr().add(y * bytes_per_px as usize * tex.w as usize);
                let dst_row = dst.add(y * row_pitch as usize);
                std::ptr::copy_nonoverlapping(src_row, dst_row, bytes_per_px as usize * tex.w as usize);
            }
            let unmap: unsafe extern "system" fn(*mut c_void, u32, *const c_void) = com_fn(self.upload, 9);
            unmap(self.upload, 0, std::ptr::null());
        }
        let before = if tex.state == D3D12_RESOURCE_STATE_COMMON {
            D3D12_RESOURCE_STATE_COMMON
        } else {
            D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE
        };
        self.d3d_barrier(&tex, before, D3D12_RESOURCE_STATE_COPY_DEST);
        let dst_loc = D3d12TextureCopyLocation {
            p_resource: tex.resource,
            copy_type: 0, // SUBRESOURCE_INDEX(d3d12.h:D3D12_TEXTURE_COPY_TYPE 枚举 0-based)
            placed: D3d12PlacedSubresourceFootprint {
                offset: 0,
                footprint: D3d12SubresourceFootprint { format: tex.format, width: 0, height: 0, depth: 0, row_pitch: 0 },
            },
        };
        let src_loc = D3d12TextureCopyLocation {
            p_resource: self.upload,
            copy_type: 1, // PLACED_FOOTPRINT(枚举值 1)
            placed: D3d12PlacedSubresourceFootprint {
                offset: upload_offset,
                footprint: D3d12SubresourceFootprint {
                    format: tex.format,
                    width: tex.w,
                    height: tex.h,
                    depth: 1,
                    row_pitch: row_pitch as u32,
                },
            },
        };
        // SAFETY: CopyTextureRegion @16;dst/src loc 栈上存活。
        unsafe {
            let f: unsafe extern "system" fn(*mut c_void, *const D3d12TextureCopyLocation, u32, u32, u32, *const D3d12TextureCopyLocation, *const c_void) = com_fn(self.cmd_list, 16);
            f(self.cmd_list, &dst_loc, 0, 0, 0, &src_loc, std::ptr::null());
        }
        self.d3d_barrier(&tex, D3D12_RESOURCE_STATE_COPY_DEST, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
        let new_state = D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE;
        match slot {
            D3dInputSlot::Color => self.color_in.state = new_state,
            D3dInputSlot::Depth => self.depth_in.state = new_state,
            D3dInputSlot::Mv => self.mv_in.state = new_state,
            D3dInputSlot::Reactive => self.reactive_in.state = new_state,
        }
        Ok(())
    }

    fn d3d_submit_wait(&mut self) -> Result<(), VendorError> {
        // SAFETY: ID3D12GraphicsCommandList::Close @9。
        unsafe {
            let f: unsafe extern "system" fn(*mut c_void) -> Hresult = com_fn(self.cmd_list, 9);
            let hr = f(self.cmd_list);
            if hr != S_OK {
                return Err(VendorError::ApiError(format!("cmdlist Close → 0x{hr:08x}")));
            }
        }
        // SAFETY: ID3D12CommandQueue::ExecuteCommandLists @10。
        unsafe {
            let lists = [self.cmd_list];
            let f: unsafe extern "system" fn(*mut c_void, u32, *const *mut c_void) = com_fn(self.queue, 10);
            f(self.queue, 1, lists.as_ptr());
        }
        self.fence_value += 1;
        // SAFETY: ID3D12CommandQueue::Signal @14。
        unsafe {
            let f: unsafe extern "system" fn(*mut c_void, *mut c_void, u64) -> Hresult = com_fn(self.queue, 14);
            let hr = f(self.queue, self.fence, self.fence_value);
            if hr != S_OK {
                return Err(VendorError::ApiError(format!("queue Signal → 0x{hr:08x}")));
            }
        }
        // SAFETY: ID3D12Fence::SetEventOnCompletion @9(HANDLE=NULL → 忙等降级?
        // 否——NULL 事件语义非法;用 GetCompletedValue 轮询,有界)。
        unsafe {
            let get: unsafe extern "system" fn(*mut c_void) -> u64 = com_fn(self.fence, 8);
            let mut spins = 0u64;
            loop {
                if get(self.fence) >= self.fence_value {
                    break;
                }
                spins += 1;
                if spins > 200_000_000 {
                    return Err(VendorError::ApiError("fence 等待超界(疑似 TDR)".into()));
                }
                std::hint::spin_loop();
                if spins.is_multiple_of(1_000_000) {
                    std::thread::yield_now();
                }
            }
        }
        // SAFETY: 下一帧重录:ID3D12CommandAllocator::Reset @8(IUnknown0-2 +
        // ID3D12Object3-6 + ID3D12DeviceChild::GetDevice@7 之后首个自有方法;d3d12.h
        // L5193 核对)+ ID3D12GraphicsCommandList::Reset @10。
        unsafe {
            let f: unsafe extern "system" fn(*mut c_void) -> Hresult = com_fn(self.allocator, 8);
            let hr = f(self.allocator);
            if hr != S_OK {
                return Err(VendorError::ApiError(format!("allocator Reset → 0x{hr:08x}")));
            }
            let f: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut c_void) -> Hresult = com_fn(self.cmd_list, 10);
            let hr = f(self.cmd_list, self.allocator, std::ptr::null_mut());
            if hr != S_OK {
                return Err(VendorError::ApiError(format!("cmdlist Reset → 0x{hr:08x}")));
            }
        }
        Ok(())
    }

    /// 执行一帧 FSR 3.1.5 超分;返回 `out_size` 的 3 通道 f32 显示域图像。
    pub fn upscale(&mut self, input: &VendorFrameInput) -> Result<Vec<f32>, VendorError> {
        let mut out = vec![0f32; (self.out_w * self.out_h * 3) as usize];
        self.frame_impl_into(input, &mut out)?;
        Ok(out)
    }

    /// G14.6 Stage A：驻留输出变体（与 `upscale` 逐位一致——同一 frame_impl_into
    /// 主体，调用方驻留 Vec 消逐帧 ~out_px·12B 分配+清零；pack 直写上传堆面归
    /// G14.x wave 2 登记，本波不动 FSR pack/upload 路径）。
    pub fn upscale_into(&mut self, input: &VendorFrameInput, dst: &mut Vec<f32>) -> Result<(), VendorError> {
        let need = (self.out_w * self.out_h * 3) as usize;
        if dst.len() != need {
            dst.resize(need, 0.0);
        }
        self.frame_impl_into(input, dst)
    }

    fn frame_impl_into(&mut self, input: &VendorFrameInput, out_px: &mut [f32]) -> Result<(), VendorError> {
        let (iw, ih) = (self.in_w, self.in_h);
        let px = (iw * ih) as usize;
        if input.color.len() != px * 3 || input.depth.len() != px || input.mv.len() != px * 2 {
            return Err(VendorError::ApiError("输入切片长度与 session 分辨率不符".into()));
        }
        if out_px.len() != (self.out_w * self.out_h * 3) as usize {
            return Err(VendorError::ApiError("输出切片长度与 session 输出分辨率不符".into()));
        }
        // G14.3 性能波:内部分解遥测(env `RURIX_VENDOR_TIMING=1` 门控,默认关,
        // 零行为变更;轴同 DLSS 臂——pack/upload/evaluate/submit_wait/readback)。
        let vtm_on = std::env::var("RURIX_VENDOR_TIMING").ok().as_deref() == Some("1");
        let vtm_t0 = std::time::Instant::now();
        // 打包四输入(color f16 RGBA / depth f32 / mv f32 RG / reactive R8;
        // reactive 行距恒 = iw(R8 逐像素直排,无 256 对齐段——off_reac 仅作段偏移)。
        let color_row = (8u64 * iw as u64 + 255) & !255;
        let depth_row = (4u64 * iw as u64 + 255) & !255;
        let mv_row = (8u64 * iw as u64 + 255) & !255;
        let off_depth = color_row * ih as u64;
        let off_mv = off_depth + depth_row * ih as u64;
        let off_reac = off_mv + mv_row * ih as u64;
        // G14.3 性能波:session 常驻打包缓冲 take/复用(消逐帧 4 次新分配+清零)
        // + chunks_exact 定长块写;上传字节面与逐帧新分配逐位一致(reactive
        // None 臂 fill(0) ≡ 新零 vec 字面)。take/restore 全路径配对(上传错误
        // 经 up_res 缓传,恢复后再 `?`)。
        // G14.7:四缓冲像素带并行(pack_vendor_inputs 共用主体;四缓冲精确贴合
        // px*8/px*4/px*8/px,带内同式同序,字节面与 G14.3 面逐位一致)。
        let mut color_px = std::mem::take(&mut self.pack_color);
        let mut depth_px = std::mem::take(&mut self.pack_depth);
        let mut mv_px = std::mem::take(&mut self.pack_mv);
        let mut reac_px = std::mem::take(&mut self.pack_reac);
        pack_vendor_inputs(
            px,
            input.color,
            input.depth,
            input.mv,
            input.reactive,
            &mut color_px,
            &mut depth_px,
            &mut mv_px,
            &mut reac_px,
        );
        let vtm_pack = vtm_t0.elapsed();
        // G14.7:fsr-dbg 逐帧诊断打印门控(FSR_DBG_STAGE 置位才打印;缺省零 stderr
        // IO——诊断面非生产路径固有面,CI 零消费)。
        let dbg_stage = std::env::var("FSR_DBG_STAGE").unwrap_or_default();
        let dbg_on = !dbg_stage.is_empty();
        if dbg_on {
            eprintln!("[fsr-dbg] pre-upload frame={}", input.frame_index);
        }
        let up_res = self
            .d3d_upload_tex(D3dInputSlot::Color, &color_px, 8, 0)
            .and_then(|()| {
                if dbg_on {
                    eprintln!("[fsr-dbg] upload color ok");
                }
                self.d3d_upload_tex(D3dInputSlot::Depth, &depth_px, 4, off_depth)
            })
            .and_then(|()| {
                if dbg_on {
                    eprintln!("[fsr-dbg] upload depth ok");
                }
                self.d3d_upload_tex(D3dInputSlot::Mv, &mv_px, 8, off_mv)
            })
            .and_then(|()| {
                if dbg_on {
                    eprintln!("[fsr-dbg] upload mv ok");
                }
                self.d3d_upload_tex(D3dInputSlot::Reactive, &reac_px, 1, off_reac)
            });
        self.pack_color = color_px;
        self.pack_depth = depth_px;
        self.pack_mv = mv_px;
        self.pack_reac = reac_px;
        up_res?;
        if dbg_on {
            eprintln!("[fsr-dbg] upload reactive ok");
        }
        let vtm_upload = vtm_t0.elapsed();
        if dbg_stage == "uploads_only" {
            self.d3d_submit_wait()?;
            eprintln!("[fsr-dbg] submit ok (uploads_only)");
            if vtm_on {
                let ms = |d: std::time::Duration| d.as_secs_f64() * 1e3;
                eprintln!(
                    "[vendor-timing fsr] frame={} pack={:.3} upload={:.3} (uploads_only 探针臂)",
                    input.frame_index,
                    ms(vtm_pack),
                    ms(vtm_upload - vtm_pack),
                );
            }
            return {
                // G14.6 Stage A:探针臂产出面 = 全零帧(与旧逐帧新零 vec 字面一致)。
                out_px.fill(0.0);
                Ok(())
            };
        }
        if self.color_out.state == D3D12_RESOURCE_STATE_COMMON {
            let out = self.color_out.clone_shallow();
            self.d3d_barrier(&out, D3D12_RESOURCE_STATE_COMMON, D3D12_RESOURCE_STATE_UNORDERED_ACCESS);
            self.color_out.state = D3D12_RESOURCE_STATE_UNORDERED_ACCESS;
        }

        let mk_res = |t: &D3dTexture, state: u32, usage: u32| -> FfxResource {
            FfxResource {
                resource: t.resource,
                description: FfxResourceDescription {
                    res_type: FFX_RESOURCE_TYPE_TEXTURE2D,
                    format: match t.format {
                        DXGI_FORMAT_R16G16B16A16_FLOAT => FFX_SURFACE_FORMAT_R16G16B16A16_FLOAT,
                        DXGI_FORMAT_R32G32_FLOAT => FFX_SURFACE_FORMAT_R32G32_FLOAT,
                        DXGI_FORMAT_R32_FLOAT => FFX_SURFACE_FORMAT_R32_FLOAT,
                        DXGI_FORMAT_R8_UNORM => FFX_SURFACE_FORMAT_R8_UNORM,
                        _ => FFX_SURFACE_FORMAT_R32G32B32A32_FLOAT,
                    },
                    width: t.w,
                    height: t.h,
                    depth: 1,
                    mip_count: 1,
                    flags: 0,
                    usage,
                },
                state,
            }
        };
        let dispatch = FfxDispatchDescUpscale {
            header: FfxApiHeader { desc_type: FFX_DISPATCH_DESC_TYPE_UPSCALE, p_next: std::ptr::null_mut() },
            command_list: self.cmd_list,
            color: mk_res(&self.color_in, FFX_RESOURCE_STATE_COMPUTE_READ, FFX_RESOURCE_USAGE_READ_ONLY),
            depth: mk_res(&self.depth_in, FFX_RESOURCE_STATE_COMPUTE_READ, FFX_RESOURCE_USAGE_READ_ONLY),
            motion_vectors: mk_res(&self.mv_in, FFX_RESOURCE_STATE_COMPUTE_READ, FFX_RESOURCE_USAGE_READ_ONLY),
            exposure: mk_res(&self.color_in, FFX_RESOURCE_STATE_COMPUTE_READ, FFX_RESOURCE_USAGE_READ_ONLY), // 空占位(见下)
            reactive: mk_res(&self.reactive_in, FFX_RESOURCE_STATE_COMPUTE_READ, FFX_RESOURCE_USAGE_READ_ONLY),
            transparency_and_composition: mk_res(&self.color_in, FFX_RESOURCE_STATE_COMPUTE_READ, FFX_RESOURCE_USAGE_READ_ONLY), // 空占位
            output: mk_res(&self.color_out, FFX_RESOURCE_STATE_UNORDERED_ACCESS, FFX_RESOURCE_USAGE_UAV),
            jitter_offset: FfxFloatCoords2D { x: input.jitter[0], y: input.jitter[1] },
            motion_vector_scale: FfxFloatCoords2D { x: iw as f32, y: ih as f32 }, // uv → 像素
            render_size: FfxDimensions2D { width: iw, height: ih },
            upscale_size: FfxDimensions2D { width: self.out_w, height: self.out_h },
            enable_sharpening: 0,
            _pad0: [0; 3],
            sharpness: 0.0,
            frame_time_delta: 16.6667,
            pre_exposure: input.exposure,
            reset: if input.reset { 1 } else { 0 },
            _pad1: [0; 3],
            camera_near: 0.1,
            camera_far: 100.0,
            camera_fov_angle_vertical: 60.0f32.to_radians(),
            view_space_to_meters_factor: 1.0,
            flags: 0,
        };
        // exposure/transparency 为可选空槽——ffx 空资源语义 = resource 指针 null;
        // 上式占位仅填充字段位,实际传空:
        let mut dispatch = dispatch;
        dispatch.exposure.resource = std::ptr::null_mut();
        dispatch.transparency_and_composition.resource = std::ptr::null_mut();
        eprintln!("[fsr-dbg] pre-dispatch frame={}", input.frame_index);
        if std::env::var("FSR_DBG_SKIP_DISPATCH").ok().as_deref() != Some("1") {
            // SAFETY: dispatch 栈上存活至 ffxDispatch 返回;cmd_list 录制中(Reset 后开录);
            // context 有效;资源全部已上传且状态与声明一致。
            let rc = unsafe { (self.ffx_dispatch)(&mut self.context, &dispatch.header) };
            eprintln!("[fsr-dbg] dispatch rc={rc}");
            if rc != FFX_OK {
                return Err(VendorError::VendorCall(format!("ffxDispatch(upscale) → {rc}")));
            }
        }
        let vtm_eval = vtm_t0.elapsed();

        // 输出回读:UAV → COPY_SOURCE → readback。
        let out = self.color_out.clone_shallow();
        self.d3d_barrier(&out, D3D12_RESOURCE_STATE_UNORDERED_ACCESS, D3D12_RESOURCE_STATE_COPY_SOURCE);
        let dst_loc = D3d12TextureCopyLocation {
            p_resource: self.readback,
            copy_type: 1, // PLACED_FOOTPRINT
            placed: D3d12PlacedSubresourceFootprint {
                offset: 0,
                footprint: D3d12SubresourceFootprint {
                    format: DXGI_FORMAT_R16G16B16A16_FLOAT,
                    width: self.out_w,
                    height: self.out_h,
                    depth: 1,
                    row_pitch: ((8u64 * self.out_w as u64 + 255) & !255) as u32,
                },
            },
        };
        let src_loc = D3d12TextureCopyLocation {
            p_resource: self.color_out.resource,
            copy_type: 0, // SUBRESOURCE_INDEX
            placed: D3d12PlacedSubresourceFootprint {
                offset: 0,
                footprint: D3d12SubresourceFootprint { format: 0, width: 0, height: 0, depth: 0, row_pitch: 0 },
            },
        };
        // SAFETY: CopyTextureRegion @16;输出在 COPY_SOURCE 态。
        unsafe {
            let f: unsafe extern "system" fn(*mut c_void, *const D3d12TextureCopyLocation, u32, u32, u32, *const D3d12TextureCopyLocation, *const c_void) = com_fn(self.cmd_list, 16);
            f(self.cmd_list, &dst_loc, 0, 0, 0, &src_loc, std::ptr::null());
        }
        self.d3d_barrier(&out, D3D12_RESOURCE_STATE_COPY_SOURCE, D3D12_RESOURCE_STATE_UNORDERED_ACCESS);
        self.d3d_submit_wait()?;
        let vtm_wait = vtm_t0.elapsed();

        // G14.6 Stage A:输出直写调用方驻留切片(消逐帧 ~out_px·12B 新分配+清零;
        // 转换逐值同式同序,字节面与 G14.3 面逐位一致)。
        let row_pitch = ((8u64 * self.out_w as u64 + 255) & !255) as usize;
        // SAFETY: readback heap fence 排空后 map;逐行按 row_pitch 读 f16。
        unsafe {
            let mut ptr: *mut c_void = std::ptr::null_mut();
            let f: unsafe extern "system" fn(*mut c_void, u32, *const c_void, *mut *mut c_void) -> Hresult = com_fn(self.readback, 8);
            let hr = f(self.readback, 0, std::ptr::null(), &mut ptr);
            if hr != S_OK || ptr.is_null() {
                return Err(VendorError::ApiError(format!("readback Map → 0x{hr:08x}")));
            }
            let ow = self.out_w as usize;
            let oh = self.out_h as usize;
            // G14.7:裸指针步行改 from_raw_parts 视图(U8 镜像纪律,0 新 U 号——U58
            // 扩注)+ 行带并行转换;带内逐行逐值同式同序,输出字节面与 G14.3 面逐位
            // 一致。切片长度 = (oh−1)·row_pitch/2 + ow·4 = 触及区间精确界,不越
            // map 区间(readback 分配 = row_pitch·oh 字节 ≥ 触及界·2)。
            let data = std::slice::from_raw_parts(
                ptr as *const u16,
                (oh - 1) * (row_pitch / 2) + ow * 4,
            );
            convert_out_pitched_par(data, row_pitch / 2, ow, oh, out_px);
            let unmap: unsafe extern "system" fn(*mut c_void, u32, *const c_void) = com_fn(self.readback, 9);
            unmap(self.readback, 0, std::ptr::null());
        }
        if vtm_on {
            let total = vtm_t0.elapsed().as_secs_f64() * 1e3;
            let ms = |d: std::time::Duration| d.as_secs_f64() * 1e3;
            eprintln!(
                "[vendor-timing fsr] frame={} pack={:.3} upload={:.3} evaluate={:.3} submit_wait={:.3} readback={:.3} total={:.3}ms",
                input.frame_index,
                ms(vtm_pack),
                ms(vtm_upload - vtm_pack),
                ms(vtm_eval - vtm_upload),
                ms(vtm_wait - vtm_eval),
                ms(vtm_t0.elapsed() - vtm_wait),
                total,
            );
        }
        Ok(())
    }

    /// G14.11 驻留 dispatch(buffer 共享形态):输入内容驻留共享 staging
    /// buffer(Vulkan pack kernel 直写,调用方保证该帧 Vulkan fence 已完成 +
    /// EXTERNAL release 已录),**零 host 输入上传/打包**;本方法先录三段
    /// `CopyTextureRegion`(staging placed footprint → color/depth/mv 纹理,
    /// GPU 内拷,barrier 律与 d3d_upload_tex 逐字同),再 ffx dispatch 录制 →
    /// 提交 → fence 有界等待(返回 = D3D12 读窗关闭,下帧 Vulkan 重写 staging
    /// 安全——CPU 序同步,与 DLSS 驻留车道 submit_wait 同律)。输入声明态 =
    /// COMPUTE_READ(拷后 barrier 至 NON_PIXEL_SHADER_RESOURCE,与现路径
    /// frame_impl_into 逐字同);输出驻留 color_out(UAV),digest/出图帧按需
    /// [`Self::readback_output_resident`]。jitter/mv scale/camera 等 dispatch
    /// 参数与现路径 frame_impl_into 逐字同值。
    pub fn dispatch_resident(
        &mut self,
        jitter: [f32; 2],
        exposure: f32,
        _frame_index: u32,
        reset: bool,
    ) -> Result<(), VendorError> {
        if self.staging_handle == 0 {
            return Err(VendorError::ApiError(
                "dispatch_resident 仅驻留 session 可用(create_resident)".into(),
            ));
        }
        let (iw, ih) = (self.in_w, self.in_h);
        // staging → 三输入纹理 GPU 内拷(staging buffer 恒 COMMON,buffer 隐式
        // 提升 COPY_SOURCE 免 barrier;纹理走显式 COPY_DEST 往返)。
        let [color_row, depth_row, mv_row, off_depth, off_mv, _] = self.staging_layout;
        let copies: [(D3dTexture, u64, u64); 3] = [
            (self.color_in.clone_shallow(), 0, color_row),
            (self.depth_in.clone_shallow(), off_depth, depth_row),
            (self.mv_in.clone_shallow(), off_mv, mv_row),
        ];
        for (tex, offset, row) in &copies {
            let before = if tex.state == D3D12_RESOURCE_STATE_COMMON {
                D3D12_RESOURCE_STATE_COMMON
            } else {
                D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE
            };
            self.d3d_barrier(tex, before, D3D12_RESOURCE_STATE_COPY_DEST);
            let dst_loc = D3d12TextureCopyLocation {
                p_resource: tex.resource,
                copy_type: 0, // SUBRESOURCE_INDEX
                placed: D3d12PlacedSubresourceFootprint {
                    offset: 0,
                    footprint: D3d12SubresourceFootprint { format: tex.format, width: 0, height: 0, depth: 0, row_pitch: 0 },
                },
            };
            let src_loc = D3d12TextureCopyLocation {
                p_resource: self.staging_buf,
                copy_type: 1, // PLACED_FOOTPRINT
                placed: D3d12PlacedSubresourceFootprint {
                    offset: *offset,
                    footprint: D3d12SubresourceFootprint {
                        format: tex.format,
                        width: tex.w,
                        height: tex.h,
                        depth: 1,
                        row_pitch: *row as u32,
                    },
                },
            };
            // SAFETY: CopyTextureRegion @16;dst/src loc 栈上存活;staging_buf
            // 驻留 session 恒非空(staging_handle 已判)。
            unsafe {
                let f: unsafe extern "system" fn(*mut c_void, *const D3d12TextureCopyLocation, u32, u32, u32, *const D3d12TextureCopyLocation, *const c_void) = com_fn(self.cmd_list, 16);
                f(self.cmd_list, &dst_loc, 0, 0, 0, &src_loc, std::ptr::null());
            }
            self.d3d_barrier(tex, D3D12_RESOURCE_STATE_COPY_DEST, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE);
        }
        self.color_in.state = D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE;
        self.depth_in.state = D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE;
        self.mv_in.state = D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE;
        if self.color_out.state == D3D12_RESOURCE_STATE_COMMON {
            let out = self.color_out.clone_shallow();
            self.d3d_barrier(&out, D3D12_RESOURCE_STATE_COMMON, D3D12_RESOURCE_STATE_UNORDERED_ACCESS);
            self.color_out.state = D3D12_RESOURCE_STATE_UNORDERED_ACCESS;
        }
        let mk_res = |t: &D3dTexture, state: u32, usage: u32| -> FfxResource {
            FfxResource {
                resource: t.resource,
                description: FfxResourceDescription {
                    res_type: FFX_RESOURCE_TYPE_TEXTURE2D,
                    format: match t.format {
                        DXGI_FORMAT_R32G32B32A32_FLOAT => FFX_SURFACE_FORMAT_R32G32B32A32_FLOAT,
                        DXGI_FORMAT_R16G16B16A16_FLOAT => FFX_SURFACE_FORMAT_R16G16B16A16_FLOAT,
                        DXGI_FORMAT_R32G32_FLOAT => FFX_SURFACE_FORMAT_R32G32_FLOAT,
                        DXGI_FORMAT_R32_FLOAT => FFX_SURFACE_FORMAT_R32_FLOAT,
                        _ => FFX_SURFACE_FORMAT_R8_UNORM,
                    },
                    width: t.w,
                    height: t.h,
                    depth: 1,
                    mip_count: 1,
                    flags: 0,
                    usage,
                },
                state,
            }
        };
        let mut dispatch = FfxDispatchDescUpscale {
            header: FfxApiHeader { desc_type: FFX_DISPATCH_DESC_TYPE_UPSCALE, p_next: std::ptr::null_mut() },
            command_list: self.cmd_list,
            color: mk_res(&self.color_in, FFX_RESOURCE_STATE_COMPUTE_READ, FFX_RESOURCE_USAGE_READ_ONLY),
            depth: mk_res(&self.depth_in, FFX_RESOURCE_STATE_COMPUTE_READ, FFX_RESOURCE_USAGE_READ_ONLY),
            motion_vectors: mk_res(&self.mv_in, FFX_RESOURCE_STATE_COMPUTE_READ, FFX_RESOURCE_USAGE_READ_ONLY),
            exposure: mk_res(&self.color_in, FFX_RESOURCE_STATE_COMPUTE_READ, FFX_RESOURCE_USAGE_READ_ONLY), // 空占位(下方置 null)
            reactive: mk_res(&self.reactive_in, FFX_RESOURCE_STATE_COMPUTE_READ, FFX_RESOURCE_USAGE_READ_ONLY),
            transparency_and_composition: mk_res(&self.color_in, FFX_RESOURCE_STATE_COMPUTE_READ, FFX_RESOURCE_USAGE_READ_ONLY), // 空占位
            output: mk_res(&self.color_out, FFX_RESOURCE_STATE_UNORDERED_ACCESS, FFX_RESOURCE_USAGE_UAV),
            jitter_offset: FfxFloatCoords2D { x: jitter[0], y: jitter[1] },
            motion_vector_scale: FfxFloatCoords2D { x: iw as f32, y: ih as f32 }, // uv → 像素
            render_size: FfxDimensions2D { width: iw, height: ih },
            upscale_size: FfxDimensions2D { width: self.out_w, height: self.out_h },
            enable_sharpening: 0,
            _pad0: [0; 3],
            sharpness: 0.0,
            frame_time_delta: 16.6667,
            pre_exposure: exposure,
            reset: if reset { 1 } else { 0 },
            _pad1: [0; 3],
            camera_near: 0.1,
            camera_far: 100.0,
            camera_fov_angle_vertical: 60.0f32.to_radians(),
            view_space_to_meters_factor: 1.0,
            flags: 0,
        };
        dispatch.exposure.resource = std::ptr::null_mut();
        dispatch.transparency_and_composition.resource = std::ptr::null_mut();
        // SAFETY: dispatch 栈上存活至 ffxDispatch 返回;cmd_list 录制中(Reset 后
        // 开录);context 有效;共享输入内容有效性由调用方 Vulkan fence 契约保证。
        let rc = unsafe { (self.ffx_dispatch)(&mut self.context, &dispatch.header) };
        if rc != FFX_OK {
            return Err(VendorError::VendorCall(format!("ffxDispatch(upscale,resident) → {rc}")));
        }
        self.d3d_submit_wait()
    }

    /// G14.11 驻留输出按需回读(color_out UAV → COPY_SOURCE → readback 堆 →
    /// f16→f32 转换直写调用方切片;与 frame_impl_into 尾段逐字同式——digest/
    /// EXR 面同一转换事实源)。
    pub fn readback_output_resident(&mut self, out_px: &mut [f32]) -> Result<(), VendorError> {
        if out_px.len() != (self.out_w * self.out_h * 3) as usize {
            return Err(VendorError::ApiError("输出切片长度与 session 输出分辨率不符".into()));
        }
        let out = self.color_out.clone_shallow();
        self.d3d_barrier(&out, D3D12_RESOURCE_STATE_UNORDERED_ACCESS, D3D12_RESOURCE_STATE_COPY_SOURCE);
        let dst_loc = D3d12TextureCopyLocation {
            p_resource: self.readback,
            copy_type: 1, // PLACED_FOOTPRINT
            placed: D3d12PlacedSubresourceFootprint {
                offset: 0,
                footprint: D3d12SubresourceFootprint {
                    format: DXGI_FORMAT_R16G16B16A16_FLOAT,
                    width: self.out_w,
                    height: self.out_h,
                    depth: 1,
                    row_pitch: ((8u64 * self.out_w as u64 + 255) & !255) as u32,
                },
            },
        };
        let src_loc = D3d12TextureCopyLocation {
            p_resource: self.color_out.resource,
            copy_type: 0, // SUBRESOURCE_INDEX
            placed: D3d12PlacedSubresourceFootprint {
                offset: 0,
                footprint: D3d12SubresourceFootprint { format: 0, width: 0, height: 0, depth: 0, row_pitch: 0 },
            },
        };
        // SAFETY: CopyTextureRegion @16;输出在 COPY_SOURCE 态。
        unsafe {
            let f: unsafe extern "system" fn(*mut c_void, *const D3d12TextureCopyLocation, u32, u32, u32, *const D3d12TextureCopyLocation, *const c_void) = com_fn(self.cmd_list, 16);
            f(self.cmd_list, &dst_loc, 0, 0, 0, &src_loc, std::ptr::null());
        }
        self.d3d_barrier(&out, D3D12_RESOURCE_STATE_COPY_SOURCE, D3D12_RESOURCE_STATE_UNORDERED_ACCESS);
        self.d3d_submit_wait()?;
        let row_pitch = ((8u64 * self.out_w as u64 + 255) & !255) as usize;
        // SAFETY: readback heap fence 排空后 map;逐行按 row_pitch 读 f16(转换
        // 面与 frame_impl_into 同一事实源 convert_out_pitched_par)。
        unsafe {
            let mut ptr: *mut c_void = std::ptr::null_mut();
            let f: unsafe extern "system" fn(*mut c_void, u32, *const c_void, *mut *mut c_void) -> Hresult = com_fn(self.readback, 8);
            let hr = f(self.readback, 0, std::ptr::null(), &mut ptr);
            if hr != S_OK || ptr.is_null() {
                return Err(VendorError::ApiError(format!("readback Map → 0x{hr:08x}")));
            }
            let ow = self.out_w as usize;
            let oh = self.out_h as usize;
            let data = std::slice::from_raw_parts(
                ptr as *const u16,
                (oh - 1) * (row_pitch / 2) + ow * 4,
            );
            convert_out_pitched_par(data, row_pitch / 2, ow, oh, out_px);
            let unmap: unsafe extern "system" fn(*mut c_void, u32, *const c_void) = com_fn(self.readback, 9);
            unmap(self.readback, 0, std::ptr::null());
        }
        Ok(())
    }

    /// G14.11:D3D12 adapter LUID(Vulkan deviceLUID 对拍面)。
    pub fn adapter_luid(&self) -> [u8; 8] {
        self.adapter_luid
    }

    /// G14.11 诊断臂:从 **D3D12 侧** 回读共享 color 输入纹理(RGBA32F 紧凑
    /// 字节直写 `out`)——跨 API tiling 解释一致性对拍(Vulkan 侧写入 → 两侧
    /// 各自 readback 逐字节对比;G14.10e dlss 臂 OPAQUE_WIN32 跨 VkDevice
    /// OPTIMAL 布局乱序前科,本方向官方 cross-API 协定仍须实证)。复用输出
    /// readback 堆(须 iw·ih·16 ≤ 堆容量——t67 及以下成立;越界确定性 Err)。
    pub fn debug_readback_input_color(&mut self, out: &mut Vec<u8>) -> Result<(), VendorError> {
        if self.staging_handle == 0 {
            return Err(VendorError::ApiError("仅驻留 session 可用".into()));
        }
        let (iw, ih) = (self.in_w, self.in_h);
        // color_in 为 f16 RGBA(8B/px,与 host 链逐字同);回读后 host 侧
        // f16→f32 转换,输出契约保持紧凑 f32 RGBA 字节(对拍脚本免改)。
        let row_pitch = ((8u64 * iw as u64 + 255) & !255) as usize;
        let need = row_pitch as u64 * ih as u64;
        let cap = ((8u64 * self.out_w as u64 + 255) & !255) * self.out_h as u64;
        if need > cap {
            return Err(VendorError::ApiError(format!(
                "诊断回读需 {need}B > readback 堆 {cap}B(用 t67 及以下档位对拍)"
            )));
        }
        // dispatch_resident 后 color_in 粘 NON_PIXEL_SHADER_RESOURCE;拷前显式
        // 转 COPY_SOURCE,拷后还原(状态机字段同步)。
        let tex = self.color_in.clone_shallow();
        let before = if tex.state == D3D12_RESOURCE_STATE_COMMON {
            D3D12_RESOURCE_STATE_COMMON
        } else {
            D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE
        };
        self.d3d_barrier(&tex, before, D3D12_RESOURCE_STATE_COPY_SOURCE);
        let dst_loc = D3d12TextureCopyLocation {
            p_resource: self.readback,
            copy_type: 1, // PLACED_FOOTPRINT
            placed: D3d12PlacedSubresourceFootprint {
                offset: 0,
                footprint: D3d12SubresourceFootprint {
                    format: DXGI_FORMAT_R16G16B16A16_FLOAT,
                    width: iw,
                    height: ih,
                    depth: 1,
                    row_pitch: row_pitch as u32,
                },
            },
        };
        let src_loc = D3d12TextureCopyLocation {
            p_resource: tex.resource,
            copy_type: 0, // SUBRESOURCE_INDEX
            placed: D3d12PlacedSubresourceFootprint {
                offset: 0,
                footprint: D3d12SubresourceFootprint { format: 0, width: 0, height: 0, depth: 0, row_pitch: 0 },
            },
        };
        // SAFETY: CopyTextureRegion @16;src 在 COPY_SOURCE 态。
        unsafe {
            let f: unsafe extern "system" fn(*mut c_void, *const D3d12TextureCopyLocation, u32, u32, u32, *const D3d12TextureCopyLocation, *const c_void) = com_fn(self.cmd_list, 16);
            f(self.cmd_list, &dst_loc, 0, 0, 0, &src_loc, std::ptr::null());
        }
        self.d3d_barrier(&tex, D3D12_RESOURCE_STATE_COPY_SOURCE, before);
        self.d3d_submit_wait()?;
        out.clear();
        out.reserve((iw * ih) as usize * 16);
        // SAFETY: readback heap fence 排空后 map;逐行按 row_pitch 取 f16 紧凑
        // 段,逐通道 f16_to_f32(与输出面同一转换事实源)后追加。
        unsafe {
            let mut ptr: *mut c_void = std::ptr::null_mut();
            let f: unsafe extern "system" fn(*mut c_void, u32, *const c_void, *mut *mut c_void) -> Hresult = com_fn(self.readback, 8);
            let hr = f(self.readback, 0, std::ptr::null(), &mut ptr);
            if hr != S_OK || ptr.is_null() {
                return Err(VendorError::ApiError(format!("readback Map(诊断) → 0x{hr:08x}")));
            }
            for y in 0..ih as usize {
                let row = std::slice::from_raw_parts(
                    (ptr as *const u8).add(y * row_pitch),
                    iw as usize * 8,
                );
                for px in row.chunks_exact(8) {
                    for c in 0..4 {
                        let h = u16::from_le_bytes([px[c * 2], px[c * 2 + 1]]);
                        out.extend_from_slice(&f16_to_f32(h).to_le_bytes());
                    }
                }
            }
            let unmap: unsafe extern "system" fn(*mut c_void, u32, *const c_void) = com_fn(self.readback, 9);
            unmap(self.readback, 0, std::ptr::null());
        }
        Ok(())
    }

    /// validation(D3D12 debug layer info queue)ERROR/CORRUPTION 级消息计数。
    pub fn validation_errors(&self) -> u64 {
        if self.info_queue.is_null() {
            return 0;
        }
        // SAFETY: ID3D12InfoQueue::GetNumStoredMessages @8。
        let n = unsafe {
            let f: unsafe extern "system" fn(*mut c_void) -> u64 = com_fn(self.info_queue, 8);
            f(self.info_queue)
        };
        let mut errors = 0u64;
        for i in 0..n {
            let mut len: usize = 0;
            // SAFETY: GetMessage @5(先查长度)。
            unsafe {
                let f: unsafe extern "system" fn(*mut c_void, u64, *mut c_void, *mut usize) -> Hresult = com_fn(self.info_queue, 5);
                let hr = f(self.info_queue, i, std::ptr::null_mut(), &mut len);
                if hr != S_OK || len == 0 {
                    continue;
                }
                let mut buf = vec![0u8; len];
                let hr = f(self.info_queue, i, buf.as_mut_ptr() as *mut c_void, &mut len);
                if hr == S_OK && len >= 24 {
                    let severity = (buf.as_ptr().add(4) as *const i32).read_unaligned();
                    if severity <= 1 {
                        // CORRUPTION(0)/ERROR(1)
                        errors += 1;
                    }
                }
            }
        }
        errors + self.ffx_errors + FFX_ERROR_COUNT.load(Ordering::Relaxed)
    }

    /// session 报告(evidence provenance 面;FSR4 ML 不可用如实登记)。
    pub fn report(&self) -> VendorSessionReport {
        VendorSessionReport {
            backend: "fsr_3.1.5_ffx_sdk_2.0.0_dx12".into(),
            gpu_name: self.gpu_name.clone(),
            validation_errors: self.validation_errors(),
            dlls: self.dlls.clone(),
            engine_version: self.provider_version.clone(),
            fsr4_ml_available: Some(self.fsr4_ml_available),
            fsr4_note: Some(if self.fsr4_ml_available {
                "FSR4 ML upscaler 可用面(未消费——本波兑现 = FSR 3.1.5 分析版)".into()
            } else {
                format!(
                    "FSR4 ML upscaler 不可用(适配器 = {},非 AMD RDNA4)→ 自动回退 FSR 3.1.5 分析版(provider: {})",
                    self.gpu_name, self.provider_version
                )
            }),
            available_versions: self.available_versions.clone(),
            log_tail: self.log_tail.clone(),
        }
    }
}

trait D3dTextureShallow {
    fn clone_shallow(&self) -> Self;
}
impl D3dTextureShallow for D3dTexture {
    fn clone_shallow(&self) -> Self {
        D3dTexture { resource: self.resource, format: self.format, w: self.w, h: self.h, state: self.state }
    }
}

// kernel32 `CloseHandle`(G14.11:CreateSharedHandle 导出的 NT handle 归
// session 所有,Drop 单点关闭——导入方不得 CloseHandle,防双关)。
#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn CloseHandle(handle: *mut c_void) -> i32;
}

impl Drop for FsrDx12Session {
    fn drop(&mut self) {
        // G14.11:shared NT handle 单点关闭(先于资源 Release;导入侧 Vulkan
        // device memory 自持引用,句柄关闭不影响已导入内存)。
        #[cfg(windows)]
        if self.staging_handle != 0 {
            // SAFETY: handle 为本 session CreateSharedHandle 产出的有效 NT
            // handle,单点关闭不重入。
            unsafe {
                let _ = CloseHandle(self.staging_handle as *mut c_void);
            }
        }
        self.staging_handle = 0;
        // fence 排空 → ffxDestroyContext → COM 逆序 Release。
        if !self.fence.is_null() && self.fence_value > 0 {
            // SAFETY: fence 有效;轮询排空(有界)。
            unsafe {
                let get: unsafe extern "system" fn(*mut c_void) -> u64 = com_fn(self.fence, 8);
                let mut spins = 0u64;
                while get(self.fence) < self.fence_value && spins < 200_000_000 {
                    spins += 1;
                    std::hint::spin_loop();
                }
            }
        }
        if !self.context.is_null() {
            let mut ctx = self.context;
            // SAFETY: context 有效;destroy 后不再消费。
            unsafe { (self.ffx_destroy)(&mut ctx, std::ptr::null()) };
            self.context = std::ptr::null_mut();
        }
        for obj in [
            self.color_in.resource,
            self.depth_in.resource,
            self.mv_in.resource,
            self.reactive_in.resource,
            self.color_out.resource,
            self.staging_buf,
            self.upload,
            self.readback,
            self.fence,
            self.cmd_list,
            self.allocator,
            self.queue,
            self.info_queue,
            self.device,
        ] {
            com_release(obj);
        }
        self.device = std::ptr::null_mut();
    }
}

/// SDK 目录解析(DLSS 臂)。
pub fn streamline_sdk_dir() -> Result<PathBuf, VendorError> {
    default_sdk_dir(STREAMLINE_SDK_DIR_ENV, "external/streamline-2.10.3")
}

/// SDK 目录解析(FSR 臂)。
pub fn fsr_sdk_dir() -> Result<PathBuf, VendorError> {
    default_sdk_dir(FSR_SDK_DIR_ENV, "external/fidelityfx-sdk-2.0.0")
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn ffi_size_probe() {
        eprintln!("PROBE FfxDispatchDescUpscale = {}", size_of::<FfxDispatchDescUpscale>());
        eprintln!("PROBE FfxResource = {}", size_of::<FfxResource>());
        eprintln!("PROBE FfxApiHeader = {}", size_of::<FfxApiHeader>());
        eprintln!("PROBE VkApplicationInfo = {}", size_of::<VkApplicationInfo>());
        eprintln!("PROBE VkDeviceCreateInfo = {}", size_of::<VkDeviceCreateInfo>());
        eprintln!("PROBE VkImageViewCreateInfo = {}", size_of::<VkImageViewCreateInfo>());
        eprintln!("PROBE D3d12ResourceDesc = {}", size_of::<D3d12ResourceDesc>());
    }
    #[test]
    fn sl_struct_layout_anchors() {
        assert_eq!(size_of::<SlBaseStructure>(), 32);
        assert_eq!(size_of::<SlPreferencesLayout>(), 144);
        assert_eq!(size_of::<SlConstants>(), 456);
        assert_eq!(size_of::<SlResource>(), 112);
        assert_eq!(size_of::<SlResourceTag>(), 64);
        assert_eq!(size_of::<SlViewportHandle>(), 40);
        assert_eq!(size_of::<SlDlssOptions>(), 88);
        assert_eq!(size_of::<SlDlssOptimalSettings>(), 64);
        assert_eq!(size_of::<SlFeatureVersion>(), 56);
    }

    #[test]
    fn vk_struct_layout_anchors() {
        assert_eq!(size_of::<VkApplicationInfo>(), 48);
        assert_eq!(size_of::<VkInstanceCreateInfo>(), 64);
        assert_eq!(size_of::<VkDeviceQueueCreateInfo>(), 40);
        assert_eq!(size_of::<VkDeviceCreateInfo>(), 72);
        assert_eq!(size_of::<VkImageCreateInfo>(), 88);
        assert_eq!(size_of::<VkImageViewCreateInfo>(), 80);
        assert_eq!(size_of::<VkBufferCreateInfo>(), 56);
        assert_eq!(size_of::<VkMemoryAllocateInfo>(), 32);
        assert_eq!(size_of::<VkCommandPoolCreateInfo>(), 24);
        assert_eq!(size_of::<VkCommandBufferAllocateInfo>(), 32);
        assert_eq!(size_of::<VkCommandBufferBeginInfo>(), 32);
        assert_eq!(size_of::<VkSubmitInfo>(), 72);
        assert_eq!(size_of::<VkImageMemoryBarrier>(), 72);
        assert_eq!(size_of::<VkBufferImageCopy>(), 56);
        assert_eq!(size_of::<VkDebugUtilsMessengerCreateInfo>(), 48);
    }

    #[test]
    fn ffx_struct_layout_anchors() {
        assert_eq!(size_of::<FfxApiHeader>(), 16);
        assert_eq!(size_of::<FfxCreateContextDescUpscale>(), 48);
        assert_eq!(size_of::<FfxCreateBackendDx12Desc>(), 24);
        assert_eq!(size_of::<FfxResource>(), 48);
        assert_eq!(size_of::<FfxDispatchDescUpscale>(), 432);
        assert_eq!(size_of::<FfxQueryDescGetVersions>(), 56);
        assert_eq!(size_of::<FfxOverrideVersion>(), 24);
        assert_eq!(size_of::<FfxQueryGetProviderVersion>(), 32);
    }

    #[test]
    fn d3d_struct_layout_anchors() {
        assert_eq!(size_of::<D3d12CommandQueueDesc>(), 16);
        assert_eq!(size_of::<D3d12HeapProperties>(), 20);
        assert_eq!(size_of::<D3d12ResourceDesc>(), 56);
        assert_eq!(size_of::<D3d12ResourceBarrier>(), 32);
        assert_eq!(size_of::<D3d12TextureCopyLocation>(), 48);
    }

    #[test]
    fn f16_roundtrip_and_edges() {
        for v in [0.0f32, 1.0, 0.5, 0.1, 65504.0, 1e-4, 3.14159265, 2.0, 0.999] {
            let h = f32_to_f16(v);
            let back = f16_to_f32(h);
            let tol = (v.abs() * 1e-3).max(1e-7);
            assert!((back - v).abs() <= tol, "f16 roundtrip {v} → {h:#x} → {back}");
        }
        assert_eq!(f16_to_f32(f32_to_f16(0.0)), 0.0);
        assert!(f16_to_f32(f32_to_f16(f32::INFINITY)).is_infinite());
    }

    /// G14.11 修正回归锚:f16→f32 全 65536 位型枚举 vs 位精确公式参考
    /// (f32 可精确表示一切 f16 值——subnormal = mant·2⁻²⁴、normal =
    /// (1024+mant)·2^(exp−25);NaN 仅验类别)。fsr 对拍臂检出的 subnormal
    /// 减半缺陷(e 初值 −1)由本测试永久钉死。
    #[test]
    fn f16_to_f32_exhaustive_bitexact() {
        for h in 0u32..=0xffff {
            let h = h as u16;
            let exp = (h >> 10) & 0x1f;
            let mant = (h & 0x3ff) as f64;
            let sgn = if h & 0x8000 != 0 { -1.0f64 } else { 1.0 };
            let got = f16_to_f32(h);
            if exp == 31 {
                if mant == 0.0 {
                    assert!(got.is_infinite() && (got < 0.0) == (sgn < 0.0), "inf {h:#06x}");
                } else {
                    assert!(got.is_nan(), "nan {h:#06x}");
                }
                continue;
            }
            let want = if exp == 0 {
                sgn * mant * (-24f64).exp2()
            } else {
                sgn * (1024.0 + mant) * f64::from(exp as i32 - 25).exp2()
            } as f32;
            assert_eq!(
                got.to_bits(),
                want.to_bits(),
                "f16 {h:#06x} → got {got:e} ≠ want {want:e}"
            );
        }
    }

    #[test]
    fn g14_7_parallel_conversion_bitexact() {
        // G14.7 延续波：像素带并行 vs 单带串行的输出字节面必须逐位一致（带切分
        // 仅改线程归属，元素间零依赖）。覆盖面：并行阈上 px（真多带）/ 阈下小格
        // （强制单带）+ NaN/Inf/subnormal/上溢边角值 + reactive Some/None 双臂
        // + 连续 RGBA 回读 / 行距对齐回读双转换面。单测试内聚（env 互斥面）。
        let px = 200_000usize; // ≥ PAR_MIN_PX(131072) → 真多带面
        let mut color = vec![0.0f32; px * 3];
        let mut depth = vec![0.0f32; px];
        let mut mv = vec![0.0f32; px * 2];
        for i in 0..px {
            color[i * 3] = (i as f32) * 0.001;
            color[i * 3 + 1] = match i % 5 {
                0 => f32::NAN,
                1 => f32::INFINITY,
                2 => f32::NEG_INFINITY,
                3 => 1e-7 * (i as f32), // f16 subnormal/下溢域
                _ => 70000.0,           // f16 上溢 → inf
            };
            color[i * 3 + 2] = 0.5;
            depth[i] = ((i as f32) * 0.000_002).min(1.0);
            mv[i * 2] = -0.5;
            mv[i * 2 + 1] = 1e-6 * (i as f32);
        }
        let reac: Vec<f32> = (0..px).map(|i| (i % 997) as f32 / 997.0).collect();
        for reactive in [None, Some(reac.as_slice())] {
            let mut refer: Option<(Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>)> = None;
            for bands in [1usize, 8] {
                let (mut c, mut d, mut m, mut r8) =
                    (vec![0u8; px * 8], vec![0u8; px * 4], vec![0u8; px * 8], vec![0u8; px]);
                pack_vendor_inputs_bands(px, &color, &depth, &mv, reactive, &mut c, &mut d, &mut m, &mut r8, bands);
                match &refer {
                    None => refer = Some((c, d, m, r8)),
                    Some((rc, rd, rm, rr)) => assert!(
                        c == *rc && d == *rd && m == *rm && r8 == *rr,
                        "pack 并行/串行字节面不一致（reactive={}）",
                        reactive.is_some()
                    ),
                }
            }
        }
        // 带数决策纯函数锚（阈下小格恒单带；PAR=0 串行对照臂；显式 N 带）
        assert_eq!(par_band_count_with(1000, Some("8")), 1);
        assert_eq!(par_band_count_with(200_000, Some("0")), 1);
        assert_eq!(par_band_count_with(200_000, Some("3")), 3);
        assert!(par_band_count_with(200_000, None) >= 1);
        // 阈下小格字节面（显式带数直驱）
        let spx = 1000usize;
        let mut refs: Option<(Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>)> = None;
        for bands in [1usize, 8] {
            let (mut c, mut d, mut m, mut r8) =
                (vec![0u8; spx * 8], vec![0u8; spx * 4], vec![0u8; spx * 8], vec![0u8; spx]);
            pack_vendor_inputs_bands(
                spx,
                &color[..spx * 3],
                &depth[..spx],
                &mv[..spx * 2],
                None,
                &mut c,
                &mut d,
                &mut m,
                &mut r8,
                bands,
            );
            match &refs {
                None => refs = Some((c, d, m, r8)),
                Some((rc, rd, rm, rr)) => assert!(c == *rc && d == *rd && m == *rm && r8 == *rr),
            }
        }
        // 连续 RGBA f16 → f32 回读转换面（含任意位型）
        let mut h = vec![0u16; px * 4];
        for (i, v) in h.iter_mut().enumerate() {
            *v = match i % 6 {
                0 => 0x7c00,              // +inf
                1 => 0x7e01,              // NaN
                2 => 0x0001,              // subnormal
                3 => 0x8000,              // −0
                4 => 0xfc00,              // −inf
                _ => (i as u16).wrapping_mul(7),
            };
        }
        let mut ref_out: Option<Vec<f32>> = None;
        for bands in [1usize, 8] {
            let mut out = vec![9.9f32; px * 3];
            convert_out_par_bands(&h, &mut out, bands);
            match &ref_out {
                None => ref_out = Some(out),
                Some(r) => assert!(
                    out.iter().zip(r.iter()).all(|(a, b)| a.to_bits() == b.to_bits()),
                    "连续回读并行/串行位级不一致"
                ),
            }
        }
        // 行距对齐回读转换面（行距余量区零消费）
        let (ow, oh) = (642usize, 362usize);
        let rp2 = ow * 4 + 32; // 人为行距余量
        let mut pdata = vec![0u16; (oh - 1) * rp2 + ow * 4];
        for (i, v) in pdata.iter_mut().enumerate() {
            *v = (i as u16).wrapping_mul(13) ^ 0x5555;
        }
        let mut refp: Option<Vec<f32>> = None;
        for bands in [1usize, 8] {
            let mut out = vec![0.0f32; ow * oh * 3];
            convert_out_pitched_par_bands(&pdata, rp2, ow, oh, &mut out, bands);
            match &refp {
                None => refp = Some(out),
                Some(r) => assert!(
                    out.iter().zip(r.iter()).all(|(a, b)| a.to_bits() == b.to_bits()),
                    "行距回读并行/串行位级不一致"
                ),
            }
        }
    }

    #[test]
    fn missing_sdk_fail_closed() {
        // 未设环境变量且默认目录缺失时,目录解析或装载必须确定性 Err(不 panic)。
        let bogus = Path::new("Z:/rurix-definitely-not-here/streamline");
        let r = DlssVkSession::create(bogus, (64, 64), (128, 128), false);
        assert!(matches!(r, Err(VendorError::DllNotFound(_))));
        let r = FsrDx12Session::create(bogus, (64, 64), (128, 128), false);
        assert!(matches!(r, Err(VendorError::DllNotFound(_))));
    }

    /// SL 全局态串行门(slInit/slShutdown 进程级单例;多 SL 测试在 cargo test
    /// 默认并行下互斥——G14.10b 加第二个 SL 真跑测试后必需)。poisoned 锁照拿
    /// (前测试 panic 不连坐后测试)。
    static SL_GATE: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// G14plus vendor 域(RFC-0030)驻留输出真跑冒烟:SDK/GPU 环境缺失 → SKIP
    /// (eprintln 登记,不硬红——真跑硬门在 bin/smoke 层,RURIX_REQUIRE_REAL
    /// 纪律同律);环境在位则全链断言。覆盖:① 未评估前按需回读 fail-closed;
    /// ② `upscale_resident` 两帧真 evaluate;③ `output_image_raw` 簿记
    /// (GENERAL/尺寸/RGBA16F);④ `readback_output_into` 输出有限非全零 +
    /// 长度错 fail-closed;⑤ 既有 `upscale` 路径同 session 继跑(0-byte 回归面)。
    #[test]
    fn g14plus_resident_output_gpu_smoke() {
        let _gate = SL_GATE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let Ok(sdk) = streamline_sdk_dir() else {
            eprintln!("SKIP g14plus_resident_output_gpu_smoke: Streamline SDK 目录不可用");
            return;
        };
        let (iw, ih, ow, oh) = (64u32, 64u32, 128u32, 128u32);
        let mut sess = match DlssVkSession::create(&sdk, (iw, ih), (ow, oh), false) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("SKIP g14plus_resident_output_gpu_smoke: DLSS session 不可用({e})");
                return;
            }
        };
        let px = (iw * ih) as usize;
        let out_px = (ow * oh) as usize;
        // ① 未评估前回读 fail-closed(layout 仍 UNDEFINED)。
        let mut out = vec![0f32; out_px * 3];
        assert!(matches!(sess.readback_output_into(&mut out), Err(VendorError::ApiError(_))));
        // 合成输入(渐变 color / 常深度 / 零 MV)。
        let color: Vec<f32> = (0..px * 3).map(|i| (i % 255) as f32 / 255.0).collect();
        let depth = vec![0.5f32; px];
        let mv = vec![0f32; px * 2];
        // ② 驻留输出两帧(真 evaluate;输出不回读)。
        for (fi, reset) in [(1u32, true), (2u32, false)] {
            let input = VendorFrameInput {
                color: &color,
                depth: &depth,
                mv: &mv,
                reactive: None,
                exposure: 1.0,
                jitter: [0.0, 0.0],
                frame_index: fi,
                reset,
            };
            sess.upscale_resident(&input).expect("upscale_resident 真跑");
        }
        // ③ 输出 image 簿记:已有评估内容(GENERAL)+ 尺寸/格式锚。
        let raw = sess.output_image_raw();
        assert_eq!(raw.layout, VK_IMAGE_LAYOUT_GENERAL);
        assert_eq!((raw.width, raw.height), (ow, oh));
        assert_eq!(raw.vk_format, VK_FORMAT_R16G16B16A16_SFLOAT);
        assert!(raw.image != 0 && raw.view != 0 && raw.memory != 0);
        // ④ 按需回读:长度错 fail-closed;正确长度输出有限且非全零。
        let mut short = vec![0f32; out_px];
        assert!(matches!(sess.readback_output_into(&mut short), Err(VendorError::ApiError(_))));
        sess.readback_output_into(&mut out).expect("readback_output_into");
        assert!(out.iter().all(|v| v.is_finite()), "回读输出须全有限");
        assert!(out.iter().any(|&v| v > 0.0), "回读输出须非全零");
        // ⑤ 既有 upscale 路径同 session 继跑(行为 0-byte 回归面)。
        let input3 = VendorFrameInput {
            color: &color,
            depth: &depth,
            mv: &mv,
            reactive: None,
            exposure: 1.0,
            jitter: [0.0, 0.0],
            frame_index: 3,
            reset: false,
        };
        let legacy = sess.upscale(&input3).expect("既有 upscale 路径");
        assert_eq!(legacy.len(), out_px * 3);
        assert!(legacy.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn sl_guid_layout() {
        let g = SL_GUID_PREFERENCES;
        assert_eq!(g.data1, 0x1ca10965);
        assert_eq!(g.data2, 0xbf8e);
        assert_eq!(g.data3, 0x432b);
        assert_eq!(g.data4[0], 0x8d);
        assert_eq!(g.data4[7], 0x14);
    }

    // ── G14.10b external memory 导入面 ──

    /// G14.10b FFI 布局锚(SDK 1.3.296 `vulkan_core.h` 逐字段核对)。
    #[test]
    fn g14_10b_external_ffi_layout_anchors() {
        assert_eq!(size_of::<VkExternalMemoryImageCreateInfo>(), 24);
        assert_eq!(size_of::<VkImportMemoryWin32HandleInfoKHR>(), 40);
        assert_eq!(size_of::<VkMemoryDedicatedAllocateInfo>(), 32);
        assert_eq!(size_of::<VkPhysicalDeviceIDProperties>(), 64);
        assert_eq!(align_of::<VkPhysicalDeviceProperties2Blob>(), 8);
        // OPAQUE_WIN32 = 0x2(0x1 是 OPAQUE_FD——两侧同值锚死,防再犯)。
        assert_eq!(VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_WIN32, 0x2);
        assert_eq!(VK_QUEUE_FAMILY_EXTERNAL, u32::MAX - 1);
    }

    /// G14.10b 最小 noop compute SPIR-V(render_exec exportable session 的
    /// 占位 pass;无绑定无 push,LocalSize 1×1×1 空 main)。
    #[cfg(windows)]
    fn noop_compute_spv() -> Vec<u8> {
        fn inst(v: &mut Vec<u32>, opcode: u32, operands: &[u32]) {
            v.push(((operands.len() as u32 + 1) << 16) | opcode);
            v.extend_from_slice(operands);
        }
        // header:magic / version 1.3 / generator 0 / bound 5 / schema 0。
        let mut w = vec![0x0723_0203u32, 0x0001_0300, 0, 5, 0];
        inst(&mut w, 17, &[1]); // OpCapability Shader
        inst(&mut w, 14, &[0, 1]); // OpMemoryModel Logical GLSL450
        inst(&mut w, 15, &[5, 3, 0x6E69_616D, 0]); // OpEntryPoint GLCompute %3 "main"
        inst(&mut w, 16, &[3, 17, 1, 1, 1]); // OpExecutionMode %3 LocalSize 1 1 1
        inst(&mut w, 19, &[1]); // %1 = OpTypeVoid
        inst(&mut w, 33, &[2, 1]); // %2 = OpTypeFunction %1
        inst(&mut w, 54, &[1, 3, 0, 2]); // %3 = OpFunction %1 None %2
        inst(&mut w, 248, &[4]); // %4 = OpLabel
        inst(&mut w, 253, &[]); // OpReturn
        inst(&mut w, 56, &[]); // OpFunctionEnd
        w.iter().flat_map(|x| x.to_le_bytes()).collect()
    }

    /// G14.10b 探明臂:render_exec exportable(color_fmt/depth_fmt/RG32F mv,
    /// 合成图案 data 上传 + 帧末 EXTERNAL release)→ LUID 对拍 → DLSS 导入
    /// 三输入 → `upscale_resident_external` 两帧 → 按需回读。返回诊断字面
    /// (Ok = 臂全链通过且输出有限非全零)。
    #[cfg(windows)]
    #[allow(clippy::too_many_lines)]
    fn run_external_input_arm(
        sdk: &Path,
        color_fmt: crate::render_exec::TexFormat,
        depth_fmt: crate::render_exec::TexFormat,
    ) -> Result<String, String> {
        use crate::render_exec::{
            Bindings, BufferUsage, ComputePass, DeviceFrameSession, DispatchSpec, Pass,
            ResourceDesc, TexFormat, TextureDesc, TextureUsage,
        };
        let _ = BufferUsage::default(); // 引用面稳定(无 buffer 资源)
        let (iw, ih, ow, oh) = (64u32, 64u32, 128u32, 128u32);
        let px = (iw * ih) as usize;
        // 合成输入:color 渐变(a=1),depth 常量 0.5,mv 零。
        let color_f32: Vec<f32> = (0..px)
            .flat_map(|i| {
                let x = (i as u32 % iw) as f32 / (iw - 1) as f32;
                let y = (i as u32 / iw) as f32 / (ih - 1) as f32;
                [x, y, 0.25, 1.0]
            })
            .collect();
        let color_bytes: Vec<u8> = match color_fmt {
            TexFormat::Rgba32Float => color_f32.iter().flat_map(|v| v.to_le_bytes()).collect(),
            TexFormat::Rgba16Float => color_f32
                .iter()
                .flat_map(|&v| f32_to_f16(v).to_le_bytes())
                .collect(),
            other => return Err(format!("臂不支持 color 格式 {other:?}")),
        };
        let depth_bytes: Vec<u8> = std::iter::repeat_n(0.5f32.to_le_bytes(), px)
            .flatten()
            .collect();
        let mv_bytes: Vec<u8> = vec![0u8; px * 8];
        let spv = noop_compute_spv();
        let mk_usage = || TextureUsage {
            sampled: true,
            storage: false,
            color: false,
            depth: false,
        };
        let resources = vec![
            ResourceDesc::Texture(TextureDesc {
                width: iw,
                height: ih,
                format: color_fmt,
                usage: mk_usage(),
                data: Some(&color_bytes),
            }),
            ResourceDesc::Texture(TextureDesc {
                width: iw,
                height: ih,
                format: depth_fmt,
                usage: mk_usage(),
                data: Some(&depth_bytes),
            }),
            ResourceDesc::Texture(TextureDesc {
                width: iw,
                height: ih,
                format: TexFormat::Rg32Float,
                usage: mk_usage(),
                data: Some(&mv_bytes),
            }),
        ];
        let passes = vec![Pass::Compute(ComputePass {
            name: "noop",
            spirv: &spv,
            entry: None,
            dispatch: DispatchSpec::Direct([1, 1, 1]),
            bindings: Bindings::default(),
        })];
        let plan: Vec<Vec<(u32, crate::render_exec::TargetState)>> = vec![vec![]];
        let brefs: Vec<&[(u32, crate::render_exec::TargetState)]> =
            plan.iter().map(Vec::as_slice).collect();
        let mut session = DeviceFrameSession::new_with_exportable_textures(
            &resources,
            &passes,
            &brefs,
            &[],
            2,
            &[],
            &[0, 1, 2],
        )
        .map_err(|e| format!("exportable session: {e}"))?;
        let src_luid = session
            .physical_device_luid()
            .ok_or("render_exec 侧 deviceLUIDValid=false")?;
        // 帧 1:上传三输入图案 + 帧末 EXTERNAL release。
        session.execute().map_err(|e| format!("export 帧 1: {e}"))?;
        let exp_color = session
            .export_texture_win32_handle(0)
            .map_err(|e| format!("导出 color: {e}"))?;
        let exp_depth = session
            .export_texture_win32_handle(1)
            .map_err(|e| format!("导出 depth: {e}"))?;
        let exp_mv = session
            .export_texture_win32_handle(2)
            .map_err(|e| format!("导出 mv: {e}"))?;

        let mut dlss = DlssVkSession::create(sdk, (iw, ih), (ow, oh), false)
            .map_err(|e| format!("DLSS session: {e}"))?;
        if !dlss.external_memory_enabled() {
            return Err("DLSS 侧 VK_KHR_external_memory_win32 不在位".into());
        }
        let dst_luid = dlss
            .physical_device_luid()
            .ok_or("DLSS 侧 deviceLUIDValid=false")?;
        if src_luid != dst_luid {
            return Err(format!(
                "LUID 不匹配({src_luid:?} vs {dst_luid:?})——不同 adapter 不可共享,fail-closed"
            ));
        }
        let to_desc = |e: &crate::render_exec::ExportedTextureWin32| ExternalImageImportDesc {
            handle: e.handle,
            width: e.width,
            height: e.height,
            vk_format: e.vk_format,
            usage_flags: e.usage_flags,
            allocation_size: e.allocation_size,
            memory_type_index: e.memory_type_index,
        };
        dlss.import_win32_input(ExternalInputSlot::Color, &to_desc(&exp_color))
            .map_err(|e| format!("导入 color: {e}"))?;
        dlss.import_win32_input(ExternalInputSlot::Depth, &to_desc(&exp_depth))
            .map_err(|e| format!("导入 depth: {e}"))?;
        dlss.import_win32_input(ExternalInputSlot::Mv, &to_desc(&exp_mv))
            .map_err(|e| format!("导入 mv: {e}"))?;
        // 重复导入 fail-closed(单次导入纪律)。
        if dlss
            .import_win32_input(ExternalInputSlot::Color, &to_desc(&exp_color))
            .is_ok()
        {
            return Err("重复导入未被拒(fail-closed 漏)".into());
        }
        // 两帧 evaluate(帧间 render_exec 再 execute 一帧维持 release/acquire
        // 逐帧配对纪律;CPU 顺序化:execute 返回 = fence 完成)。
        for (fi, reset) in [(1u32, true), (2u32, false)] {
            if fi > 1 {
                session
                    .execute()
                    .map_err(|e| format!("export 帧 {fi}: {e}"))?;
            }
            dlss.upscale_resident_external(&VendorExternalFrameParams {
                reactive: None,
                exposure: 1.0,
                jitter: [0.0, 0.0],
                frame_index: fi,
                reset,
            })
            .map_err(|e| format!("evaluate 帧 {fi}: {e}"))?;
        }
        let out_px = (ow * oh) as usize;
        let mut out = vec![0f32; out_px * 3];
        dlss.readback_output_into(&mut out)
            .map_err(|e| format!("readback: {e}"))?;
        if !out.iter().all(|v| v.is_finite()) {
            return Err("输出含非有限值".into());
        }
        let nonzero = out.iter().filter(|&&v| v > 0.0).count();
        if nonzero == 0 {
            return Err("输出全零(evaluate 未产内容)".into());
        }
        Ok(format!(
            "color={color_fmt:?} depth={depth_fmt:?} 两帧 evaluate 输出非全零({nonzero}/{} 正值)",
            out_px * 3
        ))
    }

    /// G14.10b DLSS resident-input 全链真跑冒烟 + **SL 输入格式容忍度探明**
    /// (SKIP 纪律同 g14plus smoke)。臂序(降级链,如实登记):
    /// ① RGBA32F color + R32F depth(全 f32 直通——copy 路线零转换理想);
    /// ② RGBA16F color + R32F depth(compute 直写路线:imageStore 硬件 f32→f16,
    ///   depth R32F 是 storage 直写关键——D32 不可 storage 写);
    /// ③ RGBA16F color + D32 depth(与 DLSS 现自建输入完全同格式,保底臂)。
    /// 任一臂过 = 全链闭环(exportable→导出→导入→evaluate→非全零);全臂败才红。
    #[test]
    #[cfg(windows)]
    fn g14_10b_dlss_external_input_gpu_smoke() {
        use crate::render_exec::TexFormat;
        let _gate = SL_GATE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let Ok(sdk) = streamline_sdk_dir() else {
            eprintln!("SKIP g14_10b_dlss_external_input_gpu_smoke: Streamline SDK 目录不可用");
            return;
        };
        if !crate::vk::vulkan_available() {
            eprintln!("SKIP g14_10b_dlss_external_input_gpu_smoke: vulkan loader 不可用");
            return;
        }
        let arms = [
            (TexFormat::Rgba32Float, TexFormat::R32Float, "①全f32直通"),
            (TexFormat::Rgba16Float, TexFormat::R32Float, "②f16color+R32F depth"),
            (TexFormat::Rgba16Float, TexFormat::Depth32Float, "③现状同格式保底"),
        ];
        let mut failures: Vec<String> = Vec::new();
        for (color_fmt, depth_fmt, label) in arms {
            match run_external_input_arm(&sdk, color_fmt, depth_fmt) {
                Ok(diag) => {
                    eprintln!("[g14.10b] 臂{label} PASS: {diag}");
                    if !failures.is_empty() {
                        eprintln!("[g14.10b] 先行臂失败登记(格式容忍度探明): {failures:?}");
                    }
                    return;
                }
                Err(e) => {
                    eprintln!("[g14.10b] 臂{label} 失败: {e}");
                    failures.push(format!("{label}: {e}"));
                }
            }
        }
        panic!("g14.10b 全臂失败(含现状同格式保底臂——非格式问题,链路有病): {failures:?}");
    }
}
