//! G9.3 M106 Execution Set 与 PSO 衔接 device harness(g9.p1.m106.execution_set_pso;
//! spec/gpu_driven_submit.md RXS-0355;RFC-0023 §4.2;U57)。
//!
//! ## 判据承载
//! 两条**同状态仅换 fragment shader** 的 graphics pipeline(成员 0 = 红,成员 1 =
//! 蓝)组成 execution set;DgcBuffer 两 sequence 各携 [execution-set token: 管线
//! 索引 | draw token: VkDrawIndirectCommand{3,1,0,firstInstance}],vertex shader 按
//! InstanceIndex 左右半屏偏移——GPU 侧索引切换出图(`run_execution_set_offscreen`
//! U57 lane)vs CPU 侧 `vkCmdBindPipeline` PSO 切换 golden **逐字节一致**;失效
//! (destroy)重建(同输入)后重跑 GPU 臂**逐字节一致**;左红右蓝采样点断言 =
//! 索引切换真发生的证据(区别于「两臂都画了同一 shader」的假绿)。
//!
//! ## host 面(引用 execution_set.rs / pso_cache.rs 结论进 evidence checks)
//! - ExecutionSet build → invalidate → rebuild(同输入)canonical 字节与
//!   `execution_set_identity` digest 逐位一致(RXS-0355 L3);
//! - capability 缺失 fail-closed:缺 `submit.execution_set` snapshot →
//!   `ExecSetError::CapabilityMissing` typed Err(RXS-0355 L4,RED-OK);
//! - D3D12 诚实降级:Auto → `CpuPsoSwitchDegraded` 显式登记「GPU 侧 shader 索引
//!   切换不可表达」;RequireGpuIndexSwitch → `GpuIndexSwitchInexpressible`(不静默
//!   降级为模拟)。
//!
//! ## 三态与 capability fail-closed
//! 无 loader/设备 → `VK_ES: SKIP`(dev-env degrade 登记,退 0;REQUIRE_REAL=1 翻
//! 硬红);probe 不支持 execution set(扩展/feature/上限缺失)→ **DEV_ENV_DEGRADE
//! 显式登记**,门证据主体 = host 面 capability 缺失路径(不假绿),退 0。判据
//! 不符 / validation ERROR → `VK_ES: FAIL` 退 1。evidence JSON
//! (`rurix.g9m106.execution_set.v1`)落 `--evidence <path>`(缺省
//! `evidence/g9_m106_execution_set_<UTC>.json`)。

use rurix_rt::dgc::DgcBackend;
use rurix_rt::execution_set::{
    D3D12_GPU_INDEX_SWITCH_INEXPRESSIBLE, ExecSetError, ExecutionSet, ExecutionSetMemberSpec,
    ExecutionSetPath, ExecutionSetRequest, ExecutionSetSpec, select_execution_set_path,
};
use rurix_rt::pso_cache;
use rurix_rt::vk::{
    ExecutionSetScene, probe_execution_set_capability, run_execution_set_offscreen,
};

/// 无设备/加载器(SKIP)信号(镜像 bin/vk_dgc / bin/vk_clas_rt 纪律)。
const NO_DEVICE_KEYS: &[&str] = &[
    "vulkan loader",
    "vulkan-1.dll",
    "libvulkan",
    "物理设备",
    "graphics queue",
    "vkCreateInstance",
];

/// 出图尺寸(64×64 RGBA8;左 1/4 红 = 成员 0,其余覆盖区蓝 = 成员 1 后画 wins)。
const W: u32 = 64;
const H: u32 = 64;

fn is_no_device(e: &str) -> bool {
    NO_DEVICE_KEYS.iter().any(|k| e.contains(k))
}

fn fail(msg: &str) -> ! {
    eprintln!("VK_ES: FAIL {msg}");
    std::process::exit(1)
}

// ---------------------------------------------------------------------------
// 手编 SPIR-V(沿 bin/vk_clas_rt / vk.rs mesh_witness_fs_spv 先例)
// ---------------------------------------------------------------------------

fn inst(v: &mut Vec<u32>, op: u32, ops: &[u32]) {
    v.push(op | ((ops.len() as u32 + 1) << 16));
    v.extend_from_slice(ops);
}

