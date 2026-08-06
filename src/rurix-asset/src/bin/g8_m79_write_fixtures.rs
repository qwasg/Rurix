//! one-shot fixture writer for M79 canon goldens.
use rurix_asset::canon::*;
use std::fs;
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/asset/canon");
    let accept = root.join("accept");
    let reject = root.join("reject");
    fs::create_dir_all(&accept).unwrap();
    fs::create_dir_all(&reject).unwrap();
    let objs: Vec<(&str, Value)> = vec![
        (
            "source_asset",
            Value::map_of([
                (1, Value::Int(1)),
                (2, Value::text_ascii("asset/tri_min.gltf").unwrap()),
                (3, Value::Bytes(vec![0xab; 32])),
                (4, Value::Int(128)),
            ])
            .unwrap(),
        ),
        (
            "import_recipe",
            Value::map_of([
                (1, Value::Int(1)),
                (2, Value::text_ascii("rurix.gltf.import.v1").unwrap()),
                (3, Value::text_ascii("1.0.0").unwrap()),
            ])
            .unwrap(),
        ),
        (
            "cook_profile",
            Value::map_of([
                (1, Value::Int(1)),
                (2, Value::text_ascii("win").unwrap()),
                (3, Value::text_ascii("x86_64").unwrap()),
                (4, Value::text_ascii("vulkan").unwrap()),
            ])
            .unwrap(),
        ),
        (
            "derived_artifact",
            Value::map_of([
                (1, Value::Int(1)),
                (2, Value::text_ascii("geom.pages").unwrap()),
                (3, Value::Bytes(vec![1; 32])),
                (4, Value::Int(4096)),
            ])
            .unwrap(),
        ),
        (
            "tool_manifest",
            Value::map_of([
                (1, Value::Int(1)),
                (2, Value::text_ascii("rurix.geom.pages.v1").unwrap()),
                (3, Value::text_ascii("1.0.0").unwrap()),
            ])
            .unwrap(),
        ),
        (
            "build_manifest",
            Value::map_of([
                (1, Value::Int(1)),
                (2, Value::Array(vec![Value::text_ascii("a").unwrap()])),
                (3, Value::Int(2)),
            ])
            .unwrap(),
        ),
    ];
    for (name, v) in objs {
        let sd = schema_digest_for(name, 1, 1, 0);
        let bytes = wrap_value(1, 1, 0, sd, &v).unwrap();
        fs::write(accept.join(format!("{name}.rxap")), &bytes).unwrap();
    }
    fs::write(reject.join("non_shortest_int.cbor"), [0x18u8, 0x01]).unwrap();
    fs::write(reject.join("indefinite_array.cbor"), [0x9f]).unwrap();
    fs::write(reject.join("non_ascii_text.cbor"), [0x62, 0xc3, 0xa9]).unwrap();
    fs::write(
        reject.join("duplicate_field_id.cbor"),
        [0xa2, 0x01, 0x00, 0x01, 0x00],
    )
    .unwrap();
    let mut env = fs::read(accept.join("source_asset.rxap")).unwrap();
    env[0] = b'X';
    fs::write(reject.join("bad_magic.rxap"), &env).unwrap();
    fs::write(reject.join("float_half.cbor"), [0xf9, 0x00, 0x00]).unwrap();
    let gdir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/asset/graph/reject");
    fs::create_dir_all(&gdir).unwrap();
    fs::write(gdir.join("cycle.txt"), "tool_a -> tool_b -> tool_a\n").unwrap();
    fs::write(gdir.join("unregistered_tool.txt"), "shell.evil\n").unwrap();
    eprintln!("fixtures written");
}
