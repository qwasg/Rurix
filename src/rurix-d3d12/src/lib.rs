//! `rurix-d3d12` — D3D12/DXGI present 薄 C/C++ shim 的 Rust 边界（G1.1，RFC-0001 §4.2）。
//!
//! D3D12/DXGI 的 COM 复杂度全部留在 C++ shim（`shim/rx_d3d12_shim.cpp`），**不进语言**
//! （D-130）；Rust 侧仅见版本化扁平 `extern "C"` 面。
//!
//! - **默认（stub）**：不编译 C++，全部入口返回 [`RX_D3D12_E_NOTIMPL`]（shim 不可用）。
//!   无 Windows SDK D3D12 环境亦可编译（常驻回归网绿）。
//! - **feature `real-shim`**：经 `build.rs` + `cc` 编译真实 shim 并 FFI 调用。
//!
//! 窗口 / 消息泵 / DXGI factory·adapter / D3D12 device·queue·swapchain / 固定 present
//! shader / 共享 resource·fence **全部由 shim 拥有**；对象固定在创建线程（RFC-0001 §4.2.1）。
//! ABI 版本 [`RX_D3D12_ABI_VERSION`]，[`InteropExport`] 结构 96 字节（编译期断言核对）。

use core::ffi::c_void;

/// shim C ABI 版本（与 C++ `RX_D3D12_ABI_VERSION` 一致，RFC-0001 §4.2.1）。
pub const RX_D3D12_ABI_VERSION: u32 = 1;
/// present 标志：VSync（RFC-0001 §4.2.1）。
pub const RX_D3D12_PRESENT_VSYNC: u32 = 0x1;
/// stub / shim 不可用返回码（`E_NOTIMPL` 的 i32 位模式，0x8000_4001）。
pub const RX_D3D12_E_NOTIMPL: i32 = -2_147_467_263;

/// `RxD3D12InteropExport`（RFC-0001 §4.2.1）：`create` 出参，向 CUDA 侧导出 import 所需事实。
/// Windows x64 布局 = **96 字节**（编译期断言 + 与 C++ `static_assert(sizeof==96)` 双向核对）。
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct InteropExport {
    /// = [`RX_D3D12_ABI_VERSION`]。
    pub abi_version: u32,
    /// = 96（C++ 侧回填，Rust 侧校核）。
    pub struct_size: u32,
    /// committed resource 的 NT HANDLE（caller-owned；import 后须 close，RFC-0001 §4.2.2）。
    pub memory_handle: *mut c_void,
    /// `GetResourceAllocationInfo.SizeInBytes`（CUDA import descriptor size）。
    pub allocation_size: u64,
    /// 逻辑 RGB buffer 字节数 = `render_w * render_h * 3 * 4`（mapped buffer size）。
    pub mapping_size: u64,
    /// 共享 fence 的 NT HANDLE（caller-owned）。
    pub fence_handle: *mut c_void,
    /// adapter LUID（与 `cuDeviceGetLuid` 逐字节相同，RFC-0001 §4.4）。
    pub adapter_luid: [u8; 8],
    pub node_mask: u32,
    pub render_width: u32,
    pub render_height: u32,
    pub window_width: u32,
    pub window_height: u32,
    /// 固定为 3（RGB）。
    pub channels: u32,
    /// 必须为 0。
    pub reserved: [u32; 6],
}
const _: () = assert!(size_of::<InteropExport>() == 96);

impl InteropExport {
    /// 全零（stub / 调用前占位）。
    pub fn zeroed() -> InteropExport {
        // 全字段可零（指针 null、数组 0）。
        InteropExport {
            abi_version: 0,
            struct_size: 0,
            memory_handle: core::ptr::null_mut(),
            allocation_size: 0,
            mapping_size: 0,
            fence_handle: core::ptr::null_mut(),
            adapter_luid: [0; 8],
            node_mask: 0,
            render_width: 0,
            render_height: 0,
            window_width: 0,
            window_height: 0,
            channels: 0,
            reserved: [0; 6],
        }
    }
}

