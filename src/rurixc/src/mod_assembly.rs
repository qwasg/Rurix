//! out-of-line 模块装配 pass(RXS-0196,RFC-0009 §4.2;MS1.2)。
//!
//! parse 后、resolve 前:对 AST 中 `mod name;`(out-of-line 形态)按「当前文件
//! 同目录 `name.rx`」加载——注册进 [`SourceMap`](多文件已支持)、递归 lex+parse
//! 后 splice 为内联 mod 等价形态(resolve/typeck 零改动)。文件缺失 / IO 失败 /
//! 循环引用 → **RX1005**(1xxx 名称/模块段位)。
//!
//! 范围红线(RFC-0009 §8):嵌套子目录(`mod a/b;` 形态)不做;被加载文件内的
//! `mod name;` 允许(同目录递归);循环检测用装配栈(canonicalize 路径比对)。
//! 被加载文件的内部属性(`#![...]`)不装配(仅取 items,MVP 取舍)。

use std::path::{Path, PathBuf};

use crate::ast;
use crate::diag::{DiagCtxt, ErrorCode};
use crate::lexer::lex;
use crate::parser::parse;
use crate::source_map::SourceMap;
use crate::span::{Edition, SourceId};

pub const E_MODULE_LOAD: ErrorCode = ErrorCode(1005); // RX1005

/// 装配入口:`root_path` = 根编译单元文件路径(模块文件按其同目录定位)。
///
/// 返回加载的 `(SourceId, 源文本)` 列表,供 [`crate::query::QueryCtx::add_module_src`]
/// 注册(多文件 span 切片:字面量取值等按 span.file 归属正确源文本)。
//@ spec: RXS-0196
pub fn assemble_out_of_line_mods(
    root: &mut ast::SourceFile,
    root_path: &Path,
    sm: &mut SourceMap,
    edition: Edition,
    diag: &DiagCtxt,
) -> Vec<(SourceId, String)> {
    let dir = root_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    // 装配栈以根文件为栈底:`mod main;` 自引用同样判为循环(RX1005)
    let mut stack: Vec<PathBuf> = Vec::new();
    if let Ok(canon) = root_path.canonicalize() {
        stack.push(canon);
    }
    let mut loaded = Vec::new();
    assemble_items(
        &mut root.items,
        &dir,
        sm,
        edition,
        diag,
        &mut stack,
        &mut loaded,
    );
    loaded
}

/// 递归遍历 items:out-of-line mod 装配;内联 mod 继续下钻(其中的 `mod name;`
/// 仍按同目录定位——本期不做子目录映射,RFC-0009 §8)。
fn assemble_items(
    items: &mut [ast::Item],
    dir: &Path,
    sm: &mut SourceMap,
    edition: Edition,
    diag: &DiagCtxt,
    stack: &mut Vec<PathBuf>,
    loaded: &mut Vec<(SourceId, String)>,
) {
    for item in items.iter_mut() {
        if let ast::ItemKind::Mod(m) = &mut item.kind {
            if m.out_of_line {
                load_module(m, dir, sm, edition, diag, stack, loaded);
            } else {
                assemble_items(&mut m.items, dir, sm, edition, diag, stack, loaded);
            }
        }
    }
}

/// 加载单个 out-of-line 模块文件并 splice(失败发 RX1005 后保持 items 为空,
/// 阶段化中止由 driver 的 has_errors 关卡承接)。
fn load_module(
    m: &mut ast::ModItem,
    dir: &Path,
    sm: &mut SourceMap,
    edition: Edition,
    diag: &DiagCtxt,
    stack: &mut Vec<PathBuf>,
    loaded: &mut Vec<(SourceId, String)>,
) {
    let path = dir.join(format!("{}.rx", m.name.name));
    let src = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            diag.struct_error(E_MODULE_LOAD, "resolve.module_load_failure")
                .arg("name", format!("`{}`", m.name.name))
                .arg("detail", format!("cannot read {}: {e}", path.display()))
                .span_label(m.name.span, "cannot load this out-of-line module")
                .emit();
            return;
        }
    };
    // 循环检测(装配栈):canonicalize 失败(理论不可达:读取已成功)退化为原路径
    let canon = path.canonicalize().unwrap_or_else(|_| path.clone());
    if stack.contains(&canon) {
        diag.struct_error(E_MODULE_LOAD, "resolve.module_load_failure")
            .arg("name", format!("`{}`", m.name.name))
            .arg(
                "detail",
                format!("module cycle detected while loading {}", path.display()),
            )
            .span_label(m.name.span, "cyclic module reference")
            .emit();
        return;
    }

    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    let id = sm.add_file(file_name, src.as_str(), edition);
    let tokens = lex(&src, id, edition, diag);
    let sub = parse(&src, tokens, id, edition, diag);

    stack.push(canon);
    let mut sub_items = sub.items;
    assemble_items(&mut sub_items, dir, sm, edition, diag, stack, loaded);
    stack.pop();

    // splice:装配后与内联 mod 无差别(out_of_line 标记保留为出处记录)
    m.items = sub_items;
    loaded.push((id, src));
}
