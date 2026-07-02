//! conformance/dxil DXIL 第二后端语料批跑(G2.2 PR-C2 分片1,RFC-0003;cargo
//! feature `dxil-backend`)。RXS-0157:codegen target 分发与 DXIL 后端分叉——
//! accept(合法最小 compute kernel 经 DXIL 后端产 DirectX 三元组 LLVM IR,0 诊断)+
//! reject(子集外构造 / target 不支持 → RX6007,strict-only 无 fallback)。
//!
//! 管线:resolve → typeck → 着色/barrier → 穷尽性 → const eval → `dxil_codegen::
//! build_and_emit_dxil`(device MIR kernel 根 → DXIL IR)。纯 host/CPU-only(本测试
//! 仅到 IR emit;patched llc → DXIL 容器 → dxc validator 真跑由 `dxil_golden` /
//! `rx build --target dxil` 工具链关卡覆盖)。reject 体例:文件次行
//! `//@ expect-error: RX####`。
#![cfg(feature = "dxil-backend")]

use std::fs;
use std::path::{Path, PathBuf};

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

fn dxil_dir(sub: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/dxil")
        .join(sub)
}

fn rx_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if !root.is_dir() {
        return out;
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap_or_else(|e| panic!("读取 {} 失败: {e}", d.display())) {
            let p = e.expect("读取目录项失败").path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rx") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// resolve → typeck → 着色 → 穷尽性 → const eval → DXIL codegen(阶段化:前段有错
/// 即停)。返回 (DXIL IR 文本 Option, 错误码序列)。
fn run_dxil(src: &str, module: &str) -> (Option<String>, Vec<u16>) {
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    if !diag.has_errors() {
        cx.check_coloring();
    }
    if !diag.has_errors() {
        cx.check_crate_patterns();
    }
    if !diag.has_errors() {
        cx.check_consteval();
    }
    let ir = if !diag.has_errors() {
        rurixc::dxil_codegen::build_and_emit_dxil(&cx, module)
    } else {
        None
    };
    let codes = diag
        .emitted()
        .iter()
        .filter_map(|d| d.code.map(|c| c.0))
        .collect();
    (ir, codes)
}

fn spirv_opcodes(module: &[u32]) -> Vec<u16> {
    let mut out = Vec::new();
    let mut i = 5;
    while i < module.len() {
        let word = module[i];
        let wc = (word >> 16) as usize;
        if wc == 0 || i + wc > module.len() {
            break;
        }
        out.push((word & 0xffff) as u16);
        i += wc;
    }
    out
}

/// accept 正例:0 诊断 + 产出 DirectX 三元组 DXIL IR(compute shader 形态)。
#[test]
fn accept_corpus_emits_dxil() {
    let files = rx_files(&dxil_dir("accept"));
    assert!(!files.is_empty(), "conformance/dxil/accept 正例集为空");
    for f in files {
        let src = fs::read_to_string(&f).expect("读取样例失败");
        let stem = f.file_stem().unwrap().to_string_lossy().into_owned();
        let (ir, codes) = run_dxil(&src, &stem);
        assert!(
            codes.is_empty(),
            "{} 产生诊断: {codes:?}(accept 须 0 诊断)",
            f.display()
        );
        let ir = ir.unwrap_or_else(|| panic!("{} 未产出 DXIL IR", f.display()));
        assert!(
            ir.contains("target triple = \"dxil-unknown-shadermodel6.0-compute\""),
            "{} DXIL IR 缺 DirectX 三元组",
            f.display()
        );
        assert!(
            ir.contains("\"hlsl.shader\"=\"compute\""),
            "{} DXIL IR 缺 hlsl.shader=compute 入口属性",
            f.display()
        );
        // RD-013 slice 1 hardening(全 accept 通用):
        // 1) SSA 定义名(`  %name = ...`)全 body 唯一 —— 重复 store 同一 index 不得撞名。
        let mut ssa_defs = std::collections::HashSet::new();
        for line in ir.lines() {
            let Some(rest) = line.strip_prefix("  %") else {
                continue;
            };
            let Some((name, _)) = rest.split_once(" = ") else {
                continue;
            };
            assert!(
                ssa_defs.insert(name.to_owned()),
                "{} IR 出现 duplicate SSA 定义名 %{name}",
                f.display()
            );
        }
        // 1b) 顶格 label 行(`name:`)全 body 唯一 —— if lowering 的 br 目标不得撞名。
        let mut labels = std::collections::HashSet::new();
        for line in ir.lines() {
            let Some(label) = line.strip_suffix(':') else {
                continue;
            };
            if label.is_empty() || label.contains(' ') {
                continue;
            }
            assert!(
                labels.insert(label.to_owned()),
                "{} IR 出现 duplicate label {label}:",
                f.display()
            );
        }
        // 2) 资源 global 恒 [1 x float] → 所有 inbounds GEP 只允许索引 0(无越界访问)。
        for line in ir.lines() {
            if line.contains("getelementptr inbounds [1 x float]") {
                assert!(
                    line.trim_end().ends_with("i32 0, i32 0"),
                    "{} IR 对 [1 x float] 产生非 0 索引 inbounds GEP: {line}",
                    f.display()
                );
            }
        }
        if stem == "two_stores" {
            assert_eq!(
                ir.matches("store float").count(),
                2,
                "{} 应恰含 2 个 store(重复 store 同一 index)",
                f.display()
            );
        }
        if stem == "copy_arith" {
            for needle in ["load float", "fmul float", "fadd float", "store float"] {
                assert!(
                    ir.contains(needle),
                    "{} DXIL IR 缺 slice 1 {needle} 证据",
                    f.display()
                );
            }
        }
        if stem == "copy_one" {
            assert!(
                ir.contains("load float"),
                "{} DXIL IR 缺 slice 1 load 证据",
                f.display()
            );
            assert!(
                ir.contains("store float"),
                "{} DXIL IR 缺 slice 1 store 证据",
                f.display()
            );
            let load_pos = ir.find("load float").expect("load 已断言存在");
            let store_pos = ir.find("store float").expect("store 已断言存在");
            let ret_pos = ir.find("ret void").expect("compute 入口应含 ret void");
            assert!(
                load_pos < ret_pos && store_pos < ret_pos,
                "{} slice 1 body lowering 必须出现在 ret void 之前",
                f.display()
            );
        }
        if stem == "scalar_gain" {
            for needle in [
                "@rx_gain_root_constant",
                "load float",
                "fmul float",
                "store float",
            ] {
                assert!(
                    ir.contains(needle),
                    "{} DXIL IR 缺 segment 3a scalar gain 证据 {needle}",
                    f.display()
                );
            }
        }
        if stem == "scalar_select" {
            for needle in [
                "@rx_width_root_constant",
                "icmp sgt i64",
                "select i1",
                "store float",
            ] {
                assert!(
                    ir.contains(needle),
                    "{} DXIL IR 缺 segment 3a select 证据 {needle}",
                    f.display()
                );
            }
            let select_pos = ir.find("select i1").expect("select 已断言存在");
            let ret_pos = ir.find("ret void").expect("compute 入口应含 ret void");
            assert!(
                select_pos < ret_pos,
                "{} segment 3a select lowering 必须出现在 ret void 之前",
                f.display()
            );
        }
        if stem == "if_statement_store" {
            for needle in [
                "br i1 ",
                "if.then.0:",
                "br label %if.end.0",
                "if.end.0:",
                "store float",
            ] {
                assert!(
                    ir.contains(needle),
                    "{} DXIL IR 缺 segment 3a statement if 证据 {needle}",
                    f.display()
                );
            }
            let br_pos = ir.find("br i1 ").expect("br 已断言存在");
            let then_pos = ir.find("if.then.0:").expect("then label 已断言存在");
            let store_pos = ir.find("store float").expect("store 已断言存在");
            let end_pos = ir.find("if.end.0:").expect("end label 已断言存在");
            assert!(
                br_pos < then_pos && then_pos < store_pos && store_pos < end_pos,
                "{} statement if 结构次序应为 br i1 < if.then.0 < store < if.end.0",
                f.display()
            );
        }
        if stem == "if_then_more_stores" {
            assert_eq!(
                ir.matches("store float").count(),
                2,
                "{} 应恰含 2 个 store(then 块内 1 个 + if.end 之后 1 个)",
                f.display()
            );
            let end_pos = ir.find("if.end.0:").expect("end label 应存在");
            let last_store = ir.rfind("store float").expect("store 已断言存在");
            assert!(
                end_pos < last_store,
                "{} 语句位 if 之后的 store 应落在 if.end.0 块内",
                f.display()
            );
        }
        if stem == "threadctx_global_id" {
            for needle in [
                "declare i32 @rx.dxil.thread_id.x()",
                "call i32 @rx.dxil.thread_id.x()",
                "zext i32",
                "icmp slt i64",
                "select i1",
                "store float",
            ] {
                assert!(
                    ir.contains(needle),
                    "{} DXIL IR 缺 segment 3a ThreadCtx.global_id 证据 {needle}",
                    f.display()
                );
            }
            let thread_id_pos = ir
                .find("call i32 @rx.dxil.thread_id.x()")
                .expect("thread id lowering 已断言存在");
            let ret_pos = ir.find("ret void").expect("compute 入口应含 ret void");
            assert!(
                thread_id_pos < ret_pos,
                "{} segment 3a ThreadCtx.global_id lowering 必须出现在 ret void 之前",
                f.display()
            );
        }
    }
}

/// reject 反例:全拦截到 `//@ expect-error` 声明的码(RX6007 全覆盖)。
#[test]
fn reject_corpus_all_intercepted() {
    let files = rx_files(&dxil_dir("reject"));
    assert!(!files.is_empty(), "conformance/dxil/reject 反例集为空");
    for f in files {
        let src = fs::read_to_string(&f).expect("读取样例失败");
        let expected: u16 = src
            .lines()
            .find_map(|l| l.trim().strip_prefix("//@ expect-error: RX"))
            .unwrap_or_else(|| panic!("{} 缺 //@ expect-error: RX#### 头", f.display()))
            .trim()
            .parse()
            .expect("expect-error 码格式非法");
        let stem = f.file_stem().unwrap().to_string_lossy().into_owned();
        let (ir, codes) = run_dxil(&src, &stem);
        assert!(ir.is_none(), "{} 不应产出 DXIL IR(reject)", f.display());
        assert!(
            codes.contains(&expected),
            "{} 未拦截到 RX{expected}: {codes:?}",
            f.display()
        );
    }
}

#[test]
fn corpus_files_carry_spec_anchor() {
    for f in rx_files(&dxil_dir("")) {
        let src = fs::read_to_string(&f).expect("读取样例失败");
        let first = src.lines().next().unwrap_or("");
        assert!(
            first.starts_with("//@ spec: RXS-"),
            "{} 缺条款锚定头(//@ spec: RXS-####)",
            f.display()
        );
    }
}

// ═══════════════════ 图形=B 语料(G2.2 PR-D2,RXS-0158/0159/0161/0162) ═══════════════════
//
// 图形阶段(vertex/fragment)语料置于 `conformance/dxil/graphics/{accept,reject}/`,与
// 上方 compute A 路语料(`accept`/`reject` 顶层)互不干扰(各自 `rx_files` 根不相交)。
// B 路真链(spirv-cross→dxc→DXIL)依赖外部工具且**环境相关**(trivial passthrough 被
// DCE → 校验门如期拒),故本语料只断言 **host 侧确定性面**(阶段分类 + io_sig 携带 +
// `dxil_spirv::emit_spirv` 合法 SPIR-V + 确定性 / 不可映射 strict-only 6xxx),不在此跑
// 真链;B 全链确定性 ×N + validator gate + 签名校验门红绿见 `ci/dxil_codegen_smoke.py`
// (CI 步骤 46)+ `dxil_sig_gate` / `dxil_spirv` 单测 + owner pin 环境 device 真跑。
//
// 锚定(`//@ spec`)恒被 `corpus_files_carry_spec_anchor`(递归 `dxil_dir("")`)覆盖。

/// 合法图形语料:前段 0 诊断 + 收图形阶段根(io_sig 非空)+ `emit_spirv` 产合法且
/// 确定性的 SPIR-V 字流(RXS-0158 阶段分类 / RXS-0161 降级面 / RXS-0162 确定性 host 面)。
#[cfg(feature = "shader-stages")]
#[test]
fn accept_graphics_corpus_lowers_to_spirv() {
    use rurixc::ast::ShaderStage;
    let files = rx_files(&dxil_dir("graphics/accept"));
    assert!(
        !files.is_empty(),
        "conformance/dxil/graphics/accept 正例集为空"
    );
    for f in files {
        let src = fs::read_to_string(&f).expect("读取样例失败");
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(&src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        cx.check_coloring();
        cx.check_crate_patterns();
        cx.check_consteval();
        let bodies = cx.device_mir_crate();
        let codes: Vec<u16> = diag
            .emitted()
            .iter()
            .filter_map(|d| d.code.map(|c| c.0))
            .collect();
        assert!(
            codes.is_empty(),
            "{} 产生诊断: {codes:?}(graphics accept 须 0 诊断)",
            f.display()
        );
        let gfx: Vec<_> = bodies
            .iter()
            .filter(|b| matches!(b.stage, Some(ShaderStage::Vertex | ShaderStage::Fragment)))
            .collect();
        assert!(
            !gfx.is_empty(),
            "{} 应收 ≥1 vertex/fragment 图形阶段根",
            f.display()
        );
        for b in gfx {
            assert!(
                !b.io_sig.is_empty(),
                "{} 图形阶段根 io_sig 应非空(RXS-0161 收集根携意图签名)",
                f.display()
            );
            let stage = b.stage.expect("图形根 stage");
            let spv = rurixc::dxil_spirv::emit_spirv(stage, &b.io_sig, &b.resources)
                .unwrap_or_else(|e| {
                    panic!("{} emit_spirv 应 Ok(已建模子集), 实得 {e:?}", f.display())
                });
            assert_eq!(
                spv.first().copied(),
                Some(0x0723_0203),
                "{} SPIR-V magic 应为 0x07230203",
                f.display()
            );
            // RXS-0162 确定性(host 面):同 io_sig ×N emit_spirv 字节全等(Property 3 的
            // host 可达面;B 全链容器 SHA256 确定性见步骤 46)。
            let spv2 =
                rurixc::dxil_spirv::emit_spirv(stage, &b.io_sig, &b.resources).expect("二次 emit");
            assert_eq!(
                spv,
                spv2,
                "{} emit_spirv 非确定性(同输入字节漂移)",
                f.display()
            );
        }
    }
}

/// RXS-0171:图形 body I/O 数据流最小切片。RXS-0171 专用 accept 语料必须经
/// body-aware `emit_spirv_body` 产出真实 `OpLoad` / `OpStore` / 白名单算术,
/// 不再只停在签名-only void main。
#[cfg(feature = "shader-stages")]
#[test]
fn accept_graphics_body_corpus_lowers_io_dataflow() {
    use rurixc::ast::ShaderStage;
    const OP_LOAD: u16 = 61;
    const OP_STORE: u16 = 62;
    const OP_FADD: u16 = 129;

    let files: Vec<PathBuf> = rx_files(&dxil_dir("graphics/accept"))
        .into_iter()
        .filter(|f| {
            fs::read_to_string(f)
                .map(|src| src.lines().next().unwrap_or("").contains("RXS-0171"))
                .unwrap_or(false)
        })
        .collect();
    assert!(
        !files.is_empty(),
        "graphics/accept 应含 RXS-0171 body dataflow 语料"
    );

    for f in files {
        let src = fs::read_to_string(&f).expect("读取样例失败");
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(&src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        cx.check_coloring();
        cx.check_crate_patterns();
        cx.check_consteval();
        assert!(
            !diag.has_errors(),
            "{} RXS-0171 graphics accept 须 0 前段诊断",
            f.display()
        );
        let bodies = cx.device_mir_crate();
        let gfx: Vec<_> = bodies
            .iter()
            .filter(|b| matches!(b.stage, Some(ShaderStage::Vertex | ShaderStage::Fragment)))
            .collect();
        assert!(!gfx.is_empty(), "{} 应收图形阶段根", f.display());
        for b in gfx {
            let stage = b.stage.expect("图形根 stage");
            let spv = rurixc::dxil_spirv::emit_spirv_body(stage, b)
                .unwrap_or_else(|e| panic!("{} emit_spirv_body 应 Ok, 实得 {e:?}", f.display()));
            let ops = spirv_opcodes(&spv);
            assert!(
                ops.contains(&OP_STORE),
                "{} body 应写出 OpStore",
                f.display()
            );
            if b.io_sig
                .iter()
                .any(|e| matches!(e.dir, rurixc::mir::IoDir::In))
            {
                assert!(ops.contains(&OP_LOAD), "{} body 应读取 OpLoad", f.display());
            }
            if f.file_stem().and_then(|s| s.to_str()) == Some("fs_body_arith") {
                assert!(
                    ops.contains(&OP_FADD),
                    "{} body 应含 f32 OpFAdd",
                    f.display()
                );
            }
        }
    }
}

/// RXS-0160:vertex+fragment 配对的图形 accept 语料经多阶段联编点链接核对 → `Linked`。
/// 对 graphics/accept 中同时含 vertex+fragment 阶段根的文件(如 `vs_fs_link.rx`)断言
/// [`link_graphics_stages`] 链接一致(host 侧确定性;builtin 不参与、location 不比对
/// ABI 中立)。单阶段文件 → `NoPair`(无配对,不断言)。错链错误码 = `RX6014`
/// `codegen.dxil_stage_link_mismatch`(agent 裁定方案 B 新开码,G2.3 PR-E2b-2,
/// spec §2 RXS-0160 IR3),accept 路径不涉错误码。
#[cfg(feature = "shader-stages")]
#[test]
fn accept_graphics_link_consistent() {
    use rurixc::ast::ShaderStage;
    use rurixc::dxil_codegen::{StageLinkOutcome, link_graphics_stages};
    let files = rx_files(&dxil_dir("graphics/accept"));
    let mut linked_any = false;
    for f in files {
        let src = fs::read_to_string(&f).expect("读取样例失败");
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(&src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        cx.check_coloring();
        cx.check_crate_patterns();
        cx.check_consteval();
        assert!(
            !diag.has_errors(),
            "{} graphics accept 须 0 诊断",
            f.display()
        );
        let bodies = cx.device_mir_crate();
        let has_vs = bodies
            .iter()
            .any(|b| matches!(b.stage, Some(ShaderStage::Vertex)));
        let has_fs = bodies
            .iter()
            .any(|b| matches!(b.stage, Some(ShaderStage::Fragment)));
        if has_vs && has_fs {
            assert_eq!(
                link_graphics_stages(&bodies),
                StageLinkOutcome::Linked,
                "{} vertex+fragment 配对应链接一致(RXS-0160)",
                f.display()
            );
            linked_any = true;
        }
    }
    assert!(
        linked_any,
        "graphics/accept 应含 ≥1 vertex+fragment 链接一致配对样例(RXS-0160)"
    );
}

/// 反例图形语料:前段 0 诊断后,B 路分发对不可映射构造 strict-only 落/// `//@ expect-error` 声明的 6xxx(host 确定性:编码器在映射处停手,工具链不可达),
/// 绝不产物。
#[cfg(feature = "shader-stages")]
#[test]
fn reject_graphics_corpus_intercepted() {
    use rurixc::dxil_codegen::{DispatchOutcome, dispatch_and_emit};
    let files = rx_files(&dxil_dir("graphics/reject"));
    assert!(
        !files.is_empty(),
        "conformance/dxil/graphics/reject 反例集为空"
    );
    for f in files {
        let src = fs::read_to_string(&f).expect("读取样例失败");
        let expected: u16 = src
            .lines()
            .find_map(|l| l.trim().strip_prefix("//@ expect-error: RX"))
            .unwrap_or_else(|| panic!("{} 缺 //@ expect-error: RX#### 头", f.display()))
            .trim()
            .parse()
            .expect("expect-error 码格式非法");
        let diag = DiagCtxt::new();
        let cx = QueryCtx::new(&src, SourceId(0), Edition::Rx0, &diag);
        cx.check_crate();
        cx.check_coloring();
        cx.check_crate_patterns();
        cx.check_consteval();
        assert!(
            !diag.has_errors(),
            "{} 前段应先 0 诊断(reject 须来自 B 路降级 strict-only,非前段)",
            f.display()
        );
        let bodies = cx.device_mir_crate();
        let mut produced = false;
        for b in &bodies {
            match dispatch_and_emit(&diag, b, "gfx") {
                DispatchOutcome::PathAIr(_) | DispatchOutcome::PathBSignatures { .. } => {
                    produced = true;
                }
                DispatchOutcome::SkippedB(_) | DispatchOutcome::Diagnosed => {}
            }
        }
        assert!(!produced, "{} reject 不应产出 DXIL 产物", f.display());
        let codes: Vec<u16> = diag
            .emitted()
            .iter()
            .filter_map(|d| d.code.map(|c| c.0))
            .collect();
        assert!(
            codes.contains(&expected),
            "{} 未拦截到 RX{expected}: {codes:?}",
            f.display()
        );
    }
}
