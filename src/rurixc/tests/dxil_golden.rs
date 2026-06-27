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

// ═══════════════════ 图形=B DXIL golden(G2.2 PR-D2,RXS-0162) ═══════════════════
//
// B 路 golden 置于 `tests/dxil/graphics/`(子目录;A 路 `rx_files()` 用 `read_dir`
// **非递归**,自然不收;本组用独立 lister)。形态:DXIL 文本反汇编(`.dxil-disasm`),
// 经 B 全链(dxil_spirv::emit_spirv→SPIRV-Cross→dxc→dumpbin)产出。validator gate:
// 若签名 validator 目录(`RURIX_DXC_DIR` 含 dxv.exe)可用则入 golden 前 dxv 验证;本机
// Vulkan SDK dxc **无** dxil.dll/dxv → 结构性 dxc 编译成功即过(NOT BLESSED,owner 在
// pin 环境带签名 validator 重 bless)。版本噪声行(shader hash / dxc ident)规范化,使
// golden 不写死工具版本布局为语言保证(硬约束;RXS-0162 IR5)。spirv-cross/dxc 缺失 →
// SKIP(开发环境降级,exit 0,对齐 RXS-0073)。`RURIX_BLESS=1` 重写 + bless_log 留痕。

#[cfg(feature = "shader-stages")]
fn graphics_rx_files() -> Vec<PathBuf> {
    let dir = dxil_dir().join("graphics");
    let mut out: Vec<PathBuf> = match fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "rx"))
            .collect(),
        Err(_) => Vec::new(),
    };
    out.sort();
    out
}

/// 规范化 dxc 反汇编中的版本噪声行(shader hash 内容/版本派生 + dxc ident 构建串),
/// 使 golden 聚焦语言相关结构(签名表 / 入口 / 着色器类型),不写死工具版本。
#[cfg(feature = "shader-stages")]
fn normalize_b_disasm(s: &str) -> String {
    let mut lines = Vec::new();
    for raw in s.replace("\r\n", "\n").lines() {
        let t = raw.trim_start();
        if t.starts_with("; shader hash:") {
            lines.push("; shader hash: <NOT-BLESSED-NORMALIZED>".to_owned());
        } else if raw.contains("dxc(private)") {
            // 保留 metadata id 前缀(如 `!0 = `),仅规范化版本串。
            let id = raw.split('=').next().unwrap_or("").trim_end();
            lines.push(format!(
                "{id} = !{{!\"dxc(private) <NOT-BLESSED-NORMALIZED>\"}}"
            ));
        } else {
            lines.push(raw.to_owned());
        }
    }
    lines.join("\n")
}

/// 从图形 golden 语料源码取首个 vertex/fragment 阶段根的 (stage, io_sig)。
#[cfg(feature = "shader-stages")]
fn graphics_stage_io(src: &str) -> Option<(rurixc::ast::ShaderStage, Vec<rurixc::mir::IoSigElem>)> {
    use rurixc::ast::ShaderStage;
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    cx.check_coloring();
    cx.check_crate_patterns();
    cx.check_consteval();
    assert!(!diag.has_errors(), "DXIL B golden 语料须 0 诊断");
    let bodies = cx.device_mir_crate();
    bodies
        .into_iter()
        .find(|b| matches!(b.stage, Some(ShaderStage::Vertex | ShaderStage::Fragment)))
        .map(|b| (b.stage.expect("图形 stage"), b.io_sig))
}

