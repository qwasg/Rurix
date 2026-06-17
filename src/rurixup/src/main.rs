//! rurixup 发布链路 CLI(M8.4,spec/release.md RXS-0135~0139)。
//!
//! `rurixup release` 由发布链路冒烟脚本(`ci/release_pipeline_smoke.py`,步骤 38)与
//! Release workflow 驱动:读组件路径算 content SHA-256 → 建 bundle 清单(语言本体 /
//! NVIDIA 再分发分区)→ 生成 SBOM SPDX + CycloneDX → 读外部验签状态建签名清单 →
//! NVIDIA 白名单审计 → Release 层 hard-block 发布门决策 → 写出清单 / SBOM / 门决策
//! JSON。退出码:`0` = 放行上传,`2` = 发布阻断(任一门红),`1` = 用法/IO 错误。
//!
//! 字段以 `|` 分隔(Windows `C:\` 路径含 `:`,不用 `:` 分隔)。

use std::path::Path;
use std::process::ExitCode;

use rurixup::bundle::{BundleManifest, Component, Partition};
use rurixup::signing::{SignBackend, SignStatus, SignedArtifact, SigningManifest};
use rurixup::{CiFacts, Faults, json_escape, run_release};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("release") => match run(&args[1..]) {
            Ok(code) => code,
            Err(msg) => {
                eprintln!("rurixup: 错误:{msg}");
                ExitCode::from(1)
            }
        },
        Some("--help") | Some("-h") | None => {
            print_usage();
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("rurixup: 未知子命令 `{other}`(支持:release)");
            print_usage();
            ExitCode::from(1)
        }
    }
}

fn print_usage() {
    eprintln!(
        "用法: rurixup release \\\n  \
         --version <ver> \\\n  \
         --component 'name|version|license|partition|path' (可重复;partition ∈ core|nvidia) \\\n  \
         --sign 'name|status|timestamped|backend' (可重复;status ∈ Valid|Unsigned|Invalid;timestamped ∈ true|false;backend ∈ azure|selftest) \\\n  \
         [--bench-strict <true|false>] [--conformance <true|false>] [--ui-golden <true|false>] [--l1-regression-ok <true|false>] \\\n  \
         [--simulate-missing-sbom] \\\n  \
         --out-dir <dir>"
    );
}

