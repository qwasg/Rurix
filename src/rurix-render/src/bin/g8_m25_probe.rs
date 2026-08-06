//! G8.5b M25 host probe:ABI 十项/hash/fail-closed/双 backend 序列 digest。

use std::env;
use std::fs;
use std::path::PathBuf;

use rurix_render::temporal::abi::{
    assemble, run_via_abi, sequence_digest, synthetic_frame, AssembleError, NoOpPassthroughUpscaler,
    UpscalerInputAbi, ABI_SLOT_NAMES,
};
use rurix_render::temporal::cas::CasUpscaler;
use rurix_render::temporal::tsr::TsrUpscaler;
use rurix_render::temporal::upscale::UpscaleBackend;

const IW: u32 = 16;
const IH: u32 = 16;
const OW: u32 = 32;
const OH: u32 = 32;
const SEQ_FRAMES: u32 = 8;

fn main() {
    let mut golden_dir: Option<PathBuf> = None;
    let mut write_golden = false;
    let args: Vec<String> = env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--golden-dir" => {
                i += 1;
                golden_dir = Some(PathBuf::from(args.get(i).expect("--golden-dir path")));
            }
            "--write-golden" => write_golden = true,
            other => {
                eprintln!("unknown arg {other}");
                std::process::exit(2);
            }
        }
        i += 1;
    }

    let abi = UpscalerInputAbi::v1();
    let abi_hash = abi.hash();
    let abi_hash_hex = abi.hash_hex();
    let names: Vec<&str> = abi.slots.iter().map(|s| s.name).collect();
    let abi_ten = names == ABI_SLOT_NAMES;

    let b1 = abi.canonical_bytes();
    let b2 = UpscalerInputAbi::v1().canonical_bytes();
    let abi_layout_hash_stable = b1 == b2 && abi.hash() == UpscalerInputAbi::v1().hash();

    let h_flip = abi.with_required_flipped("reactive").hash();
    let h_ch = abi.with_channels_tampered("color", "4").hash();
    let abi_hash_sensitive = h_flip != abi_hash && h_ch != abi_hash;

    // 逐项摘除 required → Err
    let required_slots = [
        "color",
        "depth",
        "motion",
        "exposure",
        "jitter",
        "render_extent",
        "output_extent",
        "reset",
    ];
    let frame0 = synthetic_frame(0, IW, IH);
    let mut missing_ok = 0u32;
    for slot in required_slots {
        let mut bind = frame0.bind_set(OW, OH, abi_hash);
        match slot {
            "color" => bind.color = None,
            "depth" => bind.depth = None,
            "motion" => bind.motion = None,
            "exposure" => bind.exposure = None,
            "jitter" => bind.jitter = None,
            "render_extent" => bind.render_extent = None,
            "output_extent" => bind.output_extent = None,
            "reset" => bind.reset = None,
            _ => unreachable!(),
        }
        match assemble(&bind, abi_hash) {
            Err(AssembleError::MissingRequired(s)) if s == slot => missing_ok += 1,
            other => {
                eprintln!("missing {slot}: unexpected {other:?}");
            }
        }
    }
    let missing_input_fail_closed = missing_ok == required_slots.len() as u32;

    let mut bad_hash = abi_hash;
    bad_hash[0] ^= 0xff;
    let bind_bad = frame0.bind_set(OW, OH, bad_hash);
    let hash_mismatch_fail_closed =
        matches!(assemble(&bind_bad, abi_hash), Err(AssembleError::HashMismatch { .. }));

    // 双 backend 序列
    let mut tsr = TsrUpscaler::default();
    let mut cas = CasUpscaler::default();
    let mut tsr_outs = Vec::new();
    let mut cas_outs = Vec::new();
    let mut tsr_consume_all = true;
    let mut cas_consume_all = true;
    let mut output_extent_and_finite = true;
    for f in 0..SEQ_FRAMES {
        let fr = synthetic_frame(f, IW, IH);
        let bind = fr.bind_set(OW, OH, abi_hash);
        let (o_tsr, r_tsr) = run_via_abi(&mut tsr, &bind).expect("tsr abi");
        let (o_cas, r_cas) = run_via_abi(&mut cas, &bind).expect("cas abi");
        tsr_consume_all &= r_tsr.contains_all_required();
        cas_consume_all &= r_cas.contains_all_required();
        // optional 槽也在本 fixture 绑定
        tsr_consume_all &= r_tsr.contains_named(&["reactive", "transparent"]);
        cas_consume_all &= r_cas.contains_named(&["reactive", "transparent"]);
        output_extent_and_finite &= o_tsr.w == OW
            && o_tsr.h == OH
            && o_cas.w == OW
            && o_cas.h == OH
            && o_tsr.data.iter().all(|v| v.is_finite())
            && o_cas.data.iter().all(|v| v.is_finite());
        tsr_outs.push(o_tsr);
        cas_outs.push(o_cas);
    }
    let tsr_digest = sequence_digest(&tsr_outs);
    let cas_digest = sequence_digest(&cas_outs);

    // backend 切换调用侧 ABI 同一
    let backend_switch_abi_identical = tsr.abi_hash() == cas.abi_hash() && tsr.abi_hash() == abi_hash;

    // 反假绿:noop 不得吃满 required
    let mut noop = NoOpPassthroughUpscaler::default();
    let bind = frame0.bind_set(OW, OH, abi_hash);
    let (_onoop, rnoop) = run_via_abi(&mut noop, &bind).expect("noop");
    let not_stub = !rnoop.contains_all_required() && tsr_digest != sequence_digest(&[_onoop]);

    let mut sequence_digest_match = false;
    if let Some(dir) = &golden_dir {
        let tsr_g = dir.join("tsr_sequence.sha256");
        let cas_g = dir.join("cas_sequence.sha256");
        if write_golden {
            fs::create_dir_all(dir).expect("mkdir");
            fs::write(&tsr_g, format!("{tsr_digest}\n")).expect("write tsr");
            fs::write(&cas_g, format!("{cas_digest}\n")).expect("write cas");
            fs::write(dir.join("abi_hash.sha256"), format!("{abi_hash_hex}\n")).expect("abi");
            eprintln!("[g8_m25_probe] wrote golden → {}", dir.display());
        }
        let tsr_ok = fs::read_to_string(&tsr_g)
            .map(|s| s.trim() == tsr_digest)
            .unwrap_or(false);
        let cas_ok = fs::read_to_string(&cas_g)
            .map(|s| s.trim() == cas_digest)
            .unwrap_or(false);
        sequence_digest_match = tsr_ok && cas_ok;
    }

    let pass = abi_ten
        && abi_layout_hash_stable
        && abi_hash_sensitive
        && missing_input_fail_closed
        && hash_mismatch_fail_closed
        && tsr_consume_all
        && cas_consume_all
        && output_extent_and_finite
        && backend_switch_abi_identical
        && not_stub
        && (golden_dir.is_none() || sequence_digest_match);

    println!(
        "{{\"subject\":\"g8_m25_upscaler_input_abi\",\"pass\":{},\
\"abi_ten_inputs_enumerated\":{},\"abi_layout_hash_stable\":{},\"abi_hash_sensitive\":{},\
\"missing_input_fail_closed\":{},\"hash_mismatch_fail_closed\":{},\
\"backend_tsr_consumes_all\":{},\"backend_cas_consumes_all\":{},\
\"output_extent_and_finite\":{},\"sequence_digest_match\":{},\
\"backend_switch_abi_identical\":{},\"not_stub\":{},\
\"abi_hash\":\"{}\",\"tsr_sequence_digest\":\"{}\",\"cas_sequence_digest\":\"{}\",\
\"iw\":{},\"ih\":{},\"ow\":{},\"oh\":{},\"seq_frames\":{}}}",
        pass,
        abi_ten,
        abi_layout_hash_stable,
        abi_hash_sensitive,
        missing_input_fail_closed,
        hash_mismatch_fail_closed,
        tsr_consume_all,
        cas_consume_all,
        output_extent_and_finite,
        sequence_digest_match,
        backend_switch_abi_identical,
        not_stub,
        abi_hash_hex,
        tsr_digest,
        cas_digest,
        IW,
        IH,
        OW,
        OH,
        SEQ_FRAMES
    );
}
