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
            eprintln!("usage: emit_grx012_taa_resolve_rts0 <descriptor_layout.json> <out.bin>");
            return std::process::ExitCode::from(2);
        }
    };
    let out = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("usage: emit_grx012_taa_resolve_rts0 <descriptor_layout.json> <out.bin>");
            return std::process::ExitCode::from(2);
        }
    };
    if args.next().is_some() {
        eprintln!("usage: emit_grx012_taa_resolve_rts0 <descriptor_layout.json> <out.bin>");
        return std::process::ExitCode::from(2);
    }

    let descriptor_text = match std::fs::read_to_string(&descriptor) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("read descriptor failed: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    // The GRX-012 taa_resolve descriptor contract: color/depth/velocity/
    // last_velocity/history SRVs t0..t4 (texture2d), output_buffer UAV u0
    // (rwtexture2d), and the canonical 28-byte / 7-dword root-constant block
    // ([i64, i64, f32, f32, f32] packing shape shared with the
    // GRX-009/GRX-010/GRX-011 canonical layouts).
    for needle in [
        "\"name\": \"color_buffer\"",
        "\"name\": \"depth_buffer\"",
        "\"name\": \"velocity_buffer\"",
        "\"name\": \"last_velocity_buffer\"",
        "\"name\": \"history_buffer\"",
        "\"name\": \"output_buffer\"",
        "\"binding_kind\": \"texture2d\"",
        "\"binding_kind\": \"rwtexture2d\"",
        "\"name\": \"disocclusion_threshold\"",
        "\"name\": \"variance_dynamic\"",
        "\"name\": \"reserved0\"",
    ] {
        if !descriptor_text.contains(needle) {
            eprintln!("unsupported GRX-012 taa_resolve descriptor: missing {needle}");
            return std::process::ExitCode::FAILURE;
        }
    }

    // Five SRVs (t0..t4) then one UAV (u0). infer_root_signature aggregates the
    // SRV range (5 descriptors) ahead of the UAV range (1 descriptor) in a
    // single descriptor table. The element PrimTy is irrelevant to the root
    // signature shape (it only affects the descriptor table register ranges);
    // the descriptor layout JSON documents the real per-slot HLSL types.
    let resources = vec![
        ResourceBinding {
            name: "color_buffer".to_owned(),
            res: MirResourceType::Texture2D(PrimTy::F32),
            count: ResourceCount::One,
        },
        ResourceBinding {
            name: "depth_buffer".to_owned(),
            res: MirResourceType::Texture2D(PrimTy::F32),
            count: ResourceCount::One,
        },
        ResourceBinding {
            name: "velocity_buffer".to_owned(),
            res: MirResourceType::Texture2D(PrimTy::F32),
            count: ResourceCount::One,
        },
        ResourceBinding {
            name: "last_velocity_buffer".to_owned(),
            res: MirResourceType::Texture2D(PrimTy::F32),
            count: ResourceCount::One,
        },
        ResourceBinding {
            name: "history_buffer".to_owned(),
            res: MirResourceType::Texture2D(PrimTy::F32),
            count: ResourceCount::One,
        },
        ResourceBinding {
            name: "output_buffer".to_owned(),
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
    // Canonical GRX-012 taa_resolve root-constant layout (7 dwords = 28 bytes at
    // root_parameter_index 0), matching artifacts/taa_resolve_descriptor_layout.json.
    let constants = pack_root_constants(vec![
        ("source_width".to_owned(), RootConstantType::I64),
        ("source_height".to_owned(), RootConstantType::I64),
        ("disocclusion_threshold".to_owned(), RootConstantType::F32),
        ("variance_dynamic".to_owned(), RootConstantType::F32),
        ("reserved0".to_owned(), RootConstantType::F32),
    ]);
    rs.parameters
        .insert(0, RootParameter::RootConstants { constants });
    let rts0 = serialize_rts0(&rs);
    if let Err(e) = std::fs::write(&out, &rts0) {
        eprintln!("write RTS0 failed: {e}");
        return std::process::ExitCode::FAILURE;
    }
    println!(
        "emit_grx012_taa_resolve_rts0: wrote {} bytes to {out}",
        rts0.len()
    );
    std::process::ExitCode::SUCCESS
}

#[cfg(not(feature = "dxil-backend"))]
fn main() {
    eprintln!("emit_grx012_taa_resolve_rts0 requires --features dxil-backend");
}
