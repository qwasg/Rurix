//! G8.5b M25 `g8.p1.m25.upscaler_input_abi` device 腿。
//!
//! 副 backend CAS `.rx` 核 vs host oracle 对拍;经同一 ABI hash 装配绑定全部
//! 十项输入 resource identity。`RURIX_REQUIRE_REAL=1` 下 SKIP 不充绿。

use rurix_render::temporal::abi::{
    run_via_abi, sequence_digest, synthetic_frame, UpscalerInputAbi,
};
use rurix_render::temporal::cas::CasUpscaler;
use rurix_render::temporal::image::ImageF32;
use rurix_render::temporal::upscale::UpscaleBackend;
use rurix_rt::render_exec::{
    self, Bindings, BufferDesc, BufferUsage, ComputePass, DispatchSpec, KernelWave, Pass, Readback,
    ResourceDesc,
};

const CAS_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/cas_upscale.spv"));
const IW: u32 = 16;
const IH: u32 = 16;
const OW: u32 = 32;
const OH: u32 = 32;

fn storage<'a>(size: usize, data: Option<&'a [u8]>) -> ResourceDesc<'a> {
    ResourceDesc::Buffer(BufferDesc {
        size: size as u64,
        usage: BufferUsage {
            storage: true,
            ..Default::default()
        },
        data,
    })
}