/// 不透明 shim present 句柄（`RxD3D12Present*`）。
#[repr(C)]
pub struct RxD3D12Present {
    _private: [u8; 0],
}

#[cfg(feature = "real-shim")]
unsafe extern "C" {
    fn rx_d3d12_present_create(
        abi_version: u32,
        cuda_luid: *const u8,
        cuda_node_mask: u32,
        render_width: u32,
        render_height: u32,
        window_width: u32,
        window_height: u32,
        flags: u32,
        out_present: *mut *mut RxD3D12Present,
        out_export: *mut InteropExport,
    ) -> i32;
    fn rx_d3d12_present_pump(present: *mut RxD3D12Present, out_should_close: *mut u32) -> i32;
    fn rx_d3d12_present_submit(
        present: *mut RxD3D12Present,
        cuda_done_value: u64,
        d3d_done_value: u64,
    ) -> i32;
    fn rx_d3d12_present_wait_idle(present: *mut RxD3D12Present) -> i32;
    fn rx_d3d12_close_shared_handle(handle: *mut c_void) -> i32;
    fn rx_d3d12_present_destroy(present: *mut RxD3D12Present);
}

/// D3D12 present 会话（拥有 shim 侧 `RxD3D12Present*`，affine，Drop 销毁）。`!Send + !Sync`
/// （shim 对象固定创建线程，RFC-0001 §4.2.1：裸 `*mut` 字段天然 `!Send`/`!Sync`）。
pub struct Presenter {
    // stub 段所有方法 cfg-out，`raw` 仅 real-shim 段读取/销毁。
    #[cfg_attr(not(feature = "real-shim"), allow(dead_code))]
    raw: *mut RxD3D12Present,
}

impl Presenter {
    /// 创建 present 会话：shim 在与 `cuda_luid` 同 adapter 上建 device/swapchain/共享
    /// resource·fence，回填 [`InteropExport`]。**stub 返回 [`RX_D3D12_E_NOTIMPL`]**。
    pub fn create(
        cuda_luid: [u8; 8],
        cuda_node_mask: u32,
        render: [u32; 2],
        window: [u32; 2],
        flags: u32,
    ) -> Result<(Presenter, InteropExport), i32> {
        #[cfg(feature = "real-shim")]
        {
            let mut raw: *mut RxD3D12Present = core::ptr::null_mut();
            let mut export = InteropExport::zeroed();
            // SAFETY: 出参 `raw`/`export` 为有效可写本地存储；`cuda_luid` 为 8 字节有效只读
            // 数组指针;shim 按版本化 C ABI（RFC-0001 §4.2.1）回填或返回 HRESULT 位码。
            let rc = unsafe {
                rx_d3d12_present_create(
                    RX_D3D12_ABI_VERSION,
                    cuda_luid.as_ptr(),
                    cuda_node_mask,
                    render[0],
                    render[1],
                    window[0],
                    window[1],
                    flags,
                    &mut raw,
                    &mut export,
                )
            };
            if rc != 0 || raw.is_null() {
                return Err(if rc != 0 { rc } else { RX_D3D12_E_NOTIMPL });
            }
            Ok((Presenter { raw }, export))
        }
        #[cfg(not(feature = "real-shim"))]
        {
            let _ = (cuda_luid, cuda_node_mask, render, window, flags);
            Err(RX_D3D12_E_NOTIMPL)
        }
    }

    /// 抽干窗口消息泵；`Ok(true)` = 收到关闭请求（RFC-0001 §4.2.1）。
    pub fn pump(&self) -> Result<bool, i32> {
        #[cfg(feature = "real-shim")]
        {
            let mut should_close: u32 = 0;
            // SAFETY: `self.raw` 为 `create` 成功且未销毁的 shim 句柄;`should_close` 有效可写。
            let rc = unsafe { rx_d3d12_present_pump(self.raw, &mut should_close) };
            if rc != 0 {
                return Err(rc);
            }
            Ok(should_close != 0)
        }
        #[cfg(not(feature = "real-shim"))]
        Err(RX_D3D12_E_NOTIMPL)
    }

