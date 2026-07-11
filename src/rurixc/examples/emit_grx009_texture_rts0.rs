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
            eprintln!("usage: emit_grx009_texture_rts0 <descriptor_layout.json> <out.bin>");
            return std::process::ExitCode::from(2);
        }
    };
    let out = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("usage: emit_grx009_texture_rts0 <descriptor_layout.json> <out.bin>");
            return std::process::ExitCode::from(2);
        }
    };
    if args.next().is_some() {
        eprintln!("usage: emit_grx009_texture_rts0 <descriptor_layout.json> <out.bin>");
        return std::process::ExitCode::from(2);
    }

    let descriptor_text = match std::fs::read_to_string(&descriptor) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("read descriptor failed: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    for needle in [
        "\"name\": \"src_luminance\"",
        "\"binding_kind\": \"texture2d\"",
        "\"name\": \"dst_luminance\"",
        "\"binding_kind\": \"rwtexture2d\"",
    ] {
        if !descriptor_text.contains(needle) {
            eprintln!("unsupported GRX-009 texture descriptor: missing {needle}");
            return std::process::ExitCode::FAILURE;
        }
    }
    // Optional GRX-009 stage A2 extensions, driven by descriptor content so
    // the original dxc_texture_bridge descriptor (root_constants: "none",
    // no prev_luminance) keeps producing byte-identical RTS0 output.
    let has_root_constants_28 = descriptor_text.contains("\"root_constants\": \"28_bytes\"");
    let has_prev_luminance = descriptor_text.contains("\"name\": \"prev_luminance\"");

    let mut resources = vec![ResourceBinding {
        name: "src_luminance".to_owned(),
        res: MirResourceType::Texture2D(PrimTy::F32),
        count: ResourceCount::One,
    }];
    if has_prev_luminance {
        resources.push(ResourceBinding {
            name: "prev_luminance".to_owned(),
            res: MirResourceType::Texture2D(PrimTy::F32),
            count: ResourceCount::One,
        });
    }
    resources.push(ResourceBinding {
        name: "dst_luminance".to_owned(),
        res: MirResourceType::RWTexture2D(PrimTy::F32),
        count: ResourceCount::One,
    });
    let mut rs = match infer_root_signature(&resources) {
        Ok(rs) => rs,
        Err(e) => {
            eprintln!("infer_root_signature failed: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    if has_root_constants_28 {
        // Canonical GRX-009 luminance root-constant layout (7 dwords =
        // 28 bytes at root_parameter_index 0), matching the canonical
        // luminance_reduction_descriptor_layout.json root_constant_layout.
        let constants = pack_root_constants(vec![
            ("source_width".to_owned(), RootConstantType::I64),
            ("source_height".to_owned(), RootConstantType::I64),
            ("max_luminance".to_owned(), RootConstantType::F32),
            ("min_luminance".to_owned(), RootConstantType::F32),
            ("exposure_adjust".to_owned(), RootConstantType::F32),
        ]);
        rs.parameters
            .insert(0, RootParameter::RootConstants { constants });
    }
    let rts0 = serialize_rts0(&rs);
    if let Err(e) = std::fs::write(&out, &rts0) {
        eprintln!("write RTS0 failed: {e}");
        return std::process::ExitCode::FAILURE;
    }
    println!(
        "emit_grx009_texture_rts0: wrote {} bytes to {out}",
        rts0.len()
    );
    std::process::ExitCode::SUCCESS
}

#[cfg(not(feature = "dxil-backend"))]
fn main() {
    eprintln!("emit_grx009_texture_rts0 requires --features dxil-backend");
}
