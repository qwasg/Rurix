//! 手写严格 JSON parser(RXS-0333)——拒重复 key、非法 UTF-8/裸控制字符、
//! 非有限数字、深度超限;保留对象内 key 出现序。

use crate::error::{AssetError, ErrorKind, Result};
use std::collections::HashSet;

const MAX_DEPTH: usize = 64;

/// 严格 JSON 值。对象保留插入序。
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    /// 有限整数(无小数/指数形态且落入 i64)。
    I64(i64),
    /// 有限整数(无小数/指数形态,落入 u64 但超出 i64)。
    /// 仅 `parse_str_u64`/`parse_bytes_u64` 全域入口产出(G11.3 R5 修复面:
    /// 契约 time.random_seed u64 顶格合法消费);默认 `parse_str` 入口对
    /// 超出 i64 的整数维持 fail-closed 拒绝(行为逐字节不变)。
    U64(u64),
    /// 有限浮点(含科学计数或小数形态)。
    F64(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    pub fn as_object(&self) -> Option<&[(String, JsonValue)]> {
        match self {
            JsonValue::Object(o) => Some(o),
            _ => None,
        }
    }
    pub fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            JsonValue::Array(a) => Some(a),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Bool(b) => Some(*b),
            _ => None,
        }
    }
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            JsonValue::I64(i) => Some(*i),
            JsonValue::U64(u) => i64::try_from(*u).ok(),
            JsonValue::F64(f)
                if f.fract() == 0.0 && *f >= i64::MIN as f64 && *f <= i64::MAX as f64 =>
            {
                Some(*f as i64)
            }
            _ => None,
        }
    }
    pub fn as_u32(&self) -> Option<u32> {
        self.as_i64().and_then(|i| u32::try_from(i).ok())
    }
    /// u64 域整数读取(G11.3 R5 修复面):I64 非负 / U64 全值 / F64 整值在域。
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            JsonValue::I64(i) => u64::try_from(*i).ok(),
            JsonValue::U64(u) => Some(*u),
            JsonValue::F64(f) if f.fract() == 0.0 && *f >= 0.0 && *f <= u64::MAX as f64 => {
                Some(*f as u64)
            }
            _ => None,
        }
    }
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            JsonValue::F64(f) => Some(*f),
            JsonValue::I64(i) => Some(*i as f64),
            JsonValue::U64(u) => Some(*u as f64),
            _ => None,
        }
    }
    pub fn get<'a>(&'a self, key: &str) -> Option<&'a JsonValue> {
        self.as_object()?
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }
}

struct Parser<'a> {
    bytes: &'a [u8],
    i: usize,
    /// u64 全域模式(G11.3 R5 修复面):true 时整数先 i64 后 u64 落地;
    /// false = 默认面——超出 i64 的整数 fail-closed 拒绝(逐字节既有行为)。
    u64_domain: bool,
}

