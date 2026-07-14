//! stable channel 最小清单(spec/release.md RXS-0185 ~ RXS-0186;V1.2,Mini-RFC/MR-0008)。
//!
//! 语言 1.0 首个 stable 发行的**发行渠道身份锚**:`rurixup release` 追加产出确定性
//! `channel_manifest.json`(channel=stable),记录 rurix 版号 + bundle 清单字节流
//! digest 引用(内容寻址,复用 `rurix_pkg::sha256`,RXS-0093 口径)+ 组件清单
//! (干名字典序)。**日期/时间戳不进清单**——同一 bundle 两次生成逐字节一致
//! (确定性纪律,镜像 RXS-0138 SBOM);channel 合法集首版 = `{"stable"}`。
//! 清单一致性(与 bundle 同版号 + 组件全集对应)为 Release 层 hard-block 第 8
//! 子门 `channel-manifest`(RXS-0186;既有 7 门相对顺序 0-byte)。
//!
//! **不实现** install/update/channel 切换(rustup 式前端为后续里程碑按档处置,
//! 08 §9);不建 nightly channel;零网络端点;零新 RX 码(工具层 Result / 退出码,
//! spec/release.md §3);纯 safe(`unsafe_code = "deny"`)。

use crate::bundle::{BundleManifest, Component};
use crate::json_escape;

/// channel 合法值集(首版仅 `stable`;未来 channel 扩集须随条款修订,不预造)。
pub const VALID_CHANNELS: &[&str] = &["stable"];

/// stable channel 清单(RXS-0185):发行渠道身份 + 版号 + bundle 清单 digest 引用 +
/// 组件清单(生成时从 bundle 拷贝,供 [`consistent`] 一致性判据独立核对)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelManifest {
    /// 发行渠道名(∈ [`VALID_CHANNELS`])。
    pub channel: String,
    /// 发行版号(须与 bundle `rurix_version` 一致,RXS-0186 / RXS-0135 同版号判据延续)。
    pub rurix_version: String,
    /// `bundle.json` 字节流 SHA-256 十六进制(内容寻址引用,RXS-0093 口径)。
    pub bundle_manifest_sha256: String,
    /// 组件清单(干名字典序;与 bundle 组件全集一一对应为一致性判据,RXS-0186)。
    pub components: Vec<Component>,
}

/// 生成 channel 清单(RXS-0185):校验 channel ∈ 合法集(未知 channel → `Err`,
/// 工具层用法错误,退出码 1,零新 RX 码)→ 拷贝 bundle 版号与组件(干名字典序)→
/// 对 `bundle_json` 字节流取 SHA-256 作内容寻址引用。纯函数、确定性。
pub fn generate(
    bundle: &BundleManifest,
    channel: &str,
    bundle_json: &str,
) -> Result<ChannelManifest, String> {
    if !VALID_CHANNELS.contains(&channel) {
        return Err(format!(
            "未知 channel `{channel}`(支持:{})",
            VALID_CHANNELS.join("|")
        ));
    }
    let mut components = bundle.components.clone();
    components.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(ChannelManifest {
        channel: channel.to_string(),
        rurix_version: bundle.rurix_version.clone(),
        bundle_manifest_sha256: rurix_pkg::sha256::hex_digest(bundle_json.as_bytes()),
        components,
    })
}

/// 一致性判据(RXS-0186):channel ∈ 合法集 **且** 清单版号 == bundle 版号 **且**
/// 组件全集一一对应(干名 / 版号 / 分区 / digest 逐项一致,字典序比较)。任一不符
/// → `false` → Release 层第 8 子门 `channel-manifest` 红(发布阻断,RXS-0139 延伸)。
pub fn consistent(bundle: &BundleManifest, manifest: &ChannelManifest) -> bool {
    if !VALID_CHANNELS.contains(&manifest.channel.as_str()) {
        return false;
    }
    if manifest.rurix_version != bundle.rurix_version {
        return false;
    }
    let mut expected = bundle.components.clone();
    expected.sort_by(|a, b| a.name.cmp(&b.name));
    manifest.components == expected
}

