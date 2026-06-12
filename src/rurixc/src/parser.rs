//! 手写递归下降 parser(spec 条款 RXS-0011 ~ RXS-0030,spec/syntax.md)。
//!
//! - **错误恢复优先**(RXS-0030):报错后跳至同步点(item 起始 token / `;` / `}` /
//!   EOF)继续,单文件可多错,仍产出部分 AST(错误子树以 `Err` 节点占位);
//! - 表达式按 RXS-0025 优先级表以 Pratt 方式解析;比较与区间不可链式;
//! - 泛型实参闭合位置拆分 `>>` / `>=` / `>>=`(RXS-0021);
//! - 事件流接口预留:见 [`ParseEvent`] // STUB(RD-004)。
//!
//! 错误码 RX0008 / RX0009(registry/error_codes.json,M1.3 分配);
//! feature gate 检查(RX0010 / RX0011)在 [`crate::feature_gate`],parse 后单独跑。

use crate::ast::*;
use crate::diag::{DiagCtxt, ErrorCode};
use crate::lexer::{Keyword as Kw, Token, TokenKind as Tk};
use crate::span::{Edition, SourceId, Span};

pub const E_EXPECTED_TOKEN: ErrorCode = ErrorCode(8); // RX0008
pub const E_UNCLOSED_DELIMITER: ErrorCode = ErrorCode(9); // RX0009

// ---------------------------------------------------------------------------
// 事件流接口预留
// ---------------------------------------------------------------------------

/// STUB(RD-004): parser 事件流接口预留(07 §9;RXS-0030 第 5 条)。
///
/// M1 仅定义事件形态并在主要节点边界与 token 消费点产出,无消费者;
/// 完整无损语法树(rowan 式)通道随 M6 LSP MVP 评估接通(RD-004)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParseEvent {
    Start(NodeKind),
    Token(Span),
    Finish(NodeKind),
}

/// STUB(RD-004): 事件节点粒度首批(粗粒度;细化随 RD-004 回填)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NodeKind {
    SourceFile,
    Item,
    Block,
    Expr,
    Pat,
    Ty,
}

/// 解析入口:消费 lexer 产出的 token 流(RXS-0011)。
///
/// 诊断经 `diag` 产出(RXS-0030);出错仍返回部分 AST。
pub fn parse(
    src: &str,
    tokens: Vec<Token>,
    file: SourceId,
    edition: Edition,
    diag: &DiagCtxt,
) -> SourceFile {
    parse_with_events(src, tokens, file, edition, diag).0
}

/// 同 [`parse`],附带事件流(STUB(RD-004):当前无消费者,供单测与未来无损树通道)。
pub fn parse_with_events(
    src: &str,
    tokens: Vec<Token>,
    file: SourceId,
    edition: Edition,
    diag: &DiagCtxt,
) -> (SourceFile, Vec<ParseEvent>) {
    debug_assert!(
        tokens.last().is_some_and(|t| t.kind == Tk::Eof),
        "token 流必须以 Eof 收尾(RXS-0010)"
    );
    let mut p = Parser {
        src,
        toks: tokens,
        pos: 0,
        file,
        edition,
        diag,
        events: Vec::new(),
        no_struct: false,
        last_err_pos: usize::MAX,
    };
    let sf = p.parse_source_file();
    (sf, p.events)
}

struct Parser<'a> {
    src: &'a str,
    toks: Vec<Token>,
    pos: usize,
    file: SourceId,
    edition: Edition,
    diag: &'a DiagCtxt,
    events: Vec<ParseEvent>,
    /// 结构体字面量限制(RXS-0026):if/while/for/match 头部位置为 true。
    no_struct: bool,
    /// 抑制同一 token 位置的重复 RX0008(防错误级联)。
    last_err_pos: usize,
}

/// 函数所在容器(函数体/`;` 合法性,RXS-0014)。
#[derive(Clone, Copy, PartialEq, Eq)]
enum FnCtx {
    /// 顶层/mod/块内:必须有函数体。
    Free,
    /// trait 体:函数体或 `;` 均可。
    Trait,
    /// impl 体:必须有函数体(RXS-0016)。
    Impl,
    /// extern 块:必须 `;`(RXS-0019)。
    Extern,
}

impl<'a> Parser<'a> {
    // -- token 基础 ---------------------------------------------------------

    fn peek(&self) -> Token {
        self.toks[self.pos.min(self.toks.len() - 1)]
    }

    fn nth_kind(&self, n: usize) -> Tk {
        self.toks[(self.pos + n).min(self.toks.len() - 1)].kind
    }

    fn kind(&self) -> Tk {
        self.peek().kind
    }

    fn bump(&mut self) -> Token {
        let tok = self.peek();
        if tok.kind != Tk::Eof {
            self.pos += 1;
        }
        self.events.push(ParseEvent::Token(tok.span)); // STUB(RD-004)
        tok
    }

    fn at(&self, kind: Tk) -> bool {
        self.kind() == kind
    }

    fn at_kw(&self, kw: Kw) -> bool {
        self.kind() == Tk::Kw(kw)
    }

    fn nth_is_kw(&self, n: usize, kw: Kw) -> bool {
        self.nth_kind(n) == Tk::Kw(kw)
    }

    fn eat(&mut self, kind: Tk) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn eat_kw(&mut self, kw: Kw) -> bool {
        self.eat(Tk::Kw(kw))
    }