/// `.dxil-disasm` golden(B 路):emit_spirv → SPIRV-Cross → dxc → dumpbin,validator
/// gate(可用时)→ 规范化反汇编 → golden 比对。工具缺失 → SKIP。
#[cfg(feature = "shader-stages")]
#[test]
fn dxil_b_disasm_golden_matches_when_toolchain_present() {
    // 版本相关 NOT-BLESSED golden:仅在**显式配置**的 pin 工具(env 指向真实文件)下
    // 跑字节比对——`locate_*` 的 PATH by-name 回落(spawn 决定)不触发,避免随机 PATH
    // 工具产不同反汇编致误红。env 未设 → SKIP(对齐 A 路 .dxil-disasm 经 RURIX_DXC_DIR
    // 显式门控的纪律;真实红绿在带 pin B 工具链的 dev/owner 环境)。
    let (Some(spvx), Some(dxc)) = (
        rurixc::toolchain::locate_spirv_cross().filter(|p| p.is_file()),
        rurixc::toolchain::locate_dxc().filter(|p| p.is_file()),
    ) else {
        eprintln!(
            "dxil_b_disasm_golden: 未显式配置 pin B 工具链(RURIX_SPIRV_CROSS / RURIX_DXC \
             指向真实文件)→ SKIP(开发环境降级,RXS-0162;真实红绿在带 pin B 工具链环境)"
        );
        return;
    };
    let bless = bless_mode();
    let header = concat!(
        "; NOT BLESSED (local) — RXS-0162 图形=B DXIL 反汇编 golden。\n",
        "; 本机 dxc(Vulkan SDK)无签名 validator(dxil.dll/dxv),owner 在 pin 环境重 bless;\n",
        "; 版本噪声行(shader hash / dxc ident)已规范化为占位,不写死工具版本布局为语言保证。\n",
        "; 平凡 passthrough(RD-013 入口 body 数据流降级 deferred)→ spirv-cross DCE → 签名退化。\n",
    );
    let tmp = std::env::temp_dir().join(format!("rxdxilbgold_{}", std::process::id()));
    fs::create_dir_all(&tmp).expect("临时目录");
    let mut mismatches = Vec::new();
    for path in graphics_rx_files() {
        let src = fs::read_to_string(&path)
            .expect("读取语料失败")
            .replace("\r\n", "\n");
        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
        let Some((stage, io_sig)) = graphics_stage_io(&src) else {
            mismatches.push(format!(
                "{}: 未收到 vertex/fragment 图形阶段根",
                path.display()
            ));
            continue;
        };
        // 1) MIR→SPIR-V(Rurix 自有降级)。
        let spv = rurixc::dxil_spirv::emit_spirv(stage, &io_sig)
            .unwrap_or_else(|e| panic!("{stem}: emit_spirv 应 Ok, 实得 {e:?}"));
        let mut bytes = Vec::with_capacity(spv.len() * 4);
        for w in &spv {
            bytes.extend_from_slice(&w.to_le_bytes());
        }
        let spv_path = tmp.join(format!("{stem}.spv"));
        fs::write(&spv_path, &bytes).expect("写 .spv");
        // 2) SPIRV-Cross → HLSL。
        let hlsl_path = tmp.join(format!("{stem}.hlsl"));
        rurixc::toolchain::spirv_cross_to_hlsl(&spvx, &spv_path, &hlsl_path, 60, &[])
            .unwrap_or_else(|e| panic!("{stem}: spirv-cross 失败: {e}"));
        // 3) dxc → DXIL 容器(vertex=vs_6_0)。
        let profile = match stage {
            rurixc::ast::ShaderStage::Vertex => "vs_6_0",
            rurixc::ast::ShaderStage::Fragment => "ps_6_0",
            other => panic!("{stem}: 非图形阶段 {other:?}"),
        };
        let dxil_path = tmp.join(format!("{stem}.dxil"));
        rurixc::toolchain::dxc_hlsl_to_dxil(&dxc, &hlsl_path, profile, "main", &dxil_path)
            .unwrap_or_else(|e| panic!("{stem}: dxc HLSL→DXIL 失败: {e}"));
        // 4) validator gate(签名 validator 可用时;入 golden 前必须接受)。
        if let Some(dxc_dir) = rurixc::toolchain::locate_dxc_dir()
            && dxc_dir.join("dxv.exe").is_file()
        {
            assert!(
                rurixc::toolchain::dxv_validate(&dxc_dir, &dxil_path).expect("dxv 调用失败"),
                "{stem}: DXIL 容器未通过签名 validator(不得入 golden)"
            );
        }
        // 5) dumpbin 反汇编 → 规范化 + NOT BLESSED 头。
        let dxc_dir = dxc.parent().map(Path::to_path_buf).unwrap_or_default();
        let disasm = rurixc::toolchain::dxc_disasm(&dxc_dir, &dxil_path)
            .unwrap_or_else(|e| panic!("{stem}: dxc 反汇编失败: {e}"));
        let produced = format!("{header}{}\n", normalize_b_disasm(&disasm));
        let golden = path.with_extension("dxil-disasm");
        if bless {
            fs::write(&golden, &produced).expect("bless 写入失败");
            continue;
        }
        match fs::read_to_string(&golden) {
            Ok(s) if s.replace("\r\n", "\n") == produced => {}
            Ok(s) => mismatches.push(format!(
                "{}: B DXIL 反汇编 golden 漂移\n--- expected ---\n{}\n--- actual ---\n{produced}",
                golden.display(),
                s.replace("\r\n", "\n")
            )),
            Err(_) => mismatches.push(format!(
                "{}: 缺 .dxil-disasm golden(新语料需 RURIX_BLESS=1 + bless_log.md 留痕)",
                golden.display()
            )),
        }
    }
    let _ = fs::remove_dir_all(&tmp);
    assert!(
        mismatches.is_empty(),
        "B DXIL 反汇编 golden 比对失败:\n{}",
        mismatches.join("\n")
    );
}
