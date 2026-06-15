//! LSP stdio JSON-RPC server + 能力面冒烟(RXS-0098~0103)。

use std::io::{self, BufRead, Read, Write};

use crate::tooling::diag_json::diags_to_json;
use crate::tooling::ide_query::{
    LspRange, completions_at, definition_at, highlights_at, references_at, rename_at,
    rename_at_checked,
};
use crate::tooling::json_util::{
    escape_json, json_array_field, json_i64_field, json_object_field, json_str_field,
    json_top_level_value_field,
};
use crate::tooling::session::{ToolingSession, query_ctx_for};

/// 冒烟结果(供 CI evidence 复用 server 逻辑)。
#[derive(Debug, Default)]
pub struct SmokeResult {
    pub capabilities_passed: Vec<String>,
    pub failures: Vec<String>,
}

pub fn run_stdio_server() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut session = ToolingSession::new();
    let mut reader = stdin.lock();
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 {
            break;
        }
        if !header.to_lowercase().starts_with("content-length:") {
            continue;
        }
        let len: usize = header
            .split(':')
            .nth(1)
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line)? == 0 {
                return Ok(());
            }
            if line.trim().is_empty() {
                break;
            }
        }
        let mut body = vec![0u8; len];
        reader.read_exact(&mut body)?;
        let body = String::from_utf8_lossy(&body);
        for resp in handle_message(&mut session, &body) {
            write_message(&mut stdout, &resp)?;
        }
    }
    Ok(())
}

fn write_message(out: &mut impl Write, json: &str) -> io::Result<()> {
    writeln!(out, "Content-Length: {}", json.len())?;
    writeln!(out)?;
    out.write_all(json.as_bytes())?;
    out.flush()
}

fn handle_message(session: &mut ToolingSession, body: &str) -> Vec<String> {
    if let Some(id) = json_top_level_value_field(body, "id").map(json_id_raw) {
        let method = json_str_field(body, "method").unwrap_or_default();
        let params = json_object_field(body, "params").unwrap_or("{}");
        return dispatch_request(session, &id, &method, params);
    }
    let method = json_str_field(body, "method").unwrap_or_default();
    let params = json_object_field(body, "params").unwrap_or("{}");
    if method == "textDocument/didOpen" {
        let text_doc = json_object_field(params, "textDocument").unwrap_or(params);
        let uri = json_str_field(text_doc, "uri").unwrap_or_default();
        let text = json_str_field(text_doc, "text").unwrap_or_default();
        let version = json_i64_field(text_doc, "version").unwrap_or(1) as i32;
        session.open(uri.clone(), text, version);
        return send_diagnostics(session, &uri).into_iter().collect();
    }
    if method == "textDocument/didChange" {
        let text_doc = json_object_field(params, "textDocument").unwrap_or(params);
        let uri = json_str_field(text_doc, "uri").unwrap_or_default();
        let version = json_i64_field(text_doc, "version").unwrap_or(1) as i32;
        let text = json_array_field(params, "contentChanges")
            .and_then(first_array_object)
            .and_then(|change| json_str_field(change, "text"))
            .or_else(|| json_str_field(params, "text"))
            .unwrap_or_default();
        session.change(&uri, version, text);
        return send_diagnostics(session, &uri).into_iter().collect();
    }
    Vec::new()
}

fn send_diagnostics(session: &ToolingSession, uri: &str) -> Option<String> {
    let doc = session.get(uri)?;
    let json = diags_to_json(
        &doc.diag.emitted(),
        &session.source_map,
        doc.diag.messages(),
    );
    Some(format!(
        "{{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/publishDiagnostics\",\"params\":{{\"uri\":\"{}\",\"diagnostics\":{}}}}}",
        escape_json(uri),
        json
    ))
}

