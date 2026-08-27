//! G31+ 波 C Task C3:统一运行时能力探测聚合面 harness(设备兼容矩阵与能力降级链
//! 系统化;`G31_PLUS_COMMERCIAL_RENDERER_TODO` §5 #50 兑现载体)。
//!
//! 聚合既有探测逻辑为**单一 capability report**(JSON,schema
//! `rurix.g31.capability_report.v1`),不重复造:
//! 1. **设备能力面** = [`rurix_rt::vk::probe_device_capability`](vk.rs G31 C3 聚合段,
//!    逐物理设备:vendor/device id、RT/RayQuery、mesh shader、descriptor 面上限、
//!    显存 heap/budget、feature 链九节点);
//! 2. **DLSS 可用性** = `streamline_sdk_dir` + `DlssVkSession::create` 真建 session
//!    (320×180→640×360,G13 M-a 实测口径;DLL 在树/装载/NGX init/feature 创建
//!    全链 fail-closed,Err 原文如实进 detail);
//! 3. **FSR 可用性** = `fsr_sdk_dir` + `FsrDx12Session::create` 同口径(D3D12 臂);
//! 4. **TSR** = 自研恒可用面(kernels/g13_tsr_{resample,resolve}.rx 经
//!    `vk::run_compute`;需求 = Vulkan compute,不发起额外探测,设备面非空即
//!    available,与 G13/G14 生产车道事实同源)。
//!
//! 用法:
//!   vk_capability_report                 # 报告 JSON 输出到 stdout
//!   vk_capability_report --out <path>    # 落盘(LF;父目录须存在)
//!   RURIX_REQUIRE_REAL=1 vk_capability_report  # 任一面不可用 → 退 1(硬红)
//!
//! 三态:vulkan loader/设备缺失或 vendor SDK 缺失 = 报告 `state=dev_env_degrade`
//! 逐面如实登记(不冒充 available);进程退码:无 `--require-real` 恒 0(报告即
//! 产物),`RURIX_REQUIRE_REAL=1` 时任一必需面缺失退 1。
//!
//! 数字纪律:报告全字段来自真实探测输出(vendor SDK 路径经 `streamline_sdk_dir`/
//! `fsr_sdk_dir` 解析真值;session 创建为真建真毁,非 mock)。

use rurix_rt::vendor_upscale::{DlssVkSession, FsrDx12Session, fsr_sdk_dir, streamline_sdk_dir};
use rurix_rt::vk::probe_device_capability;

/// 报告 schema 标识(milestones/g31/g31_capability_fallback_evidence_schema.json
/// 消费面同字面;check_schemas 路由前缀 `g31_capability_fallback_`)。
const REPORT_SCHEMA: &str = "rurix.g31.capability_report.v1";

/// vendor 探测分辨率(G13 M-a harness 实测口径;NGX/FFX 均支持)。
const PROBE_IN: (u32, u32) = (320, 180);
const PROBE_OUT: (u32, u32) = (640, 360);

/// JSON 串转义(vk.rs `cap_json_escape` 同一防御面;bin 自持有副本)。
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

/// 单 vendor 后端探测结果(JSON 渲染直消费)。
struct BackendProbe {
    available: bool,
    detail: String,
    sdk_dir: String,
}

/// DLSS 可用性探测(真建 session;任一环节 Err → available=false + 原文 detail)。
fn probe_dlss() -> BackendProbe {
    match streamline_sdk_dir() {
        Ok(dir) => {
            let dir_text = dir.display().to_string().replace('\\', "/");
            match DlssVkSession::create(&dir, PROBE_IN, PROBE_OUT, false) {
                Ok(session) => {
                    let r = session.report();
                    BackendProbe {
                        available: true,
                        detail: format!(
                            "DlssVkSession::create Ok({}x{}→{}x{};ngx={};dlls={} 件 provenance sha256 在案;gpu={})",
                            PROBE_IN.0,
                            PROBE_IN.1,
                            PROBE_OUT.0,
                            PROBE_OUT.1,
                            r.engine_version,
                            r.dlls.len(),
                            r.gpu_name
                        ),
                        sdk_dir: dir_text,
                    }
                }
                Err(e) => BackendProbe {
                    available: false,
                    detail: format!("DlssVkSession::create Err: {e}"),
                    sdk_dir: dir_text,
                },
            }
        }
        Err(e) => BackendProbe {
            available: false,
            detail: format!("streamline_sdk_dir Err: {e}"),
            sdk_dir: String::new(),
        },
    }
}

/// FSR 可用性探测(D3D12 臂;真建 session,同 DLSS 口径)。
fn probe_fsr() -> BackendProbe {
    match fsr_sdk_dir() {
        Ok(dir) => {
            let dir_text = dir.display().to_string().replace('\\', "/");
            match FsrDx12Session::create(&dir, PROBE_IN, PROBE_OUT, false) {
                Ok(session) => {
                    let r = session.report();
                    BackendProbe {
                        available: true,
                        detail: format!(
                            "FsrDx12Session::create Ok({}x{}→{}x{};provider={};dlls={} 件 provenance sha256 在案;gpu={})",
                            PROBE_IN.0,
                            PROBE_IN.1,
                            PROBE_OUT.0,
                            PROBE_OUT.1,
                            r.engine_version,
                            r.dlls.len(),
                            r.gpu_name
                        ),
                        sdk_dir: dir_text,
                    }
                }
                Err(e) => BackendProbe {
                    available: false,
                    detail: format!("FsrDx12Session::create Err: {e}"),
                    sdk_dir: dir_text,
                },
            }
        }
        Err(e) => BackendProbe {
            available: false,
            detail: format!("fsr_sdk_dir Err: {e}"),
            sdk_dir: String::new(),
        },
    }
}

