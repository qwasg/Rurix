//! rurixup 发布链路 CLI(M8.4,spec/release.md RXS-0135~0139 + V1.2 RXS-0185~0186)。
//!
//! `rurixup release` 由发布链路冒烟脚本(`ci/release_pipeline_smoke.py`,步骤 38 /
//! `ci/channel_manifest_smoke.py`,步骤 50)与 Release workflow 驱动:读组件路径算
//! content SHA-256 → 建 bundle 清单(语言本体 / NVIDIA 再分发分区)→ 生成 stable
//! channel 清单(channel=stable 缺省,RXS-0185)→ 生成 SBOM SPDX + CycloneDX →
//! 读外部验签状态建签名清单 → NVIDIA 白名单审计 → Release 层 hard-block 发布门
//! 决策(含第 8 子门 channel-manifest,RXS-0186)→ 写出清单 / SBOM / 门决策 JSON。
//! 退出码:`0` = 放行上传,`2` = 发布阻断(任一门红),`1` = 用法/IO 错误(含未知
//! channel)。
//!
//! 字段以 `|` 分隔(Windows `C:\` 路径含 `:`,不用 `:` 分隔)。

use std::path::Path;
use std::process::ExitCode;

use rurixup::bundle::{BundleManifest, Component, Partition};
use rurixup::install::{self, MaterializeReceipt};
use rurixup::signing::{SignBackend, SignStatus, SignedArtifact, SigningManifest};
use rurixup::toolchain::ToolchainRegistry;
use rurixup::{CiFacts, Faults, channel, json_escape, run_release, shim};

fn main() -> ExitCode {
    // 活跃版本切换 shim(RXS-0215):current_exe 干名 ≠ "rurixup" → 代理转发 default
    // 版本同名 exe 并透传退出码(此调用在代理成功/失败时 std::process::exit,不返回);
    // 干名 == "rurixup" → 返回,走正常子命令分发。
    let args: Vec<String> = std::env::args().skip(1).collect();
    shim::forward_if_shim(&args);
    let dispatch = |r: Result<ExitCode, String>| -> ExitCode {
        match r {
            Ok(code) => code,
            Err(msg) => {
                eprintln!("rurixup: 错误:{msg}");
                ExitCode::from(1)
            }
        }
    };
    match args.first().map(String::as_str) {
        Some("release") => dispatch(run(&args[1..])),
        // MR-0009 工具链前端(RXS-0187/0188):消费 stable channel + 注册 + 默认切换。
        // EA1.1a(RXS-0214/0215):`install --from-dir` 真实 FS 物化 + `setup` PATH 接入。
        Some("install") => dispatch(cmd_install(&args[1..])),
        Some("list") => dispatch(cmd_list(&args[1..])),
        Some("default") => dispatch(cmd_default(&args[1..])),
        Some("setup") => dispatch(cmd_setup(&args[1..])),
        Some("--help") | Some("-h") | None => {
            print_usage();
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("rurixup: 未知子命令 `{other}`(支持:release|install|list|default)");
            print_usage();
            ExitCode::from(1)
        }
    }
}

fn print_usage() {
    eprintln!(
        "用法:\n  \
         rurixup release --version <ver> --component '...' --sign '...' [选项] --out-dir <dir>\n  \
         rurixup install --channel-manifest <path> --bundle <path> [--registry <path>]\n  \
         rurixup install --from-dir <dir> [--registry <path>] [--home <dir>]   (真实 FS 物化,RXS-0214)\n  \
         rurixup list [--registry <path>] [--verify]\n  \
         rurixup default <version> [--registry <path>] [--home <dir>]\n  \
         rurixup setup [--add-path] [--home <dir>]   (缺省只打印 PATH 接入指令,RXS-0215)\n\
         \n\
         release 详细选项:\n  \
         --component 'name|version|license|partition|path' (可重复;partition ∈ core|nvidia)\n  \
         --sign 'name|status|timestamped|backend' (可重复;status ∈ Valid|Unsigned|Invalid;backend ∈ azure|selftest)\n  \
         [--bench-strict|--conformance|--ui-golden|--l1-regression-ok <true|false>]\n  \
         [--channel <name>] (缺省 stable) [--simulate-missing-sbom] [--simulate-channel-drift]\n\
         \n\
         install/list/default(MR-0009):--registry 缺省 ./toolchains.json"
    );
}

/// 单值 flag 取参数(缺省值可选)。
fn opt_arg(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1).cloned())
}

