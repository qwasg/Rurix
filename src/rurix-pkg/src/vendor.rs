//! vendor/ 与离线解析路径(RXS-0094)+ filesystem PackageLoader(RXS-0090)。
//!
//! `rx vendor` 解析图后将 path 依赖内容落 `vendor/<name>/` 并写 `rurix.lock`
//! (含每包内容树 SHA-256)。`--offline` 仅许本地来源(path + vendor 缓存),
//! 远端来源无缓存 → RX7009。`--locked` 不重写 lock,校验重解析图一致(RX7007)
//! + vendor 内容树 digest 一致(RX7008)。为 M6.3 三包离线重建逐字节复现门铺底。

use std::path::{Path, PathBuf};

use crate::content_tree;
use crate::error::{PkgError, PkgResult};
use crate::lock::Lock;
use crate::manifest::{Manifest, Source};
use crate::resolve::{self, LoadedPackage, PackageLoader, ResolveGraph};

const MANIFEST_NAME: &str = "rurix.toml";
const LOCK_NAME: &str = "rurix.lock";
const VENDOR_DIR: &str = "vendor";

/// filesystem 包加载器:path 源相对 `base_dir` 解析,远端源经 vendor 缓存或离线裁决。
pub struct FsLoader {
    base_dir: PathBuf,
    offline: bool,
}

impl FsLoader {
    pub fn new(base_dir: &Path, offline: bool) -> FsLoader {
        FsLoader {
            base_dir: base_dir.to_path_buf(),
            offline,
        }
    }
}

impl PackageLoader for FsLoader {
    fn load(&self, name: &str, source: &Source) -> PkgResult<LoadedPackage> {
        match source {
            Source::Path(rel) => {
                let dir = self.base_dir.join(rel);
                if !dir.join(MANIFEST_NAME).is_file() {
                    return Err(PkgError::SourceUnreachable(format!(
                        "依赖 {name:?} 的 path 源不可达:{} 无 {MANIFEST_NAME}",
                        dir.display()
                    )));
                }
                load_dir(&dir, rel, source.clone())
            }
            Source::Git { .. } | Source::Archive { .. } => {
                // 远端源:仅当 vendor/<name> 缓存存在时离线可用;否则不可达(M6.2
                // 不抓取网络,三包远端来源端到端归 M6.3,G-M6-1)。
                let cached = self.base_dir.join(VENDOR_DIR).join(name);
                if cached.join(MANIFEST_NAME).is_file() {
                    let rel = format!("{VENDOR_DIR}/{name}");
                    load_dir(&cached, &rel, source.clone())
                } else if self.offline {
                    Err(PkgError::SourceUnreachable(format!(
                        "依赖 {name:?} 远端源在 --offline 下不可达(无 vendor 缓存):{}",
                        source.locator()
                    )))
                } else {
                    Err(PkgError::SourceUnreachable(format!(
                        "依赖 {name:?} 远端源 {} 的网络抓取未在 M6.2 实现(归 M6.3 G-M6-1);请先 vendor",
                        source.locator()
                    )))
                }
            }
        }
    }
}

/// 加载某目录的清单并改写其 path 依赖为相对 base_dir(`pkg_rel` = 该包相对 base_dir
/// 的目录),保证解析图来源 locator 一致(避免相对路径视角差异引入伪冲突)。
fn load_dir(dir: &Path, pkg_rel: &str, source: Source) -> PkgResult<LoadedPackage> {
    let text = std::fs::read_to_string(dir.join(MANIFEST_NAME)).map_err(|e| {
        PkgError::ManifestInvalid(format!(
            "读取 {} 失败:{e}",
            dir.join(MANIFEST_NAME).display()
        ))
    })?;
    let mut manifest = Manifest::parse(&text)?;
    for dep in manifest.dependencies.values_mut() {
        if let Source::Path(p) = &dep.source {
            dep.source = Source::Path(join_normalize(pkg_rel, p));
        }
    }
    let content_sha256 = content_tree::hash_dir(dir).map_err(|e| {
        PkgError::ManifestInvalid(format!("计算 {} 内容树哈希失败:{e}", dir.display()))
    })?;
    Ok(LoadedPackage {
        manifest,
        content_sha256,
        source,
    })
}

