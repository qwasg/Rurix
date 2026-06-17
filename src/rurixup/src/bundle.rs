//! 发布 bundle 清单与语言本体 / NVIDIA 再分发组件分离打包(spec/release.md RXS-0136)。
//!
//! 发布 bundle **分区**为「语言本体」([`Partition::LanguageCore`],Rurix 自研编译器 /
//! 运行时 / 标准库,自有许可)与「NVIDIA 再分发组件」([`Partition::NvidiaRedist`],
//! 仅 Attachment A 白名单最小集——MVP 实际只需 `libdevice.10.bc`,cuBLAS 绑定包按需
//! 附带 `cublas64_*.dll`)。**完整 Toolkit / 驱动 / Nsight 永不捆绑**(许可红线 r6):
//! NVIDIA 分区中任一非 Attachment A 白名单组件即审计违例([`audit_redistribution`]),
//! 延续 M5.4 `ci/check_redistribution.py` 口径。

use std::fmt;

/// 发布 bundle 分区(语言本体 ⟂ NVIDIA 再分发组件,RXS-0136)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Partition {
    /// 语言本体:Rurix 自研编译器 / 运行时 / 标准库(自有许可)。
    LanguageCore,
    /// NVIDIA 再分发组件:仅 Attachment A 白名单最小集(完整 Toolkit/驱动/Nsight 永不捆绑)。
    NvidiaRedist,
}

impl Partition {
    /// 稳定字符串标签(SBOM / 清单序列化用)。
    pub fn label(self) -> &'static str {
        match self {
            Partition::LanguageCore => "language-core",
            Partition::NvidiaRedist => "nvidia-redist",
        }
    }
}

impl fmt::Display for Partition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// 单个分发组件(干名 + 版本 + 许可标识 + 分区 + content SHA-256 十六进制)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    /// 产物干名(如 `rurixc.exe` / `libdevice.10.bc` / `cublas64_12.dll`)。
    pub name: String,
    /// 版本号(语言本体组件随同一版号原子分发,RXS-0135)。
    pub version: String,
    /// 许可标识(SPDX license id 或再分发条款标签)。
    pub license: String,
    /// 所属分区。
    pub partition: Partition,
    /// 组件内容 SHA-256 十六进制(64 字符)。
    pub sha256: String,
}

/// 发布 bundle 清单:同一版号下的全部分发组件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleManifest {
    /// 发布版号(语言本体组件须同号,RXS-0135)。
    pub rurix_version: String,
    /// 全部分发组件(序列化前按 `name` 排序,确定性)。
    pub components: Vec<Component>,
}

/// Attachment A 白名单审计判定(RXS-0136 / r6;延续 M5.4 check_redistribution)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedistributionAudit {
    /// 综合判定:NVIDIA 分区组件是否全部落 Attachment A 白名单最小集。
    pub pass: bool,
    /// 白名单外的 NVIDIA 组件干名(违例项;`pass=false` 时非空)。
    pub violations: Vec<String>,
}

impl BundleManifest {
    /// 构造空清单(指定版号)。
    pub fn new(rurix_version: impl Into<String>) -> Self {
        BundleManifest {
            rurix_version: rurix_version.into(),
            components: Vec::new(),
        }
    }

    /// 追加组件。
    pub fn push(&mut self, component: Component) {
        self.components.push(component);
    }

