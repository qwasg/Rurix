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

/// GPU 产物变体类别(RXS-0150;RXS-0209 加 `Spirv`)。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ArtifactKind {
    /// PTX 文本(保守 fallback 变体,前向兼容兜底;`cuModuleLoadDataEx` JIT 装载)。
    Ptx,
    /// 按架构预编 cubin(`ptxas -arch=sm_xx`;`cuModuleLoadData` 二进制装载,首启免 JIT)。
    Cubin,
    /// fatbin 容器(多变体打包;本期以变体集表达,真 NVIDIA fatbinary 容器格式 defer RD-010)。
    Fatbin,
    /// Vulkan 可移植 device 产物(SPIR-V 字节;驱动 JIT 装载,占「可移植槽」,RXS-0209)。
    /// 与 `Ptx` 是同一可移植槽的两个厂商实现(NVIDIA=PTX / Vulkan=SPIR-V)。
    Spirv,
}

impl ArtifactKind {
    /// lockfile `[[artifact]]` `kind` 字段字面量(RXS-0152;字典序确定)。
    pub fn as_str(self) -> &'static str {
        match self {
            ArtifactKind::Ptx => "ptx",
            ArtifactKind::Cubin => "cubin",
            ArtifactKind::Fatbin => "fatbin",
            ArtifactKind::Spirv => "spirv",
        }
    }
}

/// device 产物架构键(RXS-0209;泛化自 G1.5 `SmTarget`)。NVIDIA `sm_89`(cubin AOT)/
/// AMD `gfx1100`(hsaco AOT)/ 可移植槽(驱动 JIT:Vulkan SPIR-V 或 NVPTX PTX)。
///
/// 语义(RFC-0011 §4.8):NVIDIA 模型 PTX=可移植 JIT fallback / cubin=per-arch AOT;
/// Vulkan 世界 **SPIR-V 占可移植槽**(驱动 JIT),`gfxNNNN` AOT(AMD hsaco)占 per-arch 槽。
/// `SpirvPortable` 与 `Ptx` 是同一「可移植槽」的两个厂商实现。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArchKey {
    /// NVIDIA compute capability,per-arch AOT cubin(`"sm_89"`)。
    Sm(String),
    /// AMD GCN/RDNA ISA,per-arch AOT hsaco(`"gfx1100"`)。
    Gfx(String),
    /// 可移植槽(无 per-arch 键;lock `sm_target` = `""`;Vulkan SPIR-V 装载)。
    SpirvPortable,
}

impl ArchKey {
    /// 由 device compute capability(`cuDeviceGetAttribute` major/minor)构造架构键
    /// (承 RXS-0151 既有语义,零漂移:恒产 `Sm(sm_xx)`)。
    pub fn from_capability(major: u32, minor: u32) -> Self {
        ArchKey::Sm(format!("sm_{major}{minor}"))
    }

    /// prefix-dispatch 解析:`sm_<digits>` → `Sm` / `gfx<alnum>` → `Gfx` / `""` →
    /// `SpirvPortable`;其余形态 → `None`(装载协商降级,非致命)。
    pub fn parse(s: &str) -> Option<Self> {
        if s.is_empty() {
            return Some(ArchKey::SpirvPortable);
        }
        if let Some(d) = s.strip_prefix("sm_") {
            return (!d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
                .then(|| ArchKey::Sm(s.to_owned()));
        }
        if let Some(d) = s.strip_prefix("gfx") {
            return (!d.is_empty() && d.bytes().all(|b| b.is_ascii_alphanumeric()))
                .then(|| ArchKey::Gfx(s.to_owned()));
        }
        None
    }

    /// lock `sm_target` 字面量:`Sm`/`Gfx` 回其键;`SpirvPortable` → `""`。
    pub fn as_str(&self) -> &str {
        match self {
            ArchKey::Sm(s) | ArchKey::Gfx(s) => s,
            ArchKey::SpirvPortable => "",
        }
    }
}

/// 单个 per-arch AOT 预编变体(RXS-0150;RXS-0209 键泛化 `SmTarget→ArchKey`)。字段
/// 语义扩为「per-arch AOT 变体键」:现承 `Sm`(NVIDIA cubin)、后承 `Gfx`(AMD hsaco);
/// 为省 churn 保留 `CubinVariant` 名。`bytes` 由 build.rs 经 `ptxas` 预编嵌入。
#[derive(Clone, Debug)]
pub struct CubinVariant {
    sm: ArchKey,
    bytes: Vec<u8>,
}

impl CubinVariant {
    /// per-arch AOT 架构键(`Sm`/`Gfx`)。
    pub fn sm(&self) -> &ArchKey {
        &self.sm
    }