    fn token_text(&self, tok: Token) -> &'a str {
        &self.src[tok.span.lo.0 as usize..tok.span.hi.0 as usize]
    }

    fn cur_text(&self) -> &'a str {
        self.token_text(self.peek())
    }

    fn at_ident(&self) -> bool {
        self.at(Tk::Ident)
    }

    fn at_int(&self) -> bool {
        matches!(self.kind(), Tk::IntLit { .. })
    }

    fn lo(&self) -> u32 {
        self.peek().span.lo.0
    }

    fn prev_hi(&self) -> u32 {
        if self.pos == 0 {
            self.peek().span.lo.0
        } else {
            self.toks[self.pos - 1].span.hi.0
        }
    }

    fn span_from(&self, lo: u32) -> Span {
        Span::new(self.file, lo, self.prev_hi().max(lo), self.edition)
    }

    // -- 诊断与恢复(RXS-0030) ---------------------------------------------

    fn error_expected(&mut self, expected: &str) {
        if self.last_err_pos == self.pos {
            return;
        }
        self.last_err_pos = self.pos;
        let tok = self.peek();
        let found = if tok.kind == Tk::Eof {
            "end of file".to_owned()
        } else {
            format!("`{}`", self.token_text(tok))
        };
        self.diag
            .struct_error(E_EXPECTED_TOKEN, "parse.expected_token")
            .arg("expected", expected)
            .arg("found", found)
            .span_label(tok.span, format!("expected {expected}"))
            .emit();
    }

    fn error_unclosed(&mut self, open_span: Span, delim: &str) {
        self.diag
            .struct_error(E_UNCLOSED_DELIMITER, "parse.unclosed_delimiter")
            .arg("delim", format!("`{delim}`"))
            .span_label(open_span, "unclosed delimiter")
            .emit();
    }

    fn expect(&mut self, kind: Tk, expected: &str) -> bool {
        if self.eat(kind) {
            true
        } else {
            self.error_expected(expected);
            false
        }
    }

    fn expect_ident(&mut self, expected: &str) -> Ident {
        if self.at_ident() {
            self.make_ident()
        } else {
            self.error_expected(expected);
            Ident {
                name: String::new(),
                span: self.peek().span,
            }
        }
    }

    fn make_ident(&mut self) -> Ident {
        debug_assert!(self.at_ident());
        let tok = self.bump();
        Ident {
            name: self.token_text(tok).to_owned(),
            span: tok.span,
        }
    }

    fn is_item_start(kind: Tk) -> bool {
        matches!(
            kind,
            Tk::Kw(
                Kw::Fn
                    | Kw::Kernel
                    | Kw::Device
                    | Kw::Const
                    | Kw::Struct
                    | Kw::Enum
                    | Kw::Trait
                    | Kw::Impl
                    | Kw::Mod
                    | Kw::Use
                    | Kw::Static
                    | Kw::Type
                    | Kw::Extern
                    | Kw::Pub
            ) | Tk::Pound
        )
    }

    /// 同步点恢复:跳过 token 直至 item 起始 / `;`(消费) / `}`(不消费) / EOF。
    /// 保证至少消费一个 token(调用方负责前置进度判断)。
    fn recover_to_sync(&mut self) {
        if self.at(Tk::Eof) {
            return;
        }
        let mut depth = 0u32;
        self.bump();
        loop {
            match self.kind() {
                Tk::Eof => return,
                Tk::OpenParen | Tk::OpenBracket | Tk::OpenBrace => {
                    depth += 1;
                    self.bump();
                }
                Tk::CloseParen | Tk::CloseBracket => {
                    depth = depth.saturating_sub(1);
                    self.bump();
                }
                Tk::CloseBrace => {
                    if depth == 0 {
                        return;
                    }
                    depth -= 1;
                    self.bump();
                }
                Tk::Semi if depth == 0 => {
                    self.bump();
                    return;
                }
                k if depth == 0 && Self::is_item_start(k) => return,
                _ => {
                    self.bump();
                }
            }
        }
    }

    fn with_no_struct<T>(&mut self, v: bool, f: impl FnOnce(&mut Self) -> T) -> T {
        let saved = self.no_struct;
        self.no_struct = v;
        let r = f(self);
        self.no_struct = saved;
        r
    }

    // -- 源文件与 item(RXS-0011 ~ RXS-0019) -------------------------------

    fn parse_source_file(&mut self) -> SourceFile {
        self.events.push(ParseEvent::Start(NodeKind::SourceFile)); // STUB(RD-004)
        let lo = self.lo();
        let mut attrs = Vec::new();
        while self.at(Tk::Pound) && self.nth_kind(1) == Tk::Not {
            attrs.push(self.parse_attr(true));
        }
        let mut items = Vec::new();
        while !self.at(Tk::Eof) {
            items.push(self.parse_item());
        }
        self.events.push(ParseEvent::Finish(NodeKind::SourceFile)); // STUB(RD-004)
        SourceFile {
            attrs,
            items,
            span: self.span_from(lo),
        }
    }

    fn parse_item(&mut self) -> Item {
        self.events.push(ParseEvent::Start(NodeKind::Item)); // STUB(RD-004)
        let lo = self.lo();
        let attrs = self.parse_outer_attrs();
        let vis = self.parse_visibility();
        let kind = self.parse_item_kind();
        self.events.push(ParseEvent::Finish(NodeKind::Item)); // STUB(RD-004)
        Item {
            attrs,
            vis,
            kind,
            span: self.span_from(lo),
        }
    }

    fn parse_item_kind(&mut self) -> ItemKind {
        match self.kind() {
            Tk::Kw(Kw::Fn) => {
                self.bump();
                ItemKind::Fn(self.parse_fn(FnColor::Host, FnCtx::Free))
            }
            Tk::Kw(Kw::Kernel) => {
                self.bump();
                self.expect(Tk::Kw(Kw::Fn), "`fn`");
                ItemKind::Fn(self.parse_fn(FnColor::Kernel, FnCtx::Free))
            }
            Tk::Kw(Kw::Device) => {
                self.bump();
                self.expect(Tk::Kw(Kw::Fn), "`fn`");
                ItemKind::Fn(self.parse_fn(FnColor::Device, FnCtx::Free))
            }
            Tk::Kw(Kw::Const) if self.nth_is_kw(1, Kw::Fn) => {
                self.bump();
                self.bump();
                ItemKind::Fn(self.parse_fn(FnColor::Const, FnCtx::Free))
            }
            Tk::Kw(Kw::Const) => {
                self.bump();
                ItemKind::Const(self.parse_const_item())
            }
            Tk::Kw(Kw::Struct) => {
                self.bump();
                ItemKind::Struct(self.parse_struct())
            }
            Tk::Kw(Kw::Enum) => {
                self.bump();
                ItemKind::Enum(self.parse_enum())
            }
            Tk::Kw(Kw::Trait) => {
                self.bump();
                ItemKind::Trait(self.parse_trait())
            }
            Tk::Kw(Kw::Impl) => {
                self.bump();
                ItemKind::Impl(self.parse_impl())
            }
            Tk::Kw(Kw::Mod) => {
                self.bump();
                ItemKind::Mod(self.parse_mod())
            }
            Tk::Kw(Kw::Use) => {
                self.bump();
                ItemKind::Use(self.parse_use())
            }
            Tk::Kw(Kw::Static) => {
                self.bump();
                ItemKind::Static(self.parse_static())
            }
            Tk::Kw(Kw::Type) => {
                self.bump();
                ItemKind::TypeAlias(self.parse_type_alias())
            }
            Tk::Kw(Kw::Extern) => {
                self.bump();
                ItemKind::ExternBlock(self.parse_extern_block())
            }
            _ => {
                self.error_expected("an item");
                self.recover_to_sync();
                ItemKind::Err
            }
        }
    }

    fn parse_visibility(&mut self) -> Visibility {
        if !self.at_kw(Kw::Pub) {
            return Visibility::Inherited;
        }
        let tok = self.bump();
        if self.at(Tk::OpenParen) {
            self.bump();
            if self.at_ident() && self.cur_text() == "package" {
                self.bump();
            } else {
                // RXS-0013:pub(…) 仅允许 package
                self.error_expected("`package`");
            }
            self.expect(Tk::CloseParen, "`)`");
            Visibility::PubPackage(self.span_from(tok.span.lo.0))
        } else {
            Visibility::Pub(tok.span)
        }
    }

    // -- 属性(RXS-0012) ----------------------------------------------------

    fn parse_outer_attrs(&mut self) -> Vec<Attr> {
        let mut attrs = Vec::new();
        while self.at(Tk::Pound) {
            if self.nth_kind(1) == Tk::Not {
                // 内部属性仅文件顶部(RXS-0011)
                self.error_expected(
                    "an outer attribute (inner attributes are only allowed at the top of the file)",
                );
                self.parse_attr(true); // 消费以恢复
                continue;
            }
            attrs.push(self.parse_attr(false));
        }
        attrs
    }

    fn parse_attr(&mut self, inner: bool) -> Attr {
        let lo = self.lo();
        self.expect(Tk::Pound, "`#`");
        if inner {
            self.expect(Tk::Not, "`!`");
        }
        let open = self.peek().span;
        self.expect(Tk::OpenBracket, "`[`");
        let meta = self.parse_meta_item();
        if !self.eat(Tk::CloseBracket) {
            if self.at(Tk::Eof) {
                self.error_unclosed(open, "[");
            } else {
                self.error_expected("`]`");
            }
        }
        Attr {
            inner,
            meta,
            span: self.span_from(lo),
        }
    }

    fn parse_meta_item(&mut self) -> MetaItem {
        let lo = self.lo();
        let path = self.parse_plain_path("an attribute path");
        let kind = if self.at(Tk::OpenParen) {
            let open = self.peek().span;
            self.bump();
            let mut inner = Vec::new();
            while !self.at(Tk::CloseParen) {
                if self.at(Tk::Eof) {
                    self.error_unclosed(open, "(");
                    break;
                }
                if let Some(lit) = self.try_parse_lit() {
                    inner.push(MetaInner::Lit(lit));
                } else if self.at_ident() {
                    inner.push(MetaInner::Meta(self.parse_meta_item()));
                } else {
                    self.error_expected("a meta item or literal");
                    self.bump();
                    continue;
                }
                if !self.eat(Tk::Comma) {
                    break;
                }
            }
            self.expect(Tk::CloseParen, "`)`");
            MetaKind::List(inner)
        } else if self.eat(Tk::Eq) {
            if let Some(lit) = self.try_parse_lit() {
                MetaKind::NameValue(lit)
            } else {
                self.error_expected("a literal");
                MetaKind::Path
            }
        } else {
            MetaKind::Path
        };
        MetaItem {
            path,
            kind,
            span: self.span_from(lo),
        }
    }

    // -- 函数(RXS-0014) ----------------------------------------------------

    fn parse_fn(&mut self, color: FnColor, ctx: FnCtx) -> FnItem {
        let name = self.expect_ident("a function name");
        let mut generics = self.parse_generic_params();
        let params = self.parse_fn_params();
        let ret = if self.eat(Tk::Arrow) {
            Some(self.parse_type())
        } else {
            None
        };
        generics.where_preds = self.parse_where_clause();
        let body = if self.at(Tk::OpenBrace) {
            if ctx == FnCtx::Extern {
                // RXS-0019:extern 块内不允许函数体
                self.error_expected("`;` (functions in `extern` blocks have no body)");
            }
            Some(self.parse_block())
        } else if self.eat(Tk::Semi) {
            if matches!(ctx, FnCtx::Free | FnCtx::Impl) {
                // RXS-0014/RXS-0016:此处必须有函数体
                self.error_expected("a function body");
            }
            None
        } else {
            self.error_expected("a function body or `;`");
            None
        };
        FnItem {
            color,
            name,
            generics,
            params,
            ret,
            body,
        }
    }

    fn parse_fn_params(&mut self) -> Vec<Param> {
        let mut params = Vec::new();
        let open = self.peek().span;
        if !self.expect(Tk::OpenParen, "`(`") {
            return params;
        }
        while !self.at(Tk::CloseParen) {
            if self.at(Tk::Eof) {
                self.error_unclosed(open, "(");
                return params;
            }
            let lo = self.lo();
            let attrs = self.parse_outer_attrs();
            let kind = if let Some(kind) = self.try_parse_self_param() {
                if !params.is_empty() {
                    // RXS-0014:self 仅允许首位
                    self.error_expected("a parameter (`self` is only allowed in first position)");
                }
                kind
            } else {
                let pat = self.parse_pattern();
                self.expect(Tk::Colon, "`:`");
                let ty = self.parse_type();
                ParamKind::Typed { pat, ty }
            };
            params.push(Param {
                attrs,
                kind,
                span: self.span_from(lo),
            });
            if !self.eat(Tk::Comma) {
                break;
            }
        }
        self.expect(Tk::CloseParen, "`)`");
        params
    }

    /// `self` / `mut self` / `&self` / `&'a self` / `&mut self` / `&'a mut self`。
    fn try_parse_self_param(&mut self) -> Option<ParamKind> {
        let is_self_at = |p: &Self, n: usize| {
            p.nth_kind(n) == Tk::Ident && {
                let tok = p.toks[(p.pos + n).min(p.toks.len() - 1)];
                p.token_text(tok) == "self"
            }
        };
        if self.at(Tk::And) {
            let mut n = 1;
            let mut lifetime = false;
            if self.nth_kind(n) == Tk::Lifetime {
                lifetime = true;
                n += 1;
            }
            let mut mutable = false;
            if self.nth_is_kw(n, Kw::Mut) {
                mutable = true;
                n += 1;
            }
            if !is_self_at(self, n) {
                return None;
            }
            self.bump(); // &
            let lt = if lifetime {
                let tok = self.bump();
                Some(Lifetime {
                    name: self.token_text(tok).trim_start_matches('\'').to_owned(),
                    span: tok.span,
                })
            } else {
                None
            };
            if mutable {
                self.bump();
            }
            self.bump(); // self
            return Some(ParamKind::SelfParam {
                by_ref: true,
                lifetime: lt,
                mutable,
            });
        }
        if self.at_kw(Kw::Mut) && is_self_at(self, 1) {
            self.bump();
            self.bump();
            return Some(ParamKind::SelfParam {
                by_ref: false,
                lifetime: None,
                mutable: true,
            });
        }
        if is_self_at(self, 0) {
            self.bump();
            return Some(ParamKind::SelfParam {
                by_ref: false,
                lifetime: None,
                mutable: false,
            });
        }
        None
    }

    // -- struct / enum(RXS-0015) -------------------------------------------

    fn parse_struct(&mut self) -> StructItem {
        let name = self.expect_ident("a struct name");
        let mut generics = self.parse_generic_params();
        generics.where_preds = self.parse_where_clause();
        let body = if self.at(Tk::OpenBrace) {
            VariantBody::Named(self.parse_field_defs())
        } else if self.at(Tk::OpenParen) {
            let fields = self.parse_tuple_fields();
            self.expect(Tk::Semi, "`;`");
            VariantBody::Tuple(fields)
        } else if self.eat(Tk::Semi) {
            VariantBody::Unit
        } else {
            self.error_expected("`{`, `(`, or `;`");
            VariantBody::Unit
        };
        StructItem {
            name,
            generics,
            body,
        }
    }

    fn parse_field_defs(&mut self) -> Vec<FieldDef> {
        let open = self.peek().span;
        self.expect(Tk::OpenBrace, "`{`");
        let mut fields = Vec::new();
        while !self.at(Tk::CloseBrace) {
            if self.at(Tk::Eof) {
                self.error_unclosed(open, "{");
                return fields;
            }
            let lo = self.lo();
            let attrs = self.parse_outer_attrs();
            let vis = self.parse_visibility();
            let name = self.expect_ident("a field name");
            self.expect(Tk::Colon, "`:`");
            let ty = self.parse_type();
            fields.push(FieldDef {
                attrs,
                vis,
                name,
                ty,
                span: self.span_from(lo),
            });
            if !self.eat(Tk::Comma) {
                break;
            }
        }
        self.expect(Tk::CloseBrace, "`}`");
        fields
    }

    fn parse_tuple_fields(&mut self) -> Vec<TupleField> {
        let open = self.peek().span;
        self.expect(Tk::OpenParen, "`(`");
        let mut fields = Vec::new();
        while !self.at(Tk::CloseParen) {
            if self.at(Tk::Eof) {
                self.error_unclosed(open, "(");
                return fields;
            }
            let lo = self.lo();
            let attrs = self.parse_outer_attrs();
            let vis = self.parse_visibility();
            let ty = self.parse_type();
            fields.push(TupleField {
                attrs,
                vis,
                ty,
                span: self.span_from(lo),
            });
            if !self.eat(Tk::Comma) {
                break;
            }
        }
        self.expect(Tk::CloseParen, "`)`");
        fields
    }

    fn parse_enum(&mut self) -> EnumItem {
        let name = self.expect_ident("an enum name");
        let mut generics = self.parse_generic_params();
        generics.where_preds = self.parse_where_clause();
        let open = self.peek().span;
        self.expect(Tk::OpenBrace, "`{`");
        let mut variants = Vec::new();
        while !self.at(Tk::CloseBrace) {
            if self.at(Tk::Eof) {
                self.error_unclosed(open, "{");
                break;
            }
            let lo = self.lo();
            let attrs = self.parse_outer_attrs();
            let vname = self.expect_ident("a variant name");
            let body = if self.at(Tk::OpenBrace) {
                VariantBody::Named(self.parse_field_defs())
            } else if self.at(Tk::OpenParen) {
                VariantBody::Tuple(self.parse_tuple_fields())
            } else {
                VariantBody::Unit
            };
            variants.push(Variant {
                attrs,
                name: vname,
                body,
                span: self.span_from(lo),
            });
            if !self.eat(Tk::Comma) {
                break;
            }
        }
        self.expect(Tk::CloseBrace, "`}`");
        EnumItem {
            name,
            generics,
            variants,
        }
    }

    // -- trait / impl(RXS-0016) --------------------------------------------

    fn parse_trait(&mut self) -> TraitItem {
        let name = self.expect_ident("a trait name");
        let mut generics = self.parse_generic_params();
        generics.where_preds = self.parse_where_clause();
        let items = self.parse_assoc_items(FnCtx::Trait);
        TraitItem {
            name,
            generics,
            items,
        }
    }

    fn parse_impl(&mut self) -> ImplItem {
        let mut generics = self.parse_generic_params();
        let first_ty = self.parse_type();
        let (trait_ty, self_ty) = if self.eat_kw(Kw::For) {
            (Some(first_ty), self.parse_type())
        } else {
            (None, first_ty)
        };
        generics.where_preds = self.parse_where_clause();
        let items = self.parse_assoc_items(FnCtx::Impl);
        ImplItem {
            generics,
            trait_ty,
            self_ty,
            items,
        }
    }

    fn parse_assoc_items(&mut self, ctx: FnCtx) -> Vec<AssocItem> {
        let open = self.peek().span;
        self.expect(Tk::OpenBrace, "`{`");
        let mut items = Vec::new();
        while !self.at(Tk::CloseBrace) {
            if self.at(Tk::Eof) {
                self.error_unclosed(open, "{");
                return items;
            }
            let lo = self.lo();
            let attrs = self.parse_outer_attrs();
            let vis = self.parse_visibility();
            let kind = match self.kind() {
                Tk::Kw(Kw::Fn) => {
                    self.bump();
                    AssocItemKind::Fn(self.parse_fn(FnColor::Host, ctx))
                }
                Tk::Kw(Kw::Kernel) => {
                    self.bump();
                    self.expect(Tk::Kw(Kw::Fn), "`fn`");
                    AssocItemKind::Fn(self.parse_fn(FnColor::Kernel, ctx))
                }
                Tk::Kw(Kw::Device) => {
                    self.bump();
                    self.expect(Tk::Kw(Kw::Fn), "`fn`");
                    AssocItemKind::Fn(self.parse_fn(FnColor::Device, ctx))
                }
                Tk::Kw(Kw::Const) if self.nth_is_kw(1, Kw::Fn) => {
                    self.bump();
                    self.bump();
                    AssocItemKind::Fn(self.parse_fn(FnColor::Const, ctx))
                }
                Tk::Kw(Kw::Const) => {
                    self.bump();
                    AssocItemKind::Const(self.parse_const_item())
                }
                Tk::Kw(Kw::Type) => {
                    self.bump();
                    let name = self.expect_ident("an associated type name");
                    let bounds = if self.eat(Tk::Colon) {
                        self.parse_bounds()
                    } else {
                        Vec::new()
                    };
                    let default = if self.eat(Tk::Eq) {
                        Some(self.parse_type())
                    } else {
                        None
                    };
                    self.expect(Tk::Semi, "`;`");
                    AssocItemKind::Type {
                        name,
                        bounds,
                        default,
                    }
                }
                _ => {
                    self.error_expected("an associated item (`fn`, `type`, or `const`)");
                    self.recover_to_sync();
                    continue;
                }
            };
            items.push(AssocItem {
                attrs,
                vis,
                kind,
                span: self.span_from(lo),
            });
        }
        self.expect(Tk::CloseBrace, "`}`");
        items
    }

    // -- mod / use / static / const / type / extern(RXS-0017 ~ RXS-0019) ---

    fn parse_mod(&mut self) -> ModItem {
        let name = self.expect_ident("a module name");
        let open = self.peek().span;
        self.expect(Tk::OpenBrace, "`{`");
        let mut items = Vec::new();
        while !self.at(Tk::CloseBrace) {
            if self.at(Tk::Eof) {
                self.error_unclosed(open, "{");
                return ModItem { name, items };
            }
            items.push(self.parse_item());
        }
        self.expect(Tk::CloseBrace, "`}`");
        ModItem { name, items }
    }

    fn parse_use(&mut self) -> UseItem {
        let path = self.parse_plain_path("a path");
        let alias = if self.eat_kw(Kw::As) {
            Some(self.expect_ident("an alias name"))
        } else {
            None
        };
        self.expect(Tk::Semi, "`;`");
        UseItem { path, alias }
    }

    fn parse_static(&mut self) -> StaticItem {
        let mutable = self.eat_kw(Kw::Mut);
        let name = self.expect_ident("a static name");
        self.expect(Tk::Colon, "`:`");
        let ty = self.parse_type();
        self.expect(Tk::Eq, "`=`");
        let init = self.parse_expr();
        self.expect(Tk::Semi, "`;`");
        StaticItem {
            mutable,
            name,
            ty,
            init,
        }
    }

    fn parse_const_item(&mut self) -> ConstItem {
        let name = self.expect_ident("a const name");
        self.expect(Tk::Colon, "`:`");
        let ty = self.parse_type();
        self.expect(Tk::Eq, "`=`");
        let init = self.parse_expr();
        self.expect(Tk::Semi, "`;`");
        ConstItem { name, ty, init }
    }

    fn parse_type_alias(&mut self) -> TypeAlias {
        let name = self.expect_ident("a type alias name");
        let generics = self.parse_generic_params();
        self.expect(Tk::Eq, "`=`");
        let ty = self.parse_type();
        self.expect(Tk::Semi, "`;`");
        TypeAlias { name, generics, ty }
    }

    fn parse_extern_block(&mut self) -> ExternBlock {
        let (abi, abi_span) = if self.at(Tk::StrLit) {
            let tok = self.bump();
            let text = self.token_text(tok);
            (text.trim_matches('"').to_owned(), tok.span)
        } else {
            self.error_expected("an ABI string (`\"C\"`)");
            (String::new(), self.peek().span)
        };
        let open = self.peek().span;
        self.expect(Tk::OpenBrace, "`{`");
        let mut items = Vec::new();
        while !self.at(Tk::CloseBrace) {
            if self.at(Tk::Eof) {
                self.error_unclosed(open, "{");
                return ExternBlock {
                    abi,
                    abi_span,
                    items,
                };
            }
            let lo = self.lo();
            let attrs = self.parse_outer_attrs();
            let vis = self.parse_visibility();
            let kind = if self.eat_kw(Kw::Fn) {
                ItemKind::Fn(self.parse_fn(FnColor::Host, FnCtx::Extern))
            } else {
                self.error_expected("`fn`");
                self.recover_to_sync();
                ItemKind::Err
            };
            items.push(Item {
                attrs,
                vis,
                kind,
                span: self.span_from(lo),
            });
        }
        self.expect(Tk::CloseBrace, "`}`");
        ExternBlock {
            abi,
            abi_span,
            items,
        }
    }

    // -- 泛型(RXS-0020 / RXS-0021) -----------------------------------------

    fn parse_generic_params(&mut self) -> Generics {
        let mut generics = Generics::default();
        if !self.eat(Tk::Lt) {
            return generics;
        }
        while !self.at_gt_like() {
            if self.at(Tk::Eof) {
                self.error_expected("`>`");
                return generics;
            }
            let lo = self.lo();
            let kind = match self.kind() {
                Tk::Lifetime => {
                    let tok = self.bump();
                    GenericParamKind::Lifetime(Lifetime {
                        name: self.token_text(tok).trim_start_matches('\'').to_owned(),
                        span: tok.span,
                    })
                }
                Tk::Kw(Kw::Const) => {
                    self.bump();
                    let name = self.expect_ident("a const parameter name");
                    // RXS-0020:const 泛型参数必须带类型标注
                    self.expect(Tk::Colon, "`:`");
                    let ty = self.parse_type();
                    GenericParamKind::Const { name, ty }
                }
                Tk::Ident => {
                    let name = self.make_ident();
                    let bounds = if self.eat(Tk::Colon) {
                        self.parse_bounds()
                    } else {
                        Vec::new()
                    };
                    let default = if self.eat(Tk::Eq) {
                        Some(self.parse_type())
                    } else {
                        None
                    };
                    GenericParamKind::Type {
                        name,
                        bounds,
                        default,
                    }
                }
                _ => {
                    self.error_expected("a generic parameter");
                    self.bump();
                    continue;
                }
            };
            generics.params.push(GenericParam {
                kind,
                span: self.span_from(lo),
            });
            if !self.eat(Tk::Comma) {
                break;
            }
        }
        self.expect_gt();
        generics
    }

    fn parse_bounds(&mut self) -> Vec<Bound> {
        let mut bounds = Vec::new();
        loop {
            if self.at(Tk::Lifetime) {
                let tok = self.bump();
                bounds.push(Bound::Lifetime(Lifetime {
                    name: self.token_text(tok).trim_start_matches('\'').to_owned(),
                    span: tok.span,
                }));
            } else if self.at_ident() {
                bounds.push(Bound::Trait(self.parse_path_in_type()));
            } else {
                self.error_expected("a trait or lifetime bound");
                break;
            }
            if !self.eat(Tk::Plus) {
                break;
            }
        }
        bounds
    }

    fn parse_where_clause(&mut self) -> Vec<WherePred> {
        let mut preds = Vec::new();
        if !self.eat_kw(Kw::Where) {
            return preds;
        }
        loop {
            if matches!(self.kind(), Tk::OpenBrace | Tk::Semi | Tk::Eof) {
                break;
            }
            let lo = self.lo();
            let ty = self.parse_type();
            self.expect(Tk::Colon, "`:`");
            let bounds = self.parse_bounds();
            preds.push(WherePred {
                ty,
                bounds,
                span: self.span_from(lo),
            });
            if !self.eat(Tk::Comma) {
                break;
            }
        }
        preds
    }

    /// 泛型实参闭合检测:`>` 及含 `>` 前缀的复合 token(RXS-0021 拆分)。
    fn at_gt_like(&self) -> bool {
        matches!(self.kind(), Tk::Gt | Tk::Shr | Tk::Ge | Tk::ShrEq)
    }

    /// 消费一个 `>`;复合 token 就地拆分(RXS-0021):
    /// `>>` → `>` + `>`,`>=` → `>` + `=`,`>>=` → `>` + `>=`。
    fn expect_gt(&mut self) {
        let idx = self.pos.min(self.toks.len() - 1);
        let tok = self.toks[idx];
        let rest = match tok.kind {
            Tk::Gt => {
                self.bump();
                return;
            }
            Tk::Shr => Tk::Gt,
            Tk::Ge => Tk::Eq,
            Tk::ShrEq => Tk::Ge,
            _ => {
                self.error_expected("`>`");
                return;
            }
        };
        let lo = tok.span.lo.0;
        self.events.push(ParseEvent::Token(Span::new(
            self.file,
            lo,
            lo + 1,
            self.edition,
        ))); // STUB(RD-004)
        self.toks[idx] = Token {
            kind: rest,
            span: Span::new(self.file, lo + 1, tok.span.hi.0, self.edition),
        };
    }

    fn parse_generic_args(&mut self) -> GenericArgs {
        // 调用方已消费 `<`(或 `::<` 的 `<`)
        let lo = self.prev_hi().saturating_sub(1);
        let mut args = Vec::new();
        while !self.at_gt_like() {
            if self.at(Tk::Eof) {
                self.error_expected("`>`");
                return GenericArgs {
                    args,
                    span: self.span_from(lo),
                };
            }
            let arg = match self.kind() {
                Tk::Lifetime => {
                    let tok = self.bump();
                    GenericArg::Lifetime(Lifetime {
                        name: self.token_text(tok).trim_start_matches('\'').to_owned(),
                        span: tok.span,
                    })
                }
                Tk::OpenBrace => {
                    // const 实参块形态(RXS-0021)
                    let block_lo = self.lo();
                    let block = self.with_no_struct(false, |p| p.parse_block());
                    GenericArg::Const(Expr {
                        attrs: Vec::new(),
                        kind: ExprKind::Block(block),
                        span: self.span_from(block_lo),
                    })
                }
                Tk::Minus => {
                    let neg_lo = self.lo();
                    self.bump();
                    if let Some(lit) = self.try_parse_lit() {
                        let lit_span = lit.span;
                        GenericArg::Const(Expr {
                            attrs: Vec::new(),
                            kind: ExprKind::Unary {
                                op: UnOp::Neg,
                                expr: Box::new(Expr {
                                    attrs: Vec::new(),
                                    kind: ExprKind::Lit(lit),
                                    span: lit_span,
                                }),
                            },
                            span: self.span_from(neg_lo),
                        })
                    } else {
                        self.error_expected("an integer literal");
                        continue;
                    }
                }
                _ => GenericArg::Type(self.parse_type()),
            };
            args.push(arg);
            if !self.eat(Tk::Comma) {
                break;
            }
        }
        self.expect_gt();
        GenericArgs {
            args,
            span: self.span_from(lo),
        }
    }

    // -- 路径(RXS-0013) ----------------------------------------------------

    /// 不带泛型实参的纯路径(use / 属性 / 模式)。
    fn parse_plain_path(&mut self, expected: &str) -> Path {
        let lo = self.lo();
        let mut segments = Vec::new();
        segments.push(PathSegment {
            ident: self.expect_ident(expected),
            args: None,
        });
        while self.at(Tk::PathSep) && self.nth_kind(1) == Tk::Ident {
            self.bump();
            segments.push(PathSegment {
                ident: self.make_ident(),
                args: None,
            });
        }
        Path {
            segments,
            span: self.span_from(lo),
        }
    }

    /// 类型位置路径:段后可直接跟 `<…>`(RXS-0013)。
    fn parse_path_in_type(&mut self) -> Path {
        let lo = self.lo();
        let mut segments = Vec::new();
        loop {
            let ident = self.expect_ident("a path segment");
            let args = if self.eat(Tk::Lt) {
                Some(self.parse_generic_args())
            } else {
                None
            };
            segments.push(PathSegment { ident, args });
            if !(self.at(Tk::PathSep) && self.nth_kind(1) == Tk::Ident) {
                break;
            }
            self.bump();
        }
        Path {
            segments,
            span: self.span_from(lo),
        }
    }

    /// 表达式位置路径:泛型实参须经 turbofish `::<…>`(RXS-0013)。
    fn parse_path_in_expr(&mut self) -> Path {
        let lo = self.lo();
        let mut segments = Vec::new();
        loop {
            let ident = self.expect_ident("a path segment");
            let mut args = None;
            if self.at(Tk::PathSep) && self.nth_kind(1) == Tk::Lt {
                self.bump(); // ::
                self.bump(); // <
                args = Some(self.parse_generic_args());
            }
            segments.push(PathSegment { ident, args });
            if !(self.at(Tk::PathSep) && self.nth_kind(1) == Tk::Ident) {
                break;
            }
            self.bump();
        }
        Path {
            segments,
            span: self.span_from(lo),
        }
    }

    // -- 类型(RXS-0022) ----------------------------------------------------

    fn parse_type(&mut self) -> Ty {
        self.events.push(ParseEvent::Start(NodeKind::Ty)); // STUB(RD-004)
        let ty = self.parse_type_inner();
        self.events.push(ParseEvent::Finish(NodeKind::Ty)); // STUB(RD-004)
        ty
    }

    fn parse_type_inner(&mut self) -> Ty {
        let lo = self.lo();
        let kind = match self.kind() {
            Tk::AndAnd => {
                // `&&T`:最长匹配产出的 AndAnd 在类型位置等于两层引用(RXS-0022)
                self.bump();
                let inner_lo = self.prev_hi().saturating_sub(1);
                let lifetime = if self.at(Tk::Lifetime) {
                    let tok = self.bump();
                    Some(Lifetime {
                        name: self.token_text(tok).trim_start_matches('\'').to_owned(),
                        span: tok.span,
                    })
                } else {
                    None
                };
                let mutable = self.eat_kw(Kw::Mut);
                let inner = Ty {
                    kind: TyKind::Ref {
                        lifetime,
                        mutable,
                        inner: Box::new(self.parse_type_inner()),
                    },
                    span: self.span_from(inner_lo),
                };
                TyKind::Ref {
                    lifetime: None,
                    mutable: false,
                    inner: Box::new(inner),
                }
            }
            Tk::And => {
                self.bump();
                let lifetime = if self.at(Tk::Lifetime) {
                    let tok = self.bump();
                    Some(Lifetime {
                        name: self.token_text(tok).trim_start_matches('\'').to_owned(),
                        span: tok.span,
                    })
                } else {
                    None
                };
                let mutable = self.eat_kw(Kw::Mut);
                TyKind::Ref {
                    lifetime,
                    mutable,
                    inner: Box::new(self.parse_type_inner()),
                }
            }
            Tk::Star => {
                self.bump();
                let mutable = if self.eat_kw(Kw::Mut) {
                    true
                } else if self.eat_kw(Kw::Const) {
                    false
                } else {
                    // RXS-0022:`*` 后必须 const / mut
                    self.error_expected("`const` or `mut`");
                    false
                };
                TyKind::RawPtr {
                    mutable,
                    inner: Box::new(self.parse_type_inner()),
                }
            }
            Tk::OpenParen => {
                let open = self.peek().span;
                self.bump();
                if self.eat(Tk::CloseParen) {
                    TyKind::Tuple(Vec::new())
                } else {
                    let first = self.parse_type_inner();
                    if self.eat(Tk::Comma) {
                        let mut elems = vec![first];
                        while !self.at(Tk::CloseParen) {
                            if self.at(Tk::Eof) {
                                self.error_unclosed(open, "(");
                                return Ty {
                                    kind: TyKind::Tuple(elems),
                                    span: self.span_from(lo),
                                };
                            }
                            elems.push(self.parse_type_inner());
                            if !self.eat(Tk::Comma) {
                                break;
                            }
                        }
                        self.expect(Tk::CloseParen, "`)`");
                        TyKind::Tuple(elems)
                    } else {
                        self.expect(Tk::CloseParen, "`)`");
                        TyKind::Paren(Box::new(first))
                    }
                }
            }
            Tk::OpenBracket => {
                let open = self.peek().span;
                self.bump();
                let elem = self.parse_type_inner();
                let kind = if self.eat(Tk::Semi) {
                    let len = self.with_no_struct(false, |p| p.parse_expr());
                    TyKind::Array {
                        elem: Box::new(elem),
                        len: Box::new(len),
                    }
                } else {
                    TyKind::Slice(Box::new(elem))
                };
                if !self.eat(Tk::CloseBracket) {
                    if self.at(Tk::Eof) {
                        self.error_unclosed(open, "[");
                    } else {
                        self.error_expected("`]`");
                    }
                }
                kind
            }
            Tk::Kw(Kw::Fn) => {
                self.bump();
                self.expect(Tk::OpenParen, "`(`");
                let mut params = Vec::new();
                while !self.at(Tk::CloseParen) {
                    if self.at(Tk::Eof) {
                        self.error_expected("`)`");
                        break;
                    }
                    params.push(self.parse_type_inner());
                    if !self.eat(Tk::Comma) {
                        break;
                    }
                }
                self.expect(Tk::CloseParen, "`)`");
                let ret = if self.eat(Tk::Arrow) {
                    Some(Box::new(self.parse_type_inner()))
                } else {
                    None
                };
                TyKind::FnPtr { params, ret }
            }
            Tk::Underscore => {
                self.bump();
                TyKind::Infer
            }
            Tk::IntLit { .. } => {
                // 类型位置 const 实参形态(RXS-0021/0022)
                let tok = self.bump();
                TyKind::ConstArg(Lit {
                    kind: LitKind::Int,
                    suffix: None,
                    span: tok.span,
                })
            }
            Tk::Ident => TyKind::Path(self.parse_path_in_type()),
            _ => {
                self.error_expected("a type");
                TyKind::Err
            }
        };
        Ty {
            kind,
            span: self.span_from(lo),
        }
    }

    // -- 模式(RXS-0023) ----------------------------------------------------

    fn parse_pattern(&mut self) -> Pat {
        self.events.push(ParseEvent::Start(NodeKind::Pat)); // STUB(RD-004)
        let pat = self.parse_pattern_inner();
        self.events.push(ParseEvent::Finish(NodeKind::Pat)); // STUB(RD-004)
        pat
    }

    fn parse_pattern_inner(&mut self) -> Pat {
        let lo = self.lo();
        let kind = match self.kind() {
            Tk::Underscore => {
                self.bump();
                PatKind::Wild
            }
            Tk::Kw(Kw::Mut) => {
                self.bump();
                let name = self.expect_ident("a binding name");
                PatKind::Binding {
                    mutable: true,
                    name,
                }
            }
            Tk::And => {
                self.bump();
                let mutable = self.eat_kw(Kw::Mut);
                PatKind::Ref {
                    mutable,
                    pat: Box::new(self.parse_pattern_inner()),
                }
            }
            Tk::OpenParen => {
                let open = self.peek().span;
                self.bump();
                if self.eat(Tk::CloseParen) {
                    PatKind::Tuple(Vec::new())
                } else {
                    let first = self.parse_pattern_inner();
                    if self.eat(Tk::Comma) {
                        let mut elems = vec![first];
                        while !self.at(Tk::CloseParen) {
                            if self.at(Tk::Eof) {
                                self.error_unclosed(open, "(");
                                return Pat {
                                    kind: PatKind::Tuple(elems),
                                    span: self.span_from(lo),
                                };
                            }
                            elems.push(self.parse_pattern_inner());
                            if !self.eat(Tk::Comma) {
                                break;
                            }
                        }
                        self.expect(Tk::CloseParen, "`)`");
                        PatKind::Tuple(elems)
                    } else {
                        self.expect(Tk::CloseParen, "`)`");
                        return first;
                    }
                }
            }
            Tk::OpenBracket => {
                let open = self.peek().span;
                self.bump();
                let mut elems = Vec::new();
                while !self.at(Tk::CloseBracket) {
                    if self.at(Tk::Eof) {
                        self.error_unclosed(open, "[");
                        break;
                    }
                    elems.push(self.parse_pattern_inner());
                    if !self.eat(Tk::Comma) {
                        break;
                    }
                }
                self.expect(Tk::CloseBracket, "`]`");
                PatKind::Slice(elems)
            }
            Tk::Minus
            | Tk::IntLit { .. }
            | Tk::FloatLit { .. }
            | Tk::StrLit
            | Tk::CharLit
            | Tk::Kw(Kw::True)
            | Tk::Kw(Kw::False) => return self.parse_lit_or_range_pattern(),
            Tk::Ident => {
                // `name @ pat`(单段绑定 + at)
                if self.nth_kind(1) == Tk::At {
                    let name = self.make_ident();
                    self.bump(); // @
                    let sub = self.parse_pattern_inner();
                    return Pat {
                        kind: PatKind::At {
                            name,
                            pat: Box::new(sub),
                        },
                        span: self.span_from(lo),
                    };
                }
                let path = self.parse_plain_path("a pattern");
                if self.at(Tk::OpenParen) {
                    let open = self.peek().span;
                    self.bump();
                    let mut elems = Vec::new();
                    while !self.at(Tk::CloseParen) {
                        if self.at(Tk::Eof) {
                            self.error_unclosed(open, "(");
                            break;
                        }
                        elems.push(self.parse_pattern_inner());
                        if !self.eat(Tk::Comma) {
                            break;
                        }
                    }
                    self.expect(Tk::CloseParen, "`)`");
                    PatKind::TupleStruct { path, elems }
                } else if self.at(Tk::OpenBrace) {
                    let (fields, rest) = self.parse_struct_pattern_fields();
                    PatKind::Struct { path, fields, rest }
                } else if path.segments.len() == 1 {
                    let seg = path.segments.into_iter().next().unwrap();
                    PatKind::Binding {
                        mutable: false,
                        name: seg.ident,
                    }
                } else {
                    PatKind::Path(path)
                }
            }
            _ => {
                self.error_expected("a pattern");
                PatKind::Err
            }
        };
        Pat {
            kind,
            span: self.span_from(lo),
        }
    }

    fn parse_struct_pattern_fields(&mut self) -> (Vec<FieldPat>, bool) {
        let open = self.peek().span;
        self.expect(Tk::OpenBrace, "`{`");
        let mut fields = Vec::new();
        let mut rest = false;
        while !self.at(Tk::CloseBrace) {
            if self.at(Tk::Eof) {
                self.error_unclosed(open, "{");
                return (fields, rest);
            }
            if self.eat(Tk::DotDot) {
                // rest 模式仅尾部(RXS-0023)
                rest = true;
                self.eat(Tk::Comma);
                break;
            }
            let lo = self.lo();
            let name = self.expect_ident("a field name");
            let pat = if self.eat(Tk::Colon) {
                Some(self.parse_pattern_inner())
            } else {
                None
            };
            fields.push(FieldPat {
                name,
                pat,
                span: self.span_from(lo),
            });
            if !self.eat(Tk::Comma) {
                break;
            }
        }
        self.expect(Tk::CloseBrace, "`}`");
        (fields, rest)
    }

    fn parse_lit_or_range_pattern(&mut self) -> Pat {
        let lo = self.lo();
        let first = self.parse_lit_pattern();
        if matches!(self.kind(), Tk::DotDotEq | Tk::DotDot) {
            let inclusive = self.at(Tk::DotDotEq);
            self.bump();
            // RXS-0023:范围模式两端必须是字面量形态
            let hi = if matches!(
                self.kind(),
                Tk::Minus
                    | Tk::IntLit { .. }
                    | Tk::FloatLit { .. }
                    | Tk::StrLit
                    | Tk::CharLit
                    | Tk::Kw(Kw::True)
                    | Tk::Kw(Kw::False)
            ) {
                self.parse_lit_pattern()
            } else {
                self.error_expected("a literal (range pattern bounds must be literals)");
                Pat {
                    kind: PatKind::Err,
                    span: self.peek().span,
                }
            };
            return Pat {
                kind: PatKind::Range {
                    lo: Box::new(first),
                    hi: Box::new(hi),
                    inclusive,
                },
                span: self.span_from(lo),
            };
        }
        first
    }

    fn parse_lit_pattern(&mut self) -> Pat {
        let lo = self.lo();
        let negated = self.eat(Tk::Minus);
        let kind = match self.try_parse_lit() {
            Some(lit) => PatKind::Lit { negated, lit },
            None => {
                self.error_expected("a literal");
                PatKind::Err
            }
        };
        Pat {
            kind,
            span: self.span_from(lo),
        }
    }

    fn try_parse_lit(&mut self) -> Option<Lit> {
        use crate::lexer::{FloatSuffix, IntSuffix};
        let (kind, suffix) = match self.kind() {
            Tk::IntLit { suffix, .. } => (
                LitKind::Int,
                suffix.map(|s| match s {
                    IntSuffix::I8 => LitSuffix::I8,
                    IntSuffix::I16 => LitSuffix::I16,
                    IntSuffix::I32 => LitSuffix::I32,
                    IntSuffix::I64 => LitSuffix::I64,
                    IntSuffix::U8 => LitSuffix::U8,
                    IntSuffix::U16 => LitSuffix::U16,
                    IntSuffix::U32 => LitSuffix::U32,
                    IntSuffix::U64 => LitSuffix::U64,
                    IntSuffix::Usize => LitSuffix::Usize,
                }),
            ),
            Tk::FloatLit { suffix } => (
                LitKind::Float,
                suffix.map(|s| match s {
                    FloatSuffix::F32 => LitSuffix::F32,
                    FloatSuffix::F64 => LitSuffix::F64,
                }),
            ),
            Tk::StrLit => (LitKind::Str, None),
            Tk::CharLit => (LitKind::Char, None),
            Tk::Kw(Kw::True) => (LitKind::Bool(true), None),
            Tk::Kw(Kw::False) => (LitKind::Bool(false), None),
            _ => return None,
        };
        let tok = self.bump();
        Some(Lit {
            kind,
            suffix,
            span: tok.span,
        })
    }

    // -- 语句与块(RXS-0024) ------------------------------------------------

    fn parse_block(&mut self) -> Block {
        self.events.push(ParseEvent::Start(NodeKind::Block)); // STUB(RD-004)
        let block = self.with_no_struct(false, |p| p.parse_block_inner());
        self.events.push(ParseEvent::Finish(NodeKind::Block)); // STUB(RD-004)
        block
    }

    fn parse_block_inner(&mut self) -> Block {
        let lo = self.lo();
        let open = self.peek().span;
        self.expect(Tk::OpenBrace, "`{`");
        let mut stmts = Vec::new();
        let mut tail: Option<Box<Expr>> = None;
        while !self.at(Tk::CloseBrace) {
            if self.at(Tk::Eof) {
                self.error_unclosed(open, "{");
                return Block {
                    stmts,
                    tail,
                    span: self.span_from(lo),
                };
            }
            let stmt_lo = self.lo();
            match self.kind() {
                Tk::Semi => {
                    self.bump();
                    stmts.push(Stmt {
                        kind: StmtKind::Empty,
                        span: self.span_from(stmt_lo),
                    });
                }
                Tk::Kw(Kw::Let) => {
                    let stmt = self.parse_let_stmt(false, stmt_lo);
                    stmts.push(stmt);
                }
                Tk::Kw(Kw::Shared) => {
                    self.bump();
                    let stmt = self.parse_let_stmt(true, stmt_lo);
                    stmts.push(stmt);
                }
                // item 语句(`const` 的 item/`const fn` 形态均为 item)
                k if Self::is_item_start(k) && k != Tk::Pound => {
                    let item = self.parse_item();
                    stmts.push(Stmt {
                        span: item.span,
                        kind: StmtKind::Item(Box::new(item)),
                    });
                }
                _ => {
                    // 块形态表达式作语句:不延伸后缀/二元(RXS-0024;与 Rust 同策略,
                    // 防 `loop { … }` 后的 `-x;` 被误并为二元减法)
                    let block_like_start = matches!(
                        self.kind(),
                        Tk::OpenBrace
                            | Tk::Kw(
                                Kw::If | Kw::While | Kw::For | Kw::Loop | Kw::Match | Kw::Unsafe
                            )
                    );
                    let expr = if block_like_start {
                        self.events.push(ParseEvent::Start(NodeKind::Expr)); // STUB(RD-004)
                        let e = self.parse_primary_core(stmt_lo);
                        self.events.push(ParseEvent::Finish(NodeKind::Expr)); // STUB(RD-004)
                        e
                    } else {
                        self.parse_expr()
                    };
                    if self.eat(Tk::Semi) {
                        stmts.push(Stmt {
                            kind: StmtKind::Expr { expr, semi: true },
                            span: self.span_from(stmt_lo),
                        });
                    } else if self.at(Tk::CloseBrace) {
                        tail = Some(Box::new(expr));
                    } else if expr.is_block_like() {
                        stmts.push(Stmt {
                            kind: StmtKind::Expr { expr, semi: false },
                            span: self.span_from(stmt_lo),
                        });
                    } else {
                        self.error_expected("`;`");
                        stmts.push(Stmt {
                            kind: StmtKind::Expr { expr, semi: false },
                            span: self.span_from(stmt_lo),
                        });
                        self.recover_in_block();
                    }
                }
            }
        }
        self.expect(Tk::CloseBrace, "`}`");
        Block {
            stmts,
            tail,
            span: self.span_from(lo),
        }
    }

    /// 块内语句级恢复:跳到 `;`(消费)/ `}`(不消费)/ 语句起始 / EOF。
    fn recover_in_block(&mut self) {
        let mut depth = 0u32;
        loop {
            match self.kind() {
                Tk::Eof => return,
                Tk::OpenParen | Tk::OpenBracket | Tk::OpenBrace => {
                    depth += 1;
                    self.bump();
                }
                Tk::CloseParen | Tk::CloseBracket => {
                    depth = depth.saturating_sub(1);
                    self.bump();
                }
                Tk::CloseBrace => {
                    if depth == 0 {
                        return;
                    }
                    depth -= 1;
                    self.bump();
                }
                Tk::Semi if depth == 0 => {
                    self.bump();
                    return;
                }
                Tk::Kw(Kw::Let) if depth == 0 => return,
                k if depth == 0 && Self::is_item_start(k) => return,
                _ => {
                    self.bump();
                }
            }
        }
    }

    fn parse_let_stmt(&mut self, shared: bool, lo: u32) -> Stmt {
        self.expect(Tk::Kw(Kw::Let), "`let`");
        let pat = self.parse_pattern();
        let ty = if self.eat(Tk::Colon) {
            Some(self.parse_type())
        } else {
            None
        };
        let init = if self.eat(Tk::Eq) {
            Some(self.parse_expr())
        } else {
            None
        };
        if !self.eat(Tk::Semi) {
            self.error_expected("`;`");
            self.recover_in_block();
        }
        Stmt {
            kind: StmtKind::Let(LetStmt {
                shared,
                pat,
                ty,
                init,
            }),
            span: self.span_from(lo),
        }
    }

    // -- 表达式(RXS-0025 ~ RXS-0029) ----------------------------------------

    pub(crate) fn parse_expr(&mut self) -> Expr {
        self.events.push(ParseEvent::Start(NodeKind::Expr)); // STUB(RD-004)
        let expr = self.parse_assign_expr();
        self.events.push(ParseEvent::Finish(NodeKind::Expr)); // STUB(RD-004)
        expr
    }

    fn parse_expr_no_struct(&mut self) -> Expr {
        self.with_no_struct(true, |p| p.parse_expr())
    }

    /// 级 14:赋值(右结合,RXS-0025)。
    fn parse_assign_expr(&mut self) -> Expr {
        let lo = self.lo();
        let lhs = self.parse_range_expr();
        let op = match self.kind() {
            Tk::Eq => None,
            Tk::PlusEq => Some(BinOp::Add),
            Tk::MinusEq => Some(BinOp::Sub),
            Tk::StarEq => Some(BinOp::Mul),
            Tk::SlashEq => Some(BinOp::Div),
            Tk::PercentEq => Some(BinOp::Rem),
            Tk::AndEq => Some(BinOp::BitAnd),
            Tk::OrEq => Some(BinOp::BitOr),
            Tk::CaretEq => Some(BinOp::BitXor),
            Tk::ShlEq => Some(BinOp::Shl),
            Tk::ShrEq => Some(BinOp::Shr),
            _ => return lhs,
        };
        self.bump();
        let rhs = self.parse_assign_expr();
        Expr {
            attrs: Vec::new(),
            kind: ExprKind::Assign {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            },
            span: self.span_from(lo),
        }
    }

    /// 级 13:区间(不可链式,RXS-0025)。
    fn parse_range_expr(&mut self) -> Expr {
        let lo = self.lo();
        let first = self.parse_binary_expr(0);
        if !matches!(self.kind(), Tk::DotDot | Tk::DotDotEq) {
            return first;
        }
        let inclusive = self.at(Tk::DotDotEq);
        self.bump();
        let hi = self.parse_binary_expr(0);
        if matches!(self.kind(), Tk::DotDot | Tk::DotDotEq) {
            // 区间不可链式(RXS-0025)
            self.error_expected("an operand (range operators cannot be chained)");
        }
        Expr {
            attrs: Vec::new(),
            kind: ExprKind::Range {
                lo: Box::new(first),
                hi: Box::new(hi),
                inclusive,
            },
            span: self.span_from(lo),
        }
    }

    /// 级 4 ~ 12:二元运算(Pratt;比较不可链式)。
    fn parse_binary_expr(&mut self, min_prec: u8) -> Expr {
        let lo = self.lo();
        let mut lhs = self.parse_cast_expr();
        loop {
            let Some((op, prec, nonassoc)) = binop_of(self.kind()) else {
                break;
            };
            if prec < min_prec {
                break;
            }
            self.bump();
            let rhs = self.parse_binary_expr(prec + 1);
            lhs = Expr {
                attrs: Vec::new(),
                kind: ExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                span: self.span_from(lo),
            };
            if nonassoc
                && let Some((_, p2, _)) = binop_of(self.kind())
                && p2 == prec
            {
                // 比较不可链式(RXS-0025)
                self.error_expected(
                    "an operand (comparison operators cannot be chained; use parentheses)",
                );
            }
        }
        lhs
    }

    /// 级 3:`as` 转换。
    fn parse_cast_expr(&mut self) -> Expr {
        let lo = self.lo();
        let mut e = self.parse_unary_expr();
        while self.eat_kw(Kw::As) {
            let ty = self.parse_type();
            e = Expr {
                attrs: Vec::new(),
                kind: ExprKind::Cast {
                    expr: Box::new(e),
                    ty,
                },
                span: self.span_from(lo),
            };
        }
        e
    }

    /// 级 2:一元前缀。
    fn parse_unary_expr(&mut self) -> Expr {
        let lo = self.lo();
        let kind = match self.kind() {
            Tk::Minus => {
                self.bump();
                ExprKind::Unary {
                    op: UnOp::Neg,
                    expr: Box::new(self.parse_unary_expr()),
                }
            }
            Tk::Not => {
                self.bump();
                ExprKind::Unary {
                    op: UnOp::Not,
                    expr: Box::new(self.parse_unary_expr()),
                }
            }
            Tk::Star => {
                self.bump();
                ExprKind::Unary {
                    op: UnOp::Deref,
                    expr: Box::new(self.parse_unary_expr()),
                }
            }
            Tk::And => {
                self.bump();
                let mutable = self.eat_kw(Kw::Mut);
                ExprKind::Borrow {
                    mutable,
                    expr: Box::new(self.parse_unary_expr()),
                }
            }
            Tk::AndAnd => {
                // `&&expr` = 双重借用
                self.bump();
                let mutable = self.eat_kw(Kw::Mut);
                let inner_lo = self.prev_hi();
                let inner = Expr {
                    attrs: Vec::new(),
                    kind: ExprKind::Borrow {
                        mutable,
                        expr: Box::new(self.parse_unary_expr()),
                    },
                    span: self.span_from(inner_lo),
                };
                ExprKind::Borrow {
                    mutable: false,
                    expr: Box::new(inner),
                }
            }
            _ => return self.parse_postfix_expr(),
        };
        Expr {
            attrs: Vec::new(),
            kind,
            span: self.span_from(lo),
        }
    }

    /// 级 1:后缀(调用/索引/字段/方法/`?`,RXS-0027)。
    fn parse_postfix_expr(&mut self) -> Expr {
        let lo = self.lo();
        let mut e = self.parse_primary_expr();
        loop {
            match self.kind() {
                Tk::OpenParen => {
                    let args = self.parse_call_args();
                    e = Expr {
                        attrs: Vec::new(),
                        kind: ExprKind::Call {
                            callee: Box::new(e),
                            args,
                        },
                        span: self.span_from(lo),
                    };
                }
                Tk::OpenBracket => {
                    let open = self.peek().span;
                    self.bump();
                    let index = self.with_no_struct(false, |p| p.parse_expr());
                    if !self.eat(Tk::CloseBracket) {
                        if self.at(Tk::Eof) {
                            self.error_unclosed(open, "[");
                        } else {
                            self.error_expected("`]`");
                        }
                    }
                    e = Expr {
                        attrs: Vec::new(),
                        kind: ExprKind::Index {
                            expr: Box::new(e),
                            index: Box::new(index),
                        },
                        span: self.span_from(lo),
                    };
                }
                Tk::Question => {
                    self.bump();
                    e = Expr {
                        attrs: Vec::new(),
                        kind: ExprKind::Try(Box::new(e)),
                        span: self.span_from(lo),
                    };
                }
                Tk::Dot => {
                    self.bump();
                    if self.at_int() {
                        let tok = self.bump();
                        let index = self.token_text(tok).parse::<u32>().unwrap_or(u32::MAX);
                        e = Expr {
                            attrs: Vec::new(),
                            kind: ExprKind::TupleField {
                                expr: Box::new(e),
                                index,
                                index_span: tok.span,
                            },
                            span: self.span_from(lo),
                        };
                    } else if self.at_ident() {
                        let method = self.make_ident();
                        let mut generic_args = None;
                        if self.at(Tk::PathSep) && self.nth_kind(1) == Tk::Lt {
                            self.bump();
                            self.bump();
                            generic_args = Some(self.parse_generic_args());
                        }
                        if self.at(Tk::OpenParen) {
                            let args = self.parse_call_args();
                            e = Expr {
                                attrs: Vec::new(),
                                kind: ExprKind::MethodCall {
                                    receiver: Box::new(e),
                                    method,
                                    generic_args,
                                    args,
                                },
                                span: self.span_from(lo),
                            };
                        } else if generic_args.is_some() {
                            // turbofish 后必须是调用(RXS-0027)
                            self.error_expected("`(`");
                            e = Expr {
                                attrs: Vec::new(),
                                kind: ExprKind::MethodCall {
                                    receiver: Box::new(e),
                                    method,
                                    generic_args,
                                    args: Vec::new(),
                                },
                                span: self.span_from(lo),
                            };
                        } else {
                            e = Expr {
                                attrs: Vec::new(),
                                kind: ExprKind::Field {
                                    expr: Box::new(e),
                                    field: method,
                                },
                                span: self.span_from(lo),
                            };
                        }
                    } else {
                        self.error_expected("a field or method name");
                        break;
                    }
                }
                _ => break,
            }
        }
        e
    }

    fn parse_call_args(&mut self) -> Vec<Expr> {
        let open = self.peek().span;
        self.expect(Tk::OpenParen, "`(`");
        let mut args = Vec::new();
        self.with_no_struct(false, |p| {
            while !p.at(Tk::CloseParen) {
                if p.at(Tk::Eof) {
                    p.error_unclosed(open, "(");
                    return;
                }
                args.push(p.parse_expr());
                if !p.eat(Tk::Comma) {
                    break;
                }
            }
            p.expect(Tk::CloseParen, "`)`");
        });
        args
    }

    /// 基本表达式(RXS-0026 / RXS-0028 / RXS-0029)。
    fn parse_primary_expr(&mut self) -> Expr {
        let lo = self.lo();
        // 表达式前置外部属性(RXS-0026)
        let attrs = if self.at(Tk::Pound) {
            self.parse_outer_attrs()
        } else {
            Vec::new()
        };
        let mut expr = self.parse_primary_core(lo);
        if !attrs.is_empty() {
            expr.attrs = attrs;
            expr.span = self.span_from(lo);
        }
        expr
    }

    fn parse_primary_core(&mut self, lo: u32) -> Expr {
        let kind = match self.kind() {
            Tk::IntLit { .. }
            | Tk::FloatLit { .. }
            | Tk::StrLit
            | Tk::CharLit
            | Tk::Kw(Kw::True)
            | Tk::Kw(Kw::False) => ExprKind::Lit(self.try_parse_lit().unwrap()),
            Tk::Ident => {
                let path = self.parse_path_in_expr();
                if self.at(Tk::OpenBrace) && !self.no_struct {
                    let fields = self.parse_struct_lit_fields();
                    ExprKind::StructLit { path, fields }
                } else {
                    ExprKind::Path(path)
                }
            }
            Tk::OpenParen => {
                let open = self.peek().span;
                self.bump();
                self.with_no_struct(false, |p| {
                    if p.eat(Tk::CloseParen) {
                        return ExprKind::Tuple(Vec::new());
                    }
                    let first = p.parse_expr();
                    if p.eat(Tk::Comma) {
                        let mut elems = vec![first];
                        while !p.at(Tk::CloseParen) {
                            if p.at(Tk::Eof) {
                                p.error_unclosed(open, "(");
                                return ExprKind::Tuple(elems);
                            }
                            elems.push(p.parse_expr());
                            if !p.eat(Tk::Comma) {
                                break;
                            }
                        }
                        p.expect(Tk::CloseParen, "`)`");
                        ExprKind::Tuple(elems)
                    } else {
                        p.expect(Tk::CloseParen, "`)`");
                        ExprKind::Paren(Box::new(first))
                    }
                })
            }
            Tk::OpenBracket => {
                let open = self.peek().span;
                self.bump();
                self.with_no_struct(false, |p| {
                    if p.eat(Tk::CloseBracket) {
                        return ExprKind::Array(Vec::new());
                    }
                    let first = p.parse_expr();
                    if p.eat(Tk::Semi) {
                        let len = p.parse_expr();
                        if !p.eat(Tk::CloseBracket) {
                            if p.at(Tk::Eof) {
                                p.error_unclosed(open, "[");
                            } else {
                                p.error_expected("`]`");
                            }
                        }
                        ExprKind::Repeat {
                            elem: Box::new(first),
                            len: Box::new(len),
                        }
                    } else {
                        let mut elems = vec![first];
                        while p.eat(Tk::Comma) {
                            if p.at(Tk::CloseBracket) {
                                break;
                            }
                            elems.push(p.parse_expr());
                        }
                        if !p.eat(Tk::CloseBracket) {
                            if p.at(Tk::Eof) {
                                p.error_unclosed(open, "[");
                            } else {
                                p.error_expected("`]`");
                            }
                        }
                        ExprKind::Array(elems)
                    }
                })
            }
            Tk::OpenBrace => ExprKind::Block(self.parse_block()),
            Tk::Kw(Kw::Unsafe) => {
                self.bump();
                ExprKind::Unsafe(self.parse_block())
            }
            Tk::Kw(Kw::Move) => {
                self.bump();
                return self.parse_closure_expr(lo, true);
            }
            Tk::Or | Tk::OrOr => return self.parse_closure_expr(lo, false),
            Tk::Kw(Kw::If) => return self.parse_if_expr(),
            Tk::Kw(Kw::While) => {
                self.bump();
                let cond = self.parse_expr_no_struct();
                let body = self.parse_block();
                ExprKind::While {
                    cond: Box::new(cond),
                    body,
                }
            }
            Tk::Kw(Kw::For) => {
                self.bump();
                let pat = self.parse_pattern();
                self.expect(Tk::Kw(Kw::In), "`in`");
                let iter = self.parse_expr_no_struct();
                let body = self.parse_block();
                ExprKind::For {
                    pat,
                    iter: Box::new(iter),
                    body,
                }
            }
            Tk::Kw(Kw::Loop) => {
                self.bump();
                ExprKind::Loop {
                    body: self.parse_block(),
                }
            }
            Tk::Kw(Kw::Match) => {
                self.bump();
                let scrutinee = self.parse_expr_no_struct();
                let arms = self.parse_match_arms();
                ExprKind::Match {
                    scrutinee: Box::new(scrutinee),
                    arms,
                }
            }
            Tk::Kw(Kw::Return) => {
                self.bump();
                let operand = if self.expr_operand_follows() {
                    Some(Box::new(self.parse_expr()))
                } else {
                    None
                };
                ExprKind::Return(operand)
            }
            Tk::Kw(Kw::Break) => {
                self.bump();
                let operand = if self.expr_operand_follows() {
                    Some(Box::new(self.parse_expr()))
                } else {
                    None
                };
                ExprKind::Break(operand)
            }
            Tk::Kw(Kw::Continue) => {
                self.bump();
                ExprKind::Continue
            }
            _ => {
                self.error_expected("an expression");
                // 进度保证:非结构性 token 消费入 Err 节点
                if !matches!(
                    self.kind(),
                    Tk::CloseParen
                        | Tk::CloseBracket
                        | Tk::CloseBrace
                        | Tk::Semi
                        | Tk::Comma
                        | Tk::Eof
                ) {
                    self.bump();
                }
                ExprKind::Err
            }
        };
        Expr {
            attrs: Vec::new(),
            kind,
            span: self.span_from(lo),
        }
    }

    fn expr_operand_follows(&self) -> bool {
        !matches!(
            self.kind(),
            Tk::Semi | Tk::CloseBrace | Tk::CloseParen | Tk::CloseBracket | Tk::Comma | Tk::Eof
        )
    }

    fn parse_struct_lit_fields(&mut self) -> Vec<FieldInit> {
        let open = self.peek().span;
        self.expect(Tk::OpenBrace, "`{`");
        let mut fields = Vec::new();
        self.with_no_struct(false, |p| {
            while !p.at(Tk::CloseBrace) {
                if p.at(Tk::Eof) {
                    p.error_unclosed(open, "{");
                    return;
                }
                let lo = p.lo();
                let name = p.expect_ident("a field name");
                let expr = if p.eat(Tk::Colon) {
                    Some(p.parse_expr())
                } else {
                    None
                };
                fields.push(FieldInit {
                    name,
                    expr,
                    span: p.span_from(lo),
                });
                if !p.eat(Tk::Comma) {
                    break;
                }
            }
            p.expect(Tk::CloseBrace, "`}`");
        });
        fields
    }

    fn parse_if_expr(&mut self) -> Expr {
        let lo = self.lo();
        self.expect(Tk::Kw(Kw::If), "`if`");
        let cond = self.parse_expr_no_struct();
        let then = self.parse_block();
        let else_ = if self.eat_kw(Kw::Else) {
            if self.at_kw(Kw::If) {
                Some(Box::new(self.parse_if_expr()))
            } else {
                let else_lo = self.lo();
                let block = self.parse_block();
                Some(Box::new(Expr {
                    attrs: Vec::new(),
                    kind: ExprKind::Block(block),
                    span: self.span_from(else_lo),
                }))
            }
        } else {
            None
        };
        Expr {
            attrs: Vec::new(),
            kind: ExprKind::If {
                cond: Box::new(cond),
                then,
                else_,
            },
            span: self.span_from(lo),
        }
    }

    fn parse_match_arms(&mut self) -> Vec<Arm> {
        let open = self.peek().span;
        self.expect(Tk::OpenBrace, "`{`");
        let mut arms = Vec::new();
        self.with_no_struct(false, |p| {
            while !p.at(Tk::CloseBrace) {
                if p.at(Tk::Eof) {
                    p.error_unclosed(open, "{");
                    return;
                }
                let lo = p.lo();
                let attrs = p.parse_outer_attrs();
                let mut pats = vec![p.parse_pattern()];
                while p.eat(Tk::Or) {
                    pats.push(p.parse_pattern());
                }
                let guard = if p.eat_kw(Kw::If) {
                    Some(p.parse_expr_no_struct())
                } else {
                    None
                };
                p.expect(Tk::FatArrow, "`=>`");
                let body = p.parse_expr();
                let body_block_like = body.is_block_like();
                arms.push(Arm {
                    attrs,
                    pats,
                    guard,
                    body,
                    span: p.span_from(lo),
                });
                if p.eat(Tk::Comma) {
                    continue;
                }
                if p.at(Tk::CloseBrace) {
                    break;
                }
                if !body_block_like {
                    // 非块臂体之间必须 `,`(RXS-0029)
                    p.error_expected("`,`");
                }
            }
            p.expect(Tk::CloseBrace, "`}`");
        });
        arms
    }

    /// 闭包表达式(语法解析无条件;gate 检查在 feature_gate,RXS-0031)。
    fn parse_closure_expr(&mut self, lo: u32, is_move: bool) -> Expr {
        let mut params = Vec::new();
        if !self.eat(Tk::OrOr) {
            self.expect(Tk::Or, "`|`");
            while !self.at(Tk::Or) {
                if self.at(Tk::Eof) {
                    self.error_expected("`|`");
                    break;
                }
                let pat = self.parse_pattern();
                let ty = if self.eat(Tk::Colon) {
                    Some(self.parse_type())
                } else {
                    None
                };
                params.push(ClosureParam { pat, ty });
                if !self.eat(Tk::Comma) {
                    break;
                }
            }
            self.expect(Tk::Or, "`|`");
        }
        let body = self.parse_expr();
        Expr {
            attrs: Vec::new(),
            kind: ExprKind::Closure {
                is_move,
                params,
                body: Box::new(body),
            },
            span: self.span_from(lo),
        }
    }
}