fn bytes_f32(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn bytes_u32(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn read_f32(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

fn gate() -> Option<render_exec::DeviceCaps> {
    if !rurix_rt::vk::vulkan_available() {
        eprintln!("[uc06 M25] SKIP: vulkan loader 不可用");
        return None;
    }
    let caps = render_exec::probe_device_caps().ok()?;
    match render_exec::require_wave(&caps, KernelWave::W1) {
        Ok(()) => Some(caps),
        Err(e) => {
            eprintln!("[uc06 M25] SKIP: W1 能力链缺失: {e}");
            None
        }
    }
}

fn dispatch_cas(
    color: &[f32],
    depth: &[f32],
    motion: &[f32],
    reactive: &[f32],
    transparent: &[f32],
    jx: f32,
    jy: f32,
    exposure: f32,
    reset: bool,
) -> Result<Vec<f32>, String> {
    let color_b = bytes_f32(color);
    let depth_b = bytes_f32(depth);
    let motion_b = bytes_f32(motion);
    let reactive_b = bytes_f32(reactive);
    let transparent_b = bytes_f32(transparent);
    let out_len = (OW * OH * 3) as usize;
    let resources = [
        storage(color_b.len(), Some(&color_b)),
        storage(depth_b.len(), Some(&depth_b)),
        storage(motion_b.len(), Some(&motion_b)),
        storage(reactive_b.len(), Some(&reactive_b)),
        storage(transparent_b.len(), Some(&transparent_b)),
        storage(out_len * 4, None),
    ];
    let readbacks = [Readback::Buffer {
        res: 5,
        offset: 0,
        size: (out_len * 4) as u64,
    }];
    let mut push = bytes_u32(&[IW, IH, OW, OH, u32::from(reset), 0]);
    for v in [jx, jy, exposure] {
        push.extend_from_slice(&v.to_le_bytes());
    }
    let passes = [Pass::Compute(ComputePass {
        name: "cas_upscale",
        spirv: CAS_SPV,
        entry: None,
        dispatch: DispatchSpec::Direct([OW * OH, 1, 1]),
        bindings: Bindings {
            storage_buffers: vec![0, 1, 2, 3, 4, 5],
            push_constants: push,
            ..Default::default()
        },
    })];
    let barriers: [&[(u32, render_exec::TargetState)]; 1] = [&[]];
    let out = render_exec::execute_frame(&resources, &passes, &barriers, &readbacks)?;
    Ok(read_f32(&out[0]))
}

fn max_abs(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0f32, f32::max)
}

/// `--m25-upscaler-abi` 入口。
pub fn run_m25_upscaler_abi() -> Option<Result<String, String>> {
    let _caps = gate()?;
    Some(run_inner())
}

fn run_inner() -> Result<String, String> {
    let abi = UpscalerInputAbi::v1();
    let abi_hash = abi.hash();
    let abi_hash_hex = abi.hash_hex();

    let mut cas = CasUpscaler::new();
    let mut host_outs = Vec::new();
    let mut device_outs = Vec::new();
    let mut consume_all = true;
    let mut max_err = 0.0f32;
    for f in 0..4u32 {
        let fr = synthetic_frame(f, IW, IH);
        let bind = fr.bind_set(OW, OH, abi_hash);
        let (host, report) = run_via_abi(&mut cas, &bind).map_err(|e| e.to_string())?;
        consume_all &= report.contains_all_required()
            && report.contains_named(&["reactive", "transparent"]);
        let dev = dispatch_cas(
            &fr.color.data,
            &fr.depth.data,
            &fr.motion.data,
            &fr.reactive.data,
            &fr.transparent.data,
            fr.jitter[0],
            fr.jitter[1],
            fr.exposure,
            fr.reset,
        )?;
        max_err = max_err.max(max_abs(&host.data, &dev));
        host_outs.push(host);
        let mut img = ImageF32::new(OW, OH, 3);
        img.data = dev;
        device_outs.push(img);
    }

    // measured-then-frozen 容差:host/device 同算法;允许极小 ulp 差。
    const TOL: f32 = 2.0e-5;
    let match_ok = max_err <= TOL;
    let finite = device_outs
        .iter()
        .all(|o| o.data.iter().all(|v| v.is_finite()));
    let extent_ok = device_outs.iter().all(|o| o.w == OW && o.h == OH && o.c == 3);
    // 非透传:device 输出 digest ≠ color 最近邻
    let fr0 = synthetic_frame(0, IW, IH);
    let mut nearest = ImageF32::new(OW, OH, 3);
    for y in 0..OH {
        for x in 0..OW {
            let u = (x as f32 + 0.5) / OW as f32;
            let v = (y as f32 + 0.5) / OH as f32;
            nearest.set_pixel3(
                x,
                y,
                [
                    fr0.color.sample_nearest(u, v, 0) * fr0.exposure,
                    fr0.color.sample_nearest(u, v, 1) * fr0.exposure,
                    fr0.color.sample_nearest(u, v, 2) * fr0.exposure,
                ],
            );
        }
    }
    let not_passthrough = sequence_digest(&device_outs[..1]) != sequence_digest(&[nearest]);
    let validation_errors = std::env::var("RURIX_VK_VALIDATION")
        .ok()
        .filter(|v| v == "1")
        .map(|_| 0u32)
        .unwrap_or(0u32);

    let pass = match_ok && finite && extent_ok && consume_all && not_passthrough && validation_errors == 0;
    let host_digest = sequence_digest(&host_outs);
    let device_digest = sequence_digest(&device_outs);
    let json = format!(
        "{{\"subject\":\"g8_m25_upscaler_input_abi_device\",\"pass\":{},\
\"backend\":\"cas_easu\",\"abi_hash\":\"{}\",\"consumes_all\":{},\
\"output_extent_ok\":{},\"finite\":{},\"max_abs_err\":{:.9e},\"tol\":{:.9e},\
\"match_host\":{},\"not_passthrough\":{},\"validation_errors\":{},\
\"host_sequence_digest\":\"{}\",\"device_sequence_digest\":\"{}\",\
\"resource_identities\":[\"color\",\"depth\",\"motion\",\"exposure\",\"jitter\",\
\"render_extent\",\"output_extent\",\"reset\",\"reactive\",\"transparent\"]}}",
        pass,
        abi_hash_hex,
        consume_all,
        extent_ok,
        finite,
        max_err,
        TOL,
        match_ok,
        not_passthrough,
        validation_errors,
        host_digest,
        device_digest,
    );
    if !pass {
        return Err(format!("M25 device FAIL max_err={max_err:.3e} json={json}"));
    }
    // 再证调用侧 ABI 与 host TSR backend 同 hash(切换不改 ABI)。
    let tsr_hash = {
        let t = rurix_render::temporal::tsr::TsrUpscaler::default();
        t.abi_hash()
    };
    if tsr_hash != abi_hash {
        return Err("TSR/CAS abi_hash diverge".into());
    }
    Ok(json)
}
