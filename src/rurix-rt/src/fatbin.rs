//! 生产分发 fatbin:分发产物变体模型 + 装载协商决策(G1.5,Mini-RFC/MR-0005,RXS-0150/0151)。
//!
//! 把 device codegen 分发从 M8 PTX-only 开发期形态推进到「按架构预编 cubin + 保守 PTX
//! fallback」(07 §7 / D-207)。本模块为**纯 host 类型**(无 FFI / device 依赖):
//!
//! - [`DeviceArtifactSet`]:分发产物变体集——**PTX fallback 必存**(保守兜底,RXS-0150)+
//!   按架构(`sm_xx`)预编 cubin 变体(可空,降级时仅 PTX)。
//! - [`select_load_variant`]:装载协商决策(RXS-0151,纯函数)——cubin 命中即用、未命中降级
//!   PTX fallback;**降级而非 reject**,不 poison context。
//!
//! 实际 cubin 二进制装载(`cuModuleLoadData`)与 compute capability 查询
//! (`cuDeviceGetAttribute`)落 [`crate::sys`] / [`Context::load_module`](crate::Context::load_module)
//! 装载协商边界(unsafe-audit U22);默认回归网全覆盖本模块且不依赖 GPU 而绿。

/// GPU 产物变体类别(RXS-0150)。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ArtifactKind {
    /// PTX 文本(保守 fallback 变体,前向兼容兜底;`cuModuleLoadDataEx` JIT 装载)。
    Ptx,
    /// 按架构预编 cubin(`ptxas -arch=sm_xx`;`cuModuleLoadData` 二进制装载,首启免 JIT)。
    Cubin,
    /// fatbin 容器(多变体打包;本期以变体集表达,真 NVIDIA fatbinary 容器格式 defer RD-010)。
    Fatbin,
}

impl ArtifactKind {
    /// lockfile `[[artifact]]` `kind` 字段字面量(RXS-0152;字典序确定)。
    pub fn as_str(self) -> &'static str {
        match self {
            ArtifactKind::Ptx => "ptx",
            ArtifactKind::Cubin => "cubin",
            ArtifactKind::Fatbin => "fatbin",
        }
    }
}

/// cubin 预编架构键(`sm_<major><minor>`,基线 `sm_89`)。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SmTarget(String);

impl SmTarget {
    /// 由 device compute capability(`cuDeviceGetAttribute` major/minor)构造架构键。
    pub fn from_capability(major: u32, minor: u32) -> Self {
        SmTarget(format!("sm_{major}{minor}"))
    }

    /// 解析 `sm_<digits>`(其余形态返回 `None`)。
    pub fn parse(s: &str) -> Option<Self> {
        let digits = s.strip_prefix("sm_")?;
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        Some(SmTarget(s.to_owned()))
    }

    /// 架构键字面量(`sm_89`)。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 单个按架构预编 cubin 变体(RXS-0150;`bytes` 由 build.rs 经 `ptxas` 预编嵌入)。
#[derive(Clone, Debug)]
pub struct CubinVariant {
    sm: SmTarget,
    bytes: Vec<u8>,
}

impl CubinVariant {
    /// 架构键。
    pub fn sm(&self) -> &SmTarget {
        &self.sm
    }

    /// 预编 cubin 字节(`cuModuleLoadData` 装载输入)。
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// 分发产物变体集(RXS-0150):PTX fallback 必存 + 按架构 cubin 变体(可空)。
#[derive(Clone, Debug)]
pub struct DeviceArtifactSet {
    ptx_fallback: String,
    cubin_variants: Vec<CubinVariant>,
}

impl DeviceArtifactSet {
    /// 构造:**PTX fallback 必存**(保守兜底,RXS-0150);cubin 变体初始为空。
    pub fn new(ptx_fallback: impl Into<String>) -> Self {
        DeviceArtifactSet {
            ptx_fallback: ptx_fallback.into(),
            cubin_variants: Vec::new(),
        }
    }

    /// 追加按架构预编 cubin 变体;架构键唯一(同 `sm` 覆盖,保持 builder 幂等)。
    #[must_use]
    pub fn with_cubin(mut self, sm: SmTarget, bytes: Vec<u8>) -> Self {
        if let Some(existing) = self.cubin_variants.iter_mut().find(|v| v.sm == sm) {
            existing.bytes = bytes;
        } else {
            self.cubin_variants.push(CubinVariant { sm, bytes });
        }
        self
    }

    /// PTX fallback 文本(始终存在,保守兜底前向兼容)。
    pub fn ptx_fallback(&self) -> &str {
        &self.ptx_fallback
    }

    /// 是否有按架构预编 cubin(否 → 仅 PTX 路径,等价 M8 PTX-only)。
    pub fn has_cubin(&self) -> bool {
        !self.cubin_variants.is_empty()
    }

