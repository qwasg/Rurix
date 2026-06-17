//! Release 层 hard-block 发布门(spec/release.md RXS-0139)。
//!
//! CI 第三层 **Release**(14 §8;PR Smoke / Nightly 之外)在打 tag / 发布工作流触发。
//! 门集 = 签名验签 + SBOM 齐备 + 许可白名单审计 + `bench --strict` + conformance/UI
//! golden 全绿 + L1 基准无 Critical 回归;**任一门失败 → 不上传 artifact**(发布阻断,
//! 10 §6 工具链发布门)。本决策为纯函数:输入各子门机器事实,输出是否放行上传 +
//! 失败子门清单(确定性枚举,反 YAML-only 留痕)。

/// Release 层各子门机器事实输入。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GateInputs {
    /// 全部产物验签通过(Authenticode + 时间戳;[`crate::signing::SigningManifest::upload_permitted`])。
    pub signing_all_valid: bool,
    /// SBOM 齐备(SPDX + CycloneDX 覆盖全部组件;[`crate::sbom::components_covered`])。
    pub sbom_present: bool,
    /// NVIDIA 再分发 Attachment A 白名单审计通过([`crate::bundle::audit_redistribution`])。
    pub redistribution_audit_pass: bool,
    /// `bench --strict` 通过(无容错跳过,零 estimated 残留)。
    pub bench_strict_pass: bool,
    /// conformance 全绿。
    pub conformance_green: bool,
    /// UI golden 全绿。
    pub ui_golden_green: bool,
    /// L1 基准无 Critical 回归。
    pub l1_no_critical_regression: bool,
}

impl GateInputs {
    /// 全门绿的构造便捷(测试 / 全通过基线)。
    pub fn all_green() -> Self {
        GateInputs {
            signing_all_valid: true,
            sbom_present: true,
            redistribution_audit_pass: true,
            bench_strict_pass: true,
            conformance_green: true,
            ui_golden_green: true,
            l1_no_critical_regression: true,
        }
    }
}

/// 发布门决策(放行上传与否 + 失败子门清单)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseDecision {
    /// 全门绿 → 允许上传发布 artifact;任一门红 → 阻断(`false`)。
    pub allow_upload: bool,
    /// 失败子门稳定标签(`allow_upload=false` 时非空,确定性顺序)。
    pub failed_gates: Vec<String>,
}

/// Release 层 hard-block 决策(RXS-0139):任一子门红 → 不放行上传,并枚举失败门。
pub fn release_decision(inputs: &GateInputs) -> ReleaseDecision {
    // 子门顺序固定(14 §8 口径:签名 / SBOM / 许可审计 / bench strict /
    // conformance / UI golden / L1 回归),确定性枚举。
    let checks: [(&str, bool); 7] = [
        ("signing", inputs.signing_all_valid),
        ("sbom", inputs.sbom_present),
        ("redistribution-audit", inputs.redistribution_audit_pass),
        ("bench-strict", inputs.bench_strict_pass),
        ("conformance", inputs.conformance_green),
        ("ui-golden", inputs.ui_golden_green),
        ("l1-regression", inputs.l1_no_critical_regression),
    ];
    let failed_gates: Vec<String> = checks
        .iter()
        .filter(|(_, ok)| !ok)
        .map(|(name, _)| (*name).to_string())
        .collect();
    ReleaseDecision {
        allow_upload: failed_gates.is_empty(),
        failed_gates,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0139
    // Release 层 hard-block:全门绿 → 放行上传;任一子门红 → 阻断上传 + 失败门枚举
    // (签名 / SBOM / 许可审计任一缺失即阻断,反 YAML-only)。
    #[test]
    fn release_gate_hard_blocks_on_any_failure() {
        // 绿:全门通过 → 放行。
        let green = release_decision(&GateInputs::all_green());
        assert!(green.allow_upload);
        assert!(green.failed_gates.is_empty());

        // 红 1:未签名 → 阻断。
        let mut unsigned = GateInputs::all_green();
        unsigned.signing_all_valid = false;
        let d = release_decision(&unsigned);
        assert!(!d.allow_upload);
        assert_eq!(d.failed_gates, vec!["signing".to_string()]);

        // 红 2:缺 SBOM → 阻断。
        let mut no_sbom = GateInputs::all_green();
        no_sbom.sbom_present = false;
        let d = release_decision(&no_sbom);
        assert!(!d.allow_upload);
        assert_eq!(d.failed_gates, vec!["sbom".to_string()]);

        // 红 3:白名单外组件 → 许可审计红 → 阻断。
        let mut bad_redist = GateInputs::all_green();
        bad_redist.redistribution_audit_pass = false;
        let d = release_decision(&bad_redist);
        assert!(!d.allow_upload);
        assert_eq!(d.failed_gates, vec!["redistribution-audit".to_string()]);

        // 多门同红 → 全部枚举(确定性顺序)。
        let all_red = GateInputs {
            signing_all_valid: false,
            sbom_present: false,
            redistribution_audit_pass: false,
            bench_strict_pass: false,
            conformance_green: false,
            ui_golden_green: false,
            l1_no_critical_regression: false,
        };
        let d = release_decision(&all_red);
        assert!(!d.allow_upload);
        assert_eq!(
            d.failed_gates,
            vec![
                "signing",
                "sbom",
                "redistribution-audit",
                "bench-strict",
                "conformance",
                "ui-golden",
                "l1-regression",
            ]
        );
    }
}
