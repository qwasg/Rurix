//! rurixup — Rurix 发布链路引导器与发布产物语义实现(M8.4,D-M8-4)。
//!
//! 条款:spec/release.md RXS-0135 ~ RXS-0139 + RXS-0185 ~ RXS-0186——
//! - [`install`] 原子分发与 content-tree 完整性(RXS-0135;复用 `rurix-pkg` 内容树 SHA-256)
//! - [`bundle`] 语言本体 ⟂ NVIDIA 再分发组件分离打包 + Attachment A 白名单审计(RXS-0136)
//! - [`signing`] 签名清单约定与验签发布前置(RXS-0137;of-record Azure Artifact Signing)
//! - [`sbom`] SBOM SPDX 构建视图 + CycloneDX 发布视图(RXS-0138)
//! - [`gate`] Release 层 hard-block 发布门(RXS-0139;RXS-0186 第 8 子门延伸)
//! - [`channel`] stable channel 最小清单(RXS-0185 ~ RXS-0186,V1.2/MR-0008)
//! - [`toolchain`] 本地工具链版本注册 + stable channel 消费(RXS-0187 ~ RXS-0188,MR-0009)
//!
//! 纪律:**全 safe**(`unsafe_code = "deny"`,继承 workspace lints);**零外部依赖**
//! (标准库 + `rurix-pkg` 手写 SHA-256 / 内容树),纯函数、确定性——同一发布输入产
//! 逐字节一致的 SBOM / 清单字节流。发布门失败以**工具层 Result / 退出码**表达,
//! **不分配编译器 RX 段位**(spec/release.md §3:本里程碑零追加)。

pub mod bundle;
pub mod channel;
pub mod gate;
pub mod install;
pub mod sbom;
pub mod signing;
pub mod toolchain;

use bundle::{BundleManifest, RedistributionAudit};
use channel::ChannelManifest;
use gate::{GateInputs, ReleaseDecision};
use sbom::SbomViews;
use signing::SigningManifest;

/// 最小 JSON 字符串转义(引号 / 反斜杠 / 控制符),供 SBOM 与清单序列化共用。
pub fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// 由 CI 层外部提供的子门机器事实(签名 / SBOM / 许可审计由 rurixup 内算,
/// 这四项由 Release workflow 回填)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CiFacts {
    /// `bench --strict` 通过。
    pub bench_strict_pass: bool,
    /// conformance 全绿。
    pub conformance_green: bool,
    /// UI golden 全绿。
    pub ui_golden_green: bool,
    /// L1 基准无 Critical 回归。
    pub l1_no_critical_regression: bool,
}

impl CiFacts {
    /// 四项 CI 子门全绿(冒烟前哨默认;Release workflow 实测回填)。
    pub fn all_green() -> Self {
        CiFacts {
            bench_strict_pass: true,
            conformance_green: true,
            ui_golden_green: true,
            l1_no_critical_regression: true,
        }
    }
}

/// 发布门故障注入(**仅供 Release 层 hard-block 真实红绿自检**:构造缺 SBOM 等
/// 子门红场景,断言发布门阻断;反 YAML-only,CI_GATES §6 第 5 项)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Faults {
    /// 强制 SBOM 不齐备(模拟「缺 SBOM」发布门红;SBOM 仍照常生成写出供核对)。
    pub force_missing_sbom: bool,
    /// 强制 channel 清单漂移(模拟第 8 子门 `channel-manifest` 红,RXS-0186;
    /// 清单仍照常生成写出供核对)。
    pub force_channel_drift: bool,
}

impl Faults {
    /// 无故障注入(正常发布路径)。
    pub fn none() -> Self {
        Faults::default()
    }
}

/// 发布编排报告:bundle / SBOM 双视图 / 签名清单 / 白名单审计 / channel 清单 /
/// 发布门决策 / 验签通过产物集(= `m8.counter.release_artifacts_signed` 的
/// `signed_artifacts`)。
#[derive(Debug, Clone)]
pub struct ReleaseReport {
    /// 发布 bundle 清单。
    pub bundle: BundleManifest,
    /// SBOM 双视图。
    pub sbom: SbomViews,
    /// 签名清单。
    pub signing: SigningManifest,
    /// NVIDIA 再分发白名单审计。
    pub audit: RedistributionAudit,
    /// stable channel 清单(RXS-0185,V1.2/MR-0008)。
    pub channel: ChannelManifest,
    /// Release 层发布门决策。
    pub decision: ReleaseDecision,
    /// SBOM 是否齐备(覆盖全部组件)。
    pub sbom_present: bool,
    /// channel 清单一致性判据(RXS-0186;受 [`Faults::force_channel_drift`] 注入影响)。
    pub channel_ok: bool,
    /// 验签通过产物干名去重集(机器事实)。
    pub signed_artifacts: Vec<String>,
}