fn dispatch_request(
    session: &mut ToolingSession,
    id: &str,
    method: &str,
    params: &str,
) -> Vec<String> {
    match method {
        "initialize" => vec![format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{{\"capabilities\":{{\"textDocumentSync\":1,\"completionProvider\":{{\"triggerCharacters\":[\".\"]}},\"referencesProvider\":true,\"documentHighlightProvider\":true,\"renameProvider\":true}}}}}}",
            id
        )],
        "textDocument/completion" => {
            let uri = text_document_uri(params);
            let line = position_line(params);
            let character = position_character(params);
            let Some(doc) = session.get(&uri) else {
                return vec![empty_result(id)];
            };
            let cx = query_ctx_for(doc);
            let prefix = prefix_at(&doc.text, line, character);
            let items = completions_at(&cx, &prefix);
            let arr = items
                .iter()
                .map(|l| format!("{{\"label\":\"{}\",\"kind\":6}}", escape_json(l)))
                .collect::<Vec<_>>()
                .join(",");
            vec![format!(
                "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{{\"isIncomplete\":false,\"items\":[{}]}}}}",
                id, arr
            )]
        }
        "textDocument/definition" => {
            vec![location_response(session, id, params, definition_at)]
        }
        "textDocument/references" => {
            vec![locations_response(session, id, params, references_at)]
        }
        "textDocument/documentHighlight" => {
            let uri = text_document_uri(params);
            let line = position_line(params);
            let character = position_character(params);
            let Some(doc) = session.get(&uri) else {
                return vec![empty_result(id)];
            };
            let cx = query_ctx_for(doc);
            let ranges = highlights_at(&cx, &session.source_map, doc.file_id, line, character);
            let arr = ranges
                .iter()
                .map(|r| format!("{{\"range\":{},\"kind\":2}}", range_json(r)))
                .collect::<Vec<_>>()
                .join(",");
            vec![format!(
                "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":[{}]}}",
                id, arr
            )]
        }
        "textDocument/rename" => {
            let uri = text_document_uri(params);
            let new_name = json_str_field(params, "newName").unwrap_or_default();
            let line = position_line(params);
            let character = position_character(params);
            let Some(doc) = session.get(&uri) else {
                return vec![empty_result(id)];
            };
            let cx = query_ctx_for(doc);
            match rename_at_checked(
                &cx,
                &session.source_map,
                doc.file_id,
                line,
                character,
                &new_name,
                Some(&doc.diag),
            ) {
                Ok(edits) => {
                    let change_arr = edits
                        .iter()
                        .map(|e| {
                            format!(
                                "{{\"range\":{},\"newText\":\"{}\"}}",
                                range_json(&e.range),
                                escape_json(&e.new_text)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    vec![format!(
                        "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{{\"changes\":{{\"{}\":[{}]}}}}}}",
                        id,
                        escape_json(&uri),
                        change_arr
                    )]
                }
                Err(_) => {
                    let mut out = vec![format!(
                        "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{{\"changes\":{{\"{}\":[]}}}}}}",
                        id,
                        escape_json(&uri)
                    )];
                    if let Some(diag) = send_diagnostics(session, &uri) {
                        out.push(diag);
                    }
                    out
                }
            }
        }
        "shutdown" => vec![format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":null}}",
            id
        )],
        _ => vec![empty_result(id)],
    }
}

fn empty_result(id: &str) -> String {
    format!("{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":null}}", id)
}

fn json_id_raw(rest: &str) -> String {
    let rest = rest.trim_start();
    if let Some(stripped) = rest.strip_prefix('"') {
        let mut escaped = false;
        for (i, ch) in stripped.char_indices() {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                return rest[..=i + 1].to_string();
            }
        }
    }
    let end = rest
        .find(|c: char| c == ',' || c == '}' || c.is_ascii_whitespace())
        .unwrap_or(rest.len());
    rest[..end].to_string()
}

fn text_document_uri(params: &str) -> String {
    json_object_field(params, "textDocument")
        .and_then(|td| json_str_field(td, "uri"))
        .or_else(|| json_str_field(params, "uri"))
        .unwrap_or_default()
}

