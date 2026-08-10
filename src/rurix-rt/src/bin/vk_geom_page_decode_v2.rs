//! G9.2 M91 page_format_v2_abi **device 解码 harness**(RXS-0344/RXS-0342 体例;
//! 门 `g9.p0.m91.page_format_v2_abi`)。
//!
//! 用法:
//! ```text
//! vk_geom_page_decode_v2 --spv <geom_page_decode_v2.spv> --rxpl <page.rxpl(major=2)>
//! ```
//!
//! 上传 RXPL v2 页字节 → compute 结构性展开 → 回读展开流 → stdout JSON:
//! `expanded_digest`(SHA-256 hex)、`expanded_u32_count`、`validation_errors`、
//! `device_state`。digest 比对在 smoke 层与 CPU 对照(逐位等判据)。
//!
//! 三态:无 loader/GPU → `device_state=skipped_dev_env` 退 0;
//! `RURIX_REQUIRE_REAL=1` 翻硬红由 smoke 裁决。validation 经
//! `RURIX_VK_VALIDATION=1` 装层,ERROR → 非零退出。

use std::path::PathBuf;

const NO_DEVICE_KEYS: &[&str] = &[
    "vulkan loader",
    "vulkan-1.dll",
    "libvulkan",
    "物理设备",
    "compute queue",
    "vkCreateInstance",
];

fn is_no_device(e: &str) -> bool {
    NO_DEVICE_KEYS.iter().any(|k| e.contains(k))
}

fn fail(msg: &str) -> ! {
    eprintln!("GPD2: FAIL {msg}");
    std::process::exit(1)
}

fn hex_of(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let spv_path = arg_value(&args, "--spv")
        .unwrap_or_else(|| fail("用法: vk_geom_page_decode_v2 --spv <path> --rxpl <path>"));
    let rxpl_path = arg_value(&args, "--rxpl")
        .unwrap_or_else(|| fail("用法: vk_geom_page_decode_v2 --spv <path> --rxpl <path>"));

    eprintln!(
        "[vk_geom_page_decode_v2] G9.2 M91 device decode harness(RXS-0344); spv={} rxpl={}",
        spv_path.display(),
        rxpl_path.display()
    );

    let spv_raw = std::fs::read(&spv_path).unwrap_or_else(|e| fail(&format!("读 spv: {e}")));
    if spv_raw.len() % 4 != 0 {
        fail("SPIR-V 字节数非 4 对齐");
    }
    let spv: Vec<u32> = spv_raw
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    let entry = rurix_rt::vk::entry_point_name(&spv).unwrap_or_else(|| fail("无 OpEntryPoint"));

    let mut rxpl = std::fs::read(&rxpl_path).unwrap_or_else(|e| fail(&format!("读 rxpl: {e}")));
    if rxpl.len() < 160 {
        fail("RXPL v2 过短");
    }
    if &rxpl[0..4] != b"RXPL" {
        fail("非 RXPL magic");
    }
    if u16::from_le_bytes([rxpl[8], rxpl[9]]) != 2 {
        fail("非 RXPL major=2");
    }
    // pad to 4
    while rxpl.len() % 4 != 0 {
        rxpl.push(0);
    }

    let expect = arg_value_str(&args, "--expect-u32-count")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or_else(|| fail("--expect-u32-count <n> 必需(与 CPU expand_u32_count_v2 对齐)"));
    // 输出缓冲:期望字数 + 余量(kernel 边界守卫用 out_cap)。
    let out_cap = (expect + 64) as u32;
    let out = vec![0u8; out_cap as usize * 4];
    let page_bytes_logical = std::fs::metadata(&rxpl_path)
        .map(|m| m.len() as u32)
        .unwrap_or(rxpl.len() as u32);

    let mut buffers = vec![rxpl, out];
    let mut pc = Vec::new();
    pc.extend_from_slice(&page_bytes_logical.to_le_bytes());
    pc.extend_from_slice(&out_cap.to_le_bytes());

    match rurix_rt::vk::run_compute(&spv, &entry, &mut buffers, &pc, [1, 1, 1]) {
        Ok(()) => {}
        Err(e) if is_no_device(&e) => {
            eprintln!("GPD2: SKIP 无 Vulkan 设备({})", e.trim());
            println!(
                "{{\n  \"device_state\": \"skipped_dev_env\",\n  \"reason\": \"{}\"\n}}",
                e.trim().replace('\\', "\\\\").replace('"', "\\\"")
            );
            return;
        }
        Err(e) if e.contains("validation") || e.contains("VK_LAYER") => {
            eprintln!("GPD2: FAIL validation: {e}");
            std::process::exit(2);
        }
        Err(e) => fail(&format!("dispatch: {e}")),
    }

    let out_bytes = &buffers[1];
    let take = expect * 4;
    let stream = &out_bytes[..take];
    let digest = sha256_impl(stream);

    if args.iter().any(|a| a == "--dump-stream") {
        let mut hex = String::with_capacity(stream.len() * 2);
        for b in stream {
            hex.push_str(&format!("{b:02x}"));
        }
        println!("{{\"stream_hex\": \"{hex}\"}}");
    }

    println!(
        "{{\n  \"device_state\": \"executed\",\n  \"entry\": \"{entry}\",\n  \
         \"expanded_u32_count\": {expect},\n  \"expanded_digest\": \"{}\",\n  \
         \"validation_errors\": 0\n}}",
        hex_of(&digest)
    );
}

fn arg_value(args: &[String], key: &str) -> Option<PathBuf> {
    args.iter()
        .position(|a| a == key)
        .and_then(|i| args.get(i + 1).map(PathBuf::from))
}

fn arg_value_str<'a>(args: &'a [String], key: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == key)
        .and_then(|i| args.get(i + 1).map(|s| s.as_str()))
}

// ── 最小 SHA-256(公有域;与 rurix_pkg 逐位兼容;与 vk_geom_page_decode.rs 同份)──

fn sha256_impl(msg: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let bit_len = (msg.len() as u64).wrapping_mul(8);
    let mut buf = msg.to_vec();
    buf.push(0x80);
    while (buf.len() % 64) != 56 {
        buf.push(0);
    }
    buf.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in buf.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes(chunk[i * 4..i * 4 + 4].try_into().unwrap());
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = [0u8; 32];
    for (i, v) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes());
    }
    out
}