    /// 预编 cubin 字节(`cuModuleLoadData` 装载输入)。
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// v2 按名索引 SPIR-V 入口(RXS-0292;artifacts 描述表 v2 表项的运行时形态):
/// 入口名 = PTX launch 同名内核标识(codegen `Body.symbol` 同源),`stage_tag` =
/// `ShaderStage` 枚举声明序(0..=10,单一事实源在编译器侧 ast.rs),`spv` = 该入口
/// 独立 SPIR-V 模块小端字节(每入口一模块,无合并/链接器,RXS-0291)。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpirvEntry {
    name: String,
    stage_tag: u32,
    spv: Vec<u8>,
}

impl SpirvEntry {
    /// 构造按名索引 SPIR-V 入口(畸形判据 — 空名/越界 stage_tag — 已在 artifacts
    /// 描述表解析侧确定性拒绝,RXS-0290;本构造不重复校验)。
    pub fn new(name: impl Into<String>, stage_tag: u32, spv: Vec<u8>) -> Self {
        SpirvEntry {
            name: name.into(),
            stage_tag,
            spv,
        }
    }

    /// 入口名(PTX launch 同名内核标识)。
    pub fn name(&self) -> &str {
        &self.name
    }

    /// `ShaderStage` 枚举声明序(0=Vertex / 1=Fragment / 2=Compute / …,RXS-0290)。
    pub fn stage_tag(&self) -> u32 {
        self.stage_tag
    }

    /// 独立 SPIR-V 模块小端字节(`vkCreateShaderModule` 直接消费形态)。
    pub fn spv(&self) -> &[u8] {
        &self.spv
    }
}

/// 分发产物变体集(RXS-0150;RXS-0209 加 Vulkan 可移植槽):PTX fallback 必存 + 按架构
/// AOT 变体(可空)+ SPIR-V 可移植槽(可空)。两个可移植槽(`ptx_fallback` NVIDIA JIT /
/// `spirv_fallback` Vulkan JIT)平行,per-arch AOT 未命中时按目标厂商择一降级。
/// RXS-0292 加 v2 按名索引 SPIR-V 入口集合(可空;仅 Vulkan 通道消费,CUDA-only
/// 进程不触碰)。
#[derive(Clone, Debug)]
pub struct DeviceArtifactSet {
    ptx_fallback: String,
    spirv_fallback: Option<Vec<u8>>,
    cubin_variants: Vec<CubinVariant>,
    spirv_entries: Vec<SpirvEntry>,
}

impl DeviceArtifactSet {
    /// 构造:**PTX fallback 必存**(保守兜底,RXS-0150);cubin 变体 + SPIR-V 槽初始为空
    /// (NVIDIA-only 集,行为逐字节等价 G1.5)。
    pub fn new(ptx_fallback: impl Into<String>) -> Self {
        DeviceArtifactSet {
            ptx_fallback: ptx_fallback.into(),
            spirv_fallback: None,
            cubin_variants: Vec::new(),
            spirv_entries: Vec::new(),
        }
    }

    /// 追加按架构预编 AOT 变体;架构键唯一(同 `sm` 覆盖,保持 builder 幂等)。
    #[must_use]
    pub fn with_cubin(mut self, sm: ArchKey, bytes: Vec<u8>) -> Self {
        if let Some(existing) = self.cubin_variants.iter_mut().find(|v| v.sm == sm) {
            existing.bytes = bytes;
        } else {
            self.cubin_variants.push(CubinVariant { sm, bytes });
        }
        self
    }

    /// 追加 Vulkan 可移植槽(SPIR-V 字节;驱动 JIT,RXS-0209)。不动 NV `ptx_fallback`
    /// 构造签名——平行加槽。
    #[must_use]
    pub fn with_spirv_fallback(mut self, spv: Vec<u8>) -> Self {
        self.spirv_fallback = Some(spv);
        self
    }

    /// PTX fallback 文本(始终存在,保守兜底前向兼容)。
    pub fn ptx_fallback(&self) -> &str {
        &self.ptx_fallback
    }