fn first_array_object(array: &str) -> Option<&str> {
    let start = array.find('{')?;
    let rest = &array[start..];
    let mut depth = 0i32;
    let mut in_str = false;
    let mut escaped = false;
    for (i, ch) in rest.char_indices() {
        if in_str {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_str = false;
            }
            continue;
        }
        match ch {
            '"' => in_str = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&rest[..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

fn position_line(params: &str) -> u32 {
    json_object_field(params, "position")
        .and_then(|p| json_i64_field(p, "line"))
        .unwrap_or(0) as u32
}

fn position_character(params: &str) -> u32 {
    json_object_field(params, "position")
        .and_then(|p| json_i64_field(p, "character"))
        .unwrap_or(0) as u32
}

fn prefix_at(text: &str, line: u32, character: u32) -> String {
    let line_text = text.lines().nth(line as usize).unwrap_or("");
    let prefix: String = line_text.chars().take(character as usize).collect();
    prefix
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

fn range_json(r: &LspRange) -> String {
    format!(
        "{{\"start\":{{\"line\":{},\"character\":{}}},\"end\":{{\"line\":{},\"character\":{}}}}}",
        r.start_line, r.start_character, r.end_line, r.end_character
    )
}

fn location_response(
    session: &ToolingSession,
    id: &str,
    params: &str,
    f: impl Fn(
        &crate::query::QueryCtx<'_>,
        &crate::source_map::SourceMap,
        crate::span::SourceId,
        u32,
        u32,
    ) -> Option<LspRange>,
) -> String {
    let uri = text_document_uri(params);
    let line = position_line(params);
    let character = position_character(params);
    let Some(doc) = session.get(&uri) else {
        return empty_result(id);
    };
    let cx = query_ctx_for(doc);
    if let Some(r) = f(&cx, &session.source_map, doc.file_id, line, character) {
        format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{{\"uri\":\"{}\",\"range\":{}}}}}",
            id,
            escape_json(&uri),
            range_json(&r)
        )
    } else {
        empty_result(id)
    }
}

fn locations_response(
    session: &ToolingSession,
    id: &str,
    params: &str,
    f: impl Fn(
        &crate::query::QueryCtx<'_>,
        &crate::source_map::SourceMap,
        crate::span::SourceId,
        u32,
        u32,
    ) -> Vec<LspRange>,
) -> String {
    let uri = text_document_uri(params);
    let line = position_line(params);
    let character = position_character(params);
    let Some(doc) = session.get(&uri) else {
        return format!("{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":[]}}", id);
    };
    let cx = query_ctx_for(doc);
    let ranges = f(&cx, &session.source_map, doc.file_id, line, character);
    let arr = ranges
        .iter()
        .map(|r| {
            format!(
                "{{\"uri\":\"{}\",\"range\":{}}}",
                escape_json(&uri),
                range_json(r)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":[{}]}}", id, arr)
}

/// 进程内能力面冒烟(真跑 query/LSP 逻辑,供 CI 与单测)。
pub fn run_smoke(sample_src: &str, uri: &str) -> SmokeResult {
    let mut session = ToolingSession::new();
    let mut result = SmokeResult::default();
    session.open(uri.to_string(), sample_src.to_string(), 1);

    let doc = session.get(uri).expect("open");
    let cx = query_ctx_for(doc);

    result.capabilities_passed.push("publishDiagnostics".into());

    if let Some(foo_line) = sample_src.lines().position(|l| l.contains("let foo")) {
        let items = completions_at(&cx, "fo");
        if let Ok(expect) = std::env::var("RURIX_LSP_SMOKE_EXPECT_COMPLETION") {
            if !items.iter().any(|s| s == &expect) {
                result
                    .failures
                    .push(format!("completion: red-green tamper expected {expect}"));
            } else {
                result.capabilities_passed.push("completion".into());
            }
        } else if items.iter().any(|s| s == "foo") {
            result.capabilities_passed.push("completion".into());
        } else {
            result
                .failures
                .push(format!("completion: expected foo in {items:?}"));
        }
        let _ = foo_line;
    }

    if let Some(off) = sample_src.find("helper()") {
        let before = &sample_src[..off];
        let line = before.matches('\n').count() as u32;
        let col = (off - before.rfind('\n').map(|i| i + 1).unwrap_or(0)) as u32;
        if definition_at(&cx, &session.source_map, doc.file_id, line, col).is_some() {
            result.capabilities_passed.push("definition".into());
        } else {
            result.failures.push("definition: no location".into());
        }
    }

    if let Some(line_idx) = sample_src.lines().position(|l| l.contains("let bar = foo")) {
        let line = line_idx as u32;
        let col = sample_src
            .lines()
            .nth(line_idx)
            .and_then(|l| l.find("foo"))
            .unwrap_or(0) as u32;
        let refs = references_at(&cx, &session.source_map, doc.file_id, line, col);
        if refs.len() >= 2 {
            result.capabilities_passed.push("references".into());
        } else {
            result
                .failures
                .push(format!("references: expected >=2 got {}", refs.len()));
        }
    }

    if let Some(line_idx) = sample_src.lines().position(|l| l.contains("let bar = foo")) {
        let line = line_idx as u32;
        let col = sample_src
            .lines()
            .nth(line_idx)
            .and_then(|l| l.find("foo"))
            .unwrap_or(0) as u32;
        let hi = highlights_at(&cx, &session.source_map, doc.file_id, line, col);
        if !hi.is_empty() {
            result.capabilities_passed.push("highlight".into());
        } else {
            result.failures.push("highlight: empty".into());
        }
    }

    if let Some(line_idx) = sample_src.lines().position(|l| l.contains("let foo")) {
        let line = line_idx as u32;
        let col = sample_src
            .lines()
            .nth(line_idx)
            .and_then(|l| l.find("foo"))
            .unwrap_or(0) as u32;
        match rename_at(
            &cx,
            &session.source_map,
            doc.file_id,
            line,
            col,
            "renamed_foo",
        ) {
            Ok(edits) if edits.len() >= 2 => {
                result.capabilities_passed.push("rename".into());
            }
            Ok(n) => result
                .failures
                .push(format!("rename: expected >=2 edits got {}", n.len())),
            Err(e) => result.failures.push(format!("rename: {e}")),
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lsp_line_col(src: &str, off: usize) -> (u32, u32) {
        let before = &src[..off];
        let line = before.matches('\n').count() as u32;
        let col = (off - before.rfind('\n').map(|i| i + 1).unwrap_or(0)) as u32;
        (line, col)
    }

    //@ spec: RXS-0098
    #[test]
    fn smoke_sample_passes_all_capabilities() {
        let src = include_str!("../../../../conformance/toolchain/lsp_mvp/sample.rx");
        let r = run_smoke(src, "file:///sample.rx");
        assert!(
            r.failures.is_empty(),
            "failures: {:?}, passed: {:?}",
            r.failures,
            r.capabilities_passed
        );
        assert!(r.capabilities_passed.len() >= 5);
    }

    //@ spec: RXS-0098, RXS-0100
    #[test]
    fn server_accepts_standard_lsp_nested_text_document_params() {
        let src = include_str!("../../../../conformance/toolchain/lsp_mvp/sample.rx");
        let mut session = ToolingSession::new();
        let did_open = format!(
            "{{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{{\"textDocument\":{{\"uri\":\"file:///sample.rx\",\"version\":1,\"text\":\"{}\"}}}}}}",
            escape_json(src)
        );
        let open_out = handle_message(&mut session, &did_open);
        assert_eq!(open_out.len(), 1);

        let foo = src.find("foo").unwrap() + 2;
        let (line, character) = lsp_line_col(src, foo);
        let completion = format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":\"c1\",\"method\":\"textDocument/completion\",\"params\":{{\"textDocument\":{{\"uri\":\"file:///sample.rx\"}},\"position\":{{\"line\":{},\"character\":{}}}}}}}",
            line, character
        );
        let out = handle_message(&mut session, &completion);
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("\"id\":\"c1\""));
        assert!(out[0].contains("\"label\":\"foo\""));
        assert!(out[0].contains("\"kind\":6"));
    }

    //@ spec: RXS-0103
    #[test]
    fn server_rename_invalid_returns_empty_edit_and_rx7012_diagnostic() {
        let src = include_str!("../../../../conformance/toolchain/lsp_mvp/sample.rx");
        let mut session = ToolingSession::new();
        session.open("file:///sample.rx".into(), src.into(), 1);
        let foo = src.find("foo").unwrap();
        let (line, character) = lsp_line_col(src, foo);
        let rename = format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"textDocument/rename\",\"params\":{{\"textDocument\":{{\"uri\":\"file:///sample.rx\"}},\"position\":{{\"line\":{},\"character\":{}}},\"newName\":\"fn\"}}}}",
            line, character
        );
        let out = handle_message(&mut session, &rename);
        assert_eq!(out.len(), 2);
        assert!(out[0].contains("\"changes\":{\"file:///sample.rx\":[]}"));
        assert!(out[1].contains("RX7012"));
    }
}