    /// 按架构键查命中预编 cubin(`None` = 未命中,装载协商降级 PTX,RXS-0151)。
    pub fn cubin_for(&self, sm: &SmTarget) -> Option<&CubinVariant> {
        self.cubin_variants.iter().find(|v| &v.sm == sm)
    }

    /// 已预编 cubin 架构键集(字典序确定,供 lockfile `[[artifact]]` 与诊断消费)。
    pub fn cubin_targets(&self) -> Vec<&SmTarget> {
        let mut targets: Vec<&SmTarget> =
            self.cubin_variants.iter().map(CubinVariant::sm).collect();
        targets.sort();
        targets
    }
}

/// 装载变体决策(RXS-0151)。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadChoice {
    /// 命中按架构预编 cubin(`cuModuleLoadData`,首启免 JIT)。
    Cubin(SmTarget),
    /// 降级保守 PTX fallback(既有 PTX 版号梯子 `cuModuleLoadDataEx`,RXS-0076/0077 语义 0-byte)。
    PtxFallback,
}

/// fatbin 装载协商决策(RXS-0151,**纯函数**,host 可测,不触 device)。
///
/// 协商序:命中 `device_sm` 架构的预编 cubin → [`LoadChoice::Cubin`];未命中 →
/// [`LoadChoice::PtxFallback`](既有 PTX 装载协商,RXS-0076/0077)。装载期 cubin 被驱动拒绝
/// 时由 [`Context::load_module`](crate::Context::load_module) 降级 PTX 重试(降级而非 reject,
/// 不 poison context),与本决策一致。
pub fn select_load_variant(device_sm: &SmTarget, set: &DeviceArtifactSet) -> LoadChoice {
    match set.cubin_for(device_sm) {
        Some(variant) => LoadChoice::Cubin(variant.sm().clone()),
        None => LoadChoice::PtxFallback,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0150
    #[test]
    fn device_artifact_set_requires_ptx_fallback() {
        // PTX fallback 必存(保守兜底,D-207);cubin 变体可空 → 仅 PTX(等价 M8 PTX-only)。
        let ptx_only = DeviceArtifactSet::new(".version 8.0\n.target sm_89\n");
        assert!(!ptx_only.has_cubin());
        assert_eq!(ptx_only.ptx_fallback(), ".version 8.0\n.target sm_89\n");
        assert!(ptx_only.cubin_targets().is_empty());

        // 按架构预编 cubin 变体:架构键唯一查命中。
        let sm89 = SmTarget::from_capability(8, 9);
        assert_eq!(sm89.as_str(), "sm_89");
        let set = ptx_only
            .with_cubin(sm89.clone(), vec![0xDE, 0xAD])
            .with_cubin(SmTarget::parse("sm_90").unwrap(), vec![0xBE, 0xEF]);
        assert!(set.has_cubin());
        assert_eq!(set.cubin_for(&sm89).unwrap().bytes(), &[0xDE, 0xAD]);
        assert_eq!(
            set.cubin_targets()
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>(),
            vec!["sm_89", "sm_90"],
        );
        // 同架构键覆盖(builder 幂等,变体唯一)。
        let set = set.with_cubin(sm89.clone(), vec![0x01]);
        assert_eq!(set.cubin_for(&sm89).unwrap().bytes(), &[0x01]);
        assert_eq!(set.cubin_targets().len(), 2);

        // kind 字面量(RXS-0152 lockfile [[artifact]] 字段)。
        assert_eq!(ArtifactKind::Ptx.as_str(), "ptx");
        assert_eq!(ArtifactKind::Cubin.as_str(), "cubin");
        assert_eq!(ArtifactKind::Fatbin.as_str(), "fatbin");
        // 非 sm_ 形态拒解析。
        assert!(SmTarget::parse("compute_89").is_none());
        assert!(SmTarget::parse("sm_").is_none());
    }

    //@ spec: RXS-0151
    #[test]
    fn load_negotiation_prefers_cubin_then_falls_back() {
        let sm89 = SmTarget::from_capability(8, 9);
        let set = DeviceArtifactSet::new(".version 8.0\n").with_cubin(sm89.clone(), vec![0xCB]);

        // 命中:device sm 匹配预编 cubin → Cubin(首启免 JIT)。
        assert_eq!(
            select_load_variant(&sm89, &set),
            LoadChoice::Cubin(sm89.clone())
        );

        // 未命中:device sm 无匹配 cubin → 降级保守 PTX fallback(RXS-0076/0077,降级而非 reject)。
        let sm75 = SmTarget::from_capability(7, 5);
        assert_eq!(select_load_variant(&sm75, &set), LoadChoice::PtxFallback);

        // 无 cubin 变体集(等价 M8 PTX-only)→ 任何 device 都降级 PTX。
        let ptx_only = DeviceArtifactSet::new(".version 8.0\n");
        assert_eq!(
            select_load_variant(&sm89, &ptx_only),
            LoadChoice::PtxFallback
        );
    }
}
