//! 最小 JSON 读写(无 serde;LSP stdio 通道专用)。

/// 从 JSON 对象体中提取 `"key":"value"` 字符串值(浅层;会跳过字符串内的括号)。
pub fn json_str_field(body: &str, key: &str) -> Option<String> {
    let rest = json_value_field(body, key)?.trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let mut out = String::new();
    let mut chars = rest[1..].chars();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                let esc = chars.next()?;
                match esc {
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    _ => out.push(esc),
                }
            }
            '"' => return Some(out),
            c => out.push(c),
        }
    }
    None
}

pub fn json_i64_field(body: &str, key: &str) -> Option<i64> {
    let rest = json_value_field(body, key)?.trim_start();
    let end = rest
        .find(|c: char| !c.is_ascii_digit() && c != '-')
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

pub fn json_object_field<'a>(body: &'a str, key: &str) -> Option<&'a str> {
    let rest = json_value_field(body, key)?.trim_start();
    bounded_json_value(rest, '{', '}')
}

pub fn json_array_field<'a>(body: &'a str, key: &str) -> Option<&'a str> {
    let rest = json_value_field(body, key)?.trim_start();
    bounded_json_value(rest, '[', ']')
}

pub fn json_top_level_value_field<'a>(body: &'a str, key: &str) -> Option<&'a str> {
    json_value_field_at_depth(body, key, 1)
}

fn json_value_field<'a>(body: &'a str, key: &str) -> Option<&'a str> {
    json_value_field_at_depth(body, key, -1)
}

fn json_value_field_at_depth<'a>(body: &'a str, key: &str, wanted_depth: i32) -> Option<&'a str> {
    let needle = format!("\"{key}\"");
    let bytes = body.as_bytes();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut escaped = false;
    let mut i = 0usize;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if in_str {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match ch {
            '"' => {
                if (wanted_depth < 0 || depth == wanted_depth) && body[i..].starts_with(&needle) {
                    let mut j = i + needle.len();
                    while j < bytes.len() && (bytes[j] as char).is_ascii_whitespace() {
                        j += 1;
                    }
                    if j < bytes.len() && bytes[j] == b':' {
                        return Some(&body[j + 1..]);
                    }
                }
                in_str = true;
            }
            '{' | '[' => depth += 1,
            '}' | ']' => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    None
}

fn bounded_json_value(rest: &str, open: char, close: char) -> Option<&str> {
    if !rest.starts_with(open) {
        return None;
    }
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
            c if c == open => depth += 1,
            c if c == close => {
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

pub fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_method_and_uri() {
        let body = r#"{"method":"textDocument/completion","params":{"uri":"file:///a.rx"}}"#;
        assert_eq!(
            json_str_field(body, "method").as_deref(),
            Some("textDocument/completion")
        );
        let params = json_object_field(body, "params").unwrap();
        assert_eq!(
            json_str_field(params, "uri").as_deref(),
            Some("file:///a.rx")
        );
    }

    #[test]
    fn top_level_id_ignores_nested_id_string() {
        let body = r#"{"method":"x","params":{"id":1,"text":"{\"id\":2}"}}"#;
        assert!(json_top_level_value_field(body, "id").is_none());
    }
}
