//! message-key 骨架(07 §5 第 7 条,D-206):诊断文本统一经 key → 模板表。
//!
//! - 表为自有行格式(`key = 模板`),静态内嵌(`include_str!`),**不用 build.rs**;
//! - key 有效性在 [`crate::diag::DiagCtxt`] 构造诊断时强制校验,未注册 key 即 ICE
//!   ——"编译期校验 key 有效性"的 M1.1 形态,配合下方全量解析单测;
//! - 单语基线;中英双语全量覆盖 → RD-006(M8)。

use std::collections::HashMap;
use std::sync::OnceLock;

/// 内嵌消息源(en);与 `registry/error_codes.json` 的 message_key 互查。
pub const EN_SOURCE: &str = include_str!("messages/en.messages");

/// 内嵌消息源(zh);中英双语全量覆盖(RD-006,M8)。key 集与 [`EN_SOURCE`] 完全对齐
/// ——`zh_aligns_with_en` 单测 + `ci/bilingual_coverage.py` 双语覆盖门(步骤 37)交叉守护,
/// 缺键即红。
pub const ZH_SOURCE: &str = include_str!("messages/zh.messages");

#[derive(Clone, Debug, Default)]
pub struct MessageTable {
    map: HashMap<String, String>,
}

impl MessageTable {
    /// 解析行格式:`key = 模板`;`#` 行注释;空行忽略。
    /// 重复 key 或缺 `=` 的非注释行 → Err(表损坏即 ICE 级问题,由调用方裁决)。
    pub fn parse(src: &str) -> Result<Self, String> {
        let mut map = HashMap::new();
        for (lineno, line) in src.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, template)) = line.split_once('=') else {
                return Err(format!("第 {} 行缺 '=': {line:?}", lineno + 1));
            };
            let key = key.trim();
            if key.is_empty() || key.contains(char::is_whitespace) {
                return Err(format!("第 {} 行 key 非法: {key:?}", lineno + 1));
            }
            if map
                .insert(key.to_owned(), template.trim().to_owned())
                .is_some()
            {
                return Err(format!("第 {} 行 key 重复: {key:?}", lineno + 1));
            }
        }
        Ok(Self { map })
    }

    pub fn contains(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// 渲染:`{name}` 占位替换;key 未注册返回 None。
    pub fn render(&self, key: &str, args: &[(&str, &str)]) -> Option<String> {
        let mut text = self.map.get(key)?.clone();
        for (name, value) in args {
            text = text.replace(&format!("{{{name}}}"), value);
        }
        Some(text)
    }
}

/// 全局 en 消息表;内嵌源解析失败即 ICE(panic)。
pub fn table() -> &'static MessageTable {
    static TABLE: OnceLock<MessageTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        MessageTable::parse(EN_SOURCE)
            .unwrap_or_else(|e| panic!("ICE: 内嵌消息表 en.messages 损坏: {e}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_table_parses() {
        // key 有效性通道的兜底:内嵌源必须始终可解析(损坏即此测试红)
        let _ = table();
    }

    #[test]
    fn zh_aligns_with_en() {
        // 中英双语全量覆盖(RD-006,M8):zh 与 en 的 key 集必须完全对齐(缺键/多键即红)。
        // 与 ci/bilingual_coverage.py(步骤 37)互为双保险。
        use std::collections::BTreeSet;
        let en = MessageTable::parse(EN_SOURCE).expect("en.messages 必须可解析");
        let zh = MessageTable::parse(ZH_SOURCE).expect("zh.messages 必须可解析");
        let en_keys: BTreeSet<&str> = en.map.keys().map(String::as_str).collect();
        let zh_keys: BTreeSet<&str> = zh.map.keys().map(String::as_str).collect();
        let missing_in_zh: Vec<&&str> = en_keys.difference(&zh_keys).collect();
        let extra_in_zh: Vec<&&str> = zh_keys.difference(&en_keys).collect();
        assert!(
            missing_in_zh.is_empty() && extra_in_zh.is_empty(),
            "zh/en message-key 集不对齐:zh 缺 {missing_in_zh:?};zh 多 {extra_in_zh:?}"
        );
    }

    #[test]
    fn parse_synthetic_table_and_render() {
        let t = MessageTable::parse(
            "# comment\n\nlex.bad_char = unexpected character {ch}\nparse.eof = unexpected end of file\n",
        )
        .unwrap();
        assert_eq!(t.len(), 2);
        assert!(t.contains("lex.bad_char"));
        assert_eq!(
            t.render("lex.bad_char", &[("ch", "'@'")]).unwrap(),
            "unexpected character '@'"
        );
        assert_eq!(
            t.render("parse.eof", &[]).unwrap(),
            "unexpected end of file"
        );
        assert!(t.render("unknown.key", &[]).is_none());
    }

    #[test]
    fn parse_rejects_duplicate_key() {
        assert!(MessageTable::parse("a.b = x\na.b = y\n").is_err());
    }

    #[test]
    fn parse_rejects_malformed_line() {
        assert!(MessageTable::parse("not a key value line\n").is_err());
        assert!(MessageTable::parse("bad key = x\n").is_err());
    }
}