/// 读根清单 + 根内容树哈希。
pub fn load_root(base_dir: &Path) -> PkgResult<(Manifest, String)> {
    let manifest_path = base_dir.join(MANIFEST_NAME);
    if !manifest_path.is_file() {
        return Err(PkgError::ManifestInvalid(format!(
            "根清单不存在:{}",
            manifest_path.display()
        )));
    }
    let text = std::fs::read_to_string(&manifest_path)
        .map_err(|e| PkgError::ManifestInvalid(format!("读取根清单失败:{e}")))?;
    let manifest = Manifest::parse(&text)?;
    let sha = content_tree::hash_dir(base_dir)
        .map_err(|e| PkgError::ManifestInvalid(format!("计算根内容树哈希失败:{e}")))?;
    Ok((manifest, sha))
}

/// 解析整个 workspace(单根锁,RXS-0091)→ 解析图。
pub fn resolve_workspace(base_dir: &Path, offline: bool) -> PkgResult<ResolveGraph> {
    let (root, root_sha) = load_root(base_dir)?;
    let loader = FsLoader::new(base_dir, offline);
    resolve::resolve(&root, &root_sha, &loader)
}

/// `rx vendor`:解析 → 落 vendor/<name>(path 依赖)→ 写 rurix.lock(RXS-0094)。
pub fn run_vendor(base_dir: &Path, offline: bool) -> PkgResult<ResolveGraph> {
    let graph = resolve_workspace(base_dir, offline)?;
    let vendor_root = base_dir.join(VENDOR_DIR);
    for pkg in graph.nodes.values() {
        if pkg.name == graph.root {
            continue;
        }
        if let Source::Path(rel) = &pkg.source {
            let src_dir = base_dir.join(rel);
            let dst_dir = vendor_root.join(&pkg.name);
            copy_content_tree(&src_dir, &dst_dir).map_err(|e| {
                PkgError::SourceUnreachable(format!("vendor {:?} 落盘失败:{e}", pkg.name))
            })?;
        }
        // 远端源:M6.2 不抓取(归 M6.3);其 vendor 缓存若已存在则原样保留。
    }
    let lock = Lock::from_graph(&graph);
    std::fs::write(base_dir.join(LOCK_NAME), lock.serialize())
        .map_err(|e| PkgError::LockMismatch(format!("写 rurix.lock 失败:{e}")))?;
    Ok(graph)
}

/// `--locked` 校验(RXS-0094):入库 lock 与重解析图一致(RX7007)+ vendor 内容树
/// digest 与 lock 记录一致(RX7008)。`offline` 透传解析(远端无缓存 → RX7009)。
pub fn verify_locked(base_dir: &Path, offline: bool) -> PkgResult<ResolveGraph> {
    let lock_path = base_dir.join(LOCK_NAME);
    if !lock_path.is_file() {
        return Err(PkgError::LockMismatch(format!(
            "--locked 要求 rurix.lock 存在:{}",
            lock_path.display()
        )));
    }
    let lock_text = std::fs::read_to_string(&lock_path)
        .map_err(|e| PkgError::LockMismatch(format!("读取 rurix.lock 失败:{e}")))?;
    let lock = Lock::parse(&lock_text)?;
    let graph = resolve_workspace(base_dir, offline)?;
    lock.check_consistent(&graph)?;
    verify_vendor_digests(base_dir, &lock)?;
    Ok(graph)
}

/// 逐包校验 vendor/<name> 内容树 SHA-256 == lock 记录(RX7008)。
fn verify_vendor_digests(base_dir: &Path, lock: &Lock) -> PkgResult<()> {
    for pkg in &lock.packages {
        if pkg.name == lock.root {
            continue;
        }
        // 仅校验已 vendor 的包(path 依赖落 vendor;远端缓存若存在亦校验)。
        let vendored = base_dir.join(VENDOR_DIR).join(&pkg.name);
        if !vendored.is_dir() {
            return Err(PkgError::DigestMismatch(format!(
                "依赖 {:?} 缺 vendor 快照:{}(请先 `rx vendor`)",
                pkg.name,
                vendored.display()
            )));
        }
        let actual = content_tree::hash_dir(&vendored).map_err(|e| {
            PkgError::DigestMismatch(format!("计算 vendor/{} 内容树哈希失败:{e}", pkg.name))
        })?;
        if actual != pkg.content_sha256 {
            return Err(PkgError::DigestMismatch(format!(
                "依赖 {:?} 内容树 digest 不符:vendor 实测 {} != lock 记录 {}",
                pkg.name, actual, pkg.content_sha256
            )));
        }
    }
    Ok(())
}

