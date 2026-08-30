//! G37 W2(夜航 night_0830)postchain LUT + PSO warmup 两模块 host 自检
//! harness(纯 CPU,零 device 依赖;不计真门——窗口 bin 合入前的模块级
//! 机核面):
//!
//! 1. `g37_w2/g31_lut_assets.rs`:中性 LUT trilinear 恒等界、warm preset
//!    判别、.cube 解析双构建相等 + fail-closed 拒臂、encode 参数尾挂布局
//!    ([134] 门/[135] dim/[136..) 表体)、格点采样位级取角、表体 digest
//!    跨构建稳定(双生成 == )。
//! 2. `g37_w2/g31_pso_warmup.rs`:era 0 precache 幂等零告警、era 1 同变体
//!    集全命中零告警、新变体遭遇告警 +1 且登记行、报告 JSON 字段自洽。
//! 3. `g37_w2/g31_ris_lamps.rs`(G37 W2 ris_nee):灯片表 + 功率 CDF 构建
//!    ——解析夹具 CDF/记录段逐槽断言、末项恒 1.0、双构建位级相等、谓词
//!    (非发射/灯面尾段排除 + 零面积保留)、fail-closed 拒臂(零命中/长度
//!    失配/顶点越界/超二分覆盖域)。
//!
//! 通过 = 单行 JSON(schema `rurix.g37.w2_selfcheck.v1`)退 0;任一断言
//! 失败即 panic 非零退出(fail-closed)。
#![forbid(unsafe_code)]

include!("g37_w2/g31_lut_assets.rs");
include!("g37_w2/g31_pso_warmup.rs");
include!("g37_w2/g31_ris_lamps.rs");

use g31_lut_assets::{
    G31_LUT_DIM_DEFAULT, G31_LUT_PARAMS_BASE, extend_encode_params, from_arg, neutral,
    parse_cube, preset_warm, sample_trilinear_f32,
};
use g31_pso_warmup::G31PsoLedger;
use g31_ris_lamps::{
    G31_RIS_LAMP_DUMMY_BYTES, G31_RIS_LAMP_HDR, G31_RIS_LAMP_MAX, G31_RIS_LAMP_STRIDE,
    build_lamp_table,
};

/// 中性 LUT 采样恒等界(trilinear 对逐坐标线性函数精确至 f32 舍入;
/// 8-bit 量化 quantum = 1/255 ≈ 3.9e-3,界取 4 ULP 级 ≪ quantum)。
const NEUTRAL_EPS: f32 = 5.0e-7;

