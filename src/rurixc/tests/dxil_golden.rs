//! DXIL golden guardrail(G2.2 PR-C2 分片1,RXS-0157;RFC-0003 §9 Q-Golden;cargo
//! feature `dxil-backend`)。两层 golden:
//! - **`.dxil-ll`**(always-on):rurixc 自有 DirectX 三元组 LLVM IR 文本产物
//!   (确定性、无外部工具依赖,对齐 ptx_golden 取 IR 层的纪律);
//! - **`.dxil-disasm`**(工具链关卡):经 patched llc `-filetype=obj` 产 DXIL 容器 +
//!   dxc validator **接受后**的文本反汇编(RFC-0003 §9 Q-Golden);patched llc
//!   (`RURIX_LLC`)/ dxc validator(`RURIX_DXC_DIR`)缺失 → SKIP(开发环境降级,真实
//!   红绿在带工具链环境,对齐 RXS-0073 ptxas 干验证 SKIP 纪律)。
//!
//! **bless 是审批动作**:`RURIX_BLESS=1` 重写 golden;变更须伴随 `tests/dxil/
//! bless_log.md` 追加记录(ci/check_guardrails.py 核对)。
#![cfg(feature = "dxil-backend")]

use std::fs;
use std::path::{Path, PathBuf};

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

fn dxil_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/dxil")
}

fn rx_files() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = fs::read_dir(dxil_dir())
        .expect("读取 tests/dxil 失败")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "rx"))
        .collect();
    out.sort();
    out
}

fn bless_mode() -> bool {
    std::env::var("RURIX_BLESS").is_ok_and(|v| v == "1")
}

/// 全管线产出 device DXIL DirectX 三元组 LLVM IR 文本(`kernel fn` 根;0 诊断断言)。
fn dxil_ir(src: &str, module: &str) -> String {
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    cx.check_coloring();
    cx.check_crate_patterns();
    cx.check_consteval();
    assert!(!diag.has_errors(), "DXIL golden 语料须 0 诊断");
    rurixc::dxil_codegen::build_and_emit_dxil(&cx, module).expect("应产出 DXIL IR")
}

/// `.dxil-ll` golden:rurixc 自有 DirectX 三元组 LLVM IR(确定性,always-on)。
#[test]
fn dxil_ll_golden_matches() {
    let bless = bless_mode();
    let mut mismatches = Vec::new();
    for path in rx_files() {
        let src = fs::read_to_string(&path)
            .expect("读取语料失败")
            .replace("\r\n", "\n");
        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
        let ir = dxil_ir(&src, &stem);
        let golden = path.with_extension("dxil-ll");
        if bless {
            fs::write(&golden, &ir).expect("bless 写入失败");
            continue;
        }
        match fs::read_to_string(&golden) {
            Ok(s) if s.replace("\r\n", "\n") == ir => {}
            Ok(s) => mismatches.push(format!(
                "{}: DXIL IR golden 漂移\n--- expected ---\n{}\n--- actual ---\n{ir}",
                golden.display(),
                s.replace("\r\n", "\n")
            )),
            Err(_) => mismatches.push(format!(
                "{}: 缺 .dxil-ll golden(新语料需 RURIX_BLESS=1 + bless_log.md 留痕)",
                golden.display()
            )),
        }
    }
    assert!(
        mismatches.is_empty(),
        "DXIL IR golden 比对失败:\n{}",
        mismatches.join("\n")
    );
}

/// `.dxil-disasm` golden:patched llc → DXIL 容器 → dxc validator **接受** → dxc
/// 反汇编(RFC-0003 §9 Q-Golden)。工具链缺失 → SKIP。
#[test]
fn dxil_disasm_golden_matches_when_toolchain_present() {
    let (Some(llc), Some(dxc_dir)) = (
        rurixc::toolchain::locate_llc(),
        rurixc::toolchain::locate_dxc_dir(),
    ) else {
        eprintln!("dxil_disasm_golden: patched llc / dxc validator 不可用 → SKIP(RXS-0157)");
        return;
    };
    let bless = bless_mode();
    let tmp = std::env::temp_dir().join(format!("rxdxilgold_{}", std::process::id()));
    fs::create_dir_all(&tmp).expect("临时目录");
    let mut mismatches = Vec::new();
    for path in rx_files() {
        let src = fs::read_to_string(&path)
            .expect("读取语料失败")
            .replace("\r\n", "\n");
        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
        let ir = dxil_ir(&src, &stem);
        let obj = tmp.join(format!("{stem}.dxc"));
        rurixc::toolchain::llc_emit_dxil(&llc, &ir, &obj).expect("patched llc emit DXIL 失败");
        // strict-only:入 golden 前 validator 必须接受(不合规 DXIL 不得入 golden)。
        assert!(
            rurixc::toolchain::dxv_validate(&dxc_dir, &obj).expect("dxv 调用失败"),
            "{}: DXIL 容器未通过 dxc validator(不得入 golden)",
            stem
        );
        let disasm = rurixc::toolchain::dxc_disasm(&dxc_dir, &obj).expect("dxc 反汇编失败");
        let golden = path.with_extension("dxil-disasm");
        if bless {
            fs::write(&golden, &disasm).expect("bless 写入失败");
            continue;
        }
        match fs::read_to_string(&golden) {
            Ok(s) if s.replace("\r\n", "\n") == disasm => {}
            Ok(s) => mismatches.push(format!(
                "{}: DXIL 反汇编 golden 漂移\n--- expected ---\n{}\n--- actual ---\n{disasm}",
                golden.display(),
                s.replace("\r\n", "\n")
            )),
            Err(_) => mismatches.push(format!(
                "{}: 缺 .dxil-disasm golden(RURIX_BLESS=1 + bless_log.md 留痕)",
                golden.display()
            )),
        }
    }
    let _ = fs::remove_dir_all(&tmp);
    assert!(
        mismatches.is_empty(),
        "DXIL 反汇编 golden 比对失败:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn dxil_corpus_carries_spec_anchor() {
    for path in rx_files() {
        let src = fs::read_to_string(&path).expect("读取语料失败");
        assert!(
            src.lines()
                .next()
                .unwrap_or("")
                .starts_with("//@ spec: RXS-"),
            "{} 缺条款锚定头",
            path.display()
        );
    }
}