impl<'a> Parser<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            i: 0,
            u64_domain: false,
        }
    }

    fn new_u64(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            i: 0,
            u64_domain: true,
        }
    }

    fn err(&self, msg: impl Into<String>) -> AssetError {
        AssetError::new(ErrorKind::JsonStrict, msg.into())
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.i).copied()
    }

    fn bump(&mut self) -> Result<u8> {
        let b = self
            .peek()
            .ok_or_else(|| self.err("unexpected end of JSON"))?;
        self.i += 1;
        Ok(b)
    }

    fn skip_ws(&mut self) {
        while let Some(b) = self.peek() {
            match b {
                b' ' | b'\t' | b'\n' | b'\r' => self.i += 1,
                _ => break,
            }
        }
    }

    fn parse_value(&mut self, depth: usize) -> Result<JsonValue> {
        if depth > MAX_DEPTH {
            return Err(self.err(format!("JSON depth exceeds {MAX_DEPTH}")));
        }
        self.skip_ws();
        match self.peek() {
            Some(b'n') => self.parse_null(),
            Some(b't') => self.parse_true(),
            Some(b'f') => self.parse_false(),
            Some(b'"') => Ok(JsonValue::String(self.parse_string()?)),
            Some(b'[') => self.parse_array(depth),
            Some(b'{') => self.parse_object(depth),
            Some(b'-') | Some(b'0'..=b'9') => self.parse_number(),
            Some(b) => Err(self.err(format!("unexpected byte 0x{b:02x} in JSON value"))),
            None => Err(self.err("unexpected end of JSON")),
        }
    }

    fn expect_literal(&mut self, lit: &[u8]) -> Result<()> {
        for &c in lit {
            let b = self.bump()?;
            if b != c {
                return Err(self.err(format!(
                    "invalid literal, expected {} got 0x{b:02x}",
                    String::from_utf8_lossy(lit)
                )));
            }
        }
        Ok(())
    }

    fn parse_null(&mut self) -> Result<JsonValue> {
        self.expect_literal(b"null")?;
        Ok(JsonValue::Null)
    }
    fn parse_true(&mut self) -> Result<JsonValue> {
        self.expect_literal(b"true")?;
        Ok(JsonValue::Bool(true))
    }
    fn parse_false(&mut self) -> Result<JsonValue> {
        self.expect_literal(b"false")?;
        Ok(JsonValue::Bool(false))
    }

    fn parse_string(&mut self) -> Result<String> {
        let quote = self.bump()?;
        if quote != b'"' {
            return Err(self.err("string must start with '\"'"));
        }
        let mut out = String::new();
        loop {
            let b = self.bump()?;
            match b {
                b'"' => return Ok(out),
                b'\\' => {
                    let e = self.bump()?;
                    match e {
                        b'"' | b'\\' | b'/' => out.push(e as char),
                        b'b' => out.push('\u{0008}'),
                        b'f' => out.push('\u{000c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let mut code = 0u32;
                            for _ in 0..4 {
                                let h = self.bump()?;
                                code = (code << 4)
                                    | hex_nibble(h)
                                        .ok_or_else(|| self.err("invalid \\u escape hex digit"))?;
                            }
                            let ch = char::from_u32(code).ok_or_else(|| {
                                self.err(format!("invalid unicode scalar U+{code:04X}"))
                            })?;
                            out.push(ch);
                        }
                        _ => {
                            return Err(self.err(format!("invalid string escape 0x{e:02x}")));
                        }
                    }
                }
                0x00..=0x1f => {
                    return Err(
                        self.err(format!("bare control character 0x{b:02x} in JSON string"))
                    );
                }
                _ => {
                    // UTF-8 multi-byte: validate sequence from current byte.
                    let width = utf8_width(b)
                        .ok_or_else(|| self.err(format!("invalid UTF-8 lead byte 0x{b:02x}")))?;
                    if width == 1 {
                        out.push(b as char);
                    } else {
                        let start = self.i - 1;
                        if start + width > self.bytes.len() {
                            return Err(self.err("truncated UTF-8 sequence"));
                        }
                        let slice = &self.bytes[start..start + width];
                        let s = std::str::from_utf8(slice)
                            .map_err(|_| self.err("invalid UTF-8 sequence"))?;
                        out.push_str(s);
                        self.i = start + width;
                    }
                }
            }
        }
    }

    fn parse_number(&mut self) -> Result<JsonValue> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        match self.peek() {
            Some(b'0') => self.i += 1,
            Some(b'1'..=b'9') => {
                self.i += 1;
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.i += 1;
                }
            }
            _ => return Err(self.err("invalid number integer part")),
        }
        let mut is_float = false;
        if self.peek() == Some(b'.') {
            is_float = true;
            self.i += 1;
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.err("invalid number fraction"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.i += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            is_float = true;
            self.i += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.i += 1;
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.err("invalid number exponent"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.i += 1;
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.i])
            .map_err(|_| self.err("number is not UTF-8"))?;
        if !is_float {
            match text.parse::<i64>() {
                Ok(i) => return Ok(JsonValue::I64(i)),
                Err(_) => {
                    // u64 全域入口(G11.3 R5):i64 溢出后非负整数尝试 u64 落地;
                    // 仍溢出(>u64::MAX 或负向越界)维持显式 Err,不静默降为 f64。
                    if self.u64_domain && !text.starts_with('-') {
                        if let Ok(u) = text.parse::<u64>() {
                            return Ok(JsonValue::U64(u));
                        }
                    }
                    // 超出 i64:显式 Err(溢出),不静默降为 f64。
                    return Err(self.err(format!("integer overflow: {text}")));
                }
            }
        }
        let f: f64 = text
            .parse()
            .map_err(|_| self.err(format!("invalid float: {text}")))?;
        if !f.is_finite() {
            return Err(self.err(format!("non-finite number: {text}")));
        }
        Ok(JsonValue::F64(f))
    }

    fn parse_array(&mut self, depth: usize) -> Result<JsonValue> {
        self.bump()?; // '['
        self.skip_ws();
        let mut items = Vec::new();
        if self.peek() == Some(b']') {
            self.i += 1;
            return Ok(JsonValue::Array(items));
        }
        loop {
            items.push(self.parse_value(depth + 1)?);
            self.skip_ws();
            match self.bump()? {
                b',' => {
                    self.skip_ws();
                    continue;
                }
                b']' => return Ok(JsonValue::Array(items)),
                b => return Err(self.err(format!("expected ',' or ']' in array, got 0x{b:02x}"))),
            }
        }
    }

    fn parse_object(&mut self, depth: usize) -> Result<JsonValue> {
        self.bump()?; // '{'
        self.skip_ws();
        let mut items = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        if self.peek() == Some(b'}') {
            self.i += 1;
            return Ok(JsonValue::Object(items));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return Err(self.err("object key must be a string"));
            }
            let key = self.parse_string()?;
            if !seen.insert(key.clone()) {
                return Err(self.err(format!("duplicate object key: {key}")));
            }
            self.skip_ws();
            let colon = self.bump()?;
            if colon != b':' {
                return Err(self.err("expected ':' after object key"));
            }
            let value = self.parse_value(depth + 1)?;
            items.push((key, value));
            self.skip_ws();
            match self.bump()? {
                b',' => continue,
                b'}' => return Ok(JsonValue::Object(items)),
                b => {
                    return Err(self.err(format!("expected ',' or '}}' in object, got 0x{b:02x}")));
                }
            }
        }
    }
}

