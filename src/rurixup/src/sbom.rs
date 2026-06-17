//! SBOM 约定:SPDX 构建视图 + CycloneDX 发布视图(spec/release.md RXS-0138)。
//!
//! 构建期生成 **SPDX**(构建视图);发布附 **CycloneDX**(发布视图)。两视图组件
//! 清单覆盖 bundle 全部分发组件(语言本体 + NVIDIA 再分发组件,含版本与许可标识);
//! **SBOM 齐备为发布前置**(缺 SBOM 即阻断,10 §6 / 14 §8)。零外部依赖:手写
//! 确定性 JSON 序列化(同一 bundle 产逐字节一致字节流)。

use crate::bundle::BundleManifest;

/// SBOM 双视图(SPDX 构建视图 + CycloneDX 发布视图)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SbomViews {
    /// SPDX 2.3 JSON(构建视图)。
    pub spdx: String,
    /// CycloneDX 1.5 JSON(发布视图)。
    pub cyclonedx: String,
}

use crate::json_escape;

/// bundle 组件按干名字典序的稳定视图(两视图共用,确保确定性与覆盖一致)。
fn sorted_components(bundle: &BundleManifest) -> Vec<&crate::bundle::Component> {
    let mut all: Vec<&crate::bundle::Component> = bundle.components.iter().collect();
    all.sort_by(|a, b| a.name.cmp(&b.name));
    all
}

/// 生成 SPDX 2.3 构建视图 JSON(确定性)。
pub fn generate_spdx(bundle: &BundleManifest) -> String {
    let ver = json_escape(&bundle.rurix_version);
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str("  \"spdxVersion\": \"SPDX-2.3\",\n");
    s.push_str("  \"dataLicense\": \"CC0-1.0\",\n");
    s.push_str("  \"SPDXID\": \"SPDXRef-DOCUMENT\",\n");
    s.push_str(&format!("  \"name\": \"rurix-{ver}\",\n"));
    s.push_str(&format!(
        "  \"documentNamespace\": \"rurix://sbom/spdx/{ver}\",\n"
    ));
    s.push_str("  \"packages\": [\n");
    let comps = sorted_components(bundle);
    for (i, c) in comps.iter().enumerate() {
        let comma = if i + 1 < comps.len() { "," } else { "" };
        s.push_str("    {\n");
        s.push_str(&format!("      \"SPDXID\": \"SPDXRef-Package-{i}\",\n"));
        s.push_str(&format!("      \"name\": \"{}\",\n", json_escape(&c.name)));
        s.push_str(&format!(
            "      \"versionInfo\": \"{}\",\n",
            json_escape(&c.version)
        ));
        s.push_str(&format!(
            "      \"licenseConcluded\": \"{}\",\n",
            json_escape(&c.license)
        ));
        s.push_str(&format!(
            "      \"comment\": \"partition={}\",\n",
            c.partition.label()
        ));
        s.push_str("      \"checksums\": [\n");
        s.push_str("        {\n");
        s.push_str("          \"algorithm\": \"SHA256\",\n");
        s.push_str(&format!(
            "          \"checksumValue\": \"{}\"\n",
            json_escape(&c.sha256)
        ));
        s.push_str("        }\n");
        s.push_str("      ]\n");
        s.push_str(&format!("    }}{comma}\n"));
    }
    s.push_str("  ]\n");
    s.push_str("}\n");
    s
}

/// 生成 CycloneDX 1.5 发布视图 JSON(确定性)。
pub fn generate_cyclonedx(bundle: &BundleManifest) -> String {
    let ver = json_escape(&bundle.rurix_version);
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str("  \"bomFormat\": \"CycloneDX\",\n");
    s.push_str("  \"specVersion\": \"1.5\",\n");
    s.push_str("  \"version\": 1,\n");
    s.push_str("  \"metadata\": {\n");
    s.push_str("    \"component\": {\n");
    s.push_str("      \"type\": \"application\",\n");
    s.push_str("      \"name\": \"rurix\",\n");
    s.push_str(&format!("      \"version\": \"{ver}\"\n"));
    s.push_str("    }\n");
    s.push_str("  },\n");
    s.push_str("  \"components\": [\n");
    let comps = sorted_components(bundle);
    for (i, c) in comps.iter().enumerate() {
        let comma = if i + 1 < comps.len() { "," } else { "" };
        s.push_str("    {\n");
        s.push_str("      \"type\": \"library\",\n");
        s.push_str(&format!("      \"name\": \"{}\",\n", json_escape(&c.name)));
        s.push_str(&format!(
            "      \"version\": \"{}\",\n",
            json_escape(&c.version)
        ));
        s.push_str("      \"licenses\": [\n");
        s.push_str("        {\n");
        s.push_str("          \"license\": {\n");
        s.push_str(&format!(
            "            \"id\": \"{}\"\n",
            json_escape(&c.license)
        ));
        s.push_str("          }\n");
        s.push_str("        }\n");
        s.push_str("      ],\n");
        s.push_str("      \"hashes\": [\n");
        s.push_str("        {\n");
        s.push_str("          \"alg\": \"SHA-256\",\n");
        s.push_str(&format!(
            "          \"content\": \"{}\"\n",
            json_escape(&c.sha256)
        ));
        s.push_str("        }\n");
        s.push_str("      ],\n");
        s.push_str("      \"properties\": [\n");
        s.push_str("        {\n");
        s.push_str("          \"name\": \"rurix:partition\",\n");
        s.push_str(&format!(
            "          \"value\": \"{}\"\n",
            c.partition.label()
        ));
        s.push_str("        }\n");
        s.push_str("      ]\n");
        s.push_str(&format!("    }}{comma}\n"));
    }
    s.push_str("  ]\n");
    s.push_str("}\n");
    s
}

