//! 语料/golden 批跑测试的共享工具(Direct 档重构:抽取各 `tests/*.rs` 逐字节
//! 重复的目录定位、`.rx` 收集、`//@ expect-error` 头解析、golden bless 判据)。
//!
//! 集成测试各自成 crate,故本模块以 `mod common;` 内联而非发布 API;各测试只用其中
//! 一部分,顶层 `allow(dead_code)` 使未用项不触 `-D warnings`。
//!
//! 口径不变:递归 `rx_files` 对缺失目录返回空(由调用方 `assert!(!files.is_empty())`
//! 给出语料级报错),`expect_error_code` 缺头即 panic。

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

/// 仓库根(`src/rurixc` 的上两级)。
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `conformance/<rel>`(唯一验收边界,10 §7)。
pub fn conformance_dir(rel: impl AsRef<Path>) -> PathBuf {
    repo_root().join("conformance").join(rel)
}

/// `tests/<rel>`(golden 语料根)。
pub fn tests_dir(rel: impl AsRef<Path>) -> PathBuf {
    repo_root().join("tests").join(rel)
}

/// 递归收集 `root` 下全部 `.rx`,路径序稳定;`root` 不存在时返回空。
pub fn rx_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if !root.is_dir() {
        return out;
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap_or_else(|e| panic!("读取 {} 失败: {e}", d.display())) {
            let p = e.expect("读取目录项失败").path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rx") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// 仅收集 `root` 直属(不递归)的 `.rx`,路径序稳定;`root` 不存在时返回空。
pub fn rx_files_shallow(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if !root.is_dir() {
        return out;
    }
    for e in fs::read_dir(root).unwrap_or_else(|e| panic!("读取 {} 失败: {e}", root.display()))
    {
        let p = e.expect("读取目录项失败").path();
        if p.extension().is_some_and(|x| x == "rx") {
            out.push(p);
        }
    }
    out.sort();
    out
}

/// 读取样例源码。
pub fn read_source(path: &Path) -> String {
    fs::read_to_string(path).expect("读取样例失败")
}

/// 解析反例文件头的 `//@ expect-error: RX####`,缺头或码非法即 panic。
pub fn expect_error_code(src: &str, path: &Path) -> u16 {
    src.lines()
        .find_map(|l| l.trim().strip_prefix("//@ expect-error: RX"))
        .unwrap_or_else(|| panic!("{} 缺 //@ expect-error: RX#### 头", path.display()))
        .trim()
        .parse()
        .expect("expect-error 码格式非法")
}

/// golden bless 判据(`RURIX_BLESS=1`;bless 是审批动作,须伴随 bless_log 留痕)。
pub fn bless_mode() -> bool {
    std::env::var("RURIX_BLESS").is_ok_and(|v| v == "1")
}

/// CRLF → LF(golden 跨平台逐字节比对前的唯一归一化)。
pub fn normalize_newlines(s: &str) -> String {
    s.replace("\r\n", "\n")
}

/// 读取语料并归一化行尾(golden 比对入口的统一读法)。
pub fn read_corpus_normalized(path: &Path) -> String {
    normalize_newlines(&fs::read_to_string(path).expect("读取语料失败"))
}

/// 条款锚定头断言(traceability:首行 `//@ spec: RXS-####`)。
pub fn assert_spec_anchor(src: &str, path: &Path) {
    assert!(
        src.lines()
            .next()
            .unwrap_or("")
            .starts_with("//@ spec: RXS-"),
        "{} 缺条款锚定头(//@ spec: RXS-####)",
        path.display()
    );
}

/// golden 逐字节比对(bless 模式改为重写);一致返回 `None`,否则返回可读的漂移/缺失
/// 说明。`label` 用于漂移消息的形态名(如 `MIR` / `NVPTX IR`),缺 golden 的消息用
/// golden 自身扩展名。
pub fn check_golden(golden: &Path, produced: &str, bless: bool, label: &str) -> Option<String> {
    if bless {
        fs::write(golden, produced).expect("bless 写入失败");
        return None;
    }
    let ext = golden
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_default();
    match fs::read_to_string(golden) {
        Ok(s) if normalize_newlines(&s) == produced => None,
        Ok(s) => Some(format!(
            "{}: {label} golden 漂移\n--- expected ---\n{}\n--- actual ---\n{produced}",
            golden.display(),
            normalize_newlines(&s)
        )),
        Err(_) => Some(format!(
            "{}: 缺 .{ext} golden(新语料需经审批 bless:RURIX_BLESS=1 + bless_log.md 留痕)",
            golden.display()
        )),
    }
}