/// `rurixup install`:两条路径——
/// - `--from-dir <dir>`(RXS-0214,EA1.1a):本地目录源,组件字节真实物化到磁盘版本
///   目录(staging→逐组件 sha256→tree_digest 双向复算→同卷单次 rename),失败零半装;
/// - `--channel-manifest + --bundle`(RXS-0188,既有):纯账面内容寻址校验 + 注册。
///
/// 机器 token 行(纯追加,既有字段 0-byte):`RURIXUP_INSTALL: version=.. channel=..
/// default=.. registered=..`(既有)+ `components=.. digest_levels_verified=..
/// installed=..`(--from-dir 新增)+ 失败 `RURIXUP_INSTALL_ERROR: kind=<integrity|io|usage>`。
fn cmd_install(args: &[String]) -> Result<ExitCode, String> {
    if let Some(from_dir) = opt_arg(args, "--from-dir") {
        return cmd_install_from_dir(args, &from_dir);
    }

    let manifest_path = opt_arg(args, "--channel-manifest").ok_or("缺 --channel-manifest")?;
    let bundle_path = opt_arg(args, "--bundle").ok_or("缺 --bundle")?;
    let registry_path = opt_arg(args, "--registry").unwrap_or_else(|| "toolchains.json".into());

    let bundle_json =
        std::fs::read_to_string(&bundle_path).map_err(|e| format!("读 {bundle_path} 失败:{e}"))?;
    let bundle = BundleManifest::from_json(&bundle_json)?;
    let manifest_text = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("读 {manifest_path} 失败:{e}"))?;
    let declared_channel =
        json_string_field(&manifest_text, "channel").ok_or("channel_manifest.json 缺 channel")?;
    let declared_digest = json_string_field(&manifest_text, "bundle_manifest_sha256")
        .ok_or("channel_manifest.json 缺 bundle_manifest_sha256")?;

    // 内容寻址校验:清单声明的 digest 必须指向这份 bundle(篡改/错配即拒,RXS-0135/0187)。
    let actual = rurix_pkg::sha256::hex_digest(bundle_json.as_bytes());
    if declared_digest != actual {
        return Err(format!(
            "channel 清单 bundle_manifest_sha256 与实测 bundle 不符(声明 {declared_digest} != 实测 {actual});清单未指向此 bundle"
        ));
    }
    // 由 bundle 重生规范 channel 清单(校验 channel ∈ 合法集 + 一致性,RXS-0186)。
    let manifest = channel::generate(&bundle, &declared_channel, &bundle_json)?;

    let mut registry = if std::path::Path::new(&registry_path).exists() {
        ToolchainRegistry::from_json(
            &std::fs::read_to_string(&registry_path)
                .map_err(|e| format!("读 {registry_path} 失败:{e}"))?,
        )?
    } else {
        ToolchainRegistry::new()
    };
    let version = registry
        .install(&manifest, &bundle, &bundle_json)
        .map_err(|e| format!("install 校验失败:{e:?}"))?;
    std::fs::write(&registry_path, registry.to_json())
        .map_err(|e| format!("写 {registry_path} 失败:{e}"))?;

    println!(
        "RURIXUP_INSTALL: version={} channel={} default={} registered={}",
        version,
        declared_channel,
        registry.default_version().unwrap_or("-"),
        registry.list().len()
    );
    Ok(ExitCode::SUCCESS)
}