fn check_lut() -> (String, String) {
    let neu = neutral(G31_LUT_DIM_DEFAULT);
    assert_eq!(neu.table.len(), 3 * 17 * 17 * 17, "neutral 表体长度");
    // 确定性网格点(含端点/格点/格间)恒等界。
    let mut worst = 0.0f32;
    for i in 0..=20u32 {
        for j in 0..=4u32 {
            let x = i as f32 / 20.0;
            let y = ((i + 7 * j) % 21) as f32 / 20.0;
            let z = ((i * 3 + j) % 21) as f32 / 20.0;
            let out = sample_trilinear_f32(&neu, [x, y, z]);
            for (o, e) in out.iter().zip([x, y, z]) {
                worst = worst.max((o - e).abs());
            }
        }
    }
    assert!(
        worst <= NEUTRAL_EPS,
        "中性 LUT trilinear 恒等界越界: worst={worst:e} > {NEUTRAL_EPS:e}"
    );
    // 格点采样位级取角(f=0 权重精确;host/device 同序同语义的结构性锚)。
    let dim = neu.dim;
    for (r, g, b) in [(0usize, 0usize, 0usize), (16, 16, 16), (8, 3, 12)] {
        let input = [
            r as f32 / (dim - 1) as f32,
            g as f32 / (dim - 1) as f32,
            b as f32 / (dim - 1) as f32,
        ];
        let out = sample_trilinear_f32(&neu, input);
        let base = (r + g * dim + b * dim * dim) * 3;
        assert_eq!(
            out.to_vec(),
            neu.table[base..base + 3].to_vec(),
            "格点 ({r},{g},{b}) 采样须位级取角"
        );
    }
    // warm preset 判别(与中性必不同;digest 跨构建稳定)。
    let warm = preset_warm(G31_LUT_DIM_DEFAULT);
    assert_eq!(neu.source, "neutral", "来源登记字面");
    assert_eq!(warm.source, "warm", "来源登记字面");
    assert_ne!(
        neu.table_sha256(),
        warm.table_sha256(),
        "warm preset 须与 neutral 判别"
    );
    assert_eq!(neutral(17).table_sha256(), neu.table_sha256(), "neutral 双生成 ==");
    assert_eq!(
        preset_warm(17).table_sha256(),
        warm.table_sha256(),
        "warm 双生成 =="
    );
    let mid = sample_trilinear_f32(&warm, [0.5, 0.5, 0.5]);
    assert!(mid[0] > mid[2], "warm 中灰须 R>B(暖移方向锚): {mid:?}");
    // .cube round-trip:neutral 表体序列化为 .cube 文本 → 解析 → 表体位级相等。
    let mut cube = format!("TITLE \"g37 w2 roundtrip\"\nLUT_3D_SIZE {}\n", neu.dim);
    cube.push_str("DOMAIN_MIN 0 0 0\nDOMAIN_MAX 1 1 1\n");
    for p in neu.table.chunks(3) {
        cube.push_str(&format!("{:.9} {:.9} {:.9}\n", p[0], p[1], p[2]));
    }
    let parsed = parse_cube(&cube, "roundtrip").unwrap_or_else(|e| panic!("round-trip 解析: {e}"));
    assert_eq!(parsed.dim, neu.dim);
    // 1/16 步进的 9 位十进制往返 f32 精确(短小数),位级相等成立。
    assert_eq!(
        parsed.table_sha256(),
        neu.table_sha256(),
        ".cube round-trip 表体须位级相等"
    );
    // fail-closed 拒臂:1D / 非单位域 / 行数不符 / 非法尺寸。
    assert!(parse_cube("LUT_1D_SIZE 4\n0 0 0\n", "red").is_err(), "1D 须拒");
    assert!(
        parse_cube("LUT_3D_SIZE 2\nDOMAIN_MAX 2 2 2\n", "red").is_err(),
        "非单位域须拒"
    );
    assert!(
        parse_cube("LUT_3D_SIZE 2\n0 0 0\n", "red").is_err(),
        "行数 ≠ N³ 须拒"
    );
    assert!(parse_cube("LUT_3D_SIZE 65\n", "red").is_err(), "尺寸越界须拒");
    // from_arg 闭集。
    assert!(from_arg("off").unwrap_or_else(|e| panic!("{e}")).is_none());
    assert!(from_arg("neutral").unwrap_or_else(|e| panic!("{e}")).is_some());
    assert!(from_arg("warm").unwrap_or_else(|e| panic!("{e}")).is_some());
    assert!(from_arg(".tmp/night_0830/__missing__.cube").is_err(), "缺文件须拒");
    // encode 参数尾挂布局([134] 门/[135] dim/[136..) 表体;断言卫兵在内)。
    let mut params = vec![0.0f32; G31_LUT_PARAMS_BASE];
    extend_encode_params(&mut params, &neu);
    assert_eq!(params.len(), G31_LUT_PARAMS_BASE + neu.table.len());
    assert_eq!(params[134], 1.0, "[134] lut_gate");
    assert_eq!(params[135], 17.0, "[135] lut_dim");
    assert_eq!(params[136], neu.table[0], "[136] 表体首元");
    (neu.table_sha256(), warm.table_sha256())
}

