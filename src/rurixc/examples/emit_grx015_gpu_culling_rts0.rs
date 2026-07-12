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
            eprintln!("usage: emit_grx015_gpu_culling_rts0 <descriptor_layout.json> <out.bin>");
            return std::process::ExitCode::from(2);
        }
    };
    let out = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("usage: emit_grx015_gpu_culling_rts0 <descriptor_layout.json> <out.bin>");
            return std::process::ExitCode::from(2);
        }
    };
    if args.next().is_some() {
        eprintln!("usage: emit_grx015_gpu_culling_rts0 <descriptor_layout.json> <out.bin>");
        return std::process::ExitCode::from(2);
    }

    let descriptor_text = match std::fs::read_to_string(&descriptor) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("read descriptor failed: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    // The GRX-015 gpu_culling descriptor contract: src_transforms SRV t0
    // (structured_buffer, f32 transform lanes), dst_commands UAV u0
    // (rwstructured_buffer, 5-dword-stride indirect command blocks — only the
    // instance-count dword is atomically accumulated), dst_visibility UAV u1
    // (rwstructured_buffer, u32 bitmask, the GRX-016/018 interface), and the
    // 144-byte / 36-dword RURIX-DEFINED root-constant block (gpu_culling is
    // an additive pass: no Godot push constant exists to mirror).
    for needle in [
        "\"name\": \"src_transforms\"",
        "\"name\": \"dst_commands\"",
        "\"name\": \"dst_visibility\"",
        "\"binding_kind\": \"structured_buffer\"",
        "\"binding_kind\": \"rwstructured_buffer\"",
        "\"name\": \"frustum_plane_0_nx\"",
        "\"name\": \"frustum_plane_5_d\"",
        "\"name\": \"instance_count\"",
        "\"name\": \"transform_stride_floats\"",
        "\"name\": \"surface_count\"",
        "\"name\": \"command_stride_dwords\"",
        "\"name\": \"instance_count_dword_index\"",
        "\"name\": \"mesh_bound_radius_local\"",
    ] {
        if !descriptor_text.contains(needle) {
            eprintln!("unsupported GRX-015 gpu_culling descriptor: missing {needle}");
            return std::process::ExitCode::FAILURE;
        }
    }

    // One SRV (t0) then two UAVs (u0, u1). infer_root_signature aggregates
    // the SRV range (1 descriptor) ahead of the UAV range (2 descriptors) in
    // a single descriptor table; the descriptor layout JSON documents the
    // real per-slot HLSL types (f32 transform lanes / u32 command and bitmask
    // words).
    let resources = vec![
        ResourceBinding {
            name: "src_transforms".to_owned(),
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
    // Canonical GRX-015 gpu_culling root-constant layout: the Rurix-defined
    // 144-byte block = 36 dwords at root_parameter_index 0 (six normalized
    // inward-facing frustum planes, the instance/stride/surface/command
    // parameters, and the local bounding sphere). u32 fields are carried as
    // 1-dword F32 slots (RTS0 root constants only encode dword count/layout,
    // not semantic type; the descriptor JSON records the true u32/f32 type
    // per field). Field order and dword offsets match resource_mapping.md
    // exactly.
    let f = RootConstantType::F32;
    let mut fields: Vec<(String, RootConstantType)> = Vec::with_capacity(36);
    for plane in 0..6u32 {
        for comp in ["nx", "ny", "nz", "d"] {
            // dwords 0-23
            fields.push((format!("frustum_plane_{plane}_{comp}"), f));
        }
    }
    fields.push(("instance_count".to_owned(), f)); // dword 24 (u32)
    fields.push(("motion_vectors_current_offset".to_owned(), f)); // dword 25 (u32)
    fields.push(("transform_stride_floats".to_owned(), f)); // dword 26 (u32)
    fields.push(("surface_count".to_owned(), f)); // dword 27 (u32)
    fields.push(("command_stride_dwords".to_owned(), f)); // dword 28 (u32)
    fields.push(("instance_count_dword_index".to_owned(), f)); // dword 29 (u32)
    fields.push(("mesh_bound_center_local_x".to_owned(), f)); // dword 30
    fields.push(("mesh_bound_center_local_y".to_owned(), f)); // dword 31
    fields.push(("mesh_bound_center_local_z".to_owned(), f)); // dword 32
    fields.push(("mesh_bound_radius_local".to_owned(), f)); // dword 33
    fields.push(("pad1".to_owned(), f)); // dword 34 (u32)
    fields.push(("pad2".to_owned(), f)); // dword 35 (u32)
    let constants = pack_root_constants(fields);
    rs.parameters
        .insert(0, RootParameter::RootConstants { constants });
    let rts0 = serialize_rts0(&rs);
    if let Err(e) = std::fs::write(&out, &rts0) {
        eprintln!("write RTS0 failed: {e}");
        return std::process::ExitCode::FAILURE;
    }
    println!(
        "emit_grx015_gpu_culling_rts0: wrote {} bytes to {out}",
        rts0.len()
    );
    std::process::ExitCode::SUCCESS
}

#[cfg(not(feature = "dxil-backend"))]
fn main() {
    eprintln!("emit_grx015_gpu_culling_rts0 requires --features dxil-backend");
}
