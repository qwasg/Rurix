//! 07 §5 结构化诊断 JSON(RXS-0099;LSP publishDiagnostics 与 `--error-format=json` 共用)。

use crate::diag::{Applicability, DiagData, Level};
use crate::messages::MessageTable;
use crate::source_map::SourceMap;
use crate::span::Span;

/// 单条诊断 JSON 对象(手写序列化,无 serde 依赖)。
pub fn diag_to_json(d: &DiagData, sm: &SourceMap, table: &MessageTable) -> String {
    let mut out = String::new();
    out.push('{');
    out.push_str("\"level\":");
    out.push_str(match d.level {
        Level::Error => "\"error\"",
        Level::Warning => "\"warning\"",
    });
    out.push_str(",\"message\":");
    push_str_json(&mut out, &d.message(table));
    if let Some(code) = d.code {
        out.push_str(",\"code\":");
        push_str_json(&mut out, &code.to_string());
    }
    if !d.labels.is_empty() {
        out.push_str(",\"labels\":[");
        for (i, lb) in d.labels.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('{');
            push_span_fields(&mut out, sm, lb.span);
            out.push_str(",\"message\":");
            push_str_json(&mut out, &lb.message);
            out.push('}');
        }
        out.push(']');
    }
    if !d.suggestions.is_empty() {
        out.push_str(",\"suggestions\":[");
        for (i, s) in d.suggestions.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('{');
            push_span_fields(&mut out, sm, s.span);
            out.push_str(",\"replacement\":");
            push_str_json(&mut out, &s.replacement);
            out.push_str(",\"message\":");
            push_str_json(&mut out, &s.message);
            out.push_str(",\"applicability\":");
            push_str_json(
                &mut out,
                match s.applicability {
                    Applicability::MachineApplicable => "MachineApplicable",
                    Applicability::MaybeIncorrect => "MaybeIncorrect",
                },
            );
            out.push('}');
        }
        out.push(']');
    }
    out.push('}');
    out
}

/// 诊断数组 JSON。
pub fn diags_to_json(diags: &[DiagData], sm: &SourceMap, table: &MessageTable) -> String {
    let mut out = String::from("[");
    for (i, d) in diags.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&diag_to_json(d, sm, table));
    }
    out.push(']');
    out
}

fn push_span_fields(out: &mut String, sm: &SourceMap, span: Span) {
    let (line, col) = sm.to_lsp_position(span);
    let end = sm.lookup(span.file, span.hi);
    out.push_str("\"start\":{\"line\":");
    out.push_str(&line.to_string());
    out.push_str(",\"character\":");
    out.push_str(&col.to_string());
    out.push_str("},\"end\":{\"line\":");
    out.push_str(&(end.line - 1).to_string());
    out.push_str(",\"character\":");
    out.push_str(&(end.col - 1).to_string());
    out.push('}');
}

pub fn push_str_json(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::{DiagCtxt, ErrorCode};

    //@ spec: RXS-0099
    #[test]
    fn diag_json_includes_level_and_code() {
        let diag = DiagCtxt::new();
        let file = crate::span::SourceId(0);
        let mut sm = SourceMap::new();
        sm.add_file("t.rx", "fn main() {}", crate::span::Edition::Rx0);
        diag.struct_error(ErrorCode(2001), "codegen.missing_main")
            .span_label(
                crate::span::Span::new(file, 3, 7, crate::span::Edition::Rx0),
                "here",
            )
            .emit();
        let json = diags_to_json(&diag.emitted(), &sm, diag.messages());
        assert!(json.contains("\"level\":\"error\""));
        assert!(json.contains("RX2001"));
    }
}
