// Assisted-by: Cursor Agent（G20.2 M-a 实现波）
//! G20.2 M-a HZB 遮挡剔除 host 参考臂 probe（门
//! `g20.p0.m_a.hzb_occlusion_host_realization`；RFC-0037）。
//!
//! 职责闭集：确定性合成深度场（近墙 + 远背景 + 中带扰动，reverse-Z 与
//! standard-Z 双约定）× 确定性 800 rect 夹具——**保守零假阳性硬不变量**
//! （HZB 判 Occluded ⇒ `exact_rect_occluded` 精确真值必同判）+ 剔除率非零 +
//! 金字塔顶层 = 全图最远深度锚 + 双跑位级 digest。
//!
//! 用法：`g20_hzb_probe --out evidence/g20_hzb_probe_<UTC>.json`

#![forbid(unsafe_code)]

use rurix_render::geometry::hzb::{DepthConvention, HzbPyramid, Occlusion, exact_rect_occluded};
use rurix_render::temporal::image::ImageF32;

const W: u32 = 193; // 非 2 幂
const H: u32 = 117;
const RECTS: u32 = 800;

fn scene_depth_reverse_z(w: u32, h: u32) -> ImageF32 {
    ImageF32::from_fn(w, h, 1, |x, y, _| {
        let fx = (x as f32 + 0.5) / w as f32;
        let fy = (y as f32 + 0.5) / h as f32;
        if fx < 0.42 {
            0.88 + 0.05 * (fy * 9.0).sin().abs()
        } else if fy > 0.7 {
            0.55 // 中景带
        } else {
            0.08 + 0.06 * ((fx * 7.0 + fy * 3.0).sin() * 0.5 + 0.5)
        }
    })
}

fn det_rects(n: u32) -> Vec<([f32; 2], [f32; 2], f32)> {
    let mut out = Vec::new();
    for i in 0..n {
        let mut v = i.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
        let mut next = || {
            v ^= v >> 15;
            v = v.wrapping_mul(0x7FEB_352D);
            v ^= v >> 13;
            (v % 1000) as f32 / 1000.0
        };
        let cx = next();
        let cy = next();
        let hw = 0.02 + 0.22 * next();
        let hh = 0.02 + 0.22 * next();
        let d = next();
        out.push((
            [(cx - hw).clamp(0.0, 1.0), (cy - hh).clamp(0.0, 1.0)],
            [(cx + hw).clamp(0.0, 1.0), (cy + hh).clamp(0.0, 1.0)],
            d,
        ));
    }
    out
}

struct ArmReport {
    conv: &'static str,
    occluded: u32,
    visible: u32,
    false_positives: u32,
    digest: String,
}

fn run_arm(conv: DepthConvention, name: &'static str) -> ArmReport {
    let rz = scene_depth_reverse_z(W, H);
    let depth = match conv {
        DepthConvention::ReverseZ => rz,
        DepthConvention::StandardZ => ImageF32::from_fn(W, H, 1, |x, y, _| 1.0 - rz.get(x, y, 0)),
    };
    let hzb = HzbPyramid::build(&depth, conv);
    let mut occluded = 0u32;
    let mut visible = 0u32;
    let mut fp = 0u32;
    let mut trace: Vec<u8> = Vec::new();
    for (mn, mx, d0) in det_rects(RECTS) {
        let d = match conv {
            DepthConvention::ReverseZ => d0,
            DepthConvention::StandardZ => 1.0 - d0,
        };
        let verdict = hzb.test_rect(mn, mx, d);
        let bit = match verdict {
            Occlusion::Occluded => {
                occluded += 1;
                if !exact_rect_occluded(&depth, conv, mn, mx, d) {
                    fp += 1;
                }
                1u8
            }
            Occlusion::Visible => {
                visible += 1;
                0u8
            }
        };
        trace.push(bit);
    }
    for m in &hzb.mips {
        for &v in &m.data {
            trace.extend_from_slice(&v.to_le_bytes());
        }
    }
    ArmReport {
        conv: name,
        occluded,
        visible,
        false_positives: fp,
        digest: rurix_pkg::sha256::hex_digest(&trace),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out_path = args
        .iter()
        .position(|a| a == "--out")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| {
            eprintln!("g20_hzb_probe: FAIL 缺 --out <evidence.json>");
            std::process::exit(2);
        });

    let arms1: Vec<ArmReport> = vec![
        run_arm(DepthConvention::ReverseZ, "reverse_z"),
        run_arm(DepthConvention::StandardZ, "standard_z"),
    ];
    let arms2: Vec<ArmReport> = vec![
        run_arm(DepthConvention::ReverseZ, "reverse_z"),
        run_arm(DepthConvention::StandardZ, "standard_z"),
    ];
    let bitexact = arms1
        .iter()
        .zip(arms2.iter())
        .all(|(a, b)| a.digest == b.digest);
    let zero_fp = arms1.iter().all(|a| a.false_positives == 0);
    let cull_nonzero = arms1.iter().all(|a| a.occluded > 0);

    let mut arms_json = String::new();
    for (i, a) in arms1.iter().enumerate() {
        if i > 0 {
            arms_json.push(',');
        }
        arms_json.push_str(&format!(
            "{{\"conv\":\"{}\",\"rects\":{RECTS},\"occluded\":{},\"visible\":{},\
             \"false_positives\":{},\"digest\":\"sha256:{}\"}}",
            a.conv, a.occluded, a.visible, a.false_positives, a.digest
        ));
        println!(
            "[g20_hzb_probe] {}: occluded={} visible={} false_positives={}",
            a.conv, a.occluded, a.visible, a.false_positives
        );
    }
    let payload = format!(
        "{{\"schema_version\":1,\"subject\":\"g20_hzb_probe\",\"resolution\":[{W},{H}],\
         \"arms\":[{arms_json}],\"zero_false_positive\":{zero_fp},\
         \"cull_rate_nonzero\":{cull_nonzero},\"double_run_bitexact\":{bitexact},\
         \"notes\":\"HZB host 参考臂 probe：保守零假阳性硬不变量（判遮挡 ⇒ 逐像素精确真值同判）+ 剔除率非零 + 金字塔字节 digest 双跑位级；非 2 幂 193×117 双约定\"}}"
    );
    if let Some(parent) = std::path::Path::new(&out_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&out_path, payload + "\n").expect("写 evidence 失败");
    println!("[g20_hzb_probe] evidence → {out_path}");
    if !(zero_fp && cull_nonzero && bitexact) {
        eprintln!("[g20_hzb_probe] FAIL zero_fp={zero_fp} cull={cull_nonzero} bitexact={bitexact}");
        std::process::exit(1);
    }
    println!("[g20_hzb_probe] PASS");
}