fn run(args: &[String]) -> Result<ExitCode, String> {
    let mut version: Option<String> = None;
    let mut component_specs: Vec<String> = Vec::new();
    let mut sign_specs: Vec<String> = Vec::new();
    let mut out_dir: Option<String> = None;
    let mut ci = CiFacts::all_green();
    let mut faults = Faults::none();

    let mut i = 0;
    while i < args.len() {
        let take = |i: &mut usize, flag: &str| -> Result<String, String> {
            *i += 1;
            args.get(*i)
                .cloned()
                .ok_or_else(|| format!("{flag} 缺参数"))
        };
        match args[i].as_str() {
            "--version" => version = Some(take(&mut i, "--version")?),
            "--component" => component_specs.push(take(&mut i, "--component")?),
            "--sign" => sign_specs.push(take(&mut i, "--sign")?),
            "--out-dir" => out_dir = Some(take(&mut i, "--out-dir")?),
            "--bench-strict" => {
                ci.bench_strict_pass = parse_bool(&take(&mut i, "--bench-strict")?)?
            }
            "--conformance" => ci.conformance_green = parse_bool(&take(&mut i, "--conformance")?)?,
            "--ui-golden" => ci.ui_golden_green = parse_bool(&take(&mut i, "--ui-golden")?)?,
            "--l1-regression-ok" => {
                ci.l1_no_critical_regression = parse_bool(&take(&mut i, "--l1-regression-ok")?)?
            }
            // 故障注入(发布门真实红绿自检:模拟缺 SBOM 子门红)。
            "--simulate-missing-sbom" => faults.force_missing_sbom = true,
            other => return Err(format!("未知参数 `{other}`")),
        }
        i += 1;
    }

    let version = version.ok_or("缺 --version")?;
    let out_dir = out_dir.ok_or("缺 --out-dir")?;
    if component_specs.is_empty() {
        return Err("至少需一个 --component".to_string());
    }

    // 建 bundle 清单(读组件文件算 content SHA-256)。
    let mut bundle = BundleManifest::new(&version);
    for spec in &component_specs {
        bundle.push(parse_component(spec)?);
    }

    // 建签名清单(外部 Get-AuthenticodeSignature 验签状态回填)。
    let mut signing = SigningManifest::new();
    for spec in &sign_specs {
        signing.push(parse_sign(spec, &bundle)?);
    }

    let report = run_release(bundle, signing, ci, faults);

    // 写出产物(SBOM 双视图 + bundle / 签名 / 门决策清单)。
    let out = Path::new(&out_dir);
    std::fs::create_dir_all(out).map_err(|e| format!("建 out-dir 失败:{e}"))?;
    write_file(out, "sbom.spdx.json", &report.sbom.spdx)?;
    write_file(out, "sbom.cdx.json", &report.sbom.cyclonedx)?;
    write_file(out, "bundle.json", &bundle_json(&report))?;
    write_file(out, "signing_manifest.json", &signing_json(&report))?;
    write_file(out, "gate_decision.json", &gate_json(&report))?;

    // 摘要行(冒烟脚本解析 + 人读)。
    println!(
        "RURIXUP_RELEASE: allow_upload={} signed_artifacts={} sbom_present={} audit_pass={} failed_gates=[{}]",
        report.decision.allow_upload,
        report.signed_artifacts.len(),
        report.sbom_present,
        report.audit.pass,
        report.decision.failed_gates.join(",")
    );

    if report.decision.allow_upload {
        Ok(ExitCode::SUCCESS)
    } else {
        // 发布阻断(hard block):任一门红 → 不上传 artifact。
        Ok(ExitCode::from(2))
    }
}

fn parse_bool(s: &str) -> Result<bool, String> {
    match s {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        other => Err(format!("非法布尔值 `{other}`(需 true|false)")),
    }
}

fn parse_partition(s: &str) -> Result<Partition, String> {
    match s {
        "core" | "language-core" => Ok(Partition::LanguageCore),
        "nvidia" | "nvidia-redist" => Ok(Partition::NvidiaRedist),
        other => Err(format!("未知分区 `{other}`(需 core|nvidia)")),
    }
}

fn parse_component(spec: &str) -> Result<Component, String> {
    let f: Vec<&str> = spec.split('|').collect();
    if f.len() != 5 {
        return Err(format!(
            "--component 需 5 段 'name|version|license|partition|path',得:{spec}"
        ));
    }
    let (name, version, license, partition, path) = (f[0], f[1], f[2], f[3], f[4]);
    let bytes = std::fs::read(path).map_err(|e| format!("读组件 `{path}` 失败:{e}"))?;
    let sha256 = rurix_pkg::sha256::hex_digest(&bytes);
    Ok(Component {
        name: name.to_string(),
        version: version.to_string(),
        license: license.to_string(),
        partition: parse_partition(partition)?,
        sha256,
    })
}

fn parse_sign(spec: &str, bundle: &BundleManifest) -> Result<SignedArtifact, String> {
    let f: Vec<&str> = spec.split('|').collect();
    if f.len() != 4 {
        return Err(format!(
            "--sign 需 4 段 'name|status|timestamped|backend',得:{spec}"
        ));
    }
    let (name, status, timestamped, backend) = (f[0], f[1], f[2], f[3]);
    let status = SignStatus::parse(status).ok_or_else(|| format!("未知验签状态 `{status}`"))?;
    let backend = match backend {
        "azure" | "azure-artifact-signing" => SignBackend::AzureArtifactSigning,
        "selftest" | "self-signed-test" => SignBackend::SelfSignedTest,
        other => return Err(format!("未知签名后端 `{other}`(需 azure|selftest)")),
    };
    // 签名项 digest 取 bundle 中同名组件摘要(签名对象 = 分发产物)。
    let digest = bundle
        .components
        .iter()
        .find(|c| c.name == name)
        .map(|c| c.sha256.clone())
        .unwrap_or_default();
    Ok(SignedArtifact {
        name: name.to_string(),
        digest,
        status,
        timestamped: parse_bool(timestamped)?,
        backend,
    })
}

