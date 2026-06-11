//! lexer(spec 条款 RXS-0001 ~ RXS-0010,spec/lexical.md)。
//!
//! - span 全保留(每 token 携带字节区间);
//! - 错误恢复按 RXS-0010:报告诊断后继续,单文件可多错,出错仍产出 token 流;
//! - 错误码 RX0001 ~ RX0007(registry/error_codes.json,M1.2 分配)。

use crate::diag::{DiagCtxt, ErrorCode};
use crate::span::{Edition, SourceId, Span};

pub const E_UNEXPECTED_CHAR: ErrorCode = ErrorCode(1); // RX0001
pub const E_UNTERMINATED_BLOCK_COMMENT: ErrorCode = ErrorCode(2); // RX0002
pub const E_UNTERMINATED_STRING: ErrorCode = ErrorCode(3); // RX0003
pub const E_BAD_CHAR_LITERAL: ErrorCode = ErrorCode(4); // RX0004
pub const E_BAD_ESCAPE: ErrorCode = ErrorCode(5); // RX0005
pub const E_BAD_NUMBER: ErrorCode = ErrorCode(6); // RX0006
pub const E_BAD_SUFFIX: ErrorCode = ErrorCode(7); // RX0007

/// 保留关键字首批(RXS-0005,表只追加)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Keyword {
    As,
    Break,
    Const,
    Continue,
    Device,
    Else,
    Enum,
    Extern,
    False,
    Fn,
    For,
    If,
    Impl,
    In,
    Kernel,
    Let,
    Loop,
    Match,
    Mod,
    Move,
    Mut,
    Pub,
    Return,
    Shared,
    Static,
    Struct,
    Trait,
    True,
    Type,
    Unsafe,
    Use,
    Where,
    While,
}

impl Keyword {
    pub fn from_ident(s: &str) -> Option<Self> {
        use Keyword::*;
        Some(match s {
            "as" => As,
            "break" => Break,
            "const" => Const,
            "continue" => Continue,
            "device" => Device,
            "else" => Else,
            "enum" => Enum,
            "extern" => Extern,
            "false" => False,
            "fn" => Fn,
            "for" => For,
            "if" => If,
            "impl" => Impl,
            "in" => In,
            "kernel" => Kernel,
            "let" => Let,
            "loop" => Loop,
            "match" => Match,
            "mod" => Mod,
            "move" => Move,
            "mut" => Mut,
            "pub" => Pub,
            "return" => Return,
            "shared" => Shared,
            "static" => Static,
            "struct" => Struct,
            "trait" => Trait,
            "true" => True,
            "type" => Type,
            "unsafe" => Unsafe,
            "use" => Use,
            "where" => Where,
            "while" => While,
            _ => return None,
        })
    }
}

/// 整数字面量进制(RXS-0006)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Base {
    Dec,
    Hex,
    Oct,
    Bin,
}

/// 整数类型后缀(RXS-0006)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IntSuffix {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    Usize,
}

impl IntSuffix {
    fn from_str(s: &str) -> Option<Self> {
        use IntSuffix::*;
        Some(match s {
            "i8" => I8,
            "i16" => I16,
            "i32" => I32,
            "i64" => I64,
            "u8" => U8,
            "u16" => U16,
            "u32" => U32,
            "u64" => U64,
            "usize" => Usize,
            _ => return None,
        })
    }
}

/// 浮点类型后缀(RXS-0007;f16/bf16 延后追加)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FloatSuffix {
    F32,
    F64,
}

