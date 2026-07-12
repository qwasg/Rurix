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
            eprintln!("usage: emit_grx014_cluster_store_rts0 <descriptor_layout.json> <out.bin>");
            return std::process::ExitCode::from(2);
        }
    };
    let out = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("usage: emit_grx014_cluster_store_rts0 <descriptor_layout.json> <out.bin>");
            return std::process::ExitCode::from(2);
        }
    };
    if args.next().is_some() {
        eprintln!("usage: emit_grx014_cluster_store_rts0 <descriptor_layout.json> <out.bin>");
        return std::process::ExitCode::from(2);
    }

    let descriptor_text = match std::fs::read_to_string(&descriptor) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("read descriptor failed: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    // The GRX-014 cluster_store descriptor contract: cluster_render SRV t0
    // (structured_buffer, uint words), render_elements SRV t1
    // (structured_buffer, 80-byte RenderElementData), cluster_store UAV u0
    // (rwstructured_buffer, uint words), and the 32-byte / 8-dword
    // ClusterStore::PushConstant root-constant block (Godot's
    // ClusterBuilderSharedDataRD::ClusterStore::PushConstant, mirrored
    // field-by-field).
    for needle in [
        "\"name\": \"cluster_render\"",
        "\"name\": \"render_elements\"",
        "\"name\": \"cluster_store\"",
        "\"binding_kind\": \"structured_buffer\"",
        "\"binding_kind\": \"rwstructured_buffer\"",
        "\"name\": \"cluster_render_data_size\"",
        "\"name\": \"max_render_element_count_div_32\"",
        "\"name\": \"cluster_screen_size_x\"",
        "\"name\": \"cluster_screen_size_y\"",
        "\"name\": \"render_element_count_div_32\"",
        "\"name\": \"max_cluster_element_count_div_32\"",
    ] {
        if !descriptor_text.contains(needle) {
            eprintln!("unsupported GRX-014 cluster_store descriptor: missing {needle}");
            return std::process::ExitCode::FAILURE;
        }
    }

    // Two SRVs (t0, t1) then one UAV (u0). infer_root_signature aggregates the
    // SRV range (2 descriptors) ahead of the UAV range (1 descriptor) in a
    // single descriptor table; the descriptor layout JSON documents the real
    // per-slot HLSL types (uint word buffers / the 80-byte RenderElementData).
    let resources = vec![
        ResourceBinding {
            name: "cluster_render".to_owned(),
            res: MirResourceType::StructuredBuffer { read_only: true },
            count: ResourceCount::One,
        },
        ResourceBinding {
            name: "render_elements".to_owned(),
            res: MirResourceType::StructuredBuffer { read_only: true },
            count: ResourceCount::One,
        },
        ResourceBinding {
            name: "cluster_store".to_owned(),
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
    // Canonical GRX-014 cluster_store root-constant layout: Godot's 32-byte
    // ClusterStore::PushConstant = 8 dwords at root_parameter_index 0. All
    // eight fields are u32, carried as 1-dword F32 slots (RTS0 root constants
    // only encode dword count/layout, not semantic type; the descriptor JSON
    // records the true u32 type per field). Field order and dword offsets
    // match the struct byte layout exactly (see resource_mapping.md).
    let f = RootConstantType::F32;
    let constants = pack_root_constants(vec![
        ("cluster_render_data_size".to_owned(), f), // dword 0 (u32)
        ("max_render_element_count_div_32".to_owned(), f), // dword 1 (u32)
        ("cluster_screen_size_x".to_owned(), f),    // dword 2 (u32)
        ("cluster_screen_size_y".to_owned(), f),    // dword 3 (u32)
        ("render_element_count_div_32".to_owned(), f), // dword 4 (u32)
        ("max_cluster_element_count_div_32".to_owned(), f), // dword 5 (u32)
        ("pad1".to_owned(), f),                     // dword 6 (u32)
        ("pad2".to_owned(), f),                     // dword 7 (u32)
    ]);
    rs.parameters
        .insert(0, RootParameter::RootConstants { constants });
    let rts0 = serialize_rts0(&rs);
    if let Err(e) = std::fs::write(&out, &rts0) {
        eprintln!("write RTS0 failed: {e}");
        return std::process::ExitCode::FAILURE;
    }
    println!(
        "emit_grx014_cluster_store_rts0: wrote {} bytes to {out}",
        rts0.len()
    );
    std::process::ExitCode::SUCCESS
}

#[cfg(not(feature = "dxil-backend"))]
fn main() {
    eprintln!("emit_grx014_cluster_store_rts0 requires --features dxil-backend");
}
