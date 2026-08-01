//! uc09-taichi-spike — G6.5 Taichi Vulkan AOT spike demo(RFC-0017 §4.E;工程形态照 uc08)。
//!
//! host 腿(恒跑,无 GPU/DLL 也绿):`assets/particles.tcm` + `.sha256` 实测 hash
//! 核验 + 生成脚本在树;RenderGraph `import` 外部资源 `taichi_particles`(Buffer,
//! 256B = 64×f32,device 腿 = TiRT 导出 VkBuffer 的只读引用,不入 transient 池)
//! → `create` transient `particles_copy` → 单 pass 录 `cmd.copy(imported, copy,
//! 256)` → 真编译 → CommandLog 含 copy 记录断言。
//!
//! device 腿(feature `taichi-tirt`,恒尝试):`run_particles_spike` 全链(launch →
//! 导出 VkBuffer → readback)+ spec §4.E3 四段闭合断言。裁决逐字仿 uc08:
//! `RURIX_REQUIRE_REAL=1` 时任何 device 失败硬红 exit 非零;仅 provisioning 缺失
//! (缺 taichi_c_api.dll / 无 Vulkan 设备)且非 REQUIRE_REAL 才 SKIP 降级
//! (dev-env degrade,不充绿);真失败/断言失败永远硬红。feature 未开 → device 腿
//! 报 `feature_off` skip(不红)。
//!
//! CLI:`uc09-taichi-spike [--json]` —— `--json` 输出单行 JSON(smoke 脚本消费,
//! 字段集冻结);exit 0 仅当全部断言过(device skip/feature_off 不入判定);
//! exit 1 = 断言红/运行错/device 腿硬红;exit 2 = CLI 错。

#[cfg(feature = "taichi-tirt")]
mod device;
mod host;
mod sha256;

use host::HostSummary;

/// 运行模式(device 腿是否随本二进制启用;JSON `mode` 字段)。
#[cfg(feature = "taichi-tirt")]
const MODE: &str = "device";
#[cfg(not(feature = "taichi-tirt"))]
const MODE: &str = "host";

/// CLI 参数(解析确定性;未知参数 = Err)。
#[derive(Debug, Clone, Default)]
struct Cli {
    json: bool,
}

fn parse_cli(args: &[String]) -> Result<Cli, String> {
    let mut c = Cli::default();
    for a in args {
        match a.as_str() {
            "--json" => c.json = true,
            other => return Err(format!("未知参数 {other}")),
        }
    }
    Ok(c)
}