impl FloatSuffix {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "f32" => Some(Self::F32),
            "f64" => Some(Self::F64),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TokenKind {
    Kw(Keyword),
    Ident,
    Underscore,
    Lifetime,
    IntLit {
        base: Base,
        suffix: Option<IntSuffix>,
    },
    FloatLit {
        suffix: Option<FloatSuffix>,
    },
    StrLit,
    CharLit,
    // 标点与运算符(RXS-0009,最长匹配)
    OpenParen,
    CloseParen,
    OpenBracket,
    CloseBracket,
    OpenBrace,
    CloseBrace,
    Comma,
    Semi,
    Colon,
    PathSep,  // ::
    Arrow,    // ->
    FatArrow, // =>
    Dot,
    DotDot,
    DotDotEq,
    Question,
    At,
    Pound,
    Eq,
    EqEq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Not,
    AndAnd,
    OrOr,
    And,
    Or,
    Caret,
    Shl,
    Shr,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    AndEq,
    OrEq,
    CaretEq,
    ShlEq,
    ShrEq,
    Eof,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// 词法分析入口:产出以 Eof 收尾的 token 流;诊断经 `diag` 产出(RXS-0010)。
pub fn lex(src: &str, file: SourceId, edition: Edition, diag: &DiagCtxt) -> Vec<Token> {
    let mut lexer = Lexer {
        src,
        pos: 0,
        file,
        edition,
        diag,
    };
    let mut tokens = Vec::new();
    loop {
        let tok = lexer.next_token();
        let is_eof = tok.kind == TokenKind::Eof;
        tokens.push(tok);
        if is_eof {
            break;
        }
    }
    tokens
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// 可以开始某个 token(或被空白/注释路径处理)的码点(RXS-0001 合法起始集)。
fn is_token_start(c: char) -> bool {
    is_ident_start(c)
        || c.is_ascii_digit()
        || matches!(c, ' ' | '\t' | '\r' | '\n')
        || matches!(
            c,
            '(' | ')'
                | '['
                | ']'
                | '{'
                | '}'
                | ','
                | ';'
                | ':'
                | '.'
                | '?'
                | '@'
                | '#'
                | '='
                | '<'
                | '>'
                | '+'
                | '-'
                | '*'
                | '/'
                | '%'
                | '!'
                | '&'
                | '|'
                | '^'
                | '\''
                | '"'
        )
}

struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    file: SourceId,
    edition: Edition,
    diag: &'a DiagCtxt,
}

impl Lexer<'_> {
    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn peek_at(&self, n: usize) -> Option<char> {
        self.src[self.pos..].chars().nth(n)
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn eat(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn span_from(&self, lo: usize) -> Span {
        Span::new(self.file, lo as u32, self.pos as u32, self.edition)
    }

    fn tok(&self, kind: TokenKind, lo: usize) -> Token {
        Token {
            kind,
            span: self.span_from(lo),
        }
    }

    fn next_token(&mut self) -> Token {
        loop {
            let lo = self.pos;
            let Some(c) = self.peek() else {
                return self.tok(TokenKind::Eof, lo);
            };
            match c {
                // RXS-0002 空白
                ' ' | '\t' | '\r' | '\n' => {
                    self.bump();
                }
                // RXS-0003 注释 / 除法
                '/' => match self.peek_at(1) {
                    Some('/') => {
                        while let Some(c) = self.peek() {
                            if c == '\n' {
                                break;
                            }
                            self.bump();
                        }
                    }
                    Some('*') => self.block_comment(lo),
                    Some('=') => {
                        self.bump();
                        self.bump();
                        return self.tok(TokenKind::SlashEq, lo);
                    }
                    _ => {
                        self.bump();
                        return self.tok(TokenKind::Slash, lo);
                    }
                },
                // RXS-0004 / RXS-0005 标识符与关键字
                c if is_ident_start(c) => return self.ident(lo),
                // RXS-0006 / RXS-0007 数字字面量
                c if c.is_ascii_digit() => return self.number(lo),
                // RXS-0008 字符字面量 / 生命周期标记
                '\'' => return self.quote(lo),
                // RXS-0008 字符串字面量
                '"' => return self.string(lo),
                // RXS-0009 标点(最长匹配)
                c if is_token_start(c) => return self.punct(lo),
                // RXS-0001 非法码点(连续违例合并为单条诊断)
                _ => {
                    while let Some(c) = self.peek() {
                        if is_token_start(c) {
                            break;
                        }
                        self.bump();
                    }
                    let found = &self.src[lo..self.pos];
                    self.diag
                        .struct_error(E_UNEXPECTED_CHAR, "lex.unexpected_char")
                        .arg("found", format!("{found:?}"))
                        .span_label(self.span_from(lo), "not a valid token")
                        .emit();
                }
            }
        }
    }

    fn block_comment(&mut self, lo: usize) {
        self.bump(); // '/'
        self.bump(); // '*'
        let mut depth = 1u32;
        while depth > 0 {
            match (self.peek(), self.peek_at(1)) {
                (None, _) => {
                    self.diag
                        .struct_error(
                            E_UNTERMINATED_BLOCK_COMMENT,
                            "lex.unterminated_block_comment",
                        )
                        .span_label(self.span_from(lo), "comment opened here")
                        .emit();
                    return;
                }
                (Some('/'), Some('*')) => {
                    self.bump();
                    self.bump();
                    depth += 1;
                }
                (Some('*'), Some('/')) => {
                    self.bump();
                    self.bump();
                    depth -= 1;
                }
                _ => {
                    self.bump();
                }
            }
        }
    }

    fn scan_ident(&mut self) -> &str {
        let lo = self.pos;
        while let Some(c) = self.peek() {
            if !is_ident_continue(c) {
                break;
            }
            self.bump();
        }
        &self.src[lo..self.pos]
    }

    fn ident(&mut self, lo: usize) -> Token {
        let text = self.scan_ident();
        let kind = if text == "_" {
            TokenKind::Underscore
        } else if let Some(kw) = Keyword::from_ident(text) {
            TokenKind::Kw(kw)
        } else {
            TokenKind::Ident
        };
        self.tok(kind, lo)
    }

    fn number(&mut self, lo: usize) -> Token {
        let mut base = Base::Dec;
        let mut digits = 0usize;
        let mut bad_digit = false;
        if self.peek() == Some('0') {
            match self.peek_at(1) {
                Some('x') => {
                    base = Base::Hex;
                }
                Some('o') => {
                    base = Base::Oct;
                }
                Some('b') => {
                    base = Base::Bin;
                }
                _ => {}
            }
            if base != Base::Dec {
                self.bump();
                self.bump();
            }
        }
        // 数字体:hex 接受 hex 数字;其余进制接受十进制数字(超出进制范围记 RX0006)
        while let Some(c) = self.peek() {
            let consume = match base {
                Base::Hex => c.is_ascii_hexdigit() || c == '_',
                _ => c.is_ascii_digit() || c == '_',
            };
            if !consume {
                break;
            }
            if c != '_' {
                digits += 1;
                let in_range = match base {
                    Base::Dec | Base::Hex => true,
                    Base::Oct => c.is_digit(8),
                    Base::Bin => c.is_digit(2),
                };
                if !in_range {
                    bad_digit = true;
                }
            }
            self.bump();
        }
        if base != Base::Dec && digits == 0 {
            self.diag
                .struct_error(E_BAD_NUMBER, "lex.bad_number")
                .arg("reason", "missing digits after base prefix")
                .span_label(self.span_from(lo), "empty literal body")
                .emit();
        }
        if bad_digit {
            self.diag
                .struct_error(E_BAD_NUMBER, "lex.bad_number")
                .arg("reason", "digit out of range for base")
                .span_label(self.span_from(lo), "invalid digit for this base")
                .emit();
        }
        // RXS-0007:小数与指数(仅十进制)
        let mut is_float = false;
        if base == Base::Dec {
            if self.peek() == Some('.') {
                let after = self.peek_at(1);
                let starts_ident_or_range =
                    matches!(after, Some(c) if is_ident_start(c) || c == '.');
                if !starts_ident_or_range {
                    is_float = true;
                    self.bump(); // '.'
                    while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '_') {
                        self.bump();
                    }
                }
            }
            if matches!(self.peek(), Some('e' | 'E')) {
                let next = self.peek_at(1);
                let is_exponent = match next {
                    Some(c) if c.is_ascii_digit() || c == '+' || c == '-' => true,
                    Some(c) if is_ident_continue(c) => false, // 后缀路径(如 1end)
                    _ => true,                                // `1e` / `1e;`:按缺指数处理
                };
                if is_exponent {
                    is_float = true;
                    self.bump(); // e/E
                    if matches!(self.peek(), Some('+' | '-')) {
                        self.bump();
                    }
                    let mut exp_digits = 0usize;
                    while let Some(c) = self.peek() {
                        if c.is_ascii_digit() || c == '_' {
                            if c != '_' {
                                exp_digits += 1;
                            }
                            self.bump();
                        } else {
                            break;
                        }
                    }
                    if exp_digits == 0 {
                        self.diag
                            .struct_error(E_BAD_NUMBER, "lex.bad_number")
                            .arg("reason", "missing exponent digits")
                            .span_label(self.span_from(lo), "exponent has no digits")
                            .emit();
                    }
                }
            }
        }
        // 后缀(紧贴,RXS-0006)
        let suffix_text = if matches!(self.peek(), Some(c) if is_ident_start(c)) {
            Some(self.scan_ident().to_owned())
        } else {
            None
        };
        let kind = match suffix_text.as_deref() {
            None => {
                if is_float {
                    TokenKind::FloatLit { suffix: None }
                } else {
                    TokenKind::IntLit { base, suffix: None }
                }
            }
            Some(s) => {
                if let Some(fs) = FloatSuffix::from_str(s) {
                    if base == Base::Dec {
                        TokenKind::FloatLit { suffix: Some(fs) }
                    } else {
                        // 非十进制 + 浮点后缀(hex 中 f32 会被吞为数字体,实际仅 oct/bin 可达)
                        self.bad_suffix(lo, s);
                        TokenKind::IntLit { base, suffix: None }
                    }
                } else if let Some(is) = IntSuffix::from_str(s) {
                    if is_float {
                        self.bad_suffix(lo, s);
                        TokenKind::FloatLit { suffix: None }
                    } else {
                        TokenKind::IntLit {
                            base,
                            suffix: Some(is),
                        }
                    }
                } else {
                    self.bad_suffix(lo, s);
                    if is_float {
                        TokenKind::FloatLit { suffix: None }
                    } else {
                        TokenKind::IntLit { base, suffix: None }
                    }
                }
            }
        };
        self.tok(kind, lo)
    }

    fn bad_suffix(&self, lo: usize, suffix: &str) {
        self.diag
            .struct_error(E_BAD_SUFFIX, "lex.bad_suffix")
            .arg("suffix", suffix)
            .span_label(self.span_from(lo), "invalid literal suffix")
            .emit();
    }

    /// `'` 起始:生命周期标记或字符字面量(RXS-0008 消歧)。
    fn quote(&mut self, lo: usize) -> Token {
        self.bump(); // '\''
        // 消歧:' + 标识符起始 且其后不是 ' → 生命周期
        if matches!(self.peek(), Some(c) if is_ident_start(c)) && self.peek_at(1) != Some('\'') {
            self.scan_ident();
            return self.tok(TokenKind::Lifetime, lo);
        }
        match self.peek() {
            None | Some('\n') => {
                self.bad_char(lo, "unterminated");
                return self.tok(TokenKind::CharLit, lo);
            }
            Some('\'') => {
                self.bump();
                self.bad_char(lo, "empty");
                return self.tok(TokenKind::CharLit, lo);
            }
            Some('\\') => self.escape(),
            Some(_) => {
                self.bump();
            }
        }
        if self.eat('\'') {
            return self.tok(TokenKind::CharLit, lo);
        }
        // 未立即闭合:同一行内找闭合引号 → 多码点;否则未终结
        let mut closed = false;
        while let Some(c) = self.peek() {
            if c == '\n' {
                break;
            }
            self.bump();
            if c == '\'' {
                closed = true;
                break;
            }
        }
        self.bad_char(
            lo,
            if closed {
                "more than one character"
            } else {
                "unterminated"
            },
        );
        self.tok(TokenKind::CharLit, lo)
    }

    fn bad_char(&self, lo: usize, reason: &str) {
        self.diag
            .struct_error(E_BAD_CHAR_LITERAL, "lex.bad_char_literal")
            .arg("reason", reason)
            .span_label(self.span_from(lo), "invalid character literal")
            .emit();
    }

    fn string(&mut self, lo: usize) -> Token {
        self.bump(); // '"'
        loop {
            match self.peek() {
                None => {
                    self.diag
                        .struct_error(E_UNTERMINATED_STRING, "lex.unterminated_string")
                        .span_label(self.span_from(lo), "string opened here")
                        .emit();
                    break;
                }
                Some('"') => {
                    self.bump();
                    break;
                }
                Some('\\') => self.escape(),
                Some(_) => {
                    self.bump();
                }
            }
        }
        self.tok(TokenKind::StrLit, lo)
    }

    /// 转义序列(RXS-0008),起始于 `\`;非法转义报 RX0005 并继续。
    fn escape(&mut self) {
        let lo = self.pos;
        self.bump(); // '\\'
        match self.peek() {
            Some('n' | 'r' | 't' | '\\' | '\'' | '"' | '0') => {
                self.bump();
            }
            Some('x') => {
                self.bump();
                let mut value = 0u32;
                let mut count = 0;
                while count < 2 {
                    match self.peek() {
                        Some(c) if c.is_ascii_hexdigit() => {
                            value = value * 16 + c.to_digit(16).unwrap();
                            self.bump();
                            count += 1;
                        }
                        _ => break,
                    }
                }
                if count != 2 || value > 0x7F {
                    self.bad_escape(lo);
                }
            }
            Some('u') => {
                self.bump();
                let mut ok = self.eat('{');
                let mut value = 0u32;
                let mut count = 0;
                if ok {
                    while let Some(c) = self.peek() {
                        if c.is_ascii_hexdigit() {
                            value = value
                                .saturating_mul(16)
                                .saturating_add(c.to_digit(16).unwrap());
                            self.bump();
                            count += 1;
                        } else {
                            break;
                        }
                    }
                    ok = self.eat('}')
                        && (1..=6).contains(&count)
                        && char::from_u32(value).is_some();
                }
                if !ok {
                    self.bad_escape(lo);
                }
            }
            _ => {
                self.bump(); // 吞掉非法转义名(若有)
                self.bad_escape(lo);
            }
        }
    }

    fn bad_escape(&self, lo: usize) {
        let esc = &self.src[lo..self.pos];
        self.diag
            .struct_error(E_BAD_ESCAPE, "lex.bad_escape")
            .arg("esc", format!("{esc:?}"))
            .span_label(self.span_from(lo), "invalid escape")
            .emit();
    }

    /// 标点与运算符,最长匹配(RXS-0009)。
    fn punct(&mut self, lo: usize) -> Token {
        use TokenKind::*;
        let c = self.bump().expect("punct 入口必有码点");
        let kind = match c {
            '(' => OpenParen,
            ')' => CloseParen,
            '[' => OpenBracket,
            ']' => CloseBracket,
            '{' => OpenBrace,
            '}' => CloseBrace,
            ',' => Comma,
            ';' => Semi,
            '?' => Question,
            '@' => At,
            '#' => Pound,
            ':' => {
                if self.eat(':') {
                    PathSep
                } else {
                    Colon
                }
            }
            '.' => {
                if self.eat('.') {
                    if self.eat('=') { DotDotEq } else { DotDot }
                } else {
                    Dot
                }
            }
            '=' => {
                if self.eat('=') {
                    EqEq
                } else if self.eat('>') {
                    FatArrow
                } else {
                    Eq
                }
            }
            '!' => {
                if self.eat('=') {
                    Ne
                } else {
                    Not
                }
            }
            '<' => {
                if self.eat('<') {
                    if self.eat('=') { ShlEq } else { Shl }
                } else if self.eat('=') {
                    Le
                } else {
                    Lt
                }
            }
            '>' => {
                if self.eat('>') {
                    if self.eat('=') { ShrEq } else { Shr }
                } else if self.eat('=') {
                    Ge
                } else {
                    Gt
                }
            }
            '+' => {
                if self.eat('=') {
                    PlusEq
                } else {
                    Plus
                }
            }
            '-' => {
                if self.eat('=') {
                    MinusEq
                } else if self.eat('>') {
                    Arrow
                } else {
                    Minus
                }
            }
            '*' => {
                if self.eat('=') {
                    StarEq
                } else {
                    Star
                }
            }
            '%' => {
                if self.eat('=') {
                    PercentEq
                } else {
                    Percent
                }
            }
            '&' => {
                if self.eat('&') {
                    AndAnd
                } else if self.eat('=') {
                    AndEq
                } else {
                    And
                }
            }
            '|' => {
                if self.eat('|') {
                    OrOr
                } else if self.eat('=') {
                    OrEq
                } else {
                    Or
                }
            }
            '^' => {
                if self.eat('=') {
                    CaretEq
                } else {
                    Caret
                }
            }
            _ => unreachable!("punct 入口码点未覆盖: {c:?}"),
        };
        self.tok(kind, lo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::DiagCtxt;

    /// lex 并断言无诊断,返回去 Eof 的 kind 序列。
    fn lex_ok(src: &str) -> Vec<TokenKind> {
        let diag = DiagCtxt::new();
        let toks = lex(src, SourceId(0), Edition::Rx0, &diag);
        assert_eq!(
            diag.emitted().len(),
            0,
            "unexpected diagnostics for {src:?}: {:?}",
            diag.emitted()
        );
        assert_eq!(toks.last().map(|t| t.kind), Some(TokenKind::Eof));
        toks[..toks.len() - 1].iter().map(|t| t.kind).collect()
    }

    /// lex 并返回 (去 Eof 的 kind 序列, 诊断错误码序列)。
    fn lex_err(src: &str) -> (Vec<TokenKind>, Vec<u16>) {
        let diag = DiagCtxt::new();
        let toks = lex(src, SourceId(0), Edition::Rx0, &diag);
        let codes = diag
            .emitted()
            .iter()
            .map(|d| d.code.expect("词法诊断必带错误码").0)
            .collect();
        (
            toks[..toks.len() - 1].iter().map(|t| t.kind).collect(),
            codes,
        )
    }

    // ---- RXS-0005 关键字 ----

    #[test]
    fn keywords_full_table() {
        let src = "as break const continue device else enum extern false fn \
                   for if impl in kernel let loop match mod move \
                   mut pub return shared static struct trait true type unsafe \
                   use where while";
        let kinds = lex_ok(src);
        assert_eq!(kinds.len(), 33);
        assert!(kinds.iter().all(|k| matches!(k, TokenKind::Kw(_))));
    }

    #[test]
    fn contextual_keywords_lex_as_idents() {
        // RXS-0005:地址空间名是上下文关键字,词法层按标识符产出
        for s in ["global", "constant", "local", "host"] {
            assert_eq!(lex_ok(s), vec![TokenKind::Ident]);
        }
    }

    // ---- RXS-0004 标识符 ----

    #[test]
    fn idents_and_underscore() {
        assert_eq!(lex_ok("_"), vec![TokenKind::Underscore]);
        assert_eq!(lex_ok("_a"), vec![TokenKind::Ident]);
        assert_eq!(lex_ok("a1_b"), vec![TokenKind::Ident]);
    }

    // ---- RXS-0006 整数字面量 ----

    #[test]
    fn int_literals_all_bases_and_suffixes() {
        use TokenKind::IntLit;
        assert_eq!(
            lex_ok("123 0xfF 0o17 0b1010 1_000"),
            vec![
                IntLit {
                    base: Base::Dec,
                    suffix: None
                },
                IntLit {
                    base: Base::Hex,
                    suffix: None
                },
                IntLit {
                    base: Base::Oct,
                    suffix: None
                },
                IntLit {
                    base: Base::Bin,
                    suffix: None
                },
                IntLit {
                    base: Base::Dec,
                    suffix: None
                },
            ]
        );
        assert_eq!(
            lex_ok("42i32 7usize 255u8"),
            vec![
                IntLit {
                    base: Base::Dec,
                    suffix: Some(IntSuffix::I32)
                },
                IntLit {
                    base: Base::Dec,
                    suffix: Some(IntSuffix::Usize)
                },
                IntLit {
                    base: Base::Dec,
                    suffix: Some(IntSuffix::U8)
                },
            ]
        );
    }

    #[test]
    fn int_literal_errors() {
        assert_eq!(lex_err("0x").1, vec![6]); // 空进制体 → RX0006
        assert_eq!(lex_err("0b102").1, vec![6]); // 进制外数字 → RX0006
        assert_eq!(lex_err("1i128").1, vec![7]); // 未知后缀 → RX0007
    }

    // ---- RXS-0007 浮点字面量 ----

    #[test]
    fn float_literals_and_disambiguation() {
        use TokenKind::*;
        assert_eq!(lex_ok("1.5"), vec![FloatLit { suffix: None }]);
        assert_eq!(lex_ok("1."), vec![FloatLit { suffix: None }]);
        assert_eq!(lex_ok("1e10 2.5e-3"), vec![FloatLit { suffix: None }; 2]);
        assert_eq!(
            lex_ok("1f32 1.5f64"),
            vec![
                FloatLit {
                    suffix: Some(FloatSuffix::F32)
                },
                FloatLit {
                    suffix: Some(FloatSuffix::F64)
                },
            ]
        );
        // 消歧:1..2 与 1.foo(RXS-0007)
        assert_eq!(
            lex_ok("1..2"),
            vec![
                IntLit {
                    base: Base::Dec,
                    suffix: None
                },
                DotDot,
                IntLit {
                    base: Base::Dec,
                    suffix: None
                },
            ]
        );
        assert_eq!(
            lex_ok("1.foo"),
            vec![
                IntLit {
                    base: Base::Dec,
                    suffix: None
                },
                Dot,
                Ident
            ]
        );
    }

    #[test]
    fn float_literal_errors() {
        assert_eq!(lex_err("1e").1, vec![6]); // 缺指数 → RX0006
        assert_eq!(lex_err("1.5i32").1, vec![7]); // 浮点 + 整数后缀 → RX0007
    }

    // ---- RXS-0008 字符 / 字符串 / 生命周期 ----

    #[test]
    fn char_and_lifetime_disambiguation() {
        assert_eq!(lex_ok("'a'"), vec![TokenKind::CharLit]);
        assert_eq!(lex_ok("'ctx"), vec![TokenKind::Lifetime]);
        assert_eq!(lex_ok(r"'\n'"), vec![TokenKind::CharLit]);
        assert_eq!(
            lex_ok("Stream<'ctx>"),
            vec![
                TokenKind::Ident,
                TokenKind::Lt,
                TokenKind::Lifetime,
                TokenKind::Gt
            ]
        );
    }

    #[test]
    fn char_literal_errors() {
        assert_eq!(lex_err("''").1, vec![4]); // 空 → RX0004
        assert_eq!(lex_err("'@@'").1, vec![4]); // 多码点 → RX0004
        assert_eq!(lex_err("'@").1, vec![4]); // 未终结 → RX0004
    }

    #[test]
    fn string_literals_and_escapes() {
        assert_eq!(lex_ok(r#""hi""#), vec![TokenKind::StrLit]);
        assert_eq!(
            lex_ok(r#""\x41 \u{1F600} \n \\ \" \0""#),
            vec![TokenKind::StrLit]
        );
        assert_eq!(lex_ok("\"line1\nline2\""), vec![TokenKind::StrLit]); // 字面换行合法
    }

    #[test]
    fn string_and_escape_errors() {
        assert_eq!(lex_err(r#""open"#).1, vec![3]); // 未终结 → RX0003
        assert_eq!(lex_err(r#""\q""#).1, vec![5]); // 未知转义 → RX0005
        assert_eq!(lex_err(r#""\x80""#).1, vec![5]); // \x 超界 → RX0005
        assert_eq!(lex_err(r#""\u{}""#).1, vec![5]); // \u 空位数 → RX0005
    }

    // ---- RXS-0003 注释 ----

    #[test]
    fn comments_line_and_nested_block() {
        assert_eq!(lex_ok("// line\nlet"), vec![TokenKind::Kw(Keyword::Let)]);
        assert_eq!(
            lex_ok("/* a /* nested */ b */ let"),
            vec![TokenKind::Kw(Keyword::Let)]
        );
    }

    #[test]
    fn unterminated_block_comment() {
        assert_eq!(lex_err("/* open").1, vec![2]); // RX0002
    }

    // ---- RXS-0009 标点与最长匹配 ----

    #[test]
    fn punctuation_longest_match() {
        use TokenKind::*;
        assert_eq!(lex_ok(">>="), vec![ShrEq]);
        assert_eq!(lex_ok(">>"), vec![Shr]);
        assert_eq!(lex_ok("..="), vec![DotDotEq]);
        assert_eq!(lex_ok("::"), vec![PathSep]);
        assert_eq!(lex_ok("-> =>"), vec![Arrow, FatArrow]);
        assert_eq!(
            lex_ok("a += b && c"),
            vec![Ident, PlusEq, Ident, AndAnd, Ident]
        );
    }

    // ---- RXS-0001 非法字符与合并诊断 ----

    //@ spec: RXS-0001
    #[test]
    fn illegal_chars_merged_into_one_diag() {
        // 非 ASCII 标识符尝试:连续违例合并为单条 RX0001(RXS-0001 实现要求)
        let (kinds, codes) = lex_err("\u{53d8}\u{91cf}x");
        assert_eq!(codes, vec![1]);
        assert_eq!(kinds, vec![TokenKind::Ident]); // 后续 x 恢复为标识符
    }

    #[test]
    fn bom_and_nul_rejected() {
        assert_eq!(lex_err("\u{feff}let").1, vec![1]);
        assert_eq!(lex_err("\u{0}").1, vec![1]);
    }

    // ---- RXS-0010 错误恢复 ----

    //@ spec: RXS-0010
    #[test]
    fn multi_error_recovery_in_one_file() {
        // 三处独立词法错误,lexing 不中断且 token 流完整(RXS-0010)
        let (kinds, codes) = lex_err("let $ = 0x; /* open");
        assert_eq!(codes, vec![1, 6, 2]);
        assert!(kinds.contains(&TokenKind::Kw(Keyword::Let)));
        assert!(kinds.contains(&TokenKind::Eq));
    }

    #[test]
    fn token_spans_are_exact() {
        let diag = DiagCtxt::new();
        let toks = lex("let xs", SourceId(0), Edition::Rx0, &diag);
        assert_eq!((toks[0].span.lo.0, toks[0].span.hi.0), (0, 3));
        assert_eq!((toks[1].span.lo.0, toks[1].span.hi.0), (4, 6));
        assert_eq!(toks[2].kind, TokenKind::Eof);
    }
}
