//! 签名清单约定与验签发布前置(spec/release.md RXS-0137)。
//!
//! 全部 `.exe` / `.dll` / `.msi` 产物经 **Authenticode + 时间戳**签名;**签名后端
//! of-record = Azure Artifact Signing**([`SignBackend::AzureArtifactSigning`],生产
//! 签名经 CI secret + 人工门控,本机/CI 不自动调用真实证书,spec/release.md §4)。
//! 本地/CI 冒烟以临时自签测试证书([`SignBackend::SelfSignedTest`])产真实
//! Authenticode 红绿(机器事实层验签判定)。
//!
//! 发布产物携**签名清单**(每产物:干名 → content digest → 验签状态);**验签通过
//! ([`SignStatus::Valid`] 且时间戳齐备)为上传前置**——未签名 / 验签失败产物不得进入
//! 发布 artifact([`SigningManifest::upload_permitted`])。

use std::fmt;

/// 签名后端(of-record vs 本地冒烟)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignBackend {
    /// of-record 生产后端(Authenticode + 时间戳;CI secret + 人工门控,不自动调用)。
    AzureArtifactSigning,
    /// 本地/CI 冒烟临时自签测试证书(产真实 Authenticode 红绿)。
    SelfSignedTest,
}

impl SignBackend {
    /// 稳定字符串标签(清单序列化用)。
    pub fn label(self) -> &'static str {
        match self {
            SignBackend::AzureArtifactSigning => "azure-artifact-signing",
            SignBackend::SelfSignedTest => "self-signed-test",
        }
    }
}

/// 产物验签状态(机器事实:`Get-AuthenticodeSignature` 判定)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignStatus {
    /// 验签通过(Authenticode 签名有效)。
    Valid,
    /// 未签名。
    Unsigned,
    /// 签名无效(篡改 / 证书不可信 / 验签失败)。
    Invalid,
}

impl SignStatus {
    /// 从字符串解析(CLI / 冒烟脚本回填 `Get-AuthenticodeSignature` 结果)。
    pub fn parse(s: &str) -> Option<SignStatus> {
        match s {
            "Valid" => Some(SignStatus::Valid),
            "Unsigned" | "NotSigned" => Some(SignStatus::Unsigned),
            "Invalid" | "HashMismatch" | "UnknownError" => Some(SignStatus::Invalid),
            _ => None,
        }
    }

    /// 稳定字符串标签。
    pub fn label(self) -> &'static str {
        match self {
            SignStatus::Valid => "Valid",
            SignStatus::Unsigned => "Unsigned",
            SignStatus::Invalid => "Invalid",
        }
    }
}

impl fmt::Display for SignStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// 单个产物的签名清单项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedArtifact {
    /// 产物干名(如 `rurixup.exe`)。
    pub name: String,
    /// 产物 content SHA-256 十六进制。
    pub digest: String,
    /// 验签状态。
    pub status: SignStatus,
    /// 是否携 RFC 3161 时间戳(Authenticode + 时间戳为发布前置)。
    pub timestamped: bool,
    /// 签名后端。
    pub backend: SignBackend,
}

impl SignedArtifact {
    /// 验签通过判据:`Valid` **且**时间戳齐备(缺时间戳不计通过)。
    pub fn verified(&self) -> bool {
        self.status == SignStatus::Valid && self.timestamped
    }
}

/// 发布产物签名清单。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SigningManifest {
    /// 全部产物签名项。
    pub artifacts: Vec<SignedArtifact>,
}

impl SigningManifest {
    /// 空清单。
    pub fn new() -> Self {
        SigningManifest {
            artifacts: Vec::new(),
        }
    }

    /// 追加签名项。
    pub fn push(&mut self, a: SignedArtifact) {
        self.artifacts.push(a);
    }

