#![cfg(feature = "vulkan-backend")]
//! G37 W1 回归:「if 包 while」结构化控制流 codegen(day_0828 A1 相登记缺陷,
//! `artifacts/day_0828/e_final/HANDOVER.md` §A.3)。
//!
//! 缺陷根因:`vulkan_codegen::structured_merge` 求 if 的 selection merge 用
//! 「两臂最近共同前向可达块」,但遍历不裁剪循环回边——if 落在循环体内时 CFG
//! 有环,else 臂经「join→latch→循环头→then 臂」绕整圈把 then 臂内块(含 then
//! 目标本身)算进共同可达集;臂内 while 之后再有语句时真 join 距离被拉长,绕环
//! 假候选以更小 max 距离胜出 → `OpSelectionMerge` 指向臂内块 → spirv-val 拒
//! (`block <ID> branches to the selection construct, but not to the selection
//! header <ID>`)。修 = 可达性遍历排除已识别循环的回边(latch→header)。
//!
//! 语料四形态(conformance/vulkan/accept/):if 包 while / if-else 双臂各含
//! while / 嵌套两层 if 包 while / while 包 if 包 (while+后随 if)——末者为缺陷
//! **字面触发形态**(前三者修复前也绿,回归价值在末者;全收为组合面覆盖)。
//!
//! 两腿:①结构不变量(恒跑,无外部工具):`OpSelectionMerge` 的 merge 目标不得
//! 等于紧随 `OpBranchConditional` 的任一分支目标(修复前对触发形态 merge == then
//! 目标);`OpLoopMerge` 的 merge ≠ continue。②spirv-val 严格校验(缺工具 SKIP,
//! dev-env degrade 非 fake pass;镜像 compute_w1_vulkan_spirv_val 口径)。

use std::path::{Path, PathBuf};
use std::process::Command;

use rurixc::diag::DiagCtxt;
use rurixc::query::QueryCtx;
use rurixc::span::{Edition, SourceId};

/// 回归语料 stem(conformance/vulkan/accept/<stem>.rx)。
const SHAPES: [&str; 4] = [
    "vk_if_while",
    "vk_if_else_both_while",
    "vk_if_if_while",
    "vk_loop_if_while_if", // day_0828 A1 缺陷字面触发形态(修复前 spirv-val 拒)
];

const OP_LOOP_MERGE: u16 = 246;
const OP_SELECTION_MERGE: u16 = 247;
const OP_BRANCH_CONDITIONAL: u16 = 250;

fn accept_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/vulkan/accept")
}

fn emit(stem: &str) -> Vec<u32> {
    let path = accept_dir().join(format!("{stem}.rx"));
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("读取 {} 失败:{e}", path.display()))
        .replace("\r\n", "\n");
    let diag = DiagCtxt::new();
    let cx = QueryCtx::new(&src, SourceId(0), Edition::Rx0, &diag);
    cx.check_crate();
    cx.check_coloring();
    cx.check_crate_patterns();
    cx.check_consteval();
    assert!(
        !diag.has_errors(),
        "{stem} 前端应 0 诊断:{:?}",
        diag.emitted()
    );
    let words = rurixc::vulkan_codegen::build_and_emit_vulkan(&cx, stem)
        .unwrap_or_else(|| panic!("{stem} Vulkan codegen 未产出:{:?}", diag.emitted()));
    assert!(
        !diag.has_errors(),
        "{stem} Vulkan codegen 应 0 诊断:{:?}",
        diag.emitted()
    );
    words
}

/// SPIR-V 指令流 → (opcode, 操作数切片起点/长度) 序列(头 5 词跳过)。
fn instructions(words: &[u32]) -> Vec<(u16, &[u32])> {
    let mut out = Vec::new();
    let mut at = 5usize;
    while at < words.len() {
        let first = words[at];
        let len = (first >> 16) as usize;
        assert!(len > 0 && at + len <= words.len(), "SPIR-V 指令长度非法");
        out.push(((first & 0xffff) as u16, &words[at + 1..at + len]));
        at += len;
    }
    out
}

/// 结构不变量(恒跑,无外部工具依赖;修复前对 vk_loop_if_while_if 即红):
/// - 每个 `OpSelectionMerge` 后必紧随 `OpBranchConditional`,且 merge 目标
///   ≠ then 目标 且 ≠ else 目标(缺陷形态 = merge 指向 then 臂内块);
/// - 每个 `OpLoopMerge` 的 merge ≠ continue(合成 continue 块保证);
/// - 四形态均含 while 与 if ⇒ 两种 merge 指令都必须出现。
#[test]
fn if_while_shapes_selection_merge_targets_are_join_blocks() {
    for stem in SHAPES {
        let words = emit(stem);
        let insts = instructions(&words);
        let mut n_sel = 0usize;
        let mut n_loop = 0usize;
        for (i, &(op, operands)) in insts.iter().enumerate() {
            match op {
                OP_SELECTION_MERGE => {
                    n_sel += 1;
                    let merge = operands[0];
                    let Some(&(next_op, next_operands)) = insts.get(i + 1) else {
                        panic!("{stem}: OpSelectionMerge 后指令流截断");
                    };
                    assert_eq!(
                        next_op, OP_BRANCH_CONDITIONAL,
                        "{stem}: OpSelectionMerge 后应紧随 OpBranchConditional"
                    );
                    let (t, e) = (next_operands[1], next_operands[2]);
                    assert!(
                        merge != t && merge != e,
                        "{stem}: selection merge %{merge} 不得等于分支目标 \
                         (then %{t} / else %{e})——「if 包 while」缺陷回归形态"
                    );
                }
                OP_LOOP_MERGE => {
                    n_loop += 1;
                    assert_ne!(
                        operands[0], operands[1],
                        "{stem}: loop merge 与 continue 目标不得复用同一块"
                    );
                }
                _ => {}
            }
        }
        assert!(n_sel > 0, "{stem}: 语料含 if,应有 OpSelectionMerge");
        assert!(n_loop > 0, "{stem}: 语料含 while,应有 OpLoopMerge");
    }
}

/// spirv-val 严格校验腿(缺工具 SKIP,dev-env degrade 非 fake pass;镜像
/// compute_w1_vulkan_spirv_val::compute_w1_passes_spirv_val 口径)。
#[test]
fn if_while_shapes_pass_spirv_val() {
    let Some(tool) = rurixc::toolchain::locate_spirv_val() else {
        eprintln!("[SKIP] spirv-val 定位失败(dev-env degrade,非 fake pass)");
        return;
    };
    for stem in SHAPES {
        let words = emit(stem);
        let bytes = rurixc::vulkan_codegen::words_to_bytes(&words);
        let path = std::env::temp_dir().join(format!(
            "rurix_if_while_{}_{stem}.spv",
            std::process::id()
        ));
        std::fs::write(&path, bytes).expect("写临时 SPIR-V");
        let output = Command::new(&tool)
            .arg("--target-env")
            .arg("vulkan1.0")
            .arg(&path)
            .output();
        let _ = std::fs::remove_file(&path);
        match output {
            Err(_) => {
                eprintln!("[SKIP] spirv-val 不可执行(dev-env degrade)");
                return;
            }
            Ok(out) if out.status.success() => {
                eprintln!("[OK] spirv-val --target-env vulkan1.0 accept:{stem}");
            }
            Ok(out) => panic!(
                "spirv-val 拒绝 {stem}:stdout={} stderr={}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ),
        }
    }
}