fn words(s: &str) -> Vec<u32> {
    let mut b = s.as_bytes().to_vec();
    b.push(0);
    while b.len() % 4 != 0 {
        b.push(0);
    }
    b.chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// 无顶点输入 vertex SPIR-V:内建 VertexIndex(42)/InstanceIndex(43)生成
/// 全屏三角形并按实例左右偏移——instance 0 → x-0.5(左),instance 1 → x+0.5(右)。
/// `gl_Position = vec4(x_base + x_off, y_base, 0, 1)`,x_base = vid==1 ? 3 : -1,
/// y_base = vid==2 ? 3 : -1,x_off = iid==0 ? -0.5 : 0.5。
fn exec_set_vs_spv() -> Vec<u32> {
    let mut v = vec![0x0723_0203u32, 0x0001_0400, 0, 128, 0];
    inst(&mut v, 17, &[1]); // OpCapability Shader
    inst(&mut v, 14, &[0, 1]); // OpMemoryModel Logical GLSL450
    let mut ep = vec![0u32, 1]; // Vertex = 0,%1
    ep.extend(words("main"));
    ep.extend_from_slice(&[9, 10, 12]); // interface:vid/iid/pos(SPIR-V 1.4 全量枚举)
    inst(&mut v, 15, &ep); // OpEntryPoint Vertex %1 "main" %9 %10 %12
    // ── 注解 ──
    inst(&mut v, 71, &[9, 11, 42]); // %9 BuiltIn VertexIndex(42)
    inst(&mut v, 71, &[10, 11, 43]); // %10 BuiltIn InstanceIndex(43)
    inst(&mut v, 71, &[12, 11, 0]); // %12 BuiltIn Position(0)
    // ── 类型 / 常量 / 全局变量 ──
    inst(&mut v, 19, &[2]); // %2 = void
    inst(&mut v, 33, &[3, 2]); // %3 = fn
    inst(&mut v, 21, &[4, 32, 0]); // %4 = u32
    inst(&mut v, 20, &[5]); // %5 = bool
    inst(&mut v, 22, &[6, 32]); // %6 = f32
    inst(&mut v, 23, &[7, 6, 4]); // %7 = vec4f
    inst(&mut v, 32, &[8, 1, 4]); // %8 = ptr Input u32
    inst(&mut v, 59, &[8, 9, 1]); // %9 = vid var(Input)
    inst(&mut v, 59, &[8, 10, 1]); // %10 = iid var(Input)
    inst(&mut v, 32, &[11, 3, 7]); // %11 = ptr Output vec4f
    inst(&mut v, 59, &[11, 12, 3]); // %12 = gl_Position var(Output)
    inst(&mut v, 43, &[4, 13, 1]); // %13 = u32 1
    inst(&mut v, 43, &[4, 14, 2]); // %14 = u32 2
    inst(&mut v, 43, &[4, 15, 0]); // %15 = u32 0
    inst(&mut v, 43, &[6, 16, 0x4040_0000]); // %16 = f32 3.0
    inst(&mut v, 43, &[6, 17, 0xBF80_0000]); // %17 = f32 -1.0
    inst(&mut v, 43, &[6, 18, 0xBF00_0000]); // %18 = f32 -0.5
    inst(&mut v, 43, &[6, 19, 0x3F00_0000]); // %19 = f32 0.5
    inst(&mut v, 43, &[6, 20, 0x0000_0000]); // %20 = f32 0.0
    inst(&mut v, 43, &[6, 21, 0x3F80_0000]); // %21 = f32 1.0
    // ── 函数体 ──
    inst(&mut v, 54, &[2, 1, 0, 3]); // %1 = OpFunction
    inst(&mut v, 248, &[100]); // label
    inst(&mut v, 61, &[4, 101, 9]); // %101 = load vid
    inst(&mut v, 61, &[4, 102, 10]); // %102 = load iid
    inst(&mut v, 170, &[5, 103, 101, 13]); // %103 = vid == 1
    inst(&mut v, 169, &[6, 104, 103, 16, 17]); // %104 = select → x_base
    inst(&mut v, 170, &[5, 105, 101, 14]); // %105 = vid == 2
    inst(&mut v, 169, &[6, 106, 105, 16, 17]); // %106 = select → y_base
    inst(&mut v, 170, &[5, 107, 102, 15]); // %107 = iid == 0
    inst(&mut v, 169, &[6, 108, 107, 18, 19]); // %108 = select → x_off
    inst(&mut v, 129, &[6, 109, 104, 108]); // %109 = x_base + x_off(OpFAdd=129)
    inst(&mut v, 80, &[7, 110, 109, 106, 20, 21]); // %110 = vec4(x,y,0,1)
    inst(&mut v, 62, &[12, 110]); // store gl_Position
    inst(&mut v, 253, &[]); // OpReturn
    inst(&mut v, 56, &[]); // OpFunctionEnd
    v
}

/// const-color fragment SPIR-V(写 `color` → location 0;镜像
/// `vk::mesh_witness_fs_spv` 结构,SPIR-V 1.4 头 + interface 枚举)。
/// id 分配:1 void / 2 fn 类型 / 3 f32 / 4 vec4f / 5 ptr / 6 out var /
/// 7,8,9,11 = 四分量常量(10 留 main)/ 12 composite / 13 label。
fn exec_set_fs_spv(color: [f32; 4]) -> Vec<u32> {
    let mut v = vec![0x0723_0203u32, 0x0001_0400, 0, 16, 0];
    inst(&mut v, 17, &[1]); // OpCapability Shader
    inst(&mut v, 14, &[0, 1]); // OpMemoryModel Logical GLSL450
    let mut ep = vec![4u32, 10]; // Fragment = 4,%10
    ep.extend(words("main"));
    ep.push(6); // interface:%6
    inst(&mut v, 15, &ep); // OpEntryPoint Fragment %10 "main" %6
    inst(&mut v, 16, &[10, 7]); // OpExecutionMode %10 OriginUpperLeft
    inst(&mut v, 71, &[6, 30, 0]); // OpDecorate %6 Location 0
    inst(&mut v, 19, &[1]); // %1 = void
    inst(&mut v, 33, &[2, 1]); // %2 = fn
    inst(&mut v, 22, &[3, 32]); // %3 = f32
    inst(&mut v, 23, &[4, 3, 4]); // %4 = vec4f
    inst(&mut v, 32, &[5, 3, 4]); // %5 = ptr Output vec4f
    inst(&mut v, 59, &[5, 6, 3]); // %6 = out color var
    let const_ids = [7u32, 8, 9, 11];
    for (i, c) in color.iter().enumerate() {
        inst(&mut v, 43, &[3, const_ids[i], c.to_bits()]); // f32 分量常量
    }
    inst(&mut v, 44, &[4, 12, 7, 8, 9, 11]); // %12 = OpConstantComposite vec4
    inst(&mut v, 54, &[1, 10, 0, 2]); // %10 = OpFunction
    inst(&mut v, 248, &[13]); // %13 = label
    inst(&mut v, 62, &[6, 12]); // OpStore %6 %12
    inst(&mut v, 253, &[]); // OpReturn
    inst(&mut v, 56, &[]); // OpFunctionEnd
    v
}

// ---------------------------------------------------------------------------
// 辅助
// ---------------------------------------------------------------------------

/// FNV-1a 64 digest(沿 rt_clas `hit_stream_digest` 体例;evidence 留痕面)。
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// unix 秒 → UTC `YYYYMMDDTHHMMSSZ`(Howard Hinnant civil-from-days;evidence 文件名用)。
fn utc_stamp(secs: u64) -> String {
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (hh, mm, ss) = (rem / 3600, (rem / 60) % 60, rem % 60);
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}{m:02}{d:02}T{hh:02}{mm:02}{ss:02}Z")
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', " ")
}