fn hex_nibble(b: u8) -> Option<u32> {
    match b {
        b'0'..=b'9' => Some((b - b'0') as u32),
        b'a'..=b'f' => Some((b - b'a' + 10) as u32),
        b'A'..=b'F' => Some((b - b'A' + 10) as u32),
        _ => None,
    }
}

fn utf8_width(lead: u8) -> Option<usize> {
    if lead < 0x80 {
        Some(1)
    } else if lead & 0xe0 == 0xc0 {
        Some(2)
    } else if lead & 0xf0 == 0xe0 {
        Some(3)
    } else if lead & 0xf8 == 0xf0 {
        Some(4)
    } else {
        None
    }
}

/// 解析 UTF-8 JSON 文本(已是 str)。
pub fn parse_str(text: &str) -> Result<JsonValue> {
    parse_bytes(text.as_bytes())
}

/// 解析字节;非法 UTF-8 在字符串/整体层面拒录。
pub fn parse_bytes(bytes: &[u8]) -> Result<JsonValue> {
    parse_bytes_impl(bytes, false)
}

/// u64 全域入口(G11.3 R5 修复面):无小数/指数形态的非负整数先落 i64、
/// i64 溢出后落 u64(`JsonValue::U64`),超出 u64 维持 fail-closed;
/// 其余严格谓词(重复 key/UTF-8/裸控制字符/深度/非有限浮点)与默认面同。
pub fn parse_str_u64(text: &str) -> Result<JsonValue> {
    parse_bytes_impl(text.as_bytes(), true)
}

/// `parse_str_u64` 的字节形态。
pub fn parse_bytes_u64(bytes: &[u8]) -> Result<JsonValue> {
    parse_bytes_impl(bytes, true)
}

fn parse_bytes_impl(bytes: &[u8], u64_domain: bool) -> Result<JsonValue> {
    // 整体必须是合法 UTF-8(GLB JSON chunk 也走此路径)。
    std::str::from_utf8(bytes)
        .map_err(|_| AssetError::new(ErrorKind::JsonStrict, "JSON bytes are not valid UTF-8"))?;
    let mut p = if u64_domain {
        Parser::new_u64(bytes)
    } else {
        Parser::new(bytes)
    };
    let v = p.parse_value(0)?;
    p.skip_ws();
    if p.i != p.bytes.len() {
        return Err(AssetError::new(
            ErrorKind::JsonStrict,
            "trailing garbage after JSON value",
        ));
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0333
    #[test]
    fn rejects_duplicate_keys() {
        let r = parse_str(r#"{"a":1,"a":2}"#);
        assert!(r.is_err());
        assert_eq!(r.unwrap_err().kind, ErrorKind::JsonStrict);
    }

    //@ spec: RXS-0333
    #[test]
    fn preserves_key_order() {
        let v = parse_str(r#"{"z":1,"a":2}"#).unwrap();
        let keys: Vec<&str> = v
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, _)| k.as_str())
            .collect();
        assert_eq!(keys, ["z", "a"]);
    }

    //@ spec: RXS-0333
    #[test]
    fn rejects_bare_control_in_string() {
        assert!(parse_str("{\"a\":\"\n\"}").is_err());
    }

    //@ spec: RXS-0333
    #[test]
    fn rejects_integer_overflow() {
        assert!(parse_str("9223372036854775808").is_err());
    }

    // G11.3 R5 修复面:u64 全域入口合法消费 i64 域外整数,默认面维持拒绝。
    #[test]
    fn u64_domain_entry_accepts_u64_max() {
        let v = parse_str_u64("18446744073709551615").unwrap();
        assert_eq!(v, JsonValue::U64(u64::MAX));
        assert_eq!(v.as_u64(), Some(u64::MAX));
        // i64 域内整数两入口同型落地(既有消费面 0-byte)。
        let w = parse_str_u64("42").unwrap();
        assert_eq!(w, JsonValue::I64(42));
        assert_eq!(w.as_u64(), Some(42));
        // i64 上界邻域:2^63−1 两入口均为 I64;2^63 仅 u64 入口落地。
        assert_eq!(
            parse_str_u64("9223372036854775807").unwrap(),
            JsonValue::I64(i64::MAX)
        );
        assert_eq!(
            parse_str_u64("9223372036854775808").unwrap(),
            JsonValue::U64(9223372036854775808)
        );
    }

    #[test]
    fn u64_domain_entry_still_fail_closed_beyond_u64() {
        // 2^64 与负向越界维持显式拒绝(不静默降为 f64)。
        assert!(parse_str_u64("18446744073709551616").is_err());
        assert!(parse_str_u64("-9223372036854775809").is_err());
        // 默认面对 u64 域外整数维持逐字节既有拒绝(G10 探针 parity 面)。
        assert!(parse_str("18446744073709551615").is_err());
        let e = parse_str("18446744073709551615").unwrap_err();
        assert!(e.message.contains("integer overflow"));
    }
}
