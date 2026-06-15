//! 内容树规范化 SHA-256(RXS-0093)——逐字节复现根。
//!
//! 规范化消除非确定性源(09 §7.1 / M6_PLAN §6 风险):仅纳入相对路径(`/` 归一)
//! 与文件字节内容,排除时间戳/权限等元数据,文件项按相对路径字典序排序,排除
//! `vendor/`/`target/`/VCS 元数据。同一内容树在不同机器/时刻哈希一致——M6.3 两次
//! 重建逐字节比对的判据来源。

use std::path::Path;

use crate::sha256::{self, Sha256};

/// 内容树哈希排除的目录名(不纳入哈希)。
const EXCLUDED_DIRS: &[&str] = &["vendor", "target", ".git"];

/// 内容树哈希排除的文件名(生成物,不纳入哈希;否则写 lock 会扰动根哈希)。
const EXCLUDED_FILES: &[&str] = &["rurix.lock"];

/// 对已收集的 `(相对路径, 内容)` 项做规范化哈希(纯函数,确定性)。
///
/// 规范化:按相对路径字典序排序后,逐项喂入 `len(path)‖path‖len(content)‖content`
/// (长度为定宽 8 字节小端),再取 SHA-256 十六进制小写。长度前缀消歧义,避免
/// 不同 (path, content) 拼接碰撞。
pub fn hash_entries(entries: &[(String, Vec<u8>)]) -> String {
    let mut sorted: Vec<&(String, Vec<u8>)> = entries.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let mut h = Sha256::new();
    for (path, content) in sorted {
        let norm = path.replace('\\', "/");
        h.update(&(norm.len() as u64).to_le_bytes());
        h.update(norm.as_bytes());
        h.update(&(content.len() as u64).to_le_bytes());
        h.update(content);
    }
    sha256::hex(&h.finalize())
}

/// walk 一个包目录,收集内容树 `(相对路径, 内容)` 项(排除 EXCLUDED_DIRS)。
pub fn collect_dir(root: &Path) -> std::io::Result<Vec<(String, Vec<u8>)>> {
    let mut entries = Vec::new();
    walk(root, root, &mut entries)?;
    Ok(entries)
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) -> std::io::Result<()> {
    let mut children: Vec<_> = std::fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    children.sort_by_key(std::fs::DirEntry::file_name);
    for entry in children {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let ft = entry.file_type()?;
        if ft.is_dir() {
            if EXCLUDED_DIRS.contains(&name.as_ref()) {
                continue;
            }
            walk(root, &path, out)?;
        } else if ft.is_file() {
            if EXCLUDED_FILES.contains(&name.as_ref()) {
                continue;
            }
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let content = std::fs::read(&path)?;
            out.push((rel, content));
        }
    }
    Ok(())
}

/// 计算包目录的规范化内容树 SHA-256(RXS-0093)。
pub fn hash_dir(root: &Path) -> std::io::Result<String> {
    Ok(hash_entries(&collect_dir(root)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    //@ spec: RXS-0093
    #[test]
    fn order_independent_and_deterministic() {
        // 文件项顺序不影响哈希(规范化排序);同输入恒等。
        let a = vec![
            ("src/lib.rx".to_owned(), b"fn a(){}".to_vec()),
            ("rurix.toml".to_owned(), b"name=x".to_vec()),
        ];
        let mut b = a.clone();
        b.reverse();
        assert_eq!(hash_entries(&a), hash_entries(&b));
        assert_eq!(hash_entries(&a), hash_entries(&a));
    }

    //@ spec: RXS-0093
    #[test]
    fn path_and_content_changes_change_hash() {
        let base = vec![("a".to_owned(), b"x".to_vec())];
        let diff_content = vec![("a".to_owned(), b"y".to_vec())];
        let diff_path = vec![("b".to_owned(), b"x".to_vec())];
        assert_ne!(hash_entries(&base), hash_entries(&diff_content));
        assert_ne!(hash_entries(&base), hash_entries(&diff_path));
    }

    //@ spec: RXS-0093
    #[test]
    fn length_prefix_avoids_concat_collision() {
        // ("ab","c") 与 ("a","bc") 不应同哈希(长度前缀消歧义)。
        let x = vec![("ab".to_owned(), b"c".to_vec())];
        let y = vec![("a".to_owned(), b"bc".to_vec())];
        assert_ne!(hash_entries(&x), hash_entries(&y));
    }

    #[test]
    fn backslash_path_normalized() {
        let win = vec![("src\\lib.rx".to_owned(), b"z".to_vec())];
        let posix = vec![("src/lib.rx".to_owned(), b"z".to_vec())];
        assert_eq!(hash_entries(&win), hash_entries(&posix));
    }
}
