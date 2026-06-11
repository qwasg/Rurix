//! feature gate 骨架(spec 条款 RXS-0031,spec/syntax.md;10 §5)。
//!
//! - gate 经文件顶部内部属性 `#![feature(name, …)]` 启用(RXS-0012/0031);
//! - gate 注册表只追加,gate 名永不复用;稳定化生命周期见 10 §5;
//! - 检查通道在 parse 之后对 AST 整体跑一遍:gated 语法未启用 → `RX0010`,
//!   未知 gate 名 → `RX0011`。
//!
//! 首批 gate:`closures`——闭包表达式(RXS-0026 `closure_expr`)。

use crate::ast::*;
use crate::diag::{DiagCtxt, ErrorCode};

pub const E_GATED_FEATURE: ErrorCode = ErrorCode(10); // RX0010
pub const E_UNKNOWN_FEATURE: ErrorCode = ErrorCode(11); // RX0011

/// gate 注册表(RXS-0031:只追加,名字永不复用)。
pub const KNOWN_GATES: &[&str] = &["closures"];

/// 已启用 gate 集(自 `#![feature(...)]` 收集)。
#[derive(Default, Debug)]
pub struct Features {
    enabled: Vec<String>,
}

impl Features {
    pub fn is_enabled(&self, name: &str) -> bool {
        self.enabled.iter().any(|g| g == name)
    }
}

/// 自源文件内部属性收集 `#![feature(...)]`(RXS-0031);未知 gate 名 → `RX0011`。
pub fn collect_features(file: &SourceFile, diag: &DiagCtxt) -> Features {
    let mut features = Features::default();
    for attr in &file.attrs {
        if !attr.inner {
            continue;
        }
        let [seg] = attr.meta.path.segments.as_slice() else {
            continue;
        };
        if seg.ident.name != "feature" {
            continue;
        }
        let MetaKind::List(inner) = &attr.meta.kind else {
            continue;
        };
        for entry in inner {
            let MetaInner::Meta(meta) = entry else {
                continue;
            };
            let [name_seg] = meta.path.segments.as_slice() else {
                continue;
            };
            let name = &name_seg.ident.name;
            if KNOWN_GATES.contains(&name.as_str()) {
                features.enabled.push(name.clone());
            } else {
                diag.struct_error(E_UNKNOWN_FEATURE, "parse.unknown_feature")
                    .arg("feature", format!("`{name}`"))
                    .span_label(name_seg.ident.span, "unknown feature")
                    .emit();
            }
        }
    }
    features
}

/// gate 检查通道(RXS-0031):收集启用集后遍历 AST,gated 语法未启用 → `RX0010`。
pub fn check_feature_gates(file: &SourceFile, diag: &DiagCtxt) {
    let features = collect_features(file, diag);
    let checker = GateChecker { features, diag };
    for item in &file.items {
        checker.check_item(item);
    }
}

struct GateChecker<'a> {
    features: Features,
    diag: &'a DiagCtxt,
}

