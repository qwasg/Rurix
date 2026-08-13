//! rxhlod — G9.5 M110 波 HLOD 离线烘焙工具(RXS-0364 语义锚最小面;asset graph
//! 注册工具 `rurix.hlod.bake.v1`)。
//!
//! 输入 cell 几何资产(逐 Component 三角面),经 [`rurix_asset::hlod`] 离线
//! Builder 逐 Component 分发产出 HLOD 层级资产字节(产物即资产,canonical
//! 编码 + digest 寻址)。纯 host 离线,GPU 非必需。
//!
//! 用法:
//! ```text
//! rxhlod bake [--out <dir>]     # 烘焙确定性 demo cell 几何,写 hlod_demo.rxhlod,
//!                               #   stdout 打印 {digest, levels, tris_per_level}
//! rxhlod check                  # 双构建 hash 相等 + 声明序扰动免疫 + 几何扰动
//!                               #   分叉(RED 臂能红证明),stdout JSON verdict
//! ```

#![forbid(unsafe_code)]

use rurix_asset::hlod::{
    HlodBakeInput, bake_hlod, demo_bake_input, encode_hlod_asset, hlod_asset_digest,
};
use std::env;
use std::fs;
use std::path::PathBuf;

fn hex(d: &[u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

fn fail(msg: &str) -> ! {
    eprintln!("[rxhlod] FAIL {msg}");
    std::process::exit(1)
}

fn bake_json(input: &HlodBakeInput) -> (Vec<u8>, String) {
    let asset = bake_hlod(input).unwrap_or_else(|e| fail(&format!("烘焙: {e}")));
    let bytes = encode_hlod_asset(&asset);
    let digest = hlod_asset_digest(&asset);
    let tris: Vec<String> = asset
        .levels
        .iter()
        .map(|l| {
            format!(
                "{{\"level\": {}, \"proxy_triangles\": {}}}",
                l.level,
                l.proxies
                    .iter()
                    .map(|p| p.proxy_triangles.len() as u64)
                    .sum::<u64>()
            )
        })
        .collect();
    let json = format!(
        "{{\"ok\": true, \"tool\": \"{}\", \"cell\": \"{}\", \"digest\": \"{}\", \"bytes\": {}, \"levels\": [{}]}}",
        rurix_asset::graph::TOOL_HLOD_BAKE,
        asset.cell_name,
        hex(&digest),
        bytes.len(),
        tris.join(", ")
    );
    (bytes, json)
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some(cmd) = args.first() else {
        fail("用法: rxhlod bake [--out <dir>] | rxhlod check");
    };
    match cmd.as_str() {
        "bake" => {
            let mut out_dir: Option<PathBuf> = None;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--out" => {
                        i += 1;
                        out_dir = Some(PathBuf::from(
                            args.get(i).unwrap_or_else(|| fail("--out 缺目录")),
                        ));
                    }
                    other => fail(&format!("未知参数: {other}")),
                }
                i += 1;
            }
            let (bytes, json) = bake_json(&demo_bake_input());
            if let Some(dir) = out_dir {
                fs::create_dir_all(&dir).unwrap_or_else(|e| fail(&format!("建目录 {dir:?}: {e}")));
                let path = dir.join("hlod_demo.rxhlod");
                fs::write(&path, &bytes).unwrap_or_else(|e| fail(&format!("写 {path:?}: {e}")));
                eprintln!("[rxhlod] asset written: {path:?}");
            }
            println!("{json}");
        }
        "check" => {
            // 双构建 hash 相等(同输入两次独立烘焙逐位一致)。
            let input = demo_bake_input();
            let a = bake_hlod(&input).unwrap_or_else(|e| fail(&format!("bake#1: {e}")));
            let b = bake_hlod(&input).unwrap_or_else(|e| fail(&format!("bake#2: {e}")));
            let bytes_a = encode_hlod_asset(&a);
            let bytes_b = encode_hlod_asset(&b);
            let double_build_equal = bytes_a == bytes_b;
            // 声明序扰动免疫(Component 逆序 + 三角面乱序 ⇒ digest 不变)。
            let mut shuffled = input.clone();
            shuffled.components.reverse();
            for c in shuffled.components.iter_mut() {
                c.triangles.reverse();
                c.triangles.rotate_left(13);
            }
            let order_invariant =
                hlod_asset_digest(&bake_hlod(&shuffled).unwrap_or_else(|e| fail(&e.to_string())))
                    == hlod_asset_digest(&a);
            // 几何扰动必须分叉(RED 臂能红证明)。
            let mut moved = input;
            moved.components[0].triangles[0][0] += 1.0;
            let drift_detected =
                hlod_asset_digest(&bake_hlod(&moved).unwrap_or_else(|e| fail(&e.to_string())))
                    != hlod_asset_digest(&a);
            let ok = double_build_equal && order_invariant && drift_detected;
            println!(
                "{{\"ok\": {ok}, \"double_build_equal\": {double_build_equal}, \"order_invariant\": {order_invariant}, \"drift_detected\": {drift_detected}, \"digest\": \"{}\"}}",
                hex(&hlod_asset_digest(&a))
            );
            if !ok {
                fail("check 判据不全真");
            }
        }
        other => fail(&format!("未知子命令: {other}(bake|check)")),
    }
}