fn write_file(dir: &Path, name: &str, content: &str) -> Result<(), String> {
    std::fs::write(dir.join(name), content).map_err(|e| format!("写 {name} 失败:{e}"))
}

fn bundle_json(report: &rurixup::ReleaseReport) -> String {
    let mut comps = report.bundle.components.clone();
    comps.sort_by(|a, b| a.name.cmp(&b.name));
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str(&format!(
        "  \"rurix_version\": \"{}\",\n",
        json_escape(&report.bundle.rurix_version)
    ));
    s.push_str("  \"components\": [\n");
    for (i, c) in comps.iter().enumerate() {
        let comma = if i + 1 < comps.len() { "," } else { "" };
        s.push_str("    {\n");
        s.push_str(&format!("      \"name\": \"{}\",\n", json_escape(&c.name)));
        s.push_str(&format!(
            "      \"version\": \"{}\",\n",
            json_escape(&c.version)
        ));
        s.push_str(&format!(
            "      \"license\": \"{}\",\n",
            json_escape(&c.license)
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
    s.push_str("  ]\n}\n");
    s
}

fn signing_json(report: &rurixup::ReleaseReport) -> String {
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str(&format!(
        "  \"upload_permitted\": {},\n",
        report.signing.upload_permitted()
    ));
    s.push_str("  \"signed_artifacts\": [");
    s.push_str(
        &report
            .signed_artifacts
            .iter()
            .map(|n| format!("\"{}\"", json_escape(n)))
            .collect::<Vec<_>>()
            .join(", "),
    );
    s.push_str("],\n");
    s.push_str("  \"artifacts\": [\n");
    for (i, a) in report.signing.artifacts.iter().enumerate() {
        let comma = if i + 1 < report.signing.artifacts.len() {
            ","
        } else {
            ""
        };
        s.push_str("    {\n");
        s.push_str(&format!("      \"name\": \"{}\",\n", json_escape(&a.name)));
        s.push_str(&format!("      \"status\": \"{}\",\n", a.status.label()));
        s.push_str(&format!("      \"timestamped\": {},\n", a.timestamped));
        s.push_str(&format!("      \"backend\": \"{}\",\n", a.backend.label()));
        s.push_str(&format!("      \"verified\": {}\n", a.verified()));
        s.push_str(&format!("    }}{comma}\n"));
    }
    s.push_str("  ]\n}\n");
    s
}

fn gate_json(report: &rurixup::ReleaseReport) -> String {
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str(&format!(
        "  \"allow_upload\": {},\n",
        report.decision.allow_upload
    ));
    s.push_str(&format!("  \"sbom_present\": {},\n", report.sbom_present));
    s.push_str(&format!(
        "  \"redistribution_audit_pass\": {},\n",
        report.audit.pass
    ));
    s.push_str("  \"audit_violations\": [");
    s.push_str(
        &report
            .audit
            .violations
            .iter()
            .map(|n| format!("\"{}\"", json_escape(n)))
            .collect::<Vec<_>>()
            .join(", "),
    );
    s.push_str("],\n");
    s.push_str("  \"failed_gates\": [");
    s.push_str(
        &report
            .decision
            .failed_gates
            .iter()
            .map(|n| format!("\"{}\"", json_escape(n)))
            .collect::<Vec<_>>()
            .join(", "),
    );
    s.push_str("]\n}\n");
    s
}