/// JSON 字符串转义(device_name 自由文本 / 诊断文本含反斜杠路径与引号)。
fn json_escape(s: &str) -> String {
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

/// 断言面 → JSON 对象体内文(键序 = 声明序,冻结)。
fn asserts_json(asserts: &[(String, bool)]) -> String {
    asserts
        .iter()
        .map(|(k, v)| format!("\"{k}\":{v}"))
        .collect::<Vec<_>>()
        .join(",")
}

/// device 腿 → JSON 对象(measured 字段 + 断言面;`{v:?}` 保留 f32 小数形态)。
#[cfg(feature = "taichi-tirt")]
fn device_leg_json(l: &device::DeviceLeg) -> String {
    let first_values: Vec<String> = l.first_values.iter().map(|v| format!("{v:?}")).collect();
    format!(
        "{{\"device_name\":\"{}\",\"particle_count\":{},\"nonzero_count\":{},\"exported_buffer_size\":{},\"first_values\":[{}],\"asserts\":{{{}}}}}",
        json_escape(&l.device_name),
        l.particle_count,
        l.nonzero_count,
        l.exported_buffer_size,
        first_values.join(","),
        asserts_json(&l.asserts)
    )
}

/// 单行 JSON(smoke 消费;字段集冻结,逐字仿 uc08 形态)。
fn summary_json(
    s: &HostSummary,
    device_field: &str,
    device_status: &str,
    device_skip_reason: Option<&str>,
    exit_ok: bool,
) -> String {
    let reason =
        device_skip_reason.map_or_else(|| "null".to_owned(), |r| format!("\"{}\"", json_escape(r)));
    format!(
        "{{\"subject\":\"uc09_taichi_spike\",\"mode\":\"{MODE}\",\"tcm_bytes\":{},\"tcm_sha256\":\"{}\",\"registered_sha256\":\"{}\",\"gen_script_present\":{},\"asserts\":{{{}}},\"graph\":{{\"pass_count\":{},\"resource_count\":{},\"copy_byte_size\":{}}},\"device\":{},\"device_status\":\"{}\",\"device_skip_reason\":{},\"exit_ok\":{}}}",
        s.tcm.len(),
        s.tcm_sha256,
        s.registered_sha256,
        s.gen_script_present,
        asserts_json(&s.asserts),
        s.graph_pass_count,
        s.graph_resource_count,
        s.copy_byte_size,
        device_field,
        device_status,
        reason,
        exit_ok
    )
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cli = match parse_cli(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("uc09-taichi-spike: {e}");
            std::process::exit(2);
        }
    };

    // host 腿(恒跑;硬失败 = 仓内资产缺失/图编译确定性拒,直接红)。
    let summary = match host::run_host_leg() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("uc09-taichi-spike: host 腿失败: {e}");
            std::process::exit(1);
        }
    };

    // device 腿(feature taichi-tirt 恒尝试;裁决逐字仿 uc08:
    // RURIX_REQUIRE_REAL=1 → 硬红;仅 provisioning 缺失且非 REQUIRE_REAL →
    // SKIP 降级不充绿;真失败永远硬红。断言失败仍出 JSON 后经 exit_ok 翻硬红)。
    #[cfg(feature = "taichi-tirt")]
    let (device_field, device_status, device_skip_reason, device_ok) =
        match device::run_device_leg(&summary.tcm, summary.copy_byte_size) {
            Ok(leg) => {
                let ok = leg.asserts_pass();
                (device_leg_json(&leg), "ok", None, ok)
            }
            Err(e) => {
                let require_real = std::env::var("RURIX_REQUIRE_REAL").ok().as_deref() == Some("1");
                if require_real || !device::is_provisioning_missing(&e) {
                    eprintln!(
                        "uc09-taichi-spike: device 腿失败(回归硬红;仅 provisioning 缺失可降级): {e}"
                    );
                    std::process::exit(1);
                }
                eprintln!("uc09-taichi-spike: device 腿降级(dev-env degrade,不充绿): {e}");
                ("null".to_owned(), "skip", Some(e.to_string()), true)
            }
        };
    #[cfg(not(feature = "taichi-tirt"))]
    let (device_field, device_status, device_skip_reason, device_ok) = (
        "null".to_owned(),
        "feature_off",
        Some("feature taichi-tirt 未启用(--features taichi-tirt)".to_owned()),
        true,
    );

    let exit_ok = summary.host_asserts_pass() && device_ok;
    let json = summary_json(
        &summary,
        &device_field,
        device_status,
        device_skip_reason.as_deref(),
        exit_ok,
    );
    if cli.json {
        println!("{json}");
    } else {
        println!("uc09-taichi-spike OK: {}", summary.one_line());
        println!("{json}");
    }
    if !exit_ok {
        eprintln!("uc09-taichi-spike: 断言未全过(见 JSON asserts)");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parse_defaults_and_json() {
        let c = parse_cli(&[]).unwrap();
        assert!(!c.json);
        let c = parse_cli(&["--json".into()]).unwrap();
        assert!(c.json);
    }

    #[test]
    fn cli_parse_rejects_unknown() {
        assert!(parse_cli(&["--bogus".into()]).is_err());
        assert!(parse_cli(&["--json".into(), "extra".into()]).is_err());
    }

    #[test]
    fn json_escape_covers_quotes_backslash_controls() {
        assert_eq!(json_escape("a\"b\\c\nd"), "a\\\"b\\\\c\\nd");
        assert_eq!(json_escape("纯文本"), "纯文本");
        assert_eq!(json_escape("\u{0001}"), "\\u0001");
    }

    /// 单行 JSON 形态(host 模式恒可核:单行 + 关键冻结字段在位)。
    #[test]
    fn summary_json_is_single_line_and_frozen_shape() {
        let s = host::run_host_leg().expect("host 腿");
        let json = summary_json(&s, "null", "feature_off", None, true);
        assert!(!json.contains('\n'), "JSON 须单行");
        for key in [
            "\"subject\":\"uc09_taichi_spike\"",
            "\"tcm_sha256\":\"",
            "\"graph\":{",
            "\"copy_byte_size\":256",
            "\"device\":null",
            "\"device_status\":\"feature_off\"",
            "\"exit_ok\":true",
        ] {
            assert!(json.contains(key), "JSON 缺冻结字段 {key}: {json}");
        }
    }
}
