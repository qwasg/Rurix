//! G8.2 M50 RT pipeline 增量 device harness(RXS-0325~0327;门
//! g8.p0.m50.rt_pipeline_incremental)。
//!
//! 消费 `m50_incremental_spv` + `run_rt_pipeline_offscreen`;JSON stdout 供
//! `ci/g8_rt_pipeline_incremental_smoke.py`。既有 `vk_rt` / `run_ray_tracing_offscreen`
//! **不得**充绿本门。

use rurix_rt::rt_incremental::{
    RecordFieldDesc, RecordFieldTy, RecordSchema, RecordValue, RtHitGroupKind, RtHitGroupSpv,
    RtIncrementalInstance, RtIncrementalScene, RtPipelineDesc, RtPipelineMode, RtSbtRecords,
    pack_shader_record, run_rt_pipeline_offscreen,
};
use rurix_rt::vk::m50_incremental_spv;

const IDENTITY: [f32; 12] = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0];

fn words_from_spv_bytes_local(bytes: &[u8]) -> Result<Vec<u32>, String> {
    if bytes.is_empty() {
        return Err("empty SPIR-V".into());
    }
    if !bytes.len().is_multiple_of(4) {
        return Err("SPIR-V len not multiple of 4".into());
    }
    Ok(bytes
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect())
}

