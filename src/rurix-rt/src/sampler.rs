//! 宿主 sampler 状态面(G3.3,RXS-0225;RFC-0013 §4.B2 形态 b)。
//!
//! `SamplerDesc` = 与着色阶段静态属性 `#[sampler(...)]`(spec/shader_stages.md
//! RXS-0224 / rurixc `binding_layout::SamplerState`)**镜像同一状态空间**的宿主运行期
//! 形态。纯 host 类型(`unsafe_code=deny`,零 FFI):字段经 [`SamplerDesc::vk_fields`]
//! 降级为 Vulkan `VkSamplerCreateInfo` 的 plain 枚举/标量值(feature `vulkan` 的 vk.rs
//! descriptor 建面 RXS-0230 消费构造真 `VkSamplerCreateInfo`)。
//!
//! **单一事实源**:状态空间语义与 `binding_layout::SamplerState` 一致(两 crate 分立,
//! 无法字面共享类型 → 镜像同一枚举集);wrap-vs-clamp 像素对照(步骤 63 模式⑤)走本形态。
//! `max_anisotropy > 1` 时 device 探测 `samplerAnisotropy`,缺失 → 运行期确定性 Err
//! (RFC-0011 §4.11;**不占 RX 码**,RXS-0193 口径),该探测归 vk 运行时(本 host 面不判)。

/// sampler 过滤模式(min/mag/mip 三合一;RXS-0225,镜像 RXS-0224)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    /// 最近邻(point)。
    Nearest,
    /// 线性。
    Linear,
}

/// sampler 寻址模式(RXS-0225)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Address {
    /// clamp-to-edge。
    Clamp,
    /// wrap / repeat。
    Wrap,
    /// mirror。
    Mirror,
    /// border(色限三预置)。
    Border,
}

/// 比较函数(仅比较采样 `SamplerCmp`;RXS-0225)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compare {
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

/// 宿主 sampler 状态对象(RXS-0225)。`lod_bias` 钳 [-16,16)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SamplerDesc {
    /// 过滤模式(min/mag/mip 三合一)。
    pub filter: Filter,
    /// 寻址模式(UVW 三向同值,首期)。
    pub address: Address,
    /// 各向异性(1=off;>1 时 device 探测 `samplerAnisotropy`)。
    pub max_anisotropy: u32,
    /// LOD bias(钳 [-16,16))。
    pub lod_bias: f32,
    /// min LOD。
    pub min_lod: f32,
    /// max LOD。
    pub max_lod: f32,
    /// 比较函数(`SamplerCmp` 用;`None` = 普通采样)。
    pub compare: Option<Compare>,
}

impl Default for SamplerDesc {
    /// 现行静态默认(linear + clamp,RXS-0176 DS4 向后一致)。
    fn default() -> Self {
        SamplerDesc {
            filter: Filter::Linear,
            address: Address::Clamp,
            max_anisotropy: 1,
            lod_bias: 0.0,
            min_lod: 0.0,
            max_lod: f32::MAX,
            compare: None,
        }
    }
}

/// `VkSamplerCreateInfo` 的 plain 字段降级(RXS-0225 IR1)。vk.rs descriptor 建面
/// (feature `vulkan`,RXS-0230)以这些值构造真 `VkSamplerCreateInfo`——本 host 面不
/// 触 FFI(`unsafe_code=deny`),仅确定性映射枚举/标量。数值取 Vulkan 规范既定常量。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VkSamplerFields {
    /// `VkFilter`(min/mag):NEAREST=0 / LINEAR=1。
    pub mag_min_filter: u32,
    /// `VkSamplerMipmapMode`:NEAREST=0 / LINEAR=1。
    pub mipmap_mode: u32,
    /// `VkSamplerAddressMode`(UVW 同值):REPEAT=0 / MIRRORED_REPEAT=1 / CLAMP_TO_EDGE=2 / CLAMP_TO_BORDER=3。
    pub address_mode: u32,
    /// `mipLodBias`。
    pub mip_lod_bias: f32,
    /// `anisotropyEnable`(VkBool32:>1 → 1)。
    pub anisotropy_enable: u32,
    /// `maxAnisotropy`。
    pub max_anisotropy: f32,
    /// `compareEnable`(VkBool32:比较型 → 1)。
    pub compare_enable: u32,
    /// `VkCompareOp`:NEVER=0 / LESS=1 / LESS_OR_EQUAL=3 / GREATER=4 / GREATER_OR_EQUAL=5。
    pub compare_op: u32,
    /// `minLod`。
    pub min_lod: f32,
    /// `maxLod`。
    pub max_lod: f32,
}

