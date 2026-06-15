//! 手写 TOML 子集解析器(RXS-0089)——零外部依赖、确定性。
//!
//! 支持 `rurix.toml` 与 `rurix.lock` 所需的最小 TOML 子集:
//! - `[table]` / `[table.sub]` 表头;`[[array.table]]` 数组表头;
//! - `key = value`,value ∈ 字符串 / 整数 / 布尔 / 字符串(或值)数组 / 内联表;
//! - `#` 行注释与值后行尾注释(字符串内 `#` 不视作注释)。
//!
//! 表以 [`BTreeMap`] 存(键有序)→ 序列化确定性(逐字节复现铺底,RXS-0092/0093)。
//! 数组/内联表为单逻辑行(子集约束,清单/锁均满足)。解析错误返回 `Err(String)`,
//! 调用方(manifest/lock)归一为 [`crate::PkgError`]。

use std::collections::BTreeMap;

/// TOML 子集值。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    String(String),
    Integer(i64),
    Boolean(bool),
    Array(Vec<Value>),
    Table(BTreeMap<String, Value>),
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_table(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Value::Table(t) => Some(t),
            _ => None,
        }
    }
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Value::Integer(i) => Some(*i),
            _ => None,
        }
    }
    /// 字符串数组便捷取值(成员/feature 列表常用);非字符串元素 → Err。
    pub fn as_str_array(&self) -> Result<Vec<String>, String> {
        match self {
            Value::Array(a) => a
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| "数组元素非字符串".to_owned())
                })
                .collect(),
            _ => Err("期待字符串数组".to_owned()),
        }
    }
}

/// 解析 TOML 子集文本 → 根表。
pub fn parse(text: &str) -> Result<BTreeMap<String, Value>, String> {
    let mut root: BTreeMap<String, Value> = BTreeMap::new();
    let mut cur_path: Vec<String> = Vec::new();
    let mut cur_is_array = false;

    for (lineno, raw) in text.lines().enumerate() {
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        let n = lineno + 1;
        if let Some(rest) = line.strip_prefix("[[") {
            let inner = rest
                .strip_suffix("]]")
                .ok_or_else(|| format!("第 {n} 行:未闭合 [[array.table]] 头"))?;
            cur_path = parse_header_path(inner.trim()).map_err(|e| format!("第 {n} 行:{e}"))?;
            cur_is_array = true;
            open_array_table(&mut root, &cur_path).map_err(|e| format!("第 {n} 行:{e}"))?;
        } else if let Some(rest) = line.strip_prefix('[') {
            let inner = rest
                .strip_suffix(']')
                .ok_or_else(|| format!("第 {n} 行:未闭合 [table] 头"))?;
            cur_path = parse_header_path(inner.trim()).map_err(|e| format!("第 {n} 行:{e}"))?;
            cur_is_array = false;
            open_table(&mut root, &cur_path).map_err(|e| format!("第 {n} 行:{e}"))?;
        } else {
            let (key, value) = parse_assignment(line).map_err(|e| format!("第 {n} 行:{e}"))?;
            insert_current(&mut root, &cur_path, cur_is_array, &key, value)
                .map_err(|e| format!("第 {n} 行:{e}"))?;
        }
    }
    Ok(root)
}

fn parse_header_path(s: &str) -> Result<Vec<String>, String> {
    if s.is_empty() {
        return Err("空表头".to_owned());
    }
    let mut parts = Vec::new();
    for seg in s.split('.') {
        let seg = seg.trim();
        if !is_bare_key(seg) {
            return Err(format!("非法表头键 {seg:?}"));
        }
        parts.push(seg.to_owned());
    }
    Ok(parts)
}

fn is_bare_key(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// 去除行尾注释(字符串内 `#` 不算注释)。
fn strip_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut in_str = false;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => in_str = !in_str,
            b'\\' if in_str => i += 1, // 跳过转义下一字节
            b'#' if !in_str => return &line[..i],
            _ => {}
        }
        i += 1;
    }
    line
}

fn open_table(root: &mut BTreeMap<String, Value>, path: &[String]) -> Result<(), String> {
    descend_table(root, path).map(|_| ())
}

fn open_array_table(root: &mut BTreeMap<String, Value>, path: &[String]) -> Result<(), String> {
    let (last, prefix) = path.split_last().ok_or_else(|| "空数组表头".to_owned())?;
    let parent = descend_table(root, prefix)?;
    let entry = parent
        .entry(last.clone())
        .or_insert_with(|| Value::Array(Vec::new()));
    match entry {
        Value::Array(arr) => {
            arr.push(Value::Table(BTreeMap::new()));
            Ok(())
        }
        _ => Err(format!("键 {last:?} 已存在且非数组表")),
    }
}

