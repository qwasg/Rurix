//! device 腿(feature `taichi-tirt`;spec §4.E3 四段闭合,device 见证):
//! TiRT Vulkan AOT kernel `fill_particles` 在渲染设备上下文(并行 Vk 设备,
//! RFC-0017 §4.E2)launch → NdArray `ti_export_vulkan_memory` 导出 VkBuffer →
//! host 侧 graph external import copy 计划接线对拍 → readback 非零 + 契约值
//! 逐位断言。Err 由调用方按 RURIX_REQUIRE_REAL / provisioning 缺失纪律裁决
//! 红或 SKIP(真失败/断言失败永远硬红,逐字仿 uc08)。

use rurix_rt::tirt::{self, TirtError};

use crate::host::{PARTICLE_BYTES, PARTICLE_COUNT};

/// device 腿结果(JSON 冻结面;`first_values` = readback 前 ≤4 个 f32)。
pub struct DeviceLeg {
    /// 物理设备名(`vkGetPhysicalDeviceProperties.deviceName`)。
    pub device_name: String,
    /// 粒子数(= ndarray shape = 读回 f32 个数)。
    pub particle_count: u32,
    /// 读回非零元素个数。
    pub nonzero_count: u32,
    /// `ti_export_vulkan_memory` 报告的导出 buffer 字节数。
    pub exported_buffer_size: u64,
    /// 读回前 ≤4 个元素值(契约 p[i] = i*1.5+1.0 抽查)。
    pub first_values: Vec<f32>,
    /// device 断言面(§4.E3 四段闭合 + 值域逐位契约;字段名冻结)。
    pub asserts: Vec<(String, bool)>,
}

impl DeviceLeg {
    /// device 断言全过。
    pub fn asserts_pass(&self) -> bool {
        self.asserts.iter().all(|(_, ok)| *ok)
    }
}

/// 契约期望值:p[i] = i*1.5+1.0(i ∈ [0,n);全部 f32 精确可表,逐位比较可行——
/// i ≤ 63 时 i*1.5 为 0.5 的整数倍 ≤ 94.5,+1.0 后仍 ≤ 95.5,二进制浮点无损)。
fn expected_values(n: usize) -> Vec<f32> {
    (0..n).map(|i| i as f32 * 1.5 + 1.0).collect()
}

/// §4.E3 四段闭合真跑(库面全 safe;tcm 字节 = host 腿核验过的资产本体)。
pub fn run_device_leg(tcm: &[u8], graph_copy_byte_size: u64) -> Result<DeviceLeg, TirtError> {
    let out = tirt::run_particles_spike(tcm, PARTICLE_COUNT)?;
    // ④b 值域逐位契约先行计算(前 4 值与 p[i] = i*1.5+1.0 逐位相等,宁严勿宽)。
    let first_exact =
        out.first_values.len() == 4 && out.first_values == expected_values(out.first_values.len());

    let asserts: Vec<(String, bool)> = vec![
        // ① launch 成功:到达此处 = import runtime → alloc → create module →
        //   get kernel → ti_launch_kernel → ti_flush+ti_wait 全链 Ok。
        ("device_launch_ok".into(), true),
        // ② NdArray 导出 VkBuffer:导出尺寸 == 64×f32 = 256B。
        (
            "device_buffer_exported".into(),
            out.exported_buffer_size == PARTICLE_BYTES,
        ),
        // ③ graph external import 消费接线:host 计划 copy byte_size == 导出尺寸
        //   (graph 侧 import 的 `taichi_particles` 与该导出 VkBuffer 同一对象)。
        (
            "device_graph_copy_wired".into(),
            graph_copy_byte_size == out.exported_buffer_size,
        ),
        // ④ readback 非零(device 见证):64/64 元素非零。
        (
            "device_readback_nonzero".into(),
            out.nonzero_count == PARTICLE_COUNT,
        ),
        // ④b 值域逐位契约。
        ("device_first_values_exact".into(), first_exact),
    ];

    Ok(DeviceLeg {
        device_name: out.device_name,
        particle_count: out.particle_count,
        nonzero_count: out.nonzero_count,
        exported_buffer_size: out.exported_buffer_size,
        first_values: out.first_values,
        asserts,
    })
}

/// provisioning/环境缺失分类(裁决面,逐字仿 uc08「仅 loader 缺失可降级」):
/// 缺 `taichi_c_api.dll`(未设环境变量/路径不存在/装载失败/符号缺失)或无 Vulkan
/// 设备 → 非 REQUIRE_REAL 时可 SKIP 降级;其余变体(TaichiError 运行时错 /
/// BufferExport 导出面失败 / ReadbackMismatch 读回异常)= 真失败,永远硬红。
pub fn is_provisioning_missing(e: &TirtError) -> bool {
    matches!(
        e,
        TirtError::DllNotFound(_) | TirtError::DeviceUnavailable(_)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: 「spike 成功判据」裁决面(provisioning 缺失 ⇔ 可降级;真失败 ⇔ 硬红)
    #[test]
    fn provisioning_classification() {
        assert!(is_provisioning_missing(&TirtError::DllNotFound("x".into())));
        assert!(is_provisioning_missing(&TirtError::DeviceUnavailable(
            "x".into()
        )));
        assert!(!is_provisioning_missing(&TirtError::TaichiError(
            "x".into()
        )));
        assert!(!is_provisioning_missing(&TirtError::BufferExport(
            "x".into()
        )));
        assert!(!is_provisioning_missing(&TirtError::ReadbackMismatch(
            "x".into()
        )));
    }

    //@ spec: AOT 资产契约值域(f32 精确可表,逐位断言基准)
    #[test]
    fn expected_values_match_contract() {
        assert_eq!(expected_values(4), vec![1.0, 2.5, 4.0, 5.5]);
        assert_eq!(expected_values(1), vec![1.0]);
        // 末元素:63*1.5+1.0 = 95.5(精确可表)。
        assert_eq!(expected_values(64)[63], 95.5);
    }
}