fn main() {
    let mut out_path: Option<String> = None;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                i += 1;
                out_path = Some(
                    args.get(i)
                        .unwrap_or_else(|| {
                            eprintln!("CAPABILITY_REPORT: FAIL --out 缺路径");
                            std::process::exit(2)
                        })
                        .clone(),
                );
            }
            other => {
                eprintln!("CAPABILITY_REPORT: FAIL 未知参数 {other}(闭集 [--out <path>])");
                std::process::exit(2);
            }
        }
        i += 1;
    }
    let require_real = std::env::var("RURIX_REQUIRE_REAL").ok().as_deref() == Some("1");

    // ── ① 设备能力聚合面(逐物理设备)──
    let (devices_json, device_count, vk_err) = match probe_device_capability() {
        Ok(reports) => {
            let n = reports.len();
            let body = reports
                .iter()
                .map(|r| r.to_json())
                .collect::<Vec<_>>()
                .join(",\n");
            (body, n, String::new())
        }
        Err(e) => (String::new(), 0, e),
    };

    // ── ②/③ vendor 双臂真建探测(DLSS Vulkan interop / FSR D3D12)──
    let dlss = probe_dlss();
    let fsr = probe_fsr();

    // ── ④ TSR 自研恒可用面(需求 = Vulkan compute;设备面非空即 available)──
    let tsr_available = device_count > 0;
    let tsr_detail = if tsr_available {
        "自研恒可用(kernels/g13_tsr_{resample,resolve}.rx 经 vk::run_compute;需求 = Vulkan compute,设备面非空实测在位)".to_owned()
    } else {
        format!("Vulkan 设备面缺失({vk_err});TSR 不可用如实登记(自研臂不冒充)")
    };

    // ── 报告装配(键序固定、UTF-8、LF;无绝对路径注入——sdk_dir 为环境真值,
    // 报告属非 stable 事实面不进 canonical 产物,RXS-0351 L9 同律)──
    let state = if vk_err.is_empty() && dlss.available && fsr.available {
        "measured"
    } else {
        "dev_env_degrade"
    };
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str(&format!("  \"schema\": \"{REPORT_SCHEMA}\",\n"));
    s.push_str(&format!("  \"state\": \"{state}\",\n"));
    s.push_str("  \"probes\": [\n");
    s.push_str("    \"rurix_rt::vk::probe_device_capability(instance 级逐物理设备:身份/扩展/feature 链九节点/limits/显存 budget;U56 探测段同体例不建 device 句柄)\",\n");
    s.push_str(&format!(
        "    \"rurix_rt::vendor_upscale::DlssVkSession::create({}x{}→{}x{};Streamline 2.10.3 Vulkan interop 臂全链真建)\",\n",
        PROBE_IN.0, PROBE_IN.1, PROBE_OUT.0, PROBE_OUT.1
    ));
    s.push_str(&format!(
        "    \"rurix_rt::vendor_upscale::FsrDx12Session::create({}x{}→{}x{};FidelityFX SDK 2.0.0 / FSR 3.1.5 D3D12 臂全链真建)\",\n",
        PROBE_IN.0, PROBE_IN.1, PROBE_OUT.0, PROBE_OUT.1
    ));
    s.push_str("    \"TSR 自研恒可用面(不发起额外探测;与 G13.3 M-b/G14 生产车道事实同源)\"\n");
    s.push_str("  ],\n");
    if vk_err.is_empty() {
        s.push_str("  \"vulkan_probe\": {\"ok\": true, \"error\": null},\n");
    } else {
        s.push_str(&format!(
            "  \"vulkan_probe\": {{\"ok\": false, \"error\": \"{}\"}},\n",
            jesc(&vk_err)
        ));
    }
    if device_count == 0 {
        s.push_str("  \"devices\": [],\n");
    } else {
        s.push_str("  \"devices\": [\n");
        s.push_str(&devices_json);
        s.push_str("\n  ],\n");
    }
    s.push_str("  \"upscale\": {\n");
    s.push_str(&format!(
        "    \"dlss_sr\": {{\"available\": {}, \"detail\": \"{}\", \"sdk_dir\": \"{}\"}},\n",
        dlss.available,
        jesc(&dlss.detail),
        jesc(&dlss.sdk_dir)
    ));
    s.push_str(&format!(
        "    \"fsr_3_1_5\": {{\"available\": {}, \"detail\": \"{}\", \"sdk_dir\": \"{}\"}},\n",
        fsr.available,
        jesc(&fsr.detail),
        jesc(&fsr.sdk_dir)
    ));
    s.push_str(&format!(
        "    \"tsr_device\": {{\"available\": {}, \"detail\": \"{}\"}}\n",
        tsr_available,
        jesc(&tsr_detail)
    ));
    s.push_str("  }\n");
    s.push_str("}\n");

    match out_path {
        Some(p) => {
            if let Err(e) = std::fs::write(&p, &s) {
                eprintln!("CAPABILITY_REPORT: FAIL 落盘 {p}: {e}");
                std::process::exit(1);
            }
            println!(
                "CAPABILITY_REPORT: PASS state={state} devices={device_count} dlss={} fsr={} tsr={} out={p}",
                dlss.available, fsr.available, tsr_available
            );
        }
        None => print!("{s}"),
    }
    if require_real && state != "measured" {
        eprintln!(
            "CAPABILITY_REPORT: FAIL RURIX_REQUIRE_REAL=1 但能力面降级(vk_err={vk_err} dlss={} fsr={})",
            dlss.available, fsr.available
        );
        std::process::exit(1);
    }
}