/// 生成 SBOM 双视图。
pub fn generate(bundle: &BundleManifest) -> SbomViews {
    SbomViews {
        spdx: generate_spdx(bundle),
        cyclonedx: generate_cyclonedx(bundle),
    }
}

/// 组件齐备判据(RXS-0138):bundle 每个组件的干名与版本均出现于 **两** 视图——
/// 任一视图缺任一组件即不齐备(发布阻断的 SBOM 子门)。空 bundle 视为不齐备。
pub fn components_covered(bundle: &BundleManifest, views: &SbomViews) -> bool {
    if bundle.components.is_empty() {
        return false;
    }
    bundle.components.iter().all(|c| {
        let name = json_escape(&c.name);
        let ver = json_escape(&c.version);
        views.spdx.contains(&name)
            && views.spdx.contains(&ver)
            && views.cyclonedx.contains(&name)
            && views.cyclonedx.contains(&ver)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::{Component, Partition};

    fn demo_bundle() -> BundleManifest {
        let mut b = BundleManifest::new("0.1.0");
        b.push(Component {
            name: "rurixc.exe".to_string(),
            version: "0.1.0".to_string(),
            license: "Apache-2.0".to_string(),
            partition: Partition::LanguageCore,
            sha256: "11".repeat(32),
        });
        b.push(Component {
            name: "libdevice.10.bc".to_string(),
            version: "12.3".to_string(),
            license: "NVIDIA-SLA-Attachment-A".to_string(),
            partition: Partition::NvidiaRedist,
            sha256: "22".repeat(32),
        });
        b
    }

    //@ spec: RXS-0138
    // 生成 SPDX 构建视图 + CycloneDX 发布视图,两视图覆盖全部分发组件(版本 + 许可),
    // 且生成确定性(同一 bundle 两次产逐字节一致)。
    #[test]
    fn sbom_spdx_cyclonedx_generation() {
        let bundle = demo_bundle();
        let views = generate(&bundle);

        // 两视图格式标识。
        assert!(views.spdx.contains("\"spdxVersion\": \"SPDX-2.3\""));
        assert!(views.cyclonedx.contains("\"bomFormat\": \"CycloneDX\""));

        // 覆盖全部组件(名 + 版本 + 许可 + 分区均落两视图)。
        assert!(components_covered(&bundle, &views));
        assert!(views.spdx.contains("libdevice.10.bc"));
        assert!(views.spdx.contains("NVIDIA-SLA-Attachment-A"));
        assert!(views.spdx.contains("partition=nvidia-redist"));
        assert!(views.cyclonedx.contains("rurixc.exe"));
        assert!(views.cyclonedx.contains("\"value\": \"language-core\""));

        // 确定性:重生逐字节一致。
        let again = generate(&bundle);
        assert_eq!(views, again);
    }

    //@ spec: RXS-0138
    // bundle 含未进 SBOM 的组件 → 组件不齐备(SBOM 子门红);空 bundle 视为不齐备。
    #[test]
    fn sbom_coverage_detects_missing_component() {
        let bundle = demo_bundle();
        let views = generate(&bundle);

        // 在已生成视图外再追加一个组件 → 视图未覆盖它 → 不齐备。
        let mut extended = bundle.clone();
        extended.push(Component {
            name: "rx_extra.dll".to_string(),
            version: "0.1.0".to_string(),
            license: "Apache-2.0".to_string(),
            partition: Partition::LanguageCore,
            sha256: "33".repeat(32),
        });
        assert!(!components_covered(&extended, &views));

        // 空 bundle 不齐备。
        let empty = BundleManifest::new("0.1.0");
        let empty_views = generate(&empty);
        assert!(!components_covered(&empty, &empty_views));
    }
}
