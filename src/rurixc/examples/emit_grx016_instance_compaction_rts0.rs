#[cfg(feature = "dxil-backend")]
fn main() -> std::process::ExitCode {
    use rurixc::binding_layout::{
        RootConstantType, RootParameter, infer_root_signature, pack_root_constants, serialize_rts0,
    };
    use rurixc::mir::{MirResourceType, ResourceBinding, ResourceCount};

    const USAGE: &str = "usage: emit_grx016_instance_compaction_rts0 <scan_local|scan_groups|scatter> <descriptor_layout.json> <out.bin>";

    let mut args = std::env::args().skip(1);
    let variant = match args.next() {
        Some(v) => v,
        None => {
            eprintln!("{USAGE}");
            return std::process::ExitCode::from(2);
        }
    };
    let descriptor = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("{USAGE}");
            return std::process::ExitCode::from(2);
        }
    };
    let out = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("{USAGE}");
            return std::process::ExitCode::from(2);
        }
    };
    if args.next().is_some() {
        eprintln!("{USAGE}");
        return std::process::ExitCode::from(2);
    }

    let descriptor_text = match std::fs::read_to_string(&descriptor) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("read descriptor failed: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    // The GRX-016 instance_compaction descriptor contract: ONE descriptor JSON
    // carrying the three-dispatch chain variants (scan_local / scan_groups /
    // scatter) plus the shared Rurix-defined 32-byte / 8-dword CompactionParams
    // root-constant block (there is no native Godot push constant to mirror —
    // Godot has no native compaction pass; see resource_mapping.md). Shared
    // needles first, then per-variant resource needles.
    for needle in [
        "\"pass_id\": \"instance_compaction\"",
        "\"variant\": \"scan_local\"",
        "\"variant\": \"scan_groups\"",
        "\"variant\": \"scatter\"",
        "\"binding_kind\": \"structured_buffer\"",
        "\"binding_kind\": \"rwstructured_buffer\"",
        "\"name\": \"total_instances\"",
        "\"name\": \"bitmask_words\"",
        "\"name\": \"num_groups\"",
        "\"name\": \"transform_stride_vec4\"",
        "\"name\": \"visibility_mask\"",
        "\"name\": \"local_prefix\"",
        "\"name\": \"group_totals\"",
        "\"name\": \"group_offsets\"",
        "\"name\": \"survivor_count\"",
        "\"name\": \"src_transforms\"",
        "\"name\": \"dst_transforms\"",
    ] {
        if !descriptor_text.contains(needle) {
            eprintln!("unsupported GRX-016 instance_compaction descriptor: missing {needle}");
            return std::process::ExitCode::FAILURE;
        }
    }

    let srv = |name: &str| ResourceBinding {
        name: name.to_owned(),
        res: MirResourceType::StructuredBuffer { read_only: true },
        count: ResourceCount::One,
    };
    let uav = |name: &str| ResourceBinding {
        name: name.to_owned(),
        res: MirResourceType::StructuredBuffer { read_only: false },
        count: ResourceCount::One,
    };

    // Per-variant binding surfaces (resource_mapping.md): infer_root_signature
    // aggregates the SRV range ahead of the UAV range in a single descriptor
    // table, assigning registers in declaration order; the descriptor layout
    // JSON documents the real per-slot HLSL types (uint word buffers / the
    // uint4 bit-preserving transform payload).
    let resources = match variant.as_str() {
        // D1: t0 visibility_mask, u0 local_prefix, u1 group_totals.
        "scan_local" => vec![
            srv("visibility_mask"),
            uav("local_prefix"),
            uav("group_totals"),
        ],
        // D2: t0 group_totals, u0 group_offsets, u1 survivor_count.
        "scan_groups" => vec![
            srv("group_totals"),
            uav("group_offsets"),
            uav("survivor_count"),
        ],
        // D3: t0 visibility_mask, t1 src_transforms, t2 local_prefix,
        //     t3 group_offsets, u0 dst_transforms.
        "scatter" => vec![
            srv("visibility_mask"),
            srv("src_transforms"),
            srv("local_prefix"),
            srv("group_offsets"),
            uav("dst_transforms"),
        ],
        other => {
            eprintln!("unknown GRX-016 variant {other:?}; {USAGE}");
            return std::process::ExitCode::from(2);
        }
    };

    let mut rs = match infer_root_signature(&resources) {
        Ok(rs) => rs,
        Err(e) => {
            eprintln!("infer_root_signature failed: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    // Canonical GRX-016 root-constant layout: the Rurix-defined 32-byte
    // CompactionParams = 8 dwords at root_parameter_index 0, shared
    // byte-identical by all three variants so the runtime binds one blob for
    // the whole chain. All eight fields are u32, carried as 1-dword F32 slots
    // (RTS0 root constants only encode dword count/layout, not semantic type;
    // the descriptor JSON records the true u32 type per field). Field order
    // and dword offsets are normative (see resource_mapping.md).
    let f = RootConstantType::F32;
    let constants = pack_root_constants(vec![
        ("total_instances".to_owned(), f),       // dword 0 (u32): N
        ("bitmask_words".to_owned(), f),         // dword 1 (u32): ceil(N/32)
        ("num_groups".to_owned(), f),            // dword 2 (u32): ceil(N/256), <= 256
        ("transform_stride_vec4".to_owned(), f), // dword 3 (u32): 3 in scope
        ("pad0".to_owned(), f),                  // dword 4 (u32)
        ("pad1".to_owned(), f),                  // dword 5 (u32)
        ("pad2".to_owned(), f),                  // dword 6 (u32)
        ("pad3".to_owned(), f),                  // dword 7 (u32)
    ]);
    rs.parameters
        .insert(0, RootParameter::RootConstants { constants });
    let rts0 = serialize_rts0(&rs);
    if let Err(e) = std::fs::write(&out, &rts0) {
        eprintln!("write RTS0 failed: {e}");
        return std::process::ExitCode::FAILURE;
    }
    println!(
        "emit_grx016_instance_compaction_rts0: wrote {} bytes to {out} (variant {variant})",
        rts0.len()
    );
    std::process::ExitCode::SUCCESS
}

#[cfg(not(feature = "dxil-backend"))]
fn main() {
    eprintln!("emit_grx016_instance_compaction_rts0 requires --features dxil-backend");
}