/// 二元运算符表(RXS-0025;返回 (op, 优先级, 不可链式)):级 4 比较为不可链式。
fn binop_of(kind: Tk) -> Option<(BinOp, u8, bool)> {
    Some(match kind {
        Tk::Star => (BinOp::Mul, 10, false),
        Tk::Slash => (BinOp::Div, 10, false),
        Tk::Percent => (BinOp::Rem, 10, false),
        Tk::Plus => (BinOp::Add, 9, false),
        Tk::Minus => (BinOp::Sub, 9, false),
        Tk::Shl => (BinOp::Shl, 8, false),
        Tk::Shr => (BinOp::Shr, 8, false),
        Tk::And => (BinOp::BitAnd, 7, false),
        Tk::Caret => (BinOp::BitXor, 6, false),
        Tk::Or => (BinOp::BitOr, 5, false),
        Tk::EqEq => (BinOp::Eq, 4, true),
        Tk::Ne => (BinOp::Ne, 4, true),
        Tk::Lt => (BinOp::Lt, 4, true),
        Tk::Gt => (BinOp::Gt, 4, true),
        Tk::Le => (BinOp::Le, 4, true),
        Tk::Ge => (BinOp::Ge, 4, true),
        Tk::AndAnd => (BinOp::And, 3, false),
        Tk::OrOr => (BinOp::Or, 2, false),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    fn parse_str(src: &str) -> (SourceFile, DiagCtxt) {
        let diag = DiagCtxt::new();
        let tokens = lex(src, SourceId(0), Edition::Rx0, &diag);
        assert!(
            diag.emitted().is_empty(),
            "测试源含词法错误: {:?}",
            diag.emitted()
        );
        let file = parse(src, tokens, SourceId(0), Edition::Rx0, &diag);
        (file, diag)
    }

    fn parse_ok(src: &str) -> SourceFile {
        let (file, diag) = parse_str(src);
        assert!(
            diag.emitted().is_empty(),
            "意外诊断: {:?}\n源:\n{src}",
            diag.emitted()
                .iter()
                .map(|d| (d.code, d.message(diag.messages())))
                .collect::<Vec<_>>()
        );
        file
    }

    fn parse_err(src: &str) -> (SourceFile, Vec<Option<ErrorCode>>) {
        let (file, diag) = parse_str(src);
        let codes = diag.emitted().iter().map(|d| d.code).collect();
        (file, codes)
    }

    fn only_fn(file: &SourceFile) -> &FnItem {
        match &file.items[0].kind {
            ItemKind::Fn(f) => f,
            other => panic!("期待 fn item,实得 {other:?}"),
        }
    }

    fn tail_of(f: &FnItem) -> &Expr {
        f.body.as_ref().unwrap().tail.as_ref().unwrap()
    }

    // -- item(RXS-0011, RXS-0014 ~ RXS-0019) --------------------------------

    //@ spec: RXS-0014
    #[test]
    fn fn_colors() {
        let file = parse_ok(
            "fn host() {}\nkernel fn k(grid: Grid<(64,)>) {}\ndevice fn d() -> f32 { 0.0 }\nconst fn c() -> usize { 1 }",
        );
        let colors: Vec<FnColor> = file
            .items
            .iter()
            .map(|i| match &i.kind {
                ItemKind::Fn(f) => f.color,
                _ => panic!(),
            })
            .collect();
        assert_eq!(
            colors,
            vec![
                FnColor::Host,
                FnColor::Kernel,
                FnColor::Device,
                FnColor::Const
            ]
        );
    }

    //@ spec: RXS-0014
    #[test]
    fn self_params() {
        let file = parse_ok(
            "trait T { fn a(self); fn b(&self); fn c(&mut self); fn d(&'a self, x: i32); fn e(mut self); }",
        );
        let ItemKind::Trait(t) = &file.items[0].kind else {
            panic!()
        };
        let kinds: Vec<&ParamKind> = t
            .items
            .iter()
            .map(|a| match &a.kind {
                AssocItemKind::Fn(f) => &f.params[0].kind,
                _ => panic!(),
            })
            .collect();
        assert!(matches!(
            kinds[0],
            ParamKind::SelfParam {
                by_ref: false,
                mutable: false,
                ..
            }
        ));
        assert!(matches!(
            kinds[1],
            ParamKind::SelfParam { by_ref: true, .. }
        ));
        assert!(matches!(
            kinds[2],
            ParamKind::SelfParam {
                by_ref: true,
                mutable: true,
                ..
            }
        ));
        assert!(matches!(
            kinds[3],
            ParamKind::SelfParam {
                lifetime: Some(_),
                ..
            }
        ));
        assert!(matches!(
            kinds[4],
            ParamKind::SelfParam {
                by_ref: false,
                mutable: true,
                ..
            }
        ));
    }

    //@ spec: RXS-0015
    #[test]
    fn struct_forms() {
        let file = parse_ok("struct A { x: f32, pub y: f32 }\nstruct B(f32, pub u32);\nstruct C;");
        let bodies: Vec<&VariantBody> = file
            .items
            .iter()
            .map(|i| match &i.kind {
                ItemKind::Struct(s) => &s.body,
                _ => panic!(),
            })
            .collect();
        assert!(matches!(bodies[0], VariantBody::Named(f) if f.len() == 2));
        assert!(matches!(bodies[1], VariantBody::Tuple(f) if f.len() == 2));
        assert!(matches!(bodies[2], VariantBody::Unit));
    }

    //@ spec: RXS-0015
    #[test]
    fn enum_variants() {
        let file = parse_ok("enum E { Unit, Tup(f32, u32), Named { a: i32 } }");
        let ItemKind::Enum(e) = &file.items[0].kind else {
            panic!()
        };
        assert_eq!(e.variants.len(), 3);
        assert!(matches!(e.variants[0].body, VariantBody::Unit));
        assert!(matches!(&e.variants[1].body, VariantBody::Tuple(f) if f.len() == 2));
        assert!(matches!(&e.variants[2].body, VariantBody::Named(f) if f.len() == 1));
    }

    //@ spec: RXS-0016
    #[test]
    fn trait_and_impl() {
        let file = parse_ok(
            "trait Integrator { type State; fn step(state: &mut Self::State, dt: f32); }\nimpl Integrator for Euler { type State = f32; fn step(state: &mut f32, dt: f32) { *state += dt; } }\nimpl Euler { fn name() -> u32 { 1 } }",
        );
        let ItemKind::Impl(im) = &file.items[1].kind else {
            panic!()
        };
        assert!(im.trait_ty.is_some());
        let ItemKind::Impl(inherent) = &file.items[2].kind else {
            panic!()
        };
        assert!(inherent.trait_ty.is_none());
    }

    //@ spec: RXS-0017
    #[test]
    fn mod_and_use() {
        let file = parse_ok(
            "mod geometry { pub fn area(w: f32, h: f32) -> f32 { w * h } }\nuse geometry::area;\nuse std::mem::size_of as sizeof;",
        );
        let ItemKind::Use(u) = &file.items[2].kind else {
            panic!()
        };
        assert_eq!(u.alias.as_ref().unwrap().name, "sizeof");
    }

    //@ spec: RXS-0018
    #[test]
    fn static_const_type_alias() {
        let file =
            parse_ok("static mut COUNT: u32 = 0;\nconst MAX: usize = 1 << 16;\ntype Scalar = f32;");
        assert!(matches!(&file.items[0].kind, ItemKind::Static(s) if s.mutable));
        assert!(matches!(&file.items[1].kind, ItemKind::Const(_)));
        assert!(matches!(&file.items[2].kind, ItemKind::TypeAlias(_)));
    }

    //@ spec: RXS-0019
    #[test]
    fn extern_block() {
        let file = parse_ok(
            "#[link(name = \"cublas64_13\")]\nextern \"C\" {\n    fn cublasCreate_v2(handle: *mut CublasHandle) -> i32;\n}",
        );
        let ItemKind::ExternBlock(e) = &file.items[0].kind else {
            panic!()
        };
        assert_eq!(e.abi, "C");
        assert_eq!(e.items.len(), 1);
        let ItemKind::Fn(f) = &e.items[0].kind else {
            panic!()
        };
        assert!(f.body.is_none());
    }

    //@ spec: RXS-0019
    #[test]
    fn extern_fn_with_body_is_error() {
        let (_, codes) = parse_err("extern \"C\" { fn bad() {} }");
        assert!(codes.contains(&Some(E_EXPECTED_TOKEN)));
    }

    //@ spec: RXS-0016
    #[test]
    fn impl_fn_without_body_is_error() {
        let (_, codes) = parse_err("impl Euler { fn nope(); }");
        assert!(codes.contains(&Some(E_EXPECTED_TOKEN)));
    }

    //@ spec: RXS-0012
    #[test]
    fn attributes_meta_forms() {
        let file = parse_ok(
            "#![feature(closures)]\n#[derive(Copy, Clone, DeviceCopy)]\nstruct P { x: f32 }\n#[export(c)]\npub fn f() {}\n#[repr(C)]\nstruct Q { y: u32 }",
        );
        assert_eq!(file.attrs.len(), 1);
        assert!(file.attrs[0].inner);
        let derive = &file.items[0].attrs[0].meta;
        assert_eq!(derive.path.segments[0].ident.name, "derive");
        assert!(matches!(&derive.kind, MetaKind::List(l) if l.len() == 3));
    }

    //@ spec: RXS-0011
    #[test]
    fn inner_attr_after_item_is_error() {
        let (_, codes) = parse_err("fn f() {}\n#![feature(closures)]\nfn g() {}");
        assert!(codes.contains(&Some(E_EXPECTED_TOKEN)));
    }

    //@ spec: RXS-0013
    #[test]
    fn visibility_pub_package() {
        let file = parse_ok("pub(package) fn internal() {}\npub fn open() {}");
        assert!(matches!(file.items[0].vis, Visibility::PubPackage(_)));
        assert!(matches!(file.items[1].vis, Visibility::Pub(_)));
    }

    // -- 泛型(RXS-0020 / RXS-0021) ------------------------------------------

    //@ spec: RXS-0020
    #[test]
    fn generic_params_full() {
        let file = parse_ok(
            "fn constrained<'a, T: DeviceCopy + Clone, U, const N: usize>(t: &'a T, u: U) -> T where U: Default, T: Clone { t.clone() }",
        );
        let f = only_fn(&file);
        assert_eq!(f.generics.params.len(), 4);
        assert!(matches!(
            f.generics.params[0].kind,
            GenericParamKind::Lifetime(_)
        ));
        assert!(matches!(
            &f.generics.params[1].kind,
            GenericParamKind::Type { bounds, .. } if bounds.len() == 2
        ));
        assert!(matches!(
            f.generics.params[3].kind,
            GenericParamKind::Const { .. }
        ));
        assert_eq!(f.generics.where_preds.len(), 2);
    }

    //@ spec: RXS-0020
    #[test]
    fn generic_param_default() {
        let file = parse_ok("trait Add<Rhs = Self> { type Output; }");
        let ItemKind::Trait(t) = &file.items[0].kind else {
            panic!()
        };
        assert!(matches!(
            &t.generics.params[0].kind,
            GenericParamKind::Type {
                default: Some(_),
                ..
            }
        ));
    }

    //@ spec: RXS-0021
    #[test]
    fn nested_generics_shr_split() {
        let file = parse_ok("fn nested(v: Vec<Vec<f32>>) -> usize { v.len() }");
        let f = only_fn(&file);
        let ParamKind::Typed { ty, .. } = &f.params[0].kind else {
            panic!()
        };
        let TyKind::Path(p) = &ty.kind else { panic!() };
        let args = p.segments[0].args.as_ref().unwrap();
        let GenericArg::Type(inner) = &args.args[0] else {
            panic!()
        };
        assert!(matches!(&inner.kind, TyKind::Path(ip) if ip.segments[0].args.is_some()));
    }

    //@ spec: RXS-0021
    #[test]
    fn turbofish_with_nested_const_args() {
        // 注:`kernel` 是保留关键字(RXS-0005),方法名取 get_kernel
        let file = parse_ok(
            "fn run() { let k = module.get_kernel::<tile_gemm<32>>(); let xs = Vec::<i32>::new(); }",
        );
        let f = only_fn(&file);
        assert_eq!(f.body.as_ref().unwrap().stmts.len(), 2);
    }

    //@ spec: RXS-0021
    #[test]
    fn const_args_in_generics() {
        parse_ok("fn f(b: TileBuf<32, 8>, g: Grid<(1024,)>, n: Foo<-1>, blk: Bar<{ 1 + 2 }>) {}");
    }

    //@ spec: RXS-0021
    #[test]
    fn method_turbofish() {
        parse_ok("fn f() { let per_block = input.split::<256>().per_block(); }");
    }

    // -- 类型(RXS-0022) ------------------------------------------------------

    //@ spec: RXS-0022
    #[test]
    fn type_forms() {
        let file = parse_ok(
            "fn t(a: &f32, b: &'a mut Stream<'ctx>, c: *mut *mut SimHandle, d: (), e: (f32,), f: (f32, u32), g: [f32; 4], h: &[f32], i: fn(i32) -> i32, j: _) {}",
        );
        let f = only_fn(&file);
        let kinds: Vec<&TyKind> = f
            .params
            .iter()
            .map(|p| match &p.kind {
                ParamKind::Typed { ty, .. } => &ty.kind,
                _ => panic!(),
            })
            .collect();
        assert!(matches!(kinds[0], TyKind::Ref { mutable: false, .. }));
        assert!(matches!(
            kinds[1],
            TyKind::Ref {
                lifetime: Some(_),
                mutable: true,
                ..
            }
        ));
        assert!(matches!(kinds[2], TyKind::RawPtr { mutable: true, .. }));
        assert!(matches!(kinds[3], TyKind::Tuple(t) if t.is_empty()));
        assert!(matches!(kinds[4], TyKind::Tuple(t) if t.len() == 1));
        assert!(matches!(kinds[5], TyKind::Tuple(t) if t.len() == 2));
        assert!(matches!(kinds[6], TyKind::Array { .. }));
        assert!(matches!(kinds[7], TyKind::Ref { .. }));
        assert!(matches!(kinds[8], TyKind::FnPtr { .. }));
        assert!(matches!(kinds[9], TyKind::Infer));
    }

    //@ spec: RXS-0022
    #[test]
    fn raw_ptr_without_qualifier_is_error() {
        let (_, codes) = parse_err("fn f(p: *SimHandle) {}");
        assert!(codes.contains(&Some(E_EXPECTED_TOKEN)));
    }

    // -- 模式(RXS-0023) ------------------------------------------------------

    //@ spec: RXS-0023
    #[test]
    fn pattern_forms() {
        parse_ok(
            "fn p(particle: Particle) {\n    let (a, b) = (1.0, 2.0);\n    let (first, _, third) = (1, 2, 3);\n    let Particle { pos, vel, mass } = particle;\n    let Particle { pos: position, .. } = make();\n    let [x, y, z] = [1.0, 2.0, 3.0];\n    let &value = &42;\n    let mut acc = 0;\n    let _ = ignored();\n}",
        );
    }

    //@ spec: RXS-0023
    #[test]
    fn range_and_at_patterns() {
        let file = parse_ok(
            "fn m(v: i32) -> i32 { match v { 0 => 0, n if n < 0 => -n, small @ 1..=9 => small, -5..=-1 => 1, _ => v, } }",
        );
        let f = only_fn(&file);
        let ExprKind::Match { arms, .. } = &tail_of(f).kind else {
            panic!()
        };
        assert_eq!(arms.len(), 5);
        assert!(arms[1].guard.is_some());
        assert!(matches!(arms[2].pats[0].kind, PatKind::At { .. }));
        assert!(matches!(arms[3].pats[0].kind, PatKind::Range { .. }));
    }

    //@ spec: RXS-0023
    #[test]
    fn range_pattern_non_literal_bound_is_error() {
        let (_, codes) = parse_err("fn m(v: i32) { match v { 1..=x => 0, _ => 1, }; }");
        assert!(codes.contains(&Some(E_EXPECTED_TOKEN)));
    }

    //@ spec: RXS-0029
    #[test]
    fn match_or_patterns() {
        let file = parse_ok("fn m(v: i32) -> i32 { match v { 0 | 1 | 2 => 0, _ => 1, } }");
        let f = only_fn(&file);
        let ExprKind::Match { arms, .. } = &tail_of(f).kind else {
            panic!()
        };
        assert_eq!(arms[0].pats.len(), 3);
    }

    // -- 语句与块(RXS-0024) --------------------------------------------------

    //@ spec: RXS-0024
    #[test]
    fn shared_let_and_tail_expr() {
        let file = parse_ok(
            "kernel fn k(grid: Grid<(64,)>) { shared let tile: [[f32; 32]; 32]; let i = grid.thread_index(); }\nfn t() -> i32 { let a = 1; a + 1 }",
        );
        let ItemKind::Fn(k) = &file.items[0].kind else {
            panic!()
        };
        let body = k.body.as_ref().unwrap();
        assert!(matches!(&body.stmts[0].kind, StmtKind::Let(l) if l.shared && l.init.is_none()));
        let ItemKind::Fn(t) = &file.items[1].kind else {
            panic!()
        };
        assert!(t.body.as_ref().unwrap().tail.is_some());
    }

    //@ spec: RXS-0024
    #[test]
    fn block_like_stmt_no_semi_and_no_binary_extension() {
        // `loop {}` 语句后跟一元负号语句:不得并为二元减法(RXS-0024)
        let file = parse_ok("fn f() { loop { break; } -g(); if t() { h(); } }");
        let f = only_fn(&file);
        let body = f.body.as_ref().unwrap();
        assert_eq!(body.stmts.len(), 2);
        assert!(matches!(
            &body.stmts[0].kind,
            StmtKind::Expr { expr, semi: false } if matches!(expr.kind, ExprKind::Loop { .. })
        ));
        assert!(matches!(
            &body.stmts[1].kind,
            StmtKind::Expr { expr, semi: true } if matches!(expr.kind, ExprKind::Unary { op: UnOp::Neg, .. })
        ));
        // 末尾无分号的块形态 `if` 是尾表达式(RXS-0024)
        assert!(matches!(
            body.tail.as_deref(),
            Some(Expr {
                kind: ExprKind::If { .. },
                ..
            })
        ));
    }

    //@ spec: RXS-0024
    #[test]
    fn item_stmt_in_block() {
        let file = parse_ok("fn f() { const TILE: usize = 32; fn inner() {} let x = TILE; }");
        let f = only_fn(&file);
        let body = f.body.as_ref().unwrap();
        assert!(matches!(&body.stmts[0].kind, StmtKind::Item(_)));
        assert!(matches!(&body.stmts[1].kind, StmtKind::Item(_)));
    }

    // -- 表达式(RXS-0025 ~ RXS-0028) ------------------------------------------

    //@ spec: RXS-0025
    #[test]
    fn precedence_mul_over_add() {
        let file = parse_ok("fn f() -> i32 { 1 + 2 * 3 }");
        let f = only_fn(&file);
        let ExprKind::Binary { op, rhs, .. } = &tail_of(f).kind else {
            panic!()
        };
        assert_eq!(*op, BinOp::Add);
        assert!(matches!(rhs.kind, ExprKind::Binary { op: BinOp::Mul, .. }));
    }

    //@ spec: RXS-0025
    #[test]
    fn precedence_shift_cast_range_assign() {
        parse_ok(
            "fn f() { let a = 1 << 2 >> 3; let b = x as f64 as f32; let c = 0..10; let d = 0..=255; let mut e = 0; e += 1; e <<= 2; }",
        );
    }

    //@ spec: RXS-0025
    #[test]
    fn chained_comparison_is_error() {
        let (_, codes) = parse_err("fn f() { let x = 1 < 2 < 3; }");
        assert!(codes.contains(&Some(E_EXPECTED_TOKEN)));
    }

    //@ spec: RXS-0025
    #[test]
    fn chained_range_is_error() {
        let (_, codes) = parse_err("fn f() { let x = 1..2..3; }");
        assert!(codes.contains(&Some(E_EXPECTED_TOKEN)));
    }

    //@ spec: RXS-0025
    #[test]
    fn assignment_right_assoc_and_compound() {
        let file = parse_ok("fn f() { *out = *value * 2.0; f %= 3; f ^= 0b101; }");
        let f = only_fn(&file);
        let body = f.body.as_ref().unwrap();
        assert!(matches!(
            &body.stmts[0].kind,
            StmtKind::Expr { expr, .. } if matches!(expr.kind, ExprKind::Assign { op: None, .. })
        ));
        assert!(matches!(
            &body.stmts[1].kind,
            StmtKind::Expr { expr, .. } if matches!(expr.kind, ExprKind::Assign { op: Some(BinOp::Rem), .. })
        ));
    }

    //@ spec: RXS-0026
    #[test]
    fn struct_literal_and_shorthand() {
        let file = parse_ok("fn f() -> P { P { pos: zero(), mass } }");
        let f = only_fn(&file);
        let ExprKind::StructLit { fields, .. } = &tail_of(f).kind else {
            panic!()
        };
        assert!(fields[0].expr.is_some());
        assert!(fields[1].expr.is_none());
    }

    //@ spec: RXS-0026
    #[test]
    fn struct_literal_restriction_in_cond() {
        // `if x == S { … }`:`{` 必须归属条件体(RXS-0026)
        let file = parse_ok(
            "fn f() { if x == s { g(); } while t { h(); } for i in c { k(); } match m { _ => 0, }; }",
        );
        let f = only_fn(&file);
        assert_eq!(f.body.as_ref().unwrap().stmts.len(), 4);
    }

    //@ spec: RXS-0026
    #[test]
    fn expr_attrs_and_literals() {
        let file = parse_ok(
            "fn f() { let attr = #[allow] 0; let s = \"hi\"; let c = 'x'; let t = true; }",
        );
        let f = only_fn(&file);
        let StmtKind::Let(l) = &f.body.as_ref().unwrap().stmts[0].kind else {
            panic!()
        };
        assert_eq!(l.init.as_ref().unwrap().attrs.len(), 1);
    }

    //@ spec: RXS-0026
    #[test]
    fn array_tuple_repeat_unit() {
        parse_ok(
            "fn f() { let a = [1.0, 2.0, 3.0]; let r = [0u8; 16]; let t = (1, 2.0, 'c'); let one = (1,); let u = (); let p = (1 + 2) * 3; }",
        );
    }

    //@ spec: RXS-0027
    #[test]
    fn postfix_chains() {
        let file = parse_ok(
            "fn f(xs: &[f32]) -> f32 { let first = xs[0]; let total = xs.iter().copied().sum(); let v = buf.read_one(0)?; let z = pair.0; total }",
        );
        let f = only_fn(&file);
        assert_eq!(f.body.as_ref().unwrap().stmts.len(), 4);
    }

    //@ spec: RXS-0028
    #[test]
    fn control_flow_forms() {
        parse_ok(
            "fn flow(n: i32) -> i32 {\n    let mut acc = 0;\n    for i in 0..n { if i % 2 == 0 { acc += i; } else { continue; } }\n    while acc < 10 { acc += 1; }\n    loop { acc += 1; if acc > 100 { break; } }\n    if acc > 1000 { return 1000; }\n    acc\n}",
        );
    }

    //@ spec: RXS-0028
    #[test]
    fn if_else_chain_as_tail() {
        let file = parse_ok(
            "device fn clamp01(x: f32) -> f32 { if x < 0.0 { 0.0 } else if x > 1.0 { 1.0 } else { x } }",
        );
        let f = only_fn(&file);
        let ExprKind::If { else_, .. } = &tail_of(f).kind else {
            panic!()
        };
        assert!(matches!(else_.as_ref().unwrap().kind, ExprKind::If { .. }));
    }

    //@ spec: RXS-0026
    #[test]
    fn unsafe_block_expr() {
        let file = parse_ok("fn create() -> i32 { unsafe { cublasCreate_v2(&mut h) } }");
        let f = only_fn(&file);
        assert!(matches!(tail_of(f).kind, ExprKind::Unsafe(_)));
    }

    //@ spec: RXS-0026 / RXS-0031
    #[test]
    fn closure_syntax_parses() {
        // 语法层无条件解析;gate 检查在 feature_gate(RXS-0031)
        let file = parse_ok("fn f() { let g = |x: i32, y| x; let h = || 0; let m = move |v| v; }");
        let f = only_fn(&file);
        let StmtKind::Let(l) = &f.body.as_ref().unwrap().stmts[0].kind else {
            panic!()
        };
        let ExprKind::Closure { params, .. } = &l.init.as_ref().unwrap().kind else {
            panic!()
        };
        assert_eq!(params.len(), 2);
    }

    // -- 错误恢复(RXS-0030) --------------------------------------------------

    //@ spec: RXS-0030
    #[test]
    fn multiple_errors_one_file_with_partial_ast() {
        let src = "fn good1() {}\nfn bad1( {}\nstruct 123\nfn good2() -> i32 { 42 }";
        let (file, codes) = parse_err(src);
        assert!(codes.len() >= 2, "应产出多条诊断: {codes:?}");
        assert!(
            codes
                .iter()
                .all(|c| matches!(c, Some(E_EXPECTED_TOKEN) | Some(E_UNCLOSED_DELIMITER)))
        );
        // 部分 AST:两端的完好 fn 必须存在
        let names: Vec<&str> = file
            .items
            .iter()
            .filter_map(|i| match &i.kind {
                ItemKind::Fn(f) => Some(f.name.name.as_str()),
                _ => None,
            })
            .collect();
        assert!(names.contains(&"good1"));
        assert!(names.contains(&"good2"));
    }

    //@ spec: RXS-0030
    #[test]
    fn stmt_level_recovery_keeps_following_stmts() {
        let src = "fn f() { let a = 1 let b = 2; let c = 3; }";
        let (file, codes) = parse_err(src);
        assert!(!codes.is_empty());
        let f = only_fn(&file);
        // 恢复后 b/c 语句仍被解析
        assert!(f.body.as_ref().unwrap().stmts.len() >= 3);
    }

    //@ spec: RXS-0030
    #[test]
    fn unclosed_brace_is_rx0009() {
        let (_, codes) = parse_err("fn f() { let a = 1;");
        assert!(codes.contains(&Some(E_UNCLOSED_DELIMITER)));
    }

    //@ spec: RXS-0030
    #[test]
    fn unclosed_paren_is_rx0009() {
        let (_, codes) = parse_err("fn f(a: i32, ");
        assert!(codes.contains(&Some(E_UNCLOSED_DELIMITER)));
    }

    //@ spec: RXS-0030
    #[test]
    fn parser_never_loops_on_garbage() {
        // 任意标点序列必须终止并产出诊断
        let (_, codes) = parse_err("fn f() { @ # ? ; } => :: ->");
        assert!(!codes.is_empty());
    }

    //@ spec: RXS-0030
    #[test]
    fn events_emitted_and_balanced() {
        // STUB(RD-004):事件通道冒烟——Start/Finish 配平,token 事件非空
        let diag = DiagCtxt::new();
        let src = "fn f() -> i32 { 1 + 2 }";
        let tokens = lex(src, SourceId(0), Edition::Rx0, &diag);
        let (_, events) = parse_with_events(src, tokens, SourceId(0), Edition::Rx0, &diag);
        let starts = events
            .iter()
            .filter(|e| matches!(e, ParseEvent::Start(_)))
            .count();
        let finishes = events
            .iter()
            .filter(|e| matches!(e, ParseEvent::Finish(_)))
            .count();
        assert_eq!(starts, finishes);
        assert!(events.iter().any(|e| matches!(e, ParseEvent::Token(_))));
    }

    //@ spec: RXS-0014
    #[test]
    fn self_not_first_is_error() {
        let (_, codes) = parse_err("impl T { fn bad(x: i32, self) {} }");
        assert!(codes.contains(&Some(E_EXPECTED_TOKEN)));
    }

    //@ spec: RXS-0013
    #[test]
    fn pub_non_package_is_error() {
        let (_, codes) = parse_err("pub(crate) fn f() {}");
        assert!(codes.contains(&Some(E_EXPECTED_TOKEN)));
    }
}
