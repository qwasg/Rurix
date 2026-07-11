#[cfg(feature = "dxil-backend")]
fn main() -> std::process::ExitCode {
    use rurixc::binding_layout::{
        RootConstantType, RootParameter, infer_root_signature, pack_root_constants, serialize_rts0,
    };
    use rurixc::hir::PrimTy;
    use rurixc::mir::{MirResourceType, ResourceBinding, ResourceCount};

    let mut args = std::env::args().skip(1);
    let descriptor = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("usage: emit_grx011_ssao_blur_rts0 <descriptor_layout.json> <out.bin>");
            return std::process::ExitCode::from(2);
        }
    };
    let out = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("usage: emit_grx011_ssao_blur_rts0 <descriptor_layout.json> <out.bin>");
            return std::process::ExitCode::from(2);
        }
    };
    if args.next().is_some() {
        eprintln!("usage: emit_grx011_ssao_blur_rts0 <descriptor_layout.json> <out.bin>");
        return std::process::ExitCode::from(2);
    }

    let descriptor_text = match std::fs::read_to_string(&descriptor) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("read descriptor failed: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    // The GRX-011 ssao_blur descriptor contract: src_ssao SRV t0 (texture2d),
    // dst_ssao UAV u0 (rwtexture2d), and the canonical 28-byte / 7-dword
    // root-constant block ([i64, i64, f32, f32, f32] packing shape shared
    // with the GRX-009/GRX-010 canonical layouts).
    for needle in [
        "\"name\": \"src_ssao\"",
        "\"name\": \"dst_ssao\"",
        "\"binding_kind\": \"texture2d\"",
        "\"binding_kind\": \"rwtexture2d\"",
        "\"name\": \"edge_sharpness\"",
        "\"name\": \"half_screen_pixel_size_x\"",
        "\"name\": \"half_screen_pixel_size_y\"",
    ] {
        if !descriptor_text.contains(needle) {
            eprintln!("unsupported GRX-011 ssao_blur descriptor: missing {needle}");
            return std::process::ExitCode::FAILURE;
        }
    }

    let resources = vec![
        ResourceBinding {
            name: "src_ssao".to_owned(),
            res: MirResourceType::Texture2D(PrimTy::F32),
            count: ResourceCount::One,
        },
        ResourceBinding {
            name: "dst_ssao".to_owned(),
            res: MirResourceType::RWTexture2D(PrimTy::F32),
            count: ResourceCount::One,
        },
    ];
    let mut rs = match infer_root_signature(&resources) {
        Ok(rs) => rs,
        Err(e) => {
            eprintln!("infer_root_signature failed: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    // Canonical GRX-011 ssao_blur root-constant layout (7 dwords = 28 bytes at
    // root_parameter_index 0), matching artifacts/ssao_blur_descriptor_layout.json.
    let constants = pack_root_constants(vec![
        ("source_width".to_owned(), RootConstantType::I64),
        ("source_height".to_owned(), RootConstantType::I64),
        ("edge_sharpness".to_owned(), RootConstantType::F32),
        ("half_screen_pixel_size_x".to_owned(), RootConstantType::F32),
        ("half_screen_pixel_size_y".to_owned(), RootConstantType::F32),
    ]);
    rs.parameters
        .insert(0, RootParameter::RootConstants { constants });
    let rts0 = serialize_rts0(&rs);
    if let Err(e) = std::fs::write(&out, &rts0) {
        eprintln!("write RTS0 failed: {e}");
        return std::process::ExitCode::FAILURE;
    }
    println!(
        "emit_grx011_ssao_blur_rts0: wrote {} bytes to {out}",
        rts0.len()
    );
    std::process::ExitCode::SUCCESS
}

#[cfg(not(feature = "dxil-backend"))]
fn main() {
    eprintln!("emit_grx011_ssao_blur_rts0 requires --features dxil-backend");
}