fn check_pso() -> String {
    let mut ledger = G31PsoLedger::new();
    // era 0 = precache 面(五 pass 形态镜像:encode 与 encode_fg1 同 SPV
    // 同变体——会话级去重同判)。
    assert_eq!(ledger.begin_session(), 0);
    let spv_a = vec![1u8, 2, 3, 4];
    let spv_b = vec![9u8, 9, 9];
    let spv_c = vec![7u8; 16];
    assert!(!ledger.register("g14_3_direct_gi", &spv_a));
    assert!(!ledger.register("g14_mv", &spv_b));
    assert!(!ledger.register("g31_display_encode", &spv_c));
    assert!(!ledger.register("g31_display_encode_fg1", &spv_c));
    assert_eq!(ledger.unique_variants(), 3, "同 SPV 复用 = 同变体");
    assert_eq!(ledger.runtime_creates(), 0, "precache 面零告警");
    // era 1 = 同变体集重建(resize/风暴口径)全命中零告警。
    assert_eq!(ledger.begin_session(), 1);
    assert!(!ledger.register("g14_3_direct_gi", &spv_a));
    assert!(!ledger.register("g31_display_encode", &spv_c));
    assert_eq!(ledger.runtime_creates(), 0, "era 重建同变体集须零告警");
    // 运行期新变体遭遇 = 告警 +1 且登记行(验收归零口径的 RED 臂)。
    let spv_new = vec![0xAAu8; 8];
    assert!(ledger.register("lazy_new_pass", &spv_new), "新变体须报 miss");
    assert_eq!(ledger.runtime_creates(), 1);
    assert!(!ledger.register("lazy_new_pass", &spv_new), "二遇同变体不再涨");
    assert_eq!(ledger.runtime_creates(), 1);
    assert_eq!(ledger.sessions(), 2, "会话计数");
    let report = ledger.report_json();
    assert!(report.contains("\"pso_runtime_creates\":1"), "报告字段: {report}");
    assert!(report.contains("\"unique_variants\":4"), "报告字段: {report}");
    assert!(report.contains("\"sessions\":2"), "报告字段: {report}");
    assert!(report.contains("lazy_new_pass"), "登记行须含新变体 pass 名");
    report
}

