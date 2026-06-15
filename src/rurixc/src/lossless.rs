//! RD-004 无损语法树通道:parser 事件流 → rowan 式绿/红树(07 §9;RXS-0030 第 5 条)。

use rowan::{GreenNodeBuilder, Language, SyntaxKind, SyntaxNode, TextRange, TextSize};

use crate::parser::{NodeKind, ParseEvent};
use crate::span::Span;

/// rowan [`Language`] 实现(Rurix 语法树种类)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RxLanguage {}

impl Language for RxLanguage {
    type Kind = RxSyntaxKind;

    fn kind_from_raw(raw: SyntaxKind) -> Self::Kind {
        RxSyntaxKind(raw.0)
    }

    fn kind_to_raw(kind: Self::Kind) -> SyntaxKind {
        SyntaxKind(kind.0)
    }
}

/// 语法树节点/ token 种类(RD-004;与 [`NodeKind`] 对齐并扩展 TOKEN)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RxSyntaxKind(u16);

impl RxSyntaxKind {
    pub const TOKEN: Self = Self(0);
    pub const ROOT: Self = Self(1);

    pub fn from_node(kind: NodeKind) -> Self {
        Self(10 + kind as u16)
    }

    fn raw(self) -> SyntaxKind {
        SyntaxKind(self.0)
    }
}

/// 从 parser 事件流构建无损语法树。
pub fn build_tree(events: &[ParseEvent], src: &str) -> SyntaxNode<RxLanguage> {
    let mut builder = GreenNodeBuilder::new();
    builder.start_node(RxSyntaxKind::ROOT.raw());
    for ev in events {
        match *ev {
            ParseEvent::Start(kind) => {
                builder.start_node(RxSyntaxKind::from_node(kind).raw());
            }
            ParseEvent::Token(span) => {
                let text = snippet_span(src, span);
                builder.token(RxSyntaxKind::TOKEN.raw(), text);
            }
            ParseEvent::Finish(_) => {
                builder.finish_node();
            }
        }
    }
    builder.finish_node();
    SyntaxNode::new_root(builder.finish())
}

fn snippet_span(src: &str, span: Span) -> &str {
    &src[span.lo.0 as usize..span.hi.0 as usize]
}

/// 字节偏移处的 token 文本 range(若落在 TOKEN 叶子上)。
pub fn token_at_offset(root: &SyntaxNode<RxLanguage>, offset: TextSize) -> Option<TextRange> {
    let token = root.token_at_offset(offset).left_biased()?;
    if token.kind() == RxSyntaxKind::TOKEN {
        Some(token.text_range())
    } else {
        None
    }
}

/// 覆盖 offset 的最内层节点 text range。
pub fn covering_range(root: &SyntaxNode<RxLanguage>, offset: TextSize) -> Option<TextRange> {
    let range = TextRange::new(offset, offset);
    Some(root.covering_element(range).text_range())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::DiagCtxt;
    use crate::lexer::lex;
    use crate::parser::parse_with_events;
    use crate::span::{Edition, SourceId};

    //@ spec: RXS-0030
    #[test]
    fn event_stream_builds_tree_with_token_offsets() {
        let src = "fn main() { let x = 1; }";
        let diag = DiagCtxt::new();
        let file = SourceId(0);
        let tokens = lex(src, file, Edition::Rx0, &diag);
        let (_ast, events) = parse_with_events(src, tokens, file, Edition::Rx0, &diag);
        let tree = build_tree(&events, src);
        assert!(!tree.text().to_string().is_empty());
        let main_off = TextSize::from((src.find("main").unwrap() + 2) as u32);
        assert!(token_at_offset(&tree, main_off).is_some());
        assert!(covering_range(&tree, main_off).is_some());
    }
}