impl ChannelManifest {
    /// 确定性 JSON 序列化(RXS-0185):手写零依赖(`crate::json_escape` + 字典序),
    /// **不含时间戳**——同一输入两次产逐字节一致字节流(发布日期归 Release 元数据
    /// 与 evidence `timestamp` 承载)。
    pub fn to_json(&self) -> String {
        let mut s = String::new();
        s.push_str("{\n");
        s.push_str("  \"schema_version\": 1,\n");
        s.push_str(&format!(
            "  \"channel\": \"{}\",\n",
            json_escape(&self.channel)
        ));
        s.push_str(&format!(
            "  \"rurix_version\": \"{}\",\n",
            json_escape(&self.rurix_version)
        ));
        s.push_str(&format!(
            "  \"bundle_manifest_sha256\": \"{}\",\n",
            json_escape(&self.bundle_manifest_sha256)
        ));
        s.push_str("  \"components\": [\n");
        for (i, c) in self.components.iter().enumerate() {
            let comma = if i + 1 < self.components.len() {
                ","
            } else {
                ""
            };
            s.push_str("    {\n");
            s.push_str(&format!("      \"name\": \"{}\",\n", json_escape(&c.name)));
            s.push_str(&format!(
                "      \"version\": \"{}\",\n",
                json_escape(&c.version)
            ));
            s.push_str(&format!(
                "      \"partition\": \"{}\",\n",
                c.partition.label()
            ));
            s.push_str(&format!(
                "      \"sha256\": \"{}\"\n",
                json_escape(&c.sha256)
            ));
            s.push_str(&format!("    }}{comma}\n"));
        }
        s.push_str("  ],\n");
        s.push_str(
            "  \"sbom\": { \"spdx\": \"sbom.spdx.json\", \"cyclonedx\": \"sbom.cdx.json\" },\n",
        );
        s.push_str("  \"signing_manifest\": \"signing_manifest.json\"\n");
        s.push_str("}\n");
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::Partition;

    fn bundle() -> BundleManifest {
        let mut b = BundleManifest::new("1.0.0");
        b.push(Component {
            name: "rx.exe".to_string(),
            version: "1.0.0".to_string(),
            license: "Apache-2.0".to_string(),
            partition: Partition::LanguageCore,
            sha256: "cc".repeat(32),
        });
        b.push(Component {
            name: "rurixup.exe".to_string(),
            version: "1.0.0".to_string(),
            license: "Apache-2.0".to_string(),
            partition: Partition::LanguageCore,
            sha256: "aa".repeat(32),
        });
        b
    }

    //@ spec: RXS-0185
    // channel 清单形态 + 确定性:channel=stable / 版号拷贝 bundle / digest = bundle.json
    // 字节流 SHA-256 / 组件干名字典序;同一输入两次 to_json 逐字节一致(无时间戳)。
    #[test]
    fn channel_manifest_stable_shape_and_determinism() {
        let b = bundle();
        let bundle_json = b.to_json();
        let m = generate(&b, "stable", &bundle_json).expect("stable 合法");
        assert_eq!(m.channel, "stable");
        assert_eq!(m.rurix_version, "1.0.0");
        assert_eq!(
            m.bundle_manifest_sha256,
            rurix_pkg::sha256::hex_digest(bundle_json.as_bytes())
        );
        // 组件干名字典序(rurixup.exe < rx.exe)。
        let names: Vec<&str> = m.components.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["rurixup.exe", "rx.exe"]);
        // 确定性:两次生成 + 两次序列化逐字节一致。
        let m2 = generate(&b, "stable", &bundle_json).expect("stable 合法");
        assert_eq!(m, m2);
        assert_eq!(m.to_json(), m2.to_json());
        // 序列化含身份锚字段,不含时间戳。
        let json = m.to_json();
        assert!(json.contains("\"channel\": \"stable\""));
        assert!(json.contains("\"rurix_version\": \"1.0.0\""));
        assert!(!json.to_lowercase().contains("timestamp"));
    }

    //@ spec: RXS-0185
    // 未知 channel → Err(工具层用法错误;首版合法集仅 stable,不建 nightly)。
    #[test]
    fn unknown_channel_rejected() {
        let b = bundle();
        let bundle_json = b.to_json();
        assert!(generate(&b, "nightly", &bundle_json).is_err());
        assert!(generate(&b, "beta", &bundle_json).is_err());
        assert!(generate(&b, "", &bundle_json).is_err());
    }

    //@ spec: RXS-0186
    // 一致性判据:生成即一致;版号漂移 / 组件 digest 漂移 / 组件缺失 / 非法 channel
    // 任一 → consistent=false(→ 第 8 子门 channel-manifest 红,发布阻断)。
    #[test]
    fn channel_version_consistency_detected() {
        let b = bundle();
        let bundle_json = b.to_json();
        let m = generate(&b, "stable", &bundle_json).expect("stable 合法");
        assert!(consistent(&b, &m));

        // 版号漂移(channel 清单版号 ≠ bundle 版号,RXS-0135 同版号判据延续)。
        let mut skew = m.clone();
        skew.rurix_version = "0.1.0".to_string();
        assert!(!consistent(&b, &skew));

        // 组件 digest 漂移。
        let mut tampered = m.clone();
        tampered.components[0].sha256 = "ff".repeat(32);
        assert!(!consistent(&b, &tampered));

        // 组件缺失(全集不对应)。
        let mut missing = m.clone();
        missing.components.pop();
        assert!(!consistent(&b, &missing));

        // 非法 channel(防御:直接构造绕过 generate 校验者)。
        let mut bad = m.clone();
        bad.channel = "nightly".to_string();
        assert!(!consistent(&b, &bad));
    }
}