/// 沿表路径下降(中间均为子表),返回末端表的可变引用。
fn descend_table<'a>(
    root: &'a mut BTreeMap<String, Value>,
    path: &[String],
) -> Result<&'a mut BTreeMap<String, Value>, String> {
    let mut cur = root;
    for seg in path {
        let entry = cur
            .entry(seg.clone())
            .or_insert_with(|| Value::Table(BTreeMap::new()));
        match entry {
            Value::Table(t) => cur = t,
            _ => return Err(format!("键 {seg:?} 已存在且非表")),
        }
    }
    Ok(cur)
}

fn insert_current(
    root: &mut BTreeMap<String, Value>,
    path: &[String],
    is_array: bool,
    key: &str,
    value: Value,
) -> Result<(), String> {
    if path.is_empty() {
        return insert_unique(root, key, value);
    }
    if is_array {
        let (last, prefix) = path.split_last().unwrap();
        let parent = descend_table(root, prefix)?;
        let arr = match parent.get_mut(last) {
            Some(Value::Array(a)) => a,
            _ => return Err(format!("数组表 {last:?} 不存在")),
        };
        let tbl = match arr.last_mut() {
            Some(Value::Table(t)) => t,
            _ => return Err("数组表末项非表".to_owned()),
        };
        insert_unique(tbl, key, value)
    } else {
        let tbl = descend_table(root, path)?;
        insert_unique(tbl, key, value)
    }
}

fn insert_unique(tbl: &mut BTreeMap<String, Value>, key: &str, value: Value) -> Result<(), String> {
    if tbl.contains_key(key) {
        return Err(format!("重复键 {key:?}"));
    }
    tbl.insert(key.to_owned(), value);
    Ok(())
}

fn parse_assignment(line: &str) -> Result<(String, Value), String> {
    let eq = line.find('=').ok_or_else(|| "赋值缺 '='".to_owned())?;
    let key = line[..eq].trim();
    if !is_bare_key(key) {
        return Err(format!("非法键 {key:?}"));
    }
    let (value, rest) = parse_value(line[eq + 1..].trim())?;
    if !rest.trim().is_empty() {
        return Err(format!("值之后有多余内容 {:?}", rest.trim()));
    }
    Ok((key.to_owned(), value))
}

/// 解析单个值,返回 (值, 剩余串)。供数组/内联表递归调用。
fn parse_value(s: &str) -> Result<(Value, &str), String> {
    let s = s.trim_start();
    let first = s.chars().next().ok_or_else(|| "缺值".to_owned())?;
    match first {
        '"' => parse_string(s),
        '[' => parse_array(s),
        '{' => parse_inline_table(s),
        't' | 'f' => parse_bool(s),
        c if c == '-' || c.is_ascii_digit() => parse_integer(s),
        _ => Err(format!("无法识别的值起始 {first:?}")),
    }
}

fn parse_string(s: &str) -> Result<(Value, &str), String> {
    let bytes = s.as_bytes();
    debug_assert_eq!(bytes[0], b'"');
    let mut out = String::new();
    let mut i = 1;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => return Ok((Value::String(out), &s[i + 1..])),
            b'\\' => {
                i += 1;
                let esc = *bytes.get(i).ok_or_else(|| "字符串转义未终结".to_owned())?;
                match esc {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'n' => out.push('\n'),
                    b't' => out.push('\t'),
                    b'r' => out.push('\r'),
                    other => return Err(format!("不支持的转义 \\{}", other as char)),
                }
            }
            _ => {
                // 以字节边界推进,UTF-8 多字节原样纳入
                let ch = s[i..].chars().next().unwrap();
                out.push(ch);
                i += ch.len_utf8();
                continue;
            }
        }
        i += 1;
    }
    Err("未终结字符串字面量".to_owned())
}

fn parse_bool(s: &str) -> Result<(Value, &str), String> {
    if let Some(rest) = s.strip_prefix("true") {
        Ok((Value::Boolean(true), rest))
    } else if let Some(rest) = s.strip_prefix("false") {
        Ok((Value::Boolean(false), rest))
    } else {
        Err("非法布尔值".to_owned())
    }
}