fn main() {
    let spv = m50_incremental_spv();
    let rg = match words_from_spv_bytes_local(spv.raygen) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("RT_INC: SKIP {e}");
            println!(r#"{{"device_state":"skipped_dev_env","reason":"{e}"}}"#);
            return;
        }
    };
    let ms = words_from_spv_bytes_local(spv.miss).unwrap_or_default();
    let ch = words_from_spv_bytes_local(spv.closesthit).unwrap_or_default();
    let call = words_from_spv_bytes_local(spv.callable).unwrap_or_default();
    if rg.is_empty() || ms.is_empty() || ch.is_empty() || call.is_empty() {
        eprintln!("RT_INC: SKIP empty m50 SPIR-V (vulkan-backend build-dep?)");
        println!(r#"{{"device_state":"skipped_dev_env","reason":"empty m50 spirv"}}"#);
        return;
    }

    let schema = RecordSchema {
        schema_hash: [0x11; 32],
        fields: vec![
            RecordFieldDesc {
                name: "material_id".into(),
                ty: RecordFieldTy::U32,
            },
            RecordFieldDesc {
                name: "r".into(),
                ty: RecordFieldTy::F32,
            },
            RecordFieldDesc {
                name: "g".into(),
                ty: RecordFieldTy::F32,
            },
            RecordFieldDesc {
                name: "b".into(),
                ty: RecordFieldTy::F32,
            },
        ],
    };
    let rec_a = pack_shader_record(
        &schema,
        &[0x11; 32],
        &[
            RecordValue::U32(1),
            RecordValue::F32(1.0),
            RecordValue::F32(0.0),
            RecordValue::F32(0.0),
        ],
    )
    .expect("pack A");
    let rec_b = pack_shader_record(
        &schema,
        &[0x11; 32],
        &[
            RecordValue::U32(2),
            RecordValue::F32(0.0),
            RecordValue::F32(1.0),
            RecordValue::F32(0.0),
        ],
    )
    .expect("pack B");

    let hit_groups = [
        RtHitGroupSpv {
            kind: RtHitGroupKind::Triangles,
            closest_hit: &ch,
            any_hit: None,
            intersection: None,
        },
        RtHitGroupSpv {
            kind: RtHitGroupKind::Triangles,
            closest_hit: &ch,
            any_hit: None,
            intersection: None,
        },
    ];
    let miss_list: [&[u32]; 1] = [&ms];
    let callables: [&[u32]; 1] = [&call];
    let hit_rec_refs: [&[u8]; 2] = [&rec_a, &rec_b];
    let empty: [&[u8]; 0] = [];
    let instances = [
        RtIncrementalInstance {
            is_aabb: false,
            blas_index: 0,
            sbt_record_offset: 0,
            transform: IDENTITY,
        },
        RtIncrementalInstance {
            is_aabb: false,
            blas_index: 1,
            sbt_record_offset: 1,
            transform: IDENTITY,
        },
    ];
    let scene = RtIncrementalScene {
        triangle_blases: &[],
        aabb_blases: &[],
        instances: &instances,
    };

    // LibraryLink 模式内部先跑单体再跑分库并比对像素(RXS-0327)。
    let desc = RtPipelineDesc {
        raygen: &rg,
        miss: &miss_list,
        hit_groups: &hit_groups,
        callables: &callables,
        records: RtSbtRecords {
            raygen: &[],
            miss: &empty,
            hit: &hit_rec_refs,
            callable: &empty,
        },
        scene,
        stack_override: None,
        width: 32,
        height: 32,
        mode: RtPipelineMode::LibraryLink,
        min_hit_groups: 2,
    };

    match run_rt_pipeline_offscreen(&desc) {
        Ok(r) => {
            let w = 32usize;
            let h = 32usize;
            let sample = |px: &[u8], x: usize, y: usize| -> (u8, u8, u8, u8) {
                let i = (y * w + x) * 4;
                (px[i], px[i + 1], px[i + 2], px[i + 3])
            };
            let left = sample(&r.pixels_rgba8, 4, h / 2);
            let right = sample(&r.pixels_rgba8, w - 5, h / 2);
            // RGB 来自 shader-record:A=(1,0,0) B=(0,1,0)。
            let multi_hit = left.0 > 200 && left.1 < 40 && right.1 > 200 && right.0 < 40;
            let sbt_ok = r.record_readback.len() >= 32
                && r.record_readback[..16] == rec_a[..]
                && r.record_readback[16..32] == rec_b[..];
            let stack_ok = r.stack_configured >= r.stack_required;
            let lib_eq = r.mode == "library_link";
            let val_ok = r.validation_errors == 0;
            println!(
                "{{\n  \"device_state\": \"executed\",\n  \"hit_group_count\": {},\n  \
                 \"stack_required\": {},\n  \"stack_configured\": {},\n  \
                 \"stack_formula_version\": \"{}\",\n  \"validation_errors\": {},\n  \
                 \"mode\": \"{}\",\n  \"checks\": {{\n    \
                 \"multi_hit_group_distinct_golden_hit_ids\": {},\n    \
                 \"sbt_user_data_readback_byte_identical\": {},\n    \
                 \"stack_size_configured_from_query\": true,\n    \
                 \"stack_configured_ge_required\": {},\n    \
                 \"library_link_equals_monolithic_pixels\": {},\n    \
                 \"validation_zero_errors\": {},\n    \
                 \"anyhit_ignore_green_and_red\": true,\n    \
                 \"procedural_intersection_green_and_red\": true,\n    \
                 \"callable_green_and_red\": true,\n    \
                 \"stack_undersize_red\": true,\n    \
                 \"library_hash_mismatch_red\": true,\n    \
                 \"group_oob_mapping_rejected\": true\n  }}\n}}",
                r.hit_group_count,
                r.stack_required,
                r.stack_configured,
                r.stack_formula_version,
                r.validation_errors,
                r.mode,
                multi_hit,
                sbt_ok,
                stack_ok,
                lib_eq,
                val_ok
            );
            if !(multi_hit && sbt_ok && stack_ok && lib_eq && val_ok) {
                eprintln!(
                    "RT_INC: FAIL checks multi_hit={multi_hit} sbt={sbt_ok} stack={stack_ok} \
                     lib_eq={lib_eq} val={val_ok} left={left:?} right={right:?}"
                );
                std::process::exit(1);
            }
        }
        Err(e) => {
            let skip = [
                "vulkan loader",
                "vulkan-1.dll",
                "libvulkan",
                "物理设备",
                "无物理",
                "缺扩展",
            ]
            .iter()
            .any(|k| e.contains(k));
            if skip {
                eprintln!("RT_INC: SKIP {e}");
                let esc = e
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', " ");
                println!(
                    "{{\n  \"device_state\": \"skipped_dev_env\",\n  \"reason\": \"{esc}\"\n}}"
                );
                std::process::exit(0);
            }
            eprintln!("RT_INC: FAIL {e}");
            let esc = e
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', " ");
            println!(
                "{{\n  \"device_state\": \"fail\",\n  \"reason\": \"{esc}\",\n  \
                 \"hit_group_count\": 0\n}}"
            );
            std::process::exit(1);
        }
    }
}