/// 发布编排(RXS-0135~0139 + RXS-0185~0186 汇流):由 bundle + 签名清单 + 预生成
/// channel 清单 + CI 子门事实算出 SBOM、白名单审计、channel 一致性与发布门决策。
/// 纯函数、确定性。`channel` 由调用方经 [`channel::generate`] 预生成(未知 channel
/// 在 CLI 层即为用法错误退出码 1,不进入编排);`faults` 仅供发布门真实红绿自检
/// 注入子门红场景(正常路径传 [`Faults::none`])。
pub fn run_release(
    bundle: BundleManifest,
    signing: SigningManifest,
    channel: ChannelManifest,
    ci: CiFacts,
    faults: Faults,
) -> ReleaseReport {
    let sbom = sbom::generate(&bundle);
    // SBOM 照常生成(供写出核对);发布门 sbom_present 受故障注入影响。
    let sbom_present = sbom::components_covered(&bundle, &sbom) && !faults.force_missing_sbom;
    let audit = bundle::audit_redistribution(&bundle);
    let signed_artifacts = signing.verified_artifacts();
    // channel 清单照常写出(供核对);第 8 子门判定受故障注入影响(RXS-0186)。
    let channel_ok = channel::consistent(&bundle, &channel) && !faults.force_channel_drift;

    let inputs = GateInputs {
        signing_all_valid: signing.upload_permitted(),
        sbom_present,
        redistribution_audit_pass: audit.pass,
        bench_strict_pass: ci.bench_strict_pass,
        conformance_green: ci.conformance_green,
        ui_golden_green: ci.ui_golden_green,
        l1_no_critical_regression: ci.l1_no_critical_regression,
        channel_manifest_ok: channel_ok,
    };
    let decision = gate::release_decision(&inputs);

    ReleaseReport {
        bundle,
        sbom,
        signing,
        audit,
        channel,
        decision,
        sbom_present,
        channel_ok,
        signed_artifacts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bundle::{Component, Partition};
    use signing::{SignBackend, SignStatus, SignedArtifact};

    fn green_bundle() -> BundleManifest {
        let mut b = BundleManifest::new("0.1.0");
        b.push(Component {
            name: "rurixup.exe".to_string(),
            version: "0.1.0".to_string(),
            license: "Apache-2.0".to_string(),
            partition: Partition::LanguageCore,
            sha256: "aa".repeat(32),
        });
        b.push(Component {
            name: "libdevice.10.bc".to_string(),
            version: "12.3".to_string(),
            license: "NVIDIA-SLA-Attachment-A".to_string(),
            partition: Partition::NvidiaRedist,
            sha256: "bb".repeat(32),
        });
        b
    }

    fn green_signing() -> SigningManifest {
        let mut m = SigningManifest::new();
        m.push(SignedArtifact {
            name: "rurixup.exe".to_string(),
            digest: "aa".repeat(32),
            status: SignStatus::Valid,
            timestamped: true,
            backend: SignBackend::SelfSignedTest,
        });
        m
    }

    fn green_channel(bundle: &BundleManifest) -> ChannelManifest {
        channel::generate(bundle, "stable", &bundle.to_json()).expect("stable 合法")
    }

    //@ spec: RXS-0139
    //@ spec: RXS-0186
    // 端到端编排:全门绿 → 放行上传 + signed_artifacts ≥1 + channel_ok;注入
    // 未签名 / 缺 SBOM / channel 漂移 → 阻断(第 8 子门 channel-manifest 末位)。
    #[test]
    fn run_release_end_to_end_green_then_blocked() {
        let b = green_bundle();
        let report = run_release(
            b.clone(),
            green_signing(),
            green_channel(&b),
            CiFacts::all_green(),
            Faults::none(),
        );
        assert!(report.decision.allow_upload);
        assert!(report.sbom_present);
        assert!(report.audit.pass);
        assert!(report.channel_ok);
        assert_eq!(report.channel.channel, "stable");
        assert_eq!(report.signed_artifacts, vec!["rurixup.exe".to_string()]);

        // 未签名产物注入 → 发布门阻断。
        let mut unsigned = green_signing();
        unsigned.push(SignedArtifact {
            name: "rx.exe".to_string(),
            digest: "cc".repeat(32),
            status: SignStatus::Unsigned,
            timestamped: false,
            backend: SignBackend::SelfSignedTest,
        });
        let blocked = run_release(
            b.clone(),
            unsigned,
            green_channel(&b),
            CiFacts::all_green(),
            Faults::none(),
        );
        assert!(!blocked.decision.allow_upload);
        assert_eq!(blocked.decision.failed_gates, vec!["signing".to_string()]);

        // 缺 SBOM 故障注入 → 发布门阻断(SBOM 仍生成,仅门判定红)。
        let missing_sbom = run_release(
            b.clone(),
            green_signing(),
            green_channel(&b),
            CiFacts::all_green(),
            Faults {
                force_missing_sbom: true,
                ..Faults::none()
            },
        );
        assert!(!missing_sbom.decision.allow_upload);
        assert_eq!(missing_sbom.decision.failed_gates, vec!["sbom".to_string()]);
        // SBOM 字节流仍照常生成(供写出核对)。
        assert!(missing_sbom.sbom.spdx.contains("SPDX-2.3"));

        // channel 清单漂移注入 → 第 8 子门红 → 发布门阻断(RXS-0186;清单仍生成)。
        let drifted = run_release(
            b.clone(),
            green_signing(),
            green_channel(&b),
            CiFacts::all_green(),
            Faults {
                force_channel_drift: true,
                ..Faults::none()
            },
        );
        assert!(!drifted.decision.allow_upload);
        assert!(!drifted.channel_ok);
        assert_eq!(
            drifted.decision.failed_gates,
            vec!["channel-manifest".to_string()]
        );
        assert!(
            drifted
                .channel
                .to_json()
                .contains("\"channel\": \"stable\"")
        );
    }
}
