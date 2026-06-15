//! ToolingSession:已打开文档 + query 层 + 无损语法树(RXS-0098)。

use std::collections::HashMap;

use rowan::SyntaxNode;

use crate::diag::DiagCtxt;
use crate::feature_gate::check_feature_gates;
use crate::lexer::lex;
use crate::lossless::{self, RxLanguage};
use crate::parser::parse_with_events;
use crate::query::QueryCtx;
use crate::source_map::SourceMap;
use crate::span::{Edition, SourceId};

/// 单份已打开文档的状态。
pub struct OpenDoc {
    pub uri: String,
    pub version: i32,
    pub file_id: SourceId,
    pub text: String,
    pub diag: DiagCtxt,
    pub syntax: SyntaxNode<RxLanguage>,
}

fn analyze(text: &str, file_id: SourceId) -> (DiagCtxt, SyntaxNode<RxLanguage>) {
    let diag = DiagCtxt::new();
    let tokens = lex(text, file_id, Edition::Rx0, &diag);
    let (ast, events) = parse_with_events(text, tokens, file_id, Edition::Rx0, &diag);
    let syntax = lossless::build_tree(&events, text);
    let cx = QueryCtx::from_ast(ast, text, file_id, &diag);
    check_feature_gates(cx.ast(), &diag);
    if !diag.has_errors() {
        cx.check_crate();
    }
    (diag, syntax)
}

/// 为 IDE 查询构造 QueryCtx(复用已收集诊断,不再重复 emit)。
pub fn query_ctx_for<'a>(doc: &'a OpenDoc) -> QueryCtx<'a> {
    let tokens = lex(&doc.text, doc.file_id, Edition::Rx0, &doc.diag);
    let ast = crate::parser::parse(&doc.text, tokens, doc.file_id, Edition::Rx0, &doc.diag);
    QueryCtx::from_ast(ast, &doc.text, doc.file_id, &doc.diag)
}

/// 常驻 tooling 会话(进程内 memo 在 QueryCtx 实例生命周期内有效)。
pub struct ToolingSession {
    pub source_map: SourceMap,
    docs: HashMap<String, OpenDoc>,
}

impl ToolingSession {
    pub fn new() -> Self {
        Self {
            source_map: SourceMap::new(),
            docs: HashMap::new(),
        }
    }

    pub fn open(&mut self, uri: String, text: String, version: i32) {
        let file_id = self
            .source_map
            .add_file(uri.clone(), text.clone(), Edition::Rx0);
        let (diag, syntax) = analyze(&text, file_id);
        self.docs.insert(
            uri.clone(),
            OpenDoc {
                uri,
                version,
                file_id,
                text,
                diag,
                syntax,
            },
        );
    }

    pub fn change(&mut self, uri: &str, version: i32, text: String) {
        if let Some(doc) = self.docs.get_mut(uri) {
            let (diag, syntax) = analyze(&text, doc.file_id);
            self.source_map.update_file(doc.file_id, text.clone());
            doc.version = version;
            doc.text = text;
            doc.diag = diag;
            doc.syntax = syntax;
        }
    }

    pub fn get(&self, uri: &str) -> Option<&OpenDoc> {
        self.docs.get(uri)
    }
}

impl Default for ToolingSession {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0098
    #[test]
    fn session_open_and_change_updates_version() {
        let mut sess = ToolingSession::new();
        sess.open("file:///t.rx".into(), "fn main() {}".into(), 1);
        sess.change("file:///t.rx", 2, "fn main() { let x = 1; }".into());
        let doc = sess.get("file:///t.rx").unwrap();
        assert_eq!(doc.version, 2);
        assert!(doc.text.contains("let x"));
        let off = sess.source_map.from_lsp_position(doc.file_id, 0, 12);
        assert_eq!(
            &sess.source_map.file(doc.file_id).src()[off.0 as usize..],
            "let x = 1; }"
        );
    }
}