    /// SPIR-V 可移植槽字节(`None` = 无 Vulkan 可移植产物,RXS-0209)。
    pub fn spirv_fallback(&self) -> Option<&[u8]> {
        self.spirv_fallback.as_deref()
    }

    /// 是否有按架构预编 AOT 变体(否 → 仅可移植槽路径,等价 M8 PTX-only)。
    pub fn has_cubin(&self) -> bool {
        !self.cubin_variants.is_empty()
    }

    /// 按架构键查命中预编 AOT 变体(`None` = 未命中,装载协商降级可移植槽,RXS-0151)。
    pub fn cubin_for(&self, sm: &ArchKey) -> Option<&CubinVariant> {
        self.cubin_variants.iter().find(|v| &v.sm == sm)
    }

    /// 追加 v2 按名索引 SPIR-V 入口集合(RXS-0292,加性;既有 `spirv_fallback` 槽与
    /// 全部既有访问器 0-byte)。入口名唯一:与既有表项或批内重名 → 确定性 `Err`
    /// (重名 = 畸形,与 artifacts 描述表解析同族判据,RXS-0290),不自覆盖、不部分
    /// 追加;空批 = no-op。仅 Vulkan 通道消费(RXS-0293),CUDA-only 进程不触碰。
    pub fn with_spirv_entries(
        mut self,
        entries: impl IntoIterator<Item = SpirvEntry>,
    ) -> Result<Self, String> {
        for entry in entries {
            if self.spirv_entries.iter().any(|e| e.name == entry.name) {
                return Err(format!(
                    "duplicate SPIR-V entry name `{}` (RXS-0292)",
                    entry.name
                ));
            }
            self.spirv_entries.push(entry);
        }
        Ok(self)
    }

    /// 按入口名查 v2 SPIR-V 入口(`None` = 未命中;按名索引,RXS-0292)。
    pub fn spirv_entry(&self, name: &str) -> Option<&SpirvEntry> {
        self.spirv_entries.iter().find(|e| e.name == name)
    }

    /// v2 SPIR-V 入口表(追加序;空 = 无 artifacts v2 SPIR-V 产物,既有面 0-byte)。
    pub fn spirv_entries(&self) -> &[SpirvEntry] {
        &self.spirv_entries
    }

    /// 已预编 AOT 架构键集(字典序确定,供 lockfile `[[artifact]]` 与诊断消费)。
    pub fn cubin_targets(&self) -> Vec<&ArchKey> {
        let mut targets: Vec<&ArchKey> = self.cubin_variants.iter().map(CubinVariant::sm).collect();
        targets.sort();
        targets
    }
}

/// 装载变体决策(RXS-0151;RXS-0209 加 SPIR-V 可移植槽臂)。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadChoice {
    /// 命中按架构预编 AOT 变体(`cuModuleLoadData`,首启免 JIT)。
    Cubin(ArchKey),
    /// 降级 Vulkan 可移植槽(SPIR-V 驱动 JIT 装载,RXS-0209)。
    SpirvPortable,
    /// 降级保守 PTX fallback(既有 PTX 版号梯子 `cuModuleLoadDataEx`,RXS-0076/0077 语义 0-byte)。
    PtxFallback,
}