/// `rurixup install --from-dir <dir>`(RXS-0214):本地目录源真实 FS 物化。
///
/// `<dir>` 含 `channel_manifest.json` + `bundle.json` + 逐组件字节文件(按组件干名)。
/// 流程:内容寻址交叉核对(声明 digest == 实测 bundle digest,RXS-0188)→ channel
/// 一致性(RXS-0186)→ 读组件字节 → `install::materialize_to_disk`(staging→逐组件
/// sha256→tree_digest 双向复算→同卷单次 rename)→ 注册表 v2 单写(install_path +
/// tree_digest)。**任一校验失败** → 不写注册表(materialize 已清 staging、零残留)。
fn cmd_install_from_dir(args: &[String], from_dir: &str) -> Result<ExitCode, String> {
    let dir = Path::new(from_dir);
    let registry_path = opt_arg(args, "--registry").unwrap_or_else(|| "toolchains.json".into());
    // --home 覆盖 RURIX_HOME(测试缝);缺省用 install::rurix_home()。
    let home = match opt_arg(args, "--home") {
        Some(h) => std::path::PathBuf::from(h),
        None => install::rurix_home().map_err(|e| fs_err_report(&format!("{e:?}")))?,
    };

    let bundle_json = std::fs::read_to_string(dir.join("bundle.json"))
        .map_err(|e| fs_err_report(&format!("读 {}/bundle.json 失败:{e}", from_dir)))?;
    let bundle = BundleManifest::from_json(&bundle_json).map_err(|e| integrity_report(&e))?;
    let manifest_text = std::fs::read_to_string(dir.join("channel_manifest.json"))
        .map_err(|e| fs_err_report(&format!("读 {}/channel_manifest.json 失败:{e}", from_dir)))?;
    let declared_channel = json_string_field(&manifest_text, "channel")
        .ok_or_else(|| integrity_report("channel_manifest.json 缺 channel"))?;
    let declared_digest = json_string_field(&manifest_text, "bundle_manifest_sha256")
        .ok_or_else(|| integrity_report("channel_manifest.json 缺 bundle_manifest_sha256"))?;

    // 级② 内容寻址:声明 digest == 实测 sha256(bundle_json)(RXS-0188/0135)。
    let actual = rurix_pkg::sha256::hex_digest(bundle_json.as_bytes());
    if declared_digest != actual {
        return Err(integrity_report(&format!(
            "channel 清单 bundle_manifest_sha256 与实测 bundle 不符(声明 {declared_digest} != 实测 {actual})"
        )));
    }
    // 级③ 一致性:由 bundle 重生规范 channel 清单(channel ∈ 合法集 + 组件全集,RXS-0186)。
    let _manifest = channel::generate(&bundle, &declared_channel, &bundle_json)
        .map_err(|e| integrity_report(&e))?;

    // 读逐组件字节(按组件干名,扁平布局 <dir>/<name>)。
    let mut staged: Vec<(String, Vec<u8>)> = Vec::with_capacity(bundle.components.len());
    for c in &bundle.components {
        let p = dir.join(&c.name);
        let bytes = std::fs::read(&p)
            .map_err(|e| fs_err_report(&format!("读组件 {} 失败:{e}", p.display())))?;
        staged.push((c.name.clone(), bytes));
    }

    // 级④ 真实 FS 物化(逐组件 sha256 + tree_digest 双向复算 + 同卷单次 rename)。
    let receipt: MaterializeReceipt = install::materialize_to_disk(&home, &bundle, &staged)
        .map_err(|e| integrity_report(&format!("物化失败:{e:?}")))?;

    // 物化成功 → 注册表 v2 单写(install_path + tree_digest);失败前绝不触注册表。
    let mut registry = if std::path::Path::new(&registry_path).exists() {
        ToolchainRegistry::from_json(
            &std::fs::read_to_string(&registry_path)
                .map_err(|e| fs_err_report(&format!("读 {registry_path} 失败:{e}")))?,
        )
        .map_err(|e| integrity_report(&e))?
    } else {
        ToolchainRegistry::new()
    };
    registry.register_materialized(
        &receipt.version,
        &actual,
        &receipt.install_path.to_string_lossy(),
        &receipt.tree_digest,
    );
    std::fs::write(&registry_path, registry.to_json())
        .map_err(|e| fs_err_report(&format!("写 {registry_path} 失败:{e}")))?;

    println!(
        "RURIXUP_INSTALL: version={} channel={} default={} registered={} components={} digest_levels_verified=4 installed={}",
        receipt.version,
        declared_channel,
        registry.default_version().unwrap_or("-"),
        registry.list().len(),
        receipt.component_count,
        receipt.install_path.display(),
    );
    Ok(ExitCode::SUCCESS)
}

/// 完整性/内容寻址失败诊断(机器 token `RURIXUP_INSTALL_ERROR: kind=integrity`)。
fn integrity_report(detail: &str) -> String {
    println!("RURIXUP_INSTALL_ERROR: kind=integrity");
    detail.to_string()
}

/// IO 失败诊断(机器 token `RURIXUP_INSTALL_ERROR: kind=io`)。
fn fs_err_report(detail: &str) -> String {
    println!("RURIXUP_INSTALL_ERROR: kind=io");
    detail.to_string()
}