    /// 验签通过的产物干名去重集(字典序;= 计入 `m8.counter.release_artifacts_signed`
    /// 的 `signed_artifacts`,机器事实:验签通过 + 时间戳齐备)。
    pub fn verified_artifacts(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .artifacts
            .iter()
            .filter(|a| a.verified())
            .map(|a| a.name.clone())
            .collect();
        names.sort();
        names.dedup();
        names
    }

    /// 未验签通过的产物(未签名 / 验签失败 / 缺时间戳;`upload_permitted=false` 时非空)。
    pub fn unverified(&self) -> Vec<&SignedArtifact> {
        self.artifacts.iter().filter(|a| !a.verified()).collect()
    }

    /// **验签发布前置**(RXS-0137):清单非空 **且**全部产物验签通过 → 允许上传;
    /// 任一未签名 / 验签失败 / 缺时间戳 → 不得进入发布 artifact。
    pub fn upload_permitted(&self) -> bool {
        !self.artifacts.is_empty() && self.artifacts.iter().all(SignedArtifact::verified)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn art(name: &str, status: SignStatus, timestamped: bool) -> SignedArtifact {
        SignedArtifact {
            name: name.to_string(),
            digest: "ab".repeat(32),
            status,
            timestamped,
            backend: SignBackend::SelfSignedTest,
        }
    }

    //@ spec: RXS-0137
    // 签名清单形态 + 验签发布前置:全部 Valid + 时间戳 → 允许上传 + verified 集 ≥1;
    // 任一未签名 / 验签失败 / 缺时间戳 → 阻断上传(未签名/验签失败产物不进 artifact)。
    #[test]
    fn signing_manifest_shape_and_verify_gate() {
        // 绿:两产物均 Valid + 时间戳齐备。
        let mut m = SigningManifest::new();
        m.push(art("rurixup.exe", SignStatus::Valid, true));
        m.push(art("rurix_rt.dll", SignStatus::Valid, true));
        assert!(m.upload_permitted());
        assert_eq!(
            m.verified_artifacts(),
            vec!["rurix_rt.dll".to_string(), "rurixup.exe".to_string()]
        );
        assert!(m.unverified().is_empty());

        // 红 1:一产物未签名 → 阻断上传,未验签项可枚举。
        let mut unsigned = m.clone();
        unsigned.push(art("rx.exe", SignStatus::Unsigned, false));
        assert!(!unsigned.upload_permitted());
        assert_eq!(unsigned.unverified().len(), 1);
        assert_eq!(unsigned.unverified()[0].name, "rx.exe");
        // 已签名的两件仍计入 verified 集。
        assert_eq!(unsigned.verified_artifacts().len(), 2);

        // 红 2:Valid 但缺时间戳 → 不计验签通过(Authenticode + 时间戳为前置)。
        let mut no_ts = SigningManifest::new();
        no_ts.push(art("rurixup.exe", SignStatus::Valid, false));
        assert!(!no_ts.upload_permitted());
        assert!(no_ts.verified_artifacts().is_empty());

        // 红 3:篡改 → Invalid → 阻断。
        let mut tampered = SigningManifest::new();
        tampered.push(art("rurixup.exe", SignStatus::Invalid, true));
        assert!(!tampered.upload_permitted());

        // 空清单不允许上传(无产物可发布)。
        assert!(!SigningManifest::new().upload_permitted());
    }

    //@ spec: RXS-0137
    // 验签状态解析覆盖 Get-AuthenticodeSignature 主要返回类目。
    #[test]
    fn sign_status_parse_roundtrip() {
        assert_eq!(SignStatus::parse("Valid"), Some(SignStatus::Valid));
        assert_eq!(SignStatus::parse("NotSigned"), Some(SignStatus::Unsigned));
        assert_eq!(SignStatus::parse("HashMismatch"), Some(SignStatus::Invalid));
        assert_eq!(SignStatus::parse("bogus"), None);
        assert_eq!(
            SignBackend::AzureArtifactSigning.label(),
            "azure-artifact-signing"
        );
    }
}