/// G37 W2 ris_nee:灯片表构建机核(夹具解析断言 + 双构建位级 + fail-closed)。
fn check_ris_lamps() -> String {
    const TRI_NONE: u32 = u32::MAX;
    // 夹具:tri0 = 单位直角三角(面积 0.5,Le [2,0,0] ⇒ 功率 π)/tri1 =
    // 非发射/tri2 = 发射但灯面尾段(排除)/tri3 = 2×2 直角三角(面积 2,
    // Le [0,1,0] ⇒ 功率 2π)/tri4 = 零面积发射(保留,功率 0)。
    let pos: Vec<[f32; 3]> = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [2.0, 0.0, 0.0],
        [4.0, 0.0, 0.0],
        [2.0, 2.0, 0.0],
    ];
    let idx: Vec<[u32; 3]> = vec![
        [0, 1, 2],
        [0, 1, 2],
        [0, 1, 2],
        [3, 4, 5],
        [0, 0, 0],
    ];
    let em: Vec<[f32; 3]> = vec![
        [2.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [5.0, 5.0, 5.0],
        [0.0, 1.0, 0.0],
        [3.0, 0.0, 0.0],
    ];
    let tm: Vec<u32> = vec![0, 0, TRI_NONE, 1, 2];
    let (t1, s1) = build_lamp_table(&pos, &idx, &em, &tm, TRI_NONE).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(s1.emissive_tris, 3, "表成员 = tri0/tri3/tri4");
    assert_eq!(s1.zero_area_tris, 1, "零面积成员 tri4");
    assert_eq!(s1.pdf_underflow_tris, 0, "夹具无 f32 CDF 差下溢");
    let p = f64::from(std::f32::consts::PI);
    assert_eq!(s1.total_power, (p + 2.0 * p) + 0.0, "总功率 = π + 2π(同序精确)");
    assert_eq!(
        s1.table_f32_len,
        G31_RIS_LAMP_HDR + 3 + 3 * G31_RIS_LAMP_STRIDE,
        "表长 = 头 + CDF + 记录"
    );
    assert_eq!(t1.len(), s1.table_f32_len);
    // 头/CDF 段:Q=3、末项恒 1.0、单调不减、首项 = 1/3(功率占比)。
    assert_eq!(t1[0], 3.0, "[0] = Q");
    assert_eq!(t1[1], s1.total_power as f32, "[1] = 总功率下投");
    assert!((t1[4] - 1.0 / 3.0).abs() < 1e-6, "cdf[0] = π/3π: {}", t1[4]);
    assert_eq!(t1[5], 1.0, "cdf[1] = 3π/3π 精确");
    assert_eq!(t1[6], 1.0, "cdf[末] 强制 1.0");
    assert!(t1[4] <= t1[5] && t1[5] <= t1[6], "CDF 单调不减");
    // 记录段逐槽(tri0 @ base 7:A/e1/e2/n/area/Le)。
    let r0 = &t1[7..23];
    assert_eq!(&r0[0..3], &[0.0, 0.0, 0.0], "A");
    assert_eq!(&r0[3..6], &[1.0, 0.0, 0.0], "e1");
    assert_eq!(&r0[6..9], &[0.0, 1.0, 0.0], "e2");
    assert_eq!(&r0[9..12], &[0.0, 0.0, 1.0], "单位法线");
    assert_eq!(r0[12], 0.5, "面积");
    assert_eq!(&r0[13..16], &[2.0, 0.0, 0.0], "Le");
    // 零面积成员(tri4 @ base 7+32):法线占位 0 向量 + 面积 0。
    let r2 = &t1[7 + 32..7 + 48];
    assert_eq!(&r2[9..12], &[0.0, 0.0, 0.0], "零面积法线占位");
    assert_eq!(r2[12], 0.0, "零面积");
    // 双构建位级相等(确定性机核)。
    let (t2, _) = build_lamp_table(&pos, &idx, &em, &tm, TRI_NONE).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(t1.len(), t2.len());
    assert!(
        t1.iter().zip(&t2).all(|(a, b)| a.to_bits() == b.to_bits()),
        "双构建须位级相等"
    );
    // fail-closed 拒臂:零命中/长度失配/顶点越界/超二分覆盖域。
    assert!(
        build_lamp_table(&pos, &idx[1..2], &em[1..2], &tm[1..2], TRI_NONE).is_err(),
        "零命中须拒"
    );
    assert!(
        build_lamp_table(&pos, &idx, &em[..4], &tm, TRI_NONE).is_err(),
        "长度失配须拒"
    );
    assert!(
        build_lamp_table(&pos[..2], &idx[..1], &em[..1], &tm[..1], TRI_NONE).is_err(),
        "顶点越界须拒"
    );
    let big_n = G31_RIS_LAMP_MAX + 1;
    assert!(
        build_lamp_table(
            &pos,
            &vec![[0u32, 1, 2]; big_n],
            &vec![[1.0f32, 1.0, 1.0]; big_n],
            &vec![0u32; big_n],
            TRI_NONE,
        )
        .is_err(),
        "Q > 65536 须拒(二分覆盖域)"
    );
    // 关臂哑表触达域(kernel 保底读 [0..20) ⇒ 80 B)。
    assert_eq!(G31_RIS_LAMP_DUMMY_BYTES, 80, "哑表 = 20 f32 全零");
    format!(
        "{{\"emissive_tris\":{},\"zero_area_tris\":{},\"pdf_underflow_tris\":{},\"total_power\":{:.9},\"table_f32_len\":{},\"double_build\":\"bitexact\"}}",
        s1.emissive_tris, s1.zero_area_tris, s1.pdf_underflow_tris, s1.total_power, s1.table_f32_len
    )
}

fn main() {
    let (neutral_sha, warm_sha) = check_lut();
    let pso_report = check_pso();
    let ris_report = check_ris_lamps();
    println!(
        "{{\"schema\":\"rurix.g37.w2_selfcheck.v1\",\"lut\":{{\"dim\":17,\"neutral_table_sha256\":\"{neutral_sha}\",\"warm_table_sha256\":\"{warm_sha}\",\"neutral_eps\":{NEUTRAL_EPS:e},\"cube_roundtrip\":\"bitexact\"}},\"pso_ledger_red_arm_report\":{pso_report},\"ris_lamps\":{ris_report},\"pass\":true}}"
    );
}
