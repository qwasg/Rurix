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
            eprintln!("usage: emit_grx015_gpu_culling_rd_native_rts0 <descriptor_layout.json> <out.bin>");
            return std::process::ExitCode::from(2);
        }
    };
    let out = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("usage: emit_grx015_gpu_culling_rd_native_rts0 <descriptor_layout.json> <out.bin>");
            return std::process::ExitCode::from(2);
        }
    };
    if args.next().is_some() {
        eprintln!("usage: emit_grx015_gpu_culling_rd_native_rts0 <descriptor_layout.json> <out.bin>");
        return std::process::ExitCode::from(2);
    }

    let descriptor_text = match std::fs::read_to_string(&descriptor) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("read descriptor failed: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    // The GRX-015 gpu_culling RD-NATIVE descriptor contract (patch 0046): unlike
    // the shim variant (144-byte b0 with the 6 frustum planes as root constants),
    // the RD-native variant moves the frustum planes into a StructuredBuffer<float4>
    // SRV at register t1 and shrinks b0 to a 48-byte / 12-dword block so it fits the
    // RD/D3D12 128-byte root-constant window. Binding order: src_transforms SRV t0
    // (structured_buffer), frustum_planes SRV t1 (structured_buffer), dst_commands
    // UAV u0 (rwstructured_buffer), dst_visibility UAV u1 (rwstructured_buffer).
    for needle in [
        "\"name\": \"src_transforms\"",
        "\"name\": \"frustum_planes\"",
        "\"name\": \"dst_commands\"",
        "\"name\": \"dst_visibility\"",
        "\"binding_kind\": \"structured_buffer\"",
        "\"binding_kind\": \"rwstructured_buffer\"",
        "\"name\": \"instance_count\"",
        "\"name\": \"transform_stride_floats\"",
        "\"name\": \"surface_count\"",
        "\"name\": \"command_stride_dwords\"",
        "\"name\": \"instance_count_dword_index\"",
        "\"name\": \"mesh_bound_radius_local\"",
    ] {
        if !descriptor_text.contains(needle) {
            eprintln!("unsupported GRX-015 gpu_culling rd_native descriptor: missing {needle}");
            return std::process::ExitCode::FAILURE;
        }
    }
    // Reject the shim variant's root-constant plane fields — those belong to the
    // 144-byte b0 and must NOT appear in the 48-byte rd_native descriptor.
    if descriptor_text.contains("\"name\": \"frustum_plane_0_nx\"") {
        eprintln!("unsupported: descriptor carries frustum_plane_* root constants (shim 144-byte b0), not the rd_native 48-byte b0 with frustum_planes as a t1 StructuredBuffer");
        return std::process::ExitCode::FAILURE;
    }

    // Two SRVs (t0 transforms, t1 frustum_planes) then two UAVs (u0 commands, u1
    // visibility). infer_root_signature assigns registers per class in declaration
    // order (t0, t1 / u0, u1) and aggregates the SRV range (2 descriptors) ahead of
    // the UAV range (2 descriptors) in a single descriptor table — the taa_resolve
    // 5-SRV aggregation precedent.
    let resources = vec![
        ResourceBinding {
            name: "src_transforms".to_owned(),
            res: MirResourceType::StructuredBuffer { read_only: true },
            count: ResourceCount::One,
        },
        ResourceBinding {
            name: "frustum_planes".to_owned(),
            res: MirResourceType::StructuredBuffer { read_only: true },
            count: ResourceCount::One,
        },
        ResourceBinding {
            name: "dst_commands".to_owned(),
            res: MirResourceType::StructuredBuffer { read_only: false },
            count: ResourceCount::One,
        },
        ResourceBinding {
            name: "dst_visibility".to_owned(),
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
    // Canonical GRX-015 gpu_culling RD-native root-constant layout: the 48-byte
    // block = 12 dwords at root_parameter_index 0 (the instance/stride/surface/
    // command parameters + the local bounding sphere; the 6 frustum planes are the
    // t1 StructuredBuffer, NOT root constants). u32 fields are carried as 1-dword
    // F32 slots (RTS0 root constants only encode dword count/layout, not semantic
    // type; the descriptor JSON records the true u32/f32 type per field). Field
    // order and dword offsets match gpu_culling_rd_native_descriptor_layout.json.
    let f = RootConstantType::F32;
    let fields: Vec<(String, RootConstantType)> = vec![
        ("instance_count".to_owned(), f),                // dword 0 (u32)
        ("motion_vectors_current_offset".to_owned(), f), // dword 1 (u32)
        ("transform_stride_floats".to_owned(), f),       // dword 2 (u32)
        ("surface_count".to_owned(), f),                 // dword 3 (u32)
        ("command_stride_dwords".to_owned(), f),         // dword 4 (u32)
        ("instance_count_dword_index".to_owned(), f),    // dword 5 (u32)
        ("mesh_bound_center_local_x".to_owned(), f),     // dword 6
        ("mesh_bound_center_local_y".to_owned(), f),     // dword 7
        ("mesh_bound_center_local_z".to_owned(), f),     // dword 8
        ("mesh_bound_radius_local".to_owned(), f),       // dword 9
        ("pad1".to_owned(), f),                          // dword 10 (u32)
        ("pad2".to_owned(), f),                          // dword 11 (u32)
    ];
    let constants = pack_root_constants(fields);
    rs.parameters
        .insert(0, RootParameter::RootConstants { constants });
    let rts0 = serialize_rts0(&rs);
    if let Err(e) = std::fs::write(&out, &rts0) {
        eprintln!("write RTS0 failed: {e}");
        return std::process::ExitCode::FAILURE;
    }
    println!(
        "emit_grx015_gpu_culling_rd_native_rts0: wrote {} bytes to {out}",
        rts0.len()
    );
    std::process::ExitCode::SUCCESS
}

#[cfg(not(feature = "dxil-backend"))]
fn main() {
    eprintln!("emit_grx015_gpu_culling_rd_native_rts0 requires --features dxil-backend");
}
