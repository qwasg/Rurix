//! DLSS 5 Neural Rendering(NGX feature id 18)D3D12 直驱可用性探针 harness
//! (artifacts/day_0830_dlss5nr Phase 1)。
//!
//! 经 [`rurix_rt::vendor_upscale::NrDx12Probe::run`] 真跑驱动侧 NGX core 装载 +
//! snippet(nvngx_dlssnr.dll,40系 Ada 变体)定位 + Init + GetCapabilityParameters
//! (vtable 逆序自检)+ CreateFeature(18) 双臂(core 标准路 / 直驱 snippet 破签
//! 绕行路),产出 evidence JSON(schema `rurix.dlss5nr.probe.v1`)。fail-closed。
//!
//! 用法:
//!   g13_dlss5_nr_probe                       # 报告 JSON → stdout(默认 1920x1080)
//!   g13_dlss5_nr_probe --out <path>          # 落盘(LF;父目录须存在)
//!   g13_dlss5_nr_probe --size 2560x1440      # 指定 in==out 分辨率(NR 不上采样)
//!   RURIX_DLSS5NR_SDK_DIR=<dir>              # snippet 目录覆盖(默认 external/ 40系变体)
//!   RURIX_NVNGX_CORE_DLL=<_nvngx.dll>        # NGX core 显式路径覆盖
//!   RURIX_REQUIRE_REAL=1                      # verdict=not_available → 退 1(硬红)
//!
//! 数字纪律:全字段来自真实 NGX 调用结果码 + NGX 官方日志回调原文(非 mock)。
//! 泄露件 evaluation-only,default off,env opt-in。

use rurix_rt::vendor_upscale::NrDx12Probe;

const REPORT_SCHEMA: &str = "rurix.dlss5nr.probe.v1";

/// JSON 串转义(vk_capability_report `jesc` 同一防御面)。
fn jesc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn parse_size(s: &str) -> Option<(u32, u32)> {
    let (w, h) = s.split_once(['x', 'X', '*'])?;
    Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
}

