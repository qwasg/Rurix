//! `rx test` 窄 harness helper(RXS-0095).
//!
//! 只负责测试发现、签名校验与逐测试临时 `main` 渲染。编译与执行仍由
//! `rx` 经 [`crate::driver`] 走单一前端。

use std::error::Error;
use std::fmt;

use crate::ast::{
    Attr, FnColor, FnItem, ItemKind, MetaInner, MetaItem, MetaKind, Path, Ty, TyKind,
};
use crate::diag::DiagCtxt;
use crate::lexer::lex;
use crate::parser::parse;
use crate::source_map::SourceMap;
use crate::span::Edition;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestKind {
    Host,
    Gpu,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestReturn {
    Unit,
    I32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestCase {
    pub name: String,
    pub kind: TestKind,
    pub returns: TestReturn,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestHarnessError {
    detail: String,
}

impl TestHarnessError {
    fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for TestHarnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.detail)
    }
}

impl Error for TestHarnessError {}

/// 发现顶层 `#[test]` / `#[test(gpu)]` free functions 并校验 M6.3 v1 签名。
pub fn discover_tests(src: &str) -> Result<Vec<TestCase>, TestHarnessError> {
    let diag = DiagCtxt::new();
    let mut sm = SourceMap::new();
    let id = sm.add_file("rx-test-input.rx", src, Edition::Rx0);
    let toks = lex(src, id, Edition::Rx0, &diag);
    let ast = parse(src, toks, id, Edition::Rx0, &diag);
    if diag.has_errors() {
        return Err(TestHarnessError::new(
            "测试源包含词法/语法错误,无法发现 #[test]",
        ));
    }

    let mut tests = Vec::new();
    let mut has_main = false;
    for item in &ast.items {
        let ItemKind::Fn(f) = &item.kind else {
            continue;
        };
        if f.name.name == "main" {
            has_main = true;
        }
        let Some(kind) = test_kind(&item.attrs)? else {
            continue;
        };
        validate_test_fn(f)?;
        let returns = classify_return(f.ret.as_ref())?;
        tests.push(TestCase {
            name: f.name.name.clone(),
            kind,
            returns,
        });
    }
    if has_main && !tests.is_empty() {
        return Err(TestHarnessError::new(
            "`rx test` M6.3 v1 不支持测试源同时定义顶层 `fn main`",
        ));
    }
    Ok(tests)
}

/// 将原始源附加一个只调用 `case` 的临时入口。
pub fn render_harness(src: &str, case: &TestCase) -> String {
    let mut out = String::with_capacity(src.len() + case.name.len() + 64);
    out.push_str(src);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    match case.returns {
        TestReturn::Unit => {
            out.push_str("\nfn main() {\n    ");
            out.push_str(&case.name);
            out.push_str("();\n}\n");
        }
        TestReturn::I32 => {
            out.push_str("\nfn main() -> i32 {\n    ");
            out.push_str(&case.name);
            out.push_str("()\n}\n");
        }
    }
    out
}

fn test_kind(attrs: &[Attr]) -> Result<Option<TestKind>, TestHarnessError> {
    let mut out = None;
    for attr in attrs {
        let Some(kind) = test_attr_kind(&attr.meta)? else {
            continue;
        };
        if out.replace(kind).is_some() {
            return Err(TestHarnessError::new("同一函数重复标注 #[test]"));
        }
    }
    Ok(out)
}

fn test_attr_kind(meta: &MetaItem) -> Result<Option<TestKind>, TestHarnessError> {
    if !path_is(&meta.path, "test") {
        return Ok(None);
    }
    match &meta.kind {
        MetaKind::Path => Ok(Some(TestKind::Host)),
        MetaKind::List(items) => {
            if items.len() == 1
                && matches!(
                    &items[0],
                    MetaInner::Meta(MetaItem {
                        path,
                        kind: MetaKind::Path,
                        ..
                    }) if path_is(path, "gpu")
                )
            {
                Ok(Some(TestKind::Gpu))
            } else {
                Err(TestHarnessError::new(
                    "仅支持 #[test] 与 #[test(gpu)] 两种测试属性",
                ))
            }
        }
        MetaKind::NameValue(_) => Err(TestHarnessError::new(
            "仅支持 #[test] 与 #[test(gpu)] 两种测试属性",
        )),
    }
}

fn validate_test_fn(f: &FnItem) -> Result<(), TestHarnessError> {
    if f.color != FnColor::Host {
        return Err(TestHarnessError::new(format!(
            "测试函数 `{}` 必须是 host 普通函数",
            f.name.name
        )));
    }
    if !f.generics.params.is_empty() || !f.generics.where_preds.is_empty() {
        return Err(TestHarnessError::new(format!(
            "测试函数 `{}` 不得声明泛型或 where 子句",
            f.name.name
        )));
    }
    if !f.params.is_empty() {
        return Err(TestHarnessError::new(format!(
            "测试函数 `{}` 不得携带参数",
            f.name.name
        )));
    }
    if f.body.is_none() {
        return Err(TestHarnessError::new(format!(
            "测试函数 `{}` 必须有函数体",
            f.name.name
        )));
    }
    Ok(())
}

fn classify_return(ret: Option<&Ty>) -> Result<TestReturn, TestHarnessError> {
    let Some(ret) = ret else {
        return Ok(TestReturn::Unit);
    };
    if is_unit_ty(ret) {
        return Ok(TestReturn::Unit);
    }
    if is_i32_ty(ret) {
        return Ok(TestReturn::I32);
    }
    Err(TestHarnessError::new(
        "测试函数返回类型只能是 `()` 或 `i32`",
    ))
}

fn is_unit_ty(ty: &Ty) -> bool {
    match &ty.kind {
        TyKind::Tuple(items) => items.is_empty(),
        TyKind::Paren(inner) => is_unit_ty(inner),
        _ => false,
    }
}

fn is_i32_ty(ty: &Ty) -> bool {
    match &ty.kind {
        TyKind::Path(path) => path_is(path, "i32"),
        TyKind::Paren(inner) => is_i32_ty(inner),
        _ => false,
    }
}

fn path_is(path: &Path, name: &str) -> bool {
    path.segments.len() == 1 && path.segments[0].ident.name == name
}

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0095
    #[test]
    fn discovers_host_and_gpu_tests() {
        let src = r#"
#[test]
fn host_ok() {}

#[test(gpu)]
fn gpu_ok() -> i32 { 0 }
"#;
        let tests = discover_tests(src).unwrap();
        assert_eq!(tests.len(), 2);
        assert_eq!(tests[0].name, "host_ok");
        assert_eq!(tests[0].kind, TestKind::Host);
        assert_eq!(tests[0].returns, TestReturn::Unit);
        assert_eq!(tests[1].name, "gpu_ok");
        assert_eq!(tests[1].kind, TestKind::Gpu);
        assert_eq!(tests[1].returns, TestReturn::I32);
    }

    //@ spec: RXS-0095
    #[test]
    fn rejects_bad_signature_and_main_conflict() {
        assert!(discover_tests("#[test]\nfn bad(x: i32) {}\n").is_err());
        assert!(discover_tests("#[test]\nfn t() {}\nfn main() {}\n").is_err());
    }

    //@ spec: RXS-0095
    #[test]
    fn renders_one_test_main() {
        let case = TestCase {
            name: "passes".to_owned(),
            kind: TestKind::Host,
            returns: TestReturn::I32,
        };
        let harness = render_harness("#[test]\nfn passes() -> i32 { 0 }\n", &case);
        assert!(harness.contains("fn main() -> i32"));
        assert!(harness.contains("passes()"));
    }
}
