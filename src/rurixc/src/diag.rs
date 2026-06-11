//! 诊断结构(07 §5,D-206):`DiagCtxt` + `Diag` builder。
//!
//! - **emit-or-cancel 强制**:`Diag` 在 Drop 时若既未 `emit` 也未 `cancel` 即 ICE
//!   (panic)——诊断泄漏是编译器 bug,不是可忽略状态;
//! - 结构:error/warning + 多 span label + note + help + suggestion(携带
//!   [`Applicability`]);
//! - 文本经 message-key(未注册 key 在构造时即 ICE,见 [`crate::messages`]);
//! - 渲染(annotate-snippets)与 `--error-format=json` 输出随 M1.2 首批真实诊断接入。

use std::cell::Ref;
use std::cell::RefCell;
use std::fmt;

use crate::messages::{self, MessageTable};
use crate::span::Span;

/// 错误码 `RX####`(分配制,`registry/error_codes.json` 为唯一事实源)。
///
/// 段位(07 §5):0xxx 词法/语法、1xxx 名称/模块、2xxx 类型、3xxx 着色/地址空间、
/// 4xxx 借用/生命周期、5xxx const eval、6xxx codegen/目标、7xxx 链接/工具链。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ErrorCode(pub u16);

impl ErrorCode {
    /// 段位首位数字(0–7)。
    pub fn segment(&self) -> u16 {
        self.0 / 1000
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RX{:04}", self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    Error,
    Warning,
}

/// suggestion 的机器可用性(07 §5;`MachineApplicable` 是 `rx fix` 的数据源)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Applicability {
    MachineApplicable,
    MaybeIncorrect,
}

#[derive(Clone, Debug)]
pub struct Label {
    pub span: Span,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct Suggestion {
    pub span: Span,
    pub replacement: String,
    pub message: String,
    pub applicability: Applicability,
}

/// 一条已构造诊断的全部数据(emit 后归档于 [`DiagCtxt`])。
#[derive(Clone, Debug)]
pub struct DiagData {
    pub level: Level,
    pub code: Option<ErrorCode>,
    pub message_key: String,
    pub args: Vec<(String, String)>,
    pub labels: Vec<Label>,
    pub notes: Vec<String>,
    pub helps: Vec<String>,
    pub suggestions: Vec<Suggestion>,
}

impl DiagData {
    /// 主消息文本(key + args 经消息表渲染)。
    pub fn message(&self, table: &MessageTable) -> String {
        let args: Vec<(&str, &str)> = self
            .args
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        table
            .render(&self.message_key, &args)
            .unwrap_or_else(|| panic!("ICE: message key 未注册: {:?}", self.message_key))
    }
}

/// 诊断上下文:构造、收集与计数。
pub struct DiagCtxt {
    messages: MessageTable,
    emitted: RefCell<Vec<DiagData>>,
}

impl Default for DiagCtxt {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagCtxt {
    /// 使用内嵌 en 消息表。
    pub fn new() -> Self {
        Self::with_messages(messages::table().clone())
    }

    /// 注入消息表(单测/未来多语言通道)。
    pub fn with_messages(messages: MessageTable) -> Self {
        Self {
            messages,
            emitted: RefCell::new(Vec::new()),
        }
    }

    pub fn struct_error(&self, code: ErrorCode, message_key: &str) -> Diag<'_> {
        self.struct_diag(Level::Error, Some(code), message_key)
    }

    pub fn struct_warning(&self, message_key: &str) -> Diag<'_> {
        self.struct_diag(Level::Warning, None, message_key)
    }

    fn struct_diag(&self, level: Level, code: Option<ErrorCode>, message_key: &str) -> Diag<'_> {
        // key 有效性在构造点强制(07 §5:编译期校验 key 有效性的运行时哨兵)
        assert!(
            self.messages.contains(message_key),
            "ICE: message key 未注册: {message_key:?}"
        );
        Diag {
            ctxt: self,
            data: Some(DiagData {
                level,
                code,
                message_key: message_key.to_owned(),
                args: Vec::new(),
                labels: Vec::new(),
                notes: Vec::new(),
                helps: Vec::new(),
                suggestions: Vec::new(),
            }),
        }
    }

    pub fn messages(&self) -> &MessageTable {
        &self.messages
    }

    pub fn emitted(&self) -> Ref<'_, Vec<DiagData>> {
        self.emitted.borrow()
    }

    pub fn error_count(&self) -> usize {
        self.emitted
            .borrow()
            .iter()
            .filter(|d| d.level == Level::Error)
            .count()
    }

    pub fn has_errors(&self) -> bool {
        self.error_count() > 0
    }

    fn record(&self, data: DiagData) {
        self.emitted.borrow_mut().push(data);
    }
}

