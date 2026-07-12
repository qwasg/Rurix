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
            eprintln!(
                "usage: emit_grx019_fused_post_chain_rts0 <descriptor_layout.json> <out.bin>"
            );
            return std::process::ExitCode::from(2);
        }
    };
    let out = match args.next() {
        Some(p) => p,
        None => {
            eprintln!(
                "usage: emit_grx019_fused_post_chain_rts0 <descriptor_layout.json> <out.bin>"
            );
            return std::process::ExitCode::from(2);
        }
    };
    if args.next().is_some() {
        eprintln!("usage: emit_grx019_fused_post_chain_rts0 <descriptor_layout.json> <out.bin>");
        return std::process::ExitCode::from(2);
    }

    let descriptor_text = match std::fs::read_to_string(&descriptor) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("read descriptor failed: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    // The GRX-019 fused_post_chain descriptor contract: src_color/lum_source/
    // prev_luminance SRVs t0..t2 (texture2d), dst_color/dst_luminance UAVs
    // u0..u1 (rwtexture2d), and the merged 64-byte / 16-dword root-constant
    // block ([i64 x4, f32 x8]: the two member canonical layouts merged plus
    // the first_frame / auto_exposure_scale fusion controls).
    for needle in [
        "\"name\": \"src_color\"",
        "\"name\": \"lum_source\"",
        "\"name\": \"prev_luminance\"",
        "\"name\": \"dst_color\"",
        "\"name\": \"dst_luminance\"",
        "\"binding_kind\": \"texture2d\"",
        "\"binding_kind\": \"rwtexture2d\"",
        "\"name\": \"lum_source_width\"",
        "\"name\": \"lum_source_height\"",
        "\"name\": \"max_luminance\"",
        "\"name\": \"min_luminance\"",
        "\"name\": \"exposure_adjust\"",
        "\"name\": \"exposure\"",
        "\"name\": \"white\"",
        "\"name\": \"luminance_multiplier\"",
        "\"name\": \"first_frame\"",
        "\"name\": \"auto_exposure_scale\"",
    ] {
        if !descriptor_text.contains(needle) {
            eprintln!("unsupported GRX-019 fused_post_chain descriptor: missing {needle}");
            return std::process::ExitCode::FAILURE;
        }
    }

    // Three SRVs (t0..t2) then two UAVs (u0..u1). infer_root_signature
    // aggregates the SRV range (3 descriptors) ahead of the UAV range (2
    // descriptors) in a single descriptor table. The element PrimTy is
    // irrelevant to the root signature shape (it only affects the descriptor
    // table register ranges); the descriptor layout JSON documents the real
    // per-slot HLSL types.
    let resources = vec![
        ResourceBinding {
            name: "src_color".to_owned(),
            res: MirResourceType::Texture2D(PrimTy::F32),
            count: ResourceCount::One,
        },
        ResourceBinding {
            name: "lum_source".to_owned(),
            res: MirResourceType::Texture2D(PrimTy::F32),
            count: ResourceCount::One,
        },
        ResourceBinding {
            name: "prev_luminance".to_owned(),
            res: MirResourceType::Texture2D(PrimTy::F32),
            count: ResourceCount::One,
        },
        ResourceBinding {
            name: "dst_color".to_owned(),
            res: MirResourceType::RWTexture2D(PrimTy::F32),
            count: ResourceCount::One,
        },
        ResourceBinding {
            name: "dst_luminance".to_owned(),
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
    // GRX-019 fused_post_chain root-constant layout (16 dwords = 64 bytes at
    // root_parameter_index 0), matching
    // artifacts/fused_post_chain_descriptor_layout.json: the two member
    // canonical layouts merged (dims first, f32 scalars after) plus the
    // first_frame / auto_exposure_scale fusion controls.
    let constants = pack_root_constants(vec![
        ("source_width".to_owned(), RootConstantType::I64),
        ("source_height".to_owned(), RootConstantType::I64),
        ("lum_source_width".to_owned(), RootConstantType::I64),
        ("lum_source_height".to_owned(), RootConstantType::I64),
        ("max_luminance".to_owned(), RootConstantType::F32),
        ("min_luminance".to_owned(), RootConstantType::F32),
        ("exposure_adjust".to_owned(), RootConstantType::F32),
        ("exposure".to_owned(), RootConstantType::F32),
        ("white".to_owned(), RootConstantType::F32),
        ("luminance_multiplier".to_owned(), RootConstantType::F32),
        ("first_frame".to_owned(), RootConstantType::F32),
        ("auto_exposure_scale".to_owned(), RootConstantType::F32),
    ]);
    rs.parameters
        .insert(0, RootParameter::RootConstants { constants });
    let rts0 = serialize_rts0(&rs);
    if let Err(e) = std::fs::write(&out, &rts0) {
        eprintln!("write RTS0 failed: {e}");
        return std::process::ExitCode::FAILURE;
    }
    println!(
        "emit_grx019_fused_post_chain_rts0: wrote {} bytes to {out}",
        rts0.len()
    );
    std::process::ExitCode::SUCCESS
}

#[cfg(not(feature = "dxil-backend"))]
fn main() {
    eprintln!("emit_grx019_fused_post_chain_rts0 requires --features dxil-backend");
}