// ---------------------------------------------------------------------------
// main:host 三段 + device 三臂 + evidence
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_lines)]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    println!(
        "[vk_execution_set] G9.3 M106 Execution Set×PSO 衔接 harness(RXS-0355;门 g9.p1.m106.execution_set_pso;U57)"
    );
    let require_real = std::env::var("RURIX_REQUIRE_REAL").as_deref() == Ok("1");
    let validation_on = std::env::var("RURIX_VK_VALIDATION").as_deref() == Ok("1");
    let mut checks: Vec<(&str, bool)> = Vec::new();
    let mut degrade_notes: Vec<String> = Vec::new();

    // ── host 段(引用 execution_set.rs / pso_cache.rs 结论;不触 device)──
    // ① 失效重建确定性(RXS-0355 L3):build → invalidate → rebuild 同输入 →
    //    canonical 字节与 identity digest 逐位一致。
    let spec = ExecutionSetSpec {
        name: "m106_two_material_set".to_owned(),
        state_canonical: vec![0xAA, 0xBB, 0xCC, 0xDD],
        members: vec![
            ExecutionSetMemberSpec {
                name: "mat_red".to_owned(),
                pso_key: [0xA0; 32],
            },
            ExecutionSetMemberSpec {
                name: "mat_blue".to_owned(),
                pso_key: [0xB0; 32],
            },
        ],
    };
    let set = ExecutionSet::build(&spec).expect("合法 spec 构建");
    let identity = pso_cache::execution_set_identity(&set);
    let mut lost = set.clone();
    lost.invalidate();
    let rebuilt = ExecutionSet::rebuild(&spec, lost.version() + 1).expect("同输入重建");
    let rebuild_equal = rebuilt.canonical_bytes() == set.canonical_bytes()
        && rebuilt.members() == set.members()
        && pso_cache::execution_set_identity(&rebuilt) == identity;
    checks.push(("host_rebuild_digest_equal", rebuild_equal));
    if !rebuild_equal {
        fail("host 失效重建 digest/canonical 不一致(RXS-0355 L3)");
    }
    println!("ES_HOST_SET_IDENTITY: 0x{}", hex64(&identity));

    // ② capability 缺失 fail-closed(RXS-0355 L4;合成缺 snapshot → typed Err)。
    let cap_missing = select_execution_set_path(
        DgcBackend::Vulkan,
        ExecutionSetRequest::RequireGpuIndexSwitch,
        &["submit.dgc"],
    );
    let cap_red = matches!(&cap_missing, Err(ExecSetError::CapabilityMissing))
        && cap_missing
            .as_ref()
            .err()
            .map(|e| e.to_string().contains("submit.execution_set"))
            .unwrap_or(false);
    checks.push(("host_capability_missing_fail_closed", cap_red));
    if !cap_red {
        fail("capability 缺失未返 CapabilityMissing(RXS-0355 L4)");
    }
    println!("VK_ES: RED-OK capability-missing(CapabilityMissing typed Err)");

    // ③ D3D12 诚实降级(RXS-0355 L4):Auto → CpuPsoSwitchDegraded 显式登记;
    //    Require → GpuIndexSwitchInexpressible(不静默模拟)。
    let d3_auto = select_execution_set_path(
        DgcBackend::D3D12,
        ExecutionSetRequest::Auto,
        &["submit.execution_set"],
    );
    let d3_degrade_ok = matches!(
        &d3_auto,
        Ok(ExecutionSetPath::CpuPsoSwitchDegraded(reg))
            if reg.backend == DgcBackend::D3D12 && reg.fact == D3D12_GPU_INDEX_SWITCH_INEXPRESSIBLE
    );
    let d3_require = select_execution_set_path(
        DgcBackend::D3D12,
        ExecutionSetRequest::RequireGpuIndexSwitch,
        &["submit.execution_set"],
    );
    let d3_require_ok = matches!(
        d3_require,
        Err(ExecSetError::GpuIndexSwitchInexpressible {
            backend: DgcBackend::D3D12
        })
    );
    checks.push(("host_d3d12_honest_degradation_registered", d3_degrade_ok));
    checks.push(("host_d3d12_require_inexpressible", d3_require_ok));
    if !d3_degrade_ok || !d3_require_ok {
        fail("D3D12 诚实降级/不可表达路径不符(RXS-0355 L4)");
    }
    println!("VK_ES: RED-OK d3d12-degrade(登记「GPU 侧 shader 索引切换不可表达」)");

    // ── device 段:capability 探测 → 三臂真跑 ──
    let mut device_state = "skipped_dev_env";
    let mut digests: Vec<(String, String)> = Vec::new();
    let mut probe_line = String::new();
    match probe_execution_set_capability() {
        Err(e) => {
            degrade_notes.push(format!("DEV_ENV_DEGRADE: probe 不可用({})", e.trim()));
            println!("VK_ES: SKIP probe 不可用({})", e.trim());
        }
        Ok(report) => {
            probe_line = report.summary_line();
            println!("ES_CAP: {probe_line}");
            if !report.execution_set_supported() {
                // capability fail-closed(RXS-0355 L2):显式 DEV_ENV_DEGRADE 登记,
                // 门证据主体 = host 面 capability 缺失路径(不假绿)。
                device_state = "degraded_capability_missing";
                degrade_notes.push(format!(
                    "DEV_ENV_DEGRADE: execution set capability 缺失 missing={:?};门证据主体=host capability 缺失路径(RXS-0355 L2/L4)",
                    report.missing()
                ));
                println!("VK_ES: {}", degrade_notes.last().expect("刚 push"));
            } else {
                let scene = ExecutionSetScene {
                    vs_spv: &exec_set_vs_spv(),
                    fs_spv_a: &exec_set_fs_spv([1.0, 0.0, 0.0, 1.0]),
                    fs_spv_b: &exec_set_fs_spv([0.0, 0.0, 1.0, 1.0]),
                    width: W,
                    height: H,
                    clear: [0.0, 0.0, 0.0, 1.0],
                };
                match run_execution_set_offscreen(&scene) {
                    Ok(out) => {
                        device_state = "executed";
                        // ④ GPU 索引切换臂 vs CPU PSO 切换 golden 逐字节一致。
                        let gpu_vs_cpu = out.pixels_gpu == out.pixels_cpu;
                        checks.push(("device_gpu_vs_cpu_pso_byte_exact", gpu_vs_cpu));
                        if !gpu_vs_cpu {
                            let diff = out
                                .pixels_gpu
                                .iter()
                                .zip(out.pixels_cpu.iter())
                                .position(|(a, b)| a != b);
                            fail(&format!(
                                "GPU 索引切换臂 vs CPU PSO 切换逐字节不一致(首分叉字节 {diff:?})"
                            ));
                        }
                        // ⑤ 失效重建确定性:重建 set 重跑 GPU 臂逐字节一致。
                        let rebuild_eq = out.pixels_rebuild == out.pixels_gpu;
                        checks.push(("device_rebuild_render_equal", rebuild_eq));
                        if !rebuild_eq {
                            fail("失效重建臂与 GPU 臂逐字节不一致(RXS-0355 L3)");
                        }
                        // ⑥ 索引切换真发生:左采样点 = 成员 0 红,右采样点 = 成员 1 蓝。
                        let px = |buf: &[u8], x: u32, y: u32| -> [u8; 4] {
                            let o = ((y * W + x) * 4) as usize;
                            [buf[o], buf[o + 1], buf[o + 2], buf[o + 3]]
                        };
                        let left = px(&out.pixels_gpu, W / 8, H / 2);
                        let right = px(&out.pixels_gpu, 7 * W / 8, H / 2);
                        let switch_observed = left == [255, 0, 0, 255] && right == [0, 0, 255, 255];
                        checks.push(("device_index_switch_observed", switch_observed));
                        if !switch_observed {
                            fail(&format!(
                                "索引切换证据缺失:左={left:?} 右={right:?}(期望 [255,0,0,255]/[0,0,255,255])"
                            ));
                        }
                        digests
                            .push(("gpu".into(), format!("0x{:016x}", fnv1a64(&out.pixels_gpu))));
                        digests
                            .push(("cpu".into(), format!("0x{:016x}", fnv1a64(&out.pixels_cpu))));
                        digests.push((
                            "rebuild".into(),
                            format!("0x{:016x}", fnv1a64(&out.pixels_rebuild)),
                        ));
                        println!(
                            "ES_DIGESTS: gpu={} cpu={} rebuild={} preprocess_size={} maxPipelineCount={}",
                            digests[0].1,
                            digests[1].1,
                            digests[2].1,
                            out.preprocess_size,
                            out.max_indirect_pipeline_count
                        );
                    }
                    Err(e) if is_no_device(&e) => {
                        degrade_notes
                            .push(format!("DEV_ENV_DEGRADE: device 真跑不可用({})", e.trim()));
                        println!("VK_ES: SKIP device 真跑不可用({})", e.trim());
                    }
                    Err(e) => fail(&format!("run_execution_set_offscreen: {e}")),
                }
            }
        }
    }
    checks.push(("device_validation_zero", device_state == "executed"));

    // ── evidence JSON(rurix.g9m106.execution_set.v1)──
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let ts = utc_stamp(secs);
    let host_all = checks
        .iter()
        .filter(|(k, _)| k.starts_with("host_"))
        .all(|(_, c)| *c);
    let device_ok = device_state == "executed";
    let degrade_registered = !degrade_notes.is_empty();
    // PASS 口径:device executed → 全 checks 绿;degrade/skip → host 三段绿 +
    // degrade 显式登记(门证据主体,不假绿——device checks 不冒充 true)。
    let status = if device_ok && checks.iter().all(|(_, c)| *c) {
        "pass"
    } else if !device_ok && host_all && degrade_registered {
        "pass_with_dev_env_degrade"
    } else {
        "fail"
    };
    let checks_json = checks
        .iter()
        .map(|(k, v)| format!("    \"{k}\": {v}"))
        .collect::<Vec<_>>()
        .join(",\n");
    let digests_json = digests
        .iter()
        .map(|(k, v)| format!("    \"{k}\": \"{v}\""))
        .collect::<Vec<_>>()
        .join(",\n");
    let degrade_json = if degrade_notes.is_empty() {
        "null".to_owned()
    } else {
        format!(
            "[{}]",
            degrade_notes
                .iter()
                .map(|n| format!("\"{}\"", json_escape(n)))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let base_commit = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_default();
    let evidence = format!(
        "{{\n  \"schema\": \"rurix.g9m106.execution_set.v1\",\n  \
         \"gate\": \"g9.p1.m106.execution_set_pso\",\n  \"spec\": \"RXS-0355\",\n  \
         \"status\": \"{status}\",\n  \"device_state\": \"{device_state}\",\n  \
         \"base_commit\": \"{base_commit}\",\n  \"timestamp\": \"{ts}\",\n  \
         \"validation_mode\": \"{}\",\n  \"checks\": {{\n{checks_json}\n  }},\n  \
         \"digests\": {{\n{digests_json}\n  }},\n  \
         \"host_set_identity\": \"0x{}\",\n  \
         \"probe\": \"{}\",\n  \
         \"commands\": [\n    \
         \"cargo build -p rurix-rt --features vulkan --bin vk_execution_set\",\n    \
         \"vk_execution_set (host 三段 + probe + run_execution_set_offscreen 三臂)\"\n  ],\n  \
         \"dev_env_degrade\": {degrade_json}\n}}",
        if validation_on { "on" } else { "off" },
        hex64(&pso_cache::execution_set_identity(&set)),
        json_escape(&probe_line),
    );
    let default_path = format!("evidence/g9_m106_execution_set_{ts}.json");
    let ev_path = args
        .iter()
        .position(|a| a == "--evidence")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or(default_path);
    if let Err(e) = std::fs::write(&ev_path, format!("{evidence}\n")) {
        eprintln!("VK_ES: 写 evidence {ev_path} 失败: {e}");
    } else {
        println!("VK_ES: evidence → {ev_path}");
    }
    println!("{evidence}");

    match status {
        "pass" => {
            println!(
                "VK_ES: PASS gpu==cpu==rebuild 逐字节一致 index_switch[左红右蓝]=OK \
                 host[rebuild_digest,capability_missing,d3d12_degrade]=OK validation={} digest={}",
                if validation_on { "on(0)" } else { "off" },
                digests.first().map(|(_, d)| d.as_str()).unwrap_or("n/a"),
            );
        }
        "pass_with_dev_env_degrade" => {
            if require_real {
                fail("device SKIP/degrade(RURIX_REQUIRE_REAL=1 翻硬红)");
            }
            println!(
                "VK_ES: PASS(dev-env degrade 已显式登记;门证据主体=host capability 缺失路径,不假绿)"
            );
        }
        _ => fail("checks 未全绿且无 degrade 登记"),
    }
}

/// 32B digest → hex 串(evidence 留痕)。
fn hex64(b: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}