/// fatbin 装载协商决策(RXS-0151,**纯函数**,host 可测,不触 device)。
///
/// 协商序:命中 `device_key` 架构的预编 AOT 变体 → [`LoadChoice::Cubin`];未命中且存在
/// SPIR-V 可移植槽 → [`LoadChoice::SpirvPortable`](RXS-0209);否则 →
/// [`LoadChoice::PtxFallback`](既有 PTX 装载协商,RXS-0076/0077)。**NVIDIA 零回归**:
/// NV-only 集 `spirv_fallback = None` → 未命中恒回 `PtxFallback`(逐字节等价 G1.5)。
/// 装载期 AOT 被驱动拒绝时由 [`Context::load_module`](crate::Context::load_module) 降级
/// 可移植槽重试(降级而非 reject,不 poison context),与本决策一致。
pub fn select_load_variant(device_key: &ArchKey, set: &DeviceArtifactSet) -> LoadChoice {
    match set.cubin_for(device_key) {
        Some(variant) => LoadChoice::Cubin(variant.sm().clone()),
        None if set.spirv_fallback().is_some() => LoadChoice::SpirvPortable,
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

        // 按架构预编 AOT 变体:架构键唯一查命中。
        let sm89 = ArchKey::from_capability(8, 9);
        assert_eq!(sm89.as_str(), "sm_89");
        let set = ptx_only
            .with_cubin(sm89.clone(), vec![0xDE, 0xAD])
            .with_cubin(ArchKey::parse("sm_90").unwrap(), vec![0xBE, 0xEF]);
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
        assert!(ArchKey::parse("compute_89").is_none());
        assert!(ArchKey::parse("sm_").is_none());
    }

    //@ spec: RXS-0151
    #[test]
    fn load_negotiation_prefers_cubin_then_falls_back() {
        let sm89 = ArchKey::from_capability(8, 9);
        let set = DeviceArtifactSet::new(".version 8.0\n").with_cubin(sm89.clone(), vec![0xCB]);

        // 命中:device key 匹配预编 AOT → Cubin(首启免 JIT)。
        assert_eq!(
            select_load_variant(&sm89, &set),
            LoadChoice::Cubin(sm89.clone())
        );

        // 未命中:device key 无匹配 AOT → 降级保守 PTX fallback(RXS-0076/0077,降级而非 reject)。
        let sm75 = ArchKey::from_capability(7, 5);
        assert_eq!(select_load_variant(&sm75, &set), LoadChoice::PtxFallback);

        // 无 AOT 变体集(等价 M8 PTX-only)→ 任何 device 都降级 PTX。
        let ptx_only = DeviceArtifactSet::new(".version 8.0\n");
        assert_eq!(
            select_load_variant(&sm89, &ptx_only),
            LoadChoice::PtxFallback
        );
    }

    //@ spec: RXS-0209
    #[test]
    fn artifact_kind_and_archkey_spirv_generalization() {
        // ArtifactKind 加性:Spirv 变体(Vulkan 可移植 device 产物)。
        assert_eq!(ArtifactKind::Spirv.as_str(), "spirv");

        // ArchKey prefix-dispatch 解析(泛化自 G1.5 SmTarget):
        // `sm_<digits>` → Sm(NVIDIA cubin AOT)。
        assert_eq!(
            ArchKey::parse("sm_89"),
            Some(ArchKey::Sm("sm_89".to_owned()))
        );
        assert_eq!(
            ArchKey::from_capability(9, 0),
            ArchKey::Sm("sm_90".to_owned())
        );
        // `gfx<alnum>` → Gfx(AMD hsaco AOT;正是 G1.5 SmTarget 会误拒的形态)。
        assert_eq!(
            ArchKey::parse("gfx1100"),
            Some(ArchKey::Gfx("gfx1100".to_owned()))
        );
        // `""` → SpirvPortable(可移植槽,无 per-arch 键)。
        assert_eq!(ArchKey::parse(""), Some(ArchKey::SpirvPortable));
        // 非法前缀 → None(装载协商降级,非致命)。
        assert!(ArchKey::parse("compute_89").is_none());
        assert!(ArchKey::parse("sm_").is_none());
        assert!(ArchKey::parse("gfx").is_none());

        // as_str():Sm/Gfx 回其键;SpirvPortable → ""(lock sm_target 可移植槽空)。
        assert_eq!(ArchKey::Sm("sm_89".to_owned()).as_str(), "sm_89");
        assert_eq!(ArchKey::Gfx("gfx1100".to_owned()).as_str(), "gfx1100");
        assert_eq!(ArchKey::SpirvPortable.as_str(), "");

        // DeviceArtifactSet SPIR-V 可移植槽 builder/accessor(不动 NV ptx_fallback 字节)。
        let nv_only = DeviceArtifactSet::new(".version 8.0\n");
        assert!(nv_only.spirv_fallback().is_none()); // NV-only 集无 SPIR-V 槽。
        let vk_set = nv_only.with_spirv_fallback(vec![0x03, 0x02, 0x23, 0x07]);
        assert_eq!(vk_set.spirv_fallback(), Some(&[0x03, 0x02, 0x23, 0x07][..]));
        assert_eq!(vk_set.ptx_fallback(), ".version 8.0\n"); // NV 可移植槽字节不动。

        // select_load_variant SPIR-V 兜底臂(RXS-0209):per-arch AOT 未命中 + 有 SPIR-V 槽
        // → SpirvPortable;NV-only 集(无 SPIR-V 槽)未命中恒 PtxFallback(零回归)。
        let amd_set = DeviceArtifactSet::new(".version 8.0\n")
            .with_cubin(ArchKey::Gfx("gfx1100".to_owned()), vec![0xAA])
            .with_spirv_fallback(vec![0x03, 0x02, 0x23, 0x07]);
        // sm_89(NVIDIA)不匹配 gfx1100 AOT → 降级 SPIR-V 可移植槽。
        assert_eq!(
            select_load_variant(&ArchKey::from_capability(8, 9), &amd_set),
            LoadChoice::SpirvPortable
        );
        // gfx1100 命中 per-arch AOT → Cubin(hsaco 槽)。
        assert_eq!(
            select_load_variant(&ArchKey::Gfx("gfx1100".to_owned()), &amd_set),
            LoadChoice::Cubin(ArchKey::Gfx("gfx1100".to_owned()))
        );
        // NV-only 集未命中 → PtxFallback(spirv_fallback=None,逐字节等价 G1.5)。
        let nv_set = DeviceArtifactSet::new(".version 8.0\n")
            .with_cubin(ArchKey::from_capability(8, 9), vec![0xCB]);
        assert_eq!(
            select_load_variant(&ArchKey::from_capability(7, 5), &nv_set),
            LoadChoice::PtxFallback
        );
    }

    /// v2 按名索引 SPIR-V 入口集合(RXS-0292):插入/按名查找/重名确定性拒;
    /// 既有槽(`spirv_fallback`/cubin/ptx)与既有访问器 0-byte。
    //@ spec: RXS-0292
    #[test]
    fn with_spirv_entries_name_indexed_and_duplicates_rejected() {
        // 空集(既有形态):v2 入口表空、按名查未命中;既有访问器不受影响(0-byte)。
        let base = DeviceArtifactSet::new(".version 8.0\n").with_spirv_fallback(vec![0x01]);
        assert!(base.spirv_entries().is_empty());
        assert!(base.spirv_entry("rx_k_1").is_none());

        // 按名索引插入 + 查找命中(名字/stage_tag/模块字节逐项回读)。
        let set = base
            .with_spirv_entries([
                SpirvEntry::new("rx_vs_5", 0, vec![0x03, 0x02, 0x23, 0x07]),
                SpirvEntry::new("rx_k_3", 2, vec![0xAA, 0xBB]),
            ])
            .expect("两不重合名插入");
        assert_eq!(set.spirv_entries().len(), 2);
        let vs = set.spirv_entry("rx_vs_5").expect("vertex 入口命中");
        assert_eq!(vs.stage_tag(), 0);
        assert_eq!(vs.spv(), &[0x03, 0x02, 0x23, 0x07]);
        let k = set.spirv_entry("rx_k_3").expect("compute 入口命中");
        assert_eq!(k.stage_tag(), 2);
        assert_eq!(k.spv(), &[0xAA, 0xBB]);
        assert!(set.spirv_entry("rx_fs_7").is_none());
        // 既有槽 0-byte:spirv_fallback / ptx_fallback / 装载协商语义不变。
        assert_eq!(set.spirv_fallback(), Some(&[0x01][..]));
        assert_eq!(set.ptx_fallback(), ".version 8.0\n");
        assert_eq!(
            select_load_variant(&ArchKey::from_capability(7, 5), &set),
            LoadChoice::SpirvPortable
        );

        // 重名 = 畸形确定性拒(与既有表项重;不部分追加)。
        let dup_existing = DeviceArtifactSet::new(".version 8.0\n")
            .with_spirv_entries([SpirvEntry::new("rx_k_3", 2, vec![0x01])])
            .expect("首次插入")
            .with_spirv_entries([SpirvEntry::new("rx_k_3", 2, vec![0x02])]);
        assert!(dup_existing.is_err(), "与既有表项重名须确定性拒");
        // 批内重名同样拒。
        let dup_in_batch = DeviceArtifactSet::new(".version 8.0\n").with_spirv_entries([
            SpirvEntry::new("rx_a_1", 2, vec![0x01]),
            SpirvEntry::new("rx_a_1", 0, vec![0x02]),
        ]);
        assert!(dup_in_batch.is_err(), "批内重名须确定性拒");
        // 空批 = no-op。
        let noop = DeviceArtifactSet::new(".version 8.0\n")
            .with_spirv_entries([])
            .expect("空批 no-op");
        assert!(noop.spirv_entries().is_empty());
    }
}