impl GateChecker<'_> {
    fn check_item(&self, item: &Item) {
        match &item.kind {
            ItemKind::Fn(f) => {
                if let Some(body) = &f.body {
                    self.check_block(body);
                }
            }
            ItemKind::Static(s) => self.check_expr(&s.init),
            ItemKind::Const(c) => self.check_expr(&c.init),
            ItemKind::Mod(m) => {
                for it in &m.items {
                    self.check_item(it);
                }
            }
            ItemKind::Trait(t) => {
                for a in &t.items {
                    self.check_assoc_item(a);
                }
            }
            ItemKind::Impl(i) => {
                for a in &i.items {
                    self.check_assoc_item(a);
                }
            }
            ItemKind::ExternBlock(e) => {
                for it in &e.items {
                    self.check_item(it);
                }
            }
            ItemKind::Struct(_)
            | ItemKind::Enum(_)
            | ItemKind::Use(_)
            | ItemKind::TypeAlias(_)
            | ItemKind::Err => {}
        }
    }

    fn check_assoc_item(&self, item: &AssocItem) {
        match &item.kind {
            AssocItemKind::Fn(f) => {
                if let Some(body) = &f.body {
                    self.check_block(body);
                }
            }
            AssocItemKind::Const(c) => self.check_expr(&c.init),
            AssocItemKind::Type { .. } => {}
        }
    }

    fn check_block(&self, block: &Block) {
        for stmt in &block.stmts {
            match &stmt.kind {
                StmtKind::Item(item) => self.check_item(item),
                StmtKind::Let(l) => {
                    if let Some(init) = &l.init {
                        self.check_expr(init);
                    }
                }
                StmtKind::Expr { expr, .. } => self.check_expr(expr),
                StmtKind::Empty => {}
            }
        }
        if let Some(tail) = &block.tail {
            self.check_expr(tail);
        }
    }

    fn check_expr(&self, expr: &Expr) {
        if let ExprKind::Closure { .. } = &expr.kind
            && !self.features.is_enabled("closures")
        {
            self.diag
                .struct_error(E_GATED_FEATURE, "parse.gated_feature")
                .arg("feature", "closures")
                .span_label(expr.span, "gated syntax")
                .help("add `#![feature(closures)]` at the top of the file to enable")
                .emit();
        }
        match &expr.kind {
            ExprKind::Lit(_) | ExprKind::Path(_) | ExprKind::Continue | ExprKind::Err => {}
            ExprKind::Unary { expr, .. }
            | ExprKind::Borrow { expr, .. }
            | ExprKind::Cast { expr, .. }
            | ExprKind::Try(expr)
            | ExprKind::Paren(expr)
            | ExprKind::Field { expr, .. }
            | ExprKind::TupleField { expr, .. } => self.check_expr(expr),
            ExprKind::Binary { lhs, rhs, .. }
            | ExprKind::Assign { lhs, rhs, .. }
            | ExprKind::Range {
                lo: lhs, hi: rhs, ..
            } => {
                self.check_expr(lhs);
                self.check_expr(rhs);
            }
            ExprKind::Call { callee, args } => {
                self.check_expr(callee);
                for a in args {
                    self.check_expr(a);
                }
            }
            ExprKind::MethodCall { receiver, args, .. } => {
                self.check_expr(receiver);
                for a in args {
                    self.check_expr(a);
                }
            }
            ExprKind::Index { expr, index } => {
                self.check_expr(expr);
                self.check_expr(index);
            }
            ExprKind::Tuple(elems) | ExprKind::Array(elems) => {
                for e in elems {
                    self.check_expr(e);
                }
            }
            ExprKind::Repeat { elem, len } => {
                self.check_expr(elem);
                self.check_expr(len);
            }
            ExprKind::StructLit { fields, .. } => {
                for f in fields {
                    if let Some(e) = &f.expr {
                        self.check_expr(e);
                    }
                }
            }
            ExprKind::Block(b) | ExprKind::Unsafe(b) | ExprKind::Loop { body: b } => {
                self.check_block(b);
            }
            ExprKind::If { cond, then, else_ } => {
                self.check_expr(cond);
                self.check_block(then);
                if let Some(e) = else_ {
                    self.check_expr(e);
                }
            }
            ExprKind::While { cond, body } => {
                self.check_expr(cond);
                self.check_block(body);
            }
            ExprKind::For { iter, body, .. } => {
                self.check_expr(iter);
                self.check_block(body);
            }
            ExprKind::Match { scrutinee, arms } => {
                self.check_expr(scrutinee);
                for arm in arms {
                    if let Some(g) = &arm.guard {
                        self.check_expr(g);
                    }
                    self.check_expr(&arm.body);
                }
            }
            ExprKind::Return(operand) | ExprKind::Break(operand) => {
                if let Some(e) = operand {
                    self.check_expr(e);
                }
            }
            ExprKind::Closure { body, .. } => self.check_expr(body),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::DiagCtxt;
    use crate::lexer::lex;
    use crate::parser::parse;
    use crate::span::{Edition, SourceId};

    fn check(src: &str) -> DiagCtxt {
        let diag = DiagCtxt::new();
        let tokens = lex(src, SourceId(0), Edition::Rx0, &diag);
        let file = parse(src, tokens, SourceId(0), Edition::Rx0, &diag);
        check_feature_gates(&file, &diag);
        diag
    }

    //@ spec: RXS-0031
    #[test]
    fn closure_without_gate_is_rx0010() {
        let diag = check("fn f() { let g = |x: i32| x; }");
        let emitted = diag.emitted();
        assert_eq!(emitted.len(), 1, "{emitted:?}");
        assert_eq!(emitted[0].code, Some(E_GATED_FEATURE));
    }

    //@ spec: RXS-0031
    #[test]
    fn closure_with_gate_is_clean() {
        let diag = check("#![feature(closures)]\nfn f() { let g = |x: i32| x; }");
        assert!(diag.emitted().is_empty(), "{:?}", diag.emitted());
    }

    //@ spec: RXS-0031
    #[test]
    fn move_closure_without_gate_is_rx0010() {
        let diag = check("fn f() { let g = move || 1; }");
        assert_eq!(diag.emitted()[0].code, Some(E_GATED_FEATURE));
    }

    //@ spec: RXS-0031
    #[test]
    fn unknown_feature_is_rx0011() {
        let diag = check("#![feature(telepathy)]\nfn f() {}");
        let emitted = diag.emitted();
        assert_eq!(emitted.len(), 1, "{emitted:?}");
        assert_eq!(emitted[0].code, Some(E_UNKNOWN_FEATURE));
    }

    //@ spec: RXS-0031
    #[test]
    fn nested_closure_in_match_arm_is_flagged() {
        let diag = check("fn f(n: i32) { match n { _ => { let g = || 0; } } }");
        assert_eq!(diag.emitted()[0].code, Some(E_GATED_FEATURE));
    }
}