    /// 提交本帧 D3D12 present：queue wait `cuda_done` → present pass → Present → queue
    /// signal `d3d_done`（RFC-0001 §4.2.1 / §4.3）。
    pub fn submit(&self, cuda_done_value: u64, d3d_done_value: u64) -> Result<(), i32> {
        #[cfg(feature = "real-shim")]
        {
            // SAFETY: `self.raw` 为有效未销毁 shim 句柄;fence 值由调用方按 §4.3 偶奇协议给出。
            let rc = unsafe { rx_d3d12_present_submit(self.raw, cuda_done_value, d3d_done_value) };
            if rc != 0 { Err(rc) } else { Ok(()) }
        }
        #[cfg(not(feature = "real-shim"))]
        {
            let _ = (cuda_done_value, d3d_done_value);
            Err(RX_D3D12_E_NOTIMPL)
        }
    }

    /// 阻塞至 shim D3D12 queue 空闲（shutdown 前置，RFC-0001 §4.4）。
    pub fn wait_idle(&self) -> Result<(), i32> {
        #[cfg(feature = "real-shim")]
        {
            // SAFETY: `self.raw` 为有效未销毁 shim 句柄。
            let rc = unsafe { rx_d3d12_present_wait_idle(self.raw) };
            if rc != 0 { Err(rc) } else { Ok(()) }
        }
        #[cfg(not(feature = "real-shim"))]
        Err(RX_D3D12_E_NOTIMPL)
    }
}

impl Drop for Presenter {
    fn drop(&mut self) {
        #[cfg(feature = "real-shim")]
        if !self.raw.is_null() {
            // SAFETY: `self.raw` 为 `create` 成功、本类型独占（非 Clone）的 shim 句柄,Drop 仅一次;
            // shim 内部按 RFC-0001 §4.4 释放 fence/resource/queue/swapchain/device/window。
            unsafe { rx_d3d12_present_destroy(self.raw) };
        }
    }
}

/// 关闭 import 后的临时 NT HANDLE（每个 handle 恰好一次，RFC-0001 §4.2.2）。stub：no-op 返回不可用。
///
/// # Safety
///
/// `handle` 必须是 shim `create` 移交、尚未关闭的有效 NT HANDLE。
pub unsafe fn close_shared_handle(handle: *mut c_void) -> i32 {
    #[cfg(feature = "real-shim")]
    {
        // SAFETY: `handle` 为 shim `create` 经 out-export 移交、尚未关闭的 NT HANDLE。
        unsafe { rx_d3d12_close_shared_handle(handle) }
    }
    #[cfg(not(feature = "real-shim"))]
    {
        let _ = handle;
        RX_D3D12_E_NOTIMPL
    }
}

/// 本 build 是否含真实 D3D12 shim（`real-shim` feature）。
pub const fn has_real_shim() -> bool {
    cfg!(feature = "real-shim")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interop_export_abi_layout() {
        // 与 C++ static_assert(sizeof(RxD3D12InteropExport)==96) 双向核对（RFC-0001 §4.2.1）。
        assert_eq!(size_of::<InteropExport>(), 96);
        assert_eq!(align_of::<InteropExport>(), 8);
        // 关键字段偏移（与 C++ 布局对齐）。
        let e = InteropExport::zeroed();
        let base = &e as *const _ as usize;
        assert_eq!((&e.memory_handle as *const _ as usize) - base, 8);
        assert_eq!((&e.allocation_size as *const _ as usize) - base, 16);
        assert_eq!((&e.adapter_luid as *const _ as usize) - base, 40);
        assert_eq!((&e.channels as *const _ as usize) - base, 68);
    }

    #[test]
    fn stub_reports_unavailable_without_real_shim() {
        // 默认 stub：create 返回不可用码（无 D3D12 SDK 环境亦绿）。real-shim build 跳过此断言。
        if !has_real_shim() {
            let r = Presenter::create([0; 8], 0, [2, 2], [2, 2], 0);
            assert_eq!(r.err(), Some(RX_D3D12_E_NOTIMPL));
            assert_eq!(
                unsafe { close_shared_handle(core::ptr::null_mut()) },
                RX_D3D12_E_NOTIMPL
            );
        }
    }
}