fn main() {
    let mut out_path: Option<String> = None;
    let mut size = (1920u32, 1080u32);
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                i += 1;
                out_path = Some(
                    args.get(i)
                        .unwrap_or_else(|| {
                            eprintln!("DLSS5NR_PROBE: FAIL --out 缺路径");
                            std::process::exit(2)
                        })
                        .clone(),
                );
            }
            "--size" => {
                i += 1;
                size = args
                    .get(i)
                    .and_then(|s| parse_size(s))
                    .unwrap_or_else(|| {
                        eprintln!("DLSS5NR_PROBE: FAIL --size 需 WxH(如 1920x1080)");
                        std::process::exit(2)
                    });
            }
            other => {
                eprintln!("DLSS5NR_PROBE: FAIL 未知参数 {other}(闭集 [--out <path>] [--size WxH])");
                std::process::exit(2);
            }
        }
        i += 1;
    }
    let require_real = std::env::var("RURIX_REQUIRE_REAL").ok().as_deref() == Some("1");

    let report = match NrDx12Probe::run(size) {
        Ok(r) => r,
        Err(e) => {
            // 探针基建失败(snippet/core 缺失、设备不可用)= 环境降级,如实登记非崩溃。
            let mut s = String::new();
            s.push_str("{\n");
            s.push_str(&format!("  \"schema\": \"{REPORT_SCHEMA}\",\n"));
            s.push_str("  \"verdict\": \"probe_setup_error\",\n");
            s.push_str(&format!("  \"error\": \"{}\"\n", jesc(&e.to_string())));
            s.push_str("}\n");
            match &out_path {
                Some(p) => {
                    if std::fs::write(p, &s).is_err() {
                        eprintln!("DLSS5NR_PROBE: FAIL 落盘 {p}");
                        std::process::exit(1);
                    }
                }
                None => print!("{s}"),
            }
            eprintln!("DLSS5NR_PROBE: setup_error {e}");
            std::process::exit(if require_real { 1 } else { 0 });
        }
    };

    // ── JSON 装配(键序固定、UTF-8、LF)──
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str(&format!("  \"schema\": \"{REPORT_SCHEMA}\",\n"));
    s.push_str(&format!("  \"verdict\": \"{}\",\n", report.verdict));
    s.push_str(&format!(
        "  \"verdict_basis\": \"{}\",\n",
        jesc(&report.verdict_basis)
    ));
    s.push_str(&format!("  \"gpu\": \"{}\",\n", jesc(&report.gpu_name)));
    s.push_str(&format!(
        "  \"in_size\": [{}, {}],\n",
        report.in_size.0, report.in_size.1
    ));
    s.push_str(&format!(
        "  \"out_size\": [{}, {}],\n",
        report.out_size.0, report.out_size.1
    ));
    s.push_str(&format!(
        "  \"core_dll\": {{\"name\": \"{}\", \"sha256\": \"{}\", \"bytes\": {}}},\n",
        jesc(&report.core_dll.name),
        report.core_dll.sha256,
        report.core_dll.bytes
    ));
    s.push_str(&format!(
        "  \"snippet_dll\": {{\"name\": \"{}\", \"sha256\": \"{}\", \"bytes\": {}}},\n",
        jesc(&report.snippet_dll.name),
        report.snippet_dll.sha256,
        report.snippet_dll.bytes
    ));
    s.push_str(&format!(
        "  \"vtable_selfcheck_ok\": {},\n",
        report.vtable_selfcheck_ok
    ));
    s.push_str(&format!("  \"snippet_loaded\": {},\n", report.snippet_loaded));
    s.push_str(&format!(
        "  \"create_feature_core_ok\": {},\n",
        report.create_feature_core_ok
    ));
    s.push_str(&format!(
        "  \"create_feature_direct_ok\": {},\n",
        report.create_feature_direct_ok
    ));
    match &report.feature_requirement {
        Some((sup, arch, os)) => s.push_str(&format!(
            "  \"feature_requirement\": {{\"supported_bitfield\": {sup}, \"min_hw_arch\": {arch}, \"min_os\": \"{}\"}},\n",
            jesc(os)
        )),
        None => s.push_str("  \"feature_requirement\": null,\n"),
    }
    s.push_str("  \"steps\": [\n");
    for (k, st) in report.steps.iter().enumerate() {
        s.push_str(&format!(
            "    {{\"step\": \"{}\", \"result\": {}, \"result_name\": \"{}\"}}{}\n",
            jesc(&st.step),
            st.result,
            jesc(&st.result_name),
            if k + 1 < report.steps.len() { "," } else { "" }
        ));
    }
    s.push_str("  ],\n");
    s.push_str("  \"ngx_log\": [\n");
    for (k, line) in report.ngx_log.iter().enumerate() {
        s.push_str(&format!(
            "    \"{}\"{}\n",
            jesc(line),
            if k + 1 < report.ngx_log.len() { "," } else { "" }
        ));
    }
    s.push_str("  ]\n");
    s.push_str("}\n");

    match out_path {
        Some(p) => {
            if let Err(e) = std::fs::write(&p, &s) {
                eprintln!("DLSS5NR_PROBE: FAIL 落盘 {p}: {e}");
                std::process::exit(1);
            }
            println!(
                "DLSS5NR_PROBE: verdict={} core_arm={} direct_arm={} snippet_loaded={} vtable_ok={} out={p}",
                report.verdict,
                report.create_feature_core_ok,
                report.create_feature_direct_ok,
                report.snippet_loaded,
                report.vtable_selfcheck_ok
            );
        }
        None => print!("{s}"),
    }
    if require_real && report.verdict == "not_available" {
        eprintln!("DLSS5NR_PROBE: FAIL RURIX_REQUIRE_REAL=1 但 NR 本机不可用");
        std::process::exit(1);
    }
}