/// 诊断 builder。**必须**以 [`Diag::emit`] 或 [`Diag::cancel`] 终结。
pub struct Diag<'a> {
    ctxt: &'a DiagCtxt,
    /// emit/cancel 时取走;Drop 时仍为 Some 即泄漏 → ICE。
    data: Option<DiagData>,
}

impl Diag<'_> {
    fn data_mut(&mut self) -> &mut DiagData {
        self.data.as_mut().expect("Diag 已终结")
    }

    /// 消息模板参数(`{name}` 占位)。
    pub fn arg(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.data_mut().args.push((name.into(), value.into()));
        self
    }

    pub fn span_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.data_mut().labels.push(Label {
            span,
            message: message.into(),
        });
        self
    }

    pub fn note(mut self, message: impl Into<String>) -> Self {
        self.data_mut().notes.push(message.into());
        self
    }

    pub fn help(mut self, message: impl Into<String>) -> Self {
        self.data_mut().helps.push(message.into());
        self
    }

    pub fn suggestion(
        mut self,
        span: Span,
        message: impl Into<String>,
        replacement: impl Into<String>,
        applicability: Applicability,
    ) -> Self {
        self.data_mut().suggestions.push(Suggestion {
            span,
            replacement: replacement.into(),
            message: message.into(),
            applicability,
        });
        self
    }

    pub fn emit(mut self) {
        let data = self.data.take().expect("Diag 已终结");
        self.ctxt.record(data);
    }

    pub fn cancel(mut self) {
        self.data.take();
    }
}

impl Drop for Diag<'_> {
    fn drop(&mut self) {
        // 已在 panic 展开中则不二次 panic(避免 abort 吞掉原始 ICE 信息)
        if self.data.is_some() && !std::thread::panicking() {
            panic!(
                "ICE: Diag 泄漏(message key {:?}):既未 emit() 也未 cancel()(emit-or-cancel 强制,07 §5)",
                self.data.as_ref().map(|d| d.message_key.as_str())
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::MessageTable;
    use crate::span::{Edition, SourceId, Span};

    fn test_ctxt() -> DiagCtxt {
        let table =
            MessageTable::parse("test.bad_thing = bad thing: {what}\ntest.warn = something odd\n")
                .unwrap();
        DiagCtxt::with_messages(table)
    }

    fn dummy_span() -> Span {
        Span::new(SourceId(0), 0, 3, Edition::Rx0)
    }

    #[test]
    fn emit_records_full_structure() {
        let ctxt = test_ctxt();
        ctxt.struct_error(ErrorCode(1), "test.bad_thing")
            .arg("what", "'@'")
            .span_label(dummy_span(), "here")
            .note("a note")
            .help("a help")
            .suggestion(
                dummy_span(),
                "replace it",
                "x",
                Applicability::MachineApplicable,
            )
            .emit();

        assert_eq!(ctxt.error_count(), 1);
        assert!(ctxt.has_errors());
        let emitted = ctxt.emitted();
        let d = &emitted[0];
        assert_eq!(d.code, Some(ErrorCode(1)));
        assert_eq!(d.message(ctxt.messages()), "bad thing: '@'");
        assert_eq!(d.labels.len(), 1);
        assert_eq!(d.notes, vec!["a note".to_owned()]);
        assert_eq!(d.helps, vec!["a help".to_owned()]);
        assert_eq!(
            d.suggestions[0].applicability,
            Applicability::MachineApplicable
        );
    }

    #[test]
    fn warning_does_not_count_as_error() {
        let ctxt = test_ctxt();
        ctxt.struct_warning("test.warn").emit();
        assert_eq!(ctxt.error_count(), 0);
        assert!(!ctxt.has_errors());
        assert_eq!(ctxt.emitted().len(), 1);
    }

    #[test]
    fn cancel_discards_without_ice() {
        let ctxt = test_ctxt();
        ctxt.struct_error(ErrorCode(2), "test.bad_thing").cancel();
        assert_eq!(ctxt.emitted().len(), 0);
    }

    #[test]
    #[should_panic(expected = "ICE: Diag 泄漏")]
    fn leaked_diag_is_ice() {
        let ctxt = test_ctxt();
        let diag = ctxt.struct_error(ErrorCode(3), "test.bad_thing");
        drop(diag); // 既未 emit 也未 cancel
    }

    #[test]
    #[should_panic(expected = "ICE: message key 未注册")]
    fn unknown_message_key_is_ice() {
        let ctxt = test_ctxt();
        let _ = ctxt.struct_error(ErrorCode(4), "no.such.key");
    }

    #[test]
    fn error_code_display_and_segment() {
        assert_eq!(ErrorCode(301).to_string(), "RX0301");
        assert_eq!(ErrorCode(301).segment(), 0);
        assert_eq!(ErrorCode(6010).segment(), 6);
    }
}
