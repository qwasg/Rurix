#[cfg(feature = "dxil-backend")]
fn main() -> std::process::ExitCode {
    use rurixc::binding_layout::{
        RootConstantType, RootParameter, infer_root_signature, pack_root_constants, serialize_rts0,
    };
    use rurixc::mir::{MirResourceType, ResourceBinding, ResourceCount};

    let mut args = std::env::args().skip(1);
    let descriptor = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("usage: emit_grx013_particles_copy_rts0 <descriptor_layout.json> <out.bin>");
            return std::process::ExitCode::from(2);
        }
    };
    let out = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("usage: emit_grx013_particles_copy_rts0 <descriptor_layout.json> <out.bin>");
            return std::process::ExitCode::from(2);
        }
    };
    if args.next().is_some() {
        eprintln!("usage: emit_grx013_particles_copy_rts0 <descriptor_layout.json> <out.bin>");
        return std::process::ExitCode::from(2);
    }

    let descriptor_text = match std::fs::read_to_string(&descriptor) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("read descriptor failed: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    // The GRX-013 particles_copy descriptor contract: src_particles SRV t0
    // (structured_buffer), dst_instances UAV u0 (rwstructured_buffer), and the
    // 128-byte / 32-dword CopyPushConstant root-constant block (Godot's
    // ParticlesShader::CopyPushConstant, mirrored field-by-field).
    for needle in [
        "\"name\": \"src_particles\"",
        "\"name\": \"dst_instances\"",
        "\"binding_kind\": \"structured_buffer\"",
        "\"binding_kind\": \"rwstructured_buffer\"",
        "\"name\": \"sort_direction_x\"",
        "\"name\": \"total_particles\"",
        "\"name\": \"align_mode\"",
        "\"name\": \"align_channel_filter\"",
        "\"name\": \"motion_vectors_current_offset\"",
    ] {
        if !descriptor_text.contains(needle) {
            eprintln!("unsupported GRX-013 particles_copy descriptor: missing {needle}");
            return std::process::ExitCode::FAILURE;
        }
    }

    let resources = vec![
        ResourceBinding {
            name: "src_particles".to_owned(),
            res: MirResourceType::StructuredBuffer { read_only: true },
            count: ResourceCount::One,
        },
        ResourceBinding {
            name: "dst_instances".to_owned(),
            res: MirResourceType::StructuredBuffer { read_only: false },
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
    // Canonical GRX-013 particles_copy root-constant layout: Godot's 128-byte
    // ParticlesShader::CopyPushConstant = 32 dwords at root_parameter_index 0.
    // uint32 fields are carried as 1-dword F32 slots (RTS0 root constants only
    // encode dword count/layout, not semantic type; the descriptor JSON records
    // the true u32/f32 type per field). Field order and dword offsets match the
    // struct byte layout exactly (see resource_mapping.md).
    let f = RootConstantType::F32;
    let constants = pack_root_constants(vec![
        ("sort_direction_x".to_owned(), f),              // dword 0
        ("sort_direction_y".to_owned(), f),              // dword 1
        ("sort_direction_z".to_owned(), f),              // dword 2
        ("total_particles".to_owned(), f),               // dword 3  (u32)
        ("trail_size".to_owned(), f),                    // dword 4  (u32)
        ("trail_total".to_owned(), f),                   // dword 5  (u32)
        ("frame_delta".to_owned(), f),                   // dword 6
        ("frame_remainder".to_owned(), f),               // dword 7
        ("align_up_x".to_owned(), f),                    // dword 8
        ("align_up_y".to_owned(), f),                    // dword 9
        ("align_up_z".to_owned(), f),                    // dword 10
        ("align_mode".to_owned(), f),                    // dword 11 (u32)
        ("lifetime_split".to_owned(), f),                // dword 12 (u32)
        ("lifetime_reverse".to_owned(), f),              // dword 13 (u32)
        ("motion_vectors_current_offset".to_owned(), f), // dword 14 (u32)
        ("flags_bits".to_owned(), f),                    // dword 15 (u32)
        ("inv_emission_transform_0".to_owned(), f),      // dword 16
        ("inv_emission_transform_1".to_owned(), f),      // dword 17
        ("inv_emission_transform_2".to_owned(), f),      // dword 18
        ("inv_emission_transform_3".to_owned(), f),      // dword 19
        ("inv_emission_transform_4".to_owned(), f),      // dword 20
        ("inv_emission_transform_5".to_owned(), f),      // dword 21
        ("inv_emission_transform_6".to_owned(), f),      // dword 22
        ("inv_emission_transform_7".to_owned(), f),      // dword 23
        ("inv_emission_transform_8".to_owned(), f),      // dword 24
        ("inv_emission_transform_9".to_owned(), f),      // dword 25
        ("inv_emission_transform_10".to_owned(), f),     // dword 26
        ("inv_emission_transform_11".to_owned(), f),     // dword 27
        ("align_channel_filter".to_owned(), f),          // dword 28 (u32)
        ("align_axis".to_owned(), f),                    // dword 29 (u32)
        ("pad1".to_owned(), f),                          // dword 30 (u32)
        ("pad2".to_owned(), f),                          // dword 31 (u32)
    ]);
    rs.parameters
        .insert(0, RootParameter::RootConstants { constants });
    let rts0 = serialize_rts0(&rs);
    if let Err(e) = std::fs::write(&out, &rts0) {
        eprintln!("write RTS0 failed: {e}");
        return std::process::ExitCode::FAILURE;
    }
    println!(
        "emit_grx013_particles_copy_rts0: wrote {} bytes to {out}",
        rts0.len()
    );
    std::process::ExitCode::SUCCESS
}

#[cfg(not(feature = "dxil-backend"))]
fn main() {
    eprintln!("emit_grx013_particles_copy_rts0 requires --features dxil-backend");
}