/// `rurixup list`(RXS-0187;RXS-0214 `--verify`):列出已注册版本 + 标注 default +
/// registered-only/物化区分;`--verify` 对已物化条目经 `install::tree_digest_from_dir`
/// 重哈希标注 corrupted(内容树漂移;失败模式 #11)——**corrupted 计数 >0 → 退出 1**。
fn cmd_list(args: &[String]) -> Result<ExitCode, String> {
    let registry_path = opt_arg(args, "--registry").unwrap_or_else(|| "toolchains.json".into());
    let verify = args.iter().any(|a| a == "--verify");
    let registry = load_registry(&registry_path)?;
    let default = registry.default_version();
    let mut corrupted = 0usize;
    println!(
        "RURIXUP_LIST: count={} default={}",
        registry.list().len(),
        default.unwrap_or("-")
    );
    for t in registry.list() {
        let mark = if Some(t.version.as_str()) == default {
            " (default)"
        } else {
            ""
        };
        let kind = match &t.install_path {
            Some(p) => {
                let mut status = format!("path={p}");
                if verify {
                    match (&t.tree_digest, install::tree_digest_from_dir(Path::new(p))) {
                        (Some(expected), Ok(on_disk)) if &on_disk == expected => {
                            status.push_str(" verify=ok")
                        }
                        (Some(_), Ok(_)) => {
                            corrupted += 1;
                            status.push_str(" verify=CORRUPTED(tree_digest 漂移)")
                        }
                        (_, Err(e)) => {
                            corrupted += 1;
                            status.push_str(&format!(" verify=CORRUPTED({e:?})"))
                        }
                        (None, _) => status.push_str(" verify=skip(无 tree_digest)"),
                    }
                }
                status
            }
            None => "registered-only".to_string(),
        };
        println!("  {}{}  {}  [{}]", t.version, mark, t.content_digest, kind);
    }
    if verify && corrupted > 0 {
        return Err(format!(
            "{corrupted} 个已物化版本 tree_digest 校验失败(corrupted)"
        ));
    }
    Ok(ExitCode::SUCCESS)
}

/// `rurixup default <version>`(RXS-0187/0215):设默认版本(须已注册);对已物化条目
/// (install_path 非空)额外校验磁盘目录**存在**——切换指向缺失目录 → 诚实错误退出 1
/// (RXS-0215),且**不写注册表**(全有或全无)。
fn cmd_default(args: &[String]) -> Result<ExitCode, String> {
    let version = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .cloned()
        .ok_or("缺 <version>")?;
    let registry_path = opt_arg(args, "--registry").unwrap_or_else(|| "toolchains.json".into());
    let mut registry = load_registry(&registry_path)?;
    // 切换目标磁盘存在性校验(已物化条目):指向缺失目录 → 诚实错误,不写注册表。
    if let Some(t) = registry.get(&version)
        && let Some(path) = &t.install_path
        && !Path::new(path).is_dir()
    {
        return Err(format!(
            "切换目标版本目录不存在:{path}(版本 {version} 可能已删除;重装或选其它版本)"
        ));
    }
    registry
        .set_default(&version)
        .map_err(|e| format!("set default 失败:{e:?}"))?;
    std::fs::write(&registry_path, registry.to_json())
        .map_err(|e| format!("写 {registry_path} 失败:{e}"))?;
    println!("RURIXUP_DEFAULT: default={version}");
    Ok(ExitCode::SUCCESS)
}

