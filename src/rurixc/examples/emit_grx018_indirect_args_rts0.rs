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
            eprintln!("usage: emit_grx018_indirect_args_rts0 <descriptor_layout.json> <out.bin>");
            return std::process::ExitCode::from(2);
        }
    };
    let out = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("usage: emit_grx018_indirect_args_rts0 <descriptor_layout.json> <out.bin>");
            return std::process::ExitCode::from(2);
        }
    };
    if args.next().is_some() {
        eprintln!("usage: emit_grx018_indirect_args_rts0 <descriptor_layout.json> <out.bin>");
        return std::process::ExitCode::from(2);
    }

    let descriptor_text = match std::fs::read_to_string(&descriptor) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("read descriptor failed: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    // The GRX-018 indirect_args descriptor contract: src_survivor_counts SRV
    // t0 (structured_buffer, uint words; the GRX-015/016 survivor-count
    // producer interface), dst_command_buffer UAV u0 (rwstructured_buffer,
    // 5-dword Godot INDIRECT_MULTIMESH_COMMAND_STRIDE blocks),
    // dst_validation UAV u1 (rwstructured_buffer, the resident validation
    // red-leg output), and the 176-byte / 44-dword Rurix-owned parameter
    // block (surface_count / max_instance_count / survivor_count_word_offset
    // / pad0 + 8 per-surface 5-dword command templates). ONE root signature
    // serves BOTH kernels (write + validate): they declare the identical
    // binding surface and the write kernel simply never references u1.
    for needle in [
        "\"name\": \"src_survivor_counts\"",
        "\"name\": \"dst_command_buffer\"",
        "\"name\": \"dst_validation\"",
        "\"binding_kind\": \"structured_buffer\"",
        "\"binding_kind\": \"rwstructured_buffer\"",
        "\"name\": \"surface_count\"",
        "\"name\": \"max_instance_count\"",
        "\"name\": \"survivor_count_word_offset\"",
        "\"name\": \"surface0_index_count\"",
        "\"name\": \"surface0_instance_count_reserved\"",
        "\"name\": \"surface7_first_instance\"",
    ] {
        if !descriptor_text.contains(needle) {
            eprintln!("unsupported GRX-018 indirect_args descriptor: missing {needle}");
            return std::process::ExitCode::FAILURE;
        }
    }

    // One SRV (t0) then two UAVs (u0, u1). infer_root_signature aggregates
    // the SRV range (1 descriptor) ahead of the UAV range (2 descriptors) in
    // a single descriptor table; the descriptor layout JSON documents the
    // real per-slot HLSL types (uint word buffers).
    let resources = vec![
        ResourceBinding {
            name: "src_survivor_counts".to_owned(),
            res: MirResourceType::StructuredBuffer { read_only: true },
            count: ResourceCount::One,
        },
        ResourceBinding {
            name: "dst_command_buffer".to_owned(),
            res: MirResourceType::StructuredBuffer { read_only: false },
            count: ResourceCount::One,
        },
        ResourceBinding {
            name: "dst_validation".to_owned(),
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
    // Canonical GRX-018 indirect_args root-constant layout: a Rurix-owned
    // 176-byte / 44-dword parameter block at root_parameter_index 0 (NO
    // native push constant exists — the native producer is CPU buffer_update
    // code in mesh_storage.cpp, not a dispatch). All 44 fields are u32,
    // carried as 1-dword F32 slots (RTS0 root constants only encode dword
    // count/layout, not semantic type; the descriptor JSON records the true
    // u32 type per field). Field order and dword offsets match the HLSL
    // cbuffer layout exactly (the template array is uint4[10] there, so the
    // 40 template dwords are tightly packed; see resource_mapping.md).
    let f = RootConstantType::F32;
    let mut fields: Vec<(String, RootConstantType)> = vec![
        ("surface_count".to_owned(), f),              // dword 0 (u32)
        ("max_instance_count".to_owned(), f),         // dword 1 (u32)
        ("survivor_count_word_offset".to_owned(), f), // dword 2 (u32)
        ("pad0".to_owned(), f),                       // dword 3 (u32)
    ];
    // dwords 4..43: 8 surfaces x {index_count, instance_count_reserved,
    // first_index, vertex_offset, first_instance} (all u32).
    for s in 0..8 {
        for field in [
            "index_count",
            "instance_count_reserved",
            "first_index",
            "vertex_offset",
            "first_instance",
        ] {
            fields.push((format!("surface{s}_{field}"), f));
        }
    }
    debug_assert_eq!(fields.len(), 44);
    let constants = pack_root_constants(fields);
    rs.parameters
        .insert(0, RootParameter::RootConstants { constants });
    let rts0 = serialize_rts0(&rs);
    if let Err(e) = std::fs::write(&out, &rts0) {
        eprintln!("write RTS0 failed: {e}");
        return std::process::ExitCode::FAILURE;
    }
    println!(
        "emit_grx018_indirect_args_rts0: wrote {} bytes to {out}",
        rts0.len()
    );
    std::process::ExitCode::SUCCESS
}

#[cfg(not(feature = "dxil-backend"))]
fn main() {
    eprintln!("emit_grx018_indirect_args_rts0 requires --features dxil-backend");
}