/// 将 src 目录的规范化内容树(排除 vendor/target/.git/rurix.lock)拷入 dst。
fn copy_content_tree(src: &Path, dst: &Path) -> std::io::Result<()> {
    if dst.exists() {
        std::fs::remove_dir_all(dst)?;
    }
    for (rel, content) in content_tree::collect_dir(src)? {
        let target = dst.join(&rel);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, &content)?;
    }
    Ok(())
}

/// 拼接并规范化两个相对路径(`/` 归一,解析 `.`/`..`)。
fn join_normalize(base: &str, rel: &str) -> String {
    let mut comps: Vec<String> = Vec::new();
    for seg in base.split(['/', '\\']).chain(rel.split(['/', '\\'])) {
        match seg {
            "" | "." => {}
            ".." => {
                if matches!(comps.last().map(String::as_str), Some(s) if s != "..") {
                    comps.pop();
                } else {
                    comps.push("..".to_owned());
                }
            }
            s => comps.push(s.to_owned()),
        }
    }
    if comps.is_empty() {
        ".".to_owned()
    } else {
        comps.join("/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TempWs(PathBuf);
    impl TempWs {
        fn new() -> TempWs {
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let pid = std::process::id();
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let dir = std::env::temp_dir().join(format!("rurix_pkg_test_{pid}_{nanos}_{n}"));
            std::fs::create_dir_all(&dir).unwrap();
            TempWs(dir)
        }
        fn write(&self, rel: &str, content: &str) {
            let p = self.0.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, content).unwrap();
        }
    }
    impl Drop for TempWs {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn sample_ws() -> TempWs {
        let ws = TempWs::new();
        ws.write(
            "rurix.toml",
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\n[dependencies]\nfoo = { path = \"foo\" }\n",
        );
        ws.write("src/main.rx", "fn main() {}\n");
        ws.write(
            "foo/rurix.toml",
            "[package]\nname = \"foo\"\nversion = \"0.2.0\"\n",
        );
        ws.write("foo/src/lib.rx", "fn foo() {}\n");
        ws
    }

    //@ spec: RXS-0094
    #[test]
    fn vendor_then_verify_locked_ok() {
        let ws = sample_ws();
        let g = run_vendor(&ws.0, true).unwrap();
        assert!(g.nodes.contains_key("foo"));
        assert!(ws.0.join("rurix.lock").is_file());
        assert!(ws.0.join("vendor/foo/rurix.toml").is_file());
        // --locked --offline 校验通过。
        verify_locked(&ws.0, true).unwrap();
    }

    //@ spec: RXS-0093
    #[test]
    fn tampered_vendor_digest_is_rejected() {
        let ws = sample_ws();
        run_vendor(&ws.0, true).unwrap();
        // 篡改 vendor 快照内容 → digest 不符 RX7008。
        std::fs::write(
            ws.0.join("vendor/foo/src/lib.rx"),
            "fn foo() { /* tampered */ }\n",
        )
        .unwrap();
        let err = verify_locked(&ws.0, true).unwrap_err();
        assert_eq!(err.code(), "RX7008");
    }

    //@ spec: RXS-0092
    #[test]
    fn tampered_lock_is_rejected() {
        let ws = sample_ws();
        run_vendor(&ws.0, true).unwrap();
        // 篡改 lock 中 foo 的 content_sha256 → 与重解析图不一致 RX7007。
        let lock_text = std::fs::read_to_string(ws.0.join("rurix.lock")).unwrap();
        let tampered = lock_text.replace("content_sha256 = \"", "content_sha256 = \"0000");
        std::fs::write(ws.0.join("rurix.lock"), tampered).unwrap();
        let err = verify_locked(&ws.0, true).unwrap_err();
        assert_eq!(err.code(), "RX7007");
    }

    //@ spec: RXS-0090
    #[test]
    fn missing_path_source_is_unreachable() {
        let ws = TempWs::new();
        ws.write(
            "rurix.toml",
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\n[dependencies]\ngone = { path = \"gone\" }\n",
        );
        ws.write("src/main.rx", "fn main() {}\n");
        let err = resolve_workspace(&ws.0, true).unwrap_err();
        assert_eq!(err.code(), "RX7009");
    }

    #[test]
    fn join_normalize_resolves_dotdot() {
        assert_eq!(join_normalize(".", "../foo"), "../foo");
        assert_eq!(join_normalize("mid", "../foo"), "foo");
        assert_eq!(join_normalize("a/b", "../../c"), "c");
        assert_eq!(join_normalize(".", "."), ".");
    }
}
