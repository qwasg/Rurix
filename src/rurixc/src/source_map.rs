//! `SourceMap`:多文件注册、行列映射与 snippet 提取(07 §5 第 1 条)。
//!
//! 行列口径:1-based;列按**字符**计(诊断渲染口径,多字节 UTF-8 安全)。

use crate::span::{BytePos, Edition, SourceId, Span};

/// 1-based 行列位置。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LineCol {
    pub line: u32,
    pub col: u32,
}

pub struct SourceFile {
    pub id: SourceId,
    pub name: String,
    pub src: String,
    pub edition: Edition,
    /// 每行行首的字节偏移(首元素恒为 0),供二分查找。
    line_starts: Vec<u32>,
}

impl SourceFile {
    fn new(id: SourceId, name: String, src: String, edition: Edition) -> Self {
        let line_starts = line_starts(&src);
        Self {
            id,
            name,
            src,
            edition,
            line_starts,
        }
    }

    fn update_src(&mut self, src: String) {
        self.line_starts = line_starts(&src);
        self.src = src;
    }

    /// 字节偏移 → 1-based 行列(列按字符计)。
    pub fn lookup(&self, pos: BytePos) -> LineCol {
        debug_assert!(
            (pos.0 as usize) <= self.src.len(),
            "BytePos {} 超出文件 {}(len={})",
            pos.0,
            self.name,
            self.src.len()
        );
        let line_idx = match self.line_starts.binary_search(&pos.0) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        let line_start = self.line_starts[line_idx] as usize;
        let col = self.src[line_start..pos.0 as usize].chars().count() as u32 + 1;
        LineCol {
            line: line_idx as u32 + 1,
            col,
        }
    }

    /// 1-based 行号 → 该行文本(不含行尾换行符)。
    pub fn line_text(&self, line: u32) -> &str {
        let idx = (line - 1) as usize;
        let start = self.line_starts[idx] as usize;
        let end = self
            .line_starts
            .get(idx + 1)
            .map_or(self.src.len(), |&next| next as usize);
        self.src[start..end].trim_end_matches(['\n', '\r'])
    }

    pub fn line_count(&self) -> u32 {
        self.line_starts.len() as u32
    }

    /// LSP 0-based 行列 → 字节偏移(列按字符计;ASCII fixture 与 UTF-16 一致)。
    pub fn offset_at_lsp(&self, line: u32, character: u32) -> BytePos {
        let line_idx = line as usize;
        debug_assert!(line_idx < self.line_starts.len());
        let start = self.line_starts[line_idx] as usize;
        let text = self.line_text(line + 1);
        let mut byte = start;
        for (i, ch) in text.chars().enumerate() {
            if i as u32 >= character {
                break;
            }
            byte += ch.len_utf8();
        }
        BytePos(byte as u32)
    }

    pub fn src(&self) -> &str {
        &self.src
    }
}

fn line_starts(src: &str) -> Vec<u32> {
    let mut line_starts = vec![0u32];
    for (i, b) in src.bytes().enumerate() {
        if b == b'\n' {
            line_starts.push(i as u32 + 1);
        }
    }
    line_starts
}

#[derive(Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_file(
        &mut self,
        name: impl Into<String>,
        src: impl Into<String>,
        edition: Edition,
    ) -> SourceId {
        let id = SourceId(self.files.len() as u32);
        self.files
            .push(SourceFile::new(id, name.into(), src.into(), edition));
        id
    }

    pub fn file(&self, id: SourceId) -> &SourceFile {
        &self.files[id.0 as usize]
    }

    pub fn update_file(&mut self, id: SourceId, src: impl Into<String>) {
        self.files[id.0 as usize].update_src(src.into());
    }

    /// span 覆盖的源码文本。
    pub fn snippet(&self, span: Span) -> &str {
        let f = self.file(span.file);
        &f.src[span.lo.0 as usize..span.hi.0 as usize]
    }

    pub fn lookup(&self, id: SourceId, pos: BytePos) -> LineCol {
        self.file(id).lookup(pos)
    }

    /// 字节偏移 → LSP 0-based `(line, character)`(列按字符计)。
    pub fn to_lsp_position(&self, span: Span) -> (u32, u32) {
        let lc = self.lookup(span.file, span.lo);
        (lc.line - 1, lc.col - 1)
    }

    /// LSP 0-based 位置 → 字节偏移。
    pub fn from_lsp_position(&self, id: SourceId, line: u32, character: u32) -> BytePos {
        self.file(id).offset_at_lsp(line, character)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map_with(src: &str) -> (SourceMap, SourceId) {
        let mut sm = SourceMap::new();
        let id = sm.add_file("test.rx", src, Edition::Rx0);
        (sm, id)
    }

    #[test]
    fn empty_file_lookup() {
        let (sm, id) = map_with("");
        assert_eq!(sm.lookup(id, BytePos(0)), LineCol { line: 1, col: 1 });
        assert_eq!(sm.file(id).line_count(), 1);
    }

    #[test]
    fn line_start_and_end_positions() {
        // 偏移:  a=0 b=1 \n=2 c=3 d=4 \n=5
        let (sm, id) = map_with("ab\ncd\n");
        assert_eq!(sm.lookup(id, BytePos(0)), LineCol { line: 1, col: 1 });
        assert_eq!(sm.lookup(id, BytePos(2)), LineCol { line: 1, col: 3 }); // 行尾 \n 本身
        assert_eq!(sm.lookup(id, BytePos(3)), LineCol { line: 2, col: 1 }); // 次行行首
        assert_eq!(sm.lookup(id, BytePos(6)), LineCol { line: 3, col: 1 }); // EOF(末行为空)
    }

    #[test]
    fn multibyte_utf8_column_counts_chars() {
        // "变量x" = 3+3+1 字节;x 的字节偏移 6,字符列 3
        let (sm, id) = map_with("变量x = 1");
        assert_eq!(sm.lookup(id, BytePos(6)), LineCol { line: 1, col: 3 });
    }

    #[test]
    fn snippet_and_line_text() {
        let (sm, id) = map_with("let a = 1;\nlet bb = 2;\r\nlet c = 3;");
        let span = Span::new(id, 4, 5, Edition::Rx0);
        assert_eq!(sm.snippet(span), "a");
        assert_eq!(sm.file(id).line_text(1), "let a = 1;");
        assert_eq!(sm.file(id).line_text(2), "let bb = 2;"); // \r\n 剥除
        assert_eq!(sm.file(id).line_text(3), "let c = 3;"); // 末行无换行
    }

    #[test]
    fn multiple_files_independent_ids() {
        let mut sm = SourceMap::new();
        let a = sm.add_file("a.rx", "aaa", Edition::Rx0);
        let b = sm.add_file("b.rx", "b\nb", Edition::Rx0);
        assert_ne!(a, b);
        assert_eq!(sm.file(b).line_count(), 2);
        assert_eq!(sm.lookup(b, BytePos(2)), LineCol { line: 2, col: 1 });
    }
}