/// `rurixup setup`(RXS-0215):PATH 接入。**缺省只打印指令**(免副作用);`--add-path`
/// 显式 opt-in 才改用户 PATH——经 PowerShell `[Environment]::SetEnvironmentVariable(
/// 'Path', <new>, 'User')`(std::process 外呼,零 unsafe / 零第三方,免 setx 1024 截断)。
fn cmd_setup(args: &[String]) -> Result<ExitCode, String> {
    let home = match opt_arg(args, "--home") {
        Some(h) => std::path::PathBuf::from(h),
        None => install::rurix_home().map_err(|e| format!("{e:?}"))?,
    };
    let bin = home.join("bin");
    let bin_str = bin.display().to_string();
    let add_path = args.iter().any(|a| a == "--add-path");

    if !add_path {
        // 缺省:只打印手动接入指令(不触碰用户环境)。
        println!("RURIXUP_SETUP: mode=print bin={bin_str}");
        println!("把以下目录加入用户 PATH(shim 目录,一次入 PATH 后所有版本经切换即时生效):");
        println!("  {bin_str}");
        println!("PowerShell(手动执行以持久化,或用 `rurixup setup --add-path`):");
        println!(
            "  [Environment]::SetEnvironmentVariable('Path', ([Environment]::GetEnvironmentVariable('Path','User').TrimEnd(';') + ';{bin_str}'), 'User')"
        );
        return Ok(ExitCode::SUCCESS);
    }

    // --add-path:经 PowerShell 幂等追加(已含则 no-op),写用户级 Path。
    let ps = format!(
        "$b='{bin_str}'; $p=[Environment]::GetEnvironmentVariable('Path','User'); if([string]::IsNullOrEmpty($p)){{ $p='' }}; if(($p -split ';') -notcontains $b){{ $np=($p.TrimEnd(';')); if($np -ne ''){{ $np=$np + ';' }}; $np=$np + $b; [Environment]::SetEnvironmentVariable('Path', $np, 'User'); Write-Output 'added' }} else {{ Write-Output 'present' }}"
    );
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps])
        .output()
        .map_err(|e| format!("外呼 PowerShell 写用户 PATH 失败:{e}"))?;
    if !out.status.success() {
        return Err(format!(
            "PowerShell 写用户 PATH 非零退出:{}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let result = String::from_utf8_lossy(&out.stdout);
    println!(
        "RURIXUP_SETUP: mode=add-path bin={bin_str} result={}",
        result.trim()
    );
    println!("已把 {bin_str} 写入用户 PATH(新开 shell 生效)。");
    Ok(ExitCode::SUCCESS)
}

fn load_registry(path: &str) -> Result<ToolchainRegistry, String> {
    if std::path::Path::new(path).exists() {
        ToolchainRegistry::from_json(
            &std::fs::read_to_string(path).map_err(|e| format!("读 {path} 失败:{e}"))?,
        )
    } else {
        Err(format!("工具链注册表 {path} 不存在(先 rurixup install)"))
    }
}

/// 从本 crate 规范 JSON 抽取一个字符串字段值(`  "key": "value"` 行扫描;
/// 仅用于 channel_manifest.json 的 channel / bundle_manifest_sha256 标量字段)。
fn json_string_field(text: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\":");
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(&needle) {
            return Some(
                rest.trim()
                    .trim_end_matches(',')
                    .trim()
                    .trim_matches('"')
                    .to_string(),
            );
        }
    }
    None
}

fn run(args: &[String]) -> Result<ExitCode, String> {
    let mut version: Option<String> = None;
    let mut component_specs: Vec<String> = Vec::new();
    let mut sign_specs: Vec<String> = Vec::new();
    let mut out_dir: Option<String> = None;
    let mut channel_name = "stable".to_string();
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
            // channel(RXS-0185;缺省 stable,未知值经 channel::generate 拒 → 用法错误)。
            "--channel" => channel_name = take(&mut i, "--channel")?,
            // 故障注入(发布门真实红绿自检:模拟缺 SBOM / channel 漂移子门红)。
            "--simulate-missing-sbom" => faults.force_missing_sbom = true,
            "--simulate-channel-drift" => faults.force_channel_drift = true,
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

    // 生成 stable channel 清单(RXS-0185):未知 channel → 用法错误(退出码 1,
    // 零新 RX 码);digest 锚定即将写出的 bundle.json 字节流(内容寻址引用)。
    let bundle_json_str = bundle.to_json();
    let channel_manifest = channel::generate(&bundle, &channel_name, &bundle_json_str)?;

    let report = run_release(bundle, signing, channel_manifest, ci, faults);

    // 写出产物(SBOM 双视图 + bundle / channel / 签名 / 门决策清单)。
    let out = Path::new(&out_dir);
    std::fs::create_dir_all(out).map_err(|e| format!("建 out-dir 失败:{e}"))?;
    write_file(out, "sbom.spdx.json", &report.sbom.spdx)?;
    write_file(out, "sbom.cdx.json", &report.sbom.cyclonedx)?;
    write_file(out, "bundle.json", &bundle_json_str)?;
    write_file(out, "channel_manifest.json", &report.channel.to_json())?;
    write_file(out, "signing_manifest.json", &signing_json(&report))?;
    write_file(out, "gate_decision.json", &gate_json(&report))?;

    // 摘要行(冒烟脚本解析 + 人读;token 纯追加,既有 token 0-byte)。
    println!(
        "RURIXUP_RELEASE: allow_upload={} signed_artifacts={} sbom_present={} audit_pass={} channel={} channel_ok={} failed_gates=[{}]",
        report.decision.allow_upload,
        report.signed_artifacts.len(),
        report.sbom_present,
        report.audit.pass,
        report.channel.channel,
        report.channel_ok,
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
    s.push_str(&format!(
        "  \"channel_manifest_ok\": {},\n",
        report.channel_ok
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