    /// 按分区筛选组件(只读引用,按 `name` 字典序稳定排序)。
    pub fn partition(&self, p: Partition) -> Vec<&Component> {
        let mut out: Vec<&Component> = self
            .components
            .iter()
            .filter(|c| c.partition == p)
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    /// 语言本体组件须**同一版号**原子分发(RXS-0135):任一语言本体组件版本
    /// 与 bundle 版号不符即返回 `false`(NVIDIA 再分发组件各有上游版本,豁免)。
    pub fn language_core_versions_uniform(&self) -> bool {
        self.partition(Partition::LanguageCore)
            .iter()
            .all(|c| c.version == self.rurix_version)
    }
}

/// 判定组件干名是否为 NVIDIA libdevice bitcode(`libdevice.<digits>.bc`)。
fn is_libdevice(name: &str) -> bool {
    match name
        .strip_prefix("libdevice.")
        .and_then(|s| s.strip_suffix(".bc"))
    {
        Some(mid) => !mid.is_empty() && mid.bytes().all(|b| b.is_ascii_digit()),
        None => false,
    }
}

/// 判定组件干名是否为 cuBLAS runtime DLL(`cublas64_<digits>.dll` /
/// `cublasLt64_<digits>.dll`;对齐 M5.4 check_redistribution 断言 3c 白名单正则)。
fn is_cublas_runtime(name: &str) -> bool {
    for stem in ["cublas64_", "cublasLt64_"] {
        if let Some(mid) = name.strip_prefix(stem).and_then(|s| s.strip_suffix(".dll"))
            && !mid.is_empty()
            && mid.bytes().all(|b| b.is_ascii_digit())
        {
            return true;
        }
    }
    false
}

/// Attachment A 白名单最小集判定(RXS-0136):仅 libdevice bitcode 与 cuBLAS
/// runtime DLL;完整 Toolkit(`nvcc`/`ptxas`)/ 驱动 / Nsight 等一概不在白名单。
pub fn is_attachment_a_whitelisted(name: &str) -> bool {
    is_libdevice(name) || is_cublas_runtime(name)
}

/// NVIDIA 再分发白名单审计(RXS-0136 / r6):NVIDIA 分区中任一非 Attachment A
/// 白名单组件即违例;语言本体分区不参与本审计。
pub fn audit_redistribution(bundle: &BundleManifest) -> RedistributionAudit {
    let mut violations: Vec<String> = bundle
        .partition(Partition::NvidiaRedist)
        .iter()
        .filter(|c| !is_attachment_a_whitelisted(&c.name))
        .map(|c| c.name.clone())
        .collect();
    violations.sort();
    RedistributionAudit {
        pass: violations.is_empty(),
        violations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comp(name: &str, ver: &str, p: Partition) -> Component {
        Component {
            name: name.to_string(),
            version: ver.to_string(),
            license: "Apache-2.0".to_string(),
            partition: p,
            sha256: "00".repeat(32),
        }
    }

    //@ spec: RXS-0136
    // 语言本体 ⟂ NVIDIA 再分发组件分区:NVIDIA 分区仅容 Attachment A 白名单最小集
    // (libdevice bitcode + cuBLAS runtime DLL),完整 Toolkit/驱动/Nsight 即违例(r6)。
    #[test]
    fn bundle_separates_core_from_nvidia_redist() {
        let mut b = BundleManifest::new("0.1.0");
        b.push(comp("rurixc.exe", "0.1.0", Partition::LanguageCore));
        b.push(comp("rurix_rt.dll", "0.1.0", Partition::LanguageCore));
        b.push(comp("libdevice.10.bc", "12.3", Partition::NvidiaRedist));
        b.push(comp("cublas64_12.dll", "12.3", Partition::NvidiaRedist));

        // 分区筛选确定(字典序)。
        let core = b.partition(Partition::LanguageCore);
        assert_eq!(core.len(), 2);
        assert_eq!(core[0].name, "rurix_rt.dll");
        let redist = b.partition(Partition::NvidiaRedist);
        assert_eq!(redist.len(), 2);

        // Attachment A 白名单识别。
        assert!(is_attachment_a_whitelisted("libdevice.10.bc"));
        assert!(is_attachment_a_whitelisted("cublas64_12.dll"));
        assert!(is_attachment_a_whitelisted("cublasLt64_12.dll"));
        // 完整 Toolkit / 驱动 / Nsight 永不入白名单(r6)。
        assert!(!is_attachment_a_whitelisted("nvcc.exe"));
        assert!(!is_attachment_a_whitelisted("ptxas.exe"));
        assert!(!is_attachment_a_whitelisted("nsight-compute.exe"));
        assert!(!is_attachment_a_whitelisted("nvcuda.dll"));
        assert!(!is_attachment_a_whitelisted("libdevice.bc")); // 缺版本号段
        assert!(!is_attachment_a_whitelisted("cublas64_.dll")); // 缺数字段

        // 全白名单 → 审计通过。
        let audit = audit_redistribution(&b);
        assert!(audit.pass);
        assert!(audit.violations.is_empty());

        // 语言本体同版号原子分发判据。
        assert!(b.language_core_versions_uniform());
    }

    //@ spec: RXS-0136
    // NVIDIA 分区混入白名单外组件(完整 Toolkit / Nsight)→ 审计违例,违例项可枚举。
    #[test]
    fn non_whitelisted_nvidia_component_fails_audit() {
        let mut b = BundleManifest::new("0.1.0");
        b.push(comp("libdevice.10.bc", "12.3", Partition::NvidiaRedist));
        b.push(comp(
            "nsight-compute.exe",
            "2024.1",
            Partition::NvidiaRedist,
        ));
        b.push(comp(
            "cudart64_full_toolkit.dll",
            "12.3",
            Partition::NvidiaRedist,
        ));

        let audit = audit_redistribution(&b);
        assert!(!audit.pass);
        assert_eq!(
            audit.violations,
            vec![
                "cudart64_full_toolkit.dll".to_string(),
                "nsight-compute.exe".to_string(),
            ]
        );
    }

    //@ spec: RXS-0135
    // 语言本体组件版号与 bundle 版号不符 → 非同一版号原子分发(RXS-0135 完整性判据之一)。
    #[test]
    fn language_core_version_skew_detected() {
        let mut b = BundleManifest::new("0.1.0");
        b.push(comp("rurixc.exe", "0.1.0", Partition::LanguageCore));
        b.push(comp("rx.exe", "0.0.9", Partition::LanguageCore)); // 版号偏移
        assert!(!b.language_core_versions_uniform());
    }
}