fn parse_integer(s: &str) -> Result<(Value, &str), String> {
    let end = s
        .char_indices()
        .find(|(i, c)| !(c.is_ascii_digit() || (*i == 0 && *c == '-')))
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    let (num, rest) = s.split_at(end);
    let v: i64 = num.parse().map_err(|_| format!("非法整数 {num:?}"))?;
    Ok((Value::Integer(v), rest))
}

fn parse_array(s: &str) -> Result<(Value, &str), String> {
    let mut rest = &s[1..]; // skip '['
    let mut items = Vec::new();
    loop {
        rest = rest.trim_start();
        if let Some(r) = rest.strip_prefix(']') {
            return Ok((Value::Array(items), r));
        }
        if rest.is_empty() {
            return Err("未闭合数组".to_owned());
        }
        let (v, r) = parse_value(rest)?;
        items.push(v);
        rest = r.trim_start();
        if let Some(r) = rest.strip_prefix(',') {
            rest = r;
        } else if let Some(r) = rest.strip_prefix(']') {
            return Ok((Value::Array(items), r));
        } else {
            return Err(format!("数组元素后期待 ',' 或 ']',实得 {:?}", rest));
        }
    }
}

fn parse_inline_table(s: &str) -> Result<(Value, &str), String> {
    let mut rest = &s[1..]; // skip '{'
    let mut tbl: BTreeMap<String, Value> = BTreeMap::new();
    loop {
        rest = rest.trim_start();
        if let Some(r) = rest.strip_prefix('}') {
            return Ok((Value::Table(tbl), r));
        }
        if rest.is_empty() {
            return Err("未闭合内联表".to_owned());
        }
        let eq = rest.find('=').ok_or_else(|| "内联表项缺 '='".to_owned())?;
        let key = rest[..eq].trim();
        if !is_bare_key(key) {
            return Err(format!("内联表非法键 {key:?}"));
        }
        let (v, r) = parse_value(rest[eq + 1..].trim_start())?;
        if tbl.contains_key(key) {
            return Err(format!("内联表重复键 {key:?}"));
        }
        tbl.insert(key.to_owned(), v);
        rest = r.trim_start();
        if let Some(r) = rest.strip_prefix(',') {
            rest = r;
        } else if let Some(r) = rest.strip_prefix('}') {
            return Ok((Value::Table(tbl), r));
        } else {
            return Err(format!("内联表项后期待 ',' 或 '}}',实得 {:?}", rest));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0089
    #[test]
    fn parses_tables_inline_and_arrays() {
        let text = r#"
# 注释行
[package]
name = "demo"   # 行尾注释
version = "0.1.0"

[dependencies]
foo = { path = "../foo" }
bar = { git = "https://x/y", rev = "abc", features = ["a", "b"] }

[features]
default = ["a"]
"#;
        let root = parse(text).expect("parse ok");
        let pkg = root["package"].as_table().unwrap();
        assert_eq!(pkg["name"].as_str(), Some("demo"));
        let deps = root["dependencies"].as_table().unwrap();
        let bar = deps["bar"].as_table().unwrap();
        assert_eq!(bar["rev"].as_str(), Some("abc"));
        assert_eq!(bar["features"].as_str_array().unwrap(), vec!["a", "b"]);
        let feats = root["features"].as_table().unwrap();
        assert_eq!(feats["default"].as_str_array().unwrap(), vec!["a"]);
    }

    //@ spec: RXS-0092
    #[test]
    fn parses_array_of_tables() {
        let text = r#"
lock_version = 1
root = "app"

[[package]]
name = "a"
deps = ["b"]

[[package]]
name = "b"
deps = []
"#;
        let root = parse(text).expect("parse ok");
        assert_eq!(root["lock_version"].as_integer(), Some(1));
        let pkgs = root["package"].as_array().unwrap();
        assert_eq!(pkgs.len(), 2);
        assert_eq!(pkgs[0].as_table().unwrap()["name"].as_str(), Some("a"));
        assert_eq!(pkgs[1].as_table().unwrap()["name"].as_str(), Some("b"));
    }

    //@ spec: RXS-0089
    #[test]
    fn rejects_duplicate_key_and_unclosed() {
        assert!(parse("[package]\nname = \"a\"\nname = \"b\"\n").is_err());
        assert!(parse("[package\n").is_err());
        assert!(parse("foo = { path = \"x\" \n").is_err());
        assert!(parse("foo = \"unterminated\n").is_err());
    }

    #[test]
    fn hash_inside_string_not_a_comment() {
        let root = parse("[package]\nname = \"a#b\"\n").unwrap();
        assert_eq!(
            root["package"].as_table().unwrap()["name"].as_str(),
            Some("a#b")
        );
    }
}