impl SamplerDesc {
    /// 状态合法性(RXS-0225 Legality:`lod_bias` 钳 [-16,16)、`max_anisotropy` ≥ 1)。
    /// 非法状态组合走库层 `Result`(运行期确定性失败,RXS-0193);本函数为 host 侧预校验。
    pub fn is_valid(&self) -> bool {
        (-16.0..16.0).contains(&self.lod_bias) && self.max_anisotropy >= 1
    }

    /// 降级为 `VkSamplerCreateInfo` 的 plain 字段(RXS-0225 IR1;确定性)。
    pub fn vk_fields(&self) -> VkSamplerFields {
        let filter = match self.filter {
            Filter::Nearest => 0,
            Filter::Linear => 1,
        };
        // mipmap mode 与 min/mag filter 同枚举位(nearest/linear)。
        let address_mode = match self.address {
            Address::Wrap => 0,   // VK_SAMPLER_ADDRESS_MODE_REPEAT
            Address::Mirror => 1, // MIRRORED_REPEAT
            Address::Clamp => 2,  // CLAMP_TO_EDGE
            Address::Border => 3, // CLAMP_TO_BORDER
        };
        let (compare_enable, compare_op) = match self.compare {
            None => (0, 0),                        // NEVER
            Some(Compare::Less) => (1, 1),         // LESS
            Some(Compare::LessEqual) => (1, 3),    // LESS_OR_EQUAL
            Some(Compare::Greater) => (1, 4),      // GREATER
            Some(Compare::GreaterEqual) => (1, 5), // GREATER_OR_EQUAL
        };
        VkSamplerFields {
            mag_min_filter: filter,
            mipmap_mode: filter,
            address_mode,
            mip_lod_bias: self.lod_bias,
            anisotropy_enable: u32::from(self.max_anisotropy > 1),
            max_anisotropy: self.max_anisotropy as f32,
            compare_enable,
            compare_op,
            min_lod: self.min_lod,
            max_lod: self.max_lod,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RXS-0225:宿主 SamplerDesc 状态空间 + `VkSamplerCreateInfo` 降级 + aniso 探测标注。
    #[test]
    fn sampler_desc_maps_to_vk_fields() {
        //@ spec: RXS-0225
        // 默认 linear+clamp → VK LINEAR(1) + CLAMP_TO_EDGE(2),无比较。
        let d = SamplerDesc::default();
        assert!(d.is_valid());
        let f = d.vk_fields();
        assert_eq!(f.mag_min_filter, 1, "linear → VK_FILTER_LINEAR");
        assert_eq!(f.address_mode, 2, "clamp → CLAMP_TO_EDGE");
        assert_eq!(f.compare_enable, 0, "普通采样无比较");
        assert_eq!(f.anisotropy_enable, 0, "max_anisotropy=1 → aniso off");

        // wrap-vs-clamp 对照(步骤 63 模式⑤走本形态):address_mode 必异。
        let wrap = SamplerDesc {
            address: Address::Wrap,
            ..SamplerDesc::default()
        };
        assert_ne!(
            wrap.vk_fields().address_mode,
            d.vk_fields().address_mode,
            "wrap vs clamp address_mode 必异"
        );

        // SamplerCmp 比较型 → compareEnable=1 + LESS_OR_EQUAL(3)。
        let cmp = SamplerDesc {
            compare: Some(Compare::LessEqual),
            ..SamplerDesc::default()
        };
        let cf = cmp.vk_fields();
        assert_eq!(cf.compare_enable, 1);
        assert_eq!(cf.compare_op, 3, "LessEqual → VK_COMPARE_OP_LESS_OR_EQUAL");

        // aniso>1 → anisotropyEnable=1(device 探测 samplerAnisotropy 归 vk 运行时)。
        let aniso = SamplerDesc {
            max_anisotropy: 8,
            ..SamplerDesc::default()
        };
        assert_eq!(aniso.vk_fields().anisotropy_enable, 1);

        // 非法 lod_bias 越界 → is_valid 拒(运行期确定性失败,RXS-0193,不占 RX 码)。
        let bad = SamplerDesc {
            lod_bias: 32.0,
            ..SamplerDesc::default()
        };
        assert!(!bad.is_valid());
    }
}
